//! Math-mode command helpers translated from `xetex/xetex-math.c`.
use crate::consts::{
    BOX_NODE_SIZE, CUR_FONT_LOC, DEL_CODE_BASE, DIMEN_BASE, GLUE_BASE, GLUE_NODE, HLIST_NODE,
    INT_BASE, KERN_NODE, LOCAL_BASE, MATH_FONT_BASE, MAX_HALFWORD, MEDIUM_NODE_SIZE, PENALTY_NODE,
};
use crate::ext::real_get_native_glyph;
use crate::mem::{
    free_node, get_avail, get_node, new_choice, new_glue, new_kern, new_noad, new_null_box,
    new_spec, ALLOC_FAILED, TEX_NULL,
};
use crate::node::{
    AnyBox, AnyNode, ChoiceNode, GlueNode, GlueSpecNode, HListNode, KernNode, Noad, NodePtr,
    RuleDimen, RuleNode, VListNode, WhatsitNode, WhatsitSubtype,
};
use crate::scan::NodeRecycleAbort;
use crate::state::{EngineState, EqtbState, FontState, GroupCode, MemState, SaveStackState, ScannerState};
use crate::units::{MemIdx, MemPtr, NodeType, Scaled};

const INT_PAR_CUR_FAM: i32 = 44;
const LOCAL_EVERY_MATH: i32 = 3;

const EVERY_MATH_TEXT: u16 = 9;

const RELAX: u8 = 0;
const SPACER: u8 = 10;
const LETTER: u8 = 11;
const OTHER_CHAR: u8 = 12;
const DELIM_NUM: u8 = 15;
const SUP_MARK: u8 = 7;
const EMPTY: i32 = 0;
const SUB_MLIST: i32 = 3;
const DELIMITED_CODE: i32 = 3;
const ABOVE_CODE: i32 = 0;
const OVER_CODE: i32 = 1;
const ATOP_CODE: i32 = 2;
const DEFAULT_CODE: i32 = 0x4000_0000;
const GARBAGE: i32 = 4_999_987;

const HMODE: i16 = 104;
const BIGGEST_LANG: i32 = 255;
const INT_PAR_LANGUAGE: i32 = 50;
const INT_PAR_LEFT_HYPHEN_MIN: i32 = 51;
const INT_PAR_RIGHT_HYPHEN_MIN: i32 = 52;
const ACCENT: u8 = 45;
const ACCENT_NOAD_SIZE: i32 = 5;
const MATH_CHAR: i32 = 1;
const FIXED_ACC: u16 = 1;
const BOTTOM_ACC: u16 = 2;
const NUMBER_MATH_FAMILIES: i32 = 256;

const NORMAL: u16 = 0;
const LIMITS: u16 = 1;
const LIST_TAG: u16 = 2;
const EXT_TAG: u16 = 3;
const DIMEN_PAR_NULL_DELIMITER_SPACE: i32 = 11;
const WHATSIT_NODE: u16 = 8;
const GLYPH_NODE_SIZE: i32 = 5;
const DISPLAYOPERATORMINHEIGHT: i32 = 3;
const LIG_TAG: u16 = 1;
const MATH_TEXT_CHAR: i32 = 4;
const NOAD_SIZE: i32 = 4;
const INT_PAR_DELIMITER_FACTOR: i32 = 18;
const DIMEN_PAR_DELIMITER_SHORTFALL: i32 = 10;
const ACCENTBASEHEIGHT: i32 = 6;
const X_HEIGHT_CODE: i32 = 5;
const RADICAL_NOAD_SIZE: i32 = 5;
const FRACTION_NOAD_SIZE: i32 = 6;
const STYLE_NODE_SIZE: i32 = 3;

const MATH_SHIFT: u8 = 3;
const MMODE: i16 = 207;
const GLUE_PAR_RIGHT_SKIP: i32 = 8;
const GLUE_PAR_LEFT_SKIP: i32 = 7;
const DIMEN_PAR_PRE_DISPLAY_SIZE: i32 = 13;
const DIMEN_PAR_DISPLAY_WIDTH: i32 = 14;
const DIMEN_PAR_DISPLAY_INDENT: i32 = 15;
const DIMEN_PAR_HANG_INDENT: i32 = 17;
const DIMEN_PAR_HSIZE: i32 = 3;
const INT_PAR_HANG_AFTER: i32 = 41;
const INT_PAR_PRE_DISPLAY_DIRECTION: i32 = 61;
const INT_PAR_TEXXET: i32 = 69;
const INT_PAR_BIN_OP_PENALTY: i32 = 9;
const INT_PAR_REL_PENALTY: i32 = 10;
const QUAD_CODE: i32 = 6;
const L_CODE: i32 = 4;
const R_CODE: i32 = 8;
const BEGIN_L_CODE: i16 = 6;
const END_L_CODE: i16 = 7;
const TEMP_HEAD: i32 = 4_999_996;
const EVERY_DISPLAY_TEXT: u16 = 10;
const LOCAL_EVERY_DISPLAY: i32 = 4;
const LOCAL_PAR_SHAPE: i32 = 0;
const STRETCHING: u16 = 1;
const SHRINKING: u16 = 2;
const A_LEADERS: u16 = 100;
const RIGHT_TO_LEFT: i16 = 1;
const LEFT_TO_RIGHT: i16 = 0;
const BEFORE: i32 = 0;

const FONT_BASE: i32 = 0;
const TEXT_SIZE: i32 = 0;
const SCRIPT_SIZE: i32 = 256;
const SCRIPT_SCRIPT_SIZE: i32 = 512;
const TOTAL_MATHSY_PARAMS: i32 = 22;
const TOTAL_MATHEX_PARAMS: i32 = 13;
const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 0xFFFE;
const TEX_INFINITY: i32 = 0x7FFF_FFFF;
const STACKGAPMIN: i32 = 26;
const STACKDISPLAYSTYLEGAPMIN: i32 = 27;
const FRACTIONNUMERATORGAPMIN: i32 = 36;
const FRACTIONNUMDISPLAYSTYLEGAPMIN: i32 = 37;
const FRACTIONDENOMINATORGAPMIN: i32 = 39;
const FRACTIONDENOMDISPLAYSTYLEGAPMIN: i32 = 40;
const RADICALVERTICALGAP: i32 = 49;
const RADICALDISPLAYSTYLEVERTICALGAP: i32 = 50;
const RADICALRULETHICKNESS: i32 = 51;
const SUB_CMD: i32 = 1;
const SUP_CMD: i32 = 0;
const SUBSCRIPTTOPMAX: i32 = 9;
const SUPERSCRIPTBOTTOMMIN: i32 = 13;
const SUBSUPERSCRIPTGAPMIN: i32 = 15;
const SUPERSCRIPTBOTTOMMAXWITHSUBSCRIPT: i32 = 16;
const BEGIN_M_CODE: i16 = 2;
const END_M_CODE: i16 = 3;
const DISPLAY_STYLE: i16 = 0;
const TEXT_STYLE: i16 = 2;
const SCRIPT_STYLE: i16 = 4;
const ADJUST_HEAD: i32 = 4_999_994;
const PRE_ADJUST_HEAD: i32 = 4_999_985;
const FIL: usize = 1;
const FILL: usize = 2;
const FILLL: usize = 3;
const INF_PENALTY: i32 = 10000;
const DIMEN_PAR_MATH_SURROUND: i32 = 1;
const DIMEN_PAR_SCRIPT_SPACE: i32 = 12;
const INT_PAR_PRE_DISPLAY_PENALTY: i32 = 11;
const INT_PAR_POST_DISPLAY_PENALTY: i32 = 12;
const GLUE_PAR_ABOVE_DISPLAY_SKIP: i16 = 3;
const GLUE_PAR_BELOW_DISPLAY_SKIP: i16 = 4;
const GLUE_PAR_ABOVE_DISPLAY_SHORT_SKIP: i16 = 5;
const GLUE_PAR_BELOW_DISPLAY_SHORT_SKIP: i16 = 6;
const GLUE_PAR_THIN_MU_SKIP: i32 = 16;
const GLUE_PAR_MED_MU_SKIP: i32 = 17;
const GLUE_PAR_THICK_MU_SKIP: i32 = 18;
const ADDITIONAL: i16 = 1;
const EXACTLY: i16 = 0;
const AFTER: i16 = 1;
const DLIST: u16 = 2;
const MU_GLUE: u16 = 99;
const EXPLICIT: u16 = 1;
const VLIST_NODE: u16 = 1;
const SUB_BOX: i32 = 2;
const COND_MATH_GLUE: u16 = 98;
const SPACE_CODE: i32 = 2;

extern "C" {

    fn isOpenTypeMathFont(engine: *mut libc::c_void) -> bool;
    fn usingOpenType(engine: *mut libc::c_void) -> bool;
    fn get_native_mathsy_param(f: i32, n: i32) -> i32;
    fn get_native_mathex_param(f: i32, n: i32) -> i32;

    fn get_ot_math_accent_pos(f: i32, g: i32) -> i32;
    fn get_ot_math_constant(f: i32, n: i32) -> i32;
    fn get_ot_math_kern(
        base_f: i32,
        base_g: u16,
        script_f: i32,
        script_g: u16,
        cmd: i32,
        shift: i32,
    ) -> i32;
    fn get_ot_math_variant(f: i32, g: i32, v: i32, adv: *mut i32, horiz: i32) -> i32;
    fn get_ot_assembly_ptr(f: i32, g: i32, horiz: i32) -> *mut libc::c_void;
    fn get_ot_math_ital_corr(f: i32, g: i32) -> i32;
    fn free_ot_assembly(a: *mut libc::c_void);
    fn ot_min_connector_overlap(f: i32) -> i32;
    fn ot_part_count(a: *mut libc::c_void) -> i32;
    fn ot_part_is_extender(a: *mut libc::c_void, i: i32) -> bool;
    fn ot_part_glyph(a: *mut libc::c_void, i: i32) -> i32;
    fn ot_part_start_connector(f: i32, a: *mut libc::c_void, i: i32) -> i32;
    fn ot_part_end_connector(f: i32, a: *mut libc::c_void, i: i32) -> i32;
    fn ot_part_full_advance(f: i32, a: *mut libc::c_void, i: i32) -> i32;
}

#[inline]
fn math_font(eqtb: &EqtbState, n: i32) -> i32 {
    eqtb.b32s1(MATH_FONT_BASE + n)
}

#[inline]
fn font_is_ot_math(fonts: &FontState, f: i32) -> bool {
    fonts.font_area_entry(f) == OTGR_FONT_FLAG
        && unsafe { isOpenTypeMathFont(fonts.font_layout_engine_ptr(f)) }
}

#[inline]
fn math_sy_param(fonts: &FontState, eqtb: &EqtbState, size_code: i32, param: i32) -> i32 {
    let f = math_font(eqtb, 2 + size_code);
    if font_is_ot_math(fonts, f) {
        unsafe { get_native_mathsy_param(f, param) }
    } else {
        fonts.font_info_b32s1(param + fonts.param_base(f))
    }
}

#[inline]
fn math_ext_param(fonts: &FontState, eqtb: &EqtbState, cur_size: i32, param: i32) -> i32 {
    let f = math_font(eqtb, 3 + cur_size);
    if font_is_ot_math(fonts, f) {
        unsafe { get_native_mathex_param(f, param) }
    } else {
        fonts.font_info_b32s1(param + fonts.param_base(f))
    }
}

/// translated from xetex-math.c:86 (math_x_height)
pub fn math_x_height(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 5)
}

/// translated from xetex-math.c:101 (math_quad)
pub fn math_quad(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 6)
}

/// translated from xetex-math.c:115 (num1)
pub fn num1(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 8)
}

/// translated from xetex-math.c:130 (num2)
pub fn num2(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 9)
}

/// translated from xetex-math.c:145 (num3)
pub fn num3(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 10)
}

/// translated from xetex-math.c:160 (denom1)
pub fn denom1(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 11)
}

/// translated from xetex-math.c:175 (denom2)
pub fn denom2(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 12)
}

/// translated from xetex-math.c:190 (sup1)
pub fn sup1(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 13)
}

/// translated from xetex-math.c:205 (sup2)
pub fn sup2(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 14)
}

/// translated from xetex-math.c:219 (sup3)
pub fn sup3(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 15)
}

/// translated from xetex-math.c:233 (sub1)
pub fn sub1(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 16)
}

/// translated from xetex-math.c:247 (sub2)
pub fn sub2(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 17)
}

/// translated from xetex-math.c:261 (sup_drop)
pub fn sup_drop(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 18)
}

/// translated from xetex-math.c:275 (sub_drop)
pub fn sub_drop(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 19)
}

/// translated from xetex-math.c:289 (delim1)
pub fn delim1(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 20)
}

/// translated from xetex-math.c:303 (delim2)
pub fn delim2(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 21)
}

/// translated from xetex-math.c:317 (axis_height)
pub fn axis_height(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 22)
}

/// translated from xetex-math.c:88 (default_rule_thickness)
pub fn default_rule_thickness(fonts: &FontState, eqtb: &EqtbState, cur_size: i32) -> i32 {
    math_ext_param(fonts, eqtb, cur_size, 8)
}

/// translated from xetex-math.c:102 (big_op_spacing1)
pub fn big_op_spacing1(fonts: &FontState, eqtb: &EqtbState, cur_size: i32) -> i32 {
    math_ext_param(fonts, eqtb, cur_size, 9)
}

/// translated from xetex-math.c:116 (big_op_spacing2)
pub fn big_op_spacing2(fonts: &FontState, eqtb: &EqtbState, cur_size: i32) -> i32 {
    math_ext_param(fonts, eqtb, cur_size, 10)
}

/// translated from xetex-math.c:130 (big_op_spacing3)
pub fn big_op_spacing3(fonts: &FontState, eqtb: &EqtbState, cur_size: i32) -> i32 {
    math_ext_param(fonts, eqtb, cur_size, 11)
}

/// translated from xetex-math.c:144 (big_op_spacing4)
pub fn big_op_spacing4(fonts: &FontState, eqtb: &EqtbState, cur_size: i32) -> i32 {
    math_ext_param(fonts, eqtb, cur_size, 12)
}

/// translated from xetex-math.c:158 (big_op_spacing5)
pub fn big_op_spacing5(fonts: &FontState, eqtb: &EqtbState, cur_size: i32) -> i32 {
    math_ext_param(fonts, eqtb, cur_size, 13)
}

/// translated from xetex-math.c:170 (fraction_rule)
///
/// Phase-C: threaded to `&mut MemState`; bottoms out on the threaded
/// [`NodePtr::<RuleNode>::alloc`] and propagates its allocation sentinel as
/// [`NodeRecycleAbort::Overflow`] (`01b` §5).
pub fn fraction_rule(mem: &mut MemState, t: i32) -> Result<NodePtr<RuleNode>, NodeRecycleAbort> {
    let Some(p) = NodePtr::<RuleNode>::alloc(mem) else {
        return Err(NodeRecycleAbort::Overflow);
    };
    p.set_subtype(mem, 0);
    p.set_width(mem, RuleDimen::Running);
    p.set_height(mem, RuleDimen::Set(Scaled(t)));
    p.set_depth(mem, RuleDimen::Set(Scaled(0)));
    Ok(p)
}

/// translated from xetex-math.c:181 (overbar)
///
/// Phase-C: threaded to `&mut MemState`. Builds the kern/rule/kern chain
/// directly through the typed [`alloc_kern_node`]/[`fraction_rule`] handles and
/// returns its head; with_state-bearing `vpackage` callers run after the borrow
/// drops (bucket C). Allocation failure propagates the
/// [`NodeRecycleAbort`] sentinel (`01b` §5).
pub fn overbar(mem: &mut MemState, b: AnyBox, k: i32, t: i32) -> Result<i32, NodeRecycleAbort> {
    let p = alloc_kern_node(mem, k)?;
    p.set_link(mem, MemPtr::from_idx(b.idx()));
    let q = fraction_rule(mem, t)?;
    q.set_link(mem, MemPtr::from_idx(p.idx()));
    let r = alloc_kern_node(mem, t)?;
    r.set_link(mem, MemPtr::from_idx(q.idx()));
    Ok(r.idx().0)
}

/// translated from xetex-math.c:195 (math_glue)
struct MathGlueInput {
    width: i32,
    stretch_order: u16,
    stretch: i32,
    shrink_order: u16,
    shrink: i32,
}

/// `&mut ErrorState` core of [`scaled_mu_value`] (callable under a held borrow):
/// the `x_over_n`/`mult_and_add`/`xn_over_d` arithmetic touches
/// `tex_remainder`/`arith_error` directly instead of via `with_state`.
fn scaled_mu_value_in(errors: &mut crate::state::ErrorState, v: i32, m: i32) -> i32 {
    let mut n = crate::scaled_math::x_over_n_in(errors, m, 65_536);
    let mut f = errors.tex_remainder;
    if f < 0 {
        n -= 1;
        f += 65_536;
    }
    let xnd = crate::scaled_math::xn_over_d_in(errors, v, f, 65_536);
    crate::scaled_math::mult_and_add_in(errors, n, v, xnd, MAX_HALFWORD)
}

/// Threaded core of [`math_glue_global`]: builds the scaled glue spec for math
/// glue `g` at `mu` `m` under the caller's `&mut EngineState` borrow.
fn math_glue_in(st: &mut EngineState, g: i32, m: i32) -> Result<i32, NodeRecycleAbort> {
    let spec = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(g))
        .expect("non-null math glue spec");
    let stretch_order = spec.stretch_order(&st.mem);
    let shrink_order = spec.shrink_order(&st.mem);
    let width_val = spec.width(&st.mem).0;
    let stretch_val = spec.stretch(&st.mem).0;
    let shrink_val = spec.shrink(&st.mem).0;
    let width = scaled_mu_value_in(&mut st.errors, width_val, m);
    let stretch = if stretch_order == NORMAL {
        scaled_mu_value_in(&mut st.errors, stretch_val, m)
    } else {
        stretch_val
    };
    let shrink = if shrink_order == NORMAL {
        scaled_mu_value_in(&mut st.errors, shrink_val, m)
    } else {
        shrink_val
    };
    math_glue(
        &mut st.mem,
        MathGlueInput {
            width,
            stretch_order,
            stretch,
            shrink_order,
            shrink,
        },
    )
}

/// Threaded core of [`math_kern_global`]: scales an explicit math kern under the
/// caller's `&mut EngineState` borrow.
fn math_kern_in(st: &mut EngineState, p: NodePtr<KernNode>, m: i32) {
    let is_mu_glue = p.subtype(&st.mem) == MU_GLUE;
    let width_val = p.width(&st.mem).0;
    let scaled_width = if is_mu_glue {
        Some(Scaled(scaled_mu_value_in(&mut st.errors, width_val, m)))
    } else {
        None
    };
    math_kern(&mut st.mem, p, scaled_width);
}

/// Borrow-safe equivalent of [`crate::globals::mem_node_addr`]: computes the raw
/// node address from the held `mem` arena (the global re-enters `with_state`).
#[inline]
fn node_addr_in(st: &EngineState, p: i32) -> *mut libc::c_void {
    unsafe { st.mem.data.as_ptr().add(p as usize) as *mut libc::c_void }
}

fn math_glue(mem: &mut MemState, input: MathGlueInput) -> Result<i32, NodeRecycleAbort> {
    // Typesafe allocation: `alloc` confines the `from_idx_unchecked` to one
    // audited site and yields the typed handle directly (`01b` §3a). `None`
    // is the arena-exhaustion sentinel, mapped to the §5 abort here.
    let spec = NodePtr::<GlueSpecNode>::alloc(mem).ok_or(NodeRecycleAbort::Overflow)?;
    spec.set_width(mem, Scaled(input.width));
    spec.set_stretch_order(mem, input.stretch_order);
    spec.set_stretch(mem, Scaled(input.stretch));
    spec.set_shrink_order(mem, input.shrink_order);
    spec.set_shrink(mem, Scaled(input.shrink));
    Ok(spec.idx().0)
}

/// translated from xetex-math.c:223 (math_kern)
fn math_kern(mem: &mut MemState, p: NodePtr<KernNode>, scaled_width: Option<Scaled>) {
    if let Some(width) = scaled_width {
        p.set_width(mem, width);
        p.set_subtype(mem, EXPLICIT);
    }
}

/// translated from xetex-math.c:80 (flush_math)
///
/// Phase C (`01b` §4a bucket B/C): touches the `mem` arena directly through
/// `&mut MemState` and recycles the two current-list chains via the threaded
/// [`crate::packaging::flush_node_list`]. The `cur_list` reads/writes go through the
/// raw `cur_list` global (the deferred Phase-C set — safe under a live borrow).
/// The two impossible corrupt-node `confusion` arms propagate out as `Err`.
pub fn flush_math(cl: &mut crate::mem_types::ListStateRecord, mem: &mut MemState) -> Result<(), &'static std::ffi::CStr> {
    let head = cl.head;
    let p1 = mem.b32s1(head);
    crate::packaging::flush_node_list(mem, p1)?;
    let p2 = cl.aux_s1();
    crate::packaging::flush_node_list(mem, p2)?;
    mem.set_b32s1(head, TEX_NULL);
    cl.tail = head;
    cl.set_aux_s1(TEX_NULL);
    Ok(())
}

/// translated from xetex-math.c:91 (clean_box). Threaded SCC core: builds the
/// cleaned hbox for math field `p` at style `s` under the caller's single
/// `&mut EngineState` borrow, recursing into [`mlist_to_hlist_in`].
unsafe fn clean_box_in(st: &mut EngineState, p: i32, s: i16) -> Result<AnyBox, MathBuildAbort> {
    let q;
    match st.mem.b32s1(p) {
        MATH_CHAR => {
            let mlist = crate::mem::new_noad(&mut st.mem);
            if mlist == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            st.scanner.cur_mlist = mlist;
            let w = st.mem.word(p);
            st.mem.set_word(mlist + 1, w);

            let save_style = st.scanner.cur_style;
            st.scanner.cur_style = s;
            st.scanner.mlist_penalties = false;
            mlist_to_hlist_in(st)?;
            q = st.mem.b32s1(TEMP_HEAD);
            st.scanner.cur_style = save_style;
            clean_box_reset_mu(st);
        }
        SUB_BOX => {
            q = st.mem.b32s0(p);
        }
        SUB_MLIST => {
            st.scanner.cur_mlist = st.mem.b32s0(p);

            let save_style = st.scanner.cur_style;
            st.scanner.cur_style = s;
            st.scanner.mlist_penalties = false;
            mlist_to_hlist_in(st)?;
            q = st.mem.b32s1(TEMP_HEAD);
            st.scanner.cur_style = save_style;
            clean_box_reset_mu(st);
        }
        _ => {
            q = crate::mem::new_null_box(&mut st.mem);
            if q == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
        }
    }

    let x_box = if crate::mem::is_char_node(&st.mem, q) || q == TEX_NULL {
        crate::packaging::hpack_emit_in(st, q, 0, ADDITIONAL)?
    } else if st.mem.b32s1(q) == TEX_NULL
        && st.mem.b16s1(q) <= VLIST_NODE
        && st.mem.b32s1(q + 4) == 0
    {
        classify_box_or_panic(&st.mem, q)
    } else {
        crate::packaging::hpack_emit_in(st, q, 0, ADDITIONAL)?
    };
    let qq = x_box.list_ptr(&st.mem).0;
    if crate::mem::is_char_node(&st.mem, qq) {
        let r = st.mem.b32s1(qq);
        if r != TEX_NULL
            && st.mem.b32s1(r) == TEX_NULL
            && !crate::mem::is_char_node(&st.mem, r)
            && st.mem.b16s1(r) == KERN_NODE
        {
            crate::mem::free_node(&mut st.mem, r, MEDIUM_NODE_SIZE);
            st.mem.set_b32s1(qq, TEX_NULL);
        }
    }
    Ok(x_box)
}

/// The shared `cur_size`/`cur_mu` reset after a `clean_box` sub-list conversion.
#[inline]
fn clean_box_reset_mu(st: &mut EngineState) {
    if st.scanner.cur_style < SCRIPT_STYLE {
        st.scanner.cur_size = TEXT_SIZE;
    } else {
        st.scanner.cur_size = SCRIPT_SIZE * ((st.scanner.cur_style as i32 - 2) / 2);
    }
    let mq = math_quad(&st.fonts, &st.eqtb, st.scanner.cur_size);
    st.scanner.cur_mu = crate::scaled_math::x_over_n_in(&mut st.errors, mq, 18);
}

/// translated from xetex-math.c:115 (fetch)
/// Deferred-output signal from the threaded [`fetch_core`] (`01b` §4a bucket C):
/// the cold diagnostic / warning paths cannot run under a live state borrow, so
/// the core sets the `cur_*` fields directly and returns what `fetch_global`
/// must emit after the borrow drops.
enum FetchOutcome {
    /// Normal: nothing to emit.
    Ok,
    /// The math font family is undefined — emit the diagnostic + recoverable
    /// `error()`. Carries the values the message prints (`cur_*` already set).
    UndefinedFont { size: i32, fam: i32, c: i32 },
    /// The requested character is missing from the font — emit
    /// `char_warning(f, c)`.
    CharMissing { f: i32, c: i32 },
}

/// Threaded core (Phase C): set `cur_c` / `cur_f` / `cur_i` for math char `a`
/// directly from `mem` / `fonts` / `eqtb` / `scanner`. The with_state-bearing
/// diagnostic and `char_warning` paths are deferred to `fetch_global` via
/// [`FetchOutcome`].
fn fetch_core(
    mem: &mut MemState,
    fonts: &FontState,
    eqtb: &EqtbState,
    scanner: &mut ScannerState,
    a: i32,
) -> FetchOutcome {
    let size = scanner.cur_size;
    let fam = mem.b16s1(a) as i32 % 256;
    scanner.cur_c = mem.b16s0(a) as i32;
    scanner.cur_f = math_font(eqtb, fam + size);
    scanner.cur_c += (mem.b16s1(a) as i32 / 256) * 65_536;

    if scanner.cur_f == FONT_BASE {
        let c = scanner.cur_c;
        scanner.cur_i = fonts.null_character;
        mem.set_b32s1(a, EMPTY);
        return FetchOutcome::UndefinedFont { size, fam, c };
    }

    let area = fonts.font_area_entry(scanner.cur_f);
    if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
        scanner.cur_i = fonts.null_character;
    } else {
        if scanner.cur_c >= fonts.font_bc_entry(scanner.cur_f) as i32
            && scanner.cur_c <= fonts.font_ec_entry(scanner.cur_f) as i32
        {
            scanner.cur_i =
                fonts.font_info_b16(fonts.char_base_entry(scanner.cur_f) + scanner.cur_c);
        } else {
            scanner.cur_i = fonts.null_character;
        }
        if scanner.cur_i.s3 == 0 {
            let f = scanner.cur_f;
            let c = scanner.cur_c;
            mem.set_b32s1(a, EMPTY);
            scanner.cur_i = fonts.null_character;
            return FetchOutcome::CharMissing { f, c };
        }
    }
    FetchOutcome::Ok
}

