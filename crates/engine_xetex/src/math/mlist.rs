//! Typesetting math formulas: `mlist_to_hlist` and the `make_*` noad handlers.

use crate::codes::LimitCode;
use crate::consts::MIN_HALFWORD;
use crate::consts::{
    EMPTY, INFINITE_DIMEN, INF_PENALTY, MATH_CHAR, MATH_TEXT_CHAR, MAX_DIMEN, NULL_FONT, SUB_BOX,
    SUB_MLIST, TEXT_SIZE,
};
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::fonts::{
    CharInfo, CharTag, LigChar, LigKernAction, LigKernIdx, LigOp, AAT_FONT_FLAG, OTGR_FONT_FLAG,
    SPACE_CODE, X_HEIGHT_CODE,
};
use crate::glue_store::GlueSpecId;
use crate::mem::{TEMP_HEAD, TEX_NULL, ZERO_GLUE};
use crate::mem_types::ListStateRecord;
use crate::node::MathWord;
use crate::node::{
    AccentNoad, AnyNoad, AnyNode, BoxNode, ChoiceNode, FenceNoad, FractionNoad, GlueKind, GlueNode,
    GlyphNode, KernNode, KernSubtype, MathAccentKind, MathSlot, Noad, NoadVariant, NodePtr,
    RadicalNoad, RuleDimen, RuleNode, StyleNode, View, WhatsitSubtype, WhatsitVariant,
};
use crate::packaging::PackSpec;
use crate::params::{DimenPar, GluePar, IntPar};
use crate::scaled_math::{mult_and_add, x_over_n, xn_over_d};
use crate::state::{EngineState, EqtbState, FontState, MemState, ScannerState};
use crate::units::MemIdx;
use crate::units::{FontId, GlueOrder, MemPtr, NodeType, Scaled};

use super::delimiters::{build_opentype_assembly, rebox, var_delimiter};
use super::scripts::make_scripts;
use super::{MathStyle, DEFAULT_CODE};

const DISPLAYOPERATORMINHEIGHT: i32 = 3;
const ACCENTBASEHEIGHT: i32 = 6;

const STACKGAPMIN: i32 = 26;
const STACKDISPLAYSTYLEGAPMIN: i32 = 27;
const FRACTIONNUMERATORGAPMIN: i32 = 36;
const FRACTIONNUMDISPLAYSTYLEGAPMIN: i32 = 37;
const FRACTIONDENOMINATORGAPMIN: i32 = 39;
const FRACTIONDENOMDISPLAYSTYLEGAPMIN: i32 = 40;
const RADICALVERTICALGAP: i32 = 49;
const RADICALDISPLAYSTYLEVERTICALGAP: i32 = 50;
const RADICALRULETHICKNESS: i32 = 51;

impl MemState {
    pub fn fraction_rule(&mut self, t: Scaled) -> Result<NodePtr<RuleNode>, Abort> {
        let p = NodePtr::<RuleNode>::alloc(self)?;
        let node = &mut self[p];
        node.width = RuleDimen::Running.into();
        node.height = RuleDimen::Set(t).into();
        node.depth = RuleDimen::Set(Scaled::ZERO).into();
        Ok(p)
    }

    pub fn overbar(&mut self, b: NodePtr<BoxNode>, k: Scaled, t: Scaled) -> Result<MemPtr, Abort> {
        let p = self.new_kern(k)?;
        self[p].link = b.ptr();
        let q = self.fraction_rule(t)?;
        self[q].link = p.ptr();
        let r = self.new_kern(t)?;
        self[r].link = q.ptr();
        Ok(r.ptr())
    }
}

/// The integer and fractional parts of the mu factor `m`, as `math_glue` splits it.
fn mu_parts(m: Scaled) -> (i32, i32) {
    let split = x_over_n(m.raw(), 65_536);
    if split.remainder < 0 {
        (split.quotient - 1, split.remainder + 65_536)
    } else {
        (split.quotient, split.remainder)
    }
}

fn mu_mult((n, f): (i32, i32), v: Scaled) -> Scaled {
    let xnd = xn_over_d(v, f, 65_536).quotient;
    mult_and_add(n, v, xnd, MAX_DIMEN).value
}

fn math_glue(st: &mut EngineState, spec: GlueSpecId, m: Scaled) -> Result<GlueSpecId, Abort> {
    let mu = mu_parts(m);
    let stretch_order = st.mem[spec].stretch_order;
    let shrink_order = st.mem[spec].shrink_order;
    let width = mu_mult(mu, st.mem[spec].width);
    let mut stretch = st.mem[spec].stretch;
    if stretch_order == GlueOrder::Normal {
        stretch = mu_mult(mu, stretch);
    }
    let mut shrink = st.mem[spec].shrink;
    if shrink_order == GlueOrder::Normal {
        shrink = mu_mult(mu, shrink);
    }

    let p = st.mem.glue.alloc()?;
    let node = &mut st.mem[p];
    node.width = width;
    node.stretch_order = stretch_order;
    node.stretch = stretch;
    node.shrink_order = shrink_order;
    node.shrink = shrink;
    Ok(p)
}

fn math_kern(st: &mut EngineState, p: NodePtr<KernNode>, m: Scaled) {
    if st.mem[p].subtype == KernSubtype::MuGlue {
        let mu = mu_parts(m);
        let width = mu_mult(mu, st.mem[p].width);
        let node = &mut st.mem[p];
        node.width = width;
        node.subtype = KernSubtype::Explicit.into();
    }
}

impl ListStateRecord {
    pub fn flush_math(&mut self, mem: &mut MemState) -> Result<(), Abort> {
        mem.flush_node_list(self.head.link(mem))?;
        if let Some(n) = self.incompleat_noad()? {
            mem.flush_node_list(n.ptr())?;
        }
        self.head.set_link(mem, None);
        self.tail = self.head;
        self.set_incompleat_noad(None)
    }
}

pub(super) fn clean_box(
    st: &mut EngineState,
    p: MathWord,
    s: MathStyle,
) -> Result<NodePtr<BoxNode>, Abort> {
    let q = match p.math_type {
        MATH_CHAR => {
            let noad = st.mem.new_noad()?;
            st.mem[noad].nucleus = p;
            mlist_to_hlist(st, noad.ptr(), s, false)?
        }
        SUB_BOX => p.info(),
        SUB_MLIST => mlist_to_hlist(st, p.info(), s, false)?,
        _ => st.mem.new_null_box()?.ptr(),
    };

    let lone_box = match q {
        Some(idx) if !st.mem.is_char_node(idx) && idx.link(&st.mem).is_none() => {
            match st.mem.classify_box(idx) {
                Ok(Some(b)) if st.mem[b].shift_amount == Scaled::ZERO => Some(b),
                _ => None,
            }
        }
        _ => None,
    };
    let x_box = match lone_box {
        Some(b) => b,
        None => crate::packaging::hpack(st, q, PackSpec::NATURAL)?,
    };

    let head = st.mem[x_box].list_ptr;
    if let Some(h) = st.mem.as_char(head) {
        let r = h.idx().link(&st.mem);
        if let Some(r_idx) = r
            && r_idx.link(&st.mem).is_none()
            && matches!(st.mem.classify_non_char(r), Some(AnyNode::Kern(_)))
        {
            st.mem.free_node(r_idx, KernNode::SIZE);
            h.idx().set_link(&mut st.mem, None);
        }
    }
    Ok(x_box)
}

/// The math unit at `style`: an eighteenth of the quad of family 2.
pub(super) fn math_mu(st: &mut EngineState, style: MathStyle) -> Scaled {
    let mq = st.fonts.math_quad(&st.eqtb, style.size());
    x_over_n(mq, 18).quotient
}

/// What `fetch` found for a math character: its font, code and `char_info`,
/// which is null when the character is missing or the font is native.
#[derive(Copy, Clone)]
pub(super) struct Fetched {
    pub f: FontId,
    pub c: i32,
    /// The character of a TFM font, `None` for a native font or a missing character.
    pub i: Option<CharInfo>,
}

