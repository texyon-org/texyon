//! Internal quantities: `scan_something_internal` and the font queries behind it.

use crate::cmd::Cmd;
use crate::codes::AuxCode;
use crate::codes::FontIntCode;
use crate::codes::PageDimenCode;
use crate::codes::{BoxDimenCode, FiOrElseCode, IfCode, LastItemCode, PageIntCode, ValueLevel};
use crate::command::Command;
use crate::command::RegisterRef;
use crate::consts::{
    ACTIVE_MATH_CHAR, CHAR_CLASS_LIMIT, DEL_CODE_BASE, MATH_CODE_BASE, MAX_DIMEN, NULL_FONT,
    PAR_SHAPE_LOC, SF_CODE_BASE, VMODE,
};
use crate::errors::OrConfusion;
use crate::errors::{mu_error, Abort};
use crate::ext::{
    aat_font_get, aat_font_get_1, aat_font_get_2, aat_font_get_named, aat_font_get_named_1,
    gr_font_get_named, gr_font_get_named_1, ot_font_get, ot_font_get_1, ot_font_get_2,
    ot_font_get_3,
};
use crate::fonts::Side;
use crate::fonts::{AAT_FONT_FLAG, SPACE_CODE, SPACE_SHRINK_CODE};
use crate::glue_store::GlueSpecId;
use crate::input::back_input;
use crate::mem::ZERO_GLUE;
use crate::node::{AnyNode, GlueKind, MathCode, SaLeaf};
use crate::params::TokPar;
use crate::state::{EngineState, FontEngineHandle};
use crate::token::TokenList;
use crate::units::CsId;
use crate::units::{EqtbPtr, FontId, FontInfoIdx, NodeType, Scaled, StrNumber};
use std::rc::Rc;

use super::next::get_x_non_blank;
use super::numbers::{
    bad_int_error, get_primitive_token, glue_spec, map_glue, scan_char_class_not_ignored,
    scan_char_num, scan_expr, scan_int, scan_math_fam_int, scan_mu_glue, scan_normal_glue,
    scan_register_num, scan_usv_num,
};
use super::{fetch_box, math_char, math_class, math_fam, scan_keyword};

const ETEX_VERSION: i32 = 2;
const XETEX_VERSION: i32 = 0;

fn missing_number_error(st: &mut EngineState) -> Result<(), Abort> {
    crate::errors::back_error(
        st,
        "Missing number, treated as zero",
        &[
            "A number should have been here; I inserted `0'.",
            "(If you can't figure out why I needed to see a number,",
            "look up `weird error' in the index to The TeXbook.)",
        ],
    )
}

/// A scanned internal quantity: TeX's `cur_val` together with its `cur_val_level`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CurVal {
    Int(i32),
    Dimen(Scaled),
    Glue(GlueSpecId),
    Mu(GlueSpecId),
    /// A font identifier, as the control sequence that names it.
    Ident(CsId),
    Tok(TokenList),
}

impl CurVal {
    /// The `cur_val_level` of this value.
    pub(crate) fn level(self) -> ValueLevel {
        match self {
            CurVal::Int(_) => ValueLevel::Int,
            CurVal::Dimen(_) => ValueLevel::Dimen,
            CurVal::Glue(_) => ValueLevel::Glue,
            CurVal::Mu(_) => ValueLevel::Mu,
            CurVal::Ident(_) => ValueLevel::Ident,
            CurVal::Tok(_) => ValueLevel::Tok,
        }
    }

    /// Decodes the word `w` that a register or expression stores at `level`.
    pub(crate) fn from_word(level: ValueLevel, w: i32) -> Result<CurVal, Abort> {
        Ok(match level {
            ValueLevel::Int => CurVal::Int(w),
            ValueLevel::Dimen => CurVal::Dimen(Scaled::from_raw(w)),
            ValueLevel::Glue => CurVal::Glue(glue_spec(w)),
            ValueLevel::Mu => CurVal::Mu(glue_spec(w)),
            ValueLevel::Ident => CurVal::Ident(CsId::from_raw(w)),
            ValueLevel::Tok => CurVal::Tok(TokenList::from_raw(w)),
            ValueLevel::InterChar | ValueLevel::Mark => {
                return Err(Abort::Confusion("cur_val_level"))
            }
        })
    }

    /// The value as a scaled word, an integer counting as that many sp.
    pub(crate) fn scaled(self) -> Result<Scaled, Abort> {
        match self {
            CurVal::Int(n) => Ok(Scaled::from_raw(n)),
            CurVal::Dimen(d) => Ok(d),
            _ => Err(Abort::Confusion("cur_val_level")),
        }
    }

