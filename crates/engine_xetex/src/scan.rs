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
    ACTIVE_BASE, ACTIVE_MATH_CHAR, BIGGEST_CHAR, BIGGEST_USV, CHAR_CLASS_LIMIT, CLOSED, COUNT_BASE,
    CS_TOKEN_FLAG, DEL_CODE_BASE, END_M_CODE, EQTB_SIZE, FIL, FILLL, FONT_BASE, FONT_ID_BASE,
    FROZEN_CONTROL_SEQUENCE, FROZEN_CR, FROZEN_ENDV, FROZEN_FI, FROZEN_PRIMITIVE,
    FROZEN_PROTECTION, FROZEN_RELAX, INT_PAR_CUR_FAM, INT_PAR_END_LINE_CHAR, INT_PAR_MAG,
    INT_PAR_NEW_LINE_CHAR, INT_PAR_TRACING_IFS, INT_PAR_TRACING_NESTING,
    INT_PAR_TRACING_SCAN_TOKENS, INT_PAR_XETEX_HYPHENATABLE_LENGTH, JUST_OPEN, LEFT_SIDE,
    LOCAL_BASE, LOCAL_EVERY_EOF, LOCAL_PAR_SHAPE, LOCAL_TECTONIC_CODA, LOCAL_XETEX_INTER_CHAR_TOKS,
    MATH_CHAR, MATH_CODE_BASE, MAX_HALFWORD, MU_GLUE, MU_SKIP_BASE, NORMAL, NULL_CS,
    NUMBER_MATH_FAMILIES, PRIM_EQTB_BASE, RIGHT_SIDE, SCALED_BASE, SELECTOR_LOG_ONLY,
    SELECTOR_NEW_STRING, SF_CODE_BASE, SINGLE_BASE, SKIP_BASE, TEX_INFINITY, TEX_NULL,
    TOO_BIG_CHAR, UNDEFINED_PRIMITIVE, UNLESS_CODE, VMODE,
};
use crate::errors::Abort;
use crate::errors::{error, int_error, mu_error};
use crate::expand::{expand, macro_call, ELSE_CODE, FI_CODE, OR_CODE};
use crate::ext::{
    aat_font_get, aat_font_get_1, aat_font_get_2, aat_font_get_named, aat_font_get_named_1,
    gr_font_get_named, gr_font_get_named_1, ot_font_get, ot_font_get_1, ot_font_get_2,
    ot_font_get_3,
};
use crate::fonts::{
    AAT_FONT_FLAG, OTGR_FONT_FLAG, QUAD_CODE, SPACE_CODE, SPACE_SHRINK_CODE, X_HEIGHT_CODE,
};
use crate::input::{
    back_error, back_input, begin_token_list, ins_error, BACKED_UP, EVERY_EOF_TEXT, INSERTED,
    LEFT_BRACE_LIMIT, MID_LINE, NEW_LINE, PARAMETER, RIGHT_BRACE_LIMIT, SKIP_BLANKS,
    TECTONIC_CODA_TEXT, TOKEN_LIST, V_TEMPLATE,
};
use crate::main_control::{LP_CODE_BASE, RP_CODE_BASE};
use crate::mem::{BACKUP_HEAD, OMIT_TEMPLATE, TEMP_HEAD};
use crate::node::{
    AlignRecord, AnyBox, AnyNode, ExprNode, GlueSpecNode, MathField, NodePtr, PseudoLineNode,
    RuleDimen, RuleNode, SaNode,
};
use crate::output::{
    capture_to_diagnostic, error_here_with_diagnostic, print, print_cmd_chr, print_err, print_esc,
    print_int, print_nl,
};
use crate::packaging::{ADDITIONAL, EXACTLY};
use crate::state::{
    EngineState, EqtbState, ErrorState, GroupCode, InputClusterState, MemState, ScannerState,
};
use crate::units::{MemIdx, MemPtr, NodeType, Scaled};

extern "C" {
    fn usingGraphite(engine: *mut libc::c_void) -> bool;
    fn usingOpenType(engine: *mut libc::c_void) -> bool;
    fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;
}

pub(crate) const NORMAL_SCANNER: u8 = 0;
pub(crate) const SKIPPING: u8 = 1;
pub(crate) const DEFINING: u8 = 2;
pub(crate) const MATCHING: u8 = 3;
pub(crate) const ALIGNING: u8 = 4;
pub(crate) const ABSORBING: u8 = 5;

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
pub(crate) const LEFT_BRACE_TOKEN: i32 = 0x20_0000;
pub(crate) const RIGHT_BRACE_TOKEN: i32 = 0x40_0000;
pub(crate) const MATH_SHIFT_TOKEN: i32 = 0x60_0000;
pub(crate) const TAB_TOKEN: i32 = 0x80_0000;
pub(crate) const OUT_PARAM_TOKEN: i32 = 0xA0_0000;
pub(crate) const SPACE_TOKEN: i32 = 0x140_0020;
pub(crate) const LETTER_TOKEN: i32 = 0x160_0000;
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
const DEFAULT_RULE: i32 = 26214;

const HYPHENATABLE_LENGTH_LIMIT: i32 = 4095;
const ETEX_VERSION: i32 = 2;
const XETEX_VERSION: i32 = 0;

const EXPR_NONE: i16 = 0;
const EXPR_ADD: i16 = 1;
const EXPR_SUB: i16 = 2;
const EXPR_MULT: i16 = 3;
const EXPR_DIV: i16 = 4;
const EXPR_SCALE: i16 = 5;
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
    err: &str,
    value: i32,
    help1: &'static str,
    help0: &'static str,
) -> Result<(), Abort> {
    let errmsg = error_here_with_diagnostic(st, err);
    unsafe { diag_append_str(errmsg, &format!(" ({value})")) };
    capture_to_diagnostic(st, std::ptr::null_mut());
    st.errors.help(&[help1, help0]);
    int_error(st, value)
}

fn int_range_error(st: &mut EngineState, err: &str, help1: &'static str) -> Result<(), Abort> {
    bad_int_error(
        st,
        err,
        st.scanner.cur_val,
        help1,
        "I changed this one to zero.",
    )?;
    st.scanner.cur_val = 0;
    Ok(())
}

fn scan_int_up_to(
    st: &mut EngineState,
    max: i32,
    err: &str,
    help: &'static str,
) -> Result<(), Abort> {
    scan_int(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > max {
        int_range_error(st, err, help)?;
    }
    Ok(())
}

pub(crate) fn scan_char_class(st: &mut EngineState) -> Result<(), Abort> {
    scan_int_up_to(
        st,
        CHAR_CLASS_LIMIT,
        "Bad character class",
        "A character class must be between 0 and 4096.",
    )
}

pub(crate) fn scan_char_class_not_ignored(st: &mut EngineState) -> Result<(), Abort> {
    scan_int_up_to(
        st,
        CHAR_CLASS_LIMIT,
        "Bad character class",
        "A class for inter-character transitions must be between 0 and 4095.",
    )
}

pub(crate) fn scan_eight_bit_int(st: &mut EngineState) -> Result<(), Abort> {
    scan_int_up_to(
        st,
        255,
        "Bad register code",
        "A register code or char class must be between 0 and 255.",
    )
}

pub(crate) fn scan_math_class_int(st: &mut EngineState) -> Result<(), Abort> {
    scan_int_up_to(
        st,
        7,
        "Bad math class",
        "Since I expected to read a number between 0 and 7,",
    )
}

pub(crate) fn scan_math_fam_int(st: &mut EngineState) -> Result<(), Abort> {
    scan_int_up_to(
        st,
        NUMBER_MATH_FAMILIES - 1,
        "Bad math family",
        "Since I expected to read a number between 0 and 255,",
    )
}

pub(crate) fn scan_four_bit_int(st: &mut EngineState) -> Result<(), Abort> {
    scan_int_up_to(
        st,
        15,
        "Bad number",
        "Since I expected to read a number between 0 and 15,",
    )
}

pub(crate) fn scan_four_bit_int_or_18(st: &mut EngineState) -> Result<(), Abort> {
    scan_int(st)?;
    if st.scanner.cur_val < 0 || (st.scanner.cur_val > 15 && st.scanner.cur_val != 18) {
        int_range_error(
            st,
            "Bad number",
            "Since I expected to read a number between 0 and 15,",
        )?;
    }
    Ok(())
}

pub(crate) fn scan_fifteen_bit_int(st: &mut EngineState) -> Result<(), Abort> {
    scan_int_up_to(
        st,
        32767,
        "Bad mathchar",
        "A mathchar number must be between 0 and 32767.",
    )
}

pub(crate) fn scan_char_num(st: &mut EngineState) -> Result<(), Abort> {
    scan_int_up_to(
        st,
        BIGGEST_CHAR,
        "Bad character code",
        "A character number must be between 0 and 65535.",
    )
}

pub(crate) fn scan_usv_num(st: &mut EngineState) -> Result<(), Abort> {
    scan_int_up_to(
        st,
        BIGGEST_USV,
        "Bad character code",
        "A Unicode scalar value must be between 0 and \"10FFFF.",
    )
}

pub(crate) fn scan_delimiter_int(st: &mut EngineState) -> Result<(), Abort> {
    scan_int_up_to(
        st,
        0x7FF_FFFF,
        "Bad delimiter code",
        "A numeric delimiter code must be between 0 and 2^{27}-1.",
    )
}

pub(crate) fn scan_register_num(st: &mut EngineState) -> Result<(), Abort> {
    scan_int(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > st.scanner.max_reg_num {
        int_range_error(
            st,
            "Bad register code",
            "A register number must be between 0 and 32767.",
        )?;
    }
    Ok(())
}

pub(crate) fn scan_xetex_math_char_int(st: &mut EngineState) -> Result<(), Abort> {
    scan_int(st)?;
    if math_char_fn(st.scanner.cur_val) == ACTIVE_MATH_CHAR {
        if st.scanner.cur_val != ACTIVE_MATH_CHAR {
            bad_int_error(
                st,
                "Bad active XeTeX math code",
                st.scanner.cur_val,
                "Since I ignore class and family for active math chars,",
                "I changed this one to \"1FFFFF.",
            )?;
            st.scanner.cur_val = ACTIVE_MATH_CHAR;
        }
    } else if math_char_fn(st.scanner.cur_val) > BIGGEST_USV {
        int_range_error(
            st,
            "Bad XeTeX math character code",
            "Since I expected a character number between 0 and \"10FFFF,",
        )?;
    }
    Ok(())
}

impl EqtbState {
    /// `\XeTeXhyphenatablelength`, capped at the supported maximum.
    pub fn max_hyphenatable_length(&self) -> i32 {
        self.int_par(INT_PAR_XETEX_HYPHENATABLE_LENGTH)
            .min(HYPHENATABLE_LENGTH_LIMIT)
    }
}

pub(crate) fn insert_dollar_sign(st: &mut EngineState) -> Result<(), Abort> {
    back_input(st)?;
    st.scanner.cur_tok = MATH_SHIFT_TOKEN + '$' as i32;
    print_err(st, "Missing $ inserted");
    st.errors.help(&[
        "I've inserted a begin-math/end-math symbol since I think",
        "you left one out. Proceed, with fingers crossed.",
    ]);
    ins_error(st)
}

pub(crate) fn scan_optional_equals(st: &mut EngineState) -> Result<(), Abort> {
    get_x_non_blank(st)?;
    if st.scanner.cur_tok != OTHER_TOKEN + '=' as i32 {
        back_input(st)?;
    }
    Ok(())
}

impl MemState {
    /// Whether the macro whose token list starts at `ref_count` is `\protected`.
    #[inline]
    pub(crate) fn is_protected_macro(&self, ref_count: MemPtr) -> bool {
        ref_count.link(self).token(self) == PROTECTED_TOKEN
    }
}

pub(crate) fn get_x_or_protected(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        get_token(st)?;
        if st.scanner.cur_cmd <= MAX_COMMAND {
            return Ok(());
        }
        if st.scanner.cur_cmd >= CALL
            && st.scanner.cur_cmd < END_TEMPLATE
            && st.mem.is_protected_macro(MemPtr(st.scanner.cur_chr))
        {
            return Ok(());
        }
        expand(st)?;
    }
}

pub(crate) fn scan_spec(
    st: &mut EngineState,
    c: GroupCode,
    three_codes: bool,
) -> Result<(), Abort> {
    let s = if three_codes {
        st.save_stack.saved(0)
    } else {
        0
    };
    let spec_code;
    if scan_keyword(st, "to")? {
        spec_code = EXACTLY;
        scan_dimen(st, false, false, false)?;
    } else if scan_keyword(st, "spread")? {
        spec_code = ADDITIONAL;
        scan_dimen(st, false, false, false)?;
    } else {
        spec_code = ADDITIONAL;
        st.scanner.cur_val = 0;
    }
    if three_codes {
        st.save_stack.set_saved(0, s);
        st.save_stack.save_ptr += 1;
    }
    st.save_stack.set_saved(0, spec_code as i32);
    st.save_stack.set_saved(1, st.scanner.cur_val);
    st.save_stack.save_ptr += 2;
    st.save_stack.new_save_level(c, st.mem.line)?;
    scan_left_brace(st)?;
    Ok(())
}

impl ErrorState {
    fn round_decimals(&self, k: i32) -> i32 {
        let a = self.dig[..k as usize]
            .iter()
            .rev()
            .fold(0, |a, &d| (a + d as i32 * 0x2_0000) / 10);
        (a + 1) / 2
    }
}

pub(crate) fn scan_rule_spec(st: &mut EngineState) -> Result<NodePtr<RuleNode>, Abort> {
    let r = st.mem.new_rule()?;
    if st.scanner.cur_cmd == VRULE {
        r.set_width(&mut st.mem, RuleDimen::from_raw(DEFAULT_RULE));
    } else {
        r.set_height(&mut st.mem, RuleDimen::from_raw(DEFAULT_RULE));
        r.set_depth(&mut st.mem, RuleDimen::Set(Scaled(0)));
    }
    loop {
        if scan_keyword(st, "width")? {
            xetex_scan_dimen(st, false, false, false, true)?;
            r.set_width(&mut st.mem, RuleDimen::from_raw(st.scanner.cur_val));
        } else if scan_keyword(st, "height")? {
            xetex_scan_dimen(st, false, false, false, true)?;
            r.set_height(&mut st.mem, RuleDimen::from_raw(st.scanner.cur_val));
        } else if scan_keyword(st, "depth")? {
            xetex_scan_dimen(st, false, false, false, true)?;
            r.set_depth(&mut st.mem, RuleDimen::from_raw(st.scanner.cur_val));
        } else {
            break;
        }
    }
    Ok(r)
}

impl MemState {
    /// Converts `str_pool[b..pool_ptr]` to a token list at `link(temp_head)` and returns its tail.
    pub(crate) fn str_toks_cat(
        &mut self,
        strings: &mut crate::state::StringsState,
        b: i32,
        cat: i16,
    ) -> Result<MemPtr, Abort> {
        if strings.pool_ptr + 1 > strings.pool_size {
            return Err(Abort::PoolOverflow);
        }

        let mut p = TEMP_HEAD;
        p.set_link(self, MemPtr::NULL);
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

            let q = self.get_avail()?;
            p.set_link(self, q);
            q.set_info(self, t);
            p = q;
            k += 1;
        }

        strings.pool_ptr = b;
        Ok(p)
    }
}

