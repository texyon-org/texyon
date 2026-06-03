//! Scanner range-validation helpers and mu_error.
//!
//! Translated from xetex-xetex0.c.
use crate::consts::{
    ACTIVE_BASE, BIGGEST_CHAR, BOX_BASE, CAT_CODE_BASE, CS_TOKEN_FLAG, CUR_FONT_LOC, DEL_CODE_BASE,
    EQTB_SIZE, GLUE_NODE, INT_BASE, KERN_NODE, LOCAL_BASE, MATH_CODE_BASE, MATH_NODE, MAX_HALFWORD,
    PENALTY_NODE, PRIM_EQTB_BASE, SF_CODE_BASE, SINGLE_BASE, TEX_NULL, TOKS_BASE, TOO_BIG_CHAR,
};
use crate::errors::error;
use crate::errors::int_error;
use crate::errors::not_aat_font_error;
use crate::errors::not_aat_gr_font_error;
use crate::errors::not_native_font_error;
use crate::errors::not_ot_font_error;
// Relocated to errors.rs (workstream 02); still called from scan.rs.
use crate::errors::{mu_error, runaway};
use crate::ext::aat_font_get;
use crate::ext::aat_font_get_1;
use crate::ext::aat_font_get_2;
use crate::ext::aat_font_get_named;
use crate::ext::aat_font_get_named_1;
use crate::ext::get_font_char_range_global;
use crate::ext::get_glyph_bounds_global;
use crate::ext::getnativechardp_global;
use crate::ext::getnativecharht_global;
use crate::ext::getnativecharic_global;
use crate::ext::getnativecharwd_global;
use crate::ext::gr_font_get_named;
use crate::ext::gr_font_get_named_1;
use crate::ext::map_char_to_glyph_global;
use crate::ext::map_glyph_to_index_global;
use crate::ext::ot_font_get;
use crate::ext::ot_font_get_1;
use crate::ext::ot_font_get_2;
use crate::ext::ot_font_get_3;
use crate::input::begin_token_list;
use crate::io::input_line;
use crate::io::u_close;
use crate::mem::new_spec_global;
use crate::output::capture_to_diagnostic;
use crate::output::error_here_with_diagnostic;
use crate::pic::count_pdf_file_pages;

use crate::globals::{
    align_state, arith_error, avail, buffer_entry, cur_chr, cur_cmd, cur_cs, cur_input, cur_tok,
    cur_val, cur_val_level, def_ref, eqtb_b32s1, eqtb_is_null, expand_depth, expand_depth_count,
    file_offset, first, fmem_ptr, font_glue, font_mem_size, font_params, font_ptr, hash_s1,
    init_pool_ptr, last, max_print_line, max_reg_help_line, max_reg_num, mem_b16s0, mem_b16s1,
    mem_b32s0, mem_b32s1, old_setting, open_parens, param_base, pool_ptr, pool_size, pseudo_files,
    read_file_entry, read_open_entry, rust_stdout, save_ptr, save_stack_b32s1, scanner_status,
    selector, set_align_state, set_arith_error, set_avail, set_buffer_entry, set_cur_chr,
    set_cur_cmd, set_cur_cs, set_cur_input_limit, set_cur_input_loc, set_cur_input_name,
    set_cur_input_state, set_cur_input_synctex_tag, set_cur_tok, set_cur_val, set_cur_val_level,
    set_def_ref, set_expand_depth_count, set_first, set_fmem_ptr, set_font_glue,
    set_font_info_b32s1, set_font_params, set_help_line, set_help_ptr, set_line, set_mem_b16s0,
    set_mem_b16s1, set_mem_b16s2, set_mem_b16s3, set_mem_b32s0, set_mem_b32s1,
    set_no_new_control_sequence, set_old_setting, set_open_parens, set_pool_ptr, set_pseudo_files,
    set_read_open_entry, set_save_ptr, set_save_stack_b32s1, set_scanner_status, set_selector,
    set_str_pool, set_str_ptr, set_warning_index, str_pool_entry, str_ptr, str_start, term_offset,
    warning_index,
};
use crate::mem::new_rule_global;
// Relocated to packaging.rs (workstream 02); still called from scan.rs.
// Relocated to expand.rs (workstream 02); still called from scan.rs.
use crate::expand::{expand, macro_call};
// Relocated to main_control.rs (workstream 02); still called from scan.rs.
use crate::main_control::push_math;

use crate::fonts::{effective_char, effective_char_info, new_native_word_node};
use crate::io::{open_log_file, scan_and_pack_name};
use crate::output::print_cmd_chr;

extern "C" {

    fn ttstub_diag_printf(d: *mut std::ffi::c_void, fmt: *const libc::c_char, ...) -> libc::c_int;

    fn ttstub_output_flush(handle: usize) -> libc::c_int;

    fn _tt_abort(fmt: *const libc::c_char) -> !;

    fn usingGraphite(engine: *mut libc::c_void) -> bool;
    fn usingOpenType(engine: *mut libc::c_void) -> bool;

    fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;

}

// Value-type constants for scan_glue (xetex_format.h:568–570).
const GLUE_VAL: i16 = 2;
const MU_VAL_SCAN: i16 = 3;

// INT_BASE + INT_PAR__xetex_hyphenatable_length offset for max_hyphenatable_length
// (xetex_format.h:568, xetex_format.h:115; xetex_format.h:7826729 = INT_BASE).
const INT_PAR_XETEX_HYPHENATABLE_LENGTH: i32 = 80; // xetex_format.h:115
const INT_PAR_XETEX_INPUT_NORMALIZATION: i32 = 74; // xetex_format.h:109
const INT_PAR_XETEX_TRACING_FONTS: i32 = 77; // xetex_format.h:112
const HYPHENATABLE_LENGTH_LIMIT: i32 = 4095; // xetex-constants.h:334

// xetex-constants.h:26
const BIGGEST_USV: i32 = 0x10_FFFF;
// xetex-constants.h:335
const CHAR_CLASS_LIMIT: i32 = 4096;
// xetex-constants.h:343
const ACTIVE_MATH_CHAR: i32 = 0x1F_FFFF;
// xetex-constants.h:25
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
const COUNT_BASE: i32 = 7_826_812;
const SCALED_BASE: i32 = 8_941_203;
const FONT_ID_BASE: i32 = 2_245_338; // FROZEN_NULL_FONT
const HASH_BASE: i32 = 2_228_226;
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
pub unsafe fn scan_four_bit_int() {
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
        set_help_line(
            1,
            c"A mathchar number must be between 0 and 32767.".as_ptr(),
        );
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
    if v > HYPHENATABLE_LENGTH_LIMIT {
        HYPHENATABLE_LENGTH_LIMIT
    } else {
        v
    }
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
    if eqtb_is_null() {
        0
    } else {
        eqtb_b32s1(INT_BASE + INT_PAR_XETEX_INPUT_NORMALIZATION)
    }
}

/// Returns INTPAR(xetex_tracing_fonts).
///
/// translated from xetex-xetex0.c:9082 (get_tracing_fonts_state)
pub unsafe fn get_tracing_fonts_state() -> i32 {
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
    set_help_line(
        1,
        c"I've inserted a begin-math/end-math symbol since I think".as_ptr(),
    );
    set_help_line(
        0,
        c"you left one out. Proceed, with fingers crossed.".as_ptr(),
    );
    crate::input::ins_error();
}

