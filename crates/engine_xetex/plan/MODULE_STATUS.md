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
| NOT STARTED    |         14 |
| IN PROGRESS    |          2 |
| DONE           |          0 |
| SKIPPED        |          0 |
| **Total**      |     **16** |

Last updated: session 5.

## Core engine files (to translate)

| C source file                | LOC   | Fns (C) | Rust target module                | Status       |
|------------------------------|------:|--------:|-----------------------------------|--------------|
| xetex-scaledmath.c           |    66 |      1 | `src/scaled_math.rs`              | IN PROGRESS — 93%  |
| xetex-stringpool.c           |   206 |      8 | `src/string_pool.rs`              | NOT STARTED  |
| xetex-errors.c               |   175 |      7 | `src/errors.rs`                   | NOT STARTED  |
| xetex-output.c               |   768 |     30 | `src/output.rs`                   | NOT STARTED  |
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
| xetex-texmfmp.c              |   467 |    ~?? | `src/texmfmp.rs`                  | NOT STARTED  |
| xetex-engine-interface.c     |   112 |    ~?? | `src/engine_interface.rs`         | NOT STARTED  |

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
| 9 | `m_log`               |  287 | extern          | diagnostic+help globals   | BLOCKED on Phase 3 |
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

Status: not started. Functions `print_*`, `show_*`.

### Phases 4+

Populate when Phase 1–3 are done. Don't speculate.

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