/// Appends `\endlinechar` to the line just read and starts reading it.
#[inline]
pub(crate) fn finish_line(st: &mut EngineState) {
    let end_char = st.eqtb.int_par(INT_PAR_END_LINE_CHAR);
    if (0..=255).contains(&end_char) {
        st.input.buffer[st.input.cur_input.limit as usize] = end_char;
    } else {
        st.input.cur_input.limit -= 1;
    }
    st.input.first = st.input.cur_input.limit + 1;
    st.input.cur_input.loc = st.input.cur_input.start;
}

fn close_read_file(st: &mut EngineState, m: usize) -> Result<(), Abort> {
    let slot = std::mem::take(&mut st.input.read_file[m]);
    // SAFETY: closes the handle taken out of the read-file slot.
    if unsafe { crate::io::u_close(slot) } {
        return Err(Abort::InputClose);
    }
    st.input.read_open[m] = CLOSED;
    Ok(())
}

pub(crate) fn read_toks(st: &mut EngineState, n: i32, r: i32, j: i32) -> Result<(), Abort> {
    st.scanner.scanner_status = DEFINING;
    st.scanner.warning_index = r;
    let def_ref = new_def_ref(st)?;
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
        crate::input::begin_file_reading(st)?;
        st.input.cur_input.name = (m + 1) as i32;

        if st.input.read_open[m] == CLOSED {
            return Err(Abort::Internal("terminal input forbidden"));
        } else if st.input.read_open[m] == JUST_OPEN {
            let rf = st.input.read_file_ptr(m);
            if unsafe { crate::io::input_line(st, rf)? } != 0 {
                st.input.read_open[m] = NORMAL as u8;
            } else {
                close_read_file(st, m)?;
            }
        } else {
            let rf = st.input.read_file_ptr(m);
            if unsafe { crate::io::input_line(st, rf)? } == 0 {
                close_read_file(st, m)?;
                if st.scanner.align_state != 1_000_000 {
                    crate::errors::runaway(st);
                    error_here_with_diagnostic(st, "File ended within ");
                    print_esc(st, "read");
                    capture_to_diagnostic(st, std::ptr::null_mut());
                    st.errors.help(&[r"This \read has unbalanced braces."]);
                    st.scanner.align_state = 1_000_000;
                    st.input.cur_input.limit = 0;
                    error(st)?;
                }
            }
        }

        st.input.cur_input.limit = st.input.last;
        finish_line(st);
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
                get_token(st)?;
                if st.scanner.cur_tok == 0 {
                    break;
                }
                if st.scanner.align_state < 1_000_000 {
                    loop {
                        get_token(st)?;
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
        crate::input::end_file_reading(st)?;

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
pub(crate) fn eTeX_enabled(st: &mut EngineState, b: bool, j: u8, k: i32) -> Result<bool, Abort> {
    if !b {
        error_here_with_diagnostic(st, "Improper ");
        print_cmd_chr(st, j, k);
        capture_to_diagnostic(st, std::ptr::null_mut());
        st.errors
            .help(&["Sorry, this optional e-TeX feature has been disabled."]);
        error(st)?;
    }
    Ok(b)
}

pub(crate) fn tokens_to_string(st: &mut EngineState, p: i32) -> Result<i32, Abort> {
    if st.shipout.selector == SELECTOR_NEW_STRING {
        return Err(Abort::PdfError(&[
            "tokens",
            "tokens_to_string() called while selector = new_string",
        ]));
    }
    st.shipout.old_setting = st.shipout.selector as u8;
    st.shipout.selector = SELECTOR_NEW_STRING;
    let arg = MemPtr(p).link(&st.mem);
    let l = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list(st, arg, TEX_NULL, l);
    st.shipout.selector = st.shipout.old_setting as i32;
    st.strings.make_string().map_err(|_| Abort::StringOverflow)
}

pub(crate) fn get_r_token(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        loop {
            get_token(st)?;
            if st.scanner.cur_tok != SPACE_TOKEN {
                break;
            }
        }
        if st.scanner.cur_cs == 0
            || st.scanner.cur_cs > st.eqtb.eqtb_top
            || (st.scanner.cur_cs > FROZEN_CONTROL_SEQUENCE && st.scanner.cur_cs <= EQTB_SIZE)
        {
            print_err(st, "Missing control sequence inserted");
            st.errors.help(&[
                r"Please don't say `\def cs{...}', say `\def\cs{...}'.",
                "I've inserted an inaccessible control sequence so that your",
                "definition will be completed without mixing me up too badly.",
                "You can recover graciously from this error, if you're",
                "careful; see exercise 27.2 in The TeXbook.",
            ]);
            if st.scanner.cur_cs == 0 {
                back_input(st)?;
            }
            st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_PROTECTION;
            ins_error(st)?;
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
fn scan_umath_code(st: &mut EngineState) -> Result<i32, Abort> {
    scan_math_class_int(st)?;
    let class = set_class_fn(st.scanner.cur_val);
    scan_math_fam_int(st)?;
    let family = set_family_fn(st.scanner.cur_val);
    scan_usv_num(st)?;
    Ok(class + family + st.scanner.cur_val)
}

/// `get_x_token` until the result is neither a space nor `\relax`.
pub(crate) fn get_x_non_blank_non_relax(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        get_x_token(st)?;
        if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
            return Ok(());
        }
    }
}

/// Scans one math field into the nucleus-like field `p`.
pub(crate) fn scan_math(st: &mut EngineState, p: NodePtr<MathField>) -> Result<(), Abort> {
    let c: i32;
    'main: loop {
        get_x_non_blank_non_relax(st)?;
        c = loop {
            break match st.scanner.cur_cmd {
                LETTER | OTHER_CHAR | CHAR_GIVEN => {
                    let mc = st.eqtb.math_code(st.scanner.cur_chr);
                    if mc & 0x1FFFFF == ACTIVE_MATH_CHAR {
                        st.scanner.cur_cs = st.scanner.cur_chr + 1;
                        fetch_cs_meaning(st);
                        x_token(st)?;
                        back_input(st)?;
                        continue 'main;
                    }
                    mc
                }
                CHAR_NUM => {
                    scan_char_num(st)?;
                    st.scanner.cur_chr = st.scanner.cur_val;
                    st.scanner.cur_cmd = CHAR_GIVEN;
                    continue;
                }
                MATH_CHAR_NUM => match st.scanner.cur_chr {
                    2 => scan_umath_code(st)?,
                    1 => {
                        scan_xetex_math_char_int(st)?;
                        st.scanner.cur_val
                    }
                    _ => {
                        scan_fifteen_bit_int(st)?;
                        math_code_from_tex(st.scanner.cur_val)
                    }
                },
                MATH_GIVEN => math_code_from_tex(st.scanner.cur_chr),
                XETEX_MATH_GIVEN => st.scanner.cur_chr,
                DELIM_NUM => {
                    if st.scanner.cur_chr == 1 {
                        scan_umath_code(st)?
                    } else {
                        scan_delimiter_int(st)?;
                        math_code_from_tex(st.scanner.cur_val / 4096)
                    }
                }
                _ => {
                    back_input(st)?;
                    scan_left_brace(st)?;
                    st.save_stack.set_saved(0, p.idx().0);
                    st.save_stack.save_ptr += 1;
                    crate::main_control::push_math(st, GroupCode::Math)?;
                    return Ok(());
                }
            };
        };
        break;
    }
    p.set_math_type(&mut st.mem, MATH_CHAR);
    p.set_character(&mut st.mem, (c % 65536) as u16);
    let cur_fam = st.eqtb.int_par(INT_PAR_CUR_FAM);
    let fam = if math_class_fn(c) == 7 && (0..NUMBER_MATH_FAMILIES).contains(&cur_fam) {
        cur_fam as u16
    } else {
        math_fam_fn(c) as u16
    };
    p.set_plane_and_fam(&mut st.mem, fam + (math_char_fn(c) / 65536) as u16 * 256);
    Ok(())
}

pub(crate) fn set_math_char(st: &mut EngineState, c: i32) -> Result<(), Abort> {
    if math_char_fn(c) == ACTIVE_MATH_CHAR {
        st.scanner.cur_cs = st.scanner.cur_chr + 1;
        fetch_cs_meaning(st);
        x_token(st)?;
        back_input(st)?;
    } else {
        let nd = st.mem.new_noad()?;
        nd.nucleus().set_math_type(&mut st.mem, MATH_CHAR);
        let ch = math_char_fn(c);
        nd.nucleus().set_character(&mut st.mem, (ch % 65536) as u16);
        nd.nucleus()
            .set_plane_and_fam(&mut st.mem, math_fam_fn(c) as u16);
        if math_class_fn(c) == 7 {
            let cf = st.eqtb.int_par(INT_PAR_CUR_FAM);
            if (0..NUMBER_MATH_FAMILIES).contains(&cf) {
                nd.nucleus().set_plane_and_fam(&mut st.mem, cf as u16);
            }
            nd.set_noad_type(&mut st.mem, NodeType::OrdNoad);
        } else {
            nd.set_noad_type(
                &mut st.mem,
                NodeType::from_raw(NodeType::OrdNoad.as_raw() + math_class_fn(c) as u16)
                    .expect("valid noad type"),
            );
        }
        let fam = nd.nucleus().plane_and_fam(&st.mem) + (ch / 65536) as u16 * 256;
        nd.nucleus().set_plane_and_fam(&mut st.mem, fam);
        st.input.cur_list.tail_append(&mut st.mem, nd.ptr());
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

pub(crate) fn check_outer_validity(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.scanner_status == NORMAL_SCANNER {
        return Ok(());
    }
    st.errors.deletions_allowed = false;

    if st.scanner.cur_cs != 0 {
        let ci = st.input.cur_input;
        if ci.state == TOKEN_LIST || ci.name < 1 || ci.name > 17 {
            let p = st.mem.get_avail()?;
            p.set_info(&mut st.mem, CS_TOKEN_FLAG + st.scanner.cur_cs);
            begin_token_list(st, p, BACKED_UP)?;
        }
        st.scanner.cur_cmd = SPACER;
        st.scanner.cur_chr = ' ' as i32;
    }

    let outcome = if st.scanner.scanner_status > SKIPPING {
        crate::errors::runaway(st);
        if st.scanner.cur_cs == 0 {
            error_here_with_diagnostic(st, "File ended");
        } else {
            st.scanner.cur_cs = 0;
            error_here_with_diagnostic(st, "Forbidden control sequence found");
        }

        let p = st.mem.get_avail()?;
        let mut inserted = p;
        match st.scanner.scanner_status {
            DEFINING => {
                print(st, " while scanning definition");
                p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
            }
            MATCHING => {
                print(st, " while scanning use");
                p.set_info(&mut st.mem, st.input.par_token);
                st.scanner.long_state = OUTER_CALL;
            }
            ALIGNING => {
                print(st, " while scanning preamble");
                p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
                inserted = st.mem.get_avail()?;
                inserted.set_link(&mut st.mem, p);
                inserted.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_CR);
                st.scanner.align_state = -1_000_000;
            }
            ABSORBING => {
                print(st, " while scanning text");
                p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
            }
            _ => {}
        }

        begin_token_list(st, inserted, INSERTED)?;
        print(st, " of ");
        let warning_index = st.scanner.warning_index;
        crate::output::sprint_cs(st, warning_index);
        capture_to_diagnostic(st, std::ptr::null_mut());
        st.errors.help(&[
            "I suspect you have forgotten a `}', causing me",
            "to read past where you wanted me to stop.",
            "I'll try to recover; but if the error is serious,",
            "you'd better type `E' or `X' now and fix your file.",
        ]);
        error(st)
    } else {
        error_here_with_diagnostic(st, "Incomplete ");
        let cur_if = st.scanner.cur_if;
        print_cmd_chr(st, IF_TEST, cur_if as i32);
        print(st, "; all text was ignored after line ");
        let skip_line = st.scanner.skip_line;
        print_int(st, skip_line);
        capture_to_diagnostic(st, std::ptr::null_mut());
        let cause = if st.scanner.cur_cs != 0 {
            st.scanner.cur_cs = 0;
            "A forbidden control sequence occurred in skipped text."
        } else {
            "The file ended while I was skipping conditional text."
        };
        st.errors.help(&[
            cause,
            r"This kind of error happens when you say `\if...' and forget",
            r"the matching `\fi'. I've inserted a `\fi'; this might work.",
        ]);
        st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_FI;
        ins_error(st)
    };
    st.errors.deletions_allowed = true;
    outcome
}

/// Sets `cur_cmd` and `cur_chr` to the meaning of `cur_cs`.
#[inline(always)]
fn fetch_cs_meaning(st: &mut EngineState) {
    st.scanner.cur_cmd = st.eqtb.eq_type(st.scanner.cur_cs);
    st.scanner.cur_chr = st.eqtb.equiv(st.scanner.cur_cs);
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

impl InputClusterState {
    /// Reduces a `^^` sequence at `buffer[k - 1..]` inside a control-sequence name.
    /// Returns whether the buffer changed, in which case the name must be rescanned.
    #[cold]
    fn reduce_cs_sup_mark(&mut self, k: i32, chr: i32) -> bool {
        let limit = self.cur_input.limit;
        let buf = &mut self.buffer;
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
        self.cur_input.limit -= shift;
        let mut i = k;
        while i <= self.cur_input.limit {
            self.buffer[i as usize] = self.buffer[(i + shift) as usize];
            i += 1;
        }
        true
    }
}

/// The central tokenizer.
pub(crate) fn get_next(st: &mut EngineState) -> Result<(), Abort> {
    st.scanner.logical_clock = st.scanner.logical_clock.wrapping_add(1);

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
                        let cat = st.eqtb.cat_code(st.scanner.cur_chr) as u8;
                        st.scanner.cur_cmd = cat;

                        // Dispatch on state + category (states 1, 17, 33, categories 0..=15).
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
                                        let first_cat = st.eqtb.cat_code(first_chr) as u8;
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
                                                cat = st.eqtb.cat_code(chr) as u8;
                                                k += 1;
                                                if cat != LETTER || k > st.input.cur_input.limit {
                                                    break;
                                                }
                                            }
                                            if cat == SUP_MARK
                                                && st.input.buffer[k as usize] == chr
                                                && k < st.input.cur_input.limit
                                                && st.input.reduce_cs_sup_mark(k, chr)
                                            {
                                                continue 'start_cs;
                                            }
                                            if cat != LETTER {
                                                k -= 1;
                                            }
                                            if k > st.input.cur_input.loc + 1 {
                                                let loc = st.input.cur_input.loc;
                                                st.scanner.cur_cs = st.eqtb.id_lookup(
                                                    &mut st.strings,
                                                    &st.input.buffer,
                                                    loc,
                                                    k - loc,
                                                )?;
                                                st.input.cur_input.loc = k;
                                                break 'start_cs;
                                            }
                                        } else if first_cat == SUP_MARK
                                            && st.input.buffer[k as usize] == first_chr
                                            && k < st.input.cur_input.limit
                                            && st.input.reduce_cs_sup_mark(k, first_chr)
                                        {
                                            continue 'start_cs;
                                        }

                                        let loc = st.input.cur_input.loc;
                                        if st.input.buffer[loc as usize] > 65535 {
                                            st.scanner.cur_cs = st.eqtb.id_lookup(
                                                &mut st.strings,
                                                &st.input.buffer,
                                                loc,
                                                1,
                                            )?;
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
                                    check_outer_validity(st)?;
                                }
                            }

                            // Active character.
                            14 | 30 | 46 => {
                                st.scanner.cur_cs = st.scanner.cur_chr + 1;
                                fetch_cs_meaning(st);
                                st.input.cur_input.state = MID_LINE;
                                if st.scanner.cur_cmd >= OUTER_CALL {
                                    check_outer_validity(st)?;
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
                                print_err(st, "Text line contains an invalid character");
                                st.errors.help(&[
                                    "A funny symbol that I can't read has just been input.",
                                    "Continue, and I'll forget that it ever happened.",
                                ]);
                                st.errors.deletions_allowed = false;
                                error(st)?;
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
                                    check_outer_validity(st)?;
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
                            crate::input::pseudo_input(st)?
                        } else {
                            let f = st.input.input_file_ptr(st.input.cur_input.index as i32);
                            unsafe { crate::io::input_line(st, f) }? != 0
                        };
                        if got_line {
                            st.input.cur_input.limit = st.input.last;
                        } else {
                            let index = st.input.cur_input.index as usize;
                            let every_eof = st.eqtb.local(LOCAL_EVERY_EOF);
                            if !every_eof.is_null() && !st.input.eof_seen[index] {
                                st.input.cur_input.limit = st.input.first - 1;
                                st.input.eof_seen[index] = true;
                                begin_token_list(st, every_eof, EVERY_EOF_TEXT)?;
                                continue 'restart;
                            }
                            st.input.force_eof = true;
                        }
                    }

                    if st.input.force_eof {
                        let in_open = st.input.in_open as usize;
                        if st.eqtb.int_par(INT_PAR_TRACING_NESTING) > 0
                            && (st.input.grp_stack[in_open] != st.save_stack.cur_boundary
                                || st.input.if_stack[in_open] != st.scanner.cond_ptr)
                        {
                            crate::output::file_warning(st);
                        }
                        if st.input.cur_input.name >= 19 {
                            crate::output::print_char(st, b')' as i32);
                            st.input.open_parens -= 1;
                            unsafe { ttstub_output_flush(st.shipout.rust_stdout) };
                        }
                        st.input.force_eof = false;
                        crate::input::end_file_reading(st)?;
                        check_outer_validity(st)?;
                        continue 'restart;
                    }

                    finish_line(st);
                } else {
                    if st.input.cur_input.name != 0 {
                        st.scanner.cur_cmd = 0;
                        st.scanner.cur_chr = 0;
                        return Ok(());
                    }
                    if st.input.input_ptr > 0 {
                        crate::input::end_file_reading(st)?;
                        continue 'restart;
                    }
                    let coda = st.eqtb.local(LOCAL_TECTONIC_CODA);
                    if !st.scanner.used_tectonic_coda_tokens && !coda.is_null() {
                        st.scanner.used_tectonic_coda_tokens = true;
                        begin_token_list(st, coda, TECTONIC_CODA_TEXT)?;
                        continue 'restart;
                    }
                    return Err(Abort::NoLegalEnd {
                        open_log: st.shipout.selector < SELECTOR_LOG_ONLY,
                    });
                }
            }
        } else if st.input.cur_input.loc != TEX_NULL {
            let p = MemPtr(st.input.cur_input.loc);
            let t = p.token(&st.mem);
            st.input.cur_input.loc = p.link(&st.mem).0;

            if t >= CS_TOKEN_FLAG {
                st.scanner.cur_cs = t - CS_TOKEN_FLAG;
                fetch_cs_meaning(st);
                if st.scanner.cur_cmd >= OUTER_CALL {
                    if st.scanner.cur_cmd == DONT_EXPAND {
                        st.scanner.cur_cs =
                            MemPtr(st.input.cur_input.loc).token(&st.mem) - CS_TOKEN_FLAG;
                        st.input.cur_input.loc = TEX_NULL;
                        fetch_cs_meaning(st);
                        if st.scanner.cur_cmd > MAX_COMMAND {
                            st.scanner.cur_cmd = RELAX;
                            st.scanner.cur_chr = NO_EXPAND_FLAG;
                        }
                    } else {
                        check_outer_validity(st)?;
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
                        begin_token_list(st, param, PARAMETER)?;
                        continue 'restart;
                    }
                    _ => {}
                }
            }
        } else {
            crate::input::end_token_list(st)?;
            continue 'restart;
        }

        // An alignment entry ends at `&` or `\cr` when braces are balanced.
        if st.scanner.cur_cmd <= CAR_RET
            && st.scanner.cur_cmd >= TAB_MARK
            && st.scanner.align_state == 0
        {
            if st.scanner.scanner_status == ALIGNING || st.scanner.cur_align.0 == TEX_NULL {
                return Err(Abort::Fatal(
                    "(interwoven alignment preambles are not allowed)",
                ));
            }
            let column = NodePtr::<AlignRecord>::at(st.scanner.cur_align);
            let saved_cmd = column.extra_info(&st.mem);
            column.set_extra_info(&mut st.mem, st.scanner.cur_chr);
            let template = if saved_cmd as u8 == OMIT {
                OMIT_TEMPLATE
            } else {
                column.v_part(&st.mem)
            };
            begin_token_list(st, template, V_TEMPLATE)?;
            st.scanner.cur_cmd = saved_cmd as u8;
            st.scanner.align_state = 1_000_000;
            continue 'restart;
        }
        break 'restart;
    }
    Ok(())
}

