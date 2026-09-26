//! The tokenizer (`get_next`) and the scanning routines built on it.

use crate::bridge::{diag_append_str, ttstub_output_flush};
use crate::cmd::{
    ACTIVE_CHAR, ASSIGN_DIMEN, ASSIGN_FONT_DIMEN, ASSIGN_FONT_INT, ASSIGN_GLUE, ASSIGN_INT,
    ASSIGN_MU_GLUE, ASSIGN_TOKS, CALL, CAR_RET, CHAR_GIVEN, CHAR_NUM, DEF_CODE, DEF_FAMILY,
    DEF_FONT, DELIM_NUM, DONT_EXPAND, ENDV, END_TEMPLATE, FI_OR_ELSE, IF_TEST, IGNORE_SPACES,
    LAST_ITEM, LEFT_BRACE, LETTER, MAC_PARAM, MATH_CHAR_NUM, MATH_GIVEN, MAX_COMMAND, MAX_INTERNAL,
    MIN_INTERNAL, OMIT, OTHER_CHAR, OUTER_CALL, OUT_PARAM, REGISTER, RELAX, RIGHT_BRACE, SET_AUX,
    SET_BOX_DIMEN, SET_FONT, SET_PAGE_DIMEN, SET_PAGE_INT, SET_PREV_GRAF, SET_SHAPE, SPACER,
    SUP_MARK, TAB_MARK, THE, TOKS_REGISTER, VRULE, XETEX_DEF_CODE, XETEX_MATH_GIVEN,
};
use crate::consts::{
    ACTIVE_BASE, ACTIVE_MATH_CHAR, BACKED_UP, BIGGEST_CHAR, BIGGEST_USV, BOX_BASE, CAT_CODE_BASE,
    CHAR_CLASS_LIMIT, CLOSED, COUNT_BASE, CS_TOKEN_FLAG, CUR_FONT_LOC, DEL_CODE_BASE, ELSE_CODE,
    END_M_CODE, EQTB_SIZE, FIL, FILLL, FI_CODE, FONT_BASE, FONT_ID_BASE, FROZEN_CONTROL_SEQUENCE,
    FROZEN_CR, FROZEN_ENDV, FROZEN_FI, FROZEN_PRIMITIVE, FROZEN_PROTECTION, FROZEN_RELAX, INSERTED,
    INT_BASE, INT_PAR_CUR_FAM, INT_PAR_END_LINE_CHAR, INT_PAR_MAG, INT_PAR_NEW_LINE_CHAR,
    INT_PAR_TRACING_IFS, INT_PAR_TRACING_NESTING, INT_PAR_TRACING_SCAN_TOKENS,
    INT_PAR_XETEX_HYPHENATABLE_LENGTH, JUST_OPEN, LEFT_SIDE, LOCAL_BASE, LOCAL_EVERY_EOF,
    LOCAL_PAR_SHAPE, LOCAL_TECTONIC_CODA, LOCAL_XETEX_INTER_CHAR_TOKS, MATH_CODE_BASE,
    MAX_HALFWORD, MEM_TOP, MU_GLUE, MU_SKIP_BASE, NEW_LINE, NORMAL, NORMAL_SCANNER, NULL_CS,
    NUMBER_MATH_FAMILIES, OR_CODE, PRIM_EQTB_BASE, RIGHT_SIDE, SCALED_BASE, SELECTOR_LOG_ONLY,
    SELECTOR_NEW_STRING, SF_CODE_BASE, SINGLE_BASE, SKIP_BASE, SPACE_CODE, TEX_INFINITY, TEX_NULL,
    TOKEN_LIST, TOKS_BASE, TOO_BIG_CHAR, UNDEFINED_PRIMITIVE, UNLESS_CODE, VMODE,
};
use crate::errors::{error_in, int_error_in, mu_error_in};
use crate::expand::{expand_in, macro_call_in};
use crate::ext::{
    aat_font_get, aat_font_get_1, aat_font_get_2, aat_font_get_named, aat_font_get_named_1,
    gr_font_get_named, gr_font_get_named_1, ot_font_get, ot_font_get_1, ot_font_get_2,
    ot_font_get_3,
};
use crate::input::{
    back_error_in, back_input_in, begin_token_list_in, ins_error_in, set_packed_chars,
    EVERY_EOF_TEXT, LEFT_BRACE_LIMIT, MID_LINE, PARAMETER, RIGHT_BRACE_LIMIT, SKIP_BLANKS,
    TECTONIC_CODA_TEXT, V_TEMPLATE,
};
use crate::main_control::{LP_CODE_BASE, RP_CODE_BASE};
use crate::mem::{BACKUP_HEAD, OMIT_TEMPLATE, TEMP_HEAD};
use crate::node::{AnyBox, AnyNode, GlueSpecNode, NodePtr, RuleDimen, RuleNode, SaNode};
use crate::output::{
    capture_to_diagnostic_in, error_here_with_diagnostic_in, print_cmd_chr_in, print_cstr_in,
    print_esc_cstr_in, print_int_in, print_nl_cstr_in,
};
use crate::state::{EngineState, GroupCode};
use crate::units::{MemIdx, MemPtr, NodeType, Scaled};

extern "C" {
    fn usingGraphite(engine: *mut libc::c_void) -> bool;
    fn usingOpenType(engine: *mut libc::c_void) -> bool;
    fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;
}

const SKIPPING: u8 = 1;
const DEFINING: u8 = 2;
pub(crate) const MATCHING: u8 = 3;
const ALIGNING: u8 = 4;
const ABSORBING: u8 = 5;

pub(crate) const INT_VAL: u8 = 0;
pub(crate) const DIMEN_VAL: u8 = 1;
pub(crate) const GLUE_VAL: u8 = 2;
pub(crate) const MU_VAL: u8 = 3;
pub(crate) const IDENT_VAL: u8 = 4;
pub(crate) const BOX_VAL: u8 = 4;
pub(crate) const TOK_VAL: u8 = 5;
pub(crate) const INTER_CHAR_VAL: u8 = 6;
pub(crate) const MARK_VAL: u8 = 7;

pub(crate) const MAX_CHAR_VAL: i32 = 0x20_0000;
const LEFT_BRACE_TOKEN: i32 = 0x20_0000;
const RIGHT_BRACE_TOKEN: i32 = 0x40_0000;
const MATH_SHIFT_TOKEN: i32 = 0x60_0000;
const OUT_PARAM_TOKEN: i32 = 0xA0_0000;
pub(crate) const SPACE_TOKEN: i32 = 0x140_0020;
const LETTER_TOKEN: i32 = 0x160_0000;
pub(crate) const OTHER_TOKEN: i32 = 0x180_0000;
pub(crate) const MATCH_TOKEN: i32 = 0x1A0_0000;
pub(crate) const END_MATCH_TOKEN: i32 = 0x1C0_0000;
pub(crate) const PROTECTED_TOKEN: i32 = END_MATCH_TOKEN + 1;
const ZERO_TOKEN: i32 = OTHER_TOKEN + b'0' as i32;
const MINUS_TOKEN: i32 = OTHER_TOKEN + b'-' as i32;
const PLUS_TOKEN: i32 = OTHER_TOKEN + b'+' as i32;
const A_TOKEN: i32 = LETTER_TOKEN + b'A' as i32;
const OTHER_A_TOKEN: i32 = OTHER_TOKEN + b'A' as i32;
const HEX_TOKEN: i32 = OTHER_TOKEN + b'"' as i32;
const OCTAL_TOKEN: i32 = OTHER_TOKEN + b'\'' as i32;
const CONTINENTAL_POINT_TOKEN: i32 = OTHER_TOKEN + b',' as i32;
const POINT_TOKEN: i32 = OTHER_TOKEN + b'.' as i32;
const ALPHA_TOKEN: i32 = OTHER_TOKEN + b'`' as i32;
const FROZEN_PRIM_TOKEN: i32 = CS_TOKEN_FLAG + FROZEN_PRIMITIVE;

pub(crate) const NO_EXPAND_FLAG: i32 = BIGGEST_USV + 2;
const MATH_CHAR: i32 = 1;
pub(crate) const AAT_FONT_FLAG: i32 = 0xFFFF;
pub(crate) const OTGR_FONT_FLAG: i32 = 0xFFFE;
const DEFAULT_RULE: i32 = 26214;

const HYPHENATABLE_LENGTH_LIMIT: i32 = 4095;

const EXACTLY: u8 = 0;
const ADDITIONAL: u8 = 1;
const X_HEIGHT_CODE: i32 = 5;
const QUAD_CODE: i32 = 6;
const SPACE_SHRINK_CODE: i32 = 4;
const UNSET_NODE: u16 = 13;
const ETEX_VERSION: i32 = 2;
const XETEX_VERSION: i32 = 0;

const EXPR_NONE: i16 = 0;
const EXPR_ADD: i16 = 1;
const EXPR_SUB: i16 = 2;
const EXPR_MULT: i16 = 3;
const EXPR_DIV: i16 = 4;
const EXPR_SCALE: i16 = 5;
const EXPR_NODE_SIZE: i32 = 4;
const NORMAL_EXPR: u16 = 0;

pub(crate) const LAST_NODE_TYPE_CODE: i32 = 3;
pub(crate) const WIDTH_OFFSET: i32 = 1;
pub(crate) const DEPTH_OFFSET: i32 = 2;
pub(crate) const HEIGHT_OFFSET: i32 = 3;
pub(crate) const INPUT_LINE_NO_CODE: i32 = 4;
pub(crate) const BADNESS_CODE: i32 = 5;
pub(crate) const PDF_LAST_X_POS_CODE: i32 = 12;
pub(crate) const PDF_LAST_Y_POS_CODE: i32 = 13;
pub(crate) const ELAPSED_TIME_CODE: i32 = 16;
pub(crate) const PDF_SHELL_ESCAPE_CODE: i32 = 17;
pub(crate) const RANDOM_SEED_CODE: i32 = 18;
pub(crate) const ETEX_VERSION_CODE: i32 = 19;
pub(crate) const CURRENT_GROUP_LEVEL_CODE: i32 = 20;
pub(crate) const CURRENT_GROUP_TYPE_CODE: i32 = 21;
pub(crate) const CURRENT_IF_LEVEL_CODE: i32 = 22;
pub(crate) const CURRENT_IF_TYPE_CODE: i32 = 23;
pub(crate) const CURRENT_IF_BRANCH_CODE: i32 = 24;
pub(crate) const GLUE_STRETCH_ORDER_CODE: i32 = 25;
pub(crate) const GLUE_SHRINK_ORDER_CODE: i32 = 26;
pub(crate) const XETEX_VERSION_CODE: i32 = 27;
pub(crate) const XETEX_COUNT_GLYPHS_CODE: i32 = 28;
pub(crate) const XETEX_COUNT_VARIATIONS_CODE: i32 = 29;
pub(crate) const XETEX_VARIATION_CODE: i32 = 30;
pub(crate) const XETEX_FIND_VARIATION_BY_NAME_CODE: i32 = 31;
pub(crate) const XETEX_VARIATION_MIN_CODE: i32 = 32;
pub(crate) const XETEX_VARIATION_MAX_CODE: i32 = 33;
pub(crate) const XETEX_VARIATION_DEFAULT_CODE: i32 = 34;
pub(crate) const XETEX_COUNT_FEATURES_CODE: i32 = 35;
pub(crate) const XETEX_FEATURE_CODE_CODE: i32 = 36;
pub(crate) const XETEX_FIND_FEATURE_BY_NAME_CODE: i32 = 37;
pub(crate) const XETEX_IS_EXCLUSIVE_FEATURE_CODE: i32 = 38;
pub(crate) const XETEX_COUNT_SELECTORS_CODE: i32 = 39;
pub(crate) const XETEX_SELECTOR_CODE_CODE: i32 = 40;
pub(crate) const XETEX_FIND_SELECTOR_BY_NAME_CODE: i32 = 41;
pub(crate) const XETEX_IS_DEFAULT_SELECTOR_CODE: i32 = 42;
pub(crate) const XETEX_OT_COUNT_SCRIPTS_CODE: i32 = 43;
pub(crate) const XETEX_OT_COUNT_LANGUAGES_CODE: i32 = 44;
pub(crate) const XETEX_OT_COUNT_FEATURES_CODE: i32 = 45;
pub(crate) const XETEX_OT_SCRIPT_CODE: i32 = 46;
pub(crate) const XETEX_OT_LANGUAGE_CODE: i32 = 47;
pub(crate) const XETEX_OT_FEATURE_CODE: i32 = 48;
pub(crate) const XETEX_MAP_CHAR_TO_GLYPH_CODE: i32 = 49;
pub(crate) const XETEX_GLYPH_INDEX_CODE: i32 = 50;
pub(crate) const XETEX_FONT_TYPE_CODE: i32 = 51;
pub(crate) const XETEX_FIRST_CHAR_CODE: i32 = 52;
pub(crate) const XETEX_LAST_CHAR_CODE: i32 = 53;
pub(crate) const XETEX_PDF_PAGE_COUNT_CODE: i32 = 54;
pub(crate) const XETEX_DIM: i32 = 55;
pub(crate) const XETEX_GLYPH_BOUNDS_CODE: i32 = 55;
pub(crate) const FONT_CHAR_WD_CODE: i32 = 56;
pub(crate) const FONT_CHAR_HT_CODE: i32 = 57;
pub(crate) const FONT_CHAR_DP_CODE: i32 = 58;
pub(crate) const FONT_CHAR_IC_CODE: i32 = 59;
pub(crate) const PAR_SHAPE_LENGTH_CODE: i32 = 60;
pub(crate) const PAR_SHAPE_INDENT_CODE: i32 = 61;
pub(crate) const PAR_SHAPE_DIMEN_CODE: i32 = 62;
pub(crate) const GLUE_STRETCH_CODE: i32 = 63;
pub(crate) const GLUE_SHRINK_CODE: i32 = 64;
pub(crate) const ETEX_GLUE: i32 = 65;
pub(crate) const MU_TO_GLUE_CODE: i32 = 65;
pub(crate) const ETEX_MU: i32 = 66;
pub(crate) const GLUE_TO_MU_CODE: i32 = 66;
pub(crate) const ETEX_EXPR: i32 = 67;

/// Reports a bad integer `value` with two help lines.
fn bad_int_error(
    st: &mut EngineState,
    err: *const libc::c_char,
    value: i32,
    help1: *const libc::c_char,
    help0: &std::ffi::CStr,
) -> Result<(), SccAbort> {
    let errmsg = error_here_with_diagnostic_in(st, err);
    unsafe { diag_append_str(errmsg, &format!(" ({value})")) };
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 2;
    st.errors.help_line[1] = help1;
    st.errors.help_line[0] = help0.as_ptr();
    scc_err(int_error_in(st, value))
}

fn int_range_error(
    st: &mut EngineState,
    err: *const libc::c_char,
    help1: *const libc::c_char,
) -> Result<(), SccAbort> {
    bad_int_error(
        st,
        err,
        st.scanner.cur_val,
        help1,
        c"I changed this one to zero.",
    )?;
    st.scanner.cur_val = 0;
    Ok(())
}

fn scan_int_up_to_in(
    st: &mut EngineState,
    max: i32,
    err: &std::ffi::CStr,
    help: &std::ffi::CStr,
) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > max {
        int_range_error(st, err.as_ptr(), help.as_ptr())?;
    }
    Ok(())
}

pub(crate) fn scan_char_class_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_up_to_in(
        st,
        CHAR_CLASS_LIMIT,
        c"Bad character class",
        c"A character class must be between 0 and 4096.",
    )
}

pub(crate) fn scan_char_class_not_ignored_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_up_to_in(
        st,
        CHAR_CLASS_LIMIT,
        c"Bad character class",
        c"A class for inter-character transitions must be between 0 and 4095.",
    )
}

pub(crate) fn scan_eight_bit_int_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_up_to_in(
        st,
        255,
        c"Bad register code",
        c"A register code or char class must be between 0 and 255.",
    )
}

pub(crate) fn scan_math_class_int_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_up_to_in(
        st,
        7,
        c"Bad math class",
        c"Since I expected to read a number between 0 and 7,",
    )
}

pub(crate) fn scan_math_fam_int_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_up_to_in(
        st,
        NUMBER_MATH_FAMILIES - 1,
        c"Bad math family",
        c"Since I expected to read a number between 0 and 255,",
    )
}

pub(crate) fn scan_four_bit_int_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_up_to_in(
        st,
        15,
        c"Bad number",
        c"Since I expected to read a number between 0 and 15,",
    )
}

pub(crate) fn scan_four_bit_int_or_18_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || (st.scanner.cur_val > 15 && st.scanner.cur_val != 18) {
        int_range_error(
            st,
            c"Bad number".as_ptr(),
            c"Since I expected to read a number between 0 and 15,".as_ptr(),
        )?;
    }
    Ok(())
}

pub(crate) fn scan_fifteen_bit_int_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_up_to_in(
        st,
        32767,
        c"Bad mathchar",
        c"A mathchar number must be between 0 and 32767.",
    )
}

pub(crate) fn scan_char_num_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_up_to_in(
        st,
        BIGGEST_CHAR,
        c"Bad character code",
        c"A character number must be between 0 and 65535.",
    )
}

