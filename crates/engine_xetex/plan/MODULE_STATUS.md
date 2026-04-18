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
| NOT STARTED    |          9 |
| IN PROGRESS    |          3 |
| DONE           |          4 |
| SKIPPED        |          0 |
| **Total**      |     **16** |

Last updated: session 14.

## Core engine files (to translate)

| C source file                | LOC   | Fns (C) | Rust target module                | Status       |
|------------------------------|------:|--------:|-----------------------------------|--------------|
| xetex-scaledmath.c           |    20 |      0 | `src/scaled_math.rs`              | DONE               |
| xetex-stringpool.c           |    13 |      0 | `src/string_pool.rs`              | DONE               |
| xetex-errors.c               |    12 |      0 | `src/errors.rs`                   | DONE         |
| xetex-output.c               |    15 |      0 | `src/output.rs`                   | DONE               |
| xetex-ini.c                  |  3881 |    ~?? | `src/ini.rs` + state modules      | NOT STARTED  |
| xetex-xetex0.c               | 18387 |    268 | multiple (see below)              | IN PROGRESS — 6%  |
| xetex-pagebuilder.c          |   804 |    ~?? | `src/pagebuilder.rs`              | NOT STARTED  |
| xetex-linebreak.c            |  2843 |    ~?? | `src/linebreak.rs`                | NOT STARTED  |
| xetex-math.c                 |  3833 |    ~?? | `src/math.rs`                     | NOT STARTED  |
| xetex-shipout.c              |  2451 |    ~?? | `src/shipout.rs`                  | NOT STARTED  |
| xetex-synctex.c              |  1283 |    ~?? | `src/synctex.rs`                  | NOT STARTED  |
| xetex-io.c                   |   555 |    ~?? | `src/io.rs`                       | NOT STARTED  |
| xetex-ext.c                  |  2170 |    ~?? | `src/ext.rs`                      | NOT STARTED  |
| xetex-pic.c                  |   664 |    ~?? | `src/pic.rs`                      | NOT STARTED  |
| xetex-texmfmp.c              |   302 |      8 | `src/texmfmp.rs`                  | IN PROGRESS — 38% |
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

Remaining texmfmp.c functions (8 in C):
- `makepdftime` (static, complex timezone `struct tm` math) + `init_start_time` + `getcreationdate` — cluster around C-static `start_time_str`; defer.
- `getfilemoddate`, `getfilesize`, `getfiledump` — call `gettexstring` + bridge file I/O; straightforward next targets.
- `compare_paths` (static) + `is_new_source` + `remember_source_info` + `make_src_special` — `is_new_source`/`remember_source_info` use C-static `last_source_name`; the rest are straightforward.

### Phases 9+

Continue xetex-texmfmp.c, then tackle larger files.

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