enum FetchError {
    UndefinedFont { size: i32, fam: i32, c: i32 },
    CharMissing { f: FontId, c: i32 },
}

impl MemState {
    fn fetch(
        &mut self,
        fonts: &FontState,
        eqtb: &EqtbState,
        scanner: &mut ScannerState,
        a: MathSlot,
        size: i32,
    ) -> (Fetched, Option<FetchError>) {
        let fam = self[a].fam();
        let f = eqtb.math_font(fam + size);
        scanner.cur_f = f;
        let c = self[a].full_char();
        let mut fetched = Fetched { f, c, i: None };

        if f == NULL_FONT {
            self[a].math_type = EMPTY;
            return (fetched, Some(FetchError::UndefinedFont { size, fam, c }));
        }

        let area = fonts.font_area[f];
        if area != AAT_FONT_FLAG && area != OTGR_FONT_FLAG {
            if c >= i32::from(fonts.font_bc[f]) && c <= i32::from(fonts.font_ec[f]) {
                fetched.i = fonts.char_info(f, c);
            }
            if fetched.i.is_none() {
                self[a].math_type = EMPTY;
                return (fetched, Some(FetchError::CharMissing { f, c }));
            }
        }
        (fetched, None)
    }
}

/// Look up math character `a`, reporting a missing font or character. Also sets `cur_f`.
pub(super) fn fetch(st: &mut EngineState, a: MathSlot, size: i32) -> Result<Fetched, Abort> {
    let (fetched, error) = st.mem.fetch(&st.fonts, &st.eqtb, &mut st.scanner, a, size);
    match error {
        None => {}
        Some(FetchError::UndefinedFont { size, fam, c }) => crate::errors::error(
            st,
            (
                crate::output::FamFont(size),
                ' ',
                fam,
                " is undefined (character ",
                crate::units::StrNumber::single_char(c),
                ')',
            ),
            &[
                "Somewhere in the math formula just ended, you used the",
                "stated character from an undefined font family. For example,",
                r"plain TeX doesn't allow \it or \sl in subscripts. Proceed,",
                "and I'll try to forget that I needed that character.",
            ],
        )?,
        Some(FetchError::CharMissing { f, c }) => crate::output::char_warning(st, f, c)?,
    }
    Ok(fetched)
}

impl MemState {
    fn set_nucleus_box(&mut self, q: NodePtr<Noad>, b: NodePtr<BoxNode>) {
        self[q].nucleus.set_info(b.ptr());
        self[q].nucleus.math_type = SUB_BOX;
    }
}

fn make_over(st: &mut EngineState, q: NodePtr<Noad>, style: MathStyle) -> Result<(), Abort> {
    let x = clean_box(st, st.mem[q].nucleus, style.cramped())?;
    let rule_thickness = st.fonts.default_rule_thickness(&st.eqtb, style.size());
    let p = st.mem.overbar(x, rule_thickness * 3, rule_thickness)?;
    let x = crate::packaging::vpackage(st, p, PackSpec::NATURAL, MAX_DIMEN)?;
    st.mem.set_nucleus_box(q, x);
    Ok(())
}

fn make_under(st: &mut EngineState, q: NodePtr<Noad>, style: MathStyle) -> Result<(), Abort> {
    let x = clean_box(st, st.mem[q].nucleus, style)?;
    let rule_thickness = st.fonts.default_rule_thickness(&st.eqtb, style.size());
    let kern = st.mem.new_kern(rule_thickness * 3)?;
    st.mem[x].link = kern.ptr();
    let rule = st.mem.fraction_rule(rule_thickness)?;
    st.mem[kern].link = rule.ptr();

    let y = crate::packaging::vpackage(st, x.ptr(), PackSpec::NATURAL, MAX_DIMEN)?;
    let mem = &mut st.mem;
    let delta = mem[y].height + mem[y].depth + rule_thickness;
    let height = mem[x].height;
    let node = &mut mem[y];
    node.height = height;
    node.depth = delta - height;
    mem.set_nucleus_box(q, y);
    Ok(())
}

fn make_vcenter(st: &mut EngineState, q: NodePtr<Noad>, style: MathStyle) -> Result<(), Abort> {
    let axis_height = st.fonts.axis_height(&st.eqtb, style.size());
    let mem = &mut st.mem;
    let v = match mem[q].nucleus.info().map(|i| mem.classify_box(i)) {
        Some(Ok(Some(b))) if mem[b].is_vlist() => b,
        _ => return Err(Abort::Confusion("vcenter")),
    };
    let delta = mem[v].height + mem[v].depth;
    let node = &mut mem[v];
    node.height = axis_height + delta.half();
    node.depth = delta - node.height;
    Ok(())
}

fn make_radical(
    st: &mut EngineState,
    q: NodePtr<RadicalNoad>,
    style: MathStyle,
) -> Result<(), Abort> {
    let f = st
        .eqtb
        .math_font(st.mem[q].left_delimiter.small().0 + style.size());
    let is_ot_math = st.fonts.font_is_ot_math(f);
    let rule_thickness = if is_ot_math {
        st.fonts.get_ot_math_constant(f, RADICALRULETHICKNESS)
    } else {
        st.fonts.default_rule_thickness(&st.eqtb, style.size())
    };

    let clean_style = style.cramped();
    let x = clean_box(st, st.mem[q].base.nucleus, clean_style)?;
    let cur_size = style.size();
    let mut clr = if is_ot_math {
        if style < MathStyle::TEXT {
            st.fonts
                .get_ot_math_constant(f, RADICALDISPLAYSTYLEVERTICALGAP)
        } else {
            st.fonts.get_ot_math_constant(f, RADICALVERTICALGAP)
        }
    } else if style < MathStyle::TEXT {
        rule_thickness + st.fonts.math_x_height(&st.eqtb, cur_size).abs() / 4
    } else {
        rule_thickness + rule_thickness.abs() / 4
    };

    let delimiter_height = st.mem[x].height + st.mem[x].depth + clr + rule_thickness;
    let y = var_delimiter(st, st.mem[q].left_delimiter, cur_size, delimiter_height)?;
    let mem = &mut st.mem;
    if is_ot_math {
        let node = &mut mem[y];
        node.depth = node.depth + node.height - rule_thickness;
        node.height = rule_thickness;
    }
    let delta = mem[y].depth - (mem[x].height + mem[x].depth + clr);
    if delta > Scaled::ZERO {
        clr += delta.half();
    }
    mem[y].shift_amount = -(mem[x].height + clr);
    let p = mem.overbar(x, clr, mem[y].height)?;
    let overbar_box = crate::packaging::vpackage(st, p, PackSpec::NATURAL, MAX_DIMEN)?;
    st.mem[y].link = overbar_box.ptr();
    let hbox = crate::packaging::hpack(st, y.ptr(), PackSpec::NATURAL)?;
    st.mem.set_nucleus_box(q.as_noad(), hbox);
    Ok(())
}

struct FractionAssembly {
    x: NodePtr<BoxNode>,
    z: NodePtr<BoxNode>,
    rule_thickness: Scaled,
    shift_up: Scaled,
    shift_down: Scaled,
    delta: Scaled,
    axis_height: Scaled,
}