    /// The glue spec of a glue or mu glue value.
    pub(crate) fn glue(self) -> Option<GlueSpecId> {
        match self {
            CurVal::Glue(spec) | CurVal::Mu(spec) => Some(spec),
            _ => None,
        }
    }

    /// Lowers the value one level: glue to its width, mu glue with an error.
    fn lower(self, st: &mut EngineState, release: bool) -> Result<CurVal, Abort> {
        Ok(match self {
            CurVal::Dimen(d) => CurVal::Int(d.raw()),
            CurVal::Glue(spec) => {
                let width = st.mem[spec].width;
                if release {
                    st.mem.delete_glue_ref(spec);
                }
                CurVal::Dimen(width)
            }
            CurVal::Mu(spec) => {
                mu_error(st)?;
                CurVal::Glue(spec)
            }
            CurVal::Int(_) | CurVal::Ident(_) | CurVal::Tok(_) => {
                return Err(Abort::Confusion("cur_val_level"))
            }
        })
    }

    /// Coerces the value down to at most `level`, releasing coerced glue if `release`.
    fn coerce(
        mut self,
        st: &mut EngineState,
        level: ValueLevel,
        release: bool,
    ) -> Result<CurVal, Abort> {
        while self.level() > level {
            self = self.lower(st, release)?;
        }
        Ok(self)
    }

    /// The negated value, glue as a fresh negated copy.
    fn negate(self, st: &mut EngineState) -> Result<CurVal, Abort> {
        Ok(match self {
            CurVal::Int(n) => CurVal::Int(-n),
            CurVal::Dimen(d) => CurVal::Dimen(-d),
            CurVal::Glue(spec) => CurVal::Glue(negated_glue(st, spec)?),
            CurVal::Mu(spec) => CurVal::Mu(negated_glue(st, spec)?),
            CurVal::Ident(_) | CurVal::Tok(_) => return Err(Abort::Confusion("cur_val_level")),
        })
    }
}

/// The sparse-array element of type `t` for register `n`, if it exists.
pub(super) fn find_sa(st: &mut EngineState, t: ValueLevel, n: i32) -> Option<SaLeaf> {
    st.mem.find_sa_element(&st.scanner, t, n)
}

/// The token list held by sparse register `sa`, null when unset.
pub(crate) fn sa_token_list(st: &EngineState, sa: Option<SaLeaf>) -> TokenList {
    match sa {
        Some(SaLeaf::Pointer(p)) => st.mem[p].toks(),
        _ => TokenList::NULL,
    }
}

/// A new spec holding `old` with all three components negated.
fn negated_glue(st: &mut EngineState, old: GlueSpecId) -> Result<GlueSpecId, Abort> {
    let spec = st.mem.new_spec(old)?;
    map_glue(st, spec, |x| -x);
    Ok(spec)
}

/// Scans a font identifier, returning the font, its OpenType/Graphite layout
/// engine and its area flag.
fn scan_font_engine(
    st: &mut EngineState,
) -> Result<(FontId, Option<Rc<FontEngineHandle>>, StrNumber), Abort> {
    let f = scan_font_ident(st)?;
    Ok((f, st.fonts.shared_layout_engine(f), st.fonts.font_area[f]))
}

fn graphite_engine(engine: &Option<Rc<FontEngineHandle>>) -> Option<&FontEngineHandle> {
    engine.as_deref().filter(|e| e.using_graphite())
}

fn opentype_engine(engine: &Option<Rc<FontEngineHandle>>) -> Option<&FontEngineHandle> {
    engine.as_deref().filter(|e| e.using_opentype())
}

/// Reports a font query on an unsuitable font.
fn font_query_error(
    st: &mut EngineState,
    m: LastItemCode,
    f: FontId,
    msg: &str,
) -> Result<(), Abort> {
    crate::errors::not_font_error_common(st, Cmd::LastItem, m.raw(), f, msg)
}

fn fetch_def_code(st: &mut EngineState, m: EqtbPtr) -> Result<i32, Abort> {
    let n = scan_usv_num(st)?;
    Ok(if m == MATH_CODE_BASE {
        let mut code = st.eqtb.math_code(n);
        if math_char(code) == ACTIVE_MATH_CHAR {
            code = 0x8000;
        } else if math_class(code) > 7 || math_fam(code) > 15 || math_char(code) > 255 {
            bad_int_error(
                st,
                "Extended mathchar used as mathchar",
                code,
                "A mathchar number must be between 0 and \"7FFF.",
                "I changed this one to zero.",
            )?;
            code = 0;
        }
        math_class(code) * 0x1000 + math_fam(code) * 0x100 + math_char(code)
    } else if m == DEL_CODE_BASE {
        let code = st.eqtb.del_code(n);
        if code >= 0x4000_0000 {
            crate::errors::error(
                st,
                "Extended delcode used as delcode",
                &[
                    "A delimiter code must be between 0 and \"7FFFFFF.",
                    "I changed this one to zero.",
                ],
            )?;
            0
        } else {
            code
        }
    } else if m < SF_CODE_BASE {
        st.eqtb.equiv(m + n)
    } else if m < MATH_CODE_BASE {
        st.eqtb.equiv(m + n) % 65536
    } else {
        st.eqtb.equiv(m + n)
    })
}

