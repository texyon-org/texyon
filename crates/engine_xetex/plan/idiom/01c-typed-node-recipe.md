# 01c — Typed-node foundation cookbook (`NodePtr<K>` per-slice recipe)

The `READY` execution recipe for the **typed foundation** that `01`-§4a
requires before the fused "type-as-you-thread" Phase-C slices (`01b`
§3a/§4b) can run. It is the per-slice analog of `01b`: where `01b` is the
recipe for *threading* a function, this is the recipe for *building the
typed surface* (`MemIdx`/`MemPtr`/`Scaled`/… scalar newtypes → the `Mem`
ownership move → the per-family `NodePtr<K>` handle layer) that a threaded
slice then adopts.

> Status: **READY** for execution. Spot-checked against the current
> source 2026-06-06 (paths, node families, field offsets, and the recon
> counts in §1 all verified; see the inline `file:line` citations).
> Known, disclosed limitation: the per-family traffic split in §1 is a
> *header-informed split of the mechanical (subfield, offset) histogram*,
> not a per-site family classification — Stage 1 collapsed the per-family
> C macros (`BOX_width`, `GLUE_SPEC_stretch`, …) into the generic
> `mem_b32s1(p + N)` accessor (`src/globals.rs:307`), so a given
> `(subfield, offset)` signature maps to *several* families. The build
> **order** (§1, §2) is robust to this; do not read the per-family
> numbers as exact site counts.

Read this **with** `01-state-and-memory.md` §4 (option (c)), §4a (the
foundation-first prerequisite this doc executes), §5 (the `Mem` arena),
§7 (longjmp/`Drop`), §8 (validation incl. the disasm gate);
`01b-phase-c-recipe.md` §3a/§4b (how a slice adopts the surface this doc
builds); and the target-level spec `../03-memory-and-nodes.md`.

---

## 1. Node-family recon (the data-driven build order)

### 1.1 Method (reproducible)

Stage 1 lost the per-family field macros: every node-field read/write is
now `mem_<subfield>(expr + N)` through the generic word accessors in
`src/globals.rs:307` (`mem_b32s1`/`set_mem_b32s1`/…). The only mechanical
signal left is the `(subfield, offset)` pair, which is exactly what the C
field macros key on (`xetex/xetex-xetexd.h:204–322`). `node_field_recon.py`
(this directory) counts every site by that pair:

```bash
python3 plan/idiom/node_field_recon.py --by-file
```

Measured 2026-06-06 (`HEAD` = 9cff278c):

| metric | count |
|---|---:|
| node-**field** sites (offset `+N`, `N ≥ 1`) | **742** |
| node-**header**-word sites (offset 0: link/info/type/subtype/char) | 1378 |
| distinct `(subfield, offset)` signatures | 37 |

The 742 matches the session-143 "~725 (p + N) field-access sites"
estimate. The 1378 offset-0 sites are the *shared header word* every node
has (link `b32.s1@0`, info `b32.s0@0`, type `b16.s1@0`, subtype
`b16.s0@0`, and the char-node `font`/`character` aliases of those).

Top signatures (full table from the script):

