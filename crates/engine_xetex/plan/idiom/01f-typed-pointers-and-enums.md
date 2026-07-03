# 01f — Phase-2: typed pointers + constants→enums (execution plan)

Follows 01c (the `NodePtr<K>` family cookbook — the family layer is BUILT, 30+
families in `node.rs`). This phase is ADOPTION + enum introduction, driven from a
session-249 recon workflow (4 parallel readers + synthesis, source-verified).

**User's quality bar** (see memory `phase2-typing-derive-at-match-arms`): cleaner /
more readable / less error-prone — NOT types-for-types-sake. Derive the concrete type
AT the big classification match-arms and flow it outward; no cast/`expect` at use
sites. Fold match-dispatched magic constants into checked enums (`from_raw`->Result,
never `transmute`). Leave genuine i32 alone.

## Ground truth
- `NodeType` + `from_raw(u16)->Result` at `units.rs:155/201` (0–15, noads 16–31,
  MarginKern=40). `MemState::classify(p)->Result<AnyNode,_>` at `node.rs:595`;
  per-family `classify_*` at `node.rs:632…1832`; `AnyNode` at `node.rs:529`.
- Canonical consume: `match mem.classify(idx) { Ok(AnyNode::HList(h)) => … }`
  (`packaging.rs:2122` hpack_in, `:2745` vpackage_in) — the TEMPLATE.
- Allocators `mem.rs:266–784` all end `return p.idx().0` (i32).

## Byte-parity invariant for every classify() conversion (hot slices)
classify() is TOTAL + `Result`; raw dispatches are PARTIAL (`match b16s1(p) as i32 {…
_ => }`). Convert to: classify once at top → **single** `.expect("valid node type")`
(this IS "classify once", not a use-site expect) → match `AnyNode` with a `_ =>`
catch-all mirroring the original default. Never a per-arm expect. `from_raw` reads
**u16** (old arms cast `as i32`); route via `classify()`/`node_type()`; mind the
MARGIN_KERN=40 gap.

## DO NOT TOUCH — deliberate i32
- Noad nucleus/subfield i32 stores in `math.rs` (~28: `set_nucleus_field/type`,
  `set_list_ptr`, 695…4087) — Noad layout stores ptrs as i32 by design.
- `linebreak.rs:144–199` context-boundary `*_node_unchecked` wrappers — keep.
- `from_idx_unchecked`/`MemPtr::from_raw` list-ptr reconstruction (~296) — leave.
- `math.rs:520` nucleus content dispatch (MATH_CHAR|SUB_BOX|SUB_MLIST) — sub-field
  domain, NOT node type.
- Glue fill-order/`g_sign`/`g_order` in the shipout glue-set inner loop
  (`shipout.rs:2392–2413`) — hot-loop discriminant reads.
- Offset-arithmetic locals (`q_raw = q.idx().0; q_raw+1`), child-link raw writes
  (`linebreak.rs:233-234,680,1114`).

## Recon corrections (settled)
1. IfTest enum — DROP (`expand.rs:158` is already a match on named consts, 0 payoff).
2. WhatsitSubtype — non-contiguous, spans low (OPEN/WRITE/CLOSE/SPECIAL/LANGUAGE) +
   high (NATIVE_WORD=40/NATIVE_WORD_AT=41/GLYPH=42/PIC/PDF); needs a match-table
   `from_raw`, and the `b16s0(p) <= 4` filters must become an explicit membership
   predicate, NOT a range (gather full list from xetex_format.h first). → S8.
3. `math.rs:4880` `classify_math(..).expect().expect().width()` is a double-expect
   anti-pattern TODAY — a TARGET (S9), not done.

## Ordered slices (leverage/risk)
- **S1 PageboxType** enum (cold, ~2 sites) — `shipout.rs:2867` `match b16s1(p+8){1..=5}`.
- **S2 InteractionMode** enum (cold, ~12–15) — `consts.rs:35–37` Batch=0/Scroll=2/
  ErrorStop=3; derive `Ord` (code uses `< ERROR_STOP`); sites engine/format_io/
  main_control/errors.
