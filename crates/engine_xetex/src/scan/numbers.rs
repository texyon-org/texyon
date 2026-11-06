//! Scanning numbers, dimensions and glue, and e-TeX expressions.

use crate::cmd::Cmd;
use crate::codes::ValueLevel;
use crate::consts::{
    ACTIVE_BASE, ACTIVE_MATH_CHAR, BIGGEST_CHAR, BIGGEST_USV, CHAR_CLASS_LIMIT, FROZEN_PRIMITIVE,
    FROZEN_RELAX, MAX_DIMEN, MAX_HALFWORD, NUMBER_MATH_FAMILIES, PRIM_EQTB_BASE, SINGLE_BASE,
    TEX_INFINITY, UNDEFINED_PRIMITIVE,
};
use crate::errors::{error, int_error, mu_error, Abort};
use crate::fonts::{QUAD_CODE, X_HEIGHT_CODE};
use crate::input::{back_error, back_input};
use crate::node::{ExprNode, GlueSpecNode, NodePtr};
use crate::output::{capture_to_diagnostic, error_here_with_diagnostic, print_err};
use crate::params::IntPar;
use crate::state::EngineState;
use crate::token::Token;
use crate::units::{GlueOrder, MemPtr, Scaled};

use super::internal::scan_something_internal;
use super::next::{get_token, get_x_non_blank, get_x_token};
use super::{math_char, scan_keyword};

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
) -> Result<i32, Abort> {
    let n = scan_int(st)?;
    if n < 0 || n > max {
        int_range_error(st, err, help)?;
    }
    Ok(st.scanner.cur_val)
}

pub(crate) fn scan_char_class(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        CHAR_CLASS_LIMIT,
        "Bad character class",
        "A character class must be between 0 and 4096.",
    )
}

pub(crate) fn scan_char_class_not_ignored(st: &mut EngineState) -> Result<i32, Abort> {
    scan_int_up_to(
        st,
        CHAR_CLASS_LIMIT,
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
    scan_int(st)?;
    if st.scanner.cur_val < 0 || (st.scanner.cur_val > 15 && st.scanner.cur_val != 18) {
        int_range_error(
            st,
            "Bad number",
            "Since I expected to read a number between 0 and 15,",
        )?;
    }
    Ok(st.scanner.cur_val)
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
    scan_int(st)?;
    if st.scanner.cur_val < 0 || st.scanner.cur_val > st.scanner.max_reg_num {
        int_range_error(
            st,
            "Bad register code",
            "A register number must be between 0 and 32767.",
        )?;
    }
    Ok(st.scanner.cur_val)
}

pub(crate) fn scan_xetex_math_char_int(st: &mut EngineState) -> Result<i32, Abort> {
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
    Ok(st.scanner.cur_val)
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
    if st.scanner.cur_cmd != Cmd::Spacer {
        back_input(st)?;
    }
    Ok(())
}

/// Reads the token after `\primitive` and replaces it by the primitive's meaning.
pub(super) fn get_primitive_token(st: &mut EngineState) -> Result<(), Abort> {
    get_token(st)?;
    let p = st.eqtb.prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)?;
    if p != UNDEFINED_PRIMITIVE {
        st.scanner.cur_cmd = st.eqtb.eq_type(PRIM_EQTB_BASE + p);
        st.scanner.cur_chr = st.eqtb.equiv(PRIM_EQTB_BASE + p);
        st.scanner.cur_cs = PRIM_EQTB_BASE + p;
        st.scanner.cur_tok = Token::from_cs(st.scanner.cur_cs);
    } else {
        st.scanner.cur_cmd = Cmd::Relax;
        st.scanner.cur_chr = 0;
        st.scanner.cur_tok = Token::from_cs(FROZEN_RELAX);
        st.scanner.cur_cs = FROZEN_RELAX;
    }
    Ok(())
}

pub(crate) fn scan_int(st: &mut EngineState) -> Result<i32, Abort> {
    let mut ok_so_far = true;
    st.scanner.radix = 0;
    let negative = scan_signs(st)?;

    loop {
        if st.scanner.cur_tok == ALPHA_TOKEN {
            get_token(st)?;
            if !st.scanner.cur_tok.is_cs() {
                st.scanner.cur_val = st.scanner.cur_chr;
                if st.scanner.cur_cmd == Cmd::RightBrace {
                    st.scanner.align_state += 1;
                } else if st.scanner.cur_cmd < Cmd::RightBrace {
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
        } else if st.scanner.cur_cmd >= Cmd::MIN_INTERNAL && st.scanner.cur_cmd <= Cmd::MAX_INTERNAL
        {
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
            } else if st.scanner.cur_cmd != Cmd::Spacer {
                back_input(st)?;
            }
        }
        break;
    }

    if negative {
        st.scanner.cur_val = st.scanner.cur_val.wrapping_neg();
    }
    Ok(st.scanner.cur_val)
}
pub(crate) fn scan_dimen(
    st: &mut EngineState,
    mu: bool,
    inf: bool,
    shortcut: bool,
) -> Result<Scaled, Abort> {
    xetex_scan_dimen(st, mu, inf, shortcut, true)
}
/// A glue spec pointer held in a raw word.
pub(super) fn glue_spec(p: i32) -> NodePtr<GlueSpecNode> {
    NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(p))
}