| subfield | +off | reads | writes | total | C field macro(s) at this signature |
|---|---:|---:|---:|---:|---|
| b32.s1 | +0 | 406 | 252 | **658** | `LLIST_link` (link / rlink / glue_spec **ref_count**) |
| b16.s1 | +0 | 190 | 75 | 265 | `NODE_type` / `CHAR_NODE_font` / `*_order` |
| b16.s0 | +0 | 206 | 59 | 265 | `NODE_subtype` / `CHAR_NODE_character` |
| b32.s1 | +1 | 242 | 2 | 244 | `BOX_width`, `PENALTY_penalty`, `KERN width`, `GLUE_leader_ptr`, `ACTIVE break_node`, `DISC post_break`, `LIG lig_ptr`, `MARK_ptr`, `INS float_cost`, `LANG what_lang`, `WRITE tokens`, `PASSIVE cur_break`, `DELTA dwidth` |
| b32.s1 | +3 | 134 | 0 | 134 | `BOX_height`, `GLUE_SPEC_shrink`, `ACTIVE shortfall` |
| b32.s0 | +0 | 64 | 67 | 131 | `LLIST_info` |
| b32.s1 | +2 | 109 | 0 | 109 | `BOX_depth`, `GLUE_SPEC_stretch`, `ACTIVE total_demerits`, `PAGE_INS last_ins_ptr`, `EDGE edge_dist` |
| b32.s0 | +1 | 51 | 0 | 51 | `GLUE_glue_ptr`, `DISC pre_break`, `ACTIVE line_number`, `MARK class`, `PASSIVE prev_break`, `ETEX_SA_ref`, `PAGE_INS broken_ins` |
| b32.s1 | +5 | 40 | 0 | 40 | `BOX_list_ptr` |
| b32.s1 | +4 | 25 | 0 | 25 | `BOX_shift_amount`, `ACTIVE glue`, `INS split_top_ptr` |
| b16.s1/s2/s3/s0 | +4 | 22/21/6/2 | … | 51 | `NATIVE_NODE_length`/`glyph`, `font`, `size`, `glyph_count`; `PIC path_len` |
| b16.s1 | +5 | 13 | 0 | 13 | `BOX_glue_sign` |
| b16.s0 | +5 | 12 | 0 | 12 | `BOX_glue_order` |
| gr | +6 | 12 | 0 | 12 | `BOX_glue_set` (the glue ratio, an `f64`) |
| ptr | +5 | 3 | 0 | 3 | `NATIVE_NODE_glyph_info_ptr` (a native `*mut c_void`) |
| … | | | | | (remaining signatures in the script output) |

Per-file traffic (where the layer pays off): `main_control.rs` 398,
`math.rs` 336, `shipout.rs` 283, `linebreak.rs` 223, `packaging.rs` 210,
`output.rs` 174, `pagebuilder.rs` 145, `scan.rs` 145, `expand.rs` 111.

### 1.2 The ranked family build order

Ranked by **highest-traffic + simplest-invariant first** — a family
ranks high when it owns a wide band of high-count offsets *and* its fields
are simple (fixed-size node, non-nullable scalar fields). The C node sizes
are `xetex/xetex-constants.h:107–139`; the field map is
`xetex-xetexd.h:204–322`.

