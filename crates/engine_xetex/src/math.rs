//! Math mode: noad construction and `mlist_to_hlist`.
use crate::consts::{
    ACCENT_NOAD_SIZE, AFTER, A_LEADERS, BEFORE, BEGIN_L_CODE, BEGIN_M_CODE, BOX_NODE_SIZE,
    COND_MATH_GLUE, DIMEN_BASE, DIMEN_PAR_DELIMITER_SHORTFALL, DIMEN_PAR_DISPLAY_INDENT,
    DIMEN_PAR_DISPLAY_WIDTH, DIMEN_PAR_HANG_INDENT, DIMEN_PAR_HSIZE, DIMEN_PAR_MATH_SURROUND,
    DIMEN_PAR_NULL_DELIMITER_SPACE, DIMEN_PAR_PRE_DISPLAY_SIZE, DIMEN_PAR_SCRIPT_SPACE, DLIST,
    EMPTY, END_L_CODE, END_M_CODE, EXPLICIT, FIL, FILL, FILLL, FONT_BASE, FRACTION_NOAD_SIZE,
    GLUE_PAR_ABOVE_DISPLAY_SHORT_SKIP, GLUE_PAR_ABOVE_DISPLAY_SKIP,
    GLUE_PAR_BELOW_DISPLAY_SHORT_SKIP, GLUE_PAR_BELOW_DISPLAY_SKIP, GLUE_PAR_LEFT_SKIP,
    GLUE_PAR_MED_MU_SKIP, GLUE_PAR_RIGHT_SKIP, GLUE_PAR_THICK_MU_SKIP, GLUE_PAR_THIN_MU_SKIP,
    GLYPH_NODE_SIZE, HMODE, INF_PENALTY, INT_BASE, INT_PAR_BIN_OP_PENALTY, INT_PAR_CUR_FAM,
    INT_PAR_DELIMITER_FACTOR, INT_PAR_HANG_AFTER, INT_PAR_POST_DISPLAY_PENALTY,
    INT_PAR_PRE_DISPLAY_DIRECTION, INT_PAR_PRE_DISPLAY_PENALTY, INT_PAR_REL_PENALTY,
    INT_PAR_TEXXET, LEFT_TO_RIGHT, LOCAL_EVERY_DISPLAY, LOCAL_EVERY_MATH, L_CODE, MATH_CHAR,
    MATH_TEXT_CHAR, MAX_HALFWORD, MEDIUM_NODE_SIZE, MMODE, MU_GLUE, NOAD_SIZE, NORMAL,
    NUMBER_MATH_FAMILIES, RADICAL_NOAD_SIZE, RIGHT_TO_LEFT, R_CODE, SCRIPT_SIZE, STYLE_NODE_SIZE,
    SUB_BOX, SUB_MLIST, TEXT_SIZE, TEX_INFINITY,
};
use crate::fonts::{
    CharInfo, AAT_FONT_FLAG, EXT_TAG, LIG_TAG, LIST_TAG, OTGR_FONT_FLAG, QUAD_CODE, SPACE_CODE,
    X_HEIGHT_CODE,
};
use crate::mem::{ADJUST_HEAD, GARBAGE, PRE_ADJUST_HEAD, SS_GLUE, TEMP_HEAD, TEX_NULL, ZERO_GLUE};
use crate::node::{
    AnyBox, AnyNode, ChoiceNode, DelimiterField, GlueNode, GlueSpecNode, KernNode, MathField, Noad,
    NodePtr, RuleDimen, RuleNode, WhatsitNode, WhatsitSubtype,
};
use crate::state::{
    EngineState, EqtbState, ErrorState, FontState, GroupCode, MemState, SaveStackState,
    ScannerState,
};
use crate::units::{FontId, GlueSign, MemIdx, MemPtr, NodeType, Scaled};

pub(crate) const DELIMITED_CODE: i32 = 3;
pub(crate) const ABOVE_CODE: i32 = 0;
pub(crate) const OVER_CODE: i32 = 1;
pub(crate) const ATOP_CODE: i32 = 2;
pub(crate) const DEFAULT_CODE: i32 = 0x4000_0000;

const FIXED_ACC: u16 = 1;
const BOTTOM_ACC: u16 = 2;
pub(crate) const MIDDLE_NOAD: u16 = 1;

pub(crate) const LIMITS: u16 = 1;
pub(crate) const NO_LIMITS: u16 = 2;
const DISPLAYOPERATORMINHEIGHT: i32 = 3;
const ACCENTBASEHEIGHT: i32 = 6;

const SCRIPT_SCRIPT_SIZE: i32 = 512;
const TOTAL_MATHSY_PARAMS: i32 = 22;
const TOTAL_MATHEX_PARAMS: i32 = 13;
const STACKGAPMIN: i32 = 26;
const STACKDISPLAYSTYLEGAPMIN: i32 = 27;
const FRACTIONNUMERATORGAPMIN: i32 = 36;
const FRACTIONNUMDISPLAYSTYLEGAPMIN: i32 = 37;
const FRACTIONDENOMINATORGAPMIN: i32 = 39;
const FRACTIONDENOMDISPLAYSTYLEGAPMIN: i32 = 40;
const RADICALVERTICALGAP: i32 = 49;
const RADICALDISPLAYSTYLEVERTICALGAP: i32 = 50;
const RADICALRULETHICKNESS: i32 = 51;
const SUBSCRIPTTOPMAX: i32 = 9;
const SUPERSCRIPTBOTTOMMIN: i32 = 13;
const SUBSUPERSCRIPTGAPMIN: i32 = 15;
const SUPERSCRIPTBOTTOMMAXWITHSUBSCRIPT: i32 = 16;
const DISPLAY_STYLE: i16 = 0;
const TEXT_STYLE: i16 = 2;
const SCRIPT_STYLE: i16 = 4;

use crate::cmd::{ACCENT, DELIM_NUM, LETTER, MATH_SHIFT, OTHER_CHAR, RELAX, SPACER, SUP_MARK};
use crate::errors::Abort;
use crate::input::{EVERY_DISPLAY_TEXT, EVERY_MATH_TEXT};
use crate::mem_types::ListStateRecord;
use crate::ot_math::{GlyphAssembly, ScriptKind};
use crate::packaging::{ADDITIONAL, EXACTLY};

extern "C" {
    fn isOpenTypeMathFont(engine: *mut libc::c_void) -> bool;
    fn usingOpenType(engine: *mut libc::c_void) -> bool;
}

impl FontState {
    #[inline]
    fn font_uses_opentype(&self, f: i32) -> bool {
        self.font_area_entry(f) == OTGR_FONT_FLAG
            && unsafe { usingOpenType(self.font_layout_engine_ptr(f)) }
    }

    #[inline]
    fn font_is_ot_math(&self, f: i32) -> bool {
        self.font_area_entry(f) == OTGR_FONT_FLAG
            && unsafe { isOpenTypeMathFont(self.font_layout_engine_ptr(f)) }
    }

    #[inline]
    fn math_sy_param(&self, eqtb: &EqtbState, size_code: i32, param: i32) -> i32 {
        let f = eqtb.math_font(2 + size_code);
        if self.font_is_ot_math(f) {
            self.get_native_mathsy_param(f, param)
        } else {
            self.param(f, param)
        }
    }

    #[inline]
    fn math_ext_param(&self, eqtb: &EqtbState, cur_size: i32, param: i32) -> i32 {
        let f = eqtb.math_font(3 + cur_size);
        if self.font_is_ot_math(f) {
            self.get_native_mathex_param(f, param)
        } else {
            self.param(f, param)
        }
    }

    pub fn math_x_height(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 5)
    }

    pub fn math_quad(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 6)
    }

    pub fn num1(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 8)
    }

    pub fn num2(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 9)
    }

    pub fn num3(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 10)
    }

    pub fn denom1(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 11)
    }

    pub fn denom2(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 12)
    }

    pub fn sup1(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 13)
    }

    pub fn sup2(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 14)
    }

    pub fn sup3(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 15)
    }

    pub fn sub1(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 16)
    }

    pub fn sub2(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 17)
    }

    pub fn sup_drop(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 18)
    }

    pub fn sub_drop(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 19)
    }

    pub fn delim1(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 20)
    }

    pub fn delim2(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 21)
    }

    pub fn axis_height(&self, eqtb: &EqtbState, size_code: i32) -> i32 {
        self.math_sy_param(eqtb, size_code, 22)
    }

    pub fn default_rule_thickness(&self, eqtb: &EqtbState, cur_size: i32) -> i32 {
        self.math_ext_param(eqtb, cur_size, 8)
    }

    pub fn big_op_spacing1(&self, eqtb: &EqtbState, cur_size: i32) -> i32 {
        self.math_ext_param(eqtb, cur_size, 9)
    }

    pub fn big_op_spacing2(&self, eqtb: &EqtbState, cur_size: i32) -> i32 {
        self.math_ext_param(eqtb, cur_size, 10)
    }

    pub fn big_op_spacing3(&self, eqtb: &EqtbState, cur_size: i32) -> i32 {
        self.math_ext_param(eqtb, cur_size, 11)
    }

    pub fn big_op_spacing4(&self, eqtb: &EqtbState, cur_size: i32) -> i32 {
        self.math_ext_param(eqtb, cur_size, 12)
    }

    pub fn big_op_spacing5(&self, eqtb: &EqtbState, cur_size: i32) -> i32 {
        self.math_ext_param(eqtb, cur_size, 13)
    }
}

impl MemState {
    pub fn fraction_rule(&mut self, t: i32) -> Result<NodePtr<RuleNode>, Abort> {
        let Some(p) = NodePtr::<RuleNode>::alloc(self) else {
            return Err(Abort::NodeOverflow);
        };
        p.set_subtype(self, 0);
        p.set_width(self, RuleDimen::Running);
        p.set_height(self, RuleDimen::Set(Scaled(t)));
        p.set_depth(self, RuleDimen::Set(Scaled(0)));
        Ok(p)
    }

    pub fn overbar(&mut self, b: AnyBox, k: i32, t: i32) -> Result<MemPtr, Abort> {
        let p = self.new_kern(k).ok_or(Abort::NodeOverflow)?;
        p.set_link(self, b.ptr());
        let q = self.fraction_rule(t)?;
        q.set_link(self, p.ptr());
        let r = self.new_kern(t).ok_or(Abort::NodeOverflow)?;
        r.set_link(self, q.ptr());
        Ok(r.ptr())
    }
}

impl ErrorState {
    /// The integer and fractional parts of the mu factor `m`, as `math_glue` splits it.
    fn mu_parts(&mut self, m: i32) -> (i32, i32) {
        let mut n = self.x_over_n(m, 65_536);
        let mut f = self.tex_remainder;
        if f < 0 {
            n -= 1;
            f += 65_536;
        }
        (n, f)
    }

    fn mu_mult(&mut self, (n, f): (i32, i32), v: i32) -> i32 {
        let xnd = self.xn_over_d(v, f, 65_536);
        self.mult_and_add(n, v, xnd, MAX_HALFWORD)
    }
}

fn math_glue(
    st: &mut EngineState,
    spec: NodePtr<GlueSpecNode>,
    m: i32,
) -> Result<NodePtr<GlueSpecNode>, Abort> {
    let mu = st.errors.mu_parts(m);
    let stretch_order = spec.stretch_order(&st.mem);
    let shrink_order = spec.shrink_order(&st.mem);
    let width = st.errors.mu_mult(mu, spec.width(&st.mem).0);
    let mut stretch = spec.stretch(&st.mem).0;
    if stretch_order == NORMAL {
        stretch = st.errors.mu_mult(mu, stretch);
    }
    let mut shrink = spec.shrink(&st.mem).0;
    if shrink_order == NORMAL {
        shrink = st.errors.mu_mult(mu, shrink);
    }

    let p = NodePtr::<GlueSpecNode>::alloc(&mut st.mem).ok_or(Abort::NodeOverflow)?;
    p.set_width(&mut st.mem, Scaled(width));
    p.set_stretch_order(&mut st.mem, stretch_order);
    p.set_stretch(&mut st.mem, Scaled(stretch));
    p.set_shrink_order(&mut st.mem, shrink_order);
    p.set_shrink(&mut st.mem, Scaled(shrink));
    Ok(p)
}

fn math_kern(st: &mut EngineState, p: NodePtr<KernNode>, m: i32) {
    if p.subtype(&st.mem) == MU_GLUE {
        let mu = st.errors.mu_parts(m);
        let width = st.errors.mu_mult(mu, p.width(&st.mem).0);
        p.set_width(&mut st.mem, Scaled(width));
        p.set_subtype(&mut st.mem, EXPLICIT);
    }
}

impl ListStateRecord {
    pub fn flush_math(&mut self, mem: &mut MemState) -> Result<(), Abort> {
        mem.flush_node_list(self.head.link(mem).0)?;
        mem.flush_node_list(self.aux_s1())?;
        self.head.set_link(mem, MemPtr::NULL);
        self.tail = self.head;
        self.set_aux_s1(TEX_NULL);
        Ok(())
    }
}

fn clean_box(st: &mut EngineState, p: NodePtr<MathField>, s: i16) -> Result<AnyBox, Abort> {
    let q = match p.math_type(&st.mem) {
        MATH_CHAR => {
            let noad = st.mem.new_noad().ok_or(Abort::NodeOverflow)?;
            noad.nucleus().copy_from(&mut st.mem, p);
            sublist_to_hlist(st, noad.ptr(), s)?
        }
        SUB_BOX => p.info(&st.mem),
        SUB_MLIST => sublist_to_hlist(st, p.info(&st.mem), s)?,
        _ => st.mem.new_null_box().ok_or(Abort::NodeOverflow)?.ptr(),
    };

    let lone_box = match q.as_idx() {
        Some(idx) if !st.mem.is_char_node(q) && q.link(&st.mem).is_null() => {
            match st.mem.classify_box(idx) {
                Ok(Some(b)) if b.shift_amount(&st.mem).0 == 0 => Some(b),
                _ => None,
            }
        }
        _ => None,
    };
    let x_box = match lone_box {
        Some(b) => b,
        None => crate::packaging::hpack(st, q, 0, ADDITIONAL)?,
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
fn set_cur_size_and_mu(st: &mut EngineState) {
    st.scanner.cur_size = if st.scanner.cur_style < SCRIPT_STYLE {
        TEXT_SIZE
    } else {
        SCRIPT_SIZE * ((st.scanner.cur_style as i32 - 2) / 2)
    };
    let mq = st.fonts.math_quad(&st.eqtb, st.scanner.cur_size);
    st.scanner.cur_mu = st.errors.x_over_n(mq, 18);
}

enum FetchOutcome {
    Ok,
    UndefinedFont { size: i32, fam: i32, c: i32 },
    CharMissing { f: i32, c: i32 },
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

        if scanner.cur_f == FONT_BASE {
            let c = scanner.cur_c;
            scanner.cur_i = fonts.null_character;
            a.set_math_type(self, EMPTY);
            return FetchOutcome::UndefinedFont { size, fam, c };
        }

        let area = fonts.font_area_entry(scanner.cur_f);
        if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
            scanner.cur_i = fonts.null_character;
        } else {
            if scanner.cur_c >= fonts.font_bc_entry(scanner.cur_f) as i32
                && scanner.cur_c <= fonts.font_ec_entry(scanner.cur_f) as i32
            {
                scanner.cur_i = fonts.char_info(scanner.cur_f, scanner.cur_c);
            } else {
                scanner.cur_i = fonts.null_character;
            }
            if scanner.cur_i.s3 == 0 {
                let f = scanner.cur_f;
                let c = scanner.cur_c;
                a.set_math_type(self, EMPTY);
                scanner.cur_i = fonts.null_character;
                return FetchOutcome::CharMissing { f, c };
            }
        }
        FetchOutcome::Ok
    }
}

