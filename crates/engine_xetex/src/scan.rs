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
    set_mem_b16s2, set_mem_b16s3, set_mem_b32s0, set_mem_b32s1, set_name_in_progress,
    set_no_new_control_sequence, set_old_setting, set_open_parens, set_pool_ptr,
    set_pseudo_files, set_read_open_entry, set_save_ptr, set_save_stack_b32s1,
    set_scanner_status, set_selector, set_stop_at_space, set_str_pool, set_str_ptr,
    set_warning_index, set_xtx_ligature_present, stop_at_space, str_pool_entry, str_ptr,
    str_start, term_offset, warning_index, xtx_ligature_present,
    char_base_entry, font_info_b16, cur_list_mode, cur_group,
    active_width, set_active_width, set_best_height_plus_depth,
};
use crate::mem::new_rule;

extern "C" {
    fn expand();
    fn get_next();
    fn macro_call();
    fn begin_token_list(p: i32, t: u16);
    fn xetex_scan_dimen(mu: bool, inf: bool, shortcut: bool, requires_units: bool);
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
    fn show_box(p: i32);
    fn flush_node_list(p: i32);
    fn _tt_abort(fmt: *const libc::c_char) -> !;
    fn show_token_list(p: i32, q: i32, l: i32);
    fn ttstub_output_open(path: *const libc::c_char, gzip: libc::c_int) -> usize;
    fn print_cmd_chr(cmd: u16, chr_code: i32);
    fn scan_glyph_number(f: i32);
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
    fn do_marks(a: i16, l: i16, q: i32) -> bool;
    fn prune_page_top(p: i32, s: bool) -> i32;
    fn ship_out(p: i32);
    fn synctex_start_input();
    fn ttstub_issue_warning(fmt: *const libc::c_char, ...) -> libc::c_void;
    fn hpack(p: i32, w: i32, m: i16) -> i32;
    fn vpackage(p: i32, h: i32, m: i16, l: i32) -> i32;

    fn flush_math();
    fn resume_after_display();
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
    show_token_list(mem_b32s1(def_ref()), crate::mem::TEX_NULL, pool_size() - pool_ptr());
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

extern "C" { fn show_node_list(p: i32); }

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
    show_token_list(mem_b32s1(p), crate::mem::TEX_NULL, pool_size() - pool_ptr());
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
    extern "C" {
        fn show_activities();
    }
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
        fn hpack(p: i32, w: i32, m: i16) -> i32;
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
    extern "C" {
        fn copy_node_list(p: i32) -> i32;
    }
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
    extern "C" { fn begin_box(box_context: i32); }

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
        show_token_list(mem_b32s1(p), TEX_NULL, error_line() - 10);
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