| rank | family | node size | fields it owns (offset.subfield → type) | invariant class | why here |
|---:|---|---|---|---|---|
| **0** | **shared header word** (every node) | n/a | `link` 0.b32.s1 → `MemPtr`; `info` 0.b32.s0 → `MemPtr`/raw; `type` 0.b16.s1 → `NodeType`; `subtype` 0.b16.s0 → `u16`/subtype enum | universal; `link` is nullable (`MemPtr`) | 1378 sites; the base every `NodePtr<K>` and `classify` builds on — must land with rank 1 |
| **1** | **box (hlist/vlist)** | 8 (fixed) | `width` +1.b32.s1, `depth` +2.b32.s1, `height` +3.b32.s1 → `Scaled`; `shift_amount` +4.b32.s1 → `Scaled`; `list_ptr` +5.b32.s1 → `MemPtr`; `glue_sign` +5.b16.s1, `glue_order` +5.b16.s0; `glue_set` +6.gr → `f64` | **simplest**: fixed size, dims are pure `Scaled`, one nullable ptr | owns the widest high-traffic band (+1..+6); dims dominate packaging/shipout/linebreak |
| **2** | **glue_spec + glue node** | spec 4, glue node small (2) | spec: `ref_count` 0.b32.s1 (**a count, NOT a link**), `stretch_order` 0.b16.s1, `shrink_order` 0.b16.s0, `stretch` +2.b32.s1, `shrink` +3.b32.s1 → `Scaled`; glue node: `glue_ptr` +1.b32.s0 → `MemPtr`, `leader_ptr` +1.b32.s1 → `MemPtr` | fixed size; **one trap**: glue_spec reuses the header `b32.s1` word as a refcount | high traffic; pervasive in every list |
| **3** | **kern** | 3 (fixed) | `width` +1.b32.s1 → `Scaled` | trivial: one `Scaled` field | single field, fixed size |
| **4** | **penalty** | small (2) | `penalty` +1.b32.s1 → **`i32` (NOT `Scaled`)** | trivial; just don't mistype it `Scaled` | single field, fixed size |
| **5** | **rule** | 5 (fixed) | `width`/`depth`/`height` +1/+2/+3.b32.s1 → **`Scaled`-or-`NULL_FLAG`** (running dimension) | fixed size, **but** the running-dimension sentinel `NULL_FLAG = -0x40000000` (`xetex-constants.h:16`) means these are not plain `Scaled` | needs the running-dimension wrapper before its fields are typed |
| **6** | **disc** | small (2) | `pre_break` +1.b32.s0, `post_break` +1.b32.s1 → `MemPtr` (llink/rlink) | two nullable ptrs | moderate traffic; pointer fields |
| **7** | **ligature** | small (2) | `lig_font` +1.b16.s1 → `FontId`, `lig_char` +1.b16.s0 → char, `lig_ptr` +1.b32.s1 → `MemPtr` | mixed-width subfields of one word | moderate; mixed types |
| **8** | **math / noad** | 4 (`NOAD_SIZE`), variants 5/6 (accent/radical/fraction) | nucleus/subscr/supscr at +1/+2/+3 as packed `(math_type, fam·class·char)` records; **no C field macros** — accessed as raw `mem[p+N]` | **complex**: packed sub-records + a 2nd sub-classification (`ORD_NOAD`/`STYLE_NODE`/`CHOICE_NODE`/accent/radical/fraction) | high `math.rs` traffic but the hardest typing; do after the simple families |
| **9** | **whatsit / native** | variable (`NATIVE_NODE_SIZE`=6 + glyph data inline) | `size` +4.b16.s3, `font` +4.b16.s2, `length`/`glyph` +4.b16.s1, `glyph_count` +4.b16.s0, `glyph_info_ptr` +5.ptr (`*mut c_void`); `text` lives **inline** at `mem[p + NATIVE_NODE_SIZE]` | **hardest**: variable-size, trailing inline UTF-16/glyph data, a raw native pointer, FFI-measured (`01b` §10) | last; the `enum`-per-node argument (`03` §"Why not") dies here |
| **10** | char/token one-word nodes (hi-mem) | 1 | `font` 0.b16.s1, `character` 0.b16.s0 (aliases of type/subtype) | live **above `hi_mem_min`**; identity *is* the `is_char_node` region test (§7) | a classification concern, not a `+N` concern; folds into rank 0 |
| **11** | linebreak/page internals: active/passive/delta, page_ins, mark/ins/adjust, unset/align, edge, span, expr | 2–7 | per the macros at `xetex-xetexd.h:221–322` | specialized, lower traffic | thread with their owning subsystem (`linebreak.rs`/`pagebuilder.rs`), not up front |

**Built so far:** rank 0+1 (header word + box, session 147), rank 2
(glue/glue_spec, session 148). **Next: rank 3 (kern).**

**Build the layer in this order, one family per slice** (rank 0 lands
with rank 1). Stop promoting families to typed handles when the remaining
ranks are low-traffic enough that the worklist-fallback (§6) is cheaper
than a dedicated slice — those families stay `MemIdx`/`i32` with a
worklist row and get typed opportunistically as their subsystem threads.

## 2. Foundation build order and gating (concrete `01`-§4a)

Three stages, strictly ordered. Each is its own small commit(s); the
dashboard (`29/2/7/4`) is green at every commit.

### Stage 1 — scalar newtypes (one module, gated by disasm)

Create the newtypes in `src/units.rs` (new) or fold into
`src/mem_types.rs`. Each is `#[repr(transparent)] + #[derive(Copy, Clone)]`.

