//! Scanner range-validation helpers and mu_error.
//!
//! Translated from xetex-xetex0.c.
use crate::consts::{
    ACTIVE_BASE, ASSIGN_GLUE, ASSIGN_INT, ASSIGN_MU_GLUE, BACKED_UP, BACKUP_HEAD, BIGGEST_CHAR,
    BOX_BASE, CAT_CODE_BASE, CHAR_CLASS_LIMIT, CHAR_GIVEN, CLOSED, COUNT_BASE, CS_TOKEN_FLAG,
    CUR_FONT_LOC, DEL_CODE_BASE, ELSE_CODE, ENDV, END_TEMPLATE, EQTB_SIZE, FI_CODE, FI_OR_ELSE,
    FONT_BASE, FROZEN_CONTROL_SEQUENCE, FROZEN_ENDV, FROZEN_PRIMITIVE, GLUE_NODE, GLUE_VAL,
    INT_BASE, INT_PAR_END_LINE_CHAR, INT_PAR_NEW_LINE_CHAR, INT_PAR_TRACING_IFS, JUST_OPEN,
    KERN_NODE, LEFT_BRACE, LEFT_SIDE, LOCAL_BASE, LOCAL_PAR_SHAPE, MATH_CODE_BASE, MATH_NODE,
    MAX_COMMAND, MAX_HALFWORD, MU_GLUE, MU_SKIP_BASE, MU_VAL_SCAN, NEW_LINE, NORMAL_SCANNER,
    NUMBER_MATH_FAMILIES, OR_CODE, PENALTY_NODE, PRIM_EQTB_BASE, REGISTER, RELAX, RIGHT_BRACE,
    RIGHT_SIDE, SCALED_BASE, SELECTOR_NEW_STRING, SET_FONT, SF_CODE_BASE, SINGLE_BASE, SKIP_BASE,
    SPACER, SPACE_CODE, TEX_NULL, TOKS_BASE, TOO_BIG_CHAR, UNDEFINED_PRIMITIVE, UNLESS_CODE, VMODE,
    XETEX_INT,
};
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
use crate::state::GroupCode;

use crate::errors::int_error_in;
use crate::expand::{expand_in, macro_call_in};
use crate::output::capture_to_diagnostic_in;
use crate::output::error_here_with_diagnostic_in;
use crate::output::print_cmd_chr_in;
use crate::output::print_cstr_in;
use crate::output::print_esc_cstr_in;
use crate::output::print_int_in;
use crate::output::print_nl_cstr_in;

use crate::node::{GlueSpecNode, NodePtr, RuleDimen};
use crate::units::{MemIdx, MemPtr, NodeType, Scaled};

use crate::bridge::{diag_append_str, ttstub_output_flush};

extern "C" {

    fn usingGraphite(engine: *mut libc::c_void) -> bool;
    fn usingOpenType(engine: *mut libc::c_void) -> bool;

    fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;

}

// Value-type constants for scan_glue (xetex_format.h:568–570).

// INT_BASE + INT_PAR__xetex_hyphenatable_length offset for max_hyphenatable_length
// (xetex_format.h:568, xetex_format.h:115; xetex_format.h:7826729 = INT_BASE).
const INT_PAR_XETEX_HYPHENATABLE_LENGTH: i32 = 80; // xetex_format.h:115
const HYPHENATABLE_LENGTH_LIMIT: i32 = 4095; // xetex-constants.h:334

// xetex-constants.h:26
const BIGGEST_USV: i32 = 0x10_FFFF;
// xetex-constants.h:335
// xetex-constants.h:343
const ACTIVE_MATH_CHAR: i32 = 0x1F_FFFF;
// xetex-constants.h:25
// xetex_format.h (NUMBER_MATH_FAMILIES)

// --- Constants for scan_something_internal / scan_int ---
// Command codes (xetex_format.h).
const DEF_CODE: u8 = 86;
const XETEX_DEF_CODE: u8 = 87;
const DEF_FAMILY: u8 = 88;
const DEF_FONT: u8 = 90;
const TOKS_REGISTER: u8 = 72;
const ASSIGN_TOKS: u8 = 73;
const ASSIGN_DIMEN: u8 = 75;
const ASSIGN_FONT_DIMEN: u8 = 78;
const ASSIGN_FONT_INT: u8 = 79;
const SET_AUX: u8 = 80;
const SET_PREV_GRAF: u8 = 81;
const SET_PAGE_DIMEN: u8 = 82;
const SET_PAGE_INT: u8 = 83;
const SET_BOX_DIMEN: u8 = 84;
const SET_SHAPE: u8 = 85;
const MATH_GIVEN: u8 = 69;
const XETEX_MATH_GIVEN: u8 = 70;
const LAST_ITEM: u8 = 71;
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
// eqtb bases (xetex_format.h).
const FONT_ID_BASE: i32 = 2_245_338; // FROZEN_NULL_FONT
const HASH_BASE: i32 = 2_228_226;
const FROZEN_RELAX: i32 = 2_243_233; // FROZEN_CONTROL_SEQUENCE + 7
                                     // LOCAL__ offsets (xetex_format.h:172,183).
const LOCAL_XETEX_INTER_CHAR_TOKS: i32 = 11;
// Font flags (xetex-ext.h:47–48).
const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 0xFFFE;
// LEFT/RIGHT side (xetex_layout/src/c_api.rs).
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
const XETEX_DIM: i32 = 55;
const LAST_NODE_TYPE_CODE_VAL: i32 = 3; // distinct name from other LAST_NODE_ uses
                                        // Node type codes (xetex_format.h:389–437).
const UNSET_NODE: u16 = 13;
const END_M_CODE: u16 = 3;
// Conditional codes (xetex_format.h:537–539).
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
pub(crate) fn scan_char_class_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > CHAR_CLASS_LIMIT {
        leaf_int_range_error(
            st,
            c"Bad character class".as_ptr(),
            c"A character class must be between 0 and 4096.".as_ptr(),
        )?;
    }
    Ok(())
}

// translated from xetex-xetex0.c:5096 (scan_char_class_not_ignored)
pub(crate) fn scan_eight_bit_int_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > 255 {
        leaf_int_range_error(
            st,
            c"Bad register code".as_ptr(),
            c"A register code or char class must be between 0 and 255.".as_ptr(),
        )?;
    }
    Ok(())
}

// translated from xetex-xetex0.c:5330 (scan_math_class_int)
pub(crate) fn scan_math_class_int_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > 7 {
        leaf_int_range_error(
            st,
            c"Bad math class".as_ptr(),
            c"Since I expected to read a number between 0 and 7,".as_ptr(),
        )?;
    }
    Ok(())
}

// translated from xetex-xetex0.c:5348 (scan_math_fam_int)
// translated from xetex-xetex0.c:5366 (scan_four_bit_int)
pub(crate) fn scan_four_bit_int_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > 15 {
        leaf_int_range_error(
            st,
            c"Bad number".as_ptr(),
            c"Since I expected to read a number between 0 and 15,".as_ptr(),
        )?;
    }
    Ok(())
}

// translated from xetex-xetex0.c:5384 (scan_fifteen_bit_int)
pub(crate) fn scan_fifteen_bit_int_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > 32767 {
        leaf_int_range_error(
            st,
            c"Bad mathchar".as_ptr(),
            c"A mathchar number must be between 0 and 32767.".as_ptr(),
        )?;
    }
    Ok(())
}

// translated from xetex-xetex0.c:5168 (scan_xetex_math_char_int)
// math_char(x) = (x as u32) & 0x1FFFFF (xetex-xetexd.h:1205)
pub(crate) fn scan_xetex_math_char_int_in(
    st: &mut crate::state::EngineState,
) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    let mc = (st.scanner.cur_val as u32) & 0x1F_FFFF;
    if mc == ACTIVE_MATH_CHAR as u32 {
        if st.scanner.cur_val != ACTIVE_MATH_CHAR {
            let errmsg = error_here_with_diagnostic_in(st, c"Bad active XeTeX math code".as_ptr());
            unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val)) };
            capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 2;
            st.errors.help_line[1] =
                c"Since I ignore class and family for active math chars,".as_ptr();
            st.errors.help_line[0] = c"I changed this one to \"1FFFFF.".as_ptr();
            scc_err(int_error_in(st, st.scanner.cur_val))?;
            st.scanner.cur_val = ACTIVE_MATH_CHAR;
        }
    } else if mc > BIGGEST_USV as u32 {
        let errmsg = error_here_with_diagnostic_in(st, c"Bad XeTeX math character code".as_ptr());
        unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val)) };
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] =
            c"Since I expected a character number between 0 and \"10FFFF,".as_ptr();
        st.errors.help_line[0] = c"I changed this one to zero.".as_ptr();
        scc_err(int_error_in(st, st.scanner.cur_val))?;
        st.scanner.cur_val = 0;
    }
    Ok(())
}

pub(crate) fn scan_delimiter_int_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > 0x7FF_FFFF {
        let errmsg = error_here_with_diagnostic_in(st, c"Bad delimiter code".as_ptr());
        unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val)) };
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] =
            c"A numeric delimiter code must be between 0 and 2^{27}-1.".as_ptr();
        st.errors.help_line[0] = c"I changed this one to zero.".as_ptr();
        scc_err(int_error_in(st, st.scanner.cur_val))?;
        st.scanner.cur_val = 0;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// max_hyphenatable_length
// ---------------------------------------------------------------------------

/// Reads the `\XeTeXhyphenatablelength` integer parameter from `eqtb`.
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
// x_token / scan_optional_equals
// ---------------------------------------------------------------------------

// Command-code constants (xetex_format.h / xetex-constants.h).
const MAX_CHAR_VAL: i32 = 0x200000; // 1 << 21
const OTHER_TOKEN: i32 = 0x1800000; // OTHER_CHAR << 21
const CALL: u8 = 113;
// FROZEN_CONTROL_SEQUENCE + 6 (xetex_format.h:201, xetex-constants.h:61)
// END_MATCH_TOKEN + 1 (xetex-constants.h:360–361)
const PROTECTED_TOKEN: i32 = 0x1C00001;

// Token-encoding constants (xetex-constants.h:353).
const MATH_SHIFT_TOKEN: i32 = 0x600000;

pub(crate) fn insert_dollar_sign_in(
    st: &mut crate::state::EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    crate::input::back_input_in(st)?;
    st.scanner.cur_tok = MATH_SHIFT_TOKEN + '$' as i32;
    crate::output::error_here_with_diagnostic_in(st, c"Missing $ inserted".as_ptr());
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 2;
    st.errors.help_line[1] = c"I've inserted a begin-math/end-math symbol since I think".as_ptr();
    st.errors.help_line[0] = c"you left one out. Proceed, with fingers crossed.".as_ptr();
    crate::main_control::mc_propagate_error(crate::input::ins_error_in(st)?)
}

pub(crate) fn scan_optional_equals_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    loop {
        get_x_token_in(st)?;
        if st.scanner.cur_cmd != SPACER {
            break;
        }
    }
    if st.scanner.cur_tok != OTHER_TOKEN + '=' as i32 {
        crate::input::back_input_in(st)?;
    }
    Ok(())
}

// Command-code values for scan_left_brace (xetex_format.h).
const LEFT_BRACE_TOKEN: i32 = 0x200000;

// ---------------------------------------------------------------------------
// get_x_token / get_x_or_protected
// ---------------------------------------------------------------------------

pub(crate) fn get_x_or_protected_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    loop {
        get_token_in(st)?;
        if st.scanner.cur_cmd <= MAX_COMMAND {
            return Ok(());
        }
        if st.scanner.cur_cmd >= CALL
            && st.scanner.cur_cmd < END_TEMPLATE
            && st.mem.b32s0(st.mem.b32s1(st.scanner.cur_chr)) == PROTECTED_TOKEN
        {
            return Ok(());
        }
        expand_in(st)?;
    }
}

// ---------------------------------------------------------------------------
// scan_keyword
// ---------------------------------------------------------------------------

// MEM_TOP - 13 (xetex_format.h:30, xetex-constants.h:50)
// xetex-constants.h:202

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

/// The save stack lives in `st.save_stack`; the only cold abort is
/// `new_save_level`'s save-stack overflow, surfaced through [`SccAbort`].
pub(crate) fn scan_spec_in(
    st: &mut crate::state::EngineState,
    c: GroupCode,
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
        st.scanner.cur_val = 0;
    }
    // found:
    if three_codes {
        let sp = st.save_stack.save_ptr;
        st.save_stack.set_b32s1(sp, s);
        st.save_stack.save_ptr = sp + 1;
    }
    let sp = st.save_stack.save_ptr;
    st.save_stack.set_b32s1(sp, spec_code as i32);
    st.save_stack.set_b32s1(sp + 1, st.scanner.cur_val);
    st.save_stack.save_ptr = sp + 2;
    if let Some(abort) = crate::save::new_save_level(&mut st.save_stack, c, st.mem.line) {
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
fn round_decimals(errors: &crate::state::ErrorState, k: i32) -> i32 {
    let mut a: i32 = 0;
    let mut k = k;
    while k > 0 {
        k -= 1;
        a = (a + errors.dig[k as usize] as i32 * 0x2_0000) / 10;
    }
    (a + 1) / 2
}

// ---------------------------------------------------------------------------
// scan_dimen / scan_decimal
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// scan_rule_spec
// ---------------------------------------------------------------------------

// xetex_format.h:273 / xetex-constants.h:316,339
const VRULE: u8 = 35;
const DEFAULT_RULE: i32 = 26214;

pub(crate) fn scan_rule_spec_in(st: &mut crate::state::EngineState) -> Result<i32, SccAbort> {
    let Some(r) = crate::mem::alloc_rule(&mut st.mem) else {
        return Err(main_mem_overflow_scc());
    };
    if st.scanner.cur_cmd == VRULE {
        r.set_width(&mut st.mem, RuleDimen::from_raw(DEFAULT_RULE));
    } else {
        r.set_height(&mut st.mem, RuleDimen::from_raw(DEFAULT_RULE));
        r.set_depth(&mut st.mem, RuleDimen::Set(Scaled(0)));
    }
    loop {
        if scan_keyword_in(st, c"width".as_ptr())? {
            xetex_scan_dimen_in(st, false, false, false, true)?;
            r.set_width(&mut st.mem, RuleDimen::from_raw(st.scanner.cur_val));
        } else if scan_keyword_in(st, c"height".as_ptr())? {
            xetex_scan_dimen_in(st, false, false, false, true)?;
            r.set_height(&mut st.mem, RuleDimen::from_raw(st.scanner.cur_val));
        } else if scan_keyword_in(st, c"depth".as_ptr())? {
            xetex_scan_dimen_in(st, false, false, false, true)?;
            r.set_depth(&mut st.mem, RuleDimen::from_raw(st.scanner.cur_val));
        } else {
            break;
        }
    }
    Ok(r.idx().0)
}

// xetex-constants.h:204

// ---------------------------------------------------------------------------
// pseudo_start
// ---------------------------------------------------------------------------

// INT_BASE = 7_826_729 (xetex_format.h); INT_PAR__new_line_char = 49
const INT_PAR_TRACING_SCAN_TOKENS: i32 = 59; // xetex_format.h:94
                                             // xetex-core.h / xetex-xetexd.h:430

// ---------------------------------------------------------------------------
// str_toks_cat / str_toks
// ---------------------------------------------------------------------------

// xetex-constants.h:356
const SPACE_TOKEN: i32 = 0x1400020;
// xetex_format.h:246
const ACTIVE_CHAR: i32 = 13;

/// translated from xetex-xetex0.c:6363 (str_toks_cat)
/// Cold abort reasons for [`str_toks_cat_in`].
pub(crate) enum StrToksAbort {
    /// `pool_ptr + 1 > pool_size`: `overflow("pool size", …)`.
    PoolOverflow,
    /// `get_avail` exhausted the arena: `runaway()` then `overflow`.
    MainMemoryOverflow,
}

/// Both cold overflow paths defer to the caller via [`StrToksAbort`].
pub(crate) fn str_toks_cat_in(
    mem: &mut crate::state::MemState,
    strings: &mut crate::state::StringsState,
    b: i32,
    cat: i16,
) -> Result<crate::units::MemPtr, StrToksAbort> {
    if strings.pool_ptr + 1 > strings.pool_size {
        return Err(StrToksAbort::PoolOverflow);
    }

    let mut p = crate::units::MemPtr::from_raw(TEMP_HEAD);
    p.set_link(mem, crate::units::MemPtr::NULL);
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

        let q = crate::mem::get_avail(mem).ok_or(StrToksAbort::MainMemoryOverflow)?;
        p.set_link(mem, q);
        q.set_info(mem, t);
        p = q;
        k += 1;
    }

    strings.pool_ptr = b;
    Ok(p)
}

