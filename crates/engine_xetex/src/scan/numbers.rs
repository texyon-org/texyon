//! Scanning numbers, dimensions and glue, and e-TeX expressions.

use crate::cmd::Cmd;
use crate::codes::ValueLevel;
use crate::command::RelaxCode;
use crate::consts::{
    ACTIVE_BASE, ACTIVE_MATH_CHAR, BIGGEST_CHAR, BIGGEST_USV, CHAR_CLASS_LIMIT, FROZEN_PRIMITIVE,
    FROZEN_RELAX, MAX_DIMEN, MAX_HALFWORD, NULL_CS, NUMBER_MATH_FAMILIES, SINGLE_BASE,
    TEX_INFINITY,
};
use crate::errors::OrConfusion;
use crate::errors::{mu_error, Abort};
use crate::fonts::{QUAD_CODE, X_HEIGHT_CODE};
use crate::glue_store::GlueSpecId;
use crate::input::back_input;
use crate::mem::ZERO_GLUE;
use crate::params::IntPar;
use crate::scaled_math::{add_or_sub, fract, mult_and_add, quotient, xn_over_d, Overflowing};
use crate::scan::Command;
use crate::state::EngineState;
use crate::token::Token;
use crate::units::MemIdx;
use crate::units::RawPtr;
use crate::units::{GlueOrder, Scaled};

use super::internal::{scan_something_internal, CurVal};
use super::next::{get_token, get_x_non_blank, get_x_token};
use super::{math_char, round_decimals, scan_keyword};

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

/// A suspended level of expression scanning, resumed at its closing parenthesis.
struct ExprLevel {
    l: ValueLevel,
    s: i16,
    r: i16,
    e: i32,
    t: i32,
    n: i32,
}

const EXPR_NONE: i16 = 0;
const EXPR_ADD: i16 = 1;
const EXPR_SUB: i16 = 2;
const EXPR_MULT: i16 = 3;
const EXPR_DIV: i16 = 4;
const EXPR_SCALE: i16 = 5;

/// Reports a bad integer `value` with two help lines.
pub(super) fn bad_int_error(
    st: &mut EngineState,
    err: &str,
    value: i32,
    help1: &'static str,
    help0: &'static str,
) -> Result<(), Abort> {
    crate::errors::error(st, (err, " (", value, ")"), &[help1, help0])
}

/// Reports the out-of-range integer `value` and yields zero in its place.
fn int_range_error(
    st: &mut EngineState,
    err: &str,
    value: i32,
    help1: &'static str,
) -> Result<i32, Abort> {
    bad_int_error(st, err, value, help1, "I changed this one to zero.")?;
    Ok(0)
}

fn scan_int_up_to(
    st: &mut EngineState,
    max: i32,
    err: &str,
    help: &'static str,
) -> Result<i32, Abort> {
    let n = scan_int(st)?;
    if n < 0 || n > max {
        return int_range_error(st, err, n, help);
    }
    Ok(n)
}

pub(crate) fn scan_char_class(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        CHAR_CLASS_LIMIT,
        "Bad character class",
        "A character class must be between 0 and 4096.",
    )
}

/// A class between two characters. WEB also lets 4096 through, which indexes past the
/// interchartoks tree, so this keeps to the 0..4095 its help text promises.
pub(crate) fn scan_char_class_not_ignored(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        CHAR_CLASS_LIMIT - 1,
        "Bad character class",
        "A class for inter-character transitions must be between 0 and 4095.",
    )
}

pub(crate) fn scan_eight_bit_int(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        255,
        "Bad register code",
        "A register code or char class must be between 0 and 255.",
    )
}

pub(crate) fn scan_math_class_int(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        7,
        "Bad math class",
        "Since I expected to read a number between 0 and 7,",
    )
}

pub(crate) fn scan_math_fam_int(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        NUMBER_MATH_FAMILIES - 1,
        "Bad math family",
        "Since I expected to read a number between 0 and 255,",
    )
}

pub(crate) fn scan_four_bit_int(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        15,
        "Bad number",
        "Since I expected to read a number between 0 and 15,",
    )
}

