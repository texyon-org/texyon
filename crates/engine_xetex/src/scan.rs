//! The tokenizer (`get_next`) and the scanning routines built on it.

use crate::bridge::output_flush;
use crate::cmd::{
    ACTIVE_CHAR, ASSIGN_DIMEN, ASSIGN_FONT_DIMEN, ASSIGN_FONT_INT, ASSIGN_GLUE, ASSIGN_INT,
    ASSIGN_MU_GLUE, ASSIGN_TOKS, CALL, CAR_RET, CHAR_GIVEN, CHAR_NUM, DEF_CODE, DEF_FAMILY,
    DEF_FONT, DELIM_NUM, DONT_EXPAND, ENDV, END_TEMPLATE, FI_OR_ELSE, IF_TEST, IGNORE_SPACES,
    LAST_ITEM, LEFT_BRACE, LETTER, MAC_PARAM, MATH_CHAR_NUM, MATH_GIVEN, MAX_COMMAND, MAX_INTERNAL,
    MIN_INTERNAL, OMIT, OTHER_CHAR, OUTER_CALL, OUT_PARAM, REGISTER, RELAX, RIGHT_BRACE, SET_AUX,
    SET_BOX_DIMEN, SET_FONT, SET_PAGE_DIMEN, SET_PAGE_INT, SET_PREV_GRAF, SET_SHAPE, SPACER,
    SUP_MARK, TAB_MARK, THE, TOKS_REGISTER, VRULE, XETEX_DEF_CODE, XETEX_MATH_GIVEN,
};
use crate::codes::{
    BoxDimenCode, DelimNumCode, FiOrElseCode, IfCode, LastItemCode, MathCharNumCode, PageIntCode,
    ScannerStatus, TheCode, ValueLevel,
};
use crate::consts::{
    ACTIVE_BASE, ACTIVE_MATH_CHAR, BIGGEST_CHAR, BIGGEST_USV, CHAR_CLASS_LIMIT, CLOSED, COUNT_BASE,
    DEL_CODE_BASE, END_M_CODE, EQTB_SIZE, FIL, FILLL, FROZEN_CONTROL_SEQUENCE, FROZEN_CR,
    FROZEN_ENDV, FROZEN_FI, FROZEN_PRIMITIVE, FROZEN_PROTECTION, FROZEN_RELAX, JUST_OPEN,
    LEFT_SIDE, MATH_CHAR, MATH_CODE_BASE, MAX_DIMEN, MAX_HALFWORD, MU_GLUE, MU_SKIP_BASE, NORMAL,
    NULL_CS, NULL_FONT, NUMBER_MATH_FAMILIES, PAR_SHAPE_LOC, PRIM_EQTB_BASE, RIGHT_SIDE,
    SCALED_BASE, SF_CODE_BASE, SINGLE_BASE, SKIP_BASE, TEX_INFINITY, TEX_NULL, UNDEFINED_PRIMITIVE,
    VMODE,
};
use crate::errors::{error, int_error, mu_error, Abort};
use crate::expand::{expand, macro_call};
use crate::ext::{
    aat_font_get, aat_font_get_1, aat_font_get_2, aat_font_get_named, aat_font_get_named_1,
    gr_font_get_named, gr_font_get_named_1, ot_font_get, ot_font_get_1, ot_font_get_2,
    ot_font_get_3,
};
use crate::fonts::{AAT_FONT_FLAG, QUAD_CODE, SPACE_CODE, SPACE_SHRINK_CODE, X_HEIGHT_CODE};
use crate::input::{
    back_error, back_input, begin_token_list, ins_error, TokenType, MID_LINE, NEW_LINE,
    SKIP_BLANKS, TOKEN_LIST,
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
use crate::params::{IntPar, TokPar};
use crate::state::{
    EngineState, EqtbState, ErrorState, FontEngineHandle, GroupCode, InputClusterState, MemState,
    ScannerState, Selector,
};
use crate::token::Token;
use crate::units::{FontId, MemIdx, MemPtr, NodeType, Scaled, StrNumber};

use std::sync::Arc;

use tectonic_xetex_layout::get_cp_code;

const ZERO_TOKEN: Token = Token::other(b'0');
const MINUS_TOKEN: Token = Token::other(b'-');
const PLUS_TOKEN: Token = Token::other(b'+');
const A_TOKEN: Token = Token::letter(b'A');
const OTHER_A_TOKEN: Token = Token::other(b'A');
const HEX_TOKEN: Token = Token::other(b'"');
const OCTAL_TOKEN: Token = Token::other(b'\'');
const CONTINENTAL_POINT_TOKEN: Token = Token::other(b',');
const POINT_TOKEN: Token = Token::other(b'.');
const ALPHA_TOKEN: Token = Token::other(b'`');
const FROZEN_PRIM_TOKEN: Token = Token::from_cs(FROZEN_PRIMITIVE);

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

/// Reports a bad integer `value` with two help lines.
fn bad_int_error(
    st: &mut EngineState,
    err: &str,
    value: i32,
    help1: &'static str,
    help0: &'static str,
) -> Result<(), Abort> {
    error_here_with_diagnostic(st, err);
    crate::output::append_to_diagnostic(st, &format!(" ({value})"));
    capture_to_diagnostic(st, None);
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
    if math_char(st.scanner.cur_val) == ACTIVE_MATH_CHAR {
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
    } else if math_char(st.scanner.cur_val) > BIGGEST_USV {
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
        self.int_par(IntPar::XetexHyphenatableLength)
            .min(HYPHENATABLE_LENGTH_LIMIT)
    }
}

pub(crate) fn insert_dollar_sign(st: &mut EngineState) -> Result<(), Abort> {
    back_input(st)?;
    st.scanner.cur_tok = Token::math_shift(b'$');
    print_err(st, "Missing $ inserted");
    st.errors.help(&[
        "I've inserted a begin-math/end-math symbol since I think",
        "you left one out. Proceed, with fingers crossed.",
    ]);
    ins_error(st)
}

pub(crate) fn scan_optional_equals(st: &mut EngineState) -> Result<(), Abort> {
    get_x_non_blank(st)?;
    if st.scanner.cur_tok != Token::other(b'=') {
        back_input(st)?;
    }
    Ok(())
}

impl MemState {
    /// Whether the macro whose token list starts at `ref_count` is `\protected`.
    #[inline]
    pub(crate) fn is_protected_macro(&self, ref_count: MemPtr) -> bool {
        ref_count.link(self).token(self) == Token::PROTECTED
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
    st.save_stack.set_saved(0, i32::from(spec_code));
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
            .fold(0, |a, &d| (a + i32::from(d) * 0x2_0000) / 10);
        (a + 1) / 2
    }
}

pub(crate) fn scan_rule_spec(st: &mut EngineState) -> Result<NodePtr<RuleNode>, Abort> {
    let r = st.mem.new_rule()?;
    if st.scanner.cur_cmd == VRULE {
        r.set_width(&mut st.mem, RuleDimen::from_raw(DEFAULT_RULE));
    } else {
        r.set_height(&mut st.mem, RuleDimen::from_raw(DEFAULT_RULE));
        r.set_depth(&mut st.mem, RuleDimen::Set(Scaled::ZERO));
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
            let mut c = i32::from(strings.str_pool[k as usize]);

            let t = if c == i32::from(b' ') && cat == 0 {
                Token::SPACE
            } else {
                if (0xD800..0xDC00).contains(&c) && k + 1 < strings.pool_ptr {
                    let lo = i32::from(strings.str_pool[(k + 1) as usize]);
                    if (0xDC00..0xE000).contains(&lo) {
                        k += 1;
                        c = 0x10000
                            + ((c - 0xD800) << 10)
                            + (i32::from(strings.str_pool[k as usize]) - 0xDC00);
                    }
                }

                if cat == 0 {
                    Token::new(OTHER_CHAR, c)
                } else if cat == i16::from(ACTIVE_CHAR) {
                    Token::from_cs(ACTIVE_BASE + c)
                } else {
                    Token::new(cat as u8, c)
                }
            };

            let q = self.get_avail()?;
            p.set_link(self, q);
            q.set_token(self, t);
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
    let end_char = st.eqtb.int_par(IntPar::EndLineChar);
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
    if crate::io::u_close(slot) {
        return Err(Abort::InputClose);
    }
    st.input.read_open[m] = CLOSED;
    Ok(())
}

pub(crate) fn read_toks(st: &mut EngineState, n: i32, r: i32, j: i32) -> Result<(), Abort> {
    st.scanner.scanner_status = ScannerStatus::Defining;
    st.scanner.warning_index = r;
    let def_ref = new_def_ref(st)?;
    let mut p = def_ref;
    store_new_token(st, &mut p, Token::END_MATCH)?;

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
            if crate::io::input_slot_line(st, crate::io::FileSlot::Read(m))? != 0 {
                st.input.read_open[m] = NORMAL as u8;
            } else {
                close_read_file(st, m)?;
            }
        } else {
            if crate::io::input_slot_line(st, crate::io::FileSlot::Read(m))? == 0 {
                close_read_file(st, m)?;
                if st.scanner.align_state != 1_000_000 {
                    crate::errors::runaway(st);
                    error_here_with_diagnostic(st, "File ended within ");
                    print_esc(st, "read");
                    capture_to_diagnostic(st, None);
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
                let tok = if c == i32::from(b' ') {
                    Token::SPACE
                } else {
                    Token::new(OTHER_CHAR, c)
                };
                st.scanner.cur_chr = c;
                st.scanner.cur_tok = tok;
                store_new_token(st, &mut p, tok)?;
            }
        } else {
            loop {
                get_token(st)?;
                if st.scanner.cur_tok == Token::ZERO {
                    break;
                }
                if st.scanner.align_state < 1_000_000 {
                    loop {
                        get_token(st)?;
                        if st.scanner.cur_tok == Token::ZERO {
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
    st.scanner.scanner_status = ScannerStatus::Normal;
    st.scanner.align_state = s;
    Ok(())
}

#[allow(non_snake_case)]
pub(crate) fn eTeX_enabled(st: &mut EngineState, b: bool, j: u8, k: i32) -> Result<bool, Abort> {
    if !b {
        error_here_with_diagnostic(st, "Improper ");
        print_cmd_chr(st, j, k);
        capture_to_diagnostic(st, None);
        st.errors
            .help(&["Sorry, this optional e-TeX feature has been disabled."]);
        error(st)?;
    }
    Ok(b)
}

pub(crate) fn tokens_to_string(st: &mut EngineState, p: i32) -> Result<StrNumber, Abort> {
    if st.shipout.selector == Selector::NewString {
        return Err(Abort::PdfError(&[
            "tokens",
            "tokens_to_string() called while selector = new_string",
        ]));
    }
    st.shipout.old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    let arg = MemPtr(p).link(&st.mem);
    let l = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list(st, arg, TEX_NULL, l);
    st.shipout.selector = st.shipout.old_setting;
    st.strings.make_string()
}

pub(crate) fn get_r_token(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        loop {
            get_token(st)?;
            if st.scanner.cur_tok != Token::SPACE {
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
            st.scanner.cur_tok = Token::from_cs(FROZEN_PROTECTION);
            ins_error(st)?;
        } else {
            return Ok(());
        }
    }
}

/// Converts a classic 15-bit `\mathcode` or `\mathchar` to XeTeX's layout.
pub(crate) fn math_code_from_tex(v: i32) -> i32 {
    set_class(v / 4096) + set_family((v % 4096) / 256) + (v % 256)
}

/// Scans the class, family and character of a `\Umathchar`-style code.
pub(crate) fn scan_umath_code(st: &mut EngineState) -> Result<i32, Abort> {
    scan_math_class_int(st)?;
    let class = set_class(st.scanner.cur_val);
    scan_math_fam_int(st)?;
    let family = set_family(st.scanner.cur_val);
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
                MATH_CHAR_NUM => scan_math_char_num(st)?,
                MATH_GIVEN => math_code_from_tex(st.scanner.cur_chr),
                XETEX_MATH_GIVEN => st.scanner.cur_chr,
                DELIM_NUM => scan_delim_num(st)?,
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
    let cur_fam = st.eqtb.int_par(IntPar::CurFam);
    let fam = if math_class(c) == 7 && (0..NUMBER_MATH_FAMILIES).contains(&cur_fam) {
        cur_fam as u16
    } else {
        math_fam(c) as u16
    };
    p.set_plane_and_fam(&mut st.mem, fam + (math_char(c) / 65536) as u16 * 256);
    Ok(())
}

pub(crate) fn set_math_char(st: &mut EngineState, c: i32) -> Result<(), Abort> {
    if math_char(c) == ACTIVE_MATH_CHAR {
        st.scanner.cur_cs = st.scanner.cur_chr + 1;
        fetch_cs_meaning(st);
        x_token(st)?;
        back_input(st)?;
    } else {
        let nd = st.mem.new_noad()?;
        nd.nucleus().set_math_type(&mut st.mem, MATH_CHAR);
        let ch = math_char(c);
        nd.nucleus().set_character(&mut st.mem, (ch % 65536) as u16);
        nd.nucleus()
            .set_plane_and_fam(&mut st.mem, math_fam(c) as u16);
        if math_class(c) == 7 {
            let cf = st.eqtb.int_par(IntPar::CurFam);
            if (0..NUMBER_MATH_FAMILIES).contains(&cf) {
                nd.nucleus().set_plane_and_fam(&mut st.mem, cf as u16);
            }
            nd.set_noad_type(&mut st.mem, NodeType::OrdNoad);
        } else {
            nd.set_noad_type(
                &mut st.mem,
                NodeType::from_raw(NodeType::OrdNoad.as_raw() + math_class(c) as u16)
                    .expect("valid noad type"),
            );
        }
        let fam = nd.nucleus().plane_and_fam(&st.mem) + (ch / 65536) as u16 * 256;
        nd.nucleus().set_plane_and_fam(&mut st.mem, fam);
        st.input.cur_list.tail_append(&mut st.mem, nd.ptr());
    }
    Ok(())
}

// The fields of a XeTeX math code: class in bits 21..24, family in 24..32, char below.
#[inline]
pub(crate) fn math_char(c: i32) -> i32 {
    (c as u32 & 0x1FFFFF) as i32
}
#[inline]
pub(crate) fn math_class(c: i32) -> i32 {
    ((c as u32 >> 21) & 0x07) as i32
}
#[inline]
pub(crate) fn math_fam(c: i32) -> i32 {
    ((c as u32 >> 24) & 0xFF) as i32
}
#[inline]
pub(crate) fn set_class(x: i32) -> i32 {
    (((x as u32) & 0x07) << 21) as i32
}
#[inline]
pub(crate) fn set_family(x: i32) -> i32 {
    (((x as u32) & 0xFF) << 24) as i32
}

pub(crate) fn check_outer_validity(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.scanner_status == ScannerStatus::Normal {
        return Ok(());
    }
    st.errors.deletions_allowed = false;

    if st.scanner.cur_cs != 0 {
        let ci = st.input.cur_input;
        if ci.state == TOKEN_LIST || ci.name < 1 || ci.name > 17 {
            let p = st.mem.get_avail()?;
            p.set_token(&mut st.mem, Token::from_cs(st.scanner.cur_cs));
            begin_token_list(st, p, TokenType::BackedUp)?;
        }
        st.scanner.cur_cmd = SPACER;
        st.scanner.cur_chr = i32::from(b' ');
    }

    let outcome = if st.scanner.scanner_status > ScannerStatus::Skipping {
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
            ScannerStatus::Defining => {
                print(st, " while scanning definition");
                p.set_token(&mut st.mem, Token::right_brace(b'}'));
            }
            ScannerStatus::Matching => {
                print(st, " while scanning use");
                p.set_token(&mut st.mem, st.input.par_token);
                st.scanner.long_state = OUTER_CALL;
            }
            ScannerStatus::Aligning => {
                print(st, " while scanning preamble");
                p.set_token(&mut st.mem, Token::right_brace(b'}'));
                inserted = st.mem.get_avail()?;
                inserted.set_link(&mut st.mem, p);
                inserted.set_token(&mut st.mem, Token::from_cs(FROZEN_CR));
                st.scanner.align_state = -1_000_000;
            }
            ScannerStatus::Absorbing => {
                print(st, " while scanning text");
                p.set_token(&mut st.mem, Token::right_brace(b'}'));
            }
            _ => {}
        }

        begin_token_list(st, inserted, TokenType::Inserted)?;
        print(st, " of ");
        let warning_index = st.scanner.warning_index;
        crate::output::sprint_cs(st, warning_index);
        capture_to_diagnostic(st, None);
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
        print_cmd_chr(st, IF_TEST, i32::from(cur_if));
        print(st, "; all text was ignored after line ");
        let skip_line = st.scanner.skip_line;
        print_int(st, skip_line);
        capture_to_diagnostic(st, None);
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
        st.scanner.cur_tok = Token::from_cs(FROZEN_FI);
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
    if c <= i32::from(b'9') {
        c - i32::from(b'0')
    } else {
        c - i32::from(b'a') + 10
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
                        match i32::from(st.input.cur_input.state) + i32::from(cat) {
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
                                st.scanner.cur_chr = i32::from(b' ');
                            }

                            // End of line in mid-line.
                            6 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                st.scanner.cur_cmd = SPACER;
                                st.scanner.cur_chr = i32::from(b' ');
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
                            let slot =
                                crate::io::FileSlot::Input(st.input.cur_input.index as usize);
                            crate::io::input_slot_line(st, slot)? != 0
                        };
                        if got_line {
                            st.input.cur_input.limit = st.input.last;
                        } else {
                            let index = st.input.cur_input.index as usize;
                            let every_eof = st.eqtb.tok_par(TokPar::EveryEof);
                            if !every_eof.is_null() && !st.input.eof_seen[index] {
                                st.input.cur_input.limit = st.input.first - 1;
                                st.input.eof_seen[index] = true;
                                begin_token_list(st, every_eof, TokenType::EveryEof)?;
                                continue 'restart;
                            }
                            st.input.force_eof = true;
                        }
                    }

                    if st.input.force_eof {
                        let in_open = st.input.in_open as usize;
                        if st.eqtb.int_par(IntPar::TracingNesting) > 0
                            && (st.input.grp_stack[in_open] != st.save_stack.cur_boundary
                                || st.input.if_stack[in_open] != st.scanner.cond_ptr)
                        {
                            crate::output::file_warning(st);
                        }
                        if st.input.cur_input.name >= 19 {
                            crate::output::print_char(st, b')');
                            st.input.open_parens -= 1;
                            output_flush(st.shipout.rust_stdout);
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
                    let coda = st.eqtb.tok_par(TokPar::TectonicCoda);
                    if !st.scanner.used_tectonic_coda_tokens && !coda.is_null() {
                        st.scanner.used_tectonic_coda_tokens = true;
                        begin_token_list(st, coda, TokenType::TectonicCoda)?;
                        continue 'restart;
                    }
                    return Err(Abort::NoLegalEnd {
                        open_log: st.shipout.selector < Selector::LogOnly,
                    });
                }
            }
        } else if st.input.cur_input.loc != TEX_NULL {
            let p = MemPtr(st.input.cur_input.loc);
            let t = p.token(&st.mem);
            st.input.cur_input.loc = p.link(&st.mem).0;

            if t.is_cs() {
                st.scanner.cur_cs = t.cs();
                fetch_cs_meaning(st);
                if st.scanner.cur_cmd >= OUTER_CALL {
                    if st.scanner.cur_cmd == DONT_EXPAND {
                        st.scanner.cur_cs = MemPtr(st.input.cur_input.loc).token(&st.mem).cs();
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
                st.scanner.cur_cmd = t.cmd();
                st.scanner.cur_chr = t.chr();
                match st.scanner.cur_cmd {
                    LEFT_BRACE => st.scanner.align_state += 1,
                    RIGHT_BRACE => st.scanner.align_state -= 1,
                    OUT_PARAM => {
                        let param = st.input.param_stack
                            [(st.input.cur_input.limit + st.scanner.cur_chr - 1) as usize];
                        begin_token_list(st, param, TokenType::Parameter)?;
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
            if st.scanner.scanner_status == ScannerStatus::Aligning
                || st.scanner.cur_align.0 == TEX_NULL
            {
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
            begin_token_list(st, template, TokenType::VTemplate)?;
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
    token: Token,
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
        find_sa(st, ValueLevel::BOX, n)?.sa_ptr(&st.mem)
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
        if let Some(sa) = self.find_sa_element(scanner, ValueLevel::BOX, n) {
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
        Token::new(st.scanner.cur_cmd, st.scanner.cur_chr)
    } else {
        Token::from_cs(st.scanner.cur_cs)
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
/// The math code given by `\mathchar` or a XeTeX form, per `cur_chr`.
pub(crate) fn scan_math_char_num(st: &mut EngineState) -> Result<i32, Abort> {
    Ok(match MathCharNumCode::from_raw(st.scanner.cur_chr) {
        Some(MathCharNumCode::UmathChar) => scan_umath_code(st)?,
        Some(MathCharNumCode::UmathCharNum) => {
            scan_xetex_math_char_int(st)?;
            st.scanner.cur_val
        }
        _ => {
            scan_fifteen_bit_int(st)?;
            math_code_from_tex(st.scanner.cur_val)
        }
    })
}

/// The math code of the small character given by `\delimiter` or `\Udelimiter`.
pub(crate) fn scan_delim_num(st: &mut EngineState) -> Result<i32, Abort> {
    Ok(match DelimNumCode::from_raw(st.scanner.cur_chr) {
        Some(DelimNumCode::Udelimiter) => scan_umath_code(st)?,
        _ => {
            scan_delimiter_int(st)?;
            math_code_from_tex(st.scanner.cur_val / 4096)
        }
    })
}

pub(crate) fn the_toks(st: &mut EngineState, code: TheCode) -> Result<MemPtr, Abort> {
    if code != TheCode::The {
        scan_general_text(st)?;
        if code == TheCode::Unexpanded {
            return Ok(MemPtr::from_raw(st.scanner.cur_val));
        }
        let old_setting = st.shipout.selector;
        st.shipout.selector = Selector::NewString;
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
    scan_something_internal(st, ValueLevel::Tok, false)?;
    if st.scanner.cur_val_level >= ValueLevel::Ident {
        let mut p = TEMP_HEAD;
        p.set_link(&mut st.mem, MemPtr::NULL);
        if st.scanner.cur_val_level == ValueLevel::Ident {
            store_new_token(st, &mut p, Token::from_cs(st.scanner.cur_val))?;
        } else if st.scanner.cur_val != TEX_NULL {
            let mut r = MemPtr::from_raw(st.scanner.cur_val).link(&st.mem);
            while !r.is_null() {
                store_new_token(st, &mut p, r.token(&st.mem))?;
                r = r.link(&st.mem);
            }
        }
        Ok(p)
    } else {
        let old_setting = st.shipout.selector;
        st.shipout.selector = Selector::NewString;
        let b = st.strings.pool_ptr;
        match st.scanner.cur_val_level {
            ValueLevel::Int => print_int(st, st.scanner.cur_val),
            ValueLevel::Dimen => {
                crate::output::print_scaled(st, st.scanner.cur_dimen());
                print(st, "pt");
            }
            ValueLevel::Glue => {
                crate::output::print_spec(st, st.scanner.cur_val, Some("pt"));
                st.mem.delete_glue_ref(st.scanner.cur_glue());
            }
            ValueLevel::Mu => {
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
        st.scanner.scanner_status = ScannerStatus::Defining;
    } else {
        st.scanner.scanner_status = ScannerStatus::Absorbing;
    }
    st.scanner.warning_index = st.scanner.cur_cs;
    let def_ref = new_def_ref(st)?;
    let mut p = def_ref;
    let mut hash_brace = Token::ZERO;
    let mut t = ZERO_TOKEN;

    'found: {
        if macro_def {
            let ended_with_brace = loop {
                get_token(st)?;
                if st.scanner.cur_tok.is_brace() {
                    break false;
                }
                if st.scanner.cur_cmd == MAC_PARAM {
                    let s = Token::match_(st.scanner.cur_chr);
                    get_token(st)?;
                    if st.scanner.cur_tok.is_left_brace() {
                        hash_brace = st.scanner.cur_tok;
                        store_new_token(st, &mut p, st.scanner.cur_tok)?;
                        store_new_token(st, &mut p, Token::END_MATCH)?;
                        break true;
                    }
                    if t == Token::other(b'9') {
                        print_err(st, "You already have nine parameters");
                        st.errors.help(&[
                            "I'm going to ignore the # sign you just used,",
                            "as well as the token that followed it.",
                        ]);
                        error(st)?;
                        continue;
                    }
                    t = Token(t.0 + 1);
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
                store_new_token(st, &mut p, Token::END_MATCH)?;
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
                        let tail = the_toks(st, crate::expand::the_code(st)?)?;
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

            if st.scanner.cur_tok.is_brace() {
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
                        capture_to_diagnostic(st, None);
                        st.errors.help(&[
                            "You meant to type ## instead of #, right?",
                            "Or maybe a } was forgotten somewhere earlier, and things",
                            "are all screwed up? I'm going to assume that you meant ##.",
                        ]);
                        back_error(st)?;
                        st.scanner.cur_tok = s;
                    } else {
                        st.scanner.cur_tok = Token::out_param(st.scanner.cur_chr - i32::from(b'0'));
                    }
                }
            }
            store_new_token(st, &mut p, st.scanner.cur_tok)?;
        }
    }
    st.scanner.scanner_status = ScannerStatus::Normal;
    if hash_brace != Token::ZERO {
        store_new_token(st, &mut p, hash_brace)?;
    }
    Ok(p)
}

pub(crate) fn pass_text(st: &mut EngineState) -> Result<(), Abort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = ScannerStatus::Skipping;
    let mut l: i32 = 0;
    st.scanner.skip_line = st.mem.line;

    loop {
        get_next(st)?;
        if st.scanner.cur_cmd == FI_OR_ELSE {
            if l == 0 {
                break;
            }
            if st.scanner.cur_chr == FiOrElseCode::Fi.raw() {
                l -= 1;
            }
        } else if st.scanner.cur_cmd == IF_TEST {
            l += 1;
        }
    }
    st.scanner.scanner_status = save_scanner_status;
    if st.eqtb.int_par(IntPar::TracingIfs) > 0 {
        crate::output::show_cur_cmd_chr(st);
    }
    Ok(())
}
pub(crate) fn pseudo_start(st: &mut EngineState) -> Result<(), Abort> {
    scan_general_text(st)?;
    let old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    crate::output::token_show(st, TEMP_HEAD);
    st.shipout.selector = old_setting;
    let head = TEMP_HEAD.link(&st.mem);
    st.mem.flush_list(head);

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(Abort::PoolOverflow);
    }
    let s = st.strings.make_string()?;
    st.strings.str_pool[st.strings.pool_ptr as usize] = u16::from(b' ');
    let mut l = st.strings.str_start[s.slot()];
    let nl = st.eqtb.int_par(IntPar::NewLineChar) as u16;
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
        let mut chars = [u16::from(b' '); 4];
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

    if st.eqtb.int_par(IntPar::TracingScanTokens) > 0 {
        if st.shipout.term_offset > st.config.max_print_line - 3 {
            crate::output::print_ln(st);
        } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
            crate::output::print_char(st, b' ');
        }
        st.input.cur_input.name = 19;
        print(st, "( ");
        st.input.open_parens += 1;
        output_flush(st.shipout.rust_stdout);
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
        st.scanner.cur_tok = Token::from_cs(st.scanner.cur_cs);
    } else {
        st.scanner.cur_cmd = RELAX;
        st.scanner.cur_chr = 0;
        st.scanner.cur_tok = Token::from_cs(FROZEN_RELAX);
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
            if !st.scanner.cur_tok.is_cs() {
                st.scanner.cur_val = st.scanner.cur_chr;
                if st.scanner.cur_cmd == RIGHT_BRACE {
                    st.scanner.align_state += 1;
                } else if st.scanner.cur_cmd < RIGHT_BRACE {
                    st.scanner.align_state -= 1;
                }
            } else if st.scanner.cur_tok < Token::from_cs(SINGLE_BASE) {
                st.scanner.cur_val = st.scanner.cur_tok.cs() - ACTIVE_BASE;
            } else {
                st.scanner.cur_val = st.scanner.cur_tok.cs() - SINGLE_BASE;
            }
            if st.scanner.cur_val > BIGGEST_USV {
                print_err(st, "Improper alphabetic constant");
                st.errors.help(&[
                    "A one-character control sequence belongs after a ` mark.",
                    r"So I'm essentially inserting \0 here.",
                ]);
                st.scanner.cur_val = i32::from(b'0');
                back_error(st)?;
            } else {
                scan_optional_space(st)?;
            }
        } else if st.scanner.cur_tok == FROZEN_PRIM_TOKEN {
            get_primitive_token(st)?;
            continue;
        } else if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
            scan_something_internal(st, ValueLevel::Int, false)?;
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
                let d = if let Some(d) = tok.other_digit().filter(|&d| d < radix) {
                    d
                } else if radix == 16 && (A_TOKEN..=Token::letter(b'F')).contains(&tok) {
                    tok.0 - A_TOKEN.0 + 10
                } else if radix == 16 && (OTHER_A_TOKEN..=Token::other(b'F')).contains(&tok) {
                    tok.0 - OTHER_A_TOKEN.0 + 10
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
    let spec = glue_spec(st.scanner.cur_val);
    st.scanner.set_cur_dimen(spec.width(&st.mem));
    st.mem.delete_glue_ref(spec);
}

/// Scales `cur_val` and the fraction `f` by `num / denom`.
fn scale_dimen(st: &mut EngineState, f: &mut i32, num: i32, denom: i32) {
    st.scanner.cur_val = st.errors.xn_over_d(st.scanner.cur_val, num, denom);
    *f = (num * *f + 65536 * st.errors.tex_remainder) / denom;
    st.scanner.cur_val += *f / 65536;
    *f %= 65536;
}

/// Sets `cur_val` to `integer.fraction` times the unit `v`.
fn attach_unit(st: &mut EngineState, integer: i32, fraction: i32, v: Scaled) {
    let fraction_part = st.errors.xn_over_d(v, fraction, 65536);
    let d = st.errors.mult_and_add(integer, v, fraction_part, MAX_DIMEN);
    st.scanner.set_cur_dimen(d);
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
                    scan_something_internal(st, ValueLevel::Mu, false)?;
                    if st.scanner.cur_val_level >= ValueLevel::Glue {
                        glue_to_width(st);
                    }
                    if st.scanner.cur_val_level == ValueLevel::Mu {
                        break 'attach_sign;
                    }
                    if st.scanner.cur_val_level != ValueLevel::Int {
                        mu_error(st)?;
                    }
                } else {
                    scan_something_internal(st, ValueLevel::Dimen, false)?;
                    if st.scanner.cur_val_level == ValueLevel::Dimen {
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
                        let Some(d) = st.scanner.cur_tok.other_digit() else {
                            break;
                        };
                        if k < 17 {
                            let q = st.mem.get_avail()?;
                            q.set_link(&mut st.mem, digits);
                            q.set_info(&mut st.mem, d);
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
                        scan_something_internal(st, ValueLevel::Mu, false)?;
                        if st.scanner.cur_val_level >= ValueLevel::Glue {
                            glue_to_width(st);
                        }
                        if st.scanner.cur_val_level != ValueLevel::Mu {
                            mu_error(st)?;
                        }
                    } else {
                        scan_something_internal(st, ValueLevel::Dimen, false)?;
                    }
                    attach_unit(st, save_cur_val, f, st.scanner.cur_dimen());
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
                    let mag = st.eqtb.int_par(IntPar::Mag);
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

pub(crate) fn scan_glue(st: &mut EngineState, level: ValueLevel) -> Result<(), Abort> {
    let mu = level == ValueLevel::Mu;
    let negative = scan_signs(st)?;

    if st.scanner.cur_cmd >= MIN_INTERNAL && st.scanner.cur_cmd <= MAX_INTERNAL {
        scan_something_internal(st, level, negative)?;
        if st.scanner.cur_val_level >= ValueLevel::Glue {
            if st.scanner.cur_val_level != level {
                mu_error(st)?;
            }
            return Ok(());
        }
        if st.scanner.cur_val_level == ValueLevel::Int {
            scan_dimen(st, mu, false, true)?;
        } else if level == ValueLevel::Mu {
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
    gs.set_width(&mut st.mem, st.scanner.cur_dimen());

    if scan_keyword(st, "plus")? {
        scan_dimen(st, mu, true, false)?;
        gs.set_stretch(&mut st.mem, st.scanner.cur_dimen());
        gs.set_stretch_order(&mut st.mem, st.scanner.cur_order);
    }
    if scan_keyword(st, "minus")? {
        scan_dimen(st, mu, true, false)?;
        gs.set_shrink(&mut st.mem, st.scanner.cur_dimen());
        gs.set_shrink_order(&mut st.mem, st.scanner.cur_order);
    }

    st.scanner.cur_val = gs.ptr().0;
    Ok(())
}
pub(crate) fn scan_normal_glue(st: &mut EngineState) -> Result<(), Abort> {
    scan_glue(st, ValueLevel::Glue)
}
pub(crate) fn scan_mu_glue(st: &mut EngineState) -> Result<(), Abort> {
    scan_glue(st, ValueLevel::Mu)
}
/// Applies `op` to the width, stretch and shrink of `spec`, in that order.
fn map_glue(
    st: &mut EngineState,
    spec: NodePtr<GlueSpecNode>,
    mut op: impl FnMut(&mut crate::state::ErrorState, Scaled) -> Scaled,
) {
    let width = op(&mut st.errors, spec.width(&st.mem));
    spec.set_width(&mut st.mem, width);
    let stretch = op(&mut st.errors, spec.stretch(&st.mem));
    spec.set_stretch(&mut st.mem, stretch);
    let shrink = op(&mut st.errors, spec.shrink(&st.mem));
    spec.set_shrink(&mut st.mem, shrink);
}

fn normalize_glue_orders(st: &mut EngineState, spec: NodePtr<GlueSpecNode>) {
    if spec.stretch(&st.mem) == Scaled::ZERO {
        spec.set_stretch_order(&mut st.mem, NORMAL_EXPR);
    }
    if spec.shrink(&st.mem) == Scaled::ZERO {
        spec.set_shrink_order(&mut st.mem, NORMAL_EXPR);
    }
}

/// Adds (or subtracts) glue `t` to glue `e` and releases `t`.
fn add_glue(st: &mut EngineState, e: NodePtr<GlueSpecNode>, t: NodePtr<GlueSpecNode>, sub: bool) {
    let mem = &mut st.mem;
    let errors = &mut st.errors;
    let width = errors.add_or_sub(e.width(mem), t.width(mem), MAX_DIMEN, sub);
    e.set_width(mem, width);
    let (e_order, t_order, t_stretch) =
        (e.stretch_order(mem), t.stretch_order(mem), t.stretch(mem));
    if e_order == t_order {
        let stretch = errors.add_or_sub(e.stretch(mem), t_stretch, MAX_DIMEN, sub);
        e.set_stretch(mem, stretch);
    } else if e_order < t_order && t_stretch != Scaled::ZERO {
        e.set_stretch(mem, t_stretch);
        e.set_stretch_order(mem, t_order);
    }
    let (e_order, t_order, t_shrink) = (e.shrink_order(mem), t.shrink_order(mem), t.shrink(mem));
    if e_order == t_order {
        let shrink = errors.add_or_sub(e.shrink(mem), t_shrink, MAX_DIMEN, sub);
        e.set_shrink(mem, shrink);
    } else if e_order < t_order && t_shrink != Scaled::ZERO {
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
                let o = if s == EXPR_NONE { l } else { ValueLevel::Int };
                get_x_non_blank(st)?;
                if st.scanner.cur_tok == Token::other(b'(') {
                    let q = NodePtr::<ExprNode>::alloc(&mut st.mem)?;
                    q.set_link(&mut st.mem, p);
                    q.set_level(&mut st.mem, l);
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
                    ValueLevel::Int => scan_int(st)?,
                    ValueLevel::Dimen => scan_dimen(st, false, false, false)?,
                    ValueLevel::Glue => scan_normal_glue(st)?,
                    _ => scan_mu_glue(st)?,
                }
                f = st.scanner.cur_val;
            }
            scan_factor = true;

            get_x_non_blank(st)?;
            let tok = st.scanner.cur_tok;
            let mut o = if tok == Token::other(b'+') {
                EXPR_ADD
            } else if tok == Token::other(b'-') {
                EXPR_SUB
            } else if tok == Token::other(b'*') {
                EXPR_MULT
            } else if tok == Token::other(b'/') {
                EXPR_DIV
            } else {
                EXPR_NONE
            };
            if o == EXPR_NONE {
                if p.is_null() {
                    if st.scanner.cur_cmd != RELAX {
                        back_input(st)?;
                    }
                } else if st.scanner.cur_tok != Token::other(b')') {
                    print_err(st, "Missing ) inserted for expression");
                    st.errors
                        .help(&["I was expecting to see `+', `-', `*', `/', or `)'. Didn't."]);
                    back_error(st)?;
                }
            }

            st.errors.arith_error = b;
            if l == ValueLevel::Int || s > EXPR_SUB {
                if !(-TEX_INFINITY..=TEX_INFINITY).contains(&f) {
                    st.errors.arith_error = true;
                    f = 0;
                }
            } else if l == ValueLevel::Dimen {
                if f.abs() > MAX_HALFWORD {
                    st.errors.arith_error = true;
                    f = 0;
                }
            } else {
                let spec = glue_spec(f);
                if spec.width(&st.mem).abs() > MAX_DIMEN
                    || spec.stretch(&st.mem).abs() > MAX_DIMEN
                    || spec.shrink(&st.mem).abs() > MAX_DIMEN
                {
                    st.errors.arith_error = true;
                    st.mem.delete_glue_ref(glue_spec(f));
                    f = st.mem.new_spec(glue_spec(0))?.ptr().0;
                }
            }

            match s {
                EXPR_NONE => {
                    if l >= ValueLevel::Glue && o != EXPR_NONE {
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
                    } else if l == ValueLevel::Int {
                        t = st.errors.mult_and_add(t, f, 0, TEX_INFINITY);
                    } else if l == ValueLevel::Dimen {
                        t = st.errors.mult_and_add(t, f, 0, MAX_HALFWORD);
                    } else {
                        // TeX's `nx_plus_y` here takes the component as its integer factor.
                        map_glue(st, glue_spec(t), |errors, x| {
                            Scaled(errors.mult_and_add(x.0, f, 0, MAX_HALFWORD))
                        });
                    }
                }
                EXPR_DIV => {
                    if l < ValueLevel::Glue {
                        t = st.errors.quotient(t, f);
                    } else {
                        map_glue(st, glue_spec(t), |errors, x| {
                            Scaled(errors.quotient(x.0, f))
                        });
                    }
                }
                EXPR_SCALE => {
                    if l == ValueLevel::Int {
                        t = st.errors.fract(t, n, f, TEX_INFINITY);
                    } else if l == ValueLevel::Dimen {
                        t = st.errors.fract(t, n, f, MAX_HALFWORD);
                    } else {
                        map_glue(st, glue_spec(t), |errors, x| {
                            errors.fract(x, n, f, MAX_DIMEN)
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
                } else if l == ValueLevel::Int {
                    e = st.errors.add_or_sub(e, t, TEX_INFINITY, r == EXPR_SUB);
                } else if l == ValueLevel::Dimen {
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
            l = q.level(&st.mem);
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
        if l >= ValueLevel::Glue {
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
    st.scanner.cur_val_level = l;
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
        let ch = i32::from(keyword[i]);
        if st.scanner.cur_cs == 0 && (st.scanner.cur_chr == ch || st.scanner.cur_chr == ch - 32) {
            store_new_token(st, &mut p, st.scanner.cur_tok)?;
            i += 1;
        } else if st.scanner.cur_cmd != SPACER || p != BACKUP_HEAD {
            back_input(st)?;
            if p != BACKUP_HEAD {
                begin_token_list(st, BACKUP_HEAD.link(&st.mem), TokenType::BackedUp)?;
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
        st.scanner.cur_tok = Token::left_brace(b'{');
        st.scanner.cur_cmd = LEFT_BRACE;
        st.scanner.cur_chr = i32::from(b'{');
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
fn set_cur_val(st: &mut EngineState, value: i32, level: ValueLevel) {
    st.scanner.cur_val = value;
    st.scanner.cur_val_level = level;
}

/// The sparse-array element of type `t` for register `n`, if it exists.
fn find_sa(st: &mut EngineState, t: ValueLevel, n: i32) -> Option<NodePtr<SaNode>> {
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

/// Scans a font identifier, returning the font, its OpenType/Graphite layout
/// engine and its area flag.
fn scan_font_engine(
    st: &mut EngineState,
) -> Result<(FontId, Option<Arc<FontEngineHandle>>, StrNumber), Abort> {
    let f = scan_font_ident(st)?;
    Ok((f, st.fonts.layout_engine_arc(f), st.fonts.font_area[f]))
}

fn graphite_engine(engine: &Option<Arc<FontEngineHandle>>) -> Option<&FontEngineHandle> {
    engine.as_deref().filter(|e| e.using_graphite())
}

fn opentype_engine(engine: &Option<Arc<FontEngineHandle>>) -> Option<&FontEngineHandle> {
    engine.as_deref().filter(|e| e.using_opentype())
}

/// Reports a font query on an unsuitable font and yields `fallback`.
fn font_query_error(
    st: &mut EngineState,
    m: LastItemCode,
    f: FontId,
    msg: &str,
    fallback: i32,
) -> Result<(), Abort> {
    crate::errors::not_font_error_common(st, LAST_ITEM, m.raw(), f, msg)?;
    st.scanner.cur_val = fallback;
    Ok(())
}

fn fetch_def_code(st: &mut EngineState, m: i32) -> Result<(), Abort> {
    scan_usv_num(st)?;
    let n = st.scanner.cur_val;
    if m == MATH_CODE_BASE {
        let code = st.eqtb.math_code(n);
        st.scanner.cur_val1 = code;
        if math_char(code) == ACTIVE_MATH_CHAR {
            st.scanner.cur_val1 = 0x8000;
        } else if math_class(code) > 7 || math_fam(code) > 15 || math_char(code) > 255 {
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
        let v = math_class(code) * 0x1000 + math_fam(code) * 0x100 + math_char(code);
        set_cur_val(st, v, ValueLevel::Int);
    } else if m == DEL_CODE_BASE {
        st.scanner.cur_val1 = st.eqtb.del_code(n);
        if st.scanner.cur_val1 >= 0x4000_0000 {
            print_err(st, "Extended delcode used as delcode");
            st.errors.help(&[
                "A delimiter code must be between 0 and \"7FFFFFF.",
                "I changed this one to zero.",
            ]);
            error(st)?;
            set_cur_val(st, 0, ValueLevel::Int);
        } else {
            set_cur_val(st, st.scanner.cur_val1, ValueLevel::Int);
        }
    } else if m < SF_CODE_BASE {
        set_cur_val(st, st.eqtb.equiv(m + n), ValueLevel::Int);
    } else if m < MATH_CODE_BASE {
        set_cur_val(st, st.eqtb.equiv(m + n) % 65536, ValueLevel::Int);
    } else {
        set_cur_val(st, st.eqtb.equiv(m + n), ValueLevel::Int);
    }
    Ok(())
}

fn fetch_xetex_def_code(st: &mut EngineState, m: i32) -> Result<(), Abort> {
    scan_usv_num(st)?;
    let n = st.scanner.cur_val;
    let (err, help1, help0) = if m == SF_CODE_BASE {
        set_cur_val(st, st.eqtb.sf_code(n) / 65536, ValueLevel::Int);
        return Ok(());
    } else if m == MATH_CODE_BASE {
        set_cur_val(st, st.eqtb.math_code(n), ValueLevel::Int);
        return Ok(());
    } else if m == MATH_CODE_BASE + 1 {
        (
            "Can't use \\Umathcode as a number (try \\Umathcodenum)",
            r"\Umathcode is for setting a mathcode from separate values;",
            r"use \Umathcodenum to access them as single values.",
        )
    } else if m == DEL_CODE_BASE {
        set_cur_val(st, st.eqtb.del_code(n), ValueLevel::Int);
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
    set_cur_val(st, 0, ValueLevel::Int);
    Ok(())
}

fn fetch_token_list(st: &mut EngineState, m: i32, level: ValueLevel) -> Result<(), Abort> {
    if level != ValueLevel::Tok {
        missing_number_error(st)?;
        set_cur_val(st, 0, ValueLevel::Dimen);
    } else if st.scanner.cur_cmd < ASSIGN_TOKS {
        if m == 0 {
            scan_register_num(st)?;
            let n = st.scanner.cur_val;
            st.scanner.cur_val = if n < 256 {
                st.eqtb.toks(n)
            } else {
                let sa = find_sa(st, ValueLevel::Tok, n);
                sa_token_list(st, sa)
            }
            .0;
        } else {
            st.scanner.cur_val = NodePtr::<SaNode>::at(m).sa_ptr(&st.mem).0;
        }
        st.scanner.cur_val_level = ValueLevel::Tok;
    } else if st.scanner.cur_cmd == ASSIGN_TOKS {
        if st.scanner.cur_chr == TokPar::XetexInterCharToks.loc() {
            scan_char_class_not_ignored(st)?;
            st.scanner.cur_ptr.0 = st.scanner.cur_val;
            scan_char_class_not_ignored(st)?;
            let n = st.scanner.cur_ptr.0 * CHAR_CLASS_LIMIT + st.scanner.cur_val;
            let sa = find_sa(st, ValueLevel::InterChar, n);
            st.scanner.cur_val = sa_token_list(st, sa).0;
        } else {
            st.scanner.cur_val = st.eqtb.equiv(m);
        }
        st.scanner.cur_val_level = ValueLevel::Tok;
    } else {
        back_input(st)?;
        let f = scan_font_ident(st)?;
        set_cur_val(st, f.id_loc(), ValueLevel::Ident);
    }
    Ok(())
}

fn fetch_register(st: &mut EngineState, m: i32) -> Result<(), Abort> {
    let Some(level) = ValueLevel::from_raw(m) else {
        let sa = NodePtr::<SaNode>::at(m);
        let level = sa.sa_type(&st.mem);
        let value = if level < ValueLevel::Glue {
            sa.sa_int(&st.mem)
        } else {
            sa.sa_ptr(&st.mem).0
        };
        set_cur_val(st, value, level);
        return Ok(());
    };
    scan_register_num(st)?;
    let n = st.scanner.cur_val;
    st.scanner.cur_val_level = level;
    if n > 255 {
        st.scanner.cur_val = match find_sa(st, level, n) {
            None => 0,
            Some(sa) if level < ValueLevel::Glue => sa.sa_int(&st.mem),
            Some(sa) => sa.sa_ptr(&st.mem).0,
        };
    } else {
        let base = match level {
            ValueLevel::Int => COUNT_BASE,
            ValueLevel::Dimen => SCALED_BASE,
            ValueLevel::Glue => SKIP_BASE,
            ValueLevel::Mu => MU_SKIP_BASE,
            _ => return Ok(()),
        };
        st.scanner.cur_val = st.eqtb.equiv(base + n);
    }
    Ok(())
}

/// Integer-valued `\lastnodetype`-style and XeTeX font queries.
fn fetch_last_item_int(st: &mut EngineState, m: LastItemCode) -> Result<(), Abort> {
    match m {
        LastItemCode::InputLineNo => st.scanner.cur_val = st.mem.line,
        LastItemCode::Badness => st.scanner.cur_val = st.shipout.last_badness,
        LastItemCode::ElapsedTime => {
            st.scanner.cur_val = if st.config.deterministic_mode {
                0
            } else {
                st.shipout.get_microinterval()
            };
        }
        LastItemCode::RandomSeed => st.scanner.cur_val = st.errors.random_seed,
        LastItemCode::PdfShellEscape => {
            st.scanner.cur_val = i32::from(st.config.shell_escape_enabled)
        }
        LastItemCode::EtexVersion => st.scanner.cur_val = ETEX_VERSION,
        LastItemCode::XetexVersion => st.scanner.cur_val = XETEX_VERSION,
        LastItemCode::XetexCountGlyphs => {
            let (_, engine, fa) = scan_font_engine(st)?;
            st.scanner.cur_val = if fa == AAT_FONT_FLAG {
                aat_font_get(m)
            } else if let Some(engine) = engine.as_deref() {
                ot_font_get(m, engine)
            } else {
                0
            };
        }
        LastItemCode::XetexCountFeatures => {
            let (_, engine, fa) = scan_font_engine(st)?;
            st.scanner.cur_val = if fa == AAT_FONT_FLAG {
                aat_font_get(m)
            } else if let Some(engine) = graphite_engine(&engine) {
                ot_font_get(m, engine)
            } else {
                0
            };
        }
        LastItemCode::XetexVariation
        | LastItemCode::XetexVariationMin
        | LastItemCode::XetexVariationMax
        | LastItemCode::XetexVariationDefault
        | LastItemCode::XetexCountVariations => {
            scan_font_ident(st)?;
            st.scanner.cur_val = 0;
        }
        LastItemCode::XetexFeatureCode
        | LastItemCode::XetexIsExclusiveFeature
        | LastItemCode::XetexCountSelectors => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                scan_int(st)?;
                st.scanner.cur_val = aat_font_get_1(m, st.scanner.cur_val);
            } else if let Some(engine) = graphite_engine(&engine) {
                scan_int(st)?;
                st.scanner.cur_val = ot_font_get_1(m, engine, st.scanner.cur_val);
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font", -1)?;
            }
        }
        LastItemCode::XetexSelectorCode | LastItemCode::XetexIsDefaultSelector => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                scan_int(st)?;
                st.scanner.cur_val = aat_font_get_2(m, k, st.scanner.cur_val);
            } else if let Some(engine) = graphite_engine(&engine) {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                scan_int(st)?;
                st.scanner.cur_val = ot_font_get_2(m, engine, k, st.scanner.cur_val);
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font", -1)?;
            }
        }
        LastItemCode::XetexFindVariationByName => {
            let (f, _, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                crate::io::scan_and_pack_name(st)?;
                st.scanner.cur_val = aat_font_get_named(m);
            } else {
                font_query_error(st, m, f, "; not an AAT font", -1)?;
            }
        }
        LastItemCode::XetexFindFeatureByName => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                crate::io::scan_and_pack_name(st)?;
                st.scanner.cur_val = aat_font_get_named(m);
            } else if let Some(engine) = graphite_engine(&engine) {
                crate::io::scan_and_pack_name(st)?;
                st.scanner.cur_val = gr_font_get_named(st, m, engine);
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font", -1)?;
            }
        }
        LastItemCode::XetexFindSelectorByName => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                crate::io::scan_and_pack_name(st)?;
                st.scanner.cur_val = aat_font_get_named_1(m, k);
            } else if let Some(engine) = graphite_engine(&engine) {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                crate::io::scan_and_pack_name(st)?;
                st.scanner.cur_val = gr_font_get_named_1(st, m, engine, k);
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font", -1)?;
            }
        }
        LastItemCode::XetexOtCountScripts => {
            let (_, engine, _) = scan_font_engine(st)?;
            st.scanner.cur_val = if let Some(engine) = opentype_engine(&engine) {
                ot_font_get(m, engine)
            } else {
                0
            };
        }
        LastItemCode::XetexOtCountLanguages | LastItemCode::XetexOtScript => {
            let (f, engine, _) = scan_font_engine(st)?;
            if let Some(engine) = opentype_engine(&engine) {
                scan_int(st)?;
                st.scanner.cur_val = ot_font_get_1(m, engine, st.scanner.cur_val);
            } else {
                font_query_error(st, m, f, "; not an OpenType Layout font", -1)?;
            }
        }
        LastItemCode::XetexOtCountFeatures | LastItemCode::XetexOtLanguage => {
            let (f, engine, _) = scan_font_engine(st)?;
            if let Some(engine) = opentype_engine(&engine) {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                scan_int(st)?;
                st.scanner.cur_val = ot_font_get_2(m, engine, k, st.scanner.cur_val);
            } else {
                font_query_error(st, m, f, "; not an OpenType Layout font", -1)?;
            }
        }
        LastItemCode::XetexOtFeature => {
            let (f, engine, _) = scan_font_engine(st)?;
            if let Some(engine) = opentype_engine(&engine) {
                scan_int(st)?;
                let k = st.scanner.cur_val;
                scan_int(st)?;
                let kk = st.scanner.cur_val;
                scan_int(st)?;
                st.scanner.cur_val = ot_font_get_3(m, engine, k, kk, st.scanner.cur_val);
            } else {
                font_query_error(st, m, f, "; not an OpenType Layout font", -1)?;
            }
        }
        LastItemCode::XetexMapCharToGlyph => {
            let f = st.eqtb.cur_font();
            if st.fonts.is_native_font(f) {
                scan_int(st)?;
                st.scanner.cur_val = st.fonts.map_char_to_glyph(f, st.scanner.cur_val)?;
            } else {
                font_query_error(st, m, f, "; not a native platform font", 0)?;
            }
        }
        LastItemCode::XetexGlyphIndex => {
            let f = st.eqtb.cur_font();
            if st.fonts.is_native_font(f) {
                crate::io::scan_and_pack_name(st)?;
                let name = st.input.name_of_file_bytes();
                st.scanner.cur_val = st.fonts.map_glyph_to_index(f, name)?;
            } else {
                font_query_error(st, m, f, "; not a native platform font", 0)?;
            }
        }
        LastItemCode::XetexFontType => {
            let (_, engine, fa) = scan_font_engine(st)?;
            st.scanner.cur_val = if fa == AAT_FONT_FLAG {
                1
            } else if opentype_engine(&engine).is_some() {
                2
            } else if graphite_engine(&engine).is_some() {
                3
            } else {
                0
            };
        }
        LastItemCode::XetexFirstChar | LastItemCode::XetexLastChar => {
            let f = scan_font_ident(st)?;
            let first = m == LastItemCode::XetexFirstChar;
            st.scanner.cur_val = if st.fonts.is_native_font(f) {
                st.fonts.get_font_char_range(f, i32::from(first))?
            } else if first {
                i32::from(st.fonts.font_bc[f])
            } else {
                i32::from(st.fonts.font_ec[f])
            };
        }
        LastItemCode::PdfLastXPos => st.scanner.cur_val = st.shipout.pdf_last_x_pos,
        LastItemCode::PdfLastYPos => st.scanner.cur_val = st.shipout.pdf_last_y_pos,
        LastItemCode::XetexPdfPageCount => {
            crate::io::scan_and_pack_name(st)?;
            st.scanner.cur_val =
                crate::pic::count_pdf_file_pages_named(st.input.name_of_file_bytes())?;
        }
        LastItemCode::CurrentGroupLevel => {
            st.scanner.cur_val = i32::from(st.save_stack.cur_level) - 1
        }
        LastItemCode::CurrentGroupType => st.scanner.cur_val = st.save_stack.cur_group as i32,
        LastItemCode::CurrentIfLevel => {
            let mut q = st.scanner.cond_ptr;
            let mut depth = 0;
            while !q.is_null() {
                depth += 1;
                q = q.link(&st.mem);
            }
            st.scanner.cur_val = depth;
        }
        LastItemCode::CurrentIfType => {
            st.scanner.cur_val = if st.scanner.cond_ptr.is_null() {
                0
            } else if i32::from(st.scanner.cur_if) < IfCode::UNLESS {
                i32::from(st.scanner.cur_if) + 1
            } else {
                -(i32::from(st.scanner.cur_if) - 31)
            };
        }
        LastItemCode::CurrentIfBranch => {
            st.scanner.cur_val = match st.scanner.if_limit {
                FiOrElseCode::Or | FiOrElseCode::Else => 1,
                FiOrElseCode::Fi => -1,
                _ => 0,
            };
        }
        LastItemCode::GlueStretchOrder | LastItemCode::GlueShrinkOrder => {
            scan_normal_glue(st)?;
            let q = st.scanner.cur_val;
            let spec = glue_spec(q);
            st.scanner.cur_val = if m == LastItemCode::GlueStretchOrder {
                i32::from(spec.stretch_order(&st.mem))
            } else {
                i32::from(spec.shrink_order(&st.mem))
            };
            st.mem.delete_glue_ref(glue_spec(q));
        }
        _ => {}
    }
    st.scanner.cur_val_level = ValueLevel::Int;
    Ok(())
}

fn fetch_last_item_dimen(st: &mut EngineState, m: LastItemCode) -> Result<(), Abort> {
    match m {
        LastItemCode::XetexGlyphBounds => {
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
                    capture_to_diagnostic(st, None);
                    error(st)?;
                    st.scanner.cur_val = 0;
                } else {
                    scan_int(st)?;
                    let bound = st.fonts.get_glyph_bounds(f, edge, st.scanner.cur_val)?;
                    st.scanner.set_cur_dimen(bound);
                }
            } else {
                font_query_error(st, m, f, "; not a native platform font", 0)?;
            }
        }
        LastItemCode::FontCharWd
        | LastItemCode::FontCharHt
        | LastItemCode::FontCharDp
        | LastItemCode::FontCharIc => {
            let f = scan_font_ident(st)?;
            scan_usv_num(st)?;
            let c = st.scanner.cur_val;
            let d = if st.fonts.is_native_font(f) {
                let fonts = &st.fonts;
                match m {
                    LastItemCode::FontCharWd => fonts.native_char_width(f, c),
                    LastItemCode::FontCharHt => fonts.native_char_height(f, c),
                    LastItemCode::FontCharDp => fonts.native_char_depth(f, c),
                    _ => fonts.native_char_italic_correction(f, c),
                }?
            } else if i32::from(st.fonts.font_bc[f]) <= c && i32::from(st.fonts.font_ec[f]) >= c {
                let ec = st.fonts.effective_char(f, c as u16);
                let info = st.fonts.char_info(f, ec);
                let fonts = &st.fonts;
                match m {
                    LastItemCode::FontCharWd => fonts.char_width(f, info),
                    LastItemCode::FontCharHt => fonts.char_height(f, info),
                    LastItemCode::FontCharDp => fonts.char_depth(f, info),
                    _ => fonts.char_italic(f, info),
                }
            } else {
                Scaled::ZERO
            };
            st.scanner.set_cur_dimen(d);
        }
        LastItemCode::ParShapeLength
        | LastItemCode::ParShapeIndent
        | LastItemCode::ParShapeDimen => {
            let mut q = match m {
                LastItemCode::ParShapeLength => 0,
                LastItemCode::ParShapeIndent => 1,
                _ => 2,
            };
            scan_int(st)?;
            match st.eqtb.par_shape() {
                Some(shape) if st.scanner.cur_val > 0 => {
                    if q == 2 {
                        q = st.scanner.cur_val % 2;
                        st.scanner.cur_val = (st.scanner.cur_val + q) / 2;
                    }
                    let line = st.scanner.cur_val.min(shape.lines(&st.mem));
                    let d = if q == 0 {
                        shape.line_width(&st.mem, line)
                    } else {
                        shape.indent(&st.mem, line)
                    };
                    st.scanner.set_cur_dimen(d);
                }
                _ => st.scanner.cur_val = 0,
            }
        }
        LastItemCode::GlueStretch | LastItemCode::GlueShrink => {
            scan_normal_glue(st)?;
            let spec = glue_spec(st.scanner.cur_val);
            let d = if m == LastItemCode::GlueStretch {
                spec.stretch(&st.mem)
            } else {
                spec.shrink(&st.mem)
            };
            st.scanner.set_cur_dimen(d);
            st.mem.delete_glue_ref(spec);
        }
        _ => {}
    }
    st.scanner.cur_val_level = ValueLevel::Dimen;
    Ok(())
}

/// `\lastpenalty`, `\lastkern`, `\lastskip` and `\lastnodetype`.
fn fetch_last_node_item(st: &mut EngineState, code: LastItemCode) {
    let want_type = code == LastItemCode::LastNodeType;
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
        st.scanner.cur_val_level = ValueLevel::Int;
        if tx == head || st.input.cur_list.mode == 0 {
            st.scanner.cur_val = -1;
        }
    } else {
        st.scanner.cur_val_level = match code {
            LastItemCode::LastPenalty => ValueLevel::Int,
            LastItemCode::LastKern => ValueLevel::Dimen,
            _ => ValueLevel::Glue,
        };
    }
    match node {
        Some(_) if st.input.cur_list.mode != 0 && want_type => {
            let node_type = st.mem.type_code(MemIdx::new(tx));
            st.scanner.cur_val = if node_type <= NodeType::Unset.as_raw() {
                i32::from(node_type) + 1
            } else {
                i32::from(NodeType::Unset.as_raw()) + 2
            };
        }
        Some(node) if st.input.cur_list.mode != 0 => match (code, node) {
            (LastItemCode::LastPenalty, Ok(AnyNode::Penalty(p))) => {
                st.scanner.cur_val = p.penalty(&st.mem)
            }
            (LastItemCode::LastKern, Ok(AnyNode::Kern(k))) => {
                st.scanner.set_cur_dimen(k.width(&st.mem))
            }
            (LastItemCode::LastSkip, Ok(AnyNode::Glue(g))) => {
                st.scanner.cur_val = g.glue_ptr(&st.mem).ptr().0;
                if g.subtype(&st.mem) == MU_GLUE {
                    st.scanner.cur_val_level = ValueLevel::Mu;
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
                match code {
                    LastItemCode::LastPenalty => st.scanner.cur_val = st.shipout.last_penalty,
                    LastItemCode::LastKern => st.scanner.set_cur_dimen(st.shipout.last_kern),
                    LastItemCode::LastSkip => {
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
    level: ValueLevel,
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
            ASSIGN_INT => set_cur_val(st, st.eqtb.int_at(m), ValueLevel::Int),
            ASSIGN_DIMEN => set_cur_val(st, st.eqtb.int_at(m), ValueLevel::Dimen),
            ASSIGN_GLUE => set_cur_val(st, st.eqtb.equiv(m), ValueLevel::Glue),
            ASSIGN_MU_GLUE => set_cur_val(st, st.eqtb.equiv(m), ValueLevel::Mu),
            SET_AUX => {
                if i32::from(st.input.cur_list.mode.abs()) != m {
                    error_here_with_diagnostic(st, "Improper ");
                    print_cmd_chr(st, SET_AUX, m);
                    capture_to_diagnostic(st, None);
                    st.errors.help(&[
                        r"You can refer to \spacefactor only in horizontal mode;",
                        r"you can refer to \prevdepth only in vertical mode; and",
                        r"neither of these is meaningful inside \write. So",
                        "I'm forgetting what you said and using zero instead.",
                    ]);
                    error(st)?;
                    set_cur_val(
                        st,
                        0,
                        if level != ValueLevel::Tok {
                            ValueLevel::Dimen
                        } else {
                            ValueLevel::Int
                        },
                    );
                } else if m == i32::from(VMODE) {
                    set_cur_val(st, st.input.cur_list.aux_s1(), ValueLevel::Dimen);
                } else {
                    set_cur_val(st, st.input.cur_list.aux_s0(), ValueLevel::Int);
                }
            }
            SET_PREV_GRAF => {
                if st.input.cur_list.mode == 0 {
                    set_cur_val(st, 0, ValueLevel::Int);
                } else {
                    let np = st.input.nest_ptr as usize;
                    st.input.nest[np] = st.input.cur_list;
                    let mut p = np;
                    while st.input.nest[p].mode.abs() != VMODE {
                        p -= 1;
                    }
                    set_cur_val(st, st.input.nest[p].prev_graf, ValueLevel::Int);
                }
            }
            SET_PAGE_INT => {
                let v = match PageIntCode::from_raw(m) {
                    Some(PageIntCode::DeadCycles) => st.shipout.dead_cycles,
                    Some(PageIntCode::InteractionMode) => st.errors.interaction as i32,
                    _ => st.shipout.insert_penalties,
                };
                set_cur_val(st, v, ValueLevel::Int);
            }
            SET_PAGE_DIMEN => {
                let v = if st.shipout.page_contents == crate::state::PageContents::Empty
                    && !st.shipout.output_active
                {
                    if m == 0 {
                        MAX_DIMEN
                    } else {
                        Scaled::ZERO
                    }
                } else {
                    st.shipout.page_so_far[m as usize]
                };
                set_cur_val(st, v.0, ValueLevel::Dimen);
            }
            SET_SHAPE => {
                if m > PAR_SHAPE_LOC {
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
                st.scanner.cur_val_level = ValueLevel::Int;
            }
            SET_BOX_DIMEN => {
                scan_register_num(st)?;
                let v = match fetch_box(st, st.scanner.cur_val) {
                    None => Scaled::ZERO,
                    Some(b) if m == BoxDimenCode::Width.raw() => b.width(&st.mem),
                    Some(b) if m == BoxDimenCode::Depth.raw() => b.depth(&st.mem),
                    Some(b) => b.height(&st.mem),
                };
                set_cur_val(st, v.0, ValueLevel::Dimen);
            }
            CHAR_GIVEN | MATH_GIVEN | XETEX_MATH_GIVEN => set_cur_val(st, m, ValueLevel::Int),
            ASSIGN_FONT_DIMEN => {
                find_font_dimen(st, false)?;
                st.fonts.set_font_info_sc(st.fonts.fmem_ptr, Scaled::ZERO);
                set_cur_val(
                    st,
                    st.fonts.font_info_sc(st.scanner.cur_val).0,
                    ValueLevel::Dimen,
                );
            }
            ASSIGN_FONT_INT => {
                let f = scan_font_ident(st)?;
                match m {
                    0 => set_cur_val(st, st.fonts.hyphen_char[f], ValueLevel::Int),
                    1 => set_cur_val(st, st.fonts.skew_char[f], ValueLevel::Int),
                    _ => {
                        if st.fonts.is_native_font(f) {
                            scan_glyph_number(st, f)?;
                        } else {
                            scan_char_num(st)?;
                        }
                        let code = st.scanner.cur_val as u32;
                        match m {
                            LP_CODE_BASE => {
                                set_cur_val(st, get_cp_code(f.0, code, LEFT_SIDE), ValueLevel::Int)
                            }
                            RP_CODE_BASE => {
                                set_cur_val(st, get_cp_code(f.0, code, RIGHT_SIDE), ValueLevel::Int)
                            }
                            _ => {}
                        }
                    }
                }
            }
            REGISTER => fetch_register(st, m)?,
            LAST_ITEM => {
                let code = LastItemCode::from_raw(m).ok_or(Abort::Confusion("last_item"))?;
                if code < LastItemCode::InputLineNo {
                    fetch_last_node_item(st, code);
                } else if code < LastItemCode::XetexGlyphBounds {
                    fetch_last_item_int(st, code)?;
                } else if code < LastItemCode::MuToGlue {
                    fetch_last_item_dimen(st, code)?;
                } else {
                    match code {
                        LastItemCode::MuToGlue => {
                            scan_mu_glue(st)?;
                            st.scanner.cur_val_level = ValueLevel::Glue;
                        }
                        LastItemCode::GlueToMu => {
                            scan_normal_glue(st)?;
                            st.scanner.cur_val_level = ValueLevel::Mu;
                        }
                        _ => {
                            st.scanner.cur_val_level = match code {
                                LastItemCode::NumExpr => ValueLevel::Int,
                                LastItemCode::DimExpr => ValueLevel::Dimen,
                                LastItemCode::GlueExpr => ValueLevel::Glue,
                                _ => ValueLevel::Mu,
                            };
                            scan_expr(st)?;
                        }
                    }
                    while st.scanner.cur_val_level > level {
                        if st.scanner.cur_val_level == ValueLevel::Glue {
                            glue_to_width(st);
                        } else if st.scanner.cur_val_level == ValueLevel::Mu {
                            mu_error(st)?;
                        }
                        st.scanner.cur_val_level = st.scanner.cur_val_level.lower();
                    }
                    if negative {
                        if st.scanner.cur_val_level >= ValueLevel::Glue {
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
                capture_to_diagnostic(st, None);
                st.errors
                    .help(&["I'm forgetting what you said and using zero instead."]);
                error(st)?;
                set_cur_val(
                    st,
                    0,
                    if level != ValueLevel::Tok {
                        ValueLevel::Dimen
                    } else {
                        ValueLevel::Int
                    },
                );
            }
        }
        break;
    }

    while st.scanner.cur_val_level > level {
        if st.scanner.cur_val_level == ValueLevel::Glue {
            let width = st.scanner.cur_glue().width(&st.mem);
            st.scanner.set_cur_dimen(width);
        } else if st.scanner.cur_val_level == ValueLevel::Mu {
            mu_error(st)?;
        }
        st.scanner.cur_val_level = st.scanner.cur_val_level.lower();
    }

    if negative {
        if st.scanner.cur_val_level >= ValueLevel::Glue {
            negate_glue(st)?;
        } else {
            st.scanner.cur_val = -st.scanner.cur_val;
        }
    } else if (ValueLevel::Glue..=ValueLevel::Mu).contains(&(st.scanner.cur_val_level)) {
        let spec = st.scanner.cur_glue();
        let refs = spec.ref_count(&st.mem);
        spec.set_ref_count(&mut st.mem, refs + 1);
    }
    Ok(())
}

/// Scans a font identifier into `cur_val` and returns it.
pub(crate) fn scan_font_ident(st: &mut EngineState) -> Result<FontId, Abort> {
    get_x_non_blank(st)?;
    let f = match st.scanner.cur_cmd {
        DEF_FONT => st.eqtb.cur_font(),
        SET_FONT => FontId(st.scanner.cur_chr),
        DEF_FAMILY => {
            let m = st.scanner.cur_chr;
            scan_math_fam_int(st)?;
            FontId(st.eqtb.equiv(m + st.scanner.cur_val))
        }
        _ => {
            print_err(st, "Missing font identifier");
            st.errors.help(&[
                "I was looking for a control sequence whose",
                r"current meaning has been defined by \font.",
            ]);
            back_error(st)?;
            NULL_FONT
        }
    };
    st.scanner.cur_val = f.0;
    Ok(f)
}

pub(crate) fn find_font_dimen(st: &mut EngineState, writing: bool) -> Result<(), Abort> {
    scan_int(st)?;
    let n = st.scanner.cur_val;
    let f = scan_font_ident(st)?;

    if n <= 0 {
        st.scanner.cur_val = st.fonts.fmem_ptr;
    } else {
        if writing
            && (SPACE_CODE..=SPACE_SHRINK_CODE).contains(&n)
            && !st.fonts.font_glue[f].is_null()
        {
            st.mem
                .delete_glue_ref(NodePtr::from_glue_ptr(st.fonts.font_glue[f]));
            st.fonts.font_glue[f] = MemPtr::NULL;
        }

        if n <= st.fonts.font_params[f] {
            st.scanner.cur_val = n + st.fonts.param_base[f];
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
                st.fonts.set_font_info_sc(st.fonts.fmem_ptr, Scaled::ZERO);
                st.fonts.fmem_ptr += 1;
                st.fonts.font_params[f] += 1;
                if n == st.fonts.font_params[f] {
                    break;
                }
            }
            st.scanner.cur_val = st.fonts.fmem_ptr - 1;
        }
    }

    if st.scanner.cur_val == st.fonts.fmem_ptr {
        error_here_with_diagnostic(st, "Font ");
        crate::output::print_esc(st, st.eqtb.font_id_text(f));
        print(st, " has only ");
        print_int(st, st.fonts.font_params[f]);
        print(st, " fontdimen parameters");
        capture_to_diagnostic(st, None);
        st.errors.help(&[
            "To increase the number of font parameters, you must",
            r"use \fontdimen immediately after the \font is loaded.",
        ]);
        error(st)?;
    }
    Ok(())
}

pub(crate) fn scan_glyph_number(st: &mut EngineState, f: FontId) -> Result<(), Abort> {
    if scan_keyword(st, "/")? {
        crate::io::scan_and_pack_name(st)?;
        let name = st.input.name_of_file_bytes();
        let v = st.fonts.map_glyph_to_index(f, name)?;
        set_cur_val(st, v, ValueLevel::Int);
    } else if scan_keyword(st, "u")? {
        scan_char_num(st)?;
        let v = st.fonts.map_char_to_glyph(f, st.scanner.cur_val)?;
        set_cur_val(st, v, ValueLevel::Int);
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
    st.scanner.scanner_status = ScannerStatus::Absorbing;
    st.scanner.warning_index = st.scanner.cur_cs;
    let mut p = new_def_ref(st)?;
    scan_left_brace(st)?;
    let mut unbalance: i32 = 1;
    loop {
        get_token(st)?;
        if st.scanner.cur_tok.is_brace() {
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

fn scan_toks_to_string(st: &mut EngineState) -> Result<StrNumber, Abort> {
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

    let range = |s: StrNumber| {
        let start = st.strings.str_start[s.slot()] as usize;
        let end = st.strings.str_start[s.next().slot()] as usize;
        &st.strings.str_pool[start..end]
    };
    let result = match range(s1).cmp(range(s2)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    };

    st.strings.flush_str(s2);
    st.strings.flush_str(s1);
    set_cur_val(st, result, ValueLevel::Int);
    Ok(())
}