/// Appends `token` to the token list whose tail is `tail`.
#[inline(always)]
pub(crate) fn store_new_token(
    st: &mut EngineState,
    tail: &mut MemPtr,
    token: i32,
) -> Result<(), Abort> {
    st.mem.store_new_token(tail, token)
}

/// Starts a new reference-counted token list in `def_ref` and returns it.
fn new_def_ref(st: &mut EngineState) -> Result<MemPtr, Abort> {
    let def_ref = st.mem.get_avail()?;
    st.scanner.def_ref = def_ref;
    def_ref.set_info(&mut st.mem, TEX_NULL);
    Ok(def_ref)
}

/// The contents of box register `n`, or `None` if it is void.
pub(crate) fn fetch_box(st: &mut EngineState, n: i32) -> Option<AnyBox> {
    let contents = if n < 256 {
        st.eqtb.box_reg(n)
    } else {
        find_sa(st, BOX_VAL, n)?.sa_ptr(&st.mem)
    };
    st.mem.classify_box(contents.as_idx()?).ok().flatten()
}

impl MemState {
    /// TeX's `change_box`: sets box register `n` to `v` without saving.
    pub(crate) fn change_box(
        &mut self,
        scanner: &mut ScannerState,
        eqtb: &mut EqtbState,
        n: i32,
        v: MemPtr,
    ) {
        if n < 256 {
            eqtb.set_box_reg(n, v);
            return;
        }
        if let Some(sa) = self.find_sa_element(scanner, BOX_VAL, n) {
            sa.set_sa_ptr(self, v);
            sa.set_sa_ref(self, sa.sa_ref(self) + 1);
            self.delete_sa_ref(scanner, sa.idx());
        }
    }
}

