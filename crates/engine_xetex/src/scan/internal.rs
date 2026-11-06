//! Internal quantities: `scan_something_internal` and the font queries behind it.

use crate::cmd::Cmd;
use crate::codes::{BoxDimenCode, FiOrElseCode, IfCode, LastItemCode, PageIntCode, ValueLevel};
use crate::consts::{
    ACTIVE_MATH_CHAR, CHAR_CLASS_LIMIT, COUNT_BASE, DEL_CODE_BASE, END_M_CODE, LEFT_SIDE,
    MATH_CODE_BASE, MAX_DIMEN, MU_GLUE, MU_SKIP_BASE, NULL_FONT, PAR_SHAPE_LOC, RIGHT_SIDE,
    SCALED_BASE, SF_CODE_BASE, SKIP_BASE, VMODE,
};
use crate::errors::{error, mu_error, Abort};
use crate::ext::{
    aat_font_get, aat_font_get_1, aat_font_get_2, aat_font_get_named, aat_font_get_named_1,
    gr_font_get_named, gr_font_get_named_1, ot_font_get, ot_font_get_1, ot_font_get_2,
    ot_font_get_3,
};
use crate::fonts::{AAT_FONT_FLAG, SPACE_CODE, SPACE_SHRINK_CODE};
use crate::input::{back_error, back_input};
use crate::main_control::{LP_CODE_BASE, RP_CODE_BASE};
use crate::node::{AnyNode, NodePtr, SaNode};
use crate::output::{
    capture_to_diagnostic, error_here_with_diagnostic, print, print_cmd_chr, print_err, print_esc,
    print_int, print_nl,
};
use crate::params::TokPar;
use crate::state::{EngineState, FontEngineHandle};
use crate::token::TokenList;
use crate::units::{EqtbPtr, FontId, MemPtr, NodeType, Scaled, StrNumber};
use std::rc::Rc;
use texyon_xetex_layout::get_cp_code;

use super::next::get_x_non_blank;
use super::numbers::{
    bad_int_error, get_primitive_token, glue_spec, glue_to_width, map_glue,
    scan_char_class_not_ignored, scan_char_num, scan_expr, scan_int, scan_math_fam_int,
    scan_mu_glue, scan_normal_glue, scan_register_num, scan_usv_num,
};
use super::{fetch_box, math_char, math_class, math_fam, scan_keyword};

const ETEX_VERSION: i32 = 2;
const XETEX_VERSION: i32 = 0;

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
pub(super) fn set_cur_val(st: &mut EngineState, value: i32, level: ValueLevel) {
    st.scanner.cur_val = value;
    st.scanner.cur_val_level = level;
}

/// The sparse-array element of type `t` for register `n`, if it exists.
pub(super) fn find_sa(st: &mut EngineState, t: ValueLevel, n: i32) -> Option<NodePtr<SaNode>> {
    st.mem.find_sa_element(&mut st.scanner, t, n)
}

/// The token list held by sparse register `sa`, null when unset.
pub(crate) fn sa_token_list(st: &EngineState, sa: Option<NodePtr<SaNode>>) -> TokenList {
    sa.map_or(TokenList::NULL, |sa| sa.sa_toks(&st.mem))
}

