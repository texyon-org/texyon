//! Typesetting math formulas: `mlist_to_hlist` and the `make_*` noad handlers.

use crate::consts::{
    ACCENT_NOAD_SIZE, BOX_NODE_SIZE, COND_MATH_GLUE, EMPTY, EXPLICIT, FRACTION_NOAD_SIZE,
    GLYPH_NODE_SIZE, INFINITE_DIMEN, INF_PENALTY, MATH_CHAR, MATH_TEXT_CHAR, MAX_DIMEN,
    MEDIUM_NODE_SIZE, MU_GLUE, NOAD_SIZE, NORMAL, NULL_FONT, RADICAL_NOAD_SIZE, SCRIPT_SIZE,
    STYLE_NODE_SIZE, SUB_BOX, SUB_MLIST, TEXT_SIZE,
};
use crate::errors::Abort;
use crate::fonts::{CharInfo, CharTag, AAT_FONT_FLAG, OTGR_FONT_FLAG, SPACE_CODE, X_HEIGHT_CODE};
use crate::mem::{TEMP_HEAD, TEX_NULL, ZERO_GLUE};
use crate::mem_types::ListStateRecord;
use crate::node::{
    AnyBox, AnyNode, ChoiceNode, GlueNode, GlueSpecNode, KernNode, MathField, Noad, NodePtr,
    RuleDimen, RuleNode, WhatsitNode, WhatsitSubtype,
};
use crate::packaging::ADDITIONAL;
use crate::params::{DimenPar, GluePar, IntPar};
use crate::state::{EngineState, EqtbState, ErrorState, FontState, MemState, ScannerState};
use crate::units::{FontId, GlueOrder, MemPtr, NodeType, Scaled};

use super::delimiters::{build_opentype_assembly, rebox, var_delimiter};
use super::scripts::make_scripts;
use super::{BOTTOM_ACC, DEFAULT_CODE, LIMITS, SCRIPT_STYLE, TEXT_STYLE};

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
        p.set_subtype(self, 0);
        p.set_width(self, RuleDimen::Running);
        p.set_height(self, RuleDimen::Set(t));
        p.set_depth(self, RuleDimen::Set(Scaled::ZERO));
        Ok(p)
    }

    pub fn overbar(&mut self, b: AnyBox, k: Scaled, t: Scaled) -> Result<MemPtr, Abort> {
        let p = self.new_kern(k)?;
        p.set_link(self, b.ptr());
        let q = self.fraction_rule(t)?;
        q.set_link(self, p.ptr());
        let r = self.new_kern(t)?;
        r.set_link(self, q.ptr());
        Ok(r.ptr())
    }
}

impl ErrorState {
    /// The integer and fractional parts of the mu factor `m`, as `math_glue` splits it.
    fn mu_parts(&mut self, m: Scaled) -> (i32, i32) {
        let mut n = self.x_over_n(m.raw(), 65_536);
        let mut f = self.tex_remainder;
        if f < 0 {
            n -= 1;
            f += 65_536;
        }
        (n, f)
    }

    fn mu_mult(&mut self, (n, f): (i32, i32), v: Scaled) -> Scaled {
        let xnd = self.xn_over_d(v, f, 65_536);
        self.mult_and_add(n, v, xnd, MAX_DIMEN)
    }
}

fn math_glue(
    st: &mut EngineState,
    spec: NodePtr<GlueSpecNode>,
    m: Scaled,
) -> Result<NodePtr<GlueSpecNode>, Abort> {
    let mu = st.errors.mu_parts(m);
    let stretch_order = spec.stretch_order(&st.mem);
    let shrink_order = spec.shrink_order(&st.mem);
    let width = st.errors.mu_mult(mu, spec.width(&st.mem));
    let mut stretch = spec.stretch(&st.mem);
    if stretch_order == GlueOrder::Normal {
        stretch = st.errors.mu_mult(mu, stretch);
    }
    let mut shrink = spec.shrink(&st.mem);
    if shrink_order == GlueOrder::Normal {
        shrink = st.errors.mu_mult(mu, shrink);
    }

    let p = NodePtr::<GlueSpecNode>::alloc(&mut st.mem)?;
    p.set_width(&mut st.mem, width);
    p.set_stretch_order(&mut st.mem, stretch_order);
    p.set_stretch(&mut st.mem, stretch);
    p.set_shrink_order(&mut st.mem, shrink_order);
    p.set_shrink(&mut st.mem, shrink);
    Ok(p)
}

fn math_kern(st: &mut EngineState, p: NodePtr<KernNode>, m: Scaled) {
    if p.subtype(&st.mem) == MU_GLUE {
        let mu = st.errors.mu_parts(m);
        let width = st.errors.mu_mult(mu, p.width(&st.mem));
        p.set_width(&mut st.mem, width);
        p.set_subtype(&mut st.mem, EXPLICIT);
    }
}

impl ListStateRecord {
    pub fn flush_math(&mut self, mem: &mut MemState) -> Result<(), Abort> {
        mem.flush_node_list(self.head.link(mem))?;
        if let Some(n) = self.incompleat_noad() {
            mem.flush_node_list(n.ptr())?;
        }
        self.head.set_link(mem, MemPtr::NULL);
        self.tail = self.head;
        self.set_incompleat_noad(None);
        Ok(())
    }
}

pub(super) fn clean_box(
    st: &mut EngineState,
    p: NodePtr<MathField>,
    s: i16,
) -> Result<AnyBox, Abort> {
    let q = match p.math_type(&st.mem) {
        MATH_CHAR => {
            let noad = st.mem.new_noad()?;
            noad.nucleus().copy_from(&mut st.mem, p);
            sublist_to_hlist(st, noad.ptr(), s)?
        }
        SUB_BOX => p.info(&st.mem),
        SUB_MLIST => sublist_to_hlist(st, p.info(&st.mem), s)?,
        _ => st.mem.new_null_box()?.ptr(),
    };

    let lone_box = match q.as_idx() {
        Some(idx) if !st.mem.is_char_node(q) && q.link(&st.mem).is_null() => {
            match st.mem.classify_box(idx) {
                Ok(Some(b)) if b.shift_amount(&st.mem) == Scaled::ZERO => Some(b),
                _ => None,
            }
        }
        _ => None,
    };
    let x_box = match lone_box {
        Some(b) => b,
        None => crate::packaging::hpack(st, q, Scaled::ZERO, ADDITIONAL)?,
    };

    let head = x_box.list_ptr(&st.mem);
    if st.mem.is_char_node(head) {
        let r = head.link(&st.mem);
        if let Some(r_idx) = r.as_idx() {
            if r.link(&st.mem).is_null()
                && !st.mem.is_char_node(r)
                && matches!(st.mem.classify(r_idx), Ok(AnyNode::Kern(_)))
            {
                st.mem.free_node(r, MEDIUM_NODE_SIZE);
                head.set_link(&mut st.mem, MemPtr::NULL);
            }
        }
    }
    Ok(x_box)
}

/// Runs `mlist_to_hlist` on `mlist` at style `s` without penalties and returns
/// the resulting list, restoring the current style.
fn sublist_to_hlist(st: &mut EngineState, mlist: MemPtr, s: i16) -> Result<MemPtr, Abort> {
    st.scanner.cur_mlist = mlist;
    let save_style = st.scanner.cur_style;
    st.scanner.cur_style = s;
    st.scanner.mlist_penalties = false;
    mlist_to_hlist(st)?;
    let q = TEMP_HEAD.link(&st.mem);
    st.scanner.cur_style = save_style;
    set_cur_size_and_mu(st);
    Ok(q)
}

