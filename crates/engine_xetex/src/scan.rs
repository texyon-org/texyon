//! Scanner range-validation helpers and mu_error.
//!
//! Translated from xetex-xetex0.c.

use crate::globals::{
    align_state, arith_error, avail, buffer_entry, cur_chr, cur_cmd, cur_cs, cur_input, cur_tok,
    cur_val, cur_val_level, def_ref, eqtb_b32s1, eqtb_is_null, expand_depth,
    expand_depth_count, file_offset, first, fmem_ptr, font_glue, font_mem_size, font_params,
    font_ptr, font_mapping_ptr, hash_s1, init_pool_ptr, last, max_print_line, max_reg_help_line,
    max_reg_num, mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1, old_setting, open_parens,
    param_base, pool_ptr, pool_size, pseudo_files, read_file_entry, read_open_entry, rust_stdout,
    save_ptr, save_stack_b32s1, scanner_status, selector, set_align_state, set_arith_error,
    set_avail, set_buffer_entry, set_cur_chr, set_cur_cmd, set_cur_cs, set_cur_input_limit,
    set_cur_input_loc, set_cur_input_name, set_cur_input_start, set_cur_input_state,
    set_cur_input_synctex_tag, set_cur_tok, set_cur_val, set_cur_val_level, set_def_ref,
    set_expand_depth_count, set_first, set_fmem_ptr, set_font_glue, set_font_info_b32s1,
    set_font_params, set_help_line, set_help_ptr, set_line, set_mem_b16s0, set_mem_b16s1,
    set_mem_b16s2, set_mem_b16s3, set_mem_b32s0, set_mem_b32s1, set_name_in_progress,
    set_no_new_control_sequence, set_old_setting, set_open_parens, set_pool_ptr,
    set_pseudo_files, set_read_open_entry, set_save_ptr, set_save_stack_b32s1,
    set_scanner_status, set_selector, set_stop_at_space, set_str_pool, set_str_ptr,
    set_warning_index, set_xtx_ligature_present, stop_at_space, str_pool_entry, str_ptr,
    str_start, term_offset, warning_index, xtx_ligature_present,
    char_base_entry, font_info_b16, cur_list_mode,
};
use crate::mem::new_rule;

extern "C" {
    fn expand();
    fn get_next();
    fn macro_call();
    fn begin_token_list(p: i32, t: u16);
    fn xetex_scan_dimen(mu: bool, inf: bool, shortcut: bool, requires_units: bool);
    fn scan_glue(level: i16);
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
    fn runaway();
    fn line_break(d: bool);
    fn build_page();
    fn show_box(p: i32);
    fn flush_node_list(p: i32);
    fn _tt_abort(fmt: *const libc::c_char) -> !;
    fn pack_file_name(n: i32, a: i32, e: i32);
    fn show_token_list(p: i32, q: i32, l: i32);
    fn ttstub_output_open(path: *const libc::c_char, gzip: libc::c_int) -> usize;
    // C functions called by scan_something_internal / scan_int
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
    fn synctex_start_input();
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
