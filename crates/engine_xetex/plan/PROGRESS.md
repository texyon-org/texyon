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
|      21 | 2026-04-19 |     29 |    29 |     0 | SA cluster (delete_sa_ref + 7 more), unsave, show_cur_cmd_chr. |
|      22 | 2026-04-19 |     29 |    29 |     0 | end_token_list cluster (4 fns, new src/input.rs), new_index/find_sa_element, new_noad/style/choice, max_hyphenatable_length, new_margin_kern. |
|      23 | 2026-04-19 |     29 |    29 |     0 | Plan: TRIP test added. add_or_sub, quotient, fract, change_if_limit, scan_normal/mu_glue, flush_str, pseudo_close, get_input_normalization_state/get_tracing_fonts_state, scan_pdf_ext_toks. |
|      24 | 2026-04-20 |     29 |    29 |     0 | begin/end_file_reading, pseudo_input, bad_utf8/font_feature/font_mapping/graphite warnings, x_token, scan_optional_equals, scan_left_brace, norm_min, insert_dollar_sign, insert_relax, get_token. |
|      25 | 2026-04-20 |     29 |    29 |     0 | get_x_token, get_x_or_protected. |
|      26 | 2026-04-20 |     29 |    29 |     0 | scan_keyword. |
|      27 | 2026-04-20 |     29 |    29 |     0 | scan_rule_spec, scan_dimen, scan_decimal, scan_spec. |
|      28 | 2026-04-20 |     29 |    29 |     0 | scan_general_text. |
|      29 | 2026-04-20 |     29 |    29 |     0 | scan_expr (complex recursive expr scanner, 230 lines). |
|      30 | 2026-04-21 |     29 |    29 |     0 | pseudo_start, ins_the_toks, str_toks_cat, str_toks, the_toks, pass_text (6 fns). |
|      31 | 2026-04-21 |     29 |    29 |     0 | scan_toks, read_toks, begin_name, more_name, end_name, extra_right_brace, scan_font_ident, show_info (9 fns). |
|      32 | 2026-04-22 |     29 |    29 |     0 | find_font_dimen. TRIP remained 2/2. |
|      33 | 2026-04-22 |     29 |    29 |     0 | scan_and_pack_name. TRIP remained 2/2. |
|      34 | 2026-04-22 |     29 |    29 |     0 | scan_file_name_braced + scan_file_name. TRIP remained 2/2. |
|      35 | 2026-04-22 |     29 |    29 |     0 | pack_job_name. TRIP remained 2/2. |
|      36 | 2026-04-22 |     29 |    29 |     0 | make_name_string. TRIP remained 2/2. |
|      37 | 2026-04-22 |     29 |    29 |     0 | open_log_file. TRIP remained 2/2. |
|      38 | 2026-04-22 |     29 |    29 |     0 | scan_something_internal + scan_int. TRIP remained 2/2. |
|      39 | 2026-04-22 |     29 |    29 |     0 | conditional. TRIP remained 2/2. |
|      40 | 2026-04-23 |     29 |    29 |     0 | start_input. TRIP remained 2/2. |
|      41 | 2026-04-23 |     29 |    29 |     0 | 17 functions: effective_char cluster, eTeX_enabled, print_totals, you_cant/report_illegal_case/privileged, normal_paragraph, give_err_help, new_whatsit, end_graf, append_penalty, align_error/no_align_error/omit_error/cs_error, push_math. TRIP 2/2. |
|      42 | 2026-04-23 |     29 |    29 |     0 | 13 functions: box_error, its_all_over, append_glue, append_kern, make_mark, new_interaction, alter_aux/prev_graf/page_so_far/integer/box_dimen, trap_zero_glue, fix_language. TRIP 2/2. |
|      43 | 2026-04-24 |     29 |    29 |     0 | 11 functions: off_save, indent_in_hmode, head_for_vmode, append_src_special, tokens_to_string, compare_strings, new_write_whatsit, begin_insert_or_adjust, do_endv, get_r_token, append_discretionary. TRIP 2/2. |
|      44 | 2026-04-24 |     29 |    29 |     0 | 3 functions: shift_case, build_discretionary, issue_message. TRIP 2/2. |
|      45 | 2026-04-25 |     29 |    29 |     0 | 6 functions: append_to_vlist, align_peek, show_whatever, do_register_command, new_font. TRIP 2/2. |
|      46 | 2026-04-25 |     29 |    29 |     0 | 6 functions: make_accent, append_italic_correction, delete_last, unpackage + globals. TRIP 2/2. |
|      47 | 2026-04-25 |     29 |    29 |     0 | 9 functions: char_warning, app_space, new_graf, package, insert_src_special, scan_box, tt_insert_special, pack_file_name. TRIP 2/2. |
|      48 | 2026-04-26 |     29 |    29 |     0 | 9 functions: scan_math, set_math_char, new_character, scan_glue, push_alignment, pop_alignment, init_span, init_row, init_col. TRIP 2/2. |
|      49 | 2026-04-26 |     29 |    29 |     0 | get_preamble_token. TRIP 2/2. |
|      50 | 2026-04-26 |     29 |    29 |     0 | 3 functions: init_align, fin_col, fin_row. Added adjust_tail/pre_adjust_tail/total_stretch/total_shrink globals. TRIP 2/2. |
|      51 | 2026-04-26 |     29 |    29 |     0 | fin_align (~300 lines). Bug: set_mem_b32s1 vs set_eqtb_b32s1 for DIMENPAR; caught by TRIP. All 4 suites pass. |
|      52 | 2026-04-26 |     29 |    29 |     0 | show_save_groups (~210 lines, goto-tangle → enum After), do_assignments (14 lines). All 4 suites pass. |


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
|      21 |35,042 |                   ~8045 |      ~81.3% |
|      22 |35,440 |                   ~8479 |      ~80.7% |
|      23 |35,234 |                   ~8718 |      ~80.2% |
|      24 |34,991 |                   ~9271 |      ~79.0% |
|      25 |34,952 |                   ~9332 |      ~78.9% |
|      26 |34,897 |                   ~9402 |      ~78.8% |
|      27 |34,835 |                   ~9472 |      ~78.7% |
|      28 |34,782 |                   ~9542 |      ~78.6% |
|      29 |34,781 |                   ~9817 |      ~78.0% |
|      30 |34,796 |                  ~10215 |      ~77.3% |
|      31 |34,820 |                  ~10865 |      ~76.2% |
|      32 |34,108 |                  ~10721 |      ~76.1% |
|      33 |34,104 |                  ~10730 |      ~76.1% |
|      34 |34,030 |                  ~10809 |      ~75.9% |
|      35 |34,024 |                  ~10820 |      ~75.9% |
|      36 |33,987 |                  ~10863 |      ~75.8% |
|      37 |33,950 |                  ~10915 |      ~75.7% |
|      38 |32,842 |                  ~12188 |      ~72.9% |
|      39 |32,437 |                  ~12609 |      ~72.0% |
|      40 |32,274 |                  ~12809 |      ~71.6% |
|      41 |32,054 |                  ~13608 |      ~70.2% |
|      42 |31,811 |                  ~13931 |      ~69.6% |
|      43 |31,568 |                  ~14338 |      ~68.8% |
|      44 |31,386 |                  ~14545 |      ~68.3% |
|      45 |31,100 |                  ~15116 |      ~67.3% |
|      46 |30,900 |                  ~15461 |      ~66.7% |
|      47 |30,600 |                  ~15800 |      ~65.9% |
|      48 |30,400 |                  ~16100 |      ~65.4% |
|      49 |30,300 |                  ~16160 |      ~65.2% |
|      50 |30,000 |                  ~16500 |      ~64.5% |


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
|      21 |      171 |            ~188 |     ~47.6% |
|      22 |      182 |            ~177 |     ~50.7% |
|      23 |      193 |            ~166 |     ~53.8% |
|      24 |      207 |            ~152 |     ~57.7% |
|      25 |      209 |            ~150 |     ~58.2% |
|      26 |      210 |            ~149 |     ~58.5% |
|      27 |      214 |            ~145 |     ~59.6% |
|      28 |      215 |            ~144 |     ~59.9% |
|      29 |      216 |            ~143 |     ~60.2% |
|      30 |      222 |            ~137 |     ~61.8% |
|      31 |      231 |            ~128 |     ~64.3% |
|      32 |      232 |            ~127 |     ~64.6% |
|      33 |      233 |            ~126 |     ~64.9% |
|      34 |      235 |            ~124 |     ~65.5% |
|      35 |      236 |            ~123 |     ~65.7% |
|      36 |      237 |            ~122 |     ~66.0% |
|      37 |      238 |            ~121 |     ~66.3% |
|      38 |      240 |            ~119 |     ~66.9% |
|      39 |      241 |            ~118 |     ~67.1% |
|      40 |      242 |            ~117 |     ~67.4% |
|      41 |      259 |            ~100 |     ~72.1% |
|      42 |      272 |             ~87 |     ~75.8% |
|      43 |      283 |             ~76 |     ~78.8% |
|      44 |      286 |             ~73 |     ~79.7% |
|      45 |      292 |             ~65 |     ~81.8% |
|      46 |      300 |             ~59 |     ~83.6% |
|      47 |      309 |             ~50 |     ~86.1% |
|      48 |      318 |             ~41 |     ~88.6% |
|      49 |      319 |             ~40 |     ~88.9% |
|      50 |      322 |             ~37 |     ~89.7% |
|      51 |      323 |             ~36 |     ~90.0% |
|      52 |      325 |             ~34 |     ~90.5% |