pub(crate) fn scan_usv_num_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_up_to_in(
        st,
        BIGGEST_USV,
        c"Bad character code",
        c"A Unicode scalar value must be between 0 and \"10FFFF.",
    )
}

pub(crate) fn scan_delimiter_int_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_up_to_in(
        st,
        0x7FF_FFFF,
        c"Bad delimiter code",
        c"A numeric delimiter code must be between 0 and 2^{27}-1.",
    )
}

pub(crate) fn scan_register_num_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > st.scanner.max_reg_num {
        let help = st.scanner.max_reg_help_line;
        int_range_error(st, c"Bad register code".as_ptr(), help)?;
    }
    Ok(())
}

pub(crate) fn scan_xetex_math_char_int_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    if math_char_fn(st.scanner.cur_val) == ACTIVE_MATH_CHAR {
        if st.scanner.cur_val != ACTIVE_MATH_CHAR {
            bad_int_error(
                st,
                c"Bad active XeTeX math code".as_ptr(),
                st.scanner.cur_val,
                c"Since I ignore class and family for active math chars,".as_ptr(),
                c"I changed this one to \"1FFFFF.",
            )?;
            st.scanner.cur_val = ACTIVE_MATH_CHAR;
        }
    } else if math_char_fn(st.scanner.cur_val) > BIGGEST_USV {
        int_range_error(
            st,
            c"Bad XeTeX math character code".as_ptr(),
            c"Since I expected a character number between 0 and \"10FFFF,".as_ptr(),
        )?;
    }
    Ok(())
}

/// `\XeTeXhyphenatablelength`, capped at the supported maximum.
pub fn max_hyphenatable_length_in(eqtb: &crate::state::EqtbState) -> i32 {
    eqtb.b32s1(INT_BASE + INT_PAR_XETEX_HYPHENATABLE_LENGTH)
        .min(HYPHENATABLE_LENGTH_LIMIT)
}

pub(crate) fn insert_dollar_sign_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    back_input_in(st)?;
    st.scanner.cur_tok = MATH_SHIFT_TOKEN + '$' as i32;
    error_here_with_diagnostic_in(st, c"Missing $ inserted".as_ptr());
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 2;
    st.errors.help_line[1] = c"I've inserted a begin-math/end-math symbol since I think".as_ptr();
    st.errors.help_line[0] = c"you left one out. Proceed, with fingers crossed.".as_ptr();
    crate::main_control::mc_propagate_error(ins_error_in(st)?)
}

pub(crate) fn scan_optional_equals_in(st: &mut EngineState) -> Result<(), SccAbort> {
    get_x_non_blank_in(st)?;
    if st.scanner.cur_tok != OTHER_TOKEN + '=' as i32 {
        back_input_in(st)?;
    }
    Ok(())
}

/// Whether the macro whose token list starts at `ref_count` is `\protected`.
#[inline]
pub(crate) fn is_protected_macro(mem: &crate::state::MemState, ref_count: i32) -> bool {
    mem.b32s0(mem.b32s1(ref_count)) == PROTECTED_TOKEN
}

pub(crate) fn get_x_or_protected_in(st: &mut EngineState) -> Result<(), SccAbort> {
    loop {
        get_token_in(st)?;
        if st.scanner.cur_cmd <= MAX_COMMAND {
            return Ok(());
        }
        if st.scanner.cur_cmd >= CALL
            && st.scanner.cur_cmd < END_TEMPLATE
            && is_protected_macro(&st.mem, st.scanner.cur_chr)
        {
            return Ok(());
        }
        expand_in(st)?;
    }
}

pub(crate) fn scan_spec_in(
    st: &mut EngineState,
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
    if three_codes {
        let sp = st.save_stack.save_ptr;
        st.save_stack.set_b32s1(sp, s);
        st.save_stack.save_ptr = sp + 1;
    }
    let sp = st.save_stack.save_ptr;
    st.save_stack.set_b32s1(sp, spec_code as i32);
    st.save_stack.set_b32s1(sp + 1, st.scanner.cur_val);
    st.save_stack.save_ptr = sp + 2;
    if let Err(abort) = crate::save::new_save_level(&mut st.save_stack, c, st.mem.line) {
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

fn round_decimals(errors: &crate::state::ErrorState, k: i32) -> i32 {
    let a = errors.dig[..k as usize]
        .iter()
        .rev()
        .fold(0, |a, &d| (a + d as i32 * 0x2_0000) / 10);
    (a + 1) / 2
}

pub(crate) fn scan_rule_spec_in(st: &mut EngineState) -> Result<NodePtr<RuleNode>, SccAbort> {
    let r = crate::mem::alloc_rule(&mut st.mem).ok_or_else(main_mem_overflow_scc)?;
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
    Ok(r)
}

pub(crate) enum StrToksAbort {
    PoolOverflow,
    MainMemoryOverflow,
}

/// Converts `str_pool[b..pool_ptr]` to a token list at `link(temp_head)` and returns its tail.
pub(crate) fn str_toks_cat_in(
    mem: &mut crate::state::MemState,
    strings: &mut crate::state::StringsState,
    b: i32,
    cat: i16,
) -> Result<MemPtr, StrToksAbort> {
    if strings.pool_ptr + 1 > strings.pool_size {
        return Err(StrToksAbort::PoolOverflow);
    }

    let mut p = TEMP_HEAD;
    p.set_link(mem, MemPtr::NULL);
    let mut k = b;

    while k < strings.pool_ptr {
        let mut t = strings.str_pool[k as usize] as i32;

        if t == b' ' as i32 && cat == 0 {
            t = SPACE_TOKEN;
        } else {
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

/// Appends `\endlinechar` to the line just read and starts reading it.
#[inline]
pub(crate) fn finish_line_in(st: &mut EngineState) {
    let end_char = st.eqtb.b32s1(INT_BASE + INT_PAR_END_LINE_CHAR);
    if (0..=255).contains(&end_char) {
        st.input.buffer[st.input.cur_input.limit as usize] = end_char;
    } else {
        st.input.cur_input.limit -= 1;
    }
    st.input.first = st.input.cur_input.limit + 1;
    st.input.cur_input.loc = st.input.cur_input.start;
}

fn close_read_file_in(st: &mut EngineState, m: usize) -> Result<(), SccAbort> {
    let slot = std::mem::take(&mut st.input.read_file[m]);
    // SAFETY: closes the handle taken out of the read-file slot.
    if unsafe { crate::io::u_close(slot) } {
        return Err(SccAbort::InputClose);
    }
    st.input.read_open[m] = CLOSED;
    Ok(())
}

pub(crate) fn read_toks_in(st: &mut EngineState, n: i32, r: i32, j: i32) -> Result<(), SccAbort> {
    st.scanner.scanner_status = DEFINING;
    st.scanner.warning_index = r;
    let def_ref = new_def_ref_in(st)?;
    let mut p = def_ref;
    store_new_token(st, &mut p, END_MATCH_TOKEN)?;

    let m = if (0..=15).contains(&n) {
        n as usize
    } else {
        16
    };
    let s = st.scanner.align_state;
    st.scanner.align_state = 1_000_000;

    loop {
        crate::input::begin_file_reading_in(st)?;
        st.input.cur_input.name = (m + 1) as i32;

        if st.input.read_open[m] == CLOSED {
            return Err(SccAbort::Input(crate::input::InputAbort::TtAbort(
                c"terminal input forbidden".as_ptr(),
            )));
        } else if st.input.read_open[m] == JUST_OPEN {
            let rf = st.input.read_file_ptr(m);
            if unsafe { crate::io::input_line_in(st, rf)? } != 0 {
                st.input.read_open[m] = NORMAL as u8;
            } else {
                close_read_file_in(st, m)?;
            }
        } else {
            let rf = st.input.read_file_ptr(m);
            if unsafe { crate::io::input_line_in(st, rf)? } == 0 {
                close_read_file_in(st, m)?;
                if st.scanner.align_state != 1_000_000 {
                    crate::errors::runaway_in(st);
                    error_here_with_diagnostic_in(st, c"File ended within ".as_ptr());
                    print_esc_cstr_in(st, c"read".as_ptr());
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 1;
                    st.errors.help_line[0] = c"This \\read has unbalanced braces.".as_ptr();
                    st.scanner.align_state = 1_000_000;
                    st.input.cur_input.limit = 0;
                    scc_err(error_in(st))?;
                }
            }
        }

        st.input.cur_input.limit = st.input.last;
        finish_line_in(st);
        st.input.cur_input.state = NEW_LINE;

        if j == 1 {
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
                store_new_token(st, &mut p, tok)?;
            }
        } else {
            loop {
                get_token_in(st)?;
                if st.scanner.cur_tok == 0 {
                    break;
                }
                if st.scanner.align_state < 1_000_000 {
                    loop {
                        get_token_in(st)?;
                        if st.scanner.cur_tok == 0 {
                            break;
                        }
                    }
                    st.scanner.align_state = 1_000_000;
                    break;
                }
                store_new_token(st, &mut p, st.scanner.cur_tok)?;
            }
        }
        crate::input::end_file_reading_in(st)?;

        if st.scanner.align_state == 1_000_000 {
            break;
        }
    }

    st.scanner.cur_val = def_ref.0;
    st.scanner.scanner_status = NORMAL_SCANNER;
    st.scanner.align_state = s;
    Ok(())
}

#[allow(non_snake_case)]
pub(crate) fn eTeX_enabled_in(
    st: &mut EngineState,
    b: bool,
    j: u8,
    k: i32,
) -> Result<bool, crate::main_control::MainControlAbort> {
    if !b {
        error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
        print_cmd_chr_in(st, j, k);
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 1;
        st.errors.help_line[0] = c"Sorry, this optional e-TeX feature has been disabled.".as_ptr();
        crate::main_control::mc_propagate_error(error_in(st))?;
    }
    Ok(b)
}

pub(crate) enum TokensToStringAbort {
    SelectorNewString,
    StringOverflow,
}

pub(crate) fn tokens_to_string_in(
    st: &mut EngineState,
    p: i32,
) -> Result<i32, TokensToStringAbort> {
    if st.shipout.selector == SELECTOR_NEW_STRING {
        return Err(TokensToStringAbort::SelectorNewString);
    }
    st.shipout.old_setting = st.shipout.selector as u8;
    st.shipout.selector = SELECTOR_NEW_STRING;
    let arg = st.mem.b32s1(p);
    let l = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list_in(st, arg, TEX_NULL, l);
    st.shipout.selector = st.shipout.old_setting as i32;
    crate::string_pool::make_string(&mut st.strings)
        .map_err(|_| TokensToStringAbort::StringOverflow)
}

pub(crate) fn get_r_token_in(st: &mut EngineState) -> Result<(), SccAbort> {
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
                back_input_in(st)?;
            }
            st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_PROTECTION;
            scc_err(ins_error_in(st)?)?;
        } else {
            return Ok(());
        }
    }
}

/// Converts a TeX `\mathcode` value to XeTeX's extended layout.
fn math_code_from_tex(v: i32) -> i32 {
    set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256)
}

/// Scans the class, family and character of a `\Umathchar`-style code.
fn scan_umath_code_in(st: &mut EngineState) -> Result<i32, SccAbort> {
    scan_math_class_int_in(st)?;
    let class = set_class_fn(st.scanner.cur_val);
    scan_math_fam_int_in(st)?;
    let family = set_family_fn(st.scanner.cur_val);
    scan_usv_num_in(st)?;
    Ok(class + family + st.scanner.cur_val)
}

/// `get_x_token` until the result is neither a space nor `\relax`.
fn get_x_non_blank_non_relax_in(st: &mut EngineState) -> Result<(), SccAbort> {
    loop {
        get_x_token_in(st)?;
        if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
            return Ok(());
        }
    }
}

