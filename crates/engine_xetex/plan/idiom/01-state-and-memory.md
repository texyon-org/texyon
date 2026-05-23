# 01 — State & memory redesign

The root-cause workstream. It converts the engine from 384 C-symbol
globals, 16 promoted Rust statics, and raw `i32` arena/halfword values
into a Rust-owned `EngineState` with a typed memory arena, threaded as
`&mut`. Everything else in Stage 2 (de-unsafe, clippy's big bucket,
clean module boundaries) follows from this.

> Status: **READY** for execution. The required validation pass and
> global-inventory deliverable exist in `01a-global-inventory.md`.
> Constraint: do not migrate the `misc` linebreaker/pagebuilder/synctex/io
> scratch entries until their final sub-structs are decided.

## 1. Goal

End state for this workstream (a subset of `00-overview.md`'s targets):

-   `EngineState` struct with the `02-state.md` sub-struct clustering.
-   The memory arena is `Mem { data: Vec<MemoryWord>, … }`, accessed via
    typed `#[inline]` methods. Valid arena indexes, nullable TeX
    pointers, eqtb/hash indexes, scaled dimensions, string numbers, font
    numbers, tokens, and node types get distinct wrappers where they
    carry distinct invariants — never bare `i32`/`u16` at call sites.
-   `eqtb`, `save_stack`, `input_stack`, string pool, fonts, hyph,
    shipout, scanner, errors, config likewise owned as Rust fields.
-   State reached via `&mut EngineState` (or a narrower `&mut Mem` etc.)
    parameters. No thread-local, no `static mut` (except true consts and
    the bridge error channel).

Non-goals (explicitly out, per anti-patterns): no `Box<dyn Node>`
(A.5), no enum-per-node (`03-memory-and-nodes.md` §"Why not"), no
generic memory backend (A.6), no algorithm changes (A.7).

## 2. Current reality (verify before executing)

Measured 2026-05-23; re-check at execution time.

-   `src/globals.rs` (5,569 LOC): a private `mod raw { extern "C" { … } }`
    holding 384 still-C-defined `static mut` globals, plus 811 safe
    wrapper fns (`rover()`/`set_rover(v)`, `mem_word(p)`/`set_mem_word`,
    `eqtb_b32s1(i)`, `box_width`-style helpers, etc.). Some globals were
    already *promoted* to Rust `#[no_mangle] pub static mut` during
    Stage 1 (16 `pub static mut` definitions by text search, e.g.
    shipout DVI statics and io name buffers); those live outside
    `mod raw`.
-   The arena: `mem` is a `static mut *mut MemoryWord` allocated **in
    Rust code** at `src/ini.rs:2669` and `:3258` via
    `xmalloc_array::<MemoryWord>(MEM_TOP[+1])`, where the helper
    allocates `n + 1` elements. So the backing store is a C-`malloc`'d
    block behind a C-symbol pointer, freed by `tt_cleanup`; it is not
    yet Rust-owned in the `Vec`/`Drop` sense. `eqtb`, `str_start`,
    `str_pool`, font arrays, trie arrays, etc. follow the same
    "Rust code allocates a C-style array behind a `static mut` pointer"
    shape.
-   `MemoryWord` / `B32x2` / `B16x4` already exist as `#[repr(C)]` types
    in `src/mem_types.rs`, with `size_of == 8` compile-time asserts.
    `ListStateRecord` and `InputState` (`globals.rs`) likewise.
-   **No non-macOS C algorithm reads `mem[]`/`eqtb[]` any more**
    (verified: no `mem[`/`eqtb[` in non-macos `.c`). So storage can
    move to Rust ownership with no algorithm-level cross-language sync.
    The inventory still must account for C symbol definitions, the
    `yhash` helpers, the C entry shim, and Rust cleanup aliases.

What does *not* exist yet: `MemIdx`, a nullable pointer/halfword wrapper,
`Scaled`, `FontId`, `StrNumber`, `Token` newtypes; any `EngineState` /
`Mem` / `Eqtb` struct; any thread-local. The accessors traffic in raw
`i32`.

## 3. First executable step (the inventory deliverable)

Before any code moves, produce **`01a-global-inventory.md`** (a new doc
in this directory): the authoritative mapping of every global to its
target cluster. This is low-risk, high-value, and unblocks every later
step. It is the natural first execution session.

Status: **done** in `01a-global-inventory.md` (2026-05-23). The
inventory covers 384 raw extern globals, 16 promoted Rust `pub static
mut` globals, and an appendix of 18 private module-local `static mut`
values that are also engine state.

For each global currently declared in `globals.rs` (both `mod raw` and
promoted statics), record:

| col | meaning |
|---|---|
| name | C symbol name |
| type | Rust type today (`i32`, `*mut MemoryWord`, `[i32; N]`, …) |
| defined where | still C-defined (`mod raw`) vs. promoted Rust static |
| cluster | one of: `mem`, `eqtb`, `save_stack`, `input`, `strings`, `fonts`, `hyph`, `shipout`, `scanner`, `errors`, `config`, `misc` |
| accessors | the wrapper fn names in `globals.rs` that touch it |
| C readers/writers/linkers? | any remaining non-macOS compiled C referencing it, any non-macOS C file defining it, any Rust `#[link_name]` cleanup alias, and whether the `yhash` helpers touch it |

Generate the raw list mechanically, then classify:

```bash
# names of extern statics still C-defined
grep -nE 'pub\(super\) static mut' src/globals.rs
# promoted Rust statics
grep -nE '#\[no_mangle\]\s*$|pub static mut' src/globals.rs src/*.rs
# does any non-macOS compiled C still name a given symbol?
grep -rn '\bSYMBOL\b' xetex/*.c | grep -v macos
# does Rust cleanup still free it by C symbol?
grep -rn 'link_name = "SYMBOL"' src/ini.rs
```

The clustering follows the table in `plan/02-state.md` §"state
categories". Ambiguous globals (the `misc` bucket) get a one-line
rationale. Output: a markdown table + a per-cluster count + a list of
old symbol definitions/cleanup aliases that must be removed when each
cluster migrates. **This count becomes the unit of work** — Phase B
migrates one cluster at a time.

Do not inventory `xetex-macos.c` as an execution blocker. If it names a
symbol being moved, record "macOS-only, intentionally unvalidated" and
leave that file untranslated.

## 4. Newtypes (introduce early, sweep gradually)

Per `plan/03-memory-and-nodes.md` §"Scaled and other unit newtypes" and
§"MemIdx". Define in a new `src/units.rs` (or fold into `mem_types.rs`),
but correct/strengthen the parent sketch as follows:

-   `MemIdx(i32)` means a **valid index into `mem.data`**. It must not
    contain `TEX_NULL` or other negative sentinels.
-   Add a distinct `Halfword(i32)` / `MemPtr(i32)` wrapper for TeX fields
    that can contain either a memory index or `TEX_NULL`. Convert to
    `Option<MemIdx>` only at checked boundaries.
-   `FontId` should initially be `#[repr(transparent)] struct FontId(i32)`,
    because `internal_font_number` is `int32_t` in
    `xetex-xetexd.h:326`. We can narrow only after a dedicated audit.
-   `Scaled(i32)`, `StrNumber(i32)`, `Token(i32)`, and eqtb/hash index
    wrappers should stay representation-compatible with the translated C
    types until their ranges are fully audited.
-   `NodeType` must be constructed from raw values with a checked
    `from_raw`/`TryFrom`/`match` path. Do **not** `transmute` unchecked
    `u16` values into an enum; an invalid discriminant is Rust UB.
-   Do not add broad arithmetic operator overloads for `Scaled` in the
    first pass. TeX arithmetic has overflow/error side effects
    (`arith_error`, `tex_remainder`, `xn_over_d`, etc.); keep existing
    helpers until each operation's behavior is audited.

Sequencing tension: 811 accessors return raw C-shaped values today, and
hundreds of translated functions pass `i32` around. Converting all at
once is a giant untestable diff
(violates A.12). Options, to decide when drafting reaches `READY`:

-   **(a) Newtypes at the arena boundary first.** Give the new `Mem`
    accessors `Scaled`/valid-`MemIdx`/nullable-`MemPtr` signatures; let
    call sites do explicit unwrap/constructor conversions at the
    boundary initially, then sweep inward module-by-module. Pro:
    localized, testable. Con: conversion noise during transition.
-   **(b) Defer newtypes until after ownership moves.** Do Phase B/C
    with `i32` first, introduce newtypes as a final polishing sweep.
    Pro: separates two hard changes. Con: a second full call-site pass.

Rule for execution: use (a) only for wrappers whose invariant is
unambiguous at the accessor boundary (`Scaled`, valid `MemIdx`, and
checked nullable `MemPtr`/`Halfword` conversions). Use (b) for the rest.
Newtypes must be `#[derive(Copy, Clone)]` + `#[repr(transparent)]` so
they are zero-cost and `#[inline]` accessors compile identically to the
raw index. Confirm with a release-mode disassembly spot-check on a hot
accessor (e.g. `box_width`) before committing to the pattern.

**Deferral must be tracked, not silent (mandatory).** Stage 1 already
collapsed every semantic C type — `selector_t`, `tt_history_t`,
`group_code`, `scaled_t`, `str_number`, `internal_font_number`, the
various `*_t`/`small_number` mode enums — into raw `i32`/`u8`. A Phase-B
cluster move that lands such a field in `EngineState` as a bare integer
is faithful and correct **only if it records the debt**. Otherwise the
type information is lost for good: once the field is an `i32` in a
struct, nothing signals that it was ever a `selector_t`.

Therefore, **every cluster migration commit MUST**, for each field it
moves:

1.  Check the field's declared type in `xetex/xetex-xetexd.h` (and
    `xetex-core.h` for the enum bodies). Plain `int32_t`/`bool` counters
    that are genuinely numbers need nothing. A semantic typedef or enum
    (`selector_t`, `tt_history_t`, `group_code`, `scaled_t`,
    `str_number`, `internal_font_number`, `glue_ord`, node-type codes,
    mode codes, …) is a newtype debt.