pub(crate) fn scan_four_bit_int_or_18(st: &mut EngineState) -> Result<i32, Abort> {
    let n = scan_int(st)?;
    if n < 0 || (n > 15 && n != 18) {
        return int_range_error(
            st,
            "Bad number",
            n,
            "Since I expected to read a number between 0 and 15,",
        );
    }
    Ok(n)
}

pub(crate) fn scan_fifteen_bit_int(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        32767,
        "Bad mathchar",
        "A mathchar number must be between 0 and 32767.",
    )
}

pub(crate) fn scan_char_num(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        BIGGEST_CHAR,
        "Bad character code",
        "A character number must be between 0 and 65535.",
    )
}

pub(crate) fn scan_usv_num(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        BIGGEST_USV,
        "Bad character code",
        "A Unicode scalar value must be between 0 and \"10FFFF.",
    )
}

pub(crate) fn scan_delimiter_int(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        0x7FF_FFFF,
        "Bad delimiter code",
        "A numeric delimiter code must be between 0 and 2^{27}-1.",
    )
}

pub(crate) fn scan_register_num(st: &mut EngineState) -> Result<i32, Abort> {
    let n = scan_int(st)?;
    if n < 0 || n > st.scanner.max_reg_num {
        return int_range_error(
            st,
            "Bad register code",
            n,
            "A register number must be between 0 and 32767.",
        );
    }
    Ok(n)
}

pub(crate) fn scan_xetex_math_char_int(st: &mut EngineState) -> Result<i32, Abort> {
    let n = scan_int(st)?;
    if math_char(n) == ACTIVE_MATH_CHAR {
        if n != ACTIVE_MATH_CHAR {
            bad_int_error(
                st,
                "Bad active XeTeX math code",
                n,
                "Since I ignore class and family for active math chars,",
                "I changed this one to \"1FFFFF.",
            )?;
            return Ok(ACTIVE_MATH_CHAR);
        }
    } else if math_char(n) > BIGGEST_USV {
        return int_range_error(
            st,
            "Bad XeTeX math character code",
            n,
            "Since I expected a character number between 0 and \"10FFFF,",
        );
    }
    Ok(n)
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
    if st.scanner.cur.cmd() != Cmd::Spacer {
        back_input(st)?;
    }
    Ok(())
}

/// Reads the token after `\primitive` and replaces it by the primitive's meaning.
pub(super) fn get_primitive_token(st: &mut EngineState) -> Result<(), Abort> {
    get_token(st)?;
    if let Some(p) = st.eqtb.prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)? {
        st.scanner.cur = st.eqtb.meaning(p.eqtb_loc());
        st.scanner.cur_cs = p.eqtb_loc();
        st.scanner.cur_tok = Token::from_cs(st.scanner.cur_cs);
    } else {
        st.scanner.cur = Command::Relax(RelaxCode::Zero);
        st.scanner.cur_tok = Token::from_cs(FROZEN_RELAX);
        st.scanner.cur_cs = FROZEN_RELAX;
    }
    Ok(())
}

pub(crate) fn scan_int(st: &mut EngineState) -> Result<i32, Abort> {
    Ok(scan_int_decimal(st)?.0)
}

