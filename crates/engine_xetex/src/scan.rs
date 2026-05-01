//! Scanner range-validation helpers and mu_error.
//!
//! Translated from xetex-xetex0.c.

use crate::globals::{
    align_state, arith_error, avail, buffer_entry, cur_chr, cur_cmd, cur_cs, cur_input, cur_tok,
    cur_val, cur_val_level, def_ref, eqtb_b32s1, set_eqtb_b32s1, eqtb_is_null, expand_depth,
    expand_depth_count, file_offset, first, fmem_ptr, font_glue, font_mem_size, font_params,
    font_ptr, font_mapping_ptr, hash_s1, init_pool_ptr, last, max_print_line, max_reg_help_line,
    max_reg_num, mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1, old_setting, open_parens,
    param_base, pool_ptr, pool_size, pseudo_files, read_file_entry, read_open_entry, rust_stdout,
    save_ptr, save_stack_b32s1, scanner_status, selector, set_align_state, set_arith_error,
    set_avail, set_buffer_entry, set_cur_chr, set_cur_cmd, set_cur_cs, set_cur_input_limit,
    set_cur_input_loc, set_cur_input_name, set_cur_input_start, set_cur_input_state,
    set_cur_input_index, set_cur_input_synctex_tag, set_cur_tok, set_cur_val, set_cur_val_level, set_def_ref,
    set_expand_depth_count, set_first, set_fmem_ptr, set_font_glue, set_font_info_b32s1,
    set_font_params, set_help_line, set_help_ptr, set_line, set_mem_b16s0, set_mem_b16s1,
    set_mem_b16s2, set_mem_b16s3, set_mem_b32s0, set_mem_b32s1, set_name_in_progress, name_in_progress,
    set_no_new_control_sequence, set_old_setting, set_open_parens, set_pool_ptr,
    set_pseudo_files, set_read_open_entry, set_save_ptr, set_save_stack_b32s1,
    set_scanner_status, set_selector, set_stop_at_space, set_str_pool, set_str_ptr,
    set_warning_index, set_xtx_ligature_present, stop_at_space, str_pool_entry, str_ptr,
    str_start, term_offset, warning_index, xtx_ligature_present,
    char_base_entry, font_info_b16, cur_list_mode, cur_group,
    active_width, set_active_width, set_best_height_plus_depth,
    mem_word, set_mem_word, mem_b16s3, set_mem_ptr,
    write_open, texmf_log_name, temp_ptr,
};
use crate::mem::new_rule;
use crate::output::print_cmd_chr;

extern "C" {
    fn begin_token_list(p: i32, t: u16);
    fn error_here_with_diagnostic(msg: *const libc::c_char) -> *mut std::ffi::c_void;
    fn capture_to_diagnostic(diag: *mut std::ffi::c_void);
    fn int_error(n: i32);
    fn ttstub_diag_printf(
        d: *mut std::ffi::c_void,
        fmt: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn error();
    fn ttstub_output_flush(handle: usize) -> libc::c_int;
    fn input_line(f: *mut libc::c_void) -> libc::c_int;
    fn u_close(f: *mut libc::c_void);
    fn line_break(d: bool);
    fn build_page();
    fn _tt_abort(fmt: *const libc::c_char) -> !;
    fn ttstub_output_open(path: *const libc::c_char, gzip: libc::c_int) -> usize;
    fn aat_font_get(what: i32, engine: *mut libc::c_void) -> i32;
    fn aat_font_get_1(what: i32, engine: *mut libc::c_void, p1: i32) -> i32;
    fn aat_font_get_2(what: i32, engine: *mut libc::c_void, p1: i32, p2: i32) -> i32;
    fn aat_font_get_named(what: i32, engine: *mut libc::c_void) -> i32;
    fn aat_font_get_named_1(what: i32, engine: *mut libc::c_void, p1: i32) -> i32;
    fn ot_font_get(what: i32, engine: *mut libc::c_void) -> i32;
    fn ot_font_get_1(what: i32, engine: *mut libc::c_void, p1: i32) -> i32;
    fn ot_font_get_2(what: i32, engine: *mut libc::c_void, p1: i32, p2: i32) -> i32;
    fn ot_font_get_3(what: i32, engine: *mut libc::c_void, p1: i32, p2: i32, p3: i32) -> i32;
    fn gr_font_get_named(what: i32, engine: *mut libc::c_void) -> i32;
    fn gr_font_get_named_1(what: i32, engine: *mut libc::c_void, p1: i32) -> i32;
    fn not_native_font_error(cmd: i32, c: i32, f: i32);
    fn not_aat_font_error(cmd: i32, c: i32, f: i32);
    fn not_aat_gr_font_error(cmd: i32, c: i32, f: i32);
    fn not_ot_font_error(cmd: i32, c: i32, f: i32);
    fn getnativecharwd(f: i32, c: i32) -> i32;
    fn getnativecharht(f: i32, c: i32) -> i32;
    fn getnativechardp(f: i32, c: i32) -> i32;
    fn getnativecharic(f: i32, c: i32) -> i32;
    fn apply_tfm_font_mapping(mapping: *mut libc::c_void, c: i32) -> i32;
    fn usingGraphite(engine: *mut libc::c_void) -> bool;
    fn usingOpenType(engine: *mut libc::c_void) -> bool;
    fn get_glyph_bounds(f: i32, edge: i32, gid: i32) -> i32;
    fn get_font_char_range(f: i32, first: libc::c_int) -> i32;
    fn map_char_to_glyph(f: i32, ch: i32) -> i32;
    fn map_glyph_to_index(f: i32) -> i32;
    fn count_pdf_file_pages() -> i32;
    fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;
    fn new_spec(p: i32) -> i32;
    fn real_get_native_word_cp(node: *mut std::ffi::c_void, side: libc::c_int) -> i32;
    fn ship_out(p: i32);
    fn fin_mlist(p: i32) -> i32;
    fn build_choices();
    fn synctex_start_input();
    fn ttstub_issue_warning(fmt: *const libc::c_char, ...) -> libc::c_void;
    fn flush_math();
    fn resume_after_display();
    fn ttstub_output_close(handle: usize) -> libc::c_int;
    fn ttstub_output_putc(handle: usize, c: i32) -> i32;
    fn finalize_dvi_file();
    fn synctex_terminate(log_opened: bool);
    fn out_what(p: i32);
    fn load_picture(is_pdf: bool);
    fn measure_native_glyph(node: *mut std::ffi::c_void, use_glyph_metrics: bool);
    fn measure_native_node(node: *mut std::ffi::c_void, use_glyph_metrics: bool);
    fn get_encoding_mode_and_info(info: *mut i32) -> i32;
    fn aat_print_font_name(what: i32, attrs: *mut libc::c_void, param1: i32, param2: i32);
    fn gr_print_font_name(what: i32, engine: *mut libc::c_void, param1: i32, param2: i32);
    fn print_glyph_name(font: i32, gid: i32);
}

// Value-type constants for scan_glue (xetex_format.h:568–570).
const GLUE_VAL: i16 = 2;
const MU_VAL_SCAN: i16 = 3;

// INT_BASE + INT_PAR__xetex_hyphenatable_length offset for max_hyphenatable_length
// (xetex_format.h:568, xetex_format.h:115; xetex_format.h:7826729 = INT_BASE).
const INT_BASE: i32 = 7_826_729;
const INT_PAR_XETEX_HYPHENATABLE_LENGTH: i32 = 80;    // xetex_format.h:115
const INT_PAR_XETEX_INPUT_NORMALIZATION: i32 = 74;    // xetex_format.h:109
const INT_PAR_XETEX_TRACING_FONTS: i32 = 77;          // xetex_format.h:112
const HYPHENATABLE_LENGTH_LIMIT: i32 = 4095; // xetex-constants.h:334

// xetex-constants.h:26
const BIGGEST_USV: i32 = 0x10_FFFF;
// xetex-constants.h:335
const CHAR_CLASS_LIMIT: i32 = 4096;
// xetex-constants.h:343
const ACTIVE_MATH_CHAR: i32 = 0x1F_FFFF;
// xetex-constants.h:25
const BIGGEST_CHAR: i32 = 0xFFFF;
// xetex_format.h (NUMBER_MATH_FAMILIES)
const NUMBER_MATH_FAMILIES: i32 = 256;

// --- Constants for scan_something_internal / scan_int ---
// Command codes (xetex_format.h).
const DEF_CODE: u8 = 86;
const XETEX_DEF_CODE: u8 = 87;
const DEF_FAMILY: u8 = 88;
const SET_FONT: u8 = 89;
const DEF_FONT: u8 = 90;
const TOKS_REGISTER: u8 = 72;
const ASSIGN_TOKS: u8 = 73;
const ASSIGN_INT: u8 = 74;
const ASSIGN_DIMEN: u8 = 75;
const ASSIGN_GLUE: u8 = 76;
const ASSIGN_MU_GLUE: u8 = 77;
const ASSIGN_FONT_DIMEN: u8 = 78;
const ASSIGN_FONT_INT: u8 = 79;
const SET_AUX: u8 = 80;
const SET_PREV_GRAF: u8 = 81;
const SET_PAGE_DIMEN: u8 = 82;
const SET_PAGE_INT: u8 = 83;
const SET_BOX_DIMEN: u8 = 84;
const SET_SHAPE: u8 = 85;
const CHAR_GIVEN: u8 = 68;
const MATH_GIVEN: u8 = 69;
const XETEX_MATH_GIVEN: u8 = 70;
const LAST_ITEM: u8 = 71;
const REGISTER: u8 = 91;
const IGNORE_SPACES: u8 = 39;
const MIN_INTERNAL: u8 = 68;
const MAX_INTERNAL: u8 = 91;
// Value types (xetex-constants.h:288–296; xetex_format.h:568–572).
const INT_VAL: i16 = 0;
const DIMEN_VAL: i16 = 1;
const MU_VAL: i16 = 3;
const TOK_VAL: i16 = 5;
const IDENT_VAL: i16 = 4;
const INTER_CHAR_VAL: i16 = 6;
// Mode codes.
const VMODE: i16 = 1;
// eqtb bases (xetex_format.h).
const SKIP_BASE: i32 = 2_254_359;
const MU_SKIP_BASE: i32 = 2_254_615;
const LOCAL_BASE: i32 = 2_254_871;
const TOKS_BASE: i32 = 2_254_884;
const BOX_BASE: i32 = 2_255_144;
const CUR_FONT_LOC: i32 = 2_255_400;
const SF_CODE_BASE: i32 = 5_598_505;
const MATH_CODE_BASE: i32 = 6_712_617;
const COUNT_BASE: i32 = 7_826_812;
const DEL_CODE_BASE: i32 = 7_827_068;
const SCALED_BASE: i32 = 8_941_203;
const PRIM_EQTB_BASE: i32 = 2_243_238;
const FONT_ID_BASE: i32 = 2_245_338; // FROZEN_NULL_FONT
const HASH_BASE: i32 = 2_228_226;
const SINGLE_BASE: i32 = 1_114_113;
const ACTIVE_BASE: i32 = 1;
const FROZEN_RELAX: i32 = 2_243_233; // FROZEN_CONTROL_SEQUENCE + 7
const FROZEN_PRIMITIVE: i32 = 2_243_237; // FROZEN_CONTROL_SEQUENCE + 11
const UNDEFINED_PRIMITIVE: i32 = 0;
// LOCAL__ offsets (xetex_format.h:172,183).
const LOCAL_PAR_SHAPE: i32 = 0;
const LOCAL_XETEX_INTER_CHAR_TOKS: i32 = 11;
// Font flags (xetex-ext.h:47–48).
const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 0xFFFE;
// LEFT/RIGHT side (xetex_layout/src/c_api.rs).
const LEFT_SIDE: i32 = 0;
const RIGHT_SIDE: i32 = 1;
// LP/RP code bases for get_cp_code (xetex-constants.h:377–378).
const LP_CODE_BASE: i32 = 2;
const RP_CODE_BASE: i32 = 3;
// LAST_ITEM sub-codes (xetex_format.h:572–636).
const INPUT_LINE_NO_CODE: i32 = 4;
const BADNESS_CODE: i32 = 5;
const PDF_LAST_X_POS_CODE: i32 = 12;
const PDF_LAST_Y_POS_CODE: i32 = 13;
const ELAPSED_TIME_CODE: i32 = 16;
const PDF_SHELL_ESCAPE_CODE: i32 = 17;
const RANDOM_SEED_CODE: i32 = 18;
const ETEX_VERSION_CODE: i32 = 19;
const CURRENT_GROUP_LEVEL_CODE: i32 = 20;
const CURRENT_GROUP_TYPE_CODE: i32 = 21;
const CURRENT_IF_LEVEL_CODE: i32 = 22;
const CURRENT_IF_TYPE_CODE: i32 = 23;
const CURRENT_IF_BRANCH_CODE: i32 = 24;
const GLUE_STRETCH_ORDER_CODE: i32 = 25;
const GLUE_SHRINK_ORDER_CODE: i32 = 26;
const XETEX_VERSION_CODE: i32 = 27;
const XETEX_COUNT_GLYPHS_CODE: i32 = 28;
const XETEX_COUNT_VARIATIONS_CODE: i32 = 29;
const XETEX_VARIATION_CODE: i32 = 30;
const XETEX_FIND_VARIATION_BY_NAME_CODE: i32 = 31;
const XETEX_VARIATION_MIN_CODE: i32 = 32;
const XETEX_VARIATION_MAX_CODE: i32 = 33;
const XETEX_VARIATION_DEFAULT_CODE: i32 = 34;
const XETEX_COUNT_FEATURES_CODE: i32 = 35;
const XETEX_FEATURE_CODE_CODE: i32 = 36;
const XETEX_FIND_FEATURE_BY_NAME_CODE: i32 = 37;
const XETEX_IS_EXCLUSIVE_FEATURE_CODE: i32 = 38;
const XETEX_COUNT_SELECTORS_CODE: i32 = 39;
const XETEX_SELECTOR_CODE_CODE: i32 = 40;
const XETEX_FIND_SELECTOR_BY_NAME_CODE: i32 = 41;
const XETEX_IS_DEFAULT_SELECTOR_CODE: i32 = 42;
const XETEX_OT_COUNT_SCRIPTS_CODE: i32 = 43;
const XETEX_OT_COUNT_LANGUAGES_CODE: i32 = 44;
const XETEX_OT_COUNT_FEATURES_CODE: i32 = 45;
const XETEX_OT_SCRIPT_CODE: i32 = 46;
const XETEX_OT_LANGUAGE_CODE: i32 = 47;
const XETEX_OT_FEATURE_CODE: i32 = 48;
const XETEX_MAP_CHAR_TO_GLYPH_CODE: i32 = 49;
const XETEX_GLYPH_INDEX_CODE: i32 = 50;
const XETEX_FONT_TYPE_CODE: i32 = 51;
const XETEX_FIRST_CHAR_CODE: i32 = 52;
const XETEX_LAST_CHAR_CODE: i32 = 53;
const XETEX_PDF_PAGE_COUNT_CODE: i32 = 54;
const XETEX_GLYPH_BOUNDS_CODE: i32 = 55;
const FONT_CHAR_WD_CODE: i32 = 56;
const FONT_CHAR_HT_CODE: i32 = 57;
const FONT_CHAR_DP_CODE: i32 = 58;
const FONT_CHAR_IC_CODE: i32 = 59;
const PAR_SHAPE_LENGTH_CODE: i32 = 60;
const PAR_SHAPE_INDENT_CODE: i32 = 61;
const PAR_SHAPE_DIMEN_CODE: i32 = 62;
const GLUE_STRETCH_CODE: i32 = 63;
const GLUE_SHRINK_CODE: i32 = 64;
const ETEX_GLUE: i32 = 65;
const ETEX_MU: i32 = 66;
const ETEX_EXPR: i32 = 67;
const MU_TO_GLUE_CODE: i32 = 65;
const GLUE_TO_MU_CODE: i32 = 66;
const XETEX_INT: i32 = 27;
const XETEX_DIM: i32 = 55;
const LAST_NODE_TYPE_CODE_VAL: i32 = 3; // distinct name from other LAST_NODE_ uses
// Node type codes (xetex_format.h:389–437).
const MATH_NODE: u16 = 9;
const GLUE_NODE: u16 = 10;
const KERN_NODE: u16 = 11;
const PENALTY_NODE: u16 = 12;
const UNSET_NODE: u16 = 13;
const END_M_CODE: u16 = 3;
const MU_GLUE: u16 = 99;
// Conditional codes (xetex_format.h:537–539).
const FI_CODE: u8 = 2;
const ELSE_CODE: u8 = 3;
const OR_CODE: u8 = 4;
const UNLESS_CODE: i32 = 32;
// Engine version constants (xetex-constants.h:255,279).
const ETEX_VERSION: i32 = 2;
const XETEX_VERSION: i32 = 0;
// Token category constants for scan_int (xetex-constants.h:357,363–370).
const LETTER_TOKEN: i32 = 0x1600000;
const A_TOKEN: i32 = LETTER_TOKEN + b'A' as i32;
const OTHER_A_TOKEN: i32 = OTHER_TOKEN + b'A' as i32;
const HEX_TOKEN: i32 = OTHER_TOKEN + b'"' as i32;
const OCTAL_TOKEN: i32 = OTHER_TOKEN + b'\'' as i32;
const CONTINENTAL_POINT_TOKEN: i32 = OTHER_TOKEN + b',' as i32;
const POINT_TOKEN: i32 = OTHER_TOKEN + b'.' as i32;
const ALPHA_TOKEN: i32 = OTHER_TOKEN + b'`' as i32;
const FROZEN_PRIM_TOKEN: i32 = CS_TOKEN_FLAG + FROZEN_PRIMITIVE;

// translated from xetex-xetex0.c:5050 (mu_error)
#[no_mangle]
pub unsafe extern "C" fn mu_error() {
    error_here_with_diagnostic(c"Incompatible glue units".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(1);
    set_help_line(0, c"I'm going to assume that 1mu=1pt when they're mixed.".as_ptr());
    error();
}

// translated from xetex-xetex0.c:5078 (scan_char_class)
#[no_mangle]
pub unsafe extern "C" fn scan_char_class() {
    scan_int();
    if cur_val() < 0 || cur_val() > CHAR_CLASS_LIMIT {
        let errmsg = error_here_with_diagnostic(c"Bad character class".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, c"A character class must be between 0 and 4096.".as_ptr());
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5096 (scan_char_class_not_ignored)
#[no_mangle]
pub unsafe extern "C" fn scan_char_class_not_ignored() {
    scan_int();
    if cur_val() < 0 || cur_val() > CHAR_CLASS_LIMIT {
        let errmsg = error_here_with_diagnostic(c"Bad character class".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"A class for inter-character transitions must be between 0 and 4095.".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5114 (scan_eight_bit_int)
#[no_mangle]
pub unsafe extern "C" fn scan_eight_bit_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > 255 {
        let errmsg = error_here_with_diagnostic(c"Bad register code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"A register code or char class must be between 0 and 255.".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5132 (scan_usv_num)
#[no_mangle]
pub unsafe extern "C" fn scan_usv_num() {
    scan_int();
    if cur_val() < 0 || cur_val() > BIGGEST_USV {
        let errmsg = error_here_with_diagnostic(c"Bad character code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"A Unicode scalar value must be between 0 and \"10FFFF.".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5150 (scan_char_num)
#[no_mangle]
pub unsafe extern "C" fn scan_char_num() {
    scan_int();
    if cur_val() < 0 || cur_val() > BIGGEST_CHAR {
        let errmsg = error_here_with_diagnostic(c"Bad character code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"A character number must be between 0 and 65535.".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5330 (scan_math_class_int)
#[no_mangle]
pub unsafe extern "C" fn scan_math_class_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > 7 {
        let errmsg = error_here_with_diagnostic(c"Bad math class".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"Since I expected to read a number between 0 and 7,".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5348 (scan_math_fam_int)
#[no_mangle]
pub unsafe extern "C" fn scan_math_fam_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > NUMBER_MATH_FAMILIES - 1 {
        let errmsg = error_here_with_diagnostic(c"Bad math family".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"Since I expected to read a number between 0 and 255,".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5366 (scan_four_bit_int)
#[no_mangle]
pub unsafe extern "C" fn scan_four_bit_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > 15 {
        let errmsg = error_here_with_diagnostic(c"Bad number".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"Since I expected to read a number between 0 and 15,".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5384 (scan_fifteen_bit_int)
#[no_mangle]
pub unsafe extern "C" fn scan_fifteen_bit_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > 32767 {
        let errmsg = error_here_with_diagnostic(c"Bad mathchar".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, c"A mathchar number must be between 0 and 32767.".as_ptr());
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5168 (scan_xetex_math_char_int)
// math_char(x) = (x as u32) & 0x1FFFFF (xetex-xetexd.h:1205)
#[no_mangle]
pub unsafe extern "C" fn scan_xetex_math_char_int() {
    scan_int();
    let mc = (cur_val() as u32) & 0x1F_FFFF;
    if mc == ACTIVE_MATH_CHAR as u32 {
        if cur_val() != ACTIVE_MATH_CHAR {
            let errmsg = error_here_with_diagnostic(c"Bad active XeTeX math code".as_ptr());
            ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(2);
            set_help_line(
                1,
                c"Since I ignore class and family for active math chars,".as_ptr(),
            );
            set_help_line(0, c"I changed this one to \"1FFFFF.".as_ptr());
            int_error(cur_val());
            set_cur_val(ACTIVE_MATH_CHAR);
        }
    } else if mc > BIGGEST_USV as u32 {
        let errmsg = error_here_with_diagnostic(c"Bad XeTeX math character code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"Since I expected a character number between 0 and \"10FFFF,".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5245 (scan_register_num)
#[no_mangle]
pub unsafe extern "C" fn scan_register_num() {
    scan_int();
    if cur_val() < 0 || cur_val() > max_reg_num() {
        let errmsg = error_here_with_diagnostic(c"Bad register code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, max_reg_help_line());
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5263 (scan_four_bit_int_or_18)
#[no_mangle]
pub unsafe extern "C" fn scan_four_bit_int_or_18() {
    scan_int();
    if cur_val() < 0 || (cur_val() > 15 && cur_val() != 18) {
        let errmsg = error_here_with_diagnostic(c"Bad number".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"Since I expected to read a number between 0 and 15,".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// translated from xetex-xetex0.c:5403 (scan_delimiter_int)
#[no_mangle]
pub unsafe extern "C" fn scan_delimiter_int() {
    scan_int();
    if cur_val() < 0 || cur_val() > 0x7FF_FFFF {
        let errmsg = error_here_with_diagnostic(c"Bad delimiter code".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"A numeric delimiter code must be between 0 and 2^{27}-1.".as_ptr(),
        );
        set_help_line(0, c"I changed this one to zero.".as_ptr());
        int_error(cur_val());
        set_cur_val(0);
    }
}

// ---------------------------------------------------------------------------
// max_hyphenatable_length
// ---------------------------------------------------------------------------

/// Returns the maximum hyphenatable word length, clamped to HYPHENATABLE_LENGTH_LIMIT.
///
/// translated from xetex-xetex0.c:11330 (max_hyphenatable_length)
#[no_mangle]
pub unsafe extern "C" fn max_hyphenatable_length() -> i32 {
    let v = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_HYPHENATABLE_LENGTH);
    if v > HYPHENATABLE_LENGTH_LIMIT { HYPHENATABLE_LENGTH_LIMIT } else { v }
}

// ---------------------------------------------------------------------------
// scan_normal_glue / scan_mu_glue
// ---------------------------------------------------------------------------

/// Scan a glue specification (non-math).
///
/// translated from xetex-xetex0.c:6519 (scan_normal_glue)
#[no_mangle]
pub unsafe extern "C" fn scan_normal_glue() {
    scan_glue(GLUE_VAL);
}

/// Scan a mu-glue specification (math mode).
///
/// translated from xetex-xetex0.c:6524 (scan_mu_glue)
#[no_mangle]
pub unsafe extern "C" fn scan_mu_glue() {
    scan_glue(MU_VAL_SCAN);
}

// ---------------------------------------------------------------------------
// get_input_normalization_state / get_tracing_fonts_state / scan_pdf_ext_toks
// ---------------------------------------------------------------------------

/// Returns INTPAR(xetex_input_normalization), or 0 if eqtb is not yet initialized.
///
/// translated from xetex-xetex0.c:9073 (get_input_normalization_state)
#[no_mangle]
pub unsafe extern "C" fn get_input_normalization_state() -> i32 {
    if eqtb_is_null() { 0 } else { eqtb_b32s1(INT_BASE + INT_PAR_XETEX_INPUT_NORMALIZATION) }
}

/// Returns INTPAR(xetex_tracing_fonts).
///
/// translated from xetex-xetex0.c:9082 (get_tracing_fonts_state)
#[no_mangle]
pub unsafe extern "C" fn get_tracing_fonts_state() -> i32 {
    eqtb_b32s1(INT_BASE + INT_PAR_XETEX_TRACING_FONTS)
}

/// Scan a PDF extension token list (xpand=true, macro_def=false).
///
/// translated from xetex-xetex0.c:15945 (scan_pdf_ext_toks)
#[no_mangle]
pub unsafe extern "C" fn scan_pdf_ext_toks() {
    scan_toks(false, true);
}

// ---------------------------------------------------------------------------
// x_token / scan_optional_equals
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// get_token
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:3653 (get_token)
#[no_mangle]
pub unsafe extern "C" fn get_token() {
    set_no_new_control_sequence(false);
    get_next();
    set_no_new_control_sequence(true);
    if cur_cs() == 0 {
        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
    } else {
        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
    }
}

// Command-code constants (xetex_format.h / xetex-constants.h).
const MAX_COMMAND: u8 = 102;
const MAX_CHAR_VAL: i32 = 0x200000; // 1 << 21
const CS_TOKEN_FLAG: i32 = 0x1FFFFFF;
const SPACER: u8 = 10;
const OTHER_TOKEN: i32 = 0x1800000; // OTHER_CHAR << 21
const CALL: u8 = 113;
const END_TEMPLATE: u8 = 117;
const ENDV: u8 = 9;
// FROZEN_CONTROL_SEQUENCE + 6 (xetex_format.h:201, xetex-constants.h:61)
const FROZEN_ENDV: i32 = 2_243_226 + 6;
// END_MATCH_TOKEN + 1 (xetex-constants.h:360–361)
const PROTECTED_TOKEN: i32 = 0x1C00001;

/// translated from xetex-xetex0.c:4316 (x_token)
#[no_mangle]
pub unsafe extern "C" fn x_token() {
    while cur_cmd() > MAX_COMMAND {
        expand();
        get_next();
    }
    if cur_cs() == 0 {
        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
    } else {
        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
    }
}

// Token-encoding constants (xetex-constants.h:353).
const MATH_SHIFT_TOKEN: i32 = 0x600000;

/// translated from xetex-xetex0.c:11444 (insert_dollar_sign)
#[no_mangle]
pub unsafe extern "C" fn insert_dollar_sign() {
    crate::input::back_input();
    set_cur_tok(MATH_SHIFT_TOKEN + '$' as i32);
    crate::output::error_here_with_diagnostic(c"Missing $ inserted".as_ptr());
    crate::output::capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(2);
    set_help_line(1, c"I've inserted a begin-math/end-math symbol since I think".as_ptr());
    set_help_line(0, c"you left one out. Proceed, with fingers crossed.".as_ptr());
    crate::input::ins_error();
}

/// translated from xetex-xetex0.c:4355 (scan_optional_equals)
#[no_mangle]
pub unsafe extern "C" fn scan_optional_equals() {
    loop {
        get_x_token();
        if cur_cmd() != SPACER {
            break;
        }
    }
    if cur_tok() != OTHER_TOKEN + '=' as i32 {
        crate::input::back_input();
    }
}

// Command-code values for scan_left_brace (xetex_format.h).
const RELAX: u8 = 0;
const LEFT_BRACE: u8 = 1;
const OTHER_CHAR: u8 = 12;
const LEFT_BRACE_TOKEN: i32 = 0x200000;

/// translated from xetex-xetex0.c:4319 (scan_left_brace)
#[no_mangle]
pub unsafe extern "C" fn scan_left_brace() {
    loop {
        get_x_token();
        if cur_cmd() != SPACER && cur_cmd() != RELAX {
            break;
        }
    }
    if cur_cmd() != LEFT_BRACE {
        crate::output::error_here_with_diagnostic(c"Missing { inserted".as_ptr());
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(4);
        set_help_line(3, c"A left brace was mandatory here, so I've put one in.".as_ptr());
        set_help_line(2, c"You might want to delete and/or insert some corrections".as_ptr());
        set_help_line(1, c"so that I will find a matching right brace soon.".as_ptr());
        set_help_line(0, c"(If you're confused by all this, try typing `I}' now.)".as_ptr());
        crate::input::back_error();
        set_cur_tok(LEFT_BRACE_TOKEN + '{' as i32);
        set_cur_cmd(LEFT_BRACE);
        set_cur_chr('{' as i32);
        set_align_state(align_state() + 1);
    }
}

// ---------------------------------------------------------------------------
// get_x_token / get_x_or_protected
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:4272 (get_x_token)
#[no_mangle]
pub unsafe extern "C" fn get_x_token() {
    loop {
        get_next();
        if cur_cmd() <= MAX_COMMAND {
            break;
        }
        if cur_cmd() >= CALL {
            if cur_cmd() < END_TEMPLATE {
                macro_call();
            } else {
                set_cur_cs(FROZEN_ENDV);
                set_cur_cmd(ENDV);
                break;
            }
        } else {
            expand();
        }
    }
    if cur_cs() == 0 {
        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
    } else {
        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
    }
}

/// translated from xetex-xetex0.c:4533 (get_x_or_protected)
#[no_mangle]
pub unsafe extern "C" fn get_x_or_protected() {
    loop {
        get_token();
        if cur_cmd() <= MAX_COMMAND {
            return;
        }
        if cur_cmd() >= CALL && cur_cmd() < END_TEMPLATE {
            if mem_b32s0(mem_b32s1(cur_chr())) == PROTECTED_TOKEN {
                return;
            }
        }
        expand();
    }
}

// ---------------------------------------------------------------------------
// scan_keyword
// ---------------------------------------------------------------------------

// MEM_TOP - 13 (xetex_format.h:30, xetex-constants.h:50)
const BACKUP_HEAD: i32 = 4_999_999 - 13;
// xetex-constants.h:202
const BACKED_UP: u16 = 3;

/// translated from xetex-xetex0.c:4284 (scan_keyword)
#[no_mangle]
pub unsafe extern "C" fn scan_keyword(s: *const libc::c_char) -> bool {
    let mut p = BACKUP_HEAD;
    set_mem_b32s1(p, crate::mem::TEX_NULL);

    let slen = libc::strlen(s);

    if slen == 1 {
        let c = *s as i32;
        let save_cur_cs = cur_cs();
        loop {
            get_x_token();
            if cur_cs() == 0 && (cur_chr() == c || cur_chr() == c - 32) {
                let q = crate::mem::get_avail();
                set_mem_b32s1(p, q);
                set_mem_b32s0(q, cur_tok());
                p = q;
                crate::mem::flush_list(mem_b32s1(BACKUP_HEAD));
                return true;
            } else if cur_cmd() != SPACER || p != BACKUP_HEAD {
                back_input();
                if p != BACKUP_HEAD {
                    begin_token_list(mem_b32s1(BACKUP_HEAD), BACKED_UP);
                }
                set_cur_cs(save_cur_cs);
                return false;
            }
        }
    }

    let mut i = 0usize;
    while i < slen {
        get_x_token();
        let ch = *s.add(i) as i32;
        if cur_cs() == 0 && (cur_chr() == ch || cur_chr() == ch - 32) {
            let q = crate::mem::get_avail();
            set_mem_b32s1(p, q);
            set_mem_b32s0(q, cur_tok());
            p = q;
            i += 1;
        } else if cur_cmd() != SPACER || p != BACKUP_HEAD {
            back_input();
            if p != BACKUP_HEAD {
                begin_token_list(mem_b32s1(BACKUP_HEAD), BACKED_UP);
            }
            return false;
        }
    }
    crate::mem::flush_list(mem_b32s1(BACKUP_HEAD));
    true
}

// ---------------------------------------------------------------------------
// scan_expr
// ---------------------------------------------------------------------------

// xetex-constants.h:121,145–150,182; xetex_format.h:568–570; xetex-constants.h:12,15,288
const EXPR_NONE: i16 = 0;
const EXPR_ADD: i16 = 1;
const EXPR_SUB: i16 = 2;
const EXPR_MULT: i16 = 3;
const EXPR_DIV: i16 = 4;
const EXPR_SCALE: i16 = 5;
const EXPR_NODE_SIZE: i32 = 4;
const GLUE_VAL_EXPR: i16 = 2;
const MU_VAL_EXPR: i16 = 3;
const NORMAL_EXPR: u16 = 0;
const TEX_INFINITY: i32 = 0x7FFF_FFFF;
const MAX_HALFWORD: i32 = 0x3FFF_FFFF;

/// translated from xetex-xetex0.c:6025 (scan_expr)
#[no_mangle]
pub unsafe extern "C" fn scan_expr() {
    let mut l = cur_val_level() as i16;
    let a = arith_error();
    let mut b = false;
    let mut p = crate::mem::TEX_NULL;

    set_expand_depth_count(expand_depth_count() + 1);
    if expand_depth_count() >= expand_depth() {
        crate::errors::overflow(c"expansion depth".as_ptr(), expand_depth());
    }

    // restart:
    let mut r: i16;
    let mut e: i32;
    let mut s: i16;
    let mut t: i32;
    let mut n: i32;
    let mut f: i32 = 0;
    let mut skip_scan = false; // true means "goto found" (skip value-scan phase)

    'outer: loop {
        // restart:
        r = EXPR_NONE;
        e = 0;
        s = EXPR_NONE;
        t = 0;
        n = 0;
        skip_scan = false;

        loop {
            // continue_: (only entered when !skip_scan)
            if !skip_scan {
                let o: i16 = if s == EXPR_NONE { l } else { INT_VAL };
                loop { get_x_token(); if cur_cmd() != SPACER { break; } }
                if cur_tok() == OTHER_TOKEN + 40 { // '('
                    let q = crate::mem::get_node(EXPR_NODE_SIZE);
                    set_mem_b32s1(q, p);
                    set_mem_b16s1(q, l as u16);
                    set_mem_b16s0(q, (4 * s + r) as u16);
                    set_mem_b32s1(q + 1, e);
                    set_mem_b32s1(q + 2, t);
                    set_mem_b32s1(q + 3, n);
                    p = q;
                    l = o;
                    continue 'outer; // goto restart
                }
                back_input();
                if o == INT_VAL { scan_int(); }
                else if o == DIMEN_VAL { scan_dimen(false, false, false); }
                else if o == GLUE_VAL_EXPR { scan_normal_glue(); }
                else { scan_mu_glue(); }
                f = cur_val();
            }
            skip_scan = false;

            // found:
            loop { get_x_token(); if cur_cmd() != SPACER { break; } }
            let mut o: i16 = if cur_tok() == OTHER_TOKEN + 43 { EXPR_ADD }
                else if cur_tok() == OTHER_TOKEN + 45 { EXPR_SUB }
                else if cur_tok() == OTHER_TOKEN + 42 { EXPR_MULT }
                else if cur_tok() == OTHER_TOKEN + 47 { EXPR_DIV }
                else { EXPR_NONE };
            if o == EXPR_NONE {
                if p == crate::mem::TEX_NULL {
                    if cur_cmd() != RELAX { back_input(); }
                } else if cur_tok() != OTHER_TOKEN + 41 { // ')'
                    crate::output::error_here_with_diagnostic(
                        c"Missing ) inserted for expression".as_ptr());
                    crate::output::capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(1);
                    set_help_line(0,
                        c"I was expecting to see `+', `-', `*', `/', or `)'. Didn't.".as_ptr());
                    crate::input::back_error();
                }
            }

            set_arith_error(b);
            if l == INT_VAL || s > EXPR_SUB {
                if f > TEX_INFINITY || f < -TEX_INFINITY {
                    set_arith_error(true);
                    f = 0;
                }
            } else if l == DIMEN_VAL {
                if f.abs() > MAX_HALFWORD {
                    set_arith_error(true);
                    f = 0;
                }
            } else {
                if mem_b32s1(f + 1).abs() > MAX_HALFWORD
                    || mem_b32s1(f + 2).abs() > MAX_HALFWORD
                    || mem_b32s1(f + 3).abs() > MAX_HALFWORD
                {
                    set_arith_error(true);
                    crate::mem::delete_glue_ref(f);
                    f = crate::mem::new_spec(0);
                }
            }

            // o may be modified inside case 3 (EXPR_MULT → EXPR_SCALE)
            match s {
                0 => {
                    if l >= GLUE_VAL_EXPR && o != EXPR_NONE {
                        t = crate::mem::new_spec(f);
                        crate::mem::delete_glue_ref(f);
                        if mem_b32s1(t + 2) == 0 { set_mem_b16s1(t, NORMAL_EXPR); }
                        if mem_b32s1(t + 3) == 0 { set_mem_b16s0(t, NORMAL_EXPR); }
                    } else {
                        t = f;
                    }
                }
                3 => { // EXPR_MULT
                    if o == EXPR_DIV {
                        n = f;
                        o = EXPR_SCALE; // modifies o for post-match code
                    } else if l == INT_VAL {
                        t = crate::scaled_math::mult_and_add(t, f, 0, TEX_INFINITY);
                    } else if l == DIMEN_VAL {
                        t = crate::scaled_math::mult_and_add(t, f, 0, MAX_HALFWORD);
                    } else {
                        set_mem_b32s1(t + 1, crate::scaled_math::mult_and_add(mem_b32s1(t + 1), f, 0, MAX_HALFWORD));
                        set_mem_b32s1(t + 2, crate::scaled_math::mult_and_add(mem_b32s1(t + 2), f, 0, MAX_HALFWORD));
                        set_mem_b32s1(t + 3, crate::scaled_math::mult_and_add(mem_b32s1(t + 3), f, 0, MAX_HALFWORD));
                    }
                }
                4 => { // EXPR_DIV
                    if l < GLUE_VAL_EXPR {
                        t = crate::scaled_math::quotient(t, f);
                    } else {
                        set_mem_b32s1(t + 1, crate::scaled_math::quotient(mem_b32s1(t + 1), f));
                        set_mem_b32s1(t + 2, crate::scaled_math::quotient(mem_b32s1(t + 2), f));
                        set_mem_b32s1(t + 3, crate::scaled_math::quotient(mem_b32s1(t + 3), f));
                    }
                }
                5 => { // EXPR_SCALE
                    if l == INT_VAL {
                        t = crate::scaled_math::fract(t, n, f, TEX_INFINITY);
                    } else if l == DIMEN_VAL {
                        t = crate::scaled_math::fract(t, n, f, MAX_HALFWORD);
                    } else {
                        set_mem_b32s1(t + 1, crate::scaled_math::fract(mem_b32s1(t + 1), n, f, MAX_HALFWORD));
                        set_mem_b32s1(t + 2, crate::scaled_math::fract(mem_b32s1(t + 2), n, f, MAX_HALFWORD));
                        set_mem_b32s1(t + 3, crate::scaled_math::fract(mem_b32s1(t + 3), n, f, MAX_HALFWORD));
                    }
                }
                _ => {}
            }

            if o > EXPR_SUB {
                s = o;
            } else {
                s = EXPR_NONE;
                if r == EXPR_NONE {
                    e = t;
                } else if l == INT_VAL {
                    e = crate::scaled_math::add_or_sub(e, t, TEX_INFINITY, r == EXPR_SUB);
                } else if l == DIMEN_VAL {
                    e = crate::scaled_math::add_or_sub(e, t, MAX_HALFWORD, r == EXPR_SUB);
                } else {
                    set_mem_b32s1(e + 1, crate::scaled_math::add_or_sub(mem_b32s1(e + 1), mem_b32s1(t + 1), MAX_HALFWORD, r == EXPR_SUB));
                    if mem_b16s1(e) == mem_b16s1(t) {
                        set_mem_b32s1(e + 2, crate::scaled_math::add_or_sub(mem_b32s1(e + 2), mem_b32s1(t + 2), MAX_HALFWORD, r == EXPR_SUB));
                    } else if mem_b16s1(e) < mem_b16s1(t) && mem_b32s1(t + 2) != 0 {
                        set_mem_b32s1(e + 2, mem_b32s1(t + 2));
                        set_mem_b16s1(e, mem_b16s1(t));
                    }
                    if mem_b16s0(e) == mem_b16s0(t) {
                        set_mem_b32s1(e + 3, crate::scaled_math::add_or_sub(mem_b32s1(e + 3), mem_b32s1(t + 3), MAX_HALFWORD, r == EXPR_SUB));
                    } else if mem_b16s0(e) < mem_b16s0(t) && mem_b32s1(t + 3) != 0 {
                        set_mem_b32s1(e + 3, mem_b32s1(t + 3));
                        set_mem_b16s0(e, mem_b16s0(t));
                    }
                    crate::mem::delete_glue_ref(t);
                    if mem_b32s1(e + 2) == 0 { set_mem_b16s1(e, NORMAL_EXPR); }
                    if mem_b32s1(e + 3) == 0 { set_mem_b16s0(e, NORMAL_EXPR); }
                }
                r = o;
            }
            b = arith_error();
            if o != EXPR_NONE {
                continue; // goto continue_
            }
            if p != crate::mem::TEX_NULL {
                f = e;
                let q = p;
                e = mem_b32s1(q + 1);
                t = mem_b32s1(q + 2);
                n = mem_b32s1(q + 3);
                s = (mem_b16s0(q) / 4) as i16;
                r = (mem_b16s0(q) % 4) as i16;
                l = mem_b16s1(q) as i16;
                p = mem_b32s1(q);
                crate::mem::free_node(q, EXPR_NODE_SIZE);
                skip_scan = true;
                continue; // goto found
            }
            break 'outer;
        }
    }

    set_expand_depth_count(expand_depth_count() - 1);

    if b {
        crate::output::error_here_with_diagnostic(c"Arithmetic overflow".as_ptr());
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, c"I can't evaluate this expression,".as_ptr());
        set_help_line(0, c"since the result is out of range.".as_ptr());
        crate::errors::error();
        if l >= GLUE_VAL_EXPR {
            crate::mem::delete_glue_ref(e);
            e = 0;
            set_mem_b32s1(e, mem_b32s1(e) + 1); // GLUE_SPEC_ref_count(e)++
        } else {
            e = 0;
        }
    }
    set_arith_error(a);
    set_cur_val(e);
    set_cur_val_level(l as u8);
}

// ---------------------------------------------------------------------------
// scan_general_text
// ---------------------------------------------------------------------------

// xetex-constants.h:187
const ABSORBING: u8 = 5;
// xetex_format.h:232
const RIGHT_BRACE: u8 = 2;
// xetex-constants.h:352
const RIGHT_BRACE_LIMIT: i32 = 0x600000;
// MEM_TOP - 3 (xetex-constants.h:40)
const TEMP_HEAD: i32 = 4_999_999 - 3;

/// translated from xetex-xetex0.c:6262 (scan_general_text)
#[no_mangle]
pub unsafe extern "C" fn scan_general_text() {
    let s = scanner_status();
    let w = warning_index();
    let d = def_ref();
    set_scanner_status(ABSORBING);
    set_warning_index(cur_cs());
    let new_def = crate::mem::get_avail();
    set_def_ref(new_def);
    set_mem_b32s0(new_def, crate::mem::TEX_NULL);
    let mut p = new_def;
    scan_left_brace();
    let mut unbalance: i32 = 1;
    loop {
        get_token();
        if cur_tok() < RIGHT_BRACE_LIMIT {
            if cur_cmd() < RIGHT_BRACE {
                unbalance += 1;
            } else {
                unbalance -= 1;
                if unbalance == 0 {
                    break; // found:
                }
            }
        }
        let q = crate::mem::get_avail();
        set_mem_b32s1(p, q);
        set_mem_b32s0(q, cur_tok());
        p = q;
    }
    // found:
    let q = mem_b32s1(def_ref());
    set_mem_b32s1(def_ref(), avail());
    set_avail(def_ref());
    if q == crate::mem::TEX_NULL {
        set_cur_val(TEMP_HEAD);
    } else {
        set_cur_val(p);
    }
    set_mem_b32s1(TEMP_HEAD, q);
    set_scanner_status(s);
    set_warning_index(w);
    set_def_ref(d);
}

// ---------------------------------------------------------------------------
// scan_spec
// ---------------------------------------------------------------------------

// xetex-constants.h:256,264
const EXACTLY: u8 = 0;
const ADDITIONAL: u8 = 1;

/// translated from xetex-xetex0.c:9215 (scan_spec)
#[no_mangle]
pub unsafe extern "C" fn scan_spec(c: u8, three_codes: bool) {
    let s = if three_codes { save_stack_b32s1(save_ptr()) } else { 0 };
    let spec_code;
    if scan_keyword(c"to".as_ptr()) {
        spec_code = EXACTLY;
        scan_dimen(false, false, false);
    } else if scan_keyword(c"spread".as_ptr()) {
        spec_code = ADDITIONAL;
        scan_dimen(false, false, false);
    } else {
        spec_code = ADDITIONAL;
        set_cur_val(0);
    }
    // found:
    if three_codes {
        set_save_stack_b32s1(save_ptr(), s);
        set_save_ptr(save_ptr() + 1);
    }
    set_save_stack_b32s1(save_ptr(), spec_code as i32);
    set_save_stack_b32s1(save_ptr() + 1, cur_val());
    set_save_ptr(save_ptr() + 2);
    crate::save::new_save_level(c);
    scan_left_brace();
}

// ---------------------------------------------------------------------------
// round_decimals / xetex_scan_dimen / scan_glyph_number
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:4392 (round_decimals, was static)
unsafe fn round_decimals(k: i32) -> i32 {
    use crate::globals::dig_entry;
    let mut a: i32 = 0;
    let mut k = k;
    while k > 0 {
        k -= 1;
        a = (a + dig_entry(k as usize) as i32 * 0x2_0000) / 10;
    }
    (a + 1) / 2
}

/// translated from xetex-xetex0.c:4406 (xetex_scan_dimen)
#[no_mangle]
pub unsafe extern "C" fn xetex_scan_dimen(mu: bool, inf: bool, shortcut: bool, requires_units: bool) {
    use crate::globals::{
        cur_order, set_cur_order, radix, set_radix,
        set_dig_entry, tex_remainder, font_info_b32s1,
    };
    const NORMAL_ORDER: u8 = 0;
    const FIL_ORDER: u8 = 1;
    const FILLL_ORDER: u8 = 3;
    const QUAD_CODE: i32 = 6;       // xetex-constants.h:298
    const X_HEIGHT_CODE: i32 = 5;  // xetex-constants.h:294
    const INT_PAR_MAG: i32 = 17;   // xetex_format.h:52
    const MINUS_TOKEN: i32 = OTHER_TOKEN + b'-' as i32;
    const PLUS_TOKEN: i32 = OTHER_TOKEN + b'+' as i32;

    let mut f: i32 = 0;
    let mut negative = false;

    set_arith_error(false);
    set_cur_order(NORMAL_ORDER);

    'body: {
        if !shortcut {
            // Skip sign-prefix tokens
            loop {
                loop { get_x_token(); if cur_cmd() != SPACER { break; } }
                if cur_tok() == MINUS_TOKEN {
                    negative = !negative;
                    set_cur_tok(PLUS_TOKEN);
                }
                if cur_tok() != PLUS_TOKEN { break; }
            }

            if cur_cmd() >= MIN_INTERNAL && cur_cmd() <= MAX_INTERNAL {
                if mu {
                    scan_something_internal(MU_VAL, false);
                    if cur_val_level() as i16 >= GLUE_VAL {
                        let v = mem_b32s1(cur_val() + 1);
                        crate::mem::delete_glue_ref(cur_val());
                        set_cur_val(v);
                    }
                    if cur_val_level() as i16 == MU_VAL { break 'body; } // goto attach_sign
                    if cur_val_level() as i16 != INT_VAL { mu_error(); }
                } else {
                    scan_something_internal(DIMEN_VAL, false);
                    if cur_val_level() as i16 == DIMEN_VAL { break 'body; } // goto attach_sign
                }
            } else {
                back_input();
                if cur_tok() == CONTINENTAL_POINT_TOKEN { set_cur_tok(POINT_TOKEN); }
                if cur_tok() != POINT_TOKEN {
                    scan_int();
                } else {
                    set_radix(10);
                    set_cur_val(0);
                }
                if cur_tok() == CONTINENTAL_POINT_TOKEN { set_cur_tok(POINT_TOKEN); }

                if radix() == 10 && cur_tok() == POINT_TOKEN {
                    let mut k: i32 = 0;
                    let mut p: i32 = crate::mem::TEX_NULL;
                    get_token();
                    'decimal: loop {
                        get_x_token();
                        if cur_tok() > ZERO_TOKEN + 9 || cur_tok() < ZERO_TOKEN {
                            break 'decimal;
                        }
                        if k < 17 {
                            let q = crate::mem::get_avail();
                            set_mem_b32s1(q, p);
                            set_mem_b32s0(q, cur_tok() - ZERO_TOKEN);
                            p = q;
                            k += 1;
                        }
                    }
                    let mut kk = k;
                    while kk >= 1 {
                        set_dig_entry((kk - 1) as usize, mem_b32s0(p) as u8);
                        let old_p = p;
                        p = mem_b32s1(p);
                        set_mem_b32s1(old_p, avail());
                        set_avail(old_p);
                        kk -= 1;
                    }
                    f = round_decimals(k);
                    if cur_cmd() != SPACER { back_input(); }
                }
            }
        }

        if cur_val() < 0 { negative = !negative; set_cur_val(-cur_val()); }

        if requires_units {
            let (apply_fraction, read_spacer) = 'units: {
                // inf glue order
                if inf && scan_keyword(c"fil".as_ptr()) {
                    set_cur_order(FIL_ORDER);
                    while scan_keyword(c"l".as_ptr()) {
                        if cur_order() == FILLL_ORDER {
                            error_here_with_diagnostic(
                                c"Illegal unit of measure (replaced with filll)".as_ptr(),
                            );
                            capture_to_diagnostic(std::ptr::null_mut());
                            set_help_ptr(1);
                            set_help_line(0, c"I dddon't go any higher than filll.".as_ptr());
                            error();
                        } else {
                            set_cur_order(cur_order() + 1);
                        }
                    }
                    break 'units (true, true); // attach_fraction
                }

                let save_cv = cur_val();
                loop { get_x_token(); if cur_cmd() != SPACER { break; } }

                if cur_cmd() >= MIN_INTERNAL && cur_cmd() <= MAX_INTERNAL {
                    if mu {
                        scan_something_internal(MU_VAL, false);
                        if cur_val_level() as i16 >= GLUE_VAL {
                            let v = mem_b32s1(cur_val() + 1);
                            crate::mem::delete_glue_ref(cur_val());
                            set_cur_val(v);
                        }
                        if cur_val_level() as i16 != MU_VAL { mu_error(); }
                    } else {
                        scan_something_internal(DIMEN_VAL, false);
                    }
                    let v = cur_val();
                    set_cur_val(crate::scaled_math::mult_and_add(
                        save_cv, v,
                        crate::scaled_math::xn_over_d(v, f, 65536),
                        MAX_HALFWORD,
                    ));
                    break 'body; // goto attach_sign (found:)
                }

                back_input();

                // em/ex lookup (not available for mu)
                let em_ex = if !mu {
                    if scan_keyword(c"em".as_ptr()) {
                        let fi = eqtb_b32s1(CUR_FONT_LOC);
                        Some(font_info_b32s1(QUAD_CODE + param_base(fi)))
                    } else if scan_keyword(c"ex".as_ptr()) {
                        let fi = eqtb_b32s1(CUR_FONT_LOC);
                        Some(font_info_b32s1(X_HEIGHT_CODE + param_base(fi)))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(v) = em_ex {
                    get_x_token();
                    if cur_cmd() != SPACER { back_input(); }
                    set_cur_val(crate::scaled_math::mult_and_add(
                        save_cv, v,
                        crate::scaled_math::xn_over_d(v, f, 65536),
                        MAX_HALFWORD,
                    ));
                    break 'body; // goto attach_sign (found:)
                }

                // not_found:
                if mu {
                    if scan_keyword(c"mu".as_ptr()) {
                        break 'units (true, true); // attach_fraction
                    } else {
                        error_here_with_diagnostic(
                            c"Illegal unit of measure (mu inserted)".as_ptr(),
                        );
                        capture_to_diagnostic(std::ptr::null_mut());
                        set_help_ptr(4);
                        set_help_line(3, c"The unit of measurement in math glue must be mu.".as_ptr());
                        set_help_line(2, c"To recover gracefully from this error, it's best to".as_ptr());
                        set_help_line(1, c"delete the erroneous units; e.g., type `2' to delete".as_ptr());
                        set_help_line(0, c"two letters. (See Chapter 27 of The TeXbook.)".as_ptr());
                        error();
                        break 'units (true, true); // attach_fraction
                    }
                }

                if scan_keyword(c"true".as_ptr()) {
                    crate::save::prepare_mag();
                    let mag = eqtb_b32s1(INT_BASE + INT_PAR_MAG);
                    if mag != 1000 {
                        set_cur_val(crate::scaled_math::xn_over_d(cur_val(), 1000, mag));
                        f = (1000 * f + 65536 * tex_remainder()) / mag;
                        set_cur_val(cur_val() + f / 65536);
                        f = f % 65536;
                    }
                }

                if scan_keyword(c"pt".as_ptr()) { break 'units (true, true); } // attach_fraction

                let (num, denom) = if scan_keyword(c"in".as_ptr()) {
                    (7227_i32, 100_i32)
                } else if scan_keyword(c"pc".as_ptr()) {
                    (12, 1)
                } else if scan_keyword(c"cm".as_ptr()) {
                    (7227, 254)
                } else if scan_keyword(c"mm".as_ptr()) {
                    (7227, 2540)
                } else if scan_keyword(c"bp".as_ptr()) {
                    (7227, 7200)
                } else if scan_keyword(c"dd".as_ptr()) {
                    (1238, 1157)
                } else if scan_keyword(c"cc".as_ptr()) {
                    (14856, 1157)
                } else if scan_keyword(c"sp".as_ptr()) {
                    break 'units (false, true); // done: skip fraction
                } else {
                    error_here_with_diagnostic(
                        c"Illegal unit of measure (pt inserted)".as_ptr(),
                    );
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(6);
                    set_help_line(5, c"Dimensions can be in units of em, ex, in, pt, pc,".as_ptr());
                    set_help_line(4, c"cm, mm, dd, cc, bp, or sp; but yours is a new one!".as_ptr());
                    set_help_line(3, c"I'll assume that you meant to say pt, for printer's points.".as_ptr());
                    set_help_line(2, c"To recover gracefully from this error, it's best to".as_ptr());
                    set_help_line(1, c"delete the erroneous units; e.g., type `2' to delete".as_ptr());
                    set_help_line(0, c"two letters. (See Chapter 27 of The TeXbook.)".as_ptr());
                    error();
                    break 'units (true, true); // done2 → attach_fraction
                };

                set_cur_val(crate::scaled_math::xn_over_d(cur_val(), num, denom));
                f = (num * f + 65536 * tex_remainder()) / denom;
                set_cur_val(cur_val() + f / 65536);
                f = f % 65536;

                (true, true) // attach_fraction
            };

            if apply_fraction {
                if cur_val() >= 16384 { set_arith_error(true); }
                else { set_cur_val(cur_val() * 65536 + f); }
            }
            if read_spacer {
                get_x_token();
                if cur_cmd() != SPACER { back_input(); }
            }
        } else {
            // !requires_units
            if cur_val() >= 16384 { set_arith_error(true); }
            else { set_cur_val(cur_val() * 65536 + f); }
        }
    } // end 'body

    // attach_sign:
    if arith_error() || cur_val().abs() >= 0x4000_0000 {
        error_here_with_diagnostic(c"Dimension too large".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, c"I can't work with sizes bigger than about 19 feet.".as_ptr());
        set_help_line(0, c"Continue and I'll use the largest value I can.".as_ptr());
        error();
        set_cur_val(MAX_HALFWORD);
        set_arith_error(false);
    }
    if negative { set_cur_val(-cur_val()); }
}

/// translated from xetex-xetex0.c:4302 (scan_glyph_number)
#[no_mangle]
pub unsafe extern "C" fn scan_glyph_number(f: i32) {
    if scan_keyword(c"/".as_ptr()) {
        scan_and_pack_name();
        set_cur_val(map_glyph_to_index(f));
        set_cur_val_level(INT_VAL as u8);
    } else if scan_keyword(c"u".as_ptr()) {
        scan_char_num();
        set_cur_val(map_char_to_glyph(f, cur_val()));
        set_cur_val_level(INT_VAL as u8);
    } else {
        scan_int();
    }
}

// ---------------------------------------------------------------------------
// scan_dimen / scan_decimal
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:5956 (scan_dimen)
#[no_mangle]
pub unsafe extern "C" fn scan_dimen(mu: bool, inf: bool, shortcut: bool) {
    xetex_scan_dimen(mu, inf, shortcut, true);
}

/// translated from xetex-xetex0.c:5961 (scan_decimal)
#[no_mangle]
pub unsafe extern "C" fn scan_decimal() {
    xetex_scan_dimen(false, false, false, false);
}

// ---------------------------------------------------------------------------
// scan_rule_spec
// ---------------------------------------------------------------------------

// xetex_format.h:273 / xetex-constants.h:316,339
const VRULE: u8 = 35;
const DEFAULT_RULE: i32 = 26214;

/// translated from xetex-xetex0.c:6267 (scan_rule_spec)
#[no_mangle]
pub unsafe extern "C" fn scan_rule_spec() -> i32 {
    let q = new_rule();
    if cur_cmd() == VRULE {
        set_mem_b32s1(q + 1, DEFAULT_RULE);
    } else {
        set_mem_b32s1(q + 3, DEFAULT_RULE);
        set_mem_b32s1(q + 2, 0);
    }
    loop {
        if scan_keyword(c"width".as_ptr()) {
            xetex_scan_dimen(false, false, false, true);
            set_mem_b32s1(q + 1, cur_val());
        } else if scan_keyword(c"height".as_ptr()) {
            xetex_scan_dimen(false, false, false, true);
            set_mem_b32s1(q + 3, cur_val());
        } else if scan_keyword(c"depth".as_ptr()) {
            xetex_scan_dimen(false, false, false, true);
            set_mem_b32s1(q + 2, cur_val());
        } else {
            break;
        }
    }
    q
}

#[inline]
unsafe fn back_input() {
    crate::input::back_input();
}

// ---------------------------------------------------------------------------
// ins_the_toks
// ---------------------------------------------------------------------------

// MEM_TOP - 12 (xetex-constants.h:49)
const GARBAGE: i32 = 4_999_999 - 12;
// xetex-constants.h:204
const INSERTED: u16 = 5;

/// translated from xetex-xetex0.c:6511 (ins_the_toks)
#[no_mangle]
pub unsafe extern "C" fn ins_the_toks() {
    set_mem_b32s1(GARBAGE, the_toks());
    begin_token_list(mem_b32s1(TEMP_HEAD), INSERTED);
}

// ---------------------------------------------------------------------------
// pseudo_start
// ---------------------------------------------------------------------------

// INT_BASE = 7_826_729 (xetex_format.h); INT_PAR__new_line_char = 49
const INT_BASE_PSEUDO: i32 = 7_826_729;
const INT_PAR_NEW_LINE_CHAR: i32 = 49;    // xetex_format.h:84
const INT_PAR_TRACING_SCAN_TOKENS: i32 = 59; // xetex_format.h:94
const TOO_BIG_CHAR: i32 = 65536; // xetex-constants.h (str_number offset)
// xetex-core.h / xetex-xetexd.h:430
const SELECTOR_NEW_STRING: i32 = 21;

/// translated from xetex-xetex0.c:6263 (pseudo_start)
#[no_mangle]
pub unsafe extern "C" fn pseudo_start() {
    scan_general_text();
    let old_setting = selector();
    set_selector(SELECTOR_NEW_STRING);
    crate::output::token_show(TEMP_HEAD);
    set_selector(old_setting);
    crate::mem::flush_list(mem_b32s1(TEMP_HEAD));

    if pool_ptr() + 1 > pool_size() {
        crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr());
    }
    let s = crate::string_pool::make_string();
    set_str_pool(pool_ptr(), b' ' as u16);
    let mut l = str_start(s - TOO_BIG_CHAR);
    let nl = eqtb_b32s1(INT_BASE_PSEUDO + INT_PAR_NEW_LINE_CHAR);
    let p = crate::mem::get_avail();
    let mut q = p;

    while l < pool_ptr() {
        let m = l;
        while l < pool_ptr() && str_pool_entry(l) != nl as u16 {
            l += 1;
        }
        let mut sz = (l - m + 7) / 4;
        if sz == 1 {
            sz = 2;
        }
        let r_base = crate::mem::get_node(sz);
        set_mem_b32s1(q, r_base);
        q = r_base;
        set_mem_b32s0(q, sz);
        let mut r = r_base;
        let mut mm = m;
        while sz > 2 {
            sz -= 1;
            r += 1;
            let s3 = str_pool_entry(mm);
            let s2 = str_pool_entry(mm + 1);
            let s1 = str_pool_entry(mm + 2);
            let s0 = str_pool_entry(mm + 3);
            set_mem_b16s3(r, s3);
            set_mem_b16s2(r, s2);
            set_mem_b16s1(r, s1);
            set_mem_b16s0(r, s0);
            mm += 4;
        }
        // last word: pad with spaces, then fill from remaining pool bytes
        let mut w3: u16 = b' ' as u16;
        let mut w2: u16 = b' ' as u16;
        let mut w1: u16 = b' ' as u16;
        let mut w0: u16 = b' ' as u16;
        if l > mm {
            w3 = str_pool_entry(mm);
            if l > mm + 1 {
                w2 = str_pool_entry(mm + 1);
                if l > mm + 2 {
                    w1 = str_pool_entry(mm + 2);
                    if l > mm + 3 {
                        w0 = str_pool_entry(mm + 3);
                    }
                }
            }
        }
        set_mem_b16s3(r + 1, w3);
        set_mem_b16s2(r + 1, w2);
        set_mem_b16s1(r + 1, w1);
        set_mem_b16s0(r + 1, w0);
        if str_pool_entry(l) == nl as u16 {
            l += 1;
        }
    }

    set_mem_b32s0(p, mem_b32s1(p));
    set_mem_b32s1(p, pseudo_files());
    set_pseudo_files(p);
    // pop the temporary string (str_ptr--, pool_ptr = str_start[str_ptr - TOO_BIG_CHAR])
    set_str_ptr(str_ptr() - 1);
    set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));

    crate::input::begin_file_reading();
    set_line(0);
    let start = cur_input().start;
    set_cur_input_limit(start);
    set_cur_input_loc(start + 1);

    if eqtb_b32s1(INT_BASE_PSEUDO + INT_PAR_TRACING_SCAN_TOKENS) > 0 {
        if term_offset() > max_print_line() - 3 {
            crate::output::print_ln();
        } else if term_offset() > 0 || file_offset() > 0 {
            crate::output::print_char(b' ' as i32);
        }
        set_cur_input_name(19);
        crate::output::print_cstr(c"( ".as_ptr());
        set_open_parens(open_parens() + 1);
        ttstub_output_flush(rust_stdout());
    } else {
        set_cur_input_name(18);
        set_cur_input_synctex_tag(0);
    }
}

// ---------------------------------------------------------------------------
// str_toks_cat / str_toks
// ---------------------------------------------------------------------------

// xetex-constants.h:356
const SPACE_TOKEN: i32 = 0x1400020;
// xetex_format.h:246
const ACTIVE_CHAR: i32 = 13;

/// translated from xetex-xetex0.c:6363 (str_toks_cat)
#[no_mangle]
pub unsafe extern "C" fn str_toks_cat(b: i32, cat: i16) -> i32 {
    use crate::mem::TEX_NULL;

    if pool_ptr() + 1 > pool_size() {
        crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr());
    }

    let mut p = TEMP_HEAD;
    set_mem_b32s1(p, TEX_NULL);
    let mut k = b;

    while k < pool_ptr() {
        let mut t = str_pool_entry(k) as i32;

        if t == b' ' as i32 && cat == 0 {
            t = SPACE_TOKEN;
        } else {
            // decode UTF-16 surrogate pair
            if t >= 0xD800 && t < 0xDC00 && k + 1 < pool_ptr() {
                let lo = str_pool_entry(k + 1) as i32;
                if lo >= 0xDC00 && lo < 0xE000 {
                    k += 1;
                    t = 0x10000 + ((t - 0xD800) << 10) + (str_pool_entry(k) as i32 - 0xDC00);
                }
            }

            if cat == 0 {
                t = OTHER_TOKEN + t;
            } else if cat == ACTIVE_CHAR as i16 {
                t = CS_TOKEN_FLAG + 1 + t;
            } else {
                t = MAX_CHAR_VAL * cat as i32 + t;
            }
        }

        let q = if avail() == TEX_NULL {
            crate::mem::get_avail()
        } else {
            let qa = avail();
            set_avail(mem_b32s1(qa));
            set_mem_b32s1(qa, TEX_NULL);
            qa
        };

        set_mem_b32s1(p, q);
        set_mem_b32s0(q, t);
        p = q;
        k += 1;
    }

    set_pool_ptr(b);
    p
}

/// translated from xetex-xetex0.c:6417 (str_toks)
#[no_mangle]
pub unsafe extern "C" fn str_toks(b: i32) -> i32 {
    str_toks_cat(b, 0)
}

// ---------------------------------------------------------------------------
// the_toks
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:6427 (the_toks)
#[no_mangle]
pub unsafe extern "C" fn the_toks() -> i32 {
    use crate::mem::TEX_NULL;

    if cur_chr() & 1 != 0 {
        let c = cur_chr();
        scan_general_text();
        if c == 1 {
            return cur_val();
        } else {
            let old_setting = selector();
            set_selector(SELECTOR_NEW_STRING);
            let b = pool_ptr();
            let p = crate::mem::get_avail();
            set_mem_b32s1(p, mem_b32s1(TEMP_HEAD));
            crate::output::token_show(p);
            crate::mem::flush_list(p);
            set_selector(old_setting);
            return str_toks(b);
        }
    }
    get_x_token();
    scan_something_internal(TOK_VAL, false);
    if cur_val_level() >= IDENT_VAL as u8 {
        let mut p = TEMP_HEAD;
        set_mem_b32s1(p, TEX_NULL);
        if cur_val_level() == IDENT_VAL as u8 {
            let q = crate::mem::get_avail();
            set_mem_b32s1(p, q);
            set_mem_b32s0(q, CS_TOKEN_FLAG + cur_val());
            p = q;
        } else if cur_val() != TEX_NULL {
            let mut r = mem_b32s1(cur_val());
            while r != TEX_NULL {
                let q = if avail() == TEX_NULL {
                    crate::mem::get_avail()
                } else {
                    let qa = avail();
                    set_avail(mem_b32s1(qa));
                    set_mem_b32s1(qa, TEX_NULL);
                    qa
                };
                set_mem_b32s1(p, q);
                set_mem_b32s0(q, mem_b32s0(r));
                p = q;
                r = mem_b32s1(r);
            }
        }
        return p;
    } else {
        let old_setting = selector();
        set_selector(SELECTOR_NEW_STRING);
        let b = pool_ptr();
        match cur_val_level() {
            0 => crate::output::print_int(cur_val()),
            1 => {
                crate::output::print_scaled(cur_val());
                crate::output::print_cstr(c"pt".as_ptr());
            }
            2 => {
                crate::output::print_spec(cur_val(), c"pt".as_ptr());
                crate::mem::delete_glue_ref(cur_val());
            }
            3 => {
                crate::output::print_spec(cur_val(), c"mu".as_ptr());
                crate::mem::delete_glue_ref(cur_val());
            }
            _ => {}
        }
        set_selector(old_setting);
        str_toks(b)
    }
}

// ---------------------------------------------------------------------------
// pass_text
// ---------------------------------------------------------------------------

// xetex_format.h:349,350,537; xetex-constants.h:183
const SKIPPING: u8 = 1;
const IF_TEST: u8 = 107;
const FI_OR_ELSE: u8 = 108;
// INT_PAR__tracing_ifs = 58 (xetex_format.h:93)
const INT_PAR_TRACING_IFS: i32 = 58;

/// translated from xetex-xetex0.c:7380 (pass_text)
#[no_mangle]
pub unsafe extern "C" fn pass_text() {
    let save_scanner_status = scanner_status();
    set_scanner_status(SKIPPING);
    let mut l: i32 = 0;
    crate::globals::set_skip_line(crate::globals::line());

    loop {
        get_next();
        if cur_cmd() == FI_OR_ELSE {
            if l == 0 {
                break; // done:
            }
            if cur_chr() == FI_CODE as i32 {
                l -= 1;
            }
        } else if cur_cmd() == IF_TEST {
            l += 1;
        }
    }
    // done:
    set_scanner_status(save_scanner_status);
    if eqtb_b32s1(INT_BASE_PSEUDO + INT_PAR_TRACING_IFS) > 0 {
        crate::output::show_cur_cmd_chr();
    }
}

// ---------------------------------------------------------------------------
// scan_toks
// ---------------------------------------------------------------------------

// xetex-constants.h:182,184
const NORMAL_SCANNER: u8 = 0;
const DEFINING: u8 = 2;
// xetex-constants.h:297 / xetex_format.h:237
const MAC_PARAM: u8 = 6;
// xetex_format.h:353
const THE: u8 = 111;
// xetex-constants.h:341  (BIGGEST_USV + 2)
const NO_EXPAND_FLAG: i32 = 0x10_FFFF + 2;
// xetex-constants.h:355,359,360,350
const LEFT_BRACE_LIMIT: i32 = 0x400000;
const OUT_PARAM_TOKEN: i32 = 0xA00000;
const MATCH_TOKEN: i32 = 0x1A00000;
const END_MATCH_TOKEN: i32 = 0x1C00000;
// ZERO_TOKEN = OTHER_TOKEN + '0' = 0x1800000 + 48 = 0x1800030
const ZERO_TOKEN: i32 = OTHER_TOKEN + b'0' as i32;

/// translated from xetex-xetex0.c:7081 (scan_toks)
#[no_mangle]
pub unsafe extern "C" fn scan_toks(macro_def: bool, xpand: bool) -> i32 {
    use crate::mem::TEX_NULL;

    if macro_def {
        set_scanner_status(DEFINING);
    } else {
        set_scanner_status(ABSORBING);
    }
    set_warning_index(cur_cs());
    let new_def = crate::mem::get_avail();
    set_def_ref(new_def);
    set_mem_b32s0(new_def, TEX_NULL);
    let mut p = new_def;
    let mut hash_brace: i32 = 0;
    let mut t = ZERO_TOKEN;

    // 'found: wraps the whole function body so any goto found → break 'found
    'found: {
        if macro_def {
            // Scan parameter text (§493–495)
            // 'done1 / done: are handled by a named loop + break
            let reached_done = 'param_scan: loop {
                loop {
                    get_token();
                    if cur_tok() < RIGHT_BRACE_LIMIT {
                        break 'param_scan false; // goto done1
                    }
                    if cur_cmd() == MAC_PARAM {
                        // §495
                        let s = MATCH_TOKEN + cur_chr();
                        get_token();
                        if cur_tok() < LEFT_BRACE_LIMIT {
                            hash_brace = cur_tok();
                            let q = crate::mem::get_avail();
                            set_mem_b32s1(p, q);
                            set_mem_b32s0(q, cur_tok());
                            p = q;
                            let q2 = crate::mem::get_avail();
                            set_mem_b32s1(p, q2);
                            set_mem_b32s0(q2, END_MATCH_TOKEN);
                            p = q2;
                            break 'param_scan true; // goto done
                        }
                        if t == ZERO_TOKEN + 9 {
                            error_here_with_diagnostic(
                                c"You already have nine parameters".as_ptr(),
                            );
                            capture_to_diagnostic(std::ptr::null_mut());
                            set_help_ptr(2);
                            set_help_line(
                                1,
                                c"I'm going to ignore the # sign you just used,".as_ptr(),
                            );
                            set_help_line(
                                0,
                                c"as well as the token that followed it.".as_ptr(),
                            );
                            error();
                            continue; // back to top of inner loop
                        } else {
                            t += 1;
                            if cur_tok() != t {
                                error_here_with_diagnostic(
                                    c"Parameters must be numbered consecutively".as_ptr(),
                                );
                                set_help_ptr(2);
                                set_help_line(
                                    1,
                                    c"I've inserted the digit you should have used after the #."
                                        .as_ptr(),
                                );
                                set_help_line(
                                    0,
                                    c"Type `1' to delete what you did use.".as_ptr(),
                                );
                                crate::input::back_error();
                            }
                            set_cur_tok(s);
                        }
                    }
                    // append cur_tok to list
                    let q = crate::mem::get_avail();
                    set_mem_b32s1(p, q);
                    set_mem_b32s0(q, cur_tok());
                    p = q;
                }
            };
            // done1: append END_MATCH_TOKEN
            if !reached_done {
                let q = crate::mem::get_avail();
                set_mem_b32s1(p, q);
                set_mem_b32s0(q, END_MATCH_TOKEN);
                p = q;
                // §494: check for missing {
                if cur_cmd() == RIGHT_BRACE {
                    error_here_with_diagnostic(c"Missing { inserted".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_align_state(align_state() + 1);
                    set_help_ptr(2);
                    set_help_line(
                        1,
                        c"Where was the left brace? You said something like `\\def\\a}',".as_ptr(),
                    );
                    set_help_line(
                        0,
                        c"which I'm going to interpret as `\\def\\a{}'.".as_ptr(),
                    );
                    error();
                    break 'found; // goto found
                }
            }
            // done: fall through to body scan
        } else {
            scan_left_brace();
        }

        // Scan replacement/absorbed text (§497–499)
        let mut unbalance: i32 = 1;
        loop {
            if xpand {
                // §497: expand, but handle \the specially
                loop {
                    get_next();
                    if cur_cmd() >= CALL {
                        if mem_b32s0(mem_b32s1(cur_chr())) == PROTECTED_TOKEN {
                            set_cur_cmd(RELAX);
                            set_cur_chr(NO_EXPAND_FLAG);
                        }
                    }
                    if cur_cmd() <= MAX_COMMAND {
                        break; // done2
                    }
                    if cur_cmd() != THE {
                        expand();
                    } else {
                        let q = the_toks();
                        if mem_b32s1(TEMP_HEAD) != TEX_NULL {
                            set_mem_b32s1(p, mem_b32s1(TEMP_HEAD));
                            p = q;
                        }
                    }
                }
                // done2:
                x_token();
            } else {
                get_token();
            }

            if cur_tok() < RIGHT_BRACE_LIMIT {
                if cur_cmd() < RIGHT_BRACE {
                    unbalance += 1;
                } else {
                    unbalance -= 1;
                    if unbalance == 0 {
                        break 'found; // goto found
                    }
                }
            } else if cur_cmd() == MAC_PARAM {
                if macro_def {
                    // §498: parameter reference in definition body
                    let s = cur_tok();
                    if xpand {
                        get_x_token();
                    } else {
                        get_token();
                    }
                    if cur_cmd() != MAC_PARAM {
                        if cur_tok() <= ZERO_TOKEN || cur_tok() > t {
                            error_here_with_diagnostic(
                                c"Illegal parameter number in definition of ".as_ptr(),
                            );
                            crate::output::sprint_cs(warning_index());
                            capture_to_diagnostic(std::ptr::null_mut());
                            set_help_ptr(3);
                            set_help_line(
                                2,
                                c"You meant to type ## instead of #, right?".as_ptr(),
                            );
                            set_help_line(
                                1,
                                c"Or maybe a } was forgotten somewhere earlier, and things"
                                    .as_ptr(),
                            );
                            set_help_line(
                                0,
                                c"are all screwed up? I'm going to assume that you meant ##."
                                    .as_ptr(),
                            );
                            crate::input::back_error();
                            set_cur_tok(s);
                        } else {
                            set_cur_tok((OUT_PARAM_TOKEN - 48) + cur_chr());
                        }
                    }
                }
            }
            // append cur_tok to list
            let q = crate::mem::get_avail();
            set_mem_b32s1(p, q);
            set_mem_b32s0(q, cur_tok());
            p = q;
        }
    }
    // found:
    set_scanner_status(NORMAL_SCANNER);
    if hash_brace != 0 {
        let q = crate::mem::get_avail();
        set_mem_b32s1(p, q);
        set_mem_b32s0(q, hash_brace);
        p = q;
    }
    p
}

// ---------------------------------------------------------------------------
// read_toks
// ---------------------------------------------------------------------------

// xetex-constants.h:268,277,223
const JUST_OPEN: u8 = 1;
const CLOSED: u8 = 2;
const NEW_LINE: u16 = 33;
// INT_PAR__end_line_char = 48 (xetex_format.h:83)
const INT_PAR_END_LINE_CHAR: i32 = 48;

/// translated from xetex-xetex0.c:7268 (read_toks)
#[no_mangle]
pub unsafe extern "C" fn read_toks(n: i32, r: i32, j: i32) {
    use crate::mem::TEX_NULL;

    set_scanner_status(DEFINING);
    set_warning_index(r);
    let new_def = crate::mem::get_avail();
    set_def_ref(new_def);
    set_mem_b32s0(new_def, TEX_NULL);
    let mut p = new_def;

    let q = crate::mem::get_avail();
    set_mem_b32s1(p, q);
    set_mem_b32s0(q, END_MATCH_TOKEN);
    p = q;

    let m: usize = if n < 0 || n > 15 { 16 } else { n as usize };

    let s = align_state();
    set_align_state(1_000_000);

    loop {
        // do…while(align_state != 1_000_000) — §502
        crate::input::begin_file_reading();
        set_cur_input_name((m + 1) as i32);

        if read_open_entry(m) == CLOSED {
            _tt_abort(c"terminal input forbidden".as_ptr());
        } else if read_open_entry(m) == JUST_OPEN {
            // §504
            if input_line(read_file_entry(m) as *mut libc::c_void) != 0 {
                set_read_open_entry(m, NORMAL_SCANNER); // NORMAL = 0
            } else {
                u_close(read_file_entry(m) as *mut libc::c_void);
                set_read_open_entry(m, CLOSED);
            }
        } else {
            // §505
            if input_line(read_file_entry(m) as *mut libc::c_void) == 0 {
                u_close(read_file_entry(m) as *mut libc::c_void);
                set_read_open_entry(m, CLOSED);
                if align_state() != 1_000_000 {
                    runaway();
                    error_here_with_diagnostic(c"File ended within ".as_ptr());
                    crate::output::print_esc_cstr(c"read".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(1);
                    set_help_line(0, c"This \\read has unbalanced braces.".as_ptr());
                    set_align_state(1_000_000);
                    set_cur_input_limit(0);
                    error();
                }
            }
        }

        let lim = last();
        set_cur_input_limit(lim);
        let end_char = eqtb_b32s1(INT_BASE_PSEUDO + INT_PAR_END_LINE_CHAR);
        if end_char < 0 || end_char > 255 {
            set_cur_input_limit(lim - 1);
        } else {
            set_buffer_entry(lim, end_char);
        }
        set_first(cur_input().limit + 1);
        set_cur_input_loc(cur_input().start);
        set_cur_input_state(NEW_LINE);

        if j == 1 {
            // §: direct token scan (no expansion)
            let mut loc = cur_input().loc;
            while loc <= cur_input().limit {
                let c = buffer_entry(loc);
                loc += 1;
                let tok = if c == b' ' as i32 { SPACE_TOKEN } else { c + OTHER_TOKEN };
                set_cur_chr(c);
                set_cur_tok(tok);
                let q2 = crate::mem::get_avail();
                set_mem_b32s1(p, q2);
                set_mem_b32s0(q2, tok);
                p = q2;
            }
            // goto done → end_file_reading then check while condition
        } else {
            loop {
                get_token();
                if cur_tok() == 0 {
                    break; // goto done
                }
                if align_state() < 1_000_000 {
                    loop {
                        get_token();
                        if cur_tok() == 0 {
                            break;
                        }
                    }
                    set_align_state(1_000_000);
                    break; // goto done
                }
                let q2 = crate::mem::get_avail();
                set_mem_b32s1(p, q2);
                set_mem_b32s0(q2, cur_tok());
                p = q2;
            }
        }
        // done:
        crate::input::end_file_reading();

        if align_state() == 1_000_000 {
            break;
        }
    }

    set_cur_val(new_def);
    set_scanner_status(NORMAL_SCANNER);
    set_align_state(s);
}

// ---------------------------------------------------------------------------
// begin_name / more_name / end_name
// ---------------------------------------------------------------------------

// xetex-stringpool.h:12  (TOO_BIG_CHAR + 1)
const EMPTY_STRING: i32 = TOO_BIG_CHAR + 1;

/// translated from xetex-xetex0.c:7827 (begin_name)
#[no_mangle]
pub unsafe extern "C" fn begin_name() {
    use crate::globals::{
        set_area_delimiter, set_ext_delimiter, set_file_name_quote_char, set_quoted_filename,
    };
    set_area_delimiter(0);
    set_ext_delimiter(0);
    set_quoted_filename(false);
    set_file_name_quote_char(0);
}

/// translated from xetex-xetex0.c:7837 (more_name)
#[no_mangle]
pub unsafe extern "C" fn more_name(c: u16) -> bool {
    use crate::globals::{
        area_delimiter, ext_delimiter, file_name_quote_char, quoted_filename, set_area_delimiter,
        set_ext_delimiter, set_file_name_quote_char, set_quoted_filename, stop_at_space,
    };

    if stop_at_space() && file_name_quote_char() == 0 && c == b' ' as u16 {
        return false;
    }
    if stop_at_space() && file_name_quote_char() != 0 && c == file_name_quote_char() {
        set_file_name_quote_char(0);
        return true;
    }
    if stop_at_space()
        && file_name_quote_char() == 0
        && (c == b'"' as u16 || c == b'\'' as u16)
    {
        set_file_name_quote_char(c);
        set_quoted_filename(true);
        return true;
    }
    if pool_ptr() + 1 > pool_size() {
        crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr());
    }
    set_str_pool(pool_ptr(), c);
    set_pool_ptr(pool_ptr() + 1);
    // cur_length = pool_ptr - str_start[str_ptr - TOO_BIG_CHAR]
    let cur_len = pool_ptr() - str_start(str_ptr() - TOO_BIG_CHAR);
    if c == b'/' as u16 {
        // IS_DIR_SEP
        set_area_delimiter(cur_len);
        set_ext_delimiter(0);
    } else if c == b'.' as u16 {
        set_ext_delimiter(cur_len);
    }
    true
}

/// translated from xetex-xetex0.c:7870 (end_name)
#[no_mangle]
pub unsafe extern "C" fn end_name() {
    use crate::globals::{
        area_delimiter, cur_area, cur_ext, cur_name, ext_delimiter, init_str_ptr, max_strings,
        set_cur_area, set_cur_ext, set_cur_name, set_pool_ptr, set_str_ptr,
    };

    if str_ptr() + 3 > max_strings() {
        crate::errors::overflow(c"number of strings".as_ptr(), max_strings() - init_str_ptr());
    }

    if area_delimiter() == 0 {
        set_cur_area(EMPTY_STRING);
    } else {
        set_cur_area(str_ptr());
        // str_start[str_ptr + 1 - TOO_BIG_CHAR] = str_start[str_ptr - TOO_BIG_CHAR] + area_delimiter
        let new_start = str_start(str_ptr() - TOO_BIG_CHAR) + area_delimiter();
        crate::globals::set_str_start(str_ptr() + 1 - TOO_BIG_CHAR, new_start);
        set_str_ptr(str_ptr() + 1);
        let temp_str = crate::string_pool::search_string(cur_area());
        if temp_str > 0 {
            set_cur_area(temp_str);
            set_str_ptr(str_ptr() - 1);
            // shift pool: str_pool[j - area_delimiter] = str_pool[j] for j in start..pool_ptr
            let base = str_start(str_ptr() + 1 - TOO_BIG_CHAR);
            let ad = area_delimiter();
            let pp = pool_ptr();
            for j in base..pp {
                let v = str_pool_entry(j);
                set_str_pool(j - ad, v);
            }
            set_pool_ptr(pp - ad);
        }
    }

    if ext_delimiter() == 0 {
        set_cur_ext(EMPTY_STRING);
        set_cur_name(crate::string_pool::slow_make_string());
    } else {
        set_cur_name(str_ptr());
        // str_start[str_ptr+1 - TOO_BIG_CHAR] = str_start[str_ptr - TOO_BIG_CHAR]
        //   + ext_delimiter - area_delimiter - 1
        let new_start =
            str_start(str_ptr() - TOO_BIG_CHAR) + ext_delimiter() - area_delimiter() - 1;
        crate::globals::set_str_start(str_ptr() + 1 - TOO_BIG_CHAR, new_start);
        set_str_ptr(str_ptr() + 1);

        set_cur_ext(crate::string_pool::make_string());
        set_str_ptr(str_ptr() - 1);
        let temp_str = crate::string_pool::search_string(cur_name());
        if temp_str > 0 {
            set_cur_name(temp_str);
            set_str_ptr(str_ptr() - 1);
            // shift pool: str_pool[j - ext_delimiter + area_delimiter + 1] = str_pool[j]
            let base = str_start(str_ptr() + 1 - TOO_BIG_CHAR);
            let ed = ext_delimiter();
            let ad = area_delimiter();
            let pp = pool_ptr();
            let shift = ed - ad - 1;
            for j in base..pp {
                let v = str_pool_entry(j);
                set_str_pool(j - shift, v);
            }
            set_pool_ptr(pp - shift);
        }
        set_cur_ext(crate::string_pool::slow_make_string());
    }
}

/// translated from xetex-xetex0.c:7928 (make_name_string)
#[no_mangle]
pub unsafe extern "C" fn make_name_string() -> i32 {
    use crate::globals::{
        area_delimiter, ext_delimiter, max_strings, name_in_progress, name_length,
        name_length16, name_of_file16_entry, set_area_delimiter, set_ext_delimiter,
    };

    let cur_length = pool_ptr() - str_start(str_ptr() - TOO_BIG_CHAR);
    if pool_ptr() + name_length() > pool_size() || str_ptr() == max_strings() || cur_length > 0 {
        return b'?' as i32;
    }

    crate::io::make_utf16_name();

    for k in 0..name_length16() {
        set_str_pool(pool_ptr(), name_of_file16_entry(k));
        set_pool_ptr(pool_ptr() + 1);
    }

    let result = crate::string_pool::make_string();
    let save_area_delimiter = area_delimiter();
    let save_ext_delimiter = ext_delimiter();
    let save_name_in_progress = name_in_progress();
    let save_stop_at_space = stop_at_space();

    set_name_in_progress(true);
    begin_name();
    set_stop_at_space(false);

    let mut k = 0;
    while k < name_length16() && more_name(name_of_file16_entry(k)) {
        k += 1;
    }

    set_stop_at_space(save_stop_at_space);
    end_name();
    set_name_in_progress(save_name_in_progress);
    set_area_delimiter(save_area_delimiter);
    set_ext_delimiter(save_ext_delimiter);
    result
}

// ---------------------------------------------------------------------------
// extra_right_brace
// ---------------------------------------------------------------------------

// xetex-constants.h:166,167,168
const SEMI_SIMPLE_GROUP: u8 = 14;
const MATH_SHIFT_GROUP: u8 = 15;
const MATH_LEFT_GROUP: u8 = 16;

/// translated from xetex-xetex0.c:7411 (extra_right_brace)
#[no_mangle]
pub unsafe extern "C" fn extra_right_brace() {
    use crate::globals::cur_group;

    error_here_with_diagnostic(c"Extra }, or forgotten ".as_ptr());
    match cur_group() {
        SEMI_SIMPLE_GROUP => crate::output::print_esc_cstr(c"endgroup".as_ptr()),
        MATH_SHIFT_GROUP => crate::output::print_char(b'$' as i32),
        MATH_LEFT_GROUP => crate::output::print_esc_cstr(c"right".as_ptr()),
        _ => {}
    }
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(5);
    set_help_line(4, c"I've deleted a group-closing symbol because it seems to be".as_ptr());
    set_help_line(3, c"spurious, as in `$x}$'. But perhaps the } is legitimate and".as_ptr());
    set_help_line(2, c"you forgot something else, as in `\\hbox{$x}'. In such cases".as_ptr());
    set_help_line(1, c"the way to recover is to insert both the forgotten and the".as_ptr());
    set_help_line(0, c"deleted material, e.g., by typing `I$}'.".as_ptr());
    error();
    set_align_state(align_state() + 1);
}

// ---------------------------------------------------------------------------
// scan_font_ident
// ---------------------------------------------------------------------------

const FONT_BASE: i32 = 0;
const FROZEN_NULL_FONT: i32 = 2_245_338;
const SPACE_CODE: i32 = 2;
const SPACE_SHRINK_CODE: i32 = 4;

/// translated from xetex-xetex0.c:4467 (scan_font_ident)
#[no_mangle]
pub unsafe extern "C" fn scan_font_ident() {
    let f;
    loop {
        get_x_token();
        if cur_cmd() != SPACER {
            break;
        }
    }
    if cur_cmd() == DEF_FONT {
        f = eqtb_b32s1(CUR_FONT_LOC);
    } else if cur_cmd() == SET_FONT {
        f = cur_chr();
    } else if cur_cmd() == DEF_FAMILY {
        let m = cur_chr();
        scan_math_fam_int();
        f = eqtb_b32s1(m + cur_val());
    } else {
        error_here_with_diagnostic(c"Missing font identifier".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, c"I was looking for a control sequence whose".as_ptr());
        set_help_line(0, c"current meaning has been defined by \\font.".as_ptr());
        crate::input::back_error();
        f = FONT_BASE;
    }
    set_cur_val(f);
}

/// translated from xetex-xetex0.c:4501 (find_font_dimen)
#[no_mangle]
pub unsafe extern "C" fn find_font_dimen(writing: bool) {
    use crate::mem::TEX_NULL;

    scan_int();
    let n = cur_val();
    scan_font_ident();
    let f = cur_val();

    if n <= 0 {
        set_cur_val(fmem_ptr());
    } else {
        if writing
            && n <= SPACE_SHRINK_CODE
            && n >= SPACE_CODE
            && font_glue(f) != TEX_NULL
        {
            crate::mem::delete_glue_ref(font_glue(f));
            set_font_glue(f, TEX_NULL);
        }

        if n > font_params(f) {
            if f < font_ptr() {
                set_cur_val(fmem_ptr());
            } else {
                loop {
                    if fmem_ptr() == font_mem_size() {
                        crate::errors::overflow(c"font memory".as_ptr(), font_mem_size());
                    }
                    set_font_info_b32s1(fmem_ptr(), 0);
                    set_fmem_ptr(fmem_ptr() + 1);
                    set_font_params(f, font_params(f) + 1);
                    if n == font_params(f) {
                        break;
                    }
                }
                set_cur_val(fmem_ptr() - 1);
            }
        } else {
            set_cur_val(n + param_base(f));
        }
    }

    if cur_val() == fmem_ptr() {
        error_here_with_diagnostic(c"Font ".as_ptr());
        crate::output::print_esc(hash_s1(FONT_ID_BASE + f));
        crate::output::print_cstr(c" has only ".as_ptr());
        crate::output::print_int(font_params(f));
        crate::output::print_cstr(c" fontdimen parameters".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"To increase the number of font parameters, you must".as_ptr(),
        );
        set_help_line(
            0,
            c"use \\fontdimen immediately after the \\font is loaded.".as_ptr(),
        );
        error();
    }
}

/// translated from xetex-xetex0.c:13424 (scan_and_pack_name)
#[no_mangle]
pub unsafe extern "C" fn scan_and_pack_name() {
    scan_file_name();
    pack_file_name(crate::globals::cur_name(), crate::globals::cur_area(), crate::globals::cur_ext());
}

unsafe fn scan_file_name_braced() {
    let save_scanner_status = scanner_status();
    let save_def_ref = def_ref();
    let save_cur_cs = cur_cs();
    let save_stop_at_space = stop_at_space();

    set_cur_cs(warning_index());
    scan_toks(false, true);

    set_old_setting(selector() as u8);
    set_selector(SELECTOR_NEW_STRING);
    crate::output::show_token_list(mem_b32s1(def_ref()), crate::mem::TEX_NULL, pool_size() - pool_ptr());
    set_selector(old_setting() as i32);

    let s = crate::string_pool::make_string();
    crate::mem::delete_token_ref(def_ref());
    set_def_ref(save_def_ref);
    set_cur_cs(save_cur_cs);
    set_scanner_status(save_scanner_status);

    begin_name();
    let start = str_start(s - TOO_BIG_CHAR);
    let end = str_start(s + 1 - TOO_BIG_CHAR);
    for i in start..end {
        more_name(str_pool_entry(i));
    }
    set_stop_at_space(save_stop_at_space);
}

/// translated from xetex-xetex0.c:8003 (scan_file_name)
#[no_mangle]
pub unsafe extern "C" fn scan_file_name() {
    let save_warning_index = warning_index();
    set_warning_index(cur_cs());

    loop {
        get_x_token();
        if cur_cmd() != SPACER && cur_cmd() != RELAX {
            break;
        }
    }

    crate::input::back_input();

    if cur_cmd() == LEFT_BRACE {
        scan_file_name_braced();
    } else {
        set_name_in_progress(true);
        begin_name();

        loop {
            get_x_token();
            if cur_cmd() != SPACER {
                break;
            }
        }

        loop {
            if cur_cmd() > OTHER_CHAR || cur_chr() > BIGGEST_CHAR {
                crate::input::back_input();
                break;
            }

            if !more_name(cur_chr() as u16) {
                break;
            }

            get_x_token();
        }
    }

    end_name();
    set_name_in_progress(false);
    set_warning_index(save_warning_index);
}

/// translated from xetex-xetex0.c:7970 (pack_job_name)
#[no_mangle]
pub unsafe extern "C" fn pack_job_name(s: *const libc::c_char) {
    use crate::globals::{job_name, set_cur_area, set_cur_ext, set_cur_name};

    set_cur_area(EMPTY_STRING);
    set_cur_ext(crate::texmfmp::maketexstring(s));
    set_cur_name(job_name());
    pack_file_name(crate::globals::cur_name(), crate::globals::cur_area(), crate::globals::cur_ext());
}

// ---------------------------------------------------------------------------
// scan_something_internal
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:4505 (scan_something_internal)
#[no_mangle]
pub unsafe extern "C" fn scan_something_internal(level: i16, negative: bool) {
    use crate::globals::{
        cur_cmd, cur_chr, cur_cs, cur_group, cur_if, cur_level, cur_list_aux_b32s0,
        cur_list_aux_b32s1, cur_list_head, cur_list_mode, cur_list_tail, cur_ptr,
        cur_val, cur_val1, cur_val_level, dead_cycles, eqtb_b32s1, eqtb_is_null,
        font_area_entry, font_bc_entry, font_ec_entry, font_info_b16, font_layout_engine_ptr,
        hash_s1, hi_mem_min, hyphen_char_entry, if_limit, insert_penalties, interaction,
        last_badness, last_glue, last_kern, last_node_type, last_penalty,
        mem_b16s0, mem_b16s1, mem_b16s2, mem_b16s3, mem_b32s0, mem_b32s1,
        nest_mode, nest_prev_graf, nest_ptr, output_active, page_contents, page_so_far,
        pdf_last_x_pos, pdf_last_y_pos, random_seed, save_cur_list_to_nest,
        set_cur_cmd, set_cur_chr, set_cur_cs, set_cur_ptr, set_cur_tok, set_cur_val,
        set_cur_val1, set_cur_val_level, set_help_line, set_help_ptr, set_mem_b32s1,
        shell_escape_enabled, skew_char_entry,
        char_base_entry, width_base_entry, height_base_entry, depth_base_entry,
        italic_base_entry, font_info_b32s1,
    };
    use crate::mem::TEX_NULL;

    let mut m: i32 = 0;
    let mut n: i32 = 0;
    let mut k: i32 = 0;
    let mut kk: i32 = 0;
    let mut q: i32 = 0;
    let mut r: i32 = 0;
    let mut tx: i32 = 0;

    // The C function has a `goto restart` handled here with a loop.
    'restart: loop {
        m = cur_chr();

        match cur_cmd() {
            DEF_CODE => {
                scan_usv_num();
                if m == MATH_CODE_BASE {
                    let code = eqtb_b32s1(MATH_CODE_BASE + cur_val());
                    set_cur_val1(code);
                    let class = ((code as u32 >> 21) & 0x07) as i32;
                    let fam   = ((code as u32 >> 24) & 0xFF) as i32;
                    let ch    = (code as u32 & 0x1FFFFF) as i32;
                    if ch == ACTIVE_MATH_CHAR {
                        set_cur_val1(0x8000);
                    } else if class > 7 || fam > 15 || ch > 255 {
                        let errmsg = error_here_with_diagnostic(
                            c"Extended mathchar used as mathchar".as_ptr());
                        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val1());
                        capture_to_diagnostic(std::ptr::null_mut());
                        set_help_ptr(2);
                        set_help_line(1, c"A mathchar number must be between 0 and \"7FFF.".as_ptr());
                        set_help_line(0, c"I changed this one to zero.".as_ptr());
                        int_error(cur_val1());
                        set_cur_val1(0);
                    }
                    let v = ((cur_val1() as u32 >> 21) & 0x07) as i32 * 0x1000
                        + ((cur_val1() as u32 >> 24) & 0xFF) as i32 * 0x100
                        + (cur_val1() as u32 & 0x1FFFFF) as i32;
                    set_cur_val(v);
                    set_cur_val_level(INT_VAL as u8);
                } else if m == DEL_CODE_BASE {
                    let code = eqtb_b32s1(DEL_CODE_BASE + cur_val());
                    set_cur_val1(code);
                    if cur_val1() >= 0x40000000 {
                        error_here_with_diagnostic(
                            c"Extended delcode used as delcode".as_ptr());
                        capture_to_diagnostic(std::ptr::null_mut());
                        set_help_ptr(2);
                        set_help_line(1, c"I can only go up to 2147483647='17777777777=\"7FFFFFFF,".as_ptr());
                        set_help_line(0, c"I changed this one to zero.".as_ptr());
                        error();
                        set_cur_val(0);
                        set_cur_val_level(INT_VAL as u8);
                    } else {
                        set_cur_val(cur_val1());
                        set_cur_val_level(INT_VAL as u8);
                    }
                } else if m < SF_CODE_BASE {
                    set_cur_val(eqtb_b32s1(m + cur_val()));
                    set_cur_val_level(INT_VAL as u8);
                } else if m < MATH_CODE_BASE {
                    set_cur_val(eqtb_b32s1(m + cur_val()) % 65536);
                    set_cur_val_level(INT_VAL as u8);
                } else {
                    set_cur_val(eqtb_b32s1(m + cur_val()));
                    set_cur_val_level(INT_VAL as u8);
                }
            }
            XETEX_DEF_CODE => {
                scan_usv_num();
                if m == SF_CODE_BASE {
                    set_cur_val(eqtb_b32s1(SF_CODE_BASE + cur_val()) / 65536);
                    set_cur_val_level(INT_VAL as u8);
                } else if m == MATH_CODE_BASE {
                    set_cur_val(eqtb_b32s1(MATH_CODE_BASE + cur_val()));
                    set_cur_val_level(INT_VAL as u8);
                } else if m == MATH_CODE_BASE + 1 {
                    error_here_with_diagnostic(
                        c"Can't use \\Umathcode as a number (try \\Umathcodenum)".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(2);
                    set_help_line(1, c"\\Umathcode is for setting a mathcode from separate values;".as_ptr());
                    set_help_line(0, c"use \\Umathcodenum to access them as single values.".as_ptr());
                    error();
                    set_cur_val(0);
                    set_cur_val_level(INT_VAL as u8);
                } else if m == DEL_CODE_BASE {
                    set_cur_val(eqtb_b32s1(DEL_CODE_BASE + cur_val()));
                    set_cur_val_level(INT_VAL as u8);
                } else {
                    error_here_with_diagnostic(
                        c"Can't use \\Udelcode as a number (try \\Udelcodenum)".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(2);
                    set_help_line(1, c"\\Udelcode is for setting a delcode from separate values;".as_ptr());
                    set_help_line(0, c"use \\Udelcodenum to access them as single values.".as_ptr());
                    error();
                    set_cur_val(0);
                    set_cur_val_level(INT_VAL as u8);
                }
            }
            TOKS_REGISTER | ASSIGN_TOKS | DEF_FAMILY | SET_FONT | DEF_FONT => {
                if level != TOK_VAL {
                    error_here_with_diagnostic(c"Missing number, treated as zero".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(3);
                    set_help_line(2, c"A number should have been here; I inserted `0'.".as_ptr());
                    set_help_line(1, c"(If you can't figure out why I needed to see a number,".as_ptr());
                    set_help_line(0, c"look up `weird error' in the index to The TeXbook.)".as_ptr());
                    crate::input::back_error();
                    set_cur_val(0);
                    set_cur_val_level(DIMEN_VAL as u8);
                } else if cur_cmd() <= ASSIGN_TOKS {
                    if cur_cmd() < ASSIGN_TOKS {
                        if m == 0 {
                            scan_register_num();
                            if cur_val() < 256 {
                                set_cur_val(eqtb_b32s1(TOKS_BASE + cur_val()));
                            } else {
                                crate::save::find_sa_element(TOK_VAL as u16, cur_val(), false);
                                if cur_ptr() == TEX_NULL {
                                    set_cur_val(TEX_NULL);
                                } else {
                                    set_cur_val(mem_b32s1(cur_ptr() + 1));
                                }
                            }
                        } else {
                            set_cur_val(mem_b32s1(m + 1));
                        }
                    } else if cur_chr() == LOCAL_BASE + LOCAL_XETEX_INTER_CHAR_TOKS {
                        scan_char_class_not_ignored();
                        set_cur_ptr(cur_val());
                        scan_char_class_not_ignored();
                        crate::save::find_sa_element(
                            INTER_CHAR_VAL as u16,
                            cur_ptr() * CHAR_CLASS_LIMIT + cur_val(),
                            false,
                        );
                        if cur_ptr() == TEX_NULL {
                            set_cur_val(TEX_NULL);
                        } else {
                            set_cur_val(mem_b32s1(cur_ptr() + 1));
                        }
                    } else {
                        set_cur_val(eqtb_b32s1(m));
                    }
                    set_cur_val_level(TOK_VAL as u8);
                } else {
                    crate::input::back_input();
                    crate::scan::scan_font_ident();
                    set_cur_val(FONT_ID_BASE + cur_val());
                    set_cur_val_level(IDENT_VAL as u8);
                }
            }
            ASSIGN_INT => {
                set_cur_val(eqtb_b32s1(m));
                set_cur_val_level(INT_VAL as u8);
            }
            ASSIGN_DIMEN => {
                set_cur_val(eqtb_b32s1(m));
                set_cur_val_level(DIMEN_VAL as u8);
            }
            ASSIGN_GLUE => {
                set_cur_val(eqtb_b32s1(m));
                set_cur_val_level(GLUE_VAL as u8);
            }
            ASSIGN_MU_GLUE => {
                set_cur_val(eqtb_b32s1(m));
                set_cur_val_level(MU_VAL as u8);
            }
            SET_AUX => {
                if (cur_list_mode().abs() as i32) != m {
                    error_here_with_diagnostic(c"Improper ".as_ptr());
                    print_cmd_chr(SET_AUX as u16, m);
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(4);
                    set_help_line(3, c"You can refer to \\spacefactor only in horizontal mode;".as_ptr());
                    set_help_line(2, c"you can refer to \\prevdepth only in vertical mode; and".as_ptr());
                    set_help_line(1, c"neither of these is meaningful inside \\write. So".as_ptr());
                    set_help_line(0, c"I'm forgetting what you said and using zero instead.".as_ptr());
                    error();
                    if level != TOK_VAL {
                        set_cur_val(0);
                        set_cur_val_level(DIMEN_VAL as u8);
                    } else {
                        set_cur_val(0);
                        set_cur_val_level(INT_VAL as u8);
                    }
                } else if m == VMODE as i32 {
                    set_cur_val(cur_list_aux_b32s1());
                    set_cur_val_level(DIMEN_VAL as u8);
                } else {
                    set_cur_val(cur_list_aux_b32s0());
                    set_cur_val_level(INT_VAL as u8);
                }
            }
            SET_PREV_GRAF => {
                if cur_list_mode() == 0 {
                    set_cur_val(0);
                    set_cur_val_level(INT_VAL as u8);
                } else {
                    save_cur_list_to_nest(nest_ptr());
                    let mut p = nest_ptr();
                    while (nest_mode(p).abs() as i32) != VMODE as i32 {
                        p -= 1;
                    }
                    set_cur_val(nest_prev_graf(p));
                    set_cur_val_level(INT_VAL as u8);
                }
            }
            SET_PAGE_INT => {
                if m == 0 {
                    set_cur_val(dead_cycles());
                } else if m == 2 {
                    set_cur_val(interaction() as i32);
                } else {
                    set_cur_val(insert_penalties());
                }
                set_cur_val_level(INT_VAL as u8);
            }
            SET_PAGE_DIMEN => {
                if page_contents() == 0 && !output_active() {
                    if m == 0 {
                        set_cur_val(MAX_HALFWORD);
                    } else {
                        set_cur_val(0);
                    }
                } else {
                    set_cur_val(page_so_far(m as usize));
                }
                set_cur_val_level(DIMEN_VAL as u8);
            }
            SET_SHAPE => {
                if m > LOCAL_BASE + LOCAL_PAR_SHAPE {
                    scan_int();
                    let shape_ptr = eqtb_b32s1(m);
                    if shape_ptr == TEX_NULL || cur_val() < 0 {
                        set_cur_val(0);
                    } else {
                        let cnt = mem_b32s1(shape_ptr + 1);
                        if cur_val() > cnt {
                            set_cur_val(cnt);
                        }
                        set_cur_val(mem_b32s1(shape_ptr + cur_val() + 1));
                    }
                } else {
                    let par_shape = eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
                    if par_shape == TEX_NULL {
                        set_cur_val(0);
                    } else {
                        set_cur_val(mem_b32s0(par_shape));
                    }
                }
                set_cur_val_level(INT_VAL as u8);
            }
            SET_BOX_DIMEN => {
                scan_register_num();
                if cur_val() < 256 {
                    q = eqtb_b32s1(BOX_BASE + cur_val());
                } else {
                    crate::save::find_sa_element(4, cur_val(), false);
                    if cur_ptr() == TEX_NULL {
                        q = TEX_NULL;
                    } else {
                        q = mem_b32s1(cur_ptr() + 1);
                    }
                }
                if q == TEX_NULL {
                    set_cur_val(0);
                } else {
                    set_cur_val(mem_b32s1(q + m));
                }
                set_cur_val_level(DIMEN_VAL as u8);
            }
            CHAR_GIVEN | MATH_GIVEN | XETEX_MATH_GIVEN => {
                set_cur_val(cur_chr());
                set_cur_val_level(INT_VAL as u8);
            }
            ASSIGN_FONT_DIMEN => {
                find_font_dimen(false);
                set_font_info_b32s1(fmem_ptr(), 0);
                set_cur_val(font_info_b32s1(cur_val()));
                set_cur_val_level(DIMEN_VAL as u8);
            }
            ASSIGN_FONT_INT => {
                crate::scan::scan_font_ident();
                if m == 0 {
                    set_cur_val(hyphen_char_entry(cur_val()));
                    set_cur_val_level(INT_VAL as u8);
                } else if m == 1 {
                    set_cur_val(skew_char_entry(cur_val()));
                    set_cur_val_level(INT_VAL as u8);
                } else {
                    n = cur_val();
                    let fa = font_area_entry(n);
                    if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                        scan_glyph_number(n);
                    } else {
                        scan_char_num();
                    }
                    k = cur_val();
                    match m {
                        LP_CODE_BASE => {
                            set_cur_val(get_cp_code(n, k as u32, LEFT_SIDE));
                            set_cur_val_level(INT_VAL as u8);
                        }
                        RP_CODE_BASE => {
                            set_cur_val(get_cp_code(n, k as u32, RIGHT_SIDE));
                            set_cur_val_level(INT_VAL as u8);
                        }
                        _ => {}
                    }
                }
            }
            REGISTER => {
                if m < 0 || m > 19 {
                    set_cur_val_level((mem_b16s1(m) / 64) as u8);
                    if (cur_val_level() as i16) < GLUE_VAL {
                        set_cur_val(mem_b32s1(m + 2));
                    } else {
                        set_cur_val(mem_b32s1(m + 1));
                    }
                } else {
                    scan_register_num();
                    set_cur_val_level(m as u8);
                    if cur_val() > 255 {
                        crate::save::find_sa_element(cur_val_level() as u16, cur_val(), false);
                        if cur_ptr() == TEX_NULL {
                            set_cur_val(0);
                        } else if (cur_val_level() as i16) < GLUE_VAL {
                            set_cur_val(mem_b32s1(cur_ptr() + 2));
                        } else {
                            set_cur_val(mem_b32s1(cur_ptr() + 1));
                        }
                    } else {
                        match cur_val_level() as i16 {
                            INT_VAL => set_cur_val(eqtb_b32s1(COUNT_BASE + cur_val())),
                            DIMEN_VAL => set_cur_val(eqtb_b32s1(SCALED_BASE + cur_val())),
                            GLUE_VAL => set_cur_val(eqtb_b32s1(SKIP_BASE + cur_val())),
                            MU_VAL => set_cur_val(eqtb_b32s1(MU_SKIP_BASE + cur_val())),
                            _ => {}
                        }
                    }
                }
            }
            LAST_ITEM => {
                if m >= INPUT_LINE_NO_CODE {
                    if m >= ETEX_GLUE {
                        if m < ETEX_MU {
                            if m == MU_TO_GLUE_CODE {
                                scan_mu_glue();
                            }
                            set_cur_val_level(GLUE_VAL as u8);
                        } else if m < ETEX_EXPR {
                            if m == GLUE_TO_MU_CODE {
                                scan_normal_glue();
                            }
                            set_cur_val_level(MU_VAL as u8);
                        } else {
                            set_cur_val_level((m - ETEX_EXPR) as u8);
                            scan_expr();
                        }
                        while (cur_val_level() as i16) > level {
                            if (cur_val_level() as i16) == GLUE_VAL {
                                let gm = cur_val();
                                set_cur_val(mem_b32s1(gm + 1));
                                crate::mem::delete_glue_ref(gm);
                            } else if (cur_val_level() as i16) == MU_VAL {
                                mu_error();
                            }
                            set_cur_val_level(cur_val_level() - 1);
                        }
                        if negative {
                            if (cur_val_level() as i16) >= GLUE_VAL {
                                let gm = cur_val();
                                set_cur_val(new_spec(gm));
                                crate::mem::delete_glue_ref(gm);
                                set_mem_b32s1(cur_val() + 1, -(mem_b32s1(cur_val() + 1)));
                                set_mem_b32s1(cur_val() + 2, -(mem_b32s1(cur_val() + 2)));
                                set_mem_b32s1(cur_val() + 3, -(mem_b32s1(cur_val() + 3)));
                            } else {
                                set_cur_val(-cur_val());
                            }
                        }
                        return;
                    }
                    if m >= XETEX_DIM {
                        match m {
                            XETEX_GLYPH_BOUNDS_CODE => {
                                let cf = eqtb_b32s1(CUR_FONT_LOC);
                                let fa = font_area_entry(cf);
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    scan_int();
                                    n = cur_val();
                                    if n < 1 || n > 4 {
                                        error_here_with_diagnostic(
                                            c"\\\\XeTeXglyphbounds requires an edge index from 1 to 4;".as_ptr());
                                        crate::output::print_nl_cstr(c"I don't know anything about edge ".as_ptr());
                                        crate::output::print_int(n);
                                        capture_to_diagnostic(std::ptr::null_mut());
                                        error();
                                        set_cur_val(0);
                                    } else {
                                        scan_int();
                                        set_cur_val(get_glyph_bounds(cf, n, cur_val()));
                                    }
                                } else {
                                    not_native_font_error(LAST_ITEM as i32, m, cf);
                                    set_cur_val(0);
                                }
                            }
                            FONT_CHAR_WD_CODE | FONT_CHAR_HT_CODE | FONT_CHAR_DP_CODE | FONT_CHAR_IC_CODE => {
                                crate::scan::scan_font_ident();
                                q = cur_val();
                                scan_usv_num();
                                let fa = font_area_entry(q);
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    set_cur_val(match m {
                                        FONT_CHAR_WD_CODE => getnativecharwd(q, cur_val()),
                                        FONT_CHAR_HT_CODE => getnativecharht(q, cur_val()),
                                        FONT_CHAR_DP_CODE => getnativechardp(q, cur_val()),
                                        _ => getnativecharic(q, cur_val()),
                                    });
                                } else {
                                    let bc = font_bc_entry(q) as i32;
                                    let ec = font_ec_entry(q) as i32;
                                    if bc <= cur_val() && ec >= cur_val() {
                                        let i = font_info_b16(
                                            char_base_entry(q) + effective_char(true, q, cur_val() as u16));
                                        set_cur_val(match m {
                                            FONT_CHAR_WD_CODE =>
                                                font_info_b32s1(width_base_entry(q) + i.s3 as i32),
                                            FONT_CHAR_HT_CODE =>
                                                font_info_b32s1(height_base_entry(q) + i.s2 as i32 / 16),
                                            FONT_CHAR_DP_CODE =>
                                                font_info_b32s1(depth_base_entry(q) + i.s2 as i32 % 16),
                                            _ =>
                                                font_info_b32s1(italic_base_entry(q) + i.s1 as i32 / 4),
                                        });
                                    } else {
                                        set_cur_val(0);
                                    }
                                }
                            }
                            PAR_SHAPE_LENGTH_CODE | PAR_SHAPE_INDENT_CODE | PAR_SHAPE_DIMEN_CODE => {
                                q = cur_chr() - PAR_SHAPE_LENGTH_CODE;
                                scan_int();
                                let par_shape = eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
                                if par_shape == TEX_NULL || cur_val() <= 0 {
                                    set_cur_val(0);
                                } else {
                                    if q == 2 {
                                        q = cur_val() % 2;
                                        set_cur_val((cur_val() + q) / 2);
                                    }
                                    let cnt = mem_b32s0(par_shape);
                                    if cur_val() > cnt { set_cur_val(cnt); }
                                    set_cur_val(mem_b32s1(par_shape + 2 * cur_val() - q));
                                }
                                set_cur_val_level(DIMEN_VAL as u8);
                            }
                            GLUE_STRETCH_CODE | GLUE_SHRINK_CODE => {
                                scan_normal_glue();
                                q = cur_val();
                                if m == GLUE_STRETCH_CODE {
                                    set_cur_val(mem_b32s1(q + 2));
                                } else {
                                    set_cur_val(mem_b32s1(q + 3));
                                }
                                crate::mem::delete_glue_ref(q);
                            }
                            _ => {}
                        }
                        set_cur_val_level(DIMEN_VAL as u8);
                    } else {
                        // m >= INPUT_LINE_NO_CODE && m < XETEX_DIM
                        match m {
                            INPUT_LINE_NO_CODE => { set_cur_val(crate::globals::line()); }
                            BADNESS_CODE => { set_cur_val(last_badness()); }
                            ELAPSED_TIME_CODE => { set_cur_val(crate::texmfmp::get_microinterval()); }
                            RANDOM_SEED_CODE => { set_cur_val(random_seed()); }
                            PDF_SHELL_ESCAPE_CODE => {
                                set_cur_val(if shell_escape_enabled() { 1 } else { 0 });
                            }
                            ETEX_VERSION_CODE => { set_cur_val(ETEX_VERSION); }
                            XETEX_VERSION_CODE => { set_cur_val(XETEX_VERSION); }
                            XETEX_COUNT_GLYPHS_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                set_cur_val(if fa == AAT_FONT_FLAG {
                                    aat_font_get(m - XETEX_INT, engine)
                                } else if fa == OTGR_FONT_FLAG {
                                    ot_font_get(m - XETEX_INT, engine)
                                } else { 0 });
                            }
                            XETEX_COUNT_FEATURES_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                set_cur_val(if fa == AAT_FONT_FLAG {
                                    aat_font_get(m - XETEX_INT, engine)
                                } else if fa == OTGR_FONT_FLAG && usingGraphite(engine) {
                                    ot_font_get(m - XETEX_INT, engine)
                                } else { 0 });
                            }
                            XETEX_VARIATION_CODE | XETEX_VARIATION_MIN_CODE
                            | XETEX_VARIATION_MAX_CODE | XETEX_VARIATION_DEFAULT_CODE
                            | XETEX_COUNT_VARIATIONS_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                set_cur_val(0);
                            }
                            XETEX_FEATURE_CODE_CODE | XETEX_IS_EXCLUSIVE_FEATURE_CODE
                            | XETEX_COUNT_SELECTORS_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                if fa == AAT_FONT_FLAG {
                                    scan_int();
                                    k = cur_val();
                                    set_cur_val(aat_font_get_1(m - XETEX_INT, engine, k));
                                } else if fa == OTGR_FONT_FLAG && usingGraphite(engine) {
                                    scan_int();
                                    k = cur_val();
                                    set_cur_val(ot_font_get_1(m - XETEX_INT, engine, k));
                                } else {
                                    not_aat_gr_font_error(LAST_ITEM as i32, m, n);
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_SELECTOR_CODE_CODE | XETEX_IS_DEFAULT_SELECTOR_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                if fa == AAT_FONT_FLAG {
                                    scan_int(); k = cur_val();
                                    scan_int();
                                    set_cur_val(aat_font_get_2(m - XETEX_INT, engine, k, cur_val()));
                                } else if fa == OTGR_FONT_FLAG && usingGraphite(engine) {
                                    scan_int(); k = cur_val();
                                    scan_int();
                                    set_cur_val(ot_font_get_2(m - XETEX_INT, engine, k, cur_val()));
                                } else {
                                    not_aat_gr_font_error(LAST_ITEM as i32, m, n);
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_FIND_VARIATION_BY_NAME_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                if fa == AAT_FONT_FLAG {
                                    scan_and_pack_name();
                                    set_cur_val(aat_font_get_named(m - XETEX_INT, engine));
                                } else {
                                    not_aat_font_error(LAST_ITEM as i32, m, n);
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_FIND_FEATURE_BY_NAME_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                if fa == AAT_FONT_FLAG {
                                    scan_and_pack_name();
                                    set_cur_val(aat_font_get_named(m - XETEX_INT, engine));
                                } else if fa == OTGR_FONT_FLAG && usingGraphite(engine) {
                                    scan_and_pack_name();
                                    set_cur_val(gr_font_get_named(m - XETEX_INT, engine));
                                } else {
                                    not_aat_gr_font_error(LAST_ITEM as i32, m, n);
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_FIND_SELECTOR_BY_NAME_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                if fa == AAT_FONT_FLAG {
                                    scan_int(); k = cur_val();
                                    scan_and_pack_name();
                                    set_cur_val(aat_font_get_named_1(m - XETEX_INT, engine, k));
                                } else if fa == OTGR_FONT_FLAG && usingGraphite(engine) {
                                    scan_int(); k = cur_val();
                                    scan_and_pack_name();
                                    set_cur_val(gr_font_get_named_1(m - XETEX_INT, engine, k));
                                } else {
                                    not_aat_gr_font_error(LAST_ITEM as i32, m, n);
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_OT_COUNT_SCRIPTS_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                set_cur_val(if fa == OTGR_FONT_FLAG && usingOpenType(engine) {
                                    ot_font_get(m - XETEX_INT, engine)
                                } else { 0 });
                            }
                            XETEX_OT_COUNT_LANGUAGES_CODE | XETEX_OT_SCRIPT_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                if fa == OTGR_FONT_FLAG && usingOpenType(engine) {
                                    scan_int();
                                    set_cur_val(ot_font_get_1(m - XETEX_INT, engine, cur_val()));
                                } else {
                                    not_ot_font_error(LAST_ITEM as i32, m, n);
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_OT_COUNT_FEATURES_CODE | XETEX_OT_LANGUAGE_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                if fa == OTGR_FONT_FLAG && usingOpenType(engine) {
                                    scan_int(); k = cur_val();
                                    scan_int();
                                    set_cur_val(ot_font_get_2(m - XETEX_INT, engine, k, cur_val()));
                                } else {
                                    not_ot_font_error(LAST_ITEM as i32, m, n);
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_OT_FEATURE_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                if fa == OTGR_FONT_FLAG && usingOpenType(engine) {
                                    scan_int(); k = cur_val();
                                    scan_int(); kk = cur_val();
                                    scan_int();
                                    set_cur_val(ot_font_get_3(m - XETEX_INT, engine, k, kk, cur_val()));
                                } else {
                                    not_ot_font_error(LAST_ITEM as i32, m, n);
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_MAP_CHAR_TO_GLYPH_CODE => {
                                let cf = eqtb_b32s1(CUR_FONT_LOC);
                                let fa = font_area_entry(cf);
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    scan_int();
                                    n = cur_val();
                                    set_cur_val(map_char_to_glyph(cf, n));
                                } else {
                                    not_native_font_error(LAST_ITEM as i32, m, cf);
                                    set_cur_val(0);
                                }
                            }
                            XETEX_GLYPH_INDEX_CODE => {
                                let cf = eqtb_b32s1(CUR_FONT_LOC);
                                let fa = font_area_entry(cf);
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    scan_and_pack_name();
                                    set_cur_val(map_glyph_to_index(cf));
                                } else {
                                    not_native_font_error(LAST_ITEM as i32, m, cf);
                                    set_cur_val(0);
                                }
                            }
                            XETEX_FONT_TYPE_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                set_cur_val(if fa == AAT_FONT_FLAG { 1 }
                                    else if fa == OTGR_FONT_FLAG && usingOpenType(engine) { 2 }
                                    else if fa == OTGR_FONT_FLAG && usingGraphite(engine) { 3 }
                                    else { 0 });
                            }
                            XETEX_FIRST_CHAR_CODE | XETEX_LAST_CHAR_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let fa = font_area_entry(n);
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    set_cur_val(get_font_char_range(
                                        n, (m == XETEX_FIRST_CHAR_CODE) as i32));
                                } else if m == XETEX_FIRST_CHAR_CODE {
                                    set_cur_val(font_bc_entry(n) as i32);
                                } else {
                                    set_cur_val(font_ec_entry(n) as i32);
                                }
                            }
                            PDF_LAST_X_POS_CODE => { set_cur_val(pdf_last_x_pos()); }
                            PDF_LAST_Y_POS_CODE => { set_cur_val(pdf_last_y_pos()); }
                            XETEX_PDF_PAGE_COUNT_CODE => {
                                scan_and_pack_name();
                                set_cur_val(count_pdf_file_pages());
                            }
                            CURRENT_GROUP_LEVEL_CODE => {
                                set_cur_val(cur_level() as i32 - 1);
                            }
                            CURRENT_GROUP_TYPE_CODE => {
                                set_cur_val(cur_group() as i32);
                            }
                            CURRENT_IF_LEVEL_CODE => {
                                q = crate::globals::cond_ptr();
                                let mut cnt = 0i32;
                                while q != TEX_NULL {
                                    cnt += 1;
                                    q = mem_b32s1(q);
                                }
                                set_cur_val(cnt);
                            }
                            CURRENT_IF_TYPE_CODE => {
                                if crate::globals::cond_ptr() == TEX_NULL {
                                    set_cur_val(0);
                                } else if (cur_if() as i32) < UNLESS_CODE {
                                    set_cur_val(cur_if() as i32 + 1);
                                } else {
                                    set_cur_val(-(cur_if() as i32 - 31));
                                }
                            }
                            CURRENT_IF_BRANCH_CODE => {
                                let ifl = if_limit();
                                if ifl == OR_CODE || ifl == ELSE_CODE {
                                    set_cur_val(1);
                                } else if ifl == FI_CODE {
                                    set_cur_val(-1);
                                } else {
                                    set_cur_val(0);
                                }
                            }
                            GLUE_STRETCH_ORDER_CODE | GLUE_SHRINK_ORDER_CODE => {
                                scan_normal_glue();
                                q = cur_val();
                                if m == GLUE_STRETCH_ORDER_CODE {
                                    set_cur_val(mem_b16s1(q) as i32);
                                } else {
                                    set_cur_val(mem_b16s0(q) as i32);
                                }
                                crate::mem::delete_glue_ref(q);
                            }
                            _ => {}
                        }
                        set_cur_val_level(INT_VAL as u8);
                    }
                } else {
                    // m < INPUT_LINE_NO_CODE: last-node queries
                    set_cur_val(0);
                    tx = cur_list_tail();
                    if tx < hi_mem_min() {
                        let node_type = mem_b16s1(tx);
                        let subtype = mem_b16s0(tx);
                        if node_type == MATH_NODE && subtype == END_M_CODE {
                            r = cur_list_head();
                            loop {
                                q = r;
                                r = mem_b32s1(q);
                                if r == tx { break; }
                            }
                            tx = q;
                        }
                    }
                    if cur_chr() == LAST_NODE_TYPE_CODE_VAL {
                        set_cur_val_level(INT_VAL as u8);
                        if tx == cur_list_head() || cur_list_mode() == 0 {
                            set_cur_val(-1);
                        }
                    } else {
                        set_cur_val_level(cur_chr() as u8);
                    }
                    if tx < hi_mem_min() && cur_list_mode() != 0 {
                        let node_type = mem_b16s1(tx);
                        match cur_chr() {
                            x if x == INT_VAL as i32 => {
                                if node_type == PENALTY_NODE {
                                    set_cur_val(mem_b32s1(tx + 1));
                                }
                            }
                            x if x == DIMEN_VAL as i32 => {
                                if node_type == KERN_NODE {
                                    set_cur_val(mem_b32s1(tx + 1));
                                }
                            }
                            x if x == GLUE_VAL as i32 => {
                                if node_type == GLUE_NODE {
                                    set_cur_val(mem_b32s0(tx + 1));
                                    if mem_b16s0(tx) == MU_GLUE {
                                        set_cur_val_level(MU_VAL as u8);
                                    }
                                }
                            }
                            x if x == LAST_NODE_TYPE_CODE_VAL => {
                                if node_type <= UNSET_NODE {
                                    set_cur_val(node_type as i32 + 1);
                                } else {
                                    set_cur_val(UNSET_NODE as i32 + 2);
                                }
                            }
                            _ => {}
                        }
                    } else if cur_list_mode() == VMODE && tx == cur_list_head() {
                        match cur_chr() {
                            x if x == INT_VAL as i32 => set_cur_val(last_penalty()),
                            x if x == DIMEN_VAL as i32 => set_cur_val(last_kern()),
                            x if x == GLUE_VAL as i32 => {
                                if last_glue() != MAX_HALFWORD {
                                    set_cur_val(last_glue());
                                }
                            }
                            x if x == LAST_NODE_TYPE_CODE_VAL => set_cur_val(last_node_type()),
                            _ => {}
                        }
                    }
                }
            }
            IGNORE_SPACES => {
                if cur_chr() == 1 {
                    crate::scan::get_token();
                    if cur_cs() < HASH_BASE {
                        set_cur_cs(crate::hash::prim_lookup(cur_cs() - SINGLE_BASE));
                    } else {
                        set_cur_cs(crate::hash::prim_lookup(hash_s1(cur_cs())));
                    }
                    if cur_cs() != UNDEFINED_PRIMITIVE {
                        set_cur_cmd(crate::globals::eqtb_b16s1(PRIM_EQTB_BASE + cur_cs()) as u8);
                        set_cur_chr(eqtb_b32s1(PRIM_EQTB_BASE + cur_cs()));
                        set_cur_cs(PRIM_EQTB_BASE + cur_cs());
                        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
                    } else {
                        set_cur_cmd(RELAX);
                        set_cur_chr(0);
                        set_cur_tok(CS_TOKEN_FLAG + FROZEN_RELAX);
                        set_cur_cs(FROZEN_RELAX);
                    }
                    continue 'restart;
                }
            }
            _ => {
                error_here_with_diagnostic(c"You can't use `".as_ptr());
                print_cmd_chr(cur_cmd() as u16, cur_chr());
                crate::output::print_cstr(c"' after ".as_ptr());
                crate::output::print_esc_cstr(c"the".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(1);
                set_help_line(0, c"I'm forgetting what you said and using zero instead.".as_ptr());
                error();
                set_cur_val(0);
                if level != TOK_VAL {
                    set_cur_val_level(DIMEN_VAL as u8);
                } else {
                    set_cur_val_level(INT_VAL as u8);
                }
            }
        }
        break; // end 'restart loop
    }

    // Coerce cur_val to the requested level.
    while (cur_val_level() as i16) > level {
        if (cur_val_level() as i16) == GLUE_VAL {
            set_cur_val(mem_b32s1(cur_val() + 1));
        } else if (cur_val_level() as i16) == MU_VAL {
            mu_error();
        }
        set_cur_val_level(cur_val_level() - 1);
    }

    if negative {
        if (cur_val_level() as i16) >= GLUE_VAL {
            let gm = cur_val();
            set_cur_val(new_spec(gm));
            set_mem_b32s1(cur_val() + 1, -(mem_b32s1(cur_val() + 1)));
            set_mem_b32s1(cur_val() + 2, -(mem_b32s1(cur_val() + 2)));
            set_mem_b32s1(cur_val() + 3, -(mem_b32s1(cur_val() + 3)));
        } else {
            set_cur_val(-cur_val());
        }
    } else if (cur_val_level() as i16) >= GLUE_VAL && (cur_val_level() as i16) <= MU_VAL {
        // GLUE_SPEC_ref_count(cur_val)++
        set_mem_b32s1(cur_val(), mem_b32s1(cur_val()) + 1);
    }
}

// ---------------------------------------------------------------------------
// scan_int
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:5465 (scan_int)
#[no_mangle]
pub unsafe extern "C" fn scan_int() {
    use crate::globals::{
        cur_cmd, cur_cs, cur_tok, cur_val, eqtb_b32s1,
        hash_s1, set_cur_cmd, set_cur_chr, set_cur_cs, set_cur_tok, set_cur_val,
        set_help_line, set_help_ptr, set_radix,
    };
    use crate::mem::TEX_NULL;

    let mut negative = false;
    let mut m: i32 = 0;
    let mut d: i32 = 0;
    let mut vacuous = false;
    let mut ok_so_far = true;

    set_radix(0);

    // Skip spaces and absorb sign prefix(es).
    loop {
        loop { get_x_token(); if cur_cmd() != SPACER { break; } }
        if cur_tok() == OTHER_TOKEN + b'-' as i32 {
            negative = !negative;
            set_cur_tok(OTHER_TOKEN + b'+' as i32);
        }
        if cur_tok() != OTHER_TOKEN + b'+' as i32 { break; }
    }

    'restart: loop {
        if cur_tok() == ALPHA_TOKEN {
            get_token();
            if cur_tok() < CS_TOKEN_FLAG {
                set_cur_val(cur_chr());
                if cur_cmd() <= 2 { // RIGHT_BRACE = 2 (xetex_format.h:232)
                    if cur_cmd() == 2 {
                        set_align_state(align_state() + 1);
                    } else {
                        set_align_state(align_state() - 1);
                    }
                }
            } else if cur_tok() < CS_TOKEN_FLAG + SINGLE_BASE {
                set_cur_val(cur_tok() - (CS_TOKEN_FLAG + ACTIVE_BASE));
            } else {
                set_cur_val(cur_tok() - (CS_TOKEN_FLAG + SINGLE_BASE));
            }
            if cur_val() > BIGGEST_USV {
                error_here_with_diagnostic(c"Improper alphabetic constant".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(1, c"A one-character control sequence belongs after a ` mark.".as_ptr());
                set_help_line(0, c"So I'm essentially inserting \\0 here.".as_ptr());
                set_cur_val(b'0' as i32);
                crate::input::back_error();
            } else {
                get_x_token();
                if cur_cmd() != SPACER {
                    crate::input::back_input();
                }
            }
        } else if cur_tok() == FROZEN_PRIM_TOKEN {
            get_token();
            if cur_cs() < HASH_BASE {
                set_cur_cs(crate::hash::prim_lookup(cur_cs() - SINGLE_BASE));
            } else {
                set_cur_cs(crate::hash::prim_lookup(hash_s1(cur_cs())));
            }
            if cur_cs() != UNDEFINED_PRIMITIVE {
                set_cur_cmd(crate::globals::eqtb_b16s1(PRIM_EQTB_BASE + cur_cs()) as u8);
                set_cur_chr(eqtb_b32s1(PRIM_EQTB_BASE + cur_cs()));
                set_cur_cs(PRIM_EQTB_BASE + cur_cs());
                set_cur_tok(CS_TOKEN_FLAG + cur_cs());
            } else {
                set_cur_cmd(RELAX);
                set_cur_chr(0);
                set_cur_tok(CS_TOKEN_FLAG + FROZEN_RELAX);
                set_cur_cs(FROZEN_RELAX);
            }
            continue 'restart;
        } else if cur_cmd() >= MIN_INTERNAL && cur_cmd() <= MAX_INTERNAL {
            scan_something_internal(INT_VAL, false);
        } else {
            let mut radix_val: i32 = 10;
            m = 0x0CCC_CCCC;
            if cur_tok() == OCTAL_TOKEN {
                radix_val = 8;
                m = 0x1000_0000;
                get_x_token();
            } else if cur_tok() == HEX_TOKEN {
                radix_val = 16;
                m = 0x0800_0000;
                get_x_token();
            }
            set_radix(radix_val as u8);
            vacuous = true;
            set_cur_val(0);
            loop {
                if cur_tok() < ZERO_TOKEN + radix_val
                    && cur_tok() >= ZERO_TOKEN
                    && cur_tok() <= ZERO_TOKEN + 9
                {
                    d = cur_tok() - ZERO_TOKEN;
                } else if radix_val == 16 {
                    if cur_tok() <= A_TOKEN + 5 && cur_tok() >= A_TOKEN {
                        d = cur_tok() - A_TOKEN + 10;
                    } else if cur_tok() <= OTHER_A_TOKEN + 5 && cur_tok() >= OTHER_A_TOKEN {
                        d = cur_tok() - OTHER_A_TOKEN + 10;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
                vacuous = false;
                if cur_val() >= m && (cur_val() > m || d > 7 || radix_val != 10) {
                    if ok_so_far {
                        error_here_with_diagnostic(c"Number too big".as_ptr());
                        capture_to_diagnostic(std::ptr::null_mut());
                        set_help_ptr(2);
                        set_help_line(1, c"I can only go up to 2147483647='17777777777=\"7FFFFFFF,".as_ptr());
                        set_help_line(0, c"so I'm using that number instead of yours.".as_ptr());
                        error();
                        set_cur_val(TEX_INFINITY);
                        ok_so_far = false;
                    }
                } else {
                    set_cur_val(cur_val() * radix_val + d);
                }
                get_x_token();
            }
            if vacuous {
                error_here_with_diagnostic(c"Missing number, treated as zero".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(3);
                set_help_line(2, c"A number should have been here; I inserted `0'.".as_ptr());
                set_help_line(1, c"(If you can't figure out why I needed to see a number,".as_ptr());
                set_help_line(0, c"look up `weird error' in the index to The TeXbook.)".as_ptr());
                crate::input::back_error();
            } else if cur_cmd() != SPACER {
                crate::input::back_input();
            }
        }
        break; // end 'restart
    }

    if negative {
        set_cur_val(-cur_val());
    }
}

// ---------------------------------------------------------------------------
// conditional
// ---------------------------------------------------------------------------

// IF_ sub-codes (xetex_format.h:543–564).
const IF_CHAR_CODE: i16 = 0;
const IF_CAT_CODE: i16 = 1;
const IF_INT_CODE: i16 = 2;
const IF_DIM_CODE: i16 = 3;
const IF_ODD_CODE: i16 = 4;
const IF_VMODE_CODE: i16 = 5;
const IF_HMODE_CODE: i16 = 6;
const IF_MMODE_CODE: i16 = 7;
const IF_INNER_CODE: i16 = 8;
const IF_VOID_CODE: i16 = 9;
const IF_HBOX_CODE: i16 = 10;
const IF_VBOX_CODE: i16 = 11;
const IFX_CODE: i16 = 12;
const IF_EOF_CODE: i16 = 13;
const IF_TRUE_CODE: i16 = 14;
const IF_FALSE_CODE: i16 = 15;
const IF_CASE_CODE: i16 = 16;
const IF_DEF_CODE: i16 = 17;
const IF_CS_CODE: i16 = 18;
const IF_FONT_CHAR_CODE: i16 = 19;
const IF_IN_CSNAME_CODE: i16 = 20;
const IF_PRIMITIVE_CODE: i16 = 21;
// Node / command codes for conditional.
const IF_NODE_SIZE: i32 = 2;
const IF_CODE: u8 = 1;             // xetex-constants.h:267
const HLIST_NODE_C: u16 = 0;       // xetex_format.h:380
const VLIST_NODE_C: u16 = 1;       // xetex_format.h:381
const HMODE_C: i16 = 104;          // xetex_format.h:369
const MMODE_C: i16 = 207;          // xetex_format.h:370
const ACTIVE_CHAR_C: u8 = 13;      // xetex_format.h:246
const UNDEFINED_CS: u8 = 103;      // xetex_format.h:344
const END_CS_NAME: u8 = 67;        // xetex_format.h:305
const NULL_CS: i32 = 2_228_225;    // xetex_format.h:199
const TOO_BIG_USV_C: i32 = 1_114_112; // xetex_format.h:22 (BIGGEST_USV + 1)
const NORMAL_SCAN: u8 = 0;         // xetex-constants.h:182 / xetex_format.h:420
// INT_PAR__tracing_commands = 36 (xetex_format.h:71).
const INT_PAR_TRACING_COMMANDS: i32 = 36;

/// translated from xetex-xetex0.c:6271 (conditional)
#[no_mangle]
pub unsafe extern "C" fn conditional() {
    use crate::globals::{
        buf_size, buffer_entry, cond_ptr, cur_chr, cur_cmd, cur_cs, cur_if, cur_list_mode,
        eqtb_b16s1, eqtb_b32s1, first, hash_s1, if_limit, if_stack, if_line, in_open,
        is_in_csname, line, max_buf_stack, mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1,
        read_open_entry, scanner_status, set_buffer_entry, set_cond_ptr, set_cur_chr, set_cur_cmd,
        set_cur_cs, set_cur_if, set_if_limit, set_if_line, set_is_in_csname, set_max_buf_stack,
        set_mem_b16s0, set_mem_b16s1, set_mem_b32s1, set_scanner_status, shell_escape_enabled,
        font_area_entry, font_bc_entry, font_ec_entry, font_info_b16, char_base_entry,
    };
    use crate::mem::{TEX_NULL, get_node, free_node, get_avail, flush_list};

    let mut b = false;
    let mut e: bool;
    let mut r: u8 = 0;
    let mut m: i32 = 0;
    let mut n: i32 = 0;
    let mut p: i32;
    let mut q: i32;
    let save_scanner_status: u8;
    let save_cond_ptr: i32;
    let this_if: i16;
    let is_unless: bool;

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_IFS) > 0 {
        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) <= 1 {
            crate::output::show_cur_cmd_chr();
        }
    }

    p = get_node(IF_NODE_SIZE);
    set_mem_b32s1(p, cond_ptr());
    set_mem_b16s1(p, if_limit() as u16);
    set_mem_b16s0(p, cur_if() as u16);
    set_mem_b32s1(p + 1, if_line());
    set_cond_ptr(p);
    set_cur_if(cur_chr() as i16);
    set_if_limit(IF_CODE);
    set_if_line(line());

    save_cond_ptr = cond_ptr();
    is_unless = cur_chr() >= UNLESS_CODE as i32;
    this_if = (cur_chr() % UNLESS_CODE as i32) as i16;

    match this_if {
        IF_CHAR_CODE | IF_CAT_CODE => {
            get_x_token();
            if cur_cmd() == RELAX && cur_chr() == NO_EXPAND_FLAG {
                set_cur_cmd(ACTIVE_CHAR_C);
                set_cur_chr(cur_tok() - (CS_TOKEN_FLAG + ACTIVE_BASE));
            }
            if cur_cmd() > ACTIVE_CHAR_C || cur_chr() > BIGGEST_USV {
                m = RELAX as i32;
                n = TOO_BIG_USV_C;
            } else {
                m = cur_cmd() as i32;
                n = cur_chr();
            }
            get_x_token();
            if cur_cmd() == RELAX && cur_chr() == NO_EXPAND_FLAG {
                set_cur_cmd(ACTIVE_CHAR_C);
                set_cur_chr(cur_tok() - (CS_TOKEN_FLAG + ACTIVE_BASE));
            }
            if cur_cmd() > ACTIVE_CHAR_C || cur_chr() > BIGGEST_USV {
                set_cur_cmd(RELAX);
                set_cur_chr(TOO_BIG_USV_C);
            }
            b = if this_if == IF_CHAR_CODE { n == cur_chr() } else { m == cur_cmd() as i32 };
        }
        IF_INT_CODE | IF_DIM_CODE => {
            if this_if == IF_INT_CODE {
                scan_int();
            } else {
                scan_dimen(false, false, false);
            }
            n = crate::globals::cur_val();
            loop { get_x_token(); if cur_cmd() != SPACER { break; } }
            if cur_tok() >= OTHER_TOKEN + 60 && cur_tok() <= OTHER_TOKEN + 62 {
                r = (cur_tok() - OTHER_TOKEN) as u8;
            } else {
                error_here_with_diagnostic(c"Missing = inserted for ".as_ptr());
                print_cmd_chr(IF_TEST as u16, this_if as i32);
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(1);
                set_help_line(0, c"I was expecting to see `<', `=', or `>'. Didn't.".as_ptr());
                crate::input::back_error();
                r = b'=';
            }
            if this_if == IF_INT_CODE {
                scan_int();
            } else {
                scan_dimen(false, false, false);
            }
            let cv = crate::globals::cur_val();
            b = match r {
                60 => n < cv,
                61 => n == cv,
                _  => n > cv,
            };
        }
        IF_ODD_CODE => {
            scan_int();
            b = (crate::globals::cur_val() % 2) != 0;
        }
        IF_VMODE_CODE => { b = cur_list_mode().abs() == VMODE; }
        IF_HMODE_CODE => { b = cur_list_mode().abs() == HMODE_C; }
        IF_MMODE_CODE => { b = cur_list_mode().abs() == MMODE_C; }
        IF_INNER_CODE => { b = cur_list_mode() < 0; }
        IF_VOID_CODE | IF_HBOX_CODE | IF_VBOX_CODE => {
            scan_register_num();
            p = if crate::globals::cur_val() < 256 {
                eqtb_b32s1(BOX_BASE + crate::globals::cur_val())
            } else {
                crate::save::find_sa_element(4, crate::globals::cur_val(), false);
                if crate::globals::cur_ptr() == TEX_NULL { TEX_NULL }
                else { mem_b32s1(crate::globals::cur_ptr() + 1) }
            };
            b = if this_if == IF_VOID_CODE {
                p == TEX_NULL
            } else if p == TEX_NULL {
                false
            } else if this_if == IF_HBOX_CODE {
                mem_b16s1(p) == HLIST_NODE_C
            } else {
                mem_b16s1(p) == VLIST_NODE_C
            };
        }
        IFX_CODE => {
            save_scanner_status = scanner_status();
            set_scanner_status(NORMAL_SCAN);
            get_next();
            n = cur_cs();
            p = cur_cmd() as i32;
            q = cur_chr();
            get_next();
            if cur_cmd() != p as u8 {
                b = false;
            } else if (cur_cmd() as i32) < CALL as i32 {
                b = cur_chr() == q;
            } else {
                p = mem_b32s1(cur_chr()); // second token's replacement text (not saved q = first token's chr)
                q = mem_b32s1(eqtb_b32s1(n));
                if p == q {
                    b = true;
                } else {
                    while p != TEX_NULL && q != TEX_NULL {
                        if mem_b32s0(p) != mem_b32s0(q) {
                            p = TEX_NULL;
                        } else {
                            p = mem_b32s1(p);
                            q = mem_b32s1(q);
                        }
                    }
                    b = p == TEX_NULL && q == TEX_NULL;
                }
            }
            set_scanner_status(save_scanner_status);
        }
        IF_EOF_CODE => {
            scan_four_bit_int_or_18();
            b = if crate::globals::cur_val() == 18 {
                !shell_escape_enabled()
            } else {
                read_open_entry(crate::globals::cur_val() as usize) == 2 // CLOSED = 2
            };
        }
        IF_TRUE_CODE  => { b = true; }
        IF_FALSE_CODE => { b = false; }
        IF_DEF_CODE => {
            save_scanner_status = scanner_status();
            set_scanner_status(NORMAL_SCAN);
            get_next();
            b = cur_cmd() != UNDEFINED_CS;
            set_scanner_status(save_scanner_status);
        }
        IF_CS_CODE => {
            n = get_avail();
            p = n;
            e = is_in_csname();
            set_is_in_csname(true);
            loop {
                get_x_token();
                if cur_cs() == 0 {
                    q = get_avail();
                    set_mem_b32s1(p, q);
                    set_mem_b32s0(q, cur_tok()); // mem[q].b32.s0 = cur_tok
                    p = q;
                }
                if cur_cs() != 0 { break; }
            }
            if cur_cmd() != END_CS_NAME {
                error_here_with_diagnostic(c"Missing ".as_ptr());
                crate::output::print_esc_cstr(c"endcsname".as_ptr());
                crate::output::print_cstr(c" inserted".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(1, c"The control sequence marked <to be read again> should".as_ptr());
                set_help_line(0, c"not appear between \\csname and \\endcsname.".as_ptr());
                crate::input::back_error();
            }
            m = first();
            p = mem_b32s1(n);
            while p != TEX_NULL {
                if m >= max_buf_stack() {
                    set_max_buf_stack(m + 1);
                    if max_buf_stack() == buf_size() {
                        overflow(c"buffer size".as_ptr(), buf_size());
                    }
                }
                set_buffer_entry(m, mem_b32s0(p) % MAX_CHAR_VAL);
                m += 1;
                p = mem_b32s1(p);
            }
            let cur_cs_val = if m == first() {
                NULL_CS
            } else if m == first() + 1 {
                SINGLE_BASE + buffer_entry(first())
            } else {
                crate::hash::id_lookup(first(), m - first())
            };
            crate::globals::set_cur_cs(cur_cs_val);
            flush_list(n);
            b = eqtb_b16s1(cur_cs()) != UNDEFINED_CS as u16;
            set_is_in_csname(e);
        }
        IF_IN_CSNAME_CODE => { b = is_in_csname(); }
        IF_FONT_CHAR_CODE => {
            scan_font_ident();
            n = crate::globals::cur_val();
            scan_usv_num();
            let fa = font_area_entry(n);
            b = if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                map_char_to_glyph(n, crate::globals::cur_val()) > 0
            } else {
                let bc = font_bc_entry(n) as i32;
                let ec = font_ec_entry(n) as i32;
                if bc <= crate::globals::cur_val() && ec >= crate::globals::cur_val() {
                    font_info_b16(
                        char_base_entry(n)
                        + effective_char(true, n, crate::globals::cur_val() as u16)
                    ).s3 > 0
                } else {
                    false
                }
            };
        }
        IF_CASE_CODE => {
            scan_int();
            n = crate::globals::cur_val();
            if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 1 {
                crate::output::begin_diagnostic();
                crate::output::diagnostic_begin_capture_warning_here();
                crate::output::print_cstr(c"{case ".as_ptr());
                crate::output::print_int(n);
                crate::output::print_char(b'}' as i32);
                capture_to_diagnostic(std::ptr::null_mut());
                crate::output::end_diagnostic(false);
            }
            while n != 0 {
                pass_text();
                if cond_ptr() == save_cond_ptr {
                    if crate::globals::cur_chr() == OR_CODE as i32 {
                        n -= 1;
                    } else {
                        // goto common_ending
                        if crate::globals::cur_chr() == FI_CODE as i32 {
                            if if_stack(in_open()) == cond_ptr() { crate::output::if_warning(); }
                            pop_cond_node();
                        } else {
                            set_if_limit(FI_CODE);
                        }
                        return;
                    }
                } else if crate::globals::cur_chr() == FI_CODE as i32 {
                    if if_stack(in_open()) == cond_ptr() { crate::output::if_warning(); }
                    pop_cond_node();
                }
            }
            crate::output::change_if_limit(OR_CODE as i16, save_cond_ptr);
            return;
        }
        IF_PRIMITIVE_CODE => {
            save_scanner_status = scanner_status();
            set_scanner_status(NORMAL_SCAN);
            get_next();
            set_scanner_status(save_scanner_status);
            let cs = cur_cs();
            m = if cs < HASH_BASE {
                crate::hash::prim_lookup(cs - SINGLE_BASE)
            } else {
                crate::hash::prim_lookup(hash_s1(cs))
            };
            b = cur_cmd() != UNDEFINED_CS
                && m != UNDEFINED_PRIMITIVE
                && cur_cmd() as i32 == eqtb_b16s1(PRIM_EQTB_BASE + m) as i32
                && cur_chr() == eqtb_b32s1(PRIM_EQTB_BASE + m);
        }
        _ => {}
    }

    if is_unless { b = !b; }

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 1 {
        crate::output::begin_diagnostic();
        crate::output::diagnostic_begin_capture_warning_here();
        if b {
            crate::output::print_cstr(c"{true}".as_ptr());
        } else {
            crate::output::print_cstr(c"{false}".as_ptr());
        }
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::end_diagnostic(false);
    }

    if b {
        crate::output::change_if_limit(ELSE_CODE as i16, save_cond_ptr);
        return;
    }

    // false branch: skip until \else or \fi at our nesting level
    loop {
        pass_text();
        if cond_ptr() == save_cond_ptr {
            if crate::globals::cur_chr() != OR_CODE as i32 {
                // common_ending
                if crate::globals::cur_chr() == FI_CODE as i32 {
                    if if_stack(in_open()) == cond_ptr() { crate::output::if_warning(); }
                    pop_cond_node();
                } else {
                    set_if_limit(FI_CODE);
                }
                return;
            }
            error_here_with_diagnostic(c"Extra ".as_ptr());
            crate::output::print_esc_cstr(c"or".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(1);
            set_help_line(0, c"I'm ignoring this; it doesn't match any \\if.".as_ptr());
            error();
        } else if crate::globals::cur_chr() == FI_CODE as i32 {
            if if_stack(in_open()) == cond_ptr() { crate::output::if_warning(); }
            pop_cond_node();
        }
    }
}

/// Pop a cond_ptr node — repeated pattern in conditional (§515).
#[inline]
unsafe fn pop_cond_node() {
    use crate::globals::{cond_ptr, if_line, cur_if, if_limit, set_cond_ptr, set_if_line, set_cur_if, set_if_limit, mem_b16s0, mem_b16s1, mem_b32s1};
    let p = cond_ptr();
    set_if_line(mem_b32s1(p + 1));
    set_cur_if(mem_b16s0(p) as i16);
    set_if_limit(mem_b16s1(p) as u8);
    set_cond_ptr(mem_b32s1(p));
    crate::mem::free_node(p, IF_NODE_SIZE);
}

/// Calls `overflow` (still in C) with a pool-string message.
unsafe fn overflow(s: *const libc::c_char, n: i32) {
    extern "C" { fn overflow(s: *const libc::c_char, n: i32); }
    overflow(s, n);
}

// ---------------------------------------------------------------------------
// open_log_file
// ---------------------------------------------------------------------------

const INVALID_HANDLE: usize = 0;
const SELECTOR_LOG_ONLY: i32 = 18;

/// translated from xetex-xetex0.c:7937 (open_log_file)
#[no_mangle]
pub unsafe extern "C" fn open_log_file() {
    use crate::globals::{
        input_stack, input_ptr,
        job_name, set_job_name, set_log_file, set_log_opened,
        name_of_file,
        set_input_stack, set_texmf_log_name,
    };

    let old_setting = selector();
    if job_name() == 0 {
        set_job_name(crate::texmfmp::maketexstring(c"texput".as_ptr()));
    }

    pack_job_name(c".log".as_ptr());

    let handle = ttstub_output_open(name_of_file(), 0);
    if handle == INVALID_HANDLE {
        _tt_abort(c"cannot open log file output".as_ptr());
    }
    set_log_file(handle);

    set_texmf_log_name(make_name_string());
    set_selector(SELECTOR_LOG_ONLY);
    set_log_opened(true);

    set_input_stack(input_ptr(), cur_input());

    crate::output::print_nl_cstr(c"**".as_ptr());
    let mut l = input_stack(0).limit;
    if buffer_entry(l) == eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR) {
        l -= 1;
    }

    let mut k = 1;
    while k <= l {
        crate::output::print(buffer_entry(k));
        k += 1;
    }

    crate::output::print_ln();
    set_selector(old_setting + 2);
}

// ---------------------------------------------------------------------------
// show_info
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:9942 (show_info)
#[no_mangle]
pub unsafe extern "C" fn show_info() {
    use crate::globals::{mem_b32s0, temp_ptr};
    show_node_list(mem_b32s0(temp_ptr()));
}

// ---------------------------------------------------------------------------
// start_input
// ---------------------------------------------------------------------------

// TTBC_FILE_FORMAT values (tectonic_bridge_core_generated.h:94,93).
const TTBC_FILE_FORMAT_TEX_SI: i32 = 26;
const TTBC_FILE_FORMAT_TECTONIC_PRIMARY_SI: i32 = 59;
// INT_PAR offsets used by start_input (xetex_format.h:110,111,90).
const INT_PAR_XETEX_DEFAULT_INPUT_MODE: i32 = 75;
const INT_PAR_XETEX_DEFAULT_INPUT_ENCODING: i32 = 76;
const INT_PAR_TRACING_STACK_LEVELS: i32 = 55;

mod raw_utf8 {
    extern "C" {
        pub(super) static bytesFromUTF8: [u8; 256];
        pub(super) static offsetsFromUTF8: [u32; 6];
    }
}

/// translated from xetex-xetex0.c:6431 (start_input)
#[no_mangle]
pub unsafe extern "C" fn start_input(primary_input_name: *const libc::c_char) {
    use crate::globals::{
        cur_name, cur_area, cur_ext, full_source_filename_stack, in_open,
        input_file, input_file_ptr, input_ptr, job_name, name_length16,
        name_of_file16_entry, set_area_delimiter, set_ext_delimiter,
        set_full_source_filename_stack, set_job_name, set_source_filename_stack,
    };

    let format = if !primary_input_name.is_null() {
        TTBC_FILE_FORMAT_TECTONIC_PRIMARY_SI
    } else {
        TTBC_FILE_FORMAT_TEX_SI
    };

    if !primary_input_name.is_null() {
        set_name_in_progress(true);
        begin_name();
        set_stop_at_space(false);

        let mut cp = primary_input_name as *const u8;
        if pool_ptr() + libc::strlen(primary_input_name) as i32 * 2 >= pool_size() {
            _tt_abort(c"string pool overflow".as_ptr());
        }

        loop {
            let first_byte = *cp;
            cp = cp.add(1);
            if first_byte == 0 { break; }
            let mut rval = first_byte as u32;
            let extra_bytes = raw_utf8::bytesFromUTF8[rval as usize] as usize;
            for _ in 0..extra_bytes {
                if *cp != 0 {
                    rval = (rval << 6) + *cp as u32;
                    cp = cp.add(1);
                } else {
                    rval <<= 6;
                }
            }
            rval -= raw_utf8::offsetsFromUTF8[extra_bytes] as u32;

            if rval > 0xffff {
                rval -= 0x10000;
                let pp = pool_ptr();
                set_str_pool(pp, (0xd800 + rval / 0x0400) as u16);
                set_pool_ptr(pp + 1);
                let pp = pool_ptr();
                set_str_pool(pp, (0xdc00 + rval % 0x0400) as u16);
                set_pool_ptr(pp + 1);
            } else {
                let pp = pool_ptr();
                set_str_pool(pp, rval as u16);
                set_pool_ptr(pp + 1);
            }

            let cur_len = pool_ptr() - str_start(str_ptr() - TOO_BIG_CHAR);
            if rval == b'/' as u32 {
                // IS_DIR_SEP
                set_area_delimiter(cur_len);
                set_ext_delimiter(0);
            } else if rval == b'.' as u32 {
                set_ext_delimiter(cur_len);
            }
        }

        set_stop_at_space(true);
        end_name();
        set_name_in_progress(false);
    } else {
        scan_file_name();
    }

    pack_file_name(cur_name(), cur_area(), cur_ext());
    crate::input::begin_file_reading();

    let idx = cur_input().index as i32;
    if crate::io::u_open_in(
        input_file_ptr(idx),
        format,
        c"rb".as_ptr(),
        eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_MODE),
        eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_ENCODING),
    ) == 0 {
        // Format "failed to open input file "<name>" into a stack buffer;
        // matches the C: _tt_abort("failed to open input file \"%s\"", name_of_file)
        let mut buf = [0u8; 512];
        libc::snprintf(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            c"failed to open input file \"%s\"".as_ptr(),
            crate::globals::name_of_file(),
        );
        _tt_abort(buf.as_ptr() as *const libc::c_char);
    }

    crate::io::make_utf16_name();
    set_name_in_progress(true);
    begin_name();
    set_stop_at_space(false);
    let mut k = 0i32;
    while k < name_length16() && more_name(name_of_file16_entry(k)) {
        k += 1;
    }
    set_stop_at_space(true);
    end_name();
    set_name_in_progress(false);

    let name = make_name_string();
    set_cur_input_name(name);
    set_source_filename_stack(in_open(), cur_input().name);
    set_full_source_filename_stack(
        in_open(),
        crate::texmfmp::maketexstring(crate::io::name_of_input_file),
    );

    if cur_input().name == str_ptr() - 1 {
        let temp_str = crate::string_pool::search_string(cur_input().name);
        if temp_str > 0 {
            set_cur_input_name(temp_str);
            set_str_ptr(str_ptr() - 1);
            set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
        }
    }

    if job_name() == 0 {
        set_job_name(cur_name());
        open_log_file();
    }

    let fss = full_source_filename_stack(in_open());
    if term_offset() + crate::string_pool::length(fss) > max_print_line() - 2 {
        crate::output::print_ln();
    } else if term_offset() > 0 || file_offset() > 0 {
        crate::output::print_char(b' ' as i32);
    }
    crate::output::print_char(b'(' as i32);
    set_open_parens(open_parens() + 1);
    crate::output::print(fss);
    ttstub_output_flush(rust_stdout());

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS) > 0 {
        crate::output::begin_diagnostic();
        crate::output::diagnostic_begin_capture_warning_here();
        crate::output::print_ln();
        crate::output::print_char(b'~' as i32);
        let mut v = input_ptr() - 1;
        if v < eqtb_b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS) {
            while v > 0 {
                crate::output::print_char(b'.' as i32);
                v -= 1;
            }
        } else {
            crate::output::print_char(b'~' as i32);
        }
        crate::output::print_cstr(c"INPUT ".as_ptr());
        crate::output::print(cur_name());
        crate::output::print(cur_ext());
        crate::output::print_ln();
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::end_diagnostic(false);
    }

    set_cur_input_state(NEW_LINE);
    synctex_start_input();
    set_line(1);
    input_line(input_file(idx) as *mut libc::c_void);
    set_cur_input_limit(last());

    if eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR) < 0
        || eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR) > 255
    {
        set_cur_input_limit(cur_input().limit - 1);
    } else {
        set_buffer_entry(cur_input().limit, eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR));
    }
    set_first(cur_input().limit + 1);
    set_cur_input_loc(cur_input().start);
}

// ---------------------------------------------------------------------------
// effective_char / effective_char_info
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// eTeX_enabled
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:8811 (eTeX_enabled)
#[no_mangle]
pub unsafe extern "C" fn eTeX_enabled(b: bool, j: u16, k: i32) -> bool {
    if !b {
        error_here_with_diagnostic(c"Improper ".as_ptr());
        print_cmd_chr(j, k);
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(1);
        set_help_line(0, c"Sorry, this optional e-TeX feature has been disabled.".as_ptr());
        error();
    }
    b
}

// ---------------------------------------------------------------------------
// effective_char / effective_char_info
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:4457 (effective_char)
#[no_mangle]
pub unsafe extern "C" fn effective_char(_err_p: bool, f: i32, c: u16) -> i32 {
    let mut c = c;
    if !xtx_ligature_present() && !font_mapping_ptr(f).is_null() {
        c = apply_tfm_font_mapping(font_mapping_ptr(f), c as i32) as u16;
    }
    set_xtx_ligature_present(false);
    c as i32
}

// translated from xetex-xetex0.c:6434 (effective_char_info)
#[no_mangle]
pub unsafe extern "C" fn effective_char_info(f: i32, c: u16) -> crate::mem_types::B16x4 {
    let mut c = c;
    if !xtx_ligature_present() && !font_mapping_ptr(f).is_null() {
        c = apply_tfm_font_mapping(font_mapping_ptr(f), c as i32) as u16;
    }
    set_xtx_ligature_present(false);
    font_info_b16(char_base_entry(f) + c as i32)
}

// ---------------------------------------------------------------------------
// you_cant / report_illegal_case / privileged
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:9326 (you_cant)
#[no_mangle]
pub unsafe extern "C" fn you_cant() {
    error_here_with_diagnostic(c"You can't use `".as_ptr());
    print_cmd_chr(cur_cmd() as u16, cur_chr());
    crate::output::print_in_mode(cur_list_mode() as i32);
    capture_to_diagnostic(std::ptr::null_mut());
}

// translated from xetex-xetex0.c:9334 (report_illegal_case)
#[no_mangle]
pub unsafe extern "C" fn report_illegal_case() {
    you_cant();
    set_help_ptr(4);
    set_help_line(3, c"Sorry, but I'm not programmed to handle this case;".as_ptr());
    set_help_line(2, c"I'll just pretend that you didn't ask for it.".as_ptr());
    set_help_line(1, c"If you're in the wrong mode, you might be able to".as_ptr());
    set_help_line(0, c"return to the right one by typing `I}' or `I$' or `I\\par'.".as_ptr());
    error();
}

// translated from xetex-xetex0.c:9347 (privileged)
#[no_mangle]
pub unsafe extern "C" fn privileged() -> bool {
    if cur_list_mode() > 0 {
        true
    } else {
        report_illegal_case();
        false
    }
}

// ---------------------------------------------------------------------------
// align_error / no_align_error / omit_error / cs_error
// ---------------------------------------------------------------------------

const RIGHT_BRACE_TOKEN: i32 = 0x40_0000; // xetex-constants.h:351
const TAB_TOKEN: i32 = 0x80_0000;         // xetex-constants.h:354

// translated from xetex-xetex0.c:10390 (align_error)
#[no_mangle]
pub unsafe extern "C" fn align_error() {
    if align_state().abs() > 2 {
        error_here_with_diagnostic(c"Misplaced ".as_ptr());
        print_cmd_chr(cur_cmd() as u16, cur_chr());
        capture_to_diagnostic(std::ptr::null_mut());
        if cur_tok() == TAB_TOKEN + 38 {
            set_help_ptr(6);
            set_help_line(5, c"I can't figure out why you would want to use a tab mark".as_ptr());
            set_help_line(4, c"here. If you just want an ampersand, the remedy is".as_ptr());
            set_help_line(3, c"simple: Just type `I\\&' now. But if some right brace".as_ptr());
            set_help_line(2, c"up above has ended a previous alignment prematurely,".as_ptr());
            set_help_line(1, c"you're probably due for more error messages, and you".as_ptr());
            set_help_line(0, c"might try typing `S' now just to see what is salvageable.".as_ptr());
        } else {
            set_help_ptr(5);
            set_help_line(4, c"I can't figure out why you would want to use a tab mark".as_ptr());
            set_help_line(3, c"or \\cr or \\span just now. If something like a right brace".as_ptr());
            set_help_line(2, c"up above has ended a previous alignment prematurely,".as_ptr());
            set_help_line(1, c"you're probably due for more error messages, and you".as_ptr());
            set_help_line(0, c"might try typing `S' now just to see what is salvageable.".as_ptr());
        }
        error();
    } else {
        crate::input::back_input();
        if align_state() < 0 {
            error_here_with_diagnostic(c"Missing { inserted".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_align_state(align_state() + 1);
            set_cur_tok(LEFT_BRACE_TOKEN + 123);
        } else {
            error_here_with_diagnostic(c"Missing } inserted".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_align_state(align_state() - 1);
            set_cur_tok(RIGHT_BRACE_TOKEN + 125);
        }
        set_help_ptr(3);
        set_help_line(2, c"I've put in what seems to be necessary to fix".as_ptr());
        set_help_line(1, c"the current column of the current alignment.".as_ptr());
        set_help_line(0, c"Try to go on, since this might almost work.".as_ptr());
        crate::input::ins_error();
    }
}

// translated from xetex-xetex0.c:10443 (no_align_error)
#[no_mangle]
pub unsafe extern "C" fn no_align_error() {
    error_here_with_diagnostic(c"Misplaced ".as_ptr());
    crate::output::print_esc_cstr(c"noalign".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(2);
    set_help_line(1, c"I expect to see \\noalign only after the \\cr of".as_ptr());
    set_help_line(0, c"an alignment. Proceed, and I'll ignore this case.".as_ptr());
    error();
}

// translated from xetex-xetex0.c:10457 (omit_error)
#[no_mangle]
pub unsafe extern "C" fn omit_error() {
    error_here_with_diagnostic(c"Misplaced ".as_ptr());
    crate::output::print_esc_cstr(c"omit".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(2);
    set_help_line(1, c"I expect to see \\omit only after tab marks or the \\cr of".as_ptr());
    set_help_line(0, c"an alignment. Proceed, and I'll ignore this case.".as_ptr());
    error();
}

// translated from xetex-xetex0.c:10489 (cs_error)
#[no_mangle]
pub unsafe extern "C" fn cs_error() {
    error_here_with_diagnostic(c"Extra ".as_ptr());
    crate::output::print_esc_cstr(c"endcsname".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(1);
    set_help_line(0, c"I'm ignoring this, since I wasn't doing a \\csname.".as_ptr());
    error();
}

// ---------------------------------------------------------------------------
// push_math
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:10416 (push_math)
// group_code = unsigned char = u8 (xetex-xetexd.h:325)
#[no_mangle]
pub unsafe extern "C" fn push_math(c: u8) {
    crate::mem::push_nest();
    crate::globals::set_cur_list_mode(-207); // -MMODE (xetex_format.h:370)
    crate::globals::set_cur_list_aux_b32s1(crate::mem::TEX_NULL);
    crate::save::new_save_level(c);
}

// ---------------------------------------------------------------------------
// end_graf / append_penalty
// ---------------------------------------------------------------------------

const HMODE: i16 = 104; // xetex_format.h:369

// translated from xetex-xetex0.c:9942 (end_graf)
#[no_mangle]
pub unsafe extern "C" fn end_graf() {
    if cur_list_mode() == HMODE {
        if crate::globals::cur_list_head() == crate::globals::cur_list_tail() {
            crate::mem::pop_nest();
        } else {
            line_break(false);
        }
        if crate::globals::cur_list_e_tex_aux() != crate::mem::TEX_NULL {
            crate::mem::flush_list(crate::globals::cur_list_e_tex_aux());
            crate::globals::set_cur_list_e_tex_aux(crate::mem::TEX_NULL);
        }
        normal_paragraph();
        crate::globals::set_error_count(0);
    }
}

// translated from xetex-xetex0.c:10016 (append_penalty)
#[no_mangle]
pub unsafe extern "C" fn append_penalty() {
    scan_int();
    let p = crate::mem::new_penalty(crate::globals::cur_val());
    let tail = crate::globals::cur_list_tail();
    set_mem_b32s1(tail, p);
    crate::globals::set_cur_list_tail(p);
    if cur_list_mode() == VMODE {
        build_page();
    }
}

// ---------------------------------------------------------------------------
// normal_paragraph
// ---------------------------------------------------------------------------

const DIMEN_BASE: i32 = 8_941_180;     // xetex_format.h:222
const ETEX_PEN_BASE: i32 = 2_255_140;  // xetex_format.h:210
const SHAPE_REF: u16 = 120;            // xetex_format.h:362
const INT_PAR_LOOSENESS: i32 = 19;     // xetex_format.h:54
const INT_PAR_HANG_AFTER: i32 = 41;    // xetex_format.h:76
const DIMEN_PAR_HANG_INDENT: i32 = 17; // xetex_format.h:139

// translated from xetex-xetex0.c:9496 (normal_paragraph)
#[no_mangle]
pub unsafe extern "C" fn normal_paragraph() {
    if eqtb_b32s1(INT_BASE + INT_PAR_LOOSENESS) != 0 {
        crate::save::eq_word_define(INT_BASE + INT_PAR_LOOSENESS, 0);
    }
    if eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_HANG_INDENT) != 0 {
        crate::save::eq_word_define(DIMEN_BASE + DIMEN_PAR_HANG_INDENT, 0);
    }
    if eqtb_b32s1(INT_BASE + INT_PAR_HANG_AFTER) != 1 {
        crate::save::eq_word_define(INT_BASE + INT_PAR_HANG_AFTER, 1);
    }
    if eqtb_b32s1(LOCAL_BASE) != crate::mem::TEX_NULL {
        crate::save::eq_define(LOCAL_BASE, SHAPE_REF, crate::mem::TEX_NULL);
    }
    if eqtb_b32s1(ETEX_PEN_BASE) != crate::mem::TEX_NULL {
        crate::save::eq_define(ETEX_PEN_BASE, SHAPE_REF, crate::mem::TEX_NULL);
    }
}

// ---------------------------------------------------------------------------
// box_error
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:9270 (box_error)
#[no_mangle]
pub unsafe extern "C" fn box_error(n: u8) {
    let box_val = eqtb_b32s1(BOX_BASE + n as i32);
    error();
    crate::output::begin_diagnostic();
    crate::output::diagnostic_begin_capture_warning_here();
    crate::output::print_nl_cstr(c"The following box has been deleted:".as_ptr());
    show_box(box_val);
    capture_to_diagnostic(std::ptr::null_mut());
    crate::output::end_diagnostic(true);
    flush_node_list(box_val);
    // BOX_REG(n) = TEX_NULL — direct eqtb write, no save-stack involvement
    crate::globals::set_eqtb_b32s1(BOX_BASE + n as i32, crate::mem::TEX_NULL);
}

// ---------------------------------------------------------------------------
// its_all_over
// ---------------------------------------------------------------------------

const PAGE_HEAD: i32 = 4_999_997; // MEM_TOP - 2 (xetex_format.h:30)
const DIMEN_PAR_HSIZE: i32 = 3;   // xetex_format.h:125
const NULL_FLAG: i32 = -0x4000_0000; // xetex-constants.h:16

// translated from xetex-xetex0.c:9332 (its_all_over)
#[no_mangle]
pub unsafe extern "C" fn its_all_over() -> bool {
    if privileged() {
        if crate::globals::page_tail() == PAGE_HEAD
            && crate::globals::cur_list_head() == crate::globals::cur_list_tail()
            && crate::globals::dead_cycles() == 0
        {
            return true;
        }
        crate::input::back_input();
        let nb = crate::mem::new_null_box();
        set_mem_b32s1(crate::globals::cur_list_tail(), nb);
        crate::globals::set_cur_list_tail(nb);
        set_mem_b32s1(crate::globals::cur_list_tail() + 1,
                      eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_HSIZE));
        let ng = crate::mem::new_glue(8);
        set_mem_b32s1(crate::globals::cur_list_tail(), ng);
        crate::globals::set_cur_list_tail(ng);
        let np = crate::mem::new_penalty(NULL_FLAG);
        set_mem_b32s1(crate::globals::cur_list_tail(), np);
        crate::globals::set_cur_list_tail(np);
        build_page();
    }
    false
}

// ---------------------------------------------------------------------------
// append_glue / append_kern
// ---------------------------------------------------------------------------

const SKIP_CODE: i16 = 4;   // xetex_format.h:674

// translated from xetex-xetex0.c:9358 (append_glue)
#[no_mangle]
pub unsafe extern "C" fn append_glue() {
    let s = cur_chr() as i16;
    match s {
        0 => set_cur_val(4),
        1 => set_cur_val(8),
        2 => set_cur_val(12),
        3 => set_cur_val(16),
        4 => scan_glue(GLUE_VAL),   // SKIP_CODE
        5 => scan_glue(MU_VAL_SCAN), // MSKIP_CODE
        _ => {}
    }
    let p = crate::mem::new_glue(cur_val());
    set_mem_b32s1(crate::globals::cur_list_tail(), p);
    crate::globals::set_cur_list_tail(p);
    if s >= SKIP_CODE {
        set_mem_b32s1(cur_val(), mem_b32s1(cur_val()) - 1);
        if s > SKIP_CODE {
            set_mem_b16s0(crate::globals::cur_list_tail(), MU_GLUE);
        }
    }
}

// translated from xetex-xetex0.c:9395 (append_kern)
#[no_mangle]
pub unsafe extern "C" fn append_kern() {
    let s = cur_chr() as u16;
    scan_dimen(s == MU_GLUE, false, false);
    let p = crate::mem::new_kern(cur_val());
    set_mem_b32s1(crate::globals::cur_list_tail(), p);
    crate::globals::set_cur_list_tail(p);
    set_mem_b16s0(crate::globals::cur_list_tail(), s);
}

// ---------------------------------------------------------------------------
// make_mark
// ---------------------------------------------------------------------------

const MARK_NODE: u16 = 4; // xetex_format.h:384
const SMALL_NODE_SIZE: i32 = 2; // xetex-constants.h:110

// translated from xetex-xetex0.c:9896 (make_mark)
#[no_mangle]
pub unsafe extern "C" fn make_mark() {
    let c = if cur_chr() == 0 {
        0
    } else {
        scan_register_num();
        cur_val()
    };
    scan_toks(false, true); // side effect: sets def_ref; return value discarded
    let p = crate::mem::get_node(SMALL_NODE_SIZE);
    set_mem_b32s0(p + 1, c);
    set_mem_b16s1(p, MARK_NODE);  // NODE_type(p)
    set_mem_b16s0(p, 0);
    set_mem_b32s1(p + 1, def_ref());
    set_mem_b32s1(crate::globals::cur_list_tail(), p);
    crate::globals::set_cur_list_tail(p);
}

// ---------------------------------------------------------------------------
// new_interaction
// ---------------------------------------------------------------------------

const SELECTOR_NO_PRINT: i32 = 16;  // xetex-core.h:45
const SELECTOR_TERM_ONLY: i32 = 17; // xetex-core.h:46
const BATCH_MODE: u8 = 0;            // xetex_format.h:644

// translated from xetex-xetex0.c:11040 (new_interaction)
#[no_mangle]
pub unsafe extern "C" fn new_interaction() {
    crate::output::print_ln();
    crate::globals::set_interaction(cur_chr() as u8);
    if crate::globals::interaction() == BATCH_MODE {
        set_selector(SELECTOR_NO_PRINT);
    } else {
        set_selector(SELECTOR_TERM_ONLY);
    }
    if crate::globals::log_opened() {
        set_selector(selector() + 2);
    }
}

// ---------------------------------------------------------------------------
// alter_aux / alter_prev_graf / alter_page_so_far / alter_integer / alter_box_dimen
// ---------------------------------------------------------------------------

const ERROR_STOP_MODE: u8 = 3; // xetex_format.h:647

// translated from xetex-xetex0.c:10774 (alter_aux)
#[no_mangle]
pub unsafe extern "C" fn alter_aux() {
    if cur_chr() != cur_list_mode().unsigned_abs() as i32 {
        report_illegal_case();
    } else {
        let c = cur_chr();
        scan_optional_equals();
        if c == VMODE as i32 {
            scan_dimen(false, false, false);
            crate::globals::set_cur_list_aux_b32s1(cur_val());
        } else {
            scan_int();
            if cur_val() <= 0 || cur_val() > 32767 {
                let errmsg = error_here_with_diagnostic(c"Bad space factor".as_ptr());
                ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(1);
                set_help_line(0, c"I allow only values in the range 1..32767 here.".as_ptr());
                int_error(cur_val());
            } else {
                crate::globals::set_cur_list_aux_b32s0(cur_val());
            }
        }
    }
}

// translated from xetex-xetex0.c:10805 (alter_prev_graf)
#[no_mangle]
pub unsafe extern "C" fn alter_prev_graf() {
    crate::globals::save_cur_list_to_nest(crate::globals::nest_ptr());
    let mut p = crate::globals::nest_ptr();
    while crate::globals::nest_mode(p).unsigned_abs() != VMODE as u16 {
        p -= 1;
    }
    scan_optional_equals();
    scan_int();
    if cur_val() < 0 {
        let errmsg = error_here_with_diagnostic(c"Bad ".as_ptr());
        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
        crate::output::print_esc_cstr(c"prevgraf".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(1);
        set_help_line(0, c"I allow only nonnegative values here.".as_ptr());
        int_error(cur_val());
    } else {
        crate::globals::set_nest_prev_graf(p, cur_val());
        crate::globals::restore_cur_list_from_nest(crate::globals::nest_ptr());
    }
}

// translated from xetex-xetex0.c:10837 (alter_page_so_far)
#[no_mangle]
pub unsafe extern "C" fn alter_page_so_far() {
    let c = cur_chr() as usize;
    scan_optional_equals();
    scan_dimen(false, false, false);
    crate::globals::set_page_so_far(c, cur_val());
}

// translated from xetex-xetex0.c:10846 (alter_integer)
#[no_mangle]
pub unsafe extern "C" fn alter_integer() {
    let c = cur_chr() as u8;
    scan_optional_equals();
    scan_int();
    if c == 0 {
        crate::globals::set_dead_cycles(cur_val());
    } else if c == 2 {
        if cur_val() < BATCH_MODE as i32 || cur_val() > ERROR_STOP_MODE as i32 {
            let errmsg = error_here_with_diagnostic(c"Bad interaction mode".as_ptr());
            ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(2);
            set_help_line(1, c"Modes are 0=batch, 1=nonstop, 2=scroll, and".as_ptr());
            set_help_line(0, c"3=errorstop. Proceed, and I'll ignore this case.".as_ptr());
            int_error(cur_val());
        } else {
            set_cur_chr(cur_val());
            new_interaction();
        }
    } else {
        crate::globals::set_insert_penalties(cur_val());
    }
}

// translated from xetex-xetex0.c:10874 (alter_box_dimen)
#[no_mangle]
pub unsafe extern "C" fn alter_box_dimen() {
    let c = cur_chr();
    scan_register_num();
    let b = if cur_val() < 256 {
        eqtb_b32s1(BOX_BASE + cur_val())
    } else {
        crate::save::find_sa_element(4, cur_val(), false);
        if crate::globals::cur_ptr() == crate::mem::TEX_NULL {
            crate::mem::TEX_NULL
        } else {
            mem_b32s1(crate::globals::cur_ptr() + 1)
        }
    };
    scan_optional_equals();
    scan_dimen(false, false, false);
    if b != crate::mem::TEX_NULL {
        set_mem_b32s1(b + c, cur_val());
    }
}

// ---------------------------------------------------------------------------
// trap_zero_glue
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:10570 (trap_zero_glue)
#[no_mangle]
pub unsafe extern "C" fn trap_zero_glue() {
    if mem_b32s1(cur_val() + 1) == 0
        && mem_b32s1(cur_val() + 2) == 0
        && mem_b32s1(cur_val() + 3) == 0
    {
        // GLUE_SPEC_ref_count(0)++ = mem[0].b32.s1++
        set_mem_b32s1(0, mem_b32s1(0) + 1);
        crate::mem::delete_glue_ref(cur_val());
        set_cur_val(0);
    }
}

// ---------------------------------------------------------------------------
// fix_language
// ---------------------------------------------------------------------------

const INT_PAR_LANGUAGE: i32 = 50;          // xetex_format.h:85
const INT_PAR_LEFT_HYPHEN_MIN: i32 = 51;   // xetex_format.h:86
const INT_PAR_RIGHT_HYPHEN_MIN: i32 = 52;  // xetex_format.h:87
const LANGUAGE_NODE_SUBTYPE: i16 = 4;      // xetex_format.h:461 (whatsit subtype)

// translated from xetex-xetex0.c:11357 (fix_language)
#[no_mangle]
pub unsafe extern "C" fn fix_language() {
    let lang = eqtb_b32s1(INT_BASE + INT_PAR_LANGUAGE);
    let l = if lang <= 0 || lang > 255 { 0i32 } else { lang };
    if l != crate::globals::cur_list_aux_b32s1() {
        crate::mem::new_whatsit(LANGUAGE_NODE_SUBTYPE, SMALL_NODE_SIZE as i16);
        let tail = crate::globals::cur_list_tail();
        set_mem_b32s1(tail + 1, l);
        crate::globals::set_cur_list_aux_b32s1(l);
        set_mem_b16s1(tail + 1,
                      crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as u16);
        set_mem_b16s0(tail + 1,
                      crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as u16);
    }
}

// ---------------------------------------------------------------------------
// off_save
// ---------------------------------------------------------------------------

const BOTTOM_LEVEL: u8 = 0; // xetex_format.h (group level 0)
const FROZEN_END_GROUP: i32 = 2_243_226 + 2; // xetex_format.h:201 + xetex-constants.h:57
const FROZEN_RIGHT: i32 = 2_243_226 + 3;     // xetex_format.h:201 + xetex-constants.h:58

// translated from xetex-xetex0.c:9325 (off_save)
#[no_mangle]
pub unsafe extern "C" fn off_save() {
    if cur_group() == BOTTOM_LEVEL {
        error_here_with_diagnostic(c"Extra ".as_ptr());
        print_cmd_chr(cur_cmd() as u16, cur_chr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(1);
        set_help_line(0, c"Things are pretty mixed up, but I think the worst is over.".as_ptr());
        error();
    } else {
        crate::input::back_input();
        let p = crate::mem::get_avail();
        set_mem_b32s1(TEMP_HEAD, p);
        error_here_with_diagnostic(c"Missing ".as_ptr());
        match cur_group() {
            SEMI_SIMPLE_GROUP => {
                set_mem_b32s0(p, CS_TOKEN_FLAG + FROZEN_END_GROUP);
                crate::output::print_esc_cstr(c"endgroup".as_ptr());
            }
            MATH_SHIFT_GROUP => {
                set_mem_b32s0(p, MATH_SHIFT_TOKEN + b'$' as i32);
                crate::output::print_char(b'$' as i32);
            }
            MATH_LEFT_GROUP => {
                set_mem_b32s0(p, CS_TOKEN_FLAG + FROZEN_RIGHT);
                let q = crate::mem::get_avail();
                set_mem_b32s1(p, q);
                let p2 = mem_b32s1(p); // = LLIST_link(p)
                set_mem_b32s0(p2, OTHER_TOKEN + b'.' as i32);
                crate::output::print_esc_cstr(c"right.".as_ptr());
            }
            _ => {
                set_mem_b32s0(p, RIGHT_BRACE_TOKEN + b'}' as i32);
                crate::output::print_char(b'}' as i32);
            }
        }
        crate::output::print_cstr(c" inserted".as_ptr());
        begin_token_list(mem_b32s1(TEMP_HEAD), INSERTED);
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(5);
        set_help_line(4, c"I've inserted something that you may have forgotten.".as_ptr());
        set_help_line(3, c"(See the <inserted text> above.)".as_ptr());
        set_help_line(2, c"With luck, this will get me unwedged. But if you".as_ptr());
        set_help_line(1, c"really didn't forget anything, try typing `2' now; then".as_ptr());
        set_help_line(0, c"my insertion and my current dilemma will both disappear.".as_ptr());
        error();
    }
}

// ---------------------------------------------------------------------------
// indent_in_hmode
// ---------------------------------------------------------------------------

const HMODE_I: i16 = 104;  // xetex_format.h:369
const SUB_BOX: i32 = 2;    // xetex_format.h:283

// translated from xetex-xetex0.c:9808 (indent_in_hmode)
#[no_mangle]
pub unsafe extern "C" fn indent_in_hmode() {
    if cur_chr() > 0 {
        let p = crate::mem::new_null_box();
        set_mem_b32s1(p + 1, eqtb_b32s1(DIMEN_BASE));
        let p = if cur_list_mode().abs() == HMODE_I {
            crate::globals::set_cur_list_aux_b32s0(1000);
            p
        } else {
            let q = crate::mem::new_noad();
            set_mem_b32s1(q + 1, SUB_BOX);
            set_mem_b32s0(q + 1, p);
            q
        };
        let tail = crate::globals::cur_list_tail();
        set_mem_b32s1(tail, p);
        crate::globals::set_cur_list_tail(mem_b32s1(tail));
    }
}

// ---------------------------------------------------------------------------
// head_for_vmode
// ---------------------------------------------------------------------------

const HRULE: u8 = 36; // xetex_format.h:274

// translated from xetex-xetex0.c:9831 (head_for_vmode)
#[no_mangle]
pub unsafe extern "C" fn head_for_vmode() {
    if cur_list_mode() < 0 {
        if cur_cmd() != HRULE {
            off_save();
        } else {
            error_here_with_diagnostic(c"You can't use `".as_ptr());
            crate::output::print_esc_cstr(c"hrule".as_ptr());
            crate::output::print_cstr(c"' here except with leaders".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(2);
            set_help_line(1, c"To put a horizontal rule in an hbox or an alignment,".as_ptr());
            set_help_line(0, c"you should use \\leaders or \\hrulefill (see The TeXbook).".as_ptr());
            error();
        }
    } else {
        crate::input::back_input();
        set_cur_tok(crate::globals::par_token());
        crate::input::back_input();
        set_cur_input_index(INSERTED as u16);
    }
}

// ---------------------------------------------------------------------------
// append_src_special
// ---------------------------------------------------------------------------

const SPECIAL_NODE: i16 = 3;     // xetex_format.h:460
const WRITE_NODE_SIZE: i16 = 2;  // xetex-constants.h:112

// translated from xetex-xetex0.c:11376 (append_src_special)
#[no_mangle]
pub unsafe extern "C" fn append_src_special() {
    let fname = crate::globals::source_filename_stack(crate::globals::in_open());
    if fname > 0 && crate::texmfmp::is_new_source(fname, crate::globals::line()) {
        crate::mem::new_whatsit(SPECIAL_NODE, WRITE_NODE_SIZE);
        set_mem_b32s0(crate::globals::cur_list_tail() + 1, 0);
        let d = crate::mem::get_avail();
        set_def_ref(d);
        set_mem_b32s0(d, crate::mem::TEX_NULL);
        str_toks(crate::texmfmp::make_src_special(fname, crate::globals::line()));
        set_mem_b32s1(d, mem_b32s1(TEMP_HEAD));
        set_mem_b32s1(crate::globals::cur_list_tail() + 1, d);
        crate::texmfmp::remember_source_info(fname, crate::globals::line());
    }
}

// ---------------------------------------------------------------------------
// tokens_to_string
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:13060 (tokens_to_string)
#[no_mangle]
pub unsafe extern "C" fn tokens_to_string(p: i32) -> i32 {
    if selector() == SELECTOR_NEW_STRING {
        crate::errors::pdf_error(c"tokens".as_ptr(), c"tokens_to_string() called while selector = new_string".as_ptr());
    }
    set_old_setting(selector() as u8);
    set_selector(SELECTOR_NEW_STRING);
    crate::output::show_token_list(mem_b32s1(p), crate::mem::TEX_NULL, pool_size() - pool_ptr());
    set_selector(old_setting() as i32);
    crate::string_pool::make_string()
}

// ---------------------------------------------------------------------------
// compare_strings
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:13073 (compare_strings)
#[no_mangle]
pub unsafe extern "C" fn compare_strings() {
    let save_cur_cs = cur_cs();
    scan_toks(false, true);
    let s1 = tokens_to_string(def_ref());
    crate::mem::delete_token_ref(def_ref());
    set_cur_cs(save_cur_cs);
    scan_toks(false, true);
    let s2 = tokens_to_string(def_ref());
    crate::mem::delete_token_ref(def_ref());

    let i1 = str_start(s1 - 65536);
    let j1 = str_start(s1 + 1 - 65536);
    let i2 = str_start(s2 - 65536);
    let j2 = str_start(s2 + 1 - 65536);

    let result = 'done: {
        let mut i1 = i1;
        let mut i2 = i2;
        while i1 < j1 && i2 < j2 {
            if str_pool_entry(i1) < str_pool_entry(i2) {
                break 'done -1i32;
            }
            if str_pool_entry(i1) > str_pool_entry(i2) {
                break 'done 1i32;
            }
            i1 += 1;
            i2 += 1;
        }
        if i1 == j1 && i2 == j2 {
            0i32
        } else if i1 < j1 {
            1i32
        } else {
            -1i32
        }
    };

    crate::string_pool::flush_str(s2);
    crate::string_pool::flush_str(s1);
    set_cur_val(result);
    set_cur_val_level(INT_VAL as u8);
}

// ---------------------------------------------------------------------------
// new_write_whatsit
// ---------------------------------------------------------------------------

const OPEN_NODE_SIZE: i16 = 3; // xetex-constants.h:118

// translated from xetex-xetex0.c:11045 (new_write_whatsit)
#[no_mangle]
pub unsafe extern "C" fn new_write_whatsit(w: i16) {
    crate::mem::new_whatsit(cur_chr() as i16, w);
    if w != WRITE_NODE_SIZE {
        scan_four_bit_int();
    } else {
        scan_int();
        if cur_val() < 0 {
            set_cur_val(17);
        } else if cur_val() > 15 && cur_val() != 18 {
            set_cur_val(16);
        }
    }
    set_mem_b32s0(crate::globals::cur_list_tail() + 1, cur_val());
}

// ---------------------------------------------------------------------------
// begin_insert_or_adjust
// ---------------------------------------------------------------------------

const VADJUST: u8 = 38;         // xetex_format.h:276
const INSERT_GROUP: u8 = 11;    // xetex-constants.h:163
const IGNORE_DEPTH: i32 = -65_536_000; // xetex_format.h:388

// translated from xetex-xetex0.c:9760 (begin_insert_or_adjust)
#[no_mangle]
pub unsafe extern "C" fn begin_insert_or_adjust() {
    if cur_cmd() == VADJUST {
        set_cur_val(255);
    } else {
        scan_eight_bit_int();
        if cur_val() == 255 {
            error_here_with_diagnostic(c"You can't ".as_ptr());
            crate::output::print_esc_cstr(c"insert".as_ptr());
            crate::output::print_int(cur_val());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(1);
            set_help_line(0, c"I'm changing to \\insert0; box 255 is special.".as_ptr());
            error();
            set_cur_val(0);
        }
    }
    set_save_stack_b32s1(save_ptr(), cur_val());
    if cur_cmd() == VADJUST && scan_keyword(c"pre".as_ptr()) {
        set_save_stack_b32s1(save_ptr() + 1, 1);
    } else {
        set_save_stack_b32s1(save_ptr() + 1, 0);
    }
    set_save_ptr(save_ptr() + 2);
    crate::save::new_save_level(INSERT_GROUP);
    scan_left_brace();
    normal_paragraph();
    crate::mem::push_nest();
    crate::globals::set_cur_list_mode(-1); // -VMODE
    crate::globals::set_cur_list_aux_b32s1(IGNORE_DEPTH);
}

// ---------------------------------------------------------------------------
// do_endv
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:10161 (do_endv)
#[no_mangle]
pub unsafe extern "C" fn do_endv() {
    const V_TEMPLATE: u16 = 2;   // xetex-constants.h:201
    const TOKEN_LIST: u16 = 0;   // xetex-constants.h:261
    const ALIGN_GROUP: u8 = 6;   // xetex-constants.h:158

    crate::globals::set_base_ptr(crate::globals::input_ptr());
    crate::globals::set_input_stack(crate::globals::base_ptr(), crate::globals::cur_input());
    while crate::globals::input_stack(crate::globals::base_ptr()).index != V_TEMPLATE
        && crate::globals::input_stack(crate::globals::base_ptr()).loc == crate::mem::TEX_NULL
        && crate::globals::input_stack(crate::globals::base_ptr()).state == TOKEN_LIST
    {
        crate::globals::set_base_ptr(crate::globals::base_ptr() - 1);
    }
    if crate::globals::input_stack(crate::globals::base_ptr()).index != V_TEMPLATE
        || crate::globals::input_stack(crate::globals::base_ptr()).loc != crate::mem::TEX_NULL
        || crate::globals::input_stack(crate::globals::base_ptr()).state != TOKEN_LIST
    {
        crate::errors::fatal_error(c"(interwoven alignment preambles are not allowed)".as_ptr());
    }
    if cur_group() == ALIGN_GROUP {
        end_graf();
        if fin_col() {
            fin_row();
        }
    } else {
        off_save();
    }
}

// ---------------------------------------------------------------------------
// get_r_token
// ---------------------------------------------------------------------------

const FROZEN_CONTROL_SEQUENCE: i32 = 2_243_226; // xetex_format.h:201
const EQTB_SIZE: i32 = 8_941_458;               // xetex_format.h:224
const FROZEN_PROTECTION: i32 = FROZEN_CONTROL_SEQUENCE; // xetex-constants.h:55

// translated from xetex-xetex0.c:10407 (get_r_token)
#[no_mangle]
pub unsafe extern "C" fn get_r_token() {
    loop {
        loop {
            get_token();
            if cur_tok() != SPACE_TOKEN { break; }
        }
        if cur_cs() == 0
            || cur_cs() > crate::globals::eqtb_top()
            || (cur_cs() > FROZEN_CONTROL_SEQUENCE && cur_cs() <= EQTB_SIZE)
        {
            error_here_with_diagnostic(c"Missing control sequence inserted".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(5);
            set_help_line(4, c"Please don't say `\\def cs{...}', say `\\def\\cs{...}'.".as_ptr());
            set_help_line(3, c"I've inserted an inaccessible control sequence so that your".as_ptr());
            set_help_line(2, c"definition will be completed without mixing me up too badly.".as_ptr());
            set_help_line(1, c"You can recover graciously from this error, if you're".as_ptr());
            set_help_line(0, c"careful; see exercise 27.2 in The TeXbook.".as_ptr());
            if cur_cs() == 0 { crate::input::back_input(); }
            set_cur_tok(CS_TOKEN_FLAG + FROZEN_PROTECTION);
            crate::input::ins_error();
            // goto restart
        } else {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// append_discretionary
// ---------------------------------------------------------------------------

const DISC_GROUP: u8 = 10; // xetex-constants.h:162

// translated from xetex-xetex0.c:9956 (append_discretionary)
#[no_mangle]
pub unsafe extern "C" fn append_discretionary() {
    extern "C" {
        fn new_character(f: i32, c: u16) -> i32;
    }

    let tail = crate::globals::cur_list_tail();
    set_mem_b32s1(tail, crate::mem::new_disc());
    crate::globals::set_cur_list_tail(mem_b32s1(tail));

    if cur_chr() == 1 {
        let f = eqtb_b32s1(CUR_FONT_LOC);
        let c = crate::globals::hyphen_char_entry(f);
        if c >= 0 && c <= BIGGEST_CHAR {
            let tail2 = crate::globals::cur_list_tail();
            set_mem_b32s0(tail2 + 1, new_character(f, c as u16));
        }
    } else {
        set_save_stack_b32s1(save_ptr(), 0);
        set_save_ptr(save_ptr() + 1);
        crate::save::new_save_level(DISC_GROUP);
        scan_left_brace();
        crate::mem::push_nest();
        crate::globals::set_cur_list_mode(-104); // -HMODE
        crate::globals::set_cur_list_aux_b32s0(1000);
    }
}

// ---------------------------------------------------------------------------
// shift_case
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:10765 (shift_case)
#[no_mangle]
pub unsafe extern "C" fn shift_case() {
    let b = cur_chr();
    scan_toks(false, false);
    let mut p = mem_b32s1(def_ref());
    while p != crate::mem::TEX_NULL {
        let t = mem_b32s0(p);
        if t < CS_TOKEN_FLAG + SINGLE_BASE {
            let c = t % MAX_CHAR_VAL;
            if eqtb_b32s1(b + c) != 0 {
                set_mem_b32s0(p, t - c + eqtb_b32s1(b + c));
            }
        }
        p = mem_b32s1(p);
    }
    begin_token_list(mem_b32s1(def_ref()), BACKED_UP);
    set_mem_b32s1(def_ref(), avail());
    set_avail(def_ref());
}

// ---------------------------------------------------------------------------
// build_discretionary
// ---------------------------------------------------------------------------

const WHATSIT_NODE: u16 = 8;          // xetex_format.h:388
const NATIVE_WORD_NODE: u16 = 40;     // xetex_format.h:463
const NATIVE_WORD_NODE_AT: u16 = 41;  // xetex_format.h:464
const GLYPH_NODE_BD: u16 = 42;        // xetex_format.h:465
const MMODE: i16 = 207;               // xetex_format.h:370

// translated from xetex-xetex0.c:9958 (build_discretionary)
#[no_mangle]
pub unsafe extern "C" fn build_discretionary() {
    crate::save::unsave();
    let q_head = crate::globals::cur_list_head();
    let mut q = q_head;
    let mut p = mem_b32s1(q);
    let mut n: i32 = 0;

    'walk: loop {
        if p == crate::mem::TEX_NULL { break; }
        if p < crate::globals::hi_mem_min() {
            // not a char node — check node type
            let ntype = mem_b16s1(p);
            const RULE_NODE_BD: u16 = 2;  // xetex_format.h:387
            const LIGATURE_NODE: u16 = 6; // xetex_format.h (= mem.rs)
            if ntype > RULE_NODE_BD {
                let ok = ntype == KERN_NODE   // 11
                    || ntype == LIGATURE_NODE  // 6
                    || ntype == WHATSIT_NODE && {
                        let sub = mem_b16s0(p);
                        sub == NATIVE_WORD_NODE || sub == NATIVE_WORD_NODE_AT || sub == GLYPH_NODE_BD
                    };
                if !ok {
                    error_here_with_diagnostic(c"Improper discretionary list".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(1);
                    set_help_line(0, c"Discretionary lists must contain only boxes and kerns.".as_ptr());
                    error();

                    crate::output::begin_diagnostic();
                    crate::output::diagnostic_begin_capture_warning_here();
                    crate::output::print_nl_cstr(c"The following discretionary sublist has been deleted:".as_ptr());
                    show_box(p);
                    capture_to_diagnostic(std::ptr::null_mut());
                    crate::output::end_diagnostic(true);

                    flush_node_list(p);
                    set_mem_b32s1(q, crate::mem::TEX_NULL);
                    break 'walk;
                }
            }
        }
        q = p;
        p = mem_b32s1(q);
        n += 1;
    }

    p = mem_b32s1(q_head);
    crate::mem::pop_nest();

    let slot = save_stack_b32s1(save_ptr() - 1);
    match slot {
        0 => {
            set_mem_b32s0(crate::globals::cur_list_tail() + 1, p);
        }
        1 => {
            set_mem_b32s1(crate::globals::cur_list_tail() + 1, p);
        }
        2 => {
            if n > 0 && cur_list_mode().abs() == MMODE {
                error_here_with_diagnostic(c"Illegal math ".as_ptr());
                crate::output::print_esc_cstr(c"discretionary".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(1, c"Sorry: The third part of a discretionary break must be".as_ptr());
                set_help_line(0, c"empty, in math formulas. I had to delete your third part.".as_ptr());
                flush_node_list(p);
                n = 0;
                error();
            } else {
                set_mem_b32s1(crate::globals::cur_list_tail(), p);
            }
            if n <= u16::MAX as i32 {
                set_mem_b16s0(crate::globals::cur_list_tail(), n as u16);
            } else {
                error_here_with_diagnostic(c"Discretionary list is too long".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(1, c"Wow---I never thought anybody would tweak me here.".as_ptr());
                set_help_line(0, c"You can't seriously need such a huge discretionary list?".as_ptr());
                error();
            }
            if n > 0 {
                crate::globals::set_cur_list_tail(q);
            }
            set_save_ptr(save_ptr() - 1);
            return;
        }
        _ => {}
    }
    set_save_stack_b32s1(save_ptr() - 1, slot + 1);
    crate::save::new_save_level(DISC_GROUP);
    scan_left_brace();
    crate::mem::push_nest();
    crate::globals::set_cur_list_mode(-104); // -HMODE
    crate::globals::set_cur_list_aux_b32s0(1000);
}

// ---------------------------------------------------------------------------
// issue_message
// ---------------------------------------------------------------------------

const LOCAL__ERR_HELP: i32 = 9;    // xetex_format.h:181
const ERROR_STOP_MODE_IM: u8 = 3;  // xetex-constants.h
const TOO_BIG_CHAR_IM: i32 = 65536; // string_pool.rs

// translated from xetex-xetex0.c:10708 (issue_message)
#[no_mangle]
pub unsafe extern "C" fn issue_message() {
    let c = cur_chr() as u8;
    set_mem_b32s1(GARBAGE, scan_toks(false, true));
    let old_setting = selector() as u8;
    set_selector(SELECTOR_NEW_STRING);
    crate::output::token_show(def_ref());
    set_selector(old_setting as i32);
    crate::mem::flush_list(def_ref());
    if pool_ptr() + 1 > pool_size() {
        crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr());
    }
    let s = crate::string_pool::make_string();
    if c == 0 {
        let len = crate::string_pool::length(s);
        if term_offset() + len > max_print_line() - 2 {
            crate::output::print_ln();
        } else if term_offset() > 0 || file_offset() > 0 {
            crate::output::print_char(b' ' as i32);
        }
        crate::output::print(s);
        ttstub_output_flush(rust_stdout());
    } else {
        error_here_with_diagnostic(c"".as_ptr());
        crate::output::print(s);
        capture_to_diagnostic(std::ptr::null_mut());
        let err_help = eqtb_b32s1(LOCAL_BASE + LOCAL__ERR_HELP);
        if err_help != crate::mem::TEX_NULL {
            crate::globals::set_use_err_help(true);
        } else if crate::globals::long_help_seen() {
            set_help_ptr(1);
            set_help_line(0, c"(That was another \\errmessage.)".as_ptr());
        } else {
            if crate::globals::interaction() < ERROR_STOP_MODE_IM {
                crate::globals::set_long_help_seen(true);
            }
            set_help_ptr(4);
            set_help_line(3, c"This error message was generated by an \\errmessage".as_ptr());
            set_help_line(2, c"command, so I can't give any explicit help.".as_ptr());
            set_help_line(1, c"Pretend that you're Hercule Poirot: Examine all clues,".as_ptr());
            set_help_line(0, c"and deduce the truth by order and method.".as_ptr());
        }
        error();
        crate::globals::set_use_err_help(false);
    }
    // pop the temporary string (equivalent to flush_str(s) when s == str_ptr-1)
    set_str_ptr(str_ptr() - 1);
    set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR_IM));
}

// ---------------------------------------------------------------------------
// append_to_vlist
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:8020 (append_to_vlist)
#[no_mangle]
pub unsafe extern "C" fn append_to_vlist(b: i32) {
    const GLUE_BASE: i32 = 2_254_340;          // xetex_format.h:205
    const INT_PAR_XETEX_UPWARDS: i32 = 71;     // xetex_format.h:106
    const GLUE_PAR_BASELINE_SKIP: i32 = 1;     // xetex_format.h:150
    const GLUE_PAR_LINE_SKIP: i16 = 0;         // xetex_format.h:149
    const DIMEN_PAR_LINE_SKIP_LIMIT: i32 = 2;  // xetex_format.h:124

    let upwards = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS) > 0;
    if crate::globals::cur_list_aux_b32s1() > IGNORE_DEPTH {
        let baseline_spec = eqtb_b32s1(GLUE_BASE + GLUE_PAR_BASELINE_SKIP);
        let d = mem_b32s1(baseline_spec + 1)
            - crate::globals::cur_list_aux_b32s1()
            - if upwards { mem_b32s1(b + 2) } else { mem_b32s1(b + 3) };
        let p = if d < eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_LINE_SKIP_LIMIT) {
            crate::mem::new_param_glue(GLUE_PAR_LINE_SKIP)
        } else {
            let p = crate::mem::new_skip_param(GLUE_PAR_BASELINE_SKIP as i16);
            set_mem_b32s1(crate::globals::temp_ptr() + 1, d);
            p
        };
        set_mem_b32s1(crate::globals::cur_list_tail(), p);
        crate::globals::set_cur_list_tail(p);
    }
    set_mem_b32s1(crate::globals::cur_list_tail(), b);
    crate::globals::set_cur_list_tail(b);
    crate::globals::set_cur_list_aux_b32s1(
        if upwards { mem_b32s1(b + 3) } else { mem_b32s1(b + 2) }
    );
}

// ---------------------------------------------------------------------------
// fin_align
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:8040 (fin_align)
#[no_mangle]
pub unsafe extern "C" fn fin_align() {
    use crate::globals::{
        cur_group, cur_list_aux_b32s0, cur_list_aux_b32s1, cur_list_e_tex_aux, cur_list_head,
        cur_list_mode, cur_list_mode_line, cur_list_tail, mem_b16s0, mem_b16s1, mem_b32s0,
        mem_b32s1, mem_gr, nest_mode, nest_ptr, save_ptr, save_stack_b32s1, set_cur_list_aux_b32s0,
        set_cur_list_aux_b32s1, set_cur_list_tail, set_eqtb_b32s1, set_mem_b16s0, set_mem_b16s1,
        set_mem_b32s0, set_mem_b32s1, set_mem_gr, set_pack_begin_line, set_save_ptr,
    };
    use crate::mem::TEX_NULL;

    const ALIGN_GROUP: u8 = 6;                 // xetex-constants.h:158
    const ALIGN_HEAD: i32 = 4_999_991;         // MEM_TOP - 8
    const END_SPAN: i32 = 4_999_990;           // MEM_TOP - 9
    const HOLD_HEAD: i32 = 4_999_995;          // MEM_TOP - 4
    const NULL_FLAG: i32 = -0x4000_0000;       // xetex-constants.h:16
    const MMODE: i16 = 207;
    const INNER_VMODE: i16 = -1;               // -VMODE
    const VMODE: i16 = 1;
    const HLIST_NODE: u16 = 0;                 // xetex_format.h:380
    const VLIST_NODE: u16 = 1;                 // xetex_format.h:381
    const RULE_NODE: u16 = 2;                  // xetex_format.h:382
    const UNSET_NODE: u16 = 13;
    const SPAN_NODE_SIZE: i32 = 2;
    const ADDITIONAL: i16 = 1;
    const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
    const STRETCHING: u16 = 1;                 // xetex-constants.h:275
    const SHRINKING: u16 = 2;                  // xetex-constants.h:281
    const DLIST: u16 = 2;                      // xetex-constants.h:278
    const NORMAL: u16 = 0;
    // GLUE_PAR__above_display_skip = 3, GLUE_PAR__below_display_skip = 4
    const GLUE_PAR_ABOVE_DISPLAY: i16 = 3;
    const GLUE_PAR_BELOW_DISPLAY: i16 = 4;
    const DIMEN_BASE: i32 = 8_941_180;
    const DIMEN_PAR_DISPLAY_INDENT: i32 = 15;
    const DIMEN_PAR_OVERFULL_RULE: i32 = 16;
    const INT_PAR_PRE_DISPLAY_PENALTY: i32 = 11;
    const INT_PAR_POST_DISPLAY_PENALTY: i32 = 12;
    const MATH_SHIFT: u8 = 3;                  // xetex_format.h:233
    const GLUE_PAR_TAB_SKIP: u16 = 12;         // GLUE_PAR__tab_skip + 1

    if cur_group() != ALIGN_GROUP {
        crate::errors::confusion(c"align1".as_ptr());
    }
    crate::save::unsave();
    if cur_group() != ALIGN_GROUP {
        crate::errors::confusion(c"align0".as_ptr());
    }
    crate::save::unsave();

    let o: i32 = if nest_mode(nest_ptr() - 1) == MMODE {
        eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT)
    } else {
        0
    };

    // §830: walk preamble, compute column widths, handle span nodes
    let mut q = mem_b32s1(mem_b32s1(ALIGN_HEAD));
    loop {
        crate::mem::flush_list(mem_b32s1(q + 3));
        crate::mem::flush_list(mem_b32s1(q + 2));
        let p = mem_b32s1(mem_b32s1(q));
        // §831: if NULL_FLAG width, clear the tab_skip glue leader
        if mem_b32s1(q + 1) == NULL_FLAG {
            set_mem_b32s1(q + 1, 0);
            let r = mem_b32s1(q);
            let s = mem_b32s0(r + 1);
            if s != 0 {
                set_mem_b32s1(0, mem_b32s1(0) + 1); // GLUE_SPEC_ref_count(0)++
                crate::mem::delete_glue_ref(s);
                set_mem_b32s0(r + 1, 0);
            }
        }
        // §832: handle span nodes — distribute width across spanned columns
        if mem_b32s0(q) != END_SPAN {
            let t = mem_b32s1(q + 1) + mem_b32s1(mem_b32s0(mem_b32s1(q) + 1) + 1);
            let mut r = mem_b32s0(q);
            let mut s = END_SPAN;
            set_mem_b32s0(s, p);
            let mut n: i32 = 1;
            loop {
                set_mem_b32s1(r + 1, mem_b32s1(r + 1) - t);
                let u = mem_b32s0(r);
                while mem_b32s1(r) > n {
                    s = mem_b32s0(s);
                    n = mem_b32s1(mem_b32s0(s)) + 1;
                }
                if mem_b32s1(r) < n {
                    set_mem_b32s0(r, mem_b32s0(s));
                    set_mem_b32s0(s, r);
                    set_mem_b32s1(r, mem_b32s1(r) - 1);
                    s = r;
                } else {
                    if mem_b32s1(r + 1) > mem_b32s1(mem_b32s0(s) + 1) {
                        set_mem_b32s1(mem_b32s0(s) + 1, mem_b32s1(r + 1));
                    }
                    crate::mem::free_node(r, SPAN_NODE_SIZE);
                }
                r = u;
                if r == END_SPAN { break; }
            }
        }
        // Clear the preamble node fields (convert to UNSET_NODE)
        set_mem_b16s1(q, UNSET_NODE);
        set_mem_b16s0(q, 0);
        set_mem_b32s1(q + 3, 0);
        set_mem_b32s1(q + 2, 0);
        set_mem_b16s0(q + 5, NORMAL);
        set_mem_b16s1(q + 5, NORMAL);
        set_mem_b32s1(q + 6, 0);
        set_mem_b32s1(q + 4, 0);
        q = p;
        if q == TEX_NULL { break; }
    }

    // §833: pop the spec from save_stack and pack the whole alignment
    set_save_ptr(save_ptr() - 2);
    set_pack_begin_line(-(cur_list_mode_line() as i32));
    let p = if cur_list_mode() == INNER_VMODE {
        let rule_save = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE);
        set_eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE, 0);
        let pp = hpack(
            mem_b32s1(ALIGN_HEAD),
            save_stack_b32s1(save_ptr() + 1),
            save_stack_b32s1(save_ptr()) as i16,
        );
        set_eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE, rule_save);
        pp
    } else {
        // vmode: copy widths to depth, pack, restore
        let mut qq = mem_b32s1(mem_b32s1(ALIGN_HEAD));
        loop {
            set_mem_b32s1(qq + 3, mem_b32s1(qq + 1));
            set_mem_b32s1(qq + 1, 0);
            qq = mem_b32s1(mem_b32s1(qq));
            if qq == TEX_NULL { break; }
        }
        let pp = vpackage(
            mem_b32s1(ALIGN_HEAD),
            save_stack_b32s1(save_ptr() + 1),
            save_stack_b32s1(save_ptr()) as i16,
            MAX_HALFWORD,
        );
        qq = mem_b32s1(mem_b32s1(ALIGN_HEAD));
        loop {
            set_mem_b32s1(qq + 1, mem_b32s1(qq + 3));
            set_mem_b32s1(qq + 3, 0);
            qq = mem_b32s1(mem_b32s1(qq));
            if qq == TEX_NULL { break; }
        }
        pp
    };
    set_pack_begin_line(0);

    // §836–840: walk the row list and resolve each UNSET_NODE
    q = mem_b32s1(cur_list_head());
    let mut s = cur_list_head();
    while q != TEX_NULL {
        if q < crate::globals::hi_mem_min() {  // !is_char_node
        if mem_b16s1(q) == UNSET_NODE {
            // §836: set this row/column to its final type and dimensions
            if cur_list_mode() == INNER_VMODE {
                set_mem_b16s1(q, HLIST_NODE);
                set_mem_b32s1(q + 1, mem_b32s1(p + 1));
                if nest_mode(nest_ptr() - 1) == MMODE {
                    set_mem_b16s0(q, DLIST);
                }
            } else {
                set_mem_b16s1(q, VLIST_NODE);
                set_mem_b32s1(q + 3, mem_b32s1(p + 3));
            }
            set_mem_b16s0(q + 5, mem_b16s0(p + 5));
            set_mem_b16s1(q + 5, mem_b16s1(p + 5));
            set_mem_gr(q + 6, mem_gr(p + 6));
            set_mem_b32s1(q + 4, o);

            // Walk the column UNSET_NODEs inside this row
            let mut r = mem_b32s1(mem_b32s1(q + 5));
            let mut ss = mem_b32s1(mem_b32s1(p + 5));
            loop {
                // §837: for each column unset node r, distribute glue
                let n = mem_b16s0(r);
                let t_base = mem_b32s1(ss + 1);
                let mut t = t_base;
                let mut u = HOLD_HEAD;
                set_mem_b16s0(r, 0);
                // Insert inter-column glue for spanned columns
                let mut span_count = n;
                while span_count > 0 {
                    span_count -= 1;
                    ss = mem_b32s1(ss);
                    let v = mem_b32s0(ss + 1);
                    set_mem_b32s1(u, crate::mem::new_glue(v));
                    u = mem_b32s1(u);
                    set_mem_b16s0(u, GLUE_PAR_TAB_SKIP);
                    t += mem_b32s1(v + 1);
                    if mem_b16s1(p + 5) == STRETCHING {
                        if mem_b16s1(v) == mem_b16s0(p + 5) {
                            t += crate::scaled_math::tex_round(mem_gr(p + 6) * mem_b32s1(v + 2) as f64);
                        }
                    } else if mem_b16s1(p + 5) == SHRINKING {
                        if mem_b16s0(v) == mem_b16s0(p + 5) {
                            t -= crate::scaled_math::tex_round(mem_gr(p + 6) * mem_b32s1(v + 3) as f64);
                        }
                    }
                    ss = mem_b32s1(ss);
                    set_mem_b32s1(u, crate::mem::new_null_box());
                    u = mem_b32s1(u);
                    t += mem_b32s1(ss + 1);
                    if cur_list_mode() == INNER_VMODE {
                        set_mem_b32s1(u + 1, mem_b32s1(ss + 1));
                    } else {
                        set_mem_b16s1(u, VLIST_NODE);
                        set_mem_b32s1(u + 3, mem_b32s1(ss + 1));
                    }
                }
                // §839/840: set glue ratio for r and finalize node type
                if cur_list_mode() == INNER_VMODE {
                    // §839: hbox column
                    set_mem_b32s1(r + 3, mem_b32s1(q + 3));
                    set_mem_b32s1(r + 2, mem_b32s1(q + 2));
                    if t == mem_b32s1(r + 1) {
                        set_mem_b16s1(r + 5, NORMAL);
                        set_mem_b16s0(r + 5, NORMAL);
                        set_mem_gr(r + 6, 0.0);
                    } else if t > mem_b32s1(r + 1) {
                        set_mem_b16s1(r + 5, STRETCHING);
                        if mem_b32s1(r + 6) == 0 {
                            set_mem_gr(r + 6, 0.0);
                        } else {
                            set_mem_gr(r + 6, (t - mem_b32s1(r + 1)) as f64 / mem_b32s1(r + 6) as f64);
                        }
                    } else {
                        set_mem_b16s0(r + 5, mem_b16s1(r + 5));
                        set_mem_b16s1(r + 5, SHRINKING);
                        if mem_b32s1(r + 4) == 0 {
                            set_mem_gr(r + 6, 0.0);
                        } else if mem_b16s0(r + 5) == NORMAL && mem_b32s1(r + 1) - t > mem_b32s1(r + 4) {
                            set_mem_gr(r + 6, 1.0);
                        } else {
                            set_mem_gr(r + 6, (mem_b32s1(r + 1) - t) as f64 / mem_b32s1(r + 4) as f64);
                        }
                    }
                    set_mem_b32s1(r + 1, t_base);
                    set_mem_b16s1(r, HLIST_NODE);
                } else {
                    // §840: vbox column
                    set_mem_b32s1(r + 1, mem_b32s1(q + 1));
                    if t == mem_b32s1(r + 3) {
                        set_mem_b16s1(r + 5, NORMAL);
                        set_mem_b16s0(r + 5, NORMAL);
                        set_mem_gr(r + 6, 0.0);
                    } else if t > mem_b32s1(r + 3) {
                        set_mem_b16s1(r + 5, STRETCHING);
                        if mem_b32s1(r + 6) == 0 {
                            set_mem_gr(r + 6, 0.0);
                        } else {
                            set_mem_gr(r + 6, (t - mem_b32s1(r + 3)) as f64 / mem_b32s1(r + 6) as f64);
                        }
                    } else {
                        set_mem_b16s0(r + 5, mem_b16s1(r + 5));
                        set_mem_b16s1(r + 5, SHRINKING);
                        if mem_b32s1(r + 4) == 0 {
                            set_mem_gr(r + 6, 0.0);
                        } else if mem_b16s0(r + 5) == NORMAL && mem_b32s1(r + 3) - t > mem_b32s1(r + 4) {
                            set_mem_gr(r + 6, 1.0);
                        } else {
                            set_mem_gr(r + 6, (mem_b32s1(r + 3) - t) as f64 / mem_b32s1(r + 4) as f64);
                        }
                    }
                    set_mem_b32s1(r + 3, t_base);
                    set_mem_b16s1(r, VLIST_NODE);
                }
                set_mem_b32s1(r + 4, 0);
                // Link any inserted inter-column glue nodes
                if u != HOLD_HEAD {
                    set_mem_b32s1(u, mem_b32s1(r));
                    set_mem_b32s1(r, mem_b32s1(HOLD_HEAD));
                    r = u;
                }
                r = mem_b32s1(mem_b32s1(r));
                ss = mem_b32s1(mem_b32s1(ss));
                if r == TEX_NULL { break; }
            }
        } else if mem_b16s1(q) == RULE_NODE {
            // §835: rule node — fill in any NULL_FLAG dimensions from the packed box
            if mem_b32s1(q + 1) == NULL_FLAG { set_mem_b32s1(q + 1, mem_b32s1(p + 1)); }
            if mem_b32s1(q + 3) == NULL_FLAG { set_mem_b32s1(q + 3, mem_b32s1(p + 3)); }
            if mem_b32s1(q + 2) == NULL_FLAG { set_mem_b32s1(q + 2, mem_b32s1(p + 2)); }
            if o != 0 {
                let r = mem_b32s1(q);
                set_mem_b32s1(q, TEX_NULL);
                let qq2 = hpack(q, 0, ADDITIONAL);
                set_mem_b32s1(qq2 + 4, o);
                set_mem_b32s1(qq2, r);
                set_mem_b32s1(s, qq2);
                q = qq2;
            }
        }
        } // end !is_char_node
        s = q;
        q = mem_b32s1(q);
    }

    flush_node_list(p);
    pop_alignment();

    // Save aux and pop nest to get back to the outer list
    let aux_save_s0 = cur_list_aux_b32s0();
    let aux_save_s1 = cur_list_aux_b32s1();
    let pp = mem_b32s1(cur_list_head());
    let qq = cur_list_tail();
    crate::mem::pop_nest();

    if cur_list_mode() == MMODE {
        // §1241: display math alignment — add penalties + display glue
        do_assignments();
        if cur_cmd() != MATH_SHIFT {
            error_here_with_diagnostic(c"Missing $$ inserted".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(2);
            set_help_line(1, c"Displays can use special alignments (like \\eqalignno)".as_ptr());
            set_help_line(0, c"only if nothing but the alignment itself is between $$'s.".as_ptr());
            crate::input::back_error();
        } else {
            // §1232: check for second $$
            get_x_token();
            if cur_cmd() != MATH_SHIFT {
                error_here_with_diagnostic(c"Display math should end with $$".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(1, c"The `$' that I just saw supposedly matches a previous `$$'.".as_ptr());
                set_help_line(0, c"So I shall assume that you typed `$$' both times.".as_ptr());
                crate::input::back_error();
            }
        }
        flush_node_list(cur_list_e_tex_aux());
        crate::mem::pop_nest();
        // Insert pre-display penalty and above-display skip
        set_mem_b32s1(cur_list_tail(), crate::mem::new_penalty(eqtb_b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_PENALTY)));
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        set_mem_b32s1(cur_list_tail(), crate::mem::new_param_glue(GLUE_PAR_ABOVE_DISPLAY));
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        // Attach alignment rows
        set_mem_b32s1(cur_list_tail(), pp);
        if pp != TEX_NULL { set_cur_list_tail(qq); }
        // Insert post-display penalty and below-display skip
        set_mem_b32s1(cur_list_tail(), crate::mem::new_penalty(eqtb_b32s1(INT_BASE + INT_PAR_POST_DISPLAY_PENALTY)));
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        set_mem_b32s1(cur_list_tail(), crate::mem::new_param_glue(GLUE_PAR_BELOW_DISPLAY));
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        // Only prevdepth (aux.b32.s1) is restored for MMODE (C: cur_list.aux.b32.s1 = aux_save.b32.s1)
        set_cur_list_aux_b32s1(aux_save_s1);
        resume_after_display();
    } else {
        // Regular vmode or hmode alignment — restore full aux
        set_cur_list_aux_b32s0(aux_save_s0);
        set_cur_list_aux_b32s1(aux_save_s1);
        set_mem_b32s1(cur_list_tail(), pp);
        if pp != TEX_NULL { set_cur_list_tail(qq); }
        if cur_list_mode() == VMODE {
            build_page();
        }
    }
}

// ---------------------------------------------------------------------------
// show_save_groups
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:7752 (show_save_groups)
#[no_mangle]
pub unsafe extern "C" fn show_save_groups() {
    use crate::globals::{
        cur_boundary, cur_group, cur_level, mem_b16s1, nest_e_tex_aux, nest_mode, nest_ptr,
        save_ptr, save_stack_b16s0, save_stack_b32s1, set_cur_group, set_cur_level, set_save_ptr,
    };

    const BOTTOM_LEVEL: u8 = 0;
    const SIMPLE_GROUP: u8 = 1;
    const HBOX_GROUP: u8 = 2;
    const ADJUSTED_HBOX_GROUP: u8 = 3;
    const VBOX_GROUP: u8 = 4;
    const VTOP_GROUP: u8 = 5;
    const ALIGN_GROUP: u8 = 6;
    const NO_ALIGN_GROUP: u8 = 7;
    const OUTPUT_GROUP: u8 = 8;
    const MATH_GROUP: u8 = 9;
    const DISC_GROUP: u8 = 10;
    const INSERT_GROUP: u8 = 11;
    const VCENTER_GROUP: u8 = 12;
    const MATH_CHOICE_GROUP: u8 = 13;
    const SEMI_SIMPLE_GROUP: u8 = 14;
    const MATH_SHIFT_GROUP: u8 = 15;
    const MATH_LEFT_GROUP: u8 = 16;
    const EXACTLY: i32 = 0;                    // xetex-constants.h:256
    const BOX_FLAG: i32 = 0x4000_0000;         // xetex-constants.h:372
    const GLOBAL_BOX_FLAG: i32 = 0x4000_8000;  // xetex-constants.h:373
    const SHIP_OUT_FLAG: i32 = 0x4001_0000;    // xetex-constants.h:374
    const LEADER_FLAG: i32 = 0x4001_0001;      // xetex-constants.h:375
    const A_LEADERS: i32 = 100;                // xetex_format.h:422
    const HMOVE: u16 = 21;                     // xetex_format.h:259
    const VMOVE: u16 = 22;                     // xetex_format.h:260
    const LEADER_SHIP: u16 = 31;               // xetex_format.h:269
    const EQ_NO: u16 = 48;                     // xetex_format.h:286
    const LEFT_NOAD: u16 = 30;                 // xetex_format.h:415
    const VMODE: i16 = 1;
    const HMODE: i16 = 104;
    const MMODE: i16 = 207;

    // Save and override state for walking the group stack
    let mut p = nest_ptr();
    crate::globals::save_cur_list_to_nest(p);
    let v = save_ptr();
    let l = cur_level();
    let c = cur_group();
    set_save_ptr(cur_boundary());
    set_cur_level(cur_level() - 1);
    let mut a: i8 = 1;

    crate::output::print_nl_cstr(c"".as_ptr());
    crate::output::print_ln();

    'outer: loop {
        crate::output::print_nl_cstr(c"### ".as_ptr());
        crate::output::print_group(true);

        if cur_group() == BOTTOM_LEVEL {
            break 'outer;
        }

        // C do-while: advance p until we find a non-HMODE nest entry.
        // After the loop, m = the mode that stopped it, p = one below that entry.
        let m: i16;
        loop {
            let cur_m = nest_mode(p);
            if p > 0 { p -= 1; } else {
                m = VMODE;
                break;
            }
            if cur_m != HMODE {
                m = cur_m;
                break;
            }
        }

        crate::output::print_cstr(c" (".as_ptr());

        // Determine which continuation block to execute
        // After::Preamble = execute the i-box preamble block, then found1→found2→found
        // After::Found1   = skip preamble, execute found1→found2→found
        // After::Found2   = skip preamble+found1, execute found2→found
        // After::Found    = skip to found only
        enum After { Preamble, Found1, Found2, Found }
        let mut s: *const libc::c_char = std::ptr::null();

        let after = match cur_group() {
            SIMPLE_GROUP => { p += 1; After::Found2 }
            HBOX_GROUP | ADJUSTED_HBOX_GROUP => { s = c"hbox".as_ptr(); After::Preamble }
            VBOX_GROUP  => { s = c"vbox".as_ptr();  After::Preamble }
            VTOP_GROUP  => { s = c"vtop".as_ptr();  After::Preamble }
            ALIGN_GROUP => {
                if a == 0 {
                    s = if m == -VMODE { c"halign".as_ptr() } else { c"valign".as_ptr() };
                    a = 1;
                    After::Found1
                } else {
                    if a == 1 {
                        crate::output::print_cstr(c"align entry".as_ptr());
                    } else {
                        crate::output::print_esc_cstr(c"cr".as_ptr());
                    }
                    if p >= a as i32 { p -= a as i32; }
                    a = 0;
                    After::Found
                }
            }
            NO_ALIGN_GROUP => {
                p += 1;
                a = -1;
                crate::output::print_esc_cstr(c"noalign".as_ptr());
                After::Found2
            }
            OUTPUT_GROUP => {
                crate::output::print_esc_cstr(c"output".as_ptr());
                After::Found
            }
            MATH_GROUP => After::Found2,
            DISC_GROUP | MATH_CHOICE_GROUP => {
                if cur_group() == DISC_GROUP {
                    crate::output::print_esc_cstr(c"discretionary".as_ptr());
                } else {
                    crate::output::print_esc_cstr(c"mathchoice".as_ptr());
                }
                let limit = save_stack_b32s1(save_ptr() - 2);
                for i in 1..=3_i32 {
                    if i <= limit { crate::output::print_cstr(c"{}".as_ptr()); }
                }
                After::Found2
            }
            INSERT_GROUP => {
                let val = save_stack_b32s1(save_ptr() - 2);
                if val == 255 {
                    crate::output::print_esc_cstr(c"vadjust".as_ptr());
                } else {
                    crate::output::print_esc_cstr(c"insert".as_ptr());
                    crate::output::print_int(val);
                }
                After::Found2
            }
            VCENTER_GROUP => { s = c"vcenter".as_ptr(); After::Found1 }
            SEMI_SIMPLE_GROUP => {
                p += 1;
                crate::output::print_esc_cstr(c"begingroup".as_ptr());
                After::Found
            }
            MATH_SHIFT_GROUP => {
                // C: if m==MMODE print '$' (fall through to another '$' = "$$")
                //    else if nest[p].mode==MMODE: print EQ_NO, goto found
                //    else: (fall through to '$' + goto found)
                if m == MMODE {
                    crate::output::print_char('$' as i32);
                } else if nest_mode(p) == MMODE {
                    print_cmd_chr(EQ_NO, save_stack_b32s1(save_ptr() - 2));
                    // goto found (no '$')
                    crate::output::print_char(')' as i32);
                    set_cur_level(cur_level() - 1);
                    set_cur_group(save_stack_b16s0(save_ptr()) as u8);
                    set_save_ptr(save_stack_b32s1(save_ptr()));
                    continue 'outer;
                }
                crate::output::print_char('$' as i32);
                After::Found
            }
            MATH_LEFT_GROUP => {
                if mem_b16s1(nest_e_tex_aux(p + 1)) == LEFT_NOAD {
                    crate::output::print_esc_cstr(c"left".as_ptr());
                } else {
                    crate::output::print_esc_cstr(c"middle".as_ptr());
                }
                After::Found
            }
            _ => After::Found,
        };

        // found1's preamble: print box context for hbox/vbox/vtop (move/setbox/leaders)
        if matches!(after, After::Preamble) {
            let i = save_stack_b32s1(save_ptr() - 4);
            if i != 0 {
                if i < BOX_FLAG {
                    let j = if nest_mode(p).abs() == VMODE { HMOVE } else { VMOVE };
                    if i > 0 { print_cmd_chr(j, 0); } else { print_cmd_chr(j, 1); }
                    crate::output::print_scaled(i.abs());
                    crate::output::print_cstr(c"pt".as_ptr());
                } else if i < SHIP_OUT_FLAG {
                    let mut ii = i;
                    if ii >= GLOBAL_BOX_FLAG {
                        crate::output::print_esc_cstr(c"global".as_ptr());
                        ii -= GLOBAL_BOX_FLAG - BOX_FLAG;
                    }
                    crate::output::print_esc_cstr(c"setbox".as_ptr());
                    crate::output::print_int(ii - BOX_FLAG);
                    crate::output::print_char('=' as i32);
                } else {
                    print_cmd_chr(LEADER_SHIP, i - (LEADER_FLAG - A_LEADERS));
                }
            }
        }

        // found1: print \s and optional size spec
        if matches!(after, After::Preamble | After::Found1) {
            crate::output::print_esc_cstr(s);
            let spec = save_stack_b32s1(save_ptr() - 2);
            if spec != 0 {
                crate::output::print_char(' ' as i32);
                // C: save_stack[save_ptr - 3].b32.s1 == EXACTLY
                if save_stack_b32s1(save_ptr() - 3) == EXACTLY {
                    crate::output::print_cstr(c"to".as_ptr());
                } else {
                    crate::output::print_cstr(c"spread".as_ptr());
                }
                crate::output::print_scaled(spec);
                crate::output::print_cstr(c"pt".as_ptr());
            }
        }

        // found2: print '{'
        if !matches!(after, After::Found) {
            crate::output::print_char('{' as i32);
        }

        // found: print ')', advance group-boundary chain
        crate::output::print_char(')' as i32);
        set_cur_level(cur_level() - 1);
        set_cur_group(save_stack_b16s0(save_ptr()) as u8);
        set_save_ptr(save_stack_b32s1(save_ptr()));
    }

    // done: restore state
    set_save_ptr(v);
    set_cur_level(l);
    set_cur_group(c);
}

// ---------------------------------------------------------------------------
// align_peek
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:8785 (align_peek)
#[no_mangle]
pub unsafe extern "C" fn align_peek() {
    const NO_ALIGN: u8 = 34;           // xetex_format.h:272
    const NO_ALIGN_GROUP: u8 = 7;      // xetex-constants.h:159
    const CAR_RET: u8 = 5;             // xetex_format.h:235
    const CR_CR_CODE: i32 = 1_114_115; // xetex-constants.h:681

    loop {
        crate::globals::set_align_state(1_000_000);
        loop {
            get_x_or_protected();
            if cur_cmd() != SPACER { break; }
        }
        if cur_cmd() == NO_ALIGN {
            scan_left_brace();
            crate::save::new_save_level(NO_ALIGN_GROUP);
            if crate::globals::cur_list_mode() == -1 {
                normal_paragraph();
            }
            return;
        } else if cur_cmd() == RIGHT_BRACE {
            fin_align();
            return;
        } else if cur_cmd() == CAR_RET && cur_chr() == CR_CR_CODE {
            continue;
        } else {
            init_row();
            init_col();
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// show_whatever
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:10561 (show_whatever)
#[no_mangle]
pub unsafe extern "C" fn show_whatever() {
    const BOX_BASE: i32 = 2_255_144;          // xetex_format.h:211
    const INT_PAR_TRACING_ONLINE: i32 = 29;   // xetex_format.h:33
    const SELECTOR_TERM_AND_LOG: i32 = 19;    // xetex-core.h:48
    const IF_TEST: u16 = 107;                 // xetex_format.h:349
    const FI_CODE: u8 = 2;                    // xetex_format.h:537

    crate::output::diagnostic_begin_capture_warning_here();

    let goto_common_ending = match cur_chr() {
        3 => {
            crate::output::begin_diagnostic();
            show_activities();
            false
        }
        1 => {
            scan_register_num();
            let p = if cur_val() < 256 {
                eqtb_b32s1(BOX_BASE + cur_val())
            } else {
                crate::save::find_sa_element(4, cur_val(), false);
                if crate::globals::cur_ptr() == crate::mem::TEX_NULL {
                    crate::mem::TEX_NULL
                } else {
                    mem_b32s1(crate::globals::cur_ptr() + 1)
                }
            };
            crate::output::begin_diagnostic();
            crate::output::print_nl_cstr(c"> \\box".as_ptr());
            crate::output::print_int(cur_val());
            crate::output::print_char('=' as i32);
            if p == crate::mem::TEX_NULL {
                crate::output::print_cstr(c"void".as_ptr());
            } else {
                show_box(p);
            }
            false
        }
        0 => {
            get_token();
            crate::output::print_nl_cstr(c"> ".as_ptr());
            if cur_cs() != 0 {
                crate::output::sprint_cs(cur_cs());
                crate::output::print_char('=' as i32);
            }
            crate::output::print_meaning();
            true
        }
        4 => {
            crate::output::begin_diagnostic();
            show_save_groups();
            false
        }
        6 => {
            crate::output::begin_diagnostic();
            crate::output::print_nl_cstr(c"".as_ptr());
            crate::output::print_ln();
            if crate::globals::cond_ptr() == crate::mem::TEX_NULL {
                crate::output::print_nl_cstr(c"### ".as_ptr());
                crate::output::print_cstr(c"no active conditionals".as_ptr());
            } else {
                let mut p = crate::globals::cond_ptr();
                let mut n: i32 = 0;
                loop {
                    n += 1;
                    p = mem_b32s1(p);
                    if p == crate::mem::TEX_NULL { break; }
                }
                p = crate::globals::cond_ptr();
                let mut t = crate::globals::cur_if();
                let mut l = crate::globals::if_line();
                let mut m = crate::globals::if_limit();
                loop {
                    crate::output::print_nl_cstr(c"### level ".as_ptr());
                    crate::output::print_int(n);
                    crate::output::print_cstr(c": ".as_ptr());
                    print_cmd_chr(IF_TEST, t as i32);
                    if m == FI_CODE {
                        crate::output::print_esc_cstr(c"else".as_ptr());
                    }
                    if l != 0 {
                        crate::output::print_cstr(c" entered on line ".as_ptr());
                        crate::output::print_int(l);
                    }
                    n -= 1;
                    t = mem_b16s0(p) as i16;
                    l = mem_b32s1(p + 1);
                    m = mem_b16s1(p) as u8;
                    p = mem_b32s1(p);
                    if p == crate::mem::TEX_NULL { break; }
                }
            }
            false
        }
        _ => {
            the_toks(); // the_toks populates TEMP_HEAD
            crate::output::print_nl_cstr(c"> ".as_ptr());
            crate::output::token_show(TEMP_HEAD);
            crate::mem::flush_list(mem_b32s1(TEMP_HEAD));
            true
        }
    };

    if !goto_common_ending {
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::end_diagnostic(true);
        if crate::globals::file_line_error_style_p() != 0 {
            crate::output::print_file_line();
        } else {
            crate::output::print_nl_cstr(c"! ".as_ptr());
        }
        crate::output::print_cstr(c"OK".as_ptr());
        if crate::globals::selector() == SELECTOR_TERM_AND_LOG {
            if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) <= 0 {
                crate::globals::set_selector(SELECTOR_TERM_ONLY);
                crate::output::print_cstr(c" (see the transcript file)".as_ptr());
                crate::globals::set_selector(SELECTOR_TERM_AND_LOG);
            }
        }
    }

    // common_ending:
    capture_to_diagnostic(std::ptr::null_mut());
    if crate::globals::interaction() < ERROR_STOP_MODE {
        set_help_ptr(0);
        crate::globals::set_error_count(crate::globals::error_count() - 1);
    } else if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) > 0 {
        set_help_ptr(3);
        set_help_line(2, c"This isn't an error message; I'm just \\showing something.".as_ptr());
        set_help_line(1, c"Type `I\\show...' to show more (e.g., \\show\\cs,".as_ptr());
        set_help_line(0, c"\\showthe\\count10, \\showbox255, \\showlists).".as_ptr());
    } else {
        set_help_ptr(5);
        set_help_line(4, c"This isn't an error message; I'm just \\showing something.".as_ptr());
        set_help_line(3, c"Type `I\\show...' to show more (e.g., \\show\\cs,".as_ptr());
        set_help_line(2, c"\\showthe\\count10, \\showbox255, \\showlists).".as_ptr());
        set_help_line(1, c"And type `I\\tracingonline=1\\show...' to show boxes and".as_ptr());
        set_help_line(0, c"lists on your terminal as well as in the transcript file.".as_ptr());
    }
    error();
}

// ---------------------------------------------------------------------------
// append_italic_correction
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:9868 (append_italic_correction)
#[no_mangle]
pub unsafe extern "C" fn append_italic_correction() {
    extern "C" {
        // Note: in C these are macros: get_native_italic_correction(p) = real_get_native_italic_correction(&mem[p])
        fn real_get_native_italic_correction(p: *mut libc::c_void) -> i32;
        fn real_get_native_glyph_italic_correction(p: *mut libc::c_void) -> i32;
    }
    const LIGATURE_NODE: u16 = 6;        // xetex_format.h:386
    const EXPLICIT: u16 = 1;            // xetex_format.h:428
    const GLYPH_NODE: u16 = 42;         // xetex_format.h:465

    let tail = crate::globals::cur_list_tail();
    if tail == crate::globals::cur_list_head() { return; }

    let p = if tail >= crate::globals::hi_mem_min() {
        // is_char_node
        tail
    } else if mem_b16s1(tail) == LIGATURE_NODE {
        tail + 1
    } else if mem_b16s1(tail) == WHATSIT_NODE {
        let sub = mem_b16s0(tail);
        if sub == NATIVE_WORD_NODE || sub == NATIVE_WORD_NODE_AT {
            let kern = crate::mem::new_kern(real_get_native_italic_correction(crate::globals::mem_node_addr(tail)));
            set_mem_b32s1(tail, kern);
            let new_tail = mem_b32s1(tail);
            crate::globals::set_cur_list_tail(new_tail);
            set_mem_b16s0(new_tail, EXPLICIT);
        } else if sub == GLYPH_NODE {
            let kern = crate::mem::new_kern(real_get_native_glyph_italic_correction(crate::globals::mem_node_addr(tail)));
            set_mem_b32s1(tail, kern);
            let new_tail = mem_b32s1(tail);
            crate::globals::set_cur_list_tail(new_tail);
            set_mem_b16s0(new_tail, EXPLICIT);
        }
        return;
    } else {
        return;
    };

    let f = mem_b16s1(p) as i32; // CHAR_NODE_font(p)
    let c = effective_char(true, f, mem_b16s0(p));
    let info = crate::globals::font_info_b16(crate::globals::char_base_entry(f) + c);
    let italic = crate::globals::font_info_b32s1(crate::globals::italic_base_entry(f) + (info.s1 / 4) as i32);
    let kern = crate::mem::new_kern(italic);
    let tail2 = crate::globals::cur_list_tail();
    set_mem_b32s1(tail2, kern);
    let new_tail = mem_b32s1(tail2);
    crate::globals::set_cur_list_tail(new_tail);
    set_mem_b16s0(new_tail, EXPLICIT);
}

// ---------------------------------------------------------------------------
// make_accent
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:9910 (make_accent)
#[no_mangle]
pub unsafe extern "C" fn make_accent() {
    extern "C" {
        fn new_character(f: i32, c: i32) -> i32;
        fn get_native_char_sidebearings(f: i32, c: i32, lsb: *mut i32, rsb: *mut i32);
        fn get_native_char_height_depth(f: i32, c: i32, h: *mut i32, depth: *mut i32);
    }
    const SLANT_CODE: i32 = 1;      // xetex-constants.h:273
    const X_HEIGHT_CODE: i32 = 5;   // xetex-constants.h:294
    const ACC_KERN: u16 = 2;        // xetex_format.h:429
    const ADDITIONAL: i16 = 1;      // xetex-constants.h:264
    const LETTER: u8 = 11;          // xetex_format.h:243
    const CHAR_NUM: u8 = 16;        // xetex_format.h:253

    let is_native = |fnt: i32| -> bool {
        crate::globals::font_area_entry(fnt) == AAT_FONT_FLAG
            || crate::globals::font_area_entry(fnt) == OTGR_FONT_FLAG
    };

    scan_char_num();
    let mut f = eqtb_b32s1(CUR_FONT_LOC);
    let p = new_character(f, cur_val());

    if p == crate::mem::TEX_NULL { return; }

    let x = crate::globals::font_info_b32s1(X_HEIGHT_CODE + crate::globals::param_base(f));
    let s = crate::globals::font_info_b32s1(SLANT_CODE + crate::globals::param_base(f)) as f64 / 65536.0;
    let mut lsb: i32 = 0;
    let mut rsb: i32 = 0;
    let a = if is_native(f) {
        let av = mem_b32s1(p + 1);
        if av == 0 {
            get_native_char_sidebearings(f, cur_val(), &mut lsb, &mut rsb);
        }
        av
    } else {
        let c = effective_char(true, f, mem_b16s0(p));
        let info = crate::globals::font_info_b16(crate::globals::char_base_entry(f) + c);
        crate::globals::font_info_b32s1(crate::globals::width_base_entry(f) + info.s3 as i32)
    };

    do_assignments();
    let mut q = crate::mem::TEX_NULL;
    f = eqtb_b32s1(CUR_FONT_LOC);
    if cur_cmd() == LETTER || cur_cmd() == OTHER_CHAR || cur_cmd() == CHAR_GIVEN {
        q = new_character(f, cur_chr());
        set_cur_val(cur_chr());
    } else if cur_cmd() == CHAR_NUM {
        scan_char_num();
        q = new_character(f, cur_val());
    } else {
        crate::input::back_input();
    }

    if q != crate::mem::TEX_NULL {
        let t = crate::globals::font_info_b32s1(SLANT_CODE + crate::globals::param_base(f)) as f64 / 65536.0;
        let (w, h) = if is_native(f) {
            let wv = mem_b32s1(q + 1);
            let mut hv: i32 = 0;
            let mut dv: i32 = 0;
            get_native_char_height_depth(f, cur_val(), &mut hv, &mut dv);
            (wv, hv)
        } else {
            let c = effective_char(true, f, mem_b16s0(q));
            let info = crate::globals::font_info_b16(crate::globals::char_base_entry(f) + c);
            let wv = crate::globals::font_info_b32s1(crate::globals::width_base_entry(f) + info.s3 as i32);
            let hv = crate::globals::font_info_b32s1(crate::globals::height_base_entry(f) + (info.s2 / 16) as i32);
            (wv, hv)
        };
        let mut pa = p;
        if h != x {
            pa = hpack(p, 0, ADDITIONAL);
            set_mem_b32s1(pa + 4, x - h);
        }
        let delta = if is_native(f) && a == 0 {
            crate::scaled_math::tex_round(
                (w - lsb + rsb) as f64 / 2.0 + h as f64 * t - x as f64 * s,
            )
        } else {
            crate::scaled_math::tex_round(
                (w - a) as f64 / 2.0 + h as f64 * t - x as f64 * s,
            )
        };
        let r = crate::mem::new_kern(delta);
        set_mem_b16s0(r, ACC_KERN);
        set_mem_b32s1(crate::globals::cur_list_tail(), r);
        set_mem_b32s1(r, pa);
        let new_tail = crate::mem::new_kern(-a - delta);
        crate::globals::set_cur_list_tail(new_tail);
        set_mem_b16s0(new_tail, ACC_KERN);
        set_mem_b32s1(pa, new_tail);
        // p = q (in C); we use q directly below
    }

    // Append p (q if base was found, else accent) to hlist.
    let p_final = if q != crate::mem::TEX_NULL { q } else { p };
    set_mem_b32s1(crate::globals::cur_list_tail(), p_final);
    crate::globals::set_cur_list_tail(p_final);
    crate::globals::set_cur_list_aux_b32s0(1000);
}

// ---------------------------------------------------------------------------
// new_font
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:10233 (new_font)
#[no_mangle]
pub unsafe extern "C" fn new_font(a: i16) {
    extern "C" {
        fn read_font_info(u: i32, nom: i32, aire: i32, s: i32) -> i32;
    }
    const TOO_BIG_CHAR: i32 = 65536;       // xetex-constants.h

    if crate::globals::job_name() == 0 {
        open_log_file();
    }

    get_r_token();
    let u = cur_cs();
    let t = if u >= HASH_BASE {
        hash_s1(u)
    } else if u >= SINGLE_BASE {
        if u == NULL_CS {
            crate::texmfmp::maketexstring(c"FONT".as_ptr())
        } else {
            u - SINGLE_BASE
        }
    } else {
        let old_setting = crate::globals::selector();
        crate::globals::set_selector(SELECTOR_NEW_STRING);
        crate::output::print_cstr(c"FONT".as_ptr());
        crate::output::print(u - 1);
        crate::globals::set_selector(old_setting);
        if pool_ptr() + 1 > crate::globals::pool_size() {
            crate::errors::overflow(c"pool size".as_ptr(),
                crate::globals::pool_size() - crate::globals::init_pool_ptr());
        }
        crate::string_pool::make_string()
    };

    if a >= 4 {
        crate::save::geq_define(u, SET_FONT as u16, FONT_BASE);
    } else {
        crate::save::eq_define(u, SET_FONT as u16, FONT_BASE);
    }
    scan_optional_equals();
    scan_file_name();
    crate::globals::set_name_in_progress(true);

    let s = if scan_keyword(c"at".as_ptr()) {
        scan_dimen(false, false, false);
        let sv = cur_val();
        if sv <= 0 || sv >= 0x800_0000 {
            error_here_with_diagnostic(c"Improper `at' size (".as_ptr());
            crate::output::print_scaled(sv);
            crate::output::print_cstr(c"pt), replaced by 10pt".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(2);
            set_help_line(1, c"I can only handle fonts at positive sizes that are".as_ptr());
            set_help_line(0, c"less than 2048pt, so I've changed what you said to 10pt.".as_ptr());
            error();
            10 * 65536_i32
        } else {
            sv
        }
    } else if scan_keyword(c"scaled".as_ptr()) {
        scan_int();
        let sv = cur_val();
        if sv <= 0 || sv > 32768 {
            let errmsg = error_here_with_diagnostic(
                c"Illegal magnification has been changed to 1000".as_ptr()
            );
            ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), sv);
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(1);
            set_help_line(0, c"The magnification ratio must be between 1 and 32768.".as_ptr());
            int_error(sv);
            -1000_i32
        } else {
            -sv
        }
    } else {
        -1000_i32
    };

    crate::globals::set_name_in_progress(false);

    let f = 'search: {
        let mut fi = FONT_BASE + 1;
        while fi <= crate::globals::font_ptr() {
            if crate::string_pool::str_eq_str(crate::globals::font_name_entry(fi), crate::globals::cur_name())
                && (
                    (crate::string_pool::length(crate::globals::cur_area()) == 0
                        && (crate::globals::font_area_entry(fi) == AAT_FONT_FLAG
                            || crate::globals::font_area_entry(fi) == OTGR_FONT_FLAG))
                    || crate::string_pool::str_eq_str(crate::globals::font_area_entry(fi), crate::globals::cur_area())
                )
            {
                if s > 0 {
                    if s == crate::globals::font_size_entry(fi) {
                        break 'search fi;
                    }
                } else if crate::globals::font_size_entry(fi)
                    == crate::scaled_math::xn_over_d(crate::globals::font_dsize_entry(fi), -s, 1000)
                {
                    break 'search fi;
                }
            }
            crate::string_pool::append_str(crate::globals::cur_area());
            crate::string_pool::append_str(crate::globals::cur_name());
            crate::string_pool::append_str(crate::globals::cur_ext());
            let ms = crate::string_pool::make_string();
            if crate::string_pool::str_eq_str(crate::globals::font_name_entry(fi), ms) {
                set_str_ptr(str_ptr() - 1);
                set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
                if crate::globals::font_area_entry(fi) == AAT_FONT_FLAG
                    || crate::globals::font_area_entry(fi) == OTGR_FONT_FLAG
                {
                    if s > 0 {
                        if s == crate::globals::font_size_entry(fi) {
                            break 'search fi;
                        }
                    } else if crate::globals::font_size_entry(fi)
                        == crate::scaled_math::xn_over_d(crate::globals::font_dsize_entry(fi), -s, 1000)
                    {
                        break 'search fi;
                    }
                }
            } else {
                set_str_ptr(str_ptr() - 1);
                set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
            }
            fi += 1;
        }
        read_font_info(u, crate::globals::cur_name(), crate::globals::cur_area(), s)
    };

    // common_ending:
    if a >= 4 {
        crate::save::geq_define(u, SET_FONT as u16, f);
    } else {
        crate::save::eq_define(u, SET_FONT as u16, f);
    }
    crate::globals::set_eqtb_word(FONT_ID_BASE + f, crate::globals::eqtb_word(u));
    crate::globals::set_hash_s1(FONT_ID_BASE + f, t);
}

// ---------------------------------------------------------------------------
// do_register_command
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:10220 (do_register_command)
#[no_mangle]
pub unsafe extern "C" fn do_register_command(a: i16) {
    const ADVANCE: u8 = 92;         // xetex_format.h:332
    const MULTIPLY: u8 = 93;        // xetex_format.h:334
    const GLUE_REF: u16 = 119;      // xetex_format.h:361
    const NORMAL: u16 = 0;          // xetex_format.h:420

    let q = cur_cmd();
    let mut w: i32 = 0;
    let mut s: i32 = crate::mem::TEX_NULL;

    // Determine (l, p, e): register address, value type, is_extended_SA
    let (l, p, e) = 'determine: {
        if q != REGISTER {
            get_x_token();
            if cur_cmd() >= ASSIGN_INT && cur_cmd() <= ASSIGN_MU_GLUE {
                break 'determine (cur_chr(), (cur_cmd() - ASSIGN_INT) as i16, false);
            }
            if cur_cmd() != REGISTER {
                error_here_with_diagnostic(c"You can't use `".as_ptr());
                print_cmd_chr(cur_cmd() as u16, cur_chr());
                crate::output::print_cstr(c"' after ".as_ptr());
                print_cmd_chr(q as u16, 0);
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(1);
                set_help_line(0, c"I'm forgetting what you said and not changing anything.".as_ptr());
                error();
                return;
            }
        }
        if cur_chr() < 0 || cur_chr() > 19 {
            let l = cur_chr();
            let p = (mem_b16s1(l) / 64) as i16;
            (l, p, true)
        } else {
            let p = cur_chr() as i16;
            scan_register_num();
            if cur_val() > 255 {
                crate::save::find_sa_element(p as u16, cur_val(), true);
                (crate::globals::cur_ptr(), p, true)
            } else {
                let l = match p {
                    INT_VAL   => cur_val() + COUNT_BASE,
                    DIMEN_VAL => cur_val() + SCALED_BASE,
                    GLUE_VAL  => cur_val() + SKIP_BASE,
                    MU_VAL    => cur_val() + MU_SKIP_BASE,
                    _ => unreachable!(),
                };
                (l, p, false)
            }
        }
    };

    // found: read current value into w (int/dimen) or s (glue/mu)
    if p < GLUE_VAL {
        w = if e { mem_b32s1(l + 2) } else { eqtb_b32s1(l) };
    } else if e {
        s = mem_b32s1(l + 1);
    } else {
        s = eqtb_b32s1(l);
    }

    if q == REGISTER {
        scan_optional_equals();
    } else {
        scan_keyword(c"by".as_ptr());
    }

    crate::globals::set_arith_error(false);

    if q < MULTIPLY {
        if p < GLUE_VAL {
            if p == INT_VAL {
                scan_int();
            } else {
                scan_dimen(false, false, false);
            }
            if q == ADVANCE {
                set_cur_val(cur_val() + w);
            }
        } else {
            scan_glue(p);
            if q == ADVANCE {
                let qs = crate::mem::new_spec(cur_val());
                let r = s;
                crate::mem::delete_glue_ref(cur_val());
                set_mem_b32s1(qs + 1, mem_b32s1(qs + 1) + mem_b32s1(r + 1));
                if mem_b32s1(qs + 2) == 0 {
                    set_mem_b16s1(qs, NORMAL);
                }
                if mem_b16s1(qs) == mem_b16s1(r) {
                    set_mem_b32s1(qs + 2, mem_b32s1(qs + 2) + mem_b32s1(r + 2));
                } else if mem_b16s1(qs) < mem_b16s1(r) && mem_b32s1(r + 2) != 0 {
                    set_mem_b32s1(qs + 2, mem_b32s1(r + 2));
                    set_mem_b16s1(qs, mem_b16s1(r));
                }
                if mem_b32s1(qs + 3) == 0 {
                    set_mem_b16s0(qs, NORMAL);
                }
                if mem_b16s0(qs) == mem_b16s0(r) {
                    set_mem_b32s1(qs + 3, mem_b32s1(qs + 3) + mem_b32s1(r + 3));
                } else if mem_b16s0(qs) < mem_b16s0(r) && mem_b32s1(r + 3) != 0 {
                    set_mem_b32s1(qs + 3, mem_b32s1(r + 3));
                    set_mem_b16s0(qs, mem_b16s0(r));
                }
                set_cur_val(qs);
            }
        }
    } else {
        scan_int();
        if p < GLUE_VAL {
            if q == MULTIPLY {
                if p == INT_VAL {
                    set_cur_val(crate::scaled_math::mult_and_add(w, cur_val(), 0, TEX_INFINITY));
                } else {
                    set_cur_val(crate::scaled_math::mult_and_add(w, cur_val(), 0, MAX_HALFWORD));
                }
            } else {
                set_cur_val(crate::scaled_math::x_over_n(w, cur_val()));
            }
        } else {
            let r = crate::mem::new_spec(s);
            if q == MULTIPLY {
                set_mem_b32s1(r + 1, crate::scaled_math::mult_and_add(mem_b32s1(s + 1), cur_val(), 0, MAX_HALFWORD));
                set_mem_b32s1(r + 2, crate::scaled_math::mult_and_add(mem_b32s1(s + 2), cur_val(), 0, MAX_HALFWORD));
                set_mem_b32s1(r + 3, crate::scaled_math::mult_and_add(mem_b32s1(s + 3), cur_val(), 0, MAX_HALFWORD));
            } else {
                set_mem_b32s1(r + 1, crate::scaled_math::x_over_n(mem_b32s1(s + 1), cur_val()));
                set_mem_b32s1(r + 2, crate::scaled_math::x_over_n(mem_b32s1(s + 2), cur_val()));
                set_mem_b32s1(r + 3, crate::scaled_math::x_over_n(mem_b32s1(s + 3), cur_val()));
            }
            set_cur_val(r);
        }
    }

    if crate::globals::arith_error() {
        error_here_with_diagnostic(c"Arithmetic overflow".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, c"I can't carry out that multiplication or division,".as_ptr());
        set_help_line(0, c"since the result is out of range.".as_ptr());
        if p >= GLUE_VAL {
            crate::mem::delete_glue_ref(cur_val());
        }
        error();
        return;
    }

    if p < GLUE_VAL {
        if e {
            if a >= 4 {
                crate::save::gsa_w_def(l, cur_val());
            } else {
                crate::save::sa_w_def(l, cur_val());
            }
        } else if a >= 4 {
            crate::save::geq_word_define(l, cur_val());
        } else {
            crate::save::eq_word_define(l, cur_val());
        }
    } else {
        trap_zero_glue();
        if e {
            if a >= 4 {
                crate::save::gsa_def(l, cur_val());
            } else {
                crate::save::sa_def(l, cur_val());
            }
        } else if a >= 4 {
            crate::save::geq_define(l, GLUE_REF, cur_val());
        } else {
            crate::save::eq_define(l, GLUE_REF, cur_val());
        }
    }
}

// ---------------------------------------------------------------------------
// unpackage
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:9799 (unpackage)
#[no_mangle]
pub unsafe extern "C" fn unpackage() {
    const COPY_CODE: i32 = 1;           // xetex_format.h:481
    const HLIST_NODE: u16 = 0;          // xetex_format.h:380
    const VLIST_NODE: u16 = 1;          // xetex_format.h:381
    const MARGIN_KERN_NODE: u16 = 40;   // xetex_format.h:396
    const MARGIN_KERN_NODE_SIZE: i32 = 3; // xetex-constants.h:115
    const BOX_NODE_SIZE: i32 = 8;       // xetex-constants.h:132
    const HMODE: i16 = 104;             // xetex_format.h:369
    const MMODE: i16 = 207;             // xetex_format.h:370

    if cur_chr() > COPY_CODE {
        set_mem_b32s1(crate::globals::cur_list_tail(), crate::globals::disc_ptr(cur_chr() as usize));
        crate::globals::set_disc_ptr(cur_chr() as usize, crate::mem::TEX_NULL);
        // goto done — jump to tail-walk loop
    } else {
        let c = cur_chr();
        scan_register_num();
        let p = if cur_val() < 256 {
            eqtb_b32s1(BOX_BASE + cur_val())
        } else {
            crate::save::find_sa_element(4, cur_val(), false);
            if crate::globals::cur_ptr() == crate::mem::TEX_NULL {
                crate::mem::TEX_NULL
            } else {
                mem_b32s1(crate::globals::cur_ptr() + 1)
            }
        };

        if p == crate::mem::TEX_NULL { return; }

        let mode = cur_list_mode().abs();
        let ntype = mem_b16s1(p);
        if mode == MMODE
            || (mode == VMODE && ntype != VLIST_NODE)
            || (mode == HMODE && ntype != HLIST_NODE)
        {
            error_here_with_diagnostic(c"Incompatible list can't be unboxed".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(3);
            set_help_line(2, c"Sorry, Pandora. (You sneaky devil.)".as_ptr());
            set_help_line(1, c"I refuse to unbox an \\hbox in vertical mode or vice versa.".as_ptr());
            set_help_line(0, c"And I can't open any boxes in math mode.".as_ptr());
            error();
            return;
        }

        let list_head = mem_b32s1(p + 5);
        if c == COPY_CODE {
            set_mem_b32s1(crate::globals::cur_list_tail(), copy_node_list(list_head));
        } else {
            set_mem_b32s1(crate::globals::cur_list_tail(), list_head);
            if cur_val() < 256 {
                crate::globals::set_eqtb_b32s1(BOX_BASE + cur_val(), crate::mem::TEX_NULL);
            } else {
                crate::save::find_sa_element(4, cur_val(), false);
                if crate::globals::cur_ptr() != crate::mem::TEX_NULL {
                    set_mem_b32s1(crate::globals::cur_ptr() + 1, crate::mem::TEX_NULL);
                    let rc = mem_b32s0(crate::globals::cur_ptr() + 1) + 1;
                    set_mem_b32s0(crate::globals::cur_ptr() + 1, rc);
                    crate::save::delete_sa_ref(crate::globals::cur_ptr());
                }
            }
            crate::mem::free_node(p, BOX_NODE_SIZE);
        }
    }

    // done: advance cur_list.tail past any MARGIN_KERN_NODEs
    while mem_b32s1(crate::globals::cur_list_tail()) != crate::mem::TEX_NULL {
        let r = mem_b32s1(crate::globals::cur_list_tail());
        if r < crate::globals::hi_mem_min() && mem_b16s1(r) == MARGIN_KERN_NODE {
            set_mem_b32s1(crate::globals::cur_list_tail(), mem_b32s1(r));
            crate::mem::free_node(r, MARGIN_KERN_NODE_SIZE);
        } else {
            crate::globals::set_cur_list_tail(mem_b32s1(crate::globals::cur_list_tail()));
        }
    }
}

// ---------------------------------------------------------------------------
// delete_last
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:9716 (delete_last)
#[no_mangle]
pub unsafe extern "C" fn delete_last() {
    const DISC_NODE: u16 = 7;       // xetex_format.h:387
    const BEGIN_M_CODE: u16 = 2;    // xetex_format.h:436

    if cur_list_mode() == VMODE && crate::globals::cur_list_tail() == crate::globals::cur_list_head() {
        // On page: only allowed for \lastskip when last_glue != MAX_HALFWORD
        if cur_chr() as u16 != GLUE_NODE || crate::globals::last_glue() != MAX_HALFWORD {
            you_cant();
            set_help_ptr(2);
            set_help_line(1, c"Sorry...I usually can't take things from the current page.".as_ptr());
            set_help_line(0, c"Try `I\\vskip-\\lastskip' instead.".as_ptr());
            if cur_chr() as u16 == KERN_NODE {
                set_help_line(0, c"Try `I\\kern-\\lastkern' instead.".as_ptr());
            } else if cur_chr() as u16 != GLUE_NODE {
                set_help_line(0, c"Perhaps you can make the output routine do it.".as_ptr());
            }
            error();
        }
        return;
    }

    let mut tx = crate::globals::cur_list_tail();

    // If tail is MATH END, back up one to find the actual last "deletable" node
    if tx < crate::globals::hi_mem_min()
        && mem_b16s1(tx) == MATH_NODE
        && mem_b16s0(tx) == END_M_CODE
    {
        let mut r = crate::globals::cur_list_head();
        loop {
            let next_r = mem_b32s1(r);
            if next_r == tx { tx = r; break; }
            r = next_r;
        }
    }

    if tx >= crate::globals::hi_mem_min() || mem_b16s1(tx) as i32 != cur_chr() {
        return;
    }

    // Walk the list to find the node before tx
    let mut q = crate::globals::cur_list_head();
    let mut p = crate::mem::TEX_NULL;
    let mut r_save = p;
    let mut fm = false;
    loop {
        r_save = p;
        p = q;
        fm = false;
        if q < crate::globals::hi_mem_min() {
            if mem_b16s1(q) == DISC_NODE {
                let count = mem_b16s0(q);
                for _ in 0..count {
                    p = mem_b32s1(p);
                }
                if p == tx { return; }
            } else if mem_b16s1(q) == MATH_NODE && mem_b16s0(q) == BEGIN_M_CODE {
                fm = true;
            }
        }
        q = mem_b32s1(p);
        if q == tx { break; }
    }

    // q == tx; p is the node just before tx; r_save is the one before p
    let q_next = mem_b32s1(tx);
    set_mem_b32s1(p, q_next);
    set_mem_b32s1(tx, crate::mem::TEX_NULL);
    if q_next == crate::mem::TEX_NULL {
        if fm {
            crate::errors::confusion(c"tail1".as_ptr());
        } else {
            crate::globals::set_cur_list_tail(p);
        }
    } else if fm {
        crate::globals::set_cur_list_tail(r_save);
        set_mem_b32s1(r_save, crate::mem::TEX_NULL);
        flush_node_list(p);
    }
    flush_node_list(tx);
}

// translated from xetex-xetex0.c:6428 (char_warning)
#[no_mangle]
pub unsafe extern "C" fn char_warning(f: i32, c: i32) {
    const INT_PAR_TRACING_LOST_CHARS: i32 = 35; // xetex_format.h:70
    const INT_PAR_TRACING_ONLINE: i32 = 29;     // xetex_format.h:64
    const TOO_BIG_CHAR: i32 = 65536;

    let tracing_lost = eqtb_b32s1(INT_BASE + INT_PAR_TRACING_LOST_CHARS);
    if tracing_lost > 0 {
        let old_setting = eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE);
        if tracing_lost > 1 {
            crate::globals::set_eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE, 1);
        }
        if tracing_lost > 2 {
            if crate::globals::file_line_error_style_p() != 0 {
                crate::output::print_file_line();
            } else {
                crate::output::print_nl_cstr(c"! ".as_ptr());
            }
            crate::output::print_cstr(c"Missing character: There is no ".as_ptr());
        } else {
            crate::output::begin_diagnostic();
            crate::output::diagnostic_begin_capture_warning_here();
            crate::output::print_nl_cstr(c"Missing character: There is no ".as_ptr());
        }
        if c < 65536 {
            crate::output::print(c);
        } else {
            crate::output::print_char(c);
        }
        crate::output::print_cstr(c" (".as_ptr());
        let fa = crate::globals::font_area_entry(f);
        if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
            crate::output::print_ucs_code(c);
        } else {
            crate::output::print_hex(c);
        }
        crate::output::print_char(b')' as i32);
        crate::output::print_cstr(c" in font ".as_ptr());
        crate::output::print(crate::globals::font_name_entry(f));
        if tracing_lost < 3 {
            crate::output::print_char(b'!' as i32);
        }
        crate::globals::set_eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE, old_setting);
        if tracing_lost > 2 {
            crate::globals::set_help_ptr(0);
            error();
        } else {
            capture_to_diagnostic(std::ptr::null_mut());
            crate::output::end_diagnostic(false);
        }
    }

    // Emit a machine-readable warning via ttstub_issue_warning.
    let font_name_c = crate::texmfmp::gettexstring(crate::globals::font_name_entry(f));
    let prev_sel = crate::globals::selector();
    crate::globals::set_selector(SELECTOR_NEW_STRING);
    if c < 0x10000 {
        crate::output::print(c);
    } else {
        crate::output::print_char(c);
    }
    crate::globals::set_selector(prev_sel);
    let s = crate::string_pool::make_string();
    let chr_c = crate::texmfmp::gettexstring(s);
    // flush_string: str_ptr--, pool_ptr = str_start[str_ptr - TOO_BIG_CHAR]
    set_str_ptr(str_ptr() - 1);
    set_pool_ptr(crate::globals::str_start(str_ptr() - TOO_BIG_CHAR));
    ttstub_issue_warning(
        c"could not represent character \"%s\" (0x%x) in font \"%s\"".as_ptr(),
        chr_c,
        c as u32,
        font_name_c,
    );
    libc::free(font_name_c as *mut libc::c_void);
    libc::free(chr_c as *mut libc::c_void);
    if !crate::globals::gave_char_warning_help() {
        ttstub_issue_warning(
            c"  you may need to load the `fontspec` package and use (e.g.) \\setmainfont to".as_ptr(),
        );
        ttstub_issue_warning(
            c"  choose a different font that covers the unrepresentable character(s)".as_ptr(),
        );
        crate::globals::set_gave_char_warning_help(true);
    }
}

// translated from xetex-xetex0.c:9223 (app_space)
#[no_mangle]
pub unsafe extern "C" fn app_space() {
    use crate::globals::{
        font_glue, font_info_b32s1, main_k, main_p, param_base, set_cur_list_tail, set_font_glue,
        set_main_k, set_main_p,
    };
    const GLUE_BASE: i32 = 2_254_340; // xetex_format.h:205
    const GLUE_PAR_SPACE_SKIP: i32 = 12;  // xetex_format.h:161
    const GLUE_PAR_XSPACE_SKIP: i32 = 13; // xetex_format.h:162
    const EXTRA_SPACE_CODE: i32 = 7;      // xetex-constants.h:299

    let space_factor = crate::globals::cur_list_aux_b32s0();
    let cf = eqtb_b32s1(CUR_FONT_LOC);
    let q;
    if space_factor >= 2000 && eqtb_b32s1(GLUE_BASE + GLUE_PAR_XSPACE_SKIP) != 0 {
        q = crate::mem::new_param_glue(GLUE_PAR_XSPACE_SKIP as i16);
    } else {
        if eqtb_b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP) != 0 {
            set_main_p(eqtb_b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP));
        } else {
            set_main_p(font_glue(cf));
            if main_p() == crate::mem::TEX_NULL {
                set_main_p(crate::mem::new_spec(0));
                set_main_k(param_base(cf) + 2);
                // width = space, stretch = space_stretch, shrink = space_shrink
                crate::globals::set_mem_b32s1(main_p() + 1, font_info_b32s1(main_k()));
                crate::globals::set_mem_b32s1(main_p() + 2, font_info_b32s1(main_k() + 1));
                crate::globals::set_mem_b32s1(main_p() + 3, font_info_b32s1(main_k() + 2));
                set_font_glue(cf, main_p());
            }
        }
        set_main_p(crate::mem::new_spec(main_p()));
        if space_factor >= 2000 {
            // add extra_space to width
            let extra = font_info_b32s1(EXTRA_SPACE_CODE + param_base(cf));
            crate::globals::set_mem_b32s1(
                main_p() + 1,
                mem_b32s1(main_p() + 1) + extra,
            );
        }
        // scale stretch by space_factor / 1000
        let new_stretch = crate::scaled_math::xn_over_d(
            mem_b32s1(main_p() + 2), space_factor, 1000,
        );
        crate::globals::set_mem_b32s1(main_p() + 2, new_stretch);
        // scale shrink by 1000 / space_factor
        let new_shrink = crate::scaled_math::xn_over_d(
            mem_b32s1(main_p() + 3), 1000, space_factor,
        );
        crate::globals::set_mem_b32s1(main_p() + 3, new_shrink);
        q = crate::mem::new_glue(main_p());
        crate::globals::set_mem_b32s1(main_p(), crate::mem::TEX_NULL);
    }
    crate::globals::set_mem_b32s1(crate::globals::cur_list_tail(), q);
    set_cur_list_tail(q);
}

// translated from xetex-xetex0.c:6381 (pack_file_name)
#[no_mangle]
pub unsafe extern "C" fn pack_file_name(n: i32, a: i32, e: i32) {
    extern "C" { fn xmalloc(size: libc::size_t) -> *mut libc::c_void; }

    let len_a = crate::string_pool::length(a) as usize;
    let len_n = crate::string_pool::length(n) as usize;
    let len_e = crate::string_pool::length(e) as usize;
    // Each TeX character is at most 3 UTF-8 bytes; +2 for NUL terminators.
    let work_size = (len_a + len_n + len_e) * 3 + 2;
    let work_buffer = xmalloc(work_size as libc::size_t) as *mut libc::c_char;
    *work_buffer = 0;

    let a_utf8 = crate::texmfmp::gettexstring(a);
    libc::strcat(work_buffer, a_utf8);
    libc::free(a_utf8 as *mut libc::c_void);

    let n_utf8 = crate::texmfmp::gettexstring(n);
    libc::strcat(work_buffer, n_utf8);
    libc::free(n_utf8 as *mut libc::c_void);

    let e_utf8 = crate::texmfmp::gettexstring(e);
    libc::strcat(work_buffer, e_utf8);
    libc::free(e_utf8 as *mut libc::c_void);

    let nl = libc::strlen(work_buffer) as i32;
    crate::globals::set_name_length(nl);
    libc::free(crate::globals::name_of_file() as *mut libc::c_void);
    let new_buf = xmalloc((nl + 2) as libc::size_t) as *mut libc::c_char;
    libc::strcpy(new_buf, work_buffer);
    crate::globals::set_name_of_file(new_buf);
    libc::free(work_buffer as *mut libc::c_void);
}

// translated from xetex-xetex0.c:9978 (insert_src_special)
#[no_mangle]
pub unsafe extern "C" fn insert_src_special() {
    use crate::globals::{
        in_open, line, mem_b32s1, set_mem_b32s0, set_mem_b32s1, source_filename_stack,
    };
    const FROZEN_SPECIAL: i32 = 2_243_226 + 10; // FROZEN_CONTROL_SEQUENCE + 10
    const RIGHT_BRACE_TOKEN: i32 = 0x400000;     // xetex-constants.h:351

    let sfn = source_filename_stack(in_open());
    if sfn > 0 && crate::texmfmp::is_new_source(sfn, line()) {
        let toklist = crate::mem::get_avail();
        let mut p = toklist;
        set_mem_b32s0(p, CS_TOKEN_FLAG + FROZEN_SPECIAL);
        set_mem_b32s1(p, crate::mem::get_avail());
        p = mem_b32s1(p);
        set_mem_b32s0(p, LEFT_BRACE_TOKEN + b'{' as i32);
        let q = str_toks(crate::texmfmp::make_src_special(sfn, line()));
        set_mem_b32s1(p, mem_b32s1(TEMP_HEAD));
        p = q;
        set_mem_b32s1(p, crate::mem::get_avail());
        p = mem_b32s1(p);
        set_mem_b32s0(p, RIGHT_BRACE_TOKEN + b'}' as i32);
        begin_token_list(toklist, INSERTED);
        crate::texmfmp::remember_source_info(sfn, line());
    }
}

// translated from xetex-xetex0.c:9984 (tt_insert_special)
#[no_mangle]
pub unsafe extern "C" fn tt_insert_special(ascii_text: *const libc::c_char) {
    const FROZEN_SPECIAL: i32 = 2_243_226 + 10; // FROZEN_CONTROL_SEQUENCE + 10
    const RIGHT_BRACE_TOKEN: i32 = 0x400000;     // xetex-constants.h:351

    let start_pool = pool_ptr();
    // Copy ASCII text into str_pool
    let text = std::ffi::CStr::from_ptr(ascii_text).to_bytes();
    if pool_ptr() + text.len() as i32 >= crate::globals::pool_size() {
        crate::errors::overflow(c"string pool overflow".as_ptr(), 0);
    }
    for &b in text {
        set_str_pool(pool_ptr(), b as u16);
        set_pool_ptr(pool_ptr() + 1);
    }
    // Build token list: \special { <tokens> }
    let toklist_start = crate::mem::get_avail();
    set_mem_b32s0(toklist_start, CS_TOKEN_FLAG + FROZEN_SPECIAL);
    let q = crate::mem::get_avail();
    set_mem_b32s0(q, LEFT_BRACE_TOKEN + b'{' as i32);
    set_mem_b32s1(toklist_start, q);
    let p = str_toks(start_pool);
    set_mem_b32s1(q, mem_b32s1(TEMP_HEAD));
    let q2 = crate::mem::get_avail();
    set_mem_b32s0(q2, RIGHT_BRACE_TOKEN + b'}' as i32);
    set_mem_b32s1(p, q2);
    begin_token_list(toklist_start, INSERTED);
}

// translated from xetex-xetex0.c:9485 (scan_box)
#[no_mangle]
pub unsafe extern "C" fn scan_box(box_context: i32) {
    use crate::globals::{cur_cmd, set_cur_box};
    const MAKE_BOX: u8 = 20;       // xetex_format.h:258
    const HRULE: u8 = 36;          // xetex_format.h:274
    const VRULE: u8 = 35;          // xetex_format.h:273
    const LEADER_FLAG: i32 = 0x40010001; // xetex-constants.h:375
    loop {
        get_x_token();
        if cur_cmd() != SPACER && cur_cmd() != RELAX { break; }
    }
    if cur_cmd() == MAKE_BOX {
        begin_box(box_context);
    } else if box_context >= LEADER_FLAG && (cur_cmd() == HRULE || cur_cmd() == VRULE) {
        set_cur_box(scan_rule_spec());
        box_end(box_context);
    } else {
        error_here_with_diagnostic(c"A <box> was supposed to be here".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(3);
        set_help_line(2, c"I was expecting to see \\hbox or \\vbox or \\copy or \\box or".as_ptr());
        set_help_line(1, c"something like that. So you might find something missing in".as_ptr());
        set_help_line(0, c"your output. But keep trying; you can fix this later.".as_ptr());
        crate::input::back_error();
    }
}

// translated from xetex-xetex0.c:9509 (package)
#[no_mangle]
pub unsafe extern "C" fn package(c: i16) {
    use crate::globals::{
        cur_box, cur_list_head, cur_list_mode, save_ptr, save_stack_b32s1, set_cur_box,
        set_save_ptr,
    };
    const DIMEN_PAR_BOX_MAX_DEPTH: i32 = 7;    // xetex_format.h:129
    const INT_PAR_XETEX_UPWARDS: i32 = 71;     // xetex_format.h:106
    const VTOP_CODE: i16 = 4;                  // xetex_format.h:484
    const INNER_HMODE: i16 = -104;             // xetex_format.h:369 negated

    let d = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_BOX_MAX_DEPTH);
    let u = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS);
    crate::save::unsave();
    set_save_ptr(save_ptr() - 3);
    let v = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS);
    crate::globals::set_eqtb_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS, u);
    if cur_list_mode() == INNER_HMODE {
        set_cur_box(hpack(
            mem_b32s1(cur_list_head()),
            save_stack_b32s1(save_ptr() + 2),
            save_stack_b32s1(save_ptr() + 1) as i16,
        ));
    } else {
        set_cur_box(vpackage(
            mem_b32s1(cur_list_head()),
            save_stack_b32s1(save_ptr() + 2),
            save_stack_b32s1(save_ptr() + 1) as i16,
            d,
        ));
        if c == VTOP_CODE {
            let mut h = 0i32;
            let p = mem_b32s1(cur_box() + 5);
            if p != crate::mem::TEX_NULL && mem_b16s1(p) <= 2 {
                h = mem_b32s1(p + 3);
            }
            crate::globals::set_mem_b32s1(cur_box() + 2,
                mem_b32s1(cur_box() + 2) - h + mem_b32s1(cur_box() + 3));
            crate::globals::set_mem_b32s1(cur_box() + 3, h);
        }
    }
    crate::globals::set_eqtb_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS, v);
    crate::mem::pop_nest();
    box_end(save_stack_b32s1(save_ptr()));
}

// translated from xetex-xetex0.c:9549 (new_graf)
#[no_mangle]
pub unsafe extern "C" fn new_graf(indented: bool) {
    use crate::globals::{
        cur_list_head, cur_list_mode, cur_list_tail,
        insert_src_special_every_par, nest_ptr, set_cur_lang, set_cur_list_aux_b32s0,
        set_cur_list_aux_b32s1, set_cur_list_mode, set_cur_list_prev_graf, set_cur_list_tail,
        set_mem_b32s1,
    };
    const GLUE_PAR_PAR_SKIP: i16 = 2;  // xetex_format.h:151
    const VMODE_I16: i16 = 1;          // xetex_format.h:369
    const HMODE_I16: i16 = 104;        // xetex_format.h:369
    const BIGGEST_LANG: i32 = 255;     // xetex-constants.h:327
    const LOCAL__EVERY_PAR: i32 = 2;   // xetex_format.h:174
    const EVERY_PAR_TEXT: u16 = 8;     // xetex-constants.h:207

    set_cur_list_prev_graf(0);

    if cur_list_mode() == VMODE_I16 || cur_list_head() != cur_list_tail() {
        let pg = crate::mem::new_param_glue(GLUE_PAR_PAR_SKIP);
        set_mem_b32s1(cur_list_tail(), pg);
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
    }

    crate::mem::push_nest();
    set_cur_list_mode(HMODE_I16);
    set_cur_list_aux_b32s0(1000);

    let lang_val = eqtb_b32s1(INT_BASE + INT_PAR_LANGUAGE);
    let l: u8 = if lang_val <= 0 || lang_val > BIGGEST_LANG { 0 } else { lang_val as u8 };
    set_cur_lang(l);
    set_cur_list_aux_b32s1(l as i32);
    let lhm = crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as i32;
    let rhm = crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as i32;
    set_cur_list_prev_graf((lhm * 64 + rhm) * 65536 + l as i32);

    if indented {
        let nb = crate::mem::new_null_box();
        set_cur_list_tail(nb);
        set_mem_b32s1(cur_list_head(), nb);
        set_mem_b32s1(nb + 1, eqtb_b32s1(DIMEN_BASE));
        if insert_src_special_every_par() {
            insert_src_special();
        }
    }

    let every_par = eqtb_b32s1(LOCAL_BASE + LOCAL__EVERY_PAR);
    if every_par != crate::mem::TEX_NULL {
        begin_token_list(every_par, EVERY_PAR_TEXT as u16);
    }

    if nest_ptr() == 1 {
        build_page();
    }
}

// translated from xetex-xetex0.c:7048 (new_character)
#[no_mangle]
pub unsafe extern "C" fn new_character(f: i32, c: u16) -> i32 {
    extern "C" { fn new_native_character(f: i32, c: u32) -> i32; }
    use crate::globals::{char_base_entry, font_bc_entry, font_ec_entry, font_info_b16};

    let fa = crate::globals::font_area_entry(f);
    if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
        return new_native_character(f, c as u32);
    }
    let ec = effective_char(false, f, c) as u16;
    if font_bc_entry(f) <= ec && font_ec_entry(f) >= ec {
        let info = font_info_b16(char_base_entry(f) + ec as i32);
        if info.s3 > 0 {
            let p = crate::mem::get_avail();
            crate::globals::set_mem_b16s1(p, f as u16);
            crate::globals::set_mem_b16s0(p, c);
            return p;
        }
    }
    char_warning(f, c as i32);
    crate::mem::TEX_NULL
}

// translated from xetex-xetex0.c:4313 (scan_math)
#[no_mangle]
pub unsafe extern "C" fn scan_math(p: i32) {
    use crate::globals::{cur_chr, cur_cmd, cur_cs, eqtb_b16s1, set_cur_chr, set_cur_cmd, set_cur_cs};
    const LETTER: u8 = 11;          // xetex_format.h:243
    const OTHER_CHAR_CMD: u8 = 12;  // xetex_format.h:244
    const DELIM_NUM: u8 = 15;       // xetex_format.h:251
    const CHAR_NUM_CMD: u8 = 16;    // xetex_format.h:253
    const MATH_CHAR_NUM: u8 = 17;   // xetex_format.h:255
    const CHAR_GIVEN_CMD: u8 = 68;  // xetex_format.h:306
    const MATH_GIVEN_CMD: u8 = 69;  // xetex_format.h:308
    const XETEX_MATH_GIVEN: u8 = 70; // xetex_format.h:309
    const MATH_CODE_BASE_SC: i32 = 6_712_617; // xetex_format.h:218
    const ACTIVE_MATH_CHAR: i32 = 0x1FFFFF;   // xetex-constants.h:343
    const MATH_CHAR_NODE: i32 = 1;            // xetex-constants.h:269
    const MATH_GROUP_TYPE: u8 = 9;            // xetex-constants.h:161
    const INT_PAR_CUR_FAM: i32 = 44;          // xetex_format.h:79
    const NUMBER_MATH_FAMILIES_VAL: i32 = 256; // xetex-constants.h:31

    let c: i32;
    'main: loop {
        loop {
            get_x_token();
            if cur_cmd() != SPACER && cur_cmd() != RELAX { break; }
        }
        c = loop {
            break match cur_cmd() {
                LETTER | OTHER_CHAR_CMD | CHAR_GIVEN_CMD => {
                    let mc = eqtb_b32s1(MATH_CODE_BASE_SC + cur_chr());
                    if mc & 0x1FFFFF == ACTIVE_MATH_CHAR {
                        set_cur_cs(cur_chr() + 1);
                        set_cur_cmd(eqtb_b16s1(cur_cs()) as u8);
                        set_cur_chr(eqtb_b32s1(cur_cs()));
                        x_token();
                        crate::input::back_input();
                        continue 'main;
                    }
                    mc
                }
                CHAR_NUM_CMD => {
                    scan_char_num();
                    set_cur_chr(cur_val());
                    set_cur_cmd(CHAR_GIVEN_CMD);
                    continue; // goto reswitch
                }
                MATH_CHAR_NUM => {
                    if cur_chr() == 2 {
                        scan_math_class_int();
                        let mut cv = set_class_fn(cur_val());
                        scan_math_fam_int();
                        cv += set_family_fn(cur_val());
                        scan_usv_num();
                        cv + cur_val()
                    } else if cur_chr() == 1 {
                        scan_xetex_math_char_int();
                        cur_val()
                    } else {
                        scan_fifteen_bit_int();
                        let v = cur_val();
                        set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256)
                    }
                }
                MATH_GIVEN_CMD => {
                    let v = cur_chr();
                    set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256)
                }
                XETEX_MATH_GIVEN => cur_chr(),
                DELIM_NUM => {
                    if cur_chr() == 1 {
                        scan_math_class_int();
                        let mut cv = set_class_fn(cur_val());
                        scan_math_fam_int();
                        cv += set_family_fn(cur_val());
                        scan_usv_num();
                        cv + cur_val()
                    } else {
                        scan_delimiter_int();
                        let v = cur_val() / 4096;
                        set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256)
                    }
                }
                _ => {
                    crate::input::back_input();
                    scan_left_brace();
                    crate::globals::set_save_stack_b32s1(save_ptr(), p);
                    crate::globals::set_save_ptr(save_ptr() + 1);
                    push_math(MATH_GROUP_TYPE);
                    return;
                }
            };
        };
        break;
    }
    set_mem_b32s1(p, MATH_CHAR_NODE);
    crate::globals::set_mem_b16s0(p, (c % 65536) as u16);
    let cur_fam_val = eqtb_b32s1(INT_BASE + INT_PAR_CUR_FAM);
    let fam: u16 = if math_class_fn(c) == 7
        && cur_fam_val >= 0
        && cur_fam_val < NUMBER_MATH_FAMILIES_VAL
    {
        cur_fam_val as u16
    } else {
        math_fam_fn(c) as u16
    };
    crate::globals::set_mem_b16s1(p, fam + (math_char_fn(c) / 65536) as u16 * 256);
}

// translated from xetex-xetex0.c:4315 (set_math_char)
#[no_mangle]
pub unsafe extern "C" fn set_math_char(c: i32) {
    use crate::globals::{cur_chr, cur_cs, set_cur_chr, set_cur_cmd, set_cur_cs, set_mem_b16s0,
                         set_mem_b16s1, set_mem_b32s1};
    const ACTIVE_MATH_CHAR_SM: i32 = 0x1FFFFF;
    const MATH_CHAR_NODE_SM: i32 = 1;   // xetex-constants.h:269
    const ORD_NOAD: u16 = 16;           // xetex_format.h:401
    const INT_PAR_CUR_FAM_SM: i32 = 44;
    const NUMBER_MATH_FAM_SM: i32 = 256;

    if math_char_fn(c) == ACTIVE_MATH_CHAR_SM {
        set_cur_cs(cur_chr() + 1);
        set_cur_cmd(crate::globals::eqtb_b16s1(cur_cs()) as u8);
        set_cur_chr(eqtb_b32s1(cur_cs()));
        x_token();
        crate::input::back_input();
    } else {
        let p = crate::mem::new_noad();
        set_mem_b32s1(p + 1, MATH_CHAR_NODE_SM);
        let ch = math_char_fn(c);
        set_mem_b16s0(p + 1, (ch % 65536) as u16);
        set_mem_b16s1(p + 1, math_fam_fn(c) as u16);
        if math_class_fn(c) == 7 {
            let cf = eqtb_b32s1(INT_BASE + INT_PAR_CUR_FAM_SM);
            if cf >= 0 && cf < NUMBER_MATH_FAM_SM {
                set_mem_b16s1(p + 1, cf as u16);
            }
            crate::globals::set_mem_b16s1(p, ORD_NOAD);
        } else {
            crate::globals::set_mem_b16s1(p, ORD_NOAD + math_class_fn(c) as u16);
        }
        set_mem_b16s1(p + 1, crate::globals::mem_b16s1(p + 1) + (ch / 65536) as u16 * 256);
        set_mem_b32s1(crate::globals::cur_list_tail(), p);
        crate::globals::set_cur_list_tail(p);
    }
}

#[inline]
fn math_char_fn(c: i32) -> i32 { (c as u32 & 0x1FFFFF) as i32 }
#[inline]
fn math_class_fn(c: i32) -> i32 { ((c as u32 >> 21) & 0x07) as i32 }
#[inline]
fn math_fam_fn(c: i32) -> i32 { ((c as u32 >> 24) & 0xFF) as i32 }
#[inline]
fn set_class_fn(x: i32) -> i32 { (((x as u32) & 0x07) << 21) as i32 }
#[inline]
fn set_family_fn(x: i32) -> i32 { (((x as u32) & 0xFF) << 24) as i32 }

// translated from xetex-xetex0.c:7728 (get_preamble_token)
#[no_mangle]
pub unsafe extern "C" fn get_preamble_token() {
    use crate::globals::{cur_chr, cur_cmd};
    const TAB_MARK: u8 = 4;           // xetex_format.h:234
    const SPAN_CODE_GPT: i32 = 1_114_113; // xetex_format.h:679
    const GLUE_BASE_TAB_SKIP: i32 = 2_254_340 + 11; // GLUE_BASE + GLUE_PAR__tab_skip
    const GLUE_REF_GPT: u16 = 119;    // xetex_format.h:361
    const INT_PAR_GLOBAL_DEFS: i32 = 43; // xetex_format.h:78

    'restart: loop {
        get_token();
        while cur_chr() == SPAN_CODE_GPT && cur_cmd() == TAB_MARK {
            get_token();
            if cur_cmd() > MAX_COMMAND {
                expand();
                get_token();
            }
        }
        if cur_cmd() == ENDV {
            crate::errors::fatal_error(c"(interwoven alignment preambles are not allowed)".as_ptr());
        }
        if cur_cmd() == ASSIGN_GLUE && cur_chr() == GLUE_BASE_TAB_SKIP {
            scan_optional_equals();
            scan_glue(GLUE_VAL);
            if eqtb_b32s1(INT_BASE + INT_PAR_GLOBAL_DEFS) > 0 {
                crate::save::geq_define(GLUE_BASE_TAB_SKIP, GLUE_REF_GPT, cur_val());
            } else {
                crate::save::eq_define(GLUE_BASE_TAB_SKIP, GLUE_REF_GPT, cur_val());
            }
            continue 'restart;
        }
        break;
    }
}

// translated from xetex-xetex0.c:7880 (init_span)
#[no_mangle]
pub unsafe extern "C" fn init_span(p: i32) {
    use crate::globals::{
        cur_list_mode, set_cur_list_aux_b32s0, set_cur_list_aux_b32s1, set_cur_span,
    };
    const INNER_HMODE: i16 = -104;

    crate::mem::push_nest();
    if cur_list_mode() == INNER_HMODE {
        set_cur_list_aux_b32s0(1000);
    } else {
        set_cur_list_aux_b32s1(IGNORE_DEPTH);
        normal_paragraph();
    }
    set_cur_span(p);
}

// translated from xetex-xetex0.c:7893 (init_row)
#[no_mangle]
pub unsafe extern "C" fn init_row() {
    use crate::globals::{
        cur_head, cur_list_mode, cur_list_tail, cur_pre_head, set_cur_align, set_cur_list_aux_b32s0,
        set_cur_list_aux_b32s1, set_cur_list_mode, set_cur_list_tail, set_cur_pre_tail,
        set_cur_tail_a, set_mem_b16s0, set_mem_b32s1,
    };
    const ALIGN_HEAD_IR: i32 = 4_999_991;
    const GLUE_PAR_TAB_SKIP: u16 = 11 + 1; // GLUE_PAR__tab_skip + 1

    crate::mem::push_nest();
    let new_mode = -105 - cur_list_mode() as i32;
    set_cur_list_mode(new_mode as i16);
    if new_mode == -104 {
        set_cur_list_aux_b32s0(0);
    } else {
        set_cur_list_aux_b32s1(0);
    }
    let align_head_link = mem_b32s1(ALIGN_HEAD_IR);
    let glue_node = crate::mem::new_glue(mem_b32s0(align_head_link + 1));
    set_mem_b32s1(cur_list_tail(), glue_node);
    set_cur_list_tail(mem_b32s1(cur_list_tail()));
    set_mem_b16s0(cur_list_tail(), GLUE_PAR_TAB_SKIP);
    set_cur_align(mem_b32s1(align_head_link));
    set_cur_tail_a(cur_head());
    set_cur_pre_tail(cur_pre_head());
    init_span(crate::globals::cur_align());
}

// translated from xetex-xetex0.c:7912 (init_col)
#[no_mangle]
pub unsafe extern "C" fn init_col() {
    use crate::globals::{cur_align, cur_cmd, set_align_state, set_mem_b32s0};
    const OMIT_CMD: u8 = 63;    // xetex_format.h:301
    const U_TEMPLATE: u16 = 1;  // xetex-constants.h:200

    set_mem_b32s0(cur_align() + 5, cur_cmd() as i32);
    if cur_cmd() == OMIT_CMD {
        set_align_state(0);
    } else {
        crate::input::back_input();
        begin_token_list(mem_b32s1(cur_align() + 3), U_TEMPLATE);
    }
}

// translated from xetex-xetex0.c:7724 (push_alignment)
#[no_mangle]
pub unsafe extern "C" fn push_alignment() {
    use crate::globals::{
        align_ptr, align_state, avail, cur_align, cur_head, cur_loop, cur_pre_head, cur_pre_tail,
        cur_span, cur_tail_a, set_align_ptr, set_cur_head, set_cur_pre_head, set_mem_b32s0,
        set_mem_b32s1,
    };
    const ALIGN_HEAD: i32 = 4_999_991; // MEM_TOP - 8
    const ALIGN_STACK_NODE_SIZE: i32 = 6; // xetex-constants.h:129

    let p = crate::mem::get_node(ALIGN_STACK_NODE_SIZE);
    set_mem_b32s1(p, align_ptr());
    set_mem_b32s0(p, cur_align());
    set_mem_b32s0(p + 1, mem_b32s1(ALIGN_HEAD));
    set_mem_b32s1(p + 1, cur_span());
    set_mem_b32s1(p + 2, cur_loop());
    set_mem_b32s1(p + 3, align_state());
    set_mem_b32s0(p + 4, cur_head());
    set_mem_b32s1(p + 4, cur_tail_a());
    set_mem_b32s0(p + 5, cur_pre_head());
    set_mem_b32s1(p + 5, cur_pre_tail());
    set_align_ptr(p);
    set_cur_head(crate::mem::get_avail());
    set_cur_pre_head(crate::mem::get_avail());
}

// translated from xetex-xetex0.c:7743 (pop_alignment)
#[no_mangle]
pub unsafe extern "C" fn pop_alignment() {
    use crate::globals::{
        align_ptr, avail, cur_align, cur_head, cur_loop, cur_pre_head, cur_pre_tail, cur_span,
        mem_b32s0, set_align_ptr, set_align_state, set_cur_align, set_cur_head, set_cur_loop,
        set_cur_pre_head, set_cur_pre_tail, set_cur_span, set_cur_tail_a, set_mem_b32s1,
    };
    const ALIGN_HEAD: i32 = 4_999_991;
    const ALIGN_STACK_NODE_SIZE: i32 = 6;

    // free_avail(cur_head): mem[cur_head].b32.s1 = avail; avail = cur_head
    crate::globals::set_mem_b32s1(cur_head(), avail());
    crate::globals::set_avail(cur_head());
    // free_avail(cur_pre_head)
    crate::globals::set_mem_b32s1(cur_pre_head(), avail());
    crate::globals::set_avail(cur_pre_head());

    let p = align_ptr();
    set_cur_tail_a(mem_b32s1(p + 4));
    set_cur_head(mem_b32s0(p + 4));
    set_cur_pre_tail(mem_b32s1(p + 5));
    set_cur_pre_head(mem_b32s0(p + 5));
    set_align_state(mem_b32s1(p + 3));
    set_cur_loop(mem_b32s1(p + 2));
    set_cur_span(mem_b32s1(p + 1));
    set_mem_b32s1(ALIGN_HEAD, mem_b32s0(p + 1));
    set_cur_align(mem_b32s0(p));
    set_align_ptr(mem_b32s1(p));
    crate::mem::free_node(p, ALIGN_STACK_NODE_SIZE);
}

// translated from xetex-xetex0.c:4678 (scan_glue)
#[no_mangle]
pub unsafe extern "C" fn scan_glue(level: i16) {
    use crate::globals::{cur_order, cur_val_level, set_cur_val, set_mem_b16s0, set_mem_b16s1,
                         set_mem_b32s1};
    const MINUS_TOKEN: i32 = OTHER_TOKEN + b'-' as i32;
    const PLUS_TOKEN: i32 = OTHER_TOKEN + b'+' as i32;

    let mu = level == MU_VAL;
    let mut negative = false;

    loop {
        loop {
            get_x_token();
            if cur_cmd() != SPACER { break; }
        }
        if cur_tok() == MINUS_TOKEN {
            negative = !negative;
            set_cur_tok(PLUS_TOKEN);
        }
        if cur_tok() != PLUS_TOKEN { break; }
    }

    if cur_cmd() >= MIN_INTERNAL && cur_cmd() <= MAX_INTERNAL {
        scan_something_internal(level, negative);
        if cur_val_level() as i16 >= GLUE_VAL {
            if cur_val_level() as i16 != level { mu_error(); }
            return;
        }
        if cur_val_level() as i16 == INT_VAL {
            scan_dimen(mu, false, true);
        } else if level == MU_VAL {
            mu_error();
        }
    } else {
        crate::input::back_input();
        scan_dimen(mu, false, false);
        if negative { set_cur_val(-(cur_val() as i32)); }
    }

    let q = new_spec(0);
    set_mem_b32s1(q + 1, cur_val());

    if scan_keyword(c"plus".as_ptr()) {
        scan_dimen(mu, true, false);
        set_mem_b32s1(q + 2, cur_val());
        set_mem_b16s1(q, cur_order() as u16);
    }
    if scan_keyword(c"minus".as_ptr()) {
        scan_dimen(mu, true, false);
        set_mem_b32s1(q + 3, cur_val());
        set_mem_b16s0(q, cur_order() as u16);
    }

    set_cur_val(q);
}

// translated from xetex-xetex0.c:7859 (fin_col)
#[no_mangle]
pub unsafe extern "C" fn fin_col() -> bool {
    use crate::globals::{
        adjust_tail, cur_align, cur_list_head, cur_list_mode, cur_list_tail, cur_loop,
        cur_pre_tail, cur_span, cur_tail_a, mem_b16s0, mem_b32s0, mem_b32s1, pre_adjust_tail,
        set_adjust_tail, set_cur_align, set_cur_list_tail, set_cur_loop, set_cur_pre_tail,
        set_cur_span, set_cur_tail_a, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1,
        set_pre_adjust_tail, total_shrink, total_stretch,
    };
    use crate::mem::TEX_NULL;

    const ADDITIONAL: i16 = 1;
    const SPAN_CODE: i32 = 1_114_113;          // xetex_format.h:679
    const CR_CODE: i32 = 1_114_114;            // xetex_format.h:680
    const SPAN_NODE_SIZE: i32 = 2;             // xetex-constants.h:111
    const ALIGN_GROUP: u8 = 6;                 // xetex-constants.h:158
    const HOLD_HEAD: i32 = 4_999_995;          // MEM_TOP - 4
    const END_SPAN: i32 = 4_999_990;           // MEM_TOP - 9
    const NULL_FLAG: i32 = -0x4000_0000;       // xetex-constants.h:16
    const GLUE_PAR_TAB_SKIP: u16 = 12;         // GLUE_PAR__tab_skip + 1
    const INNER_HMODE: i16 = -104;
    const FIL: usize = 1;
    const FILL: usize = 2;
    const FILLL: usize = 3;
    const UNSET_NODE: u16 = 13;                // xetex_format.h

    if cur_align() == TEX_NULL {
        crate::errors::confusion(c"endv".as_ptr());
    }
    let q = mem_b32s1(cur_align());
    if q == TEX_NULL {
        crate::errors::confusion(c"endv".as_ptr());
    }
    if align_state() < 500_000 {
        crate::errors::fatal_error(c"(interwoven alignment preambles are not allowed)".as_ptr());
    }
    let mut p = mem_b32s1(q);
    if p == TEX_NULL && mem_b32s0(cur_align() + 5) < CR_CODE {
        if cur_loop() != TEX_NULL {
            // §822: extend preamble by duplicating the loop template
            set_mem_b32s1(q, crate::mem::new_null_box());
            p = mem_b32s1(q);
            set_mem_b32s0(p, END_SPAN);
            set_mem_b32s1(p + 1, NULL_FLAG);
            set_cur_loop(mem_b32s1(cur_loop()));
            // copy u-template from cur_loop+3 into HOLD_HEAD list → p+3
            let mut wq = HOLD_HEAD;
            let mut r = mem_b32s1(cur_loop() + 3);
            while r != TEX_NULL {
                set_mem_b32s1(wq, crate::mem::get_avail());
                wq = mem_b32s1(wq);
                set_mem_b32s0(wq, mem_b32s0(r));
                r = mem_b32s1(r);
            }
            set_mem_b32s1(wq, TEX_NULL);
            set_mem_b32s1(p + 3, mem_b32s1(HOLD_HEAD));
            // copy v-template from cur_loop+2 into HOLD_HEAD list → p+2
            wq = HOLD_HEAD;
            r = mem_b32s1(cur_loop() + 2);
            while r != TEX_NULL {
                set_mem_b32s1(wq, crate::mem::get_avail());
                wq = mem_b32s1(wq);
                set_mem_b32s0(wq, mem_b32s0(r));
                r = mem_b32s1(r);
            }
            set_mem_b32s1(wq, TEX_NULL);
            set_mem_b32s1(p + 2, mem_b32s1(HOLD_HEAD));
            set_cur_loop(mem_b32s1(cur_loop()));
            set_mem_b32s1(p, crate::mem::new_glue(mem_b32s0(cur_loop() + 1)));
            set_mem_b16s0(mem_b32s1(p), GLUE_PAR_TAB_SKIP);
        } else {
            error_here_with_diagnostic(c"Extra alignment tab has been changed to ".as_ptr());
            crate::output::print_esc_cstr(c"cr".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(3);
            set_help_line(2, c"You have given more \\span or & marks than there were".as_ptr());
            set_help_line(1, c"in the preamble to the \\halign or \\valign now in progress.".as_ptr());
            set_help_line(0, c"So I'll assume that you meant to type \\cr instead.".as_ptr());
            set_mem_b32s0(cur_align() + 5, CR_CODE);
            error();
        }
    }
    if mem_b32s0(cur_align() + 5) != SPAN_CODE {
        crate::save::unsave();
        crate::save::new_save_level(ALIGN_GROUP);
        let (u, w) = if cur_list_mode() == INNER_HMODE {
            set_adjust_tail(cur_tail_a());
            set_pre_adjust_tail(cur_pre_tail());
            let u = hpack(mem_b32s1(cur_list_head()), 0, ADDITIONAL);
            let w = mem_b32s1(u + 1);
            set_cur_tail_a(adjust_tail());
            set_adjust_tail(TEX_NULL);
            set_cur_pre_tail(pre_adjust_tail());
            set_pre_adjust_tail(TEX_NULL);
            (u, w)
        } else {
            let u = vpackage(mem_b32s1(cur_list_head()), 0, ADDITIONAL, 0);
            let w = mem_b32s1(u + 3);
            (u, w)
        };
        let mut n: i32 = 0;
        if cur_span() != cur_align() {
            // §827: compare accumulated span width with recorded widths
            let mut sq = cur_span();
            loop {
                n += 1;
                sq = mem_b32s1(mem_b32s1(sq));
                if sq == cur_align() { break; }
            }
            if n > u16::MAX as i32 {
                crate::errors::confusion(c"too many spans".as_ptr());
            }
            sq = cur_span();
            while mem_b32s1(mem_b32s0(sq)) < n {
                sq = mem_b32s0(sq);
            }
            if mem_b32s1(mem_b32s0(sq)) > n {
                let s = crate::mem::get_node(SPAN_NODE_SIZE);
                set_mem_b32s0(s, mem_b32s0(sq));
                set_mem_b32s1(s, n);
                set_mem_b32s0(sq, s);
                set_mem_b32s1(s + 1, w);
            } else if mem_b32s1(mem_b32s0(sq) + 1) < w {
                set_mem_b32s1(mem_b32s0(sq) + 1, w);
            }
        } else if w > mem_b32s1(cur_align() + 1) {
            set_mem_b32s1(cur_align() + 1, w);
        }
        set_mem_b16s1(u, UNSET_NODE);
        set_mem_b16s0(u, n as u16);
        let o_str: usize = if total_stretch(FILLL) != 0 { FILLL }
            else if total_stretch(FILL) != 0 { FILL }
            else if total_stretch(FIL) != 0 { FIL }
            else { 0 };
        set_mem_b16s0(u + 5, o_str as u16);
        set_mem_b32s1(u + 6, total_stretch(o_str));
        let o_shr: usize = if total_shrink(FILLL) != 0 { FILLL }
            else if total_shrink(FILL) != 0 { FILL }
            else if total_shrink(FIL) != 0 { FIL }
            else { 0 };
        set_mem_b16s1(u + 5, o_shr as u16);
        set_mem_b32s1(u + 4, total_shrink(o_shr));
        crate::mem::pop_nest();
        set_mem_b32s1(cur_list_tail(), u);
        set_cur_list_tail(u);
        set_mem_b32s1(cur_list_tail(), crate::mem::new_glue(mem_b32s0(mem_b32s1(cur_align()) + 1)));
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        set_mem_b16s0(cur_list_tail(), GLUE_PAR_TAB_SKIP);
        if mem_b32s0(cur_align() + 5) >= CR_CODE {
            return true;
        }
        init_span(p);
    }
    set_align_state(1_000_000);
    loop {
        get_x_or_protected();
        if cur_cmd() != SPACER { break; }
    }
    set_cur_align(p);
    init_col();
    false
}

// translated from xetex-xetex0.c:8009 (fin_row)
#[no_mangle]
pub unsafe extern "C" fn fin_row() {
    use crate::globals::{
        cur_head, cur_list_head, cur_list_mode, cur_list_tail, cur_pre_head, cur_pre_tail,
        cur_tail_a, mem_b32s1, set_cur_list_aux_b32s0, set_cur_list_tail, set_mem_b16s1,
        set_mem_b32s1,
    };
    use crate::mem::TEX_NULL;

    const ADDITIONAL: i16 = 1;
    const UNSET_NODE: u16 = 13;
    const INNER_HMODE: i16 = -104;
    const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
    const EVERY_CR_TEXT: u16 = 14;             // xetex-constants.h:213
    const LOCAL__EVERY_CR: i32 = 8;            // xetex_format.h:180

    let p = if cur_list_mode() == INNER_HMODE {
        let u = hpack(mem_b32s1(cur_list_head()), 0, ADDITIONAL);
        crate::mem::pop_nest();
        if cur_pre_head() != cur_pre_tail() {
            set_mem_b32s1(cur_list_tail(), mem_b32s1(cur_pre_head()));
            set_cur_list_tail(cur_pre_tail());
        }
        append_to_vlist(u);
        if cur_head() != cur_tail_a() {
            set_mem_b32s1(cur_list_tail(), mem_b32s1(cur_head()));
            set_cur_list_tail(cur_tail_a());
        }
        u
    } else {
        let u = vpackage(mem_b32s1(cur_list_head()), 0, ADDITIONAL, MAX_HALFWORD);
        crate::mem::pop_nest();
        set_mem_b32s1(cur_list_tail(), u);
        set_cur_list_tail(u);
        set_cur_list_aux_b32s0(1000);
        u
    };
    set_mem_b16s1(p, UNSET_NODE);
    set_mem_b32s1(p + 6, 0);
    let every_cr = eqtb_b32s1(LOCAL_BASE + LOCAL__EVERY_CR);
    if every_cr != TEX_NULL {
        begin_token_list(every_cr, EVERY_CR_TEXT);
    }
    align_peek();
}

// translated from xetex-xetex0.c:7732 (init_align)
#[no_mangle]
pub unsafe extern "C" fn init_align() {
    use crate::globals::{
        cur_align, cur_cs, cur_list_aux_b32s1, cur_list_head, cur_list_mode, cur_list_tail,
        cur_loop, mem_b32s1, set_align_state, set_cur_align, set_cur_list_aux_b32s1,
        set_cur_list_mode, set_cur_loop, set_mem_b32s0, set_mem_b32s1, set_scanner_status,
        set_warning_index,
    };
    use crate::mem::TEX_NULL;

    const ALIGN_HEAD: i32 = 4_999_991;         // MEM_TOP - 8
    const END_SPAN: i32 = 4_999_990;           // MEM_TOP - 9
    const HOLD_HEAD: i32 = 4_999_995;          // MEM_TOP - 4
    const NULL_FLAG: i32 = -0x4000_0000;
    const CS_TOKEN_FLAG: i32 = 0x1FFFFFF;
    const FROZEN_END_TEMPLATE: i32 = 2_243_226 + 5; // FROZEN_CONTROL_SEQUENCE + 5
    const ALIGN_GROUP: u8 = 6;
    const EVERY_CR_TEXT: u16 = 14;
    const LOCAL__EVERY_CR: i32 = 8;
    const GLUE_PAR_TAB_SKIP: i16 = 11;         // GLUE_PAR__tab_skip
    const MMODE: i16 = 207;
    const ALIGNING: u8 = 4;
    const NORMAL_SCANNER: u8 = 0;
    const IGNORE_DEPTH: i32 = -65536000;
    const TAB_MARK: u8 = 4;
    const CAR_RET: u8 = 5;
    const MAC_PARAM: u8 = 6;
    const SPACER: u8 = 10;

    let save_cs_ptr = cur_cs();
    push_alignment();
    set_align_state(-1_000_000);

    if cur_list_mode() == MMODE
        && (cur_list_tail() != cur_list_head()
            || cur_list_aux_b32s1() != TEX_NULL)
    {
        error_here_with_diagnostic(c"Improper ".as_ptr());
        crate::output::print_esc_cstr(c"halign".as_ptr());
        crate::output::print_cstr(c" inside $$'s".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(3);
        set_help_line(2, c"Displays can use special alignments (like \\eqalignno)".as_ptr());
        set_help_line(1, c"only if nothing but the alignment itself is between $$'s.".as_ptr());
        set_help_line(0, c"So I've deleted the formulas that preceded this alignment.".as_ptr());
        error();
        flush_math();
    }

    crate::mem::push_nest();

    if cur_list_mode() == MMODE {
        set_cur_list_mode(-1);
        let aux = crate::globals::nest_aux_b32s1(crate::globals::nest_ptr() - 2);
        set_cur_list_aux_b32s1(aux);
    } else if cur_list_mode() > 0 {
        set_cur_list_mode(-(cur_list_mode() as i32) as i16);
    }

    scan_spec(ALIGN_GROUP, false);
    set_mem_b32s1(ALIGN_HEAD, TEX_NULL);
    set_cur_align(ALIGN_HEAD);
    set_cur_loop(TEX_NULL);
    set_scanner_status(ALIGNING);
    set_warning_index(save_cs_ptr);
    set_align_state(-1_000_000);

    'outer: loop {
        set_mem_b32s1(cur_align(), crate::mem::new_param_glue(GLUE_PAR_TAB_SKIP));
        set_cur_align(mem_b32s1(cur_align()));
        if cur_cmd() == CAR_RET {
            break 'outer;
        }

        let mut p = HOLD_HEAD;
        set_mem_b32s1(p, TEX_NULL);

        // scan u-template up to first #
        loop {
            get_preamble_token();
            if cur_cmd() == MAC_PARAM {
                break; // done1
            }
            if cur_cmd() <= CAR_RET && cur_cmd() >= TAB_MARK && align_state() == -1_000_000 {
                if p == HOLD_HEAD && cur_loop() == TEX_NULL && cur_cmd() == TAB_MARK {
                    set_cur_loop(cur_align());
                } else {
                    error_here_with_diagnostic(c"Missing # inserted in alignment preamble".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(3);
                    set_help_line(2, c"There should be exactly one # between &'s, when an".as_ptr());
                    set_help_line(1, c"\\halign or \\valign is being set up. In this case you had".as_ptr());
                    set_help_line(0, c"none, so I've put one in; maybe that will work.".as_ptr());
                    crate::input::back_error();
                    break; // done1
                }
            } else if cur_cmd() != SPACER || p != HOLD_HEAD {
                set_mem_b32s1(p, crate::mem::get_avail());
                p = mem_b32s1(p);
                set_mem_b32s0(p, cur_tok());
            }
        }

        // done1: set up the column preamble node
        set_mem_b32s1(cur_align(), crate::mem::new_null_box());
        set_cur_align(mem_b32s1(cur_align()));
        set_mem_b32s0(cur_align(), END_SPAN);
        set_mem_b32s1(cur_align() + 1, NULL_FLAG);
        set_mem_b32s1(cur_align() + 3, mem_b32s1(HOLD_HEAD));
        p = HOLD_HEAD;
        set_mem_b32s1(p, TEX_NULL);

        // scan v-template after #
        loop {
            get_preamble_token();
            if cur_cmd() <= CAR_RET && cur_cmd() >= TAB_MARK && align_state() == -1_000_000 {
                break; // done2
            }
            if cur_cmd() == MAC_PARAM {
                error_here_with_diagnostic(c"Only one # is allowed per tab".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(3);
                set_help_line(2, c"There should be exactly one # between &'s, when an".as_ptr());
                set_help_line(1, c"\\halign or \\valign is being set up. In this case you had".as_ptr());
                set_help_line(0, c"more than one, so I'm ignoring all but the first.".as_ptr());
                error();
                continue; // goto continue_: restart this inner loop
            }
            set_mem_b32s1(p, crate::mem::get_avail());
            p = mem_b32s1(p);
            set_mem_b32s0(p, cur_tok());
        }

        // done2: append FROZEN_END_TEMPLATE and store v-template
        set_mem_b32s1(p, crate::mem::get_avail());
        p = mem_b32s1(p);
        set_mem_b32s0(p, CS_TOKEN_FLAG + FROZEN_END_TEMPLATE);
        set_mem_b32s1(cur_align() + 2, mem_b32s1(HOLD_HEAD));
    }

    // done:
    set_scanner_status(NORMAL_SCANNER);
    crate::save::new_save_level(ALIGN_GROUP);
    let every_cr = eqtb_b32s1(LOCAL_BASE + LOCAL__EVERY_CR);
    if every_cr != TEX_NULL {
        begin_token_list(every_cr, EVERY_CR_TEXT);
    }
    align_peek();
}

// translated from xetex-xetex0.c:10745 (do_assignments)
#[no_mangle]
pub unsafe extern "C" fn do_assignments() {
    extern "C" {
        fn prefixed_command();
    }

    const MAX_NON_PREFIXED_COMMAND: u8 = 71; // xetex_format.h:312
    const RELAX: u8 = 0;                     // xetex_format.h:229

    loop {
        loop {
            get_x_token();
            if cur_cmd() != SPACER && cur_cmd() != RELAX { break; }
        }
        if cur_cmd() <= MAX_NON_PREFIXED_COMMAND { return; }
        crate::globals::set_set_box_allowed(false);
        prefixed_command();
        crate::globals::set_set_box_allowed(true);
    }
}

// translated from xetex-xetex0.c:107 (runaway)
#[no_mangle]
pub unsafe extern "C" fn runaway() {
    use crate::globals::{def_ref, error_line, mem_b32s1, scanner_status};
    use crate::mem::TEX_NULL;

    const SKIPPING: u8 = 1;
    const DEFINING: u8 = 2;
    const MATCHING: u8 = 3;
    const ALIGNING: u8 = 4;
    const ABSORBING: u8 = 5;
    const TEMP_HEAD: i32 = 4_999_996; // MEM_TOP - 3
    const HOLD_HEAD: i32 = 4_999_995; // MEM_TOP - 4

    if scanner_status() > SKIPPING {
        let p = match scanner_status() {
            DEFINING  => { crate::output::print_nl_cstr(c"Runaway definition".as_ptr()); def_ref() }
            MATCHING  => { crate::output::print_nl_cstr(c"Runaway argument".as_ptr());   TEMP_HEAD }
            ALIGNING  => { crate::output::print_nl_cstr(c"Runaway preamble".as_ptr());   HOLD_HEAD }
            ABSORBING => { crate::output::print_nl_cstr(c"Runaway text".as_ptr());        def_ref() }
            _         => return,
        };
        crate::output::print_char('?' as i32);
        crate::output::print_ln();
        crate::output::show_token_list(mem_b32s1(p), TEX_NULL, error_line() - 10);
    }
}

// translated from xetex-xetex0.c:7725 (vert_break)
#[no_mangle]
pub unsafe extern "C" fn vert_break(p: i32, h: i32, d: i32) -> i32 {
    use crate::mem::TEX_NULL;

    const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
    const EJECT_PENALTY: i32 = -10_000;  // xetex-constants.h:336
    const INF_PENALTY: i32 = 10_000;     // xetex-constants.h:338
    const INF_BAD: i32 = 10_000;         // xetex-constants.h:337
    const MATH_NODE: u16 = 9;
    const GLUE_NODE: u16 = 10;
    const KERN_NODE: u16 = 11;
    const PENALTY_NODE: u16 = 12;
    const PIC_NODE: u16 = 43;   // xetex_format.h:466
    const PDF_NODE: u16 = 44;   // xetex_format.h:467
    const NORMAL: u16 = 0;

    let mut p = p;
    let mut prev_p = p;
    let mut least_cost = MAX_HALFWORD;
    let mut best_place = TEX_NULL;
    let mut prev_dp: i32 = 0;
    for k in 1..=6usize {
        set_active_width(k, 0);
    }

    enum Next { NotFound, UpdateHeights, CheckPi(i32) }

    'done: loop {
        let next = if p == TEX_NULL {
            Next::CheckPi(EJECT_PENALTY)
        } else {
            match mem_b16s1(p) {
                0 | 1 | 2 => {
                    set_active_width(1, active_width(1) + prev_dp + mem_b32s1(p + 3));
                    prev_dp = mem_b32s1(p + 2);
                    Next::NotFound
                }
                8 => {
                    let sub = mem_b16s0(p);
                    if sub == PIC_NODE || sub == PDF_NODE {
                        set_active_width(1, active_width(1) + prev_dp + mem_b32s1(p + 3));
                        prev_dp = mem_b32s1(p + 2);
                    }
                    Next::NotFound
                }
                10 => {  // GLUE_NODE
                    if mem_b16s1(prev_p) < MATH_NODE {  // is_non_discardable_node
                        Next::CheckPi(0)
                    } else {
                        Next::UpdateHeights
                    }
                }
                11 => {  // KERN_NODE
                    let link = mem_b32s1(p);
                    let t = if link == TEX_NULL { PENALTY_NODE } else { mem_b16s1(link) };
                    if t == GLUE_NODE {
                        Next::CheckPi(0)
                    } else {
                        Next::UpdateHeights
                    }
                }
                12 => Next::CheckPi(mem_b32s1(p + 1)),  // PENALTY_NODE
                3 | 4 => Next::NotFound,
                _ => crate::errors::confusion(c"vertbreak".as_ptr()),
            }
        };

        let do_update = match next {
            Next::NotFound => false,
            Next::UpdateHeights => true,
            Next::CheckPi(pi) => {
                if pi < INF_PENALTY {
                    let b = if active_width(1) < h {
                        if active_width(3) != 0 || active_width(4) != 0 || active_width(5) != 0 {
                            0
                        } else {
                            crate::helpers::badness(h - active_width(1), active_width(2))
                        }
                    } else if active_width(1) - h > active_width(6) {
                        MAX_HALFWORD
                    } else {
                        crate::helpers::badness(active_width(1) - h, active_width(6))
                    };
                    let b = if b < MAX_HALFWORD {
                        if pi <= EJECT_PENALTY {
                            pi
                        } else if b < INF_BAD {
                            b + pi
                        } else {
                            100_000
                        }
                    } else {
                        b
                    };
                    if b <= least_cost {
                        best_place = p;
                        least_cost = b;
                        set_best_height_plus_depth(active_width(1) + prev_dp);
                    }
                    if b == MAX_HALFWORD || pi <= EJECT_PENALTY {
                        break 'done;
                    }
                }
                // reached only when pi >= INF_PENALTY, or (pi < INF_PENALTY and did not break);
                // in the TEX_NULL case pi == EJECT_PENALTY so we always break above — p is valid.
                let nt = mem_b16s1(p);
                nt >= GLUE_NODE && nt <= KERN_NODE
            }
        };

        if do_update {
            let q = if mem_b16s1(p) == KERN_NODE {
                p
            } else {
                let q = mem_b32s0(p + 1);
                let qi = mem_b16s1(q) as usize;
                set_active_width(2 + qi, active_width(2 + qi) + mem_b32s1(q + 2));
                set_active_width(6, active_width(6) + mem_b32s1(q + 3));
                if mem_b16s0(q) != NORMAL && mem_b32s1(q + 3) != 0 {
                    error_here_with_diagnostic(
                        c"Infinite glue shrinkage found in box being split".as_ptr(),
                    );
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(4);
                    set_help_line(3, c"The box you are \\vsplitting contains some infinitely".as_ptr());
                    set_help_line(2, c"shrinkable glue, e.g., `\\vss' or `\\vskip 0pt minus 1fil'.".as_ptr());
                    set_help_line(1, c"Such glue doesn't belong there; but you can safely proceed,".as_ptr());
                    set_help_line(0, c"since the offensive shrinkability has been made finite.".as_ptr());
                    error();
                    let r = new_spec(q);
                    set_mem_b16s0(r, NORMAL);   // GLUE_SPEC_shrink_order(r) = NORMAL
                    crate::mem::delete_glue_ref(q);
                    set_mem_b32s0(p + 1, r);
                    r
                } else {
                    q
                }
            };
            set_active_width(1, active_width(1) + prev_dp + mem_b32s1(q + 1));
            prev_dp = 0;
        }

        // not_found:
        if prev_dp > d {
            set_active_width(1, active_width(1) + prev_dp - d);
            prev_dp = d;
        }
        prev_p = p;
        p = mem_b32s1(prev_p);
    }
    best_place
}

// translated from xetex-xetex0.c:7727 (vsplit)
#[no_mangle]
pub unsafe extern "C" fn vsplit(n: i32, h: i32) -> i32 {
    use crate::globals::{
        cur_mark, set_cur_mark, cur_ptr, disc_ptr, set_disc_ptr, sa_root, set_sa_root,
    };
    use crate::mem::TEX_NULL;

    const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
    const BOX_BASE: i32 = 2_255_144;      // xetex_format.h:211
    const DIMEN_BASE: i32 = 8_941_180;    // xetex_format.h:222
    const INT_BASE: i32 = 7_826_729;      // xetex_format.h:219
    const DIMEN_PAR_SPLIT_MAX_DEPTH: i32 = 6;   // xetex_format.h:128
    const INT_PAR_SAVING_VDISCARDS: i32 = 63;   // xetex_format.h:98
    const BOX_NODE_SIZE: i32 = 8;        // xetex-constants.h:132
    const EXACTLY: i16 = 0;             // xetex-constants.h:256
    const ADDITIONAL: i16 = 1;          // xetex-constants.h:264
    const VSPLIT_CODE: usize = 3;       // xetex_format.h:483
    const MARK_VAL: usize = 7;          // xetex-constants.h:300
    const VLIST_NODE: u16 = 1;          // xetex_format.h:381
    const MARK_NODE: u16 = 4;           // xetex_format.h:384
    const SPLIT_FIRST_MARK_CODE: usize = 3;  // xetex_format.h:688
    const SPLIT_BOT_MARK_CODE: usize = 4;   // xetex_format.h:689

    set_cur_val(n);
    let v = if cur_val() < 256 {
        eqtb_b32s1(BOX_BASE + cur_val())
    } else {
        crate::save::find_sa_element(4, cur_val(), false);
        if cur_ptr() == TEX_NULL {
            TEX_NULL
        } else {
            mem_b32s1(cur_ptr() + 1)
        }
    };

    flush_node_list(disc_ptr(VSPLIT_CODE));
    set_disc_ptr(VSPLIT_CODE, TEX_NULL);

    if sa_root(MARK_VAL) != TEX_NULL {
        if do_marks(0, 0, sa_root(MARK_VAL)) {
            set_sa_root(MARK_VAL, TEX_NULL);
        }
    }

    if cur_mark(SPLIT_FIRST_MARK_CODE) != TEX_NULL {
        crate::mem::delete_token_ref(cur_mark(SPLIT_FIRST_MARK_CODE));
        set_cur_mark(SPLIT_FIRST_MARK_CODE, TEX_NULL);
        crate::mem::delete_token_ref(cur_mark(SPLIT_BOT_MARK_CODE));
        set_cur_mark(SPLIT_BOT_MARK_CODE, TEX_NULL);
    }

    if v == TEX_NULL {
        return TEX_NULL;
    }

    if mem_b16s1(v) != VLIST_NODE {
        error_here_with_diagnostic(c"".as_ptr());
        crate::output::print_esc_cstr(c"vsplit".as_ptr());
        crate::output::print_cstr(c" needs a ".as_ptr());
        crate::output::print_esc_cstr(c"vbox".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, c"The box you are trying to split is an \\hbox.".as_ptr());
        set_help_line(0, c"I can't split such a box, so I'll leave it alone.".as_ptr());
        error();
        return TEX_NULL;
    }

    let split_max_d = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_SPLIT_MAX_DEPTH);
    let q = vert_break(mem_b32s1(v + 5), h, split_max_d);
    let mut p = mem_b32s1(v + 5);

    if p == q {
        set_mem_b32s1(v + 5, TEX_NULL);
    } else {
        loop {
            if mem_b16s1(p) == MARK_NODE {
                let mark_class = mem_b32s0(p + 1);
                let mark_ptr = mem_b32s1(p + 1);
                if mark_class != 0 {
                    crate::save::find_sa_element(MARK_VAL as u16, mark_class, true);
                    if mem_b32s1(cur_ptr() + 2) == TEX_NULL {
                        set_mem_b32s1(cur_ptr() + 2, mark_ptr);
                        set_mem_b32s0(mark_ptr, mem_b32s0(mark_ptr) + 1);
                    } else {
                        crate::mem::delete_token_ref(mem_b32s0(cur_ptr() + 3));
                    }
                    set_mem_b32s0(cur_ptr() + 3, mark_ptr);
                    set_mem_b32s0(mark_ptr, mem_b32s0(mark_ptr) + 1);
                } else if cur_mark(SPLIT_FIRST_MARK_CODE) == TEX_NULL {
                    set_cur_mark(SPLIT_FIRST_MARK_CODE, mark_ptr);
                    set_cur_mark(SPLIT_BOT_MARK_CODE, mark_ptr);
                    set_mem_b32s0(mark_ptr, mem_b32s0(mark_ptr) + 2);
                } else {
                    crate::mem::delete_token_ref(cur_mark(SPLIT_BOT_MARK_CODE));
                    set_cur_mark(SPLIT_BOT_MARK_CODE, mark_ptr);
                    set_mem_b32s0(mark_ptr, mem_b32s0(mark_ptr) + 1);
                }
            }
            if mem_b32s1(p) == q {
                set_mem_b32s1(p, TEX_NULL);
                break;
            }
            p = mem_b32s1(p);
        }
    }

    let mut q = prune_page_top(q, eqtb_b32s1(INT_BASE + INT_PAR_SAVING_VDISCARDS) > 0);
    p = mem_b32s1(v + 5);
    crate::mem::free_node(v, BOX_NODE_SIZE);
    if q != TEX_NULL {
        q = vpackage(q, 0, ADDITIONAL, MAX_HALFWORD);
    }
    if cur_val() < 256 {
        set_eqtb_b32s1(BOX_BASE + cur_val(), q);
    } else {
        crate::save::find_sa_element(4, cur_val(), false);
        if cur_ptr() != TEX_NULL {
            set_mem_b32s1(cur_ptr() + 1, q);
            set_mem_b32s0(cur_ptr() + 1, mem_b32s0(cur_ptr() + 1) + 1);
            crate::save::delete_sa_ref(cur_ptr());
        }
    }
    vpackage(p, h, EXACTLY, split_max_d)
}

// translated from xetex-xetex0.c:7797 (box_end)
#[no_mangle]
pub unsafe extern "C" fn box_end(box_context: i32) {
    use crate::globals::{
        adjust_tail, cur_box, cur_list_aux_b32s0, cur_list_mode, cur_list_tail, cur_ptr,
        pre_adjust_tail, set_adjust_tail, set_cur_box, set_cur_list_aux_b32s0,
        set_cur_list_tail, set_cur_val, set_pre_adjust_tail,
    };
    use crate::mem::TEX_NULL;

    const VMODE: i16 = 1;
    const HMODE: i16 = 104;
    const BOX_FLAG: i32 = 0x4000_0000;         // xetex-constants.h:372
    const GLOBAL_BOX_FLAG: i32 = 0x4000_8000;  // xetex-constants.h:373
    const SHIP_OUT_FLAG: i32 = 0x4001_0000;    // xetex-constants.h:374
    const LEADER_FLAG: i32 = 0x4001_0001;      // xetex-constants.h:375
    const A_LEADERS: i32 = 100;               // xetex_format.h:422
    const ADJUST_HEAD: i32 = 4_999_994;       // MEM_TOP - 5 (xetex-constants.h:42)
    const PRE_ADJUST_HEAD: i32 = 4_999_985;   // MEM_TOP - 14 (xetex-constants.h:51)
    const SUB_BOX: i32 = 2;                   // xetex-constants.h:283
    const BOX_BASE: i32 = 2_255_144;          // xetex_format.h:211
    const BOX_REF: u16 = 121;                 // xetex_format.h:363
    const SPACER: u8 = 10;                    // xetex_format.h:242
    const RELAX: u8 = 0;                      // xetex_format.h:229
    const HSKIP: u8 = 26;                     // xetex_format.h:264
    const VSKIP: u8 = 27;                     // xetex_format.h:265

    if box_context < BOX_FLAG {
        if cur_box() != TEX_NULL {
            set_mem_b32s1(cur_box() + 4, box_context);

            if cur_list_mode().abs() == VMODE {
                if pre_adjust_tail() != TEX_NULL {
                    if PRE_ADJUST_HEAD != pre_adjust_tail() {
                        set_mem_b32s1(cur_list_tail(), mem_b32s1(PRE_ADJUST_HEAD));
                        set_cur_list_tail(pre_adjust_tail());
                    }
                    set_pre_adjust_tail(TEX_NULL);
                }

                append_to_vlist(cur_box());

                if adjust_tail() != TEX_NULL {
                    if ADJUST_HEAD != adjust_tail() {
                        set_mem_b32s1(cur_list_tail(), mem_b32s1(ADJUST_HEAD));
                        set_cur_list_tail(adjust_tail());
                    }
                    set_adjust_tail(TEX_NULL);
                }

                if cur_list_mode() > 0 {
                    build_page();
                }
            } else {
                if cur_list_mode().abs() == HMODE {
                    set_cur_list_aux_b32s0(1000);  // space_factor
                } else {
                    let p = crate::mem::new_noad();
                    set_mem_b32s1(p + 1, SUB_BOX);
                    set_mem_b32s0(p + 1, cur_box());
                    set_cur_box(p);
                }

                set_mem_b32s1(cur_list_tail(), cur_box());
                set_cur_list_tail(cur_box());
            }
        }
    } else if box_context < SHIP_OUT_FLAG {
        let (val, a) = if box_context < GLOBAL_BOX_FLAG {
            (box_context - BOX_FLAG, 0i16)
        } else {
            (box_context - GLOBAL_BOX_FLAG, 4i16)
        };
        set_cur_val(val);

        if cur_val() < 256 {
            if a >= 4 {
                crate::save::geq_define(BOX_BASE + cur_val(), BOX_REF, cur_box());
            } else {
                crate::save::eq_define(BOX_BASE + cur_val(), BOX_REF, cur_box());
            }
        } else {
            crate::save::find_sa_element(4, cur_val(), true);
            if a >= 4 {
                crate::save::gsa_def(cur_ptr(), cur_box());
            } else {
                crate::save::sa_def(cur_ptr(), cur_box());
            }
        }
    } else if cur_box() != TEX_NULL {
        if box_context > SHIP_OUT_FLAG {
            loop {
                get_x_token();
                if cur_cmd() != SPACER && cur_cmd() != RELAX {
                    break;
                }
            }

            if (cur_cmd() == HSKIP && cur_list_mode().abs() != VMODE)
                || (cur_cmd() == VSKIP && cur_list_mode().abs() == VMODE)
            {
                append_glue();
                set_mem_b16s0(cur_list_tail(), (box_context - (LEADER_FLAG - A_LEADERS)) as u16);
                set_mem_b32s1(cur_list_tail() + 1, cur_box());
            } else {
                error_here_with_diagnostic(c"Leaders not followed by proper glue".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(3);
                set_help_line(2, c"You should say `\\leaders <box or rule><hskip or vskip>'.".as_ptr());
                set_help_line(1, c"I found the <box or rule>, but there's no suitable".as_ptr());
                set_help_line(0, c"<hskip or vskip>, so I'm ignoring these leaders.".as_ptr());
                crate::input::back_error();
                flush_node_list(cur_box());
            }
        } else {
            ship_out(cur_box());
        }
    }
}

// translated from xetex-xetex0.c:7794 (begin_box)
#[no_mangle]
pub unsafe extern "C" fn begin_box(box_context: i32) {
    use crate::globals::{
        cur_box, cur_list_aux_b32s0, cur_list_aux_b32s1, cur_list_head, cur_list_mode,
        cur_list_tail, cur_ptr, hi_mem_min, set_cur_box, set_cur_list_aux_b32s0,
        set_cur_list_aux_b32s1, set_cur_list_mode, set_cur_list_tail, set_cur_val,
        set_save_stack_b32s1,
    };
    use crate::mem::TEX_NULL;

    const BOX_CODE: i32 = 0;           // xetex_format.h:480
    const COPY_CODE: i32 = 1;          // xetex_format.h:481
    const LAST_BOX_CODE: i32 = 2;      // xetex_format.h:482
    const VSPLIT_CODE_I: i32 = 3;      // xetex_format.h:483
    const BOX_FLAG: i32 = 0x4000_0000; // xetex-constants.h:372
    const HLIST_NODE: u16 = 0;         // xetex_format.h:380
    const VLIST_NODE: u16 = 1;         // xetex_format.h:381
    const DISC_NODE: u16 = 7;          // xetex_format.h:387
    const MATH_NODE: u16 = 9;          // xetex_format.h:389
    const MMODE: i16 = 207;            // xetex_format.h:370
    const VMODE: i16 = 1;              // xetex_format.h:368
    const HMODE: i16 = 104;            // xetex_format.h:369
    const BEGIN_M_CODE: u16 = 2;       // xetex_format.h:436
    const END_M_CODE: u16 = 3;         // xetex_format.h:437
    const HBOX_GROUP: u8 = 2;          // xetex-constants.h:154
    const ADJUSTED_HBOX_GROUP: u8 = 3; // xetex-constants.h:155
    const VBOX_GROUP: u8 = 4;          // xetex-constants.h:156
    const VTOP_GROUP: u8 = 5;          // xetex-constants.h:157
    const EVERY_HBOX_TEXT: u16 = 11;   // xetex-constants.h:210
    const EVERY_VBOX_TEXT: u16 = 12;   // xetex-constants.h:211
    const IGNORE_DEPTH: i32 = -65_536_000; // xetex-constants.h:388
    const BOX_BASE: i32 = 2_255_144;   // xetex_format.h:211
    const LOCAL_BASE: i32 = 2_254_871; // xetex_format.h:208
    const LOCAL_EVERY_HBOX: i32 = 5;   // xetex_format.h:177
    const LOCAL_EVERY_VBOX: i32 = 6;   // xetex_format.h:178

    match cur_chr() as i32 {
        BOX_CODE => {
            scan_register_num();
            let v = cur_val();
            if v < 256 {
                set_cur_box(eqtb_b32s1(BOX_BASE + v));
            } else {
                crate::save::find_sa_element(4, v, false);
                set_cur_box(if cur_ptr() == TEX_NULL { TEX_NULL } else { mem_b32s1(cur_ptr() + 1) });
            }
            if v < 256 {
                set_eqtb_b32s1(BOX_BASE + v, TEX_NULL);
            } else {
                crate::save::find_sa_element(4, v, false);
                if cur_ptr() != TEX_NULL {
                    set_mem_b32s1(cur_ptr() + 1, TEX_NULL);
                    set_mem_b32s0(cur_ptr() + 1, mem_b32s0(cur_ptr() + 1) + 1);
                    crate::save::delete_sa_ref(cur_ptr());
                }
            }
        }

        COPY_CODE => {
            scan_register_num();
            let v = cur_val();
            let q = if v < 256 {
                eqtb_b32s1(BOX_BASE + v)
            } else {
                crate::save::find_sa_element(4, v, false);
                if cur_ptr() == TEX_NULL { TEX_NULL } else { mem_b32s1(cur_ptr() + 1) }
            };
            set_cur_box(copy_node_list(q));
        }

        LAST_BOX_CODE => {
            set_cur_box(TEX_NULL);

            if cur_list_mode().abs() == MMODE {
                you_cant();
                set_help_ptr(1);
                set_help_line(0, c"Sorry; this \\lastbox will be void.".as_ptr());
                error();
            } else if cur_list_mode() == VMODE && cur_list_head() == cur_list_tail() {
                you_cant();
                set_help_ptr(2);
                set_help_line(1, c"Sorry...I usually can't take things from the current page.".as_ptr());
                set_help_line(0, c"This \\lastbox will therefore be void.".as_ptr());
                error();
            } else {
                let mut tx = cur_list_tail();
                if tx < hi_mem_min() {
                    if mem_b16s1(tx) == MATH_NODE && mem_b16s0(tx) == END_M_CODE {
                        let mut r = cur_list_head();
                        loop {
                            let prev = r;
                            r = mem_b32s1(prev);
                            if r == tx { tx = prev; break; }
                        }
                    }
                }
                'last_box_done: {
                    if tx < hi_mem_min() {
                        let nt = mem_b16s1(tx);
                        if nt == HLIST_NODE || nt == VLIST_NODE {
                            let mut q = cur_list_head();
                            let mut p = TEX_NULL;
                            let mut r = TEX_NULL;
                            let mut fm = false;
                            loop {
                                r = p;
                                p = q;
                                fm = false;
                                if q < hi_mem_min() {
                                    if mem_b16s1(q) == DISC_NODE {
                                        for _ in 1..=mem_b16s0(q) {
                                            p = mem_b32s1(p);
                                        }
                                        if p == tx { break 'last_box_done; }
                                    } else if mem_b16s1(q) == MATH_NODE && mem_b16s0(q) == BEGIN_M_CODE {
                                        fm = true;
                                    }
                                }
                                q = mem_b32s1(p);
                                if q == tx { break; }
                            }
                            let q2 = mem_b32s1(tx);
                            set_mem_b32s1(p, q2);
                            set_mem_b32s1(tx, TEX_NULL);
                            if q2 == TEX_NULL {
                                if fm {
                                    crate::errors::confusion(c"tail1".as_ptr());
                                } else {
                                    set_cur_list_tail(p);
                                }
                            } else if fm {
                                set_cur_list_tail(r);
                                set_mem_b32s1(r, TEX_NULL);
                                flush_node_list(p);
                            }
                            set_cur_box(tx);
                            set_mem_b32s1(cur_box() + 4, 0);
                        }
                    }
                }
            }
        }

        VSPLIT_CODE_I => {
            scan_register_num();
            let n = cur_val();
            if !scan_keyword(c"to".as_ptr()) {
                error_here_with_diagnostic(c"Missing `to' inserted".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(1, c"I'm working on `\\vsplit<box number> to <dimen>';".as_ptr());
                set_help_line(0, c"will look for the <dimen> next.".as_ptr());
                error();
            }
            scan_dimen(false, false, false);
            set_cur_box(vsplit(n, cur_val()));
        }

        _ => {
            let k = cur_chr() as i32 - 4;
            set_save_stack_b32s1(save_ptr(), box_context);
            if k == HMODE as i32 {
                if box_context < BOX_FLAG && cur_list_mode().abs() == VMODE {
                    scan_spec(ADJUSTED_HBOX_GROUP, true);
                } else {
                    scan_spec(HBOX_GROUP, true);
                }
            } else {
                if k == VMODE as i32 {
                    scan_spec(VBOX_GROUP, true);
                } else {
                    scan_spec(VTOP_GROUP, true);
                    // k = VMODE for the nest push below
                }
                normal_paragraph();
            }
            crate::mem::push_nest();
            // If k was adjusted to VTOP_GROUP branch: k was neither HMODE nor VMODE,
            // so the else branch above ran and after scan_spec(VTOP_GROUP), we want k=VMODE.
            let k_mode = if k != HMODE as i32 && k != VMODE as i32 { VMODE as i32 } else { k };
            set_cur_list_mode(-(k_mode as i16));
            if k_mode == VMODE as i32 {
                set_cur_list_aux_b32s1(IGNORE_DEPTH);
                let every_vbox = eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_VBOX);
                if every_vbox != TEX_NULL {
                    begin_token_list(every_vbox, EVERY_VBOX_TEXT);
                }
            } else {
                set_cur_list_aux_b32s0(1000);
                let every_hbox = eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_HBOX);
                if every_hbox != TEX_NULL {
                    begin_token_list(every_hbox, EVERY_HBOX_TEXT);
                }
            }
            return;  // no box_end call for new-group cases
        }
    }

    box_end(box_context);
}

// translated from xetex-xetex0.c:8029 (just_copy)
#[no_mangle]
pub unsafe extern "C" fn just_copy(p: i32, h: i32, t: i32) {
    use crate::globals::hi_mem_min;
    use crate::mem::TEX_NULL;

    extern "C" { fn new_edge(s: i16, w: i32) -> i32; }

    const BOX_NODE_SIZE: i32 = 8;           // xetex-constants.h:132
    const RULE_NODE_SIZE: i32 = 5;          // xetex-constants.h:128
    const MEDIUM_NODE_SIZE: i32 = 3;        // xetex-constants.h:116
    const SMALL_NODE_SIZE: i32 = 2;         // xetex-constants.h:110
    const OPEN_NODE_SIZE: i32 = 3;          // xetex-constants.h:118
    const WRITE_NODE_SIZE: i32 = 2;         // xetex-constants.h:112
    const GLYPH_NODE_SIZE: i32 = 5;         // xetex-constants.h:126
    const PIC_NODE_SIZE: i32 = 9;           // xetex-constants.h:133
    const MEM_WORD_BYTES: i32 = 8;          // sizeof(memory_word)
    const SYNCTEX_FIELD_SIZE: i32 = 1;      // xetex-constants.h:102
    const HLIST_NODE: u16 = 0;
    const VLIST_NODE: u16 = 1;
    const RULE_NODE: u16 = 2;
    const LIGATURE_NODE: u16 = 6;
    const KERN_NODE: u16 = 11;
    const MATH_NODE: u16 = 9;
    const GLUE_NODE: u16 = 10;
    const WHATSIT_NODE: u16 = 8;
    const OPEN_NODE: u16 = 0;
    const WRITE_NODE: u16 = 1;
    const CLOSE_NODE: u16 = 2;
    const SPECIAL_NODE: u16 = 3;
    const LANGUAGE_NODE: u16 = 4;
    const NATIVE_WORD_NODE: u16 = 40;
    const NATIVE_WORD_NODE_AT: u16 = 41;
    const GLYPH_NODE: u16 = 42;
    const PIC_NODE: u16 = 43;
    const PDF_NODE: u16 = 44;
    const PDF_SAVE_POS_NODE: u16 = 21;

    let mut p = p;
    let mut h = h;

    while p != TEX_NULL {
        let mut words: i32 = 1;

        // Determine r and words for this node. None = skip linking (goto not_found).
        let r_opt: Option<i32> = if p >= hi_mem_min() {
            // char node
            Some(crate::mem::get_avail())
        } else {
            match mem_b16s1(p) {
                HLIST_NODE | VLIST_NODE => {
                    let r = crate::mem::get_node(BOX_NODE_SIZE);
                    // SYNCTEX_tag(r, BOX_NODE_SIZE) = SYNCTEX_tag(p, BOX_NODE_SIZE)
                    set_mem_b32s0(r + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                                  mem_b32s0(p + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE));
                    // SYNCTEX_line(r, BOX_NODE_SIZE) = SYNCTEX_line(p, BOX_NODE_SIZE)
                    set_mem_b32s1(r + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                                  mem_b32s1(p + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE));
                    set_mem_word(r + 6, mem_word(p + 6));
                    set_mem_word(r + 5, mem_word(p + 5));
                    words = 5;
                    set_mem_b32s1(r + 5, TEX_NULL);  // BOX_list_ptr(r) = TEX_NULL
                    Some(r)
                }
                RULE_NODE => {
                    words = RULE_NODE_SIZE;
                    Some(crate::mem::get_node(RULE_NODE_SIZE))
                }
                LIGATURE_NODE => {
                    let r = crate::mem::get_avail();
                    set_mem_word(r, mem_word(p + 1));  // mem[r] = mem[p + 1]
                    words = 0;  // skip the copy loop (goto found)
                    Some(r)
                }
                KERN_NODE | MATH_NODE => {
                    words = MEDIUM_NODE_SIZE;
                    Some(crate::mem::get_node(MEDIUM_NODE_SIZE))
                }
                GLUE_NODE => {
                    let r = crate::mem::get_node(MEDIUM_NODE_SIZE);
                    // GLUE_SPEC_ref_count(GLUE_NODE_glue_ptr(p))++
                    let gp = mem_b32s0(p + 1);  // GLUE_NODE_glue_ptr(p)
                    set_mem_b32s1(gp, mem_b32s1(gp) + 1);  // GLUE_SPEC_ref_count(gp)++
                    // SYNCTEX fields
                    set_mem_b32s0(r + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                                  mem_b32s0(p + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE));
                    set_mem_b32s1(r + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                                  mem_b32s1(p + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE));
                    set_mem_b32s0(r + 1, gp);       // GLUE_NODE_glue_ptr(r) = gp
                    set_mem_b32s1(r + 1, TEX_NULL); // GLUE_NODE_leader_ptr(r) = TEX_NULL
                    Some(r)
                }
                WHATSIT_NODE => {
                    match mem_b16s0(p) {
                        OPEN_NODE => {
                            words = OPEN_NODE_SIZE;
                            Some(crate::mem::get_node(OPEN_NODE_SIZE))
                        }
                        WRITE_NODE | SPECIAL_NODE => {
                            let r = crate::mem::get_node(WRITE_NODE_SIZE);
                            // TOKEN_LIST_ref_count(WRITE_NODE_tokens(p))++
                            let toks = mem_b32s1(p + 1);  // WRITE_NODE_tokens(p)
                            set_mem_b32s0(toks, mem_b32s0(toks) + 1);
                            words = WRITE_NODE_SIZE;
                            Some(r)
                        }
                        CLOSE_NODE | LANGUAGE_NODE => {
                            words = SMALL_NODE_SIZE;
                            Some(crate::mem::get_node(SMALL_NODE_SIZE))
                        }
                        NATIVE_WORD_NODE | NATIVE_WORD_NODE_AT => {
                            let sz = mem_b16s3(p + 4) as i32;  // NATIVE_NODE_size(p)
                            let r = crate::mem::get_node(sz);
                            let mut w = sz;
                            while w > 0 {
                                w -= 1;
                                set_mem_word(r + w, mem_word(p + w));
                            }
                            // NATIVE_NODE_glyph_info_ptr(r) = NULL
                            set_mem_ptr(r + 5, std::ptr::null_mut());
                            // NATIVE_NODE_glyph_count(r) = 0
                            set_mem_b16s0(r + 4, 0);
                            crate::mem::copy_native_glyph_info(p, r);
                            words = 0;  // already copied, skip outer loop
                            Some(r)
                        }
                        GLYPH_NODE => {
                            words = GLYPH_NODE_SIZE;
                            Some(crate::mem::get_node(GLYPH_NODE_SIZE))
                        }
                        PIC_NODE | PDF_NODE => {
                            let path_len = mem_b16s1(p + 4) as i32;  // PIC_NODE_path_len
                            words = PIC_NODE_SIZE + (path_len + MEM_WORD_BYTES - 1) / MEM_WORD_BYTES;
                            Some(crate::mem::get_node(words))
                        }
                        PDF_SAVE_POS_NODE => {
                            Some(crate::mem::get_node(SMALL_NODE_SIZE))
                        }
                        _ => {
                            crate::errors::confusion(c"ext2".as_ptr());
                        }
                    }
                }
                _ => None, // goto not_found: skip copy and linking
            }
        };

        if let Some(r) = r_opt {
            // Copy words from p to r (skip if words==0)
            let mut w = words;
            while w > 0 {
                w -= 1;
                set_mem_word(r + w, mem_word(p + w));
            }
            // found: link r into h
            set_mem_b32s1(h, r);  // LLIST_link(h) = r
            h = r;
        }

        // not_found: advance p
        p = mem_b32s1(p);  // LLIST_link(p)
    }

    set_mem_b32s1(h, t);  // LLIST_link(h) = t
}

// translated from xetex-xetex0.c:8155 (just_reverse)
#[no_mangle]
pub unsafe extern "C" fn just_reverse(p: i32) {
    use crate::globals::{
        avail, cur_dir, lr_problems, lr_ptr, set_avail, set_cur_dir, set_lr_problems, set_lr_ptr,
        set_temp_ptr, temp_ptr,
    };
    use crate::mem::TEX_NULL;

    extern "C" { fn new_edge(s: i16, w: i32) -> i32; }

    const MEDIUM_NODE_SIZE: i32 = 3;
    const TEMP_HEAD: i32 = 4_999_996;   // MEM_TOP - 3
    const MIN_HALFWORD: i32 = -0x0FFF_FFFF;
    const L_CODE: i32 = 4;              // xetex-constants.h:142
    const R_CODE: i32 = 8;              // xetex-constants.h:143
    const MATH_NODE: u16 = 9;
    const KERN_NODE: u16 = 11;

    let mut p = p;
    let mut m = MIN_HALFWORD;
    let mut n = MIN_HALFWORD;

    let mut q = if mem_b32s1(TEMP_HEAD) == TEX_NULL {
        just_copy(mem_b32s1(p), TEMP_HEAD, TEX_NULL);
        mem_b32s1(TEMP_HEAD)
    } else {
        let q = mem_b32s1(p);
        set_mem_b32s1(p, TEX_NULL);
        flush_node_list(mem_b32s1(TEMP_HEAD));
        q
    };

    let t = new_edge(cur_dir(), 0);
    let mut l = t;
    set_cur_dir(1 - cur_dir());

    'done: loop {
        while q != TEX_NULL {
            if q >= crate::globals::hi_mem_min() {
                // char node do-while
                loop {
                    p = q;
                    q = mem_b32s1(p);
                    set_mem_b32s1(p, l);
                    l = p;
                    if q < crate::globals::hi_mem_min() { break; }
                }
            } else {
                p = q;
                q = mem_b32s1(p);

                if mem_b16s1(p) == MATH_NODE {
                    let subtype = mem_b16s0(p) as i32;
                    if subtype & 1 != 0 {
                        // odd subtype = end math node
                        let expected = L_CODE * (subtype / L_CODE) + 3;
                        if mem_b32s0(lr_ptr()) != expected {
                            set_mem_b16s1(p, KERN_NODE);  // NODE_type(p) = KERN_NODE
                            set_lr_problems(lr_problems() + 1);
                        } else {
                            // pop LR_ptr
                            set_temp_ptr(lr_ptr());
                            set_lr_ptr(mem_b32s1(temp_ptr()));
                            set_mem_b32s1(temp_ptr(), avail());
                            set_avail(temp_ptr());

                            if n > MIN_HALFWORD {
                                n -= 1;
                                set_mem_b16s0(p, mem_b16s0(p) - 1);
                            } else if m > MIN_HALFWORD {
                                m -= 1;
                                set_mem_b16s1(p, KERN_NODE);
                            } else {
                                // found edge end: set t's link and free p
                                set_mem_b32s1(t + 1, mem_b32s1(p + 1));
                                set_mem_b32s1(t, q);
                                crate::mem::free_node(p, MEDIUM_NODE_SIZE);
                                break 'done;
                            }
                        }
                    } else {
                        // even subtype = begin math node
                        // push LR_ptr
                        set_temp_ptr(crate::mem::get_avail());
                        set_mem_b32s0(temp_ptr(), L_CODE * (subtype / L_CODE) + 3);
                        set_mem_b32s1(temp_ptr(), lr_ptr());
                        set_lr_ptr(temp_ptr());

                        if n > MIN_HALFWORD || (subtype / R_CODE) != cur_dir() as i32 {
                            n += 1;
                            set_mem_b16s0(p, mem_b16s0(p) + 1);
                        } else {
                            set_mem_b16s1(p, KERN_NODE);
                            m += 1;
                        }
                    }
                }

                set_mem_b32s1(p, l);
                l = p;
            }
        }
        break; // goto done (past dead code)
    }

    // done:
    set_mem_b32s1(TEMP_HEAD, l);
}

// translated from xetex-xetex0.c:6968 (char_pw)
#[no_mangle]
pub unsafe extern "C" fn char_pw(p: i32, side: i16) -> i32 {
    use crate::globals::{
        font_info_b32s1, hi_mem_min, mem_b16s2, mem_node_addr, mem_ptr,
        param_base, set_last_leftmost_char, set_last_rightmost_char,
    };
    use crate::mem::TEX_NULL;

    const WHATSIT_NODE: u16 = 8;
    const NATIVE_WORD_NODE: u16 = 40;
    const NATIVE_WORD_NODE_AT: u16 = 41;
    const GLYPH_NODE: u16 = 42;
    const LIGATURE_NODE: u16 = 6;
    const QUAD_CODE: i32 = 6;   // xetex-constants.h:298

    if side == 0 {
        set_last_leftmost_char(TEX_NULL);
    } else {
        set_last_rightmost_char(TEX_NULL);
    }
    if p == TEX_NULL {
        return 0;
    }

    // NATIVE_WORD_NODE or NATIVE_WORD_NODE_AT whatsit
    if p != TEX_NULL
        && p < hi_mem_min()
        && mem_b16s1(p) == WHATSIT_NODE
        && (mem_b16s0(p) == NATIVE_WORD_NODE || mem_b16s0(p) == NATIVE_WORD_NODE_AT)
    {
        if !mem_ptr(p + 5).is_null() {  // NATIVE_NODE_glyph_info_ptr != NULL
            let f = mem_b16s2(p + 4) as i32;  // NATIVE_NODE_font
            let cp = real_get_native_word_cp(
                mem_node_addr(p),
                side as libc::c_int,
            );
            return crate::scaled_math::round_xn_over_d(
                font_info_b32s1(QUAD_CODE + param_base(f)),
                cp,
                1000,
            );
        } else {
            return 0;
        }
    }

    // GLYPH_NODE whatsit
    if p != TEX_NULL
        && p < hi_mem_min()
        && mem_b16s1(p) == WHATSIT_NODE
        && mem_b16s0(p) == GLYPH_NODE
    {
        let f = mem_b16s2(p + 4) as i32;  // mem[p + 4].b16.s2
        let code = mem_b16s1(p + 4) as u32; // mem[p + 4].b16.s1 (char code)
        return crate::scaled_math::round_xn_over_d(
            font_info_b32s1(QUAD_CODE + param_base(f)),
            get_cp_code(f, code, side as i32),
            1000,
        );
    }

    // Adjust p for ligature node or non-char node
    let p = if p < hi_mem_min() {
        if mem_b16s1(p) == LIGATURE_NODE {
            p + 1  // LIGATURE_NODE: the char is at p+1
        } else {
            return 0;
        }
    } else {
        p
    };

    let f = mem_b16s1(p) as i32;  // CHAR_NODE_font(p) = mem[p].b16.s1
    let c = get_cp_code(f, mem_b16s0(p) as u32, side as i32);
    if side == 0 {
        set_last_leftmost_char(p);
    } else {
        set_last_rightmost_char(p);
    }
    if c == 0 {
        return 0;
    }
    crate::scaled_math::round_xn_over_d(font_info_b32s1(QUAD_CODE + param_base(f)), c, 1000)
}

// translated from xetex-xetex0.c:8265 (handle_right_brace)
#[no_mangle]
pub unsafe extern "C" fn handle_right_brace() {
    use crate::globals::{
        adjust_tail, cur_group, cur_input, cur_list_head, cur_list_tail, disc_ptr, insert_penalties,
        nest_ptr, nest_tail, output_active, page_tail, pre_adjust_tail, save_ptr, save_stack_b32s1,
        set_adjust_tail, set_cur_list_tail, set_cur_tok, set_disc_ptr, set_insert_penalties,
        set_mem_b16s0, set_mem_b16s1, set_nest_tail, set_output_active, set_page_tail,
        set_pre_adjust_tail, set_save_ptr,
    };
    use crate::mem::TEX_NULL;

    const BOTTOM_LEVEL: u8 = 0;          // xetex-constants.h:152
    const SIMPLE_GROUP: u8 = 1;
    const HBOX_GROUP: u8 = 2;
    const ADJUSTED_HBOX_GROUP: u8 = 3;
    const VBOX_GROUP: u8 = 4;
    const VTOP_GROUP: u8 = 5;
    const ALIGN_GROUP: u8 = 6;
    const NO_ALIGN_GROUP: u8 = 7;
    const OUTPUT_GROUP: u8 = 8;
    const MATH_GROUP: u8 = 9;
    const DISC_GROUP: u8 = 10;
    const INSERT_GROUP: u8 = 11;
    const VCENTER_GROUP: u8 = 12;
    const MATH_CHOICE_GROUP: u8 = 13;
    const SEMI_SIMPLE_GROUP: u8 = 14;
    const MATH_SHIFT_GROUP: u8 = 15;
    const MATH_LEFT_GROUP: u8 = 16;

    const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
    const BOX_NODE_SIZE: i32 = 8;
    const INS_NODE_SIZE: i32 = 5;
    const SMALL_NODE_SIZE: i32 = 2;
    const NOAD_SIZE: i32 = 4;
    const ADDITIONAL: i16 = 1;
    const VTOP_CODE: i16 = 4;           // xetex_format.h:484
    const ADJUST_HEAD: i32 = 4_999_994; // MEM_TOP - 5
    const PRE_ADJUST_HEAD: i32 = 4_999_985; // MEM_TOP - 14
    const CONTRIB_HEAD: i32 = 4_999_998; // MEM_TOP - 1
    const PAGE_HEAD: i32 = 4_999_997;   // MEM_TOP - 2
    const LAST_BOX_CODE: usize = 2;     // xetex_format.h:482
    const OUTPUT_TEXT: u16 = 7;         // xetex-constants.h:206
    const BACKED_UP: u16 = 3;           // xetex-constants.h:202
    const BOX_BASE: i32 = 2_255_144;    // xetex_format.h:211
    const INT_BASE: i32 = 7_826_729;
    const DIMEN_BASE: i32 = 8_941_180;
    const GLUE_BASE: i32 = 2_254_340;   // xetex_format.h:205
    const DIMEN_PAR_SPLIT_MAX_DEPTH: i32 = 6;      // xetex_format.h:128
    const INT_PAR_FLOATING_PENALTY: i32 = 42;      // xetex_format.h:77
    const GLUE_PAR_SPLIT_TOP_SKIP: i32 = 10;       // xetex_format.h:159
    const INS_NODE: u16 = 3;            // xetex_format.h:383
    const ADJUST_NODE: u16 = 5;         // xetex_format.h:385
    const VCENTER_NOAD: u16 = 29;       // xetex_format.h:414
    const ORD_NOAD: u16 = 16;           // xetex_format.h:401
    const ACCENT_NOAD: u16 = 28;        // xetex_format.h:413
    const SUB_BOX: i32 = 2;             // xetex-constants.h:283
    const SUB_MLIST: i32 = 3;           // xetex-constants.h:287
    const EMPTY: i32 = 0;               // xetex-constants.h:230
    const CS_TOKEN_FLAG: i32 = 0x1FF_FFFF; // xetex-constants.h:348
    const FROZEN_CR: i32 = 2_243_226 + 1; // FROZEN_CONTROL_SEQUENCE + 1

    match cur_group() {
        SIMPLE_GROUP => {
            crate::save::unsave();
        }

        BOTTOM_LEVEL => {
            error_here_with_diagnostic(c"Too many }'s".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(2);
            set_help_line(1, c"You've closed more groups than you opened.".as_ptr());
            set_help_line(0, c"Such booboos are generally harmless, so keep going.".as_ptr());
            error();
        }

        SEMI_SIMPLE_GROUP | MATH_SHIFT_GROUP | MATH_LEFT_GROUP => {
            extra_right_brace();
        }

        HBOX_GROUP => {
            package(0);
        }

        ADJUSTED_HBOX_GROUP => {
            set_adjust_tail(ADJUST_HEAD);
            set_pre_adjust_tail(PRE_ADJUST_HEAD);
            package(0);
        }

        VBOX_GROUP => {
            end_graf();
            package(0);
        }

        VTOP_GROUP => {
            end_graf();
            package(VTOP_CODE);
        }

        INSERT_GROUP => {
            end_graf();
            // GLUEPAR(split_top_skip)
            let q = eqtb_b32s1(GLUE_BASE + GLUE_PAR_SPLIT_TOP_SKIP);
            // GLUE_SPEC_ref_count(q)++
            set_mem_b32s1(q, mem_b32s1(q) + 1);
            let d = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_SPLIT_MAX_DEPTH);
            let f = eqtb_b32s1(INT_BASE + INT_PAR_FLOATING_PENALTY);
            crate::save::unsave();
            set_save_ptr(save_ptr() - 2);
            let p = vpackage(mem_b32s1(cur_list_head()), 0, ADDITIONAL, MAX_HALFWORD);
            crate::mem::pop_nest();

            if save_stack_b32s1(save_ptr()) < 255 {
                let new_node = crate::mem::get_node(INS_NODE_SIZE);
                set_mem_b32s1(cur_list_tail(), new_node);
                set_cur_list_tail(mem_b32s1(cur_list_tail()));
                set_mem_b16s1(cur_list_tail(), INS_NODE);
                set_mem_b16s0(cur_list_tail(), save_stack_b32s1(save_ptr()) as u16);
                set_mem_b32s1(cur_list_tail() + 3, mem_b32s1(p + 3) + mem_b32s1(p + 2));
                set_mem_b32s0(cur_list_tail() + 4, mem_b32s1(p + 5));  // ins list ptr
                set_mem_b32s1(cur_list_tail() + 4, q);
                set_mem_b32s1(cur_list_tail() + 2, d);
                set_mem_b32s1(cur_list_tail() + 1, f);
            } else {
                let new_node = crate::mem::get_node(SMALL_NODE_SIZE);
                set_mem_b32s1(cur_list_tail(), new_node);
                set_cur_list_tail(mem_b32s1(cur_list_tail()));
                set_mem_b16s1(cur_list_tail(), ADJUST_NODE);
                set_mem_b16s0(cur_list_tail(), save_stack_b32s1(save_ptr() + 1) as u16);
                set_mem_b32s1(cur_list_tail() + 1, mem_b32s1(p + 5));
                crate::mem::delete_glue_ref(q);
            }

            crate::mem::free_node(p, BOX_NODE_SIZE);
            if nest_ptr() == 0 {
                build_page();
            }
        }

        OUTPUT_GROUP => {
            if cur_input().loc != TEX_NULL
                || (cur_input().index != OUTPUT_TEXT && cur_input().index != BACKED_UP)
            {
                error_here_with_diagnostic(c"Unbalanced output routine".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(1, c"Your sneaky output routine has problematic {'s and/or }'s.".as_ptr());
                set_help_line(0, c"I can't handle that very well; good luck.".as_ptr());
                error();
                loop {
                    get_token();
                    if cur_input().loc == TEX_NULL { break; }
                }
            }

            crate::input::end_token_list();
            end_graf();
            crate::save::unsave();
            set_output_active(false);
            set_insert_penalties(0);

            if eqtb_b32s1(BOX_BASE + 255) != TEX_NULL {
                error_here_with_diagnostic(c"Output routine didn't use all of ".as_ptr());
                crate::output::print_esc_cstr(c"box".as_ptr());
                crate::output::print_int(255);
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(3);
                set_help_line(2, c"Your \\output commands should empty \\box255,".as_ptr());
                set_help_line(1, c"e.g., by saying `\\shipout\\box255'.".as_ptr());
                set_help_line(0, c"Proceed; I'll discard its present contents.".as_ptr());
                box_error(255);
            }

            if cur_list_tail() != cur_list_head() {
                set_mem_b32s1(page_tail(), mem_b32s1(cur_list_head()));
                set_page_tail(cur_list_tail());
            }

            if mem_b32s1(PAGE_HEAD) != TEX_NULL {
                if mem_b32s1(CONTRIB_HEAD) == TEX_NULL {
                    set_nest_tail(0, page_tail());
                }
                set_mem_b32s1(page_tail(), mem_b32s1(CONTRIB_HEAD));
                set_mem_b32s1(CONTRIB_HEAD, mem_b32s1(PAGE_HEAD));
                set_mem_b32s1(PAGE_HEAD, TEX_NULL);
                set_page_tail(PAGE_HEAD);
            }

            flush_node_list(disc_ptr(LAST_BOX_CODE));
            set_disc_ptr(LAST_BOX_CODE, TEX_NULL);
            crate::mem::pop_nest();
            build_page();
        }

        DISC_GROUP => {
            build_discretionary();
        }

        ALIGN_GROUP => {
            crate::input::back_input();
            set_cur_tok(CS_TOKEN_FLAG + FROZEN_CR);
            error_here_with_diagnostic(c"Missing ".as_ptr());
            crate::output::print_esc_cstr(c"cr".as_ptr());
            crate::output::print_cstr(c" inserted".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(1);
            set_help_line(0, c"I'm guessing that you meant to end an alignment here.".as_ptr());
            crate::input::ins_error();
        }

        NO_ALIGN_GROUP => {
            end_graf();
            crate::save::unsave();
            align_peek();
        }

        VCENTER_GROUP => {
            end_graf();
            crate::save::unsave();
            set_save_ptr(save_ptr() - 2);
            let p = vpackage(
                mem_b32s1(cur_list_head()),
                save_stack_b32s1(save_ptr() + 1),
                save_stack_b32s1(save_ptr()) as i16,
                MAX_HALFWORD,
            );
            crate::mem::pop_nest();
            let new_noad = crate::mem::new_noad();
            set_mem_b32s1(cur_list_tail(), new_noad);
            set_cur_list_tail(mem_b32s1(cur_list_tail()));
            set_mem_b16s1(cur_list_tail(), VCENTER_NOAD);
            set_mem_b32s1(cur_list_tail() + 1, SUB_BOX);
            set_mem_b32s0(cur_list_tail() + 1, p);
        }

        MATH_CHOICE_GROUP => {
            build_choices();
        }

        MATH_GROUP => {
            crate::save::unsave();
            set_save_ptr(save_ptr() - 1);
            let save_loc = save_stack_b32s1(save_ptr());
            set_mem_b32s1(save_loc, SUB_MLIST);
            let p = fin_mlist(TEX_NULL);
            set_mem_b32s0(save_loc, p);

            if p != TEX_NULL && mem_b32s1(p) == TEX_NULL {
                if mem_b16s1(p) == ORD_NOAD
                    && mem_b32s1(p + 3) == EMPTY
                    && mem_b32s1(p + 2) == EMPTY
                {
                    // collapse to inline field
                    set_mem_b32s0(save_loc, mem_b32s0(p + 1));
                    set_mem_b32s1(save_loc, mem_b32s1(p + 1));
                    crate::mem::free_node(p, NOAD_SIZE);
                } else if mem_b16s1(p) == ACCENT_NOAD
                    && save_loc == cur_list_tail() + 1
                    && mem_b16s1(cur_list_tail()) == ORD_NOAD
                {
                    // merge accent noad into ord
                    let mut q = cur_list_head();
                    while mem_b32s1(q) != cur_list_tail() {
                        q = mem_b32s1(q);
                    }
                    set_mem_b32s1(q, p);
                    crate::mem::free_node(cur_list_tail(), NOAD_SIZE);
                    set_cur_list_tail(p);
                }
            }
        }

        _ => {
            crate::errors::confusion(c"rightbrace".as_ptr());
        }
    }
}

// ---------------------------------------------------------------------------
// close_files_and_terminate
// ---------------------------------------------------------------------------

// INT_BASE + INT_PAR__new_line_char (xetex_format.h:84).
const INT_PAR_NEW_LINE_CHAR_CFT: i32 = 49;
const INT_BASE_CFT: i32 = 7_826_729;
const SELECTOR_TERM_ONLY_CFT: i32 = 17;

/// translated from xetex-xetex0.c:9722 (close_files_and_terminate)
#[no_mangle]
pub unsafe extern "C" fn close_files_and_terminate() {
    use crate::globals::{log_file, log_opened, write_file};

    for k in 0usize..=15 {
        if write_open(k) {
            ttstub_output_close(write_file(k));
        }
    }

    set_eqtb_b32s1(INT_BASE_CFT + INT_PAR_NEW_LINE_CHAR_CFT, -1);

    finalize_dvi_file();
    synctex_terminate(log_opened());

    if log_opened() {
        ttstub_output_putc(log_file(), b'\n' as i32);
        ttstub_output_close(log_file());
        set_selector(selector() - 2);
        if selector() == SELECTOR_TERM_ONLY_CFT {
            crate::output::print_nl_cstr(c"Transcript written on ".as_ptr());
            crate::output::print(texmf_log_name());
            crate::output::print_char(b'.' as i32);
        }
    }

    crate::output::print_ln();
}

// ---------------------------------------------------------------------------
// prune_page_top
// ---------------------------------------------------------------------------

// xetex_format.h:159
const GLUE_PAR_SPLIT_TOP_SKIP_PPT: i16 = 10;
const HLIST_NODE_PPT: u16 = 0;   // xetex_format.h:380
const VLIST_NODE_PPT: u16 = 1;   // xetex_format.h:381
const RULE_NODE_PPT: u16 = 2;    // xetex_format.h:382
const WHATSIT_NODE_PPT: u16 = 8; // xetex_format.h:388
const MARK_NODE_PPT: u16 = 4;    // xetex_format.h:384
const INS_NODE_PPT: u16 = 3;     // xetex_format.h:383
const GLUE_NODE_PPT: u16 = 10;   // xetex_format.h:390
const KERN_NODE_PPT: u16 = 11;   // xetex_format.h:391
const PENALTY_NODE_PPT: u16 = 12; // xetex_format.h:392
const VSPLIT_CODE_PPT: usize = 3; // xetex_format.h:483

/// translated from xetex-xetex0.c:9761 (prune_page_top)
#[no_mangle]
pub unsafe extern "C" fn prune_page_top(p: i32, s: bool) -> i32 {
    use crate::globals::{disc_ptr, set_disc_ptr};
    use crate::mem::TEX_NULL;
    let mut p = p;
    let mut prev_p;
    let mut q;
    let mut r = TEX_NULL;

    prev_p = TEMP_HEAD;
    set_mem_b32s1(TEMP_HEAD, p);

    while p != TEX_NULL {
        match mem_b16s1(p) {
            HLIST_NODE_PPT | VLIST_NODE_PPT | RULE_NODE_PPT => {
                q = crate::mem::new_skip_param(GLUE_PAR_SPLIT_TOP_SKIP_PPT);
                set_mem_b32s1(prev_p, q);
                set_mem_b32s1(q, p);
                if mem_b32s1(temp_ptr() + 1) > mem_b32s1(p + 3) {
                    set_mem_b32s1(temp_ptr() + 1, mem_b32s1(temp_ptr() + 1) - mem_b32s1(p + 3));
                } else {
                    set_mem_b32s1(temp_ptr() + 1, 0);
                }
                p = TEX_NULL;
            }
            WHATSIT_NODE_PPT | MARK_NODE_PPT | INS_NODE_PPT => {
                prev_p = p;
                p = mem_b32s1(prev_p);
            }
            GLUE_NODE_PPT | KERN_NODE_PPT | PENALTY_NODE_PPT => {
                q = p;
                p = mem_b32s1(q);
                set_mem_b32s1(q, TEX_NULL);
                set_mem_b32s1(prev_p, p);
                if s {
                    if disc_ptr(VSPLIT_CODE_PPT) == TEX_NULL {
                        set_disc_ptr(VSPLIT_CODE_PPT, q);
                    } else {
                        set_mem_b32s1(r, q);
                    }
                    r = q;
                } else {
                    flush_node_list(q);
                }
            }
            _ => {
                crate::errors::confusion(c"pruning".as_ptr());
            }
        }
    }

    mem_b32s1(TEMP_HEAD)
}

// ---------------------------------------------------------------------------
// do_marks
// ---------------------------------------------------------------------------

// xetex-constants.h:381–384
const VSPLIT_INIT_DM: i16 = 0;
const FIRE_UP_INIT_DM: i16 = 1;
const FIRE_UP_DONE_DM: i16 = 2;
const DESTROY_MARKS_DM: i16 = 3;
// xetex_format.h:685, 689
const TOP_MARK_CODE_DM: i16 = 0;
const SPLIT_BOT_MARK_CODE_DM: i16 = 4;
// xetex-constants.h:134, 123
const INDEX_NODE_SIZE_DM: i32 = 33;
const MARK_CLASS_NODE_SIZE_DM: i32 = 4;

/// translated from xetex-xetex0.c:9817 (do_marks)
#[no_mangle]
pub unsafe extern "C" fn do_marks(a: i16, l: i16, q: i32) -> bool {
    use crate::globals::{cur_ptr, set_cur_ptr};
    use crate::mem::TEX_NULL;
    let mut q = q;

    if l < 4 {
        for i in 0i16..=15 {
            let cp = if i % 2 != 0 {
                mem_b32s1(q + (i / 2) as i32 + 1)
            } else {
                mem_b32s0(q + (i / 2) as i32 + 1)
            };
            set_cur_ptr(cp);

            if cur_ptr() != TEX_NULL {
                if do_marks(a, l + 1, cur_ptr()) {
                    if i % 2 != 0 {
                        set_mem_b32s1(q + (i / 2) as i32 + 1, TEX_NULL);
                    } else {
                        set_mem_b32s0(q + (i / 2) as i32 + 1, TEX_NULL);
                    }
                    // mem[q].b16.s0-- (reference count of index node)
                    set_mem_b16s0(q, mem_b16s0(q) - 1);
                }
            }
        }

        if mem_b16s0(q) == 0 {
            crate::mem::free_node(q, INDEX_NODE_SIZE_DM);
            q = TEX_NULL;
        }
    } else {
        match a {
            VSPLIT_INIT_DM => {
                if mem_b32s1(q + 2) != TEX_NULL {
                    crate::mem::delete_token_ref(mem_b32s1(q + 2));
                    set_mem_b32s1(q + 2, TEX_NULL);
                    crate::mem::delete_token_ref(mem_b32s0(q + 3));
                    set_mem_b32s0(q + 3, TEX_NULL);
                }
            }
            FIRE_UP_INIT_DM => {
                if mem_b32s0(q + 2) != TEX_NULL {
                    if mem_b32s0(q + 1) != TEX_NULL {
                        crate::mem::delete_token_ref(mem_b32s0(q + 1));
                    }
                    crate::mem::delete_token_ref(mem_b32s1(q + 1));
                    set_mem_b32s1(q + 1, TEX_NULL);
                    let ref_ptr = mem_b32s0(q + 2);
                    if mem_b32s1(ref_ptr) == TEX_NULL {
                        crate::mem::delete_token_ref(ref_ptr);
                        set_mem_b32s0(q + 2, TEX_NULL);
                    } else {
                        set_mem_b32s0(ref_ptr, mem_b32s0(ref_ptr) + 1);
                    }
                    set_mem_b32s0(q + 1, mem_b32s0(q + 2));
                }
            }
            FIRE_UP_DONE_DM => {
                if mem_b32s0(q + 1) != TEX_NULL && mem_b32s1(q + 1) == TEX_NULL {
                    set_mem_b32s1(q + 1, mem_b32s0(q + 1));
                    set_mem_b32s0(mem_b32s0(q + 1), mem_b32s0(mem_b32s0(q + 1)) + 1);
                }
            }
            DESTROY_MARKS_DM => {
                for i in TOP_MARK_CODE_DM..=SPLIT_BOT_MARK_CODE_DM {
                    let cp = if i % 2 != 0 {
                        mem_b32s1(q + (i / 2) as i32 + 1)
                    } else {
                        mem_b32s0(q + (i / 2) as i32 + 1)
                    };
                    set_cur_ptr(cp);

                    if cur_ptr() != TEX_NULL {
                        crate::mem::delete_token_ref(cur_ptr());
                        if i % 2 != 0 {
                            set_mem_b32s1(q + (i / 2) as i32 + 1, TEX_NULL);
                        } else {
                            set_mem_b32s0(q + (i / 2) as i32 + 1, TEX_NULL);
                        }
                    }
                }
            }
            _ => {}
        }

        if mem_b32s0(q + 2) == TEX_NULL && mem_b32s0(q + 3) == TEX_NULL {
            crate::mem::free_node(q, MARK_CLASS_NODE_SIZE_DM);
            q = TEX_NULL;
        }
    }

    q == TEX_NULL
}

// ---------------------------------------------------------------------------
// do_extension
// ---------------------------------------------------------------------------

// xetex_format.h:457–461, 462, 465, 524–533
const OPEN_NODE_DE: i32 = 0;
const WRITE_NODE_DE: i32 = 1;
const CLOSE_NODE_DE: i32 = 2;
const SPECIAL_NODE_DE: i32 = 3;
const LANGUAGE_NODE_DE_W: i16 = 4;  // as i16 for new_whatsit
const PDF_SAVE_POS_NODE_DE: i32 = 21;
const PDF_SAVE_POS_NODE_DE_W: i16 = 21; // as i16 for new_whatsit
const GLYPH_NODE_DE: i32 = 43; // used in not_native_font_error (GLYPH_CODE = 43 = GLYPH_CODE_DE)
const GLYPH_NODE_TYPE_DE_W: i16 = 42; // node type 42 for new_whatsit
const IMMEDIATE_CODE_DE: i32 = 4;
const SET_LANGUAGE_CODE_DE: i32 = 5;
const RESET_TIMER_CODE_DE: i32 = 31;
const SET_RANDOM_SEED_CODE_DE: i32 = 33;
const PIC_FILE_CODE_DE: i32 = 41;
const PDF_FILE_CODE_DE: i32 = 42;
const GLYPH_CODE_DE: i32 = 43;
const XETEX_INPUT_ENCODING_EXTENSION_CODE_DE: i32 = 44;
const XETEX_DEFAULT_ENCODING_EXTENSION_CODE_DE: i32 = 45;
const XETEX_LINEBREAK_LOCALE_EXTENSION_CODE_DE: i32 = 46;
// xetex-constants.h:110, 112, 118, 126
const WRITE_NODE_SIZE_DE: i16 = 2;
const OPEN_NODE_SIZE_DE: i16 = 3;
const GLYPH_NODE_SIZE_DE: i16 = 5;
const SMALL_NODE_SIZE_DE: i16 = 2;
// xetex_format.h:297, 368–370
const EXTENSION_DE: u8 = 59;
const VMODE_DE: i16 = 1;
const HMODE_DE: i16 = 104;
const MMODE_DE: i16 = 207;
// xetex_format.h:212
const CUR_FONT_LOC_DE: i32 = 2_255_400;
// xetex-ext.h:47–48
const AAT_FONT_FLAG_DE: i32 = 0xFFFF;
const OTGR_FONT_FLAG_DE: i32 = 0xFFFE;
// xetex-constants.h:254
const XETEX_INPUT_MODE_AUTO_DE: i32 = 0;
// xetex_format.h:86–87, 66, 72, 75–76
const INT_PAR_LEFT_HYPHEN_MIN_DE: i32 = 51;
const INT_PAR_RIGHT_HYPHEN_MIN_DE: i32 = 52;
const INT_PAR_XETEX_LINEBREAK_LOCALE_DE: i32 = 66;
const INT_PAR_XETEX_USE_GLYPH_METRICS_DE: i32 = 72;
const INT_PAR_XETEX_DEFAULT_INPUT_MODE_DE: i32 = 75;
const INT_PAR_XETEX_DEFAULT_INPUT_ENCODING_DE: i32 = 76;
const INT_BASE_DE: i32 = 7_826_729; // xetex_format.h

/// translated from xetex-xetex0.c:8024 (do_extension)
#[no_mangle]
pub unsafe extern "C" fn do_extension() {
    use crate::globals::{
        cur_name, cur_area, cur_ext, cur_cs, set_cur_cs, random_seed, set_random_seed,
        cur_list_tail, set_cur_list_tail, cur_list_mode, cur_list_aux_b32s1, set_cur_list_aux_b32s1,
        in_open, input_file, font_area_entry,
        epochseconds, set_epochseconds, microseconds, set_microseconds,
    };
    use crate::mem::TEX_NULL;

    match cur_chr() {
        OPEN_NODE_DE => {
            crate::scan::new_write_whatsit(OPEN_NODE_SIZE_DE);
            scan_optional_equals();
            crate::scan::scan_file_name();
            set_mem_b32s1(cur_list_tail() + 1, cur_name());
            set_mem_b32s0(cur_list_tail() + 2, cur_area());
            set_mem_b32s1(cur_list_tail() + 2, cur_ext());
        }

        WRITE_NODE_DE => {
            let k = cur_cs();
            crate::scan::new_write_whatsit(WRITE_NODE_SIZE_DE);
            set_cur_cs(k);
            scan_toks(false, false);
            set_mem_b32s1(cur_list_tail() + 1, def_ref());
        }

        CLOSE_NODE_DE => {
            crate::scan::new_write_whatsit(WRITE_NODE_SIZE_DE);
            set_mem_b32s1(cur_list_tail() + 1, TEX_NULL);
        }

        SPECIAL_NODE_DE => {
            crate::mem::new_whatsit(SPECIAL_NODE_DE as i16, WRITE_NODE_SIZE_DE);
            set_mem_b32s0(cur_list_tail() + 1, TEX_NULL);
            scan_toks(false, true);
            set_mem_b32s1(cur_list_tail() + 1, def_ref());
        }

        IMMEDIATE_CODE_DE => {
            get_x_token();
            if cur_cmd() == EXTENSION_DE && cur_chr() <= CLOSE_NODE_DE {
                let p = cur_list_tail();
                do_extension();
                out_what(cur_list_tail());
                flush_node_list(cur_list_tail());
                set_cur_list_tail(p);
                set_mem_b32s1(p, TEX_NULL);
            } else {
                crate::input::back_input();
            }
        }

        SET_LANGUAGE_CODE_DE => {
            if (cur_list_mode() as i32).abs() != HMODE_DE as i32 {
                report_illegal_case();
            } else {
                crate::mem::new_whatsit(LANGUAGE_NODE_DE_W, SMALL_NODE_SIZE_DE);
                scan_int();
                let lang = if cur_val() <= 0 || cur_val() > 255 { 0 } else { cur_val() };
                set_cur_list_aux_b32s1(lang);
                set_mem_b32s1(cur_list_tail() + 1, cur_list_aux_b32s1());
                set_mem_b16s1(cur_list_tail() + 1,
                    crate::helpers::norm_min(eqtb_b32s1(INT_BASE_DE + INT_PAR_LEFT_HYPHEN_MIN_DE)) as u16);
                set_mem_b16s0(cur_list_tail() + 1,
                    crate::helpers::norm_min(eqtb_b32s1(INT_BASE_DE + INT_PAR_RIGHT_HYPHEN_MIN_DE)) as u16);
            }
        }

        PDF_SAVE_POS_NODE_DE => {
            crate::mem::new_whatsit(PDF_SAVE_POS_NODE_DE_W, SMALL_NODE_SIZE_DE);
        }

        RESET_TIMER_CODE_DE => {
            let mut s = epochseconds();
            let mut m = microseconds();
            crate::texmfmp::get_seconds_and_micros(&mut s, &mut m);
            set_epochseconds(s);
            set_microseconds(m);
        }

        SET_RANDOM_SEED_CODE_DE => {
            scan_int();
            if cur_val() < 0 {
                set_cur_val(-cur_val());
            }
            set_random_seed(cur_val());
            crate::scaled_math::init_randoms(random_seed());
        }

        PIC_FILE_CODE_DE => {
            if (cur_list_mode() as i32).abs() == MMODE_DE as i32 {
                report_illegal_case();
            } else {
                load_picture(false);
            }
        }

        PDF_FILE_CODE_DE => {
            if (cur_list_mode() as i32).abs() == MMODE_DE as i32 {
                report_illegal_case();
            } else {
                load_picture(true);
            }
        }

        GLYPH_CODE_DE => {
            if (cur_list_mode() as i32).abs() == VMODE_DE as i32 {
                crate::input::back_input();
                new_graf(true);
            } else if (cur_list_mode() as i32).abs() == MMODE_DE as i32 {
                report_illegal_case();
            } else {
                let cur_font = eqtb_b32s1(CUR_FONT_LOC_DE);
                let fa = font_area_entry(cur_font);
                if fa == AAT_FONT_FLAG_DE || fa == OTGR_FONT_FLAG_DE {
                    crate::mem::new_whatsit(GLYPH_NODE_TYPE_DE_W, GLYPH_NODE_SIZE_DE);
                    scan_int();
                    if cur_val() < 0 || cur_val() > 65535 {
                        let errmsg = error_here_with_diagnostic(c"Bad glyph number".as_ptr());
                        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
                        capture_to_diagnostic(std::ptr::null_mut());
                        set_help_ptr(2);
                        set_help_line(1, c"A glyph number must be between 0 and 65535.".as_ptr());
                        set_help_line(0, c"I changed this one to zero.".as_ptr());
                        int_error(cur_val());
                        set_cur_val(0);
                    }
                    set_mem_b16s2(cur_list_tail() + 4, cur_font as u16);
                    set_mem_b16s1(cur_list_tail() + 4, cur_val() as u16);
                    // set_native_glyph_metrics(cur_list.tail, ...) expands to
                    // measure_native_glyph(&mem[cur_list.tail], use_glyph_metrics)
                    let use_glyph_metrics = eqtb_b32s1(INT_BASE_DE + INT_PAR_XETEX_USE_GLYPH_METRICS_DE) > 0;
                    let node_ptr = crate::globals::mem_node_addr(cur_list_tail());
                    measure_native_glyph(node_ptr, use_glyph_metrics);
                } else {
                    not_native_font_error(EXTENSION_DE as i32, GLYPH_CODE_DE, cur_font);
                }
            }
        }

        XETEX_INPUT_ENCODING_EXTENSION_CODE_DE => {
            scan_and_pack_name();
            let mut j: i32 = 0;
            let i = get_encoding_mode_and_info(&mut j);
            if i == XETEX_INPUT_MODE_AUTO_DE {
                error_here_with_diagnostic(
                    c"Encoding mode `auto' is not valid for \\XeTeXinputencoding".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(1, c"You can't use `auto' encoding here, only for \\XeTeXdefaultencoding.".as_ptr());
                set_help_line(0, c"I'll ignore this and leave the current encoding unchanged.".as_ptr());
                error();
            } else {
                crate::io::call_set_input_file_encoding(input_file(in_open()), i, j);
            }
        }

        XETEX_DEFAULT_ENCODING_EXTENSION_CODE_DE => {
            scan_and_pack_name();
            let mut j: i32 = 0;
            let i = get_encoding_mode_and_info(&mut j);
            set_eqtb_b32s1(INT_BASE_DE + INT_PAR_XETEX_DEFAULT_INPUT_MODE_DE, i);
            set_eqtb_b32s1(INT_BASE_DE + INT_PAR_XETEX_DEFAULT_INPUT_ENCODING_DE, j);
        }

        XETEX_LINEBREAK_LOCALE_EXTENSION_CODE_DE => {
            crate::scan::scan_file_name();
            if crate::string_pool::length(cur_name()) == 0 {
                set_eqtb_b32s1(INT_BASE_DE + INT_PAR_XETEX_LINEBREAK_LOCALE_DE, 0);
            } else {
                set_eqtb_b32s1(INT_BASE_DE + INT_PAR_XETEX_LINEBREAK_LOCALE_DE, cur_name());
            }
        }

        _ => {
            crate::errors::confusion(c"ext1".as_ptr());
        }
    }
}

// ---------------------------------------------------------------------------
// hpack
// ---------------------------------------------------------------------------

// Node type constants for hpack
const HLIST_NODE_HP: u16 = 0;
const VLIST_NODE_HP: u16 = 1;
const RULE_NODE_HP: u16 = 2;
const UNSET_NODE_HP: u16 = 13;
const INS_NODE_HP: u16 = 3;
const MARK_NODE_HP: u16 = 4;
const ADJUST_NODE_HP: u16 = 5;
const DISC_NODE_HP: u16 = 7;
const WHATSIT_NODE_HP: u16 = 8;
const MATH_NODE_HP: u16 = 9;
const GLUE_NODE_HP: u16 = 10;
const KERN_NODE_HP: u16 = 11;
const KERN_NODE_HP2: u16 = 40; // MATH_NODE subtype treated as kern
const LIG_NODE_HP: u16 = 6;
// whatsit subtypes
const NATIVE_WORD_NODE_HP: u16 = 40;
const NATIVE_WORD_NODE_AT_HP: u16 = 41;
// GLYPH_NODE = 42, PIC/PDF = 43/44: contribute width/height/depth
const A_LEADERS_HP: u16 = 100;
// glue orders
const NORMAL_HP: usize = 0;
const FIL_HP: usize = 1;
const FILL_HP: usize = 2;
const FILLL_HP: usize = 3;
const STRETCHING_HP: u16 = 1;
const SHRINKING_HP: u16 = 2;
// Bidi
const BEFORE_HP: u32 = 0;
const L_CODE_HP: u16 = 4;
const EXPLICIT_HP: u16 = 1;
// INT/DIMEN params
const INT_BASE_HP: i32 = 7_826_729;
const DIMEN_BASE_HP: i32 = 8_941_180;
const INT_PAR_TEXXET_HP: i32 = 69;
const INT_PAR_HBADNESS_HP: i32 = 26;
const DIMEN_PAR_HFUZZ_HP: i32 = 8;
const DIMEN_PAR_OVERFULL_RULE_HP: i32 = 16;
const NATIVE_NODE_SIZE_HP: i32 = 6;
const SMALL_NODE_SIZE_HP: i32 = 2;
const BOX_NODE_SIZE_HP: i32 = 8;
const HLIST_NODE_VAL_HP: u16 = 0;

/// translated from xetex-xetex0.c:6972 (hpack)
#[no_mangle]
pub unsafe extern "C" fn hpack(p: i32, w: i32, m: i16) -> i32 {
    use crate::globals::{
        last_badness, set_last_badness,
        total_stretch, set_total_stretch, total_shrink, set_total_shrink,
        hi_mem_min, avail, set_avail,
        adjust_tail, set_adjust_tail, pre_adjust_tail, set_pre_adjust_tail,
        lr_ptr, set_lr_ptr, lr_problems, set_lr_problems,
        xtx_ligature_present, set_xtx_ligature_present,
        char_base_entry, width_base_entry, height_base_entry, depth_base_entry,
        font_info_b16, font_info_b32s1,
        output_active, pack_begin_line, line,
        mem_gr, set_mem_gr, mem_word, set_mem_word,
        mem_as_u16_ptr, mem_as_u16_ptr_mut,
        font_in_short_display, set_font_in_short_display,
        mem_b16s2,
    };
    use crate::mem::TEX_NULL;

    const ADDITIONAL_HP: i16 = 1;
    const GARBAGE: i32 = 4_999_999 - 12; // MEM_TOP - 12

    set_last_badness(0);
    let r = crate::mem::get_node(BOX_NODE_SIZE_HP);
    set_mem_b16s1(r, HLIST_NODE_VAL_HP);
    set_mem_b16s0(r, 0);
    set_mem_b32s1(r + 4, 0);
    let mut q = r + 5;
    set_mem_b32s1(q, p);
    let mut h: i32 = 0;
    let mut d: i32 = 0;
    let mut x: i32 = 0;
    set_total_stretch(NORMAL_HP, 0);
    set_total_shrink(NORMAL_HP, 0);
    set_total_stretch(FIL_HP, 0);
    set_total_shrink(FIL_HP, 0);
    set_total_stretch(FILL_HP, 0);
    set_total_shrink(FILL_HP, 0);
    set_total_stretch(FILLL_HP, 0);
    set_total_shrink(FILLL_HP, 0);

    if eqtb_b32s1(INT_BASE_HP + INT_PAR_TEXXET_HP) > 0 {
        let t = crate::mem::get_avail();
        set_mem_b32s0(t, BEFORE_HP as i32);
        set_mem_b32s1(t, lr_ptr());
        set_lr_ptr(t);
    }

    let mut p = p;
    'outer: while p != TEX_NULL {
        // reswitch:
        // char-node inner loop
        while p >= hi_mem_min() {
            let f = mem_b16s1(p) as i32; // CHAR_NODE_font(p)
            let c = effective_char(true, f, mem_b16s0(p));
            let info = font_info_b16(char_base_entry(f) + c);
            x += font_info_b32s1(width_base_entry(f) + info.s3 as i32);
            let s = font_info_b32s1(height_base_entry(f) + info.s2 as i32 / 16);
            if s > h { h = s; }
            let s = font_info_b32s1(depth_base_entry(f) + info.s2 as i32 % 16);
            if s > d { d = s; }
            p = mem_b32s1(p); // LLIST_link(p)
        }

        if p == TEX_NULL { break 'outer; }

        match mem_b16s1(p) {
            HLIST_NODE_HP | VLIST_NODE_HP | RULE_NODE_HP | UNSET_NODE_HP => {
                x += mem_b32s1(p + 1);
                let s = if mem_b16s1(p) >= RULE_NODE_HP { 0 } else { mem_b32s1(p + 4) };
                if mem_b32s1(p + 3) - s > h { h = mem_b32s1(p + 3) - s; }
                if mem_b32s1(p + 2) + s > d { d = mem_b32s1(p + 2) + s; }
            }
            INS_NODE_HP | MARK_NODE_HP | ADJUST_NODE_HP => {
                if adjust_tail() != TEX_NULL || pre_adjust_tail() != TEX_NULL {
                    while mem_b32s1(q) != p {
                        q = mem_b32s1(q);
                    }
                    if mem_b16s1(p) == ADJUST_NODE_HP {
                        if mem_b16s0(p) != 0 {
                            if pre_adjust_tail() == TEX_NULL {
                                crate::errors::confusion(c"pre vadjust".as_ptr());
                            }
                            set_mem_b32s1(pre_adjust_tail(), mem_b32s1(p + 1));
                            while mem_b32s1(pre_adjust_tail()) != TEX_NULL {
                                set_pre_adjust_tail(mem_b32s1(pre_adjust_tail()));
                            }
                        } else {
                            if adjust_tail() == TEX_NULL {
                                crate::errors::confusion(c"pre vadjust".as_ptr());
                            }
                            set_mem_b32s1(adjust_tail(), mem_b32s1(p + 1));
                            while mem_b32s1(adjust_tail()) != TEX_NULL {
                                set_adjust_tail(mem_b32s1(adjust_tail()));
                            }
                        }
                        p = mem_b32s1(p);
                        crate::mem::free_node(mem_b32s1(q), SMALL_NODE_SIZE_HP);
                    } else {
                        set_mem_b32s1(adjust_tail(), p);
                        set_adjust_tail(p);
                        p = mem_b32s1(p);
                    }
                    set_mem_b32s1(q, p);
                    p = q;
                }
                // fall through to p = LLIST_link(p) at bottom
            }
            WHATSIT_NODE_HP => {
                let sub = mem_b16s0(p);
                match sub {
                    NATIVE_WORD_NODE_HP | NATIVE_WORD_NODE_AT_HP => {
                        // Native word node merging logic
                        let mut k: i32;
                        if q != r + 5 && mem_b16s1(q) == DISC_NODE_HP {
                            k = mem_b16s0(q) as i32;
                        } else {
                            k = 0;
                        }
                        while mem_b32s1(q) != p {
                            k -= 1;
                            q = mem_b32s1(q);
                            if mem_b16s1(q) == DISC_NODE_HP {
                                k = mem_b16s0(q) as i32;
                            }
                        }
                        let mut pp = mem_b32s1(p);
                        // restart loop
                        loop {
                            if k <= 0 && pp != TEX_NULL && pp < hi_mem_min() {
                                let pp_type = mem_b16s1(pp);
                                let pp_sub = mem_b16s0(pp);
                                if pp_type == WHATSIT_NODE_HP
                                    && (pp_sub == NATIVE_WORD_NODE_HP || pp_sub == NATIVE_WORD_NODE_AT_HP)
                                    && mem_b16s2(pp + 4) == mem_b16s2(p + 4)
                                {
                                    pp = mem_b32s1(pp);
                                    continue; // goto restart
                                } else if pp_type == DISC_NODE_HP {
                                    let ppp = mem_b32s1(pp);
                                    if ppp != TEX_NULL && ppp < hi_mem_min()
                                        && mem_b16s1(ppp) == WHATSIT_NODE_HP
                                        && (mem_b16s0(ppp) == NATIVE_WORD_NODE_HP
                                            || mem_b16s0(ppp) == NATIVE_WORD_NODE_AT_HP)
                                        && mem_b16s2(ppp + 4) == mem_b16s2(p + 4)
                                    {
                                        pp = mem_b32s1(ppp);
                                        continue; // goto restart
                                    }
                                }
                            }
                            break;
                        }
                        // end restart loop: pp is now past the last mergeable node
                        if pp != mem_b32s1(p) {
                            // merge the run of native word nodes into one
                            let mut total_chars: i32 = 0;
                            let mut scan_p = mem_b32s1(q);
                            while scan_p != pp {
                                if mem_b16s1(scan_p) == WHATSIT_NODE_HP {
                                    total_chars += mem_b16s1(scan_p + 4) as i32;
                                }
                                scan_p = mem_b32s1(scan_p);
                            }
                            let mut scan_p = mem_b32s1(q);
                            let merged = new_native_word_node(mem_b16s2(scan_p + 4) as i32, total_chars);
                            set_mem_b16s0(merged, mem_b16s0(scan_p));
                            set_mem_b32s1(q, merged);
                            // find last node before pp
                            let mut last_before_pp = scan_p;
                            while mem_b32s1(last_before_pp) != pp {
                                last_before_pp = mem_b32s1(last_before_pp);
                            }
                            set_mem_b32s1(merged, mem_b32s1(last_before_pp));
                            set_mem_b32s1(last_before_pp, TEX_NULL);
                            // copy text chars
                            total_chars = 0;
                            let mut src_p = scan_p;
                            loop {
                                if mem_b16s1(src_p) == WHATSIT_NODE_HP {
                                    let count = mem_b16s1(src_p + 4) as i32;
                                    let src = mem_as_u16_ptr(src_p + NATIVE_NODE_SIZE_HP);
                                    let dst = mem_as_u16_ptr_mut(merged + NATIVE_NODE_SIZE_HP);
                                    for k_idx in 0..count {
                                        *dst.offset((total_chars + k_idx) as isize) =
                                            *src.offset(k_idx as isize);
                                    }
                                    total_chars += count;
                                }
                                if mem_b32s1(src_p) == TEX_NULL { break; }
                                src_p = mem_b32s1(src_p);
                            }
                            flush_node_list(scan_p);
                            p = mem_b32s1(q);
                            let use_glyph_metrics = eqtb_b32s1(INT_BASE_HP + 72) > 0;
                            measure_native_node(crate::globals::mem_node_addr(p), use_glyph_metrics);
                        }
                        // metrics are in mem[p+3].b32.s1 (height) and mem[p+2].b32.s1 (depth)
                        if mem_b32s1(p + 3) > h { h = mem_b32s1(p + 3); }
                        if mem_b32s1(p + 2) > d { d = mem_b32s1(p + 2); }
                        x += mem_b32s1(p + 1);
                    }
                    42 | 43 | 44 => {
                        // GLYPH_NODE (42), PIC_NODE (43), PDF_NODE (44)
                        if mem_b32s1(p + 3) > h { h = mem_b32s1(p + 3); }
                        if mem_b32s1(p + 2) > d { d = mem_b32s1(p + 2); }
                        x += mem_b32s1(p + 1);
                    }
                    _ => {}
                }
            }
            GLUE_NODE_HP => {
                let g = mem_b32s0(p + 1);
                x += mem_b32s1(g + 1);
                let o = mem_b16s1(g) as usize;
                set_total_stretch(o, total_stretch(o) + mem_b32s1(g + 2));
                let o = mem_b16s0(g) as usize;
                set_total_shrink(o, total_shrink(o) + mem_b32s1(g + 3));
                if mem_b16s0(p) >= A_LEADERS_HP {
                    let g = mem_b32s1(p + 1);
                    if mem_b32s1(g + 3) > h { h = mem_b32s1(g + 3); }
                    if mem_b32s1(g + 2) > d { d = mem_b32s1(g + 2); }
                }
            }
            KERN_NODE_HP | KERN_NODE_HP2 => {
                x += mem_b32s1(p + 1);
            }
            MATH_NODE_HP => {
                x += mem_b32s1(p + 1);
                if eqtb_b32s1(INT_BASE_HP + INT_PAR_TEXXET_HP) > 0 {
                    if mem_b16s0(p) % 2 != 0 {
                        // odd subtype: closing a direction
                        let expected = (L_CODE_HP as u32) * (mem_b16s0(p) as u32 / L_CODE_HP as u32) + 3;
                        if mem_b32s0(lr_ptr()) == expected as i32 {
                            let t = lr_ptr();
                            set_lr_ptr(mem_b32s1(t));
                            set_mem_b32s1(t, avail());
                            set_avail(t);
                        } else {
                            set_lr_problems(lr_problems() + 1);
                            set_mem_b16s1(p, KERN_NODE_HP);
                            set_mem_b16s0(p, EXPLICIT_HP);
                        }
                    } else {
                        // even subtype: opening a direction
                        let t = crate::mem::get_avail();
                        set_mem_b32s0(t, (L_CODE_HP as i32) * (mem_b16s0(p) as i32 / L_CODE_HP as i32) + 3);
                        set_mem_b32s1(t, lr_ptr());
                        set_lr_ptr(t);
                    }
                }
            }
            LIG_NODE_HP => {
                // Ligature node: expand in place and reprocess
                set_mem_word(GARBAGE, mem_word(p + 1));
                set_mem_b32s1(GARBAGE, mem_b32s1(p));
                p = GARBAGE;
                set_xtx_ligature_present(true);
                continue 'outer; // goto reswitch
            }
            _ => {}
        }
        p = mem_b32s1(p); // LLIST_link(p)
    }

    if adjust_tail() != TEX_NULL { set_mem_b32s1(adjust_tail(), TEX_NULL); }
    if pre_adjust_tail() != TEX_NULL { set_mem_b32s1(pre_adjust_tail(), TEX_NULL); }
    set_mem_b32s1(r + 3, h);
    set_mem_b32s1(r + 2, d);

    let w = if m == ADDITIONAL_HP { x + w } else { w };
    set_mem_b32s1(r + 1, w);
    x = w - x;

    // Glue setting: goto exit (returns r) or goto common_ending (warning + box)
    let do_common_ending: bool = 'glue: {
        if x == 0 {
            set_mem_b16s1(r + 5, 0);
            set_mem_b16s0(r + 5, 0);
            set_mem_gr(r + 6, 0.0);
            break 'glue false; // goto exit
        } else if x > 0 {
            let o: usize = if total_stretch(FILLL_HP) != 0 { FILLL_HP }
                else if total_stretch(FILL_HP) != 0 { FILL_HP }
                else if total_stretch(FIL_HP) != 0 { FIL_HP }
                else { NORMAL_HP };
            set_mem_b16s0(r + 5, o as u16);
            set_mem_b16s1(r + 5, STRETCHING_HP);
            if total_stretch(o) != 0 {
                set_mem_gr(r + 6, x as f64 / total_stretch(o) as f64);
            } else {
                set_mem_b16s1(r + 5, 0);
                set_mem_gr(r + 6, 0.0);
            }
            if o == NORMAL_HP && mem_b32s1(r + 5) != TEX_NULL {
                set_last_badness(crate::helpers::badness(x, total_stretch(NORMAL_HP)));
                if last_badness() > eqtb_b32s1(INT_BASE_HP + INT_PAR_HBADNESS_HP) {
                    crate::output::print_ln();
                    crate::output::diagnostic_begin_capture_warning_here();
                    if last_badness() > 100 {
                        crate::output::print_nl_cstr(c"Underfull".as_ptr());
                    } else {
                        crate::output::print_nl_cstr(c"Loose".as_ptr());
                    }
                    crate::output::print_cstr(c" \\hbox (badness ".as_ptr());
                    crate::output::print_int(last_badness());
                    break 'glue true; // goto common_ending
                }
            }
            break 'glue false; // goto exit
        } else {
            // x < 0: shrinking
            let o: usize = if total_shrink(FILLL_HP) != 0 { FILLL_HP }
                else if total_shrink(FILL_HP) != 0 { FILL_HP }
                else if total_shrink(FIL_HP) != 0 { FIL_HP }
                else { NORMAL_HP };
            set_mem_b16s0(r + 5, o as u16);
            set_mem_b16s1(r + 5, SHRINKING_HP);
            if total_shrink(o) != 0 {
                set_mem_gr(r + 6, (-x) as f64 / total_shrink(o) as f64);
            } else {
                set_mem_b16s1(r + 5, 0);
                set_mem_gr(r + 6, 0.0);
            }
            if total_shrink(o) < (-x) && o == NORMAL_HP && mem_b32s1(r + 5) != TEX_NULL {
                set_last_badness(1_000_000);
                set_mem_gr(r + 6, 1.0);
                if (-x) - total_shrink(NORMAL_HP) > eqtb_b32s1(DIMEN_BASE_HP + DIMEN_PAR_HFUZZ_HP)
                    || eqtb_b32s1(INT_BASE_HP + INT_PAR_HBADNESS_HP) < 100
                {
                    if eqtb_b32s1(DIMEN_BASE_HP + DIMEN_PAR_OVERFULL_RULE_HP) > 0
                        && (-x) - total_shrink(NORMAL_HP) > eqtb_b32s1(DIMEN_BASE_HP + DIMEN_PAR_HFUZZ_HP)
                    {
                        while mem_b32s1(q) != TEX_NULL {
                            q = mem_b32s1(q);
                        }
                        let rule = crate::mem::new_rule();
                        set_mem_b32s1(q, rule);
                        set_mem_b32s1(rule + 1, eqtb_b32s1(DIMEN_BASE_HP + DIMEN_PAR_OVERFULL_RULE_HP));
                    }
                    crate::output::print_ln();
                    crate::output::diagnostic_begin_capture_warning_here();
                    crate::output::print_nl_cstr(c"Overfull \\hbox (".as_ptr());
                    crate::output::print_scaled((-x) - total_shrink(NORMAL_HP));
                    crate::output::print_cstr(c"pt too wide".as_ptr());
                    break 'glue true; // goto common_ending
                }
            } else if o == NORMAL_HP && mem_b32s1(r + 5) != TEX_NULL {
                set_last_badness(crate::helpers::badness(-x, total_shrink(NORMAL_HP)));
                if last_badness() > eqtb_b32s1(INT_BASE_HP + INT_PAR_HBADNESS_HP) {
                    crate::output::print_ln();
                    crate::output::diagnostic_begin_capture_warning_here();
                    crate::output::print_nl_cstr(c"Tight \\hbox (badness ".as_ptr());
                    crate::output::print_int(last_badness());
                    break 'glue true; // goto common_ending
                }
            }
            break 'glue false; // goto exit
        }
    };

    if do_common_ending {
        if output_active() {
            crate::output::print_cstr(c") has occurred while \\output is active".as_ptr());
        } else {
            if pack_begin_line() != 0 {
                if pack_begin_line() > 0 {
                    crate::output::print_cstr(c") in paragraph at lines ".as_ptr());
                } else {
                    crate::output::print_cstr(c") in alignment at lines ".as_ptr());
                }
                crate::output::print_int(pack_begin_line().abs());
                crate::output::print_cstr(c"--".as_ptr());
            } else {
                crate::output::print_cstr(c") detected at line ".as_ptr());
            }
            crate::output::print_int(line());
        }
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::print_ln();
        set_font_in_short_display(0); // FONT_BASE = 0
        short_display(mem_b32s1(r + 5));
        crate::output::print_ln();
        crate::output::begin_diagnostic();
        show_box(r);
        crate::output::end_diagnostic(true);
    }

    // exit: LR cleanup
    if eqtb_b32s1(INT_BASE_HP + INT_PAR_TEXXET_HP) > 0 {
        if mem_b32s0(lr_ptr()) != BEFORE_HP as i32 {
            while mem_b32s1(q) != TEX_NULL {
                q = mem_b32s1(q);
            }
            loop {
                let t = q;
                let new_q = crate::mem::new_math(0, mem_b32s0(lr_ptr()) as i16);
                set_mem_b32s1(t, new_q);
                q = new_q;
                set_lr_problems(lr_problems() + 10000);
                let t2 = lr_ptr();
                set_lr_ptr(mem_b32s1(t2));
                set_mem_b32s1(t2, avail());
                set_avail(t2);
                if mem_b32s0(lr_ptr()) == BEFORE_HP as i32 { break; }
            }
        }
        if lr_problems() > 0 {
            crate::output::print_ln();
            crate::output::diagnostic_begin_capture_warning_here();
            crate::output::print_nl_cstr(c"\\endL or \\endR problem (".as_ptr());
            crate::output::print_int(lr_problems() / 10000);
            crate::output::print_cstr(c" missing, ".as_ptr());
            crate::output::print_int(lr_problems() % 10000);
            crate::output::print_cstr(c" extra".as_ptr());
            set_lr_problems(0);
            // goto common_ending equivalent - print the rest
            if output_active() {
                crate::output::print_cstr(c") has occurred while \\output is active".as_ptr());
            } else {
                if pack_begin_line() != 0 {
                    if pack_begin_line() > 0 {
                        crate::output::print_cstr(c") in paragraph at lines ".as_ptr());
                    } else {
                        crate::output::print_cstr(c") in alignment at lines ".as_ptr());
                    }
                    crate::output::print_int(pack_begin_line().abs());
                    crate::output::print_cstr(c"--".as_ptr());
                } else {
                    crate::output::print_cstr(c") detected at line ".as_ptr());
                }
                crate::output::print_int(line());
            }
            capture_to_diagnostic(std::ptr::null_mut());
            crate::output::print_ln();
            set_font_in_short_display(0);
            short_display(mem_b32s1(r + 5));
            crate::output::print_ln();
            crate::output::begin_diagnostic();
            show_box(r);
            crate::output::end_diagnostic(true);
        }
        let t = lr_ptr();
        set_lr_ptr(mem_b32s1(t));
        set_mem_b32s1(t, avail());
        set_avail(t);
        if lr_ptr() != TEX_NULL {
            crate::errors::confusion(c"LR1".as_ptr());
        }
    }

    r
}

// ---------------------------------------------------------------------------
// vpackage
// ---------------------------------------------------------------------------

// xetex-constants.h:132, 173–175, 182, 275, 281, 422
const BOX_NODE_SIZE_VP: i32 = 8;
const NORMAL_VP: usize = 0;
const FIL_VP: usize = 1;
const FILL_VP: usize = 2;
const FILLL_VP: usize = 3;
const STRETCHING_VP: u16 = 1;
const SHRINKING_VP: u16 = 2;
const A_LEADERS_VP: u16 = 100;
// xetex_format.h:381, 383–384, 466–467
const VLIST_NODE_VP: u16 = 1;
const RULE_NODE_VP: u16 = 2;
const WHATSIT_NODE_VP: u16 = 8;
const PIC_NODE_VP: u16 = 43;
const PDF_NODE_VP: u16 = 44;
const KERN_NODE_VP: u16 = 11;
const GLUE_NODE_VP: u16 = 10;
// xetex_format.h:71, 106, 131
const INT_PAR_XETEX_UPWARDS_VP: i32 = 71;
const INT_PAR_VBADNESS_VP: i32 = 27;
const DIMEN_PAR_VFUZZ_VP: i32 = 9;
const INT_BASE_VP: i32 = 7_826_729;
const DIMEN_BASE_VP: i32 = 8_941_180; // xetex_format.h:222
// xetex_format.h:297 (ADDITIONAL = 1 already defined as ADDITIONAL in this file's context)

/// translated from xetex-xetex0.c:7424 (vpackage)
#[no_mangle]
pub unsafe extern "C" fn vpackage(p: i32, h: i32, m: i16, l: i32) -> i32 {
    use crate::globals::{
        last_badness, set_last_badness, total_stretch, set_total_stretch,
        total_shrink, set_total_shrink, hi_mem_min, output_active, pack_begin_line, line,
        mem_gr, set_mem_gr,
    };
    use crate::mem::TEX_NULL;

    const ADDITIONAL_VP: i16 = 1;

    set_last_badness(0);
    let r = crate::mem::get_node(BOX_NODE_SIZE_VP);
    set_mem_b16s1(r, VLIST_NODE_VP);
    if eqtb_b32s1(INT_BASE_VP + INT_PAR_XETEX_UPWARDS_VP) > 0 {
        set_mem_b16s0(r, 1);
    } else {
        set_mem_b16s0(r, 0);
    }
    set_mem_b32s1(r + 4, 0);
    set_mem_b32s1(r + 5, p);

    let mut w: i32 = 0;
    let mut d: i32 = 0;
    let mut x: i32 = 0;
    set_total_stretch(NORMAL_VP, 0);
    set_total_shrink(NORMAL_VP, 0);
    set_total_stretch(FIL_VP, 0);
    set_total_shrink(FIL_VP, 0);
    set_total_stretch(FILL_VP, 0);
    set_total_shrink(FILL_VP, 0);
    set_total_stretch(FILLL_VP, 0);
    set_total_shrink(FILLL_VP, 0);

    let mut p = p;
    while p != TEX_NULL {
        if p >= hi_mem_min() {
            // is_char_node(p): cannot appear in vlist
            crate::errors::confusion(c"vpack".as_ptr());
        } else {
            match mem_b16s1(p) {
                0 | 1 | 2 | 13 => {
                    // HLIST_NODE, VLIST_NODE, RULE_NODE, UNSET_NODE
                    x += d + mem_b32s1(p + 3);
                    d = mem_b32s1(p + 2);
                    let s = if mem_b16s1(p) >= RULE_NODE_VP { 0 } else { mem_b32s1(p + 4) };
                    if mem_b32s1(p + 1) + s > w {
                        w = mem_b32s1(p + 1) + s;
                    }
                }
                WHATSIT_NODE_VP => {
                    let sub = mem_b16s0(p);
                    if sub == PIC_NODE_VP || sub == PDF_NODE_VP {
                        x += d + mem_b32s1(p + 3);
                        d = mem_b32s1(p + 2);
                        if mem_b32s1(p + 1) > w {
                            w = mem_b32s1(p + 1);
                        }
                    }
                }
                GLUE_NODE_VP => {
                    x += d;
                    d = 0;
                    let g = mem_b32s0(p + 1);
                    x += mem_b32s1(g + 1);
                    let o = mem_b16s1(g) as usize;
                    set_total_stretch(o, total_stretch(o) + mem_b32s1(g + 2));
                    let o = mem_b16s0(g) as usize;
                    set_total_shrink(o, total_shrink(o) + mem_b32s1(g + 3));
                    if mem_b16s0(p) >= A_LEADERS_VP {
                        let g = mem_b32s1(p + 1);
                        if mem_b32s1(g + 1) > w {
                            w = mem_b32s1(g + 1);
                        }
                    }
                }
                KERN_NODE_VP => {
                    x += d + mem_b32s1(p + 1);
                    d = 0;
                }
                _ => {}
            }
        }
        p = mem_b32s1(p); // LLIST_link(p)
    }

    set_mem_b32s1(r + 1, w);
    if d > l {
        x += d - l;
        set_mem_b32s1(r + 2, l);
    } else {
        set_mem_b32s1(r + 2, d);
    }

    let h = if m == ADDITIONAL_VP { x + h } else { h };
    set_mem_b32s1(r + 3, h);
    x = h - x;

    // Each path below either returns early (goto exit) or sets do_common_ending
    let do_common_ending: bool = 'glue: {
        if x == 0 {
            set_mem_b16s1(r + 5, 0); // NORMAL
            set_mem_b16s0(r + 5, 0);
            set_mem_gr(r + 6, 0.0);
            break 'glue false; // goto exit
        } else if x > 0 {
            // stretching
            let o: usize = if total_stretch(FILLL_VP) != 0 { FILLL_VP }
                else if total_stretch(FILL_VP) != 0 { FILL_VP }
                else if total_stretch(FIL_VP) != 0 { FIL_VP }
                else { NORMAL_VP };
            set_mem_b16s0(r + 5, o as u16);
            set_mem_b16s1(r + 5, STRETCHING_VP);
            if total_stretch(o) != 0 {
                set_mem_gr(r + 6, x as f64 / total_stretch(o) as f64);
            } else {
                set_mem_b16s1(r + 5, 0); // NORMAL
                set_mem_gr(r + 6, 0.0);
            }
            if o == NORMAL_VP && mem_b32s1(r + 5) != TEX_NULL {
                set_last_badness(crate::helpers::badness(x, total_stretch(NORMAL_VP)));
                if last_badness() > eqtb_b32s1(INT_BASE_VP + INT_PAR_VBADNESS_VP) {
                    crate::output::print_ln();
                    crate::output::diagnostic_begin_capture_warning_here();
                    if last_badness() > 100 {
                        crate::output::print_nl_cstr(c"Underfull".as_ptr());
                    } else {
                        crate::output::print_nl_cstr(c"Loose".as_ptr());
                    }
                    crate::output::print_cstr(c" \\vbox (badness ".as_ptr());
                    crate::output::print_int(last_badness());
                    break 'glue true; // goto common_ending
                }
            }
            break 'glue false; // goto exit
        } else {
            // shrinking (x < 0)
            let o: usize = if total_shrink(FILLL_VP) != 0 { FILLL_VP }
                else if total_shrink(FILL_VP) != 0 { FILL_VP }
                else if total_shrink(FIL_VP) != 0 { FIL_VP }
                else { NORMAL_VP };
            set_mem_b16s0(r + 5, o as u16);
            set_mem_b16s1(r + 5, SHRINKING_VP);
            if total_shrink(o) != 0 {
                set_mem_gr(r + 6, (-(x as i64)) as f64 / total_shrink(o) as f64);
            } else {
                set_mem_b16s1(r + 5, 0); // NORMAL
                set_mem_gr(r + 6, 0.0);
            }
            if total_shrink(o) < (-x) && o == NORMAL_VP && mem_b32s1(r + 5) != TEX_NULL {
                set_last_badness(1_000_000);
                set_mem_gr(r + 6, 1.0);
                if (-x) - total_shrink(NORMAL_VP) > eqtb_b32s1(DIMEN_BASE_VP + DIMEN_PAR_VFUZZ_VP)
                    || eqtb_b32s1(INT_BASE_VP + INT_PAR_VBADNESS_VP) < 100
                {
                    crate::output::print_ln();
                    crate::output::diagnostic_begin_capture_warning_here();
                    crate::output::print_nl_cstr(c"Overfull \\vbox (".as_ptr());
                    crate::output::print_scaled((-x) - total_shrink(NORMAL_VP));
                    crate::output::print_cstr(c"pt too high".as_ptr());
                    break 'glue true; // goto common_ending
                }
            } else if o == NORMAL_VP && mem_b32s1(r + 5) != TEX_NULL {
                set_last_badness(crate::helpers::badness(-x, total_shrink(NORMAL_VP)));
                if last_badness() > eqtb_b32s1(INT_BASE_VP + INT_PAR_VBADNESS_VP) {
                    crate::output::print_ln();
                    crate::output::diagnostic_begin_capture_warning_here();
                    crate::output::print_nl_cstr(c"Tight \\vbox (badness ".as_ptr());
                    crate::output::print_int(last_badness());
                    break 'glue true; // goto common_ending
                }
            }
            break 'glue false; // goto exit
        }
    };

    if do_common_ending {
        // common_ending:
        if output_active() {
            crate::output::print_cstr(c") has occurred while \\output is active".as_ptr());
        } else {
            if pack_begin_line() != 0 {
                crate::output::print_cstr(c") in alignment at lines ".as_ptr());
                crate::output::print_int(pack_begin_line().abs());
                crate::output::print_cstr(c"--".as_ptr());
            } else {
                crate::output::print_cstr(c") detected at line ".as_ptr());
            }
            crate::output::print_int(line());
            crate::output::print_ln();
        }
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::begin_diagnostic();
        show_box(r);
        crate::output::end_diagnostic(true);
    }

    r
}

// conv_toks
//
// translated from xetex-xetex0.c:5163 (conv_toks)
#[no_mangle]
pub unsafe extern "C" fn conv_toks() {
    use crate::globals::{
        cur_chr, cur_cs, cur_val, set_cur_val,
        font_name_entry, font_area_entry, font_size_entry, font_dsize_entry, font_layout_engine_ptr,
        hi_mem_min, job_name,
        mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1, set_mem_b32s1,
        pool_ptr, set_pool_ptr, str_ptr, set_str_ptr, str_start, str_pool_entry,
        scanner_status, set_scanner_status,
        selector, set_selector,
        warning_index, set_warning_index,
        def_ref, set_def_ref,
        avail, set_avail,
        cur_ptr,
    };
    use crate::mem::TEX_NULL;

    // node type constants (xetex_format.h:380–396)
    const INS_NODE_CT: u16 = 3;
    const MARK_NODE_CT: u16 = 4;
    const ADJUST_NODE_CT: u16 = 5;
    const HLIST_NODE_CT: u16 = 0;
    const DISC_NODE_CT: u16 = 7;
    const MATH_NODE_CT: u16 = 9;
    const PENALTY_NODE_CT: u16 = 12;
    const MARGIN_KERN_NODE_CT: u16 = 40;
    // GLUE_NODE = 10, KERN_NODE = 11 already defined at module level

    // convert command chr codes (xetex_format.h:490–519)
    const NUMBER_CODE_CT: i32 = 0;
    const ROMAN_NUMERAL_CODE_CT: i32 = 1;
    const STRING_CODE_CT: i32 = 2;
    const MEANING_CODE_CT: i32 = 3;
    const FONT_NAME_CODE_CT: i32 = 4;
    const ETEX_REVISION_CODE_CT: i32 = 5;
    const EXPANDED_CODE_CT: i32 = 6;
    const LEFT_MARGIN_KERN_CODE_CT: i32 = 16;
    const RIGHT_MARGIN_KERN_CODE_CT: i32 = 17;
    const PDF_STRCMP_CODE_CT: i32 = 18;
    const PDF_CREATION_DATE_CODE_CT: i32 = 22;
    const PDF_FILE_MOD_DATE_CODE_CT: i32 = 23;
    const PDF_FILE_SIZE_CODE_CT: i32 = 24;
    const PDF_MDFIVE_SUM_CODE_CT: i32 = 25;
    const PDF_FILE_DUMP_CODE_CT: i32 = 26;
    const UNIFORM_DEVIATE_CODE_CT: i32 = 29;
    const NORMAL_DEVIATE_CODE_CT: i32 = 30;
    const XETEX_REVISION_CODE_CT: i32 = 33;
    const XETEX_VARIATION_NAME_CODE_CT: i32 = 34;
    const XETEX_FEATURE_NAME_CODE_CT: i32 = 35;
    const XETEX_SELECTOR_NAME_CODE_CT: i32 = 36;
    const XETEX_GLYPH_NAME_CODE_CT: i32 = 37;
    const XETEX_UCHAR_CODE_CT: i32 = 38;
    const XETEX_UCHARCAT_CODE_CT: i32 = 39;
    const JOB_NAME_CODE_CT: i32 = 40;

    const GLUE_PAR_LEFT_SKIP: i32 = 7;   // xetex_format.h:156
    const GLUE_PAR_RIGHT_SKIP: i32 = 8;  // xetex_format.h:157

    const TEMP_HEAD_CT: i32 = 4_999_999 - 3;   // MEM_TOP - 3
    const GARBAGE_CT: i32 = 4_999_999 - 12;    // MEM_TOP - 12

    // CONVERT command code (xetex_format.h:352)
    const CONVERT: u16 = 110;

    let mut cat: i32 = 0;
    let c = cur_chr() as i32;
    let mut fnt: i32 = 0;
    let mut arg1: i32 = 0;
    let mut arg2: i32 = 0;
    let mut p = TEX_NULL;
    let mut boolvar: bool = false;
    let mut s: i32 = 0;
    let mut b: i32;

    // ---- Phase 1: scanning ----
    match c {
        NUMBER_CODE_CT | ROMAN_NUMERAL_CODE_CT => {
            crate::scan::scan_int();
        }
        STRING_CODE_CT | MEANING_CODE_CT => {
            let save_ss = scanner_status();
            set_scanner_status(NORMAL_SCANNER);
            crate::scan::get_token();
            set_scanner_status(save_ss);
        }
        FONT_NAME_CODE_CT => {
            crate::scan::scan_font_ident();
        }
        ETEX_REVISION_CODE_CT => {}
        EXPANDED_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string()
            } else {
                0
            };
            scan_pdf_ext_toks();
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            begin_token_list(mem_b32s1(def_ref()), INSERTED);
            set_mem_b32s1(def_ref(), avail());
            set_avail(def_ref());
            set_def_ref(save_dr);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            return;
        }
        LEFT_MARGIN_KERN_CODE_CT | RIGHT_MARGIN_KERN_CODE_CT => {
            crate::scan::scan_register_num();
            if cur_val() < 256 {
                p = eqtb_b32s1(BOX_BASE + cur_val());
            } else {
                crate::save::find_sa_element(4, cur_val(), false);
                if cur_ptr() == TEX_NULL {
                    p = TEX_NULL;
                } else {
                    p = mem_b32s1(cur_ptr() + 1);
                }
            }
            if p == TEX_NULL || mem_b16s1(p) != HLIST_NODE_CT {
                crate::errors::pdf_error(
                    c"marginkern".as_ptr(),
                    c"a non-empty hbox expected".as_ptr(),
                );
            }
        }
        PDF_CREATION_DATE_CODE_CT => {
            b = pool_ptr();
            crate::texmfmp::getcreationdate();
            set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks(b));
            begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
            // falls through to phase 2 (no PDF_CREATION_DATE_CODE_CT case there)
        }
        PDF_FILE_MOD_DATE_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string()
            } else {
                0
            };
            scan_pdf_ext_toks();
            if selector() == SELECTOR_NEW_STRING {
                crate::errors::pdf_error(
                    c"tokens".as_ptr(),
                    c"tokens_to_string() called while selector = new_string".as_ptr(),
                );
            }
            let old_sel = selector();
            set_selector(SELECTOR_NEW_STRING);
            crate::output::show_token_list(mem_b32s1(def_ref()), TEX_NULL, pool_size() - pool_ptr());
            set_selector(old_sel);
            s = crate::string_pool::make_string();
            crate::mem::delete_token_ref(def_ref());
            set_def_ref(save_dr);
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            b = pool_ptr();
            crate::texmfmp::getfilemoddate(s);
            set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks(b));
            if s == str_ptr() - 1 {
                set_str_ptr(str_ptr() - 1);
                set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
            }
            begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            return;
        }
        PDF_FILE_SIZE_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string()
            } else {
                0
            };
            scan_pdf_ext_toks();
            if selector() == SELECTOR_NEW_STRING {
                crate::errors::pdf_error(
                    c"tokens".as_ptr(),
                    c"tokens_to_string() called while selector = new_string".as_ptr(),
                );
            }
            let old_sel = selector();
            set_selector(SELECTOR_NEW_STRING);
            crate::output::show_token_list(mem_b32s1(def_ref()), TEX_NULL, pool_size() - pool_ptr());
            set_selector(old_sel);
            s = crate::string_pool::make_string();
            crate::mem::delete_token_ref(def_ref());
            set_def_ref(save_dr);
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            b = pool_ptr();
            crate::texmfmp::getfilesize(s);
            set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks(b));
            if s == str_ptr() - 1 {
                set_str_ptr(str_ptr() - 1);
                set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
            }
            begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            return;
        }
        PDF_MDFIVE_SUM_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string()
            } else {
                0
            };
            boolvar = scan_keyword(c"file".as_ptr());
            scan_pdf_ext_toks();
            if selector() == SELECTOR_NEW_STRING {
                crate::errors::pdf_error(
                    c"tokens".as_ptr(),
                    c"tokens_to_string() called while selector = new_string".as_ptr(),
                );
            }
            let old_sel = selector();
            set_selector(SELECTOR_NEW_STRING);
            crate::output::show_token_list(mem_b32s1(def_ref()), TEX_NULL, pool_size() - pool_ptr());
            set_selector(old_sel);
            s = crate::string_pool::make_string();
            crate::mem::delete_token_ref(def_ref());
            set_def_ref(save_dr);
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            b = pool_ptr();
            crate::texmfmp::getmd5sum(s, boolvar);
            set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks(b));
            if s == str_ptr() - 1 {
                set_str_ptr(str_ptr() - 1);
                set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
            }
            begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            return;
        }
        PDF_FILE_DUMP_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string()
            } else {
                0
            };
            set_cur_val(0);
            if scan_keyword(c"offset".as_ptr()) {
                crate::scan::scan_int();
                if cur_val() < 0 {
                    error_here_with_diagnostic(c"Bad file offset".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(2);
                    set_help_line(1, c"A file offset must be between 0 and 2^_31_-1,".as_ptr());
                    set_help_line(0, c"I changed this one to zero.".as_ptr());
                    int_error(cur_val());
                    set_cur_val(0);
                }
            }
            let i = cur_val();
            set_cur_val(0);
            if scan_keyword(c"length".as_ptr()) {
                crate::scan::scan_int();
                if cur_val() < 0 {
                    error_here_with_diagnostic(c"Bad dump length".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(2);
                    set_help_line(1, c"A dump length must be between 0 and 2^_31_-1,".as_ptr());
                    set_help_line(0, c"I changed this one to zero.".as_ptr());
                    int_error(cur_val());
                    set_cur_val(0);
                }
            }
            let j = cur_val();
            scan_pdf_ext_toks();
            if selector() == SELECTOR_NEW_STRING {
                crate::errors::pdf_error(
                    c"tokens".as_ptr(),
                    c"tokens_to_string() called while selector = new_string".as_ptr(),
                );
            }
            let old_sel = selector();
            set_selector(SELECTOR_NEW_STRING);
            crate::output::show_token_list(mem_b32s1(def_ref()), TEX_NULL, pool_size() - pool_ptr());
            set_selector(old_sel);
            s = crate::string_pool::make_string();
            crate::mem::delete_token_ref(def_ref());
            set_def_ref(save_dr);
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            b = pool_ptr();
            crate::texmfmp::getfiledump(s, i, j);
            set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks(b));
            if s == str_ptr() - 1 {
                set_str_ptr(str_ptr() - 1);
                set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
            }
            begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            return;
        }
        PDF_STRCMP_CODE_CT => {
            let save_ss = scanner_status();
            let save_wi = warning_index();
            let save_dr = def_ref();
            let u = if str_start(str_ptr() - TOO_BIG_CHAR) < pool_ptr() {
                crate::string_pool::make_string()
            } else {
                0
            };
            crate::scan::compare_strings();
            set_def_ref(save_dr);
            set_warning_index(save_wi);
            set_scanner_status(save_ss);
            if u != 0 {
                set_str_ptr(str_ptr() - 1);
            }
            // falls through to phase 2 (PDF_STRCMP_CODE_CT case there)
        }
        XETEX_UCHAR_CODE_CT => {
            crate::scan::scan_usv_num();
        }
        XETEX_UCHARCAT_CODE_CT => {
            crate::scan::scan_usv_num();
            let saved_chr = cur_val();
            crate::scan::scan_int();
            const LEFT_BRACE_CV: i32 = 1;
            const ACTIVE_CHAR_CV: i32 = 13;
            const OUT_PARAM_CV: i32 = 5;
            const IGNORE_CV: i32 = 9;
            if cur_val() < LEFT_BRACE_CV
                || cur_val() > ACTIVE_CHAR_CV
                || cur_val() == OUT_PARAM_CV
                || cur_val() == IGNORE_CV
            {
                error_here_with_diagnostic(c"Invalid code (".as_ptr());
                crate::output::print_int(cur_val());
                crate::output::print_cstr(c"), should be in the ranges 1..4, 6..8, 10..13".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(1);
                set_help_line(0, c"I'm going to use 12 instead of that illegal code value.".as_ptr());
                error();
                cat = 12;
            } else {
                cat = cur_val();
            }
            set_cur_val(saved_chr);
        }
        XETEX_REVISION_CODE_CT => {}
        XETEX_VARIATION_NAME_CODE_CT => {
            scan_font_ident();
            fnt = cur_val();
            if font_area_entry(fnt) == AAT_FONT_FLAG {
                crate::scan::scan_int();
                arg1 = cur_val();
                arg2 = 0;
            } else {
                not_aat_font_error(CONVERT as i32, c, fnt);
            }
        }
        XETEX_FEATURE_NAME_CODE_CT => {
            scan_font_ident();
            fnt = cur_val();
            let fa = font_area_entry(fnt);
            if fa == AAT_FONT_FLAG
                || (fa == OTGR_FONT_FLAG && usingGraphite(font_layout_engine_ptr(fnt)))
            {
                crate::scan::scan_int();
                arg1 = cur_val();
                arg2 = 0;
            } else {
                not_aat_gr_font_error(CONVERT as i32, c, fnt);
            }
        }
        XETEX_SELECTOR_NAME_CODE_CT => {
            scan_font_ident();
            fnt = cur_val();
            let fa = font_area_entry(fnt);
            if fa == AAT_FONT_FLAG
                || (fa == OTGR_FONT_FLAG && usingGraphite(font_layout_engine_ptr(fnt)))
            {
                crate::scan::scan_int();
                arg1 = cur_val();
                crate::scan::scan_int();
                arg2 = cur_val();
            } else {
                not_aat_gr_font_error(CONVERT as i32, c, fnt);
            }
        }
        XETEX_GLYPH_NAME_CODE_CT => {
            scan_font_ident();
            fnt = cur_val();
            let fa = font_area_entry(fnt);
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                crate::scan::scan_int();
                arg1 = cur_val();
            } else {
                not_native_font_error(CONVERT as i32, c, fnt);
            }
        }
        JOB_NAME_CODE_CT => {
            if job_name() == 0 {
                crate::scan::open_log_file();
            }
        }
        UNIFORM_DEVIATE_CODE_CT => {
            crate::scan::scan_int();
        }
        _ => {} // NORMAL_DEVIATE_CODE_CT and unrecognized codes
    }

    // ---- Phase 2: output ----
    let old_sel = selector();
    set_selector(SELECTOR_NEW_STRING);
    b = pool_ptr();

    match c {
        NUMBER_CODE_CT => {
            crate::output::print_int(cur_val());
        }
        ROMAN_NUMERAL_CODE_CT => {
            crate::output::print_roman_int(cur_val());
        }
        STRING_CODE_CT => {
            if cur_cs() != 0 {
                crate::output::sprint_cs(cur_cs());
            } else {
                crate::output::print_char(cur_chr() as i32);
            }
        }
        MEANING_CODE_CT => {
            crate::output::print_meaning();
        }
        FONT_NAME_CODE_CT => {
            let font_name_str = font_name_entry(cur_val());
            let fa = font_area_entry(cur_val());
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                let mut quote_char: i32 = '"' as i32;
                let base = str_start(font_name_str - TOO_BIG_CHAR);
                for i in 0..crate::string_pool::length(font_name_str) {
                    if str_pool_entry(base + i) == b'"' as u16 {
                        quote_char = '\'' as i32;
                    }
                }
                crate::output::print_char(quote_char);
                crate::output::print(font_name_str);
                crate::output::print_char(quote_char);
            } else {
                crate::output::print(font_name_str);
            }
            if font_size_entry(cur_val()) != font_dsize_entry(cur_val()) {
                crate::output::print_cstr(c" at ".as_ptr());
                crate::output::print_scaled(font_size_entry(cur_val()));
                crate::output::print_cstr(c"pt".as_ptr());
            }
        }
        ETEX_REVISION_CODE_CT => {
            crate::output::print_cstr(c".6".as_ptr());
        }
        LEFT_MARGIN_KERN_CODE_CT => {
            p = mem_b32s1(p + 5);
            while p != TEX_NULL {
                let tp = mem_b16s1(p);
                let is_low = p < hi_mem_min();
                let skip = (is_low && (
                    tp == INS_NODE_CT
                    || tp == MARK_NODE_CT
                    || tp == ADJUST_NODE_CT
                    || tp == PENALTY_NODE_CT
                    || (tp == DISC_NODE_CT
                        && mem_b32s0(p + 1) == TEX_NULL
                        && mem_b32s1(p + 1) == TEX_NULL
                        && mem_b16s0(p) == 0)
                    || (tp == MATH_NODE_CT && mem_b32s1(p + 1) == 0)
                    || (tp == KERN_NODE && (mem_b32s1(p + 1) == 0 || mem_b16s0(p) == 0))
                    || (tp == GLUE_NODE && mem_b32s0(p + 1) == 0)
                    || (tp == HLIST_NODE_CT
                        && mem_b32s1(p + 1) == 0
                        && mem_b32s1(p + 3) == 0
                        && mem_b32s1(p + 2) == 0
                        && mem_b32s1(p + 5) == TEX_NULL)
                )) || (is_low && tp == GLUE_NODE
                    && mem_b16s0(p) == (GLUE_PAR_LEFT_SKIP + 1) as u16);
                if !skip {
                    break;
                }
                p = mem_b32s1(p); // LLIST_link(p) = mem[p].b32.s1
            }
            if p != TEX_NULL && p < hi_mem_min() && mem_b16s1(p) == MARGIN_KERN_NODE_CT && mem_b16s0(p) == 0 {
                crate::output::print_scaled(mem_b32s1(p + 1));
            } else {
                crate::output::print('0' as i32);
            }
            crate::output::print_cstr(c"pt".as_ptr());
        }
        RIGHT_MARGIN_KERN_CODE_CT => {
            let q = mem_b32s1(p + 5);
            p = crate::mem::prev_rightmost(q, TEX_NULL);
            while p != TEX_NULL {
                let tp = mem_b16s1(p);
                let is_low = p < hi_mem_min();
                let skip = (is_low && (
                    tp == INS_NODE_CT
                    || tp == MARK_NODE_CT
                    || tp == ADJUST_NODE_CT
                    || tp == PENALTY_NODE_CT
                    || (tp == DISC_NODE_CT
                        && mem_b32s0(p + 1) == TEX_NULL
                        && mem_b32s1(p + 1) == TEX_NULL
                        && mem_b16s0(p) == 0)
                    || (tp == MATH_NODE_CT && mem_b32s1(p + 1) == 0)
                    || (tp == KERN_NODE && (mem_b32s1(p + 1) == 0 || mem_b16s0(p) == 0))
                    || (tp == GLUE_NODE && mem_b32s0(p + 1) == 0)
                    || (tp == HLIST_NODE_CT
                        && mem_b32s1(p + 1) == 0
                        && mem_b32s1(p + 3) == 0
                        && mem_b32s1(p + 2) == 0
                        && mem_b32s1(p + 5) == TEX_NULL)
                )) || (is_low && tp == GLUE_NODE
                    && mem_b16s0(p) == (GLUE_PAR_RIGHT_SKIP + 1) as u16);
                if !skip {
                    break;
                }
                p = crate::mem::prev_rightmost(q, p);
            }
            if p != TEX_NULL && p < hi_mem_min() && mem_b16s1(p) == MARGIN_KERN_NODE_CT && mem_b16s0(p) == 1 {
                crate::output::print_scaled(mem_b32s1(p + 1));
            } else {
                crate::output::print('0' as i32);
            }
            crate::output::print_cstr(c"pt".as_ptr());
        }
        PDF_STRCMP_CODE_CT => {
            crate::output::print_int(cur_val());
        }
        UNIFORM_DEVIATE_CODE_CT => {
            crate::output::print_int(crate::scaled_math::unif_rand(cur_val()));
        }
        NORMAL_DEVIATE_CODE_CT => {
            crate::output::print_int(crate::scaled_math::norm_rand());
        }
        XETEX_UCHAR_CODE_CT | XETEX_UCHARCAT_CODE_CT => {
            crate::output::print_char(cur_val());
        }
        XETEX_REVISION_CODE_CT => {
            crate::output::print_cstr(c".999994".as_ptr());
        }
        XETEX_VARIATION_NAME_CODE_CT => {
            if font_area_entry(fnt) == AAT_FONT_FLAG {
                aat_print_font_name(c, font_layout_engine_ptr(fnt), arg1, arg2);
            }
        }
        XETEX_FEATURE_NAME_CODE_CT | XETEX_SELECTOR_NAME_CODE_CT => {
            let fa = font_area_entry(fnt);
            if fa == AAT_FONT_FLAG {
                aat_print_font_name(c, font_layout_engine_ptr(fnt), arg1, arg2);
            } else if fa == OTGR_FONT_FLAG && usingGraphite(font_layout_engine_ptr(fnt)) {
                gr_print_font_name(c, font_layout_engine_ptr(fnt), arg1, arg2);
            }
        }
        XETEX_GLYPH_NAME_CODE_CT => {
            let fa = font_area_entry(fnt);
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                print_glyph_name(fnt, arg1);
            }
        }
        JOB_NAME_CODE_CT => {
            crate::output::print_file_name(job_name(), 0, 0);
        }
        _ => {}
    }

    set_selector(old_sel);
    set_mem_b32s1(GARBAGE_CT, crate::scan::str_toks_cat(b, cat as i16));
    begin_token_list(mem_b32s1(TEMP_HEAD_CT), INSERTED);
}

// ---------------------------------------------------------------------------
// show_box / short_display_n
// ---------------------------------------------------------------------------

// INT_BASE + INT_PAR__show_box_depth/breadth (xetex_format.h:219, :59–60).
const INT_BASE_SB: i32 = 7_826_729;
const INT_PAR_SHOW_BOX_BREADTH: i32 = 24;
const INT_PAR_SHOW_BOX_DEPTH: i32   = 25;

/// translated from xetex-xetex0.c:735 (show_box)
#[no_mangle]
pub unsafe extern "C" fn show_box(p: i32) {
    use crate::globals::{
        eqtb_b32s1, pool_ptr, pool_size,
        set_depth_threshold, set_breadth_max, breadth_max,
    };
    set_depth_threshold(eqtb_b32s1(INT_BASE_SB + INT_PAR_SHOW_BOX_DEPTH));
    set_breadth_max(eqtb_b32s1(INT_BASE_SB + INT_PAR_SHOW_BOX_BREADTH));
    if breadth_max() <= 0 {
        set_breadth_max(5);
    }
    let dt = pool_size() - pool_ptr() - 1;
    if crate::globals::depth_threshold() > dt {
        set_depth_threshold(dt);
    }
    show_node_list(p);
    crate::output::print_ln();
}

/// translated from xetex-xetex0.c:748 (short_display_n)
#[no_mangle]
pub unsafe extern "C" fn short_display_n(p: i32, m: i32) {
    use crate::globals::{pool_ptr, pool_size, set_breadth_max, set_depth_threshold};
    set_breadth_max(m);
    set_depth_threshold(pool_size() - pool_ptr() - 1);
    show_node_list(p);
}

// ---------------------------------------------------------------------------
// flush_node_list
// ---------------------------------------------------------------------------

// Node-type constants for flush_node_list (xetex_format.h / xetex-constants.h).
const HLIST_NODE_FNL: u16  =  0;
const VLIST_NODE_FNL: u16  =  1;
const RULE_NODE_FNL: u16   =  2;
const INS_NODE_FNL: u16    =  3;
const MARK_NODE_FNL: u16   =  4;
const ADJUST_NODE_FNL: u16 =  5;
const LIGATURE_NODE_FNL: u16 = 6;
const DISC_NODE_FNL: u16   =  7;
const WHATSIT_NODE_FNL: u16 = 8;
const MATH_NODE_FNL: u16   =  9;
const GLUE_NODE_FNL: u16   = 10;
const KERN_NODE_FNL: u16   = 11;
const PENALTY_NODE_FNL: u16 = 12;
const UNSET_NODE_FNL: u16  = 13;
const STYLE_NODE_FNL: u16  = 14;
const CHOICE_NODE_FNL: u16 = 15;
const ORD_NOAD_FNL: u16    = 16;
const OP_NOAD_FNL: u16     = 17;
const BIN_NOAD_FNL: u16    = 18;
const REL_NOAD_FNL: u16    = 19;
const OPEN_NOAD_FNL: u16   = 20;
const CLOSE_NOAD_FNL: u16  = 21;
const PUNCT_NOAD_FNL: u16  = 22;
const INNER_NOAD_FNL: u16  = 23;
const RADICAL_NOAD_FNL: u16 = 24;
const FRACTION_NOAD_FNL: u16 = 25;
const UNDER_NOAD_FNL: u16  = 26;
const OVER_NOAD_FNL: u16   = 27;
const ACCENT_NOAD_FNL: u16 = 28;
const VCENTER_NOAD_FNL: u16 = 29;
const LEFT_NOAD_FNL: u16   = 30;
const RIGHT_NOAD_FNL: u16  = 31;
const MARGIN_KERN_NODE_FNL: u16 = 40; // checked below
// Whatsit subtypes (xetex_format.h:457–467).
const OPEN_NODE_FNL: u16       =  0;
const WRITE_NODE_FNL: u16      =  1;
const CLOSE_NODE_FNL: u16      =  2;
const SPECIAL_NODE_FNL: u16    =  3;
const LANGUAGE_NODE_FNL: u16   =  4;
const PDF_SAVE_POS_NODE_FNL: u16 = 21;
const NATIVE_WORD_NODE_FNL: u16    = 40;
const NATIVE_WORD_NODE_AT_FNL: u16 = 41;
const GLYPH_NODE_FNL: u16      = 42;
const PIC_NODE_FNL: u16        = 43;
const PDF_NODE_FNL: u16        = 44;
// Node sizes (xetex-constants.h).
const BOX_NODE_SIZE_FNL: i32   = 8;
const RULE_NODE_SIZE_FNL: i32  = 5;
const INS_NODE_SIZE_FNL: i32   = 5;
const OPEN_NODE_SIZE_FNL: i32  = 3;
const WRITE_NODE_SIZE_FNL: i32 = 2;
const SMALL_NODE_SIZE_FNL: i32 = 2;
const MEDIUM_NODE_SIZE_FNL: i32 = 3;
const GLYPH_NODE_SIZE_FNL: i32 = 5;
const PIC_NODE_SIZE_FNL: i32   = 9;
const MARGIN_KERN_NODE_SIZE_FNL: i32 = 3;
const GLUE_SPEC_SIZE_FNL: i32  = 4;
const STYLE_NODE_SIZE_FNL: i32 = 3;
const NOAD_SIZE_FNL: i32       = 4;
const RADICAL_NOAD_SIZE_FNL: i32 = 5;
const ACCENT_NOAD_SIZE_FNL: i32  = 5;
const FRACTION_NOAD_SIZE_FNL: i32 = 6;
const SUB_BOX_FNL: i32 = 2; // xetex-constants.h:283
// sizeof(memory_word) = 8 on 64-bit platforms.
const MEMORY_WORD_SIZE_FNL: i32 = 8;

/// translated from xetex-xetex0.c:765 (flush_node_list)
#[no_mangle]
pub unsafe extern "C" fn flush_node_list(mut p: i32) {
    use crate::globals::{
        hi_mem_min, avail, set_avail,
        mem_b16s0, mem_b16s1, mem_b16s3,
        mem_b32s0, mem_b32s1, set_mem_b32s1, set_mem_b16s0,
        mem_ptr, set_mem_ptr,
    };
    while p != crate::mem::TEX_NULL {
        let q = mem_b32s1(p);
        if p >= hi_mem_min() {
            // is_char_node(p): free to avail list
            set_mem_b32s1(p, avail());
            set_avail(p);
        } else {
            // Goto-done pattern: break 'done skips free_node(p, SMALL_NODE_SIZE) at end.
            'done: {
                match mem_b16s1(p) {
                    HLIST_NODE_FNL | VLIST_NODE_FNL | UNSET_NODE_FNL => {
                        flush_node_list(mem_b32s1(p + 5));
                        crate::mem::free_node(p, BOX_NODE_SIZE_FNL);
                        break 'done;
                    }
                    RULE_NODE_FNL => {
                        crate::mem::free_node(p, RULE_NODE_SIZE_FNL);
                        break 'done;
                    }
                    INS_NODE_FNL => {
                        flush_node_list(mem_b32s0(p + 4));
                        crate::mem::delete_glue_ref(mem_b32s1(p + 4));
                        crate::mem::free_node(p, INS_NODE_SIZE_FNL);
                        break 'done;
                    }
                    WHATSIT_NODE_FNL => {
                        match mem_b16s0(p) {
                            OPEN_NODE_FNL => {
                                crate::mem::free_node(p, OPEN_NODE_SIZE_FNL);
                            }
                            WRITE_NODE_FNL | SPECIAL_NODE_FNL => {
                                crate::mem::delete_token_ref(mem_b32s1(p + 1));
                                crate::mem::free_node(p, WRITE_NODE_SIZE_FNL);
                                break 'done;
                            }
                            CLOSE_NODE_FNL | LANGUAGE_NODE_FNL => {
                                crate::mem::free_node(p, SMALL_NODE_SIZE_FNL);
                            }
                            NATIVE_WORD_NODE_FNL | NATIVE_WORD_NODE_AT_FNL => {
                                let glyph_ptr = mem_ptr(p + 5);
                                if !glyph_ptr.is_null() {
                                    libc::free(glyph_ptr);
                                    set_mem_ptr(p + 5, std::ptr::null_mut());
                                    set_mem_b16s0(p + 4, 0);
                                }
                                let sz = mem_b16s3(p + 4) as i32;
                                crate::mem::free_node(p, sz);
                            }
                            GLYPH_NODE_FNL => {
                                crate::mem::free_node(p, GLYPH_NODE_SIZE_FNL);
                            }
                            PIC_NODE_FNL | PDF_NODE_FNL => {
                                let path_len = mem_b16s1(p + 4) as i32;
                                let sz = PIC_NODE_SIZE_FNL
                                    + (path_len + MEMORY_WORD_SIZE_FNL - 1) / MEMORY_WORD_SIZE_FNL;
                                crate::mem::free_node(p, sz);
                            }
                            PDF_SAVE_POS_NODE_FNL => {
                                crate::mem::free_node(p, SMALL_NODE_SIZE_FNL);
                            }
                            _ => {
                                crate::errors::confusion(c"ext3".as_ptr());
                            }
                        }
                        break 'done;
                    }
                    GLUE_NODE_FNL => {
                        let spec = mem_b32s0(p + 1);
                        if mem_b32s1(spec) == crate::mem::TEX_NULL {
                            crate::mem::free_node(spec, GLUE_SPEC_SIZE_FNL);
                        } else {
                            set_mem_b32s1(spec, mem_b32s1(spec) - 1);
                        }
                        if mem_b32s1(p + 1) != crate::mem::TEX_NULL {
                            flush_node_list(mem_b32s1(p + 1));
                        }
                        crate::mem::free_node(p, MEDIUM_NODE_SIZE_FNL);
                        break 'done;
                    }
                    KERN_NODE_FNL | MATH_NODE_FNL | PENALTY_NODE_FNL => {
                        crate::mem::free_node(p, MEDIUM_NODE_SIZE_FNL);
                        break 'done;
                    }
                    MARGIN_KERN_NODE_FNL => {
                        crate::mem::free_node(p, MARGIN_KERN_NODE_SIZE_FNL);
                        break 'done;
                    }
                    LIGATURE_NODE_FNL => {
                        flush_node_list(mem_b32s1(p + 1));
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    MARK_NODE_FNL => {
                        crate::mem::delete_token_ref(mem_b32s1(p + 1));
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    DISC_NODE_FNL => {
                        flush_node_list(mem_b32s0(p + 1));
                        flush_node_list(mem_b32s1(p + 1));
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    ADJUST_NODE_FNL => {
                        flush_node_list(mem_b32s1(p + 1));
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    STYLE_NODE_FNL => {
                        crate::mem::free_node(p, STYLE_NODE_SIZE_FNL);
                        break 'done;
                    }
                    CHOICE_NODE_FNL => {
                        flush_node_list(mem_b32s0(p + 1));
                        flush_node_list(mem_b32s1(p + 1));
                        flush_node_list(mem_b32s0(p + 2));
                        flush_node_list(mem_b32s1(p + 2));
                        crate::mem::free_node(p, STYLE_NODE_SIZE_FNL);
                        break 'done;
                    }
                    ORD_NOAD_FNL | OP_NOAD_FNL | BIN_NOAD_FNL | REL_NOAD_FNL
                    | OPEN_NOAD_FNL | CLOSE_NOAD_FNL | PUNCT_NOAD_FNL | INNER_NOAD_FNL
                    | RADICAL_NOAD_FNL | OVER_NOAD_FNL | UNDER_NOAD_FNL | VCENTER_NOAD_FNL
                    | ACCENT_NOAD_FNL => {
                        if mem_b32s1(p + 1) >= SUB_BOX_FNL {
                            flush_node_list(mem_b32s0(p + 1));
                        }
                        if mem_b32s1(p + 2) >= SUB_BOX_FNL {
                            flush_node_list(mem_b32s0(p + 2));
                        }
                        if mem_b32s1(p + 3) >= SUB_BOX_FNL {
                            flush_node_list(mem_b32s0(p + 3));
                        }
                        let ntype = mem_b16s1(p);
                        if ntype == RADICAL_NOAD_FNL {
                            crate::mem::free_node(p, RADICAL_NOAD_SIZE_FNL);
                        } else if ntype == ACCENT_NOAD_FNL {
                            crate::mem::free_node(p, ACCENT_NOAD_SIZE_FNL);
                        } else {
                            crate::mem::free_node(p, NOAD_SIZE_FNL);
                        }
                        break 'done;
                    }
                    LEFT_NOAD_FNL | RIGHT_NOAD_FNL => {
                        crate::mem::free_node(p, NOAD_SIZE_FNL);
                        break 'done;
                    }
                    FRACTION_NOAD_FNL => {
                        flush_node_list(mem_b32s0(p + 2));
                        flush_node_list(mem_b32s0(p + 3));
                        crate::mem::free_node(p, FRACTION_NOAD_SIZE_FNL);
                        break 'done;
                    }
                    _ => {
                        crate::errors::confusion(c"flushing".as_ptr());
                    }
                }
                crate::mem::free_node(p, SMALL_NODE_SIZE_FNL);
            }
        }
        p = q;
    }
}

// ---------------------------------------------------------------------------
// copy_node_list
// ---------------------------------------------------------------------------

// Node-type constants (xetex_format.h / xetex-constants.h).
const HLIST_NODE_CNL: u16  =  0;
const VLIST_NODE_CNL: u16  =  1;
const RULE_NODE_CNL: u16   =  2;
const INS_NODE_CNL: u16    =  3;
const UNSET_NODE_CNL: u16  = 13;
const WHATSIT_NODE_CNL: u16 = 8;
const GLUE_NODE_CNL: u16   = 10;
const KERN_NODE_CNL: u16   = 11;
const MATH_NODE_CNL: u16   =  9;
const PENALTY_NODE_CNL: u16 = 12;
const MARGIN_KERN_NODE_CNL: u16 = 40;
const LIGATURE_NODE_CNL: u16 = 6;
const DISC_NODE_CNL: u16   =  7;
const MARK_NODE_CNL: u16   =  4;
const ADJUST_NODE_CNL: u16 =  5;
// Whatsit subtypes.
const OPEN_NODE_CNL: u16        =  0;
const WRITE_NODE_CNL: u16       =  1;
const SPECIAL_NODE_CNL: u16     =  3;
const CLOSE_NODE_CNL: u16       =  2;
const LANGUAGE_NODE_CNL: u16    =  4;
const NATIVE_WORD_NODE_CNL: u16    = 40;
const NATIVE_WORD_NODE_AT_CNL: u16 = 41;
const GLYPH_NODE_CNL: u16       = 42;
const PIC_NODE_CNL: u16         = 43;
const PDF_NODE_CNL: u16         = 44;
const PDF_SAVE_POS_NODE_CNL: u16 = 21;
// Node sizes.
const BOX_NODE_SIZE_CNL: i32   = 8;
const RULE_NODE_SIZE_CNL: i32  = 5;
const INS_NODE_SIZE_CNL: i32   = 5;
const OPEN_NODE_SIZE_CNL: i32  = 3;
const WRITE_NODE_SIZE_CNL: i32 = 2;
const SMALL_NODE_SIZE_CNL: i32 = 2;
const MEDIUM_NODE_SIZE_CNL: i32 = 3;
const GLYPH_NODE_SIZE_CNL: i32 = 5;
const PIC_NODE_SIZE_CNL: i32   = 9;
const MARGIN_KERN_NODE_SIZE_CNL: i32 = 3;
const MEMORY_WORD_SIZE_CNL: i32 = 8;
// SYNCTEX_FIELD_SIZE = 1, so SYNCTEX_{tag,line}(p, BOX_NODE_SIZE) = mem[p+7].b32.{s0,s1}

/// translated from xetex-xetex0.c:953 (copy_node_list)
#[no_mangle]
pub unsafe extern "C" fn copy_node_list(mut p: i32) -> i32 {
    use crate::globals::{
        avail, set_avail, hi_mem_min,
        mem_b16s0, mem_b16s1, mem_b16s3,
        mem_b32s0, mem_b32s1, set_mem_b32s0, set_mem_b32s1,
        mem_ptr, set_mem_ptr, set_mem_b16s0,
        mem_word, set_mem_word,
    };
    let h = crate::mem::get_avail();
    let mut q = h;
    while p != crate::mem::TEX_NULL {
        let mut words = 1i32;
        let mut r = 0i32;
        if p >= hi_mem_min() {
            r = crate::mem::get_avail();
        } else {
            match mem_b16s1(p) {
                HLIST_NODE_CNL | VLIST_NODE_CNL | UNSET_NODE_CNL => {
                    r = crate::mem::get_node(BOX_NODE_SIZE_CNL);
                    // SYNCTEX_tag(r, BOX_NODE_SIZE) = SYNCTEX_tag(p, BOX_NODE_SIZE)
                    set_mem_b32s0(r + 7, mem_b32s0(p + 7));
                    // SYNCTEX_line(r, BOX_NODE_SIZE) = SYNCTEX_line(p, BOX_NODE_SIZE)
                    set_mem_b32s1(r + 7, mem_b32s1(p + 7));
                    set_mem_word(r + 6, mem_word(p + 6));
                    set_mem_word(r + 5, mem_word(p + 5));
                    set_mem_b32s1(r + 5, copy_node_list(mem_b32s1(p + 5)));
                    words = 5;
                }
                RULE_NODE_CNL => {
                    r = crate::mem::get_node(RULE_NODE_SIZE_CNL);
                    words = RULE_NODE_SIZE_CNL - 1;
                }
                INS_NODE_CNL => {
                    r = crate::mem::get_node(INS_NODE_SIZE_CNL);
                    set_mem_word(r + 4, mem_word(p + 4));
                    // GLUE_SPEC_ref_count(mem[p+4].b32.s1)++ = mem[mem[p+4].b32.s1].b32.s1++
                    let spec = mem_b32s1(p + 4);
                    set_mem_b32s1(spec, mem_b32s1(spec) + 1);
                    set_mem_b32s0(r + 4, copy_node_list(mem_b32s0(p + 4)));
                    words = INS_NODE_SIZE_CNL - 1;
                }
                WHATSIT_NODE_CNL => {
                    match mem_b16s0(p) {
                        OPEN_NODE_CNL => {
                            r = crate::mem::get_node(OPEN_NODE_SIZE_CNL);
                            words = OPEN_NODE_SIZE_CNL;
                        }
                        WRITE_NODE_CNL | SPECIAL_NODE_CNL => {
                            // mem[mem[p+1].b32.s1].b32.s0++ (token list ref count)
                            let tok = mem_b32s1(p + 1);
                            set_mem_b32s0(tok, mem_b32s0(tok) + 1);
                            r = crate::mem::get_node(WRITE_NODE_SIZE_CNL);
                            words = WRITE_NODE_SIZE_CNL;
                        }
                        CLOSE_NODE_CNL | LANGUAGE_NODE_CNL => {
                            r = crate::mem::get_node(SMALL_NODE_SIZE_CNL);
                            words = SMALL_NODE_SIZE_CNL;
                        }
                        NATIVE_WORD_NODE_CNL | NATIVE_WORD_NODE_AT_CNL => {
                            let sz = mem_b16s3(p + 4) as i32;
                            r = crate::mem::get_node(sz);
                            let mut w = sz;
                            while w > 0 {
                                w -= 1;
                                set_mem_word(r + w, mem_word(p + w));
                            }
                            set_mem_ptr(r + 5, std::ptr::null_mut());
                            set_mem_b16s0(r + 4, 0);
                            crate::mem::copy_native_glyph_info(p, r);
                            words = 0;
                        }
                        GLYPH_NODE_CNL => {
                            r = crate::mem::get_node(GLYPH_NODE_SIZE_CNL);
                            words = GLYPH_NODE_SIZE_CNL;
                        }
                        PIC_NODE_CNL | PDF_NODE_CNL => {
                            let path_len = mem_b16s1(p + 4) as i32;
                            let sz = PIC_NODE_SIZE_CNL
                                + (path_len + MEMORY_WORD_SIZE_CNL - 1) / MEMORY_WORD_SIZE_CNL;
                            r = crate::mem::get_node(sz);
                            words = sz;
                        }
                        PDF_SAVE_POS_NODE_CNL => {
                            r = crate::mem::get_node(SMALL_NODE_SIZE_CNL);
                        }
                        _ => { crate::errors::confusion(c"ext2".as_ptr()); }
                    }
                }
                GLUE_NODE_CNL => {
                    r = crate::mem::get_node(MEDIUM_NODE_SIZE_CNL);
                    // GLUE_SPEC_ref_count(mem[p+1].b32.s0)++ = mem[mem[p+1].b32.s0].b32.s1++
                    let spec = mem_b32s0(p + 1);
                    set_mem_b32s1(spec, mem_b32s1(spec) + 1);
                    set_mem_b32s0(r + 2, mem_b32s0(p + 2));
                    set_mem_b32s1(r + 2, mem_b32s1(p + 2));
                    set_mem_b32s0(r + 1, mem_b32s0(p + 1));
                    set_mem_b32s1(r + 1, copy_node_list(mem_b32s1(p + 1)));
                }
                KERN_NODE_CNL | MATH_NODE_CNL | PENALTY_NODE_CNL => {
                    r = crate::mem::get_node(MEDIUM_NODE_SIZE_CNL);
                    words = MEDIUM_NODE_SIZE_CNL;
                }
                MARGIN_KERN_NODE_CNL => {
                    r = crate::mem::get_node(MARGIN_KERN_NODE_SIZE_CNL);
                    words = MARGIN_KERN_NODE_SIZE_CNL;
                }
                LIGATURE_NODE_CNL => {
                    r = crate::mem::get_node(SMALL_NODE_SIZE_CNL);
                    set_mem_word(r + 1, mem_word(p + 1));
                    set_mem_b32s1(r + 1, copy_node_list(mem_b32s1(p + 1)));
                }
                DISC_NODE_CNL => {
                    r = crate::mem::get_node(SMALL_NODE_SIZE_CNL);
                    set_mem_b32s0(r + 1, copy_node_list(mem_b32s0(p + 1)));
                    set_mem_b32s1(r + 1, copy_node_list(mem_b32s1(p + 1)));
                }
                MARK_NODE_CNL => {
                    r = crate::mem::get_node(SMALL_NODE_SIZE_CNL);
                    // mem[mem[p+1].b32.s1].b32.s0++ (token list ref count)
                    let tok = mem_b32s1(p + 1);
                    set_mem_b32s0(tok, mem_b32s0(tok) + 1);
                    words = SMALL_NODE_SIZE_CNL;
                }
                ADJUST_NODE_CNL => {
                    r = crate::mem::get_node(SMALL_NODE_SIZE_CNL);
                    set_mem_b32s1(r + 1, copy_node_list(mem_b32s1(p + 1)));
                }
                _ => { crate::errors::confusion(c"copying".as_ptr()); }
            }
        }
        let mut w = words;
        while w > 0 {
            w -= 1;
            set_mem_word(r + w, mem_word(p + w));
        }
        set_mem_b32s1(q, r);
        q = r;
        p = mem_b32s1(p);
    }
    set_mem_b32s1(q, crate::mem::TEX_NULL);
    let result = mem_b32s1(h);
    set_mem_b32s1(h, avail());
    set_avail(h);
    result
}

// ---------------------------------------------------------------------------
// short_display
// ---------------------------------------------------------------------------

// Node-type constants (xetex_format.h:380–390).
const HLIST_NODE_SD: u16  =  0;
const VLIST_NODE_SD: u16  =  1;
const RULE_NODE_SD: u16   =  2;
const INS_NODE_SD: u16    =  3;
const MARK_NODE_SD: u16   =  4;
const ADJUST_NODE_SD: u16 =  5;
const DISC_NODE_SD: u16   =  7;
const WHATSIT_NODE_SD: u16 = 8;
const MATH_NODE_SD: u16   =  9;
const GLUE_NODE_SD: u16   = 10;
const LIGATURE_NODE_SD: u16 = 6;
const UNSET_NODE_SD: u16  = 13;
const NATIVE_WORD_NODE_SD: u16    = 40;
const NATIVE_WORD_NODE_AT_SD: u16 = 41;
const L_CODE_SD: u16 = 4;            // xetex-constants.h:142
const FONT_ID_BASE_SD: i32 = 2_245_338; // xetex_format.h:188

/// translated from xetex-xetex0.c:117 (short_display)
#[no_mangle]
pub unsafe extern "C" fn short_display(mut p: i32) {
    use crate::globals::{
        mem_end, hi_mem_min, font_max,
        mem_b16s0, mem_b16s1, mem_b16s2,
        mem_b32s0, mem_b32s1,
        hash_s1,
        font_in_short_display, set_font_in_short_display,
    };
    while p > 0 {
        if p >= hi_mem_min() {
            // is_char_node(p)
            if p <= mem_end() {
                let fnt = mem_b16s1(p) as i32;
                if fnt != font_in_short_display() {
                    if fnt > font_max() {
                        crate::output::print_char('*' as i32);
                    } else {
                        crate::output::print_esc(hash_s1(FONT_ID_BASE_SD + fnt));
                    }
                    crate::output::print_char(' ' as i32);
                    set_font_in_short_display(fnt);
                }
                crate::output::print(mem_b16s0(p) as i32);
            }
        } else {
            match mem_b16s1(p) {
                HLIST_NODE_SD | VLIST_NODE_SD | INS_NODE_SD | MARK_NODE_SD
                | ADJUST_NODE_SD | UNSET_NODE_SD => {
                    crate::output::print_cstr(c"[]".as_ptr());
                }
                WHATSIT_NODE_SD => match mem_b16s0(p) {
                    NATIVE_WORD_NODE_SD | NATIVE_WORD_NODE_AT_SD => {
                        let fnt = mem_b16s2(p + 4) as i32;
                        if fnt != font_in_short_display() {
                            crate::output::print_esc(hash_s1(FONT_ID_BASE_SD + fnt));
                            crate::output::print_char(' ' as i32);
                            set_font_in_short_display(fnt);
                        }
                        crate::output::print_native_word(p);
                    }
                    _ => {
                        crate::output::print_cstr(c"[]".as_ptr());
                    }
                },
                RULE_NODE_SD => {
                    crate::output::print_char('|' as i32);
                }
                GLUE_NODE_SD => {
                    if mem_b32s0(p + 1) != 0 {
                        crate::output::print_char(' ' as i32);
                    }
                }
                MATH_NODE_SD => {
                    if mem_b16s0(p) >= L_CODE_SD {
                        crate::output::print_cstr(c"[]".as_ptr());
                    } else {
                        crate::output::print_char('$' as i32);
                    }
                }
                LIGATURE_NODE_SD => {
                    short_display(mem_b32s1(p + 1));
                }
                DISC_NODE_SD => {
                    short_display(mem_b32s0(p + 1));
                    short_display(mem_b32s1(p + 1));
                    let mut n = mem_b16s0(p) as i32;
                    while n > 0 {
                        if mem_b32s1(p) != crate::mem::TEX_NULL {
                            p = mem_b32s1(p);
                        }
                        n -= 1;
                    }
                }
                _ => {}
            }
        }
        p = mem_b32s1(p);
    }
}

// ---------------------------------------------------------------------------
// check_outer_validity
// ---------------------------------------------------------------------------

// Scanner-status discriminants (xetex-constants.h:182–187).
const NORMAL_COV: u8  = 0;
const SKIPPING_COV: u8 = 1;
const DEFINING_COV: u8 = 2;
const MATCHING_COV: u8 = 3;
const ALIGNING_COV: u8 = 4;
const ABSORBING_COV: u8 = 5;
// Token constants (xetex-constants.h:351; xetex_format.h:357; xetex-constants.h:59).
const RIGHT_BRACE_TOKEN_COV: i32 = 0x40_0000; // RIGHT_BRACE << 21
const OUTER_CALL_COV: u8 = 115;               // xetex_format.h:357
const FROZEN_CR_COV: i32  = 2_243_226 + 1;    // FROZEN_CONTROL_SEQUENCE + 1
const FROZEN_FI_COV: i32  = 2_243_226 + 4;    // FROZEN_CONTROL_SEQUENCE + 4
const CS_TOKEN_FLAG_COV: i32 = 0x1FF_FFFF;
const IF_TEST_COV: u16 = 107;                 // xetex_format.h:349
// Input-state discriminant (already TOKEN_LIST = 0 in input.rs).
const TOKEN_LIST_COV: u16 = 0;

/// translated from xetex-xetex0.c:3036 (check_outer_validity)
#[no_mangle]
pub unsafe extern "C" fn check_outer_validity() {
    use crate::globals::{
        scanner_status, set_deletions_allowed,
        cur_cs, set_cur_cs, cur_input,
        set_cur_cmd, set_cur_chr,
        par_token, set_long_state,
        set_align_state,
        set_mem_b32s0, set_mem_b32s1,
        set_cur_tok,
        warning_index,
        set_help_ptr, set_help_line,
        cur_if, skip_line,
        error_line,
    };
    if scanner_status() == NORMAL_COV {
        return;
    }
    set_deletions_allowed(false);

    // If cur_cs is a control sequence, push it back as a backed-up token.
    if cur_cs() != 0 {
        let ci = cur_input();
        if ci.state == TOKEN_LIST_COV || ci.name < 1 || ci.name > 17 {
            let p = crate::mem::get_avail();
            set_mem_b32s0(p, CS_TOKEN_FLAG_COV + cur_cs());
            crate::input::begin_token_list(p, 3 /* BACKED_UP */);
        }
        set_cur_cmd(SPACER);
        set_cur_chr(' ' as i32);
    }

    if scanner_status() > SKIPPING_COV {
        // scanner_status > SKIPPING: runaway — we hit EOF or OUTER inside a definition/macro/preamble.
        runaway();
        if cur_cs() == 0 {
            error_here_with_diagnostic(c"File ended".as_ptr());
        } else {
            set_cur_cs(0);
            error_here_with_diagnostic(c"Forbidden control sequence found".as_ptr());
        }

        let p = crate::mem::get_avail();
        match scanner_status() {
            DEFINING_COV => {
                crate::output::print_cstr(c" while scanning definition".as_ptr());
                set_mem_b32s0(p, RIGHT_BRACE_TOKEN_COV + b'}' as i32);
            }
            MATCHING_COV => {
                crate::output::print_cstr(c" while scanning use".as_ptr());
                set_mem_b32s0(p, par_token());
                set_long_state(OUTER_CALL_COV);
            }
            ALIGNING_COV => {
                crate::output::print_cstr(c" while scanning preamble".as_ptr());
                set_mem_b32s0(p, RIGHT_BRACE_TOKEN_COV + b'}' as i32);
                let q = p;
                let p2 = crate::mem::get_avail();
                set_mem_b32s1(p2, q);
                set_mem_b32s0(p2, CS_TOKEN_FLAG_COV + FROZEN_CR_COV);
                set_align_state(-1_000_000);
                crate::input::begin_token_list(p2, 5 /* INSERTED */);
                // For ALIGNING, we push p2 as the token list, not p.
                // Then fall through to the help message.
                crate::output::print_cstr(c" of ".as_ptr());
                crate::output::sprint_cs(warning_index());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(4);
                set_help_line(3, c"I suspect you have forgotten a `}', causing me".as_ptr());
                set_help_line(2, c"to read past where you wanted me to stop.".as_ptr());
                set_help_line(1, c"I'll try to recover; but if the error is serious,".as_ptr());
                set_help_line(0, c"you'd better type `E' or `X' now and fix your file.".as_ptr());
                crate::errors::error();
                set_deletions_allowed(true);
                return;
            }
            ABSORBING_COV => {
                crate::output::print_cstr(c" while scanning text".as_ptr());
                set_mem_b32s0(p, RIGHT_BRACE_TOKEN_COV + b'}' as i32);
            }
            _ => {}
        }

        crate::input::begin_token_list(p, 5 /* INSERTED */);
        crate::output::print_cstr(c" of ".as_ptr());
        crate::output::sprint_cs(warning_index());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(4);
        set_help_line(3, c"I suspect you have forgotten a `}', causing me".as_ptr());
        set_help_line(2, c"to read past where you wanted me to stop.".as_ptr());
        set_help_line(1, c"I'll try to recover; but if the error is serious,".as_ptr());
        set_help_line(0, c"you'd better type `E' or `X' now and fix your file.".as_ptr());
        crate::errors::error();
    } else {
        // scanner_status == SKIPPING: inside \if...\fi skip.
        error_here_with_diagnostic(c"Incomplete ".as_ptr());
        print_cmd_chr(IF_TEST_COV, cur_if() as i32);
        crate::output::print_cstr(c"; all text was ignored after line ".as_ptr());
        crate::output::print_int(skip_line());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(3);
        set_help_line(2, c"A forbidden control sequence occurred in skipped text.".as_ptr());
        set_help_line(1, c"This kind of error happens when you say `\\if...' and forget".as_ptr());
        set_help_line(0, c"the matching `\\fi'. I've inserted a `\\fi'; this might work.".as_ptr());
        if cur_cs() != 0 {
            set_cur_cs(0);
        } else {
            set_help_line(2, c"The file ended while I was skipping conditional text.".as_ptr());
        }
        set_cur_tok(CS_TOKEN_FLAG_COV + FROZEN_FI_COV);
        crate::input::ins_error();
    }
    set_deletions_allowed(true);
}

// ---------------------------------------------------------------------------
// show_activities
// ---------------------------------------------------------------------------

// MEM_TOP = 4999999; PAGE_INS_HEAD = MEM_TOP, CONTRIB_HEAD = MEM_TOP-1, PAGE_HEAD = MEM_TOP-2
const MEM_TOP_SA: i32 = 4_999_999;
const PAGE_INS_HEAD_SA: i32 = MEM_TOP_SA;
const CONTRIB_HEAD_SA: i32 = MEM_TOP_SA - 1;
const PAGE_HEAD_SA: i32 = MEM_TOP_SA - 2;
const MAX_COMMAND_SA: i16 = 102;  // xetex-constants.h:325
const HMODE_SA: i16 = 104;        // xetex-constants.h:369
const IGNORE_DEPTH_SA: i32 = -65_536_000; // xetex-constants.h:388
const EMPTY_SA: u8 = 0;           // xetex-constants.h:230
const SPLIT_UP_SA: u16 = 1;       // xetex-constants.h:274
const INS_NODE_SA: u16 = 3;       // xetex_format.h:383
const COUNT_BASE_SA: i32 = 7_826_812; // xetex-constants.h:220

/// translated from xetex-xetex0.c:1123 (show_activities)
#[no_mangle]
pub unsafe extern "C" fn show_activities() {
    use crate::globals::{
        save_cur_list_to_nest, nest_ptr, nest_mode, nest_head, nest_mode_line,
        nest_aux_b32s0, nest_aux_b32s1, nest_prev_graf,
        page_tail, page_contents, page_so_far, output_active,
        mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1, eqtb_b32s1,
    };
    save_cur_list_to_nest(nest_ptr());
    crate::output::print_nl_cstr(c"".as_ptr());
    crate::output::print_ln();
    let np = nest_ptr();
    for p in (0..=np).rev() {
        let m = nest_mode(p);
        let a_s0 = nest_aux_b32s0(p);
        let a_s1 = nest_aux_b32s1(p);
        crate::output::print_nl_cstr(c"### ".as_ptr());
        crate::output::print_mode(m as i32);
        crate::output::print_cstr(c" entered at line ".as_ptr());
        crate::output::print_int(nest_mode_line(p).abs());
        if m == HMODE_SA {
            let pg = nest_prev_graf(p);
            if pg != 0x83_0000 {
                crate::output::print_cstr(c" (language".as_ptr());
                crate::output::print_int(pg % 65536);
                crate::output::print_cstr(c":hyphenmin".as_ptr());
                crate::output::print_int(pg / 0x40_0000);
                crate::output::print_char(',' as i32);
                crate::output::print_int((pg / 65536) % 64);
                crate::output::print_char(')' as i32);
            }
        }
        if nest_mode_line(p) < 0 {
            crate::output::print_cstr(c" (\\output routine)".as_ptr());
        }
        if p == 0 {
            if PAGE_HEAD_SA != page_tail() {
                crate::output::print_nl_cstr(c"### current page:".as_ptr());
                if output_active() {
                    crate::output::print_cstr(c" (held over for next output)".as_ptr());
                }
                show_box(mem_b32s1(PAGE_HEAD_SA));
                if page_contents() > EMPTY_SA {
                    crate::output::print_nl_cstr(c"total height ".as_ptr());
                    crate::output::print_totals();
                    crate::output::print_nl_cstr(c" goal height ".as_ptr());
                    crate::output::print_scaled(page_so_far(0));
                    let mut r = mem_b32s1(PAGE_INS_HEAD_SA);
                    while r != PAGE_INS_HEAD_SA {
                        crate::output::print_ln();
                        crate::output::print_esc_cstr(c"insert".as_ptr());
                        let mut t = mem_b16s0(r) as i32;
                        crate::output::print_int(t);
                        crate::output::print_cstr(c" adds ".as_ptr());
                        let count_t = eqtb_b32s1(COUNT_BASE_SA + t);
                        if count_t == 1000 {
                            t = mem_b32s1(r + 3);
                        } else {
                            t = crate::scaled_math::x_over_n(mem_b32s1(r + 3), 1000) * count_t;
                        }
                        crate::output::print_scaled(t);
                        if mem_b16s1(r) == SPLIT_UP_SA {
                            let mut q = PAGE_HEAD_SA;
                            t = 0;
                            loop {
                                q = mem_b32s1(q);
                                if mem_b16s1(q) == INS_NODE_SA && mem_b16s0(q) == mem_b16s0(r) {
                                    t += 1;
                                }
                                if q == mem_b32s0(r + 1) { break; }
                            }
                            crate::output::print_cstr(c", #".as_ptr());
                            crate::output::print_int(t);
                            crate::output::print_cstr(c" might split".as_ptr());
                        }
                        r = mem_b32s1(r);
                    }
                }
            }
            if mem_b32s1(CONTRIB_HEAD_SA) != crate::mem::TEX_NULL {
                crate::output::print_nl_cstr(c"### recent contributions:".as_ptr());
            }
        }
        show_box(mem_b32s1(nest_head(p)));
        match (m.abs() / (MAX_COMMAND_SA + 1)) as i32 {
            0 => {
                crate::output::print_nl_cstr(c"prevdepth ".as_ptr());
                if a_s1 <= IGNORE_DEPTH_SA {
                    crate::output::print_cstr(c"ignored".as_ptr());
                } else {
                    crate::output::print_scaled(a_s1);
                }
                let pg = nest_prev_graf(p);
                if pg != 0 {
                    crate::output::print_cstr(c", prevgraf ".as_ptr());
                    crate::output::print_int(pg);
                    if pg != 1 {
                        crate::output::print_cstr(c" lines".as_ptr());
                    } else {
                        crate::output::print_cstr(c" line".as_ptr());
                    }
                }
            }
            1 => {
                crate::output::print_nl_cstr(c"spacefactor ".as_ptr());
                crate::output::print_int(a_s0);
                if m > 0 && a_s1 > 0 {
                    crate::output::print_cstr(c", current language ".as_ptr());
                    crate::output::print_int(a_s1);
                }
            }
            2 => {
                if a_s1 != crate::mem::TEX_NULL {
                    crate::output::print_cstr(c"this will begin denominator of:".as_ptr());
                    show_box(a_s1);
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// show_node_list and print_subsidiary_data
// ---------------------------------------------------------------------------

// Constants for show_node_list / print_subsidiary_data.
const NORMAL_SNL: u16 = 0;
const SHRINKING_SNL: u16 = 2;        // xetex-constants.h:281
const DLIST_SNL: u16 = 2;            // xetex-constants.h:278
const A_LEADERS_SNL: u16 = 100;      // xetex_format.h:422
const C_LEADERS_SNL: u16 = 101;      // xetex_format.h:423
const X_LEADERS_SNL: u16 = 102;      // xetex_format.h:424
const MU_GLUE_SNL: u16 = 99;         // xetex_format.h:421
const COND_MATH_GLUE_SNL: u16 = 98;  // xetex-constants.h:324
const BEFORE_SNL: u16 = 0;           // xetex_format.h:434
const AFTER_SNL: u16 = 1;            // xetex_format.h:435
const L_CODE_SNL: u16 = 4;           // xetex-constants.h:142
const R_CODE_SNL: u16 = 8;           // xetex-constants.h:143
const ACC_KERN_SNL: u16 = 2;         // xetex_format.h:429
const SPACE_ADJUSTMENT_SNL: u16 = 3; // xetex_format.h:430
const LIMITS_SNL: u16 = 1;           // xetex_format.h:452
const LEFT_NOAD_SNL: u16 = 30;       // xetex_format.h:415
const HLIST_NODE_SNL: u16 = 0;
const VLIST_NODE_SNL: u16 = 1;
const RULE_NODE_SNL: u16 = 2;
const INS_NODE_SNL: u16 = 3;
const WHATSIT_NODE_SNL: u16 = 8;
const MATH_NODE_SNL: u16 = 9;
const GLUE_NODE_SNL: u16 = 10;
const KERN_NODE_SNL: u16 = 11;
const MARGIN_KERN_NODE_SNL: u16 = 40;
const PENALTY_NODE_SNL: u16 = 12;
const UNSET_NODE_SNL: u16 = 13;
const STYLE_NODE_SNL: u16 = 14;
const CHOICE_NODE_SNL: u16 = 15;
const ORD_NOAD_SNL: u16 = 16;
const OP_NOAD_SNL: u16 = 17;
const BIN_NOAD_SNL: u16 = 18;
const REL_NOAD_SNL: u16 = 19;
const OPEN_NOAD_SNL: u16 = 20;
const CLOSE_NOAD_SNL: u16 = 21;
const PUNCT_NOAD_SNL: u16 = 22;
const INNER_NOAD_SNL: u16 = 23;
const RADICAL_NOAD_SNL: u16 = 24;
const FRACTION_NOAD_SNL: u16 = 25;
const UNDER_NOAD_SNL: u16 = 26;
const OVER_NOAD_SNL: u16 = 27;
const ACCENT_NOAD_SNL: u16 = 28;
const VCENTER_NOAD_SNL: u16 = 29;
const RIGHT_NOAD_SNL: u16 = 31;
const LIGATURE_NODE_SNL: u16 = 6;
const DISC_NODE_SNL: u16 = 7;
const MARK_NODE_SNL: u16 = 4;
const ADJUST_NODE_SNL: u16 = 5;
// Whatsit subtypes.
const OPEN_NODE_SNL: u16 = 0;
const WRITE_NODE_SNL: u16 = 1;
const CLOSE_NODE_SNL: u16 = 2;
const SPECIAL_NODE_SNL: u16 = 3;
const LANGUAGE_NODE_SNL: u16 = 4;
const PDF_SAVE_POS_NODE_SNL: u16 = 21;
const NATIVE_WORD_NODE_SNL: u16 = 40;
const NATIVE_WORD_NODE_AT_SNL: u16 = 41;
const GLYPH_NODE_SNL: u16 = 42;
const PIC_NODE_SNL: u16 = 43;
const PDF_NODE_SNL: u16 = 44;
// print_subsidiary_data: mem[p].b32.s1 cases.
const MATH_CHAR_SNL: i32 = 1;   // xetex-constants.h:269
const SUB_BOX_SNL: i32 = 2;     // xetex-constants.h:283
const SUB_MLIST_SNL: i32 = 3;   // xetex-constants.h:287
// Misc constants.
const DEFAULT_CODE_SNL: i32 = 0x4000_0000; // xetex-constants.h:17
const PIC_NODE_SIZE_SNL: i32 = 9;          // xetex-constants.h
// FONT_ID_BASE = FROZEN_NULL_FONT = 2245338
const FONT_ID_BASE_SNL: i32 = 2_245_338;

/// translated from xetex-xetex0.c:215 (print_subsidiary_data)
unsafe fn print_subsidiary_data(p: i32, c: u16) {
    use crate::globals::{
        depth_threshold, breadth_max, pool_ptr, set_pool_ptr, set_str_pool,
        mem_b32s0, mem_b32s1, set_temp_ptr,
    };
    let _ = breadth_max; // used indirectly via show_node_list
    if crate::hash::cur_length() >= depth_threshold() {
        if mem_b32s1(p) != 0 { // EMPTY = 0
            crate::output::print_cstr(c" []".as_ptr());
        }
    } else {
        set_str_pool(pool_ptr(), c);
        set_pool_ptr(pool_ptr() + 1);
        set_temp_ptr(p);
        match mem_b32s1(p) {
            MATH_CHAR_SNL => {
                crate::output::print_ln();
                crate::output::print_current_string();
                crate::output::print_fam_and_char(p);
            }
            SUB_BOX_SNL => {
                show_info();
            }
            SUB_MLIST_SNL => {
                if mem_b32s0(p) == crate::mem::TEX_NULL {
                    crate::output::print_ln();
                    crate::output::print_current_string();
                    crate::output::print_cstr(c"{}".as_ptr());
                } else {
                    show_info();
                }
            }
            _ => {}
        }
        set_pool_ptr(pool_ptr() - 1);
    }
}

/// translated from xetex-xetex0.c:259 (show_node_list)
#[no_mangle]
pub unsafe extern "C" fn show_node_list(mut p: i32) {
    use crate::globals::{
        depth_threshold, breadth_max, hi_mem_min, mem_end,
        pool_ptr, set_pool_ptr, set_str_pool,
        mem_b16s0, mem_b16s1, mem_b16s2, mem_b32s0, mem_b32s1,
        mem_gr, mem_node_addr, hash_s1,
    };
    if crate::hash::cur_length() > depth_threshold() {
        if p > crate::mem::TEX_NULL {
            crate::output::print_cstr(c" []".as_ptr());
        }
        return;
    }
    let mut n = 0i32;
    while p > 0 {
        crate::output::print_ln();
        crate::output::print_current_string();
        if p > mem_end() {
            crate::output::print_cstr(c"Bad link, display aborted.".as_ptr());
            return;
        }
        n += 1;
        if n > breadth_max() {
            crate::output::print_cstr(c"etc.".as_ptr());
            return;
        }
        if p >= hi_mem_min() {
            crate::output::print_font_and_char(p);
        } else {
            match mem_b16s1(p) {
                HLIST_NODE_SNL | VLIST_NODE_SNL | UNSET_NODE_SNL => {
                    let ntype = mem_b16s1(p);
                    if ntype == HLIST_NODE_SNL {
                        crate::output::print_esc('h' as i32);
                    } else if ntype == VLIST_NODE_SNL {
                        crate::output::print_esc('v' as i32);
                    } else {
                        crate::output::print_esc_cstr(c"unset".as_ptr());
                    }
                    crate::output::print_cstr(c"box(".as_ptr());
                    crate::output::print_scaled(mem_b32s1(p + 3));
                    crate::output::print_char('+' as i32);
                    crate::output::print_scaled(mem_b32s1(p + 2));
                    crate::output::print_cstr(c")x".as_ptr());
                    crate::output::print_scaled(mem_b32s1(p + 1));
                    if ntype == UNSET_NODE_SNL {
                        if mem_b16s0(p) != 0 {
                            crate::output::print_cstr(c" (".as_ptr());
                            crate::output::print_int(mem_b16s0(p) as i32 + 1);
                            crate::output::print_cstr(c" columns)".as_ptr());
                        }
                        if mem_b32s1(p + 6) != 0 {
                            crate::output::print_cstr(c", stretch ".as_ptr());
                            crate::output::print_glue(mem_b32s1(p + 6), mem_b16s0(p + 5) as i32, std::ptr::null());
                        }
                        if mem_b32s1(p + 4) != 0 {
                            crate::output::print_cstr(c", shrink ".as_ptr());
                            crate::output::print_glue(mem_b32s1(p + 4), mem_b16s1(p + 5) as i32, std::ptr::null());
                        }
                    } else {
                        let g = mem_gr(p + 6);
                        if g != 0.0 && mem_b16s1(p + 5) != NORMAL_SNL {
                            crate::output::print_cstr(c", glue set ".as_ptr());
                            if mem_b16s1(p + 5) == SHRINKING_SNL {
                                crate::output::print_cstr(c"- ".as_ptr());
                            }
                            if g.abs() > 20000.0 {
                                if g > 0.0 {
                                    crate::output::print_char('>' as i32);
                                } else {
                                    crate::output::print_cstr(c"< -".as_ptr());
                                }
                                crate::output::print_glue(20000 * 65536, mem_b16s0(p + 5) as i32, std::ptr::null());
                            } else {
                                crate::output::print_glue(
                                    crate::scaled_math::tex_round(65536.0 * g),
                                    mem_b16s0(p + 5) as i32,
                                    std::ptr::null(),
                                );
                            }
                        }
                        if mem_b32s1(p + 4) != 0 {
                            crate::output::print_cstr(c", shifted ".as_ptr());
                            crate::output::print_scaled(mem_b32s1(p + 4));
                        }
                        if ntype == HLIST_NODE_SNL && mem_b16s0(p) == DLIST_SNL {
                            crate::output::print_cstr(c", display".as_ptr());
                        }
                    }
                    set_str_pool(pool_ptr(), '.' as u16);
                    set_pool_ptr(pool_ptr() + 1);
                    show_node_list(mem_b32s1(p + 5));
                    set_pool_ptr(pool_ptr() - 1);
                }
                RULE_NODE_SNL => {
                    crate::output::print_esc_cstr(c"rule(".as_ptr());
                    crate::output::print_rule_dimen(mem_b32s1(p + 3));
                    crate::output::print_char('+' as i32);
                    crate::output::print_rule_dimen(mem_b32s1(p + 2));
                    crate::output::print_cstr(c")x".as_ptr());
                    crate::output::print_rule_dimen(mem_b32s1(p + 1));
                }
                INS_NODE_SNL => {
                    crate::output::print_esc_cstr(c"insert".as_ptr());
                    crate::output::print_int(mem_b16s0(p) as i32);
                    crate::output::print_cstr(c", natural size ".as_ptr());
                    crate::output::print_scaled(mem_b32s1(p + 3));
                    crate::output::print_cstr(c"; split(".as_ptr());
                    crate::output::print_spec(mem_b32s1(p + 4), std::ptr::null());
                    crate::output::print_char(',' as i32);
                    crate::output::print_scaled(mem_b32s1(p + 2));
                    crate::output::print_cstr(c"); float cost ".as_ptr());
                    crate::output::print_int(mem_b32s1(p + 1));
                    set_str_pool(pool_ptr(), '.' as u16);
                    set_pool_ptr(pool_ptr() + 1);
                    show_node_list(mem_b32s0(p + 4));
                    set_pool_ptr(pool_ptr() - 1);
                }
                WHATSIT_NODE_SNL => {
                    match mem_b16s0(p) {
                        OPEN_NODE_SNL => {
                            crate::output::print_write_whatsit(c"openout".as_ptr(), p);
                            crate::output::print_char('=' as i32);
                            crate::output::print_file_name(mem_b32s1(p + 1), mem_b32s0(p + 2), mem_b32s1(p + 2));
                        }
                        WRITE_NODE_SNL => {
                            crate::output::print_write_whatsit(c"write".as_ptr(), p);
                            crate::output::print_mark(mem_b32s1(p + 1));
                        }
                        CLOSE_NODE_SNL => {
                            crate::output::print_write_whatsit(c"closeout".as_ptr(), p);
                        }
                        SPECIAL_NODE_SNL => {
                            crate::output::print_esc_cstr(c"special".as_ptr());
                            crate::output::print_mark(mem_b32s1(p + 1));
                        }
                        LANGUAGE_NODE_SNL => {
                            crate::output::print_esc_cstr(c"setlanguage".as_ptr());
                            crate::output::print_int(mem_b32s1(p + 1));
                            crate::output::print_cstr(c" (hyphenmin ".as_ptr());
                            crate::output::print_int(mem_b16s1(p + 1) as i32);
                            crate::output::print_char(',' as i32);
                            crate::output::print_int(mem_b16s0(p + 1) as i32);
                            crate::output::print_char(')' as i32);
                        }
                        PDF_SAVE_POS_NODE_SNL => {
                            crate::output::print_esc_cstr(c"pdfsavepos".as_ptr());
                        }
                        NATIVE_WORD_NODE_SNL | NATIVE_WORD_NODE_AT_SNL => {
                            // hash[FONT_ID_BASE + mem[p+4].b16.s2].s1
                            crate::output::print_esc(hash_s1(FONT_ID_BASE_SNL + mem_b16s2(p + 4) as i32));
                            crate::output::print_char(' ' as i32);
                            crate::output::print_native_word(p);
                        }
                        GLYPH_NODE_SNL => {
                            crate::output::print_esc(hash_s1(FONT_ID_BASE_SNL + mem_b16s2(p + 4) as i32));
                            crate::output::print_cstr(c" glyph#".as_ptr());
                            crate::output::print_int(mem_b16s1(p + 4) as i32);
                        }
                        PIC_NODE_SNL | PDF_NODE_SNL => {
                            if mem_b16s0(p) == PIC_NODE_SNL {
                                crate::output::print_esc_cstr(c"XeTeXpicfile".as_ptr());
                            } else {
                                crate::output::print_esc_cstr(c"XeTeXpdffile".as_ptr());
                            }
                            crate::output::print_cstr(c"( ".as_ptr());
                            let path_len = mem_b16s1(p + 4) as i32;
                            let path_ptr = mem_node_addr(p + PIC_NODE_SIZE_SNL) as *const u8;
                            for i in 0..path_len {
                                crate::output::print_raw_char(*path_ptr.add(i as usize) as u16, true);
                            }
                            crate::output::print('"' as i32);
                        }
                        _ => { crate::output::print_cstr(c"whatsit?".as_ptr()); }
                    }
                }
                GLUE_NODE_SNL => {
                    let sub = mem_b16s0(p);
                    if sub >= A_LEADERS_SNL {
                        crate::output::print_esc_cstr(c"".as_ptr());
                        if sub == C_LEADERS_SNL {
                            crate::output::print_char('c' as i32);
                        } else if sub == X_LEADERS_SNL {
                            crate::output::print_char('x' as i32);
                        }
                        crate::output::print_cstr(c"leaders ".as_ptr());
                        crate::output::print_spec(mem_b32s0(p + 1), std::ptr::null());
                        set_str_pool(pool_ptr(), '.' as u16);
                        set_pool_ptr(pool_ptr() + 1);
                        show_node_list(mem_b32s1(p + 1));
                        set_pool_ptr(pool_ptr() - 1);
                    } else {
                        crate::output::print_esc_cstr(c"glue".as_ptr());
                        if sub != NORMAL_SNL {   // GLUE_SPEC_shrink_order(p) != NORMAL
                            crate::output::print_char('(' as i32);
                            if sub < COND_MATH_GLUE_SNL {
                                crate::output::print_skip_param(sub as i32 - 1);
                            } else if sub == COND_MATH_GLUE_SNL {
                                crate::output::print_esc_cstr(c"nonscript".as_ptr());
                            } else {
                                crate::output::print_esc_cstr(c"mskip".as_ptr());
                            }
                            crate::output::print_char(')' as i32);
                        }
                        if sub != COND_MATH_GLUE_SNL {
                            crate::output::print_char(' ' as i32);
                            if sub < COND_MATH_GLUE_SNL {
                                crate::output::print_spec(mem_b32s0(p + 1), std::ptr::null());
                            } else {
                                crate::output::print_spec(mem_b32s0(p + 1), c"mu".as_ptr());
                            }
                        }
                    }
                }
                KERN_NODE_SNL => {
                    if mem_b16s0(p) != MU_GLUE_SNL {
                        crate::output::print_esc_cstr(c"kern".as_ptr());
                        if mem_b16s0(p) != NORMAL_SNL {
                            crate::output::print_char(' ' as i32);
                        }
                        crate::output::print_scaled(mem_b32s1(p + 1));
                        let sub = mem_b16s0(p);
                        if sub == ACC_KERN_SNL {
                            crate::output::print_cstr(c" (for accent)".as_ptr());
                        } else if sub == SPACE_ADJUSTMENT_SNL {
                            crate::output::print_cstr(c" (space adjustment)".as_ptr());
                        }
                    } else {
                        crate::output::print_esc_cstr(c"mkern".as_ptr());
                        crate::output::print_scaled(mem_b32s1(p + 1));
                        crate::output::print_cstr(c"mu".as_ptr());
                    }
                }
                MARGIN_KERN_NODE_SNL => {
                    crate::output::print_esc_cstr(c"kern".as_ptr());
                    crate::output::print_scaled(mem_b32s1(p + 1));
                    if mem_b16s0(p) == 0 {
                        crate::output::print_cstr(c" (left margin)".as_ptr());
                    } else {
                        crate::output::print_cstr(c" (right margin)".as_ptr());
                    }
                }
                MATH_NODE_SNL => {
                    let sub = mem_b16s0(p);
                    if sub > AFTER_SNL {
                        if sub % 2 == 1 {
                            crate::output::print_esc_cstr(c"end".as_ptr());
                        } else {
                            crate::output::print_esc_cstr(c"begin".as_ptr());
                        }
                        if sub > R_CODE_SNL {
                            crate::output::print_char('R' as i32);
                        } else if sub > L_CODE_SNL {
                            crate::output::print_char('L' as i32);
                        } else {
                            crate::output::print_char('M' as i32);
                        }
                    } else {
                        crate::output::print_esc_cstr(c"math".as_ptr());
                        if sub == BEFORE_SNL {
                            crate::output::print_cstr(c"on".as_ptr());
                        } else {
                            crate::output::print_cstr(c"off".as_ptr());
                        }
                        if mem_b32s1(p + 1) != 0 {
                            crate::output::print_cstr(c", surrounded ".as_ptr());
                            crate::output::print_scaled(mem_b32s1(p + 1));
                        }
                    }
                }
                LIGATURE_NODE_SNL => {
                    crate::output::print_font_and_char(p + 1);
                    crate::output::print_cstr(c" (ligature ".as_ptr());
                    if mem_b16s0(p) > 1 {
                        crate::output::print_char('|' as i32);
                    }
                    crate::globals::set_font_in_short_display(mem_b16s1(p + 1) as i32);
                    short_display(mem_b32s1(p + 1));
                    if mem_b16s0(p) % 2 == 1 {
                        crate::output::print_char('|' as i32);
                    }
                    crate::output::print_char(')' as i32);
                }
                PENALTY_NODE_SNL => {
                    crate::output::print_esc_cstr(c"penalty ".as_ptr());
                    crate::output::print_int(mem_b32s1(p + 1));
                }
                DISC_NODE_SNL => {
                    crate::output::print_esc_cstr(c"discretionary".as_ptr());
                    if mem_b16s0(p) > 0 {
                        crate::output::print_cstr(c" replacing ".as_ptr());
                        crate::output::print_int(mem_b16s0(p) as i32);
                    }
                    set_str_pool(pool_ptr(), '.' as u16);
                    set_pool_ptr(pool_ptr() + 1);
                    show_node_list(mem_b32s0(p + 1));
                    set_pool_ptr(pool_ptr() - 1);
                    set_str_pool(pool_ptr(), '|' as u16);
                    set_pool_ptr(pool_ptr() + 1);
                    show_node_list(mem_b32s1(p + 1));
                    set_pool_ptr(pool_ptr() - 1);
                }
                MARK_NODE_SNL => {
                    crate::output::print_esc_cstr(c"mark".as_ptr());
                    if mem_b32s0(p + 1) != 0 {
                        crate::output::print_char('s' as i32);
                        crate::output::print_int(mem_b32s0(p + 1));
                    }
                    crate::output::print_mark(mem_b32s1(p + 1));
                }
                ADJUST_NODE_SNL => {
                    crate::output::print_esc_cstr(c"vadjust".as_ptr());
                    if mem_b16s0(p) != 0 {
                        crate::output::print_cstr(c" pre ".as_ptr());
                    }
                    set_str_pool(pool_ptr(), '.' as u16);
                    set_pool_ptr(pool_ptr() + 1);
                    show_node_list(mem_b32s1(p + 1));
                    set_pool_ptr(pool_ptr() - 1);
                }
                STYLE_NODE_SNL => {
                    crate::output::print_style(mem_b16s0(p) as i32);
                }
                CHOICE_NODE_SNL => {
                    crate::output::print_esc_cstr(c"mathchoice".as_ptr());
                    set_str_pool(pool_ptr(), 'D' as u16);
                    set_pool_ptr(pool_ptr() + 1);
                    show_node_list(mem_b32s0(p + 1));
                    set_pool_ptr(pool_ptr() - 1);
                    set_str_pool(pool_ptr(), 'T' as u16);
                    set_pool_ptr(pool_ptr() + 1);
                    show_node_list(mem_b32s1(p + 1));
                    set_pool_ptr(pool_ptr() - 1);
                    set_str_pool(pool_ptr(), 'S' as u16);
                    set_pool_ptr(pool_ptr() + 1);
                    show_node_list(mem_b32s0(p + 2));
                    set_pool_ptr(pool_ptr() - 1);
                    set_str_pool(pool_ptr(), 's' as u16);
                    set_pool_ptr(pool_ptr() + 1);
                    show_node_list(mem_b32s1(p + 2));
                    set_pool_ptr(pool_ptr() - 1);
                }
                ORD_NOAD_SNL | OP_NOAD_SNL | BIN_NOAD_SNL | REL_NOAD_SNL
                | OPEN_NOAD_SNL | CLOSE_NOAD_SNL | PUNCT_NOAD_SNL | INNER_NOAD_SNL
                | RADICAL_NOAD_SNL | OVER_NOAD_SNL | UNDER_NOAD_SNL | VCENTER_NOAD_SNL
                | ACCENT_NOAD_SNL | LEFT_NOAD_SNL | RIGHT_NOAD_SNL => {
                    let ntype = mem_b16s1(p);
                    match ntype {
                        ORD_NOAD_SNL => { crate::output::print_esc_cstr(c"mathord".as_ptr()); }
                        OP_NOAD_SNL => { crate::output::print_esc_cstr(c"mathop".as_ptr()); }
                        BIN_NOAD_SNL => { crate::output::print_esc_cstr(c"mathbin".as_ptr()); }
                        REL_NOAD_SNL => { crate::output::print_esc_cstr(c"mathrel".as_ptr()); }
                        OPEN_NOAD_SNL => { crate::output::print_esc_cstr(c"mathopen".as_ptr()); }
                        CLOSE_NOAD_SNL => { crate::output::print_esc_cstr(c"mathclose".as_ptr()); }
                        PUNCT_NOAD_SNL => { crate::output::print_esc_cstr(c"mathpunct".as_ptr()); }
                        INNER_NOAD_SNL => { crate::output::print_esc_cstr(c"mathinner".as_ptr()); }
                        OVER_NOAD_SNL => { crate::output::print_esc_cstr(c"overline".as_ptr()); }
                        UNDER_NOAD_SNL => { crate::output::print_esc_cstr(c"underline".as_ptr()); }
                        VCENTER_NOAD_SNL => { crate::output::print_esc_cstr(c"vcenter".as_ptr()); }
                        RADICAL_NOAD_SNL => {
                            crate::output::print_esc_cstr(c"radical".as_ptr());
                            crate::output::print_delimiter(p + 4);
                        }
                        ACCENT_NOAD_SNL => {
                            crate::output::print_esc_cstr(c"accent".as_ptr());
                            crate::output::print_fam_and_char(p + 4);
                        }
                        LEFT_NOAD_SNL => {
                            crate::output::print_esc_cstr(c"left".as_ptr());
                            crate::output::print_delimiter(p + 1);
                        }
                        RIGHT_NOAD_SNL => {
                            if mem_b16s0(p) == NORMAL_SNL {
                                crate::output::print_esc_cstr(c"right".as_ptr());
                            } else {
                                crate::output::print_esc_cstr(c"middle".as_ptr());
                            }
                            crate::output::print_delimiter(p + 1);
                        }
                        _ => {}
                    }
                    if ntype < LEFT_NOAD_SNL {
                        if mem_b16s0(p) != NORMAL_SNL {
                            if mem_b16s0(p) == LIMITS_SNL {
                                crate::output::print_esc_cstr(c"limits".as_ptr());
                            } else {
                                crate::output::print_esc_cstr(c"nolimits".as_ptr());
                            }
                        }
                        print_subsidiary_data(p + 1, '.' as u16);
                    }
                    print_subsidiary_data(p + 2, '^' as u16);
                    print_subsidiary_data(p + 3, '_' as u16);
                }
                FRACTION_NOAD_SNL => {
                    crate::output::print_esc_cstr(c"fraction, thickness ".as_ptr());
                    if mem_b32s1(p + 1) == DEFAULT_CODE_SNL {
                        crate::output::print_cstr(c"= default".as_ptr());
                    } else {
                        crate::output::print_scaled(mem_b32s1(p + 1));
                    }
                    // left delimiter check
                    if mem_b16s3(p + 4) % 256 != 0
                        || (mem_b16s2(p + 4) as i32
                            + (mem_b16s3(p + 4) as i32 / 256) * 65536) != 0
                        || mem_b16s1(p + 4) % 256 != 0
                        || (mem_b16s0(p + 4) as i32
                            + (mem_b16s1(p + 4) as i32 / 256) * 65536) != 0
                    {
                        crate::output::print_cstr(c", left-delimiter ".as_ptr());
                        crate::output::print_delimiter(p + 4);
                    }
                    // right delimiter check
                    if mem_b16s3(p + 5) % 256 != 0
                        || (mem_b16s2(p + 5) as i32
                            + (mem_b16s3(p + 5) as i32 / 256) * 65536) != 0
                        || mem_b16s1(p + 5) % 256 != 0
                        || (mem_b16s0(p + 5) as i32
                            + (mem_b16s1(p + 5) as i32 / 256) * 65536) != 0
                    {
                        crate::output::print_cstr(c", right-delimiter ".as_ptr());
                        crate::output::print_delimiter(p + 5);
                    }
                    print_subsidiary_data(p + 2, '\\' as u16);
                    print_subsidiary_data(p + 3, '/' as u16);
                }
                _ => {
                    crate::output::print_cstr(c"Unknown node type!".as_ptr());
                }
            }
        }
        p = mem_b32s1(p);
    }
}

// ---------------------------------------------------------------------------
// show_context
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:2764 (show_context)
#[no_mangle]
pub unsafe extern "C" fn show_context() {
    use crate::globals::{
        base_ptr, set_base_ptr, input_ptr, input_stack, set_input_stack, set_cur_input,
        tally, set_tally, trick_count, set_trick_count, first_count, set_first_count,
        error_line, half_error_line, line, line_stack, in_open, buffer_entry,
        trick_buf,
    };

    const TOKEN_LIST: u16 = 0;
    const BACKED_UP: u16 = 3;
    const BACKED_UP_CHAR: u16 = 4;
    const PARAMETER: u16 = 0;
    const U_TEMPLATE: u16 = 1;
    const V_TEMPLATE: u16 = 2;
    const INSERTED: u16 = 5;
    const MACRO: u16 = 6;
    const OUTPUT_TEXT: u16 = 7;
    const EVERY_PAR_TEXT: u16 = 8;
    const EVERY_MATH_TEXT: u16 = 9;
    const EVERY_DISPLAY_TEXT: u16 = 10;
    const EVERY_HBOX_TEXT: u16 = 11;
    const EVERY_VBOX_TEXT: u16 = 12;
    const EVERY_JOB_TEXT: u16 = 13;
    const EVERY_CR_TEXT: u16 = 14;
    const MARK_TEXT: u16 = 15;
    const EVERY_EOF_TEXT: u16 = 16;
    const INTER_CHAR_TEXT: u16 = 17;
    const WRITE_TEXT: u16 = 18;
    const TECTONIC_CODA_TEXT: u16 = 19;
    const SELECTOR_PSEUDO: i32 = 20;
    const INT_PAR_ERROR_CONTEXT_LINES: i32 = 54;

    set_base_ptr(input_ptr());
    set_input_stack(base_ptr(), cur_input());
    let mut nn: i32 = -1;
    let mut bottom_line = false;

    'outer: loop {
        set_cur_input(input_stack(base_ptr()));
        let ci = cur_input();
        if ci.state != TOKEN_LIST {
            if ci.name > 19 || base_ptr() == 0 {
                bottom_line = true;
            }
        }
        let error_context_lines = eqtb_b32s1(INT_BASE + INT_PAR_ERROR_CONTEXT_LINES);
        if base_ptr() == input_ptr() || bottom_line || nn < error_context_lines {
            let ci = cur_input();
            if base_ptr() == input_ptr()
                || ci.state != TOKEN_LIST
                || ci.index != BACKED_UP
                || ci.loc != crate::mem::TEX_NULL
            {
                set_tally(0);
                let old_setting = selector();
                let l;
                if ci.state != TOKEN_LIST {
                    if ci.name <= 17 {
                        if ci.name == 0 {
                            if base_ptr() == 0 {
                                crate::output::print_nl_cstr(c"<*>".as_ptr());
                            } else {
                                crate::output::print_nl_cstr(c"<insert> ".as_ptr());
                            }
                        } else {
                            crate::output::print_nl_cstr(c"<read ".as_ptr());
                            if ci.name == 17 {
                                crate::output::print_char('*' as i32);
                            } else {
                                crate::output::print_int(ci.name - 1);
                            }
                            crate::output::print_char('>' as i32);
                        }
                    } else {
                        crate::output::print_nl_cstr(c"l.".as_ptr());
                        if ci.index as i32 == in_open() {
                            crate::output::print_int(line());
                        } else {
                            crate::output::print_int(line_stack(ci.index as i32 + 1));
                        }
                    }
                    crate::output::print_char(' ' as i32);
                    l = tally();
                    set_tally(0);
                    set_selector(SELECTOR_PSEUDO);
                    set_trick_count(1_000_000);
                    let end_char = eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR);
                    let j = if buffer_entry(ci.limit) == end_char { ci.limit } else { ci.limit + 1 };
                    if j > 0 {
                        let mut i = ci.start;
                        let for_end = j - 1;
                        if i <= for_end {
                            loop {
                                if i == ci.loc {
                                    set_first_count(tally());
                                    let tc = tally() + 1 + error_line() - half_error_line();
                                    set_trick_count(tc.max(error_line()));
                                }
                                crate::output::print_char(buffer_entry(i));
                                if i >= for_end { break; }
                                i += 1;
                            }
                        }
                    }
                } else {
                    match ci.index {
                        PARAMETER => crate::output::print_nl_cstr(c"<argument> ".as_ptr()),
                        U_TEMPLATE | V_TEMPLATE => crate::output::print_nl_cstr(c"<template> ".as_ptr()),
                        BACKED_UP | BACKED_UP_CHAR => {
                            if ci.loc == crate::mem::TEX_NULL {
                                crate::output::print_nl_cstr(c"<recently read> ".as_ptr());
                            } else {
                                crate::output::print_nl_cstr(c"<to be read again> ".as_ptr());
                            }
                        }
                        INSERTED => crate::output::print_nl_cstr(c"<inserted text> ".as_ptr()),
                        MACRO => {
                            crate::output::print_ln();
                            crate::output::print_cs(ci.name);
                        }
                        OUTPUT_TEXT => crate::output::print_nl_cstr(c"<output> ".as_ptr()),
                        EVERY_PAR_TEXT => crate::output::print_nl_cstr(c"<everypar> ".as_ptr()),
                        EVERY_MATH_TEXT => crate::output::print_nl_cstr(c"<everymath> ".as_ptr()),
                        EVERY_DISPLAY_TEXT => crate::output::print_nl_cstr(c"<everydisplay> ".as_ptr()),
                        EVERY_HBOX_TEXT => crate::output::print_nl_cstr(c"<everyhbox> ".as_ptr()),
                        EVERY_VBOX_TEXT => crate::output::print_nl_cstr(c"<everyvbox> ".as_ptr()),
                        EVERY_JOB_TEXT => crate::output::print_nl_cstr(c"<everyjob> ".as_ptr()),
                        EVERY_CR_TEXT => crate::output::print_nl_cstr(c"<everycr> ".as_ptr()),
                        MARK_TEXT => crate::output::print_nl_cstr(c"<mark> ".as_ptr()),
                        EVERY_EOF_TEXT => crate::output::print_nl_cstr(c"<everyeof> ".as_ptr()),
                        INTER_CHAR_TEXT => crate::output::print_nl_cstr(c"<XeTeXinterchartoks> ".as_ptr()),
                        WRITE_TEXT => crate::output::print_nl_cstr(c"<write> ".as_ptr()),
                        TECTONIC_CODA_TEXT => crate::output::print_nl_cstr(c"<TectonicCodaTokens> ".as_ptr()),
                        _ => crate::output::print_nl('?' as i32),
                    }
                    l = tally();
                    set_tally(0);
                    set_selector(SELECTOR_PSEUDO);
                    set_trick_count(1_000_000);
                    if ci.index < MACRO {
                        crate::output::show_token_list(ci.start, ci.loc, 100_000);
                    } else {
                        crate::output::show_token_list(mem_b32s1(ci.start), ci.loc, 100_000);
                    }
                }
                set_selector(old_setting);
                if trick_count() == 1_000_000 {
                    set_first_count(tally());
                    let tc = tally() + 1 + error_line() - half_error_line();
                    set_trick_count(tc.max(error_line()));
                }
                let m = if tally() < trick_count() {
                    tally() - first_count()
                } else {
                    trick_count() - first_count()
                };
                let (mut p, n) = if l + first_count() <= half_error_line() {
                    (0i32, l + first_count())
                } else {
                    crate::output::print_cstr(c"...".as_ptr());
                    (l + first_count() - half_error_line() + 3, half_error_line())
                };
                let fc = first_count();
                let mut q = p;
                while q <= fc - 1 {
                    crate::output::print_char(trick_buf((q % error_line()) as usize) as i32);
                    q += 1;
                }
                crate::output::print_ln();
                let mut q2 = 1;
                while q2 <= n {
                    crate::output::print_raw_char(' ' as u16, true);
                    q2 += 1;
                }
                p = if m + n <= error_line() {
                    fc + m
                } else {
                    fc + (error_line() - n - 3)
                };
                let mut q3 = fc;
                while q3 <= p - 1 {
                    crate::output::print_char(trick_buf((q3 % error_line()) as usize) as i32);
                    q3 += 1;
                }
                if m + n > error_line() {
                    crate::output::print_cstr(c"...".as_ptr());
                }
                nn += 1;
            }
        } else if nn == error_context_lines {
            crate::output::print_nl_cstr(c"...".as_ptr());
            nn += 1;
        }
        if bottom_line {
            break 'outer;
        }
        set_base_ptr(base_ptr() - 1);
    }
    set_cur_input(input_stack(input_ptr()));
}

// ---------------------------------------------------------------------------
// expand
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:4003 (expand)
#[no_mangle]
pub unsafe extern "C" fn expand() {
    use crate::globals::{
        cur_val_level, set_cur_val_level, radix, set_radix, cur_order, set_cur_order,
        is_in_csname, set_is_in_csname, cur_ptr, set_cur_ptr,
        cur_mark, set_cur_mark,
        cond_ptr, set_cond_ptr, if_limit, set_if_limit,
        if_line, set_if_line, cur_if, set_cur_if,
        in_open, if_stack,
        force_eof, set_force_eof,
        max_buf_stack, set_max_buf_stack, buf_size,
        no_new_control_sequence, set_no_new_control_sequence,
        hash_s1, eqtb_b16s1,
    };

    const TOP_BOT_MARK: u8 = 112;
    const EXPAND_AFTER: u8 = 104;
    const NO_EXPAND: u8 = 105;
    const INPUT: u8 = 106;
    const CS_NAME_CMD: u8 = 109;
    const CONVERT_CMD: u8 = 110;
    const THE_CMD: u8 = 111;    // same as module-level THE
    const IF_TEST_CMD: u8 = 107;
    const FI_OR_ELSE_CMD: u8 = 108;
    const MARKS_CODE: i32 = 5;
    const MARK_VAL_IDX: usize = 7;
    const TOP_MARK_CODE: usize = 0;
    const FIRST_MARK_CODE: usize = 1;
    const BOT_MARK_CODE: usize = 2;
    const SPLIT_FIRST_MARK_CODE: usize = 3;
    const SPLIT_BOT_MARK_CODE: usize = 4;
    const MARK_TEXT_TL: u16 = 15;
    const FROZEN_DONT_EXPAND: i32 = 2_243_226 + 9;
    const FROZEN_CS_TOKEN: i32 = CS_TOKEN_FLAG + FROZEN_DONT_EXPAND;
    const FROZEN_PRIM_CS_TOKEN: i32 = CS_TOKEN_FLAG + FROZEN_PRIMITIVE;
    const NORMAL_SCANNER: u8 = 0;
    const IF_CODE_VAL: u8 = 1;
    const IF_CASE_CODE_VAL: i32 = 16;
    const IF_NODE_SIZE_V: i32 = 2;
    const TOO_BIG_USV: i32 = 1_114_112;
    const NULL_CS_V: i32 = 2_228_225;
    const UNDEFINED_CS_CMD: u8 = 103;
    const INT_PAR_TRACING_COMMANDS: i32 = 36;
    const INT_PAR_TRACING_IFS: i32 = 58;
    const FI_CODE_VAL: i32 = 2;

    set_expand_depth_count(expand_depth_count() + 1);
    if expand_depth_count() >= expand_depth() {
        crate::errors::overflow(c"expansion depth".as_ptr(), expand_depth());
    }

    let cv_backup = cur_val();
    let cvl_backup = cur_val_level();
    let radix_backup = radix();
    let co_backup = cur_order();
    let backup_backup = mem_b32s1(BACKUP_HEAD);

    'reswitch: loop {
        if cur_cmd() < CALL {
            if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 1 {
                crate::output::show_cur_cmd_chr();
            }
            match cur_cmd() {
                TOP_BOT_MARK => {
                    let t = (cur_chr() % MARKS_CODE) as usize;
                    if cur_chr() >= MARKS_CODE {
                        scan_register_num();
                    } else {
                        set_cur_val(0);
                    }
                    if cur_val() == 0 {
                        set_cur_ptr(cur_mark(t));
                    } else {
                        crate::save::find_sa_element(MARK_VAL_IDX as u16, cur_val(), false);
                        if cur_ptr() != crate::mem::TEX_NULL {
                            if t % 2 != 0 {
                                set_cur_ptr(mem_b32s1(cur_ptr() + (t as i32 / 2) + 1));
                            } else {
                                set_cur_ptr(mem_b32s0(cur_ptr() + (t as i32 / 2) + 1));
                            }
                        }
                    }
                    if cur_ptr() != crate::mem::TEX_NULL {
                        crate::input::begin_token_list(cur_ptr(), MARK_TEXT_TL);
                    }
                }
                EXPAND_AFTER => {
                    if cur_chr() == 0 {
                        get_token();
                        let t = cur_tok();
                        get_token();
                        if cur_cmd() > MAX_COMMAND {
                            expand();
                        } else {
                            crate::input::back_input();
                        }
                        set_cur_tok(t);
                        crate::input::back_input();
                    } else {
                        // \unless
                        get_token();
                        if cur_cmd() == IF_TEST_CMD && cur_chr() != IF_CASE_CODE_VAL {
                            set_cur_chr(cur_chr() + UNLESS_CODE);
                            continue 'reswitch;
                        }
                        crate::output::error_here_with_diagnostic(c"You can't use `".as_ptr());
                        crate::output::print_esc_cstr(c"unless".as_ptr());
                        crate::output::print_cstr(c"' before `".as_ptr());
                        print_cmd_chr(cur_cmd() as u16, cur_chr());
                        crate::output::print_char('\'' as i32);
                        crate::output::capture_to_diagnostic(core::ptr::null_mut());
                        set_help_ptr(1);
                        set_help_line(0, c"Continue, and I'll forget that it ever happened.".as_ptr());
                        crate::input::back_error();
                    }
                }
                NO_EXPAND => {
                    if cur_chr() == 0 {
                        let save_scanner_status = scanner_status();
                        set_scanner_status(NORMAL_SCANNER);
                        get_token();
                        set_scanner_status(save_scanner_status);
                        let t = cur_tok();
                        crate::input::back_input();
                        if t >= CS_TOKEN_FLAG {
                            let p = crate::mem::get_avail();
                            set_mem_b32s0(p, FROZEN_CS_TOKEN);
                            set_mem_b32s1(p, cur_input().loc);
                            set_cur_input_start(p);
                            set_cur_input_loc(p);
                        }
                    } else {
                        // \primitive
                        let save_scanner_status = scanner_status();
                        set_scanner_status(NORMAL_SCANNER);
                        get_token();
                        set_scanner_status(save_scanner_status);
                        let cs = if cur_cs() < HASH_BASE {
                            crate::hash::prim_lookup(cur_cs() - SINGLE_BASE)
                        } else {
                            crate::hash::prim_lookup(hash_s1(cur_cs()))
                        };
                        set_cur_cs(cs);
                        if cur_cs() != UNDEFINED_PRIMITIVE {
                            let t = eqtb_b16s1(PRIM_EQTB_BASE + cur_cs()) as u8;
                            if t > MAX_COMMAND {
                                set_cur_cmd(t);
                                set_cur_chr(eqtb_b32s1(PRIM_EQTB_BASE + cur_cs()));
                                set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
                                set_cur_cs(0);
                                continue 'reswitch;
                            } else {
                                crate::input::back_input();
                                let p = crate::mem::get_avail();
                                set_mem_b32s0(p, FROZEN_PRIM_CS_TOKEN);
                                set_mem_b32s1(p, cur_input().loc);
                                set_cur_input_loc(p);
                                set_cur_input_start(p);
                            }
                        }
                    }
                }
                CS_NAME_CMD => {
                    let r = crate::mem::get_avail();
                    let mut p = r;
                    let b = is_in_csname();
                    set_is_in_csname(true);
                    loop {
                        get_x_token();
                        if cur_cs() == 0 {
                            let q = crate::mem::get_avail();
                            set_mem_b32s1(p, q);
                            set_mem_b32s0(q, cur_tok());
                            p = q;
                        }
                        if cur_cs() != 0 { break; }
                    }
                    if cur_cmd() != END_CS_NAME {
                        crate::output::error_here_with_diagnostic(c"Missing ".as_ptr());
                        crate::output::print_esc_cstr(c"endcsname".as_ptr());
                        crate::output::print_cstr(c" inserted".as_ptr());
                        crate::output::capture_to_diagnostic(core::ptr::null_mut());
                        set_help_ptr(2);
                        set_help_line(1, c"The control sequence marked <to be read again> should".as_ptr());
                        set_help_line(0, c"not appear between \\csname and \\endcsname.".as_ptr());
                        crate::input::back_error();
                    }
                    set_is_in_csname(b);
                    let mut j = first();
                    p = mem_b32s1(r);
                    while p != crate::mem::TEX_NULL {
                        if j >= max_buf_stack() {
                            set_max_buf_stack(j + 1);
                            if max_buf_stack() == buf_size() {
                                crate::errors::overflow(c"buffer size".as_ptr(), buf_size());
                            }
                        }
                        set_buffer_entry(j, mem_b32s0(p) % MAX_CHAR_VAL);
                        j += 1;
                        p = mem_b32s1(p);
                    }
                    let cs = if j > first() + 1 || buffer_entry(first()) > 65535 {
                        set_no_new_control_sequence(false);
                        let cs = crate::hash::id_lookup(first(), j - first());
                        set_no_new_control_sequence(true);
                        cs
                    } else if j == first() {
                        NULL_CS_V
                    } else {
                        SINGLE_BASE + buffer_entry(first())
                    };
                    set_cur_cs(cs);
                    crate::mem::flush_list(r);
                    if eqtb_b16s1(cur_cs()) == UNDEFINED_CS_CMD as u16 {
                        crate::save::eq_define(cur_cs(), RELAX as u16, TOO_BIG_USV);
                    }
                    set_cur_tok(cur_cs() + CS_TOKEN_FLAG);
                    crate::input::back_input();
                }
                CONVERT_CMD => {
                    conv_toks();
                }
                THE_CMD => {
                    ins_the_toks();
                }
                IF_TEST_CMD => {
                    conditional();
                }
                FI_OR_ELSE_CMD => {
                    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_IFS) > 0 {
                        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) <= 1 {
                            crate::output::show_cur_cmd_chr();
                        }
                    }
                    if cur_chr() > if_limit() as i32 {
                        if if_limit() == IF_CODE_VAL {
                            crate::input::insert_relax();
                        } else {
                            crate::output::error_here_with_diagnostic(c"Extra ".as_ptr());
                            print_cmd_chr(FI_OR_ELSE_CMD as u16, cur_chr());
                            crate::output::capture_to_diagnostic(core::ptr::null_mut());
                            set_help_ptr(1);
                            set_help_line(0, c"I'm ignoring this; it doesn't match any \\if.".as_ptr());
                            crate::errors::error();
                        }
                    } else {
                        while cur_chr() != FI_CODE_VAL {
                            pass_text();
                        }
                        if if_stack(in_open()) == cond_ptr() {
                            crate::output::if_warning();
                        }
                        let p = cond_ptr();
                        set_if_line(mem_b32s1(p + 1));
                        set_cur_if(mem_b16s0(p) as i16);
                        set_if_limit(mem_b16s1(p) as u8);
                        set_cond_ptr(mem_b32s1(p));
                        crate::mem::free_node(p, IF_NODE_SIZE_V);
                    }
                }
                INPUT => {
                    if cur_chr() == 1 {
                        set_force_eof(true);
                    } else if cur_chr() == 2 {
                        pseudo_start();
                    } else if name_in_progress() {
                        crate::input::insert_relax();
                    } else {
                        start_input(core::ptr::null());
                    }
                }
                _ => {
                    crate::output::error_here_with_diagnostic(c"Undefined control sequence".as_ptr());
                    crate::output::capture_to_diagnostic(core::ptr::null_mut());
                    set_help_ptr(5);
                    set_help_line(4, c"The control sequence at the end of the top line".as_ptr());
                    set_help_line(3, c"of your error message was never \\def'ed. If you have".as_ptr());
                    set_help_line(2, c"misspelled it (e.g., `\\hobx'), type `I' and the correct".as_ptr());
                    set_help_line(1, c"spelling (e.g., `I\\hbox'). Otherwise just continue,".as_ptr());
                    set_help_line(0, c"and I'll forget about whatever was undefined.".as_ptr());
                    crate::errors::error();
                }
            }
        } else if cur_cmd() < END_TEMPLATE {
            macro_call();
        } else {
            set_cur_tok(CS_TOKEN_FLAG + FROZEN_ENDV);
            crate::input::back_input();
        }
        break 'reswitch;
    }

    set_cur_val(cv_backup);
    set_cur_val_level(cvl_backup);
    set_radix(radix_backup);
    set_cur_order(co_backup);
    set_mem_b32s1(BACKUP_HEAD, backup_backup);
    set_expand_depth_count(expand_depth_count() - 1);
}

// translated from xetex-xetex0.c:3167 (get_next)
#[no_mangle]
pub unsafe extern "C" fn get_next() {
    use crate::globals::{
        cond_ptr, cur_align, cur_boundary, deletions_allowed, eqtb_b16s1,
        eof_seen, force_eof, grp_stack, if_stack, in_open, input_file, input_ptr,
        par_loc, param_stack, set_cur_input, set_cur_input_index,
        set_deletions_allowed, set_eof_seen, set_force_eof,
        set_mem_b32s1, set_used_tectonic_coda_tokens, used_tectonic_coda_tokens,
        line, set_line,
    };

    // State codes (xetex-constants.h)
    const TOKEN_LIST: u16 = 0;
    const MID_LINE: u16 = 1;
    const SKIP_BLANKS: u16 = 17;
    const NEW_LINE: u16 = 33;
    // Category codes (xetex_format.h)
    const ESCAPE: u8 = 0;
    const LEFT_BRACE_CAT: u8 = 1;
    const RIGHT_BRACE_CAT: u8 = 2;
    const MATH_SHIFT: u8 = 3;
    const TAB_MARK_CAT: u8 = 4;
    const CAR_RET: u8 = 5;
    const MAC_PARAM: u8 = 6;
    const SUP_MARK: u8 = 7;
    const SUB_MARK: u8 = 8;
    const IGNORE: u8 = 9;
    const SPACER: u8 = 10;
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const ACTIVE_CHAR: u8 = 13;
    const COMMENT: u8 = 14;
    const INVALID_CHAR: u8 = 15;
    // Other constants
    const CAT_CODE_BASE: i32 = 2_256_169;     // xetex_format.h:214
    const NULL_CS_GN: i32 = 2_228_225;        // xetex_format.h:199
    const BIGGEST_USV_GN: i32 = 0x10_FFFF;
    const OUTER_CALL_GN: u8 = 115;
    const DONT_EXPAND_GN: u8 = 118;
    const MAX_COMMAND_GN: u8 = 102;
    const RELAX_GN: u8 = 0;
    const NO_EXPAND_FLAG_GN: i32 = BIGGEST_USV_GN + 2;
    const CS_TOKEN_FLAG_GN: i32 = 0x1FF_FFFF;
    const MAX_CHAR_VAL_GN: i32 = 0x200000;
    const OUT_PARAM_GN: u8 = 5;
    const PARAMETER_GN: u16 = 0;
    const V_TEMPLATE_GN: u16 = 2;
    const OMIT_GN: u8 = 63;
    const OMIT_TEMPLATE_GN: i32 = 4_999_999 - 10;    // MEM_TOP - 10
    const ALIGNING_GN: u8 = 4;
    const LOCAL_BASE_GN: i32 = 2_254_871;
    const LOCAL_EVERY_EOF: i32 = 10;
    const LOCAL_TECTONIC_CODA: i32 = 12;
    const EVERY_EOF_TEXT_GN: u16 = 16;
    const TECTONIC_CODA_TEXT_GN: u16 = 19;
    const SELECTOR_LOG_ONLY_GN: i32 = 18;
    const INT_BASE_GN: i32 = 7_826_729;
    const INT_PAR_TRACING_NESTING: i32 = 60;
    const INT_PAR_END_LINE_CHAR_GN: i32 = 48;
    const TEX_NULL_GN: i32 = crate::mem::TEX_NULL;

    #[inline(always)]
    fn is_lc_hex(c: i32) -> bool {
        (c >= 0x30 && c <= 0x39) || (c >= 0x61 && c <= 0x66)
    }

    'restart: loop {
        set_cur_cs(0);

        if cur_input().state != TOKEN_LIST {
            'texswitch: loop {
                if cur_input().loc <= cur_input().limit {
                    // Read next character from the buffer
                    set_cur_chr(buffer_entry(cur_input().loc));
                    set_cur_input_loc(cur_input().loc + 1);

                    // Handle UTF-16 surrogate pair
                    if cur_chr() >= 0xD800 && cur_chr() < 0xDC00
                        && cur_input().loc <= cur_input().limit
                        && buffer_entry(cur_input().loc) >= 0xDC00
                        && buffer_entry(cur_input().loc) < 0xE000
                    {
                        let lower = buffer_entry(cur_input().loc) - 0xDC00;
                        set_cur_input_loc(cur_input().loc + 1);
                        set_cur_chr(65536 + (cur_chr() - 0xD800) * 1024 + lower);
                    }

                    'reswitch: loop {
                        let cat = eqtb_b32s1(CAT_CODE_BASE + cur_chr()) as u8;
                        set_cur_cmd(cat);
                        let sc = cur_input().state as i32 + cat as i32;

                        match sc {
                            // ANY_STATE_PLUS(IGNORE) + SKIP_BLANKS+SPACER + NEW_LINE+SPACER
                            // → goto texswitch
                            10 | 26 | 42 | 27 | 43 => { continue 'texswitch; }

                            // ANY_STATE_PLUS(ESCAPE): escape sequence
                            1 | 17 | 33 => {
                                if cur_input().loc > cur_input().limit {
                                    set_cur_cs(NULL_CS_GN);
                                } else {
                                    'start_cs: loop {
                                        let cs_start = cur_input().loc;
                                        let first_chr = buffer_entry(cs_start);
                                        let first_cat = eqtb_b32s1(CAT_CODE_BASE + first_chr) as u8;
                                        let mut k = cs_start + 1;
                                        set_cur_input_state(
                                            if first_cat == LETTER || first_cat == SPACER { SKIP_BLANKS } else { MID_LINE }
                                        );

                                        if first_cat == LETTER && k <= cur_input().limit {
                                            // Multi-letter control word
                                            let mut chr = buffer_entry(k);
                                            let mut cat2 = eqtb_b32s1(CAT_CODE_BASE + chr) as u8;
                                            k += 1;
                                            while cat2 == LETTER && k <= cur_input().limit {
                                                chr = buffer_entry(k);
                                                cat2 = eqtb_b32s1(CAT_CODE_BASE + chr) as u8;
                                                k += 1;
                                            }

                                            // Sup-mark (^^^) handling in multi-letter path
                                            if cat2 == SUP_MARK && buffer_entry(k) == chr && k < cur_input().limit {
                                                let mut sup_count: i32 = 2;
                                                while sup_count < 6
                                                    && k + 2*sup_count - 2 <= cur_input().limit
                                                    && buffer_entry(k + sup_count - 1) == chr
                                                {
                                                    sup_count += 1;
                                                }
                                                let sup_count_save = sup_count;
                                                let mut hex_ok = true;
                                                for d in 1..=sup_count_save {
                                                    if !is_lc_hex(buffer_entry(k + sup_count - 2 + d)) {
                                                        let c = buffer_entry(k + 1);
                                                        if c < 128 {
                                                            let new_val = if c < 64 { c + 64 } else { c - 64 };
                                                            set_buffer_entry(k - 1, new_val);
                                                            set_cur_input_limit(cur_input().limit - 2);
                                                            let mut kk = k;
                                                            while kk <= cur_input().limit {
                                                                set_buffer_entry(kk, buffer_entry(kk + 2));
                                                                kk += 1;
                                                            }
                                                            continue 'start_cs;
                                                        } else {
                                                            sup_count = 0;
                                                        }
                                                        hex_ok = false;
                                                        break;
                                                    }
                                                }
                                                if hex_ok && sup_count > 0 {
                                                    let mut new_chr: i32 = 0;
                                                    for d in 1..=sup_count {
                                                        let c = buffer_entry(k + sup_count - 2 + d);
                                                        new_chr = 16 * new_chr
                                                            + if c <= b'9' as i32 { c - b'0' as i32 } else { c - b'a' as i32 + 10 };
                                                    }
                                                    if new_chr <= BIGGEST_USV_GN {
                                                        set_buffer_entry(k - 1, new_chr);
                                                        let shift = 2 * sup_count - 1;
                                                        set_cur_input_limit(cur_input().limit - shift);
                                                        let mut kk = k;
                                                        while kk <= cur_input().limit {
                                                            set_buffer_entry(kk, buffer_entry(kk + shift));
                                                            kk += 1;
                                                        }
                                                        continue 'start_cs;
                                                    }
                                                    // else: new_chr > BIGGEST_USV — fall through
                                                }
                                            }

                                            if cat2 != LETTER { k -= 1; }
                                            if k > cur_input().loc + 1 {
                                                set_cur_cs(crate::hash::id_lookup(cur_input().loc, k - cur_input().loc));
                                                set_cur_input_loc(k);
                                                break 'start_cs; // goto found
                                            }
                                            // Single letter: fall through to single-char handling
                                        } else {
                                            // Single-char (non-LETTER first char) or end of line
                                            if first_cat == SUP_MARK && buffer_entry(k) == first_chr && k < cur_input().limit {
                                                let mut sup_count: i32 = 2;
                                                while sup_count < 6
                                                    && k + 2*sup_count - 2 <= cur_input().limit
                                                    && buffer_entry(k + sup_count - 1) == first_chr
                                                {
                                                    sup_count += 1;
                                                }
                                                let sup_count_save = sup_count;
                                                let mut hex_ok = true;
                                                for d in 1..=sup_count_save {
                                                    if !is_lc_hex(buffer_entry(k + sup_count - 2 + d)) {
                                                        let c = buffer_entry(k + 1);
                                                        if c < 128 {
                                                            let new_val = if c < 64 { c + 64 } else { c - 64 };
                                                            set_buffer_entry(k - 1, new_val);
                                                            set_cur_input_limit(cur_input().limit - 2);
                                                            let mut kk = k;
                                                            while kk <= cur_input().limit {
                                                                set_buffer_entry(kk, buffer_entry(kk + 2));
                                                                kk += 1;
                                                            }
                                                            continue 'start_cs;
                                                        } else {
                                                            sup_count = 0;
                                                        }
                                                        hex_ok = false;
                                                        break;
                                                    }
                                                }
                                                if hex_ok && sup_count > 0 {
                                                    let mut new_chr: i32 = 0;
                                                    for d in 1..=sup_count {
                                                        let c = buffer_entry(k + sup_count - 2 + d);
                                                        new_chr = 16 * new_chr
                                                            + if c <= b'9' as i32 { c - b'0' as i32 } else { c - b'a' as i32 + 10 };
                                                    }
                                                    if new_chr <= BIGGEST_USV_GN {
                                                        set_buffer_entry(k - 1, new_chr);
                                                        let shift = 2 * sup_count - 1;
                                                        set_cur_input_limit(cur_input().limit - shift);
                                                        let mut kk = k;
                                                        while kk <= cur_input().limit {
                                                            set_buffer_entry(kk, buffer_entry(kk + shift));
                                                            kk += 1;
                                                        }
                                                        continue 'start_cs;
                                                    }
                                                    // else: new_chr > BIGGEST_USV — fall through
                                                }
                                            }
                                        }

                                        // Single-char CS: either buffer > 65535 uses id_lookup(1), else SINGLE_BASE
                                        if buffer_entry(cur_input().loc) > 65535 {
                                            set_cur_cs(crate::hash::id_lookup(cur_input().loc, 1));
                                            set_cur_input_loc(cur_input().loc + 1);
                                            break 'start_cs; // goto found
                                        }
                                        set_cur_cs(SINGLE_BASE + buffer_entry(cur_input().loc));
                                        set_cur_input_loc(cur_input().loc + 1);
                                        break 'start_cs;
                                    }
                                }
                                // found:
                                set_cur_cmd(eqtb_b16s1(cur_cs()) as u8);
                                set_cur_chr(eqtb_b32s1(cur_cs()));
                                if cur_cmd() >= OUTER_CALL_GN {
                                    check_outer_validity();
                                }
                            }

                            // ANY_STATE_PLUS(ACTIVE_CHAR): active character
                            14 | 30 | 46 => {
                                set_cur_cs(cur_chr() + 1);
                                set_cur_cmd(eqtb_b16s1(cur_cs()) as u8);
                                set_cur_chr(eqtb_b32s1(cur_cs()));
                                set_cur_input_state(MID_LINE);
                                if cur_cmd() >= OUTER_CALL_GN {
                                    check_outer_validity();
                                }
                            }

                            // ANY_STATE_PLUS(SUP_MARK): caret (^^) expansion
                            8 | 24 | 40 => {
                                if cur_chr() == buffer_entry(cur_input().loc)
                                    && cur_input().loc < cur_input().limit
                                {
                                    let mut sup_count: i32 = 2;
                                    while sup_count < 6
                                        && cur_input().loc + 2*sup_count - 2 <= cur_input().limit
                                        && cur_chr() == buffer_entry(cur_input().loc + sup_count - 1)
                                    {
                                        sup_count += 1;
                                    }
                                    let sup_count_save = sup_count;
                                    let mut hex_ok = true;
                                    for d in 1..=sup_count_save {
                                        if !is_lc_hex(buffer_entry(cur_input().loc + sup_count - 2 + d)) {
                                            let c = buffer_entry(cur_input().loc + 1);
                                            if c < 128 {
                                                set_cur_input_loc(cur_input().loc + 2);
                                                set_cur_chr(if c < 64 { c + 64 } else { c - 64 });
                                                continue 'reswitch;
                                            }
                                            hex_ok = false;
                                            break;
                                        }
                                    }
                                    if hex_ok {
                                        let mut new_chr: i32 = 0;
                                        for d in 1..=sup_count {
                                            let c = buffer_entry(cur_input().loc + sup_count - 2 + d);
                                            new_chr = 16 * new_chr
                                                + if c <= b'9' as i32 { c - b'0' as i32 } else { c - b'a' as i32 + 10 };
                                        }
                                        if new_chr > BIGGEST_USV_GN {
                                            set_cur_chr(buffer_entry(cur_input().loc));
                                            // fall to not_exp
                                        } else {
                                            set_cur_chr(new_chr);
                                            set_cur_input_loc(cur_input().loc + 2 * sup_count - 1);
                                            continue 'reswitch;
                                        }
                                    }
                                }
                                // not_exp:
                                set_cur_input_state(MID_LINE);
                            }

                            // ANY_STATE_PLUS(INVALID_CHAR): invalid character
                            16 | 32 | 48 => {
                                crate::output::error_here_with_diagnostic(
                                    c"Text line contains an invalid character".as_ptr()
                                );
                                capture_to_diagnostic(core::ptr::null_mut());
                                set_help_ptr(2);
                                set_help_line(1, c"A funny symbol that I can't read has just been input.".as_ptr());
                                set_help_line(0, c"Continue, and I'll forget that it ever happened.".as_ptr());
                                set_deletions_allowed(false);
                                error();
                                set_deletions_allowed(true);
                                continue 'restart;
                            }

                            // MID_LINE + SPACER
                            11 => {
                                set_cur_input_state(SKIP_BLANKS);
                                set_cur_chr(b' ' as i32);
                            }

                            // MID_LINE + CAR_RET
                            6 => {
                                set_cur_input_loc(cur_input().limit + 1);
                                set_cur_cmd(SPACER);
                                set_cur_chr(b' ' as i32);
                            }

                            // ANY_STATE_PLUS(COMMENT) + SKIP_BLANKS+CAR_RET → skip rest of line, goto texswitch
                            15 | 31 | 47 | 22 => {
                                set_cur_input_loc(cur_input().limit + 1);
                                continue 'texswitch;
                            }

                            // NEW_LINE + CAR_RET: insert \par
                            38 => {
                                set_cur_input_loc(cur_input().limit + 1);
                                set_cur_cs(par_loc());
                                set_cur_cmd(eqtb_b16s1(cur_cs()) as u8);
                                set_cur_chr(eqtb_b32s1(cur_cs()));
                                if cur_cmd() >= OUTER_CALL_GN {
                                    check_outer_validity();
                                }
                            }

                            // MID_LINE + LEFT_BRACE
                            2 => {
                                set_align_state(align_state() + 1);
                            }

                            // SKIP_BLANKS + LEFT_BRACE | NEW_LINE + LEFT_BRACE
                            18 | 34 => {
                                set_cur_input_state(MID_LINE);
                                set_align_state(align_state() + 1);
                            }

                            // MID_LINE + RIGHT_BRACE
                            3 => {
                                set_align_state(align_state() - 1);
                            }

                            // SKIP_BLANKS + RIGHT_BRACE | NEW_LINE + RIGHT_BRACE
                            19 | 35 => {
                                set_cur_input_state(MID_LINE);
                                set_align_state(align_state() - 1);
                            }

                            // ADD_DELIMS_TO(SKIP_BLANKS): SKIP_BLANKS + MATH_SHIFT/TAB_MARK/MAC_PARAM/SUB_MARK/LETTER/OTHER_CHAR
                            20 | 21 | 23 | 25 | 28 | 29 => {
                                set_cur_input_state(MID_LINE);
                            }

                            // ADD_DELIMS_TO(NEW_LINE): NEW_LINE + MATH_SHIFT/TAB_MARK/MAC_PARAM/SUB_MARK/LETTER/OTHER_CHAR
                            36 | 37 | 39 | 41 | 44 | 45 => {
                                set_cur_input_state(MID_LINE);
                            }

                            _ => {} // default: do nothing
                        }
                        break 'reswitch;
                    }
                    break 'texswitch;
                } else {
                    // End of line / end of file
                    set_cur_input_state(NEW_LINE);

                    if cur_input().name > 17 {
                        set_line(line() + 1);
                        set_first(cur_input().start);

                        if !force_eof() {
                            if cur_input().name <= 19 {
                                if crate::input::pseudo_input() {
                                    set_cur_input_limit(last());
                                } else if eqtb_b32s1(LOCAL_BASE_GN + LOCAL_EVERY_EOF) != TEX_NULL_GN
                                    && !eof_seen(cur_input().index as i32)
                                {
                                    set_cur_input_limit(first() - 1);
                                    set_eof_seen(cur_input().index as i32, true);
                                    crate::input::begin_token_list(
                                        eqtb_b32s1(LOCAL_BASE_GN + LOCAL_EVERY_EOF),
                                        EVERY_EOF_TEXT_GN,
                                    );
                                    continue 'restart;
                                } else {
                                    set_force_eof(true);
                                }
                            } else {
                                if input_line(input_file(cur_input().index as i32) as *mut libc::c_void) != 0 {
                                    set_cur_input_limit(last());
                                } else if eqtb_b32s1(LOCAL_BASE_GN + LOCAL_EVERY_EOF) != TEX_NULL_GN
                                    && !eof_seen(cur_input().index as i32)
                                {
                                    set_cur_input_limit(first() - 1);
                                    set_eof_seen(cur_input().index as i32, true);
                                    crate::input::begin_token_list(
                                        eqtb_b32s1(LOCAL_BASE_GN + LOCAL_EVERY_EOF),
                                        EVERY_EOF_TEXT_GN,
                                    );
                                    continue 'restart;
                                } else {
                                    set_force_eof(true);
                                }
                            }
                        }

                        if force_eof() {
                            if eqtb_b32s1(INT_BASE_GN + INT_PAR_TRACING_NESTING) > 0 {
                                if grp_stack(in_open() as i32) != cur_boundary()
                                    || if_stack(in_open() as i32) != cond_ptr()
                                {
                                    crate::output::file_warning();
                                }
                            }
                            if cur_input().name >= 19 {
                                crate::output::print_char(b')' as i32);
                                set_open_parens(open_parens() - 1);
                                ttstub_output_flush(rust_stdout());
                            }
                            set_force_eof(false);
                            crate::input::end_file_reading();
                            check_outer_validity();
                            continue 'restart;
                        }

                        let end_char = eqtb_b32s1(INT_BASE_GN + INT_PAR_END_LINE_CHAR_GN);
                        if end_char < 0 || end_char > 255 {
                            set_cur_input_limit(cur_input().limit - 1);
                        } else {
                            set_buffer_entry(cur_input().limit, end_char);
                        }
                        set_first(cur_input().limit + 1);
                        set_cur_input_loc(cur_input().start);
                    } else {
                        if cur_input().name != 0 {
                            set_cur_cmd(0);
                            set_cur_chr(0);
                            return;
                        }
                        if input_ptr() > 0 {
                            crate::input::end_file_reading();
                            continue 'restart;
                        }
                        if !used_tectonic_coda_tokens()
                            && eqtb_b32s1(LOCAL_BASE_GN + LOCAL_TECTONIC_CODA) != TEX_NULL_GN
                        {
                            set_used_tectonic_coda_tokens(true);
                            crate::input::begin_token_list(
                                eqtb_b32s1(LOCAL_BASE_GN + LOCAL_TECTONIC_CODA),
                                TECTONIC_CODA_TEXT_GN,
                            );
                            continue 'restart;
                        }
                        if selector() < SELECTOR_LOG_ONLY_GN {
                            open_log_file();
                        }
                        crate::errors::fatal_error(
                            c"*** (job aborted, no legal \\end found)".as_ptr()
                        );
                    }
                    continue 'texswitch;
                }
            }
        } else if cur_input().loc != TEX_NULL_GN {
            // Reading from a token list
            let t = mem_b32s0(cur_input().loc);
            set_cur_input_loc(mem_b32s1(cur_input().loc)); // LLIST_link

            if t >= CS_TOKEN_FLAG_GN {
                set_cur_cs(t - CS_TOKEN_FLAG_GN);
                set_cur_cmd(eqtb_b16s1(cur_cs()) as u8);
                set_cur_chr(eqtb_b32s1(cur_cs()));
                if cur_cmd() >= OUTER_CALL_GN {
                    if cur_cmd() == DONT_EXPAND_GN {
                        set_cur_cs(mem_b32s0(cur_input().loc) - CS_TOKEN_FLAG_GN);
                        set_cur_input_loc(TEX_NULL_GN);
                        set_cur_cmd(eqtb_b16s1(cur_cs()) as u8);
                        set_cur_chr(eqtb_b32s1(cur_cs()));
                        if cur_cmd() > MAX_COMMAND_GN {
                            set_cur_cmd(RELAX_GN);
                            set_cur_chr(NO_EXPAND_FLAG_GN);
                        }
                    } else {
                        check_outer_validity();
                    }
                }
            } else {
                set_cur_cmd((t / MAX_CHAR_VAL_GN) as u8);
                set_cur_chr(t % MAX_CHAR_VAL_GN);
                match cur_cmd() {
                    x if x == LEFT_BRACE_CAT => {
                        set_align_state(align_state() + 1);
                    }
                    x if x == RIGHT_BRACE_CAT => {
                        set_align_state(align_state() - 1);
                    }
                    x if x == OUT_PARAM_GN => {
                        crate::input::begin_token_list(
                            param_stack(cur_input().limit + cur_chr() - 1),
                            PARAMETER_GN,
                        );
                        continue 'restart;
                    }
                    _ => {}
                }
            }
        } else {
            // Token list exhausted
            crate::input::end_token_list();
            continue 'restart;
        }

        // Alignment preamble check (§818)
        if cur_cmd() <= CAR_RET && cur_cmd() >= TAB_MARK_CAT && align_state() == 0 {
            if scanner_status() == ALIGNING_GN || cur_align() == TEX_NULL_GN {
                crate::errors::fatal_error(
                    c"(interwoven alignment preambles are not allowed)".as_ptr()
                );
            }
            let saved_cmd = mem_b32s0(cur_align() + 5);
            set_mem_b32s0(cur_align() + 5, cur_chr());
            if saved_cmd as u8 == OMIT_GN {
                crate::input::begin_token_list(OMIT_TEMPLATE_GN, V_TEMPLATE_GN);
            } else {
                crate::input::begin_token_list(mem_b32s1(cur_align() + 2), V_TEMPLATE_GN);
            }
            set_cur_cmd(saved_cmd as u8);
            set_align_state(1_000_000);
            continue 'restart;
        }
        break 'restart;
    }
}

// translated from xetex-xetex0.c:3665 (macro_call)
#[no_mangle]
pub unsafe extern "C" fn macro_call() {
    use crate::globals::{
        align_state, avail, cur_chr, cur_cs, cur_tok, eqtb_b16s1, input_ptr,
        long_state, max_param_stack, mem_b32s0, mem_b32s1, param_ptr, param_size,
        param_stack, par_token, scanner_status, set_align_state, set_avail,
        set_cur_input_loc, set_cur_input_name, set_cur_tok, set_long_state,
        set_max_param_stack, set_mem_b32s0, set_mem_b32s1, set_param_ptr,
        set_param_stack, set_scanner_status, set_warning_index, warning_index,
    };
    use crate::mem::{flush_list, get_avail, TEX_NULL};

    const MATCHING: u8 = 3;          // xetex-constants.h:185
    const LONG_CALL: u8 = 114;       // xetex_format.h:356
    const OUTER_CALL: u8 = 115;      // xetex_format.h:357
    const MACRO: u16 = 6;            // xetex-constants.h (token-list type)
    const V_TEMPLATE: u16 = 2;       // xetex-constants.h:201
    const TOKEN_LIST: u16 = 0;       // xetex-constants.h:261
    const INT_PAR_TRACING_MACROS: i32 = 30;      // xetex_format.h:65
    const INT_PAR_TRACING_STACK_LEVELS: i32 = 55; // xetex_format.h:90

    let save_scanner_status = scanner_status();
    let save_warning_index = warning_index();
    set_warning_index(cur_cs());
    let ref_count = cur_chr();
    let mut r = mem_b32s1(ref_count);
    let mut n: i32 = 0;

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_MACROS) > 0 {
        crate::output::begin_diagnostic();
        crate::output::diagnostic_begin_capture_warning_here();
        let tsl = eqtb_b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS);
        if tsl > 0 {
            if input_ptr() < tsl {
                let mut v = input_ptr();
                crate::output::print_ln();
                crate::output::print_char(b'~' as i32);
                while v > 0 {
                    crate::output::print_char(b'.' as i32);
                    v -= 1;
                }
                crate::output::print_cs(warning_index());
                crate::output::token_show(ref_count);
            } else {
                crate::output::print_char(b'~' as i32);
                crate::output::print_char(b'~' as i32);
                crate::output::print_cs(warning_index());
            }
        } else {
            crate::output::print_ln();
            crate::output::print_cs(warning_index());
            crate::output::token_show(ref_count);
        }
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::end_diagnostic(false);
    }

    if mem_b32s0(r) == PROTECTED_TOKEN {
        r = mem_b32s1(r);
    }

    // pstack: accumulates gathered parameter token lists (max 9 params)
    let mut pstack: [i32; 9] = [TEX_NULL; 9];
    let mut match_chr: i32 = 0;
    let mut rbrace_ptr: i32 = TEX_NULL;

    'mac: {
        if mem_b32s0(r) != END_MATCH_TOKEN {
            set_scanner_status(MATCHING);
            let mut unbalance: i32 = 0;
            set_long_state(eqtb_b16s1(cur_cs()) as u8);
            if long_state() >= OUTER_CALL {
                set_long_state(long_state() - 2);
            }

            'outer: loop {
                set_mem_b32s1(TEMP_HEAD, TEX_NULL);
                // Determine whether this iteration gathers a delimited parameter
                let s: i32;
                let mut p: i32;
                let mut m: i32;
                if mem_b32s0(r) >= END_MATCH_TOKEN || mem_b32s0(r) < MATCH_TOKEN {
                    s = TEX_NULL;
                    p = TEX_NULL; // unused when s == TEX_NULL
                    m = 0;
                } else {
                    match_chr = mem_b32s0(r) - MATCH_TOKEN;
                    s = mem_b32s1(r);
                    r = s;
                    p = TEMP_HEAD;
                    m = 0;
                }

                'continue_: loop {
                    get_token();

                    if cur_tok() == mem_b32s0(r) {
                        r = mem_b32s1(r);
                        if mem_b32s0(r) >= MATCH_TOKEN && mem_b32s0(r) <= END_MATCH_TOKEN {
                            if cur_tok() < LEFT_BRACE_LIMIT {
                                set_align_state(align_state() - 1);
                            }
                            break 'continue_; // goto found
                        } else {
                            continue 'continue_; // goto continue_
                        }
                    }

                    if s != r {
                        if s == TEX_NULL {
                            error_here_with_diagnostic(c"Use of ".as_ptr());
                            crate::output::sprint_cs(warning_index());
                            crate::output::print_cstr(c" doesn't match its definition".as_ptr());
                            capture_to_diagnostic(std::ptr::null_mut());
                            set_help_ptr(4);
                            set_help_line(3, c"If you say, e.g., `\\def\\a1{...}', then you must always".as_ptr());
                            set_help_line(2, c"put `1' after `\\a', since control sequence names are".as_ptr());
                            set_help_line(1, c"made up of letters only. The macro here has not been".as_ptr());
                            set_help_line(0, c"followed by the required stuff, so I'm ignoring it.".as_ptr());
                            crate::errors::error();
                            break 'mac; // goto exit
                        } else {
                            let mut t = s;
                            loop {
                                let q = get_avail();
                                set_mem_b32s1(p, q);
                                set_mem_b32s0(q, mem_b32s0(t));
                                p = q;
                                m += 1;
                                let mut u = mem_b32s1(t);
                                let mut v = s;
                                loop {
                                    if u == r {
                                        if cur_tok() != mem_b32s0(v) {
                                            break; // goto done
                                        } else {
                                            r = mem_b32s1(v);
                                            continue 'continue_; // goto continue_
                                        }
                                    }
                                    if mem_b32s0(u) != mem_b32s0(v) {
                                        break; // goto done
                                    }
                                    u = mem_b32s1(u);
                                    v = mem_b32s1(v);
                                }
                                // done:
                                t = mem_b32s1(t);
                                if t == r { break; }
                            }
                            r = s;
                        }
                    }

                    if cur_tok() == par_token() {
                        if long_state() != LONG_CALL {
                            if long_state() == CALL {
                                runaway();
                                error_here_with_diagnostic(c"Paragraph ended before ".as_ptr());
                                crate::output::sprint_cs(warning_index());
                                crate::output::print_cstr(c" was complete".as_ptr());
                                capture_to_diagnostic(std::ptr::null_mut());
                                set_help_ptr(3);
                                set_help_line(2, c"I suspect you've forgotten a `}', causing me to apply this".as_ptr());
                                set_help_line(1, c"control sequence to too much text. How can we recover?".as_ptr());
                                set_help_line(0, c"My plan is to forget the whole thing and hope for the best.".as_ptr());
                                crate::input::back_error();
                            }
                            pstack[n as usize] = mem_b32s1(TEMP_HEAD);
                            set_align_state(align_state() - unbalance);
                            for m2 in 0..=n {
                                flush_list(pstack[m2 as usize]);
                            }
                            break 'mac; // goto exit
                        }
                    }

                    if cur_tok() < RIGHT_BRACE_LIMIT {
                        if cur_tok() < LEFT_BRACE_LIMIT {
                            // left brace: gather a braced group
                            unbalance = 1;
                            loop {
                                let q = if avail() != TEX_NULL {
                                    let q = avail();
                                    set_avail(mem_b32s1(q));
                                    set_mem_b32s1(q, TEX_NULL);
                                    q
                                } else {
                                    get_avail()
                                };
                                set_mem_b32s1(p, q);
                                set_mem_b32s0(q, cur_tok());
                                p = q;

                                get_token();

                                if cur_tok() == par_token() {
                                    if long_state() != LONG_CALL {
                                        if long_state() == CALL {
                                            runaway();
                                            error_here_with_diagnostic(c"Paragraph ended before ".as_ptr());
                                            crate::output::sprint_cs(warning_index());
                                            crate::output::print_cstr(c" was complete".as_ptr());
                                            capture_to_diagnostic(std::ptr::null_mut());
                                            set_help_ptr(3);
                                            set_help_line(2, c"I suspect you've forgotten a `}', causing me to apply this".as_ptr());
                                            set_help_line(1, c"control sequence to too much text. How can we recover?".as_ptr());
                                            set_help_line(0, c"My plan is to forget the whole thing and hope for the best.".as_ptr());
                                            crate::input::back_error();
                                        }
                                        pstack[n as usize] = mem_b32s1(TEMP_HEAD);
                                        set_align_state(align_state() - unbalance);
                                        for m2 in 0..=n {
                                            flush_list(pstack[m2 as usize]);
                                        }
                                        break 'mac; // goto exit
                                    }
                                }

                                if cur_tok() < RIGHT_BRACE_LIMIT {
                                    if cur_tok() < LEFT_BRACE_LIMIT {
                                        unbalance += 1;
                                    } else {
                                        unbalance -= 1;
                                        if unbalance == 0 { break; } // done1
                                    }
                                }
                            }
                            // done1:
                            rbrace_ptr = p;
                            let q = get_avail();
                            set_mem_b32s1(p, q);
                            set_mem_b32s0(q, cur_tok());
                            p = q;
                        } else {
                            // right brace with no matching left brace (413)
                            crate::input::back_input();
                            error_here_with_diagnostic(c"Argument of ".as_ptr());
                            crate::output::sprint_cs(warning_index());
                            crate::output::print_cstr(c" has an extra }".as_ptr());
                            capture_to_diagnostic(std::ptr::null_mut());
                            set_help_ptr(6);
                            set_help_line(5, c"I've run across a `}' that doesn't seem to match anything.".as_ptr());
                            set_help_line(4, c"For example, `\\def\\a#1{...}' and `\\a}' would produce".as_ptr());
                            set_help_line(3, c"this error. If you simply proceed now, the `\\par' that".as_ptr());
                            set_help_line(2, c"I've just inserted will cause me to report a runaway".as_ptr());
                            set_help_line(1, c"argument that might be the root of the problem. But if".as_ptr());
                            set_help_line(0, c"your `}' was spurious, just type `2' and it will go away.".as_ptr());
                            set_align_state(align_state() + 1);
                            set_long_state(CALL);
                            set_cur_tok(par_token());
                            crate::input::ins_error();
                            continue 'continue_; // goto continue_
                        }
                    } else {
                        // 411: non-brace token
                        if cur_tok() == SPACE_TOKEN {
                            if mem_b32s0(r) <= END_MATCH_TOKEN && mem_b32s0(r) >= MATCH_TOKEN {
                                continue 'continue_; // goto continue_
                            }
                        }
                        let q = get_avail();
                        set_mem_b32s1(p, q);
                        set_mem_b32s0(q, cur_tok());
                        p = q;
                    }

                    m += 1;
                    if mem_b32s0(r) > END_MATCH_TOKEN || mem_b32s0(r) < MATCH_TOKEN {
                        continue 'continue_; // goto continue_
                    }
                    break 'continue_; // goto found
                } // end 'continue_

                // found:
                if s != TEX_NULL {
                    if m == 1 && mem_b32s0(p) < RIGHT_BRACE_LIMIT {
                        // strip outer braces from single-token braced arg
                        set_mem_b32s1(rbrace_ptr, TEX_NULL);
                        set_mem_b32s1(p, avail());
                        set_avail(p);
                        p = mem_b32s1(TEMP_HEAD);
                        pstack[n as usize] = mem_b32s1(p);
                        set_mem_b32s1(p, avail());
                        set_avail(p);
                    } else {
                        pstack[n as usize] = mem_b32s1(TEMP_HEAD);
                    }

                    n += 1;

                    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_MACROS) > 0 {
                        let tsl = eqtb_b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS);
                        if tsl == 0 || input_ptr() < tsl {
                            crate::output::begin_diagnostic();
                            crate::output::diagnostic_begin_capture_warning_here();
                            crate::output::print_nl(match_chr);
                            crate::output::print_int(n);
                            crate::output::print_cstr(c"<-".as_ptr());
                            crate::output::show_token_list(pstack[(n - 1) as usize], TEX_NULL, 1000);
                            capture_to_diagnostic(std::ptr::null_mut());
                            crate::output::end_diagnostic(false);
                        }
                    }
                }

                if mem_b32s0(r) == END_MATCH_TOKEN { break 'outer; }
            } // end 'outer
        } // end if mem_b32s0(r) != END_MATCH_TOKEN

        // Flush exhausted token lists before installing macro body
        while cur_input().state == TOKEN_LIST
            && cur_input().loc == TEX_NULL
            && cur_input().index != V_TEMPLATE
        {
            crate::input::end_token_list();
        }

        crate::input::begin_token_list(ref_count, MACRO);
        set_cur_input_name(warning_index());
        set_cur_input_loc(mem_b32s1(r));

        if n > 0 {
            if param_ptr() + n > max_param_stack() {
                set_max_param_stack(param_ptr() + n);
                if max_param_stack() > param_size() {
                    crate::errors::overflow(c"parameter stack size".as_ptr(), param_size());
                }
            }
            for m in 0..n {
                set_param_stack(param_ptr() + m, pstack[m as usize]);
            }
            set_param_ptr(param_ptr() + n);
        }
    } // end 'mac (exit:)

    set_scanner_status(save_scanner_status);
    set_warning_index(save_warning_index);
}

// ---------------------------------------------------------------------------
// new_native_word_node
// ---------------------------------------------------------------------------

// Node type/subtype constants (xetex_format.h)
const WHATSIT_NODE_NN: u16 = 8;
const NATIVE_WORD_NODE_NN: u16 = 40;
const NATIVE_WORD_NODE_AT_NN: u16 = 41;
const NATIVE_NODE_SIZE_NN: i32 = 6;
// INT_PAR offset for xetex_generate_actual_text (xetex_format.h:114)
const INT_PAR_XETEX_GENERATE_ACTUAL_TEXT: i32 = 79;

/// Allocate a native-word whatsit node for `n` UTF-16 code units of font `f`.
///
/// translated from xetex-xetex0.c:6250 (new_native_word_node)
#[no_mangle]
pub unsafe extern "C" fn new_native_word_node(f: i32, n: i32) -> i32 {
    use crate::globals::{set_mem_b16s0, set_mem_b16s1, set_mem_b16s2, set_mem_b16s3, set_mem_ptr};
    // sizeof(UTF16_code) = 2, sizeof(memory_word) = 8
    let l = NATIVE_NODE_SIZE_NN + (n * 2 + 7) / 8;
    let q = crate::mem::get_node(l);
    set_mem_b16s1(q, WHATSIT_NODE_NN);
    if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_GENERATE_ACTUAL_TEXT) > 0 {
        set_mem_b16s0(q, NATIVE_WORD_NODE_AT_NN);
    } else {
        set_mem_b16s0(q, NATIVE_WORD_NODE_NN);
    }
    set_mem_b16s3(q + 4, l as u16);
    set_mem_b16s2(q + 4, f as u16);
    set_mem_b16s1(q + 4, n as u16);
    set_mem_b16s0(q + 4, 0);
    set_mem_ptr(q + 5, std::ptr::null_mut());
    q
}

// ---------------------------------------------------------------------------
// do_locale_linebreaks
// ---------------------------------------------------------------------------

// GLUE_BASE + GLUE_PAR__xetex_linebreak_skip (xetex_format.h:205,164)
const GLUE_BASE_DO_LB: i32 = 2_254_340;
const GLUE_PAR_XETEX_LINEBREAK_SKIP: i32 = 15;
// INT_PAR offsets (xetex_format.h:101,102,107)
const INT_PAR_XETEX_LINEBREAK_LOCALE: i32 = 66;
const INT_PAR_XETEX_LINEBREAK_PENALTY: i32 = 67;
const INT_PAR_XETEX_USE_GLYPH_METRICS: i32 = 72;

extern "C" {
    fn linebreak_start(f: i32, locale_str_num: i32, text: *mut u16, text_length: i32);
    fn linebreak_next(f: i32) -> i32;
    static mut main_f: i32;
    static mut native_text: *mut u16;
}

/// Insert locale-aware linebreaking nodes for `len` glyphs starting at `native_text[s]`.
///
/// translated from xetex-xetex0.c:6493 (do_locale_linebreaks)
#[no_mangle]
pub unsafe extern "C" fn do_locale_linebreaks(s: i32, len: i32) {
    use crate::globals::{
        cur_list_tail, set_cur_list_tail,
        mem_as_u16_ptr_mut,
        mem_b32s1, set_mem_b32s1,
    };

    let locale = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_LINEBREAK_LOCALE);
    let use_glyph = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0;

    if locale == 0 || len == 1 {
        let node = new_native_word_node(main_f, len);
        set_mem_b32s1(cur_list_tail(), node);
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        let text = mem_as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE_NN);
        for i in 0..len {
            *text.add(i as usize) = *native_text.add((s + i) as usize);
        }
        measure_native_node(crate::globals::mem_node_addr(cur_list_tail()), use_glyph);
    } else {
        let use_skip = eqtb_b32s1(GLUE_BASE_DO_LB + GLUE_PAR_XETEX_LINEBREAK_SKIP) != 0;
        let penalty_val = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_LINEBREAK_PENALTY);
        let use_penalty = penalty_val != 0 || !use_skip;
        linebreak_start(main_f, locale, native_text.add(s as usize), len);
        let mut offs: i32 = 0;
        loop {
            let prev_offs = offs;
            offs = linebreak_next(main_f);
            if offs > 0 {
                if prev_offs != 0 {
                    if use_penalty {
                        let node = crate::mem::new_penalty(penalty_val);
                        set_mem_b32s1(cur_list_tail(), node);
                        set_cur_list_tail(mem_b32s1(cur_list_tail()));
                    }
                    if use_skip {
                        let node = crate::mem::new_param_glue(GLUE_PAR_XETEX_LINEBREAK_SKIP as i16);
                        set_mem_b32s1(cur_list_tail(), node);
                        set_cur_list_tail(mem_b32s1(cur_list_tail()));
                    }
                }
                let node = new_native_word_node(main_f, offs - prev_offs);
                set_mem_b32s1(cur_list_tail(), node);
                set_cur_list_tail(mem_b32s1(cur_list_tail()));
                let text = mem_as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE_NN);
                for i in prev_offs..offs {
                    *text.add((i - prev_offs) as usize) = *native_text.add((s + i) as usize);
                }
                measure_native_node(crate::globals::mem_node_addr(cur_list_tail()), use_glyph);
            }
            if offs < 0 { break; }
        }
    }
}