2.  Add a row for that field to the **Deferred newtype worklist** in
    `01a-global-inventory.md` (target newtype + original C type), unless
    a row already exists.

Phase B keeps the `i32` representation and the accessor signature
unchanged — that part of the bridge is unchanged. The worklist is what
guarantees the §4(b) newtype sweep has a complete, auditable backlog
instead of relying on memory. Migrating a semantic-typed field **without**
adding its worklist row is a plan violation, even though the code still
compiles and passes parity.

## 5. The arena: `Mem`

Target shape is already specified in `plan/03-memory-and-nodes.md`
(`Mem` struct, `word`/`word_mut`, typed accessors, `NodeType`). Build it
there. Migration specifics for *this* codebase:

1.  Introduce `Mem { data: Vec<MemoryWord>, lo_mem_max, hi_mem_min,
    mem_end, avail, rover, var_used, dyn_used }`. Pre-allocate `data`
    to the exact inclusive size required by the current
    `xmalloc_array::<MemoryWord>(n)` call shape (`n + 1` elements; note
    the initex path currently passes `MEM_TOP + 1`). It must **never
    reallocate** during a run. A realloc would invalidate any retained
    raw pointer while cleanup/FFI edges still exist, and it would make
    codegen comparisons meaningless.
2.  Replace the `xmalloc_array` allocation in `src/ini.rs` with
    Rust-owned storage. Preserve per-array initialization semantics:
    current `xcalloc_array` allocations are zeroed; current
    `xmalloc_array` allocations are logically uninitialized but Rust
    storage must be initialized before safe reads. Prefer explicit
    initialization that parity proves harmless; use `MaybeUninit` only
    if a real byte-output or format-output regression shows that
    zero-initializing unused slots changes behavior. This is the moment
    arena storage becomes truly Rust-owned (`Drop` or explicit teardown
    frees it). Watch the longjmp and `tt_cleanup` hazards (§7).
