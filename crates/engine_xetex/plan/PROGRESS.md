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
|      53 | 2026-04-27 |     29 |    29 |     0 | vert_break (~135 lines, goto → Next enum + labeled loop), vsplit (~102 lines), box_end (~90 lines), begin_box (~180 lines, goto → labeled block break). All 4 suites pass. |
|      54 | 2026-04-27 |     29 |    29 |     0 | just_copy (~125 lines, goto → Option<i32>), just_reverse (~95 lines, goto done → break 'done), handle_right_brace (~215 lines, 16 match arms), char_pw (~45 lines). All 4 suites pass. |
|      55 | 2026-04-28 |     29 |    29 |     0 | close_files_and_terminate (~40 lines), prune_page_top (~70 lines), do_marks (~80 lines, recursive), do_extension (~165 lines, 13 switch cases). All 4 suites pass. |
|      56 | 2026-04-28 |     29 |    29 |     0 | vpackage (~220 lines), hpack (~450 lines, NATIVE_WORD_NODE merging, bidi). Fixed SIGSEGV: mem_node_addr not mem_ptr for measure_native_node. All 4 suites pass. |
|      57 | 2026-04-29 |     29 |    29 |     0 | conv_toks (~350 lines), begin_token_list (~50 lines). Bug: ASSIGN_TOKS must be 73 not 72; caught by TRIP. All 4 suites pass. |
|      58 | 2026-04-29 |     29 |    29 |     0 | flush_node_list (~182 lines), copy_node_list (~160 lines), show_activities (~122 lines), show_node_list (~470 lines), print_subsidiary_data (~37 lines). All 4 suites pass. |
|      59 | 2026-04-30 |     29 |    29 |     0 | show_context (~232 lines), expand (~272 lines). All 4 suites pass. |
|      60 | 2026-04-30 |     29 |    29 |     0 | expand (compilation fixes + wrap). All 4 suites pass. |
|      61 | 2026-04-30 |     29 |    29 |     0 | get_next (~490 lines), macro_call (~330 lines). Critical COMMENT-case bug fixed. All 4 suites pass. |
|      62 | 2026-05-01 |     29 |    29 |     0 | scan_glyph_number, round_decimals, xetex_scan_dimen. All 4 suites pass. |
|      63 | 2026-05-01 |     29 |    29 |     0 | print_cmd_chr (~1400 lines), new_native_word_node, do_locale_linebreaks. All 4 suites pass. |
|      64 | 2026-05-01 |     29 |    29 |     0 | new_native_character. All 4 suites pass. |
|      65 | 2026-05-01 |     29 |    29 |     0 | load_native_font. All 4 suites pass. |
|      66 | 2026-05-01 |     29 |    29 |     0 | read_font_info. All 4 suites pass. |
|      67 | 2026-05-01 |     29 |    29 |     0 | Partial main_control: append_normal_space label. All 4 suites pass. |
|      68 | 2026-05-01 |     29 |    29 |     0 | Partial main_control: five switch-arm clusters through alignment/endcsname cases. All 4 suites pass. |
|      69 | 2026-05-02 |     29 |    29 |     0 | Partial main_control: math cases, spacer/EX_SPACE, IGNORE_SPACES, HMODE inter-char check. xetex-xetex0.c 8105→7935 LOC. All 4 suites pass. |
|      70 | 2026-05-04 |     29 |    29 |     0 | main_control_main_loop (main_loop + lig_loop, ~874 lines). Fixed lig_result=2 wrapup bug (extra 'i' in fi-ligature). xetex-xetex0.c 7935→7065 LOC. All 4 suites pass. |
|      71 | 2026-05-03 |     29 |    29 |     0 | main_control (~272 lines, big_switch/reswitch dispatcher). xetex-xetex0.c 7065→6793 LOC. All 4 suites pass. |
|      72 | 2026-05-04 |     29 |    29 |     0 | Strip all 36 #if-0 blocks from xetex-xetex0.c. File is now 10 LOC stub. DONE. All 4 suites pass. |
|      73 | 2026-05-04 |     29 |    29 |     0 | xetex-io.c: translate set_input_file_encoding, buffer_overflow, conversion_error, apply_normalization, input_line. File DONE. All 4 suites pass. |
|      74 | 2026-05-04 |     29 |    29 |     0 | xetex-pagebuilder.c: translate initialize_pagebuilder_variables, freeze_page_specs, ensure_vbox, fire_up, build_page. File DONE. All 4 suites pass. |
|      75 | 2026-05-04 |     29 |    29 |     0 | xetex-linebreak.c: translate push_node, pop_node hlist-stack helpers. All 4 suites pass. |
|      76 | 2026-05-04 |     29 |    29 |     0 | xetex-linebreak.c: translate get_native_usv, total_pw, find_protchar_left/right, finite_shrink, reconstitute, hyphenate, post_line_break. Remaining C bodies: line_break, try_break. All 4 suites pass. |
|      77 | 2026-05-07 |     29 |    29 |     0 | xetex-linebreak.c prep: exported try_break and active-list scratch state for Rust replacement. All 4 suites pass. |
|      78 | 2026-05-07 |     29 |    29 |     0 | xetex-linebreak.c: translate try_break and route C line_break through Rust. Fixed last_line_fit goto semantics caught by eTRIP. All 4 suites pass. |
|      79 | 2026-05-07 |     29 |    29 |     0 | xetex-linebreak.c: translate line_break; file has zero compiled C bodies. Fixed hang_after param, ligature hyphenation done3 control flow, and native-node font accessor. All 4 suites pass. |
|      80 | 2026-05-07 |     29 |    29 |     0 | xetex-linebreak.c: delete disabled #if 0 reference blocks (cleanup). xetex-pic.c: translate all 11 functions to src/pic.rs; file DONE. All 4 suites pass. |
|      81 | 2026-05-07 |     29 |    29 |     0 | xetex-synctex.c: translate all ~30 functions to src/synctex.rs; file DONE. All 4 suites pass. |
|      82 | 2026-05-07 |     29 |    29 |     0 | xetex-ext.c: translate read_double, read_rgb_a, D2Fix, Fix2D to src/ext.rs. All 4 suites pass. |
|      83 | 2026-05-07 |     29 |    29 |     0 | xetex-ext.c: translate print helpers, TFM mapping wrappers, and readCommonFeatures to src/ext.rs. All 4 suites pass. |
|      84 | 2026-05-07 |     29 |    29 |     0 | xetex-ext.c: translate get_encoding_mode_and_info, feature-tag parsers, and splitFontName to src/ext.rs. All 4 suites pass. |
|      85 | 2026-05-07 |     29 |    29 |     0 | xetex-ext.c: translate OpenType/Graphite font getters, font metrics, and release helper to src/ext.rs. All 4 suites pass. |
|      86 | 2026-05-07 |     29 |    29 |     0 | xetex-ext.c: translate 12 char-metric helpers (snap_zone, get_native_char_height_depth, getnativechar{ht,dp,ic,wd}, get_native_char_sidebearings, get_glyph_bounds, map_char_to_glyph, map_glyph_to_index, get_font_char_range, print_glyph_name). All 4 suites pass. |
|      87 | 2026-05-07 |     29 |    29 |     0 | xetex-ext.c: translate 5 native node helpers (real_get_native_glyph, real_get_native_italic_correction, real_get_native_glyph_italic_correction, measure_native_glyph, real_get_native_word_cp). All 4 suites pass. |
|      88 | 2026-05-08 |     29 |    29 |     0 | xetex-ext.c: translate 13 functions (linebreak_start/next, makeXDVGlyphArrayData, make_font_def, apply_mapping, store_justified_native_glyphs, 7 aat_* stubs). Added ICU version detection in build.rs. xetex-ext.c 1256→670 LOC. All 4 suites pass. |
|      89 | 2026-05-08 |     29 |    29 |     0 | xetex-ext.c: translate loadOTfont (private Rust helper) and find_native_font (89869eaa). xetex-ext.c 670→296 LOC. All 4 suites pass. |
|      90 | 2026-05-08 |     29 |    29 |     0 | xetex-ext.c: translate measure_native_node (9acfe915), including ICU BiDi, layout copying, letterspacing, and glyph bbox metrics. xetex-ext.c 296→97 LOC / zero Linux C bodies. All 4 suites pass. |
|      91 | 2026-05-08 |     29 |    29 |     0 | xetex-shipout.c: translate new_edge (3ecb7ae8), starting src/shipout.rs. All 4 suites pass. |
|      92 | 2026-05-08 |     29 |    29 |     0 | xetex-math.c: translate 10 math command helpers (7e81f4b6), starting src/math.rs. All 4 suites pass. |
|      93 | 2026-05-08 |     29 |    29 |     0 | xetex-math.c: translate init_math (~270 lines, goto→labeled-loop, 133c1ac0). All 4 suites pass. |
|      94 | 2026-05-08 |     29 |    29 |     0 | xetex-math.c: translate app_display + after_math (~430 lines). Bug: if m < 0 must test mode before fin_mlist (49f0aee7). All 4 suites pass. |
|      95 | 2026-05-08 |     29 |    29 |     0 | xetex-math.c: translate math-symbol parameter helpers (math_x_height, math_quad, num/denom/sup/sub/drop/delim/axis cluster; 65eea6ce). All 4 suites pass. |
|      96 | 2026-05-08 |     29 |    29 |     0 | xetex-math.c: translate math-extension parameters plus fraction_rule/overbar/math_glue/math_kern (0c80ba9e). All 4 suites pass. |
|      97 | 2026-05-08 |     29 |    29 |     0 | xetex-math.c: translate flush_math/clean_box and make_over/make_under/make_vcenter (47196355). All 4 suites pass. |
|      98 | 2026-05-08 |     29 |    29 |     0 | xetex-math.c: translate fetch and compute_ot_math_accent_pos (7b62b15b). All 4 suites pass. |
|      99 | 2026-05-08 |     29 |    29 |     0 | xetex-math.c: translate make_radical and make_fraction (ca405061), promoting var_delimiter/rebox for Rust callers. All 4 suites pass. |
|     100 | 2026-05-09 |     29 |    29 |     0 | xetex-math.c: translate make_op (93c786d8), promoting build_opentype_assembly for Rust callers. Goto→labeled-block, do-while→loop. All 4 suites pass. |
|     101 | 2026-05-09 |     29 |    29 |     0 | xetex-math.c: translate make_ord (99e00684). Goto-restart→'restart: loop, lig-kern while-true→inner loop. All 4 suites pass. |
|     102 | 2026-05-09 |     29 |    29 |     0 | xetex-math.c: translate 7 helpers (e27b5336): attach_hkern_to_new_hlist, make_left_right, char_box, stack_into_box, height_plus_depth, stack_glyph_into_box, stack_glue_into_box. All 4 suites pass. |
|     103 | 2026-05-09 |     29 |    29 |     0 | xetex-math.c: translate make_math_accent (0f0435fe). Three goto patterns: done1→labeled block, done→plain break, found→labeled block. All 4 suites pass. |
|     104 | 2026-05-09 |     29 |    29 |     0 | xetex-math.c: translate initialize_math_variables (no-op) + rebox (b5acc37f). null_delimiter was write-only static; removed. All 4 suites pass. |
|     105 | 2026-05-09 |     29 |    29 |     0 | xetex-math.c: translate build_opentype_assembly (a77e095c). do-while→loop, WEB for-loops→Rust ranges, ot_part_* added to extern block. All 4 suites pass. |
|     106 | 2026-05-09 |     29 |    29 |     0 | xetex-math.c: translate var_delimiter (e4eed5c9). 6×goto found→break 'found, goto continue_→continue in inner loop. All 4 suites pass. |
|     107 | 2026-05-09 |     29 |    29 |     0 | xetex-math.c: translate make_scripts (8b15e714). Factored repeated script glyph discovery into a Rust helper. All 4 suites pass. |
|     108 | 2026-05-09 |     29 |    29 |     0 | xetex-math.c: translate mlist_to_hlist (4ee11a58). Final compiled math body moved to Rust; file now has zero C bodies. All 4 suites pass. |
|     109 | 2026-05-09 |     29 |    29 |     0 | xetex-shipout.c: translate dvi_four and dvi_two (55c6d9e9). Exported C dvi_out so Rust helpers can keep using the existing DVI buffer state. All 4 suites pass. |
|     110 | 2026-05-09 |     29 |    29 |     0 | xetex-shipout.c: translate dvi_native_font_def and dvi_font_def (0fe52f3a). Rust now emits both native and classic DVI font definitions through the existing dvi_out bridge. All 4 suites pass. |
|     111 | 2026-05-15 |     29 |    29 |     0 | xetex-shipout.c: translate DVI buffer statics + initialize/deinitialize_shipout_variables, write_to_dvi, dvi_swap, dvi_out, dvi_pop (9a26ee88). All 4 suites pass. |
|     112 | 2026-05-15 |     29 |    29 |     0 | xetex-shipout.c: translate prune_movements and finalize_dvi_file (1819fba9). All 4 suites pass. |
|     113 | 2026-05-19 |     29 |    29 |     0 | xetex-shipout.c: translate out_what, movement, special_out, write_out, pic_out (fc907f6a). All 4 suites pass. |
|     114 | 2026-05-19 |     29 |    29 |     0 | xetex-shipout.c: translate reverse (0d3b8461). All 4 suites pass. |
|     115 | 2026-05-19 |     29 |    29 |     0 | xetex-shipout.c: translate ship_out (31345c81). All 4 suites pass. |
|     116 | 2026-05-19 |     29 |    29 |     0 | xetex-shipout.c: translate vlist_out (f4cc04d5). Fixed swapped GLUE_NODE glue/leader fields caught by a single-case SIGSEGV repro. All 4 suites pass. |
|     117 | 2026-05-20 |     29 |    29 |     0 | xetex-shipout.c: translate hlist_out (f05b105f). File now has zero compiled C bodies. Fixed Rust is_char_node to use hi_mem_min after a single-case XDV divergence. All 4 suites pass. |
|     118 | 2026-05-20 |     29 |    29 |     0 | xetex-ini.c: start src/ini.rs with init_io and get_strings_started (5b846747). Avoided format-file I/O; all 4 suites pass. |
|     119 | 2026-05-20 |     29 |    29 |     0 | xetex-ini.c: translate new_trie_op (1958fc5e) and promote trie-op hash table to shared Rust static. All 4 suites pass. |
|     120 | 2026-05-20 |     29 |    29 |     0 | xetex-ini.c: translate trie_node and compress_trie (47eb2114), adding trie array accessors. All 4 suites pass. |
|     121 | 2026-05-20 |     29 |    29 |     0 | xetex-ini.c: translate first_fit, trie_pack, and trie_fix (4e756427), keeping trie packing as one cluster. All 4 suites pass. |
|     122 | 2026-05-21 |     29 |    29 |     0 | xetex-ini.c: translate init_trie and wire trie globals. All 4 suites pass. |
|     123 | 2026-05-21 |     29 |    29 |     0 | xetex-ini.c: translate new_patterns and new_hyph_exceptions (67330244, c1c91b6d prep). All 4 suites pass. |
|     124 | 2026-05-21 |     29 |    29 |     0 | xetex-ini.c: translate sort_avail and primitive (0904e3cf, baeae90e prep). All 4 suites pass. |
|     125 | 2026-05-21 |     29 |    29 |     0 | xetex-ini.c: translate final_cleanup (77978635, 6a55a6e5 prep); un-static store_fmt_file for FFI access. All 4 suites pass. |
|     126 | 2026-05-21 |     29 |    29 |     0 | xetex-ini.c: translate initialize_more_variables (a9c6e366, 71347f28 prep). All 4 suites pass. |
|     127 | 2026-05-21 |     29 |    29 |     0 | xetex-ini.c: translate initialize_more_initex_variables (edbaf81f, dc0031e4 prep). Loop with empty C bounds (ETEX_PEN_BASE..NUM_ETEX_PENALTIES-1) translated literally. All 4 suites pass. |
|     128 | 2026-05-21 |     29 |    29 |     0 | xetex-ini.c: translate initialize_primitives (4307d45d, 9c29265d prep). primitives[] lifted to file-scope xetex_primitives + count; Rust mirrors struct via #[repr(C)] and iterates via raw-pointer offset. All 4 suites pass. |
|     129 | 2026-05-22 |     29 |    29 |     0 | xetex-ini.c: translate tt_cleanup (926ae467). File-static yhash freed via a tiny C helper; all other freed globals accessed via tt_cleanup_-prefixed extern block with #[link_name] on every item. All 4 suites pass. |
|     130 | 2026-05-22 |     29 |    29 |     0 | xetex-ini.c: translate prefixed_command (1571e5cd). ~760-line assignment/definition dispatcher; goto-done→'sw labeled block, prefix-loop/non-INITEX \patterns early returns preserved. All 4 suites pass. |
|     131 | 2026-05-22 |     29 |    29 |     0 | xetex-ini.c: translate tt_run_engine. Top-level lifecycle/allocation entry point now in Rust; C body deleted, load_fmt_file exported for the remaining format-file cluster. All 4 suites pass. |


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
|      53 |29,700 |                  ~17200 |      ~63.2% |
|      54 |29,400 |                  ~17700 |      ~62.4% |
|      55 |28,900 |                  ~18200 |      ~61.3% |
|      56 |28,000 |                  ~19968 |      ~58.3% |
|      57 |27,600 |                  ~20400 |      ~57.5% |
|      58 |28,010 |                  ~21883 |      ~56.1% |
|      59 |27,400 |                  ~22500 |      ~54.9% |
|      60 |27,300 |                  ~22600 |      ~54.7% |
|      61 |26,200 |                  ~23500 |      ~52.7% |
|      62 |26,000 |                  ~23700 |      ~52.3% |
|      63 |24,600 |                  ~25000 |      ~49.6% |
|      64 |24,500 |                  ~25100 |      ~49.4% |
|      65 |24,370 |                  ~25460 |      ~48.9% |
|      66 |23,905 |                  ~26090 |      ~47.8% |
|      67 |23,870 |                  ~26170 |      ~47.7% |
|      68 |23,724 |                  ~26560 |      ~47.2% |
|      75 |17,418 |                  ~29809 |      36.9% |
|      78 |~16,775 |                  ~30440 |      ~35.5% |
|      79 |~15,810 |                  ~32700 |      ~32.6% |
|      80 |~14,025 |                  ~33230 |      ~29.7% |
|      81 |~12,752 |                  ~34,370 |      ~27.1% |
|      82 |~12,670 |                  ~34,510 |      ~26.8% |
|      83 | 12,497 |                  ~34,700 |      ~26.5% |
|      84 | 12,355 |                  ~35,060 |      ~26.1% |
|      86 | ~12,143 |                  ~35,284 |      ~25.6% |
|      87 | ~12,030 |                  ~35,390 |      ~25.4% |
|      89 | ~11,656 |                  ~35,880 |      ~24.5% |
|      90 | ~11,457 |                  ~36,166 |      ~24.1% |
|      91 | ~11,443 |                  ~36,190 |      ~24.0% |
|      92 | 10,311 |                  37,025 |       21.8% |
|      95 |  9,277 |                  38,094 |       19.6% |
|     103 |  8,217 |                  35,445 |       18.8% |
|     104 |  8,180 |                  35,487 |       18.7% |
|     105 |  8,040 |                  35,599 |       18.4% |
|     106 |  7,863 |                  35,761 |       18.0% |
|     107 |  7,398 |                  35,753 |       17.1% |
|     108 |  6,931 |                  44,316 |       13.5% |
|     109 |  6,908 |                  44,346 |       13.5% |
|     110 |  6,832 |                  44,723 |       13.3% |
|     111 |  6,720 |                  ~44,980 |       13.0% |
|     112 |  6,601 |                  ~45,080 |       12.8% |
|     113 |  6,194 |                  45,478 |       12.0% |
|     114 |  5,985 |                  45,417 |       11.6% |
|     115 |  5,774 |                  45,661 |       11.2% |
|     116 |  5,388 |                  46,044 |       10.5% |
|     119 |  4,519 |                  46,898 |        8.8% |
|     120 |  4,488 |                  46,993 |        8.7% |
|     121 |  4,404 |                  47,457 |        8.5% |
|     122 |  4,274 |                  47,623 |        8.2% |
|     123 |  3,891 |                  47,816 |        7.5% |
|     124 |  3,822 |                  47,890 |        7.4% |
|     125 |  3,729 |                  48,022 |        7.2% |
|     126 |  3,571 |                  48,178 |        6.9% |
|     127 |  3,403 |                  48,442 |        6.6% |
|     128 |  3,343 |                  48,557 |        6.4% |
|     129 |  3,256 |                  48,979 |        6.2% |
|     130 |  2,496 |                  49,986 |        4.8% |


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
|      53 |      329 |             ~29 |     ~91.9% |
|      54 |      333 |             ~25 |     ~93.0% |
|      55 |      337 |             ~21 |     ~94.1% |
|      56 |      339 |             ~19 |     ~94.7% |
|      57 |      341 |             ~17 |     ~95.3% |
|      58 |      346 |             ~13 |     ~96.4% |
|      59 |      347 |             ~12 |     ~96.7% |
|      60 |      348 |             ~11 |     ~96.9% |
|      61 |      350 |              ~9 |     ~97.5% |
|      62 |      353 |              ~7 |     ~98.1% |
|      63 |      356 |              ~5 |     ~98.6% |
|      64 |      357 |              ~4 |     ~98.9% |
|      65 |      358 |              ~3 |     ~99.2% |
|      66 |      359 |              ~2 |     ~99.4% |
|      67 |      360 |              ~2 |     ~99.4% |
|      68 |      365 |              ~2 |     ~99.4% |
|      75 |      367 |             ~?? |       ~?? |
|      78 |      368 |             ~?? |       ~?? |
|      83 |      422 |             ~?? |       ~?? |
|      84 |      426 |             ~?? |       ~?? |
|      85 |      435 |             ~?? |       ~?? |
|      86 |      447 |             ~?? |       ~?? |
|      87 |      452 |             ~?? |       ~?? |
|      91 |      453 |             ~?? |       ~?? |
|      92 |      463 |             ~?? |       ~?? |
|      95 |      485 |             ~?? |       ~?? |
|      96 |      495 |             ~?? |       ~?? |
|      97 |      500 |             ~?? |       ~?? |
|      98 |      502 |             ~?? |       ~?? |
|      99 |      504 |             ~?? |       ~?? |
|     100 |      505 |             ~?? |       ~?? |
|     101 |      506 |             ~?? |       ~?? |
|     102 |      513 |             ~?? |       ~?? |
|     103 |      514 |             ~?? |       ~?? |
|     104 |      516 |             ~?? |       ~?? |
|     105 |      517 |             ~?? |       ~?? |
|     106 |      518 |             ~?? |       ~?? |
|     107 |      519 |             ~?? |       ~?? |
|     108 |      520 |             ~?? |       ~?? |
|     109 |      522 |             ~?? |       ~?? |
|     110 |      524 |             ~?? |       ~?? |
|     111 |      530 |             ~?? |       ~?? |
|     112 |      532 |             ~?? |       ~?? |
|     113 |      537 |              ~4 |     ~99.3% |
|     114 |      538 |              ~3 |     ~99.4% |
|     115 |      539 |              ~2 |     ~99.6% |
|     116 |      540 |              ~1 |     ~99.8% |


Total C function count to be measured in session 1 via:

```
for f in xetex/xetex-*.c; do
    echo "$(grep -c '^[[:alpha:]_][[:alpha:]0-9_ *]*\s*[[:alpha:]_][[:alpha:]0-9_]*\s*(' "$f") $f"
done | sort -nr
```

## Green-build streak

Count of consecutive sessions without a parity-count regression. A
regression resets this to zero and must be noted in `SESSION_LOG.md`.

Current streak: **131** (sessions 1–131 held parity at 29/29; TRIP 2/2 maintained from session 23 onward).


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