- **S3 mem.rs allocators → `NodePtr<K>` returns** (14 allocators `mem.rs:266…784`,
  ~100 callers, 5–7 phased commits; low-fanout math-local ones first, then
  kern/penalty/glue/spec, then boxes). Kills the largest `.idx().0` cluster; store
  boundary keeps `.idx().0` (legit).
- **S4 free_node/delete_glue_ref** typed first param (~20 sites; depends on S3).
- **S5 hpack_emit_in/vpackage_emit_in** typed `list` param (~35–40, 2 commits;
  MemPtr/NodePtr<AnyNode> polymorphic).
- **S6 shipout.rs:2379** node dispatch → classify (HOT, corpus-gate; nested whatsit
  arm stays raw until S8).
- **S7 linebreak.rs disc walks** `:2799/2841/2890` → classify (HOTTEST line-break;
  corpus-gate; keep `is_char_node` fast path).
- **S8 WhatsitSubtype** enum (gated on #2).
- **S9 math.rs fetch loop `:4859`/`:4880` + mlist_to_hlist `:2752/2426`** → classify/
  classify_noad, fold out the double-expect, kill `assume_noad` (HOTTEST math, LAST,
  smallest commits, corpus-gate; preserve GARBAGE/xtx_ligature_present + LR/texxet
  side effects exactly).

Sequence: S1→S2→S3(phased)→S4→S5→S6→S7→S8→S9. Validation: dashboard 29/2/7/4 + unit 80
+ warnings baseline every commit; corpus (passed+diverged, NOXDV/NOLOG) for S6/S7/S9
(+ spot-check S3 box/glue commits).

## Execution findings (session 249, during S1/S2/S7)

- **DONE:** S1 PageboxType (c8f27c53), S2 InteractionMode (50e5e35f), S7a
  ignorable_protrusion→classify (9e137312), S7b the 3 disc break-width walks→classify
  (48c6bd39). All dashboard-green; S7 corpus-gated.
- **S6 (shipout.rs:2379) is a SKIP — not a clean target.** On inspection the `hlist_out`
  reswitch arms do raw memory manipulation + in-place retyping (`set_b16s1(p, KERN_NODE)`,
  the LR/texxet logic, `free_node`, glue-set fill-order) — they do NOT re-derive typed
  nodes. Wrapping the outer `match b16s1(p)` in `classify()` yields ZERO arm cleanup at
  MAX risk (hottest ship path) ⇒ types-for-types-sake, fails the bar. Leave it raw.
- **S9 target confirmed = the math FETCH loop `math.rs:4859`** (mlist_to_hlist reswitch).
  Genuine wins: (a) magic literals `0..=2 | 6 | 11|40 | 9` → named `AnyNode` variants;
  (b) the recon-flagged **double-expect** at 4880
  (`classify_math(..).expect().expect().width()`) collapses to `AnyNode::Math(m)=>m.width()`.
  The `6`(ligature)/`9`(math) arm bodies stay raw (GARBAGE splice, LR/texxet) — that is
  fine, they're not re-derives. HOTTEST math path ⇒ corpus-gate, smallest commit.
- **NoadType — DONE, folded into `NodeType` not a new enum (S9, 6d048e87).** The noad types
  are ALREADY `NodeType` variants (units.rs:172 `OrdNoad=16`..`RightNoad=31`) with a checked
  `from_raw` and a typed `NodePtr::<Noad>::noad_type()/set_noad_type()`; a separate `NoadType`
  would have duplicated that. Consolidated the ~30 duplicated `const ORD_NOAD/…/RIGHT_NOAD`
  (+ STYLE_NODE/CHOICE_NODE) across math/main_control/scan/output:
  * the two `mlist_to_hlist` dispatch matches + the script-scan match now `match
    NodeType::from_raw(b16s1(q)) { Ok(NodeType::Variant) => … }` — the derive-at-match-arm win;
  * `mlist_to_hlist`'s `r_type`/`t` spacing-machine locals stay numeric `u16` (the `0` sentinel
    collides with `HList=0`; the `offset_table[(type-ORD)]` indexing is inherently ORD-relative)
    but source every value from `NodeType::X.as_raw()` — enum as single source of truth, no magic
    literals, no `Option<NodeType>` refactor risk on the hottest path;
  * noad-type writes → `set_noad_type(NodeType::X)`; scattered `==`/range checks → `.as_raw()`.
  Behavior-preserving (every `X_NOAD` maps to the same u16; `from_raw` None folds into the
  existing catch-all). Corpus-gated.

