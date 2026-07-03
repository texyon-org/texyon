# State consolidation for engine cloning (post-flip)

**Goal (user, session 251).** Make the ENTIRE mutable engine state live inside one
`EngineState` so the engine can be **cloned mid-compilation** (for incremental-ish
recompilation via efficient tree/diff cloning of the memory arenas). This requires
**zero mutable global state outside `EngineState`**.

**Prerequisite DONE.** The §6 flip (`01d` §6.2) made `tt_run_engine` OWN an
`EngineState` and thread `&mut st`. That is exactly what unblocks folding the
`mod raw` scalars: the old reason they had to stay out (the thread-local `RefCell`
borrow bridge — see the now-OBSOLETE `mod-raw-is-intentional-not-debt` memory) is
gone. `with_state` 142 → 8 already.

## Recon verdict (session 251, 4-agent workflow, source-verified): ACHIEVABLE, no hard blocker

- **All 31 hot-path C globals are DEFINITION-ONLY** in `xetex-ini.c` (e.g.
  `int32_t cur_cs;`). **No C function reads or writes them** — the TeX engine logic
  is 100% Rust; the remaining compiled C files are stubs / thin ICU/font-library
  wrappers. The only C funcs Rust still calls are `xetex_ucnv_*` (pure ICU wrappers,
  no global access). ⇒ each scalar's STORAGE can move into `EngineState` and its
  C def be deleted, breaking no C logic.
- **Locators vs state.** The `with_state` thread-local pointer (`ENGINE_STATE_PTR`)
  and the 3 font FFI mirror pointers (`FONT_SIZE`/`FONT_AREA`/`FONT_LAYOUT_ENGINE`)
  are **locators, not state** — a clone need NOT capture them; re-install the pointer
  and call `refresh_font_ffi_mirrors` on the clone at resume. Only
  `ENGINE_STATE_FALLBACK` (`RefCell<EngineState>`) is a genuine second engine → delete.
- **One C++ callback needs mid-C-call state:** `font_feature_warning` (output.rs, fires
  inside the C `loadAATfont`). Rehome by threading an opaque `*mut EngineState` param
  through `loadAATfont` (xetex-macos.c) OR buffer the warning + flush after the C call.
  NOT a cloning blocker (the handle is a per-call locator).
- **Handle caches can't be memcpy-cloned** (double-free/close): `BRK_ITER` (ICU
  ubrk), `NORMALIZERS`/ICU converter (TECkit), `SAVED_MAPPING_NAME` (xstrdup),
  font engine/mapping `*mut c_void` Vecs. ⇒ **custom (non-derive) `Clone`** that
  resets these fields to null/unopened in the clone; they rebuild lazily &
  deterministically (keyed by locale/mapping name). `SAVED_MAPPING_NAME` →
  owned `CString`. **DESIGN DECISION NEEDED FROM USER:** for the loaded-font Vecs
  (`font_layout_engine`/`font_mapping`), does the clone SHARE them (fonts don't change;
  best perf, but tt_cleanup must not double-free — needs refcount/ownership rule) or
  RESET+reload (safe, slow)? This depends on the incremental-compile model.
- `EngineState` today derives only `Default`; **no `Drop` impls** anywhere (arenas
  freed explicitly in `tt_cleanup`), so a hand-written `Clone` is straightforward.

## Fold plan — dependency-ordered (each a small validatable commit; TRIP must stay 2/2 + corpus doc-count check per [[corpus-doccount-regression-check]])

1. **Clone scaffold.** Hand-written `Clone` for `EngineState` (derive where possible;
   reset handle-cache fields) + `resume()` (re-install locator, `refresh_font_ffi_mirrors`).
   Defines the clone target before state moves. *(gated on the handle-Vec design decision.)*
2. **Pre-run capacity scalars** (`stack_size`/`param_size`/`nest_size`/`save_size`/
   `buf_size`/`max_in_open`/`expand_depth`, 2–20 reads each) → `st.limits`. Constant
   after init; delete `tre_*` C defs. Trivial, proves the C-def-deletion loop.
