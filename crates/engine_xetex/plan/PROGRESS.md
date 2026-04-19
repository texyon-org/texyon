# Progress dashboard

Live metrics for the C → Rust migration. Updated at the end of every
session in Step 8 of `06-workflow.md`.

## Test parity

The authoritative parity command (run from the repository root):

```
cargo test -p tectonic --test tex-outputs
```

| Session | Date       | Passed | Total | Delta | Notes                              |
|--------:|------------|-------:|------:|------:|------------------------------------|
|       0 | 2026-04-15 |      — |     — |     — | Plan scaffolding only. Baseline to be recorded in session 1. |
|       1 | 2026-04-16 |     29 |    29 |     — | Baseline recorded. See `plan/parity-history.log`. |
|       2 | 2026-04-16 |     29 |    29 |     0 | Translated 7 more scaledmath functions. |
|       3 | 2026-04-16 |     29 |    29 |     0 | badness, get_avail, flush_list, get_node, free_node. |
|       4 | 2026-04-16 |     29 |    29 |     0 | new_null_box, new_rule, new_ligature, new_lig_item, new_disc, new_math, new_spec, new_param_glue, new_glue, new_skip_param, new_kern, new_penalty. |
|       5 | 2026-04-16 |     29 |    29 |     0 | copy_native_glyph_info. |
|       6 | 2026-04-16 |     29 |    29 |     0 | Phase 3 start: print_int, print_hex, print_ucs_code, print_the_digs (private helper). |
|       7 | 2026-04-17 |     29 |    29 |     0 | Phase 3: print_cstr, print_nl_cstr, print_esc_cstr. |
|       8 | 2026-04-17 |     29 |    29 |     0 | Phase 3: print_nl, print_esc, print_two, print_roman_int, print_scaled, warn_char, print_ln, print_raw_char, print_char, print, print_current_string. |
|       9 | 2026-04-18 |     29 |    29 |     0 | Phase 3 complete: capture_to_diagnostic, diagnostic_begin_capture_warning_here, error_here_with_diagnostic, print_file_line, print_size, print_write_whatsit, print_file_name, print_cs, sprint_cs, print_native_word, print_sa_num + private helper. xetex-output.c fully translated. |
|      10 | 2026-04-18 |     29 |    29 |     0 | Phase 1 complete: m_log (last scaledmath fn). xetex-stringpool.c fully translated (8 fns). Two more C files emptied. |
|      11 | 2026-04-18 |     29 |    29 |     0 | xetex-errors.c fully translated: error, fatal_error, overflow, confusion, pdf_error + 2 private helpers. |
|      12 | 2026-04-18 |     29 |    29 |     0 | Phase 6: tt_xetex_set_int_variable, tt_xetex_set_string_variable (2 fns stay in C). Back-filled clippy lints in output.rs and scaled_math.rs. |
|      13 | 2026-04-18 |     29 |    29 |     0 | Phase 7 start: get_date_and_time, get_seconds_and_micros (pure libc, no engine globals). |
|      14 | 2026-04-18 |     29 |    29 |     0 | Phase 8: gettexstring, maketexstring (UTF-8↔UTF-16 pool), checkpool_pointer, convertStringToHexString, getmd5sum. |
|      15 | 2026-04-18 |     29 |    29 |     0 | Phase 9 complete: xetex-texmfmp.c fully translated (make_src_special, getfilesize, getfiledump, is_new_source cluster, makepdftime cluster). |
|      16 | 2026-04-18 |     29 |    29 |     0 | Phase 10: xetex-io.c — tt_xetex_open_input, u_open_in, make_utf16_name, get_uni_c. UFile struct defined in Rust. |
|      17 | 2026-04-18 |     29 |    29 |     0 | Phase 11: open_or_close_in, delete_token_ref, delete_glue_ref, begin/end_diagnostic, print_mode/in_mode, print_group, new_save_level, eq_save, eq_word_define, geq_word_define, save_for_after, int_error. New src/save.rs. |
|      18 | 2026-04-18 |     29 |    29 |     0 | Phase 12: prepare_mag, push/pop_nest, eq_destroy/define/geq_define, prev_rightmost, get_microinterval, token_show, print_skip/param/length_param, print_style/fam_and_char/delimiter/rule_dimen/glue/spec, not_*_font_error. |
|      19 | 2026-04-18 |     29 |    29 |     0 | Phase 11+: print_font_and_char, print_mark, print_meaning, group_warning, if_warning, file_warning. New globals: font_max, cur_cmd, cur_mark, input_stack/ptr, base_ptr, grp_stack, if_stack, cond_ptr, if_limit, cur_if, if_line. |
|      20 | 2026-04-19 |     29 |    29 |     0 | id_lookup, prim_lookup (new src/hash.rs); mu_error + 15 scan-validation helpers (new src/scan.rs). |