/// Scans one math field into the nucleus-like field `p`.
pub(crate) fn scan_math_in(
    st: &mut EngineState,
    p: i32,
) -> Result<(), crate::main_control::MainControlAbort> {
    let c: i32;
    'main: loop {
        get_x_non_blank_non_relax_in(st)?;
        c = loop {
            break match st.scanner.cur_cmd {
                LETTER | OTHER_CHAR | CHAR_GIVEN => {
                    let mc = st.eqtb.b32s1(MATH_CODE_BASE + st.scanner.cur_chr);
                    if mc & 0x1FFFFF == ACTIVE_MATH_CHAR {
                        st.scanner.cur_cs = st.scanner.cur_chr + 1;
                        fetch_cs_meaning(st);
                        x_token_in(st)?;
                        back_input_in(st)?;
                        continue 'main;
                    }
                    mc
                }
                CHAR_NUM => {
                    scan_char_num_in(st)?;
                    st.scanner.cur_chr = st.scanner.cur_val;
                    st.scanner.cur_cmd = CHAR_GIVEN;
                    continue;
                }
                MATH_CHAR_NUM => match st.scanner.cur_chr {
                    2 => scan_umath_code_in(st)?,
                    1 => {
                        scan_xetex_math_char_int_in(st)?;
                        st.scanner.cur_val
                    }
                    _ => {
                        scan_fifteen_bit_int_in(st)?;
                        math_code_from_tex(st.scanner.cur_val)
                    }
                },
                MATH_GIVEN => math_code_from_tex(st.scanner.cur_chr),
                XETEX_MATH_GIVEN => st.scanner.cur_chr,
                DELIM_NUM => {
                    if st.scanner.cur_chr == 1 {
                        scan_umath_code_in(st)?
                    } else {
                        scan_delimiter_int_in(st)?;
                        math_code_from_tex(st.scanner.cur_val / 4096)
                    }
                }
                _ => {
                    back_input_in(st)?;
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
    st.mem.set_b32s1(p, MATH_CHAR);
    st.mem.set_b16s0(p, (c % 65536) as u16);
    let cur_fam = st.eqtb.b32s1(INT_BASE + INT_PAR_CUR_FAM);
    let fam = if math_class_fn(c) == 7 && (0..NUMBER_MATH_FAMILIES).contains(&cur_fam) {
        cur_fam as u16
    } else {
        math_fam_fn(c) as u16
    };
    st.mem
        .set_b16s1(p, fam + (math_char_fn(c) / 65536) as u16 * 256);
    Ok(())
}

pub(crate) fn set_math_char_in(
    st: &mut EngineState,
    c: i32,
) -> Result<(), crate::main_control::MainControlAbort> {
    if math_char_fn(c) == ACTIVE_MATH_CHAR {
        st.scanner.cur_cs = st.scanner.cur_chr + 1;
        fetch_cs_meaning(st);
        x_token_in(st)?;
        back_input_in(st)?;
    } else {
        let nd = crate::mem::alloc_noad(&mut st.mem).expect("new noad");
        nd.set_nucleus_type(&mut st.mem, MATH_CHAR);
        let ch = math_char_fn(c);
        nd.set_nucleus_char(&mut st.mem, (ch % 65536) as u16);
        nd.set_nucleus_fam(&mut st.mem, math_fam_fn(c) as u16);
        if math_class_fn(c) == 7 {
            let cf = st.eqtb.b32s1(INT_BASE + INT_PAR_CUR_FAM);
            if (0..NUMBER_MATH_FAMILIES).contains(&cf) {
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
        st.input.cur_list.tail.set_link(&mut st.mem, nd.ptr());
        st.input.cur_list.tail = nd.ptr();
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

pub(crate) enum NodeRecycleAbort {
    Overflow,
    OverflowRunaway,
    Confusion(&'static std::ffi::CStr),
    NativeFont(crate::ext::NativeFontAbort),
}

impl From<crate::ext::NativeFontAbort> for NodeRecycleAbort {
    fn from(e: crate::ext::NativeFontAbort) -> Self {
        NodeRecycleAbort::NativeFont(e)
    }
}

impl NodeRecycleAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            NodeRecycleAbort::Overflow => crate::errors::EngineAbort::Overflow {
                what: c"main memory size".as_ptr(),
                n: MEM_TOP + 1,
            },
            NodeRecycleAbort::OverflowRunaway => crate::mem::main_memory_overflow_abort(),
            NodeRecycleAbort::Confusion(msg) => crate::errors::EngineAbort::Confusion(msg.as_ptr()),
            NodeRecycleAbort::NativeFont(e) => e.into_engine_abort(),
        }
    }
}

pub(crate) fn check_outer_validity_in(
    st: &mut EngineState,
) -> Result<crate::errors::ErrorOutcome, crate::input::InputAbort> {
    use crate::errors::ErrorOutcome;
    use crate::input::InputAbort;

    if st.scanner.scanner_status == NORMAL_SCANNER {
        return Ok(ErrorOutcome::Continue);
    }
    st.errors.deletions_allowed = false;

    if st.scanner.cur_cs != 0 {
        let ci = st.input.cur_input;
        if ci.state == TOKEN_LIST || ci.name < 1 || ci.name > 17 {
            let p = crate::mem::get_avail(&mut st.mem).ok_or(InputAbort::MainMemory)?;
            p.set_info(&mut st.mem, CS_TOKEN_FLAG + st.scanner.cur_cs);
            begin_token_list_in(st, p.0, BACKED_UP)?;
        }
        st.scanner.cur_cmd = SPACER;
        st.scanner.cur_chr = ' ' as i32;
    }

    let oc = if st.scanner.scanner_status > SKIPPING {
        crate::errors::runaway_in(st);
        if st.scanner.cur_cs == 0 {
            error_here_with_diagnostic_in(st, c"File ended".as_ptr());
        } else {
            st.scanner.cur_cs = 0;
            error_here_with_diagnostic_in(st, c"Forbidden control sequence found".as_ptr());
        }

        let p = crate::mem::get_avail(&mut st.mem).ok_or(InputAbort::MainMemory)?;
        let mut inserted = p;
        match st.scanner.scanner_status {
            DEFINING => {
                print_cstr_in(st, c" while scanning definition".as_ptr());
                p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
            }
            MATCHING => {
                print_cstr_in(st, c" while scanning use".as_ptr());
                p.set_info(&mut st.mem, st.input.par_token);
                st.scanner.long_state = OUTER_CALL;
            }
            ALIGNING => {
                print_cstr_in(st, c" while scanning preamble".as_ptr());
                p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
                inserted = crate::mem::get_avail(&mut st.mem).ok_or(InputAbort::MainMemory)?;
                inserted.set_link(&mut st.mem, p);
                inserted.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_CR);
                st.scanner.align_state = -1_000_000;
            }
            ABSORBING => {
                print_cstr_in(st, c" while scanning text".as_ptr());
                p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
            }
            _ => {}
        }

        begin_token_list_in(st, inserted.0, INSERTED)?;
        print_cstr_in(st, c" of ".as_ptr());
        let warning_index = st.scanner.warning_index;
        crate::output::sprint_cs_in(st, warning_index);
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 4;
        st.errors.help_line[3] = c"I suspect you have forgotten a `}', causing me".as_ptr();
        st.errors.help_line[2] = c"to read past where you wanted me to stop.".as_ptr();
        st.errors.help_line[1] = c"I'll try to recover; but if the error is serious,".as_ptr();
        st.errors.help_line[0] = c"you'd better type `E' or `X' now and fix your file.".as_ptr();
        error_in(st)
    } else {
        error_here_with_diagnostic_in(st, c"Incomplete ".as_ptr());
        let cur_if = st.scanner.cur_if;
        print_cmd_chr_in(st, IF_TEST, cur_if as i32);
        print_cstr_in(st, c"; all text was ignored after line ".as_ptr());
        let skip_line = st.scanner.skip_line;
        print_int_in(st, skip_line);
        capture_to_diagnostic_in(st, std::ptr::null_mut());
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
        st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_FI;
        ins_error_in(st)?
    };
    st.errors.deletions_allowed = true;
    Ok(oc)
}

pub(crate) enum GetNextOutcome {
    Done,
    Error(crate::errors::ErrorOutcome),
    Abort(crate::input::InputAbort),
    /// The input ended without `\end`.
    NoLegalEnd {
        open_log: bool,
    },
}

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

/// Sets `cur_cmd` and `cur_chr` to the meaning of `cur_cs`.
#[inline(always)]
fn fetch_cs_meaning(st: &mut EngineState) {
    st.scanner.cur_cmd = st.eqtb.eq_type(st.scanner.cur_cs);
    st.scanner.cur_chr = st.eqtb.b32s1(st.scanner.cur_cs);
}

#[inline(always)]
fn is_lc_hex(c: i32) -> bool {
    (0x30..=0x39).contains(&c) || (0x61..=0x66).contains(&c)
}

#[inline]
fn hex_digit_value(c: i32) -> i32 {
    if c <= b'9' as i32 {
        c - b'0' as i32
    } else {
        c - b'a' as i32 + 10
    }
}

/// Reduces a `^^` sequence at `buffer[k - 1..]` inside a control-sequence name.
/// Returns whether the buffer changed, in which case the name must be rescanned.
#[cold]
fn reduce_cs_sup_mark(input: &mut crate::state::InputClusterState, k: i32, chr: i32) -> bool {
    let limit = input.cur_input.limit;
    let buf = &mut input.buffer;
    let mut sup_count: i32 = 2;
    while sup_count < 6
        && k + 2 * sup_count - 2 <= limit
        && buf[(k + sup_count - 1) as usize] == chr
    {
        sup_count += 1;
    }
    let digits = (k + sup_count - 1)..(k + 2 * sup_count - 1);
    let (new_chr, shift) = if digits.clone().all(|i| is_lc_hex(buf[i as usize])) {
        let new_chr = digits.fold(0, |v, i| 16 * v + hex_digit_value(buf[i as usize]));
        if new_chr > BIGGEST_USV {
            return false;
        }
        (new_chr, 2 * sup_count - 1)
    } else {
        let c = buf[(k + 1) as usize];
        if c >= 128 {
            return false;
        }
        (if c < 64 { c + 64 } else { c - 64 }, 2)
    };
    buf[(k - 1) as usize] = new_chr;
    input.cur_input.limit -= shift;
    let mut i = k;
    while i <= input.cur_input.limit {
        input.buffer[i as usize] = input.buffer[(i + shift) as usize];
        i += 1;
    }
    true
}

/// The central tokenizer.
pub(crate) fn get_next_in(st: &mut EngineState) -> GetNextOutcome {
    st.scanner.logical_clock = st.scanner.logical_clock.wrapping_add(1);

    macro_rules! try_in {
        ($e:expr) => {
            match $e {
                Ok(v) => v,
                Err(a) => return GetNextOutcome::Abort(a),
            }
        };
    }

    macro_rules! check_outer_validity {
        () => {
            match check_outer_validity_in(st) {
                Ok(crate::errors::ErrorOutcome::Continue) => {}
                Ok(oc) => return GetNextOutcome::Error(oc),
                Err(a) => return GetNextOutcome::Abort(a),
            }
        };
    }

    macro_rules! id_lookup {
        ($j:expr, $l:expr) => {
            match crate::hash::id_lookup(&mut st.eqtb, &mut st.strings, &st.input.buffer, $j, $l) {
                Ok(p) => p,
                Err(e) => return GetNextOutcome::Abort(hash_err_abort(e)),
            }
        };
    }

    'restart: loop {
        st.scanner.cur_cs = 0;

        if st.input.cur_input.state != TOKEN_LIST {
            'texswitch: loop {
                if st.input.cur_input.loc <= st.input.cur_input.limit {
                    let loc = st.input.cur_input.loc;
                    st.scanner.cur_chr = st.input.buffer[loc as usize];
                    st.input.cur_input.loc = loc + 1;

                    if (0xD800..0xDC00).contains(&st.scanner.cur_chr)
                        && loc + 1 <= st.input.cur_input.limit
                        && (0xDC00..0xE000).contains(&st.input.buffer[(loc + 1) as usize])
                    {
                        let lower = st.input.buffer[(loc + 1) as usize] - 0xDC00;
                        st.input.cur_input.loc = loc + 2;
                        st.scanner.cur_chr = 65536 + (st.scanner.cur_chr - 0xD800) * 1024 + lower;
                    }

                    'reswitch: loop {
                        let cat = st.eqtb.b32s1(CAT_CODE_BASE + st.scanner.cur_chr) as u8;
                        st.scanner.cur_cmd = cat;

                        // Dispatch on state + category (states 1, 17, 33; categories 0..=15).
                        match st.input.cur_input.state as i32 + cat as i32 {
                            // Ignored characters, and spaces when skipping blanks.
                            10 | 26 | 42 | 27 | 43 => continue 'texswitch,

                            // Escape: scan a control sequence name.
                            1 | 17 | 33 => {
                                if st.input.cur_input.loc > st.input.cur_input.limit {
                                    st.scanner.cur_cs = NULL_CS;
                                } else {
                                    'start_cs: loop {
                                        let start = st.input.cur_input.loc;
                                        let first_chr = st.input.buffer[start as usize];
                                        let first_cat =
                                            st.eqtb.b32s1(CAT_CODE_BASE + first_chr) as u8;
                                        let mut k = start + 1;
                                        st.input.cur_input.state =
                                            if first_cat == LETTER || first_cat == SPACER {
                                                SKIP_BLANKS
                                            } else {
                                                MID_LINE
                                            };

                                        if first_cat == LETTER && k <= st.input.cur_input.limit {
                                            let mut chr;
                                            let mut cat;
                                            loop {
                                                chr = st.input.buffer[k as usize];
                                                cat = st.eqtb.b32s1(CAT_CODE_BASE + chr) as u8;
                                                k += 1;
                                                if cat != LETTER || k > st.input.cur_input.limit {
                                                    break;
                                                }
                                            }
                                            if cat == SUP_MARK
                                                && st.input.buffer[k as usize] == chr
                                                && k < st.input.cur_input.limit
                                                && reduce_cs_sup_mark(&mut st.input, k, chr)
                                            {
                                                continue 'start_cs;
                                            }
                                            if cat != LETTER {
                                                k -= 1;
                                            }
                                            if k > st.input.cur_input.loc + 1 {
                                                let loc = st.input.cur_input.loc;
                                                st.scanner.cur_cs = id_lookup!(loc, k - loc);
                                                st.input.cur_input.loc = k;
                                                break 'start_cs;
                                            }
                                        } else if first_cat == SUP_MARK
                                            && st.input.buffer[k as usize] == first_chr
                                            && k < st.input.cur_input.limit
                                            && reduce_cs_sup_mark(&mut st.input, k, first_chr)
                                        {
                                            continue 'start_cs;
                                        }

                                        let loc = st.input.cur_input.loc;
                                        if st.input.buffer[loc as usize] > 65535 {
                                            st.scanner.cur_cs = id_lookup!(loc, 1);
                                        } else {
                                            st.scanner.cur_cs =
                                                SINGLE_BASE + st.input.buffer[loc as usize];
                                        }
                                        st.input.cur_input.loc = loc + 1;
                                        break 'start_cs;
                                    }
                                }
                                fetch_cs_meaning(st);
                                if st.scanner.cur_cmd >= OUTER_CALL {
                                    check_outer_validity!();
                                }
                            }

                            // Active character.
                            14 | 30 | 46 => {
                                st.scanner.cur_cs = st.scanner.cur_chr + 1;
                                fetch_cs_meaning(st);
                                st.input.cur_input.state = MID_LINE;
                                if st.scanner.cur_cmd >= OUTER_CALL {
                                    check_outer_validity!();
                                }
                            }

                            // Superscript: maybe a `^^` notation.
                            8 | 24 | 40 => {
                                let loc = st.input.cur_input.loc;
                                let limit = st.input.cur_input.limit;
                                let buf = &st.input.buffer;
                                if st.scanner.cur_chr == buf[loc as usize] && loc < limit {
                                    let mut sup_count: i32 = 2;
                                    while sup_count < 6
                                        && loc + 2 * sup_count - 2 <= limit
                                        && st.scanner.cur_chr == buf[(loc + sup_count - 1) as usize]
                                    {
                                        sup_count += 1;
                                    }
                                    let digits = (loc + sup_count - 1)..(loc + 2 * sup_count - 1);
                                    if digits.clone().all(|i| is_lc_hex(buf[i as usize])) {
                                        let new_chr = digits.fold(0, |v, i| {
                                            16 * v + hex_digit_value(buf[i as usize])
                                        });
                                        if new_chr <= BIGGEST_USV {
                                            st.scanner.cur_chr = new_chr;
                                            st.input.cur_input.loc = loc + 2 * sup_count - 1;
                                            continue 'reswitch;
                                        }
                                        st.scanner.cur_chr = buf[loc as usize];
                                    } else {
                                        let c = buf[(loc + 1) as usize];
                                        if c < 128 {
                                            st.input.cur_input.loc = loc + 2;
                                            st.scanner.cur_chr =
                                                if c < 64 { c + 64 } else { c - 64 };
                                            continue 'reswitch;
                                        }
                                    }
                                }
                                st.input.cur_input.state = MID_LINE;
                            }

                            // Invalid character.
                            16 | 32 | 48 => {
                                error_here_with_diagnostic_in(
                                    st,
                                    c"Text line contains an invalid character".as_ptr(),
                                );
                                capture_to_diagnostic_in(st, std::ptr::null_mut());
                                st.errors.help_ptr = 2;
                                st.errors.help_line[1] =
                                    c"A funny symbol that I can't read has just been input."
                                        .as_ptr();
                                st.errors.help_line[0] =
                                    c"Continue, and I'll forget that it ever happened.".as_ptr();
                                st.errors.deletions_allowed = false;
                                let oc = error_in(st);
                                if !matches!(oc, crate::errors::ErrorOutcome::Continue) {
                                    return GetNextOutcome::Error(oc);
                                }
                                st.errors.deletions_allowed = true;
                                continue 'restart;
                            }

                            // Space in mid-line.
                            11 => {
                                st.input.cur_input.state = SKIP_BLANKS;
                                st.scanner.cur_chr = b' ' as i32;
                            }

                            // End of line in mid-line.
                            6 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                st.scanner.cur_cmd = SPACER;
                                st.scanner.cur_chr = b' ' as i32;
                            }

                            // Comments, and end of line when skipping blanks.
                            15 | 31 | 47 | 22 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                continue 'texswitch;
                            }

                            // End of an empty line: `\par`.
                            38 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                st.scanner.cur_cs = st.input.par_loc;
                                fetch_cs_meaning(st);
                                if st.scanner.cur_cmd >= OUTER_CALL {
                                    check_outer_validity!();
                                }
                            }

                            2 => st.scanner.align_state += 1,
                            18 | 34 => {
                                st.input.cur_input.state = MID_LINE;
                                st.scanner.align_state += 1;
                            }
                            3 => st.scanner.align_state -= 1,
                            19 | 35 => {
                                st.input.cur_input.state = MID_LINE;
                                st.scanner.align_state -= 1;
                            }

                            // Other delimiters after blanks or at line start.
                            20 | 21 | 23 | 25 | 28 | 29 | 36 | 37 | 39 | 41 | 44 | 45 => {
                                st.input.cur_input.state = MID_LINE;
                            }

                            _ => {}
                        }
                        break 'reswitch;
                    }
                    break 'texswitch;
                }

                st.input.cur_input.state = NEW_LINE;

                if st.input.cur_input.name > 17 {
                    st.mem.line += 1;
                    st.input.first = st.input.cur_input.start;

                    if !st.input.force_eof {
                        let got_line = if st.input.cur_input.name <= 19 {
                            try_in!(crate::input::pseudo_input_in(st))
                        } else {
                            let f = st.input.input_file_ptr(st.input.cur_input.index as i32);
                            try_in!(unsafe { crate::io::input_line_in(st, f) }) != 0
                        };
                        if got_line {
                            st.input.cur_input.limit = st.input.last;
                        } else {
                            let index = st.input.cur_input.index as usize;
                            let every_eof = st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_EOF);
                            if every_eof != TEX_NULL && !st.input.eof_seen[index] {
                                st.input.cur_input.limit = st.input.first - 1;
                                st.input.eof_seen[index] = true;
                                try_in!(begin_token_list_in(st, every_eof, EVERY_EOF_TEXT));
                                continue 'restart;
                            }
                            st.input.force_eof = true;
                        }
                    }

                    if st.input.force_eof {
                        let in_open = st.input.in_open as usize;
                        if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_NESTING) > 0
                            && (st.input.grp_stack[in_open] != st.save_stack.cur_boundary
                                || st.input.if_stack[in_open] != st.scanner.cond_ptr)
                        {
                            crate::output::file_warning_in(st);
                        }
                        if st.input.cur_input.name >= 19 {
                            crate::output::print_char_in(st, b')' as i32);
                            st.input.open_parens -= 1;
                            unsafe { ttstub_output_flush(st.shipout.rust_stdout) };
                        }
                        st.input.force_eof = false;
                        try_in!(crate::input::end_file_reading_in(st));
                        check_outer_validity!();
                        continue 'restart;
                    }

                    finish_line_in(st);
                } else {
                    if st.input.cur_input.name != 0 {
                        st.scanner.cur_cmd = 0;
                        st.scanner.cur_chr = 0;
                        return GetNextOutcome::Done;
                    }
                    if st.input.input_ptr > 0 {
                        try_in!(crate::input::end_file_reading_in(st));
                        continue 'restart;
                    }
                    let coda = st.eqtb.b32s1(LOCAL_BASE + LOCAL_TECTONIC_CODA);
                    if !st.scanner.used_tectonic_coda_tokens && coda != TEX_NULL {
                        st.scanner.used_tectonic_coda_tokens = true;
                        try_in!(begin_token_list_in(st, coda, TECTONIC_CODA_TEXT));
                        continue 'restart;
                    }
                    return GetNextOutcome::NoLegalEnd {
                        open_log: st.shipout.selector < SELECTOR_LOG_ONLY,
                    };
                }
            }
        } else if st.input.cur_input.loc != TEX_NULL {
            let t = st.mem.b32s0(st.input.cur_input.loc);
            st.input.cur_input.loc = st.mem.b32s1(st.input.cur_input.loc);

            if t >= CS_TOKEN_FLAG {
                st.scanner.cur_cs = t - CS_TOKEN_FLAG;
                fetch_cs_meaning(st);
                if st.scanner.cur_cmd >= OUTER_CALL {
                    if st.scanner.cur_cmd == DONT_EXPAND {
                        st.scanner.cur_cs = st.mem.b32s0(st.input.cur_input.loc) - CS_TOKEN_FLAG;
                        st.input.cur_input.loc = TEX_NULL;
                        fetch_cs_meaning(st);
                        if st.scanner.cur_cmd > MAX_COMMAND {
                            st.scanner.cur_cmd = RELAX;
                            st.scanner.cur_chr = NO_EXPAND_FLAG;
                        }
                    } else {
                        check_outer_validity!();
                    }
                }
            } else {
                st.scanner.cur_cmd = (t / MAX_CHAR_VAL) as u8;
                st.scanner.cur_chr = t % MAX_CHAR_VAL;
                match st.scanner.cur_cmd {
                    LEFT_BRACE => st.scanner.align_state += 1,
                    RIGHT_BRACE => st.scanner.align_state -= 1,
                    OUT_PARAM => {
                        let param = st.input.param_stack
                            [(st.input.cur_input.limit + st.scanner.cur_chr - 1) as usize];
                        try_in!(begin_token_list_in(st, param, PARAMETER));
                        continue 'restart;
                    }
                    _ => {}
                }
            }
        } else {
            try_in!(crate::input::end_token_list_in(st));
            continue 'restart;
        }

        // An alignment entry ends at `&` or `\cr` when braces are balanced.
        if st.scanner.cur_cmd <= CAR_RET
            && st.scanner.cur_cmd >= TAB_MARK
            && st.scanner.align_state == 0
        {
            if st.scanner.scanner_status == ALIGNING || st.scanner.cur_align.0 == TEX_NULL {
                return GetNextOutcome::Abort(crate::input::InputAbort::Fatal(
                    c"(interwoven alignment preambles are not allowed)".as_ptr(),
                ));
            }
            let saved_cmd = st.mem.b32s0(st.scanner.cur_align + 5);
            st.mem
                .set_b32s0(st.scanner.cur_align + 5, st.scanner.cur_chr);
            let template = if saved_cmd as u8 == OMIT {
                OMIT_TEMPLATE.0
            } else {
                st.mem.b32s1(st.scanner.cur_align + 2)
            };
            try_in!(begin_token_list_in(st, template, V_TEMPLATE));
            st.scanner.cur_cmd = saved_cmd as u8;
            st.scanner.align_state = 1_000_000;
            continue 'restart;
        }
        break 'restart;
    }
    GetNextOutcome::Done
}

