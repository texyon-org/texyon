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
// Relocated to errors.rs (workstream 02); still called from scan.rs.
use crate::errors::runaway;
use crate::ext::aat_font_get;
use crate::ext::aat_font_get_1;
use crate::ext::aat_font_get_2;
use crate::ext::aat_font_get_named;
use crate::ext::aat_font_get_named_1;
use crate::ext::gr_font_get_named;
use crate::ext::gr_font_get_named_1;
use crate::ext::ot_font_get;
use crate::ext::ot_font_get_1;
use crate::ext::ot_font_get_2;
use crate::ext::ot_font_get_3;
use crate::io::input_line;
use crate::io::u_close;
use crate::output::capture_to_diagnostic;
use crate::output::error_here_with_diagnostic;

use crate::globals::{
    align_state, buffer_entry, cur_chr, cur_cmd, cur_cs, cur_input, cur_tok, cur_val, eqtb_b32s1,
    expand_depth, first, init_pool_ptr, last, mem_b32s0, mem_b32s1, pool_size, read_file_entry,
    read_open_entry, save_ptr, set_align_state, set_buffer_entry, set_cur_chr, set_cur_cmd,
    set_cur_cs, set_cur_input_limit, set_cur_input_loc, set_cur_input_name, set_cur_input_state,
    set_cur_input_synctex_tag, set_cur_tok, set_cur_val, set_def_ref, set_first, set_help_line,
    set_help_ptr, set_line, set_mem_b32s0, set_mem_b32s1, set_no_new_control_sequence,
    set_read_open_entry, set_scanner_status, set_warning_index,
};
// Relocated to packaging.rs (workstream 02); still called from scan.rs.
// Relocated to expand.rs (workstream 02); still called from scan.rs.
use crate::errors::int_error_in;
use crate::expand::{expand, expand_in, macro_call_in};
use crate::globals::{
    cur_list, cur_list_aux_b32s0, cur_list_aux_b32s1, cur_list_head, cur_list_mode, cur_list_tail,
    cur_ptr, set_cur_ptr,
};
use crate::output::capture_to_diagnostic_in;
use crate::output::error_here_with_diagnostic_in;
use crate::output::print_cmd_chr_in;
use crate::output::print_cstr_in;
use crate::output::print_esc_cstr_in;
use crate::output::print_int_in;
use crate::output::print_nl_cstr_in;
// Relocated to main_control.rs (workstream 02); still called from scan.rs.
use crate::main_control::push_math;