/// Threaded form of [`fetch_global`]: runs [`fetch_core`] and emits its deferred
/// diagnostic / warning under the held `&mut EngineState` borrow (the print/error
/// `_in` cores from sessions 182–185), returning the recoverable
/// [`ErrorOutcome`](crate::errors::ErrorOutcome) so the threaded caller can defer
/// a non-`Continue` abort to its wrapper (`01b` §5).
fn fetch_in(st: &mut EngineState, a: i32) -> crate::errors::ErrorOutcome {
    use crate::errors::ErrorOutcome;
    match fetch_core(&mut st.mem, &st.fonts, &st.eqtb, &mut st.scanner, a) {
        FetchOutcome::Ok => ErrorOutcome::Continue,
        FetchOutcome::UndefinedFont { size, fam, c } => {
            crate::output::error_here_with_diagnostic_in(st, c"".as_ptr());
            crate::output::print_size_in(st, size);
            crate::output::print_char_in(st, ' ' as i32);
            crate::output::print_int_in(st, fam);
            crate::output::print_cstr_in(st, c" is undefined (character ".as_ptr());
            crate::output::print_in(st, c);
            crate::output::print_char_in(st, ')' as i32);
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 4;
            st.errors.help_line[3] =
                c"Somewhere in the math formula just ended, you used the".as_ptr();
            st.errors.help_line[2] =
                c"stated character from an undefined font family. For example,".as_ptr();
            st.errors.help_line[1] =
                c"plain TeX doesn't allow \\it or \\sl in subscripts. Proceed,".as_ptr();
            st.errors.help_line[0] =
                c"and I'll try to forget that I needed that character.".as_ptr();
            crate::errors::error_in(st)
        }
        FetchOutcome::CharMissing { f, c } => crate::output::char_warning_in(st, f, c),
    }
}

/// translated from xetex-math.c:198 (make_over)
fn make_over(mem: &mut MemState, q: NodePtr<Noad>, x: AnyBox) {
    q.set_nucleus_field(mem, x.idx().0);
    q.set_nucleus_type(mem, SUB_BOX);
}

unsafe fn make_over_in(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), MathBuildAbort> {
    let q_raw = q.idx().0;
    let x = clean_box_in(st, q_raw + 1, 2 * (st.scanner.cur_style / 2) + 1)?;
    let rule_thickness = default_rule_thickness(&st.fonts, &st.eqtb, st.scanner.cur_size);
    let p = overbar(&mut st.mem, x, 3 * rule_thickness, rule_thickness)?;
    let x = crate::packaging::vpackage_emit_in(st, p, 0, ADDITIONAL, MAX_HALFWORD)?;
    make_over(&mut st.mem, q, x);
    Ok(())
}

/// translated from xetex-math.c:207 (make_under)
fn make_under_prepare(
    mem: &mut MemState,
    x: AnyBox,
    rule_thickness: i32,
) -> Result<(), NodeRecycleAbort> {
    let p = alloc_kern_node(mem, 3 * rule_thickness)?;
    x.set_link(mem, MemPtr::from_idx(p.idx()));
    let rule = fraction_rule(mem, rule_thickness)?;
    p.set_link(mem, MemPtr::from_idx(rule.idx()));
    Ok(())
}

fn make_under_finish(
    mem: &mut MemState,
    q: NodePtr<Noad>,
    xb: AnyBox,
    yb: AnyBox,
    rule_thickness: i32,
) {
    let delta = yb.height(mem).0 + yb.depth(mem).0 + rule_thickness;
    yb.set_height(mem, xb.height(mem));
    yb.set_depth(mem, Scaled(delta - yb.height(mem).0));
    q.set_nucleus_field(mem, yb.idx().0);
    q.set_nucleus_type(mem, SUB_BOX);
}

unsafe fn make_under_in(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), MathBuildAbort> {
    let q_raw = q.idx().0;
    let x = clean_box_in(st, q_raw + 1, st.scanner.cur_style)?;
    let rule_thickness = default_rule_thickness(&st.fonts, &st.eqtb, st.scanner.cur_size);
    make_under_prepare(&mut st.mem, x, rule_thickness)?;
    let y = crate::packaging::vpackage_emit_in(st, x.idx().0, 0, ADDITIONAL, MAX_HALFWORD)?;
    make_under_finish(&mut st.mem, q, x, y, rule_thickness);
    Ok(())
}

/// translated from xetex-math.c:225 (make_vcenter)
fn make_vcenter(
    mem: &mut MemState,
    q: NodePtr<Noad>,
    axis_height: i32,
) -> Result<(), NodeRecycleAbort> {
    let v = q.nucleus_field(mem);
    if mem.b16s1(v) != VLIST_NODE {
        return Err(NodeRecycleAbort::Confusion(c"vcenter"));
    }
    let vb = mem.classify_box(MemIdx::new(v)).unwrap().unwrap();
    let delta = vb.height(mem).0 + vb.depth(mem).0;
    vb.set_height(mem, Scaled(axis_height + crate::scaled_math::half(delta)));
    vb.set_depth(mem, Scaled(delta - vb.height(mem).0));
    Ok(())
}

// The session-193 workaround that kept `axis_height` outside the borrow is no
// longer needed: the OT-math FFI's `tt_get_font_*` callbacks are borrow-free
// (session 207 mirror), so `axis_height` is safe under the held borrow.
unsafe fn make_vcenter_in(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), MathBuildAbort> {
    let axis_height = axis_height(&st.fonts, &st.eqtb, st.scanner.cur_size);
    make_vcenter(&mut st.mem, q, axis_height)?;
    Ok(())
}

fn make_radical_finish(mem: &mut MemState, q: NodePtr<Noad>, x: AnyBox) {
    q.set_nucleus_field(mem, x.idx().0);
    q.set_nucleus_type(mem, SUB_BOX);
}

fn make_radical_prepare_delimiter(
    mem: &mut MemState,
    y_box: AnyBox,
    x_box: AnyBox,
    clr: i32,
    rule_thickness: i32,
    is_ot_math: bool,
) -> (i32, i32, i32) {
    if is_ot_math {
        let new_depth = Scaled(y_box.depth(mem).0 + y_box.height(mem).0 - rule_thickness);
        y_box.set_depth(mem, new_depth);
        y_box.set_height(mem, Scaled(rule_thickness));
    }

    let delta = Scaled(y_box.depth(mem).0 - (x_box.height(mem).0 + x_box.depth(mem).0 + clr));
    let x_height = x_box.height(mem).0;
    let rule_height = y_box.height(mem).0;
    (delta.0, x_height, rule_height)
}

/// translated from xetex-math.c:80 (make_radical). Threaded SCC core: one borrow
/// across the OT/TFM radical build (the OT readers are borrow-safe, session 207).
unsafe fn make_radical_in(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), MathBuildAbort> {
    let q_raw = q.idx().0;
    let f = math_font(
        &st.eqtb,
        (st.mem.b16s3(q_raw + 4) as i32 % 256) + st.scanner.cur_size,
    );
    let is_ot_math = font_is_ot_math(&st.fonts, f);
    let rule_thickness = if is_ot_math {
        get_ot_math_constant(f, RADICALRULETHICKNESS)
    } else {
        default_rule_thickness(&st.fonts, &st.eqtb, st.scanner.cur_size)
    };

    let clean_style = 2 * (st.scanner.cur_style / 2) + 1;
    let x = clean_box_in(st, q_raw + 1, clean_style)?;
    let cur_style = st.scanner.cur_style;
    let cur_size = st.scanner.cur_size;
    let mut clr = if is_ot_math {
        if cur_style < TEXT_STYLE {
            get_ot_math_constant(f, RADICALDISPLAYSTYLEVERTICALGAP)
        } else {
            get_ot_math_constant(f, RADICALVERTICALGAP)
        }
    } else if cur_style < TEXT_STYLE {
        rule_thickness + math_x_height(&st.fonts, &st.eqtb, cur_size).abs() / 4
    } else {
        rule_thickness + rule_thickness.abs() / 4
    };

    let delimiter_height = x.height(&st.mem).0 + x.depth(&st.mem).0 + clr + rule_thickness;
    let y = var_delimiter_in(st, q_raw + 4, cur_size, delimiter_height)?;
    let (delta, x_height, rule_height) = make_radical_prepare_delimiter(
        &mut st.mem,
        y,
        x,
        clr,
        rule_thickness,
        is_ot_math,
    );
    if delta > 0 {
        clr += crate::scaled_math::half(delta);
    }
    y.set_shift_amount(&mut st.mem, Scaled(-(x_height + clr)));
    let p = overbar(&mut st.mem, x, clr, rule_height)?;
    let overbar_box = crate::packaging::vpackage_emit_in(st, p, 0, ADDITIONAL, MAX_HALFWORD)?;
    y.set_link(&mut st.mem, MemPtr::from_idx(overbar_box.idx()));
    let hbox = crate::packaging::hpack_emit_in(st, y.idx().0, 0, ADDITIONAL)?;
    make_radical_finish(&mut st.mem, q, hbox);
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

fn make_fraction(
    mem: &mut MemState,
    assembly: FractionAssembly,
) -> Result<AnyBox, NodeRecycleAbort> {
    let vb = AnyBox::VList(alloc_null_vbox(mem)?);
    let xb = assembly.x;
    let zb = assembly.z;
    vb.set_height(mem, Scaled(assembly.shift_up + xb.height(mem).0));
    vb.set_depth(mem, Scaled(zb.depth(mem).0 + assembly.shift_down));
    vb.set_width(mem, xb.width(mem));

    let p: NodePtr<KernNode>;
    if assembly.rule_thickness == 0 {
        let kw = (assembly.shift_up - xb.depth(mem).0) - (zb.height(mem).0 - assembly.shift_down);
        p = alloc_kern_node(mem, kw)?;
        p.set_link(mem, MemPtr::from_idx(assembly.z.idx()));
    } else {
        let y = fraction_rule(mem, assembly.rule_thickness)?;
        let p1w =
            (assembly.axis_height - assembly.delta) - (zb.height(mem).0 - assembly.shift_down);
        let p1 = alloc_kern_node(mem, p1w)?;
        y.set_link(mem, MemPtr::from_idx(p1.idx()));
        p1.set_link(mem, MemPtr::from_idx(assembly.z.idx()));
        let pw = (assembly.shift_up - xb.depth(mem).0) - (assembly.axis_height + assembly.delta);
        p = alloc_kern_node(mem, pw)?;
        p.set_link(mem, MemPtr::from_idx(y.idx()));
    }

    xb.set_link(mem, MemPtr::from_idx(p.idx()));
    vb.set_list_ptr(mem, MemPtr::from_idx(assembly.x.idx()));
    Ok(vb)
}

fn make_fraction_link_delimiters(
    mem: &mut MemState,
    left_delim: AnyBox,
    v: AnyBox,
    right_delim: AnyBox,
) {
    left_delim.set_link(mem, MemPtr::from_idx(v.idx()));
    v.set_link(mem, MemPtr::from_idx(right_delim.idx()));
}

fn make_fraction_finish(mem: &mut MemState, q: NodePtr<Noad>, hbox: AnyBox) {
    q.set_nucleus_type(mem, hbox.idx().0);
}

#[inline]
fn classify_box_or_panic(mem: &MemState, p: i32) -> AnyBox {
    mem.classify_box(MemIdx::new(p))
        .expect("valid node type")
        .expect("box node")
}

#[inline]
fn assume_noad(p: i32) -> NodePtr<Noad> {
    unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(p)) }
}

fn make_op_latch_limits(mem: &mut MemState, scanner: &ScannerState, q: NodePtr<Noad>) {
    if q.subtype(mem) == NORMAL && scanner.cur_style < TEXT_STYLE {
        q.set_subtype(mem, LIMITS);
    }
}

fn make_op_finish_math_char(
    mem: &mut MemState,
    q: NodePtr<Noad>,
    x: AnyBox,
    delta: i32,
    axis_height: i32,
) {
    if q.subscr_type(mem) != EMPTY && q.subtype(mem) != LIMITS {
        x.set_width(mem, Scaled(x.width(mem).0 - delta));
    }
    let shift = crate::scaled_math::half(x.height(mem).0 - x.depth(mem).0) - axis_height;
    x.set_shift_amount(mem, Scaled(shift));
    q.set_nucleus_type(mem, SUB_BOX);
    q.set_nucleus_field(mem, x.idx().0);
}

fn make_op_prepare_limits_box(
    mem: &mut MemState,
    v: AnyBox,
    x: AnyBox,
    y: AnyBox,
    z: AnyBox,
) -> i32 {
    let mut w = y.width(mem).0;
    if x.width(mem).0 > w {
        w = x.width(mem).0;
    }
    if z.width(mem).0 > w {
        w = z.width(mem).0;
    }
    v.set_width(mem, Scaled(w));
    w
}

struct BigOpSpacings {
    bos1: i32,
    bos2: i32,
    bos3: i32,
    bos4: i32,
    bos5: i32,
}

fn alloc_kern_node(mem: &mut MemState, width: i32) -> Result<NodePtr<KernNode>, NodeRecycleAbort> {
    let Some(p) = NodePtr::<KernNode>::alloc(mem) else {
        return Err(NodeRecycleAbort::Overflow);
    };
    p.set_subtype(mem, NORMAL);
    p.set_width(mem, Scaled(width));
    Ok(p)
}

/// Typed HLIST twin of [`new_null_box`]: confines the family's single
/// `from_idx_unchecked` to one audited allocator site (`01c` §8) so box-building
/// callers receive the handle directly instead of re-classifying the `i32`
/// index. `Err` is the arena-exhaustion sentinel (`01b` §5).
fn alloc_null_box(mem: &mut MemState) -> Result<NodePtr<HListNode>, NodeRecycleAbort> {
    let b = new_null_box(mem);
    if b == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    // SAFETY: `new_null_box` type-stamps HLIST_NODE and zeroes the box fields.
    Ok(unsafe { NodePtr::<HListNode>::from_idx_unchecked(MemIdx::new(b)) })
}

/// Typed VLIST twin of [`new_null_box`]: allocates the null box, re-stamps it
/// `VLIST_NODE`, and returns the typed handle (same single-audited-site
/// confinement as [`alloc_null_box`]).
fn alloc_null_vbox(mem: &mut MemState) -> Result<NodePtr<VListNode>, NodeRecycleAbort> {
    let b = new_null_box(mem);
    if b == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b16s1(b, VLIST_NODE);
    // SAFETY: `b` was just stamped `VLIST_NODE`.
    Ok(unsafe { NodePtr::<VListNode>::from_idx_unchecked(MemIdx::new(b)) })
}

#[allow(clippy::too_many_arguments)]
fn make_op_finish_limits(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    q: NodePtr<Noad>,
    x: AnyBox,
    y: AnyBox,
    z: AnyBox,
    v: AnyBox,
    delta: i32,
    save_f: i32,
    spacings: BigOpSpacings,
) -> Result<(), NodeRecycleAbort> {
    let half_delta = crate::scaled_math::half(delta);
    x.set_shift_amount(mem, Scaled(half_delta));
    z.set_shift_amount(mem, Scaled(-half_delta));
    v.set_height(mem, y.height(mem));
    v.set_depth(mem, y.depth(mem));
    scanner.cur_f = save_f;

    if q.supscr_type(mem) == EMPTY {
        free_node(mem, x.idx().0, BOX_NODE_SIZE);
        v.set_list_ptr(mem, MemPtr::from_idx(y.idx()));
    } else {
        let mut shift_up = spacings.bos3 - x.depth(mem).0;
        if shift_up < spacings.bos1 {
            shift_up = spacings.bos1;
        }
        let p = alloc_kern_node(mem, shift_up)?;
        p.set_link(mem, MemPtr::from_idx(y.idx()));
        x.set_link(mem, MemPtr::from_idx(p.idx()));
        let p2 = alloc_kern_node(mem, spacings.bos5)?;
        p2.set_link(mem, MemPtr::from_idx(x.idx()));
        v.set_list_ptr(mem, MemPtr::from_idx(p2.idx()));
        let height = v.height(mem).0 + spacings.bos5 + x.height(mem).0 + x.depth(mem).0 + shift_up;
        v.set_height(mem, Scaled(height));
    }

    if q.subscr_type(mem) == EMPTY {
        free_node(mem, z.idx().0, BOX_NODE_SIZE);
    } else {
        let mut shift_down = spacings.bos4 - z.height(mem).0;
        if shift_down < spacings.bos2 {
            shift_down = spacings.bos2;
        }
        let p = alloc_kern_node(mem, shift_down)?;
        y.set_link(mem, MemPtr::from_idx(p.idx()));
        p.set_link(mem, MemPtr::from_idx(z.idx()));
        let p2 = alloc_kern_node(mem, spacings.bos5)?;
        z.set_link(mem, MemPtr::from_idx(p2.idx()));
        let depth = v.depth(mem).0 + spacings.bos5 + z.height(mem).0 + z.depth(mem).0 + shift_down;
        v.set_depth(mem, Scaled(depth));
    }

    q.set_nucleus_type(mem, v.idx().0);
    Ok(())
}

