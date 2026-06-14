# Idiomatization status

The authoritative tracker for Stage 2. Two sections: **plan
construction** (are the workstream plans written and ready?) and
**execution** (are the refactors done?). Update at every session end.

Last updated: 2026-06-14 (last session = session 183, **execution — large low-docs batch driven by a reusable Python transform: threaded the output.rs cmd/meaning/param printers, the `show_save_groups`/`show_activities`/`show_context` walkers, and the `print_file_line`/`begin_diagnostic`/`end_diagnostic` framing.** Cleared the entire session-182 "Next" list, then extended. Built `plan/idiom/thread_output_fn.py` (splits a print/show fn into a thin `with_state_mut` wrapper + `pub(crate) fn NAME_in(st: &mut EngineState, …)` core; rewrites print wrappers → `_in`, `with_state` accessors → direct `st.<cluster>.<field>`, RAW accessors left alone; balanced-paren + literal-aware) and `delete_fns.py` (brace-safe dead-fn removal). (1) `e8bedb2f`: `print_cmd_chr` (~850 lines, incl. retyping its `print_char_or_wide` closure to take `st`), `print_meaning`, `print_param`, `print_length_param`, `show_save_groups`, `show_activities`, `show_context`; added `x_over_n_in(&mut ErrorState)` (the global `x_over_n` silently wrote `tex_remainder`/`arith_error` via `with_state`, nest-panicking under `show_activities`'s borrow — caught by TRIP) + `globals::cur_list()` getter; removed ~14 dead global wrappers. (2) `c382c536`: `print_file_line`, `begin_diagnostic`, `end_diagnostic` — unblocks the diagnostic-warning cluster. Hazard sweep of all non-print/non-accessor calls found + fixed two re-entries (`x_over_n`, `length_global` → `string_pool::length(&st.strings,…)`). Validation: `fmt --check`; `cargo check`; dashboard **29/2/7/4** after each batch; rebuilt `target/release/tectonic`; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1`/`1905.00148v1`. Metrics: warnings **41** (held); `unsafe fn` **551 → 544**; `with_state(_mut)` call sites 1134. **Next:** thread the diagnostic-warning cluster — the capture pair (`capture_to_diagnostic`/`diagnostic_print_file_line`/`diagnostic_begin_capture_warning_here`, keeping global wrappers for `errors.rs` callers), then `show_cur_cmd_chr`/`bad_utf8_warning`/`group_warning`/`if_warning`/`file_warning` (their `begin/end_diagnostic_in` + `show_context_in` callees now exist); leave `char_warning`/`show_whatever` until their `error`/scanner bucket-D helpers thread.)

Last updated: 2026-06-13 (last session = session 182, **execution — large low-docs batch: threaded the print subsystem keystone, then the output.rs show-node walkers.** (1) `f0475cdd`: threaded the **print core** over `&mut EngineState` — `_in` cores for `print_char`/`print_raw_char`/`print_ln`/`print`, the cstr/nl/esc wrappers, `print_int`/`hex`/`scaled`/`roman_int`/`ucs_code`, `print_the_digs`, `warn_char`, `print_current_string` (direct `shipout`/`errors`/`strings`/`eqtb`/`config` field access); globals became thin `with_state_mut` wrappers. (2) `9d898677`: threaded `short_display`, `show_node_list` (+ `show_box`/`show_info`/`print_subsidiary_data`) and ~22 leaf print helpers (`print_native_word`/`print_cs`/`sprint_cs`/`print_file_name`/`print_spec`/`print_glue`/`print_rule_dimen`/`print_fam_and_char`/`print_delimiter`/`print_font_and_char`/`print_mark`/`show_token_list`/`token_show`/`print_size`/`print_write_whatsit`/`print_sa_num`/`print_mode`/`print_in_mode`/`print_group`/`print_skip_param`/`print_style`/`print_totals`), unblocking the largest mechanical vein (the show-walkers nest-panicked on `with_state`-bearing print calls). Per precedent the `_in` cores are **safe `fn`** with narrow `unsafe {}` only around FFI/raw-ptr/`CStr`; added `MemState::gr`; `show_node_list` converted via a scoped Python transform; removed 11 dead global wrappers + `breadth_max`/`depth_threshold`/`set_*`/`font_in_short_display`/`doing_special`/`set_trick_buf` accessors. Validation: `fmt --check`; `cargo check`; `node::` 48/48; dashboard **29/2/7/4** per commit; rebuilt `target/release/tectonic`; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1`/`1905.00148v1`. Metrics: warnings 41; `unsafe fn` **564 → 551**. **Next:** thread `show_activities` (top recon target, needs the `nest_*` cluster), then `show_save_groups`/`show_context`/`print_cmd_chr`/`print_meaning`/`print_param`/`print_length_param` — all now unblocked by the print `_in` cores.)

Last updated: 2026-06-13 (last session = session 181, **execution — bigger low-docs hyphenation batch with two explorer subagents for scouting.** Threaded `hyphenation.rs` trie-op creation and init/pattern insertion clusters over `&mut HyphState`: `new_trie_op_in`, `init_trie_in`, completed-pattern word insertion, and saved-hyphen-code insertion. Kept scanner/output diagnostics outside hyph borrows; overflow paths use the existing sentinel/`trie_abort` shape. Removed dead trie-op/trie-pack bridge accessors from `globals.rs` (hash/min setters, trie-op lang/val getters, hyf op setters, trie c/o/l/r getters, etc.); engine/format/linebreak live bridges remain. Explorer finding retained: output/shipout walkers need a primitive print-core threading slice first. Validation: `fmt --check`; `cargo check`; `node::` 48/48; dashboard **29/2/7/4**; rebuilt `target/release/tectonic`; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1`/`1905.00148v1`. Metrics: warnings 41; `unsafe fn` **566 → 564**. **Next:** either continue `hyphenation.rs` with short-borrow `new_hyph_exceptions` subclusters, or take the print-core slice (`print_char`/`print_ln`/string/numeric primitives behind wrappers) before `short_display`/`show_node_list`.)

Last updated: 2026-06-13 (last session = session 180, **execution — larger low-docs mechanical batch using two explorer subagents for scouting.** (1) `node.rs`/`packaging.rs`: added typed `WhatsitNode::native_text_ptr{,_mut}` and adopted `WhatsitNode`/`MarkNode`/`AdjustNode`/`InsNode` accessors in the already-threaded `hpack_in` native-word merge / adjust extraction path; no algorithm change. (2) `hyphenation.rs`: threaded the pure trie compression/packing helpers (`trie_node`, `compress_trie`, `trie_fix`, then `first_fit`/`trie_pack`) over `&mut HyphState`; `first_fit`/`trie_pack` now return a tiny overflow sentinel so `overflow("pattern memory")` still fires after the borrow drops; removed the now-dead `trie_taken`/`trie_min` bridge accessors from `globals.rs`. Validation: `cargo check` + `fmt`; `node::` 48/48; dashboard **29/2/7/4**; rebuilt `target/release/tectonic`; full corpus **637 passed / 2 diverged / 0 missing-golden**, only known `1905.00073v1`/`1905.00148v1`. Metrics: warnings 41; `unsafe fn` **568 → 566**. **Next:** subagent scout says the output/shipout walkers need a threaded print core first; the cleanest next mechanical work is continuing `hyphenation.rs` around `new_trie_op` (overflow-sentinel shape) or taking a deliberate print-core slice before `short_display`/`show_node_list`.)

Last updated: 2026-06-13 (last session = session 179, **execution — typed the last untyped list-node families (2 commits, low-docs per user request). (1) `12ea8d28`: built typed `MarkNode`/`AdjustNode`/`InsNode`/`UnsetNode` surfaces in `node.rs` and promoted `AnyNode::{Mark,Adjust,Ins,Unset}` from raw `MemIdx` to `NodePtr<K>`, so `classify()` confines each family's `from_idx_unchecked` to one audited arm; adopted the typed accessors in `flush_node_list`/`copy_node_list`/`hpack_in`/`vpackage_in`. `UnsetNode` deliberately omits `HasDimensions` (its +4 slot is `glue_shrink`, not `shift_amount`). **`Whatsit` is now the only raw `AnyNode` variant.** (2) built `SaNode` (eTeX sparse-array record: indexed `child(mem,i)` + `ref_count`) and fully typed the threaded `do_marks` with it (~17 raw sparse-array accesses removed; pointed-to token-list refcounts stay raw). Disasm gate skipped — all new accessors are already-ICF-proven shapes (session-150 precedent). 5 new `node::` tests (47/47). Validation: `cargo check`+`fmt --check`; dashboard **29/2/7/4** after each slice; rebuilt `target/release/tectonic`; corpus 637 passed / 2 diverged / 0 missing-golden, only known `1905.00073v1`/`1905.00148v1`. Metrics: warnings 41; `unsafe fn` 568; `from_idx_unchecked` 125 → 131 (+6 audited boundaries; node 79, math 36, linebreak 12, mem 3, packaging 1); **packaging method-form `+N` accesses 89 → 52 (−37)**. **Next: the only untyped `AnyNode` family left is Whatsit/native (rank 9 — variable-size, inline glyph data, FFI; the documented hardest/last family, a dedicated slice). Otherwise bucket-C/D threading: the print subsystem (unblocks shipout/output walkers), the `init_math`/`app_display`/`after_math` display-math trio, or the `pagebuilder`/`hyphenate`/`line_break` tails.**)

Last updated: 2026-06-13 (last session = session 178, **execution — threaded/typed the central math driver `mlist_to_hlist` across both passes (4 commits), after first committing the inherited 174–177 WIP as a clean baseline (`1dbd2808`). Pass-1: carved CHOICE_NODE (new `mlist_choice_node` over `classify_choice`/`ChoiceNode` + threaded `flush_node_list`) and GLUE_NODE (`GlueNode` accessors + threaded `delete_glue_ref`/`flush_node_list`) reswitch arms into typed `with_state` islands; extracted `mlist_track_box` (hpacked-box dims → max_h/max_d + typed `free_node`) and `mlist_advance` (noad step + style reset in one borrow) and applied to ProcessNucleus/CheckDimensions/DoneWithNoad; folded the ProcessNucleus nucleus store + script-presence check into one borrow. Pass-2: BIN fixup/list-reset prelude, move-and-detach arm, new-hlist splice+tail walk, inter-element glue spacing (threaded `new_glue`), penalty insertion (threaded `new_penalty`), and final link-read + threaded `free_node` all became typed islands. Bucket-D calls (`make_*_global`, `hpack`, native FFI, recursion) deliberately stay outside the borrows. Recon finding: no easy bucket-A/B work remains — the `output.rs` show-node walkers are bucket-C (their `print_*` calls are `with_state`-bearing, so a mem borrow across a print nest-panics). Validation: `cargo check`+`fmt --check`; `node::` 42/42; dashboard 29/2/7/4; rebuilt `target/release/tectonic`; corpus 637 passed / 2 diverged / 0 missing-golden, only known `1905.00073v1`/`1905.00148v1`; `git diff --check`. Metrics: warnings 41; `unsafe fn` 568; `from_idx_unchecked` total 125 (node 74, math 36, linebreak 12, mem 3, packaging 0); **`math.rs` raw `mem_*` bridge calls 343 → 276 (−67)**. **Next: `mlist_to_hlist`'s remaining head is bucket-D-heavy; the biggest unblocked lever is threading the print subsystem (`print_*` → state params) to unblock the `shipout`/`output` show-node walkers, else the `pagebuilder`/`hyphenate`/`line_break` tails.**)

Last updated: 2026-06-13 (last session = session 177, **execution — continued with another real bridge host by threading/typing the `make_math_accent_global` state islands. Added typed accent helpers for the initial noad/font snapshot, scripted-base reclean, native glyph setup around measurement callbacks, accent box dimension sync, and final top/bottom accent stack assembly. Kept `clean_box`, `vpackage`, OpenType variant/assembly callbacks, and native glyph measurement outside live state borrows. Kern allocation in the top-accent stack now uses the typed `KernNode` allocator; noad conversions are confined through a local `assume_noad` boundary. Validation: `cargo check`+`fmt --check`; `node::` 42/42; dashboard 29/2/7/4; rebuilt `target/release/tectonic`; final corpus reported 637 passed / 2 diverged / 0 missing-golden, only known `1905.00073v1` and `1905.00148v1`; `git diff --check`. Metrics: warnings 41; `unsafe fn` 568; `from_idx_unchecked` total 124 (packaging 0; node.rs 74, math 35, linebreak 12, mem 3). **Next: continue with another real bridge host: `mlist_to_hlist` itself, `make_op_global`, shipout/output walkers, or the `hyphenate`/`line_break` tail. No pointer-only cleanup.**)

Last updated: 2026-06-13 (last session = session 176, **execution — threaded `vsplit` as the coherent packaging state slice that session 175 deferred. Added an explicit `start_vsplit`/`vsplit_resume` core over `MemState`/`EqtbState`/`ScannerState`/`ShipoutState` plus a preserved legacy wrapper. The core now uses threaded `find_sa_element`, `delete_sa_ref`, `flush_node_list`, `do_marks`, `prune_page_top`, `vpackage_in`, `free_node`, and `delete_token_ref`, mutates `scanner.cur_mark`/`disc_ptr`/`sa_root` and `eqtb` directly, and classifies the source box as typed `NodePtr<VListNode>`. It reuses the resumable `vert_break` core, pausing through the wrapper for the infinite-shrinkage diagnostic/repair exactly like the legacy `vert_break` wrapper. `cur_val`/`cur_ptr` remain raw hot globals; Mark-family fields remain raw by design. Vpack diagnostics are carried out as `PackOutcome`s and printed after dropping the state borrow. Validation: `cargo check`+`fmt --check`; `node::` 42/42; dashboard 29/2/7/4; rebuilt `target/release/tectonic`; corpus reported 637 passed / 2 diverged / 0 missing-golden, only known `1905.00073v1` and `1905.00148v1`; `git diff --check`. Metrics: warnings 41; `unsafe fn` 568; `from_idx_unchecked` total 123 unchanged (packaging 0; node.rs 74, math 34, linebreak 12, mem 3). **Next: packaging’s `vert_break`/`vsplit` bridge island is threaded; continue with another real bridge host such as `mlist_to_hlist`, shipout/output walkers, `make_op_global`/`make_math_accent_global`, or the `hyphenate`/`line_break` tail. No pointer-only cleanup.**)

Last updated: 2026-06-13 (last session = session 175, **execution — threaded and typed the packaging `vert_break` global-accessor island. Split it into a resumable explicit-state core over `MemState`/`ScannerState` plus the preserved legacy bridge wrapper. The core no longer calls `mem_b16s1`/`mem_b32s1`/`active_width`/`best_height_plus_depth` globals; it dispatches through `MemState::classify`, uses typed HList/VList/Rule dimensions and Glue/Kern/Penalty/GlueSpec accessors, and writes `scanner.active_width` / `scanner.best_height_plus_depth` directly. Preserved the mid-loop infinite-shrinkage error ordering by pausing the core, dropping the state borrow for the original diagnostic/error, then reborrowing to copy the glue spec with threaded `new_spec`, set typed `shrink_order`, delete the old glue ref, and resume. Whatsit PIC/PDF and untyped Ins/Mark remain raw. `vsplit` was inspected but left for the next coherent slice because it crosses sparse-array, mark, `prune_page_top`, `vpackage`, and box-register bridge boundaries. Validation: `cargo check`+`fmt --check`; `node::` 42/42; dashboard 29/2/7/4; rebuilt `target/release/tectonic`; corpus 637 passed / 2 diverged / 0 missing-golden, only known `1905.00073v1` and `1905.00148v1`; `git diff --check`. Metrics: warnings 41; `unsafe fn` 568; `from_idx_unchecked` total 123 unchanged (packaging 0; node.rs 74, math 34, linebreak 12, mem 3). **Next: thread `vsplit` as a coherent state slice, likely pulling in the mark loop, sparse-array calls, `prune_page_top`, and `vpackage` outcome plumbing. Keep Unset/Ins/Mark/Adjust/Whatsit raw unless their typed surfaces are deliberately built.**)

Last updated: 2026-06-13 (last session = session 173, **execution — adopted `classify` (`AnyNode`) across the four packaging list-walkers, completing the session-172 "Next" item (`01c` §3). `flush_node_list`/`copy_node_list`/`just_copy`/`prune_page_top` now dispatch on `MemState::classify(p)` instead of a raw `match mem.b16s1(p)` over `*_FNL`/`*_CNL`/`*_PPT` constants. This **removed all 3 of packaging's `from_idx_unchecked`** (the `flush_node_list` GLUE arm binds the typed `NodePtr<GlueNode>` from `AnyNode::Glue`; its three noad arms collapse to one `AnyNode::Noad` arm sub-dispatching on `noad_type`). Per-walker fall-through preserved: `flush_node_list`'s match is total; `copy_node_list`/`prune_page_top` keep `_ => confusion(...)` and `just_copy` keeps `_ => None`, reached via `classify().map_err(...)?`/`.ok()`. Dead node-type consts + now-unused imports removed. Behaviour-preserving (each typed arm lowers to the same field access; `classify` `Err` maps to the prior catch-all). Validation: `cargo check`+`fmt --check`; `node::` 42/42; dashboard 29/2/7/4; release corpus exit 0, only known FP divergences `1905.00073v1`/`1905.00148v1` (no regression). Metrics: warnings 43→42; `unsafe fn` 568; `from_idx_unchecked` total 126→123 (packaging 3→0; node.rs 74, math 34, linebreak 12, mem 3). **Next: the threaded-function `classify`-adoption vein is EXHAUSTED — every remaining `match mem.b16s1` dispatch loop (`shipout`/`output`/`mlist_to_hlist`) reads through the `mem_b16s1` global, i.e. is unthreaded, so adopting `classify` needs threading the host first (bucket C/D). Levers: thread `mlist_to_hlist`, the shipout/output node-walkers, `vert_break`/`vsplit`, `make_op_global`/`make_math_accent_global`, the `hyphenate`/`line_break` tail — then fuse `classify` in the same slice (`01b` §3a). linebreak's `*_node()`/`from_idx_unchecked` are the intentional confinement layer, not classify targets.**)

Last updated: 2026-06-13 (last session = session 172, **execution — built the unified classification surface the plan specified but never landed (`01c` §3): `AnyNode` (typed `NodePtr<K>` per built family, raw `MemIdx` for not-yet-typed native/whatsit/unset/ins/mark/adjust) + `MemState::classify(p) -> Result<AnyNode, _>`, confining every built family's `from_idx_unchecked` to one audited site so consumers match `AnyNode` with zero caller-side `unsafe` and the static type inferred per arm. Disasm gate (`01c` §4): a comprehensive typed classify-dispatch folds to a jump table → direct field load, no enum materialized — tighter than the raw multi-arm `NodeType` match (123 vs 217 bytes). Adopted `classify` in the `hpack`/`vpackage` main loops, deleting 11 per-arm `from_idx_unchecked` (`packaging.rs` 14→3). Validation: `node::` 42/42; dashboard 29/2/7/4; release corpus exit 0, only known FP divergences `1905.00073v1`/`1905.00148v1` (no regression). Metrics: warnings 43; `unsafe fn` 568; `from_idx_unchecked` total 126 (node.rs 74 = the audited confinement layer + classify + per-family classifiers + allocs + tests). **Next clean `classify` adoptions: `flush_node_list`/`copy_node_list`/`just_copy` (convert the raw-`*_FNL`-constant match to `AnyNode`; noad arm sub-dispatches on `noad_type`, box arm groups HList|VList|Unset), then `mlist_to_hlist` (math, 34 sites). linebreak's `from_idx_unchecked` are the intentional `*_in` confinement helpers, not classify targets. Threading levers from session 171 still stand: `vert_break`/`vsplit`, `make_op_global`, `hyphenate`/`line_break` tail.**)

Last updated: 2026-06-13 (last session = session 171, **execution — landed + cleaned + committed the inherited hpack/vpackage threading WIP (`hpack_in`/`vpackage_in` over explicit state, diagnostics split into borrow-dropping `PackDiag` wrappers, `classify_box`/`VListNode::alloc` adopted; dead `total_stretch`/`total_shrink` setters and orphaned node-type constants removed). Per user design feedback, deleted the never-wired `AnyBox::from_node_type`/`RuleNode::from_node_type` helpers: moving `from_idx_unchecked` into a helper does not eliminate the unsafe tag→type assertion — `classify_box`/`classify_*` are the single audited boundary that yields a typed handle, after which each match arm is statically typed with zero further unsafe. Typed more box/glue/noad/choice consumers (`flush_node_list` glue arm; math `apply_math_limit_switch`/`stack_glue_into_box`/`build_ext_delimiter_box`/`first_script_glyph_step`/`rebox`; shipout `finish_{h,v}list_rule`). Validation: `node::` 41/41; dashboard 29/2/7/4; release corpus exit 0, only known FP divergences `1905.00073v1`/`1905.00148v1` (no regression). Metrics: warnings 43; `with_state` 1156; `unsafe fn` 568; `#[no_mangle]` 11; bare `[pqr]: i32` 347. **Next: the mechanical consumer vein is largely exhausted; remaining gains require threading large bridge-based functions — `vert_break`/`vsplit` (bucket D: mid-loop print+continue blocks a single borrow), `make_op_global`/`make_math_accent_global`, `hyphenate`/`line_break` tail (65/47 bridge sites), and the unthreaded shipout/pagebuilder/output/main_control/scan bodies. No more pointer-only helper passes.**)

Last updated: 2026-06-12 (last session = session 170, **execution - typed shipout records + threaded/typed `post_line_break`, low-docs.** Added `EdgeNode` and untagged `MovementNode` typed surfaces and adopted them in shipout `new_edge`, `prune_movements`, and `movement`. Split `post_line_break` into borrowed prefix/suffix phases around the still-global `hpack`, threading `MemState`/`ScannerState`/`LinebreakState`/`EqtbState`/`FontState`/`ConfigState` and typing disc/glue/glue-spec/kern/math/passive/box operations directly instead of adding bridge helpers. Removed dead one-line global wrappers/accessors (`set_just_box`, margin-kern globals, LR-stack global, linebreak width/indent/best-line getters). Caught and fixed the split regression where display math needed `scanner.just_box` restored before `math::init_math`. Validation green: `cargo check -p tectonic_engine_xetex`; `cargo fmt --check -p tectonic_engine_xetex`; `node::` 41/41; dashboard 29/2/7/4; `git diff --check`. Release corpus after rebuilding `target/release/tectonic`: 637 passed / 2 diverged / 0 missing-golden, only known `1905.00073v1` and `1905.00148v1`. Warnings 43; `with_state` 1155; `unsafe fn` 568; direct `#[no_mangle]` grep 11; current bare `[pqr]: i32` grep 341. **Next: `hpack` is now the main boundary; continue with a large thread+type slice through `hpack`/packaging or another closed post-line-break dependency. No pointer-only helper passes.**)

Last updated: 2026-06-12 (last session = session 169, **execution — inline delta fields + post-line-break entry, low-docs.** Removed the index-based `delta_node_width_in` / `set_delta_node_width_in` wrappers and inlined direct `DeltaNode` `dwidth`/`dstretch*`/`dshrink` access in already-threaded `try_break` and `deactivate_node`. Threaded the first `post_line_break` island, `reverse_best_breaks`, over `MemState` + `LinebreakState` and typed it through `ActiveNode`/`PassiveNode`; deleted the now-dead `globals::best_bet` one-line bridge. Validation green: `cargo check -p tectonic_engine_xetex`; `node::` 41/41; dashboard 29/2/7/4. Release corpus after rebuilding `target/release/tectonic`: 637 passed / 2 diverged / 0 missing-golden, only known `1905.00073v1` and `1905.00148v1`. Warnings 43; `with_state` 1162; `unsafe fn` 571; direct `#[no_mangle]` grep 11; current bare `[pqr]: i32` grep 343. **Next: continue `post_line_break` only in closed slices that can thread state and type nodes together; `hpack` remains a boundary until it is threaded.**)

Last updated: 2026-06-12 (last session = session 167, **execution — typed `linebreak` main-loop consumers, low-docs.** Continued from the existing uncommitted linebreak WIP. Added typed context-boundary helpers for already-classified linebreak/main-list families and used them immediately in threaded `try_break` and protrusion helpers: discretionary widths, trailing discard widths, glue-spec stretch/shrink, math width, kern width/subtype, ligature font/char, and passive/active/delta allocation setup now go through typed handles. Added typed active/delta list-link accessors in `node.rs`. Native/whatsit left raw for its own family slice. Dashboard green: `node::` 41/41, `tex-outputs` 29/29, `trip` 2/2, `engine_rust` 7/7, `latex_goldens` 4/4. Release corpus checked after rebuilding `target/release/tectonic`: 637 passed / 2 diverged / 0 missing-golden, with only known divergences `1905.00073v1` and `1905.00148v1`. Warnings 43; `with_state` 1162; `unsafe fn` 571; direct `#[no_mangle]` grep 11; bare `[pqr]: i32` params 349. **Next: finish remaining `line_break` main-loop raw islands or move into `post_line_break`; native/whatsit typing remains a separate family-sized slice.**)

Last updated: 2026-06-10 (last session = session 163, **execution — subscr/supscr rename slice + bulk noad & glue-spec consumer typing (large, low-docs, 5 commits). (1) `e8c364db` **fixed the session-162 naming landmine**: offset-preserving swap of the inverted `NodePtr<Noad>` accessor names so `supscr_*`=+2, `subscr_*`=+3 match TeX (definitions in node.rs + every call site in math.rs/mem.rs swapped together; behaviour-preserving, compiler-checked). (2) `818b95a2` typed noad consumers in math.rs (`make_over`/`make_under_finish`/`make_vcenter`/`make_fraction_finish` nucleus writes; `append_script_box`/`finish_subsup_stack_global`/`finish_superscript_branch_global`/`make_scripts_global`/`make_left_right_global` reads). (3) `20c6283b` typed noad consumers in packaging.rs `flush_node_list` (standard + fraction noad arms). (4) `1ff467c7` typed the glue-spec `ref_count` trap at 4 packaging.rs sites (just_copy, copy_node_list ×2, flush_node_list) via `NodePtr<GlueSpecNode>::from_glue_ptr`. Per-family closure rule honoured; bridge-mixed hosts (`make_op_global`, `make_math_accent_global`, `mlist_to_hlist`), the UNSET-sharing box arms, and linebreak's `*_in` helper layer deliberately left raw. `node::` 38/38; dashboard 29/2/7/4; corpus 637 passed / 2 diverged / 0 missing (the 2 are the known FP/native-text divergences `1905.00073v1`+`1905.00148v1`, matches baseline); warnings 43; `with_state` 1197, `unsafe fn` 571 (unchanged). **Next: the cheap consumer vein in already-threaded fns is largely exhausted — next is THREADING work: thread `make_op_global`/`make_math_accent_global` end-to-end, or `hpack`/`vpackage`, or build a rule-9/11 family (native/whatsit, or page/linebreak internals).**)

Last updated: 2026-06-10 (last session = session 162, **execution — bulk box-dimension consumer typing in `math.rs` (large mechanical batch, 2 commits, low-docs per user request). Retyped ~21 raw `mem.b32s1(box+N)` accessors across 10 already-threaded functions to typed `AnyBox`/`NodePtr` dimension accessors (`classify_box` + width/depth/height/shift_amount/list_ptr/link): commit 1 (`2ff59e2f`) `add_script_space_and_restore_font`, `initial_script_shifts`, `finish_subsup_stack`, `finish_subscript_only`, `prepare_superscript`, `finish_superscript_branch`, `make_under_finish`, `make_vcenter`; commit 2 (`adbd33f9`) `char_box`, `make_fraction`. Method: match offset→node.rs accessor; each function's box family fully closed, other families left raw. `math.rs` method-style accessors 145→124. **⚠ FOUND: node.rs `NodePtr<Noad>` subscr/supscr names are INVERTED vs TeX — `subscr_*`=offset+2, `supscr_*`=offset+3, but TeX/this-build has supscr=+2, subscr=+3. Offset-consistent (no behaviour bug) but a naming landmine; left all noad q+N reads raw this session. Next session should do an atomic subscr↔supscr rename slice (swap method names + every call site, offset-preserving) before adopting noad accessors.** `node::` 38/38; dashboard 29/2/7/4; warnings 44→43; metrics stable (`with_state` 1198, `unsafe fn` 571). **Next: the subscr/supscr rename slice; then box-dim consumers once their hosts (`make_op_global`, `make_math_accent`) are fully threaded.**)

Last updated: 2026-06-08 (last session = session 161, **execution — MarginKernNode implementation and `attach_hkern_to_new_hlist` refactoring. Added `MARGIN_KERN_NODE` (type 40) and `MARGIN_KERN_NODE_SIZE` (3) constants to `src/consts.rs`. Defined `MarginKernNode` marker, its `alloc`/`width`/`set_width` implementations, `HasNodeHeader` implementation, and `MemState::classify_margin_kern` in `src/node.rs`. Added address getters (`nucleus_addr`, `subscr_addr`, `supscr_addr`) to `NodePtr<Noad>`. Refactored `new_margin_kern` in `src/mem.rs` to use `MarginKernNode` and typed accessors. Refactored `attach_hkern_to_new_hlist` and `attach_hkern_to_new_hlist_global` in `src/math.rs` to adopt typed node pointers. `node::` 38/38; dashboard 29/2/7/4; warnings 44; metrics stable (`with_state` 1195, `unsafe fn` 570, direct `#[no_mangle]` grep 11). **Next: implement safe `maybe_attach_script_kern` core.**)

Last updated: 2026-06-08 (last session = session 160, **execution — typed math list and radical consumers adoption. Added `noad_type` to `NodePtr<Noad>`, and `set_width`, `set_depth`, `set_height`, `set_shift_amount`, `list_ptr`, `set_list_ptr`, `link`, and `set_link` to `AnyBox`. Refactored `fin_mlist`, `build_choices_finish`, `overbar`, `make_radical_finish`, and `make_radical_prepare_delimiter` in `src/math.rs` to use typed getters and setters from `NodePtr<KernNode>`, `NodePtr<RuleNode>`, `NodePtr<Noad>`, and `AnyBox`. `node::` 34/34; dashboard 29/2/7/4; warnings 43; metrics stable (`with_state` 1195, `unsafe fn` 570, direct `#[no_mangle]` grep 11). **Next: continue implementing typed consumers in math/packaging (e.g. dimension-reading leaves) or remaining raw constructor families.**)

Last updated: 2026-06-08 (last session = session 159, **execution — typed glue spec constructors and consumers adoption. Refactored `new_spec`, `new_skip_param`, and `delete_glue_ref` in `src/mem.rs` to use typed getters and setters from `NodePtr<GlueSpecNode>` instead of raw macro reads and writes. Also refactored `math_glue_global` and `math_kern_global` in `src/math.rs` to use typed accessors. Cleaned up unused imports `SMALL_NODE_SIZE` from `src/mem.rs` and `get_node` from `src/math.rs`. `node::` 34/34; dashboard 29/2/7/4; warnings 42; metrics stable (`with_state` 1195, `unsafe fn` 570, direct `#[no_mangle]` grep 11). **Next: continue implementing typed consumers in math/packaging (e.g. dimension-reading leaves) or remaining raw constructor families.**)

Last updated: 2026-06-08 (last session = session 158, **execution — typed math noad consumers adoption. Added generic `link`/`set_link` to `NodePtr<K>` for all header-bearing families. Added character/family getters and setters on `Noad` for math fields (nucleus, subscr, supscr) and accent character field. Refactored `make_ord_body`, `make_ord_global`, and `finish_accent_math_field` to adopt the typed interfaces. `node::` 34/34; dashboard 29/2/7/4; warnings 44; metrics stable (`with_state` 1195, `unsafe fn` 570, direct `#[no_mangle]` grep 11). **Next: continue implementing typed consumers in math/packaging (e.g. dimension-reading leaves) or remaining raw constructor families.**)

Last updated: 2026-06-08 (last session = session 157, **execution — typed noad constructor adoption. Added the unified `Noad` typed surface with multiple allocators for standard noad (OrdNoad..RightNoad), radical, accent, and fraction variants. Added getters and setters for math fields (nucleus, subscr, supscr), fraction thickness, and accent char. Adopted the surface in `new_noad`. `node::` 34/34; dashboard 29/2/7/4; warnings 42; metrics stable (`with_state` 1196, `unsafe fn` 571, direct `#[no_mangle]` grep 11). **Next: `new_lig_item` needs a non-header lig-stack record if typed, or begin implementing typed consumers in math/packaging.**)

Last updated: 2026-06-08 (last session = session 156, **execution — typed
style/choice constructor adoption. Added `StyleNode` and `ChoiceNode` typed
surfaces with allocators; `ChoiceNode` models display/text/script/scriptscript
nullable branch pointers. Adopted them in `new_style` and `new_choice`.
`node::` 32/32; dashboard 29/2/7/4; warnings 42; metrics stable
(`with_state` 1196, `unsafe fn` 571, direct `#[no_mangle]` grep 11). **Next:
`new_lig_item` needs a non-header lig-stack record if typed; otherwise start
the noad constructor surface (`new_noad`) or typed consumers in math/packaging.**)

Last updated: 2026-06-08 (last session = session 155, **execution — typed
glue/math constructor adoption. Added `NodePtr::<GlueNode>::alloc` (preserving
this build's `MEDIUM_NODE_SIZE` glue allocation) and adopted it in
`new_glue`/`new_param_glue`, including typed glue-spec refcount bumps. Added
the narrow `MathNode` surface for `MATH_NODE` width/subtype and adopted it in
`new_math`. `node::` 28/28; dashboard 29/2/7/4; warnings 42; metrics:
`with_state` grep 1196, `unsafe fn` 571, direct `#[no_mangle]` grep 11.
**Next: remaining raw constructor families (`new_lig_item`, noad/style/choice)
or typed consumers in packaging/math/shipout now that common list-node
constructors are typed.**)

Last updated: 2026-06-08 (last session = session 154, **execution — typed
rule/ligature/disc allocation plus `append_to_vlist` box typing. Added
`RuleDimen` + `RuleNode`; typed `new_rule` and `fraction_rule`. Added
`LigatureNode`; typed `new_ligature`. Added `DiscNode`; typed `new_disc`. Added
`AnyBox` dimension helpers and retyped `append_to_vlist` to take `AnyBox`, with
classification in the legacy wrapper. `node::` 25/25; dashboard 29/2/7/4;
warnings 42; metrics unchanged (`with_state` 1197, `unsafe fn` 571, bare
`p/q/r: i32` params 379, direct `#[no_mangle]` grep 11). **Next: more typed
consumers now that rule/disc/ligature exist, or remaining raw constructor
families (`new_math`, `new_glue`, lig-item/noad).**)

Last updated: 2026-06-08 (last session = session 153, **execution — box-family
typed `alloc` adoption for `new_null_box`. 1 code slice. Added `HasNodeHeader`
for real list-node families only (`HListNode`, `VListNode`, `GlueNode`,
`KernNode`, `PenaltyNode`) with shared `subtype`/`set_subtype`, deliberately
excluding `GlueSpecNode` because its word 0 is ref_count/order data, not a
`NODE_type` header. Deduped the per-family subtype methods from glue/kern/
penalty. Added `NodePtr::<HListNode>::alloc(mem) -> Option<Self>`: wraps
`get_node(BOX_NODE_SIZE)`, writes `NODE_type = HLIST_NODE`, and confines the
family's audited `from_idx_unchecked` construction site. Adopted it in the
already-threaded `new_null_box` (`src/mem.rs`), now fully typed: typed alloc +
typed subtype/dimension/list_ptr/glue setters; legacy return remains `i32` and
`ALLOC_FAILED`/`_global` overflow behavior is unchanged. New `node::` test
asserts the carved hlist classifies back as `AnyBox::HList` and has writable
typed fields. Warnings 42→42; `node::` 18→19; metrics unchanged by existing
grep commands (`with_state` 1197, `unsafe fn` 571, bare-`i32` node-ptr params
379, `#[no_mangle]` direct grep 11). New audited unsafe construction site +1
for `HListNode::alloc`, matching the per-family alloc pattern. Dashboard
29/2/7/4. **Next: rank-5 rule (`new_rule` waiting; needs a
running-dimension wrapper for `Scaled`-or-`NULL_FLAG`), or box-dimension-reading
leaves in `packaging.rs`/`shipout.rs`; `append_to_vlist` is already threaded
and wants `AnyBox` dimension helpers because `b` can be hlist or vlist.**)

7: Last updated: 2026-06-07 (last session = session 152, **execution — extended the typed `alloc` pattern to the kern and penalty families: the FIRST *type-stamping* allocators (session 151's `glue_spec` alloc stamps nothing — a glue_spec carries no `NODE_type`). 2 commits (one family each, A.12). Added `NodePtr::<KernNode>::alloc(mem) -> Option<Self>` and `NodePtr::<PenaltyNode>::alloc` (`src/node.rs`): each wraps `get_node(MEDIUM_NODE_SIZE)`, **writes `NODE_type`** (`KERN_NODE`/`PENALTY_NODE` — exercising the documented-but-never-run `set_b16s1(NODE_type)` half of `01c` §8), then constructs the handle via the family's single audited `from_idx_unchecked`; `None` = the §5 arena-exhaustion sentinel mapped to `overflow()` by the `_global` wrapper after the borrow drops. Added a small `PenaltyNode::set_subtype` (the shared-header b16.s0 the C `new_penalty` zeroes). Adopted both in their **already-threaded** real consumers `new_kern`/`new_penalty` (`src/mem.rs`), now **fully typed + `unsafe`-free**: `let Some(p) = NodePtr::<K>::alloc(mem) else {{ return ALLOC_FAILED }}; p.set_subtype(..); p.set_width/penalty(..); p.idx().0` — zero raw `mem.set_*` indexing, the hand-rolled `get_node`+raw-stamp confined into `alloc`. `KERN_NODE`/`PENALTY_NODE` imports net-moved mem.rs→node.rs. 2 new `node::` tests (16→18) each assert the carved handle **classifies back** (`classify_kern`/`classify_penalty` → `Some`), directly verifying the type-stamp the glue_spec test couldn't. Behaviour-preserving (each typed setter/alloc lowers identically). Warnings 42→42; `node::` 18/18; `with_state` 1197, `unsafe fn` 571, `#[no_mangle]`, bare-`i32` node-ptr params 379 all unchanged (the constructors still *return* `i32` → no param metric moves; the win is the typed surface + the confined `from_idx_unchecked`). Net `unsafe` blocks +2 (the two `alloc` audited sites — the confined per-family construction site, not a leak). Dashboard 29/2/7/4. Three families (glue_spec, kern, penalty) now share the `alloc` shape; per A.6 do NOT generalise to one `alloc<K>()` yet (it needs a per-`K` size+type-code trait — revisit when box/rule adopt it). **Next: `new_null_box` (box family) alloc adoption — but it needs a box `set_subtype`; `subtype` is the rank-0 *shared header* field, so consider a single universal `subtype`/`set_subtype` on `NodePtr<K>` for all `K` + deduping the per-family copies (own slice — a blanket `impl<K: NodeKind>` collides with the existing inherent per-family methods unless removed together). Also rank-5 rule (`new_rule` waiting; needs the running-dimension `Scaled`-or-`NULL_FLAG` wrapper), and box-dimension-reading leaves in `packaging.rs`/`shipout.rs` (`append_to_vlist` fully threaded; `b` is hlist-or-vlist → needs `AnyBox` dim helpers) for the biggest §8 param-metric win.** prior session 151, **execution — typesafe node allocation (answering a design question: "shouldn't we allocate a node of the correct type type-safely instead of an unsafe assertion?"). 1 commit. Added `NodePtr::<GlueSpecNode>::alloc(mem) -> Option<Self>` (`src/node.rs`): wraps the existing `get_node(GLUE_SPEC_SIZE)`, returns the typed handle, `None` = arena-exhaustion sentinel (`01b` §5). This **confines the family's one `from_idx_unchecked` to a single audited site** — the "node being initialised has no NODE_type to classify" boundary — so call sites never write `unsafe`. Rewrote `math_glue` to allocate through it: it is now **`unsafe`-free** (was `unsafe { from_idx_unchecked(MemIdx::new(get_node(...)))} ` + manual ALLOC_FAILED check → `alloc(mem).ok_or(Overflow)?`). Keeps TeX's free-list arena (`01c` §8 NON-GOAL — typed lift of the C `new_*` constructors, NOT a new allocator; no per-node heap alloc). New unit test (16 in `node::`) sets up a minimal single-free-block arena and checks `alloc` carves a usable, in-bounds, writable handle. Design discipline (recorded in `01c` §8): the pattern extends per family as real consumers appear; a type-stamping family's `alloc` also `set_b16s1(NODE_type)`; do NOT generalise to a single `alloc<K>()` until ≥2 families share it (A.6). Companion Q (subtype-as-enum) deferred by design: glue subtype is used arithmetically (`subtype = skip_param + 1`, mem.rs:540/620), so a blanket enum is unfaithful (A.7); keep `u16`, add per-family tag enums only where verified (kern qualifies, glue does not). Warnings 42→42; `node::` 16/16; `unsafe` block net-moved from math.rs into the one node.rs site; `with_state`/`unsafe fn`/`#[no_mangle]` unchanged. Dashboard 29/2/7/4. **Next: continue fused adoptions (box-dimension-reading leaves in `packaging.rs`/`shipout.rs` are the biggest §8 metric win), and/or build rank-5 rule (first sentinel-bearing family — running-dimension wrapper for `Scaled`-or-`NULL_FLAG` dims). Extend the typed `alloc` pattern to kern/penalty/box when a real allocating consumer threads.** prior session 150, **execution — the FIRST fused `01b` §3a threading+typing adoptions + a recon-discovered glue_spec family fix. 3 commits. This is the milestone the foundation was built for: the typed `NodePtr<K>` layer now has live consumers, not just unit tests. (1) `math_kern` (`math.rs`, already `&mut MemState`-threaded, reached from `mlist_to_hlist`'s `KERN_NODE` arm) — retyped its `p: i32` node ptr to `NodePtr<KernNode>`; the two raw writes became `p.set_width(mem, Scaled)` / `p.set_subtype(mem, EXPLICIT)`; the classification boundary is the `math_kern_global` bridge wrapper whose sole caller is the `KERN_NODE` match arm, so `from_idx_unchecked` is justified. (2) RECON during (1) found the glue_spec family was **missing its `width` field** (+1.b32.s1): the session-147 field map omitted it because `width` shares the generic `BOX_width` offset (no dedicated `GLUE_SPEC_width` C macro), but `new_spec`/`math_glue` use `mem[p+1]` as the spec width — added `NodePtr::<GlueSpecNode>::width`/`set_width` + test (same +1.b32.s1 Scaled shape already ICF-proven, no new disasm run); corrected `01c` §1 table + family doc. (3) `math_glue` (`math.rs`) — its freshly `get_node`'d glue_spec retyped to `NodePtr<GlueSpecNode>`, all 5 field writes via typed setters; the "construct a handle to a node I'm initializing" boundary (fresh node has no NODE_type to classify → `from_idx_unchecked`, not `classify_*`). Both adoptions are behavior-preserving (each typed setter lowers to the identical `set_b32s1`/`set_b16s*`); validated the full dashboard on each live-path commit. Metric (`01b` §8): bare-`i32` `[pqr]: i32` params 380→**379** (math_kern's `p` retyped; trend down). Warnings 42→42; `node::` 15/15; `with_state`/`unsafe fn`/`#[no_mangle]` unchanged. Dashboard 29/2/7/4. **Next: continue fused adoptions (more box/glue/kern/penalty-reading leaves in `math.rs`/`packaging.rs`/`shipout.rs`/`linebreak.rs` — grep for already-`&mut MemState`-threaded fns with a single-kind node-ptr param), AND/OR build rank-5 rule (first sentinel-bearing family — needs a running-dimension wrapper for `Scaled`-or-`NULL_FLAG` width/depth/height). The cheapest adoptions retype an already-threaded fn's single-family node ptr + swap raw `mem.bXXsY(p+N)` for typed methods in one commit.** prior session 149, **execution — `01c` Stage 3 rank-3 + rank-4: the kern and penalty `NodePtr<K>` families. 2 code commits (one family each, A.12) + 1 docs commit. RECON FIRST verified offsets against `xetex-xetexd.h:283` (penalty) + `new_kern`/`new_penalty` in `src/mem.rs` (kern width = mem[p+1].b32.s1, both allocate MEDIUM_NODE_SIZE=3 — a recon finding: the §1 table's penalty "small (2)" is generic-TeX; SyncTeX makes them medium here). Added to `src/node.rs`: `KernNode` (NODE_type 11 — concrete impls `subtype` 0.b16.s0, `width` +1.b32.s1 → `Scaled`; **concrete, NOT `HasDimensions`** — a kern has no depth/height/shift at +2..+4, so reusing the box trait would falsely expose them) + `MemState::classify_kern`; `PenaltyNode` (NODE_type 12 — `penalty` +1.b32.s1 → **`i32` NOT `Scaled`**, the trap: same slot as box/kern width but an integer breakpoint penalty, encoded in the return type + a test asserting `let v: i32 = pen.penalty(..)`) + `MemState::classify_penalty`. 6 new unit tests (15 total in `node::`, all pass): offset round-trips, the penalty-is-i32-not-Scaled distinction, classify dispatch + invalid-code rejection. **DISASM GATE PASSED** on both new hot accessors: `kern.width` (Scaled +1.b32.s1) ICF-folded to raw `mem.b32s1(p+1)` (size 0x28); `penalty.penalty` (raw i32 +1.b32.s1) folded to the raw read (size 0x28). Harnesses removed before commit, recorded in `01c` §4. Warnings 42→42 (still `#[allow(dead_code)]`); `with_state`/`unsafe fn`/`#[no_mangle]` unchanged; behavior-neutral (additive). Dashboard 29/2/7/4, `node::` 15/15. **Next: continue Stage 3 — rank 5 = rule (the first NON-trivial family: width/depth/height at +1/+2/+3 are `Scaled`-OR-`NULL_FLAG` *running dimensions*, NULL_FLAG = -0x40000000 `xetex-constants.h:16`; needs a running-dimension wrapper type — e.g. `enum RuleDimen { Running, Set(Scaled) }` — before its fields can be typed; this is the slice where the typed layer first models a sentinel-bearing field). Then disc/ligature/math/native. OR the first fused `01b` §3a adoption: thread a small box/glue/kern/penalty-reading leaf to `&mut MemState` AND retype its node ptr — proves type-as-you-thread end-to-end + starts driving the bare-i32 node-ptr-param metric (01b §8) down; a leaf in `packaging.rs`/`shipout.rs`/`linebreak.rs` reading dims/penalties under a single `mem` borrow is a good first target.** prior session 148, **execution — `01c` Stage 3 rank-2: the glue/glue_spec `NodePtr<K>` family. 1 code commit (+ docs). RECON FIRST verified every offset against `xetex-xetexd.h:243-244,297-301` + sizes `xetex-constants.h` (GLUE_SPEC_SIZE=4, SMALL_NODE_SIZE=2, GLUE_NODE=10) before building. Added the glue family to `src/node.rs`: `GlueNode` (concrete impls — `subtype` 0.b16.s0, `glue_ptr` +1.b32.s0 → `MemPtr`, `leader_ptr` +1.b32.s1 → nullable `MemPtr`, `glue_spec()` classify helper) + `GlueSpecNode` (`ref_count` 0.b32.s1 → **`i32`, NOT a link — the TRAP, named `ref_count` not `link`**; `stretch_order` 0.b16.s1 / `shrink_order` 0.b16.s0 → `u16`; `stretch` +2.b32.s1 / `shrink` +3.b32.s1 → `Scaled`). Classification differs from box: glue NODE via `MemState::classify_glue` (NODE_type == GLUE_NODE = 10); glue_spec via `NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr)` — a from-`MemPtr` boundary, NOT a node_type branch (a glue_spec carries no type code; its `0.b16.s1` word holds `stretch_order`, and it is only ever reached through a glue node's glue_ptr / a param-glue pointer). No new capability traits (glue fields are family-specific → concrete impls, per the recipe). 5 new unit tests (9 total in `node::`, all pass): offset round-trips into the C-exact raw subfields, the ref_count-is-not-a-link distinction, glue_ptr/leader_ptr nullability, classify_glue dispatch + invalid-code rejection. **DISASM GATE PASSED (strongest form):** release `objdump -t` shows typed `glue_ptr` (+1.b32.s0) ICF-folded to the same section/address as raw `mem.b32s0(p+1)` (size 0x27) AND typed `stretch` (+2.b32.s1) folded with raw `mem.b32s1(p+2)` (size 0x29) — byte-identical machine code, provably zero-cost. Harness removed before commit, result recorded in `01c` §4. Warnings 42→42 (still `#[allow(dead_code)]`, nothing consumes it yet); `with_state`/`unsafe fn`/`#[no_mangle]` unchanged; behavior-neutral (additive types). Dashboard 29/2/7/4, `node::` 9/9. **Next: continue Stage 3 — rank 3 = kern (trivial, one `Scaled` at +1.b32.s1), then penalty (TRAP: +1.b32.s1 is `i32` NOT `Scaled`), then rule (TRAP: width/depth/height are `Scaled`-or-`NULL_FLAG` running dimensions — needs a running-dimension wrapper type before its fields are typed). OR the first fused `01b` §3a adoption: thread a small box- or glue-reading leaf to `&mut MemState` AND retype its node ptr to NodePtr/AnyBox — proves type-as-you-thread end-to-end + starts driving the bare-i32 node-ptr-param metric (01b §8) down. A leaf box/glue consumer in `packaging.rs`/`shipout.rs` reading dims under a single `mem` borrow is a good first target.** prior session 147, **execution — `01c` Stage 2 verified ALREADY DONE + Stage 3 rank-0/1 built. Commit 1 (plan): recon found Stage 2 (the `Mem` `Vec<MemoryWord>` ownership move) was completed sessions 11/51 before `01c` was written (`MemState.data` is a Vec, alloc engine.rs:1073/format_io.rs:854 with inclusive sizing, byte-exact `.fmt` dump, no C symbol/double-free, raw-ptr consumers derive from the Vec) — corrected `01c` §2 + `01`-§4a to DONE so it isn't redone. Commit 2 (code): NEW `src/node.rs` — the `NodePtr<K>` typed-handle core (`#[repr(transparent)]` MemIdx+PhantomData; NodeKind; capability traits HasDimensions/HasListPtr/HasGlueSet) + the BOX family (HListNode/VListNode) with all 8 field methods at C-exact offsets (width+1/depth+2/height+3/shift+4 Scaled, list_ptr+5 MemPtr, glue_sign/order+5 b16, glue_set+6 gr) reusing the audited MemState word accessors + the `node_type`/`classify_box` checked boundary; 4 unit tests. DISASM GATE PASSED (strongest form): release `objdump` shows typed `width()` and raw `b32s1(p+1)` identical-code-folded to the same address — provably zero-cost. `#[allow(dead_code)] mod node;` (foundation before consumers). Warnings 42→42; `with_state`/`unsafe fn`/`#[no_mangle]` unchanged; behavior-neutral. Dashboard 29/2/7/4. **Next: continue Stage 3 (glue/glue_spec family — trap: glue_spec `0.b32.s1` is a ref_count not a link), OR the first fused `01b` §3a adoption (thread a small box consumer to `&mut MemState` + retype its box ptr to NodePtr/AnyBox).** prior session 146, **execution — typed foundation Stage 1 COMPLETE: added `NodeType` (`#[repr(u16)]`, 33 discriminants) + checked `NodeType::from_raw` + `InvalidNodeType` to `src/units.rs`, mirroring `xetex_format.h:380–416` exactly (`HList`=0…`RightNoad`=31, `MarginKern`=40). Encoded the faithfulness subtleties: `DELTA_NODE`/`EDGE_NODE` are context aliases of `Rule`(2)/`Style`(14), not separate codes; whatsit subtypes (`NATIVE_WORD_NODE`40/`GLYPH_NODE`42/`PIC_NODE`43/`PDF_NODE`44) live in `b16.s0`, NOT node types; `from_raw` rejects the 32–39 gap + 41+. 3 new unit tests (6 total in `units::`, all pass): discriminants==C codes+round-trip, invalid-code rejection, `size_of==2`. Split from the 145 scalars because the enumeration is the one parity-risky part of Stage 1. Warnings 42→42 (in the `#[allow(dead_code)]` module). `with_state`/`unsafe fn`/`#[no_mangle]` unchanged; behavior-neutral (additive). Dashboard 29/2/7/4 green. **Next: `01c` Stage 2 — the `Mem` `Vec<MemoryWord>` ownership move (`01`-§5 steps 1–2; the first Stage touching a live code path → real parity slice: `n+1` sizing, `tt_cleanup`/longjmp teardown, byte-exact `.fmt` dump/undump `format_io.rs:299–339`). Then Stage 3 (per-family `NodePtr<K>`, box first).** prior session 145, **execution — typed foundation Stage 1: the scalar-newtypes module (`01c` §2). NEW `src/units.rs` with the `#[repr(transparent)]`+`Copy` scalar newtypes `MemIdx`(no null)/`MemPtr`(null=`TEX_NULL`)/`Scaled`/`StrNumber`/`FontId`(now `i32`)/`Token`, applying the two `01c`-§2 source-fixes (`MemPtr::NULL=MemPtr(TEX_NULL=-0x0FFFFFFF)`≠0 with `as_idx()->Option<MemIdx>` as the checked boundary; `FontId` wraps `i32` not `u16`). `MemIdx::new` debug-asserts `!=TEX_NULL`; 3 unit tests pass (`#[repr(transparent)]` size==`i32` = the Stage-1 zero-cost evidence). `lib.rs`: `#[allow(dead_code)] mod units;` (foundation-before-consumers, `01`-§4a). Deleted the vestigial duplicate newtypes from `src/mem.rs` (zero call sites, not re-exported → behavior-neutral). Scoped to pure scalars; `NodeType`+`from_raw` DEFERRED to its own slice (faithful ~30-discriminant enumeration is a parity hazard, do it with the Stage-3 classification boundary). Warnings 48→42 (−6, from the dead-stub deletion; new module adds none). `with_state`/`unsafe fn`/`#[no_mangle]` unchanged. Dashboard 29/2/7/4 green; units 3/3. **Next: the `NodeType` enum + checked `from_raw` slice, then `01c` Stage 2 (the `Mem` `Vec<MemoryWord>` ownership move), then Stage 3 (per-family `NodePtr<K>`, box first, where the disasm gate becomes load-bearing).** prior session 144, **plan construction — drafted the missing typed-foundation cookbook to READY: NEW `01c-typed-node-recipe.md` (the per-slice analog of `01b`, for building the `NodePtr<K>` layer) + a reproducible recon script `node_field_recon.py`. The recon measured 742 node-FIELD sites (offset `+N`; matches the session-143 "~725 (p+N)" estimate) + 1378 node-HEADER-word sites (offset 0), over 37 distinct `(subfield, offset)` signatures. Data-driven family build order (highest-traffic + simplest-invariant first): shared-header-word → box(hlist/vlist) → glue/glue_spec → kern → penalty → rule → disc → ligature → math/noad → whatsit/native → char/one-word → linebreak-internals. `01c` specifies the §4a foundation order made concrete (scalar newtypes → `Mem` ownership move → per-family `NodePtr<K>`), the `NodePtr<K>` design (MemIdx+marker, capability traits, semantic-typed returns, classify-once boundary), a MANDATORY release-disasm zero-cost gate, the per-slice recipe, the `01b` dovetail + worklist fallback, the HARD constraints (`is_char_node(p)≡p>=hi_mem_min` mem.rs:910; `.fmt` raw-index+free-list dump format_io.rs:299–339, `FORMAT_SERIAL=33`; `var_used`/`dyn_used` printed format_io.rs:355–357), and the explicit NON-GOAL (keep the free-list arena; no per-node heap alloc). Two verified source-fixes flagged for the scalar-newtype slice: `TEX_NULL=MIN_HALFWORD=-0x0FFFFFFF` (consts.rs:13) makes both the vestigial `MemIdx::NULL=MemIdx(0)`/`is_null()` (mem.rs:64–69) and the parent `03` `MemPtr::NULL=MemPtr(0)` sketch WRONG for this codebase; the existing `MemIdx`/`Scaled`/`FontId`/`Token`/`StrNumber` (mem.rs:56–96) are vestigial stubs (used only at their defs; `MemPtr`/`NodePtr` absent) so the foundation is greenfield. Cross-refs to `01c` added in `01`-§4a/§5, `01b` intro, `00`-overview. No code change. **Next: fused threading+typing slices can now begin — first foundation execution slice = the scalar-newtypes module (`01c` §2 Stage 1), gated on the disasm check.** prior session 143, **plan construction — adopted "TYPE-AS-YOU-THREAD" (decided 2026-06-06). Updated the strategy docs so every *remaining* Phase-C threading slice carries its typing (`NodePtr<K>`/`MemIdx`/`MemPtr`/`Scaled`) in the SAME touch, instead of the previously-implied deferred second call-site pass. Edits: `01`-§4 flips the newtype-rollout decision to option (c) (type-as-you-thread), superseding the old (a)-mostly rule; NEW `01`-§4a "foundation-first prerequisite" (build scalar newtypes + the `Mem` ownership move + the per-family `NodePtr<K>` layer up front, gated on a zero-cost release-disasm check; a deferred-newtype-worklist row is the fallback that keeps threading un-blocked when a family's layer isn't built yet); `01`-§6/§9 updated. `01b` reframed to "threading + typing" with NEW §3a (typed-surface adoption = mirror of the bridge-down convergence), NEW §4b (typing is compiler-driven, NOT regex/codemod — `thread_fn.py` stays a threading tool), typed §7 recipe steps, and typing metrics in §8 (bare-`i32` node-ptr params → 0, deferred-newtype-worklist length → 0). `00-overview` sequencing rationale records the fusion. No code change this session. **Next: draft the missing `READY` cookbook for the `NodePtr<K>` layer (the per-slice analog of `01b`) + a node-field-access recon per node family to order the families data-driven; THEN fused slices begin.** prior session 142, **resumed workstream-01 Phase-C threading now that ws03 is complete — threaded `app_space` (the hmode inter-word-glue builder, `main_control.rs`) to disjoint `&mut MemState`/`&EqtbState`/`&mut FontState`, 1 commit. Swapped the `new_param_glue`/`new_spec`×2/`new_glue` `_global` calls to the threaded cores with the §5 `NodeRecycleAbort::Overflow` sentinel; resolved the §129 trap by computing the two `xn_over_d` via the pure `xn_over_d_parts` core under the borrow and applying the deferred `tex_remainder`/`arith_error` side effects in `app_space_global` after the borrow drops. Added `FontState::font_glue` reader. Dashboard 29/2/7/4, warnings 48→48 flat. Metrics: `with_state` 1196→1197 (+1 wrapper), `unsafe fn` 576 flat, `#[no_mangle]` flat. CONFIRMS session 138: the clean leaf backlog is drained — this borderline one-caller fn needed 3 disjoint borrows + the §5 sentinel + the §129 deferral. Next: thread the print core (`output.rs`) as a new foundational `&mut ShipoutState` subsystem, or take the `vpackage` scoped-island giant.** Bench not run — needs an `ORACLE_BIN` not configured here; `app_space` is plain hmode spacing (not OT-math/native), so the §135 corpus gate is not load-bearing for this slice.) — prior session 141, **executed workstream-03 / overview target #2 — DEAD-CODE PASS & STATICS MIGRATION. Cleanly deleted 4 deferred-dead functions, migrated 3 statics into EngineState, and resolved borrow-checker panics. Dashboard 29/2/7/4 + warnings down to 48. Metric: 0 external internal statics remaining.** Next: centralize constants / continue phase C.) — prior session 140, **executed workstream-03 / overview target #4 — THE DE-UNSAFE SWEEP, the natural continuation of Session 139's de-export sweep. 1 commit, 70 functions converted from `unsafe fn` to safe `fn`, 8 unused unsafe blocks removed. Dashboard 29/2/7/4 + warnings flat at 50. Metric: true `unsafe fn` count dropped from 651 to 580 (-71).** Next is the dead-code-removal pass (4 deferred-dead fns + 3 statics). — prior session 138, **executed workstream-01 Phase C — tested the session-137 "batch with the codemod" decision in practice: BATCH-threaded two no-HAZARD leaf clusters off `with_state` with `thread_fn.py`, 2 commits, 5 functions, and recorded the throughput.** Surveyed the real leaf backlog with a one-off scanner (column-0 fns taking no state borrow + still calling bridge indexed `mem_*`/`eqtb_*` + HAZARD-free per the tool) — ~30 candidates, but the mandatory §4/§135 eyeball gate cut it to ~6 (the regex has false-negatives). **Commit 1 (`289e36a8`) the main_control paragraph/mode-reset verb cluster (4 fns) in ONE validation pass:** `trap_zero_glue`→`&mut MemState` (no alloc/sentinel; threaded `delete_glue_ref`); `fix_language`+`indent_in_hmode`→`&mut MemState`+`&EqtbState` (`new_whatsit`/`new_null_box`/`new_noad` `ALLOC_FAILED`→`01b` §5 `NodeRecycleAbort::Overflow` via `_global` wrappers; `norm_min` unsafe-blocked); `normal_paragraph`→disjoint `&mut EqtbState`/`SaveStackState`/`MemState`/`ScannerState` (threaded `eq_word_define`/`eq_define` cores, first-`SaveAbort` short-circuit; `save::raise_save_abort` made `pub(crate)`). `cur_val`/`cur_chr`/`cur_list.*` are **raw** (safe under the borrow); 12 legacy callers → bridge-down `_global` wraps; three `extern "C"` cores de-exported. **Commit 2 (`dd0a73ca`) `pseudo_close`→`&mut MemState`+`&mut InputClusterState`** (`\scantokens` closer; pure node recycle, no sentinel; `pseudo_files`/`avail` become fields; free-size hoisted before the `&mut free_node`; both callers→`pseudo_close_global`; 4 dead imports removed). **THROUGHPUT (the point of the session): 5 fns / 2 validation passes vs session-137's 3 fns / 3 passes ≈ ~1.7× fns/session, ~2.5× fns/validation-pass — real but MODEST, well short of 3×.** Why: the codemod removed the keystrokes (22 accessor swaps) + first-pass grep, but the binding cost was (a) *finding* clean leaves and (b) the *manual hazard eyeball*, neither codemoddable; and clean single-borrow mem/eqtb leaves are nearly drained (`save`/`mem`/`pagebuilder` done; recon bucket-B now mostly output/print bucket-C). **Codemod blind spots found+fixed:** HAZARD_RE missed `xn_over_d` (sets `tex_remainder` via `with_state`, §129) in `app_space`, `mlist_to_hlist`/`hpack` in `clean_box`, `the_toks`/`begin_token_list` in `ins_the_toks` — caught all by reading the body (discipline #1), then **hardened `thread_fn.py`'s HAZARD_RE** to flag the re-entrant giants/helpers without a safe core (`mlist_to_hlist`/`hpack`/`vpackage`/`vsplit`/`vert_break`/`the_toks`/`begin_token_list`/`macro_call`/`conv_toks`/`expand`/`xn_over_d`/`x_over_n`/`unsave`/`build_page`/`push_nest`/`pop_nest`/`insert_src_special`; `prune_page_top` excluded — has a safe core), verified it now flags those three. **Strategy implication:** the modest speed-up + leaf scarcity supports redirecting the codemod toward driving `#[no_mangle]`(293→289)/`unsafe fn` down (mechanical, abundant, not blocked on the giants) rather than chasing `with_state`→0 through a near-empty leaf backlog; the next-session vpackage scoped-island spike gives the cost-per-giant half of the decision. Metrics: `#[no_mangle]` **293 → 289** (−4); `with_state` sites **1129 → 1134** (+5 transitional wrappers); `unsafe fn` **363 → 367** (+4 wrappers); warnings **50 → 50**; dashboard **29/2/7/4**, bench `--scope tier1` **12/12** + `tier1-real` **2/2**. **Next: vpackage scoped-island bucket-C spike (cost-per-giant), then the keep-`with_state`→0-vs-redirect-to-de-export/de-unsafe call.** — prior session 137, **executed workstream-01 Phase C — threaded the FFI-free `packaging.rs` node-recycle / vlist-append cluster (`just_copy`, `just_reverse`, `append_to_vlist`), 3 commits**. Triaged the user's primary candidate `unsave`/`prepare_mag` and confirmed still blocked (`unsave` interleaves `input::back_input` mid-loop + `output::group_warning` after the loop — both `with_state`-bearing, bottoming on the large input/output-print subsystems); the recon `--bucket B` list is dominated by output/print (bucket-C). Picked instead a genuinely **FFI-free mem/fonts consumer cluster bottoming on already-threaded leaves**, beside the threaded `flush_node_list`/`copy_node_list`. Each body grepped for a C++ measurer / `new_native_character` / print / scanner re-entry first (the §135/136 discipline) — all three clean. **Commit 1 (`9465e7c4`) `just_copy` → one `&mut MemState`**: the bidi-LR-reversal deep-copy; ~30 `with_state`-bearing `mem_*` accessors collapse to direct methods under one borrow, reusing the shared `cnl_get_node!`/`cnl_get_avail!` macros + threaded `copy_native_glyph_info`; cold corrupt-node `confusion`/exhaustion → `01b` §5 `NodeRecycleAbort` sentinel via new `just_copy_global`; deleted the now-dead `copy_native_glyph_info_global` (last caller removed). **Commit 2 (`b2ac5318`) `just_reverse` → `&mut MemState` + `&mut ScannerState`**: `cur_dir`/`LR_ptr`/`LR_problems` → `ScannerState`; **`temp_ptr` stays the raw global accessor** (reads `raw::temp_ptr`, not `with_state` — safe under the borrow, §4); threaded `new_edge`/`flush_node_list`/`get_avail`/`free_node`/`just_copy` called directly with the sentinel mapping; new `just_reverse_global`. **Commit 3 (`bae24b48`) `append_to_vlist` → `&mut MemState` + `&EqtbState`**: `eqtb_b32s1` → methods, `cur_list`+`temp_ptr` raw, threaded `new_param_glue`/`new_skip_param` with the ALLOC_FAILED check **before** the `temp_ptr` write (abort-before-use order preserved); new `append_to_vlist_global`, four callers repointed. **All three de-exported** (`#[no_mangle] extern "C"` → `pub fn`; dead `xetex-xetexd.h` prototypes left as harmless declarations). **Validated per §135 with the bench corpus, not just the four suites**: `--scope tier1` **12/12** (incl. `unicode_math`/`bidi_arabic`) and `--scope tier1-real` **2/2** (incl. `unimath-example`), exit 0 on every commit. **Deferred (confirmed not param-threadable):** `vert_break`/`vsplit`/`hpack`/`vpackage` all interleave the `error()`/`*_diagnostic` print path with mem allocation *inside their loops* (and `hpack` calls the C++ `measure_native_node`) — they need scoped-island bucket-C work, one per session. Metrics: `#[no_mangle]` **296 → 293** (−3); `with_state` sites **1127 → 1129** (+2 transitional, three new wrappers − one deleted); `unsafe fn` **→ 364** (+3 transitional `_global` wrappers; the three cores are now safe `pub fn`); warnings **50 → 50** (no §7 trap). Dashboard **29/2/7/4**. **Lesson: the safe Phase-C wins right now are the small FFI-free mem/eqtb/scanner consumers that bottom on threaded leaves — grep each body for a measurer/native/print/scanner re-entry before single-borrow threading. The packaging node-recycle/vlist cluster is now fully drained; the rest of `packaging.rs` is bucket-C.** **Tooling (speed-up): built `plan/idiom/thread_fn.py` — the `01b` §4a codemod that (1) automates the §4/§135-136 FFI-hazard grep (HAZARD report of `with_state`-bearing re-entries), (2) auto-rewrites the regular indexed `mem`/`eqtb` accessors, (3) emits caller list + wrapper skeleton; signature/§5-sentinel/scalar/`*_global` swaps stay manual. Workflow: classify a module's no-HAZARD functions, `--apply` each, wire + commit the cluster in one validation pass (`01b` §4a has the invocation). Caveat: this drains the leaf backlog faster but `with_state`→0 stays gated on the few giant callers (`hpack`/`line_break`/`mlist_to_hlist`/`build_page`/`main_control`), which are bucket-C scoped-island work, not codemod-able.** **Next: batch-thread no-HAZARD leaf clusters with `thread_fn.py`; thread `back_input`/`group_warning` to unblock `unsave`/`prepare_mag`; or start the scoped-island bucket-C giants where the headline number actually falls.** — prior session 136, **executed workstream-01 Phase C — re-threaded the *safe half* of the reverted OT-math `math.rs` slice with scoped borrows (the session-135 "Next"), 2 commits**. Triage: post-revert the slice was already almost entirely in correct scattered-scoped-island form; only two wins were safely recoverable. **Commit 1 (`8644172c`)** threaded `build_ext_delimiter_box` to **one disjoint `mem`+`fonts` borrow** — extensible delimiters are a **pure-TFM** path (every font read is a `FontState` lookup; every piece box uses the new shared `char_box_tfm` helper) with **no C++ measurer / `with_state` re-entry**, so it was reverted only as collateral (the `unicode_math` panic came from OT-assembly + scripts, not this path); deleted the now-dead `stack_into_box_global` wrapper + 2 unused imports. **Commit 2 (`72662f8b`)** re-threaded `build_opentype_assembly` with **three scoped `with_state` islands over borrow-free regions only** (box-type set, natural-size measurement, final dimension write) and **de-exported** it (`#[no_mangle] extern "C"` → `pub unsafe fn`; dead C prototype) — it **cannot** be param-threaded because its part-build loop measures native glyphs via the C++ measurer (the session-130 hazard). **Left in scattered-island form (required):** `native_glyph_vbox`, `stack_glyph_into_box`, and the scripts slice (`first_script_glyph`/`prepare_superscript`/`prepare_subscript`) — they interleave the C++ measurer / `new_native_character` with mem allocation. **Validated per the §135 lesson with the bench corpus, not just the four suites:** `--scope tier1` **12/12** (incl. `unicode_math`), `--scope tier1-real` **2/2** (incl. `unimath-example` — real `unicode-math` + OpenType math font); the 2 tier2 `--scope docs` failures (`1905.00073v1`/`1905.00148v1`) are **pre-existing oracle divergences** — HEAD~2 baseline and HEAD candidate produce **byte-identical XDV** (verified), so this work alters zero output. Metrics: `#[no_mangle]` **297 → 296**; `with_state` sites **1125 → 1127** (+2 net transitional, cf. sessions 127/128/132); `unsafe fn` **→ 360**; warnings **50 → 50**; dashboard **29/2/7/4**. **Lesson: a function that interleaves the C++ OT-math measurer with mem allocation cannot be param-threaded (the caller's borrow would span the FFI re-entry); the safe gains are param-threading the genuinely FFI-free sub-paths + de-exporting/scoped-islanding the rest. Always grep an OT-math/native-font body for a C++ measurer / `new_native_character` call before attempting a single-borrow thread.** **Next: continue climbing the call graph per `01b` §9** (e.g. `unsave`/`prepare_mag` once `back_input`/`group_warning` thread, or other FFI-free `mem`/`fonts` consumers). — prior session 135, **reverted sessions 130–131's OT-math `math.rs` threading (`e0b7f297`/`2fd44937`)**: they wrapped each OT-math body in **one `with_state_mut` borrow** while calling the C++ readers (`get_native_mathsy_param`/`measure_native_*` → `tt_get_font_size_entry`/`tt_get_font_area_entry`) and `new_native_character`, all of which **re-enter via `with_state`** → the tier1 corpus `unicode_math` aborted with `RefCell already borrowed`. The four suites never exercise the OT-math delimiter/assembly/script paths, so they stayed green; the `bench_phase_c.py` corpus gate caught it (the §127 "clean run ≠ safe" lesson, one tier deeper — a *threaded* slice can pass all four suites and still hold a borrow across an FFI re-entry only a corpus doc triggers). `git revert` (code auto-merged; append-only docs resolved `--ours`; the session-134 `eq_word_define_global` renames survived). Bench `--scope tier1` runs clean **10/10**. Metrics after revert: `#[no_mangle]` **→ 297**; `with_state` sites **→ 1125**. Dashboard **29/2/7/4**. **Next: re-thread the OT-math slice (delimiters, OT assemblies, scripts) with *scoped* borrows — drop the `with_state*` borrow around every C++ OT-math / `new_native_character` FFI call (the pre-130 discipline) — and validate with the bench corpus, not just the four suites.** — prior session 134, **executed workstream-01 Phase C — threaded the `save.rs` save/define cluster (8 of 10 fns)**; threaded `new_save_level`/`eq_save`/`eq_word_define`/`geq_word_define`/`save_for_after`/`eq_destroy`/`eq_define`/`geq_define` to narrow `&mut SaveStackState`/`&mut EqtbState`/`&mut MemState`/`&mut ScannerState` borrows, each with a `_global` bridge-down wrapper (`eq_save` internal-only); added a `SaveAbort` `Overflow`/`Confusion` sentinel (`01b` §5) so the cold `overflow`/`confusion` paths fire in the wrapper after the borrow drops; dropped `#[no_mangle] extern "C"` from the 6 exported ones (dead `xetex-xetexd.h` prototypes, no C callers); renamed ~140 call sites in main_control/math/expand/pagebuilder/scan; added `EqtbState`/`SaveStackState` inherent methods; **deleted 3 now-dead `globals.rs` accessors** (`max_save_stack`, `set_save_stack_b16s0/b16s1`). **Left bridged:** `unsave` (calls `back_input`/`group_warning` mid-loop — `with_state`-bearing, blocked on the scanner/output cluster, `01b` §9) and `prepare_mag` (bucket-C error-reporting, `01b` §5); `save.rs` retains 2 `#[no_mangle]`. **This commit also bundles the previously-uncommitted session-133 `hash.rs` work (one big commit, per user request).** Metrics (from the session-132 string_pool baseline): `#[no_mangle]` **→ 296**; `unsafe fn` **→ 358**; `with_state` sites **→ 1117**. Dashboard **29/2/7/4** green, net 0 new warnings from the save slice. **Next: thread `unsave`/`prepare_mag` once `back_input`/`group_warning` are threaded; otherwise continue climbing the call graph.** — prior session 133, **executed workstream-01 Phase C — threaded the entire `src/hash.rs` module, 1 commit; threaded `id_lookup` and `prim_lookup` to take narrow states and returned `HashLookupError` to defer `overflow` aborts to global wrappers; added inherent hash/prim accessors to `EqtbState`; deleted dead legacy `no_new_control_sequence`/`prim_used` accessors**; Metrics: `#[no_mangle]` **304 → 302** (−2); `unsafe fn` **345 → 347** (+2); `with_state` sites **1112 → 1113** (+1 net). Dashboard **29/2/7/4** on every commit, warnings **50 → 50**. **Next: continue climbing the call graph or other Phase-C threading.** — prior session 132, **executed workstream-01 Phase C — threaded the entire `src/string_pool.rs` module, 1 commit; threaded length, make_string, append_str, str_eq_buf, str_eq_str, search_string, slow_make_string, load_pool_strings, and flush_str to take narrow StringsState/buffer states, added 9 global wrappers for legacy callers**; Metrics: `#[no_mangle]` **309 → 304** (−5); `unsafe fn` **345** (flat); `with_state` sites **1103 → 1112** (+9 net). Dashboard **29/2/7/4** on every commit, warnings **50 → 50**. **Next: continue climbing the call graph or other Phase-C threading.** — prior session 131, **executed workstream-01 Phase C — threaded the script/superscript-processing helper slice in `math.rs` (`first_script_glyph` and `prepare_superscript`), 1 commit; resolved the nest-panic hazard via pure helper `x_over_n_parts` to avoid mutating thread-local state under a borrow; wrappers `first_script_glyph_global` and `prepare_superscript_global` retained for legacy callers**; `handle_fetch_outcome` is safe. Metrics: `#[no_mangle]` **309** (flat); `unsafe fn` **346 → 345** (−1); `with_state` sites **1108 → 1103** (−5). Dashboard **29/2/7/4** on every commit, warnings **50 → 50**, no §7 trap. **Next: continue math.rs threading (`prepare_subscript`).** — prior session 130, **executed workstream-01 Phase C — threaded a coherent math.rs consumer slice top-down, including extensible delimiters (`build_ext_delimiter_box`) and OpenType assemblies (`build_opentype_assembly`), 1 commit; math.rs and ext.rs wrapper call sites collapsed, 4 dead wrappers deleted (`stack_glyph_into_box_global`, `stack_glue_into_box_global`, `native_glyph_vbox_global`, `build_ext_delimiter_box_global`)**; TFM font box building is clean (`char_box_tfm`), native glyph vbox is threaded (`native_glyph_vbox`). Metrics: `#[no_mangle]` **310 → 309** (−1); `unsafe fn` **349 → 346** (−3); `with_state` sites **1111 → 1108** (−3 net). Dashboard **29/2/7/4** on every commit, warnings **50 → 50**, no §7 trap. **Next: continue math.rs threading or other legacy global wrapper call site cleanup.** — prior session 129, **executed workstream-01 Phase C — threaded the `fonts.rs` font-LOADING core (`read_font_info` + `load_native_font`) to `&mut FontState`, 2 commits; `fonts.rs` `crate::globals::` calls 151 → 58 (`font_*` → 17, all now correctly outside the borrows)**. **A** (`9f1948fb`): extracted `read_font_info`'s TFM metric-parse + font-table-fill body (xetex0.c:6350, post-`file_opened`) into `read_font_info_tfm(&mut FontState, &EqtbState, …) -> (TfmParse, Option<(i32,bool)>)`, one `with_state_mut` borrow (~38 `font_*` calls → direct method/field access); the cold `_tt_abort` font-full path returns a `TfmParse::OutOfMemory` sentinel run after the borrow drops (01b §5), the trailing `load_tfm_font_mapping` write stays in the wrapper. **The hidden with_state call:** `scaled_math::xn_over_d` is *not* pure — it sets `tex_remainder`/`arith_error`, reached only on the `s<0` scaled-font branch that **only etrip** exercises (tex-outputs/engine_rust/latex stayed green; TRIP/etrip caught it as a `RefCell already borrowed` nest-panic — the 127 "clean run ≠ safe" lesson). Fix: added pure `xn_over_d_parts(x,n,d) -> (quotient,remainder,overflowed)` (and `xn_over_d` now wraps it), used inside the borrow, side effects applied in the wrapper after the borrow drops — captured via the returned `Option` so the `bad_tfm` path preserves them too. **B** (`7f5d1dc6`): extracted `load_native_font`'s contiguous per-font metric-table init block (xetex0.c:6284, ~lines 952–1014) into `init_native_font_tables(&mut FontState, &EqtbState, …) -> (font_ptr, x_ht, cap_ht, font_slant)`, one borrow (~25 `font_*` calls); the extern-C metric extractors (`aat`/`ot_get_font_metrics`, `get_loaded_font_design_size`) are borrow-safe, and the with_state-bearing `new_native_character`/`free_node`/`push_font_info` calls all come *after* the block (stay outside); the boundary-straddling metric out-params are returned. Added the mirroring `FontState` `set_*` methods (parity-exact field writes); **deleted 4 now-dead `globals.rs` accessors** (§6): `set_font_info_b16`+`exten_base_entry` (A), `set_font_check`+`set_font_letter_space_entry` (B), each a benign +1 dead-code warning caught by the count check. Metrics: `#[no_mangle]` **310** unchanged; `unsafe fn` **347 → 349** (+2 helpers); `with_state` sites **1113 → 1111** (−2 net). Dashboard **29/2/7/4** on every commit, warnings **50 → 50** (the 127–128 "60" was stale; `cargo check -p tectonic_engine_xetex` reports **50**), no §7 trap. **Lesson: a "pure math" helper can be with_state-bearing — `xn_over_d` sets the remainder global; split a pure `*_parts` core and apply the side effects after the borrow. etrip is the only guard for the scaled-TFM path.** **Next: thread `new_native_character` (`&FontState`/`&mut MemState`), or collapse the 127–128 `_global` wrappers (but their callers are the giants — `mlist_to_hlist`/`line_break`/`hpack`/`ship_out`/`build_page`); `find_native_font` once the input-name buffer is threadable.**ce the input-name buffer is threadable.** — prior session 128, **executed workstream-01 Phase C — drained the rest of the `ext.rs` native-font subsystem (8 fns, 4 commits); every `crate::globals::font_*` accessor call is now gone from every native-font function in `ext.rs`**. **A** (`441d78dc`): threaded the node-measuring cluster `measure_native_node`/`measure_native_glyph`/`store_justified_native_glyphs` + `linebreak_next` to take `fonts: &FontState` (the three measurers write node fields through the raw `*p_node` pointer, not via `mem_*` accessors, so `&FontState` suffices — no `&mut MemState`); swapped `crate::globals::{font_area_entry,font_layout_engine_ptr,font_letter_space_entry,height_base_entry,depth_base_entry}` → `fonts.<method>` (all pre-existing), added 4 `_global` wrappers, renamed ~16 legacy callers (math ×9, linebreak ×3, main_control ×3, packaging/fonts/shipout, internal `do_extension`) to `_global`. **B** (`47e13c30`): the bucket-C edges via the §4a borrow-dropping shape — `map_glyph_to_index` threaded to `(&FontState, font, name)` with the with_state-bearing `name_of_file()` hoisted into the `_global` wrapper (form **a**); `print_glyph_name` (calls `print_char`) + `linebreak_start` (calls diagnostics) rewritten **in place** to read the font engine under a short scoped `with_state`, drop it, then run the output/diagnostic path borrow-free (form **b**, public signature unchanged → callers untouched); scan.rs ×2 → `_global`. **C** (`03fbf33d`): `make_font_def` → `&mut FontState` (its only non-font `with_state` were its own `xdv_buffer` accesses → direct field access; `font_area`/`layout`/`flags` → `fonts.<method>`); added `FontState::font_flags_entry`; **deleted the now-dead `globals::font_flags_entry` read accessor** (§6, last caller removed; benign +1 dead-code warning caught by the count check); shipout → `_global`. **D** (`6881530a`): `get_encoding_mode_and_info` + `check_for_tfm_font_mapping` threaded to take the `name_of_file` pointer as a param (form **a** wrappers) — these are input-name-cluster, not font; pure pointer pass-through. **Remaining in `ext.rs`:** only `find_native_font` still has bridge calls, all `name_of_file`/`name_length`/`set_name_length` (input-name cluster, **not** `font_*`); it *mutates* the name buffer interleaved with logic → bucket-D, deferred. Metrics: `#[no_mangle]` **310** unchanged; `unsafe fn` **338 → 346** (+8 transitional wrappers, collapse as callers thread); `with_state` sites **1108 → 1113** (+5 net). Dashboard **29/2/7/4** on every commit, warnings **60 → 60**, no §7 trap. **Lesson: the §4a borrow-dropping shape has two forms — (a) hoist the with_state-bearing read into the wrapper and thread the value in (for inputs), (b) read+drop inside the fn before the output/diagnostic work (for outputs); form (b) keeps the signature so callers don't change.** **Next: `find_native_font` once the input-name buffer is threadable; otherwise climb the graph to collapse the 127–128 `_global` wrappers by threading their math/linebreak/shipout/main_control/scan callers.** — prior session 127, **executed workstream-01 Phase C — threaded the `ext.rs` native-font-metric reader cluster (11 fns), the first post-ws02 threading slice**: ws02 (118–126) decomposed `scan.rs`, unblocking the recon's long-standing #1 bucket-B target that sessions 105–112 deferred for scanner-coupling. Threaded `map_char_to_glyph`/`get_font_char_range`/`get_glyph_bounds`/`getnativecharwd`/`getnativecharic`/`get_native_char_sidebearings`/`getnativecharht`/`getnativechardp`/`get_native_char_height_depth`/`real_get_native_italic_correction`/`real_get_native_glyph_italic_correction` to take `fonts: &FontState` (calling `FontState` methods instead of the `crate::globals::font_*` `with_state` accessors); added 11 transitional `_global` bridge-down wrappers; blanket-renamed the ~22 external call sites (scan/expand/fonts/math/main_control) to `_global` — **all callers legacy, zero pass-through/borrow analysis**. Added one `FontState` method (`font_letter_space_entry`); the other 4 reads already existed. **Excluded `print_glyph_name` + `map_glyph_to_index`** (same recon cluster) — they call `with_state`-bearing `print_char` / `name_of_file()`, so a `with_state` wrapper nest-panics (§4); both are bucket-C output/input edges, reverted to bare-globals and left for a borrow-dropping slice. **Lesson: bucket-B-by-score is insufficient — grep the body for any `with_state`-bearing callee before wrapping.** Metrics: `#[no_mangle]` 310 unchanged; `unsafe fn` 327→338 (+11 transitional wrappers, collapses as callers thread); `with_state` sites 1135→1146 (+11). Dashboard 29/2/7/4, warnings 60→60, no §7 trap. **Next: bucket-C `print_glyph_name`/`map_glyph_to_index`; then `measure_native_node`/`measure_native_glyph` (write node fields → `&mut MemState`+`&FontState`); continue climbing the font/scanner call graph.** — prior session 126, **executed workstream-02 slice 11 — the `ini.rs` drain, 4 sub-commits; `ini.rs` is now fully dissolved**: split the entire `xetex-ini.c` translation into its `§5` destinations and deleted `ini.rs`. **11a** `hyphenation.rs` (new) ← the 11 trie-build / pattern-exception fns (`trie_op_hash`/`set_trie_op_hash`/`new_trie_op`/`trie_node`/`compress_trie`/`first_fit`/`trie_pack`/`trie_fix`/`init_trie`/`new_patterns`/`new_hyph_exceptions`) + the trie consts + `_trie_op_hash_array`; repointed `init_trie` (linebreak.rs) and `new_hyph_exceptions`/`new_patterns` (main_control.rs). **11b** `format_io.rs` (new) ← the 16 format dump/load fns (`dump_*`/`undump_*`/`store_fmt_file`/`pack_buffered_name`/`bad_format_file`/`load_fmt_file`); the format-const block moved with them, the engine-side subset duplicated back, `TRE_POOL_SIZE`/`TRE_MAX_STRINGS` made pub(crate); focused ttstub/tre extern block. **11c** `engine.rs` (new) ← the 9 driver fns (`get_strings_started`/`sort_avail`/`primitive`/`final_cleanup`/`initialize_more_variables`/`initialize_more_initex_variables`/`initialize_primitives`/`tt_run_engine`/`tt_cleanup`) + all support (consts, `TRE_*` statics, the two extern blocks, `XetexPrimitiveDef`, and `intpar`); repointed `tt_cleanup` (errors.rs), `intpar`/`sort_avail`/`TRE_*` (format_io), `intpar` (main_control). **11d** moved `init_io`+`STDIN_UFILE` to `io.rs` (reusing its `UFile`/`INVALID_HANDLE`/`UTF8`, `UTF8 as i16` for the i16 field) and **deleted `ini.rs`** (removed `mod ini;`). LOC: `ini.rs` 3,804 → **0 (gone)**; `hyphenation.rs` 803, `format_io.rs` 1,478, `engine.rs` 1,533. Dashboard 29/2/7/4, warnings 50 on every commit; **no §7 trap fired** (the rise to 70 in 11b was benign `dead_code` from over-copied consts, cleaned via the warning-count check). **Lesson:** the generous-copy-then-prune approach (copy the whole consts/globals `use` block + the format-const block into the destination, then `cargo fix` + dead_code cleanup) is the fast reliable path for big shared-import carves; shared consts used by both the moved code and the residue get duplicated (faithful) rather than back-imported. **Decision: deferred slice 10** (rename `scan.rs`→`scanner.rs` + carve `token.rs`) — high churn (every `crate::scan::` ref) for cosmetic gain; `scan.rs`-as-scanner-core is fine to leave. **Next: Phase-C threading (ws01) resumes inside the now-navigable modules per `01b`** — `scan.rs` (4,998) scanner core, `main_control.rs` (6,823), `engine.rs`/`format_io.rs`/`hyphenation.rs`/`packaging.rs`/`expand.rs` all carved. — prior session 125, **executed workstream-02 slice 9b — the main-control carve, all 4 sub-commits**: drained the entire main_control dispatch+verb family out of `scan.rs` + `prefixed_command` out of `ini.rs` into `main_control.rs`. `scan.rs` 9,690 → **5,011** LOC (now **pure scanner core**); `main_control.rs` 1,185 → **6,821**; `ini.rs` 4,789 → **3,804**. Dashboard 29/2/7/4, warnings 50 on every commit. **9b-i** box/mode verbs (19 fns); **9b-ii** assign/font verbs (20 fns) — the **§7 trap fired** here (caught by the warning-count check 50→99, not errors: `off_save`'s SEMI_SIMPLE_GROUP/MATH_SHIFT_GROUP/MATH_LEFT_GROUP and `do_register_command`'s DIMEN_VAL/MU_VAL match arms silently rebound until duplicated in); **9b-iii** the dispatch loop (14 fns incl. the giant `main_control` match + `do_lookahead`/`lig_inner_loop`, repointed the `ini.rs:3642` entry call); **9b-iv** `prefixed_command` (~900 ln) — its 87 i32-typed consts **inlined as function-local consts** (not relocated module-level: 22 collide by name with main_control consts, 13 type-incompatibly, and 9 are still used by other ini.rs code — the §10 "move block wholesale" assumption was wrong), `intpar` stays in ini.rs (pub(crate)), `set_cp_code` extern moved, 81 dead block consts removed. **Two recon lessons:** the §10 "zero external callers" claim was wrong (a silently-failed zsh grep loop — the verbs *do* have qualified-path callers in math/linebreak/ini/ext/pagebuilder; trust `cargo check`, not pre-move greps); and the §7 trap is real+silent, caught only by the warning count. The §10 plan itself (4-sub-commit structure, fn lists, recipe) held up well. **Next: slice 10 — optionally rename `scan.rs`→`scanner.rs` + carve `token.rs` consts (small); then `ini.rs` (3,804) drains into `engine.rs`/`format_io.rs`/`hyphenation.rs`; then Phase-C threading (ws01) resumes.** — prior session 124, **executed workstream-02 slices 7+8+9a — the packaging, expander, and alignment carves** (scan.rs 14,710→**9,696** LOC across the session, now under 10k): slice 9a relocated the 14 `\halign` alignment fns (`init_align`/`align_peek`/`fin_align`/`init_span`/`init_row`/`init_col`/`push_alignment`/`pop_alignment`/`fin_col`/`fin_row`/`get_preamble_token`/`align_error`/`no_align_error`/`omit_error`) verbatim from `scan.rs` into a **new `main_control.rs`** (1,151 fn-lines, one commit) — seeding the main_control module; the dispatch + verbs + `prefixed_command` (gnarliest, spans ini.rs) follow in slice 9b. Zero external callers (only scan.rs calls them); big fns carry consts inline, dup'd 10 module-private consts, no §7 trap. `scan.rs` 10,846→**9,696** LOC (−1,150); `main_control.rs` = 1,185. Dashboard 29/2/7/4, warnings 50. **Next: slice 9b — main_control dispatch + verbs + prefixed_command (consider sub-commit splits), then scanner.rs/token.rs.** — slice 8 relocated the 6 expansion fns (`expand`/`macro_call`/`conditional`/`pop_cond_node`/`conv_toks`/`ins_the_toks`) verbatim from `scan.rs` into a **new `expand.rs`** (1,737 fn-lines, one commit). Most const-entangled slice so far — the match-heavy expander fns reference ~30 `scan.rs` **module-private** consts not centralized in `consts.rs` (command/token codes, brace/match-token limits, frozen-cs offsets), all **duplicated into `expand.rs` with their exact scan.rs values** (the §7 dup-shared-const step at scale). The §7 trap surfaced as hard E0408 (`IF_*_CODE` match arms) + one E0308 typed cousin (`SELECTOR_NEW_STRING` u8→i32), both caught at compile time; warnings 50→50. `scan.rs` 12,633→**10,846** LOC (−1,787); `expand.rs` = 1,848. Dashboard 29/2/7/4. **Next: main_control.rs (gnarliest — dispatch giant match + verbs + `\halign` + `prefixed_command` spanning scan.rs+ini.rs), then scanner.rs/token.rs.** — prior in same session, **executed slice 7 — the packaging carve** (first core-carve slice): drained the box/node-packing & list-utility group verbatim from `scan.rs` into a **new `packaging.rs`** (transitional → `node/` in ws03) across **two commits** — 7a node-recycle utils (`flush_node_list`/`copy_node_list`/`just_copy`/`just_reverse` + `_global` wrappers, 655 fn-lines) and 7b the packers (`hpack`/`vpackage`/`vsplit`/`vert_break`/`append_to_vlist`/`prune_page_top`/`do_marks` + `_global` wrappers, 1,250 fn-lines). Relocated all six private const blocks wholesale (`_FNL`/`_CNL`/`_PPT`/`_DM`/`_HP`/`_VP`); kept the shared `NodeRecycleAbort`/`node_recycle_abort`/`cnl_get_node!`/`cnl_get_avail!` infra in `scan.rs` (added `cnl_get_avail` re-export); duplicated module-private `TEMP_HEAD`/`IGNORE_DEPTH`. §7 trap fired once on `DISC_NODE` (caught via warning rise, fixed by import); the pre-existing `KERN_NODE | KERN_NODE` unreachable in `hpack` relocated cleanly (warnings 50→50). Biggest external-caller repoint yet — `crate::scan::<fn>`→`crate::packaging::` across math/shipout/linebreak/pagebuilder/ext/save/ini/errors. `scan.rs` 14,710→**12,633** LOC (−2,077); `packaging.rs` = 2,110. Dashboard 29/2/7/4. **Next: §8 core carve continues — expand.rs (expand/macro_call/conditional/conv_toks/ins_the_toks), then main_control.rs, then scanner.rs/token.rs.** — prior session 123, **executed workstream-02 slice 6** (last `scan.rs` leaf): mechanically relocated the 2 locale-linebreak fns (`char_pw_in`/`do_locale_linebreaks`) verbatim from `scan.rs` into `linebreak.rs` via `move_fns.py` (160 fn-lines). Two forced name-disambiguations to original referents (faithful, not behavior changes; both caught as hard E0308): qualified `LIGATURE_NODE`→`consts::LIGATURE_NODE` (linebreak.rs shadows it with an i32) and re-declared the raw `extern { static mut main_f }` it reads (Decision 4 hot scalar). Cleaned scan.rs orphans. No trap; warnings 50→50. `scan.rs` 14,890→**14,710** LOC. **Milestone: all 6 home-having leaf groups now drained; scan.rs holds the scanner/expand/main_control/packaging core — next is the §8 core carve.** Dashboard 29/2/7/4. — prior session 122, **executed workstream-02 slice 5**: mechanically relocated the 11 TFM/native font-loading fns (`effective_char`/`effective_char_info`/`read_font_info`/`read_tfm_word`/`read_fifteen`/`read_scaled_word`/`tfm_getc`/`load_native_font`/`new_native_word_node`/`build_native_character`/`new_native_character`) verbatim from `scan.rs` into a **new `fonts.rs`** (created + `mod fonts;` in lib.rs) via `move_fns.py` (966 fn-lines). Generalized `move_fns.py`'s regex to match `pub(crate)` fns; relocated the private `NativeCharSrc` enum and exposed the `cnl_get_node!` macro via `pub(crate) use`. Relocated 15 orphan consts, dup'd 5 shared, cleaned scan.rs orphans (8 ext/io imports, 5 dead externs). No trap; warnings 50→50. `scan.rs` 15,897→**14,890** LOC (−1,007; still largest module). Dashboard 29/2/7/4. — prior session 121, **executed workstream-02 slice 4**: mechanically relocated the 5 extension / `\special` whatsit fns (`append_src_special`/`new_write_whatsit`/`insert_src_special`/`tt_insert_special`/`do_extension`) verbatim from `scan.rs` into `ext.rs` via `move_fns.py` (297 fn-lines). Relocated `do_extension`'s entire 35-const `_DE` private block wholesale (dodges the match-arm trap that cherry-picking would leave), dup'd 3 shared consts, repointed `crate::scan::<fn>`→`crate::ext::` in math.rs/etc., cleaned scan.rs orphans. Warnings 51→**50** (benign clash-lint canonicalization from adding ext.rs's `ttstub_diag_printf` decl; a drop, not a trap-rise). `scan.rs` 16,255→**15,897** LOC (−358; still largest module). Dashboard 29/2/7/4. — prior session 120, **executed workstream-02 slice 3**: mechanically relocated the 12 file-naming / `start_input` / `close_files` fns (`begin_name`/`more_name`/`end_name`/`make_name_string`/`scan_and_pack_name`/`scan_file_name_braced`/`scan_file_name`/`pack_job_name`/`open_log_file`/`start_input`/`pack_file_name`/`close_files_and_terminate`) verbatim from `scan.rs` into `io.rs` via `move_fns.py` (559 fn-lines), plus the private `mod raw_utf8` (start_input's only scan-private dep). Larger import fixup (40 globals accessors + 13 dup consts + ttstub externs into io.rs); repointed `crate::scan::<fn>` call-sites in ini.rs/shipout.rs/errors.rs/pic.rs to `crate::io::`. Warning-count discipline drove the orphan cleanup in scan.rs (6 unused imports, 7 orphaned consts, 3 dead ttstub externs, 4 stale banners). No pattern-const trap this slice. `scan.rs` 16,850→**16,255** LOC (−595; still largest module). Dashboard 29/2/7/4, warnings 51. — prior session 119, **executed workstream-02 slice 2**: mechanically relocated the 12 `show_*`/display/messaging fns (`show_box`/`show_node_list`/`short_display`/`short_display_n`/`show_activities`/`show_context`/`show_save_groups`/`show_whatever`/`show_info`/`print_subsidiary_data`/`issue_message`/`char_warning`) verbatim from `scan.rs` into `output.rs` via `move_fns.py` (1,556 fn-lines). Also relocated the 78 single-function-private `*_SD`/`*_SA`/`*_SNL` const blocks and imported the 7 shared node-type consts from `consts.rs` — the §7 pattern-const trap fired **in bulk** here (51→102 warnings from 22 `unreachable pattern` + 14 binding warnings; warning count was the only signal, zero compile errors), fixed by following the consts. `scan.rs` 18,537→**16,850** LOC (−1,687; still largest module). Dashboard 29/2/7/4, warnings 51. — prior session 118, **executed workstream-02 slice 1**: mechanically relocated the 8 `errors`-group fns (`you_cant`/`report_illegal_case`/`privileged`/`cs_error`/`box_error`/`mu_error`/`runaway`/`extra_right_brace`) verbatim from `scan.rs` into `errors.rs` using the new `plan/idiom/move_fns.py` relocator. `scan.rs` 18,689→18,537 LOC. Caught + fixed a latent match-pattern-const behavior trap (now a §7 recipe gotcha: check warning count, not just errors). Dashboard 29/2/7/4, warnings 51. — prior session 117, **drafted `02-modules.md` to READY** — the module-decomposition workstream that lifts the `scan.rs`/`ini.rs` deferral. It maps the two grab-bags onto the `01-architecture.md` layout, decides **mechanical pre-split first then thread in place** (parity-trivial relocation decoupled from semantic threading; supersedes the `01b` §9 "thread+move together" suggestion for giant files), fences out-of-scope work (`node/` typed accessors → ws03, hot-scalar migration → later benchmarked slice, `state/` split → independent), and sets the first slice = drain `errors`/`output` leaves out of `scan.rs`. No code change this session. — prior session 116, **finished the `char_pw` protrusion-width leaf**: folded the two deferred `char_pw` calls into a fully-threaded `total_pw` — `total_pw(mem, &FontState, linebreak, q, p) -> Option<i32>` now sums `char_pw_in` inside its own borrow (the session-76 deferred split is no longer needed since `char_pw_in` is bridge-free) — deleted the legacy `char_pw` wrapper and the now-dead `set_last_leftmost_char`/`set_last_rightmost_char` globals accessors, and reverted unrelated math.rs reborrow churn from the inherited WIP. Dashboard 29/2/7/4, warnings 51. Recon `dead-wrapper` rows for `is_char_node_global`/`font_character_width_global`/etc. are false positives — those `#[inline]` wrappers are heavily used. — prior session 115 completed the `src/synctex.rs` Phase-C threading pass** — converted SyncTeX entry points to explicit `&mut EngineState` functions, bridged callers at the current legacy boundaries in `ini.rs`/`scan.rs`/`shipout.rs`, deleted unused xform/abort API remnants exposed by de-exporting, and left `src/synctex.rs` with no active `#[no_mangle]` exports and no `with_state*` calls. Dashboard 29/2/7/4, warnings 51. — prior session 114 finished local SyncTeX bridge cleanup.)

## How to read this file

A workstream plan moves through these construction states:

-   `PLANNED` — listed, not yet drafted.
-   `DRAFTING` — being written this session.
-   `DRAFTED` — written but not yet self-reviewed/validated against the
    code.
-   `READY` — drafted **and** spot-checked against the current source
    (paths, line numbers, counts verified); safe to execute.
-   `EXECUTING` — a session is refactoring against it.
-   `DONE` — the workstream's refactor is complete and on `master`.

**Execution may only begin on a `READY` plan** (hard rule 4 in
`README.md`).

## Plan construction

| # | Workstream plan | State | Notes |
|---|---|---|---|
| — | `README.md` | DRAFTED | Navigation + hard rules; reviewed for remaining-C precision. |
| — | `00-overview.md` | DRAFTED | Diagnosis, target, guardrails, sequencing; reviewed and tightened. |
| — | `STATUS.md` | DRAFTED | This file. |
| — | `SESSION_LOG.md` | DRAFTED | Append-only; first entry written. |
| 01a | `01a-global-inventory.md` | READY | Exhaustive global inventory: 384 raw extern globals, 16 promoted Rust statics, 18 private module-local statics appendix. |
| 01 | `01-state-and-memory.md` | READY | First/highest-leverage workstream. Phase B complete. §11 is the Phase-B recipe; Phase-C threading recipe is the separate `01b`. **Session 143: §4 adopted "type-as-you-thread" (option (c)); NEW §4a foundation-first prerequisite (scalar newtypes + `Mem` ownership move + per-family `NodePtr<K>` layer, worklist fallback). Typing now rides with threading, not a deferred second pass.** **Session 144: §4a/§5 now point at the `01c` cookbook (READY) that executes the foundation; §5 gained the one-line free-list non-goal.** |
| 01b | `01b-phase-c-recipe.md` | READY | Phase-C execution recipe (thread `&mut EngineState`/`&mut MemState` **+ type in the same touch**). Resolves the longjmp/`Drop` anchor (§7) + nest-panic rule; first slice = the `mem` allocator core. Drafted + recon-validated session 60. **Session 143: reframed to "threading + typing" — NEW §3a (typed-surface adoption), §4b (typing is compiler-driven not codemod), typed §7 steps, typing metrics §8.** **Session 144: the outstanding `NodePtr<K>` per-slice cookbook is now drafted — see `01c` (READY).** **Session 150: the FIRST fused §3a adoptions landed (`math_kern`→`NodePtr<KernNode>`, `math_glue`→`NodePtr<GlueSpecNode>`); §3a gained the worked-example note + the "retype an already-threaded fn's single-family node ptr" lesson; bare-`i32` node-ptr-param metric (§8) now moving (380→379).** |
| 01c | `01c-typed-node-recipe.md` | READY | **The typed-foundation cookbook (the per-slice analog of `01b`), drafted + source-verified session 144.** Carries the data-driven node-family build order (§1, from `node_field_recon.py`: 742 node-field sites + 1378 header-word sites; box→glue→kern→penalty→rule→disc→ligature→math→native), the concrete `NodePtr<K>` design (§3), the mandatory zero-cost disasm gate (§4), the per-slice recipe (§5), the `01b` dovetail + worklist fallback (§6), the hard arena constraints (§7: `is_char_node` partition, `.fmt` dump, `var_used`/`dyn_used` parity), and the NON-GOAL (§8: keep the free-list arena). Flags two source-fixes for the scalar-newtype slice (`TEX_NULL`=MIN_HALFWORD≠0; existing newtypes are vestigial stubs). **Unblocks the fused threading+typing slices; first execution slice = the scalar-newtypes module (§2 Stage 1).** **EXECUTION: §2 Stage 1 COMPLETE (145–146: `src/units.rs` scalar newtypes + `NodeType`). §2 Stage 2 (`Mem` ownership move) found ALREADY DONE (sessions 11/51). §3 rank-0/1 built (147: `src/node.rs` — `NodePtr<K>` core + box family + disasm gate passed). §3 rank-2 built (148: glue/glue_spec family — `GlueNode`/`GlueSpecNode` concrete impls, `classify_glue` + `from_glue_ptr` boundaries, ref_count-not-a-link trap encoded, disasm gate passed on glue_ptr + stretch; 9 `node::` tests). §3 rank-3+4 built (149: kern family — `width` concrete not `HasDimensions`; penalty family — `penalty` is i32-not-Scaled trap; both `classify_*`; disasm gate passed on kern.width + penalty.penalty; 15 `node::` tests). 150: glue_spec `width` field completed (recon found §1 map omitted it) + the FIRST fused `01b` §3a adoptions (`math_kern`→`NodePtr<KernNode>`, `math_glue`→`NodePtr<GlueSpecNode>`, both live `mlist_to_hlist` paths). Next = more fused adoptions, and/or §3 rank-5 rule (first sentinel-bearing family — running-dimension wrapper for `Scaled`-or-`NULL_FLAG` dims).** |
| 02 | `02-modules.md` | EXECUTING | Module decomposition. Slice 1 done (session 118: `errors` group → `errors.rs`); slice 2 done (session 119: `show_*`/display group → `output.rs`); slice 3 done (session 120: file-naming/`start_input`/`close_files` group → `io.rs`); slice 4 done (session 121: extensions/`\special` group → `ext.rs`); slice 5 done (session 122: TFM/native font-loading group → new `fonts.rs`); slice 6 done (session 123: locale-linebreak group → `linebreak.rs`); slice 7 done (session 124: **the packaging carve** — box/node packing & list utils → new `packaging.rs`, two commits, `scan.rs` 14,710→12,633 LOC); slice 8 done (session 124: **the expander carve** — `expand`/`macro_call`/`conditional`/`conv_toks`/`ins_the_toks` → new `expand.rs`, `scan.rs` 12,633→10,846 LOC); slice 9a done (session 124: **the alignment carve** — the 14 `\halign` fns → new `main_control.rs`, `scan.rs` 10,846→9,696 LOC, warnings 50). slice 9b done (session 125: **the main-control carve**, 4 sub-commits — box/mode verbs, assign/font verbs, dispatch loop, `prefixed_command` from ini.rs — into `main_control.rs`; `scan.rs` 9,696→**5,011** LOC, now pure scanner core; `main_control.rs`→6,821; `ini.rs`→3,804; §7 trap fired+caught in 9b-ii). slice 11 done (session 126: **the `ini.rs` drain**, 4 sub-commits — `hyphenation.rs` (trie), `format_io.rs` (dump/load), `engine.rs` (init/run/cleanup), `init_io`→`io.rs` — **`ini.rs` fully dissolved/deleted**). **Slice 10 deferred** (scan.rs→scanner.rs rename + token.rs: high churn, cosmetic — left as-is). **All of `scan.rs` (scanner core, 4,998) and `ini.rs` (gone) are decomposed. Next: Phase-C threading (ws01) resumes inside the navigable modules per `01b`.** Drafted + source-verified session 117: drains the two grab-bags (`scan.rs`←xetex0.c 18,689 LOC/222 fns, `ini.rs`←xetex-ini.c) into the `01-architecture.md` layout. Decision: **mechanical pre-split first, thread in place after** (supersedes `01b` §9 "together"). New modules: `scanner`/`expand`/`main_control`/`token`/`engine`/`format_io`/`packaging`(transitional→`node/`)/`hyphenation`. Unblocks scanner-side Phase C. |
| 03 | `03-deunsafe-ffi.md` | DONE | De-unsafe + FFI teardown. **FFI teardown, de-unsafe, dead code deletion, and statics migration fully complete.** |
| 04 | `04-clippy.md` | PLANNED | Cheap clippy wins now; unsafe-block bucket after 01/03. |
| 05 | `05-errors-and-misc.md` | PLANNED (seed) | Error-handling shape, dead-comment cleanup, longjmp/Drop. Partly optional. Seed written (session 62): target error model = `engine_bibtex` (`Result`/`History`, `with_global_lock` entry, stack-threaded state); not yet a READY recipe. Draft after Phase C + 03. |
| 06 | `06-constants.md` | EXECUTING | Magic-number / duplicated-const cleanup into a shared computed `consts.rs`. Phase A (centralize 193 cross-module dup consts) is low-risk and independent; Phase B (call-site literal sweep) rides Phase C. Not covered by 01–05; clippy can't catch it. |

## Execution

Workstream 03 (FFI Teardown) executed for all redeclared functions (Session 50, 2026-05-30):
- **FFI Teardown complete for 148 functions:** converted 145 redeclared `#[no_mangle] extern "C"` functions to pure Rust signatures, and cleaned up duplicate FFI declarations for all 148 functions (including C-FFI `D2Fix`, `Fix2D`, `tt_xetex_set_int_variable` which keep their export ABI) in favor of direct compile-time safe `use crate::...` imports.
- **21 type-safety compile errors resolved:** patched pointer casts (`u_close`/`input_line` cast to `*mut c_void` internally to avoid call site churn), boolean parameters (`end_diagnostic` 0/1 to false/true), character casts (`new_character(..., as u16)`), and removed E0255 duplicate definition of `overflow` in `scan.rs`.
- **Zero regressions:** all 29/2/7/4 tests pass perfectly.

Workstream 01 is executing. Phase-B bridge has started with
least-coupled non-`misc` clusters:

-   `config` moved into `EngineState.config`:
    `file_line_error_style_p`, `halt_on_error_p`, `in_initex_mode`,
    `synctex_enabled`, `semantic_pagination_enabled`,
    `shell_escape_enabled`, `max_print_line`.
-   Old C symbol definitions and Rust `#[link_name]` aliases for the
    moved config globals were removed/redirected in the same change.
-   `errors` moved into `EngineState.errors`: `arith_error`,
    `tex_remainder`, `dig`, `randoms`, `j_random`, `help_line`,
    `help_ptr`, `two_to_the`, `spec_log`, `interaction`, `history`,
    `error_count`, `use_err_help`, `long_help_seen`, `log_opened`,
    `tally`, `error_line`, `half_error_line`, `trick_buf`,
    `trick_count`, `first_count`, `deletions_allowed`, `random_seed`,
    `no_shrink_error_yet`, `font_in_short_display`,
    `depth_threshold`, and `breadth_max`.
-   Old C definitions in `xetex/xetex-ini.c` /
    `xetex/xetex-linebreak.c` and the `src/ini.rs` `#[link_name]`
    aliases for `error_line` / `half_error_line` were removed in the
    same change.
-   Tiny `shipout` source-special flag slice moved into
    `EngineState.shipout`: `insert_src_special_every_par`,
    `insert_src_special_every_math`, and
    `insert_src_special_every_vbox`.
-   `shipout` scalar cluster moved into `EngineState.shipout` (21 more
    fields): `selector`, `doing_special`, `term_offset`, `file_offset`,
    `old_setting`, `epochseconds`, `microseconds`, `dead_cycles`,
    `page_tail`, `last_badness`, `page_contents`, `page_so_far`,
    `last_glue`, `last_penalty`, `last_kern`, `last_node_type`,
    `insert_penalties`, `output_active`, `pdf_last_x_pos`,
    `pdf_last_y_pos`, and `insert_src_special_auto`. Raw externs and C
    definitions removed in the same change; no cleanup aliases existed.
    Remaining `shipout` rows are pointer/file-handle state with
    init/cleanup edges (`rust_stdout`, `log_file`, `write_file`,
    `current_diagnostic`).
-   `scanner` file-name group moved into `EngineState.scanner` (new
    `ScannerState`): `cur_name`, `cur_area`, `cur_ext`,
    `area_delimiter`, `ext_delimiter`, `quoted_filename`,
    `file_name_quote_char`, `name_in_progress`, `stop_at_space`. Raw
    externs and C defs removed; no cleanup aliases. Newtype debts
    (`cur_*` → `StrNumber`, `*_delimiter` → `PoolPtr`) recorded in the
    inventory worklist. The hot scanner scalars (`cur_cmd`, `cur_chr`,
    `cur_tok`, `cur_cs`, `cur_val`) are intentionally deferred to avoid
    a thread-local cost on the tokenizer hot path.
-   `save_stack` scalar cluster moved into `EngineState.save_stack` (new
    `SaveStackState`): `save_ptr`, `max_save_stack`, `cur_level`,
    `cur_group`, `cur_boundary`. The `save_stack` pointer and `save_size`
    capacity stay behind (ownership/cleanup edges) for a later slice.
    Raw externs and C defs removed; no cleanup aliases touched.
    `cur_group` → `GroupCode` recorded in the worklist.
-   `eqtb` scalar set moved into `EngineState.eqtb` (new `EqtbState`):
    `hash_used`, `hash_high`, `no_new_control_sequence`, `prim_used`,
    `cs_count`. The `eqtb`/`hash`/`prim` arrays and the alias-bearing
    `hash_extra`/`eqtb_top`/`hash_top` stay behind for the ownership-
    aware `eqtb` array slice. Raw externs and C defs removed; no cleanup
    aliases touched. No newtype debts (all plain int/bool).
-   **First ownership move:** the `save_stack` *array* moved from a
    C-`malloc`'d pointer into a Rust-owned `Vec<MemoryWord>` on
    `EngineState.save_stack.data`. Accessors index the `Vec` (safe,
    bounds-checked; only the `MemoryWord` union projection stays
    `unsafe`); `ini.rs` allocates the `Vec` at init and `tt_cleanup`
    drops it; both `#[link_name="save_stack"]` aliases, the raw extern,
    and the C def were removed in the same commit. `save_size` capacity
    stays a C global. Establishes the §5 arena pattern.
-   **First `input`-cluster ownership move:** the `param_stack` *array*
    moved into a Rust-owned `Vec<i32>` on
    `EngineState.input.param_stack` (new `InputClusterState` sub-struct).
    Accessors index the `Vec` (safe, bounds-checked, no `unsafe`);
    `ini.rs` allocates the `Vec` (`param_size + 1`) at init and
    `tt_cleanup` drops it; **both** `#[link_name="param_stack"]` aliases
    (alloc `tre_*` + cleanup `tt_cleanup_*`), the raw extern, and the C
    def were removed in the same commit. `param_ptr`, `max_param_stack`,
    `param_size` stay C globals. Not in format dump. No newtype debt
    (plain `int32_t *`).
-   **Input line/group stacks (5 arrays):** `line_stack`, `grp_stack`,
    `if_stack`, `source_filename_stack`, `full_source_filename_stack`
    moved from C-`malloc`'d pointers into Rust-owned `Vec<i32>` on
    `EngineState.input` (same session-12 arena shape). All sized
    `max_in_open + 1`; allocated in one `ini.rs` block, dropped in
    `tt_cleanup`. The explicit slot-0 init writes were rewritten to the
    new `set_*` accessors. Both `#[link_name]` aliases per array (10
    decls), the 5 raw externs, and the 5 C defs (two blocks in
    `xetex-ini.c`) were removed in the same commit. Not in format dump.
    Newtype debts recorded: `grp_stack` (`save_pointer`),
    `source_filename_stack`/`full_source_filename_stack` (`str_number`);
    `line_stack`/`if_stack` are plain `int32_t` (no row). `eof_seen`
    (`*mut bool`) is the last simple input array, left for a later slice.
-   **`eof_seen` (last simple input array):** moved from a C-`malloc`'d
    `*mut bool` into a Rust-owned `Vec<bool>` on `EngineState.input`
    (`vec![false; max_in_open + 1]`), folded into the session-13 alloc and
    cleanup blocks. Accessors index the `Vec` (no `unsafe`). Both
    `#[link_name]` aliases (alloc + cleanup), the raw extern, and the C def
    removed in the same commit. Not in format dump. Plain `bool`, no
    newtype debt. All simple input-cluster arrays are now Rust-owned; the
    remaining input arrays (`input_file`, `input_stack`, `nest`, `buffer`)
    have non-trivial element types.
-   **`buffer` (input buffer):** moved from a C-`malloc`'d `*mut i32`
    (`UnicodeScalar *`) into a Rust-owned `Vec<i32>` on
    `EngineState.input.buffer` (`vec![0i32; buf_size + 1]`). Fixed-size:
    overflow paths error rather than growing, so the no-realloc invariant
    holds. Accessors `buffer_entry`/`set_buffer_entry` index the `Vec`
    (no `unsafe`); `buffer_ptr` (still `unsafe fn`, used only by io.rs's
    byte-decode path) returns `as_mut_ptr().add(offset)`, valid past the
    borrow because the thread-local owner never reallocates. Removed the
    redundant `write_bytes` re-zero, both `#[link_name]` aliases, the raw
    extern, and the C def in the same commit. Not in format dump. Newtype
    debt recorded (`buffer[]` → `UnicodeScalar`).
-   **`input_stack` array → Rust-owned Vec:** moved from a C-`malloc`'d
    `*mut InputState` into a Rust-owned `Vec<InputState>` on
    `EngineState.input.input_stack` (`vec![<zero InputState>; stack_size + 1]`).
    Lowest-difficulty of the three remaining input arrays — only two
    accessors (`input_stack(k)` full-struct copy-out + `set_input_stack(k, v)`
    full-struct assign), no pointer-escaping accessor. Both now index the
    `Vec` (no `unsafe`). `InputState` is already a `Copy` struct so
    `Vec<InputState>` derives `Default`. Both `#[link_name]` aliases (alloc +
    cleanup), the raw extern, and the C def removed in the same commit. Not in
    format dump. **No** newtype debt — element type `input_state_t` is a
    struct typedef already mirrored 1:1 by the Rust `InputState`, so no type
    info is lost (checked).
-   **`input_file` array → Rust-owned Vec:** moved from a C-`malloc`'d
    `*mut *mut c_void` (`UFILE **`) into a Rust-owned `Vec<*mut c_void>` on
    `EngineState.input.input_file` (`vec![null_mut(); max_in_open + 1]`).
    Second-to-last input array; fewest accessors of the two remaining (three:
    `input_file`, `set_input_file`, and the pointer-escaping
    `input_file_ptr`). The first two index the `Vec` (no `unsafe`);
    `input_file_ptr(k)` keeps the session-15 `buffer_ptr` shape
    (`as_mut_ptr().add(k)` cast to `*mut *mut UFile`, valid past the borrow
    because the thread-local owner never reallocates). Both
    `#[link_name="input_file"]` aliases (alloc + cleanup), the raw extern, and
    the C def removed in the same commit. Not in format dump. **No** newtype
    debt — element is an opaque `UFILE*` handle stored as `*mut c_void`, a
    plain pointer (checked).
-   **`nest` array → Rust-owned Vec (last input array):** moved from a
    C-`malloc`'d `*mut ListStateRecord` into a Rust-owned
    `Vec<ListStateRecord>` on `EngineState.input.nest`
    (`vec![<zero ListStateRecord>; nest_size + 1]`). 12 accessors repointed:
    the two full-struct copies to/from the separate `cur_list` static
    (`restore_cur_list_from_nest`/`save_cur_list_to_nest`, `cur_list` itself
    stays a raw global) and 10 field-projection accessors that now index the
    `Vec` (8 fully safe; the two `aux.b32.*` reads keep an inner `unsafe`
    union projection). No pointer-escaping accessor. Both
    `#[link_name="nest"]` aliases (alloc + cleanup), the raw extern, and the
    C def removed in the same commit. Not in format dump. **No** newtype debt
    — element `list_state_record` is a struct typedef mirrored 1:1 by
    `ListStateRecord`, so no type info lost (checked). **All `input`-cluster
    arrays are now Rust-owned.**
-   **`shipout` write-stream arrays:** `write_file` (`rust_output_handle_t[16]`)
    and `write_open` (`bool[18]`) moved into `EngineState.shipout` as
    `[usize; 16]` / `[bool; 18]`. Inline fixed C arrays (BSS, not `malloc`'d),
    so a scalar-style move: no `ini.rs` alloc, no `tt_cleanup` free, no cleanup
    alias. All 4 accessors now fully safe. Raw externs and C defs removed in
    the same commit. Not in format dump. Newtype debt recorded for
    `write_file` (`rust_output_handle_t` → `OutputHandle`); `write_open` plain
    `bool`, no row. Remaining `shipout` rows are file-*handle* state with
    init/cleanup edges (`rust_stdout`, `log_file`, `current_diagnostic`).
-   `scanner` alignment-state scalars moved into `EngineState.scanner` (8
    fields): `cur_align`, `cur_span`, `cur_loop`, `align_ptr`, `cur_head`,
    `cur_tail`, `cur_pre_head`, `cur_pre_tail`. All plain `int32_t` (cold
    `\halign`/`\valign` scratch, not on the tokenizer/main-loop hot path); raw
    externs and C defs removed in the same change; no cleanup aliases, no
    newtype debt (all checked plain `int32_t`).
-   `scanner` clean cold scratch scalars moved into `EngineState.scanner` (4
    fields): `shown_mode` (`short`), `mag_set` (`int32_t`), `radix`
    (`small_number`, stored `u8`), `expand_depth_count` (`int32_t`). Non-hot
    (show-mode latch / once-per-job magnification latch / scanner number base /
    expression-depth guard), so the thread-local bridge has no hot-path cost.
    Raw externs and C defs removed in the same change; no cleanup aliases. One
    newtype debt row added: `radix` (`small_number` → `SmallNumber`/`Radix`);
    the other three are plain `short`/`int32_t`.
-   `scanner` clean cold sparse-array register scalars moved into
    `EngineState.scanner` (3 fields): `sa_chain` (`int32_t`), `sa_level`
    (`uint16_t`), `max_reg_num` (`int32_t`). All plain integer types (cold
    `\global`-register sparse-array machinery, not on any hot path); raw externs
    and C defs removed in the same change; no cleanup aliases, no newtype debt
    (all checked plain `int32_t`/`uint16_t`). The other `sa_*` members stay
    behind: `sa_root` (link alias → ownership), `sa_null` (`MemoryWord`),
    `max_reg_help_line` (pointer), `cur_ptr` (148 uses, too broad to bridge
    without benchmarking).
-   `shipout` remaining file-handle state moved into `EngineState.shipout` (3 fields):
    `rust_stdout` (`usize`), `log_file` (`usize`), `current_diagnostic`
    (`*mut c_void`). Removes the last remaining BSS/extern file-handle state
    from `shipout`. `rust_stdout` has its `#[link_name]` alias in `src/ini.rs`
    removed and repointed to `set_rust_stdout`; definitions in C
    `xetex-ini.c` and `xetex-output.c` were removed. All 5 accessors now fully
    safe. Newtype debt recorded for `rust_stdout` and `log_file`.
-   **Strings ownership move (session 24):** `str_start` / `str_pool` arenas (and 11 other string-related scalar fields: `str_ptr`, `max_strings`, `init_str_ptr`, `init_pool_ptr`, `job_name`, `texmf_log_name`, `string_vacancies`, `pool_size`, `pool_ptr`, `format_ident`, `edit_name_start`) moved from C `malloc`'d arrays/globals into a Rust-owned `Vec<i32>` / `Vec<u16>` and scalar fields on `EngineState.strings` (new `StringsState` sub-struct). Evaluates limits and allocations safely in Rust during init/undump and cleans them up in `tt_cleanup`. Removed all 13 C definitions and all related `#[link_name]` aliases in `src/ini.rs`. Accessors repointed and now 12 are fully safe (only `str_pool_ptr` keeps a small `unsafe` pointer addition block).
-   **Hyphenation scalars move (session 25):** 25 clean scalar fields of the `hyph` (hyphenation and trie) cluster (`is_hyph`, `hf`, `max_hyph_char`, `init_list`, `init_lig`, `init_lft`, `hyphen_passed`, `trie_max`, `trie_size`, `trie_ptr`, `hyph_count`, `hyph_size`, `hyph_next`, `trie_op_ptr`, `max_op_used`, `trie_not_ready`, `hyph_start`, `hyph_index`, `cur_lang` from `xetex-ini.c` and `hyf_char`, `l_hyf`, `r_hyf`, `init_l_hyf`, `init_r_hyf`, `hyf_bchar` from `xetex-linebreak.c`) moved from C globals into a Rust-owned sub-struct `EngineState.hyph` (`HyphState`). Capacity limits initialized in state. Accessors repointed, with all of them now fully safe. C definitions and `trie_size` / `hyph_size` external link aliases removed. Newtype debts recorded in the global inventory.
-   **Hyphenation fixed arrays move (session 26):** 11 fixed-size `hyph`
    arrays (`hc`, `hu`, `hyf`, `trie_min`, `hyf_distance`, `hyf_num`,
    `hyf_next`, `op_start`, `trie_used`, `trie_op_lang`, `trie_op_val`)
    moved from C globals into heap-backed, fixed-size `Vec`s on
    `EngineState.hyph`. All 22 array accessors now index state safely; the
    four format-dump aliases in `src/ini.rs` (`hyf_distance`, `hyf_num`,
    `hyf_next`, `trie_used`) were removed/replaced with state-derived
    pointers/accessors. C definitions removed from `xetex-ini.c`. The arrays
    are heap-backed rather than inline fixed arrays because inline storage
    overflowed the `engine_rust` test thread stack during thread-local state
    initialization. Newtype debts recorded for semantic typedef arrays.
-   **Hyphenation dynamic trie arrays move (session 27):** 9 dynamically
    allocated `hyph` trie arrays (`trie_trl`, `trie_tro`, `trie_trc`,
    `trie_c`, `trie_o`, `trie_l`, `trie_r`, `trie_hash`, `trie_taken`)
    moved from C globals / `malloc` storage into Rust-owned `Vec`s on
    `EngineState.hyph`. Initex allocation now sizes all 9 as
    `trie_size + 1`; format load/dump for the packed trie arrays uses
    state-derived pointers; `tt_cleanup` resets the vectors. Removed 9 raw
    externs, 12 `src/ini.rs` link aliases, and the C definitions from
    `xetex-ini.c`. Newtype debts recorded for the semantic typedef arrays.
-   **Hyphenation exception arrays move (session 28):** the remaining 3
    dynamically allocated `hyph` exception arrays (`hyph_word`, `hyph_list`,
    `hyph_link`) moved from C globals / `malloc` storage into Rust-owned
    `Vec`s on `EngineState.hyph`. Init alloc now sizes all 3 as
    `hyph_size + 1`; format load/dump continues through the accessor layer;
    `tt_cleanup` resets the vectors. Removed 3 raw externs, 6 `src/ini.rs`
    link aliases, and the C definitions from `xetex-ini.c`. Newtype debts
    recorded for `hyph_word[]` and `hyph_link[]`; `hyph_list[]` is plain
    `int32_t *`.
-   **Font-loading scratch scalars move (session 29):** 6 cold `fonts`
    scalars (`loaded_font_mapping`, `loaded_font_flags`,
    `loaded_font_letter_space`, `xtx_ligature_present`,
    `gave_char_warning_help`, `native_font_type_flag`) moved from C globals
    into new `EngineState.fonts` (`FontState`). These have no alloc/free or
    format aliases; accessors are now fully safe and C definitions were
    removed from `xetex-ini.c`. Newtype debt recorded for
    `loaded_font_letter_space` (`scaled_t`); the other five are plain pointer,
    char, bool, or `int32_t`.
-   **Font capacity scalars move (session 30):** `font_mem_size` and
    `font_max` moved from C globals into `EngineState.fonts`. Their
    `src/ini.rs` link aliases were replaced with the accessor layer in default
    initialization, format loading, and font-table allocation checks. Removed 2
    raw externs and the C definitions from `xetex-ini.c`. No newtype debts:
    both are plain `int32_t` capacity integers.
-   **Font null-character scalar move (session 31):** `null_character`
    (`b16x4`) moved from a C global into `EngineState.fonts`. It has no
    alloc/free or `src/ini.rs` link alias, and its existing `B16x4` Rust type
    preserves the C struct shape. Accessors are now fully safe; the raw extern
    and C definition were removed. No newtype debt.
-   **Font checksum array ownership move (session 32):** `font_check`
    (`b16x4 *`) moved from a C-`malloc`'d pointer into a Rust-owned
    `Vec<B16x4>` on `EngineState.fonts`. Init and format-load allocation now
    size the vector as `font_max + 1`; format dump/undump uses the vector
    storage from `FONT_BASE`; `tt_cleanup` drops the vector. Removed both
    `#[link_name="font_check"]` aliases, the raw extern, and the C definition.
    No newtype debt: the element is already represented as the `B16x4` struct.
-   **Font size arrays ownership move (session 33):** `font_size` and
    `font_dsize` (`scaled_t *`) moved from C-`malloc`'d pointers into
    Rust-owned `Vec<i32>` fields on `EngineState.fonts`. Init and format-load
    allocation now size both vectors as `font_max + 1`; format dump/undump uses
    state-derived pointers from `FONT_BASE`; `tt_cleanup` drops both vectors.
    Removed both allocation/cleanup link aliases per array, the raw externs,
    and the C definitions. The hidden C++ OpenType math reader in
    `xetex-XeTeXOTMath.cpp` now calls a narrow Rust bridge,
    `tt_get_font_size_entry`. Newtype debts recorded for both `scaled_t`
    arrays.
-   **Font parameter-count array ownership move (session 34):** `font_params`
    (`font_index *`) moved from C-`malloc`'d pointer storage into a Rust-owned
    `Vec<i32>` on `EngineState.fonts`. Init and format-load allocation now
    size the vector as `font_max + 1`; format dump and checked undump use
    state-derived storage from `FONT_BASE`; `tt_cleanup` drops the vector.
    Removed both `#[link_name="font_params"]` aliases, the raw extern, and the
    C definition. Newtype debt recorded for `font_index`.
-   **Font name/area arrays ownership move (session 35):** `font_name` and
    `font_area` (`str_number *`) moved from C-`malloc`'d pointer storage into
    Rust-owned `Vec<i32>` fields on `EngineState.fonts`. Init and format-load
    allocation now size both vectors as `font_max + 1`; format dump and
    checked undump use state-derived storage from `FONT_BASE`; checked undump
    computes `str_ptr()` before the mutable state borrow to avoid nested
    `RefCell` borrows; `tt_cleanup` drops both vectors. Removed both
    allocation/cleanup link aliases per array, the raw externs, and the C
    definitions. The hidden C++ OpenType math `font_area` readers now call a
    narrow Rust bridge, `tt_get_font_area_entry`. Newtype debts recorded for
    both `str_number` arrays.
-   **Font character-range arrays ownership move (session 36):** `font_bc`
    and `font_ec` (`UTF16_code *`) moved from C-`malloc`'d pointer storage
    into Rust-owned `Vec<u16>` fields on `EngineState.fonts`. Init and
    format-load allocation now size both vectors as `font_max + 1`; format
    dump/undump use state-derived storage from `FONT_BASE`; `tt_cleanup` drops
    both vectors. Removed both allocation/cleanup link aliases per array, the
    raw externs, and the C definitions. Newtype debts recorded for both
    `UTF16_code` arrays.
-   **Font boundary-character arrays ownership move (session 37):**
    `bchar_label` (`font_index *`), `font_bchar` (`nine_bits *`), and
    `font_false_bchar` (`nine_bits *`) moved from C-`malloc`'d pointer storage
    into Rust-owned `Vec<i32>` fields on `EngineState.fonts`. Init and
    format-load allocation now size all three vectors as `font_max + 1`;
    format dump and checked undump use state-derived storage from `FONT_BASE`;
    `tt_cleanup` drops the vectors. Removed all six allocation/cleanup link
    aliases, the raw externs, and the C definitions. Newtype debts recorded
    for `font_index`/`nine_bits`.
-   **`font_glue` + `font_used` arrays ownership move (session 38):** the two
    remaining simple per-font table arrays moved from C-`malloc`'d pointer
    storage into Rust-owned `Vec<i32>` / `Vec<bool>` fields on
    `EngineState.fonts`. `font_glue` (`int32_t *`) is format-dumped with a
    checked undump against `lo_mem_max` (count/bound precomputed before
    `with_state_mut` to avoid nested borrows); `font_used` (`bool *`) is not
    format-dumped and is allocated unconditionally as `vec![false; font_max+1]`.
    Init/format-load allocation folds `font_glue` into the existing init
    closures; `tt_cleanup` drops both vectors. Removed all four
    allocation/cleanup link aliases, the two raw externs, and the two C
    definitions. Newtype debt recorded for `font_glue[]` (nullable glue-spec
    mem pointer -> `Vec<MemPtr>`); `font_used[]` is plain `bool`, no row.
-   **9 `*_base` offset arrays ownership move (session 39):** `char_base`,
    `width_base`, `height_base`, `depth_base`, `italic_base`, `lig_kern_base`,
    `kern_base`, `exten_base`, and `param_base` (all `int32_t *`) moved from
    C-`malloc`'d pointers into Rust-owned `Vec<i32>` fields on
    `EngineState.fonts`. All format-serialized; dump/undump use state-derived
    pointers, with `font_ptr() + 1` pre-computed outside every `with_state_mut`
    closure to avoid nested RefCell borrows. Removed 18 link aliases, 9 raw
    externs, and 9 C definitions. No newtype debt (plain `int32_t *`).
-   **`font_flags`, `font_letter_space`, `fmem_ptr`, `font_ptr` move (session 39):**
    `font_flags` (`char *` → `Vec<i8>`) and `font_letter_space` (`scaled_t *` →
    `Vec<i32>`) not format-serialized (fresh alloc). `fmem_ptr` and `font_ptr`
    are plain `i32` scalars with no cleanup aliases. Removed 4 raw externs, 4
    link aliases, 4 C definitions. **New gotcha:** scalars backed by `with_state`
    must not be called inside `with_state_mut` closures — three format-load undump
    closures had `font_ptr() + 1` inside them, causing RefCell panics; fixed by
    pre-computing the value before the borrow. Newtype debt rows for `fmem_ptr`,
    `font_ptr` (semantic typedefs), and `font_letter_space[]` (`scaled_t`).
-   **`font_mapping` + `font_layout_engine` ownership move (session 39):** both
    `void **` pointer arrays moved into `Vec<*mut c_void>` on `EngineState.fonts`.
    Not format-serialized; handles released explicitly in `tt_cleanup` before Vec
    drop. `font_layout_engine` was directly accessed in `xetex-XeTeXOTMath.cpp`
    (19 uses); routed through new `tt_get_font_layout_engine_entry` bridge (same
    pattern as prior `font_area`/`font_size` bridges). Removed 2 raw externs, 4
    link aliases, 2 C definitions. No newtype debt (plain opaque void *).
-   **`read_file[16]` inline array move (session 43):** `read_file[16]` (`UFILE *[16]`)
    moved as `[*mut c_void; 16]` into `EngineState.input`. BSS array (xetex-ini.c only,
    no cleanup aliases). `read_file_entry` repointed to `with_state`; `read_file_ptr` repointed
    to `with_state_mut` + `as_mut_ptr().add(n)` (same pointer-escaping pattern as
    `input_file_ptr`; valid because the array lives in the `'static` thread-local and is never
    moved). Removed raw extern static and C definition. No newtype debt (plain opaque pointer).
-   **`name_length`, `name_length16`, `read_open` scalar/array move (session 42):**
    `name_length` (`int32_t`) and `name_length16` (`int32_t`) moved as plain `i32`
    scalars into `EngineState.input`; `read_open[17]` (`unsigned char[17]`) moved
    as `[u8; 17]` inline array into `EngineState.input`. All BSS/C globals
    (xetex-ini.c only, no cleanup aliases). Accessors repointed to
    `with_state`/`with_state_mut`; raw extern statics and C definitions removed in
    the same commit. Not in format dump. No newtype debts: all plain
    `int32_t`/`unsigned char` (confirmed).
-   **`misc` bucket resolved (session 44 — planning only, no code change):** the
    former `misc` holding bucket (34 linebreak-scratch raw externs + 8 private
    statics) was resolved into concrete sub-structs and the inventory's cluster
    columns updated — `LinebreakState` (34 externs), `ShipoutState` fold (4
    pagebuilder statics), `EngineState.synctex` (`SYNCTEX_CTXT`), `TexmfState` (3 io
    statics). Decision block + per-field lists + newtype-debt candidates in
    `01a-global-inventory.md`. No `MathState`/`DviState` created — those rows are
    already classified `scanner`/`shipout`. Sets up sessions 45+ batch migrations.
-   **`LinebreakState` batch move (session 45):** all 34 former-`misc` line-breaker
    scratch globals (`last_leftmost_char`, `last_rightmost_char`, `hlist_stack`,
    `hlist_stack_level`, `first_p`, `global_prev_p`, `cur_p`, `last_special_line`,
    `first_width`, `second_width`, `first_indent`, `second_indent`, `best_bet`,
    `best_line`, `hn`, `ha`, `hb`, `passive`, `cur_active_width`, `background`,
    `break_width`, `best_place`, `best_pl_line`, `disc_width`, `final_pass`,
    `threshold`, `minimal_demerits`, `minimum_demerits`, `easy_line`,
    `do_last_line_fit`, `active_node_size`, `fill_width`, `best_pl_short`,
    `best_pl_glue`) moved from C globals into a new `EngineState.linebreak`
    (`LinebreakState`). Pure scalar/inline-array move — no `ini.rs` alloc, no
    `tt_cleanup` free, no format-dump, no cleanup aliases (all pre-verified).
    `hlist_stack` heap-backed as `Vec<i32>` sized 513 in a hand-written
    `impl Default` (the inline-`[i32;513]` stack-overflow risk + arrays >32 lack
    derived `Default`); small fixed arrays stay inline. All 68 accessor bodies
    repointed to `with_state`/`with_state_mut`; 36 C defs removed across
    `xetex-ini.c` (6) and `xetex-linebreak.c` (28). 13 newtype-debt rows added
    (11 `scaled_t`→`Scaled`, 2 `small_number`→`SmallNumber`); the `int32_t`
    mem-pointer fields are pointer-taxonomy-sweep candidates, no typedef row.
-   **Three resolved-`misc` private-static clusters (session 46) — `misc` bucket fully
    drained.** All *private module-local* `static mut`s (none in `globals.rs`
    `mod raw`, so no C defs / no `mod raw` externs / no cleanup aliases to remove);
    cross-module-reader greps all empty. Three separate commits.
    (1) **`ShipoutState` fold** ← 4 pagebuilder statics (`BEST_PAGE_BREAK`,
    `BEST_SIZE`, `LEAST_PAGE_COST`, `PAGE_MAX_DEPTH`, `src/pagebuilder.rs`) as plain
    `i32` fields (derive `Default`) + 4 module-private `with_state` accessor pairs;
    ~18 direct-use sites repointed; no newtype debt.
    (2) **`TexmfState` (new)** ← `LAST_SOURCE_NAME`/`LAST_LINENO`/`START_TIME_STR[30]`
    (`src/texmfmp.rs`); `init_start_time` formats into a local `[u8;30]` then stores
    it, `getcreationdate` reads a local copy out; `last_source_name` free/reassign
    lifecycle kept entirely in `remember_source_info` (no `tt_cleanup` edge → no
    double-free, last alloc leaks at teardown as the C static did). Ownership-debt row
    for `last_source_name`.
    (3) **`EngineState.synctex: SynctexCtxt`** ← `SYNCTEX_CTXT` (`src/synctex.rs`, 264
    sites): `SynctexCtxt`/`SynctexFlags`/the `Recorder` alias moved into `state.rs`
    `pub(crate)`; hand-written `impl Default` to preserve the former `lastv: -1` init
    (a derived `Default` would give `lastv: 0`). Per-field bridge generated by two
    `macro_rules!` (`sc_scalar!`/`sc_flag!` → 15 scalar + 5 flag getter/setter pairs)
    over the thread-local; all 264 sites repointed by an ordered 6-pass `perl`
    transform (flag-writes → flag-reads → `+=`/`-=` → simple-writes → simple-reads).
    The synctex fns read fields into locals and write scalars back, so the bridge
    never holds a borrow across an engine call. Debt rows: `synctex.root_name` (owned
    C string), `synctex.file` (output handle); rest moved verbatim, no typedef loss.
    Dashboard 29/2/7/4 on each commit.
-   **`shipout` DVI-output scratch (session 47):** 16 classified-but-unmigrated
    `shipout` scalars folded into the existing `EngineState.shipout`
    (`ShipoutState`): `total_pages`, `max_v`, `max_h`, `max_push`, `last_bop`,
    `doing_leaders`, `rule_ht`, `rule_dp`, `rule_wd`, `cur_h`, `cur_v`,
    `cur_page_width`, `cur_page_height`, `cur_h_offset`, `cur_v_offset`,
    `write_loc`. Plain scalar/BSS move (no `ini.rs` alloc, no `tt_cleanup`, no
    cleanup alias). `write_loc` is in the format dump but dumped/undumped purely
    through its accessor, so the thread-local bridge is transparent. The only
    cross-references are false positives (`max_h` is a `make_left_right`
    parameter name in `xetex-math.c`/`math.rs`) or non-nested accessor reads
    (`cur_h()`/`cur_v()` in synctex helper fns). 32 accessor bodies repointed;
    16 raw externs and 16 C defs (two interleaved `xetex-ini.c` blocks) removed
    in the same commit. 11 `scaled_t`→`Scaled` newtype-debt rows added; the 5
    plain `int32_t`/`bool` fields need none. Dashboard 29/2/7/4.
-   **`scanner` math-list-conversion scratch (session 48):** 5
    classified-but-unmigrated math globals folded into the existing
    `EngineState.scanner` (`ScannerState`): `cur_mlist`, `cur_style`, `cur_size`,
    `cur_mu`, `mlist_penalties`. Defined together in `xetex-math.c` (only
    non-`ini.c` C file touched), used by `mlist_to_hlist`; cold (not on the
    tokenizer hot path). No cleanup alias, no format dump, no C uses outside the
    5 definition lines. 10 accessor bodies repointed; 5 raw externs and 5 C defs
    removed in the same commit. 2 newtype-debt rows added (`cur_style`→
    `SmallNumber`, `cur_mu`→`Scaled`); `cur_mlist`/`cur_size`/`mlist_penalties`
    plain. Dashboard 29/2/7/4.
-   **`eqtb` inline arrays `prim` + `xeq_level` (session 49):** first move into the
    `eqtb`-cluster arrays — the two **inline BSS** arrays (no `malloc`, no
    `#[link_name]` alias). `prim` (`[B32x2; 2101]`) and `_xeq_level_array`
    (`[u16; 1_114_730]`) → heap-backed `Vec`s on `EqtbState`, sized in a
    hand-written `impl Default` (the session-45 shape; not `ini.rs`-allocated
    because they were inline BSS, and heap-backed to avoid the thread-stack
    overflow). 6 accessors repointed; `prim`'s format dump/undump and `xeq_level`'s
    save/restore stay accessor-mediated (transparent). 2 raw externs and 2 C defs
    removed. No newtype debt (faithful `B32x2`/`u16`). The malloc'd `eqtb`/`hash`
    arrays (sparse `dump_slice` reconciliation; `yhash` offset trick) remain for
    dedicated sessions. Dashboard 29/2/7/4.
-   **`eqtb` array proper → Rust-owned `Vec<MemoryWord>` (session 51):** the
    format-dump-reconciliation centerpiece. The `malloc`'d `memory_word *eqtb`
    moved to `EngineState.eqtb.data`; 9 accessors repointed (session-11 union-Vec
    idiom). The sparse region-based format dump/undump (`dump_slice`/`undump_slice`
    + undefined-CS replication) reconciled via a state-derived base pointer
    precomputed once outside `with_state`/`with_state_mut` (the font-array
    pattern) — valid across the loops because the thread-local Vec never
    reallocates. initex `xcalloc_array` / format-load `xmalloc_array` replaced by
    `vec![<zero MemoryWord>; eqtb_top+1]`; `tt_cleanup` + format-reset free via
    `Vec::new()`. Removed both `#[link_name="eqtb"]` aliases, the raw extern, and
    the C def (xetex-ini.c:20). No newtype debt (element `memory_word`).
    Dashboard 29/2/7/4.
-   **`eqtb` hash table proper → Rust-owned `Vec<B32x2>` (session 52):** the
    equivalents hash table `hash` moved to `EngineState.eqtb.hash`; 4 accessors
    repointed (`hash_s0`/`set_hash_s0`/`hash_s1`/`set_hash_s1`), which are now
    fully safe (no `unsafe` block). The scalars `hash_extra`, `eqtb_top`, and
    `hash_top` also moved into `EqtbState`. Native allocations replaced the C FFI
    allocator helper `tt_run_engine_allocate_yhash()` and FFI free cleanup helper
    `tt_cleanup_free_yhash()`. Reconciled `undump_slice` for format load using
    pre-computed `hash_ptr` and subtracting `514` from indices. Removed FFI link aliases,
    raw extern statics in `globals.rs`, and C variables/helpers in `xetex-ini.c`.
    Dashboard 29/2/7/4.
-   **DVI positioning & output variables → ShipoutState (session 53):** the
    13 DVI positioning and output variables (`dvi_file`, `output_file_name`,
    `dvi_buf`, `dvi_limit`, `dvi_ptr`, `dvi_offset`, `dvi_gone`, `down_ptr`,
    `right_ptr`, `cur_s`, `dvi_h`, `dvi_v`, `dvi_f`) moved into `EngineState.shipout`
    (`ShipoutState`). `dvi_buf` (`*mut u8`) is now owned as a `Vec<u8>`. Removed
    all 13 module-level `pub static mut` declarations from `src/shipout.rs`. Accessors index the
    `Vec` or return the thread-local state. `initialize_shipout_variables` natively allocates the
    buffer with `vec![0u8; (DVI_BUF_SIZE + 1) as usize]` and `deinitialize_shipout_variables`
    releases it with `Vec::new()`. Checked and verified with no regressions; all 29/2/7/4 tests pass.
    Added newtype-debt rows for `output_file_name`, `dvi_f`, `down_ptr`, `right_ptr`, `dvi_h`, `dvi_v` to
    `01a-global-inventory.md`.
-   **Scanner box/packaging/scratch variables moved to ScannerState (session 54):** the remaining 18 scanner box/packaging/scratch globals moved into `EngineState.scanner` (`ScannerState`). Accessors delegate to the thread-local state.
-   **Equivalents sparse-array/sentinel globals moved to ScannerState (session 55):** the equivalents sparse-array metadata and sentinel globals (`sa_root`, `sa_null`, `empty`, `max_reg_help_line`) migrated to `EngineState.scanner`. `fmt_sa_root` C-linkage alias removed from `src/ini.rs` and replaced with safe, native format load/dump state accessors.
-   **`name_of_file` & `name_of_file16` buffer globals moved to `EngineState.input` (session 56):** the filename buffer globals `name_of_file` and `name_of_file16` moved into `EngineState.input` as Rust-owned `Vec<libc::c_char>` and `Vec<u16>`. Manual `libc::free` and `xmalloc`/`xcalloc` allocations in `src/scan.rs` and `src/io.rs` replaced with safe, native vector manipulations. The module-local `static mut NAME16LEN` in `src/io.rs` and C definitions in `xetex/xetex-ini.c` were completely removed. All accessors repointed.
-   **`xdv_buffer`, `mapped_text`, and `native_text` cluster moved to state (session 57):** the buffers `xdv_buffer`, `mapped_text`, and the `native_text` cluster (`native_text`, `native_text_size`, `native_len`, `save_native_len`) migrated to `EngineState.fonts` and `EngineState.scanner` respectively. C definitions in `xetex/xetex-ini.c` and raw static mut declarations in `src/globals.rs` were removed. Manual allocations, reallocations, and `libc::free` calls were replaced with safe, native vector manipulations. Resolved a linker error in `scan.rs` (`do_locale_linebreaks`) and designed a safe `apply_mapping` slice-pointer FFI bridge.
-   **The `mem` arena → Rust-owned `Vec<MemoryWord>` on `EngineState.mem` (session 58):** the dynamic-memory arena `mem` (`memory_word *`, the engine's hottest storage) and its seven bookkeeping scalars (`lo_mem_max`, `hi_mem_min`, `avail`, `mem_end`, `rover`, `var_used`, `dyn_used`) moved into a new `MemState`. `data` is allocated in `ini.rs` (two preserved inclusive sizes: format-load `MEM_TOP + 1`, initex `MEM_TOP + 2`) and dropped by `tt_cleanup`/format-reset via `Vec::new()`; never reallocates. ~30 accessors repointed (indexed word accessors index `s.mem.data[p]`; the three pointer-escaping accessors keep the `as_mut_ptr().add(p)` shape). The free-list format dump/undump regions reconciled via a state-derived base pointer precomputed once outside `with_state`/`with_state_mut` (the session-51 eqtb pattern), valid across the interleaved `mem_*` accessor reads. Removed both `#[link_name="mem"]` aliases (`tre_mem`, `tt_cleanup_mem`), the 8 raw externs (+ the now-unused `MemoryWord` import in `mod raw`), and the 7-line C def block. No newtype debt (element `memory_word`; the 7 scalars are plain `int32_t`, the mem-pointer ones pointer-taxonomy candidates only). `xmalloc_array`/`xcalloc_array` are now dead in `ini.rs`. Dashboard 29/2/7/4.
-   **Dead-code cleanup (session 59):** removed the two now-orphaned generic
    allocator helpers `xmalloc_array<T>` / `xcalloc_array<T>` from `src/ini.rs`
    (zero non-comment callers after session 58's `mem` arena move) and the
    now-unused `xcalloc` extern declaration (its only user was `xcalloc_array`;
    the `xmalloc` extern stays — still used for `native_text` scratch and the
    format-default name dup). Pure dead-code removal, no state migration; the
    parity comments documenting the preserved inclusive Vec sizes were kept.
    `globals.rs` raw extern count unchanged at 40 (Phase B complete). Dashboard
    29/2/7/4.
-   **First Phase-C threading slice — `free_node` (session 61):** threaded
    `free_node` from `pub unsafe fn free_node(p, s)` (thread-local bridge) to
    `pub fn free_node(mem: &mut MemState, p, s)`, touching the `mem` arena
    directly through a new `impl MemState` method surface (`b32s0`/`set_b32s0`/
    `set_b32s1`; the `rover` scalar is a field). Per `01b` §4 it makes zero
    `with_state`-bearing calls, so it is safe under any borrow. All 95 legacy
    (un-threaded) call sites across 9 files were bridged-down through a new
    transitional `free_node_global(p, s)` wrapper
    (`with_state_mut(|st| free_node(&mut st.mem, p, s))`), rather than the recipe's
    inline `with_state_mut(|s| …)` closure, because ~16 sites pass
    `with_state`-bearing accessors as arguments (`mem_b16s3(…)`,
    `active_node_size()`, `eqtb_b32s1(…)`, …) — these would nest-panic if
    evaluated inside the borrow — and two sites pass a local named `s`
    (`free_node(r, s)`) that an inline `|s|` closure param would shadow. The
    wrapper evaluates its args before borrowing, sidestepping both hazards, and is
    still removed incrementally (per-caller, as callers thread). No newtype debt
    (`p`/`s` are plain `int32_t`). Metrics essentially flat this foundation slice
    (`unsafe fn` net unchanged: `free_node` dropped `unsafe`, `free_node_global`
    added it; one transient `with_state_mut` wrapper added, to be removed as
    callers thread). Dashboard 29/2/7/4.
-   **`mem` allocator core completed — `flush_list`/`get_avail`/`get_node`
    (session 62):** threaded the remaining three allocator-core functions to
    `&mut MemState` in one commit (user-authorized batch), finishing the
    four-function core begun with `free_node` (session 61). Added
    `MemState::b32s1` read accessor. `get_avail`/`get_node` implement the `01b`
    §5 cold-path overflow sentinel: a module const `ALLOC_FAILED = i32::MIN` is
    returned on arena exhaustion (instead of calling `runaway()`/`overflow()`
    under the borrow), and the `*_global` bridge-down wrappers perform the abort
    after the borrow drops. `get_node`'s legitimate `MAX_HALFWORD` sentinel-size
    return is unchanged. All ~187 legacy call sites (`flush_list` 20, `get_avail`
    82, `get_node` 85) bridged-down through `flush_list_global`/`get_avail_global`/
    `get_node_global` (same wrapper pattern as session 61); `get_node`'s 16
    in-`mem.rs` callers are the `new_*` constructors, next to thread. Dropped
    `#[no_mangle]` from `get_avail`/`flush_list` (−2 exports) and `unsafe` from all
    three signatures (offset by the three new `unsafe` wrappers → `unsafe fn` net
    flat). Removed 8 now-unused `globals` imports from `mem.rs` (no new warnings).
    No newtype debt (`p`/`s`/sentinel all plain `i32`). Dashboard 29/2/7/4.
-   **Mem-only `new_*` node constructors threaded (session 63):** the 14
    `i32`-returning, mem-only constructors (`new_null_box`, `new_rule`,
    `new_ligature`, `new_lig_item`, `new_disc`, `new_math`, `new_spec`, `new_glue`,
    `new_kern`, `new_penalty`, `new_noad`, `new_style`, `new_choice`,
    `new_margin_kern`) threaded to `&mut MemState`, calling the threaded `get_node`
    directly (the `get_node_global` calls became pass-throughs). Added 5 `impl
    MemState` methods (`set_b16s0`/`set_b16s1`/`set_gr`/`word`/`set_word`). Each
    constructor propagates the `01b` §5 `ALLOC_FAILED` sentinel
    (`if p == ALLOC_FAILED { return p }`); its `new_X_global` bridge-down wrapper runs
    `overflow(c"main memory size", MEM_TOP+1)` after the borrow drops (same shape as
    `get_node_global`). All legacy call sites (~153) blanket-renamed to `new_X_global`
    across 5 files via one bare-identifier `perl` pass (imports + aliased imports
    included; no comment mentions to corrupt). Dropped `#[no_mangle] extern "C"` from
    the 12 that had it (−12 exports) and `unsafe` from all 14 signatures (offset by 14
    `_global` wrappers → `unsafe fn` net flat). Removed 3 now-unused `globals` imports
    from `mem.rs` (no new warnings). **No C callers** (header decls only, left in place
    per the session-61/62 allocator precedent). **Deferred:** `new_param_glue`/
    `new_skip_param` (read `eqtb` — hot-path `with_state` hazard, need a disjoint
    `&EqtbState` borrow) and `new_whatsit` (`()`-return can't carry the sentinel). No
    newtype debt (all params plain `i32`/`u16`/`i16`). Dashboard 29/2/7/4.
-   **Final deferred constructors threaded (session 64):** `new_param_glue` and
    `new_skip_param` now take a widened disjoint borrow (`&mut MemState` +
    `&EqtbState`) so their `eqtb` reads do not nest-panic under the mem borrow,
    and `new_whatsit` now returns `i32` to propagate `ALLOC_FAILED` to its
    wrapper. All three have `_global` bridge-down wrappers. Dropped 2
    `#[no_mangle]` exports (`new_param_glue`, `new_whatsit`), leaving **404**.
    Dashboard 29/2/7/4.
-   **First `save.rs` call-graph slice (session 65):** threaded the sparse-array
    allocation/traversal helper cluster (`find_sa_element`, private `new_index`,
    `sa_get_child`, `sa_set_child`, `sa_alloc_chain`) to direct `&mut MemState`
    and `&mut ScannerState` access. The helper now calls threaded `get_node`,
    propagates `ALLOC_FAILED`, and reads/writes `scanner.sa_root` directly. Added
    `find_sa_element_global` for legacy callers in `ini.rs`, `pagebuilder.rs`,
    and `scan.rs`; removed the now-dead `globals::sa_null` accessor. No C callers
    or redeclarations; no cleanup aliases. No newtype debt. Dashboard 29/2/7/4.
-   **Second `save.rs` call-graph slice (session 66):** threaded the clean SA
    refcount/save helpers (`delete_sa_ref`, `sa_save`) to direct state borrows.
    `delete_sa_ref` now uses `&mut MemState` / `&mut ScannerState` and calls
    threaded `free_node` plus a tiny direct glue-ref helper. `sa_save` now uses
    `&mut MemState`, `&mut ScannerState`, and `&mut SaveStackState`, calling
    threaded `get_node` and propagating `ALLOC_FAILED` / `SAVE_OVERFLOW` to
    `sa_save_global` so cold aborts happen after the borrow drops. Legacy callers
    in `save.rs`/`scan.rs` use `_global` wrappers. Removed the now-dead
    `globals::sa_level` accessor. No C callers or redeclarations; no cleanup
    aliases. No newtype debt. Dashboard 29/2/7/4.
-   **Third `save.rs` call-graph slice (session 67):** threaded the SA
    restore/define side (`sa_destroy`, `sa_restore`, `sa_def`, `sa_w_def`,
    `gsa_def`, `gsa_w_def`) to direct `MemState` / `ScannerState` /
    `SaveStackState` borrows. `sa_destroy` handles glue/token refs with threaded
    mem helpers and returns a pending box-list flush so the `_global` wrappers
    can drop the borrow before calling legacy `scan.rs::flush_node_list`.
    `sa_restore_global` similarly alternates direct-state phases with any needed
    flush. Legacy callers in `save.rs`/`scan.rs`/`ini.rs` use `_global` wrappers;
    dead `sa_save_global` / `sa_destroy_global` wrappers removed. No C callers
    or redeclarations; no cleanup aliases. No newtype debt. Dashboard 29/2/7/4.
-   **First `shipout.rs` call-graph slice (session 68):** threaded the clean
    DVI movement-node maintenance helpers (`new_edge`, `prune_movements`,
    `dvi_pop`). `new_edge` now takes `&mut MemState`, calls threaded
    `get_node`, and propagates `ALLOC_FAILED` to `new_edge_global`.
    `prune_movements` walks `ShipoutState.down_ptr` / `right_ptr` directly and
    frees movement nodes with threaded `free_node`. `dvi_pop` updates
    `ShipoutState` directly and returns whether `dvi_pop_global` must emit POP
    after the borrow drops. No cleanup aliases. C references are stale
    prototypes/comments only; no C/C++ callers. No newtype debt. Dashboard
    29/2/7/4.
-   **Second `shipout.rs` call-graph slice (session 69):** threaded
    `movement` to direct `MemState` / `ShipoutState` borrows. The direct helper
    now allocates and links the movement node with threaded `get_node`, patches
    `dvi_buf` directly, and returns the DVI bytes that `movement_global` emits
    after the borrow drops. Removed dead `down_ptr` / `right_ptr` bridge
    accessors; init writes those state fields directly. C references are stale
    prototype/comment only; no C/C++ callers. No newtype debt. Dashboard
    29/2/7/4.
-   **Third `shipout.rs` call-graph slice (session 71):** threaded the buffered
    DVI byte writer. `dvi_out`/`dvi_swap` now take `&mut ShipoutState`, do the
    buffer/bookkeeping work directly, and return an `Option<DviFlush>` /
    `DviFlush` deferred-action (`Write(a, b)` byte range or `Fatal` overflow)
    instead of doing file I/O or the cold `fatal_error` under the borrow.
    `dvi_out_global` runs the deferred flush / abort after the `with_state_mut`
    borrow drops (§5 sentinel). `write_to_dvi` became the borrow-dropping
    `write_to_dvi_global` wrapper (grabs `dvi_file`/buffer ptr under a short
    borrow, FFI-writes / `_tt_abort`s outside it — preserving the no-borrow-
    across-longjmp structure). ~76 legacy call sites renamed by a `perl` pass
    (bucket A); the trio is bucket C. Deleted the now-dead `dvi_gone()` getter.
    `dvi_four`/`dvi_two`/`dvi_font_def` left legacy for the next slices. C
    references are stale prototypes/comments only; no C/C++
    callers; no cleanup aliases. No newtype debt. Dashboard 29/2/7/4.
-   **Fourth `shipout.rs` call-graph slice (session 72):** threaded the pure
    DVI byte formatters `dvi_four` / `dvi_two`. They now take
    `&mut ShipoutState` and emit through the direct `dvi_out`, but stop at the
    first `DviFlush` and return a small continuation (`DviEmit { next, flush }`)
    so the bridge wrappers can drop the borrow, flush / fatal-abort, and resume
    before later bytes overwrite a flushed half-buffer. All legacy callers now
    use `dvi_four_global` / `dvi_two_global`. C references are stale
    prototypes/comments only; no C/C++ callers; no cleanup aliases. No newtype
    debt. Dashboard 29/2/7/4.
-   **Fifth `shipout.rs` call-graph slice (session 73):** threaded the DVI
    font-definition / font-selection path. `dvi_font_def` is now a direct
    `&mut ShipoutState` byte emitter over a prebuilt byte slice;
    `dvi_font_def_global` prepares native and non-native font-definition bytes
    outside the state borrow (including `make_font_def` and string-pool reads),
    then emits with continuation-aware `DviFlush` handling. `ensure_dvi_font`
    now emits font-selection bytes under `&mut ShipoutState`, while
    `ensure_dvi_font_global` handles font-used bookkeeping and updates
    `dvi_f` only after emission is fully complete. C references are stale
    prototype/comment only; no C/C++ callers; no cleanup aliases. No newtype
    debt. Dashboard 29/2/7/4.
-   **First `linebreak.rs` call-graph slice (session 74):** threaded the
    hlist-stack helper pair. `push_node` / `pop_node` now take
    `&mut LinebreakState` and touch `hlist_stack` / `hlist_stack_level`
    directly; `push_node_global` / `pop_node_global` preserve legacy callers and
    run cold `pdf_error` overflow/underflow paths after the state borrow drops.
    `find_protchar_left` / `find_protchar_right` call the wrappers, and the
    now-dead `hlist_stack_entry` / `set_hlist_stack_entry` bridge accessors were
    removed. No executable C/C++ callers; no cleanup aliases; no newtype debt.
    Dashboard 29/2/7/4.
-   **Second `linebreak.rs` call-graph slice (session 75):** threaded the
    protrusion scanner helpers `find_protchar_left` / `find_protchar_right` to
    direct `MemState` / `LinebreakState` borrows. The threaded path reads the
    mem arena directly, uses direct local predicates and `prev_rightmost_in`,
    and calls direct `push_node` / `pop_node`; `_global` wrappers preserve
    legacy callers and defer cold hlist-stack failure `pdf_error` calls until
    after the borrow drops. Removed dead `push_node_global` / `pop_node_global`,
    old bridged protrusion predicates, and the last hlist-stack bridge
    accessors. No executable C/C++ callers; no cleanup aliases; no newtype debt.
    Dashboard 29/2/7/4.
-   **Third `linebreak.rs` call-graph slice (session 76):** threaded
    `total_pw` to direct `MemState` / `LinebreakState` node selection. The
    direct helper reads `first_p` / `global_prev_p` directly, walks mem through
    `MemState`, and calls the direct protrusion scanners; `total_pw_global`
    preserves the legacy caller, defers cold hlist-stack failure `pdf_error`,
    and runs `char_pw` only after the borrow drops because `char_pw` still
    crosses linebreak/font/scanner bridges. Removed dead `first_p` /
    `global_prev_p` bridge getters. No executable C/C++ callers; no cleanup
    aliases; no newtype debt. Dashboard 29/2/7/4.
-   **Fourth `linebreak.rs` call-graph slice (session 77):** threaded the
    post-line-break margin-kern protrusion insertion blocks. New direct helpers
    select right/left protrusion targets under `MemState` / `LinebreakState`,
    `char_pw` still runs after the state borrow drops, and a short reborrow
    allocates/links margin kerns directly with `new_margin_kern`, deferring cold
    allocation `overflow` until after the borrow drops. Removed dead
    `find_protchar_left_global` / `find_protchar_right_global` wrappers and the
    dead `new_margin_kern_global` bridge. No executable C/C++ callers; no
    cleanup aliases; no newtype debt. Dashboard 29/2/7/4.
-   **Fifth `linebreak.rs` call-graph slice (session 78):** threaded the
    `post_line_break` TeXXeT LR-stack maintenance helper `update_lr_stack` to
    direct `&mut MemState`. The helper touches the `mem` arena through
    `MemState` methods and allocates via the threaded `get_avail`, returning the
    `01b` §5 sentinel on exhaustion; `update_lr_stack_global` runs the cold
    `runaway()`+`overflow()` abort after the borrow drops. The three legacy
    callers in `post_line_break` use the wrapper. No executable C/C++ callers; no
    cleanup aliases; no newtype debt. Dashboard 29/2/7/4.
-   **Node-disposal cluster threaded (session 79):** threaded the three
    fundamental node-recycling routines — `delete_token_ref` / `delete_glue_ref`
    (mem.rs) and the big recursive `flush_node_list` (scan.rs) — to
    `&mut MemState`. `flush_node_list`'s body was rewritten onto `MemState`
    methods (added `b16s3`/`ptr`/`set_ptr`), recurses on itself, calls the
    threaded `free_node`/`delete_glue_ref`/`delete_token_ref`, precomputes
    recursive-call args into locals, and returns the `01b` §5 sentinel
    (`Err(msg)`) for the two impossible corrupt-node `confusion` arms;
    `flush_node_list_global` fires `confusion` after the borrow drops. Bridged
    **116 legacy call sites** across 7 files (scan, math, linebreak, save,
    pagebuilder, shipout, ini) to `_global` wrappers. This bottoms out the
    disposal dependency for `post_line_break`/`sa_destroy`/math. No C/C++
    callers; no cleanup aliases; no newtype debt. `unsafe fn` 684→682, raw
    `unsafe {}` 160→163. Dashboard 29/2/7/4.
-   **Node-copy / mark-recycle / page-prune cluster threaded (session 80):**
    the counterpart to session 79's disposal cluster — `copy_native_glyph_info`
    (mem.rs), the recursive `copy_node_list`, the recursive `do_marks`, and
    `prune_page_top` (scan.rs) — threaded to `&mut MemState` (`prune_page_top`
    takes disjoint `&mut MemState` / `&mut ScannerState` / `&EqtbState` because
    `disc_ptr` is on `ScannerState` and `new_skip_param` needs `&EqtbState`).
    `copy_node_list`/`prune_page_top` allocate via the threaded
    `get_avail`/`get_node`/`new_skip_param` and return a shared
    `NodeRecycleAbort { Overflow, OverflowRunaway, Confusion(&CStr) }` §5 sentinel
    (fired by the `_global` wrappers after the borrow drops via
    `node_recycle_abort`); `get_avail` exhaustion preserves the `runaway()`+
    `overflow` flavor. `do_marks` has **no** cold path. Recursive/free-function
    args precomputed into locals to avoid overlapping `&mut mem` reborrows.
    Bridged **10 external** legacy sites (math, scan, ini, pagebuilder); ~15
    internal allocator/disposal `_global` calls became direct pass-throughs.
    `disc_ptr`/`set_disc_ptr` accessors keep other callers (none deleted). No
    C/C++ callers (only stale `.h` prototypes); no cleanup aliases; no newtype
    debt. `#[no_mangle]` 381→380, `unsafe fn` 682→680, raw `unsafe {}` 163→165.
    Dashboard 29/2/7/4.
-   **Math box-building helper cluster threaded (session 81):** the five
    math box-construction leaf helpers — `fraction_rule`, `overbar`, `char_box`,
    `stack_into_box`, `stack_glue_into_box` (all `math.rs`) — threaded to
    `&mut MemState`, bottoming out on the threaded allocator
    (`new_rule`/`new_kern`/`new_spec`/`new_glue`/`new_null_box`/`get_avail`).
    `fraction_rule`/`stack_into_box`/`stack_glue_into_box` are bucket B (pure mem
    / precomputed box pointer). `overbar` is bucket C — builds the kern/rule/kern
    chain directly and defers the `with_state`-bearing `vpackage` finish to
    `overbar_global`. `char_box` is bucket C — its native-character allocation
    (`new_native_character`) and TFM font-table reads (`effective_char` +
    `font_info`/`*_base`) are precomputed into a `CharBoxInput` enum by
    `char_box_global` *before* the borrow; the threaded core does only the mem
    work. Reused session-80's shared `NodeRecycleAbort` sentinel (made
    `node_recycle_abort` `pub(crate)`): `get_avail` exhaustion → `OverflowRunaway`,
    `new_*`/`get_node` → `Overflow`. **13 external** legacy sites (all in math.rs)
    bridged to `_global` wrappers. The native `char_box` branch now allocates the
    native node before the box (reverse of C order, forced by the `with_state`
    allocation) — only `mem` addresses differ, invisible to all output/`mem`-image
    paths (TRIP is TFM-only); confirmed by the green native-font tex-outputs cases.
    Excluded `height_plus_depth` (pure font read, no mem) and `stack_glyph_into_box`
    (`measure_native_glyph` is `with_state`-bearing mid-function). No C/C++ callers
    (stale `xetex-math.c` prototypes only); no cleanup aliases; no newtype debt.
    `#[no_mangle]` 380→375, `unsafe fn` 680 (net 0), raw `unsafe {}` 165 (unchanged).
    Dashboard 29/2/7/4.
-   **Math native-glyph stack split threaded (session 83):** split
    `stack_glyph_into_box` into a threaded mem-only glyph allocator/linker plus
    a `_global` wrapper that drops the borrow before `measure_native_glyph`
    (font bridge reads), then reborrows to splice the measured node. The adjacent
    native-glyph vbox fallback in `var_delimiter` now uses the same start/finish
    pattern around measurement. Classification: bucket B for mem accessor rewrites
    and direct threaded `get_node`/`new_null_box`; bucket C for the
    borrow-dropping measurement boundary. Bridged 2 legacy call sites in
    `build_opentype_assembly` plus the native vbox fallback. No cleanup aliases;
    no live C/C++ callers (stale `xetex-math.c` prototypes/comments only); no
    moved fields, so no newtype debt. `#[no_mangle]` 370→369, `unsafe fn` 679→680
    (one extra transitional wrapper), raw `unsafe {}` 168 (unchanged). Dashboard
    29/2/7/4.
-   **Math parameter helper cluster threaded (session 84):** threaded the shared
    math font/parameter helpers (`math_font`, `font_is_ot_math`,
    `math_sy_param`, `math_ext_param`, `sym_font_ok`, `ext_font_ok`) plus the 23
    public math parameter helpers (`math_x_height` through `big_op_spacing5`) to
    narrow `&FontState`/`&EqtbState` reads. Classification: bucket B accessor
    rewrite; hot callees are direct state reads and native-font C helpers, with
    no hot `with_state` callee. Bridged 79 legacy math call sites to `_global`
    wrappers; `cur_size()` is pre-read before the mathex wrapper borrow. No
    cleanup aliases; no live C/C++ caller beyond stale `xetex-math.c`; no moved
    fields, so no newtype debt. `#[no_mangle]` 369→346, `unsafe fn` 680→651,
    raw `unsafe {}` 168→171. Dashboard 29/2/7/4.
-   **Math glue/rebox node helper cluster threaded (session 85):** threaded
    `math_glue`, `math_kern`, `rebox`, and `make_under`'s node assembly to
    narrow `&mut MemState` cores. Classification: bucket B for the glue/kern
    mem rewrites after precomputing scaled-math values outside the borrow, bucket
    C for `rebox`/`make_under` borrow-dropping wrappers around `hpack`,
    `effective_char_global`, `clean_box`, and `vpackage`. Bridged 10 legacy math
    call sites; no cleanup aliases; no live C/C++ caller beyond stale
    `xetex-math.c`; no moved fields, so no newtype debt. `#[no_mangle]`
    346→342, `unsafe fn` 651 (unchanged), raw `unsafe {}` 171→173. Dashboard
    29/2/7/4.
-   **Math noad-finishing assembly cluster threaded (session 86):** threaded
    `make_over`, `make_radical`, `make_vcenter`, and `make_fraction` to narrow
    `&mut MemState` cores. Classification: bucket B for `make_vcenter` plus a
    cold `Confusion("vcenter")` sentinel; bucket C for the over/radical/fraction
    wrappers, with `clean_box`/`vpackage`/`hpack`/`var_delimiter` and font reads
    outside live borrows. Bridged 4 `mlist_to_hlist` noad-switch call sites and
    deleted dead `fraction_rule_global` / `overbar_global` wrappers. No cleanup
    aliases; no live C/C++ caller beyond stale `xetex-math.c`; no moved fields,
    so no newtype debt. `#[no_mangle]` 342→338, `unsafe fn` 651→650, raw
    `unsafe {}` 173→174. Dashboard 29/2/7/4.
-   **Native text-node constructor cluster threaded (session 87):** the two
    functions that build XeTeX native-text nodes — `new_native_word_node` and
    `new_native_character` (`scan.rs`) — threaded onto narrow state borrows.
    `new_native_word_node(mem: &mut MemState, eqtb: &EqtbState, f, n)` is bucket
    **B** (pure mem over the threaded `get_node` / `cnl_get_node!`
    `NodeRecycleAbort::Overflow` §5 sentinel; `new_native_word_node_global`
    aborts after the borrow drops). `new_native_character` is bucket **C**: its
    mem allocation + UTF-16 payload fill (mapped-run and single-scalar branches)
    moved into a threaded `build_native_character(&mut MemState, &EqtbState,
    &FontState, …)` core run under one `with_state_mut` borrow, while the
    orchestrator keeps the with_state-bearing string-pool / TECkit
    `apply_mapping` / lost-char `char_warning` / `measure_native_node` work
    outside the borrow (so `new_native_character` keeps its name — it is the
    permanent orchestrator, not a transient `_global` wrapper). Added two
    `MemState` methods (`set_b16s3`, `as_u16_ptr_mut`). Bridged 11 external
    `new_native_word_node` sites to `_global` (scan 6, linebreak 4, shipout 1).
    No C/C++ callers (header prototypes only); no cleanup aliases; no newtype
    debt (no fields moved). `#[no_mangle]` 338→337, raw externs 40 unchanged,
    `unsafe fn` net flat (~649), raw `unsafe {}` 174→176. Dashboard 29/2/7/4.
-   **Pagebuilder leaf-helper block threaded (session 88):** threaded
    `freeze_page_specs` and `ensure_vbox` (`src/pagebuilder.rs`). `freeze_page_specs`
    is bucket **B** — pure disjoint `&EqtbState` read + `&mut ShipoutState` page-spec
    writes, no `with_state`-bearing call. `ensure_vbox` is bucket **C** — split into a
    threaded predicate `ensure_vbox_is_hbox(&EqtbState, &MemState, n) -> bool` plus the
    `ensure_vbox_global` wrapper that runs the cold `error_here_with_diagnostic` /
    `box_error` only after the borrow drops. Added the first `impl ShipoutState`
    methods (`set_page_contents`/`set_page_so_far`/`set_page_max_depth`/
    `set_least_page_cost`); reused `EqtbState::b32s1` / `MemState::b16s1`. Bridged 4
    `build_page`/`fire_up` sites to `_global`; no bridge accessor lost its last caller
    (the `set_page_*`/`page_*` accessors are still used across build_page/fire_up/ini/
    scan/output), so none deleted. The chosen-over alternative `make_op`/`make_ord`
    remain large bucket-C/D (heavy `fetch`/`clean_box`/OT-math interleaving) — deferred.
    No C/C++ callers (translated `xetex-pagebuilder.c` stub, comment-only mentions); no
    cleanup aliases; no fields moved, so no newtype debt. `#[no_mangle]` 337 unchanged,
    `unsafe fn` 649→648, raw externs 40 / raw `unsafe {}` 176 unchanged. Dashboard
    29/2/7/4.
-   **Math `fetch` keystone + `make_ord` + leaf helpers threaded (session 89):**
    a deliberately larger 4-function slice in the math noad family (`src/math.rs`).
    **`fetch`** (bucket C) — the math-char setter the whole noad family funnels
    through — split into a threaded `fetch_core(&mut MemState, &FontState,
    &EqtbState, &mut ScannerState, a) -> FetchOutcome` (sets `cur_c`/`cur_f`/`cur_i`
    directly) plus `fetch_global`, which emits the two cold deferred paths after the
    borrow drops (undefined-math-font diagnostic + recoverable `error()`; missing-char
    `char_warning`). **`make_ord`** (bucket C/B) — orchestrator `make_ord_global`
    keeps the early mem checks + `fetch_global` between borrows, each `\restart`
    iteration runs the threaded `make_ord_body(&mut MemState, &FontState,
    &mut ScannerState, q, p)` under one disjoint borrow, with a
    `MakeOrdStep { Return, Restart, Overflow }` enum carrying control back out so the
    cold allocator overflow aborts via `node_recycle_abort` after the borrow.
    **`attach_hkern_to_new_hlist`** (bucket B, mem + threaded `new_kern`, `_global`
    wrapper) and **`set_empty_math_field`** (bucket B, trivial mem) round it out.
    Key recon finding: `cur_f`/`cur_c`/`cur_i` and the `font_info`/`*_base` reads are
    all with_state-bearing, so threading to direct `&mut ScannerState`/`&FontState`
    removes the bridge, leaving `fetch` the only with_state-bearing call (kept between
    borrows). Added `FontState::font_bc_entry`/`font_ec_entry`. Bridged the call sites
    (`fetch` 6 ext + 1 internal, `make_ord` 1, `attach_hkern` 4, `set_empty_math_field`
    7) and deleted the now-dead `globals::null_character` getter. No C/C++ callers
    (translated `xetex-math.c` stub, header prototypes only); no cleanup aliases; no
    fields moved → no newtype debt. `#[no_mangle]` 337→334, `unsafe fn` 648→647, raw
    externs 40 / raw `unsafe {}` 176 unchanged. Dashboard 29/2/7/4.
-   **Math `make_op` threaded (session 90):** the last big noad-finishing helper
    (`src/math.rs`), a hard bucket-**C/D** function, threaded to `make_op_global`
    (drops `#[no_mangle] extern "C"`, returns `delta: i32`) as a series of short
    disjoint `with_state_mut` borrows with the with_state-bearing calls between
    them: NORMAL→LIMITS latch `(mem+scanner)`; the non-OT italic/list-tag delta
    block in one `(mem+fonts+scanner)` borrow (`cur_f`/`cur_i`/`cur_style` scanner +
    `font_area`/`*_base`/`font_info` font reads + the pure-C `usingOpenType`, all
    under the borrow); `clean_box` between borrows; the MATH_CHAR box finish via the
    threaded `axis_height(&fonts,&eqtb,size)` in one borrow; and the whole LIMITS
    branch — 3 `clean_box` → a `(mem)` borrow allocating the wrapper vbox + width →
    3 `rebox_global` → one big `(mem+fonts+eqtb+scanner)` tail borrow with the
    threaded `big_op_spacing1..5` precomputed into locals before the mem mutations,
    threaded `new_kern`/`new_null_box`/`free_node`, the `cur_f=save_f` write, and
    `NodeRecycleAbort::Overflow` deferred to a post-borrow `node_recycle_abort`
    (`01b` §5). The `measure_native_glyph`-interleaved OT-math glyph-variant/assembly
    region is left on bridge accessors (bucket **D** — no live borrow held).
    Classification recon: header prototype/comment only in `xetex-math.c` (no body,
    no caller); no cleanup alias; no cross-module redeclaration; the lone
    `mlist_to_hlist` call site is not inside a `with_state*` closure. Added
    `FontState::italic_base_entry` (the handoff wrongly assumed it existed — only the
    `italic_base` field did). Bridged the 1 call site to `make_op_global`; **deleted**
    the five now-dead `big_op_spacing{1,2,3,4,5}_global` wrappers (make_op was their
    last caller; the cores are now called directly under the LIMITS tail) and the dead
    `globals::set_cur_c` setter (math.rs was its last caller). No C/C++ callers; no
    cleanup aliases; no fields moved → no newtype debt (`italic_base` is plain
    `int32_t *`, already no-debt per session 39). `#[no_mangle]` 334→333, raw externs
    40, `unsafe fn` 647 (net flat), raw `unsafe {}` 176 unchanged. Dashboard 29/2/7/4.
-   **Math `make_scripts_global` OT script-kern helper extracted (session 95):**
    bounded cleanup inside the existing `make_scripts_global` orchestrator (`src/math.rs`).
    Added `ScriptKernAttachment` and `maybe_attach_script_kern_global` to consolidate
    the repeated OT-math script-kern calculation/attachment blocks. The helper is
    bucket **C**: one short state read for `scanner.cur_f` plus `glyph_nucleus(&MemState,
    p)`, then the borrow drops before `font_is_ot_math_global`, OT-math C
    `get_ot_math_kern`, and the with-state-bearing `attach_hkern_to_new_hlist_global`.
    No external call-site rename, no bridge accessor deletion, no method surface, and no
    state fields moved → no newtype debt. Recon found only stale `xetex-math.c`
    prototype/comment hits; no cleanup aliases or redeclarations; no call site inside a
    live `with_state*` closure. Metrics unchanged by the slice; current counts after
    validation are `#[no_mangle]` 328, raw extern statics 38, raw `unsafe {}` 174
    (`unsafe fn` grep 646). Dashboard 29/2/7/4; `cargo check` reports 51 warnings.
-   **Sessions 96–98** continued bounded structural cleanup inside
    `make_scripts_global` (script-box prep helper, sub/sup stack tail, branch-body
    helpers); see `SESSION_LOG.md`. Metrics flat across all three.
-   **`flush_math` threaded (session 99):** the small math-mode list-flusher
    threaded off the bridge to `pub fn flush_math(mem: &mut MemState) -> Result<(),
    &CStr>` (was `pub unsafe fn flush_math()`). It recycles the two current-list
    chains via the already-threaded `crate::scan::flush_node_list` under the same
    borrow and resets `cur_list` (raw deferred global, safe under a live borrow per
    `01b` §4); the two impossible corrupt-node `confusion` arms propagate as `Err`,
    fired by the new `flush_math_global` bridge-down wrapper after the borrow drops
    (`01b` §5). 3 legacy call sites (`math.rs` ×2, `scan.rs` ×1, none inside a live
    `with_state*` closure) bridged to `flush_math_global`; `scan.rs` import updated.
    No bridge accessor lost its last caller; no new method surface; no fields moved
    → no newtype debt. No C/C++ callers (stale `xetex-math.c` prototype/comment
    only); no cleanup aliases. Metrics flat: `#[no_mangle]` 328, raw extern statics
    38, raw `unsafe {}` 174, `unsafe fn` net 0. Dashboard 29/2/7/4.
-   **`fin_mlist` threaded (session 99, slice 2):** the math-mode mlist finalizer
    threaded off the bridge to `fn fin_mlist(mem: &mut MemState, p) -> Result<i32,
    &CStr>` (was `pub unsafe fn fin_mlist(p) -> i32`). The mem-link work + raw
    `cur_list` head/tail/aux reads run under one borrow; the impossible corrupt-node
    `confusion` (`q`-not-`LEFT_NOAD`) propagates as `Err`, and the trailing
    `pop_nest` (with_state-bearing through the `nest` array) is deferred — both fired
    by the new `fin_mlist_global` wrapper after the borrow drops. `pop_nest` only
    recycles `cur_list.head`, which the core never modifies, so the original ordering
    is preserved. 5 legacy call sites (`math.rs` ×4, `scan.rs` ×1, none inside a live
    `with_state*` closure) bridged; `scan.rs` import updated. No bridge accessor lost
    its last caller; no new method surface; no fields moved → no newtype debt. No
    C/C++ callers (stale `xetex-math.c` prototype/comment only); no cleanup aliases.
    Metrics flat: `#[no_mangle]` 328, raw extern statics 38, raw `unsafe {}` 174,
    `unsafe fn` net 0. Dashboard 29/2/7/4.
-   **`var_delimiter` delimiter-box construction threaded (session 100):** the
    documented session-99 next target — the EXT_TAG extensible-delimiter
    box-building block inside `var_delimiter_global` (`src/math.rs`) — extracted
    to `build_ext_delimiter_box_global(f, q, v)` (bucket C, session-81/83 style).
    The box alloc + every TFM font read (the `exten` row `r`, the repeatable
    piece width via `effective_char`, the `height_plus_depth` of top/mid/bottom/
    repeat pieces) now run under one `mem`+`fonts` borrow island via the threaded
    `new_null_box`/`effective_char`/`height_plus_depth`, replacing ~12 scattered
    single-statement bridge closures; the piece stacking keeps the threaded
    `stack_into_box_global`, so allocation order (vlist box first, then each
    stacked char box) is byte-identical to the inline version (the parity
    guarantee for the EXT_TAG TFM path TRIP/`display_math` exercise).
    `ALLOC_FAILED` → `NodeRecycleAbort::Overflow` deferred to `node_recycle_abort`
    after the borrow (`01b` §5). Also consolidated the null-delimiter branch
    (`\nulldelimiterspace`) from 3 closures into one disjoint `mem`+`eqtb` island.
    No external call-site rename; no bridge accessor lost its last caller; no new
    method surface; no fields moved → no newtype debt. No live C/C++ callers
    (stale `xetex-math.c` prototype/comment only); no cleanup aliases. Metrics:
    `#[no_mangle]` 328, raw `unsafe {}` 174 unchanged; `unsafe fn` 655→656 (+1,
    the new helper). Dashboard 29/2/7/4.

### Idiom metrics (baseline = session 132, end of Stage 1)

These trend toward their targets as workstreams execute. Re-measure with
the snippets in `00-overview.md` / the workstream plans.

| Metric | Baseline (2026-05-23) | Target | Current |
|---|---:|---|---:|
| `extern static mut` globals (`globals.rs`) | 384 | 0 (non-const) | 38 (current direct grep) |
| promoted Rust `pub static mut` globals | 16 | 0 (non-const) | 3 (session 53 moved 13 DVI positioning/output statics) |
| thin global accessors (`globals.rs`) | 811 | 0 | 799 function defs / 706 `with_state*` call sites in `globals.rs` (current direct grep) |
| `#[no_mangle]` exports | 576 | boundary-only | 327 (current direct grep; session 110 de-exported `get_native_usv`) |
| `unsafe fn` (crate-wide) | 694 | boundary-only | 640 (current direct grep over `unsafe fn` / `unsafe extern "C" fn`; session 111 threaded 10 synctex readers + math `is_char_node`, −11) |
| raw `unsafe { ... }` blocks | 846 text-search / 688 clippy-documented bucket | boundary-only | 175 / 688 not remeasured (current direct grep) |
| clippy errors | 694 | 0 | 110 (`cargo clippy -p tectonic_engine_xetex --message-format=short`) |
| clippy warnings | 358 | 0 | 61 (`cargo clippy -p tectonic_engine_xetex --message-format=short`) |
| largest module LOC | 18,748 (`scan.rs`) | none >~3k | 18,537 (`scan.rs`; ws02 slice 1 drained the `errors` group) |

### Test dashboard (must never regress)

| Suite | Count |
|---|---|
| `cargo test -p tectonic --test tex-outputs` | 29/29 |
| `cargo test -p tectonic --test trip` | 2/2 |
| `cargo test -p tectonic --test engine_rust` | 7/7 |
| `cargo test -p tectonic --test latex_goldens` | 4/4 |

## Decisions log (Stage 2)

-   **2026-05-23** — End-state target = **full Phase C** (state threaded
    as `&mut EngineState`, near-zero `unsafe`, no internal
    `#[no_mangle]`). Chosen by the user over the thread-local middle
    ground and the pragmatic option.
-   **2026-05-23** — First workstream = **state & memory redesign (01)**,
    as the root cause that dissolves the 688-block clippy bucket and
    sets module boundaries.
-   **2026-05-23** — **Do not** bulk-add SAFETY comments to clear
    `undocumented_unsafe_blocks` before 01/03; let those lints stay red
    in the interim.
-   **2026-05-23** — Type taxonomy correction: `MemIdx` means a valid
    arena index only. Sentinel-carrying TeX halfwords use a distinct
    `MemPtr`/`Halfword` wrapper; `FontId` stays `i32`-backed initially;
    `NodeType` construction is checked, never unchecked `transmute`.
-   **2026-05-23** — Ownership-transfer rule: every state-cluster move
    must update C symbol definitions and `tt_cleanup`/`#[link_name]`
    aliases in the same commit to avoid stale symbols and double frees.
-   **2026-05-23** — `xetex-macos.c` is out of scope for Stage 2. Do not
    translate it or let macOS-only references block Linux-validated
    idiomatization work.
-   **2026-05-23** — `01a-global-inventory.md` created. Workstream 01 is
    READY for execution, starting with least-coupled non-`misc` clusters.
    `misc` rows are a holding area and require final sub-struct decisions
    before migration.
-   **2026-05-23** — Phase-B migrations keep the raw `i32`/`u8`
    representation by design (newtypes are the §4(b) sweep), **but** a
    field whose C type was a semantic typedef/enum (`selector_t`,
    `tt_history_t`, `scaled_t`, …) must get a row in the new
    **Deferred newtype worklist** in `01a-global-inventory.md` in the
    same commit that migrates it. This closes the gap where moving a
    `selector_t` into `EngineState` as `i32` would silently lose the type
    forever. Worklist seeded retroactively for `selector`/`history`/etc.
    Goal is *more* type safety, recovered on a tracked schedule — not
    less.
-   **2026-05-23** — Arena ownership pattern established on `save_stack`
    (session 11): array storage becomes a `Vec<T>` on `EngineState`,
    allocated at init and dropped by `tt_cleanup`; accessors index the
    `Vec`. longjmp/Drop (§7) is handled by keeping the owner in the
    `'static` thread-local, so abort never jumps over live storage and
    `Drop`-on-abort is intentionally skipped (matches prior behavior).
    Reuse this exact shape for the remaining arena arrays.
-   **2026-05-30** — Added **workstream 06 (constants & magic numbers)**,
    `06-constants.md`. A measured gap not covered by 01–05 (and invisible
    to clippy): **193 distinct `const`s are duplicated across 2+ modules**
    (`INT_BASE` ×16, `MAX_HALFWORD` ×12, …), and region bases are stored
    as opaque literals where the WEB computes them. Two phases: **A**
    centralize the duplicated cross-module consts into a shared computed
    `src/consts.rs` (low-risk, value-preserving, do independently now);
    **B** sweep call-site literals module-by-module **as Phase C threads
    each module**, so literals that typed accessors will absorb are not
    named prematurely. Hard rule: no value may change — static assertions
    pin every computed base; TRIP + format goldens are the guard.
-   **2026-05-30 (session 60)** — **Phase C is unblocked**:
    `01b-phase-c-recipe.md` drafted to READY. Decisions baked in: (1) the
    `EngineState` owner **stays in the thread-local anchor** for all of Phase C;
    threading passes `&mut` *below* a single boundary borrow, never stack-owning
    state across the setjmp boundary (resolves §7). (2) Threading uses a
    **bridge-down** technique (call-site `with_state_mut(|s| f(&mut s.mem, …))`),
    the mirror of Phase B's bridge-up, removed per-call-site as callers thread.
    (3) Cold abort paths (`overflow`/`runaway`) are **not** threaded; threaded
    allocators return an **overflow sentinel** and let the wrapper abort after the
    borrow drops. (4) Recon corrected the session-59 blocker — `line()` /
    `cur_input_synctex_tag()` read raw `mod raw` globals, not the thread-local, so
    they are safe under a live borrow. (5) **First subsystem = the `mem` allocator
    core** (`free_node`/`flush_list`/`get_avail`/`get_node`), one fn + all sites
    per commit. No code changed this session.
-   **2026-05-30 (session 62, follow-up)** — **Target error model = the
    `engine_bibtex` shape; longjmp removal is the *output* of the Rust
    conversion, not a separate ABI change.** Prompted by a question about the
    `ALLOC_FAILED` sentinel and "why keep any C." Findings: the
    `setjmp`/`longjmp` escape is **not** xetex-owned — `_tt_abort()` +
    `ttbc_global_engine_enter()` live in the shared `tectonic_bridge_core`
    (`crates/bridge_core/support/support.c`); every C engine plugs into them,
    and bridge_core's own docs call setjmp/longjmp-across-FFI Undefined
    Behavior, which is *why* the `setjmp` sits in a C frame with no Rust `Drop`.
    **`engine_bibtex` (already fully Rust) is the existence proof of the clean
    end-state** and is hereby the reference implementation for workstream 05:
    (a) it has **no** `setjmp`/`longjmp`/`_tt_abort` anywhere — verified by
    grep; (b) "fatal error" is a `Result` — `enum BibtexError { Fatal, Recover,
    NoBst }` propagated with `?` to the top, with a code comment stating the
    classic TeX fatal outcome "is represented as an `Err` result"; (c) the
    `History` enum (Spotless/Warning/Error/Fatal) is returned as a *value*;
    (d) state is **stack-owned and `&mut`-threaded** — `bibtex_main` builds the
    state structs as locals and threads a `GlobalItems { &mut … }` bundle down
    the call graph (no thread-local, no `with_state` bridge) — i.e. the literal
    Phase-C end-state; (e) it enters through bridge_core's **Rust** API
    (`CoreBridgeLauncher::with_global_lock`, returning `Result<BibtexOutcome>`),
    not a C `tt_*_main`/`jmp_buf` shim. **Correction to an earlier verbal
    claim:** removing xetex's longjmp does **not** require a bridge_core ABI
    change — the Rust `with_global_lock` entry already exists and is proven;
    xetex would switch to it once the conversion is done. The actual blockers
    are scope-bound, not mechanism-bound: (1) xetex still has C/C++ that bibtex
    never did — the shaping stack (harfbuzz/graphite/freetype/ICU) and
    `xetex-XeTeXOTMath.cpp`, plus the bridge_core C alloc/abort callbacks
    (`xmalloc`/`ttstub_*` that `_tt_abort`); the Result route tolerates external
    shaping libs (they return values) but the alloc/abort callbacks must become
    their Rust equivalents (largely happening as arenas became `Vec`s); (2)
    `overflow`/`confusion`/`error` fire from hundreds of nested sites, so the
    `Result`/`?` conversion + stack-threading is the *output* of Phase C +
    workstream 03, not a Stage-2 slice. Until then the thin C `setjmp` anchor is
    the correct quarantine (§7). The `ALLOC_FAILED` sentinel is consistent with
    this: a transient borrow artifact (`01b` §5), to be deleted when the
    allocators' callers are threaded and the fatal path becomes a `Result`.
    Recorded as the workstream-05 target in `05-errors-and-misc.md` (seed).

## Next action for a fresh session

0.  Follow the **Execution recipe** in `01-state-and-memory.md` §11
    (grep checks → add field → repoint accessors → remove defs/aliases →
    worklist rows → dashboard → docs → commit) and heed its recurring
    gotchas.
1.  **Phase C is executing. The `mem` allocator core and all 17 `new_*` node constructors are fully threaded (sessions 61–64). Sessions 65–67 completed the current `save.rs` sparse-array helper run; sessions 68–69 started `shipout.rs` with DVI movement-node helpers plus `movement`; session 71 threaded the buffered DVI byte writer (`dvi_out`/`dvi_swap` → `&mut ShipoutState` returning a `DviFlush` deferred-action; `write_to_dvi` → borrow-dropping `write_to_dvi_global`); session 72 threaded the pure DVI byte formatters (`dvi_four`/`dvi_two`) with continuation-aware flush propagation; session 73 threaded the DVI font-definition / font-selection path (`dvi_font_def` / `dvi_native_font_def` / `ensure_dvi_font`); sessions 74–78 continued `linebreak.rs` with the hlist-stack helper pair, protrusion scanners, `total_pw`, post-line-break margin-kern protrusion insertion, and the `post_line_break` LR-stack maintenance helper `update_lr_stack` (→ direct `LinebreakState`/`MemState`, allocating via threaded `get_avail` with cold overflow deferred to a `_global` wrapper; `char_pw` kept outside live state borrows). Session 70 added the `01b` §4a classified-codemod lane: classify each target/group first, use regex/scripted rewrites only for the A/B mechanical buckets, and keep borrow-dropping/error/output/scanner cases manual. **Session 79 threaded the node-disposal cluster** (`delete_token_ref`/`delete_glue_ref` + the big recursive `flush_node_list`, 116 call sites). **Session 80 threaded the node-copy / mark-recycle / page-prune cluster** (`copy_native_glyph_info`, `copy_node_list`, `do_marks`, `prune_page_top` → `&mut MemState` ± `&mut ScannerState`/`&EqtbState`; shared `NodeRecycleAbort` §5 sentinel). **Session 81 threaded the math box-building helper cluster** (`fraction_rule`, `overbar`, `char_box`, `stack_into_box`, `stack_glue_into_box` → `&mut MemState`; bucket-C font/native reads precomputed outside the borrow, `vpackage` deferred; 13 external sites). **Session 82 threaded the font/effective-char + linebreak reconstruction cluster** (`effective_char`, `effective_char_info`, `height_plus_depth`, `finite_shrink`, `reconstitute` → `&mut FontState` / `&mut MemState` / `&mut HyphState`; 29 bridged legacy call sites; two dead bridge getters deleted). **Session 83 threaded the native-glyph stack split** (`stack_glyph_into_box` + native vbox start/finish around `measure_native_glyph`). **Session 84 threaded the math parameter helper cluster** (`math_font`/`font_is_ot_math`/`math_sy_param`/`math_ext_param`, `sym_font_ok`/`ext_font_ok`, and `math_x_height` through `big_op_spacing5` → `&FontState`/`&EqtbState`; 79 bridged legacy call sites). **Session 85 threaded the math glue/rebox node helper cluster** (`math_glue`, `math_kern`, `rebox`, `make_under` node assembly → `&mut MemState`; 10 bridged legacy call sites; scaled-math values precomputed outside borrows). Next: a pagebuilder helper block, another math node-building block, or another closed linebreak/post-line-break block. Keep `scan.rs`'s scanner hot path last. Draft `02-modules.md` to READY before threading reaches scanner-heavy code.**

    **Session 86 update:** the math noad-finishing assembly cluster
    (`make_over`, `make_radical`, `make_vcenter`, `make_fraction`) is now
    threaded to `&mut MemState`; 4 `mlist_to_hlist` sites were bridged and the
    dead `fraction_rule_global` / `overbar_global` wrappers were removed.

    **Session 87 update:** the native text-node constructor cluster
    (`new_native_word_node` → `&mut MemState` + disjoint `&EqtbState`;
    `new_native_character`'s mem alloc/fill → threaded `build_native_character`
    core, orchestrator keeps string/mapping/warning/measurement outside the
    borrow) is now threaded; 11 external `new_native_word_node` sites bridged to
    `_global`.

    **Session 88 update:** the pagebuilder leaf-helper block (`freeze_page_specs`
    → disjoint `&EqtbState` + `&mut ShipoutState`, bucket B; `ensure_vbox` →
    threaded `ensure_vbox_is_hbox` predicate + cold `box_error` deferred to
    `ensure_vbox_global`, bucket C) is now threaded; added the first `impl
    ShipoutState` page-spec setters; 4 `build_page`/`fire_up` sites bridged to
    `_global`.

    **Session 89 update:** the math `fetch` keystone + `make_ord` +
    `attach_hkern_to_new_hlist` + `set_empty_math_field` are now threaded (a
    deliberately larger 4-function slice). `fetch` → `fetch_core` (direct
    `mem`/`fonts`/`eqtb`/`scanner`) + `fetch_global` (cold undefined-font /
    missing-char emit deferred); `make_ord` → `make_ord_global` orchestrator +
    threaded `make_ord_body` with a `MakeOrdStep` control enum so `fetch_global`
    stays between borrows and cold overflow aborts after the borrow. Added
    `FontState::font_bc_entry`/`font_ec_entry`; deleted dead `globals::null_character`
    getter.

    **Session 90 update:** **`make_op`** is now threaded to `make_op_global` (the last
    big noad-finishing helper). Built as a series of short disjoint borrows
    (NORMAL→LIMITS latch; non-OT italic/list-tag delta block; MATH_CHAR box finish via
    threaded `axis_height`; LIMITS tail via threaded `big_op_spacing1..5`/`new_kern`/
    `new_null_box`/`free_node` with overflow deferred to `node_recycle_abort`), keeping
    `fetch`/`clean_box`/`rebox`/measurement between borrows and leaving the
    `measure_native_glyph`-interleaved OT-math glyph-variant/assembly region on the
    bridge (bucket D). Added `FontState::italic_base_entry`; deleted the five dead
    `big_op_spacing*_global` wrappers + the dead `globals::set_cur_c` setter. Next
    targets: `make_scripts` (large), `make_math_accent`, the interleaved
    `first_script_glyph`, `var_delimiter`/`make_left_right`, or a pagebuilder/
    linebreak block. Remaining high-count functions are orchestrators
    (`hpack`/`vpackage`/`build_page`/`line_break`/`mlist_to_hlist`) — do not take one
    whole in a single session. Keep scanner-heavy `scan.rs` last.

    **Session 91 update:** `make_math_accent` + `compute_ot_math_accent_pos` are now
    threaded to borrow-dropping `_global` orchestration. The recursive OT accent-position
    helper is a short `MemState` dispatcher loop, with `fetch_global`,
    `new_native_character`, `real_get_native_glyph`, and `get_ot_math_accent_pos` kept
    outside live borrows. `make_math_accent_global` precomputes initial scanner/font/mem
    state in short borrows, keeps `clean_box`/native glyph/OT-math/measurement calls
    between borrows, and threads the TFM skew-kern scan through direct
    `FontState`/`ScannerState` access. Added `FontState::width_base_entry` and
    `FontState::skew_char_entry`; deleted dead `globals::set_cur_i`.

    **Sessions 92–98 update:** `make_scripts_global` (the last large noad helper) and
    the math delimiter helpers were threaded/decomposed across seven bounded slices —
    borrow-island splits, then extracted helpers for OT script-kern attachment,
    script-box prep, the sub/sup stack tail, and the dispatcher branch bodies. It is now
    a small dispatcher. See `SESSION_LOG.md` for each.

    **Session 99 update:** `flush_math` + `fin_mlist` threaded to `&mut MemState`
    (3 + 5 sites).

    **Session 100 update:** the EXT_TAG extensible-delimiter box-building block
    inside `var_delimiter_global` is now threaded into one `mem`+`fonts` borrow
    island (`build_ext_delimiter_box_global`), plus the null-delimiter branch
    consolidated into one `mem`+`eqtb` island. `var_delimiter_global`'s remaining
    bridge churn is the two small/large-family scan loops (bucket D —
    `math_font`/`map_char_to_glyph`/`get_ot_math_variant`/`get_ot_assembly_ptr`/
    TFM char-list scan deeply interleaved; leave on the bridge) and already-minimal
    single-island reads. Next targets: another small math/list leaf —
    `build_choices`/`append_choices` mem work (keep `unsave`/`push_math`/
    `scan_left_brace` scanner calls outside live borrows), or a pagebuilder/
    linebreak leaf. Remaining high-count functions are orchestrators
    (`hpack`/`vpackage`/`build_page`/`line_break`/`mlist_to_hlist`) — do not take one
    whole in a single session. Keep scanner-heavy `scan.rs` last.

    **Session 101 update:** the bounded math-command helpers
    `math_limit_switch`, `math_radical`, `append_choices`, `build_choices`, and
    `sub_sup` now run their allocator/mem/save-stack work under short direct
    borrows and expose legacy `_global` wrappers to `scan.rs`. Scanner/error
    calls (`push_math`, `scan_left_brace`, `scan_delimiter`, `scan_math`, double
    sub/sup diagnostics) stay after borrow drop. `new_choice_global` is gone.
    Good next targets are adjacent command helpers with the same shape:
    `math_ac`'s noad allocation/final accent field writes, the allocation branch
    of `math_fraction`, or a very small `math_left_right` allocation island.
    Keep the delimiter scan / keyword scan / fraction error paths outside live
    borrows, and avoid taking `scan.rs` proper or any high-count orchestrator.

    **Session 102 update:** `math_fraction_global`, `math_ac_global`, and
    `math_left_right_global` now split their noad allocation/finalization work
    into short direct `MemState`/`EqtbState` borrow islands. Delimiter scans,
    keyword scans, `scan_dimen`, `scan_math`, `fin_mlist_global`, `unsave`,
    `push_math`, and diagnostics stay outside live borrows. Deleted the dead
    `set_empty_math_field_global` bridge. The remaining nearby math-command
    helpers are less uniformly small: good next targets are local write islands
    inside `math_fraction`/`math_left_right` only if a follow-up simplification
    is obvious, or move to a pagebuilder/linebreak/shipout leaf. Keep
    scanner-heavy `scan.rs` and high-count orchestrators deferred.

    **Session 103 update:** the math radical-making delimiter adjustment
    in `make_radical_global` is now threaded. Skew/clearance adjustments and rule/overbar
    links run under direct disjoint `MemState` borrows, while `clean_box`, `overbar`,
    `vpackage`, and delimiter scans stay outside live borrows.

    **Session 105 update:** the local `deactivate_node` helper inside `src/linebreak.rs`'s
    `try_break` is now threaded to `&mut MemState`, `&ScannerState`, and `&mut LinebreakState`,
    directing active-list mutations and deallocations under safe borrows.

    **Session 107 update:** the DVI native/standard font definition helper functions
    `dvi_native_font_def` and `dvi_font_def_bytes` are now threaded to `&FontState` and
    `&StringsState` in `src/shipout.rs`. The `dvi_font_def_global` wrapper runs `make_font_def`
    completely outside the borrow to prevent nested mutable borrows, then formats and emits the
    DVI bytes under short, disjoint borrows. The dead bridge accessor `font_check_entry` is deleted
    from `src/globals.rs`.

    **Session 108 update:** the core memory leaf helper `prev_rightmost` is now threaded to `&MemState` in `src/mem.rs` and the redundant duplicate helper `prev_rightmost_in` is fully deleted from `src/linebreak.rs`. Legacy `scan.rs` call sites are repointed to `prev_rightmost_global`. We also threaded the local `is_ignorable_interword_node` and `skip_ignorable_interword_nodes` helpers in `src/shipout.rs` to safe `&MemState` borrows, calling them inside `merge_justifiable_native_words` via transient borrows.

    **Session 110 update:** completed an uncommitted WIP slice (it did **not** compile — an E0502 borrow conflict in `font_character_width`, fixed by precomputing `effective_char` into a local): `get_native_usv` (→ `&MemState`) and `font_character_width` (→ `&mut FontState`) in `src/linebreak.rs`, plus `font_character_width` (→ `&FontState`) and `is_char_node` (→ `&MemState`) in `src/shipout.rs`, all with `_global` bridge-down wrappers; removed 6 now-unused font `globals` imports from `shipout.rs`. Then **consolidated `is_char_node`**: added one canonical `pub fn is_char_node(mem: &MemState, p)` to `src/mem.rs` and deleted the three duplicated `is_char_node_in`/`is_char_node` copies in linebreak/math/shipout, repointing all `(mem, p)`-form callers to `crate::mem::is_char_node` (session-108 `prev_rightmost` dedup precedent). The separate `unsafe fn is_char_node(p)` global-bridge form in `math.rs` (6 in-module callers) is left for a later slice. **Recon note:** the `ext.rs` native-metric cluster scores as clean bucket B but its callers concentrate in `scan.rs`, so it is deferred until scanner threading — do not pull it in early.

    **Session 111 update:** two coherent leaf clusters in one commit (larger than
    110). (1) The 10 pure `mem`-arena **synctex node-field readers**
    (`synctex_tag_box`/`line_box`/`tag_rule`/`line_rule`/`tag_medium`/`line_medium`/
    `type`/`width`/`depth`/`height`, `src/synctex.rs`) threaded `unsafe fn(p)` →
    safe `fn(mem: &MemState, p)` (direct `b32s0`/`b32s1`/`b16s1`), each with a
    `_global` bridge-down wrapper; ~40 in-module call sites in the
    `synctex_record_node_*` recorders + public `synctex_*` recording functions
    renamed by a line-scoped `perl` pass. Parity-covered by the `synctex` golden.
    Removed 3 now-unused `mem_*` imports from `synctex.rs`. (2) The deferred
    `unsafe fn is_char_node(p)` global-bridge form in `math.rs` deleted; its 6
    callers routed through `crate::mem::is_char_node` via a safe `is_char_node_global`
    wrapper. `#[no_mangle]` 327, raw `unsafe {}` 175, raw externs 38 unchanged;
    `unsafe fn` 651→640.

    **Session 113 update:** fully threaded remaining `synctex_record_node_*` recorders and FFI call sites inside `src/synctex.rs` to accept explicit safe state borrows (`&MemState` / `&ShipoutState` / etc.) instead of calling unthreaded global wrappers. Cleaned up the now-dead internal `_global` accessor wrappers from `src/synctex.rs`, bringing Rust compiler warnings successfully down to **51**. All 29/2/7/4 tests pass green. Next: round out the synctex cluster (`intpar_synctex` -> `&EqtbState`, ~16 callers) or another small leaf; `ext.rs` stays scan.rs-deferred.

    Reuse the session-61/62/63 pattern: add any new methods to `impl MemState` (or
    `impl EqtbState`) in `state.rs`, thread the fn (propagating `ALLOC_FAILED` for the
    `i32` returns), and bridge-down legacy callers through a transitional `<fn>_global`
    wrapper (args evaluated before the borrow — avoids the argument nest-panic and the
    `|s|` closure-shadowing hazards that ruled out the recipe's bare inline closure;
    the wrapper runs the cold `overflow()` abort after the borrow drops). The wrapper
    dies when its last legacy caller threads. After the constructors and first save
    slices, climb the graph subsystem by subsystem (`pagebuilder.rs`/`shipout.rs` →
    `math.rs`/`linebreak.rs` → `scan.rs` last). Draft `02-modules.md` to READY before
    threading reaches `scan.rs`.

    `misc` bucket fully drained (sessions 44–46); classified-but-unmigrated
    scratch completed (sessions 47–48, 53–54); all ownership slices done,
    including the `mem` arena (session 58); dead `xmalloc_array`/`xcalloc_array`
    glue removed (session 59). The 40 remaining `mod raw` statics are the
    intentionally-deferred hot-tokenizer / capacity / `cur_ptr` set — no
    Phase-B slice remains.

    **Session 60 drafted `01b-phase-c-recipe.md` to READY** (hard rule 4 now
    satisfied for Phase C): the bridge-down technique (mirror of Phase B), the
    longjmp/`Drop` anchor resolution (§7 — owner stays in the thread-local,
    thread `&mut` below it), the precise nest-panic rule, and the cold-error-path
    sentinel for `overflow`/`runaway`. Recon **refuted** the session-59 blocker:
    `get_node`'s internal `line()`/`cur_input_synctex_tag()` read **raw** `mod raw`
    globals (not the thread-local), so they do **not** nest-panic under a live
    borrow; only the cold `overflow`/`runaway` abort path is `with_state`-bearing,
    and the sentinel technique handles it. `free_node`/`flush_list` touch `mem`
    only with zero `with_state` calls.

    **First slice (per `01b` §9):** the `mem` allocator core threaded as
    `&mut MemState`, in commit order `free_node` → `flush_list` → `get_avail` →
    `get_node`, one function + all its call sites per commit (recon counts:
    ~96 / ~21 / ~83 / ~86 sites, scan.rs-dominated). It is large but mechanical
    and is the dependency-graph foundation that makes later subsystems
    pass-through. Workstream 02 (module split) is still `PLANNED` and does not
    block the allocator core, but should reach READY before threading climbs into
    `scan.rs`.

    Sessions 44 (planning), 45 (`LinebreakState`, 34 globals), and 46 (the three
    private-static clusters below) drained `misc`. **Sessions 47–48** continued the
    classified-but-unmigrated scratch: 16 `shipout` DVI-output scalars (47) and 5
    `scanner` math-conversion scalars (48). **Session 49** began the `eqtb`-cluster
    arrays with the two inline BSS arrays (`prim`, `xeq_level`). **Session 53** moved
    13 DVI positioning/output statics to `ShipoutState`. **Session 54** completed the
    classified-but-unmigrated scratch by moving the remaining 18 scanner box/packaging/scratch
    globals to `ScannerState`. The decision block + per-field lists + newtype-debt rows live in
    `01a-global-inventory.md`.

    **`eqtb` array proper done (session 51)** — the malloc'd `*mut MemoryWord`
    moved to a Rust-owned `Vec` on `EqtbState.data`, with the sparse-region
    dump/undump reconciled via the font-array base-pointer pattern. **`hash`
    done (session 52)** and **the `mem` arena done (session 58)** — both via the
    same base-pointer dump/undump reconciliation. **Phase B is essentially
    complete:** every remaining static is the intentionally-deferred Phase-C set
    (hot tokenizer / lig-kern), a capacity limit, or `cur_ptr`. The next move is
    **Phase C** — thread `&mut EngineState`/`&mut MemState` through one decoupled
    subsystem (the `src/mem.rs` allocators are the natural first consumers, §5
    step 4) and delete its thread-local bridge.

    | Target | Members | Shape | Status |
    |---|---|---|---|
    | ~~`LinebreakState`~~ | 34 linebreak/LR-reverse scratch externs | scalar/inline-array; `hlist_stack` heap-backed `Vec<i32>` | **done (session 45)** |
    | ~~`ShipoutState` (extend)~~ | 4 pagebuilder private statics | plain `i32` fields + module-private accessors | **done (session 46)** |
    | ~~`EngineState.synctex: SynctexCtxt`~~ | `SYNCTEX_CTXT` (264 sites) | struct moved to `state.rs`, manual `Default` (`lastv:-1`), macro-generated per-field bridge | **done (session 46)** |
    | ~~`TexmfState` (new)~~ | `LAST_SOURCE_NAME`/`LAST_LINENO`/`START_TIME_STR[30]` | new sub-struct; owned-C-string lifecycle preserved | **done (session 46)** |

    **Sessions 47+ = Phase C becomes viable.** With `misc` drained, the only
    `mod raw` statics of substance are the intentionally-deferred Phase-C set (hot tokenizer
    `cur_cmd/chr/tok/cs/val`, mem arena, hot lig/kern) plus the ownership slices (`eqtb`,
    `hash`, `prim`, `sa_root`, …) and classified-but-unmigrated scratch. Phase C = threading
    `&mut EngineState` through signatures, removing the thread-local bridge, near-zero unsafe.
    Candidate next slices: an ownership slice (`eqtb`/`hash`/`prim` arrays — these are in the
    format dump, so reconcile load/dump per §11 step 6) or begin migrating the
    classified-but-unmigrated `scanner`/`shipout` scratch into their assigned clusters.

    After session 49, ~82 raw statics remained, in four buckets:

    After session 58, **40 raw statics remain, all Phase-C / capacity / cur_ptr —
    Phase B is essentially complete** (the last ownership slice, the `mem` arena,
    is done):

    | Bucket | ~Count | Fate |
    |---|---:|---|
    | Intentionally deferred: hot tokenizer (`cur_cmd/chr/tok/cs/val`), hot lig/kern (`main_*`, `cur_l/r/q`, `lig_stack`, `bchar`, …), `cur_input`/`cur_list`/`line`/`temp_ptr` | ~27 | Stay raw until Phase C threading |
    | Ownership slices — **all done**: `prim`/`xeq_level` s49, `eqtb` array s51, `hash` s52, `sa_root` s55, `xdv_buffer`/`mapped_text`/`native_text` s57, `mem` arena **s58** | 0 | Completed |
    | Capacity/size limits (`stack_size`, `nest_size`, `buf_size`, `param_size`, `save_size`, `max_in_open`, `expand_depth`, `first`) | 8 | Probably stay as C forever |
    | `cur_ptr` (148 uses, too broad to bridge without benchmarking) | 1 | Thread in Phase C |

    **Do not bridge the hot tokenizer scalars** (`cur_cmd`, `cur_chr`, `cur_tok`, `cur_cs`,
    `cur_val`) through the thread-local — they are read on nearly every
    `get_next`/`get_token`. Thread them in Phase C instead.

    **Recurring gotchas (still apply to all batch sessions):**
    - Any scalar backed by `with_state` must NOT be called inside a `with_state_mut`
      closure — the RefCell panics. Pre-compute before the borrow.
    - Writing to a `MemoryWord` union field through a `Vec<MemoryWord>` index (via `&mut`)
      is *safe* — no `unsafe` needed for union writes, only for reads (session 41).
    - Use `01a-global-inventory.md` for C defs / cleanup aliases / accessors, and add a
      worklist row for any semantic-typed field (§4 rule).

2.  The `config` bridge pattern in `src/state.rs` is intentionally small:
    keep `with_state` / `with_state_mut` borrows to single-field
    read/write closures and avoid calling translated engine code while a
    borrow is live.
3.  The `misc` bucket is resolved (session 44) **and fully migrated** (sessions
    45–46): `LinebreakState` + the three private-static clusters are all on
    `EngineState`. No `misc` rows remain.
4.  Cheap clippy wins (04) are always available as low-risk session
    filler, but do not touch the unsafe-block bucket yet.