3.  Port the relevant `globals.rs` arena accessors onto `Mem` as inherent
    methods, returning semantic types. The `unsafe` union field read
    becomes a single audited site inside each accessor — and, crucially,
    once `data` is a `Vec`, the access is a safe slice index, so the
    `unsafe` is confined to the `MemoryWord` union projection, not the
    pointer deref. This is where most of the 688 unsafe-block lints die.
4.  The allocator functions (`get_avail`, `get_node`, `free_node`,
    `flush_list`, `sort_avail`, the `new_*` node constructors) already
    exist in `src/mem.rs`. Re-target them from `globals::mem_word` to
    `&mut Mem`. They are the natural first consumers of the new arena
    and have tight golden coverage. Any accessor whose field may contain
    `TEX_NULL` returns `MemPtr`/`Halfword`, not `MemIdx`.

## 6. Phases B and C (per cluster)

For **each cluster** from the §3 inventory, least-coupled first
(`02-state.md` order: strings → errors → config → fonts → scanner →
save_stack → eqtb → input → mem → hyph/shipout):

**Phase B (bridge):**
1.  Define the sub-struct (e.g. `StringPool`) as a field of
    `EngineState`.
2.  Move storage ownership from the `static mut` global into the struct
    field. Since no C algorithm reads it, this is: delete/neutralize the
    C definition or promoted Rust static, remove any `#[link_name]`
    cleanup alias, allocate the field on `EngineState` init, and make
    explicit teardown own the free/release path. Do not leave the old
    C symbol pointing at a Rust `Vec` unless a documented FFI edge still
    needs that temporary shim.