## More findings (session 249, S9 investigation)

- **math.rs:4859 whole-match classify() DEFERRED — thornier than recon thought.** The
  GARBAGE landmine is a FALSE alarm (GARBAGE=4_999_987 ≥ hi_mem_min ⇒ caught by the char
  guard before the match; classify-at-match is memory-safe). BUT: (a) type 14 is EDGE_NODE
  *aliased to STYLE* — `classify()` returns `AnyNode::Style` for it, so matching an edge as
  `Style` would be MISLEADING (fails "readable"); (b) arms 9 and 10 are
  `classify_math/glue(..).expect().expect()` double-expects (a `Result<Option<_>>` artifact).
  A clean whole-match conversion needs an EDGE/STYLE-disambiguating classify + resolving the
  double-expects first. Leave until those exist.
- **The `classify_X(..).expect().expect()` double-expect (16 sites in math.rs) is a SYMPTOM,
  not an independent slice.** `classify_math/glue/whatsit/noad/box` return
  `Result<Option<NodePtr<K>>>` (Err=bad type, Ok(None)=other family, Ok(Some)=this family);
  callers that KNOW the family double-unwrap. The PROPER fix is propagating the typed handle
  from where the family is determined (the S3–S5 cascade) — a single-expect helper would just
  be "less expecting," still a use-site assert. Tests (node.rs) rely on the `Result<Option>`
  distinction (`is_err`/`Ok(None)`), so DON'T fold the signature. Fix these as the cascade lands.
- **WhatsitSubtype (S8) — DONE (7581a011 S8a/b, c635964b S8c, dfab391c S8d).** Added the
  `WhatsitSubtype` enum (node.rs; Open..Language 0–4, PdfSavePos 21, NativeWord..Pdf 40–44)
  with `from_raw` + `is_native_word()`/`has_dimensions()`/`is_io()`. Consolidated ALL ~40
  duplicated `const OPEN_NODE/NATIVE_WORD_NODE/…` across shipout/linebreak/math/main_control/
  packaging/pagebuilder/pic/ext: predicates → `from_raw(..).is_some_and(|s| …)`; whatsit-subtype
  dispatch matches → `match whatsit_subtype(..) { Some(Variant) => … }`; construction/write sites
  → `WhatsitSubtype::Variant as u16/i16`. The extension `_DE` COMMAND-code domain (do_extension)
  is left alone — that's a distinct `ExtensionCmd` enum candidate, NOT whatsit subtypes.

- **GroupCode — DONE (session 250, `bcef98a7`).** Typed the field `SaveStackState.cur_group:
  u8 → GroupCode` (`state.rs`; BottomLevel..MathLeft 0-16). Group codes are pure classification
  (equality + dispatch, NO arithmetic), so unlike the noad spacing machine the FIELD is typed,
  not just the boundaries — comparisons/dispatch arms are clean `GroupCode::Variant` equality,
  no `.as_raw()` casts. Folded ~75 duplicated `const SIMPLE_GROUP/…/MATH_LEFT_GROUP` (+ the
  `_MC`/`_TYPE` suffixed twins) across main_control/output/math/save/errors/engine/scan/
  pagebuilder. `new_save_level`/`push_math_in`/`scan_spec_in` params → GroupCode. The ONLY
  numeric touchpoints are the save-stack (de)serialize boundary (`as_raw()` on push,
  `from_raw().expect("valid group code")` on restore, in save.rs + output.rs). handle_right_brace
  + show_save_groups are now exhaustive over GroupCode → their unreachable `_ =>` arms removed
  (invalid-group upgraded from runtime `confusion` to compile-time exhaustiveness).
  **Method (reusable for field-type flips):** flip the field type first, let `cargo check`
  enumerate every mismatch as the checklist; fix leaf fn signatures + the (de)serialize
  boundary; bulk `X_CONST → Enum::Variant` by word-boundary regex (remove the `const` DEFS
  first). Watch suffixed twins (`_MC`/`_TYPE`) and benign unreachable-`_` warnings.

