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
| NOT STARTED    |          1 |
| IN PROGRESS    |          1 |
| DONE           |         14 |
| SKIPPED        |          0 |
| **Total**      |     **16** |

Last updated: session 117.


## Core engine files (to translate)

| C source file                | LOC   | Fns (C) | Rust target module                | Status       |
|------------------------------|------:|--------:|-----------------------------------|--------------|
| xetex-scaledmath.c           |    20 |      0 | `src/scaled_math.rs`              | DONE               |
| xetex-stringpool.c           |    13 |      0 | `src/string_pool.rs`              | DONE               |
| xetex-errors.c               |    12 |      0 | `src/errors.rs`                   | DONE         |
| xetex-output.c               |    15 |      0 | `src/output.rs`                   | DONE               |
| xetex-ini.c                  |  3881 |    ~?? | `src/ini.rs` + state modules      | NOT STARTED  |
| xetex-xetex0.c               |    10 |      0 | multiple (see below)              | DONE         |

| xetex-pagebuilder.c          |    11 |      0 | `src/pagebuilder.rs`              | DONE         |
| xetex-linebreak.c            |    47 |      0 | `src/linebreak.rs`                | DONE |
| xetex-math.c                 |    99 |      0 | `src/math.rs`                     | DONE |
| xetex-shipout.c              |    88 |      0 | `src/shipout.rs`                  | DONE |
| xetex-synctex.c              |    10 |      0 | `src/synctex.rs`                  | DONE         |
| xetex-io.c                   |   101 |      0 | `src/io.rs`                       | DONE               |
| xetex-ext.c                  |    97 |      0 | `src/ext.rs`                      | DONE |
| xetex-pic.c                  |    48 |      0 | `src/pic.rs`                      | DONE         |
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

### Phase 5.5 — linebreak helpers (target: `src/linebreak.rs`)

Status: DONE (session 79).

`xetex-linebreak.c` has 12 function definitions in the starting file:
`get_native_usv`, `line_break`, 8 private helpers, plus `push_node` and
`pop_node`. All compiled bodies are now in Rust. The old `line_break` and
`try_break` bodies are still present under `#if 0` as temporary reference
blocks and should be deleted when convenient.

| # | Function                | Line | Status |
|--:|-------------------------|-----:|--------|
| 1 | `push_node`             | 2827 | MIGRATED (session 75) |
| 2 | `pop_node`              | 2837 | MIGRATED (session 75) |
| 3 | `get_native_usv`        |   77 | MIGRATED (session 76) |
| 4 | `total_pw`              | 2692 | MIGRATED (session 76) |
| 5 | `find_protchar_left`    | 2731 | MIGRATED (session 76) |
| 6 | `find_protchar_right`   | 2778 | MIGRATED (session 76) |
| 7 | `finite_shrink`         | 2377 | MIGRATED (session 76) |
| 8 | `reconstitute`          | 2428 | MIGRATED (session 76) |
| 9 | `hyphenate`             | 2010 | MIGRATED (session 76) |
|10 | `post_line_break`       |  971 | MIGRATED (session 76) |
|11 | `try_break`             |  978 | MIGRATED (session 78) |
|12 | `line_break`            |   90 | MIGRATED (session 79) |

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

Status: DONE (session 73).

| # | Function                  | Line | Status |
|--:|---------------------------|-----:|--------|
| 1 | `tt_xetex_open_input`     |   22 | MIGRATED (session 16, 979cde6d) |
| 2 | `u_open_in`               |  127 | MIGRATED (session 16, 979cde6d) |
| 3 | `make_utf16_name`         |  481 | MIGRATED (session 16, 979cde6d) |
| 4 | `get_uni_c`               |  373 | MIGRATED (session 16, 979cde6d) |
| 5 | `u_close`                 |  357 | MIGRATED (session 73, 25da31e5) — via `xetex_ucnv_close` C wrapper |
| 6 | `set_input_file_encoding` |   87 | MIGRATED (session 73, 725c7a39) — via `xetex_ucnv_open` C wrapper |
| 7 | `buffer_overflow` (static)|  176 | MIGRATED (session 73, 725c7a39) — private Rust fn |
| 8 | `conversion_error` (static)|  183 | MIGRATED (session 73, 725c7a39) — private Rust fn |
| 9 | `apply_normalization` (static)| 195 | MIGRATED (session 73, 725c7a39) — TECkit extern "C", NORMALIZERS static |
|10 | `input_line`              |  220 | MIGRATED (session 73, 725c7a39) — via `xetex_ucnv_to_utf32` C wrapper |
|11 | `open_or_close_in`        |  294 | MIGRATED (session 17, f4a94ffc) |

