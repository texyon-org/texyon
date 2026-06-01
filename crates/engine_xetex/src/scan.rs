//! Scanner range-validation helpers and mu_error.
//!
//! Translated from xetex-xetex0.c.
use crate::consts::{
    ACTIVE_BASE, BATCH_MODE, BIGGEST_CHAR, BOX_BASE, CAT_CODE_BASE, CS_TOKEN_FLAG,
    CUR_FONT_LOC, DEL_CODE_BASE, DISC_NODE, EQTB_SIZE, ERROR_STOP_MODE,
    GLUE_BASE, GLUE_NODE, INT_BASE, KERN_NODE, LIGATURE_NODE, LOCAL_BASE,
    MATH_CODE_BASE, MATH_NODE, MAX_HALFWORD, PENALTY_NODE, PRIM_EQTB_BASE, RULE_NODE,
    SELECTOR_TERM_ONLY, SF_CODE_BASE, SINGLE_BASE, SMALL_NODE_SIZE, TEX_NULL, TOKS_BASE,
    TOO_BIG_CHAR,
};
use crate::errors::error;
use crate::errors::int_error;
use crate::errors::not_aat_font_error;
use crate::errors::not_aat_gr_font_error;
use crate::errors::not_native_font_error;
use crate::errors::not_ot_font_error;
// Relocated to errors.rs (workstream 02); still called from scan.rs.
use crate::errors::{
    cs_error, mu_error, privileged, report_illegal_case, runaway,
};
use crate::ext::aat_font_get;
use crate::ext::aat_font_get_1;
use crate::ext::aat_font_get_2;
use crate::ext::aat_font_get_named;
use crate::ext::aat_font_get_named_1;
use crate::ext::apply_mapping;
use crate::ext::get_font_char_range;
use crate::ext::get_glyph_bounds;
use crate::ext::get_native_char_height_depth;
use crate::ext::get_native_char_sidebearings;
use crate::ext::getnativechardp;
use crate::ext::getnativecharht;
use crate::ext::getnativecharic;
use crate::ext::getnativecharwd;
use crate::ext::gr_font_get_named;
use crate::ext::gr_font_get_named_1;
use crate::ext::map_char_to_glyph;
use crate::ext::map_glyph_to_index;
use crate::ext::measure_native_node;
use crate::ext::ot_font_get;
use crate::ext::ot_font_get_1;
use crate::ext::ot_font_get_2;
use crate::ext::ot_font_get_3;
use crate::ext::real_get_native_glyph_italic_correction;
use crate::ext::real_get_native_italic_correction;
use crate::ini::prefixed_command;
use crate::input::begin_token_list;
use crate::io::input_line;
use crate::io::u_close;
use crate::linebreak::do_locale_linebreaks;
use crate::math::after_math;
use crate::math::append_choices_global;
use crate::math::init_math;
use crate::math::math_ac_global;
use crate::math::math_fraction_global;
use crate::math::math_left_right_global;
use crate::math::math_limit_switch_global;
use crate::math::math_radical_global;
use crate::math::start_eq_no;
use crate::math::sub_sup_global;
use crate::mem::new_spec_global;
use crate::output::capture_to_diagnostic;
use crate::output::error_here_with_diagnostic;
use crate::pagebuilder::build_page;
use crate::pic::count_pdf_file_pages;

use crate::globals::{
    align_state, arith_error, avail, buffer_entry, cur_chr, cur_cmd, cur_cs, cur_group, cur_input,
    cur_list_mode, cur_tok, cur_val, cur_val_level, def_ref, eqtb_b32s1, eqtb_is_null,
    expand_depth, expand_depth_count, file_offset, first, fmem_ptr, font_glue, font_mem_size,
    font_params, font_ptr, hash_s1, init_pool_ptr, last, max_print_line, max_reg_help_line,
    max_reg_num, mem_b16s0, mem_b16s1, mem_b16s3, mem_b32s0, mem_b32s1, old_setting, open_parens,
    param_base, pool_ptr, pool_size, pseudo_files, read_file_entry, read_open_entry, rust_stdout,
    save_ptr, save_stack_b32s1, scanner_status, selector, set_align_state, set_arith_error,
    set_avail, set_buffer_entry, set_cur_chr, set_cur_cmd, set_cur_cs,
    set_cur_input_limit, set_cur_input_loc, set_cur_input_name, set_cur_input_state,
    set_cur_input_synctex_tag, set_cur_tok, set_cur_val, set_cur_val_level, set_def_ref, set_expand_depth_count, set_first, set_fmem_ptr, set_font_glue,
    set_font_info_b32s1, set_font_params, set_help_line, set_help_ptr, set_line, set_mem_b16s0,
    set_mem_b16s1, set_mem_b16s2, set_mem_b16s3, set_mem_b32s0, set_mem_b32s1,
    set_no_new_control_sequence, set_old_setting, set_open_parens, set_pool_ptr, set_pseudo_files,
    set_read_open_entry, set_save_ptr, set_save_stack_b32s1, set_scanner_status, set_selector,
    set_str_pool, set_str_ptr, set_warning_index, str_pool_entry, str_ptr, str_start, term_offset,
    warning_index,
};
use crate::mem::new_rule_global;
// Relocated to packaging.rs (workstream 02); still called from scan.rs.
use crate::packaging::{
    flush_node_list_global, hpack,
};
// Relocated to expand.rs (workstream 02); still called from scan.rs.
use crate::expand::{expand, macro_call};
// Relocated to main_control.rs (workstream 02); still called from scan.rs.
use crate::main_control::{
    align_error, app_space, append_penalty, begin_box, begin_insert_or_adjust,
    delete_last, do_endv, end_graf, handle_right_brace, head_for_vmode,
    indent_in_hmode, init_align, its_all_over, new_character, new_graf, no_align_error,
    normal_paragraph, omit_error, push_math, scan_box, unpackage,
};

// NULL_CS duplicated from the relocated conditional const block (still used here).
const NULL_CS: i32 = 2_228_225; // xetex_format.h:199
use crate::ext::{append_src_special, do_extension, insert_src_special};
use crate::fonts::{
    effective_char, effective_char_info, new_native_word_node, read_font_info,
};
use crate::io::{open_log_file, scan_and_pack_name, scan_file_name};
use crate::output::print_cmd_chr;
use crate::output::{char_warning, issue_message, show_box, show_whatever};

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
const OTHER_CHAR: u8 = 12;
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
unsafe fn back_input() {
    crate::input::back_input();
}

// xetex-constants.h:204
const INSERTED: u16 = 5;

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
const SEMI_SIMPLE_GROUP: u8 = 14;
const MATH_SHIFT_GROUP: u8 = 15;
const MATH_LEFT_GROUP: u8 = 16;

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
                                        set_cur_val(get_glyph_bounds(cf, n, cur_val()));
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
                                    set_cur_val(get_font_char_range(
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

const RIGHT_BRACE_TOKEN: i32 = 0x40_0000; // xetex-constants.h:351

// ---------------------------------------------------------------------------
// push_math
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// append_glue / append_kern
// ---------------------------------------------------------------------------

const SKIP_CODE: i16 = 4; // xetex_format.h:674

// translated from xetex-xetex0.c:9358 (append_glue)
#[no_mangle]
pub unsafe extern "C" fn append_glue() {
    let s = cur_chr() as i16;
    match s {
        0 => set_cur_val(4),
        1 => set_cur_val(8),
        2 => set_cur_val(12),
        3 => set_cur_val(16),
        4 => scan_glue(GLUE_VAL),    // SKIP_CODE
        5 => scan_glue(MU_VAL_SCAN), // MSKIP_CODE
        _ => {}
    }
    let p = crate::mem::new_glue_global(cur_val());
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
    let p = crate::mem::new_kern_global(cur_val());
    set_mem_b32s1(crate::globals::cur_list_tail(), p);
    crate::globals::set_cur_list_tail(p);
    set_mem_b16s0(crate::globals::cur_list_tail(), s);
}

// ---------------------------------------------------------------------------
// make_mark
// ---------------------------------------------------------------------------

const MARK_NODE: u16 = 4; // xetex_format.h:384

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
    let p = crate::mem::get_node_global(SMALL_NODE_SIZE);
    set_mem_b32s0(p + 1, c);
    set_mem_b16s1(p, MARK_NODE); // NODE_type(p)
    set_mem_b16s0(p, 0);
    set_mem_b32s1(p + 1, def_ref());
    set_mem_b32s1(crate::globals::cur_list_tail(), p);
    crate::globals::set_cur_list_tail(p);
}

// ---------------------------------------------------------------------------
// new_interaction
// ---------------------------------------------------------------------------

const SELECTOR_NO_PRINT: i32 = 16; // xetex-core.h:45

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
                set_help_line(
                    0,
                    c"I allow only values in the range 1..32767 here.".as_ptr(),
                );
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
            set_help_line(
                0,
                c"3=errorstop. Proceed, and I'll ignore this case.".as_ptr(),
            );
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
        crate::save::find_sa_element_global(4, cur_val(), false);
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
        crate::mem::delete_glue_ref_global(cur_val());
        set_cur_val(0);
    }
}

// ---------------------------------------------------------------------------
// fix_language
// ---------------------------------------------------------------------------

const INT_PAR_LANGUAGE: i32 = 50; // xetex_format.h:85
const INT_PAR_LEFT_HYPHEN_MIN: i32 = 51; // xetex_format.h:86
const INT_PAR_RIGHT_HYPHEN_MIN: i32 = 52; // xetex_format.h:87
const LANGUAGE_NODE_SUBTYPE: i16 = 4; // xetex_format.h:461 (whatsit subtype)

// translated from xetex-xetex0.c:11357 (fix_language)
#[no_mangle]
pub unsafe extern "C" fn fix_language() {
    let lang = eqtb_b32s1(INT_BASE + INT_PAR_LANGUAGE);
    let l = if lang <= 0 || lang > 255 { 0i32 } else { lang };
    if l != crate::globals::cur_list_aux_b32s1() {
        crate::mem::new_whatsit_global(LANGUAGE_NODE_SUBTYPE, SMALL_NODE_SIZE as i16);
        let tail = crate::globals::cur_list_tail();
        set_mem_b32s1(tail + 1, l);
        crate::globals::set_cur_list_aux_b32s1(l);
        set_mem_b16s1(
            tail + 1,
            crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as u16,
        );
        set_mem_b16s0(
            tail + 1,
            crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as u16,
        );
    }
}

// ---------------------------------------------------------------------------
// off_save
// ---------------------------------------------------------------------------

const BOTTOM_LEVEL: u8 = 0; // xetex_format.h (group level 0)
const FROZEN_END_GROUP: i32 = 2_243_226 + 2; // xetex_format.h:201 + xetex-constants.h:57
const FROZEN_RIGHT: i32 = 2_243_226 + 3; // xetex_format.h:201 + xetex-constants.h:58

