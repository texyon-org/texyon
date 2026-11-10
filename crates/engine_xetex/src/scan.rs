//! The tokenizer (`get_next`) and the scanning routines built on it.

use crate::cmd::Cmd;
use crate::codes::{DelimNumCode, MathCharNumCode, ValueLevel};
use crate::command::Command;
use crate::consts::{
    ACTIVE_BASE, ACTIVE_MATH_CHAR, BIGGEST_USV, EQTB_SIZE, FROZEN_CONTROL_SEQUENCE,
    FROZEN_PROTECTION, MATH_CHAR, NUMBER_MATH_FAMILIES,
};
use crate::errors::Abort;
use crate::input::{back_input, begin_token_list, TokenType};
use crate::mem::BACKUP_HEAD;
use crate::node::{BoxNode, MathField, NodePtr, RuleDimen, RuleNode, SaLeaf, Stored};
use crate::packaging::PackSpec;
use crate::params::IntPar;
use crate::state::{EngineState, EqtbState, GroupCode, MemState, ScannerState};
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
    crate::errors::ins_error(
        st,
        "Missing $ inserted",
        &[
            "I've inserted a begin-math/end-math symbol since I think",
            "you left one out. Proceed, with fingers crossed.",
        ],
    )
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
    let spec = if scan_keyword(st, "to")? {
        PackSpec::Exactly(scan_dimen(st, false, false)?)
    } else if scan_keyword(st, "spread")? {
        PackSpec::Additional(scan_dimen(st, false, false)?)
    } else {
        PackSpec::NATURAL
    };
    if three_codes {
        st.save_stack.set_saved(0, s);
        st.save_stack.save_ptr += 1;
    }
    st.save_stack.push_spec(spec);
    st.save_stack.new_save_level(c, st.mem.line)?;
    scan_left_brace(st)?;
    Ok(())
}

/// The scaled fraction `.d0 d1 ...` for decimal `digits`, rounded.
fn round_decimals(digits: &[u8]) -> i32 {
    let a = digits
        .iter()
        .rev()
        .fold(0, |a, &d| (a + i32::from(d) * 0x2_0000) / 10);
    (a + 1) / 2
}

pub(crate) fn scan_rule_spec(st: &mut EngineState) -> Result<NodePtr<RuleNode>, Abort> {
    let r = st.mem.new_rule()?;
    if st.scanner.cur_cmd == Cmd::Vrule {
        st.mem[r].width = RuleDimen::from_raw(DEFAULT_RULE).into();
    } else {
        st.mem[r].height = RuleDimen::from_raw(DEFAULT_RULE).into();
        st.mem[r].depth = RuleDimen::Set(Scaled::ZERO).into();
    }
    loop {
        if scan_keyword(st, "width")? {
            let d = xetex_scan_dimen(st, false, false, true)?;
            st.mem[r].width = RuleDimen::from_raw(d.raw()).into();
        } else if scan_keyword(st, "height")? {
            let d = xetex_scan_dimen(st, false, false, true)?;
            st.mem[r].height = RuleDimen::from_raw(d.raw()).into();
        } else if scan_keyword(st, "depth")? {
            let d = xetex_scan_dimen(st, false, false, true)?;
            st.mem[r].depth = RuleDimen::from_raw(d.raw()).into();
        } else {
            break;
        }
    }
    Ok(r)
}