/// translated from xetex-math.c:313 (make_fraction). Threaded SCC core.
unsafe fn make_fraction_in(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), MathBuildAbort> {
    let q_raw = q.idx().0;
    if q.thickness(&st.mem) == DEFAULT_CODE {
        let drt = default_rule_thickness(&st.fonts, &st.eqtb, st.scanner.cur_size);
        q.set_thickness(&mut st.mem, drt);
    }

    let cs = st.scanner.cur_style;
    let mut x = clean_box_in(st, q_raw + 2, cs + 2 - 2 * (cs / 6))?;
    let cs = st.scanner.cur_style;
    let mut z = clean_box_in(st, q_raw + 3, 2 * (cs / 2) + 3 - 2 * (cs / 6))?;

    if x.width(&st.mem).0 < z.width(&st.mem).0 {
        let w = z.width(&st.mem).0;
        x = rebox_in(st, x, w)?;
    } else {
        let w = x.width(&st.mem).0;
        z = rebox_in(st, z, w)?;
    }

    let cur_style = st.scanner.cur_style;
    let cur_size = st.scanner.cur_size;
    let cur_f = st.scanner.cur_f;
    let (mut shift_up, mut shift_down) = if cur_style < TEXT_STYLE {
        (
            num1(&st.fonts, &st.eqtb, cur_size),
            denom1(&st.fonts, &st.eqtb, cur_size),
        )
    } else {
        (
            if q.thickness(&st.mem) != 0 {
                num2(&st.fonts, &st.eqtb, cur_size)
            } else {
                num3(&st.fonts, &st.eqtb, cur_size)
            },
            denom2(&st.fonts, &st.eqtb, cur_size),
        )
    };

    let mut delta = 0;
    if q.thickness(&st.mem) == 0 {
        let clr = if font_is_ot_math(&st.fonts, cur_f) {
            if cur_style < TEXT_STYLE {
                get_ot_math_constant(cur_f, STACKDISPLAYSTYLEGAPMIN)
            } else {
                get_ot_math_constant(cur_f, STACKGAPMIN)
            }
        } else if cur_style < TEXT_STYLE {
            7 * default_rule_thickness(&st.fonts, &st.eqtb, cur_size)
        } else {
            3 * default_rule_thickness(&st.fonts, &st.eqtb, cur_size)
        };

        delta = crate::scaled_math::half(
            clr - ((shift_up - x.depth(&st.mem).0) - (z.height(&st.mem).0 - shift_down)),
        );
        if delta > 0 {
            shift_up += delta;
            shift_down += delta;
        }
    } else {
        let axis_height = axis_height(&st.fonts, &st.eqtb, cur_size);
        let (delta1, delta2) = if font_is_ot_math(&st.fonts, cur_f) {
            delta = crate::scaled_math::half(q.thickness(&st.mem));
            let clr_num = if cur_style < TEXT_STYLE {
                get_ot_math_constant(cur_f, FRACTIONNUMDISPLAYSTYLEGAPMIN)
            } else {
                get_ot_math_constant(cur_f, FRACTIONNUMERATORGAPMIN)
            };
            let delta1 = clr_num - ((shift_up - x.depth(&st.mem).0) - (axis_height + delta));

            let clr_den = if cur_style < TEXT_STYLE {
                get_ot_math_constant(cur_f, FRACTIONDENOMDISPLAYSTYLEGAPMIN)
            } else {
                get_ot_math_constant(cur_f, FRACTIONDENOMINATORGAPMIN)
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

    let axis_height = axis_height(&st.fonts, &st.eqtb, cur_size);
    let rule_thickness = q.thickness(&st.mem);
    let v = make_fraction(
        &mut st.mem,
        FractionAssembly {
            x,
            z,
            rule_thickness,
            shift_up,
            shift_down,
            delta,
            axis_height,
        },
    )?;

    let delimiter_size = if cur_style < TEXT_STYLE {
        delim1(&st.fonts, &st.eqtb, cur_size)
    } else {
        delim2(&st.fonts, &st.eqtb, cur_size)
    };

    let left_delim = var_delimiter_in(st, q_raw + 4, cur_size, delimiter_size)?;
    let right_delim = var_delimiter_in(st, q_raw + 5, cur_size, delimiter_size)?;
    make_fraction_link_delimiters(&mut st.mem, left_delim, v, right_delim);
    let hbox = crate::packaging::hpack_emit_in(st, left_delim.idx().0, 0, ADDITIONAL)?;
    make_fraction_finish(&mut st.mem, q, hbox);
    Ok(())
}

/// translated from xetex-math.c:270 (make_op). Phase-C orchestrator (bucket
/// C/D): the clean mem-write clusters run under short disjoint `mem`/`fonts`/
/// `eqtb`/`scanner` borrows (threaded `new_kern`/`new_null_box`/`free_node` +
/// threaded `big_op_spacing*`/`axis_height`), while the with_state-bearing
/// `fetch_global`/`clean_box`/`rebox_global`/OT-math/measurement stay between
/// borrows. The OT-math glyph-variant/assembly region is left on bridge
/// accessors (genuinely interleaved with `measure_native_glyph` —
/// bucket D). Returns the math `delta` italic correction. A cold allocator
/// exhaustion in the LIMITS tail aborts via `node_recycle_abort` after the
/// borrow drops (`01b` §5).
unsafe fn make_op_in(st: &mut EngineState, q: NodePtr<Noad>) -> Result<i32, MathBuildAbort> {
    use crate::errors::ErrorOutcome;
    let q_raw = q.idx().0;
    let mut delta: i32 = 0;
    let mut ot_assembly_ptr: *mut libc::c_void = std::ptr::null_mut();

    // NORMAL -> LIMITS latch.
    make_op_latch_limits(&mut st.mem, &st.scanner, q);

    if q.nucleus_type(&st.mem) == MATH_CHAR {
        match fetch_in(st, q_raw + 1) {
            ErrorOutcome::Continue => {}
            other => return Err(MathBuildAbort::Error(other)),
        }

        // Non-OpenType italic-correction / list-tag bump.
        let f = st.scanner.cur_f;
        let is_ot = st.fonts.font_area_entry(f) == OTGR_FONT_FLAG
            && usingOpenType(st.fonts.font_layout_engine_ptr(f));
        if !is_ot {
            if st.scanner.cur_style < TEXT_STYLE && st.scanner.cur_i.s1 % 4 == LIST_TAG {
                let c = st.scanner.cur_i.s0;
                let i_info = st
                    .fonts
                    .font_info_b16(st.fonts.char_base_entry(f) + c as i32);
                if i_info.s3 > 0 {
                    st.scanner.cur_c = c as i32;
                    st.scanner.cur_i = i_info;
                    q.set_nucleus_char(&mut st.mem, c);
                }
            }
            delta = st
                .fonts
                .font_info_b32s1(st.fonts.italic_base_entry(f) + st.scanner.cur_i.s1 as i32 / 4);
        }

        let x = clean_box_in(st, q_raw + 1, st.scanner.cur_style)?;
        let x_box = x;

        // OT-math glyph variant / assembly selection (now safe under the borrow,
        // session 207): `measure_native_glyph` is `&FontState`, the `get_ot_*`
        // FFI reads `font_*` through the borrow-free mirror.
        let cur_f = st.scanner.cur_f;
        if st.fonts.font_area_entry(cur_f) == OTGR_FONT_FLAG
            && isOpenTypeMathFont(st.fonts.font_layout_engine_ptr(cur_f))
        {
            let mut p = x_box.list_ptr(&st.mem).0;
            if p != TEX_NULL
                && !crate::mem::is_char_node(&st.mem, p)
                && st.mem.b16s1(p) == WHATSIT_NODE
                && WhatsitSubtype::from_raw(st.mem.b16s0(p)) == Some(WhatsitSubtype::Glyph)
            {
                'found: {
                    let p_w = st
                        .mem
                        .classify_whatsit(MemIdx::new(p))
                        .expect("valid whatsit")
                        .expect("glyph node");
                    if st.scanner.cur_style < TEXT_STYLE {
                        let mut h1 = get_ot_math_constant(cur_f, DISPLAYOPERATORMINHEIGHT);
                        let size_h =
                            (p_w.height(&st.mem).0 + p_w.depth(&st.mem).0) as f64 * 5.0 / 4.0;
                        if (h1 as f64) < size_h {
                            h1 = size_h as i32;
                        }
                        let c_glyph = p_w.native_glyph(&st.mem);
                        let mut n: i32 = 0;
                        let mut h2: i32 = 0;
                        loop {
                            let g = get_ot_math_variant(cur_f, c_glyph as i32, n, &mut h2, 0);
                            if h2 > 0 {
                                p_w.set_native_glyph(&mut st.mem, g as u16);
                                let addr = node_addr_in(st, p);
                                crate::ext::measure_native_glyph(&st.fonts, addr, 1)?;
                            }
                            n += 1;
                            if h2 < 0 || h2 >= h1 {
                                break;
                            }
                        }
                        if h2 < 0 {
                            let assembly_ptr = get_ot_assembly_ptr(cur_f, c_glyph as i32, 0);
                            if !assembly_ptr.is_null() {
                                ot_assembly_ptr = assembly_ptr;
                                crate::mem::free_node(&mut st.mem, p, GLYPH_NODE_SIZE);
                                p = build_opentype_assembly_in(
                                    st,
                                    cur_f,
                                    ot_assembly_ptr,
                                    h1,
                                    false,
                                )?
                                .idx()
                                .0;
                                x_box.set_list_ptr(&mut st.mem, MemPtr::from_raw(p));
                                delta = 0;
                                break 'found;
                            }
                        } else {
                            let addr = node_addr_in(st, p);
                            crate::ext::measure_native_glyph(&st.fonts, addr, 1)?;
                        }
                    }
                    delta = get_ot_math_ital_corr(cur_f, p_w.native_glyph(&st.mem) as i32);
                }
                // found:
                let p1 = st.mem.b32s1(p + 1);
                let p3 = st.mem.b32s1(p + 3);
                let p2 = st.mem.b32s1(p + 2);
                x_box.set_width(&mut st.mem, Scaled(p1));
                x_box.set_height(&mut st.mem, Scaled(p3));
                x_box.set_depth(&mut st.mem, Scaled(p2));
            }
        }

        // Finish the MATH_CHAR box.
        let size = st.scanner.cur_size;
        let ah = axis_height(&st.fonts, &st.eqtb, size);
        make_op_finish_math_char(&mut st.mem, q, x_box, delta, ah);
    }

    let save_f = st.scanner.cur_f;
    if st.mem.b16s0(q_raw) == LIMITS {
        let cs = st.scanner.cur_style as i32;
        let mut x = clean_box_in(st, q_raw + 2, (2 * (cs / 4) + 4 + (cs % 2)) as i16)?;
        let mut y = clean_box_in(st, q_raw + 1, st.scanner.cur_style)?;
        let mut z = clean_box_in(st, q_raw + 3, (2 * (cs / 4) + 5) as i16)?;

        // Allocate the wrapper vbox directly as a typed handle (`Copy`, holds no
        // borrow, so it stays valid across the `rebox_in` calls below).
        let v_box = AnyBox::VList(alloc_null_vbox(&mut st.mem)?);
        let w = make_op_prepare_limits_box(&mut st.mem, v_box, x, y, z);

        x = rebox_in(st, x, w)?;
        y = rebox_in(st, y, w)?;
        z = rebox_in(st, z, w)?;

        let size = st.scanner.cur_size;
        let spacings = BigOpSpacings {
            bos1: big_op_spacing1(&st.fonts, &st.eqtb, size),
            bos2: big_op_spacing2(&st.fonts, &st.eqtb, size),
            bos3: big_op_spacing3(&st.fonts, &st.eqtb, size),
            bos4: big_op_spacing4(&st.fonts, &st.eqtb, size),
            bos5: big_op_spacing5(&st.fonts, &st.eqtb, size),
        };
        make_op_finish_limits(
            &mut st.mem,
            &mut st.scanner,
            q,
            x,
            y,
            z,
            v_box,
            delta,
            save_f,
            spacings,
        )?;
    }

    free_ot_assembly(ot_assembly_ptr);
    Ok(delta)
}

/// Outcome of one threaded [`make_ord_body`] pass (`01b` §4a / §5): the body
/// runs under a disjoint `mem`/`fonts`/`scanner` borrow, but the `\restart`
/// re-runs the with_state-bearing `fetch_global`, and allocator exhaustion must
/// abort after the borrow drops, so it cannot loop or abort itself.
enum MakeOrdStep {
    /// Done — return from `make_ord`.
    Return,
    /// A ligature replaced the nucleus — re-run the `\restart` loop.
    Restart,
    /// `new_kern` / `new_noad` exhausted the arena — abort after the borrow drops.
    Overflow,
}

/// Threaded core (Phase C, bucket B): the ligature/kern body of `make_ord`,
/// running entirely on direct `mem` / `fonts` / `scanner` access (no
/// `with_state`-bearing call). `cur_f` / `cur_i` were already set by the
/// preceding `fetch_global`. Allocator exhaustion returns `Overflow`.
fn make_ord_body(
    mem: &mut MemState,
    fonts: &FontState,
    scanner: &mut ScannerState,
    q: NodePtr<Noad>,
    p: NodePtr<Noad>,
) -> MakeOrdStep {
    let mut a = fonts.lig_kern_base_entry(scanner.cur_f) + scanner.cur_i.s0 as i32;
    scanner.cur_c = p.nucleus_char(mem) as i32;
    scanner.cur_i = fonts.font_info_b16(a);
    if scanner.cur_i.s3 > 128 {
        a = fonts.lig_kern_base_entry(scanner.cur_f)
            + 256 * scanner.cur_i.s1 as i32
            + scanner.cur_i.s0 as i32
            + 32768
            - 256 * 128;
        scanner.cur_i = fonts.font_info_b16(a);
    }
    loop {
        if scanner.cur_i.s2 == scanner.cur_c as u16 && scanner.cur_i.s3 <= 128 {
            if scanner.cur_i.s1 >= 128 {
                let kern_width = fonts.font_info_b32s1(
                    fonts.kern_base_entry(scanner.cur_f)
                        + 256 * scanner.cur_i.s1 as i32
                        + scanner.cur_i.s0 as i32,
                );
                let Ok(p_kern) = alloc_kern_node(mem, kern_width) else {
                    return MakeOrdStep::Overflow;
                };
                let link_q = q.link(mem);
                p_kern.set_link(mem, link_q);
                q.set_link(mem, MemPtr::from_idx(p_kern.idx()));
                return MakeOrdStep::Return;
            } else {
                match scanner.cur_i.s1 {
                    1 | 5 => {
                        q.set_nucleus_char(mem, scanner.cur_i.s0);
                    }
                    2 | 6 => {
                        p.set_nucleus_char(mem, scanner.cur_i.s0);
                    }
                    3 | 7 | 11 => {
                        let r_idx = new_noad(mem);
                        if r_idx == ALLOC_FAILED {
                            return MakeOrdStep::Overflow;
                        }
                        let r = unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(r_idx)) };
                        r.set_nucleus_char(mem, scanner.cur_i.s0);
                        r.set_nucleus_fam(mem, q.nucleus_fam(mem) % 256);
                        q.set_link(mem, MemPtr::from_raw(r_idx));
                        r.set_link(mem, MemPtr::from_idx(p.idx()));
                        if scanner.cur_i.s1 < 11 {
                            r.set_nucleus_type(mem, MATH_CHAR);
                        } else {
                            r.set_nucleus_type(mem, MATH_TEXT_CHAR);
                        }
                    }
                    _ => {
                        let link_p = p.link(mem);
                        q.set_link(mem, link_p);
                        q.set_nucleus_char(mem, scanner.cur_i.s0);
                        let w3 = mem.word(p.idx().offset(3).0);
                        mem.set_word(q.idx().offset(3).0, w3);
                        let w2 = mem.word(p.idx().offset(2).0);
                        mem.set_word(q.idx().offset(2).0, w2);
                        free_node(mem, p.idx().0, NOAD_SIZE);
                    }
                }
                if scanner.cur_i.s1 > 3 {
                    return MakeOrdStep::Return;
                }
                q.set_nucleus_type(mem, MATH_CHAR);
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

/// translated from xetex-math.c:272 (make_ord). Orchestrator: the early mem
/// checks + the with_state-bearing `fetch_global` run between borrows; each
/// `\restart` iteration runs the threaded [`make_ord_body`] under one disjoint
/// `mem`/`fonts`/`scanner` borrow and aborts a cold allocator overflow after it
/// drops.
/// Deferred-abort sentinel for the threaded [`make_ord_in`] (`01b` §5): the
/// allocator overflow and the recoverable `fetch` error both abort after the
/// state borrow drops.
enum MakeOrdAbort {
    Overflow,
    Error(crate::errors::ErrorOutcome),
}

fn make_ord_in(st: &mut EngineState, q: NodePtr<Noad>) -> Result<(), MakeOrdAbort> {
    use crate::errors::ErrorOutcome;
    let q_raw = q.idx().0;
    'restart: loop {
        if q.subscr_type(&st.mem) != EMPTY {
            return Ok(());
        }
        if q.supscr_type(&st.mem) != EMPTY {
            return Ok(());
        }
        if q.nucleus_type(&st.mem) != MATH_CHAR {
            return Ok(());
        }
        let p = st.mem.b32s1(q_raw);
        if p == TEX_NULL {
            return Ok(());
        }
        if st.mem.b16s1(p) < NodeType::OrdNoad.as_raw() || st.mem.b16s1(p) > NodeType::PunctNoad.as_raw() {
            return Ok(());
        }
        // `p` is now a known ORD..PUNCT noad.
        let p_node = assume_noad(p);
        if p_node.nucleus_type(&st.mem) != MATH_CHAR {
            return Ok(());
        }
        if p_node.nucleus_fam(&st.mem) % 256 != q.nucleus_fam(&st.mem) % 256 {
            return Ok(());
        }

        q.set_nucleus_type(&mut st.mem, MATH_TEXT_CHAR);
        match fetch_in(st, q_raw + 1) {
            ErrorOutcome::Continue => {}
            other => return Err(MakeOrdAbort::Error(other)),
        }
        if st.scanner.cur_i.s1 % 4 != LIG_TAG {
            return Ok(());
        }

        match make_ord_body(&mut st.mem, &st.fonts, &mut st.scanner, q, p_node) {
            MakeOrdStep::Return => return Ok(()),
            MakeOrdStep::Restart => continue 'restart,
            MakeOrdStep::Overflow => return Err(MakeOrdAbort::Overflow),
        }
    }
}

struct MathAccentInitial {
    f: i32,
    c: i32,
    i: crate::mem_types::B16x4,
    area: i32,
    kind: u16,
}

fn math_accent_initial(
    mem: &MemState,
    fonts: &FontState,
    scanner: &ScannerState,
    q: NodePtr<Noad>,
) -> MathAccentInitial {
    MathAccentInitial {
        f: scanner.cur_f,
        c: scanner.cur_c,
        i: scanner.cur_i,
        area: fonts.font_area_entry(scanner.cur_f),
        kind: q.subtype(mem),
    }
}

fn box_height_width(mem: &MemState, b: AnyBox) -> (i32, i32) {
    (b.width(mem).0, b.height(mem).0)
}

fn accent_base_needs_reclean(mem: &MemState, q: NodePtr<Noad>) -> bool {
    (q.supscr_type(mem) != EMPTY || q.subscr_type(mem) != EMPTY) && q.nucleus_type(mem) == MATH_CHAR
}

fn move_accent_base_scripts_to_mlist(
    mem: &mut MemState,
    q: NodePtr<Noad>,
    box_node: i32,
) -> Result<(), NodeRecycleAbort> {
    crate::packaging::flush_node_list(mem, box_node).map_err(NodeRecycleAbort::Confusion)?;
    let x = new_noad(mem);
    if x == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }

    mem.set_word(x + 1, mem.word(q.idx().offset(1).0));
    mem.set_word(x + 2, mem.word(q.idx().offset(2).0));
    mem.set_word(x + 3, mem.word(q.idx().offset(3).0));
    q.set_supscr_field(mem, 0);
    q.set_supscr_type(mem, 0);
    q.set_subscr_field(mem, 0);
    q.set_subscr_type(mem, 0);
    q.set_nucleus_type(mem, SUB_MLIST);
    q.set_nucleus_field(mem, x);
    Ok(())
}

fn math_accent_delta(
    fonts: &FontState,
    f: i32,
    kind: u16,
    h: i32,
    is_ot_math: bool,
    accent_base_height: i32,
) -> i32 {
    if is_ot_math {
        if kind == BOTTOM_ACC || kind == BOTTOM_ACC + 1 {
            0
        } else if h < accent_base_height {
            h
        } else {
            accent_base_height
        }
    } else {
        let x_height = fonts.font_info_b32s1(X_HEIGHT_CODE + fonts.param_base(f));
        if h < x_height {
            h
        } else {
            x_height
        }
    }
}

#[derive(Clone, Copy)]
struct NativeAccentGlyph {
    p: i32,
    old_node: i32,
}

fn start_native_accent_glyph(
    mem: &mut MemState,
    y: AnyBox,
    f: i32,
) -> Result<NativeAccentGlyph, NodeRecycleAbort> {
    let pw = alloc_glyph_whatsit(mem)?;
    let old_node = y.list_ptr(mem).0;
    pw.set_native_font(mem, f as u16);
    Ok(NativeAccentGlyph {
        p: pw.idx().0,
        old_node,
    })
}

fn set_native_accent_glyph_id(mem: &mut MemState, glyph: NativeAccentGlyph, glyph_id: u16) {
    mem.set_b16s1(glyph.p + 4, glyph_id);
}

fn finish_native_accent_glyph_start(mem: &mut MemState, y: AnyBox, glyph: NativeAccentGlyph) {
    free_node(mem, glyph.old_node, mem.b16s3(glyph.old_node + 4) as i32);
    y.set_list_ptr(mem, MemPtr::from_raw(glyph.p));
}

fn sync_accent_box_to_glyph(mem: &mut MemState, y: AnyBox, p: i32, bottom: bool) {
    y.set_width(mem, Scaled(mem.b32s1(p + 1)));
    y.set_height(mem, Scaled(mem.b32s1(p + 3)));
    y.set_depth(mem, Scaled(mem.b32s1(p + 2)));
    if bottom {
        if y.height(mem).0 < 0 {
            y.set_height(mem, Scaled(0));
        }
    } else if y.depth(mem).0 < 0 {
        y.set_depth(mem, Scaled(0));
    }
}

fn set_accent_char_box_shift(mem: &mut MemState, y: AnyBox, shift: i32) {
    y.set_shift_amount(mem, Scaled(shift));
}

fn zero_box_width(mem: &mut MemState, box_node: AnyBox) {
    box_node.set_width(mem, Scaled(0));
}

fn link_bottom_accent_stack(mem: &mut MemState, x: AnyBox, y: AnyBox) {
    x.set_link(mem, MemPtr::from_idx(y.idx()));
}

fn start_top_accent_stack(
    mem: &mut MemState,
    x: AnyBox,
    y: AnyBox,
    delta: i32,
) -> Result<(), NodeRecycleAbort> {
    let p = alloc_kern_node(mem, -delta)?;
    p.set_link(mem, MemPtr::from_idx(x.idx()));
    y.set_link(mem, MemPtr::from_idx(p.idx()));
    Ok(())
}

fn finish_top_accent_package(
    mem: &mut MemState,
    y: AnyBox,
    h: i32,
) -> Result<(), NodeRecycleAbort> {
    if y.height(mem).0 < h {
        let p = alloc_kern_node(mem, h - y.height(mem).0)?;
        p.set_link(mem, y.list_ptr(mem));
        y.set_list_ptr(mem, MemPtr::from_idx(p.idx()));
        y.set_height(mem, Scaled(h));
    }
    Ok(())
}

fn finish_accent_nucleus(mem: &mut MemState, q: NodePtr<Noad>, x: AnyBox, y: AnyBox) {
    y.set_width(mem, x.width(mem));
    q.set_nucleus_field(mem, y.idx().0);
    q.set_nucleus_type(mem, SUB_BOX);
}

/// translated from xetex-math.c:79 (make_math_accent). Threaded SCC core: one
/// borrow across the AAT/OTGR/TFM accent build (OT readers borrow-safe, session 207).
unsafe fn make_math_accent_in(
    st: &mut EngineState,
    q: NodePtr<Noad>,
) -> Result<(), MathBuildAbort> {
    use crate::errors::ErrorOutcome;
    let q_raw = q.idx().0;
    match fetch_in(st, q_raw + 4) {
        ErrorOutcome::Continue => {}
        other => return Err(MathBuildAbort::Error(other)),
    }
    let mut x: i32;
    let mut ot_assembly_ptr: *mut libc::c_void = std::ptr::null_mut();
    let mut c: i32;
    let f: i32;
    let mut s: i32;
    let w: i32;
    let mut h: i32;

    let initial = math_accent_initial(&st.mem, &st.fonts, &st.scanner, q);

    if initial.area == AAT_FONT_FLAG || initial.area == OTGR_FONT_FLAG {
        c = initial.c;
        f = initial.f;
        if !(initial.kind == BOTTOM_ACC || initial.kind == BOTTOM_ACC + 1) {
            s = compute_ot_math_accent_pos_in(st, q_raw)?;
        } else {
            s = 0;
        }
        let clean_style = 2 * (st.scanner.cur_style / 2) + 1;
        let xb = clean_box_in(st, q_raw + 1, clean_style)?;
        (w, h) = box_height_width(&st.mem, xb);
        x = xb.idx().0;
    } else if initial.i.s3 > 0 {
        let mut acc_i = initial.i;
        c = initial.c;
        f = initial.f;
        s = 0;
        let base_is_math_char = q.nucleus_type(&st.mem) == MATH_CHAR;
        if base_is_math_char {
            match fetch_in(st, q_raw + 1) {
                ErrorOutcome::Continue => {}
                other => return Err(MathBuildAbort::Error(other)),
            }
            s = if st.scanner.cur_i.s1 % 4 != LIG_TAG {
                0
            } else {
                let mut a =
                    st.fonts.lig_kern_base_entry(st.scanner.cur_f) + st.scanner.cur_i.s0 as i32;
                st.scanner.cur_i = st.fonts.font_info_b16(a);
                if st.scanner.cur_i.s3 > 128 {
                    a = st.fonts.lig_kern_base_entry(st.scanner.cur_f)
                        + 256 * st.scanner.cur_i.s1 as i32
                        + st.scanner.cur_i.s0 as i32
                        + 32768
                        - 256 * 128;
                    st.scanner.cur_i = st.fonts.font_info_b16(a);
                }
                loop {
                    if st.scanner.cur_i.s2 == st.fonts.skew_char_entry(st.scanner.cur_f) as u16 {
                        if st.scanner.cur_i.s1 >= 128 && st.scanner.cur_i.s3 <= 128 {
                            break st.fonts.font_info_b32s1(
                                st.fonts.kern_base_entry(st.scanner.cur_f)
                                    + 256 * st.scanner.cur_i.s1 as i32
                                    + st.scanner.cur_i.s0 as i32,
                            );
                        }
                        break 0;
                    }
                    if st.scanner.cur_i.s3 >= 128 {
                        break 0;
                    }
                    a += st.scanner.cur_i.s3 as i32 + 1;
                    st.scanner.cur_i = st.fonts.font_info_b16(a);
                }
            };
        }
        let clean_style = 2 * (st.scanner.cur_style / 2) + 1;
        let xb = clean_box_in(st, q_raw + 1, clean_style)?;
        (w, h) = box_height_width(&st.mem, xb);
        x = xb.idx().0;
        loop {
            if acc_i.s1 % 4 != LIST_TAG {
                break;
            }
            let y_tmp = acc_i.s0 as i32;
            acc_i = st.fonts.font_info_b16(st.fonts.char_base_entry(f) + y_tmp);
            if acc_i.s3 <= 0 {
                break;
            }
            let acc_width = st
                .fonts
                .font_info_b32s1(st.fonts.width_base_entry(f) + acc_i.s3 as i32);
            if acc_width > w {
                break;
            }
            c = y_tmp;
        }
        // done:
    } else {
        return Ok(());
    }

    if x == TEX_NULL {
        free_ot_assembly(ot_assembly_ptr);
        return Ok(());
    }

    let bottom_accent = initial.kind == BOTTOM_ACC || initial.kind == BOTTOM_ACC + 1;
    let is_ot_math =
        initial.area == OTGR_FONT_FLAG && isOpenTypeMathFont(st.fonts.font_layout_engine_ptr(f));
    let accent_base_height = if is_ot_math && !bottom_accent {
        get_ot_math_constant(f, ACCENTBASEHEIGHT)
    } else {
        0
    };
    let mut delta: i32 = math_accent_delta(
        &st.fonts,
        f,
        initial.kind,
        h,
        is_ot_math,
        accent_base_height,
    );

    let needs_reclean = accent_base_needs_reclean(&st.mem, q);
    if needs_reclean {
        move_accent_base_scripts_to_mlist(&mut st.mem, q, x)?;
        let xb = clean_box_in(st, q_raw + 1, st.scanner.cur_style)?;
        let new_h = xb.height(&st.mem).0;
        x = xb.idx().0;
        delta += new_h - h;
        h = new_h;
    }

    let y = char_box_in(st, f, c)?;
    let mut p: i32;
    if initial.area == AAT_FONT_FLAG || initial.area == OTGR_FONT_FLAG {
        let glyph = start_native_accent_glyph(&mut st.mem, y, f)?;
        let glyph_id = {
            let addr = node_addr_in(st, glyph.old_node);
            real_get_native_glyph(addr, 0)
        };
        set_native_accent_glyph_id(&mut st.mem, glyph, glyph_id);
        p = glyph.p;
        {
            let addr = node_addr_in(st, p);
            crate::ext::measure_native_glyph(&st.fonts, addr, 1)?;
        }
        finish_native_accent_glyph_start(&mut st.mem, y, glyph);
        if !initial.kind.is_multiple_of(2) {
            let addr = node_addr_in(st, p);
            crate::ext::measure_native_glyph(&st.fonts, addr, 1)?;
        } else {
            let c_glyph = st.mem.b16s1(p + 4) as i32;
            let mut a_var: i32 = 0;
            let mut w2: i32 = 0;
            loop {
                let g = get_ot_math_variant(f, c_glyph, a_var, &mut w2, 1);
                if w2 > 0 && w2 <= w {
                    st.mem.set_b16s1(p + 4, g as u16);
                    let addr = node_addr_in(st, p);
                    crate::ext::measure_native_glyph(&st.fonts, addr, 1)?;
                    a_var += 1;
                }
                if w2 < 0 || w2 >= w {
                    break;
                }
            }
            'found: {
                if w2 < 0 {
                    let assembly_ptr = get_ot_assembly_ptr(f, c_glyph, 1);
                    if !assembly_ptr.is_null() {
                        ot_assembly_ptr = assembly_ptr;
                        crate::mem::free_node(&mut st.mem, p, GLYPH_NODE_SIZE);
                        p = build_opentype_assembly_in(st, f, ot_assembly_ptr, w, true)?
                            .idx()
                            .0;
                        y.set_list_ptr(&mut st.mem, MemPtr::from_raw(p));
                        break 'found;
                    }
                } else {
                    let addr = node_addr_in(st, p);
                    crate::ext::measure_native_glyph(&st.fonts, addr, 1)?;
                }
            }
            // found:
        }
        let y_box = y;
        sync_accent_box_to_glyph(&mut st.mem, y_box, p, bottom_accent);
        if p != TEX_NULL
            && !crate::mem::is_char_node(&st.mem, p)
            && st.mem.b16s1(p) == WHATSIT_NODE
            && WhatsitSubtype::from_raw(st.mem.b16s0(p)) == Some(WhatsitSubtype::Glyph)
        {
            let sa0 = get_ot_math_accent_pos(f, st.mem.b16s1(p + 4) as i32);
            let sa = if sa0 == TEX_INFINITY {
                crate::scaled_math::half(y_box.width(&st.mem).0)
            } else {
                sa0
            };
            if bottom_accent || s == TEX_INFINITY {
                s = crate::scaled_math::half(w);
            }
            y_box.set_shift_amount(&mut st.mem, Scaled(s - sa));
        } else {
            let sa = crate::scaled_math::half(y_box.width(&st.mem).0);
            if bottom_accent || s == TEX_INFINITY {
                s = crate::scaled_math::half(w);
            }
            y_box.set_shift_amount(&mut st.mem, Scaled(s - sa));
        }
    } else {
        let shift = s + crate::scaled_math::half(w - y.width(&st.mem).0);
        set_accent_char_box_shift(&mut st.mem, y, shift);
        p = TEX_NULL;
    }
    let _ = p;

    // `x` was proven non-null at the top guard; classify it once here and reuse
    // the handle across both accent-stacking branches.
    let x_box = classify_box_or_panic(&st.mem, x);
    zero_box_width(&mut st.mem, y);
    if bottom_accent {
        link_bottom_accent_stack(&mut st.mem, x_box, y);
        let y_box = crate::packaging::vpackage_emit_in(st, x, 0, ADDITIONAL, MAX_HALFWORD)?;
        let y_height = y_box.height(&st.mem).0;
        y_box.set_shift_amount(&mut st.mem, Scaled(-(h - y_height)));
        finish_accent_nucleus(&mut st.mem, q, x_box, y_box);
    } else {
        start_top_accent_stack(&mut st.mem, x_box, y, delta)?;
        let y_box = crate::packaging::vpackage_emit_in(st, y.idx().0, 0, ADDITIONAL, MAX_HALFWORD)?;
        finish_top_accent_package(&mut st.mem, y_box, h)?;
        finish_accent_nucleus(&mut st.mem, q, x_box, y_box);
    }

    free_ot_assembly(ot_assembly_ptr);
    Ok(())
}

/// translated from xetex-math.c:275 (attach_hkern_to_new_hlist)
/// Threaded core (Phase C, bucket B): builds a kern and appends it to the
/// `new_hlist` of noad `q`, all in `mem`. `new_kern` exhaustion propagates the
/// `01b` §5 [`NodeRecycleAbort::Overflow`] sentinel; `attach_hkern_to_new_hlist_global`
/// aborts after the borrow drops.
fn attach_hkern_to_new_hlist(
    mem: &mut MemState,
    q: NodePtr<Noad>,
    delta: Scaled,
) -> Result<MemPtr, NodeRecycleAbort> {
    let Some(z) = NodePtr::<KernNode>::alloc(mem) else {
        return Err(NodeRecycleAbort::Overflow);
    };
    z.set_subtype(mem, NORMAL);
    z.set_width(mem, delta);

    let head = MemPtr::from_raw(q.nucleus_type(mem));
    if head.is_null() {
        q.set_nucleus_type(mem, z.idx().0);
    } else {
        let mut y = head.as_idx().unwrap();
        while mem.b32s1(y.0) != TEX_NULL {
            y = MemIdx::new(mem.b32s1(y.0));
        }
        mem.set_b32s1(y.0, z.idx().0);
    }
    Ok(MemPtr::from_raw(q.nucleus_type(mem)))
}

unsafe fn attach_hkern_to_new_hlist_in(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    delta: i32,
) -> Result<i32, MathBuildAbort> {
    Ok(attach_hkern_to_new_hlist(&mut st.mem, q, Scaled(delta))?.0)
}

#[derive(Clone, Copy)]
struct ScriptGlyphNucleus {
    font: i32,
    glyph: u16,
}

struct ScriptKernAttachment {
    p: i32,
    kern: i32,
    is_ot_math: bool,
}

fn glyph_nucleus(mem: &MemState, p: i32) -> Option<ScriptGlyphNucleus> {
    if p != TEX_NULL
        && !crate::mem::is_char_node(mem, p)
        && mem.b16s1(p) == WHATSIT_NODE
        && WhatsitSubtype::from_raw(mem.b16s0(p)) == Some(WhatsitSubtype::Glyph)
    {
        let pw = mem
            .classify_whatsit(MemIdx::new(p))
            .expect("valid whatsit")
            .expect("glyph node");
        Some(ScriptGlyphNucleus {
            font: pw.native_font(mem).0,
            glyph: pw.native_glyph(mem),
        })
    } else {
        None
    }
}

unsafe fn maybe_attach_script_kern_in(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    p: i32,
    script_f: i32,
    script_g: u16,
    cmd: i32,
    shift: i32,
) -> Result<ScriptKernAttachment, MathBuildAbort> {
    let cur_f = st.scanner.cur_f;
    let nucleus = glyph_nucleus(&st.mem, p);

    if !font_is_ot_math(&st.fonts, cur_f) {
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

    let kern = get_ot_math_kern(nucleus.font, nucleus.glyph, script_f, script_g, cmd, shift);
    Ok(ScriptKernAttachment {
        p: if kern == 0 {
            p
        } else {
            attach_hkern_to_new_hlist_in(st, q, kern)?
        },
        kern,
        is_ot_math: true,
    })
}

fn add_script_space_and_restore_font(
    mem: &mut MemState,
    eqtb: &EqtbState,
    scanner: &mut ScannerState,
    box_node: AnyBox,
    saved_font: i32,
) {
    scanner.cur_f = saved_font;
    let w = box_node.width(mem).0 + eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_SCRIPT_SPACE);
    box_node.set_width(mem, Scaled(w));
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

unsafe fn clean_script_box_in(
    st: &mut EngineState,
    script_field: i32,
    clean_style: i16,
    saved_font: i32,
    adjustment: ScriptBoxAdjustment,
) -> Result<ScriptBoxPrep, MathBuildAbort> {
    let node = clean_box_in(st, script_field, clean_style)?;
    let shift = match adjustment {
        ScriptBoxAdjustment::Sub1 => sub1(&st.fonts, &st.eqtb, st.scanner.cur_size),
        ScriptBoxAdjustment::Sub2 => sub2(&st.fonts, &st.eqtb, st.scanner.cur_size),
        ScriptBoxAdjustment::Superscript => {
            if st.scanner.cur_style % 2 != 0 {
                sup3(&st.fonts, &st.eqtb, st.scanner.cur_size)
            } else if st.scanner.cur_style < TEXT_STYLE {
                sup1(&st.fonts, &st.eqtb, st.scanner.cur_size)
            } else {
                sup2(&st.fonts, &st.eqtb, st.scanner.cur_size)
            }
        }
    };
    let box_node = node;
    add_script_space_and_restore_font(&mut st.mem, &st.eqtb, &mut st.scanner, box_node, saved_font);

    Ok(ScriptBoxPrep { box_node, shift })
}

struct ScriptShifts {
    up: i32,
    down: i32,
}

unsafe fn initial_script_shifts_in(
    st: &mut EngineState,
    p: i32,
) -> Result<ScriptShifts, MathBuildAbort> {
    if crate::mem::is_char_node(&st.mem, p) || glyph_nucleus(&st.mem, p).is_some() {
        Ok(ScriptShifts { up: 0, down: 0 })
    } else {
        let zb = crate::packaging::hpack_emit_in(st, p, 0, ADDITIONAL)?;
        let t = if st.scanner.cur_style < SCRIPT_STYLE {
            SCRIPT_SIZE
        } else {
            SCRIPT_SCRIPT_SIZE
        };
        let shifts = ScriptShifts {
            up: zb.height(&st.mem).0 - sup_drop(&st.fonts, &st.eqtb, t),
            down: zb.depth(&st.mem).0 + sub_drop(&st.fonts, &st.eqtb, t),
        };
        free_node(&mut st.mem, zb.idx().0, BOX_NODE_SIZE);
        Ok(shifts)
    }
}

fn append_script_box(mem: &mut MemState, q: NodePtr<Noad>, box_node: AnyBox) {
    let box_idx = box_node.idx().0;
    if q.nucleus_type(mem) == TEX_NULL {
        q.set_nucleus_type(mem, box_idx);
    } else {
        let mut p = q.nucleus_type(mem);
        while mem.b32s1(p) != TEX_NULL {
            p = mem.b32s1(p);
        }
        mem.set_b32s1(p, box_idx);
    }
}

fn append_script_box_in(st: &mut EngineState, q: NodePtr<Noad>, box_node: AnyBox) {
    append_script_box(&mut st.mem, q, box_node);
}

struct SubSupStackInput {
    q: NodePtr<Noad>,
    p: i32,
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

unsafe fn finish_subsup_stack_in(
    st: &mut EngineState,
    input: SubSupStackInput,
) -> Result<(), MathBuildAbort> {
    let mut p = input.p;
    let mut sub_kern;
    let mut sup_kern = 0;
    let attachment = maybe_attach_script_kern_in(
        st,
        input.q,
        p,
        input.sub_f,
        input.sub_g,
        SUB_CMD,
        input.shift_down,
    )?;
    p = attachment.p;
    sub_kern = attachment.kern;
    if attachment.is_ot_math {
        let sub_empty = input.q.subscr_type(&st.mem) == EMPTY;
        if sub_empty {
            let attachment = maybe_attach_script_kern_in(
                st,
                input.q,
                p,
                input.sup_f,
                input.sup_g,
                SUP_CMD,
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
    let kern = new_kern(&mut st.mem, kern_width);
    if kern == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow.into());
    }
    input.x.set_link(&mut st.mem, MemPtr::from_raw(kern));
    st.mem.set_b32s1(kern, input.y.idx().0);

    let packaged =
        crate::packaging::vpackage_emit_in(st, input.x.idx().0, 0, ADDITIONAL, MAX_HALFWORD)?;
    packaged.set_shift_amount(&mut st.mem, Scaled(input.shift_down));
    append_script_box_in(st, input.q, packaged);
    Ok(())
}

struct PreparedScript {
    box_node: AnyBox,
    glyph: u16,
    font: i32,
    shift: i32,
}

unsafe fn prepare_subscript_in(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    adjustment: ScriptBoxAdjustment,
    mut shift_down: i32,
    saved_font_before_glyph_probe: Option<i32>,
) -> Result<PreparedScript, MathBuildAbort> {
    let script_field = q.subscr_addr();
    let (save_f, glyph, font) = if let Some(save_f) = saved_font_before_glyph_probe {
        let (glyph, font) = first_script_glyph_in(st, script_field)?;
        (save_f, glyph, font)
    } else {
        let (glyph, font) = first_script_glyph_in(st, script_field)?;
        let save_f = st.scanner.cur_f;
        (save_f, glyph, font)
    };
    let clean_style = 2 * (st.scanner.cur_style / 4) + 5;
    let prepared = clean_script_box_in(st, script_field, clean_style, save_f, adjustment)?;
    if shift_down < prepared.shift {
        shift_down = prepared.shift;
    }

    Ok(PreparedScript {
        box_node: prepared.box_node,
        glyph,
        font,
        shift: shift_down,
    })
}

unsafe fn finish_subscript_only_in(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    p: i32,
    shift_down: i32,
) -> Result<(), MathBuildAbort> {
    let PreparedScript {
        box_node: x,
        glyph: sub_g,
        font: sub_f,
        shift: mut shift_down,
    } = prepare_subscript_in(st, q, ScriptBoxAdjustment::Sub1, shift_down, None)?;

    let cur_f = st.scanner.cur_f;
    let box_depth = x.height(&st.mem).0;
    let x_height = math_x_height(&st.fonts, &st.eqtb, st.scanner.cur_size);
    let clr = if font_is_ot_math(&st.fonts, cur_f) {
        box_depth - get_ot_math_constant(cur_f, SUBSCRIPTTOPMAX)
    } else {
        box_depth - (x_height * 4).abs() / 5
    };

    if shift_down < clr {
        shift_down = clr;
    }

    x.set_shift_amount(&mut st.mem, Scaled(shift_down));

    let _ = maybe_attach_script_kern_in(st, q, p, sub_f, sub_g, SUB_CMD, shift_down)?;
    append_script_box_in(st, q, x);
    Ok(())
}

unsafe fn prepare_superscript_in(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    mut shift_up: i32,
) -> Result<PreparedScript, MathBuildAbort> {
    let (glyph, font) = first_script_glyph_in(st, q.supscr_addr())?;
    let save_f = st.scanner.cur_f;
    let clean_style = 2 * (st.scanner.cur_style / 4) + 4 + (st.scanner.cur_style % 2);
    let prepared = clean_script_box_in(
        st,
        q.supscr_addr(),
        clean_style,
        save_f,
        ScriptBoxAdjustment::Superscript,
    )?;
    if shift_up < prepared.shift {
        shift_up = prepared.shift;
    }

    let cur_f = st.scanner.cur_f;
    let box_height = prepared.box_node.depth(&st.mem).0;
    let x_height = math_x_height(&st.fonts, &st.eqtb, st.scanner.cur_size);
    let clr = if font_is_ot_math(&st.fonts, cur_f) {
        box_height + get_ot_math_constant(cur_f, SUPERSCRIPTBOTTOMMIN)
    } else {
        box_height + x_height.abs() / 4
    };

    if shift_up < clr {
        shift_up = clr;
    }

    Ok(PreparedScript {
        box_node: prepared.box_node,
        glyph,
        font,
        shift: shift_up,
    })
}

unsafe fn finish_superscript_branch_in(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    p: i32,
    superscript: PreparedScript,
    mut shift_down: i32,
    delta: i32,
) -> Result<(), MathBuildAbort> {
    let x = superscript.box_node;
    let shift_up = superscript.shift;
    let sub_empty = q.subscr_type(&st.mem) == EMPTY;
    if sub_empty {
        let _ = maybe_attach_script_kern_in(
            st,
            q,
            p,
            superscript.font,
            superscript.glyph,
            SUP_CMD,
            shift_up,
        )?;
        x.set_shift_amount(&mut st.mem, Scaled(-shift_up));
        append_script_box_in(st, q, x);
        return Ok(());
    }

    let save_f = st.scanner.cur_f;
    let PreparedScript {
        box_node: y,
        glyph: sub_g,
        font: sub_f,
        shift,
    } = prepare_subscript_in(st, q, ScriptBoxAdjustment::Sub2, shift_down, Some(save_f))?;
    shift_down = shift;

    let cur_f = st.scanner.cur_f;
    let x_height_node = x.depth(&st.mem).0;
    let y_depth = y.height(&st.mem).0;
    let default_rule_thickness = default_rule_thickness(&st.fonts, &st.eqtb, st.scanner.cur_size);
    let mut clr = if font_is_ot_math(&st.fonts, cur_f) {
        get_ot_math_constant(cur_f, SUBSUPERSCRIPTGAPMIN)
            - ((shift_up - x_height_node) - (y_depth - shift_down))
    } else {
        4 * default_rule_thickness - ((shift_up - x_height_node) - (y_depth - shift_down))
    };

    let mut shift_up = shift_up;
    if clr > 0 {
        shift_down += clr;
        let cur_f = st.scanner.cur_f;
        let x_height_node = x.depth(&st.mem).0;
        let x_height = math_x_height(&st.fonts, &st.eqtb, st.scanner.cur_size);
        clr = if font_is_ot_math(&st.fonts, cur_f) {
            get_ot_math_constant(cur_f, SUPERSCRIPTBOTTOMMAXWITHSUBSCRIPT)
                - (shift_up - x_height_node)
        } else {
            (x_height * 4).abs() / 5 - (shift_up - x_height_node)
        };
        if clr > 0 {
            shift_up += clr;
            shift_down -= clr;
        }
    }

    finish_subsup_stack_in(
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

// translated from xetex-math.c:805 (repeated script glyph discovery block)
fn first_script_glyph_step(
    mem: &MemState,
    script_head: i32,
    initial_math_style: i16,
) -> (i32, i16) {
    let mut script_head = script_head;
    let mut this_math_style = initial_math_style;

    if mem.b32s1(script_head) == SUB_MLIST {
        let mut script_ptr = mem.b32s0(script_head);
        script_head = TEX_NULL;

        while script_ptr >= 0 && script_ptr <= mem.mem_end {
            match NodeType::from_raw(mem.b16s1(script_ptr)) {
                Ok(NodeType::Kern) | Ok(NodeType::Glue) | Ok(NodeType::Choice) => {}
                Ok(NodeType::Style) => {
                    this_math_style = mem.b16s0(script_ptr) as i16;
                }
                Ok(NodeType::OrdNoad)
                | Ok(NodeType::OpNoad)
                | Ok(NodeType::BinNoad)
                | Ok(NodeType::RelNoad)
                | Ok(NodeType::OpenNoad)
                | Ok(NodeType::CloseNoad)
                | Ok(NodeType::PunctNoad) => {
                    script_head = script_ptr + 1;
                    script_ptr = TEX_NULL;
                }
                _ => {
                    script_ptr = TEX_NULL;
                }
            }

            if script_ptr >= 0 && script_ptr <= mem.mem_end {
                if mem.b16s1(script_ptr) == NodeType::Choice.as_raw() {
                    // SAFETY: the `NODE_type == CHOICE_NODE` check proves the kind.
                    let cn = unsafe {
                        NodePtr::<ChoiceNode>::from_idx_unchecked(MemIdx::new(script_ptr))
                    };
                    script_ptr = match this_math_style / 2 {
                        0 => cn.display_mlist(mem).0,
                        1 => cn.text_mlist(mem).0,
                        2 => cn.script_mlist(mem).0,
                        3 => cn.script_script_mlist(mem).0,
                        _ => script_ptr,
                    };
                } else {
                    script_ptr = mem.b32s1(script_ptr);
                }
            }
        }
    }

    (script_head, this_math_style)
}

unsafe fn first_script_glyph_in(
    st: &mut EngineState,
    script_head: i32,
) -> Result<(u16, i32), MathBuildAbort> {
    use crate::errors::ErrorOutcome;
    let mut script_g: u16 = 0;
    let mut script_f: i32 = 0;

    let initial_math_style = 2 * (st.scanner.cur_style / 4) + 5;
    let (script_head, this_math_style) =
        first_script_glyph_step(&st.mem, script_head, initial_math_style);
    let is_math_char =
        script_head >= 0 && script_head <= st.mem.mem_end && st.mem.b32s1(script_head) == MATH_CHAR;

    if is_math_char {
        let save_f = st.scanner.cur_f;
        let saved_math_style = st.scanner.cur_style;
        let cur_size_for_script = if this_math_style < SCRIPT_STYLE {
            TEXT_SIZE
        } else {
            SCRIPT_SIZE * ((this_math_style as i32 - 2) / 2)
        };
        let mq = math_quad(&st.fonts, &st.eqtb, cur_size_for_script);
        let script_mu = crate::scaled_math::x_over_n_in(&mut st.errors, mq, 18);
        st.scanner.cur_style = this_math_style;
        st.scanner.cur_size = cur_size_for_script;
        st.scanner.cur_mu = script_mu;
        match fetch_in(st, script_head) {
            ErrorOutcome::Continue => {}
            other => return Err(MathBuildAbort::Error(other)),
        }

        let cur_f_after_fetch = st.scanner.cur_f;
        let cur_c_after_fetch = st.scanner.cur_c;
        if font_is_ot_math(&st.fonts, cur_f_after_fetch) {
            let script_c = crate::fonts::new_native_character_in(
                st,
                cur_f_after_fetch,
                cur_c_after_fetch as u32,
            )?;
            let addr = node_addr_in(st, script_c);
            script_g = real_get_native_glyph(addr, 0);
            script_f = cur_f_after_fetch;
        }

        let restored_size = if saved_math_style < SCRIPT_STYLE {
            TEXT_SIZE
        } else {
            SCRIPT_SIZE * ((saved_math_style as i32 - 2) / 2)
        };
        let mq2 = math_quad(&st.fonts, &st.eqtb, restored_size);
        let restored_mu = crate::scaled_math::x_over_n_in(&mut st.errors, mq2, 18);
        st.scanner.cur_f = save_f;
        st.scanner.cur_style = saved_math_style;
        st.scanner.cur_size = restored_size;
        st.scanner.cur_mu = restored_mu;
    }

    Ok((script_g, script_f))
}

/// translated from xetex-math.c:84 (make_scripts)
unsafe fn make_scripts_in(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    delta: i32,
) -> Result<(), MathBuildAbort> {
    let p = q.nucleus_type(&st.mem);
    let shifts = initial_script_shifts_in(st, p)?;

    if q.supscr_type(&st.mem) == EMPTY {
        finish_subscript_only_in(st, q, p, shifts.down)?;
    } else {
        let superscript = prepare_superscript_in(st, q, shifts.up)?;
        finish_superscript_branch_in(st, q, p, superscript, shifts.down, delta)?;
    }
    Ok(())
}

/// translated from xetex-math.c:759 (make_left_right)
unsafe fn make_left_right_in(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    style: i16,
    max_d: i32,
    max_h: i32,
) -> Result<i16, MathBuildAbort> {
    let q_raw = q.idx().0;
    let size = if style < SCRIPT_STYLE {
        TEXT_SIZE
    } else {
        SCRIPT_SIZE * ((style as i32 - 2) / 2)
    };
    let math_quad = math_quad(&st.fonts, &st.eqtb, size);
    let mu = crate::scaled_math::x_over_n_in(&mut st.errors, math_quad, 18);
    st.scanner.cur_style = style;
    st.scanner.cur_size = size;
    st.scanner.cur_mu = mu;

    let axis_height = axis_height(&st.fonts, &st.eqtb, size);
    let delimiter_factor = st.eqtb.b32s1(INT_BASE + INT_PAR_DELIMITER_FACTOR);
    let delimiter_shortfall = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_DELIMITER_SHORTFALL);
    let mut delta2 = max_d + axis_height;
    let mut delta1 = max_h + max_d - delta2;
    if delta2 > delta1 {
        delta1 = delta2;
    }
    let mut delta = (delta1 / 500) * delimiter_factor;
    delta2 = delta1 + delta1 - delimiter_shortfall;
    if delta < delta2 {
        delta = delta2;
    }
    let delimiter = var_delimiter_in(st, q_raw + 1, size, delta)?;
    q.set_nucleus_type(&mut st.mem, delimiter.idx().0);
    Ok((q.noad_type(&st.mem).as_raw() as i32 - (NodeType::LeftNoad.as_raw() as i32 - 20)) as i16)
}

/// Threaded equivalent of `update_math_style_params` (callable under the held
/// `&mut EngineState` borrow of the `mlist_to_hlist` SCC).
#[inline]
fn update_math_style_params_in(st: &mut EngineState) {
    if st.scanner.cur_style < SCRIPT_STYLE {
        st.scanner.cur_size = TEXT_SIZE;
    } else {
        st.scanner.cur_size = SCRIPT_SIZE * ((st.scanner.cur_style as i32 - 2) / 2);
    }
    let mq = math_quad(&st.fonts, &st.eqtb, st.scanner.cur_size);
    st.scanner.cur_mu = crate::scaled_math::x_over_n_in(&mut st.errors, mq, 18);
}

/// First-pass `CHOICE_NODE` handling for [`mlist_to_hlist`]: keep the branch
/// matching the current style, flush the other three, then stamp the node down
/// to a style node and splice the kept branch into the main list after it.
/// Returns any `flush_node_list` confusion message for the caller to raise
/// after the `mem` borrow drops (`01b` §4a bucket C).
fn mlist_choice_node(
    mem: &mut MemState,
    choice: NodePtr<ChoiceNode>,
    cur_style: i16,
) -> Result<(), &'static std::ffi::CStr> {
    let q = choice.idx().0;
    // Detach the branch for the current style; the other three are flushed.
    let kept = match cur_style / 2 {
        0 => {
            let p = choice.display_mlist(mem);
            choice.set_display_mlist(mem, MemPtr::NULL);
            p
        }
        1 => {
            let p = choice.text_mlist(mem);
            choice.set_text_mlist(mem, MemPtr::NULL);
            p
        }
        2 => {
            let p = choice.script_mlist(mem);
            choice.set_script_mlist(mem, MemPtr::NULL);
            p
        }
        3 => {
            let p = choice.script_script_mlist(mem);
            choice.set_script_script_mlist(mem, MemPtr::NULL);
            p
        }
        _ => MemPtr::NULL,
    };
    crate::packaging::flush_node_list(mem, choice.display_mlist(mem).0)?;
    crate::packaging::flush_node_list(mem, choice.text_mlist(mem).0)?;
    crate::packaging::flush_node_list(mem, choice.script_mlist(mem).0)?;
    crate::packaging::flush_node_list(mem, choice.script_script_mlist(mem).0)?;
    // Rewrite the node as a style node carrying the current style.
    mem.set_b16s1(q, NodeType::Style.as_raw());
    mem.set_b16s0(q, cur_style as u16);
    mem.set_b32s1(q + 1, 0);
    mem.set_b32s1(q + 2, 0);
    if let Some(kept) = kept.as_idx() {
        let z = mem.b32s1(q);
        mem.set_b32s1(q, kept.0);
        let mut p = kept.0;
        while mem.b32s1(p) != TEX_NULL {
            p = mem.b32s1(p);
        }
        mem.set_b32s1(p, z);
    }
    Ok(())
}

fn mlist_track_rule(mem: &MemState, rule: NodePtr<RuleNode>, max_h: &mut i32, max_d: &mut i32) {
    let h = rule.height(mem).raw();
    let d = rule.depth(mem).raw();
    if h > *max_h {
        *max_h = h;
    }
    if d > *max_d {
        *max_d = d;
    }
}

/// Record the hpacked box `z`'s height/depth into the running maxima and
/// recycle it (one `mem` borrow; `hpack` itself runs before, bucket C).
fn mlist_track_box(mem: &mut MemState, z: AnyBox, max_h: &mut i32, max_d: &mut i32) {
    let h = z.height(mem).0;
    let d = z.depth(mem).0;
    if h > *max_h {
        *max_h = h;
    }
    if d > *max_d {
        *max_d = d;
    }
    crate::mem::free_node(mem, z.idx().0, BOX_NODE_SIZE);
}

fn mlist_cond_math_glue(
    mem: &mut MemState,
    glue: NodePtr<GlueNode>,
    cur_size: i32,
) -> Result<(), &'static std::ffi::CStr> {
    if cur_size == TEXT_SIZE {
        return Ok(());
    }

    let p = glue.link(mem);
    let Some(p_idx) = p.as_idx() else {
        return Ok(());
    };
    match mem.classify(p_idx).expect("valid node type") {
        AnyNode::Glue(_) | AnyNode::Kern(_) => {
            glue.set_link(mem, MemPtr::from_raw(mem.b32s1(p_idx.0)));
            mem.set_b32s1(p_idx.0, TEX_NULL);
            crate::packaging::flush_node_list(mem, p_idx.0)?;
        }
        _ => {}
    }
    Ok(())
}

fn mlist_move_output_node(mem: &mut MemState, tail: i32, node: AnyNode) -> (i32, i32) {
    let q = node.idx().0;
    mem.set_b32s1(tail, q);
    let next = mem.b32s1(q);
    mem.set_b32s1(q, TEX_NULL);
    (q, next)
}

/// First-pass noad advance: remember the current noad as `r` with its type,
/// reset to the outer `style` after a right delimiter, and step `q` to the next
/// node. Returns the new `(r, r_type, q)`. (`r_type` and `q`'s link are read in
/// one borrow; the intervening style reset does not touch `mem[q].link`.)
fn mlist_advance_in(st: &mut EngineState, q: i32, style: i16) -> (i32, u16, i32) {
    let r = q;
    let mut r_type = st.mem.b16s1(r);
    let next = st.mem.b32s1(q);
    if r_type == NodeType::RightNoad.as_raw() {
        r_type = NodeType::LeftNoad.as_raw();
        st.scanner.cur_style = style;
        update_math_style_params_in(st);
    }
    (r, r_type, next)
}

/// translated from xetex-math.c:84 (mlist_to_hlist)
/// translated from xetex-math.c:84 (mlist_to_hlist). Thin wrapper for the
/// non-SCC callers (`after_math`/`init_math`/`app_display`): runs the threaded
/// Threaded SCC core: converts `cur_mlist` to an hlist on `TEMP_HEAD` under a
/// single `&mut EngineState` borrow, recursing through `clean_box_in`/`make_*_in`
/// (the OT-math FFI is borrow-safe, session 207).
unsafe fn mlist_to_hlist_in(st: &mut EngineState) -> Result<(), MathBuildAbort> {
    use crate::errors::ErrorOutcome;
    let mlist = st.scanner.cur_mlist;
    let penalties = st.scanner.mlist_penalties;
    let style = st.scanner.cur_style;
    let mut q = mlist;
    let mut r = TEX_NULL;
    let mut r_type = NodeType::OpNoad.as_raw();
    let mut max_h = 0;
    let mut max_d = 0;

    update_math_style_params_in(st);

    enum FirstPass {
        ProcessNucleus,
        CheckDimensions,
        DoneWithNoad,
        DoneWithNode,
    }

    while q != TEX_NULL {
        let mut delta = 0;
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
                        break 'reswitch FirstPass::ProcessNucleus;
                    }
                    nt @ (NodeType::RelNoad
                    | NodeType::CloseNoad
                    | NodeType::PunctNoad
                    | NodeType::RightNoad) => {
                        if r_type == NodeType::BinNoad.as_raw() {
                            assume_noad(r).set_noad_type(&mut st.mem, NodeType::OrdNoad);
                        }
                        if nt == NodeType::RightNoad {
                            break 'reswitch FirstPass::DoneWithNoad;
                        }
                        break 'reswitch FirstPass::ProcessNucleus;
                    }
                    NodeType::LeftNoad => break 'reswitch FirstPass::DoneWithNoad,
                    NodeType::FractionNoad => {
                        make_fraction_in(st, qn)?;
                        break 'reswitch FirstPass::CheckDimensions;
                    }
                    NodeType::OpNoad => {
                        delta = make_op_in(st, qn)?;
                        if st.mem.b16s0(q) == LIMITS {
                            break 'reswitch FirstPass::CheckDimensions;
                        }
                        break 'reswitch FirstPass::ProcessNucleus;
                    }
                    NodeType::OrdNoad => {
                        make_ord_in(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus;
                    }
                    NodeType::OpenNoad | NodeType::InnerNoad => {
                        break 'reswitch FirstPass::ProcessNucleus
                    }
                    NodeType::RadicalNoad => {
                        make_radical_in(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus;
                    }
                    NodeType::OverNoad => {
                        make_over_in(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus;
                    }
                    NodeType::UnderNoad => {
                        make_under_in(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus;
                    }
                    NodeType::AccentNoad => {
                        make_math_accent_in(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus;
                    }
                    NodeType::VcenterNoad => {
                        make_vcenter_in(st, qn)?;
                        break 'reswitch FirstPass::ProcessNucleus;
                    }
                    _ => unreachable!("AnyNode::Noad always has a noad type (16..=31)"),
                },
                Ok(AnyNode::Style(_)) => {
                    st.scanner.cur_style = st.mem.b16s0(q) as i16;
                    update_math_style_params_in(st);
                    break 'reswitch FirstPass::DoneWithNode;
                }
                Ok(AnyNode::Choice(choice)) => {
                    let cs = st.scanner.cur_style;
                    mlist_choice_node(&mut st.mem, choice, cs)
                        .map_err(NodeRecycleAbort::Confusion)?;
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
                    mlist_track_rule(&st.mem, rule, &mut max_h, &mut max_d);
                    break 'reswitch FirstPass::DoneWithNode;
                }
                Ok(AnyNode::Glue(glue)) => {
                    let subtype = glue.subtype(&st.mem);
                    let mu = st.scanner.cur_mu;
                    let x = glue.glue_ptr(&st.mem).0;
                    if subtype == MU_GLUE {
                        let y = math_glue_in(st, x, mu)?;
                        crate::mem::delete_glue_ref(&mut st.mem, x);
                        glue.set_glue_ptr(&mut st.mem, MemPtr::from_raw(y));
                        glue.set_subtype(&mut st.mem, NORMAL);
                    } else if subtype == COND_MATH_GLUE {
                        let cur_size = st.scanner.cur_size;
                        mlist_cond_math_glue(&mut st.mem, glue, cur_size)
                            .map_err(NodeRecycleAbort::Confusion)?;
                    }
                    break 'reswitch FirstPass::DoneWithNode;
                }
                Ok(AnyNode::Kern(kern)) => {
                    let mu = st.scanner.cur_mu;
                    math_kern_in(st, kern, mu);
                    break 'reswitch FirstPass::DoneWithNode;
                }
                _ => return Err(NodeRecycleAbort::Confusion(c"mlist1").into()),
            }
        };

        match action {
            FirstPass::ProcessNucleus => {
                let qn = assume_noad(q);
                let p = match qn.nucleus_type(&st.mem) {
                    MATH_CHAR | MATH_TEXT_CHAR => {
                        match fetch_in(st, q + 1) {
                            ErrorOutcome::Continue => {}
                            other => return Err(MathBuildAbort::Error(other)),
                        }
                        let p;
                        let cur_f = st.scanner.cur_f;
                        let area = st.fonts.font_area_entry(cur_f);
                        if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
                            let cc = st.scanner.cur_c as u32;
                            let z = crate::fonts::new_native_character_in(st, cur_f, cc)?;
                            let p_w = alloc_glyph_whatsit(&mut st.mem)?;
                            p = p_w.idx().0;
                            p_w.set_native_font(&mut st.mem, cur_f as u16);
                            let zaddr = node_addr_in(st, z);
                            p_w.set_native_glyph(&mut st.mem, real_get_native_glyph(zaddr, 0));
                            let paddr = node_addr_in(st, p);
                            crate::ext::measure_native_glyph(&st.fonts, paddr, 1)?;
                            let z_w = st
                                .mem
                                .classify_whatsit(MemIdx::new(z))
                                .expect("valid whatsit")
                                .expect("native char node");
                            let zsize = z_w.native_size(&st.mem);
                            crate::mem::free_node(&mut st.mem, z, zsize);
                            delta = get_ot_math_ital_corr(cur_f, p_w.native_glyph(&st.mem) as i32);
                            if qn.nucleus_type(&st.mem) == MATH_TEXT_CHAR
                                && !font_is_ot_math(&st.fonts, cur_f)
                            {
                                delta = 0;
                            }
                            if qn.subscr_type(&st.mem) == EMPTY && delta != 0 {
                                let k = new_kern(&mut st.mem, delta);
                                if k == ALLOC_FAILED {
                                    return Err(NodeRecycleAbort::Overflow.into());
                                }
                                st.mem.set_b32s1(p, k);
                                delta = 0;
                            }
                        } else if st.scanner.cur_i.s3 > 0 {
                            delta = st.fonts.font_info_b32s1(
                                st.fonts.italic_base_entry(cur_f) + st.scanner.cur_i.s1 as i32 / 4,
                            );
                            let cc = st.scanner.cur_c as u16;
                            p = crate::main_control::new_character_in(st, cur_f, cc)?;
                            if qn.nucleus_type(&st.mem) == MATH_TEXT_CHAR
                                && st
                                    .fonts
                                    .font_info_b32s1(SPACE_CODE + st.fonts.param_base(cur_f))
                                    != 0
                            {
                                delta = 0;
                            }
                            if qn.subscr_type(&st.mem) == EMPTY && delta != 0 {
                                let k = new_kern(&mut st.mem, delta);
                                if k == ALLOC_FAILED {
                                    return Err(NodeRecycleAbort::Overflow.into());
                                }
                                st.mem.set_b32s1(p, k);
                                delta = 0;
                            }
                        } else {
                            p = TEX_NULL;
                        }
                        p
                    }
                    EMPTY => TEX_NULL,
                    SUB_BOX => qn.nucleus_field(&st.mem),
                    SUB_MLIST => {
                        st.scanner.cur_mlist = qn.nucleus_field(&st.mem);
                        let save_style = st.scanner.cur_style;
                        st.scanner.mlist_penalties = false;
                        mlist_to_hlist_in(st)?;
                        st.scanner.cur_style = save_style;
                        update_math_style_params_in(st);
                        let list = st.mem.b32s1(TEMP_HEAD);
                        crate::packaging::hpack_emit_in(st, list, 0, ADDITIONAL)?
                            .idx()
                            .0
                    }
                    _ => return Err(NodeRecycleAbort::Confusion(c"mlist2").into()),
                };
                st.mem.set_b32s1(q + 1, p);
                let need_scripts =
                    qn.subscr_type(&st.mem) != EMPTY || qn.supscr_type(&st.mem) != EMPTY;
                if need_scripts {
                    make_scripts_in(st, assume_noad(q), delta)?;
                }

                let list = st.mem.b32s1(q + 1);
                let z_box = crate::packaging::hpack_emit_in(st, list, 0, ADDITIONAL)?;
                mlist_track_box(&mut st.mem, z_box, &mut max_h, &mut max_d);
                (r, r_type, q) = mlist_advance_in(st, q, style);
            }
            FirstPass::CheckDimensions => {
                let list = st.mem.b32s1(q + 1);
                let z_box = crate::packaging::hpack_emit_in(st, list, 0, ADDITIONAL)?;
                mlist_track_box(&mut st.mem, z_box, &mut max_h, &mut max_d);
                (r, r_type, q) = mlist_advance_in(st, q, style);
            }
            FirstPass::DoneWithNoad => {
                (r, r_type, q) = mlist_advance_in(st, q, style);
            }
            FirstPass::DoneWithNode => {
                q = st.mem.b32s1(q);
            }
        }
    }

    let mut p = TEMP_HEAD;
    if r_type == NodeType::BinNoad.as_raw() {
        assume_noad(r).set_noad_type(&mut st.mem, NodeType::OrdNoad);
    }
    st.mem.set_b32s1(p, TEX_NULL);
    q = mlist;
    r_type = 0;
    st.scanner.cur_style = style;
    update_math_style_params_in(st);

    while q != TEX_NULL {
        let mut t = NodeType::OrdNoad.as_raw();
        let mut s = NOAD_SIZE;
        let mut pen = INF_PENALTY;
        let delete_q = match NodeType::from_raw(st.mem.b16s1(q)) {
            Ok(NodeType::OpNoad)
            | Ok(NodeType::OpenNoad)
            | Ok(NodeType::CloseNoad)
            | Ok(NodeType::PunctNoad)
            | Ok(NodeType::InnerNoad) => {
                t = st.mem.b16s1(q);
                true
            }
            Ok(NodeType::BinNoad) => {
                t = NodeType::BinNoad.as_raw();
                pen = st.eqtb.b32s1(INT_BASE + INT_PAR_BIN_OP_PENALTY);
                true
            }
            Ok(NodeType::RelNoad) => {
                t = NodeType::RelNoad.as_raw();
                pen = st.eqtb.b32s1(INT_BASE + INT_PAR_REL_PENALTY);
                true
            }
            Ok(NodeType::OrdNoad)
            | Ok(NodeType::VcenterNoad)
            | Ok(NodeType::OverNoad)
            | Ok(NodeType::UnderNoad) => true,
            Ok(NodeType::RadicalNoad) => {
                s = RADICAL_NOAD_SIZE;
                true
            }
            Ok(NodeType::AccentNoad) => {
                s = ACCENT_NOAD_SIZE;
                true
            }
            Ok(NodeType::FractionNoad) => {
                s = FRACTION_NOAD_SIZE;
                true
            }
            Ok(NodeType::LeftNoad) | Ok(NodeType::RightNoad) => {
                t = make_left_right_in(st, assume_noad(q), style, max_d, max_h)? as u16;
                true
            }
            Ok(NodeType::Style) => {
                st.scanner.cur_style = st.mem.b16s0(q) as i16;
                s = STYLE_NODE_SIZE;
                update_math_style_params_in(st);
                true
            }
            Ok(NodeType::Whatsit)
            | Ok(NodeType::Penalty)
            | Ok(NodeType::Rule)
            | Ok(NodeType::Disc)
            | Ok(NodeType::Adjust)
            | Ok(NodeType::Ins)
            | Ok(NodeType::Mark)
            | Ok(NodeType::Glue)
            | Ok(NodeType::Kern) => {
                // Move this node onto the output list and detach it.
                let node = st.mem.classify(MemIdx::new(q)).expect("valid node type");
                let (np, nq) = mlist_move_output_node(&mut st.mem, p, node);
                p = np;
                q = nq;
                false
            }
            _ => return Err(NodeRecycleAbort::Confusion(c"mlist3").into()),
        };

        if !delete_q {
            continue;
        }

        if st.mem.b16s1(q) != NodeType::Style.as_raw() {
            if r_type > 0 {
                let offset_table = [
                    b"02340001",
                    b"22*40001",
                    b"33**3**3",
                    b"44*04004",
                    b"00*00000",
                    b"02340001",
                    b"11*11111",
                    b"12341011",
                ];
                // The 8×8 inter-noad spacing table is indexed by (noad type − ORD),
                // so this stays deliberate ORD-relative offset arithmetic.
                let ord = NodeType::OrdNoad.as_raw();
                let spacing = offset_table[(r_type - ord) as usize][(t - ord) as usize];
                let x = match spacing {
                    b'0' => 0,
                    b'1' => {
                        if st.scanner.cur_style < SCRIPT_STYLE {
                            GLUE_PAR_THIN_MU_SKIP
                        } else {
                            0
                        }
                    }
                    b'2' => GLUE_PAR_THIN_MU_SKIP,
                    b'3' => {
                        if st.scanner.cur_style < SCRIPT_STYLE {
                            GLUE_PAR_MED_MU_SKIP
                        } else {
                            0
                        }
                    }
                    b'4' => {
                        if st.scanner.cur_style < SCRIPT_STYLE {
                            GLUE_PAR_THICK_MU_SKIP
                        } else {
                            0
                        }
                    }
                    _ => return Err(NodeRecycleAbort::Confusion(c"mlist4").into()),
                };
                if x != 0 {
                    let g = st.eqtb.b32s1(GLUE_BASE + x);
                    let mu = st.scanner.cur_mu;
                    let y = math_glue_in(st, g, mu)?;
                    let z = crate::mem::new_glue(&mut st.mem, y);
                    st.mem.set_b32s1(y, TEX_NULL);
                    st.mem.set_b32s1(p, z);
                    st.mem.set_b16s0(z, (x + 1) as u16);
                    p = z;
                }
            }

            {
                let head = st.mem.b32s1(q + 1);
                if head != TEX_NULL {
                    // Splice this noad's new hlist in after `p` and walk to its tail.
                    st.mem.set_b32s1(p, head);
                    let mut tail = head;
                    while st.mem.b32s1(tail) != TEX_NULL {
                        tail = st.mem.b32s1(tail);
                    }
                    p = tail;
                }
            }

            {
                let ql = st.mem.b32s1(q);
                if penalties && ql != TEX_NULL && pen < INF_PENALTY {
                    let next_type = st.mem.b16s1(ql);
                    if next_type != PENALTY_NODE && next_type != NodeType::RelNoad.as_raw() {
                        let z = crate::mem::new_penalty(&mut st.mem, pen);
                        st.mem.set_b32s1(p, z);
                        p = z;
                    }
                }
            }

            if st.mem.b16s1(q) == NodeType::RightNoad.as_raw() {
                t = NodeType::OpenNoad.as_raw();
            }
            r_type = t;
        }

        r = q;
        let nq = st.mem.b32s1(r);
        crate::mem::free_node(&mut st.mem, r, s);
        q = nq;
    }
    Ok(())
}

/// Precomputed (outside the `mem` borrow) inputs for the threaded [`char_box`].
/// Both the native-character allocation and the TFM font-table reads are
/// `with_state`-bearing, so [`char_box_global`] performs them before borrowing
/// (bucket C, "precompute font reads outside the mem borrow").
enum CharBoxInput {
    /// Native (AAT/OTGR) font: `p` is the `new_native_character` node.
    Native { p: i32 },
    /// TFM font: precomputed width/height/depth and the (char, font) pair.
    Tfm {
        width: i32,
        height: i32,
        depth: i32,
        c: u16,
        f: i32,
    },
}

/// translated from xetex-math.c:1432 (char_box)
///
/// Phase-C: threaded to `&mut MemState`. The `with_state`-bearing work — the
/// native-character allocation and the TFM font-table reads — is precomputed
/// into [`CharBoxInput`] by [`char_box_global`] before the borrow. Allocation
/// failure propagates the [`NodeRecycleAbort`] sentinel (`01b` §5).
///
/// Note: on the native branch the `new_native_character` node is now allocated
/// (in the wrapper) *before* the box, the reverse of the C order, because that
/// allocation is `with_state`-bearing; only absolute `mem` addresses differ,
/// which no output (`.xdv`/`.log`) or `mem`-image (TRIP uses TFM fonts only)
/// path observes.
fn char_box(mem: &mut MemState, input: CharBoxInput) -> Result<AnyBox, NodeRecycleAbort> {
    let bb = AnyBox::HList(alloc_null_box(mem)?);
    let p = match input {
        CharBoxInput::Native { p } => {
            let pw = mem
                .classify_whatsit(MemIdx::new(p))
                .expect("valid whatsit")
                .expect("native char node");
            let h = pw.height(mem).0;
            let w = pw.width(mem).0;
            let d = pw.depth(mem).0;
            bb.set_height(mem, Scaled(h));
            bb.set_width(mem, Scaled(w));
            bb.set_depth(mem, Scaled(if d < 0 { 0 } else { d }));
            p
        }
        CharBoxInput::Tfm {
            width,
            height,
            depth,
            c,
            f,
        } => {
            bb.set_width(mem, Scaled(width));
            bb.set_height(mem, Scaled(height));
            bb.set_depth(mem, Scaled(depth));
            let p = get_avail(mem);
            if p == ALLOC_FAILED {
                return Err(NodeRecycleAbort::OverflowRunaway);
            }
            mem.set_b16s0(p, c);
            mem.set_b16s1(p, f as u16);
            p
        }
    };
    bb.set_list_ptr(mem, MemPtr::from_raw(p));
    Ok(bb)
}

/// Unified cold-abort sentinel for the threaded math builders (`01b` §5): a
/// fused-borrow `_in` core may hit either an arena-recycle overflow
/// ([`NodeRecycleAbort`]) or the native-character build path
/// ([`NativeCharAbort`]). The wrapper fires the appropriate divergent tail
/// after the state borrow drops.
pub(crate) enum MathBuildAbort {
    Recycle(NodeRecycleAbort),
    NativeChar(crate::fonts::NativeCharAbort),
    /// A recoverable `error()` (from `fetch_in`) went terminal under the borrow.
    Error(crate::errors::ErrorOutcome),
    /// A native-font measurement cold abort (`01d` §5.6).
    NativeFont(crate::ext::NativeFontAbort),
}

impl From<crate::ext::NativeFontAbort> for MathBuildAbort {
    fn from(e: crate::ext::NativeFontAbort) -> Self {
        MathBuildAbort::NativeFont(e)
    }
}

impl From<NodeRecycleAbort> for MathBuildAbort {
    fn from(e: NodeRecycleAbort) -> Self {
        MathBuildAbort::Recycle(e)
    }
}

impl From<crate::fonts::NativeCharAbort> for MathBuildAbort {
    fn from(a: crate::fonts::NativeCharAbort) -> Self {
        MathBuildAbort::NativeChar(a)
    }
}

impl From<MakeOrdAbort> for MathBuildAbort {
    fn from(a: MakeOrdAbort) -> Self {
        match a {
            MakeOrdAbort::Overflow => MathBuildAbort::Recycle(NodeRecycleAbort::Overflow),
            MakeOrdAbort::Error(o) => MathBuildAbort::Error(o),
        }
    }
}

impl From<crate::main_control::NewCharacterAbort> for MathBuildAbort {
    fn from(a: crate::main_control::NewCharacterAbort) -> Self {
        use crate::main_control::NewCharacterAbort as N;
        match a {
            N::Native(x) => MathBuildAbort::NativeChar(x),
            N::MainMemoryOverflow => MathBuildAbort::Recycle(NodeRecycleAbort::Overflow),
            N::Error(o) => MathBuildAbort::Error(o),
        }
    }
}

fn math_build_abort_main(a: MathBuildAbort) -> crate::main_control::MainControlAbort {
    match a {
        MathBuildAbort::Recycle(e) => crate::main_control::MainControlAbort::NodeRecycle(e),
        MathBuildAbort::NativeChar(e) => crate::main_control::MainControlAbort::NewCharacter(
            crate::main_control::NewCharacterAbort::Native(e),
        ),
        MathBuildAbort::Error(o) => crate::main_control::MainControlAbort::Error(o),
        MathBuildAbort::NativeFont(e) => crate::main_control::MainControlAbort::NativeFont(e),
    }
}

/// Threaded core (`01b` §3a): build the char box for `f`/`c` under one borrow.
/// The native path's `new_native_character_in` and the TFM metric reads are all
/// `_in`/`&FontState` (no `with_state` re-entry), so the whole build holds the
/// caller's `&mut EngineState`.
fn char_box_in(st: &mut EngineState, f: i32, c: i32) -> Result<AnyBox, MathBuildAbort> {
    let area = st.fonts.font_area_entry(f);
    // `char_box`/`char_box_tfm` classify the freshly-built box at their creation
    // boundary and return the typed `AnyBox`, so the delimiter/accent callers
    // carry the handle without re-classifying.
    if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
        let p = crate::fonts::new_native_character_in(st, f, c as u32)?;
        let input = CharBoxInput::Native { p };
        Ok(char_box(&mut st.mem, input)?)
    } else {
        Ok(char_box_tfm(&mut st.mem, &mut st.fonts, f, c as u16)?)
    }
}

/// Build a TFM character box under a single `mem`+`fonts` borrow. All the metric
/// reads (`effective_char` + the width/italic/height/depth table lookups) are
/// pure `FontState` accesses with no `with_state`-bearing FFI re-entry, so this
/// is safe to call from inside an existing state borrow (used by
/// [`build_ext_delimiter_box`]).
fn char_box_tfm(
    mem: &mut MemState,
    fonts: &mut FontState,
    f: i32,
    c: u16,
) -> Result<AnyBox, NodeRecycleAbort> {
    let c_eff = crate::fonts::effective_char(fonts, true, f, c);
    let q = fonts.font_info_b16(fonts.char_base_entry(f) + c_eff);
    let input = CharBoxInput::Tfm {
        width: fonts.font_info_b32s1(fonts.width_base_entry(f) + q.s3 as i32)
            + fonts.font_info_b32s1(fonts.italic_base_entry(f) + q.s1 as i32 / 4),
        height: fonts.font_info_b32s1(fonts.height_base_entry(f) + q.s2 as i32 / 16),
        depth: fonts.font_info_b32s1(fonts.depth_base_entry(f) + q.s2 as i32 % 16),
        c,
        f,
    };
    char_box(mem, input)
}

/// translated from xetex-math.c:1463 (stack_into_box)
///
/// Phase-C: threaded to `&mut MemState`; takes the character box `p` already
/// built by [`char_box_tfm`] (in [`build_ext_delimiter_box`]) and links it.
fn stack_into_box(mem: &mut MemState, b_box: AnyBox, p: AnyBox) {
    let link = b_box.list_ptr(mem);
    p.set_link(mem, link);
    b_box.set_list_ptr(mem, MemPtr::from_idx(p.idx()));
    let h = p.height(mem).0;
    b_box.set_height(mem, Scaled(h));
}

/// translated from xetex-math.c:1474 (height_plus_depth)
///
/// Phase-C: threaded to `&mut FontState`.
pub fn height_plus_depth(fonts: &mut FontState, f: i32, c: u16) -> i32 {
    let c = crate::fonts::effective_char(fonts, true, f, c);
    let q = fonts.font_info_b16(fonts.char_base_entry(f) + c);
    fonts.font_info_b32s1(fonts.height_base_entry(f) + q.s2 as i32 / 16)
        + fonts.font_info_b32s1(fonts.depth_base_entry(f) + q.s2 as i32 % 16)
}

/// Allocate + type-stamp a fresh `GLYPH_NODE` whatsit and hand back its typed
/// handle. This is the single audited `from_idx_unchecked` construction site for
/// glyph whatsits (`01c` §8): callers receive a `NodePtr<WhatsitNode>` directly
/// instead of re-deriving one from a raw index via
/// `classify_whatsit().expect().expect()`.
fn alloc_glyph_whatsit(mem: &mut MemState) -> Result<NodePtr<WhatsitNode>, NodeRecycleAbort> {
    let p = get_node(mem, GLYPH_NODE_SIZE);
    if p == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b16s1(p, WHATSIT_NODE);
    mem.set_b16s0(p, WhatsitSubtype::Glyph as u16);
    Ok(unsafe { NodePtr::<WhatsitNode>::from_idx_unchecked(MemIdx::new(p)) })
}

fn new_stack_glyph(
    mem: &mut MemState,
    f: i32,
    g: i32,
) -> Result<NodePtr<WhatsitNode>, NodeRecycleAbort> {
    let pw = alloc_glyph_whatsit(mem)?;
    pw.set_native_font(mem, f as u16);
    pw.set_native_glyph(mem, g as u16);
    Ok(pw)
}

/// translated from xetex-math.c:1482 (stack_glyph_into_box)
///
/// Phase-C: the measured glyph node `p` (a typed `GLYPH_NODE` whatsit handle)
/// is spliced into `b_box` under the caller's `MemState` borrow. The threaded
/// `stack_glyph_into_box_in` allocates the node, drops the borrow for
/// `measure_native_glyph`, then links it here.
pub fn stack_glyph_into_box(mem: &mut MemState, b_box: AnyBox, p: NodePtr<WhatsitNode>) {
    if matches!(b_box, AnyBox::HList(_)) {
        let mut q = b_box.list_ptr(mem).0;
        if q == TEX_NULL {
            b_box.set_list_ptr(mem, MemPtr::from_idx(p.idx()));
        } else {
            while mem.b32s1(q) != TEX_NULL {
                q = mem.b32s1(q);
            }
            mem.set_b32s1(q, p.idx().0);
            let glyph_height = p.height(mem).0;
            if b_box.height(mem).0 < glyph_height {
                b_box.set_height(mem, Scaled(glyph_height));
            }
            let glyph_depth = p.depth(mem).0;
            if b_box.depth(mem).0 < glyph_depth {
                b_box.set_depth(mem, Scaled(glyph_depth));
            }
        }
    } else {
        p.set_link(mem, b_box.list_ptr(mem));
        b_box.set_list_ptr(mem, MemPtr::from_idx(p.idx()));
        let glyph_height = p.height(mem).0;
        b_box.set_height(mem, Scaled(glyph_height));
        let glyph_width = p.width(mem).0;
        if b_box.width(mem).0 < glyph_width {
            b_box.set_width(mem, Scaled(glyph_width));
        }
    }
}

/// Threaded core (`01b` §3a): allocate the glyph node, measure it via the
/// re-entry-free `measure_native_glyph` (`&FontState`, session 187), and splice
/// it — all under the caller's `&mut EngineState`.
fn stack_glyph_into_box_in(
    st: &mut EngineState,
    b: AnyBox,
    f: i32,
    g: i32,
) -> Result<(), NodeRecycleAbort> {
    let p = new_stack_glyph(&mut st.mem, f, g)?;
    let addr = st.mem.data.as_mut_ptr().wrapping_add(p.idx().index()) as *mut std::ffi::c_void;
    // SAFETY: `measure_native_glyph` reads only `&FontState` and writes the
    // freshly-allocated node `p` through `addr`; no `with_state` re-entry.
    unsafe { crate::ext::measure_native_glyph(&st.fonts, addr, 1) }?;
    stack_glyph_into_box(&mut st.mem, b, p);
    Ok(())
}

fn native_glyph_vbox_start(
    mem: &mut MemState,
    f: i32,
    g: u16,
) -> Result<(AnyBox, NodePtr<WhatsitNode>), NodeRecycleAbort> {
    let b_box = AnyBox::VList(alloc_null_vbox(mem)?);
    let p = new_stack_glyph(mem, f, g as i32)?;
    Ok((b_box, p))
}

fn native_glyph_vbox_finish(mem: &mut MemState, b_box: AnyBox, p: NodePtr<WhatsitNode>) {
    b_box.set_list_ptr(mem, MemPtr::from_idx(p.idx()));
    let w = p.width(mem).0;
    let h = p.height(mem).0;
    let d = p.depth(mem).0;
    b_box.set_width(mem, Scaled(w));
    b_box.set_height(mem, Scaled(h));
    b_box.set_depth(mem, Scaled(d));
}

/// Threaded core (`01b` §3a): build the single-glyph vbox under one borrow.
fn native_glyph_vbox_in(st: &mut EngineState, f: i32, g: u16) -> Result<AnyBox, NodeRecycleAbort> {
    let (b, p) = native_glyph_vbox_start(&mut st.mem, f, g)?;
    let addr = st.mem.data.as_mut_ptr().wrapping_add(p.idx().index()) as *mut std::ffi::c_void;
    // SAFETY: see `stack_glyph_into_box_in` — re-entry-free `&FontState` measure.
    unsafe { crate::ext::measure_native_glyph(&st.fonts, addr, 1) }?;
    native_glyph_vbox_finish(&mut st.mem, b, p);
    Ok(b)
}

/// translated from xetex-math.c:1517 (stack_glue_into_box)
///
/// Phase-C: threaded to `&mut MemState`; bottoms out on the threaded
/// [`new_spec`]/[`new_glue`] and propagates their [`ALLOC_FAILED`] sentinel as
/// [`NodeRecycleAbort::Overflow`] (`01b` §5). Legacy callers use
/// [`stack_glue_into_box_global`].
fn stack_glue_into_box(
    mem: &mut MemState,
    b_box: AnyBox,
    min: i32,
    max: i32,
) -> Result<(), NodeRecycleAbort> {
    let q_spec = new_spec(mem, 0);
    if q_spec == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    // `q_spec` is a freshly allocated glue spec; type its width/stretch writes.
    let spec = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(q_spec)).unwrap();
    spec.set_width(mem, Scaled(min));
    spec.set_stretch(mem, Scaled(max - min));
    let p = new_glue(mem, q_spec);
    if p == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    if matches!(b_box, AnyBox::HList(_)) {
        let mut q = b_box.list_ptr(mem).0;
        if q == TEX_NULL {
            b_box.set_list_ptr(mem, MemPtr::from_raw(p));
        } else {
            while mem.b32s1(q) != TEX_NULL {
                q = mem.b32s1(q);
            }
            mem.set_b32s1(q, p);
        }
    } else {
        let link = b_box.list_ptr(mem).0;
        mem.set_b32s1(p, link);
        b_box.set_list_ptr(mem, MemPtr::from_raw(p));
        let pd = mem.b32s1(p + 3);
        b_box.set_height(mem, Scaled(pd));
        let pw = mem.b32s1(p + 1);
        b_box.set_width(mem, Scaled(pw));
    }
    Ok(())
}

/// Threaded core (`01b` §3a): classify `b` and splice the glue under the
/// caller's `&mut EngineState`.
fn stack_glue_into_box_in(
    st: &mut EngineState,
    b: AnyBox,
    min: i32,
    max: i32,
) -> Result<(), NodeRecycleAbort> {
    stack_glue_into_box(&mut st.mem, b, min, max)
}

enum OtMathAccentPosStep {
    FetchMathChar,
    Recurse(NodePtr<Noad>),
    Infinity,
}

fn compute_ot_math_accent_pos_step(mem: &MemState, p: NodePtr<Noad>) -> OtMathAccentPosStep {
    if p.nucleus_type(mem) == MATH_CHAR {
        OtMathAccentPosStep::FetchMathChar
    } else if p.nucleus_type(mem) == SUB_MLIST {
        let r = p.nucleus_field(mem);
        if r != TEX_NULL && mem.b16s1(r) == NodeType::AccentNoad.as_raw() {
            let r_node = unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(r)) };
            OtMathAccentPosStep::Recurse(r_node)
        } else {
            OtMathAccentPosStep::Infinity
        }
    } else {
        OtMathAccentPosStep::Infinity
    }
}