/// Replaces the glue in `cur_val` by a negated copy and returns the original glue.
fn negate_glue(st: &mut EngineState) -> Result<i32, Abort> {
    let old = st.scanner.cur_val;
    let spec = st.mem.new_spec(glue_spec(old))?;
    st.scanner.cur_val = spec.ptr().raw();
    map_glue(st, spec, |_, x| -x);
    Ok(old)
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

/// Reports a font query on an unsuitable font and yields `fallback`.
fn font_query_error(
    st: &mut EngineState,
    m: LastItemCode,
    f: FontId,
    msg: &str,
    fallback: i32,
) -> Result<(), Abort> {
    crate::errors::not_font_error_common(st, Cmd::LastItem, m.raw(), f, msg)?;
    st.scanner.cur_val = fallback;
    Ok(())
}

fn fetch_def_code(st: &mut EngineState, m: EqtbPtr) -> Result<(), Abort> {
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

fn fetch_xetex_def_code(st: &mut EngineState, m: EqtbPtr) -> Result<(), Abort> {
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
    } else if st.scanner.cur_cmd < Cmd::AssignToks {
        if m == 0 {
            scan_register_num(st)?;
            let n = st.scanner.cur_val;
            st.scanner.cur_val = if n < 256 {
                st.eqtb.toks(n)
            } else {
                let sa = find_sa(st, ValueLevel::Tok, n);
                sa_token_list(st, sa)
            }
            .ptr()
            .raw();
        } else {
            st.scanner.cur_val = NodePtr::<SaNode>::at(MemPtr::from_raw(m))
                .sa_ptr(&st.mem)
                .raw();
        }
        st.scanner.cur_val_level = ValueLevel::Tok;
    } else if st.scanner.cur_cmd == Cmd::AssignToks {
        if st.scanner.chr_loc() == TokPar::XetexInterCharToks.loc() {
            scan_char_class_not_ignored(st)?;
            st.scanner.cur_ptr = MemPtr::from_raw(st.scanner.cur_val);
            scan_char_class_not_ignored(st)?;
            let n = st.scanner.cur_ptr.raw() * CHAR_CLASS_LIMIT + st.scanner.cur_val;
            let sa = find_sa(st, ValueLevel::InterChar, n);
            st.scanner.cur_val = sa_token_list(st, sa).ptr().raw();
        } else {
            st.scanner.cur_val = st.eqtb.equiv(st.scanner.chr_loc());
        }
        st.scanner.cur_val_level = ValueLevel::Tok;
    } else {
        back_input(st)?;
        let f = scan_font_ident(st)?;
        set_cur_val(st, f.id_loc().raw(), ValueLevel::Ident);
    }
    Ok(())
}

fn fetch_register(st: &mut EngineState, m: i32) -> Result<(), Abort> {
    let Some(level) = ValueLevel::from_raw(m) else {
        let sa = NodePtr::<SaNode>::at(MemPtr::from_raw(m));
        let level = sa.sa_type(&st.mem);
        let value = if level < ValueLevel::Glue {
            sa.sa_int(&st.mem)
        } else {
            sa.sa_ptr(&st.mem).raw()
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
            Some(sa) => sa.sa_ptr(&st.mem).raw(),
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
                i32::from(spec.stretch_order(&st.mem).as_raw())
            } else {
                i32::from(spec.shrink_order(&st.mem).as_raw())
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
                        st.scanner.cur_val = st.scanner.cur_val / 2 + q;
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
    let mut node = (!st.mem.is_char_node(tx)).then(|| st.mem.classify(tx));
    if let Some(Ok(AnyNode::Math(math))) = node {
        if math.subtype(&st.mem) == END_M_CODE {
            tx = st.mem.predecessor(head, tx);
            node = (!st.mem.is_char_node(tx)).then(|| st.mem.classify(tx));
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
            let node_type = st.mem.type_code(tx);
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
                st.scanner.cur_val = g.glue_ptr(&st.mem).ptr().raw();
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
                            st.scanner.cur_val = spec.ptr().raw();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Scans an internal quantity of at most `level`, negated if `negative`.
pub(super) fn scan_something_internal(
    st: &mut EngineState,
    level: ValueLevel,
    negative: bool,
) -> Result<(), Abort> {
    loop {
        let m = st.scanner.cur_chr;
        match st.scanner.cur_cmd {
            Cmd::DefCode => fetch_def_code(st, st.scanner.chr_loc())?,
            Cmd::XetexDefCode => fetch_xetex_def_code(st, st.scanner.chr_loc())?,
            Cmd::ToksRegister | Cmd::AssignToks | Cmd::DefFamily | Cmd::SetFont | Cmd::DefFont => {
                fetch_token_list(st, m, level)?
            }
            Cmd::AssignInt => {
                set_cur_val(st, st.eqtb.int_at(st.scanner.chr_loc()), ValueLevel::Int)
            }
            Cmd::AssignDimen => {
                set_cur_val(st, st.eqtb.int_at(st.scanner.chr_loc()), ValueLevel::Dimen)
            }
            Cmd::AssignGlue => {
                set_cur_val(st, st.eqtb.equiv(st.scanner.chr_loc()), ValueLevel::Glue)
            }
            Cmd::AssignMuGlue => {
                set_cur_val(st, st.eqtb.equiv(st.scanner.chr_loc()), ValueLevel::Mu)
            }
            Cmd::SetAux => {
                if i32::from(st.input.cur_list.mode.abs()) != m {
                    error_here_with_diagnostic(st, "Improper ");
                    print_cmd_chr(st, Cmd::SetAux, m);
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
                    set_cur_val(st, st.input.cur_list.prev_depth().raw(), ValueLevel::Dimen);
                } else {
                    set_cur_val(st, st.input.cur_list.space_factor(), ValueLevel::Int);
                }
            }
            Cmd::SetPrevGraf => {
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
            Cmd::SetPageInt => {
                let v = match PageIntCode::from_raw(m) {
                    Some(PageIntCode::DeadCycles) => st.shipout.dead_cycles,
                    Some(PageIntCode::InteractionMode) => st.errors.interaction as i32,
                    _ => st.shipout.insert_penalties,
                };
                set_cur_val(st, v, ValueLevel::Int);
            }
            Cmd::SetPageDimen => {
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
                set_cur_val(st, v.raw(), ValueLevel::Dimen);
            }
            Cmd::SetShape => {
                let m = st.scanner.chr_loc();
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
            Cmd::SetBoxDimen => {
                scan_register_num(st)?;
                let v = match fetch_box(st, st.scanner.cur_val) {
                    None => Scaled::ZERO,
                    Some(b) if m == BoxDimenCode::Width.raw() => b.width(&st.mem),
                    Some(b) if m == BoxDimenCode::Depth.raw() => b.depth(&st.mem),
                    Some(b) => b.height(&st.mem),
                };
                set_cur_val(st, v.raw(), ValueLevel::Dimen);
            }
            Cmd::CharGiven | Cmd::MathGiven | Cmd::XetexMathGiven => {
                set_cur_val(st, m, ValueLevel::Int)
            }
            Cmd::AssignFontDimen => {
                find_font_dimen(st, false)?;
                st.fonts.set_font_info_sc(st.fonts.fmem_ptr, Scaled::ZERO);
                set_cur_val(
                    st,
                    st.fonts.font_info_sc(st.scanner.cur_val).raw(),
                    ValueLevel::Dimen,
                );
            }
            Cmd::AssignFontInt => {
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
                            LP_CODE_BASE => set_cur_val(
                                st,
                                get_cp_code(f.raw(), code, LEFT_SIDE),
                                ValueLevel::Int,
                            ),
                            RP_CODE_BASE => set_cur_val(
                                st,
                                get_cp_code(f.raw(), code, RIGHT_SIDE),
                                ValueLevel::Int,
                            ),
                            _ => {}
                        }
                    }
                }
            }
            Cmd::Register => fetch_register(st, m)?,
            Cmd::LastItem => {
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
            Cmd::IgnoreSpaces => {
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
        Cmd::DefFont => st.eqtb.cur_font(),
        Cmd::SetFont => FontId::from_raw(st.scanner.cur_chr),
        Cmd::DefFamily => {
            let m = st.scanner.chr_loc();
            scan_math_fam_int(st)?;
            FontId::from_raw(st.eqtb.equiv(m + st.scanner.cur_val))
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
    st.scanner.cur_val = f.raw();
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