impl MemState {
    fn make_fraction(&mut self, assembly: FractionAssembly) -> Result<NodePtr<BoxNode>, Abort> {
        let vb = self.new_null_vbox()?;
        let xb = assembly.x;
        let zb = assembly.z;
        let (x, z_depth) = (self[xb], self[zb].depth);
        let node = &mut self[vb];
        node.height = assembly.shift_up + x.height;
        node.depth = z_depth + assembly.shift_down;
        node.width = x.width;

        let p: NodePtr<KernNode>;
        if assembly.rule_thickness == Scaled::ZERO {
            let kw = (assembly.shift_up - self[xb].depth) - (self[zb].height - assembly.shift_down);
            p = self.new_kern(kw)?;
            self[p].link = assembly.z.ptr();
        } else {
            let y = self.fraction_rule(assembly.rule_thickness)?;
            let p1w =
                (assembly.axis_height - assembly.delta) - (self[zb].height - assembly.shift_down);
            let p1 = self.new_kern(p1w)?;
            self[y].link = p1.ptr();
            self[p1].link = assembly.z.ptr();
            let pw = (assembly.shift_up - self[xb].depth) - (assembly.axis_height + assembly.delta);
            p = self.new_kern(pw)?;
            self[p].link = y.ptr();
        }

        self[xb].link = p.ptr();
        self[vb].list_ptr = assembly.x.ptr();
        Ok(vb)
    }

    pub(super) fn glyph_whatsit(&self, p: MemPtr) -> Option<NodePtr<GlyphNode>> {
        match self.classify_non_char(p)? {
            AnyNode::Whatsit(w) => match w.variant(self)? {
                WhatsitVariant::Glyph(glyph) => Some(glyph),
                _ => None,
            },
            _ => None,
        }
    }
}

fn make_fraction(
    st: &mut EngineState,
    q: NodePtr<FractionNoad>,
    style: MathStyle,
) -> Result<(), Abort> {
    if st.mem[q].thickness == DEFAULT_CODE {
        let drt = st.fonts.default_rule_thickness(&st.eqtb, style.size());
        st.mem[q].thickness = drt;
    }

    let mut x = clean_box(st, st.mem[q].numerator, style.num())?;
    let mut z = clean_box(st, st.mem[q].denominator, style.denom())?;

    if st.mem[x].width < st.mem[z].width {
        let w = st.mem[z].width;
        x = rebox(st, x, w)?;
    } else {
        let w = st.mem[x].width;
        z = rebox(st, z, w)?;
    }

    let cur_size = style.size();
    let cur_f = st.scanner.cur_f;
    let (mut shift_up, mut shift_down) = if style < MathStyle::TEXT {
        (
            st.fonts.num1(&st.eqtb, cur_size),
            st.fonts.denom1(&st.eqtb, cur_size),
        )
    } else {
        (
            if st.mem[q].thickness != Scaled::ZERO {
                st.fonts.num2(&st.eqtb, cur_size)
            } else {
                st.fonts.num3(&st.eqtb, cur_size)
            },
            st.fonts.denom2(&st.eqtb, cur_size),
        )
    };

    let delta;
    if st.mem[q].thickness == Scaled::ZERO {
        let clr = if st.fonts.font_is_ot_math(cur_f) {
            if style < MathStyle::TEXT {
                st.fonts
                    .get_ot_math_constant(cur_f, STACKDISPLAYSTYLEGAPMIN)
            } else {
                st.fonts.get_ot_math_constant(cur_f, STACKGAPMIN)
            }
        } else if style < MathStyle::TEXT {
            st.fonts.default_rule_thickness(&st.eqtb, cur_size) * 7
        } else {
            st.fonts.default_rule_thickness(&st.eqtb, cur_size) * 3
        };

        delta = (clr - ((shift_up - st.mem[x].depth) - (st.mem[z].height - shift_down))).half();
        if delta > Scaled::ZERO {
            shift_up += delta;
            shift_down += delta;
        }
    } else {
        let axis_height = st.fonts.axis_height(&st.eqtb, cur_size);
        let (delta1, delta2) = if st.fonts.font_is_ot_math(cur_f) {
            delta = st.mem[q].thickness.half();
            let clr_num = if style < MathStyle::TEXT {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONNUMDISPLAYSTYLEGAPMIN)
            } else {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONNUMERATORGAPMIN)
            };
            let delta1 = clr_num - ((shift_up - st.mem[x].depth) - (axis_height + delta));

            let clr_den = if style < MathStyle::TEXT {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONDENOMDISPLAYSTYLEGAPMIN)
            } else {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONDENOMINATORGAPMIN)
            };
            let delta2 = clr_den - ((axis_height - delta) - (st.mem[z].height - shift_down));
            (delta1, delta2)
        } else {
            let clr = if style < MathStyle::TEXT {
                st.mem[q].thickness * 3
            } else {
                st.mem[q].thickness
            };
            delta = st.mem[q].thickness.half();
            (
                clr - ((shift_up - st.mem[x].depth) - (axis_height + delta)),
                clr - ((axis_height - delta) - (st.mem[z].height - shift_down)),
            )
        };

        if delta1 > Scaled::ZERO {
            shift_up += delta1;
        }
        if delta2 > Scaled::ZERO {
            shift_down += delta2;
        }
    }

    let axis_height = st.fonts.axis_height(&st.eqtb, cur_size);
    let rule_thickness = st.mem[q].thickness;
    let v = st.mem.make_fraction(FractionAssembly {
        x,
        z,
        rule_thickness,
        shift_up,
        shift_down,
        delta,
        axis_height,
    })?;

    let delimiter_size = if style < MathStyle::TEXT {
        st.fonts.delim1(&st.eqtb, cur_size)
    } else {
        st.fonts.delim2(&st.eqtb, cur_size)
    };

    let left_delim = var_delimiter(st, st.mem[q].left_delimiter, cur_size, delimiter_size)?;
    let right_delim = var_delimiter(st, st.mem[q].right_delimiter, cur_size, delimiter_size)?;
    st.mem[left_delim].link = v.ptr();
    st.mem[v].link = right_delim.ptr();
    let hbox = crate::packaging::hpack(st, left_delim.ptr(), PackSpec::NATURAL)?;
    st.mem[q.any()].new_hlist = hbox.ptr();
    Ok(())
}

