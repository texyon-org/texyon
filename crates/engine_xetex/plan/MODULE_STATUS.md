# Module status

Authoritative status of the C → Rust migration. Updated at the end of
every session. The column meanings:

-   **LOC (C)**: lines of C still remaining in that file
-   **Fns (C)**: count of functions still in C (best-effort — use
    `grep -c '^[[:alpha:]_].*(' <file>` for a rough count)
-   **Fns (Rust)**: count of functions translated to Rust in the
    corresponding module
-   **Status**:
    -   `NOT STARTED` — no functions translated
    -   `IN PROGRESS — N%` — partial
    -   `DONE` — zero C remaining in the file
    -   `SKIPPED` — deliberately not translating (third-party, e.g.
        teckit)

## Summary

| Status         | File count |
|----------------|-----------:|
| NOT STARTED    |          8 |
| IN PROGRESS    |          3 |
| DONE           |          5 |
| SKIPPED        |          0 |
| **Total**      |     **16** |

Last updated: session 38.


## Core engine files (to translate)

| C source file                | LOC   | Fns (C) | Rust target module                | Status       |
|------------------------------|------:|--------:|-----------------------------------|--------------|
| xetex-scaledmath.c           |    20 |      0 | `src/scaled_math.rs`              | DONE               |
| xetex-stringpool.c           |    13 |      0 | `src/string_pool.rs`              | DONE               |
| xetex-errors.c               |    12 |      0 | `src/errors.rs`                   | DONE         |
| xetex-output.c               |    15 |      0 | `src/output.rs`                   | DONE               |
| xetex-ini.c                  |  3881 |    ~?? | `src/ini.rs` + state modules      | NOT STARTED  |
| xetex-xetex0.c               | 14434 |    119 | multiple (see below)              | IN PROGRESS — 57% |

| xetex-pagebuilder.c          |   804 |    ~?? | `src/pagebuilder.rs`              | NOT STARTED  |
| xetex-linebreak.c            |  2843 |    ~?? | `src/linebreak.rs`                | NOT STARTED  |
| xetex-math.c                 |  3833 |    ~?? | `src/math.rs`                     | NOT STARTED  |
| xetex-shipout.c              |  2451 |    ~?? | `src/shipout.rs`                  | NOT STARTED  |
| xetex-synctex.c              |  1283 |    ~?? | `src/synctex.rs`                  | NOT STARTED  |
| xetex-io.c                   |   294 |      3 | `src/io.rs`                       | IN PROGRESS — 73% |
| xetex-ext.c                  |  2170 |    ~?? | `src/ext.rs`                      | NOT STARTED  |
| xetex-pic.c                  |   664 |    ~?? | `src/pic.rs`                      | NOT STARTED  |
| xetex-texmfmp.c              |    45 |      0 | `src/texmfmp.rs`                  | DONE              |
| xetex-engine-interface.c     |    85 |       2 | `src/engine_interface.rs`        | IN PROGRESS — 50% (2 fns stay in C: setjmp + platform hack) |

## Files NOT to translate

| File                              | Reason                                       |
|-----------------------------------|----------------------------------------------|
| teckit-Engine.cpp                 | third-party TECkit, retain as C++            |
| teckit-NormalizationData.c        | third-party TECkit                           |
| xetex-XeTeXOTMath.cpp             | OpenType math — C++ binding to HarfBuzz      |
| xetex-macos.c                     | platform glue; not present on Linux          |

## Function-level tracker for xetex-xetex0.c (the big one)

`xetex-xetex0.c` is 273 functions. This is where the bulk of the work
lives. We track functions in phases. Each phase maps to a Rust module
in `04-migration-order.md`. When a phase is started, fill in the sub-
table with per-function rows; don't pre-populate all 273 up front —
that's busywork and goes stale.

### Phase 1 — pure helpers (target: `src/scaled_math.rs`, `src/helpers.rs`)

Status: IN PROGRESS (Session 1).

#### `xetex-scaledmath.c` (14 functions, 513 LOC)