- `MemIdx(i32)` — a **valid** arena index; **never** a sentinel.
- `MemPtr(i32)` — a nullable TeX halfword: a `MemIdx` *or* `TEX_NULL`.
  `MemPtr::NULL == MemPtr(TEX_NULL)`. `as_idx() -> Option<MemIdx>`.
- `Scaled(i32)`, `StrNumber(i32)`, `FontId(i32)` (note: `int32_t` in
  `xetex-xetexd.h:326` — keep `i32`, do **not** prematurely narrow to
  `u16`), `Token(i32)`, and the eqtb/hash index wrappers.
- `NodeType` with a **checked** `from_raw(u16) -> Result<NodeType, _>`
  (`TryFrom`/`match`) — never `transmute` (invalid discriminant = UB,
  `03` §"Node-type variants").

**Two corrections this slice MUST make (verified hazards, do not
re-litigate):**

1. **`TEX_NULL = MIN_HALFWORD = -0x0FFFFFFF`**, not `0`
   (`src/consts.rs:13`, `xetex/xetex-constants.h:14`). The vestigial
   `MemIdx::NULL = MemIdx(0)` / `is_null()` at `src/mem.rs:64–69` is
   **wrong** (it tests `0`, not `MIN_HALFWORD`) and the parent `03`
   sketch's `MemPtr::NULL = MemPtr(0)` (`03-memory-and-nodes.md:126`) is
   **wrong for this codebase**. Put null-ness on `MemPtr` keyed to
   `TEX_NULL`; remove it from `MemIdx`.