#[inline]
pub(super) fn set_cur_size_and_mu(st: &mut EngineState) {
    st.scanner.cur_size = if st.scanner.cur_style < SCRIPT_STYLE {
        TEXT_SIZE
    } else {
        SCRIPT_SIZE * ((i32::from(st.scanner.cur_style) - 2) / 2)
    };
    let mq = st.fonts.math_quad(&st.eqtb, st.scanner.cur_size);
    st.scanner.cur_mu = st.errors.x_over_n(mq, 18);
}

enum FetchOutcome {
    Ok,
    UndefinedFont { size: i32, fam: i32, c: i32 },
    CharMissing { f: FontId, c: i32 },
}

impl MemState {
    fn fetch(
        &mut self,
        fonts: &FontState,
        eqtb: &EqtbState,
        scanner: &mut ScannerState,
        a: NodePtr<MathField>,
    ) -> FetchOutcome {
        let size = scanner.cur_size;
        let fam = a.fam(self);
        scanner.cur_f = eqtb.math_font(fam + size);
        scanner.cur_c = a.full_char(self);

        if scanner.cur_f == NULL_FONT {
            let c = scanner.cur_c;
            scanner.cur_i = CharInfo::NULL;
            a.set_math_type(self, EMPTY);
            return FetchOutcome::UndefinedFont { size, fam, c };
        }

        let area = fonts.font_area[scanner.cur_f];
        if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
            scanner.cur_i = CharInfo::NULL;
        } else {
            if scanner.cur_c >= i32::from(fonts.font_bc[scanner.cur_f])
                && scanner.cur_c <= i32::from(fonts.font_ec[scanner.cur_f])
            {
                scanner.cur_i = fonts.char_info(scanner.cur_f, scanner.cur_c);
            } else {
                scanner.cur_i = CharInfo::NULL;
            }
            if !scanner.cur_i.exists() {
                let f = scanner.cur_f;
                let c = scanner.cur_c;
                a.set_math_type(self, EMPTY);
                scanner.cur_i = CharInfo::NULL;
                return FetchOutcome::CharMissing { f, c };
            }
        }
        FetchOutcome::Ok
    }
}

pub(super) fn fetch(st: &mut EngineState, a: NodePtr<MathField>) -> Result<(), Abort> {
    match st.mem.fetch(&st.fonts, &st.eqtb, &mut st.scanner, a) {
        FetchOutcome::Ok => Ok(()),
        FetchOutcome::UndefinedFont { size, fam, c } => {
            crate::output::error_here_with_diagnostic(st, "");
            crate::output::print_size(st, size);
            crate::output::print_char(st, i32::from(b' '));
            crate::output::print_int(st, fam);
            crate::output::print(st, " is undefined (character ");
            crate::output::print(st, c);
            crate::output::print_char(st, i32::from(b')'));
            crate::output::capture_to_diagnostic(st, None);
            st.errors.help(&[
                "Somewhere in the math formula just ended, you used the",
                "stated character from an undefined font family. For example,",
                r"plain TeX doesn't allow \it or \sl in subscripts. Proceed,",
                "and I'll try to forget that I needed that character.",
            ]);
            crate::errors::error(st)
        }
        FetchOutcome::CharMissing { f, c } => crate::output::char_warning(st, f, c),
    }
}

impl MemState {
    fn set_nucleus_box(&mut self, q: NodePtr<Noad>, b: AnyBox) {
        q.nucleus().set_info(self, b.ptr());
        q.nucleus().set_math_type(self, SUB_BOX);
    }
}

fn make_over(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), Abort> {
    let x = clean_box(st, q.nucleus(), 2 * (st.scanner.cur_style / 2) + 1)?;
    let rule_thickness = st
        .fonts
        .default_rule_thickness(&st.eqtb, st.scanner.cur_size);
    let p = st.mem.overbar(x, rule_thickness * 3, rule_thickness)?;
    let x = crate::packaging::vpackage(st, p, Scaled::ZERO, ADDITIONAL, MAX_DIMEN)?;
    st.mem.set_nucleus_box(q, x);
    Ok(())
}

fn make_under(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), Abort> {
    let x = clean_box(st, q.nucleus(), st.scanner.cur_style)?;
    let rule_thickness = st
        .fonts
        .default_rule_thickness(&st.eqtb, st.scanner.cur_size);
    let kern = st.mem.new_kern(rule_thickness * 3)?;
    x.set_link(&mut st.mem, kern.ptr());
    let rule = st.mem.fraction_rule(rule_thickness)?;
    kern.set_link(&mut st.mem, rule.ptr());

    let y = crate::packaging::vpackage(st, x.ptr(), Scaled::ZERO, ADDITIONAL, MAX_DIMEN)?;
    let mem = &mut st.mem;
    let delta = y.height(mem) + y.depth(mem) + rule_thickness;
    y.set_height(mem, x.height(mem));
    y.set_depth(mem, delta - y.height(mem));
    mem.set_nucleus_box(q, y);
    Ok(())
}

fn make_vcenter(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), Abort> {
    let axis_height = st.fonts.axis_height(&st.eqtb, st.scanner.cur_size);
    let mem = &mut st.mem;
    let Ok(Some(v @ AnyBox::VList(_))) = mem.classify_box(q.nucleus().info(mem)) else {
        return Err(Abort::Confusion("vcenter"));
    };
    let delta = v.height(mem) + v.depth(mem);
    v.set_height(mem, axis_height + delta.half());
    v.set_depth(mem, delta - v.height(mem));
    Ok(())
}

fn make_radical(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), Abort> {
    let f = st
        .eqtb
        .math_font(q.left_delimiter().small(&st.mem).0 + st.scanner.cur_size);
    let is_ot_math = st.fonts.font_is_ot_math(f);
    let rule_thickness = if is_ot_math {
        st.fonts.get_ot_math_constant(f, RADICALRULETHICKNESS)
    } else {
        st.fonts
            .default_rule_thickness(&st.eqtb, st.scanner.cur_size)
    };

    let clean_style = 2 * (st.scanner.cur_style / 2) + 1;
    let x = clean_box(st, q.nucleus(), clean_style)?;
    let cur_style = st.scanner.cur_style;
    let cur_size = st.scanner.cur_size;
    let mut clr = if is_ot_math {
        if cur_style < TEXT_STYLE {
            st.fonts
                .get_ot_math_constant(f, RADICALDISPLAYSTYLEVERTICALGAP)
        } else {
            st.fonts.get_ot_math_constant(f, RADICALVERTICALGAP)
        }
    } else if cur_style < TEXT_STYLE {
        rule_thickness + st.fonts.math_x_height(&st.eqtb, cur_size).abs() / 4
    } else {
        rule_thickness + rule_thickness.abs() / 4
    };

    let delimiter_height = x.height(&st.mem) + x.depth(&st.mem) + clr + rule_thickness;
    let y = var_delimiter(st, q.left_delimiter(), cur_size, delimiter_height)?;
    let mem = &mut st.mem;
    if is_ot_math {
        y.set_depth(mem, y.depth(mem) + y.height(mem) - rule_thickness);
        y.set_height(mem, rule_thickness);
    }
    let delta = y.depth(mem) - (x.height(mem) + x.depth(mem) + clr);
    if delta > Scaled::ZERO {
        clr += delta.half();
    }
    y.set_shift_amount(mem, -(x.height(mem) + clr));
    let p = mem.overbar(x, clr, y.height(mem))?;
    let overbar_box = crate::packaging::vpackage(st, p, Scaled::ZERO, ADDITIONAL, MAX_DIMEN)?;
    y.set_link(&mut st.mem, overbar_box.ptr());
    let hbox = crate::packaging::hpack(st, y.ptr(), Scaled::ZERO, ADDITIONAL)?;
    st.mem.set_nucleus_box(q, hbox);
    Ok(())
}