/// translated from xetex-math.c:199 (compute_ot_math_accent_pos). Threaded SCC core.
unsafe fn compute_ot_math_accent_pos_in(
    st: &mut EngineState,
    mut p: i32,
) -> Result<i32, MathBuildAbort> {
    use crate::errors::ErrorOutcome;
    loop {
        let p_node = NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(p));
        match compute_ot_math_accent_pos_step(&st.mem, p_node) {
            OtMathAccentPosStep::FetchMathChar => {
                match fetch_in(st, p + 1) {
                    ErrorOutcome::Continue => {}
                    other => return Err(MathBuildAbort::Error(other)),
                }
                let f = st.scanner.cur_f;
                let c = st.scanner.cur_c;
                let q = crate::fonts::new_native_character_in(st, f, c as u32)?;
                let addr = node_addr_in(st, q);
                let g = real_get_native_glyph(addr, 0);
                return Ok(get_ot_math_accent_pos(f, g as i32));
            }
            OtMathAccentPosStep::Recurse(r) => {
                p = r.idx().0;
            }
            OtMathAccentPosStep::Infinity => return Ok(TEX_INFINITY),
        }
    }
}

/// translated from xetex-math.c:388 (scan_delimiter)
/// The whole delimiter
/// scan under one borrow over the threaded scanner cores; the recoverable
/// "Missing delimiter" `back_error` is printed in order then deferred via
/// [`mc_propagate_error`](crate::main_control::mc_propagate_error).
pub(crate) fn scan_delimiter_in(
    st: &mut EngineState,
    p: i32,
    r: bool,
) -> Result<(), crate::main_control::MainControlAbort> {
    if r {
        if st.scanner.cur_chr == 1 {
            st.scanner.cur_val1 = 0x4000_0000;
            crate::scan::scan_math_fam_int_in(st)?;
            st.scanner.cur_val1 += st.scanner.cur_val * 0x20_0000;
            crate::scan::scan_usv_num_in(st)?;
            st.scanner.cur_val = st.scanner.cur_val + st.scanner.cur_val1;
        } else {
            crate::scan::scan_delimiter_int_in(st)?;
        }
    } else {
        loop {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
                break;
            }
        }

        match st.scanner.cur_cmd {
            LETTER | OTHER_CHAR => st.scanner.cur_val = st.eqtb.b32s1(DEL_CODE_BASE + st.scanner.cur_chr),
            DELIM_NUM => {
                if st.scanner.cur_chr == 1 {
                    st.scanner.cur_val1 = 0x4000_0000;
                    crate::scan::scan_math_class_int_in(st)?;
                    crate::scan::scan_math_fam_int_in(st)?;
                    st.scanner.cur_val1 += st.scanner.cur_val * 0x2_0000;
                    crate::scan::scan_usv_num_in(st)?;
                    st.scanner.cur_val = st.scanner.cur_val + st.scanner.cur_val1;
                } else {
                    crate::scan::scan_delimiter_int_in(st)?;
                }
            }
            _ => st.scanner.cur_val = -1,
        }
    }

    if st.scanner.cur_val < 0 {
        if st.config.file_line_error_style_p != 0 {
            crate::output::print_file_line_in(st);
        } else {
            crate::output::print_nl_cstr_in(st, c"! ".as_ptr());
        }
        crate::output::print_cstr_in(st, c"Missing delimiter (. inserted)".as_ptr());
        st.errors.help_ptr = 6;
        st.errors.help_line[5] = c"I was expecting to see something like `(' or `\\{' or".as_ptr();
        st.errors.help_line[4] =
            c"`\\}' here. If you typed, e.g., `{' instead of `\\{', you".as_ptr();
        st.errors.help_line[3] =
            c"should probably delete the `{' by typing `1' now, so that".as_ptr();
        st.errors.help_line[2] = c"braces don't get unbalanced. Otherwise just proceed.".as_ptr();
        st.errors.help_line[1] =
            c"Acceptable delimiters are characters whose \\delcode is".as_ptr();
        st.errors.help_line[0] =
            c"nonnegative, or you can use `\\delimiter <delimiter code>'.".as_ptr();
        crate::main_control::mc_propagate_error(crate::input::back_error_in(st)?)?;
        st.scanner.cur_val = 0;
    }

    if st.scanner.cur_val >= 0x4000_0000 {
        st.mem.set_b16s3(
            p,
            (((st.scanner.cur_val % 0x20_0000) / 0x1_0000) * 0x100 + (st.scanner.cur_val / 0x20_0000) % 0x100) as u16,
        );
        st.mem.set_b16s2(p, (st.scanner.cur_val % 0x1_0000) as u16);
        st.mem.set_b16s1(p, 0);
        st.mem.set_b16s0(p, 0);
    } else {
        st.mem.set_b16s3(p, ((st.scanner.cur_val / 0x10_0000) % 16) as u16);
        st.mem.set_b16s2(p, ((st.scanner.cur_val / 0x1000) % 0x100) as u16);
        st.mem.set_b16s1(p, ((st.scanner.cur_val / 0x100) % 16) as u16);
        st.mem.set_b16s0(p, (st.scanner.cur_val % 0x100) as u16);
    }
    Ok(())
}

