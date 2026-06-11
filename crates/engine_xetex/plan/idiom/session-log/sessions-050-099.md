# Archived Idiom Sessions 050-099

Older Stage 2 idiomatization log entries. Newest entries are at the top. Read `../SESSION_LOG.md` first; open this archive only when investigating older context.

## Idiom session 99 — 2026-05-31 — thread `flush_math` + `fin_mlist` to `&mut MemState`

**Type:** execution (workstream 01, Phase C — math noad family). **Three commits**: a preceding docs-refresh commit for the pre-existing WIP plan-doc/metrics changes, then two threading slices (`flush_math`, `fin_mlist`), one function + all call sites each. Dashboard **29/2/7/4** after each. Crate warnings did not regress.

### Slice 2 — `fin_mlist`

Threaded the math-mode mlist finalizer `fin_mlist` (`src/math.rs`) off the bridge. Recon: 5 call sites (`math.rs` ×4 — `build_choices`, `after_math`, `init_math` deletion paths; `scan.rs` ×1 in `its_all_over`/eqno handling); none inside a live `with_state*` closure; only stale `xetex-math.c` prototype/comment hits; no cleanup aliases, no cross-module redeclarations.

**Classification:** bucket **C**. `fin_mlist(mem: &mut MemState, p) -> Result<i32, &CStr>` does the mem-link work + raw `cur_list` head/tail/aux reads under one borrow. Two with_state hazards are deferred to `fin_mlist_global` after the borrow drops: the impossible corrupt-node `confusion` (the `q`-not-`LEFT_NOAD` arm → `Err(c"right")`), and the trailing `pop_nest` (with_state-bearing through the `nest` array). `pop_nest` only recycles `cur_list.head`, which the core never modifies, so the original ordering (`pop_nest` last) is preserved exactly. Local temporaries (`head_link`, `etex_link`) avoid `&mut mem` + `&mem` argument-evaluation conflicts.

**Bridged call sites:** 5 sites renamed to `fin_mlist_global`; `scan.rs`'s `use crate::math::fin_mlist` import updated. No bridge accessor lost its last caller; no new method surface; no fields moved → no newtype debt. Metrics flat (`unsafe fn` net 0: `fin_mlist` dropped `unsafe`, `fin_mlist_global` added it).

### Slice 1 — `flush_math`

**Slice:** threaded the small math-mode list-flusher `flush_math` (`src/math.rs`) off the bridge. It is a clean, fully-closeable leaf: it recycles the two current-list chains and resets `cur_list`. Recon: 3 call sites (`math.rs` ×2 in the math-symbol/extension-font-deleted error paths, `scan.rs` ×1 in `after_math`); none sits inside a live `with_state*` closure; only stale `xetex-math.c` prototype/comment hits for `flush_math`; no cleanup aliases, no cross-module redeclarations.

**Classification:** bucket **B/C**. `flush_math(mem: &mut MemState)` does its mem work directly and calls the already-threaded `crate::scan::flush_node_list(mem, …)` under the same borrow (so the two `flush_node_list_global` bridge calls become direct threaded calls). The `cur_list` head/tail/aux reads/writes go through the raw `cur_list` global (the deferred Phase-C set — safe under a live borrow per `01b` §4). The two impossible corrupt-node `confusion` arms inside `flush_node_list` propagate out as `Err`, and `flush_math_global` fires the cold `confusion` after the borrow drops (`01b` §5).

**Structure:** `flush_math` changed from `pub unsafe fn flush_math()` to `pub fn flush_math(mem: &mut MemState) -> Result<(), &'static CStr>`; added the bridge-down wrapper `flush_math_global`. All 3 legacy call sites renamed to `flush_math_global()`; `scan.rs`'s `use crate::math::flush_math` import updated to `flush_math_global`.

**Bridged call sites / deleted accessors:** 3 sites bridged to `flush_math_global`. No bridge accessor lost its last caller, so none deleted. No new state method surface (`MemState::b32s1`/`set_b32s1` already existed).

**Newtype debt:** none. No state fields moved.

**Metrics:** `#[no_mangle]` **328** (unchanged — `flush_math` was not exported). raw extern statics **38** unchanged. raw `unsafe {}` **174** unchanged. `unsafe fn` net flat (`flush_math` dropped `unsafe`; `flush_math_global` added it).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (100):** continue bounded Phase-C threading of small math/list leaf helpers (`flush_math` and `fin_mlist` are now done). Good candidates near the math noad family: the EXT_TAG extensible-delimiter box-building block inside `var_delimiter_global` (extract a threaded mem-only stacker in the session-81/83 style; keep `height_plus_depth`/`stack_into_box_global`/`effective_char` between borrows), or `build_choices`/`append_choices` mem work (mind the `unsave`/`push_math`/`scan_left_brace` scanner calls — bucket C/D, keep them outside live borrows). Keep `clean_box`, `vpackage`, `hpack`, OT-math calls, `font_is_ot_math_global`, and `first_script_glyph_global` outside live borrows. Do not take a whole high-count orchestrator (`mlist_to_hlist`, `hpack`, `vpackage`, `build_page`, `line_break`). Keep scanner-heavy `scan.rs` last; draft `02-modules.md` to READY before threading reaches it.

---

## Idiom session 98 — 2026-05-31 — collapse `make_scripts_global` branch bodies into helpers

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (`cargo check` reports **51** Rust warnings in this worktree).

**Slice:** larger but bounded structural cleanup inside `make_scripts_global` (`src/math.rs`) after session 97's sub/sup stack-tail extraction. Recon again found only stale `xetex-math.c` prototype/comment hits for `make_scripts`; no cleanup aliases; no cross-module extern/static redeclarations for the new helper slice; and the single live `make_scripts_global` caller in `mlist_to_hlist` is not inside a live `with_state*` closure. Candidate bridge accessors and math helpers still have other callers, so none were deleted.

**Classification:** `make_scripts_global` remains bucket **C/D**, but is now a small dispatcher. New `initial_script_shifts_global`, `finish_subscript_only_global`, `prepare_superscript_global`, and `finish_superscript_branch_global` are bucket **C/D** wrappers: they interleave short direct borrow islands with `hpack`, `clean_box` via `clean_script_box_global`, `first_script_glyph_global`, `maybe_attach_script_kern_global`, OT-math constants, and the existing `finish_subsup_stack_global`. `prepare_subscript_global` is also bucket **C/D** because its saved-font ordering differs between the subscript-only and sub/sup paths and must preserve `first_script_glyph_global` side effects exactly. `append_script_box` stays bucket **B** over `MemState`.

**Structure:** added `ScriptShifts`, `PreparedScript`, `initial_script_shifts_global`, `prepare_subscript_global`, `finish_subscript_only_global`, `prepare_superscript_global`, and `finish_superscript_branch_global`. `make_scripts_global` now reads the nucleus list head, computes initial shifts, and dispatches to either the subscript-only or superscript branch helper. The helpers preserve the original saved-font ordering: subscript-only saves after probing the subscript glyph; the sub/sup path saves before probing the subscript glyph. `hpack`, `clean_box`, `first_script_glyph_global`, `font_is_ot_math_global`, OT-math C calls, and `vpackage` remain outside live state borrows.

**Bridged call sites / deleted accessors:** no external call-site rename. No bridge accessor lost its last caller, so none deleted. No new state method surface.

**Newtype debt:** none. No state fields moved; helper input/return structs carry existing plain `i32`/`u16` values.

**Metrics:** `#[no_mangle]` unchanged. raw extern statics unchanged. raw `unsafe {}` unchanged. Current worktree counts after validation: `#[no_mangle]` **328**, raw extern statics **38**, raw `unsafe {}` **174**.

**Validation:** `cargo check -p tectonic_engine_xetex` -> 0 errors, 51 warnings; full dashboard -> **29 / 2 / 7 / 4**.

**For the next session (99):** `make_scripts_global` is structurally much smaller; good next math targets are another nearby box/list helper outside `make_scripts_global`, or a narrowly scoped pass to simplify the newly extracted helpers if a clearer shared parameter shape emerges. Do not take a whole high-count orchestrator (`mlist_to_hlist`, `hpack`, `vpackage`, `build_page`, `line_break`). Keep `clean_box`, `vpackage`, OT-math calls, `font_is_ot_math_global`, `first_script_glyph_global`, and `attach_hkern_to_new_hlist_global` outside live borrows unless explicitly classified/threaded.

---

## Idiom session 97 — 2026-05-31 — extract `make_scripts_global` sub/sup stack tail

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (`cargo check` reports **51** Rust warnings in this worktree).

**Slice:** bounded structural cleanup inside `make_scripts_global` (`src/math.rs`) for the remaining sub/sup stack tail: OT kern attachment results, inter-script kern allocation/linking, `vpackage`, final shift write, and nearby final box append. Recon found only stale `xetex-math.c` prototype/comment hits for `make_scripts`; `clean_box` is still the live Rust export already known from earlier sessions. No cleanup aliases; no cross-module extern/static redeclarations for the new helper slice; the single live `make_scripts_global` caller in `mlist_to_hlist` is not inside a live `with_state*` closure. Candidate bridge accessors (`mem_b32s1`, `set_mem_b32s1`, `mem_b16s*`, `math_x_height_global`, `default_rule_thickness_global`, scanner accessors) still have other callers, so none were deleted.

**Classification:** `make_scripts_global` remains bucket **C/D**. New `finish_subsup_stack_global` is bucket **C/D** because it calls with-state-bearing `maybe_attach_script_kern_global` and `crate::scan::vpackage` between direct borrow islands. New `append_script_box` is bucket **B** over `MemState`; `append_script_box_global` is the legacy wrapper. `maybe_attach_script_kern_global` remains bucket **C**; `new_kern` is bucket **B** with `ALLOC_FAILED` converted to `NodeRecycleAbort::Overflow` only after the borrow drops. `clean_box`, `first_script_glyph_global`, `font_is_ot_math_global`, OT-math C calls, and `vpackage` remain outside live borrows.

**Structure:** added `append_script_box`, `append_script_box_global`, `SubSupStackInput`, and `finish_subsup_stack_global`. The helper attaches sub/sup script kerns at wrapper level, computes the inter-script kern width under a read borrow, allocates and links the kern under one short `MemState` borrow, aborts on allocation failure after that borrow drops, calls `vpackage` after all borrows drop, writes the final box shift under a fresh borrow, and appends the packaged stack through the same final append helper used by the single-script paths.

**Bridged call sites / deleted accessors:** no external call-site rename. No bridge accessor lost its last caller, so none deleted. No new state method surface.

**Newtype debt:** none. No state fields moved; helper input fields are existing plain `i32`/`u16` values.

**Metrics:** `#[no_mangle]` unchanged. raw extern statics unchanged. raw `unsafe {}` unchanged. Current worktree counts after validation: `#[no_mangle]` **328**, raw extern statics **38**, raw `unsafe {}` **174**.

**Validation:** `cargo check -p tectonic_engine_xetex` -> 0 errors, 51 warnings; full dashboard -> **29 / 2 / 7 / 4**.

**For the next session (98):** continue bounded math noad cleanup adjacent to `make_scripts_global` or move to another small box/list helper near delimiter/script construction. Remaining `make_scripts_global` work is mostly local readability around the earlier shift/gap calculations and single-script append paths; avoid pulling in unrelated noad construction. Keep `clean_box`, `vpackage`, OT-math calls, `font_is_ot_math_global`, `first_script_glyph_global`, and `attach_hkern_to_new_hlist_global` outside live borrows unless explicitly classified/threaded. Do not take `mlist_to_hlist`, `hpack`, `vpackage`, `build_page`, or `line_break` as a whole session.

---

## Idiom session 96 — 2026-05-31 — extract `make_scripts_global` script-box prep helper

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (`cargo check` reports **51** Rust warnings in this worktree).

**Slice:** bounded structural cleanup inside `make_scripts_global` (`src/math.rs`) for the repeated `clean_box` + script-space/font-restore + min-shift calculation blocks. Recon found only stale `xetex-math.c` prototype/comment hits for `make_scripts`, `clean_box`, and the math-parameter prototypes; no cleanup aliases; no cross-module extern/static redeclarations except the live `clean_box` Rust export itself; the single live `make_scripts_global` caller in `mlist_to_hlist` is not inside a live `with_state*` closure. Candidate bridge accessors (`math_x_height_global`, `default_rule_thickness_global`, `mem_b16s2`, `set_mem_b16s2`) still have other callers, so none were deleted.

**Classification:** `make_scripts_global` remains bucket **C/D** as an orchestrator. New `clean_script_box_global` is bucket **C**: it calls with-state-bearing `clean_box` before any borrow, then runs one short direct state mutation/read for `add_script_space_and_restore_font` plus `sub1`/`sub2`/`sup1`/`sup2`/`sup3`. `add_script_space_and_restore_font` remains bucket **B** over direct `MemState`/`EqtbState`/`ScannerState`; the math-parameter helpers are bucket **B** over `FontState`/`EqtbState`. No with-state-bearing callee is reached under the helper borrow.

**Structure:** added `ScriptBoxAdjustment`, `ScriptBoxPrep`, and `clean_script_box_global`, then replaced the three repeated subscript/superscript box-prep blocks in `make_scripts_global`. `clean_box` stays outside live borrows; the helper returns the box node plus the relevant min-shift/clr value for the caller to merge into `shift_down` or `shift_up`. OT-math calls, `font_is_ot_math_global`, `maybe_attach_script_kern_global`, `new_kern`, and `vpackage` remain between short disjoint borrows.

**Bridged call sites / deleted accessors:** no external call-site rename. No bridge accessor lost its last caller, so none deleted. No new state method surface.

**Newtype debt:** none. No state fields moved; helper parameters and returned shifts are plain `i32`/`i16`.

**Metrics:** `#[no_mangle]` unchanged. raw extern statics unchanged. raw `unsafe {}` unchanged. Current worktree counts after validation: `#[no_mangle]` **328**, raw extern statics **38**, raw `unsafe {}` **174**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 51 warnings; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (97):** continue bounded math noad cleanup adjacent to `make_scripts_global`. Good next targets are the remaining sub/sup stack tail in `make_scripts_global` (kern-width allocation + `vpackage`/final append structure) or another small box/list helper near delimiter/script construction. Keep `clean_box`, `vpackage`, OT-math calls, `font_is_ot_math_global`, `first_script_glyph_global`, and `attach_hkern_to_new_hlist_global` outside live borrows unless explicitly classified/threaded. Do not take `mlist_to_hlist`, `hpack`, `vpackage`, `build_page`, or `line_break` as a whole session.

---

## Idiom session 95 — 2026-05-31 — extract `make_scripts_global` OT script-kern helper

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (`cargo check` reports **51** Rust warnings in this worktree).

**Slice:** bounded helper extraction inside `make_scripts_global` (`src/math.rs`) for the repeated OT-math script-kern calculation/attachment block. Recon found only stale `xetex-math.c` prototype/comment hits for `make_scripts`/`attach_hkern_to_new_hlist`; no cleanup aliases; no cross-module extern/static redeclarations; the single live `make_scripts_global` caller in `mlist_to_hlist` is not inside a live `with_state*` closure. Candidate bridge accessors (`cur_f`, `mem_b16s2`, `set_mem_b16s2`, `math_x_height_global`, `default_rule_thickness_global`) still have other callers, so none were deleted.

**Classification:** `make_scripts_global` remains bucket **C/D** as an orchestrator. New `maybe_attach_script_kern_global` is bucket **C**: it performs one short state read for `scanner.cur_f` plus `glyph_nucleus(&MemState, p)`; then drops the borrow before calling `font_is_ot_math_global`, the OT-math C `get_ot_math_kern`, and the with-state-bearing `attach_hkern_to_new_hlist_global`. `glyph_nucleus` is bucket **B** over `MemState`; `attach_hkern_to_new_hlist_global` remains the existing bucket-B bridge with `NodeRecycleAbort::Overflow` cold sentinel.

**Structure:** added `ScriptKernAttachment` and `maybe_attach_script_kern_global`, then replaced the repeated subscript/superscript kern blocks in `make_scripts_global`. The helper returns the possibly-updated noad hlist head, the kern amount when the caller needs it for sub/sup stacking, and whether the active font was OT math. `clean_box`, `vpackage`, `first_script_glyph_global`, `font_is_ot_math_global`, OT-math C calls, and `attach_hkern_to_new_hlist_global` all remain outside live state borrows.

**Bridged call sites / deleted accessors:** no external call-site rename. No bridge accessor lost its last caller, so none deleted. No new method surface.

**Newtype debt:** none. No state fields moved; helper parameters and returned kerns are plain `i32`/`u16`.

**Metrics:** `#[no_mangle]` unchanged by this slice. raw extern statics unchanged. raw `unsafe {}` unchanged. Current worktree counts after validation: `#[no_mangle]` **328**, raw extern statics **38**, raw `unsafe {}` **174** (`unsafe fn` grep: **646**).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 51 warnings; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (96):** continue bounded math noad cleanup adjacent to `make_scripts_global`, or take another small box/list helper near delimiter/script construction. Remaining higher-risk `make_scripts_global` work is structural cleanup around script box adjustment and the sub/sup stack; keep `clean_box`, `vpackage`, OT-math calls, `font_is_ot_math_global`, `first_script_glyph_global`, and `attach_hkern_to_new_hlist_global` outside live borrows unless explicitly classified/threaded. Do not take `mlist_to_hlist`, `hpack`, `vpackage`, `build_page`, or `line_break` as a whole session.

---

## Idiom session 94 — 2026-05-31 — split `make_scripts_global` borrow islands

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. Crate warnings did not regress (`cargo check` in this worktree reports **51** Rust warnings).

**Slice:** bounded deeper threading inside `make_scripts_global` (`src/math.rs`) around its `hpack`/`clean_box`/OT-math/`vpackage` regions. Recon found only stale xetex-math.c prototype/comment hits for `make_scripts`/`attach_hkern_to_new_hlist`, no cleanup aliases, no cross-module redeclarations, and the single live `make_scripts_global` caller in `mlist_to_hlist` is not inside a live `with_state*` closure.

**Classification:** `make_scripts_global` remains bucket **C/D** as an orchestrator. The threaded sub-blocks are bucket **B/C** islands: direct mem/eqtb/scanner reads and script math-parameter calculations are safe under short borrows; `hpack`, `clean_box`, `first_script_glyph_global`, `attach_hkern_to_new_hlist_global`, `vpackage`, `font_is_ot_math_global`, and all OT-math C calls stay between borrows. The local `new_kern` call in the sub/sup stack path uses the existing `ALLOC_FAILED` → `NodeRecycleAbort::Overflow` cold sentinel and aborts only after the borrow drops.

**Structure:** added a direct `MemState::b16s2` method plus local `is_char_node_in`, `glyph_nucleus`, and `add_script_space_and_restore_font` helpers. `make_scripts_global` now snapshots `p`, classifies glyph nuclei, restores `cur_f`, adds script space, computes `sup/sub` shifts, reads box dimensions, links the final script box, and allocates the inter-script kern through short disjoint state borrows. `clean_box`, OT kern/constant calls, and `vpackage` remain outside live borrows.

**Bridged call sites / deleted accessors:** no external call-site rename this session. Deleted now-dead math-parameter bridges `sup1_global`, `sup2_global`, `sup3_global`, `sub1_global`, `sub2_global`, `sup_drop_global`, and `sub_drop_global` after confirming their last caller was the rewritten `make_scripts_global`. `default_rule_thickness_global` and `math_x_height_global` remain live elsewhere.

**Newtype debt:** none. No state fields moved; `MemState::b16s2` mirrors an existing `u16` halfword accessor.

**Metrics:** `#[no_mangle]` unchanged in this slice. raw extern statics unchanged. raw `unsafe {}` unchanged by the slice after deleting only safe bridge wrappers. Current worktree counts after validation: `#[no_mangle]` **329**, raw extern statics **38**, raw `unsafe {}` **174**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (95):** continue bounded math noad cleanup. Remaining `make_scripts_global` work is the larger structural step: consider extracting a threaded helper for the repeated script-box adjustment/kern calculation while keeping `clean_box`, OT-math calls, `font_is_ot_math_global`, and `vpackage` outside live borrows. Another good small target is a box/list helper adjacent to delimiter/script construction. Do not take `mlist_to_hlist`, `hpack`, `vpackage`, `build_page`, or `line_break` as a whole session.

---

## Idiom session 93 — 2026-05-31 — thread math delimiter helpers

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. Crate warnings **78**.