/// Aborts and terminal error outcomes of the scanner and expander.
pub(crate) enum SccAbort {
    Cancelled,
    Error(crate::errors::ErrorOutcome),
    Input(crate::input::InputAbort),
    NoLegalEnd {
        open_log: bool,
    },
    TokensToString(TokensToStringAbort),
    PdfError {
        t: *const libc::c_char,
        p: *const libc::c_char,
    },
    OpenLog(crate::io::OpenLogAbort),
    MakeTexString(crate::texmfmp::MakeTexStringAbort),
    InputClose,
    NativeFont(crate::ext::NativeFontAbort),
}

impl From<crate::ext::NativeFontAbort> for SccAbort {
    fn from(e: crate::ext::NativeFontAbort) -> Self {
        SccAbort::NativeFont(e)
    }
}

impl SccAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            SccAbort::Cancelled => crate::errors::EngineAbort::Cancelled,
            SccAbort::Error(oc) => oc.into_engine_abort(),
            SccAbort::Input(a) => a.into_engine_abort(),
            SccAbort::NoLegalEnd { open_log } => {
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

/// Lifts a non-`Continue` error outcome into an abort.
pub(crate) fn scc_err(o: crate::errors::ErrorOutcome) -> Result<(), SccAbort> {
    if matches!(o, crate::errors::ErrorOutcome::Continue) {
        Ok(())
    } else {
        Err(SccAbort::Error(o))
    }
}

#[inline]
pub(crate) fn get_next_scc(st: &mut EngineState) -> Result<(), SccAbort> {
    Result::from(get_next_in(st))
}

pub(crate) fn main_mem_overflow_scc() -> SccAbort {
    SccAbort::Input(crate::input::InputAbort::Overflow {
        what: c"main memory size".as_ptr(),
        n: MEM_TOP + 1,
    })
}

pub(crate) fn get_node_scc(st: &mut EngineState, s: i32) -> Result<MemPtr, SccAbort> {
    crate::mem::get_node(&mut st.mem, s).ok_or_else(main_mem_overflow_scc)
}

// Always inlined: an out-of-line `Result<MemPtr, SccAbort>` return goes through memory.
#[inline(always)]
pub(crate) fn get_avail_scc(st: &mut EngineState) -> Result<MemPtr, SccAbort> {
    crate::mem::get_avail(&mut st.mem).ok_or(SccAbort::Input(crate::input::InputAbort::MainMemory))
}

/// Appends `token` to the token list whose tail is `tail`.
#[inline(always)]
pub(crate) fn store_new_token(
    st: &mut EngineState,
    tail: &mut MemPtr,
    token: i32,
) -> Result<(), SccAbort> {
    let q = get_avail_scc(st)?;
    tail.set_link(&mut st.mem, q);
    q.set_info(&mut st.mem, token);
    *tail = q;
    Ok(())
}

/// Starts a new reference-counted token list in `def_ref` and returns it.
fn new_def_ref_in(st: &mut EngineState) -> Result<MemPtr, SccAbort> {
    let def_ref = get_avail_scc(st)?;
    st.scanner.def_ref = def_ref;
    def_ref.set_info(&mut st.mem, TEX_NULL);
    Ok(def_ref)
}

/// The contents of box register `n`, or `None` if it is void.
pub(crate) fn fetch_box(st: &mut EngineState, n: i32) -> Option<AnyBox> {
    let contents = if n < 256 {
        MemPtr::from_raw(st.eqtb.b32s1(BOX_BASE + n))
    } else {
        find_sa_in(st, BOX_VAL, n)?.sa_ptr(&st.mem)
    };
    let idx = contents.as_idx()?;
    Some(
        st.mem
            .classify_box(idx)
            .ok()
            .flatten()
            .expect("box register holds a box"),
    )
}

/// `get_x_token` until the result is not a space.
#[inline]
pub(crate) fn get_x_non_blank_in(st: &mut EngineState) -> Result<(), SccAbort> {
    loop {
        get_x_token_in(st)?;
        if st.scanner.cur_cmd != SPACER {
            return Ok(());
        }
    }
}

pub(crate) fn new_spec_scc(
    st: &mut EngineState,
    p: impl crate::units::MemAddr,
) -> Result<NodePtr<GlueSpecNode>, SccAbort> {
    crate::mem::alloc_spec(&mut st.mem, p).ok_or_else(main_mem_overflow_scc)
}

pub(crate) fn str_toks_scc(st: &mut EngineState, b: i32, cat: i16) -> Result<MemPtr, SccAbort> {
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

#[inline(always)]
fn pack_cur_tok(st: &mut EngineState) {
    st.scanner.cur_tok = if st.scanner.cur_cs == 0 {
        st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr
    } else {
        CS_TOKEN_FLAG + st.scanner.cur_cs
    };
}

pub(crate) fn x_token_in(st: &mut EngineState) -> Result<(), SccAbort> {
    while st.scanner.cur_cmd > MAX_COMMAND {
        expand_in(st)?;
        get_next_scc(st)?;
    }
    pack_cur_tok(st);
    Ok(())
}

pub(crate) fn get_x_token_in(st: &mut EngineState) -> Result<(), SccAbort> {
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
    pack_cur_tok(st);
    Ok(())
}

/// Builds the token list for `\the` (or `\unexpanded`/`\detokenize`) at
/// `link(temp_head)` and returns its tail.
pub(crate) fn the_toks_in(st: &mut EngineState) -> Result<MemPtr, SccAbort> {
    if st.scanner.cur_chr & 1 != 0 {
        let c = st.scanner.cur_chr;
        scan_general_text_in(st)?;
        if c == 1 {
            return Ok(MemPtr::from_raw(st.scanner.cur_val));
        }
        let old_setting = st.shipout.selector;
        st.shipout.selector = SELECTOR_NEW_STRING;
        let b = st.strings.pool_ptr;
        let p = get_avail_scc(st)?;
        let list = TEMP_HEAD.link(&st.mem);
        p.set_link(&mut st.mem, list);
        crate::output::token_show_in(st, p);
        crate::mem::flush_list(&mut st.mem, p);
        st.shipout.selector = old_setting;
        return str_toks_scc(st, b, 0);
    }
    get_x_token_in(st)?;
    scan_something_internal_in(st, TOK_VAL, false)?;
    if st.scanner.cur_val_level >= IDENT_VAL {
        let mut p = TEMP_HEAD;
        p.set_link(&mut st.mem, MemPtr::NULL);
        if st.scanner.cur_val_level == IDENT_VAL {
            store_new_token(st, &mut p, CS_TOKEN_FLAG + st.scanner.cur_val)?;
        } else if st.scanner.cur_val != TEX_NULL {
            let mut r = MemPtr::from_raw(st.scanner.cur_val).link(&st.mem);
            while !r.is_null() {
                store_new_token(st, &mut p, r.info(&st.mem))?;
                r = r.link(&st.mem);
            }
        }
        Ok(p)
    } else {
        let old_setting = st.shipout.selector;
        st.shipout.selector = SELECTOR_NEW_STRING;
        let b = st.strings.pool_ptr;
        match st.scanner.cur_val_level {
            0 => print_int_in(st, st.scanner.cur_val),
            1 => {
                crate::output::print_scaled_in(st, st.scanner.cur_val);
                print_cstr_in(st, c"pt".as_ptr());
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
    st: &mut EngineState,
    macro_def: bool,
    xpand: bool,
) -> Result<i32, SccAbort> {
    if macro_def {
        st.scanner.scanner_status = DEFINING;
    } else {
        st.scanner.scanner_status = ABSORBING;
    }
    st.scanner.warning_index = st.scanner.cur_cs;
    let def_ref = new_def_ref_in(st)?;
    let mut p = def_ref;
    let mut hash_brace: i32 = 0;
    let mut t = ZERO_TOKEN;

    'found: {
        if macro_def {
            let ended_with_brace = loop {
                get_token_in(st)?;
                if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
                    break false;
                }
                if st.scanner.cur_cmd == MAC_PARAM {
                    let s = MATCH_TOKEN + st.scanner.cur_chr;
                    get_token_in(st)?;
                    if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
                        hash_brace = st.scanner.cur_tok;
                        store_new_token(st, &mut p, st.scanner.cur_tok)?;
                        store_new_token(st, &mut p, END_MATCH_TOKEN)?;
                        break true;
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
                        st.errors.help_line[0] = c"as well as the token that followed it.".as_ptr();
                        scc_err(error_in(st))?;
                        continue;
                    }
                    t += 1;
                    if st.scanner.cur_tok != t {
                        error_here_with_diagnostic_in(
                            st,
                            c"Parameters must be numbered consecutively".as_ptr(),
                        );
                        st.errors.help_ptr = 2;
                        st.errors.help_line[1] =
                            c"I've inserted the digit you should have used after the #.".as_ptr();
                        st.errors.help_line[0] = c"Type `1' to delete what you did use.".as_ptr();
                        scc_err(back_error_in(st)?)?;
                    }
                    st.scanner.cur_tok = s;
                }
                store_new_token(st, &mut p, st.scanner.cur_tok)?;
            };
            if !ended_with_brace {
                store_new_token(st, &mut p, END_MATCH_TOKEN)?;
                if st.scanner.cur_cmd == RIGHT_BRACE {
                    error_here_with_diagnostic_in(st, c"Missing { inserted".as_ptr());
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.scanner.align_state += 1;
                    st.errors.help_ptr = 2;
                    st.errors.help_line[1] =
                        c"Where was the left brace? You said something like `\\def\\a}',".as_ptr();
                    st.errors.help_line[0] =
                        c"which I'm going to interpret as `\\def\\a{}'.".as_ptr();
                    scc_err(error_in(st))?;
                    break 'found;
                }
            }
        } else {
            scan_left_brace_in(st)?;
        }

        let mut unbalance: i32 = 1;
        loop {
            if xpand {
                loop {
                    get_next_scc(st)?;
                    if st.scanner.cur_cmd >= CALL && is_protected_macro(&st.mem, st.scanner.cur_chr)
                    {
                        st.scanner.cur_cmd = RELAX;
                        st.scanner.cur_chr = NO_EXPAND_FLAG;
                    }
                    if st.scanner.cur_cmd <= MAX_COMMAND {
                        break;
                    }
                    if st.scanner.cur_cmd != THE {
                        expand_in(st)?;
                    } else {
                        let tail = the_toks_in(st)?;
                        let list = TEMP_HEAD.link(&st.mem);
                        if !list.is_null() {
                            p.set_link(&mut st.mem, list);
                            p = tail;
                        }
                    }
                }
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
                        break 'found;
                    }
                }
            } else if st.scanner.cur_cmd == MAC_PARAM && macro_def {
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
                        scc_err(back_error_in(st)?)?;
                        st.scanner.cur_tok = s;
                    } else {
                        st.scanner.cur_tok = OUT_PARAM_TOKEN - b'0' as i32 + st.scanner.cur_chr;
                    }
                }
            }
            store_new_token(st, &mut p, st.scanner.cur_tok)?;
        }
    }
    st.scanner.scanner_status = NORMAL_SCANNER;
    if hash_brace != 0 {
        store_new_token(st, &mut p, hash_brace)?;
    }
    Ok(p.0)
}

pub(crate) fn pass_text_in(st: &mut EngineState) -> Result<(), SccAbort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = SKIPPING;
    let mut l: i32 = 0;
    st.scanner.skip_line = st.mem.line;

    loop {
        get_next_scc(st)?;
        if st.scanner.cur_cmd == FI_OR_ELSE {
            if l == 0 {
                break;
            }
            if st.scanner.cur_chr == FI_CODE as i32 {
                l -= 1;
            }
        } else if st.scanner.cur_cmd == IF_TEST {
            l += 1;
        }
    }
    st.scanner.scanner_status = save_scanner_status;
    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_IFS) > 0 {
        crate::output::show_cur_cmd_chr_in(st);
    }
    Ok(())
}
pub(crate) fn pseudo_start_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_general_text_in(st)?;
    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    crate::output::token_show_in(st, TEMP_HEAD);
    st.shipout.selector = old_setting;
    let head = st.mem.b32s1(TEMP_HEAD);
    crate::mem::flush_list(&mut st.mem, MemPtr::from_raw(head));

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(SccAbort::Error(crate::errors::ErrorOutcome::Overflow {
            what: c"pool size",
            n: st.strings.pool_size - st.strings.init_pool_ptr,
        }));
    }
    let s = crate::string_pool::make_string(&mut st.strings).map_err(|_| {
        SccAbort::Error(crate::errors::ErrorOutcome::Overflow {
            what: c"number of strings",
            n: st.strings.max_strings - st.strings.init_str_ptr,
        })
    })?;
    st.strings.str_pool[st.strings.pool_ptr as usize] = b' ' as u16;
    let mut l = st.strings.str_start[(s - TOO_BIG_CHAR) as usize];
    let nl = st.eqtb.b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR) as u16;
    let file = get_avail_scc(st)?;
    let mut last_line = file;

    // Each line becomes a node of words holding four characters each.
    while l < st.strings.pool_ptr {
        let m = l;
        while l < st.strings.pool_ptr && st.strings.str_pool[l as usize] != nl {
            l += 1;
        }
        let size = ((l - m + 7) / 4).max(2);
        let line = get_node_scc(st, size)?;
        last_line.set_link(&mut st.mem, line);
        last_line = line;
        line.set_info(&mut st.mem, size);
        let mut w = line.0 + 1;
        let mut k = m;
        for _ in 2..size {
            let pool = &st.strings.str_pool[k as usize..];
            let chars = [pool[0], pool[1], pool[2], pool[3]];
            set_packed_chars(&mut st.mem, w, chars);
            w += 1;
            k += 4;
        }
        let mut chars = [b' ' as u16; 4];
        for (i, c) in chars.iter_mut().enumerate() {
            if l > k + i as i32 {
                *c = st.strings.str_pool[(k + i as i32) as usize];
            }
        }
        set_packed_chars(&mut st.mem, w, chars);
        if st.strings.str_pool[l as usize] == nl {
            l += 1;
        }
    }

    let lines = file.link(&st.mem);
    file.set_info(&mut st.mem, lines.0);
    file.set_link(&mut st.mem, st.input.pseudo_files);
    st.input.pseudo_files = file;
    crate::string_pool::flush_str(&mut st.strings, s);

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
        print_cstr_in(st, c"( ".as_ptr());
        st.input.open_parens += 1;
        unsafe { ttstub_output_flush(st.shipout.rust_stdout) };
    } else {
        st.input.cur_input.name = 18;
        st.input.cur_input.synctex_tag = 0;
        st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
    }
    Ok(())
}
/// Skips blanks and signs, returning whether the net sign is negative.
fn scan_signs_in(st: &mut EngineState) -> Result<bool, SccAbort> {
    let mut negative = false;
    loop {
        get_x_non_blank_in(st)?;
        if st.scanner.cur_tok == MINUS_TOKEN {
            negative = !negative;
            st.scanner.cur_tok = PLUS_TOKEN;
        }
        if st.scanner.cur_tok != PLUS_TOKEN {
            return Ok(negative);
        }
    }
}