pub(crate) fn start_eq_no_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    use crate::main_control::MainControlAbort;
    st.save_stack.set_b32s1(st.save_stack.save_ptr, st.scanner.cur_chr);
    st.save_stack.save_ptr += 1;

    crate::main_control::push_math_in(st, GroupCode::MathShift)?;
    if let Some(a) = crate::save::eq_word_define(
        &mut st.eqtb,
        &mut st.save_stack,
        INT_BASE + INT_PAR_CUR_FAM,
        -1,
    ) {
        return Err(MainControlAbort::Save(a));
    }
    if st.shipout.insert_src_special_every_math {
        crate::ext::insert_src_special_in(st)?;
    }
    let every_math = st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_MATH);
    if every_math != TEX_NULL {
        crate::input::begin_token_list_in(st, every_math, EVERY_MATH_TEXT)?;
    }
    Ok(())
}

/// translated from xetex-math.c:368 (math_limit_switch)
fn apply_math_limit_switch(cl: &mut crate::mem_types::ListStateRecord, mem: &mut MemState, limit: u16) -> bool {
    let tail = cl.tail;
    if cl.head != tail && mem.b16s1(tail) == NodeType::OpNoad.as_raw() {
        // SAFETY: the `NODE_type == OP_NOAD` check just proved this is an op noad.
        let noad = unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(tail)) };
        noad.set_subtype(mem, limit);
        true
    } else {
        false
    }
}