// ---------------------------------------------------------------------------
// the_toks
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// pass_text
// ---------------------------------------------------------------------------

// xetex_format.h:349,350,537; xetex-constants.h:183
const SKIPPING: u8 = 1;
const IF_TEST: u8 = 107;
// INT_PAR__tracing_ifs = 58 (xetex_format.h:93)

// ---------------------------------------------------------------------------
// scan_toks
// ---------------------------------------------------------------------------

// xetex-constants.h:182,184
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

// ---------------------------------------------------------------------------
// read_toks
// ---------------------------------------------------------------------------

// xetex-constants.h:268,277,223
// INT_PAR__end_line_char = 48 (xetex_format.h:83)

/// translated from xetex-xetex0.c:7268 (read_toks)
pub(crate) fn read_toks_in(
    st: &mut crate::state::EngineState,
    n: i32,
    r: i32,
    j: i32,
) -> Result<(), SccAbort> {
    use crate::mem::TEX_NULL;

    st.scanner.scanner_status = DEFINING;
    st.scanner.warning_index = r;
    let new_def = get_avail_scc(st)?;
    st.scanner.def_ref = new_def;
    new_def.set_info(&mut st.mem, TEX_NULL);
    let mut p = new_def;

    let q = get_avail_scc(st)?;
    p.set_link(&mut st.mem, q);
    q.set_info(&mut st.mem, END_MATCH_TOKEN);
    p = q;

    let m: usize = if !(0..=15).contains(&n) {
        16
    } else {
        n as usize
    };

    let s = st.scanner.align_state;
    st.scanner.align_state = 1_000_000;

    loop {
        // do…while(align_state != 1_000_000) — §502
        crate::input::begin_file_reading_in(st)?;
        st.input.cur_input.name = (m + 1) as i32;

        if st.input.read_open[m] == CLOSED {
            return Err(SccAbort::Input(crate::input::InputAbort::TtAbort(
                c"terminal input forbidden".as_ptr(),
            )));
        } else if st.input.read_open[m] == JUST_OPEN {
            // §504
            let rf = st.input.read_file_ptr(m);
            if unsafe { crate::io::input_line_in(st, rf)? } != 0 {
                st.input.read_open[m] = NORMAL_SCANNER; // NORMAL = 0
            } else {
                let slot = std::mem::take(&mut st.input.read_file[m]);
                if unsafe { crate::io::u_close(slot) } {
                    return Err(SccAbort::InputClose);
                }
                st.input.read_open[m] = CLOSED;
            }
        } else {
            // §505
            let rf = st.input.read_file_ptr(m);
            if unsafe { crate::io::input_line_in(st, rf)? } == 0 {
                let slot = std::mem::take(&mut st.input.read_file[m]);
                if unsafe { crate::io::u_close(slot) } {
                    return Err(SccAbort::InputClose);
                }
                st.input.read_open[m] = CLOSED;
                if st.scanner.align_state != 1_000_000 {
                    crate::errors::runaway_in(st);
                    error_here_with_diagnostic_in(st, c"File ended within ".as_ptr());
                    crate::output::print_esc_cstr_in(st, c"read".as_ptr());
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 1;
                    st.errors.help_line[0] = c"This \\read has unbalanced braces.".as_ptr();
                    st.scanner.align_state = 1_000_000;
                    st.input.cur_input.limit = 0;
                    scc_err(crate::errors::error_in(st))?;
                }
            }
        }

        let lim = st.input.last;
        st.input.cur_input.limit = lim;
        let end_char = unsafe {
            st.eqtb.data[(INT_BASE + INT_PAR_END_LINE_CHAR) as usize]
                .b32
                .s1
        };
        if !(0..=255).contains(&end_char) {
            st.input.cur_input.limit = lim - 1;
        } else {
            st.input.buffer[lim as usize] = end_char;
        }
        st.input.first = st.input.cur_input.limit + 1;
        st.input.cur_input.loc = st.input.cur_input.start;
        st.input.cur_input.state = NEW_LINE;

        if j == 1 {
            // §: direct token scan (no expansion)
            let mut loc = st.input.cur_input.loc;
            while loc <= st.input.cur_input.limit {
                let c = st.input.buffer[loc as usize];
                loc += 1;
                let tok = if c == b' ' as i32 {
                    SPACE_TOKEN
                } else {
                    c + OTHER_TOKEN
                };
                st.scanner.cur_chr = c;
                st.scanner.cur_tok = tok;
                let q2 = get_avail_scc(st)?;
                p.set_link(&mut st.mem, q2);
                q2.set_info(&mut st.mem, tok);
                p = q2;
            }
            // goto done → end_file_reading then check while condition
        } else {
            loop {
                get_token_in(st)?;
                if st.scanner.cur_tok == 0 {
                    break; // goto done
                }
                if st.scanner.align_state < 1_000_000 {
                    loop {
                        get_token_in(st)?;
                        if st.scanner.cur_tok == 0 {
                            break;
                        }
                    }
                    st.scanner.align_state = 1_000_000;
                    break; // goto done
                }
                let q2 = get_avail_scc(st)?;
                p.set_link(&mut st.mem, q2);
                q2.set_info(&mut st.mem, st.scanner.cur_tok);
                p = q2;
            }
        }
        // done:
        crate::input::end_file_reading_in(st).map_err(SccAbort::Input)?;

        if st.scanner.align_state == 1_000_000 {
            break;
        }
    }

    st.scanner.cur_val = new_def.0;
    st.scanner.scanner_status = NORMAL_SCANNER;
    st.scanner.align_state = s;
    Ok(())
}

// ---------------------------------------------------------------------------
// extra_right_brace
// ---------------------------------------------------------------------------

// xetex-constants.h:166,167,168

// ---------------------------------------------------------------------------
// scan_font_ident
// ---------------------------------------------------------------------------

const SPACE_SHRINK_CODE: i32 = 4;

// ---------------------------------------------------------------------------
// scan_something_internal
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// scan_int
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// eTeX_enabled
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:8811 (eTeX_enabled)
/// `&mut EngineState` core of [`eTeX_enabled`]: the recoverable "disabled
/// feature" diagnostic prints in order then defers a non-`Continue` outcome via
/// [`crate::main_control::MainControlAbort`].
#[allow(non_snake_case)]
pub(crate) fn eTeX_enabled_in(
    st: &mut crate::state::EngineState,
    b: bool,
    j: u16,
    k: i32,
) -> Result<bool, crate::main_control::MainControlAbort> {
    if !b {
        crate::output::error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
        crate::output::print_cmd_chr_in(st, j, k);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 1;
        st.errors.help_line[0] = c"Sorry, this optional e-TeX feature has been disabled.".as_ptr();
        crate::main_control::mc_propagate_error(crate::errors::error_in(st))?;
    }
    Ok(b)
}

// ---------------------------------------------------------------------------
// effective_char / effective_char_info
// ---------------------------------------------------------------------------

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
/// Cold-abort sentinel for [`tokens_to_string_in`].
pub(crate) enum TokensToStringAbort {
    /// Called with `selector` already `new_string` (a programmer error).
    SelectorNewString,
    /// The string pool's `str_ptr` is exhausted.
    StringOverflow,
}

/// Renders token list `p` into the string pool via `show_token_list_in`; the
/// `new_string`-reentrancy guard and the string-pool overflow ride
/// [`TokensToStringAbort`].
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

const FROZEN_PROTECTION: i32 = FROZEN_CONTROL_SEQUENCE; // xetex-constants.h:55

// translated from xetex-xetex0.c:10407 (get_r_token)
pub(crate) fn get_r_token_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    loop {
        loop {
            get_token_in(st)?;
            if st.scanner.cur_tok != SPACE_TOKEN {
                break;
            }
        }
        if st.scanner.cur_cs == 0
            || st.scanner.cur_cs > st.eqtb.eqtb_top
            || (st.scanner.cur_cs > FROZEN_CONTROL_SEQUENCE && st.scanner.cur_cs <= EQTB_SIZE)
        {
            error_here_with_diagnostic_in(st, c"Missing control sequence inserted".as_ptr());
            capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 5;
            st.errors.help_line[4] =
                c"Please don't say `\\def cs{...}', say `\\def\\cs{...}'.".as_ptr();
            st.errors.help_line[3] =
                c"I've inserted an inaccessible control sequence so that your".as_ptr();
            st.errors.help_line[2] =
                c"definition will be completed without mixing me up too badly.".as_ptr();
            st.errors.help_line[1] =
                c"You can recover graciously from this error, if you're".as_ptr();
            st.errors.help_line[0] = c"careful; see exercise 27.2 in The TeXbook.".as_ptr();
            if st.scanner.cur_cs == 0 {
                crate::input::back_input_in(st)?;
            }
            st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_PROTECTION;
            scc_err(crate::input::ins_error_in(st)?)?;
            // goto restart
        } else {
            break;
        }
    }
    Ok(())
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

/// Scans one math field; the `{`...`}` sub-formula arm pushes a
/// `GroupCode::Math` and returns.
pub(crate) fn scan_math_in(
    st: &mut crate::state::EngineState,
    p: i32,
) -> Result<(), crate::main_control::MainControlAbort> {
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
    const INT_PAR_CUR_FAM: i32 = 44; // xetex_format.h:79
    const NUMBER_MATH_FAMILIES_VAL: i32 = 256; // xetex-constants.h:31

    let c: i32;
    'main: loop {
        loop {
            get_x_token_in(st)?;
            if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
                break;
            }
        }
        c = loop {
            break match st.scanner.cur_cmd {
                LETTER | OTHER_CHAR_CMD | CHAR_GIVEN_CMD => {
                    let mc = st.eqtb.b32s1(MATH_CODE_BASE + st.scanner.cur_chr);
                    if mc & 0x1FFFFF == ACTIVE_MATH_CHAR {
                        st.scanner.cur_cs = st.scanner.cur_chr + 1;
                        st.scanner.cur_cmd = st.eqtb.b16s1(st.scanner.cur_cs) as u8;
                        st.scanner.cur_chr = st.eqtb.b32s1(st.scanner.cur_cs);
                        x_token_in(st)?;
                        crate::input::back_input_in(st)?;
                        continue 'main;
                    }
                    mc
                }
                CHAR_NUM_CMD => {
                    scan_char_num_in(st)?;
                    st.scanner.cur_chr = st.scanner.cur_val;
                    st.scanner.cur_cmd = CHAR_GIVEN_CMD;
                    continue; // goto reswitch
                }
                MATH_CHAR_NUM => {
                    if st.scanner.cur_chr == 2 {
                        scan_math_class_int_in(st)?;
                        let mut cv = set_class_fn(st.scanner.cur_val);
                        scan_math_fam_int_in(st)?;
                        cv += set_family_fn(st.scanner.cur_val);
                        scan_usv_num_in(st)?;
                        cv + st.scanner.cur_val
                    } else if st.scanner.cur_chr == 1 {
                        scan_xetex_math_char_int_in(st)?;
                        st.scanner.cur_val
                    } else {
                        scan_fifteen_bit_int_in(st)?;
                        let v = st.scanner.cur_val;
                        set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256)
                    }
                }
                MATH_GIVEN_CMD => {
                    let v = st.scanner.cur_chr;
                    set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256)
                }
                XETEX_MATH_GIVEN => st.scanner.cur_chr,
                DELIM_NUM => {
                    if st.scanner.cur_chr == 1 {
                        scan_math_class_int_in(st)?;
                        let mut cv = set_class_fn(st.scanner.cur_val);
                        scan_math_fam_int_in(st)?;
                        cv += set_family_fn(st.scanner.cur_val);
                        scan_usv_num_in(st)?;
                        cv + st.scanner.cur_val
                    } else {
                        scan_delimiter_int_in(st)?;
                        let v = st.scanner.cur_val / 4096;
                        set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256)
                    }
                }
                _ => {
                    crate::input::back_input_in(st)?;
                    scan_left_brace_in(st)?;
                    st.save_stack.set_b32s1(st.save_stack.save_ptr, p);
                    st.save_stack.save_ptr += 1;
                    crate::main_control::push_math_in(st, GroupCode::Math)?;
                    return Ok(());
                }
            };
        };
        break;
    }
    st.mem.set_b32s1(p, MATH_CHAR_NODE);
    st.mem.set_b16s0(p, (c % 65536) as u16);
    let cur_fam_val = st.eqtb.b32s1(INT_BASE + INT_PAR_CUR_FAM);
    let fam: u16 = if math_class_fn(c) == 7 && (0..NUMBER_MATH_FAMILIES_VAL).contains(&cur_fam_val)
    {
        cur_fam_val as u16
    } else {
        math_fam_fn(c) as u16
    };
    st.mem
        .set_b16s1(p, fam + (math_char_fn(c) / 65536) as u16 * 256);
    Ok(())
}