fn fetch(st: &mut EngineState, a: NodePtr<MathField>) -> Result<(), Abort> {
    match st.mem.fetch(&st.fonts, &st.eqtb, &mut st.scanner, a) {
        FetchOutcome::Ok => Ok(()),
        FetchOutcome::UndefinedFont { size, fam, c } => {
            crate::output::error_here_with_diagnostic(st, "");
            crate::output::print_size(st, size);
            crate::output::print_char(st, ' ' as i32);
            crate::output::print_int(st, fam);
            crate::output::print(st, " is undefined (character ");
            crate::output::print(st, c);
            crate::output::print_char(st, ')' as i32);
            crate::output::capture_to_diagnostic(st, std::ptr::null_mut());
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
    let p = st.mem.overbar(x, 3 * rule_thickness, rule_thickness)?;
    let x = crate::packaging::vpackage(st, p, 0, ADDITIONAL, MAX_HALFWORD)?;
    st.mem.set_nucleus_box(q, x);
    Ok(())
}

fn make_under(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), Abort> {
    let x = clean_box(st, q.nucleus(), st.scanner.cur_style)?;
    let rule_thickness = st
        .fonts
        .default_rule_thickness(&st.eqtb, st.scanner.cur_size);
    let kern = st
        .mem
        .new_kern(3 * rule_thickness)
        .ok_or(Abort::NodeOverflow)?;
    x.set_link(&mut st.mem, kern.ptr());
    let rule = st.mem.fraction_rule(rule_thickness)?;
    kern.set_link(&mut st.mem, rule.ptr());

    let y = crate::packaging::vpackage(st, x.ptr(), 0, ADDITIONAL, MAX_HALFWORD)?;
    let mem = &mut st.mem;
    let delta = y.height(mem).0 + y.depth(mem).0 + rule_thickness;
    y.set_height(mem, x.height(mem));
    y.set_depth(mem, Scaled(delta - y.height(mem).0));
    mem.set_nucleus_box(q, y);
    Ok(())
}

fn make_vcenter(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), Abort> {
    let axis_height = st.fonts.axis_height(&st.eqtb, st.scanner.cur_size);
    let mem = &mut st.mem;
    let Ok(Some(v @ AnyBox::VList(_))) = mem.classify_box(MemIdx::new(q.nucleus().info(mem)))
    else {
        return Err(Abort::Confusion("vcenter"));
    };
    let delta = v.height(mem).0 + v.depth(mem).0;
    v.set_height(mem, Scaled(axis_height + crate::scaled_math::half(delta)));
    v.set_depth(mem, Scaled(delta - v.height(mem).0));
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

    let delimiter_height = x.height(&st.mem).0 + x.depth(&st.mem).0 + clr + rule_thickness;
    let y = var_delimiter(st, q.left_delimiter(), cur_size, delimiter_height)?;
    let mem = &mut st.mem;
    if is_ot_math {
        y.set_depth(
            mem,
            Scaled(y.depth(mem).0 + y.height(mem).0 - rule_thickness),
        );
        y.set_height(mem, Scaled(rule_thickness));
    }
    let delta = y.depth(mem).0 - (x.height(mem).0 + x.depth(mem).0 + clr);
    if delta > 0 {
        clr += crate::scaled_math::half(delta);
    }
    y.set_shift_amount(mem, Scaled(-(x.height(mem).0 + clr)));
    let p = mem.overbar(x, clr, y.height(mem).0)?;
    let overbar_box = crate::packaging::vpackage(st, p, 0, ADDITIONAL, MAX_HALFWORD)?;
    y.set_link(&mut st.mem, overbar_box.ptr());
    let hbox = crate::packaging::hpack(st, y.ptr(), 0, ADDITIONAL)?;
    st.mem.set_nucleus_box(q, hbox);
    Ok(())
}

struct FractionAssembly {
    x: AnyBox,
    z: AnyBox,
    rule_thickness: i32,
    shift_up: i32,
    shift_down: i32,
    delta: i32,
    axis_height: i32,
}

impl MemState {
    fn make_fraction(&mut self, assembly: FractionAssembly) -> Result<AnyBox, Abort> {
        let vb = AnyBox::VList(
            self.new_null_box()
                .ok_or(Abort::NodeOverflow)?
                .into_vlist(self),
        );
        let xb = assembly.x;
        let zb = assembly.z;
        vb.set_height(self, Scaled(assembly.shift_up + xb.height(self).0));
        vb.set_depth(self, Scaled(zb.depth(self).0 + assembly.shift_down));
        vb.set_width(self, xb.width(self));

        let p: NodePtr<KernNode>;
        if assembly.rule_thickness == 0 {
            let kw =
                (assembly.shift_up - xb.depth(self).0) - (zb.height(self).0 - assembly.shift_down);
            p = self.new_kern(kw).ok_or(Abort::NodeOverflow)?;
            p.set_link(self, assembly.z.ptr());
        } else {
            let y = self.fraction_rule(assembly.rule_thickness)?;
            let p1w =
                (assembly.axis_height - assembly.delta) - (zb.height(self).0 - assembly.shift_down);
            let p1 = self.new_kern(p1w).ok_or(Abort::NodeOverflow)?;
            y.set_link(self, p1.ptr());
            p1.set_link(self, assembly.z.ptr());
            let pw =
                (assembly.shift_up - xb.depth(self).0) - (assembly.axis_height + assembly.delta);
            p = self.new_kern(pw).ok_or(Abort::NodeOverflow)?;
            p.set_link(self, y.ptr());
        }

        xb.set_link(self, p.ptr());
        vb.set_list_ptr(self, assembly.x.ptr());
        Ok(vb)
    }

    fn glyph_whatsit(&self, p: MemPtr) -> Option<NodePtr<WhatsitNode>> {
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

    if x.width(&st.mem).0 < z.width(&st.mem).0 {
        let w = z.width(&st.mem).0;
        x = rebox(st, x, w)?;
    } else {
        let w = x.width(&st.mem).0;
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
            if q.thickness(&st.mem) != 0 {
                st.fonts.num2(&st.eqtb, cur_size)
            } else {
                st.fonts.num3(&st.eqtb, cur_size)
            },
            st.fonts.denom2(&st.eqtb, cur_size),
        )
    };

    let delta;
    if q.thickness(&st.mem) == 0 {
        let clr = if st.fonts.font_is_ot_math(cur_f) {
            if cur_style < TEXT_STYLE {
                st.fonts
                    .get_ot_math_constant(cur_f, STACKDISPLAYSTYLEGAPMIN)
            } else {
                st.fonts.get_ot_math_constant(cur_f, STACKGAPMIN)
            }
        } else if cur_style < TEXT_STYLE {
            7 * st.fonts.default_rule_thickness(&st.eqtb, cur_size)
        } else {
            3 * st.fonts.default_rule_thickness(&st.eqtb, cur_size)
        };

        delta = crate::scaled_math::half(
            clr - ((shift_up - x.depth(&st.mem).0) - (z.height(&st.mem).0 - shift_down)),
        );
        if delta > 0 {
            shift_up += delta;
            shift_down += delta;
        }
    } else {
        let axis_height = st.fonts.axis_height(&st.eqtb, cur_size);
        let (delta1, delta2) = if st.fonts.font_is_ot_math(cur_f) {
            delta = crate::scaled_math::half(q.thickness(&st.mem));
            let clr_num = if cur_style < TEXT_STYLE {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONNUMDISPLAYSTYLEGAPMIN)
            } else {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONNUMERATORGAPMIN)
            };
            let delta1 = clr_num - ((shift_up - x.depth(&st.mem).0) - (axis_height + delta));

            let clr_den = if cur_style < TEXT_STYLE {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONDENOMDISPLAYSTYLEGAPMIN)
            } else {
                st.fonts
                    .get_ot_math_constant(cur_f, FRACTIONDENOMINATORGAPMIN)
            };
            let delta2 = clr_den - ((axis_height - delta) - (z.height(&st.mem).0 - shift_down));
            (delta1, delta2)
        } else {
            let clr = if cur_style < TEXT_STYLE {
                3 * q.thickness(&st.mem)
            } else {
                q.thickness(&st.mem)
            };
            delta = crate::scaled_math::half(q.thickness(&st.mem));
            (
                clr - ((shift_up - x.depth(&st.mem).0) - (axis_height + delta)),
                clr - ((axis_height - delta) - (z.height(&st.mem).0 - shift_down)),
            )
        };

        if delta1 > 0 {
            shift_up += delta1;
        }
        if delta2 > 0 {
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
    let hbox = crate::packaging::hpack(st, left_delim.ptr(), 0, ADDITIONAL)?;
    q.set_new_hlist(&mut st.mem, hbox.ptr());
    Ok(())
}

/// Returns the italic correction `delta`.
fn make_op(st: &mut EngineState, q: NodePtr<Noad>) -> Result<i32, Abort> {
    let mut delta: i32 = 0;

    if q.subtype(&st.mem) == NORMAL && st.scanner.cur_style < TEXT_STYLE {
        q.set_subtype(&mut st.mem, LIMITS);
    }

    if q.nucleus().math_type(&st.mem) == MATH_CHAR {
        fetch(st, q.nucleus())?;

        let f = st.scanner.cur_f;
        if !st.fonts.font_uses_opentype(f) {
            if st.scanner.cur_style < TEXT_STYLE && st.scanner.cur_i.s1 % 4 == LIST_TAG {
                let c = st.scanner.cur_i.s0;
                let i_info = st
                    .fonts
                    .font_info_b16(st.fonts.char_base_entry(f) + c as i32);
                if i_info.s3 > 0 {
                    st.scanner.cur_c = c as i32;
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
                            (glyph.height(&st.mem).0 + glyph.depth(&st.mem).0) as f64 * 5.0 / 4.0;
                        if (h1 as f64) < size_h {
                            h1 = size_h as i32;
                        }
                        let c_glyph = glyph.native_glyph(&st.mem) as i32;
                        let mut n: i32 = 0;
                        let mut h2: i32 = 0;
                        loop {
                            let g = st
                                .fonts
                                .get_ot_math_variant(cur_f, c_glyph, n, &mut h2, false);
                            if h2 > 0 {
                                glyph.set_native_glyph(&mut st.mem, g as u16);
                                st.fonts.measure_glyph(&mut st.mem, glyph)?;
                            }
                            n += 1;
                            if h2 < 0 || h2 >= h1 {
                                break;
                            }
                        }
                        if h2 < 0 {
                            if let Some(assembly) = st.fonts.get_ot_assembly(cur_f, c_glyph, false)
                            {
                                st.mem.free_node(glyph.ptr(), GLYPH_NODE_SIZE);
                                let b = build_opentype_assembly(st, cur_f, &assembly, h1, false)?;
                                x.set_list_ptr(&mut st.mem, b.ptr());
                                delta = 0;
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
                        .get_ot_math_ital_corr(cur_f, glyph.native_glyph(&st.mem) as i32);
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
            x.set_width(mem, Scaled(x.width(mem).0 - delta));
        }
        let shift = crate::scaled_math::half(x.height(mem).0 - x.depth(mem).0) - axis_height;
        x.set_shift_amount(mem, Scaled(shift));
        mem.set_nucleus_box(q, x);
    }

    let save_f = st.scanner.cur_f;
    if q.subtype(&st.mem) == LIMITS {
        let cs = st.scanner.cur_style as i32;
        let x = clean_box(st, q.supscr(), (2 * (cs / 4) + 4 + (cs % 2)) as i16)?;
        let y = clean_box(st, q.nucleus(), st.scanner.cur_style)?;
        let z = clean_box(st, q.subscr(), (2 * (cs / 4) + 5) as i16)?;

        let v = AnyBox::VList(
            st.mem
                .new_null_box()
                .ok_or(Abort::NodeOverflow)?
                .into_vlist(&mut st.mem),
        );
        let w = y
            .width(&st.mem)
            .0
            .max(x.width(&st.mem).0)
            .max(z.width(&st.mem).0);
        v.set_width(&mut st.mem, Scaled(w));

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
        let half_delta = crate::scaled_math::half(delta);
        x.set_shift_amount(mem, Scaled(half_delta));
        z.set_shift_amount(mem, Scaled(-half_delta));
        v.set_height(mem, y.height(mem));
        v.set_depth(mem, y.depth(mem));
        st.scanner.cur_f = save_f;

        if q.supscr().math_type(mem) == EMPTY {
            mem.free_node(x.ptr(), BOX_NODE_SIZE);
            v.set_list_ptr(mem, y.ptr());
        } else {
            let shift_up = (bos3 - x.depth(mem).0).max(bos1);
            let p = mem.new_kern(shift_up).ok_or(Abort::NodeOverflow)?;
            p.set_link(mem, y.ptr());
            x.set_link(mem, p.ptr());
            let p = mem.new_kern(bos5).ok_or(Abort::NodeOverflow)?;
            p.set_link(mem, x.ptr());
            v.set_list_ptr(mem, p.ptr());
            let height = v.height(mem).0 + bos5 + x.height(mem).0 + x.depth(mem).0 + shift_up;
            v.set_height(mem, Scaled(height));
        }

        if q.subscr().math_type(mem) == EMPTY {
            mem.free_node(z.ptr(), BOX_NODE_SIZE);
        } else {
            let shift_down = (bos4 - z.height(mem).0).max(bos2);
            let p = mem.new_kern(shift_down).ok_or(Abort::NodeOverflow)?;
            y.set_link(mem, p.ptr());
            p.set_link(mem, z.ptr());
            let p = mem.new_kern(bos5).ok_or(Abort::NodeOverflow)?;
            z.set_link(mem, p.ptr());
            let depth = v.depth(mem).0 + bos5 + z.height(mem).0 + z.depth(mem).0 + shift_down;
            v.set_depth(mem, Scaled(depth));
        }

        q.set_new_hlist(mem, v.ptr());
    }

    Ok(delta)
}

impl FontState {
    /// Points `cur_i` at the first instruction of the lig/kern program for `cur_i`
    /// in `cur_f` and returns its index.
    fn lig_kern_start(&self, scanner: &mut ScannerState) -> i32 {
        let mut a = self.lig_kern_base_entry(scanner.cur_f) + scanner.cur_i.s0 as i32;
        scanner.cur_i = self.font_info_b16(a);
        if scanner.cur_i.s3 > 128 {
            a = self.lig_kern_base_entry(scanner.cur_f)
                + 256 * scanner.cur_i.s1 as i32
                + scanner.cur_i.s0 as i32
                + 32768
                - 256 * 128;
            scanner.cur_i = self.font_info_b16(a);
        }
        a
    }
}

enum MakeOrdStep {
    Return,
    Restart,
    Overflow,
}

impl MemState {
    fn make_ord_body(
        &mut self,
        fonts: &FontState,
        scanner: &mut ScannerState,
        q: NodePtr<Noad>,
        p: NodePtr<Noad>,
    ) -> MakeOrdStep {
        scanner.cur_c = p.nucleus().character(self) as i32;
        let mut a = fonts.lig_kern_start(scanner);
        loop {
            if scanner.cur_i.s2 == scanner.cur_c as u16 && scanner.cur_i.s3 <= 128 {
                if scanner.cur_i.s1 >= 128 {
                    let kern_width = fonts.char_kern(scanner.cur_f, scanner.cur_i);
                    let Ok(p_kern) = self.new_kern(kern_width).ok_or(Abort::NodeOverflow) else {
                        return MakeOrdStep::Overflow;
                    };
                    let link_q = q.link(self);
                    p_kern.set_link(self, link_q);
                    q.set_link(self, p_kern.ptr());
                    return MakeOrdStep::Return;
                } else {
                    match scanner.cur_i.s1 {
                        1 | 5 => {
                            q.nucleus().set_character(self, scanner.cur_i.s0);
                        }
                        2 | 6 => {
                            p.nucleus().set_character(self, scanner.cur_i.s0);
                        }
                        3 | 7 | 11 => {
                            let Some(r) = self.new_noad() else {
                                return MakeOrdStep::Overflow;
                            };
                            r.nucleus().set_character(self, scanner.cur_i.s0);
                            r.nucleus()
                                .set_plane_and_fam(self, q.nucleus().plane_and_fam(self) % 256);
                            q.set_link(self, r.ptr());
                            r.set_link(self, p.ptr());
                            if scanner.cur_i.s1 < 11 {
                                r.nucleus().set_math_type(self, MATH_CHAR);
                            } else {
                                r.nucleus().set_math_type(self, MATH_TEXT_CHAR);
                            }
                        }
                        _ => {
                            let link_p = p.link(self);
                            q.set_link(self, link_p);
                            q.nucleus().set_character(self, scanner.cur_i.s0);
                            q.subscr().copy_from(self, p.subscr());
                            q.supscr().copy_from(self, p.supscr());
                            self.free_node(p.ptr(), NOAD_SIZE);
                        }
                    }
                    if scanner.cur_i.s1 > 3 {
                        return MakeOrdStep::Return;
                    }
                    q.nucleus().set_math_type(self, MATH_CHAR);
                    return MakeOrdStep::Restart;
                }
            }
            if scanner.cur_i.s3 >= 128 {
                return MakeOrdStep::Return;
            }
            a += scanner.cur_i.s3 as i32 + 1;
            scanner.cur_i = fonts.font_info_b16(a);
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
        if st.scanner.cur_i.s1 % 4 != LIG_TAG {
            return Ok(());
        }

        match st.mem.make_ord_body(&st.fonts, &mut st.scanner, q, p_node) {
            MakeOrdStep::Return => return Ok(()),
            MakeOrdStep::Restart => continue 'restart,
            MakeOrdStep::Overflow => return Err(Abort::NodeOverflow),
        }
    }
}

fn make_math_accent(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), Abort> {
    fetch(st, q.accent_chr())?;
    let f = st.scanner.cur_f;
    let mut c = st.scanner.cur_c;
    let area = st.fonts.font_area_entry(f);
    let kind = q.subtype(&st.mem);
    let bottom_accent = kind == BOTTOM_ACC || kind == BOTTOM_ACC + 1;
    let is_native = area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG;

    let mut s: i32 = 0;
    let mut x: AnyBox;
    if is_native {
        if !bottom_accent {
            s = compute_ot_math_accent_pos(st, q)?;
        }
        x = clean_box(st, q.nucleus(), 2 * (st.scanner.cur_style / 2) + 1)?;
    } else if st.scanner.cur_i.s3 > 0 {
        let mut acc_i = st.scanner.cur_i;
        if q.nucleus().math_type(&st.mem) == MATH_CHAR {
            fetch(st, q.nucleus())?;
            if st.scanner.cur_i.s1 % 4 == LIG_TAG {
                let mut a = st.fonts.lig_kern_start(&mut st.scanner);
                let skew_char = st.fonts.skew_char_entry(st.scanner.cur_f) as u16;
                s = loop {
                    let i = st.scanner.cur_i;
                    if i.s2 == skew_char {
                        if i.s1 >= 128 && i.s3 <= 128 {
                            break st.fonts.char_kern(st.scanner.cur_f, i);
                        }
                        break 0;
                    }
                    if i.s3 >= 128 {
                        break 0;
                    }
                    a += i.s3 as i32 + 1;
                    st.scanner.cur_i = st.fonts.font_info_b16(a);
                };
            }
        }
        x = clean_box(st, q.nucleus(), 2 * (st.scanner.cur_style / 2) + 1)?;
        let w = x.width(&st.mem).0;
        while acc_i.s1 % 4 == LIST_TAG {
            let larger = acc_i.s0 as i32;
            acc_i = st.fonts.char_info(f, larger);
            if acc_i.s3 == 0 || st.fonts.char_width(f, acc_i) > w {
                break;
            }
            c = larger;
        }
    } else {
        return Ok(());
    }
    let w = x.width(&st.mem).0;
    let mut h = x.height(&st.mem).0;

    let is_ot_math = st.fonts.font_is_ot_math(f);
    let mut delta = if is_ot_math {
        if bottom_accent {
            0
        } else {
            h.min(st.fonts.get_ot_math_constant(f, ACCENTBASEHEIGHT))
        }
    } else {
        h.min(st.fonts.param(f, X_HEIGHT_CODE))
    };

    if (q.supscr().math_type(&st.mem) != EMPTY || q.subscr().math_type(&st.mem) != EMPTY)
        && q.nucleus().math_type(&st.mem) == MATH_CHAR
    {
        st.mem.flush_node_list(x.idx().0)?;
        let base = st.mem.new_noad().ok_or(Abort::NodeOverflow)?;
        let mem = &mut st.mem;
        base.nucleus().copy_from(mem, q.nucleus());
        base.supscr().copy_from(mem, q.supscr());
        base.subscr().copy_from(mem, q.subscr());
        for script in [q.supscr(), q.subscr()] {
            script.set_info(mem, MemPtr(0));
            script.set_math_type(mem, EMPTY);
        }
        q.nucleus().set_math_type(mem, SUB_MLIST);
        q.nucleus().set_info(mem, base.ptr());
        x = clean_box(st, q.nucleus(), st.scanner.cur_style)?;
        delta += x.height(&st.mem).0 - h;
        h = x.height(&st.mem).0;
    }

    let y;
    if is_native {
        let old = crate::fonts::new_native_character(st, f, c as u32)?;
        y = st.mem.native_char_box(old)?;
        let glyph =
            NodePtr::<WhatsitNode>::alloc(&mut st.mem, GLYPH_NODE_SIZE, WhatsitSubtype::Glyph)
                .ok_or(Abort::NodeOverflow)?;
        glyph.set_native_font(&mut st.mem, FontId(f));
        let glyph_id = st.mem.real_get_native_glyph(old, 0);
        glyph.set_native_glyph(&mut st.mem, glyph_id);
        st.fonts.measure_glyph(&mut st.mem, glyph)?;
        old.free_native(&mut st.mem);
        y.set_list_ptr(&mut st.mem, glyph.ptr());

        let mut assembly_box = None;
        if !kind.is_multiple_of(2) {
            st.fonts.measure_glyph(&mut st.mem, glyph)?;
        } else {
            let c_glyph = glyph.native_glyph(&st.mem) as i32;
            let mut n: i32 = 0;
            let mut w2: i32 = 0;
            loop {
                let g = st.fonts.get_ot_math_variant(f, c_glyph, n, &mut w2, true);
                if w2 > 0 && w2 <= w {
                    glyph.set_native_glyph(&mut st.mem, g as u16);
                    st.fonts.measure_glyph(&mut st.mem, glyph)?;
                    n += 1;
                }
                if w2 < 0 || w2 >= w {
                    break;
                }
            }
            if w2 < 0 {
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
            if y.height(mem).0 < 0 {
                y.set_height(mem, Scaled(0));
            }
        } else if y.depth(mem).0 < 0 {
            y.set_depth(mem, Scaled(0));
        }

        let attach = match assembly_box {
            None => st
                .fonts
                .get_ot_math_accent_pos(f, glyph.native_glyph(mem) as i32),
            Some(_) => TEX_INFINITY,
        };
        let sa = if attach == TEX_INFINITY {
            crate::scaled_math::half(y.width(mem).0)
        } else {
            attach
        };
        if bottom_accent || s == TEX_INFINITY {
            s = crate::scaled_math::half(w);
        }
        y.set_shift_amount(mem, Scaled(s - sa));
    } else {
        y = st.mem.char_box_tfm(&mut st.fonts, f, c as u16)?;
        let shift = s + crate::scaled_math::half(w - y.width(&st.mem).0);
        y.set_shift_amount(&mut st.mem, Scaled(shift));
    }

    y.set_width(&mut st.mem, Scaled(0));
    let v = if bottom_accent {
        x.set_link(&mut st.mem, y.ptr());
        let v = crate::packaging::vpackage(st, x.ptr(), 0, ADDITIONAL, MAX_HALFWORD)?;
        let v_height = v.height(&st.mem).0;
        v.set_shift_amount(&mut st.mem, Scaled(-(h - v_height)));
        v
    } else {
        let kern = st.mem.new_kern(-delta).ok_or(Abort::NodeOverflow)?;
        kern.set_link(&mut st.mem, x.ptr());
        y.set_link(&mut st.mem, kern.ptr());
        let v = crate::packaging::vpackage(st, y.ptr(), 0, ADDITIONAL, MAX_HALFWORD)?;
        let mem = &mut st.mem;
        if v.height(mem).0 < h {
            let kern = mem
                .new_kern(h - v.height(mem).0)
                .ok_or(Abort::NodeOverflow)?;
            kern.set_link(mem, v.list_ptr(mem));
            v.set_list_ptr(mem, kern.ptr());
            v.set_height(mem, Scaled(h));
        }
        v
    };
    let x_width = x.width(&st.mem);
    v.set_width(&mut st.mem, x_width);
    st.mem.set_nucleus_box(q, v);
    Ok(())
}

impl MemState {
    fn append_to_new_hlist(&mut self, q: NodePtr<Noad>, node: MemPtr) {
        let head = q.new_hlist(self);
        if head.is_null() {
            q.set_new_hlist(self, node);
        } else {
            let mut tail = head;
            while !tail.link(self).is_null() {
                tail = tail.link(self);
            }
            tail.set_link(self, node);
        }
    }
}

struct ScriptKernAttachment {
    p: MemPtr,
    kern: i32,
    is_ot_math: bool,
}

fn maybe_attach_script_kern(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    p: MemPtr,
    script_f: i32,
    script_g: u16,
    kind: ScriptKind,
    shift: i32,
) -> Result<ScriptKernAttachment, Abort> {
    let cur_f = st.scanner.cur_f;
    let nucleus = st.mem.glyph_whatsit(p);

    if !st.fonts.font_is_ot_math(cur_f) {
        return Ok(ScriptKernAttachment {
            p,
            kern: 0,
            is_ot_math: false,
        });
    }

    let Some(nucleus) = nucleus else {
        return Ok(ScriptKernAttachment {
            p,
            kern: 0,
            is_ot_math: true,
        });
    };

    let kern = st.fonts.get_ot_math_kern(
        nucleus.native_font(&st.mem).0,
        nucleus.native_glyph(&st.mem) as i32,
        script_f,
        script_g as i32,
        kind,
        shift,
    );
    let p = if kern == 0 {
        p
    } else {
        let k = st.mem.new_kern(kern).ok_or(Abort::NodeOverflow)?;
        st.mem.append_to_new_hlist(q, k.ptr());
        q.new_hlist(&st.mem)
    };
    Ok(ScriptKernAttachment {
        p,
        kern,
        is_ot_math: true,
    })
}

#[derive(Clone, Copy)]
enum ScriptBoxAdjustment {
    Sub1,
    Sub2,
    Superscript,
}

struct ScriptBoxPrep {
    box_node: AnyBox,
    shift: i32,
}

fn clean_script_box(
    st: &mut EngineState,
    script_field: NodePtr<MathField>,
    clean_style: i16,
    saved_font: i32,
    adjustment: ScriptBoxAdjustment,
) -> Result<ScriptBoxPrep, Abort> {
    let node = clean_box(st, script_field, clean_style)?;
    let shift = match adjustment {
        ScriptBoxAdjustment::Sub1 => st.fonts.sub1(&st.eqtb, st.scanner.cur_size),
        ScriptBoxAdjustment::Sub2 => st.fonts.sub2(&st.eqtb, st.scanner.cur_size),
        ScriptBoxAdjustment::Superscript => {
            if st.scanner.cur_style % 2 != 0 {
                st.fonts.sup3(&st.eqtb, st.scanner.cur_size)
            } else if st.scanner.cur_style < TEXT_STYLE {
                st.fonts.sup1(&st.eqtb, st.scanner.cur_size)
            } else {
                st.fonts.sup2(&st.eqtb, st.scanner.cur_size)
            }
        }
    };
    st.scanner.cur_f = saved_font;
    let w = node.width(&st.mem).0 + st.eqtb.dimen_par(DIMEN_PAR_SCRIPT_SPACE);
    node.set_width(&mut st.mem, Scaled(w));
    Ok(ScriptBoxPrep {
        box_node: node,
        shift,
    })
}

struct ScriptShifts {
    up: i32,
    down: i32,
}

fn initial_script_shifts(st: &mut EngineState, p: MemPtr) -> Result<ScriptShifts, Abort> {
    if st.mem.is_char_node(p) || st.mem.glyph_whatsit(p).is_some() {
        Ok(ScriptShifts { up: 0, down: 0 })
    } else {
        let zb = crate::packaging::hpack(st, p, 0, ADDITIONAL)?;
        let t = if st.scanner.cur_style < SCRIPT_STYLE {
            SCRIPT_SIZE
        } else {
            SCRIPT_SCRIPT_SIZE
        };
        let shifts = ScriptShifts {
            up: zb.height(&st.mem).0 - st.fonts.sup_drop(&st.eqtb, t),
            down: zb.depth(&st.mem).0 + st.fonts.sub_drop(&st.eqtb, t),
        };
        st.mem.free_node(zb.ptr(), BOX_NODE_SIZE);
        Ok(shifts)
    }
}

struct SubSupStackInput {
    q: NodePtr<Noad>,
    p: MemPtr,
    x: AnyBox,
    y: AnyBox,
    sub_f: i32,
    sub_g: u16,
    sup_f: i32,
    sup_g: u16,
    shift_down: i32,
    shift_up: i32,
    delta: i32,
}

fn finish_subsup_stack(st: &mut EngineState, input: SubSupStackInput) -> Result<(), Abort> {
    let mut p = input.p;
    let mut sub_kern;
    let mut sup_kern = 0;
    let attachment = maybe_attach_script_kern(
        st,
        input.q,
        p,
        input.sub_f,
        input.sub_g,
        ScriptKind::Sub,
        input.shift_down,
    )?;
    p = attachment.p;
    sub_kern = attachment.kern;
    if attachment.is_ot_math {
        let sub_empty = input.q.subscr().math_type(&st.mem) == EMPTY;
        if sub_empty {
            let attachment = maybe_attach_script_kern(
                st,
                input.q,
                p,
                input.sup_f,
                input.sup_g,
                ScriptKind::Sup,
                input.shift_up,
            )?;
            sup_kern = attachment.kern;
        }
    } else {
        sup_kern = 0;
        sub_kern = 0;
    }

    let kern_width = (input.shift_up - input.x.depth(&st.mem).0)
        - (input.y.height(&st.mem).0 - input.shift_down);
    input
        .x
        .set_shift_amount(&mut st.mem, Scaled(sup_kern + input.delta - sub_kern));
    let Some(kern) = st.mem.new_kern(kern_width) else {
        return Err(Abort::NodeOverflow);
    };
    input.x.set_link(&mut st.mem, kern.ptr());
    kern.set_link(&mut st.mem, input.y.ptr());

    let packaged = crate::packaging::vpackage(st, input.x.ptr(), 0, ADDITIONAL, MAX_HALFWORD)?;
    packaged.set_shift_amount(&mut st.mem, Scaled(input.shift_down));
    st.mem.append_to_new_hlist(input.q, packaged.ptr());
    Ok(())
}

struct PreparedScript {
    box_node: AnyBox,
    glyph: u16,
    font: i32,
    shift: i32,
}

fn prepare_subscript(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    adjustment: ScriptBoxAdjustment,
    shift_down: i32,
) -> Result<PreparedScript, Abort> {
    let script_field = q.subscr();
    let (glyph, font) = first_script_glyph(st, script_field)?;
    let save_f = st.scanner.cur_f;
    let clean_style = 2 * (st.scanner.cur_style / 4) + 5;
    let prepared = clean_script_box(st, script_field, clean_style, save_f, adjustment)?;
    Ok(PreparedScript {
        box_node: prepared.box_node,
        glyph,
        font,
        shift: shift_down.max(prepared.shift),
    })
}

fn finish_subscript_only(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    p: MemPtr,
    shift_down: i32,
) -> Result<(), Abort> {
    let PreparedScript {
        box_node: x,
        glyph: sub_g,
        font: sub_f,
        shift: shift_down,
    } = prepare_subscript(st, q, ScriptBoxAdjustment::Sub1, shift_down)?;

    let cur_f = st.scanner.cur_f;
    let box_depth = x.height(&st.mem).0;
    let x_height = st.fonts.math_x_height(&st.eqtb, st.scanner.cur_size);
    let clr = if st.fonts.font_is_ot_math(cur_f) {
        box_depth - st.fonts.get_ot_math_constant(cur_f, SUBSCRIPTTOPMAX)
    } else {
        box_depth - (x_height * 4).abs() / 5
    };

    let shift_down = shift_down.max(clr);
    x.set_shift_amount(&mut st.mem, Scaled(shift_down));

    let _ = maybe_attach_script_kern(st, q, p, sub_f, sub_g, ScriptKind::Sub, shift_down)?;
    st.mem.append_to_new_hlist(q, x.ptr());
    Ok(())
}

fn prepare_superscript(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    mut shift_up: i32,
) -> Result<PreparedScript, Abort> {
    let (glyph, font) = first_script_glyph(st, q.supscr())?;
    let save_f = st.scanner.cur_f;
    let clean_style = 2 * (st.scanner.cur_style / 4) + 4 + (st.scanner.cur_style % 2);
    let prepared = clean_script_box(
        st,
        q.supscr(),
        clean_style,
        save_f,
        ScriptBoxAdjustment::Superscript,
    )?;
    shift_up = shift_up.max(prepared.shift);

    let cur_f = st.scanner.cur_f;
    let box_height = prepared.box_node.depth(&st.mem).0;
    let x_height = st.fonts.math_x_height(&st.eqtb, st.scanner.cur_size);
    let clr = if st.fonts.font_is_ot_math(cur_f) {
        box_height + st.fonts.get_ot_math_constant(cur_f, SUPERSCRIPTBOTTOMMIN)
    } else {
        box_height + x_height.abs() / 4
    };

    Ok(PreparedScript {
        box_node: prepared.box_node,
        glyph,
        font,
        shift: shift_up.max(clr),
    })
}

fn finish_superscript_branch(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    p: MemPtr,
    superscript: PreparedScript,
    mut shift_down: i32,
    delta: i32,
) -> Result<(), Abort> {
    let x = superscript.box_node;
    let shift_up = superscript.shift;
    let sub_empty = q.subscr().math_type(&st.mem) == EMPTY;
    if sub_empty {
        let _ = maybe_attach_script_kern(
            st,
            q,
            p,
            superscript.font,
            superscript.glyph,
            ScriptKind::Sup,
            shift_up,
        )?;
        x.set_shift_amount(&mut st.mem, Scaled(-shift_up));
        st.mem.append_to_new_hlist(q, x.ptr());
        return Ok(());
    }

    let PreparedScript {
        box_node: y,
        glyph: sub_g,
        font: sub_f,
        shift,
    } = prepare_subscript(st, q, ScriptBoxAdjustment::Sub2, shift_down)?;
    shift_down = shift;

    let cur_f = st.scanner.cur_f;
    let x_height_node = x.depth(&st.mem).0;
    let y_depth = y.height(&st.mem).0;
    let default_rule_thickness = st
        .fonts
        .default_rule_thickness(&st.eqtb, st.scanner.cur_size);
    let mut clr = if st.fonts.font_is_ot_math(cur_f) {
        st.fonts.get_ot_math_constant(cur_f, SUBSUPERSCRIPTGAPMIN)
            - ((shift_up - x_height_node) - (y_depth - shift_down))
    } else {
        4 * default_rule_thickness - ((shift_up - x_height_node) - (y_depth - shift_down))
    };

    let mut shift_up = shift_up;
    if clr > 0 {
        shift_down += clr;
        let cur_f = st.scanner.cur_f;
        let x_height_node = x.depth(&st.mem).0;
        let x_height = st.fonts.math_x_height(&st.eqtb, st.scanner.cur_size);
        clr = if st.fonts.font_is_ot_math(cur_f) {
            st.fonts
                .get_ot_math_constant(cur_f, SUPERSCRIPTBOTTOMMAXWITHSUBSCRIPT)
                - (shift_up - x_height_node)
        } else {
            (x_height * 4).abs() / 5 - (shift_up - x_height_node)
        };
        if clr > 0 {
            shift_up += clr;
            shift_down -= clr;
        }
    }

    finish_subsup_stack(
        st,
        SubSupStackInput {
            q,
            p,
            x,
            y,
            sub_f,
            sub_g,
            sup_f: superscript.font,
            sup_g: superscript.glyph,
            shift_down,
            shift_up,
            delta,
        },
    )?;
    Ok(())
}

impl MemState {
    /// Finds the first noad of a sub-mlist script field to probe for an OpenType
    /// glyph, tracking style changes on the way. Returns the field and its style.
    fn first_script_glyph_step(
        &self,
        script: NodePtr<MathField>,
        initial_math_style: i16,
    ) -> (Option<NodePtr<MathField>>, i16) {
        if script.math_type(self) != SUB_MLIST {
            return (Some(script), initial_math_style);
        }
        let mut script_ptr = script.info(self);
        let mut found = None;
        let mut style = initial_math_style;
        while script_ptr.0 >= 0 && script_ptr.0 <= self.mem_end {
            let node = self.classify(MemIdx::new(script_ptr));
            script_ptr = match node {
                Ok(AnyNode::Kern(_) | AnyNode::Glue(_)) => script_ptr.link(self),
                Ok(AnyNode::Style(sn)) => {
                    style = sn.subtype(self) as i16;
                    script_ptr.link(self)
                }
                Ok(AnyNode::Choice(cn)) => match style / 2 {
                    0 => cn.display_mlist(self),
                    1 => cn.text_mlist(self),
                    2 => cn.script_mlist(self),
                    3 => cn.script_script_mlist(self),
                    _ => script_ptr,
                },
                Ok(AnyNode::Noad(n))
                    if n.noad_type(self).as_raw() <= NodeType::PunctNoad.as_raw() =>
                {
                    found = Some(n.nucleus());
                    MemPtr::NULL
                }
                _ => MemPtr::NULL,
            };
        }
        (found, style)
    }
}

fn first_script_glyph(
    st: &mut EngineState,
    script_head: NodePtr<MathField>,
) -> Result<(u16, i32), Abort> {
    let mut script_g: u16 = 0;
    let mut script_f: i32 = 0;

    let initial_math_style = 2 * (st.scanner.cur_style / 4) + 5;
    let (script_head, this_math_style) = st
        .mem
        .first_script_glyph_step(script_head, initial_math_style);
    let math_char = script_head.filter(|h| h.math_type(&st.mem) == MATH_CHAR);

    if let Some(script_head) = math_char {
        let save_f = st.scanner.cur_f;
        let saved_math_style = st.scanner.cur_style;
        st.scanner.cur_style = this_math_style;
        set_cur_size_and_mu(st);
        fetch(st, script_head)?;

        let cur_f_after_fetch = st.scanner.cur_f;
        let cur_c_after_fetch = st.scanner.cur_c;
        if st.fonts.font_is_ot_math(cur_f_after_fetch) {
            let script_c = crate::fonts::new_native_character(
                st,
                cur_f_after_fetch,
                cur_c_after_fetch as u32,
            )?;
            script_g = st.mem.real_get_native_glyph(script_c, 0);
            script_f = cur_f_after_fetch;
        }

        st.scanner.cur_f = save_f;
        st.scanner.cur_style = saved_math_style;
        set_cur_size_and_mu(st);
    }

    Ok((script_g, script_f))
}

fn make_scripts(st: &mut EngineState, q: NodePtr<Noad>, delta: i32) -> Result<(), Abort> {
    let p = q.new_hlist(&st.mem);
    let shifts = initial_script_shifts(st, p)?;

    if q.supscr().math_type(&st.mem) == EMPTY {
        finish_subscript_only(st, q, p, shifts.down)?;
    } else {
        let superscript = prepare_superscript(st, q, shifts.up)?;
        finish_superscript_branch(st, q, p, superscript, shifts.down, delta)?;
    }
    Ok(())
}

fn make_left_right(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    style: i16,
    max_d: i32,
    max_h: i32,
) -> Result<NodeType, Abort> {
    st.scanner.cur_style = style;
    set_cur_size_and_mu(st);
    let size = st.scanner.cur_size;
    let axis_height = st.fonts.axis_height(&st.eqtb, size);
    let delimiter_factor = st.eqtb.int_par(INT_PAR_DELIMITER_FACTOR);
    let delimiter_shortfall = st.eqtb.dimen_par(DIMEN_PAR_DELIMITER_SHORTFALL);
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
        self.flush_node_list(choice.display_mlist(self).0)?;
        self.flush_node_list(choice.text_mlist(self).0)?;
        self.flush_node_list(choice.script_mlist(self).0)?;
        self.flush_node_list(choice.script_script_mlist(self).0)?;
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

    fn mlist_track_rule(&self, rule: NodePtr<RuleNode>, max_h: &mut i32, max_d: &mut i32) {
        *max_h = (*max_h).max(rule.height(self).raw());
        *max_d = (*max_d).max(rule.depth(self).raw());
    }

    fn mlist_track_box(&mut self, z: AnyBox, max_h: &mut i32, max_d: &mut i32) {
        *max_h = (*max_h).max(z.height(self).0);
        *max_d = (*max_d).max(z.depth(self).0);
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
            self.flush_node_list(p.0)?;
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
fn mlist_nucleus(st: &mut EngineState, qn: NodePtr<Noad>, mut delta: i32) -> Result<(), Abort> {
    let p = match qn.nucleus().math_type(&st.mem) {
        MATH_CHAR | MATH_TEXT_CHAR => {
            fetch(st, qn.nucleus())?;
            let cur_f = st.scanner.cur_f;
            let area = st.fonts.font_area_entry(cur_f);
            let p = if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
                let z = crate::fonts::new_native_character(st, cur_f, st.scanner.cur_c as u32)?;
                let glyph = NodePtr::<WhatsitNode>::alloc(
                    &mut st.mem,
                    GLYPH_NODE_SIZE,
                    WhatsitSubtype::Glyph,
                )
                .ok_or(Abort::NodeOverflow)?;
                glyph.set_native_font(&mut st.mem, FontId(cur_f));
                let z_glyph = st.mem.real_get_native_glyph(z, 0);
                glyph.set_native_glyph(&mut st.mem, z_glyph);
                st.fonts.measure_glyph(&mut st.mem, glyph)?;
                z.free_native(&mut st.mem);
                delta = st
                    .fonts
                    .get_ot_math_ital_corr(cur_f, glyph.native_glyph(&st.mem) as i32);
                if qn.nucleus().math_type(&st.mem) == MATH_TEXT_CHAR
                    && !st.fonts.font_is_ot_math(cur_f)
                {
                    delta = 0;
                }
                glyph.ptr()
            } else if st.scanner.cur_i.s3 > 0 {
                delta = st.fonts.char_italic(cur_f, st.scanner.cur_i);
                let cc = st.scanner.cur_c as u16;
                let p = crate::main_control::new_character(st, cur_f, cc)?;
                let p = p.map_or(MemPtr::NULL, crate::main_control::NewChar::ptr);
                if qn.nucleus().math_type(&st.mem) == MATH_TEXT_CHAR
                    && st.fonts.param(cur_f, SPACE_CODE) != 0
                {
                    delta = 0;
                }
                p
            } else {
                MemPtr::NULL
            };
            if !p.is_null() && qn.subscr().math_type(&st.mem) == EMPTY && delta != 0 {
                let k = st.mem.new_kern(delta).ok_or(Abort::NodeOverflow)?;
                p.set_link(&mut st.mem, k.ptr());
                delta = 0;
            }
            p
        }
        EMPTY => MemPtr::NULL,
        SUB_BOX => qn.nucleus().info(&st.mem),
        SUB_MLIST => {
            let mlist = qn.nucleus().info(&st.mem);
            let style = st.scanner.cur_style;
            let list = sublist_to_hlist(st, mlist, style)?;
            crate::packaging::hpack(st, list, 0, ADDITIONAL)?.ptr()
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

fn mlist_to_hlist(st: &mut EngineState) -> Result<(), Abort> {
    let mlist = st.scanner.cur_mlist;
    let penalties = st.scanner.mlist_penalties;
    let style = st.scanner.cur_style;
    let mut q = mlist;
    let mut r: Option<NodePtr<Noad>> = None;
    let mut r_type = NodeType::OpNoad.as_raw();
    let mut max_h = 0;
    let mut max_d = 0;

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
            delta = 0;
            match st.mem.classify(MemIdx::new(q)) {
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
                let z_box = crate::packaging::hpack(st, list, 0, ADDITIONAL)?;
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
        let step = match st.mem.classify(MemIdx::new(q)) {
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
                        pen = st.eqtb.int_par(INT_PAR_BIN_OP_PENALTY);
                    }
                    NodeType::RelNoad => {
                        t = NodeType::RelNoad.as_raw();
                        pen = st.eqtb.int_par(INT_PAR_REL_PENALTY);
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
                    b'0' => 0,
                    b'2' => GLUE_PAR_THIN_MU_SKIP,
                    b'1' | b'3' | b'4' if st.scanner.cur_style >= SCRIPT_STYLE => 0,
                    b'1' => GLUE_PAR_THIN_MU_SKIP,
                    b'3' => GLUE_PAR_MED_MU_SKIP,
                    b'4' => GLUE_PAR_THICK_MU_SKIP,
                    _ => return Err(Abort::Confusion("mlist4")),
                };
                if x != 0 {
                    let g = st.eqtb.glue_par(x);
                    let mu = st.scanner.cur_mu;
                    let y = math_glue(st, g, mu)?;
                    let z = st.mem.new_glue(y).ok_or(Abort::NodeOverflow)?;
                    y.set_ref_count(&mut st.mem, TEX_NULL);
                    p.set_link(&mut st.mem, z.ptr());
                    z.set_subtype(&mut st.mem, (x + 1) as u16);
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
                        let z = st.mem.new_penalty(pen).ok_or(Abort::NodeOverflow)?;
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

impl FontState {
    fn measure_glyph(&self, mem: &mut MemState, p: NodePtr<WhatsitNode>) -> Result<(), Abort> {
        // SAFETY: `p` is a glyph whatsit and the measurer only reads font tables over FFI.
        unsafe { self.measure_native_glyph(mem, p, true) }
    }
}

impl MemState {
    fn native_char_box(&mut self, p: NodePtr<WhatsitNode>) -> Result<AnyBox, Abort> {
        let b = self.new_null_box().ok_or(Abort::NodeOverflow)?;
        b.set_height(self, p.height(self));
        b.set_width(self, p.width(self));
        b.set_depth(self, Scaled(p.depth(self).0.max(0)));
        b.set_list_ptr(self, p.ptr());
        Ok(AnyBox::HList(b))
    }
}

fn char_box(st: &mut EngineState, f: i32, c: i32) -> Result<AnyBox, Abort> {
    let area = st.fonts.font_area_entry(f);
    if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
        let p = crate::fonts::new_native_character(st, f, c as u32)?;
        Ok(st.mem.native_char_box(p)?)
    } else {
        Ok(st.mem.char_box_tfm(&mut st.fonts, f, c as u16)?)
    }
}

impl MemState {
    fn char_box_tfm(&mut self, fonts: &mut FontState, f: i32, c: u16) -> Result<AnyBox, Abort> {
        let c_eff = fonts.effective_char(f, c);
        let q = fonts.char_info(f, c_eff);
        let width = fonts.char_width(f, q) + fonts.char_italic(f, q);
        let height = fonts.char_height(f, q);
        let depth = fonts.char_depth(f, q);

        let b = self.new_null_box().ok_or(Abort::NodeOverflow)?;
        b.set_width(self, Scaled(width));
        b.set_height(self, Scaled(height));
        b.set_depth(self, Scaled(depth));
        let p = self.new_char(FontId(f), c).ok_or(Abort::AvailOverflow)?;
        b.set_list_ptr(self, p.ptr());
        Ok(AnyBox::HList(b))
    }

    fn stack_into_box(&mut self, b_box: AnyBox, p: AnyBox) {
        let link = b_box.list_ptr(self);
        p.set_link(self, link);
        b_box.set_list_ptr(self, p.ptr());
        let h = p.height(self).0;
        b_box.set_height(self, Scaled(h));
    }
}

impl FontState {
    fn height_plus_depth(&mut self, f: i32, c: u16) -> i32 {
        let c = self.effective_char(f, c);
        let q = self.char_info(f, c);
        self.char_height(f, q) + self.char_depth(f, q)
    }
}

/// A measured glyph whatsit for glyph `g` of font `f`.
fn new_measured_glyph(st: &mut EngineState, f: i32, g: i32) -> Result<NodePtr<WhatsitNode>, Abort> {
    let p = NodePtr::<WhatsitNode>::alloc(&mut st.mem, GLYPH_NODE_SIZE, WhatsitSubtype::Glyph)
        .ok_or(Abort::NodeOverflow)?;
    p.set_native_font(&mut st.mem, FontId(f));
    p.set_native_glyph(&mut st.mem, g as u16);
    st.fonts.measure_glyph(&mut st.mem, p)?;
    Ok(p)
}

fn stack_glyph_into_box(st: &mut EngineState, b: AnyBox, f: i32, g: i32) -> Result<(), Abort> {
    let p = new_measured_glyph(st, f, g)?;
    let mem = &mut st.mem;
    if matches!(b, AnyBox::HList(_)) {
        let mut q = b.list_ptr(mem);
        if q.is_null() {
            b.set_list_ptr(mem, p.ptr());
        } else {
            while !q.link(mem).is_null() {
                q = q.link(mem);
            }
            q.set_link(mem, p.ptr());
            if b.height(mem) < p.height(mem) {
                b.set_height(mem, p.height(mem));
            }
            if b.depth(mem) < p.depth(mem) {
                b.set_depth(mem, p.depth(mem));
            }
        }
    } else {
        p.set_link(mem, b.list_ptr(mem));
        b.set_list_ptr(mem, p.ptr());
        b.set_height(mem, p.height(mem));
        if b.width(mem) < p.width(mem) {
            b.set_width(mem, p.width(mem));
        }
    }
    Ok(())
}

fn native_glyph_vbox(st: &mut EngineState, f: i32, g: u16) -> Result<AnyBox, Abort> {
    let b = AnyBox::VList(
        st.mem
            .new_null_box()
            .ok_or(Abort::NodeOverflow)?
            .into_vlist(&mut st.mem),
    );
    let p = new_measured_glyph(st, f, g as i32)?;
    let mem = &mut st.mem;
    b.set_list_ptr(mem, p.ptr());
    b.set_width(mem, p.width(mem));
    b.set_height(mem, p.height(mem));
    b.set_depth(mem, p.depth(mem));
    Ok(b)
}

impl MemState {
    fn stack_glue_into_box(&mut self, b: AnyBox, min: i32, max: i32) -> Result<(), Abort> {
        let spec = self.new_spec(ZERO_GLUE).ok_or(Abort::NodeOverflow)?;
        spec.set_width(self, Scaled(min));
        spec.set_stretch(self, Scaled(max - min));
        let p = self.new_glue(spec).ok_or(Abort::NodeOverflow)?;
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
            let raw = p.ptr().0;
            b.set_height(self, Scaled(self.b32s1(raw + 3)));
            b.set_width(self, Scaled(self.b32s1(raw + 1)));
        }
        Ok(())
    }
}

fn compute_ot_math_accent_pos(st: &mut EngineState, mut p: NodePtr<Noad>) -> Result<i32, Abort> {
    loop {
        match p.nucleus().math_type(&st.mem) {
            MATH_CHAR => {
                fetch(st, p.nucleus())?;
                let f = st.scanner.cur_f;
                let c = st.scanner.cur_c;
                let q = crate::fonts::new_native_character(st, f, c as u32)?;
                let g = st.mem.real_get_native_glyph(q, 0);
                return Ok(st.fonts.get_ot_math_accent_pos(f, g as i32));
            }
            SUB_MLIST => {
                let r = p.nucleus().info(&st.mem);
                match r.as_idx().map(|ri| st.mem.classify_noad(ri)) {
                    Some(Ok(Some(rn))) if rn.noad_type(&st.mem) == NodeType::AccentNoad => p = rn,
                    _ => return Ok(TEX_INFINITY),
                }
            }
            _ => return Ok(TEX_INFINITY),
        }
    }
}

pub(crate) fn scan_delimiter(
    st: &mut EngineState,
    p: NodePtr<DelimiterField>,
    r: bool,
) -> Result<(), Abort> {
    if r {
        if st.scanner.cur_chr == 1 {
            st.scanner.cur_val1 = 0x4000_0000;
            crate::scan::scan_math_fam_int(st)?;
            st.scanner.cur_val1 += st.scanner.cur_val * 0x20_0000;
            crate::scan::scan_usv_num(st)?;
            st.scanner.cur_val = st.scanner.cur_val + st.scanner.cur_val1;
        } else {
            crate::scan::scan_delimiter_int(st)?;
        }
    } else {
        loop {
            crate::scan::get_x_token(st)?;
            if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
                break;
            }
        }

        match st.scanner.cur_cmd {
            LETTER | OTHER_CHAR => st.scanner.cur_val = st.eqtb.del_code(st.scanner.cur_chr),
            DELIM_NUM => {
                if st.scanner.cur_chr == 1 {
                    st.scanner.cur_val1 = 0x4000_0000;
                    crate::scan::scan_math_class_int(st)?;
                    crate::scan::scan_math_fam_int(st)?;
                    st.scanner.cur_val1 += st.scanner.cur_val * 0x20_0000;
                    crate::scan::scan_usv_num(st)?;
                    st.scanner.cur_val = st.scanner.cur_val + st.scanner.cur_val1;
                } else {
                    crate::scan::scan_delimiter_int(st)?;
                }
            }
            _ => st.scanner.cur_val = -1,
        }
    }

    if st.scanner.cur_val < 0 {
        crate::output::print_error_prefix(st);
        crate::output::print(st, "Missing delimiter (. inserted)");
        st.errors.help(&[
            r"I was expecting to see something like `(' or `\{' or",
            r"`\}' here. If you typed, e.g., `{' instead of `\{', you",
            "should probably delete the `{' by typing `1' now, so that",
            "braces don't get unbalanced. Otherwise just proceed.",
            r"Acceptable delimiters are characters whose \delcode is",
            r"nonnegative, or you can use `\delimiter <delimiter code>'.",
        ]);
        crate::input::back_error(st)?;
        st.scanner.cur_val = 0;
    }

    let v = st.scanner.cur_val;
    if v >= 0x4000_0000 {
        let plane_and_fam = ((v % 0x20_0000) / 0x1_0000) * 0x100 + (v / 0x20_0000) % 0x100;
        p.set_small(&mut st.mem, plane_and_fam as u16, (v % 0x1_0000) as u16);
        p.set_large(&mut st.mem, 0, 0);
    } else {
        p.set_small(
            &mut st.mem,
            ((v / 0x10_0000) % 16) as u16,
            ((v / 0x1000) % 0x100) as u16,
        );
        p.set_large(&mut st.mem, ((v / 0x100) % 16) as u16, (v % 0x100) as u16);
    }
    Ok(())
}

fn define_word(st: &mut EngineState, loc: i32, val: i32) -> Result<(), Abort> {
    Ok(st.eqtb.eq_word_define(&mut st.save_stack, loc, val)?)
}

pub(crate) fn start_eq_no(st: &mut EngineState) -> Result<(), Abort> {
    st.save_stack.set_saved(0, st.scanner.cur_chr);
    st.save_stack.save_ptr += 1;

    crate::main_control::push_math(st, GroupCode::MathShift)?;
    define_word(st, INT_BASE + INT_PAR_CUR_FAM, -1)?;
    if st.shipout.insert_src_special_every_math {
        crate::ext::insert_src_special(st)?;
    }
    let every_math = st.eqtb.local(LOCAL_EVERY_MATH);
    if !every_math.is_null() {
        crate::input::begin_token_list(st, every_math, EVERY_MATH_TEXT)?;
    }
    Ok(())
}

impl ListStateRecord {
    fn apply_math_limit_switch(&mut self, mem: &mut MemState, limit: u16) -> bool {
        let tail = self.tail.0;
        if self.head.0 == tail {
            return false;
        }
        match mem.classify_noad(MemIdx::new(tail)) {
            Ok(Some(noad)) if noad.noad_type(mem) == NodeType::OpNoad => {
                noad.set_subtype(mem, limit);
                true
            }
            _ => false,
        }
    }
}

pub(crate) fn math_limit_switch(st: &mut EngineState) -> Result<(), Abort> {
    if st
        .input
        .cur_list
        .apply_math_limit_switch(&mut st.mem, st.scanner.cur_chr as u16)
    {
        return Ok(());
    }
    crate::output::print_err(st, "Limit controls must follow a math operator");
    st.errors
        .help(&[r"I'm ignoring this misplaced \limits or \nolimits command."]);
    crate::errors::error(st)
}

impl ListStateRecord {
    fn append_radical_noad(&mut self, mem: &mut MemState) -> Result<NodePtr<Noad>, Abort> {
        let node = NodePtr::<Noad>::alloc_radical(mem).ok_or(Abort::NodeOverflow)?;
        self.tail_append(mem, node.ptr());
        node.clear_fields(mem);
        Ok(node)
    }
}

pub(crate) fn math_radical(st: &mut EngineState) -> Result<(), Abort> {
    let tail = st.input.cur_list.append_radical_noad(&mut st.mem)?;
    scan_delimiter(st, tail.left_delimiter(), true)?;
    crate::scan::scan_math(st, tail.nucleus())
}

impl ListStateRecord {
    fn append_choices(
        &mut self,
        mem: &mut MemState,
        save_stack: &mut SaveStackState,
    ) -> Result<(), Abort> {
        let choice = mem.new_choice().ok_or(Abort::NodeOverflow)?;
        self.tail_append(mem, choice.ptr());
        save_stack.save_ptr += 1;
        save_stack.set_saved(-1, 0);
        Ok(())
    }
}

pub(crate) fn append_choices(st: &mut EngineState) -> Result<(), Abort> {
    st.input
        .cur_list
        .append_choices(&mut st.mem, &mut st.save_stack)?;
    crate::main_control::push_math(st, GroupCode::MathChoice)?;
    crate::scan::scan_left_brace(st)?;
    Ok(())
}

impl ListStateRecord {
    fn fin_mlist(
        &mut self,
        mem: &mut MemState,
        p: impl crate::units::MemAddr,
    ) -> Result<i32, Abort> {
        let p = p.addr();
        let q;
        let aux = self.aux_s1();
        if aux != TEX_NULL {
            // SAFETY: a math list's `incompleat_noad` is a fraction noad.
            let aux_noad = unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(aux)) };
            aux_noad.subscr().set_math_type(mem, SUB_MLIST);
            aux_noad.subscr().set_info(mem, self.head.link(mem));
            if p == TEX_NULL {
                q = aux;
            } else {
                q = aux_noad.supscr().info(mem).0;
                let q_idx = MemIdx::new(q);
                // SAFETY: the numerator of an open `\middle` group is a noad list.
                let q_noad = unsafe { NodePtr::<Noad>::from_idx_unchecked(q_idx) };
                let etex_aux = self.e_tex_aux.0;
                if q_noad.noad_type(mem) != NodeType::LeftNoad || etex_aux == TEX_NULL {
                    return Err(Abort::Confusion("right"));
                }
                // SAFETY: a math-left group keeps its last noad in `eTeX_aux`.
                let etex_aux_noad =
                    unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(etex_aux)) };
                let etex_link = etex_aux_noad.link(mem);
                aux_noad.supscr().set_info(mem, etex_link);
                etex_aux_noad.set_link(mem, MemPtr::from_raw(aux));
                aux_noad.set_link(mem, MemPtr::from_raw(p));
            }
        } else {
            self.tail.set_link(mem, MemPtr::from_raw(p));
            q = self.head.link(mem).0;
        }
        Ok(q)
    }
}

pub(crate) fn fin_mlist(st: &mut crate::state::EngineState, p: i32) -> Result<i32, Abort> {
    let q = st.input.cur_list.fin_mlist(&mut st.mem, p)?;
    st.mem.pop_nest(&mut st.input);
    Ok(q)
}

impl ListStateRecord {
    fn build_choices_finish(
        &mut self,
        mem: &mut MemState,
        save_stack: &mut SaveStackState,
        p: impl crate::units::MemAddr,
    ) -> bool {
        let p = p.addr();
        // SAFETY: a math-choice group builds its branches after the choice node.
        let tail = unsafe { NodePtr::<ChoiceNode>::from_idx_unchecked(MemIdx::new(self.tail.0)) };
        let p_ptr = MemPtr::from_raw(p);
        match save_stack.saved(-1) {
            0 => tail.set_display_mlist(mem, p_ptr),
            1 => tail.set_text_mlist(mem, p_ptr),
            2 => tail.set_script_mlist(mem, p_ptr),
            3 => {
                tail.set_script_script_mlist(mem, p_ptr);
                save_stack.save_ptr -= 1;
                return true;
            }
            _ => {}
        }
        let choice_state = save_stack.saved(-1);
        save_stack.set_saved(-1, choice_state + 1);
        false
    }
}

pub(crate) fn build_choices(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    crate::save::unsave(st)?;
    let p = st.input.cur_list.fin_mlist(&mut st.mem, TEX_NULL)?;
    st.mem.pop_nest(&mut st.input);
    let complete = st
        .input
        .cur_list
        .build_choices_finish(&mut st.mem, &mut st.save_stack, p);
    if complete {
        return Ok(());
    }
    crate::main_control::push_math(st, GroupCode::MathChoice)?;
    crate::scan::scan_left_brace(st)?;
    Ok(())
}

enum ScriptDuplication {
    Superscript,
    Subscript,
}

impl ListStateRecord {
    fn prepare_sub_sup(
        &mut self,
        mem: &mut MemState,
        cmd: u8,
    ) -> Result<(NodePtr<MathField>, Option<ScriptDuplication>), Abort> {
        let script = |n: NodePtr<Noad>| {
            if cmd == SUP_MARK {
                n.supscr()
            } else {
                n.subscr()
            }
        };
        let mut t = EMPTY;
        let mut p = None;

        if self.tail != self.head {
            if let Ok(Some(n)) = mem.classify_noad(MemIdx::new(self.tail)) {
                if n.noad_type(mem).as_raw() < NodeType::LeftNoad.as_raw() {
                    let field = script(n);
                    t = field.math_type(mem);
                    p = Some(field);
                }
            }
        }

        let mut duplicate = None;
        let p = match p {
            Some(field) if t == EMPTY => field,
            _ => {
                let noad = mem.new_noad().ok_or(Abort::NodeOverflow)?;
                self.tail_append(mem, noad.ptr());
                if t != EMPTY {
                    duplicate = Some(if cmd == SUP_MARK {
                        ScriptDuplication::Superscript
                    } else {
                        ScriptDuplication::Subscript
                    });
                }
                script(noad)
            }
        };
        Ok((p, duplicate))
    }
}

pub(crate) fn sub_sup(st: &mut EngineState) -> Result<(), Abort> {
    let cmd = st.scanner.cur_cmd;
    let (p, duplicate) = st.input.cur_list.prepare_sub_sup(&mut st.mem, cmd)?;
    if let Some(duplicate) = duplicate {
        match duplicate {
            ScriptDuplication::Superscript => {
                crate::output::error_here_with_diagnostic(st, "Double superscript");
                st.errors
                    .help(&["I treat `x^1^2' essentially like `x^1{}^2'."]);
            }
            ScriptDuplication::Subscript => {
                crate::output::error_here_with_diagnostic(st, "Double subscript");
                st.errors
                    .help(&["I treat `x_1_2' essentially like `x_1{}_2'."]);
            }
        }
        crate::output::capture_to_diagnostic(st, std::ptr::null_mut());
        crate::errors::error(st)?;
    }
    crate::scan::scan_math(st, p)
}

impl ListStateRecord {
    fn start_fraction_noad(&mut self, mem: &mut MemState) -> Result<NodePtr<Noad>, Abort> {
        let fraction = NodePtr::<Noad>::alloc_fraction(mem).ok_or(Abort::NodeOverflow)?;
        self.set_aux_s1(fraction.idx().0);
        fraction.set_subtype(mem, NORMAL);
        fraction.supscr().set_math_type(mem, SUB_MLIST);
        fraction.supscr().set_info(mem, self.head.link(mem));
        fraction.subscr().clear(mem);
        fraction.left_delimiter().clear(mem);
        fraction.right_delimiter().clear(mem);

        self.head.set_link(mem, MemPtr::NULL);
        self.tail = self.head;
        Ok(fraction)
    }
}

pub(crate) fn math_fraction(st: &mut EngineState) -> Result<(), Abort> {
    let c = st.scanner.cur_chr;

    if st.input.cur_list.aux_s1() != TEX_NULL {
        if c >= DELIMITED_CODE {
            scan_delimiter(st, NodePtr::at(GARBAGE), false)?;
            scan_delimiter(st, NodePtr::at(GARBAGE), false)?;
        }

        if c % DELIMITED_CODE == ABOVE_CODE {
            crate::scan::scan_dimen(st, false, false, false)?;
        }

        crate::output::print_error_prefix(st);
        crate::output::print(st, "Ambiguous; you need another { and }");
        st.errors.help(&[
            "I'm ignoring this fraction specification, since I don't",
            r"know whether a construction like `x \over y \over z'",
            r"means `{x \over y} \over z' or `x \over {y \over z}'.",
        ]);
        crate::errors::error(st)?;
    } else {
        let aux = st.input.cur_list.start_fraction_noad(&mut st.mem)?;

        if c >= DELIMITED_CODE {
            scan_delimiter(st, aux.left_delimiter(), false)?;
            scan_delimiter(st, aux.right_delimiter(), false)?;
        }

        match c % DELIMITED_CODE {
            ABOVE_CODE => {
                crate::scan::scan_dimen(st, false, false, false)?;
                aux.set_thickness(&mut st.mem, st.scanner.cur_val);
            }
            OVER_CODE => aux.set_thickness(&mut st.mem, DEFAULT_CODE),
            ATOP_CODE => aux.set_thickness(&mut st.mem, 0),
            _ => {}
        }
    }
    Ok(())
}

impl ListStateRecord {
    fn append_accent_noad(&mut self, mem: &mut MemState) -> Result<NodePtr<Noad>, Abort> {
        let accent = NodePtr::<Noad>::alloc_accent(mem).ok_or(Abort::NodeOverflow)?;
        self.tail_append(mem, accent.ptr());
        accent.clear_fields(mem);
        accent.accent_chr().set_math_type(mem, MATH_CHAR);
        Ok(accent)
    }
}

impl MemState {
    fn finish_accent_math_field(&mut self, eqtb: &EqtbState, tail: NodePtr<Noad>, value: i32) {
        tail.accent_chr()
            .set_character(self, (value % 65536) as u16);
        let intpar_cur_fam = eqtb.int_par(INT_PAR_CUR_FAM);
        if math_class_field(value) == 7 && (0..NUMBER_MATH_FAMILIES).contains(&intpar_cur_fam) {
            tail.accent_chr()
                .set_plane_and_fam(self, intpar_cur_fam as u16);
        } else {
            tail.accent_chr()
                .set_plane_and_fam(self, math_fam_field(value) as u16);
        }
        let current_fam = tail.accent_chr().plane_and_fam(self);
        tail.accent_chr().set_plane_and_fam(
            self,
            current_fam + (math_char_field(value) / 65536) as u16 * 256,
        );
    }
}

pub(crate) fn math_left_right(st: &mut EngineState) -> Result<(), Abort> {
    let t = st.scanner.cur_chr as u16;
    if t != NodeType::LeftNoad.as_raw() && st.save_stack.cur_group != GroupCode::MathLeft {
        if st.save_stack.cur_group == GroupCode::MathShift {
            scan_delimiter(st, NodePtr::at(GARBAGE), false)?;
            crate::output::error_here_with_diagnostic(st, "Extra ");
            if t == MIDDLE_NOAD {
                crate::output::print_esc(st, "middle");
                st.errors
                    .help(&[r"I'm ignoring a \middle that had no matching \left."]);
            } else {
                crate::output::print_esc(st, "right");
                st.errors
                    .help(&[r"I'm ignoring a \right that had no matching \left."]);
            }
            crate::output::capture_to_diagnostic(st, std::ptr::null_mut());
            crate::errors::error(st)?;
        } else {
            crate::main_control::off_save(st)?;
        }
    } else {
        let p = st.mem.new_noad().ok_or(Abort::NodeOverflow)?;
        p.set_type_code(&mut st.mem, t);
        scan_delimiter(st, p.delimiter(), false)?;
        if t == MIDDLE_NOAD {
            p.set_noad_type(&mut st.mem, NodeType::RightNoad);
            p.set_subtype(&mut st.mem, MIDDLE_NOAD);
        }
        let q = if t == NodeType::LeftNoad.as_raw() {
            p.idx().0
        } else {
            let q = fin_mlist(st, p.idx().0)?;
            crate::save::unsave(st)?;
            q
        };
        if t != NodeType::RightNoad.as_raw() {
            crate::main_control::push_math(st, GroupCode::MathLeft)?;
            let cl = &mut st.input.cur_list;
            cl.head.set_link(&mut st.mem, MemPtr::from_raw(q));
            cl.tail = p.ptr();
            cl.e_tex_aux = p.ptr();
        } else {
            let inner = st.mem.new_noad().ok_or(Abort::NodeOverflow)?;
            let cl = &mut st.input.cur_list;
            cl.tail_append(&mut st.mem, inner.ptr());
            inner.set_noad_type(&mut st.mem, NodeType::InnerNoad);
            inner.nucleus().set_math_type(&mut st.mem, SUB_MLIST);
            inner.nucleus().set_info(&mut st.mem, MemPtr(q));
        }
    }
    Ok(())
}

#[inline]
fn math_class_field(x: i32) -> i32 {
    (((x as u32) >> 21) & 7) as i32
}
#[inline]
fn math_fam_field(x: i32) -> i32 {
    (((x as u32) >> 24) & 0xFF) as i32
}
#[inline]
fn math_char_field(x: i32) -> i32 {
    ((x as u32) & 0x1F_FFFF) as i32
}
#[inline]
fn set_class_bits(x: i32) -> i32 {
    (((x as u32) & 7) << 21) as i32
}
#[inline]
fn set_family_bits(x: i32) -> i32 {
    (((x as u32) & 0xFF) << 24) as i32
}

pub(crate) fn math_ac(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.cur_cmd == ACCENT {
        crate::output::error_here_with_diagnostic(st, "Please use ");
        crate::output::print_esc(st, "mathaccent");
        crate::output::print(st, " for accents in math mode");
        crate::output::capture_to_diagnostic(st, std::ptr::null_mut());
        st.errors.help(&[
            r"I'm changing \accent to \mathaccent here; wish me luck.",
            "(Accents are not the same in formulas as they are in text.)",
        ]);
        crate::errors::error(st)?;
    }

    let tail = st.input.cur_list.append_accent_noad(&mut st.mem)?;

    if st.scanner.cur_chr == 1 {
        if crate::scan::scan_keyword(st, "fixed")? {
            tail.set_subtype(&mut st.mem, FIXED_ACC);
        } else if crate::scan::scan_keyword(st, "bottom")? {
            if crate::scan::scan_keyword(st, "fixed")? {
                tail.set_subtype(&mut st.mem, BOTTOM_ACC + 1);
            } else {
                tail.set_subtype(&mut st.mem, BOTTOM_ACC);
            }
        }
        crate::scan::scan_math_class_int(st)?;
        let mut c = set_class_bits(st.scanner.cur_val);
        crate::scan::scan_math_fam_int(st)?;
        c += set_family_bits(st.scanner.cur_val);
        crate::scan::scan_usv_num(st)?;
        st.scanner.cur_val = st.scanner.cur_val + c;
    } else {
        crate::scan::scan_fifteen_bit_int(st)?;
        st.scanner.cur_val = set_class_bits(st.scanner.cur_val / 4096)
            + set_family_bits((st.scanner.cur_val % 4096) / 256)
            + (st.scanner.cur_val % 256);
    }

    let value = st.scanner.cur_val;
    st.mem.finish_accent_math_field(&st.eqtb, tail, value);
    crate::scan::scan_math(st, tail.nucleus())
}

pub(crate) fn resume_after_display(st: &mut EngineState) -> Result<(), Abort> {
    if st.save_stack.cur_group != GroupCode::MathShift {
        return Err(Abort::Confusion("display"));
    }

    crate::save::unsave(st)?;
    st.input.cur_list.prev_graf = st.input.cur_list.prev_graf + 3;
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = HMODE;
    st.input.cur_list.set_aux_s0(1000);

    crate::main_control::start_paragraph_language(st);

    crate::scan::get_x_token(st)?;
    if st.scanner.cur_cmd != SPACER {
        crate::input::back_input(st)?;
    }
    if st.input.nest_ptr == 1 {
        crate::pagebuilder::build_page(st)?;
    }
    Ok(())
}

impl FontState {
    #[inline]
    fn sym_font_ok(&self, eqtb: &EqtbState, size: i32) -> bool {
        let f = eqtb.math_font(2 + size);
        self.font_params(f) >= TOTAL_MATHSY_PARAMS || self.font_is_ot_math(f)
    }

    #[inline]
    fn ext_font_ok(&self, eqtb: &EqtbState, size: i32) -> bool {
        let f = eqtb.math_font(3 + size);
        self.font_params(f) >= TOTAL_MATHEX_PARAMS || self.font_is_ot_math(f)
    }
}

impl MemState {
    /// Fills the fresh spec of `skip` with `w` minus the glue of `g`, negating its stretch and shrink.
    fn set_display_skip(&mut self, skip: NodePtr<GlueNode>, g: NodePtr<GlueNode>, w: i32) {
        let src = g.glue_ptr(self);
        let dst = skip.glue_ptr(self);
        dst.set_stretch_order(self, src.stretch_order(self));
        dst.set_shrink_order(self, src.shrink_order(self));
        dst.set_width(self, Scaled(w - src.width(self).0));
        dst.set_stretch(self, Scaled(-src.stretch(self).0));
        dst.set_shrink(self, Scaled(-src.shrink(self).0));
    }
}

fn app_display(st: &mut EngineState, j: Option<AnyBox>, orig: AnyBox, d: i32) -> Result<(), Abort> {
    let s = st.eqtb.dimen_par(DIMEN_PAR_DISPLAY_INDENT);
    let x = st.eqtb.int_par(INT_PAR_PRE_DISPLAY_DIRECTION);
    if x == 0 {
        orig.set_shift_amount(&mut st.mem, Scaled(s + d));
        crate::packaging::append_to_vlist(st, orig)?;
        return Ok(());
    }

    let mut d = d;
    let z = st.eqtb.dimen_par(DIMEN_PAR_DISPLAY_WIDTH);
    let mut e;
    if x > 0 {
        e = z - d - orig.width(&st.mem).0;
    } else {
        e = d;
        d = z - e - orig.width(&st.mem).0;
    }
    let copy = if let Some(j) = j {
        let bc = st.mem.copy_box(j)?;
        let (h, dp) = (orig.height(&st.mem), orig.depth(&st.mem));
        bc.set_height(&mut st.mem, h);
        bc.set_depth(&mut st.mem, dp);
        let s2 = s - bc.shift_amount(&st.mem).0;
        d += s2;
        e = e + bc.width(&st.mem).0 - z - s2;
        Some(bc)
    } else {
        None
    };
    let mut b = copy.unwrap_or(orig);

    let (p, q);
    if orig.subtype(&st.mem) == DLIST {
        p = orig.ptr();
        q = p;
    } else {
        let mut r = orig.list_ptr(&st.mem);
        st.mem.free_node(orig.ptr(), BOX_NODE_SIZE);
        if r.is_null() {
            return Err(Abort::Confusion("LR4"));
        }
        if x > 0 {
            p = r;
            q = st.mem.list_tail(r);
        } else {
            let mut rev = MemPtr::NULL;
            q = r;
            while !r.is_null() {
                let t = r.link(&st.mem);
                r.set_link(&mut st.mem, rev);
                rev = r;
                r = t;
            }
            p = rev;
        }
    }

    let (r, t) = match copy {
        None => (
            st.mem.new_kern(0).ok_or(Abort::NodeOverflow)?.ptr(),
            st.mem.new_kern(0).ok_or(Abort::NodeOverflow)?.ptr(),
        ),
        Some(bc) => {
            let r = bc.list_ptr(&st.mem);
            (r, r.link(&st.mem))
        }
    };
    let u = st
        .mem
        .new_math(0, END_M_CODE)
        .ok_or(Abort::NodeOverflow)?
        .ptr();
    if let Ok(AnyNode::Glue(tg)) = st.mem.classify(MemIdx::new(t.0)) {
        let skip = st
            .mem
            .new_skip_param(&st.eqtb, GLUE_PAR_RIGHT_SKIP)
            .ok_or(Abort::NodeOverflow)?;
        q.set_link(&mut st.mem, skip.ptr());
        skip.set_link(&mut st.mem, u);
        st.mem.set_display_skip(skip, tg, e);
        u.set_link(&mut st.mem, t);
    } else {
        st.mem.set_b32s1(t + 1, e);
        t.set_link(&mut st.mem, u);
        q.set_link(&mut st.mem, t);
    }

    let u = st
        .mem
        .new_math(0, BEGIN_M_CODE)
        .ok_or(Abort::NodeOverflow)?
        .ptr();
    if let Ok(AnyNode::Glue(rg)) = st.mem.classify(MemIdx::new(r.0)) {
        let skip = st
            .mem
            .new_skip_param(&st.eqtb, GLUE_PAR_LEFT_SKIP)
            .ok_or(Abort::NodeOverflow)?;
        u.set_link(&mut st.mem, skip.ptr());
        skip.set_link(&mut st.mem, p);
        st.mem.set_display_skip(skip, rg, d);
        r.set_link(&mut st.mem, u);
    } else {
        st.mem.set_b32s1(r + 1, d);
        r.set_link(&mut st.mem, p);
        u.set_link(&mut st.mem, r);
        match copy {
            None => {
                b = crate::packaging::hpack(st, u, 0, ADDITIONAL)?;
                b.set_shift_amount(&mut st.mem, Scaled(s));
            }
            Some(bc) => bc.set_list_ptr(&mut st.mem, u),
        }
    }
    crate::packaging::append_to_vlist(st, b)?;
    Ok(())
}

/// Checks the family 2 and 3 fonts. On failure reports the error, flushes the
/// formula and returns `true` (TeX's `danger`).
fn check_math_fonts(st: &mut EngineState) -> Result<bool, Abort> {
    let sizes = [TEXT_SIZE, SCRIPT_SIZE, SCRIPT_SCRIPT_SIZE];
    let (msg, help) = if !sizes.iter().all(|&z| st.fonts.sym_font_ok(&st.eqtb, z)) {
        (
            "Math formula deleted: Insufficient symbol fonts",
            [
                r"Sorry, but I can't typeset math unless \textfont 2",
                r"and \scriptfont 2 and \scriptscriptfont 2 have all",
                r"the \fontdimen values needed in math symbol fonts.",
            ],
        )
    } else if !sizes.iter().all(|&z| st.fonts.ext_font_ok(&st.eqtb, z)) {
        (
            "Math formula deleted: Insufficient extension fonts",
            [
                r"Sorry, but I can't typeset math unless \textfont 3",
                r"and \scriptfont 3 and \scriptscriptfont 3 have all",
                r"the \fontdimen values needed in math extension fonts.",
            ],
        )
    } else {
        return Ok(false);
    };
    crate::output::print_err(st, msg);
    st.errors.help(&help);
    crate::errors::error(st)?;
    st.input.cur_list.flush_math(&mut st.mem)?;
    Ok(true)
}

pub(crate) fn check_display_end(st: &mut EngineState) -> Result<(), Abort> {
    crate::scan::get_x_token(st)?;
    if st.scanner.cur_cmd != MATH_SHIFT {
        crate::output::print_err(st, "Display math should end with $$");
        st.errors.help(&[
            "The `$' that I just saw supposedly matches a previous `$$'.",
            "So I shall assume that you typed `$$' both times.",
        ]);
        crate::input::back_error(st)?;
    }
    Ok(())
}

/// The box `init_math` saved for the paragraph line before a display, if any.
fn pre_display_box(st: &EngineState) -> Option<AnyBox> {
    if st.input.cur_list.mode != MMODE {
        return None;
    }
    let j = st.input.cur_list.e_tex_aux.as_idx()?;
    st.mem.classify_box(j).ok().flatten()
}

pub(crate) fn after_math(st: &mut EngineState) -> Result<(), Abort> {
    let mut j = pre_display_box(st);
    let mut danger = check_math_fonts(st)?;
    let mut m = st.input.cur_list.mode as i32;
    let mut l = false;
    let mut p = fin_mlist(st, TEX_NULL)?;

    let a;
    if st.input.cur_list.mode as i32 == -m {
        check_display_end(st)?;
        st.scanner.cur_mlist.0 = p;
        st.scanner.cur_style = TEXT_STYLE;
        st.scanner.mlist_penalties = false;
        mlist_to_hlist(st)?;
        let eqno = crate::packaging::hpack(st, TEMP_HEAD.link(&st.mem), 0, ADDITIONAL)?;
        eqno.set_subtype(&mut st.mem, DLIST);
        a = Some(eqno);
        crate::save::unsave(st)?;
        st.save_stack.save_ptr -= 1;
        if st.save_stack.saved(0) == 1 {
            l = true;
        }

        j = pre_display_box(st);
        danger = check_math_fonts(st)?;
        m = st.input.cur_list.mode as i32;
        p = fin_mlist(st, TEX_NULL)?;
    } else {
        a = None;
    }

    if m < 0 {
        let math_surround = st.eqtb.dimen_par(DIMEN_PAR_MATH_SURROUND);
        let node = st
            .mem
            .new_math(math_surround, BEFORE)
            .ok_or(Abort::NodeOverflow)?;
        st.input.cur_list.tail_append(&mut st.mem, node.ptr());
        st.scanner.cur_mlist.0 = p;
        st.scanner.cur_style = TEXT_STYLE;
        st.scanner.mlist_penalties = st.input.cur_list.mode > 0;
        mlist_to_hlist(st)?;
        let hlist = TEMP_HEAD.link(&st.mem);
        let cl = &mut st.input.cur_list;
        cl.tail.set_link(&mut st.mem, hlist);
        while !cl.tail.link(&st.mem).is_null() {
            cl.tail = cl.tail.link(&st.mem);
        }
        let node = st
            .mem
            .new_math(math_surround, AFTER)
            .ok_or(Abort::NodeOverflow)?;
        st.input.cur_list.tail_append(&mut st.mem, node.ptr());
        st.input.cur_list.set_aux_s0(1000);

        if st.config.semantic_pagination_enabled {
            crate::ext::tt_insert_special(st, "tdux:ce math")?;
        }

        crate::save::unsave(st)?;
        return Ok(());
    }

    if a.is_none() {
        check_display_end(st)?;
    }
    st.scanner.cur_mlist.0 = p;
    st.scanner.cur_style = DISPLAY_STYLE;
    st.scanner.mlist_penalties = false;
    mlist_to_hlist(st)?;
    let mut p = TEMP_HEAD.link(&st.mem);
    st.scanner.adjust_tail = ADJUST_HEAD;
    st.scanner.pre_adjust_tail = PRE_ADJUST_HEAD;
    let mut b = crate::packaging::hpack(st, p, 0, ADDITIONAL)?;
    p = b.list_ptr(&st.mem);
    let t = st.scanner.adjust_tail.0;
    st.scanner.adjust_tail.0 = TEX_NULL;
    let pre_t = st.scanner.pre_adjust_tail.0;
    st.scanner.pre_adjust_tail.0 = TEX_NULL;
    let mut w = b.width(&st.mem).0;
    let z = st.eqtb.dimen_par(DIMEN_PAR_DISPLAY_WIDTH);
    let mut s = st.eqtb.dimen_par(DIMEN_PAR_DISPLAY_INDENT);
    if st.eqtb.int_par(INT_PAR_PRE_DISPLAY_DIRECTION) < 0 {
        s = -s - z;
    }
    let (mut e, q);
    match a {
        Some(eqno) if !danger => {
            e = eqno.width(&st.mem).0;
            q = e + st.fonts.math_quad(&st.eqtb, TEXT_SIZE);
        }
        _ => {
            e = 0;
            q = 0;
        }
    }
    if w + q > z {
        if e != 0
            && (w - st.scanner.total_shrink[0] + q <= z
                || st.scanner.total_shrink[FIL as usize] != 0
                || st.scanner.total_shrink[FILL as usize] != 0
                || st.scanner.total_shrink[FILLL as usize] != 0)
        {
            st.mem.free_node(b.ptr(), BOX_NODE_SIZE);
            b = crate::packaging::hpack(st, p, z - q, EXACTLY)?;
        } else {
            e = 0;
            if w > z {
                st.mem.free_node(b.ptr(), BOX_NODE_SIZE);
                b = crate::packaging::hpack(st, p, z, EXACTLY)?;
            }
        }
        w = b.width(&st.mem).0;
    }
    b.set_subtype(&mut st.mem, DLIST);
    let mut d = crate::scaled_math::half(z - w);
    if e > 0 && d < 2 * e {
        d = crate::scaled_math::half(z - w - e);
        if let Some(first) = p.as_idx() {
            if !st.mem.is_char_node(p) && st.mem.node_type(first) == Ok(NodeType::Glue) {
                d = 0;
            }
        }
    }

    let pen = st
        .mem
        .new_penalty(st.eqtb.int_par(INT_PAR_PRE_DISPLAY_PENALTY))
        .ok_or(Abort::NodeOverflow)?;
    st.input.cur_list.tail_append(&mut st.mem, pen.ptr());

    let pre_display_size = st.eqtb.dimen_par(DIMEN_PAR_PRE_DISPLAY_SIZE);
    let (g1, mut g2) = if d + s <= pre_display_size || l {
        (GLUE_PAR_ABOVE_DISPLAY_SKIP, GLUE_PAR_BELOW_DISPLAY_SKIP)
    } else {
        (
            GLUE_PAR_ABOVE_DISPLAY_SHORT_SKIP,
            GLUE_PAR_BELOW_DISPLAY_SHORT_SKIP,
        )
    };

    if let Some(eqno) = a.filter(|_| l && e == 0) {
        app_display(st, j, eqno, 0)?;
        let pen = st.mem.new_penalty(INF_PENALTY).ok_or(Abort::NodeOverflow)?;
        st.input.cur_list.tail_append(&mut st.mem, pen.ptr());
    } else {
        let glue = st
            .mem
            .new_param_glue(&st.eqtb, g1)
            .ok_or(Abort::NodeOverflow)?;
        st.input.cur_list.tail_append(&mut st.mem, glue.ptr());
    }

    if let Some(eqno) = a.filter(|_| e != 0) {
        let r = st.mem.new_kern(z - w - e - d).ok_or(Abort::NodeOverflow)?;
        if l {
            eqno.set_link(&mut st.mem, r.ptr());
            r.set_link(&mut st.mem, b.ptr());
            b = eqno;
            d = 0;
        } else {
            b.set_link(&mut st.mem, r.ptr());
            r.set_link(&mut st.mem, eqno.ptr());
        }
        b = crate::packaging::hpack(st, b.ptr(), 0, ADDITIONAL)?;
    }

    app_display(st, j, b, d)?;

    if let Some(eqno) = a.filter(|_| e == 0 && !l) {
        let pen = st.mem.new_penalty(INF_PENALTY).ok_or(Abort::NodeOverflow)?;
        st.input.cur_list.tail_append(&mut st.mem, pen.ptr());
        let a_width = eqno.width(&st.mem).0;
        app_display(st, j, eqno, z - a_width)?;
        g2 = 0;
    }

    let cl = &mut st.input.cur_list;
    if t != ADJUST_HEAD.0 {
        let adjusts = ADJUST_HEAD.link(&st.mem);
        cl.tail.set_link(&mut st.mem, adjusts);
        cl.tail.0 = t;
    }
    if pre_t != PRE_ADJUST_HEAD.0 {
        let pre_adjusts = PRE_ADJUST_HEAD.link(&st.mem);
        cl.tail.set_link(&mut st.mem, pre_adjusts);
        cl.tail.0 = pre_t;
    }

    let pen = st
        .mem
        .new_penalty(st.eqtb.int_par(INT_PAR_POST_DISPLAY_PENALTY))
        .ok_or(Abort::NodeOverflow)?;
    st.input.cur_list.tail_append(&mut st.mem, pen.ptr());

    if g2 > 0 {
        let glue = st
            .mem
            .new_param_glue(&st.eqtb, g2)
            .ok_or(Abort::NodeOverflow)?;
        st.input.cur_list.tail_append(&mut st.mem, glue.ptr());
    }

    if let Some(j) = j {
        st.mem.flush_node_list(j.ptr())?;
    }

    if st.config.semantic_pagination_enabled {
        crate::ext::tt_insert_special(st, "tdux:ce dmath")?;
    }

    resume_after_display(st)
}

/// TeX's `\predisplaydirection` for the paragraph just ended.
fn pre_display_direction(st: &EngineState) -> i32 {
    let aux = st.input.cur_list.e_tex_aux;
    if aux.is_null() {
        0
    } else if aux.info(&st.mem) >= R_CODE.into() {
        -1
    } else {
        1
    }
}

/// A kern, or the parameter glue when that skip is nonzero.
fn side_skip(st: &mut EngineState, param: i16) -> Result<MemPtr, Abort> {
    let p = if st.eqtb.glue_par(param) == ZERO_GLUE {
        st.mem.new_kern(0).map(|k| k.ptr())
    } else {
        st.mem.new_param_glue(&st.eqtb, param).map(|g| g.ptr())
    };
    p.ok_or(Abort::NodeOverflow)
}

pub(crate) fn init_math(st: &mut EngineState) -> Result<(), Abort> {
    crate::scan::get_token(st)?;

    if st.scanner.cur_cmd == MATH_SHIFT && st.input.cur_list.mode > 0 {
        let mut j = TEX_NULL;
        let mut w = -MAX_HALFWORD;
        let x;

        if st.input.cur_list.head == st.input.cur_list.tail {
            st.mem.pop_nest(&mut st.input);
            x = pre_display_direction(st);
        } else {
            crate::linebreak::line_break(st, true)?;

            let right = side_skip(st, GLUE_PAR_RIGHT_SKIP)?;
            let left = side_skip(st, GLUE_PAR_LEFT_SKIP)?;
            left.set_link(&mut st.mem, right);

            let jbox = st.mem.new_null_box().ok_or(Abort::NodeOverflow)?;
            j = jbox.idx().0;
            let jb = st.scanner.just_box.expect("line_break sets just_box");
            let mem = &mut st.mem;
            jbox.set_width(mem, jb.width(mem));
            jbox.set_shift_amount(mem, jb.shift_amount(mem));
            jbox.set_list_ptr(mem, left);
            jbox.set_glue_order(mem, jb.glue_order(mem));
            jbox.set_glue_sign(mem, jb.glue_sign(mem));
            jbox.set_glue_set(mem, jb.glue_set(mem));

            let mut v = jb.shift_amount(&st.mem).0;
            x = pre_display_direction(st);

            let mut p;
            if x >= 0 {
                p = jb.list_ptr(&st.mem);
                TEMP_HEAD.set_link(&mut st.mem, MemPtr::NULL);
            } else {
                v = -v - jb.width(&st.mem).0;
                let begin = st
                    .mem
                    .new_math(0, BEGIN_L_CODE)
                    .ok_or(Abort::NodeOverflow)?
                    .ptr();
                TEMP_HEAD.set_link(&mut st.mem, begin);
                let end = st
                    .mem
                    .new_math(0, END_L_CODE)
                    .ok_or(Abort::NodeOverflow)?
                    .ptr();
                let list = jb.list_ptr(&st.mem);
                st.mem.just_copy(list, begin, end)?;
                st.scanner.cur_dir = RIGHT_TO_LEFT;
                p = begin;
            }

            let cur_font = st.eqtb.cur_font();
            v += 2 * st.fonts.param(cur_font, QUAD_CODE);

            let texxet = st.eqtb.int_par(INT_PAR_TEXXET) > 0;
            if texxet {
                st.mem
                    .push_lr(&mut st.scanner.lr_ptr, BEFORE.into())
                    .ok_or(Abort::NodeOverflow)?;
            }

            'outer: while !p.is_null() {
                let (d, found) = 'reswitch: loop {
                    if let Some(ch) = st.mem.as_char(p) {
                        let f = ch.font(&st.mem).0;
                        let c = st.fonts.effective_char(f, ch.character(&st.mem));
                        let d = st.fonts.char_width(f, st.fonts.char_info(f, c));
                        break (d, true);
                    }
                    break match st.mem.classify(MemIdx::new(p)) {
                        Ok(node @ (AnyNode::HList(_) | AnyNode::VList(_) | AnyNode::Rule(_))) => {
                            (node.hlist_width(&st.mem), true)
                        }
                        Ok(AnyNode::Ligature(lig)) => {
                            st.mem.set_word(GARBAGE, st.mem.word(lig.char_word().idx()));
                            let next = lig.link(&st.mem);
                            GARBAGE.set_link(&mut st.mem, next);
                            p = GARBAGE;
                            st.fonts.xtx_ligature_present = true;
                            continue 'reswitch;
                        }
                        Ok(AnyNode::Kern(k)) => (k.width(&st.mem).0, false),
                        Ok(AnyNode::MarginKern(k)) => (k.width(&st.mem).0, false),
                        Ok(AnyNode::Math(m)) => {
                            let d = m.width(&st.mem).0;
                            let subtype = m.subtype(&st.mem);
                            if texxet {
                                if subtype & 1 != 0 {
                                    if st.scanner.lr_ptr.info(&st.mem)
                                        == crate::mem::end_lr_type(subtype)
                                    {
                                        st.mem.pop_lr(&mut st.scanner.lr_ptr);
                                    } else if subtype > L_CODE {
                                        w = MAX_HALFWORD;
                                        break 'outer;
                                    }
                                } else {
                                    st.mem
                                        .push_lr(
                                            &mut st.scanner.lr_ptr,
                                            crate::mem::end_lr_type(subtype),
                                        )
                                        .ok_or(Abort::NodeOverflow)?;
                                    if (subtype / R_CODE) as i32 != st.scanner.cur_dir as i32 {
                                        st.mem.just_reverse(&mut st.scanner, p)?;
                                        p = TEMP_HEAD;
                                    }
                                }
                            } else if subtype >= L_CODE {
                                w = MAX_HALFWORD;
                                break 'outer;
                            }
                            (d, false)
                        }
                        // An EDGE_NODE, which shares type 14 with STYLE_NODE.
                        Ok(AnyNode::Style(edge)) => {
                            st.scanner.cur_dir = edge.subtype(&st.mem) as i16;
                            (edge.as_edge().width(&st.mem).0, false)
                        }
                        Ok(AnyNode::Glue(g)) => {
                            let spec = g.glue_ptr(&st.mem);
                            let order = jb.glue_order(&st.mem);
                            match jb.glue_sign(&st.mem) {
                                GlueSign::Stretching
                                    if order == spec.stretch_order(&st.mem)
                                        && spec.stretch(&st.mem).0 != 0 =>
                                {
                                    v = MAX_HALFWORD;
                                }
                                GlueSign::Shrinking
                                    if order == spec.shrink_order(&st.mem)
                                        && spec.shrink(&st.mem).0 != 0 =>
                                {
                                    v = MAX_HALFWORD;
                                }
                                _ => {}
                            }
                            (spec.width(&st.mem).0, g.subtype(&st.mem) >= A_LEADERS)
                        }
                        Ok(AnyNode::Whatsit(wn))
                            if wn
                                .whatsit_subtype(&st.mem)
                                .is_some_and(|s| s.has_dimensions()) =>
                        {
                            (wn.width(&st.mem).0, true)
                        }
                        _ => (0, false),
                    };
                };

                if found {
                    if v < MAX_HALFWORD {
                        v += d;
                        w = v;
                    } else {
                        w = MAX_HALFWORD;
                        break 'outer;
                    }
                } else if v < MAX_HALFWORD {
                    v += d;
                }

                p = p.link(&st.mem);
            }

            if texxet {
                while !st.scanner.lr_ptr.is_null() {
                    st.mem.pop_lr(&mut st.scanner.lr_ptr);
                }
                if st.scanner.lr_problems != 0 {
                    w = MAX_HALFWORD;
                    st.scanner.lr_problems = 0;
                }
            }

            st.scanner.cur_dir = LEFT_TO_RIGHT;
            let temp_head = TEMP_HEAD.link(&st.mem);
            st.mem.flush_node_list(temp_head)?;
        }

        let l;
        let s;
        if let Some(shape) = st.eqtb.par_shape() {
            let line = (st.input.cur_list.prev_graf + 2).min(shape.lines(&st.mem));
            s = shape.indent(&st.mem, line);
            l = shape.line_width(&st.mem, line);
        } else {
            let hang = st.eqtb.dimen_par(DIMEN_PAR_HANG_INDENT);
            let hang_after = st.eqtb.int_par(INT_PAR_HANG_AFTER);
            let prev_graf = st.input.cur_list.prev_graf;
            if hang != 0
                && ((hang_after >= 0 && prev_graf + 2 > hang_after) || prev_graf + 1 < -hang_after)
            {
                l = st.eqtb.dimen_par(DIMEN_PAR_HSIZE) - hang.abs();
                s = if hang > 0 { hang } else { 0 };
            } else {
                l = st.eqtb.dimen_par(DIMEN_PAR_HSIZE);
                s = 0;
            }
        }

        crate::main_control::push_math(st, GroupCode::MathShift)?;
        st.input.cur_list.mode = MMODE;
        define_word(st, INT_BASE + INT_PAR_CUR_FAM, -1)?;
        define_word(st, DIMEN_BASE + DIMEN_PAR_PRE_DISPLAY_SIZE, w)?;
        st.input.cur_list.e_tex_aux = MemPtr::from_raw(j);
        define_word(st, INT_BASE + INT_PAR_PRE_DISPLAY_DIRECTION, x)?;
        define_word(st, DIMEN_BASE + DIMEN_PAR_DISPLAY_WIDTH, l)?;
        define_word(st, DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT, s)?;

        if st.config.semantic_pagination_enabled {
            crate::ext::tt_insert_special(st, "tdux:cs dmath")?;
        }

        let every_display = st.eqtb.local(LOCAL_EVERY_DISPLAY);
        if !every_display.is_null() {
            crate::input::begin_token_list(st, every_display, EVERY_DISPLAY_TEXT)?;
        }
        if st.input.nest_ptr == 1 {
            crate::pagebuilder::build_page(st)?;
        }
    } else {
        crate::input::back_input(st)?;
        crate::main_control::push_math(st, GroupCode::MathShift)?;
        define_word(st, INT_BASE + INT_PAR_CUR_FAM, -1)?;
        if st.shipout.insert_src_special_every_math {
            crate::ext::insert_src_special(st)?;
        }
        if st.config.semantic_pagination_enabled {
            crate::ext::tt_insert_special(st, "tdux:cs math")?;
        }
        let every_math = st.eqtb.local(LOCAL_EVERY_MATH);
        if !every_math.is_null() {
            crate::input::begin_token_list(st, every_math, EVERY_MATH_TEXT)?;
        }
    }
    Ok(())
}

fn rebox(st: &mut EngineState, b: AnyBox, w: i32) -> Result<AnyBox, Abort> {
    let mut b = b;
    if b.width(&st.mem).0 != w && !b.list_ptr(&st.mem).is_null() && matches!(b, AnyBox::VList(_)) {
        b = crate::packaging::hpack(st, b.ptr(), 0, ADDITIONAL)?;
    }
    if b.width(&st.mem).0 == w || b.list_ptr(&st.mem).is_null() {
        b.set_width(&mut st.mem, Scaled(w));
        return Ok(b);
    }

    let mut p = b.list_ptr(&st.mem);
    if let Some(ch) = st.mem.as_char(p).filter(|_| p.link(&st.mem).is_null()) {
        let f = ch.font(&st.mem).0;
        let c = st.fonts.effective_char(f, ch.character(&st.mem));
        let q = st.fonts.char_info(f, c);
        let v = st.fonts.char_width(f, q);
        let bw = b.width(&st.mem).0;
        if v != bw {
            let k = st.mem.new_kern(bw - v).ok_or(Abort::NodeOverflow)?;
            p.set_link(&mut st.mem, k.ptr());
        }
    }
    let mem = &mut st.mem;
    mem.free_node(b.ptr(), BOX_NODE_SIZE);
    let g = mem.new_glue(SS_GLUE).ok_or(Abort::NodeOverflow)?;
    g.set_link(mem, p);
    while !p.link(mem).is_null() {
        p = p.link(mem);
    }
    let tail_glue = mem.new_glue(SS_GLUE).ok_or(Abort::NodeOverflow)?;
    p.set_link(mem, tail_glue.ptr());
    crate::packaging::hpack(st, g.ptr(), w, EXACTLY)
}

fn build_opentype_assembly(
    st: &mut EngineState,
    f: i32,
    a: &GlyphAssembly,
    s: i32,
    horiz: bool,
) -> Result<AnyBox, Abort> {
    let b = if horiz {
        AnyBox::HList(st.mem.new_null_box().ok_or(Abort::NodeOverflow)?)
    } else {
        AnyBox::VList(
            st.mem
                .new_null_box()
                .ok_or(Abort::NodeOverflow)?
                .into_vlist(&mut st.mem),
        )
    };
    // Extenders repeat `n` times, other parts once.
    let repeats = |i: i32, n: i32| if a.is_extender(i) { n } else { 1 };
    let no_extenders = (0..a.count()).all(|i| !a.is_extender(i));
    let min_o = st.fonts.ot_min_connector_overlap(f);
    let mut n: i32 = -1;
    loop {
        n += 1;
        let mut s_max: i32 = 0;
        let mut prev_o: i32 = 0;
        for i in 0..a.count() {
            for _ in 0..repeats(i, n) {
                let o = a.start_connector(&st.fonts, f, i).min(min_o).min(prev_o);
                s_max = s_max - o + a.full_advance(&st.fonts, f, i);
                prev_o = a.end_connector(&st.fonts, f, i);
            }
        }
        if s_max >= s || no_extenders {
            break;
        }
    }
    let mut prev_o: i32 = 0;
    for i in 0..a.count() {
        for _ in 0..repeats(i, n) {
            let oo = a.start_connector(&st.fonts, f, i).min(prev_o);
            if oo > 0 {
                st.mem.stack_glue_into_box(b, -oo, -oo.min(min_o))?;
            }
            stack_glyph_into_box(st, b, f, a.glyph(i))?;
            prev_o = a.end_connector(&st.fonts, f, i);
        }
    }
    let mut p = b.list_ptr(&st.mem);
    let mut nat: i32 = 0;
    let mut str: i32 = 0;
    while !p.is_null() {
        match st.mem.classify(MemIdx::new(p)) {
            Ok(AnyNode::Whatsit(pw)) => {
                if horiz {
                    nat += pw.width(&st.mem).0;
                } else {
                    nat += pw.height(&st.mem).0 + pw.depth(&st.mem).0;
                }
            }
            Ok(AnyNode::Glue(gn)) => {
                let gs = gn.glue_ptr(&st.mem);
                nat += gs.width(&st.mem).0;
                str += gs.stretch(&st.mem).0;
            }
            _ => {}
        }
        p = p.link(&st.mem);
    }
    if s > nat && str > 0 {
        let o = (s - nat).min(str);
        b.set_glue_order(&mut st.mem, NORMAL);
        b.set_glue_sign(&mut st.mem, GlueSign::Stretching);
        let ratio = o as f64 / str as f64;
        b.set_glue_set(&mut st.mem, ratio);
        let set_dim = Scaled(nat + crate::scaled_math::tex_round(str as f64 * ratio));
        if horiz {
            b.set_width(&mut st.mem, set_dim);
        } else {
            b.set_height(&mut st.mem, set_dim);
        }
    } else if horiz {
        b.set_width(&mut st.mem, Scaled(nat));
    } else {
        b.set_height(&mut st.mem, Scaled(nat));
    }
    Ok(b)
}

impl MemState {
    fn build_ext_delimiter_box(
        &mut self,
        fonts: &mut FontState,
        f: i32,
        q: CharInfo,
        v: i32,
    ) -> Result<AnyBox, Abort> {
        let vb = AnyBox::VList(
            self.new_null_box()
                .ok_or(Abort::NodeOverflow)?
                .into_vlist(self),
        );
        let r = fonts.font_info_b16(fonts.exten_base_entry(f) + q.s0 as i32);
        let u = fonts.height_plus_depth(f, r.s0);
        let effective_c = fonts.effective_char(f, r.s0);
        let qq = fonts.char_info(f, effective_c);
        let box_width = fonts.char_width(f, qq) + fonts.char_italic(f, qq);
        vb.set_width(self, Scaled(box_width));
        let mut w = 0;
        if r.s1 != 0 {
            w += fonts.height_plus_depth(f, r.s1);
        }
        if r.s2 != 0 {
            w += fonts.height_plus_depth(f, r.s2);
        }
        if r.s3 != 0 {
            w += fonts.height_plus_depth(f, r.s3);
        }

        let mut n: i32 = 0;
        if u > 0 {
            while w < v {
                w += u;
                n += 1;
                if r.s2 != 0 {
                    w += u;
                }
            }
        }
        if r.s1 != 0 {
            let p = self.char_box_tfm(fonts, f, r.s1)?;
            self.stack_into_box(vb, p);
        }
        for _ in 1..=n {
            let p = self.char_box_tfm(fonts, f, r.s0)?;
            self.stack_into_box(vb, p);
        }
        if r.s2 != 0 {
            let p = self.char_box_tfm(fonts, f, r.s2)?;
            self.stack_into_box(vb, p);
            for _ in 1..=n {
                let p = self.char_box_tfm(fonts, f, r.s0)?;
                self.stack_into_box(vb, p);
            }
        }
        if r.s3 != 0 {
            let p = self.char_box_tfm(fonts, f, r.s3)?;
            self.stack_into_box(vb, p);
        }
        let height = vb.height(self).0;
        vb.set_depth(self, Scaled(w - height));
        Ok(vb)
    }
}

fn var_delimiter(
    st: &mut EngineState,
    d: NodePtr<DelimiterField>,
    s: i32,
    v: i32,
) -> Result<AnyBox, Abort> {
    let mut f: i32 = FONT_BASE;
    let mut ot_assembly: Option<GlyphAssembly> = None;
    let mut c: u16 = 0;
    let mut w: i32 = 0;
    let mut q = CharInfo {
        s3: 0,
        s2: 0,
        s1: 0,
        s0: 0,
    };
    let mut large_attempt = false;
    let (mut z, mut x) = d.small(&st.mem);
    'found: {
        loop {
            if z != 0 || x != 0 {
                z = z + s + 256;
                loop {
                    z -= 256;
                    let g = st.eqtb.math_font(z);
                    if g != FONT_BASE {
                        if st.fonts.font_area_entry(g) == AAT_FONT_FLAG
                            || st.fonts.font_uses_opentype(g)
                        {
                            // SAFETY: pure font-table and FFI glyph lookup.
                            x = unsafe { st.fonts.map_char_to_glyph(g, x) }?;
                            f = g;
                            c = x as u16;
                            w = 0;
                            let mut n: i32 = 0;
                            let mut u: i32 = 0;
                            loop {
                                let yv =
                                    st.fonts.get_ot_math_variant(g, x, n, &mut u, false) as u16;
                                if u > w {
                                    c = yv;
                                    w = u;
                                    if u >= v {
                                        break 'found;
                                    }
                                }
                                n += 1;
                                if u < 0 {
                                    break;
                                }
                            }
                            ot_assembly = st.fonts.get_ot_assembly(g, x, false);
                            if ot_assembly.is_some() {
                                break 'found;
                            }
                        } else {
                            let fonts = &st.fonts;
                            let mut y = x as u16;
                            if y >= fonts.font_bc_entry(g) && y <= fonts.font_ec_entry(g) {
                                loop {
                                    q = fonts.char_info(g, y as i32);
                                    if q.s3 == 0 {
                                        break;
                                    }
                                    if q.s1 % 4 == EXT_TAG {
                                        f = g;
                                        c = y;
                                        break 'found;
                                    }
                                    let u = fonts.char_height(g, q) + fonts.char_depth(g, q);
                                    if u > w {
                                        f = g;
                                        c = y;
                                        w = u;
                                        if u >= v {
                                            break 'found;
                                        }
                                    }
                                    if q.s1 % 4 != LIST_TAG {
                                        break;
                                    }
                                    y = q.s0;
                                }
                            }
                        }
                    }
                    if z < SCRIPT_SIZE {
                        break;
                    }
                }
            }
            if large_attempt {
                break 'found;
            }
            large_attempt = true;
            (z, x) = d.large(&st.mem);
        }
    }
    let b = if f == FONT_BASE {
        let b = st.mem.new_null_box().ok_or(Abort::NodeOverflow)?;
        let width = st.eqtb.dimen_par(DIMEN_PAR_NULL_DELIMITER_SPACE);
        b.set_width(&mut st.mem, Scaled(width));
        AnyBox::HList(b)
    } else if !st.fonts.font_uses_opentype(f) {
        if q.s1 % 4 == EXT_TAG {
            st.mem.build_ext_delimiter_box(&mut st.fonts, f, q, v)?
        } else {
            char_box(st, f, c as i32)?
        }
    } else if let Some(assembly) = &ot_assembly {
        build_opentype_assembly(st, f, assembly, v, false)?
    } else {
        native_glyph_vbox(st, f, c)?
    };
    let axis = st.fonts.axis_height(&st.eqtb, s);
    let shift = crate::scaled_math::half(b.height(&st.mem).0 - b.depth(&st.mem).0) - axis;
    b.set_shift_amount(&mut st.mem, Scaled(shift));
    Ok(b)
}
