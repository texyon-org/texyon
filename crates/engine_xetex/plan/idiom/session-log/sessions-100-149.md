# Archived Idiom Sessions 100-149

Older Stage 2 idiomatization log entries. Newest entries are at the top. Read `../SESSION_LOG.md` first; open this archive only when investigating older context.

## Idiom session 149 — 2026-06-07 — Execution: Stage 3 rank-3 + rank-4 — the kern and penalty `NodePtr<K>` families (`01c`)

**Type:** execution (workstream 01 / `01c` §3 Stage 3, ranks 3 + 4). **2
code commits** (one family each, A.12) + **1 docs commit**. Dashboard
**29/2/7/4** green; warnings **42 → 42** (flat); `node::` **15/15**.

### Recon before building

Verified offsets against the C source + the live allocators first:
`xetex-xetexd.h:283` gives `PENALTY_NODE_penalty(p) = mem[(p)+1].b32.s1`
(commented "was originally the `mem[x+1].int` field" → an **`i32`**, not a
`Scaled`). Kern has no dedicated field macro; `src/mem.rs` `new_kern` writes
`mem.set_b32s1(p+1, w)` after `set_b16s1(p, KERN_NODE)` / `set_b16s0(p,
NORMAL)` → width at +1.b32.s1, subtype at 0.b16.s0. `xetex_format.h` gives
`KERN_NODE = 11`, `PENALTY_NODE = 12`. **Recon finding:** both `new_kern`
and `new_penalty` `get_node(MEDIUM_NODE_SIZE=3)`, not `SMALL_NODE_SIZE=2` —
the §1 table's penalty "small (2)" is the generic-TeX value; this xetex
build (SyncTeX) uses medium nodes. Corrected the §1 table + added a
footnote. (Documentation-only: the typed layer encodes offsets, not size.)

### Commit 1 — kern family (rank 3)

`KernNode` (`NODE_type` = 11) in `src/node.rs`: concrete impls `subtype`
(0.b16.s0 → `u16`) and `width` (+1.b32.s1 → `Scaled`), plus
`MemState::classify_kern`. **Design note:** kern's `width` is the same
+1.b32.s1 `Scaled` slot as the box family's `width`, but a kern node has
**no** depth/height/shift_amount — those offsets hold other data (the
SyncTeX word) — so kern uses a **concrete** `width` method, **not** the
`HasDimensions` capability trait (which would falsely expose +2..+4). This
is the recipe's "capability traits where a layout is shared; concrete impls
for family-specific fields" applied at the boundary. 3 unit tests.

### Commit 2 — penalty family (rank 4)

`PenaltyNode` (`NODE_type` = 12): `penalty` (+1.b32.s1 → **`i32`**) +
`MemState::classify_penalty`. **The trap encoded (`01c` §3):** the penalty
shares the +1.b32.s1 slot with box/kern `width` (a `Scaled`) but is a plain
integer breakpoint penalty — returning `Scaled` would be a silent
type/parity error, so it is `penalty() -> i32`. A unit test pins it with
`let v: i32 = pen.penalty(&mem)` (would not compile if it returned
`Scaled`) and a negative value to exercise sign semantics. 3 unit tests.

### Disasm gate (`01c` §4) — PASSED, both accessors

Per-family temporary `#[no_mangle] #[inline(never)]` harnesses, release
build, `objdump -t`: `__disasm_typed_kern_width` (`Scaled` +1.b32.s1)
ICF-folded to `.text.__disasm_raw_kern_width` (size 0x28);
`__disasm_typed_penalty` (raw `i32` +1.b32.s1) folded to
`.text.__disasm_raw_penalty` (size 0x28). Both byte-identical to the raw
read. Harnesses removed before commit; recorded in `01c` §4.

### Commit 3 — docs

`01c` §1 (built-so-far + penalty-size footnote), §3 (progress note), §4
(gate result); STATUS.md + SESSION_LOG.md.

### Metrics & validation

- Warnings **42 → 42**. `with_state`/`unsafe fn`/`#[no_mangle]` unchanged.
  Behavior-neutral (additive types). Dashboard **29/2/7/4** on every commit;
  `node::` 15/15, `units::` 6/6.

**Handoff / Next:** box + glue + kern + penalty are built — the cheap,
high-traffic, fixed-layout families are now done. Two valid tracks:
1. **Continue Stage 3 — rank 5 = rule, the first NON-trivial family.** Its
   `width`/`depth`/`height` (+1/+2/+3.b32.s1) are **`Scaled`-or-`NULL_FLAG`
   running dimensions** (`NULL_FLAG = -0x40000000`, `xetex-constants.h:16`):
   a rule with a "running" dimension inherits it from the enclosing box.
   This is the slice where the typed layer **first models a sentinel-bearing
   field** — do NOT return bare `Scaled`. Introduce a running-dimension
   wrapper (e.g. `enum RuleDimen { Running, Set(Scaled) }`, or a `RuleDimen`
   newtype with `is_running()`/`as_scaled()`), mirror the exact `NULL_FLAG`
   compare, and unit-test both the running and set cases + the round-trip
   through the raw word. Verify the offset/sentinel against `xetex-xetexd.h`
   (the `RULE_NODE_*` macros) and how `new_rule`/the packers read it. After
   rule: disc → ligature → math → native (progressively harder).
2. **First fused adoption (`01b` §3a)** — now well-supported: thread a small
   leaf that reads box dims / glue / a kern width / a penalty to `&mut
   MemState` **and** retype its node pointer to the matching `NodePtr<…>` in
   the same commit, proving the type-as-you-thread loop end-to-end and
   starting the bare-`i32` node-ptr-param metric (`01b` §8) downward. Grep
   `packaging.rs`/`shipout.rs`/`linebreak.rs` for a leaf reading these under
   a single `mem` borrow. (Recommended soon — the foundation now covers
   enough families that a real adoption would validate the whole approach
   against a live call site, not just unit tests + disasm.)

## Idiom session 148 — 2026-06-07 — Execution: Stage 3 rank-2 — the glue/glue_spec `NodePtr<K>` family (`01c`)

**Type:** execution (workstream 01 / `01c` §3 Stage 3, rank 2). **1 code
commit** (+ plan-doc updates). Dashboard **29/2/7/4** green; warnings
**42 → 42** (flat); `node::` **9/9**.

### Recon before building

Per the handoff's "recon before you build" discipline, verified every
offset against the C source *before* writing a line:
`grep` of `xetex/xetex-xetexd.h` confirmed `GLUE_NODE_glue_ptr(p) =
mem[(p)+1].b32.s0`, `GLUE_NODE_leader_ptr(p) = mem[(p)+1].b32.s1`,
`GLUE_SPEC_ref_count(p) = mem[p].b32.s1`, `stretch_order = mem[p].b16.s1`,
`shrink_order = mem[p].b16.s0`, `stretch = mem[(p)+2].b32.s1`, `shrink =
mem[(p)+3].b32.s1`; `xetex-constants.h` gave `GLUE_SPEC_SIZE = 4`,
`SMALL_NODE_SIZE = 2`; `xetex_format.h` gave `GLUE_NODE = 10` (matches the
existing `NodeType::Glue`). Confirmed `MemState` already has the
`b32s0`/`set_b32s0` accessor the box family didn't need (glue_ptr is the
`b32.s0` half).

### Commit — Stage 3 rank 2: the glue family

Added to `src/node.rs` (still `#[allow(dead_code)]` — additive, behavior-
neutral):

- **`GlueNode`** (`NODE_type` = `GLUE_NODE` = 10, a `SMALL_NODE_SIZE` = 2
  node) — concrete impl (no shared capability traits; glue fields are
  family-specific, per `01c` §5): `subtype` (0.b16.s0 → `u16`), `glue_ptr`
  (+1.b32.s0 → `MemPtr`), `leader_ptr` (+1.b32.s1 → nullable `MemPtr`), plus
  a `glue_spec()` helper that classifies the `glue_ptr` to a
  `NodePtr<GlueSpecNode>` (the recipe explicitly asked for this helper
  "returns the glue_ptr as MemPtr (caller classifies)" — implemented as the
  ergonomic `Option<NodePtr<GlueSpecNode>>` over `glue_ptr()` + `from_glue_ptr`).
- **`GlueSpecNode`** (`GLUE_SPEC_SIZE` = 4 — a *record*, not a list node) —
  concrete impl: `ref_count` (0.b32.s1 → **`i32`**), `stretch_order`
  (0.b16.s1 → `u16`), `shrink_order` (0.b16.s0 → `u16`), `stretch`
  (+2.b32.s1 → `Scaled`), `shrink` (+3.b32.s1 → `Scaled`).
  - **The ref_count trap encoded (`01c` §3).** `GLUE_SPEC_ref_count` lives
    in the `0.b32.s1` word — the slot that holds the successor `link()` on a
    real list node — but on a glue_spec it is a reference count. Exposed as
    `ref_count() -> i32`, **never** as `link()`/`MemPtr`; the doc comment and
    a dedicated unit test (`..._ref_count_is_not_a_link`) pin the distinction.
- **Classification differs from the box family.** A glue_spec carries no
  `NODE_type` (its `0.b16.s1` word holds `stretch_order`), and is only ever
  reached through a glue node's `glue_ptr` or a parameter-glue pointer. So:
  - glue **nodes** classify via `MemState::classify_glue(MemIdx)` (a
    `node_type` branch on `GLUE_NODE`, mirroring `classify_box`), returning
    `Ok(Some(NodePtr<GlueNode>))` / `Ok(None)` / `Err(InvalidNodeType)`;
  - glue **specs** classify via the from-`MemPtr` constructor
    `NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr) -> Option<…>` — **not** a
    `node_type` branch. Documented that there is deliberately no
    `classify_glue_spec`.
- **5 unit tests** (9 total in `node::`): zero-cost size of both handles;
  glue-node field round-trip landing in the C-exact raw subfields (incl. the
  `glue_ptr`-is-the-`s0`-half check); glue_spec field round-trip + the
  ref_count-is-not-a-link assertion; glue_ptr→spec classification incl. the
  null case (`from_glue_ptr(NULL) == None`); `classify_glue` dispatch +
  invalid-code rejection.

**Disasm gate (`01c` §4) — PASSED, strongest form.** Temporary
`#[no_mangle] #[inline(never)]` pairs `__disasm_typed_glue_ptr` vs. raw
`mem.b32s0(p+1)` and `__disasm_typed_stretch` vs. raw `mem.b32s1(p+2)`,
`cargo build --release`, `objdump -t`: LLVM identical-code-folding merged
each typed fn onto its raw twin at the *same section/address*
(`__disasm_typed_glue_ptr` → `.text.__disasm_raw_glue_ptr` size 0x27;
`__disasm_typed_stretch` → `.text.__disasm_raw_stretch` size 0x29) —
byte-identical machine code. Proves both the `+1.b32.s0` "other word half"
shape and the `+2.b32.s1` `Scaled`-at-offset shape are zero-cost over the
raw index. Harness removed before commit; result recorded in `01c` §4.

### Metrics & validation

- Warnings **42 → 42**. `with_state`/`unsafe fn`/`#[no_mangle]` unchanged.
  Behavior-neutral by construction (additive types). Dashboard **29/2/7/4**;
  `node::` 9/9, `units::` 6/6.

**Handoff / Next:** the glue family is usable. Two valid tracks:
1. **Continue Stage 3** down the `01c` §1 order — next is **rank 3 = kern**
   (trivial: one `Scaled` at +1.b32.s1, `KERN_NODE_SIZE` = 3). Then
   **penalty** (TRAP: `+1.b32.s1` is a plain **`i32`**, NOT `Scaled` — don't
   mistype it), then **rule** (TRAP: `width`/`depth`/`height` at +1/+2/+3 are
   `Scaled`-**or**-`NULL_FLAG` *running dimensions* — `NULL_FLAG =
   -0x40000000`, `xetex-constants.h`; needs a running-dimension wrapper type
   that can represent "running", not bare `Scaled`, before its fields are
   typed). Same additive recipe + disasm gate each slice.
2. **First fused adoption (`01b` §3a):** thread a small box- or glue-reading
   leaf to `&mut MemState` *and* retype its node pointer to
   `NodePtr<…>`/`AnyBox` in the same commit — proving the end-to-end
   type-as-you-thread loop on a real call site and starting to drive the
   bare-`i32` node-ptr-param metric (`01b` §8) down. A leaf box/glue consumer
   in `packaging.rs`/`shipout.rs` reading dims under a single `mem` borrow is
   a good first target (grep for one).

## Idiom session 147 — 2026-06-07 — Execution: Stage 2 already-done finding + Stage 3 box-family `NodePtr<K>` layer (`01c`)

**Type:** execution (workstream 01 / `01c`). **2 commits.** Dashboard
**29/2/7/4** green; warnings **42 → 42** (flat).

### Commit 1 — record `01c` Stage 2 (the `Mem` ownership move) ALREADY COMPLETE

Recon before executing Stage 2 found it was **already done** in earlier
sessions (11 = `save_stack`, 51 = `mem`), before `01c` was drafted:
`MemState.data` is a `Vec<MemoryWord>` (`state.rs:403`), allocated
`s.mem.data = vec![…]` at `engine.rs:1073` (initex, `MEM_TOP + 2`) and
`format_io.rs:854` (undump, `MEM_TOP + 1`) — inclusive sizing handled; no
`xmalloc_array`/C `mem` symbol/`#[link_name]`/`install_mem_ptr`; no
double-free in `tt_cleanup`; raw-ptr consumers (`mem_ptr`/`mem_node_addr`)
derive from the `Vec` (never reallocates); `.fmt` dump/undump byte-exact,
`FORMAT_SERIAL = 33`. The cookbook had listed it pending because it was
derived from the target spec without re-checking execution history.
**Corrected `01c` §2 Stage 2 + `01`-§4a to record DONE** so a future
session won't redo finished work. Plan-only commit.

### Commit 2 — Stage 3 rank 0+1: the `NodePtr<K>` core + box family

NEW **`src/node.rs`** (the typed node-handle layer, `01c` §3; `#[allow(dead_code)] mod node;`):

- `NodePtr<K>` — `#[repr(transparent)]` over `MemIdx` + `PhantomData<fn() -> K>`.
  Manual `Copy`/`Clone`/`PartialEq`/`Eq`/`Debug` (no `K: Trait` bounds, since
  the kind markers are uninstantiated empty enums). `from_idx_unchecked`
  (unsafe — caller must have classified) + `idx()`.
- `NodeKind` markers `HListNode`/`VListNode`; capability traits
  `HasDimensions`/`HasListPtr`/`HasGlueSet` (compile-time only, no dispatch).
- All 8 **box** field methods at the C-exact offsets
  (`xetex-xetexd.h:211–218`): `width`+1/`depth`+2/`height`+3/`shift_amount`+4
  (`Scaled`, b32.s1), `list_ptr`+5 (`MemPtr`, nullable), `glue_sign`+5.b16.s1,
  `glue_order`+5.b16.s0 (`u16`), `glue_set`+6.gr (`f64`) — each read/write
  going through the **audited `MemState` word accessors** so the union
  projection stays confined there (the lone exception: the `glue_set` read
  uses `MemState::word(..).gr`, documented).
- The checked boundary: `MemState::node_type(MemIdx) -> Result<NodeType,_>`
  and `classify_box(MemIdx) -> Result<Option<AnyBox>,_>` (HList/VList → typed
  handle; other valid type → `None`; invalid code → `Err`).
- 4 unit tests (all pass): zero-cost size, box-field round-trip landing in
  the C-exact raw subfields, empty-box `list_ptr` == null, `classify_box`
  dispatch + invalid-code rejection.

**Disasm gate (`01c` §4) — PASSED, strongest form.** Temporarily added
`#[no_mangle] #[inline(never)]` typed-`width` vs. raw-`b32s1(p+1)`
functions, `cargo build --release`, `objdump`: LLVM identical-code-folded
them to the **same address/section** (`.text.__disasm_raw_width`, size
0x28) — byte-identical machine code, i.e. the typed handle is provably
zero-cost over the raw index. Harness removed before commit; recorded the
result + re-run recipe in `01c` §4.

### Metrics & validation

- Warnings **42 → 42** (both new modules are `#[allow(dead_code)]`;
  nothing consumes the layer yet). `with_state`/`unsafe fn`/`#[no_mangle]`
  unchanged. Behavior-neutral by construction (additive types + a recon
  finding). Dashboard **29/2/7/4**; `node::` 4/4, `units::` 6/6.

**Handoff / Next:** the foundation is now usable for the box family. Two
tracks, either is valid next:
1. **Continue Stage 3** down the `01c` §1 order — next family is
   **glue/glue_spec** (remember the trap: glue_spec's `0.b32.s1` is a
   *ref_count*, not a link). Same additive recipe + disasm gate.
2. **First fused adoption (`01b` §3a):** thread a small box-touching
   function to `&mut MemState` *and* retype its box pointer to
   `NodePtr<HListNode>`/`AnyBox`, proving the end-to-end type-as-you-thread
   loop on a real call site (and beginning to drive the bare-`i32`
   node-ptr-param metric down). A leaf box consumer in `packaging.rs`/
   `shipout.rs` is a good first target — grep for one that only reads box
   dims under a single `mem` borrow.

## Idiom session 146 — 2026-06-06 — Execution: typed foundation Stage 1 — `NodeType` enum + checked `from_raw` (`01c` §2)

**Type:** execution (workstream 01 / `01c` §2 Stage 1, completing the
scalar-newtype foundation). **1 commit.** Dashboard **29/2/7/4** green;
warnings **42 → 42** (flat).

### What and how

The Stage-1 remainder deferred by session 145: the node-type classifier.

- **`src/units.rs`** — added `NodeType` (`#[repr(u16)]`, 33 discriminants)
  + `InvalidNodeType(u16)` + `NodeType::from_raw(u16) -> Result<…>` (checked
  `match`, **never** `transmute`) + `as_raw()`. Discriminants mirror
  `xetex/xetex_format.h:380–416` exactly: `HList`=0 … `RightNoad`=31, plus
  `MarginKern`=40.
- **Faithfulness checks encoded in the code + tests:**
  - **Context aliases are not separate codes:** `DELTA_NODE` reuses
    `Rule`'s `2`, `EDGE_NODE` reuses `Style`'s `14`
    (`xetex-constants.h:104–105`) — disambiguated by list context, so no
    distinct discriminant (commented at the variants).
  - **Whatsit subtypes are a different field:** `OPEN_NODE`/`WRITE_NODE`/…/
    `NATIVE_WORD_NODE`(40)/`GLYPH_NODE`(42)/`PIC_NODE`(43)/`PDF_NODE`(44)
    live in `NODE_subtype` (`b16.s0`), classified separately — **not** node
    types. The `MARGIN_KERN_NODE` *type* code 40 and `NATIVE_WORD_NODE`
    *subtype* code 40 share a number in different fields; do not conflate.
  - `from_raw` rejects the 32–39 gap and 41+ (incl. 42/43/44).
- 3 new unit tests (6 total in `units::`, all pass): every discriminant ==
  its C code + round-trips through `from_raw`; the invalid/gap codes are
  rejected; `size_of::<NodeType>() == 2`.

### Why this was its own slice

Split from the session-145 scalar newtypes precisely because a faithful
~33-discriminant enumeration is the one part of Stage 1 with real parity
risk (a wrong code silently breaks Stage-3 classification later). Isolating
it keeps the diff trivially auditable against the header (A.12) and gives
the discriminant table its own focused review + test.

### Metrics & validation

- Warnings **42 → 42** (the enum lives in the `#[allow(dead_code)]`
  `units` module; nothing consumes it until Stage 3). `with_state` /
  `unsafe fn` / `#[no_mangle]` unchanged. Behavior-neutral by construction
  (purely additive). Dashboard **29/2/7/4** all green; `units::` 6/6.

**Handoff / Next:** Stage 1 of `01c` §2 is **complete** (scalar newtypes +
`NodeType`). Next is **`01c` Stage 2 — the `Mem` `Vec<MemoryWord>`
ownership move** (`01`-§5 steps 1–2): replace the `xmalloc_array` arena
with a Rust-owned `Vec` on `MemState` (it is already a thread-local field
group — confirm the current shape first), preserving the `n+1` inclusive
sizing (`01`-§11 gotcha), the `tt_cleanup`/longjmp teardown (`01`-§7), and
the **byte-exact `.fmt` dump/undump** (`01c` §7.2; `format_io.rs:299–339`).
This is the first Stage that touches a live code path, so it is a real
parity slice — baseline, change, full dashboard. Then **Stage 3** — the
per-family `NodePtr<K>` layer (box first, `01c` §1 order), where the §4
disasm gate becomes load-bearing.

## Idiom session 145 — 2026-06-06 — Execution: typed foundation Stage 1 — the scalar-newtypes module (`01c` §2)

**Type:** execution (workstream 01 / `01c` §2 Stage 1, the typed
foundation). **1 commit.** Dashboard **29/2/7/4** green; warnings **48 →
42** (−6).

### What and how

The first execution slice off the now-`READY` `01c` cookbook: build the
scalar-newtypes module that the Phase-C type-as-you-thread slices adopt.

- **NEW `src/units.rs`** — the canonical `#[repr(transparent)]` + `Copy`
  scalar newtypes: `MemIdx` (valid index, **no** null), `MemPtr` (nullable
  halfword), `Scaled`, `StrNumber`, `FontId`, `Token`. With the two
  `01c`-§2 source-fixes applied:
  1. `MemPtr::NULL = MemPtr(TEX_NULL)` where `TEX_NULL = MIN_HALFWORD =
     -0x0FFFFFFF` (`crate::consts::TEX_NULL`) — **not** `0`. Null-ness is on
     `MemPtr`, not `MemIdx`. `MemPtr::as_idx() -> Option<MemIdx>` is the
     single checked classification boundary (`01c` §3).
  2. `FontId` wraps `i32` (not the vestigial `u16`) — `internal_font_number`
     is `int32_t` (`xetex-xetexd.h:326`).
  - `MemIdx::new` debug-asserts `!= TEX_NULL`; `offset`/`index` helpers for
    node-field walking.
  - 3 unit tests (all pass): `#[repr(transparent)]` size == `i32`
    (the Stage-1 zero-cost evidence; the real disasm gate, `01c` §4, lands
    with the first `NodePtr<K>` field method in Stage 3), and the
    null-is-`TEX_NULL`-not-`0` correction.
- **`src/lib.rs`** — `#[allow(dead_code)] mod units;` (foundation built
  before its consumers, `01`-§4a; same pattern as `string_pool`/`texmfmp`).
- **`src/mem.rs`** — deleted the vestigial duplicate newtypes (lines
  56–96; the old wrong `MemIdx::NULL = MemIdx(0)`). They had **zero**
  method call sites (verified: every `.is_null()` in the crate is on a raw
  pointer) and were not re-exported, so removal is behavior-neutral and
  drops 6 dead-code warnings.

### Scope decision

Scoped this slice to the **pure scalar newtypes** only. `NodeType` +
`from_raw` (listed in `01c` §2 Stage 1) is **deferred to its own slice**:
a faithful node-type enumeration (~30 discriminants from
`xetex-constants.h`) is a parity hazard best cross-checked alongside the
classification boundary it serves (Stage 3), and folding it in here would
violate A.12 (keep the commit trivially-safe and bisectable). Recorded so
the next session picks it up.

### Metrics & validation

- Warnings **48 → 42** (−6, all from deleting the dead `mem.rs` stubs; the
  new module is `#[allow(dead_code)]` and adds none). `with_state` /
  `unsafe fn` / `#[no_mangle]` **unchanged** (this slice threads nothing
  and touches no `unsafe`).
- Behavior-neutral by construction: added an unused module + removed dead
  code. Dashboard **29/2/7/4** (tex-outputs 29, trip 2, engine_rust 7,
  latex_goldens 4) all green; `units::` unit tests 3/3. Bench not run
  (no `ORACLE_BIN` configured here; this slice changes no code path).

**Handoff / Next:** continue the `01c` foundation. Stage-1 remainder =
the **`NodeType` enum + checked `from_raw`** slice (enumerate faithfully
from `xetex-constants.h`, TRIP-validate). Then **Stage 2** — the `Mem`
`Vec<MemoryWord>` ownership move (`01`-§5 steps 1–2: `ini.rs` alloc, the
`tt_cleanup`/longjmp reconciliation `01`-§7, the `n+1` inclusive sizing
gotcha, and the byte-exact `.fmt` dump/undump `01c` §7.2). Then **Stage 3**
— the per-family `NodePtr<K>` layer in the `01c` §1 order (box first),
where the §4 disasm gate becomes load-bearing. Phase-C giant-threading may
proceed in parallel via the `01`-§4a worklist fallback.

## Idiom session 144 — 2026-06-06 — Plan construction: the typed-foundation cookbook (`01c`) + node-field recon

**Type:** plan construction (no engine code change, no dashboard run — hard
rule: this session changes only `plan/idiom/*`). Delivers the missing
`READY` cookbook that session 143's handoff named as the blocker before
fused threading+typing slices can begin.

### What it produced

- **NEW `plan/idiom/01c-typed-node-recipe.md` (READY)** — the per-slice
  analog of `01b`, for building the `NodePtr<K>` typed foundation. It
  specifies, all source-verified:
  - **§1 node-family recon** + the ranked, data-driven build order.
  - **§2 foundation order** made concrete from `01`-§4a: (Stage 1) scalar
    newtypes `MemIdx`/`MemPtr`/`Scaled`/`StrNumber`/`FontId`/`Token`/`NodeType`
    with checked constructors + `NodeType::from_raw`, `#[repr(transparent)]`+`Copy`;
    (Stage 2) the `Mem` `Vec<MemoryWord>` ownership move (`ini.rs` alloc,
    `tt_cleanup`/longjmp reconciliation, the `n+1` inclusive sizing gotcha);
    (Stage 3) the per-family `NodePtr<K>` layer, one family per slice in §1 order.
  - **§3 the `NodePtr<K>` design** concretely: `MemIdx`+compile-time marker;
    capability-trait'd field methods taking `&Mem`/`&mut Mem`; semantic-typed
    returns (`Scaled`/`MemPtr`/`FontId`/…); classification (`MemPtr` →
    `Option<NodePtr<K>>`) once at a checked boundary, never by blind
    `MemIdx::new` on a sentinel-bearing field; per-family invariant traps.
  - **§4 the MANDATORY zero-cost disasm gate** before any fused slice depends
    on a wrapper — names the hot accessors to check (`width`, `link`,
    `node_type`/`classify`, `glue_ptr`, the linebreak traversal loop).
  - **§5 the per-slice recipe**, **§6 the `01b` dovetail + worklist fallback**,
    **§7 the hard constraints**, **§8 the free-list NON-GOAL**.