- **S3 (targeted subset) — typed-twin allocators DONE (session 250, `41029e94` + `97bb7a42`).**
  The user picked the CLEAN subset of the typed-pointer cascade (not the net-negative global
  flip). For each `new_*`/`get_node` site whose caller immediately re-derived a handle via
  `classify_*().expect().expect()` (or `classify_box_or_panic`), added a typed twin
  `alloc_*() -> Option<NodePtr<K>>` that returns the handle; `new_*` became a thin
  `.map_or(ALLOC_FAILED, |p| p.idx().0)` wrapper kept for the many i32-link callers (no
  anti-vein spray). Twins: `mem::alloc_glue`/`alloc_kern`/`alloc_null_box`/`alloc_rule`/
  `alloc_noad`/`alloc_margin_kern` + `math::alloc_glyph_whatsit` (the single audited glyph-
  whatsit `from_idx_unchecked` site). Removed ~7 double-expects + 2 `classify_box_or_panic`
  reclassifies across mlist fetch-loop / append_glue / append_kern / init_math / packaging
  overfull-rule / scan set_math_char / margin-kern insert. `new_margin_kern` deleted (callerless).
  **CRITICAL lesson (the `e_tex_aux` near-miss):** when a twin replaces `let x = new_*(); …
  classify(x)`, the raw `x` index is often reused far below (init_math stored it into
  `cur_list.e_tex_aux` ~60 lines later) — it COMPILES against an outer `mut x` holding a stale
  value. ALWAYS grep the raw index's FULL function scope (word-boundary) before dropping it;
  keep `x = handle.idx().0` wherever the raw index is later read.
- **Remaining double-expects are the thorny spots, not twin candidates:** the mlist first-pass
  dispatch arms (`classify_glue/kern/rule/choice` at ~2802–2860) and the fetch loop
  (~4885–4971) — both need the big `match classify()` restructure (AnyNode typed handles +
  noad_type() nesting + EDGE/STYLE care), the deferred S9 whole-match. Read-guard double-expects
  (glyph_nucleus) are awkward to convert. Leave until/unless that restructure is taken on.

## Session 251 — i32→NodePtr propagation cascade (goal: "all i32s are nodeptrs")

Propagating `AnyBox`/typed handles so raw i32 node indices disappear from use sites,
leaving `.idx().0` only at genuine i32 boundaries (mem-arena links, packer list-args,
`free_node`, nest-stack fields — i32 by the `memory_word` layout, NOT convertible).

- **DONE (corpus-gated):** after_math_in display hbox `b`→AnyBox (`26798d6a`);
  build_opentype_assembly + its two stack wrappers→AnyBox (`26798d6a`); make_op
  wrapper vbox via `alloc_null_vbox` twin (`287b50bd`); `ReboxResult::Done(i32)`→
  `Done(AnyBox)` + rebox param→AnyBox, killing rebox's double-expect (`3be6c78a`).
  classify_box_or_panic 18→12.
- **The remaining 12 classify_box_or_panic are DELIBERATE, not round-trips:**
  * nullable box handles (`after_math a`, `make_math_accent x`) that flow into i32-param
    fns (`app_display_in`) + i32 links — typing them `Option<AnyBox>` would spray
    `map_or(TEX_NULL, |x| x.idx().0)` sentinels (the net-negative anti-vein). Leave i32.
  * state-field provenance (`scanner.just_box`) — a typed field needs `Option<AnyBox>`
    (default None) + a guarded `.expect()` at the read; marginal, deferred.
  * raw-list-index conditionally-a-box (`clean_box_in:536`) — genuine classify.