fn fetch_xetex_def_code(st: &mut EngineState, m: EqtbPtr) -> Result<i32, Abort> {
    let n = scan_usv_num(st)?;
    let (err, help1, help0) = if m == SF_CODE_BASE {
        return Ok(st.eqtb.sf_code(n) / 65536);
    } else if m == MATH_CODE_BASE {
        return Ok(st.eqtb.math_code(n));
    } else if m == MATH_CODE_BASE + 1 {
        (
            "Can't use \\Umathcode as a number (try \\Umathcodenum)",
            r"\Umathcode is for setting a mathcode from separate values;",
            r"use \Umathcodenum to access them as single values.",
        )
    } else if m == DEL_CODE_BASE {
        return Ok(st.eqtb.del_code(n));
    } else {
        (
            "Can't use \\Udelcode as a number (try \\Udelcodenum)",
            r"\Udelcode is for setting a delcode from separate values;",
            r"use \Udelcodenum to access them as single values.",
        )
    };
    crate::errors::error(st, err, &[help1, help0])?;
    Ok(0)
}

fn fetch_token_list(
    st: &mut EngineState,
    cmd: Command,
    level: ValueLevel,
) -> Result<CurVal, Abort> {
    if level != ValueLevel::Tok {
        missing_number_error(st)?;
        return Ok(CurVal::Dimen(Scaled::ZERO));
    }
    Ok(match cmd {
        Command::ToksRegister(r) if r.get().is_none() => {
            let n = scan_register_num(st)?;
            CurVal::Tok(if n < 256 {
                st.eqtb.toks(n)
            } else {
                let sa = find_sa(st, ValueLevel::Tok, n);
                sa_token_list(st, sa)
            })
        }
        Command::ToksRegister(r) => match r.get() {
            Some(sa) => CurVal::Tok(st.mem[sa].toks()),
            None => return Err(Abort::Confusion("toks register")),
        },
        Command::AssignToks(loc) if loc == TokPar::XetexInterCharToks.loc() => {
            let first = scan_char_class_not_ignored(st)?;
            let second = scan_char_class_not_ignored(st)?;
            let n = first * CHAR_CLASS_LIMIT + second;
            let sa = find_sa(st, ValueLevel::InterChar, n);
            CurVal::Tok(sa_token_list(st, sa))
        }
        Command::AssignToks(loc) => CurVal::Tok(st.eqtb.equiv_tokens(loc)),
        _ => {
            back_input(st)?;
            let f = scan_font_ident(st)?;
            CurVal::Ident(f.id_loc())
        }
    })
}

/// A sparse register's value as its raw word.
fn sa_register_word(st: &EngineState, leaf: SaLeaf) -> Result<i32, Abort> {
    match leaf {
        SaLeaf::Word(w) => Ok(st.mem[w].sa_int),
        SaLeaf::Pointer(p) => Ok(st.mem[p].sa_ptr.raw()),
        SaLeaf::MarkClass(_) => Err(Abort::Confusion("register")),
    }
}

fn fetch_register(st: &mut EngineState, r: RegisterRef) -> Result<CurVal, Abort> {
    let level = match r {
        RegisterRef::Scanned(level) => level,
        RegisterRef::Sparse(sa) => {
            let level = st.mem[sa].sa_type().get().or_confusion("sa_type")?;
            let leaf = sa.leaf(&st.mem).or_confusion("sa_type")?;
            return CurVal::from_word(level, sa_register_word(st, leaf)?);
        }
    };
    let n = scan_register_num(st)?;
    let word = if n > 255 {
        match find_sa(st, level, n) {
            None if level >= ValueLevel::Glue => ZERO_GLUE.raw(),
            None => 0,
            Some(leaf) => sa_register_word(st, leaf)?,
        }
    } else {
        let base = level.register_base().or_confusion("register")?;
        st.eqtb.equiv(base + n)
    };
    CurVal::from_word(level, word)
}