3. **Handle/scratch caches** with custom-Clone reset: `BYTE_BUFFER`/`UTF32_BUF` → owned
   Vecs on `st.input`; `NORMALIZERS` → `st.io`; `SAVED_MAPPING_NAME` → `st.fonts`;
   `BRK_ITER`+`BRK_LOCALE_STR_NUM` → `st.ext`. Decide `STDIN_UFILE` (keep process-global
   documented, or move + exclude from clone).
4. **main_control scratch block** (`main_p/k/f/h/pp/ppp/s/i/j`, `cur_l/r/q`, `lig_stack`,
   `ligature_present`, `lft_hit`, `rt_hit`, `bchar`, `false_bchar`, `cancel_boundary`,
   `ins_disc`, `space_class`, `prev_class`) → new `st.main_loop`. Lives only in the
   already-st-threaded `main_control_in` inner loop (+ a few linebreak.rs sites). Early
   win, dead at token-boundary clone points. Delete C defs.
5. **`cur_cs`/`cur_tok`/`cur_chr`/`cur_cmd`/`cur_val`** → `st.scanner`, ONE scalar/commit
   (125/132/259/221/**625** sites). Convert only sites already in st-holding fns; leave
   the raw static for un-threaded callers until threaded. Volume-gated on the frontier.
6. **`cur_ptr`/`temp_ptr`** (82 sites each) → `st.scanner`/`st.mem`.
7. **`cur_input`(229)/`line`/`first`** → `st.input`; then **`cur_list`(560, largest)** →
   `st.input`/NestState LAST, after mode-stack callers threaded. Delete C defs.
8. **Rehome `font_feature_warning`** (opaque `*mut EngineState` through `loadAATfont`).
9. **Drain remaining globals.rs `with_state`** (`arith_error`/`tex_remainder`/
   `set_arith_error`, `name_of_file`/`name_length`, `hash_s1`) + scan.rs open_log site.
10. **Delete `ENGINE_STATE_FALLBACK` + `with_state`/`with_state_mut` + `set_engine_state_ptr`**
    once the drain hits zero; delete the empty `mod raw` blocks + orphaned xetex-ini.c defs.
11. **Add the clone API + validate:** clone `st` at a token boundary, resume both, diff
    `.xdv`/`.log`; TRIP 2/2; corpus doc-count check after steps 4–7.

**Config atoms (`CFG_*`)** are set pre-run (before any `EngineState` exists), copied into
`st.config` at `tt_run_engine` entry, never mutated mid-run ⇒ harmless to clones. For
strict zero-global, replace with a pre-run `Config` builder owned by the C-API shim; low
priority.

## Execution notes (session 251, from doing the capacity fold)

- **The fold loop, proven (commits `8efe108c`, `45585eb9`):** add field to the semantic
  `EngineState` sub-struct → set it in `tt_run_engine` (it also drives arena sizing +
  consistency checks) → convert reader sites to direct `st.<field>` → delete the accessor
  + `raw::` decl + `tre_*` extern + imports. **Rust-only** — leave the orphaned C
  `xetex-ini.c` global def (it's a harmless unused symbol; no C logic uses it, per recon).
  DONE: `stack_size`/`buf_size`/`param_size`/`max_in_open` → `st.input`,
  `expand_depth` → `st.scanner`. `mod raw` scalar count ~41 → 36.
- **The gate is real and pervasive (KEY):** a scalar folds only when ALL its sites hold
  `st`. The remaining hot-path scalars FAIL this because they're read from functions that
  take SUB-STRUCTS, not `st`:
  - `nest_size` — 1 site in a `*Abort::into_engine_abort(self)` (no st); `save_size` — ALL
    8 sites in `save.rs` fns taking `&mut SaveStackState`.
  - **main-loop scratch** (`main_p/k/f/…`, `cur_l/r/q`, `lig_stack`, flags — 381 sites):
    most in `main_control.rs` `st`-fns, BUT `app_space` takes `mem/eqtb/fonts` and
    `linebreak.rs` fns take `hyph/mem/fonts` — both access these scalars without `st`.
  - `cur_cs/cur_tok/cur_chr/cur_cmd/cur_val/cur_ptr/temp_ptr/cur_input/line/cur_list`
    (thousands of sites) are spread across scan/expand/main_control/linebreak/… with the
    same mix.
  ⇒ **"zero global state" = threading the ENTIRE remaining frontier**: every sub-struct-
  taking fn (`app_space`, the `linebreak.rs`/`save.rs` cores, …) must be converted to take
  `&mut EngineState` so its callers (which already hold `st`) pass `st`, THEN the scalar
  folds. This is the culmination of the whole port's threading effort — large & systematic,
  best driven per-function up the call tree, one scalar/cluster per validated commit.
- **Do NOT route a moved scalar through `with_state`** (would double-borrow `st`-holding
  callers). And the into_engine_abort `n:` count sites: defer to `into_history` via a
  no-count variant (the `PoolOverflow`/`StringOverflow`/`MainMemoryOverflow` pattern from
  `01d` §6.2) so those sites need no state.

**Headline:** zero external mutable state + mid-run cloning is achievable with no logical
blocker; the real cost is moving the hot-path scalar STORAGE into `EngineState` via direct
`st`-field access (gated on the threading frontier, NOT the `with_state` bridge — routing a
moved scalar through the bridge would double-borrow callers that already hold `&mut st`),
plus a custom non-memcpy `Clone` for the ICU/TECkit handle caches and rehoming the one
`font_feature_warning` C++ callback.

## Execution notes (session 249, continuation — the clean-scalar sweep)

Folded the scalars that DON'T hit the threading gate — the ones whose sites already
hold `st` (or a sub-struct their accessors uniformly hold). `mod raw` scalar count
**31 → 26** across five validated commits (each: dashboard 29/2/7/4, unit 80, warnings 36):

- `cur_val` → `st.scanner.cur_val` (625 sites; 2 borrow-hoist + 1 else-if restructure).
- `temp_ptr` → `st.mem.temp_ptr` (82 sites). Home is **MemState not ScannerState**: every
  non-`st` user (`new_skip_param` + its `append_to_vlist`/`insert_split_top_skip_before`
  callers that read it as an out-param, plus `reverse`/`just_reverse`) holds `&mut MemState`;
  two-phase borrows keep `mem.f(mem.temp_ptr, …)` legal.
- `cur_ptr` → `st.scanner.cur_ptr` (84 sites; sibling to `sa_root`). Promoted
  `new_index`/`sa_alloc_chain` to `&mut ScannerState`; threaded `scanner` into `do_marks`
  (5 callers, sa_root args hoisted).
- `first` → `st.input.first` (43 sites; sibling to `last`). All sites hold `st`.
- `nest_size` → `st.input.nest_size` (config constant; `NestOverflow` no-count variant).

**Two structural knots block the remaining 26 — map for next session:**

1. **`get_node` couples `line` + `cur_input` (+ synctex).** `get_node(mem: &mut MemState, s)`
   writes a synctex trailer for MEDIUM+ nodes reading BOTH `line()` and
   `cur_input_synctex_tag()` (mem.rs:250-251). `get_node` is called from hundreds of
   mem-only sites, so it cannot reach `st.input`. ⇒ `line` and `cur_input` (the InputState
   struct) are a **coupled fold**: either thread the two synctex scalars as params through
   every `get_node` call, or give `get_node` a `&InputClusterState` (cascade). `line` could
   be mechanically forced into `MemState` (get_node holds mem; only straggler = `new_save_level`
   which holds `save_stack` → pass `line` as a param, its 11 callers all hold `st`) — but that
   splits input scalars (`first`/`last` in `st.input`, `line` in `st.mem`) — a semantic wart.
   Defer as a deliberate coupled unit.

2. **`app_space` + `linebreak.rs` gate the main-loop scratch block** (`main_p/k/f/h/pp/ppp/s`,
   `bchar`, `false_bchar`, `space_class`, `prev_class` — main_control-only; plus `cur_l/r/q`,
   `lig_stack`, `ligature_present`, `lft_hit`, `rt_hit`, `cancel_boundary`, `ins_disc` which
   also leak to linebreak/engine). `app_space(mem, eqtb, fonts)` uses `main_p` etc. as local
   scratch but holds no `st`; `linebreak.rs` ligature cores hold `hyph/mem/fonts`. Folding into
   a `MainLoopState`/`LigatureState` sub-struct requires FIRST threading `app_space` and the
   linebreak ligature cores to `&mut EngineState` (their `st`-holding callers then pass `st`),
   OR proving the scratch is never shared across the app_space↔main_loop call boundary and
   localizing it. The latter needs care in TeX's ligature loop (highest-risk code) — thread
   first, localize never.

Remaining after this session: `cur_input` (InputState struct, knot 1), `line` (knot 1),
`cur_list` (~560 sites via its field accessors — big but mechanical, gated on whether every
`cur_list_*` site holds `st`), the main-loop block (knot 2), and the 3 font FFI mirror
pointers (locators, NOT state — leave them).

## Session 249 continued — the main-loop + ligature block (mod-raw 25→5)

Threading-then-fold cleared the entire main-loop/ligature scratch, the biggest
gated cluster. Pattern proven twice: **thread the one non-`st` scratch user to
`&mut EngineState` first (its callers already hold `st`), then fold.**

- **app_space** `(mem,eqtb,fonts)` → `(st)` [commit] — pure refactor, sole caller
  `app_space_in` passes `st`. Unblocked the 11 main_control-only scalars.
- **fold main-loop block** → new `MainLoopState`: `main_p/k/f/h/pp/ppp/s`, `bchar`,
  `false_bchar`, `space_class`, `prev_class` (25→14). **Regression caught+fixed:**
  `linebreak.rs` had its OWN `extern "C" { static mut main_f }` read bare by
  `do_locale_linebreaks_in` → native word nodes got font 0 (`bad native font flag`,
  graphite/otf/utf8 tests). Bound a local `main_f = st.mainloop.main_f`, deleted the
  rogue extern. **New rule: audit `grep -rn "static mut <name>"` across all src before
  every fold** (see the memory note).
- **reconstitute** `(mem,hyph,fonts,…)` → `(st,…)` [commit] — the ligature/kern core;
  body `mem./hyph./fonts.` → `st.*`, helper args → `&mut st.mem` etc., two `new_ligature`
  sites hoist `mem.b32s1(cur_q())` (E0502). Inner `update_right` also threaded (it sets
  cur_r). Callers = `goto_found` (holds st).
- **fold ligature block** → `MainLoopState`: `cur_l/r/q`, `lig_stack`,
  `ligature_present`, `lft_hit`, `rt_hit`, `cancel_boundary`, `ins_disc` (14→5).

**Remaining mod-raw = 5, only 2 are real state:**
- `cur_input` (InputState struct, 125 sites) — DEFERRED, same `get_node` synctex knot as
  `line`: `get_node` (mem-only) reads `cur_input.synctex_tag` for the node trailer. Can't
  split one field off the struct; options are (a) a `MemState` synctex-pos mirror updated
  on input push/pop (duplicate state, bad for clone), or (b) thread the synctex tag through
  `get_node` (huge). Fold cur_input into `st.input.cur_input` and solve the trailer read
  as its own step.
- `cur_list` (ListStateRecord, ~585 field-accessor sites across 15 files) — DEFERRED, big +
  delicate: needs a custom multi-accessor fold (`cur_list_head/tail/mode/prev_graf/
  e_tex_aux/mode_line/aux_b32s0/aux_b32s1` → `st.input.cur_list.<field>`; aux is a union →
  `unsafe`). Most sites hold `st` (main_control 304, math 83, linebreak reconstitute-free);
  audit stragglers first. Home = `st.input.cur_list` (sibling to `st.input.nest`;
  push_nest/pop_nest already hold st).
- `FONT_SIZE`/`FONT_AREA`/`FONT_LAYOUT_ENGINE` — FFI mirror *locators*, re-established on
  resume; NOT state, leave them.

## cur_list fold — scoped, ready to execute (attempted session 249, reverted)

Home = `st.input.cur_list` (added `ListStateRecord: Default` via MemoryWord's manual
Default; `push_nest`/`pop_nest` already hold st). Custom fold script exists in scratch
(`fold_cur_list.py`): maps the 8 field accessors to `st.input.cur_list.<field>`, the two
`aux` accessors to methods `aux_s0()/aux_s1()/set_aux_s0/set_aux_s1` (add these to
`impl ListStateRecord` — reading the `aux` union field is `unsafe`, writing is safe),
whole-record `cur_list()` → `st.input.cur_list`, `set_cur_list_record(x)` → assign.

**511 sites; 443 hold `st` (fold them directly). 60 stragglers in 17 fns** (hold
sub-structs, NOT st) — the gate. Fix each by adding a `cl: &mut ListStateRecord` param
(NARROW — `&mut st.input.cur_list` is disjoint from the `&mut st.mem`/`eqtb`/… these fns
already take, so no borrow conflict) and replacing `st.input.cur_list` → `cl`; callers
(mostly st-holding) pass `&mut st.input.cur_list`. The straggler fns:
- `mem::pop_nest_in` — already holds `InputClusterState`, so use `input.cur_list` (no param).
- `math.rs` (12): `flush_math`, `apply_math_limit_switch`, `append_radical_noad`,
  `append_choices_core`, `build_choices_finish`, `fin_mlist`, `prepare_sub_sup`,
  `start_fraction_noad`, `finish_fraction_rule`, `append_accent_noad`,
  `finish_left_right_group`, `append_inner_noad_for_right` — all hold `MemState` (±SaveStack).
- `linebreak::post_line_break_suffix`, `linebreak::char_pw_in`, `packaging::append_to_vlist`,
  `main_control::fix_language`, `mem::new_whatsit`.

Validate via dashboard (the main_f-class native-font bug WAS caught by tex-outputs, so the
dashboard is a strong gate) + corpus. Do the straggler-threading as prep refactor commits
(like app_space/reconstitute), THEN run the fold, per the [[fold-scalar-audit-rogue-externs]]
rule (audit first — cur_list has no rogue extern, checked).

### cur_list cascade WARNING (learned from the session-249 attempt)

The 18 DIRECT cur_list users are NOT the whole surface. The narrow `cl: &mut
ListStateRecord` param must thread through their non-`st` CALLERS too, and in
`linebreak.rs` that is a deep pass-through chain of big multi-substruct functions
that only FORWARD `cl` (they don't touch cur_list themselves):

    char_pw_in  ← total_pw            ← try_break ← line_break_try_break ← line_break_in(st)
    char_pw_in  ← insert_{left,right}_margin_kern ← post_line_break_prefix ← post_line_break_in(st)

So the real cl-needing set is ~25+ fns (the 18 + total_pw, try_break,
line_break_try_break, post_line_break_prefix, insert_left/right_margin_kern, + the
math.rs noad chain). Terminates at `line_break_in`/`post_line_break_in`/`after_math_in`/
main_control align fns, which hold `st` and pass `&mut st.input.cur_list`.

**Do this MANUALLY, compile-guided, one wave at a time — NOT with blanket regex.**
Blanket `s/name(/name(cl, /` mis-fires: it hits st-holding call sites (which must pass
`&mut st.input.cur_list`, not `cl`), double-inserts, and matches substrings
(`try_break(` inside `line_break_try_break(`). Correct recipe per wave: (1) add `cl`
first-param to the fns in this wave; (2) inside them, `&mut st.input.cur_list`→`cl` and
`st.input.cur_list`→`cl`; (3) at their forwarding call sites pass `cl`; (4) compile —
the next wave surfaces as "missing argument #1" at the callers; repeat until all callers
hold `st` (pass `&mut st.input.cur_list`). `pop_nest_in` is special: it already holds
`&mut input`, so use `input.cur_list` (NO cl param — passing `&mut st.input.cur_list`
alongside `&mut st.input` is an E0499 double-borrow). Infra (aux methods + Default derive
+ field) and `fold_cur_list.py` are ready in scratch/the reverted diff.

## cur_input is the EASIER remaining fold (revised — do it BEFORE cur_list)

Measured: 191 sites, but only **4 straggler sites in 2 real fns** (vs cur_list's 25+
cascade). Home = `st.input.cur_input` (folds the whole InputState struct; push/pop already
save/restore it via `input_stack`). Custom multi-field fold like cur_list but simpler:
`cur_input()` → `st.input.cur_input` (field reads `.loc/.state/…` follow for free since it's
Copy); `set_cur_input_X(v)` → `st.input.cur_input.X = v`; `set_cur_input(rec)` → assign.

**The only knot is `get_node`'s `cur_input_synctex_tag()`** (mem.rs:250) — the sole reader,
mem-only, stamps the synctex trailer. `synctex_tag` is a field of InputState (stack-saved),
so it can't leave the struct. Solution = a **MemState mirror** `mem.cur_synctex_tag` (same
spirit as folding `line`→mem): get_node reads `mem.cur_synctex_tag`; update the mirror at the
7 sites that change `cur_input.synctex_tag`, ALL of which hold `st`:
- 3 `set_cur_input_synctex_tag` (synctex.rs:640, input.rs:275, scan.rs:2931)
- 4 whole `set_cur_input` restores (input.rs:117,306; output.rs:4586,4795)
i.e. append `st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;` after each. (Mirror is
fine for cloning — both copies clone together; it's only a maintenance concern.)
Second straggler: `undump_upper_u16` (format_io, uses `cur_input.loc` as undump scratch) —
thread to `st` (its caller `load_fmt_file_in` holds st). All other 187 sites hold st.
scan.rs is 101 of them (hot tokenizer) — validate hard via dashboard (runs the tokenizer for
every test) + corpus. Audit rogue externs first ([[fold-scalar-audit-rogue-externs]]) — none
found for cur_input in a prior grep, but re-check.

## STATUS: mod-raw scalar fold COMPLETE (session 249) — mod-raw 31 → 3

Every hot-path `mod raw` C-extern scalar is now folded into `EngineState`. All engine
STATE lives in `EngineState`; a mid-run clone of `st` captures the whole engine.
The 3 remaining `static mut` are **NOT state** — they are `font_ffi_mirror::{FONT_SIZE,
FONT_AREA,FONT_LAYOUT_ENGINE}`, `*const` pointers INTO `st.fonts.{font_size,font_area,
font_layout_engine}` that the C++ OT-math FFI callbacks (`tt_get_font_*_entry`) read
borrow-free. `refresh_font_ffi_mirrors(&fonts)` re-points them at every fonts-Vec
realloc (init/undump/cleanup); a clone just re-runs it against the clone's `st.fonts`.
Corpus-validated across the whole fold (main-loop, ligature, cur_input, cur_list all
637/2 clean; the 2 divergences are the standing known FPs).

**Remaining for STRICT zero-global (the with_state bridge + fallback — separate phase):**
`with_state` occurrences 142 (flip) → 8. Those 8: 3 dead-but-`#[allow(dead_code)]`
accessor wrappers kept for unit tests (`arith_error`/`tex_remainder`/`hash_s1`), and 5
live (`set_arith_error` ← scaled_math ×6; `name_of_file`/`name_length` ← ext.rs FFI;
`output.rs:2929`; `scan.rs:2321`). Deleting `ENGINE_STATE_FALLBACK` (the pre-run/test
second `EngineState`) + `with_state`/`with_state_mut` + `ENGINE_STATE_PTR` requires
repointing the 5 live sites to direct `st` (gated on their FFI/setjmp-boundary callers,
per [[globals-wrappers-gated-on-setjmp-boundary]]) and reworking the 3 dead wrappers'
test usage. THEN phase 2 (the user's second ask): properly-typed pointers (kill the
residual `i32`/`b32.s1` handles).
