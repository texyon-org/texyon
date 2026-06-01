# 01b — Phase-C execution recipe (threading `&mut EngineState`)

The `READY` execution recipe for **Phase C** of workstream 01: replacing
the thread-local `with_state` bridge with explicit `&mut EngineState`
(or narrower `&mut MemState` etc.) parameters, one subsystem at a time,
until the thread-local can be deleted. This is the Phase-C analog of the
§11 recipe in `01-state-and-memory.md` (which is the Phase-**B** recipe).

> Status: **READY** for execution (drafted + recon-validated session 60).
> Hard rule 4 (`README.md`) is satisfied: threading code may now begin,
> following this recipe. Phase B is complete (all bridgeable clusters are
> on `EngineState`; the 40 remaining `mod raw` statics are the
> intentionally-deferred hot-tokenizer / capacity / `cur_ptr` set).

Read this **with** `01-state-and-memory.md` §5 (the arena), §6 (Phase
B/C split), §7 (the longjmp/`Drop` hazard), and §10 (risks). This doc
makes those concrete and adds the recon that picks the first slice.

---

## 1. What Phase C actually changes

Phase B made every migrated global a field on a single thread-local
`EngineState`, reached through `with_state`/`with_state_mut` accessors in
`globals.rs`. Signatures of the ~540 translated fns did **not** change —
that was the point of the bridge.

Phase C reverses the bridge direction. A function that today does

```rust
pub unsafe extern "C" fn free_node(p: i32, s: i32) {
    set_mem_b32s0(p, s);              // -> with_state_mut(|s| s.mem...)
    ...
}
```

becomes a function that **takes the state it needs as a parameter** and
touches it directly:

```rust
pub fn free_node(mem: &mut MemState, p: i32, s: i32) {
    mem.set_b32s0(p, s);             // direct field access, no bridge
    ...
}
```

The win is exactly the §5/§10 payoff: the `unsafe` pointer/bridge access
collapses into safe slice indexing, `#[no_mangle] extern "C"` drops off
internal functions, and borrow disjointness becomes checkable by the
compiler instead of enforced by the "don't nest `with_state`" convention.

Phase C is **mechanical but large** and touches hundreds of signatures.
The discipline is: **one subsystem (often one function + all its call
sites) per commit, parity green every time** — identical to Phase B.

## 2. The core model: anchor + thread-below

From §7: `tt_engine_xetex_main` wraps the run in `setjmp`; error paths
`longjmp` back, unwinding the C stack **without running Rust `Drop`**.
The resolution that lets Phase C proceed without restructuring the
setjmp boundary:

-   **The owner stays in the thread-local anchor.** `EngineState` lives
    in the `'static thread_local! ENGINE_STATE` for all of Phase C. A
    `longjmp` never jumps over its storage, so there is no
    use-after-free; its `Drop` (freeing `Vec`s) simply does not run on
    the abort path, which matches today's behavior (`tt_cleanup` is the
    explicit teardown; globals were never freed on abort either).
-   **Thread `&mut` *below* the anchor.** A threaded subsystem is entered
    by borrowing the anchor *once* at the boundary and passing the
    `&mut` down:

    ```rust
    // legacy call site, during transition:
    with_state_mut(|s| free_node(&mut s.mem, p, sz));
    ```

    No owning `EngineState` is ever placed in a stack frame the
    `longjmp` jumps over. This is the "thin thread-local anchor at the
    entry, thread `&mut` below it" resolution promised in §7.

-   **Do not** introduce a stack-owned `EngineState` (e.g.
    `let mut st = EngineState::new(); run(&mut st);`) until/unless the
    setjmp boundary itself is restructured. That is explicitly out of
    scope for the per-subsystem threading slices.

## 3. The bridge-down technique (mirror of Phase B)

Phase B wrapped *storage* in an accessor that reached *up* into the
thread-local. Phase C wraps *call sites* that reach *down* into the
threaded function:

| | Phase B (bridge-up) | Phase C (bridge-down) |
|---|---|---|
| What changes | the accessor body | the function signature |
| Bridge lives at | `globals.rs` accessor | the **call site** |
| Shape | `fn rover() { with_state(\|s\| s.mem.rover) }` | `with_state_mut(\|s\| f(&mut s.mem, …))` |
| Removed when | — (Phase C removes it) | the *caller* is itself threaded |

So threading a function `f` is a two-part edit in one commit:

1.  Change `f`'s signature to take `&mut MemState` / `&mut EngineState`
    and rewrite its body to touch fields directly (or call the narrow
    `MemState` methods — see §6).
2.  At **every** call site of `f`, either
    -   wrap it `with_state_mut(|s| f(&mut s.mem, …))` if the caller is
        still legacy (this is the transient bridge-down), **or**
    -   pass the already-available `&mut` through if the caller is
        already threaded.

When every caller of `f` is threaded, the wrappers vanish and `f`'s
bridge-down is gone. When every function that touches a cluster is
threaded, that cluster's `with_state` accessors in `globals.rs` are dead
and get deleted. When *all* clusters are threaded, the thread-local
itself is deleted. That last deletion is the end of Phase C.

## 4. The nest-panic rule (precise, with the recon finding)

`with_state`/`with_state_mut` borrow the **entire** `ENGINE_STATE`
`RefCell` (not a per-field lock). Therefore:

> **A function reached under a live `with_state*` borrow must not call
> any other `with_state*`-bearing function.** The second borrow panics
> (`already borrowed`/`already mutably borrowed`).

This is the single hazard that governs which functions can be threaded
together. Crucially — and this corrects the blanket claim in the session
59 handoff — **not every accessor a threaded function calls is
`with_state`-bearing.** During recon (session 60) two categories were
distinguished inside the allocator core:

-   **Thread-local accessors** (`set_help_ptr`, `print_*`, all the
    migrated-cluster getters/setters) → go through `with_state` → **do**
    nest-panic under a live borrow.
-   **Still-raw `mod raw` accessors** — `line()` and
    `cur_input_synctex_tag()` read `raw::line` / `raw::cur_input`
    directly (those globals are the intentionally-deferred Phase-C set,
    *not* on the thread-local). They are `unsafe { raw::… }`, **not**
    `with_state`, so they **do not** nest-panic under a live borrow.

Concrete consequence for the allocator core (verified bodies, session 60):

| fn | touches | `with_state`-bearing internal calls |
|---|---|---|
| `free_node` | `mem` only | **none** |
| `flush_list` | `mem` only | **none** |
| `get_avail` | `mem`; cold `runaway()`+`overflow()` | only the **cold** abort path |
| `get_node` | `mem`; `line()`/`cur_input_synctex_tag()` (raw, safe); cold `overflow()` | only the **cold** abort path |

So `free_node`/`flush_list` can be threaded to `&mut MemState` and
wrapped at legacy sites with **zero** nest-panic risk on any path.
`get_avail`/`get_node` are safe on the **hot** path; the only nest hazard
is the cold memory-exhaustion path that calls `overflow()`/`runaway()`
(see §5). This is why the allocator core is threadable now, contrary to
the pessimistic "would nest-panic" note in the session 59 handoff.

**Per-slice obligation:** before threading a function, classify every
call it makes into (a) about-to-be-threaded (passes `&mut` through),
(b) raw-`mod raw` (safe), or (c) still-bridged `with_state` (hazard).
Any (c) call on a *hot* path means the slice is not closed yet — thread
its callee first, or widen the slice to include it.

## 4a. Classified codemod passes

Phase C should not be hand-edited one call at a time where the shape is
mechanical. Use a two-step workflow:

1.  **Classify first.** For each target function/group, record which of
    these buckets it falls into before editing:
    -   **A: pure pass-through threading** — body only needs already-
        threaded helpers or direct state methods; callers only need
        `foo(...)` → `foo_global(...)` or pass existing `&mut` arguments.
    -   **B: accessor rewrite** — body has direct equivalents such as
        `mem_b32s1(p)` → `mem.b32s1(p)`, `set_mem_b32s1(p, v)` →
        `mem.set_b32s1(p, v)`, or `get_node_global(n)` →
        `get_node(mem, n)` plus the established sentinel handling.
    -   **C: borrow-dropping wrapper** — body calls output/printing/error
        helpers (`dvi_out`, `write_to_dvi`, `overflow`, `confusion`,
        `runaway`, `flush_node_list`, scanner/output routines) and must
        split into a direct prepare/mutate helper plus a `_global` wrapper
        that performs those calls after the state borrow drops.
    -   **D: manual only** — argument evaluation or control flow can
        re-borrow state, depend on side effects, or cross a scanner/error
        boundary. Read and rewrite by hand.

    Use the read-only recon helper to get a first-pass ranked list:

    ```bash
    python3 plan/idiom/phase_c_recon.py --limit 40
    python3 plan/idiom/phase_c_recon.py --bucket B --module src/math.rs
    python3 plan/idiom/phase_c_recon.py --dead-wrappers
    ```

    The script is heuristic, not authoritative. It deliberately defaults
    broad scans away from `src/scan.rs`; use `--module src/scan.rs` or
    `--include-scan` only when you are intentionally investigating scanner
    work.

2.  **Codemod only buckets A/B.** Regex/scripted edits are encouraged
    for classified-safe work: legacy call-site rename to `_global`,
    removal of `#[no_mangle] extern "C"` from no-C-caller functions,
    wrapper skeleton insertion, narrow-state parameter threading, obvious
    accessor-to-method rewrites, and import cleanup. Keep the codemod
    manifest small enough to review in the diff: function name, required
    state borrows, caller files, and the allowed rewrite rules.

Do not apply blind regexes to buckets C/D. The migration is repetitive,
but the correctness hazards are semantic: borrow lifetime, abort
sentinels, DVI byte ordering, output flushing, and argument evaluation
under `with_state`. A good session can still be large: classify a whole
cluster, run the codemod for the A/B subset, then manually finish the C
wrappers in the same coherent commit.

## 5. Cold error/abort paths (`overflow`, `confusion`, `runaway`)

These are `-> !` longjmp paths reached only on capacity exhaustion /
internal-consistency failure. They are `with_state`-bearing (they print
through the migrated `selector`/`help_line` state) and they are **never
reached in any passing test or normal run**. But correctness on a real
overflow must be preserved, so a threaded allocator holding a borrow
cannot simply call `overflow()` (it would panic instead of printing the
capacity message).

Resolution for the first slice (surgical, behavior-preserving):

-   Thread the allocators as `&mut MemState` (narrow), **not** the full
    `&mut EngineState`. They do not need any other cluster on the hot
    path.
-   On the exhaustion branch, instead of calling `overflow()`/`runaway()`
    *inside* the threaded body (under the borrow), **return an overflow
    sentinel** and let the thin bridge-down wrapper invoke
    `overflow()`/`runaway()` **after** the `with_state_mut` borrow has
    dropped. `overflow()` never returns, so the sentinel is only ever
    observed by the wrapper, never by real callers.

Do **not** thread `overflow`/`confusion`/`runaway` themselves in the
first slices — they pull in the entire print/diagnostic subtree and are
a much later (or never, given they abort) target. Keep them bridge-based.

## 6. Narrow borrows and the `MemState` method surface

`with_state`/`with_state_mut` lock the whole state, but a threaded
function should take the **narrowest** borrow it needs so the compiler
can prove disjointness (§10 borrow-checker friction):

-   `free_node(mem: &mut MemState, …)`, not `&mut EngineState`.
-   A function that needs both arenas takes `mem: &mut MemState,
    eqtb: &mut EqtbState` (disjoint fields of `EngineState` borrow
    independently — `let s = &mut *state; f(&mut s.mem, &mut s.eqtb)`).
-   Never `.clone()` an arena to dodge the borrow checker (A.2/A.4).

Give `MemState` inherent methods mirroring the `globals.rs` accessors
(`mem.b32s0(p)`, `mem.set_b32s1(p, v)`, `mem.word(p)`, the `avail`/`rover`/
`mem_end` scalars). The `unsafe` `MemoryWord` union *read* projection
stays inside those methods (one audited site each); union *writes*
through a `&mut` index are safe (session 41). These methods are what
threaded bodies call instead of the free-function bridge accessors. The
free-function `globals.rs` accessors stay until their last caller is
threaded, then are deleted.