/// Returns the italic correction `delta`.
fn make_op(st: &mut EngineState, q: NodePtr<Noad>, style: MathStyle) -> Result<Scaled, Abort> {
    let mut delta = Scaled::ZERO;

    if st.mem[q].limits() == LimitCode::Normal && style < MathStyle::TEXT {
        st.mem[q].set_limits(LimitCode::Limits);
    }

    if st.mem[q].nucleus.math_type == MATH_CHAR {
        let Fetched { f, mut i, .. } = fetch(st, MathSlot::Nucleus(q), style.size())?;

        if !st.fonts.font_uses_opentype(f) {
            if let Some(CharInfo {
                tag: CharTag::List(c),
                ..
            }) = i.filter(|_| style < MathStyle::TEXT)
                && let Some(larger) = st.fonts.char_info(f, i32::from(c))
            {
                i = Some(larger);
                st.mem[q].nucleus.set_character(c);
            }
            delta = i.map_or(Scaled::ZERO, |i| i.italic);
        }

        let x = clean_box(st, st.mem[q].nucleus, style)?;

        let cur_f = st.scanner.cur_f;
        if st.fonts.font_is_ot_math(cur_f)
            && let Some(glyph) = st.mem.glyph_whatsit(st.mem[x].list_ptr)
        {
            let (width, height, depth) = 'found: {
                if style < MathStyle::TEXT {
                    let mut h1 = st
                        .fonts
                        .get_ot_math_constant(cur_f, DISPLAYOPERATORMINHEIGHT);
                    let size_h = f64::from(st.mem[glyph].height + st.mem[glyph].depth) * 5.0 / 4.0;
                    if f64::from(h1) < size_h {
                        h1 = Scaled::from_raw(size_h as i32);
                    }
                    let c_glyph = i32::from(st.mem[glyph].glyph);
                    let mut n: i32 = 0;
                    let mut h2 = Scaled::ZERO;
                    loop {
                        let g = st
                            .fonts
                            .get_ot_math_variant(cur_f, c_glyph, n, &mut h2, false);
                        if h2 > Scaled::ZERO {
                            st.mem[glyph].glyph = g as u16;
                            st.fonts.measure_glyph(&mut st.mem, glyph)?;
                        }
                        n += 1;
                        if h2 < Scaled::ZERO || h2 >= h1 {
                            break;
                        }
                    }
                    if h2 < Scaled::ZERO {
                        if let Some(assembly) = st.fonts.get_ot_assembly(cur_f, c_glyph, false) {
                            glyph.free(&mut st.mem);
                            let b = build_opentype_assembly(st, cur_f, &assembly, h1, false)?;
                            st.mem[x].list_ptr = b.ptr();
                            delta = Scaled::ZERO;
                            break 'found (st.mem[b].width, st.mem[b].height, st.mem[b].depth);
                        }
                    } else {
                        st.fonts.measure_glyph(&mut st.mem, glyph)?;
                    }
                }
                delta = st
                    .fonts
                    .get_ot_math_ital_corr(cur_f, i32::from(st.mem[glyph].glyph));
                (
                    st.mem[glyph].width,
                    st.mem[glyph].height,
                    st.mem[glyph].depth,
                )
            };
            let node = &mut st.mem[x];
            node.width = width;
            node.height = height;
            node.depth = depth;
        }

        let axis_height = st.fonts.axis_height(&st.eqtb, style.size());
        let mem = &mut st.mem;
        if mem[q].subscr.math_type != EMPTY && mem[q].limits() != LimitCode::Limits {
            mem[x].width -= delta;
        }
        let shift = (mem[x].height - mem[x].depth).half() - axis_height;
        mem[x].shift_amount = shift;
        mem.set_nucleus_box(q, x);
    }

    let save_f = st.scanner.cur_f;
    if st.mem[q].limits() == LimitCode::Limits {
        let cs = style;
        let x = clean_box(st, st.mem[q].supscr, cs.sup())?;
        let y = clean_box(st, st.mem[q].nucleus, cs)?;
        let z = clean_box(st, st.mem[q].subscr, cs.sub())?;

        let v = st.mem.new_null_vbox()?;
        let w = st.mem[y].width.max(st.mem[x].width).max(st.mem[z].width);
        st.mem[v].width = w;

        let x = rebox(st, x, w)?;
        let y = rebox(st, y, w)?;
        let z = rebox(st, z, w)?;

        let size = style.size();
        let bos1 = st.fonts.big_op_spacing1(&st.eqtb, size);
        let bos2 = st.fonts.big_op_spacing2(&st.eqtb, size);
        let bos3 = st.fonts.big_op_spacing3(&st.eqtb, size);
        let bos4 = st.fonts.big_op_spacing4(&st.eqtb, size);
        let bos5 = st.fonts.big_op_spacing5(&st.eqtb, size);

        let mem = &mut st.mem;
        let half_delta = delta.half();
        mem[x].shift_amount = half_delta;
        mem[z].shift_amount = -half_delta;
        let (height, depth) = (mem[y].height, mem[y].depth);
        let node = &mut mem[v];
        node.height = height;
        node.depth = depth;
        st.scanner.cur_f = save_f;

        if mem[q].supscr.math_type == EMPTY {
            x.free(mem);
            mem[v].list_ptr = y.ptr();
        } else {
            let shift_up = (bos3 - mem[x].depth).max(bos1);
            let p = mem.new_kern(shift_up)?;
            mem[p].link = y.ptr();
            mem[x].link = p.ptr();
            let p = mem.new_kern(bos5)?;
            mem[p].link = x.ptr();
            mem[v].list_ptr = p.ptr();
            let height = mem[v].height + bos5 + mem[x].height + mem[x].depth + shift_up;
            mem[v].height = height;
        }

        if mem[q].subscr.math_type == EMPTY {
            z.free(mem);
        } else {
            let shift_down = (bos4 - mem[z].height).max(bos2);
            let p = mem.new_kern(shift_down)?;
            mem[y].link = p.ptr();
            mem[p].link = z.ptr();
            let p = mem.new_kern(bos5)?;
            mem[z].link = p.ptr();
            let depth = mem[v].depth + bos5 + mem[z].height + mem[z].depth + shift_down;
            mem[v].depth = depth;
        }

        mem[q.any()].new_hlist = v.ptr();
    }

    Ok(delta)
}

enum MakeOrdStep {
    Return,
    Restart,
}

impl MemState {
    /// Apply the first ligature or kern between `q`'s character, fetched as
    /// `fetched`, and the character of the next noad `p`.
    fn make_ord_body(
        &mut self,
        fonts: &FontState,
        f: FontId,
        start: LigKernIdx,
        q: NodePtr<Noad>,
        p: NodePtr<Noad>,
    ) -> Result<MakeOrdStep, Abort> {
        let next_c = LigChar::of(self[p].nucleus.character());
        let (mut a, mut step) = fonts.lig_kern_start(f, start)?;
        loop {
            match step.action_for(next_c) {
                Some(LigKernAction::Kern(kern_width)) => {
                    let p_kern = self.new_kern(kern_width)?;
                    self[p_kern].link = self[q].link;
                    self[q].link = p_kern.ptr();
                    return Ok(MakeOrdStep::Return);
                }
                Some(LigKernAction::Lig { op, char }) => {
                    match op {
                        LigOp::KeepRight | LigOp::KeepRightSkip => {
                            self[q].nucleus.set_character(char);
                        }
                        LigOp::KeepLeft | LigOp::KeepLeftSkip => {
                            self[p].nucleus.set_character(char);
                        }
                        LigOp::KeepBoth | LigOp::KeepBothSkip | LigOp::KeepBothSkip2 => {
                            let r = self.new_noad()?;
                            self[r].nucleus.set_character(char);
                            let fam = self[q].nucleus.plane_and_fam() % 256;
                            self[r].nucleus.set_plane_and_fam(fam);
                            self[q].link = r.ptr();
                            self[r].link = p.ptr();
                            self[r].nucleus.math_type = if op == LigOp::KeepBothSkip2 {
                                MATH_TEXT_CHAR
                            } else {
                                MATH_CHAR
                            };
                        }
                        LigOp::Replace => {
                            let src = self[p];
                            let node = &mut self[q];
                            node.link = src.link;
                            node.nucleus.set_character(char);
                            node.subscr = src.subscr;
                            node.supscr = src.supscr;
                            p.free(self);
                        }
                    }
                    if !matches!(
                        op,
                        LigOp::Replace | LigOp::KeepRight | LigOp::KeepLeft | LigOp::KeepBoth
                    ) {
                        return Ok(MakeOrdStep::Return);
                    }
                    self[q].nucleus.math_type = MATH_CHAR;
                    return Ok(MakeOrdStep::Restart);
                }
                None => {}
            }
            let Some(next) = step.next(a) else {
                return Ok(MakeOrdStep::Return);
            };
            a = next;
            step = fonts.lig_kern_step(f, a)?;
        }
    }
}

fn make_ord(st: &mut EngineState, q: NodePtr<Noad>, style: MathStyle) -> Result<(), Abort> {
    'restart: loop {
        if st.mem[q].subscr.math_type != EMPTY {
            return Ok(());
        }
        if st.mem[q].supscr.math_type != EMPTY {
            return Ok(());
        }
        if st.mem[q].nucleus.math_type != MATH_CHAR {
            return Ok(());
        }
        let p = st.mem[q].link;
        let Some(p) = p else {
            return Ok(());
        };
        let p_node = match st
            .mem
            .classify_noad(p)
            .map(|n| n.map(|n| n.variant(&st.mem)))
        {
            Ok(Some(NoadVariant::Noad(pn)))
                if st.mem[pn].noad_type >= NodeType::OrdNoad
                    && st.mem[pn].noad_type <= NodeType::PunctNoad =>
            {
                pn
            }
            _ => return Ok(()),
        };
        if st.mem[p_node].nucleus.math_type != MATH_CHAR {
            return Ok(());
        }
        if st.mem[p_node].nucleus.plane_and_fam() % 256 != st.mem[q].nucleus.plane_and_fam() % 256 {
            return Ok(());
        }

        st.mem[q].nucleus.math_type = MATH_TEXT_CHAR;
        let fetched = fetch(st, MathSlot::Nucleus(q), style.size())?;
        let Some(CharTag::Lig(start)) = fetched.i.map(|i| i.tag) else {
            return Ok(());
        };

        match st
            .mem
            .make_ord_body(&st.fonts, fetched.f, start, q, p_node)?
        {
            MakeOrdStep::Return => return Ok(()),
            MakeOrdStep::Restart => continue 'restart,
        }
    }
}