use crate::fonts::{effective_char, effective_char_info, new_native_word_node};
use crate::io::open_log_file;
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
pub unsafe fn scan_char_class() {
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
pub unsafe fn scan_char_class_not_ignored() {
    if let Err(e) = crate::state::with_state_mut(|st| scan_char_class_not_ignored_in(st)) {
        unsafe { e.fire() }
    }
}

// translated from xetex-xetex0.c:5114 (scan_eight_bit_int)
pub unsafe fn scan_eight_bit_int() {
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
pub unsafe fn scan_usv_num() {
    if let Err(e) = crate::state::with_state_mut(|st| scan_usv_num_in(st)) {
        unsafe { e.fire() }
    }
}

// translated from xetex-xetex0.c:5150 (scan_char_num)
pub unsafe fn scan_char_num() {
    if let Err(e) = crate::state::with_state_mut(|st| scan_char_num_in(st)) {
        unsafe { e.fire() }
    }
}

// translated from xetex-xetex0.c:5330 (scan_math_class_int)
pub unsafe fn scan_math_class_int() {
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
pub unsafe fn scan_math_fam_int() {
    if let Err(e) = crate::state::with_state_mut(|st| scan_math_fam_int_in(st)) {
        unsafe { e.fire() }
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
pub unsafe fn scan_fifteen_bit_int() {
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
pub unsafe fn scan_xetex_math_char_int() {
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
pub unsafe fn scan_register_num() {
    if let Err(e) = crate::state::with_state_mut(|st| scan_register_num_in(st)) {
        unsafe { e.fire() }
    }
}

// translated from xetex-xetex0.c:5403 (scan_delimiter_int)
pub unsafe fn scan_delimiter_int() {
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
pub fn max_hyphenatable_length() -> i32 {
    crate::state::with_state(|st| max_hyphenatable_length_in(&st.eqtb))
}

/// Threaded (Phase C) core of [`max_hyphenatable_length`]: reads the
/// `\XeTeXhyphenatablelength` integer parameter directly from the borrowed
/// `eqtb`, so it is safe to call under a live `&mut EngineState` borrow.
pub fn max_hyphenatable_length_in(eqtb: &crate::state::EqtbState) -> i32 {
    let v = eqtb.b32s1(INT_BASE + INT_PAR_XETEX_HYPHENATABLE_LENGTH);
    if v > HYPHENATABLE_LENGTH_LIMIT {
        HYPHENATABLE_LENGTH_LIMIT
    } else {
        v
    }
}

// ---------------------------------------------------------------------------
// scan_normal_glue / scan_mu_glue
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// get_tracing_fonts_state / scan_pdf_ext_toks
// ---------------------------------------------------------------------------

/// Returns INTPAR(xetex_tracing_fonts).
///
/// translated from xetex-xetex0.c:9082 (get_tracing_fonts_state)
pub fn get_tracing_fonts_state() -> i32 {
    eqtb_b32s1(INT_BASE + INT_PAR_XETEX_TRACING_FONTS)
}

// ---------------------------------------------------------------------------
// x_token / scan_optional_equals
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// get_token
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:3653 (get_token)
pub unsafe fn get_token() {
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
pub unsafe fn x_token() {
    if let Err(e) = crate::state::with_state_mut(|st| x_token_in(st)) {
        unsafe { e.fire() }
    }
}

// Token-encoding constants (xetex-constants.h:353).
const MATH_SHIFT_TOKEN: i32 = 0x600000;

/// translated from xetex-xetex0.c:11444 (insert_dollar_sign)
pub unsafe fn insert_dollar_sign() {
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
    if let Err(e) = crate::state::with_state_mut(|st| scan_left_brace_in(st)) {
        unsafe { e.fire() }
    }
}

// ---------------------------------------------------------------------------
// get_x_token / get_x_or_protected
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:4272 (get_x_token)
pub unsafe fn get_x_token() {
    if let Err(e) = crate::state::with_state_mut(|st| get_x_token_in(st)) {
        unsafe { e.fire() }
    }
}

/// translated from xetex-xetex0.c:4533 (get_x_or_protected)
pub unsafe fn get_x_or_protected() {
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
pub unsafe fn scan_keyword(s: *const libc::c_char) -> bool {
    match crate::state::with_state_mut(|st| scan_keyword_in(st, s)) {
        Ok(v) => v,
        Err(e) => unsafe { e.fire() },
    }
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

// ---------------------------------------------------------------------------
// scan_spec
// ---------------------------------------------------------------------------

// xetex-constants.h:256,264
const EXACTLY: u8 = 0;
const ADDITIONAL: u8 = 1;

/// translated from xetex-xetex0.c:9215 (scan_spec)
pub unsafe fn scan_spec(c: u8, three_codes: bool) {
    if let Err(e) = crate::state::with_state_mut(|st| scan_spec_in(st, c, three_codes)) {
        e.fire();
    }
}

/// Threaded (Phase C) core of [`scan_spec`]. The save stack lives in
/// `st.save_stack`; the only cold abort is `new_save_level`'s save-stack
/// overflow, surfaced through [`SccAbort`].
pub(crate) fn scan_spec_in(
    st: &mut crate::state::EngineState,
    c: u8,
    three_codes: bool,
) -> Result<(), SccAbort> {
    let s = if three_codes {
        st.save_stack.b32s1(st.save_stack.save_ptr)
    } else {
        0
    };
    let spec_code;
    if scan_keyword_in(st, c"to".as_ptr())? {
        spec_code = EXACTLY;
        scan_dimen_in(st, false, false, false)?;
    } else if scan_keyword_in(st, c"spread".as_ptr())? {
        spec_code = ADDITIONAL;
        scan_dimen_in(st, false, false, false)?;
    } else {
        spec_code = ADDITIONAL;
        set_cur_val(0);
    }
    // found:
    if three_codes {
        let sp = st.save_stack.save_ptr;
        st.save_stack.set_b32s1(sp, s);
        st.save_stack.save_ptr = sp + 1;
    }
    let sp = st.save_stack.save_ptr;
    st.save_stack.set_b32s1(sp, spec_code as i32);
    st.save_stack.set_b32s1(sp + 1, cur_val());
    st.save_stack.save_ptr = sp + 2;
    if let Some(abort) = crate::save::new_save_level(&mut st.save_stack, c) {
        return Err(match abort {
            crate::save::SaveAbort::Overflow(what, n) => {
                SccAbort::Input(crate::input::InputAbort::Overflow {
                    what: what.as_ptr(),
                    n,
                })
            }
            crate::save::SaveAbort::Confusion(what) => {
                SccAbort::Input(crate::input::InputAbort::Fatal(what.as_ptr()))
            }
        });
    }
    scan_left_brace_in(st)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// round_decimals / xetex_scan_dimen / scan_glyph_number
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:4392 (round_decimals, was static)
//
// Phase-C (scanner-SCC prereq): reads `dig` directly from `&ErrorState` so a
// threaded caller holding `&mut EngineState` can call it under the borrow.
fn round_decimals(errors: &crate::state::ErrorState, k: i32) -> i32 {
    let mut a: i32 = 0;
    let mut k = k;
    while k > 0 {
        k -= 1;
        a = (a + errors.dig[k as usize] as i32 * 0x2_0000) / 10;
    }
    (a + 1) / 2
}

/// translated from xetex-xetex0.c:4406 (xetex_scan_dimen)
pub unsafe fn xetex_scan_dimen(mu: bool, inf: bool, shortcut: bool, requires_units: bool) {
    if let Err(e) = crate::state::with_state_mut(|st| {
        xetex_scan_dimen_in(st, mu, inf, shortcut, requires_units)
    }) {
        unsafe { e.fire() }
    }
}

/// translated from xetex-xetex0.c:4302 (scan_glyph_number)
pub unsafe fn scan_glyph_number(f: i32) {
    if let Err(e) = crate::state::with_state_mut(|st| scan_glyph_number_in(st, f)) {
        unsafe { e.fire() }
    }
}

// ---------------------------------------------------------------------------
// scan_dimen / scan_decimal
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:5956 (scan_dimen)
pub unsafe fn scan_dimen(mu: bool, inf: bool, shortcut: bool) {
    if let Err(e) = crate::state::with_state_mut(|st| scan_dimen_in(st, mu, inf, shortcut)) {
        unsafe { e.fire() }
    }
}

/// translated from xetex-xetex0.c:5961 (scan_decimal)
pub unsafe fn scan_decimal() {
    xetex_scan_dimen(false, false, false, false);
}

// ---------------------------------------------------------------------------
// scan_rule_spec
// ---------------------------------------------------------------------------

// xetex_format.h:273 / xetex-constants.h:316,339
const VRULE: u8 = 35;
const DEFAULT_RULE: i32 = 26214;

/// translated from xetex-xetex0.c:6267 (scan_rule_spec)
pub unsafe fn scan_rule_spec() -> i32 {
    match crate::state::with_state_mut(scan_rule_spec_in) {
        Ok(q) => q,
        Err(e) => e.fire(),
    }
}

/// Threaded (Phase C) core of [`scan_rule_spec`].
pub(crate) fn scan_rule_spec_in(st: &mut crate::state::EngineState) -> Result<i32, SccAbort> {
    let q = crate::mem::new_rule(&mut st.mem);
    if q == crate::mem::ALLOC_FAILED {
        return Err(main_mem_overflow_scc());
    }
    if cur_cmd() == VRULE {
        st.mem.set_b32s1(q + 1, DEFAULT_RULE);
    } else {
        st.mem.set_b32s1(q + 3, DEFAULT_RULE);
        st.mem.set_b32s1(q + 2, 0);
    }
    loop {
        if scan_keyword_in(st, c"width".as_ptr())? {
            xetex_scan_dimen_in(st, false, false, false, true)?;
            st.mem.set_b32s1(q + 1, cur_val());
        } else if scan_keyword_in(st, c"height".as_ptr())? {
            xetex_scan_dimen_in(st, false, false, false, true)?;
            st.mem.set_b32s1(q + 3, cur_val());
        } else if scan_keyword_in(st, c"depth".as_ptr())? {
            xetex_scan_dimen_in(st, false, false, false, true)?;
            st.mem.set_b32s1(q + 2, cur_val());
        } else {
            break;
        }
    }
    Ok(q)
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

// ---------------------------------------------------------------------------
// str_toks_cat / str_toks
// ---------------------------------------------------------------------------

// xetex-constants.h:356
const SPACE_TOKEN: i32 = 0x1400020;
// xetex_format.h:246
const ACTIVE_CHAR: i32 = 13;

/// translated from xetex-xetex0.c:6363 (str_toks_cat)
/// Cold abort reasons threaded out of [`str_toks_cat_in`] so the bridge
/// wrapper can run the `overflow` longjmp *after* the `with_state` borrow
/// drops (`01b` §5). Never observed on a passing run.
pub(crate) enum StrToksAbort {
    /// `pool_ptr + 1 > pool_size`: `overflow("pool size", …)`.
    PoolOverflow,
    /// `get_avail` exhausted the arena: `runaway()` then `overflow`.
    MainMemoryOverflow,
}

/// `&mut` core of `str_toks_cat` (`01b` §3). Touches `mem` (token list) and
/// `strings` (pool) only; both cold overflow paths defer to the caller via
/// [`StrToksAbort`] (`01b` §5).
pub(crate) fn str_toks_cat_in(
    mem: &mut crate::state::MemState,
    strings: &mut crate::state::StringsState,
    b: i32,
    cat: i16,
) -> Result<i32, StrToksAbort> {
    use crate::mem::TEX_NULL;

    if strings.pool_ptr + 1 > strings.pool_size {
        return Err(StrToksAbort::PoolOverflow);
    }

    let mut p = TEMP_HEAD;
    mem.set_b32s1(p, TEX_NULL);
    let mut k = b;

    while k < strings.pool_ptr {
        let mut t = strings.str_pool[k as usize] as i32;

        if t == b' ' as i32 && cat == 0 {
            t = SPACE_TOKEN;
        } else {
            // decode UTF-16 surrogate pair
            if (0xD800..0xDC00).contains(&t) && k + 1 < strings.pool_ptr {
                let lo = strings.str_pool[(k + 1) as usize] as i32;
                if (0xDC00..0xE000).contains(&lo) {
                    k += 1;
                    t = 0x10000
                        + ((t - 0xD800) << 10)
                        + (strings.str_pool[k as usize] as i32 - 0xDC00);
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

        let q = if mem.avail == TEX_NULL {
            let qa = crate::mem::get_avail(mem);
            if qa == crate::mem::ALLOC_FAILED {
                return Err(StrToksAbort::MainMemoryOverflow);
            }
            qa
        } else {
            let qa = mem.avail;
            mem.avail = mem.b32s1(qa);
            mem.set_b32s1(qa, TEX_NULL);
            qa
        };

        mem.set_b32s1(p, q);
        mem.set_b32s0(q, t);
        p = q;
        k += 1;
    }

    strings.pool_ptr = b;
    Ok(p)
}

/// Bridge-down wrapper for legacy `str_toks_cat` callers (`01b` §3, §5). Runs
/// the deferred cold overflow after the state borrow drops.
pub unsafe fn str_toks_cat(b: i32, cat: i16) -> i32 {
    match crate::state::with_state_mut(|st| str_toks_cat_in(&mut st.mem, &mut st.strings, b, cat)) {
        Ok(p) => p,
        Err(StrToksAbort::PoolOverflow) => {
            crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr())
        }
        Err(StrToksAbort::MainMemoryOverflow) => crate::mem::fire_main_memory_overflow(),
    }
}

/// translated from xetex-xetex0.c:6417 (str_toks)
pub unsafe fn str_toks(b: i32) -> i32 {
    str_toks_cat(b, 0)
}

// ---------------------------------------------------------------------------
// the_toks
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:6427 (the_toks)
pub unsafe fn the_toks() -> i32 {
    match crate::state::with_state_mut(|st| the_toks_in(st)) {
        Ok(v) => v,
        Err(e) => unsafe { e.fire() },
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
pub unsafe fn scan_toks(macro_def: bool, xpand: bool) -> i32 {
    match crate::state::with_state_mut(|st| scan_toks_in(st, macro_def, xpand)) {
        Ok(v) => v,
        Err(e) => unsafe { e.fire() },
    }
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
pub unsafe fn read_toks(n: i32, r: i32, j: i32) {
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
pub unsafe fn scan_font_ident() {
    if let Err(e) = crate::state::with_state_mut(|st| scan_font_ident_in(st)) {
        unsafe { e.fire() }
    }
}

/// translated from xetex-xetex0.c:4501 (find_font_dimen)
pub unsafe fn find_font_dimen(writing: bool) {
    if let Err(e) = crate::state::with_state_mut(|st| find_font_dimen_in(st, writing)) {
        unsafe { e.fire() }
    }
}

// ---------------------------------------------------------------------------
// scan_something_internal
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// scan_int
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:5465 (scan_int)
pub unsafe fn scan_int() {
    if let Err(e) = crate::state::with_state_mut(|st| scan_int_in(st)) {
        unsafe { e.fire() }
    }
}

// ---------------------------------------------------------------------------
// eTeX_enabled
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:8811 (eTeX_enabled)
#[allow(non_snake_case)]
pub unsafe fn eTeX_enabled(b: bool, j: u16, k: i32) -> bool {
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
pub fn effective_char_global(err_p: bool, f: i32, c: u16) -> i32 {
    crate::state::with_state_mut(|st| effective_char(&mut st.fonts, err_p, f, c))
}

/// Transitional bridge-down wrapper for legacy `effective_char_info` callers.
pub fn effective_char_info_global(f: i32, c: u16) -> crate::mem_types::B16x4 {
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
/// Cold-abort sentinel for [`tokens_to_string_in`] (`01b` §5).
pub(crate) enum TokensToStringAbort {
    /// Called with `selector` already `new_string` (a programmer error).
    SelectorNewString,
    /// The string pool's `str_ptr` is exhausted.
    StringOverflow,
}

/// `&mut EngineState` core of [`tokens_to_string`] (scanner-SCC prereq).
///
/// Renders token list `p` into the string pool under one borrow via the
/// threaded `show_token_list_in`; the `new_string`-reentrancy guard and the
/// string-pool overflow ride [`TokensToStringAbort`] (`01b` §5).
pub(crate) fn tokens_to_string_in(
    st: &mut crate::state::EngineState,
    p: i32,
) -> Result<i32, TokensToStringAbort> {
    if st.shipout.selector == SELECTOR_NEW_STRING {
        return Err(TokensToStringAbort::SelectorNewString);
    }
    st.shipout.old_setting = st.shipout.selector as u8;
    st.shipout.selector = SELECTOR_NEW_STRING;
    let arg = st.mem.b32s1(p);
    let l = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list_in(st, arg, crate::mem::TEX_NULL, l);
    st.shipout.selector = st.shipout.old_setting as i32;
    crate::string_pool::make_string(&mut st.strings)
        .map_err(|_| TokensToStringAbort::StringOverflow)
}

// ---------------------------------------------------------------------------
// compare_strings
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// do_endv
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// get_r_token
// ---------------------------------------------------------------------------

const FROZEN_CONTROL_SEQUENCE: i32 = 2_243_226; // xetex_format.h:201
const FROZEN_PROTECTION: i32 = FROZEN_CONTROL_SEQUENCE; // xetex-constants.h:55

// translated from xetex-xetex0.c:10407 (get_r_token)
pub unsafe fn get_r_token() {
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
pub unsafe fn scan_math(p: i32) {
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
pub unsafe fn set_math_char(c: i32) {
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
pub unsafe fn scan_glue(level: i16) {
    if let Err(e) = crate::state::with_state_mut(|st| scan_glue_in(st, level)) {
        unsafe { e.fire() }
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

/// `&mut EngineState` core of [`check_outer_validity`] (`01b` §3). The whole
/// cold runaway/forbidden-cs/incomplete-`\if` recovery now runs under one
/// borrow: scanner/error/input state is touched directly; `cur_cs`/`cur_input`/
/// `set_cur_*` are raw `mod raw` globals; the token-list pushes go through
/// `begin_token_list_in`, the diagnostics through `runaway_in`/the threaded
/// `print_*_in`/`error_here_with_diagnostic_in` cores, and the recovery error
/// through `error_in`/`ins_error_in`. The final `error`/`ins_error`
/// [`ErrorOutcome`](crate::errors::ErrorOutcome) is returned for the wrapper to
/// `finish` after the borrow drops; arena exhaustion and `begin_token_list`
/// overflow ride [`InputAbort`](crate::input::InputAbort). Normal (`NORMAL`)
/// status returns `ErrorOutcome::Continue`.
pub(crate) fn check_outer_validity_in(
    st: &mut crate::state::EngineState,
) -> Result<crate::errors::ErrorOutcome, crate::input::InputAbort> {
    use crate::errors::ErrorOutcome;
    use crate::globals::{cur_cs, cur_input, set_cur_chr, set_cur_cmd, set_cur_cs, set_cur_tok};
    use crate::input::InputAbort;

    if st.scanner.scanner_status == NORMAL_COV {
        return Ok(ErrorOutcome::Continue);
    }
    st.errors.deletions_allowed = false;

    // If cur_cs is a control sequence, push it back as a backed-up token.
    if cur_cs() != 0 {
        let ci = cur_input();
        if ci.state == TOKEN_LIST_COV || ci.name < 1 || ci.name > 17 {
            let p = crate::mem::get_avail(&mut st.mem);
            if p == crate::mem::ALLOC_FAILED {
                return Err(InputAbort::MainMemory);
            }
            st.mem.set_b32s0(p, CS_TOKEN_FLAG + cur_cs());
            crate::input::begin_token_list_in(st, p, 3 /* BACKED_UP */)?;
        }
        set_cur_cmd(SPACER);
        set_cur_chr(' ' as i32);
    }

    let oc = if st.scanner.scanner_status > SKIPPING_COV {
        // scanner_status > SKIPPING: runaway — we hit EOF or OUTER inside a definition/macro/preamble.
        crate::errors::runaway_in(st);
        if cur_cs() == 0 {
            crate::output::error_here_with_diagnostic_in(st, c"File ended".as_ptr());
        } else {
            set_cur_cs(0);
            crate::output::error_here_with_diagnostic_in(
                st,
                c"Forbidden control sequence found".as_ptr(),
            );
        }

        let p = crate::mem::get_avail(&mut st.mem);
        if p == crate::mem::ALLOC_FAILED {
            return Err(InputAbort::MainMemory);
        }
        match st.scanner.scanner_status {
            DEFINING_COV => {
                crate::output::print_cstr_in(st, c" while scanning definition".as_ptr());
                st.mem.set_b32s0(p, RIGHT_BRACE_TOKEN_COV + b'}' as i32);
            }
            MATCHING_COV => {
                crate::output::print_cstr_in(st, c" while scanning use".as_ptr());
                let par_token = st.input.par_token;
                st.mem.set_b32s0(p, par_token);
                st.scanner.long_state = OUTER_CALL_COV;
            }
            ALIGNING_COV => {
                crate::output::print_cstr_in(st, c" while scanning preamble".as_ptr());
                st.mem.set_b32s0(p, RIGHT_BRACE_TOKEN_COV + b'}' as i32);
                let q = p;
                let p2 = crate::mem::get_avail(&mut st.mem);
                if p2 == crate::mem::ALLOC_FAILED {
                    return Err(InputAbort::MainMemory);
                }
                st.mem.set_b32s1(p2, q);
                st.mem.set_b32s0(p2, CS_TOKEN_FLAG + FROZEN_CR_COV);
                st.scanner.align_state = -1_000_000;
                crate::input::begin_token_list_in(st, p2, 5 /* INSERTED */)?;
                // For ALIGNING, we push p2 as the token list, not p.
                // Then fall through to the help message.
                crate::output::print_cstr_in(st, c" of ".as_ptr());
                let warning_index = st.scanner.warning_index;
                crate::output::sprint_cs_in(st, warning_index);
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 4;
                st.errors.help_line[3] = c"I suspect you have forgotten a `}', causing me".as_ptr();
                st.errors.help_line[2] = c"to read past where you wanted me to stop.".as_ptr();
                st.errors.help_line[1] =
                    c"I'll try to recover; but if the error is serious,".as_ptr();
                st.errors.help_line[0] =
                    c"you'd better type `E' or `X' now and fix your file.".as_ptr();
                let oc = crate::errors::error_in(st);
                st.errors.deletions_allowed = true;
                return Ok(oc);
            }
            ABSORBING_COV => {
                crate::output::print_cstr_in(st, c" while scanning text".as_ptr());
                st.mem.set_b32s0(p, RIGHT_BRACE_TOKEN_COV + b'}' as i32);
            }
            _ => {}
        }

        crate::input::begin_token_list_in(st, p, 5 /* INSERTED */)?;
        crate::output::print_cstr_in(st, c" of ".as_ptr());
        let warning_index = st.scanner.warning_index;
        crate::output::sprint_cs_in(st, warning_index);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 4;
        st.errors.help_line[3] = c"I suspect you have forgotten a `}', causing me".as_ptr();
        st.errors.help_line[2] = c"to read past where you wanted me to stop.".as_ptr();
        st.errors.help_line[1] = c"I'll try to recover; but if the error is serious,".as_ptr();
        st.errors.help_line[0] = c"you'd better type `E' or `X' now and fix your file.".as_ptr();
        crate::errors::error_in(st)
    } else {
        // scanner_status == SKIPPING: inside \if...\fi skip.
        crate::output::error_here_with_diagnostic_in(st, c"Incomplete ".as_ptr());
        let cur_if = st.scanner.cur_if;
        crate::output::print_cmd_chr_in(st, IF_TEST_COV, cur_if as i32);
        crate::output::print_cstr_in(st, c"; all text was ignored after line ".as_ptr());
        let skip_line = st.scanner.skip_line;
        crate::output::print_int_in(st, skip_line);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 3;
        st.errors.help_line[2] = c"A forbidden control sequence occurred in skipped text.".as_ptr();
        st.errors.help_line[1] =
            c"This kind of error happens when you say `\\if...' and forget".as_ptr();
        st.errors.help_line[0] =
            c"the matching `\\fi'. I've inserted a `\\fi'; this might work.".as_ptr();
        if cur_cs() != 0 {
            set_cur_cs(0);
        } else {
            st.errors.help_line[2] =
                c"The file ended while I was skipping conditional text.".as_ptr();
        }
        set_cur_tok(CS_TOKEN_FLAG + FROZEN_FI_COV);
        crate::input::ins_error_in(st)?
    };
    st.errors.deletions_allowed = true;
    Ok(oc)
}

// ---------------------------------------------------------------------------
// expand
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:3167 (get_next)
pub unsafe fn get_next() {
    match crate::state::with_state_mut(get_next_in) {
        GetNextOutcome::Done => {}
        GetNextOutcome::Error(oc) => oc.finish(),
        GetNextOutcome::Abort(a) => a.fire(),
        GetNextOutcome::NoLegalEnd { open_log } => {
            if open_log {
                open_log_file();
            }
            crate::errors::fatal_error(c"*** (job aborted, no legal \\end found)".as_ptr());
        }
    }
}

/// Deferred cold outcomes of [`get_next_in`] (`01b` §5): each re-enters
/// `with_state` (print / error / log), so it runs in the thin wrapper after the
/// `&mut EngineState` borrow drops. Never produced on a normal token read.
pub(crate) enum GetNextOutcome {
    /// Normal completion — `cur_cmd`/`cur_chr`/`cur_cs` are now set.
    Done,
    /// A recoverable error / outer-validity check produced a terminal outcome.
    Error(crate::errors::ErrorOutcome),
    /// An input-stack cold abort (overflow / fatal / main-memory).
    Abort(crate::input::InputAbort),
    /// `\end`-less job: open the log (unless already open) then `fatal_error`.
    NoLegalEnd { open_log: bool },
}

/// Map a hash `id_lookup` capacity overflow onto the deferred input-abort
/// sentinel so the `overflow(...)` print runs after the borrow drops.
pub(crate) fn hash_err_abort(e: crate::hash::HashLookupError) -> crate::input::InputAbort {
    use crate::hash::HashLookupError::*;
    let (what, n) = match e {
        HashSizeOverflow(n) => (c"hash size".as_ptr(), n),
        PoolSizeOverflow(n) => (c"pool size".as_ptr(), n),
        StringNumberOverflow(n) => (c"number of strings".as_ptr(), n),
        PrimitiveSizeOverflow(n) => (c"primitive size".as_ptr(), n),
    };
    crate::input::InputAbort::Overflow { what, n }
}

/// Threaded `_in` core of [`get_next`] (`01b` §3) — the central tokenizer,
/// holding one `&mut EngineState` borrow across the whole read. Buffer / eqtb /
/// mem / scanner / input / save state is touched directly and the threaded
/// input-stack / error `_in` cores run under the borrow; `cur_input` / `cur_chr`
/// / `cur_cmd` / `cur_cs` / `line` / `first` stay raw `mod raw` (safe under the
/// borrow). Cold abort / recoverable-error / no-legal-`\end` tails ride
/// [`GetNextOutcome`].
pub(crate) fn get_next_in(st: &mut crate::state::EngineState) -> GetNextOutcome {
    use crate::globals::{line, set_line};

    macro_rules! try_in {
        ($e:expr) => {
            match $e {
                Ok(v) => v,
                Err(a) => return GetNextOutcome::Abort(a),
            }
        };
    }

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
                    set_cur_chr(st.input.buffer[(cur_input().loc) as usize]);
                    set_cur_input_loc(cur_input().loc + 1);

                    // Handle UTF-16 surrogate pair
                    if cur_chr() >= 0xD800
                        && cur_chr() < 0xDC00
                        && cur_input().loc <= cur_input().limit
                        && st.input.buffer[(cur_input().loc) as usize] >= 0xDC00
                        && st.input.buffer[(cur_input().loc) as usize] < 0xE000
                    {
                        let lower = st.input.buffer[(cur_input().loc) as usize] - 0xDC00;
                        set_cur_input_loc(cur_input().loc + 1);
                        set_cur_chr(65536 + (cur_chr() - 0xD800) * 1024 + lower);
                    }

                    'reswitch: loop {
                        let cat = st.eqtb.b32s1(CAT_CODE_BASE + cur_chr()) as u8;
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
                                        let first_chr = st.input.buffer[(cs_start) as usize];
                                        let first_cat =
                                            st.eqtb.b32s1(CAT_CODE_BASE + first_chr) as u8;
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
                                            let mut chr = st.input.buffer[(k) as usize];
                                            let mut cat2 = st.eqtb.b32s1(CAT_CODE_BASE + chr) as u8;
                                            k += 1;
                                            while cat2 == LETTER && k <= cur_input().limit {
                                                chr = st.input.buffer[(k) as usize];
                                                cat2 = st.eqtb.b32s1(CAT_CODE_BASE + chr) as u8;
                                                k += 1;
                                            }

                                            // Sup-mark (^^^) handling in multi-letter path
                                            if cat2 == SUP_MARK
                                                && st.input.buffer[(k) as usize] == chr
                                                && k < cur_input().limit
                                            {
                                                let mut sup_count: i32 = 2;
                                                while sup_count < 6
                                                    && k + 2 * sup_count - 2 <= cur_input().limit
                                                    && st.input.buffer[(k + sup_count - 1) as usize]
                                                        == chr
                                                {
                                                    sup_count += 1;
                                                }
                                                let sup_count_save = sup_count;
                                                let mut hex_ok = true;
                                                for d in 1..=sup_count_save {
                                                    if !is_lc_hex(
                                                        st.input.buffer
                                                            [(k + sup_count - 2 + d) as usize],
                                                    ) {
                                                        let c = st.input.buffer[(k + 1) as usize];
                                                        if c < 128 {
                                                            let new_val = if c < 64 {
                                                                c + 64
                                                            } else {
                                                                c - 64
                                                            };
                                                            st.input.buffer[(k - 1) as usize] =
                                                                new_val;
                                                            set_cur_input_limit(
                                                                cur_input().limit - 2,
                                                            );
                                                            let mut kk = k;
                                                            while kk <= cur_input().limit {
                                                                st.input.buffer[(kk) as usize] =
                                                                    st.input.buffer
                                                                        [(kk + 2) as usize];
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
                                                        let c = st.input.buffer
                                                            [(k + sup_count - 2 + d) as usize];
                                                        new_chr = 16 * new_chr
                                                            + if c <= b'9' as i32 {
                                                                c - b'0' as i32
                                                            } else {
                                                                c - b'a' as i32 + 10
                                                            };
                                                    }
                                                    if new_chr <= BIGGEST_USV_GN {
                                                        st.input.buffer[(k - 1) as usize] = new_chr;
                                                        let shift = 2 * sup_count - 1;
                                                        set_cur_input_limit(
                                                            cur_input().limit - shift,
                                                        );
                                                        let mut kk = k;
                                                        while kk <= cur_input().limit {
                                                            st.input.buffer[(kk) as usize] =
                                                                st.input.buffer
                                                                    [(kk + shift) as usize];
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
                                                set_cur_cs(
                                                    match crate::hash::id_lookup(
                                                        &mut st.eqtb,
                                                        &mut st.strings,
                                                        &st.input.buffer,
                                                        cur_input().loc,
                                                        k - cur_input().loc,
                                                    ) {
                                                        Ok(p) => p,
                                                        Err(e) => {
                                                            return GetNextOutcome::Abort(
                                                                hash_err_abort(e),
                                                            )
                                                        }
                                                    },
                                                );
                                                set_cur_input_loc(k);
                                                break 'start_cs; // goto found
                                            }
                                            // Single letter: fall through to single-char handling
                                        } else {
                                            // Single-char (non-LETTER first char) or end of line
                                            if first_cat == SUP_MARK
                                                && st.input.buffer[(k) as usize] == first_chr
                                                && k < cur_input().limit
                                            {
                                                let mut sup_count: i32 = 2;
                                                while sup_count < 6
                                                    && k + 2 * sup_count - 2 <= cur_input().limit
                                                    && st.input.buffer[(k + sup_count - 1) as usize]
                                                        == first_chr
                                                {
                                                    sup_count += 1;
                                                }
                                                let sup_count_save = sup_count;
                                                let mut hex_ok = true;
                                                for d in 1..=sup_count_save {
                                                    if !is_lc_hex(
                                                        st.input.buffer
                                                            [(k + sup_count - 2 + d) as usize],
                                                    ) {
                                                        let c = st.input.buffer[(k + 1) as usize];
                                                        if c < 128 {
                                                            let new_val = if c < 64 {
                                                                c + 64
                                                            } else {
                                                                c - 64
                                                            };
                                                            st.input.buffer[(k - 1) as usize] =
                                                                new_val;
                                                            set_cur_input_limit(
                                                                cur_input().limit - 2,
                                                            );
                                                            let mut kk = k;
                                                            while kk <= cur_input().limit {
                                                                st.input.buffer[(kk) as usize] =
                                                                    st.input.buffer
                                                                        [(kk + 2) as usize];
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
                                                        let c = st.input.buffer
                                                            [(k + sup_count - 2 + d) as usize];
                                                        new_chr = 16 * new_chr
                                                            + if c <= b'9' as i32 {
                                                                c - b'0' as i32
                                                            } else {
                                                                c - b'a' as i32 + 10
                                                            };
                                                    }
                                                    if new_chr <= BIGGEST_USV_GN {
                                                        st.input.buffer[(k - 1) as usize] = new_chr;
                                                        let shift = 2 * sup_count - 1;
                                                        set_cur_input_limit(
                                                            cur_input().limit - shift,
                                                        );
                                                        let mut kk = k;
                                                        while kk <= cur_input().limit {
                                                            st.input.buffer[(kk) as usize] =
                                                                st.input.buffer
                                                                    [(kk + shift) as usize];
                                                            kk += 1;
                                                        }
                                                        continue 'start_cs;
                                                    }
                                                    // else: new_chr > BIGGEST_USV — fall through
                                                }
                                            }
                                        }

                                        // Single-char CS: either buffer > 65535 uses id_lookup(1), else SINGLE_BASE
                                        if st.input.buffer[(cur_input().loc) as usize] > 65535 {
                                            set_cur_cs(
                                                match crate::hash::id_lookup(
                                                    &mut st.eqtb,
                                                    &mut st.strings,
                                                    &st.input.buffer,
                                                    cur_input().loc,
                                                    1,
                                                ) {
                                                    Ok(p) => p,
                                                    Err(e) => {
                                                        return GetNextOutcome::Abort(
                                                            hash_err_abort(e),
                                                        )
                                                    }
                                                },
                                            );
                                            set_cur_input_loc(cur_input().loc + 1);
                                            break 'start_cs; // goto found
                                        }
                                        set_cur_cs(
                                            SINGLE_BASE
                                                + st.input.buffer[(cur_input().loc) as usize],
                                        );
                                        set_cur_input_loc(cur_input().loc + 1);
                                        break 'start_cs;
                                    }
                                }
                                // found:
                                set_cur_cmd(st.eqtb.b16s1(cur_cs()) as u8);
                                set_cur_chr(st.eqtb.b32s1(cur_cs()));
                                if cur_cmd() >= OUTER_CALL_GN {
                                    match check_outer_validity_in(st) {
                                        Ok(crate::errors::ErrorOutcome::Continue) => {}
                                        Ok(oc) => return GetNextOutcome::Error(oc),
                                        Err(a) => return GetNextOutcome::Abort(a),
                                    }
                                }
                            }

                            // ANY_STATE_PLUS(ACTIVE_CHAR): active character
                            14 | 30 | 46 => {
                                set_cur_cs(cur_chr() + 1);
                                set_cur_cmd(st.eqtb.b16s1(cur_cs()) as u8);
                                set_cur_chr(st.eqtb.b32s1(cur_cs()));
                                set_cur_input_state(MID_LINE);
                                if cur_cmd() >= OUTER_CALL_GN {
                                    match check_outer_validity_in(st) {
                                        Ok(crate::errors::ErrorOutcome::Continue) => {}
                                        Ok(oc) => return GetNextOutcome::Error(oc),
                                        Err(a) => return GetNextOutcome::Abort(a),
                                    }
                                }
                            }

                            // ANY_STATE_PLUS(SUP_MARK): caret (^^) expansion
                            8 | 24 | 40 => {
                                if cur_chr() == st.input.buffer[(cur_input().loc) as usize]
                                    && cur_input().loc < cur_input().limit
                                {
                                    let mut sup_count: i32 = 2;
                                    while sup_count < 6
                                        && cur_input().loc + 2 * sup_count - 2 <= cur_input().limit
                                        && cur_chr()
                                            == st.input.buffer
                                                [(cur_input().loc + sup_count - 1) as usize]
                                    {
                                        sup_count += 1;
                                    }
                                    let sup_count_save = sup_count;
                                    let mut hex_ok = true;
                                    for d in 1..=sup_count_save {
                                        if !is_lc_hex(
                                            st.input.buffer
                                                [(cur_input().loc + sup_count - 2 + d) as usize],
                                        ) {
                                            let c = st.input.buffer[(cur_input().loc + 1) as usize];
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
                                            let c = st.input.buffer
                                                [(cur_input().loc + sup_count - 2 + d) as usize];
                                            new_chr = 16 * new_chr
                                                + if c <= b'9' as i32 {
                                                    c - b'0' as i32
                                                } else {
                                                    c - b'a' as i32 + 10
                                                };
                                        }
                                        if new_chr > BIGGEST_USV_GN {
                                            set_cur_chr(
                                                st.input.buffer[(cur_input().loc) as usize],
                                            );
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
                                crate::output::error_here_with_diagnostic_in(
                                    st,
                                    c"Text line contains an invalid character".as_ptr(),
                                );
                                crate::output::capture_to_diagnostic_in(st, core::ptr::null_mut());
                                st.errors.help_ptr = 2;
                                st.errors.help_line[(1) as usize] =
                                    c"A funny symbol that I can't read has just been input."
                                        .as_ptr();
                                st.errors.help_line[(0) as usize] =
                                    c"Continue, and I'll forget that it ever happened.".as_ptr();
                                st.errors.deletions_allowed = false;
                                let oc = crate::errors::error_in(st);
                                if !matches!(oc, crate::errors::ErrorOutcome::Continue) {
                                    return GetNextOutcome::Error(oc);
                                }
                                st.errors.deletions_allowed = true;
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
                                set_cur_cs(st.input.par_loc);
                                set_cur_cmd(st.eqtb.b16s1(cur_cs()) as u8);
                                set_cur_chr(st.eqtb.b32s1(cur_cs()));
                                if cur_cmd() >= OUTER_CALL_GN {
                                    match check_outer_validity_in(st) {
                                        Ok(crate::errors::ErrorOutcome::Continue) => {}
                                        Ok(oc) => return GetNextOutcome::Error(oc),
                                        Err(a) => return GetNextOutcome::Abort(a),
                                    }
                                }
                            }

                            // MID_LINE + LEFT_BRACE
                            2 => {
                                st.scanner.align_state = st.scanner.align_state + 1;
                            }

                            // SKIP_BLANKS + LEFT_BRACE | NEW_LINE + LEFT_BRACE
                            18 | 34 => {
                                set_cur_input_state(MID_LINE);
                                st.scanner.align_state = st.scanner.align_state + 1;
                            }

                            // MID_LINE + RIGHT_BRACE
                            3 => {
                                st.scanner.align_state = st.scanner.align_state - 1;
                            }

                            // SKIP_BLANKS + RIGHT_BRACE | NEW_LINE + RIGHT_BRACE
                            19 | 35 => {
                                set_cur_input_state(MID_LINE);
                                st.scanner.align_state = st.scanner.align_state - 1;
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

                        if !st.input.force_eof {
                            if cur_input().name <= 19 {
                                if try_in!(crate::input::pseudo_input_in(st)) {
                                    set_cur_input_limit(st.input.last);
                                } else if st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_EOF) != TEX_NULL
                                    && !st.input.eof_seen[(cur_input().index as i32) as usize]
                                {
                                    set_cur_input_limit(first() - 1);
                                    st.input.eof_seen[(cur_input().index as i32) as usize] = true;
                                    {
                                        let __p = st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_EOF);
                                        try_in!(crate::input::begin_token_list_in(
                                            st,
                                            __p,
                                            EVERY_EOF_TEXT_GN
                                        ))
                                    };
                                    continue 'restart;
                                } else {
                                    st.input.force_eof = true;
                                }
                            } else if {
                                let __f = st.input.input_file[(cur_input().index as i32) as usize];
                                try_in!(unsafe { crate::io::input_line_in(st, __f) })
                            } != 0
                            {
                                set_cur_input_limit(st.input.last);
                            } else if st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_EOF) != TEX_NULL
                                && !st.input.eof_seen[(cur_input().index as i32) as usize]
                            {
                                set_cur_input_limit(first() - 1);
                                st.input.eof_seen[(cur_input().index as i32) as usize] = true;
                                {
                                    let __p = st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_EOF);
                                    try_in!(crate::input::begin_token_list_in(
                                        st,
                                        __p,
                                        EVERY_EOF_TEXT_GN
                                    ))
                                };
                                continue 'restart;
                            } else {
                                st.input.force_eof = true;
                            }
                        }

                        if st.input.force_eof {
                            if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_NESTING) > 0
                                && (st.input.grp_stack[(st.input.in_open) as usize]
                                    != st.save_stack.cur_boundary
                                    || st.input.if_stack[(st.input.in_open) as usize]
                                        != st.scanner.cond_ptr)
                            {
                                crate::output::file_warning_in(st);
                            }
                            if cur_input().name >= 19 {
                                crate::output::print_char_in(st, b')' as i32);
                                st.input.open_parens = st.input.open_parens - 1;
                                unsafe { ttstub_output_flush(st.shipout.rust_stdout) };
                            }
                            st.input.force_eof = false;
                            crate::input::end_file_reading_in(st);
                            match check_outer_validity_in(st) {
                                Ok(crate::errors::ErrorOutcome::Continue) => {}
                                Ok(oc) => return GetNextOutcome::Error(oc),
                                Err(a) => return GetNextOutcome::Abort(a),
                            }
                            continue 'restart;
                        }

                        let end_char = st.eqtb.b32s1(INT_BASE + INT_PAR_END_LINE_CHAR_GN);
                        if !(0..=255).contains(&end_char) {
                            set_cur_input_limit(cur_input().limit - 1);
                        } else {
                            st.input.buffer[(cur_input().limit) as usize] = end_char;
                        }
                        set_first(cur_input().limit + 1);
                        set_cur_input_loc(cur_input().start);
                    } else {
                        if cur_input().name != 0 {
                            set_cur_cmd(0);
                            set_cur_chr(0);
                            return GetNextOutcome::Done;
                        }
                        if st.input.input_ptr > 0 {
                            crate::input::end_file_reading_in(st);
                            continue 'restart;
                        }
                        if !st.scanner.used_tectonic_coda_tokens
                            && st.eqtb.b32s1(LOCAL_BASE + LOCAL_TECTONIC_CODA) != TEX_NULL
                        {
                            st.scanner.used_tectonic_coda_tokens = true;
                            {
                                let __p = st.eqtb.b32s1(LOCAL_BASE + LOCAL_TECTONIC_CODA);
                                try_in!(crate::input::begin_token_list_in(
                                    st,
                                    __p,
                                    TECTONIC_CODA_TEXT_GN
                                ))
                            };
                            continue 'restart;
                        }
                        return GetNextOutcome::NoLegalEnd {
                            open_log: st.shipout.selector < SELECTOR_LOG_ONLY_GN,
                        };
                    }
                    continue 'texswitch;
                }
            }
        } else if cur_input().loc != TEX_NULL {
            // Reading from a token list
            let t = st.mem.b32s0(cur_input().loc);
            set_cur_input_loc(st.mem.b32s1(cur_input().loc)); // LLIST_link

            if t >= CS_TOKEN_FLAG {
                set_cur_cs(t - CS_TOKEN_FLAG);
                set_cur_cmd(st.eqtb.b16s1(cur_cs()) as u8);
                set_cur_chr(st.eqtb.b32s1(cur_cs()));
                if cur_cmd() >= OUTER_CALL_GN {
                    if cur_cmd() == DONT_EXPAND_GN {
                        set_cur_cs(st.mem.b32s0(cur_input().loc) - CS_TOKEN_FLAG);
                        set_cur_input_loc(TEX_NULL);
                        set_cur_cmd(st.eqtb.b16s1(cur_cs()) as u8);
                        set_cur_chr(st.eqtb.b32s1(cur_cs()));
                        if cur_cmd() > MAX_COMMAND_GN {
                            set_cur_cmd(RELAX_GN);
                            set_cur_chr(NO_EXPAND_FLAG_GN);
                        }
                    } else {
                        match check_outer_validity_in(st) {
                            Ok(crate::errors::ErrorOutcome::Continue) => {}
                            Ok(oc) => return GetNextOutcome::Error(oc),
                            Err(a) => return GetNextOutcome::Abort(a),
                        }
                    }
                }
            } else {
                set_cur_cmd((t / MAX_CHAR_VAL_GN) as u8);
                set_cur_chr(t % MAX_CHAR_VAL_GN);
                match cur_cmd() {
                    x if x == LEFT_BRACE_CAT => {
                        st.scanner.align_state = st.scanner.align_state + 1;
                    }
                    x if x == RIGHT_BRACE_CAT => {
                        st.scanner.align_state = st.scanner.align_state - 1;
                    }
                    x if x == OUT_PARAM_GN => {
                        {
                            let __p =
                                st.input.param_stack[(cur_input().limit + cur_chr() - 1) as usize];
                            try_in!(crate::input::begin_token_list_in(st, __p, PARAMETER_GN))
                        };
                        continue 'restart;
                    }
                    _ => {}
                }
            }
        } else {
            // Token list exhausted
            try_in!(crate::input::end_token_list_in(st));
            continue 'restart;
        }

        // Alignment preamble check (§818)
        if cur_cmd() <= CAR_RET && cur_cmd() >= TAB_MARK_CAT && st.scanner.align_state == 0 {
            if st.scanner.scanner_status == ALIGNING_GN || st.scanner.cur_align == TEX_NULL {
                return GetNextOutcome::Abort(crate::input::InputAbort::Fatal(
                    c"(interwoven alignment preambles are not allowed)".as_ptr(),
                ));
            }
            let saved_cmd = st.mem.b32s0(st.scanner.cur_align + 5);
            st.mem.set_b32s0(st.scanner.cur_align + 5, cur_chr());
            if saved_cmd as u8 == OMIT_GN {
                {
                    let __p = OMIT_TEMPLATE_GN;
                    try_in!(crate::input::begin_token_list_in(st, __p, V_TEMPLATE_GN))
                };
            } else {
                {
                    let __p = st.mem.b32s1(st.scanner.cur_align + 2);
                    try_in!(crate::input::begin_token_list_in(st, __p, V_TEMPLATE_GN))
                };
            }
            set_cur_cmd(saved_cmd as u8);
            st.scanner.align_state = 1_000_000;
            continue 'restart;
        }
        break 'restart;
    }
    GetNextOutcome::Done
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

pub(crate) fn push_font_info_b32s1(v: i32) {
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

/// Deferred cold outcomes of the scanner / `expand` SCC `_in` cores (`01b` §5).
/// Every SCC `_in` returns `Result<T, SccAbort>`; sibling calls propagate with
/// `?`. Each arm re-enters `with_state` (print / error / input-stack / log), so
/// it runs in the thin wrapper *after* the `&mut EngineState` borrow drops.
/// Never produced on a normal scan.
pub(crate) enum SccAbort {
    /// A recoverable `error()`/`int_error()` produced a terminal outcome.
    Error(crate::errors::ErrorOutcome),
    /// An input-stack / tokenizer cold abort.
    Input(crate::input::InputAbort),
    /// `\end`-less job: open the log (unless already open) then `fatal_error`.
    NoLegalEnd { open_log: bool },
    /// `tokens_to_string` cold tail (reached only from `compare_strings`).
    TokensToString(TokensToStringAbort),
    /// A `pdf_error(t, p)` cold path (reached from `conv_toks`).
    PdfError {
        t: *const libc::c_char,
        p: *const libc::c_char,
    },
    /// An `open_log_file` cold abort (reached from `conv_toks` `\jobname`).
    OpenLog(crate::io::OpenLogAbort),
    /// A `maketexstring` overflow (reached from `start_input`).
    MakeTexString(crate::texmfmp::MakeTexStringAbort),
}

impl SccAbort {
    /// Run the deferred cold tail. Must be called *after* the borrow drops.
    pub(crate) unsafe fn fire(self) -> ! {
        match self {
            SccAbort::Error(oc) => {
                oc.finish();
                unreachable!("ErrorOutcome::finish returned for a stored (non-Continue) outcome")
            }
            SccAbort::Input(a) => a.fire(),
            SccAbort::NoLegalEnd { open_log } => {
                if open_log {
                    open_log_file();
                }
                crate::errors::fatal_error(c"*** (job aborted, no legal \\end found)".as_ptr());
                unreachable!("fatal_error returned")
            }
            SccAbort::TokensToString(a) => match a {
                TokensToStringAbort::SelectorNewString => crate::errors::pdf_error(
                    c"tokens".as_ptr(),
                    c"tokens_to_string() called while selector = new_string".as_ptr(),
                ),
                TokensToStringAbort::StringOverflow => crate::errors::overflow(
                    c"number of strings".as_ptr(),
                    crate::globals::max_strings() - crate::globals::init_str_ptr(),
                ),
            },
            SccAbort::PdfError { t, p } => crate::errors::pdf_error(t, p),
            SccAbort::OpenLog(e) => e.fire(),
            SccAbort::MakeTexString(e) => crate::texmfmp::fire_make_tex_string(e),
        }
    }
}

impl From<GetNextOutcome> for Result<(), SccAbort> {
    fn from(o: GetNextOutcome) -> Self {
        match o {
            GetNextOutcome::Done => Ok(()),
            GetNextOutcome::Error(oc) => Err(SccAbort::Error(oc)),
            GetNextOutcome::Abort(a) => Err(SccAbort::Input(a)),
            GetNextOutcome::NoLegalEnd { open_log } => Err(SccAbort::NoLegalEnd { open_log }),
        }
    }
}

impl From<crate::input::InputAbort> for SccAbort {
    fn from(a: crate::input::InputAbort) -> Self {
        SccAbort::Input(a)
    }
}

/// Propagate a recoverable-error outcome (`error_in`/`int_error_in`/…) as an
/// `SccAbort`: `Continue` keeps the scan going, anything else became terminal
/// under the borrow and must fire after it drops. Use as `scc_err(o)?;`.
pub(crate) fn scc_err(o: crate::errors::ErrorOutcome) -> Result<(), SccAbort> {
    if matches!(o, crate::errors::ErrorOutcome::Continue) {
        Ok(())
    } else {
        Err(SccAbort::Error(o))
    }
}

/// `get_next` under the borrow, with its [`GetNextOutcome`] cold tails lifted
/// into [`SccAbort`]. Use as `get_next_scc(st)?;` inside the SCC.
pub(crate) fn get_next_scc(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    Result::from(get_next_in(st))
}

/// The deferred `overflow("main memory size", MEM_TOP + 1)` tail of the node
/// allocators (`get_node`/`new_spec` `ALLOC_FAILED`), lifted into [`SccAbort`].
pub(crate) fn main_mem_overflow_scc() -> SccAbort {
    SccAbort::Input(crate::input::InputAbort::Overflow {
        what: c"main memory size".as_ptr(),
        n: 4_999_999 + 1,
    })
}

/// `get_node` under the borrow with its `ALLOC_FAILED` lifted into [`SccAbort`].
pub(crate) fn get_node_scc(st: &mut crate::state::EngineState, s: i32) -> Result<i32, SccAbort> {
    let r = crate::mem::get_node(&mut st.mem, s);
    if r == crate::mem::ALLOC_FAILED {
        return Err(main_mem_overflow_scc());
    }
    Ok(r)
}

/// `get_avail` under the borrow with its `ALLOC_FAILED` lifted into
/// [`SccAbort`] (`get_avail_global` runs `runaway()` then `overflow`).
pub(crate) fn get_avail_scc(st: &mut crate::state::EngineState) -> Result<i32, SccAbort> {
    let r = crate::mem::get_avail(&mut st.mem);
    if r == crate::mem::ALLOC_FAILED {
        return Err(SccAbort::Input(crate::input::InputAbort::MainMemory));
    }
    Ok(r)
}

/// `new_spec` under the borrow with its `ALLOC_FAILED` lifted into [`SccAbort`].
pub(crate) fn new_spec_scc(st: &mut crate::state::EngineState, p: i32) -> Result<i32, SccAbort> {
    let r = crate::mem::new_spec(&mut st.mem, p);
    if r == crate::mem::ALLOC_FAILED {
        return Err(main_mem_overflow_scc());
    }
    Ok(r)
}

/// `str_toks_cat` under the borrow with its [`StrToksAbort`] cold overflow tails
/// lifted into [`SccAbort`] (via [`crate::input::InputAbort`]).
pub(crate) fn str_toks_scc(
    st: &mut crate::state::EngineState,
    b: i32,
    cat: i16,
) -> Result<i32, SccAbort> {
    match str_toks_cat_in(&mut st.mem, &mut st.strings, b, cat) {
        Ok(p) => Ok(p),
        Err(StrToksAbort::PoolOverflow) => {
            Err(SccAbort::Input(crate::input::InputAbort::Overflow {
                what: c"pool size".as_ptr(),
                n: st.strings.pool_size - st.strings.init_pool_ptr,
            }))
        }
        Err(StrToksAbort::MainMemoryOverflow) => {
            Err(SccAbort::Input(crate::input::InputAbort::MainMemory))
        }
    }
}

/// Map a hash `id_lookup` capacity overflow onto the deferred input-abort
/// sentinel so the `overflow(...)` print runs after the borrow drops.

// scanner / `expand` SCC — threaded `_in` cores (session 204 big pass).
// Stubs first (manifest §d step 1); bodies translated from the global versions
// below, sibling calls propagate via `?`, cold tails ride `SccAbort`.
// ---------------------------------------------------------------------------

/// Shared tail of the leaf `scan_int` range-check members: emit the "Bad …"
/// diagnostic for an out-of-range `cur_val`, then reset it to zero. Mirrors the
/// global leaves (`scan_char_num` & kin); `help1` is the variable second help
/// line (constant for most, `max_reg_help_line` for `scan_register_num`).
fn leaf_int_range_error(
    st: &mut crate::state::EngineState,
    err_msg: *const libc::c_char,
    help1: *const libc::c_char,
) -> Result<(), SccAbort> {
    let errmsg = error_here_with_diagnostic_in(st, err_msg);
    unsafe { ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val()) };
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 2;
    st.errors.help_line[1] = help1;
    st.errors.help_line[0] = c"I changed this one to zero.".as_ptr();
    scc_err(int_error_in(st, cur_val()))?;
    set_cur_val(0);
    Ok(())
}

pub(crate) fn x_token_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    while cur_cmd() > MAX_COMMAND {
        expand_in(st)?;
        get_next_scc(st)?;
    }
    if cur_cs() == 0 {
        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
    } else {
        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
    }
    Ok(())
}
pub(crate) fn get_x_token_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    loop {
        get_next_scc(st)?;
        if cur_cmd() <= MAX_COMMAND {
            break;
        }
        if cur_cmd() >= CALL {
            if cur_cmd() < END_TEMPLATE {
                macro_call_in(st)?;
            } else {
                set_cur_cs(FROZEN_ENDV);
                set_cur_cmd(ENDV);
                break;
            }
        } else {
            expand_in(st)?;
        }
    }
    if cur_cs() == 0 {
        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
    } else {
        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
    }
    Ok(())
}
pub(crate) fn the_toks_in(st: &mut crate::state::EngineState) -> Result<i32, SccAbort> {
    use crate::mem::TEX_NULL;

    if cur_chr() & 1 != 0 {
        let c = cur_chr();
        scan_general_text_in(st)?;
        if c == 1 {
            return Ok(cur_val());
        } else {
            let old_setting = st.shipout.selector;
            st.shipout.selector = SELECTOR_NEW_STRING;
            let b = st.strings.pool_ptr;
            let p = crate::mem::get_avail(&mut st.mem);
            let th = st.mem.b32s1(TEMP_HEAD);
            st.mem.set_b32s1(p, th);
            crate::output::token_show_in(st, p);
            crate::mem::flush_list(&mut st.mem, p);
            st.shipout.selector = old_setting;
            return str_toks_scc(st, b, 0);
        }
    }
    get_x_token_in(st)?;
    scan_something_internal_in(st, TOK_VAL, false)?;
    if st.scanner.cur_val_level >= IDENT_VAL as u8 {
        let mut p = TEMP_HEAD;
        st.mem.set_b32s1(p, TEX_NULL);
        if st.scanner.cur_val_level == IDENT_VAL as u8 {
            let q = crate::mem::get_avail(&mut st.mem);
            st.mem.set_b32s1(p, q);
            st.mem.set_b32s0(q, CS_TOKEN_FLAG + cur_val());
            p = q;
        } else if cur_val() != TEX_NULL {
            let mut r = st.mem.b32s1(cur_val());
            while r != TEX_NULL {
                let q = if st.mem.avail == TEX_NULL {
                    crate::mem::get_avail(&mut st.mem)
                } else {
                    let qa = st.mem.avail;
                    st.mem.avail = st.mem.b32s1(qa);
                    st.mem.set_b32s1(qa, TEX_NULL);
                    qa
                };
                st.mem.set_b32s1(p, q);
                let v = st.mem.b32s0(r);
                st.mem.set_b32s0(q, v);
                p = q;
                r = st.mem.b32s1(r);
            }
        }
        Ok(p)
    } else {
        let old_setting = st.shipout.selector;
        st.shipout.selector = SELECTOR_NEW_STRING;
        let b = st.strings.pool_ptr;
        match st.scanner.cur_val_level {
            0 => crate::output::print_int_in(st, cur_val()),
            1 => {
                crate::output::print_scaled_in(st, cur_val());
                crate::output::print_cstr_in(st, c"pt".as_ptr());
            }
            2 => {
                crate::output::print_spec_in(st, cur_val(), c"pt".as_ptr());
                crate::mem::delete_glue_ref(&mut st.mem, cur_val());
            }
            3 => {
                crate::output::print_spec_in(st, cur_val(), c"mu".as_ptr());
                crate::mem::delete_glue_ref(&mut st.mem, cur_val());
            }
            _ => {}
        }
        st.shipout.selector = old_setting;
        str_toks_scc(st, b, 0)
    }
}
pub(crate) fn scan_toks_in(
    st: &mut crate::state::EngineState,
    macro_def: bool,
    xpand: bool,
) -> Result<i32, SccAbort> {
    use crate::mem::TEX_NULL;

    if macro_def {
        st.scanner.scanner_status = DEFINING;
    } else {
        st.scanner.scanner_status = ABSORBING;
    }
    st.scanner.warning_index = cur_cs();
    let new_def = crate::mem::get_avail(&mut st.mem);
    st.scanner.def_ref = new_def;
    st.mem.set_b32s0(new_def, TEX_NULL);
    let mut p = new_def;
    let mut hash_brace: i32 = 0;
    let mut t = ZERO_TOKEN;

    // 'found: wraps the whole function body so any goto found → break 'found
    'found: {
        if macro_def {
            // Scan parameter text (§493–495)
            let reached_done = 'param_scan: loop {
                loop {
                    get_token_in(st)?;
                    if cur_tok() < RIGHT_BRACE_LIMIT {
                        break 'param_scan false; // goto done1
                    }
                    if cur_cmd() == MAC_PARAM {
                        // §495
                        let s = MATCH_TOKEN + cur_chr();
                        get_token_in(st)?;
                        if cur_tok() < LEFT_BRACE_LIMIT {
                            hash_brace = cur_tok();
                            let q = crate::mem::get_avail(&mut st.mem);
                            st.mem.set_b32s1(p, q);
                            st.mem.set_b32s0(q, cur_tok());
                            p = q;
                            let q2 = crate::mem::get_avail(&mut st.mem);
                            st.mem.set_b32s1(p, q2);
                            st.mem.set_b32s0(q2, END_MATCH_TOKEN);
                            p = q2;
                            break 'param_scan true; // goto done
                        }
                        if t == ZERO_TOKEN + 9 {
                            error_here_with_diagnostic_in(
                                st,
                                c"You already have nine parameters".as_ptr(),
                            );
                            capture_to_diagnostic_in(st, std::ptr::null_mut());
                            st.errors.help_ptr = 2;
                            st.errors.help_line[1] =
                                c"I'm going to ignore the # sign you just used,".as_ptr();
                            st.errors.help_line[0] =
                                c"as well as the token that followed it.".as_ptr();
                            scc_err(crate::errors::error_in(st))?;
                            continue; // back to top of inner loop
                        } else {
                            t += 1;
                            if cur_tok() != t {
                                error_here_with_diagnostic_in(
                                    st,
                                    c"Parameters must be numbered consecutively".as_ptr(),
                                );
                                st.errors.help_ptr = 2;
                                st.errors.help_line[1] =
                                    c"I've inserted the digit you should have used after the #."
                                        .as_ptr();
                                st.errors.help_line[0] =
                                    c"Type `1' to delete what you did use.".as_ptr();
                                let oc = crate::input::back_error_in(st)?;
                                scc_err(oc)?;
                            }
                            set_cur_tok(s);
                        }
                    }
                    // append cur_tok to list
                    let q = crate::mem::get_avail(&mut st.mem);
                    st.mem.set_b32s1(p, q);
                    st.mem.set_b32s0(q, cur_tok());
                    p = q;
                }
            };
            // done1: append END_MATCH_TOKEN
            if !reached_done {
                let q = crate::mem::get_avail(&mut st.mem);
                st.mem.set_b32s1(p, q);
                st.mem.set_b32s0(q, END_MATCH_TOKEN);
                p = q;
                // §494: check for missing {
                if cur_cmd() == RIGHT_BRACE {
                    error_here_with_diagnostic_in(st, c"Missing { inserted".as_ptr());
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.scanner.align_state += 1;
                    st.errors.help_ptr = 2;
                    st.errors.help_line[1] =
                        c"Where was the left brace? You said something like `\\def\\a}',".as_ptr();
                    st.errors.help_line[0] =
                        c"which I'm going to interpret as `\\def\\a{}'.".as_ptr();
                    scc_err(crate::errors::error_in(st))?;
                    break 'found; // goto found
                }
            }
            // done: fall through to body scan
        } else {
            scan_left_brace_in(st)?;
        }

        // Scan replacement/absorbed text (§497–499)
        let mut unbalance: i32 = 1;
        loop {
            if xpand {
                // §497: expand, but handle \the specially
                loop {
                    get_next_scc(st)?;
                    if cur_cmd() >= CALL && st.mem.b32s0(st.mem.b32s1(cur_chr())) == PROTECTED_TOKEN
                    {
                        set_cur_cmd(RELAX);
                        set_cur_chr(NO_EXPAND_FLAG);
                    }
                    if cur_cmd() <= MAX_COMMAND {
                        break; // done2
                    }
                    if cur_cmd() != THE {
                        expand_in(st)?;
                    } else {
                        let q = the_toks_in(st)?;
                        let th = st.mem.b32s1(TEMP_HEAD);
                        if th != TEX_NULL {
                            st.mem.set_b32s1(p, th);
                            p = q;
                        }
                    }
                }
                // done2:
                x_token_in(st)?;
            } else {
                get_token_in(st)?;
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
                    get_x_token_in(st)?;
                } else {
                    get_token_in(st)?;
                }
                if cur_cmd() != MAC_PARAM {
                    if cur_tok() <= ZERO_TOKEN || cur_tok() > t {
                        error_here_with_diagnostic_in(
                            st,
                            c"Illegal parameter number in definition of ".as_ptr(),
                        );
                        crate::output::sprint_cs_in(st, st.scanner.warning_index);
                        capture_to_diagnostic_in(st, std::ptr::null_mut());
                        st.errors.help_ptr = 3;
                        st.errors.help_line[2] =
                            c"You meant to type ## instead of #, right?".as_ptr();
                        st.errors.help_line[1] =
                            c"Or maybe a } was forgotten somewhere earlier, and things".as_ptr();
                        st.errors.help_line[0] =
                            c"are all screwed up? I'm going to assume that you meant ##.".as_ptr();
                        let oc = crate::input::back_error_in(st)?;
                        scc_err(oc)?;
                        set_cur_tok(s);
                    } else {
                        set_cur_tok((OUT_PARAM_TOKEN - 48) + cur_chr());
                    }
                }
            }
            // append cur_tok to list
            let q = crate::mem::get_avail(&mut st.mem);
            st.mem.set_b32s1(p, q);
            st.mem.set_b32s0(q, cur_tok());
            p = q;
        }
    }
    // found:
    st.scanner.scanner_status = NORMAL_SCANNER;
    if hash_brace != 0 {
        let q = crate::mem::get_avail(&mut st.mem);
        st.mem.set_b32s1(p, q);
        st.mem.set_b32s0(q, hash_brace);
        p = q;
    }
    Ok(p)
}
pub(crate) fn pass_text_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = SKIPPING;
    let mut l: i32 = 0;
    st.scanner.skip_line = crate::globals::line();

    loop {
        get_next_scc(st)?;
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
    st.scanner.scanner_status = save_scanner_status;
    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_IFS) > 0 {
        crate::output::show_cur_cmd_chr_in(st);
    }
    Ok(())
}
pub(crate) fn pseudo_start_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_general_text_in(st)?;
    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    crate::output::token_show_in(st, TEMP_HEAD);
    st.shipout.selector = old_setting;
    let head = st.mem.b32s1(TEMP_HEAD);
    crate::mem::flush_list(&mut st.mem, head);

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(SccAbort::Error(crate::errors::ErrorOutcome::Overflow {
            what: c"pool size",
            n: st.strings.pool_size - st.strings.init_pool_ptr,
        }));
    }
    let s = match crate::string_pool::make_string(&mut st.strings) {
        Ok(s) => s,
        Err(_) => {
            return Err(SccAbort::Error(crate::errors::ErrorOutcome::Overflow {
                what: c"number of strings",
                n: st.strings.max_strings - st.strings.init_str_ptr,
            }))
        }
    };
    let pp = st.strings.pool_ptr;
    st.strings.str_pool[pp as usize] = b' ' as u16;
    let mut l = st.strings.str_start[(s - TOO_BIG_CHAR) as usize];
    let nl = st.eqtb.b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR);
    let p = crate::mem::get_avail(&mut st.mem);
    let mut q = p;

    while l < st.strings.pool_ptr {
        let m = l;
        while l < st.strings.pool_ptr && st.strings.str_pool[l as usize] != nl as u16 {
            l += 1;
        }
        let mut sz = (l - m + 7) / 4;
        if sz == 1 {
            sz = 2;
        }
        let r_base = crate::mem::get_node(&mut st.mem, sz);
        st.mem.set_b32s1(q, r_base);
        q = r_base;
        st.mem.set_b32s0(q, sz);
        let mut r = r_base;
        let mut mm = m;
        while sz > 2 {
            sz -= 1;
            r += 1;
            let s3 = st.strings.str_pool[mm as usize];
            let s2 = st.strings.str_pool[(mm + 1) as usize];
            let s1 = st.strings.str_pool[(mm + 2) as usize];
            let s0 = st.strings.str_pool[(mm + 3) as usize];
            st.mem.set_b16s3(r, s3);
            st.mem.set_b16s2(r, s2);
            st.mem.set_b16s1(r, s1);
            st.mem.set_b16s0(r, s0);
            mm += 4;
        }
        // last word: pad with spaces, then fill from remaining pool bytes
        let mut w3: u16 = b' ' as u16;
        let mut w2: u16 = b' ' as u16;
        let mut w1: u16 = b' ' as u16;
        let mut w0: u16 = b' ' as u16;
        if l > mm {
            w3 = st.strings.str_pool[mm as usize];
            if l > mm + 1 {
                w2 = st.strings.str_pool[(mm + 1) as usize];
                if l > mm + 2 {
                    w1 = st.strings.str_pool[(mm + 2) as usize];
                    if l > mm + 3 {
                        w0 = st.strings.str_pool[(mm + 3) as usize];
                    }
                }
            }
        }
        st.mem.set_b16s3(r + 1, w3);
        st.mem.set_b16s2(r + 1, w2);
        st.mem.set_b16s1(r + 1, w1);
        st.mem.set_b16s0(r + 1, w0);
        if st.strings.str_pool[l as usize] == nl as u16 {
            l += 1;
        }
    }

    let pl = st.mem.b32s1(p);
    st.mem.set_b32s0(p, pl);
    st.mem.set_b32s1(p, st.input.pseudo_files);
    st.input.pseudo_files = p;
    // pop the temporary string (str_ptr--, pool_ptr = str_start[str_ptr - TOO_BIG_CHAR])
    st.strings.str_ptr -= 1;
    st.strings.pool_ptr = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];

    crate::input::begin_file_reading_in(st)?;
    set_line(0);
    let start = cur_input().start;
    set_cur_input_limit(start);
    set_cur_input_loc(start + 1);

    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_SCAN_TOKENS) > 0 {
        if st.shipout.term_offset > st.config.max_print_line - 3 {
            crate::output::print_ln_in(st);
        } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
            crate::output::print_char_in(st, b' ' as i32);
        }
        set_cur_input_name(19);
        crate::output::print_cstr_in(st, c"( ".as_ptr());
        st.input.open_parens += 1;
        unsafe { ttstub_output_flush(st.shipout.rust_stdout) };
    } else {
        set_cur_input_name(18);
        set_cur_input_synctex_tag(0);
    }
    Ok(())
}
pub(crate) fn scan_int_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let mut negative = false;
    let mut m: i32 = 0;
    let mut d: i32 = 0;
    let mut vacuous = false;
    let mut ok_so_far = true;

    st.scanner.radix = 0;

    // Skip spaces and absorb sign prefix(es).
    loop {
        loop {
            get_x_token_in(st)?;
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
            get_token_in(st)?;
            if cur_tok() < CS_TOKEN_FLAG {
                set_cur_val(cur_chr());
                if cur_cmd() <= 2 {
                    // RIGHT_BRACE = 2 (xetex_format.h:232)
                    if cur_cmd() == 2 {
                        st.scanner.align_state += 1;
                    } else {
                        st.scanner.align_state -= 1;
                    }
                }
            } else if cur_tok() < CS_TOKEN_FLAG + SINGLE_BASE {
                set_cur_val(cur_tok() - (CS_TOKEN_FLAG + ACTIVE_BASE));
            } else {
                set_cur_val(cur_tok() - (CS_TOKEN_FLAG + SINGLE_BASE));
            }
            if cur_val() > BIGGEST_USV {
                error_here_with_diagnostic_in(st, c"Improper alphabetic constant".as_ptr());
                capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"A one-character control sequence belongs after a ` mark.".as_ptr();
                st.errors.help_line[0] = c"So I'm essentially inserting \\0 here.".as_ptr();
                set_cur_val(b'0' as i32);
                let oc = crate::input::back_error_in(st)?;
                scc_err(oc)?;
            } else {
                get_x_token_in(st)?;
                if cur_cmd() != SPACER {
                    crate::input::back_input_in(st)?;
                }
            }
        } else if cur_tok() == FROZEN_PRIM_TOKEN {
            get_token_in(st)?;
            if cur_cs() < HASH_BASE {
                let v =
                    crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, cur_cs() - SINGLE_BASE)
                        .map_err(|e| SccAbort::Input(hash_err_abort(e)))?;
                set_cur_cs(v);
            } else {
                let s = st.eqtb.hash[(cur_cs() as usize) - 514].s1;
                let v = crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, s)
                    .map_err(|e| SccAbort::Input(hash_err_abort(e)))?;
                set_cur_cs(v);
            }
            if cur_cs() != UNDEFINED_PRIMITIVE {
                set_cur_cmd(st.eqtb.b16s1(PRIM_EQTB_BASE + cur_cs()) as u8);
                set_cur_chr(st.eqtb.b32s1(PRIM_EQTB_BASE + cur_cs()));
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
            scan_something_internal_in(st, INT_VAL, false)?;
        } else {
            let mut radix_val: i32 = 10;
            m = 0x0CCC_CCCC;
            if cur_tok() == OCTAL_TOKEN {
                radix_val = 8;
                m = 0x1000_0000;
                get_x_token_in(st)?;
            } else if cur_tok() == HEX_TOKEN {
                radix_val = 16;
                m = 0x0800_0000;
                get_x_token_in(st)?;
            }
            st.scanner.radix = radix_val as u8;
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
                        error_here_with_diagnostic_in(st, c"Number too big".as_ptr());
                        capture_to_diagnostic_in(st, std::ptr::null_mut());
                        st.errors.help_ptr = 2;
                        st.errors.help_line[1] =
                            c"I can only go up to 2147483647='17777777777=\"7FFFFFFF,".as_ptr();
                        st.errors.help_line[0] =
                            c"so I'm using that number instead of yours.".as_ptr();
                        scc_err(crate::errors::error_in(st))?;
                        set_cur_val(TEX_INFINITY);
                        ok_so_far = false;
                    }
                } else {
                    set_cur_val(cur_val() * radix_val + d);
                }
                get_x_token_in(st)?;
            }
            if vacuous {
                error_here_with_diagnostic_in(st, c"Missing number, treated as zero".as_ptr());
                capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 3;
                st.errors.help_line[2] =
                    c"A number should have been here; I inserted `0'.".as_ptr();
                st.errors.help_line[1] =
                    c"(If you can't figure out why I needed to see a number,".as_ptr();
                st.errors.help_line[0] =
                    c"look up `weird error' in the index to The TeXbook.)".as_ptr();
                let oc = crate::input::back_error_in(st)?;
                scc_err(oc)?;
            } else if cur_cmd() != SPACER {
                crate::input::back_input_in(st)?;
            }
        }
        break; // end 'restart
    }

    if negative {
        set_cur_val(-cur_val());
    }
    Ok(())
}
pub(crate) fn scan_dimen_in(
    st: &mut crate::state::EngineState,
    mu: bool,
    inf: bool,
    shortcut: bool,
) -> Result<(), SccAbort> {
    xetex_scan_dimen_in(st, mu, inf, shortcut, true)
}
pub(crate) fn xetex_scan_dimen_in(
    st: &mut crate::state::EngineState,
    mu: bool,
    inf: bool,
    shortcut: bool,
    requires_units: bool,
) -> Result<(), SccAbort> {
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

    st.errors.arith_error = false;
    st.scanner.cur_order = NORMAL_ORDER;

    'body: {
        if !shortcut {
            // Skip sign-prefix tokens
            loop {
                loop {
                    get_x_token_in(st)?;
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
                    scan_something_internal_in(st, MU_VAL, false)?;
                    if st.scanner.cur_val_level as i16 >= GLUE_VAL {
                        let v = st.mem.b32s1(cur_val() + 1);
                        crate::mem::delete_glue_ref(&mut st.mem, cur_val());
                        set_cur_val(v);
                    }
                    if st.scanner.cur_val_level as i16 == MU_VAL {
                        break 'body;
                    } // goto attach_sign
                    if st.scanner.cur_val_level as i16 != INT_VAL {
                        scc_err(crate::errors::mu_error_in(st))?;
                    }
                } else {
                    scan_something_internal_in(st, DIMEN_VAL, false)?;
                    if st.scanner.cur_val_level as i16 == DIMEN_VAL {
                        break 'body;
                    } // goto attach_sign
                }
            } else {
                crate::input::back_input_in(st)?;
                if cur_tok() == CONTINENTAL_POINT_TOKEN {
                    set_cur_tok(POINT_TOKEN);
                }
                if cur_tok() != POINT_TOKEN {
                    scan_int_in(st)?;
                } else {
                    st.scanner.radix = 10;
                    set_cur_val(0);
                }
                if cur_tok() == CONTINENTAL_POINT_TOKEN {
                    set_cur_tok(POINT_TOKEN);
                }

                if st.scanner.radix == 10 && cur_tok() == POINT_TOKEN {
                    let mut k: i32 = 0;
                    let mut p: i32 = crate::mem::TEX_NULL;
                    get_token_in(st)?;
                    'decimal: loop {
                        get_x_token_in(st)?;
                        if cur_tok() > ZERO_TOKEN + 9 || cur_tok() < ZERO_TOKEN {
                            break 'decimal;
                        }
                        if k < 17 {
                            let q = crate::mem::get_avail(&mut st.mem);
                            st.mem.set_b32s1(q, p);
                            st.mem.set_b32s0(q, cur_tok() - ZERO_TOKEN);
                            p = q;
                            k += 1;
                        }
                    }
                    let mut kk = k;
                    while kk >= 1 {
                        st.errors.dig[(kk - 1) as usize] = st.mem.b32s0(p) as u8;
                        let old_p = p;
                        p = st.mem.b32s1(p);
                        st.mem.set_b32s1(old_p, st.mem.avail);
                        st.mem.avail = old_p;
                        kk -= 1;
                    }
                    f = round_decimals(&st.errors, k);
                    if cur_cmd() != SPACER {
                        crate::input::back_input_in(st)?;
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
                if inf && scan_keyword_in(st, c"fil".as_ptr())? {
                    st.scanner.cur_order = FIL_ORDER;
                    while scan_keyword_in(st, c"l".as_ptr())? {
                        if st.scanner.cur_order == FILLL_ORDER {
                            error_here_with_diagnostic_in(
                                st,
                                c"Illegal unit of measure (replaced with filll)".as_ptr(),
                            );
                            capture_to_diagnostic_in(st, std::ptr::null_mut());
                            st.errors.help_ptr = 1;
                            st.errors.help_line[0] =
                                c"I dddon't go any higher than filll.".as_ptr();
                            scc_err(crate::errors::error_in(st))?;
                        } else {
                            st.scanner.cur_order += 1;
                        }
                    }
                    break 'units (true, true); // attach_fraction
                }

                let save_cv = cur_val();
                loop {
                    get_x_token_in(st)?;
                    if cur_cmd() != SPACER {
                        break;
                    }
                }

                if cur_cmd() >= MIN_INTERNAL && cur_cmd() <= MAX_INTERNAL {
                    if mu {
                        scan_something_internal_in(st, MU_VAL, false)?;
                        if st.scanner.cur_val_level as i16 >= GLUE_VAL {
                            let v = st.mem.b32s1(cur_val() + 1);
                            crate::mem::delete_glue_ref(&mut st.mem, cur_val());
                            set_cur_val(v);
                        }
                        if st.scanner.cur_val_level as i16 != MU_VAL {
                            scc_err(crate::errors::mu_error_in(st))?;
                        }
                    } else {
                        scan_something_internal_in(st, DIMEN_VAL, false)?;
                    }
                    let v = cur_val();
                    let xod = crate::scaled_math::xn_over_d_in(&mut st.errors, v, f, 65536);
                    set_cur_val(crate::scaled_math::mult_and_add_in(
                        &mut st.errors,
                        save_cv,
                        v,
                        xod,
                        MAX_HALFWORD,
                    ));
                    break 'body; // goto attach_sign (found:)
                }

                crate::input::back_input_in(st)?;

                // em/ex lookup (not available for mu)
                let em_ex = if !mu {
                    if scan_keyword_in(st, c"em".as_ptr())? {
                        let fi = st.eqtb.b32s1(CUR_FONT_LOC);
                        let k = QUAD_CODE + st.fonts.param_base[fi as usize];
                        Some(unsafe { st.fonts.font_info[k as usize].b32.s1 })
                    } else if scan_keyword_in(st, c"ex".as_ptr())? {
                        let fi = st.eqtb.b32s1(CUR_FONT_LOC);
                        let k = X_HEIGHT_CODE + st.fonts.param_base[fi as usize];
                        Some(unsafe { st.fonts.font_info[k as usize].b32.s1 })
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(v) = em_ex {
                    get_x_token_in(st)?;
                    if cur_cmd() != SPACER {
                        crate::input::back_input_in(st)?;
                    }
                    let xod = crate::scaled_math::xn_over_d_in(&mut st.errors, v, f, 65536);
                    set_cur_val(crate::scaled_math::mult_and_add_in(
                        &mut st.errors,
                        save_cv,
                        v,
                        xod,
                        MAX_HALFWORD,
                    ));
                    break 'body; // goto attach_sign (found:)
                }

                // not_found:
                if mu {
                    if scan_keyword_in(st, c"mu".as_ptr())? {
                        break 'units (true, true); // attach_fraction
                    } else {
                        error_here_with_diagnostic_in(
                            st,
                            c"Illegal unit of measure (mu inserted)".as_ptr(),
                        );
                        capture_to_diagnostic_in(st, std::ptr::null_mut());
                        st.errors.help_ptr = 4;
                        st.errors.help_line[3] =
                            c"The unit of measurement in math glue must be mu.".as_ptr();
                        st.errors.help_line[2] =
                            c"To recover gracefully from this error, it's best to".as_ptr();
                        st.errors.help_line[1] =
                            c"delete the erroneous units; e.g., type `2' to delete".as_ptr();
                        st.errors.help_line[0] =
                            c"two letters. (See Chapter 27 of The TeXbook.)".as_ptr();
                        scc_err(crate::errors::error_in(st))?;
                        break 'units (true, true); // attach_fraction
                    }
                }

                if scan_keyword_in(st, c"true".as_ptr())? {
                    scc_err(crate::save::prepare_mag_in(st))?;
                    let mag = st.eqtb.b32s1(INT_BASE + INT_PAR_MAG);
                    if mag != 1000 {
                        set_cur_val(crate::scaled_math::xn_over_d_in(
                            &mut st.errors,
                            cur_val(),
                            1000,
                            mag,
                        ));
                        f = (1000 * f + 65536 * st.errors.tex_remainder) / mag;
                        set_cur_val(cur_val() + f / 65536);
                        f %= 65536;
                    }
                }

                if scan_keyword_in(st, c"pt".as_ptr())? {
                    break 'units (true, true);
                } // attach_fraction

                let (num, denom) = if scan_keyword_in(st, c"in".as_ptr())? {
                    (7227_i32, 100_i32)
                } else if scan_keyword_in(st, c"pc".as_ptr())? {
                    (12, 1)
                } else if scan_keyword_in(st, c"cm".as_ptr())? {
                    (7227, 254)
                } else if scan_keyword_in(st, c"mm".as_ptr())? {
                    (7227, 2540)
                } else if scan_keyword_in(st, c"bp".as_ptr())? {
                    (7227, 7200)
                } else if scan_keyword_in(st, c"dd".as_ptr())? {
                    (1238, 1157)
                } else if scan_keyword_in(st, c"cc".as_ptr())? {
                    (14856, 1157)
                } else if scan_keyword_in(st, c"sp".as_ptr())? {
                    break 'units (false, true); // done: skip fraction
                } else {
                    error_here_with_diagnostic_in(
                        st,
                        c"Illegal unit of measure (pt inserted)".as_ptr(),
                    );
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 6;
                    st.errors.help_line[5] =
                        c"Dimensions can be in units of em, ex, in, pt, pc,".as_ptr();
                    st.errors.help_line[4] =
                        c"cm, mm, dd, cc, bp, or sp; but yours is a new one!".as_ptr();
                    st.errors.help_line[3] =
                        c"I'll assume that you meant to say pt, for printer's points.".as_ptr();
                    st.errors.help_line[2] =
                        c"To recover gracefully from this error, it's best to".as_ptr();
                    st.errors.help_line[1] =
                        c"delete the erroneous units; e.g., type `2' to delete".as_ptr();
                    st.errors.help_line[0] =
                        c"two letters. (See Chapter 27 of The TeXbook.)".as_ptr();
                    scc_err(crate::errors::error_in(st))?;
                    break 'units (true, true); // done2 → attach_fraction
                };

                set_cur_val(crate::scaled_math::xn_over_d_in(
                    &mut st.errors,
                    cur_val(),
                    num,
                    denom,
                ));
                f = (num * f + 65536 * st.errors.tex_remainder) / denom;
                set_cur_val(cur_val() + f / 65536);
                f %= 65536;

                (true, true) // attach_fraction
            };

            if apply_fraction {
                if cur_val() >= 16384 {
                    st.errors.arith_error = true;
                } else {
                    set_cur_val(cur_val() * 65536 + f);
                }
            }
            if read_spacer {
                get_x_token_in(st)?;
                if cur_cmd() != SPACER {
                    crate::input::back_input_in(st)?;
                }
            }
        } else {
            // !requires_units
            if cur_val() >= 16384 {
                st.errors.arith_error = true;
            } else {
                set_cur_val(cur_val() * 65536 + f);
            }
        }
    } // end 'body

    // attach_sign:
    if st.errors.arith_error || cur_val().abs() >= 0x4000_0000 {
        error_here_with_diagnostic_in(st, c"Dimension too large".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] = c"I can't work with sizes bigger than about 19 feet.".as_ptr();
        st.errors.help_line[0] = c"Continue and I'll use the largest value I can.".as_ptr();
        scc_err(crate::errors::error_in(st))?;
        set_cur_val(MAX_HALFWORD);
        st.errors.arith_error = false;
    }
    if negative {
        set_cur_val(-cur_val());
    }
    Ok(())
}
pub(crate) fn scan_glue_in(st: &mut crate::state::EngineState, level: i16) -> Result<(), SccAbort> {
    const MINUS_TOKEN: i32 = OTHER_TOKEN + b'-' as i32;
    const PLUS_TOKEN: i32 = OTHER_TOKEN + b'+' as i32;

    let mu = level == MU_VAL;
    let mut negative = false;

    loop {
        loop {
            get_x_token_in(st)?;
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
        scan_something_internal_in(st, level, negative)?;
        if st.scanner.cur_val_level as i16 >= GLUE_VAL {
            if st.scanner.cur_val_level as i16 != level {
                scc_err(crate::errors::mu_error_in(st))?;
            }
            return Ok(());
        }
        if st.scanner.cur_val_level as i16 == INT_VAL {
            scan_dimen_in(st, mu, false, true)?;
        } else if level == MU_VAL {
            scc_err(crate::errors::mu_error_in(st))?;
        }
    } else {
        crate::input::back_input_in(st)?;
        scan_dimen_in(st, mu, false, false)?;
        if negative {
            set_cur_val(-cur_val());
        }
    }

    let q = new_spec_scc(st, 0)?;
    st.mem.set_b32s1(q + 1, cur_val());

    if scan_keyword_in(st, c"plus".as_ptr())? {
        scan_dimen_in(st, mu, true, false)?;
        st.mem.set_b32s1(q + 2, cur_val());
        st.mem.set_b16s1(q, st.scanner.cur_order as u16);
    }
    if scan_keyword_in(st, c"minus".as_ptr())? {
        scan_dimen_in(st, mu, true, false)?;
        st.mem.set_b32s1(q + 3, cur_val());
        st.mem.set_b16s0(q, st.scanner.cur_order as u16);
    }

    set_cur_val(q);
    Ok(())
}
pub(crate) fn scan_normal_glue_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_glue_in(st, GLUE_VAL)
}
pub(crate) fn scan_mu_glue_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_glue_in(st, MU_VAL_SCAN)
}
pub(crate) fn scan_expr_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let mut l = st.scanner.cur_val_level as i16;
    let a = st.errors.arith_error;
    let mut b = false;
    let mut p = crate::mem::TEX_NULL;

    st.scanner.expand_depth_count += 1;
    if st.scanner.expand_depth_count >= expand_depth() {
        return Err(SccAbort::Input(crate::input::InputAbort::Overflow {
            what: c"expansion depth".as_ptr(),
            n: expand_depth(),
        }));
    }

    // restart:
    let mut r: i16;
    let mut e: i32;
    let mut s: i16;
    let mut t: i32;
    let mut n: i32;
    let mut f: i32 = 0;
    let mut skip_scan; // true means "goto found" (skip value-scan phase)

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
                    get_x_token_in(st)?;
                    if cur_cmd() != SPACER {
                        break;
                    }
                }
                if cur_tok() == OTHER_TOKEN + 40 {
                    // '('
                    let q = get_node_scc(st, EXPR_NODE_SIZE)?;
                    st.mem.set_b32s1(q, p);
                    st.mem.set_b16s1(q, l as u16);
                    st.mem.set_b16s0(q, (4 * s + r) as u16);
                    st.mem.set_b32s1(q + 1, e);
                    st.mem.set_b32s1(q + 2, t);
                    st.mem.set_b32s1(q + 3, n);
                    p = q;
                    l = o;
                    continue 'outer; // goto restart
                }
                crate::input::back_input_in(st)?;
                if o == INT_VAL {
                    scan_int_in(st)?;
                } else if o == DIMEN_VAL {
                    scan_dimen_in(st, false, false, false)?;
                } else if o == GLUE_VAL_EXPR {
                    scan_normal_glue_in(st)?;
                } else {
                    scan_mu_glue_in(st)?;
                }
                f = cur_val();
            }
            skip_scan = false;

            // found:
            loop {
                get_x_token_in(st)?;
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
                        crate::input::back_input_in(st)?;
                    }
                } else if cur_tok() != OTHER_TOKEN + 41 {
                    // ')'
                    error_here_with_diagnostic_in(
                        st,
                        c"Missing ) inserted for expression".as_ptr(),
                    );
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 1;
                    st.errors.help_line[0] =
                        c"I was expecting to see `+', `-', `*', `/', or `)'. Didn't.".as_ptr();
                    let oc = crate::input::back_error_in(st)?;
                    scc_err(oc)?;
                }
            }

            st.errors.arith_error = b;
            if l == INT_VAL || s > EXPR_SUB {
                if !(-TEX_INFINITY..=TEX_INFINITY).contains(&f) {
                    st.errors.arith_error = true;
                    f = 0;
                }
            } else if l == DIMEN_VAL {
                if f.abs() > MAX_HALFWORD {
                    st.errors.arith_error = true;
                    f = 0;
                }
            } else if st.mem.b32s1(f + 1).abs() > MAX_HALFWORD
                || st.mem.b32s1(f + 2).abs() > MAX_HALFWORD
                || st.mem.b32s1(f + 3).abs() > MAX_HALFWORD
            {
                st.errors.arith_error = true;
                crate::mem::delete_glue_ref(&mut st.mem, f);
                f = new_spec_scc(st, 0)?;
            }

            // o may be modified inside case 3 (EXPR_MULT → EXPR_SCALE)
            match s {
                0 => {
                    if l >= GLUE_VAL_EXPR && o != EXPR_NONE {
                        t = new_spec_scc(st, f)?;
                        crate::mem::delete_glue_ref(&mut st.mem, f);
                        if st.mem.b32s1(t + 2) == 0 {
                            st.mem.set_b16s1(t, NORMAL_EXPR);
                        }
                        if st.mem.b32s1(t + 3) == 0 {
                            st.mem.set_b16s0(t, NORMAL_EXPR);
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
                        t = crate::scaled_math::mult_and_add_in(
                            &mut st.errors,
                            t,
                            f,
                            0,
                            TEX_INFINITY,
                        );
                    } else if l == DIMEN_VAL {
                        t = crate::scaled_math::mult_and_add_in(
                            &mut st.errors,
                            t,
                            f,
                            0,
                            MAX_HALFWORD,
                        );
                    } else {
                        let v1 = crate::scaled_math::mult_and_add_in(
                            &mut st.errors,
                            st.mem.b32s1(t + 1),
                            f,
                            0,
                            MAX_HALFWORD,
                        );
                        st.mem.set_b32s1(t + 1, v1);
                        let v2 = crate::scaled_math::mult_and_add_in(
                            &mut st.errors,
                            st.mem.b32s1(t + 2),
                            f,
                            0,
                            MAX_HALFWORD,
                        );
                        st.mem.set_b32s1(t + 2, v2);
                        let v3 = crate::scaled_math::mult_and_add_in(
                            &mut st.errors,
                            st.mem.b32s1(t + 3),
                            f,
                            0,
                            MAX_HALFWORD,
                        );
                        st.mem.set_b32s1(t + 3, v3);
                    }
                }
                4 => {
                    // EXPR_DIV
                    if l < GLUE_VAL_EXPR {
                        t = crate::scaled_math::quotient_in(&mut st.errors, t, f);
                    } else {
                        let v1 =
                            crate::scaled_math::quotient_in(&mut st.errors, st.mem.b32s1(t + 1), f);
                        st.mem.set_b32s1(t + 1, v1);
                        let v2 =
                            crate::scaled_math::quotient_in(&mut st.errors, st.mem.b32s1(t + 2), f);
                        st.mem.set_b32s1(t + 2, v2);
                        let v3 =
                            crate::scaled_math::quotient_in(&mut st.errors, st.mem.b32s1(t + 3), f);
                        st.mem.set_b32s1(t + 3, v3);
                    }
                }
                5 => {
                    // EXPR_SCALE
                    if l == INT_VAL {
                        t = crate::scaled_math::fract_in(&mut st.errors, t, n, f, TEX_INFINITY);
                    } else if l == DIMEN_VAL {
                        t = crate::scaled_math::fract_in(&mut st.errors, t, n, f, MAX_HALFWORD);
                    } else {
                        let v1 = crate::scaled_math::fract_in(
                            &mut st.errors,
                            st.mem.b32s1(t + 1),
                            n,
                            f,
                            MAX_HALFWORD,
                        );
                        st.mem.set_b32s1(t + 1, v1);
                        let v2 = crate::scaled_math::fract_in(
                            &mut st.errors,
                            st.mem.b32s1(t + 2),
                            n,
                            f,
                            MAX_HALFWORD,
                        );
                        st.mem.set_b32s1(t + 2, v2);
                        let v3 = crate::scaled_math::fract_in(
                            &mut st.errors,
                            st.mem.b32s1(t + 3),
                            n,
                            f,
                            MAX_HALFWORD,
                        );
                        st.mem.set_b32s1(t + 3, v3);
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
                    e = crate::scaled_math::add_or_sub_in(
                        &mut st.errors,
                        e,
                        t,
                        TEX_INFINITY,
                        r == EXPR_SUB,
                    );
                } else if l == DIMEN_VAL {
                    e = crate::scaled_math::add_or_sub_in(
                        &mut st.errors,
                        e,
                        t,
                        MAX_HALFWORD,
                        r == EXPR_SUB,
                    );
                } else {
                    let v1 = crate::scaled_math::add_or_sub_in(
                        &mut st.errors,
                        st.mem.b32s1(e + 1),
                        st.mem.b32s1(t + 1),
                        MAX_HALFWORD,
                        r == EXPR_SUB,
                    );
                    st.mem.set_b32s1(e + 1, v1);
                    if st.mem.b16s1(e) == st.mem.b16s1(t) {
                        let v2 = crate::scaled_math::add_or_sub_in(
                            &mut st.errors,
                            st.mem.b32s1(e + 2),
                            st.mem.b32s1(t + 2),
                            MAX_HALFWORD,
                            r == EXPR_SUB,
                        );
                        st.mem.set_b32s1(e + 2, v2);
                    } else if st.mem.b16s1(e) < st.mem.b16s1(t) && st.mem.b32s1(t + 2) != 0 {
                        let v = st.mem.b32s1(t + 2);
                        st.mem.set_b32s1(e + 2, v);
                        let o2 = st.mem.b16s1(t);
                        st.mem.set_b16s1(e, o2);
                    }
                    if st.mem.b16s0(e) == st.mem.b16s0(t) {
                        let v3 = crate::scaled_math::add_or_sub_in(
                            &mut st.errors,
                            st.mem.b32s1(e + 3),
                            st.mem.b32s1(t + 3),
                            MAX_HALFWORD,
                            r == EXPR_SUB,
                        );
                        st.mem.set_b32s1(e + 3, v3);
                    } else if st.mem.b16s0(e) < st.mem.b16s0(t) && st.mem.b32s1(t + 3) != 0 {
                        let v = st.mem.b32s1(t + 3);
                        st.mem.set_b32s1(e + 3, v);
                        let o0 = st.mem.b16s0(t);
                        st.mem.set_b16s0(e, o0);
                    }
                    crate::mem::delete_glue_ref(&mut st.mem, t);
                    if st.mem.b32s1(e + 2) == 0 {
                        st.mem.set_b16s1(e, NORMAL_EXPR);
                    }
                    if st.mem.b32s1(e + 3) == 0 {
                        st.mem.set_b16s0(e, NORMAL_EXPR);
                    }
                }
                r = o;
            }
            b = st.errors.arith_error;
            if o != EXPR_NONE {
                continue; // goto continue_
            }
            if p != crate::mem::TEX_NULL {
                f = e;
                let q = p;
                e = st.mem.b32s1(q + 1);
                t = st.mem.b32s1(q + 2);
                n = st.mem.b32s1(q + 3);
                s = (st.mem.b16s0(q) / 4) as i16;
                r = (st.mem.b16s0(q) % 4) as i16;
                l = st.mem.b16s1(q) as i16;
                p = st.mem.b32s1(q);
                crate::mem::free_node(&mut st.mem, q, EXPR_NODE_SIZE);
                skip_scan = true;
                continue; // goto found
            }
            break 'outer;
        }
    }

    st.scanner.expand_depth_count -= 1;

    if b {
        error_here_with_diagnostic_in(st, c"Arithmetic overflow".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] = c"I can't evaluate this expression,".as_ptr();
        st.errors.help_line[0] = c"since the result is out of range.".as_ptr();
        scc_err(crate::errors::error_in(st))?;
        if l >= GLUE_VAL_EXPR {
            crate::mem::delete_glue_ref(&mut st.mem, e);
            e = 0;
            let v = st.mem.b32s1(e) + 1; // GLUE_SPEC_ref_count(e)++
            st.mem.set_b32s1(e, v);
        } else {
            e = 0;
        }
    }
    st.errors.arith_error = a;
    set_cur_val(e);
    st.scanner.cur_val_level = l as u8;
    Ok(())
}
pub(crate) fn scan_keyword_in(
    st: &mut crate::state::EngineState,
    s: *const libc::c_char,
) -> Result<bool, SccAbort> {
    let mut p = BACKUP_HEAD;
    st.mem.set_b32s1(p, crate::mem::TEX_NULL);

    let slen = unsafe { libc::strlen(s) };

    if slen == 1 {
        let c = unsafe { *s } as i32;
        let save_cur_cs = cur_cs();
        loop {
            get_x_token_in(st)?;
            if cur_cs() == 0 && (cur_chr() == c || cur_chr() == c - 32) {
                let q = crate::mem::get_avail(&mut st.mem);
                st.mem.set_b32s1(p, q);
                st.mem.set_b32s0(q, cur_tok());
                p = q;
                let head = st.mem.b32s1(BACKUP_HEAD);
                crate::mem::flush_list(&mut st.mem, head);
                return Ok(true);
            } else if cur_cmd() != SPACER || p != BACKUP_HEAD {
                crate::input::back_input_in(st)?;
                if p != BACKUP_HEAD {
                    let head = st.mem.b32s1(BACKUP_HEAD);
                    crate::input::begin_token_list_in(st, head, BACKED_UP)?;
                }
                set_cur_cs(save_cur_cs);
                return Ok(false);
            }
        }
    }

    let mut i = 0usize;
    while i < slen {
        get_x_token_in(st)?;
        let ch = unsafe { *s.add(i) } as i32;
        if cur_cs() == 0 && (cur_chr() == ch || cur_chr() == ch - 32) {
            let q = crate::mem::get_avail(&mut st.mem);
            st.mem.set_b32s1(p, q);
            st.mem.set_b32s0(q, cur_tok());
            p = q;
            i += 1;
        } else if cur_cmd() != SPACER || p != BACKUP_HEAD {
            crate::input::back_input_in(st)?;
            if p != BACKUP_HEAD {
                let head = st.mem.b32s1(BACKUP_HEAD);
                crate::input::begin_token_list_in(st, head, BACKED_UP)?;
            }
            return Ok(false);
        }
    }
    let head = st.mem.b32s1(BACKUP_HEAD);
    crate::mem::flush_list(&mut st.mem, head);
    Ok(true)
}
pub(crate) fn scan_left_brace_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    loop {
        get_x_token_in(st)?;
        if cur_cmd() != SPACER && cur_cmd() != RELAX {
            break;
        }
    }
    if cur_cmd() != LEFT_BRACE {
        error_here_with_diagnostic_in(st, c"Missing { inserted".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 4;
        st.errors.help_line[3] = c"A left brace was mandatory here, so I've put one in.".as_ptr();
        st.errors.help_line[2] =
            c"You might want to delete and/or insert some corrections".as_ptr();
        st.errors.help_line[1] = c"so that I will find a matching right brace soon.".as_ptr();
        st.errors.help_line[0] = c"(If you're confused by all this, try typing `I}' now.)".as_ptr();
        let oc = crate::input::back_error_in(st)?;
        scc_err(oc)?;
        set_cur_tok(LEFT_BRACE_TOKEN + '{' as i32);
        set_cur_cmd(LEFT_BRACE);
        set_cur_chr('{' as i32);
        st.scanner.align_state += 1;
    }
    Ok(())
}
pub(crate) fn scan_register_num_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if cur_val() < 0 || cur_val() > st.scanner.max_reg_num {
        let help1 = st.scanner.max_reg_help_line;
        leaf_int_range_error(st, c"Bad register code".as_ptr(), help1)?;
    }
    Ok(())
}
pub(crate) fn scan_four_bit_int_or_18_in(
    st: &mut crate::state::EngineState,
) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if cur_val() < 0 || (cur_val() > 15 && cur_val() != 18) {
        leaf_int_range_error(
            st,
            c"Bad number".as_ptr(),
            c"Since I expected to read a number between 0 and 15,".as_ptr(),
        )?;
    }
    Ok(())
}
pub(crate) fn scan_something_internal_in(
    st: &mut crate::state::EngineState,
    level: i16,
    negative: bool,
) -> Result<(), SccAbort> {
    use crate::mem::TEX_NULL;

    let mut m: i32;
    let mut n: i32 = 0;
    let k: i32;
    let kk: i32;
    let mut q: i32;
    let mut r: i32;
    let mut tx: i32;

    // The C function has a `goto restart` handled here with a loop.
    'restart: loop {
        m = cur_chr();

        match cur_cmd() {
            DEF_CODE => {
                scan_usv_num_in(st)?;
                if m == MATH_CODE_BASE {
                    let code = st.eqtb.b32s1(MATH_CODE_BASE + cur_val());
                    st.scanner.cur_val1 = code;
                    let class = ((code as u32 >> 21) & 0x07) as i32;
                    let fam = ((code as u32 >> 24) & 0xFF) as i32;
                    let ch = (code as u32 & 0x1FFFFF) as i32;
                    if ch == ACTIVE_MATH_CHAR {
                        st.scanner.cur_val1 = 0x8000;
                    } else if class > 7 || fam > 15 || ch > 255 {
                        let errmsg = error_here_with_diagnostic_in(
                            st,
                            c"Extended mathchar used as mathchar".as_ptr(),
                        );
                        unsafe {
                            ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), st.scanner.cur_val1)
                        };
                        capture_to_diagnostic_in(st, std::ptr::null_mut());
                        st.errors.help_ptr = 2;
                        st.errors.help_line[1] =
                            c"A mathchar number must be between 0 and \"7FFF.".as_ptr();
                        st.errors.help_line[0] = c"I changed this one to zero.".as_ptr();
                        let cv1 = st.scanner.cur_val1;
                        scc_err(int_error_in(st, cv1))?;
                        st.scanner.cur_val1 = 0;
                    }
                    let cv1 = st.scanner.cur_val1;
                    let v = ((cv1 as u32 >> 21) & 0x07) as i32 * 0x1000
                        + ((cv1 as u32 >> 24) & 0xFF) as i32 * 0x100
                        + (cv1 as u32 & 0x1FFFFF) as i32;
                    set_cur_val(v);
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else if m == DEL_CODE_BASE {
                    let code = st.eqtb.b32s1(DEL_CODE_BASE + cur_val());
                    st.scanner.cur_val1 = code;
                    if st.scanner.cur_val1 >= 0x40000000 {
                        error_here_with_diagnostic_in(
                            st,
                            c"Extended delcode used as delcode".as_ptr(),
                        );
                        capture_to_diagnostic_in(st, std::ptr::null_mut());
                        st.errors.help_ptr = 2;
                        st.errors.help_line[1] =
                            c"I can only go up to 2147483647='17777777777=\"7FFFFFFF,".as_ptr();
                        st.errors.help_line[0] = c"I changed this one to zero.".as_ptr();
                        scc_err(crate::errors::error_in(st))?;
                        set_cur_val(0);
                        st.scanner.cur_val_level = INT_VAL as u8;
                    } else {
                        set_cur_val(st.scanner.cur_val1);
                        st.scanner.cur_val_level = INT_VAL as u8;
                    }
                } else if m < SF_CODE_BASE {
                    set_cur_val(st.eqtb.b32s1(m + cur_val()));
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else if m < MATH_CODE_BASE {
                    set_cur_val(st.eqtb.b32s1(m + cur_val()) % 65536);
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else {
                    set_cur_val(st.eqtb.b32s1(m + cur_val()));
                    st.scanner.cur_val_level = INT_VAL as u8;
                }
            }
            XETEX_DEF_CODE => {
                scan_usv_num_in(st)?;
                if m == SF_CODE_BASE {
                    set_cur_val(st.eqtb.b32s1(SF_CODE_BASE + cur_val()) / 65536);
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else if m == MATH_CODE_BASE {
                    set_cur_val(st.eqtb.b32s1(MATH_CODE_BASE + cur_val()));
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else if m == MATH_CODE_BASE + 1 {
                    error_here_with_diagnostic_in(
                        st,
                        c"Can't use \\Umathcode as a number (try \\Umathcodenum)".as_ptr(),
                    );
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 2;
                    st.errors.help_line[1] =
                        c"\\Umathcode is for setting a mathcode from separate values;".as_ptr();
                    st.errors.help_line[0] =
                        c"use \\Umathcodenum to access them as single values.".as_ptr();
                    scc_err(crate::errors::error_in(st))?;
                    set_cur_val(0);
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else if m == DEL_CODE_BASE {
                    set_cur_val(st.eqtb.b32s1(DEL_CODE_BASE + cur_val()));
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else {
                    error_here_with_diagnostic_in(
                        st,
                        c"Can't use \\Udelcode as a number (try \\Udelcodenum)".as_ptr(),
                    );
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 2;
                    st.errors.help_line[1] =
                        c"\\Udelcode is for setting a delcode from separate values;".as_ptr();
                    st.errors.help_line[0] =
                        c"use \\Udelcodenum to access them as single values.".as_ptr();
                    scc_err(crate::errors::error_in(st))?;
                    set_cur_val(0);
                    st.scanner.cur_val_level = INT_VAL as u8;
                }
            }
            TOKS_REGISTER | ASSIGN_TOKS | DEF_FAMILY | SET_FONT | DEF_FONT => {
                if level != TOK_VAL {
                    error_here_with_diagnostic_in(st, c"Missing number, treated as zero".as_ptr());
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 3;
                    st.errors.help_line[2] =
                        c"A number should have been here; I inserted `0'.".as_ptr();
                    st.errors.help_line[1] =
                        c"(If you can't figure out why I needed to see a number,".as_ptr();
                    st.errors.help_line[0] =
                        c"look up `weird error' in the index to The TeXbook.)".as_ptr();
                    let oc = crate::input::back_error_in(st)?;
                    scc_err(oc)?;
                    set_cur_val(0);
                    st.scanner.cur_val_level = DIMEN_VAL as u8;
                } else if cur_cmd() <= ASSIGN_TOKS {
                    if cur_cmd() < ASSIGN_TOKS {
                        if m == 0 {
                            scan_register_num_in(st)?;
                            if cur_val() < 256 {
                                set_cur_val(st.eqtb.b32s1(TOKS_BASE + cur_val()));
                            } else {
                                crate::save::find_sa_element(
                                    &mut st.mem,
                                    &mut st.scanner,
                                    TOK_VAL as u16,
                                    cur_val(),
                                    false,
                                );
                                if cur_ptr() == TEX_NULL {
                                    set_cur_val(TEX_NULL);
                                } else {
                                    set_cur_val(st.mem.b32s1(cur_ptr() + 1));
                                }
                            }
                        } else {
                            set_cur_val(st.mem.b32s1(m + 1));
                        }
                    } else if cur_chr() == LOCAL_BASE + LOCAL_XETEX_INTER_CHAR_TOKS {
                        scan_char_class_not_ignored_in(st)?;
                        set_cur_ptr(cur_val());
                        scan_char_class_not_ignored_in(st)?;
                        crate::save::find_sa_element(
                            &mut st.mem,
                            &mut st.scanner,
                            INTER_CHAR_VAL as u16,
                            cur_ptr() * CHAR_CLASS_LIMIT + cur_val(),
                            false,
                        );
                        if cur_ptr() == TEX_NULL {
                            set_cur_val(TEX_NULL);
                        } else {
                            set_cur_val(st.mem.b32s1(cur_ptr() + 1));
                        }
                    } else {
                        set_cur_val(st.eqtb.b32s1(m));
                    }
                    st.scanner.cur_val_level = TOK_VAL as u8;
                } else {
                    crate::input::back_input_in(st)?;
                    scan_font_ident_in(st)?;
                    set_cur_val(FONT_ID_BASE + cur_val());
                    st.scanner.cur_val_level = IDENT_VAL as u8;
                }
            }
            ASSIGN_INT => {
                set_cur_val(st.eqtb.b32s1(m));
                st.scanner.cur_val_level = INT_VAL as u8;
            }
            ASSIGN_DIMEN => {
                set_cur_val(st.eqtb.b32s1(m));
                st.scanner.cur_val_level = DIMEN_VAL as u8;
            }
            ASSIGN_GLUE => {
                set_cur_val(st.eqtb.b32s1(m));
                st.scanner.cur_val_level = GLUE_VAL as u8;
            }
            ASSIGN_MU_GLUE => {
                set_cur_val(st.eqtb.b32s1(m));
                st.scanner.cur_val_level = MU_VAL as u8;
            }
            SET_AUX => {
                if (cur_list_mode().abs() as i32) != m {
                    error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
                    print_cmd_chr_in(st, SET_AUX as u16, m);
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 4;
                    st.errors.help_line[3] =
                        c"You can refer to \\spacefactor only in horizontal mode;".as_ptr();
                    st.errors.help_line[2] =
                        c"you can refer to \\prevdepth only in vertical mode; and".as_ptr();
                    st.errors.help_line[1] =
                        c"neither of these is meaningful inside \\write. So".as_ptr();
                    st.errors.help_line[0] =
                        c"I'm forgetting what you said and using zero instead.".as_ptr();
                    scc_err(crate::errors::error_in(st))?;
                    if level != TOK_VAL {
                        set_cur_val(0);
                        st.scanner.cur_val_level = DIMEN_VAL as u8;
                    } else {
                        set_cur_val(0);
                        st.scanner.cur_val_level = INT_VAL as u8;
                    }
                } else if m == VMODE as i32 {
                    set_cur_val(cur_list_aux_b32s1());
                    st.scanner.cur_val_level = DIMEN_VAL as u8;
                } else {
                    set_cur_val(cur_list_aux_b32s0());
                    st.scanner.cur_val_level = INT_VAL as u8;
                }
            }
            SET_PREV_GRAF => {
                if cur_list_mode() == 0 {
                    set_cur_val(0);
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else {
                    let np = st.input.nest_ptr;
                    st.input.nest[np as usize] = cur_list();
                    let mut p = np;
                    while (st.input.nest[p as usize].mode.abs() as i32) != VMODE as i32 {
                        p -= 1;
                    }
                    set_cur_val(st.input.nest[p as usize].prev_graf);
                    st.scanner.cur_val_level = INT_VAL as u8;
                }
            }
            SET_PAGE_INT => {
                if m == 0 {
                    set_cur_val(st.shipout.dead_cycles);
                } else if m == 2 {
                    set_cur_val(st.errors.interaction as i32);
                } else {
                    set_cur_val(st.shipout.insert_penalties);
                }
                st.scanner.cur_val_level = INT_VAL as u8;
            }
            SET_PAGE_DIMEN => {
                if st.shipout.page_contents == 0 && !st.shipout.output_active {
                    if m == 0 {
                        set_cur_val(MAX_HALFWORD);
                    } else {
                        set_cur_val(0);
                    }
                } else {
                    set_cur_val(st.shipout.page_so_far[m as usize]);
                }
                st.scanner.cur_val_level = DIMEN_VAL as u8;
            }
            SET_SHAPE => {
                if m > LOCAL_BASE + LOCAL_PAR_SHAPE {
                    scan_int_in(st)?;
                    let shape_ptr = st.eqtb.b32s1(m);
                    if shape_ptr == TEX_NULL || cur_val() < 0 {
                        set_cur_val(0);
                    } else {
                        let cnt = st.mem.b32s1(shape_ptr + 1);
                        if cur_val() > cnt {
                            set_cur_val(cnt);
                        }
                        set_cur_val(st.mem.b32s1(shape_ptr + cur_val() + 1));
                    }
                } else {
                    let par_shape = st.eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
                    if par_shape == TEX_NULL {
                        set_cur_val(0);
                    } else {
                        set_cur_val(st.mem.b32s0(par_shape));
                    }
                }
                st.scanner.cur_val_level = INT_VAL as u8;
            }
            SET_BOX_DIMEN => {
                scan_register_num_in(st)?;
                if cur_val() < 256 {
                    q = st.eqtb.b32s1(BOX_BASE + cur_val());
                } else {
                    crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, cur_val(), false);
                    if cur_ptr() == TEX_NULL {
                        q = TEX_NULL;
                    } else {
                        q = st.mem.b32s1(cur_ptr() + 1);
                    }
                }
                if q == TEX_NULL {
                    set_cur_val(0);
                } else {
                    set_cur_val(st.mem.b32s1(q + m));
                }
                st.scanner.cur_val_level = DIMEN_VAL as u8;
            }
            CHAR_GIVEN | MATH_GIVEN | XETEX_MATH_GIVEN => {
                set_cur_val(cur_chr());
                st.scanner.cur_val_level = INT_VAL as u8;
            }
            ASSIGN_FONT_DIMEN => {
                find_font_dimen_in(st, false)?;
                let fp = st.fonts.fmem_ptr;
                st.fonts.font_info[fp as usize].b32.s1 = 0;
                set_cur_val(unsafe { st.fonts.font_info[cur_val() as usize].b32.s1 });
                st.scanner.cur_val_level = DIMEN_VAL as u8;
            }
            ASSIGN_FONT_INT => {
                scan_font_ident_in(st)?;
                if m == 0 {
                    set_cur_val(st.fonts.hyphen_char[cur_val() as usize]);
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else if m == 1 {
                    set_cur_val(st.fonts.skew_char[cur_val() as usize]);
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else {
                    n = cur_val();
                    let fa = st.fonts.font_area[n as usize];
                    if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                        scan_glyph_number_in(st, n)?;
                    } else {
                        scan_char_num_in(st)?;
                    }
                    k = cur_val();
                    match m {
                        LP_CODE_BASE => {
                            set_cur_val(unsafe { get_cp_code(n, k as u32, LEFT_SIDE) });
                            st.scanner.cur_val_level = INT_VAL as u8;
                        }
                        RP_CODE_BASE => {
                            set_cur_val(unsafe { get_cp_code(n, k as u32, RIGHT_SIDE) });
                            st.scanner.cur_val_level = INT_VAL as u8;
                        }
                        _ => {}
                    }
                }
            }
            REGISTER => {
                if !(0..=19).contains(&m) {
                    st.scanner.cur_val_level = (st.mem.b16s1(m) / 64) as u8;
                    if (st.scanner.cur_val_level as i16) < GLUE_VAL {
                        set_cur_val(st.mem.b32s1(m + 2));
                    } else {
                        set_cur_val(st.mem.b32s1(m + 1));
                    }
                } else {
                    scan_register_num_in(st)?;
                    st.scanner.cur_val_level = m as u8;
                    if cur_val() > 255 {
                        let cvl = st.scanner.cur_val_level as u16;
                        crate::save::find_sa_element(
                            &mut st.mem,
                            &mut st.scanner,
                            cvl,
                            cur_val(),
                            false,
                        );
                        if cur_ptr() == TEX_NULL {
                            set_cur_val(0);
                        } else if (st.scanner.cur_val_level as i16) < GLUE_VAL {
                            set_cur_val(st.mem.b32s1(cur_ptr() + 2));
                        } else {
                            set_cur_val(st.mem.b32s1(cur_ptr() + 1));
                        }
                    } else {
                        match st.scanner.cur_val_level as i16 {
                            INT_VAL => set_cur_val(st.eqtb.b32s1(COUNT_BASE + cur_val())),
                            DIMEN_VAL => set_cur_val(st.eqtb.b32s1(SCALED_BASE + cur_val())),
                            GLUE_VAL => set_cur_val(st.eqtb.b32s1(SKIP_BASE + cur_val())),
                            MU_VAL => set_cur_val(st.eqtb.b32s1(MU_SKIP_BASE + cur_val())),
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
                                scan_mu_glue_in(st)?;
                            }
                            st.scanner.cur_val_level = GLUE_VAL as u8;
                        } else if m < ETEX_EXPR {
                            if m == GLUE_TO_MU_CODE {
                                scan_normal_glue_in(st)?;
                            }
                            st.scanner.cur_val_level = MU_VAL as u8;
                        } else {
                            st.scanner.cur_val_level = (m - ETEX_EXPR) as u8;
                            scan_expr_in(st)?;
                        }
                        while (st.scanner.cur_val_level as i16) > level {
                            if (st.scanner.cur_val_level as i16) == GLUE_VAL {
                                let gm = cur_val();
                                set_cur_val(st.mem.b32s1(gm + 1));
                                crate::mem::delete_glue_ref(&mut st.mem, gm);
                            } else if (st.scanner.cur_val_level as i16) == MU_VAL {
                                scc_err(crate::errors::mu_error_in(st))?;
                            }
                            st.scanner.cur_val_level -= 1;
                        }
                        if negative {
                            if (st.scanner.cur_val_level as i16) >= GLUE_VAL {
                                let gm = cur_val();
                                let nv = new_spec_scc(st, gm)?;
                                set_cur_val(nv);
                                crate::mem::delete_glue_ref(&mut st.mem, gm);
                                let v1 = -(st.mem.b32s1(cur_val() + 1));
                                st.mem.set_b32s1(cur_val() + 1, v1);
                                let v2 = -(st.mem.b32s1(cur_val() + 2));
                                st.mem.set_b32s1(cur_val() + 2, v2);
                                let v3 = -(st.mem.b32s1(cur_val() + 3));
                                st.mem.set_b32s1(cur_val() + 3, v3);
                            } else {
                                set_cur_val(-cur_val());
                            }
                        }
                        return Ok(());
                    }
                    if m >= XETEX_DIM {
                        match m {
                            XETEX_GLYPH_BOUNDS_CODE => {
                                let cf = st.eqtb.b32s1(CUR_FONT_LOC);
                                let fa = st.fonts.font_area[cf as usize];
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    scan_int_in(st)?;
                                    n = cur_val();
                                    if !(1..=4).contains(&n) {
                                        error_here_with_diagnostic_in(st,
                                            c"\\\\XeTeXglyphbounds requires an edge index from 1 to 4;".as_ptr());
                                        print_nl_cstr_in(
                                            st,
                                            c"I don't know anything about edge ".as_ptr(),
                                        );
                                        print_int_in(st, n);
                                        capture_to_diagnostic_in(st, std::ptr::null_mut());
                                        scc_err(crate::errors::error_in(st))?;
                                        set_cur_val(0);
                                    } else {
                                        scan_int_in(st)?;
                                        let v = unsafe {
                                            crate::ext::get_glyph_bounds(
                                                &st.fonts,
                                                cf,
                                                n,
                                                cur_val(),
                                            )
                                        };
                                        set_cur_val(v);
                                    }
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        cf,
                                        c"; not a native platform font".as_ptr(),
                                    ))?;
                                    set_cur_val(0);
                                }
                            }
                            FONT_CHAR_WD_CODE | FONT_CHAR_HT_CODE | FONT_CHAR_DP_CODE
                            | FONT_CHAR_IC_CODE => {
                                scan_font_ident_in(st)?;
                                q = cur_val();
                                scan_usv_num_in(st)?;
                                let fa = st.fonts.font_area[q as usize];
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    let v = unsafe {
                                        match m {
                                            FONT_CHAR_WD_CODE => {
                                                crate::ext::getnativecharwd(&st.fonts, q, cur_val())
                                            }
                                            FONT_CHAR_HT_CODE => {
                                                crate::ext::getnativecharht(&st.fonts, q, cur_val())
                                            }
                                            FONT_CHAR_DP_CODE => {
                                                crate::ext::getnativechardp(&st.fonts, q, cur_val())
                                            }
                                            _ => {
                                                crate::ext::getnativecharic(&st.fonts, q, cur_val())
                                            }
                                        }
                                    };
                                    set_cur_val(v);
                                } else {
                                    let bc = st.fonts.font_bc[q as usize] as i32;
                                    let ec = st.fonts.font_ec[q as usize] as i32;
                                    if bc <= cur_val() && ec >= cur_val() {
                                        let ec_char = crate::fonts::effective_char(
                                            &mut st.fonts,
                                            true,
                                            q,
                                            cur_val() as u16,
                                        );
                                        let i = unsafe {
                                            st.fonts.font_info[(st.fonts.char_base[q as usize]
                                                + ec_char)
                                                as usize]
                                                .b16
                                        };
                                        let v = unsafe {
                                            match m {
                                                FONT_CHAR_WD_CODE => {
                                                    st.fonts.font_info[(st.fonts.width_base
                                                        [q as usize]
                                                        + i.s3 as i32)
                                                        as usize]
                                                        .b32
                                                        .s1
                                                }
                                                FONT_CHAR_HT_CODE => {
                                                    st.fonts.font_info[(st.fonts.height_base
                                                        [q as usize]
                                                        + i.s2 as i32 / 16)
                                                        as usize]
                                                        .b32
                                                        .s1
                                                }
                                                FONT_CHAR_DP_CODE => {
                                                    st.fonts.font_info[(st.fonts.depth_base
                                                        [q as usize]
                                                        + i.s2 as i32 % 16)
                                                        as usize]
                                                        .b32
                                                        .s1
                                                }
                                                _ => {
                                                    st.fonts.font_info[(st.fonts.italic_base
                                                        [q as usize]
                                                        + i.s1 as i32 / 4)
                                                        as usize]
                                                        .b32
                                                        .s1
                                                }
                                            }
                                        };
                                        set_cur_val(v);
                                    } else {
                                        set_cur_val(0);
                                    }
                                }
                            }
                            PAR_SHAPE_LENGTH_CODE
                            | PAR_SHAPE_INDENT_CODE
                            | PAR_SHAPE_DIMEN_CODE => {
                                q = cur_chr() - PAR_SHAPE_LENGTH_CODE;
                                scan_int_in(st)?;
                                let par_shape = st.eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
                                if par_shape == TEX_NULL || cur_val() <= 0 {
                                    set_cur_val(0);
                                } else {
                                    if q == 2 {
                                        q = cur_val() % 2;
                                        set_cur_val((cur_val() + q) / 2);
                                    }
                                    let cnt = st.mem.b32s0(par_shape);
                                    if cur_val() > cnt {
                                        set_cur_val(cnt);
                                    }
                                    set_cur_val(st.mem.b32s1(par_shape + 2 * cur_val() - q));
                                }
                                st.scanner.cur_val_level = DIMEN_VAL as u8;
                            }
                            GLUE_STRETCH_CODE | GLUE_SHRINK_CODE => {
                                scan_normal_glue_in(st)?;
                                q = cur_val();
                                if m == GLUE_STRETCH_CODE {
                                    set_cur_val(st.mem.b32s1(q + 2));
                                } else {
                                    set_cur_val(st.mem.b32s1(q + 3));
                                }
                                crate::mem::delete_glue_ref(&mut st.mem, q);
                            }
                            _ => {}
                        }
                        st.scanner.cur_val_level = DIMEN_VAL as u8;
                    } else {
                        // m >= INPUT_LINE_NO_CODE && m < XETEX_DIM
                        match m {
                            INPUT_LINE_NO_CODE => {
                                set_cur_val(crate::globals::line());
                            }
                            BADNESS_CODE => {
                                set_cur_val(st.shipout.last_badness);
                            }
                            ELAPSED_TIME_CODE => {
                                set_cur_val(unsafe { crate::texmfmp::get_microinterval() });
                            }
                            RANDOM_SEED_CODE => {
                                set_cur_val(st.errors.random_seed);
                            }
                            PDF_SHELL_ESCAPE_CODE => {
                                set_cur_val(if st.config.shell_escape_enabled { 1 } else { 0 });
                            }
                            ETEX_VERSION_CODE => {
                                set_cur_val(ETEX_VERSION);
                            }
                            XETEX_VERSION_CODE => {
                                set_cur_val(XETEX_VERSION);
                            }
                            XETEX_COUNT_GLYPHS_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                set_cur_val(if fa == AAT_FONT_FLAG {
                                    unsafe { aat_font_get(m - XETEX_INT, engine) }
                                } else if fa == OTGR_FONT_FLAG {
                                    unsafe { ot_font_get(m - XETEX_INT, engine) }
                                } else {
                                    0
                                });
                            }
                            XETEX_COUNT_FEATURES_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                set_cur_val(if fa == AAT_FONT_FLAG {
                                    unsafe { aat_font_get(m - XETEX_INT, engine) }
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    unsafe { ot_font_get(m - XETEX_INT, engine) }
                                } else {
                                    0
                                });
                            }
                            XETEX_VARIATION_CODE
                            | XETEX_VARIATION_MIN_CODE
                            | XETEX_VARIATION_MAX_CODE
                            | XETEX_VARIATION_DEFAULT_CODE
                            | XETEX_COUNT_VARIATIONS_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                set_cur_val(0);
                            }
                            XETEX_FEATURE_CODE_CODE
                            | XETEX_IS_EXCLUSIVE_FEATURE_CODE
                            | XETEX_COUNT_SELECTORS_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG {
                                    scan_int_in(st)?;
                                    k = cur_val();
                                    set_cur_val(unsafe {
                                        aat_font_get_1(m - XETEX_INT, engine, k)
                                    });
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    scan_int_in(st)?;
                                    k = cur_val();
                                    set_cur_val(unsafe { ot_font_get_1(m - XETEX_INT, engine, k) });
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an AAT or Graphite font".as_ptr(),
                                    ))?;
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_SELECTOR_CODE_CODE | XETEX_IS_DEFAULT_SELECTOR_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG {
                                    scan_int_in(st)?;
                                    k = cur_val();
                                    scan_int_in(st)?;
                                    set_cur_val(unsafe {
                                        aat_font_get_2(m - XETEX_INT, engine, k, cur_val())
                                    });
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    scan_int_in(st)?;
                                    k = cur_val();
                                    scan_int_in(st)?;
                                    set_cur_val(unsafe {
                                        ot_font_get_2(m - XETEX_INT, engine, k, cur_val())
                                    });
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an AAT or Graphite font".as_ptr(),
                                    ))?;
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_FIND_VARIATION_BY_NAME_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG {
                                    crate::io::scan_and_pack_name_in(st)?;
                                    set_cur_val(unsafe {
                                        aat_font_get_named(m - XETEX_INT, engine)
                                    });
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an AAT font".as_ptr(),
                                    ))?;
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_FIND_FEATURE_BY_NAME_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG {
                                    crate::io::scan_and_pack_name_in(st)?;
                                    set_cur_val(unsafe {
                                        aat_font_get_named(m - XETEX_INT, engine)
                                    });
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    crate::io::scan_and_pack_name_in(st)?;
                                    set_cur_val(unsafe {
                                        gr_font_get_named(m - XETEX_INT, engine)
                                    });
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an AAT or Graphite font".as_ptr(),
                                    ))?;
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_FIND_SELECTOR_BY_NAME_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG {
                                    scan_int_in(st)?;
                                    k = cur_val();
                                    crate::io::scan_and_pack_name_in(st)?;
                                    set_cur_val(unsafe {
                                        aat_font_get_named_1(m - XETEX_INT, engine, k)
                                    });
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    scan_int_in(st)?;
                                    k = cur_val();
                                    crate::io::scan_and_pack_name_in(st)?;
                                    set_cur_val(unsafe {
                                        gr_font_get_named_1(m - XETEX_INT, engine, k)
                                    });
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an AAT or Graphite font".as_ptr(),
                                    ))?;
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_OT_COUNT_SCRIPTS_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                set_cur_val(
                                    if fa == OTGR_FONT_FLAG && unsafe { usingOpenType(engine) } {
                                        unsafe { ot_font_get(m - XETEX_INT, engine) }
                                    } else {
                                        0
                                    },
                                );
                            }
                            XETEX_OT_COUNT_LANGUAGES_CODE | XETEX_OT_SCRIPT_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                if fa == OTGR_FONT_FLAG && unsafe { usingOpenType(engine) } {
                                    scan_int_in(st)?;
                                    set_cur_val(unsafe {
                                        ot_font_get_1(m - XETEX_INT, engine, cur_val())
                                    });
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an OpenType Layout font".as_ptr(),
                                    ))?;
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_OT_COUNT_FEATURES_CODE | XETEX_OT_LANGUAGE_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                if fa == OTGR_FONT_FLAG && unsafe { usingOpenType(engine) } {
                                    scan_int_in(st)?;
                                    k = cur_val();
                                    scan_int_in(st)?;
                                    set_cur_val(unsafe {
                                        ot_font_get_2(m - XETEX_INT, engine, k, cur_val())
                                    });
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an OpenType Layout font".as_ptr(),
                                    ))?;
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_OT_FEATURE_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                if fa == OTGR_FONT_FLAG && unsafe { usingOpenType(engine) } {
                                    scan_int_in(st)?;
                                    k = cur_val();
                                    scan_int_in(st)?;
                                    kk = cur_val();
                                    scan_int_in(st)?;
                                    set_cur_val(unsafe {
                                        ot_font_get_3(m - XETEX_INT, engine, k, kk, cur_val())
                                    });
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an OpenType Layout font".as_ptr(),
                                    ))?;
                                    set_cur_val(-1);
                                }
                            }
                            XETEX_MAP_CHAR_TO_GLYPH_CODE => {
                                let cf = st.eqtb.b32s1(CUR_FONT_LOC);
                                let fa = st.fonts.font_area[cf as usize];
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    scan_int_in(st)?;
                                    n = cur_val();
                                    let v =
                                        unsafe { crate::ext::map_char_to_glyph(&st.fonts, cf, n) };
                                    set_cur_val(v);
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        cf,
                                        c"; not a native platform font".as_ptr(),
                                    ))?;
                                    set_cur_val(0);
                                }
                            }
                            XETEX_GLYPH_INDEX_CODE => {
                                let cf = st.eqtb.b32s1(CUR_FONT_LOC);
                                let fa = st.fonts.font_area[cf as usize];
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    crate::io::scan_and_pack_name_in(st)?;
                                    let name = if st.input.name_of_file.is_empty() {
                                        std::ptr::null_mut()
                                    } else {
                                        st.input.name_of_file.as_ptr() as *mut libc::c_char
                                    };
                                    let v = unsafe {
                                        crate::ext::map_glyph_to_index(&st.fonts, cf, name)
                                    };
                                    set_cur_val(v);
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        cf,
                                        c"; not a native platform font".as_ptr(),
                                    ))?;
                                    set_cur_val(0);
                                }
                            }
                            XETEX_FONT_TYPE_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let engine = st.fonts.font_layout_engine[n as usize];
                                let fa = st.fonts.font_area[n as usize];
                                set_cur_val(if fa == AAT_FONT_FLAG {
                                    1
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingOpenType(engine) } {
                                    2
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    3
                                } else {
                                    0
                                });
                            }
                            XETEX_FIRST_CHAR_CODE | XETEX_LAST_CHAR_CODE => {
                                scan_font_ident_in(st)?;
                                n = cur_val();
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    let v = unsafe {
                                        crate::ext::get_font_char_range(
                                            &st.fonts,
                                            n,
                                            (m == XETEX_FIRST_CHAR_CODE) as i32,
                                        )
                                    };
                                    set_cur_val(v);
                                } else if m == XETEX_FIRST_CHAR_CODE {
                                    set_cur_val(st.fonts.font_bc[n as usize] as i32);
                                } else {
                                    set_cur_val(st.fonts.font_ec[n as usize] as i32);
                                }
                            }
                            PDF_LAST_X_POS_CODE => {
                                set_cur_val(st.shipout.pdf_last_x_pos);
                            }
                            PDF_LAST_Y_POS_CODE => {
                                set_cur_val(st.shipout.pdf_last_y_pos);
                            }
                            XETEX_PDF_PAGE_COUNT_CODE => {
                                crate::io::scan_and_pack_name_in(st)?;
                                let name = if st.input.name_of_file.is_empty() {
                                    std::ptr::null_mut()
                                } else {
                                    st.input.name_of_file.as_ptr() as *const libc::c_char
                                };
                                set_cur_val(unsafe {
                                    crate::pic::count_pdf_file_pages_named(name)
                                });
                            }
                            CURRENT_GROUP_LEVEL_CODE => {
                                set_cur_val(st.save_stack.cur_level as i32 - 1);
                            }
                            CURRENT_GROUP_TYPE_CODE => {
                                set_cur_val(st.save_stack.cur_group as i32);
                            }
                            CURRENT_IF_LEVEL_CODE => {
                                q = st.scanner.cond_ptr;
                                let mut cnt = 0i32;
                                while q != TEX_NULL {
                                    cnt += 1;
                                    q = st.mem.b32s1(q);
                                }
                                set_cur_val(cnt);
                            }
                            CURRENT_IF_TYPE_CODE => {
                                if st.scanner.cond_ptr == TEX_NULL {
                                    set_cur_val(0);
                                } else if (st.scanner.cur_if as i32) < UNLESS_CODE {
                                    set_cur_val(st.scanner.cur_if as i32 + 1);
                                } else {
                                    set_cur_val(-(st.scanner.cur_if as i32 - 31));
                                }
                            }
                            CURRENT_IF_BRANCH_CODE => {
                                let ifl = st.scanner.if_limit;
                                if ifl == OR_CODE || ifl == ELSE_CODE {
                                    set_cur_val(1);
                                } else if ifl == FI_CODE {
                                    set_cur_val(-1);
                                } else {
                                    set_cur_val(0);
                                }
                            }
                            GLUE_STRETCH_ORDER_CODE | GLUE_SHRINK_ORDER_CODE => {
                                scan_normal_glue_in(st)?;
                                q = cur_val();
                                if m == GLUE_STRETCH_ORDER_CODE {
                                    set_cur_val(st.mem.b16s1(q) as i32);
                                } else {
                                    set_cur_val(st.mem.b16s0(q) as i32);
                                }
                                crate::mem::delete_glue_ref(&mut st.mem, q);
                            }
                            _ => {}
                        }
                        st.scanner.cur_val_level = INT_VAL as u8;
                    }
                } else {
                    // m < INPUT_LINE_NO_CODE: last-node queries
                    set_cur_val(0);
                    tx = cur_list_tail();
                    if tx < st.mem.hi_mem_min {
                        let node_type = st.mem.b16s1(tx);
                        let subtype = st.mem.b16s0(tx);
                        if node_type == MATH_NODE && subtype == END_M_CODE {
                            r = cur_list_head();
                            loop {
                                q = r;
                                r = st.mem.b32s1(q);
                                if r == tx {
                                    break;
                                }
                            }
                            tx = q;
                        }
                    }
                    if cur_chr() == LAST_NODE_TYPE_CODE_VAL {
                        st.scanner.cur_val_level = INT_VAL as u8;
                        if tx == cur_list_head() || cur_list_mode() == 0 {
                            set_cur_val(-1);
                        }
                    } else {
                        st.scanner.cur_val_level = cur_chr() as u8;
                    }
                    if tx < st.mem.hi_mem_min && cur_list_mode() != 0 {
                        let node_type = st.mem.b16s1(tx);
                        match cur_chr() {
                            x if x == INT_VAL as i32 => {
                                if node_type == PENALTY_NODE {
                                    set_cur_val(st.mem.b32s1(tx + 1));
                                }
                            }
                            x if x == DIMEN_VAL as i32 => {
                                if node_type == KERN_NODE {
                                    set_cur_val(st.mem.b32s1(tx + 1));
                                }
                            }
                            x if x == GLUE_VAL as i32 => {
                                if node_type == GLUE_NODE {
                                    set_cur_val(st.mem.b32s0(tx + 1));
                                    if st.mem.b16s0(tx) == MU_GLUE {
                                        st.scanner.cur_val_level = MU_VAL as u8;
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
                            x if x == INT_VAL as i32 => set_cur_val(st.shipout.last_penalty),
                            x if x == DIMEN_VAL as i32 => set_cur_val(st.shipout.last_kern),
                            x if x == GLUE_VAL as i32 => {
                                if st.shipout.last_glue != MAX_HALFWORD {
                                    set_cur_val(st.shipout.last_glue);
                                }
                            }
                            x if x == LAST_NODE_TYPE_CODE_VAL => {
                                set_cur_val(st.shipout.last_node_type)
                            }
                            _ => {}
                        }
                    }
                }
            }
            IGNORE_SPACES => {
                if cur_chr() == 1 {
                    get_token_in(st)?;
                    if cur_cs() < HASH_BASE {
                        let v = crate::hash::prim_lookup(
                            &mut st.eqtb,
                            &mut st.strings,
                            cur_cs() - SINGLE_BASE,
                        )
                        .map_err(|e| SccAbort::Input(hash_err_abort(e)))?;
                        set_cur_cs(v);
                    } else {
                        let s = st.eqtb.hash[(cur_cs() as usize) - 514].s1;
                        let v = crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, s)
                            .map_err(|e| SccAbort::Input(hash_err_abort(e)))?;
                        set_cur_cs(v);
                    }
                    if cur_cs() != UNDEFINED_PRIMITIVE {
                        set_cur_cmd(st.eqtb.b16s1(PRIM_EQTB_BASE + cur_cs()) as u8);
                        set_cur_chr(st.eqtb.b32s1(PRIM_EQTB_BASE + cur_cs()));
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
                error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
                print_cmd_chr_in(st, cur_cmd() as u16, cur_chr());
                print_cstr_in(st, c"' after ".as_ptr());
                print_esc_cstr_in(st, c"the".as_ptr());
                capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 1;
                st.errors.help_line[0] =
                    c"I'm forgetting what you said and using zero instead.".as_ptr();
                scc_err(crate::errors::error_in(st))?;
                set_cur_val(0);
                if level != TOK_VAL {
                    st.scanner.cur_val_level = DIMEN_VAL as u8;
                } else {
                    st.scanner.cur_val_level = INT_VAL as u8;
                }
            }
        }
        break; // end 'restart loop
    }

    // Coerce cur_val to the requested level.
    while (st.scanner.cur_val_level as i16) > level {
        if (st.scanner.cur_val_level as i16) == GLUE_VAL {
            set_cur_val(st.mem.b32s1(cur_val() + 1));
        } else if (st.scanner.cur_val_level as i16) == MU_VAL {
            scc_err(crate::errors::mu_error_in(st))?;
        }
        st.scanner.cur_val_level -= 1;
    }

    if negative {
        if (st.scanner.cur_val_level as i16) >= GLUE_VAL {
            let gm = cur_val();
            let nv = new_spec_scc(st, gm)?;
            set_cur_val(nv);
            let v1 = -(st.mem.b32s1(cur_val() + 1));
            st.mem.set_b32s1(cur_val() + 1, v1);
            let v2 = -(st.mem.b32s1(cur_val() + 2));
            st.mem.set_b32s1(cur_val() + 2, v2);
            let v3 = -(st.mem.b32s1(cur_val() + 3));
            st.mem.set_b32s1(cur_val() + 3, v3);
        } else {
            set_cur_val(-cur_val());
        }
    } else if (st.scanner.cur_val_level as i16) >= GLUE_VAL
        && (st.scanner.cur_val_level as i16) <= MU_VAL
    {
        // GLUE_SPEC_ref_count(cur_val)++
        let v = st.mem.b32s1(cur_val()) + 1;
        st.mem.set_b32s1(cur_val(), v);
    }
    Ok(())
}
pub(crate) fn scan_char_num_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if cur_val() < 0 || cur_val() > BIGGEST_CHAR {
        leaf_int_range_error(
            st,
            c"Bad character code".as_ptr(),
            c"A character number must be between 0 and 65535.".as_ptr(),
        )?;
    }
    Ok(())
}
pub(crate) fn scan_usv_num_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if cur_val() < 0 || cur_val() > BIGGEST_USV {
        leaf_int_range_error(
            st,
            c"Bad character code".as_ptr(),
            c"A Unicode scalar value must be between 0 and \"10FFFF.".as_ptr(),
        )?;
    }
    Ok(())
}
pub(crate) fn scan_char_class_not_ignored_in(
    st: &mut crate::state::EngineState,
) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if cur_val() < 0 || cur_val() > CHAR_CLASS_LIMIT {
        leaf_int_range_error(
            st,
            c"Bad character class".as_ptr(),
            c"A class for inter-character transitions must be between 0 and 4095.".as_ptr(),
        )?;
    }
    Ok(())
}
pub(crate) fn scan_math_fam_int_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if cur_val() < 0 || cur_val() > NUMBER_MATH_FAMILIES - 1 {
        leaf_int_range_error(
            st,
            c"Bad math family".as_ptr(),
            c"Since I expected to read a number between 0 and 255,".as_ptr(),
        )?;
    }
    Ok(())
}
pub(crate) fn scan_font_ident_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let f;
    loop {
        get_x_token_in(st)?;
        if cur_cmd() != SPACER {
            break;
        }
    }
    if cur_cmd() == DEF_FONT {
        f = st.eqtb.b32s1(CUR_FONT_LOC);
    } else if cur_cmd() == SET_FONT {
        f = cur_chr();
    } else if cur_cmd() == DEF_FAMILY {
        let m = cur_chr();
        scan_math_fam_int_in(st)?;
        f = st.eqtb.b32s1(m + cur_val());
    } else {
        error_here_with_diagnostic_in(st, c"Missing font identifier".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] = c"I was looking for a control sequence whose".as_ptr();
        st.errors.help_line[0] = c"current meaning has been defined by \\font.".as_ptr();
        let oc = crate::input::back_error_in(st)?;
        scc_err(oc)?;
        f = FONT_BASE;
    }
    set_cur_val(f);
    Ok(())
}
pub(crate) fn find_font_dimen_in(
    st: &mut crate::state::EngineState,
    writing: bool,
) -> Result<(), SccAbort> {
    use crate::mem::TEX_NULL;

    scan_int_in(st)?;
    let n = cur_val();
    scan_font_ident_in(st)?;
    let f = cur_val();

    if n <= 0 {
        set_cur_val(st.fonts.fmem_ptr);
    } else {
        if writing
            && (SPACE_CODE..=SPACE_SHRINK_CODE).contains(&n)
            && st.fonts.font_glue[f as usize] != TEX_NULL
        {
            let g = st.fonts.font_glue[f as usize];
            crate::mem::delete_glue_ref(&mut st.mem, g);
            st.fonts.font_glue[f as usize] = TEX_NULL;
        }

        if n > st.fonts.font_params[f as usize] {
            if f < st.fonts.font_ptr {
                set_cur_val(st.fonts.fmem_ptr);
            } else {
                loop {
                    if st.fonts.fmem_ptr == st.fonts.font_mem_size {
                        return Err(SccAbort::Error(crate::errors::ErrorOutcome::Overflow {
                            what: c"font memory",
                            n: st.fonts.font_mem_size,
                        }));
                    }
                    let k = st.fonts.fmem_ptr;
                    st.fonts.font_info[k as usize].b32.s1 = 0;
                    st.fonts.fmem_ptr += 1;
                    st.fonts.font_params[f as usize] += 1;
                    if n == st.fonts.font_params[f as usize] {
                        break;
                    }
                }
                set_cur_val(st.fonts.fmem_ptr - 1);
            }
        } else {
            set_cur_val(n + st.fonts.param_base[f as usize]);
        }
    }

    if cur_val() == st.fonts.fmem_ptr {
        error_here_with_diagnostic_in(st, c"Font ".as_ptr());
        let s1 = st.eqtb.hash[((FONT_ID_BASE + f) as usize) - 514].s1;
        crate::output::print_esc_in(st, s1);
        crate::output::print_cstr_in(st, c" has only ".as_ptr());
        let fp = st.fonts.font_params[f as usize];
        crate::output::print_int_in(st, fp);
        crate::output::print_cstr_in(st, c" fontdimen parameters".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] = c"To increase the number of font parameters, you must".as_ptr();
        st.errors.help_line[0] =
            c"use \\fontdimen immediately after the \\font is loaded.".as_ptr();
        scc_err(crate::errors::error_in(st))?;
    }
    Ok(())
}
pub(crate) fn scan_glyph_number_in(
    st: &mut crate::state::EngineState,
    f: i32,
) -> Result<(), SccAbort> {
    if scan_keyword_in(st, c"/".as_ptr())? {
        crate::io::scan_and_pack_name_in(st)?;
        // `name_of_file` read directly (its `_global` getter is with_state-bearing).
        let name = if st.input.name_of_file.is_empty() {
            std::ptr::null_mut()
        } else {
            st.input.name_of_file.as_ptr() as *mut libc::c_char
        };
        let v = unsafe { crate::ext::map_glyph_to_index(&st.fonts, f, name) };
        set_cur_val(v);
        st.scanner.cur_val_level = INT_VAL as u8;
    } else if scan_keyword_in(st, c"u".as_ptr())? {
        scan_char_num_in(st)?;
        let v = unsafe { crate::ext::map_char_to_glyph(&st.fonts, f, cur_val()) };
        set_cur_val(v);
        st.scanner.cur_val_level = INT_VAL as u8;
    } else {
        scan_int_in(st)?;
    }
    Ok(())
}
/// Threaded core of `get_token` (`01b` §3). Not itself an SCC member, but
/// reached under the borrow by several SCC scanners, so it needs an `_in` form.
pub(crate) fn get_token_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    st.eqtb.no_new_control_sequence = false;
    get_next_scc(st)?;
    st.eqtb.no_new_control_sequence = true;
    if cur_cs() == 0 {
        set_cur_tok(cur_cmd() as i32 * MAX_CHAR_VAL + cur_chr());
    } else {
        set_cur_tok(CS_TOKEN_FLAG + cur_cs());
    }
    Ok(())
}
pub(crate) fn scan_general_text_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let s = st.scanner.scanner_status;
    let w = st.scanner.warning_index;
    let d = st.scanner.def_ref;
    st.scanner.scanner_status = ABSORBING;
    st.scanner.warning_index = cur_cs();
    let new_def = crate::mem::get_avail(&mut st.mem);
    st.scanner.def_ref = new_def;
    st.mem.set_b32s0(new_def, crate::mem::TEX_NULL);
    let mut p = new_def;
    scan_left_brace_in(st)?;
    let mut unbalance: i32 = 1;
    loop {
        get_token_in(st)?;
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
        let q = crate::mem::get_avail(&mut st.mem);
        st.mem.set_b32s1(p, q);
        st.mem.set_b32s0(q, cur_tok());
        p = q;
    }
    // found:
    let dr = st.scanner.def_ref;
    let q = st.mem.b32s1(dr);
    let av = st.mem.avail;
    st.mem.set_b32s1(dr, av);
    st.mem.avail = dr;
    if q == crate::mem::TEX_NULL {
        set_cur_val(TEMP_HEAD);
    } else {
        set_cur_val(p);
    }
    st.mem.set_b32s1(TEMP_HEAD, q);
    st.scanner.scanner_status = s;
    st.scanner.warning_index = w;
    st.scanner.def_ref = d;
    Ok(())
}
pub(crate) fn scan_pdf_ext_toks_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_toks_in(st, false, true)?;
    Ok(())
}
pub(crate) fn compare_strings_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let save_cur_cs = cur_cs();
    scan_toks_in(st, false, true)?;
    let dr1 = st.scanner.def_ref;
    let s1 = tokens_to_string_in(st, dr1).map_err(SccAbort::TokensToString)?;
    let dr1 = st.scanner.def_ref;
    crate::mem::delete_token_ref(&mut st.mem, dr1);
    set_cur_cs(save_cur_cs);
    scan_toks_in(st, false, true)?;
    let dr2 = st.scanner.def_ref;
    let s2 = tokens_to_string_in(st, dr2).map_err(SccAbort::TokensToString)?;
    let dr2 = st.scanner.def_ref;
    crate::mem::delete_token_ref(&mut st.mem, dr2);

    let i1 = st.strings.str_start[(s1 - 65536) as usize];
    let j1 = st.strings.str_start[(s1 + 1 - 65536) as usize];
    let i2 = st.strings.str_start[(s2 - 65536) as usize];
    let j2 = st.strings.str_start[(s2 + 1 - 65536) as usize];

    let result = 'done: {
        let mut i1 = i1;
        let mut i2 = i2;
        while i1 < j1 && i2 < j2 {
            if st.strings.str_pool[i1 as usize] < st.strings.str_pool[i2 as usize] {
                break 'done -1i32;
            }
            if st.strings.str_pool[i1 as usize] > st.strings.str_pool[i2 as usize] {
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

    crate::string_pool::flush_str(&mut st.strings, s2);
    crate::string_pool::flush_str(&mut st.strings, s1);
    set_cur_val(result);
    st.scanner.cur_val_level = INT_VAL as u8;
    Ok(())
}

// ---------------------------------------------------------------------------
// main_control_main_loop
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// main_control
// ---------------------------------------------------------------------------