**Slice:** `var_delimiter` + `make_left_right` (`src/math.rs`), with the small `height_plus_depth_global` bridge cleanup needed after the delimiter path stopped using it. Recon found only stale xetex-math.c prototypes/comments, no cleanup aliases, no cross-module redeclarations, and no live call site already inside a `with_state*` closure.

**Classification:** both targets are bucket **C/D**. `var_delimiter` scans delimiter variants through mem/font/eqtb data and interleaves OT-math/native glyph calls, `new_null_box`, TFM metric reads, `effective_char`, `char_box_global`, `stack_into_box_global`, `build_opentype_assembly`, and `native_glyph_vbox_global`. `make_left_right` mutates scanner style/size/mu, computes delimiter sizing from `axis_height` and eqtb parameters, then stores the delimiter node in mem. Callees were classified explicitly: `math_font`/`math_quad`/`axis_height`/`height_plus_depth` have threaded cores; mem/font/eqtb/scanner accessors are with-state bridges unless rewritten to direct state methods; `char_box_global`, `stack_into_box_global`, `build_opentype_assembly`, `native_glyph_vbox_global`, OT-math calls, and the initial remaining `mem_b16s2` bridge are kept outside live borrows.

**Structure:** dropped the two internal `#[no_mangle] extern "C"` exports. `make_left_right_global` now computes `math_quad`/`x_over_n` into locals, updates scanner style/size/mu under one short borrow, computes `axis_height`/delimiter parameters under a read borrow, calls `var_delimiter_global` with no live borrow, and stores the result under a final mem borrow. `var_delimiter_global` became a borrow-dropping orchestrator: delimiter-family and font-table reads use direct `FontState`/`EqtbState`/`MemState` methods in short borrows, OT variant/assembly/native calls run between borrows, and cold `new_null_box` allocation overflow returns through `node_recycle_abort(NodeRecycleAbort::Overflow)` only after the mem borrow drops.

**Bridged call sites:** `make_radical_global`, `make_fraction_global`, and `make_left_right_global` now call `var_delimiter_global`; `mlist_to_hlist` now calls `make_left_right_global`. `char_box_global`, `stack_into_box_global`, `build_opentype_assembly`, and `native_glyph_vbox_global` remain intentionally bridged because they build boxes or call native/OT code between delimiter borrow slices.

**Method surface / deleted accessors:** added `FontState::exten_base_entry`, mirroring the existing globals accessor. Deleted the now-dead `height_plus_depth_global` wrapper after confirming no remaining `src/*.rs` callers. `font_bc_entry`/`font_ec_entry`/`exten_base_entry` globals remain live outside this slice.

**Newtype debt:** none. No state fields moved; the new method exposes an existing plain `int32_t` font-table field.

**Metrics:** `#[no_mangle]` **330 → 328**. raw externs **40** unchanged. raw `unsafe {}` **176** unchanged. Crate warnings stayed **78**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 78 warnings; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (94):** continue math noad cleanup with another bounded helper block. Good candidates are deeper threading inside `make_scripts_global` around its `clean_box`/OT-math/vpackage regions, or a small remaining box/list helper adjacent to delimiter construction. Do not take a whole high-count orchestrator (`mlist_to_hlist`, `hpack`, `vpackage`, `build_page`, `line_break`) in one session; keep all with-state-bearing calls between short disjoint borrows.

---

## Idiom session 92 — 2026-05-31 — thread math script glyph discovery

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. Crate warnings **78**.

**Slice:** `make_scripts` + `first_script_glyph` (`src/math.rs`). Recon found only the stale xetex-math.c `make_scripts` prototype/comment, no cleanup aliases, no cross-module redeclarations, and the one live `make_scripts` caller in `mlist_to_hlist` was not inside a live `with_state*` closure.

**Classification:** both targets are bucket **C/D**. `first_script_glyph` walks mem, mutates scanner math style/size/mu/font scratch, calls `fetch_global`, computes `math_quad_global`/`x_over_n`, checks `font_is_ot_math_global`, and runs `new_native_character`/`real_get_native_glyph`. `make_scripts` interleaves mem reads/writes with `hpack`, `clean_box`, `attach_hkern_to_new_hlist_global`, OT-math kern/constant calls, `vpackage`, and threaded allocator wrappers. Callees were classified explicitly: mem/scanner/font/eqtb accessors are with-state bridge accessors unless threaded; `clean_box`, `hpack`, `vpackage`, `fetch_global`, `font_is_ot_math_global`, `math_quad_global`, `new_native_character`, `real_get_native_glyph`, and OT-math calls stay outside live borrows; `attach_hkern_to_new_hlist` and `new_kern`/`free_node` already have threaded cores with `NodeRecycleAbort` bridge handling.

**Structure:** dropped the internal `#[no_mangle] extern "C"` export from `make_scripts` and rewired the `mlist_to_hlist` call to `make_scripts_global`. Split `first_script_glyph` into a pure `first_script_glyph_step(&MemState, ...)` mem-walk/classification core plus a borrow-dropping `first_script_glyph_global` orchestrator. The wrapper precomputes `math_quad_global`/`x_over_n` results before scanner borrows, runs `fetch_global`, native character construction, native glyph lookup, and `font_is_ot_math_global` between borrows, and restores `cur_f`/`cur_style`/`cur_size`/`cur_mu` in a final short scanner borrow.

**Bridged call sites:** `make_scripts`'s one Rust caller now uses `make_scripts_global`; `first_script_glyph`'s three internal callers use `first_script_glyph_global`. No globals.rs accessor lost its last caller; scanner/font/math bridge accessors still have many remaining legacy math callers, so none were deleted.

**Newtype debt:** none. No state fields moved; the touched scanner fields and params were already tracked where semantic (`cur_style`, `cur_mu`) or are plain integer scratch.

**Metrics:** `#[no_mangle]` **331 → 330**. raw externs unchanged. raw `unsafe {}` **176** unchanged. Crate warnings stayed **78**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, 78 warnings; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (93):** continue math noad cleanup with `var_delimiter`/`make_left_right`, or take another bounded math helper block. `make_scripts_global` still contains legacy bridge accessors around `clean_box`/OT-math/vpackage regions; deeper threading should split those regions only with the with-state-bearing calls between short borrows. Do not take a whole high-count orchestrator.

---

## Idiom session 91 — 2026-05-31 — thread math accent prep + OT accent-position helper

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. Crate warnings **78**.

**Slice:** `make_math_accent` + `compute_ot_math_accent_pos` (`src/math.rs`). Both classified bucket **C/D**: they interleave `fetch_global`, native-character/glyph helpers, OT-math calls, `clean_box`, and scattered mem/font/scanner access. C recon found only stale xetex-math.c prototypes/comments, no cleanup aliases, no cross-module redeclarations, and no call site already inside a live `with_state*` closure.

**Structure:** dropped the two internal `#[no_mangle] extern "C"` exports. `compute_ot_math_accent_pos` is now a thin `compute_ot_math_accent_pos_global` dispatcher: a short `MemState` read classifies `MATH_CHAR` / recursive `SUB_MLIST` / infinity, then the wrapper performs `fetch_global`, `new_native_character`, `real_get_native_glyph`, and `get_ot_math_accent_pos` after the borrow drops; recursive args are carried as locals through a loop. `make_math_accent_global` now precomputes initial scanner/font/mem state in one short borrow, keeps `fetch_global` and `clean_box` outside borrows, and threads the TFM skew-kern scan through direct `FontState`/`ScannerState` access.

**Method surface:** added `FontState::width_base_entry` and `FontState::skew_char_entry`, mirroring existing globals accessors. Deleted the now-dead `globals::set_cur_i` bridge after confirming no remaining `src/*.rs` caller.

**Newtype debt:** none. No state fields moved; the new methods expose existing `int32_t` font-table fields, and the slice parameters/locals are plain `i32`.

**Metrics:** `#[no_mangle]` **333 → 331**. raw externs **40** unchanged. `unsafe fn` **647** unchanged. raw `unsafe {}` **176** unchanged. Crate warnings **79 → 78** after removing the orphaned `set_cur_i` bridge/imports.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (92):** continue the remaining math noad family with either `make_scripts` + `first_script_glyph` (large, hard bucket C/D: scanner/font/fetch/native-char interleaving inside a mem walk) or `var_delimiter`/`make_left_right`. Keep scanner-heavy `scan.rs` last; draft `02-modules.md` to READY before the big scanner decomposition.

---

## Idiom session 90 — 2026-05-31 — thread `make_op` (the last big noad-finishing helper)

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. The hard bucket-C/D `make_op` threaded as a series of short borrows.

**Slice:** **`make_op` alone** (`src/math.rs`), threaded to `make_op_global` (drops `#[no_mangle] extern "C"`, returns `delta: i32`). Deliberately *did not* add the handoff's suggested companions — `first_script_glyph` is itself a hard interleaved scanner/font/`fetch`/native-char function (its own bucket-C/D session) and `compute_ot_math_accent_pos` is a thin recursive dispatcher dominated by with_state-bearing calls with no extractable mem core. Either would raise RefCell-nest risk without making the slice *cleaner*; `make_op` alone is genuinely large.

**Structure (bucket C/D, the session 85/86/89 pattern scaled up):** the clean mem-write clusters run under short disjoint `with_state_mut` borrows, the with_state-bearing calls stay between them:
- **NORMAL→LIMITS latch** — one `(mem + scanner)` borrow.
- **Non-OT italic/list-tag delta block** — one disjoint `(mem + fonts + scanner)` borrow: `cur_f`/`cur_i`/`cur_style` are scanner fields, the `font_area`/`*_base`/`font_info` lookups are `FontState` reads, and `usingOpenType` is a pure C call safe under the borrow, so the whole guard + `cur_c`/`cur_i` updates + `delta` read run in one borrow (no bridge).
- **`clean_box(q+1, …)`** — with_state-bearing, between borrows.
- **OT-math glyph-variant/assembly region** (bucket **D**) — *left on bridge accessors* with no live borrow held, because the variant loop runs `measure_native_glyph` (with_state-bearing) between mem writes and `build_opentype_assembly` mid-block; threading it would need drop/reborrow inside the loop (deferred).
- **MATH_CHAR box finish** — one `(mem + fonts + eqtb)` borrow calling the threaded `axis_height(&fonts, &eqtb, size)` directly (`cur_size` precomputed).
- **LIMITS branch:** 3 `clean_box` (between borrows) → one `(mem)` borrow allocating the wrapper vbox via threaded `new_null_box` and computing its width into a local `w` → 3 `rebox_global(_, w)` (between borrows) → one big `(mem + fonts + eqtb + scanner)` borrow doing the entire tail: the threaded `big_op_spacing1..5(&fonts, &eqtb, size)` (precomputed into locals *before* the mem mutations to avoid overlapping reborrows), threaded `new_kern`/`free_node`, the `cur_f = save_f` scanner write, all direct mem writes; allocator exhaustion returns `NodeRecycleAbort::Overflow` so `node_recycle_abort` fires after the borrow drops (`01b` §5).

**Classification / recon (`01b` §4a, §11):** bucket **C/D**. C readers — only the header prototype (`xetex-math.c:56`) + a "translated to Rust" comment (`:71`); no body, no caller. No cleanup alias; no cross-module redeclaration; the lone call site (`mlist_to_hlist`, math.rs) is not inside a `with_state*` closure. Carried gotcha confirmed: `cur_f`/`cur_c`/`cur_i`, the `font_info`/`*_base` reads, `clean_box`, `rebox_global`, and the OT-math/measurement calls are all with_state-bearing — none called under a live borrow.

**Method surface:** added `FontState::italic_base_entry` (the handoff said it already existed — it did **not**; only the `italic_base` field was present). Reused existing `FontState`/`EqtbState`/`ScannerState`/`MemState` methods + the threaded `axis_height`/`big_op_spacing1..5`/`new_null_box`/`new_kern`/`free_node`.

**Bridged call sites:** the 1 `make_op` site (`mlist_to_hlist`) → `make_op_global`. **Deleted now-dead bridge accessors (§6):** all five `big_op_spacing{1,2,3,4,5}_global` wrappers (make_op was their last caller; the threaded cores are now called directly under the LIMITS-tail borrow) and the `globals::set_cur_c` setter (math.rs was its last caller; `fetch_core` and this slice write `scanner.cur_c` directly).

**Newtype debt:** none. No state fields moved (only added `FontState::italic_base_entry` over the existing field); `italic_base` is plain `int32_t *` (already recorded no-debt in session 39); `q`/`delta` are plain `i32` (checked).

**Metrics:** `#[no_mangle]` **334 → 333** (`make_op` dropped the export). raw externs **40** unchanged. `unsafe fn` **647** unchanged (`make_op` `unsafe extern "C" fn` → `make_op_global` `unsafe fn`, net flat; the deleted `big_op_spacing*_global`/`set_cur_c` were safe `fn`s). raw `unsafe {}` **176** unchanged. Crate warnings stayed **79**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** (the `otf_basic`/`graphite_basic` native-math + TRIP ligature/kern paths exercise `make_op` directly — green confirms parity, including the OT-math display-operator variant/assembly path left on the bridge). rust-analyzer again showed transient stale E0107/E0425 mid-edit; `cargo check` is ground truth.

**For the next session (91):** the big noad-finishing helpers are now all threaded (`make_ord`/`make_op`/`make_over`/`make_radical`/`make_vcenter`/`make_fraction`/`make_under`). Remaining math targets: `make_scripts` (large), `make_math_accent` (interleaves `fetch`/`compute_ot_math_accent_pos`/native-glyph), `first_script_glyph` + `compute_ot_math_accent_pos` (the two interleaved scanner/font/fetch/native helpers deferred this session), or `var_delimiter`/`make_left_right`. Also available: a pagebuilder/linebreak block. Remaining high-count functions are orchestrators (`hpack`/`vpackage`/`build_page`/`line_break`/`mlist_to_hlist`) — don't take one whole in one session. Keep scanner-heavy `scan.rs` last; draft `02-modules.md` to READY before the big scanner decomposition.

---

## Idiom session 89 — 2026-05-31 — thread the math `fetch` keystone + `make_ord` + leaf helpers

**Type:** execution (workstream 01, Phase C — math noad family). **One commit.** Dashboard **29/2/7/4**. A deliberately larger slice (4 functions, the keystone among them).

**Slice:** threaded `fetch` (the math-char setter the whole noad family funnels through), `make_ord`, `attach_hkern_to_new_hlist`, and `set_empty_math_field` (all `src/math.rs`).
- **`fetch`** (bucket **C**): split into a threaded `fetch_core(&mut MemState, &FontState, &EqtbState, &mut ScannerState, a) -> FetchOutcome` that sets `cur_c`/`cur_f`/`cur_i` directly from state, plus the `fetch_global` wrapper that emits the two cold deferred paths after the borrow drops — the undefined-math-font diagnostic + recoverable `error()` (`FetchOutcome::UndefinedFont`) and the missing-character `char_warning` (`FetchOutcome::CharMissing`). The core sets the `cur_*`/`mem[a]` fields before returning the signal; since `error()`/`char_warning` don't read them, the print-before-vs-after reorder is invisible (same idiom as sessions 85/86; `set_b32s1(a,EMPTY)` writes a union projection disjoint from the `b16.s1` family field the diagnostic prints — checked).
- **`make_ord`** (bucket **C/B**): the orchestrator `make_ord_global` keeps the early mem checks + the with_state-bearing `fetch_global` between borrows; each `\restart` iteration runs the threaded `make_ord_body(&mut MemState, &FontState, &mut ScannerState, q, p)` under one disjoint borrow doing the ligature/kern lookup + node assembly on direct field access. A `MakeOrdStep { Return, Restart, Overflow }` enum carries control back out so the loop re-runs `fetch_global` outside the borrow and a cold `new_kern`/`new_noad` exhaustion aborts via `node_recycle_abort(Overflow)` after it drops.
- **`attach_hkern_to_new_hlist`** (bucket **B**): pure mem over the threaded `new_kern`, `ALLOC_FAILED` → `NodeRecycleAbort::Overflow`, `_global` wrapper aborts after the borrow.
- **`set_empty_math_field`** (bucket **B**): trivial mem writes; `_global` wrapper.

**Classification / recon (`01b` §4a, §11):** key finding — `cur_f`/`cur_c`/`cur_i` and the `font_info`/`*_base` reads are **all with_state-bearing** (scanner/font clusters), so the noad helpers can't hold a mem borrow across the bare accessors; threading them to direct `&mut ScannerState`/`&FontState` removes the bridge entirely, with `fetch` the only remaining with_state-bearing call (kept between borrows). C readers: only stale prototypes/comments in the translated `xetex-math.c` stub (`fetch`/`make_ord`/`attach_hkern` have header-only `void …;` decls, no bodies/callers). No cleanup aliases, no cross-module redeclaration, no call site inside an existing `with_state*` closure.

**Method surface:** added `FontState::font_bc_entry` / `font_ec_entry` (mirroring the `globals` accessors); reused `MemState`/`EqtbState`/`ScannerState` fields + existing `FontState` reads + `math_font(eqtb, n)`.

**Bridged call sites:** `fetch` 6 external sites + the internal `make_ord` site → `fetch_global`; `make_ord` 1 site (`mlist_to_hlist`) → `make_ord_global`; `attach_hkern_to_new_hlist` 4 sites (`make_scripts`) → `_global`; `set_empty_math_field` 7 sites → `_global`. Deleted the now-dead `globals::null_character` getter (its last caller was the removed math.rs import; `set_null_character` stays — used by `ini.rs`).

**Newtype debt:** none. No state fields moved (only added `FontState` methods over existing fields); params (`a`/`q`/`delta`/`p`) are plain `i32` (checked).