/// `scan_int` that also returns whether the integer was a decimal constant.
fn scan_int_decimal(st: &mut EngineState) -> Result<(i32, bool), Abort> {
    let mut ok_so_far = true;
    let mut v: i32;
    let mut decimal = false;
    let negative = scan_signs(st)?;

    loop {
        if st.scanner.cur_tok == ALPHA_TOKEN {
            get_token(st)?;
            if !st.scanner.cur_tok.is_cs() {
                v = st.scanner.cur_tok.chr();
                if st.scanner.cur.cmd() == Cmd::RightBrace {
                    st.scanner.align_state += 1;
                } else if st.scanner.cur.cmd() < Cmd::RightBrace {
                    st.scanner.align_state -= 1;
                }
            } else if st.scanner.cur_tok < Token::from_cs(SINGLE_BASE) {
                v = st.scanner.cur_tok.cs() - ACTIVE_BASE;
            } else if st.scanner.cur_tok < Token::from_cs(NULL_CS) {
                v = st.scanner.cur_tok.cs() - SINGLE_BASE;
            } else {
                v = st
                    .strings
                    .astral_char(st.eqtb.text(st.scanner.cur_tok.cs()));
            }
            if v > BIGGEST_USV {
                v = i32::from(b'0');
                crate::errors::back_error(
                    st,
                    "Improper alphabetic constant",
                    &[
                        "A one-character control sequence belongs after a ` mark.",
                        r"So I'm essentially inserting \0 here.",
                    ],
                )?;
            } else {
                scan_optional_space(st)?;
            }
        } else if st.scanner.cur_tok == FROZEN_PRIM_TOKEN {
            get_primitive_token(st)?;
            continue;
        } else if st.scanner.cur.cmd() >= Cmd::MIN_INTERNAL
            && st.scanner.cur.cmd() <= Cmd::MAX_INTERNAL
        {
            v = match scan_something_internal(st, ValueLevel::Int, false)? {
                CurVal::Int(n) => n,
                _ => return Err(Abort::Confusion("scan_int")),
            };
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
            decimal = radix == 10;
            let mut vacuous = true;
            v = 0;
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
                if v >= m && (v > m || d > 7 || radix != 10) {
                    if ok_so_far {
                        crate::errors::error(
                            st,
                            "Number too big",
                            &[
                                "I can only go up to 2147483647='17777777777=\"7FFFFFFF,",
                                "so I'm using that number instead of yours.",
                            ],
                        )?;
                        v = TEX_INFINITY;
                        ok_so_far = false;
                    }
                } else {
                    v = v * radix + d;
                }
                get_x_token(st)?;
            }
            if vacuous {
                crate::errors::back_error(
                    st,
                    "Missing number, treated as zero",
                    &[
                        "A number should have been here; I inserted `0'.",
                        "(If you can't figure out why I needed to see a number,",
                        "look up `weird error' in the index to The TeXbook.)",
                    ],
                )?;
            } else if st.scanner.cur.cmd() != Cmd::Spacer {
                back_input(st)?;
            }
        }
        break;
    }

    if negative {
        v = v.wrapping_neg();
    }
    Ok((v, decimal))
}

/// Scans a dimension, in math units if `mu`.
pub(crate) fn scan_dimen(st: &mut EngineState, mu: bool) -> Result<Scaled, Abort> {
    Ok(scan_dimen_from(st, mu, false, None, true)?.0)
}

/// Scans a number with an optional fraction and no unit, as XeTeX's
/// `xetex_scan_dimen` does for an angle.
pub(crate) fn scan_decimal(st: &mut EngineState) -> Result<Scaled, Abort> {
    Ok(scan_dimen_from(st, false, false, None, false)?.0)
}

/// Scans the amount after `plus` or `minus`, which may be `fil`, `fill` or `filll`.
fn scan_stretch(st: &mut EngineState, mu: bool) -> Result<(Scaled, GlueOrder), Abort> {
    scan_dimen_from(st, mu, true, None, true)
}

/// A glue spec pointer held in a raw word.
pub(super) fn glue_spec(p: i32) -> GlueSpecId {
    GlueSpecId::from_raw(MemIdx::from_raw(p).raw())
}

/// The width of a scanned glue value, releasing the glue.
fn release_to_width(st: &mut EngineState, spec: GlueSpecId) -> Scaled {
    let width = st.mem[spec].width;
    st.mem.delete_glue_ref(spec);
    width
}

/// Scales `v` and the fraction `f` by `num / denom`. Returns whether that overflowed.
fn scale_dimen(v: &mut i32, f: &mut i32, num: i32, denom: i32) -> bool {
    let scaled = xn_over_d(*v, num, denom);
    *v = scaled.quotient;
    *f = (num * *f + 65536 * scaled.remainder) / denom;
    *v += *f / 65536;
    *f %= 65536;
    scaled.overflowed
}