pub(crate) fn math_limit_switch_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    if apply_math_limit_switch(&mut st.input.cur_list, &mut st.mem, st.scanner.cur_chr as u16) {
        return Ok(());
    }
    crate::output::error_here_with_diagnostic_in(
        st,
        c"Limit controls must follow a math operator".as_ptr(),
    );
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 1;
    st.errors.help_line[0] =
        c"I'm ignoring this misplaced \\limits or \\nolimits command.".as_ptr();
    crate::main_control::mc_propagate_error(crate::errors::error_in(st))
}

/// translated from xetex-math.c:461 (math_radical)
fn append_radical_noad(cl: &mut crate::mem_types::ListStateRecord, mem: &mut MemState) -> Result<NodePtr<Noad>, NodeRecycleAbort> {
    let Some(node) = NodePtr::<Noad>::alloc_radical(mem) else {
        return Err(NodeRecycleAbort::Overflow);
    };
    mem.set_b32s1(cl.tail, node.idx().0);
    cl.tail = node.idx().0;
    node.set_subtype(mem, NORMAL);
    node.set_nucleus_type(mem, EMPTY);
    node.set_nucleus_field(mem, TEX_NULL);
    node.set_supscr_type(mem, EMPTY);
    node.set_supscr_field(mem, TEX_NULL);
    node.set_subscr_type(mem, EMPTY);
    node.set_subscr_field(mem, TEX_NULL);
    Ok(node)
}

pub(crate) fn math_radical_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    let tail = append_radical_noad(&mut st.input.cur_list, &mut st.mem)?;
    scan_delimiter_in(st, tail.delimiter_addr(), true)?;
    crate::scan::scan_math_in(st, tail.idx().0 + 1)
}

/// translated from xetex-math.c:533 (append_choices)
fn append_choices_core(cl: &mut crate::mem_types::ListStateRecord, 
    mem: &mut MemState,
    save_stack: &mut SaveStackState,
) -> Result<(), NodeRecycleAbort> {
    let choice = new_choice(mem);
    if choice == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(cl.tail, choice);
    cl.tail = mem.b32s1(cl.tail);
    save_stack.save_ptr += 1;
    save_stack.set_b32s1(save_stack.save_ptr - 1, 0);
    Ok(())
}

pub(crate) fn append_choices_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    append_choices_core(&mut st.input.cur_list, &mut st.mem, &mut st.save_stack)?;
    crate::main_control::push_math_in(st, GroupCode::MathChoice)?;
    crate::scan::scan_left_brace_in(st)?;
    Ok(())
}

/// translated from xetex-math.c:545 (fin_mlist)
///
/// Phase C (`01b` §4a bucket C): the mem work + raw `cur_list` reads run under a
/// single `&mut MemState` borrow. The impossible corrupt-node `confusion`
/// (`q` not a `LEFT_NOAD`) propagates out as `Err`; the threaded caller then
/// runs `pop_nest_in` under the same `EngineState` borrow.
fn fin_mlist(cl: &mut crate::mem_types::ListStateRecord, mem: &mut MemState, p: i32) -> Result<i32, &'static std::ffi::CStr> {
    let q;
    let aux = cl.aux_s1();
    if aux != TEX_NULL {
        let aux_noad = unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(aux)) };
        aux_noad.set_subscr_type(mem, SUB_MLIST);
        let head_link = mem.b32s1(cl.head);
        aux_noad.set_subscr_field(mem, head_link);
        if p == TEX_NULL {
            q = aux;
        } else {
            q = aux_noad.supscr_field(mem);
            let q_idx = MemIdx::new(q);
            let q_noad = unsafe { NodePtr::<Noad>::from_idx_unchecked(q_idx) };
            let etex_aux = cl.e_tex_aux;
            if q_noad.noad_type(mem) != NodeType::LeftNoad || etex_aux == TEX_NULL {
                return Err(c"right");
            }
            let etex_aux_noad =
                unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(etex_aux)) };
            let etex_link = etex_aux_noad.link(mem);
            aux_noad.set_supscr_field(mem, etex_link.0);
            etex_aux_noad.set_link(mem, MemPtr::from_raw(aux));
            aux_noad.set_link(mem, MemPtr::from_raw(p));
        }
    } else {
        mem.set_b32s1(cl.tail, p);
        q = mem.b32s1(cl.head);
    }
    Ok(q)
}

/// `&mut EngineState` core: runs [`fin_mlist`] then `pop_nest_in` under one
/// borrow, deferring the cold corrupt-node `confusion` via
/// [`MainControlAbort::Confusion`].
pub(crate) fn fin_mlist_in(
    st: &mut crate::state::EngineState,
    p: i32,
) -> Result<i32, crate::main_control::MainControlAbort> {
    let q = fin_mlist(&mut st.input.cur_list, &mut st.mem, p).map_err(crate::main_control::MainControlAbort::Confusion)?;
    crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
    Ok(q)
}

/// translated from xetex-math.c:571 (build_choices)
fn build_choices_finish(cl: &mut crate::mem_types::ListStateRecord, mem: &mut MemState, save_stack: &mut SaveStackState, p: i32) -> bool {
    let tail = unsafe { NodePtr::<ChoiceNode>::from_idx_unchecked(MemIdx::new(cl.tail)) };
    let p_ptr = MemPtr::from_raw(p);
    match save_stack.b32s1(save_stack.save_ptr - 1) {
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
    let choice_state = save_stack.b32s1(save_stack.save_ptr - 1);
    save_stack.set_b32s1(save_stack.save_ptr - 1, choice_state + 1);
    false
}

/// `&mut EngineState` core: closes one math-choice sublist (`\mathchoice`)
/// under one borrow. Mirrors `fin_mlist_in`'s fin_mlist+pop_nest, then either
/// completes or pushes the next choice group.
///
/// translated from xetex-math.c:571 (build_choices)
pub(crate) fn build_choices_in(
    st: &mut crate::state::EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    crate::save::unsave_in(st)?;
    let p = fin_mlist(&mut st.input.cur_list, &mut st.mem, TEX_NULL)
        .map_err(crate::main_control::MainControlAbort::Confusion)?;
    crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
    let complete = build_choices_finish(&mut st.input.cur_list, &mut st.mem, &mut st.save_stack, p);
    if complete {
        return Ok(());
    }
    crate::main_control::push_math_in(st, GroupCode::MathChoice)?;
    crate::scan::scan_left_brace_in(st)?;
    Ok(())
}

/// translated from xetex-math.c:599 (sub_sup)
enum ScriptDuplication {
    Superscript,
    Subscript,
}

fn prepare_sub_sup(cl: &mut crate::mem_types::ListStateRecord, 
    mem: &mut MemState,
    cmd: u8,
) -> Result<(i32, Option<ScriptDuplication>), NodeRecycleAbort> {
    let mut t = EMPTY;
    let mut p = TEX_NULL;

    if cl.tail != cl.head {
        let node_type = mem.b16s1(cl.tail);
        if (NodeType::OrdNoad.as_raw()..NodeType::LeftNoad.as_raw()).contains(&node_type) {
            p = cl.tail + 2 + cmd as i32 - 7;
            t = mem.b32s1(p);
        }
    }

    let mut duplicate = None;
    if p == TEX_NULL || t != EMPTY {
        let noad = new_noad(mem);
        if noad == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        mem.set_b32s1(cl.tail, noad);
        cl.tail = mem.b32s1(cl.tail);
        p = cl.tail + 2 + cmd as i32 - 7;
        if t != EMPTY {
            duplicate = Some(if cmd == SUP_MARK {
                ScriptDuplication::Superscript
            } else {
                ScriptDuplication::Subscript
            });
        }
    }
    Ok((p, duplicate))
}

pub(crate) fn sub_sup_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    let cmd = st.scanner.cur_cmd;
    let (p, duplicate) = prepare_sub_sup(&mut st.input.cur_list, &mut st.mem, cmd)?;
    if let Some(duplicate) = duplicate {
        match duplicate {
            ScriptDuplication::Superscript => {
                crate::output::error_here_with_diagnostic_in(st, c"Double superscript".as_ptr());
                st.errors.help_line[0] = c"I treat `x^1^2' essentially like `x^1{}^2'.".as_ptr();
            }
            ScriptDuplication::Subscript => {
                crate::output::error_here_with_diagnostic_in(st, c"Double subscript".as_ptr());
                st.errors.help_line[0] = c"I treat `x_1_2' essentially like `x_1{}_2'.".as_ptr();
            }
        }
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 1;
        crate::main_control::mc_propagate_error(crate::errors::error_in(st))?;
    }
    crate::scan::scan_math_in(st, p)
}

/// translated from xetex-math.c:644 (math_fraction)
fn start_fraction_noad(cl: &mut crate::mem_types::ListStateRecord, mem: &mut MemState) -> Result<NodePtr<Noad>, NodeRecycleAbort> {
    let Some(fraction) = NodePtr::<Noad>::alloc_fraction(mem) else {
        return Err(NodeRecycleAbort::Overflow);
    };
    cl.set_aux_s1(fraction.idx().0);
    fraction.set_subtype(mem, NORMAL);
    fraction.set_supscr_type(mem, SUB_MLIST);
    fraction.set_supscr_field(mem, mem.b32s1(cl.head));
    fraction.set_subscr_type(mem, EMPTY);
    fraction.set_subscr_field(mem, TEX_NULL);

    let delim_l = fraction.fraction_left_delimiter_addr();
    mem.set_b16s3(delim_l, 0);
    mem.set_b16s2(delim_l, 0);
    mem.set_b16s1(delim_l, 0);
    mem.set_b16s0(delim_l, 0);

    let delim_r = fraction.fraction_right_delimiter_addr();
    mem.set_b16s3(delim_r, 0);
    mem.set_b16s2(delim_r, 0);
    mem.set_b16s1(delim_r, 0);
    mem.set_b16s0(delim_r, 0);

    mem.set_b32s1(cl.head, TEX_NULL);
    cl.tail = cl.head;
    Ok(fraction)
}

fn finish_fraction_rule(cl: &mut crate::mem_types::ListStateRecord, mem: &mut MemState, c: i32, rule_thickness: i32) {
    if let Some(aux) = MemPtr(cl.aux_s1()).as_idx() {
        if let Ok(Some(fraction)) = mem.classify_noad(aux) {
            match c % DELIMITED_CODE {
                ABOVE_CODE => fraction.set_thickness(mem, rule_thickness),
                OVER_CODE => fraction.set_thickness(mem, DEFAULT_CODE),
                ATOP_CODE => fraction.set_thickness(mem, 0),
                _ => {}
            }
        }
    }
}

pub(crate) fn math_fraction_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    let c = st.scanner.cur_chr;

    if st.input.cur_list.aux_s1() != TEX_NULL {
        if c >= DELIMITED_CODE {
            scan_delimiter_in(st, GARBAGE, false)?;
            scan_delimiter_in(st, GARBAGE, false)?;
        }

        if c % DELIMITED_CODE == ABOVE_CODE {
            crate::scan::scan_dimen_in(st, false, false, false)?;
        }

        if st.config.file_line_error_style_p != 0 {
            crate::output::print_file_line_in(st);
        } else {
            crate::output::print_nl_cstr_in(st, c"! ".as_ptr());
        }
        crate::output::print_cstr_in(st, c"Ambiguous; you need another { and }".as_ptr());
        st.errors.help_ptr = 3;
        st.errors.help_line[2] =
            c"I'm ignoring this fraction specification, since I don't".as_ptr();
        st.errors.help_line[1] = c"know whether a construction like `x \\over y \\over z'".as_ptr();
        st.errors.help_line[0] =
            c"means `{x \\over y} \\over z' or `x \\over {y \\over z}'.".as_ptr();
        crate::main_control::mc_propagate_error(crate::errors::error_in(st))?;
    } else {
        let aux = start_fraction_noad(&mut st.input.cur_list, &mut st.mem)?;

        if c >= DELIMITED_CODE {
            scan_delimiter_in(st, aux.fraction_left_delimiter_addr(), false)?;
            scan_delimiter_in(st, aux.fraction_right_delimiter_addr(), false)?;
        }

        let rule_thickness = if c % DELIMITED_CODE == ABOVE_CODE {
            crate::scan::scan_dimen_in(st, false, false, false)?;
            st.scanner.cur_val
        } else {
            0
        };
        finish_fraction_rule(&mut st.input.cur_list, &mut st.mem, c, rule_thickness);
    }
    Ok(())
}

fn append_accent_noad(cl: &mut crate::mem_types::ListStateRecord, mem: &mut MemState) -> Result<NodePtr<Noad>, NodeRecycleAbort> {
    let Some(accent) = NodePtr::<Noad>::alloc_accent(mem) else {
        return Err(NodeRecycleAbort::Overflow);
    };
    mem.set_b32s1(cl.tail, accent.idx().0);
    cl.tail = accent.idx().0;
    accent.set_subtype(mem, NORMAL);
    accent.set_nucleus_type(mem, EMPTY);
    accent.set_nucleus_field(mem, TEX_NULL);
    accent.set_supscr_type(mem, EMPTY);
    accent.set_supscr_field(mem, TEX_NULL);
    accent.set_subscr_type(mem, EMPTY);
    accent.set_subscr_field(mem, TEX_NULL);
    accent.set_accent_char_type(mem, MATH_CHAR);
    Ok(accent)
}

fn finish_accent_math_field(mem: &mut MemState, eqtb: &EqtbState, tail: NodePtr<Noad>, value: i32) {
    tail.set_accent_char_val(mem, (value % 65536) as u16);
    let intpar_cur_fam = eqtb.b32s1(INT_BASE + INT_PAR_CUR_FAM);
    if math_class_field(value) == 7 && (0..NUMBER_MATH_FAMILIES).contains(&intpar_cur_fam) {
        tail.set_accent_char_fam(mem, intpar_cur_fam as u16);
    } else {
        tail.set_accent_char_fam(mem, math_fam_field(value) as u16);
    }
    let current_fam = tail.accent_char_fam(mem);
    tail.set_accent_char_fam(
        mem,
        current_fam + (math_char_field(value) / 65536) as u16 * 256,
    );
}

fn set_accent_kind(mem: &mut MemState, tail: NodePtr<Noad>, kind: u16) {
    tail.set_subtype(mem, kind);
}

/// translated from xetex-math.c:703 (math_left_right)
fn append_left_right_noad(
    mem: &mut MemState,
    node_type: u16,
) -> Result<NodePtr<Noad>, NodeRecycleAbort> {
    let Some(p) = NodePtr::<Noad>::alloc(mem) else {
        return Err(NodeRecycleAbort::Overflow);
    };
    p.set_subtype(mem, NORMAL);
    p.set_nucleus_type(mem, EMPTY);
    p.set_nucleus_field(mem, TEX_NULL);
    p.set_supscr_type(mem, EMPTY);
    p.set_supscr_field(mem, TEX_NULL);
    p.set_subscr_type(mem, EMPTY);
    p.set_subscr_field(mem, TEX_NULL);

    let nt = NodeType::from_raw(node_type).expect("valid noad type");
    p.set_noad_type(mem, nt);
    Ok(p)
}

fn mark_middle_noad(mem: &mut MemState, p: NodePtr<Noad>) {
    p.set_noad_type(mem, NodeType::RightNoad);
    p.set_subtype(mem, 1);
}

fn finish_left_right_group(cl: &mut crate::mem_types::ListStateRecord, mem: &mut MemState, q: i32, p: i32) {
    mem.set_b32s1(cl.head, q);
    cl.tail = p;
    cl.e_tex_aux = p;
}

fn append_inner_noad_for_right(cl: &mut crate::mem_types::ListStateRecord, mem: &mut MemState, q: i32) -> Result<(), NodeRecycleAbort> {
    let Some(inner) = NodePtr::<Noad>::alloc(mem) else {
        return Err(NodeRecycleAbort::Overflow);
    };
    inner.set_subtype(mem, NORMAL);
    inner.set_nucleus_type(mem, EMPTY);
    inner.set_nucleus_field(mem, TEX_NULL);
    inner.set_supscr_type(mem, EMPTY);
    inner.set_supscr_field(mem, TEX_NULL);
    inner.set_subscr_type(mem, EMPTY);
    inner.set_subscr_field(mem, TEX_NULL);

    mem.set_b32s1(cl.tail, inner.idx().0);
    cl.tail = inner.idx().0;
    inner.set_noad_type(mem, NodeType::InnerNoad);
    inner.set_nucleus_type(mem, SUB_MLIST);
    inner.set_nucleus_field(mem, q);
    Ok(())
}