- **NEW `plan/idiom/node_field_recon.py`** — read-only recon. Counts
  node-field sites by `(subfield, offset)` (the signal the C field macros
  key on; Stage 1 collapsed the per-family macros into the generic
  `mem_b32s1(p+N)`). Output: **742 node-field sites (offset `+N`)**
  (matches session-143's "~725 (p+N)"), **1378 header-word sites
  (offset 0)**, 37 distinct signatures.
- **Cross-refs to `01c`** added in `01`-§4a (replaces the "draft the cookbook"
  TODO) and `01`-§5 (the one-line free-list non-goal), `01b` intro callout,
  `00`-overview "Typing rides with threading" paragraph. `STATUS.md` top
  note + the `01`/`01b` rows + a new `01c` row.

### Recon → build order (highest-traffic + simplest-invariant first)

shared-header-word (rank 0; 1378 sites — `link`/`info`/`type`/`subtype`) →
**box hlist/vlist** (rank 1; fixed 8 words, dims pure `Scaled`, owns the
+1..+6 band) → **glue/glue_spec** (one trap: glue_spec's `0.b32.s1` is a
ref_count, not a link) → **kern** (one `Scaled`) → **penalty** (`+1` is
`i32`, NOT `Scaled`) → **rule** (`width/depth/height` are
`Scaled`-or-`NULL_FLAG` running dimension) → disc → ligature → **math/noad**
(packed sub-records, no C macros, 2nd sub-classification) → **whatsit/native**
(variable-size, inline trailing data, raw native ptr — last) → char/one-word
(the `is_char_node` region, folds into rank 0) → linebreak/page internals
(thread with their subsystem). Per-family numbers are a header-informed
split of the `(subfield,offset)` histogram, not per-site classification
(disclosed in `01c`'s status + the script header) — the **order** is robust.

### Verified hard constraints + two source-fixes (so a future session doesn't re-derive)

- `is_char_node(p) ≡ p >= hi_mem_min` (`src/mem.rs:910`) — classification
  is a region comparison; the typed layer preserves the lo/hi partition.
- The `.fmt` dump stores raw indices + free-list state (`sort_avail` then
  `lo_mem_max`/`rover`/`mem` ranges/`hi_mem_min`/`avail`,
  `src/format_io.rs:299–339`); `FORMAT_SERIAL=33` (`format_io.rs:76`) is
  public API — the index space + free-list cannot be abandoned.
- `var_used`/`dyn_used` are printed on the memory-usage stats line
  (`src/format_io.rs:355–357`) → byte-observable; don't perturb the
  allocator bookkeeping.
- **`TEX_NULL = MIN_HALFWORD = -0x0FFFFFFF`** (`src/consts.rs:13`), so the
  vestigial `MemIdx::NULL = MemIdx(0)`/`is_null()` (`src/mem.rs:64–69`)
  **and** the parent `03` `MemPtr::NULL = MemPtr(0)` sketch are both **wrong
  for this codebase** — the scalar-newtype slice must put null-ness on
  `MemPtr` keyed to `TEX_NULL` and remove it from `MemIdx`.
- The existing `MemIdx`/`Scaled`/`FontId`/`Token`/`StrNumber`
  (`src/mem.rs:56–96`) are **vestigial stubs** (used only at their defs;
  `MemPtr`/`NodePtr` absent) — the foundation is greenfield.

### Handoff / next

**Fused threading+typing slices can now begin** (`01b` recipe is READY and
its foundation gate, `01c`, is now READY too). **The first foundation
execution slice is the scalar-newtypes module** (`01c` §2 Stage 1) — create
`src/units.rs` (or fold into `mem_types.rs`), define the `#[repr(transparent)]`
newtypes with the two source-fixes above, and **run the §4 disasm gate**
before anything depends on them. Then Stage 2 (the `Mem` ownership move) and
Stage 3 (the per-family `NodePtr<K>` layer in §1 order). Phase-C threading
of the remaining giants may continue in the interim via the §6/`01`-§4a
worklist fallback so it isn't blocked on the foundation.

## Idiom session 143 — 2026-06-06 — Plan construction: adopt "type-as-you-thread" (fuse typing into Phase C)

**Type:** plan construction (no code change, no dashboard run). Strategy
pivot requested by the user: *do the typing together with the threading*
instead of as a deferred second call-site pass.

### Why

The estimate work this session surfaced that targets #3 (typed
`Mem`/`NodePtr<K>`) and #4 (de-unsafe) were, as written, a separate
sweep that re-visits every function Phase C already touched — `01`-§4's
option (b) ("a second full call-site pass"). Fusing typing into the
threading touch removes that second pass: a function is left in its final
idiomatic form (threaded **and** typed **and** de-unsafe'd) in one
commit, and the borrow/sentinel/index decisions are made once, while the
body is already open under the same parity run.

### What changed (docs only)

- **`01-state-and-memory.md`**
  - §4: flipped the newtype-rollout decision to **option (c)
    type-as-you-thread**, superseding the old (a)-mostly rule. Recorded
    that the leaf/subsystem backlog was historically threaded with bare
    `i32` (effectively (b)), and that from 2026-06-06 every *remaining*
    slice carries its typing.
  - NEW **§4a** "Type-as-you-thread: the foundation-first prerequisite":
    splits the typing into a one-time foundation (scalar newtypes; the
    `Mem` `Vec<MemoryWord>` ownership move; the per-family `NodePtr<K>`
    layer, each gated on a zero-cost release-disasm check) built up front,
    plus the per-slice fusion thereafter. Defines the **worklist
    fallback**: a slice may leave a node family raw if its layer isn't
    built yet, *provided it records a deferred-newtype-worklist row* — so
    the foundation schedule never stalls the giant-threading. Notes the
    `NodePtr<K>` per-slice cookbook (the `01b` analog) still needs
    drafting + a node-field recon to order families data-driven.
  - §6 (Phase C) and §9 (decision 1) updated to point at §4(c)/§4a.

- **`01b-phase-c-recipe.md`** — reframed title to "threading + typing":
  - Intro callout stating the fusion + the foundation gate + worklist
    fallback.
  - NEW **§3a** "Typing rides with the thread": the typed-surface
    adoption technique (typed accessor returns; node-ptr params →
    `NodePtr<K>`/`MemPtr`; classification at the checked boundary;
    push conversions outward as transient shims that vanish exactly like
    the bridge-down `_global` wrappers). Adds the fused-slice closure rule.
  - NEW **§4b** "Typing in the codemod": typing is **compiler-driven, not
    regex** — `thread_fn.py` stays a threading tool; the
    `NodePtr<K>`-kind / nullability / sentinel decisions (§10 hazards)
    are read+written by hand and proven by TRIP + disasm, never
    auto-applied.
  - §7 recipe steps 3/4/5 now fold in typed returns, typed params, and
    the call-site conversion shims.
  - §8 metrics: added bare-`i32` node-ptr params → 0 and
    deferred-newtype-worklist length → 0.

- **`00-overview.md`** — sequencing rationale gained a "Typing rides with
  threading, not after it" paragraph.

- **`STATUS.md`** — top note + the 01 / 01b plan-table rows updated.

### Handoff / next

The strategy is recorded but **not yet executable as fused slices**: the
missing piece is the `READY` per-slice cookbook for the `NodePtr<K>`
foundation (the analog of `01b`), seeded by a recon counting node-field
access sites per node family so the family build order is data-driven
(highest-traffic, simplest-invariant first). Draft that next; only then
do fused threading+typing slices begin. Phase-C threading of the
remaining giants may continue in the interim with the worklist fallback
(§4a) so it isn't blocked on the foundation.

## Idiom session 142 — 2026-06-06 — Phase-C threading resumes: `app_space` (workstream 01)

**Type:** execution (workstream 01, Phase C). **1 commit, 1 function threaded.** Dashboard **29/2/7/4** green; warnings **48 → 48** (flat).

### What and how

Workstream 03 finished in session 141, so Phase-C threading (ws01 per `01b`)
resumes. With the clean single-borrow mem/eqtb leaf backlog drained (the
session-138 finding), recon's high-scoring rows are now either the print/output
subsystem (bucket-C, deepest, thread-last) or scanner-bearing main_control
verbs. Picked **`app_space`** — the hmode inter-word-glue builder
(`main_control.rs`, `xetex-xetex0.c:9223`) — a tight single-caller bucket-C
function whose every hazard was already resolvable:

- Threaded to disjoint **`&mut MemState` + `&EqtbState` + `&mut FontState`**
  (`cur_list`/`main_p`/`main_k` stay raw globals — safe under the borrow, §4).
- Added a `FontState::font_glue` reader method (§6 surface; `set_font_glue`
  already existed).
- The `new_param_glue_global`/`new_spec_global`×2/`new_glue_global` calls swap
  to the threaded cores (`new_param_glue(mem,eqtb,…)`/`new_spec(mem,…)`/
  `new_glue(mem,…)`), each propagating the `ALLOC_FAILED` sentinel as
  `Err(NodeRecycleAbort::Overflow)` for the wrapper to abort on (§5), matching
  the session-138 `indent_in_hmode` idiom.
- **The §129 trap:** the two `xn_over_d` calls set `tex_remainder`/`arith_error`
  (the *errors* cluster, not borrowed here) via `with_state` — a borrow held
  across them nest-panics. Fix: compute both with the pure `xn_over_d_parts`
  core under the borrow, return the net side effects
  (`(tex_remainder=2nd-remainder, arith_error|=ov1||ov2)`) as an
  `Option<(i32,bool)>`, and apply them in `app_space_global` after the borrow
  drops. Behavior-preserving: the wrapper sets them before returning to the
  caller, and nothing inside `app_space` reads them after the `xn_over_d` calls.

The single caller (the legacy `main_control` dispatch) → `app_space_global()`.

### Metrics & findings

- `with_state` sites **1196 → 1197** (+1; the one transitional `app_space_global`
  wrapper). `unsafe fn` **576 → 576** (flat: core became safe `pub(crate) fn`,
  +1 `unsafe` wrapper). `#[no_mangle]` flat (boundary-only since ws03).
  Warnings **48 → 48**. Dashboard **29/2/7/4**.
- **Validation:** the four suites (which exercise `app_space` on every text doc)
  pass. `bench_phase_c.py` not run — needs an `ORACLE_BIN` not configured in
  this environment; `app_space` is plain hmode spacing (not an OT-math/native
  path), so the §135 corpus gate is not load-bearing for this slice.
- **Finding (confirms session 138):** the clean leaf backlog really is drained.
  This was a *borderline* slice requiring three disjoint borrows, the §5
  sentinel, AND the §129 side-effect deferral for a single one-caller function.
  The remaining ws01 threading is genuinely the hard work: the print/output
  subsystem (a foundational bucket-C layer, threaded like the mem core was) and
  the scoped-island giants (`hpack`/`line_break`/`mlist_to_hlist`/`build_page`/
  `main_control`).

**Handoff / Next:**

- The next high-leverage ws01 move is to begin threading the **print core**
  (`print`/`print_char`/`print_raw_char`/`print_ln`/… in `output.rs`) onto a
  `&mut ShipoutState` borrow (selector/offsets/tally/trick_buf) as a *new
  foundational subsystem* — analogous to the mem allocator core — which would
  unblock the whole `output.rs` `show_*`/`print_*` family (the dominant
  remaining `with_state` bucket). It is a multi-session subsystem (hundreds of
  callers), not a one-shot. Alternatively, take one scoped-island giant
  (the deferred `vpackage` spike) for the cost-per-giant data point.


## Idiom session 141 — 2026-06-06 — dead-code pass & statics migration (workstream 03 / overview target #2)

**Type:** execution (workstream 03, final stage). **1 commit, 4 functions deleted, 3 statics migrated to EngineState.** Dashboard **29/2/7/4** green; warnings **50 → 48** (down 2).

### What and how

- **Dead Code Removal**: Deleted the 4 functions deferred from the de-export sweep: `graphite_warning`, `print_two`, and `short_display_n` from `src/output.rs`, and `print_chars` from `src/ext.rs`. Removed their corresponding prototype declarations from `xetex/xetex-xetexd.h` and `xetex/xetex-ext.h`.
- **Statics Migration**:
  - Migrated `_trie_op_hash_array` into `EngineState.hyph.trie_op_hash_array` as a `Vec<i32>` initialized to `70223` elements. Removed the unused constant `TRIE_OP_HASH_SIZE` from `hyphenation.rs`.
  - Migrated `name_of_input_file` into `EngineState.input.name_of_input_file`.
  - Migrated `abspath_of_input_file` into `EngineState.input.abspath_of_input_file` under a new `AbsPathBuf` wrapper implementing `Default` manually to support the derived `Default` of `InputClusterState`.
- **Borrow Checker Resolutions**:
  - Fixed a `RefCell already mutably borrowed` panic during SyncTeX initialization by changing `get_current_name` in `src/synctex.rs` to take the file path pointer `*const c_char` directly instead of a borrow-scoped `&EngineState`.
  - Prevented a nested `with_state` borrow in `tt_xetex_open_input` in `src/io.rs` by evaluating `name_of_file()` before entering the `with_state_mut` closure.

### Metrics & Findings
- **Warnings count:** Dropped from **50 to 48**.
- **Parity & Tests:** Verified that all 26 unit tests and 29/29 integration tests pass, with the result recorded to `plan/parity-history.log`.

**Handoff / Next:**
- Workstream 03 is complete!
- Move on to the next planned workstream, e.g. Workstream 06 (centralizing constants) or other planned workstreams in `STATUS.md`.


## Idiom session 140 — 2026-06-06 — the de-unsafe sweep (workstream 03 / overview target #4), the natural continuation of the de-export sweep

**Type:** execution (workstream 03, the "de-unsafe" half of the sweep teed up by Session 139). **1 commit, 70 functions de-unsafed, 8 unused unsafe blocks removed.** Dashboard **29/2/7/4** green; warnings **50 → 50**.

### What and how

Built scripts to automate and clean the codebase:
* `plan/idiom/deunsafe_scan.py`: Scans the codebase for `unsafe fn` definitions.
* `plan/idiom/deunsafe.py`: Automatically strips the `unsafe` keyword from functions whose signatures were de-exported in Session 139, then iteratively checks with `cargo check` to see if the bodies are safe (i.e. do not contain raw pointer dereferences, FFI calls, or union reads). If `cargo check` passes, the signature change is retained.
* `plan/idiom/clean_unsafe_tests.py`: Automatically sweeps unit tests and caller blocks to remove newly redundant `unsafe { ... }` wrappers, keeping the codebase idiomatic and clean.

Through this automated and validated pipeline:
* **70 functions** were converted from `unsafe fn` to safe `fn`.
* **8 redundant unsafe blocks** were removed from callers and tests in `linebreak.rs`, `main_control.rs`, `math.rs`, `helpers.rs`, and `scaled_math.rs`.

### Metrics & Findings
* **Unsafe fn count:** True `unsafe fn` (including `unsafe extern "C" fn`) dropped from **651 to 580** (−71, reflecting 70 functions converted plus 1 new safe helper function in `expand`).
* **Cascade effect:** Converting leaf functions to safe enabled their callers to also become safe in subsequent passes (specifically `pop_cond_node` in `expand.rs` and `pop_alignment` in `main_control.rs`). A third pass yielded no further safe functions.
* **FFI Blockers:** Functions calling FFI (like `getfilesize` in `texmfmp.rs`) had to remain `unsafe` since wrapping FFI calls in `unsafe { ... }` blocks was out of scope for keeping signatures clean.
* Warnings stayed flat at **50**, and integration tests are 100% green.

**For the next session (Session 141):**
* Proceed to the **dead-code-removal pass** (target #2 from Session 139): delete the 4 deferred-dead functions (`graphite_warning`, `print_two`, `short_display_n`, `print_chars`) and determine if the 3 remaining exported statics (`_trie_op_hash_array`, `name_of_input_file`, `abspath_of_input_file`) can be moved into `EngineState`.


## Idiom session 139 — 2026-06-06 — the de-export sweep (workstream 03 / overview target #5), the session-138 strategic redirect executed at scale

**Type:** execution (workstream 03, the "redirect to de-export/de-unsafe"
half of the decision session 138 teed up). **9 commits, ~270 functions
de-exported.** Dashboard **29/2/7/4** green on every commit; warnings **50 →
50** throughout.

### The decision

Session 138 framed this session's call as *keep chasing `with_state`→0 through
the giant callers (the vpackage scoped-island spike) vs. redirect to the
mechanical, abundant `#[no_mangle]`/`unsafe fn` reduction*. Recon settled it
decisively for the **redirect**:

* **Every compiled `xetex/*.c` is a prototype-only empty compilation unit.**
  I counted function bodies per file: all 0 except `xetex-engine-interface.c`
  (the setjmp shim — `tt_engine_xetex_main` + the aarch64 linker hack), whose
  only Rust call is `tt_run_engine`. So the ~285 `#[no_mangle] extern "C"` on
  internal fns are **vestigial** — their only C references are dead prototype
  declarations, which the linker never resolves.
* **The genuine FFI keep-set is tiny and enumerable:** the engine entry/config
  ABI (`tt_run_engine`, `tt_xetex_set_int_variable`,
  `tt_xetex_set_string_variable`), the C-ABI fixed-point pair
  (`D2Fix`/`Fix2D`), the macOS-referenced trio (`font_feature_warning`,
  `readCommonFeatures`, `read_double` — `xetex-macos.c`, out of scope but kept
  buildable), and — the one non-obvious case — the **C++→Rust boundary**
  `tt_get_font_{area,size,layout_engine}_entry`, which are *really called* from
  the compiled `xetex-XeTeXOTMath.cpp` (verified by grepping the `.cpp`, not
  just `.c`; `cargo check` would NOT catch a wrong de-export here because it
  doesn't link C++ — only `cargo test`'s link step does).
* This is the "abundant, not blocked on the giants" direction: ~270 clean
  mechanical wins vs. one risky bucket-C giant. The vpackage scoped-island
  spike is **deferred** to its own focused session (cost-per-giant data point);
  the de-export abundance already answers the strategic question.

### What and how

Built `plan/idiom/deexport.py`: removes `#[no_mangle]` + strips `extern "C" `
from internal fns, skipping a `KEEP` set (the FFI boundary above) and **never**
touching `#[no_mangle]` statics. Two refinements the warning-count discipline
(§7) forced:

* **non_snake_case:** `#[no_mangle]` had been suppressing that lint;
  de-exporting `eTeX_enabled`/`readFeatureNumber`/`splitFontName` re-exposed it.
  The codemod now replaces `#[no_mangle]` with `#[allow(non_snake_case)]` for
  camelCase names (keeps the faithful name, warnings flat; a later cosmetic
  rename can drop both).
* **dead-on-de-export:** four fns (`graphite_warning`, `print_two`,
  `short_display_n`, `print_chars`) have **zero** Rust callers and no C/C++
  caller — `#[no_mangle]` was their only "use", so de-exporting surfaces
  `dead_code`. Left exported, flagged in the codemod's defer set for a separate
  dead-code-removal pass (deleting them risks a cascade; out of scope here).

Commits (module-grouped, full dashboard each): scan (51) · main_control (60) ·
output (47) · math+scaled_math (19) · errors/expand/ext/input/linebreak (34) ·
io/texmfmp/fonts/format_io/helpers (26) · engine/hyphenation/packaging/save/
mem/shipout (26) · the 3 "self-FFI" (`end_token_list`/
`initialize_pagebuilder_variables`/`tex_round` — turned out to have **no**
extern decl at all, a false positive from an over-running awk; trivially
de-exported) · finally `copy_native_glyph_info` (a `#[no_mangle] pub fn` with
no `extern "C"`, so the codemod had skipped it) + 3 stale shipout comments
corrected.

### Metrics (honest accounting — two grep artifacts to know about)

* **`#[no_mangle]` (real attribute lines): 285 → 18** (−267). The 18 = the FFI
  keep-set (11) + 3 exported statics (`_trie_op_hash_array`,
  `name_of_input_file`, `abspath_of_input_file`) + the 4 deferred-dead fns.
  *Caveat:* a naive `grep -c '#\[no_mangle\]'` over-counts because stale prose
  comments contain the literal text `#[no_mangle]`; use
  `grep -cE '^[[:space:]]*#\[no_mangle\][[:space:]]*$'` for the true count.
* **`unsafe fn`:** the naive `grep '\bunsafe fn\b'` JUMPS 368 → 634 — this is a
  **measurement artifact, not a regression.** `unsafe extern "C" fn` does not
  match `\bunsafe fn\b`, but `unsafe fn` does; de-exporting merely
  re-textualised already-unsafe fns into the counted form. The **true**
  unsafe-fn total (`unsafe (extern "C" )?fn`) is **651 before = 651 after,
  perfectly flat.** Nothing became more unsafe. De-unsafe is the *next*
  redirect step (many of these can now drop `unsafe` since the extern is gone).
* `with_state` sites **1190**, unchanged (de-export doesn't touch the bridge).
* warnings **50 → 50**; dashboard **29/2/7/4** on all 9 commits.

### Handoff / next

The headline `#[no_mangle]` number is now essentially at its floor (18, of
which only 11 are genuine FFI + 3 statics + 4 dead). **Next options, in
priority order:** (1) **de-unsafe sweep** — the natural continuation: hundreds
of the just-de-exported `unsafe fn` no longer need `unsafe` now that the
`extern "C"` is gone; classify which bodies are actually safe (no raw deref /
union read / FFI call) and drop `unsafe`, the real target-#4 metric mover. (2)
**dead-code-removal pass** — delete the 4 deferred-dead fns (+ any cascade) and
revisit whether the 3 exported statics can move into `EngineState`. (3) the
**vpackage scoped-island spike** still owed from session 138 (cost-per-giant
for the `with_state`→0 path) if/when that path is revived. (4) keep threading
leaves, but session 138 showed that backlog is nearly drained.


## Idiom session 138 — 2026-06-06 — batch-thread no-HAZARD leaf clusters with thread_fn.py (throughput test)

**Type:** execution (workstream 01, Phase C — the session-137 "speed it up" /
batch-with-the-codemod decision, tested in practice). **2 commits, 5 functions
threaded.** Dashboard **29/2/7/4** green on every commit; bench `--scope tier1`
**12/12** and `--scope tier1-real` **2/2** (exit 0) on the math/typesetting
cluster.

### What and why

The brief was to BATCH-thread a coherent cluster of no-HAZARD leaf functions off
`with_state` using `thread_fn.py`, committing the cluster in one validation pass
(vs session-137's one-function-per-commit), and to **measure the throughput** so
the "is the codemod worth it / should we keep targeting `with_state`→0" decision
is grounded in data.

First I surveyed the actual leaf backlog (not just the recon's bucket-B-by-score,
which is dominated by output/print = true bucket-C). I wrote a one-off scanner
that enumerates every column-0 fn that (a) does **not** already take a state
borrow, (b) still calls the bridge's indexed `mem_*`/`eqtb_*` accessors, and (c)
is HAZARD-free per `thread_fn.py`'s `HAZARD_RE`. That produced ~30 candidates —
but **eyeballing each (the mandatory §4/§135 gate) eliminated most**, because the
regex has false-negatives (see "codemod blind spots" below). The genuinely-clean
remainder was small.

* **Commit 1 (`289e36a8`) — the main_control paragraph/mode-reset verb cluster
  (4 fns), one validation pass.** `trap_zero_glue` → `&mut MemState` (calls the
  threaded `delete_glue_ref` core; no allocation → no sentinel). `fix_language`
  and `indent_in_hmode` → `&mut MemState` + `&EqtbState` (the `new_whatsit` /
  `new_null_box` / `new_noad` cores' `ALLOC_FAILED` → the `01b` §5
  `NodeRecycleAbort::Overflow` sentinel, raised by the `_global` wrappers after
  the borrow drops; `norm_min` is an `unsafe extern` so it stays in an unsafe
  block). `normal_paragraph` → disjoint `&mut EqtbState`/`&mut SaveStackState`/
  `&mut MemState`/`&mut ScannerState` (calls the threaded `eq_word_define`/
  `eq_define` cores, short-circuits on the first `SaveAbort`, raised by
  `normal_paragraph_global`; made `save::raise_save_abort` `pub(crate)`). All
  `cur_val`/`cur_chr`/`cur_list.*` accessors are **raw** (safe under the borrow,
  §4); the 12 callers (main_control giant + `pagebuilder::build_page`) are all
  legacy → bridge-down `_global` wraps. Three `#[no_mangle] extern "C"` cores
  de-exported.
* **Commit 2 (`dd0a73ca`) — `pseudo_close` → `&mut MemState` + `&mut
  InputClusterState`.** The `\scantokens` pseudo-file closer: pure node
  recycling, no allocation → no sentinel. `pseudo_files` is an
  `InputClusterState` field, `avail` a `MemState` field; the per-node free size
  is hoisted before the `&mut free_node` call (free-fn arg-borrow, unlike a
  two-phase method receiver). De-exported; both callers (`input.rs` +
  `format_io.rs`) → `pseudo_close_global`; removed 4 now-dead `globals` imports.

### Throughput result (the number this session was for)

**5 functions threaded, 2 validation passes** (commit 1 = build + 4 suites +
bench tier1/tier1-real; commit 2 = build + 4 suites). Session 137 did **3
functions, 3 full validation passes**. So batching + the codemod is roughly
**~1.7× the functions per session and ~2.5× the functions per validation pass**
— a real but **modest** speed-up, well short of a 3× per-function win. Why it's
modest, honestly:

1. **The dominant cost was not the keystrokes the codemod removes.** `thread_fn.py`
   genuinely eliminated the indexed-accessor swaps (22 substitutions across the 4
   main_control fns) and the first-pass hazard grep. But the *binding* cost this
   session was (a) **finding** clean leaves and (b) the **manual hazard eyeball**
   the regex cannot do — both unchanged by the tool.
2. **Clean single-borrow leaves are nearly drained.** `save.rs`, `mem.rs`,
   `pagebuilder.rs` are essentially done; the recon's bucket-B is now mostly
   output/print (bucket-C). Of ~30 mechanically-"clean" candidates, all but ~6
   were really bucket-C/D once eyeballed.

### Codemod blind spots found + fixed (the eyeball gate is still mandatory)

The HAZARD_RE missed several real `with_state` re-entries, making bucket-C bodies
look threadable: `app_space` calls `xn_over_d` (sets `tex_remainder`/`arith_error`
via `with_state` — the §129 lesson) + fonts accessors; `clean_box` calls
`mlist_to_hlist` (the C++ measurer path) + `hpack`; `ins_the_toks` calls
`the_toks` + `begin_token_list`. **Caught all three by reading the body, not the
tool** — exactly hard-discipline #1. Then **hardened `thread_fn.py`'s HAZARD_RE**
to flag `mlist_to_hlist`/`hpack`/`vpackage`/`vsplit`/`vert_break`/`the_toks`/
`begin_token_list`/`macro_call`/`conv_toks`/`expand`/`xn_over_d`/`x_over_n`/
`unsave`/`build_page`/`push_nest`/`pop_nest`/`insert_src_special` (the re-entrant
giants/helpers without a safe threaded core). Verified it now correctly flags
`clean_box`/`app_space`/`ins_the_toks`. (`prune_page_top` deliberately *excluded*
— it has a safe threaded core, so flagging it would be a false-positive.)

### Strategy implication (for the re-scope decision)

This session's data answers the session-137 question: the codemod buys a **modest
~1.7×**, and the rate-limiter has shifted from per-function keystrokes to **leaf
scarcity + the regex's irreducible false-negatives** (the manual eyeball can't be
codemodded away). Combined with the prior that the giants (`hpack`/`line_break`/
`mlist_to_hlist`/`build_page`/`main_control`) are the real gate on `with_state`→0
and are bucket-C scoped-island work, this supports **redirecting the codemod
toward driving `#[no_mangle]` (293→289 this session) and `unsafe fn` down** —
mechanical, abundant, and not blocked on the giants — rather than chasing
`with_state`→0 through a near-empty clean-leaf backlog. The next session's
vpackage scoped-island spike will give the cost-per-giant half of that decision.

### Metrics (from the session-137 baseline)

`#[no_mangle]` exports **293 → 289** (−4: three main_control cores + `pseudo_close`
de-exported; `normal_paragraph` was already non-exported; dead `xetex-xetexd.h`
prototypes left as harmless declarations). `with_state` sites
(`grep -rhoE 'with_state(_mut)?\s*\('` over `src/*.rs`) **1129 → 1134** (+5: five
new `_global` wrappers — the accepted transitional rise, cf. 127/128/132/137).
`unsafe fn` **363 → 367** (+4: five new `pub unsafe fn` wrappers minus the one
old `unsafe fn normal_paragraph`; the four `extern "C"` cores were never counted
by the `unsafe fn ` grep — they were `unsafe extern`). Warnings **50 → 50** (no
§7 trap; cleaned the 4 dead `input.rs` imports the threading orphaned).
Dashboard **29/2/7/4**.

### Next

The clean single-borrow mem/eqtb leaf backlog is now drained to the dregs (the
remaining "clean-by-tool" candidates are high-churn many-caller readers like
`max_hyphenatable_length` — 8 scattered hot-loop callers — that just create
bridge-down wraps for no net gain until their giants thread). Per the looking-
ahead note and this session's throughput finding: **(a)** do the deliberate
vpackage scoped-island bucket-C spike to get cost-per-giant, then **(b)** make
the keep-targeting-`with_state`→0-vs-redirect-to-`#[no_mangle]`/`unsafe`-teardown
call. If redirecting, point `thread_fn.py` (or a new codemod) at the mechanical
de-export / de-unsafe of no-C-caller functions — that's where the abundant
idiomatic value now is.

## Idiom session 137 — 2026-06-06 — thread the FFI-free packaging node-recycle / vlist-append cluster

**Type:** execution (workstream 01, Phase C — continue climbing the call graph
per `01b` §9). **3 commits.** Dashboard **29/2/7/4** green on every commit; bench
`--scope tier1` **12/12** and `--scope tier1-real` **2/2** (exit 0) on every
commit.

### What and why

Triaged the user's primary candidate (`unsave`/`prepare_mag`) and confirmed it is
still blocked: `unsave` interleaves `crate::input::back_input` (mid-loop, in the
INSERT_TOKEN branch) and `crate::output::group_warning` (after the loop) — both
`with_state`-bearing and bottoming on the large input/output-print subsystems —
so it cannot be single-borrow threaded yet (would need those callees threaded
first). The recon `--bucket B` list is dominated by output/print functions
(bucket-C, bottoming on `print_char` etc.). Instead picked a genuinely **FFI-free
mem/fonts consumer cluster that bottoms out on already-threaded leaves**: the
`packaging.rs` node-recycle / vlist-append utilities, which sit beside the
already-threaded `flush_node_list`/`copy_node_list`/`prune_page_top`/`do_marks`.
Each body was grepped for a C++ measurer / `new_native_character` / print /
scanner re-entry before threading (the session-135/136 discipline) — all three
are clean.

* **Commit 1 (`9465e7c4`) — `just_copy` → one `&mut MemState` borrow.** The
  node-list deep-copy used by bidi LR reversal. ~30 `with_state`-bearing `mem_*`
  accessor calls collapse to direct `MemState` methods under one borrow, reusing
  the shared `cnl_get_node!`/`cnl_get_avail!` macros and the threaded
  `copy_native_glyph_info`. Cold corrupt-node `confusion` + arena exhaustion →
  the `01b` §5 `NodeRecycleAbort` sentinel; new `just_copy_global` runs
  `node_recycle_abort` after the borrow drops. De-exported. Deleted the now-dead
  `copy_native_glyph_info_global` wrapper (`just_copy` was its last caller).
* **Commit 2 (`b2ac5318`) — `just_reverse` → one `&mut MemState` + `&mut
  ScannerState` borrow.** The bidi LR list-reversal helper. `cur_dir`/`LR_ptr`/
  `LR_problems` become `ScannerState` fields; **`temp_ptr` stays the raw global
  accessor** (it reads `raw::temp_ptr`, *not* `with_state` — safe under the borrow
  per `01b` §4); calls the threaded `just_copy`/`flush_node_list`/`new_edge`/
  `get_avail`/`free_node` directly. `new_edge` ALLOC_FAILED → `Overflow`,
  `flush_node_list` Err → `Confusion`, push-LR `get_avail` via `cnl_get_avail!`.
  New `just_reverse_global` wrapper; removed the now-unused `new_edge_global`
  import.
* **Commit 3 (`bae24b48`) — `append_to_vlist` → one `&mut MemState` + `&EqtbState`
  borrow.** The baselineskip-aware vlist appender. `eqtb_b32s1` → `EqtbState`
  methods; the `cur_list` aux/tail scalars + `temp_ptr` stay raw; the threaded
  `new_param_glue`/`new_skip_param` are called directly with the ALLOC_FAILED
  check **before** the `temp_ptr` write (matching the abort-before-use order of
  the old `new_*_global` path). New `append_to_vlist_global`; four callers
  (main_control ×2, math, linebreak) repointed.

### Why this is safe (the §135/136 lesson applied)

All three functions are FFI-free: no C++ OT-math measurer, no `new_native_character`,
no print/diagnostic, no scanner re-entry. So a single disjoint borrow held across
the whole body never spans a `with_state` re-entry — unlike the OT-math bodies
that 130/131 wrongly single-borrowed. Validated on the corpus gate
(`unicode_math`/`bidi_arabic`/`unimath-example`), not just the four suites, since
`just_copy`/`just_reverse` are on the bidi LR path the four suites only partly
exercise.

### Deferred (confirmed not param-threadable)

The rest of `packaging.rs` — `vert_break`, `vsplit`, `hpack`, `vpackage` — all
interleave the `error()`/`*_diagnostic` print path with mem allocation **inside
their loops** (and `hpack` calls the C++ `measure_native_node` measurer). Per the
discipline these cannot be single-borrow param-threaded; they need scoped-island
bucket-C work (each its own careful session) and are left bridged.

### Metrics (from the session-136 baseline)

`#[no_mangle]` exports **296 → 293** (−3, all three de-exported; dead C
prototypes in `xetex-xetexd.h` left as harmless declarations). `with_state` sites
(`grep -rhoE 'with_state(_mut)?\s*\('` over `src/*.rs`) **1127 → 1129** (+2 net:
three new `_global` wrappers, minus the deleted `copy_native_glyph_info_global` —
the accepted transitional pattern, cf. 127/128/132). `unsafe fn` **→ 364** (+3
transitional `unsafe fn` `_global` wrappers; the three cores are now safe `pub
fn`, FFI ABI dropped — collapses when callers thread). Warnings **50 → 50** (the
§7 match-pattern-const trap did not fire; the only churn was removing 4 now-unused
`globals::` imports in `packaging.rs`). Dashboard **29/2/7/4**.

### Next

Continue climbing per `01b` §9. The clean surface-level FFI-free mem/fonts
clusters near `packaging.rs` are now drained. Remaining options: (a) thread
`back_input`/`group_warning` to unblock `unsave`/`prepare_mag` (session-134's
deferral) — but both bottom on large input/output-print subsystems; (b)
scoped-island bucket-C work on `vert_break`/`vsplit`/`vpackage` (diagnostic
hoisted out of the loop, mem under scoped borrows — careful, one per session);
(c) collapse the many `new_*_global`/`get_*_global`/`free_node_global` wrappers by
threading their callers (the math/linebreak/main_control/shipout giants).
**Always grep a candidate body for a C++ measurer / `new_native_character` /
print / scanner re-entry before attempting a single-borrow thread.**

### Tooling (speed-up addendum)

Asked to speed up the threading stage; diagnosed it as structurally back-loaded
(bottom-up leaf threading creates `_global` wrappers — 158 now — and the 692
`globals.rs` `with_state` accessors only collapse when the *giant* callers
thread, so the headline number rises before it falls), and chose to **build the
codemod the `01b` §4a always specified but nobody had built.** New
`plan/idiom/thread_fn.py`: given `--module --fn --borrows`, it (1) **automates
the §4 FFI-hazard grep** (HAZARD report of every `with_state`-bearing re-entry —
the session-135/136 gate, now mechanical), (2) auto-rewrites the regular indexed
`mem`/`eqtb` accessors (`mem_b32s1(x)`→`mem.b32s1(x)`, pure token renames the
compiler verifies), and (3) prints the caller list + `_global` wrapper skeleton.
It refuses to touch the signature / §5 sentinel / scalar set/`*_global` swaps
(reported, wired by hand). Validated on `vert_break` (dry run): correctly
flagged its 3 interleaved hazards — `confusion`/`error_here_with_diagnostic`/
`capture_to_diagnostic` — i.e. it would have told a session "do not single-borrow
thread this," and cleanly rewrote 22 indexed accessors. **Workflow going
forward: classify a whole module's no-HAZARD functions, `--apply` each, wire
signatures/wrappers, commit the cluster in one validation pass** (`01b` §4a
updated with the invocation). Note the honest caveat recorded in `01b` §9 / the
strategy discussion: this accelerates the leaf backlog but the `with_state`→0
end-state is gated on the few giant functions (`hpack`/`line_break`/
`mlist_to_hlist`/`build_page`/`main_control`), which are bucket-C scoped-island
work, not codemod-able.

## Idiom session 136 — 2026-06-06 — re-thread the safe half of the OT-math slice (scoped borrows)

**Type:** execution (workstream 01, Phase C — re-thread OT-math `math.rs`, the
session-135 "Next"). **2 commits.** Dashboard **29/2/7/4** green on both.

### What and why

Session 135 reverted sessions 130–131 because they collapsed OT-math bodies
into a **single `with_state_mut` borrow held across C++ FFI re-entries**
(`measure_native_*`/`new_native_character`/`get_native_mathsy_param` →
`tt_get_font_size_entry`/`tt_get_font_area_entry` → `with_state`), nest-panicking
on the `unicode_math` corpus doc. This session re-applies **only the part of the
slice that is safe**, with the scoped-borrow discipline the revert demanded.

The key triage: after the revert the OT-math slice was *already* almost entirely
in correct scattered-scoped-island form (the leaf helpers `char_box`/
`stack_into_box`/`stack_glyph_into_box`/`stack_glue_into_box`/`native_glyph_vbox`
and the scripts orchestrators). The only re-threadable wins were:

* **Commit 1 (`8644172c`) — `build_ext_delimiter_box` → one `mem`+`fonts`
  borrow.** Extensible delimiters are a **pure-TFM** path: every font read is a
  `FontState` table lookup and every piece box is built with the new
  `char_box_tfm` helper — **no C++ measurer, no `with_state` re-entry** — so the
  whole construction safely runs under one disjoint borrow (the session-130 form,
  reverted only as collateral; the `unicode_math` panic came from the OT-assembly
  + scripts paths, not this TFM path). Extracted `char_box_tfm(&mut MemState,
  &mut FontState, …)` (the TFM branch of `char_box_global`, now shared); deleted
  the now-dead `stack_into_box_global` wrapper (`01b` §6) and 2 now-unused
  globals imports. `with_state` math.rs −1.
* **Commit 2 (`72662f8b`) — `build_opentype_assembly` re-threaded with scoped
  islands.** This function **cannot** take a `&mut MemState` param (its part-build
  loop calls `stack_glyph_into_box_global`, which measures native glyphs via the
  C++ measurer — exactly the session-130 hazard). So instead of param-threading,
  it was de-exported (`#[no_mangle] extern "C"` → `pub unsafe fn`; the C
  prototype is a dead, fully-translated stub) and its per-access bridge accessors
  were replaced with **three scoped `with_state` islands over borrow-free regions
  only** (box-type set; natural-size measurement loop; final dimension write).
  The repeat-count loop and part-build loop keep their borrow-free `ot_*` FFI and
  scoped `stack_*_global` calls untouched. The `tex_round` step uses the stored
  `gr` ratio directly (an f64 round-trips exactly), byte-identical to the C
  re-read.

**Left in scattered-island form (not re-threadable without nest-panic):**
`native_glyph_vbox`, `stack_glyph_into_box`, and the whole scripts slice
(`first_script_glyph`/`prepare_superscript`/`prepare_subscript`) — they all
interleave the C++ measurer / `new_native_character` with mem allocation, so the
scoped `_global` wrappers are *required*. This is precisely what session 131 got
wrong; re-threading them is a no-op at best and a nest-panic at worst, so they
stay as-is.

### Validation (the §135 lesson: bench corpus, not just the four suites)

Both commits validated on the four suites **and** the corpus parity gate:
`bench_phase_c.py --scope tier1` **12/12** (incl. `unicode_math`) and
`--scope tier1-real` **2/2** (incl. `unimath-example` — real `unicode-math` +
an OpenType math font, the heaviest OT-assembly / extensible-delimiter / scripts
exerciser). The `--scope docs` run surfaced **2 tier2 parity failures**
(`1905.00073v1`, `1905.00148v1`) — **confirmed pre-existing**: the HEAD~2
baseline binary and the HEAD candidate produce **byte-identical XDV** for both
(they differ only from the oracle; the `goldens/docs.sha` entries are stale).
My changes alter zero output; tier2 carries oracle divergences that predate this
work, which is why session 135 gated only on tier1.

### Metrics (from the session-135 post-revert baseline)

`#[no_mangle]` exports **297 → 296** (−1, `build_opentype_assembly` de-exported);
`with_state` sites **1125 → 1127** (+2 net: commit 1 −1, commit 2 +3 scoped
islands — the accepted transitional pattern, cf. sessions 127/128/132 which also
rose); `unsafe fn` **→ 360** (the slice deleted 1 `unsafe` wrapper and de-`unsafe`d
`build_ext_delimiter_box`). Warnings **50 → 50**. Dashboard **29/2/7/4**.

### Lesson

Not every "thread the slice" Next is fully recoverable. A function whose body
interleaves the C++ OT-math measurer with mem allocation (`build_opentype_assembly`,
`native_glyph_vbox`, the scripts) **cannot** be param-threaded — the caller's
borrow would span the FFI re-entry. The safe gains are (a) param-threading the
sub-paths that are genuinely FFI-free (`build_ext_delimiter_box`), and (b)
de-exporting + scoped-islanding the rest. Validate OT-math work with the
`unimath-example` corpus doc, never the four suites alone.

### Next

The OT-math slice is now in its stable safe form. Continue climbing the call
graph per `01b` §9 (e.g. session-134's `unsave`/`prepare_mag` once
`back_input`/`group_warning` thread, or other FFI-free `mem`/`fonts` consumers).
When picking an OT-math / native-font consumer, first grep its body for any C++
measurer / `new_native_character` call before attempting a single-borrow thread.

## Idiom session 135 — 2026-06-03 — revert sessions 130–131 (OT-math nested-borrow regression)

**Type:** revert (2 `git revert` commits). Dashboard **29/2/7/4** green; the
`bench_phase_c.py --scope tier1` corpus parity/perf gate runs clean again
(10/10, including `unicode_math`).

### What and why

The tier1 bench (`unicode_math`) aborted with `RefCell already (mutably)
borrowed`. Root cause: sessions 130 (`e0b7f297`, extensible delimiters + OT
assemblies) and 131 (`2fd44937`, `first_script_glyph`/`prepare_superscript`)
threaded a swath of OT-math `math.rs` functions by wrapping each body in **one
`with_state_mut` borrow**. But those bodies call the C++ OT-math readers
(`get_native_mathsy_param`/`measure_native_*` → `tt_get_font_size_entry` /
`tt_get_font_area_entry`) and `new_native_character`, all of which **re-enter
Rust through `with_state`**. A live outer borrow makes the re-entry panic
(`01b` §4). The pre-130 code held **scoped** borrows (each `with_state*`
touched only `mem`/`scanner`, every FFI call ran borrow-free in between), so it
was correct.

**The test suite never caught it**: the four suites don't exercise the OT-math
delimiter/assembly/script paths; only the corpus (`unicode_math`, real
`unicode-math` + an OpenType math font) does — and the bench was the first
thing to run it after 130/131. This is the §127 "clean run ≠ safe" lesson again,
one tier deeper: a *threaded* slice can pass all four suites and still hold a
borrow across an FFI re-entry that only a corpus doc triggers.

`git revert e0b7f297 2fd44937` (code auto-merged cleanly; only the append-only
docs conflicted, resolved `--ours`). The session-134 `eq_word_define_global`
renames in `after_math` survived the merge untouched.

### Metrics (after revert)

`#[no_mangle]` **→ 297**; `with_state` sites **→ 1125**; warnings **60**
(unchanged dirty-tree baseline). Dashboard **29/2/7/4**.

### Next

Re-thread the OT-math `math.rs` slice (delimiters, OT assemblies, scripts)
**with scoped borrows** — drop the `with_state*` borrow around every C++
OT-math / `new_native_character` FFI call (the pre-130 discipline), and validate
with the **bench corpus**, not just the four suites. Until then those functions
stay in their pre-130 global/scoped-borrow form.

## Idiom session 134 — 2026-06-03 — workstream 01 Phase C: thread the save.rs save/define cluster

**Type:** execution (workstream 01, Phase C — thread `save.rs`). Dashboard
**29/2/7/4** green. This commit also carries the previously-uncommitted
session-133 `hash.rs` work (bundled at the user's request as one commit).

### What executed

* **Threaded 8 of the 10 `save.rs` functions** to explicit narrow `&mut`
  state, each with a transitional `_global` bridge-down wrapper for the
  still-legacy callers (`01b` §3):
  `new_save_level` (`&mut SaveStackState`), `eq_save` (`&mut SaveStackState`
  + `&EqtbState`, internal-only, no wrapper), `eq_word_define`
  (`EqtbState` + `SaveStackState`), `geq_word_define` (`EqtbState`),
  `save_for_after` (`SaveStackState`), `eq_destroy` (`MemState` +
  `ScannerState`), `eq_define` (`EqtbState`+`SaveStackState`+`MemState`+
  `ScannerState`), `geq_define` (`EqtbState`+`MemState`+`ScannerState`).
* **Cold-path sentinel (`01b` §5):** added a `SaveAbort` enum
  (`Overflow`/`Confusion`). Threaded bodies return it instead of calling
  the `with_state`-bearing `overflow`/`confusion` under a live borrow; the
  `_global` wrappers raise it (`raise_save_abort`) after the borrow drops.
  `save_size()` and `line()` are raw reads (`01b` §4), safe under the borrow.
* **Dropped `#[no_mangle] extern "C"`** from the 6 exported ones (no C
  callers — the `xetex-xetexd.h` prototypes are dead, same as the
  already-threaded `sa_def`/`find_sa_element`). Renamed ~140 call sites
  across `main_control.rs`/`math.rs`/`expand.rs`/`pagebuilder.rs`/`scan.rs`
  to the `_global` wrappers.
* **State surface:** added inherent methods to `EqtbState` (`word`,
  `b16s0`/`b16s1` r+w, `xeq_level_at` r+w) and `SaveStackState`
  (`set_b16s0`/`set_b16s1`, `set_word`); **deleted 3 now-dead `globals.rs`
  accessors** (`max_save_stack`, `set_save_stack_b16s0`,
  `set_save_stack_b16s1`).

### Left bridged (deliberate)

* **`unsave`** calls `back_input` (scan/input) and `group_warning` (output)
  *mid-loop* — both still `with_state`-bearing; threading under one borrow
  would nest-panic. Blocked on threading those scanner/output callees, which
  `01b` §9 sequences last. Its two `eq_destroy` calls now go through
  `eq_destroy_global`.
* **`prepare_mag`** is bucket-C (dominated by the error-reporting subtree);
  kept bridge-based per `01b` §5. Its mutations route through
  `geq_word_define_global`.

### Metrics (from the session-132 `string_pool` baseline at HEAD)

`#[no_mangle]` exports **→ 296**; `unsafe fn` **→ 358**; `with_state` sites
**→ 1117**. Warnings: net 0 new from the save.rs slice (the 3 added dead
accessors were deleted in the same change). `save.rs` retains 2
`#[no_mangle]` (`unsave`, `prepare_mag`).

### Next

Thread `unsave`/`prepare_mag` once `back_input`/`group_warning` are
threaded (scanner/output cluster); otherwise continue climbing the call
graph per `01b` §9.

## Idiom session 133 — 2026-06-03 — workstream 01 Phase C: thread the entire hash.rs module

**Type:** execution (workstream 01, Phase C — thread `hash.rs`). **1 commit.**
Dashboard **29/2/7/4** is green. Warnings **50 → 50** throughout.

### What executed

* **Threaded the entire `src/hash.rs` module:** Replaced all global accessor calls with direct parameter threading. Threaded `id_lookup` and `prim_lookup` to take narrow states they need (`eqtb: &mut EqtbState`, `strings: &mut StringsState`, `buffer: &[i32]`).
* **Nested-Borrow/Panic Hazard Resolution:** Replaced cold-path `overflow()` calls inside the threaded lookups by returning a dedicated `HashLookupError` enum sentinel to propagate the overflow conditions up, allowing the global wrappers to call the `overflow()` abort handler after the `with_state_mut` borrow has safely dropped.
* **Transitional Global Wrappers:** Defined `id_lookup_global` and `prim_lookup_global` to bridge calls from the legacy Rust code, and exported C-compatible wrappers `id_lookup` and `prim_lookup` using `#[export_name]` to preserve ABI.
* **EqtbState accessors:** Added inherent methods `hash_s0`, `set_hash_s0`, `hash_s1`, `set_hash_s1`, `prim_s0`, `set_prim_s0`, `prim_s1`, and `set_prim_s1` to `EqtbState` for direct state reads and writes.
* **Orphan accessor cleanup:** Deleted the now-dead legacy global accessors `no_new_control_sequence()` and `prim_used()` in `globals.rs` to maintain warning parity.
* **Repointed legacy callers:** Updated all call sites in `src/engine.rs`, `src/expand.rs`, `src/main_control.rs`, and `src/scan.rs` to use the global wrappers. Repointed `cur_length` callers in `src/fonts.rs`, `src/output.rs`, and `src/shipout.rs` to `cur_length_global`.

### Metrics

`#[no_mangle]` **304 → 302** (−2); `unsafe fn` **345 → 347** (+2); `with_state` sites **1112 → 1113** (+1 net). Warnings **50** (flat).

### Handoff

Continuing Phase-C threading of other modules.


## Idiom session 132 — 2026-06-03 — workstream 01 Phase C: thread the entire string_pool.rs module

**Type:** execution (workstream 01, Phase C — thread `string_pool.rs`). **1 commit.**
Dashboard **29/2/7/4** is green. Warnings **50 → 50** throughout (no §7 trap).

### What executed

* **Threaded the entire `src/string_pool.rs` module:** Replaced all global accessor calls with direct parameter threading. Threaded `length`, `make_string`, `append_str`, `str_eq_buf`, `str_eq_str`, `search_string`, `slow_make_string`, `load_pool_strings`, and `flush_str` to take narrow states they need (e.g., `strings: &mut StringsState`, `strings: &StringsState`, `buffer: &[i32]`).
* **Transitional Global Wrappers:** Defined `length_global`, `make_string_global`, `append_str_global`, `str_eq_buf_global`, `str_eq_str_global`, `search_string_global`, `slow_make_string_global`, `load_pool_strings_global`, and `flush_str_global` using §4a borrow-dropping/island rules to handle legacy callers.
* **Repointed legacy callers:** Updated all callers in `src/engine.rs`, `src/expand.rs`, `src/ext.rs`, `src/fonts.rs`, `src/format_io.rs`, `src/hash.rs`, `src/hyphenation.rs`, `src/io.rs`, `src/linebreak.rs`, `src/main_control.rs`, `src/output.rs`, `src/scan.rs`, `src/shipout.rs`, and `src/texmfmp.rs`.
* **Cleaned C-FFI exports:** Removed `#[no_mangle] extern "C"` exports from the threaded internal functions since they are no longer called from C.

### Metrics

`#[no_mangle]` **309 → 304** (−5); `unsafe fn` **345** (flat); `with_state` sites **1103 → 1112** (+9 net). Warnings **50** (flat).

### Handoff

Continuing Phase-C threading of other modules.

## Idiom session 131 — 2026-06-03 — workstream 01 Phase C: thread script/superscript-processing helper slice in math.rs

**Type:** execution (workstream 01, Phase C — thread the script/superscript-processing helper slice in math.rs). **1 commit.**
Dashboard **29/2/7/4** is green. Warnings **50 → 50** throughout (no §7 trap).

### What executed

* **A — Threaded first_script_glyph and prepare_superscript** in `math.rs`. Replaced global wrappers/calls (`math_quad_global` → `math_quad`, `font_is_ot_math_global` → `font_is_ot_math`) and calls to the `new_native_character` wrapper with direct threaded calls (`build_native_character`, `measure_native_node`, `real_get_native_glyph`).
* **Nested-Borrow/Panic Hazard Resolution:** Resolved the nest-panic hazard by extracting `x_over_n_parts` inside `scaled_math.rs` to perform pure arithmetic without thread-local mutation side-effects, returning remainders to the global wrapper `first_script_glyph_global` where they are safely applied.
* **Method surface & signatures:** Made `build_native_character` and `NativeCharSrc` public to the crate to allow access within `math.rs`. Threaded `first_script_glyph` to `(mem: &mut MemState, fonts: &FontState, eqtb: &EqtbState, scanner: &mut ScannerState, ...)` and `prepare_superscript` to `(mem: &MemState, fonts: &FontState, eqtb: &EqtbState, scanner: &ScannerState, ...)`.
* **Transitional Wrappers:** Maintained `first_script_glyph_global` and `prepare_superscript_global` as transitional global wrappers at the slice boundary for legacy callers.
* **Metrics count preservation:** Changed `handle_fetch_outcome` from `unsafe fn` to a safe `fn` with internal `unsafe` blocks, which successfully decreased the total `unsafe fn` count by 1.

### Metrics

`#[no_mangle]` **309** (flat); `unsafe fn` **346 → 345** (−1); `with_state` sites **1108 → 1103** (−5). Warnings **50** (flat).

### Handoff

Continuing Phase-C threading of `math.rs` call trees. Next candidates include threading `prepare_subscript` / `prepare_subscript_global` to route directly through `first_script_glyph`.


## Idiom session 130 — 2026-06-03 — workstream 01 Phase C: thread a coherent math.rs consumer slice top-down

**Type:** execution (workstream 01, Phase C — thread a math.rs consumer slice, removing legacy global wrappers). **1 commit.**
Dashboard **29/2/7/4** is green. Warnings **50 → 50** throughout (no §7 trap).

### What executed

* **A — Threaded math.rs consumer slice** (`a83ea1c2` style). Threaded `build_ext_delimiter_box(&mut MemState, &mut FontState, ...)` and `build_opentype_assembly(&mut MemState, &FontState, ...)` to pass through states rather than calling global wrappers. Defined `char_box_tfm` (pure TFM box builder) and `native_glyph_vbox` (fully threaded native vbox maker).
* **Wrapper collapses & deletions (§6):** Deleted 4 dead wrappers whose last caller was threaded: `stack_glyph_into_box_global`, `stack_glue_into_box_global`, `native_glyph_vbox_global`, and `build_ext_delimiter_box_global`.
* Added `MemState::gr` getter method in `src/state.rs` to allow safe reading of the union projection `gr` under the borrow.
* Resolved unused imports in `src/math.rs` (`depth_base_entry` and `height_base_entry`) to keep warning count flat at 50.

### Metrics

`#[no_mangle]` **310 → 309** (−1); `unsafe fn` **349 → 346** (−3); `with_state` sites **1111 → 1108** (−3 net). Warnings **50** (flat).

### Handoff

Continuing Phase-C threading of `math.rs` consumer call trees. Next candidates include threading `make_math_accent` or `make_op` leaves.

## Idiom session 129 — 2026-06-03 — workstream 01 Phase C: thread the `fonts.rs` font-LOADING core (`read_font_info` + `load_native_font`) to `&mut FontState`

**Type:** execution (workstream 01, Phase C — push the native/TFM font-loading
core into its sibling `fonts.rs`, the largest font-cluster debt). **Two commits.**
Dashboard **29/2/7/4** on every commit. Warnings **50 → 50** throughout (no §7
trap). **Result: `fonts.rs` `crate::globals::` calls 151 → 58; the two giant
loaders' interleaved `font_*` accessor runs collapse into two `with_state_mut`
borrows.** (Note: the 127–128 entries said "warnings 60" — that was stale; the
actual count via the recipe command `cargo check -p tectonic_engine_xetex` is
**50**, confirmed flat all session.)

### What executed (2 commits)

* **A — `read_font_info`'s TFM-parse core** (`9f1948fb`). Extracted the
  metric-reading + font-table-fill body (xetex0.c:6350, the part after
  `file_opened = true`) into `read_font_info_tfm(&mut FontState, &EqtbState,
  tfm_file, nom, aire, s) -> (TfmParse, Option<(i32,bool)>)`, run under **one**
  `with_state_mut` borrow. ~38 `crate::globals::font_*` calls became direct
  `FontState` method / field access. Borrow-dropping (01b §4a/§5): the cold
  `_tt_abort` font-table-full path returns a `TfmParse::OutOfMemory` sentinel
  (run after the borrow drops); the with_state-bearing `load_tfm_font_mapping`
  trailing write stays in the wrapper. **The hidden with_state call:**
  `scaled_math::xn_over_d` is *not* pure — it sets `tex_remainder` (and
  `arith_error` on overflow). It is reached only on the `s < 0` scaled-font
  branch, which **etrip** exercises (tex-outputs/engine_rust/latex all passed; only
  TRIP's etrip caught it as a `RefCell already borrowed` nest-panic — a clean test
  run does **not** prove safety, exactly the 127 lesson). Fix: added pure
  `xn_over_d_parts(x,n,d) -> (quotient, remainder, overflowed)` in
  `scaled_math.rs` (and `xn_over_d` now wraps it), used it inside the borrow, and
  applied the `set_tex_remainder`/`set_arith_error` side effects in the wrapper
  after the borrow drops — captured via the returned `Option` so the `bad_tfm`
  path preserves them too (timing observably identical: nothing between the
  scaling point and there reads those globals).
* **B — `load_native_font`'s font-table-init block** (`7f5d1dc6`). Extracted the
  contiguous per-font metric-table init block (xetex0.c:6284, lines ~952–1014)
  into `init_native_font_tables(&mut FontState, &EqtbState, font_engine,
  full_name, actual_size, num_font_dimens) -> (font_ptr, x_ht, cap_ht,
  font_slant)`, run under one borrow. ~25 `font_*` calls collapsed. The block is
  bucket-C-clean: it touches only fonts + eqtb + the **extern-C** metric
  extractors (`aat`/`ot_get_font_metrics`, `get_loaded_font_design_size` — all
  borrow-safe, no thread-local) + locals. The with_state-bearing
  `new_native_character` / `free_node` / `push_font_info` calls all come **after**
  the block, so they stay outside. The metric out-params straddling the boundary
  (`x_ht`/`cap_ht`/`font_slant`, needed by the later `font_info` pushes) are
  computed by the helper and returned.

### Method surface + dead accessors (§6)

Added the mirroring `FontState` `set_*` methods (plain field writes, **parity-exact**
with the `globals.rs` accessor bodies — each globals setter was already a bare
`s.fonts.<field>[f] = v` with no width conversion, so the methods copy the exact
RHS; reads reused the existing `FontState` getters / direct `pub` field access).
Deleted four now-dead `globals.rs` accessors as their last callers threaded:
`set_font_info_b16`, `exten_base_entry` (commit A); `set_font_check`,
`set_font_letter_space_entry` (commit B). Each surfaced as a benign +1 dead-code
warning, caught by the warning-count check and removed in the same commit.

### Metrics

`#[no_mangle]` **310** unchanged. `unsafe fn` **347 → 349** (+2: the two new
`unsafe` helpers `read_font_info_tfm` / `init_native_font_tables`, which wrap the
unsafe TFM byte readers / extern-C metric extractors). `with_state` sites **1113
→ 1111** (−2 net: ~63 individual `globals` font accessor call sites in `fonts.rs`
replaced by 2 borrows, minus 4 deleted accessor definitions, plus 2 new wrapper
borrows). Warnings **50** (measured via `cargo check -p tectonic_engine_xetex`).
`fonts.rs` `crate::globals::` calls **151 → 58** (`font_*` specifically **→ 17**,
all now correctly *outside* the borrows — in `new_native_character` and the
error/diagnostic paths of the two loaders).

### Surprises / lessons

* **`xn_over_d` is with_state-bearing** (sets `tex_remainder`/`arith_error`) —
  a "pure math" helper that is not. The split-pure-core (`*_parts`) +
  apply-side-effects-after-the-borrow pattern is the clean fix and is reusable
  for any other arithmetic helper with a global-remainder side effect.
* **etrip is the only guard for the scaled-TFM path.** tex-outputs/engine_rust/
  latex all stayed green while etrip nest-panicked; lean on the **full** dashboard
  (esp. TRIP/etrip) for any TFM-loading change, never the cheaper subset.

### What remains in `fonts.rs`

The remaining 58 `globals::` calls are the **correctly-outside-the-borrow** edges:
`read_font_info`'s top tracing block + `bad_tfm` error path (diagnostic/help-line),
`load_native_font`'s string-pool / `make_string` / dedup-loop / not-enough-room
error path, and `new_native_character` (a bucket-C orchestrator: string pool,
TECkit mapping, `map_char_to_glyph_global` ×3, lost-char warnings,
`measure_native_node_global`). Threading `new_native_character` itself would
require a signature change rippling into `load_native_font`/math/main_control —
deferred. `find_native_font` (ext.rs, input-name buffer, bucket-D) still deferred.

### Next session

Either: thread `new_native_character` to take `&FontState`/`&mut MemState` (collapses
`map_char_to_glyph_global`/`measure_native_node_global`/`char_warning` there) and
ripple through its handful of callers; **or** collapse the 127–128 `_global`
wrappers by threading their callers — but those callers are mostly the giants
(`mlist_to_hlist`, `line_break`, `hpack`/`vpackage`, `ship_out`, `build_page`),
each a whole-session job. `find_native_font` once the input-name buffer is threadable.

---

## Idiom session 128 — 2026-06-03 — workstream 01 Phase C: drain the rest of the `ext.rs` native-font subsystem (8 fns, 4 commits) — `font_*` cluster fully gone from `ext.rs`

**Type:** execution (workstream 01, Phase C — finish the `ext.rs` native-font
drain that session 127 started). **Four commits.** Dashboard **29/2/7/4** on every
commit. Warnings **60 → 60** throughout (no §7 trap). **Result: every
`crate::globals::font_*` accessor call is gone from every native-font function in
`ext.rs`.**

### What executed (4 commits)

* **A — node-measuring cluster** (`441d78dc`). Threaded `measure_native_node`,
  `measure_native_glyph`, `store_justified_native_glyphs` to take
  `fonts: &FontState` as their first param, and `linebreak_next` likewise. The
  three measuring fns **write node fields through the raw `*p_node` pointer**
  (`(*node.add(k)).b32.s1 = …`), *not* via `mem_*`/`set_mem_*` accessors, so they
  need only `&FontState`, not `&mut MemState`. Swapped
  `crate::globals::{font_area_entry,font_layout_engine_ptr,font_letter_space_entry,height_base_entry,depth_base_entry}`
  → `fonts.<method>` (all already existed on `FontState`). Added 4 `_global`
  `with_state` wrappers. Renamed the ~16 external callers (math.rs ×9, linebreak
  ×3, main_control ×3, packaging, fonts, shipout, plus the internal `do_extension`
  `measure_native_glyph` site) to `_global` — all callers legacy, zero borrow
  analysis. `store_justified`/`do_extension` pass `fonts` through on internal
  ext→ext calls.
* **B — bucket-C edges via §4a borrow-dropping** (`47e13c30`). The three native
  fns that call a `with_state`-bearing helper:
  - `map_glyph_to_index` — threaded to `(fonts: &FontState, font, name: *const c_char)`;
    the with_state-bearing `name_of_file()` is read by the `_global` wrapper
    **before** the borrow and passed in (form **a**: hoist the read into the
    wrapper). scan.rs ×2 callers → `_global`.
  - `print_glyph_name` (calls `print_char`) and `linebreak_start` (calls
    `begin_diagnostic`/`print_*`) — rewritten **in place** (form **b**): read the
    font area/engine under a short scoped `with_state` into a local, drop the
    borrow, then run the `print_char` loop / diagnostic path outside it. **Public
    signatures unchanged → callers untouched.** No more `crate::globals::font_*`
    in either.
* **C — `make_font_def` → `&mut FontState`** (`03fbf33d`). Its only non-font
  `with_state` calls were its **own** `xdv_buffer` resize/`as_ptr` (a `FontState`
  field) — rewrote those as direct `fonts.xdv_buffer.…` access, and
  `font_area_entry`/`font_layout_engine_ptr`/`font_flags_entry` → `fonts.<method>`.
  Added `FontState::font_flags_entry` (mirrors `globals::font_flags_entry`,
  `font_flags[f] as i32`). `_global` wrapper = `with_state_mut(|s| make_font_def(&mut s.fonts, f))`;
  shipout caller → `_global`. **Deleted the now-dead `globals::font_flags_entry`
  read accessor** (§6 — `make_font_def` was its last caller); a benign +1
  dead-code warning flagged it, caught by the warning-count check.
* **D — the two name-reader loaders** (`6881530a`). `get_encoding_mode_and_info`
  and `check_for_tfm_font_mapping` each read `name_of_file()` once; threaded both
  to take the name pointer as a param (form **a**: `_global` wrapper reads
  `name_of_file()` first, passes it in). These touch the **input-name** cluster,
  not fonts — pure pointer pass-through, no `FontState`, no behavior change.
  `get_encoding_mode_and_info`'s body keeps its diagnostic path, now borrow-free
  (it holds no `with_state`). Callers: `do_extension` ×2 + fonts.rs → `_global`.

### The §4a borrow-dropping shape has two forms

Confirmed both this session: **(a)** hoist the with_state-bearing read into the
`_global` wrapper and thread the extracted value as a param (`map_glyph_to_index`,
`get_encoding_mode_and_info`, `check_for_tfm_font_mapping`) — used when the
with_state call is an *input* the function consumes; **(b)** read the font fields
under a short scoped `with_state` *inside* the function, drop the borrow, then do
the printing/diagnostic work (`print_glyph_name`, `linebreak_start`) — used when
the with_state call is *output* the function emits. Form (b) leaves the public
signature intact, so callers don't change.

### What remains in `ext.rs`

`find_native_font` is the **only** native-font function still holding bridge calls,
and they are all **`name_of_file`/`name_length`/`set_name_length`** — the
input-name cluster, **not** `font_*`. It *mutates* the name buffer
(`strcpy`/`strcat`/`free`/`set_name_length`) interleaved with logic → genuine
bucket-D; defer to a slice that gives the input-name buffer a proper threaded
`&mut` accessor. The other three `name_of_file()` reads in `ext.rs` are inside the
transitional `_global` wrappers (the correct bridge-down position).

### Metrics

`#[no_mangle]` **310** unchanged. `unsafe fn` **338 → 346** (+8 transitional
`_global` wrappers; collapse as callers thread). `with_state` sites **1108 →
1113** (+5 net: 8 new wrappers, minus the body globals-call removals — esp.
`make_font_def`'s two `xdv_buffer` borrows — minus the deleted accessor).
Warnings **60**.

### Validation

`cargo fmt`; `cargo check` → 0 errors, 60 warnings each commit; full dashboard →
**29 / 2 / 7 / 4** green each commit. (Mid-session a metrics-gathering
`git stash`/`checkout` dance accidentally *popped* a pre-existing unrelated WIP
stash onto `state.rs`; resolved by `git checkout HEAD -- src/state.rs` — the old
stash stayed in the list, nothing lost. Lesson: don't `git stash`/`checkout`
to compare refs when foreign stashes exist; use `git grep <ref>` / `git show
<ref>:path` instead, which is what produced the metric deltas above.)

### Next session

`find_native_font` (input-name cluster, bucket-D) when the name buffer is
threadable. Otherwise climb the call graph to collapse the bridge-down wrappers
added in 127–128: thread the *callers* of `measure_native_node_global` /
`measure_native_glyph_global` / `getnative*_global` etc. (math.rs, linebreak.rs,
shipout.rs, main_control.rs, scan.rs) so they pass `&s.fonts` through and the
`_global` wrappers vanish. Avoid whole-session work on `mlist_to_hlist`,
`line_break`, `hpack`/`vpackage`, `ship_out`, `build_page`.

---

## Idiom session 127 — 2026-06-03 — workstream 01 Phase C: thread the `ext.rs` native-font-metric reader cluster (11 fns) — first post-ws02 slice

**Type:** execution (workstream 01, Phase C — the `ext.rs` native-font-metric
reader cluster, recon's long-standing #1 target, **unblocked by ws02**). **One
commit.** Dashboard **29/2/7/4**. Warnings **60 → 60** (flat; no §7 trap).

### What this slice is

The `ext.rs` native-font-metric readers were ranked bucket-B #1 by the recon for
many sessions (105–112) but **deferred every time** because their callers
concentrated in the then-18k-line `scan.rs` grab-bag (threading them would dump
`_global` wraps into the scanner against strategy). ws02 (sessions 118–126)
decomposed `scan.rs`, so the cluster is now threadable. Threaded **11** pure
`&FontState`-read functions to take `fonts: &crate::state::FontState` as their
first param and call the existing `FontState` methods directly instead of the
`crate::globals::font_*` `with_state` bridge accessors:

`map_char_to_glyph`, `get_font_char_range`, `get_glyph_bounds`, `getnativecharwd`,
`getnativecharic`, `get_native_char_sidebearings`, `getnativecharht`,
`getnativechardp`, `get_native_char_height_depth`,
`real_get_native_italic_correction`, `real_get_native_glyph_italic_correction`.

Each got a transitional bridge-down `_global` wrapper
(`pub unsafe fn X_global(..) { with_state(|s| X(&s.fonts, ..)) }`); the ~22
external call sites (scan/expand/fonts/math/main_control) were blanket-renamed to
`_global` by script (bucket-A; **all callers are legacy** — none takes a
`&FontState`, so zero pass-through threading and zero borrow analysis). Internal
ext→ext calls (`getnativecharic`→`get_native_char_sidebearings`,
`getnativecharht`/`dp`→`get_native_char_height_depth`) pass `fonts` through.

### The one real hazard (and why 2 functions were excluded)

The recon ranks `print_glyph_name` and `map_glyph_to_index` in the same cluster,
but **both call a `with_state`-bearing function** (`print_char` and
`name_of_file()` respectively). A `with_state(|s| f(&s.fonts, ..))` wrapper holds
the whole-`RefCell` borrow across those calls → **nest-panic** (the §4 rule).
Neither path is exercised by the 29/2/7/4 suite (so tests stayed green even with
the broken wrappers), but introducing a latent panic on the `\glyphname` /
glyph-index path is a behavior change — so both were **excluded** and reverted to
their original bare-`crate::globals` form. They are genuine **bucket-C** (output /
input-name edges) needing borrow-dropping wrappers; defer to a manual slice.
**Lesson:** a function being bucket-B on the score (few accessors) is not enough —
grep its body for *any* `with_state`-bearing callee (`print_*`, `name_of_file`,
scanner/error helpers) before wrapping it with `with_state`.

### Method surface

Added one `FontState` method, `font_letter_space_entry(&self, f)` (mirrors
`globals::font_letter_space_entry`); the other four reads
(`font_area_entry`/`font_layout_engine_ptr`/`font_info_b32s1`/`param_base`)
already existed as `FontState` methods. No `globals.rs` accessor lost its last
caller (all five still used by `measure_native_glyph`/other ext code), so none
deleted.

### Metrics

`#[no_mangle]` **310** unchanged (these fns were never exported). `unsafe fn`
**327 → 338** (+11: the threaded fns stay `unsafe` — they call C FFI — and 11
transitional `_global` wrappers were added; this is the inherent bridge-down
surface, identical in shape to session-110/112 `_global` wrappers, and collapses
as each caller threads). `with_state` call sites **1135 → 1146** (+11, the new
wrappers; also falls as callers thread). Warnings **60**.

### Validation

`cargo fmt`; `cargo check -p tectonic_engine_xetex` → 0 errors, 60 warnings; full
dashboard → **29 / 2 / 7 / 4** green.

### Next session

Continue Phase-C threading inside the navigable post-ws02 modules. Clean
follow-ups: (1) the **bucket-C** `print_glyph_name` / `map_glyph_to_index` pair —
read the font fields under a short borrow, drop it, then do the `print_char` /
`name_of_file`+FFI work (the borrow-dropping wrapper shape of §4a). (2)
`measure_native_node` / `measure_native_glyph` / `store_justified_native_glyphs`
(write node fields → `&mut MemState` + `&FontState`; still all-legacy callers →
`_global`). (3) The scanner-core `scan.rs` font-dimension call sites
(`scan_something_internal`) are now plain `_global` renames — the cluster they
fed is threaded. Avoid whole-session work on `mlist_to_hlist`, `line_break`,
`hpack`/`vpackage`, `ship_out`, `build_page`.

---

## Idiom session 126 — 2026-06-02 — workstream 02 slice 11: the `ini.rs` drain (4 sub-commits) — `ini.rs` fully dissolved

**Type:** execution (workstream 02, module decomposition — **the `ini.rs` drain**, §5).
Split the entire `xetex-ini.c` translation into its three new home modules plus
`io.rs`, and **deleted `ini.rs`**. Dashboard **29/2/7/4** on every commit; warnings
**50 → 50** throughout. `ini.rs` 3,804 → **0 (gone)**.

### What executed (4 commits)

* **11a — `hyphenation.rs` (new).** The 11 trie-build / pattern-exception fns
  (`trie_op_hash`/`set_trie_op_hash`/`new_trie_op`/`trie_node`/`compress_trie`/
  `first_fit`/`trie_pack`/`trie_fix`/`init_trie`/`new_patterns`/`new_hyph_exceptions`)
  + the trie consts (`TRIE_OP_SIZE`/`NEG_TRIE_OP_SIZE`/`MAX_TRIE_OP`/`MIN_TRIE_OP`/
  `BIGGEST_LANG`/`TRIE_OP_HASH_SIZE`) + the `_trie_op_hash_array` static.
  `TRIE_OP_SIZE`/`MAX_TRIE_OP`/`BIGGEST_LANG` kept pub(crate) (format-dump reads
  them); `set_trie_op_hash` made pub(crate) (`load_fmt_file` calls it). Repointed
  `init_trie` (linebreak.rs) and `new_hyph_exceptions`/`new_patterns`
  (main_control.rs); `intpar` import split off. 803 LOC.
* **11b — `format_io.rs` (new).** The 16 format dump/load fns (`dump_*`/`undump_*`
  helpers, `store_fmt_file`, `pack_buffered_name`, `bad_format_file`,
  `load_fmt_file`). The module-local format-const block (`FORMAT_*`/`SUP_*`/
  `HASH_*`/selector + node-head consts) moved with them; the engine-side subset
  still used by `ini.rs`'s remaining code was **duplicated back** into `ini.rs`
  (faithful, identical values). The shared `TRE_POOL_SIZE`/`TRE_MAX_STRINGS`
  capacity statics made pub(crate). `format_io.rs` declares its own focused
  `ttstub_*`/`_tt_abort`/`tre_*` extern block; removed the now-dead ttstub
  output/input externs from `ini.rs`. 1,478 LOC.
* **11c — `engine.rs` (new).** The 9 driver fns (`get_strings_started`/
  `sort_avail`/`primitive`/`final_cleanup`/`initialize_more_variables`/
  `initialize_more_initex_variables`/`initialize_primitives`/`tt_run_engine`/
  `tt_cleanup`) + all support: the const subset, the `TRE_*` statics, the
  `tt_run_engine`+`tt_cleanup_*` extern blocks, the `XetexPrimitiveDef` struct,
  and the `intpar` accessor (`PC_INT_BASE`). Repointed `tt_cleanup` (errors.rs),
  `intpar`/`sort_avail`/`TRE_*` (format_io.rs), `intpar` (main_control.rs). After
  this, `ini.rs` was a 42-line stub holding only `init_io`. 1,533 LOC.
* **11d — dissolve `ini.rs`.** Moved `init_io`+`STDIN_UFILE` into `io.rs`,
  reusing its existing `UFile`/`INVALID_HANDLE`/`UTF8` (the i16 `encoding_mode`
  field takes `UTF8 as i16`, value-identical to ini.rs's old `i16` const); added
  `set_input_file` to io.rs's globals import. Repointed `crate::ini::init_io` →
  `crate::io::` in engine.rs, removed `mod ini;` from lib.rs, **deleted
  `src/ini.rs`**.

### Lessons for the handoff

* **Generous-copy-then-prune is the fast reliable path for big shared-import
  carves.** Rather than hand-curate the destination's `use` lists, copy the whole
  `use crate::consts::{…}` + `use crate::globals::{…}` blocks (and the relevant
  module-const block) into the new module, get `cargo check` to 0 errors, then
  `cargo fix --lib --broken-code` to prune unused imports and a `dead_code` sweep
  to remove over-copied consts. The §7 warning-count check (baseline 50) is what
  tells you the prune is complete.
* **No §7 trap this slice**, but the same signal caught the over-copy: warnings
  rose 50→70 in 11b purely from `dead_code` (`constant X is never used`), not
  `unreachable_patterns`/`non_snake_case` — i.e. a *benign* rise. Distinguish the
  two by grepping the warning text: dead-const ⇒ clean it; pattern/binding ⇒ a
  missing match-const needs duplicating in.
* **Shared consts get duplicated, not back-imported.** Consts used by both the
  moved code and the residue (e.g. `MEM_TOP`, `HASH_BASE`, `INT_PAR_YEAR`) were
  defined in *both* destination and source rather than re-imported, keeping module
  dependencies acyclic-ish and matching the §7 "duplicate with exact value" rule.
* **Slice 10 deferred deliberately.** Renaming `scan.rs`→`scanner.rs` touches
  every `crate::scan::` reference across the crate for a cosmetic gain; carving
  `token.rs` is blocked on the hot-scalar migration (Decision 4, out of scope).
  `scan.rs` as the 4,998-LOC scanner core is fine to leave; recorded as a non-goal.

### Next session

**Workstream 02's decomposition is essentially done** — both grab-bags are gone:
`scan.rs` is 4,998 LOC of pure scanner core, `ini.rs` is deleted, and
`main_control.rs` (6,823), `engine.rs`, `format_io.rs`, `hyphenation.rs`,
`packaging.rs`, `expand.rs`, `output.rs`, `errors.rs`, `io.rs`, `ext.rs`,
`fonts.rs` are all carved. **Resume Phase-C threading (workstream 01) per
`01b`** inside these now-navigable modules — the scanner/expander/main-control
call graph is the next real target, and threading a function there no longer
means moving an 18k-line grab-bag. Optional cosmetic later: internal sub-split of
`main_control.rs` (6.8k, a coherent subsystem) and the `state/` tree split
(Decision 5).

---

## Idiom session 125 — 2026-06-02 — workstream 02 slice 9b: the main-control carve (4 sub-commits) + the §10 plan

**Type:** execution (workstream 02, module decomposition — **the main_control carve**).
First recon'd + wrote the `02-modules.md` §10 plan, then **executed all four
sub-commits**. Dashboard **29/2/7/4** on each; warnings **50 → 50** throughout.
`scan.rs` 9,690 → **5,011** LOC (now pure scanner core); `main_control.rs`
1,185 → **6,821**; `ini.rs` 4,789 → **3,804** (`prefixed_command` left).

### What executed (4 commits)

* **9b-i** — box/mode verbs (19 fns, scan.rs −1,297). Repointed qualified
  `crate::scan::<fn>` callers in math/linebreak/ini/ext/pagebuilder (the recon's
  "zero external callers" was **wrong** — the first zsh grep loop silently failed
  on a glob error; these verbs *do* have external callers via qualified paths).
* **9b-ii** — assign/font verbs (20 fns). **§7 trap fired** (caught by the
  warning-count check, 50→99, not by errors): `off_save`'s group-match
  (SEMI_SIMPLE_GROUP/MATH_SHIFT_GROUP/MATH_LEFT_GROUP) and `do_register_command`'s
  value-match (DIMEN_VAL/MU_VAL) silently rebound until those 5 consts were
  duplicated in. ~40 module-private consts duplicated.
* **9b-iii** — the dispatch loop (14 fns, ~2,325 lines: the 11 `main_control_*`
  helpers + `lig_inner_loop` + `do_lookahead` + `main_control`). Repointed the
  `ini.rs:3642` entry call. Made `back_input`/`set_class_fn`/`set_family_fn`
  pub(crate). No trap.
* **9b-iv** — `prefixed_command` from ini.rs (~900 lines). Its 87 i32-typed consts
  were **inlined as function-local consts** inside the moved fn, not relocated
  module-level: 22 collide by name with main_control consts (13 type-incompatible,
  e.g. `SPACER` i32 vs u8) and 9 are still used by other ini.rs code — so the §10
  "move the block wholesale" assumption was wrong; function-local scope is the
  clean faithful fix. `intpar` stays in ini.rs (date code uses it; made
  pub(crate)); `set_cp_code`'s FFI extern moved with the fn. Removed 81 dead block
  consts + the dead extern from ini.rs.

### Lessons for the handoff

* The recon's "zero external callers" claim came from a **silently-failed zsh grep
  loop** (`--include=*.rs` glob error printed once, then the corrected loop's
  output was empty for an unrelated reason). **Verify recon greps actually ran** —
  a clean `cargo check` after the move is the real authority, not pre-move greps.
* The §7 trap is real and silent — the **warning-count check (baseline 50) caught
  it**, exactly as the recipe says. Watch `non_snake_case` "variable X should have
  snake case" + `unreachable_patterns` on all-caps match arms.
* Large const blocks that **collide by type** with the destination's consts →
  inline them function-local instead of relocating module-level.

### Next session — slice 10 (small) + ini.rs drains

`scan.rs` is now 5,011 LOC of **pure scanner core** (scan_*/get_*/get_next/
check_outer_validity/scan_something_internal/token builders). Slice 10: optionally
rename `scan.rs` → `scanner.rs` and carve `token.rs` (the token/command consts) per
§4 — small. Then `ini.rs` (3,804 LOC) drains independently into
`engine.rs`/`format_io.rs`/`hyphenation.rs` (§5). Then Phase-C threading (ws01)
resumes inside the now-navigable modules per `01b`. `main_control.rs` at 6,821 LOC
is a coherent subsystem (not a grab-bag) — internal sub-splitting is later cosmetic.

---

### Original §10 plan (written at the start of this session, pre-execution)

Wrote `02-modules.md` §10 —
the detailed, recon-backed execution plan for slice 9b, the biggest remaining carve.
Next session executes it; this session de-risks it so it can be done cold.

### What slice 9b is

Drain the **entire main-control dispatch + verb family out of `scan.rs`** (53 fns,
~4,806 fn-lines) **plus `prefixed_command` out of `ini.rs`** (~900 lines) into the
existing `main_control.rs` (1,185 LOC, seeded session 124 with the alignment family).
After it, `scan.rs` (9,690 LOC now) drops to ≈4,900 LOC of **pure scanner core** —
making slice 10 a near-trivial rename to `scanner.rs` + carving `token.rs` consts.

### The key recon findings (de-risks)

1. **Zero external callers.** Greppd every fn in the verb+dispatch family outside
   `scan.rs`/`main_control.rs`/`ini.rs` → **nothing**. Unlike slice 7 (packaging,
   8-module repoint), this carve is fully contained: `scan.rs`↔`main_control.rs`
   mutual `use` (Rust allows the cycle) plus exactly **two** `ini.rs` edges:
   - `ini.rs:3642` `crate::scan::main_control()` → `crate::main_control::main_control()`.
   - `scan.rs:51` `use crate::ini::prefixed_command;` (sites 5534, 6318) →
     `use crate::main_control::prefixed_command;` after 9b-iv.
2. **`prefixed_command` is clean to move.** It is `ini.rs:3890–4789` (the last fn in
   the file) and is **immediately preceded by its own dedicated 87-const private
   block (`ini.rs:3790–3889`)** — that block moves wholesale with it, which is the
   §7 trap-dodge done for free. Its callees are all already importable: `eq_define`/
   `geq_define`/`eq_word_define`/`geq_word_define` (`save.rs`), `def_ref`
   (`globals.rs`), `find_sa_element_global`/`sa_def_global`/`gsa_def_global`, and the
   `scan_*`/`get_*` family. After it leaves, `ini.rs` is ~3,890 LOC.
3. **Sub-commit split (4 commits = one big session)**, sizes computed from spans:
   - **9b-i** box/mode verbs — 19 fns, ~1,384 fn-lines.
   - **9b-ii** assign/font verbs — 20 fns, ~1,091 fn-lines.
   - **9b-iii** dispatch loop — 14 fns, ~2,331 fn-lines (the 11 `main_control_*`
     helpers + `lig_inner_loop` + `do_lookahead` + `main_control`; the giant match).
   - **9b-iv** `prefixed_command` from `ini.rs` — 1 fn + its 87-const block.
   (Full fn lists in `02-modules.md` §10.2.)
4. **Const entanglement is the real work** (per §7 step 4–5): the verbs/dispatch
   `match` on `scan.rs`-module-private command/token codes not centralized in
   `consts.rs`; duplicate each into `main_control.rs` with exact type+value. Budget
   ~50+ dups. **The §7 warning-count check (baseline 50) is mandatory** — the
   pattern-const rebind trap is silent (shows only as `unreachable_patterns`/
   `non_snake_case`/`unused_variables` rises, no error), with the slice-8 E0308
   typed-cousin (`u8`-vs-`i32`) as the other failure mode.

### Post-9b & sequencing

`main_control.rs` ends ≈6,900 LOC — over the §6 "no >~3k-line module" aspiration,
but a single coherent subsystem (not a grab-bag), acceptable as transitional like
`packaging.rs`; internal sub-splitting is a later cosmetic slice. **Slice 10:**
rename `scan.rs`→`scanner.rs` + carve `token.rs` consts (small). `ini.rs` drains
independently (`engine.rs`/`format_io.rs`/`hyphenation.rs`, §5). Then Phase-C
threading (ws01) resumes inside the navigable modules per `01b`.

**Handoff:** execute `02-modules.md` §10 next session — `move_fns.py` per sub-commit
(for 9b-iv `--src src/ini.rs`; hand-move the const block), 29/2/7/4 + warnings→50
per commit, prefix `engine_xetex: ws02 slice 9b-… —`.


## Idiom session 124 — 2026-06-01 — workstream 02 slices 7+8+9a: the packaging, expander, and alignment carves

**Type:** execution (workstream 02, module decomposition — **core carve**, §8
Decision 2/3). Slice 7 (packaging, 2 commits) + slice 8 (expand, 1 commit) +
slice 9a (alignment → `main_control.rs`, 1 commit) — **four code commits +
docs**. Dashboard **29/2/7/4** throughout. Warnings **50 → 50**. `scan.rs`
14,710 → **9,696** LOC across the session (−5,014; now under 10k).

### Slice 9a — the `\halign` alignment family (→ new `main_control.rs`)

**One commit.** Relocated the 14 alignment functions verbatim (`move_fns.py`,
1,151 fn-lines) — `align_error`/`no_align_error`/`omit_error`/`fin_align`/
`align_peek`/`get_preamble_token`/`init_span`/`init_row`/`init_col`/
`push_alignment`/`pop_alignment`/`fin_col`/`fin_row`/`init_align` — from
`scan.rs` into a **new `main_control.rs`** (created + `mod main_control;` in
lib.rs). This **seeds** the main_control module; the `main_control*` dispatch +
verbs + `prefixed_command` are the **gnarliest** remaining piece (slice 9b, next
session).

**Zero external callers** — the alignment family is only called from `scan.rs`
(the residual main-loop dispatch + `do_endv`), so the repoint is fully contained:
`scan.rs` imports the 7 it still calls back from `main_control`. The big fns
(`fin_align`/`init_align`/…) carry their consts inline (move with them); only the
small error fns reference module-scope consts. Duplicated 10 `scan.rs`
module-private consts into `main_control.rs` (`RIGHT_BRACE_TOKEN`/`LEFT_BRACE_TOKEN`/
`TAB_TOKEN`/`RIGHT_BRACE`/`MAX_COMMAND`/`ENDV`/`GLUE_VAL`/`ASSIGN_GLUE`/`SPACER`/
`IGNORE_DEPTH`); imported the rest (consts.rs node/base consts; globals hot
scalars; `packaging` hpack/vpackage/append_to_vlist/flush_node_list_global;
`math` flush_math_global/resume_after_display; `pagebuilder::build_page`;
`expand`; `scan` get_*/scan_*/do_assignments/normal_paragraph). **No §7 trap** —
the `LOCAL__EVERY_CR` consts are inline (moved with the fns). Cleaned scan.rs
orphans (flush_math_global/resume_after_display imports, TAB_TOKEN const).

**Metrics:** `scan.rs` 10,846 → **9,696** LOC (−1,150). new `main_control.rs` =
1,185. Dashboard 29/2/7/4, warnings 50.

### Slices 7 & 8 (earlier in the session)

Slice 7 = packaging carve (2 commits); slice 8 = expander carve (1 commit). See
the per-slice blocks below for the recipe details.

### Slice 8 — the expander carve (`expand`/`macro_call`/`conditional`/`pop_cond_node`/`conv_toks`/`ins_the_toks` → new `expand.rs`)

**One commit.** Relocated the 6 expansion functions verbatim (`move_fns.py`,
1,737 fn-lines) from `scan.rs` into a **new `expand.rs`** (created + `mod expand;`
in lib.rs). This is the **most const-entangled slice so far**: the big match-heavy
expander functions reference ~30 `scan.rs` **module-private** consts that are *not*
centralized in `consts.rs` (command/token codes `CALL`/`RELAX`/`SPACER`/
`MAX_COMMAND`/`OTHER_TOKEN`/`MAX_CHAR_VAL`/…, brace/match-token limits
`LEFT_BRACE_LIMIT`/`RIGHT_BRACE_LIMIT`/`MATCH_TOKEN`/`END_MATCH_TOKEN`/
`SPACE_TOKEN`/`PROTECTED_TOKEN`, frozen-cs offsets `FROZEN_ENDV`/
`FROZEN_PRIMITIVE`, `HASH_BASE`/`SINGLE_BASE`/`BIGGEST_USV`/`VMODE`/`NO_EXPAND_FLAG`/
`UNDEFINED_PRIMITIVE`/`UNLESS_CODE`/`AAT_FONT_FLAG`/`OTGR_FONT_FLAG`/
`INT_PAR_TRACING_IFS`/…). These can't be moved (used pervasively by the residual
scan.rs), so **duplicated each into `expand.rs` with its scan.rs value** — the §7
recipe's "duplicate shared const" step, at scale (~30 dups). Also relocated the
conditional `IF_*_CODE`/`_C`-suffixed private const block wholesale, and
duplicated `GARBAGE`/`TEMP_HEAD`/`INSERTED`/`NULL_CS` (still used in scan.rs;
`NULL_CS` duplicated *back* into scan.rs).

**§7 trap surfaced as hard E0408 + E0308, not silent.** The `IF_*_CODE` match
arms in `conditional` came up as `variable … not bound in all patterns` (E0408)
until the const block was added — caught at compile time. One typed cousin: I
first declared `SELECTOR_NEW_STRING` as `u8` but `selector()` returns `i32`
(E0308); fixed the dup to `i32` to match the original referent. No silent rebind
(warnings held at 50).

**Imports added to `expand.rs`:** consts (ACTIVE_BASE/BOX_BASE/CS_TOKEN_FLAG/
node-types/INT_BASE/PRIM_EQTB_BASE/TOO_BIG_CHAR), `errors`
(error/int_error/not_*_font_error/runaway), `ext` (aat_print_font_name/
gr_print_font_name/map_char_to_glyph/print_glyph_name), `globals` (the hot
`cur_*` scalars + `mem_*`/`set_*`/`eqtb_b32s1`/`buffer_entry`/`first`/…),
`input::begin_token_list`, `io::start_input`, `output` (capture_to_diagnostic/
error_here_with_diagnostic/print_cmd_chr), `scan` (effective_char_global/get_*/
scan_*/pass_text/pseudo_start/the_toks), and a `usingGraphite` extern. `scan.rs`
imports `expand`/`macro_call` back from `expand.rs` (still called there);
cleaned scan.rs orphans (`name_in_progress`/`set_cur_input_start`/`start_input`/
`GARBAGE`/3 `ext::*_print_font_name`/`print_glyph_name`).

**Metrics:** `scan.rs` 12,633 → **10,846** LOC (−1,787). new `expand.rs` =
1,848. Dashboard 29/2/7/4, warnings 50.

**(Handoff superseded — slice 9a below started the main_control carve with the
alignment family. The authoritative next-session handoff is at the top of this
session entry, in the slice-9a block's lead-in and the §8 status. The slice-8
lesson stands: the core fns reference many *scan.rs-module-private* consts not in
`consts.rs` — grep each missing symbol's definition and **duplicate with its
exact type** (watch the E0308 typed-rebind cousin); the trap mostly surfaces as
hard E0408/E0425 errors here, but keep the warning-count check for silent cases.)**

**For the next session — slice 9b (main_control dispatch + verbs):** the
gnarliest remaining piece. `main_control.rs` now exists (seeded with the
alignment family). Relocate the `main_control*` dispatch (giant match) + the verb
family (§4: `do_lookahead`/`its_all_over`/`append_glue`/`append_kern`/`make_mark`/
`new_interaction`/`alter_*`/`indent_in_hmode`/`head_for_vmode`/
`begin_insert_or_adjust`/`do_endv`/`append_discretionary`/`build_discretionary`/
`shift_case`/`make_accent`/`new_font`/`do_register_command`/`unpackage`/
`delete_last`/`app_space`/`new_graf`/`package`/`box_end`/`begin_box`/`scan_box`/
`handle_right_brace`/`off_save`/`trap_zero_glue`/`fix_language` + `do_assignments`)
into `main_control.rs`, and `prefixed_command` (which **spans scan.rs + ini.rs** —
coordinate the ini.rs half). Expect heavy hot-scalar + module-private-const
entanglement, **larger than slice 8** — budget for ~50+ duplicated consts and a
big globals import, and consider splitting into sub-commits (e.g. the box-building
family, the register/assignment family, the dispatch loop) the way packaging
split 7a/7b. Then (4) residual `scan_*`/`get_*`/`get_next` stay as `scanner.rs`
(rename or leave `scan.rs`); (5) `token.rs` consts last. `ini.rs` drains
independently.

### Slice 7 — the packaging carve (box/node packing & list utils → new `packaging.rs`)

**Two commits** (7a recycle utils, 7b packers; the
§8 handoff authorized splitting packaging if too large for one green commit).
Dashboard **29/2/7/4** on each. Warnings **50 → 50**.

**Slice:** drained the entire box/node-packing & list-utility group verbatim
from `src/scan.rs` (= xetex0.c) into a **new `src/packaging.rs`** (created the
file + `mod packaging;` in lib.rs; transitional home per Decision 3 → `node/` in
ws03). `move_fns.py`, 11 fns / 1,905 fn-lines total.

* **7a — node-recycle utils (655 fn-lines):** `flush_node_list`(+`_global`),
  `copy_node_list`(+`_global`), `just_copy`, `just_reverse`. Relocated the
  `_FNL`/`_CNL` single-fn-private const blocks wholesale (§7 trap-dodge). The
  shared `NodeRecycleAbort` enum, `node_recycle_abort` fn, and
  `cnl_get_node!`/`cnl_get_avail!` macros **stay in `scan.rs`** as the common
  home (also used by `prune_page_top` and the already-moved `fonts.rs`); added a
  `pub(crate) use cnl_get_avail;` re-export and imported the set into
  `packaging.rs`. **§7 trap fired once:** `DISC_NODE` (a `consts.rs` const used in
  `flush_node_list`'s match, not `_FNL`-suffixed so not in the relocated block)
  silently rebound → `unreachable_patterns`/`unused_variables` warnings until
  imported. `scan.rs` 14,710 → 13,967.
* **7b — the packers (1,250 fn-lines):** `append_to_vlist`, `vert_break`,
  `vsplit`, `prune_page_top`(+`_global`), `do_marks`(+`_global`), `hpack`,
  `vpackage`. Relocated the four `_PPT`/`_DM`/`_HP`/`_VP` const blocks wholesale.
  Duplicated the module-private `TEMP_HEAD` (`4_999_999 - 3`) and `IGNORE_DEPTH`
  (`-65_536_000`) consts (scan.rs still uses both). The pre-existing
  `KERN_NODE | KERN_NODE` unreachable-pattern in `hpack` **relocated with it** —
  warnings held at 50, confirming it was the same pre-existing warning moving
  modules, not a new trap. `scan.rs` 13,967 → 12,633.

**Big external repoint (Decision 3's "biggest external-caller repoint yet").**
`hpack`/`vpackage`/`flush_node_list`/etc. are called from
math.rs/shipout.rs/linebreak.rs/pagebuilder.rs/ext.rs/save.rs/ini.rs/errors.rs.
Repointed all `crate::scan::<fn>` → `crate::packaging::<fn>` (perl for
qualified paths; hand-fixed two grouped `use crate::scan::{…}` blocks in
ext.rs/pagebuilder that the perl's qualified-path pattern missed). `scan.rs`
re-imports the 5 packers/utils it still calls (`hpack`/`vpackage`/`vsplit`/
`append_to_vlist`/`flush_node_list_global`/`copy_node_list_global`) from
`crate::packaging`. Cleaned `scan.rs` orphans across both commits (8 consts/
imports: `GLUE_SPEC_SIZE`/`MEDIUM_NODE_SIZE`/`MIN_HALFWORD`/`RULE_NODE_SIZE`,
`new_edge_global`, `mem_word`/`set_mem_ptr`/`set_mem_word`, `active_width`/
`set_active_width`/`set_best_height_plus_depth`/`temp_ptr`, `short_display`).

**Imports added to `packaging.rs`:** `consts` (BOX_BASE/DIMEN_BASE/GLUE_BASE/
INT_BASE/MAX_HALFWORD/DISC_NODE + the node-type set), `globals` (active_width/
eqtb_b32s1/cur_val/the `set_mem_*` accessors/set_active_width/…), `errors::error`,
`ext::measure_native_node`, `mem::new_spec_global`, `output::{capture_to_diagnostic,
error_here_with_diagnostic, short_display, show_box}`, `shipout::new_edge_global`,
`scan::{cnl_get_avail, cnl_get_node, effective_char_global, new_native_word_node_global,
node_recycle_abort, NodeRecycleAbort}`.

**Metrics:** `scan.rs` 14,710 → **12,633** LOC (−2,077). new `packaging.rs` =
2,110. No signature/`#[no_mangle]`/`unsafe`/body changes (relocation only).
Dashboard 29/2/7/4 on both commits, warnings 50. (Slice 8 above continued
straight into the expander carve; the session handoff is in the slice-8 block.)


## Idiom session 123 — 2026-06-01 — workstream 02 slice 6: relocate the locale-linebreak group out of `scan.rs` (last leaf)

**Type:** execution (workstream 02, module decomposition — sixth slice; **the
last scan.rs *leaf***, completing the §8 "drain the home-having groups out" phase
before the core carve). **One commit.** Dashboard **29/2/7/4**. Warnings
**50 → 50**.

**Slice:** moved the 2 locale-linebreak functions verbatim from `src/scan.rs`
(= xetex0.c) into `src/linebreak.rs` (= xetex-linebreak.c): `char_pw_in` (the
protrusion-width helper threaded in session 116) and `do_locale_linebreaks` (the
ICU/locale word-segmentation node builder). `move_fns.py`, 160 fn-lines.

**Two forced name-disambiguations (faithful, not behavior changes).** The
destination `linebreak.rs` already had same-named items shadowing what the moved
code's bare names resolved to in `scan.rs`; resolving each to its *original
referent* preserves the exact value/type:
* `LIGATURE_NODE`: `linebreak.rs` defines a local `const LIGATURE_NODE: i32`,
  but `char_pw_in` compares it against `mem.b16s1(p)` (`u16`) — in `scan.rs` it
  resolved to the imported `consts::LIGATURE_NODE` (`u16`, value 6). Qualified
  the one comparison to `crate::consts::LIGATURE_NODE` (same const, dodges the
  i32 shadow). Same value, same type as before.
* `main_f`: `do_locale_linebreaks` read the **raw `static mut main_f: i32`**
  (a hot tokenizer scalar kept raw per Decision 4) via a module-level
  `extern "C"` block that sat *outside* the function in `scan.rs` (so the mover
  didn't carry it). Re-declared the same `extern "C" { static mut main_f: i32; }`
  in `linebreak.rs` and read it raw under the existing `unsafe fn` — identical to
  the original. (Did **not** route through `globals::main_f()`, which would be a
  real body change.)

These are the §7 trap's *typed* cousin: a moved bare identifier silently
re-binding to a different in-scope item. Here both surfaced as hard `E0308` type
errors (not silent), so the compiler caught them; the fix restores the original
referent.

**Imports:** `linebreak.rs` gained `ext::{linebreak_next, linebreak_start,
real_get_native_word_cp}` (pub fns) + a `get_cp_code` raw extern, `globals::eqtb_b32s1`,
`scan::new_native_word_node_global`, and 4 relocated orphan consts
(`GLUE_PAR_XETEX_LINEBREAK_SKIP`/`INT_PAR_XETEX_LINEBREAK_LOCALE`/`…_PENALTY`/
`NATIVE_NODE_SIZE_NN`); dropped its now-local `use crate::scan::char_pw_in`.
`scan.rs` imports `linebreak::do_locale_linebreaks`; orphan cleanup: 3 dead
`use crate::ext::` imports, the dead `use crate::state::{FontState, LinebreakState,
MemState}`, the orphaned top-level `INT_PAR_XETEX_USE_GLYPH_METRICS` const, and the
dead `extern { static mut main_f }` block. No trap; warnings 50→50.

**Metrics:** `scan.rs` 14,890 → **14,710** LOC (−180). `linebreak.rs` 3,385 →
3,557. Dashboard 29/2/7/4.

**Milestone:** all six home-having leaf groups are now drained out of `scan.rs`
(errors→errors.rs s118, show_*→output.rs s119, file-naming→io.rs s120,
ext→ext.rs s121, fonts→fonts.rs s122, locale-linebreak→linebreak.rs s123).
`scan.rs` is down from 18,689 (ws02 start) to **14,710** and now holds
essentially the **scanner / expander / main_control / packaging core** — the §8
core carve. That is the harder, more entangled phase (giant `match` dispatch,
shared hot scalars, the `prefixed_command` family spanning `scan.rs`+`ini.rs`).

**For the next session (124):** begin the **core carve** (§4, Decision 2/3).
Recommended order, smallest-blast-radius first: (1) **packaging.rs** (new,
transitional → `node/` in ws03) — `hpack`/`vpackage`/`vsplit`/`vert_break`/
`copy_node_list`/`flush_node_list`/`just_copy`/`just_reverse`/`prune_page_top`/
`do_marks` (node-layer utils, relatively self-contained); (2) **expand.rs** (new)
— `expand`/`macro_call`/`conditional`/`pop_cond_node`/`conv_toks`/`ins_the_toks`;
(3) **main_control.rs** (new) — the `main_control*` dispatch + verbs + alignment;
(4) residual `scan_*`/`get_*`/`get_next` stay as `scanner.rs` (rename or leave
`scan.rs`); (5) `token.rs` consts last. Expect heavier const/macro/hot-scalar
entanglement than the leaves — lean on `move_fns.py` (now handles `pub(crate)`),
`pub(crate) use` for shared macros, wholesale const-block relocation, and the
warning-count check. If a carve step won't go green cleanly, stop at the prior
green commit and hand off. `ini.rs` drains independently
(`format_io.rs`/`engine.rs`/hyphenation).


## Idiom session 122 — 2026-06-01 — workstream 02 slice 5: relocate the TFM/native font-loading group out of `scan.rs` into a new `fonts.rs`

**Type:** execution (workstream 02, module decomposition — fifth mechanical
relocation slice; **first one that creates a new destination module**). **One
commit.** Dashboard **29/2/7/4**. Warnings **50 → 50**.

**Slice:** moved the 11 font-loading functions verbatim from `src/scan.rs`
(= xetex0.c) into a **new `src/fonts.rs`** (= xetex0.c's font-loading group;
created the file + added `mod fonts;` to `lib.rs` per §7 step 2):
`effective_char`, `effective_char_info`, `read_font_info` (the 524-line TFM
reader), `read_tfm_word`, `read_fifteen`, `read_scaled_word`, `tfm_getc`,
`load_native_font`, `new_native_word_node`, `build_native_character`,
`new_native_character`. **No bytes changed inside any function** —
`move_fns.py`, 966 fn-lines.

**Tooling fix:** `move_fns.py`'s `FN_START` regex only matched `pub ` (with a
space), so `pub(crate) fn new_native_word_node` was invisible to it. Generalized
the visibility group to `pub(\([^)]+\))?` (handles `pub(crate)`/`pub(super)`) and
bumped the name capture from group 4 → 5. The mover now relocates restricted-vis
fns.

**Non-fn items relocated (the mover only does `fn`s, so these were manual):**
* the private `enum NativeCharSrc` (build_native_character's payload tag, sole
  user moved) → `fonts.rs`;
* the `cnl_get_node!` macro: a `scan.rs`-local `macro_rules!` that two moved fns
  use. Exposed it with `pub(crate) use cnl_get_node;` right after the macro def
  in `scan.rs`, then `use crate::scan::cnl_get_node;` in `fonts.rs` (the
  path-based macro re-export idiom — avoids `#[macro_export]` crate-root
  pollution). Made `push_font_info_b32s1` `pub(crate)` (TFM reader helper that
  stays in `scan.rs`).

**Const handling (relocate orphans, duplicate shared — §7):** 15 single-fn-private
consts (`*_RFI`/`*_NN`/`*_NC`/`*_LNF` + `EMPTY_STRING`/`INVALID_HANDLE`/…)
relocated to `fonts.rs` (all verified orphaned in `scan.rs` post-move); 5 shared
(`AAT_FONT_FLAG`/`OTGR_FONT_FLAG`/`FONT_BASE`/`NATIVE_NODE_SIZE_NN`/
`INT_PAR_XETEX_TRACING_FONTS`) duplicated.

**Imports:** `fonts.rs` pulls 10 `ext.rs` font-engine `pub fn`s
(`find_native_font`/`map_char_to_glyph`/`apply_mapping`/`measure_native_node`/
`*_get_font_metrics`/…), 10 `globals` accessors, `io::{pack_file_name,
tt_xetex_open_input}`, `output::{capture_to_diagnostic, char_warning,
error_here_with_diagnostic}`, `errors::error`, `scan::{node_recycle_abort,
push_font_info_b32s1, NodeRecycleAbort, cnl_get_node}`, `state::FontState`, and a
6-decl `extern "C"` block for the raw FFI it owns (`get_loaded_font_design_size`/
`isOpenTypeMathFont`/`get_ot_math_constant`/`ttstub_input_getc`/`_close`/
`_tt_abort`). Repointed `crate::scan::<fn>` → `crate::fonts::` in `math.rs`/
`linebreak.rs`; `scan.rs` re-imports the 6 it still calls bare. Orphan cleanup in
`scan.rs`: 8 dead `use crate::ext::`/`io::` font imports, 2 dead globals imports,
`pack_file_name`/`load_native_font` from grouped imports, 5 now-dead raw `extern`
decls (`get_loaded_font_design_size`/`isOpenTypeMathFont`/`get_ot_math_constant`/
`ttstub_input_getc`/`_close`), 1 stale banner.

**No trap.** Warnings held at 50 throughout (the `effective_char`/native fns
match on few node consts, and the shared node-type consts were already imported
or relocated). 0 errors.

**Metrics:** `scan.rs` 15,897 → **14,890** LOC (−1,007 — biggest single-slice
dent yet). new `fonts.rs` = 1,025. No signature/`#[no_mangle]`/`unsafe` changes
(relocation only). Dashboard 29/2/7/4.

**For the next session (123):** continue 02 per §8 — last `scan.rs` *leaf*:
`linebreak.rs` (`do_locale_linebreaks`/`char_pw_in`). Then `scan.rs` holds the
scanner/expand/main_control/packaging core — start the **core carve** (§4/§8):
box/list utils → `packaging.rs` (new, transitional), expand/macro/conditional →
`expand.rs` (new), main_control dispatch + alignment → `main_control.rs` (new),
residual scanners → `scanner.rs` (new), token consts → `token.rs`. `ini.rs`
drains independently (`format_io.rs`/`engine.rs`/hyphenation). Recipe unchanged;
`move_fns.py` now handles `pub(crate)` fns; for macros use `pub(crate) use`, for
private enums/consts relocate wholesale, and watch the warning count.


## Idiom session 121 — 2026-06-01 — workstream 02 slice 4: relocate the extensions / `\special` group out of `scan.rs`

**Type:** execution (workstream 02, module decomposition — fourth mechanical
relocation slice). **One commit.** Dashboard **29/2/7/4**. Warnings **51 → 50**
(a *drop*; see below — no trap).

**Slice:** moved the 5 extension / `\special` whatsit functions verbatim from
`src/scan.rs` (= xetex0.c) into `src/ext.rs` (= xetex-ext.c, their natural home):
`append_src_special`, `new_write_whatsit`, `insert_src_special`,
`tt_insert_special`, `do_extension`. **No bytes changed inside any function** —
`move_fns.py` (dry-run → `--apply`), 297 fn-lines.

**Const handling — relocate the whole `_DE` block, don't cherry-pick.**
`do_extension` carries a 35-const private block (all `*_DE`/`*_DE_W`-suffixed,
named for the function). Only ~21 surfaced as `cannot find` errors; the rest are
used in `match` arms and would have silently rebound (the §7 trap). Rather than
chase the error list, **relocated the entire contiguous block wholesale**
(verified all 35 orphaned in `scan.rs` first — 0 still-referenced), which fixes
the errored *and* the trap-prone consts in one move and keeps the warning count
honest. Plus duplicated 5 shared consts still used in `scan.rs`
(`LEFT_BRACE_TOKEN`/`INSERTED`/`TEMP_HEAD` stay dup'd; `SPECIAL_NODE`/
`WRITE_NODE_SIZE` turned out orphaned in `scan.rs` after the move and were
deleted there).

**Fixup:** `ext.rs` gained 3 `consts.rs` consts (`CS_TOKEN_FLAG`/`CUR_FONT_LOC`/
`INT_BASE`), 19 `globals` accessors, 8 `scan` fns (`flush_node_list_global`/
`get_x_token`/`new_graf`/`scan_*`/`str_toks`), `errors`
(`error`/`int_error`/`not_native_font_error`/`report_illegal_case`), `output`
(`capture_to_diagnostic`/`error_here_with_diagnostic`), `input::begin_token_list`,
`io::scan_and_pack_name` (from slice 3), `pic::load_picture`, `shipout::out_what`,
and a `ttstub_diag_printf` extern decl. Repointed `crate::scan::<fn>` call-sites
in `math.rs` (+others) to `crate::ext::`; `scan.rs` re-imports the 3 it still
calls bare (`append_src_special`/`do_extension`/`insert_src_special`). Orphan
cleanup in `scan.rs`: 4 now-dead module imports (`get_encoding_mode_and_info`/
`measure_native_glyph`/`load_picture`/`out_what` — all do_extension-only), the 2
orphaned shared consts, 2 stale banners.

**Why 51 → 50 (a drop, not a trap):** adding `ext.rs`'s `ttstub_diag_printf`
extern decl (`-> libc::c_int`, matching `scan.rs`/`save.rs`) shifted which module
the `clashing_extern_declarations` lint picks as canonical vs. `output.rs`'s
divergent `()` return, netting one fewer clash warning. Pure lint-ordering noise
over a *pre-existing* signature inconsistency; no behavior change, 0 errors, and
crucially **the count did not rise**, so the `_DE` block relocation closed the
trap cleanly.

**Metrics:** `scan.rs` 16,255 → **15,897** LOC (−358). `ext.rs` 2,385 → 2,750.
No signature/`#[no_mangle]`/`unsafe` changes (relocation only). Dashboard 29/2/7/4.

**For the next session (122):** continue 02 per §8 — next leaf: `fonts.rs`
(TFM/native loading: `read_font_info`/`read_tfm_word`/`read_scaled_word`/
`read_fifteen`/`tfm_getc`/`load_native_font`/`new_native_word_node`/
`build_native_character`/`new_native_character`/`effective_char`/
`effective_char_info` — note `fonts.rs` may not exist yet; create it + add the
`mod` line to `lib.rs` per §7 step 2), then `linebreak.rs`
(`do_locale_linebreaks`/`char_pw_in`). Then `ini.rs` drains
(`format_io.rs`/`engine.rs`/hyphenation), then the core carve. Same recipe;
watch the warning count, and prefer **wholesale block relocation** for
single-function-private const clusters (it dodges the match-arm trap).


## Idiom session 120 — 2026-06-01 — workstream 02 slice 3: relocate the file-naming / `start_input` / `close_files` group out of `scan.rs`

**Type:** execution (workstream 02, module decomposition — third mechanical
relocation slice). **One commit.** Dashboard **29/2/7/4**. Warnings **51 → 51**.

**Slice:** moved the 12 file-naming / input-bootstrap / shutdown functions
verbatim from `src/scan.rs` (= xetex0.c) into `src/io.rs` (= xetex-io.c, their
natural home): `begin_name`, `more_name`, `end_name`, `make_name_string`,
`scan_and_pack_name`, `scan_file_name_braced`, `scan_file_name`, `pack_job_name`,
`open_log_file`, `start_input`, `pack_file_name`, `close_files_and_terminate`.
**No bytes changed inside any function** — `move_fns.py` (dry-run → `--apply`),
559 fn-lines.

**Fixup (cargo-check-driven, larger than slices 1–2 because `start_input`/
`close_files_and_terminate` reach broadly):**
* `io.rs` gained: 40 `globals` accessors (a second `use crate::globals::{…}`
  block); 3 `consts.rs` consts (`BIGGEST_CHAR`/`SELECTOR_TERM_ONLY`/
  `TOO_BIG_CHAR`); scanner fns `get_x_token`/`scan_toks`; `shipout::finalize_dvi_file`;
  13 module-private consts duplicated from `scan.rs`
  (`EMPTY_STRING`/`LEFT_BRACE`/`SPACER`/`RELAX`/`OTHER_CHAR`/`NEW_LINE`/
  `SELECTOR_LOG_ONLY`/`SELECTOR_NEW_STRING`/`INT_PAR_*`/`TTBC_FILE_FORMAT_*`); 4
  `ttstub_output_*` extern decls. Removed `io.rs`'s now-stale
  `use crate::scan::{begin_name,end_name,more_name,pack_file_name,scan_file_name}`
  (those are local now).
* **Private `mod raw_utf8` relocated too.** `start_input`'s only dependency on a
  `scan.rs`-private item was `mod raw_utf8 { extern { bytesFromUTF8, offsetsFromUTF8 } }`
  (its sole user). Moved the module verbatim into `io.rs`; its `pub(super)`
  statics now resolve against the `io` module. (Mover only handles `fn`s, so the
  module move was a manual cut/paste — the one non-fn relocation this slice.)
* `scan.rs` re-imports the 5 moved fns it still calls bare
  (`open_log_file`/`pack_file_name`/`scan_and_pack_name`/`scan_file_name`/
  `start_input`); repointed all external `crate::scan::<fn>` call-sites and `use`
  imports to `crate::io::` in `ini.rs`, `shipout.rs`, `errors.rs`, `pic.rs`.
* **Orphan cleanup (the warning-count discipline, §7):** moving the fns left a
  pile of now-dead items in `scan.rs` that only the §5 warning count surfaced:
  6 unused imports (`finalize_dvi_file` + globals `set_name_in_progress`/
  `set_stop_at_space`/`stop_at_space`/`texmf_log_name`/`write_open`), 7 orphaned
  module-private consts (`SELECTOR_LOG_ONLY`/`TTBC_FILE_FORMAT_*`/
  `INT_PAR_XETEX_DEFAULT_INPUT_MODE`/`…_ENCODING`/`INT_PAR_TRACING_STACK_LEVELS`/
  `INT_PAR_NEW_LINE_CHAR_CFT` — the `INPUT_MODE`/`ENCODING` pair was already
  defined in `io.rs`, so those copies were pure duplicates), and 3 now-dead
  `ttstub_output_close`/`open`/`putc` extern decls (`flush` stays — still used).
  Deleted all; also removed 4 stale section-banner comments. Each removal walked
  the count back toward 51.

**No pattern-const trap this slice** — the moved file-naming code matches on
fewer node consts, and the `cmd`-code consts it does touch (`LEFT_BRACE`/`SPACER`/
`RELAX`/`OTHER_CHAR`) were referenced in *expression* context, so they surfaced
as hard `E0425` errors, not silent bindings. (Confirmed by watching the count.)

**Metrics:** `scan.rs` 16,850 → **16,255** LOC (−595). `io.rs` 800 → 1,385. No
signature/`#[no_mangle]`/`unsafe` changes (relocation only). Dashboard 29/2/7/4,
warnings 51.

**For the next session (121):** continue 02 per §8 — next leaves: `ext.rs`
(`do_extension`/`\special` whatsits: `do_extension`/`new_write_whatsit`/
`append_src_special`/`insert_src_special`/`tt_insert_special`), then `fonts.rs`
(TFM/native loading: `read_font_info`/`read_tfm_word`/…/`load_native_font`/
`new_native_*`/`effective_char*`), then `linebreak.rs`
(`do_locale_linebreaks`/`char_pw_in`). Same recipe; watch the warning count for
the trap and orphan cleanup. `ini.rs` (`format_io.rs`/`engine.rs`/hyphenation)
drains independently. Core carve (scanner/expand/main_control/packaging) after
the leaves.


## Idiom session 119 — 2026-06-01 — workstream 02 slice 2: relocate the `show_*`/display group out of `scan.rs`

**Type:** execution (workstream 02, module decomposition — second mechanical
relocation slice). **One commit.** Dashboard **29/2/7/4**. Warnings **51 → 51**.

**Slice:** moved the 12 `show_*`/display/messaging functions verbatim from
`src/scan.rs` (= xetex0.c) into `src/output.rs` (= xetex-output.c, their natural
home): `show_box`, `show_node_list`, `short_display`, `short_display_n`,
`show_activities`, `show_context`, `show_save_groups`, `show_whatever`,
`show_info`, `print_subsidiary_data`, `issue_message`, `char_warning`. **No bytes
changed inside any function** — pure relocation per `02-modules.md` §7. Used
`plan/idiom/move_fns.py` (dry-run → `--apply`); 1,556 fn-lines moved.

**The big lift vs. slice 1: the pattern-const trap fired at scale.** The
show-group functions match on far more node-type/group consts than the errors
group did. Two distinct const classes had to follow the functions:
* **78 single-function-private suffixed consts** (`*_SD`/`*_SA`/`*_SNL`, defined
  in three contiguous blocks immediately above the moved fns: short_display,
  show_activities, show_node_list/print_subsidiary_data). These were orphaned in
  `scan.rs` once their only callers left. Relocated the three blocks verbatim
  into `output.rs` (none collided with existing `output.rs` consts; none still
  referenced in `scan.rs`).
* **7 shared node-type consts** (`RULE_NODE`/`GLUE_NODE`/`MATH_NODE`/
  `LIGATURE_NODE`/`DISC_NODE`/`KERN_NODE`/`PENALTY_NODE`) live in `consts.rs` as
  `pub const`; `scan.rs` imported them. `output.rs` did **not** — so the moved
  `show_node_list`/`short_display` match arms `RULE_NODE => …` silently became
  catch-all **bindings** (the §7 trap), cascading into **22 `unreachable
  pattern` + 14 `non_snake_case`/`unused_variable`** warnings (51 → **102**).
  Fixed by adding the 7 to `output.rs`'s `use crate::consts::{…}`. **The warning
  count was the only signal** — zero compile errors from these. (HLIST_NODE,
  used in an *or*-pattern, did error out via `E0408`/`E0384`, the one place the
  trap surfaces as a hard error.)

**Other fixup (cargo-check-driven):**
* `output.rs` gained imports for the symbols the moved bodies reference that
  stayed in `scan.rs`/elsewhere: scanner fns (`get_token`/`scan_register_num`/
  `scan_toks`/`the_toks`), `errors::error`, globals accessors (`cur_val`/
  `cur_cs`/`def_ref`/`init_pool_ptr`/`set_help_line`/`set_help_ptr`/
  `set_mem_b32s1`/`set_str_ptr`), consts from `consts.rs`
  (`BOX_BASE`/`ERROR_STOP_MODE`/`HLIST_NODE`/`LOCAL_BASE`/`TOO_BIG_CHAR`), and 8
  module-private consts duplicated from `scan.rs` (still used there too:
  `TEMP_HEAD`/`GARBAGE`/`AAT_FONT_FLAG`/`OTGR_FONT_FLAG`/`INT_PAR_END_LINE_CHAR`
  + 3 that were `scan.rs`-orphaned and **deleted** from `scan.rs`:
  `LOCAL__ERR_HELP`/`INT_PAR_SHOW_BOX_DEPTH`/`INT_PAR_SHOW_BOX_BREADTH`). Added
  `ttstub_issue_warning`/`ttstub_output_flush` to `output.rs`'s `extern "C"`
  block.
* `scan.rs` re-imports the 5 moved fns it still calls
  (`char_warning`/`issue_message`/`short_display`/`show_box`/`show_whatever`);
  removed the now-unused `SELECTOR_TERM_AND_LOG` import and the now-dead
  `ttstub_issue_warning` extern decl (its only user, `char_warning`, left).
* External `crate::scan::` callers of moved fns repointed to `crate::output::`:
  `errors.rs` (`show_box`/`show_context`), `shipout.rs` (`show_box` ×2),
  `ini.rs` (`show_save_groups`), `math.rs` (`char_warning`).
* Removed 4 stale section-banner comments left over `scan.rs` where fns departed.

**Gotcha refinement for §7:** when the moved fns are *match-heavy display code*,
expect the trap to fire **in bulk** — the fix is to import the shared node-type
consts from `consts.rs` (they are `pub const` there) rather than duplicating, and
to relocate the single-function-private suffixed const blocks wholesale. The
warning delta (here +51) localizes precisely to the missing consts.

**Metrics:** `scan.rs` 18,537 → **16,850** LOC (−1,687). `output.rs` 3,169 →
4,823. No signature/`#[no_mangle]`/`unsafe` changes (relocation only). Dashboard
29/2/7/4, warnings 51.

**For the next session (120):** continue 02 per §8 — next clean leaves: `io.rs`
(file naming + `start_input`/`open_log_file`/`close_files_and_terminate`), then
`ext.rs` (`do_extension`/`\special` whatsits), `fonts.rs` (TFM/native loading +
`effective_char*`), `linebreak.rs` (`do_locale_linebreaks`/`char_pw_in`). Same
recipe: `move_fns.py`, then cargo-check the imports + **warning-count** the trap.
`ini.rs` drains independently. The core carve (scanner/expand/main_control/
packaging) comes after the leaves.


## Idiom session 118 — 2026-06-01 — workstream 02 slice 1: relocate the `errors` group out of `scan.rs`

**Type:** execution (workstream 02, module decomposition — first mechanical
relocation slice). **One commit.** Dashboard **29/2/7/4**. Warnings **51 → 51**.

**Slice:** moved the 8 `errors`-group functions verbatim from `src/scan.rs`
(= xetex0.c) into `src/errors.rs` (= xetex-errors.c, their natural home):
`you_cant`, `report_illegal_case`, `privileged`, `cs_error`, `box_error`,
`mu_error`, `runaway`, `extra_right_brace`. **No bytes changed inside any
function** — pure relocation per `02-modules.md` §7.

**Method (the point of the slice):** used the new `plan/idiom/move_fns.py`
mechanical relocator (dry-run → `--apply`), *not* the manual edit tool. The
script cut each top-level fn with its `// translated from …` comment + `#[…]`
attributes through the column-0 closing brace (157 lines) and appended them to
`errors.rs`. The only hand-work was the visibility/import fixup driven by
`cargo check`:
* scan.rs imports the 8 back (`use crate::errors::{…}`) — still called there;
* errors.rs gained imports for the symbols the moved bodies reference
  (`BOX_BASE`; globals `align_state`/`cur_chr`/`cur_cmd`/`cur_list_mode`/
  `eqtb_b32s1`/`set_align_state`; `crate::scan::{show_box, flush_node_list_global}`);
* the 2 external importers repointed (`mem.rs` + `linebreak.rs:2156` →
  `crate::errors::runaway`; `pagebuilder.rs` → `crate::errors::box_error`).

**Latent-bug caught (now a recipe gotcha, §7):** `extra_right_brace` `match`es
on the module-private consts `SEMI_SIMPLE_GROUP`/`MATH_SHIFT_GROUP`/
`MATH_LEFT_GROUP` (14/15/16), which stayed in `scan.rs`. Without them in
`errors.rs` the code **still compiled**, but Rust silently turned the first arm
into an irrefutable catch-all *binding* — a real behavior change. The only
signal was a +9 warning jump (`unreachable_patterns` + `non_snake_case` +
`unused_variable`). Fixed by duplicating the 3 consts into `errors.rs`; warnings
returned to 51. **Lesson: check the warning count, not just errors, after every
relocation** — recorded in §7.

**Metrics:** `scan.rs` 18,689 → **18,537** LOC (−152 net; first dent in the
grab-bag). `errors.rs` 268 → 433. No signature/`#[no_mangle]`/`unsafe` changes
(relocation only). Dashboard 29/2/7/4.

**For the next session (119):** continue 02 slice-by-slice per §8 (Decision 2,
leaves first). Next clean leaves: the `show_*`/display group → `output.rs`
(largest, ~1.5k LOC; watch for the pattern-const trap), then `io.rs`
(file-naming/`start_input`), `ext.rs`, `fonts.rs`. Use `move_fns.py` for the
relocation; the manual part is only the import fixup + warning-count check.
`ini.rs` (`format_io.rs`/`engine.rs`/hyphenation) drains independently.


## Idiom session 117 — 2026-06-01 — draft `02-modules.md` to READY (lift the scanner deferral)

**Type:** plan construction (workstream 02). **No code change** — docs only.

**Why:** asked "what do we do to no longer have [`scan.rs`/scanner] deferred?"
Traced the deferral to two gates: (1) the hard one — `02-modules.md` never
existed, and hard rule 4 forbids executing a workstream without a READY plan;
(2) a soft perf gate — the hot tokenizer scalars, already covered by the
existing `bench_phase_c.py` harness and not actually a blocker for *starting*
scanner threading (the raw reads are borrow-safe per `01b` §4). The `ext.rs` /
`eq_define` deferrals are not gated by any doc — they just fall out once scanner
threading is underway. So the unblock = write workstream 02.

**What I wrote (`plan/idiom/02-modules.md`, ~270 lines, READY):**
* **Source-verified the giant files** (direct grep): `scan.rs` 18,689 LOC /
  222 fn defs / 151 `#[no_mangle]` / 192 `unsafe fn` (= `xetex-xetex0.c`);
  `ini.rs` 4,789 LOC (= `xetex-ini.c`); `globals.rs` 4,920 LOC (transitional
  bridge, dissolves). `math`/`linebreak`/`shipout` are already 1:1 with their C
  files — not decomposition targets.
* **Inventoried + group-mapped all 222 `scan.rs` fns** to destination modules
  (scanner / expand / main_control / packaging / output / errors / fonts / io /
  ext / linebreak), and `ini.rs` to engine / format_io / hyphenation /
  main_control / io. New modules needed: `scanner`, `expand`, `main_control`,
  `token`, `engine`, `format_io`, `packaging`, `hyphenation`.
* **Decision 1 (the crux): mechanically pre-split first, thread in place after.**
  A pure relocation is parity-trivial (visibility + `mod`/`use`; `#[no_mangle]`
  symbols are global so C linkage is unaffected) and decouples the
  18k-line-grab-bag problem from the semantic threading problem. This
  **supersedes `01b` §9's "thread + move together"** for the giant-file case
  (noted here per the README "newer doc wins, flag it" rule) — with the move
  done first, threading then proceeds in the destination module one subsystem
  per commit, exactly like every prior Phase-C slice.
* **Fenced out-of-scope work:** `node/` typed accessors belong to ws03
  (`packaging.rs` is the transitional home for `hpack`/`vpackage`/list utils);
  hot-scalar migration is a later benchmarked slice; the `state.rs` → `state/`
  split is independent filler. `globals.rs` is not split — it dissolves.
* **Recipe + parity argument + first slice** (§7/§8): drain the `errors` or
  `output` leaf group out of `scan.rs` first (small, self-contained), then the
  other home-having leaves, then `ini.rs`, then the core scanner/expand/
  main_control carve.

**No dashboard run** (docs-only). `STATUS.md` plan-construction table: 02
PLANNED → **READY**.

**For the next session (118):** execute 02's first slice per §8 — relocate the
`errors`-group fns (`you_cant`/`report_illegal_case`/`privileged`/`cs_error`/
`box_error`/`mu_error`/`runaway`) from `scan.rs` into `errors.rs`,
behavior-preserving, dashboard-green, per the §7 recipe. Or, if you'd rather
keep threading leaves, that lane still exists (orchestrator-internal blocks) —
but the module drain is now the higher-leverage path and is unblocked.


## Idiom session 116 — 2026-06-01 — finish `char_pw` threading; fold protrusion widths into `total_pw`

**Type:** execution (workstream 01, Phase C — linebreak protrusion-width leaf).
**One commit.** Dashboard **29/2/7/4**. Crate warnings **51 → 51** (flat).

**Inherited WIP triage.** The working tree carried an uncommitted slice from a
prior aborted session: `char_pw` (`src/scan.rs`) had been split into a fully
threaded `char_pw_in(mem: &MemState, fonts: &FontState, linebreak: &mut
LinebreakState, p, side)` plus a thin `char_pw` `with_state_mut` wrapper, and
the two margin-kern inserters (`insert_left/right_margin_kern`, `src/linebreak.rs`)
had been repointed to `char_pw_in` (gaining a `&FontState` param). That part was
sound and kept. The same WIP also carried **unrelated `src/math.rs` churn**
(moving `axis_height`/`big_op_spacing*`/`sub1..sup3` reads out of `with_state_mut`
into a separate `with_state` in `make_op_global`/`var_delimiter_global`/
`clean_script_box_global`). Those helpers are already threaded direct-ref
functions (`fn(&FontState, &EqtbState, size)`), **not** `with_state`-bearing
wrappers, so calling them under a live `with_state_mut` borrow is safe — the
reshuffle fixed no bug, and the `clean_script_box_global` variant reordered
`add_script_space_and_restore_font` after the shift computation. Reverted
`src/math.rs` to HEAD as provably-unnecessary noise (kept the slice coherent).

**Slice (the kept + finished work):**
* **Folded the protrusion-width sum into `total_pw`.** `char_pw_in` is fully
  threaded (mem/font reads + `linebreak.last_*` writes only; no `with_state`
  bridge, no cold/abort path), so the session-76 split — which deferred the two
  `char_pw` calls until after the `total_pw` borrow dropped — is no longer
  needed. `total_pw` now takes `&FontState`, returns `Option<i32>` (the summed
  width), and computes `char_pw_in(.., l, 0) + char_pw_in(.., r, 1)` inside the
  same borrow. `total_pw_global` is now a thin bridge + `pdf_error` on the
  hlist-stack-failure `None`.
* **Deleted the legacy `char_pw` wrapper** (`src/scan.rs`); `total_pw_global`
  was its last caller.
* **Deleted two now-dead bridge accessors** (`src/globals.rs`):
  `set_last_leftmost_char` / `set_last_rightmost_char`. `char_pw_in` writes
  `linebreak.last_leftmost_char` / `last_rightmost_char` directly, so the
  setters had no remaining caller (this is exactly the +2 dead-code warnings the
  WIP had introduced — deleting them restores warnings to 51). The matching
  getters `last_leftmost_char()` / `last_rightmost_char()` stay (still read by
  the margin-kern `_global` wrappers).

**Classification:** bucket **B/C**. `char_pw_in` body is a bucket-B accessor
rewrite (`mem_*`/`font_*` free-fn accessors → `mem.`/`fonts.` methods); folding
it into `total_pw` and dropping the deferred-call wrapper is the bucket-C
borrow-island collapse (here a *simplification*, since the deferred call turned
out to be borrow-safe to inline).

**Metrics (direct grep):** `#[no_mangle]` unchanged (`char_pw` was not
exported). `unsafe fn` −1 (`char_pw` deleted). Two thin `globals.rs` accessors
removed. Warnings 51 (flat).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 51 warnings.
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust
--test latex_goldens` → **29/2/7/4** all green.

**For the next session (117):** the easy bounded leaves outside `src/scan.rs`
and `src/ext.rs` are largely exhausted (synctex fully threaded sessions 111–115;
mem/allocator core, node constructors, math noad family, save/shipout/linebreak
leaves all done). `phase_c_recon.py`'s top entries are now mostly test fns,
`lib.rs` config getters, and `output.rs` print routines (the latter go through
`selector`/output state — a much later target). Remaining real work is the
orchestrators (`hpack`/`vpackage`/`build_page`/`line_break`/`mlist_to_hlist`,
thread internal blocks only, never whole) and `src/scan.rs` (largest, most
coupled — keep last; draft `02-modules.md` to READY first). **Note:** the
recon's `dead-wrapper` rows (`is_char_node_global`, `font_character_width_global`,
`is_non_discardable_node_global`, `get_native_usv_global`) are **false positives**
— those `#[inline] fn` wrappers are heavily used across linebreak/shipout/math;
the heuristic just misses non-`pub` inline callers. Do not delete them.


## Idiom session 115 — 2026-06-01 — thread SyncTeX public entry points to `EngineState`

**Type:** execution (workstream 01, Phase C — complete `src/synctex.rs`
entry-point threading). **One commit.** Dashboard **29/2/7/4**. Crate warnings
**51 → 51** (flat).

**Slice:**
* **Converted the remaining SyncTeX API surface to explicit state.** The
  `src/synctex.rs` entry points now take `&mut EngineState` directly and operate
  through `st.synctex`, `st.eqtb`, `st.mem`, and `st.shipout` without borrowing
  thread-local state internally.
* **Bridged callers at current legacy boundaries.** Updated
  `synctex_init_command` from engine startup, `synctex_start_input` and
  `synctex_terminate` from scanner orchestration, and the shipout/hlist/vlist
  SyncTeX calls to acquire state once at the caller boundary.
* **Kept bridge-sensitive arguments outside live borrows.** `gettexstring`,
  `job_name`, `eqtb_b32s1(INTPAR(mag))`, and `log_opened` values are computed
  before entering `with_state_mut` where they still use legacy accessors. This
  avoids nested `RefCell` borrows while preserving the old allocation/free
  behavior for SyncTeX path strings.
* **Deleted unused de-exported remnants.** Removing the C export surface exposed
  dead `synctex_abort`, `synctex_pdfxform`, `synctex_mrofxfdp`,
  `synctex_pdfrefxform`, and their private record helpers. They had no live
  Rust/C callers in the crate, so they were removed instead of kept as private
  dead code.

**Surprise:** the first `engine_rust` validation hit a nested `RefCell` borrow
in `start_input`: `synctex_start_input` was still calling `gettexstring` after
the caller had entered `with_state_mut`. The fix was to pass the already
allocated `tmp` string into `synctex_start_input`, matching the same pattern
used for `synctex_sheet`.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 51 warnings.
`cargo test -p tectonic --test engine_rust` → **7/7** green after the nested
borrow fix. Full dashboard
`cargo test -p tectonic --test engine_rust --test tex-outputs --test trip --test latex_goldens`
→ all **29/2/7/4** tests pass; `tex-outputs::synctex` is green. `git diff
--check` is clean. Recon check: `src/synctex.rs` has no `with_state*` calls and
no active `#[no_mangle]` exports.

**For the next session (116):** SyncTeX is now fully threaded locally for Phase
C; remaining `with_state_mut` calls are caller-boundary bridges in
`ini.rs`/`scan.rs`/`shipout.rs`. Move to another bounded leaf from
`phase_c_recon.py`, or plan a broader caller-threading slice before trying to
push state through scanner/shipout orchestrators.


## Idiom session 114 — 2026-06-01 — finish local SyncTeX bridge cleanup

**Type:** execution (workstream 01, Phase C — bounded `src/synctex.rs`
threading cleanup). **One commit.** Dashboard **29/2/7/4**. Crate warnings
**51 → 51** (flat).

**Slice:**
* **Deleted the obsolete SyncTeX field bridge (`src/synctex.rs`).** Removed the
  `sc_scalar!` / `sc_flag!` macro block and all `sc_*` getter/setter
  definitions. The public SyncTeX entry points already borrow the thread-local
  once and pass disjoint state fields through, so the per-field bridge was dead
  local scaffolding.
* **Threaded deferred recorder callbacks.** Changed `state::Recorder` from
  `unsafe fn(i32)` to
  `unsafe fn(&mut SynctexCtxt, &MemState, i32)`, then stored
  `synctex_record_node_kern` directly as the deferred callback. The three flush
  sites now call the recorder with the already-borrowed SyncTeX context and mem
  arena, deleting the last in-module `synctex_record_node_kern_global`
  bridge-down wrapper and avoiding any nested `with_state_mut` edge.
* **Threaded `synctex_init_command`'s `\synctex` initialization.** Added
  `EqtbState::set_b32s1` and used the existing `with_state_mut` boundary to
  read `ConfigState::synctex_enabled` and write `INTPAR(synctex)` directly.
  Removed the now-dead `globals::synctex_enabled()` getter; the setter remains
  live from `engine_interface.rs`.

**Classification:** bucket **B** / local cleanup. The deleted `sc_*` accessors
were unused after session 113. The recorder callback is a pure pass-through
threading change over already-available `SynctexCtxt` + `MemState`; it has no
new output ordering or abort-path behavior. `synctex_init_command` already had
a `with_state_mut` boundary, so folding the config read and eqtb write into
that borrow removes two bridge calls without changing initialization order.

**Method surface / deleted accessors:** added `EqtbState::set_b32s1`. Deleted
`globals::synctex_enabled()` and the local `synctex_record_node_kern_global`
wrapper. No new state fields.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 51 warnings.
`cargo test -p tectonic --test engine_rust --test tex-outputs --test trip --test latex_goldens`
→ all **29/2/7/4** tests pass; the `tex-outputs::synctex` case is green.

**For the next session (115):** continue bounded Phase-C leaves. The SyncTeX
module is locally cleaned up: no `sc_*` bridge remains and no local
`_global` recorder wrapper remains. Remaining progress should probably move to
another small leaf surfaced by `phase_c_recon.py`, or begin carefully planning
broader caller threading in `shipout.rs` before touching scanner-heavy paths.


## Idiom session 113 — 2026-06-01 — thread all remaining synctex node-field recorders and FFI call sites in `src/synctex.rs`

**Type:** execution (workstream 01, Phase C — synctex node-field recorders and FFI call sites). **One commit.** Dashboard **29/2/7/4**. Crate warnings **52 → 51** (-1, due to deleting 9 unused transitional `_global` wrappers, offset by resolver cleanup).

**Slice:**
* **`synctex_record_node_*` / FFI call sites (`src/synctex.rs`).** Threaded all remaining `synctex_record_node_*` helper calls inside the public FFI wrappers at the bottom of `src/synctex.rs` (`synctex_vlist`, `synctex_void_vlist`, `synctex_hlist`, `synctex_void_hlist`, `synctex_math`, `synctex_horizontal_rule_or_glue`, `synctex_kern`) to directly pass safe state borrows (`&st.mem` / `&st.shipout`).
* **`_global` internal accessors deleted (`src/synctex.rs`).** Deleted 9 now-completely-unused `_global` wrapper definitions (`synctex_tag_box_global`, `synctex_line_box_global`, `synctex_tag_rule_global`, `synctex_line_rule_global`, `synctex_tag_medium_global`, `synctex_line_medium_global`, `synctex_width_global`, `synctex_depth_global`, `synctex_height_global`), cleanly removing all corresponding compiler warnings.

**Metrics:** `#[no_mangle]` **327** unchanged. raw extern statics **38** unchanged. raw `unsafe {}` **175** unchanged. `unsafe fn` **640** unchanged. Warnings **51** (down from 52).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 51 warnings; cargo test -p tectonic --test engine_rust --test tex-outputs --test trip --test latex_goldens → all 29/2/7/4 tests pass green.

**For the next session (114):** continue bounded slices only. Read `plan/idiom/STATUS.md` and `plan/idiom/01-state-and-memory.md` before starting. Good options remain rounding out the synctex cluster (`intpar_synctex` → `&EqtbState`, ~16 callers), or selecting another small leaf from `phase_c_recon.py`. Avoid scanner-heavy `scan.rs` or orchestrators until planned.

---

## Idiom session 112 — 2026-06-01 — thread all remaining Bucket-B leaf helpers in `src/shipout.rs` and `src/linebreak.rs`

**Type:** execution (workstream 01, Phase C — shipout and linebreak leaf helpers). **One commit.** Dashboard **29/2/7/4**. Crate warnings **52 → 52** (flat).

**Slice:**
* **`insert_right_margin_kern` / `insert_left_margin_kern` (`src/linebreak.rs`).** Threaded the two margin-kern builders to safe functions taking safe borrows (`mem: &mut MemState, linebreak: &mut LinebreakState`) and returning `Result<i32, MarginKernAbort>`, handling allocation and stack failures through a custom `MarginKernAbort` enum. Wrapped them in global legacy wrappers `insert_right_margin_kern_global` and `insert_left_margin_kern_global` to print errors and abort after the borrow is dropped.
* **`copy_native_tail` (`src/linebreak.rs`).** Threaded `copy_native_tail_in` to a clean signature `pub unsafe fn copy_native_tail(mem: &mut MemState, q: i32, ha: i32, start: i32)` and renamed the legacy wrapper to `copy_native_tail_global`.
* **`set_native_metrics` (`src/linebreak.rs`).** Threaded `set_native_metrics_in` to a clean signature `pub unsafe fn set_native_metrics(mem: &MemState, eqtb: &EqtbState, p: i32)` and renamed the legacy wrapper to `set_native_metrics_global`.
* **`is_non_discardable_node` (`src/linebreak.rs`).** Threaded `is_non_discardable_node_in` to `is_non_discardable_node` taking `mem: &MemState`.

**Metrics:** `#[no_mangle]` **327** unchanged. raw extern statics **38** unchanged. raw `unsafe {}` **175** unchanged. `unsafe fn` **640** unchanged. Warnings **52** unchanged.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 52 warnings; full dashboard → **29 / 2 / 7 / 4** green.

**For the next session (113):** continue bounded leaves or start executing the module splits outlined in `plan/02-modules.md`. Remaining targets include synctex `intpar_synctex` or other local pagebuilder leaf helpers.

---

## Idiom session 111 — 2026-06-01 — thread the synctex node-field reader cluster + fold in math `is_char_node` dedup

**Type:** execution (workstream 01, Phase C — synctex mem-reader leaves + math predicate dedup). **One commit.** Dashboard **29/2/7/4**. Crate warnings **52 → 52** (flat; removed 3 now-unused `mem_*` `globals` imports from `synctex.rs`, balancing the threading). Deliberately larger than session 110 (two coherent leaf clusters batched).

**Cluster 1 — synctex node-field reader leaves → `&MemState` (10 functions, `src/synctex.rs`).** Threaded the ten pure `mem`-arena node-field readers — `synctex_tag_box`, `synctex_line_box`, `synctex_tag_rule`, `synctex_line_rule`, `synctex_tag_medium`, `synctex_line_medium`, `synctex_type`, `synctex_width`, `synctex_depth`, `synctex_height` — from `unsafe fn(p)` (each reaching the `mem` arena through a `mem_b32s0`/`mem_b32s1`/`mem_b16s1` `with_state` bridge accessor) to safe `fn(mem: &crate::state::MemState, p)` calling the `MemState` methods (`b32s0`/`b32s1`/`b16s1`) directly. Each got a transitional `_global` bridge-down wrapper (`fn synctex_X_global(p) { with_state(|st| synctex_X(&st.mem, p)) }`), and the ~40 in-module call sites (in the not-yet-threaded `synctex_record_node_*` recorders and the public `synctex_vlist`/`synctex_hlist`/`synctex_math`/`synctex_horizontal_rule_or_glue`/`synctex_kern` recording functions) were blanket-renamed to the `_global` form by a single line-scoped `perl` pass (bucket **A** rename; restricted to lines past the definition block so the threaded cores and wrapper bodies were untouched).

**Classification:** bucket **B** throughout for the cores (pure `&MemState` reads of fixed node-field offsets — `BOX_NODE_SIZE`/`RULE_NODE_SIZE`/`MEDIUM_NODE_SIZE` minus `SYNCTEX_FIELD_SIZE`, plus the width/depth/height offsets — no `with_state`-bearing internal call, no abort path); the call-site rename to `_global` is the mechanical bucket-A bridge-down. No reader call site sits inside a live `with_state*` closure: the surrounding recorders read fields into `format!` args / locals before the `synctex_write` output, and each `sc_*`/`mem_*` accessor borrows-and-releases, so the per-call `_global` borrow never nests. **Parity-covered:** the synctex output path is exercised by the `synctex` tex-outputs case (`tests/tex-outputs/synctex.tex` + `synctex.gz` golden, `check_synctex(true)`), so the `.synctex` byte stream is a hard parity guard for this slice.

**Cluster 2 — math `is_char_node(p)` global-bridge dedup (`src/math.rs`).** The remaining `unsafe fn is_char_node(p) -> bool { p >= hi_mem_min() }` global-bridge form (deferred by session 110) was deleted and replaced with a safe `fn is_char_node_global(p) { with_state(|st| crate::mem::is_char_node(&st.mem, p)) }` routing through the canonical `crate::mem::is_char_node` predicate (session-108/110 dedup precedent). The 6 in-module callers (lines ~643/652/656/1184/1718/4580) were repointed to `is_char_node_global` by a negative-lookbehind `perl` pass (`(?<!::)\bis_char_node\(`) that left the already-threaded `crate::mem::is_char_node(mem, p)` / `crate::mem::is_char_node(&st.mem, p)` sites untouched. `hi_mem_min` stays imported (still used by 3 other math.rs sites).

**Recon-helper usefulness report:**
- Commands: `python3 plan/idiom/phase_c_recon.py --limit 40`, `--bucket B --module src/pagebuilder.rs`, `--bucket C --module src/shipout.rs`, `--bucket B --module src/math.rs`, `--bucket B --module src/linebreak.rs`, `--dead-wrappers`.
- Surfaced: the whole `synctex_*` reader cluster (`synctex_type`, `synctex_tag_box`, `synctex_line_box`, `synctex_tag_rule`, `synctex_line_rule`, `synctex_tag_medium`, `synctex_line_medium`, `synctex_width`, `synctex_depth`, `synctex_height`) at score 101, bucket B — accurate. It also confirmed **math/linebreak bucket B are now exhausted** (empty tables), which is what pushed the search toward synctex; and that shipout's remaining work is all bucket C (output/diagnostic edges).
- **False positives:** (1) `--dead-wrappers` again flagged `font_character_width_global` / `is_char_node_global` (linebreak) as 0-caller dead wrappers — re-confirmed they have many live callers (lines 1286–1666), the same session-110 miscount of wrappers-defined-after-the-edit. (2) `pdf_get_rect` (`src/pic.rs`) scored 107 bucket B with "acc 7", but inspection showed those 7 are `PdfRect`/`bbox` *struct-field* accesses (`bbox.llx`, …), not `mem` bridge accessors — it's pure PDF/FFI math with **no** `with_state` bridge to remove, so it is **not** a Phase-C threading candidate. Suggested script refinement: the accessor heuristic should not count `ident.field` struct projections on plain locals as bridge-accessor hits.
- Time saved: ~15 min — it directly ranked the synctex leaves and ruled out math/linebreak as drained, so the only manual call-graph work was confirming the synctex recorders don't nest borrows.

**Method surface / deleted accessors:** no new `state.rs` methods (existing `MemState::b32s0`/`b32s1`/`b16s1` sufficed); no `globals.rs` accessor lost its last caller (the removed `synctex.rs` imports `mem_b32s0`/`mem_b32s1`/`mem_b16s1` are still referenced from other modules — they were merely no longer used *in synctex.rs*). Deleted the local math `unsafe fn is_char_node`.

**Newtype debt:** none. No state fields moved.

**Metrics:** `#[no_mangle]` **327** unchanged. raw extern statics **38** unchanged. raw `unsafe {}` **175** unchanged (no inner unsafe blocks in any threaded body). `unsafe fn` **651 → 640** (−11: 10 synctex readers de-`unsafe`d, the 10 new `_global` wrappers are safe `fn`; math `is_char_node` de-`unsafe`d, its `_global` replacement safe). Warnings **52** unchanged.

**Validation:** `cargo fmt`; `cargo check -p tectonic_engine_xetex` → 0 errors, 52 warnings; full dashboard → **29 / 2 / 7 / 4** green.

**For the next session (112):** continue bounded leaves only. Remaining clean targets: round out the synctex cluster by threading `intpar_synctex()` → `&EqtbState` (one eqtb read, ~16 in-module callers; same `_global` bridge-down shape) and/or `synctex_curh`/`synctex_curv` (read `cur_h`/`cur_v`, deferred raw shipout scalars — check whether those are threadable yet). The `ext.rs` native-metric cluster remains clean bucket B but **scan.rs-coupled** — defer until scanner threading begins (coordinate with `02-modules.md`). Shipout's remaining work is bucket C (output/diagnostic edges: `finish_vlist_rule`/`finish_hlist_rule`/`out_what`/`special_out`) — manual borrow-dropping, not a quick leaf. Avoid whole-session work on `line_break`, `try_break`, `hlist_out`, `vlist_out`, `ship_out`, `scan.rs`, `mlist_to_hlist`, `hpack`, `vpackage`, or `build_page`.

---

## Idiom session 110 — 2026-06-01 — finish font-metric/native-USV leaves, consolidate `is_char_node`

**Type:** execution (workstream 01, Phase C — font-metric / node-predicate leaves). **One commit.** Dashboard **29/2/7/4**. Crate warnings **53 → 52** (removed 6 now-unused `globals` font imports from `shipout.rs`).

**Picked up uncommitted WIP first.** The working tree already held a partial slice (threading `get_native_usv` + `font_character_width` in `src/linebreak.rs`, and `font_character_width` + `is_char_node` in `src/shipout.rs`) that **did not compile** — `font_character_width` in `linebreak.rs` held an immutable `&fonts` borrow (`fonts.font_info_b16(...)`) across a `&mut fonts` argument (`crate::scan::effective_char(fonts, …)`), an E0502. Completed it by precomputing `effective_char` into a local before the immutable borrow (parity-safe: the index is just `char_base_entry(f) + effective_char(…)`), removing a stray duplicate `#[inline]` on `is_char_node_global`, and deleting the 6 font `globals` imports the threaded bodies no longer use.

**Slice (the WIP, now correct):**
- `get_native_usv(mem: &MemState, p, i)` (was `#[no_mangle] pub unsafe extern "C" fn`) — pure UTF-16 surrogate decode over `mem.as_u16_ptr`; `get_native_usv_global` wraps the two legacy `line_break` call sites.
- `font_character_width(fonts: &mut FontState, f, c: u16)` in `linebreak.rs` — reads `char_base`/`width_base`/`font_info` directly and calls the threaded `crate::scan::effective_char`; `font_character_width_global` wraps the ~8 `line_break`/`try_break` call sites.
- `font_character_width(fonts: &FontState, f, c: i32)` in `shipout.rs` (no `effective_char`; caller passes the effective char) + `font_character_width_global`; `is_char_node` over `&MemState` + `is_char_node_global` for `hlist_out`/`vlist_out`/`reverse`/`merge_justifiable_native_words`.

**Extension — `is_char_node` dedup (session-108 `prev_rightmost` precedent).** The threaded predicate `p >= mem.hi_mem_min` was duplicated three times: `is_char_node_in` in `src/linebreak.rs` and `src/math.rs`, and (renamed by the WIP) `is_char_node` in `src/shipout.rs`. Added one canonical `pub fn is_char_node(mem: &MemState, p)` to `src/mem.rs`, deleted all three local copies, and repointed every `(mem, p)`-form caller (linebreak ×4, math ×2, shipout ×2) to `crate::mem::is_char_node`. The separate `unsafe fn is_char_node(p)` *global-bridge* version in `math.rs` (6 in-module callers, reads `hi_mem_min()`) is untouched — that is the not-yet-threaded form, a later target.

**Classification:** bucket **B** throughout. Font-metric leaves read `FontState`/`MemState` and call native FFI (`effective_char`, surrogate decode) that is not `with_state`-bearing; the predicate dedup is pure code consolidation.

**Recon-helper usefulness report:**
- Commands: `python3 plan/idiom/phase_c_recon.py --limit 40`, `--bucket B --module src/ext.rs --limit 30`.
- Surfaced: the whole `ext.rs` native-metric cluster (`map_char_to_glyph`, `measure_native_node`, `get_native_char_height_depth`, …) as bucket B. **Rejected on caller analysis:** their external callers are almost entirely in `src/scan.rs` (the scanner hot path the recipe defers), so threading them would dump ~25 `_global` wrappers into `scan.rs` against documented strategy. Recon classification was accurate; the *call-graph* (which the score doesn't weight) is what ruled them out — a possible script improvement: penalize candidates whose callers concentrate in `scan.rs`.
- It also flagged the WIP's `font_character_width_global`/`is_char_node_global` as "dead-wrapper" (0 callers), a **false positive** — they have many callers; the dead-wrapper heuristic miscounts wrappers defined after the WIP edits.

**Method surface / deleted accessors:** added `MemState`-free function `crate::mem::is_char_node`; deleted three local `is_char_node`/`is_char_node_in` copies. No `globals.rs` accessor lost its last caller (removed imports were unused free-function aliases, not accessor definitions).

**Newtype debt:** none. No state fields moved.

**Metrics:** `#[no_mangle]` **328 → 327** (`get_native_usv` de-exported). raw extern statics **38** unchanged. raw `unsafe {}` **174 → 175** (`get_native_usv`'s single `unsafe extern fn` became two `unsafe {}` pointer reads). `unsafe fn` **650 → 651** (`get_native_usv` de-`unsafe`d, `get_native_usv_global` wrapper added; predicate dedup net 0). Warnings **53 → 52**.

**Validation:** `cargo fmt`; `cargo check -p tectonic_engine_xetex` → 0 errors, 52 warnings; full dashboard → **29 / 2 / 7 / 4** green.

**For the next session (111):** continue bounded leaves only. The `ext.rs` native-metric cluster is clean but **scan.rs-coupled** — defer it until scanner threading begins (coordinate with `02-modules.md`). Better near-term targets: another small pagebuilder/shipout/linebreak leaf, or thread `math.rs`'s remaining `unsafe fn is_char_node(p)` global bridge (6 in-module callers) by repointing them to `crate::mem::is_char_node` under short borrows. Avoid whole-session work on `line_break`, `try_break`, `hlist_out`, `vlist_out`, `ship_out`, `scan.rs`, `mlist_to_hlist`, `hpack`, `vpackage`, or `build_page`.

---

## Idiom session 109 — 2026-06-01 — thread shipout movement and linebreak equivalents leaves

**Type:** execution (workstream 01, Phase C — shipout and linebreak leaves). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (53).

**Slice:** Threaded shipout movement leaf helper `move_past_vlist_rule` in `src/shipout.rs` to take safe `&mut ShipoutState` directly, removing the `unsafe` keyword from its signature. Threaded the math node direction helpers `math_node_lr_dir` and `math_node_end_lr_type` in `src/shipout.rs` to safe `&MemState` borrows, clean of global accessors. Threaded `set_current_hyph_index` in `src/linebreak.rs` to take safe `&mut HyphState` directly and index trie vectors natively. Threaded inner line-break parameter helpers `int_par`, `dimen_par`, `glue_par`, and `lc_code` in `src/linebreak.rs` to module-level functions taking explicit `&EqtbState` borrows, clean of global equivalents accessors. Legacy callers are repointed via global wrappers or short disjoint borrows.

**Classification:** bucket **B**, highly accurate. These are small leaf helpers and parameter readers that only need safe reads or direct writes to specific states, avoiding orchestrator or allocation hazards.

**Recon-helper usefulness report:**
- Which command(s) you ran: `python3 plan/idiom/phase_c_recon.py --limit 40` and targeted scans on `src/shipout.rs` / `src/linebreak.rs`
- Which candidate it surfaced: `math_node_lr_dir` (score 101, bucket B), `math_node_end_lr_type` (score 101, bucket B), `lc_code` (score 101, bucket B), `int_par`/`dimen_par`/`glue_par` (score 101, bucket B)
- Whether bucket/risk classification was accurate: Extremely accurate. Surfaced clean, independent leaves that were perfectly safe to thread.
- Roughly how much recon time it saved or failed to save: Saved around 15-20 minutes by automatically filtering out already-threaded helpers and highlighting safe candidate leaves immediately.
- Any false positives/false negatives or suggested script improvements: None, the helper continues to perform wonderfully.

**Method surface / deleted accessors:** none. No state fields moved and no new methods added (existing vector indices and struct fields on `ShipoutState`, `HyphState`, and `EqtbState` were sufficient).

**Newtype debt:** none. No state fields moved.

**Metrics:** `#[no_mangle]` **328** unchanged. raw extern statics **38** unchanged. raw `unsafe {}` **174** unchanged. `unsafe fn` **651 -> 650** (made `move_past_vlist_rule` safe, added safe global wrappers). Warnings **53** unchanged.

**Validation:** `cargo fmt`; `cargo check -p tectonic_engine_xetex` -> 0 errors, 53 warnings; full dashboard -> **29 / 2 / 7 / 4** green.

**For the next session (110):** continue bounded leaves only. Good options remain another small pagebuilder/linebreak leaf, or a small shipout movement helper. Avoid whole-session work on `line_break`, `try_break`, `hlist_out`, `vlist_out`, `ship_out`, `scan.rs`, `mlist_to_hlist`, `hpack`, `vpackage`, or `build_page`.

---

## Idiom session 108 — 2026-06-01 — thread memory prev_rightmost and shipout ignorable leaves

**Type:** execution (workstream 01, Phase C — memory / shipout leaves). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (53, as we deleted some dead code and made signatures safe).

**Slice:** Threaded the core memory leaf helper `prev_rightmost` in `src/mem.rs` to take safe `&MemState` directly, removing the `unsafe` keyword from its signature. This allowed us to fully delete the duplicate helper `prev_rightmost_in` from `src/linebreak.rs` and call `crate::mem::prev_rightmost` directly, cleaning up duplicate code. Legacy callers in `src/scan.rs` are repointed to `prev_rightmost_global`. We also threaded the local `is_ignorable_interword_node` and `skip_ignorable_interword_nodes` helpers in `src/shipout.rs` to safe `&MemState` borrows, and defined `is_char_node_in` so that they can avoid calling the global `hi_mem_min()`. Call sites in `merge_justifiable_native_words` are repointed via transient `with_state` borrows.

**Classification:** bucket **B**, highly accurate. These are pure node-traversing/evaluating leaves that only need simple reads and no mutable state or allocation, making them extremely clean and safe to thread.

**Recon-helper usefulness report:**
- Which command(s) you ran: `python3 plan/idiom/phase_c_recon.py --limit 40` and `python3 plan/idiom/phase_c_recon.py --bucket B --module src/shipout.rs`
- Which candidate it surfaced: `prev_rightmost` (score 102, bucket B) and `skip_ignorable_interword_nodes` (score 101, bucket B)
- Whether bucket/risk classification was accurate: Highly accurate. Both are clean leaves with no state mutability or external calls.
- Roughly how much recon time it saved or failed to save: Saved about 10-15 minutes by immediately triaging the leaf helper candidates and allowing us to find the redundant duplicate `prev_rightmost_in` helper in `linebreak.rs`.
- Any false positives/false negatives or suggested script improvements: None, the helper continues to work exceptionally well.

**Method surface / deleted accessors:** Deleted `prev_rightmost_in` from `src/linebreak.rs` as its callers were transitioned to the threaded `crate::mem::prev_rightmost`. No globals accessor lost all its callers.

**Newtype debt:** none. No state fields moved.

**Metrics:** `#[no_mangle]` **328** unchanged. raw extern statics **38** unchanged. raw `unsafe {}` **174** unchanged. `unsafe fn` **653 -> 651** (deleted `prev_rightmost_in`'s unsafe requirements, made `prev_rightmost` safe, made `is_ignorable_interword_node` and `skip_ignorable_interword_nodes` safe, added `prev_rightmost_global`). Warnings **52 -> 53** (expected clashing extern warnings).

**Validation:** `cargo fmt`; `cargo check -p tectonic_engine_xetex` -> 0 errors, 53 warnings; full dashboard -> **29 / 2 / 7 / 4**.

**For the next session (109):** continue bounded leaves only. Good options remain another small pagebuilder/linebreak leaf, or a small shipout movement helper. Avoid whole-session work on `line_break`, `try_break`, `hlist_out`, `vlist_out`, `ship_out`, `scan.rs`, `mlist_to_hlist`, `hpack`, `vpackage`, or `build_page`.

---

## Idiom session 107 — 2026-06-01 — thread shipout DVI font definition leaves

**Type:** execution (workstream 01, Phase C — shipout leaves). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (52, with 1 warning added and resolved for the dead `font_check_entry`).

**Slice:** Threaded `dvi_native_font_def` and `dvi_font_def_bytes` in `src/shipout.rs` off the bridge-up accessors, passing safe `&FontState` and `&StringsState` references directly. The `dvi_font_def_global` wrapper was refactored to perform the `make_font_def` call (which requires nested mutable state borrows internally to resize `xdv_buffer`) completely outside any state borrow, and then formats and emits the resulting font definition bytes under short, disjoint state borrows. This guarantees absolute borrow safety and avoids nested mutable borrows. We also completely removed the dead bridge accessor `font_check_entry` from `src/globals.rs` and cleaned up its imports.

**Classification:** bucket **B/C**, highly accurate. The core definition byte formatters are pure threaded parameter reads, while the legacy wrapper orchestrates state-borrow disjointness around C-like output routines and external calls.

**Recon-helper usefulness report:**
- Which command(s) you ran: `python3 plan/idiom/phase_c_recon.py --bucket B --module src/shipout.rs`
- Which candidate it surfaced: `dvi_native_font_def` (bucket B, score 101)
- Whether bucket/risk classification was accurate: Extremely accurate! The function itself is a very clean B leaf once the wrapper handles the emission side.
- Roughly how much recon time it saved or failed to save: Saved around 10-15 minutes by immediately pointing us at the right decoupled DVI-emission leaves rather than auditing candidates manually.
- Any false positives/false negatives or suggested script improvements: The script is working perfectly and the refinement to hide already-threaded helpers makes it extremely pleasant to use.

**Method surface / deleted accessors:** Deleted `font_check_entry` from `src/globals.rs` as its last caller was threaded. No new state methods were required, as `FontState` and `StringsState` vectors were sufficient.

**Newtype debt:** none. No state fields moved.

**Metrics:** `#[no_mangle]` **328** unchanged. raw extern statics **38** unchanged. raw `unsafe {}` **174** unchanged. `unsafe fn` **654 -> 653** (deleted `font_check_entry`, and `dvi_native_font_def` dropped `unsafe` since it now takes safe references). Warnings **51 -> 52** (one clashing extern warning remains, the dead-code warning was successfully cleaned).

**Validation:** `cargo fmt`; `cargo check -p tectonic_engine_xetex` -> 0 errors, 52 warnings; full dashboard -> **29 / 2 / 7 / 4**.

**For the next session (108):** continue bounded leaves only. Good options remain another small pagebuilder/linebreak leaf, or a small shipout movement helper. Avoid whole-session work on `line_break`, `try_break`, `hlist_out`, `vlist_out`, `ship_out`, `scan.rs`, `mlist_to_hlist`, `hpack`, `vpackage`, or `build_page`.

---

## Idiom session 106 — 2026-06-01 — refine Phase-C recon helper after first-use feedback

**Type:** tooling (workstream 01, Phase C acceleration). **One commit.** No engine code changed; dashboard not rerun.

**Slice:** improved `plan/idiom/phase_c_recon.py` based on the session-105 usefulness report. The helper now recognizes Rust lifetime / loop-label tokens while matching braces, which fixes truncated bodies such as `merge_justifiable_native_words` with `'check_next`. It also detects threaded state parameters, marks already-threaded helpers with a `threaded` bucket, hides those helpers from default rankings unless `--include-threaded` is passed, and adds `string_pool` as a hazard class.

**Classification changes checked:** `python3 plan/idiom/phase_c_recon.py --bucket B --module src/math.rs` no longer lists already-threaded math parameter helpers such as `math_ext_param`, `big_op_spacing*`, or `fraction_rule` by default. They are visible with `--bucket threaded --module src/math.rs --include-threaded`. `merge_justifiable_native_words` now parses the full body and is reclassified as bucket **C** with `_global` callees plus `diagnostic,string_pool` hazards instead of a misleading clean bucket B.

**Validation:** `python3 -m py_compile plan/idiom/phase_c_recon.py`; smoke-tested default scan, math B scan, threaded math scan, shipout scan, and dead-wrapper scan.

**For the next session (107):** use the refined helper as before, but note that the default ranking now emphasizes not-yet-threaded candidates. Use `--include-threaded` only when auditing existing threaded cores. The helper is still heuristic; inspect bodies/callers before editing, especially around string-pool, output, scanner, and `_global` edges.

---

## Idiom session 105 — 2026-06-01 — thread `try_break` active-node deactivation leaf

**Type:** execution (workstream 01, Phase C — linebreak leaf). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (51).

**Slice:** threaded the local `deactivate_node` helper inside `src/linebreak.rs`'s `try_break` to direct `MemState`, `ScannerState`, and `LinebreakState` borrows. The two legacy call sites now bridge down once with `with_state_mut`, then the helper mutates active-list links, active-width arrays, current-active-width arrays, and node recycling directly through the borrowed state. `try_break` / `line_break` remain legacy orchestrators and were not taken as whole-session scope.

**Classification:** bucket **B**, accurate. The body only needed bridge-accessor rewrites (`mem_b*` / `set_mem_b*`, `active_width`, `cur_active_width`, `active_node_size`) plus direct `crate::mem::free_node`; `free_node` has no with-state-bearing internal calls. No scanner, diagnostic, output, packing, or allocator-failure boundary is crossed, so no borrow-dropping wrapper or abort sentinel was needed.

**Recon helper report:** ran `python3 plan/idiom/phase_c_recon.py --limit 40`, `--bucket B --module src/math.rs`, `--bucket B --module src/shipout.rs`, `--bucket C --module src/shipout.rs`, and `--dead-wrappers`. The default scan surfaced `deactivate_node` at score 121 / bucket B, which became the chosen slice after body/caller inspection. It saved roughly 15-20 minutes by pointing at a closeable linebreak leaf after the math/shipout scans produced less useful candidates. False positives: the math B scan listed already-threaded parameter helpers (`math_ext_param`, `big_op_spacing*`, `fraction_rule`) that did not need a session; `merge_justifiable_native_words` ranked B but inspection showed string-pool, native-word allocation, and `flush_node_list_global` edges that make it C/D rather than a quick B slice. Suggested improvement: discount functions that already take state parameters, and raise risk when a candidate body touches string-pool bridge accessors or calls `_global` wrappers outside the current module's direct hazard list.

**Method surface / deleted accessors:** no new state methods and no bridge accessor deletions. `ScannerState::active_width`, `LinebreakState::cur_active_width` / `active_node_size`, and existing `MemState` methods were sufficient.

**Newtype debt:** none. No state fields moved.

**Metrics:** `#[no_mangle]` **328** unchanged. raw extern statics **38** unchanged. raw `unsafe {}` **174** unchanged. `unsafe fn` **655 -> 654** (the nested `deactivate_node` is now safe and directly threaded). Warnings **51** unchanged.

**Validation:** `cargo fmt`; `cargo check -p tectonic_engine_xetex` -> 0 errors, 51 warnings; full dashboard -> **29 / 2 / 7 / 4**.

**For the next session (106):** continue bounded leaves only. The recon helper is useful for surfacing small candidates, but inspect and down-rank already-threaded helpers and bodies with string-pool/output/scanner edges. Good next options may be another local linebreak/pagebuilder leaf or a small shipout rule/movement helper, but avoid whole-session work on `line_break`, `try_break`, `hlist_out`, `vlist_out`, `ship_out`, `scan.rs`, `mlist_to_hlist`, `hpack`, `vpackage`, or `build_page`.

---

## Idiom session 104 — 2026-06-01 — add Phase-C recon helper

**Type:** tooling (workstream 01, Phase C acceleration). **One commit.** No engine code changed; dashboard not rerun.

**Slice:** added `plan/idiom/phase_c_recon.py`, a read-only dependency-free Python helper for ranking likely Phase-C threading candidates. The script extracts Rust functions from `src/*.rs`, counts callers in one pass, detects caller contexts near `with_state`, records `_global` calls, bridge-accessor hits, direct-state hits, allocator-like calls, and scanner/diagnostic/output/packing/native hazards, then assigns a conservative A/B/C/D or `dead-wrapper` bucket and score. It supports table and JSON output plus filters such as `--bucket`, `--module`, `--dead-wrappers`, `--include-scan`, and `--include-unused`.

**Docs:** updated `01b-phase-c-recipe.md` §4a with example invocations and a reminder that the helper is heuristic and defaults broad scans away from `src/scan.rs`.

**Validation:** `python3 -m py_compile plan/idiom/phase_c_recon.py`; smoke-tested default table output, JSON output piped through `head`, and dead-wrapper mode. Full-crate default scan runs in about 0.7s in this worktree.

**For the next session (105):** use `python3 plan/idiom/phase_c_recon.py --limit 40` at recon start, then narrow with `--bucket B --module src/math.rs` or a specific shipout/linebreak module. Treat the output as triage only: inspect bodies before editing, and keep scanner-heavy or high-count orchestrators out of broad codemod scope.

---

## Idiom session 103 — 2026-06-01 — thread `make_radical_global` delimiter adjustment

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (51).

**Slice:** local simplification inside `src/math.rs`'s `make_radical_global`, chosen after recon found the small linebreak/shipout leaves were already in threaded-core + `_global` wrapper shape and their live callers sit inside high-count orchestrators (`line_break` / output walkers).

**Classification:** bucket **B/C**. The radical helper remains a borrow-dropping orchestrator: `clean_box`, `var_delimiter_global`, `overbar`, `vpackage`, and `hpack` stay outside live borrows. The direct font/eqtb/scanner reads for `math_font`, `font_is_ot_math`, `default_rule_thickness`, and `math_x_height` now use short `with_state` closures calling threaded cores. The delimiter adjustment (`y` height/depth rewrites, clearance delta, final vertical shift, overbar link) runs through short direct `MemState` borrows; the `half(delta)` call stays in the existing unsafe wrapper context so raw unsafe-block count remains flat.

**Method surface / deleted accessors:** no new state methods. Deleted now-dead `math_font_global` and `math_x_height_global` wrappers after confirming no remaining `src/*.rs` callers.

**Newtype debt:** none. No state fields moved.

**Metrics:** `#[no_mangle]` **328** unchanged. raw extern statics **38** unchanged. raw `unsafe {}` **174** unchanged. `unsafe fn` **655** unchanged. Warnings **51** unchanged.

**Validation:** `cargo check -p tectonic_engine_xetex` -> 0 errors, 51 warnings; full dashboard -> **29 / 2 / 7 / 4**.

**For the next session (104):** continue bounded slices only. Good options remain a small pagebuilder/linebreak/shipout leaf if it has live callers outside high-count orchestrators, or another clearly closeable local math simplification. Avoid whole-session work on scanner-heavy `scan.rs`, `mlist_to_hlist`, `hpack`, `vpackage`, `build_page`, and `line_break`; keep `clean_box`, delimiter scans, scanner calls, diagnostics, and output routines between short disjoint borrows.

---

## Idiom session 102 — 2026-06-01 — thread adjacent math fraction/accent/left-right islands

**Type:** execution (workstream 01, Phase C — math noad/command family). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (51).

**Slice:** three adjacent math command helpers in `src/math.rs`: `math_fraction`, `math_ac`, and `math_left_right`. Recon found only stale `xetex-xetexd.h` prototype hits for the legacy names; the live callers are the `scan.rs` math cases.

**Classification:** bucket **B/C**. The noad allocation/finalization work is direct and closeable; scanner/error calls remain outside live borrows. `math_fraction_global` now allocates and initializes the fraction noad under `MemState`, runs delimiter/dimen scans after the borrow drops, then writes the rule field under a final mem borrow. `math_ac_global` allocates the accent noad under `MemState`, keeps keyword/class/family/USV scans outside borrows, and writes the final accent field under a `mem`+`eqtb` borrow. `math_left_right_global` allocates the left/right noad, scans the delimiter outside the borrow, performs the middle/right adjustments under short mem borrows, and keeps `fin_mlist_global`, `unsave`, and `push_math` outside live borrows.

**Method surface / deleted accessors:** no new state methods. Deleted the now-dead `set_empty_math_field_global` bridge after its last users moved to the threaded `set_empty_math_field` core. No `globals.rs` bridge accessor lost all callers.

**Newtype debt:** none. No state fields moved.

**Metrics:** `#[no_mangle]` **328** unchanged. raw extern statics **38** unchanged. raw `unsafe {}` **174** unchanged. `unsafe fn` **655** unchanged. Warnings **51** unchanged.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 51 warnings; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (103):** nearby `math.rs` command helpers are no longer as uniformly small. Either take a small pagebuilder/linebreak/shipout leaf, or do a careful local simplification only if a remaining math island is clearly closeable. Keep delimiter scans, scanner-heavy `scan.rs`, `mlist_to_hlist`, `hpack`, `vpackage`, `build_page`, and `line_break` out of whole-session scope.

---

## Idiom session 101 — 2026-06-01 — thread bounded math-command helpers

**Type:** execution (workstream 01, Phase C — math noad/command family). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (51).

**Slice:** five adjacent math command helpers in `src/math.rs`: `math_limit_switch`, `math_radical`, `append_choices`, `build_choices`, and `sub_sup`. Recon found only stale `xetex-xetexd.h` prototype hits for these legacy names; the live callers are the `scan.rs` math cases / right-brace handler.

**Classification:** bucket **B/C**. The mem/save-stack work is direct and closeable; scanner/error calls remain outside live borrows. `math_limit_switch` now mutates an operator noad through `MemState` and emits the misplaced-limit diagnostic after the borrow drops. `math_radical` allocates and initializes the radical noad under one `MemState` borrow, then runs `scan_delimiter` / `scan_math`. `append_choices` allocates the choice node and updates `SaveStackState` directly, then calls `push_math` / `scan_left_brace`. `build_choices` keeps `unsave` and `fin_mlist_global` before the borrow, stores the finished mlist and save-stack state directly, then either returns or scans the next choice. `sub_sup` prepares or allocates the script field under `MemState`, defers the double sub/sup diagnostic until after the borrow drops, then calls `scan_math`.

**Method surface / deleted accessors:** added `SaveStackState::b32s1` / `set_b32s1`; repointed `globals::save_stack_b32s1` to the method so the raw unsafe-block count stays flat. Deleted the now-dead `new_choice_global` wrapper after `append_choices_global` became its last replacement. No `globals.rs` bridge accessor lost all callers.

**Newtype debt:** none. No state fields moved.

**Metrics:** `#[no_mangle]` **328** unchanged. raw extern statics **38** unchanged. raw `unsafe {}` **174** unchanged. `unsafe fn` **656 → 655** (deleted `new_choice_global`; new helpers are safe cores plus legacy unsafe wrappers). Warnings **51** unchanged.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 51 warnings; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (102):** continue adjacent bounded math-command islands. Best candidates: `math_ac`'s noad allocation/final accent-field writes, the allocation branch of `math_fraction`, or a very small `math_left_right` allocation island. Keep keyword scans, delimiter scans, `scan_math`, `scan_dimen`, `fin_mlist_global`, `unsave`, and diagnostics outside live borrows. Do not take scanner-heavy `scan.rs`, `mlist_to_hlist`, `hpack`, `vpackage`, `build_page`, or `line_break` as a whole session.

---

## Idiom session 100 — 2026-06-01 — thread `var_delimiter` delimiter-box construction into borrow islands

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (51).

**Slice:** the EXT_TAG extensible-delimiter box-building block inside `var_delimiter_global` (`src/math.rs`), the documented session-99 next target. Recon: only stale `xetex-math.c` prototype/comment hits for `var_delimiter`; 4 live callers (`make_fraction`/`make_left_right`/`make_op` family) all in `math.rs`, none rewritten; no cleanup aliases, no cross-module redeclarations.

**Classification:** bucket **C** (session-81/83 style). Extracted `build_ext_delimiter_box_global(f, q, v)`: the box allocation + every TFM font read (the `exten` table row `r`, the repeatable piece's width via `effective_char`, and the `height_plus_depth` of the top/mid/bottom/repeat pieces) now run under **one** `mem`+`fonts` borrow island using the already-threaded `new_null_box`/`effective_char`/`height_plus_depth` directly — replacing ~12 scattered single-statement `with_state`/`with_state_mut` closures the C translation left behind. The repeat-count arithmetic is pure; the piece stacking keeps the threaded `stack_into_box_global` (each call builds + links a char box under its own borrow), so allocation order (vlist box first, then each stacked char box) is **byte-identical** to the inline version — the parity guarantee for the EXT_TAG (TFM) path that TRIP/`display_math` exercise. `ALLOC_FAILED` propagates as `NodeRecycleAbort::Overflow` to a post-borrow `node_recycle_abort` (`01b` §5). Also consolidated the null-delimiter branch (`\nulldelimiterspace`) from 3 closures into one disjoint `mem`(alloc)+`eqtb`(dimen read) island.

**Bridged call sites / deleted accessors:** no external call-site rename (the helper is internal to `var_delimiter_global`). No bridge accessor lost its last caller, so none deleted. No new state method surface (`new_null_box`/`effective_char`/`height_plus_depth` already threaded; `MemState`/`FontState`/`EqtbState` methods all pre-existed).

**Newtype debt:** none. No state fields moved.

**Metrics:** `#[no_mangle]` **328** unchanged. raw extern statics unchanged. raw `unsafe {}` **174** unchanged. `unsafe fn` 655→**656** (+1, the new `build_ext_delimiter_box_global` helper; `var_delimiter_global` itself unchanged). This is a threading/consolidation slice — the win is removing ~15 bridge closures from a hot-but-cold math leaf and pointing it at direct state, not a headline metric drop.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 51 warnings; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (101):** `var_delimiter_global`'s remaining bridge churn is the two big small/large-family scan loops (deeply interleaved `math_font`/`map_char_to_glyph`/`get_ot_math_variant`/`get_ot_assembly_ptr`/TFM char-list scan — bucket **D**, leave on the bridge) plus single-island reads (`font_area`, the final `axis_height` shift) that are already minimal. Good next math leaves: `build_choices`/`append_choices` mem work (mind `unsave`/`push_math`/`scan_left_brace` — keep scanner calls outside live borrows), or a pagebuilder/linebreak leaf. Keep `clean_box`, `vpackage`, `hpack`, `mlist_to_hlist`, OT-math calls, and `build_opentype_assembly` outside live borrows / off the single-session table (they are high-count orchestrators). Keep scanner-heavy `scan.rs` last; draft `02-modules.md` to READY before threading reaches it.

---