struct FractionAssembly {
    x: AnyBox,
    z: AnyBox,
    rule_thickness: Scaled,
    shift_up: Scaled,
    shift_down: Scaled,
    delta: Scaled,
    axis_height: Scaled,
}

impl MemState {
    fn make_fraction(&mut self, assembly: FractionAssembly) -> Result<AnyBox, Abort> {
        let vb = AnyBox::VList(self.new_null_box()?.into_vlist(self));
        let xb = assembly.x;
        let zb = assembly.z;
        vb.set_height(self, assembly.shift_up + xb.height(self));
        vb.set_depth(self, zb.depth(self) + assembly.shift_down);
        vb.set_width(self, xb.width(self));

        let p: NodePtr<KernNode>;
        if assembly.rule_thickness == Scaled::ZERO {
            let kw = (assembly.shift_up - xb.depth(self)) - (zb.height(self) - assembly.shift_down);
            p = self.new_kern(kw)?;
            p.set_link(self, assembly.z.ptr());
        } else {
            let y = self.fraction_rule(assembly.rule_thickness)?;
            let p1w =
                (assembly.axis_height - assembly.delta) - (zb.height(self) - assembly.shift_down);
            let p1 = self.new_kern(p1w)?;
            y.set_link(self, p1.ptr());
            p1.set_link(self, assembly.z.ptr());
            let pw = (assembly.shift_up - xb.depth(self)) - (assembly.axis_height + assembly.delta);
            p = self.new_kern(pw)?;
            p.set_link(self, y.ptr());
        }

        xb.set_link(self, p.ptr());
        vb.set_list_ptr(self, assembly.x.ptr());
        Ok(vb)
    }

    pub(super) fn glyph_whatsit(&self, p: MemPtr) -> Option<NodePtr<WhatsitNode>> {
        match self.classify_non_char(p)? {
            AnyNode::Whatsit(w) if w.whatsit_subtype(self) == Some(WhatsitSubtype::Glyph) => {
                Some(w)
            }
            _ => None,
        }
    }
}