/// `integer.fraction` times the unit `v`.
fn attach_unit(integer: i32, fraction: i32, v: Scaled) -> Overflowing<Scaled> {
    let fraction_part = xn_over_d(v, fraction, 65536);
    let mut whole = mult_and_add(integer, v, fraction_part.quotient, MAX_DIMEN);
    whole.overflowed |= fraction_part.overflowed;
    whole
}

/// Scans a dimension and its glue order, which `inf` allows to be infinite.
/// `shortcut` is an integer already scanned, which then only needs units.
fn scan_dimen_from(
    st: &mut EngineState,
    mu: bool,
    inf: bool,
    shortcut: Option<i32>,
    requires_units: bool,
) -> Result<(Scaled, GlueOrder), Abort> {
    let mut f: i32 = 0;
    let mut negative = false;
    let mut v: i32;

    let mut overflowed = false;
    let mut order = GlueOrder::Normal;

    'attach_sign: {
        if let Some(n) = shortcut {
            v = n;
        } else {
            negative = scan_signs(st)?;

            if st.scanner.cur.cmd() >= Cmd::MIN_INTERNAL
                && st.scanner.cur.cmd() <= Cmd::MAX_INTERNAL
            {
                if mu {
                    let val = scan_something_internal(st, ValueLevel::Mu, false)?;
                    v = match val.glue() {
                        Some(spec) => release_to_width(st, spec),
                        None => val.scaled()?,
                    }
                    .raw();
                    match val {
                        CurVal::Mu(_) => break 'attach_sign,
                        CurVal::Int(_) => {}
                        _ => mu_error(st)?,
                    }
                } else {
                    let val = scan_something_internal(st, ValueLevel::Dimen, false)?;
                    v = val.scaled()?.raw();
                    if let CurVal::Dimen(_) = val {
                        break 'attach_sign;
                    }
                }
            } else {
                back_input(st)?;
                if st.scanner.cur_tok == CONTINENTAL_POINT_TOKEN {
                    st.scanner.cur_tok = POINT_TOKEN;
                }
                let decimal;
                if st.scanner.cur_tok != POINT_TOKEN {
                    (v, decimal) = scan_int_decimal(st)?;
                } else {
                    (v, decimal) = (0, true);
                }
                if st.scanner.cur_tok == CONTINENTAL_POINT_TOKEN {
                    st.scanner.cur_tok = POINT_TOKEN;
                }

                if decimal && st.scanner.cur_tok == POINT_TOKEN {
                    // Up to 17 digits are kept, most recent first, in a temporary list.
                    let mut k: usize = 0;
                    let mut dig = [0u8; 17];
                    get_token(st)?;
                    loop {
                        get_x_token(st)?;
                        let Some(d) = st.scanner.cur_tok.other_digit() else {
                            break;
                        };
                        if k < 17 {
                            dig[k] = u8::try_from(d).or_confusion("digit")?;
                            k += 1;
                        }
                    }
                    f = round_decimals(&dig[..k]);
                    if st.scanner.cur.cmd() != Cmd::Spacer {
                        back_input(st)?;
                    }
                }
            }
        }

        if v < 0 {
            negative = !negative;
            v = v.wrapping_neg();
        }

        if requires_units {
            let apply_fraction = 'units: {
                if inf && scan_keyword(st, "fil")? {
                    order = GlueOrder::Fil;
                    while scan_keyword(st, "l")? {
                        match order.next() {
                            Some(next) => order = next,
                            None => {
                                crate::errors::error(
                                    st,
                                    "Illegal unit of measure (replaced by filll)",
                                    &["I dddon't go any higher than filll."],
                                )?;
                            }
                        }
                    }
                    break 'units true;
                }

                let integer = v;
                get_x_non_blank(st)?;

                if st.scanner.cur.cmd() >= Cmd::MIN_INTERNAL
                    && st.scanner.cur.cmd() <= Cmd::MAX_INTERNAL
                {
                    let unit = if mu {
                        let val = scan_something_internal(st, ValueLevel::Mu, false)?;
                        let unit = match val.glue() {
                            Some(spec) => release_to_width(st, spec),
                            None => val.scaled()?,
                        };
                        if !matches!(val, CurVal::Mu(_)) {
                            mu_error(st)?;
                        }
                        unit
                    } else {
                        scan_something_internal(st, ValueLevel::Dimen, false)?.scaled()?
                    };
                    v = attach_unit(integer, f, unit).track(&mut overflowed).raw();
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
                        let unit = st.fonts.param(font, code);
                        scan_optional_space(st)?;
                        v = attach_unit(integer, f, unit).track(&mut overflowed).raw();
                        break 'attach_sign;
                    }
                }

                if mu {
                    if scan_keyword(st, "mu")? {
                        break 'units true;
                    } else {
                        crate::errors::error(
                            st,
                            "Illegal unit of measure (mu inserted)",
                            &[
                                "The unit of measurement in math glue must be mu.",
                                "To recover gracefully from this error, it's best to",
                                "delete the erroneous units; e.g., type `2' to delete",
                                "two letters. (See Chapter 27 of The TeXbook.)",
                            ],
                        )?;
                        break 'units true;
                    }
                }

                if scan_keyword(st, "true")? {
                    crate::save::prepare_mag(st)?;
                    let mag = st.eqtb.int_par(IntPar::Mag);
                    if mag != 1000 {
                        overflowed |= scale_dimen(&mut v, &mut f, 1000, mag);
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
                    crate::errors::error(
                        st,
                        "Illegal unit of measure (pt inserted)",
                        &[
                            "Dimensions can be in units of em, ex, in, pt, pc,",
                            "cm, mm, dd, cc, bp, or sp; but yours is a new one!",
                            "I'll assume that you meant to say pt, for printer's points.",
                            "To recover gracefully from this error, it's best to",
                            "delete the erroneous units; e.g., type `2' to delete",
                            "two letters. (See Chapter 27 of The TeXbook.)",
                        ],
                    )?;
                    break 'units true;
                };
                overflowed |= scale_dimen(&mut v, &mut f, num, denom);
                true
            };

            if apply_fraction {
                overflowed |= attach_fraction(&mut v, f);
            }
            scan_optional_space(st)?;
        } else {
            overflowed |= attach_fraction(&mut v, f);
        }
    }

    if overflowed || v.wrapping_abs() >= 0x4000_0000 {
        crate::errors::error(
            st,
            "Dimension too large",
            &[
                "I can't work with sizes bigger than about 19 feet.",
                "Continue and I'll use the largest value I can.",
            ],
        )?;
        v = MAX_HALFWORD;
    }
    if negative {
        v = v.wrapping_neg();
    }
    Ok((Scaled::from_raw(v), order))
}