**Metrics:** `#[no_mangle]` **337 → 334** (`fetch`/`make_ord`/`attach_hkern` dropped the export). raw externs **40** unchanged. `unsafe fn` **648 → 647** (`set_empty_math_field` de-`unsafe`'d with a safe wrapper; the three `_core`/`_body`+`_global` pairs are net-flat). raw `unsafe {}` **176** unchanged. Crate warnings stayed **79** (removed the orphaned `null_character` import + getter to hold the line).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** (the `otf_basic`/`graphite_basic` native-math + TRIP ligature/kern paths exercise `fetch`/`make_ord` directly — green confirms parity). rust-analyzer again showed transient stale E0425/E0061 during the rename; `cargo check` is ground truth.

**For the next session (90):** `make_op` is the remaining large noad helper — bucket **C/D**, interleaving `clean_box`/`rebox`/OT-math (`get_ot_math_*`, `build_opentype_assembly`)/`measure_native_glyph`/font reads with mem writes; thread it as a series of short borrows with all the with_state-bearing calls between them (now that `fetch`/`rebox`/`new_kern`/`axis_height`/`big_op_spacing*` are threaded, the pieces exist). Also available: `make_scripts` (large), `make_math_accent`, or a pagebuilder/linebreak block. Remaining high-count functions are orchestrators (`hpack`/`vpackage`/`build_page`/`line_break`/`mlist_to_hlist`) — don't take one whole in one session. Keep scanner-heavy `scan.rs` last; draft `02-modules.md` to READY before the big scanner decomposition.

---

## Idiom session 88 — 2026-05-31 — thread the pagebuilder leaf-helper block

**Type:** execution (workstream 01, Phase C — pagebuilder leaf helpers). **One commit.** Dashboard **29/2/7/4**.

**Slice:** threaded the two pagebuilder leaf helpers `freeze_page_specs` and `ensure_vbox` (`src/pagebuilder.rs`) onto narrow state borrows. Chose this clean, fully-closeable block over the session-87 handoff's designated `make_op`/`make_ord` (genuinely large bucket-C/D: each interleaves `fetch`/`clean_box`/`rebox`/OT-math/font reads with scattered mem writes throughout a loop — high regression risk for one session, deferred). `freeze_page_specs(eqtb: &EqtbState, shipout: &mut ShipoutState, s: u8)` writes the page-spec scratch directly from a disjoint `EqtbState` read; `ensure_vbox` split into a threaded predicate `ensure_vbox_is_hbox(&EqtbState, &MemState, n) -> bool` plus the `ensure_vbox_global` wrapper that runs the cold diagnostic + `box_error` only after the borrow drops.

**Classification (`01b` §4a):** `freeze_page_specs` is bucket **B** (pure `EqtbState` read + `ShipoutState` writes, zero `with_state`-bearing calls). `ensure_vbox` is bucket **C** (the cold `error_here_with_diagnostic`/`box_error` path is the with_state-bearing portion, deferred to the `_global` wrapper; the threaded core is just two `EqtbState`/`MemState` reads). Recon (§11 a–d): C readers grep hits only the comment header in the translated `xetex-pagebuilder.c` stub (no live C/C++ caller); no cleanup aliases; no cross-module redeclaration; no call site sat inside an existing `with_state*` closure.

**Method surface:** added `impl ShipoutState` (first methods on this sub-struct) with `set_page_contents`, `set_page_so_far`, `set_page_max_depth`, `set_least_page_cost`, mirroring the `globals`/pagebuilder accessors. Reused existing `EqtbState::b32s1` and `MemState::b16s1`.

**Bridged call sites:** 4 `build_page`/`fire_up` legacy call sites renamed to `_global` (`freeze_page_specs` ×2, `ensure_vbox` ×2). No `globals.rs` (or module-private) bridge accessor lost its last caller — `set_page_*`/`page_*` are still used across `build_page`, `fire_up`, `ini.rs`, `scan.rs`, `output.rs` — so nothing deleted.

**Newtype debt:** none. No state fields moved (only added methods over existing `ShipoutState` fields + read `EqtbState`); the `s: u8` / `n: u8` parameters are plain (checked).

**Metrics:** `#[no_mangle]` **337** unchanged (neither fn was exported). raw externs **40** unchanged. `unsafe fn` **649 → 648** (`freeze_page_specs` de-`unsafe`'d to a safe fn with a safe wrapper; `ensure_vbox` pair net flat). raw `unsafe {}` **176** unchanged. Crate warning count during `cargo check` stayed **79**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**. (rust-analyzer showed transient stale E0107/E0412 errors mid-edit; `cargo check` is ground truth and is clean.)

**For the next session (89):** good closeable targets remain a math noad helper (`make_op`/`make_ord` — split each `clean_box`/`rebox`/`fetch`/OT-math call out of a series of short borrows, the session 85/86 pattern scaled up; expect a large, careful session) or another linebreak/post-line-break block. Clean leaf helpers are nearly exhausted; the remaining high-call-count functions are orchestrators (`hpack`/`vpackage`/`build_page`/`line_break`/`mlist_to_hlist`) — do not take one whole in a single session. Keep scanner-heavy `scan.rs` last; draft `02-modules.md` to READY before the big scanner decomposition.

---

## Idiom session 87 — 2026-05-31 — thread the native text-node constructor cluster

**Type:** execution (workstream 01, Phase C — native-text node constructors). **One commit.** Dashboard **29/2/7/4**.

**Slice:** threaded the two functions that build XeTeX native-text nodes — `new_native_word_node` and `new_native_character` (both `scan.rs`) — onto narrow state borrows. `new_native_word_node(mem: &mut MemState, eqtb: &EqtbState, f, n)` is now pure-mem over the threaded `get_node` (using the `cnl_get_node!` macro / `NodeRecycleAbort::Overflow` §5 sentinel); `new_native_word_node_global` aborts after the borrow drops. For `new_native_character`, the mem allocation + UTF-16 payload fill (both the mapped-run and single-scalar branches) were extracted into a threaded `build_native_character(&mut MemState, &EqtbState, &FontState, …)` core run under one `with_state_mut` borrow; the orchestrator keeps the with_state-bearing work (string pool, TECkit `apply_mapping`, lost-char `char_warning`, `measure_native_node`) outside that borrow.

**Classification (`01b` §4a):** `new_native_word_node` bucket **B** (mem + one disjoint eqtb read). `new_native_character` bucket **C** (the `build_native_character` mem core is B; the surrounding string/mapping/output/measurement stays in the orchestrator). Recon: header prototypes only (`xetex-xetexd.h:905-906`), **no** C/C++ callers, no cleanup aliases, no cross-module redeclaration, and no call site sat inside an existing `with_state*` closure.

**Bridged call sites:** 11 external `new_native_word_node` sites renamed to `_global` (scan 6, linebreak 4, shipout 1) via a guarded `perl` pass; the internal call inside `build_native_character` and the wrapper call the threaded core directly. `new_native_character`'s 7 callers are unchanged — it remains the (inherently with_state-bearing) public orchestrator, not a transient `_global` wrapper, so it keeps its name.

**Newtype debt:** none. No state fields moved; only two `MemState` methods added (`set_b16s3`, `as_u16_ptr_mut`). The header typedefs (`internal_font_number`, `UnicodeScalar`) are function parameters, not moved fields (checked).

**Metrics:** `#[no_mangle]` **338 → 337** (dropped the `new_native_word_node` export). raw externs **40** unchanged. `unsafe fn` measured **649** (net flat: `new_native_word_node` lost `unsafe`, `new_native_word_node_global` added). raw `unsafe {}` **174 → 176** (the two explicit text-pointer write blocks now live in the safe `build_native_character` core). Crate warning count during `cargo check` stayed **79**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** (native-font cases `otf_basic`/`graphite_basic`/`file_encoding`/`unicode_file_name` all green, confirming the native-node restructure is behavior-identical).

**For the next session (88):** good closeable targets remain a math noad helper block (`make_op`/`make_ord` are large bucket-C — split each `clean_box`/`rebox`/`fetch`/OT-math call out of the borrow), a pagebuilder helper block, or another linebreak/post-line-break block. The clean leaf-helper slices are thinning out; remaining high-count functions are mostly orchestrators (`hpack`/`vpackage`/`build_page`/`line_break`/`mlist_to_hlist`). Keep scanner-heavy `scan.rs` paths last; draft `02-modules.md` before the big scanner decomposition.

---

## Idiom session 86 — 2026-05-31 — thread math noad finishing assembly

**Type:** execution (workstream 01, Phase C — math node-building helpers). **One commit.** Dashboard **29/2/7/4**.

**Slice:** threaded the noad-finishing assembly for `make_over`, `make_radical`, `make_vcenter`, and `make_fraction` to narrow `&mut MemState` cores. The `_global` wrappers keep `clean_box`, `vpackage`, `hpack`, `var_delimiter`, font-parameter reads, and delimiter construction outside live state borrows, then run only direct node linking/allocation under `MemState`.

**Classification (`01b` §4a):** `make_vcenter` is bucket **B** plus a cold `Confusion("vcenter")` sentinel. `make_over` / `make_radical` / `make_fraction` are bucket **C** borrow-dropping wrappers with bucket **B** node assembly inside. Recon found no call site inside an existing `with_state*` closure, no cleanup aliases, and no live C/C++ caller beyond stale `xetex-math.c` prototypes/comments.

**Bridged call sites:** 4 `mlist_to_hlist` noad-switch call sites now use `_global` wrappers. Deleted the now-dead `fraction_rule_global` and `overbar_global` bridge wrappers.

**Newtype debt:** none. No state fields moved; typedefs here are function parameters or already-tracked state fields, not newly moved fields.

**Metrics:** `#[no_mangle]` **342 → 338**. raw externs **40** unchanged. `unsafe fn` **651 → 650**. raw `unsafe {}` **173 → 174** (the safe `make_vcenter` core wraps `scaled_math::half` explicitly). Crate warning count during `cargo check` stayed **79**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (87):** good closeable targets remain the next math noad helper block (`make_op`, `make_ord`, or `make_math_accent` only if font/native calls can be cleanly split), a pagebuilder helper block, or another linebreak/post-line-break block. Keep scanner-heavy `scan.rs` paths last; draft `02-modules.md` before the big scanner decomposition.

---

## Idiom session 85 — 2026-05-31 — thread math glue/rebox node helpers

**Type:** execution (workstream 01, Phase C — math node-building/recycling helpers). **One commit.** Dashboard **29/2/7/4**.

**Slice:** threaded `math_glue`, `math_kern`, `rebox`, and the underline assembly around `make_under` to narrow `&mut MemState` cores. Legacy math callers now use `_global` wrappers; `rebox_global` drops the borrow around `hpack` / `effective_char_global`, and `make_under_global` drops it around `clean_box` / `vpackage`.

**Classification (`01b` §4a):** `math_glue` / `math_kern` are bucket **B** after precomputing scaled-math values outside the borrow; `rebox` and `make_under` are bucket **C** borrow-dropping wrappers. Recon found no call site inside an existing `with_state*` closure, no cleanup aliases, and no live C/C++ caller beyond stale `xetex-math.c` prototypes/comments.

**Bridged call sites:** 10 legacy math call sites moved to `_global` wrappers. No `globals.rs` bridge accessor lost its last caller.

**Newtype debt:** none. No state fields moved; semantic typedefs here (`scaled_t`, `small_number`, `internal_font_number`) are function parameters or existing field debt, not new moved fields.

**Metrics:** `#[no_mangle]` **346 → 342**. raw externs **40** unchanged. `unsafe fn` **651** unchanged. raw `unsafe {}` **171 → 173** (the scaled-math calls are now explicit before the borrow). Crate warning count during `cargo check` stayed **79**.

**Validation:** first full-dashboard attempt exposed a `RefCell` nest panic from `tex_remainder()`/`x_over_n()` inside `math_glue`'s borrow; fixed by precomputing scaled values before `with_state_mut`. Then `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (86):** good closeable targets remain a pagebuilder helper block, another math node-building block whose font/scanner/output calls can be precomputed or deferred, or a closed linebreak/post-line-break block. Keep scanner-heavy `scan.rs` paths last; draft `02-modules.md` before the big scanner decomposition.

---

## Idiom session 84 — 2026-05-31 — thread the math parameter helper cluster

**Type:** execution (workstream 01, Phase C — math font/parameter reads). **One commit.** Dashboard **29/2/7/4**.

**Slice:** threaded the shared math parameter helpers (`math_font`, `font_is_ot_math`, `math_sy_param`, `math_ext_param`, `sym_font_ok`, `ext_font_ok`) plus the 23 public math parameter helpers (`math_x_height` through `big_op_spacing5`) to narrow `&FontState`/`&EqtbState` reads. Legacy math callers now use `_global` bridges; the bridge wrappers pre-read `cur_size()` before borrowing for the mathex helpers.

**Classification (`01b` §4a):** bucket **B** accessor rewrite. Hot callees are direct state reads plus native-font C helpers (`isOpenTypeMathFont`, `get_native_mathsy_param`, `get_native_mathex_param`), not `with_state` calls. Recon found 79 Rust legacy call sites, no call site inside an existing `with_state*` closure, no cleanup aliases, and no live C/C++ caller beyond stale `xetex-math.c` source comments/definitions.

**Bridged call sites:** 79 legacy math call sites moved to `_global` wrappers. No `globals.rs` bridge accessor lost its last caller; font/eqtb bridge accessors are still used across other modules.

**Newtype debt:** none. No state fields moved; this slice only added `FontState` methods over already-moved fields. Semantic typedefs involved here (`internal_font_number`, `scaled_t`, `str_number`) are existing field debt already tracked from the font-state ownership sessions, not new moved fields.

**Metrics:** `#[no_mangle]` **369 → 346**. raw externs **40** unchanged. `unsafe fn` **680 → 651**. raw `unsafe {}` **168 → 171** (native-font C calls are now explicit inside safe threaded helpers). Crate warning count during `cargo check` stayed **79**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (85):** good closeable targets remain a pagebuilder node-building/recycling helper block or another linebreak/post-line-break block that bottoms out on threaded allocator/disposal helpers. Keep scanner-heavy `scan.rs` paths last; draft `02-modules.md` before the big scanner decomposition.

---

## Idiom session 83 — 2026-05-31 — thread the math native-glyph stack split

**Type:** execution (workstream 01, Phase C — math native-glyph helper split). **One commit.** Dashboard **29/2/7/4**.

**Slice:** split `math::stack_glyph_into_box` so allocation/initialization (`new_stack_glyph`) and final splicing (`stack_glyph_into_box`) run under narrow `&mut MemState` borrows, while the with-state-bearing `measure_native_glyph` call runs between borrows in `stack_glyph_into_box_global`. Also folded the adjacent native-glyph vbox fallback in `var_delimiter` into the same start/measure/finish pattern.

**Classification (`01b` §4a):** bucket **B** for direct `MemState` accessor rewrites and threaded `get_node`/`new_null_box`; bucket **C** for the borrow-dropping `measure_native_glyph` boundary. Recon found no cleanup aliases, no call sites inside an existing `with_state*` closure, and no live C/C++ callers (only stale `xetex-math.c` prototypes/comments).

**Bridged call sites:** 2 legacy `stack_glyph_into_box` sites in `build_opentype_assembly` now call `stack_glyph_into_box_global`; the native vbox fallback now calls `native_glyph_vbox_global`.

**Newtype debt:** none. No state fields moved; semantic typedefs here are function parameters (`internal_font_number`, `scaled_t`), not moved fields.

**Metrics:** `#[no_mangle]` **370 → 369**. raw externs **40** unchanged. `unsafe fn` **679 → 680** (one extra transitional wrapper net). raw `unsafe {}` **168** unchanged. Crate warning count during `cargo check` stayed **79**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (84):** good closeable targets are a pagebuilder helper block that bottoms out on threaded allocator/disposal, another closed linebreak block, or a larger math helper only if its scanner/output/error/font calls can be precomputed or deferred outside live state borrows. Keep the scanner hot path in `scan.rs` last; draft `02-modules.md` before the big scanner decomposition.

---

## Idiom session 82 — 2026-05-31 — thread font/effective-char plus linebreak ligature reconstruction

**Type:** execution (workstream 01, Phase C — font/read + linebreak helper cluster). **One commit.** Dashboard **29/2/7/4**.

**Slice (5 functions):** threaded `scan::effective_char` / `effective_char_info` to `&mut FontState`; `math::height_plus_depth` to `&mut FontState`; `linebreak::finite_shrink` to `&mut MemState`; and the larger `linebreak::reconstitute` to `&mut MemState` + `&mut HyphState` + `&mut FontState`.

**Classification (`01b` §4a):** `effective_char` / `effective_char_info` / `height_plus_depth` are bucket **B** font accessor rewrites (the TECkit mapping call uses the pre-read mapping pointer, no `with_state`). `finite_shrink` is bucket **C**: the cold no-shrink diagnostic stays in `finite_shrink_global`, then the threaded core mutates only `mem`. `reconstitute` is bucket **B** plus cold allocator sentinels: font/hyph bridge accessors became direct state reads, raw ligature scratch globals remain raw `mod raw`, and `get_avail` exhaustion returns **OverflowRunaway** while `new_lig_item`/`new_ligature`/`new_kern` exhaustion returns **Overflow**.

**Bridged call sites:** `effective_char` 17 external legacy sites + `effective_char_info` 1, `height_plus_depth` 4, `finite_shrink` 3, `reconstitute` 4. Deleted the now-dead `globals.rs` getters `xtx_ligature_present` and `init_lig`.

**Recon / safety checks (§11 + `01b`):** no cleanup aliases; no call site already inside a `with_state*` closure; no live C/C++ callers (only the stale `xetex-math.c` prototype/comment for `height_plus_depth`). `scan.rs` scanner-heavy callers stay bridged through `_global` wrappers.

**Newtype debt:** none. No state fields moved; only methods were added on existing `FontState` fields. Function parameters still use legacy `i32`/`u16` typedef erasure, so there is no `01a` moved-field row to add.

**Metrics:** `#[no_mangle]` **375 → 370**. raw externs **40** unchanged. `unsafe fn` measured **679**. raw `unsafe {}` **168** (+3: two `FontState` font-info union projections + the TECkit mapping call now explicit in `effective_char`). Crate warning count during `cargo check` **82 → 79**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (83):** good closeable targets are `stack_glyph_into_box` (two-borrow split around `measure_native_glyph`), a pagebuilder helper block that bottoms out on threaded mem/disposal, or another linebreak block whose hot callees can all take narrow state. Keep the scanner hot path in `scan.rs` last; draft `02-modules.md` before the big scanner decomposition.

---

## Idiom session 81 — 2026-05-31 — thread the math box-building helper cluster

**Type:** execution (workstream 01, Phase C — `math.rs` box-construction cluster). **One commit.** Dashboard **29/2/7/4**.

**Slice (5 functions):** the math box-building leaf helpers that now bottom out on the threaded allocator (`get_avail`/`new_rule`/`new_kern`/`new_spec`/`new_glue`/`new_null_box`), threaded to `&mut MemState`:
- `fraction_rule(mem, t) -> Result<i32, NodeRecycleAbort>` (math.rs) — bucket **B**, pure mem over threaded `new_rule`.
- `overbar(mem, b, k, t) -> Result<i32, NodeRecycleAbort>` (math.rs) — bucket **C**: builds the kern/rule/kern chain directly (threaded `new_kern`/`fraction_rule`) and returns its head; the `with_state`-bearing `vpackage` finish runs in `overbar_global` after the borrow drops.
- `char_box(mem, CharBoxInput) -> Result<i32, NodeRecycleAbort>` (math.rs) — bucket **C**: the `with_state`-bearing native-character allocation (`new_native_character`) and TFM font-table reads (`effective_char` + `font_info`/`*_base` reads) are precomputed into a `CharBoxInput` enum by `char_box_global` *before* the borrow ("precompute font reads outside the mem borrow"). Threaded core does only the mem allocation/field writes.
- `stack_into_box(mem, b, p)` (math.rs) — bucket **B**: links the precomputed character box; `stack_into_box_global` builds the box via `char_box_global` first.
- `stack_glue_into_box(mem, b, min, max) -> Result<(), NodeRecycleAbort>` (math.rs) — bucket **B**, pure mem over threaded `new_spec`/`new_glue`.

**Sentinel design:** reused session-80's shared `NodeRecycleAbort { Overflow, OverflowRunaway, Confusion }` (made `node_recycle_abort` `pub(crate)` in scan.rs). `new_*`/`get_node`-flavoured exhaustion → **Overflow**; `char_box`'s TFM `get_avail` exhaustion → **OverflowRunaway** (matches `get_avail_global`'s `runaway()`-then-`overflow`). Each `_global` wrapper drops the `with_state` borrow then runs `node_recycle_abort`.

**Allocation-order note (char_box native branch):** `new_native_character` is `with_state`-bearing, so it now runs (in the wrapper) *before* the box allocation — the reverse of the C order. Only absolute `mem` addresses differ; no `.xdv`/`.log` output or `mem`-image path observes them (TRIP uses TFM fonts only, so its native branch is never taken). Confirmed by the green dashboard, incl. the native-font tex-outputs cases (otf_*/graphite_*). The TFM branch preserves order (font reads have no allocation; `new_null_box` before `get_avail` as in C).

**Classification (`01b` §4a):** `fraction_rule`/`stack_into_box`/`stack_glue_into_box` bucket **B**; `overbar`/`char_box` bucket **C** (font/scanner calls precomputed or deferred via the shared sentinel).

**Bridged call sites:** fraction_rule 2 external (the 3rd is the now-direct call inside `overbar`), overbar 2, char_box 2 (the 3rd is now inside `stack_into_box_global`), stack_into_box 5, stack_glue_into_box 2 → **13 external** legacy sites repointed to `_global` wrappers, all in math.rs.

**Deliberately excluded:** `height_plus_depth` (pure font read, no mem — belongs to a future `&FontState` slice) and `stack_glyph_into_box` (`measure_native_glyph` is `with_state`-bearing mid-function → needs a fragmenting two-borrow split; deferred).

**Recon / safety checks (§11 + `01b`):** no extern statics; no `ini.rs` cleanup aliases; no C/C++ callers (only stale prototypes/`translated to Rust` comments in `xetex-math.c`); no call site already inside a `with_state*` closure. No globals.rs bridge accessor lost its last caller (font/mem accessors still used throughout); `new_rule_global`/`new_spec_global` still have many non-math callers.

**Newtype debt:** none. The C prototypes use `scaled_t`/`internal_font_number`, but those are function parameters, not moved state fields (the §4 rule applies to moved fields) — no semantic typedef moved, no `MemState` field added.

**Metrics:** `#[no_mangle]` **380 → 375** (the 5 functions dropped their export). raw externs **40** (unchanged). `unsafe fn` net **680** (5 bodies dropped `unsafe`, 5 `_global` wrappers added it). raw `unsafe {}` **165** (unchanged — the threaded bodies and wrappers have no explicit `unsafe` blocks).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (82):** with the box-building leaves threaded, the remaining math layout (`make_fraction`/`make_op`/`make_ord`/`var_delimiter`/`rebox`) is still bucket-C/D — gated on the `with_state`-heavy `hpack`/`vpack`/`vpackage`/`mlist_to_hlist`/`new_native_character`, none of which is threaded yet. Cleaner next targets: thread `effective_char`/`height_plus_depth` as a small `&mut FontState` slice (would unblock the TFM font-read paths across math and the main loop), thread `stack_glyph_into_box` with the two-borrow `measure_native_glyph` split, or climb a `pagebuilder.rs`/`linebreak.rs` block. Keep `scan.rs`'s scanner hot path last; draft `02-modules.md` to READY before the big `scan.rs` decomposition.

---

## Idiom session 80 — 2026-05-31 — thread the node-copy / mark-recycle / page-prune cluster

**Type:** execution (workstream 01, Phase C — `mem`/`scan` node-recycling cluster). **One commit.** Dashboard **29/2/7/4**.

**Slice (large, 4 functions):** the natural counterpart to session 79's node-*disposal* cluster — the node-*copy* / mark-tree-recycle / page-top-prune routines, all of which now bottom out on the threaded allocator (`get_avail`/`get_node`/`new_skip_param`) and disposal (`free_node`/`flush_node_list`/`delete_token_ref`/`copy_native_glyph_info`) primitives:
- `copy_native_glyph_info(mem: &mut MemState, src, dest)` (mem.rs) — bucket **B**, mem-only + a raw `xmalloc`/`copy_nonoverlapping` of the native-glyph buffer. Added `copy_native_glyph_info_global`.
- `copy_node_list(mem: &mut MemState, p) -> Result<i32, NodeRecycleAbort>` (scan.rs) — the big recursive list copy. Rewrote the whole body onto `MemState` methods, recurses on itself (recursive-call args precomputed into locals to avoid overlapping the `&mut mem` reborrow), allocates via two local macros (`cnl_get_node!`/`cnl_get_avail!`) that return the `01b` §5 sentinel on exhaustion, and the two impossible corrupt-node `confusion` arms return the sentinel too.
- `do_marks(mem: &mut MemState, a, l, q) -> bool` (scan.rs) — recursive mark-class-tree recycler. Pure mem + threaded `free_node`/`delete_token_ref`; `cur_ptr` is a raw `mod raw` global (safe under the borrow). **No cold path.** Free-function calls taking a `mem`-derived arg precompute it (no two-phase borrow for free fns).
- `prune_page_top(mem: &mut MemState, scanner: &mut ScannerState, eqtb: &EqtbState, p, s) -> Result<i32, NodeRecycleAbort>` (scan.rs) — disjoint triple borrow: `disc_ptr` is on `ScannerState`, `new_skip_param` needs `&EqtbState`, `temp_ptr` is a raw global. Frees discarded nodes via threaded `flush_node_list`.

**Sentinel design:** one shared `NodeRecycleAbort { Overflow, OverflowRunaway, Confusion(&CStr) }` enum + a `node_recycle_abort(e) -> !` helper. `get_avail` failure maps to **OverflowRunaway** (the `get_avail_global` path runs `runaway()` *before* `overflow`); `get_node`/`new_skip_param` failure to **Overflow**; the corrupt-node arms to **Confusion**. Each `_global` wrapper drops the `with_state` borrow, then runs `node_recycle_abort`.

**Classification (`01b` §4a):** all four bucket **B** + the cold abort edges as bucket **C** via the shared sentinel.

**Bridged call sites:** copy_node_list 3 (math 1, scan 2), copy_native_glyph_info 1 (scan; the in-`copy_node_list` site is now a pass-through), do_marks 4 (ini 1, pagebuilder 2, scan 1), prune_page_top 2 (pagebuilder 1, scan 1) → **10 external** legacy sites repointed to `_global` wrappers; pagebuilder's `do_marks`/`prune_page_top` imports repointed. Internally, ~15 `get_node_global`/`get_avail_global` + several `free_node_global`/`delete_token_ref_global`/`flush_node_list_global`/`new_skip_param_global` calls became direct threaded pass-throughs.

**Recon / safety checks (§11 + `01b`):** no extern statics; no `ini.rs` cleanup aliases; no C/C++ callers (only stale `.h` prototypes for `copy_node_list`/`copy_native_glyph_info`); no call site already inside a `with_state*` closure. `disc_ptr`/`set_disc_ptr` global accessors still have many other callers — none deleted.

**Newtype debt:** none. Params/returns are plain `i32` / `bool` / `*mut c_void` (native-glyph pointer) / `&'static CStr` (confusion message) — no semantic typedef moved.

**Metrics:** `#[no_mangle]` **381 → 380** (`copy_node_list`). raw externs **40** (unchanged). `unsafe fn` **682 → 680** (the four bodies dropped `unsafe`; two safe + two unsafe `_global` wrappers + `node_recycle_abort`). raw `unsafe {}` text-search **163 → 165** (+2: the audited `xmalloc`/`copy_nonoverlapping` block in `copy_native_glyph_info`).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (81):** the node-copy and node-disposal clusters are both threaded, so more of `pagebuilder.rs` (`fire_up`/`build_page`) and `math.rs` box-stacking now bottom out cleanly. Math box helpers (`char_box`/`stack_*_into_box`/`height_plus_depth`/`fraction_rule`) are bucket-C (font-table reads must be precomputed outside the mem borrow). Keep `scan.rs`'s scanner hot path last; draft `02-modules.md` to READY before the big `scan.rs` decomposition.

---

## Idiom session 79 — 2026-05-31 — thread the node-disposal cluster (flush_node_list + delete_glue_ref/delete_token_ref)

**Type:** execution (workstream 01, Phase C — `mem`/`scan` node-disposal cluster). **One commit.** Dashboard **29/2/7/4**.

**Slice (large):** threaded the three fundamental node-recycling routines that bottom out the dependency graph for `post_line_break`, `sa_destroy`, math, pagebuilder, etc.:
- `delete_token_ref(mem: &mut MemState, p)` and `delete_glue_ref(mem: &mut MemState, p)` (mem.rs) — bucket **B**, mem-only, call the already-threaded `flush_list`/`free_node`. Added safe `_global` wrappers.
- `flush_node_list(mem: &mut MemState, p) -> Result<(), &'static CStr>` (scan.rs) — the big recursive list-disposal. Rewrote the whole body onto `MemState` methods (added `b16s3`/`ptr`/`set_ptr`), recurses on itself, calls the threaded `free_node`/`delete_glue_ref`/`delete_token_ref`. Recursive-call arguments are precomputed into locals to avoid overlapping the `&mut mem` reborrow. The two impossible corrupt-node `confusion()` arms return the `01b` §5 sentinel `Err(msg)`; `flush_node_list_global` fires `confusion()` after the borrow drops. The `libc::free` of the native-glyph buffer stays, wrapped in an audited `unsafe` block (the fn is now safe).
- Bridged **116 legacy call sites** across 7 files (scan, math, linebreak, save, pagebuilder, shipout, ini) to the `_global` wrappers via a scripted rename, with the 4 bare-name `use` imports repointed.

**Classification (`01b` §4a):** `delete_*` bucket **B**; `flush_node_list` bucket **B** + the cold `confusion` edge handled as bucket **C** via the `Result` sentinel.

**Recon / safety checks (§11 + `01b`):**
- no extern statics, no `ini.rs` cleanup aliases.
- no C/C++ callers (grep empty for all three).
- no call site already inside a `with_state*` closure (the one save.rs hit is a different threaded helper, not these).

**Newtype debt:** none. Params/returns are plain `i32` / `*mut c_void` (native-glyph pointer) / `&'static CStr` (the confusion message) — no semantic typedef moved.

**Metrics:** `#[no_mangle]` unchanged at **381** (none were exported). raw externs **40**. `unsafe fn` **684 → 682** (both `delete_*` dropped `unsafe`, safe wrappers; `flush_node_list`↔`flush_node_list_global` net zero). raw `unsafe {}` blocks **160 → 163** (+3: the three audited union projections in the new `MemState` methods + the explicit `libc::free` block; the old bodies' unsafe was implicit in `unsafe fn`). Net crate warnings **90 → 89**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4**.

**For the next session (80):** `flush_node_list` is now threaded, which unblocks more of `post_line_break` and `sa_destroy`. Good next targets: continue climbing `math.rs`/`pagebuilder.rs` node helpers that now bottom out on the threaded disposal cluster, or thread another `linebreak.rs` block. Keep `scan.rs`'s scanner hot path last; draft `02-modules.md` to READY before the big `scan.rs` decomposition.

---

## Idiom session 78 — 2026-05-31 — thread linebreak LR-stack update

**Type:** execution (workstream 01, Phase C — fifth `linebreak.rs` slice). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread `update_lr_stack`, the `post_line_break` TeXXeT LR-stack maintenance helper:
- `update_lr_stack` now takes `&mut MemState`, touches the `mem` arena directly through `MemState` methods (`b16s0`/`b32s0`/`b32s1`/`set_b32s0`/`set_b32s1` + the `avail` scalar), and allocates through the threaded `get_avail` instead of `get_avail_global`.
- On arena exhaustion it returns the `01b` §5 sentinel (`true`) rather than aborting under the borrow.
- New `update_lr_stack_global` bridge-down wrapper runs the helper under one `with_state_mut` borrow, then performs the cold `runaway()`+`overflow()` abort after the borrow drops (mirroring `get_avail_global`).
- The three legacy callers in `post_line_break` now use the `_global` wrapper.

**Classification (`01b` §4a):** bucket **B** (accessor-to-direct-state rewrite) plus the bucket-**C** cold allocation-overflow edge deferred to the wrapper.

**Recon / safety checks (§11 + `01b`):**
- helper, not a global — no extern statics or `ini.rs` cleanup aliases.
- no C/C++ callers (grep empty); Rust-internal helper only.
- all three call sites are in `post_line_break`; none sit inside a `with_state*` closure.

**Newtype debt:** none. No state fields added; params/return are plain `i32`/`&mut i32`/`bool`.

**Metrics:** `#[no_mangle]` exports unchanged at **381**. `globals.rs` raw externs unchanged at **40**. `unsafe fn` net unchanged at **684** (`update_lr_stack` dropped `unsafe`; `update_lr_stack_global` added it). Raw `unsafe { ... }` text-search count unchanged at **160**. Thin accessors unchanged (no bridge accessor lost its last caller).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** passed perfectly.

**For the next session (79):** `post_line_break` itself is now closer to threadable — its remaining bridge calls are mem/eqtb/scanner accessors plus `cur_list_*`. Consider threading another closed `post_line_break` sub-block, or begin classifying `post_line_break` as a larger bucket-C/D target. Keep `scan.rs` last and draft `02-modules.md` to READY before scanner-heavy work.

---

## Idiom session 77 — 2026-05-31 — thread linebreak margin-kern protrusion insertion

**Type:** execution (workstream 01, Phase C — fourth `linebreak.rs` slice). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread the post-line-break margin-kern protrusion insertion blocks:
- Added direct target-selection helpers for the right and left margin-kern paths. They take `&mut MemState` + `&mut LinebreakState`, walk mem directly, and call the direct protrusion scanners.
- Added borrow-dropping insertion wrappers that compute protrusion targets under state, run `char_pw` after the borrow drops, then reborrow briefly to allocate/link the margin kern directly with `new_margin_kern`.
- `post_line_break` now delegates both protrusion insertion blocks to these wrappers, keeping `char_pw` and cold `overflow` outside live state borrows.
- Removed the now-dead `find_protchar_left_global` / `find_protchar_right_global` wrappers, the old `prev_rightmost` import, and the dead `new_margin_kern_global` bridge.

**Classification (`01b` §4a):** bucket **C**. Direct target selection and margin-kern linking are B-style accessor rewrites, but `char_pw` still crosses linebreak/font/scanner bridges and allocation overflow must remain borrow-dropping.

**Recon / safety checks (§11 + `01b`):**
- no extern statics or `ini.rs` cleanup aliases.
- no executable C/C++ callers; post-slice C-reference grep is empty for the removed wrappers.
- no cross-module extern/static/link-name redeclarations.
- no call sites already inside a `with_state*` closure.

**Newtype debt:** none. No state fields added; parameters and return values are existing plain TeX integer node pointers (`i32`) and booleans.

**Metrics:** `#[no_mangle]` exports unchanged at **381**. `globals.rs` raw externs unchanged at **40**. `unsafe fn` **685 → 684** by removing `new_margin_kern_global` (two protrusion `_global` wrappers were replaced by two new borrow-dropping helpers). Raw `unsafe { ... }` text-search count unchanged at **160**. Thin accessors unchanged at **817**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** passed perfectly.

**For the next session (78):** continue `linebreak.rs`, likely with another closed `post_line_break` helper such as the LR-stack update path. Keep `char_pw` outside live state borrows until scanner/font/string state is threaded. Keep `scan.rs` last and draft `02-modules.md` to READY before scanner-heavy work.

---

## Idiom session 76 — 2026-05-31 — thread linebreak total_pw

**Type:** execution (workstream 01, Phase C — third `linebreak.rs` slice). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread `total_pw`, the line-break protrusion-width node selector:
- `total_pw` now takes `&mut MemState` and `&mut LinebreakState`, reads `first_p` / `global_prev_p` directly, walks mem through `MemState`, and calls the direct protrusion scanners.
- `total_pw_global` preserves the legacy caller, defers cold hlist-stack `pdf_error`, and runs `char_pw` after the state borrow drops because `char_pw` still crosses linebreak/font/scanner bridges.
- The sole legacy caller in `try_break` now uses `total_pw_global`.
- Removed the now-dead `first_p` / `global_prev_p` bridge getters; setters remain because `line_break` still updates those fields.

**Classification (`01b` §4a):** bucket **C**. The search itself is direct accessor rewrite, but `char_pw` must stay outside the borrow until scanner/font/string state is threaded.

**Recon / safety checks (§11 + `01b`):**
- no extern statics or `ini.rs` cleanup aliases.
- no executable C/C++ callers; post-slice C-reference grep is empty.
- no cross-module extern/static/link-name redeclarations beyond the Rust definition before the slice.
- no call site already inside a `with_state*` closure.

**Newtype debt:** none. No state fields added; parameters and returned nodes are existing plain TeX integer node pointers (`i32`).

**Metrics:** `#[no_mangle]` exports **382 → 381** (`total_pw`). `globals.rs` raw externs unchanged at **40**. `unsafe fn` unchanged at **685** (`total_pw` export became a direct safe helper plus unsafe `_global` wrapper). Raw `unsafe { ... }` text-search count unchanged at **160**. Thin accessors **819 → 817** by removing `first_p` / `global_prev_p` getters.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** passed perfectly.

**For the next session (77):** continue `linebreak.rs` with another closed protrusion/linebreak helper. Good candidates are the nearby post-line-break protrusion insertion blocks that already use `find_protchar_*_global` plus `char_pw`; keep `char_pw` outside live state borrows until scanner/font/string state is threaded. Keep `scan.rs` last and draft `02-modules.md` to READY before scanner-heavy work.

---

## Idiom session 75 — 2026-05-31 — thread linebreak protrusion scanners

**Type:** execution (workstream 01, Phase C — second `linebreak.rs` slice). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread the protrusion scanner helper pair:
- `find_protchar_left` and `find_protchar_right` now take `&mut MemState` and `&mut LinebreakState`, read the mem arena directly, and use the direct `push_node` / `pop_node` hlist-stack helpers.
- Added direct local predicates for the threaded path (`*_in` variants) plus a direct `prev_rightmost_in` helper, avoiding nested `with_state` calls under the borrow.
- `find_protchar_left_global` / `find_protchar_right_global` preserve legacy callers and run the cold hlist-stack failure `pdf_error` after the state borrow drops.
- Removed the now-dead `push_node_global` / `pop_node_global`, old bridged protrusion predicates, and the last hlist-stack bridge accessors (`hlist_stack_level` / `set_hlist_stack_level`).

**Classification (`01b` §4a):** main rewrite is bucket **B** (accessor-to-direct-state rewrite). The impossible hlist stack overflow/underflow edge is bucket **C** and is deferred to the `_global` wrappers.

**Recon / safety checks (§11 + `01b`):**
- no extern statics or `ini.rs` cleanup aliases.
- no executable C/C++ callers; post-slice C-reference grep is empty.
- no cross-module extern/static/link-name redeclarations beyond the Rust definitions themselves before the slice.

**Newtype debt:** none. No state fields added; parameters/returns are existing plain TeX integer node pointers (`i32`).

**Metrics:** `#[no_mangle]` exports **384 → 382** (`find_protchar_left` / `find_protchar_right`). `globals.rs` raw externs unchanged at **40**. `unsafe fn` **687 → 685** (two unsafe exports became direct safe helpers; wrappers are unsafe but two now-dead hlist-stack wrappers were removed). Raw `unsafe { ... }` text-search count unchanged at **160**. Thin accessors **821 → 819** by removing the last two hlist-stack bridge accessors.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** passed perfectly.

**For the next session (76):** continue `linebreak.rs`, likely by threading `total_pw` now that the protrusion scanners are direct. Watch `char_pw`: if it still crosses scanner/string/font state through bridges, either keep `total_pw` as a wrapper-only slice or choose another closed linebreak helper first. Keep `scan.rs` last and draft `02-modules.md` to READY before scanner-heavy work.

---

## Idiom session 74 — 2026-05-31 — thread linebreak hlist stack helpers

**Type:** execution (workstream 01, Phase C — first `linebreak.rs` slice). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread the closed hlist-stack helper pair:
- `push_node` now takes `&mut LinebreakState` and mutates `hlist_stack` / `hlist_stack_level` directly.
- `pop_node` now takes `&mut LinebreakState`, decrements `hlist_stack_level` directly, and returns the stack entry.
- `push_node_global` / `pop_node_global` preserve legacy callers and run the cold `pdf_error` paths after the state borrow drops.
- `find_protchar_left` / `find_protchar_right` now call the `_global` wrappers; the now-dead `hlist_stack_entry` / `set_hlist_stack_entry` bridge accessors were removed.

**Classification (`01b` §4a):** direct stack mutation is bucket **B** (accessor rewrite). The overflow/underflow `pdf_error` edge makes the wrappers bucket **C** so no error/longjmp path runs under a live `with_state_mut` borrow.

**Recon / safety checks (§11 + `01b`):**
- no extern statics or `ini.rs` cleanup aliases.
- no executable C/C++ callers; post-slice C-reference grep is empty.
- no cross-module extern/static/link-name redeclarations beyond the Rust definitions themselves before the slice.

**Newtype debt:** none. No state fields added; parameters/return values are existing plain TeX integer node pointers (`i32`), and `hlist_stack_level` was already recorded when `LinebreakState` moved.

**Metrics:** `#[no_mangle]` exports **386 → 384** (`push_node` / `pop_node`). `globals.rs` raw externs unchanged at **40**. `unsafe fn` unchanged at **687** (two unsafe exports became two unsafe bridge wrappers). Raw `unsafe { ... }` text-search count unchanged at **160**. Thin accessors **823 → 821** by removing the two hlist-stack entry accessors.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** passed perfectly.

**For the next session (75):** continue `linebreak.rs` with another closed non-scanner Phase-C cluster. Good candidates are small helpers around protrusion/linebreak state before attempting large routines like `line_break`; classify first and keep `scan.rs` last. Draft `02-modules.md` to READY before scanner-heavy work.

---

## Idiom session 73 — 2026-05-31 — thread DVI font emission path

**Type:** execution (workstream 01, Phase C — fifth `shipout.rs` slice). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread the DVI font-definition / font-selection path:
- `dvi_font_def` is now the direct `&mut ShipoutState` byte emitter over a prebuilt byte slice; `dvi_font_def_global` prepares the bytes outside the state borrow, then emits with the same continuation-aware `DviFlush` loop as `dvi_four` / `dvi_two`.
- `dvi_native_font_def` now prepares its byte vector (`DEFINE_NATIVE_FONT`, the four-byte font id, and the `make_font_def` / `xdv_buffer` payload) without holding `ShipoutState`.
- The non-native font-definition path builds the exact old byte sequence from font/string-pool state before borrowing `ShipoutState`.
- `ensure_dvi_font` now emits only the font-selection bytes under `&mut ShipoutState`; `ensure_dvi_font_global` handles the font-used check, possible `dvi_font_def_global`, deferred flushes, and updates `dvi_f` after emission is fully complete (including the edge case where the last selection byte triggers a flush).

**Classification (`01b` §4a):** bucket **C/D manual**. The byte emission itself is direct `&mut ShipoutState`, but font/string-pool reads and `make_font_def` stay outside the borrow. Legacy call-site renames to `dvi_font_def_global` / `ensure_dvi_font_global` were bucket **A** after the wrappers were established.

**Recon / safety checks (§11 + `01b`):**
- no extern statics or `ini.rs` cleanup aliases.
- C side has only a stale `dvi_font_def` prototype and translated-to-Rust comments in `xetex/xetex-shipout.c`; no executable C/C++ callers.
- no cross-module extern/static/link-name redeclarations.
- no call sites already inside a `with_state*` closure.

**Newtype debt:** none. No state fields added; parameters are existing plain `internal_font_number` representation (`i32`) and byte slices.

**Metrics:** `#[no_mangle]` exports **387 → 386** (`dvi_font_def`). `globals.rs` raw externs unchanged at **40**. `unsafe fn` unchanged at **687**. Raw `unsafe { ... }` text-search count unchanged at **160**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** passed perfectly.

**For the next session (74):** continue past the now-threaded shipout DVI font path toward the next non-scanner Phase-C subsystem. Prefer `math.rs` / `linebreak.rs` next as STATUS says; keep `scan.rs` last and draft `02-modules.md` to READY before scanner-heavy work. If taking another `shipout.rs` slice first, pick a closed DVI command-emission helper and keep output/error calls as deferred actions.

---

## Idiom session 72 — 2026-05-31 — thread DVI byte formatters

**Type:** execution (workstream 01, Phase C — fourth `shipout.rs` slice). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread `dvi_four` / `dvi_two`, the pure DVI byte formatters:
- `dvi_four` and `dvi_two` now take `&mut ShipoutState` and emit through the direct threaded `dvi_out`.
- Because `dvi_out` may require an immediate buffer flush before later bytes overwrite the flushed half-buffer, the direct helpers stop at the first `DviFlush` and return a small continuation (`DviEmit { next, flush }`).
- `dvi_four_global` / `dvi_two_global` bridge legacy callers by borrowing state, emitting until a flush is needed, dropping the borrow, performing the flush / fatal abort, then resuming at `next`.
- `dvi_four_bytes` preserves the old signed four-byte conversion exactly; `dvi_two` keeps the old high/low byte order.

**Classification (`01b` §4a):** `dvi_four` / `dvi_two` are bucket **C**, not plain A, because `dvi_out` can trigger file I/O / fatal overflow. Legacy call-site rewrites to `_global` were bucket **A** mechanical after the wrapper shape was established.

**Recon / safety checks (§11 + `01b`):**
- no extern statics or `ini.rs` cleanup aliases.
- C side has only stale prototypes/comments in `xetex/xetex-shipout.c`; no executable C/C++ callers. Left them in place, matching sessions 68–71 scope.
- no cross-module extern/static/link-name redeclarations.
- no call sites already inside a `with_state*` closure.

**Deferred:** `dvi_font_def` / `dvi_native_font_def` / `ensure_dvi_font` stay legacy this commit. They call font/string-pool accessors and should be handled as the next manual `shipout.rs` slice, using the new continuation-aware byte helpers.

**Newtype debt:** none. No state fields added; parameters are existing plain `int32_t` / `UTF16_code` representations.

**Metrics:** `#[no_mangle]` exports **389 → 387** (`dvi_four`, `dvi_two`). `globals.rs` raw externs unchanged at **40**. `unsafe fn` unchanged at **687** (two unsafe externs became two unsafe bridge wrappers). Raw `unsafe { ... }` text-search count unchanged at **160**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, no new `shipout.rs` warnings (the lone `xmalloc` unused warning pre-dates this slice); full dashboard → **29 / 2 / 7 / 4** passed perfectly.

**For the next session (73):** continue `shipout.rs` with `dvi_font_def` / `dvi_native_font_def` / `ensure_dvi_font`. Classify carefully: these are not pure byte formatters because they read font/string-pool state and call `make_font_def`; avoid holding `ShipoutState` across those bridged reads. Keep `math.rs`/`linebreak.rs` after the remaining shipout DVI-font path; `scan.rs` remains last. Draft `02-modules.md` to READY before scanner-heavy work.

---

## Idiom session 71 — 2026-05-31 — thread shipout DVI byte writer

**Type:** execution (workstream 01, Phase C — third `shipout.rs` slice). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread the buffered DVI byte-writer trio to direct `&mut ShipoutState`:
- `dvi_out` now takes `&mut ShipoutState`, writes the byte and bumps `dvi_ptr` directly, and returns an `Option<DviFlush>` describing any deferred buffer flush instead of doing I/O under the borrow.
- `dvi_swap` now takes `&mut ShipoutState`, does the swap bookkeeping directly, and returns `DviFlush::Write(a, b)` (the byte range to flush) or `DviFlush::Fatal` (the >2GB DVI-length overflow). State updates are reordered before the deferred flush — sound because `dvi_limit/offset/ptr/gone` do not affect the bytes written.
- `dvi_out`'s `dvi_out_global` bridge-down wrapper runs `dvi_out` under `with_state_mut`, then after the borrow drops performs the flush (`write_to_dvi_global`) or the cold `fatal_error` (§5 sentinel).
- `write_to_dvi` became the borrow-dropping `write_to_dvi_global` wrapper: it grabs `(dvi_file, dvi_buf ptr)` under a short `with_state` borrow and does the FFI write / `_tt_abort` outside it — preserving the pre-thread "never hold a borrow across the abort longjmp" structure (the longjmp would leak the `RefCell` guard). It threads to a direct `&ShipoutState` helper once its callers (`finalize_dvi_file`, a threaded `dvi_swap`) thread.

**Classification (`01b` §4a):** `write_to_dvi` / `dvi_swap` / `dvi_out` = bucket **C** (output/error/abort under a live borrow → borrow-dropping wrappers + a deferred-action return). The ~76 legacy call sites (`dvi_out` ×71, `write_to_dvi` ×4 incl. the two `finalize_dvi_file` sites, `dvi_swap`'s single internal site) = bucket **A** mechanical rename to `dvi_out_global` / `write_to_dvi_global` (one `perl` word-boundary pass; verified no comment/string false positives first).

**Recon / safety checks (§11):**
- (a) no extern *statics* — these are functions; no state fields added → no newtype debt.
- (b) no `ini.rs` cleanup aliases.
- (c) C side: only stale prototypes (`xetex-shipout.c:37-40`) + "translated to Rust" comments; `finalize_dvi_file` (the reason for `write_to_dvi`'s old `#[no_mangle]`) is itself already Rust, so no executable C/C++ callers. Left the stale prototypes in place (out of commit scope; session-68/69 precedent).
- (d) no cross-module extern/static/link-name redeclarations.
- All `dvi_out`/`dvi_four`/`dvi_two` call sites live in `shipout.rs` (single file).

**Work done:**
1. `shipout.rs`: rewrote `dvi_out`/`dvi_swap` as direct `&mut ShipoutState` helpers returning the `DviFlush` action; added `dvi_out_global`; converted `write_to_dvi` → `write_to_dvi_global`.
2. Renamed all legacy callers to `dvi_out_global` / `write_to_dvi_global`.
3. Deleted the now-dead `dvi_gone()` getter (its sole caller was `dvi_swap`; movement code reads `shipout.dvi_gone` directly, and `set_dvi_gone` is still used at init).

**Deferred:** `dvi_four` / `dvi_two` / `dvi_font_def` stay legacy this commit (they call `dvi_out_global`); they are the natural next slice as pure pass-throughs once threaded to forward `&mut ShipoutState` and propagate the `DviFlush` short-circuit.

**Newtype debt:** none. No state fields added; parameters are existing plain `eight_bits`/`int32_t` (`u8`/`i32`).

**Metrics:** `#[no_mangle]` exports **391 → 389** (−2: `write_to_dvi`, `dvi_out`; the raw `grep` also drops a stale comment that contained the literal `#[no_mangle]` text, so the grep reads 392 → 389). `globals.rs` raw externs unchanged at **40**. `unsafe fn` **688 → 687** (three unsafe defs `write_to_dvi`/`dvi_swap`/`dvi_out` → two unsafe wrappers + two safe threaded helpers). Raw `unsafe { ... }` text-search count unchanged at **160**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, no new warnings in `shipout.rs` (the lone `xmalloc` unused warning pre-dates this slice on HEAD); full dashboard → **29 / 2 / 7 / 4** passed perfectly.

**For the next session (72):** continue `shipout.rs` with the pure DVI byte formatters `dvi_four` / `dvi_two` (and then `dvi_font_def` / `dvi_native_font_def` / `ensure_dvi_font`) as pass-through callers of the now-threaded `dvi_out`. They thread to take `&mut ShipoutState`, forward it to `dvi_out`, and must short-circuit / propagate the `DviFlush` overflow so the abort still fires after the borrow drops. Keep climbing `shipout.rs` (then `math.rs`/`linebreak.rs`, `scan.rs` last). Draft `02-modules.md` to READY before threading reaches scanner-heavy code.

---

## Idiom session 70 — 2026-05-31 — add classified codemod lane

**Type:** planning update (workstream 01, Phase C recipe). **One commit.** No code changed; dashboard not run.

**Why:** Phase C threading is repetitive enough that hand-editing every call/site is not the right operating model. The safe version is not blind regex; it is classify first, then codemod only the mechanically safe subset.

**Work done:**
1. `01b-phase-c-recipe.md`: added §4a, "Classified codemod passes".
2. The new rule divides targets into A/B codemod-safe work (pure pass-through threading and direct accessor rewrites) vs. C/D manual work (borrow-dropping wrappers, output/error/scanner boundaries, argument-evaluation hazards).
3. The per-slice recipe now has step 2a: classify the rewrite shape before editing and review the scripted diff before semantic work.

**Validation:** documentation-only change; no dashboard run.

**For the next session (71):** continue `shipout.rs` with the next clean DVI/output-position cluster, likely `dvi_out` / `dvi_four` / `dvi_two`. Start by classifying the cluster per `01b` §4a, codemod only A/B pieces, then manually handle any borrow-dropping DVI output or error paths. Keep scanner-heavy work for later; draft `02-modules.md` to READY before threading reaches scanner internals.

---

## Idiom session 69 — 2026-05-31 — thread shipout DVI movement

**Type:** execution (workstream 01, Phase C — second `shipout.rs` slice). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread `movement`, the DVI movement command/list helper:
- `movement` now takes `&mut MemState` and `&mut ShipoutState`, allocates and links the movement node with threaded `get_node`, patches `dvi_buf` directly, and updates movement-list state directly.
- The direct helper returns the DVI bytes to emit instead of calling `dvi_out` / `dvi_four` under the borrow.
- `movement_global` performs the legacy `dvi_out` writes after the borrow drops.

**Recon / safety checks:**
- `movement` has only a stale prototype/comment in `xetex/xetex-shipout.c`; no executable C/C++ callers.
- No cleanup aliases in `src/ini.rs`.
- No cross-module extern/static/link-name redeclarations beyond the pre-slice Rust definition itself.

**Work done:**
1. `shipout.rs`: removed `#[no_mangle] extern "C"` from `movement`, made the direct helper safe, and added `movement_global`.
2. Repointed all `shipout.rs` callers to `movement_global`.
3. Replaced the no-longer-used `down_ptr` / `right_ptr` bridge accessors with direct init writes in `initialize_shipout_variables`.

**Newtype debt:** none. No state fields were added. Parameters are existing plain `scaled_t` / `eight_bits` representations (`i32` / `u8`), and movement-list pointers remain raw TeX integer pointers.

**Metrics:** `#[no_mangle]` exports **394 → 392** (measured by direct grep). `globals.rs` raw externs unchanged at **40**. `unsafe fn` unchanged at **688**. Raw `unsafe { ... }` text-search count unchanged at **160**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** passed perfectly.

**For the next session (70):** continue `shipout.rs`. Natural candidates are the smaller DVI byte helpers (`dvi_out` / `dvi_four` / `dvi_two`) or another output-position cluster that can be split into direct state mutation plus borrow-dropping output wrappers. Keep scanner-heavy work for later; draft `02-modules.md` to READY before threading reaches scanner internals.

---

## Idiom session 68 — 2026-05-30 — thread shipout DVI movement-node helpers

**Type:** execution (workstream 01, Phase C — first `shipout.rs` slice after the `save.rs` run). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread the clean DVI movement-node maintenance helpers:
- `new_edge` now takes `&mut MemState`, calls threaded `get_node`, initializes the edge node directly, and propagates `ALLOC_FAILED` to `new_edge_global`.
- `prune_movements` now takes `&mut MemState` and `&mut ShipoutState`, walks `down_ptr` / `right_ptr` directly, and calls threaded `free_node`.
- `dvi_pop` now takes `&mut ShipoutState` and returns whether a POP byte must be emitted; `dvi_pop_global` drops the borrow before calling legacy `dvi_out`.

**Recon / safety checks:**
- `new_edge` is header-prototype-only (`xetex-xetexd.h`); no C/C++ callers.
- `prune_movements` / `dvi_pop` have stale prototypes and "translated to Rust" comments in `xetex/xetex-shipout.c`, but no executable C callers.
- No cleanup aliases in `src/ini.rs`.
- No cross-module extern/static/link-name redeclarations beyond the pre-slice Rust definitions themselves.

**Work done:**
1. `shipout.rs`: added direct-state helper signatures for `new_edge`, `prune_movements`, and `dvi_pop`; added `_global` wrappers for legacy callers.
2. `scan.rs` and remaining `shipout.rs` callers now use `new_edge_global`, `prune_movements_global`, and `dvi_pop_global`.
3. Left `movement` legacy for now because it interleaves mem-list edits with `dvi_out`/`dvi_four` output and DVI-buffer mutation; that needs its own borrow-dropping command shape.

**Newtype debt:** none. No state fields were added. `new_edge` parameters are existing plain `small_number`/`scaled_t` representations (`i16`/`i32`), and DVI list pointers are existing raw TeX integer pointers.

**Metrics:** `#[no_mangle]` exports **396 → 394**. `globals.rs` raw externs unchanged at **40**. `unsafe fn` **689 → 688**. Raw `unsafe { ... }` text-search count unchanged at **160**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** passed perfectly.

**For the next session (69):** continue `shipout.rs` with the next clean cluster. Likely candidates are `movement` (requires a direct mem/shipout pass that returns DVI op emissions for the wrapper) or a smaller output-position helper that can avoid calling `dvi_out` while a state borrow is live. Keep scanner-heavy code for later; draft `02-modules.md` to READY before threading reaches scanner internals.

---

## Idiom session 67 — 2026-05-30 — thread `save.rs` SA restore/define helpers

**Type:** execution (workstream 01, Phase C — third `save.rs` call-graph slice). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread the remaining sparse-array restore/define side:
- `sa_destroy` is now a direct `&mut MemState` helper. Its glue/token branches use threaded mem helpers; its box-list branch returns a pending `flush_node_list` pointer so wrappers can drop the state borrow before calling the still-legacy `scan.rs` flusher.
- `sa_def`, `sa_w_def`, `gsa_def`, and `gsa_w_def` now operate on direct mem/scanner/save-stack borrows where needed, call threaded `sa_save`, and propagate `ALLOC_FAILED` / `SAVE_OVERFLOW` through one wrapper finisher.
- `sa_restore` is split into a direct prepare step plus a `_global` wrapper loop that performs any pending `flush_node_list` between borrow phases, then calls threaded `delete_sa_ref` / `free_node`.

**Recon / safety checks:**
- Header prototypes only for `sa_destroy` / `sa_restore` / definition helpers; no C/C++ callers.
- No cleanup aliases in `src/ini.rs`.
- `grep -rnE '\b(sa_destroy|sa_restore|sa_def|sa_w_def|gsa_def|gsa_w_def)\b' xetex/*.c | grep -v macos` was empty before and after.
- No cross-module extern/static/link-name redeclarations.

**Work done:**
1. `save.rs`: removed `#[no_mangle] extern "C"` and `unsafe` from the six helper definitions; added bridge-down wrappers for legacy callers.
2. Added a small threaded token-ref helper and reused threaded `flush_list`, `free_node`, `delete_sa_ref`, and `sa_save`.
3. Repointed legacy callers in `save.rs`, `scan.rs`, and `ini.rs` to the `_global` wrappers.
4. Removed now-dead `sa_save_global` / `sa_destroy_global` wrappers.

**Newtype debt:** none. No state fields were added; moved parameters are plain TeX integer pointers/values, and the direct state fields were already tracked when `MemState`, `ScannerState`, and `SaveStackState` moved.

**Metrics:** `#[no_mangle]` exports **402 → 396**. `globals.rs` raw externs unchanged at **40**. `unsafe fn` **690 → 689** (six unsafe exports removed, five legacy wrappers plus one finisher added, two dead unsafe wrappers removed). Raw `unsafe { ... }` text-search count unchanged at **160**.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** passed perfectly. Post-slice C-reference grep for the six helpers was empty.

**For the next session (68):** move out of `save.rs` and climb to `pagebuilder.rs`/`shipout.rs`. Pick a clean call-graph cluster that depends on the now-threaded save/mem helpers; keep `scan.rs` last, and draft `02-modules.md` to READY before threading reaches scanner-heavy code.

---

## Idiom session 66 — 2026-05-30 — thread `save.rs` SA ref/save helpers

**Type:** execution (workstream 01, Phase C — second `save.rs` call-graph slice). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread the clean SA refcount/save half of the save subsystem:
- `delete_sa_ref` now takes `&mut MemState` and `&mut ScannerState`, updates SA trie roots directly, and calls threaded `free_node` / a small direct `delete_glue_ref_threaded` helper instead of the thread-local mem bridge.
- `sa_save` now takes `&mut MemState`, `&mut ScannerState`, and `&mut SaveStackState`, calls threaded `get_node`, writes the save stack directly, and propagates `ALLOC_FAILED` / `SAVE_OVERFLOW` sentinels to its wrapper.
- Legacy callers use `delete_sa_ref_global` / `sa_save_global`; `sa_save_global` performs `overflow("main memory size", MEM_TOP+1)` or `overflow("save size", save_size())` after the borrow drops.

**Recon / safety checks:**
- Header prototypes only for `delete_sa_ref` / `sa_save`; no C/C++ callers.
- No cleanup aliases in `src/ini.rs`.
- `grep -rnE '\b(delete_sa_ref|sa_save)\b' xetex/*.c | grep -v macos` was empty before and after the slice.
- No cross-module extern/static/link-name redeclarations.

**Work done:**
1. `state.rs`: added `MemState::b16s1` to support direct SA node-type reads.
2. `save.rs`: dropped `#[no_mangle] extern "C"` and `unsafe` from `delete_sa_ref` and `sa_save`; added bridge-down wrappers; removed the now-dead `globals::sa_level` bridge.
3. Repointed legacy callers in `save.rs` and `scan.rs` to the `_global` wrappers.

**Newtype debt:** none. No new state fields were added. The newly threaded parameters are existing mem/save/scanner state borrows plus plain integer pointers; `sa_level` debt, if any, was already covered when the scanner SA state moved.

**Metrics:** `#[no_mangle]` exports **404 → 402**. `globals.rs` raw externs unchanged at **40**. `unsafe fn` unchanged at **690** (two safe threaded functions replacing two unsafe exports, two unsafe wrappers added). Raw `unsafe { ... }` text-search count now **160** (the new `MemState::b16s1` union read accounts for the +1).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** passed perfectly. Post-slice C-reference grep for `delete_sa_ref|sa_save` was empty.

**For the next session (67):** continue `save.rs` with the restore/destroy/define side. Natural next cluster: `sa_destroy` + `sa_restore`, then the definition wrappers (`sa_def` / `sa_w_def` / `gsa_def` / `gsa_w_def`) if the direct `sa_destroy` shape is clean. Watch the `flush_node_list` dependency: it still lives in `scan.rs`, so either keep `sa_destroy` legacy for that path or thread only the mem-only branches until list flushing has a clean bridge.

---

## Idiom session 65 — 2026-05-30 — thread `save.rs` sparse-array allocation helpers

**Type:** execution (workstream 01, Phase C — first `save.rs` call-graph slice after the allocator/constructor work). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread the sparse-array allocation/traversal helper cluster in `save.rs`:
- `find_sa_element` now takes `&mut MemState` and `&mut ScannerState`, reads/writes `scanner.sa_root` directly, and returns the `ALLOC_FAILED` sentinel for cold allocation failure.
- Private helpers `new_index`, `sa_get_child`, `sa_set_child`, and `sa_alloc_chain` now operate on `MemState` directly and call the threaded `get_node`.
- Legacy callers use `find_sa_element_global`, which runs under one disjoint state borrow and performs the `overflow("main memory size", MEM_TOP+1)` abort after the borrow drops.

**Recon / safety checks:**
- Header prototypes only for `find_sa_element` / `new_index`; no C/C++ callers.
- No cleanup aliases in `src/ini.rs`.
- `grep -lnE '\b(find_sa_element|new_index)\b' xetex/*.c | grep -v macos` was empty.
- No cross-module extern/static/link-name redeclarations.

**Work done:**
1. `state.rs`: added `MemState::b16s0` to complete the direct mem read surface needed by `sa_set_child`.
2. `mem.rs`: made the allocator sentinel `ALLOC_FAILED` `pub(crate)` so threaded callers above `mem.rs` can propagate it without aborting under a live borrow.
3. `save.rs`: removed the helper cluster's internal `get_node_global` / mem-accessor use, eliminated the now-unused `globals::sa_null` bridge, and added `find_sa_element_global`.
4. Repointed legacy callers in `ini.rs`, `pagebuilder.rs`, and `scan.rs` to the `_global` wrapper. This keeps `scan.rs` legacy for now while allowing the `save.rs` helper itself to be threaded.

**Newtype debt:** none. No state fields were added; the threaded parameters remain existing `small_number`/plain integer values, and the moved direct accesses are existing `MemState` / `ScannerState` fields whose debts were already recorded when the fields moved.

**Metrics:** `#[no_mangle]` exports unchanged at **404**. `globals.rs` raw externs unchanged at **40**. `unsafe fn` count now **690** (four private/public helper signatures made safe, one `_global` wrapper added). Raw `unsafe { ... }` text-search count now **159**. Removed the now-dead `globals::sa_null` accessor.

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors; full dashboard → **29 / 2 / 7 / 4** passed perfectly. Post-slice C-reference grep for `find_sa_element|new_index` was empty.

**For the next session (66):** continue `save.rs` before moving to `pagebuilder.rs`/`shipout.rs`. The next natural cluster is the SA save/restore/free side (`delete_sa_ref` / `sa_save` / `sa_restore`, possibly `sa_destroy`/`sa_def` only as needed); run the same grep and nest-panic checks first because those functions mix `MemState`, `ScannerState`, save-stack state, and legacy helpers like `flush_node_list`.

---

## Idiom session 64 — 2026-05-30 — thread the three deferred constructors to state

**Type:** execution (workstream 01, Phase C — the final three constructors named in `01b` §9 / STATUS Next action). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread the final three deferred constructors:
- `new_param_glue` and `new_skip_param` — both read equivalents table state (`eqtb`). Threaded with a **widened disjoint borrow** (`mem: &mut MemState, eqtb: &EqtbState`). Added an `EqtbState::b32s1` read method mirroring `globals::eqtb_b32s1`. `new_skip_param` directly calls the threaded `new_spec` and `new_glue` now.
- `new_whatsit` — originally returned `()`. Rewritten to return `i32` to cleanly propagate the `ALLOC_FAILED` sentinel upwards, allowing `new_whatsit_global` to perform the `overflow()` abort outside the mutable borrow.

**Recon (per `01b` §7 step 2):**
- Rust call sites (per fn): `new_param_glue` 16, `new_skip_param` 5, `new_whatsit` 8.
- **No C/C++ callers** — only legacy prototype in `xetex-xetexd.h` which is left in place.
- `grep` checks proved no call sites inside `with_state` closures, and no other internal state-backed calls.

**Work done (1 commit):**
1. `state.rs`: implemented `EqtbState::b32s1` to allow threaded access to the equivalents table.
2. `mem.rs`:
   - Imported `EqtbState` along with `MemState`.
   - Threaded `new_param_glue` to `new_param_glue(mem: &mut MemState, eqtb: &EqtbState, n: i16) -> i32` and `new_skip_param` to `new_skip_param(mem: &mut MemState, eqtb: &EqtbState, n: i16) -> i32`.
   - Threaded `new_whatsit` to `new_whatsit(mem: &mut MemState, s: i16, w: i16) -> i32`.
   - Removed `#[no_mangle] extern "C"` from `new_param_glue` and `new_whatsit` (-2 exports) and dropped `unsafe` from all three signatures.
   - Added the transitional `_global` bridge-down wrappers (`new_param_glue_global`, `new_skip_param_global`, and `new_whatsit_global`) to cleanly handle sentinel-based overflow checks after the borrow drops.
3. Repointed all legacy callers across `src/math.rs`, `src/linebreak.rs`, `src/scan.rs`, `src/pagebuilder.rs`, and `src/pic.rs` to the `_global` wrappers via a single idempotent `perl` pass.
4. Verified that the crate checks successfully with zero new warnings.

**Newtype debt:** none — all params are plain integers (`i16`/`i32`), and no new fields were added to the state structs themselves.

**Metrics:** `#[no_mangle]` exports 406 → **404** (−2). `unsafe fn` net flat (3 signatures dropped `unsafe`; 3 wrappers added it). `globals.rs` raw externs unchanged (40).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, no new warnings; full dashboard → **29 / 2 / 7 / 4** passed perfectly.

**For the next session (65):** climb the call graph subsystem by subsystem, starting with `save.rs` (e.g. `save.rs` → `pagebuilder.rs`/`shipout.rs` → `math.rs`/`linebreak.rs` → `scan.rs` last). Draft `02-modules.md` to READY before threading reaches `scan.rs`.

---

## Idiom session 63 — 2026-05-30 — thread the mem-only `new_*` node constructors to `&mut MemState`

**Type:** execution (workstream 01, Phase C — the post-allocator-core slice named in
`01b` §9 / STATUS Next action). **One commit.** Dashboard **29/2/7/4**.

**Slice:** thread the **14 `i32`-returning, mem-only** `new_*` node constructors to take
`&mut MemState` and call the now-threaded [`get_node`] directly:
`new_null_box`, `new_rule`, `new_ligature`, `new_lig_item`, `new_disc`, `new_math`,
`new_spec`, `new_glue`, `new_kern`, `new_penalty`, `new_noad`, `new_style`,
`new_choice`, `new_margin_kern`. Each now calls `get_node(mem, …)` as a direct
pass-through instead of `get_node_global(…)`.

**Recon (per `01b` §7 step 2):**
- Rust call sites (per fn): null_box 18, rule 4, ligature 6, lig_item 6, disc 7,
  math 12, spec 17, glue 15, kern 35, penalty 13, noad 13, style 2, choice 2,
  margin_kern 3.
- **No C/C++ callers** — `xetex-xetexd.h` carries only the prototypes (no `.c`/`.cpp`
  call sites); like the session-61/62 allocator de-`#[no_mangle]`, the stale header
  decls are left in place (harmless, no caller).
- `grep -rnB3 … | grep with_state` empty: **no call site sits inside a `with_state*`
  closure**, so every legacy caller becomes a clean bridge-down wrap.
- Body classification (§4): all 14 touch the `mem` arena only (+ the already-threaded
  `get_node`); **zero** other `with_state`-bearing internal calls.

**Deferred (not closeable in a mem-only slice), left as `_global`-less legacy fns:**
- `new_param_glue`, `new_skip_param` — read `eqtb_b32s1(GLUE_BASE+n)`, a hot-path
  `with_state`-bearing call → nest-panic under the borrow. Need a widened disjoint
  `&EqtbState` borrow; a follow-up slice. (`new_skip_param`'s internal `new_spec`/
  `new_glue` calls were repointed to `new_spec_global`/`new_glue_global` this session.)
- `new_whatsit` — returns `()`, so it can't carry the `ALLOC_FAILED` sentinel up to a
  wrapper the way the 14 `i32`-returning ones do; threading it cleanly needs a
  different shape (separate slice).

**Sentinel propagation (`01b` §5), the new wrinkle vs. the allocator core:** because a
threaded constructor now calls the threaded `get_node` directly (which *returns*
`ALLOC_FAILED` on exhaustion rather than aborting under the borrow), each constructor
guards `let p = get_node(mem, s); if p == ALLOC_FAILED { return p; }` and propagates the
sentinel upward; its `new_X_global` bridge-down wrapper checks for it and runs
`overflow(c"main memory size", MEM_TOP+1)` **after** the `with_state_mut` borrow drops —
the exact message/shape `get_node_global` uses. `overflow()` never returns, so real
callers never observe the sentinel.

**Work done (1 commit):**
1. `state.rs`: added 5 `impl MemState` methods mirroring the `globals.rs` accessor
   widths — `set_b16s0`/`set_b16s1` (u16 writes), `set_gr` (f64 write), `word`/`set_word`
   (full `MemoryWord` copy). (`b32s0`/`b32s1`/`set_b32s0`/`set_b32s1` already existed
   from sessions 61–62.) Union writes through `&mut` index are safe; the `word` read is
   a `Copy` of the union (no projection).
2. `mem.rs`: threaded the 14 fns to `&mut MemState`, dropped `#[no_mangle] extern "C"`
   (−12 exports; `new_null_box`/`new_spec` were already plain `pub unsafe fn`) and
   `unsafe` from all 14 signatures (the union `unsafe` is encapsulated in the
   `MemState` methods). Added the 14 `new_X_global` bridge-down wrappers + sentinel
   abort. Two-phase-borrow hazards in `new_spec`/`new_glue` pre-empted by hoisting the
   read into a local before the `&mut` write (`let w = mem.b32s1(p+1); mem.set_b32s1(…)`).
3. Repointed `new_skip_param`'s internal `new_spec`/`new_glue` calls to the `_global`
   wrappers (it stays legacy this session).
4. Blanket-renamed all legacy call sites to `new_X_global` across 5 files
   (`math`/`linebreak`/`shipout`/`pagebuilder`/`scan`) via a single bare-identifier
   `perl` pass (idempotent — `NAME_global` doesn't re-match `\bNAME\b`); pre-checked
   there are **no** comment/string mentions to corrupt, and the pass also fixes the
   `use crate::mem::{…}` imports and the two aliased imports (`new_disc as _nd`,
   `new_ligature as _nl`) in one go. `mem.rs` call sites (the wrapper closures
   `new_X(&mut st.mem, …)`) were left untouched by editing it by hand.
5. Removed 3 now-unused `globals` imports from `mem.rs` (`mem_word`, `set_mem_gr`,
   `set_mem_word`) so the slice introduces **no** new warnings. The globals.rs
   accessors themselves stay (29–35 callers each elsewhere).

**Same `_global` bridge-down wrapper pattern as sessions 61–62** (named wrapper
evaluates its args at the caller before the borrow — sidesteps the argument nest-panic
and the `|s|`-closure shadowing). Each wrapper dies when its last legacy caller threads;
threading these constructors already turned their own `get_node_global` calls into
direct `get_node` pass-throughs.

**Newtype debt:** none — all params (`w`/`m`/`s`/`q`/`f`/`c`/`side`/`p`) are plain
`int32_t`/`uint16_t`/`small_number`(`i16`)/`internal_font_number`(`i32`) and the sentinel
is plain `i32`. (`small_number`/`internal_font_number` rows already exist from prior
sessions; the constructors preserve the existing `i16`/`i32` representation.)

**Metrics:** `#[no_mangle]` exports 418 → **406** (−12). `unsafe fn` net flat (14 fns
dropped `unsafe`; 14 `new_X_global` wrappers added it). `globals.rs` raw externs
unchanged (40).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, no new warnings
introduced by the slice (verified: the only newly-unused imports were the 3 removed);
full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29 / 2 / 7 / 4**, no regressions.

**For the next session (64):** thread the three deferred constructors —
`new_param_glue`/`new_skip_param` need a widened disjoint borrow
(`new_param_glue(mem: &mut MemState, eqtb: &EqtbState, …)`, wrapper splits
`&mut st.mem` / `&st.eqtb`); `new_whatsit` needs an `()`→sentinel rework (or thread its
`cur_list` writes too). Then climb the graph subsystem by subsystem
(`save.rs` → `pagebuilder.rs`/`shipout.rs` → `math.rs`/`linebreak.rs` → `scan.rs` last).
Draft `02-modules.md` to READY before threading reaches `scan.rs`.

---

## Idiom session 62 — 2026-05-30 — finish the `mem` allocator core: thread `flush_list`, `get_avail`, `get_node`

**Type:** execution (workstream 01, Phase C — slices 2–4 of `01b` §9). **One
commit** (user explicitly authorized batching the three). Dashboard **29/2/7/4**.
Completes the four-function `mem` allocator core started in session 61.

**Slice:** thread the remaining three allocator-core functions to `&mut MemState`:
- `flush_list` — same shape as `free_node` (no cold path, mem-only).
- `get_avail` — introduces the §5 cold-path overflow sentinel.
- `get_node` — cold-path sentinel + the two safe raw-global reads
  (`cur_input_synctex_tag()`/`line()`, which read `mod raw`, safe under a borrow
  per `01b` §4).

**Recon (per `01b` §7 step 2):** call-site counts (per file) matched the session-60
recon — `flush_list` 20, `get_avail` 82, `get_node` 85 (+ wrapper defs = 21/83/86).
`grep -rnB3 … | grep with_state` empty: no call site sits inside a `with_state*`
closure. `get_node`'s 16 in-`mem.rs` callers are the `new_*` node constructors.

**The cold-path overflow sentinel (`01b` §5), implemented:** added a module const
`ALLOC_FAILED: i32 = i32::MIN` (outside every valid index, `TEX_NULL`, and
`MAX_HALFWORD`, so it can never collide with a real return — and it crashes loudly
as an index if it ever leaked). The threaded `get_avail`/`get_node` **return
`ALLOC_FAILED`** on arena exhaustion instead of calling `runaway()`/`overflow()`
under the borrow; their `_global` wrappers check for it and perform the abort
**after** the `with_state_mut` borrow drops (`get_avail_global` → `runaway()` +
`overflow()`; `get_node_global` → `overflow()`). `overflow()` never returns, so
real callers never observe the sentinel. `get_node`'s legitimate `MAX_HALFWORD`
return for the `s == 0x4000_0000` sentinel request is unchanged (it is *not*
overflow).

**Work done (1 commit):**
1. `state.rs`: added `MemState::b32s1(&self, p)` (the read accessor the three fns
   needed beyond session-61's `b32s0`/`set_b32s0`/`set_b32s1`).
2. `mem.rs`: threaded all three fns to `&mut MemState`, touching fields/methods
   directly (the scalars `avail`/`mem_end`/`hi_mem_min`/`lo_mem_max`/`rover` are
   public `MemState` fields, read/written directly). Dropped `#[no_mangle]
   extern "C"` from `get_avail`/`flush_list` (now internal) and `unsafe` from all
   three signatures (the union `unsafe` is encapsulated in the `MemState` methods).
   Added the three `*_global` bridge-down wrappers + the `ALLOC_FAILED` const.
   In `get_node`, two-phase-borrow hazards were pre-empted by hoisting
   `mem.b32s0(q + 1)` into a `q_left` local (fed both a `&mut` write arg and a
   second write) and capturing `let rover = mem.rover` once in the lo-mem-grow
   block (constant there until the final `mem.rover = q`).
3. Renamed all legacy call sites to `<fn>_global` (8 files blanket via `perl`;
   `mem.rs` via `perl` with a `&mut st.mem`/`pub fn` exclusion so the threaded
   defs and the wrappers' internal calls were left untouched) and updated the
   `use crate::mem::{…}` imports across math/scan/shipout/pagebuilder/linebreak.
4. Removed the 8 now-unused `globals` imports from `mem.rs`
   (`hi_mem_min`/`lo_mem_max`/`mem_end`/`rover` + their `set_*`) so the slice
   introduces **no** new warnings.

**Same `_global` bridge-down wrapper pattern as session 61** (rationale recorded
there): a named wrapper evaluates its args at the caller before the borrow, which
sidesteps both the argument nest-panic hazard and the `|s|`-closure shadowing that
ruled out the recipe's bare inline closure. Each wrapper dies when its last legacy
caller threads.

**Newtype debt:** none — `p`/`s` are plain `int32_t`; the sentinel is a plain
`i32` (checked).

**Metrics:** `#[no_mangle]` exports −2 (`get_avail` + `flush_list` dropped their
exports; `get_node`/`free_node` were already non-`#[no_mangle]`). `unsafe fn` net
flat (the three fns dropped `unsafe`; three `*_global` wrappers added it). The
`mem` allocator core (`free_node`/`flush_list`/`get_avail`/`get_node`) is now fully
threaded — the dependency-graph foundation for the rest of Phase C. `globals.rs`
raw externs unchanged (40).

**Validation:** `cargo check -p tectonic_engine_xetex` → 0 errors, no new warnings
introduced by the slice (verified by grep for unused `get_*`/`*mem*`/`rover`);
full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29 / 2 / 7 / 4**, no regressions.

**For the next session (63):** thread the `new_*` node constructors
(`new_null_box`, `new_rule`, `new_ligature`, `new_lig_item`, `new_disc`,
`new_math`, `new_spec`, `new_param_glue`, `new_glue`, `new_kern`, `new_penalty`,
`new_noad`, `new_choice`, the margin-kern/style/character builders, …) — they call
`get_node_global`/`get_avail_global`/`free_node_global`, so threading them to take
`&mut MemState` turns those wrapper calls into direct `get_node(&mut mem, …)`
pass-throughs, shrinking the bridge-down wrapper count. After the constructors,
climb the graph subsystem by subsystem (`save.rs` → `pagebuilder.rs`/`shipout.rs`
→ `math.rs`/`linebreak.rs` → `scan.rs` last). Draft `02-modules.md` to READY
before threading reaches `scan.rs`.

---

## Idiom session 61 — 2026-05-30 — first Phase-C threading slice: `free_node(&mut MemState, …)`

**Type:** execution (workstream 01, Phase C — the first threading slice per
`01b-phase-c-recipe.md` §9). One commit. Dashboard **29/2/7/4**.

**Slice:** thread the `mem` allocator core's purest member, `free_node`, from the
thread-local bridge to a direct `&mut MemState` parameter, and bridge-down all its
legacy call sites. This is the recon-justified foundation slice (`01b` §9): per the
§4 table `free_node` touches the `mem` arena only with **zero** `with_state`-bearing
internal calls, so it threads with no nest-panic risk on any path.

**Recon (before touching anything, `01b` §7 step 2):**
- Call sites: 95 across 9 files (scan 46, math 16, linebreak 13, shipout 8, save 4,
  pagebuilder 4, input 2, mem 1, ini 1) — matches the session-60 count.
- `grep -rnB3 'free_node[(]' | grep with_state` → empty: no call site already sits
  inside a `with_state*` closure.
- `free_node` body uses only `mem` accessors (`set_mem_b32s0`, `set_mem_b32s1`,
  `mem_b32s0`, `rover()`) — all `mem`-cluster, no other cluster, confirming the §4
  classification.

**Work done (1 commit):**
1. `state.rs`: added `impl MemState` with `b32s0(&self,p)` / `set_b32s0(&mut self,p,v)`
   / `set_b32s1(&mut self,p,v)`, mirroring the `globals.rs` `mem_b32s0`/`set_mem_b32s0`/
   `set_mem_b32s1` accessor widths (`01b` §6). The `unsafe` `MemoryWord` union *read*
   projection lives inside `b32s0`; the writes through `&mut` index are safe. `rover`
   stays a public field, read directly.
2. `mem.rs`: `free_node` → `pub fn free_node(mem: &mut MemState, p: i32, s: i32)`,
   body rewritten to the methods / direct field access (no `unsafe` needed on the fn
   anymore). Added the transitional `pub unsafe fn free_node_global(p, s)` bridge-down
   wrapper (`with_state_mut(|st| free_node(&mut st.mem, p, s))`). Imported `MemState`.
3. Renamed all 95 legacy call sites `free_node(` → `free_node_global(` (8 files via
   `perl`; the single in-`mem.rs` legacy call by hand, leaving the threaded def and
   the wrapper's internal `free_node(&mut st.mem,…)` call untouched) and updated the
   5 `use crate::mem::{… free_node …}` imports to `free_node_global`.

**Deviation from the recipe's inline-closure shape (deliberate, documented):** `01b`
§3 shows bridging legacy sites with an inline `with_state_mut(|s| free_node(&mut s.mem,
…))`. That is **unsafe here** for two reasons found during the edit: (a) ~16 sites pass
a `with_state`-bearing accessor *as an argument* (`mem_b16s3(…)`, `active_node_size()`,
`ha()`, `eqtb_b32s1(…)`, `mem_b32s0(…)`, `mem_b32s1(…)`) — evaluated *inside* the
closure these re-borrow `ENGINE_STATE` and panic; (b) two sites are `free_node(r, s)`
where `s` is a local size, which an inline `|s|` closure param would shadow. A named
`free_node_global` wrapper evaluates its arguments at the *caller* before the borrow is
taken, eliminating both hazards, and it is just as incrementally removable (it dies
when its last legacy caller threads). **Recommended pattern for future Phase-C slices
of many-site / non-trivial-arg functions** — reserve the bare inline closure for
few-site fns with only simple args. (Noted in STATUS "Next action".)

**Newtype debt:** none — `p`/`s` are plain `int32_t` (checked).

**Metrics:** essentially flat for this foundation slice (the payoff lands as callers
thread and accessors are deleted). `globals.rs` raw externs unchanged (40). `unsafe fn`
net unchanged (`free_node` dropped `unsafe`; `free_node_global` added it). One transient
`with_state_mut` wrapper added; the `b32s0` method moved one union-read `unsafe` block
from the `globals.rs` accessor side into `MemState`. No `#[no_mangle]` change
(`free_node` was already internal).

**Validation:** `cargo check -p tectonic_engine_xetex` clean (0 errors; pre-existing
warnings only); full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29 / 2 / 7 / 4**, no regressions.

**For the next session (62):** thread `flush_list` next (`01b` §9, same shape as
`free_node`, ~21 sites), then `get_avail` (first user of the §5 cold-path overflow
sentinel — the threaded body returns a sentinel on exhaustion and the wrapper calls
`overflow()`/`runaway()` after the borrow drops), then `get_node`. Reuse the
`<fn>_global` bridge-down wrapper pattern. After the four-fn core, thread the `new_*`
node constructors so they become pass-through callers (shrinking the wrapper count).
Consider drafting `02-modules.md` to READY before threading climbs into `scan.rs`.

---

## Idiom session 60 — 2026-05-30 — planning slice: draft the Phase-C execution recipe (`01b`) to READY

**Type:** planning (workstream 01, Phase-C kickoff). **No code change** — the
deliverable is `plan/idiom/01b-phase-c-recipe.md` plus this log + STATUS updates.
One commit (plan files only).

**Why a planning slice (not a code dive):** Phase B is complete — the 40 remaining
`mod raw` statics are all the intentionally-deferred hot-tokenizer / capacity /
`cur_ptr` set, so there is no Phase-B slice left. Phase C is the inflection point
and hard rule 4 (`README.md`) forbids executing without a `READY` plan. The §11
recipe in `01-state-and-memory.md` is the Phase-**B** recipe; there was no
equivalent for Phase-C threading. This session writes it. (User confirmed scope:
"recipe + concretely pick the first subsystem with call-site/coupling recon.")

**Recon done (grounds the recipe):**
- **Allocator-core internals classified** (`src/mem.rs`): `free_node`/`flush_list`
  touch the `mem` arena only, with **zero** `with_state`-bearing calls.
  `get_avail`/`get_node` add only raw-global reads `line()` /
  `cur_input_synctex_tag()` **plus** the cold `overflow()`/`runaway()` abort path.
- **Session-59 blocker refuted:** `line()` (`globals.rs:457`) and
  `cur_input_synctex_tag()` (`globals.rs:393`) read `raw::line` / `raw::cur_input`
  directly — those are still `mod raw` (the deferred Phase-C set), **not** the
  thread-local. So they do **not** nest-panic under a live `with_state*` borrow.
  Only the **cold** `overflow`/`runaway` path is `with_state`-bearing (it prints
  through migrated `selector`/`help_line` state, then `_tt_abort` longjmps —
  never reached in any passing test). So the allocator core *is* threadable now,
  contrary to the session-59 handoff's blanket "would nest-panic" claim.
- **Call-site counts** (`grep -rcE`): `free_node` ~96 (scan 46, math 16,
  linebreak 13, shipout 8, …), `flush_list` ~21 (scan 13, …), `get_avail` ~83
  (scan 60, …), `get_node` ~86 (scan 40, mem 17, …). Large but mechanical; no
  smaller self-contained subsystem exists because everything bottoms out here.

**Decisions baked into `01b` (also added to STATUS Decisions log):**
1. **Anchor + thread-below** (resolves §7 longjmp/`Drop`): the `EngineState`
   owner stays in the `'static` thread-local for all of Phase C; threading
   borrows the anchor once at a boundary and passes `&mut` down. Never stack-own
   `EngineState` across the setjmp boundary.
2. **Bridge-down technique** (mirror of Phase B's bridge-up): the transient bridge
   moves from the accessor body to the *call site*
   (`with_state_mut(|s| f(&mut s.mem, …))`); each wrap is deleted when its caller
   is itself threaded. The thread-local is deleted when the last `with_state` site
   is gone.
3. **Nest-panic rule** stated precisely: `with_state*` borrows the *whole* RefCell;
   classify every call a threaded fn makes into (a) threaded pass-through,
   (b) raw-`mod raw` (safe under borrow), (c) still-bridged `with_state` (hazard).
4. **Cold-error-path sentinel:** don't thread `overflow`/`confusion`/`runaway`
   (they pull in the whole print subtree and abort anyway); threaded allocators
   return an overflow sentinel and let the wrapper call `overflow()` after the
   borrow drops.
5. **Narrow borrows / `MemState` method surface** + disjoint-field borrows
   (`&mut s.mem`, `&mut s.eqtb`), never `.clone()` an arena.

**First subsystem picked (recipe §9):** the `mem` allocator core threaded as
`&mut MemState`, commit order `free_node` → `flush_list` → `get_avail` →
`get_node` (easiest/purest first; the cold-path sentinel arrives with
`get_avail`). Then the `new_*` constructors become pass-through callers, then
climb the graph (`save` → pagebuilder/shipout → math/linebreak → `scan.rs` last,
coordinated with workstream 02).

**Validation:** none required (no code changed). Dashboard untouched at 29/2/7/4.

**For the next session (61):** execute the first Phase-C slice — thread
`free_node(&mut MemState, …)` and bridge-down its ~96 call sites — following
`01b` §7. Per-site check (`01b` §7 step 2): confirm no `free_node` call already
sits inside a `with_state*` closure before wrapping (engine fns call it directly,
so expected clean, but verify). Add the `impl MemState` method surface in
`state.rs` mirroring the `globals.rs` mem accessor widths. Consider drafting
`02-modules.md` to READY before threading reaches `scan.rs`.

---

## Idiom session 59 — 2026-05-30 — dead-code cleanup: remove orphaned `xmalloc_array`/`xcalloc_array` glue

**Type:** execution (workstream 01 follow-up; dead-code removal explicitly teed
up by session 58). One commit. **No state migration** — Phase B is complete.

**Context / why this slice (not a Phase-C dive):** Phase B is essentially done.
The 40 remaining `mod raw` statics are all the intentionally-deferred Phase-C set
(hot tokenizer `cur_cmd/chr/tok/cs/val`, hot lig/kern `main_*`/`cur_l/r/q`/
`lig_stack`, cursor state `cur_input`/`line`/`cur_list`/`temp_ptr`), capacity
limits (`stack_size`, `param_size`, `nest_size`, `save_size`, `expand_depth`,
`first`, `buf_size`, `max_in_open`), or `cur_ptr`. None is a clean Phase-B bridge.
The STATUS-suggested Phase-C start (thread `&mut MemState` through `src/mem.rs`)
was scoped and is **not** a clean one-commit slice: `get_avail`/`get_node`/
`free_node` have 83/86/96 call sites, and `get_node` internally calls `overflow`,
`cur_input_synctex_tag()`, `line()` (all `with_state`-bearing), so wrapping a
threaded call in `with_state_mut(|s| get_node(&mut s.mem, …))` at legacy call
sites would nest-panic. Phase C here needs top-down threading + a READY plan
(hard rule 4); it is not a single safe slice. User chose the safe dead-code
cleanup this session.

**Work done (1 commit):**
- Removed the two now-dead generic helpers `xmalloc_array<T>` /
  `xcalloc_array<T>` from `src/ini.rs`. After session 58 moved the `mem` arena to
  a Rust `Vec`, the last `xmalloc_array`/`xcalloc_array` caller disappeared
  (verified: zero non-comment references crate-wide).
- Removed the now-orphaned `xcalloc` extern declaration from `ini.rs`'s
  `extern "C"` block (its only remaining user was `xcalloc_array`). The `xmalloc`
  extern stays — still used at `ini.rs` for `native_text` scratch (1128) and the
  format-default name dup (3164).
- Kept the explanatory `// xmalloc_array(n) allocated n + 1 …` comments that
  document the preserved inclusive Vec sizes — they remain useful parity notes.

**Validation:** `cargo check -p tectonic_engine_xetex` clean (warnings only, all
pre-existing); full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29/2/7/4**, no regressions. (The mid-edit rust-analyzer E0425 for `xcalloc`
was stale line-position noise — `cargo check` is authoritative and passed.)

**Metrics after this session:** `globals.rs` raw extern count unchanged at **40**
(this slice removed dead `ini.rs` glue, not a `mod raw` static). One fewer FFI
extern decl; two fewer `unsafe fn`s in `ini.rs`.

**For the next session (60+) — the Phase-C inflection:**
- Phase B is complete. The clean next step is **not** a blind code dive but a
  **planning slice**: draft a READY Phase-C execution recipe (the §11 analog for
  threading), resolving the longjmp/`Drop` boundary (§7) and the nested-borrow
  hazard, and pick the first genuinely-decoupled subsystem to thread top-down.
  Workstream 02 (module split) is still `PLANNED` and Phase C is told to
  coordinate with it — consider drafting that to READY too.
- When code threading does start: the `src/mem.rs` allocators are the §5-step-4
  natural first consumers of `&mut MemState`, but only once their whole call
  subtree (and the error paths they touch) can be threaded together — they cannot
  be wrapped piecemeal.

---

## Idiom session 58 — 2026-05-30 — the `mem` arena (+ bookkeeping scalars) → EngineState.mem (MemState)

**Type:** execution (workstream 01, Phase-B; the dynamic-memory arena ownership
move — the hottest/highest-leverage slice). One commit.

**Slice:** `mem` (`memory_word *`, the engine's dynamic-memory arena) → Rust-owned
`Vec<MemoryWord>` on a new `EngineState.mem` (`MemState`), plus its seven
bookkeeping scalars `lo_mem_max`, `hi_mem_min`, `avail`, `mem_end`, `rover`,
`var_used`, `dyn_used` (all moved *with* the array per §11's gotcha — they are not
a loose scalar slice). This is the only true Phase-B-bridgeable group left; the
remaining 40 statics are intentionally-deferred Phase-C (hot tokenizer
`cur_cmd/chr/tok/cs/val`, hot lig/kern `main_*`/`cur_l/r/q`), capacity limits
(`stack_size`, …), or `cur_ptr` (too broad).

**Pre-move grep checks (§11 step 2):**
- (a) C types (`xetex-xetexd.h:460–466`): `memory_word *mem` (element already
  `MemoryWord` → no debt); the 7 scalars are all plain `int32_t`.
- (b) aliases: `tre_mem` (alloc) + `tt_cleanup_mem` (cleanup) in `src/ini.rs` —
  this is an ownership slice (§11 step 6) with format-dump reconciliation.
- (c) C readers: only `xetex-ini.c` (the definitions). The `mem[...]` hits in
  `xetex-constants.h`/`xetex-xetexd.h` are unused field macros, not compiled
  reads.
- (d) cross-module Rust redeclarations: only the two `src/ini.rs` aliases (handled).
- format dump: **yes** — `mem` is serialized (the free-list/upper regions via
  `dump_slice`/`undump_slice`). Reconciled with the session-51 eqtb base-pointer
  pattern.

**Work done (1 commit):**
- Added `MemState` (`data: Vec<MemoryWord>` + the 7 `i32` scalars) to `src/state.rs`,
  `#[derive(Default)]` (Vec allocated in `ini.rs`); added `mem: MemState` to
  `EngineState`.
- Repointed ~30 accessors in `src/globals.rs`: the indexed word accessors
  (`mem_b32s0/1`, `mem_b16s0/1/2/3`, `mem_gr`, `mem_ptr`, `mem_word` + setters) now
  index `s.mem.data[p]` (reads keep an inner `unsafe` union projection, writes are
  fully safe — session-41 rule); the three pointer-escaping accessors
  (`mem_node_addr`, `mem_as_u16_ptr`, `mem_as_u16_ptr_mut`) use
  `as_ptr()/as_mut_ptr().add(p)` (valid past the borrow — the thread-local owner
  never reallocates); the 7 scalars are plain `with_state`/`with_state_mut`.
- `src/ini.rs`: replaced both `xmalloc_array::<MemoryWord>(…)` allocations with
  `vec![<zero MemoryWord>; …]` — **two different inclusive sizes preserved**:
  format-load `xmalloc_array(MEM_TOP)` → `MEM_TOP + 1` words; initex
  `xmalloc_array(MEM_TOP + 1)` → `MEM_TOP + 2` words. Format dump/undump take a
  state-derived base pointer once outside the loop (the eqtb pattern), valid
  across the interleaved `mem_*` accessor reads. The initex-reset `libc::free` and
  the `tt_cleanup` `libc::free` are now `s.mem.data = Vec::new()`. Removed both
  `#[link_name="mem"]` externs (`tre_mem`, `tt_cleanup_mem`).
- Removed the 8 raw `static mut`s from `globals.rs` `mod raw` (and its now-unused
  `use crate::mem_types::MemoryWord;`), and the 7-line C definition block from
  `xetex/xetex-ini.c`.

**Newtype debts (§4):**
- None. The `mem` element is `memory_word` (faithful `MemoryWord`). The 7 scalars
  are plain `int32_t` — the mem-pointer scalars (`lo_mem_max`, `hi_mem_min`,
  `avail`, `mem_end`, `rover`) are pointer-taxonomy-sweep candidates only, not
  semantic typedefs, so no worklist rows (checked in `xetex-xetexd.h:460–466`).

**Validation:** full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29/2/7/4**, no regressions. (tex-outputs 39.4s — the hot mem path now goes
through the thread-local `with_state` bridge; acceptable for Phase B, removed when
the arena is threaded in Phase C.)

**Metrics after this session:**
- `globals.rs` raw extern count: **48 → 40** (−8).
- All `input`-cluster, font, hyph, string, eqtb, **and now the `mem` arena** are
  Rust-owned `Vec`s. `xmalloc_array`/`xcalloc_array` are now dead in `ini.rs`
  (no remaining caller) — a candidate for removal in a follow-up.

**For the next session (59+):**
- The remaining 40 statics are the **Phase-C set**: hot tokenizer scalars
  (`cur_cmd/chr/tok/cs/val`), the hot lig/kern block (`main_*`, `cur_l/r/q`,
  `lig_stack`, …), capacity limits (`stack_size`, `nest_size`, `buf_size`,
  `param_size`, `save_size`, `max_in_open`, `expand_depth`, `first`), `cur_ptr`,
  `cur_input`/`cur_list`/`line`/`temp_ptr`. Phase B is essentially complete.
- **Begin Phase C**: thread `&mut EngineState` (or a narrower `&mut MemState`/
  `&mut Mem`) through one decoupled subsystem and remove its thread-local bridge.
  The `mem` allocator functions in `src/mem.rs` (`get_avail`/`get_node`/
  `free_node`/…) are the natural first consumers of `&mut MemState` (§5 step 4).

---

## Idiom session 57 — 2026-05-30 — xdv_buffer, mapped_text, and native_text cluster → EngineState.fonts / ScannerState

**Type:** execution (workstream 01, Phase-B; buffer globals migration). One commit.

**Slice:** `xdv_buffer`, `mapped_text`, and the `native_text` cluster (`native_text`, `native_text_size`, `native_len`, `save_native_len`) → Rust-owned `Vec<u8>` (for `xdv_buffer`), `Vec<u16>` (for `mapped_text`), and `Vec<u16>` (for `native_text`) on `EngineState.fonts` (`FontState`) and `EngineState.scanner` (`ScannerState`) respectively.

**Pre-move grep checks (§11 step 2):**
- (a) C types: `char *xdv_buffer`, `UTF16_code *mapped_text`, `UTF16_code *native_text`, `int32_t native_text_size`, `int32_t native_len`, `int32_t save_native_len`.
- (b) aliases: none.
- (c) C readers: only `xetex-ini.c` (definition). No C code references them.
- (d) cross-module Rust extern redeclarations: `native_text` was redeclared in `src/scan.rs` inside an `extern "C"` block.
- format dump: none.

**Work done (1 commit):**
- Added `xdv_buffer: Vec<u8>` and `mapped_text: Vec<u16>` to `FontState` in `src/state.rs`.
- Added `native_text: Vec<u16>`, `native_text_size: i32`, `native_len: i32`, and `save_native_len: i32` to `ScannerState` in `src/state.rs`.
- Removed their raw static mut declarations from `src/globals.rs`.
- Repointed accessors `xdv_buffer_entry`, `mapped_text_entry`, `native_text`, `set_native_text`, `native_text_size`, `set_native_text_size`, `native_len`, `set_native_len`, `save_native_len`, and `set_save_native_len` in `src/globals.rs` to delegate to their respective thread-local state.
- Added type-safe `xdv_buffer()` and `mapped_text()` accessors that return pointer references to vector storage for FFI/ext compatibility.
- Cleaned up the manual allocation and reallocation in `src/ext.rs` (`makeXDVGlyphArrayData`, `make_font_def`, and `apply_mapping`) to use native vector resizing and slice pointers rather than dynamic malloc/free.
- Replaced all `xrealloc` loops on `native_text` in `src/scan.rs` with safe, native vector `resize` calls.
- Resolved a linker error (`undefined symbol: native_text`) by removing `native_text` from the `extern "C"` block in `src/scan.rs` (`do_locale_linebreaks`) and instead using a safe local pointer retrieved via `crate::globals::native_text()`.
- Updated `tt_cleanup()` in `src/ini.rs` to clear the buffers and reset tracking variables, completely removing the manual `libc::free(tt_cleanup_native_text)`.
- Removed C definitions from `xetex/xetex-ini.c`.

**Newtype debts (§4):**
- None.

**Validation:** full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29/2/7/4** passes perfectly with no regressions!

**Metrics after this session:**
- `globals.rs` raw extern count: **54 → 48** (−6).
- `thin global accessors`: 823 total / 182 state-backed scalars/arrays (+6) + 74 Rust-owned arrays/arenas (+3 Rust-owned vectors: `xdv_buffer`, `mapped_text`, `native_text`).

**For the next session (58+):**
- Migrate the **`mem` arena** (§5) — the hottest/highest-leverage ownership move and the real pivot into Phase C.
- Or transition to **Phase C** by threading state explicitly and removing the thread-local bridge for one of the decoupled subsystems (e.g., `config`, `errors`, or `shipout`).

---

## Idiom session 56 — 2026-05-30 — filename buffer globals name_of_file and name_of_file16 → EngineState.input

**Type:** execution (workstream 01, Phase-B; filename buffer globals migration). One commit.

**Slice:** the filename buffer globals (`name_of_file` and `name_of_file16`) → Rust-owned `Vec<libc::c_char>` and `Vec<u16>` on `EngineState.input` (`InputClusterState`).

**Pre-move grep checks (§11 step 2):**
- (a) C types: `char *name_of_file`, `UTF16_code *name_of_file16`. No newtype debt (plain character pointer arrays).
- (b) aliases: none.
- (c) C readers: only `xetex-ini.c` (definition). No C code references them.
- (d) cross-module Rust extern redeclarations: none.
- format dump: none.

**Work done (1 commit):**
- Added `name_of_file: Vec<libc::c_char>` and `name_of_file16: Vec<u16>` fields to `InputClusterState` in `src/state.rs`.
- Removed their raw static mut declarations from `src/globals.rs`.
- Repointed accessors `name_of_file`, `set_name_of_file`, `name_of_file16`, `set_name_of_file16`, and `name_of_file16_entry` in `src/globals.rs` to delegate to `state.input`. Accessors return pointer to `Vec` storage (`.as_ptr()` / `.as_mut_ptr()`), preserving the interface at FFI boundaries.
- Cleaned up the manual allocation and reallocation in `src/scan.rs` (`pack_file_name`) and `src/io.rs` (`make_utf16_name`), completely eliminating `libc::free` and `xmalloc`/`xcalloc` on filename buffers in favor of safe, native vector manipulations.
- Removed the module-local `static mut NAME16LEN` from `src/io.rs`.
- Removed C definitions from `xetex/xetex-ini.c`.
- Updated `tt_cleanup()` in `src/ini.rs` to clear the filename vectors on session teardown.

**Newtype debts (§4):**
- None. `name_of_file` and `name_of_file16` are plain character pointers, not semantic scalar typedefs.

**Validation:** full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29/2/7/4** passes perfectly with no regressions!

**Metrics after this session:**
- `globals.rs` raw extern count: **56 → 54** (−2).
- `thin global accessors`: 823 total / 182 state-backed scalars/arrays (+2 Rust-owned vectors).

**For the next session (57+):**
- Migrate the **`mem` arena** (§5) — the hottest/highest-leverage ownership move and the real pivot into Phase C.
- Or migrate remaining ownership slices (`xdv_buffer`, `native_text`).

---

## Idiom session 55 — 2026-05-30 — equivalents sparse-array metadata and sentinel globals → ScannerState

**Type:** execution (workstream 01, Phase-B; equivalents sparse-array / sentinel globals migration). One commit.

**Slice:** the sparse-array metadata and sentinel globals (`sa_root`, `sa_null`, `empty`, `max_reg_help_line`) → fields on `EngineState.scanner` (`ScannerState`).

**Pre-move grep checks (§11 step 2):**
- (a) C types: `int32_t sa_root[8]`, `memory_word sa_null`, `b32x2 empty`, `const char* max_reg_help_line`.
- (b) aliases: `#[link_name = "sa_root"] static mut fmt_sa_root: [i32; 8]` in `src/ini.rs`.
- (c) C readers: only `xetex-ini.c` (definition). No C code references them.
- (d) cross-module Rust extern redeclarations: none.
- format dump: yes, `sa_root` is format-serialized. Reconciled load/dump paths natively.

**Work done (1 commit):**
- Derived `Default` for `B32x2` struct in `src/mem_types.rs`.
- Implemented `Default` for `MemoryWord` union in `src/mem_types.rs` so that `ScannerState` can derive `Default` with a `MemoryWord` field.
- Added four fields (`sa_root`, `sa_null`, `empty`, `max_reg_help_line`) to `ScannerState` in `src/state.rs`.
- Removed their raw static mut declarations from `src/globals.rs`.
- Repointed all their getter/setter/accessor functions in `src/globals.rs` to delegate to `state.scanner` using `with_state` and `with_state_mut`.
- Removed their C definitions from `xetex/xetex-ini.c`.
- Removed `fmt_sa_root` raw linkage alias in `src/ini.rs` and replaced it with direct, safe `with_state` and `with_state_mut` calls.

**Newtype debts (§4):**
Added 1 row to the Deferred newtype worklist backlog in `01a-global-inventory.md`:
- `sa_root[]` (`[MemPtr; 8]`)

**Validation:** full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29/2/7/4** passes perfectly with no regressions!

**Metrics after this session:**
- `globals.rs` raw extern count: **60 → 56** (−4).
- `thin global accessors`: 823 total / 182 state-backed scalars/arrays (+4).

**For the next session (56+):**
- Migrate the **`mem` arena** (§5) — the hottest/highest-leverage ownership move and the real pivot into Phase C.
- Or migrate remaining ownership slices (`xdv_buffer`, `name_of_file*`, `native_text`).

---

## Idiom session 54 — 2026-05-30 — remaining 18 scanner scratch globals → ScannerState

**Type:** execution (workstream 01, Phase-B; remaining `scanner` scratch migration). One commit.

**Slice:** the remaining 18 box/packaging/scratch globals (`cur_box`, `after_token`, `cur_order`, `adjust_tail`, `pre_adjust_tail`, `total_stretch`, `total_shrink`, `just_box`, `cur_f`, `cur_c`, `cur_i`, `pack_begin_line`, `set_box_allowed`, `active_width`, `best_height_plus_depth`, `LR_ptr`, `LR_problems`, `cur_dir`) → fields on `EngineState.scanner` (`ScannerState`).

**Pre-move grep checks (§11 step 2):**
- (a) C types: `int32_t cur_box`, `int32_t after_token`, `glue_ord cur_order`, `int32_t adjust_tail`, `int32_t pre_adjust_tail`, `scaled_t total_stretch[4]`, `scaled_t total_shrink[4]`, `int32_t just_box`, `internal_font_number cur_f`, `int32_t cur_c`, `b16x4 cur_i`, `int32_t pack_begin_line`, `bool set_box_allowed`, `scaled_t active_width[7]`, `scaled_t best_height_plus_depth`, `int32_t LR_ptr`, `int32_t LR_problems`, `small_number cur_dir`.
- (b) aliases: none.
- (c) C readers: only `xetex-ini.c` (definition). No C code references them.
- (d) cross-module Rust extern redeclarations: none.
- format dump: none.

**Work done (1 commit):**
- Derived `Default` for `B16x4` struct in `src/mem_types.rs` so that `ScannerState` can derive `Default` with a `B16x4` field.
- Added all 18 fields to `ScannerState` in `src/state.rs`.
- Removed all 18 raw static mut declarations from `src/globals.rs`.
- Repointed all 18 getter/setter accessor pairs in `src/globals.rs` to delegate to `state.scanner` using `with_state` and `with_state_mut`.
- Removed all 18 C definitions from `xetex/xetex-ini.c`.

**Newtype debts (§4):**
Added 12 rows to the Deferred newtype worklist backlog in `01a-global-inventory.md`:
- `cur_box` (`MemPtr`/`Halfword`)
- `cur_order` (`GlueOrd`)
- `adjust_tail` (`MemPtr`/`Halfword`)
- `pre_adjust_tail` (`MemPtr`/`Halfword`)
- `total_stretch` (`[Scaled; 4]`)
- `total_shrink` (`[Scaled; 4]`)
- `just_box` (`MemPtr`/`Halfword`)
- `cur_f` (`FontId`)
- `active_width` (`[Scaled; 7]`)
- `best_height_plus_depth` (`Scaled`)
- `LR_ptr` (`MemPtr`/`Halfword`)
- `cur_dir` (`SmallNumber`)

**Validation:** full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29/2/7/4** passes perfectly with no regressions!

**Metrics after this session:**
- `globals.rs` raw extern count: **78 → 60** (−18).
- `thin global accessors`: 823 total / 178 state-backed scalars/arrays (+18).

**For the next session (55+):**
- Migrate the **`mem` arena** (§5) — the hottest/highest-leverage ownership move and the real pivot into Phase C (thread `&mut Mem` rather than bridge).
- Or migrate remaining ownership slices (`sa_root`, `xdv_buffer`, `name_of_file*`, `native_text`).

---

## Idiom session 53 — 2026-05-30 — DVI positioning & output variables → ShipoutState

**Type:** execution (workstream 01, Phase-B; DVI positioning/output scratch migration). One commit.

**Slice:** the 13 DVI positioning and output variables (`dvi_file`, `output_file_name`, `dvi_buf`, `dvi_limit`, `dvi_ptr`, `dvi_offset`, `dvi_gone`, `down_ptr`, `right_ptr`, `cur_s`, `dvi_h`, `dvi_v`, `dvi_f`) → Rust-owned fields on `EngineState.shipout` (`ShipoutState`).

**Pre-move grep checks (§11 step 2):**
- (a) C types: `rust_output_handle_t dvi_file`, `str_number output_file_name`, `eight_bits *dvi_buf`, `int32_t dvi_limit`, `int32_t dvi_ptr`, `int32_t dvi_offset`, `int32_t dvi_gone`, `int32_t down_ptr`, `int32_t right_ptr`, `int32_t cur_s`, `scaled_t dvi_h`, `scaled_t dvi_v`, `internal_font_number dvi_f`.
- (b) aliases: none. Genuinely promoted Rust statics.
- (c) C readers: none. Audit shows no other compiled non-macOS C algorithms reference them.
- (d) cross-module Rust extern redeclarations: none.
- format dump: none.

**Work done (1 commit):**
- Added all 13 fields to `ShipoutState` in `src/state.rs`. `dvi_buf` is now owned as a `Vec<u8>`.
- Removed all 13 module-level `pub static mut` declarations from `src/shipout.rs`.
- Defined thin module-local helper getter/setter functions in `src/shipout.rs` that delegate to the `'static` thread-local `EngineState.shipout` struct.
- Updated `initialize_shipout_variables` to natively allocate the buffer with `vec![0u8; (DVI_BUF_SIZE + 1) as usize]` on the thread-local state instead of raw `xmalloc`.
- Updated `deinitialize_shipout_variables` to natively release the buffer with `Vec::new()` instead of manually invoking `libc::free`.
- Updated `write_to_dvi` to pass `s.shipout.dvi_buf.as_ptr().add(a as usize)` to FFI instead of pointer arithmetic on raw pointers.
- Refactored `dvi_out`, `dvi_swap`, `movement`, `special_out`, and `pic_out` to use the helper functions and safe vector indexing/modifications instead of direct raw pointer arithmetic.

**Newtype debts (§4):**
Added 6 rows to the Deferred newtype worklist backlog in `01a-global-inventory.md`:
- `output_file_name` (`StrNumber`)
- `dvi_f` (`FontId`)
- `down_ptr` (`MemPtr`/`Halfword`)
- `right_ptr` (`MemPtr`/`Halfword`)
- `dvi_h` (`Scaled`)
- `dvi_v` (`Scaled`)

**Validation:** full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29/2/7/4** passes perfectly with no regressions!

**Metrics after this session:**
- Promoted Rust `pub static mut` globals outside `globals.rs`: **16 → 3** (-13: `dvi_file`, `output_file_name`, `dvi_buf`, `dvi_limit`, `dvi_ptr`, `dvi_offset`, `dvi_gone`, `down_ptr`, `right_ptr`, `cur_s`, `dvi_h`, `dvi_v`, `dvi_f`).
- `globals.rs` raw extern count: **78** (unchanged).

**For the next session (54+):**
- Migrate the **`mem` arena** (§5) — the hottest/highest-leverage ownership move and the real pivot into Phase C (thread `&mut Mem` rather than bridge).
- Or migrate remaining classified-but-unmigrated scratch: box/packaging scratch (`cur_box`, `adjust_tail`, etc.).

---

## Idiom session 52 — 2026-05-30 — eqtb hash table → Rust-owned `Vec<B32x2>`

**Type:** execution (workstream 01, Phase-B; the `eqtb`-cluster hash ownership slice). One commit.

**Slice:** the equivalents hash table `hash` (C-`malloc`d `b32x2 *`, size `1 + hash_top - hash_offset`) and its control scalars (`hash_extra`, `hash_top`, `eqtb_top`) → Rust-owned fields on `EngineState.eqtb` (`EqtbState`).

**Pre-move grep checks (§11 step 2):**
- (a) C types: `b32x2 *hash`, `int32_t hash_extra`, `int32_t hash_top`, `int32_t eqtb_top`. No newtype debt (plain `B32x2` and `int32_t`).
- (b) aliases: 4 total FFI aliases/helpers in `ini.rs` (`tre_hash_extra`, `tre_hash`, `tre_eqtb_top`, `tre_hash_top`, `tt_run_engine_allocate_yhash`, `tt_cleanup_free_yhash`, `tt_cleanup_yhash`) → ownership slice (§11 step 6).
- (c) C readers: only `xetex-ini.c` (definition). No C code references them.
- (d) cross-module Rust extern redeclarations: none.
- format dump: **yes** — undump_slice used in `ini.rs:2783-2790`.

**Work done (1 commit):**
- `EqtbState` gains `hash: Vec<B32x2>`, `hash_extra: i32`, `eqtb_top: i32`, `hash_top: i32` fields (empty/zero by default).
- 4 `globals.rs` accessors (`hash_s0`, `set_hash_s0`, `hash_s1`, `set_hash_s1`) repointed to index `s.eqtb.hash[(p as usize) - 514]`. No `unsafe` needed.
- `hash_extra()`, `eqtb_top()`, `hash_top()` and their setters repointed to state.
- Removed raw statics `hash`, `hash_extra`, `eqtb_top` from `globals.rs` `mod raw`.
- Completely removed `yhash` static, FFI allocation helpers `tt_run_engine_allocate_yhash`/`tt_cleanup_free_yhash`, and variables `hash`/`hash_extra`/`hash_top`/`eqtb_top` from C (`xetex/xetex-ini.c`).
- Replaced FFI `tt_run_engine_allocate_yhash()` and FFI alias cleanup with safe, native Rust `Vec::new()` and `vec![crate::mem_types::B32x2 { s0: 0, s1: 0 }; n]` allocations inside `src/ini.rs` (Initex and Format Load paths).
- Reconciled `undump_slice` for the hash table at format load using the proven **font-array pattern**: a state-derived mutable pointer `hash_ptr` is precomputed, then used with subtracting `514` from indices.
- Cleared `state.eqtb.hash = Vec::new()` inside `tt_cleanup()` and format load reset.

**Newtype debts (§4):**
- `hash[].s0` (link) -> `Halfword` / `MemPtr` (original C type: `halfword`)
- `hash[].s1` (text) -> `StrNumber` (original C type: `str_number`)
Added rows in the tracking backlog (noted, plain `B32x2` structure wrapper keeps them safe for now).

**Validation:** full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29/2/7/4** passes perfectly with no regressions! Format undump roundtrip fully verified.

**Metrics after this session:**
- `globals.rs` raw extern count **81 → 78** (−3: `hash`, `hash_extra`, `eqtb_top`).
- Raw `unsafe { }` text count **244 → 236** (−8).

**For the next session (53+):**
- Migrate the **`mem` arena** (§5) — the hottest/highest-leverage ownership move and the real pivot into Phase C (thread `&mut Mem` rather than bridge).
- Or migrate remaining classified-but-unmigrated scratch: DVI promoted Rust statics (`dvi_*`), box/packaging scratch (`cur_box`, `adjust_tail`, etc.).

## Idiom session 51 — 2026-05-30 — eqtb array → Rust-owned `Vec<MemoryWord>`

**Type:** execution (workstream 01, Phase-B; the `eqtb`-cluster ownership slice —
the format-dump-reconciliation centerpiece flagged by sessions 49–50). One commit.

**Slice:** the equivalents table `eqtb` (C-`malloc`'d `memory_word *`, size
`eqtb_top + 1`) → Rust-owned `Vec<MemoryWord>` on `EngineState.eqtb.data`.

**Pre-move grep checks (§11 step 2):**
- (a) C type: `memory_word *eqtb` (xetex-xetexd.h:386) — element `memory_word`,
  faithfully mirrored by `MemoryWord`; **no newtype debt** (checked).
- (b) aliases: two `#[link_name = "eqtb"]` (alloc/dump alias `ini.rs:1818`,
  cleanup alias `ini.rs:3675`) → ownership slice (§11 step 6).
- (c) C readers (non-macos): only `xetex-ini.c` (the definition, line 20); no C
  algorithm indexes `eqtb[]`.
- (d) cross-module Rust extern redeclarations: none beyond the two ini.rs aliases.
- format dump: **yes** — sparse region-based `dump_slice`/`undump_slice`.

**Work done (1 commit):**
- `EqtbState` gains `data: Vec<MemoryWord>` (empty in `Default`; allocated in
  `ini.rs`, dropped in `tt_cleanup`).
- 9 `globals.rs` accessors repointed to index `s.eqtb.data` via
  `with_state`/`with_state_mut` (the session-11 union-Vec idiom: the 3 reads keep
  an `unsafe` `MemoryWord` projection, the writes need none). `eqtb_is_null` →
  `data.is_empty()`.
- Format dump/undump reconciled with the **font-array pattern**: a state-derived
  base pointer (`eqtb_ptr`) is precomputed once outside `with_state` (dump,
  `as_ptr`) / `with_state_mut` (undump, `as_mut_ptr`), then used in the
  sparse-region `dump_slice`/`undump_slice` + the undump replication loop. The Vec
  never reallocates (thread-local owner), so the raw base stays valid across the
  loops' interleaved `eqtb_*` accessor calls (no live borrow held).
- initex (`ini.rs:3362`, was `xcalloc_array`) and format-load (`ini.rs:2559`, was
  `xmalloc_array`) allocations replaced by `vec![<zero MemoryWord>; eqtb_top+1]`
  (matching the `x*alloc_array(n)` = `n+1` inclusive size); the format-load
  undefined-CS replication loop now uses `eqtb_word`/`set_eqtb_word`.
- `tt_cleanup` (`ini.rs:3741`) and the format-load reset (`ini.rs:2520`) free via
  `s.eqtb.data = Vec::new()` instead of `libc::free`.
- Removed both `#[link_name = "eqtb"]` aliases, the `mod raw` extern static, and
  the C definition `memory_word *eqtb;` (xetex-ini.c:20) in the same commit.
  `eqtb_top` stays a C global (not this slice).

**Newtype debts (§4):** none — element type `memory_word` is faithful
(`MemoryWord`); checked, no worklist row.

**Validation:** full dashboard
`cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`
→ **29/2/7/4**. `cargo check` clean (pre-existing warnings only). The format-load
path (latex_goldens, TRIP, `prim_filedump`) exercises the sparse-region eqtb
dump/undump round-trip; TRIP + `save_restore` exercise eqtb save/restore writes.

**Metrics after this session:**
- `globals.rs` raw extern count **82 → 81** (−1: `eqtb`).
- Raw `unsafe { }` text count **250 → 244** (−6; the 3 union *reads* keep theirs).

**Tooling note:** the harness tool-output channel stalled for very long stretches
this session (results arrived in delayed bursts). The migration itself is
unaffected — every step was verified by the clean `cargo check`, a successful
link, and the green 29/2/7/4 dashboard.

**For the next session (52+):**
- **`hash`** is the next eqtb-cluster slice: the `yhash` offset-pointer trick
  (`hash = yhash - hash_offset`) + the C alloc helper
  `tt_run_engine_allocate_yhash` + `hash_offset`/cleanup must be reworked; the
  `Vec` accessor subtracts `hash_offset`.
- Then the **`mem` arena** (§5) — the hottest/highest-leverage ownership move and
  the real pivot into Phase C (thread `&mut Mem` rather than bridge).

---

## Idiom session 50 — 2026-05-30 — FFI Teardown of 148 Redeclared Rust-to-Rust Functions

**Type:** execution (workstream 03, FFI teardown and signature conversion).

**Goal:** convert 145 redeclared `#[no_mangle]` extern "C" functions into pure Rust calls, and clean up duplicate `extern "C"` declarations for all 148 functions (including the 3 C-FFI ones `D2Fix`, `Fix2D`, `tt_xetex_set_int_variable`) by replacing them with idiomatic `use crate::...` imports.

**Work done:**
- Automated parsing of `results.json` to identify definitions, calling sites, and declarations of all 148 redeclared functions.
- Signature Refactoring: Removed `#[no_mangle]` and `extern "C"` on all 145 pure Rust-only function definitions across files like `ext.rs`, `mem.rs`, `output.rs`, `errors.rs`, `math.rs`, `save.rs`, `string_pool.rs`, `shipout.rs`, `ini.rs`, `helpers.rs`, `input.rs`, `linebreak.rs`, etc.
- Safe FFI Cleanups: Replaced 148 duplicate FFI function declarations within `extern "C"` blocks in all calling files with precise `use crate::<module>::<name>;` imports at the top of the calling files. Used a semicolon-terminated regex `[^{;]*?;` to safely prevent matching across function bodies and definitions.
- Localized Type-Safety Fixes (21 compile errors resolved):
  - In `src/io.rs`, changed `u_close` and `input_line` to accept `*mut libc::c_void` and cast to `*mut UFile` internally. This completely resolved the type mismatch errors at their FFI call sites across the entire crate without modifying a single call site.
  - In `src/scan.rs`, resolved `E0255` for `overflow` by updating its local wrapper to call `crate::errors::overflow(s, n)` directly and removing the redundant top-level import.
  - In `src/scan.rs` and `src/ext.rs`, cast `new_character` arguments to `u16` (`cur_val() as u16` / `cur_chr() as u16`), `measure_native_glyph` boolean to `i32` (`use_glyph_metrics as i32`), and `print_raw_char` parameter to `u16`.
  - In `src/ext.rs` and `src/io.rs`, replaced invalid integer flags in `end_diagnostic` with correct `bool` values (`false`/`true`).
  - In `src/output.rs`, cast void pointer arguments of `print_utf8_str` to `*const u8`.

**Validation:**
- `cargo check`: pass, completely clean with 0 compile errors and no new warnings.
- `cargo test -p tectonic --test tex-outputs --test trip --test engine_rust --test latex_goldens`: pass (29/2/7/4). All format undump/dump paths and OpenType math logic verified.

**Metrics after this session:**
- `#[no_mangle]` exports: 579 → 434 (−145).
- duplicate FFI declarations removed: 148.

**For the next session:**
- Continue Workstream 01 execution: move the `eqtb` array proper into the Rust heap `Vec<MemoryWord>` on `EqtbState` as planned.

---
