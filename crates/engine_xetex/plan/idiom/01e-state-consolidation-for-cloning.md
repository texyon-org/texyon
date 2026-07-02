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