/// translated from xetex-xetex0.c:4355 (scan_optional_equals)
pub unsafe fn scan_optional_equals() {
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
const LEFT_BRACE_TOKEN: i32 = 0x200000;

/// translated from xetex-xetex0.c:4319 (scan_left_brace)
pub unsafe fn scan_left_brace() {
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
        set_help_line(
            3,
            c"A left brace was mandatory here, so I've put one in.".as_ptr(),
        );
        set_help_line(
            2,
            c"You might want to delete and/or insert some corrections".as_ptr(),
        );
        set_help_line(
            1,
            c"so that I will find a matching right brace soon.".as_ptr(),
        );
        set_help_line(
            0,
            c"(If you're confused by all this, try typing `I}' now.)".as_ptr(),
        );
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
        if cur_cmd() >= CALL
            && cur_cmd() < END_TEMPLATE
            && mem_b32s0(mem_b32s1(cur_chr())) == PROTECTED_TOKEN
        {
            return;
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
                let q = crate::mem::get_avail_global();
                set_mem_b32s1(p, q);
                set_mem_b32s0(q, cur_tok());
                p = q;
                crate::mem::flush_list_global(mem_b32s1(BACKUP_HEAD));
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
            let q = crate::mem::get_avail_global();
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
    crate::mem::flush_list_global(mem_b32s1(BACKUP_HEAD));
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
const NORMAL_EXPR: u16 = 0;
const TEX_INFINITY: i32 = 0x7FFF_FFFF;

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
                loop {
                    get_x_token();
                    if cur_cmd() != SPACER {
                        break;
                    }
                }
                if cur_tok() == OTHER_TOKEN + 40 {
                    // '('
                    let q = crate::mem::get_node_global(EXPR_NODE_SIZE);
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
                if o == INT_VAL {
                    scan_int();
                } else if o == DIMEN_VAL {
                    scan_dimen(false, false, false);
                } else if o == GLUE_VAL_EXPR {
                    scan_normal_glue();
                } else {
                    scan_mu_glue();
                }
                f = cur_val();
            }
            skip_scan = false;

            // found:
            loop {
                get_x_token();
                if cur_cmd() != SPACER {
                    break;
                }
            }
            let mut o: i16 = if cur_tok() == OTHER_TOKEN + 43 {
                EXPR_ADD
            } else if cur_tok() == OTHER_TOKEN + 45 {
                EXPR_SUB
            } else if cur_tok() == OTHER_TOKEN + 42 {
                EXPR_MULT
            } else if cur_tok() == OTHER_TOKEN + 47 {
                EXPR_DIV
            } else {
                EXPR_NONE
            };
            if o == EXPR_NONE {
                if p == crate::mem::TEX_NULL {
                    if cur_cmd() != RELAX {
                        back_input();
                    }
                } else if cur_tok() != OTHER_TOKEN + 41 {
                    // ')'
                    crate::output::error_here_with_diagnostic(
                        c"Missing ) inserted for expression".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(1);
                    set_help_line(
                        0,
                        c"I was expecting to see `+', `-', `*', `/', or `)'. Didn't.".as_ptr(),
                    );
                    crate::input::back_error();
                }
            }

            set_arith_error(b);
            if l == INT_VAL || s > EXPR_SUB {
                if !(-TEX_INFINITY..=TEX_INFINITY).contains(&f) {
                    set_arith_error(true);
                    f = 0;
                }
            } else if l == DIMEN_VAL {
                if f.abs() > MAX_HALFWORD {
                    set_arith_error(true);
                    f = 0;
                }
            } else if mem_b32s1(f + 1).abs() > MAX_HALFWORD
                || mem_b32s1(f + 2).abs() > MAX_HALFWORD
                || mem_b32s1(f + 3).abs() > MAX_HALFWORD
            {
                set_arith_error(true);
                crate::mem::delete_glue_ref_global(f);
                f = crate::mem::new_spec_global(0);
            }

            // o may be modified inside case 3 (EXPR_MULT → EXPR_SCALE)
            match s {
                0 => {
                    if l >= GLUE_VAL_EXPR && o != EXPR_NONE {
                        t = crate::mem::new_spec_global(f);
                        crate::mem::delete_glue_ref_global(f);
                        if mem_b32s1(t + 2) == 0 {
                            set_mem_b16s1(t, NORMAL_EXPR);
                        }
                        if mem_b32s1(t + 3) == 0 {
                            set_mem_b16s0(t, NORMAL_EXPR);
                        }
                    } else {
                        t = f;
                    }
                }
                3 => {
                    // EXPR_MULT
                    if o == EXPR_DIV {
                        n = f;
                        o = EXPR_SCALE; // modifies o for post-match code
                    } else if l == INT_VAL {
                        t = crate::scaled_math::mult_and_add(t, f, 0, TEX_INFINITY);
                    } else if l == DIMEN_VAL {
                        t = crate::scaled_math::mult_and_add(t, f, 0, MAX_HALFWORD);
                    } else {
                        set_mem_b32s1(
                            t + 1,
                            crate::scaled_math::mult_and_add(mem_b32s1(t + 1), f, 0, MAX_HALFWORD),
                        );
                        set_mem_b32s1(
                            t + 2,
                            crate::scaled_math::mult_and_add(mem_b32s1(t + 2), f, 0, MAX_HALFWORD),
                        );
                        set_mem_b32s1(
                            t + 3,
                            crate::scaled_math::mult_and_add(mem_b32s1(t + 3), f, 0, MAX_HALFWORD),
                        );
                    }
                }
                4 => {
                    // EXPR_DIV
                    if l < GLUE_VAL_EXPR {
                        t = crate::scaled_math::quotient(t, f);
                    } else {
                        set_mem_b32s1(t + 1, crate::scaled_math::quotient(mem_b32s1(t + 1), f));
                        set_mem_b32s1(t + 2, crate::scaled_math::quotient(mem_b32s1(t + 2), f));
                        set_mem_b32s1(t + 3, crate::scaled_math::quotient(mem_b32s1(t + 3), f));
                    }
                }
                5 => {
                    // EXPR_SCALE
                    if l == INT_VAL {
                        t = crate::scaled_math::fract(t, n, f, TEX_INFINITY);
                    } else if l == DIMEN_VAL {
                        t = crate::scaled_math::fract(t, n, f, MAX_HALFWORD);
                    } else {
                        set_mem_b32s1(
                            t + 1,
                            crate::scaled_math::fract(mem_b32s1(t + 1), n, f, MAX_HALFWORD),
                        );
                        set_mem_b32s1(
                            t + 2,
                            crate::scaled_math::fract(mem_b32s1(t + 2), n, f, MAX_HALFWORD),
                        );
                        set_mem_b32s1(
                            t + 3,
                            crate::scaled_math::fract(mem_b32s1(t + 3), n, f, MAX_HALFWORD),
                        );
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
                    set_mem_b32s1(
                        e + 1,
                        crate::scaled_math::add_or_sub(
                            mem_b32s1(e + 1),
                            mem_b32s1(t + 1),
                            MAX_HALFWORD,
                            r == EXPR_SUB,
                        ),
                    );
                    if mem_b16s1(e) == mem_b16s1(t) {
                        set_mem_b32s1(
                            e + 2,
                            crate::scaled_math::add_or_sub(
                                mem_b32s1(e + 2),
                                mem_b32s1(t + 2),
                                MAX_HALFWORD,
                                r == EXPR_SUB,
                            ),
                        );
                    } else if mem_b16s1(e) < mem_b16s1(t) && mem_b32s1(t + 2) != 0 {
                        set_mem_b32s1(e + 2, mem_b32s1(t + 2));
                        set_mem_b16s1(e, mem_b16s1(t));
                    }
                    if mem_b16s0(e) == mem_b16s0(t) {
                        set_mem_b32s1(
                            e + 3,
                            crate::scaled_math::add_or_sub(
                                mem_b32s1(e + 3),
                                mem_b32s1(t + 3),
                                MAX_HALFWORD,
                                r == EXPR_SUB,
                            ),
                        );
                    } else if mem_b16s0(e) < mem_b16s0(t) && mem_b32s1(t + 3) != 0 {
                        set_mem_b32s1(e + 3, mem_b32s1(t + 3));
                        set_mem_b16s0(e, mem_b16s0(t));
                    }
                    crate::mem::delete_glue_ref_global(t);
                    if mem_b32s1(e + 2) == 0 {
                        set_mem_b16s1(e, NORMAL_EXPR);
                    }
                    if mem_b32s1(e + 3) == 0 {
                        set_mem_b16s0(e, NORMAL_EXPR);
                    }
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
                crate::mem::free_node_global(q, EXPR_NODE_SIZE);
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
            crate::mem::delete_glue_ref_global(e);
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
    let new_def = crate::mem::get_avail_global();
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
        let q = crate::mem::get_avail_global();
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
    let s = if three_codes {
        save_stack_b32s1(save_ptr())
    } else {
        0
    };
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
pub unsafe extern "C" fn xetex_scan_dimen(
    mu: bool,
    inf: bool,
    shortcut: bool,
    requires_units: bool,
) {
    use crate::globals::{
        cur_order, font_info_b32s1, radix, set_cur_order, set_dig_entry, set_radix, tex_remainder,
    };
    const NORMAL_ORDER: u8 = 0;
    const FIL_ORDER: u8 = 1;
    const FILLL_ORDER: u8 = 3;
    const QUAD_CODE: i32 = 6; // xetex-constants.h:298
    const X_HEIGHT_CODE: i32 = 5; // xetex-constants.h:294
    const INT_PAR_MAG: i32 = 17; // xetex_format.h:52
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
                loop {
                    get_x_token();
                    if cur_cmd() != SPACER {
                        break;
                    }
                }
                if cur_tok() == MINUS_TOKEN {
                    negative = !negative;
                    set_cur_tok(PLUS_TOKEN);
                }
                if cur_tok() != PLUS_TOKEN {
                    break;
                }
            }

            if cur_cmd() >= MIN_INTERNAL && cur_cmd() <= MAX_INTERNAL {
                if mu {
                    scan_something_internal(MU_VAL, false);
                    if cur_val_level() as i16 >= GLUE_VAL {
                        let v = mem_b32s1(cur_val() + 1);
                        crate::mem::delete_glue_ref_global(cur_val());
                        set_cur_val(v);
                    }
                    if cur_val_level() as i16 == MU_VAL {
                        break 'body;
                    } // goto attach_sign
                    if cur_val_level() as i16 != INT_VAL {
                        mu_error();
                    }
                } else {
                    scan_something_internal(DIMEN_VAL, false);
                    if cur_val_level() as i16 == DIMEN_VAL {
                        break 'body;
                    } // goto attach_sign
                }
            } else {
                back_input();
                if cur_tok() == CONTINENTAL_POINT_TOKEN {
                    set_cur_tok(POINT_TOKEN);
                }
                if cur_tok() != POINT_TOKEN {
                    scan_int();
                } else {
                    set_radix(10);
                    set_cur_val(0);
                }
                if cur_tok() == CONTINENTAL_POINT_TOKEN {
                    set_cur_tok(POINT_TOKEN);
                }

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
                            let q = crate::mem::get_avail_global();
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
                    if cur_cmd() != SPACER {
                        back_input();
                    }
                }
            }
        }

        if cur_val() < 0 {
            negative = !negative;
            set_cur_val(-cur_val());
        }

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
                loop {
                    get_x_token();
                    if cur_cmd() != SPACER {
                        break;
                    }
                }

                if cur_cmd() >= MIN_INTERNAL && cur_cmd() <= MAX_INTERNAL {
                    if mu {
                        scan_something_internal(MU_VAL, false);
                        if cur_val_level() as i16 >= GLUE_VAL {
                            let v = mem_b32s1(cur_val() + 1);
                            crate::mem::delete_glue_ref_global(cur_val());
                            set_cur_val(v);
                        }
                        if cur_val_level() as i16 != MU_VAL {
                            mu_error();
                        }
                    } else {
                        scan_something_internal(DIMEN_VAL, false);
                    }
                    let v = cur_val();
                    set_cur_val(crate::scaled_math::mult_and_add(
                        save_cv,
                        v,
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
                    if cur_cmd() != SPACER {
                        back_input();
                    }
                    set_cur_val(crate::scaled_math::mult_and_add(
                        save_cv,
                        v,
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
                        set_help_line(
                            3,
                            c"The unit of measurement in math glue must be mu.".as_ptr(),
                        );
                        set_help_line(
                            2,
                            c"To recover gracefully from this error, it's best to".as_ptr(),
                        );
                        set_help_line(
                            1,
                            c"delete the erroneous units; e.g., type `2' to delete".as_ptr(),
                        );
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
                        f %= 65536;
                    }
                }

                if scan_keyword(c"pt".as_ptr()) {
                    break 'units (true, true);
                } // attach_fraction

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
                    error_here_with_diagnostic(c"Illegal unit of measure (pt inserted)".as_ptr());
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(6);
                    set_help_line(
                        5,
                        c"Dimensions can be in units of em, ex, in, pt, pc,".as_ptr(),
                    );
                    set_help_line(
                        4,
                        c"cm, mm, dd, cc, bp, or sp; but yours is a new one!".as_ptr(),
                    );
                    set_help_line(
                        3,
                        c"I'll assume that you meant to say pt, for printer's points.".as_ptr(),
                    );
                    set_help_line(
                        2,
                        c"To recover gracefully from this error, it's best to".as_ptr(),
                    );
                    set_help_line(
                        1,
                        c"delete the erroneous units; e.g., type `2' to delete".as_ptr(),
                    );
                    set_help_line(0, c"two letters. (See Chapter 27 of The TeXbook.)".as_ptr());
                    error();
                    break 'units (true, true); // done2 → attach_fraction
                };

                set_cur_val(crate::scaled_math::xn_over_d(cur_val(), num, denom));
                f = (num * f + 65536 * tex_remainder()) / denom;
                set_cur_val(cur_val() + f / 65536);
                f %= 65536;

                (true, true) // attach_fraction
            };

            if apply_fraction {
                if cur_val() >= 16384 {
                    set_arith_error(true);
                } else {
                    set_cur_val(cur_val() * 65536 + f);
                }
            }
            if read_spacer {
                get_x_token();
                if cur_cmd() != SPACER {
                    back_input();
                }
            }
        } else {
            // !requires_units
            if cur_val() >= 16384 {
                set_arith_error(true);
            } else {
                set_cur_val(cur_val() * 65536 + f);
            }
        }
    } // end 'body

    // attach_sign:
    if arith_error() || cur_val().abs() >= 0x4000_0000 {
        error_here_with_diagnostic(c"Dimension too large".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"I can't work with sizes bigger than about 19 feet.".as_ptr(),
        );
        set_help_line(
            0,
            c"Continue and I'll use the largest value I can.".as_ptr(),
        );
        error();
        set_cur_val(MAX_HALFWORD);
        set_arith_error(false);
    }
    if negative {
        set_cur_val(-cur_val());
    }
}

/// translated from xetex-xetex0.c:4302 (scan_glyph_number)
#[no_mangle]
pub unsafe extern "C" fn scan_glyph_number(f: i32) {
    if scan_keyword(c"/".as_ptr()) {
        scan_and_pack_name();
        set_cur_val(map_glyph_to_index_global(f));
        set_cur_val_level(INT_VAL as u8);
    } else if scan_keyword(c"u".as_ptr()) {
        scan_char_num();
        set_cur_val(map_char_to_glyph_global(f, cur_val()));
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
    let q = new_rule_global();
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
pub(crate) unsafe fn back_input() {
    crate::input::back_input();
}

// xetex-constants.h:204

// ---------------------------------------------------------------------------
// pseudo_start
// ---------------------------------------------------------------------------

// INT_BASE = 7_826_729 (xetex_format.h); INT_PAR__new_line_char = 49
const INT_PAR_NEW_LINE_CHAR: i32 = 49; // xetex_format.h:84
const INT_PAR_TRACING_SCAN_TOKENS: i32 = 59; // xetex_format.h:94
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
    crate::mem::flush_list_global(mem_b32s1(TEMP_HEAD));

    if pool_ptr() + 1 > pool_size() {
        crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr());
    }
    let s = crate::string_pool::make_string();
    set_str_pool(pool_ptr(), b' ' as u16);
    let mut l = str_start(s - TOO_BIG_CHAR);
    let nl = eqtb_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR);
    let p = crate::mem::get_avail_global();
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
        let r_base = crate::mem::get_node_global(sz);
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

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_SCAN_TOKENS) > 0 {
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
            if (0xD800..0xDC00).contains(&t) && k + 1 < pool_ptr() {
                let lo = str_pool_entry(k + 1) as i32;
                if (0xDC00..0xE000).contains(&lo) {
                    k += 1;
                    t = 0x10000 + ((t - 0xD800) << 10) + (str_pool_entry(k) as i32 - 0xDC00);
                }
            }

            if cat == 0 {
                t += OTHER_TOKEN;
            } else if cat == ACTIVE_CHAR as i16 {
                t += CS_TOKEN_FLAG + 1;
            } else {
                t += MAX_CHAR_VAL * cat as i32;
            }
        }

        let q = if avail() == TEX_NULL {
            crate::mem::get_avail_global()
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
            let p = crate::mem::get_avail_global();
            set_mem_b32s1(p, mem_b32s1(TEMP_HEAD));
            crate::output::token_show(p);
            crate::mem::flush_list_global(p);
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
            let q = crate::mem::get_avail_global();
            set_mem_b32s1(p, q);
            set_mem_b32s0(q, CS_TOKEN_FLAG + cur_val());
            p = q;
        } else if cur_val() != TEX_NULL {
            let mut r = mem_b32s1(cur_val());
            while r != TEX_NULL {
                let q = if avail() == TEX_NULL {
                    crate::mem::get_avail_global()
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
        p
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
                crate::mem::delete_glue_ref_global(cur_val());
            }
            3 => {
                crate::output::print_spec(cur_val(), c"mu".as_ptr());
                crate::mem::delete_glue_ref_global(cur_val());
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
    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_IFS) > 0 {
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
    let new_def = crate::mem::get_avail_global();
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
                            let q = crate::mem::get_avail_global();
                            set_mem_b32s1(p, q);
                            set_mem_b32s0(q, cur_tok());
                            p = q;
                            let q2 = crate::mem::get_avail_global();
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
                            set_help_line(0, c"as well as the token that followed it.".as_ptr());
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
                                set_help_line(0, c"Type `1' to delete what you did use.".as_ptr());
                                crate::input::back_error();
                            }
                            set_cur_tok(s);
                        }
                    }
                    // append cur_tok to list
                    let q = crate::mem::get_avail_global();
                    set_mem_b32s1(p, q);
                    set_mem_b32s0(q, cur_tok());
                    p = q;
                }
            };
            // done1: append END_MATCH_TOKEN
            if !reached_done {
                let q = crate::mem::get_avail_global();
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
                    set_help_line(0, c"which I'm going to interpret as `\\def\\a{}'.".as_ptr());
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
                    if cur_cmd() >= CALL && mem_b32s0(mem_b32s1(cur_chr())) == PROTECTED_TOKEN {
                        set_cur_cmd(RELAX);
                        set_cur_chr(NO_EXPAND_FLAG);
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
            } else if cur_cmd() == MAC_PARAM && macro_def {
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
                        set_help_line(2, c"You meant to type ## instead of #, right?".as_ptr());
                        set_help_line(
                            1,
                            c"Or maybe a } was forgotten somewhere earlier, and things".as_ptr(),
                        );
                        set_help_line(
                            0,
                            c"are all screwed up? I'm going to assume that you meant ##.".as_ptr(),
                        );
                        crate::input::back_error();
                        set_cur_tok(s);
                    } else {
                        set_cur_tok((OUT_PARAM_TOKEN - 48) + cur_chr());
                    }
                }
            }
            // append cur_tok to list
            let q = crate::mem::get_avail_global();
            set_mem_b32s1(p, q);
            set_mem_b32s0(q, cur_tok());
            p = q;
        }
    }
    // found:
    set_scanner_status(NORMAL_SCANNER);
    if hash_brace != 0 {
        let q = crate::mem::get_avail_global();
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
    let new_def = crate::mem::get_avail_global();
    set_def_ref(new_def);
    set_mem_b32s0(new_def, TEX_NULL);
    let mut p = new_def;

    let q = crate::mem::get_avail_global();
    set_mem_b32s1(p, q);
    set_mem_b32s0(q, END_MATCH_TOKEN);
    p = q;

    let m: usize = if !(0..=15).contains(&n) {
        16
    } else {
        n as usize
    };

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
            if input_line(read_file_entry(m)) != 0 {
                set_read_open_entry(m, NORMAL_SCANNER); // NORMAL = 0
            } else {
                u_close(read_file_entry(m));
                set_read_open_entry(m, CLOSED);
            }
        } else {
            // §505
            if input_line(read_file_entry(m)) == 0 {
                u_close(read_file_entry(m));
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
        let end_char = eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR);
        if !(0..=255).contains(&end_char) {
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
                let tok = if c == b' ' as i32 {
                    SPACE_TOKEN
                } else {
                    c + OTHER_TOKEN
                };
                set_cur_chr(c);
                set_cur_tok(tok);
                let q2 = crate::mem::get_avail_global();
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
                let q2 = crate::mem::get_avail_global();
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

// xetex-stringpool.h:12  (TOO_BIG_CHAR + 1)

// ---------------------------------------------------------------------------
// extra_right_brace
// ---------------------------------------------------------------------------

// xetex-constants.h:166,167,168

// ---------------------------------------------------------------------------
// scan_font_ident
// ---------------------------------------------------------------------------

const FONT_BASE: i32 = 0;
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
        if writing && (SPACE_CODE..=SPACE_SHRINK_CODE).contains(&n) && font_glue(f) != TEX_NULL {
            crate::mem::delete_glue_ref_global(font_glue(f));
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

// ---------------------------------------------------------------------------
// scan_something_internal
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:4505 (scan_something_internal)
#[no_mangle]
pub unsafe extern "C" fn scan_something_internal(level: i16, negative: bool) {
    use crate::globals::{
        char_base_entry, cur_chr, cur_cmd, cur_cs, cur_group, cur_if, cur_level,
        cur_list_aux_b32s0, cur_list_aux_b32s1, cur_list_head, cur_list_mode, cur_list_tail,
        cur_ptr, cur_val, cur_val1, cur_val_level, dead_cycles, depth_base_entry, eqtb_b32s1,
        font_area_entry, font_bc_entry, font_ec_entry, font_info_b16, font_info_b32s1,
        font_layout_engine_ptr, hash_s1, height_base_entry, hi_mem_min, hyphen_char_entry,
        if_limit, insert_penalties, interaction, italic_base_entry, last_badness, last_glue,
        last_kern, last_node_type, last_penalty, mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1,
        nest_mode, nest_prev_graf, nest_ptr, output_active, page_contents, page_so_far,
        pdf_last_x_pos, pdf_last_y_pos, random_seed, save_cur_list_to_nest, set_cur_chr,
        set_cur_cmd, set_cur_cs, set_cur_ptr, set_cur_tok, set_cur_val, set_cur_val1,
        set_cur_val_level, set_help_line, set_help_ptr, set_mem_b32s1, shell_escape_enabled,
        skew_char_entry, width_base_entry,
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
                    let fam = ((code as u32 >> 24) & 0xFF) as i32;
                    let ch = (code as u32 & 0x1FFFFF) as i32;
                    if ch == ACTIVE_MATH_CHAR {
                        set_cur_val1(0x8000);
                    } else if class > 7 || fam > 15 || ch > 255 {
                        let errmsg = error_here_with_diagnostic(
                            c"Extended mathchar used as mathchar".as_ptr(),
                        );
                        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val1());
                        capture_to_diagnostic(std::ptr::null_mut());
                        set_help_ptr(2);
                        set_help_line(
                            1,
                            c"A mathchar number must be between 0 and \"7FFF.".as_ptr(),
                        );
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
                        error_here_with_diagnostic(c"Extended delcode used as delcode".as_ptr());
                        capture_to_diagnostic(std::ptr::null_mut());
                        set_help_ptr(2);
                        set_help_line(
                            1,
                            c"I can only go up to 2147483647='17777777777=\"7FFFFFFF,".as_ptr(),
                        );
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
                        c"Can't use \\Umathcode as a number (try \\Umathcodenum)".as_ptr(),
                    );
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(2);
                    set_help_line(
                        1,
                        c"\\Umathcode is for setting a mathcode from separate values;".as_ptr(),
                    );
                    set_help_line(
                        0,
                        c"use \\Umathcodenum to access them as single values.".as_ptr(),
                    );
                    error();
                    set_cur_val(0);
                    set_cur_val_level(INT_VAL as u8);
                } else if m == DEL_CODE_BASE {
                    set_cur_val(eqtb_b32s1(DEL_CODE_BASE + cur_val()));
                    set_cur_val_level(INT_VAL as u8);
                } else {
                    error_here_with_diagnostic(
                        c"Can't use \\Udelcode as a number (try \\Udelcodenum)".as_ptr(),
                    );
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(2);
                    set_help_line(
                        1,
                        c"\\Udelcode is for setting a delcode from separate values;".as_ptr(),
                    );
                    set_help_line(
                        0,
                        c"use \\Udelcodenum to access them as single values.".as_ptr(),
                    );
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
                    set_help_line(
                        2,
                        c"A number should have been here; I inserted `0'.".as_ptr(),
                    );
                    set_help_line(
                        1,
                        c"(If you can't figure out why I needed to see a number,".as_ptr(),
                    );
                    set_help_line(
                        0,
                        c"look up `weird error' in the index to The TeXbook.)".as_ptr(),
                    );
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
                                crate::save::find_sa_element_global(
                                    TOK_VAL as u16,
                                    cur_val(),
                                    false,
                                );
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
                        crate::save::find_sa_element_global(
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
                    set_help_line(
                        3,
                        c"You can refer to \\spacefactor only in horizontal mode;".as_ptr(),
                    );
                    set_help_line(
                        2,
                        c"you can refer to \\prevdepth only in vertical mode; and".as_ptr(),
                    );
                    set_help_line(
                        1,
                        c"neither of these is meaningful inside \\write. So".as_ptr(),
                    );
                    set_help_line(
                        0,
                        c"I'm forgetting what you said and using zero instead.".as_ptr(),
                    );
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
                    crate::save::find_sa_element_global(4, cur_val(), false);
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
                if !(0..=19).contains(&m) {
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
                        crate::save::find_sa_element_global(
                            cur_val_level() as u16,
                            cur_val(),
                            false,
                        );
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
                                crate::mem::delete_glue_ref_global(gm);
                            } else if (cur_val_level() as i16) == MU_VAL {
                                mu_error();
                            }
                            set_cur_val_level(cur_val_level() - 1);
                        }
                        if negative {
                            if (cur_val_level() as i16) >= GLUE_VAL {
                                let gm = cur_val();
                                set_cur_val(new_spec_global(gm));
                                crate::mem::delete_glue_ref_global(gm);
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
                                    if !(1..=4).contains(&n) {
                                        error_here_with_diagnostic(
                                            c"\\\\XeTeXglyphbounds requires an edge index from 1 to 4;".as_ptr());
                                        crate::output::print_nl_cstr(
                                            c"I don't know anything about edge ".as_ptr(),
                                        );
                                        crate::output::print_int(n);
                                        capture_to_diagnostic(std::ptr::null_mut());
                                        error();
                                        set_cur_val(0);
                                    } else {
                                        scan_int();
                                        set_cur_val(get_glyph_bounds_global(cf, n, cur_val()));
                                    }
                                } else {
                                    not_native_font_error(LAST_ITEM as i32, m, cf);
                                    set_cur_val(0);
                                }
                            }
                            FONT_CHAR_WD_CODE | FONT_CHAR_HT_CODE | FONT_CHAR_DP_CODE
                            | FONT_CHAR_IC_CODE => {
                                crate::scan::scan_font_ident();
                                q = cur_val();
                                scan_usv_num();
                                let fa = font_area_entry(q);
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    set_cur_val(match m {
                                        FONT_CHAR_WD_CODE => getnativecharwd_global(q, cur_val()),
                                        FONT_CHAR_HT_CODE => getnativecharht_global(q, cur_val()),
                                        FONT_CHAR_DP_CODE => getnativechardp_global(q, cur_val()),
                                        _ => getnativecharic_global(q, cur_val()),
                                    });
                                } else {
                                    let bc = font_bc_entry(q) as i32;
                                    let ec = font_ec_entry(q) as i32;
                                    if bc <= cur_val() && ec >= cur_val() {
                                        let i = font_info_b16(
                                            char_base_entry(q)
                                                + effective_char_global(true, q, cur_val() as u16),
                                        );
                                        set_cur_val(match m {
                                            FONT_CHAR_WD_CODE => {
                                                font_info_b32s1(width_base_entry(q) + i.s3 as i32)
                                            }
                                            FONT_CHAR_HT_CODE => font_info_b32s1(
                                                height_base_entry(q) + i.s2 as i32 / 16,
                                            ),
                                            FONT_CHAR_DP_CODE => font_info_b32s1(
                                                depth_base_entry(q) + i.s2 as i32 % 16,
                                            ),
                                            _ => font_info_b32s1(
                                                italic_base_entry(q) + i.s1 as i32 / 4,
                                            ),
                                        });
                                    } else {
                                        set_cur_val(0);
                                    }
                                }
                            }
                            PAR_SHAPE_LENGTH_CODE
                            | PAR_SHAPE_INDENT_CODE
                            | PAR_SHAPE_DIMEN_CODE => {
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
                                    if cur_val() > cnt {
                                        set_cur_val(cnt);
                                    }
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
                                crate::mem::delete_glue_ref_global(q);
                            }
                            _ => {}
                        }
                        set_cur_val_level(DIMEN_VAL as u8);
                    } else {
                        // m >= INPUT_LINE_NO_CODE && m < XETEX_DIM
                        match m {
                            INPUT_LINE_NO_CODE => {
                                set_cur_val(crate::globals::line());
                            }
                            BADNESS_CODE => {
                                set_cur_val(last_badness());
                            }
                            ELAPSED_TIME_CODE => {
                                set_cur_val(crate::texmfmp::get_microinterval());
                            }
                            RANDOM_SEED_CODE => {
                                set_cur_val(random_seed());
                            }
                            PDF_SHELL_ESCAPE_CODE => {
                                set_cur_val(if shell_escape_enabled() { 1 } else { 0 });
                            }
                            ETEX_VERSION_CODE => {
                                set_cur_val(ETEX_VERSION);
                            }
                            XETEX_VERSION_CODE => {
                                set_cur_val(XETEX_VERSION);
                            }
                            XETEX_COUNT_GLYPHS_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let engine = font_layout_engine_ptr(n);
                                let fa = font_area_entry(n);
                                set_cur_val(if fa == AAT_FONT_FLAG {
                                    aat_font_get(m - XETEX_INT, engine)
                                } else if fa == OTGR_FONT_FLAG {
                                    ot_font_get(m - XETEX_INT, engine)
                                } else {
                                    0
                                });
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
                                } else {
                                    0
                                });
                            }
                            XETEX_VARIATION_CODE
                            | XETEX_VARIATION_MIN_CODE
                            | XETEX_VARIATION_MAX_CODE
                            | XETEX_VARIATION_DEFAULT_CODE
                            | XETEX_COUNT_VARIATIONS_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                set_cur_val(0);
                            }
                            XETEX_FEATURE_CODE_CODE
                            | XETEX_IS_EXCLUSIVE_FEATURE_CODE
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
                                    scan_int();
                                    k = cur_val();
                                    scan_int();
                                    set_cur_val(aat_font_get_2(
                                        m - XETEX_INT,
                                        engine,
                                        k,
                                        cur_val(),
                                    ));
                                } else if fa == OTGR_FONT_FLAG && usingGraphite(engine) {
                                    scan_int();
                                    k = cur_val();
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
                                    scan_int();
                                    k = cur_val();
                                    scan_and_pack_name();
                                    set_cur_val(aat_font_get_named_1(m - XETEX_INT, engine, k));
                                } else if fa == OTGR_FONT_FLAG && usingGraphite(engine) {
                                    scan_int();
                                    k = cur_val();
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
                                } else {
                                    0
                                });
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
                                    scan_int();
                                    k = cur_val();
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
                                    scan_int();
                                    k = cur_val();
                                    scan_int();
                                    kk = cur_val();
                                    scan_int();
                                    set_cur_val(ot_font_get_3(
                                        m - XETEX_INT,
                                        engine,
                                        k,
                                        kk,
                                        cur_val(),
                                    ));
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
                                    set_cur_val(map_char_to_glyph_global(cf, n));
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
                                    set_cur_val(map_glyph_to_index_global(cf));
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
                                set_cur_val(if fa == AAT_FONT_FLAG {
                                    1
                                } else if fa == OTGR_FONT_FLAG && usingOpenType(engine) {
                                    2
                                } else if fa == OTGR_FONT_FLAG && usingGraphite(engine) {
                                    3
                                } else {
                                    0
                                });
                            }
                            XETEX_FIRST_CHAR_CODE | XETEX_LAST_CHAR_CODE => {
                                crate::scan::scan_font_ident();
                                n = cur_val();
                                let fa = font_area_entry(n);
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    set_cur_val(get_font_char_range_global(
                                        n,
                                        (m == XETEX_FIRST_CHAR_CODE) as i32,
                                    ));
                                } else if m == XETEX_FIRST_CHAR_CODE {
                                    set_cur_val(font_bc_entry(n) as i32);
                                } else {
                                    set_cur_val(font_ec_entry(n) as i32);
                                }
                            }
                            PDF_LAST_X_POS_CODE => {
                                set_cur_val(pdf_last_x_pos());
                            }
                            PDF_LAST_Y_POS_CODE => {
                                set_cur_val(pdf_last_y_pos());
                            }
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
                                crate::mem::delete_glue_ref_global(q);
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
                                if r == tx {
                                    break;
                                }
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
                set_help_line(
                    0,
                    c"I'm forgetting what you said and using zero instead.".as_ptr(),
                );
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
            set_cur_val(new_spec_global(gm));
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
        cur_cmd, cur_cs, cur_tok, cur_val, eqtb_b32s1, hash_s1, set_cur_chr, set_cur_cmd,
        set_cur_cs, set_cur_tok, set_cur_val, set_help_line, set_help_ptr, set_radix,
    };

    let mut negative = false;
    let mut m: i32 = 0;
    let mut d: i32 = 0;
    let mut vacuous = false;
    let mut ok_so_far = true;

    set_radix(0);

    // Skip spaces and absorb sign prefix(es).
    loop {
        loop {
            get_x_token();
            if cur_cmd() != SPACER {
                break;
            }
        }
        if cur_tok() == OTHER_TOKEN + b'-' as i32 {
            negative = !negative;
            set_cur_tok(OTHER_TOKEN + b'+' as i32);
        }
        if cur_tok() != OTHER_TOKEN + b'+' as i32 {
            break;
        }
    }

    'restart: loop {
        if cur_tok() == ALPHA_TOKEN {
            get_token();
            if cur_tok() < CS_TOKEN_FLAG {
                set_cur_val(cur_chr());
                if cur_cmd() <= 2 {
                    // RIGHT_BRACE = 2 (xetex_format.h:232)
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
                set_help_line(
                    1,
                    c"A one-character control sequence belongs after a ` mark.".as_ptr(),
                );
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
                        set_help_line(
                            1,
                            c"I can only go up to 2147483647='17777777777=\"7FFFFFFF,".as_ptr(),
                        );
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
                set_help_line(
                    2,
                    c"A number should have been here; I inserted `0'.".as_ptr(),
                );
                set_help_line(
                    1,
                    c"(If you can't figure out why I needed to see a number,".as_ptr(),
                );
                set_help_line(
                    0,
                    c"look up `weird error' in the index to The TeXbook.)".as_ptr(),
                );
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
        set_help_line(
            0,
            c"Sorry, this optional e-TeX feature has been disabled.".as_ptr(),
        );
        error();
    }
    b
}

// ---------------------------------------------------------------------------
// effective_char / effective_char_info
// ---------------------------------------------------------------------------

/// Transitional bridge-down wrapper for legacy `effective_char` callers.
pub unsafe fn effective_char_global(err_p: bool, f: i32, c: u16) -> i32 {
    crate::state::with_state_mut(|st| effective_char(&mut st.fonts, err_p, f, c))
}

/// Transitional bridge-down wrapper for legacy `effective_char_info` callers.
pub unsafe fn effective_char_info_global(f: i32, c: u16) -> crate::mem_types::B16x4 {
    crate::state::with_state_mut(|st| effective_char_info(&mut st.fonts, f, c))
}

// ---------------------------------------------------------------------------
// you_cant / report_illegal_case / privileged
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// align_error / no_align_error / omit_error / cs_error
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// push_math
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// append_glue / append_kern
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// make_mark
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// new_interaction
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// alter_aux / alter_prev_graf / alter_page_so_far / alter_integer / alter_box_dimen
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// trap_zero_glue
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// fix_language
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// off_save
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// tokens_to_string
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:13060 (tokens_to_string)
#[no_mangle]
pub unsafe extern "C" fn tokens_to_string(p: i32) -> i32 {
    if selector() == SELECTOR_NEW_STRING {
        crate::errors::pdf_error(
            c"tokens".as_ptr(),
            c"tokens_to_string() called while selector = new_string".as_ptr(),
        );
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
    crate::mem::delete_token_ref_global(def_ref());
    set_cur_cs(save_cur_cs);
    scan_toks(false, true);
    let s2 = tokens_to_string(def_ref());
    crate::mem::delete_token_ref_global(def_ref());

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
// do_endv
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// get_r_token
// ---------------------------------------------------------------------------

const FROZEN_CONTROL_SEQUENCE: i32 = 2_243_226; // xetex_format.h:201
const FROZEN_PROTECTION: i32 = FROZEN_CONTROL_SEQUENCE; // xetex-constants.h:55

// translated from xetex-xetex0.c:10407 (get_r_token)
#[no_mangle]
pub unsafe extern "C" fn get_r_token() {
    loop {
        loop {
            get_token();
            if cur_tok() != SPACE_TOKEN {
                break;
            }
        }
        if cur_cs() == 0
            || cur_cs() > crate::globals::eqtb_top()
            || (cur_cs() > FROZEN_CONTROL_SEQUENCE && cur_cs() <= EQTB_SIZE)
        {
            error_here_with_diagnostic(c"Missing control sequence inserted".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(5);
            set_help_line(
                4,
                c"Please don't say `\\def cs{...}', say `\\def\\cs{...}'.".as_ptr(),
            );
            set_help_line(
                3,
                c"I've inserted an inaccessible control sequence so that your".as_ptr(),
            );
            set_help_line(
                2,
                c"definition will be completed without mixing me up too badly.".as_ptr(),
            );
            set_help_line(
                1,
                c"You can recover graciously from this error, if you're".as_ptr(),
            );
            set_help_line(0, c"careful; see exercise 27.2 in The TeXbook.".as_ptr());
            if cur_cs() == 0 {
                crate::input::back_input();
            }
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

// ---------------------------------------------------------------------------
// shift_case
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// build_discretionary
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// append_to_vlist
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// fin_align
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// align_peek
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// append_italic_correction
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// make_accent
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// new_font
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// do_register_command
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// unpackage
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// delete_last
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:4313 (scan_math)
#[no_mangle]
pub unsafe extern "C" fn scan_math(p: i32) {
    use crate::globals::{
        cur_chr, cur_cmd, cur_cs, eqtb_b16s1, set_cur_chr, set_cur_cmd, set_cur_cs,
    };
    const LETTER: u8 = 11; // xetex_format.h:243
    const OTHER_CHAR_CMD: u8 = 12; // xetex_format.h:244
    const DELIM_NUM: u8 = 15; // xetex_format.h:251
    const CHAR_NUM_CMD: u8 = 16; // xetex_format.h:253
    const MATH_CHAR_NUM: u8 = 17; // xetex_format.h:255
    const CHAR_GIVEN_CMD: u8 = 68; // xetex_format.h:306
    const MATH_GIVEN_CMD: u8 = 69; // xetex_format.h:308
    const XETEX_MATH_GIVEN: u8 = 70; // xetex_format.h:309
    const ACTIVE_MATH_CHAR: i32 = 0x1FFFFF; // xetex-constants.h:343
    const MATH_CHAR_NODE: i32 = 1; // xetex-constants.h:269
    const MATH_GROUP_TYPE: u8 = 9; // xetex-constants.h:161
    const INT_PAR_CUR_FAM: i32 = 44; // xetex_format.h:79
    const NUMBER_MATH_FAMILIES_VAL: i32 = 256; // xetex-constants.h:31

    let c: i32;
    'main: loop {
        loop {
            get_x_token();
            if cur_cmd() != SPACER && cur_cmd() != RELAX {
                break;
            }
        }
        c = loop {
            break match cur_cmd() {
                LETTER | OTHER_CHAR_CMD | CHAR_GIVEN_CMD => {
                    let mc = eqtb_b32s1(MATH_CODE_BASE + cur_chr());
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
    let fam: u16 = if math_class_fn(c) == 7 && (0..NUMBER_MATH_FAMILIES_VAL).contains(&cur_fam_val)
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
    use crate::globals::{
        cur_chr, cur_cs, set_cur_chr, set_cur_cmd, set_cur_cs, set_mem_b16s0, set_mem_b16s1,
        set_mem_b32s1,
    };
    const ACTIVE_MATH_CHAR_SM: i32 = 0x1FFFFF;
    const MATH_CHAR_NODE_SM: i32 = 1; // xetex-constants.h:269
    const ORD_NOAD: u16 = 16; // xetex_format.h:401
    const INT_PAR_CUR_FAM_SM: i32 = 44;
    const NUMBER_MATH_FAM_SM: i32 = 256;

    if math_char_fn(c) == ACTIVE_MATH_CHAR_SM {
        set_cur_cs(cur_chr() + 1);
        set_cur_cmd(crate::globals::eqtb_b16s1(cur_cs()) as u8);
        set_cur_chr(eqtb_b32s1(cur_cs()));
        x_token();
        crate::input::back_input();
    } else {
        let p = crate::mem::new_noad_global();
        set_mem_b32s1(p + 1, MATH_CHAR_NODE_SM);
        let ch = math_char_fn(c);
        set_mem_b16s0(p + 1, (ch % 65536) as u16);
        set_mem_b16s1(p + 1, math_fam_fn(c) as u16);
        if math_class_fn(c) == 7 {
            let cf = eqtb_b32s1(INT_BASE + INT_PAR_CUR_FAM_SM);
            if (0..NUMBER_MATH_FAM_SM).contains(&cf) {
                set_mem_b16s1(p + 1, cf as u16);
            }
            crate::globals::set_mem_b16s1(p, ORD_NOAD);
        } else {
            crate::globals::set_mem_b16s1(p, ORD_NOAD + math_class_fn(c) as u16);
        }
        set_mem_b16s1(
            p + 1,
            crate::globals::mem_b16s1(p + 1) + (ch / 65536) as u16 * 256,
        );
        set_mem_b32s1(crate::globals::cur_list_tail(), p);
        crate::globals::set_cur_list_tail(p);
    }
}

#[inline]
fn math_char_fn(c: i32) -> i32 {
    (c as u32 & 0x1FFFFF) as i32
}
#[inline]
fn math_class_fn(c: i32) -> i32 {
    ((c as u32 >> 21) & 0x07) as i32
}
#[inline]
fn math_fam_fn(c: i32) -> i32 {
    ((c as u32 >> 24) & 0xFF) as i32
}
#[inline]
pub(crate) fn set_class_fn(x: i32) -> i32 {
    (((x as u32) & 0x07) << 21) as i32
}
#[inline]
pub(crate) fn set_family_fn(x: i32) -> i32 {
    (((x as u32) & 0xFF) << 24) as i32
}

// translated from xetex-xetex0.c:4678 (scan_glue)
#[no_mangle]
pub unsafe extern "C" fn scan_glue(level: i16) {
    use crate::globals::{
        cur_order, cur_val_level, set_cur_val, set_mem_b16s0, set_mem_b16s1, set_mem_b32s1,
    };
    const MINUS_TOKEN: i32 = OTHER_TOKEN + b'-' as i32;
    const PLUS_TOKEN: i32 = OTHER_TOKEN + b'+' as i32;

    let mu = level == MU_VAL;
    let mut negative = false;

    loop {
        loop {
            get_x_token();
            if cur_cmd() != SPACER {
                break;
            }
        }
        if cur_tok() == MINUS_TOKEN {
            negative = !negative;
            set_cur_tok(PLUS_TOKEN);
        }
        if cur_tok() != PLUS_TOKEN {
            break;
        }
    }

    if cur_cmd() >= MIN_INTERNAL && cur_cmd() <= MAX_INTERNAL {
        scan_something_internal(level, negative);
        if cur_val_level() as i16 >= GLUE_VAL {
            if cur_val_level() as i16 != level {
                mu_error();
            }
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
        if negative {
            set_cur_val(-cur_val());
        }
    }

    let q = new_spec_global(0);
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

/// Cold abort reasons threaded out of the node-recycling functions
/// (`copy_node_list`, `prune_page_top`) so the `_global` wrappers can run the
/// `runaway`/`overflow`/`confusion` longjmp *after* the `with_state` borrow
/// drops (`01b` §5). Never observed on a passing run.
pub(crate) enum NodeRecycleAbort {
    /// `get_node` / `new_skip_param` exhausted the arena: `overflow`.
    Overflow,
    /// `get_avail` exhausted the arena: `runaway()` then `overflow`.
    OverflowRunaway,
    /// Internal-consistency failure: `confusion(msg)`.
    Confusion(&'static std::ffi::CStr),
}

/// Run the deferred cold abort for a [`NodeRecycleAbort`] after the state
/// borrow has dropped. Never returns (each arm longjmps).
pub(crate) unsafe fn node_recycle_abort(e: NodeRecycleAbort) -> ! {
    const MEM_TOP_NR: i32 = 4_999_999;
    match e {
        NodeRecycleAbort::Overflow => {
            crate::errors::overflow(c"main memory size".as_ptr(), MEM_TOP_NR + 1)
        }
        NodeRecycleAbort::OverflowRunaway => {
            runaway();
            crate::errors::overflow(c"main memory size".as_ptr(), MEM_TOP_NR + 1)
        }
        NodeRecycleAbort::Confusion(msg) => crate::errors::confusion(msg.as_ptr()),
    }
}

/// `get_node` under the `mem` borrow, returning [`NodeRecycleAbort::Overflow`]
/// from the enclosing function on arena exhaustion (`01b` §5 sentinel).
macro_rules! cnl_get_node {
    ($mem:expr, $sz:expr) => {{
        let __r = crate::mem::get_node($mem, $sz);
        if __r == crate::mem::ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        __r
    }};
}

/// `get_avail` under the `mem` borrow, returning
/// [`NodeRecycleAbort::OverflowRunaway`] from the enclosing function on
/// exhaustion (`get_avail_global` runs `runaway()` before `overflow`).
macro_rules! cnl_get_avail {
    ($mem:expr) => {{
        let __r = crate::mem::get_avail($mem);
        if __r == crate::mem::ALLOC_FAILED {
            return Err(NodeRecycleAbort::OverflowRunaway);
        }
        __r
    }};
}

// Re-export cnl_get_node for the relocated font-loading group in fonts.rs (ws02)
// and copy_node_list in packaging.rs (ws02).
pub(crate) use cnl_get_node;
// Re-export cnl_get_avail for copy_node_list in packaging.rs (ws02).
pub(crate) use cnl_get_avail;

// ---------------------------------------------------------------------------
// check_outer_validity
// ---------------------------------------------------------------------------

// Scanner-status discriminants (xetex-constants.h:182–187).
const NORMAL_COV: u8 = 0;
const SKIPPING_COV: u8 = 1;
const DEFINING_COV: u8 = 2;
const MATCHING_COV: u8 = 3;
const ALIGNING_COV: u8 = 4;
const ABSORBING_COV: u8 = 5;
// Token constants (xetex-constants.h:351; xetex_format.h:357; xetex-constants.h:59).
const RIGHT_BRACE_TOKEN_COV: i32 = 0x40_0000; // RIGHT_BRACE << 21
const OUTER_CALL_COV: u8 = 115; // xetex_format.h:357
const FROZEN_CR_COV: i32 = 2_243_226 + 1; // FROZEN_CONTROL_SEQUENCE + 1
const FROZEN_FI_COV: i32 = 2_243_226 + 4; // FROZEN_CONTROL_SEQUENCE + 4
const IF_TEST_COV: u16 = 107; // xetex_format.h:349
                              // Input-state discriminant (already TOKEN_LIST = 0 in input.rs).
const TOKEN_LIST_COV: u16 = 0;

/// translated from xetex-xetex0.c:3036 (check_outer_validity)
#[no_mangle]
pub unsafe extern "C" fn check_outer_validity() {
    use crate::globals::{
        cur_cs, cur_if, cur_input, par_token, scanner_status, set_align_state, set_cur_chr,
        set_cur_cmd, set_cur_cs, set_cur_tok, set_deletions_allowed, set_help_line, set_help_ptr,
        set_long_state, set_mem_b32s0, set_mem_b32s1, skip_line, warning_index,
    };
    if scanner_status() == NORMAL_COV {
        return;
    }
    set_deletions_allowed(false);

    // If cur_cs is a control sequence, push it back as a backed-up token.
    if cur_cs() != 0 {
        let ci = cur_input();
        if ci.state == TOKEN_LIST_COV || ci.name < 1 || ci.name > 17 {
            let p = crate::mem::get_avail_global();
            set_mem_b32s0(p, CS_TOKEN_FLAG + cur_cs());
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

        let p = crate::mem::get_avail_global();
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
                let p2 = crate::mem::get_avail_global();
                set_mem_b32s1(p2, q);
                set_mem_b32s0(p2, CS_TOKEN_FLAG + FROZEN_CR_COV);
                set_align_state(-1_000_000);
                crate::input::begin_token_list(p2, 5 /* INSERTED */);
                // For ALIGNING, we push p2 as the token list, not p.
                // Then fall through to the help message.
                crate::output::print_cstr(c" of ".as_ptr());
                crate::output::sprint_cs(warning_index());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(4);
                set_help_line(
                    3,
                    c"I suspect you have forgotten a `}', causing me".as_ptr(),
                );
                set_help_line(2, c"to read past where you wanted me to stop.".as_ptr());
                set_help_line(
                    1,
                    c"I'll try to recover; but if the error is serious,".as_ptr(),
                );
                set_help_line(
                    0,
                    c"you'd better type `E' or `X' now and fix your file.".as_ptr(),
                );
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
        set_help_line(
            3,
            c"I suspect you have forgotten a `}', causing me".as_ptr(),
        );
        set_help_line(2, c"to read past where you wanted me to stop.".as_ptr());
        set_help_line(
            1,
            c"I'll try to recover; but if the error is serious,".as_ptr(),
        );
        set_help_line(
            0,
            c"you'd better type `E' or `X' now and fix your file.".as_ptr(),
        );
        crate::errors::error();
    } else {
        // scanner_status == SKIPPING: inside \if...\fi skip.
        error_here_with_diagnostic(c"Incomplete ".as_ptr());
        print_cmd_chr(IF_TEST_COV, cur_if() as i32);
        crate::output::print_cstr(c"; all text was ignored after line ".as_ptr());
        crate::output::print_int(skip_line());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(3);
        set_help_line(
            2,
            c"A forbidden control sequence occurred in skipped text.".as_ptr(),
        );
        set_help_line(
            1,
            c"This kind of error happens when you say `\\if...' and forget".as_ptr(),
        );
        set_help_line(
            0,
            c"the matching `\\fi'. I've inserted a `\\fi'; this might work.".as_ptr(),
        );
        if cur_cs() != 0 {
            set_cur_cs(0);
        } else {
            set_help_line(
                2,
                c"The file ended while I was skipping conditional text.".as_ptr(),
            );
        }
        set_cur_tok(CS_TOKEN_FLAG + FROZEN_FI_COV);
        crate::input::ins_error();
    }
    set_deletions_allowed(true);
}

// ---------------------------------------------------------------------------
// expand
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:3167 (get_next)
#[no_mangle]
pub unsafe extern "C" fn get_next() {
    use crate::globals::{
        cond_ptr, cur_align, cur_boundary, eof_seen, eqtb_b16s1, force_eof, grp_stack, if_stack,
        in_open, input_file, input_ptr, line, par_loc, param_stack, set_deletions_allowed,
        set_eof_seen, set_force_eof, set_line, set_used_tectonic_coda_tokens,
        used_tectonic_coda_tokens,
    };

    // State codes (xetex-constants.h)
    const TOKEN_LIST: u16 = 0;
    const MID_LINE: u16 = 1;
    const SKIP_BLANKS: u16 = 17;
    const NEW_LINE: u16 = 33;
    // Category codes (xetex_format.h)
    const LEFT_BRACE_CAT: u8 = 1;
    const RIGHT_BRACE_CAT: u8 = 2;
    const TAB_MARK_CAT: u8 = 4;
    const CAR_RET: u8 = 5;
    const SUP_MARK: u8 = 7;
    const SPACER: u8 = 10;
    const LETTER: u8 = 11;
    // Other constants
    const NULL_CS_GN: i32 = 2_228_225; // xetex_format.h:199
    const BIGGEST_USV_GN: i32 = 0x10_FFFF;
    const OUTER_CALL_GN: u8 = 115;
    const DONT_EXPAND_GN: u8 = 118;
    const MAX_COMMAND_GN: u8 = 102;
    const RELAX_GN: u8 = 0;
    const NO_EXPAND_FLAG_GN: i32 = BIGGEST_USV_GN + 2;
    const MAX_CHAR_VAL_GN: i32 = 0x200000;
    const OUT_PARAM_GN: u8 = 5;
    const PARAMETER_GN: u16 = 0;
    const V_TEMPLATE_GN: u16 = 2;
    const OMIT_GN: u8 = 63;
    const OMIT_TEMPLATE_GN: i32 = 4_999_999 - 10; // MEM_TOP - 10
    const ALIGNING_GN: u8 = 4;
    const LOCAL_EVERY_EOF: i32 = 10;
    const LOCAL_TECTONIC_CODA: i32 = 12;
    const EVERY_EOF_TEXT_GN: u16 = 16;
    const TECTONIC_CODA_TEXT_GN: u16 = 19;
    const SELECTOR_LOG_ONLY_GN: i32 = 18;
    const INT_PAR_TRACING_NESTING: i32 = 60;
    const INT_PAR_END_LINE_CHAR_GN: i32 = 48;

    #[inline(always)]
    fn is_lc_hex(c: i32) -> bool {
        (0x30..=0x39).contains(&c) || (0x61..=0x66).contains(&c)
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
                    if cur_chr() >= 0xD800
                        && cur_chr() < 0xDC00
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
                            10 | 26 | 42 | 27 | 43 => {
                                continue 'texswitch;
                            }

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
                                            if first_cat == LETTER || first_cat == SPACER {
                                                SKIP_BLANKS
                                            } else {
                                                MID_LINE
                                            },
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
                                            if cat2 == SUP_MARK
                                                && buffer_entry(k) == chr
                                                && k < cur_input().limit
                                            {
                                                let mut sup_count: i32 = 2;
                                                while sup_count < 6
                                                    && k + 2 * sup_count - 2 <= cur_input().limit
                                                    && buffer_entry(k + sup_count - 1) == chr
                                                {
                                                    sup_count += 1;
                                                }
                                                let sup_count_save = sup_count;
                                                let mut hex_ok = true;
                                                for d in 1..=sup_count_save {
                                                    if !is_lc_hex(buffer_entry(
                                                        k + sup_count - 2 + d,
                                                    )) {
                                                        let c = buffer_entry(k + 1);
                                                        if c < 128 {
                                                            let new_val = if c < 64 {
                                                                c + 64
                                                            } else {
                                                                c - 64
                                                            };
                                                            set_buffer_entry(k - 1, new_val);
                                                            set_cur_input_limit(
                                                                cur_input().limit - 2,
                                                            );
                                                            let mut kk = k;
                                                            while kk <= cur_input().limit {
                                                                set_buffer_entry(
                                                                    kk,
                                                                    buffer_entry(kk + 2),
                                                                );
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
                                                            + if c <= b'9' as i32 {
                                                                c - b'0' as i32
                                                            } else {
                                                                c - b'a' as i32 + 10
                                                            };
                                                    }
                                                    if new_chr <= BIGGEST_USV_GN {
                                                        set_buffer_entry(k - 1, new_chr);
                                                        let shift = 2 * sup_count - 1;
                                                        set_cur_input_limit(
                                                            cur_input().limit - shift,
                                                        );
                                                        let mut kk = k;
                                                        while kk <= cur_input().limit {
                                                            set_buffer_entry(
                                                                kk,
                                                                buffer_entry(kk + shift),
                                                            );
                                                            kk += 1;
                                                        }
                                                        continue 'start_cs;
                                                    }
                                                    // else: new_chr > BIGGEST_USV — fall through
                                                }
                                            }

                                            if cat2 != LETTER {
                                                k -= 1;
                                            }
                                            if k > cur_input().loc + 1 {
                                                set_cur_cs(crate::hash::id_lookup(
                                                    cur_input().loc,
                                                    k - cur_input().loc,
                                                ));
                                                set_cur_input_loc(k);
                                                break 'start_cs; // goto found
                                            }
                                            // Single letter: fall through to single-char handling
                                        } else {
                                            // Single-char (non-LETTER first char) or end of line
                                            if first_cat == SUP_MARK
                                                && buffer_entry(k) == first_chr
                                                && k < cur_input().limit
                                            {
                                                let mut sup_count: i32 = 2;
                                                while sup_count < 6
                                                    && k + 2 * sup_count - 2 <= cur_input().limit
                                                    && buffer_entry(k + sup_count - 1) == first_chr
                                                {
                                                    sup_count += 1;
                                                }
                                                let sup_count_save = sup_count;
                                                let mut hex_ok = true;
                                                for d in 1..=sup_count_save {
                                                    if !is_lc_hex(buffer_entry(
                                                        k + sup_count - 2 + d,
                                                    )) {
                                                        let c = buffer_entry(k + 1);
                                                        if c < 128 {
                                                            let new_val = if c < 64 {
                                                                c + 64
                                                            } else {
                                                                c - 64
                                                            };
                                                            set_buffer_entry(k - 1, new_val);
                                                            set_cur_input_limit(
                                                                cur_input().limit - 2,
                                                            );
                                                            let mut kk = k;
                                                            while kk <= cur_input().limit {
                                                                set_buffer_entry(
                                                                    kk,
                                                                    buffer_entry(kk + 2),
                                                                );
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
                                                            + if c <= b'9' as i32 {
                                                                c - b'0' as i32
                                                            } else {
                                                                c - b'a' as i32 + 10
                                                            };
                                                    }
                                                    if new_chr <= BIGGEST_USV_GN {
                                                        set_buffer_entry(k - 1, new_chr);
                                                        let shift = 2 * sup_count - 1;
                                                        set_cur_input_limit(
                                                            cur_input().limit - shift,
                                                        );
                                                        let mut kk = k;
                                                        while kk <= cur_input().limit {
                                                            set_buffer_entry(
                                                                kk,
                                                                buffer_entry(kk + shift),
                                                            );
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
                                        && cur_input().loc + 2 * sup_count - 2 <= cur_input().limit
                                        && cur_chr()
                                            == buffer_entry(cur_input().loc + sup_count - 1)
                                    {
                                        sup_count += 1;
                                    }
                                    let sup_count_save = sup_count;
                                    let mut hex_ok = true;
                                    for d in 1..=sup_count_save {
                                        if !is_lc_hex(buffer_entry(
                                            cur_input().loc + sup_count - 2 + d,
                                        )) {
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
                                            let c =
                                                buffer_entry(cur_input().loc + sup_count - 2 + d);
                                            new_chr = 16 * new_chr
                                                + if c <= b'9' as i32 {
                                                    c - b'0' as i32
                                                } else {
                                                    c - b'a' as i32 + 10
                                                };
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
                                    c"Text line contains an invalid character".as_ptr(),
                                );
                                capture_to_diagnostic(core::ptr::null_mut());
                                set_help_ptr(2);
                                set_help_line(
                                    1,
                                    c"A funny symbol that I can't read has just been input."
                                        .as_ptr(),
                                );
                                set_help_line(
                                    0,
                                    c"Continue, and I'll forget that it ever happened.".as_ptr(),
                                );
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
                                } else if eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_EOF) != TEX_NULL
                                    && !eof_seen(cur_input().index as i32)
                                {
                                    set_cur_input_limit(first() - 1);
                                    set_eof_seen(cur_input().index as i32, true);
                                    crate::input::begin_token_list(
                                        eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_EOF),
                                        EVERY_EOF_TEXT_GN,
                                    );
                                    continue 'restart;
                                } else {
                                    set_force_eof(true);
                                }
                            } else if input_line(input_file(cur_input().index as i32)) != 0 {
                                set_cur_input_limit(last());
                            } else if eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_EOF) != TEX_NULL
                                && !eof_seen(cur_input().index as i32)
                            {
                                set_cur_input_limit(first() - 1);
                                set_eof_seen(cur_input().index as i32, true);
                                crate::input::begin_token_list(
                                    eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_EOF),
                                    EVERY_EOF_TEXT_GN,
                                );
                                continue 'restart;
                            } else {
                                set_force_eof(true);
                            }
                        }

                        if force_eof() {
                            if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_NESTING) > 0
                                && (grp_stack(in_open()) != cur_boundary()
                                    || if_stack(in_open()) != cond_ptr())
                            {
                                crate::output::file_warning();
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

                        let end_char = eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR_GN);
                        if !(0..=255).contains(&end_char) {
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
                            && eqtb_b32s1(LOCAL_BASE + LOCAL_TECTONIC_CODA) != TEX_NULL
                        {
                            set_used_tectonic_coda_tokens(true);
                            crate::input::begin_token_list(
                                eqtb_b32s1(LOCAL_BASE + LOCAL_TECTONIC_CODA),
                                TECTONIC_CODA_TEXT_GN,
                            );
                            continue 'restart;
                        }
                        if selector() < SELECTOR_LOG_ONLY_GN {
                            open_log_file();
                        }
                        crate::errors::fatal_error(
                            c"*** (job aborted, no legal \\end found)".as_ptr(),
                        );
                    }
                    continue 'texswitch;
                }
            }
        } else if cur_input().loc != TEX_NULL {
            // Reading from a token list
            let t = mem_b32s0(cur_input().loc);
            set_cur_input_loc(mem_b32s1(cur_input().loc)); // LLIST_link

            if t >= CS_TOKEN_FLAG {
                set_cur_cs(t - CS_TOKEN_FLAG);
                set_cur_cmd(eqtb_b16s1(cur_cs()) as u8);
                set_cur_chr(eqtb_b32s1(cur_cs()));
                if cur_cmd() >= OUTER_CALL_GN {
                    if cur_cmd() == DONT_EXPAND_GN {
                        set_cur_cs(mem_b32s0(cur_input().loc) - CS_TOKEN_FLAG);
                        set_cur_input_loc(TEX_NULL);
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
            if scanner_status() == ALIGNING_GN || cur_align() == TEX_NULL {
                crate::errors::fatal_error(
                    c"(interwoven alignment preambles are not allowed)".as_ptr(),
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

// ---------------------------------------------------------------------------
// new_native_word_node
// ---------------------------------------------------------------------------

// Node type/subtype constants (xetex_format.h)
// INT_PAR offset for xetex_generate_actual_text (xetex_format.h:114)

/// Transitional bridge-down wrapper for legacy `new_native_word_node` callers
/// (`01b` §3, §5). Removed when its last legacy caller is itself threaded.
pub unsafe fn new_native_word_node_global(f: i32, n: i32) -> i32 {
    match crate::state::with_state_mut(|st| {
        let s = &mut *st;
        new_native_word_node(&mut s.mem, &s.eqtb, f, n)
    }) {
        Ok(q) => q,
        Err(e) => node_recycle_abort(e),
    }
}

// ---------------------------------------------------------------------------
// load_native_font
// ---------------------------------------------------------------------------

pub(crate) unsafe fn push_font_info_b32s1(v: i32) {
    let k = crate::globals::fmem_ptr();
    crate::globals::set_font_info_b32s1(k, v);
    crate::globals::set_fmem_ptr(k + 1);
}

// ---------------------------------------------------------------------------
// main_control_main_loop
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// main_control
// ---------------------------------------------------------------------------