// translated from xetex-xetex0.c:9325 (off_save)
#[no_mangle]
pub unsafe extern "C" fn off_save() {
    if cur_group() == BOTTOM_LEVEL {
        error_here_with_diagnostic(c"Extra ".as_ptr());
        print_cmd_chr(cur_cmd() as u16, cur_chr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(1);
        set_help_line(
            0,
            c"Things are pretty mixed up, but I think the worst is over.".as_ptr(),
        );
        error();
    } else {
        crate::input::back_input();
        let p = crate::mem::get_avail_global();
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
                let q = crate::mem::get_avail_global();
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
        set_help_line(
            4,
            c"I've inserted something that you may have forgotten.".as_ptr(),
        );
        set_help_line(3, c"(See the <inserted text> above.)".as_ptr());
        set_help_line(
            2,
            c"With luck, this will get me unwedged. But if you".as_ptr(),
        );
        set_help_line(
            1,
            c"really didn't forget anything, try typing `2' now; then".as_ptr(),
        );
        set_help_line(
            0,
            c"my insertion and my current dilemma will both disappear.".as_ptr(),
        );
        error();
    }
}

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

const DISC_GROUP: u8 = 10; // xetex-constants.h:162

// translated from xetex-xetex0.c:9956 (append_discretionary)
#[no_mangle]
pub unsafe extern "C" fn append_discretionary() {
    extern "C" {}

    let tail = crate::globals::cur_list_tail();
    set_mem_b32s1(tail, crate::mem::new_disc_global());
    crate::globals::set_cur_list_tail(mem_b32s1(tail));

    if cur_chr() == 1 {
        let f = eqtb_b32s1(CUR_FONT_LOC);
        let c = crate::globals::hyphen_char_entry(f);
        if (0..=BIGGEST_CHAR).contains(&c) {
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

const WHATSIT_NODE: u16 = 8; // xetex_format.h:388
const NATIVE_WORD_NODE: u16 = 40; // xetex_format.h:463
const NATIVE_WORD_NODE_AT: u16 = 41; // xetex_format.h:464
const GLYPH_NODE_BD: u16 = 42; // xetex_format.h:465
const MMODE: i16 = 207; // xetex_format.h:370

// translated from xetex-xetex0.c:9958 (build_discretionary)
#[no_mangle]
pub unsafe extern "C" fn build_discretionary() {
    crate::save::unsave();
    let q_head = crate::globals::cur_list_head();
    let mut q = q_head;
    let mut p = mem_b32s1(q);
    let mut n: i32 = 0;

    'walk: loop {
        if p == crate::mem::TEX_NULL {
            break;
        }
        if p < crate::globals::hi_mem_min() {
            // not a char node — check node type
            let ntype = mem_b16s1(p);
            if ntype > RULE_NODE {
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
                    set_help_line(
                        0,
                        c"Discretionary lists must contain only boxes and kerns.".as_ptr(),
                    );
                    error();

                    crate::output::begin_diagnostic();
                    crate::output::diagnostic_begin_capture_warning_here();
                    crate::output::print_nl_cstr(
                        c"The following discretionary sublist has been deleted:".as_ptr(),
                    );
                    show_box(p);
                    capture_to_diagnostic(std::ptr::null_mut());
                    crate::output::end_diagnostic(true);

                    flush_node_list_global(p);
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
                set_help_line(
                    1,
                    c"Sorry: The third part of a discretionary break must be".as_ptr(),
                );
                set_help_line(
                    0,
                    c"empty, in math formulas. I had to delete your third part.".as_ptr(),
                );
                flush_node_list_global(p);
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
                set_help_line(
                    1,
                    c"Wow---I never thought anybody would tweak me here.".as_ptr(),
                );
                set_help_line(
                    0,
                    c"You can't seriously need such a huge discretionary list?".as_ptr(),
                );
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

// translated from xetex-xetex0.c:9868 (append_italic_correction)
#[no_mangle]
pub unsafe extern "C" fn append_italic_correction() {
    extern "C" {
        // Note: in C these are macros: get_native_italic_correction(p) = real_get_native_italic_correction(&mem[p])

    }
    const EXPLICIT: u16 = 1; // xetex_format.h:428
    const GLYPH_NODE: u16 = 42; // xetex_format.h:465

    let tail = crate::globals::cur_list_tail();
    if tail == crate::globals::cur_list_head() {
        return;
    }

    let p = if tail >= crate::globals::hi_mem_min() {
        // is_char_node
        tail
    } else if mem_b16s1(tail) == LIGATURE_NODE {
        tail + 1
    } else if mem_b16s1(tail) == WHATSIT_NODE {
        let sub = mem_b16s0(tail);
        if sub == NATIVE_WORD_NODE || sub == NATIVE_WORD_NODE_AT {
            let kern = crate::mem::new_kern_global(real_get_native_italic_correction(
                crate::globals::mem_node_addr(tail),
            ));
            set_mem_b32s1(tail, kern);
            let new_tail = mem_b32s1(tail);
            crate::globals::set_cur_list_tail(new_tail);
            set_mem_b16s0(new_tail, EXPLICIT);
        } else if sub == GLYPH_NODE {
            let kern = crate::mem::new_kern_global(real_get_native_glyph_italic_correction(
                crate::globals::mem_node_addr(tail),
            ));
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
    let c = effective_char_global(true, f, mem_b16s0(p));
    let info = crate::globals::font_info_b16(crate::globals::char_base_entry(f) + c);
    let italic = crate::globals::font_info_b32s1(
        crate::globals::italic_base_entry(f) + (info.s1 / 4) as i32,
    );
    let kern = crate::mem::new_kern_global(italic);
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
    extern "C" {}
    const SLANT_CODE: i32 = 1; // xetex-constants.h:273
    const X_HEIGHT_CODE: i32 = 5; // xetex-constants.h:294
    const ACC_KERN: u16 = 2; // xetex_format.h:429
    const ADDITIONAL: i16 = 1; // xetex-constants.h:264
    const LETTER: u8 = 11; // xetex_format.h:243
    const CHAR_NUM: u8 = 16; // xetex_format.h:253

    let is_native = |fnt: i32| -> bool {
        crate::globals::font_area_entry(fnt) == AAT_FONT_FLAG
            || crate::globals::font_area_entry(fnt) == OTGR_FONT_FLAG
    };

    scan_char_num();
    let mut f = eqtb_b32s1(CUR_FONT_LOC);
    let p = new_character(f, cur_val() as u16);

    if p == crate::mem::TEX_NULL {
        return;
    }

    let x = crate::globals::font_info_b32s1(X_HEIGHT_CODE + crate::globals::param_base(f));
    let s = crate::globals::font_info_b32s1(SLANT_CODE + crate::globals::param_base(f)) as f64
        / 65536.0;
    let mut lsb: i32 = 0;
    let mut rsb: i32 = 0;
    let a = if is_native(f) {
        let av = mem_b32s1(p + 1);
        if av == 0 {
            get_native_char_sidebearings(f, cur_val(), &mut lsb, &mut rsb);
        }
        av
    } else {
        let c = effective_char_global(true, f, mem_b16s0(p));
        let info = crate::globals::font_info_b16(crate::globals::char_base_entry(f) + c);
        crate::globals::font_info_b32s1(crate::globals::width_base_entry(f) + info.s3 as i32)
    };

    do_assignments();
    let mut q = crate::mem::TEX_NULL;
    f = eqtb_b32s1(CUR_FONT_LOC);
    if cur_cmd() == LETTER || cur_cmd() == OTHER_CHAR || cur_cmd() == CHAR_GIVEN {
        q = new_character(f, cur_chr() as u16);
        set_cur_val(cur_chr());
    } else if cur_cmd() == CHAR_NUM {
        scan_char_num();
        q = new_character(f, cur_val() as u16);
    } else {
        crate::input::back_input();
    }

    if q != crate::mem::TEX_NULL {
        let t = crate::globals::font_info_b32s1(SLANT_CODE + crate::globals::param_base(f)) as f64
            / 65536.0;
        let (w, h) = if is_native(f) {
            let wv = mem_b32s1(q + 1);
            let mut hv: i32 = 0;
            let mut dv: i32 = 0;
            get_native_char_height_depth(f, cur_val(), &mut hv, &mut dv);
            (wv, hv)
        } else {
            let c = effective_char_global(true, f, mem_b16s0(q));
            let info = crate::globals::font_info_b16(crate::globals::char_base_entry(f) + c);
            let wv = crate::globals::font_info_b32s1(
                crate::globals::width_base_entry(f) + info.s3 as i32,
            );
            let hv = crate::globals::font_info_b32s1(
                crate::globals::height_base_entry(f) + (info.s2 / 16) as i32,
            );
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
            crate::scaled_math::tex_round((w - a) as f64 / 2.0 + h as f64 * t - x as f64 * s)
        };
        let r = crate::mem::new_kern_global(delta);
        set_mem_b16s0(r, ACC_KERN);
        set_mem_b32s1(crate::globals::cur_list_tail(), r);
        set_mem_b32s1(r, pa);
        let new_tail = crate::mem::new_kern_global(-a - delta);
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
    extern "C" {}

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
            crate::errors::overflow(
                c"pool size".as_ptr(),
                crate::globals::pool_size() - crate::globals::init_pool_ptr(),
            );
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
            set_help_line(
                1,
                c"I can only handle fonts at positive sizes that are".as_ptr(),
            );
            set_help_line(
                0,
                c"less than 2048pt, so I've changed what you said to 10pt.".as_ptr(),
            );
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
                c"Illegal magnification has been changed to 1000".as_ptr(),
            );
            ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), sv);
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(1);
            set_help_line(
                0,
                c"The magnification ratio must be between 1 and 32768.".as_ptr(),
            );
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
            if crate::string_pool::str_eq_str(
                crate::globals::font_name_entry(fi),
                crate::globals::cur_name(),
            ) && ((crate::string_pool::length(crate::globals::cur_area()) == 0
                && (crate::globals::font_area_entry(fi) == AAT_FONT_FLAG
                    || crate::globals::font_area_entry(fi) == OTGR_FONT_FLAG))
                || crate::string_pool::str_eq_str(
                    crate::globals::font_area_entry(fi),
                    crate::globals::cur_area(),
                ))
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
                        == crate::scaled_math::xn_over_d(
                            crate::globals::font_dsize_entry(fi),
                            -s,
                            1000,
                        )
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
    const ADVANCE: u8 = 92; // xetex_format.h:332
    const MULTIPLY: u8 = 93; // xetex_format.h:334
    const GLUE_REF: u16 = 119; // xetex_format.h:361
    const NORMAL: u16 = 0; // xetex_format.h:420

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
                set_help_line(
                    0,
                    c"I'm forgetting what you said and not changing anything.".as_ptr(),
                );
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
                crate::save::find_sa_element_global(p as u16, cur_val(), true);
                (crate::globals::cur_ptr(), p, true)
            } else {
                let l = match p {
                    INT_VAL => cur_val() + COUNT_BASE,
                    DIMEN_VAL => cur_val() + SCALED_BASE,
                    GLUE_VAL => cur_val() + SKIP_BASE,
                    MU_VAL => cur_val() + MU_SKIP_BASE,
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
                let qs = crate::mem::new_spec_global(cur_val());
                let r = s;
                crate::mem::delete_glue_ref_global(cur_val());
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
                    set_cur_val(crate::scaled_math::mult_and_add(
                        w,
                        cur_val(),
                        0,
                        TEX_INFINITY,
                    ));
                } else {
                    set_cur_val(crate::scaled_math::mult_and_add(
                        w,
                        cur_val(),
                        0,
                        MAX_HALFWORD,
                    ));
                }
            } else {
                set_cur_val(crate::scaled_math::x_over_n(w, cur_val()));
            }
        } else {
            let r = crate::mem::new_spec_global(s);
            if q == MULTIPLY {
                set_mem_b32s1(
                    r + 1,
                    crate::scaled_math::mult_and_add(mem_b32s1(s + 1), cur_val(), 0, MAX_HALFWORD),
                );
                set_mem_b32s1(
                    r + 2,
                    crate::scaled_math::mult_and_add(mem_b32s1(s + 2), cur_val(), 0, MAX_HALFWORD),
                );
                set_mem_b32s1(
                    r + 3,
                    crate::scaled_math::mult_and_add(mem_b32s1(s + 3), cur_val(), 0, MAX_HALFWORD),
                );
            } else {
                set_mem_b32s1(
                    r + 1,
                    crate::scaled_math::x_over_n(mem_b32s1(s + 1), cur_val()),
                );
                set_mem_b32s1(
                    r + 2,
                    crate::scaled_math::x_over_n(mem_b32s1(s + 2), cur_val()),
                );
                set_mem_b32s1(
                    r + 3,
                    crate::scaled_math::x_over_n(mem_b32s1(s + 3), cur_val()),
                );
            }
            set_cur_val(r);
        }
    }

    if crate::globals::arith_error() {
        error_here_with_diagnostic(c"Arithmetic overflow".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"I can't carry out that multiplication or division,".as_ptr(),
        );
        set_help_line(0, c"since the result is out of range.".as_ptr());
        if p >= GLUE_VAL {
            crate::mem::delete_glue_ref_global(cur_val());
        }
        error();
        return;
    }

    if p < GLUE_VAL {
        if e {
            if a >= 4 {
                crate::save::gsa_w_def_global(l, cur_val());
            } else {
                crate::save::sa_w_def_global(l, cur_val());
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
                crate::save::gsa_def_global(l, cur_val());
            } else {
                crate::save::sa_def_global(l, cur_val());
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

// ---------------------------------------------------------------------------
// delete_last
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:8221 (main_control append_normal_space label)
#[no_mangle]
pub unsafe extern "C" fn main_control_append_normal_space() {
    const INT_PAR_XETEX_INTER_CHAR_TOKENS_MC: i32 = 73; // xetex_format.h:108
    const GLUE_PAR_SPACE_SKIP_MC: i16 = 12; // xetex_format.h:161
    const INTER_CHAR_TEXT_MC: u16 = 17; // xetex-constants.h:216
    const CHAR_NUM_MC: u8 = 16; // xetex_format.h:253

    if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS_MC) > 0
        && crate::globals::space_class() != CHAR_CLASS_LIMIT
        && crate::globals::prev_class() != CHAR_CLASS_LIMIT - 1
    {
        crate::globals::set_prev_class(CHAR_CLASS_LIMIT - 1);
        crate::save::find_sa_element_global(
            INTER_CHAR_VAL as u16,
            crate::globals::space_class() * CHAR_CLASS_LIMIT + (CHAR_CLASS_LIMIT - 1),
            false,
        );
        if crate::globals::cur_ptr() != crate::mem::TEX_NULL
            && mem_b32s1(crate::globals::cur_ptr() + 1) != crate::mem::TEX_NULL
        {
            if cur_cs() == 0 {
                if cur_cmd() == CHAR_NUM_MC {
                    set_cur_cmd(OTHER_CHAR);
                }
                set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
            } else {
                set_cur_tok(CS_TOKEN_FLAG + cur_cs());
            }
            back_input();
            begin_token_list(mem_b32s1(crate::globals::cur_ptr() + 1), INTER_CHAR_TEXT_MC);
            return;
        }
    }

    let cur_font = eqtb_b32s1(CUR_FONT_LOC);
    if eqtb_b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP_MC as i32) == 0 {
        crate::globals::set_main_p(font_glue(cur_font));
        if crate::globals::main_p() == crate::mem::TEX_NULL {
            crate::globals::set_main_p(crate::mem::new_spec_global(0));
            crate::globals::set_main_k(param_base(cur_font) + 2);
            set_mem_b32s1(
                crate::globals::main_p() + 1,
                crate::globals::font_info_b32s1(crate::globals::main_k()),
            );
            set_mem_b32s1(
                crate::globals::main_p() + 2,
                crate::globals::font_info_b32s1(crate::globals::main_k() + 1),
            );
            set_mem_b32s1(
                crate::globals::main_p() + 3,
                crate::globals::font_info_b32s1(crate::globals::main_k() + 2),
            );
            set_font_glue(cur_font, crate::globals::main_p());
        }
        crate::globals::set_temp_ptr(crate::mem::new_glue_global(crate::globals::main_p()));
    } else {
        crate::globals::set_temp_ptr(crate::mem::new_param_glue_global(GLUE_PAR_SPACE_SKIP_MC));
    }

    set_mem_b32s1(crate::globals::cur_list_tail(), crate::globals::temp_ptr());
    crate::globals::set_cur_list_tail(crate::globals::temp_ptr());
}

// translated from xetex-xetex0.c:7277 (main_control mode-independent cases)
#[no_mangle]
pub unsafe extern "C" fn main_control_mode_independent() {
    extern "C" {}

    const AFTER_ASSIGNMENT_MC: u8 = 40;
    const AFTER_GROUP_MC: u8 = 41;
    const XRAY_MC: u8 = 19;
    const CASE_SHIFT_MC: u8 = 57;
    const MESSAGE_MC: u8 = 58;
    const EXTENSION_MC: u8 = 59;
    const IN_STREAM_MC: u8 = 60;
    const TOKS_REGISTER_MC: u8 = 72;
    const ASSIGN_TOKS_MC: u8 = 73;
    const ASSIGN_INT_MC: u8 = 74;
    const ASSIGN_DIMEN_MC: u8 = 75;
    const ASSIGN_GLUE_MC: u8 = 76;
    const ASSIGN_MU_GLUE_MC: u8 = 77;
    const ASSIGN_FONT_DIMEN_MC: u8 = 78;
    const ASSIGN_FONT_INT_MC: u8 = 79;
    const SET_AUX_MC: u8 = 80;
    const SET_PREV_GRAF_MC: u8 = 81;
    const SET_PAGE_DIMEN_MC: u8 = 82;
    const SET_PAGE_INT_MC: u8 = 83;
    const SET_BOX_DIMEN_MC: u8 = 84;
    const SET_SHAPE_MC: u8 = 85;
    const DEF_CODE_MC: u8 = 86;
    const XETEX_DEF_CODE_MC: u8 = 87;
    const DEF_FAMILY_MC: u8 = 88;
    const SET_FONT_MC: u8 = 89;
    const DEF_FONT_MC: u8 = 90;
    const REGISTER_MC: u8 = 91;
    const ADVANCE_MC: u8 = 92;
    const MULTIPLY_MC: u8 = 93;
    const DIVIDE_MC: u8 = 94;
    const PREFIX_MC: u8 = 95;
    const LET_MC: u8 = 96;
    const SHORTHAND_DEF_MC: u8 = 97;
    const READ_TO_CS_MC: u8 = 98;
    const DEF_MC: u8 = 99;
    const SET_BOX_MC: u8 = 100;
    const HYPH_DATA_MC: u8 = 101;
    const SET_INTERACTION_MC: u8 = 102;

    match cur_cmd() {
        TOKS_REGISTER_MC | ASSIGN_TOKS_MC | ASSIGN_INT_MC | ASSIGN_DIMEN_MC | ASSIGN_GLUE_MC
        | ASSIGN_MU_GLUE_MC | ASSIGN_FONT_DIMEN_MC | ASSIGN_FONT_INT_MC | SET_AUX_MC
        | SET_PREV_GRAF_MC | SET_PAGE_DIMEN_MC | SET_PAGE_INT_MC | SET_BOX_DIMEN_MC
        | SET_SHAPE_MC | DEF_CODE_MC | XETEX_DEF_CODE_MC | DEF_FAMILY_MC | SET_FONT_MC
        | DEF_FONT_MC | REGISTER_MC | ADVANCE_MC | MULTIPLY_MC | DIVIDE_MC | PREFIX_MC | LET_MC
        | SHORTHAND_DEF_MC | READ_TO_CS_MC | DEF_MC | SET_BOX_MC | HYPH_DATA_MC
        | SET_INTERACTION_MC => prefixed_command(),
        AFTER_ASSIGNMENT_MC => {
            get_token();
            crate::globals::set_after_token(cur_tok());
        }
        AFTER_GROUP_MC => {
            get_token();
            crate::save::save_for_after(cur_tok());
        }
        IN_STREAM_MC => crate::io::open_or_close_in(),
        MESSAGE_MC => issue_message(),
        CASE_SHIFT_MC => shift_case(),
        XRAY_MC => show_whatever(),
        EXTENSION_MC => do_extension(),
        _ => {}
    }
}

// translated from xetex-xetex0.c:6900 (main_control forbidden/math-mode mismatch cases)
#[no_mangle]
pub unsafe extern "C" fn main_control_mode_error_cases() {
    const VMODE_MC: u16 = 1;
    const HMODE_MC: u16 = 104;
    const MMODE_MC: u16 = 207;
    const MAC_PARAM_MC: u8 = 6;
    const SUP_MARK_MC: u8 = 7;
    const SUB_MARK_MC: u8 = 8;
    const ENDV_MC: u8 = 9;
    const PAR_END_MC: u8 = 13;
    const STOP_MC: u8 = 14;
    const DELIM_NUM_MC: u8 = 15;
    const MATH_CHAR_NUM_MC: u8 = 17;
    const HMOVE_MC: u8 = 21;
    const VMOVE_MC: u8 = 22;
    const UN_VBOX_MC: u8 = 24;
    const VSKIP_MC: u8 = 27;
    const MSKIP_MC: u8 = 28;
    const MKERN_MC: u8 = 30;
    const VALIGN_MC: u8 = 33;
    const HRULE_MC: u8 = 36;
    const VADJUST_MC: u8 = 38;
    const ITAL_CORR_MC: u8 = 44;
    const MATH_ACCENT_MC: u8 = 46;
    const EQ_NO_MC: u8 = 48;
    const LEFT_RIGHT_MC: u8 = 49;
    const MATH_COMP_MC: u8 = 50;
    const LIMIT_SWITCH_MC: u8 = 51;
    const ABOVE_MC: u8 = 52;
    const MATH_STYLE_MC: u8 = 53;
    const MATH_CHOICE_MC: u8 = 54;
    const NON_SCRIPT_MC: u8 = 55;
    const VCENTER_MC: u8 = 56;
    const RADICAL_MC: u8 = 66;
    const MATH_GIVEN_MC: u8 = 69;
    const XETEX_MATH_GIVEN_MC: u8 = 70;
    const LAST_ITEM_MC: u8 = 71;

    match (crate::globals::cur_list_mode().unsigned_abs(), cur_cmd()) {
        (VMODE_MC, VMOVE_MC)
        | (HMODE_MC, HMOVE_MC)
        | (MMODE_MC, HMOVE_MC)
        | (VMODE_MC, LAST_ITEM_MC)
        | (HMODE_MC, LAST_ITEM_MC)
        | (MMODE_MC, LAST_ITEM_MC)
        | (VMODE_MC, VADJUST_MC)
        | (VMODE_MC, ITAL_CORR_MC)
        | (VMODE_MC, EQ_NO_MC)
        | (HMODE_MC, EQ_NO_MC)
        | (VMODE_MC, MAC_PARAM_MC)
        | (HMODE_MC, MAC_PARAM_MC)
        | (MMODE_MC, MAC_PARAM_MC) => {
            report_illegal_case();
        }
        (VMODE_MC, SUP_MARK_MC)
        | (HMODE_MC, SUP_MARK_MC)
        | (VMODE_MC, SUB_MARK_MC)
        | (HMODE_MC, SUB_MARK_MC)
        | (VMODE_MC, MATH_CHAR_NUM_MC)
        | (HMODE_MC, MATH_CHAR_NUM_MC)
        | (VMODE_MC, MATH_GIVEN_MC)
        | (HMODE_MC, MATH_GIVEN_MC)
        | (VMODE_MC, XETEX_MATH_GIVEN_MC)
        | (HMODE_MC, XETEX_MATH_GIVEN_MC)
        | (VMODE_MC, MATH_COMP_MC)
        | (HMODE_MC, MATH_COMP_MC)
        | (VMODE_MC, DELIM_NUM_MC)
        | (HMODE_MC, DELIM_NUM_MC)
        | (VMODE_MC, LEFT_RIGHT_MC)
        | (HMODE_MC, LEFT_RIGHT_MC)
        | (VMODE_MC, ABOVE_MC)
        | (HMODE_MC, ABOVE_MC)
        | (VMODE_MC, RADICAL_MC)
        | (HMODE_MC, RADICAL_MC)
        | (VMODE_MC, MATH_STYLE_MC)
        | (HMODE_MC, MATH_STYLE_MC)
        | (VMODE_MC, MATH_CHOICE_MC)
        | (HMODE_MC, MATH_CHOICE_MC)
        | (VMODE_MC, VCENTER_MC)
        | (HMODE_MC, VCENTER_MC)
        | (VMODE_MC, NON_SCRIPT_MC)
        | (HMODE_MC, NON_SCRIPT_MC)
        | (VMODE_MC, MKERN_MC)
        | (HMODE_MC, MKERN_MC)
        | (VMODE_MC, LIMIT_SWITCH_MC)
        | (HMODE_MC, LIMIT_SWITCH_MC)
        | (VMODE_MC, MSKIP_MC)
        | (HMODE_MC, MSKIP_MC)
        | (VMODE_MC, MATH_ACCENT_MC)
        | (HMODE_MC, MATH_ACCENT_MC)
        | (MMODE_MC, ENDV_MC)
        | (MMODE_MC, PAR_END_MC)
        | (MMODE_MC, STOP_MC)
        | (MMODE_MC, VSKIP_MC)
        | (MMODE_MC, UN_VBOX_MC)
        | (MMODE_MC, VALIGN_MC)
        | (MMODE_MC, HRULE_MC) => insert_dollar_sign(),
        _ => {}
    }
}

// translated from xetex-xetex0.c:6932 (main_control simple list/group cases)
#[no_mangle]
pub unsafe extern "C" fn main_control_simple_list_group_cases() {
    const VMODE_MC: u16 = 1;
    const HMODE_MC: u16 = 104;
    const MMODE_MC: u16 = 207;
    const LEFT_BRACE_MC: u8 = 1;
    const KERN_MC: u8 = 29;
    const MKERN_MC: u8 = 30;
    const HSKIP_MC: u8 = 26;
    const VSKIP_MC: u8 = 27;
    const MSKIP_MC: u8 = 28;
    const BEGIN_GROUP_MC: u8 = 61;
    const END_GROUP_MC: u8 = 62;
    const HRULE_MC: u8 = 36;
    const VRULE_MC: u8 = 35;
    const SIMPLE_GROUP_MC: u8 = 1;
    const SEMI_SIMPLE_GROUP_MC: u8 = 14;
    const IGNORE_DEPTH_MC: i32 = -65_536_000;

    let mode = crate::globals::cur_list_mode().unsigned_abs();
    match (mode, cur_cmd()) {
        (VMODE_MC, HRULE_MC) | (HMODE_MC, VRULE_MC) | (MMODE_MC, VRULE_MC) => {
            let p = scan_rule_spec();
            crate::globals::set_mem_b32s1(crate::globals::cur_list_tail(), p);
            crate::globals::set_cur_list_tail(p);
            if mode == VMODE_MC {
                crate::globals::set_cur_list_aux_b32s1(IGNORE_DEPTH_MC);
            } else if mode == HMODE_MC {
                crate::globals::set_cur_list_aux_b32s0(1000);
            }
        }
        (VMODE_MC, VSKIP_MC)
        | (HMODE_MC, HSKIP_MC)
        | (MMODE_MC, HSKIP_MC)
        | (MMODE_MC, MSKIP_MC) => append_glue(),
        (VMODE_MC, KERN_MC) | (HMODE_MC, KERN_MC) | (MMODE_MC, KERN_MC) | (MMODE_MC, MKERN_MC) => {
            append_kern()
        }
        (VMODE_MC, LEFT_BRACE_MC) | (HMODE_MC, LEFT_BRACE_MC) => {
            crate::save::new_save_level(SIMPLE_GROUP_MC);
        }
        (VMODE_MC, BEGIN_GROUP_MC) | (HMODE_MC, BEGIN_GROUP_MC) | (MMODE_MC, BEGIN_GROUP_MC) => {
            crate::save::new_save_level(SEMI_SIMPLE_GROUP_MC);
        }
        (VMODE_MC, END_GROUP_MC) | (HMODE_MC, END_GROUP_MC) | (MMODE_MC, END_GROUP_MC) => {
            if crate::globals::cur_group() == SEMI_SIMPLE_GROUP_MC {
                crate::save::unsave();
            } else {
                off_save();
            }
        }
        _ => {}
    }
}

// translated from xetex-xetex0.c:6964 (main_control box/list construction cases)
#[no_mangle]
pub unsafe extern "C" fn main_control_box_list_cases() {
    const VMODE_MC: u16 = 1;
    const HMODE_MC: u16 = 104;
    const MMODE_MC: u16 = 207;
    const RIGHT_BRACE_MC: u8 = 2;
    const MATH_SHIFT_MC: u8 = 3;
    const TAB_MARK_MC: u8 = 4;
    const CAR_RET_MC: u8 = 5;
    const LETTER_MC: u8 = 11;
    const OTHER_CHAR_MC: u8 = 12;
    const PAR_END_MC: u8 = 13;
    const STOP_MC: u8 = 14;
    const CHAR_NUM_MC: u8 = 16;
    const MARK_MC: u8 = 18;
    const MAKE_BOX_MC: u8 = 20;
    const HMOVE_MC: u8 = 21;
    const VMOVE_MC: u8 = 22;
    const UN_HBOX_MC: u8 = 23;
    const UN_VBOX_MC: u8 = 24;
    const REMOVE_ITEM_MC: u8 = 25;
    const HSKIP_MC: u8 = 26;
    const VSKIP_MC: u8 = 27;
    const LEADER_SHIP_MC: u8 = 31;
    const HALIGN_MC: u8 = 32;
    const VALIGN_MC: u8 = 33;
    const VRULE_MC: u8 = 35;
    const HRULE_MC: u8 = 36;
    const INSERT_MC: u8 = 37;
    const VADJUST_MC: u8 = 38;
    const BREAK_PENALTY_MC: u8 = 42;
    const START_PAR_MC: u8 = 43;
    const ITAL_CORR_MC: u8 = 44;
    const ACCENT_MC: u8 = 45;
    const DISCRETIONARY_MC: u8 = 47;
    const EX_SPACE_MC: u8 = 64;
    const NO_BOUNDARY_MC: u8 = 65;
    const CHAR_GIVEN_MC: u8 = 68;
    const LEADER_FLAG_MC: i32 = 0x4001_0001;
    const A_LEADERS_MC: i32 = 100;

    match (crate::globals::cur_list_mode().unsigned_abs(), cur_cmd()) {
        (VMODE_MC, RIGHT_BRACE_MC) | (HMODE_MC, RIGHT_BRACE_MC) | (MMODE_MC, RIGHT_BRACE_MC) => {
            handle_right_brace()
        }
        (VMODE_MC, HMOVE_MC) | (HMODE_MC, VMOVE_MC) | (MMODE_MC, VMOVE_MC) => {
            let t = cur_chr();
            scan_dimen(false, false, false);
            if t == 0 {
                scan_box(cur_val());
            } else {
                scan_box(-cur_val());
            }
        }
        (VMODE_MC, LEADER_SHIP_MC) | (HMODE_MC, LEADER_SHIP_MC) | (MMODE_MC, LEADER_SHIP_MC) => {
            scan_box(LEADER_FLAG_MC - A_LEADERS_MC + cur_chr());
        }
        (VMODE_MC, MAKE_BOX_MC) | (HMODE_MC, MAKE_BOX_MC) | (MMODE_MC, MAKE_BOX_MC) => begin_box(0),
        (VMODE_MC, START_PAR_MC) => new_graf(cur_chr() > 0),
        (VMODE_MC, LETTER_MC)
        | (VMODE_MC, OTHER_CHAR_MC)
        | (VMODE_MC, CHAR_NUM_MC)
        | (VMODE_MC, CHAR_GIVEN_MC)
        | (VMODE_MC, MATH_SHIFT_MC)
        | (VMODE_MC, UN_HBOX_MC)
        | (VMODE_MC, VRULE_MC)
        | (VMODE_MC, ACCENT_MC)
        | (VMODE_MC, DISCRETIONARY_MC)
        | (VMODE_MC, HSKIP_MC)
        | (VMODE_MC, VALIGN_MC)
        | (VMODE_MC, EX_SPACE_MC)
        | (VMODE_MC, NO_BOUNDARY_MC) => {
            crate::input::back_input();
            new_graf(true);
        }
        (HMODE_MC, START_PAR_MC) | (MMODE_MC, START_PAR_MC) => indent_in_hmode(),
        (VMODE_MC, PAR_END_MC) => {
            normal_paragraph();
            if crate::globals::cur_list_mode() > 0 {
                build_page();
            }
        }
        (HMODE_MC, PAR_END_MC) => {
            if crate::globals::align_state() < 0 {
                off_save();
            }
            end_graf();
            if crate::globals::cur_list_mode() == VMODE_MC as i16 {
                build_page();
            }
        }
        (HMODE_MC, STOP_MC)
        | (HMODE_MC, VSKIP_MC)
        | (HMODE_MC, HRULE_MC)
        | (HMODE_MC, UN_VBOX_MC)
        | (HMODE_MC, HALIGN_MC) => head_for_vmode(),
        (VMODE_MC, INSERT_MC)
        | (HMODE_MC, INSERT_MC)
        | (MMODE_MC, INSERT_MC)
        | (HMODE_MC, VADJUST_MC)
        | (MMODE_MC, VADJUST_MC) => begin_insert_or_adjust(),
        (VMODE_MC, MARK_MC) | (HMODE_MC, MARK_MC) | (MMODE_MC, MARK_MC) => make_mark(),
        (VMODE_MC, BREAK_PENALTY_MC)
        | (HMODE_MC, BREAK_PENALTY_MC)
        | (MMODE_MC, BREAK_PENALTY_MC) => append_penalty(),
        (VMODE_MC, REMOVE_ITEM_MC) | (HMODE_MC, REMOVE_ITEM_MC) | (MMODE_MC, REMOVE_ITEM_MC) => {
            delete_last()
        }
        (VMODE_MC, UN_VBOX_MC) | (HMODE_MC, UN_HBOX_MC) | (MMODE_MC, UN_HBOX_MC) => unpackage(),
        (HMODE_MC, ITAL_CORR_MC) => append_italic_correction(),
        (MMODE_MC, ITAL_CORR_MC) => {
            let p = crate::mem::new_kern_global(0);
            crate::globals::set_mem_b32s1(crate::globals::cur_list_tail(), p);
            crate::globals::set_cur_list_tail(p);
        }
        (HMODE_MC, DISCRETIONARY_MC) | (MMODE_MC, DISCRETIONARY_MC) => append_discretionary(),
        (HMODE_MC, ACCENT_MC) => make_accent(),
        (VMODE_MC, CAR_RET_MC)
        | (HMODE_MC, CAR_RET_MC)
        | (MMODE_MC, CAR_RET_MC)
        | (VMODE_MC, TAB_MARK_MC)
        | (HMODE_MC, TAB_MARK_MC)
        | (MMODE_MC, TAB_MARK_MC) => align_error(),
        _ => {}
    }
}

// translated from xetex-xetex0.c:6993 (main_control alignment/endcsname cases)
#[no_mangle]
pub unsafe extern "C" fn main_control_alignment_cases() {
    const VMODE_MC: u16 = 1;
    const HMODE_MC: u16 = 104;
    const MMODE_MC: u16 = 207;
    const ENDV_MC: u8 = 9;
    const HALIGN_MC: u8 = 32;
    const VALIGN_MC: u8 = 33;
    const NO_ALIGN_MC: u8 = 34;
    const OMIT_MC: u8 = 63;
    const END_CS_NAME_MC: u8 = 67;
    const INT_PAR_TEXXET_MC: i32 = 69;
    const MATH_SHIFT_GROUP_MC: u8 = 15;

    match (crate::globals::cur_list_mode().unsigned_abs(), cur_cmd()) {
        (VMODE_MC, NO_ALIGN_MC) | (HMODE_MC, NO_ALIGN_MC) | (MMODE_MC, NO_ALIGN_MC) => {
            no_align_error()
        }
        (VMODE_MC, OMIT_MC) | (HMODE_MC, OMIT_MC) | (MMODE_MC, OMIT_MC) => omit_error(),
        (VMODE_MC, HALIGN_MC) => init_align(),
        (HMODE_MC, VALIGN_MC) => {
            if cur_chr() > 0 {
                if eTeX_enabled(
                    crate::globals::eqtb_b32s1(INT_BASE + INT_PAR_TEXXET_MC) > 0,
                    cur_cmd() as u16,
                    cur_chr(),
                ) {
                    let p = crate::mem::new_math_global(0, cur_chr() as i16);
                    crate::globals::set_mem_b32s1(crate::globals::cur_list_tail(), p);
                    crate::globals::set_cur_list_tail(p);
                }
            } else {
                init_align();
            }
        }
        (MMODE_MC, HALIGN_MC) => {
            if privileged() {
                if crate::globals::cur_group() == MATH_SHIFT_GROUP_MC {
                    init_align();
                } else {
                    off_save();
                }
            }
        }
        (VMODE_MC, ENDV_MC) | (HMODE_MC, ENDV_MC) => do_endv(),
        (VMODE_MC, END_CS_NAME_MC) | (HMODE_MC, END_CS_NAME_MC) | (MMODE_MC, END_CS_NAME_MC) => {
            cs_error()
        }
        _ => {}
    }
}

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
fn set_class_fn(x: i32) -> i32 {
    (((x as u32) & 0x07) << 21) as i32
}
#[inline]
fn set_family_fn(x: i32) -> i32 {
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

// translated from xetex-xetex0.c:10745 (do_assignments)
#[no_mangle]
pub unsafe extern "C" fn do_assignments() {
    extern "C" {}

    const MAX_NON_PREFIXED_COMMAND: u8 = 71; // xetex_format.h:312
    const RELAX: u8 = 0; // xetex_format.h:229

    loop {
        loop {
            get_x_token();
            if cur_cmd() != SPACER && cur_cmd() != RELAX {
                break;
            }
        }
        if cur_cmd() <= MAX_NON_PREFIXED_COMMAND {
            return;
        }
        crate::globals::set_set_box_allowed(false);
        prefixed_command();
        crate::globals::set_set_box_allowed(true);
    }
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

// translated from xetex-xetex0.c:7006 (main_control math entry/exit cases)
#[no_mangle]
pub unsafe extern "C" fn main_control_math_cases() {
    extern "C" {}

    const HMODE_MC: u16 = 104; // xetex_format.h:369
    const MMODE_MC: u16 = 207; // xetex_format.h:370
    const MATH_SHIFT_MC: u8 = 3; // xetex_format.h:233
    const LEFT_BRACE_MC: u8 = 1; // xetex_format.h:231
    const SUP_MARK_MC: u8 = 7; // xetex_format.h:238
    const SUB_MARK_MC: u8 = 8; // xetex_format.h:239
    const LETTER_MC: u8 = 11; // xetex_format.h:243
    const OTHER_CHAR_MC: u8 = 12; // xetex_format.h:244
    const DELIM_NUM_MC: u8 = 15; // xetex_format.h:251
    const CHAR_NUM_MC: u8 = 16; // xetex_format.h:253
    const MATH_CHAR_NUM_MC: u8 = 17; // xetex_format.h:255
    const ACCENT_MC: u8 = 45; // xetex_format.h:283
    const MATH_ACCENT_MC: u8 = 46; // xetex_format.h:284
    const EQ_NO_MC: u8 = 48; // xetex_format.h:286
    const LEFT_RIGHT_MC: u8 = 49; // xetex_format.h:287
    const MATH_COMP_MC: u8 = 50; // xetex_format.h:288
    const LIMIT_SWITCH_MC: u8 = 51; // xetex_format.h:289
    const ABOVE_MC: u8 = 52; // xetex_format.h:290
    const MATH_STYLE_MC: u8 = 53; // xetex_format.h:291
    const MATH_CHOICE_MC: u8 = 54; // xetex_format.h:292
    const NON_SCRIPT_MC: u8 = 55; // xetex_format.h:293
    const VCENTER_MC: u8 = 56; // xetex_format.h:294
    const RADICAL_MC: u8 = 66; // xetex_format.h:304
    const CHAR_GIVEN_MC: u8 = 68; // xetex_format.h:306
    const MATH_GIVEN_MC: u8 = 69; // xetex_format.h:308
    const XETEX_MATH_GIVEN_MC: u8 = 70; // xetex_format.h:309

    const MATH_SHIFT_GROUP_MC: u8 = 15; // xetex-constants.h:167
    const VCENTER_GROUP_MC: u8 = 12; // xetex-constants.h:164
    const COND_MATH_GLUE_MC: u16 = 98; // xetex-constants.h:324
    const IGNORE_DEPTH_MC: i32 = -65_536_000; // xetex_format.h:388
    const EVERY_VBOX_TEXT_MC: u16 = 12; // xetex-constants.h:211
    const LOCAL_EVERY_VBOX_MC: i32 = 6; // xetex_format.h:178

    use crate::globals::{
        cur_chr, cur_cmd, cur_group, cur_list_mode, cur_list_tail, eqtb_b32s1, mem_b32s1,
        set_cur_chr, set_cur_list_aux_b32s1, set_cur_list_mode, set_cur_list_tail, set_mem_b16s0,
        set_mem_b16s1, set_mem_b32s1,
    };
    use crate::mem::TEX_NULL;

    match (cur_list_mode().unsigned_abs(), cur_cmd()) {
        (HMODE_MC, MATH_SHIFT_MC) => init_math(),

        (MMODE_MC, EQ_NO_MC) => {
            if privileged() {
                if cur_group() == MATH_SHIFT_GROUP_MC {
                    start_eq_no();
                } else {
                    off_save();
                }
            }
        }

        (MMODE_MC, LEFT_BRACE_MC) => {
            let n = crate::mem::new_noad_global();
            set_mem_b32s1(cur_list_tail(), n);
            set_cur_list_tail(mem_b32s1(cur_list_tail())); // LLIST_link(tail) = n
            crate::input::back_input();
            scan_math(cur_list_tail() + 1);
        }

        (MMODE_MC, LETTER_MC) | (MMODE_MC, OTHER_CHAR_MC) | (MMODE_MC, CHAR_GIVEN_MC) => {
            set_math_char(eqtb_b32s1(MATH_CODE_BASE + cur_chr()));
        }

        (MMODE_MC, CHAR_NUM_MC) => {
            scan_char_num();
            set_cur_chr(cur_val());
            set_math_char(eqtb_b32s1(MATH_CODE_BASE + cur_chr()));
        }

        (MMODE_MC, MATH_CHAR_NUM_MC) => {
            if cur_chr() == 2 {
                // \Umathchar
                scan_math_class_int();
                let mut t = set_class_fn(cur_val());
                scan_math_fam_int();
                t += set_family_fn(cur_val());
                scan_usv_num();
                t += cur_val();
                set_math_char(t);
            } else if cur_chr() == 1 {
                // \Umathcharnum
                scan_xetex_math_char_int();
                set_math_char(cur_val());
            } else {
                scan_fifteen_bit_int();
                let v = cur_val();
                set_math_char(set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256));
            }
        }

        (MMODE_MC, MATH_GIVEN_MC) => {
            let c = cur_chr();
            set_math_char(set_class_fn(c / 4096) + set_family_fn((c % 4096) / 256) + (c % 256));
        }

        (MMODE_MC, XETEX_MATH_GIVEN_MC) => {
            set_math_char(cur_chr());
        }

        (MMODE_MC, DELIM_NUM_MC) => {
            if cur_chr() == 1 {
                // \Udelimiter
                scan_math_class_int();
                let mut t = set_class_fn(cur_val());
                scan_math_fam_int();
                t += set_family_fn(cur_val());
                scan_usv_num();
                t += cur_val();
                set_math_char(t);
            } else {
                scan_delimiter_int();
                let v = cur_val() / 4096;
                set_math_char(set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256));
            }
        }

        (MMODE_MC, MATH_COMP_MC) => {
            let n = crate::mem::new_noad_global();
            set_mem_b32s1(cur_list_tail(), n);
            set_cur_list_tail(mem_b32s1(cur_list_tail())); // LLIST_link(tail)
            set_mem_b16s1(cur_list_tail(), cur_chr() as u16);
            scan_math(cur_list_tail() + 1);
        }

        (MMODE_MC, LIMIT_SWITCH_MC) => math_limit_switch_global(),
        (MMODE_MC, RADICAL_MC) => math_radical_global(),
        (MMODE_MC, ACCENT_MC) | (MMODE_MC, MATH_ACCENT_MC) => math_ac_global(),

        (MMODE_MC, VCENTER_MC) => {
            scan_spec(VCENTER_GROUP_MC, false);
            normal_paragraph();
            crate::mem::push_nest();
            set_cur_list_mode(-1);
            set_cur_list_aux_b32s1(IGNORE_DEPTH_MC);
            if crate::globals::insert_src_special_every_vbox() {
                insert_src_special();
            }
            let every_vbox = eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_VBOX_MC);
            if every_vbox != TEX_NULL {
                crate::input::begin_token_list(every_vbox, EVERY_VBOX_TEXT_MC);
            }
        }

        (MMODE_MC, MATH_STYLE_MC) => {
            let s = crate::mem::new_style_global(cur_chr() as i16);
            set_mem_b32s1(cur_list_tail(), s);
            set_cur_list_tail(mem_b32s1(cur_list_tail())); // LLIST_link(tail)
        }

        (MMODE_MC, NON_SCRIPT_MC) => {
            let g = crate::mem::new_glue_global(0);
            set_mem_b32s1(cur_list_tail(), g);
            set_cur_list_tail(mem_b32s1(cur_list_tail())); // LLIST_link(tail)
            set_mem_b16s0(cur_list_tail(), COND_MATH_GLUE_MC);
        }

        (MMODE_MC, MATH_CHOICE_MC) => append_choices_global(),
        (MMODE_MC, SUB_MARK_MC) | (MMODE_MC, SUP_MARK_MC) => sub_sup_global(),
        (MMODE_MC, ABOVE_MC) => math_fraction_global(),
        (MMODE_MC, LEFT_RIGHT_MC) => math_left_right_global(),

        (MMODE_MC, MATH_SHIFT_MC) => {
            if cur_group() == MATH_SHIFT_GROUP_MC {
                after_math();
            } else {
                off_save();
            }
        }

        _ => {}
    }
}

// translated from xetex-xetex0.c:6857 (main_control IGNORE_SPACES body; caller does goto reswitch)
#[no_mangle]
pub unsafe extern "C" fn main_control_ignore_spaces() {
    const SPACER_MC: u8 = 10;
    const NORMAL_SCANNER_MC: u8 = 0;

    use crate::globals::{
        cur_chr, cur_cmd, cur_cs, eqtb_b16s1, eqtb_b32s1, hash_s1, scanner_status, set_cur_chr,
        set_cur_cmd, set_cur_cs, set_cur_tok, set_scanner_status,
    };

    if cur_chr() == 0 {
        loop {
            get_x_token();
            if cur_cmd() != SPACER_MC {
                break;
            }
        }
    } else {
        let saved = scanner_status();
        set_scanner_status(NORMAL_SCANNER_MC);
        get_next();
        set_scanner_status(saved);

        if cur_cs() < HASH_BASE {
            set_cur_cs(crate::hash::prim_lookup(cur_cs() - SINGLE_BASE));
        } else {
            set_cur_cs(crate::hash::prim_lookup(hash_s1(cur_cs())));
        }

        if cur_cs() != UNDEFINED_PRIMITIVE {
            set_cur_cmd(eqtb_b16s1(PRIM_EQTB_BASE + cur_cs()) as u8);
            set_cur_chr(eqtb_b32s1(PRIM_EQTB_BASE + cur_cs()));
            set_cur_tok(CS_TOKEN_FLAG + PRIM_EQTB_BASE + cur_cs());
        }
    }
    // Caller must do: goto reswitch
}

// translated from xetex-xetex0.c:6804 (main_control HMODE inter-char pre-dispatch check)
// Returns true if the caller should goto big_switch (inter-char token was injected).
#[no_mangle]
pub unsafe extern "C" fn main_control_hmode_inter_char_check() -> bool {
    const HMODE_MC: i16 = 104;
    const CHAR_NUM_MC: u8 = 16;
    const INT_PAR_XETEX_INTER_CHAR_TOKENS_MC: i32 = 73; // xetex_format.h:108
    const INTER_CHAR_TEXT_MC: u16 = 17; // xetex-constants.h:216

    use crate::globals::{
        cur_chr, cur_cmd, cur_cs, cur_list_mode, cur_ptr, eqtb_b32s1, mem_b32s1, set_cur_cmd,
        set_cur_tok, set_prev_class,
    };
    use crate::mem::TEX_NULL;

    if cur_list_mode().abs() != HMODE_MC {
        return false;
    }
    if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS_MC) <= 0 {
        return false;
    }
    let sc = crate::globals::space_class();
    let pc = crate::globals::prev_class();
    if sc == CHAR_CLASS_LIMIT || pc == CHAR_CLASS_LIMIT - 1 {
        return false;
    }

    set_prev_class(CHAR_CLASS_LIMIT - 1);
    crate::save::find_sa_element_global(
        INTER_CHAR_VAL as u16,
        sc * CHAR_CLASS_LIMIT + CHAR_CLASS_LIMIT - 1,
        false,
    );
    if cur_ptr() == TEX_NULL || mem_b32s1(cur_ptr() + 1) == TEX_NULL {
        return false;
    }

    if cur_cs() == 0 {
        if cur_cmd() == CHAR_NUM_MC {
            set_cur_cmd(OTHER_CHAR);
        }
        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
    } else {
        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
    }
    back_input();
    crate::input::begin_token_list(mem_b32s1(cur_ptr() + 1), INTER_CHAR_TEXT_MC);
    true
}

// translated from xetex-xetex0.c:6833 (main_control spacer/ex_space cases)
#[no_mangle]
pub unsafe extern "C" fn main_control_spacer_cases() {
    const HMODE_MC: u16 = 104; // xetex_format.h:369
    const MMODE_MC: u16 = 207; // xetex_format.h:370
    const SPACER_MC: u8 = 10; // xetex_format.h:242
    const EX_SPACE_MC: u8 = 64; // xetex_format.h:302

    match (crate::globals::cur_list_mode().unsigned_abs(), cur_cmd()) {
        (HMODE_MC, SPACER_MC) => {
            if crate::globals::cur_list_aux_b32s0() == 1000 {
                main_control_append_normal_space();
            } else {
                app_space();
            }
        }
        (HMODE_MC, EX_SPACE_MC) | (MMODE_MC, EX_SPACE_MC) => {
            main_control_append_normal_space();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// main_control_main_loop
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:7030 (main_loop through main_loop_move_lig)
// Returns true → caller must goto big_switch; false → caller must goto reswitch.
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
#[no_mangle]
pub unsafe extern "C" fn main_control_main_loop() -> bool {
    use crate::globals::{
        avail, bchar, bchar_label_entry, cancel_boundary, char_base_entry, cur_chr, cur_cmd,
        cur_input, cur_l, cur_list_aux_b32s0, cur_list_aux_b32s1, cur_list_head, cur_list_mode,
        cur_list_tail, cur_ptr, cur_q, cur_r, eqtb_b32s1, font_area_entry, font_bc_entry,
        font_bchar_entry, font_ec_entry, font_false_bchar_entry, font_glue, font_info_b16,
        font_mapping_ptr, hyphen_char_entry, ins_disc, insert_src_special_auto, is_hyph, lft_hit,
        lig_kern_base_entry, lig_stack, ligature_present, main_f, main_h, main_p, main_pp,
        main_ppp, mapped_text_entry, mem_as_u16_ptr_mut, mem_b16s0, mem_b16s1, mem_b16s2,
        mem_b32s1, mem_node_addr, native_len, native_text, native_text_size, prev_class, rt_hit,
        save_native_len, set_avail, set_bchar, set_cancel_boundary, set_cur_chr, set_cur_cmd,
        set_cur_input_index, set_cur_l, set_cur_list_aux_b32s0, set_cur_list_tail, set_cur_ptr,
        set_cur_q, set_cur_r, set_cur_tok, set_false_bchar, set_ins_disc, set_is_hyph, set_lft_hit,
        set_lig_stack, set_ligature_present, set_main_f, set_main_h, set_main_p, set_main_pp,
        set_main_ppp, set_main_s, set_mem_b16s0, set_mem_b16s1, set_mem_b32s1, set_native_len,
        set_prev_class, set_rt_hit, set_save_native_len, set_space_class, space_class,
    };
    use crate::input::{back_input, begin_token_list};
    use crate::mem::{
        free_node_global, get_avail_global, new_disc_global, new_kern_global, new_ligature_global,
        TEX_NULL,
    };
    use crate::save::find_sa_element_global;

    // Constants
    const INT_PAR_LANGUAGE: i32 = 50;
    const INT_PAR_TRACING_LOST_CHARS: i32 = 35;
    const INT_PAR_XETEX_INTER_CHAR_TOKENS: i32 = 73;
    const INT_PAR_XETEX_INTERWORD_SPACE_SHAPING: i32 = 78;
    const INT_PAR_XETEX_DASH_BREAK: i32 = 70;
    const INT_PAR_XETEX_USE_GLYPH_METRICS: i32 = 72;
    const CHAR_CLASS_LIMIT: i32 = 4096;
    const MAX_CHAR_VAL: i32 = 0x200000;
    const INTER_CHAR_TEXT: u16 = 17;
    const BACKED_UP_CHAR: u16 = 4;
    const TOKEN_LIST: u16 = 0;
    const INTER_CHAR_VAL_U: u16 = 6;
    const LIG_TAG: u16 = 1;
    const NON_ADDRESS: i32 = 0;
    const NATIVE_NODE_SIZE: i32 = 6;
    const SPACE_ADJUSTMENT: u16 = 3;
    const WHATSIT_NODE: u16 = 8;
    const NATIVE_WORD_NODE: u16 = 40;
    const NATIVE_WORD_NODE_AT: u16 = 41;
    const INS_NODE: u16 = 3;
    const MARK_NODE: u16 = 4;
    const ADJUST_NODE: u16 = 5;
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const CHAR_GIVEN: u8 = 68;
    const CHAR_NUM: u8 = 16;

    #[inline(always)]
    unsafe fn is_char_node(p: i32) -> bool {
        p >= crate::globals::hi_mem_min()
    }

    // === main_loop: entry point ===
    // SyncTeX: append special at start of new paragraph
    if cur_list_head() == cur_list_tail() && cur_list_mode() > 0 && insert_src_special_auto() {
        append_src_special();
    }
    set_prev_class(CHAR_CLASS_LIMIT - 1);

    let mf = eqtb_b32s1(CUR_FONT_LOC);
    let fa = font_area_entry(mf);

    // =========================================================================
    // AAT / OTGR native font path (collect_native loop)
    // =========================================================================
    if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
        if cur_list_mode() > 0 && eqtb_b32s1(INT_BASE + INT_PAR_LANGUAGE) != cur_list_aux_b32s1() {
            fix_language();
        }
        set_main_h(0);
        set_main_f(mf);
        set_native_len(0);

        // collect_native loop: accumulate consecutive chars into native_text buffer
        'collect: loop {
            // Update space factor from SF_CODE(cur_chr)
            let sf = eqtb_b32s1(SF_CODE_BASE + cur_chr());
            let ms = sf % 65536;
            set_main_s(ms);
            if ms == 1000 {
                set_cur_list_aux_b32s0(1000);
            } else if ms < 1000 {
                if ms > 0 {
                    set_cur_list_aux_b32s0(ms);
                }
            } else if cur_list_aux_b32s0() < 1000 {
                set_cur_list_aux_b32s0(1000);
            } else {
                set_cur_list_aux_b32s0(ms);
            }

            set_cur_ptr(TEX_NULL);
            set_space_class(sf / 65536);
            let sc = space_class();

            // Inter-char check (collect_native variant)
            if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0 && sc != CHAR_CLASS_LIMIT
            {
                if prev_class() == CHAR_CLASS_LIMIT - 1 {
                    if cur_input().state != TOKEN_LIST || cur_input().index != BACKED_UP_CHAR {
                        find_sa_element_global(
                            INTER_CHAR_VAL_U,
                            (CHAR_CLASS_LIMIT - 1) * CHAR_CLASS_LIMIT + sc,
                            false,
                        );
                        if cur_ptr() != TEX_NULL && mem_b32s1(cur_ptr() + 1) != TEX_NULL {
                            if cur_cmd() != LETTER {
                                set_cur_cmd(OTHER_CHAR);
                            }
                            set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
                            back_input();
                            set_cur_input_index(BACKED_UP_CHAR);
                            begin_token_list(mem_b32s1(cur_ptr() + 1), INTER_CHAR_TEXT);
                            return true; // goto big_switch
                        }
                    }
                } else {
                    find_sa_element_global(
                        INTER_CHAR_VAL_U,
                        prev_class() * CHAR_CLASS_LIMIT + sc,
                        false,
                    );
                    if cur_ptr() != TEX_NULL && mem_b32s1(cur_ptr() + 1) != TEX_NULL {
                        if cur_cmd() != LETTER {
                            set_cur_cmd(OTHER_CHAR);
                        }
                        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
                        back_input();
                        set_cur_input_index(BACKED_UP_CHAR);
                        begin_token_list(mem_b32s1(cur_ptr() + 1), INTER_CHAR_TEXT);
                        set_prev_class(CHAR_CLASS_LIMIT - 1);
                        break 'collect; // goto collected
                    }
                }
                set_prev_class(sc);
            }

            // Append cur_chr to native_text buffer
            let ch = cur_chr();
            if ch > 65535 {
                while native_text_size() <= native_len() + 2 {
                    crate::state::with_state_mut(|s| {
                        let size = s.scanner.native_text_size + 128;
                        s.scanner.native_text_size = size;
                        s.scanner.native_text.resize(size as usize, 0);
                    });
                }
                *native_text().add(native_len() as usize) = ((ch - 65536) / 1024 + 0xD800) as u16;
                set_native_len(native_len() + 1);
                *native_text().add(native_len() as usize) = ((ch - 65536) % 1024 + 0xDC00) as u16;
                set_native_len(native_len() + 1);
            } else {
                while native_text_size() <= native_len() + 1 {
                    crate::state::with_state_mut(|s| {
                        let size = s.scanner.native_text_size + 128;
                        s.scanner.native_text_size = size;
                        s.scanner.native_text.resize(size as usize, 0);
                    });
                }
                *native_text().add(native_len() as usize) = ch as u16;
                set_native_len(native_len() + 1);
            }

            // Check if this char is a hyphen/dash
            let is_h = ch == hyphen_char_entry(main_f())
                || (eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) > 0
                    && (ch == 8212 || ch == 8211));
            set_is_hyph(is_h);
            if main_h() == 0 && is_h {
                set_main_h(native_len());
            }

            // Look ahead for more chars (same font)
            get_next();
            if cur_cmd() == LETTER || cur_cmd() == OTHER_CHAR || cur_cmd() == CHAR_GIVEN {
                continue 'collect;
            }
            x_token();
            if cur_cmd() == LETTER || cur_cmd() == OTHER_CHAR || cur_cmd() == CHAR_GIVEN {
                continue 'collect;
            }
            if cur_cmd() == CHAR_NUM {
                scan_usv_num();
                set_cur_chr(cur_val());
                continue 'collect;
            }

            // End of char run: inter-char suffix check
            if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0
                && sc != CHAR_CLASS_LIMIT
                && prev_class() != CHAR_CLASS_LIMIT - 1
            {
                set_prev_class(CHAR_CLASS_LIMIT - 1);
                find_sa_element_global(
                    INTER_CHAR_VAL_U,
                    sc * CHAR_CLASS_LIMIT + CHAR_CLASS_LIMIT - 1,
                    false,
                );
                if cur_ptr() != TEX_NULL && mem_b32s1(cur_ptr() + 1) != TEX_NULL {
                    if cur_cs() == 0 {
                        if cur_cmd() == CHAR_NUM {
                            set_cur_cmd(OTHER_CHAR);
                        }
                        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
                    } else {
                        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
                    }
                    back_input();
                    begin_token_list(mem_b32s1(cur_ptr() + 1), INTER_CHAR_TEXT);
                    break 'collect; // goto collected (then process mapping)
                }
            }
            break 'collect; // natural end of collect_native
        } // end 'collect

        // collected:
        // Apply font mapping if present
        let fm = font_mapping_ptr(main_f());
        if !fm.is_null() {
            let mk = apply_mapping(fm, native_text(), native_len());
            set_native_len(0);
            while native_text_size() <= native_len() + mk {
                crate::state::with_state_mut(|s| {
                    let size = s.scanner.native_text_size + 128;
                    s.scanner.native_text_size = size;
                    s.scanner.native_text.resize(size as usize, 0);
                });
            }
            set_main_h(0);
            for mp in 0..mk {
                let c = mapped_text_entry(mp);
                *native_text().add(native_len() as usize) = c;
                set_native_len(native_len() + 1);
                if main_h() == 0
                    && (c == hyphen_char_entry(main_f()) as u16
                        || (eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) > 0
                            && (c == 8212 || c == 8211)))
                {
                    set_main_h(native_len());
                }
            }
        }

        // Tracing lost chars
        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_LOST_CHARS) > 0 {
            let mut tp = 0i32;
            while tp < native_len() {
                let mut mc = *native_text().add(tp as usize) as i32;
                tp += 1;
                if (0xD800..0xDC00).contains(&mc) {
                    mc = 65536 + (mc - 0xD800) * 1024;
                    mc += *native_text().add(tp as usize) as i32 - 0xDC00;
                    tp += 1;
                }
                if map_char_to_glyph(main_f(), mc) == 0 {
                    char_warning(main_f(), mc);
                }
            }
        }

        set_main_p(cur_list_tail());
        let mk = native_len();
        set_main_pp(cur_list_tail());

        if cur_list_mode() == 104
        /* HMODE */
        {
            set_main_ppp(cur_list_head());
            // Advance main_ppp to find node just before main_pp
            while main_ppp() != main_pp() && mem_b32s1(main_ppp()) != main_pp() {
                if !is_char_node(main_ppp()) && mem_b16s1(main_ppp()) == DISC_NODE {
                    let tp = main_ppp();
                    for _ in 1..=mem_b16s0(tp) as i32 {
                        set_main_ppp(mem_b32s1(main_ppp()));
                    }
                }
                if main_ppp() != main_pp() {
                    set_main_ppp(mem_b32s1(main_ppp()));
                }
            }

            // C uses `main_k` as the running remaining-length and `temp_ptr` as
            // the offset into native_text of the current segment; both mutate
            // across the do-while. `main_h` (global) is the next hyphen offset.
            let mut temp_ptr = 0i32;
            let mut main_k = mk;
            loop {
                if main_h() == 0 {
                    set_main_h(main_k);
                }

                let tail_is_native = main_pp() != TEX_NULL
                    && !is_char_node(main_pp())
                    && mem_b16s1(main_pp()) == WHATSIT_NODE
                    && (mem_b16s0(main_pp()) == NATIVE_WORD_NODE
                        || mem_b16s0(main_pp()) == NATIVE_WORD_NODE_AT)
                    && mem_b16s2(main_pp() + 4) == main_f() as u16
                    && main_ppp() != main_pp()
                    && !is_char_node(main_ppp())
                    && mem_b16s1(main_ppp()) != DISC_NODE;

                if tail_is_native {
                    let old_len = mem_b16s1(main_pp() + 4) as i32;
                    main_k = main_h() + old_len;
                    while native_text_size() <= native_len() + main_k {
                        crate::state::with_state_mut(|s| {
                            let size = s.scanner.native_text_size + 128;
                            s.scanner.native_text_size = size;
                            s.scanner.native_text.resize(size as usize, 0);
                        });
                    }
                    set_save_native_len(native_len());
                    for i in 0..old_len {
                        *native_text().add(native_len() as usize) =
                            *mem_as_u16_ptr_mut(main_pp() + NATIVE_NODE_SIZE).add(i as usize);
                        set_native_len(native_len() + 1);
                    }
                    for i in 0..main_h() {
                        *native_text().add(native_len() as usize) =
                            *native_text().add((temp_ptr + i) as usize);
                        set_native_len(native_len() + 1);
                    }
                    do_locale_linebreaks(save_native_len(), main_k);
                    set_native_len(save_native_len());
                    main_k = native_len() - main_h() - temp_ptr;
                    temp_ptr = main_h();
                    set_main_h(0);
                    while main_h() < main_k
                        && (*native_text().add((temp_ptr + main_h()) as usize) as i32)
                            != hyphen_char_entry(main_f())
                        && (eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) <= 0
                            || ((*native_text().add((temp_ptr + main_h()) as usize) as i32)
                                != 8212
                                && (*native_text().add((temp_ptr + main_h()) as usize) as i32)
                                    != 8211))
                    {
                        set_main_h(main_h() + 1);
                    }
                    if main_h() < main_k {
                        set_main_h(main_h() + 1);
                    }
                    set_mem_b32s1(main_ppp(), mem_b32s1(main_pp()));
                    set_mem_b32s1(main_pp(), TEX_NULL);
                    flush_node_list_global(main_pp());
                    set_main_pp(cur_list_tail());
                    while mem_b32s1(main_ppp()) != main_pp() {
                        set_main_ppp(mem_b32s1(main_ppp()));
                    }
                } else {
                    do_locale_linebreaks(temp_ptr, main_h());
                    temp_ptr += main_h();
                    main_k -= main_h();
                    set_main_h(0);
                    while main_h() < main_k
                        && (*native_text().add((temp_ptr + main_h()) as usize) as i32)
                            != hyphen_char_entry(main_f())
                        && (eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) <= 0
                            || ((*native_text().add((temp_ptr + main_h()) as usize) as i32)
                                != 8212
                                && (*native_text().add((temp_ptr + main_h()) as usize) as i32)
                                    != 8211))
                    {
                        set_main_h(main_h() + 1);
                    }
                    if main_h() < main_k {
                        set_main_h(main_h() + 1);
                    }
                }

                if main_k > 0 || is_hyph() {
                    set_mem_b32s1(cur_list_tail(), new_disc_global());
                    set_cur_list_tail(mem_b32s1(cur_list_tail()));
                    set_main_pp(cur_list_tail());
                }

                if main_k == 0 {
                    break;
                }
            }
        } else {
            // MMODE (or not HMODE): simple native word node creation
            set_main_ppp(cur_list_head());
            while main_ppp() != main_pp() && mem_b32s1(main_ppp()) != main_pp() {
                if !is_char_node(main_ppp()) && mem_b16s1(main_ppp()) == DISC_NODE {
                    let tp = main_ppp();
                    for _ in 1..=mem_b16s0(tp) as i32 {
                        set_main_ppp(mem_b32s1(main_ppp()));
                    }
                }
                if main_ppp() != main_pp() {
                    set_main_ppp(mem_b32s1(main_ppp()));
                }
            }

            let tail_is_native = main_pp() != TEX_NULL
                && !is_char_node(main_pp())
                && mem_b16s1(main_pp()) == WHATSIT_NODE
                && (mem_b16s0(main_pp()) == NATIVE_WORD_NODE
                    || mem_b16s0(main_pp()) == NATIVE_WORD_NODE_AT)
                && mem_b16s2(main_pp() + 4) == mf as u16
                && main_ppp() != main_pp()
                && !is_char_node(main_ppp())
                && mem_b16s1(main_ppp()) != DISC_NODE;

            if tail_is_native {
                let old_len = mem_b16s1(main_pp() + 4) as i32;
                let new_node = new_native_word_node_global(mf, mk + old_len);
                set_mem_b32s1(main_pp(), new_node);
                set_cur_list_tail(mem_b32s1(main_pp()));
                for i in 0..old_len {
                    *mem_as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE).add(i as usize) =
                        *mem_as_u16_ptr_mut(main_pp() + NATIVE_NODE_SIZE).add(i as usize);
                }
                for i in 0..mk {
                    *mem_as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE)
                        .add((i + old_len) as usize) = *native_text().add(i as usize);
                }
                measure_native_node(
                    mem_node_addr(cur_list_tail()),
                    (eqtb_b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0) as libc::c_int,
                );
                let mut prev = cur_list_head();
                if prev != main_pp() {
                    while mem_b32s1(prev) != main_pp() {
                        prev = mem_b32s1(prev);
                    }
                }
                set_mem_b32s1(prev, mem_b32s1(main_pp()));
                set_mem_b32s1(main_pp(), TEX_NULL);
                free_node_global(main_pp(), mem_b16s3(main_pp() + 4) as i32);
            } else {
                let new_node = new_native_word_node_global(mf, mk);
                set_mem_b32s1(main_pp(), new_node);
                set_cur_list_tail(mem_b32s1(main_pp()));
                for i in 0..mk {
                    *mem_as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE).add(i as usize) =
                        *native_text().add(i as usize);
                }
                measure_native_node(
                    mem_node_addr(cur_list_tail()),
                    (eqtb_b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0) as libc::c_int,
                );
            }
        }

        // Interword space shaping (both HMODE and MMODE)
        if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_INTERWORD_SPACE_SHAPING) > 0 {
            let mut p = cur_list_head();
            let mut pp = TEX_NULL;
            while p != cur_list_tail() {
                if p != TEX_NULL
                    && !is_char_node(p)
                    && mem_b16s1(p) == WHATSIT_NODE
                    && (mem_b16s0(p) == NATIVE_WORD_NODE || mem_b16s0(p) == NATIVE_WORD_NODE_AT)
                {
                    pp = p;
                }
                p = mem_b32s1(p);
            }
            if pp != TEX_NULL && mem_b16s2(pp + 4) == mf as u16 {
                let mut p2 = mem_b32s1(pp);
                while !is_char_node(p2)
                    && (mem_b16s1(p2) == PENALTY_NODE
                        || mem_b16s1(p2) == INS_NODE
                        || mem_b16s1(p2) == MARK_NODE
                        || mem_b16s1(p2) == ADJUST_NODE
                        || (mem_b16s1(p2) == WHATSIT_NODE && mem_b16s0(p2) <= 4))
                {
                    p2 = mem_b32s1(p2);
                }
                if !is_char_node(p2) && mem_b16s1(p2) == GLUE_NODE {
                    let p3 = mem_b32s1(p2);
                    let mut p3m = p3;
                    while !is_char_node(p3m)
                        && (mem_b16s1(p3m) == PENALTY_NODE
                            || mem_b16s1(p3m) == INS_NODE
                            || mem_b16s1(p3m) == MARK_NODE
                            || mem_b16s1(p3m) == ADJUST_NODE
                            || (mem_b16s1(p3m) == WHATSIT_NODE && mem_b16s0(p3m) <= 4))
                    {
                        p3m = mem_b32s1(p3m);
                    }
                    if p3m == cur_list_tail() {
                        let pp_len = mem_b16s1(pp + 4) as i32;
                        let tail_len = mem_b16s1(cur_list_tail() + 4) as i32;
                        let tmp = new_native_word_node_global(mf, pp_len + 1 + tail_len);
                        let mut ki = 0i32;
                        for t in 0..pp_len {
                            *mem_as_u16_ptr_mut(tmp + NATIVE_NODE_SIZE).add(ki as usize) =
                                *mem_as_u16_ptr_mut(pp + NATIVE_NODE_SIZE).add(t as usize);
                            ki += 1;
                        }
                        *mem_as_u16_ptr_mut(tmp + NATIVE_NODE_SIZE).add(ki as usize) = b' ' as u16;
                        ki += 1;
                        for t in 0..tail_len {
                            *mem_as_u16_ptr_mut(tmp + NATIVE_NODE_SIZE).add(ki as usize) =
                                *mem_as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE)
                                    .add(t as usize);
                            ki += 1;
                        }
                        measure_native_node(
                            mem_node_addr(tmp),
                            (eqtb_b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0)
                                as libc::c_int,
                        );
                        let t_val =
                            mem_b32s1(tmp + 1) - mem_b32s1(pp + 1) - mem_b32s1(cur_list_tail() + 1);
                        free_node_global(tmp, mem_b16s3(tmp + 4) as i32);
                        if t_val != mem_b32s1(font_glue(mf) + 1) {
                            let kn = new_kern_global(t_val - mem_b32s1(font_glue(mf) + 1));
                            set_mem_b16s0(kn, SPACE_ADJUSTMENT);
                            set_mem_b32s1(kn, mem_b32s1(p2));
                            set_mem_b32s1(p2, kn);
                        }
                    }
                }
            }
        }

        // Exit: cur_ptr set iff inter-char token injected
        return cur_ptr() != TEX_NULL;
    }

    // =========================================================================
    // Regular TFM font path
    // =========================================================================

    // Update space factor from SF_CODE(cur_chr)
    let sf = eqtb_b32s1(SF_CODE_BASE + cur_chr());
    let ms = sf % 65536;
    set_main_s(ms);
    if ms == 1000 {
        set_cur_list_aux_b32s0(1000);
    } else if ms < 1000 {
        if ms > 0 {
            set_cur_list_aux_b32s0(ms);
        }
    } else if cur_list_aux_b32s0() < 1000 {
        set_cur_list_aux_b32s0(1000);
    } else {
        set_cur_list_aux_b32s0(ms);
    }

    set_cur_ptr(TEX_NULL);
    set_space_class(sf / 65536);
    let sc = space_class();

    // Inter-char check (regular font entry)
    if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0 && sc != CHAR_CLASS_LIMIT {
        if prev_class() == CHAR_CLASS_LIMIT - 1 {
            if cur_input().state != TOKEN_LIST || cur_input().index != BACKED_UP_CHAR {
                find_sa_element_global(
                    INTER_CHAR_VAL_U,
                    (CHAR_CLASS_LIMIT - 1) * CHAR_CLASS_LIMIT + sc,
                    false,
                );
                if cur_ptr() != TEX_NULL && mem_b32s1(cur_ptr() + 1) != TEX_NULL {
                    if cur_cmd() != LETTER {
                        set_cur_cmd(OTHER_CHAR);
                    }
                    set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
                    back_input();
                    set_cur_input_index(BACKED_UP_CHAR);
                    begin_token_list(mem_b32s1(cur_ptr() + 1), INTER_CHAR_TEXT);
                    return true; // goto big_switch
                }
            }
        } else {
            find_sa_element_global(
                INTER_CHAR_VAL_U,
                prev_class() * CHAR_CLASS_LIMIT + sc,
                false,
            );
            if cur_ptr() != TEX_NULL && mem_b32s1(cur_ptr() + 1) != TEX_NULL {
                if cur_cmd() != LETTER {
                    set_cur_cmd(OTHER_CHAR);
                }
                set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
                back_input();
                set_cur_input_index(BACKED_UP_CHAR);
                begin_token_list(mem_b32s1(cur_ptr() + 1), INTER_CHAR_TEXT);
                return true; // goto big_switch
            }
        }
        set_prev_class(sc);
    }

    // Set up font state
    set_main_f(mf);
    set_bchar(font_bchar_entry(mf));
    set_false_bchar(font_false_bchar_entry(mf));

    if cur_list_mode() > 0 && eqtb_b32s1(INT_BASE + INT_PAR_LANGUAGE) != cur_list_aux_b32s1() {
        fix_language();
    }

    // Allocate lig_stack node from avail list
    let ls = if avail() == TEX_NULL {
        get_avail_global()
    } else {
        let a = avail();
        set_avail(mem_b32s1(a));
        set_mem_b32s1(a, TEX_NULL);
        a
    };
    set_lig_stack(ls);
    set_mem_b16s1(ls, mf as u16);
    set_cur_l(cur_chr());
    set_mem_b16s0(ls, cur_l() as u16);
    set_cur_q(cur_list_tail());

    // Determine entry point: NON_ADDRESS → move_2; else → lig_loop_1 (bchar)
    let bchar_lbl = if cancel_boundary() {
        set_cancel_boundary(false);
        NON_ADDRESS
    } else {
        bchar_label_entry(mf)
    };

    // `enter_lig1_k`: if Some(k), enter lig loop at _1 with that k; else enter at move_2
    let mut enter_lig1_k: Option<i32> = if bchar_lbl != NON_ADDRESS {
        set_cur_r(cur_l());
        set_cur_l(TOO_BIG_CHAR);
        Some(bchar_lbl)
    } else {
        None
    };

    // main_i: char info for cur_l (set at move_2 or when cur_l changes in lig loop)
    let mut main_i = crate::mem_types::B16x4 {
        s0: 0,
        s1: 0,
        s2: 0,
        s3: 0,
    };

    // -------------------------------------------------------------------------
    // Outer char-processing loop.
    // Each iteration: run the lig/kern table, then advance to the next char.
    // -------------------------------------------------------------------------
    'char_proc: loop {
        // Determine how to enter the lig/kern loop this iteration:
        // - enter_lig1_k = Some(k): start at lig_loop_1 (skips the precondition check)
        // - enter_lig1_k = None: start at lig_loop top (checks LIG_TAG and cur_r)

        // "lig_result": 0=wrapup, 1=goto_move (skip wrapup), 2=goto_move1 (skip wrapup+move-check)
        let lig_result: u8;

        if let Some(init_k) = enter_lig1_k.take() {
            // Enter at main_lig_loop_1 directly (bchar case or after lig advance)
            lig_result = lig_inner_loop(init_k, &mut main_i, &mut enter_lig1_k);
        } else {
            // Enter at main_lig_loop top: check preconditions
            if (main_i.s1 % 4) as u8 != LIG_TAG as u8 || cur_r() == TOO_BIG_CHAR {
                lig_result = 0; // goto main_loop_wrapup
            } else {
                let init_k = lig_kern_base_entry(main_f()) + main_i.s0 as i32;
                let init_j = font_info_b16(init_k);
                let real_k = if init_j.s3 > 128 {
                    lig_kern_base_entry(main_f())
                        + 256 * init_j.s1 as i32
                        + init_j.s0 as i32
                        + 32768
                        - 256 * 128
                } else {
                    init_k
                };
                lig_result = lig_inner_loop(real_k, &mut main_i, &mut enter_lig1_k);
            }
        }

        // -----------------------------------------------------------------------
        // Handle lig_result: wrapup, move, or move1
        // -----------------------------------------------------------------------
        // 0xFF: lig type 1/2/3/7 advanced cur_l; re-enter lig_loop top (no wrapup/move)
        if lig_result == 0xFF {
            continue 'char_proc;
        }

        if lig_result == 0 {
            // main_loop_wrapup: only when lig_result==0 (result 1=goto_move, 2=goto_move1 both skip wrapup)
            if cur_l() < TOO_BIG_CHAR {
                if mem_b32s1(cur_q()) > TEX_NULL
                    && mem_b16s0(cur_list_tail()) == hyphen_char_entry(main_f()) as u16
                {
                    set_ins_disc(true);
                }
                if ligature_present() {
                    let mp = new_ligature_global(main_f(), cur_l() as u16, mem_b32s1(cur_q()));
                    if lft_hit() {
                        set_mem_b16s0(mp, 2);
                        set_lft_hit(false);
                    }
                    if rt_hit() && lig_stack() == TEX_NULL {
                        set_mem_b16s0(mp, mem_b16s0(mp) + 1);
                        set_rt_hit(false);
                    }
                    set_mem_b32s1(cur_q(), mp);
                    set_cur_list_tail(mp);
                    set_ligature_present(false);
                }
                if ins_disc() {
                    set_ins_disc(false);
                    if cur_list_mode() > 0 {
                        set_mem_b32s1(cur_list_tail(), new_disc_global());
                        set_cur_list_tail(mem_b32s1(cur_list_tail()));
                    }
                }
            }
        }
        // If lig_result == 1: skip wrapup, go directly to move

        // main_loop_move: check if lig_stack is exhausted
        // (for lig_result==2 / move1: skip this check, cur_l already set)
        if lig_result != 2 {
            if lig_stack() == TEX_NULL {
                return false; // goto reswitch
            }
            set_cur_q(cur_list_tail());
            set_cur_l(mem_b16s0(lig_stack()) as i32);
        }

        // main_loop_move_1: is lig_stack a char node or a lig item?
        if !is_char_node(lig_stack()) {
            // main_loop_move_lig: handle lig item in lig_stack
            set_main_p(mem_b32s1(lig_stack() + 1));
            if main_p() > TEX_NULL {
                set_mem_b32s1(cur_list_tail(), main_p());
                set_cur_list_tail(mem_b32s1(cur_list_tail()));
            }
            let temp = lig_stack();
            set_lig_stack(mem_b32s1(temp));
            free_node_global(temp, SMALL_NODE_SIZE);
            main_i = font_info_b16(
                char_base_entry(main_f()) + effective_char_global(true, main_f(), cur_l() as u16),
            );
            set_ligature_present(true);

            if lig_stack() == TEX_NULL {
                if main_p() > TEX_NULL {
                    // goto main_loop_lookahead (enter with main_i set, skip move_2)
                    // We do the lookahead inline here:
                    if !do_lookahead(&mut main_i, &mut enter_lig1_k, mf) {
                        return true; // inter-char token injected → big_switch
                    }
                    continue 'char_proc;
                } else {
                    set_cur_r(bchar());
                }
            } else {
                set_cur_r(mem_b16s0(lig_stack()) as i32);
            }
            // goto main_lig_loop (outer top) — continue 'char_proc
            continue 'char_proc;
        }

        // main_loop_move_2: validate and append the char node (lig_stack is char)
        let eff = effective_char_global(false, main_f(), cur_chr() as u16);
        if eff > font_ec_entry(main_f()) as i32 || eff < font_bc_entry(main_f()) as i32 {
            char_warning(main_f(), cur_chr());
            set_mem_b32s1(lig_stack(), avail());
            set_avail(lig_stack());
            return true; // goto big_switch
        }
        main_i = effective_char_info_global(main_f(), cur_l() as u16);
        if main_i.s3 == 0 {
            char_warning(main_f(), cur_chr());
            set_mem_b32s1(lig_stack(), avail());
            set_avail(lig_stack());
            return true; // goto big_switch
        }
        set_mem_b32s1(cur_list_tail(), lig_stack());
        set_cur_list_tail(lig_stack());

        // main_loop_lookahead: get next token, set up lig_stack for it
        if !do_lookahead(&mut main_i, &mut enter_lig1_k, mf) {
            return true; // inter-char token injected → big_switch
        }
        // continue 'char_proc → back to top of lig loop
    } // end 'char_proc
}