/// `get_x_token` until the result is not a space.
#[inline]
pub(crate) fn get_x_non_blank(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        get_x_token(st)?;
        if st.scanner.cur_cmd != SPACER {
            return Ok(());
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

pub(crate) fn x_token(st: &mut EngineState) -> Result<(), Abort> {
    while st.scanner.cur_cmd > MAX_COMMAND {
        expand(st)?;
        get_next(st)?;
    }
    pack_cur_tok(st);
    Ok(())
}

pub(crate) fn get_x_token(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        if st.cancellation.at_expansion_token() {
            return Err(Abort::Cancelled);
        }
        get_next(st)?;
        if st.scanner.cur_cmd <= MAX_COMMAND {
            break;
        }
        if st.scanner.cur_cmd >= CALL {
            if st.scanner.cur_cmd < END_TEMPLATE {
                macro_call(st)?;
            } else {
                st.scanner.cur_cs = FROZEN_ENDV;
                st.scanner.cur_cmd = ENDV;
                break;
            }
        } else {
            expand(st)?;
        }
    }
    pack_cur_tok(st);
    Ok(())
}

/// Builds the token list for `\the` (or `\unexpanded`/`\detokenize`) at
/// `link(temp_head)` and returns its tail.
pub(crate) fn the_toks(st: &mut EngineState) -> Result<MemPtr, Abort> {
    if st.scanner.cur_chr & 1 != 0 {
        let c = st.scanner.cur_chr;
        scan_general_text(st)?;
        if c == 1 {
            return Ok(MemPtr::from_raw(st.scanner.cur_val));
        }
        let old_setting = st.shipout.selector;
        st.shipout.selector = SELECTOR_NEW_STRING;
        let b = st.strings.pool_ptr;
        let p = st.mem.get_avail()?;
        let list = TEMP_HEAD.link(&st.mem);
        p.set_link(&mut st.mem, list);
        crate::output::token_show(st, p);
        st.mem.flush_list(p);
        st.shipout.selector = old_setting;
        return st.mem.str_toks_cat(&mut st.strings, b, 0);
    }
    get_x_token(st)?;
    scan_something_internal(st, TOK_VAL, false)?;
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
            0 => print_int(st, st.scanner.cur_val),
            1 => {
                crate::output::print_scaled(st, st.scanner.cur_val);
                print(st, "pt");
            }
            2 => {
                crate::output::print_spec(st, st.scanner.cur_val, Some("pt"));
                st.mem.delete_glue_ref(st.scanner.cur_glue());
            }
            3 => {
                crate::output::print_spec(st, st.scanner.cur_val, Some("mu"));
                st.mem.delete_glue_ref(st.scanner.cur_glue());
            }
            _ => {}
        }
        st.shipout.selector = old_setting;
        st.mem.str_toks_cat(&mut st.strings, b, 0)
    }
}
pub(crate) fn scan_toks(
    st: &mut EngineState,
    macro_def: bool,
    xpand: bool,
) -> Result<MemPtr, Abort> {
    if macro_def {
        st.scanner.scanner_status = DEFINING;
    } else {
        st.scanner.scanner_status = ABSORBING;
    }
    st.scanner.warning_index = st.scanner.cur_cs;
    let def_ref = new_def_ref(st)?;
    let mut p = def_ref;
    let mut hash_brace: i32 = 0;
    let mut t = ZERO_TOKEN;

    'found: {
        if macro_def {
            let ended_with_brace = loop {
                get_token(st)?;
                if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
                    break false;
                }
                if st.scanner.cur_cmd == MAC_PARAM {
                    let s = MATCH_TOKEN + st.scanner.cur_chr;
                    get_token(st)?;
                    if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
                        hash_brace = st.scanner.cur_tok;
                        store_new_token(st, &mut p, st.scanner.cur_tok)?;
                        store_new_token(st, &mut p, END_MATCH_TOKEN)?;
                        break true;
                    }
                    if t == ZERO_TOKEN + 9 {
                        print_err(st, "You already have nine parameters");
                        st.errors.help(&[
                            "I'm going to ignore the # sign you just used,",
                            "as well as the token that followed it.",
                        ]);
                        error(st)?;
                        continue;
                    }
                    t += 1;
                    if st.scanner.cur_tok != t {
                        error_here_with_diagnostic(st, "Parameters must be numbered consecutively");
                        st.errors.help(&[
                            "I've inserted the digit you should have used after the #.",
                            "Type `1' to delete what you did use.",
                        ]);
                        back_error(st)?;
                    }
                    st.scanner.cur_tok = s;
                }
                store_new_token(st, &mut p, st.scanner.cur_tok)?;
            };
            if !ended_with_brace {
                store_new_token(st, &mut p, END_MATCH_TOKEN)?;
                if st.scanner.cur_cmd == RIGHT_BRACE {
                    print_err(st, "Missing { inserted");
                    st.scanner.align_state += 1;
                    st.errors.help(&[
                        r"Where was the left brace? You said something like `\def\a}',",
                        r"which I'm going to interpret as `\def\a{}'.",
                    ]);
                    error(st)?;
                    break 'found;
                }
            }
        } else {
            scan_left_brace(st)?;
        }

        let mut unbalance: i32 = 1;
        loop {
            if xpand {
                loop {
                    get_next(st)?;
                    if st.scanner.cur_cmd >= CALL
                        && st.mem.is_protected_macro(MemPtr(st.scanner.cur_chr))
                    {
                        st.scanner.cur_cmd = RELAX;
                        st.scanner.cur_chr = NO_EXPAND_FLAG;
                    }
                    if st.scanner.cur_cmd <= MAX_COMMAND {
                        break;
                    }
                    if st.scanner.cur_cmd != THE {
                        expand(st)?;
                    } else {
                        let tail = the_toks(st)?;
                        let list = TEMP_HEAD.link(&st.mem);
                        if !list.is_null() {
                            p.set_link(&mut st.mem, list);
                            p = tail;
                        }
                    }
                }
                x_token(st)?;
            } else {
                get_token(st)?;
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
                    get_x_token(st)?;
                } else {
                    get_token(st)?;
                }
                if st.scanner.cur_cmd != MAC_PARAM {
                    if st.scanner.cur_tok <= ZERO_TOKEN || st.scanner.cur_tok > t {
                        error_here_with_diagnostic(
                            st,
                            "Illegal parameter number in definition of ",
                        );
                        crate::output::sprint_cs(st, st.scanner.warning_index);
                        capture_to_diagnostic(st, std::ptr::null_mut());
                        st.errors.help(&[
                            "You meant to type ## instead of #, right?",
                            "Or maybe a } was forgotten somewhere earlier, and things",
                            "are all screwed up? I'm going to assume that you meant ##.",
                        ]);
                        back_error(st)?;
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
    Ok(p)
}

pub(crate) fn pass_text(st: &mut EngineState) -> Result<(), Abort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = SKIPPING;
    let mut l: i32 = 0;
    st.scanner.skip_line = st.mem.line;

    loop {
        get_next(st)?;
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
    if st.eqtb.int_par(INT_PAR_TRACING_IFS) > 0 {
        crate::output::show_cur_cmd_chr(st);
    }
    Ok(())
}
pub(crate) fn pseudo_start(st: &mut EngineState) -> Result<(), Abort> {
    scan_general_text(st)?;
    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    crate::output::token_show(st, TEMP_HEAD);
    st.shipout.selector = old_setting;
    let head = TEMP_HEAD.link(&st.mem);
    st.mem.flush_list(head);

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(Abort::Overflow {
            what: "pool size",
            n: st.strings.pool_size - st.strings.init_pool_ptr,
        });
    }
    let s = st.strings.make_string().map_err(|_| Abort::Overflow {
        what: "number of strings",
        n: st.strings.max_strings - st.strings.init_str_ptr,
    })?;
    st.strings.str_pool[st.strings.pool_ptr as usize] = b' ' as u16;
    let mut l = st.strings.str_start[(s - TOO_BIG_CHAR) as usize];
    let nl = st.eqtb.int_par(INT_PAR_NEW_LINE_CHAR) as u16;
    let file = st.mem.get_avail()?;
    let mut last_line = file;

    // Each line becomes a node of words holding four characters each.
    while l < st.strings.pool_ptr {
        let m = l;
        while l < st.strings.pool_ptr && st.strings.str_pool[l as usize] != nl {
            l += 1;
        }
        let size = ((l - m + 7) / 4).max(2);
        let line = NodePtr::<PseudoLineNode>::at(st.mem.get_node(size)?);
        last_line.set_link(&mut st.mem, line.ptr());
        last_line = line.ptr();
        line.set_size(&mut st.mem, size);
        let mut w = 1;
        let mut k = m;
        for _ in 2..size {
            let pool = &st.strings.str_pool[k as usize..];
            let chars = [pool[0], pool[1], pool[2], pool[3]];
            line.set_chars(&mut st.mem, w, chars);
            w += 1;
            k += 4;
        }
        let mut chars = [b' ' as u16; 4];
        for (i, c) in chars.iter_mut().enumerate() {
            if l > k + i as i32 {
                *c = st.strings.str_pool[(k + i as i32) as usize];
            }
        }
        line.set_chars(&mut st.mem, w, chars);
        if st.strings.str_pool[l as usize] == nl {
            l += 1;
        }
    }

    let lines = file.link(&st.mem);
    file.set_info(&mut st.mem, lines.0);
    file.set_link(&mut st.mem, st.input.pseudo_files);
    st.input.pseudo_files = file;
    st.strings.flush_str(s);

    crate::input::begin_file_reading(st)?;
    st.mem.line = 0;
    let start = st.input.cur_input.start;
    st.input.cur_input.limit = start;
    st.input.cur_input.loc = start + 1;

    if st.eqtb.int_par(INT_PAR_TRACING_SCAN_TOKENS) > 0 {
        if st.shipout.term_offset > st.config.max_print_line - 3 {
            crate::output::print_ln(st);
        } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
            crate::output::print_char(st, b' ' as i32);
        }
        st.input.cur_input.name = 19;
        print(st, "( ");
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
fn scan_signs(st: &mut EngineState) -> Result<bool, Abort> {
    let mut negative = false;
    loop {
        get_x_non_blank(st)?;
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
fn scan_optional_space(st: &mut EngineState) -> Result<(), Abort> {
    get_x_token(st)?;
    if st.scanner.cur_cmd != SPACER {
        back_input(st)?;
    }
    Ok(())
}

/// Reads the token after `\primitive` and replaces it by the primitive's meaning.
fn get_primitive_token(st: &mut EngineState) -> Result<(), Abort> {
    get_token(st)?;
    let p = st
        .eqtb
        .prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)
        .map_err(|e| e)?;
    if p != UNDEFINED_PRIMITIVE {
        st.scanner.cur_cmd = st.eqtb.eq_type(PRIM_EQTB_BASE + p);
        st.scanner.cur_chr = st.eqtb.equiv(PRIM_EQTB_BASE + p);
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

pub(crate) fn scan_int(st: &mut EngineState) -> Result<(), Abort> {
    let mut ok_so_far = true;
    st.scanner.radix = 0;
    let negative = scan_signs(st)?;

    loop {
        if st.scanner.cur_tok == ALPHA_TOKEN {
            get_token(st)?;
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
                print_err(st, "Improper alphabetic constant");
                st.errors.help(&[
                    "A one-character control sequence belongs after a ` mark.",
                    r"So I'm essentially inserting \0 here.",
                ]);
                st.scanner.cur_val = b'0' as i32;
                back_error(st)?;
            } else {
                scan_optional_space(st)?;
            }
        } else if st.scanner.cur_tok == FROZEN_PRIM_TOKEN {
            get_primitive_token(st)?;
            continue;
        } else if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
            scan_something_internal(st, INT_VAL, false)?;
        } else {
            let (radix, m) = if st.scanner.cur_tok == OCTAL_TOKEN {
                get_x_token(st)?;
                (8, 0x1000_0000)
            } else if st.scanner.cur_tok == HEX_TOKEN {
                get_x_token(st)?;
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
                        print_err(st, "Number too big");
                        st.errors.help(&[
                            "I can only go up to 2147483647='17777777777=\"7FFFFFFF,",
                            "so I'm using that number instead of yours.",
                        ]);
                        error(st)?;
                        st.scanner.cur_val = TEX_INFINITY;
                        ok_so_far = false;
                    }
                } else {
                    st.scanner.cur_val = st.scanner.cur_val * radix + d;
                }
                get_x_token(st)?;
            }
            if vacuous {
                print_err(st, "Missing number, treated as zero");
                st.errors.help(&[
                    "A number should have been here; I inserted `0'.",
                    "(If you can't figure out why I needed to see a number,",
                    "look up `weird error' in the index to The TeXbook.)",
                ]);
                back_error(st)?;
            } else if st.scanner.cur_cmd != SPACER {
                back_input(st)?;
            }
        }
        break;
    }

    if negative {
        st.scanner.cur_val = -st.scanner.cur_val;
    }
    Ok(())
}
pub(crate) fn scan_dimen(
    st: &mut EngineState,
    mu: bool,
    inf: bool,
    shortcut: bool,
) -> Result<(), Abort> {
    xetex_scan_dimen(st, mu, inf, shortcut, true)
}
/// A glue spec pointer held in a raw word.
fn glue_spec(p: i32) -> NodePtr<GlueSpecNode> {
    NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(p))
}