fn make_fraction(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), Abort> {
    if q.thickness(&st.mem) == DEFAULT_CODE {
        let drt = st
            .fonts
            .default_rule_thickness(&st.eqtb, st.scanner.cur_size);
        q.set_thickness(&mut st.mem, drt);
    }

    let cs = st.scanner.cur_style;
    let mut x = clean_box(st, q.numerator(), cs + 2 - 2 * (cs / 6))?;
    let cs = st.scanner.cur_style;
    let mut z = clean_box(st, q.denominator(), 2 * (cs / 2) + 3 - 2 * (cs / 6))?;

    if x.width(&st.mem) < z.width(&st.mem) {
        let w = z.width(&st.mem);
        x = rebox(st, x, w)?;
    } else {
        let w = x.width(&st.mem);
        z = rebox(st, z, w)?;
    }

    let cur_style = st.scanner.cur_style;
    let cur_size = st.scanner.cur_size;
    let cur_f = st.scanner.cur_f;
    let (mut shift_up, mut shift_down) = if cur_style < TEXT_STYLE {
        (
            st.fonts.num1(&st.eqtb, cur_size),
            st.fonts.denom1(&st.eqtb, cur_size),
        )
    } else {
        (
            if q.thickness(&st.mem) != Scaled::ZERO {
                st.fonts.num2(&st.eqtb, cur_size)
            } else {
                st.fonts.num3(&st.eqtb, cur_size)
            },
            st.fonts.denom2(&st.eqtb, cur_size),
        )
    };

    let delta;
    if q.thickness(&st.mem) == Scaled::ZERO {
        let clr = if st.fonts.font_is_ot_math(cur_f) {
            if cur_style < TEXT_STYLE {
                st.fonts
                    .get_ot_math_constant(cur_f, STACKDISPLAYSTYLEGAPMIN)
            } else {
                st.fonts.get_ot_math_constant(cur_f, STACKGAPMIN)
            }
        } else if cur_style < TEXT_STYLE {
            st.fonts.default_rule_thickness(&st.eqtb, cur_size) * 7
        } else {
            st.fonts.default_rule_thickness(&st.eqtb, cur_size) * 3
        };

        delta = (clr - ((shift_up - x.depth(&st.mem)) - (z.height(&st.mem) - shift_down))).half();
        if delta > Scaled::ZERO {
            shift_up += delta;
            shift_down += delta;
        }
    } else {
        let axis_height = st.fonts.axis_height(&st.eqtb, cur_size);
        let (delta1, delta2) = if st.fonts.font_is_ot_math(cur_f) {
            delta = q.thickness(&st.mem).half();
            let clr_num = if cur_style < TEXT_STYLE {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONNUMDISPLAYSTYLEGAPMIN)
            } else {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONNUMERATORGAPMIN)
            };
            let delta1 = clr_num - ((shift_up - x.depth(&st.mem)) - (axis_height + delta));

            let clr_den = if cur_style < TEXT_STYLE {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONDENOMDISPLAYSTYLEGAPMIN)
            } else {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONDENOMINATORGAPMIN)
            };
            let delta2 = clr_den - ((axis_height - delta) - (z.height(&st.mem) - shift_down));
            (delta1, delta2)
        } else {
            let clr = if cur_style < TEXT_STYLE {
                q.thickness(&st.mem) * 3
            } else {
                q.thickness(&st.mem)
            };
            delta = q.thickness(&st.mem).half();
            (
                clr - ((shift_up - x.depth(&st.mem)) - (axis_height + delta)),
                clr - ((axis_height - delta) - (z.height(&st.mem) - shift_down)),
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
    let rule_thickness = q.thickness(&st.mem);
    let v = st.mem.make_fraction(FractionAssembly {
        x,
        z,
        rule_thickness,
        shift_up,
        shift_down,
        delta,
        axis_height,
    })?;

    let delimiter_size = if cur_style < TEXT_STYLE {
        st.fonts.delim1(&st.eqtb, cur_size)
    } else {
        st.fonts.delim2(&st.eqtb, cur_size)
    };

    let left_delim = var_delimiter(st, q.left_delimiter(), cur_size, delimiter_size)?;
    let right_delim = var_delimiter(st, q.right_delimiter(), cur_size, delimiter_size)?;
    left_delim.set_link(&mut st.mem, v.ptr());
    v.set_link(&mut st.mem, right_delim.ptr());
    let hbox = crate::packaging::hpack(st, left_delim.ptr(), Scaled::ZERO, ADDITIONAL)?;
    q.set_new_hlist(&mut st.mem, hbox.ptr());
    Ok(())
}

/// Returns the italic correction `delta`.
fn make_op(st: &mut EngineState, q: NodePtr<Noad>) -> Result<Scaled, Abort> {
    let mut delta = Scaled::ZERO;

    if q.subtype(&st.mem) == NORMAL && st.scanner.cur_style < TEXT_STYLE {
        q.set_subtype(&mut st.mem, LIMITS);
    }

    if q.nucleus().math_type(&st.mem) == MATH_CHAR {
        fetch(st, q.nucleus())?;

        let f = st.scanner.cur_f;
        if !st.fonts.font_uses_opentype(f) {
            if st.scanner.cur_style < TEXT_STYLE && st.scanner.cur_i.tag() == CharTag::List {
                let c = st.scanner.cur_i.remainder();
                let i_info = st.fonts.char_info(f, i32::from(c));
                if i_info.exists() {
                    st.scanner.cur_c = i32::from(c);
                    st.scanner.cur_i = i_info;
                    q.nucleus().set_character(&mut st.mem, c);
                }
            }
            delta = st.fonts.char_italic(f, st.scanner.cur_i);
        }

        let x = clean_box(st, q.nucleus(), st.scanner.cur_style)?;

        let cur_f = st.scanner.cur_f;
        if st.fonts.font_is_ot_math(cur_f) {
            if let Some(glyph) = st.mem.glyph_whatsit(x.list_ptr(&st.mem)) {
                let (width, height, depth) = 'found: {
                    if st.scanner.cur_style < TEXT_STYLE {
                        let mut h1 = st
                            .fonts
                            .get_ot_math_constant(cur_f, DISPLAYOPERATORMINHEIGHT);
                        let size_h =
                            f64::from(glyph.height(&st.mem) + glyph.depth(&st.mem)) * 5.0 / 4.0;
                        if f64::from(h1) < size_h {
                            h1 = Scaled::from_raw(size_h as i32);
                        }
                        let c_glyph = i32::from(glyph.native_glyph(&st.mem));
                        let mut n: i32 = 0;
                        let mut h2 = Scaled::ZERO;
                        loop {
                            let g = st
                                .fonts
                                .get_ot_math_variant(cur_f, c_glyph, n, &mut h2, false);
                            if h2 > Scaled::ZERO {
                                glyph.set_native_glyph(&mut st.mem, g as u16);
                                st.fonts.measure_glyph(&mut st.mem, glyph)?;
                            }
                            n += 1;
                            if h2 < Scaled::ZERO || h2 >= h1 {
                                break;
                            }
                        }
                        if h2 < Scaled::ZERO {
                            if let Some(assembly) = st.fonts.get_ot_assembly(cur_f, c_glyph, false)
                            {
                                st.mem.free_node(glyph.ptr(), GLYPH_NODE_SIZE);
                                let b = build_opentype_assembly(st, cur_f, &assembly, h1, false)?;
                                x.set_list_ptr(&mut st.mem, b.ptr());
                                delta = Scaled::ZERO;
                                break 'found (
                                    b.width(&st.mem),
                                    b.height(&st.mem),
                                    b.depth(&st.mem),
                                );
                            }
                        } else {
                            st.fonts.measure_glyph(&mut st.mem, glyph)?;
                        }
                    }
                    delta = st
                        .fonts
                        .get_ot_math_ital_corr(cur_f, i32::from(glyph.native_glyph(&st.mem)));
                    (
                        glyph.width(&st.mem),
                        glyph.height(&st.mem),
                        glyph.depth(&st.mem),
                    )
                };
                x.set_width(&mut st.mem, width);
                x.set_height(&mut st.mem, height);
                x.set_depth(&mut st.mem, depth);
            }
        }

        let axis_height = st.fonts.axis_height(&st.eqtb, st.scanner.cur_size);
        let mem = &mut st.mem;
        if q.subscr().math_type(mem) != EMPTY && q.subtype(mem) != LIMITS {
            x.set_width(mem, x.width(mem) - delta);
        }
        let shift = (x.height(mem) - x.depth(mem)).half() - axis_height;
        x.set_shift_amount(mem, shift);
        mem.set_nucleus_box(q, x);
    }

    let save_f = st.scanner.cur_f;
    if q.subtype(&st.mem) == LIMITS {
        let cs = i32::from(st.scanner.cur_style);
        let x = clean_box(st, q.supscr(), (2 * (cs / 4) + 4 + (cs % 2)) as i16)?;
        let y = clean_box(st, q.nucleus(), st.scanner.cur_style)?;
        let z = clean_box(st, q.subscr(), (2 * (cs / 4) + 5) as i16)?;

        let v = AnyBox::VList(st.mem.new_null_box()?.into_vlist(&mut st.mem));
        let w = y.width(&st.mem).max(x.width(&st.mem)).max(z.width(&st.mem));
        v.set_width(&mut st.mem, w);

        let x = rebox(st, x, w)?;
        let y = rebox(st, y, w)?;
        let z = rebox(st, z, w)?;

        let size = st.scanner.cur_size;
        let bos1 = st.fonts.big_op_spacing1(&st.eqtb, size);
        let bos2 = st.fonts.big_op_spacing2(&st.eqtb, size);
        let bos3 = st.fonts.big_op_spacing3(&st.eqtb, size);
        let bos4 = st.fonts.big_op_spacing4(&st.eqtb, size);
        let bos5 = st.fonts.big_op_spacing5(&st.eqtb, size);

        let mem = &mut st.mem;
        let half_delta = delta.half();
        x.set_shift_amount(mem, half_delta);
        z.set_shift_amount(mem, -half_delta);
        v.set_height(mem, y.height(mem));
        v.set_depth(mem, y.depth(mem));
        st.scanner.cur_f = save_f;

        if q.supscr().math_type(mem) == EMPTY {
            mem.free_node(x.ptr(), BOX_NODE_SIZE);
            v.set_list_ptr(mem, y.ptr());
        } else {
            let shift_up = (bos3 - x.depth(mem)).max(bos1);
            let p = mem.new_kern(shift_up)?;
            p.set_link(mem, y.ptr());
            x.set_link(mem, p.ptr());
            let p = mem.new_kern(bos5)?;
            p.set_link(mem, x.ptr());
            v.set_list_ptr(mem, p.ptr());
            let height = v.height(mem) + bos5 + x.height(mem) + x.depth(mem) + shift_up;
            v.set_height(mem, height);
        }

        if q.subscr().math_type(mem) == EMPTY {
            mem.free_node(z.ptr(), BOX_NODE_SIZE);
        } else {
            let shift_down = (bos4 - z.height(mem)).max(bos2);
            let p = mem.new_kern(shift_down)?;
            y.set_link(mem, p.ptr());
            p.set_link(mem, z.ptr());
            let p = mem.new_kern(bos5)?;
            z.set_link(mem, p.ptr());
            let depth = v.depth(mem) + bos5 + z.height(mem) + z.depth(mem) + shift_down;
            v.set_depth(mem, depth);
        }

        q.set_new_hlist(mem, v.ptr());
    }

    Ok(delta)
}

enum MakeOrdStep {
    Return,
    Restart,
}