// Inner lig/kern table scan (main_lig_loop_1 / main_lig_loop_2).
// Returns: 0=wrapup, 1=goto_move (skip wrapup), 2=goto_move1 (lig type 0/4 with stack)
// Also sets enter_lig1_k if we need to re-enter lig_loop_1 (bchar advance).
#[inline(always)]
unsafe fn lig_inner_loop(
    init_k: i32,
    main_i: &mut crate::mem_types::B16x4,
    enter_lig1_k: &mut Option<i32>,
) -> u8 {
    use crate::globals::{
        bchar_label_entry, char_base_entry, cur_l, cur_list_mode, cur_list_tail, cur_q, cur_r,
        font_info_b16, hyphen_char_entry, ins_disc, kern_base_entry, lft_hit, lig_stack,
        ligature_present, main_f, mem_b16s0, mem_b32s1, set_bchar, set_cur_l, set_cur_list_tail,
        set_cur_q, set_cur_r, set_ins_disc, set_lft_hit, set_lig_stack, set_ligature_present,
        set_mem_b16s0, set_mem_b32s1, set_rt_hit,
    };
    use crate::mem::{new_disc_global, new_kern_global, new_lig_item_global, new_ligature_global};

    #[inline(always)]
    unsafe fn is_char_node(p: i32) -> bool {
        p >= crate::globals::hi_mem_min()
    }

    let mut mk = init_k;

    loop {
        // main_lig_loop_1 / main_lig_loop_2
        let mj = font_info_b16(mk);

        if mj.s2 as i32 == cur_r() {
            if mj.s3 <= 128 {
                if mj.s1 >= 128 {
                    // Kern case: partial wrapup + append kern
                    if cur_l() < TOO_BIG_CHAR {
                        if mem_b32s1(cur_q()) > TEX_NULL
                            && mem_b16s0(cur_list_tail()) == hyphen_char_entry(main_f()) as u16
                        {
                            set_ins_disc(true);
                        }
                        if ligature_present() {
                            let mp =
                                new_ligature_global(main_f(), cur_l() as u16, mem_b32s1(cur_q()));
                            if lft_hit() {
                                set_mem_b16s0(mp, 2);
                                set_lft_hit(false);
                            }
                            // rt_hit: only if lig_stack == NULL (not applicable here since we have kern)
                            set_mem_b32s1(cur_q(), mp);
                            set_cur_list_tail(mp);
                            set_ligature_present(false);
                        }
                        if ins_disc() {
                            set_ins_disc(false);
                            if cur_list_mode() > 0 {
                                set_mem_b32s1(cur_list_tail(), new_disc_global());
                                set_cur_list_tail(mem_b32s1(cur_list_tail()));
                            }
                        }
                    }
                    let kval = crate::globals::font_info_b32s1(
                        kern_base_entry(main_f()) + 256 * mj.s1 as i32 + mj.s0 as i32,
                    );
                    let kn = new_kern_global(kval);
                    set_mem_b32s1(cur_list_tail(), kn);
                    set_cur_list_tail(mem_b32s1(cur_list_tail()));
                    return 1; // goto main_loop_move (skip wrapup)
                }

                // Ligature case
                if cur_l() == TOO_BIG_CHAR {
                    set_lft_hit(true);
                } else if lig_stack() == TEX_NULL {
                    set_rt_hit(true);
                }

                match mj.s1 {
                    1 | 5 => {
                        set_cur_l(mj.s0 as i32);
                        *main_i = font_info_b16(
                            char_base_entry(main_f())
                                + effective_char_global(true, main_f(), cur_l() as u16),
                        );
                        set_ligature_present(true);
                    }
                    2 | 6 => {
                        set_cur_r(mj.s0 as i32);
                        if lig_stack() == TEX_NULL {
                            set_lig_stack(new_lig_item_global(cur_r() as u16));
                            set_bchar(TOO_BIG_CHAR);
                        } else if is_char_node(lig_stack()) {
                            let prev = lig_stack();
                            set_lig_stack(new_lig_item_global(cur_r() as u16));
                            set_mem_b32s1(lig_stack() + 1, prev);
                        } else {
                            set_mem_b16s0(lig_stack(), cur_r() as u16);
                        }
                    }
                    3 => {
                        set_cur_r(mj.s0 as i32);
                        let prev = lig_stack();
                        set_lig_stack(new_lig_item_global(cur_r() as u16));
                        set_mem_b32s1(lig_stack(), prev);
                    }
                    7 | 11 => {
                        // Partial wrapup
                        if cur_l() < TOO_BIG_CHAR {
                            if mem_b32s1(cur_q()) > TEX_NULL
                                && mem_b16s0(cur_list_tail()) == hyphen_char_entry(main_f()) as u16
                            {
                                set_ins_disc(true);
                            }
                            if ligature_present() {
                                let mp = new_ligature_global(
                                    main_f(),
                                    cur_l() as u16,
                                    mem_b32s1(cur_q()),
                                );
                                if lft_hit() {
                                    set_mem_b16s0(mp, 2);
                                    set_lft_hit(false);
                                }
                                // no rt_hit for type 7/11
                                set_mem_b32s1(cur_q(), mp);
                                set_cur_list_tail(mp);
                                set_ligature_present(false);
                            }
                            if ins_disc() {
                                set_ins_disc(false);
                                if cur_list_mode() > 0 {
                                    set_mem_b32s1(cur_list_tail(), new_disc_global());
                                    set_cur_list_tail(mem_b32s1(cur_list_tail()));
                                }
                            }
                        }
                        set_cur_q(cur_list_tail());
                        set_cur_l(mj.s0 as i32);
                        *main_i = font_info_b16(
                            char_base_entry(main_f())
                                + effective_char_global(true, main_f(), cur_l() as u16),
                        );
                        set_ligature_present(true);
                    }
                    _ => {
                        // Type 0 or 4 (default): =: or |=:|
                        set_cur_l(mj.s0 as i32);
                        set_ligature_present(true);
                        if lig_stack() == TEX_NULL {
                            return 0; // goto main_loop_wrapup
                        } else {
                            return 2; // goto main_loop_move_1
                        }
                    }
                }

                // Post-switch tail for types 1,2,3,5,6,7,11:
                if mj.s1 > 4 && mj.s1 != 7 {
                    return 0; // types 5,6,11 → wrapup
                }
                // Types 1,2,3,7: re-enter lig loop
                if cur_l() < TOO_BIG_CHAR {
                    // re-enter main_lig_loop (outer): signal caller via enter_lig1_k = None
                    // The outer loop will re-enter at top (checking LIG_TAG)
                    // We need to break out and let caller restart 'char_proc at lig_loop top
                    return 0xFF; // sentinel: re-enter lig_loop (not wrapup!)
                }
                // cur_l == TOO_BIG_CHAR: re-enter at lig_loop_1 with bchar_label
                mk = bchar_label_entry(main_f());
                // continue inner loop (main_lig_loop_1)
            }
            // mj.s3 > 128: this entry's match byte > 128 means we skip (it's a stop/redirect)
            // but we already handled the redirect at the top. If we reach here with s3 > 128
            // in the inner loop, it means no match → go to wrapup.
            // Actually: if s3 > 128 when s2 == cur_r, that would be a redirect, but redirects
            // are only in the first entry and we've already computed the real index.
            // So s3 > 128 here when s2 == cur_r can't happen after the first entry redirect.
            // The s3 > 128 at line 7722 means "first entry has large byte → use redirect".
            // After redirect, s3 values in subsequent entries are normal (0-128).
        } else {
            // No match for this lig/kern table entry: advance
            if mj.s3 == 0 {
                mk += 1;
            } else if mj.s3 >= 128 {
                return 0; // no more entries → wrapup
            } else {
                mk += mj.s3 as i32 + 1;
            }
            // continue inner loop (main_lig_loop_1)
        }
    }
}