/// Replaces the glue in `cur_val` by its width, releasing the glue.
fn glue_to_width(st: &mut EngineState) {
    let spec = st.scanner.cur_val;
    st.scanner.cur_val = glue_spec(spec).width(&st.mem).0;
    st.mem.delete_glue_ref(glue_spec(spec));
}

/// Scales `cur_val` and the fraction `f` by `num / denom`.
fn scale_dimen(st: &mut EngineState, f: &mut i32, num: i32, denom: i32) {
    st.scanner.cur_val = st.errors.xn_over_d(st.scanner.cur_val, num, denom);
    *f = (num * *f + 65536 * st.errors.tex_remainder) / denom;
    st.scanner.cur_val += *f / 65536;
    *f %= 65536;
}

/// Sets `cur_val` to `integer.fraction` times the unit `v`.
fn attach_unit(st: &mut EngineState, integer: i32, fraction: i32, v: i32) {
    let fraction_part = st.errors.xn_over_d(v, fraction, 65536);
    st.scanner.cur_val = st
        .errors
        .mult_and_add(integer, v, fraction_part, MAX_HALFWORD);
}

pub(crate) fn xetex_scan_dimen(
    st: &mut EngineState,
    mu: bool,
    inf: bool,
    shortcut: bool,
    requires_units: bool,
) -> Result<(), Abort> {
    let mut f: i32 = 0;
    let mut negative = false;

    st.errors.arith_error = false;
    st.scanner.cur_order = NORMAL;

    'attach_sign: {
        if !shortcut {
            negative = scan_signs(st)?;

            if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
                if mu {
                    scan_something_internal(st, MU_VAL, false)?;
                    if st.scanner.cur_val_level >= GLUE_VAL {
                        glue_to_width(st);
                    }
                    if st.scanner.cur_val_level == MU_VAL {
                        break 'attach_sign;
                    }
                    if st.scanner.cur_val_level != INT_VAL {
                        mu_error(st)?;
                    }
                } else {
                    scan_something_internal(st, DIMEN_VAL, false)?;
                    if st.scanner.cur_val_level == DIMEN_VAL {
                        break 'attach_sign;
                    }
                }
            } else {
                back_input(st)?;
                if st.scanner.cur_tok == CONTINENTAL_POINT_TOKEN {
                    st.scanner.cur_tok = POINT_TOKEN;
                }
                if st.scanner.cur_tok != POINT_TOKEN {
                    scan_int(st)?;
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
                    get_token(st)?;
                    loop {
                        get_x_token(st)?;
                        if st.scanner.cur_tok > ZERO_TOKEN + 9 || st.scanner.cur_tok < ZERO_TOKEN {
                            break;
                        }
                        if k < 17 {
                            let q = st.mem.get_avail()?;
                            q.set_link(&mut st.mem, digits);
                            q.set_info(&mut st.mem, st.scanner.cur_tok - ZERO_TOKEN);
                            digits = q;
                            k += 1;
                        }
                    }
                    for i in (0..k as usize).rev() {
                        st.errors.dig[i] = digits.info(&st.mem) as u8;
                        let next = digits.link(&st.mem);
                        st.mem.free_avail(digits);
                        digits = next;
                    }
                    f = st.errors.round_decimals(k);
                    if st.scanner.cur_cmd != SPACER {
                        back_input(st)?;
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
                if inf && scan_keyword(st, "fil")? {
                    st.scanner.cur_order = FIL;
                    while scan_keyword(st, "l")? {
                        if st.scanner.cur_order == FILLL {
                            print_err(st, "Illegal unit of measure (replaced by filll)");
                            st.errors.help(&["I dddon't go any higher than filll."]);
                            error(st)?;
                        } else {
                            st.scanner.cur_order += 1;
                        }
                    }
                    break 'units true;
                }

                let save_cur_val = st.scanner.cur_val;
                get_x_non_blank(st)?;

                if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
                    if mu {
                        scan_something_internal(st, MU_VAL, false)?;
                        if st.scanner.cur_val_level >= GLUE_VAL {
                            glue_to_width(st);
                        }
                        if st.scanner.cur_val_level != MU_VAL {
                            mu_error(st)?;
                        }
                    } else {
                        scan_something_internal(st, DIMEN_VAL, false)?;
                    }
                    attach_unit(st, save_cur_val, f, st.scanner.cur_val);
                    break 'attach_sign;
                }

                back_input(st)?;

                if !mu {
                    let font_param = if scan_keyword(st, "em")? {
                        Some(QUAD_CODE)
                    } else if scan_keyword(st, "ex")? {
                        Some(X_HEIGHT_CODE)
                    } else {
                        None
                    };
                    if let Some(code) = font_param {
                        let font = st.eqtb.cur_font();
                        let v = st.fonts.param(font, code);
                        scan_optional_space(st)?;
                        attach_unit(st, save_cur_val, f, v);
                        break 'attach_sign;
                    }
                }

                if mu {
                    if scan_keyword(st, "mu")? {
                        break 'units true;
                    } else {
                        print_err(st, "Illegal unit of measure (mu inserted)");
                        st.errors.help(&[
                            "The unit of measurement in math glue must be mu.",
                            "To recover gracefully from this error, it's best to",
                            "delete the erroneous units; e.g., type `2' to delete",
                            "two letters. (See Chapter 27 of The TeXbook.)",
                        ]);
                        error(st)?;
                        break 'units true;
                    }
                }

                if scan_keyword(st, "true")? {
                    crate::save::prepare_mag(st)?;
                    let mag = st.eqtb.int_par(INT_PAR_MAG);
                    if mag != 1000 {
                        scale_dimen(st, &mut f, 1000, mag);
                    }
                }

                if scan_keyword(st, "pt")? {
                    break 'units true;
                }

                let (num, denom) = if scan_keyword(st, "in")? {
                    (7227_i32, 100_i32)
                } else if scan_keyword(st, "pc")? {
                    (12, 1)
                } else if scan_keyword(st, "cm")? {
                    (7227, 254)
                } else if scan_keyword(st, "mm")? {
                    (7227, 2540)
                } else if scan_keyword(st, "bp")? {
                    (7227, 7200)
                } else if scan_keyword(st, "dd")? {
                    (1238, 1157)
                } else if scan_keyword(st, "cc")? {
                    (14856, 1157)
                } else if scan_keyword(st, "sp")? {
                    break 'units false;
                } else {
                    print_err(st, "Illegal unit of measure (pt inserted)");
                    st.errors.help(&[
                        "Dimensions can be in units of em, ex, in, pt, pc,",
                        "cm, mm, dd, cc, bp, or sp; but yours is a new one!",
                        "I'll assume that you meant to say pt, for printer's points.",
                        "To recover gracefully from this error, it's best to",
                        "delete the erroneous units; e.g., type `2' to delete",
                        "two letters. (See Chapter 27 of The TeXbook.)",
                    ]);
                    error(st)?;
                    break 'units true;
                };
                scale_dimen(st, &mut f, num, denom);
                true
            };

            if apply_fraction {
                attach_fraction(st, f);
            }
            scan_optional_space(st)?;
        } else {
            attach_fraction(st, f);
        }
    }

    if st.errors.arith_error || st.scanner.cur_val.abs() >= 0x4000_0000 {
        print_err(st, "Dimension too large");
        st.errors.help(&[
            "I can't work with sizes bigger than about 19 feet.",
            "Continue and I'll use the largest value I can.",
        ]);
        error(st)?;
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

pub(crate) fn scan_glue(st: &mut EngineState, level: u8) -> Result<(), Abort> {
    let mu = level == MU_VAL;
    let negative = scan_signs(st)?;

    if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
        scan_something_internal(st, level, negative)?;
        if st.scanner.cur_val_level >= GLUE_VAL {
            if st.scanner.cur_val_level != level {
                mu_error(st)?;
            }
            return Ok(());
        }
        if st.scanner.cur_val_level == INT_VAL {
            scan_dimen(st, mu, false, true)?;
        } else if level == MU_VAL {
            mu_error(st)?;
        }
    } else {
        back_input(st)?;
        scan_dimen(st, mu, false, false)?;
        if negative {
            st.scanner.cur_val = -st.scanner.cur_val;
        }
    }

    let gs = st.mem.new_spec(glue_spec(0))?;
    gs.set_width(&mut st.mem, Scaled(st.scanner.cur_val));

    if scan_keyword(st, "plus")? {
        scan_dimen(st, mu, true, false)?;
        gs.set_stretch(&mut st.mem, Scaled(st.scanner.cur_val));
        gs.set_stretch_order(&mut st.mem, st.scanner.cur_order);
    }
    if scan_keyword(st, "minus")? {
        scan_dimen(st, mu, true, false)?;
        gs.set_shrink(&mut st.mem, Scaled(st.scanner.cur_val));
        gs.set_shrink_order(&mut st.mem, st.scanner.cur_order);
    }

    st.scanner.cur_val = gs.ptr().0;
    Ok(())
}
pub(crate) fn scan_normal_glue(st: &mut EngineState) -> Result<(), Abort> {
    scan_glue(st, GLUE_VAL)
}
pub(crate) fn scan_mu_glue(st: &mut EngineState) -> Result<(), Abort> {
    scan_glue(st, MU_VAL)
}
/// Applies `op` to the width, stretch and shrink of `spec`, in that order.
fn map_glue(
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
fn add_glue(st: &mut EngineState, e: NodePtr<GlueSpecNode>, t: NodePtr<GlueSpecNode>, sub: bool) {
    let mem = &mut st.mem;
    let errors = &mut st.errors;
    let width = errors.add_or_sub(e.width(mem).0, t.width(mem).0, MAX_HALFWORD, sub);
    e.set_width(mem, Scaled(width));
    let (e_order, t_order, t_stretch) =
        (e.stretch_order(mem), t.stretch_order(mem), t.stretch(mem));
    if e_order == t_order {
        let stretch = errors.add_or_sub(e.stretch(mem).0, t_stretch.0, MAX_HALFWORD, sub);
        e.set_stretch(mem, Scaled(stretch));
    } else if e_order < t_order && t_stretch.0 != 0 {
        e.set_stretch(mem, t_stretch);
        e.set_stretch_order(mem, t_order);
    }
    let (e_order, t_order, t_shrink) = (e.shrink_order(mem), t.shrink_order(mem), t.shrink(mem));
    if e_order == t_order {
        let shrink = errors.add_or_sub(e.shrink(mem).0, t_shrink.0, MAX_HALFWORD, sub);
        e.set_shrink(mem, Scaled(shrink));
    } else if e_order < t_order && t_shrink.0 != 0 {
        e.set_shrink(mem, t_shrink);
        e.set_shrink_order(mem, t_order);
    }
    mem.delete_glue_ref(t);
    normalize_glue_orders(st, e);
}

pub(crate) fn scan_expr(st: &mut EngineState) -> Result<(), Abort> {
    let mut l = st.scanner.cur_val_level;
    let a = st.errors.arith_error;
    let mut b = false;
    let mut p = MemPtr::NULL;

    st.scanner.expand_depth_count += 1;
    if st.scanner.expand_depth_count >= st.scanner.expand_depth {
        return Err(Abort::Overflow {
            what: "expansion depth",
            n: st.scanner.expand_depth,
        });
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
                get_x_non_blank(st)?;
                if st.scanner.cur_tok == OTHER_TOKEN + b'(' as i32 {
                    let q = NodePtr::<ExprNode>::alloc(&mut st.mem)?;
                    q.set_link(&mut st.mem, p);
                    q.set_level(&mut st.mem, l as u16);
                    q.set_state(&mut st.mem, (4 * s + r) as u16);
                    q.set_e(&mut st.mem, e);
                    q.set_t(&mut st.mem, t);
                    q.set_n(&mut st.mem, n);
                    p = q.ptr();
                    l = o;
                    continue 'restart;
                }
                back_input(st)?;
                match o {
                    INT_VAL => scan_int(st)?,
                    DIMEN_VAL => scan_dimen(st, false, false, false)?,
                    GLUE_VAL => scan_normal_glue(st)?,
                    _ => scan_mu_glue(st)?,
                }
                f = st.scanner.cur_val;
            }
            scan_factor = true;

            get_x_non_blank(st)?;
            let mut o = match u8::try_from(st.scanner.cur_tok - OTHER_TOKEN) {
                Ok(b'+') => EXPR_ADD,
                Ok(b'-') => EXPR_SUB,
                Ok(b'*') => EXPR_MULT,
                Ok(b'/') => EXPR_DIV,
                _ => EXPR_NONE,
            };
            if o == EXPR_NONE {
                if p.is_null() {
                    if st.scanner.cur_cmd != RELAX {
                        back_input(st)?;
                    }
                } else if st.scanner.cur_tok != OTHER_TOKEN + b')' as i32 {
                    print_err(st, "Missing ) inserted for expression");
                    st.errors
                        .help(&["I was expecting to see `+', `-', `*', `/', or `)'. Didn't."]);
                    back_error(st)?;
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
                    st.mem.delete_glue_ref(glue_spec(f));
                    f = st.mem.new_spec(glue_spec(0))?.ptr().0;
                }
            }

            match s {
                EXPR_NONE => {
                    if l >= GLUE_VAL && o != EXPR_NONE {
                        let term = st.mem.new_spec(glue_spec(f))?;
                        t = term.ptr().0;
                        st.mem.delete_glue_ref(glue_spec(f));
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
                        t = st.errors.mult_and_add(t, f, 0, TEX_INFINITY);
                    } else if l == DIMEN_VAL {
                        t = st.errors.mult_and_add(t, f, 0, MAX_HALFWORD);
                    } else {
                        map_glue(st, glue_spec(t), |errors, x| {
                            errors.mult_and_add(x, f, 0, MAX_HALFWORD)
                        });
                    }
                }
                EXPR_DIV => {
                    if l < GLUE_VAL {
                        t = st.errors.quotient(t, f);
                    } else {
                        map_glue(st, glue_spec(t), |errors, x| errors.quotient(x, f));
                    }
                }
                EXPR_SCALE => {
                    if l == INT_VAL {
                        t = st.errors.fract(t, n, f, TEX_INFINITY);
                    } else if l == DIMEN_VAL {
                        t = st.errors.fract(t, n, f, MAX_HALFWORD);
                    } else {
                        map_glue(st, glue_spec(t), |errors, x| {
                            errors.fract(x, n, f, MAX_HALFWORD)
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
                    e = st.errors.add_or_sub(e, t, TEX_INFINITY, r == EXPR_SUB);
                } else if l == DIMEN_VAL {
                    e = st.errors.add_or_sub(e, t, MAX_HALFWORD, r == EXPR_SUB);
                } else {
                    add_glue(st, glue_spec(e), glue_spec(t), r == EXPR_SUB);
                }
                r = o;
            }
            b = st.errors.arith_error;
            if o != EXPR_NONE {
                continue;
            }
            if p.is_null() {
                break 'restart;
            }
            f = e;
            let q = NodePtr::<ExprNode>::at(p);
            e = q.e(&st.mem);
            t = q.t(&st.mem);
            n = q.n(&st.mem);
            s = (q.state(&st.mem) / 4) as i16;
            r = (q.state(&st.mem) % 4) as i16;
            l = q.level(&st.mem) as u8;
            p = q.link(&st.mem);
            q.free(&mut st.mem);
            scan_factor = false;
        }
    }

    st.scanner.expand_depth_count -= 1;

    if b {
        print_err(st, "Arithmetic overflow");
        st.errors.help(&[
            "I can't evaluate this expression,",
            "since the result is out of range.",
        ]);
        error(st)?;
        if l >= GLUE_VAL {
            st.mem.delete_glue_ref(glue_spec(e));
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

pub(crate) fn scan_keyword(st: &mut EngineState, keyword: &str) -> Result<bool, Abort> {
    let keyword = keyword.as_bytes();
    let save_cur_cs = st.scanner.cur_cs;
    let mut p = BACKUP_HEAD;
    p.set_link(&mut st.mem, MemPtr::NULL);
    let mut i = 0;
    while i < keyword.len() {
        get_x_token(st)?;
        let ch = keyword[i] as i32;
        if st.scanner.cur_cs == 0 && (st.scanner.cur_chr == ch || st.scanner.cur_chr == ch - 32) {
            store_new_token(st, &mut p, st.scanner.cur_tok)?;
            i += 1;
        } else if st.scanner.cur_cmd != SPACER || p != BACKUP_HEAD {
            back_input(st)?;
            if p != BACKUP_HEAD {
                begin_token_list(st, BACKUP_HEAD.link(&st.mem), BACKED_UP)?;
            }
            if keyword.len() == 1 {
                st.scanner.cur_cs = save_cur_cs;
            }
            return Ok(false);
        }
    }
    let matched = BACKUP_HEAD.link(&st.mem);
    st.mem.flush_list(matched);
    Ok(true)
}

pub(crate) fn scan_left_brace(st: &mut EngineState) -> Result<(), Abort> {
    get_x_non_blank_non_relax(st)?;
    if st.scanner.cur_cmd != LEFT_BRACE {
        print_err(st, "Missing { inserted");
        st.errors.help(&[
            "A left brace was mandatory here, so I've put one in.",
            "You might want to delete and/or insert some corrections",
            "so that I will find a matching right brace soon.",
            "(If you're confused by all this, try typing `I}' now.)",
        ]);
        back_error(st)?;
        st.scanner.cur_tok = LEFT_BRACE_TOKEN + '{' as i32;
        st.scanner.cur_cmd = LEFT_BRACE;
        st.scanner.cur_chr = '{' as i32;
        st.scanner.align_state += 1;
    }
    Ok(())
}

fn missing_number_error(st: &mut EngineState) -> Result<(), Abort> {
    print_err(st, "Missing number, treated as zero");
    st.errors.help(&[
        "A number should have been here; I inserted `0'.",
        "(If you can't figure out why I needed to see a number,",
        "look up `weird error' in the index to The TeXbook.)",
    ]);
    back_error(st)
}

#[inline]
fn set_cur_val(st: &mut EngineState, value: i32, level: u8) {
    st.scanner.cur_val = value;
    st.scanner.cur_val_level = level as u8;
}

/// The sparse-array element of type `t` for register `n`, if it exists.
fn find_sa(st: &mut EngineState, t: u8, n: i32) -> Option<NodePtr<SaNode>> {
    st.mem.find_sa_element(&mut st.scanner, t, n)
}

/// The token list held by sparse register `sa`, null when unset.
pub(crate) fn sa_token_list(st: &EngineState, sa: Option<NodePtr<SaNode>>) -> MemPtr {
    sa.map_or(MemPtr::NULL, |sa| sa.sa_ptr(&st.mem))
}

/// Replaces the glue in `cur_val` by a negated copy and returns the original glue.
fn negate_glue(st: &mut EngineState) -> Result<i32, Abort> {
    let old = st.scanner.cur_val;
    let spec = st.mem.new_spec(glue_spec(old))?;
    st.scanner.cur_val = spec.ptr().0;
    map_glue(st, spec, |_, x| -x);
    Ok(old)
}

/// Scans a font identifier, returning the font, its layout engine and its area flag.
fn scan_font_engine(st: &mut EngineState) -> Result<(i32, *mut libc::c_void, i32), Abort> {
    scan_font_ident(st)?;
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
fn font_query_error(
    st: &mut EngineState,
    m: i32,
    f: i32,
    msg: &str,
    fallback: i32,
) -> Result<(), Abort> {
    crate::errors::not_font_error_common(st, LAST_ITEM, m, f, msg)?;
    st.scanner.cur_val = fallback;
    Ok(())
}

fn fetch_def_code(st: &mut EngineState, m: i32) -> Result<(), Abort> {
    scan_usv_num(st)?;
    let n = st.scanner.cur_val;
    if m == MATH_CODE_BASE {
        let code = st.eqtb.math_code(n);
        st.scanner.cur_val1 = code;
        if math_char_fn(code) == ACTIVE_MATH_CHAR {
            st.scanner.cur_val1 = 0x8000;
        } else if math_class_fn(code) > 7 || math_fam_fn(code) > 15 || math_char_fn(code) > 255 {
            bad_int_error(
                st,
                "Extended mathchar used as mathchar",
                code,
                "A mathchar number must be between 0 and \"7FFF.",
                "I changed this one to zero.",
            )?;
            st.scanner.cur_val1 = 0;
        }
        let code = st.scanner.cur_val1;
        let v = math_class_fn(code) * 0x1000 + math_fam_fn(code) * 0x100 + math_char_fn(code);
        set_cur_val(st, v, INT_VAL);
    } else if m == DEL_CODE_BASE {
        st.scanner.cur_val1 = st.eqtb.del_code(n);
        if st.scanner.cur_val1 >= 0x4000_0000 {
            print_err(st, "Extended delcode used as delcode");
            st.errors.help(&[
                "A delimiter code must be between 0 and \"7FFFFFF.",
                "I changed this one to zero.",
            ]);
            error(st)?;
            set_cur_val(st, 0, INT_VAL);
        } else {
            set_cur_val(st, st.scanner.cur_val1, INT_VAL);
        }
    } else if m < SF_CODE_BASE {
        set_cur_val(st, st.eqtb.equiv(m + n), INT_VAL);
    } else if m < MATH_CODE_BASE {
        set_cur_val(st, st.eqtb.equiv(m + n) % 65536, INT_VAL);
    } else {
        set_cur_val(st, st.eqtb.equiv(m + n), INT_VAL);
    }
    Ok(())
}

fn fetch_xetex_def_code(st: &mut EngineState, m: i32) -> Result<(), Abort> {
    scan_usv_num(st)?;
    let n = st.scanner.cur_val;
    let (err, help1, help0) = if m == SF_CODE_BASE {
        set_cur_val(st, st.eqtb.sf_code(n) / 65536, INT_VAL);
        return Ok(());
    } else if m == MATH_CODE_BASE {
        set_cur_val(st, st.eqtb.math_code(n), INT_VAL);
        return Ok(());
    } else if m == MATH_CODE_BASE + 1 {
        (
            "Can't use \\Umathcode as a number (try \\Umathcodenum)",
            r"\Umathcode is for setting a mathcode from separate values;",
            r"use \Umathcodenum to access them as single values.",
        )
    } else if m == DEL_CODE_BASE {
        set_cur_val(st, st.eqtb.del_code(n), INT_VAL);
        return Ok(());
    } else {
        (
            "Can't use \\Udelcode as a number (try \\Udelcodenum)",
            r"\Udelcode is for setting a delcode from separate values;",
            r"use \Udelcodenum to access them as single values.",
        )
    };
    print_err(st, err);
    st.errors.help(&[help1, help0]);
    error(st)?;
    set_cur_val(st, 0, INT_VAL);
    Ok(())
}

fn fetch_token_list(st: &mut EngineState, m: i32, level: u8) -> Result<(), Abort> {
    if level != TOK_VAL {
        missing_number_error(st)?;
        set_cur_val(st, 0, DIMEN_VAL);
    } else if st.scanner.cur_cmd < ASSIGN_TOKS {
        if m == 0 {
            scan_register_num(st)?;
            let n = st.scanner.cur_val;
            st.scanner.cur_val = if n < 256 {
                st.eqtb.toks(n)
            } else {
                let sa = find_sa(st, TOK_VAL, n);
                sa_token_list(st, sa)
            }
            .0;
        } else {
            st.scanner.cur_val = NodePtr::<SaNode>::at(m).sa_ptr(&st.mem).0;
        }
        st.scanner.cur_val_level = TOK_VAL;
    } else if st.scanner.cur_cmd == ASSIGN_TOKS {
        if st.scanner.cur_chr == LOCAL_BASE + LOCAL_XETEX_INTER_CHAR_TOKS {
            scan_char_class_not_ignored(st)?;
            st.scanner.cur_ptr.0 = st.scanner.cur_val;
            scan_char_class_not_ignored(st)?;
            let n = st.scanner.cur_ptr.0 * CHAR_CLASS_LIMIT + st.scanner.cur_val;
            let sa = find_sa(st, INTER_CHAR_VAL, n);
            st.scanner.cur_val = sa_token_list(st, sa).0;
        } else {
            st.scanner.cur_val = st.eqtb.equiv(m);
        }
        st.scanner.cur_val_level = TOK_VAL;
    } else {
        back_input(st)?;
        scan_font_ident(st)?;
        set_cur_val(st, FONT_ID_BASE + st.scanner.cur_val, IDENT_VAL);
    }
    Ok(())
}

fn fetch_register(st: &mut EngineState, m: i32) -> Result<(), Abort> {
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
    scan_register_num(st)?;
    let n = st.scanner.cur_val;
    let level = m as u8;
    st.scanner.cur_val_level = level as u8;
    if n > 255 {
        st.scanner.cur_val = match find_sa(st, level, n) {
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
        st.scanner.cur_val = st.eqtb.equiv(base + n);
    }
    Ok(())
}

/// Integer-valued `\lastnodetype`-style and XeTeX font queries.
fn fetch_last_item_int(st: &mut EngineState, m: i32) -> Result<(), Abort> {
    match m {
        INPUT_LINE_NO_CODE => st.scanner.cur_val = st.mem.line,
        BADNESS_CODE => st.scanner.cur_val = st.shipout.last_badness,
        ELAPSED_TIME_CODE => {
            st.scanner.cur_val = if st.config.deterministic_mode {
                0
            } else {
                st.shipout.get_microinterval()
            };
        }
        RANDOM_SEED_CODE => st.scanner.cur_val = st.errors.random_seed,
        PDF_SHELL_ESCAPE_CODE => st.scanner.cur_val = st.config.shell_escape_enabled as i32,
        ETEX_VERSION_CODE => st.scanner.cur_val = ETEX_VERSION,
        XETEX_VERSION_CODE => st.scanner.cur_val = XETEX_VERSION,
        XETEX_COUNT_GLYPHS_CODE => {
            let (_, engine, fa) = scan_font_engine(st)?;
            st.scanner.cur_val = if fa == AAT_FONT_FLAG {
                aat_font_get(m, engine)
            } else if fa == OTGR_FONT_FLAG {
                unsafe { ot_font_get(m, engine) }
            } else {
                0
            };
        }
        XETEX_COUNT_FEATURES_CODE => {
            let (_, engine, fa) = scan_font_engine(st)?;
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
            scan_font_ident(st)?;
            st.scanner.cur_val = 0;
        }
        XETEX_FEATURE_CODE_CODE | XETEX_IS_EXCLUSIVE_FEATURE_CODE | XETEX_COUNT_SELECTORS_CODE => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                scan_int(st)?;
                st.scanner.cur_val = aat_font_get_1(m, engine, st.scanner.cur_val);
            } else if uses_graphite(fa, engine) {
                scan_int(st)?;
                st.scanner.cur_val = unsafe { ot_font_get_1(m, engine, st.scanner.cur_val) };
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font", -1)?;
            }
        }
        XETEX_SELECTOR_CODE_CODE | XETEX_IS_DEFAULT_SELECTOR_CODE => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                scan_int(st)?;
                st.scanner.cur_val = aat_font_get_2(m, engine, k, st.scanner.cur_val);
            } else if uses_graphite(fa, engine) {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                scan_int(st)?;
                st.scanner.cur_val = unsafe { ot_font_get_2(m, engine, k, st.scanner.cur_val) };
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font", -1)?;
            }
        }
        XETEX_FIND_VARIATION_BY_NAME_CODE => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                crate::io::scan_and_pack_name(st)?;
                st.scanner.cur_val = aat_font_get_named(m, engine);
            } else {
                font_query_error(st, m, f, "; not an AAT font", -1)?;
            }
        }
        XETEX_FIND_FEATURE_BY_NAME_CODE => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                crate::io::scan_and_pack_name(st)?;
                st.scanner.cur_val = aat_font_get_named(m, engine);
            } else if uses_graphite(fa, engine) {
                crate::io::scan_and_pack_name(st)?;
                st.scanner.cur_val = unsafe { gr_font_get_named(st, m, engine) };
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font", -1)?;
            }
        }
        XETEX_FIND_SELECTOR_BY_NAME_CODE => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                crate::io::scan_and_pack_name(st)?;
                st.scanner.cur_val = aat_font_get_named_1(m, engine, k);
            } else if uses_graphite(fa, engine) {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                crate::io::scan_and_pack_name(st)?;
                st.scanner.cur_val = unsafe { gr_font_get_named_1(st, m, engine, k) };
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font", -1)?;
            }
        }
        XETEX_OT_COUNT_SCRIPTS_CODE => {
            let (_, engine, fa) = scan_font_engine(st)?;
            st.scanner.cur_val = if uses_opentype(fa, engine) {
                unsafe { ot_font_get(m, engine) }
            } else {
                0
            };
        }
        XETEX_OT_COUNT_LANGUAGES_CODE | XETEX_OT_SCRIPT_CODE => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if uses_opentype(fa, engine) {
                scan_int(st)?;
                st.scanner.cur_val = unsafe { ot_font_get_1(m, engine, st.scanner.cur_val) };
            } else {
                font_query_error(st, m, f, "; not an OpenType Layout font", -1)?;
            }
        }
        XETEX_OT_COUNT_FEATURES_CODE | XETEX_OT_LANGUAGE_CODE => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if uses_opentype(fa, engine) {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                scan_int(st)?;
                st.scanner.cur_val = unsafe { ot_font_get_2(m, engine, k, st.scanner.cur_val) };
            } else {
                font_query_error(st, m, f, "; not an OpenType Layout font", -1)?;
            }
        }
        XETEX_OT_FEATURE_CODE => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if uses_opentype(fa, engine) {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                scan_int(st)?;
                let kk = st.scanner.cur_val;
                scan_int(st)?;
                st.scanner.cur_val = unsafe { ot_font_get_3(m, engine, k, kk, st.scanner.cur_val) };
            } else {
                font_query_error(st, m, f, "; not an OpenType Layout font", -1)?;
            }
        }
        XETEX_MAP_CHAR_TO_GLYPH_CODE => {
            let f = st.eqtb.cur_font();
            if st.fonts.is_native_font(f) {
                scan_int(st)?;
                st.scanner.cur_val = unsafe { st.fonts.map_char_to_glyph(f, st.scanner.cur_val) }?;
            } else {
                font_query_error(st, m, f, "; not a native platform font", 0)?;
            }
        }
        XETEX_GLYPH_INDEX_CODE => {
            let f = st.eqtb.cur_font();
            if st.fonts.is_native_font(f) {
                crate::io::scan_and_pack_name(st)?;
                let name = st.input.name_of_file_ptr();
                st.scanner.cur_val = unsafe { st.fonts.map_glyph_to_index(f, name) }?;
            } else {
                font_query_error(st, m, f, "; not a native platform font", 0)?;
            }
        }
        XETEX_FONT_TYPE_CODE => {
            let (_, engine, fa) = scan_font_engine(st)?;
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
            scan_font_ident(st)?;
            let f = st.scanner.cur_val;
            let first = m == XETEX_FIRST_CHAR_CODE;
            st.scanner.cur_val = if st.fonts.is_native_font(f) {
                unsafe { st.fonts.get_font_char_range(f, first as i32) }?
            } else if first {
                st.fonts.font_bc[f as usize] as i32
            } else {
                st.fonts.font_ec[f as usize] as i32
            };
        }
        PDF_LAST_X_POS_CODE => st.scanner.cur_val = st.shipout.pdf_last_x_pos,
        PDF_LAST_Y_POS_CODE => st.scanner.cur_val = st.shipout.pdf_last_y_pos,
        XETEX_PDF_PAGE_COUNT_CODE => {
            crate::io::scan_and_pack_name(st)?;
            let name = st.input.name_of_file_ptr();
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
            scan_normal_glue(st)?;
            let q = st.scanner.cur_val;
            let spec = glue_spec(q);
            st.scanner.cur_val = if m == GLUE_STRETCH_ORDER_CODE {
                spec.stretch_order(&st.mem) as i32
            } else {
                spec.shrink_order(&st.mem) as i32
            };
            st.mem.delete_glue_ref(glue_spec(q));
        }
        _ => {}
    }
    st.scanner.cur_val_level = INT_VAL;
    Ok(())
}