## 7. The recipe (per Phase-C slice)

Run from the crate root. One function (or a tightly-coupled function
group) plus **all** its call sites per commit.

**1. Pick the slice** from `STATUS.md` "Next action" / the §9 sequence
below. Confirm its callee set is closeable per §4 (no hot-path (c)
calls).

**2. Recon the call graph (before touching anything):**

```bash
# every call site, per file (these become bridge-down wraps or pass-throughs)
grep -rcE '\bFN\(' src/*.rs | grep -v ':0'
# does any call site already sit inside a with_state* closure? (nest hazard)
grep -rnB3 '\bFN\(' src/*.rs | grep -iE 'with_state'
# what with_state-bearing fns does FN call internally? (must be (a) or cold)
#   read FN's body; classify each call per §4
```

If a call site is already inside a `with_state*` closure, that wrap must
be restructured (the closure can't call a function that re-borrows) —
handle it explicitly, do not wrap blind.

**2a. Classify the rewrite shape (§4a).** Before editing, decide which
calls/functions are A/B codemod-safe and which are C/D manual. Run
scripted or regex rewrites only for the A/B subset, and review the diff
before doing the semantic C/D work.

**3. Add the method surface** (§6) on the target sub-struct in
`state.rs` if not present (`impl MemState { … }`), mirroring the exact
field widths of the `globals.rs` accessors (parity hazard: a typed
accessor that sign-extends differently is a silent break — §10).

**4. Thread the function.** Change its signature to take the narrow
`&mut` borrow; drop `#[no_mangle]`/`extern "C"` (it is now internal);
rewrite the body to call the §6 methods / touch fields directly; replace
any cold-path `overflow()`/`runaway()` with the sentinel return (§5).
De-`unsafe` what the move makes safe (coordinate with workstream 03).

**5. Bridge-down every call site** (§3): legacy callers get
`with_state_mut(|s| FN(&mut s.mem, …))`; already-threaded callers pass
their `&mut` through. Handle the cold sentinel at the wrapper.

**6. Delete now-dead bridge accessors.** If this slice removed the last
caller of a `globals.rs` `with_state` accessor, delete that accessor in
the same commit. (Most will survive until later slices — that's fine.)

**7. Build + full dashboard. Never regress.**

```bash
cargo check -p tectonic_engine_xetex
cargo test -p tectonic --test tex-outputs --test trip \
  --test engine_rust --test latex_goldens     # expect 29 / 2 / 7 / 4
```

**8. Update docs + commit** (`SESSION_LOG.md`, `STATUS.md` execution
list + metrics + Next action + Last-updated), engine-crate files only
(`src/*.rs`, `xetex/xetex-ini.c`, `plan/idiom/*`), prefix
`engine_xetex:`.

## 8. Metrics this phase moves (record in `STATUS.md`)

Phase C is where the headline numbers fall, so track them each slice:

-   `#[no_mangle]` exports: trend toward boundary-only (each threaded
    internal fn drops its export).
-   `unsafe fn` / raw `unsafe {}` blocks: trend down sharply as bridge
    accessors and union derefs collapse into safe slice indexing.
-   thin `globals.rs` accessors: trend to 0 (deleted as their last
    callers thread).
-   `with_state`/`with_state_mut` call sites: trend to 0; the
    thread-local is deleted when this hits 0.

They need not hit 0 until the end of Phase C / workstream 03–04, but
must never go **up**.

## 9. First subsystem to thread (recon-justified, session 60)

**Pick: the `mem` allocator core — `free_node`, `flush_list`,
`get_avail`, `get_node` — threaded as `&mut MemState`.**

Why this subsystem (per §5 step 4 of `01-state-and-memory.md`, now
validated):

-   It is the documented §5-step-4 "natural first consumer" of the new
    arena, and it has the tightest golden/unit coverage (`src/mem.rs`
    tests + every TRIP/format path exercises it).
-   Recon (§4 table) shows it is **closeable now**: `free_node`/
    `flush_list` touch only `mem` with zero `with_state` calls;
    `get_avail`/`get_node` add only raw-global reads (safe under a
    borrow) plus the **cold** `overflow`/`runaway` path, handled by the
    sentinel technique (§5). This refutes the session-59 blocker.
-   It bottoms out the dependency graph: almost every other subsystem
    calls these four. Threading them first means later subsystems can
    pass `&mut s.mem` straight through instead of re-wrapping — so this
    is the foundation that makes top-down threading of `scan.rs`,
    `math.rs`, `linebreak.rs`, etc. tractable.

It is **not** a tiny slice — there is no tiny first slice, because every
self-contained subsystem bottoms out in these allocators. Call-site
counts (session 60):

| fn | total call sites | scan.rs | others |
|---|---:|---:|---|
| `free_node` | ~96 | 46 | math 16, linebreak 13, shipout 8, save 4, pagebuilder 4, input 2, mem 2, ini 1 |
| `flush_list` | ~21 | 13 | shipout 2, input 2, mem 2, linebreak 1, ini 1 |
| `get_avail` | ~83 | 60 | shipout 7, linebreak 5, ini 4, math 3, mem 2, save 1, input 1 |
| `get_node` | ~86 | 40 | mem 17, math 8, save 7, linebreak 6, shipout 4, ini 3, pagebuilder 1 |

**Commit sequence** (one function + all its sites per commit, easiest
first to prove the pattern before the cold-path complication):

1.  `free_node` — purest (mem-only, no cold path); ~96 sites, all
    bridge-down wraps. Establishes the §3 bridge-down pattern and the §6
    `MemState` method surface.
2.  `flush_list` — same shape, smaller (~21 sites).
3.  `get_avail` — introduces the §5 cold-path sentinel.
4.  `get_node` — cold-path sentinel + the safe raw-global reads
    (`line()`/`cur_input_synctex_tag()`); largest internal logic.

After the core: the `new_*` node constructors (`new_null_box`,
`new_rule`, `new_glue`, …) thread next — they call `get_node`/`get_avail`
and so become pass-through callers, shrinking the bridge-down wrapper
count. Then climb the graph subsystem by subsystem (likely
`save.rs` → `pagebuilder.rs`/`shipout.rs` → `math.rs`/`linebreak.rs` →
`scan.rs` last, since it is both the largest and the most-coupled).

**Sequencing note vs. workstream 02 (module split):** 02 is still
`PLANNED`. Phase C does not need 02 to start (the allocator core stays in
`src/mem.rs`), but the later climb into `scan.rs` should be coordinated
with 02 so a function is threaded, de-`#[no_mangle]`'d, de-`unsafe`'d,
and moved to its final module together rather than in four passes. Draft
02 to READY before threading reaches the `scan.rs` decomposition.

## 10. Recurring gotchas (carried from §11 + new for Phase C)

-   **Whole-`RefCell` borrow:** `with_state*` locks all of
    `EngineState`, not one field. The §4 nest-panic rule is about *any*
    second `with_state`, even on a "different" cluster.
-   **Raw vs. thread-local accessors are different hazards:** `line()` /
    `cur_input_synctex_tag()` (and the rest of the deferred `mod raw`
    set) are `unsafe { raw::… }`, safe to call under a live borrow;
    migrated-cluster accessors are not. Classify, don't assume.
-   **Field width parity:** mirror the exact C field width in every
    `MemState` method; a differing sign-extend/truncate is a silent
    parity break (lean on TRIP).
-   **`MemoryWord` union:** reads need the audited `unsafe` projection;
    writes through `&mut` index are safe (session 41).
-   **Disjoint borrows, not clones:** split `&mut s.mem` / `&mut s.eqtb`;
    never `.clone()` an arena (§10 of `01-state-and-memory.md`).
-   **Don't stack-own `EngineState`** across the setjmp boundary (§2/§7).
-   **Cold paths abort, don't return:** the overflow sentinel is an
    artifact of the borrow; never let it leak to a real caller.