Note: `name_of_input_file` and `abspath_of_input_file` moved from C file-statics
to Rust `#[no_mangle] pub static mut` in `src/io.rs`. `NAME16LEN`, `BYTE_BUFFER`,
`UTF32_BUF`, and `NORMALIZERS` also moved.
Permanent C helpers (thin ICU wrappers): `xetex_ucnv_close`, `xetex_ucnv_open`,
`xetex_ucnv_to_utf32`. The lookup tables `offsetsFromUTF8`, `bytesFromUTF8`,
`firstByteMark` remain as C `const` arrays (referenced from Rust via `extern "C"`).
xetex-io.c is now 101 LOC (header, lookup tables, 3 ICU wrappers).

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
|100| `conditional`             |        6271 | `src/scan.rs`   | MIGRATED (session 39, cdd6f395) |
|101| `start_input`             |        6431 | `src/scan.rs`   | MIGRATED (session 40, 54064e99) |
|102| `effective_char`          |        4457 | `src/scan.rs`   | MIGRATED (session 41, 223bdfc2) |
|103| `effective_char_info`     |        6434 | `src/scan.rs`   | MIGRATED (session 41, 223bdfc2) |
|104| `eTeX_enabled`            |        8811 | `src/scan.rs`   | MIGRATED (session 41, 3521e31f) |
|105| `print_totals`            |        9282 | `src/output.rs` | MIGRATED (session 41, 3521e31f) |
|106| `you_cant`                |        9326 | `src/scan.rs`   | MIGRATED (session 41, 11e31397) |
|107| `report_illegal_case`     |        9334 | `src/scan.rs`   | MIGRATED (session 41, 11e31397) |
|108| `privileged`              |        9347 | `src/scan.rs`   | MIGRATED (session 41, 11e31397) |
|109| `normal_paragraph`        |        9496 | `src/scan.rs`   | MIGRATED (session 41, 862f5198) |
|110| `give_err_help`           |       13499 | `src/errors.rs` | MIGRATED (session 41, e8430054) |
|111| `new_whatsit`             |       13763 | `src/mem.rs`    | MIGRATED (session 41, e8430054) |
|112| `end_graf`                |        9942 | `src/scan.rs`   | MIGRATED (session 41, 4af3b2b8) |
|113| `append_penalty`          |       10016 | `src/scan.rs`   | MIGRATED (session 41, 4af3b2b8) |
|114| `align_error`             |       10390 | `src/scan.rs`   | MIGRATED (session 41, 5f06879e) |
|115| `no_align_error`          |       10443 | `src/scan.rs`   | MIGRATED (session 41, 5f06879e) |
|116| `omit_error`              |       10457 | `src/scan.rs`   | MIGRATED (session 41, 5f06879e) |
|117| `cs_error`                |       10489 | `src/scan.rs`   | MIGRATED (session 41, 5f06879e) |
|118| `push_math`               |       10416 | `src/scan.rs`   | MIGRATED (session 41, 650e6dd0) |
|119| `box_error`               |        9270 | `src/scan.rs`   | MIGRATED (session 42, b2c2f894) |
|120| `its_all_over`            |        9317 | `src/scan.rs`   | MIGRATED (session 42, b2c2f894) |
|121| `append_glue`             |        9358 | `src/scan.rs`   | MIGRATED (session 42, b2c2f894) |
|122| `append_kern`             |        9395 | `src/scan.rs`   | MIGRATED (session 42, b2c2f894) |
|123| `make_mark`               |        9896 | `src/scan.rs`   | MIGRATED (session 42, 1669e578) |
|124| `new_interaction`         |       11040 | `src/scan.rs`   | MIGRATED (session 42, 1669e578) |
|125| `alter_aux`               |       10774 | `src/scan.rs`   | MIGRATED (session 42, 49b91f62) |
|126| `alter_prev_graf`         |       10805 | `src/scan.rs`   | MIGRATED (session 42, 49b91f62) |
|127| `alter_page_so_far`       |       10837 | `src/scan.rs`   | MIGRATED (session 42, 49b91f62) |
|128| `alter_integer`           |       10846 | `src/scan.rs`   | MIGRATED (session 42, 49b91f62) |
|129| `alter_box_dimen`         |       10874 | `src/scan.rs`   | MIGRATED (session 42, 49b91f62) |
|130| `trap_zero_glue`          |       10570 | `src/scan.rs`   | MIGRATED (session 42, 94b086c4) |
|131| `fix_language`            |       11357 | `src/scan.rs`   | MIGRATED (session 42, 94b086c4) |
|132| `append_to_vlist`         |        8020 | `src/scan.rs`   | MIGRATED (session 45, c2b751b9) |
|133| `align_peek`              |        8785 | `src/scan.rs`   | MIGRATED (session 45, c2b751b9) |
|134| `show_whatever`           |       10561 | `src/scan.rs`   | MIGRATED (session 45, b8e223bc) |
|135| `do_register_command`     |       10220 | `src/scan.rs`   | MIGRATED (session 45, 177ee736) |
|136| `new_font`                |       10233 | `src/scan.rs`   | MIGRATED (session 45, e44c3dce) |
|137| `make_accent`             |        9910 | `src/scan.rs`   | MIGRATED (session 46, 3a0adf77) |
|138| `append_italic_correction`|        9868 | `src/scan.rs`   | MIGRATED (session 46, 53f45e8b) |
|139| `delete_last`             |        9716 | `src/scan.rs`   | MIGRATED (session 46, e5313fe2) |
|140| `unpackage`               |        9799 | `src/scan.rs`   | MIGRATED (session 46, e5313fe2) |
|141| `char_warning`            |        6428 | `src/scan.rs`   | MIGRATED (session 47, a7098003) |
|142| `app_space`               |        9223 | `src/scan.rs`   | MIGRATED (session 47, a7098003) |
|143| `new_graf`                |        9549 | `src/scan.rs`   | MIGRATED (session 47, 6732bacf) |
|144| `package`                 |        9509 | `src/scan.rs`   | MIGRATED (session 47, c283130c) |
|145| `insert_src_special`      |        9978 | `src/scan.rs`   | MIGRATED (session 47, 2ee99a3c) |
|146| `scan_box`                |        9485 | `src/scan.rs`   | MIGRATED (session 47, 5ffb71e6) |
|147| `tt_insert_special`       |        9984 | `src/scan.rs`   | MIGRATED (session 47, 9e355d5b) |
|148| `pack_file_name`          |        6381 | `src/scan.rs`   | MIGRATED (session 47, feb6bd59) |
|149| `scan_math`               |        4313 | `src/scan.rs`   | MIGRATED (session 48, d90d1986) |
|150| `set_math_char`           |        4315 | `src/scan.rs`   | MIGRATED (session 48, 38f8512d) |
|151| `new_character`           |        7048 | `src/scan.rs`   | MIGRATED (session 48, e05d388f) |
|152| `scan_glue`               |        4678 | `src/scan.rs`   | MIGRATED (session 48, 04720330) |
|153| `push_alignment`          |        7724 | `src/scan.rs`   | MIGRATED (session 48, 57961939) |
|154| `pop_alignment`           |        7743 | `src/scan.rs`   | MIGRATED (session 48, 57961939) |
|155| `init_span`               |        7880 | `src/scan.rs`   | MIGRATED (session 48, 300e358f) |
|156| `init_row`                |        7893 | `src/scan.rs`   | MIGRATED (session 48, 300e358f) |
|157| `init_col`                |        7912 | `src/scan.rs`   | MIGRATED (session 48, 300e358f) |
|158| `get_preamble_token`      |        7728 | `src/scan.rs`   | MIGRATED (session 49, 236b6293) |
|159| `fin_col`                 |        7859 | `src/scan.rs`   | MIGRATED (session 50, 724e76bb) |
|160| `fin_row`                 |        8009 | `src/scan.rs`   | MIGRATED (session 50, 724e76bb) |
|161| `init_align`              |        7732 | `src/scan.rs`   | MIGRATED (session 50, 724e76bb) |
|162| `fin_align`               |        8040 | `src/scan.rs`   | MIGRATED (session 51, 97f42b9a) |
|163| `show_save_groups`        |        7752 | `src/scan.rs`   | MIGRATED (session 52, 0963a923) |
|164| `do_assignments`          |       10745 | `src/scan.rs`   | MIGRATED (session 52, 04e4888a) |
|165| `vert_break`              |        7725 | `src/scan.rs`   | MIGRATED (session 53, 0d1c6960) |
|166| `vsplit`                  |        7727 | `src/scan.rs`   | MIGRATED (session 53, 9c2a6546) |
|167| `box_end`                 |        7797 | `src/scan.rs`   | MIGRATED (session 53, 32d30dd9) |
|168| `begin_box`               |        7794 | `src/scan.rs`   | MIGRATED (session 53, bf5b0f45) |
|169| `just_copy`               |        8029 | `src/scan.rs`   | MIGRATED (session 54, a6f7231b) |
|170| `just_reverse`            |        8155 | `src/scan.rs`   | MIGRATED (session 54, a6f7231b) |
|171| `handle_right_brace`      |        8265 | `src/scan.rs`   | MIGRATED (session 54, b6d9eca8) |
|172| `char_pw`                 |        6968 | `src/scan.rs`   | MIGRATED (session 54, 978de7d8) |
|173| `close_files_and_terminate` |      9722 | `src/scan.rs`   | MIGRATED (session 55, 5830f81c) |
|174| `prune_page_top`          |        9761 | `src/scan.rs`   | MIGRATED (session 55, 5830f81c) |
|175| `do_marks`                |        9817 | `src/scan.rs`   | MIGRATED (session 55, 5830f81c) |
|176| `do_extension`            |        8024 | `src/scan.rs`   | MIGRATED (session 55, bc4e9176) |
|177| `conv_toks`               |        5162 | `src/scan.rs`   | MIGRATED (session 57, 719c7946) |
|178| `begin_token_list`        |        2970 | `src/input.rs`  | MIGRATED (session 57, 73ff1645) |
|179| `flush_node_list`         |         754 | `src/scan.rs`   | MIGRATED (session 58, 9dbfdea7) |
|180| `copy_node_list`          |         954 | `src/scan.rs`   | MIGRATED (session 58, 0fc07566) |
|181| `show_activities`         |        1123 | `src/scan.rs`   | MIGRATED (session 58, 947f0a58) |
|182| `print_subsidiary_data`   |         215 | `src/scan.rs`   | MIGRATED (session 58, a5e002a6) |
|183| `show_node_list`          |         259 | `src/scan.rs`   | MIGRATED (session 58, a5e002a6) |
|184| `show_context`            |        2752 | `src/scan.rs`   | MIGRATED (session 59, 64464240) |
|185| `expand`                  |        3988 | `src/scan.rs`   | MIGRATED (session 60, b01234a8) |
|186| `get_next`                |        3152 | `src/scan.rs`   | MIGRATED (session 61, 97dbabde) |
|187| `macro_call`              |        3648 | `src/scan.rs`   | MIGRATED (session 61, 6c98b044) |
|188| `scan_glyph_number`       |        6409 | `src/scan.rs`   | MIGRATED (session 62, 7058f857) |
|189| `round_decimals`          |        6442 | `src/scan.rs`   | MIGRATED (session 62, 7058f857) |
|190| `xetex_scan_dimen`        |        6461 | `src/scan.rs`   | MIGRATED (session 62, 7058f857) |
|191| `print_cmd_chr`           |        1253 | `src/output.rs` | MIGRATED (session 63, 5184bb4d) |
|192| `new_native_word_node`    |        6250 | `src/scan.rs`   | MIGRATED (session 63, 64b6b451) |
|193| `do_locale_linebreaks`    |        6493 | `src/scan.rs`   | MIGRATED (session 63, 64b6b451) |
|194| `new_native_character`    |        6276 | `src/scan.rs`   | MIGRATED (session 64, ea82f5de) |
|195| `load_native_font`        |        6284 | `src/scan.rs`   | MIGRATED (session 65, bd70d52b) |
|196| `read_font_info`          |        6350 | `src/scan.rs`   | MIGRATED (session 66, 78da31d7) |