- **NEXT big clean win = the mlist_to_hlist first-pass restructure** (`math.rs:2720`).
  Currently `match NodeType::from_raw(b16s1(q))` forces `classify_{choice,rule,glue,kern}()
  .expect().expect()` in the non-noad arms (~6 double-expects, incl. 3× classify_glue).
  Convert to `match st.mem.classify(MemIdx::new(q))`: `Ok(AnyNode::Noad(qn)) => match
  qn.noad_type() {…}` keeps noad granularity (qn replaces assume_noad(q); the 16 noad
  types are exhaustive, `_=>unreachable`), while `AnyNode::Glue(glue)/Kern/Rule/Choice`
  give typed handles directly. EDGE/STYLE aliasing (type 14) does NOT apply — math lists
  contain no edge nodes, so `AnyNode::Style` is correct here. HOTTEST math path: preserve
  the `continue 'reswitch` retype flow + r_type coupling exactly; dedicated corpus gate.
  Do the same for the second-pass match (`:2992`).

### Session 251 result — clean box/dispatch propagation COMPLETE

Commits `26798d6a`→`29813451` (all corpus-gated 637/2/0): after_math hbox, build_opentype
assembly + stack wrappers, make_op vbox, ReboxResult::Done, **mlist first-pass dispatch →
`match classify()` (typed AnyNode arms, −6 double-expects on the hottest path)**,
make_radical params, box_height_width param, make_math_accent `x` → AnyBox.
**classify_box_or_panic 18→10, double-expect chains 18→8.**

**The 10 remaining classify_box_or_panic are all DELIBERATE** (verified site-by-site):
the helper def; a raw-list-index conditionally-a-box (`clean_box:536`); the dynamic
hlist/vlist creation classify (`build_opentype:5193`); a state field (`just_box`); the
genuinely-nullable `after_math a` (real `a = TEX_NULL` branch → `Option` would spray
`map_or(TEX_NULL,…)` sentinels) and its `app_display_in` b_in coupling.
**LESSON: distinguish a *defensive-dead* null guard (var only ever assigned non-null,
e.g. make_math_accent `x` from clean_box → clean `AnyBox`, no Option) from a *genuine*
null branch (after_math `a` → stays i32).** Check for an actual `= TEX_NULL` assignment
before deciding a "nullable" box can't be typed.

The fetch loop (`~4835/4890/4920`) stays deferred: the `6 =>` arm splices `p = GARBAGE`
(≥ hi_mem_min) and re-loops, so a whole-match `classify()` hits the GARBAGE landmine; its
double-expects sit inside arms already dispatched on raw `b16s1`. Not cleanly convertible.

Everything past this point (nullable→Option sentinel spray, producer returns whose callers
do offset arithmetic, state-field provenance, the fetch loop) trades readability for
`.idx().0` sprawl — the net-negative anti-vein. The clean i32→NodePtr scope is complete.

## Status after session 250 (S8 + S9 + GroupCode + typed-twins done)

Constants→enums for node/whatsit/noad type codes AND group codes is now COMPLETE: no
`const *_NOAD`, `const *_NODE` whatsit-subtype, duplicated node-type literals, or
`const *_GROUP` remain — all route through `NodeType`/`WhatsitSubtype`/`GroupCode`/
`PageboxType`/`InteractionMode`. Remaining phase-2 work is the **S3–S5 typed-pointer cascade**
(typed allocator returns → typed `free_node`/`hpack`/`vpackage` params), which is also the
proper fix for the 16 math.rs `classify_X().expect().expect()` double-expects and the deferred
`math.rs` fetch-loop whole-match — BUT memory `typing-frontier-is-bucket-d-residue` flags the
global constructor-return flip as a **net-negative anti-vein** (sprays `.idx().0` at the i32
link/state-field stores), so it must be done as a targeted phased bundle, not a blanket flip.
`ExtensionCmd` enum (do_extension `_DE` codes) is DESCOPED like IfTest — a single well-named
dispatch, not a duplicated cluster, so low payoff.