// Lookahead: get next token and set up lig_stack for it.
// Returns false if an inter-char token was injected (caller should return true = big_switch).
// On normal return, cur_r and lig_stack are set for the next lig/kern iteration.
#[inline(always)]
unsafe fn do_lookahead(
    main_i: &mut crate::mem_types::B16x4,
    enter_lig1_k: &mut Option<i32>,
    _mf: i32,
) -> bool {
    use crate::globals::{
        avail, bchar, cur_chr, cur_cmd, cur_input, cur_list_aux_b32s0, cur_ptr, eqtb_b32s1,
        false_bchar, main_f, mem_b32s1, prev_class, set_avail, set_bchar, set_cur_chr, set_cur_cmd,
        set_cur_input_index, set_cur_list_aux_b32s0, set_cur_ptr, set_cur_r, set_cur_tok,
        set_lig_stack, set_mem_b16s0, set_mem_b16s1, set_mem_b32s1, set_prev_class,
        set_space_class, space_class,
    };
    use crate::input::{back_input, begin_token_list};
    use crate::mem::get_avail_global;
    use crate::save::find_sa_element_global;

    const INTER_CHAR_TEXT: u16 = 17;
    const BACKED_UP_CHAR: u16 = 4;
    const TOKEN_LIST: u16 = 0;
    const INTER_CHAR_VAL_U: u16 = 6;
    const CHAR_CLASS_LIMIT: i32 = 4096;
    const MAX_CHAR_VAL: i32 = 0x200000;
    const INT_PAR_XETEX_INTER_CHAR_TOKENS: i32 = 73;
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const CHAR_GIVEN: u8 = 68;
    const CHAR_NUM: u8 = 16;
    const NO_BOUNDARY: u8 = 65;

    get_next();

    let is_char_cmd = |cmd: u8| cmd == LETTER || cmd == OTHER_CHAR || cmd == CHAR_GIVEN;

    if !is_char_cmd(cur_cmd()) {
        x_token();
    }
    if !is_char_cmd(cur_cmd()) {
        if cur_cmd() == CHAR_NUM {
            scan_char_num();
            set_cur_chr(cur_val());
            // fall through to is_char_cmd path
        } else {
            // Not a char: set cur_r = bchar, lig_stack = NULL
            if cur_cmd() == NO_BOUNDARY {
                set_bchar(TOO_BIG_CHAR);
            }
            set_cur_r(bchar());
            set_lig_stack(crate::mem::TEX_NULL);
            return true; // normal: re-enter lig loop at top
        }
    }

    // main_loop_lookahead_1: we have a char in cur_chr
    let sf = eqtb_b32s1(SF_CODE_BASE + cur_chr());
    let ms = sf % 65536;
    if ms == 1000 {
        set_cur_list_aux_b32s0(1000);
    } else if ms < 1000 {
        if ms > 0 {
            set_cur_list_aux_b32s0(ms);
        }
    } else if cur_list_aux_b32s0() < 1000 {
        set_cur_list_aux_b32s0(1000);
    } else {
        set_cur_list_aux_b32s0(ms);
    }
    set_cur_ptr(crate::mem::TEX_NULL);
    set_space_class(sf / 65536);
    let sc = space_class();

    // Inter-char check (lookahead_1 variant)
    if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0 && sc != CHAR_CLASS_LIMIT {
        if prev_class() == CHAR_CLASS_LIMIT - 1 {
            if cur_input().state != TOKEN_LIST || cur_input().index != BACKED_UP_CHAR {
                find_sa_element_global(
                    INTER_CHAR_VAL_U,
                    (CHAR_CLASS_LIMIT - 1) * CHAR_CLASS_LIMIT + sc,
                    false,
                );
                if cur_ptr() != crate::mem::TEX_NULL
                    && mem_b32s1(cur_ptr() + 1) != crate::mem::TEX_NULL
                {
                    if cur_cmd() != LETTER {
                        set_cur_cmd(OTHER_CHAR);
                    }
                    set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
                    back_input();
                    set_cur_input_index(BACKED_UP_CHAR);
                    begin_token_list(mem_b32s1(cur_ptr() + 1), INTER_CHAR_TEXT);
                    return false; // inter-char injected → caller returns true (big_switch)
                }
            }
        } else {
            find_sa_element_global(
                INTER_CHAR_VAL_U,
                prev_class() * CHAR_CLASS_LIMIT + sc,
                false,
            );
            if cur_ptr() != crate::mem::TEX_NULL && mem_b32s1(cur_ptr() + 1) != crate::mem::TEX_NULL
            {
                if cur_cmd() != LETTER {
                    set_cur_cmd(OTHER_CHAR);
                }
                set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
                back_input();
                set_cur_input_index(BACKED_UP_CHAR);
                begin_token_list(mem_b32s1(cur_ptr() + 1), INTER_CHAR_TEXT);
                set_prev_class(CHAR_CLASS_LIMIT - 1);
                return false; // inter-char injected → caller returns true (big_switch)
            }
        }
        set_prev_class(sc);
    }

    // Allocate lig_stack for the new char
    let ls = if avail() == crate::mem::TEX_NULL {
        get_avail_global()
    } else {
        let a = avail();
        set_avail(mem_b32s1(a));
        set_mem_b32s1(a, crate::mem::TEX_NULL);
        a
    };
    set_lig_stack(ls);
    set_mem_b16s1(ls, main_f() as u16);
    let cr = cur_chr();
    set_cur_r(cr);
    set_mem_b16s0(ls, cr as u16);
    if cr == false_bchar() {
        set_cur_r(TOO_BIG_CHAR);
    }

    true // normal lookahead complete, continue lig loop
}