### Phase 12 — xetex-pic.c (target: `src/pic.rs`)

Status: DONE (session 80).

| # | Function                   | Line | Status |
|--:|----------------------------|-----:|--------|
| 1 | `count_pdf_file_pages`     |   76 | MIGRATED (session 80, cb2b9ec9) |
| 2 | `pdf_get_rect` (static)    |   99 | MIGRATED (session 80, cb2b9ec9 — private Rust fn) |
| 3 | `get_image_size_in_inches` (static) | 191 | MIGRATED (session 80, cb2b9ec9 — private Rust fn) |
| 4 | `find_pic_file` (static)   |  224 | MIGRATED (session 80, cb2b9ec9 — private Rust fn) |
| 5 | `transform_point` (static) |  258 | MIGRATED (session 80, cb2b9ec9 — private Rust fn) |
| 6 | `make_identity` (static)   |  270 | MIGRATED (session 80, cb2b9ec9 — private Rust fn) |
| 7 | `make_scale` (static)      |  282 | MIGRATED (session 80, cb2b9ec9 — private Rust fn) |
| 8 | `make_translation` (static)|  294 | MIGRATED (session 80, cb2b9ec9 — private Rust fn) |
| 9 | `make_rotation` (static)   |  306 | MIGRATED (session 80, cb2b9ec9 — private Rust fn) |
|10 | `transform_concat` (static)|  318 | MIGRATED (session 80, cb2b9ec9 — private Rust fn) |
|11 | `load_picture`             |  336 | MIGRATED (session 80, cb2b9ec9) |

