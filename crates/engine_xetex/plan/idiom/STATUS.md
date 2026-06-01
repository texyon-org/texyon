# Idiomatization status

The authoritative tracker for Stage 2. Two sections: **plan
construction** (are the workstream plans written and ready?) and
**execution** (are the refactors done?). Update at every session end.

Last updated: 2026-06-01 (last session = session 120, **executed workstream-02 slice 3**: mechanically relocated the 12 file-naming / `start_input` / `close_files` fns (`begin_name`/`more_name`/`end_name`/`make_name_string`/`scan_and_pack_name`/`scan_file_name_braced`/`scan_file_name`/`pack_job_name`/`open_log_file`/`start_input`/`pack_file_name`/`close_files_and_terminate`) verbatim from `scan.rs` into `io.rs` via `move_fns.py` (559 fn-lines), plus the private `mod raw_utf8` (start_input's only scan-private dep). Larger import fixup (40 globals accessors + 13 dup consts + ttstub externs into io.rs); repointed `crate::scan::<fn>` call-sites in ini.rs/shipout.rs/errors.rs/pic.rs to `crate::io::`. Warning-count discipline drove the orphan cleanup in scan.rs (6 unused imports, 7 orphaned consts, 3 dead ttstub externs, 4 stale banners). No pattern-const trap this slice. `scan.rs` 16,850→**16,255** LOC (−595; still largest module). Dashboard 29/2/7/4, warnings 51. — prior session 119, **executed workstream-02 slice 2**: mechanically relocated the 12 `show_*`/display/messaging fns (`show_box`/`show_node_list`/`short_display`/`short_display_n`/`show_activities`/`show_context`/`show_save_groups`/`show_whatever`/`show_info`/`print_subsidiary_data`/`issue_message`/`char_warning`) verbatim from `scan.rs` into `output.rs` via `move_fns.py` (1,556 fn-lines). Also relocated the 78 single-function-private `*_SD`/`*_SA`/`*_SNL` const blocks and imported the 7 shared node-type consts from `consts.rs` — the §7 pattern-const trap fired **in bulk** here (51→102 warnings from 22 `unreachable pattern` + 14 binding warnings; warning count was the only signal, zero compile errors), fixed by following the consts. `scan.rs` 18,537→**16,850** LOC (−1,687; still largest module). Dashboard 29/2/7/4, warnings 51. — prior session 118, **executed workstream-02 slice 1**: mechanically relocated the 8 `errors`-group fns (`you_cant`/`report_illegal_case`/`privileged`/`cs_error`/`box_error`/`mu_error`/`runaway`/`extra_right_brace`) verbatim from `scan.rs` into `errors.rs` using the new `plan/idiom/move_fns.py` relocator. `scan.rs` 18,689→18,537 LOC. Caught + fixed a latent match-pattern-const behavior trap (now a §7 recipe gotcha: check warning count, not just errors). Dashboard 29/2/7/4, warnings 51. — prior session 117, **drafted `02-modules.md` to READY** — the module-decomposition workstream that lifts the `scan.rs`/`ini.rs` deferral. It maps the two grab-bags onto the `01-architecture.md` layout, decides **mechanical pre-split first then thread in place** (parity-trivial relocation decoupled from semantic threading; supersedes the `01b` §9 "thread+move together" suggestion for giant files), fences out-of-scope work (`node/` typed accessors → ws03, hot-scalar migration → later benchmarked slice, `state/` split → independent), and sets the first slice = drain `errors`/`output` leaves out of `scan.rs`. No code change this session. — prior session 116, **finished the `char_pw` protrusion-width leaf**: folded the two deferred `char_pw` calls into a fully-threaded `total_pw` — `total_pw(mem, &FontState, linebreak, q, p) -> Option<i32>` now sums `char_pw_in` inside its own borrow (the session-76 deferred split is no longer needed since `char_pw_in` is bridge-free) — deleted the legacy `char_pw` wrapper and the now-dead `set_last_leftmost_char`/`set_last_rightmost_char` globals accessors, and reverted unrelated math.rs reborrow churn from the inherited WIP. Dashboard 29/2/7/4, warnings 51. Recon `dead-wrapper` rows for `is_char_node_global`/`font_character_width_global`/etc. are false positives — those `#[inline]` wrappers are heavily used. — prior session 115 completed the `src/synctex.rs` Phase-C threading pass** — converted SyncTeX entry points to explicit `&mut EngineState` functions, bridged callers at the current legacy boundaries in `ini.rs`/`scan.rs`/`shipout.rs`, deleted unused xform/abort API remnants exposed by de-exporting, and left `src/synctex.rs` with no active `#[no_mangle]` exports and no `with_state*` calls. Dashboard 29/2/7/4, warnings 51. — prior session 114 finished local SyncTeX bridge cleanup.)

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
| 01 | `01-state-and-memory.md` | READY | First/highest-leverage workstream. Phase B complete. §11 is the Phase-B recipe; Phase-C threading recipe is the separate `01b`. |
| 01b | `01b-phase-c-recipe.md` | READY | Phase-C execution recipe (thread `&mut EngineState`/`&mut MemState`). Resolves the longjmp/`Drop` anchor (§7) + nest-panic rule; first slice = the `mem` allocator core. Drafted + recon-validated session 60. |
| 02 | `02-modules.md` | EXECUTING | Module decomposition. Slice 1 done (session 118: `errors` group → `errors.rs`); slice 2 done (session 119: `show_*`/display group → `output.rs`); slice 3 done (session 120: file-naming/`start_input`/`close_files` group → `io.rs`, `scan.rs` 18,537→16,255 LOC). Drafted + source-verified session 117: drains the two grab-bags (`scan.rs`←xetex0.c 18,689 LOC/222 fns, `ini.rs`←xetex-ini.c) into the `01-architecture.md` layout. Decision: **mechanical pre-split first, thread in place after** (supersedes `01b` §9 "together"). New modules: `scanner`/`expand`/`main_control`/`token`/`engine`/`format_io`/`packaging`(transitional→`node/`)/`hyphenation`. Unblocks scanner-side Phase C. |
| 03 | `03-deunsafe-ffi.md` | PLANNED | De-unsafe + FFI teardown. Mostly falls out of 01. |
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