/// Makes `v.f` a scaled value in `v`. Returns whether it is too large for one.
fn attach_fraction(v: &mut i32, f: i32) -> bool {
    if *v >= 16384 {
        return true;
    }
    *v = v.wrapping_mul(65536).wrapping_add(f);
    false
}

pub(crate) fn scan_glue(st: &mut EngineState, level: ValueLevel) -> Result<GlueSpecId, Abort> {
    let mu = level == ValueLevel::Mu;
    let negative = scan_signs(st)?;

    let width =
        if st.scanner.cur.cmd() >= Cmd::MIN_INTERNAL && st.scanner.cur.cmd() <= Cmd::MAX_INTERNAL {
            let val = scan_something_internal(st, level, negative)?;
            if let Some(spec) = val.glue() {
                if val.level() != level {
                    mu_error(st)?;
                }
                return Ok(spec);
            }
            match val {
                CurVal::Int(n) => scan_dimen_from(st, mu, false, Some(n), true)?.0,
                _ => {
                    if level == ValueLevel::Mu {
                        mu_error(st)?;
                    }
                    val.scaled()?
                }
            }
        } else {
            back_input(st)?;
            let d = scan_dimen(st, mu)?;
            if negative {
                -d
            } else {
                d
            }
        };

    let gs = st.mem.new_spec(ZERO_GLUE)?;
    st.mem[gs].width = width;

    if scan_keyword(st, "plus")? {
        let (stretch, order) = scan_stretch(st, mu)?;
        st.mem[gs].stretch = stretch;
        st.mem[gs].stretch_order = order.into();
    }
    if scan_keyword(st, "minus")? {
        let (shrink, order) = scan_stretch(st, mu)?;
        st.mem[gs].shrink = shrink;
        st.mem[gs].shrink_order = order.into();
    }

    Ok(gs)
}
pub(super) fn scan_normal_glue(st: &mut EngineState) -> Result<GlueSpecId, Abort> {
    scan_glue(st, ValueLevel::Glue)
}
pub(super) fn scan_mu_glue(st: &mut EngineState) -> Result<GlueSpecId, Abort> {
    scan_glue(st, ValueLevel::Mu)
}
/// Applies `op` to the width, stretch and shrink of `spec`, in that order.
pub(super) fn map_glue(
    st: &mut EngineState,
    spec: GlueSpecId,
    mut op: impl FnMut(Scaled) -> Scaled,
) {
    let node = &mut st.mem[spec];
    node.width = op(node.width);
    node.stretch = op(node.stretch);
    node.shrink = op(node.shrink);
}