2. The existing `MemIdx`/`Scaled`/`FontId`/`Token`/`StrNumber` in
   `src/mem.rs:56–96` are **vestigial stubs** — used only at their
   definitions (`MemPtr`/`NodePtr` don't exist yet). This slice
   *consolidates* them into the foundation module and fixes (1); it is
   effectively greenfield, not a migration of live types.

**Gate:** run the §4 disasm spot-check proving the newtypes are zero-cost
vs. raw `i32` before any later stage depends on them.

### Stage 2 — the `Mem` ownership move (`01`-§5 steps 1–2) — ALREADY COMPLETE

> **STATUS (verified session 147): DONE in earlier execution sessions
> (11 = `save_stack`, 51 = `mem`), before this cookbook was written.** The
> arena already lives on `MemState { data: Vec<MemoryWord>, lo_mem_max,
> hi_mem_min, avail, mem_end, rover, var_used, dyn_used }` (`src/state.rs:403`)
> — the exact shape below. Evidence: allocation is `s.mem.data = vec![…]`
> at `src/engine.rs:1073` (initex, `MEM_TOP + 2`) and `src/format_io.rs:854`
> (undump, `MEM_TOP + 1`), both with comments tracking the inclusive
> `xmalloc_array` sizing (the `01`-§11 off-by-one gotcha — handled). No
> `xmalloc_array` for `mem`, no C `mem` symbol, no `#[link_name="mem"]`, no
> `install_mem_ptr` shim, and `tt_cleanup` does **not** `free()` it (no
> double-free). Raw-pointer consumers (`mem_ptr`/`mem_node_addr`,
> `src/globals.rs:553`) derive from the `Vec` and are documented as valid
> because the arena never reallocates. Format dump/undump walk the arena via
> `dump_slice`/`undump_slice` byte-exactly; `FORMAT_SERIAL = 33` unchanged
> (`format_io.rs:76`). The surviving `unsafe` is already confined to the
> `MemoryWord` union projection inside the accessors — not a pointer deref.
>
> **So Stage 2 requires no work.** The cookbook described it as pending
> because it was derived from the target spec (`01`-§5, `03`) without
> re-checking that sessions 11/51 had landed it. Proceed directly to Stage 3.
> The original target text is kept below for reference.

`mem` was originally a `static mut *mut MemoryWord` allocated by Rust via
`xmalloc_array` (`01`-§2). It is now a Rust-owned
`Mem { data: Vec<MemoryWord>, lo_mem_max, hi_mem_min, mem_end, avail,
rover, var_used, dyn_used }` (named `MemState` in the code):

- Pre-allocate `data` to the **exact inclusive** size (`xmalloc_array(n)`
  allocates `n + 1`; the off-by-one gotcha, `01`-§11). It must **never
  reallocate** during a run (a realloc invalidates any retained raw
  pointer and ruins codegen comparisons).
- Reconcile `tt_cleanup`/longjmp (`01`-§7): `Drop` does not run on the
  `longjmp` abort path; keep the owner in the thread-local anchor and let
  `tt_cleanup` call an explicit `Mem` teardown (matches today's behavior —
  globals were never freed on abort either).
- Reconcile the format dump/undump for the arena (§7 below): the free-list
  ranges are dumped as raw indices; this slice must keep them byte-exact.

This must land **before** the `NodePtr<K>` field methods, because the safe
`Vec` slice index is what confines the surviving `unsafe` to the
`MemoryWord` *union projection* (not a pointer deref) — that is where most
of the 688 unsafe-block lints die (`01`-§5 step 3).

### Stage 3 — the per-family `NodePtr<K>` layer (one family per slice, §1 order)

> **Progress (session 147): rank 0+1 built** — `src/node.rs` holds the
> `NodePtr<K>` core (transparent handle + `NodeKind` + capability traits
> `HasDimensions`/`HasListPtr`/`HasGlueSet`), the **box family**
> (`HListNode`/`VListNode`) with all 8 box field methods
> (`width`/`depth`/`height`/`shift_amount`/`list_ptr`/`glue_sign`/
> `glue_order`/`glue_set`) reusing the audited `MemState` word accessors,
> and the `MemState::node_type`/`classify_box` checked boundary. 4 unit
> tests; the §4 disasm gate passed. Nothing consumes it yet
> (`#[allow(dead_code)]`) — a Phase-C box-threading slice adopts it next
> (`01b` §3a).
>
> **Progress (session 148): glue family built** — added the
> **glue/glue_spec family** to `src/node.rs`: `GlueNode` (concrete impls:
> `subtype` 0.b16.s0, `glue_ptr` +1.b32.s0 → `MemPtr`, `leader_ptr`
> +1.b32.s1 → nullable `MemPtr`, plus the `glue_spec()` classify helper) and
> `GlueSpecNode` (`ref_count` 0.b32.s1 → **`i32`, NOT a link**;
> `stretch_order` 0.b16.s1, `shrink_order` 0.b16.s0 → `u16`; `stretch`
> +2.b32.s1, `shrink` +3.b32.s1 → `Scaled`). Classification: glue node via
> `MemState::classify_glue` (NODE_type == `GLUE_NODE` = 10); glue_spec via
> `NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr)` (a from-`MemPtr` boundary,
> **not** a `node_type` branch — a glue_spec carries no type code). No new
> capability traits (glue/glue_spec fields are family-specific → concrete
> impls). 5 unit tests (offset round-trips, the ref_count-is-not-a-link
> distinction, glue_ptr/leader_ptr nullability, classify dispatch); the §4
> disasm gate passed on `glue_ptr` (+1.b32.s0) AND `stretch` (+2.b32.s1).
> Remaining families in §1 order: kern → penalty → rule → disc → ligature →
> math → native.

For each family in the §1 ranking (rank 0+1 together, then 2, 3, …): add
its `NodeKind` marker, its capability-trait impls, its field methods, and
its classification entry (§3). Gate each family's hot accessors on the §4
disasm check. A fused threading slice (`01b`) may adopt typed handles
**only for families whose layer is built**; until then it uses the §6
fallback.

## 3. The `NodePtr<K>` design, concretely

Target shape: `../03-memory-and-nodes.md` §"Typed node handles". The
concrete rules for *this* codebase:

- **Payload.** `NodePtr<K>` is `#[repr(transparent)]` over a `MemIdx`
  plus a `PhantomData<fn() -> K>` (compile-time only). It is `Copy`, holds
  **no borrow** of `Mem`, and must compile identically to a raw `i32`
  after inlining (§4).
- **Field methods live on `NodePtr<K>`**, gated by capability traits
  (`HasDimensions`, `HasListPtr`, `HasShiftAmount`, …) for shared layouts
  and concrete `impl`s for one-family fields (`penalty`, `glue_ptr`,
  `native_len`, …). Each method takes `&Mem` (read) or `&mut Mem` (write)
  as its context parameter — **no view structs** (`03`
  §"Avoidance of View Structs").
- **Methods return semantic types**, never raw `i32`/`u16`: `width(&Mem)
  -> Scaled`, `list_ptr(&Mem) -> MemPtr`, `node_type(&Mem) -> NodeType`,
  `lig_font(&Mem) -> FontId`, `glyph_info_ptr(&Mem) -> *mut c_void`.
  **Mirror the exact C field width** — a method that sign-extends or
  truncates differently from `mem_b32s1` is a silent parity break
  (`01`-§10; lean on TRIP).
- **Classification happens ONCE, at a checked boundary.** `Mem::classify(p:
  MemPtr) -> Result<Option<AnyNode>, InvalidNodeType>` checks `TEX_NULL`,
  then the char-node region (`is_char_node`, §7), then `NodeType::from_raw`,
  then subtype where a family is sub-classified (whatsit, noad). A raw
  halfword becomes a `NodePtr<K>` **only** through this boundary — **never**
  by blind `MemIdx::new` on a field that can hold `TEX_NULL` or `NULL_FLAG`
  (`01`-§10 sentinel hazard). Classify once at the control-flow boundary
  and carry the handle; do not re-match the tag per access (`03`
  §"Performance rules").
- **Per-family invariant traps to encode in the methods (from §1):**
  - glue_spec's `0.b32.s1` is a **ref_count**, not a link — do not expose
    it as `link()`/`MemPtr` on `NodePtr<GlueSpec>`.
  - penalty's `+1.b32.s1` is a plain **`i32`**, not `Scaled`.
  - rule's `width/depth/height` are **`Scaled`-or-`NULL_FLAG`** (running
    dimension) — return a type that can represent "running", not bare
    `Scaled`.
  - char nodes have no `+N` fields; their `font`/`character` are the
    header word and only valid once `is_char_node` is true (§7).