fn make_math_accent(
    st: &mut EngineState,
    q: NodePtr<AccentNoad>,
    style: MathStyle,
) -> Result<(), Abort> {
    let n = q.as_noad();
    let accent = fetch(st, MathSlot::AccentChr(q), style.size())?;
    let f = accent.f;
    let mut c = accent.c;
    let area = st.fonts.font_area[f];
    let kind = st.mem[q].kind().get();
    let bottom_accent = kind.is_some_and(MathAccentKind::is_bottom);
    let is_native = area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG;

    let mut s = Scaled::ZERO;
    let mut x: NodePtr<BoxNode>;
    if is_native {
        let attach = if bottom_accent {
            None
        } else {
            Some(compute_ot_math_accent_pos(st, q, style)?)
        };
        x = clean_box(st, st.mem[n].nucleus, style.cramped())?;
        s = match attach {
            None => Scaled::ZERO,
            Some(AccentAttach::At(pos)) => pos,
            Some(AccentAttach::Skew(skew)) => st.mem[x].width.half() + skew,
            Some(AccentAttach::Centre) => INFINITE_DIMEN,
        };
    } else if let Some(mut acc_i) = accent.i {
        if st.mem[n].nucleus.math_type == MATH_CHAR {
            let nucleus = fetch(st, MathSlot::Nucleus(n), style.size())?;
            s = tfm_skew(st, &nucleus)?;
        }
        x = clean_box(st, st.mem[n].nucleus, style.cramped())?;
        let w = st.mem[x].width;
        while let CharTag::List(larger) = acc_i.tag {
            let larger = i32::from(larger);
            match st.fonts.char_info(f, larger) {
                Some(info) if info.width <= w => acc_i = info,
                _ => break,
            }
            c = larger;
        }
    } else {
        return Ok(());
    }
    let w = st.mem[x].width;
    let mut h = st.mem[x].height;

    let is_ot_math = st.fonts.font_is_ot_math(f);
    let mut delta = if is_ot_math {
        if bottom_accent {
            Scaled::ZERO
        } else {
            h.min(st.fonts.get_ot_math_constant(f, ACCENTBASEHEIGHT))
        }
    } else {
        h.min(st.fonts.param(f, X_HEIGHT_CODE))
    };

    if (st.mem[n].supscr.math_type != EMPTY || st.mem[n].subscr.math_type != EMPTY)
        && st.mem[n].nucleus.math_type == MATH_CHAR
    {
        st.mem.flush_node_list(x.ptr())?;
        let base = st.mem.new_noad()?;
        let mem = &mut st.mem;
        let src = mem[n];
        let node = &mut mem[base];
        node.nucleus = src.nucleus;
        node.supscr = src.supscr;
        node.subscr = src.subscr;
        for script in [MathSlot::Supscr(n), MathSlot::Subscr(n)] {
            mem[script].set_info(MemIdx::from_raw(0));
            mem[script].math_type = EMPTY;
        }
        mem[n].nucleus.math_type = SUB_MLIST;
        mem[n].nucleus.set_info(base.ptr());
        x = clean_box(st, st.mem[n].nucleus, style)?;
        delta += st.mem[x].height - h;
        h = st.mem[x].height;
    }

    let y;
    if is_native {
        let old = crate::fonts::new_native_character(st, f, c as u32)?;
        y = st.mem.native_char_box(old)?;
        let glyph = NodePtr::<GlyphNode>::alloc_whatsit(&mut st.mem, WhatsitSubtype::Glyph)?;
        let glyph_id = st.mem.get_native_glyph(old, 0);
        let node = &mut st.mem[glyph];
        node.font = f;
        node.glyph = glyph_id;
        st.fonts.measure_glyph(&mut st.mem, glyph)?;
        old.free_native(&mut st.mem);
        st.mem[y].list_ptr = glyph.ptr();

        let mut assembly_box = None;
        if kind.is_some_and(MathAccentKind::is_fixed) {
            st.fonts.measure_glyph(&mut st.mem, glyph)?;
        } else {
            let c_glyph = i32::from(st.mem[glyph].glyph);
            let mut n: i32 = 0;
            let mut w2 = Scaled::ZERO;
            loop {
                let g = st.fonts.get_ot_math_variant(f, c_glyph, n, &mut w2, true);
                if w2 > Scaled::ZERO && w2 <= w {
                    st.mem[glyph].glyph = g as u16;
                    st.fonts.measure_glyph(&mut st.mem, glyph)?;
                    n += 1;
                }
                if w2 < Scaled::ZERO || w2 >= w {
                    break;
                }
            }
            if w2 < Scaled::ZERO {
                if let Some(assembly) = st.fonts.get_ot_assembly(f, c_glyph, true) {
                    glyph.free(&mut st.mem);
                    let b = build_opentype_assembly(st, f, &assembly, w, true)?;
                    st.mem[y].list_ptr = b.ptr();
                    assembly_box = Some(b);
                }
            } else {
                st.fonts.measure_glyph(&mut st.mem, glyph)?;
            }
        }

        let mem = &mut st.mem;
        let (gw, gh, gd) = match assembly_box {
            Some(b) => (mem[b].width, mem[b].height, mem[b].depth),
            None => (mem[glyph].width, mem[glyph].height, mem[glyph].depth),
        };
        let node = &mut mem[y];
        node.width = gw;
        node.height = gh;
        node.depth = gd;
        if bottom_accent {
            if mem[y].height < Scaled::ZERO {
                mem[y].height = Scaled::ZERO;
            }
        } else if mem[y].depth < Scaled::ZERO {
            mem[y].depth = Scaled::ZERO;
        }

        let attach = match assembly_box {
            None => st
                .fonts
                .get_ot_math_accent_pos(f, i32::from(mem[glyph].glyph)),
            Some(_) => INFINITE_DIMEN,
        };
        let sa = if attach == INFINITE_DIMEN {
            mem[y].width.half()
        } else {
            attach
        };
        if bottom_accent || s == INFINITE_DIMEN {
            s = w.half();
        }
        mem[y].shift_amount = s - sa;
    } else {
        y = st.mem.char_box_tfm(&mut st.fonts, f, c as u16)?;
        let shift = s + (w - st.mem[y].width).half();
        st.mem[y].shift_amount = shift;
    }

    st.mem[y].width = Scaled::ZERO;
    let v = if bottom_accent {
        st.mem[x].link = y.ptr();
        let v = crate::packaging::vpackage(st, x.ptr(), PackSpec::NATURAL, MAX_DIMEN)?;
        let v_height = st.mem[v].height;
        st.mem[v].shift_amount = -(h - v_height);
        v
    } else {
        let kern = st.mem.new_kern(-delta)?;
        st.mem[kern].link = x.ptr();
        st.mem[y].link = kern.ptr();
        let v = crate::packaging::vpackage(st, y.ptr(), PackSpec::NATURAL, MAX_DIMEN)?;
        let mem = &mut st.mem;
        if mem[v].height < h {
            let kern = mem.new_kern(h - mem[v].height)?;
            mem[kern].link = mem[v].list_ptr;
            let node = &mut mem[v];
            node.list_ptr = kern.ptr();
            node.height = h;
        }
        v
    };
    let x_width = st.mem[x].width;
    st.mem[v].width = x_width;
    st.mem.set_nucleus_box(n, v);
    Ok(())
}