fn normalize_glue_orders(st: &mut EngineState, spec: GlueSpecId) {
    if st.mem[spec].stretch == Scaled::ZERO {
        st.mem[spec].stretch_order = GlueOrder::Normal.into();
    }
    if st.mem[spec].shrink == Scaled::ZERO {
        st.mem[spec].shrink_order = GlueOrder::Normal.into();
    }
}

/// Adds (or subtracts) glue `t` to glue `e` and releases `t`. Raises
/// `overflowed` if a component overflows.
fn add_glue(st: &mut EngineState, e: GlueSpecId, t: GlueSpecId, sub: bool, overflowed: &mut bool) {
    let mem = &mut st.mem;
    let width = add_or_sub(mem[e].width, mem[t].width, MAX_DIMEN, sub).track(overflowed);
    mem[e].width = width;
    let (e_order, t_order, t_stretch) =
        (mem[e].stretch_order, mem[t].stretch_order, mem[t].stretch);
    if e_order == t_order {
        let stretch = add_or_sub(mem[e].stretch, t_stretch, MAX_DIMEN, sub).track(overflowed);
        mem[e].stretch = stretch;
    } else if e_order < t_order && t_stretch != Scaled::ZERO {
        mem[e].stretch = t_stretch;
        mem[e].stretch_order = t_order;
    }
    let (e_order, t_order, t_shrink) = (mem[e].shrink_order, mem[t].shrink_order, mem[t].shrink);
    if e_order == t_order {
        let shrink = add_or_sub(mem[e].shrink, t_shrink, MAX_DIMEN, sub).track(overflowed);
        mem[e].shrink = shrink;
    } else if e_order < t_order && t_shrink != Scaled::ZERO {
        mem[e].shrink = t_shrink;
        mem[e].shrink_order = t_order;
    }
    mem.delete_glue_ref(t);
    normalize_glue_orders(st, e);
}