impl MemState {
    fn make_ord_body(
        &mut self,
        fonts: &FontState,
        scanner: &mut ScannerState,
        q: NodePtr<Noad>,
        p: NodePtr<Noad>,
    ) -> Result<MakeOrdStep, Abort> {
        scanner.cur_c = i32::from(p.nucleus().character(self));
        let (mut a, mut step) = fonts.lig_kern_start(scanner.cur_f, scanner.cur_i);
        loop {
            if step.matches(scanner.cur_c) {
                if step.is_kern() {
                    let kern_width = fonts.char_kern(scanner.cur_f, step);
                    let p_kern = self.new_kern(kern_width)?;
                    let link_q = q.link(self);
                    p_kern.set_link(self, link_q);
                    q.set_link(self, p_kern.ptr());
                    return Ok(MakeOrdStep::Return);
                }
                let op = step.lig_op();
                match op {
                    1 | 5 => {
                        q.nucleus().set_character(self, step.lig_char());
                    }
                    2 | 6 => {
                        p.nucleus().set_character(self, step.lig_char());
                    }
                    3 | 7 | 11 => {
                        let r = self.new_noad()?;
                        r.nucleus().set_character(self, step.lig_char());
                        r.nucleus()
                            .set_plane_and_fam(self, q.nucleus().plane_and_fam(self) % 256);
                        q.set_link(self, r.ptr());
                        r.set_link(self, p.ptr());
                        if op < 11 {
                            r.nucleus().set_math_type(self, MATH_CHAR);
                        } else {
                            r.nucleus().set_math_type(self, MATH_TEXT_CHAR);
                        }
                    }
                    _ => {
                        let link_p = p.link(self);
                        q.set_link(self, link_p);
                        q.nucleus().set_character(self, step.lig_char());
                        q.subscr().copy_from(self, p.subscr());
                        q.supscr().copy_from(self, p.supscr());
                        self.free_node(p.ptr(), NOAD_SIZE);
                    }
                }
                if op > 3 {
                    return Ok(MakeOrdStep::Return);
                }
                q.nucleus().set_math_type(self, MATH_CHAR);
                return Ok(MakeOrdStep::Restart);
            }
            if step.is_last() {
                return Ok(MakeOrdStep::Return);
            }
            a = step.next_index(a);
            step = fonts.lig_kern_step(a);
        }
    }
}

fn make_ord(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), Abort> {
    'restart: loop {
        if q.subscr().math_type(&st.mem) != EMPTY {
            return Ok(());
        }
        if q.supscr().math_type(&st.mem) != EMPTY {
            return Ok(());
        }
        if q.nucleus().math_type(&st.mem) != MATH_CHAR {
            return Ok(());
        }
        let p = q.link(&st.mem);
        let Some(p) = p.as_idx() else {
            return Ok(());
        };
        let p_node = match st.mem.classify_noad(p) {
            Ok(Some(pn))
                if (NodeType::OrdNoad.as_raw()..=NodeType::PunctNoad.as_raw())
                    .contains(&pn.noad_type(&st.mem).as_raw()) =>
            {
                pn
            }
            _ => return Ok(()),
        };
        if p_node.nucleus().math_type(&st.mem) != MATH_CHAR {
            return Ok(());
        }
        if p_node.nucleus().plane_and_fam(&st.mem) % 256 != q.nucleus().plane_and_fam(&st.mem) % 256
        {
            return Ok(());
        }

        q.nucleus().set_math_type(&mut st.mem, MATH_TEXT_CHAR);
        fetch(st, q.nucleus())?;
        if st.scanner.cur_i.tag() != CharTag::Lig {
            return Ok(());
        }

        match st
            .mem
            .make_ord_body(&st.fonts, &mut st.scanner, q, p_node)?
        {
            MakeOrdStep::Return => return Ok(()),
            MakeOrdStep::Restart => continue 'restart,
        }
    }
}

fn make_math_accent(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), Abort> {
    fetch(st, q.accent_chr())?;
    let f = st.scanner.cur_f;
    let mut c = st.scanner.cur_c;
    let area = st.fonts.font_area[f];
    let kind = q.subtype(&st.mem);
    let bottom_accent = kind == BOTTOM_ACC || kind == BOTTOM_ACC + 1;
    let is_native = area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG;

    let mut s = Scaled::ZERO;
    let mut x: AnyBox;
    if is_native {
        if !bottom_accent {
            s = compute_ot_math_accent_pos(st, q)?;
        }
        x = clean_box(st, q.nucleus(), 2 * (st.scanner.cur_style / 2) + 1)?;
    } else if st.scanner.cur_i.exists() {
        let mut acc_i = st.scanner.cur_i;
        if q.nucleus().math_type(&st.mem) == MATH_CHAR {
            fetch(st, q.nucleus())?;
            if st.scanner.cur_i.tag() == CharTag::Lig {
                let cur_f = st.scanner.cur_f;
                let (mut a, mut step) = st.fonts.lig_kern_start(cur_f, st.scanner.cur_i);
                let skew_char = st.fonts.skew_char[cur_f];
                s = loop {
                    if i32::from(step.next_char()) == skew_char {
                        if step.is_kern() && !step.is_redirect() {
                            break st.fonts.char_kern(cur_f, step);
                        }
                        break Scaled::ZERO;
                    }
                    if step.is_last() {
                        break Scaled::ZERO;
                    }
                    a = step.next_index(a);
                    step = st.fonts.lig_kern_step(a);
                };
            }
        }
        x = clean_box(st, q.nucleus(), 2 * (st.scanner.cur_style / 2) + 1)?;
        let w = x.width(&st.mem);
        while acc_i.tag() == CharTag::List {
            let larger = i32::from(acc_i.remainder());
            acc_i = st.fonts.char_info(f, larger);
            if !acc_i.exists() || st.fonts.char_width(f, acc_i) > w {
                break;
            }
            c = larger;
        }
    } else {
        return Ok(());
    }
    let w = x.width(&st.mem);
    let mut h = x.height(&st.mem);

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

    if (q.supscr().math_type(&st.mem) != EMPTY || q.subscr().math_type(&st.mem) != EMPTY)
        && q.nucleus().math_type(&st.mem) == MATH_CHAR
    {
        st.mem.flush_node_list(x.ptr())?;
        let base = st.mem.new_noad()?;
        let mem = &mut st.mem;
        base.nucleus().copy_from(mem, q.nucleus());
        base.supscr().copy_from(mem, q.supscr());
        base.subscr().copy_from(mem, q.subscr());
        for script in [q.supscr(), q.subscr()] {
            script.set_info(mem, MemPtr::default());
            script.set_math_type(mem, EMPTY);
        }
        q.nucleus().set_math_type(mem, SUB_MLIST);
        q.nucleus().set_info(mem, base.ptr());
        x = clean_box(st, q.nucleus(), st.scanner.cur_style)?;
        delta += x.height(&st.mem) - h;
        h = x.height(&st.mem);
    }

    let y;
    if is_native {
        let old = crate::fonts::new_native_character(st, f, c as u32)?;
        y = st.mem.native_char_box(old)?;
        let glyph =
            NodePtr::<WhatsitNode>::alloc(&mut st.mem, GLYPH_NODE_SIZE, WhatsitSubtype::Glyph)?;
        glyph.set_native_font(&mut st.mem, f);
        let glyph_id = st.mem.get_native_glyph(old, 0);
        glyph.set_native_glyph(&mut st.mem, glyph_id);
        st.fonts.measure_glyph(&mut st.mem, glyph)?;
        old.free_native(&mut st.mem);
        y.set_list_ptr(&mut st.mem, glyph.ptr());

        let mut assembly_box = None;
        if !kind.is_multiple_of(2) {
            st.fonts.measure_glyph(&mut st.mem, glyph)?;
        } else {
            let c_glyph = i32::from(glyph.native_glyph(&st.mem));
            let mut n: i32 = 0;
            let mut w2 = Scaled::ZERO;
            loop {
                let g = st.fonts.get_ot_math_variant(f, c_glyph, n, &mut w2, true);
                if w2 > Scaled::ZERO && w2 <= w {
                    glyph.set_native_glyph(&mut st.mem, g as u16);
                    st.fonts.measure_glyph(&mut st.mem, glyph)?;
                    n += 1;
                }
                if w2 < Scaled::ZERO || w2 >= w {
                    break;
                }
            }
            if w2 < Scaled::ZERO {
                if let Some(assembly) = st.fonts.get_ot_assembly(f, c_glyph, true) {
                    st.mem.free_node(glyph.ptr(), GLYPH_NODE_SIZE);
                    let b = build_opentype_assembly(st, f, &assembly, w, true)?;
                    y.set_list_ptr(&mut st.mem, b.ptr());
                    assembly_box = Some(b);
                }
            } else {
                st.fonts.measure_glyph(&mut st.mem, glyph)?;
            }
        }

        let mem = &mut st.mem;
        let (gw, gh, gd) = match assembly_box {
            Some(b) => (b.width(mem), b.height(mem), b.depth(mem)),
            None => (glyph.width(mem), glyph.height(mem), glyph.depth(mem)),
        };
        y.set_width(mem, gw);
        y.set_height(mem, gh);
        y.set_depth(mem, gd);
        if bottom_accent {
            if y.height(mem) < Scaled::ZERO {
                y.set_height(mem, Scaled::ZERO);
            }
        } else if y.depth(mem) < Scaled::ZERO {
            y.set_depth(mem, Scaled::ZERO);
        }

        let attach = match assembly_box {
            None => st
                .fonts
                .get_ot_math_accent_pos(f, i32::from(glyph.native_glyph(mem))),
            Some(_) => INFINITE_DIMEN,
        };
        let sa = if attach == INFINITE_DIMEN {
            y.width(mem).half()
        } else {
            attach
        };
        if bottom_accent || s == INFINITE_DIMEN {
            s = w.half();
        }
        y.set_shift_amount(mem, s - sa);
    } else {
        y = st.mem.char_box_tfm(&mut st.fonts, f, c as u16)?;
        let shift = s + (w - y.width(&st.mem)).half();
        y.set_shift_amount(&mut st.mem, shift);
    }

    y.set_width(&mut st.mem, Scaled::ZERO);
    let v = if bottom_accent {
        x.set_link(&mut st.mem, y.ptr());
        let v = crate::packaging::vpackage(st, x.ptr(), Scaled::ZERO, ADDITIONAL, MAX_DIMEN)?;
        let v_height = v.height(&st.mem);
        v.set_shift_amount(&mut st.mem, -(h - v_height));
        v
    } else {
        let kern = st.mem.new_kern(-delta)?;
        kern.set_link(&mut st.mem, x.ptr());
        y.set_link(&mut st.mem, kern.ptr());
        let v = crate::packaging::vpackage(st, y.ptr(), Scaled::ZERO, ADDITIONAL, MAX_DIMEN)?;
        let mem = &mut st.mem;
        if v.height(mem) < h {
            let kern = mem.new_kern(h - v.height(mem))?;
            kern.set_link(mem, v.list_ptr(mem));
            v.set_list_ptr(mem, kern.ptr());
            v.set_height(mem, h);
        }
        v
    };
    let x_width = x.width(&st.mem);
    v.set_width(&mut st.mem, x_width);
    st.mem.set_nucleus_box(q, v);
    Ok(())
}