/// Integer-valued `\lastnodetype`-style and XeTeX font queries.
fn fetch_last_item_int(st: &mut EngineState, m: LastItemCode) -> Result<i32, Abort> {
    Ok(match m {
        LastItemCode::InputLineNo => st.mem.line,
        LastItemCode::Badness => st.shipout.last_badness,
        LastItemCode::ElapsedTime => {
            if st.config.deterministic_mode {
                0
            } else {
                st.shipout.get_microinterval()
            }
        }
        LastItemCode::RandomSeed => st.random.seed,
        LastItemCode::PdfShellEscape => i32::from(st.config.shell_escape_enabled),
        LastItemCode::EtexVersion => ETEX_VERSION,
        LastItemCode::XetexVersion => XETEX_VERSION,
        LastItemCode::XetexCountGlyphs => {
            let (_, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                aat_font_get(m)
            } else if let Some(engine) = engine.as_deref() {
                ot_font_get(m, engine)
            } else {
                0
            }
        }
        LastItemCode::XetexCountFeatures => {
            let (_, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                aat_font_get(m)
            } else if let Some(engine) = graphite_engine(&engine) {
                ot_font_get(m, engine)
            } else {
                0
            }
        }
        LastItemCode::XetexVariation
        | LastItemCode::XetexVariationMin
        | LastItemCode::XetexVariationMax
        | LastItemCode::XetexVariationDefault
        | LastItemCode::XetexCountVariations => {
            scan_font_ident(st)?;
            0
        }
        LastItemCode::XetexFeatureCode
        | LastItemCode::XetexIsExclusiveFeature
        | LastItemCode::XetexCountSelectors => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                let k = scan_int(st)?;
                aat_font_get_1(m, k)
            } else if let Some(engine) = graphite_engine(&engine) {
                let k = scan_int(st)?;
                ot_font_get_1(m, engine, k)
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font")?;
                -1
            }
        }
        LastItemCode::XetexSelectorCode | LastItemCode::XetexIsDefaultSelector => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                let k = scan_int(st)?;
                let kk = scan_int(st)?;
                aat_font_get_2(m, k, kk)
            } else if let Some(engine) = graphite_engine(&engine) {
                let k = scan_int(st)?;
                let kk = scan_int(st)?;
                ot_font_get_2(m, engine, k, kk)
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font")?;
                -1
            }
        }
        LastItemCode::XetexFindVariationByName => {
            let (f, _, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                crate::io::scan_file_name(st)?;
                aat_font_get_named(m)
            } else {
                font_query_error(st, m, f, "; not an AAT font")?;
                -1
            }
        }
        LastItemCode::XetexFindFeatureByName => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                crate::io::scan_file_name(st)?;
                aat_font_get_named(m)
            } else if let Some(engine) = graphite_engine(&engine) {
                let name = crate::io::scan_name_path(st)?;
                gr_font_get_named(m, engine, &name)
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font")?;
                -1
            }
        }
        LastItemCode::XetexFindSelectorByName => {
            let (f, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                let k = scan_int(st)?;
                crate::io::scan_file_name(st)?;
                aat_font_get_named_1(m, k)
            } else if let Some(engine) = graphite_engine(&engine) {
                let k = scan_int(st)?;
                let name = crate::io::scan_name_path(st)?;
                gr_font_get_named_1(m, engine, k, &name)
            } else {
                font_query_error(st, m, f, "; not an AAT or Graphite font")?;
                -1
            }
        }
        LastItemCode::XetexOtCountScripts => {
            let (_, engine, _) = scan_font_engine(st)?;
            if let Some(engine) = opentype_engine(&engine) {
                ot_font_get(m, engine)
            } else {
                0
            }
        }
        LastItemCode::XetexOtCountLanguages | LastItemCode::XetexOtScript => {
            let (f, engine, _) = scan_font_engine(st)?;
            if let Some(engine) = opentype_engine(&engine) {
                let k = scan_int(st)?;
                ot_font_get_1(m, engine, k)
            } else {
                font_query_error(st, m, f, "; not an OpenType Layout font")?;
                -1
            }
        }
        LastItemCode::XetexOtCountFeatures | LastItemCode::XetexOtLanguage => {
            let (f, engine, _) = scan_font_engine(st)?;
            if let Some(engine) = opentype_engine(&engine) {
                let k = scan_int(st)?;
                let kk = scan_int(st)?;
                ot_font_get_2(m, engine, k, kk)
            } else {
                font_query_error(st, m, f, "; not an OpenType Layout font")?;
                -1
            }
        }
        LastItemCode::XetexOtFeature => {
            let (f, engine, _) = scan_font_engine(st)?;
            if let Some(engine) = opentype_engine(&engine) {
                let k = scan_int(st)?;
                let kk = scan_int(st)?;
                let kkk = scan_int(st)?;
                ot_font_get_3(m, engine, k, kk, kkk)
            } else {
                font_query_error(st, m, f, "; not an OpenType Layout font")?;
                -1
            }
        }
        LastItemCode::XetexMapCharToGlyph => {
            let f = st.eqtb.cur_font();
            if st.fonts.is_native_font(f) {
                let c = scan_int(st)?;
                st.fonts.map_char_to_glyph(f, c)?
            } else {
                font_query_error(st, m, f, "; not a native platform font")?;
                0
            }
        }
        LastItemCode::XetexGlyphIndex => {
            let f = st.eqtb.cur_font();
            if st.fonts.is_native_font(f) {
                let name = crate::io::scan_name_path(st)?;
                st.fonts.map_glyph_to_index(f, &name)?
            } else {
                font_query_error(st, m, f, "; not a native platform font")?;
                0
            }
        }
        LastItemCode::XetexFontType => {
            let (_, engine, fa) = scan_font_engine(st)?;
            if fa == AAT_FONT_FLAG {
                1
            } else if opentype_engine(&engine).is_some() {
                2
            } else if graphite_engine(&engine).is_some() {
                3
            } else {
                0
            }
        }
        LastItemCode::XetexFirstChar | LastItemCode::XetexLastChar => {
            let f = scan_font_ident(st)?;
            let first = m == LastItemCode::XetexFirstChar;
            if st.fonts.is_native_font(f) {
                st.fonts.get_font_char_range(f, i32::from(first))?
            } else if first {
                i32::from(st.fonts.font_bc[f])
            } else {
                i32::from(st.fonts.font_ec[f])
            }
        }
        LastItemCode::PdfLastXPos => st.shipout.pdf_last_x_pos,
        LastItemCode::PdfLastYPos => st.shipout.pdf_last_y_pos,
        LastItemCode::XetexPdfPageCount => {
            let name = crate::io::scan_name_path(st)?;
            crate::pic::count_pdf_file_pages_named(&name)?
        }
        LastItemCode::CurrentGroupLevel => i32::from(st.save_stack.cur_level) - 1,
        LastItemCode::CurrentGroupType => st.save_stack.cur_group as i32,
        LastItemCode::CurrentIfLevel => i32::try_from(st.scanner.conds.depth()).unwrap_or(i32::MAX),
        LastItemCode::CurrentIfType => {
            if st.scanner.conds.top().is_none() {
                0
            } else if i32::from(st.scanner.cur_if) < IfCode::UNLESS {
                i32::from(st.scanner.cur_if) + 1
            } else {
                -(i32::from(st.scanner.cur_if) - 31)
            }
        }
        LastItemCode::CurrentIfBranch => match st.scanner.if_limit.get() {
            Some(FiOrElseCode::Or | FiOrElseCode::Else) => 1,
            Some(FiOrElseCode::Fi) => -1,
            _ => 0,
        },
        LastItemCode::GlueStretchOrder | LastItemCode::GlueShrinkOrder => {
            let spec = scan_normal_glue(st)?;
            let order = if m == LastItemCode::GlueStretchOrder {
                st.mem[spec].stretch_order
            } else {
                st.mem[spec].shrink_order
            };
            st.mem.delete_glue_ref(spec);
            i32::from(order.raw())
        }
        _ => return Err(Abort::Confusion("last_item")),
    })
}