/// Scans an `\numexpr`-style expression whose result has level `l`.
pub(super) fn scan_expr(st: &mut EngineState, mut l: ValueLevel) -> Result<CurVal, Abort> {
    let mut b = false;
    let mut stack: Vec<ExprLevel> = Vec::new();

    st.scanner.expand_depth_count += 1;
    if st.scanner.expand_depth_count >= st.scanner.expand_depth {
        return Err(Abort::Overflow {
            capacity: crate::errors::Capacity::ExpansionDepth,
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
                    stack.push(ExprLevel { l, s, r, e, t, n });
                    l = o;
                    continue 'restart;
                }
                back_input(st)?;
                f = match o {
                    ValueLevel::Int => scan_int(st)?,
                    ValueLevel::Dimen => scan_dimen(st, false)?.raw(),
                    ValueLevel::Glue => scan_normal_glue(st)?.raw(),
                    _ => scan_mu_glue(st)?.raw(),
                };
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
                if stack.is_empty() {
                    if st.scanner.cur.cmd() != Cmd::Relax {
                        back_input(st)?;
                    }
                } else if st.scanner.cur_tok != Token::other(b')') {
                    crate::errors::back_error(
                        st,
                        "Missing ) inserted for expression",
                        &["I was expecting to see `+', `-', `*', `/', or `)'. Didn't."],
                    )?;
                }
            }

            if l == ValueLevel::Int || s > EXPR_SUB {
                if !(-TEX_INFINITY..=TEX_INFINITY).contains(&f) {
                    b = true;
                    f = 0;
                }
            } else if l == ValueLevel::Dimen {
                if f.abs() > MAX_HALFWORD {
                    b = true;
                    f = 0;
                }
            } else {
                let spec = glue_spec(f);
                if st.mem[spec].width.abs() > MAX_DIMEN
                    || st.mem[spec].stretch.abs() > MAX_DIMEN
                    || st.mem[spec].shrink.abs() > MAX_DIMEN
                {
                    b = true;
                    st.mem.delete_glue_ref(glue_spec(f));
                    f = st.mem.new_spec(ZERO_GLUE)?.raw();
                }
            }

            match s {
                EXPR_NONE => {
                    if l >= ValueLevel::Glue && o != EXPR_NONE {
                        let term = st.mem.new_spec(glue_spec(f))?;
                        t = term.raw();
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
                        t = mult_and_add(t, f, 0, TEX_INFINITY).track(&mut b);
                    } else if l == ValueLevel::Dimen {
                        t = mult_and_add(t, f, 0, MAX_HALFWORD).track(&mut b);
                    } else {
                        // TeX's `nx_plus_y` here takes the component as its integer factor.
                        map_glue(st, glue_spec(t), |x| {
                            Scaled::from_raw(
                                mult_and_add(x.raw(), f, 0, MAX_HALFWORD).track(&mut b),
                            )
                        });
                    }
                }
                EXPR_DIV => {
                    if l < ValueLevel::Glue {
                        t = quotient(t, f).track(&mut b);
                    } else {
                        map_glue(st, glue_spec(t), |x| {
                            Scaled::from_raw(quotient(x.raw(), f).track(&mut b))
                        });
                    }
                }
                EXPR_SCALE => {
                    if l == ValueLevel::Int {
                        t = fract(t, n, f, TEX_INFINITY).track(&mut b);
                    } else if l == ValueLevel::Dimen {
                        t = fract(t, n, f, MAX_HALFWORD).track(&mut b);
                    } else {
                        map_glue(st, glue_spec(t), |x| {
                            fract(x, n, f, MAX_DIMEN).track(&mut b)
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
                    e = add_or_sub(e, t, TEX_INFINITY, r == EXPR_SUB).track(&mut b);
                } else if l == ValueLevel::Dimen {
                    e = add_or_sub(e, t, MAX_HALFWORD, r == EXPR_SUB).track(&mut b);
                } else {
                    add_glue(st, glue_spec(e), glue_spec(t), r == EXPR_SUB, &mut b);
                }
                r = o;
            }
            if o != EXPR_NONE {
                continue;
            }
            let Some(outer) = stack.pop() else {
                break 'restart;
            };
            f = e;
            ExprLevel { l, s, r, e, t, n } = outer;
            scan_factor = false;
        }
    }

    st.scanner.expand_depth_count -= 1;

    if b {
        crate::errors::error(
            st,
            "Arithmetic overflow",
            &[
                "I can't evaluate this expression,",
                "since the result is out of range.",
            ],
        )?;
        if l >= ValueLevel::Glue {
            st.mem.delete_glue_ref(glue_spec(e));
            e = ZERO_GLUE.raw();
            let refs = st.mem[ZERO_GLUE].ref_count;
            st.mem[ZERO_GLUE].ref_count = refs + 1;
        } else {
            e = 0;
        }
    }
    CurVal::from_word(l, e)
}