fn make_left_right(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    style: i16,
    max_d: Scaled,
    max_h: Scaled,
) -> Result<NodeType, Abort> {
    st.scanner.cur_style = style;
    set_cur_size_and_mu(st);
    let size = st.scanner.cur_size;
    let axis_height = st.fonts.axis_height(&st.eqtb, size);
    let delimiter_factor = st.eqtb.int_par(IntPar::DelimiterFactor);
    let delimiter_shortfall = st.eqtb.dimen_par(DimenPar::DelimiterShortfall);
    let delta2 = max_d + axis_height;
    let delta1 = (max_h + max_d - delta2).max(delta2);
    let delta = ((delta1 / 500) * delimiter_factor).max(delta1 + delta1 - delimiter_shortfall);
    let delimiter = var_delimiter(st, q.delimiter(), size, delta)?;
    q.set_new_hlist(&mut st.mem, delimiter.ptr());
    Ok(if q.noad_type(&st.mem) == NodeType::LeftNoad {
        NodeType::OpenNoad
    } else {
        NodeType::CloseNoad
    })
}

impl MemState {
    fn mlist_choice_node(
        &mut self,
        choice: NodePtr<ChoiceNode>,
        cur_style: i16,
    ) -> Result<(), Abort> {
        let kept = match cur_style / 2 {
            0 => {
                let p = choice.display_mlist(self);
                choice.set_display_mlist(self, MemPtr::NULL);
                p
            }
            1 => {
                let p = choice.text_mlist(self);
                choice.set_text_mlist(self, MemPtr::NULL);
                p
            }
            2 => {
                let p = choice.script_mlist(self);
                choice.set_script_mlist(self, MemPtr::NULL);
                p
            }
            3 => {
                let p = choice.script_script_mlist(self);
                choice.set_script_script_mlist(self, MemPtr::NULL);
                p
            }
            _ => MemPtr::NULL,
        };
        self.flush_node_list(choice.display_mlist(self))?;
        self.flush_node_list(choice.text_mlist(self))?;
        self.flush_node_list(choice.script_mlist(self))?;
        self.flush_node_list(choice.script_script_mlist(self))?;
        let style = choice.into_style(self, cur_style as u16);
        if !kept.is_null() {
            let z = style.link(self);
            style.set_link(self, kept);
            let mut p = kept;
            while !p.link(self).is_null() {
                p = p.link(self);
            }
            p.set_link(self, z);
        }
        Ok(())
    }

    fn mlist_track_rule(&self, rule: NodePtr<RuleNode>, max_h: &mut Scaled, max_d: &mut Scaled) {
        *max_h = (*max_h).max(rule.height(self).to_scaled());
        *max_d = (*max_d).max(rule.depth(self).to_scaled());
    }

    fn mlist_track_box(&mut self, z: AnyBox, max_h: &mut Scaled, max_d: &mut Scaled) {
        *max_h = (*max_h).max(z.height(self));
        *max_d = (*max_d).max(z.depth(self));
        self.free_node(z.ptr(), BOX_NODE_SIZE);
    }

    fn mlist_cond_math_glue(
        &mut self,
        glue: NodePtr<GlueNode>,
        cur_size: i32,
    ) -> Result<(), Abort> {
        if cur_size == TEXT_SIZE {
            return Ok(());
        }

        let p = glue.link(self);
        let Some(p_idx) = p.as_idx() else {
            return Ok(());
        };
        if matches!(
            self.classify(p_idx),
            Ok(AnyNode::Glue(_) | AnyNode::Kern(_))
        ) {
            glue.set_link(self, p.link(self));
            p.set_link(self, MemPtr::NULL);
            self.flush_node_list(p)?;
        }
        Ok(())
    }

    /// Moves `node` onto the output list after `tail`, returning it and its old successor.
    fn mlist_move_output_node(&mut self, tail: MemPtr, node: AnyNode) -> (MemPtr, MemPtr) {
        let q = MemPtr::from_idx(node.idx());
        tail.set_link(self, q);
        let next = q.link(self);
        q.set_link(self, MemPtr::NULL);
        (q, next)
    }
}

fn mlist_advance(st: &mut EngineState, qn: NodePtr<Noad>, style: i16) -> (u16, MemPtr) {
    let mut r_type = qn.noad_type(&st.mem).as_raw();
    let next = qn.link(&st.mem);
    if r_type == NodeType::RightNoad.as_raw() {
        r_type = NodeType::LeftNoad.as_raw();
        st.scanner.cur_style = style;
        set_cur_size_and_mu(st);
    }
    (r_type, next)
}