fn fetch_last_item_dimen(st: &mut EngineState, m: LastItemCode) -> Result<Scaled, Abort> {
    Ok(match m {
        LastItemCode::XetexGlyphBounds => {
            let f = st.eqtb.cur_font();
            if st.fonts.is_native_font(f) {
                let edge = scan_int(st)?;
                if !(1..=4).contains(&edge) {
                    crate::errors::error(
                        st,
                        (
                            "\\\\XeTeXglyphbounds requires an edge index from 1 to 4;",
                            crate::output::Nl("I don't know anything about edge "),
                            edge,
                        ),
                        &[],
                    )?;
                    Scaled::ZERO
                } else {
                    let glyph = scan_int(st)?;
                    st.fonts.get_glyph_bounds(f, edge, glyph)?
                }
            } else {
                font_query_error(st, m, f, "; not a native platform font")?;
                Scaled::ZERO
            }
        }
        LastItemCode::FontCharWd
        | LastItemCode::FontCharHt
        | LastItemCode::FontCharDp
        | LastItemCode::FontCharIc => {
            let f = scan_font_ident(st)?;
            let c = scan_usv_num(st)?;
            if st.fonts.is_native_font(f) {
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
            }
        }
        LastItemCode::ParShapeLength
        | LastItemCode::ParShapeIndent
        | LastItemCode::ParShapeDimen => {
            let mut q = match m {
                LastItemCode::ParShapeLength => 0,
                LastItemCode::ParShapeIndent => 1,
                _ => 2,
            };
            let mut n = scan_int(st)?;
            match st.eqtb.par_shape() {
                Some(shape) if n > 0 => {
                    if q == 2 {
                        q = n % 2;
                        n = n / 2 + q;
                    }
                    let line = shape.line(&st.mem, n.min(st.mem[shape].lines));
                    if q == 0 {
                        line.width
                    } else {
                        line.indent
                    }
                }
                _ => Scaled::ZERO,
            }
        }
        LastItemCode::GlueStretch | LastItemCode::GlueShrink => {
            let spec = scan_normal_glue(st)?;
            let d = if m == LastItemCode::GlueStretch {
                st.mem[spec].stretch
            } else {
                st.mem[spec].shrink
            };
            st.mem.delete_glue_ref(spec);
            d
        }
        _ => return Err(Abort::Confusion("last_item")),
    })
}