## 4. The mandatory zero-cost disasm gate (blocking)

> **Box-family result (session 147): PASSED — strongest form.** A release
> build of `NodePtr::<HListNode>::width(mem)` vs. the raw `mem.b32s1(p + 1)`
> produced **byte-identical machine code**: LLVM's identical-code-folding
> merged the two `#[no_mangle] #[inline(never)]` harness functions to the
> *same address/section* (`objdump -t`: both → `.text.__disasm_raw_width`,
> size 0x28). The typed handle is provably zero-cost over the raw index for
> the box dimension accessors. (The harness was temporary and removed before
> commit; re-run it the same way when a new family's hot accessor lands.)

> **Glue-family result (session 148): PASSED — strongest form.** Same ICF
> outcome for the glue accessors. Temporary `#[no_mangle] #[inline(never)]`
> pairs `__disasm_typed_glue_ptr` vs. raw `mem.b32s0(p + 1)` and
> `__disasm_typed_stretch` vs. raw `mem.b32s1(p + 2)`, release build,
> `objdump -t`: each typed fn folded to the *same section/address* as its raw
> twin (`__disasm_typed_glue_ptr` → `.text.__disasm_raw_glue_ptr` size 0x27;
> `__disasm_typed_stretch` → `.text.__disasm_raw_stretch` size 0x29) —
> byte-identical machine code. Proves the `+1.b32.s0` "other word half" shape
> and the `+2.b32.s1` `Scaled`-at-offset shape are both zero-cost. Harness
> removed before commit.

**Before any fused slice depends on a wrapper**, prove in a *release-mode*
build that the typed access compiles to the same load/store as the raw
`i32` deref (`01`-§4, §8; `03` §"Performance rules"). This is a hard gate,
not advisory — TeX is hot in the tokenizer/expansion/linebreak loops
(`00`-overview "Performance is preserved"). Check these specific hot
accessors (one per representative shape):

| accessor | shape it proves |
|---|---|
| `width(&Mem) -> Scaled` (box, +1.b32.s1) | `Scaled`/`MemIdx`-offset + `Vec` index vs. raw `mem[p+1].b32.s1` |
| `link(&Mem) -> MemPtr` (header, 0.b32.s1) | nullable-ptr return, the linked-list traversal step |
| `node_type`/`classify` (header, 0.b16.s1) | the `NodeType::from_raw` checked path — must fold to a bounds-checked load + jump, no panic-path bloat in the hot branch |
| `glue_ptr(&Mem) -> MemPtr` (glue, +1.b32.s0) | the other word half |
| the linebreak traversal inner loop (`linebreak.rs`) | the end-to-end "carry a `NodePtr` across a loop" pattern |

How: build `--release`, `cargo asm` / `objdump -d` the accessor and a
hand-written raw equivalent, diff the instruction sequences. If a bounds
check appears in a proven-hot path and profiling confirms it costs, add a
*tiny audited* unchecked helper **inside `Mem`** (`03`
§"Performance rules") — do not expose unchecked indexing broadly, and do
not skip the gate. Record the result in the `SESSION_LOG` entry.

## 5. The per-slice recipe (one family per commit)

Run from the crate root. Mirrors `01b` §7's structure for the typing half.

1. **Pick the family** = the next rank in §1 whose prerequisites (Stage 1
   newtypes; Stage 2 `Mem`; rank 0 header layer) are built.
2. **Add the handle + capability impls + field methods** mirroring the
   exact C field widths (`xetex-xetexd.h`), returning semantic types (§3).
   Encode the family's invariant trap (§3 list).
3. **Place the classification boundary** (§3): one `classify`/`expect_<K>`
   entry; the rest of the family's code receives `NodePtr<K>`.
4. **Gate**: run the §4 disasm spot-check on the family's hot accessors.
5. **Validate**: full dashboard
   ```bash
   cargo check -p tectonic_engine_xetex
   cargo test -p tectonic --test tex-outputs --test trip \
     --test engine_rust --test latex_goldens     # expect 29 / 2 / 7 / 4
   ```
   plus the disasm check from step 4. Never regress parity.
6. **Commit small** (`engine_xetex:` prefix, engine-crate files only) and
   update `SESSION_LOG`/`STATUS` (hard rule 5). One family per commit
   (A.12) — do not build six families in one untested diff.

## 6. Dovetail with `01b`'s fused threading (+ the worklist fallback)

The foundation and the giant-threading run **concurrently**, decoupled by
the worklist (`01`-§4a):

- A `01b` threading slice that touches a family whose layer **is built**
  adopts its typed handles in the same commit (`01b` §3a): typed accessor
  returns, `NodePtr<K>`/`MemPtr` params, classification at the boundary,
  conversion shims at still-`i32` callers that vanish as those callers
  thread.
- A slice that touches a family whose layer is **not built yet** (or a
  field whose index-vs-sentinel invariant is not yet audited) leaves that
  family raw (`i32`/`MemIdx`) **and records a deferred-newtype-worklist
  row** in `01a-global-inventory.md` (`01`-§4a fallback). This is what
  keeps the foundation schedule from stalling the threading that is
  currently flowing.
- **The worklist is the typing backlog of record** — there is no separate
  "second full call-site pass." The §8 metrics track convergence:
  bare-`i32` node-pointer params → 0, and worklist length → 0. A fused
  slice may *add* a row, but the net trend is down (`01b` §8).
- **Closure rule** (`01b` §3a): a slice is unclosed if a single body mixes
  typed and raw indexing for the *same* family with no worklist row —
  finish the family's layer or record the debt.

`thread_fn.py` stays a **threading** tool (`01b` §4b): typing is
compiler-driven, never regex. The `NodePtr<K>`-kind / nullability /
sentinel decisions are the `01b` §10 hazards — read and written by hand,
proven by TRIP + the §4 disasm check, never auto-applied.

## 7. Hard constraints / hazards this codebase imposes (verified)

State these so a future session does not re-litigate them. The typed layer
must **preserve** these properties, not hide or "clean up" them.

1. **The lo/hi `is_char_node` partition is an index/region comparison.**
   `is_char_node(p) ≡ p >= hi_mem_min` (`src/mem.rs:910`). Character nodes
   are not a separate allocation — they live in the upper one-word region
   of the same `mem` arena. `classify` (§3) must implement this region
   test, and `MemIdx`/`NodePtr<K>` must preserve the single contiguous
   index space across the lo/hi boundary. Do **not** model char nodes as a
   type outside the arena.
2. **The format file (`.fmt`) stores raw indices + free-list state.**
   `store_fmt_file` calls `sort_avail()` then dumps `lo_mem_max`, `rover`,
   the free-list-walked `mem` ranges, `hi_mem_min`, and `avail`
   (`src/format_io.rs:299–339`); `FORMAT_SERIAL` (currently `33`,
   `format_io.rs:76`) is public API (`../07-anti-patterns.md` A.10). The
   index space and the free-list representation **cannot be abandoned** —
   any typed layer must dump/undump byte-identically. Bump `FORMAT_SERIAL`
   only if the on-disk image deliberately changes (it must not, for a
   behavior-preserving refactor).
3. **`var_used`/`dyn_used` are byte-observable.** They are printed on the
   memory-usage stats line (`crate::output::print_int(var_used())` /
   `dyn_used()`, `src/format_io.rs:355–357`). The allocator's bookkeeping
   is therefore part of the output parity — do **not** perturb when/how
   `var_used`/`dyn_used` are incremented. (They move *with* the `Mem`
   arena per `01`-§5 / `01`-§11 gotcha, not as a loose scalar slice.)

## 8. NON-GOAL — keep TeX's free-list arena (do NOT heap-allocate per node)

**Do not** replace TeX's free-list arena with per-node Rust heap
allocation — no `Box<Node>`, no `enum`-per-node, no `slotmap`. This is a
hard non-goal, not a deferred option. It is forbidden by `../07-anti-patterns.md`
A.5 (no `Box<dyn Node>`) and A.7 (no algorithm changes), and the three §7
constraints make it actively breaking:

- It **breaks the format dump** (§7.2): the `.fmt` image is raw indices +
  free-list ranges; a heap allocator has neither.
- It **breaks the `is_char_node` partition** (§7.1): the lo/hi region test
  presupposes one contiguous index space.
- It **breaks `var_used`/`dyn_used` parity** (§7.3): a different allocator
  produces different bookkeeping, and that bookkeeping is printed.
- It **ruins performance** (`00`-overview "Performance is preserved";
  `03` §"Why not an enum-per-node model?"). TeX's allocation pattern is the
  pathological case for a general allocator: huge frequency, tiny objects,
  high churn (the tokenizer mints a one-word node per token via
  `get_avail`). The free-list over a contiguous `Vec<MemoryWord>` gives
  O(1) pop/push, cache locality on linked-list walks, 4-byte indices (not
  8-byte pointers — half the memory traffic in loops that are
  memory-bound), and no `Drop` cost on free. `malloc`/`free` per node is
  ~1–2 orders of magnitude heavier and cache-hostile.

The free-list allocator (`get_avail`/`get_node`/`free_node`/`flush_list`/
`sort_avail`, `src/mem.rs`) **stays**, as safe `&mut Mem` methods.
"De-manual-memory" means removing the **unsafe raw-pointer derefs and
bare-`i32` arithmetic** — replacing them with safe `Vec` indexing and
typed `NodePtr<K>` handles — **not** removing Knuth's allocator algorithm.
That is the only point in the design space that wins the safety/idiom
prize without paying the perf, format, and parity costs. (A one-line
version of this is folded into `01`-§5.)