**Baseline rule:** session 1 must run the test suite *before* making
any Rust changes and record the (Passed, Total) pair as the "Session 0"
baseline. After that, no session may commit changes that drop the
Passed count below the previous session's recorded value.

## Code footprint

| Session | C LOC | Rust LOC (engine only) | C/(C+Rust) |
|--------:|------:|-----------------------:|-----------:|
|       0 |39,419 |                      0 |       100% |
|       1 |39,272 |                    ~220 |      ~99.4% |
|       2 |38,825 |                   ~1000 |      ~97.4% |
|       3 |38,674 |                   ~1530 |      ~96.2% |
|       4 |38,467 |                   ~1790 |      ~95.6% |
|       5 |38,453 |                   ~1820 |      ~95.5% |
|       6 |38,453 |                   ~1960 |      ~95.2% |
|       7 |38,418 |                   ~2070 |      ~94.9% |
|       8 |38,063 |                   ~3000 |      ~92.7% |
|       9 |37,912 |                   ~3143 |      ~92.4% |
|      10 |37,673 |                   ~3521 |      ~91.5% |
|      11 |37,510 |                   ~3816 |      ~90.8% |
|      12 |37,483 |                   ~3935 |      ~90.5% |
|      13 |37,455 |                   ~3980 |      ~90.4% |
|      14 |37,309 |                   ~4200 |      ~89.9% |
|      15 |37,022 |                   ~4750 |      ~88.7% |
|      16 |36,796 |                   ~5200 |      ~87.9% |
|      17 |36,567 |                   ~5878 |      ~86.1% |
|      18 |35,910 |                   ~6571 |      ~84.5% |
|      19 |36,406 |                   ~7007 |      ~83.9% |
|      20 |35,346 |                   ~7608 |      ~82.3% |


(C LOC excludes teckit, xetex-XeTeXOTMath, and xetex-macos, which are
not in scope for translation. Measured with `wc -l` over the 16 in-scope
`.c` files listed in `MODULE_STATUS.md`. Rust LOC counts the engine's
own `src/*.rs` modules, excluding tests and `lib.rs`.)

## Functions migrated

| Session | Rust fns | C fns remaining | % migrated |
|--------:|---------:|----------------:|-----------:|
|       0 |        0 |             ~?? |         0% |
|       1 |        6 |           ~???? |     <0.5%  |
|       2 |       13 |           ~???? |      ~1%   |
|       3 |       18 |           ~???? |     ~1.5%  |
|       4 |       30 |           ~???? |      ~2.5% |
|       5 |       31 |           ~???? |      ~2.6% |
|       6 |       35 |           ~???? |      ~3.0% |
|       7 |       38 |           ~???? |      ~3.2% |
|       8 |       49 |           ~???? |      ~4.1% |
|       9 |       61 |           ~???? |      ~5.1% |
|      10 |       70 |           ~???? |      ~5.9% |
|      11 |       77 |           ~???? |      ~6.5% |
|      12 |       79 |           ~???? |      ~6.7% |
|      13 |       81 |           ~???? |      ~6.8% |
|      14 |       86 |           ~???? |      ~7.3% |
|      15 |       97 |           ~???? |      ~8.2% |
|      16 |      102 |           ~???? |      ~8.6% |
|      17 |      116 |           ~???? |      ~9.7% |
|      18 |      138 |           ~???? |     ~11.6% |
|      19 |      144 |            ~214 |     ~40.2% |
|      20 |      160 |            ~198 |     ~44.7% |


Total C function count to be measured in session 1 via:

```
for f in xetex/xetex-*.c; do
    echo "$(grep -c '^[[:alpha:]_][[:alpha:]0-9_ *]*\s*[[:alpha:]_][[:alpha:]0-9_]*\s*(' "$f") $f"
done | sort -nr
```

## Green-build streak

Count of consecutive sessions without a parity-count regression. A
regression resets this to zero and must be noted in `SESSION_LOG.md`.

Current streak: **20** (sessions 1–20 held parity at 29/29).


## Health checks

A session should fail fast if any of these are broken:

-   [ ] `cargo build -p tectonic_engine_xetex` succeeds
-   [ ] `cargo build -p tectonic` succeeds
-   [ ] `cargo test -p tectonic_engine_xetex` passes (unit tests inside the crate)
-   [ ] `cargo test -p tectonic --test tex-outputs` passes at ≥ the previous count
-   [ ] `cargo clippy -p tectonic_engine_xetex -- -D warnings` is clean (once we have non-trivial Rust)

If any box is unchecked at session start, **fixing the foundation is
the first task of the session**. You do not start new translation work
on a broken baseline.