All use only `arith_error` / `tex_remainder` globals and pure arithmetic,
except `m_log` which calls into the diagnostic machinery (defer until
Phase 3 is far enough along).

| # | Function              | Line | Kind            | Globals used              | Status |
|--:|-----------------------|-----:|-----------------|---------------------------|--------|
| 1 | `tex_round`           |   11 | extern, pure    | —                         | MIGRATED (session 1) |
| 2 | `half`                |   51 | extern, pure    | —                         | MIGRATED (session 1) |
| 3 | `mult_and_add`        |   60 | extern          | `arith_error`             | MIGRATED (session 1) |
| 4 | `x_over_n`            |   79 | extern          | `arith_error`, `tex_remainder` | MIGRATED (session 1) |
| 5 | `xn_over_d`           |  105 | extern          | `arith_error`, `tex_remainder` | MIGRATED (session 1) |
| 6 | `round_xn_over_d`     |  137 | extern          | `arith_error`             | MIGRATED (session 1) |
| 7 | `make_frac`           |  165 | **static**→extern | `arith_error`          | MIGRATED (session 2, 3f096961) |
| 8 | `take_frac`           |  220 | **static**→extern | `arith_error`          | MIGRATED (session 2, 3f096961) |
| 9 | `m_log`               |  287 | extern          | diagnostic+help globals   | MIGRATED (session 10, 7509cc5d) |
|10 | `ab_vs_cd`            |  332 | **static**→extern | —                      | MIGRATED (session 2, 3f096961) |
|11 | `new_randoms`         |  418 | **static**→extern | `randoms`, `j_random`  | MIGRATED (session 2, 3f096961) |
|12 | `init_randoms`        |  441 | extern          | `randoms`, `j_random`     | MIGRATED (session 2, 3f096961) |
|13 | `unif_rand`           |  468 | extern          | `randoms`, `j_random`     | MIGRATED (session 2, 3f096961) |
|14 | `norm_rand`           |  487 | extern          | `randoms`, `j_random`     | MIGRATED (session 2, 3f096961) |

Session-1 rule-of-thumb on `static` C functions: when translating a
`static` helper, either translate its caller at the same time (so the
Rust version calls Rust directly), or remove the `static` qualifier in
the C (exporting the symbol) so the Rust replacement can be reached by
name across the FFI boundary. Do **not** invent a new C wrapper just to
un-hide a symbol; edit the definition directly.

### Phase 1.5 — pure helpers from xetex-xetex0.c (target: `src/helpers.rs`)

| # | Function  | Line | Status |
|--:|-----------|-----:|--------|
| 1 | `badness` |   25 | MIGRATED (session 3, e19ace3f) |

### Phase 2 — memory / node allocation (target: `src/mem.rs`)

Status: IN PROGRESS (session 3).

| # | Function                 | Line | Status |
|--:|--------------------------|-----:|--------|
| 1 | `get_avail`              |  170 | MIGRATED (session 3, e19ace3f) |
| 2 | `flush_list`             |  192 | MIGRATED (session 3, e19ace3f) |
| 3 | `get_node`               |  206 | MIGRATED (session 3, fc914449) |
| 4 | `free_node`              |  284 | MIGRATED (session 3, fc914449) |
| 5 | `new_null_box`           |  296 | MIGRATED (session 4, 8fef23ac) |
| 6 | `new_rule`               |  313 | MIGRATED (session 4, 8fef23ac) |
| 7 | `new_ligature`           |  325 | MIGRATED (session 4, 8fef23ac) |
| 8 | `new_lig_item`           |  337 | MIGRATED (session 4, 8fef23ac) |
| 9 | `new_disc`               |  346 | MIGRATED (session 4, 8fef23ac) |
|10 | `copy_native_glyph_info` |  359 | MIGRATED (session 5, 37480639) |
|11 | `new_math`               |  372 | MIGRATED (session 4, 8fef23ac) |
|12 | `new_spec`               |  382 | MIGRATED (session 4, 8fef23ac) |
|13 | `new_param_glue`         |  394 | MIGRATED (session 4, 8fef23ac) |
|14 | `new_glue`               |  409 | MIGRATED (session 4, 8fef23ac) |
|15 | `new_skip_param`         |  421 | MIGRATED (session 4, 8fef23ac) |
|16 | `new_kern`               |  432 | MIGRATED (session 4, 8fef23ac) |
|17 | `new_penalty`            |  442 | MIGRATED (session 4, 8fef23ac) |

