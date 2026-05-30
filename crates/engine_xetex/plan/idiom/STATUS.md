# Idiomatization status

The authoritative tracker for Stage 2. Two sections: **plan
construction** (are the workstream plans written and ready?) and
**execution** (are the refactors done?). Update at every session end.

Last updated: 2026-05-30 (last execution = session 54, remaining 18 box/packaging/scratch globals → EngineState.scanner).

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
| 01 | `01-state-and-memory.md` | READY | First/highest-leverage workstream. Execute least-coupled non-`misc` clusters first; do not migrate `misc` rows until their final sub-structs are decided. |
| 02 | `02-modules.md` | PLANNED | Module decomposition. Draft after 01's cluster boundaries are settled. |
| 03 | `03-deunsafe-ffi.md` | PLANNED | De-unsafe + FFI teardown. Mostly falls out of 01. |
| 04 | `04-clippy.md` | PLANNED | Cheap clippy wins now; unsafe-block bucket after 01/03. |
| 05 | `05-errors-and-misc.md` | PLANNED | Error-handling shape, dead-comment cleanup, longjmp/Drop. Partly optional. |
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

### Idiom metrics (baseline = session 132, end of Stage 1)

These trend toward their targets as workstreams execute. Re-measure with
the snippets in `00-overview.md` / the workstream plans.

| Metric | Baseline (2026-05-23) | Target | Current |
|---|---:|---|---:|
| `extern static mut` globals (`globals.rs`) | 384 | 0 (non-const) | 60 (session 54 moved remaining 18 scanner scratch globals) |
| promoted Rust `pub static mut` globals | 16 | 0 (non-const) | 3 (session 53 moved 13 DVI positioning/output statics) |
| thin global accessors (`globals.rs`) | 811 | 0 | 823 total / 178 state-backed scalars/arrays (incl. 18 session-54 scanner scratch and 33 `LinebreakState` inline scalars/arrays) + 69 Rust-owned arrays/arenas (save_stack, param_stack, line/grp/if/source_filename/full_source_filename stacks, eof_seen, buffer, input_stack, input_file, nest, str_start, str_pool, 11 fixed-size hyph arrays, 9 dynamic trie arrays, 3 hyph exception arrays, font_check, font_size, font_dsize, font_params, font_name, font_area, font_bc, font_ec, bchar_label, font_bchar, font_false_bchar, font_glue, font_used, 9 *_base arrays, font_flags, font_letter_space, font_mapping, font_layout_engine, hyphen_char, skew_char, font_info, hlist_stack, hash) + session-46 clusters: 4 `ShipoutState` pagebuilder scalars, `TexmfState` (3 fields), `EngineState.synctex` (`SynctexCtxt`, 15 scalar + 5 flag accessor pairs) |
| `#[no_mangle]` exports | 576 | boundary-only | 579 |
| `unsafe fn` (crate-wide) | 694 | boundary-only | 694 |
| raw `unsafe { ... }` blocks | 846 text-search / 688 clippy-documented bucket | boundary-only | 236 / 688 not remeasured |
| clippy errors | 694 | 0 | 694 not remeasured |
| clippy warnings | 358 | 0 | 358 not remeasured |
| largest module LOC | 18,748 (`scan.rs`) | none >~3k | 18,748 (`scan.rs`) |

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

## Next action for a fresh session

0.  Follow the **Execution recipe** in `01-state-and-memory.md` §11
    (grep checks → add field → repoint accessors → remove defs/aliases →
    worklist rows → dashboard → docs → commit) and heed its recurring
    gotchas.
1.  **`misc` bucket fully drained (sessions 44–46). Classified-but-unmigrated
    scratch completed (sessions 47-48, 53-54). Next: the `mem` arena (§5), remaining ownership slices, or Phase C.**

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
    dump/undump reconciled via the font-array base-pointer pattern. **Next
    high-value slice: `hash`** (the `yhash` offset-pointer trick `hash = yhash -
    hash_offset` + the `tt_run_engine_allocate_yhash` C helper + `hash_offset`/
    cleanup; the `Vec` accessor subtracts `hash_offset`). Then the `mem` arena
    (§5) — the hottest move and the real pivot into Phase C.

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

    | Bucket | ~Count | Fate |
    |---|---:|---|
    | Intentionally deferred: hot tokenizer (`cur_cmd/chr/tok/cs/val`), mem arena (`mem`, `lo_mem_max`, `hi_mem_min`, `avail`, `mem_end`, `rover`, `var_used`, `dyn_used`), hot lig/kern (`main_*`, `cur_l/r/q`, etc.) | 37 | Stay raw until Phase C |
    | Ownership slices (`hash`, `sa_root`, `xdv_buffer`, `name_of_file*`, `native_text`) — `prim`/`xeq_level` done s49, `eqtb` array done s51 | 9 | Need format-dump or alloc rewrite work |
    | Classified-but-unmigrated scratch (shipout DVI statics done s53, scanner box/packaging done s54) | 0 | Completed |
    | Capacity/size limits (`stack_size`, `nest_size`, etc.) | 5 | Probably stay as C forever |

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