/// `\lastpenalty`, `\lastkern`, `\lastskip` and `\lastnodetype`.
fn fetch_last_node_item(st: &EngineState, code: LastItemCode) -> CurVal {
    let head = st.input.cur_list.head;
    let mode = st.input.cur_list.mode;
    // The list head is no node, as TeX's hi-mem head counted as a character.
    let item = |tx| (tx != head && !st.mem.is_char_node(tx)).then(|| st.mem.classify(tx));
    let mut tx = st.input.cur_list.tail;
    let mut node = item(tx);
    if let Some(Ok(AnyNode::Math(math))) = node {
        if st.mem[math].subtype == MathCode::EndM {
            tx = st.mem.predecessor(head, tx);
            node = item(tx);
        }
    }
    let node = node.filter(|_| mode != 0);
    let at_page_head = node.is_none() && mode == VMODE && tx == head;
    match code {
        LastItemCode::LastNodeType => CurVal::Int(if node.is_some() {
            let node_type = st.mem.type_code(tx);
            if node_type <= NodeType::Unset.to_u16() {
                i32::from(node_type) + 1
            } else {
                i32::from(NodeType::Unset.to_u16()) + 2
            }
        } else if at_page_head {
            st.shipout.last_node_type
        } else if tx == head || mode == 0 {
            -1
        } else {
            0
        }),
        LastItemCode::LastPenalty => CurVal::Int(match node {
            Some(Ok(AnyNode::Penalty(p))) => st.mem[p].penalty,
            None if at_page_head => st.shipout.last_penalty,
            _ => 0,
        }),
        LastItemCode::LastKern => CurVal::Dimen(match node {
            Some(Ok(AnyNode::Kern(k))) => st.mem[k].width,
            None if at_page_head => st.shipout.last_kern,
            _ => Scaled::ZERO,
        }),
        _ => match node {
            Some(Ok(AnyNode::Glue(g))) if st.mem[g].subtype == GlueKind::Mu => {
                CurVal::Mu(st.mem[g].glue_ptr)
            }
            Some(Ok(AnyNode::Glue(g))) => CurVal::Glue(st.mem[g].glue_ptr),
            None if at_page_head => CurVal::Glue(st.shipout.last_glue.unwrap_or(ZERO_GLUE)),
            _ => CurVal::Glue(ZERO_GLUE),
        },
    }
}

/// The value of an `\numexpr`-style or glue conversion `last_item`, coerced to at most `level`.
fn fetch_expr_item(
    st: &mut EngineState,
    code: LastItemCode,
    level: ValueLevel,
    negative: bool,
) -> Result<CurVal, Abort> {
    let v = match code {
        LastItemCode::MuToGlue => CurVal::Glue(scan_mu_glue(st)?),
        LastItemCode::GlueToMu => CurVal::Mu(scan_normal_glue(st)?),
        LastItemCode::NumExpr => scan_expr(st, ValueLevel::Int)?,
        LastItemCode::DimExpr => scan_expr(st, ValueLevel::Dimen)?,
        LastItemCode::GlueExpr => scan_expr(st, ValueLevel::Glue)?,
        _ => scan_expr(st, ValueLevel::Mu)?,
    };
    let v = v.coerce(st, level, true)?;
    if !negative {
        return Ok(v);
    }
    let negated = v.negate(st)?;
    if let Some(old) = v.glue() {
        st.mem.delete_glue_ref(old);
    }
    Ok(negated)
}