/// Translates the nucleus of `qn` into its `new_hlist` and attaches its scripts.
fn mlist_nucleus(st: &mut EngineState, qn: NodePtr<Noad>, mut delta: Scaled) -> Result<(), Abort> {
    let p = match qn.nucleus().math_type(&st.mem) {
        MATH_CHAR | MATH_TEXT_CHAR => {
            fetch(st, qn.nucleus())?;
            let cur_f = st.scanner.cur_f;
            let area = st.fonts.font_area[cur_f];
            let p = if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
                let z = crate::fonts::new_native_character(st, cur_f, st.scanner.cur_c as u32)?;
                let glyph = NodePtr::<WhatsitNode>::alloc(
                    &mut st.mem,
                    GLYPH_NODE_SIZE,
                    WhatsitSubtype::Glyph,
                )?;
                glyph.set_native_font(&mut st.mem, cur_f);
                let z_glyph = st.mem.get_native_glyph(z, 0);
                glyph.set_native_glyph(&mut st.mem, z_glyph);
                st.fonts.measure_glyph(&mut st.mem, glyph)?;
                z.free_native(&mut st.mem);
                delta = st
                    .fonts
                    .get_ot_math_ital_corr(cur_f, i32::from(glyph.native_glyph(&st.mem)));
                if qn.nucleus().math_type(&st.mem) == MATH_TEXT_CHAR
                    && !st.fonts.font_is_ot_math(cur_f)
                {
                    delta = Scaled::ZERO;
                }
                glyph.ptr()
            } else if st.scanner.cur_i.exists() {
                delta = st.fonts.char_italic(cur_f, st.scanner.cur_i);
                let cc = st.scanner.cur_c as u16;
                let p = crate::main_control::new_character(st, cur_f, cc)?;
                let p = p.map_or(MemPtr::NULL, crate::main_control::NewChar::ptr);
                if qn.nucleus().math_type(&st.mem) == MATH_TEXT_CHAR
                    && st.fonts.param(cur_f, SPACE_CODE) != Scaled::ZERO
                {
                    delta = Scaled::ZERO;
                }
                p
            } else {
                MemPtr::NULL
            };
            if !p.is_null() && qn.subscr().math_type(&st.mem) == EMPTY && delta != Scaled::ZERO {
                let k = st.mem.new_kern(delta)?;
                p.set_link(&mut st.mem, k.ptr());
                delta = Scaled::ZERO;
            }
            p
        }
        EMPTY => MemPtr::NULL,
        SUB_BOX => qn.nucleus().info(&st.mem),
        SUB_MLIST => {
            let mlist = qn.nucleus().info(&st.mem);
            let style = st.scanner.cur_style;
            let list = sublist_to_hlist(st, mlist, style)?;
            crate::packaging::hpack(st, list, Scaled::ZERO, ADDITIONAL)?.ptr()
        }
        _ => return Err(Abort::Confusion("mlist2")),
    };
    qn.set_new_hlist(&mut st.mem, p);
    let need_scripts =
        qn.subscr().math_type(&st.mem) != EMPTY || qn.supscr().math_type(&st.mem) != EMPTY;
    if need_scripts {
        make_scripts(st, qn, delta)?;
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

pub(super) fn mlist_to_hlist(st: &mut EngineState) -> Result<(), Abort> {
    let mlist = st.scanner.cur_mlist;
    let penalties = st.scanner.mlist_penalties;
    let style = st.scanner.cur_style;
    let mut q = mlist;
    let mut r: Option<NodePtr<Noad>> = None;
    let mut r_type = NodeType::OpNoad.as_raw();
    let mut max_h = Scaled::ZERO;
    let mut max_d = Scaled::ZERO;

    set_cur_size_and_mu(st);

    enum FirstPass {
        ProcessNucleus(NodePtr<Noad>),
        CheckDimensions(NodePtr<Noad>),
        DoneWithNoad(NodePtr<Noad>),
        DoneWithNode,
    }

    while !q.is_null() {
        let mut delta;
        let action = 'reswitch: loop {
            delta = Scaled::ZERO;
            match st.mem.classify(q) {
                Ok(AnyNode::Noad(qn)) => match qn.noad_type(&st.mem) {
                    NodeType::BinNoad => {
                        match NodeType::from_raw(r_type) {
                            Ok(NodeType::BinNoad)
                            | Ok(NodeType::OpNoad)
                            | Ok(NodeType::RelNoad)
                            | Ok(NodeType::OpenNoad)
                            | Ok(NodeType::PunctNoad)
                            | Ok(NodeType::LeftNoad) => {
                                qn.set_noad_type(&mut st.mem, NodeType::OrdNoad);
                                continue 'reswitch;
                            }
                            _ => {}
                        }
                        break 'reswitch FirstPass::ProcessNucleus(qn);
                    }
                    nt @ (NodeType::RelNoad
                    | NodeType::CloseNoad
                    | NodeType::PunctNoad
                    | NodeType::RightNoad) => {
                        if r_type == NodeType::BinNoad.as_raw() {
                            r.expect("r_type == Bin implies a previous noad")
                                .set_noad_type(&mut st.mem, NodeType::OrdNoad);
                        }
                        if nt == NodeType::RightNoad {
                            break 'reswitch FirstPass::DoneWithNoad(qn);
                        }
                        break 'reswitch FirstPass::ProcessNucleus(qn);
                    }
                    NodeType::LeftNoad => break 'reswitch FirstPass::DoneWithNoad(qn),
                    NodeType::FractionNoad => {
                        make_fraction(st, qn)?;
                        break 'reswitch FirstPass::CheckDimensions(qn);
                    }
                    NodeType::OpNoad => {
                        delta = make_op(st, qn)?;
                        if qn.subtype(&st.mem) == LIMITS {
                            break 'reswitch FirstPass::CheckDimensions(qn);
                        }
                        break 'reswitch FirstPass::ProcessNucleus(qn);
                    }
                    NodeType::OrdNoad => {
                        make_ord(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus(qn);
                    }
                    NodeType::OpenNoad | NodeType::InnerNoad => {
                        break 'reswitch FirstPass::ProcessNucleus(qn)
                    }
                    NodeType::RadicalNoad => {
                        make_radical(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus(qn);
                    }
                    NodeType::OverNoad => {
                        make_over(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus(qn);
                    }
                    NodeType::UnderNoad => {
                        make_under(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus(qn);
                    }
                    NodeType::AccentNoad => {
                        make_math_accent(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus(qn);
                    }
                    NodeType::VcenterNoad => {
                        make_vcenter(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus(qn);
                    }
                    _ => unreachable!("AnyNode::Noad always has a noad type (16..=31)"),
                },
                Ok(AnyNode::Style(sn)) => {
                    st.scanner.cur_style = sn.subtype(&st.mem) as i16;
                    set_cur_size_and_mu(st);
                    break 'reswitch FirstPass::DoneWithNode;
                }
                Ok(AnyNode::Choice(choice)) => {
                    let cs = st.scanner.cur_style;
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
                    let subtype = glue.subtype(&st.mem);
                    let mu = st.scanner.cur_mu;
                    let x = glue.glue_ptr(&st.mem);
                    if subtype == MU_GLUE {
                        let y = math_glue(st, x, mu)?;
                        st.mem.delete_glue_ref(x);
                        glue.set_glue_ptr(&mut st.mem, y);
                        glue.set_subtype(&mut st.mem, NORMAL);
                    } else if subtype == COND_MATH_GLUE {
                        let cur_size = st.scanner.cur_size;
                        st.mem.mlist_cond_math_glue(glue, cur_size)?;
                    }
                    break 'reswitch FirstPass::DoneWithNode;
                }
                Ok(AnyNode::Kern(kern)) => {
                    let mu = st.scanner.cur_mu;
                    math_kern(st, kern, mu);
                    break 'reswitch FirstPass::DoneWithNode;
                }
                _ => return Err(Abort::Confusion("mlist1")),
            }
        };

        let qn = match action {
            FirstPass::DoneWithNode => {
                q = q.link(&st.mem);
                continue;
            }
            FirstPass::DoneWithNoad(qn) => qn,
            FirstPass::ProcessNucleus(qn) | FirstPass::CheckDimensions(qn) => {
                if matches!(action, FirstPass::ProcessNucleus(_)) {
                    mlist_nucleus(st, qn, delta)?;
                }
                let list = qn.new_hlist(&st.mem);
                let z_box = crate::packaging::hpack(st, list, Scaled::ZERO, ADDITIONAL)?;
                st.mem.mlist_track_box(z_box, &mut max_h, &mut max_d);
                qn
            }
        };
        r = Some(qn);
        (r_type, q) = mlist_advance(st, qn, style);
    }

    let mut p = TEMP_HEAD;
    if r_type == NodeType::BinNoad.as_raw() {
        r.expect("r_type == Bin implies a previous noad")
            .set_noad_type(&mut st.mem, NodeType::OrdNoad);
    }
    p.set_link(&mut st.mem, MemPtr::NULL);
    q = mlist;
    r_type = 0;
    st.scanner.cur_style = style;
    set_cur_size_and_mu(st);

    enum SecondPass {
        Splice(NodePtr<Noad>),
        StyleNode,
        Moved,
    }

    while !q.is_null() {
        let mut t = NodeType::OrdNoad.as_raw();
        let mut s = NOAD_SIZE;
        let mut pen = INF_PENALTY;
        let step = match st.mem.classify(q) {
            Ok(AnyNode::Noad(qn)) => {
                match qn.noad_type(&st.mem) {
                    NodeType::OpNoad
                    | NodeType::OpenNoad
                    | NodeType::CloseNoad
                    | NodeType::PunctNoad
                    | NodeType::InnerNoad => {
                        t = qn.noad_type(&st.mem).as_raw();
                    }
                    NodeType::BinNoad => {
                        t = NodeType::BinNoad.as_raw();
                        pen = st.eqtb.int_par(IntPar::BinOpPenalty);
                    }
                    NodeType::RelNoad => {
                        t = NodeType::RelNoad.as_raw();
                        pen = st.eqtb.int_par(IntPar::RelPenalty);
                    }
                    NodeType::OrdNoad
                    | NodeType::VcenterNoad
                    | NodeType::OverNoad
                    | NodeType::UnderNoad => {}
                    NodeType::RadicalNoad => s = RADICAL_NOAD_SIZE,
                    NodeType::AccentNoad => s = ACCENT_NOAD_SIZE,
                    NodeType::FractionNoad => s = FRACTION_NOAD_SIZE,
                    NodeType::LeftNoad | NodeType::RightNoad => {
                        t = make_left_right(st, qn, style, max_d, max_h)?.as_raw();
                    }
                    _ => return Err(Abort::Confusion("mlist3")),
                }
                SecondPass::Splice(qn)
            }
            Ok(AnyNode::Style(sn)) => {
                st.scanner.cur_style = sn.subtype(&st.mem) as i16;
                s = STYLE_NODE_SIZE;
                set_cur_size_and_mu(st);
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
                let ord = NodeType::OrdNoad.as_raw();
                let spacing = MATH_SPACING[(r_type - ord) as usize][(t - ord) as usize];
                let x = match spacing {
                    b'0' => None,
                    b'2' => Some(GluePar::ThinMuSkip),
                    b'1' | b'3' | b'4' if st.scanner.cur_style >= SCRIPT_STYLE => None,
                    b'1' => Some(GluePar::ThinMuSkip),
                    b'3' => Some(GluePar::MedMuSkip),
                    b'4' => Some(GluePar::ThickMuSkip),
                    _ => return Err(Abort::Confusion("mlist4")),
                };
                if let Some(x) = x {
                    let g = st.eqtb.glue_par(x);
                    let mu = st.scanner.cur_mu;
                    let y = math_glue(st, g, mu)?;
                    let z = st.mem.new_glue(y)?;
                    y.set_ref_count(&mut st.mem, TEX_NULL);
                    p.set_link(&mut st.mem, z.ptr());
                    z.set_subtype(&mut st.mem, x.subtype());
                    p = z.ptr();
                }
            }

            let head = qn.new_hlist(&st.mem);
            if !head.is_null() {
                p.set_link(&mut st.mem, head);
                let mut tail = head;
                while !tail.link(&st.mem).is_null() {
                    tail = tail.link(&st.mem);
                }
                p = tail;
            }

            if penalties && pen < INF_PENALTY {
                if let Some(next) = qn.link(&st.mem).as_idx() {
                    if !matches!(
                        st.mem.node_type(next),
                        Ok(NodeType::Penalty | NodeType::RelNoad)
                    ) {
                        let z = st.mem.new_penalty(pen)?;
                        p.set_link(&mut st.mem, z.ptr());
                        p = z.ptr();
                    }
                }
            }

            if qn.noad_type(&st.mem) == NodeType::RightNoad {
                t = NodeType::OpenNoad.as_raw();
            }
            r_type = t;
        }

        let nq = q.link(&st.mem);
        st.mem.free_node(q, s);
        q = nq;
    }
    Ok(())
}

impl MemState {
    pub(super) fn stack_glue_into_box(
        &mut self,
        b: AnyBox,
        min: Scaled,
        max: Scaled,
    ) -> Result<(), Abort> {
        let spec = self.new_spec(ZERO_GLUE)?;
        spec.set_width(self, min);
        spec.set_stretch(self, max - min);
        let p = self.new_glue(spec)?;
        if matches!(b, AnyBox::HList(_)) {
            let mut q = b.list_ptr(self);
            if q.is_null() {
                b.set_list_ptr(self, p.ptr());
            } else {
                while !q.link(self).is_null() {
                    q = q.link(self);
                }
                q.set_link(self, p.ptr());
            }
        } else {
            p.set_link(self, b.list_ptr(self));
            b.set_list_ptr(self, p.ptr());
            // XeTeX reads box height and width off the glue node here.
            let glue = p.ptr();
            b.set_height(self, Scaled::from_raw(self.b32s1(glue + 3)));
            b.set_width(self, Scaled::from_raw(self.b32s1(glue + 1)));
        }
        Ok(())
    }
}

fn compute_ot_math_accent_pos(st: &mut EngineState, mut p: NodePtr<Noad>) -> Result<Scaled, Abort> {
    loop {
        match p.nucleus().math_type(&st.mem) {
            MATH_CHAR => {
                fetch(st, p.nucleus())?;
                let f = st.scanner.cur_f;
                let c = st.scanner.cur_c;
                let q = crate::fonts::new_native_character(st, f, c as u32)?;
                let g = st.mem.get_native_glyph(q, 0);
                return Ok(st.fonts.get_ot_math_accent_pos(f, i32::from(g)));
            }
            SUB_MLIST => {
                let r = p.nucleus().info(&st.mem);
                match r.as_idx().map(|ri| st.mem.classify_noad(ri)) {
                    Some(Ok(Some(rn))) if rn.noad_type(&st.mem) == NodeType::AccentNoad => p = rn,
                    _ => return Ok(INFINITE_DIMEN),
                }
            }
            _ => return Ok(INFINITE_DIMEN),
        }
    }
}