3.  Repoint the `globals.rs` accessors at the thread-local:
    `pub fn rover() -> i32 { state::with_state(|s| s.mem.rover) }`.
    Signatures of the ~540 translated fns **do not change** in Phase B —
    that is the point of the bridge. Build, run all four suites, commit.
4.  One cluster per commit (or a few commits for a big cluster like
    `eqtb`/`mem`). Parity must hold each time.

Phase-B borrowing rule: `with_state` accessors may borrow the
thread-local only long enough to read or write the field. Do not call
legacy translated code while a `RefCell` borrow is live; nested
`with_state` calls will panic. Longer operations should be explicit
methods taking `&mut EngineState`/`&mut Cluster` even before the full
Phase-C sweep.

**Phase C (thread):** once all clusters are bridged and the thread-local
is the sole owner, replace `with_state(|s| …)` call sites with explicit
`&mut EngineState` (or narrower) parameters, **one subsystem at a
time**. The thread-local stays as a shrinking safety net until the last
subsystem is threaded, then is deleted. This is mechanical but large;
it is where `scan.rs`'s 179 `#[no_mangle]` fns lose their `extern "C"`
and gain a `state: &mut EngineState` parameter. Coordinate with
workstream 02 (module split) and 03 (de-unsafe), since a function being
threaded is also the moment to drop its `#[no_mangle]` and de-`unsafe`
it.

## 7. The longjmp / `Drop` hazard (design constraint, not afterthought)

`tt_engine_xetex_main` (kept in C) wraps the run in `setjmp`; error
paths `longjmp` back, unwinding the C stack **without running Rust
`Drop`**. Implications:

-   While `EngineState` lives in a `'static` thread-local (Phase B and
    most of C), a `longjmp` does not jump over its storage, so no
    use-after-free — but its `Drop` (freeing `Vec`s) won't run on the
    abort path. That matches today's behavior (globals are never freed
    on abort either) and is acceptable: the process/engine invocation is
    tearing down. Document it; don't rely on `Drop` for correctness on
    the abort path. `tt_cleanup` (already in Rust) is the explicit
    teardown today; as ownership moves into `EngineState`, move those
    frees/releases into an explicit `EngineState::reset/cleanup` path
    and make `tt_cleanup` call that instead of freeing C symbols.
-   **Do not** put an owning `EngineState` in a Rust stack frame that the
    `longjmp` jumps over while it owns live `Vec`s — that frame's
    destructors are skipped *and* its slot may be clobbered. Keep the
    owner in the thread-local (or behind a `ManuallyDrop` owned by the
    entry shim) until/unless the setjmp boundary itself is restructured.
    Fully stack-threaded Phase C across the longjmp boundary is the one
    place where "most idiomatic" collides with the C escape mechanism;
    resolve it explicitly when drafting reaches that subsystem (likely:
    keep a thin thread-local/`ManuallyDrop` anchor at the entry point and
    thread `&mut` *below* it, so no owning value is ever jumped over).

This is the main reason `00-overview.md` insists we go *through* Phase B
rather than leaping to stack-threaded state.

## 8. Per-step validation

Identical to `plan/06-workflow.md` steps 1/6: baseline all four suites,
make the change, re-run all four, `diff` the outputs, commit only on
zero regression. Additionally for this workstream:

-   After the arena move (§5), run the unit tests in `src/mem.rs` /
    `scaled_math.rs` and add round-trip tests for any new typed accessor.
-   Spot-check release codegen for one hot accessor to confirm newtypes +
    `Vec` indexing are zero-cost vs. the old raw-pointer deref.
-   Watch `clippy` error/warn counts trend **down** as unsafe blocks
    dissolve — record in `STATUS.md`'s metrics row. (They need not hit 0
    until 03/04; they must not go *up*.)

## 9. Decisions and deferred decisions

These are no longer blockers for starting 01, but they constrain how it
is executed:

1.  Newtype rollout: use boundary-first newtypes only where invariants
    are unambiguous; defer the rest. The per-accessor taxonomy is part
    of each accessor migration.
2.  Exact `EngineState` field list: use `01a-global-inventory.md` as the
    source of truth. `misc` is a real holding cluster; decide a final
    sub-struct before migrating any `misc` row.
3.  Non-macOS C/link edges: use the inventory's `C/link status` column
    per symbol. The `hash`/`yhash` helper edge belongs to the
    `eqtb`/hash cluster. macOS-only references do not block this plan.
4.  `EngineState` anchor vs. setjmp: keep the owner in a
    thread-local/`ManuallyDrop` anchor until the entry path is explicitly
    redesigned. Decide the final anchor before threading the entry path;
    this does not block early Phase-B cluster moves.
5.  Cluster migration order: start with least-coupled clusters
    (`strings`, `errors`, `config`) and leave `mem`, `eqtb`, and `misc`
    until after cleanup ownership and pointer taxonomy are exercised on
    smaller clusters.
6.  Pointer/index taxonomy: every arena accessor commit must decide
    which fields are valid `MemIdx`, nullable `MemPtr`/`Halfword`, or
    checked `Option<MemIdx>`.
7.  Cleanup ownership: every cluster migration must update the
    inventory-listed `#[link_name]` aliases/free paths in the same
    commit that moves ownership.

## 10. Risks

-   **Borrow-checker friction at cluster seams.** A function that today
    freely touches `mem` and `eqtb` via globals will, once threaded, need
    `&mut s.mem` and `&mut s.eqtb` simultaneously — fine, disjoint
    fields — but a function that re-borrows `s` while holding a field
    borrow will fight the checker. Fix by splitting borrows or passing
    narrower params (A.2/A.4), never by `.clone()`-ing the arena.
-   **Accidental behavior change in an accessor.** A typed accessor that
    sign-extends or truncates differently from the raw `i32` read is a
    silent parity break. Mirror the exact C field width; lean on TRIP.