/// Appends a math-char noad (or expands the active-math-char).
pub(crate) fn set_math_char_in(
    st: &mut crate::state::EngineState,
    c: i32,
) -> Result<(), crate::main_control::MainControlAbort> {
    const ACTIVE_MATH_CHAR_SM: i32 = 0x1FFFFF;
    const MATH_CHAR_NODE_SM: i32 = 1; // xetex-constants.h:269
    const INT_PAR_CUR_FAM_SM: i32 = 44;
    const NUMBER_MATH_FAM_SM: i32 = 256;

    if math_char_fn(c) == ACTIVE_MATH_CHAR_SM {
        st.scanner.cur_cs = st.scanner.cur_chr + 1;
        st.scanner.cur_cmd = st.eqtb.b16s1(st.scanner.cur_cs) as u8;
        st.scanner.cur_chr = st.eqtb.b32s1(st.scanner.cur_cs);
        x_token_in(st)?;
        crate::input::back_input_in(st)?;
    } else {
        // `alloc_noad` hands back the typed handle directly; this site does not
        // guard main-memory overflow (as before), so a failure panics here.
        let nd = crate::mem::alloc_noad(&mut st.mem).expect("new noad");
        nd.set_nucleus_type(&mut st.mem, MATH_CHAR_NODE_SM);
        let ch = math_char_fn(c);
        nd.set_nucleus_char(&mut st.mem, (ch % 65536) as u16);
        nd.set_nucleus_fam(&mut st.mem, math_fam_fn(c) as u16);
        if math_class_fn(c) == 7 {
            let cf = st.eqtb.b32s1(INT_BASE + INT_PAR_CUR_FAM_SM);
            if (0..NUMBER_MATH_FAM_SM).contains(&cf) {
                nd.set_nucleus_fam(&mut st.mem, cf as u16);
            }
            nd.set_noad_type(&mut st.mem, NodeType::OrdNoad);
        } else {
            nd.set_noad_type(
                &mut st.mem,
                NodeType::from_raw(NodeType::OrdNoad.as_raw() + math_class_fn(c) as u16)
                    .expect("valid noad type"),
            );
        }
        let fam = nd.nucleus_fam(&st.mem) + (ch / 65536) as u16 * 256;
        nd.set_nucleus_fam(&mut st.mem, fam);
        st.mem.set_b32s1(st.input.cur_list.tail, nd.idx().0);
        st.input.cur_list.tail.0 = nd.idx().0;
    }
    Ok(())
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

/// Cold abort reasons for the node-recycling functions (`copy_node_list`,
/// `prune_page_top`).
pub(crate) enum NodeRecycleAbort {
    /// `get_node` / `new_skip_param` exhausted the arena: `overflow`.
    Overflow,
    /// `get_avail` exhausted the arena: `runaway()` then `overflow`.
    OverflowRunaway,
    /// Internal-consistency failure: `confusion(msg)`.
    Confusion(&'static std::ffi::CStr),
    /// A native-font measurement cold abort carried through a node-building path.
    NativeFont(crate::ext::NativeFontAbort),
}

impl From<crate::ext::NativeFontAbort> for NodeRecycleAbort {
    fn from(e: crate::ext::NativeFontAbort) -> Self {
        NodeRecycleAbort::NativeFont(e)
    }
}

impl NodeRecycleAbort {
    /// Map to the unified [`EngineAbort`] terminal.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        const MEM_TOP_NR: i32 = 4_999_999;
        match self {
            NodeRecycleAbort::Overflow => crate::errors::EngineAbort::Overflow {
                what: c"main memory size".as_ptr(),
                n: MEM_TOP_NR + 1,
            },
            NodeRecycleAbort::OverflowRunaway => crate::mem::main_memory_overflow_abort(),
            NodeRecycleAbort::Confusion(msg) => crate::errors::EngineAbort::Confusion(msg.as_ptr()),
            NodeRecycleAbort::NativeFont(e) => e.into_engine_abort(),
        }
    }
}

/// `get_node` as a raw index, returning [`NodeRecycleAbort::Overflow`] from
/// the enclosing function on arena exhaustion.
macro_rules! cnl_get_node {
    ($mem:expr, $sz:expr) => {
        crate::mem::get_node($mem, $sz)
            .ok_or(NodeRecycleAbort::Overflow)?
            .0
    };
}

// Re-export cnl_get_node for the font-loading group in fonts.rs and
// copy_node_list in packaging.rs.
pub(crate) use cnl_get_node;

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

/// Handles the cold runaway / forbidden-cs / incomplete-`\if` recovery,
/// returning the recovering `error`/`ins_error`
/// [`ErrorOutcome`](crate::errors::ErrorOutcome); arena exhaustion and
/// `begin_token_list` overflow ride [`InputAbort`](crate::input::InputAbort).
/// Normal (`NORMAL`) status returns `ErrorOutcome::Continue`.
pub(crate) fn check_outer_validity_in(
    st: &mut crate::state::EngineState,
) -> Result<crate::errors::ErrorOutcome, crate::input::InputAbort> {
    use crate::errors::ErrorOutcome;
    use crate::input::InputAbort;

    if st.scanner.scanner_status == NORMAL_COV {
        return Ok(ErrorOutcome::Continue);
    }
    st.errors.deletions_allowed = false;

    // If cur_cs is a control sequence, push it back as a backed-up token.
    if st.scanner.cur_cs != 0 {
        let ci = st.input.cur_input;
        if ci.state == TOKEN_LIST_COV || ci.name < 1 || ci.name > 17 {
            let p = crate::mem::get_avail(&mut st.mem).ok_or(InputAbort::MainMemory)?;
            p.set_info(&mut st.mem, CS_TOKEN_FLAG + st.scanner.cur_cs);
            crate::input::begin_token_list_in(st, p.0, 3 /* BACKED_UP */)?;
        }
        st.scanner.cur_cmd = SPACER;
        st.scanner.cur_chr = ' ' as i32;
    }

    let oc = if st.scanner.scanner_status > SKIPPING_COV {
        // scanner_status > SKIPPING: runaway — we hit EOF or OUTER inside a definition/macro/preamble.
        crate::errors::runaway_in(st);
        if st.scanner.cur_cs == 0 {
            crate::output::error_here_with_diagnostic_in(st, c"File ended".as_ptr());
        } else {
            st.scanner.cur_cs = 0;
            crate::output::error_here_with_diagnostic_in(
                st,
                c"Forbidden control sequence found".as_ptr(),
            );
        }

        let p = crate::mem::get_avail(&mut st.mem).ok_or(InputAbort::MainMemory)?;
        match st.scanner.scanner_status {
            DEFINING_COV => {
                crate::output::print_cstr_in(st, c" while scanning definition".as_ptr());
                p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN_COV + b'}' as i32);
            }
            MATCHING_COV => {
                crate::output::print_cstr_in(st, c" while scanning use".as_ptr());
                let par_token = st.input.par_token;
                p.set_info(&mut st.mem, par_token);
                st.scanner.long_state = OUTER_CALL_COV;
            }
            ALIGNING_COV => {
                crate::output::print_cstr_in(st, c" while scanning preamble".as_ptr());
                p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN_COV + b'}' as i32);
                let q = p;
                let p2 = crate::mem::get_avail(&mut st.mem).ok_or(InputAbort::MainMemory)?;
                p2.set_link(&mut st.mem, q);
                p2.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_CR_COV);
                st.scanner.align_state = -1_000_000;
                crate::input::begin_token_list_in(st, p2.0, 5 /* INSERTED */)?;
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
                p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN_COV + b'}' as i32);
            }
            _ => {}
        }

        crate::input::begin_token_list_in(st, p.0, 5 /* INSERTED */)?;
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
        if st.scanner.cur_cs != 0 {
            st.scanner.cur_cs = 0;
        } else {
            st.errors.help_line[2] =
                c"The file ended while I was skipping conditional text.".as_ptr();
        }
        st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_FI_COV;
        crate::input::ins_error_in(st)?
    };
    st.errors.deletions_allowed = true;
    Ok(oc)
}

// ---------------------------------------------------------------------------
// expand
// ---------------------------------------------------------------------------

/// Deferred cold outcomes of [`get_next_in`].
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
/// sentinel.
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