### Phase 3 — printing (target: `src/output.rs`, `src/show.rs`)

Status: IN PROGRESS (session 6).

Key constraint: `warn_char` and `current_diagnostic` are a C file-static
variable and static function in `xetex-output.c`. Functions that call
`warn_char` (i.e., `print_ln`, `print_raw_char`) must be translated after
`warn_char` is un-static'd or together with it.

Functions that don't call `warn_char` can be translated independently.

| # | Function             | Line | Status |
|--:|----------------------|-----:|--------|
| 1 | `capture_to_diagnostic`           |   13 | MIGRATED (session 9, 1d48b016) |
| 2 | `diagnostic_begin_capture_warning_here` | 47 | MIGRATED (session 9, 1d48b016) |
| 3 | `error_here_with_diagnostic`      |   57 | MIGRATED (session 9, 1d48b016) |
| 3b| `warn_char` (was static)          |   75 | MIGRATED (session 8, 392e3220 — un-static'd d7e39b4d) |
| 4 | `print_ln`           |   84 | MIGRATED (session 8, 392e3220) |
| 5 | `print_raw_char`     |  116 | MIGRATED (session 8, 7b8944cf) |
| 6 | `print_char`         |  177 | MIGRATED (session 8, 62cd93b1) |
| 7 | `print`              |  247 | MIGRATED (session 8, 13671cf5) |
| 8 | `print_cstr`         |  301 | MIGRATED (session 7, 555d6b46) |
| 9 | `print_nl`           |  310 | MIGRATED (session 8, 4f7ad6bd) |
|10 | `print_nl_cstr`      |  318 | MIGRATED (session 7, 555d6b46) |
|11 | `print_esc`          |  327 | MIGRATED (session 8, 4f7ad6bd) |
|12 | `print_esc_cstr`     |  338 | MIGRATED (session 7, 555d6b46) |
|13 | `print_the_digs`     |  350 | MIGRATED (session 6, 6e9a8bea — private Rust helper, C static deleted) |
|14 | `print_int`          |  363 | MIGRATED (session 6, 6e9a8bea) |
|15 | `print_cs`           |  397 | MIGRATED (session 9, 1d48b016) |
|16 | `sprint_cs`          |  429 | MIGRATED (session 9, 1d48b016) |
|17 | `print_file_name`    |  449 | MIGRATED (session 9, 1d48b016) |
|17b| `diagnostic_print_file_line` (was static) | — | MIGRATED (session 9, 1d48b016 — private Rust helper) |
|18 | `print_file_line`    |  554 | MIGRATED (session 9, 1d48b016) |
|19 | `print_two`          |  554 | MIGRATED (session 8, a4e5e260) |
|20 | `print_hex`          |  658 | MIGRATED (session 6, 6e9a8bea) |
|21 | `print_roman_int`    |  563 | MIGRATED (session 8, a4e5e260) |
|22 | `print_current_string` | 600 | MIGRATED (session 8, 16cdc895) |
|23 | `print_scaled`       |  612 | MIGRATED (session 8, a4e5e260) |
|24 | `print_ucs_code`     |  749 | MIGRATED (session 6, 6e9a8bea) |
|25 | `print_size`         |  233 | MIGRATED (session 9, 1d48b016) |
|26 | `print_write_whatsit`|  245 | MIGRATED (session 9, 1d48b016) |
|27 | `print_native_word`  |  260 | MIGRATED (session 9, 1d48b016) |
|28 | `print_sa_num`       |  285 | MIGRATED (session 9, 1d48b016) |

### Phase 4 — string pool (target: `src/string_pool.rs`)

Status: DONE (session 10).

| # | Function             | Line | Status |
|--:|----------------------|-----:|--------|
| 1 | `load_pool_strings`  |   17 | MIGRATED (session 10, 5e966ad6) |
| 2 | `length`             |   42 | MIGRATED (session 10, 5e966ad6) |
| 3 | `make_string`        |   57 | MIGRATED (session 10, 5e966ad6) |
| 4 | `append_str`         |   69 | MIGRATED (session 10, 5e966ad6) |
| 5 | `str_eq_buf`         |   91 | MIGRATED (session 10, 5e966ad6) |
| 6 | `str_eq_str`         |  119 | MIGRATED (session 10, 5e966ad6) |
| 7 | `search_string`      |  162 | MIGRATED (session 10, 5e966ad6) |
| 8 | `slow_make_string`   |  190 | MIGRATED (session 10, 5e966ad6) |

### Phase 5 — error handling (target: `src/errors.rs`)

Status: DONE (session 11).

| # | Function                | Line | Status |
|--:|-------------------------|-----:|--------|
| 1 | `pre_error_message` (static) |  13 | MIGRATED (session 11, 480fe9f1 — private Rust fn) |
| 2 | `post_error_message` (static) | 35 | MIGRATED (session 11, 480fe9f1 — private Rust fn) |
| 3 | `error`                 |   52 | MIGRATED (session 11, 480fe9f1) |
| 4 | `fatal_error`           |   98 | MIGRATED (session 11, 480fe9f1) |
| 5 | `overflow`              |  113 | MIGRATED (session 11, 480fe9f1) |
| 6 | `confusion`             |  132 | MIGRATED (session 11, 480fe9f1) |
| 7 | `pdf_error`             |  157 | MIGRATED (session 11, 480fe9f1) |

### Phase 6 — engine interface (target: `src/engine_interface.rs`)

Status: IN PROGRESS (session 12). Two functions translated; two kept in C permanently.

| # | Function                              | Line | Status |
|--:|---------------------------------------|-----:|--------|
| 1 | `tt_xetex_set_int_variable`           |   25 | MIGRATED (session 12, 73725033) |
| 2 | `tt_xetex_set_string_variable`        |   44 | MIGRATED (session 12, 73725033) |
| 3 | `tt_engine_xetex_main`                |   51 | KEEP IN C (setjmp/longjmp cannot cross FFI boundary safely) |
| 4 | `__terrible_aarch64_musl_linker_hack` |  103 | KEEP IN C (platform linker workaround) |

### Phase 7 — texmfmp time helpers (target: `src/texmfmp.rs`)

Status: IN PROGRESS (session 13).

| # | Function                | Line | Status |
|--:|-------------------------|-----:|--------|
| 1 | `get_date_and_time`     |  116 | MIGRATED (session 13, 4bafb5ef) |
| 2 | `get_seconds_and_micros`|  128 | MIGRATED (session 13, 4bafb5ef) |

### Phase 8 — texmfmp string/UTF-8 helpers (target: `src/texmfmp.rs`)

Status: IN PROGRESS (session 14).

| # | Function                        | Line | Status |
|--:|---------------------------------|-----:|--------|
| 1 | `gettexstring`                  |  282 | MIGRATED (session 14, 717cf3d4) |
| 2 | `checkpool_pointer` (was static)|  237 | MIGRATED (session 14, 717cf3d4 — private Rust fn) |
| 3 | `maketexstring`                 |  245 | MIGRATED (session 14, 717cf3d4) |
| 4 | `convertStringToHexString` (was static)| 396 | MIGRATED (session 14, 717cf3d4 — private Rust fn) |
| 5 | `getmd5sum`                     |  413 | MIGRATED (session 14, 717cf3d4) |

| # | Function                        | Line | Status |
|--:|---------------------------------|-----:|--------|
| 6 | `make_src_special`              |  274 | MIGRATED (session 15, 23d0822a) |
| 7 | `getfilesize`                   |  155 | MIGRATED (session 15, 23d0822a) |
| 8 | `getfiledump`                   |  189 | MIGRATED (session 15, 23d0822a) |
| 9 | `compare_paths` (was static)    |  243 | MIGRATED (session 15, 23d0822a — private Rust fn) |
|10 | `is_new_source`                 |  257 | MIGRATED (session 15, 23d0822a) |
|11 | `remember_source_info`          |  264 | MIGRATED (session 15, 23d0822a) |
|12 | `makepdftime` (was static)      |   30 | MIGRATED (session 15, 13fb941d — private Rust fn) |
|13 | `init_start_time`               |   84 | MIGRATED (session 15, 13fb941d) |
|14 | `getcreationdate`               |   91 | MIGRATED (session 15, 13fb941d) |
|15 | `getfilemoddate`                |  120 | MIGRATED (session 15, 13fb941d) |

`xetex-texmfmp.c` is **DONE** (0 C functions remaining).

### Phase 10 — xetex-io.c I/O helpers (target: `src/io.rs`)

Status: IN PROGRESS (session 16).

| # | Function                  | Line | Status |
|--:|---------------------------|-----:|--------|
| 1 | `tt_xetex_open_input`     |   22 | MIGRATED (session 16, 979cde6d) |
| 2 | `u_open_in`               |  127 | MIGRATED (session 16, 979cde6d) |
| 3 | `make_utf16_name`         |  481 | MIGRATED (session 16, 979cde6d) |
| 4 | `get_uni_c`               |  373 | MIGRATED (session 16, 979cde6d) |
| 5 | `u_close`                 |  357 | KEEP IN C — `ucnv_close` (ICU) not linkable from Rust without build-script changes |
| 6 | `set_input_file_encoding` |   87 | DEFER — uses ICU (ucnv_open/ucnv_close) |
| 7 | `buffer_overflow` (static)|  176 | DEFER — called only from `input_line` |
| 8 | `conversion_error` (static)|  183 | DEFER — called only from `input_line` |
| 9 | `apply_normalization` (static)| 195 | DEFER — uses TECkit static state |
|10 | `input_line`              |  220 | DEFER — complex ICU/TECkit encoding |
|11 | `open_or_close_in`        |  294 | MIGRATED (session 17, f4a94ffc) |

Note: `name_of_input_file` and `abspath_of_input_file` moved from C file-statics
to Rust `#[no_mangle] pub static mut` in `src/io.rs`. `NAME16LEN` also moved.

### Phase 11 — xetex-xetex0.c scattered functions (sessions 17+)

New Rust module `src/save.rs` created for save-stack management.

| # | Function                  | Source Line | Module       | Status |
|--:|---------------------------|------------:|--------------|--------|
| 1 | `delete_token_ref`        |         976 | `src/mem.rs` | MIGRATED (session 17, 3e1f337e) |
| 2 | `delete_glue_ref`         |         984 | `src/mem.rs` | MIGRATED (session 17, 3e1f337e) |
| 3 | `begin_diagnostic`        |        1785 | `src/output.rs` | MIGRATED (session 17, 1be1cb50) |
| 4 | `end_diagnostic`          |        1797 | `src/output.rs` | MIGRATED (session 17, 1be1cb50) |
| 5 | `print_mode`              |        1327 | `src/output.rs` | MIGRATED (session 17, d30eb43b) |
| 6 | `print_in_mode`           |        1356 | `src/output.rs` | MIGRATED (session 17, d30eb43b) |
| 7 | `print_group`             |        3464 | `src/output.rs` | MIGRATED (session 17, 6a81482c) |
| 8 | `new_save_level`          |        3859 | `src/save.rs` | MIGRATED (session 17, 5c29bdaf) |
| 9 | `eq_save`                 |        3914 | `src/save.rs` | MIGRATED (session 17, 5c29bdaf) |
|10 | `eq_word_define`          |        3954 | `src/save.rs` | MIGRATED (session 17, 5c29bdaf) |
|11 | `geq_word_define`         |        3977 | `src/save.rs` | MIGRATED (session 17, 5c29bdaf) |
|12 | `save_for_after`          |        3984 | `src/save.rs` | MIGRATED (session 17, 5c29bdaf) |
|13 | `int_error` (was static)  |          15 | `src/errors.rs` | MIGRATED (session 17, 9840db6e) |
|14 | `print_font_and_char`     |         225 | `src/output.rs` | MIGRATED (session 19, 4a027338) |
|15 | `print_mark`              |         240 | `src/output.rs` | MIGRATED (session 19, 4a027338) |
|16 | `print_meaning`           |        3383 | `src/output.rs` | MIGRATED (session 19, 4a027338) |
|17 | `group_warning`           |        2961 | `src/output.rs` | MIGRATED (session 19, 4a027338) |
|18 | `if_warning`              |        2998 | `src/output.rs` | MIGRATED (session 19, 4a027338) |
|19 | `file_warning`            |        3039 | `src/output.rs` | MIGRATED (session 19, 4a027338) |
|20 | `id_lookup`               |        2713 | `src/hash.rs`   | MIGRATED (session 20, 8ae4e8d1) |
|21 | `prim_lookup`             |        2803 | `src/hash.rs`   | MIGRATED (session 20, 8ae4e8d1) |
|22 | `mu_error`                |        5050 | `src/scan.rs`   | MIGRATED (session 20, 467271cd) |
|23 | `scan_char_class`         |        5078 | `src/scan.rs`   | MIGRATED (session 20, 467271cd) |
|24 | `scan_char_class_not_ignored` |    5096 | `src/scan.rs`   | MIGRATED (session 20, 467271cd) |
|25 | `scan_eight_bit_int`      |        5114 | `src/scan.rs`   | MIGRATED (session 20, 467271cd) |
|26 | `scan_usv_num`            |        5132 | `src/scan.rs`   | MIGRATED (session 20, 467271cd) |
|27 | `scan_char_num`           |        5150 | `src/scan.rs`   | MIGRATED (session 20, 467271cd) |
|28 | `scan_math_class_int`     |        5330 | `src/scan.rs`   | MIGRATED (session 20, 467271cd) |
|29 | `scan_math_fam_int`       |        5348 | `src/scan.rs`   | MIGRATED (session 20, 467271cd) |
|30 | `scan_four_bit_int`       |        5366 | `src/scan.rs`   | MIGRATED (session 20, 467271cd) |
|31 | `scan_fifteen_bit_int`    |        5384 | `src/scan.rs`   | MIGRATED (session 20, 467271cd) |
|32 | `scan_delimiter_int`      |        5403 | `src/scan.rs`   | MIGRATED (session 20, 467271cd) |
|33 | `scan_register_num`       |        5245 | `src/scan.rs`   | MIGRATED (session 20, c54d3a06) |
|34 | `scan_four_bit_int_or_18` |        5263 | `src/scan.rs`   | MIGRATED (session 20, c54d3a06) |
|35 | `scan_xetex_math_char_int`|        5168 | `src/scan.rs`   | MIGRATED (session 20, 64deab5b) |
|36 | `delete_sa_ref`           |        2789 | `src/save.rs`   | MIGRATED (session 21, 2c28bd03) |
|37 | `sa_save`                 |        2837 | `src/save.rs`   | MIGRATED (session 21, 2c28bd03) |
|38 | `sa_destroy`              |        2878 | `src/save.rs`   | MIGRATED (session 21, 2c28bd03) |
|39 | `sa_def`                  |        2891 | `src/save.rs`   | MIGRATED (session 21, 2c28bd03) |
|40 | `sa_w_def`                |        2908 | `src/save.rs`   | MIGRATED (session 21, 2c28bd03) |
|41 | `gsa_def`                 |        2923 | `src/save.rs`   | MIGRATED (session 21, 2c28bd03) |
|42 | `gsa_w_def`               |        2933 | `src/save.rs`   | MIGRATED (session 21, 2c28bd03) |
|43 | `sa_restore`              |        2942 | `src/save.rs`   | MIGRATED (session 21, 2c28bd03) |
|44 | `unsave`                  |        2814 | `src/save.rs`   | MIGRATED (session 21, ddf8d1cc) |
|45 | `show_cur_cmd_chr`        |        2824 | `src/output.rs` | MIGRATED (session 21, 1a27b4e9) |
|46 | `end_token_list`          |        3108 | `src/input.rs`  | MIGRATED (session 22, d55b8f05) |
|47 | `back_input`              |        3136 | `src/input.rs`  | MIGRATED (session 22, d55b8f05) |
|48 | `back_error`              |        3167 | `src/input.rs`  | MIGRATED (session 22, d55b8f05) |
|49 | `ins_error`               |        3174 | `src/input.rs`  | MIGRATED (session 22, d55b8f05) |
|50 | `new_index`               |        4111 | `src/save.rs`   | MIGRATED (session 22, 250f183b) |
|51 | `find_sa_element`         |        4129 | `src/save.rs`   | MIGRATED (session 22, 250f183b) |
|52 | `new_noad`                |       10542 | `src/mem.rs`    | MIGRATED (session 22, 2334ba03) |
|53 | `new_style`               |       10554 | `src/mem.rs`    | MIGRATED (session 22, 2334ba03) |
|54 | `new_choice`              |       10565 | `src/mem.rs`    | MIGRATED (session 22, 2334ba03) |
|55 | `max_hyphenatable_length` |       11330 | `src/scan.rs`   | MIGRATED (session 22, 4394fae3) |
|56 | `new_margin_kern`         |        9838 | `src/mem.rs`    | MIGRATED (session 22, 5147b292) |
|57 | `add_or_sub`              |        6281 | `src/scaled_math.rs` | MIGRATED (session 23, 3b22036d) |
|58 | `quotient`                |        6305 | `src/scaled_math.rs` | MIGRATED (session 23, 3b22036d) |
|59 | `fract`                   |        6336 | `src/scaled_math.rs` | MIGRATED (session 23, 3b22036d) |
|60 | `change_if_limit`         |        7889 | `src/output.rs` | MIGRATED (session 23, 3b22036d) |
|61 | `scan_normal_glue`        |        6519 | `src/scan.rs`   | MIGRATED (session 23, f239d532) |
|62 | `scan_mu_glue`            |        6524 | `src/scan.rs`   | MIGRATED (session 23, f239d532) |
|63 | `flush_str`               |       15948 | `src/string_pool.rs` | MIGRATED (session 23, a9eac0dc) |
|64 | `pseudo_close`            |        2766 | `src/input.rs`  | MIGRATED (session 23, a58a8268) |
|65 | `get_input_normalization_state` |  9073 | `src/scan.rs`   | MIGRATED (session 23, 2f8134e1) |
|66 | `get_tracing_fonts_state` |        9082 | `src/scan.rs`   | MIGRATED (session 23, 2f8134e1) |
|67 | `scan_pdf_ext_toks`       |       15945 | `src/scan.rs`   | MIGRATED (session 23, 2f8134e1) |
|68 | `begin_file_reading`      |        3097 | `src/input.rs`  | MIGRATED (session 24, 778f6b77) |
|69 | `end_file_reading`        |        3127 | `src/input.rs`  | MIGRATED (session 24, 778f6b77) |
|70 | `pseudo_input`            |        2722 | `src/input.rs`  | MIGRATED (session 24, 061dc82a) |
|71 | `bad_utf8_warning`        |        8970 | `src/output.rs` | MIGRATED (session 24, 4fa969cd) |
|72 | `font_feature_warning`    |        8712 | `src/output.rs` | MIGRATED (session 24, 3802c1cf) |
|73 | `font_mapping_warning`    |        8732 | `src/output.rs` | MIGRATED (session 24, 3802c1cf) |
|74 | `graphite_warning`        |        8764 | `src/output.rs` | MIGRATED (session 24, 3802c1cf) |
|75 | `x_token`                 |        4316 | `src/scan.rs`   | MIGRATED (session 24, 81d5e934) |
|76 | `scan_optional_equals`    |        4355 | `src/scan.rs`   | MIGRATED (session 24, 81d5e934) |
|77 | `scan_left_brace`         |        4319 | `src/scan.rs`   | MIGRATED (session 24, 01c50e13) |
|78 | `norm_min`                |       12014 | `src/helpers.rs`| MIGRATED (session 24, 8440fd8c) |
|79 | `insert_dollar_sign`      |       11444 | `src/scan.rs`   | MIGRATED (session 24, 8440fd8c) |
|80 | `insert_relax`            |        3998 | `src/input.rs`  | MIGRATED (session 24, c6e5204d) |
|81 | `get_token`               |        3653 | `src/scan.rs`   | MIGRATED (session 24, e789d157) |
|82 | `get_x_token`             |        4272 | `src/scan.rs`   | MIGRATED (session 25, 42ba5a71) |
|83 | `get_x_or_protected`      |        4533 | `src/scan.rs`   | MIGRATED (session 25, 42ba5a71) |
|84 | `scan_keyword`            |        4283 | `src/scan.rs`   | MIGRATED (session 26, c8ccdb2d) |
|85 | `scan_rule_spec`          |        6267 | `src/scan.rs`   | MIGRATED (session 27, 96737d98) |
|86 | `scan_dimen`              |        5956 | `src/scan.rs`   | MIGRATED (session 27, 6e9aea72) |
|87 | `scan_decimal`            |        5961 | `src/scan.rs`   | MIGRATED (session 27, 6e9aea72) |
|88 | `scan_spec`               |        9215 | `src/scan.rs`   | MIGRATED (session 27, f614c4f3) |
|89 | `scan_general_text`       |        6262 | `src/scan.rs`   | MIGRATED (session 28, a19aad9a) |
|90 | `scan_expr`               |        6025 | `src/scan.rs`   | MIGRATED (session 29, edba0a63) |
|91 | `find_font_dimen`         |        4501 | `src/scan.rs`   | MIGRATED (session 32/33, bbf2fad1 with globals in e6c0c922) |
|92 | `scan_and_pack_name`      |       13424 | `src/scan.rs`   | MIGRATED (session 33, bbf2fad1) |
|93 | `scan_file_name_braced` (static) | 7967 | `src/scan.rs` | MIGRATED (session 34, 47dd4f05) |
|94 | `scan_file_name`          |        8003 | `src/scan.rs`   | MIGRATED (session 34, 47dd4f05) |
|95 | `pack_job_name`           |        7970 | `src/scan.rs`   | MIGRATED (session 35, b375cb8f) |
|96 | `make_name_string`        |        7928 | `src/scan.rs`   | MIGRATED (session 36, 9347fe3b) |
|97 | `open_log_file`           |        7937 | `src/scan.rs`   | MIGRATED (session 37, da715977) |
|98 | `scan_something_internal` |        4505 | `src/scan.rs`   | MIGRATED (session 38, 9b03ca67) |
|99 | `scan_int`                |        5465 | `src/scan.rs`   | MIGRATED (session 38, 9b03ca67) |

### Phases 12+

Continue `xetex-xetex0.c`: next candidates:
- `conditional` (~7375): `\if` expansion machine; depends on `pass_text` (Rust) and now `scan_something_internal` (Rust).
- `show_save_groups` (~11356): complex multi-goto diagnostic helper.
- `start_input` (~7940): input file opener; depends on `open_log_file` (Rust).

## How to maintain this file

At the end of each session:

1.  Recount remaining LOC / fns with:
    ```
    wc -l xetex/xetex-*.c
    grep -c '^[[:alpha:]_][[:alpha:]0-9_ *]*[[:alpha:]_][[:alpha:]0-9_]*\s*(' xetex/xetex-xetex0.c
    ```
2.  Update the summary table counts.
3.  For any function group you touched, either:
    -   Check off completed functions with a `✓ <commit-hash>` annotation, or
    -   Update the status column of the file row
4.  Bump "Last updated" to match your session number in `SESSION_LOG.md`.