-   **Sentinel/index confusion.** Many TeX "pointers" are halfwords that
    can contain `TEX_NULL` or other negative sentinel values. Treating
    them as valid `MemIdx` values will turn a clean type layer into
    panics or, worse, wrong indexing. Keep the `MemIdx` vs.
    `MemPtr`/`Halfword` distinction strict.
-   **Double free during ownership transfer.** If a cluster moves to
    `Vec`/Rust-owned resources but `tt_cleanup` still frees the old C
    symbol, normal runs can double-free and abort paths can leak a
    different owner. Every Phase-B commit must include the cleanup edit
    for that cluster.
-   **Diff size.** Phase C threading touches hundreds of signatures. Keep
    it per-subsystem and per-commit; resist "thread the whole crate in
    one go."

## 11. Execution recipe (read before each Phase-B slice)

This is the repeatable procedure proven across sessions 4–11. A session
is one cluster (or one sub-group / one array) per commit. The model is
reset between sessions, so this recipe — not memory — is the source of
truth for *how* to execute. Run from the crate root
(`crates/engine_xetex`).

**1. Pick the slice.** Read `STATUS.md` → "Next action". Take the next
group. Hard rule: do **not** bridge the hot tokenizer scalars (`cur_cmd`,
`cur_chr`, `cur_tok`, `cur_cs`, `cur_val`) through the thread-local —
they are read per `get_next`/`get_token`; thread them in Phase C instead.

**2. Verify the slice is safe to move (grep before touching anything).**
For the symbols `S` in the slice:

```bash
# (a) original C types — needed for the §4 newtype-worklist rule
for s in S…; do grep -hE "extern .* \b$s\b *;" xetex/xetex-xetexd.h; done
# (b) cleanup aliases — if any hit, this is an ownership slice (see step 6)
grep -nE 'link_name = "(S…)"' src/ini.rs
# (c) C readers — must be empty except xetex-ini.c (the definition)
grep -lnE '\b(S…)\b' xetex/*.c | grep -v macos
# (d) cross-module extern redeclarations (the synctex.rs gotcha) — must be empty
grep -rnE '\b(S…)\b' src/*.rs | grep -v 'src/globals.rs' \
  | grep -iE 'extern|static mut|link_name'
```

If (c) shows a non-`ini.c` reader, or (d) shows a redeclaration, stop and
handle it explicitly — do not migrate blind.

**3. Add the sub-struct field(s).** In `src/state.rs`, add a field to the
cluster's sub-struct (create the sub-struct + add it to `EngineState` if
it's the cluster's first field). `#[derive(Default)]` covers scalars,
fixed arrays, and `Vec` (empty). A struct containing a large fixed array
(`[i32; 55]`, `[u16; 256]`) or a non-`Default` element needs a hand-
written `impl Default`.

**4. Repoint the accessors.** In `src/globals.rs`, change each
getter/setter body from `unsafe { raw::SYM }` /
`unsafe { raw::SYM = v }` to
`crate::state::with_state(|s| s.<cluster>.SYM)` /
`crate::state::with_state_mut(|s| { s.<cluster>.SYM = v; })`. Keep
signatures identical (this is the bridge). Closures touch one field and
never call engine code while the borrow is live.

**5. Remove the old definitions in the same commit.** Delete the
`pub(super) static mut SYM …;` lines from `globals.rs` `mod raw`, and the
matching C definitions from `xetex/xetex-ini.c` (some symbols live in
other `.c` files or are scattered — grep to confirm each). Watch for
symbols *interleaved* with ones you are keeping (e.g. `disc_ptr`,
`hash_extra`, `save_size`); remove individually around them.

**6. Ownership slices only (an array allocated by Rust / freed by C).**
Use the session-11 `save_stack` shape:
- field is `Vec<T>` on the sub-struct; accessors index it (safe,
  bounds-checked — only a `MemoryWord` union projection stays `unsafe`);