Total C function count to be measured in session 1 via:

```
for f in xetex/xetex-*.c; do
    echo "$(grep -c '^[[:alpha:]_][[:alpha:]0-9_ *]*\s*[[:alpha:]_][[:alpha:]0-9_]*\s*(' "$f") $f"
done | sort -nr
```

## Green-build streak

Count of consecutive sessions without a parity-count regression. A
regression resets this to zero and must be noted in `SESSION_LOG.md`.

Current streak: **52** (sessions 1–52 held parity at 29/29; TRIP 2/2 maintained from session 23 onward).


## Health checks

A session should fail fast if any of these are broken:

-   [ ] `cargo build -p tectonic_engine_xetex` succeeds
-   [ ] `cargo build -p tectonic` succeeds
-   [ ] `cargo test -p tectonic_engine_xetex` passes (unit tests inside the crate)
-   [ ] `cargo test -p tectonic --test trip` passes at 2/2 (TRIP + eTRIP)
-   [ ] `cargo test -p tectonic --test tex-outputs` passes at ≥ the previous count
-   [ ] `cargo run -p tectonic -- ~/Code/tectonic/test.tex --print` compiles without errors (LaTeX2e+LaTeX3 smoke test — catches conditional/scan bugs that TRIP and tex-outputs miss)
-   [ ] `cargo clippy -p tectonic_engine_xetex -- -D warnings` is clean (once we have non-trivial Rust)

If any box is unchecked at session start, **fixing the foundation is
the first task of the session**. You do not start new translation work
on a broken baseline.