fn fetch_last_item_dimen(st: &mut EngineState, m: i32) -> Result<(), Abort> {
    match m {
        XETEX_GLYPH_BOUNDS_CODE => {
            let f = st.eqtb.cur_font();
            if st.fonts.is_native_font(f) {
                scan_int(st)?;
                let edge = st.scanner.cur_val;
                if !(1..=4).contains(&edge) {
                    error_here_with_diagnostic(
                        st,
                        "\\\\XeTeXglyphbounds requires an edge index from 1 to 4;",
                    );
                    print_nl(st, "I don't know anything about edge ");
                    print_int(st, edge);
                    capture_to_diagnostic(st, std::ptr::null_mut());
                    error(st)?;
                    st.scanner.cur_val = 0;
                } else {
                    scan_int(st)?;
                    st.scanner.cur_val =
                        unsafe { st.fonts.get_glyph_bounds(f, edge, st.scanner.cur_val) }?;
                }
            } else {
                font_query_error(st, m, f, "; not a native platform font", 0)?;
            }
        }
        FONT_CHAR_WD_CODE | FONT_CHAR_HT_CODE | FONT_CHAR_DP_CODE | FONT_CHAR_IC_CODE => {
            scan_font_ident(st)?;
            let f = st.scanner.cur_val;
            scan_usv_num(st)?;
            let c = st.scanner.cur_val;
            st.scanner.cur_val = if st.fonts.is_native_font(f) {
                let fonts = &st.fonts;
                unsafe {
                    match m {
                        FONT_CHAR_WD_CODE => fonts.getnativecharwd(f, c),
                        FONT_CHAR_HT_CODE => fonts.getnativecharht(f, c),
                        FONT_CHAR_DP_CODE => fonts.getnativechardp(f, c),
                        _ => fonts.getnativecharic(f, c),
                    }
                }?
            } else if st.fonts.font_bc[f as usize] as i32 <= c
                && st.fonts.font_ec[f as usize] as i32 >= c
            {
                let fu = f as usize;
                let ec = st.fonts.effective_char(f, c as u16);
                let info = st.fonts.font_info_b16(st.fonts.char_base[fu] + ec);
                let fonts = &st.fonts;
                match m {
                    FONT_CHAR_WD_CODE => fonts.char_width(f, info),
                    FONT_CHAR_HT_CODE => fonts.char_height(f, info),
                    FONT_CHAR_DP_CODE => fonts.char_depth(f, info),
                    _ => fonts.char_italic(f, info),
                }
            } else {
                0
            };
        }
        PAR_SHAPE_LENGTH_CODE | PAR_SHAPE_INDENT_CODE | PAR_SHAPE_DIMEN_CODE => {
            let mut q = m - PAR_SHAPE_LENGTH_CODE;
            scan_int(st)?;
            match st.eqtb.par_shape() {
                Some(shape) if st.scanner.cur_val > 0 => {
                    if q == 2 {
                        q = st.scanner.cur_val % 2;
                        st.scanner.cur_val = (st.scanner.cur_val + q) / 2;
                    }
                    let line = st.scanner.cur_val.min(shape.lines(&st.mem));
                    st.scanner.cur_val = if q == 0 {
                        shape.line_width(&st.mem, line)
                    } else {
                        shape.indent(&st.mem, line)
                    };
                }
                _ => st.scanner.cur_val = 0,
            }
        }
        GLUE_STRETCH_CODE | GLUE_SHRINK_CODE => {
            scan_normal_glue(st)?;
            let q = st.scanner.cur_val;
            let spec = glue_spec(q);
            st.scanner.cur_val = if m == GLUE_STRETCH_CODE {
                spec.stretch(&st.mem).0
            } else {
                spec.shrink(&st.mem).0
            };
            st.mem.delete_glue_ref(glue_spec(q));
        }
        _ => {}
    }
    st.scanner.cur_val_level = DIMEN_VAL;
    Ok(())
}