#[allow(non_snake_case)]
pub(crate) fn eTeX_enabled(st: &mut EngineState, b: bool, j: Cmd, k: i32) -> Result<bool, Abort> {
    if !b {
        crate::errors::error(
            st,
            ("Improper ", crate::output::CmdChr(j, k)),
            &["Sorry, this optional e-TeX feature has been disabled."],
        )?;
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
            if st.scanner.cur_cs.is_none() {
                back_input(st)?;
            }
            st.scanner.cur_tok = Token::from_cs(FROZEN_PROTECTION);
            crate::errors::ins_error(
                st,
                "Missing control sequence inserted",
                &[
                    r"Please don't say `\def cs{...}', say `\def\cs{...}'.",
                    "I've inserted an inaccessible control sequence so that your",
                    "definition will be completed without mixing me up too badly.",
                    "You can recover graciously from this error, if you're",
                    "careful; see exercise 27.2 in The TeXbook.",
                ],
            )?;
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
    let class = set_class(scan_math_class_int(st)?);
    let family = set_family(scan_math_fam_int(st)?);
    Ok(class + family + scan_usv_num(st)?)
}

/// Scans one math field into the nucleus-like field `p`.
pub(crate) fn scan_math(st: &mut EngineState, p: NodePtr<MathField>) -> Result<(), Abort> {
    let c: i32;
    'main: loop {
        get_x_non_blank_non_relax(st)?;
        c = loop {
            break match st.scanner.try_command() {
                Some(Command::Letter(ch) | Command::OtherChar(ch) | Command::CharGiven(ch)) => {
                    let mc = st.eqtb.math_code(ch.raw());
                    if mc & 0x1FFFFF == ACTIVE_MATH_CHAR {
                        st.scanner.cur_cs = ACTIVE_BASE + ch.raw();
                        fetch_cs_meaning(st);
                        x_token(st)?;
                        back_input(st)?;
                        continue 'main;
                    }
                    mc
                }
                Some(Command::CharNum) => {
                    st.scanner.cur_chr = scan_char_num(st)?;
                    st.scanner.cur_cmd = Cmd::CharGiven;
                    continue;
                }
                Some(Command::MathCharNum(code)) => scan_math_char_num(st, code)?,
                Some(Command::MathGiven(code)) => math_code_from_tex(code),
                Some(Command::XetexMathGiven(code)) => code,
                Some(Command::DelimNum(code)) => scan_delim_num(st, code)?,
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
    st.mem[p].math_type = MATH_CHAR;
    st.mem[p].set_character((c % 65536) as u16);
    let cur_fam = st.eqtb.int_par(IntPar::CurFam);
    let fam = if math_class(c) == 7 && (0..NUMBER_MATH_FAMILIES).contains(&cur_fam) {
        cur_fam as u16
    } else {
        math_fam(c) as u16
    };
    st.mem[p].set_plane_and_fam(fam + (math_char(c) / 65536) as u16 * 256);
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
        st.mem[nd.nucleus()].math_type = MATH_CHAR;
        let ch = math_char(c);
        st.mem[nd.nucleus()].set_character((ch % 65536) as u16);
        st.mem[nd.nucleus()].set_plane_and_fam(math_fam(c) as u16);
        if math_class(c) == 7 {
            let cf = st.eqtb.int_par(IntPar::CurFam);
            if (0..NUMBER_MATH_FAMILIES).contains(&cf) {
                st.mem[nd.nucleus()].set_plane_and_fam(cf as u16);
            }
            st.mem[nd].noad_type = NodeType::OrdNoad.into();
        } else {
            st.mem[nd].noad_type =
                Stored::from_raw(NodeType::OrdNoad.as_raw() + math_class(c) as u16);
        }
        let fam = st.mem[nd.nucleus()].plane_and_fam() + (ch / 65536) as u16 * 256;
        st.mem[nd.nucleus()].set_plane_and_fam(fam);
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
pub(crate) fn fetch_box(st: &mut EngineState, n: i32) -> Option<NodePtr<BoxNode>> {
    let contents = if n < 256 {
        st.eqtb.box_reg(n)
    } else {
        match find_sa(st, ValueLevel::BOX, n)? {
            SaLeaf::Pointer(p) => st.mem[p].sa_ptr,
            SaLeaf::Word(_) | SaLeaf::MarkClass(_) => return None,
        }
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
    ) -> Result<(), Abort> {
        if n < 256 {
            eqtb.set_box_reg(n, v);
            return Ok(());
        }
        if let Some(SaLeaf::Pointer(p)) = self.find_sa_element(scanner, ValueLevel::BOX, n) {
            let r = &mut self[p];
            r.sa_ptr = v;
            r.sa_ref += 1;
            self.delete_sa_ref(scanner, p.element())?;
        }
        Ok(())
    }
}

/// Builds the token list for `\the` (or `\unexpanded`/`\detokenize`) at
/// `link(temp_head)` and returns its tail.
/// The math code given by `\mathchar` or a XeTeX form.
pub(crate) fn scan_math_char_num(
    st: &mut EngineState,
    code: MathCharNumCode,
) -> Result<i32, Abort> {
    Ok(match code {
        MathCharNumCode::UmathChar => scan_umath_code(st)?,
        MathCharNumCode::UmathCharNum => scan_xetex_math_char_int(st)?,
        MathCharNumCode::MathChar => math_code_from_tex(scan_fifteen_bit_int(st)?),
    })
}

/// The math code of the small character given by `\delimiter` or `\Udelimiter`.
pub(crate) fn scan_delim_num(st: &mut EngineState, code: DelimNumCode) -> Result<i32, Abort> {
    Ok(match code {
        DelimNumCode::Udelimiter => scan_umath_code(st)?,
        DelimNumCode::Delimiter => math_code_from_tex(scan_delimiter_int(st)? / 4096),
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
        crate::errors::back_error(
            st,
            "Missing { inserted",
            &[
                "A left brace was mandatory here, so I've put one in.",
                "You might want to delete and/or insert some corrections",
                "so that I will find a matching right brace soon.",
                "(If you're confused by all this, try typing `I}' now.)",
            ],
        )?;
        st.scanner.cur_tok = Token::left_brace(b'{');
        st.scanner.cur_cmd = Cmd::LeftBrace;
        st.scanner.cur_chr = i32::from(b'{');
        st.scanner.align_state += 1;
    }
    Ok(())
}