/// Replaces the glue in `cur_val` by its width, releasing the glue.
pub(super) fn glue_to_width(st: &mut EngineState) {
    let spec = st.scanner.cur_glue();
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
) -> Result<Scaled, Abort> {
    let mut f: i32 = 0;
    let mut negative = false;

    st.errors.arith_error = false;
    st.scanner.cur_order = GlueOrder::Normal;

    'attach_sign: {
        if !shortcut {
            negative = scan_signs(st)?;

            if st.scanner.cur_cmd >= Cmd::MIN_INTERNAL && st.scanner.cur_cmd <= Cmd::MAX_INTERNAL {
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
                    if st.scanner.cur_cmd != Cmd::Spacer {
                        back_input(st)?;
                    }
                }
            }
        }

        if st.scanner.cur_val < 0 {
            negative = !negative;
            st.scanner.cur_val = st.scanner.cur_val.wrapping_neg();
        }

        if requires_units {
            let apply_fraction = 'units: {
                if inf && scan_keyword(st, "fil")? {
                    st.scanner.cur_order = GlueOrder::Fil;
                    while scan_keyword(st, "l")? {
                        match st.scanner.cur_order.next() {
                            Some(next) => st.scanner.cur_order = next,
                            None => {
                                print_err(st, "Illegal unit of measure (replaced by filll)");
                                st.errors.help(&["I dddon't go any higher than filll."]);
                                error(st)?;
                            }
                        }
                    }
                    break 'units true;
                }

                let save_cur_val = st.scanner.cur_val;
                get_x_non_blank(st)?;

                if st.scanner.cur_cmd >= Cmd::MIN_INTERNAL
                    && st.scanner.cur_cmd <= Cmd::MAX_INTERNAL
                {
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

    if st.errors.arith_error || st.scanner.cur_val.wrapping_abs() >= 0x4000_0000 {
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
        st.scanner.cur_val = st.scanner.cur_val.wrapping_neg();
    }
    Ok(st.scanner.cur_dimen())
}
fn attach_fraction(st: &mut EngineState, f: i32) {
    if st.scanner.cur_val >= 16384 {
        st.errors.arith_error = true;
    } else {
        st.scanner.cur_val = st.scanner.cur_val.wrapping_mul(65536).wrapping_add(f);
    }
}

pub(crate) fn scan_glue(
    st: &mut EngineState,
    level: ValueLevel,
) -> Result<NodePtr<GlueSpecNode>, Abort> {
    let mu = level == ValueLevel::Mu;
    let negative = scan_signs(st)?;

    if st.scanner.cur_cmd >= Cmd::MIN_INTERNAL && st.scanner.cur_cmd <= Cmd::MAX_INTERNAL {
        scan_something_internal(st, level, negative)?;
        if st.scanner.cur_val_level >= ValueLevel::Glue {
            if st.scanner.cur_val_level != level {
                mu_error(st)?;
            }
            return Ok(st.scanner.cur_glue());
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
            st.scanner.cur_val = st.scanner.cur_val.wrapping_neg();
        }
    }

    let gs = st.mem.new_spec(crate::mem::ZERO_GLUE)?;
    gs.set_width(&mut st.mem, st.scanner.cur_dimen());

    if scan_keyword(st, "plus")? {
        let d = scan_dimen(st, mu, true, false)?;
        gs.set_stretch(&mut st.mem, d);
        gs.set_stretch_order(&mut st.mem, st.scanner.cur_order);
    }
    if scan_keyword(st, "minus")? {
        let d = scan_dimen(st, mu, true, false)?;
        gs.set_shrink(&mut st.mem, d);
        gs.set_shrink_order(&mut st.mem, st.scanner.cur_order);
    }

    st.scanner.cur_val = gs.ptr().raw();
    Ok(st.scanner.cur_glue())
}
pub(super) fn scan_normal_glue(st: &mut EngineState) -> Result<NodePtr<GlueSpecNode>, Abort> {
    scan_glue(st, ValueLevel::Glue)
}
pub(super) fn scan_mu_glue(st: &mut EngineState) -> Result<NodePtr<GlueSpecNode>, Abort> {
    scan_glue(st, ValueLevel::Mu)
}
/// Applies `op` to the width, stretch and shrink of `spec`, in that order.
pub(super) fn map_glue(
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
        spec.set_stretch_order(&mut st.mem, GlueOrder::Normal);
    }
    if spec.shrink(&st.mem) == Scaled::ZERO {
        spec.set_shrink_order(&mut st.mem, GlueOrder::Normal);
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

pub(super) fn scan_expr(st: &mut EngineState) -> Result<(), Abort> {
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
                f = match o {
                    ValueLevel::Int => scan_int(st)?,
                    ValueLevel::Dimen => scan_dimen(st, false, false, false)?.raw(),
                    ValueLevel::Glue => scan_normal_glue(st)?.ptr().raw(),
                    _ => scan_mu_glue(st)?.ptr().raw(),
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
                if p.is_null() {
                    if st.scanner.cur_cmd != Cmd::Relax {
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
                    f = st.mem.new_spec(crate::mem::ZERO_GLUE)?.ptr().raw();
                }
            }

            match s {
                EXPR_NONE => {
                    if l >= ValueLevel::Glue && o != EXPR_NONE {
                        let term = st.mem.new_spec(glue_spec(f))?;
                        t = term.ptr().raw();
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
                            Scaled::from_raw(errors.mult_and_add(x.raw(), f, 0, MAX_HALFWORD))
                        });
                    }
                }
                EXPR_DIV => {
                    if l < ValueLevel::Glue {
                        t = st.errors.quotient(t, f);
                    } else {
                        map_glue(st, glue_spec(t), |errors, x| {
                            Scaled::from_raw(errors.quotient(x.raw(), f))
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