fn make_left_right(
    st: &mut EngineState,
    q: NodePtr<FenceNoad>,
    style: MathStyle,
    max_d: Scaled,
    max_h: Scaled,
) -> Result<NodeType, Abort> {
    let size = style.size();
    let axis_height = st.fonts.axis_height(&st.eqtb, size);
    let delimiter_factor = st.eqtb.int_par(IntPar::DelimiterFactor);
    let delimiter_shortfall = st.eqtb.dimen_par(DimenPar::DelimiterShortfall);
    let delta2 = max_d + axis_height;
    let delta1 = (max_h + max_d - delta2).max(delta2);
    let delta = ((delta1 / 500) * delimiter_factor).max(delta1 + delta1 - delimiter_shortfall);
    let delimiter = var_delimiter(st, st.mem[q].delimiter, size, delta)?;
    st.mem[q.any()].new_hlist = delimiter.ptr();
    Ok(if st.mem[q].noad_type == NodeType::LeftNoad {
        NodeType::OpenNoad
    } else {
        NodeType::CloseNoad
    })
}

impl MemState {
    fn mlist_choice_node(
        &mut self,
        choice: NodePtr<ChoiceNode>,
        cur_style: MathStyle,
    ) -> Result<(), Abort> {
        let kept = match cur_style.level() {
            0 => {
                let p = self[choice].display_mlist;
                self[choice].display_mlist = None;
                p
            }
            1 => {
                let p = self[choice].text_mlist;
                self[choice].text_mlist = None;
                p
            }
            2 => {
                let p = self[choice].script_mlist;
                self[choice].script_mlist = None;
                p
            }
            3 => {
                let p = self[choice].script_script_mlist;
                self[choice].script_script_mlist = None;
                p
            }
            _ => None,
        };
        self.flush_node_list(self[choice].display_mlist)?;
        self.flush_node_list(self[choice].text_mlist)?;
        self.flush_node_list(self[choice].script_mlist)?;
        self.flush_node_list(self[choice].script_script_mlist)?;
        let style = choice.into_style(self, cur_style);
        if let Some(k) = kept {
            let z = self[style].link;
            self[style].link = kept;
            let p = self.list_tail(k);
            p.set_link(self, z);
        }
        Ok(())
    }

    fn mlist_track_rule(&self, rule: NodePtr<RuleNode>, max_h: &mut Scaled, max_d: &mut Scaled) {
        *max_h = (*max_h).max(self[rule].height.to_scaled());
        *max_d = (*max_d).max(self[rule].depth.to_scaled());
    }

    fn mlist_track_box(&mut self, z: NodePtr<BoxNode>, max_h: &mut Scaled, max_d: &mut Scaled) {
        *max_h = (*max_h).max(self[z].height);
        *max_d = (*max_d).max(self[z].depth);
        z.free(self);
    }

    fn mlist_cond_math_glue(
        &mut self,
        glue: NodePtr<GlueNode>,
        cur_size: i32,
    ) -> Result<(), Abort> {
        if cur_size == TEXT_SIZE {
            return Ok(());
        }

        let p = self[glue].link;
        let Some(p_idx) = p else {
            return Ok(());
        };
        if matches!(
            self.classify(p_idx),
            Ok(AnyNode::Glue(_) | AnyNode::Kern(_))
        ) {
            self[glue].link = p_idx.link(self);
            p_idx.set_link(self, None);
            self.flush_node_list(p)?;
        }
        Ok(())
    }

    /// Moves `node` onto the output list after `tail`, returning it and its old successor.
    fn mlist_move_output_node(&mut self, tail: MemIdx, node: AnyNode) -> (MemIdx, MemPtr) {
        let q = node.idx();
        tail.set_link(self, Some(q));
        let next = q.link(self);
        q.set_link(self, None);
        (q, next)
    }
}

/// The type that follows `qn` and the next node. A right fence resets the
/// current style `cur_style` to the list's `start_style`.
fn mlist_advance(
    st: &mut EngineState,
    qn: NodePtr<AnyNoad>,
    start_style: MathStyle,
    cur_style: &mut MathStyle,
) -> (u16, MemPtr) {
    let mut r_type = st.mem[qn].noad_type.raw();
    let next = st.mem[qn].link;
    if r_type == NodeType::RightNoad.to_u16() {
        r_type = NodeType::LeftNoad.to_u16();
        *cur_style = start_style;
    }
    (r_type, next)
}

/// Translates the nucleus of `qn` into its `new_hlist` and attaches its scripts.
fn mlist_nucleus(
    st: &mut EngineState,
    qn: NodePtr<Noad>,
    mut delta: Scaled,
    style: MathStyle,
) -> Result<(), Abort> {
    let p = match st.mem[qn].nucleus.math_type {
        MATH_CHAR | MATH_TEXT_CHAR => {
            let fetched = fetch(st, MathSlot::Nucleus(qn), style.size())?;
            let cur_f = fetched.f;
            let area = st.fonts.font_area[cur_f];
            let p = if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
                let z = crate::fonts::new_native_character(st, cur_f, fetched.c as u32)?;
                let glyph =
                    NodePtr::<GlyphNode>::alloc_whatsit(&mut st.mem, WhatsitSubtype::Glyph)?;
                let z_glyph = st.mem.get_native_glyph(z, 0);
                let node = &mut st.mem[glyph];
                node.font = cur_f;
                node.glyph = z_glyph;
                st.fonts.measure_glyph(&mut st.mem, glyph)?;
                z.free_native(&mut st.mem);
                delta = st
                    .fonts
                    .get_ot_math_ital_corr(cur_f, i32::from(st.mem[glyph].glyph));
                if st.mem[qn].nucleus.math_type == MATH_TEXT_CHAR
                    && !st.fonts.font_is_ot_math(cur_f)
                {
                    delta = Scaled::ZERO;
                }
                glyph.ptr()
            } else if let Some(i) = fetched.i {
                delta = i.italic;
                let cc = fetched.c as u16;
                let p = crate::main_control::new_character(st, cur_f, cc)?;
                let p = p.and_then(crate::main_control::NewChar::ptr);
                if st.mem[qn].nucleus.math_type == MATH_TEXT_CHAR
                    && st.fonts.param(cur_f, SPACE_CODE) != Scaled::ZERO
                {
                    delta = Scaled::ZERO;
                }
                p
            } else {
                None
            };
            if let Some(pk) = p
                && st.mem[qn].subscr.math_type == EMPTY
                && delta != Scaled::ZERO
            {
                let k = st.mem.new_kern(delta)?;
                pk.set_link(&mut st.mem, k.ptr());
                delta = Scaled::ZERO;
            }
            p
        }
        EMPTY => None,
        SUB_BOX => st.mem[qn].nucleus.info(),
        SUB_MLIST => {
            let mlist = st.mem[qn].nucleus.info();
            let list = mlist_to_hlist(st, mlist, style, false)?;
            crate::packaging::hpack(st, list, PackSpec::NATURAL)?.ptr()
        }
        _ => return Err(Abort::Confusion("mlist2")),
    };
    st.mem[qn.any()].new_hlist = p;
    let need_scripts = st.mem[qn].subscr.math_type != EMPTY || st.mem[qn].supscr.math_type != EMPTY;
    if need_scripts {
        make_scripts(st, qn, delta, style)?;
    }

    Ok(())
}

/// Inter-noad spacing, indexed by the left and right noad types relative to ORD.
const MATH_SPACING: [&[u8; 8]; 8] = [
    b"02340001",
    b"22*40001",
    b"33**3**3",
    b"44*04004",
    b"00*00000",
    b"02340001",
    b"11*11111",
    b"12341011",
];

/// Packs the translated hlist of `qn` to track the list's height and depth.
fn check_noad_dimensions(
    st: &mut EngineState,
    qn: NodePtr<AnyNoad>,
    max_h: &mut Scaled,
    max_d: &mut Scaled,
) -> Result<(), Abort> {
    let list = st.mem[qn].new_hlist;
    let z_box = crate::packaging::hpack(st, list, PackSpec::NATURAL)?;
    st.mem.mlist_track_box(z_box, max_h, max_d);
    Ok(())
}