pub(crate) fn math_left_right_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    let t = st.scanner.cur_chr as u16;
    if t != NodeType::LeftNoad.as_raw() && st.save_stack.cur_group != GroupCode::MathLeft {
        if st.save_stack.cur_group == GroupCode::MathShift {
            scan_delimiter_in(st, GARBAGE, false)?;
            crate::output::error_here_with_diagnostic_in(st, c"Extra ".as_ptr());
            if t == 1 {
                crate::output::print_esc_cstr_in(st, c"middle".as_ptr());
                st.errors.help_ptr = 1;
                st.errors.help_line[0] =
                    c"I'm ignoring a \\middle that had no matching \\left.".as_ptr();
            } else {
                crate::output::print_esc_cstr_in(st, c"right".as_ptr());
                st.errors.help_ptr = 1;
                st.errors.help_line[0] =
                    c"I'm ignoring a \\right that had no matching \\left.".as_ptr();
            }
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            crate::main_control::mc_propagate_error(crate::errors::error_in(st))?;
        } else {
            crate::main_control::off_save_in(st)?;
        }
    } else {
        let p = append_left_right_noad(&mut st.mem, t)?;
        scan_delimiter_in(st, p.left_right_delimiter_addr(), false)?;
        if t == 1 {
            mark_middle_noad(&mut st.mem, p);
        }
        let q = if t == NodeType::LeftNoad.as_raw() {
            p.idx().0
        } else {
            let q = fin_mlist_in(st, p.idx().0)?;
            crate::save::unsave_in(st)?;
            q
        };
        if t != NodeType::RightNoad.as_raw() {
            crate::main_control::push_math_in(st, GroupCode::MathLeft)?;
            finish_left_right_group(&mut st.input.cur_list, &mut st.mem, q, p.idx().0);
        } else {
            append_inner_noad_for_right(&mut st.input.cur_list, &mut st.mem, q)?;
        }
    }
    Ok(())
}

// Inline helpers for the C bitfield macros in xetex-xetexd.h.
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

pub(crate) fn math_ac_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    if st.scanner.cur_cmd == ACCENT {
        crate::output::error_here_with_diagnostic_in(st, c"Please use ".as_ptr());
        crate::output::print_esc_cstr_in(st, c"mathaccent".as_ptr());
        crate::output::print_cstr_in(st, c" for accents in math mode".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] =
            c"I'm changing \\accent to \\mathaccent here; wish me luck.".as_ptr();
        st.errors.help_line[0] =
            c"(Accents are not the same in formulas as they are in text.)".as_ptr();
        crate::main_control::mc_propagate_error(crate::errors::error_in(st))?;
    }

    let tail = append_accent_noad(&mut st.input.cur_list, &mut st.mem)?;

    if st.scanner.cur_chr == 1 {
        if crate::scan::scan_keyword_in(st, c"fixed".as_ptr())? {
            set_accent_kind(&mut st.mem, tail, FIXED_ACC);
        } else if crate::scan::scan_keyword_in(st, c"bottom".as_ptr())? {
            if crate::scan::scan_keyword_in(st, c"fixed".as_ptr())? {
                set_accent_kind(&mut st.mem, tail, BOTTOM_ACC + 1);
            } else {
                set_accent_kind(&mut st.mem, tail, BOTTOM_ACC);
            }
        }
        crate::scan::scan_math_class_int_in(st)?;
        let mut c = set_class_bits(st.scanner.cur_val);
        crate::scan::scan_math_fam_int_in(st)?;
        c += set_family_bits(st.scanner.cur_val);
        crate::scan::scan_usv_num_in(st)?;
        st.scanner.cur_val = st.scanner.cur_val + c;
    } else {
        crate::scan::scan_fifteen_bit_int_in(st)?;
        st.scanner.cur_val = set_class_bits(st.scanner.cur_val / 4096)
                + set_family_bits((st.scanner.cur_val % 4096) / 256)
                + (st.scanner.cur_val % 256);
    }

    let value = st.scanner.cur_val;
    finish_accent_math_field(&mut st.mem, &st.eqtb, tail, value);
    crate::scan::scan_math_in(st, tail.idx().0 + 1)
}

pub(crate) fn resume_after_display_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    use crate::main_control::MainControlAbort;

    if st.save_stack.cur_group != GroupCode::MathShift {
        return Err(MainControlAbort::Confusion(c"display"));
    }

    crate::save::unsave_in(st)?;
    st.input.cur_list.prev_graf = st.input.cur_list.prev_graf + 3;
    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = HMODE;
    st.input.cur_list.set_aux_s0(1000);

    let lang = st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE);
    let cl: u8 = if lang <= 0 || lang > BIGGEST_LANG {
        0
    } else {
        lang as u8
    };
    st.hyph.cur_lang = cl;
    st.input.cur_list.set_aux_s1(cl as i32);

    let lhm = crate::helpers::norm_min(st.eqtb.b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as i32;
    let rhm = crate::helpers::norm_min(st.eqtb.b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as i32;
    st.input.cur_list.prev_graf = (lhm * 64 + rhm) * 65536 + cl as i32;

    crate::scan::get_x_token_in(st)?;
    if st.scanner.cur_cmd != SPACER {
        crate::input::back_input_in(st)?;
    }
    if st.input.nest_ptr == 1 {
        crate::pagebuilder::build_page_in(st)?;
    }
    Ok(())
}

#[inline]
fn sym_font_ok(fonts: &FontState, eqtb: &EqtbState, size: i32) -> bool {
    let f = eqtb.b32s1(MATH_FONT_BASE + 2 + size);
    fonts.font_params(f) >= TOTAL_MATHSY_PARAMS || font_is_ot_math(fonts, f)
}

#[inline]
fn ext_font_ok(fonts: &FontState, eqtb: &EqtbState, size: i32) -> bool {
    let f = eqtb.b32s1(MATH_FONT_BASE + 3 + size);
    fonts.font_params(f) >= TOTAL_MATHEX_PARAMS || font_is_ot_math(fonts, f)
}

/// translated from xetex-math.c:82 (app_display) — private helper, threaded core.
fn app_display_in(
    st: &mut EngineState,
    j: i32,
    b_in: i32,
    d_in: i32,
) -> Result<(), crate::main_control::MainControlAbort> {
    use crate::main_control::MainControlAbort;
    let mut b = b_in;
    let mut d = d_in;
    let s = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT);
    let x = st.eqtb.b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_DIRECTION);

    if x == 0 {
        classify_box_or_panic(&st.mem, b).set_shift_amount(&mut st.mem, Scaled(s + d));
    } else {
        let z = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_WIDTH);
        let p_orig = b;
        let p_orig_box = classify_box_or_panic(&st.mem, p_orig);
        let mut e;
        if x > 0 {
            e = z - d - p_orig_box.width(&st.mem).0;
        } else {
            e = d;
            d = z - e - p_orig_box.width(&st.mem).0;
        }
        if j != TEX_NULL {
            b = crate::packaging::copy_node_list(&mut st.mem, j)?;
            let b_copy = classify_box_or_panic(&st.mem, b);
            let (po_h, po_d) = (p_orig_box.height(&st.mem), p_orig_box.depth(&st.mem));
            b_copy.set_height(&mut st.mem, po_h);
            b_copy.set_depth(&mut st.mem, po_d);
            let b4 = b_copy.shift_amount(&st.mem).0;
            let s2 = s - b4;
            d += s2;
            e = e + b_copy.width(&st.mem).0 - z - s2;
        }
        let (mut p, mut q);
        if p_orig_box.subtype(&st.mem) == DLIST {
            q = p_orig;
            p = p_orig;
        } else {
            let r0 = p_orig_box.list_ptr(&st.mem).0;
            crate::mem::free_node(&mut st.mem, p_orig, BOX_NODE_SIZE);
            if r0 == TEX_NULL {
                return Err(MainControlAbort::Confusion(c"LR4"));
            }
            let mut r = r0;
            if x > 0 {
                p = r;
                loop {
                    q = r;
                    r = st.mem.b32s1(r);
                    if r == TEX_NULL {
                        break;
                    }
                }
            } else {
                p = TEX_NULL;
                q = r;
                loop {
                    let t = st.mem.b32s1(r);
                    st.mem.set_b32s1(r, p);
                    p = r;
                    r = t;
                    if r == TEX_NULL {
                        break;
                    }
                }
            }
        }
        let (r, t);
        if j == TEX_NULL {
            r = crate::mem::new_kern(&mut st.mem, 0);
            if r == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            t = crate::mem::new_kern(&mut st.mem, 0);
            if t == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
        } else {
            r = st.mem.b32s1(b + 5);
            t = st.mem.b32s1(r);
        }
        let u = crate::mem::new_math(&mut st.mem, 0, END_M_CODE);
        if u == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        if st.mem.b16s1(t) == GLUE_NODE {
            let jj = crate::mem::new_skip_param(&mut st.mem, &st.eqtb, GLUE_PAR_RIGHT_SKIP as i16);
            if jj == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(q, jj);
            st.mem.set_b32s1(jj, u);
            let jspec = st.mem.b32s0(t + 1);
            st.mem.set_b16s1(st.mem.temp_ptr, st.mem.b16s1(jspec));
            st.mem.set_b16s0(st.mem.temp_ptr, st.mem.b16s0(jspec));
            st.mem
                .set_b32s1(st.mem.temp_ptr + 1, e - st.mem.b32s1(jspec + 1));
            st.mem.set_b32s1(st.mem.temp_ptr + 2, -st.mem.b32s1(jspec + 2));
            st.mem.set_b32s1(st.mem.temp_ptr + 3, -st.mem.b32s1(jspec + 3));
            st.mem.set_b32s1(u, t);
        } else {
            st.mem.set_b32s1(t + 1, e);
            st.mem.set_b32s1(t, u);
            st.mem.set_b32s1(q, t);
        }
        let u2 = crate::mem::new_math(&mut st.mem, 0, BEGIN_M_CODE);
        if u2 == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        if st.mem.b16s1(r) == GLUE_NODE {
            let jj = crate::mem::new_skip_param(&mut st.mem, &st.eqtb, GLUE_PAR_LEFT_SKIP as i16);
            if jj == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(u2, jj);
            st.mem.set_b32s1(jj, p);
            let jspec = st.mem.b32s0(r + 1);
            st.mem.set_b16s1(st.mem.temp_ptr, st.mem.b16s1(jspec));
            st.mem.set_b16s0(st.mem.temp_ptr, st.mem.b16s0(jspec));
            st.mem
                .set_b32s1(st.mem.temp_ptr + 1, d - st.mem.b32s1(jspec + 1));
            st.mem.set_b32s1(st.mem.temp_ptr + 2, -st.mem.b32s1(jspec + 2));
            st.mem.set_b32s1(st.mem.temp_ptr + 3, -st.mem.b32s1(jspec + 3));
            st.mem.set_b32s1(r, u2);
        } else {
            st.mem.set_b32s1(r + 1, d);
            st.mem.set_b32s1(r, p);
            st.mem.set_b32s1(u2, r);
            if j == TEX_NULL {
                b = crate::packaging::hpack_emit_in(st, u2, 0, ADDITIONAL)?
                    .idx()
                    .0;
                st.mem.set_b32s1(b + 4, s);
            } else {
                st.mem.set_b32s1(b + 5, u2);
            }
        }
    }
    crate::packaging::append_to_vlist_emit_in(st, b)?;
    Ok(())
}

/// Emit error for insufficient math fonts, flush the math list, set danger.
/// Threaded core (`01b` §3): font-param checks + recoverable error under one
/// borrow; the recoverable `error` is deferred via `mc_propagate_error`.
fn check_math_font_params_in(
    st: &mut EngineState,
    danger: &mut bool,
) -> Result<(), crate::main_control::MainControlAbort> {
    use crate::main_control::MainControlAbort;
    if !sym_font_ok(&st.fonts, &st.eqtb, TEXT_SIZE)
        || !sym_font_ok(&st.fonts, &st.eqtb, SCRIPT_SIZE)
        || !sym_font_ok(&st.fonts, &st.eqtb, SCRIPT_SCRIPT_SIZE)
    {
        crate::output::error_here_with_diagnostic_in(
            st,
            c"Math formula deleted: Insufficient symbol fonts".as_ptr(),
        );
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 3;
        st.errors.help_line[2] = c"Sorry, but I can't typeset math unless \\textfont 2".as_ptr();
        st.errors.help_line[1] = c"and \\scriptfont 2 and \\scriptscriptfont 2 have all".as_ptr();
        st.errors.help_line[0] = c"the \\fontdimen values needed in math symbol fonts.".as_ptr();
        crate::main_control::mc_propagate_error(crate::errors::error_in(st))?;
        flush_math(&mut st.input.cur_list, &mut st.mem).map_err(MainControlAbort::Confusion)?;
        *danger = true;
    } else if !ext_font_ok(&st.fonts, &st.eqtb, TEXT_SIZE)
        || !ext_font_ok(&st.fonts, &st.eqtb, SCRIPT_SIZE)
        || !ext_font_ok(&st.fonts, &st.eqtb, SCRIPT_SCRIPT_SIZE)
    {
        crate::output::error_here_with_diagnostic_in(
            st,
            c"Math formula deleted: Insufficient extension fonts".as_ptr(),
        );
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 3;
        st.errors.help_line[2] = c"Sorry, but I can't typeset math unless \\textfont 3".as_ptr();
        st.errors.help_line[1] = c"and \\scriptfont 3 and \\scriptscriptfont 3 have all".as_ptr();
        st.errors.help_line[0] = c"the \\fontdimen values needed in math extension fonts.".as_ptr();
        crate::main_control::mc_propagate_error(crate::errors::error_in(st))?;
        flush_math(&mut st.input.cur_list, &mut st.mem).map_err(MainControlAbort::Confusion)?;
        *danger = true;
    }
    Ok(())
}

/// translated from xetex-math.c:194 (after_math)
pub(crate) fn after_math_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    use crate::main_control::{mc_propagate_error, MainControlAbort};

    let mut danger = false;

    let mut j = if st.input.cur_list.mode == MMODE {
        st.input.cur_list.e_tex_aux
    } else {
        TEX_NULL
    };

    check_math_font_params_in(st, &mut danger)?;

    let mut m = st.input.cur_list.mode as i32;
    let mut l = false;
    let mut p = fin_mlist_in(st, TEX_NULL)?;

    let a;
    if st.input.cur_list.mode as i32 == -m {
        // eTeX display math: second `$$` ends an inner display
        crate::scan::get_x_token_in(st)?;
        if st.scanner.cur_cmd != MATH_SHIFT {
            crate::output::error_here_with_diagnostic_in(
                st,
                c"Display math should end with $$".as_ptr(),
            );
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 2;
            st.errors.help_line[1] =
                c"The `$' that I just saw supposedly matches a previous `$$'.".as_ptr();
            st.errors.help_line[0] = c"So I shall assume that you typed `$$' both times.".as_ptr();
            mc_propagate_error(crate::input::back_error_in(st)?)?;
        }
        st.scanner.cur_mlist = p;
        st.scanner.cur_style = TEXT_STYLE;
        st.scanner.mlist_penalties = false;
        unsafe { mlist_to_hlist_in(st) }.map_err(math_build_abort_main)?;
        a = crate::packaging::hpack_emit_in(st, st.mem.b32s1(TEMP_HEAD), 0, ADDITIONAL)?
            .idx()
            .0;
        st.mem.set_b16s0(a, DLIST);
        crate::save::unsave_in(st)?;
        st.save_stack.save_ptr -= 1;
        if st.save_stack.b32s1(st.save_stack.save_ptr) == 1 {
            l = true;
        }
        danger = false;

        j = if st.input.cur_list.mode == MMODE {
            st.input.cur_list.e_tex_aux
        } else {
            TEX_NULL
        };

        check_math_font_params_in(st, &mut danger)?;

        m = st.input.cur_list.mode as i32;
        p = fin_mlist_in(st, TEX_NULL)?;
    } else {
        a = TEX_NULL;
    }

    if m < 0 {
        // inline math (or inner/restricted hmode)
        let math_surround = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_MATH_SURROUND);
        let node = crate::mem::new_math(&mut st.mem, math_surround, BEFORE as i16);
        if node == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(st.input.cur_list.tail, node);
        st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);
        st.scanner.cur_mlist = p;
        st.scanner.cur_style = TEXT_STYLE;
        st.scanner.mlist_penalties = st.input.cur_list.mode > 0;
        unsafe { mlist_to_hlist_in(st) }.map_err(math_build_abort_main)?;
        st.mem.set_b32s1(st.input.cur_list.tail, st.mem.b32s1(TEMP_HEAD));
        while st.mem.b32s1(st.input.cur_list.tail) != TEX_NULL {
            st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);
        }
        let node2 = crate::mem::new_math(&mut st.mem, math_surround, AFTER);
        if node2 == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(st.input.cur_list.tail, node2);
        st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);
        st.input.cur_list.set_aux_s0(1000);

        if st.config.semantic_pagination_enabled {
            unsafe { crate::ext::tt_insert_special_in(st, c"tdux:ce math".as_ptr()) }?;
        }

        crate::save::unsave_in(st)?;
    } else {
        // display math
        if a == TEX_NULL {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd != MATH_SHIFT {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Display math should end with $$".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"The `$' that I just saw supposedly matches a previous `$$'.".as_ptr();
                st.errors.help_line[0] =
                    c"So I shall assume that you typed `$$' both times.".as_ptr();
                mc_propagate_error(crate::input::back_error_in(st)?)?;
            }
        }
        st.scanner.cur_mlist = p;
        st.scanner.cur_style = DISPLAY_STYLE;
        st.scanner.mlist_penalties = false;
        unsafe { mlist_to_hlist_in(st) }.map_err(math_build_abort_main)?;
        p = st.mem.b32s1(TEMP_HEAD);
        st.scanner.adjust_tail = ADJUST_HEAD;
        st.scanner.pre_adjust_tail = PRE_ADJUST_HEAD;
        let mut b = crate::packaging::hpack_emit_in(st, p, 0, ADDITIONAL)?;
        p = b.list_ptr(&st.mem).0;
        let t = st.scanner.adjust_tail;
        st.scanner.adjust_tail = TEX_NULL;
        let pre_t = st.scanner.pre_adjust_tail;
        st.scanner.pre_adjust_tail = TEX_NULL;
        let mut w = b.width(&st.mem).0;
        let z = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_WIDTH);
        let mut s = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT);
        if st.eqtb.b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_DIRECTION) < 0 {
            s = -s - z;
        }
        let (mut e, q);
        if a == TEX_NULL || danger {
            e = 0;
            q = 0;
        } else {
            e = classify_box_or_panic(&st.mem, a).width(&st.mem).0;
            q = e + math_quad(&st.fonts, &st.eqtb, TEXT_SIZE);
        }
        if w + q > z {
            if e != 0
                && (w - st.scanner.total_shrink[0] + q <= z
                    || st.scanner.total_shrink[FIL] != 0
                    || st.scanner.total_shrink[FILL] != 0
                    || st.scanner.total_shrink[FILLL] != 0)
            {
                crate::mem::free_node(&mut st.mem, b.idx().0, BOX_NODE_SIZE);
                b = crate::packaging::hpack_emit_in(st, p, z - q, EXACTLY)?;
            } else {
                e = 0;
                if w > z {
                    crate::mem::free_node(&mut st.mem, b.idx().0, BOX_NODE_SIZE);
                    b = crate::packaging::hpack_emit_in(st, p, z, EXACTLY)?;
                }
            }
            w = b.width(&st.mem).0;
        }
        b.set_subtype(&mut st.mem, DLIST);
        let mut d = crate::scaled_math::half(z - w);
        if e > 0 && d < 2 * e {
            d = crate::scaled_math::half(z - w - e);
            if p != TEX_NULL && p < st.mem.hi_mem_min && st.mem.b16s1(p) == GLUE_NODE {
                d = 0;
            }
        }

        let penalty_node = crate::mem::new_penalty(
            &mut st.mem,
            st.eqtb.b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_PENALTY),
        );
        if penalty_node == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(st.input.cur_list.tail, penalty_node);
        st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);

        let pre_display_size = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_PRE_DISPLAY_SIZE);
        let (g1, mut g2);
        if d + s <= pre_display_size || l {
            g1 = GLUE_PAR_ABOVE_DISPLAY_SKIP;
            g2 = GLUE_PAR_BELOW_DISPLAY_SKIP;
        } else {
            g1 = GLUE_PAR_ABOVE_DISPLAY_SHORT_SKIP;
            g2 = GLUE_PAR_BELOW_DISPLAY_SHORT_SKIP;
        }

        if l && e == 0 {
            app_display_in(st, j, a, 0)?;
            let inf_node = crate::mem::new_penalty(&mut st.mem, INF_PENALTY);
            if inf_node == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(st.input.cur_list.tail, inf_node);
            st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);
        } else {
            let g1_node = crate::mem::new_param_glue(&mut st.mem, &st.eqtb, g1);
            if g1_node == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(st.input.cur_list.tail, g1_node);
            st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);
        }

        if e != 0 {
            let r = crate::mem::new_kern(&mut st.mem, z - w - e - d);
            if r == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            if l {
                st.mem.set_b32s1(a, r);
                st.mem.set_b32s1(r, b.idx().0);
                // `a` is a real box in this `e != 0` branch (`e` is its width).
                b = classify_box_or_panic(&st.mem, a);
                d = 0;
            } else {
                st.mem.set_b32s1(b.idx().0, r);
                st.mem.set_b32s1(r, a);
            }
            b = crate::packaging::hpack_emit_in(st, b.idx().0, 0, ADDITIONAL)?;
        }

        app_display_in(st, j, b.idx().0, d)?;

        if a != TEX_NULL && e == 0 && !l {
            let inf_node2 = crate::mem::new_penalty(&mut st.mem, INF_PENALTY);
            if inf_node2 == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(st.input.cur_list.tail, inf_node2);
            st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);
            let a_width = classify_box_or_panic(&st.mem, a).width(&st.mem).0;
            app_display_in(st, j, a, z - a_width)?;
            g2 = 0;
        }

        if t != ADJUST_HEAD {
            st.mem.set_b32s1(st.input.cur_list.tail, st.mem.b32s1(ADJUST_HEAD));
            st.input.cur_list.tail = t;
        }
        if pre_t != PRE_ADJUST_HEAD {
            st.mem
                .set_b32s1(st.input.cur_list.tail, st.mem.b32s1(PRE_ADJUST_HEAD));
            st.input.cur_list.tail = pre_t;
        }

        let post_pen = crate::mem::new_penalty(
            &mut st.mem,
            st.eqtb.b32s1(INT_BASE + INT_PAR_POST_DISPLAY_PENALTY),
        );
        if post_pen == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(st.input.cur_list.tail, post_pen);
        st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);

        if g2 > 0 {
            let g2_node = crate::mem::new_param_glue(&mut st.mem, &st.eqtb, g2);
            if g2_node == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(st.input.cur_list.tail, g2_node);
            st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);
        }

        crate::packaging::flush_node_list(&mut st.mem, j).map_err(MainControlAbort::Confusion)?;

        if st.config.semantic_pagination_enabled {
            unsafe { crate::ext::tt_insert_special_in(st, c"tdux:ce dmath".as_ptr()) }?;
        }

        resume_after_display_in(st)?;
    }
    Ok(())
}