/// `\lastpenalty`, `\lastkern`, `\lastskip` and `\lastnodetype`.
fn fetch_last_node_item(st: &mut EngineState, m: i32) {
    let want_type = m == LAST_NODE_TYPE_CODE;
    let level = m as u8;
    st.scanner.cur_val = 0;
    let head = st.input.cur_list.head;
    let mut tx = st.input.cur_list.tail;
    let mut node = (!st.mem.is_char_node(tx)).then(|| st.mem.classify(MemIdx::new(tx)));
    if let Some(Ok(AnyNode::Math(math))) = node {
        if math.subtype(&st.mem) == END_M_CODE {
            tx = st.mem.predecessor(head, tx);
            node = (!st.mem.is_char_node(tx)).then(|| st.mem.classify(MemIdx::new(tx)));
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
            let node_type = st.mem.type_code(MemIdx::new(tx));
            st.scanner.cur_val = if node_type <= NodeType::Unset.as_raw() {
                node_type as i32 + 1
            } else {
                NodeType::Unset.as_raw() as i32 + 2
            };
        }
        Some(node) if st.input.cur_list.mode != 0 => match (level, node) {
            (INT_VAL, Ok(AnyNode::Penalty(p))) => st.scanner.cur_val = p.penalty(&st.mem),
            (DIMEN_VAL, Ok(AnyNode::Kern(k))) => st.scanner.cur_val = k.width(&st.mem).0,
            (GLUE_VAL, Ok(AnyNode::Glue(g))) => {
                st.scanner.cur_val = g.glue_ptr(&st.mem).ptr().0;
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
                        if let Some(spec) = st.shipout.last_glue {
                            st.scanner.cur_val = spec.ptr().0;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Scans an internal quantity of at most `level`, negated if `negative`.
pub(crate) fn scan_something_internal(
    st: &mut EngineState,
    level: u8,
    negative: bool,
) -> Result<(), Abort> {
    loop {
        let m = st.scanner.cur_chr;
        match st.scanner.cur_cmd {
            DEF_CODE => fetch_def_code(st, m)?,
            XETEX_DEF_CODE => fetch_xetex_def_code(st, m)?,
            TOKS_REGISTER | ASSIGN_TOKS | DEF_FAMILY | SET_FONT | DEF_FONT => {
                fetch_token_list(st, m, level)?
            }
            ASSIGN_INT => set_cur_val(st, st.eqtb.int_at(m), INT_VAL),
            ASSIGN_DIMEN => set_cur_val(st, st.eqtb.int_at(m), DIMEN_VAL),
            ASSIGN_GLUE => set_cur_val(st, st.eqtb.equiv(m), GLUE_VAL),
            ASSIGN_MU_GLUE => set_cur_val(st, st.eqtb.equiv(m), MU_VAL),
            SET_AUX => {
                if st.input.cur_list.mode.abs() as i32 != m {
                    error_here_with_diagnostic(st, "Improper ");
                    print_cmd_chr(st, SET_AUX, m);
                    capture_to_diagnostic(st, std::ptr::null_mut());
                    st.errors.help(&[
                        r"You can refer to \spacefactor only in horizontal mode;",
                        r"you can refer to \prevdepth only in vertical mode; and",
                        r"neither of these is meaningful inside \write. So",
                        "I'm forgetting what you said and using zero instead.",
                    ]);
                    error(st)?;
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
                let v = if st.shipout.page_contents == crate::state::PageContents::Empty
                    && !st.shipout.output_active
                {
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
                    scan_int(st)?;
                    st.scanner.cur_val = match st.eqtb.penalties(m) {
                        Some(penalties) if st.scanner.cur_val >= 0 => {
                            let k = st.scanner.cur_val.min(penalties.count(&st.mem));
                            penalties.penalty(&st.mem, k)
                        }
                        _ => 0,
                    };
                } else {
                    st.scanner.cur_val = st.eqtb.par_shape().map_or(0, |s| s.lines(&st.mem));
                }
                st.scanner.cur_val_level = INT_VAL;
            }
            SET_BOX_DIMEN => {
                scan_register_num(st)?;
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
                find_font_dimen(st, false)?;
                st.fonts.set_font_info_sc(st.fonts.fmem_ptr, 0);
                set_cur_val(st, st.fonts.font_info_sc(st.scanner.cur_val), DIMEN_VAL);
            }
            ASSIGN_FONT_INT => {
                scan_font_ident(st)?;
                let f = st.scanner.cur_val;
                match m {
                    0 => set_cur_val(st, st.fonts.hyphen_char[f as usize], INT_VAL),
                    1 => set_cur_val(st, st.fonts.skew_char[f as usize], INT_VAL),
                    _ => {
                        if st.fonts.is_native_font(f) {
                            scan_glyph_number(st, f)?;
                        } else {
                            scan_char_num(st)?;
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
            REGISTER => fetch_register(st, m)?,
            LAST_ITEM => {
                if m < INPUT_LINE_NO_CODE {
                    fetch_last_node_item(st, m);
                } else if m < XETEX_DIM {
                    fetch_last_item_int(st, m)?;
                } else if m < ETEX_GLUE {
                    fetch_last_item_dimen(st, m)?;
                } else {
                    if m < ETEX_MU {
                        if m == MU_TO_GLUE_CODE {
                            scan_mu_glue(st)?;
                        }
                        st.scanner.cur_val_level = GLUE_VAL;
                    } else if m < ETEX_EXPR {
                        if m == GLUE_TO_MU_CODE {
                            scan_normal_glue(st)?;
                        }
                        st.scanner.cur_val_level = MU_VAL;
                    } else {
                        st.scanner.cur_val_level = (m - ETEX_EXPR) as u8;
                        scan_expr(st)?;
                    }
                    while st.scanner.cur_val_level > level {
                        if st.scanner.cur_val_level == GLUE_VAL {
                            glue_to_width(st);
                        } else if st.scanner.cur_val_level == MU_VAL {
                            mu_error(st)?;
                        }
                        st.scanner.cur_val_level -= 1;
                    }
                    if negative {
                        if st.scanner.cur_val_level >= GLUE_VAL {
                            let old = negate_glue(st)?;
                            st.mem.delete_glue_ref(glue_spec(old));
                        } else {
                            st.scanner.cur_val = -st.scanner.cur_val;
                        }
                    }
                    return Ok(());
                }
            }
            IGNORE_SPACES => {
                if m == 1 {
                    get_primitive_token(st)?;
                    continue;
                }
            }
            _ => {
                error_here_with_diagnostic(st, "You can't use `");
                print_cmd_chr(st, st.scanner.cur_cmd, m);
                print(st, "' after ");
                print_esc(st, "the");
                capture_to_diagnostic(st, std::ptr::null_mut());
                st.errors
                    .help(&["I'm forgetting what you said and using zero instead."]);
                error(st)?;
                set_cur_val(st, 0, if level != TOK_VAL { DIMEN_VAL } else { INT_VAL });
            }
        }
        break;
    }

    while st.scanner.cur_val_level > level {
        if st.scanner.cur_val_level == GLUE_VAL {
            st.scanner.cur_val = st.scanner.cur_glue().width(&st.mem).0;
        } else if st.scanner.cur_val_level == MU_VAL {
            mu_error(st)?;
        }
        st.scanner.cur_val_level -= 1;
    }

    if negative {
        if st.scanner.cur_val_level >= GLUE_VAL {
            negate_glue(st)?;
        } else {
            st.scanner.cur_val = -st.scanner.cur_val;
        }
    } else if (GLUE_VAL..=MU_VAL).contains(&(st.scanner.cur_val_level)) {
        let spec = st.scanner.cur_glue();
        let refs = spec.ref_count(&st.mem);
        spec.set_ref_count(&mut st.mem, refs + 1);
    }
    Ok(())
}

pub(crate) fn scan_font_ident(st: &mut EngineState) -> Result<(), Abort> {
    get_x_non_blank(st)?;
    let f = match st.scanner.cur_cmd {
        DEF_FONT => st.eqtb.cur_font(),
        SET_FONT => st.scanner.cur_chr,
        DEF_FAMILY => {
            let m = st.scanner.cur_chr;
            scan_math_fam_int(st)?;
            st.eqtb.equiv(m + st.scanner.cur_val)
        }
        _ => {
            print_err(st, "Missing font identifier");
            st.errors.help(&[
                "I was looking for a control sequence whose",
                r"current meaning has been defined by \font.",
            ]);
            back_error(st)?;
            FONT_BASE
        }
    };
    st.scanner.cur_val = f;
    Ok(())
}

pub(crate) fn find_font_dimen(st: &mut EngineState, writing: bool) -> Result<(), Abort> {
    scan_int(st)?;
    let n = st.scanner.cur_val;
    scan_font_ident(st)?;
    let f = st.scanner.cur_val;
    let fu = f as usize;

    if n <= 0 {
        st.scanner.cur_val = st.fonts.fmem_ptr;
    } else {
        if writing
            && (SPACE_CODE..=SPACE_SHRINK_CODE).contains(&n)
            && st.fonts.font_glue[fu] != TEX_NULL
        {
            st.mem.delete_glue_ref(glue_spec(st.fonts.font_glue[fu]));
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
                    return Err(Abort::Overflow {
                        what: "font memory",
                        n: st.fonts.font_mem_size,
                    });
                }
                st.fonts.set_font_info_sc(st.fonts.fmem_ptr, 0);
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
        error_here_with_diagnostic(st, "Font ");
        crate::output::print_esc(st, st.eqtb.hash_s1(FONT_ID_BASE + f));
        print(st, " has only ");
        print_int(st, st.fonts.font_params[fu]);
        print(st, " fontdimen parameters");
        capture_to_diagnostic(st, std::ptr::null_mut());
        st.errors.help(&[
            "To increase the number of font parameters, you must",
            r"use \fontdimen immediately after the \font is loaded.",
        ]);
        error(st)?;
    }
    Ok(())
}

pub(crate) fn scan_glyph_number(st: &mut EngineState, f: i32) -> Result<(), Abort> {
    if scan_keyword(st, "/")? {
        crate::io::scan_and_pack_name(st)?;
        let name = st.input.name_of_file_ptr();
        let v = unsafe { st.fonts.map_glyph_to_index(f, name) }?;
        set_cur_val(st, v, INT_VAL);
    } else if scan_keyword(st, "u")? {
        scan_char_num(st)?;
        let v = unsafe { st.fonts.map_char_to_glyph(f, st.scanner.cur_val) }?;
        set_cur_val(st, v, INT_VAL);
    } else {
        scan_int(st)?;
    }
    Ok(())
}

pub(crate) fn get_token(st: &mut EngineState) -> Result<(), Abort> {
    st.eqtb.no_new_control_sequence = false;
    get_next(st)?;
    st.eqtb.no_new_control_sequence = true;
    pack_cur_tok(st);
    Ok(())
}

/// Scans a balanced text into a list at `link(temp_head)`; `cur_val` is its tail.
pub(crate) fn scan_general_text(st: &mut EngineState) -> Result<(), Abort> {
    let saved_status = st.scanner.scanner_status;
    let saved_warning_index = st.scanner.warning_index;
    let saved_def_ref = st.scanner.def_ref;
    st.scanner.scanner_status = ABSORBING;
    st.scanner.warning_index = st.scanner.cur_cs;
    let mut p = new_def_ref(st)?;
    scan_left_brace(st)?;
    let mut unbalance: i32 = 1;
    loop {
        get_token(st)?;
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
    st.mem.free_avail(def_ref);
    st.scanner.cur_val = if list.is_null() { TEMP_HEAD.0 } else { p.0 };
    TEMP_HEAD.set_link(&mut st.mem, list);
    st.scanner.scanner_status = saved_status;
    st.scanner.warning_index = saved_warning_index;
    st.scanner.def_ref = saved_def_ref;
    Ok(())
}

pub(crate) fn scan_pdf_ext_toks(st: &mut EngineState) -> Result<(), Abort> {
    scan_toks(st, false, true)?;
    Ok(())
}

fn scan_toks_to_string(st: &mut EngineState) -> Result<i32, Abort> {
    scan_toks(st, false, true)?;
    let def_ref = st.scanner.def_ref.0;
    let s = tokens_to_string(st, def_ref)?;
    st.mem.delete_token_ref(st.scanner.def_ref);
    Ok(s)
}

/// `\strcmp`: compares two general texts as strings.
pub(crate) fn compare_strings(st: &mut EngineState) -> Result<(), Abort> {
    let save_cur_cs = st.scanner.cur_cs;
    let s1 = scan_toks_to_string(st)?;
    st.scanner.cur_cs = save_cur_cs;
    let s2 = scan_toks_to_string(st)?;

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

    st.strings.flush_str(s2);
    st.strings.flush_str(s1);
    set_cur_val(st, result, INT_VAL);
    Ok(())
}