/// Convert `mlist`, starting at `start_style`, to an hlist, with line-break
/// penalties after binary and relation atoms if `penalties`. Returns the
/// hlist, which starts after `TEMP_HEAD`.
pub(super) fn mlist_to_hlist(
    st: &mut EngineState,
    mlist: MemPtr,
    start_style: MathStyle,
    penalties: bool,
) -> Result<MemPtr, Abort> {
    let mut style = start_style;
    let mut q = mlist;
    let mut r: Option<NodePtr<AnyNoad>> = None;
    let mut r_type = NodeType::OpNoad.to_u16();
    let mut max_h = Scaled::ZERO;
    let mut max_d = Scaled::ZERO;

    enum FirstPass {
        ProcessNucleus(NodePtr<Noad>),
        CheckDimensions(NodePtr<AnyNoad>),
        DoneWithNoad(NodePtr<AnyNoad>),
        DoneWithNode,
    }

    while let Some(here) = q {
        let mut delta;
        let action = 'reswitch: loop {
            delta = Scaled::ZERO;
            match st.mem.classify(here) {
                Ok(AnyNode::Noad(qn)) => match qn.variant(&st.mem) {
                    NoadVariant::Fraction(f) => {
                        make_fraction(st, f, style)?;
                        break 'reswitch FirstPass::CheckDimensions(qn);
                    }
                    NoadVariant::Radical(rad) => {
                        make_radical(st, rad, style)?;
                        break 'reswitch FirstPass::ProcessNucleus(rad.as_noad());
                    }
                    NoadVariant::Accent(acc) => {
                        make_math_accent(st, acc, style)?;
                        break 'reswitch FirstPass::ProcessNucleus(acc.as_noad());
                    }
                    NoadVariant::Fence(f) => {
                        if st.mem[f].noad_type == NodeType::RightNoad
                            && r_type == NodeType::BinNoad.to_u16()
                        {
                            let r = r.or_confusion("mlist1")?;
                            st.mem[r].noad_type = NodeType::OrdNoad.into();
                        }
                        break 'reswitch FirstPass::DoneWithNoad(qn);
                    }
                    NoadVariant::Noad(n) => match st.mem[n].noad_type.get() {
                        Some(NodeType::BinNoad) => {
                            match NodeType::of_code(r_type) {
                                Ok(NodeType::BinNoad)
                                | Ok(NodeType::OpNoad)
                                | Ok(NodeType::RelNoad)
                                | Ok(NodeType::OpenNoad)
                                | Ok(NodeType::PunctNoad)
                                | Ok(NodeType::LeftNoad) => {
                                    st.mem[n].noad_type = NodeType::OrdNoad.into();
                                    continue 'reswitch;
                                }
                                _ => {}
                            }
                            break 'reswitch FirstPass::ProcessNucleus(n);
                        }
                        Some(NodeType::RelNoad | NodeType::CloseNoad | NodeType::PunctNoad) => {
                            if r_type == NodeType::BinNoad.to_u16() {
                                let r = r.or_confusion("mlist1")?;
                                st.mem[r].noad_type = NodeType::OrdNoad.into();
                            }
                            break 'reswitch FirstPass::ProcessNucleus(n);
                        }
                        Some(NodeType::OpNoad) => {
                            delta = make_op(st, n, style)?;
                            if st.mem[n].limits() == LimitCode::Limits {
                                break 'reswitch FirstPass::CheckDimensions(qn);
                            }
                            break 'reswitch FirstPass::ProcessNucleus(n);
                        }
                        Some(NodeType::OrdNoad) => {
                            make_ord(st, n, style)?;
                            break 'reswitch FirstPass::ProcessNucleus(n);
                        }
                        Some(NodeType::OpenNoad | NodeType::InnerNoad) => {
                            break 'reswitch FirstPass::ProcessNucleus(n)
                        }
                        Some(NodeType::OverNoad) => {
                            make_over(st, n, style)?;
                            break 'reswitch FirstPass::ProcessNucleus(n);
                        }
                        Some(NodeType::UnderNoad) => {
                            make_under(st, n, style)?;
                            break 'reswitch FirstPass::ProcessNucleus(n);
                        }
                        Some(NodeType::VcenterNoad) => {
                            make_vcenter(st, n, style)?;
                            break 'reswitch FirstPass::ProcessNucleus(n);
                        }
                        _ => return Err(Abort::Confusion("mlist1")),
                    },
                },
                Ok(AnyNode::Style(sn)) => {
                    style = st.mem[sn].style;
                    break 'reswitch FirstPass::DoneWithNode;
                }
                Ok(AnyNode::Choice(choice)) => {
                    let cs = style;
                    st.mem.mlist_choice_node(choice, cs)?;
                    break 'reswitch FirstPass::DoneWithNode;
                }
                Ok(AnyNode::Ins(_))
                | Ok(AnyNode::Mark(_))
                | Ok(AnyNode::Adjust(_))
                | Ok(AnyNode::Whatsit(_))
                | Ok(AnyNode::Penalty(_))
                | Ok(AnyNode::Disc(_)) => {
                    break 'reswitch FirstPass::DoneWithNode;
                }
                Ok(AnyNode::Rule(rule)) => {
                    st.mem.mlist_track_rule(rule, &mut max_h, &mut max_d);
                    break 'reswitch FirstPass::DoneWithNode;
                }
                Ok(AnyNode::Glue(glue)) => {
                    let subtype = st.mem[glue].subtype;
                    let x = st.mem[glue].glue_ptr;
                    if subtype == GlueKind::Mu {
                        let mu = math_mu(st, style);
                        let y = math_glue(st, x, mu)?;
                        st.mem.delete_glue_ref(x);
                        let node = &mut st.mem[glue];
                        node.glue_ptr = y;
                        node.subtype = GlueKind::Normal.into();
                    } else if subtype == GlueKind::CondMath {
                        let cur_size = style.size();
                        st.mem.mlist_cond_math_glue(glue, cur_size)?;
                    }
                    break 'reswitch FirstPass::DoneWithNode;
                }
                Ok(AnyNode::Kern(kern)) => {
                    let mu = math_mu(st, style);
                    math_kern(st, kern, mu);
                    break 'reswitch FirstPass::DoneWithNode;
                }
                _ => return Err(Abort::Confusion("mlist1")),
            }
        };

        let qn = match action {
            FirstPass::DoneWithNode => {
                q = here.link(&st.mem);
                continue;
            }
            FirstPass::DoneWithNoad(qn) => qn,
            FirstPass::ProcessNucleus(n) => {
                mlist_nucleus(st, n, delta, style)?;
                check_noad_dimensions(st, n.any(), &mut max_h, &mut max_d)?;
                n.any()
            }
            FirstPass::CheckDimensions(qn) => {
                check_noad_dimensions(st, qn, &mut max_h, &mut max_d)?;
                qn
            }
        };
        r = Some(qn);
        (r_type, q) = mlist_advance(st, qn, start_style, &mut style);
    }

    let mut p = TEMP_HEAD;
    if r_type == NodeType::BinNoad.to_u16() {
        let r = r.or_confusion("mlist1")?;
        st.mem[r].noad_type = NodeType::OrdNoad.into();
    }
    p.set_link(&mut st.mem, None);
    q = mlist;
    r_type = 0;
    style = start_style;

    enum SecondPass {
        Splice(NodePtr<AnyNoad>),
        StyleNode,
        Moved,
    }

    while let Some(here) = q {
        let mut t = NodeType::OrdNoad.to_u16();
        let mut s = Noad::SIZE;
        let mut pen = INF_PENALTY;
        let step = match st.mem.classify(here) {
            Ok(AnyNode::Noad(qn)) => {
                let variant = qn.variant(&st.mem);
                s = variant.size();
                match variant {
                    NoadVariant::Noad(n) => match st.mem[n].noad_type.get() {
                        Some(
                            nt @ (NodeType::OpNoad
                            | NodeType::OpenNoad
                            | NodeType::CloseNoad
                            | NodeType::PunctNoad
                            | NodeType::InnerNoad),
                        ) => t = nt.to_u16(),
                        Some(NodeType::BinNoad) => {
                            t = NodeType::BinNoad.to_u16();
                            pen = st.eqtb.int_par(IntPar::BinOpPenalty);
                        }
                        Some(NodeType::RelNoad) => {
                            t = NodeType::RelNoad.to_u16();
                            pen = st.eqtb.int_par(IntPar::RelPenalty);
                        }
                        Some(
                            NodeType::OrdNoad
                            | NodeType::VcenterNoad
                            | NodeType::OverNoad
                            | NodeType::UnderNoad,
                        ) => {}
                        _ => return Err(Abort::Confusion("mlist3")),
                    },
                    NoadVariant::Radical(_) | NoadVariant::Accent(_) | NoadVariant::Fraction(_) => {
                    }
                    NoadVariant::Fence(f) => {
                        style = start_style;
                        t = make_left_right(st, f, start_style, max_d, max_h)?.to_u16();
                    }
                }
                SecondPass::Splice(qn)
            }
            Ok(AnyNode::Style(sn)) => {
                style = st.mem[sn].style;
                s = StyleNode::SIZE;
                SecondPass::StyleNode
            }
            Ok(
                node @ (AnyNode::Whatsit(_)
                | AnyNode::Penalty(_)
                | AnyNode::Rule(_)
                | AnyNode::Disc(_)
                | AnyNode::Adjust(_)
                | AnyNode::Ins(_)
                | AnyNode::Mark(_)
                | AnyNode::Glue(_)
                | AnyNode::Kern(_)),
            ) => {
                let (np, nq) = st.mem.mlist_move_output_node(p, node);
                p = np;
                q = nq;
                SecondPass::Moved
            }
            _ => return Err(Abort::Confusion("mlist3")),
        };

        let qn = match step {
            SecondPass::Moved => continue,
            SecondPass::StyleNode => None,
            SecondPass::Splice(qn) => Some(qn),
        };

        if let Some(qn) = qn {
            if r_type > 0 {
                let ord = NodeType::OrdNoad.to_u16();
                let spacing = MATH_SPACING[(r_type - ord) as usize][(t - ord) as usize];
                let x = match spacing {
                    b'0' => None,
                    b'2' => Some(GluePar::ThinMuSkip),
                    b'1' | b'3' | b'4' if style >= MathStyle::SCRIPT => None,
                    b'1' => Some(GluePar::ThinMuSkip),
                    b'3' => Some(GluePar::MedMuSkip),
                    b'4' => Some(GluePar::ThickMuSkip),
                    _ => return Err(Abort::Confusion("mlist4")),
                };
                if let Some(x) = x {
                    let g = st.eqtb.glue_par(x);
                    let mu = math_mu(st, style);
                    let y = math_glue(st, g, mu)?;
                    let z = st.mem.new_glue(y)?;
                    st.mem[y].ref_count = TEX_NULL;
                    p.set_link(&mut st.mem, z.ptr());
                    st.mem[z].subtype = GlueKind::Param(x).into();
                    p = z.idx();
                }
            }

            if let Some(head) = st.mem[qn].new_hlist {
                p.set_link(&mut st.mem, Some(head));
                p = st.mem.list_tail(head);
            }

            if penalties
                && pen < INF_PENALTY
                && let Some(next) = st.mem[qn].link
                && !matches!(
                    st.mem.node_type(next),
                    Ok(NodeType::Penalty | NodeType::RelNoad)
                )
            {
                let z = st.mem.new_penalty(pen)?;
                p.set_link(&mut st.mem, z.ptr());
                p = z.idx();
            }

            if st.mem[qn].noad_type == NodeType::RightNoad {
                t = NodeType::OpenNoad.to_u16();
            }
            r_type = t;
        }

        let nq = here.link(&st.mem);
        st.mem.free_node(here, s);
        q = nq;
    }
    Ok(TEMP_HEAD.link(&st.mem))
}