/// translated from xetex-math.c:81 (init_math)
pub(crate) fn init_math_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    crate::scan::get_token_in(st)?;

    if st.scanner.cur_cmd == MATH_SHIFT && st.input.cur_list.mode > 0 {
        let mut j = TEX_NULL;
        let mut w = -MAX_HALFWORD;
        let x;

        if st.input.cur_list.head == st.input.cur_list.tail {
            crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
            x = if st.input.cur_list.e_tex_aux == TEX_NULL {
                0
            } else if st.mem.b32s0(st.input.cur_list.e_tex_aux) >= R_CODE {
                -1
            } else {
                1
            };
        } else {
            match crate::linebreak::line_break_in(st, true) {
                crate::linebreak::LineBreakOutcome::Done => {}
                o => return Err(crate::main_control::MainControlAbort::LineBreak(o)),
            }

            j = if st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_RIGHT_SKIP) == 0 {
                crate::mem::new_kern(&mut st.mem, 0)
            } else {
                crate::mem::new_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_RIGHT_SKIP as i16)
            };
            if j == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }

            let p_glue = if st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_LEFT_SKIP) == 0 {
                crate::mem::new_kern(&mut st.mem, 0)
            } else {
                crate::mem::new_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_LEFT_SKIP as i16)
            };
            if p_glue == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(p_glue, j);

            let Some(jbox) = crate::mem::alloc_null_box(&mut st.mem) else {
                return Err(NodeRecycleAbort::Overflow.into());
            };
            // `j` (the raw box index) is stored into `cur_list.e_tex_aux` later
            // in this function; keep it in sync with the typed handle.
            j = jbox.idx().0;
            let just_box = st.scanner.just_box;
            let jb = classify_box_or_panic(&st.mem, just_box);
            let jb_w = jb.width(&st.mem);
            let jb_sh = jb.shift_amount(&st.mem);
            let jb_go = jb.glue_order(&st.mem);
            let jb_gs = jb.glue_sign(&st.mem);
            let jb_gset = jb.glue_set(&st.mem);
            jbox.set_width(&mut st.mem, jb_w);
            jbox.set_shift_amount(&mut st.mem, jb_sh);
            jbox.set_list_ptr(&mut st.mem, MemPtr::from_raw(p_glue));
            jbox.set_glue_order(&mut st.mem, jb_go);
            jbox.set_glue_sign(&mut st.mem, jb_gs);
            jbox.set_glue_set(&mut st.mem, jb_gset);

            let mut v = jb_sh.0;
            x = if st.input.cur_list.e_tex_aux == TEX_NULL {
                0
            } else if st.mem.b32s0(st.input.cur_list.e_tex_aux) >= R_CODE {
                -1
            } else {
                1
            };

            let mut p;
            if x >= 0 {
                p = jb.list_ptr(&st.mem).0;
                st.mem.set_b32s1(TEMP_HEAD, TEX_NULL);
            } else {
                v = -v - jb.width(&st.mem).0;
                p = crate::mem::new_math(&mut st.mem, 0, BEGIN_L_CODE);
                if p == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow.into());
                }
                st.mem.set_b32s1(TEMP_HEAD, p);
                let end = crate::mem::new_math(&mut st.mem, 0, END_L_CODE);
                if end == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow.into());
                }
                let list = jb.list_ptr(&st.mem).0;
                crate::packaging::just_copy(&mut st.mem, list, p, end)?;
                st.scanner.cur_dir = RIGHT_TO_LEFT;
            }

            let cur_font = st.eqtb.b32s1(CUR_FONT_LOC);
            v += 2 * st
                .fonts
                .font_info_b32s1(QUAD_CODE + st.fonts.param_base(cur_font));

            if st.eqtb.b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
                let tp = crate::mem::get_avail(&mut st.mem);
                if tp == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow.into());
                }
                st.mem.temp_ptr = tp;
                st.mem.set_b32s0(st.mem.temp_ptr, BEFORE);
                st.mem.set_b32s1(st.mem.temp_ptr, st.scanner.LR_ptr);
                st.scanner.LR_ptr = st.mem.temp_ptr;
            }

            'outer: loop {
                if p == TEX_NULL {
                    break 'outer;
                }

                let is_found: bool;
                let d: i32;

                'reswitch: loop {
                    if p >= st.mem.hi_mem_min {
                        let f = st.mem.b16s1(p) as i32;
                        let c =
                            crate::fonts::effective_char(&mut st.fonts, true, f, st.mem.b16s0(p));
                        d = st.fonts.font_info_b32s1(
                            st.fonts.width_base_entry(f)
                                + st.fonts.font_info_b16(st.fonts.char_base_entry(f) + c).s3 as i32,
                        );
                        is_found = true;
                        break 'reswitch;
                    }
                    match st.mem.b16s1(p) {
                        0..=2 => {
                            d = st.mem.b32s1(p + 1);
                            is_found = true;
                            break 'reswitch;
                        }
                        6 => {
                            st.mem.set_word(GARBAGE, st.mem.word(p + 1));
                            st.mem.set_b32s1(GARBAGE, st.mem.b32s1(p));
                            p = GARBAGE;
                            st.fonts.xtx_ligature_present = true;
                            continue 'reswitch;
                        }
                        11 | 40 => {
                            d = st.mem.b32s1(p + 1);
                            is_found = false;
                            break 'reswitch;
                        }
                        9 => {
                            d = st
                                .mem
                                .classify_math(MemIdx::new(p))
                                .expect("valid math node")
                                .expect("math node")
                                .width(&st.mem)
                                .0;
                            if st.eqtb.b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
                                if st.mem.b16s0(p) & 1 != 0 {
                                    if st.mem.b32s0(st.scanner.LR_ptr)
                                        == L_CODE * (st.mem.b16s0(p) as i32 / L_CODE) + 3
                                    {
                                        st.mem.temp_ptr = st.scanner.LR_ptr;
                                        st.scanner.LR_ptr = st.mem.b32s1(st.mem.temp_ptr);
                                        st.mem.set_b32s1(st.mem.temp_ptr, st.mem.avail);
                                        st.mem.avail = st.mem.temp_ptr;
                                    } else if st.mem.b16s0(p) as i32 > L_CODE {
                                        w = MAX_HALFWORD;
                                        break 'outer;
                                    }
                                } else {
                                    let tp = crate::mem::get_avail(&mut st.mem);
                                    if tp == ALLOC_FAILED {
                                        return Err(NodeRecycleAbort::Overflow.into());
                                    }
                                    st.mem.temp_ptr = tp;
                                    st.mem.set_b32s0(
                                        st.mem.temp_ptr,
                                        L_CODE * (st.mem.b16s0(p) as i32 / L_CODE) + 3,
                                    );
                                    st.mem.set_b32s1(st.mem.temp_ptr, st.scanner.LR_ptr);
                                    st.scanner.LR_ptr = st.mem.temp_ptr;
                                    if st.mem.b16s0(p) as i32 / R_CODE != st.scanner.cur_dir as i32
                                    {
                                        crate::packaging::just_reverse(
                                            &mut st.mem,
                                            &mut st.scanner,
                                            p,
                                        )?;
                                        p = TEMP_HEAD;
                                    }
                                }
                            } else if st.mem.b16s0(p) as i32 >= L_CODE {
                                w = MAX_HALFWORD;
                                break 'outer;
                            }
                            is_found = false;
                            break 'reswitch;
                        }
                        14 => {
                            d = st.mem.b32s1(p + 1);
                            st.scanner.cur_dir = st.mem.b16s0(p) as i16;
                            is_found = false;
                            break 'reswitch;
                        }
                        10 => {
                            let gn = st
                                .mem
                                .classify_glue(MemIdx::new(p))
                                .expect("valid glue")
                                .expect("glue");
                            let gs = gn.glue_spec(&st.mem).expect("glue spec");
                            d = gs.width(&st.mem).0;
                            if jb.glue_sign(&st.mem) == STRETCHING {
                                if jb.glue_order(&st.mem) == gs.stretch_order(&st.mem)
                                    && gs.stretch(&st.mem).0 != 0
                                {
                                    v = MAX_HALFWORD;
                                }
                            } else if jb.glue_sign(&st.mem) == SHRINKING
                                && jb.glue_order(&st.mem) == gs.shrink_order(&st.mem)
                                && gs.shrink(&st.mem).0 != 0
                            {
                                v = MAX_HALFWORD;
                            }
                            if st.mem.b16s0(p) >= A_LEADERS {
                                is_found = true;
                                break 'reswitch;
                            }
                            is_found = false;
                            break 'reswitch;
                        }
                        8 => {
                            if WhatsitSubtype::from_raw(st.mem.b16s0(p))
                                .is_some_and(|s| s.has_dimensions())
                            {
                                d = st
                                    .mem
                                    .classify_whatsit(MemIdx::new(p))
                                    .expect("valid whatsit")
                                    .expect("whatsit")
                                    .width(&st.mem)
                                    .0;
                                is_found = true;
                            } else {
                                d = 0;
                                is_found = false;
                            }
                            break 'reswitch;
                        }
                        _ => {
                            d = 0;
                            is_found = false;
                            break 'reswitch;
                        }
                    }
                }

                if is_found {
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

                p = st.mem.b32s1(p);
            }

            if st.eqtb.b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
                while st.scanner.LR_ptr != TEX_NULL {
                    st.mem.temp_ptr = st.scanner.LR_ptr;
                    st.scanner.LR_ptr = st.mem.b32s1(st.mem.temp_ptr);
                    st.mem.set_b32s1(st.mem.temp_ptr, st.mem.avail);
                    st.mem.avail = st.mem.temp_ptr;
                }
                if st.scanner.LR_problems != 0 {
                    w = MAX_HALFWORD;
                    st.scanner.LR_problems = 0;
                }
            }

            st.scanner.cur_dir = LEFT_TO_RIGHT;
            let temp_head = st.mem.b32s1(TEMP_HEAD);
            crate::packaging::flush_node_list(&mut st.mem, temp_head)
                .map_err(crate::main_control::MainControlAbort::Confusion)?;
        }

        let l;
        let s;
        let par_shape = st.eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
        if par_shape == TEX_NULL {
            let hang = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_HANG_INDENT);
            let hang_after = st.eqtb.b32s1(INT_BASE + INT_PAR_HANG_AFTER);
            let prev_graf = st.input.cur_list.prev_graf;
            if hang != 0
                && ((hang_after >= 0 && prev_graf + 2 > hang_after) || prev_graf + 1 < -hang_after)
            {
                l = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_HSIZE) - hang.abs();
                s = if hang > 0 { hang } else { 0 };
            } else {
                l = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_HSIZE);
                s = 0;
            }
        } else {
            let n = st.mem.b32s0(par_shape);
            let p = if st.input.cur_list.prev_graf + 2 >= n {
                par_shape + 2 * n
            } else {
                par_shape + 2 * (st.input.cur_list.prev_graf + 2)
            };
            s = st.mem.b32s1(p - 1);
            l = st.mem.b32s1(p);
        }

        crate::main_control::push_math_in(st, GroupCode::MathShift)?;
        st.input.cur_list.mode = MMODE;
        if let Some(a) = crate::save::eq_word_define(
            &mut st.eqtb,
            &mut st.save_stack,
            INT_BASE + INT_PAR_CUR_FAM,
            -1,
        ) {
            return Err(crate::main_control::MainControlAbort::Save(a));
        }
        if let Some(a) = crate::save::eq_word_define(
            &mut st.eqtb,
            &mut st.save_stack,
            DIMEN_BASE + DIMEN_PAR_PRE_DISPLAY_SIZE,
            w,
        ) {
            return Err(crate::main_control::MainControlAbort::Save(a));
        }
        st.input.cur_list.e_tex_aux = j;
        if let Some(a) = crate::save::eq_word_define(
            &mut st.eqtb,
            &mut st.save_stack,
            INT_BASE + INT_PAR_PRE_DISPLAY_DIRECTION,
            x,
        ) {
            return Err(crate::main_control::MainControlAbort::Save(a));
        }
        if let Some(a) = crate::save::eq_word_define(
            &mut st.eqtb,
            &mut st.save_stack,
            DIMEN_BASE + DIMEN_PAR_DISPLAY_WIDTH,
            l,
        ) {
            return Err(crate::main_control::MainControlAbort::Save(a));
        }
        if let Some(a) = crate::save::eq_word_define(
            &mut st.eqtb,
            &mut st.save_stack,
            DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT,
            s,
        ) {
            return Err(crate::main_control::MainControlAbort::Save(a));
        }

        if st.config.semantic_pagination_enabled {
            unsafe { crate::ext::tt_insert_special_in(st, c"tdux:cs dmath".as_ptr()) }?;
        }

        let every_display = st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_DISPLAY);
        if every_display != TEX_NULL {
            crate::input::begin_token_list_in(st, every_display, EVERY_DISPLAY_TEXT)?;
        }
        if st.input.nest_ptr == 1 {
            crate::pagebuilder::build_page_in(st)?;
        }
    } else {
        crate::input::back_input_in(st)?;
        crate::main_control::push_math_in(st, GroupCode::MathShift)?;
        if let Some(a) = crate::save::eq_word_define(
            &mut st.eqtb,
            &mut st.save_stack,
            INT_BASE + INT_PAR_CUR_FAM,
            -1,
        ) {
            return Err(crate::main_control::MainControlAbort::Save(a));
        }
        if st.shipout.insert_src_special_every_math {
            crate::ext::insert_src_special_in(st)?;
        }
        if st.config.semantic_pagination_enabled {
            unsafe { crate::ext::tt_insert_special_in(st, c"tdux:cs math".as_ptr()) }?;
        }
        let every_math = st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_MATH);
        if every_math != TEX_NULL {
            crate::input::begin_token_list_in(st, every_math, EVERY_MATH_TEXT)?;
        }
    }
    Ok(())
}

/// translated from xetex-math.c:71 (initialize_math_variables)
pub fn initialize_math_variables() {
    // null_delimiter is a file-static b16x4 that is never read outside this
    // function; zeroing it is redundant (C zero-initializes statics) and a no-op.
}

/// translated from xetex-math.c:1356 (rebox)
enum ReboxResult {
    /// The box was repacked in place (width set); the typed handle is returned.
    Done(AnyBox),
    /// The box was dissolved into a glue-wrapped list; the raw list head must be
    /// hpacked by the caller.
    Hpack(i32),
}

fn rebox(
    mem: &mut MemState,
    bb: AnyBox,
    w: i32,
    char_kern_width: Option<i32>,
) -> Result<ReboxResult, NodeRecycleAbort> {
    if bb.width(mem).0 != w && !bb.list_ptr(mem).is_null() {
        let mut p = bb.list_ptr(mem).0;
        if let Some(v) = char_kern_width {
            if v != bb.width(mem).0 {
                let k = new_kern(mem, bb.width(mem).0 - v);
                if k == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow);
                }
                mem.set_b32s1(p, k);
            }
        }
        free_node(mem, bb.idx().0, BOX_NODE_SIZE);
        let g = new_glue(mem, 12);
        if g == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        mem.set_b32s1(g, p);
        while mem.b32s1(p) != TEX_NULL {
            p = mem.b32s1(p);
        }
        let tail_glue = new_glue(mem, 12);
        if tail_glue == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        mem.set_b32s1(p, tail_glue);
        Ok(ReboxResult::Hpack(g))
    } else {
        bb.set_width(mem, Scaled(w));
        Ok(ReboxResult::Done(bb))
    }
}

/// Threaded core (`01b` §3a): repack box `b` to width `w` under one borrow. The
/// re-pack and final pack go through [`crate::packaging::hpack_emit_in`] (which
/// prints any over/underfull diagnostic in-order under the borrow, session 186).
fn rebox_in(st: &mut EngineState, b: AnyBox, w: i32) -> Result<AnyBox, NodeRecycleAbort> {
    let mut bb = b;
    if bb.width(&st.mem).0 != w && !bb.list_ptr(&st.mem).is_null() && matches!(bb, AnyBox::VList(_))
    {
        bb = crate::packaging::hpack_emit_in(st, bb.idx().0, 0, ADDITIONAL)?;
    }

    let char_kern_width = if bb.width(&st.mem).0 != w && !bb.list_ptr(&st.mem).is_null() {
        let p = bb.list_ptr(&st.mem).0;
        if crate::mem::is_char_node(&st.mem, p) && st.mem.b32s1(p) == TEX_NULL {
            let f = st.mem.b16s1(p) as i32;
            let c_raw = st.mem.b16s0(p);
            let c_eff = crate::fonts::effective_char(&mut st.fonts, true, f, c_raw);
            let q = st.fonts.font_info_b16(st.fonts.char_base_entry(f) + c_eff);
            Some(
                st.fonts
                    .font_info_b32s1(st.fonts.width_base_entry(f) + q.s3 as i32),
            )
        } else {
            None
        }
    } else {
        None
    };

    match rebox(&mut st.mem, bb, w, char_kern_width)? {
        ReboxResult::Done(rb) => Ok(rb),
        ReboxResult::Hpack(rb) => Ok(crate::packaging::hpack_emit_in(st, rb, w, EXACTLY)?),
    }
}

/// translated from xetex-math.c:1206 (build_opentype_assembly)
///
/// Phase-C (`01b` §3a): threaded to a single `&mut EngineState` borrow. The C++
/// OpenType readers (`ot_min_*`, `ot_part_*`) are raw `extern "C"` calls with no
/// `with_state` re-entry, and the part-stacking helpers now have `_in` cores
/// (`stack_glue_into_box_in` / `stack_glyph_into_box_in`, whose native-glyph
/// measurement is the re-entry-free `&FontState` `measure_native_glyph`, session
/// 187), so the whole build — box-type set, node-list construction, and the
/// natural-size measurement + dimension write — runs under one borrow.
unsafe fn build_opentype_assembly_in(
    st: &mut EngineState,
    f: i32,
    a: *mut libc::c_void,
    s: i32,
    horiz: bool,
) -> Result<AnyBox, NodeRecycleAbort> {
    let b_idx = crate::mem::new_null_box(&mut st.mem);
    if b_idx == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    // `new_null_box` stamps an hlist; retype to the requested axis, then classify
    // once — `b` is the typed handle used for all stacking + measurement below.
    st.mem
        .set_b16s1(b_idx, if horiz { HLIST_NODE } else { VLIST_NODE });
    let b = classify_box_or_panic(&st.mem, b_idx);
    let mut n: i32 = -1;
    let mut no_extenders = true;
    let min_o = ot_min_connector_overlap(f);
    // Find minimum repeat count n such that the assembly reaches size s.
    loop {
        n += 1;
        let mut s_max: i32 = 0;
        let mut prev_o: i32 = 0;
        for i in 0..ot_part_count(a) {
            if ot_part_is_extender(a, i) {
                no_extenders = false;
                for _j in 1..=n {
                    let mut o = ot_part_start_connector(f, a, i);
                    if min_o < o {
                        o = min_o;
                    }
                    if prev_o < o {
                        o = prev_o;
                    }
                    s_max = s_max - o + ot_part_full_advance(f, a, i);
                    prev_o = ot_part_end_connector(f, a, i);
                }
            } else {
                let mut o = ot_part_start_connector(f, a, i);
                if min_o < o {
                    o = min_o;
                }
                if prev_o < o {
                    o = prev_o;
                }
                s_max = s_max - o + ot_part_full_advance(f, a, i);
                prev_o = ot_part_end_connector(f, a, i);
            }
        }
        if s_max >= s || no_extenders {
            break;
        }
    }
    // Build the node list.
    let mut prev_o: i32 = 0;
    for i in 0..ot_part_count(a) {
        if ot_part_is_extender(a, i) {
            for _j in 1..=n {
                let mut o = ot_part_start_connector(f, a, i);
                if prev_o < o {
                    o = prev_o;
                }
                let oo = o;
                if min_o < o {
                    o = min_o;
                }
                if oo > 0 {
                    stack_glue_into_box_in(st, b, -oo, -o)?;
                }
                stack_glyph_into_box_in(st, b, f, ot_part_glyph(a, i))?;
                prev_o = ot_part_end_connector(f, a, i);
            }
        } else {
            let mut o = ot_part_start_connector(f, a, i);
            if prev_o < o {
                o = prev_o;
            }
            let oo = o;
            if min_o < o {
                o = min_o;
            }
            if oo > 0 {
                stack_glue_into_box_in(st, b, -oo, -o)?;
            }
            stack_glyph_into_box_in(st, b, f, ot_part_glyph(a, i))?;
            prev_o = ot_part_end_connector(f, a, i);
        }
    }
    // Measure natural size and total stretch — pure `mem` reads.
    let mut p = b.list_ptr(&st.mem).0;
    let mut nat: i32 = 0;
    let mut str: i32 = 0;
    while p != TEX_NULL {
        match st.mem.classify(MemIdx::new(p)) {
            Ok(AnyNode::Whatsit(pw)) => {
                if horiz {
                    nat += pw.width(&st.mem).0;
                } else {
                    nat += pw.height(&st.mem).0 + pw.depth(&st.mem).0;
                }
            }
            Ok(AnyNode::Glue(gn)) => {
                let gs = gn.glue_spec(&st.mem).expect("glue spec");
                nat += gs.width(&st.mem).0;
                str += gs.stretch(&st.mem).0;
            }
            _ => {}
        }
        p = st.mem.b32s1(p);
    }
    // Write the resolved dimensions — pure `mem` writes through the typed box.
    if s > nat && str > 0 {
        let o = if s - nat > str { str } else { s - nat };
        b.set_glue_order(&mut st.mem, NORMAL);
        b.set_glue_sign(&mut st.mem, STRETCHING);
        // The C reads MEM[b+6].gr back after storing it; an f64 round-trips
        // exactly, so the stored `ratio` is byte-identical to the re-read.
        let ratio = o as f64 / str as f64;
        b.set_glue_set(&mut st.mem, ratio);
        let set_dim = Scaled(nat + crate::scaled_math::tex_round(str as f64 * ratio));
        if horiz {
            b.set_width(&mut st.mem, set_dim);
        } else {
            b.set_depth(&mut st.mem, set_dim);
        }
    } else if horiz {
        b.set_width(&mut st.mem, Scaled(nat));
    } else {
        b.set_depth(&mut st.mem, Scaled(nat));
    }
    Ok(b)
}

/// Build the extensible (`EXT_TAG`) delimiter box for [`var_delimiter_global`].
///
/// Phase-C: extensible delimiters are a pure-TFM math-font path — every font
/// read is a `FontState` table lookup and every char box is built with
/// [`char_box_tfm`], so there is **no** `with_state`-bearing FFI re-entry. The
/// whole construction therefore runs under one disjoint `mem`+`fonts` borrow
/// (taken by the caller in [`var_delimiter_global`]), replacing the ~12
/// scattered bridge closures the C translation left behind. Allocation order is
/// preserved (the vlist box first, then each stacked char box), so the mem image
/// is byte-identical to the inline version. (Unlike the OpenType assembly /
/// native-glyph paths, this one holds no borrow across a C++ measurer — see
/// `01b` §4 and the session-135 revert.)
fn build_ext_delimiter_box(
    mem: &mut MemState,
    fonts: &mut FontState,
    f: i32,
    q: crate::mem_types::B16x4,
    v: i32,
) -> Result<AnyBox, NodeRecycleAbort> {
    let vb = alloc_null_vbox(mem)?;
    let r = fonts.font_info_b16(fonts.exten_base_entry(f) + q.s0 as i32);
    // The repeatable (s0) piece sets the box width and the per-step height.
    let u = height_plus_depth(fonts, f, r.s0);
    let effective_c = crate::fonts::effective_char(fonts, true, f, r.s0);
    let qq = fonts.font_info_b16(fonts.char_base_entry(f) + effective_c);
    let box_width = fonts.font_info_b32s1(fonts.width_base_entry(f) + qq.s3 as i32)
        + fonts.font_info_b32s1(fonts.italic_base_entry(f) + qq.s1 as i32 / 4);
    vb.set_width(mem, Scaled(box_width));
    let mut w = 0;
    if r.s1 != 0 {
        w += height_plus_depth(fonts, f, r.s1);
    }
    if r.s2 != 0 {
        w += height_plus_depth(fonts, f, r.s2);
    }
    if r.s3 != 0 {
        w += height_plus_depth(fonts, f, r.s3);
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
        let p = char_box_tfm(mem, fonts, f, r.s1)?;
        stack_into_box(mem, AnyBox::VList(vb), p);
    }
    for _ in 1..=n {
        let p = char_box_tfm(mem, fonts, f, r.s0)?;
        stack_into_box(mem, AnyBox::VList(vb), p);
    }
    if r.s2 != 0 {
        let p = char_box_tfm(mem, fonts, f, r.s2)?;
        stack_into_box(mem, AnyBox::VList(vb), p);
        for _ in 1..=n {
            let p = char_box_tfm(mem, fonts, f, r.s0)?;
            stack_into_box(mem, AnyBox::VList(vb), p);
        }
    }
    if r.s3 != 0 {
        let p = char_box_tfm(mem, fonts, f, r.s3)?;
        stack_into_box(mem, AnyBox::VList(vb), p);
    }
    let height = vb.height(mem).0;
    vb.set_depth(mem, Scaled(w - height));
    Ok(AnyBox::VList(vb))
}

/// translated from xetex-math.c:1017 (var_delimiter)
/// Threaded core (`01b` §3a): pick and build the variable-size delimiter under
/// one `&mut EngineState` borrow. The font scan reads `eqtb`/`fonts` directly;
/// the OT readers (`usingOpenType`/`get_ot_*`/`free_ot_assembly`) and
/// `map_char_to_glyph` are raw `extern "C"` / `&FontState` (no `with_state`
/// re-entry), and the delimiter builders are the `_in` cores threaded above.
unsafe fn var_delimiter_in(
    st: &mut EngineState,
    d: i32,
    s: i32,
    v: i32,
) -> Result<AnyBox, MathBuildAbort> {
    let mut f: i32 = FONT_BASE;
    let mut ot_assembly_ptr: *mut libc::c_void = std::ptr::null_mut();
    let mut c: u16 = 0;
    let mut w: i32 = 0;
    let mut q = crate::mem_types::B16x4 {
        s3: 0,
        s2: 0,
        s1: 0,
        s0: 0,
    };
    let mut large_attempt = false;
    let d_s3 = st.mem.b16s3(d) as i32;
    let mut z = d_s3 % 256;
    let mut x = st.mem.b16s2(d) as i32 + (d_s3 / 256) * 65536;
    // Outer loop: try small delimiter family first, then large.
    // All exits break to 'found.
    'found: {
        loop {
            if z != 0 || x != 0 {
                z = z + s + 256;
                // Scan font sizes from s down to SCRIPT_SCRIPT_SIZE.
                loop {
                    z -= 256;
                    let g = math_font(&st.eqtb, z);
                    if g != FONT_BASE {
                        let font_area = st.fonts.font_area_entry(g);
                        let layout_engine = st.fonts.font_layout_engine_ptr(g);
                        if font_area == AAT_FONT_FLAG
                            || (font_area == OTGR_FONT_FLAG && usingOpenType(layout_engine))
                        {
                            x = crate::ext::map_char_to_glyph(&st.fonts, g, x)?;
                            f = g;
                            c = x as u16;
                            w = 0;
                            let mut n: i32 = 0;
                            let mut u: i32 = 0;
                            // Scan OT math variants.
                            loop {
                                let yv = get_ot_math_variant(g, x, n, &mut u, 0) as u16;
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
                            ot_assembly_ptr = get_ot_assembly_ptr(g, x, 0);
                            if !ot_assembly_ptr.is_null() {
                                break 'found;
                            }
                        } else {
                            let mut y = x as u16;
                            let scan: Option<(i32, u16, i32, crate::mem_types::B16x4, bool)> = 'scan: {
                                let fonts = &st.fonts;
                                if y < fonts.font_bc_entry(g) || y > fonts.font_ec_entry(g) {
                                    break 'scan None;
                                }
                                // Follow character list chain, looking for tallest glyph.
                                loop {
                                    let q_tmp =
                                        fonts.font_info_b16(fonts.char_base_entry(g) + y as i32);
                                    q = q_tmp;
                                    if q.s3 > 0 {
                                        if q.s1 % 4 == EXT_TAG {
                                            break 'scan Some((g, y, w, q, true));
                                        }
                                        let u = fonts.font_info_b32s1(
                                            fonts.height_base_entry(g) + q.s2 as i32 / 16,
                                        ) + fonts.font_info_b32s1(
                                            fonts.depth_base_entry(g) + q.s2 as i32 % 16,
                                        );
                                        if u > w {
                                            if u >= v {
                                                break 'scan Some((g, y, u, q, true));
                                            }
                                            f = g;
                                            c = y;
                                            w = u;
                                        }
                                        if q.s1 % 4 == LIST_TAG {
                                            y = q.s0;
                                            continue; // goto continue_
                                        }
                                    }
                                    break; // exit character-list loop
                                }
                                None
                            };
                            if let Some((found_f, found_c, found_w, found_q, stop)) = scan {
                                f = found_f;
                                c = found_c;
                                w = found_w;
                                q = found_q;
                                if stop {
                                    break 'found;
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
            z = st.mem.b16s1(d) as i32 % 256;
            x = st.mem.b16s0(d) as i32 + (st.mem.b16s1(d) as i32 / 256) * 65536;
        }
    }
    // found:
    let b;
    if f != FONT_BASE {
        let font_area = st.fonts.font_area_entry(f);
        let layout_engine = st.fonts.font_layout_engine_ptr(f);
        if !(font_area == OTGR_FONT_FLAG && usingOpenType(layout_engine)) {
            if q.s1 % 4 == EXT_TAG {
                b = build_ext_delimiter_box(&mut st.mem, &mut st.fonts, f, q, v)?;
            } else {
                b = char_box_in(st, f, c as i32)?;
            }
        } else if !ot_assembly_ptr.is_null() {
            b = build_opentype_assembly_in(st, f, ot_assembly_ptr, v, false)?;
        } else {
            b = native_glyph_vbox_in(st, f, c)?;
        }
    } else {
        // Null delimiter: an empty box of \nulldelimiterspace width.
        let Some(bb) = crate::mem::alloc_null_box(&mut st.mem) else {
            return Err(NodeRecycleAbort::Overflow.into());
        };
        let width = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_NULL_DELIMITER_SPACE);
        bb.set_width(&mut st.mem, Scaled(width));
        b = AnyBox::HList(bb);
    }
    let axis = axis_height(&st.fonts, &st.eqtb, s);
    let b_box = b;
    let shift = crate::scaled_math::half(b_box.height(&st.mem).0 - b_box.depth(&st.mem).0) - axis;
    b_box.set_shift_amount(&mut st.mem, Scaled(shift));
    free_ot_assembly(ot_assembly_ptr);
    Ok(b_box)
}