/// Scans an internal quantity of at most `level`, negated if `negative`.
pub(super) fn scan_something_internal(
    st: &mut EngineState,
    level: ValueLevel,
    negative: bool,
) -> Result<CurVal, Abort> {
    // An error leaves zero, as a dimension unless a token list was allowed.
    let zero = if level != ValueLevel::Tok {
        CurVal::Dimen(Scaled::ZERO)
    } else {
        CurVal::Int(0)
    };
    let v = loop {
        break match st.scanner.cur {
            Command::DefCode(base) => CurVal::Int(fetch_def_code(st, base)?),
            Command::XetexDefCode(base) => CurVal::Int(fetch_xetex_def_code(st, base)?),
            cmd @ (Command::ToksRegister(_)
            | Command::AssignToks(_)
            | Command::DefFamily(_)
            | Command::SetFont(_)
            | Command::DefFont) => fetch_token_list(st, cmd, level)?,
            Command::AssignInt(loc) => CurVal::Int(st.eqtb.int_at(loc)),
            Command::AssignDimen(loc) => CurVal::Dimen(Scaled::from_raw(st.eqtb.int_at(loc))),
            Command::AssignGlue(loc) => CurVal::Glue(glue_spec(st.eqtb.equiv(loc))),
            Command::AssignMuGlue(loc) => CurVal::Mu(glue_spec(st.eqtb.equiv(loc))),
            Command::SetAux(code) => {
                if st.input.cur_list.mode.abs() != code.mode() {
                    crate::errors::error(
                        st,
                        ("Improper ", crate::output::CmdChr(Cmd::SetAux, code.raw())),
                        &[
                            r"You can refer to \spacefactor only in horizontal mode;",
                            r"you can refer to \prevdepth only in vertical mode; and",
                            r"neither of these is meaningful inside \write. So",
                            "I'm forgetting what you said and using zero instead.",
                        ],
                    )?;
                    zero
                } else if code == AuxCode::PrevDepth {
                    CurVal::Dimen(st.input.cur_list.prev_depth())
                } else {
                    CurVal::Int(st.input.cur_list.space_factor())
                }
            }
            Command::SetPrevGraf => {
                if st.input.cur_list.mode == 0 {
                    CurVal::Int(0)
                } else {
                    let np = st.input.nest.len();
                    st.input.nest[np] = st.input.cur_list;
                    let mut p = np;
                    while st.input.nest[p].mode.abs() != VMODE {
                        p -= 1;
                    }
                    CurVal::Int(st.input.nest[p].prev_graf)
                }
            }
            Command::SetPageInt(code) => CurVal::Int(match code {
                PageIntCode::DeadCycles => st.shipout.dead_cycles,
                PageIntCode::InteractionMode => st.errors.interaction as i32,
                PageIntCode::InsertPenalties => st.shipout.insert_penalties,
            }),
            Command::SetPageDimen(code) => CurVal::Dimen(
                if st.shipout.page_contents == crate::state::PageContents::Empty
                    && !st.shipout.output_active
                {
                    if code == PageDimenCode::Goal {
                        MAX_DIMEN
                    } else {
                        Scaled::ZERO
                    }
                } else {
                    st.shipout.page_so_far[code.index()]
                },
            ),
            Command::SetShape(m) => CurVal::Int(if m > PAR_SHAPE_LOC {
                let n = scan_int(st)?;
                match st.eqtb.penalties(m) {
                    Some(penalties) if n >= 0 => {
                        let k = n.min(st.mem[penalties].count);
                        penalties.penalty(&st.mem, k)
                    }
                    _ => 0,
                }
            } else {
                st.eqtb.par_shape().map_or(0, |s| st.mem[s].lines)
            }),
            Command::SetBoxDimen(code) => {
                let n = scan_register_num(st)?;
                CurVal::Dimen(match (fetch_box(st, n), code) {
                    (None, _) => Scaled::ZERO,
                    (Some(b), BoxDimenCode::Width) => st.mem[b].width,
                    (Some(b), BoxDimenCode::Depth) => st.mem[b].depth,
                    (Some(b), BoxDimenCode::Height) => st.mem[b].height,
                })
            }
            Command::CharGiven(c) => CurVal::Int(c.raw()),
            Command::MathGiven(code) | Command::XetexMathGiven(code) => CurVal::Int(code),
            Command::AssignFontDimen => {
                let k = find_font_dimen(st, false)?;
                st.fonts.set_font_info_sc(st.fonts.fmem_ptr, Scaled::ZERO);
                CurVal::Dimen(st.fonts.font_info_sc(k))
            }
            Command::AssignFontInt(code) => {
                let f = scan_font_ident(st)?;
                let side = match code {
                    FontIntCode::HyphenChar => break CurVal::Int(st.fonts.hyphen_char[f]),
                    FontIntCode::SkewChar => break CurVal::Int(st.fonts.skew_char[f]),
                    FontIntCode::LpCode => Side::Left,
                    FontIntCode::RpCode => Side::Right,
                };
                let c = if st.fonts.is_native_font(f) {
                    scan_glyph_number(st, f)?
                } else {
                    scan_char_num(st)?
                } as u32;
                CurVal::Int(st.fonts.cp_codes.get(f, side, c))
            }
            Command::Register(r) => fetch_register(st, r.get())?,
            Command::LastItem(code) => {
                if code < LastItemCode::InputLineNo {
                    fetch_last_node_item(st, code)
                } else if code < LastItemCode::XetexGlyphBounds {
                    CurVal::Int(fetch_last_item_int(st, code)?)
                } else if code < LastItemCode::MuToGlue {
                    CurVal::Dimen(fetch_last_item_dimen(st, code)?)
                } else {
                    return fetch_expr_item(st, code, level, negative);
                }
            }
            Command::IgnoreSpaces { primitive: true } => {
                get_primitive_token(st)?;
                continue;
            }
            _ => {
                crate::errors::error(
                    st,
                    (
                        "You can't use `",
                        crate::output::CmdChr::of(st.scanner.cur),
                        "' after ",
                        crate::output::Esc("the"),
                    ),
                    &["I'm forgetting what you said and using zero instead."],
                )?;
                zero
            }
        };
    };

    let v = v.coerce(st, level, false)?;
    if negative {
        v.negate(st)
    } else {
        if let Some(spec) = v.glue() {
            let refs = st.mem[spec].ref_count;
            st.mem[spec].ref_count = refs + 1;
        }
        Ok(v)
    }
}

