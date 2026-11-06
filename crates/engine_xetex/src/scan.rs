//! The tokenizer (`get_next`) and the scanning routines built on it.

use crate::cmd::Cmd;
use crate::codes::{DelimNumCode, MathCharNumCode, ValueLevel};
use crate::consts::{
    ACTIVE_BASE, ACTIVE_MATH_CHAR, BIGGEST_USV, EQTB_SIZE, FROZEN_CONTROL_SEQUENCE,
    FROZEN_PROTECTION, MATH_CHAR, NUMBER_MATH_FAMILIES,
};
use crate::errors::{error, Abort};
use crate::input::{back_error, back_input, begin_token_list, ins_error, TokenType};
use crate::mem::BACKUP_HEAD;
use crate::node::{AnyBox, MathField, NodePtr, RuleDimen, RuleNode};
use crate::output::{capture_to_diagnostic, error_here_with_diagnostic, print_cmd_chr, print_err};
use crate::packaging::{ADDITIONAL, EXACTLY};
use crate::params::IntPar;
use crate::state::{EngineState, EqtbState, ErrorState, GroupCode, MemState, ScannerState};
use crate::token::{Token, TokenList};
use crate::units::{MemPtr, NodeType, Scaled};

mod internal;
mod next;
mod numbers;
mod toks;

use self::internal::find_sa;
pub(crate) use self::internal::{
    find_font_dimen, sa_token_list, scan_font_ident, scan_glyph_number,
};
use self::next::fetch_cs_meaning;
pub(crate) use self::next::{
    copy_delimited_tokens, copy_group_tokens, get_next, get_token, get_x_non_blank,
    get_x_non_blank_non_relax, get_x_or_protected, get_x_token, pass_text, x_token,
};
pub(crate) use self::numbers::{
    scan_char_class, scan_char_class_not_ignored, scan_char_num, scan_delimiter_int, scan_dimen,
    scan_eight_bit_int, scan_fifteen_bit_int, scan_four_bit_int, scan_four_bit_int_or_18,
    scan_glue, scan_int, scan_math_class_int, scan_math_fam_int, scan_register_num, scan_usv_num,
    scan_xetex_math_char_int, xetex_scan_dimen,
};
pub(crate) use self::toks::{
    compare_strings, finish_line, pseudo_start, read_toks, scan_pdf_ext_toks, scan_toks,
    store_new_token, the_toks,
};

pub(crate) const NO_EXPAND_FLAG: i32 = BIGGEST_USV + 2;
const DEFAULT_RULE: i32 = 26214;

const HYPHENATABLE_LENGTH_LIMIT: i32 = 4095;

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
    if st.scanner.cur_cmd == Cmd::Vrule {
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

#[allow(non_snake_case)]
pub(crate) fn eTeX_enabled(st: &mut EngineState, b: bool, j: Cmd, k: i32) -> Result<bool, Abort> {
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

pub(crate) fn get_r_token(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        loop {
            get_token(st)?;
            if st.scanner.cur_tok != Token::SPACE {
                break;
            }
        }
        if st.scanner.cur_cs.is_none()
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
            if st.scanner.cur_cs.is_none() {
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

/// Scans one math field into the nucleus-like field `p`.
pub(crate) fn scan_math(st: &mut EngineState, p: NodePtr<MathField>) -> Result<(), Abort> {
    let c: i32;
    'main: loop {
        get_x_non_blank_non_relax(st)?;
        c = loop {
            break match st.scanner.cur_cmd {
                Cmd::Letter | Cmd::OtherChar | Cmd::CharGiven => {
                    let mc = st.eqtb.math_code(st.scanner.cur_chr);
                    if mc & 0x1FFFFF == ACTIVE_MATH_CHAR {
                        st.scanner.cur_cs = ACTIVE_BASE + st.scanner.cur_chr;
                        fetch_cs_meaning(st);
                        x_token(st)?;
                        back_input(st)?;
                        continue 'main;
                    }
                    mc
                }
                Cmd::CharNum => {
                    scan_char_num(st)?;
                    st.scanner.cur_chr = st.scanner.cur_val;
                    st.scanner.cur_cmd = Cmd::CharGiven;
                    continue;
                }
                Cmd::MathCharNum => scan_math_char_num(st)?,
                Cmd::MathGiven => math_code_from_tex(st.scanner.cur_chr),
                Cmd::XetexMathGiven => st.scanner.cur_chr,
                Cmd::DelimNum => scan_delim_num(st)?,
                _ => {
                    back_input(st)?;
                    scan_left_brace(st)?;
                    st.save_stack.set_saved(0, p.idx().raw());
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
        st.scanner.cur_cs = ACTIVE_BASE + st.scanner.cur_chr;
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

/// Builds the token list for `\the` (or `\unexpanded`/`\detokenize`) at
/// `link(temp_head)` and returns its tail.
/// The math code given by `\mathchar` or a XeTeX form, per `cur_chr`.
pub(crate) fn scan_math_char_num(st: &mut EngineState) -> Result<i32, Abort> {
    Ok(match st.scanner.chr_code::<MathCharNumCode>() {
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
    Ok(match st.scanner.chr_code::<DelimNumCode>() {
        Some(DelimNumCode::Udelimiter) => scan_umath_code(st)?,
        _ => {
            scan_delimiter_int(st)?;
            math_code_from_tex(st.scanner.cur_val / 4096)
        }
    })
}

pub(crate) fn scan_keyword(st: &mut EngineState, keyword: &str) -> Result<bool, Abort> {
    let keyword = keyword.as_bytes();
    let save_cur_cs = st.scanner.cur_cs;
    let mut p = BACKUP_HEAD;
    p.set_link(&mut st.mem, TokenList::NULL);
    let mut i = 0;
    while i < keyword.len() {
        get_x_token(st)?;
        let ch = i32::from(keyword[i]);
        if st.scanner.cur_cs.is_none()
            && (st.scanner.cur_chr == ch || st.scanner.cur_chr == ch - 32)
        {
            store_new_token(st, &mut p, st.scanner.cur_tok)?;
            i += 1;
        } else if st.scanner.cur_cmd != Cmd::Spacer || p != BACKUP_HEAD {
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
    st.mem.flush_token_list_with_tail(matched, p);
    Ok(true)
}

pub(crate) fn scan_left_brace(st: &mut EngineState) -> Result<(), Abort> {
    get_x_non_blank_non_relax(st)?;
    if st.scanner.cur_cmd != Cmd::LeftBrace {
        print_err(st, "Missing { inserted");
        st.errors.help(&[
            "A left brace was mandatory here, so I've put one in.",
            "You might want to delete and/or insert some corrections",
            "so that I will find a matching right brace soon.",
            "(If you're confused by all this, try typing `I}' now.)",
        ]);
        back_error(st)?;
        st.scanner.cur_tok = Token::left_brace(b'{');
        st.scanner.cur_cmd = Cmd::LeftBrace;
        st.scanner.cur_chr = i32::from(b'{');
        st.scanner.align_state += 1;
    }
    Ok(())
}