### Phase 13 — xetex-ext.c helpers (target: `src/ext.rs`)

Status: DONE (session 90).

| # | Function      | Line | Status |
|--:|---------------|-----:|--------|
| 1 | `read_double` |  279 | MIGRATED (session 82) |
| 2 | `read_rgb_a`  |  344 | MIGRATED (session 82) |
| 3 | `D2Fix`       | 1882 | MIGRATED (session 82) |
| 4 | `Fix2D`       | 1888 | MIGRATED (session 82) |
| 5 | `print_utf8_str` | 173 | MIGRATED (session 83) |
| 6 | `print_chars` | 180 | MIGRATED (session 83) |
| 7 | `load_mapping_file` (static) | 187 | MIGRATED (session 83 — private Rust fn) |
| 8 | `check_for_tfm_font_mapping` | 236 | MIGRATED (session 83) |
| 9 | `load_tfm_font_mapping` | 251 | MIGRATED (session 83) |
|10 | `apply_tfm_font_mapping` | 263 | MIGRATED (session 83) |
|11 | `readCommonFeatures` | 310 | MIGRATED (session 83) |
|12 | `get_encoding_mode_and_info` | 127 | MIGRATED (session 84) |
|13 | `read_tag_with_param` (static) | 173 | MIGRATED (session 84) |
|14 | `readFeatureNumber` (static) | 204 | MIGRATED (session 84) |
|15 | `splitFontName` (static) | 440 | MIGRATED (session 84) |
|16 | `release_font_engine` | 503 | MIGRATED (session 85) |
|17 | `ot_get_font_metrics` | 517 | MIGRATED (session 85) |
|18 | `ot_font_get` | 560 | MIGRATED (session 85) |
|19 | `ot_font_get_1` | 580 | MIGRATED (session 85) |
|20 | `ot_font_get_2` | 608 | MIGRATED (session 85) |
|21 | `ot_font_get_3` | 636 | MIGRATED (session 85) |
|22 | `gr_print_font_name` | 651 | MIGRATED (session 85) |
|23 | `gr_font_get_named` | 670 | MIGRATED (session 85) |
|24 | `gr_font_get_named_1` | 683 | MIGRATED (session 85) |
|25 | `snap_zone` (static) | 777 | MIGRATED (session 86, bf560133 — private Rust fn) |
|26 | `get_native_char_height_depth` | 785 | MIGRATED (session 86, bf560133) |
|27 | `getnativecharht` | 822 | MIGRATED (session 86, bf560133) |
|28 | `getnativechardp` | 830 | MIGRATED (session 86, bf560133) |
|29 | `get_native_char_sidebearings` | 838 | MIGRATED (session 86, bf560133) |
|30 | `get_glyph_bounds` | 862 | MIGRATED (session 86, bf560133) |
|31 | `getnativecharic` | 889 | MIGRATED (session 86, bf560133) |
|32 | `getnativecharwd` | 900 | MIGRATED (session 86, bf560133) |
|33 | `map_char_to_glyph` | 1261 | MIGRATED (session 86, bf560133) |
|34 | `map_glyph_to_index` | 1278 | MIGRATED (session 86, bf560133) |
|35 | `get_font_char_range` | 1293 | MIGRATED (session 86, bf560133) |
|36 | `print_glyph_name` | 1538 | MIGRATED (session 86, bf560133) |
|37 | `real_get_native_glyph` | 921 | MIGRATED (session 87, 3ff25d07) |
|38 | `real_get_native_italic_correction` | 1039 | MIGRATED (session 87, 3ff25d07) |
|39 | `real_get_native_glyph_italic_correction` | 1063 | MIGRATED (session 87, 3ff25d07) |
|40 | `measure_native_glyph` | 1080 | MIGRATED (session 87, 3ff25d07) |
|41 | `real_get_native_word_cp` | 1347 | MIGRATED (session 87, 3ff25d07) |
|42 | `linebreak_start` | 1558 | MIGRATED (session 88, 932d70f1) |
|43 | `linebreak_next` | 1612 | MIGRATED (session 88, 932d70f1) |
|44 | `makeXDVGlyphArrayData` | 1641 | MIGRATED (session 88, 932d70f1) |
|45 | `make_font_def` | 1714 | MIGRATED (session 88, 932d70f1) |
|46 | `apply_mapping` | 1804 | MIGRATED (session 88, 932d70f1) |
|47 | `store_justified_native_glyphs` | 1845 | MIGRATED (session 88, 932d70f1) |
|48 | `aat_get_font_metrics` | 1876 | MIGRATED (session 88, 932d70f1 — no-op on Linux) |
|49 | `aat_font_get` | 1892 | MIGRATED (session 88, 932d70f1 — returns -1 on Linux) |
|50 | `aat_font_get_1` | 1901 | MIGRATED (session 88, 932d70f1 — returns -1 on Linux) |
|51 | `aat_font_get_2` | 1912 | MIGRATED (session 88, 932d70f1 — returns -1 on Linux) |
|52 | `aat_font_get_named` | 1925 | MIGRATED (session 88, 932d70f1 — returns -1 on Linux) |
|53 | `aat_font_get_named_1` | 1936 | MIGRATED (session 88, 932d70f1 — returns -1 on Linux) |
|54 | `aat_print_font_name` | 1949 | MIGRATED (session 88, 932d70f1 — no-op on Linux) |
|55 | `loadOTfont` (static) | 72 | MIGRATED (session 89, 89869eaa — private Rust fn) |
|56 | `find_native_font` | 283 | MIGRATED (session 89, 89869eaa) |
|57 | `measure_native_node` | 98 | MIGRATED (session 90, 9acfe915) |