- in `src/ini.rs`, replace the `xmalloc_array::<T>(n)` call with
  `with_state_mut` writing `vec![<zero T>; (n as usize) + 1]`
  (`xmalloc_array` allocates `n + 1` — match it);
- `tt_cleanup` drops the `Vec` (`= Vec::new()`) instead of `libc::free`;
- remove **both** `#[link_name = "SYM"]` aliases (the alloc alias and the
  cleanup-free alias), the raw extern, and the C def — same commit;
- the owner stays in the `'static` thread-local, so longjmp/abort never
  jumps over live storage and `Drop`-on-abort is intentionally skipped
  (§7). Check whether the array is in the format dump/undump
  (`save_stack` and the `input` stacks are **not**; `str_pool`/`str_start`
  and `eqtb` **are** — those slices must reconcile the load/dump paths).

**7. Record newtype debts (§4 rule).** For any moved field whose C type
(from step 2a) is a semantic typedef/enum (`selector_t`, `tt_history_t`,
`group_code`, `scaled_t`, `str_number`, `pool_pointer`,
`internal_font_number`, …), add a row to the Deferred newtype worklist in
`01a-global-inventory.md`. Plain `int32_t`/`bool`/char counts need no row
— say so in the session log so the next session knows it was checked.

**8. Build, then run the full dashboard. Never regress it.**

```bash
cargo check -p tectonic_engine_xetex
cargo test -p tectonic --test tex-outputs --test trip \
  --test engine_rust --test latex_goldens
# expected: 29 / 2 / 7 / 4
```

Also `grep -rnE '\b(S…)\b' xetex/*.c | grep -v macos` must be empty
(no leftover C reference). Re-measure for the metrics table:
`grep -cE 'pub\(super\) static mut' src/globals.rs` and
`grep -rohE 'unsafe \{' src/*.rs | wc -l`.

**9. Update the docs and commit.** Append a `SESSION_LOG.md` entry
(work, metrics, validation, gotchas, "for the next session"), update
`STATUS.md` (execution list, metrics row, "Next action", and the
"Last updated" line), and commit **only the engine-crate files** —
`src/*.rs`, `xetex/xetex-ini.c`, and `plan/idiom/*`. Do **not** stage the
stray test-output artifacts, `test.pdf`, `bundles/`, etc. that sit in the
working tree. Commit message prefix: `engine_xetex: …`.

### Recurring gotchas (have bitten a session)

-   `xmalloc_array(n)` / `xcalloc_array(n)` allocate `n + 1` elements
    (`src/ini.rs`). A Rust `Vec` replacement must match the inclusive
    size or indexing is off by one.
-   `MemoryWord` is a `#[derive(Copy, Clone)]` **union** with no
    `Default`. Build a zero word explicitly:
    `MemoryWord { b32: B32x2 { s0: 0, s1: 0 } }` (works in `vec![…; n]`
    because it is `Copy`).
-   `mod raw` statics are `pub(super)`, so other modules cannot reach
    `raw::SYM` — but a few modules historically declared their **own**
    `extern "C"` block for a symbol (e.g. `synctex.rs` for
    `synctex_enabled`). Step 2d catches these; they must be repointed
    too.
-   Some symbols are defined in a `.c` file other than `xetex-ini.c`
    (e.g. `no_shrink_error_yet` in `xetex-linebreak.c`,
    `current_diagnostic` in `xetex-output.c`). Grep each symbol; do not
    assume `xetex-ini.c`.
-   The IDE/LSP `dead_code` / "never read" warnings that fire mid-edit
    are **stale** — they clear once all accessors are repointed and the
    crate is rebuilt. Don't chase them; trust `cargo check`.
-   `mem` bookkeeping scalars (`var_used`, `dyn_used`, `lo_mem_max`,
    `hi_mem_min`, `avail`, `rover`, `mem_end`) are best moved **with**
    the `Mem` arena (§5), not as a loose scalar slice.