/// Consumes one optional space after a constant.
fn scan_optional_space_in(st: &mut EngineState) -> Result<(), SccAbort> {
    get_x_token_in(st)?;
    if st.scanner.cur_cmd != SPACER {
        back_input_in(st)?;
    }
    Ok(())
}

/// Reads the token after `\primitive` and replaces it by the primitive's meaning.
fn get_primitive_token_in(st: &mut EngineState) -> Result<(), SccAbort> {
    get_token_in(st)?;
    let p = crate::hash::prim_lookup_cs(&mut st.eqtb, &mut st.strings, st.scanner.cur_cs)
        .map_err(|e| SccAbort::Input(hash_err_abort(e)))?;
    if p != UNDEFINED_PRIMITIVE {
        st.scanner.cur_cmd = st.eqtb.eq_type(PRIM_EQTB_BASE + p);
        st.scanner.cur_chr = st.eqtb.b32s1(PRIM_EQTB_BASE + p);
        st.scanner.cur_cs = PRIM_EQTB_BASE + p;
        st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
    } else {
        st.scanner.cur_cmd = RELAX;
        st.scanner.cur_chr = 0;
        st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_RELAX;
        st.scanner.cur_cs = FROZEN_RELAX;
    }
    Ok(())
}

pub(crate) fn scan_int_in(st: &mut EngineState) -> Result<(), SccAbort> {
    let mut ok_so_far = true;
    st.scanner.radix = 0;
    let negative = scan_signs_in(st)?;

    loop {
        if st.scanner.cur_tok == ALPHA_TOKEN {
            get_token_in(st)?;
            if st.scanner.cur_tok < CS_TOKEN_FLAG {
                st.scanner.cur_val = st.scanner.cur_chr;
                if st.scanner.cur_cmd == RIGHT_BRACE {
                    st.scanner.align_state += 1;
                } else if st.scanner.cur_cmd < RIGHT_BRACE {
                    st.scanner.align_state -= 1;
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
                scc_err(back_error_in(st)?)?;
            } else {
                scan_optional_space_in(st)?;
            }
        } else if st.scanner.cur_tok == FROZEN_PRIM_TOKEN {
            get_primitive_token_in(st)?;
            continue;
        } else if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
            scan_something_internal_in(st, INT_VAL, false)?;
        } else {
            let (radix, m) = if st.scanner.cur_tok == OCTAL_TOKEN {
                get_x_token_in(st)?;
                (8, 0x1000_0000)
            } else if st.scanner.cur_tok == HEX_TOKEN {
                get_x_token_in(st)?;
                (16, 0x0800_0000)
            } else {
                (10, 0x0CCC_CCCC)
            };
            st.scanner.radix = radix as u8;
            let mut vacuous = true;
            st.scanner.cur_val = 0;
            loop {
                let tok = st.scanner.cur_tok;
                let d = if tok < ZERO_TOKEN + radix && (ZERO_TOKEN..=ZERO_TOKEN + 9).contains(&tok)
                {
                    tok - ZERO_TOKEN
                } else if radix == 16 && (A_TOKEN..=A_TOKEN + 5).contains(&tok) {
                    tok - A_TOKEN + 10
                } else if radix == 16 && (OTHER_A_TOKEN..=OTHER_A_TOKEN + 5).contains(&tok) {
                    tok - OTHER_A_TOKEN + 10
                } else {
                    break;
                };
                vacuous = false;
                if st.scanner.cur_val >= m && (st.scanner.cur_val > m || d > 7 || radix != 10) {
                    if ok_so_far {
                        error_here_with_diagnostic_in(st, c"Number too big".as_ptr());
                        capture_to_diagnostic_in(st, std::ptr::null_mut());
                        st.errors.help_ptr = 2;
                        st.errors.help_line[1] =
                            c"I can only go up to 2147483647='17777777777=\"7FFFFFFF,".as_ptr();
                        st.errors.help_line[0] =
                            c"so I'm using that number instead of yours.".as_ptr();
                        scc_err(error_in(st))?;
                        st.scanner.cur_val = TEX_INFINITY;
                        ok_so_far = false;
                    }
                } else {
                    st.scanner.cur_val = st.scanner.cur_val * radix + d;
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
                scc_err(back_error_in(st)?)?;
            } else if st.scanner.cur_cmd != SPACER {
                back_input_in(st)?;
            }
        }
        break;
    }

    if negative {
        st.scanner.cur_val = -st.scanner.cur_val;
    }
    Ok(())
}
pub(crate) fn scan_dimen_in(
    st: &mut EngineState,
    mu: bool,
    inf: bool,
    shortcut: bool,
) -> Result<(), SccAbort> {
    xetex_scan_dimen_in(st, mu, inf, shortcut, true)
}
/// A glue spec pointer held in a raw word.
fn glue_spec(p: i32) -> NodePtr<GlueSpecNode> {
    NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(p)).expect("non-null glue spec")
}

/// Replaces the glue in `cur_val` by its width, releasing the glue.
fn glue_to_width_in(st: &mut EngineState) {
    let spec = st.scanner.cur_val;
    st.scanner.cur_val = glue_spec(spec).width(&st.mem).0;
    crate::mem::delete_glue_ref(&mut st.mem, spec);
}

/// Scales `cur_val` and the fraction `f` by `num / denom`.
fn scale_dimen_in(st: &mut EngineState, f: &mut i32, num: i32, denom: i32) {
    st.scanner.cur_val =
        crate::scaled_math::xn_over_d_in(&mut st.errors, st.scanner.cur_val, num, denom);
    *f = (num * *f + 65536 * st.errors.tex_remainder) / denom;
    st.scanner.cur_val += *f / 65536;
    *f %= 65536;
}

/// Sets `cur_val` to `integer.fraction` times the unit `v`.
fn attach_unit_in(st: &mut EngineState, integer: i32, fraction: i32, v: i32) {
    let fraction_part = crate::scaled_math::xn_over_d_in(&mut st.errors, v, fraction, 65536);
    st.scanner.cur_val = crate::scaled_math::mult_and_add_in(
        &mut st.errors,
        integer,
        v,
        fraction_part,
        MAX_HALFWORD,
    );
}

pub(crate) fn xetex_scan_dimen_in(
    st: &mut EngineState,
    mu: bool,
    inf: bool,
    shortcut: bool,
    requires_units: bool,
) -> Result<(), SccAbort> {
    let mut f: i32 = 0;
    let mut negative = false;

    st.errors.arith_error = false;
    st.scanner.cur_order = NORMAL;

    'attach_sign: {
        if !shortcut {
            negative = scan_signs_in(st)?;

            if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
                if mu {
                    scan_something_internal_in(st, MU_VAL, false)?;
                    if st.scanner.cur_val_level >= GLUE_VAL {
                        glue_to_width_in(st);
                    }
                    if st.scanner.cur_val_level == MU_VAL {
                        break 'attach_sign;
                    }
                    if st.scanner.cur_val_level != INT_VAL {
                        scc_err(mu_error_in(st))?;
                    }
                } else {
                    scan_something_internal_in(st, DIMEN_VAL, false)?;
                    if st.scanner.cur_val_level == DIMEN_VAL {
                        break 'attach_sign;
                    }
                }
            } else {
                back_input_in(st)?;
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
                    // Up to 17 digits are kept, most recent first, in a temporary list.
                    let mut k: i32 = 0;
                    let mut digits = MemPtr::NULL;
                    get_token_in(st)?;
                    loop {
                        get_x_token_in(st)?;
                        if st.scanner.cur_tok > ZERO_TOKEN + 9 || st.scanner.cur_tok < ZERO_TOKEN {
                            break;
                        }
                        if k < 17 {
                            let q = get_avail_scc(st)?;
                            q.set_link(&mut st.mem, digits);
                            q.set_info(&mut st.mem, st.scanner.cur_tok - ZERO_TOKEN);
                            digits = q;
                            k += 1;
                        }
                    }
                    for i in (0..k as usize).rev() {
                        st.errors.dig[i] = digits.info(&st.mem) as u8;
                        let next = digits.link(&st.mem);
                        crate::mem::free_avail(&mut st.mem, digits);
                        digits = next;
                    }
                    f = round_decimals(&st.errors, k);
                    if st.scanner.cur_cmd != SPACER {
                        back_input_in(st)?;
                    }
                }
            }
        }

        if st.scanner.cur_val < 0 {
            negative = !negative;
            st.scanner.cur_val = -st.scanner.cur_val;
        }

        if requires_units {
            let apply_fraction = 'units: {
                if inf && scan_keyword_in(st, c"fil".as_ptr())? {
                    st.scanner.cur_order = FIL;
                    while scan_keyword_in(st, c"l".as_ptr())? {
                        if st.scanner.cur_order == FILLL {
                            error_here_with_diagnostic_in(
                                st,
                                c"Illegal unit of measure (replaced with filll)".as_ptr(),
                            );
                            capture_to_diagnostic_in(st, std::ptr::null_mut());
                            st.errors.help_ptr = 1;
                            st.errors.help_line[0] =
                                c"I dddon't go any higher than filll.".as_ptr();
                            scc_err(error_in(st))?;
                        } else {
                            st.scanner.cur_order += 1;
                        }
                    }
                    break 'units true;
                }

                let save_cur_val = st.scanner.cur_val;
                get_x_non_blank_in(st)?;

                if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
                    if mu {
                        scan_something_internal_in(st, MU_VAL, false)?;
                        if st.scanner.cur_val_level >= GLUE_VAL {
                            glue_to_width_in(st);
                        }
                        if st.scanner.cur_val_level != MU_VAL {
                            scc_err(mu_error_in(st))?;
                        }
                    } else {
                        scan_something_internal_in(st, DIMEN_VAL, false)?;
                    }
                    attach_unit_in(st, save_cur_val, f, st.scanner.cur_val);
                    break 'attach_sign;
                }

                back_input_in(st)?;

                if !mu {
                    let font_param = if scan_keyword_in(st, c"em".as_ptr())? {
                        Some(QUAD_CODE)
                    } else if scan_keyword_in(st, c"ex".as_ptr())? {
                        Some(X_HEIGHT_CODE)
                    } else {
                        None
                    };
                    if let Some(code) = font_param {
                        let font = st.eqtb.b32s1(CUR_FONT_LOC);
                        let v = st
                            .fonts
                            .font_info_b32s1(code + st.fonts.param_base[font as usize]);
                        scan_optional_space_in(st)?;
                        attach_unit_in(st, save_cur_val, f, v);
                        break 'attach_sign;
                    }
                }

                if mu {
                    if scan_keyword_in(st, c"mu".as_ptr())? {
                        break 'units true;
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
                        scc_err(error_in(st))?;
                        break 'units true;
                    }
                }

                if scan_keyword_in(st, c"true".as_ptr())? {
                    scc_err(crate::save::prepare_mag_in(st))?;
                    let mag = st.eqtb.b32s1(INT_BASE + INT_PAR_MAG);
                    if mag != 1000 {
                        scale_dimen_in(st, &mut f, 1000, mag);
                    }
                }

                if scan_keyword_in(st, c"pt".as_ptr())? {
                    break 'units true;
                }

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
                    break 'units false;
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
                    scc_err(error_in(st))?;
                    break 'units true;
                };
                scale_dimen_in(st, &mut f, num, denom);
                true
            };

            if apply_fraction {
                attach_fraction(st, f);
            }
            scan_optional_space_in(st)?;
        } else {
            attach_fraction(st, f);
        }
    }

    if st.errors.arith_error || st.scanner.cur_val.abs() >= 0x4000_0000 {
        error_here_with_diagnostic_in(st, c"Dimension too large".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] = c"I can't work with sizes bigger than about 19 feet.".as_ptr();
        st.errors.help_line[0] = c"Continue and I'll use the largest value I can.".as_ptr();
        scc_err(error_in(st))?;
        st.scanner.cur_val = MAX_HALFWORD;
        st.errors.arith_error = false;
    }
    if negative {
        st.scanner.cur_val = -st.scanner.cur_val;
    }
    Ok(())
}
fn attach_fraction(st: &mut EngineState, f: i32) {
    if st.scanner.cur_val >= 16384 {
        st.errors.arith_error = true;
    } else {
        st.scanner.cur_val = st.scanner.cur_val * 65536 + f;
    }
}

pub(crate) fn scan_glue_in(st: &mut EngineState, level: u8) -> Result<(), SccAbort> {
    let mu = level == MU_VAL;
    let negative = scan_signs_in(st)?;

    if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
        scan_something_internal_in(st, level, negative)?;
        if st.scanner.cur_val_level >= GLUE_VAL {
            if st.scanner.cur_val_level != level {
                scc_err(mu_error_in(st))?;
            }
            return Ok(());
        }
        if st.scanner.cur_val_level == INT_VAL {
            scan_dimen_in(st, mu, false, true)?;
        } else if level == MU_VAL {
            scc_err(mu_error_in(st))?;
        }
    } else {
        back_input_in(st)?;
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
        gs.set_stretch_order(&mut st.mem, st.scanner.cur_order);
    }
    if scan_keyword_in(st, c"minus".as_ptr())? {
        scan_dimen_in(st, mu, true, false)?;
        gs.set_shrink(&mut st.mem, Scaled(st.scanner.cur_val));
        gs.set_shrink_order(&mut st.mem, st.scanner.cur_order);
    }

    st.scanner.cur_val = gs.ptr().0;
    Ok(())
}
pub(crate) fn scan_normal_glue_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_glue_in(st, GLUE_VAL)
}
pub(crate) fn scan_mu_glue_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_glue_in(st, MU_VAL)
}
/// Applies `op` to the width, stretch and shrink of `spec`, in that order.
fn map_glue_in(
    st: &mut EngineState,
    spec: NodePtr<GlueSpecNode>,
    mut op: impl FnMut(&mut crate::state::ErrorState, i32) -> i32,
) {
    let width = op(&mut st.errors, spec.width(&st.mem).0);
    spec.set_width(&mut st.mem, Scaled(width));
    let stretch = op(&mut st.errors, spec.stretch(&st.mem).0);
    spec.set_stretch(&mut st.mem, Scaled(stretch));
    let shrink = op(&mut st.errors, spec.shrink(&st.mem).0);
    spec.set_shrink(&mut st.mem, Scaled(shrink));
}

fn normalize_glue_orders(st: &mut EngineState, spec: NodePtr<GlueSpecNode>) {
    if spec.stretch(&st.mem).0 == 0 {
        spec.set_stretch_order(&mut st.mem, NORMAL_EXPR);
    }
    if spec.shrink(&st.mem).0 == 0 {
        spec.set_shrink_order(&mut st.mem, NORMAL_EXPR);
    }
}

/// Adds (or subtracts) glue `t` to glue `e` and releases `t`.
fn add_glue_in(
    st: &mut EngineState,
    e: NodePtr<GlueSpecNode>,
    t: NodePtr<GlueSpecNode>,
    sub: bool,
) {
    use crate::scaled_math::add_or_sub_in;
    let mem = &mut st.mem;
    let errors = &mut st.errors;
    let width = add_or_sub_in(errors, e.width(mem).0, t.width(mem).0, MAX_HALFWORD, sub);
    e.set_width(mem, Scaled(width));
    let (e_order, t_order, t_stretch) =
        (e.stretch_order(mem), t.stretch_order(mem), t.stretch(mem));
    if e_order == t_order {
        let stretch = add_or_sub_in(errors, e.stretch(mem).0, t_stretch.0, MAX_HALFWORD, sub);
        e.set_stretch(mem, Scaled(stretch));
    } else if e_order < t_order && t_stretch.0 != 0 {
        e.set_stretch(mem, t_stretch);
        e.set_stretch_order(mem, t_order);
    }
    let (e_order, t_order, t_shrink) = (e.shrink_order(mem), t.shrink_order(mem), t.shrink(mem));
    if e_order == t_order {
        let shrink = add_or_sub_in(errors, e.shrink(mem).0, t_shrink.0, MAX_HALFWORD, sub);
        e.set_shrink(mem, Scaled(shrink));
    } else if e_order < t_order && t_shrink.0 != 0 {
        e.set_shrink(mem, t_shrink);
        e.set_shrink_order(mem, t_order);
    }
    crate::mem::delete_glue_ref(mem, t.ptr());
    normalize_glue_orders(st, e);
}

