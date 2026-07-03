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
- **NoadType enum candidate** — the noad-type dispatches `math.rs:2426/2752/3018` (and
  `mlist_to_hlist`'s r_type spacing machine) match named noad consts (ORD_NOAD=16 … 
  RIGHT_NOAD) with raw-manipulation arms. Cleanest as a checked `NoadType` enum matched via
  `NodePtr::<Noad>::noad_type()` (classify to AnyNode::Noad, then noad_type()). Bigger slice
  (noad consts are used widely across math.rs); do after S9. This is the largest remaining
  "constants→enum" win.

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
- **Next clean win = WhatsitSubtype (S8)** — kills the repeated
  `sub == NATIVE_WORD_NODE || NATIVE_WORD_NODE_AT || GLYPH_NODE || PIC_NODE || PDF_NODE`
  predicate (readability) + the `<= 4` range filters (membership, not range).