// ---------------------------------------------------------------------------
// main_control
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:6766 (main_control)
// Top-level TeX command dispatcher: big_switch / reswitch loop.
#[no_mangle]
pub unsafe extern "C" fn main_control() {
    // Mode constants (xetex_format.h)
    const VMODE: u16 = 1;
    const HMODE: u16 = 104;
    const MMODE: u16 = 207;

    // Command constants (xetex_format.h)
    const RELAX: u8 = 0;
    const LEFT_BRACE: u8 = 1;
    const RIGHT_BRACE: u8 = 2;
    const MATH_SHIFT: u8 = 3;
    const TAB_MARK: u8 = 4;
    const CAR_RET: u8 = 5;
    const MAC_PARAM: u8 = 6;
    const SUP_MARK: u8 = 7;
    const SUB_MARK: u8 = 8;
    const ENDV: u8 = 9;
    const SPACER: u8 = 10;
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const PAR_END: u8 = 13;
    const STOP: u8 = 14;
    const DELIM_NUM: u8 = 15;
    const CHAR_NUM: u8 = 16;
    const MATH_CHAR_NUM: u8 = 17;
    const MARK: u8 = 18;
    const XRAY: u8 = 19;
    const MAKE_BOX: u8 = 20;
    const HMOVE: u8 = 21;
    const VMOVE: u8 = 22;
    const UN_HBOX: u8 = 23;
    const UN_VBOX: u8 = 24;
    const REMOVE_ITEM: u8 = 25;
    const HSKIP: u8 = 26;
    const VSKIP: u8 = 27;
    const MSKIP: u8 = 28;
    const KERN: u8 = 29;
    const MKERN: u8 = 30;
    const LEADER_SHIP: u8 = 31;
    const HALIGN: u8 = 32;
    const VALIGN: u8 = 33;
    const NO_ALIGN: u8 = 34;
    const VRULE: u8 = 35;
    const HRULE: u8 = 36;
    const INSERT: u8 = 37;
    const VADJUST: u8 = 38;
    const IGNORE_SPACES: u8 = 39;
    const AFTER_ASSIGNMENT: u8 = 40;
    const AFTER_GROUP: u8 = 41;
    const BREAK_PENALTY: u8 = 42;
    const START_PAR: u8 = 43;
    const ITAL_CORR: u8 = 44;
    const ACCENT: u8 = 45;
    const MATH_ACCENT: u8 = 46;
    const DISCRETIONARY: u8 = 47;
    const EQ_NO: u8 = 48;
    const LEFT_RIGHT: u8 = 49;
    const MATH_COMP: u8 = 50;
    const LIMIT_SWITCH: u8 = 51;
    const ABOVE: u8 = 52;
    const MATH_STYLE: u8 = 53;
    const MATH_CHOICE: u8 = 54;
    const NON_SCRIPT: u8 = 55;
    const VCENTER: u8 = 56;
    const CASE_SHIFT: u8 = 57;
    const MESSAGE: u8 = 58;
    const EXTENSION: u8 = 59;
    const IN_STREAM: u8 = 60;
    const BEGIN_GROUP: u8 = 61;
    const END_GROUP: u8 = 62;
    const OMIT: u8 = 63;
    const EX_SPACE: u8 = 64;
    const NO_BOUNDARY: u8 = 65;
    const RADICAL: u8 = 66;
    const END_CS_NAME: u8 = 67;
    const CHAR_GIVEN: u8 = 68;
    const MATH_GIVEN: u8 = 69;
    const XETEX_MATH_GIVEN: u8 = 70;
    const LAST_ITEM: u8 = 71;
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
    const DEF_CODE: u8 = 86;
    const XETEX_DEF_CODE: u8 = 87;
    const DEF_FAMILY: u8 = 88;
    const SET_FONT: u8 = 89;
    const DEF_FONT: u8 = 90;
    const REGISTER: u8 = 91;
    const ADVANCE: u8 = 92;
    const MULTIPLY: u8 = 93;
    const DIVIDE: u8 = 94;
    const PREFIX: u8 = 95;
    const LET: u8 = 96;
    const SHORTHAND_DEF: u8 = 97;
    const READ_TO_CS: u8 = 98;
    const DEF: u8 = 99;
    const SET_BOX: u8 = 100;
    const HYPH_DATA: u8 = 101;
    const SET_INTERACTION: u8 = 102;

    const CC_LOCAL_BASE: i32 = 2_254_871; // xetex_format.h:208
    const CC_LOCAL_EVERY_JOB: i32 = 7; // xetex_format.h:179
    const EVERY_JOB_TEXT: u16 = 13; // xetex-constants.h:212
    const INT_PAR_TRACING_COMMANDS: i32 = 36; // xetex_format.h:71

    use crate::globals::{
        cur_cmd, cur_list_mode, cur_val, eqtb_b32s1, set_cancel_boundary, set_cur_chr,
    };
    use crate::mem::TEX_NULL;

    let every_job = eqtb_b32s1(CC_LOCAL_BASE + CC_LOCAL_EVERY_JOB);
    if every_job != TEX_NULL {
        crate::input::begin_token_list(every_job, EVERY_JOB_TEXT);
    }

    'big_switch: loop {
        get_x_token();

        'reswitch: loop {
            if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 0 {
                crate::output::show_cur_cmd_chr();
            }

            let mode = cur_list_mode().unsigned_abs();
            let cmd = cur_cmd();

            // Fast char path and boundary check (first switch in C, before inter-char check)
            match (mode, cmd) {
                (HMODE, LETTER) | (HMODE, OTHER_CHAR) | (HMODE, CHAR_GIVEN) => {
                    if main_control_main_loop() {
                        break 'reswitch;
                    }
                    continue 'reswitch;
                }
                (HMODE, CHAR_NUM) => {
                    scan_usv_num();
                    set_cur_chr(cur_val());
                    if main_control_main_loop() {
                        break 'reswitch;
                    }
                    continue 'reswitch;
                }
                (HMODE, NO_BOUNDARY) => {
                    get_x_token();
                    if matches!(cur_cmd(), LETTER | OTHER_CHAR | CHAR_GIVEN | CHAR_NUM) {
                        set_cancel_boundary(true);
                    }
                    continue 'reswitch;
                }
                _ => {}
            }

            if main_control_hmode_inter_char_check() {
                break 'reswitch; // → big_switch
            }

            // Main command dispatch (second switch in C)
            match (mode, cmd) {
                // Spacer/ex_space → spacer_cases
                (HMODE, SPACER) | (HMODE, EX_SPACE) | (MMODE, EX_SPACE) => {
                    main_control_spacer_cases();
                }
                // No-op: ANY_MODE(RELAX), VMODE+SPACER, MMODE+SPACER, MMODE+NO_BOUNDARY
                (HMODE, RELAX)
                | (VMODE, RELAX)
                | (MMODE, RELAX)
                | (VMODE, SPACER)
                | (MMODE, SPACER)
                | (MMODE, NO_BOUNDARY) => {}
                // ANY_MODE(IGNORE_SPACES) → call helper then reswitch
                (HMODE, IGNORE_SPACES) | (VMODE, IGNORE_SPACES) | (MMODE, IGNORE_SPACES) => {
                    main_control_ignore_spaces();
                    continue 'reswitch;
                }
                // VMODE+STOP → "the only way out"
                (VMODE, STOP) => {
                    if its_all_over() {
                        return;
                    }
                }
                // Forbidden / math-mode-mismatch error cases
                (VMODE, VMOVE)
                | (HMODE, HMOVE)
                | (MMODE, HMOVE)
                | (VMODE, LAST_ITEM)
                | (HMODE, LAST_ITEM)
                | (MMODE, LAST_ITEM)
                | (VMODE, VADJUST)
                | (VMODE, ITAL_CORR)
                | (VMODE, EQ_NO)
                | (HMODE, EQ_NO)
                | (VMODE, MAC_PARAM)
                | (HMODE, MAC_PARAM)
                | (MMODE, MAC_PARAM)
                | (VMODE, SUP_MARK)
                | (HMODE, SUP_MARK)
                | (VMODE, SUB_MARK)
                | (HMODE, SUB_MARK)
                | (VMODE, MATH_CHAR_NUM)
                | (HMODE, MATH_CHAR_NUM)
                | (VMODE, MATH_GIVEN)
                | (HMODE, MATH_GIVEN)
                | (VMODE, XETEX_MATH_GIVEN)
                | (HMODE, XETEX_MATH_GIVEN)
                | (VMODE, MATH_COMP)
                | (HMODE, MATH_COMP)
                | (VMODE, DELIM_NUM)
                | (HMODE, DELIM_NUM)
                | (VMODE, LEFT_RIGHT)
                | (HMODE, LEFT_RIGHT)
                | (VMODE, ABOVE)
                | (HMODE, ABOVE)
                | (VMODE, RADICAL)
                | (HMODE, RADICAL)
                | (VMODE, MATH_STYLE)
                | (HMODE, MATH_STYLE)
                | (VMODE, MATH_CHOICE)
                | (HMODE, MATH_CHOICE)
                | (VMODE, VCENTER)
                | (HMODE, VCENTER)
                | (VMODE, NON_SCRIPT)
                | (HMODE, NON_SCRIPT)
                | (VMODE, MKERN)
                | (HMODE, MKERN)
                | (VMODE, LIMIT_SWITCH)
                | (HMODE, LIMIT_SWITCH)
                | (VMODE, MSKIP)
                | (HMODE, MSKIP)
                | (VMODE, MATH_ACCENT)
                | (HMODE, MATH_ACCENT)
                | (MMODE, ENDV)
                | (MMODE, PAR_END)
                | (MMODE, STOP)
                | (MMODE, VSKIP)
                | (MMODE, UN_VBOX)
                | (MMODE, VALIGN)
                | (MMODE, HRULE) => {
                    main_control_mode_error_cases();
                }
                // Rule/skip/kern/group cases
                (VMODE, HRULE)
                | (HMODE, VRULE)
                | (MMODE, VRULE)
                | (VMODE, VSKIP)
                | (HMODE, HSKIP)
                | (MMODE, HSKIP)
                | (MMODE, MSKIP)
                | (VMODE, KERN)
                | (HMODE, KERN)
                | (MMODE, KERN)
                | (MMODE, MKERN)
                | (VMODE, LEFT_BRACE)
                | (HMODE, LEFT_BRACE)
                | (VMODE, BEGIN_GROUP)
                | (HMODE, BEGIN_GROUP)
                | (MMODE, BEGIN_GROUP)
                | (VMODE, END_GROUP)
                | (HMODE, END_GROUP)
                | (MMODE, END_GROUP) => {
                    main_control_simple_list_group_cases();
                }
                // Box/list construction cases
                (VMODE, RIGHT_BRACE)
                | (HMODE, RIGHT_BRACE)
                | (MMODE, RIGHT_BRACE)
                | (VMODE, HMOVE)
                | (HMODE, VMOVE)
                | (MMODE, VMOVE)
                | (VMODE, LEADER_SHIP)
                | (HMODE, LEADER_SHIP)
                | (MMODE, LEADER_SHIP)
                | (VMODE, MAKE_BOX)
                | (HMODE, MAKE_BOX)
                | (MMODE, MAKE_BOX)
                | (VMODE, START_PAR)
                | (VMODE, LETTER)
                | (VMODE, OTHER_CHAR)
                | (VMODE, CHAR_NUM)
                | (VMODE, CHAR_GIVEN)
                | (VMODE, MATH_SHIFT)
                | (VMODE, UN_HBOX)
                | (VMODE, VRULE)
                | (VMODE, ACCENT)
                | (VMODE, DISCRETIONARY)
                | (VMODE, HSKIP)
                | (VMODE, VALIGN)
                | (VMODE, EX_SPACE)
                | (VMODE, NO_BOUNDARY)
                | (HMODE, START_PAR)
                | (MMODE, START_PAR)
                | (VMODE, PAR_END)
                | (HMODE, PAR_END)
                | (HMODE, STOP)
                | (HMODE, VSKIP)
                | (HMODE, HRULE)
                | (HMODE, UN_VBOX)
                | (HMODE, HALIGN)
                | (VMODE, INSERT)
                | (HMODE, INSERT)
                | (MMODE, INSERT)
                | (HMODE, VADJUST)
                | (MMODE, VADJUST)
                | (VMODE, MARK)
                | (HMODE, MARK)
                | (MMODE, MARK)
                | (VMODE, BREAK_PENALTY)
                | (HMODE, BREAK_PENALTY)
                | (MMODE, BREAK_PENALTY)
                | (VMODE, REMOVE_ITEM)
                | (HMODE, REMOVE_ITEM)
                | (MMODE, REMOVE_ITEM)
                | (VMODE, UN_VBOX)
                | (HMODE, UN_HBOX)
                | (MMODE, UN_HBOX)
                | (HMODE, ITAL_CORR)
                | (MMODE, ITAL_CORR)
                | (HMODE, DISCRETIONARY)
                | (MMODE, DISCRETIONARY)
                | (HMODE, ACCENT)
                | (VMODE, CAR_RET)
                | (HMODE, CAR_RET)
                | (MMODE, CAR_RET)
                | (VMODE, TAB_MARK)
                | (HMODE, TAB_MARK)
                | (MMODE, TAB_MARK) => {
                    main_control_box_list_cases();
                }
                // Alignment / endcsname cases
                (VMODE, NO_ALIGN)
                | (HMODE, NO_ALIGN)
                | (MMODE, NO_ALIGN)
                | (VMODE, OMIT)
                | (HMODE, OMIT)
                | (MMODE, OMIT)
                | (VMODE, HALIGN)
                | (HMODE, VALIGN)
                | (MMODE, HALIGN)
                | (VMODE, ENDV)
                | (HMODE, ENDV)
                | (VMODE, END_CS_NAME)
                | (HMODE, END_CS_NAME)
                | (MMODE, END_CS_NAME) => {
                    main_control_alignment_cases();
                }
                // Math entry/exit cases
                (HMODE, MATH_SHIFT)
                | (MMODE, EQ_NO)
                | (MMODE, LEFT_BRACE)
                | (MMODE, LETTER)
                | (MMODE, OTHER_CHAR)
                | (MMODE, CHAR_GIVEN)
                | (MMODE, CHAR_NUM)
                | (MMODE, MATH_CHAR_NUM)
                | (MMODE, MATH_GIVEN)
                | (MMODE, XETEX_MATH_GIVEN)
                | (MMODE, DELIM_NUM)
                | (MMODE, MATH_COMP)
                | (MMODE, LIMIT_SWITCH)
                | (MMODE, RADICAL)
                | (MMODE, ACCENT)
                | (MMODE, MATH_ACCENT)
                | (MMODE, VCENTER)
                | (MMODE, MATH_STYLE)
                | (MMODE, NON_SCRIPT)
                | (MMODE, MATH_CHOICE)
                | (MMODE, SUB_MARK)
                | (MMODE, SUP_MARK)
                | (MMODE, ABOVE)
                | (MMODE, LEFT_RIGHT)
                | (MMODE, MATH_SHIFT) => {
                    main_control_math_cases();
                }
                // Mode-independent cases (ANY_MODE): assignments, defs, etc.
                _ if matches!(
                    cmd,
                    TOKS_REGISTER
                        | ASSIGN_TOKS
                        | ASSIGN_INT
                        | ASSIGN_DIMEN
                        | ASSIGN_GLUE
                        | ASSIGN_MU_GLUE
                        | ASSIGN_FONT_DIMEN
                        | ASSIGN_FONT_INT
                        | SET_AUX
                        | SET_PREV_GRAF
                        | SET_PAGE_DIMEN
                        | SET_PAGE_INT
                        | SET_BOX_DIMEN
                        | SET_SHAPE
                        | DEF_CODE
                        | XETEX_DEF_CODE
                        | DEF_FAMILY
                        | SET_FONT
                        | DEF_FONT
                        | REGISTER
                        | ADVANCE
                        | MULTIPLY
                        | DIVIDE
                        | PREFIX
                        | LET
                        | SHORTHAND_DEF
                        | READ_TO_CS
                        | DEF
                        | SET_BOX
                        | HYPH_DATA
                        | SET_INTERACTION
                        | AFTER_ASSIGNMENT
                        | AFTER_GROUP
                        | IN_STREAM
                        | MESSAGE
                        | CASE_SHIFT
                        | XRAY
                        | EXTENSION
                ) =>
                {
                    main_control_mode_independent();
                }
                // Default: unrecognized / already handled → goto big_switch
                _ => {}
            }

            break 'reswitch; // → big_switch (get_x_token at top of loop)
        }
    }
}