pub(crate) fn scan_expr_in(st: &mut EngineState) -> Result<(), SccAbort> {
    use crate::scaled_math::{add_or_sub_in, fract_in, mult_and_add_in, quotient_in};

    let mut l = st.scanner.cur_val_level;
    let a = st.errors.arith_error;
    let mut b = false;
    let mut p = TEX_NULL;

    st.scanner.expand_depth_count += 1;
    if st.scanner.expand_depth_count >= st.scanner.expand_depth {
        return Err(SccAbort::Input(crate::input::InputAbort::Overflow {
            what: c"expansion depth".as_ptr(),
            n: st.scanner.expand_depth,
        }));
    }

    // `r` is the pending additive operator, `s` the pending multiplicative one,
    // `e` the expression so far, `t` the current term and `n` a scaling numerator.
    let mut r: i16;
    let mut e: i32;
    let mut s: i16;
    let mut t: i32;
    let mut n: i32;
    let mut f: i32 = 0;

    'restart: loop {
        r = EXPR_NONE;
        e = 0;
        s = EXPR_NONE;
        t = 0;
        n = 0;
        let mut scan_factor = true;

        loop {
            if scan_factor {
                let o = if s == EXPR_NONE { l } else { INT_VAL };
                get_x_non_blank_in(st)?;
                if st.scanner.cur_tok == OTHER_TOKEN + b'(' as i32 {
                    let q = get_node_scc(st, EXPR_NODE_SIZE)?.0;
                    st.mem.set_b32s1(q, p);
                    st.mem.set_b16s1(q, l as u16);
                    st.mem.set_b16s0(q, (4 * s + r) as u16);
                    st.mem.set_b32s1(q + 1, e);
                    st.mem.set_b32s1(q + 2, t);
                    st.mem.set_b32s1(q + 3, n);
                    p = q;
                    l = o;
                    continue 'restart;
                }
                back_input_in(st)?;
                match o {
                    INT_VAL => scan_int_in(st)?,
                    DIMEN_VAL => scan_dimen_in(st, false, false, false)?,
                    GLUE_VAL => scan_normal_glue_in(st)?,
                    _ => scan_mu_glue_in(st)?,
                }
                f = st.scanner.cur_val;
            }
            scan_factor = true;

            get_x_non_blank_in(st)?;
            let mut o = match u8::try_from(st.scanner.cur_tok - OTHER_TOKEN) {
                Ok(b'+') => EXPR_ADD,
                Ok(b'-') => EXPR_SUB,
                Ok(b'*') => EXPR_MULT,
                Ok(b'/') => EXPR_DIV,
                _ => EXPR_NONE,
            };
            if o == EXPR_NONE {
                if p == TEX_NULL {
                    if st.scanner.cur_cmd != RELAX {
                        back_input_in(st)?;
                    }
                } else if st.scanner.cur_tok != OTHER_TOKEN + b')' as i32 {
                    error_here_with_diagnostic_in(
                        st,
                        c"Missing ) inserted for expression".as_ptr(),
                    );
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 1;
                    st.errors.help_line[0] =
                        c"I was expecting to see `+', `-', `*', `/', or `)'. Didn't.".as_ptr();
                    scc_err(back_error_in(st)?)?;
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
            } else {
                let spec = glue_spec(f);
                if spec.width(&st.mem).0.abs() > MAX_HALFWORD
                    || spec.stretch(&st.mem).0.abs() > MAX_HALFWORD
                    || spec.shrink(&st.mem).0.abs() > MAX_HALFWORD
                {
                    st.errors.arith_error = true;
                    crate::mem::delete_glue_ref(&mut st.mem, f);
                    f = new_spec_scc(st, 0)?.ptr().0;
                }
            }

            match s {
                EXPR_NONE => {
                    if l >= GLUE_VAL && o != EXPR_NONE {
                        let term = new_spec_scc(st, f)?;
                        t = term.ptr().0;
                        crate::mem::delete_glue_ref(&mut st.mem, f);
                        normalize_glue_orders(st, term);
                    } else {
                        t = f;
                    }
                }
                EXPR_MULT => {
                    if o == EXPR_DIV {
                        n = f;
                        o = EXPR_SCALE;
                    } else if l == INT_VAL {
                        t = mult_and_add_in(&mut st.errors, t, f, 0, TEX_INFINITY);
                    } else if l == DIMEN_VAL {
                        t = mult_and_add_in(&mut st.errors, t, f, 0, MAX_HALFWORD);
                    } else {
                        map_glue_in(st, glue_spec(t), |errors, x| {
                            mult_and_add_in(errors, x, f, 0, MAX_HALFWORD)
                        });
                    }
                }
                EXPR_DIV => {
                    if l < GLUE_VAL {
                        t = quotient_in(&mut st.errors, t, f);
                    } else {
                        map_glue_in(st, glue_spec(t), |errors, x| quotient_in(errors, x, f));
                    }
                }
                EXPR_SCALE => {
                    if l == INT_VAL {
                        t = fract_in(&mut st.errors, t, n, f, TEX_INFINITY);
                    } else if l == DIMEN_VAL {
                        t = fract_in(&mut st.errors, t, n, f, MAX_HALFWORD);
                    } else {
                        map_glue_in(st, glue_spec(t), |errors, x| {
                            fract_in(errors, x, n, f, MAX_HALFWORD)
                        });
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
                    e = add_or_sub_in(&mut st.errors, e, t, TEX_INFINITY, r == EXPR_SUB);
                } else if l == DIMEN_VAL {
                    e = add_or_sub_in(&mut st.errors, e, t, MAX_HALFWORD, r == EXPR_SUB);
                } else {
                    add_glue_in(st, glue_spec(e), glue_spec(t), r == EXPR_SUB);
                }
                r = o;
            }
            b = st.errors.arith_error;
            if o != EXPR_NONE {
                continue;
            }
            if p == TEX_NULL {
                break 'restart;
            }
            f = e;
            let q = p;
            e = st.mem.b32s1(q + 1);
            t = st.mem.b32s1(q + 2);
            n = st.mem.b32s1(q + 3);
            s = (st.mem.b16s0(q) / 4) as i16;
            r = (st.mem.b16s0(q) % 4) as i16;
            l = st.mem.b16s1(q) as u8;
            p = st.mem.b32s1(q);
            crate::mem::free_node(&mut st.mem, MemPtr::from_raw(q), EXPR_NODE_SIZE);
            scan_factor = false;
        }
    }

    st.scanner.expand_depth_count -= 1;

    if b {
        error_here_with_diagnostic_in(st, c"Arithmetic overflow".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] = c"I can't evaluate this expression,".as_ptr();
        st.errors.help_line[0] = c"since the result is out of range.".as_ptr();
        scc_err(error_in(st))?;
        if l >= GLUE_VAL {
            crate::mem::delete_glue_ref(&mut st.mem, e);
            e = 0;
            let zero_glue = glue_spec(e);
            let refs = zero_glue.ref_count(&st.mem);
            zero_glue.set_ref_count(&mut st.mem, refs + 1);
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
    st: &mut EngineState,
    s: *const libc::c_char,
) -> Result<bool, SccAbort> {
    // SAFETY: callers pass NUL-terminated keyword literals.
    let keyword = unsafe { std::ffi::CStr::from_ptr(s) }.to_bytes();
    let save_cur_cs = st.scanner.cur_cs;
    let mut p = BACKUP_HEAD;
    p.set_link(&mut st.mem, MemPtr::NULL);
    let mut i = 0;
    while i < keyword.len() {
        get_x_token_in(st)?;
        let ch = keyword[i] as i32;
        if st.scanner.cur_cs == 0 && (st.scanner.cur_chr == ch || st.scanner.cur_chr == ch - 32) {
            store_new_token(st, &mut p, st.scanner.cur_tok)?;
            i += 1;
        } else if st.scanner.cur_cmd != SPACER || p != BACKUP_HEAD {
            back_input_in(st)?;
            if p != BACKUP_HEAD {
                let matched = st.mem.b32s1(BACKUP_HEAD);
                begin_token_list_in(st, matched, BACKED_UP)?;
            }
            if keyword.len() == 1 {
                st.scanner.cur_cs = save_cur_cs;
            }
            return Ok(false);
        }
    }
    let matched = st.mem.link_ptr(BACKUP_HEAD);
    crate::mem::flush_list(&mut st.mem, matched);
    Ok(true)
}

pub(crate) fn scan_left_brace_in(st: &mut EngineState) -> Result<(), SccAbort> {
    get_x_non_blank_non_relax_in(st)?;
    if st.scanner.cur_cmd != LEFT_BRACE {
        error_here_with_diagnostic_in(st, c"Missing { inserted".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 4;
        st.errors.help_line[3] = c"A left brace was mandatory here, so I've put one in.".as_ptr();
        st.errors.help_line[2] =
            c"You might want to delete and/or insert some corrections".as_ptr();
        st.errors.help_line[1] = c"so that I will find a matching right brace soon.".as_ptr();
        st.errors.help_line[0] = c"(If you're confused by all this, try typing `I}' now.)".as_ptr();
        scc_err(back_error_in(st)?)?;
        st.scanner.cur_tok = LEFT_BRACE_TOKEN + '{' as i32;
        st.scanner.cur_cmd = LEFT_BRACE;
        st.scanner.cur_chr = '{' as i32;
        st.scanner.align_state += 1;
    }
    Ok(())
}

fn missing_number_error_in(st: &mut EngineState) -> Result<(), SccAbort> {
    error_here_with_diagnostic_in(st, c"Missing number, treated as zero".as_ptr());
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 3;
    st.errors.help_line[2] = c"A number should have been here; I inserted `0'.".as_ptr();
    st.errors.help_line[1] = c"(If you can't figure out why I needed to see a number,".as_ptr();
    st.errors.help_line[0] = c"look up `weird error' in the index to The TeXbook.)".as_ptr();
    scc_err(back_error_in(st)?)
}

#[inline]
fn set_cur_val(st: &mut EngineState, value: i32, level: u8) {
    st.scanner.cur_val = value;
    st.scanner.cur_val_level = level as u8;
}

/// The sparse-array element of type `t` for register `n`, if it exists.
fn find_sa_in(st: &mut EngineState, t: u8, n: i32) -> Option<NodePtr<SaNode>> {
    crate::save::find_sa_element(&mut st.mem, &mut st.scanner, t, n, false);
    NodePtr::<SaNode>::from_ptr(st.scanner.cur_ptr)
}

fn sa_token_list(st: &EngineState, sa: Option<NodePtr<SaNode>>) -> i32 {
    sa.map_or(TEX_NULL, |sa| sa.sa_ptr(&st.mem).0)
}

/// Replaces the glue in `cur_val` by a negated copy and returns the original glue.
fn negate_glue_in(st: &mut EngineState) -> Result<i32, SccAbort> {
    let old = st.scanner.cur_val;
    let spec = new_spec_scc(st, old)?;
    st.scanner.cur_val = spec.ptr().0;
    map_glue_in(st, spec, |_, x| -x);
    Ok(old)
}

fn name_of_file_ptr(st: &EngineState) -> *mut libc::c_char {
    if st.input.name_of_file.is_empty() {
        std::ptr::null_mut()
    } else {
        st.input.name_of_file.as_ptr() as *mut libc::c_char
    }
}

fn is_native_font(st: &EngineState, f: i32) -> bool {
    let fa = st.fonts.font_area[f as usize];
    fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG
}

/// Scans a font identifier, returning the font, its layout engine and its area flag.
fn scan_font_engine_in(st: &mut EngineState) -> Result<(i32, *mut libc::c_void, i32), SccAbort> {
    scan_font_ident_in(st)?;
    let f = st.scanner.cur_val;
    Ok((
        f,
        st.fonts.font_layout_engine_ptr(f),
        st.fonts.font_area[f as usize],
    ))
}

fn uses_graphite(fa: i32, engine: *mut libc::c_void) -> bool {
    fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) }
}

fn uses_opentype(fa: i32, engine: *mut libc::c_void) -> bool {
    fa == OTGR_FONT_FLAG && unsafe { usingOpenType(engine) }
}

/// Reports a font query on an unsuitable font and yields `fallback`.
fn font_query_error_in(
    st: &mut EngineState,
    m: i32,
    f: i32,
    msg: &std::ffi::CStr,
    fallback: i32,
) -> Result<(), SccAbort> {
    scc_err(crate::errors::not_font_error_common_in(
        st,
        LAST_ITEM,
        m,
        f,
        msg.as_ptr(),
    ))?;
    st.scanner.cur_val = fallback;
    Ok(())
}

fn fetch_def_code_in(st: &mut EngineState, m: i32) -> Result<(), SccAbort> {
    scan_usv_num_in(st)?;
    let n = st.scanner.cur_val;
    if m == MATH_CODE_BASE {
        let code = st.eqtb.b32s1(MATH_CODE_BASE + n);
        st.scanner.cur_val1 = code;
        if math_char_fn(code) == ACTIVE_MATH_CHAR {
            st.scanner.cur_val1 = 0x8000;
        } else if math_class_fn(code) > 7 || math_fam_fn(code) > 15 || math_char_fn(code) > 255 {
            bad_int_error(
                st,
                c"Extended mathchar used as mathchar".as_ptr(),
                code,
                c"A mathchar number must be between 0 and \"7FFF.".as_ptr(),
                c"I changed this one to zero.",
            )?;
            st.scanner.cur_val1 = 0;
        }
        let code = st.scanner.cur_val1;
        let v = math_class_fn(code) * 0x1000 + math_fam_fn(code) * 0x100 + math_char_fn(code);
        set_cur_val(st, v, INT_VAL);
    } else if m == DEL_CODE_BASE {
        st.scanner.cur_val1 = st.eqtb.b32s1(DEL_CODE_BASE + n);
        if st.scanner.cur_val1 >= 0x4000_0000 {
            error_here_with_diagnostic_in(st, c"Extended delcode used as delcode".as_ptr());
            capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 2;
            st.errors.help_line[1] =
                c"I can only go up to 2147483647='17777777777=\"7FFFFFFF,".as_ptr();
            st.errors.help_line[0] = c"I changed this one to zero.".as_ptr();
            scc_err(error_in(st))?;
            set_cur_val(st, 0, INT_VAL);
        } else {
            set_cur_val(st, st.scanner.cur_val1, INT_VAL);
        }
    } else if m < SF_CODE_BASE {
        set_cur_val(st, st.eqtb.b32s1(m + n), INT_VAL);
    } else if m < MATH_CODE_BASE {
        set_cur_val(st, st.eqtb.b32s1(m + n) % 65536, INT_VAL);
    } else {
        set_cur_val(st, st.eqtb.b32s1(m + n), INT_VAL);
    }
    Ok(())
}

fn fetch_xetex_def_code_in(st: &mut EngineState, m: i32) -> Result<(), SccAbort> {
    scan_usv_num_in(st)?;
    let n = st.scanner.cur_val;
    let (err, help1, help0) = if m == SF_CODE_BASE {
        set_cur_val(st, st.eqtb.b32s1(SF_CODE_BASE + n) / 65536, INT_VAL);
        return Ok(());
    } else if m == MATH_CODE_BASE {
        set_cur_val(st, st.eqtb.b32s1(MATH_CODE_BASE + n), INT_VAL);
        return Ok(());
    } else if m == MATH_CODE_BASE + 1 {
        (
            c"Can't use \\Umathcode as a number (try \\Umathcodenum)",
            c"\\Umathcode is for setting a mathcode from separate values;",
            c"use \\Umathcodenum to access them as single values.",
        )
    } else if m == DEL_CODE_BASE {
        set_cur_val(st, st.eqtb.b32s1(DEL_CODE_BASE + n), INT_VAL);
        return Ok(());
    } else {
        (
            c"Can't use \\Udelcode as a number (try \\Udelcodenum)",
            c"\\Udelcode is for setting a delcode from separate values;",
            c"use \\Udelcodenum to access them as single values.",
        )
    };
    error_here_with_diagnostic_in(st, err.as_ptr());
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 2;
    st.errors.help_line[1] = help1.as_ptr();
    st.errors.help_line[0] = help0.as_ptr();
    scc_err(error_in(st))?;
    set_cur_val(st, 0, INT_VAL);
    Ok(())
}

fn fetch_token_list_in(st: &mut EngineState, m: i32, level: u8) -> Result<(), SccAbort> {
    if level != TOK_VAL {
        missing_number_error_in(st)?;
        set_cur_val(st, 0, DIMEN_VAL);
    } else if st.scanner.cur_cmd < ASSIGN_TOKS {
        if m == 0 {
            scan_register_num_in(st)?;
            let n = st.scanner.cur_val;
            st.scanner.cur_val = if n < 256 {
                st.eqtb.b32s1(TOKS_BASE + n)
            } else {
                let sa = find_sa_in(st, TOK_VAL, n);
                sa_token_list(st, sa)
            };
        } else {
            st.scanner.cur_val = st.mem.b32s1(m + 1);
        }
        st.scanner.cur_val_level = TOK_VAL;
    } else if st.scanner.cur_cmd == ASSIGN_TOKS {
        if st.scanner.cur_chr == LOCAL_BASE + LOCAL_XETEX_INTER_CHAR_TOKS {
            scan_char_class_not_ignored_in(st)?;
            st.scanner.cur_ptr.0 = st.scanner.cur_val;
            scan_char_class_not_ignored_in(st)?;
            let n = st.scanner.cur_ptr.0 * CHAR_CLASS_LIMIT + st.scanner.cur_val;
            let sa = find_sa_in(st, INTER_CHAR_VAL, n);
            st.scanner.cur_val = sa_token_list(st, sa);
        } else {
            st.scanner.cur_val = st.eqtb.b32s1(m);
        }
        st.scanner.cur_val_level = TOK_VAL;
    } else {
        back_input_in(st)?;
        scan_font_ident_in(st)?;
        set_cur_val(st, FONT_ID_BASE + st.scanner.cur_val, IDENT_VAL);
    }
    Ok(())
}

fn fetch_register_in(st: &mut EngineState, m: i32) -> Result<(), SccAbort> {
    if !(0..=19).contains(&m) {
        let sa = NodePtr::<SaNode>::from_ptr(MemPtr::from_raw(m)).expect("sparse register");
        let level = sa.sa_type(&st.mem) as u8;
        let value = if level < GLUE_VAL {
            sa.sa_int(&st.mem)
        } else {
            sa.sa_ptr(&st.mem).0
        };
        set_cur_val(st, value, level);
        return Ok(());
    }
    scan_register_num_in(st)?;
    let n = st.scanner.cur_val;
    let level = m as u8;
    st.scanner.cur_val_level = level as u8;
    if n > 255 {
        st.scanner.cur_val = match find_sa_in(st, level, n) {
            None => 0,
            Some(sa) if level < GLUE_VAL => sa.sa_int(&st.mem),
            Some(sa) => sa.sa_ptr(&st.mem).0,
        };
    } else {
        let base = match level {
            INT_VAL => COUNT_BASE,
            DIMEN_VAL => SCALED_BASE,
            GLUE_VAL => SKIP_BASE,
            MU_VAL => MU_SKIP_BASE,
            _ => return Ok(()),
        };
        st.scanner.cur_val = st.eqtb.b32s1(base + n);
    }
    Ok(())
}

/// Integer-valued `\lastnodetype`-style and XeTeX font queries.
fn fetch_last_item_int_in(st: &mut EngineState, m: i32) -> Result<(), SccAbort> {
    match m {
        INPUT_LINE_NO_CODE => st.scanner.cur_val = st.mem.line,
        BADNESS_CODE => st.scanner.cur_val = st.shipout.last_badness,
        ELAPSED_TIME_CODE => {
            st.scanner.cur_val = if st.config.deterministic_mode {
                0
            } else {
                crate::texmfmp::get_microinterval(&st.shipout)
            };
        }
        RANDOM_SEED_CODE => st.scanner.cur_val = st.errors.random_seed,
        PDF_SHELL_ESCAPE_CODE => st.scanner.cur_val = st.config.shell_escape_enabled as i32,
        ETEX_VERSION_CODE => st.scanner.cur_val = ETEX_VERSION,
        XETEX_VERSION_CODE => st.scanner.cur_val = XETEX_VERSION,
        XETEX_COUNT_GLYPHS_CODE => {
            let (_, engine, fa) = scan_font_engine_in(st)?;
            st.scanner.cur_val = if fa == AAT_FONT_FLAG {
                aat_font_get(m, engine)
            } else if fa == OTGR_FONT_FLAG {
                unsafe { ot_font_get(m, engine) }
            } else {
                0
            };
        }
        XETEX_COUNT_FEATURES_CODE => {
            let (_, engine, fa) = scan_font_engine_in(st)?;
            st.scanner.cur_val = if fa == AAT_FONT_FLAG {
                aat_font_get(m, engine)
            } else if uses_graphite(fa, engine) {
                unsafe { ot_font_get(m, engine) }
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
        XETEX_FEATURE_CODE_CODE | XETEX_IS_EXCLUSIVE_FEATURE_CODE | XETEX_COUNT_SELECTORS_CODE => {
            let (f, engine, fa) = scan_font_engine_in(st)?;
            if fa == AAT_FONT_FLAG {
                scan_int_in(st)?;
                st.scanner.cur_val = aat_font_get_1(m, engine, st.scanner.cur_val);
            } else if uses_graphite(fa, engine) {
                scan_int_in(st)?;
                st.scanner.cur_val = unsafe { ot_font_get_1(m, engine, st.scanner.cur_val) };
            } else {
                font_query_error_in(st, m, f, c"; not an AAT or Graphite font", -1)?;
            }
        }
        XETEX_SELECTOR_CODE_CODE | XETEX_IS_DEFAULT_SELECTOR_CODE => {
            let (f, engine, fa) = scan_font_engine_in(st)?;
            if fa == AAT_FONT_FLAG {
                scan_int_in(st)?;
                let k = st.scanner.cur_val;
                scan_int_in(st)?;
                st.scanner.cur_val = aat_font_get_2(m, engine, k, st.scanner.cur_val);
            } else if uses_graphite(fa, engine) {
                scan_int_in(st)?;
                let k = st.scanner.cur_val;
                scan_int_in(st)?;
                st.scanner.cur_val = unsafe { ot_font_get_2(m, engine, k, st.scanner.cur_val) };
            } else {
                font_query_error_in(st, m, f, c"; not an AAT or Graphite font", -1)?;
            }
        }
        XETEX_FIND_VARIATION_BY_NAME_CODE => {
            let (f, engine, fa) = scan_font_engine_in(st)?;
            if fa == AAT_FONT_FLAG {
                crate::io::scan_and_pack_name_in(st)?;
                st.scanner.cur_val = aat_font_get_named(m, engine);
            } else {
                font_query_error_in(st, m, f, c"; not an AAT font", -1)?;
            }
        }
        XETEX_FIND_FEATURE_BY_NAME_CODE => {
            let (f, engine, fa) = scan_font_engine_in(st)?;
            if fa == AAT_FONT_FLAG {
                crate::io::scan_and_pack_name_in(st)?;
                st.scanner.cur_val = aat_font_get_named(m, engine);
            } else if uses_graphite(fa, engine) {
                crate::io::scan_and_pack_name_in(st)?;
                st.scanner.cur_val = unsafe { gr_font_get_named(st, m, engine) };
            } else {
                font_query_error_in(st, m, f, c"; not an AAT or Graphite font", -1)?;
            }
        }
        XETEX_FIND_SELECTOR_BY_NAME_CODE => {
            let (f, engine, fa) = scan_font_engine_in(st)?;
            if fa == AAT_FONT_FLAG {
                scan_int_in(st)?;
                let k = st.scanner.cur_val;
                crate::io::scan_and_pack_name_in(st)?;
                st.scanner.cur_val = aat_font_get_named_1(m, engine, k);
            } else if uses_graphite(fa, engine) {
                scan_int_in(st)?;
                let k = st.scanner.cur_val;
                crate::io::scan_and_pack_name_in(st)?;
                st.scanner.cur_val = unsafe { gr_font_get_named_1(st, m, engine, k) };
            } else {
                font_query_error_in(st, m, f, c"; not an AAT or Graphite font", -1)?;
            }
        }
        XETEX_OT_COUNT_SCRIPTS_CODE => {
            let (_, engine, fa) = scan_font_engine_in(st)?;
            st.scanner.cur_val = if uses_opentype(fa, engine) {
                unsafe { ot_font_get(m, engine) }
            } else {
                0
            };
        }
        XETEX_OT_COUNT_LANGUAGES_CODE | XETEX_OT_SCRIPT_CODE => {
            let (f, engine, fa) = scan_font_engine_in(st)?;
            if uses_opentype(fa, engine) {
                scan_int_in(st)?;
                st.scanner.cur_val = unsafe { ot_font_get_1(m, engine, st.scanner.cur_val) };
            } else {
                font_query_error_in(st, m, f, c"; not an OpenType Layout font", -1)?;
            }
        }
        XETEX_OT_COUNT_FEATURES_CODE | XETEX_OT_LANGUAGE_CODE => {
            let (f, engine, fa) = scan_font_engine_in(st)?;
            if uses_opentype(fa, engine) {
                scan_int_in(st)?;
                let k = st.scanner.cur_val;
                scan_int_in(st)?;
                st.scanner.cur_val = unsafe { ot_font_get_2(m, engine, k, st.scanner.cur_val) };
            } else {
                font_query_error_in(st, m, f, c"; not an OpenType Layout font", -1)?;
            }
        }
        XETEX_OT_FEATURE_CODE => {
            let (f, engine, fa) = scan_font_engine_in(st)?;
            if uses_opentype(fa, engine) {
                scan_int_in(st)?;
                let k = st.scanner.cur_val;
                scan_int_in(st)?;
                let kk = st.scanner.cur_val;
                scan_int_in(st)?;
                st.scanner.cur_val = unsafe { ot_font_get_3(m, engine, k, kk, st.scanner.cur_val) };
            } else {
                font_query_error_in(st, m, f, c"; not an OpenType Layout font", -1)?;
            }
        }
        XETEX_MAP_CHAR_TO_GLYPH_CODE => {
            let f = st.eqtb.b32s1(CUR_FONT_LOC);
            if is_native_font(st, f) {
                scan_int_in(st)?;
                st.scanner.cur_val =
                    unsafe { crate::ext::map_char_to_glyph(&st.fonts, f, st.scanner.cur_val) }?;
            } else {
                font_query_error_in(st, m, f, c"; not a native platform font", 0)?;
            }
        }
        XETEX_GLYPH_INDEX_CODE => {
            let f = st.eqtb.b32s1(CUR_FONT_LOC);
            if is_native_font(st, f) {
                crate::io::scan_and_pack_name_in(st)?;
                let name = name_of_file_ptr(st);
                st.scanner.cur_val = unsafe { crate::ext::map_glyph_to_index(&st.fonts, f, name) }?;
            } else {
                font_query_error_in(st, m, f, c"; not a native platform font", 0)?;
            }
        }
        XETEX_FONT_TYPE_CODE => {
            let (_, engine, fa) = scan_font_engine_in(st)?;
            st.scanner.cur_val = if fa == AAT_FONT_FLAG {
                1
            } else if uses_opentype(fa, engine) {
                2
            } else if uses_graphite(fa, engine) {
                3
            } else {
                0
            };
        }
        XETEX_FIRST_CHAR_CODE | XETEX_LAST_CHAR_CODE => {
            scan_font_ident_in(st)?;
            let f = st.scanner.cur_val;
            let first = m == XETEX_FIRST_CHAR_CODE;
            st.scanner.cur_val = if is_native_font(st, f) {
                unsafe { crate::ext::get_font_char_range(&st.fonts, f, first as i32) }?
            } else if first {
                st.fonts.font_bc[f as usize] as i32
            } else {
                st.fonts.font_ec[f as usize] as i32
            };
        }
        PDF_LAST_X_POS_CODE => st.scanner.cur_val = st.shipout.pdf_last_x_pos,
        PDF_LAST_Y_POS_CODE => st.scanner.cur_val = st.shipout.pdf_last_y_pos,
        XETEX_PDF_PAGE_COUNT_CODE => {
            crate::io::scan_and_pack_name_in(st)?;
            let name = name_of_file_ptr(st);
            st.scanner.cur_val = unsafe { crate::pic::count_pdf_file_pages_named(name) }?;
        }
        CURRENT_GROUP_LEVEL_CODE => st.scanner.cur_val = st.save_stack.cur_level as i32 - 1,
        CURRENT_GROUP_TYPE_CODE => st.scanner.cur_val = st.save_stack.cur_group as i32,
        CURRENT_IF_LEVEL_CODE => {
            let mut q = st.scanner.cond_ptr;
            let mut depth = 0;
            while !q.is_null() {
                depth += 1;
                q = q.link(&st.mem);
            }
            st.scanner.cur_val = depth;
        }
        CURRENT_IF_TYPE_CODE => {
            st.scanner.cur_val = if st.scanner.cond_ptr.is_null() {
                0
            } else if (st.scanner.cur_if as i32) < UNLESS_CODE {
                st.scanner.cur_if as i32 + 1
            } else {
                -(st.scanner.cur_if as i32 - 31)
            };
        }
        CURRENT_IF_BRANCH_CODE => {
            st.scanner.cur_val = match st.scanner.if_limit {
                OR_CODE | ELSE_CODE => 1,
                FI_CODE => -1,
                _ => 0,
            };
        }
        GLUE_STRETCH_ORDER_CODE | GLUE_SHRINK_ORDER_CODE => {
            scan_normal_glue_in(st)?;
            let q = st.scanner.cur_val;
            let spec = glue_spec(q);
            st.scanner.cur_val = if m == GLUE_STRETCH_ORDER_CODE {
                spec.stretch_order(&st.mem) as i32
            } else {
                spec.shrink_order(&st.mem) as i32
            };
            crate::mem::delete_glue_ref(&mut st.mem, q);
        }
        _ => {}
    }
    st.scanner.cur_val_level = INT_VAL;
    Ok(())
}

fn fetch_last_item_dimen_in(st: &mut EngineState, m: i32) -> Result<(), SccAbort> {
    match m {
        XETEX_GLYPH_BOUNDS_CODE => {
            let f = st.eqtb.b32s1(CUR_FONT_LOC);
            if is_native_font(st, f) {
                scan_int_in(st)?;
                let edge = st.scanner.cur_val;
                if !(1..=4).contains(&edge) {
                    error_here_with_diagnostic_in(
                        st,
                        c"\\\\XeTeXglyphbounds requires an edge index from 1 to 4;".as_ptr(),
                    );
                    print_nl_cstr_in(st, c"I don't know anything about edge ".as_ptr());
                    print_int_in(st, edge);
                    capture_to_diagnostic_in(st, std::ptr::null_mut());
                    scc_err(error_in(st))?;
                    st.scanner.cur_val = 0;
                } else {
                    scan_int_in(st)?;
                    st.scanner.cur_val = unsafe {
                        crate::ext::get_glyph_bounds(&st.fonts, f, edge, st.scanner.cur_val)
                    }?;
                }
            } else {
                font_query_error_in(st, m, f, c"; not a native platform font", 0)?;
            }
        }
        FONT_CHAR_WD_CODE | FONT_CHAR_HT_CODE | FONT_CHAR_DP_CODE | FONT_CHAR_IC_CODE => {
            scan_font_ident_in(st)?;
            let f = st.scanner.cur_val;
            scan_usv_num_in(st)?;
            let c = st.scanner.cur_val;
            st.scanner.cur_val = if is_native_font(st, f) {
                let fonts = &st.fonts;
                unsafe {
                    match m {
                        FONT_CHAR_WD_CODE => crate::ext::getnativecharwd(fonts, f, c),
                        FONT_CHAR_HT_CODE => crate::ext::getnativecharht(fonts, f, c),
                        FONT_CHAR_DP_CODE => crate::ext::getnativechardp(fonts, f, c),
                        _ => crate::ext::getnativecharic(fonts, f, c),
                    }
                }?
            } else if st.fonts.font_bc[f as usize] as i32 <= c
                && st.fonts.font_ec[f as usize] as i32 >= c
            {
                let fu = f as usize;
                let ec = crate::fonts::effective_char(&mut st.fonts, true, f, c as u16);
                let info = st.fonts.font_info_b16(st.fonts.char_base[fu] + ec);
                let fonts = &st.fonts;
                match m {
                    FONT_CHAR_WD_CODE => {
                        fonts.font_info_b32s1(fonts.width_base[fu] + info.s3 as i32)
                    }
                    FONT_CHAR_HT_CODE => {
                        fonts.font_info_b32s1(fonts.height_base[fu] + info.s2 as i32 / 16)
                    }
                    FONT_CHAR_DP_CODE => {
                        fonts.font_info_b32s1(fonts.depth_base[fu] + info.s2 as i32 % 16)
                    }
                    _ => fonts.font_info_b32s1(fonts.italic_base[fu] + info.s1 as i32 / 4),
                }
            } else {
                0
            };
        }
        PAR_SHAPE_LENGTH_CODE | PAR_SHAPE_INDENT_CODE | PAR_SHAPE_DIMEN_CODE => {
            let mut q = m - PAR_SHAPE_LENGTH_CODE;
            scan_int_in(st)?;
            let par_shape = st.eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
            if par_shape == TEX_NULL || st.scanner.cur_val <= 0 {
                st.scanner.cur_val = 0;
            } else {
                if q == 2 {
                    q = st.scanner.cur_val % 2;
                    st.scanner.cur_val = (st.scanner.cur_val + q) / 2;
                }
                let lines = st.mem.b32s0(par_shape);
                if st.scanner.cur_val > lines {
                    st.scanner.cur_val = lines;
                }
                st.scanner.cur_val = st.mem.b32s1(par_shape + 2 * st.scanner.cur_val - q);
            }
        }
        GLUE_STRETCH_CODE | GLUE_SHRINK_CODE => {
            scan_normal_glue_in(st)?;
            let q = st.scanner.cur_val;
            let spec = glue_spec(q);
            st.scanner.cur_val = if m == GLUE_STRETCH_CODE {
                spec.stretch(&st.mem).0
            } else {
                spec.shrink(&st.mem).0
            };
            crate::mem::delete_glue_ref(&mut st.mem, q);
        }
        _ => {}
    }
    st.scanner.cur_val_level = DIMEN_VAL;
    Ok(())
}

/// `\lastpenalty`, `\lastkern`, `\lastskip` and `\lastnodetype`.
fn fetch_last_node_item_in(st: &mut EngineState, m: i32) {
    let want_type = m == LAST_NODE_TYPE_CODE;
    let level = m as u8;
    st.scanner.cur_val = 0;
    let head = st.input.cur_list.head.0;
    let mut tx = st.input.cur_list.tail.0;
    let mut node = (tx < st.mem.hi_mem_min).then(|| st.mem.classify(MemIdx::new(tx)));
    if let Some(Ok(AnyNode::Math(math))) = node {
        if math.subtype(&st.mem) == END_M_CODE {
            let mut r = head;
            loop {
                let q = r;
                r = st.mem.b32s1(q);
                if r == tx {
                    tx = q;
                    break;
                }
            }
            node = (tx < st.mem.hi_mem_min).then(|| st.mem.classify(MemIdx::new(tx)));
        }
    }
    if want_type {
        st.scanner.cur_val_level = INT_VAL;
        if tx == head || st.input.cur_list.mode == 0 {
            st.scanner.cur_val = -1;
        }
    } else {
        st.scanner.cur_val_level = level;
    }
    match node {
        Some(_) if st.input.cur_list.mode != 0 && want_type => {
            let node_type = st.mem.b16s1(tx);
            st.scanner.cur_val = if node_type <= UNSET_NODE {
                node_type as i32 + 1
            } else {
                UNSET_NODE as i32 + 2
            };
        }
        Some(node) if st.input.cur_list.mode != 0 => match (level, node) {
            (INT_VAL, Ok(AnyNode::Penalty(p))) => st.scanner.cur_val = p.penalty(&st.mem),
            (DIMEN_VAL, Ok(AnyNode::Kern(k))) => st.scanner.cur_val = k.width(&st.mem).0,
            (GLUE_VAL, Ok(AnyNode::Glue(g))) => {
                st.scanner.cur_val = g.glue_ptr(&st.mem).0;
                if g.subtype(&st.mem) == MU_GLUE {
                    st.scanner.cur_val_level = MU_VAL;
                }
            }
            _ => {}
        },
        _ => {
            if st.input.cur_list.mode == VMODE && tx == head {
                if want_type {
                    st.scanner.cur_val = st.shipout.last_node_type;
                    return;
                }
                match level {
                    INT_VAL => st.scanner.cur_val = st.shipout.last_penalty,
                    DIMEN_VAL => st.scanner.cur_val = st.shipout.last_kern,
                    GLUE_VAL => {
                        if st.shipout.last_glue.0 != MAX_HALFWORD {
                            st.scanner.cur_val = st.shipout.last_glue.0;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Scans an internal quantity of at most `level`, negated if `negative`.
pub(crate) fn scan_something_internal_in(
    st: &mut EngineState,
    level: u8,
    negative: bool,
) -> Result<(), SccAbort> {
    loop {
        let m = st.scanner.cur_chr;
        match st.scanner.cur_cmd {
            DEF_CODE => fetch_def_code_in(st, m)?,
            XETEX_DEF_CODE => fetch_xetex_def_code_in(st, m)?,
            TOKS_REGISTER | ASSIGN_TOKS | DEF_FAMILY | SET_FONT | DEF_FONT => {
                fetch_token_list_in(st, m, level)?
            }
            ASSIGN_INT => set_cur_val(st, st.eqtb.b32s1(m), INT_VAL),
            ASSIGN_DIMEN => set_cur_val(st, st.eqtb.b32s1(m), DIMEN_VAL),
            ASSIGN_GLUE => set_cur_val(st, st.eqtb.b32s1(m), GLUE_VAL),
            ASSIGN_MU_GLUE => set_cur_val(st, st.eqtb.b32s1(m), MU_VAL),
            SET_AUX => {
                if st.input.cur_list.mode.abs() as i32 != m {
                    error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
                    print_cmd_chr_in(st, SET_AUX, m);
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
                    scc_err(error_in(st))?;
                    set_cur_val(st, 0, if level != TOK_VAL { DIMEN_VAL } else { INT_VAL });
                } else if m == VMODE as i32 {
                    set_cur_val(st, st.input.cur_list.aux_s1(), DIMEN_VAL);
                } else {
                    set_cur_val(st, st.input.cur_list.aux_s0(), INT_VAL);
                }
            }
            SET_PREV_GRAF => {
                if st.input.cur_list.mode == 0 {
                    set_cur_val(st, 0, INT_VAL);
                } else {
                    let np = st.input.nest_ptr as usize;
                    st.input.nest[np] = st.input.cur_list;
                    let mut p = np;
                    while st.input.nest[p].mode.abs() != VMODE {
                        p -= 1;
                    }
                    set_cur_val(st, st.input.nest[p].prev_graf, INT_VAL);
                }
            }
            SET_PAGE_INT => {
                let v = match m {
                    0 => st.shipout.dead_cycles,
                    2 => st.errors.interaction as i32,
                    _ => st.shipout.insert_penalties,
                };
                set_cur_val(st, v, INT_VAL);
            }
            SET_PAGE_DIMEN => {
                let v = if st.shipout.page_contents == 0 && !st.shipout.output_active {
                    if m == 0 {
                        MAX_HALFWORD
                    } else {
                        0
                    }
                } else {
                    st.shipout.page_so_far[m as usize]
                };
                set_cur_val(st, v, DIMEN_VAL);
            }
            SET_SHAPE => {
                if m > LOCAL_BASE + LOCAL_PAR_SHAPE {
                    scan_int_in(st)?;
                    let penalties = st.eqtb.b32s1(m);
                    if penalties == TEX_NULL || st.scanner.cur_val < 0 {
                        st.scanner.cur_val = 0;
                    } else {
                        let count = st.mem.b32s1(penalties + 1);
                        if st.scanner.cur_val > count {
                            st.scanner.cur_val = count;
                        }
                        st.scanner.cur_val = st.mem.b32s1(penalties + st.scanner.cur_val + 1);
                    }
                } else {
                    let par_shape = st.eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
                    st.scanner.cur_val = if par_shape == TEX_NULL {
                        0
                    } else {
                        st.mem.b32s0(par_shape)
                    };
                }
                st.scanner.cur_val_level = INT_VAL;
            }
            SET_BOX_DIMEN => {
                scan_register_num_in(st)?;
                let v = match fetch_box(st, st.scanner.cur_val) {
                    None => Scaled(0),
                    Some(b) if m == WIDTH_OFFSET => b.width(&st.mem),
                    Some(b) if m == DEPTH_OFFSET => b.depth(&st.mem),
                    Some(b) => b.height(&st.mem),
                };
                set_cur_val(st, v.0, DIMEN_VAL);
            }
            CHAR_GIVEN | MATH_GIVEN | XETEX_MATH_GIVEN => set_cur_val(st, m, INT_VAL),
            ASSIGN_FONT_DIMEN => {
                find_font_dimen_in(st, false)?;
                st.fonts.set_font_info_b32s1(st.fonts.fmem_ptr, 0);
                set_cur_val(st, st.fonts.font_info_b32s1(st.scanner.cur_val), DIMEN_VAL);
            }
            ASSIGN_FONT_INT => {
                scan_font_ident_in(st)?;
                let f = st.scanner.cur_val;
                match m {
                    0 => set_cur_val(st, st.fonts.hyphen_char[f as usize], INT_VAL),
                    1 => set_cur_val(st, st.fonts.skew_char[f as usize], INT_VAL),
                    _ => {
                        if is_native_font(st, f) {
                            scan_glyph_number_in(st, f)?;
                        } else {
                            scan_char_num_in(st)?;
                        }
                        let code = st.scanner.cur_val as u32;
                        match m {
                            LP_CODE_BASE => {
                                set_cur_val(st, unsafe { get_cp_code(f, code, LEFT_SIDE) }, INT_VAL)
                            }
                            RP_CODE_BASE => set_cur_val(
                                st,
                                unsafe { get_cp_code(f, code, RIGHT_SIDE) },
                                INT_VAL,
                            ),
                            _ => {}
                        }
                    }
                }
            }
            REGISTER => fetch_register_in(st, m)?,
            LAST_ITEM => {
                if m < INPUT_LINE_NO_CODE {
                    fetch_last_node_item_in(st, m);
                } else if m < XETEX_DIM {
                    fetch_last_item_int_in(st, m)?;
                } else if m < ETEX_GLUE {
                    fetch_last_item_dimen_in(st, m)?;
                } else {
                    if m < ETEX_MU {
                        if m == MU_TO_GLUE_CODE {
                            scan_mu_glue_in(st)?;
                        }
                        st.scanner.cur_val_level = GLUE_VAL;
                    } else if m < ETEX_EXPR {
                        if m == GLUE_TO_MU_CODE {
                            scan_normal_glue_in(st)?;
                        }
                        st.scanner.cur_val_level = MU_VAL;
                    } else {
                        st.scanner.cur_val_level = (m - ETEX_EXPR) as u8;
                        scan_expr_in(st)?;
                    }
                    while st.scanner.cur_val_level > level {
                        if st.scanner.cur_val_level == GLUE_VAL {
                            glue_to_width_in(st);
                        } else if st.scanner.cur_val_level == MU_VAL {
                            scc_err(mu_error_in(st))?;
                        }
                        st.scanner.cur_val_level -= 1;
                    }
                    if negative {
                        if st.scanner.cur_val_level >= GLUE_VAL {
                            let old = negate_glue_in(st)?;
                            crate::mem::delete_glue_ref(&mut st.mem, old);
                        } else {
                            st.scanner.cur_val = -st.scanner.cur_val;
                        }
                    }
                    return Ok(());
                }
            }
            IGNORE_SPACES => {
                if m == 1 {
                    get_primitive_token_in(st)?;
                    continue;
                }
            }
            _ => {
                error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
                print_cmd_chr_in(st, st.scanner.cur_cmd, m);
                print_cstr_in(st, c"' after ".as_ptr());
                print_esc_cstr_in(st, c"the".as_ptr());
                capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 1;
                st.errors.help_line[0] =
                    c"I'm forgetting what you said and using zero instead.".as_ptr();
                scc_err(error_in(st))?;
                set_cur_val(st, 0, if level != TOK_VAL { DIMEN_VAL } else { INT_VAL });
            }
        }
        break;
    }

    while st.scanner.cur_val_level > level {
        if st.scanner.cur_val_level == GLUE_VAL {
            st.scanner.cur_val = glue_spec(st.scanner.cur_val).width(&st.mem).0;
        } else if st.scanner.cur_val_level == MU_VAL {
            scc_err(mu_error_in(st))?;
        }
        st.scanner.cur_val_level -= 1;
    }

    if negative {
        if st.scanner.cur_val_level >= GLUE_VAL {
            negate_glue_in(st)?;
        } else {
            st.scanner.cur_val = -st.scanner.cur_val;
        }
    } else if (GLUE_VAL..=MU_VAL).contains(&(st.scanner.cur_val_level)) {
        let spec = glue_spec(st.scanner.cur_val);
        let refs = spec.ref_count(&st.mem);
        spec.set_ref_count(&mut st.mem, refs + 1);
    }
    Ok(())
}

pub(crate) fn scan_font_ident_in(st: &mut EngineState) -> Result<(), SccAbort> {
    get_x_non_blank_in(st)?;
    let f = match st.scanner.cur_cmd {
        DEF_FONT => st.eqtb.b32s1(CUR_FONT_LOC),
        SET_FONT => st.scanner.cur_chr,
        DEF_FAMILY => {
            let m = st.scanner.cur_chr;
            scan_math_fam_int_in(st)?;
            st.eqtb.b32s1(m + st.scanner.cur_val)
        }
        _ => {
            error_here_with_diagnostic_in(st, c"Missing font identifier".as_ptr());
            capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 2;
            st.errors.help_line[1] = c"I was looking for a control sequence whose".as_ptr();
            st.errors.help_line[0] = c"current meaning has been defined by \\font.".as_ptr();
            scc_err(back_error_in(st)?)?;
            FONT_BASE
        }
    };
    st.scanner.cur_val = f;
    Ok(())
}

pub(crate) fn find_font_dimen_in(st: &mut EngineState, writing: bool) -> Result<(), SccAbort> {
    scan_int_in(st)?;
    let n = st.scanner.cur_val;
    scan_font_ident_in(st)?;
    let f = st.scanner.cur_val;
    let fu = f as usize;

    if n <= 0 {
        st.scanner.cur_val = st.fonts.fmem_ptr;
    } else {
        if writing
            && (SPACE_CODE..=SPACE_SHRINK_CODE).contains(&n)
            && st.fonts.font_glue[fu] != TEX_NULL
        {
            crate::mem::delete_glue_ref(&mut st.mem, st.fonts.font_glue[fu]);
            st.fonts.font_glue[fu] = TEX_NULL;
        }

        if n <= st.fonts.font_params[fu] {
            st.scanner.cur_val = n + st.fonts.param_base[fu];
        } else if f < st.fonts.font_ptr {
            st.scanner.cur_val = st.fonts.fmem_ptr;
        } else {
            // Only the most recently loaded font can grow its parameter area.
            loop {
                if st.fonts.fmem_ptr == st.fonts.font_mem_size {
                    return Err(SccAbort::Error(crate::errors::ErrorOutcome::Overflow {
                        what: c"font memory",
                        n: st.fonts.font_mem_size,
                    }));
                }
                st.fonts.set_font_info_b32s1(st.fonts.fmem_ptr, 0);
                st.fonts.fmem_ptr += 1;
                st.fonts.font_params[fu] += 1;
                if n == st.fonts.font_params[fu] {
                    break;
                }
            }
            st.scanner.cur_val = st.fonts.fmem_ptr - 1;
        }
    }

    if st.scanner.cur_val == st.fonts.fmem_ptr {
        error_here_with_diagnostic_in(st, c"Font ".as_ptr());
        crate::output::print_esc_in(st, st.eqtb.hash_s1(FONT_ID_BASE + f));
        print_cstr_in(st, c" has only ".as_ptr());
        print_int_in(st, st.fonts.font_params[fu]);
        print_cstr_in(st, c" fontdimen parameters".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] = c"To increase the number of font parameters, you must".as_ptr();
        st.errors.help_line[0] =
            c"use \\fontdimen immediately after the \\font is loaded.".as_ptr();
        scc_err(error_in(st))?;
    }
    Ok(())
}

pub(crate) fn scan_glyph_number_in(st: &mut EngineState, f: i32) -> Result<(), SccAbort> {
    if scan_keyword_in(st, c"/".as_ptr())? {
        crate::io::scan_and_pack_name_in(st)?;
        let name = name_of_file_ptr(st);
        let v = unsafe { crate::ext::map_glyph_to_index(&st.fonts, f, name) }?;
        set_cur_val(st, v, INT_VAL);
    } else if scan_keyword_in(st, c"u".as_ptr())? {
        scan_char_num_in(st)?;
        let v = unsafe { crate::ext::map_char_to_glyph(&st.fonts, f, st.scanner.cur_val) }?;
        set_cur_val(st, v, INT_VAL);
    } else {
        scan_int_in(st)?;
    }
    Ok(())
}

pub(crate) fn get_token_in(st: &mut EngineState) -> Result<(), SccAbort> {
    st.eqtb.no_new_control_sequence = false;
    get_next_scc(st)?;
    st.eqtb.no_new_control_sequence = true;
    pack_cur_tok(st);
    Ok(())
}

/// Scans a balanced text into a list at `link(temp_head)`; `cur_val` is its tail.
pub(crate) fn scan_general_text_in(st: &mut EngineState) -> Result<(), SccAbort> {
    let saved_status = st.scanner.scanner_status;
    let saved_warning_index = st.scanner.warning_index;
    let saved_def_ref = st.scanner.def_ref;
    st.scanner.scanner_status = ABSORBING;
    st.scanner.warning_index = st.scanner.cur_cs;
    let mut p = new_def_ref_in(st)?;
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
                    break;
                }
            }
        }
        store_new_token(st, &mut p, st.scanner.cur_tok)?;
    }
    let def_ref = st.scanner.def_ref;
    let list = def_ref.link(&st.mem);
    crate::mem::free_avail(&mut st.mem, def_ref);
    st.scanner.cur_val = if list.is_null() { TEMP_HEAD.0 } else { p.0 };
    TEMP_HEAD.set_link(&mut st.mem, list);
    st.scanner.scanner_status = saved_status;
    st.scanner.warning_index = saved_warning_index;
    st.scanner.def_ref = saved_def_ref;
    Ok(())
}

pub(crate) fn scan_pdf_ext_toks_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_toks_in(st, false, true)?;
    Ok(())
}

fn scan_toks_to_string_in(st: &mut EngineState) -> Result<i32, SccAbort> {
    scan_toks_in(st, false, true)?;
    let def_ref = st.scanner.def_ref.0;
    let s = tokens_to_string_in(st, def_ref).map_err(SccAbort::TokensToString)?;
    crate::mem::delete_token_ref(&mut st.mem, st.scanner.def_ref);
    Ok(s)
}

/// `\strcmp`: compares two general texts as strings.
pub(crate) fn compare_strings_in(st: &mut EngineState) -> Result<(), SccAbort> {
    let save_cur_cs = st.scanner.cur_cs;
    let s1 = scan_toks_to_string_in(st)?;
    st.scanner.cur_cs = save_cur_cs;
    let s2 = scan_toks_to_string_in(st)?;

    let range = |s: i32| {
        let start = st.strings.str_start[(s - TOO_BIG_CHAR) as usize] as usize;
        let end = st.strings.str_start[(s + 1 - TOO_BIG_CHAR) as usize] as usize;
        &st.strings.str_pool[start..end]
    };
    let result = match range(s1).cmp(range(s2)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    };

    crate::string_pool::flush_str(&mut st.strings, s2);
    crate::string_pool::flush_str(&mut st.strings, s1);
    set_cur_val(st, result, INT_VAL);
    Ok(())
}