/// Scans a font identifier.
pub(crate) fn scan_font_ident(st: &mut EngineState) -> Result<FontId, Abort> {
    get_x_non_blank(st)?;
    let f = match st.scanner.cur {
        Command::DefFont => st.eqtb.cur_font(),
        Command::SetFont(f) => f,
        Command::DefFamily(m) => {
            let fam = scan_math_fam_int(st)?;
            st.eqtb.font_equiv(m + fam)
        }
        _ => {
            crate::errors::back_error(
                st,
                "Missing font identifier",
                &[
                    "I was looking for a control sequence whose",
                    r"current meaning has been defined by \font.",
                ],
            )?;
            NULL_FONT
        }
    };
    Ok(f)
}

/// The `font_info` index of the `\fontdimen` being scanned.
pub(crate) fn find_font_dimen(st: &mut EngineState, writing: bool) -> Result<FontInfoIdx, Abort> {
    let n = scan_int(st)?;
    let f = scan_font_ident(st)?;

    let k = if n <= 0 {
        st.fonts.fmem_ptr
    } else {
        if writing && (SPACE_CODE..=SPACE_SHRINK_CODE).contains(&n) {
            if let Some(glue) = st.fonts.font_glue[f].take() {
                st.mem.delete_glue_ref(glue);
            }
        }

        if n <= st.fonts.font_params[f] {
            st.fonts.param_base[f] + n
        } else if f < st.fonts.font_ptr {
            st.fonts.fmem_ptr
        } else {
            // Only the most recently loaded font can grow its parameter area.
            loop {
                if st.fonts.fmem_ptr == st.fonts.font_mem_size {
                    return Err(Abort::Overflow {
                        capacity: crate::errors::Capacity::FontMemory,
                        n: st.fonts.font_mem_size.raw(),
                    });
                }
                st.fonts.set_font_info_sc(st.fonts.fmem_ptr, Scaled::ZERO);
                st.fonts.fmem_ptr += 1;
                st.fonts.font_params[f] += 1;
                if n == st.fonts.font_params[f] {
                    break;
                }
            }
            st.fonts.fmem_ptr - 1
        }
    };

    if k == st.fonts.fmem_ptr {
        crate::errors::error(
            st,
            (
                "Font ",
                crate::output::Esc(st.eqtb.font_id_text(f)),
                " has only ",
                st.fonts.font_params[f],
                " fontdimen parameters",
            ),
            &[
                "To increase the number of font parameters, you must",
                r"use \fontdimen immediately after the \font is loaded.",
            ],
        )?;
    }
    Ok(k)
}

pub(crate) fn scan_glyph_number(st: &mut EngineState, f: FontId) -> Result<i32, Abort> {
    let v = if scan_keyword(st, "/")? {
        let name = crate::io::scan_name_path(st)?;
        st.fonts.map_glyph_to_index(f, &name)?
    } else if scan_keyword(st, "u")? {
        let c = scan_char_num(st)?;
        st.fonts.map_char_to_glyph(f, c)?
    } else {
        return scan_int(st);
    };
    Ok(v)
}