Session 88 also promoted `brkIter` → `BRK_ITER`, `brkLocaleStrNum` → `BRK_LOCALE_STR_NUM`,
`xdvBufSize` → `XDV_BUF_SIZE` file-statics to Rust. Added ICU version detection in build.rs
(reads `uvernum.h` → `ICU_VERSION_SUFFIX` env var) to resolve version-mangled symbols via
`#[link_name = concat!("ubrk_open", env!("ICU_VERSION_SUFFIX"))]`.

Session 89 added setters for `loaded_font_letter_space` and `native_font_type_flag`, translated
the Linux OpenType/Graphite native-font loader path (`loadOTfont` as private Rust helper plus
exported `find_native_font`).

Session 90 translated `measure_native_node`, including ICU BiDi runs, layout result copying,
letterspacing, and glyph-bbox height/depth calculation. `xetex-ext.c` now has no remaining
Linux-compiled C function bodies; only the platform-guarded macOS helper remains out of the
Linux migration scope.

Session 83 also moved the former `saved_mapping_name` file-static into
Rust as `SAVED_MAPPING_NAME`.

### Phase 14 — xetex-shipout.c helpers (target: `src/shipout.rs`)

Status: DONE (session 117).

| # | Function   | Line | Status |
|--:|------------|-----:|--------|
| 1 | `new_edge` | 1708 | MIGRATED (session 91, 3ecb7ae8) |
| 2 | `dvi_four` | 2403 | MIGRATED (session 109, 55c6d9e9) |
| 3 | `dvi_two`  | 2423 | MIGRATED (session 109, 55c6d9e9) |
| 4 | `dvi_native_font_def` | 1784 | MIGRATED (session 110, 0fe52f3a; private Rust helper) |
| 5 | `dvi_font_def` | 1798 | MIGRATED (session 110, 0fe52f3a) |
| 6 | `initialize_shipout_variables` | 48 | MIGRATED (session 111, 9a26ee88) |
| 7 | `deinitialize_shipout_variables` | 63 | MIGRATED (session 111, 9a26ee88) |
| 8 | `write_to_dvi` | 2296 | MIGRATED (session 111, 9a26ee88; kept #[no_mangle] for finalize_dvi_file; can be made private once no C callers remain) |
| 9 | `dvi_swap` | 2306 | MIGRATED (session 111, 9a26ee88; private Rust helper) |
| 10 | `dvi_out` | 71 | MIGRATED (session 111, 9a26ee88) |
| 11 | `dvi_pop` | 2332 | MIGRATED (session 111, 9a26ee88; un-static'd so C callers link to Rust) |
| 12 | `prune_movements` | 1930 | MIGRATED (session 112, 1819fba9; #[no_mangle] — called by hlist_out/vlist_out still in C) |
| 13 | `finalize_dvi_file` | 2173 | MIGRATED (session 112, 1819fba9; write_to_dvi no longer needs #[no_mangle]) |
| 14 | `out_what` | 1682 | MIGRATED (session 113, fc907f6a) |
| 15 | `movement` | 1763 | MIGRATED (session 113, fc907f6a; #[no_mangle] — called by hlist_out/vlist_out still in C) |
| 16 | `special_out` | 1930 | MIGRATED (session 113, fc907f6a; private Rust helper) |
| 17 | `write_out` | 1976 | MIGRATED (session 113, fc907f6a; private Rust helper) |
| 18 | `pic_out` | 2071 | MIGRATED (session 113, fc907f6a; #[no_mangle] — called by hlist_out still in C) |
| 19 | `reverse` | 1470 | MIGRATED (session 114, 0d3b8461; #[no_mangle] — called by hlist_out still in C) |
| 20 | `ship_out` | 56 | MIGRATED (session 115, 31345c81; promoted `dvi_f`; un-static'd `hlist_out`/`vlist_out` for Rust caller) |
| 21 | `vlist_out` | 866 | MIGRATED (session 116, f4cc04d5; C `hlist_out` now calls Rust `vlist_out`, and Rust `vlist_out` calls C `hlist_out`) |
| 22 | `hlist_out` | 63 | MIGRATED (session 117, f05b105f; final compiled shipout body moved to Rust) |

### Phase 15 — xetex-math.c helpers (target: `src/math.rs`)

Status: DONE (session 108).

| # | Function            | Line | Status |
|--:|---------------------|-----:|--------|
| 1 | `start_eq_no`       | 354 | MIGRATED (session 92, 7e81f4b6) |
| 2 | `math_limit_switch` | 368 | MIGRATED (session 92, 7e81f4b6) |
| 3 | `scan_delimiter`    | 388 | MIGRATED (session 92, 7e81f4b6; promoted from C static) |
| 4 | `math_radical`      | 461 | MIGRATED (session 92, 7e81f4b6) |
| 5 | `append_choices`    | 533 | MIGRATED (session 92, 7e81f4b6) |
| 6 | `fin_mlist`         | 545 | MIGRATED (session 92, 7e81f4b6) |
| 7 | `build_choices`     | 571 | MIGRATED (session 92, 7e81f4b6) |
| 8 | `sub_sup`           | 599 | MIGRATED (session 92, 7e81f4b6) |
| 9 | `math_fraction`     | 644 | MIGRATED (session 92, 7e81f4b6) |
|10 | `math_left_right`   | 703 | MIGRATED (session 92, 7e81f4b6) |
|11 | `math_ac`           | 355 | MIGRATED (1d724580) |
|12 | `resume_after_display` | 786 | MIGRATED (7f29297a) |
|13 | `init_math`         | 81 | MIGRATED (session 93, 133c1ac0) |
|14 | `app_display`       | 82 | MIGRATED (session 94, 49f0aee7; private Rust helper) |
|15 | `after_math`        | 923 | MIGRATED (session 94, 49f0aee7) |
|16 | `math_x_height`     | 86 | MIGRATED (session 95, 65eea6ce) |
|17 | `math_quad`         | 101 | MIGRATED (session 95, 65eea6ce) |
|18 | `num1`              | 115 | MIGRATED (session 95, 65eea6ce) |
|19 | `num2`              | 130 | MIGRATED (session 95, 65eea6ce) |
|20 | `num3`              | 145 | MIGRATED (session 95, 65eea6ce) |
|21 | `denom1`            | 160 | MIGRATED (session 95, 65eea6ce) |
|22 | `denom2`            | 175 | MIGRATED (session 95, 65eea6ce) |
|23 | `sup1`              | 190 | MIGRATED (session 95, 65eea6ce) |
|24 | `sup2`              | 205 | MIGRATED (session 95, 65eea6ce) |
|25 | `sup3`              | 219 | MIGRATED (session 95, 65eea6ce) |
|26 | `sub1`              | 233 | MIGRATED (session 95, 65eea6ce) |
|27 | `sub2`              | 247 | MIGRATED (session 95, 65eea6ce) |
|28 | `sup_drop`          | 261 | MIGRATED (session 95, 65eea6ce) |
|29 | `sub_drop`          | 275 | MIGRATED (session 95, 65eea6ce) |
|30 | `delim1`            | 289 | MIGRATED (session 95, 65eea6ce) |
|31 | `delim2`            | 303 | MIGRATED (session 95, 65eea6ce) |
|32 | `axis_height`       | 317 | MIGRATED (session 95, 65eea6ce) |
|33 | `default_rule_thickness` | 88 | MIGRATED (session 96, 0c80ba9e) |
|34 | `big_op_spacing1`   | 102 | MIGRATED (session 96, 0c80ba9e) |
|35 | `big_op_spacing2`   | 116 | MIGRATED (session 96, 0c80ba9e) |
|36 | `big_op_spacing3`   | 130 | MIGRATED (session 96, 0c80ba9e) |
|37 | `big_op_spacing4`   | 144 | MIGRATED (session 96, 0c80ba9e) |
|38 | `big_op_spacing5`   | 158 | MIGRATED (session 96, 0c80ba9e) |
|39 | `fraction_rule`     | 170 | MIGRATED (session 96, 0c80ba9e) |
|40 | `overbar`           | 181 | MIGRATED (session 96, 0c80ba9e) |
|41 | `math_glue`         | 195 | MIGRATED (session 96, 0c80ba9e) |
|42 | `math_kern`         | 223 | MIGRATED (session 96, 0c80ba9e) |
|43 | `flush_math`        | 80 | MIGRATED (session 97, 47196355) |
|44 | `clean_box`         | 91 | MIGRATED (session 97, 47196355) |
|45 | `make_over`         | 198 | MIGRATED (session 97, 47196355) |
|46 | `make_under`        | 207 | MIGRATED (session 97, 47196355) |
|47 | `make_vcenter`      | 225 | MIGRATED (session 97, 47196355) |
|48 | `fetch`             | 115 | MIGRATED (session 98, 7b62b15b; promoted from C static) |
|49 | `compute_ot_math_accent_pos` | 199 | MIGRATED (session 98, 7b62b15b; promoted from C static) |
|50 | `make_radical`      | 80 | MIGRATED (session 99, ca405061; promoted from C static) |
|51 | `make_fraction`     | 313 | MIGRATED (session 99, ca405061; promoted from C static) |
|52 | `make_op`           | 270 | MIGRATED (session 100, 93c786d8; promoted from C static) |
|53 | `make_ord`          | 272 | MIGRATED (session 101, 99e00684; promoted from C static) |
|54 | `attach_hkern_to_new_hlist` | 275 | MIGRATED (session 102, e27b5336; promoted from C static) |
|55 | `make_left_right`   | 759 | MIGRATED (session 102, e27b5336; promoted from C static) |
|56 | `char_box`          | 1432 | MIGRATED (session 102, e27b5336; promoted from C static) |
|57 | `stack_into_box`    | 1463 | MIGRATED (session 102, e27b5336; promoted from C static) |
|58 | `height_plus_depth` | 1474 | MIGRATED (session 102, e27b5336; promoted from C static) |
|59 | `stack_glyph_into_box` | 1482 | MIGRATED (session 102, e27b5336; promoted from C static) |
|60 | `stack_glue_into_box` | 1517 | MIGRATED (session 102, e27b5336; promoted from C static) |
|61 | `make_scripts`     | 84 | MIGRATED (session 107, 8b15e714; promoted from C static) |
|62 | `mlist_to_hlist`   | 84 | MIGRATED (session 108, 4ee11a58) |

Remaining in C: none. `xetex-math.c` now has no remaining compiled C
bodies; only prototypes/globals/reference comments remain.

The next large candidate is `xetex-ini.c`; `xetex-shipout.c` is now done.

### Phases 16+

Continue `xetex-xetex0.c`: remaining candidates:
- `main_control`: huge main engine loop.

Session 67 carved out the `main_control` `append_normal_space` label
(xetex-xetex0.c:8221) into `src/scan.rs` as
`main_control_append_normal_space` (e9988a37). This is a partial
`main_control` migration only; the top-level dispatcher remains in C.

Session 68 carved out five `main_control` switch-arm clusters into
`src/scan.rs`: mode-independent commands, forbidden/math-mode mismatch
errors, simple list/group cases, box/list construction cases, and
alignment/endcsname cases. The C dispatcher still owns the loop and
control-flow labels, but these clusters now funnel through Rust helpers
with the same common `break`/`big_switch` continuation. The session also
kept the useful pre-existing `src/globals.rs` main-loop accessor work and
added the `after_token` binding needed by `\afterassignment`.

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