/// The central tokenizer. Cold abort / recoverable-error / no-legal-`\end`
/// tails ride [`GetNextOutcome`].
pub(crate) fn get_next_in(st: &mut crate::state::EngineState) -> GetNextOutcome {
    st.scanner.logical_clock = st.scanner.logical_clock.wrapping_add(1);

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
        st.scanner.cur_cs = 0;

        if st.input.cur_input.state != TOKEN_LIST {
            'texswitch: loop {
                if st.input.cur_input.loc <= st.input.cur_input.limit {
                    // Read next character from the buffer
                    st.scanner.cur_chr = st.input.buffer[(st.input.cur_input.loc) as usize];
                    st.input.cur_input.loc = st.input.cur_input.loc + 1;

                    // Handle UTF-16 surrogate pair
                    if st.scanner.cur_chr >= 0xD800
                        && st.scanner.cur_chr < 0xDC00
                        && st.input.cur_input.loc <= st.input.cur_input.limit
                        && st.input.buffer[(st.input.cur_input.loc) as usize] >= 0xDC00
                        && st.input.buffer[(st.input.cur_input.loc) as usize] < 0xE000
                    {
                        let lower = st.input.buffer[(st.input.cur_input.loc) as usize] - 0xDC00;
                        st.input.cur_input.loc = st.input.cur_input.loc + 1;
                        st.scanner.cur_chr = 65536 + (st.scanner.cur_chr - 0xD800) * 1024 + lower;
                    }

                    'reswitch: loop {
                        let cat = st.eqtb.b32s1(CAT_CODE_BASE + st.scanner.cur_chr) as u8;
                        st.scanner.cur_cmd = cat;
                        let sc = st.input.cur_input.state as i32 + cat as i32;

                        match sc {
                            // ANY_STATE_PLUS(IGNORE) + SKIP_BLANKS+SPACER + NEW_LINE+SPACER
                            // → goto texswitch
                            10 | 26 | 42 | 27 | 43 => {
                                continue 'texswitch;
                            }

                            // ANY_STATE_PLUS(ESCAPE): escape sequence
                            1 | 17 | 33 => {
                                if st.input.cur_input.loc > st.input.cur_input.limit {
                                    st.scanner.cur_cs = NULL_CS_GN;
                                } else {
                                    'start_cs: loop {
                                        let cs_start = st.input.cur_input.loc;
                                        let first_chr = st.input.buffer[(cs_start) as usize];
                                        let first_cat =
                                            st.eqtb.b32s1(CAT_CODE_BASE + first_chr) as u8;
                                        let mut k = cs_start + 1;
                                        st.input.cur_input.state =
                                            if first_cat == LETTER || first_cat == SPACER {
                                                SKIP_BLANKS
                                            } else {
                                                MID_LINE
                                            };

                                        if first_cat == LETTER && k <= st.input.cur_input.limit {
                                            // Multi-letter control word
                                            let mut chr = st.input.buffer[(k) as usize];
                                            let mut cat2 = st.eqtb.b32s1(CAT_CODE_BASE + chr) as u8;
                                            k += 1;
                                            while cat2 == LETTER && k <= st.input.cur_input.limit {
                                                chr = st.input.buffer[(k) as usize];
                                                cat2 = st.eqtb.b32s1(CAT_CODE_BASE + chr) as u8;
                                                k += 1;
                                            }

                                            // Sup-mark (^^^) handling in multi-letter path
                                            if cat2 == SUP_MARK
                                                && st.input.buffer[(k) as usize] == chr
                                                && k < st.input.cur_input.limit
                                            {
                                                let mut sup_count: i32 = 2;
                                                while sup_count < 6
                                                    && k + 2 * sup_count - 2
                                                        <= st.input.cur_input.limit
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
                                                            st.input.cur_input.limit =
                                                                st.input.cur_input.limit - 2;
                                                            let mut kk = k;
                                                            while kk <= st.input.cur_input.limit {
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
                                                        st.input.cur_input.limit =
                                                            st.input.cur_input.limit - shift;
                                                        let mut kk = k;
                                                        while kk <= st.input.cur_input.limit {
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
                                            if k > st.input.cur_input.loc + 1 {
                                                st.scanner.cur_cs = match crate::hash::id_lookup(
                                                    &mut st.eqtb,
                                                    &mut st.strings,
                                                    &st.input.buffer,
                                                    st.input.cur_input.loc,
                                                    k - st.input.cur_input.loc,
                                                ) {
                                                    Ok(p) => p,
                                                    Err(e) => {
                                                        return GetNextOutcome::Abort(
                                                            hash_err_abort(e),
                                                        )
                                                    }
                                                };
                                                st.input.cur_input.loc = k;
                                                break 'start_cs; // goto found
                                            }
                                            // Single letter: fall through to single-char handling
                                        } else {
                                            // Single-char (non-LETTER first char) or end of line
                                            if first_cat == SUP_MARK
                                                && st.input.buffer[(k) as usize] == first_chr
                                                && k < st.input.cur_input.limit
                                            {
                                                let mut sup_count: i32 = 2;
                                                while sup_count < 6
                                                    && k + 2 * sup_count - 2
                                                        <= st.input.cur_input.limit
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
                                                            st.input.cur_input.limit =
                                                                st.input.cur_input.limit - 2;
                                                            let mut kk = k;
                                                            while kk <= st.input.cur_input.limit {
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
                                                        st.input.cur_input.limit =
                                                            st.input.cur_input.limit - shift;
                                                        let mut kk = k;
                                                        while kk <= st.input.cur_input.limit {
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
                                        if st.input.buffer[(st.input.cur_input.loc) as usize]
                                            > 65535
                                        {
                                            st.scanner.cur_cs = match crate::hash::id_lookup(
                                                &mut st.eqtb,
                                                &mut st.strings,
                                                &st.input.buffer,
                                                st.input.cur_input.loc,
                                                1,
                                            ) {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    return GetNextOutcome::Abort(hash_err_abort(e))
                                                }
                                            };
                                            st.input.cur_input.loc = st.input.cur_input.loc + 1;
                                            break 'start_cs; // goto found
                                        }
                                        st.scanner.cur_cs = SINGLE_BASE
                                            + st.input.buffer[(st.input.cur_input.loc) as usize];
                                        st.input.cur_input.loc = st.input.cur_input.loc + 1;
                                        break 'start_cs;
                                    }
                                }
                                // found:
                                st.scanner.cur_cmd = st.eqtb.b16s1(st.scanner.cur_cs) as u8;
                                st.scanner.cur_chr = st.eqtb.b32s1(st.scanner.cur_cs);
                                if st.scanner.cur_cmd >= OUTER_CALL_GN {
                                    match check_outer_validity_in(st) {
                                        Ok(crate::errors::ErrorOutcome::Continue) => {}
                                        Ok(oc) => return GetNextOutcome::Error(oc),
                                        Err(a) => return GetNextOutcome::Abort(a),
                                    }
                                }
                            }

                            // ANY_STATE_PLUS(ACTIVE_CHAR): active character
                            14 | 30 | 46 => {
                                st.scanner.cur_cs = st.scanner.cur_chr + 1;
                                st.scanner.cur_cmd = st.eqtb.b16s1(st.scanner.cur_cs) as u8;
                                st.scanner.cur_chr = st.eqtb.b32s1(st.scanner.cur_cs);
                                st.input.cur_input.state = MID_LINE;
                                if st.scanner.cur_cmd >= OUTER_CALL_GN {
                                    match check_outer_validity_in(st) {
                                        Ok(crate::errors::ErrorOutcome::Continue) => {}
                                        Ok(oc) => return GetNextOutcome::Error(oc),
                                        Err(a) => return GetNextOutcome::Abort(a),
                                    }
                                }
                            }

                            // ANY_STATE_PLUS(SUP_MARK): caret (^^) expansion
                            8 | 24 | 40 => {
                                if st.scanner.cur_chr
                                    == st.input.buffer[(st.input.cur_input.loc) as usize]
                                    && st.input.cur_input.loc < st.input.cur_input.limit
                                {
                                    let mut sup_count: i32 = 2;
                                    while sup_count < 6
                                        && st.input.cur_input.loc + 2 * sup_count - 2
                                            <= st.input.cur_input.limit
                                        && st.scanner.cur_chr
                                            == st.input.buffer
                                                [(st.input.cur_input.loc + sup_count - 1) as usize]
                                    {
                                        sup_count += 1;
                                    }
                                    let sup_count_save = sup_count;
                                    let mut hex_ok = true;
                                    for d in 1..=sup_count_save {
                                        if !is_lc_hex(
                                            st.input.buffer[(st.input.cur_input.loc + sup_count - 2
                                                + d)
                                                as usize],
                                        ) {
                                            let c = st.input.buffer
                                                [(st.input.cur_input.loc + 1) as usize];
                                            if c < 128 {
                                                st.input.cur_input.loc = st.input.cur_input.loc + 2;
                                                st.scanner.cur_chr =
                                                    if c < 64 { c + 64 } else { c - 64 };
                                                continue 'reswitch;
                                            }
                                            hex_ok = false;
                                            break;
                                        }
                                    }
                                    if hex_ok {
                                        let mut new_chr: i32 = 0;
                                        for d in 1..=sup_count {
                                            let c = st.input.buffer[(st.input.cur_input.loc
                                                + sup_count
                                                - 2
                                                + d)
                                                as usize];
                                            new_chr = 16 * new_chr
                                                + if c <= b'9' as i32 {
                                                    c - b'0' as i32
                                                } else {
                                                    c - b'a' as i32 + 10
                                                };
                                        }
                                        if new_chr > BIGGEST_USV_GN {
                                            st.scanner.cur_chr =
                                                st.input.buffer[(st.input.cur_input.loc) as usize];
                                            // fall to not_exp
                                        } else {
                                            st.scanner.cur_chr = new_chr;
                                            st.input.cur_input.loc =
                                                st.input.cur_input.loc + 2 * sup_count - 1;
                                            continue 'reswitch;
                                        }
                                    }
                                }
                                // not_exp:
                                st.input.cur_input.state = MID_LINE;
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
                                st.input.cur_input.state = SKIP_BLANKS;
                                st.scanner.cur_chr = b' ' as i32;
                            }

                            // MID_LINE + CAR_RET
                            6 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                st.scanner.cur_cmd = SPACER;
                                st.scanner.cur_chr = b' ' as i32;
                            }

                            // ANY_STATE_PLUS(COMMENT) + SKIP_BLANKS+CAR_RET → skip rest of line, goto texswitch
                            15 | 31 | 47 | 22 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                continue 'texswitch;
                            }

                            // NEW_LINE + CAR_RET: insert \par
                            38 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                st.scanner.cur_cs = st.input.par_loc;
                                st.scanner.cur_cmd = st.eqtb.b16s1(st.scanner.cur_cs) as u8;
                                st.scanner.cur_chr = st.eqtb.b32s1(st.scanner.cur_cs);
                                if st.scanner.cur_cmd >= OUTER_CALL_GN {
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
                                st.input.cur_input.state = MID_LINE;
                                st.scanner.align_state = st.scanner.align_state + 1;
                            }

                            // MID_LINE + RIGHT_BRACE
                            3 => {
                                st.scanner.align_state = st.scanner.align_state - 1;
                            }

                            // SKIP_BLANKS + RIGHT_BRACE | NEW_LINE + RIGHT_BRACE
                            19 | 35 => {
                                st.input.cur_input.state = MID_LINE;
                                st.scanner.align_state = st.scanner.align_state - 1;
                            }

                            // ADD_DELIMS_TO(SKIP_BLANKS): SKIP_BLANKS + MATH_SHIFT/TAB_MARK/MAC_PARAM/SUB_MARK/LETTER/OTHER_CHAR
                            20 | 21 | 23 | 25 | 28 | 29 => {
                                st.input.cur_input.state = MID_LINE;
                            }

                            // ADD_DELIMS_TO(NEW_LINE): NEW_LINE + MATH_SHIFT/TAB_MARK/MAC_PARAM/SUB_MARK/LETTER/OTHER_CHAR
                            36 | 37 | 39 | 41 | 44 | 45 => {
                                st.input.cur_input.state = MID_LINE;
                            }

                            _ => {} // default: do nothing
                        }
                        break 'reswitch;
                    }
                    break 'texswitch;
                } else {
                    // End of line / end of file
                    st.input.cur_input.state = NEW_LINE;

                    if st.input.cur_input.name > 17 {
                        st.mem.line = st.mem.line + 1;
                        st.input.first = st.input.cur_input.start;

                        if !st.input.force_eof {
                            if st.input.cur_input.name <= 19 {
                                if try_in!(crate::input::pseudo_input_in(st)) {
                                    st.input.cur_input.limit = st.input.last;
                                } else if st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_EOF) != TEX_NULL
                                    && !st.input.eof_seen
                                        [(st.input.cur_input.index as i32) as usize]
                                {
                                    st.input.cur_input.limit = st.input.first - 1;
                                    st.input.eof_seen[(st.input.cur_input.index as i32) as usize] =
                                        true;
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
                                let __f = st.input.input_file_ptr(st.input.cur_input.index as i32);
                                try_in!(unsafe { crate::io::input_line_in(st, __f) })
                            } != 0
                            {
                                st.input.cur_input.limit = st.input.last;
                            } else if st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_EOF) != TEX_NULL
                                && !st.input.eof_seen[(st.input.cur_input.index as i32) as usize]
                            {
                                st.input.cur_input.limit = st.input.first - 1;
                                st.input.eof_seen[(st.input.cur_input.index as i32) as usize] =
                                    true;
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
                            if st.input.cur_input.name >= 19 {
                                crate::output::print_char_in(st, b')' as i32);
                                st.input.open_parens = st.input.open_parens - 1;
                                unsafe { ttstub_output_flush(st.shipout.rust_stdout) };
                            }
                            st.input.force_eof = false;
                            if let Err(a) = crate::input::end_file_reading_in(st) {
                                return GetNextOutcome::Abort(a);
                            }
                            match check_outer_validity_in(st) {
                                Ok(crate::errors::ErrorOutcome::Continue) => {}
                                Ok(oc) => return GetNextOutcome::Error(oc),
                                Err(a) => return GetNextOutcome::Abort(a),
                            }
                            continue 'restart;
                        }

                        let end_char = st.eqtb.b32s1(INT_BASE + INT_PAR_END_LINE_CHAR_GN);
                        if !(0..=255).contains(&end_char) {
                            st.input.cur_input.limit = st.input.cur_input.limit - 1;
                        } else {
                            st.input.buffer[(st.input.cur_input.limit) as usize] = end_char;
                        }
                        st.input.first = st.input.cur_input.limit + 1;
                        st.input.cur_input.loc = st.input.cur_input.start;
                    } else {
                        if st.input.cur_input.name != 0 {
                            st.scanner.cur_cmd = 0;
                            st.scanner.cur_chr = 0;
                            return GetNextOutcome::Done;
                        }
                        if st.input.input_ptr > 0 {
                            if let Err(a) = crate::input::end_file_reading_in(st) {
                                return GetNextOutcome::Abort(a);
                            }
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
        } else if st.input.cur_input.loc != TEX_NULL {
            // Reading from a token list
            let t = st.mem.b32s0(st.input.cur_input.loc);
            st.input.cur_input.loc = st.mem.b32s1(st.input.cur_input.loc); // LLIST_link

            if t >= CS_TOKEN_FLAG {
                st.scanner.cur_cs = t - CS_TOKEN_FLAG;
                st.scanner.cur_cmd = st.eqtb.b16s1(st.scanner.cur_cs) as u8;
                st.scanner.cur_chr = st.eqtb.b32s1(st.scanner.cur_cs);
                if st.scanner.cur_cmd >= OUTER_CALL_GN {
                    if st.scanner.cur_cmd == DONT_EXPAND_GN {
                        st.scanner.cur_cs = st.mem.b32s0(st.input.cur_input.loc) - CS_TOKEN_FLAG;
                        st.input.cur_input.loc = TEX_NULL;
                        st.scanner.cur_cmd = st.eqtb.b16s1(st.scanner.cur_cs) as u8;
                        st.scanner.cur_chr = st.eqtb.b32s1(st.scanner.cur_cs);
                        if st.scanner.cur_cmd > MAX_COMMAND_GN {
                            st.scanner.cur_cmd = RELAX_GN;
                            st.scanner.cur_chr = NO_EXPAND_FLAG_GN;
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
                st.scanner.cur_cmd = (t / MAX_CHAR_VAL_GN) as u8;
                st.scanner.cur_chr = t % MAX_CHAR_VAL_GN;
                match st.scanner.cur_cmd {
                    x if x == LEFT_BRACE_CAT => {
                        st.scanner.align_state = st.scanner.align_state + 1;
                    }
                    x if x == RIGHT_BRACE_CAT => {
                        st.scanner.align_state = st.scanner.align_state - 1;
                    }
                    x if x == OUT_PARAM_GN => {
                        {
                            let __p = st.input.param_stack
                                [(st.input.cur_input.limit + st.scanner.cur_chr - 1) as usize];
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
        if st.scanner.cur_cmd <= CAR_RET
            && st.scanner.cur_cmd >= TAB_MARK_CAT
            && st.scanner.align_state == 0
        {
            if st.scanner.scanner_status == ALIGNING_GN || st.scanner.cur_align.0 == TEX_NULL {
                return GetNextOutcome::Abort(crate::input::InputAbort::Fatal(
                    c"(interwoven alignment preambles are not allowed)".as_ptr(),
                ));
            }
            let saved_cmd = st.mem.b32s0(st.scanner.cur_align + 5);
            st.mem
                .set_b32s0(st.scanner.cur_align + 5, st.scanner.cur_chr);
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
            st.scanner.cur_cmd = saved_cmd as u8;
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

// ---------------------------------------------------------------------------
// main_control_main_loop
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// main_control
// ---------------------------------------------------------------------------

/// Deferred cold outcomes of the scanner / `expand` cores.
pub(crate) enum SccAbort {
    /// Cooperative resident-operation cancellation.
    Cancelled,
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
    /// Serious internal error closing an input file opened by a `conv_toks` file
    /// primitive (`\filesize`/`\filedump`/`\filemoddate`) or `count_pdf_file_pages`
    /// — the C `ttstub_input_close` wrapper `longjmp`'d here.
    InputClose,
    /// A native-font measurement cold abort.
    NativeFont(crate::ext::NativeFontAbort),
}

impl From<crate::ext::NativeFontAbort> for SccAbort {
    fn from(e: crate::ext::NativeFontAbort) -> Self {
        SccAbort::NativeFont(e)
    }
}

impl SccAbort {
    /// Map to the unified [`EngineAbort`] terminal.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            SccAbort::Cancelled => crate::errors::EngineAbort::Cancelled,
            SccAbort::Error(oc) => oc.into_engine_abort(),
            SccAbort::Input(a) => a.into_engine_abort(),
            SccAbort::NoLegalEnd { open_log } => {
                // Defer the (conditional) open_log_file + fatal_error to
                // into_history.
                crate::errors::EngineAbort::NoLegalEnd { open_log }
            }
            SccAbort::TokensToString(a) => match a {
                TokensToStringAbort::SelectorNewString => crate::errors::EngineAbort::PdfError {
                    t: c"tokens".as_ptr(),
                    p: c"tokens_to_string() called while selector = new_string".as_ptr(),
                },
                TokensToStringAbort::StringOverflow => crate::errors::EngineAbort::StringOverflow,
            },
            SccAbort::PdfError { t, p } => crate::errors::EngineAbort::PdfError { t, p },
            SccAbort::OpenLog(e) => e.into_engine_abort(),
            SccAbort::MakeTexString(e) => e.into_engine_abort(),
            SccAbort::InputClose => crate::errors::EngineAbort::TtAbort(
                c"serious internal error closing an input file".as_ptr(),
            ),
            SccAbort::NativeFont(e) => e.into_engine_abort(),
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
/// `SccAbort`: `Continue` keeps the scan going, anything else became terminal.
/// Use as `scc_err(o)?;`.
pub(crate) fn scc_err(o: crate::errors::ErrorOutcome) -> Result<(), SccAbort> {
    if matches!(o, crate::errors::ErrorOutcome::Continue) {
        Ok(())
    } else {
        Err(SccAbort::Error(o))
    }
}

/// `get_next`, with its [`GetNextOutcome`] cold tails lifted into [`SccAbort`].
/// Use as `get_next_scc(st)?;`.
/// Inlined: as an out-of-line symbol this pure trampoline pays a call/ret plus
/// a second multi-word outcome conversion on every token read.
#[inline]
pub(crate) fn get_next_scc(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    Result::from(get_next_in(st))
}

/// The deferred `overflow("main memory size", MEM_TOP + 1)` tail of the node
/// allocators (`get_node`/`alloc_spec` returning `None`), lifted into
/// [`SccAbort`].
pub(crate) fn main_mem_overflow_scc() -> SccAbort {
    SccAbort::Input(crate::input::InputAbort::Overflow {
        what: c"main memory size".as_ptr(),
        n: 4_999_999 + 1,
    })
}

/// `get_node` with its exhaustion `None` lifted into [`SccAbort`].
pub(crate) fn get_node_scc(
    st: &mut crate::state::EngineState,
    s: i32,
) -> Result<crate::units::MemPtr, SccAbort> {
    crate::mem::get_node(&mut st.mem, s).ok_or_else(main_mem_overflow_scc)
}

/// `get_avail` with its exhaustion `None` lifted into [`SccAbort`].
///
/// `#[inline(always)]`: `Result<MemPtr, SccAbort>` is too wide for a register
/// return, so an out-of-line call round-trips the `Ok` through a caller stack
/// slot (sret) on every single-word token allocation — where C's `get_avail`
/// returns its `int32_t` in a register (and the C original hand-expands
/// `fast_get_avail` directly inside the hot token-copy loops). Inlining folds
/// the `Ok`/`?` pair away on those paths.
#[inline(always)]
pub(crate) fn get_avail_scc(
    st: &mut crate::state::EngineState,
) -> Result<crate::units::MemPtr, SccAbort> {
    crate::mem::get_avail(&mut st.mem).ok_or(SccAbort::Input(crate::input::InputAbort::MainMemory))
}

/// [`crate::mem::alloc_spec`] with arena exhaustion lifted into [`SccAbort`].
pub(crate) fn new_spec_scc(
    st: &mut crate::state::EngineState,
    p: impl crate::units::MemAddr,
) -> Result<NodePtr<GlueSpecNode>, SccAbort> {
    crate::mem::alloc_spec(&mut st.mem, p).ok_or_else(main_mem_overflow_scc)
}

/// `str_toks_cat` with its [`StrToksAbort`] cold overflow tails lifted into
/// [`SccAbort`] (via [`crate::input::InputAbort`]).
pub(crate) fn str_toks_scc(
    st: &mut crate::state::EngineState,
    b: i32,
    cat: i16,
) -> Result<crate::units::MemPtr, SccAbort> {
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

// ---------------------------------------------------------------------------

/// Shared tail of the leaf `scan_int` range-check members: emit the "Bad …"
/// diagnostic for an out-of-range `cur_val`, then reset it to zero. `help1` is
/// the variable second help line (constant for most, `max_reg_help_line` for
/// `scan_register_num`).
fn leaf_int_range_error(
    st: &mut crate::state::EngineState,
    err_msg: *const libc::c_char,
    help1: *const libc::c_char,
) -> Result<(), SccAbort> {
    let errmsg = error_here_with_diagnostic_in(st, err_msg);
    unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val)) };
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 2;
    st.errors.help_line[1] = help1;
    st.errors.help_line[0] = c"I changed this one to zero.".as_ptr();
    scc_err(int_error_in(st, st.scanner.cur_val))?;
    st.scanner.cur_val = 0;
    Ok(())
}

pub(crate) fn x_token_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    while st.scanner.cur_cmd > MAX_COMMAND {
        expand_in(st)?;
        get_next_scc(st)?;
    }
    if st.scanner.cur_cs == 0 {
        st.scanner.cur_tok = st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
    } else {
        st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
    }
    Ok(())
}
pub(crate) fn get_x_token_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    loop {
        if st.cancellation.at_expansion_token() {
            return Err(SccAbort::Cancelled);
        }
        get_next_scc(st)?;
        if st.scanner.cur_cmd <= MAX_COMMAND {
            break;
        }
        if st.scanner.cur_cmd >= CALL {
            if st.scanner.cur_cmd < END_TEMPLATE {
                macro_call_in(st)?;
            } else {
                st.scanner.cur_cs = FROZEN_ENDV;
                st.scanner.cur_cmd = ENDV;
                break;
            }
        } else {
            expand_in(st)?;
        }
    }
    if st.scanner.cur_cs == 0 {
        st.scanner.cur_tok = st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
    } else {
        st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
    }
    Ok(())
}
pub(crate) fn the_toks_in(
    st: &mut crate::state::EngineState,
) -> Result<crate::units::MemPtr, SccAbort> {
    use crate::mem::TEX_NULL;

    if st.scanner.cur_chr & 1 != 0 {
        let c = st.scanner.cur_chr;
        scan_general_text_in(st)?;
        if c == 1 {
            return Ok(crate::units::MemPtr::from_raw(st.scanner.cur_val));
        } else {
            let old_setting = st.shipout.selector;
            st.shipout.selector = SELECTOR_NEW_STRING;
            let b = st.strings.pool_ptr;
            let p = get_avail_scc(st)?;
            let th = crate::units::MemPtr::from_raw(TEMP_HEAD).link(&st.mem);
            p.set_link(&mut st.mem, th);
            crate::output::token_show_in(st, p);
            crate::mem::flush_list(&mut st.mem, p);
            st.shipout.selector = old_setting;
            return str_toks_scc(st, b, 0);
        }
    }
    get_x_token_in(st)?;
    scan_something_internal_in(st, TOK_VAL, false)?;
    if st.scanner.cur_val_level >= IDENT_VAL as u8 {
        let mut p = crate::units::MemPtr::from_raw(TEMP_HEAD);
        p.set_link(&mut st.mem, crate::units::MemPtr::NULL);
        if st.scanner.cur_val_level == IDENT_VAL as u8 {
            let q = get_avail_scc(st)?;
            p.set_link(&mut st.mem, q);
            q.set_info(&mut st.mem, CS_TOKEN_FLAG + st.scanner.cur_val);
            p = q;
        } else if st.scanner.cur_val != TEX_NULL {
            let mut r = crate::units::MemPtr::from_raw(st.scanner.cur_val).link(&st.mem);
            while !r.is_null() {
                let q = get_avail_scc(st)?;
                p.set_link(&mut st.mem, q);
                let v = r.info(&st.mem);
                q.set_info(&mut st.mem, v);
                p = q;
                r = r.link(&st.mem);
            }
        }
        Ok(p)
    } else {
        let old_setting = st.shipout.selector;
        st.shipout.selector = SELECTOR_NEW_STRING;
        let b = st.strings.pool_ptr;
        match st.scanner.cur_val_level {
            0 => crate::output::print_int_in(st, st.scanner.cur_val),
            1 => {
                crate::output::print_scaled_in(st, st.scanner.cur_val);
                crate::output::print_cstr_in(st, c"pt".as_ptr());
            }
            2 => {
                crate::output::print_spec_in(st, st.scanner.cur_val, c"pt".as_ptr());
                crate::mem::delete_glue_ref(&mut st.mem, st.scanner.cur_val);
            }
            3 => {
                crate::output::print_spec_in(st, st.scanner.cur_val, c"mu".as_ptr());
                crate::mem::delete_glue_ref(&mut st.mem, st.scanner.cur_val);
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
    st.scanner.warning_index = st.scanner.cur_cs;
    let new_def = get_avail_scc(st)?;
    st.scanner.def_ref = new_def;
    new_def.set_info(&mut st.mem, TEX_NULL);
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
                    if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
                        break 'param_scan false; // goto done1
                    }
                    if st.scanner.cur_cmd == MAC_PARAM {
                        // §495
                        let s = MATCH_TOKEN + st.scanner.cur_chr;
                        get_token_in(st)?;
                        if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
                            hash_brace = st.scanner.cur_tok;
                            let q = get_avail_scc(st)?;
                            p.set_link(&mut st.mem, q);
                            q.set_info(&mut st.mem, st.scanner.cur_tok);
                            p = q;
                            let q2 = get_avail_scc(st)?;
                            p.set_link(&mut st.mem, q2);
                            q2.set_info(&mut st.mem, END_MATCH_TOKEN);
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
                            if st.scanner.cur_tok != t {
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
                            st.scanner.cur_tok = s;
                        }
                    }
                    // append cur_tok to list
                    let q = get_avail_scc(st)?;
                    p.set_link(&mut st.mem, q);
                    q.set_info(&mut st.mem, st.scanner.cur_tok);
                    p = q;
                }
            };
            // done1: append END_MATCH_TOKEN
            if !reached_done {
                let q = get_avail_scc(st)?;
                p.set_link(&mut st.mem, q);
                q.set_info(&mut st.mem, END_MATCH_TOKEN);
                p = q;
                // §494: check for missing {
                if st.scanner.cur_cmd == RIGHT_BRACE {
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
                    if st.scanner.cur_cmd >= CALL
                        && st.mem.b32s0(st.mem.b32s1(st.scanner.cur_chr)) == PROTECTED_TOKEN
                    {
                        st.scanner.cur_cmd = RELAX;
                        st.scanner.cur_chr = NO_EXPAND_FLAG;
                    }
                    if st.scanner.cur_cmd <= MAX_COMMAND {
                        break; // done2
                    }
                    if st.scanner.cur_cmd != THE {
                        expand_in(st)?;
                    } else {
                        let q = the_toks_in(st)?;
                        let th = crate::units::MemPtr::from_raw(TEMP_HEAD).link(&st.mem);
                        if !th.is_null() {
                            p.set_link(&mut st.mem, th);
                            p = q;
                        }
                    }
                }
                // done2:
                x_token_in(st)?;
            } else {
                get_token_in(st)?;
            }

            if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
                if st.scanner.cur_cmd < RIGHT_BRACE {
                    unbalance += 1;
                } else {
                    unbalance -= 1;
                    if unbalance == 0 {
                        break 'found; // goto found
                    }
                }
            } else if st.scanner.cur_cmd == MAC_PARAM && macro_def {
                // §498: parameter reference in definition body
                let s = st.scanner.cur_tok;
                if xpand {
                    get_x_token_in(st)?;
                } else {
                    get_token_in(st)?;
                }
                if st.scanner.cur_cmd != MAC_PARAM {
                    if st.scanner.cur_tok <= ZERO_TOKEN || st.scanner.cur_tok > t {
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
                        st.scanner.cur_tok = s;
                    } else {
                        st.scanner.cur_tok = (OUT_PARAM_TOKEN - 48) + st.scanner.cur_chr;
                    }
                }
            }
            // append cur_tok to list
            let q = get_avail_scc(st)?;
            p.set_link(&mut st.mem, q);
            q.set_info(&mut st.mem, st.scanner.cur_tok);
            p = q;
        }
    }
    // found:
    st.scanner.scanner_status = NORMAL_SCANNER;
    if hash_brace != 0 {
        let q = get_avail_scc(st)?;
        p.set_link(&mut st.mem, q);
        q.set_info(&mut st.mem, hash_brace);
        p = q;
    }
    Ok(p.0)
}
pub(crate) fn pass_text_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = SKIPPING;
    let mut l: i32 = 0;
    st.scanner.skip_line = st.mem.line;

    loop {
        get_next_scc(st)?;
        if st.scanner.cur_cmd == FI_OR_ELSE {
            if l == 0 {
                break; // done:
            }
            if st.scanner.cur_chr == FI_CODE as i32 {
                l -= 1;
            }
        } else if st.scanner.cur_cmd == IF_TEST {
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
    crate::mem::flush_list(&mut st.mem, crate::units::MemPtr::from_raw(head));

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
    let p = get_avail_scc(st)?;
    let mut q = p.0;

    while l < st.strings.pool_ptr {
        let m = l;
        while l < st.strings.pool_ptr && st.strings.str_pool[l as usize] != nl as u16 {
            l += 1;
        }
        let mut sz = (l - m + 7) / 4;
        if sz == 1 {
            sz = 2;
        }
        let r_base = get_node_scc(st, sz)?.0;
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

    let pl = p.link(&st.mem);
    p.set_info(&mut st.mem, pl.0);
    p.set_link(&mut st.mem, st.input.pseudo_files);
    st.input.pseudo_files = p;
    // pop the temporary string (str_ptr--, pool_ptr = str_start[str_ptr - TOO_BIG_CHAR])
    st.strings.str_ptr -= 1;
    st.strings.pool_ptr = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];

    crate::input::begin_file_reading_in(st)?;
    st.mem.line = 0;
    let start = st.input.cur_input.start;
    st.input.cur_input.limit = start;
    st.input.cur_input.loc = start + 1;

    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_SCAN_TOKENS) > 0 {
        if st.shipout.term_offset > st.config.max_print_line - 3 {
            crate::output::print_ln_in(st);
        } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
            crate::output::print_char_in(st, b' ' as i32);
        }
        st.input.cur_input.name = 19;
        crate::output::print_cstr_in(st, c"( ".as_ptr());
        st.input.open_parens += 1;
        unsafe { ttstub_output_flush(st.shipout.rust_stdout) };
    } else {
        st.input.cur_input.name = 18;
        st.input.cur_input.synctex_tag = 0;
        st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
    }
    Ok(())
}
pub(crate) fn scan_int_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let mut negative = false;
    let mut m: i32;
    let mut d: i32;
    let mut vacuous;
    let mut ok_so_far = true;

    st.scanner.radix = 0;

    // Skip spaces and absorb sign prefix(es).
    loop {
        loop {
            get_x_token_in(st)?;
            if st.scanner.cur_cmd != SPACER {
                break;
            }
        }
        if st.scanner.cur_tok == OTHER_TOKEN + b'-' as i32 {
            negative = !negative;
            st.scanner.cur_tok = OTHER_TOKEN + b'+' as i32;
        }
        if st.scanner.cur_tok != OTHER_TOKEN + b'+' as i32 {
            break;
        }
    }

    'restart: loop {
        if st.scanner.cur_tok == ALPHA_TOKEN {
            get_token_in(st)?;
            if st.scanner.cur_tok < CS_TOKEN_FLAG {
                st.scanner.cur_val = st.scanner.cur_chr;
                if st.scanner.cur_cmd <= 2 {
                    // RIGHT_BRACE = 2 (xetex_format.h:232)
                    if st.scanner.cur_cmd == 2 {
                        st.scanner.align_state += 1;
                    } else {
                        st.scanner.align_state -= 1;
                    }
                }
            } else if st.scanner.cur_tok < CS_TOKEN_FLAG + SINGLE_BASE {
                st.scanner.cur_val = st.scanner.cur_tok - (CS_TOKEN_FLAG + ACTIVE_BASE);
            } else {
                st.scanner.cur_val = st.scanner.cur_tok - (CS_TOKEN_FLAG + SINGLE_BASE);
            }
            if st.scanner.cur_val > BIGGEST_USV {
                error_here_with_diagnostic_in(st, c"Improper alphabetic constant".as_ptr());
                capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"A one-character control sequence belongs after a ` mark.".as_ptr();
                st.errors.help_line[0] = c"So I'm essentially inserting \\0 here.".as_ptr();
                st.scanner.cur_val = b'0' as i32;
                let oc = crate::input::back_error_in(st)?;
                scc_err(oc)?;
            } else {
                get_x_token_in(st)?;
                if st.scanner.cur_cmd != SPACER {
                    crate::input::back_input_in(st)?;
                }
            }
        } else if st.scanner.cur_tok == FROZEN_PRIM_TOKEN {
            get_token_in(st)?;
            if st.scanner.cur_cs < HASH_BASE {
                let v = crate::hash::prim_lookup(
                    &mut st.eqtb,
                    &mut st.strings,
                    st.scanner.cur_cs - SINGLE_BASE,
                )
                .map_err(|e| SccAbort::Input(hash_err_abort(e)))?;
                st.scanner.cur_cs = v;
            } else {
                let s = st.eqtb.hash[(st.scanner.cur_cs as usize) - 514].s1;
                let v = crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, s)
                    .map_err(|e| SccAbort::Input(hash_err_abort(e)))?;
                st.scanner.cur_cs = v;
            }
            if st.scanner.cur_cs != UNDEFINED_PRIMITIVE {
                st.scanner.cur_cmd = st.eqtb.b16s1(PRIM_EQTB_BASE + st.scanner.cur_cs) as u8;
                st.scanner.cur_chr = st.eqtb.b32s1(PRIM_EQTB_BASE + st.scanner.cur_cs);
                st.scanner.cur_cs = PRIM_EQTB_BASE + st.scanner.cur_cs;
                st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
            } else {
                st.scanner.cur_cmd = RELAX;
                st.scanner.cur_chr = 0;
                st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_RELAX;
                st.scanner.cur_cs = FROZEN_RELAX;
            }
            continue 'restart;
        } else if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
            scan_something_internal_in(st, INT_VAL, false)?;
        } else {
            let mut radix_val: i32 = 10;
            m = 0x0CCC_CCCC;
            if st.scanner.cur_tok == OCTAL_TOKEN {
                radix_val = 8;
                m = 0x1000_0000;
                get_x_token_in(st)?;
            } else if st.scanner.cur_tok == HEX_TOKEN {
                radix_val = 16;
                m = 0x0800_0000;
                get_x_token_in(st)?;
            }
            st.scanner.radix = radix_val as u8;
            vacuous = true;
            st.scanner.cur_val = 0;
            loop {
                if st.scanner.cur_tok < ZERO_TOKEN + radix_val
                    && st.scanner.cur_tok >= ZERO_TOKEN
                    && st.scanner.cur_tok <= ZERO_TOKEN + 9
                {
                    d = st.scanner.cur_tok - ZERO_TOKEN;
                } else if radix_val == 16 {
                    if st.scanner.cur_tok <= A_TOKEN + 5 && st.scanner.cur_tok >= A_TOKEN {
                        d = st.scanner.cur_tok - A_TOKEN + 10;
                    } else if st.scanner.cur_tok <= OTHER_A_TOKEN + 5
                        && st.scanner.cur_tok >= OTHER_A_TOKEN
                    {
                        d = st.scanner.cur_tok - OTHER_A_TOKEN + 10;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
                vacuous = false;
                if st.scanner.cur_val >= m && (st.scanner.cur_val > m || d > 7 || radix_val != 10) {
                    if ok_so_far {
                        error_here_with_diagnostic_in(st, c"Number too big".as_ptr());
                        capture_to_diagnostic_in(st, std::ptr::null_mut());
                        st.errors.help_ptr = 2;
                        st.errors.help_line[1] =
                            c"I can only go up to 2147483647='17777777777=\"7FFFFFFF,".as_ptr();
                        st.errors.help_line[0] =
                            c"so I'm using that number instead of yours.".as_ptr();
                        scc_err(crate::errors::error_in(st))?;
                        st.scanner.cur_val = TEX_INFINITY;
                        ok_so_far = false;
                    }
                } else {
                    st.scanner.cur_val = st.scanner.cur_val * radix_val + d;
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
            } else if st.scanner.cur_cmd != SPACER {
                crate::input::back_input_in(st)?;
            }
        }
        break; // end 'restart
    }

    if negative {
        st.scanner.cur_val = -st.scanner.cur_val;
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
                    if st.scanner.cur_cmd != SPACER {
                        break;
                    }
                }
                if st.scanner.cur_tok == MINUS_TOKEN {
                    negative = !negative;
                    st.scanner.cur_tok = PLUS_TOKEN;
                }
                if st.scanner.cur_tok != PLUS_TOKEN {
                    break;
                }
            }

            if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
                if mu {
                    scan_something_internal_in(st, MU_VAL, false)?;
                    if st.scanner.cur_val_level as i16 >= GLUE_VAL {
                        let gm = st.scanner.cur_val;
                        let v = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(gm))
                            .expect("non-null glue spec")
                            .width(&st.mem)
                            .0;
                        crate::mem::delete_glue_ref(&mut st.mem, gm);
                        st.scanner.cur_val = v;
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
                if st.scanner.cur_tok == CONTINENTAL_POINT_TOKEN {
                    st.scanner.cur_tok = POINT_TOKEN;
                }
                if st.scanner.cur_tok != POINT_TOKEN {
                    scan_int_in(st)?;
                } else {
                    st.scanner.radix = 10;
                    st.scanner.cur_val = 0;
                }
                if st.scanner.cur_tok == CONTINENTAL_POINT_TOKEN {
                    st.scanner.cur_tok = POINT_TOKEN;
                }

                if st.scanner.radix == 10 && st.scanner.cur_tok == POINT_TOKEN {
                    let mut k: i32 = 0;
                    let mut p: i32 = crate::mem::TEX_NULL;
                    get_token_in(st)?;
                    'decimal: loop {
                        get_x_token_in(st)?;
                        if st.scanner.cur_tok > ZERO_TOKEN + 9 || st.scanner.cur_tok < ZERO_TOKEN {
                            break 'decimal;
                        }
                        if k < 17 {
                            let q = get_avail_scc(st)?;
                            st.mem.set_b32s1(q, p);
                            q.set_info(&mut st.mem, st.scanner.cur_tok - ZERO_TOKEN);
                            p = q.0;
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
                    if st.scanner.cur_cmd != SPACER {
                        crate::input::back_input_in(st)?;
                    }
                }
            }
        }

        if st.scanner.cur_val < 0 {
            negative = !negative;
            st.scanner.cur_val = -st.scanner.cur_val;
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

                let save_cv = st.scanner.cur_val;
                loop {
                    get_x_token_in(st)?;
                    if st.scanner.cur_cmd != SPACER {
                        break;
                    }
                }

                if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
                    if mu {
                        scan_something_internal_in(st, MU_VAL, false)?;
                        if st.scanner.cur_val_level as i16 >= GLUE_VAL {
                            let gm = st.scanner.cur_val;
                            let v = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(gm))
                                .expect("non-null glue spec")
                                .width(&st.mem)
                                .0;
                            crate::mem::delete_glue_ref(&mut st.mem, gm);
                            st.scanner.cur_val = v;
                        }
                        if st.scanner.cur_val_level as i16 != MU_VAL {
                            scc_err(crate::errors::mu_error_in(st))?;
                        }
                    } else {
                        scan_something_internal_in(st, DIMEN_VAL, false)?;
                    }
                    let v = st.scanner.cur_val;
                    let xod = crate::scaled_math::xn_over_d_in(&mut st.errors, v, f, 65536);
                    st.scanner.cur_val = crate::scaled_math::mult_and_add_in(
                        &mut st.errors,
                        save_cv,
                        v,
                        xod,
                        MAX_HALFWORD,
                    );
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
                    if st.scanner.cur_cmd != SPACER {
                        crate::input::back_input_in(st)?;
                    }
                    let xod = crate::scaled_math::xn_over_d_in(&mut st.errors, v, f, 65536);
                    st.scanner.cur_val = crate::scaled_math::mult_and_add_in(
                        &mut st.errors,
                        save_cv,
                        v,
                        xod,
                        MAX_HALFWORD,
                    );
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
                        st.scanner.cur_val = crate::scaled_math::xn_over_d_in(
                            &mut st.errors,
                            st.scanner.cur_val,
                            1000,
                            mag,
                        );
                        f = (1000 * f + 65536 * st.errors.tex_remainder) / mag;
                        st.scanner.cur_val = st.scanner.cur_val + f / 65536;
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

                st.scanner.cur_val = crate::scaled_math::xn_over_d_in(
                    &mut st.errors,
                    st.scanner.cur_val,
                    num,
                    denom,
                );
                f = (num * f + 65536 * st.errors.tex_remainder) / denom;
                st.scanner.cur_val = st.scanner.cur_val + f / 65536;
                f %= 65536;

                (true, true) // attach_fraction
            };

            if apply_fraction {
                if st.scanner.cur_val >= 16384 {
                    st.errors.arith_error = true;
                } else {
                    st.scanner.cur_val = st.scanner.cur_val * 65536 + f;
                }
            }
            if read_spacer {
                get_x_token_in(st)?;
                if st.scanner.cur_cmd != SPACER {
                    crate::input::back_input_in(st)?;
                }
            }
        } else {
            // !requires_units
            if st.scanner.cur_val >= 16384 {
                st.errors.arith_error = true;
            } else {
                st.scanner.cur_val = st.scanner.cur_val * 65536 + f;
            }
        }
    } // end 'body

    // attach_sign:
    if st.errors.arith_error || st.scanner.cur_val.abs() >= 0x4000_0000 {
        error_here_with_diagnostic_in(st, c"Dimension too large".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] = c"I can't work with sizes bigger than about 19 feet.".as_ptr();
        st.errors.help_line[0] = c"Continue and I'll use the largest value I can.".as_ptr();
        scc_err(crate::errors::error_in(st))?;
        st.scanner.cur_val = MAX_HALFWORD;
        st.errors.arith_error = false;
    }
    if negative {
        st.scanner.cur_val = -st.scanner.cur_val;
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
            if st.scanner.cur_cmd != SPACER {
                break;
            }
        }
        if st.scanner.cur_tok == MINUS_TOKEN {
            negative = !negative;
            st.scanner.cur_tok = PLUS_TOKEN;
        }
        if st.scanner.cur_tok != PLUS_TOKEN {
            break;
        }
    }

    if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
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
            st.scanner.cur_val = -st.scanner.cur_val;
        }
    }

    let gs = new_spec_scc(st, 0)?;
    gs.set_width(&mut st.mem, Scaled(st.scanner.cur_val));

    if scan_keyword_in(st, c"plus".as_ptr())? {
        scan_dimen_in(st, mu, true, false)?;
        gs.set_stretch(&mut st.mem, Scaled(st.scanner.cur_val));
        gs.set_stretch_order(&mut st.mem, st.scanner.cur_order as u16);
    }
    if scan_keyword_in(st, c"minus".as_ptr())? {
        scan_dimen_in(st, mu, true, false)?;
        gs.set_shrink(&mut st.mem, Scaled(st.scanner.cur_val));
        gs.set_shrink_order(&mut st.mem, st.scanner.cur_order as u16);
    }

    st.scanner.cur_val = gs.ptr().0;
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
    if st.scanner.expand_depth_count >= st.scanner.expand_depth {
        return Err(SccAbort::Input(crate::input::InputAbort::Overflow {
            what: c"expansion depth".as_ptr(),
            n: st.scanner.expand_depth,
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
                    if st.scanner.cur_cmd != SPACER {
                        break;
                    }
                }
                if st.scanner.cur_tok == OTHER_TOKEN + 40 {
                    // '('
                    let q = get_node_scc(st, EXPR_NODE_SIZE)?.0;
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
                f = st.scanner.cur_val;
            }
            skip_scan = false;

            // found:
            loop {
                get_x_token_in(st)?;
                if st.scanner.cur_cmd != SPACER {
                    break;
                }
            }
            let mut o: i16 = if st.scanner.cur_tok == OTHER_TOKEN + 43 {
                EXPR_ADD
            } else if st.scanner.cur_tok == OTHER_TOKEN + 45 {
                EXPR_SUB
            } else if st.scanner.cur_tok == OTHER_TOKEN + 42 {
                EXPR_MULT
            } else if st.scanner.cur_tok == OTHER_TOKEN + 47 {
                EXPR_DIV
            } else {
                EXPR_NONE
            };
            if o == EXPR_NONE {
                if p == crate::mem::TEX_NULL {
                    if st.scanner.cur_cmd != RELAX {
                        crate::input::back_input_in(st)?;
                    }
                } else if st.scanner.cur_tok != OTHER_TOKEN + 41 {
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
                f = new_spec_scc(st, 0)?.ptr().0;
            }

            // o may be modified inside case 3 (EXPR_MULT → EXPR_SCALE)
            match s {
                0 => {
                    if l >= GLUE_VAL_EXPR && o != EXPR_NONE {
                        t = new_spec_scc(st, f)?.ptr().0;
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
                crate::mem::free_node(
                    &mut st.mem,
                    crate::units::MemPtr::from_raw(q),
                    EXPR_NODE_SIZE,
                );
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
    st.scanner.cur_val = e;
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
        let save_cur_cs = st.scanner.cur_cs;
        loop {
            get_x_token_in(st)?;
            if st.scanner.cur_cs == 0 && (st.scanner.cur_chr == c || st.scanner.cur_chr == c - 32) {
                let q = get_avail_scc(st)?;
                st.mem.set_b32s1(p, q.0);
                q.set_info(&mut st.mem, st.scanner.cur_tok);
                let head = st.mem.b32s1(BACKUP_HEAD);
                crate::mem::flush_list(&mut st.mem, crate::units::MemPtr::from_raw(head));
                return Ok(true);
            } else if st.scanner.cur_cmd != SPACER || p != BACKUP_HEAD {
                crate::input::back_input_in(st)?;
                if p != BACKUP_HEAD {
                    let head = st.mem.b32s1(BACKUP_HEAD);
                    crate::input::begin_token_list_in(st, head, BACKED_UP)?;
                }
                st.scanner.cur_cs = save_cur_cs;
                return Ok(false);
            }
        }
    }

    let mut i = 0usize;
    while i < slen {
        get_x_token_in(st)?;
        let ch = unsafe { *s.add(i) } as i32;
        if st.scanner.cur_cs == 0 && (st.scanner.cur_chr == ch || st.scanner.cur_chr == ch - 32) {
            let q = get_avail_scc(st)?;
            st.mem.set_b32s1(p, q.0);
            q.set_info(&mut st.mem, st.scanner.cur_tok);
            p = q.0;
            i += 1;
        } else if st.scanner.cur_cmd != SPACER || p != BACKUP_HEAD {
            crate::input::back_input_in(st)?;
            if p != BACKUP_HEAD {
                let head = st.mem.b32s1(BACKUP_HEAD);
                crate::input::begin_token_list_in(st, head, BACKED_UP)?;
            }
            return Ok(false);
        }
    }
    let head = st.mem.b32s1(BACKUP_HEAD);
    crate::mem::flush_list(&mut st.mem, crate::units::MemPtr::from_raw(head));
    Ok(true)
}
pub(crate) fn scan_left_brace_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    loop {
        get_x_token_in(st)?;
        if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
            break;
        }
    }
    if st.scanner.cur_cmd != LEFT_BRACE {
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
        st.scanner.cur_tok = LEFT_BRACE_TOKEN + '{' as i32;
        st.scanner.cur_cmd = LEFT_BRACE;
        st.scanner.cur_chr = '{' as i32;
        st.scanner.align_state += 1;
    }
    Ok(())
}
pub(crate) fn scan_register_num_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > st.scanner.max_reg_num {
        let help1 = st.scanner.max_reg_help_line;
        leaf_int_range_error(st, c"Bad register code".as_ptr(), help1)?;
    }
    Ok(())
}
pub(crate) fn scan_four_bit_int_or_18_in(
    st: &mut crate::state::EngineState,
) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || (st.scanner.cur_val > 15 && st.scanner.cur_val != 18) {
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
    let n: i32;
    let k: i32;
    let kk: i32;
    let mut q: i32;
    let mut r: i32;
    let mut tx: i32;

    // The C function has a `goto restart` handled here with a loop.
    'restart: loop {
        m = st.scanner.cur_chr;

        match st.scanner.cur_cmd {
            DEF_CODE => {
                scan_usv_num_in(st)?;
                if m == MATH_CODE_BASE {
                    let code = st.eqtb.b32s1(MATH_CODE_BASE + st.scanner.cur_val);
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
                        unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val1)) };
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
                    st.scanner.cur_val = v;
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else if m == DEL_CODE_BASE {
                    let code = st.eqtb.b32s1(DEL_CODE_BASE + st.scanner.cur_val);
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
                        st.scanner.cur_val = 0;
                        st.scanner.cur_val_level = INT_VAL as u8;
                    } else {
                        st.scanner.cur_val = st.scanner.cur_val1;
                        st.scanner.cur_val_level = INT_VAL as u8;
                    }
                } else if m < SF_CODE_BASE {
                    st.scanner.cur_val = st.eqtb.b32s1(m + st.scanner.cur_val);
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else if m < MATH_CODE_BASE {
                    st.scanner.cur_val = st.eqtb.b32s1(m + st.scanner.cur_val) % 65536;
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else {
                    st.scanner.cur_val = st.eqtb.b32s1(m + st.scanner.cur_val);
                    st.scanner.cur_val_level = INT_VAL as u8;
                }
            }
            XETEX_DEF_CODE => {
                scan_usv_num_in(st)?;
                if m == SF_CODE_BASE {
                    st.scanner.cur_val = st.eqtb.b32s1(SF_CODE_BASE + st.scanner.cur_val) / 65536;
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else if m == MATH_CODE_BASE {
                    st.scanner.cur_val = st.eqtb.b32s1(MATH_CODE_BASE + st.scanner.cur_val);
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
                    st.scanner.cur_val = 0;
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else if m == DEL_CODE_BASE {
                    st.scanner.cur_val = st.eqtb.b32s1(DEL_CODE_BASE + st.scanner.cur_val);
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
                    st.scanner.cur_val = 0;
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
                    st.scanner.cur_val = 0;
                    st.scanner.cur_val_level = DIMEN_VAL as u8;
                } else if st.scanner.cur_cmd <= ASSIGN_TOKS {
                    if st.scanner.cur_cmd < ASSIGN_TOKS {
                        if m == 0 {
                            scan_register_num_in(st)?;
                            if st.scanner.cur_val < 256 {
                                st.scanner.cur_val = st.eqtb.b32s1(TOKS_BASE + st.scanner.cur_val);
                            } else {
                                let __cv25 = st.scanner.cur_val;
                                crate::save::find_sa_element(
                                    &mut st.mem,
                                    &mut st.scanner,
                                    TOK_VAL as u16,
                                    __cv25,
                                    false,
                                );
                                if st.scanner.cur_ptr.0 == TEX_NULL {
                                    st.scanner.cur_val = TEX_NULL;
                                } else {
                                    st.scanner.cur_val = st.mem.b32s1(st.scanner.cur_ptr + 1);
                                }
                            }
                        } else {
                            st.scanner.cur_val = st.mem.b32s1(m + 1);
                        }
                    } else if st.scanner.cur_chr == LOCAL_BASE + LOCAL_XETEX_INTER_CHAR_TOKS {
                        scan_char_class_not_ignored_in(st)?;
                        st.scanner.cur_ptr.0 = st.scanner.cur_val;
                        scan_char_class_not_ignored_in(st)?;
                        let __cv24 = st.scanner.cur_val;
                        let __cp24 = st.scanner.cur_ptr.0;
                        crate::save::find_sa_element(
                            &mut st.mem,
                            &mut st.scanner,
                            INTER_CHAR_VAL as u16,
                            __cp24 * CHAR_CLASS_LIMIT + __cv24,
                            false,
                        );
                        if st.scanner.cur_ptr.0 == TEX_NULL {
                            st.scanner.cur_val = TEX_NULL;
                        } else {
                            st.scanner.cur_val = st.mem.b32s1(st.scanner.cur_ptr + 1);
                        }
                    } else {
                        st.scanner.cur_val = st.eqtb.b32s1(m);
                    }
                    st.scanner.cur_val_level = TOK_VAL as u8;
                } else {
                    crate::input::back_input_in(st)?;
                    scan_font_ident_in(st)?;
                    st.scanner.cur_val = FONT_ID_BASE + st.scanner.cur_val;
                    st.scanner.cur_val_level = IDENT_VAL as u8;
                }
            }
            ASSIGN_INT => {
                st.scanner.cur_val = st.eqtb.b32s1(m);
                st.scanner.cur_val_level = INT_VAL as u8;
            }
            ASSIGN_DIMEN => {
                st.scanner.cur_val = st.eqtb.b32s1(m);
                st.scanner.cur_val_level = DIMEN_VAL as u8;
            }
            ASSIGN_GLUE => {
                st.scanner.cur_val = st.eqtb.b32s1(m);
                st.scanner.cur_val_level = GLUE_VAL as u8;
            }
            ASSIGN_MU_GLUE => {
                st.scanner.cur_val = st.eqtb.b32s1(m);
                st.scanner.cur_val_level = MU_VAL as u8;
            }
            SET_AUX => {
                if (st.input.cur_list.mode.abs() as i32) != m {
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
                        st.scanner.cur_val = 0;
                        st.scanner.cur_val_level = DIMEN_VAL as u8;
                    } else {
                        st.scanner.cur_val = 0;
                        st.scanner.cur_val_level = INT_VAL as u8;
                    }
                } else if m == VMODE as i32 {
                    st.scanner.cur_val = st.input.cur_list.aux_s1();
                    st.scanner.cur_val_level = DIMEN_VAL as u8;
                } else {
                    st.scanner.cur_val = st.input.cur_list.aux_s0();
                    st.scanner.cur_val_level = INT_VAL as u8;
                }
            }
            SET_PREV_GRAF => {
                if st.input.cur_list.mode == 0 {
                    st.scanner.cur_val = 0;
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else {
                    let np = st.input.nest_ptr;
                    st.input.nest[np as usize] = st.input.cur_list;
                    let mut p = np;
                    while (st.input.nest[p as usize].mode.abs() as i32) != VMODE as i32 {
                        p -= 1;
                    }
                    st.scanner.cur_val = st.input.nest[p as usize].prev_graf;
                    st.scanner.cur_val_level = INT_VAL as u8;
                }
            }
            SET_PAGE_INT => {
                if m == 0 {
                    st.scanner.cur_val = st.shipout.dead_cycles;
                } else if m == 2 {
                    st.scanner.cur_val = st.errors.interaction as i32;
                } else {
                    st.scanner.cur_val = st.shipout.insert_penalties;
                }
                st.scanner.cur_val_level = INT_VAL as u8;
            }
            SET_PAGE_DIMEN => {
                if st.shipout.page_contents == 0 && !st.shipout.output_active {
                    if m == 0 {
                        st.scanner.cur_val = MAX_HALFWORD;
                    } else {
                        st.scanner.cur_val = 0;
                    }
                } else {
                    st.scanner.cur_val = st.shipout.page_so_far[m as usize];
                }
                st.scanner.cur_val_level = DIMEN_VAL as u8;
            }
            SET_SHAPE => {
                if m > LOCAL_BASE + LOCAL_PAR_SHAPE {
                    scan_int_in(st)?;
                    let shape_ptr = st.eqtb.b32s1(m);
                    if shape_ptr == TEX_NULL || st.scanner.cur_val < 0 {
                        st.scanner.cur_val = 0;
                    } else {
                        let cnt = st.mem.b32s1(shape_ptr + 1);
                        if st.scanner.cur_val > cnt {
                            st.scanner.cur_val = cnt;
                        }
                        st.scanner.cur_val = st.mem.b32s1(shape_ptr + st.scanner.cur_val + 1);
                    }
                } else {
                    let par_shape = st.eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
                    if par_shape == TEX_NULL {
                        st.scanner.cur_val = 0;
                    } else {
                        st.scanner.cur_val = st.mem.b32s0(par_shape);
                    }
                }
                st.scanner.cur_val_level = INT_VAL as u8;
            }
            SET_BOX_DIMEN => {
                scan_register_num_in(st)?;
                if st.scanner.cur_val < 256 {
                    q = st.eqtb.b32s1(BOX_BASE + st.scanner.cur_val);
                } else {
                    let __cv23 = st.scanner.cur_val;
                    crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, __cv23, false);
                    if st.scanner.cur_ptr.0 == TEX_NULL {
                        q = TEX_NULL;
                    } else {
                        q = st.mem.b32s1(st.scanner.cur_ptr + 1);
                    }
                }
                if q == TEX_NULL {
                    st.scanner.cur_val = 0;
                } else {
                    st.scanner.cur_val = st.mem.b32s1(q + m);
                }
                st.scanner.cur_val_level = DIMEN_VAL as u8;
            }
            CHAR_GIVEN | MATH_GIVEN | XETEX_MATH_GIVEN => {
                st.scanner.cur_val = st.scanner.cur_chr;
                st.scanner.cur_val_level = INT_VAL as u8;
            }
            ASSIGN_FONT_DIMEN => {
                find_font_dimen_in(st, false)?;
                let fp = st.fonts.fmem_ptr;
                st.fonts.set_font_info_b32s1(fp, 0);
                st.scanner.cur_val =
                    unsafe { st.fonts.font_info[st.scanner.cur_val as usize].b32.s1 };
                st.scanner.cur_val_level = DIMEN_VAL as u8;
            }
            ASSIGN_FONT_INT => {
                scan_font_ident_in(st)?;
                if m == 0 {
                    st.scanner.cur_val = st.fonts.hyphen_char[st.scanner.cur_val as usize];
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else if m == 1 {
                    st.scanner.cur_val = st.fonts.skew_char[st.scanner.cur_val as usize];
                    st.scanner.cur_val_level = INT_VAL as u8;
                } else {
                    n = st.scanner.cur_val;
                    let fa = st.fonts.font_area[n as usize];
                    if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                        scan_glyph_number_in(st, n)?;
                    } else {
                        scan_char_num_in(st)?;
                    }
                    k = st.scanner.cur_val;
                    match m {
                        LP_CODE_BASE => {
                            st.scanner.cur_val = unsafe { get_cp_code(n, k as u32, LEFT_SIDE) };
                            st.scanner.cur_val_level = INT_VAL as u8;
                        }
                        RP_CODE_BASE => {
                            st.scanner.cur_val = unsafe { get_cp_code(n, k as u32, RIGHT_SIDE) };
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
                        st.scanner.cur_val = st.mem.b32s1(m + 2);
                    } else {
                        st.scanner.cur_val = st.mem.b32s1(m + 1);
                    }
                } else {
                    scan_register_num_in(st)?;
                    st.scanner.cur_val_level = m as u8;
                    if st.scanner.cur_val > 255 {
                        let cvl = st.scanner.cur_val_level as u16;
                        let __cv22 = st.scanner.cur_val;
                        crate::save::find_sa_element(
                            &mut st.mem,
                            &mut st.scanner,
                            cvl,
                            __cv22,
                            false,
                        );
                        if st.scanner.cur_ptr.0 == TEX_NULL {
                            st.scanner.cur_val = 0;
                        } else if (st.scanner.cur_val_level as i16) < GLUE_VAL {
                            st.scanner.cur_val = st.mem.b32s1(st.scanner.cur_ptr + 2);
                        } else {
                            st.scanner.cur_val = st.mem.b32s1(st.scanner.cur_ptr + 1);
                        }
                    } else {
                        match st.scanner.cur_val_level as i16 {
                            INT_VAL => {
                                st.scanner.cur_val = st.eqtb.b32s1(COUNT_BASE + st.scanner.cur_val)
                            }
                            DIMEN_VAL => {
                                st.scanner.cur_val = st.eqtb.b32s1(SCALED_BASE + st.scanner.cur_val)
                            }
                            GLUE_VAL => {
                                st.scanner.cur_val = st.eqtb.b32s1(SKIP_BASE + st.scanner.cur_val)
                            }
                            MU_VAL => {
                                st.scanner.cur_val =
                                    st.eqtb.b32s1(MU_SKIP_BASE + st.scanner.cur_val)
                            }
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
                                let gm = st.scanner.cur_val;
                                let gs =
                                    NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(gm))
                                        .expect("glue spec");
                                st.scanner.cur_val = gs.width(&st.mem).0;
                                crate::mem::delete_glue_ref(&mut st.mem, gm);
                            } else if (st.scanner.cur_val_level as i16) == MU_VAL {
                                scc_err(crate::errors::mu_error_in(st))?;
                            }
                            st.scanner.cur_val_level -= 1;
                        }
                        if negative {
                            if (st.scanner.cur_val_level as i16) >= GLUE_VAL {
                                let gm = st.scanner.cur_val;
                                let gs = new_spec_scc(st, gm)?;
                                st.scanner.cur_val = gs.ptr().0;
                                crate::mem::delete_glue_ref(&mut st.mem, gm);
                                let (w, str_, shr) =
                                    (gs.width(&st.mem), gs.stretch(&st.mem), gs.shrink(&st.mem));
                                gs.set_width(&mut st.mem, Scaled(-w.0));
                                gs.set_stretch(&mut st.mem, Scaled(-str_.0));
                                gs.set_shrink(&mut st.mem, Scaled(-shr.0));
                            } else {
                                st.scanner.cur_val = -st.scanner.cur_val;
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
                                    n = st.scanner.cur_val;
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
                                        st.scanner.cur_val = 0;
                                    } else {
                                        scan_int_in(st)?;
                                        let v = unsafe {
                                            crate::ext::get_glyph_bounds(
                                                &st.fonts,
                                                cf,
                                                n,
                                                st.scanner.cur_val,
                                            )
                                        }?;
                                        st.scanner.cur_val = v;
                                    }
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        cf,
                                        c"; not a native platform font".as_ptr(),
                                    ))?;
                                    st.scanner.cur_val = 0;
                                }
                            }
                            FONT_CHAR_WD_CODE | FONT_CHAR_HT_CODE | FONT_CHAR_DP_CODE
                            | FONT_CHAR_IC_CODE => {
                                scan_font_ident_in(st)?;
                                q = st.scanner.cur_val;
                                scan_usv_num_in(st)?;
                                let fa = st.fonts.font_area[q as usize];
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    let v = unsafe {
                                        match m {
                                            FONT_CHAR_WD_CODE => crate::ext::getnativecharwd(
                                                &st.fonts,
                                                q,
                                                st.scanner.cur_val,
                                            ),
                                            FONT_CHAR_HT_CODE => crate::ext::getnativecharht(
                                                &st.fonts,
                                                q,
                                                st.scanner.cur_val,
                                            ),
                                            FONT_CHAR_DP_CODE => crate::ext::getnativechardp(
                                                &st.fonts,
                                                q,
                                                st.scanner.cur_val,
                                            ),
                                            _ => crate::ext::getnativecharic(
                                                &st.fonts,
                                                q,
                                                st.scanner.cur_val,
                                            ),
                                        }
                                    }?;
                                    st.scanner.cur_val = v;
                                } else {
                                    let bc = st.fonts.font_bc[q as usize] as i32;
                                    let ec = st.fonts.font_ec[q as usize] as i32;
                                    if bc <= st.scanner.cur_val && ec >= st.scanner.cur_val {
                                        let ec_char = crate::fonts::effective_char(
                                            &mut st.fonts,
                                            true,
                                            q,
                                            st.scanner.cur_val as u16,
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
                                        st.scanner.cur_val = v;
                                    } else {
                                        st.scanner.cur_val = 0;
                                    }
                                }
                            }
                            PAR_SHAPE_LENGTH_CODE
                            | PAR_SHAPE_INDENT_CODE
                            | PAR_SHAPE_DIMEN_CODE => {
                                q = st.scanner.cur_chr - PAR_SHAPE_LENGTH_CODE;
                                scan_int_in(st)?;
                                let par_shape = st.eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
                                if par_shape == TEX_NULL || st.scanner.cur_val <= 0 {
                                    st.scanner.cur_val = 0;
                                } else {
                                    if q == 2 {
                                        q = st.scanner.cur_val % 2;
                                        st.scanner.cur_val = (st.scanner.cur_val + q) / 2;
                                    }
                                    let cnt = st.mem.b32s0(par_shape);
                                    if st.scanner.cur_val > cnt {
                                        st.scanner.cur_val = cnt;
                                    }
                                    st.scanner.cur_val =
                                        st.mem.b32s1(par_shape + 2 * st.scanner.cur_val - q);
                                }
                                st.scanner.cur_val_level = DIMEN_VAL as u8;
                            }
                            GLUE_STRETCH_CODE | GLUE_SHRINK_CODE => {
                                scan_normal_glue_in(st)?;
                                q = st.scanner.cur_val;
                                let gs =
                                    NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(q))
                                        .expect("glue spec");
                                if m == GLUE_STRETCH_CODE {
                                    st.scanner.cur_val = gs.stretch(&st.mem).0;
                                } else {
                                    st.scanner.cur_val = gs.shrink(&st.mem).0;
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
                                st.scanner.cur_val = st.mem.line;
                            }
                            BADNESS_CODE => {
                                st.scanner.cur_val = st.shipout.last_badness;
                            }
                            ELAPSED_TIME_CODE => {
                                st.scanner.cur_val = if st.config.deterministic_mode {
                                    0
                                } else {
                                    unsafe { crate::texmfmp::get_microinterval(&st.shipout) }
                                };
                            }
                            RANDOM_SEED_CODE => {
                                st.scanner.cur_val = st.errors.random_seed;
                            }
                            PDF_SHELL_ESCAPE_CODE => {
                                st.scanner.cur_val =
                                    if st.config.shell_escape_enabled { 1 } else { 0 };
                            }
                            ETEX_VERSION_CODE => {
                                st.scanner.cur_val = ETEX_VERSION;
                            }
                            XETEX_VERSION_CODE => {
                                st.scanner.cur_val = XETEX_VERSION;
                            }
                            XETEX_COUNT_GLYPHS_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                st.scanner.cur_val = if fa == AAT_FONT_FLAG {
                                    aat_font_get(m - XETEX_INT, engine)
                                } else if fa == OTGR_FONT_FLAG {
                                    unsafe { ot_font_get(m - XETEX_INT, engine) }
                                } else {
                                    0
                                };
                            }
                            XETEX_COUNT_FEATURES_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                st.scanner.cur_val = if fa == AAT_FONT_FLAG {
                                    aat_font_get(m - XETEX_INT, engine)
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    unsafe { ot_font_get(m - XETEX_INT, engine) }
                                } else {
                                    0
                                };
                            }
                            XETEX_VARIATION_CODE
                            | XETEX_VARIATION_MIN_CODE
                            | XETEX_VARIATION_MAX_CODE
                            | XETEX_VARIATION_DEFAULT_CODE
                            | XETEX_COUNT_VARIATIONS_CODE => {
                                scan_font_ident_in(st)?;
                                st.scanner.cur_val = 0;
                            }
                            XETEX_FEATURE_CODE_CODE
                            | XETEX_IS_EXCLUSIVE_FEATURE_CODE
                            | XETEX_COUNT_SELECTORS_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG {
                                    scan_int_in(st)?;
                                    k = st.scanner.cur_val;
                                    st.scanner.cur_val = aat_font_get_1(m - XETEX_INT, engine, k);
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    scan_int_in(st)?;
                                    k = st.scanner.cur_val;
                                    st.scanner.cur_val =
                                        unsafe { ot_font_get_1(m - XETEX_INT, engine, k) };
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an AAT or Graphite font".as_ptr(),
                                    ))?;
                                    st.scanner.cur_val = -1;
                                }
                            }
                            XETEX_SELECTOR_CODE_CODE | XETEX_IS_DEFAULT_SELECTOR_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG {
                                    scan_int_in(st)?;
                                    k = st.scanner.cur_val;
                                    scan_int_in(st)?;
                                    st.scanner.cur_val = aat_font_get_2(
                                        m - XETEX_INT,
                                        engine,
                                        k,
                                        st.scanner.cur_val,
                                    );
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    scan_int_in(st)?;
                                    k = st.scanner.cur_val;
                                    scan_int_in(st)?;
                                    st.scanner.cur_val = unsafe {
                                        ot_font_get_2(m - XETEX_INT, engine, k, st.scanner.cur_val)
                                    };
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an AAT or Graphite font".as_ptr(),
                                    ))?;
                                    st.scanner.cur_val = -1;
                                }
                            }
                            XETEX_FIND_VARIATION_BY_NAME_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG {
                                    crate::io::scan_and_pack_name_in(st)?;
                                    st.scanner.cur_val = aat_font_get_named(m - XETEX_INT, engine);
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an AAT font".as_ptr(),
                                    ))?;
                                    st.scanner.cur_val = -1;
                                }
                            }
                            XETEX_FIND_FEATURE_BY_NAME_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG {
                                    crate::io::scan_and_pack_name_in(st)?;
                                    st.scanner.cur_val = aat_font_get_named(m - XETEX_INT, engine);
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    crate::io::scan_and_pack_name_in(st)?;
                                    st.scanner.cur_val =
                                        unsafe { gr_font_get_named(st, m - XETEX_INT, engine) };
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an AAT or Graphite font".as_ptr(),
                                    ))?;
                                    st.scanner.cur_val = -1;
                                }
                            }
                            XETEX_FIND_SELECTOR_BY_NAME_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG {
                                    scan_int_in(st)?;
                                    k = st.scanner.cur_val;
                                    crate::io::scan_and_pack_name_in(st)?;
                                    st.scanner.cur_val =
                                        aat_font_get_named_1(m - XETEX_INT, engine, k);
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    scan_int_in(st)?;
                                    k = st.scanner.cur_val;
                                    crate::io::scan_and_pack_name_in(st)?;
                                    st.scanner.cur_val = unsafe {
                                        gr_font_get_named_1(st, m - XETEX_INT, engine, k)
                                    };
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an AAT or Graphite font".as_ptr(),
                                    ))?;
                                    st.scanner.cur_val = -1;
                                }
                            }
                            XETEX_OT_COUNT_SCRIPTS_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                st.scanner.cur_val =
                                    if fa == OTGR_FONT_FLAG && unsafe { usingOpenType(engine) } {
                                        unsafe { ot_font_get(m - XETEX_INT, engine) }
                                    } else {
                                        0
                                    };
                            }
                            XETEX_OT_COUNT_LANGUAGES_CODE | XETEX_OT_SCRIPT_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                if fa == OTGR_FONT_FLAG && unsafe { usingOpenType(engine) } {
                                    scan_int_in(st)?;
                                    st.scanner.cur_val = unsafe {
                                        ot_font_get_1(m - XETEX_INT, engine, st.scanner.cur_val)
                                    };
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an OpenType Layout font".as_ptr(),
                                    ))?;
                                    st.scanner.cur_val = -1;
                                }
                            }
                            XETEX_OT_COUNT_FEATURES_CODE | XETEX_OT_LANGUAGE_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                if fa == OTGR_FONT_FLAG && unsafe { usingOpenType(engine) } {
                                    scan_int_in(st)?;
                                    k = st.scanner.cur_val;
                                    scan_int_in(st)?;
                                    st.scanner.cur_val = unsafe {
                                        ot_font_get_2(m - XETEX_INT, engine, k, st.scanner.cur_val)
                                    };
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an OpenType Layout font".as_ptr(),
                                    ))?;
                                    st.scanner.cur_val = -1;
                                }
                            }
                            XETEX_OT_FEATURE_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                if fa == OTGR_FONT_FLAG && unsafe { usingOpenType(engine) } {
                                    scan_int_in(st)?;
                                    k = st.scanner.cur_val;
                                    scan_int_in(st)?;
                                    kk = st.scanner.cur_val;
                                    scan_int_in(st)?;
                                    st.scanner.cur_val = unsafe {
                                        ot_font_get_3(
                                            m - XETEX_INT,
                                            engine,
                                            k,
                                            kk,
                                            st.scanner.cur_val,
                                        )
                                    };
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        n,
                                        c"; not an OpenType Layout font".as_ptr(),
                                    ))?;
                                    st.scanner.cur_val = -1;
                                }
                            }
                            XETEX_MAP_CHAR_TO_GLYPH_CODE => {
                                let cf = st.eqtb.b32s1(CUR_FONT_LOC);
                                let fa = st.fonts.font_area[cf as usize];
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    scan_int_in(st)?;
                                    n = st.scanner.cur_val;
                                    let v =
                                        unsafe { crate::ext::map_char_to_glyph(&st.fonts, cf, n) }?;
                                    st.scanner.cur_val = v;
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        cf,
                                        c"; not a native platform font".as_ptr(),
                                    ))?;
                                    st.scanner.cur_val = 0;
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
                                    }?;
                                    st.scanner.cur_val = v;
                                } else {
                                    scc_err(crate::errors::not_font_error_common_in(
                                        st,
                                        LAST_ITEM as u16,
                                        m,
                                        cf,
                                        c"; not a native platform font".as_ptr(),
                                    ))?;
                                    st.scanner.cur_val = 0;
                                }
                            }
                            XETEX_FONT_TYPE_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let engine = st.fonts.font_layout_engine_ptr(n);
                                let fa = st.fonts.font_area[n as usize];
                                st.scanner.cur_val = if fa == AAT_FONT_FLAG {
                                    1
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingOpenType(engine) } {
                                    2
                                } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                                    3
                                } else {
                                    0
                                };
                            }
                            XETEX_FIRST_CHAR_CODE | XETEX_LAST_CHAR_CODE => {
                                scan_font_ident_in(st)?;
                                n = st.scanner.cur_val;
                                let fa = st.fonts.font_area[n as usize];
                                if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                                    let v = unsafe {
                                        crate::ext::get_font_char_range(
                                            &st.fonts,
                                            n,
                                            (m == XETEX_FIRST_CHAR_CODE) as i32,
                                        )
                                    }?;
                                    st.scanner.cur_val = v;
                                } else if m == XETEX_FIRST_CHAR_CODE {
                                    st.scanner.cur_val = st.fonts.font_bc[n as usize] as i32;
                                } else {
                                    st.scanner.cur_val = st.fonts.font_ec[n as usize] as i32;
                                }
                            }
                            PDF_LAST_X_POS_CODE => {
                                st.scanner.cur_val = st.shipout.pdf_last_x_pos;
                            }
                            PDF_LAST_Y_POS_CODE => {
                                st.scanner.cur_val = st.shipout.pdf_last_y_pos;
                            }
                            XETEX_PDF_PAGE_COUNT_CODE => {
                                crate::io::scan_and_pack_name_in(st)?;
                                let name = if st.input.name_of_file.is_empty() {
                                    std::ptr::null_mut()
                                } else {
                                    st.input.name_of_file.as_ptr() as *const libc::c_char
                                };
                                let resident = st.config.execution_mode
                                    == crate::state::EngineExecutionMode::Resident;
                                st.scanner.cur_val = unsafe {
                                    crate::pic::count_pdf_file_pages_named(name, resident)
                                }?;
                            }
                            CURRENT_GROUP_LEVEL_CODE => {
                                st.scanner.cur_val = st.save_stack.cur_level as i32 - 1;
                            }
                            CURRENT_GROUP_TYPE_CODE => {
                                st.scanner.cur_val = st.save_stack.cur_group as i32;
                            }
                            CURRENT_IF_LEVEL_CODE => {
                                q = st.scanner.cond_ptr.0;
                                let mut cnt = 0i32;
                                while q != TEX_NULL {
                                    cnt += 1;
                                    q = st.mem.b32s1(q);
                                }
                                st.scanner.cur_val = cnt;
                            }
                            CURRENT_IF_TYPE_CODE => {
                                if st.scanner.cond_ptr.0 == TEX_NULL {
                                    st.scanner.cur_val = 0;
                                } else if (st.scanner.cur_if as i32) < UNLESS_CODE {
                                    st.scanner.cur_val = st.scanner.cur_if as i32 + 1;
                                } else {
                                    st.scanner.cur_val = -(st.scanner.cur_if as i32 - 31);
                                }
                            }
                            CURRENT_IF_BRANCH_CODE => {
                                let ifl = st.scanner.if_limit;
                                if ifl == OR_CODE || ifl == ELSE_CODE {
                                    st.scanner.cur_val = 1;
                                } else if ifl == FI_CODE {
                                    st.scanner.cur_val = -1;
                                } else {
                                    st.scanner.cur_val = 0;
                                }
                            }
                            GLUE_STRETCH_ORDER_CODE | GLUE_SHRINK_ORDER_CODE => {
                                scan_normal_glue_in(st)?;
                                q = st.scanner.cur_val;
                                let gs =
                                    NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(q))
                                        .expect("non-null glue spec");
                                if m == GLUE_STRETCH_ORDER_CODE {
                                    st.scanner.cur_val = gs.stretch_order(&st.mem) as i32;
                                } else {
                                    st.scanner.cur_val = gs.shrink_order(&st.mem) as i32;
                                }
                                crate::mem::delete_glue_ref(&mut st.mem, q);
                            }
                            _ => {}
                        }
                        st.scanner.cur_val_level = INT_VAL as u8;
                    }
                } else {
                    // m < INPUT_LINE_NO_CODE: last-node queries
                    st.scanner.cur_val = 0;
                    tx = st.input.cur_list.tail.0;
                    if tx < st.mem.hi_mem_min {
                        let node_type = st.mem.b16s1(tx);
                        let subtype = st.mem.b16s0(tx);
                        if node_type == MATH_NODE && subtype == END_M_CODE {
                            r = st.input.cur_list.head.0;
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
                    if st.scanner.cur_chr == LAST_NODE_TYPE_CODE_VAL {
                        st.scanner.cur_val_level = INT_VAL as u8;
                        if tx == st.input.cur_list.head.0 || st.input.cur_list.mode == 0 {
                            st.scanner.cur_val = -1;
                        }
                    } else {
                        st.scanner.cur_val_level = st.scanner.cur_chr as u8;
                    }
                    if tx < st.mem.hi_mem_min && st.input.cur_list.mode != 0 {
                        let node_type = st.mem.b16s1(tx);
                        match st.scanner.cur_chr {
                            x if x == INT_VAL as i32 => {
                                if node_type == PENALTY_NODE {
                                    let pn = st
                                        .mem
                                        .classify_penalty(MemIdx::new(tx))
                                        .expect("valid penalty")
                                        .expect("penalty");
                                    st.scanner.cur_val = pn.penalty(&st.mem);
                                }
                            }
                            x if x == DIMEN_VAL as i32 => {
                                if node_type == KERN_NODE {
                                    let kn = st
                                        .mem
                                        .classify_kern(MemIdx::new(tx))
                                        .expect("valid kern")
                                        .expect("kern");
                                    st.scanner.cur_val = kn.width(&st.mem).0;
                                }
                            }
                            x if x == GLUE_VAL as i32 => {
                                if node_type == GLUE_NODE {
                                    let gn = st
                                        .mem
                                        .classify_glue(MemIdx::new(tx))
                                        .expect("valid glue")
                                        .expect("glue");
                                    st.scanner.cur_val = gn.glue_ptr(&st.mem).0;
                                    if gn.subtype(&st.mem) == MU_GLUE {
                                        st.scanner.cur_val_level = MU_VAL as u8;
                                    }
                                }
                            }
                            x if x == LAST_NODE_TYPE_CODE_VAL => {
                                if node_type <= UNSET_NODE {
                                    st.scanner.cur_val = node_type as i32 + 1;
                                } else {
                                    st.scanner.cur_val = UNSET_NODE as i32 + 2;
                                }
                            }
                            _ => {}
                        }
                    } else if st.input.cur_list.mode == VMODE && tx == st.input.cur_list.head.0 {
                        match st.scanner.cur_chr {
                            x if x == INT_VAL as i32 => {
                                st.scanner.cur_val = st.shipout.last_penalty
                            }
                            x if x == DIMEN_VAL as i32 => st.scanner.cur_val = st.shipout.last_kern,
                            x if x == GLUE_VAL as i32 => {
                                if st.shipout.last_glue.0 != MAX_HALFWORD {
                                    st.scanner.cur_val = st.shipout.last_glue.0;
                                }
                            }
                            x if x == LAST_NODE_TYPE_CODE_VAL => {
                                st.scanner.cur_val = st.shipout.last_node_type
                            }
                            _ => {}
                        }
                    }
                }
            }
            IGNORE_SPACES => {
                if st.scanner.cur_chr == 1 {
                    get_token_in(st)?;
                    if st.scanner.cur_cs < HASH_BASE {
                        let v = crate::hash::prim_lookup(
                            &mut st.eqtb,
                            &mut st.strings,
                            st.scanner.cur_cs - SINGLE_BASE,
                        )
                        .map_err(|e| SccAbort::Input(hash_err_abort(e)))?;
                        st.scanner.cur_cs = v;
                    } else {
                        let s = st.eqtb.hash[(st.scanner.cur_cs as usize) - 514].s1;
                        let v = crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, s)
                            .map_err(|e| SccAbort::Input(hash_err_abort(e)))?;
                        st.scanner.cur_cs = v;
                    }
                    if st.scanner.cur_cs != UNDEFINED_PRIMITIVE {
                        st.scanner.cur_cmd =
                            st.eqtb.b16s1(PRIM_EQTB_BASE + st.scanner.cur_cs) as u8;
                        st.scanner.cur_chr = st.eqtb.b32s1(PRIM_EQTB_BASE + st.scanner.cur_cs);
                        st.scanner.cur_cs = PRIM_EQTB_BASE + st.scanner.cur_cs;
                        st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
                    } else {
                        st.scanner.cur_cmd = RELAX;
                        st.scanner.cur_chr = 0;
                        st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_RELAX;
                        st.scanner.cur_cs = FROZEN_RELAX;
                    }
                    continue 'restart;
                }
            }
            _ => {
                error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
                print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
                print_cstr_in(st, c"' after ".as_ptr());
                print_esc_cstr_in(st, c"the".as_ptr());
                capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 1;
                st.errors.help_line[0] =
                    c"I'm forgetting what you said and using zero instead.".as_ptr();
                scc_err(crate::errors::error_in(st))?;
                st.scanner.cur_val = 0;
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
            let gs = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(st.scanner.cur_val))
                .expect("non-null glue spec");
            st.scanner.cur_val = gs.width(&st.mem).0;
        } else if (st.scanner.cur_val_level as i16) == MU_VAL {
            scc_err(crate::errors::mu_error_in(st))?;
        }
        st.scanner.cur_val_level -= 1;
    }

    if negative {
        if (st.scanner.cur_val_level as i16) >= GLUE_VAL {
            let gm = st.scanner.cur_val;
            let gs = new_spec_scc(st, gm)?;
            st.scanner.cur_val = gs.ptr().0;
            let (w, str_, shr) = (gs.width(&st.mem), gs.stretch(&st.mem), gs.shrink(&st.mem));
            gs.set_width(&mut st.mem, Scaled(-w.0));
            gs.set_stretch(&mut st.mem, Scaled(-str_.0));
            gs.set_shrink(&mut st.mem, Scaled(-shr.0));
        } else {
            st.scanner.cur_val = -st.scanner.cur_val;
        }
    } else if (st.scanner.cur_val_level as i16) >= GLUE_VAL
        && (st.scanner.cur_val_level as i16) <= MU_VAL
    {
        // GLUE_SPEC_ref_count(cur_val)++
        let gs = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(st.scanner.cur_val))
            .expect("non-null glue spec");
        let rc = gs.ref_count(&st.mem) + 1;
        gs.set_ref_count(&mut st.mem, rc);
    }
    Ok(())
}
pub(crate) fn scan_char_num_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > BIGGEST_CHAR {
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
    if st.scanner.cur_val < 0 || st.scanner.cur_val > BIGGEST_USV {
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
    if st.scanner.cur_val < 0 || st.scanner.cur_val > CHAR_CLASS_LIMIT {
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
    if st.scanner.cur_val < 0 || st.scanner.cur_val > NUMBER_MATH_FAMILIES - 1 {
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
        if st.scanner.cur_cmd != SPACER {
            break;
        }
    }
    if st.scanner.cur_cmd == DEF_FONT {
        f = st.eqtb.b32s1(CUR_FONT_LOC);
    } else if st.scanner.cur_cmd == SET_FONT {
        f = st.scanner.cur_chr;
    } else if st.scanner.cur_cmd == DEF_FAMILY {
        let m = st.scanner.cur_chr;
        scan_math_fam_int_in(st)?;
        f = st.eqtb.b32s1(m + st.scanner.cur_val);
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
    st.scanner.cur_val = f;
    Ok(())
}
pub(crate) fn find_font_dimen_in(
    st: &mut crate::state::EngineState,
    writing: bool,
) -> Result<(), SccAbort> {
    use crate::mem::TEX_NULL;

    scan_int_in(st)?;
    let n = st.scanner.cur_val;
    scan_font_ident_in(st)?;
    let f = st.scanner.cur_val;

    if n <= 0 {
        st.scanner.cur_val = st.fonts.fmem_ptr;
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
                st.scanner.cur_val = st.fonts.fmem_ptr;
            } else {
                loop {
                    if st.fonts.fmem_ptr == st.fonts.font_mem_size {
                        return Err(SccAbort::Error(crate::errors::ErrorOutcome::Overflow {
                            what: c"font memory",
                            n: st.fonts.font_mem_size,
                        }));
                    }
                    let k = st.fonts.fmem_ptr;
                    st.fonts.set_font_info_b32s1(k, 0);
                    st.fonts.fmem_ptr += 1;
                    st.fonts.font_params[f as usize] += 1;
                    if n == st.fonts.font_params[f as usize] {
                        break;
                    }
                }
                st.scanner.cur_val = st.fonts.fmem_ptr - 1;
            }
        } else {
            st.scanner.cur_val = n + st.fonts.param_base[f as usize];
        }
    }

    if st.scanner.cur_val == st.fonts.fmem_ptr {
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
        let name = if st.input.name_of_file.is_empty() {
            std::ptr::null_mut()
        } else {
            st.input.name_of_file.as_ptr() as *mut libc::c_char
        };
        let v = unsafe { crate::ext::map_glyph_to_index(&st.fonts, f, name) }?;
        st.scanner.cur_val = v;
        st.scanner.cur_val_level = INT_VAL as u8;
    } else if scan_keyword_in(st, c"u".as_ptr())? {
        scan_char_num_in(st)?;
        let v = unsafe { crate::ext::map_char_to_glyph(&st.fonts, f, st.scanner.cur_val) }?;
        st.scanner.cur_val = v;
        st.scanner.cur_val_level = INT_VAL as u8;
    } else {
        scan_int_in(st)?;
    }
    Ok(())
}
pub(crate) fn get_token_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    st.eqtb.no_new_control_sequence = false;
    get_next_scc(st)?;
    st.eqtb.no_new_control_sequence = true;
    if st.scanner.cur_cs == 0 {
        st.scanner.cur_tok = st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
    } else {
        st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
    }
    Ok(())
}
pub(crate) fn scan_general_text_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let s = st.scanner.scanner_status;
    let w = st.scanner.warning_index;
    let d = st.scanner.def_ref.0;
    st.scanner.scanner_status = ABSORBING;
    st.scanner.warning_index = st.scanner.cur_cs;
    let new_def = get_avail_scc(st)?;
    st.scanner.def_ref = new_def;
    new_def.set_info(&mut st.mem, crate::mem::TEX_NULL);
    let mut p = new_def;
    scan_left_brace_in(st)?;
    let mut unbalance: i32 = 1;
    loop {
        get_token_in(st)?;
        if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
            if st.scanner.cur_cmd < RIGHT_BRACE {
                unbalance += 1;
            } else {
                unbalance -= 1;
                if unbalance == 0 {
                    break; // found:
                }
            }
        }
        let q = get_avail_scc(st)?;
        p.set_link(&mut st.mem, q);
        q.set_info(&mut st.mem, st.scanner.cur_tok);
        p = q;
    }
    // found:
    let dr = st.scanner.def_ref.0;
    let q = st.mem.b32s1(dr);
    let av = st.mem.avail;
    st.mem.set_b32s1(dr, av);
    st.mem.avail = dr;
    if q == crate::mem::TEX_NULL {
        st.scanner.cur_val = TEMP_HEAD;
    } else {
        st.scanner.cur_val = p.0;
    }
    st.mem.set_b32s1(TEMP_HEAD, q);
    st.scanner.scanner_status = s;
    st.scanner.warning_index = w;
    st.scanner.def_ref.0 = d;
    Ok(())
}
pub(crate) fn scan_pdf_ext_toks_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_toks_in(st, false, true)?;
    Ok(())
}
pub(crate) fn compare_strings_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let save_cur_cs = st.scanner.cur_cs;
    scan_toks_in(st, false, true)?;
    let dr1 = st.scanner.def_ref.0;
    let s1 = tokens_to_string_in(st, dr1).map_err(SccAbort::TokensToString)?;
    let dr1 = st.scanner.def_ref.0;
    crate::mem::delete_token_ref(&mut st.mem, dr1);
    st.scanner.cur_cs = save_cur_cs;
    scan_toks_in(st, false, true)?;
    let dr2 = st.scanner.def_ref.0;
    let s2 = tokens_to_string_in(st, dr2).map_err(SccAbort::TokensToString)?;
    let dr2 = st.scanner.def_ref.0;
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
    st.scanner.cur_val = result;
    st.scanner.cur_val_level = INT_VAL as u8;
    Ok(())
}

// ---------------------------------------------------------------------------
// main_control_main_loop
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// main_control
// ---------------------------------------------------------------------------