impl MemState {
    pub(super) fn stack_glue_into_box(
        &mut self,
        b: NodePtr<BoxNode>,
        min: Scaled,
        max: Scaled,
    ) -> Result<(), Abort> {
        let spec = self.new_spec(ZERO_GLUE)?;
        let node = &mut self[spec];
        node.width = min;
        node.stretch = max - min;
        let p = self.new_glue(spec)?;
        if self[b].is_hlist() {
            match self[b].list_ptr {
                None => self[b].list_ptr = p.ptr(),
                Some(q) => self.list_tail(q).set_link(self, p.ptr()),
            }
        } else {
            self[p].link = self[b].list_ptr;
            self[b].list_ptr = p.ptr();
            // XeTeX's `width(b):=width(p)` reads the glue's null leader pointer, `min_halfword` there (upstream bug 13).
            self[b].width = Scaled::from_raw(MIN_HALFWORD);
        }
        Ok(())
    }
}

/// The kern between the nucleus character and its font's `\skewchar`, which shifts an accent.
fn tfm_skew(st: &EngineState, nucleus: &Fetched) -> Result<Scaled, Abort> {
    let Some(CharTag::Lig(start)) = nucleus.i.map(|i| i.tag) else {
        return Ok(Scaled::ZERO);
    };
    let f = nucleus.f;
    let (mut a, mut step) = st.fonts.lig_kern_start(f, start)?;
    let skew_char = LigChar::from_raw(st.fonts.skew_char[f]);
    loop {
        match step.action_for(skew_char) {
            Some(LigKernAction::Kern(kern)) => return Ok(kern),
            Some(LigKernAction::Lig { .. }) => return Ok(Scaled::ZERO),
            None => {}
        }
        let Some(next) = step.next(a) else {
            return Ok(Scaled::ZERO);
        };
        a = next;
        step = st.fonts.lig_kern_step(f, a)?;
    }
}

/// Where an OpenType accent goes over its nucleus.
enum AccentAttach {
    /// The nucleus glyph's attachment point, from its left edge.
    At(Scaled),
    /// A TFM nucleus character: its skew from the centre.
    Skew(Scaled),
    /// Centred, for lack of attachment data.
    Centre,
}

fn compute_ot_math_accent_pos(
    st: &mut EngineState,
    mut p: NodePtr<AccentNoad>,
    style: MathStyle,
) -> Result<AccentAttach, Abort> {
    loop {
        match st.mem[p].base.nucleus.math_type {
            MATH_CHAR => {
                let nucleus = fetch(st, MathSlot::Nucleus(p.as_noad()), style.size())?;
                // XeTeX takes the nucleus for a native character and aborts (upstream bug 24).
                if !st.fonts.is_native_font(nucleus.f) {
                    return Ok(AccentAttach::Skew(tfm_skew(st, &nucleus)?));
                }
                let q = crate::fonts::new_native_character(st, nucleus.f, nucleus.c as u32)?;
                let g = st.mem.get_native_glyph(q, 0);
                let pos = st.fonts.get_ot_math_accent_pos(nucleus.f, i32::from(g));
                return Ok(if pos == INFINITE_DIMEN {
                    AccentAttach::Centre
                } else {
                    AccentAttach::At(pos)
                });
            }
            SUB_MLIST => {
                let r = st.mem[p].base.nucleus.info();
                match r.map(|ri| st.mem.classify_noad(ri)) {
                    Some(Ok(Some(rn))) => match rn.variant(&st.mem) {
                        NoadVariant::Accent(a) => p = a,
                        _ => return Ok(AccentAttach::Centre),
                    },
                    _ => return Ok(AccentAttach::Centre),
                }
            }
            _ => return Ok(AccentAttach::Centre),
        }
    }
}
