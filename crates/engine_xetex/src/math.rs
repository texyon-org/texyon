//! Math-mode command helpers translated from `xetex/xetex-math.c`.
use crate::consts::{
    BOX_NODE_SIZE, CUR_FONT_LOC, DEL_CODE_BASE, DIMEN_BASE, DISC_NODE, GLUE_BASE, GLUE_NODE,
    GLUE_SPEC_SIZE, HLIST_NODE, INT_BASE, KERN_NODE, LOCAL_BASE, MATH_FONT_BASE, MAX_HALFWORD,
    MEDIUM_NODE_SIZE, PENALTY_NODE, RULE_NODE,
};
use crate::ext::real_get_native_glyph;
use crate::input::begin_token_list;

use crate::globals::{
    adjust_tail, avail, char_base_entry, cur_c, cur_chr, cur_cmd, cur_dir, cur_f, cur_group, cur_i,
    cur_lang, cur_list_aux_b32s1, cur_list_e_tex_aux, cur_list_head, cur_list_mode,
    cur_list_prev_graf, cur_list_tail, cur_mu, cur_size, cur_style, cur_val, cur_val1,
    depth_base_entry, eqtb_b32s1, file_line_error_style_p, font_area_entry, font_info_b16,
    font_info_b32s1, font_layout_engine_ptr, height_base_entry, hi_mem_min,
    insert_src_special_every_math, italic_base_entry, just_box, lr_problems, lr_ptr, mem_b16s0,
    mem_b16s1, mem_b16s2, mem_b16s3, mem_b32s0, mem_b32s1, mem_gr, mem_node_addr, mem_word,
    nest_ptr, param_base, pre_adjust_tail, save_ptr, save_stack_b32s1, semantic_pagination_enabled,
    set_adjust_tail, set_avail, set_cur_dir, set_cur_lang, set_cur_list_aux_b32s0,
    set_cur_list_aux_b32s1, set_cur_list_e_tex_aux, set_cur_list_mode, set_cur_list_prev_graf,
    set_cur_list_tail, set_cur_mlist, set_cur_mu, set_cur_size, set_cur_style, set_cur_val,
    set_cur_val1, set_help_line, set_help_ptr, set_lr_problems, set_lr_ptr, set_mem_b16s0,
    set_mem_b16s1, set_mem_b16s2, set_mem_b16s3, set_mem_b32s0, set_mem_b32s1, set_mem_gr,
    set_mem_word, set_mlist_penalties, set_pre_adjust_tail, set_save_ptr, set_save_stack_b32s1,
    set_temp_ptr, set_xtx_ligature_present, temp_ptr, tex_remainder, total_shrink,
    width_base_entry,
};
use crate::mem::{
    free_node, free_node_global, get_avail, get_node, get_node_global, new_choice, new_glue,
    new_glue_global, new_kern, new_kern_global, new_noad, new_noad_global, new_null_box, new_rule,
    new_spec, ALLOC_FAILED, TEX_NULL,
};
use crate::scan::{node_recycle_abort, NodeRecycleAbort};
use crate::state::{EqtbState, FontState, MemState, SaveStackState, ScannerState};

const INT_PAR_CUR_FAM: i32 = 44;
const LOCAL_EVERY_MATH: i32 = 3;

const EVERY_MATH_TEXT: u16 = 9;
const MATH_CHOICE_GROUP: u8 = 13;
const MATH_SHIFT_GROUP: u8 = 15;
const MATH_LEFT_GROUP: u8 = 16;

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
const ACCENT_NOAD: u16 = 28;
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
const RADICAL_NOAD: u16 = 24;
const FRACTION_NOAD: u16 = 25;
const UNDER_NOAD: u16 = 26;
const OVER_NOAD: u16 = 27;
const OP_NOAD: u16 = 17;
const BIN_NOAD: u16 = 18;
const REL_NOAD: u16 = 19;
const OPEN_NOAD: u16 = 20;
const CLOSE_NOAD: u16 = 21;
const INNER_NOAD: u16 = 23;
const VCENTER_NOAD: u16 = 29;
const LEFT_NOAD: u16 = 30;
const RIGHT_NOAD: u16 = 31;
const ORD_NOAD: u16 = 16;
const PUNCT_NOAD: u16 = 22;
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
const NATIVE_WORD_NODE: u16 = 40;
const NATIVE_WORD_NODE_AT: u16 = 41;
const GLYPH_NODE: u16 = 42;
const PIC_NODE: u16 = 43;
const PDF_NODE: u16 = 44;
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
const INS_NODE: u16 = 3;
const MARK_NODE: u16 = 4;
const ADJUST_NODE: u16 = 5;
const STYLE_NODE: u16 = 14;
const CHOICE_NODE: u16 = 15;
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

/// Threaded core (Phase C, bucket B): pure `mem` writes, no `with_state`-bearing
/// call.
#[inline]
fn set_empty_math_field(mem: &mut MemState, p: i32) {
    mem.set_b32s1(p, EMPTY);
    mem.set_b32s0(p, TEX_NULL);
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
fn font_is_ot_math_global(f: i32) -> bool {
    crate::state::with_state(|st| font_is_ot_math(&st.fonts, f))
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

pub fn math_quad_global(size_code: i32) -> i32 {
    crate::state::with_state(|st| math_quad(&st.fonts, &st.eqtb, size_code))
}

/// translated from xetex-math.c:115 (num1)
pub fn num1(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 8)
}

pub fn num1_global(size_code: i32) -> i32 {
    crate::state::with_state(|st| num1(&st.fonts, &st.eqtb, size_code))
}

/// translated from xetex-math.c:130 (num2)
pub fn num2(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 9)
}

pub fn num2_global(size_code: i32) -> i32 {
    crate::state::with_state(|st| num2(&st.fonts, &st.eqtb, size_code))
}

/// translated from xetex-math.c:145 (num3)
pub fn num3(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 10)
}

pub fn num3_global(size_code: i32) -> i32 {
    crate::state::with_state(|st| num3(&st.fonts, &st.eqtb, size_code))
}

/// translated from xetex-math.c:160 (denom1)
pub fn denom1(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 11)
}

pub fn denom1_global(size_code: i32) -> i32 {
    crate::state::with_state(|st| denom1(&st.fonts, &st.eqtb, size_code))
}

/// translated from xetex-math.c:175 (denom2)
pub fn denom2(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 12)
}

pub fn denom2_global(size_code: i32) -> i32 {
    crate::state::with_state(|st| denom2(&st.fonts, &st.eqtb, size_code))
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

pub fn delim1_global(size_code: i32) -> i32 {
    crate::state::with_state(|st| delim1(&st.fonts, &st.eqtb, size_code))
}

/// translated from xetex-math.c:303 (delim2)
pub fn delim2(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 21)
}

pub fn delim2_global(size_code: i32) -> i32 {
    crate::state::with_state(|st| delim2(&st.fonts, &st.eqtb, size_code))
}

/// translated from xetex-math.c:317 (axis_height)
pub fn axis_height(fonts: &FontState, eqtb: &EqtbState, size_code: i32) -> i32 {
    math_sy_param(fonts, eqtb, size_code, 22)
}

pub fn axis_height_global(size_code: i32) -> i32 {
    crate::state::with_state(|st| axis_height(&st.fonts, &st.eqtb, size_code))
}

/// translated from xetex-math.c:88 (default_rule_thickness)
pub fn default_rule_thickness(fonts: &FontState, eqtb: &EqtbState, cur_size: i32) -> i32 {
    math_ext_param(fonts, eqtb, cur_size, 8)
}

pub fn default_rule_thickness_global() -> i32 {
    let size = cur_size();
    crate::state::with_state(|st| default_rule_thickness(&st.fonts, &st.eqtb, size))
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
/// [`new_rule`] and propagates its [`ALLOC_FAILED`] sentinel as
/// [`NodeRecycleAbort::Overflow`] (`01b` §5).
pub fn fraction_rule(mem: &mut MemState, t: i32) -> Result<i32, NodeRecycleAbort> {
    let p = new_rule(mem);
    if p == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(p + 3, t);
    mem.set_b32s1(p + 2, 0);
    Ok(p)
}

/// translated from xetex-math.c:181 (overbar)
///
/// Phase-C: threaded to `&mut MemState`. Builds the kern/rule/kern chain
/// directly through the threaded [`new_kern`]/[`fraction_rule`] and returns its
/// head; with_state-bearing `vpackage` callers run after the borrow drops
/// (bucket C). Allocation failure propagates the
/// [`NodeRecycleAbort`] sentinel (`01b` §5).
pub fn overbar(mem: &mut MemState, b: i32, k: i32, t: i32) -> Result<i32, NodeRecycleAbort> {
    let mut p = new_kern(mem, k);
    if p == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(p, b);
    let q = fraction_rule(mem, t)?;
    mem.set_b32s1(q, p);
    p = new_kern(mem, t);
    if p == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(p, q);
    Ok(p)
}

/// translated from xetex-math.c:195 (math_glue)
struct MathGlueInput {
    width: i32,
    stretch_order: u16,
    stretch: i32,
    shrink_order: u16,
    shrink: i32,
}

fn scaled_mu_value(v: i32, m: i32) -> i32 {
    let mut n = unsafe { crate::scaled_math::x_over_n(m, 65_536) };
    let mut f = tex_remainder();
    if f < 0 {
        n -= 1;
        f += 65_536;
    }
    unsafe {
        crate::scaled_math::mult_and_add(
            n,
            v,
            crate::scaled_math::xn_over_d(v, f, 65_536),
            MAX_HALFWORD,
        )
    }
}

fn math_glue(mem: &mut MemState, input: MathGlueInput) -> Result<i32, NodeRecycleAbort> {
    let p = get_node(mem, GLUE_SPEC_SIZE);
    if p == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(p + 1, input.width);
    mem.set_b16s1(p, input.stretch_order);
    mem.set_b32s1(p + 2, input.stretch);
    mem.set_b16s0(p, input.shrink_order);
    mem.set_b32s1(p + 3, input.shrink);
    Ok(p)
}

pub unsafe fn math_glue_global(g: i32, m: i32) -> i32 {
    let stretch_order = mem_b16s1(g);
    let shrink_order = mem_b16s0(g);
    let input = MathGlueInput {
        width: scaled_mu_value(mem_b32s1(g + 1), m),
        stretch_order,
        stretch: if stretch_order == NORMAL {
            scaled_mu_value(mem_b32s1(g + 2), m)
        } else {
            mem_b32s1(g + 2)
        },
        shrink_order,
        shrink: if shrink_order == NORMAL {
            scaled_mu_value(mem_b32s1(g + 3), m)
        } else {
            mem_b32s1(g + 3)
        },
    };
    match crate::state::with_state_mut(|st| math_glue(&mut st.mem, input)) {
        Ok(p) => p,
        Err(e) => node_recycle_abort(e),
    }
}

/// translated from xetex-math.c:223 (math_kern)
fn math_kern(mem: &mut MemState, p: i32, scaled_width: Option<i32>) {
    if let Some(width) = scaled_width {
        mem.set_b32s1(p + 1, width);
        mem.set_b16s0(p, EXPLICIT);
    }
}

pub unsafe fn math_kern_global(p: i32, m: i32) {
    let scaled_width = if mem_b16s0(p) == MU_GLUE {
        Some(scaled_mu_value(mem_b32s1(p + 1), m))
    } else {
        None
    };
    crate::state::with_state_mut(|st| math_kern(&mut st.mem, p, scaled_width));
}

/// translated from xetex-math.c:80 (flush_math)
///
/// Phase C (`01b` §4a bucket B/C): touches the `mem` arena directly through
/// `&mut MemState` and recycles the two current-list chains via the threaded
/// [`crate::packaging::flush_node_list`]. The `cur_list` reads/writes go through the
/// raw `cur_list` global (the deferred Phase-C set — safe under a live borrow).
/// The two impossible corrupt-node `confusion` arms propagate out as `Err`, fired
/// by [`flush_math_global`] after the borrow drops.
pub fn flush_math(mem: &mut MemState) -> Result<(), &'static std::ffi::CStr> {
    let head = cur_list_head();
    let p1 = mem.b32s1(head);
    crate::packaging::flush_node_list(mem, p1)?;
    let p2 = cur_list_aux_b32s1();
    crate::packaging::flush_node_list(mem, p2)?;
    mem.set_b32s1(head, TEX_NULL);
    set_cur_list_tail(head);
    set_cur_list_aux_b32s1(TEX_NULL);
    Ok(())
}

/// Bridge-down wrapper for legacy callers (`01b` §3): borrows the anchor, runs
/// the threaded [`flush_math`], then fires the cold `confusion` after the borrow
/// drops if a corrupt node was hit.
pub unsafe fn flush_math_global() {
    if let Err(msg) = crate::state::with_state_mut(|st| flush_math(&mut st.mem)) {
        crate::errors::confusion(msg.as_ptr());
    }
}

// Phase-C: routes through the canonical `crate::mem::is_char_node` predicate
// (session 110 dedup) via a short bridge borrow; the prior local copy
// (`p >= hi_mem_min()`) duplicated that logic.
#[inline]
fn is_char_node_global(p: i32) -> bool {
    crate::state::with_state(|st| crate::mem::is_char_node(&st.mem, p))
}

/// translated from xetex-math.c:91 (clean_box)
#[no_mangle]
pub unsafe extern "C" fn clean_box(p: i32, s: i16) -> i32 {
    let q;
    match mem_b32s1(p) {
        MATH_CHAR => {
            let mlist = new_noad_global();
            set_cur_mlist(mlist);
            set_mem_word(mlist + 1, mem_word(p));

            let save_style = cur_style();
            set_cur_style(s);
            set_mlist_penalties(false);
            mlist_to_hlist();
            q = mem_b32s1(TEMP_HEAD);
            set_cur_style(save_style);

            if cur_style() < SCRIPT_STYLE {
                set_cur_size(TEXT_SIZE);
            } else {
                set_cur_size(SCRIPT_SIZE * ((cur_style() as i32 - 2) / 2));
            }
            set_cur_mu(crate::scaled_math::x_over_n(
                math_quad_global(cur_size()),
                18,
            ));
        }
        SUB_BOX => {
            q = mem_b32s0(p);
        }
        SUB_MLIST => {
            set_cur_mlist(mem_b32s0(p));

            let save_style = cur_style();
            set_cur_style(s);
            set_mlist_penalties(false);
            mlist_to_hlist();
            q = mem_b32s1(TEMP_HEAD);
            set_cur_style(save_style);

            if cur_style() < SCRIPT_STYLE {
                set_cur_size(TEXT_SIZE);
            } else {
                set_cur_size(SCRIPT_SIZE * ((cur_style() as i32 - 2) / 2));
            }
            set_cur_mu(crate::scaled_math::x_over_n(
                math_quad_global(cur_size()),
                18,
            ));
        }
        _ => {
            q = crate::mem::new_null_box_global();
        }
    }

    let x = if is_char_node_global(q) || q == TEX_NULL {
        crate::packaging::hpack(q, 0, ADDITIONAL)
    } else if mem_b32s1(q) == TEX_NULL && mem_b16s1(q) <= VLIST_NODE && mem_b32s1(q + 4) == 0 {
        q
    } else {
        crate::packaging::hpack(q, 0, ADDITIONAL)
    };

    let q = mem_b32s1(x + 5);
    if is_char_node_global(q) {
        let r = mem_b32s1(q);
        if r != TEX_NULL
            && mem_b32s1(r) == TEX_NULL
            && !is_char_node_global(r)
            && mem_b16s1(r) == KERN_NODE
        {
            free_node_global(r, MEDIUM_NODE_SIZE);
            set_mem_b32s1(q, TEX_NULL);
        }
    }
    x
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

/// Bridge-down wrapper for legacy callers: run the threaded `fetch_core` under a
/// short disjoint borrow, then (cold paths only) emit the diagnostic / warning
/// after the borrow has dropped.
pub unsafe fn fetch_global(a: i32) {
    let outcome = crate::state::with_state_mut(|st| {
        let st = &mut *st;
        fetch_core(&mut st.mem, &st.fonts, &st.eqtb, &mut st.scanner, a)
    });
    match outcome {
        FetchOutcome::Ok => {}
        FetchOutcome::UndefinedFont { size, fam, c } => {
            crate::output::error_here_with_diagnostic(c"".as_ptr());
            crate::output::print_size(size);
            crate::output::print_char(' ' as i32);
            crate::output::print_int(fam);
            crate::output::print_cstr(c" is undefined (character ".as_ptr());
            crate::output::print(c);
            crate::output::print_char(')' as i32);
            crate::output::capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(4);
            set_help_line(
                3,
                c"Somewhere in the math formula just ended, you used the".as_ptr(),
            );
            set_help_line(
                2,
                c"stated character from an undefined font family. For example,".as_ptr(),
            );
            set_help_line(
                1,
                c"plain TeX doesn't allow \\it or \\sl in subscripts. Proceed,".as_ptr(),
            );
            set_help_line(
                0,
                c"and I'll try to forget that I needed that character.".as_ptr(),
            );
            crate::errors::error();
        }
        FetchOutcome::CharMissing { f, c } => {
            crate::output::char_warning(f, c);
        }
    }
}

/// translated from xetex-math.c:198 (make_over)
fn make_over(mem: &mut MemState, q: i32, x: i32) {
    mem.set_b32s0(q + 1, x);
    mem.set_b32s1(q + 1, SUB_BOX);
}

pub unsafe fn make_over_global(q: i32) {
    let x = clean_box(q + 1, 2 * (cur_style() / 2) + 1);
    let rule_thickness = default_rule_thickness_global();
    let p = match crate::state::with_state_mut(|st| {
        overbar(&mut st.mem, x, 3 * rule_thickness, rule_thickness)
    }) {
        Ok(p) => p,
        Err(e) => node_recycle_abort(e),
    };
    let x = crate::packaging::vpackage(p, 0, ADDITIONAL, MAX_HALFWORD);
    crate::state::with_state_mut(|st| make_over(&mut st.mem, q, x));
}

/// translated from xetex-math.c:207 (make_under)
fn make_under_prepare(
    mem: &mut MemState,
    x: i32,
    rule_thickness: i32,
) -> Result<(), NodeRecycleAbort> {
    let p = new_kern(mem, 3 * rule_thickness);
    if p == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(x, p);
    let rule = fraction_rule(mem, rule_thickness)?;
    mem.set_b32s1(p, rule);
    Ok(())
}

fn make_under_finish(mem: &mut MemState, q: i32, x: i32, y: i32, rule_thickness: i32) {
    let delta = mem.b32s1(y + 3) + mem.b32s1(y + 2) + rule_thickness;
    mem.set_b32s1(y + 3, mem.b32s1(x + 3));
    mem.set_b32s1(y + 2, delta - mem.b32s1(y + 3));
    mem.set_b32s0(q + 1, y);
    mem.set_b32s1(q + 1, SUB_BOX);
}

pub unsafe fn make_under_global(q: i32) {
    let x = clean_box(q + 1, cur_style());
    let rule_thickness = default_rule_thickness_global();
    if let Err(e) =
        crate::state::with_state_mut(|st| make_under_prepare(&mut st.mem, x, rule_thickness))
    {
        node_recycle_abort(e);
    }
    let y = crate::packaging::vpackage(x, 0, ADDITIONAL, MAX_HALFWORD);
    crate::state::with_state_mut(|st| make_under_finish(&mut st.mem, q, x, y, rule_thickness));
}

/// translated from xetex-math.c:225 (make_vcenter)
fn make_vcenter(mem: &mut MemState, q: i32, axis_height: i32) -> Result<(), NodeRecycleAbort> {
    let v = mem.b32s0(q + 1);
    if mem.b16s1(v) != VLIST_NODE {
        return Err(NodeRecycleAbort::Confusion(c"vcenter"));
    }
    let delta = mem.b32s1(v + 3) + mem.b32s1(v + 2);
    mem.set_b32s1(
        v + 3,
        axis_height + unsafe { crate::scaled_math::half(delta) },
    );
    mem.set_b32s1(v + 2, delta - mem.b32s1(v + 3));
    Ok(())
}

pub unsafe fn make_vcenter_global(q: i32) {
    let axis_height = axis_height_global(cur_size());
    if let Err(e) = crate::state::with_state_mut(|st| make_vcenter(&mut st.mem, q, axis_height)) {
        node_recycle_abort(e);
    }
}

fn make_radical_finish(mem: &mut MemState, q: i32, x: i32) {
    mem.set_b32s0(q + 1, x);
    mem.set_b32s1(q + 1, SUB_BOX);
}

fn make_radical_prepare_delimiter(
    mem: &mut MemState,
    y: i32,
    x: i32,
    clr: i32,
    rule_thickness: i32,
    is_ot_math: bool,
) -> (i32, i32, i32) {
    if is_ot_math {
        mem.set_b32s1(y + 2, mem.b32s1(y + 3) + mem.b32s1(y + 2) - rule_thickness);
        mem.set_b32s1(y + 3, rule_thickness);
    }

    let delta = mem.b32s1(y + 2) - (mem.b32s1(x + 3) + mem.b32s1(x + 2) + clr);
    let x_height = mem.b32s1(x + 3);
    let rule_height = mem.b32s1(y + 3);
    (delta, x_height, rule_height)
}

/// translated from xetex-math.c:80 (make_radical)
pub unsafe fn make_radical_global(q: i32) {
    let (f, is_ot_math) = crate::state::with_state(|st| {
        let f = math_font(
            &st.eqtb,
            (st.mem.b16s3(q + 4) as i32 % 256) + st.scanner.cur_size,
        );
        (f, font_is_ot_math(&st.fonts, f))
    });
    let rule_thickness = if is_ot_math {
        get_ot_math_constant(f, RADICALRULETHICKNESS)
    } else {
        crate::state::with_state(|st| {
            default_rule_thickness(&st.fonts, &st.eqtb, st.scanner.cur_size)
        })
    };

    let clean_style = crate::state::with_state(|st| 2 * (st.scanner.cur_style / 2) + 1);
    let x = clean_box(q + 1, clean_style);
    let (cur_style, cur_size) =
        crate::state::with_state(|st| (st.scanner.cur_style, st.scanner.cur_size));
    let mut clr = if is_ot_math {
        if cur_style < TEXT_STYLE {
            get_ot_math_constant(f, RADICALDISPLAYSTYLEVERTICALGAP)
        } else {
            get_ot_math_constant(f, RADICALVERTICALGAP)
        }
    } else if cur_style < TEXT_STYLE {
        rule_thickness
            + crate::state::with_state(|st| math_x_height(&st.fonts, &st.eqtb, cur_size)).abs() / 4
    } else {
        rule_thickness + rule_thickness.abs() / 4
    };

    let delimiter_height = crate::state::with_state(|st| {
        st.mem.b32s1(x + 3) + st.mem.b32s1(x + 2) + clr + rule_thickness
    });
    let y = var_delimiter_global(q + 4, cur_size, delimiter_height);
    let (delta, x_height, rule_height) = crate::state::with_state_mut(|st| {
        make_radical_prepare_delimiter(&mut st.mem, y, x, clr, rule_thickness, is_ot_math)
    });
    if delta > 0 {
        clr += crate::scaled_math::half(delta);
    }
    crate::state::with_state_mut(|st| st.mem.set_b32s1(y + 4, -(x_height + clr)));
    let p = match crate::state::with_state_mut(|st| overbar(&mut st.mem, x, clr, rule_height)) {
        Ok(p) => p,
        Err(e) => node_recycle_abort(e),
    };
    let overbar_box = crate::packaging::vpackage(p, 0, ADDITIONAL, MAX_HALFWORD);
    crate::state::with_state_mut(|st| st.mem.set_b32s1(y, overbar_box));
    let hbox = crate::packaging::hpack(y, 0, ADDITIONAL);
    crate::state::with_state_mut(|st| make_radical_finish(&mut st.mem, q, hbox));
}

struct FractionAssembly {
    x: i32,
    z: i32,
    rule_thickness: i32,
    shift_up: i32,
    shift_down: i32,
    delta: i32,
    axis_height: i32,
}

fn make_fraction(mem: &mut MemState, assembly: FractionAssembly) -> Result<i32, NodeRecycleAbort> {
    let v = new_null_box(mem);
    if v == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b16s1(v, VLIST_NODE);
    mem.set_b32s1(v + 3, assembly.shift_up + mem.b32s1(assembly.x + 3));
    mem.set_b32s1(v + 2, mem.b32s1(assembly.z + 2) + assembly.shift_down);
    mem.set_b32s1(v + 1, mem.b32s1(assembly.x + 1));

    let p;
    if assembly.rule_thickness == 0 {
        p = new_kern(
            mem,
            (assembly.shift_up - mem.b32s1(assembly.x + 2))
                - (mem.b32s1(assembly.z + 3) - assembly.shift_down),
        );
        if p == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        mem.set_b32s1(p, assembly.z);
    } else {
        let y = fraction_rule(mem, assembly.rule_thickness)?;
        let p1 = new_kern(
            mem,
            (assembly.axis_height - assembly.delta)
                - (mem.b32s1(assembly.z + 3) - assembly.shift_down),
        );
        if p1 == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        mem.set_b32s1(y, p1);
        mem.set_b32s1(p1, assembly.z);
        p = new_kern(
            mem,
            (assembly.shift_up - mem.b32s1(assembly.x + 2))
                - (assembly.axis_height + assembly.delta),
        );
        if p == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        mem.set_b32s1(p, y);
    }

    mem.set_b32s1(assembly.x, p);
    mem.set_b32s1(v + 5, assembly.x);
    Ok(v)
}

fn make_fraction_link_delimiters(mem: &mut MemState, left_delim: i32, v: i32, right_delim: i32) {
    mem.set_b32s1(left_delim, v);
    mem.set_b32s1(v, right_delim);
}

fn make_fraction_finish(mem: &mut MemState, q: i32, hbox: i32) {
    mem.set_b32s1(q + 1, hbox);
}

/// translated from xetex-math.c:313 (make_fraction)
pub unsafe fn make_fraction_global(q: i32) {
    if mem_b32s1(q + 1) == DEFAULT_CODE {
        set_mem_b32s1(q + 1, default_rule_thickness_global());
    }

    let mut x = clean_box(q + 2, cur_style() + 2 - 2 * (cur_style() / 6));
    let mut z = clean_box(q + 3, 2 * (cur_style() / 2) + 3 - 2 * (cur_style() / 6));

    if mem_b32s1(x + 1) < mem_b32s1(z + 1) {
        x = rebox_global(x, mem_b32s1(z + 1));
    } else {
        z = rebox_global(z, mem_b32s1(x + 1));
    }

    let (mut shift_up, mut shift_down) = if cur_style() < TEXT_STYLE {
        (num1_global(cur_size()), denom1_global(cur_size()))
    } else {
        (
            if mem_b32s1(q + 1) != 0 {
                num2_global(cur_size())
            } else {
                num3_global(cur_size())
            },
            denom2_global(cur_size()),
        )
    };

    let mut delta = 0;
    if mem_b32s1(q + 1) == 0 {
        let clr = if font_is_ot_math_global(cur_f()) {
            if cur_style() < TEXT_STYLE {
                get_ot_math_constant(cur_f(), STACKDISPLAYSTYLEGAPMIN)
            } else {
                get_ot_math_constant(cur_f(), STACKGAPMIN)
            }
        } else if cur_style() < TEXT_STYLE {
            7 * default_rule_thickness_global()
        } else {
            3 * default_rule_thickness_global()
        };

        delta = crate::scaled_math::half(
            clr - ((shift_up - mem_b32s1(x + 2)) - (mem_b32s1(z + 3) - shift_down)),
        );
        if delta > 0 {
            shift_up += delta;
            shift_down += delta;
        }
    } else {
        let (delta1, delta2) = if font_is_ot_math_global(cur_f()) {
            delta = crate::scaled_math::half(mem_b32s1(q + 1));
            let clr_num = if cur_style() < TEXT_STYLE {
                get_ot_math_constant(cur_f(), FRACTIONNUMDISPLAYSTYLEGAPMIN)
            } else {
                get_ot_math_constant(cur_f(), FRACTIONNUMERATORGAPMIN)
            };
            let delta1 = clr_num
                - ((shift_up - mem_b32s1(x + 2)) - (axis_height_global(cur_size()) + delta));

            let clr_den = if cur_style() < TEXT_STYLE {
                get_ot_math_constant(cur_f(), FRACTIONDENOMDISPLAYSTYLEGAPMIN)
            } else {
                get_ot_math_constant(cur_f(), FRACTIONDENOMINATORGAPMIN)
            };
            let delta2 = clr_den
                - ((axis_height_global(cur_size()) - delta) - (mem_b32s1(z + 3) - shift_down));
            (delta1, delta2)
        } else {
            let clr = if cur_style() < TEXT_STYLE {
                3 * mem_b32s1(q + 1)
            } else {
                mem_b32s1(q + 1)
            };
            delta = crate::scaled_math::half(mem_b32s1(q + 1));
            (
                clr - ((shift_up - mem_b32s1(x + 2)) - (axis_height_global(cur_size()) + delta)),
                clr - ((axis_height_global(cur_size()) - delta) - (mem_b32s1(z + 3) - shift_down)),
            )
        };

        if delta1 > 0 {
            shift_up += delta1;
        }
        if delta2 > 0 {
            shift_down += delta2;
        }
    }

    let axis_height = axis_height_global(cur_size());
    let rule_thickness = mem_b32s1(q + 1);
    let v = match crate::state::with_state_mut(|st| {
        make_fraction(
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
        )
    }) {
        Ok(v) => v,
        Err(e) => node_recycle_abort(e),
    };

    let delimiter_size = if cur_style() < TEXT_STYLE {
        delim1_global(cur_size())
    } else {
        delim2_global(cur_size())
    };

    let left_delim = var_delimiter_global(q + 4, cur_size(), delimiter_size);
    let right_delim = var_delimiter_global(q + 5, cur_size(), delimiter_size);
    crate::state::with_state_mut(|st| {
        make_fraction_link_delimiters(&mut st.mem, left_delim, v, right_delim)
    });
    let hbox = crate::packaging::hpack(left_delim, 0, ADDITIONAL);
    crate::state::with_state_mut(|st| make_fraction_finish(&mut st.mem, q, hbox));
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
pub unsafe fn make_op_global(q: i32) -> i32 {
    let mut delta: i32 = 0;
    let mut ot_assembly_ptr: *mut libc::c_void = std::ptr::null_mut();

    // NORMAL -> LIMITS latch.
    crate::state::with_state_mut(|st| {
        if st.mem.b16s0(q) == NORMAL && st.scanner.cur_style < TEXT_STYLE {
            st.mem.set_b16s0(q, LIMITS);
        }
    });

    if mem_b32s1(q + 1) == MATH_CHAR {
        fetch_global(q + 1);

        // Non-OpenType italic-correction / list-tag bump: `cur_f`/`cur_i`/
        // `cur_style` are scanner reads, the `*_base`/`font_info` lookups are
        // font reads, and the only with_state-bearing thing here would be the
        // bridge — so a single disjoint mem/fonts/scanner borrow does it all.
        // `usingOpenType` is a pure C call on the layout-engine pointer, safe
        // under the borrow.
        crate::state::with_state_mut(|st| {
            let st = &mut *st;
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
                        st.mem.set_b16s0(q + 1, c);
                    }
                }
                delta = st.fonts.font_info_b32s1(
                    st.fonts.italic_base_entry(f) + st.scanner.cur_i.s1 as i32 / 4,
                );
            }
        });

        let x = clean_box(q + 1, cur_style());

        // OT-math glyph variant / assembly selection (bucket D): the loop runs
        // `measure_native_glyph` (with_state-bearing) between mem writes, so it
        // stays on bridge accessors with no live borrow held.
        if font_area_entry(cur_f()) == OTGR_FONT_FLAG
            && isOpenTypeMathFont(font_layout_engine_ptr(cur_f()))
        {
            let mut p = mem_b32s1(x + 5);
            if p != TEX_NULL
                && !is_char_node_global(p)
                && mem_b16s1(p) == WHATSIT_NODE
                && mem_b16s0(p) == GLYPH_NODE
            {
                'found: {
                    if cur_style() < TEXT_STYLE {
                        let mut h1 = get_ot_math_constant(cur_f(), DISPLAYOPERATORMINHEIGHT);
                        let size_h = (mem_b32s1(p + 3) + mem_b32s1(p + 2)) as f64 * 5.0 / 4.0;
                        if (h1 as f64) < size_h {
                            h1 = size_h as i32;
                        }
                        let c_glyph = mem_b16s1(p + 4);
                        let mut n: i32 = 0;
                        let mut h2: i32 = 0;
                        loop {
                            let g = get_ot_math_variant(cur_f(), c_glyph as i32, n, &mut h2, 0);
                            if h2 > 0 {
                                set_mem_b16s1(p + 4, g as u16);
                                crate::ext::measure_native_glyph(mem_node_addr(p), 1);
                            }
                            n += 1;
                            if h2 < 0 || h2 >= h1 {
                                break;
                            }
                        }
                        if h2 < 0 {
                            let assembly_ptr = get_ot_assembly_ptr(cur_f(), c_glyph as i32, 0);
                            if !assembly_ptr.is_null() {
                                ot_assembly_ptr = assembly_ptr;
                                free_node_global(p, GLYPH_NODE_SIZE);
                                p = build_opentype_assembly(cur_f(), ot_assembly_ptr, h1, false);
                                set_mem_b32s1(x + 5, p);
                                delta = 0;
                                break 'found;
                            }
                        } else {
                            crate::ext::measure_native_glyph(mem_node_addr(p), 1);
                        }
                    }
                    delta = get_ot_math_ital_corr(cur_f(), mem_b16s1(p + 4) as i32);
                }
                // found:
                set_mem_b32s1(x + 1, mem_b32s1(p + 1));
                set_mem_b32s1(x + 3, mem_b32s1(p + 3));
                set_mem_b32s1(x + 2, mem_b32s1(p + 2));
            }
        }

        // Finish the MATH_CHAR box: `axis_height` is the threaded font helper,
        // so the writes + `axis_height` run together under one disjoint borrow.
        let size = cur_size();
        let ah = axis_height_global(size);
        crate::state::with_state_mut(|st| {
            let st = &mut *st;
            if st.mem.b32s1(q + 3) != EMPTY && st.mem.b16s0(q) != LIMITS {
                let v = st.mem.b32s1(x + 1) - delta;
                st.mem.set_b32s1(x + 1, v);
            }
            let v = crate::scaled_math::half(st.mem.b32s1(x + 3) - st.mem.b32s1(x + 2)) - ah;
            st.mem.set_b32s1(x + 4, v);
            st.mem.set_b32s1(q + 1, SUB_BOX);
            st.mem.set_b32s0(q + 1, x);
        });
    }

    let save_f = cur_f();
    if mem_b16s0(q) == LIMITS {
        let cs = cur_style() as i32;
        let mut x = clean_box(q + 2, (2 * (cs / 4) + 4 + (cs % 2)) as i16);
        let mut y = clean_box(q + 1, cur_style());
        let mut z = clean_box(q + 3, (2 * (cs / 4) + 5) as i16);

        // Allocate the wrapper vbox and set its width to the max of x/y/z.
        let (v, w) = crate::state::with_state_mut(|st| {
            let v = new_null_box(&mut st.mem);
            if v == ALLOC_FAILED {
                return (v, 0);
            }
            st.mem.set_b16s1(v, VLIST_NODE);
            let mut w = st.mem.b32s1(y + 1);
            if st.mem.b32s1(x + 1) > w {
                w = st.mem.b32s1(x + 1);
            }
            if st.mem.b32s1(z + 1) > w {
                w = st.mem.b32s1(z + 1);
            }
            st.mem.set_b32s1(v + 1, w);
            (v, w)
        });
        if v == ALLOC_FAILED {
            node_recycle_abort(NodeRecycleAbort::Overflow);
        }

        x = rebox_global(x, w);
        y = rebox_global(y, w);
        z = rebox_global(z, w);

        // The LIMITS tail is pure mem + threaded `big_op_spacing*` (font reads)
        // + threaded allocators, so it runs under one disjoint mem/fonts/eqtb/
        // scanner borrow. `cur_size` is precomputed for the spacing reads;
        // allocator exhaustion returns `Overflow` for the post-borrow abort.
        let size = cur_size();
        let (bos1, bos2, bos3, bos4, bos5) = crate::state::with_state(|st| {
            (
                big_op_spacing1(&st.fonts, &st.eqtb, size),
                big_op_spacing2(&st.fonts, &st.eqtb, size),
                big_op_spacing3(&st.fonts, &st.eqtb, size),
                big_op_spacing4(&st.fonts, &st.eqtb, size),
                big_op_spacing5(&st.fonts, &st.eqtb, size),
            )
        });
        let step = crate::state::with_state_mut(|st| -> Result<(), NodeRecycleAbort> {
            let st = &mut *st;
            let half_delta = crate::scaled_math::half(delta);
            st.mem.set_b32s1(x + 4, half_delta);
            st.mem.set_b32s1(z + 4, -half_delta);
            let y3 = st.mem.b32s1(y + 3);
            st.mem.set_b32s1(v + 3, y3);
            let y2 = st.mem.b32s1(y + 2);
            st.mem.set_b32s1(v + 2, y2);
            st.scanner.cur_f = save_f;

            if st.mem.b32s1(q + 2) == EMPTY {
                free_node(&mut st.mem, x, BOX_NODE_SIZE);
                st.mem.set_b32s1(v + 5, y);
            } else {
                let mut shift_up = bos3 - st.mem.b32s1(x + 2);
                if shift_up < bos1 {
                    shift_up = bos1;
                }
                let p = new_kern(&mut st.mem, shift_up);
                if p == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow);
                }
                st.mem.set_b32s1(p, y);
                st.mem.set_b32s1(x, p);
                let p2 = new_kern(&mut st.mem, bos5);
                if p2 == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow);
                }
                st.mem.set_b32s1(p2, x);
                st.mem.set_b32s1(v + 5, p2);
                let v3 = st.mem.b32s1(v + 3)
                    + bos5
                    + st.mem.b32s1(x + 3)
                    + st.mem.b32s1(x + 2)
                    + shift_up;
                st.mem.set_b32s1(v + 3, v3);
            }

            if st.mem.b32s1(q + 3) == EMPTY {
                free_node(&mut st.mem, z, BOX_NODE_SIZE);
            } else {
                let mut shift_down = bos4 - st.mem.b32s1(z + 3);
                if shift_down < bos2 {
                    shift_down = bos2;
                }
                let p = new_kern(&mut st.mem, shift_down);
                if p == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow);
                }
                st.mem.set_b32s1(y, p);
                st.mem.set_b32s1(p, z);
                let p2 = new_kern(&mut st.mem, bos5);
                if p2 == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow);
                }
                st.mem.set_b32s1(z, p2);
                let v2 = st.mem.b32s1(v + 2)
                    + bos5
                    + st.mem.b32s1(z + 3)
                    + st.mem.b32s1(z + 2)
                    + shift_down;
                st.mem.set_b32s1(v + 2, v2);
            }

            st.mem.set_b32s1(q + 1, v);
            Ok(())
        });
        if step.is_err() {
            node_recycle_abort(NodeRecycleAbort::Overflow);
        }
    }

    free_ot_assembly(ot_assembly_ptr);
    delta
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
    q: i32,
    p: i32,
) -> MakeOrdStep {
    let mut a = fonts.lig_kern_base_entry(scanner.cur_f) + scanner.cur_i.s0 as i32;
    scanner.cur_c = mem.b16s0(p + 1) as i32;
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
                let p_kern = new_kern(
                    mem,
                    fonts.font_info_b32s1(
                        fonts.kern_base_entry(scanner.cur_f)
                            + 256 * scanner.cur_i.s1 as i32
                            + scanner.cur_i.s0 as i32,
                    ),
                );
                if p_kern == ALLOC_FAILED {
                    return MakeOrdStep::Overflow;
                }
                let link_q = mem.b32s1(q);
                mem.set_b32s1(p_kern, link_q);
                mem.set_b32s1(q, p_kern);
                return MakeOrdStep::Return;
            } else {
                match scanner.cur_i.s1 {
                    1 | 5 => {
                        mem.set_b16s0(q + 1, scanner.cur_i.s0);
                    }
                    2 | 6 => {
                        mem.set_b16s0(p + 1, scanner.cur_i.s0);
                    }
                    3 | 7 | 11 => {
                        let r = new_noad(mem);
                        if r == ALLOC_FAILED {
                            return MakeOrdStep::Overflow;
                        }
                        mem.set_b16s0(r + 1, scanner.cur_i.s0);
                        mem.set_b16s1(r + 1, mem.b16s1(q + 1) % 256);
                        mem.set_b32s1(q, r);
                        mem.set_b32s1(r, p);
                        if scanner.cur_i.s1 < 11 {
                            mem.set_b32s1(r + 1, MATH_CHAR);
                        } else {
                            mem.set_b32s1(r + 1, MATH_TEXT_CHAR);
                        }
                    }
                    _ => {
                        let link_p = mem.b32s1(p);
                        mem.set_b32s1(q, link_p);
                        mem.set_b16s0(q + 1, scanner.cur_i.s0);
                        let w3 = mem.word(p + 3);
                        mem.set_word(q + 3, w3);
                        let w2 = mem.word(p + 2);
                        mem.set_word(q + 2, w2);
                        free_node(mem, p, NOAD_SIZE);
                    }
                }
                if scanner.cur_i.s1 > 3 {
                    return MakeOrdStep::Return;
                }
                mem.set_b32s1(q + 1, MATH_CHAR);
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
pub unsafe fn make_ord_global(q: i32) {
    'restart: loop {
        if mem_b32s1(q + 3) != EMPTY {
            return;
        }
        if mem_b32s1(q + 2) != EMPTY {
            return;
        }
        if mem_b32s1(q + 1) != MATH_CHAR {
            return;
        }
        let p = mem_b32s1(q);
        if p == TEX_NULL {
            return;
        }
        if mem_b16s1(p) < ORD_NOAD || mem_b16s1(p) > PUNCT_NOAD {
            return;
        }
        if mem_b32s1(p + 1) != MATH_CHAR {
            return;
        }
        if mem_b16s1(p + 1) % 256 != mem_b16s1(q + 1) % 256 {
            return;
        }

        set_mem_b32s1(q + 1, MATH_TEXT_CHAR);
        fetch_global(q + 1);
        if cur_i().s1 % 4 != LIG_TAG {
            return;
        }

        match crate::state::with_state_mut(|st| {
            let st = &mut *st;
            make_ord_body(&mut st.mem, &st.fonts, &mut st.scanner, q, p)
        }) {
            MakeOrdStep::Return => return,
            MakeOrdStep::Restart => continue 'restart,
            MakeOrdStep::Overflow => node_recycle_abort(NodeRecycleAbort::Overflow),
        }
    }
}

/// translated from xetex-math.c:79 (make_math_accent)
pub unsafe fn make_math_accent_global(q: i32) {
    fetch_global(q + 4);
    let mut x: i32;
    let mut ot_assembly_ptr: *mut libc::c_void = std::ptr::null_mut();
    let mut c: i32;
    let f: i32;
    let mut s: i32;
    let w: i32;
    let mut h: i32;

    let (initial_f, initial_c, initial_i, initial_area, initial_kind) =
        crate::state::with_state(|st| {
            (
                st.scanner.cur_f,
                st.scanner.cur_c,
                st.scanner.cur_i,
                st.fonts.font_area_entry(st.scanner.cur_f),
                st.mem.b16s0(q),
            )
        });

    if initial_area == AAT_FONT_FLAG || initial_area == OTGR_FONT_FLAG {
        c = initial_c;
        f = initial_f;
        if !(initial_kind == BOTTOM_ACC || initial_kind == BOTTOM_ACC + 1) {
            s = compute_ot_math_accent_pos_global(q);
        } else {
            s = 0;
        }
        let clean_style = crate::state::with_state(|st| 2 * (st.scanner.cur_style / 2) + 1);
        x = clean_box(q + 1, clean_style);
        (w, h) = crate::state::with_state(|st| (st.mem.b32s1(x + 1), st.mem.b32s1(x + 3)));
    } else if initial_i.s3 > 0 {
        let mut acc_i = initial_i;
        c = initial_c;
        f = initial_f;
        s = 0;
        let base_is_math_char = crate::state::with_state(|st| st.mem.b32s1(q + 1) == MATH_CHAR);
        if base_is_math_char {
            fetch_global(q + 1);
            s = crate::state::with_state_mut(|st| {
                let st = &mut *st;
                if st.scanner.cur_i.s1 % 4 != LIG_TAG {
                    return 0;
                }
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
                            return st.fonts.font_info_b32s1(
                                st.fonts.kern_base_entry(st.scanner.cur_f)
                                    + 256 * st.scanner.cur_i.s1 as i32
                                    + st.scanner.cur_i.s0 as i32,
                            );
                        }
                        return 0;
                    }
                    if st.scanner.cur_i.s3 >= 128 {
                        return 0;
                    }
                    a += st.scanner.cur_i.s3 as i32 + 1;
                    st.scanner.cur_i = st.fonts.font_info_b16(a);
                }
            });
        }
        let clean_style = crate::state::with_state(|st| 2 * (st.scanner.cur_style / 2) + 1);
        x = clean_box(q + 1, clean_style);
        (w, h) = crate::state::with_state(|st| (st.mem.b32s1(x + 1), st.mem.b32s1(x + 3)));
        loop {
            if acc_i.s1 % 4 != LIST_TAG {
                break;
            }
            let y_tmp = acc_i.s0 as i32;
            acc_i = crate::state::with_state(|st| {
                st.fonts.font_info_b16(st.fonts.char_base_entry(f) + y_tmp)
            });
            if acc_i.s3 <= 0 {
                break;
            }
            let acc_width = crate::state::with_state(|st| {
                st.fonts
                    .font_info_b32s1(st.fonts.width_base_entry(f) + acc_i.s3 as i32)
            });
            if acc_width > w {
                break;
            }
            c = y_tmp;
        }
        // done:
    } else {
        return;
    }

    if x == TEX_NULL {
        free_ot_assembly(ot_assembly_ptr);
        return;
    }

    let mut delta: i32;
    if font_area_entry(f) == OTGR_FONT_FLAG && isOpenTypeMathFont(font_layout_engine_ptr(f)) {
        if mem_b16s0(q) == BOTTOM_ACC || mem_b16s0(q) == BOTTOM_ACC + 1 {
            delta = 0;
        } else if h < get_ot_math_constant(f, ACCENTBASEHEIGHT) {
            delta = h;
        } else {
            delta = get_ot_math_constant(f, ACCENTBASEHEIGHT);
        }
    } else {
        let x_height = font_info_b32s1(X_HEIGHT_CODE + param_base(f));
        delta = if h < x_height { h } else { x_height };
    }

    if (mem_b32s1(q + 2) != EMPTY || mem_b32s1(q + 3) != EMPTY) && mem_b32s1(q + 1) == MATH_CHAR {
        crate::packaging::flush_node_list_global(x);
        x = new_noad_global();
        set_mem_word(x + 1, mem_word(q + 1));
        set_mem_word(x + 2, mem_word(q + 2));
        set_mem_word(x + 3, mem_word(q + 3));
        set_mem_b32s0(q + 2, 0);
        set_mem_b32s1(q + 2, 0);
        set_mem_b32s0(q + 3, 0);
        set_mem_b32s1(q + 3, 0);
        set_mem_b32s1(q + 1, SUB_MLIST);
        set_mem_b32s0(q + 1, x);
        x = clean_box(q + 1, cur_style());
        delta += mem_b32s1(x + 3) - h;
        h = mem_b32s1(x + 3);
    }

    let y = char_box_global(f, c);
    let mut p: i32;
    if font_area_entry(f) == AAT_FONT_FLAG || font_area_entry(f) == OTGR_FONT_FLAG {
        p = get_node_global(GLYPH_NODE_SIZE);
        set_mem_b16s1(p, WHATSIT_NODE);
        set_mem_b16s0(p, GLYPH_NODE);
        set_mem_b16s2(p + 4, f as u16);
        set_mem_b16s1(
            p + 4,
            real_get_native_glyph(mem_node_addr(mem_b32s1(y + 5)), 0),
        );
        crate::ext::measure_native_glyph(mem_node_addr(p), 1);
        let old_node = mem_b32s1(y + 5);
        free_node_global(old_node, mem_b16s3(old_node + 4) as i32);
        set_mem_b32s1(y + 5, p);
        if !mem_b16s0(q).is_multiple_of(2) {
            crate::ext::measure_native_glyph(mem_node_addr(p), 1);
        } else {
            let c_glyph = mem_b16s1(p + 4) as i32;
            let mut a_var: i32 = 0;
            let mut w2: i32 = 0;
            loop {
                let g = get_ot_math_variant(f, c_glyph, a_var, &mut w2, 1);
                if w2 > 0 && w2 <= w {
                    set_mem_b16s1(p + 4, g as u16);
                    crate::ext::measure_native_glyph(mem_node_addr(p), 1);
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
                        free_node_global(p, GLYPH_NODE_SIZE);
                        p = build_opentype_assembly(f, ot_assembly_ptr, w, true);
                        set_mem_b32s1(y + 5, p);
                        break 'found;
                    }
                } else {
                    crate::ext::measure_native_glyph(mem_node_addr(p), 1);
                }
            }
            // found:
        }
        set_mem_b32s1(y + 1, mem_b32s1(p + 1));
        set_mem_b32s1(y + 3, mem_b32s1(p + 3));
        set_mem_b32s1(y + 2, mem_b32s1(p + 2));
        if mem_b16s0(q) == BOTTOM_ACC || mem_b16s0(q) == BOTTOM_ACC + 1 {
            if mem_b32s1(y + 3) < 0 {
                set_mem_b32s1(y + 3, 0);
            }
        } else if mem_b32s1(y + 2) < 0 {
            set_mem_b32s1(y + 2, 0);
        }
        let sa: i32;
        if p != TEX_NULL
            && !is_char_node_global(p)
            && mem_b16s1(p) == WHATSIT_NODE
            && mem_b16s0(p) == GLYPH_NODE
        {
            sa = get_ot_math_accent_pos(f, mem_b16s1(p + 4) as i32);
            let sa = if sa == TEX_INFINITY {
                crate::scaled_math::half(mem_b32s1(y + 1))
            } else {
                sa
            };
            if mem_b16s0(q) == BOTTOM_ACC || mem_b16s0(q) == BOTTOM_ACC + 1 || s == TEX_INFINITY {
                s = crate::scaled_math::half(w);
            }
            set_mem_b32s1(y + 4, s - sa);
        } else {
            let sa = crate::scaled_math::half(mem_b32s1(y + 1));
            if mem_b16s0(q) == BOTTOM_ACC || mem_b16s0(q) == BOTTOM_ACC + 1 || s == TEX_INFINITY {
                s = crate::scaled_math::half(w);
            }
            set_mem_b32s1(y + 4, s - sa);
        }
    } else {
        set_mem_b32s1(y + 4, s + crate::scaled_math::half(w - mem_b32s1(y + 1)));
        p = TEX_NULL;
    }

    set_mem_b32s1(y + 1, 0);
    if mem_b16s0(q) == BOTTOM_ACC || mem_b16s0(q) == BOTTOM_ACC + 1 {
        set_mem_b32s1(x, y);
        let y = crate::packaging::vpackage(x, 0, ADDITIONAL, MAX_HALFWORD);
        set_mem_b32s1(y + 4, -(h - mem_b32s1(y + 3)));
        set_mem_b32s1(y + 1, mem_b32s1(x + 1));
        set_mem_b32s0(q + 1, y);
        set_mem_b32s1(q + 1, SUB_BOX);
    } else {
        p = new_kern_global(-delta);
        set_mem_b32s1(p, x);
        set_mem_b32s1(y, p);
        let y = crate::packaging::vpackage(y, 0, ADDITIONAL, MAX_HALFWORD);
        if mem_b32s1(y + 3) < h {
            p = new_kern_global(h - mem_b32s1(y + 3));
            set_mem_b32s1(p, mem_b32s1(y + 5));
            set_mem_b32s1(y + 5, p);
            set_mem_b32s1(y + 3, h);
        }
        set_mem_b32s1(y + 1, mem_b32s1(x + 1));
        set_mem_b32s0(q + 1, y);
        set_mem_b32s1(q + 1, SUB_BOX);
    }

    free_ot_assembly(ot_assembly_ptr);
}

/// translated from xetex-math.c:275 (attach_hkern_to_new_hlist)
/// Threaded core (Phase C, bucket B): builds a kern and appends it to the
/// `new_hlist` of noad `q`, all in `mem`. `new_kern` exhaustion propagates the
/// `01b` §5 [`NodeRecycleAbort::Overflow`] sentinel; `attach_hkern_to_new_hlist_global`
/// aborts after the borrow drops.
fn attach_hkern_to_new_hlist(
    mem: &mut MemState,
    q: i32,
    delta: i32,
) -> Result<i32, NodeRecycleAbort> {
    let z = new_kern(mem, delta);
    if z == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    if mem.b32s1(q + 1) == TEX_NULL {
        mem.set_b32s1(q + 1, z);
    } else {
        let mut y = mem.b32s1(q + 1);
        while mem.b32s1(y) != TEX_NULL {
            y = mem.b32s1(y);
        }
        mem.set_b32s1(y, z);
    }
    Ok(mem.b32s1(q + 1))
}

unsafe fn attach_hkern_to_new_hlist_global(q: i32, delta: i32) -> i32 {
    match crate::state::with_state_mut(|st| attach_hkern_to_new_hlist(&mut st.mem, q, delta)) {
        Ok(p) => p,
        Err(e) => node_recycle_abort(e),
    }
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
        && mem.b16s0(p) == GLYPH_NODE
    {
        Some(ScriptGlyphNucleus {
            font: mem.b16s2(p + 4) as i32,
            glyph: mem.b16s1(p + 4),
        })
    } else {
        None
    }
}

unsafe fn maybe_attach_script_kern_global(
    q: i32,
    p: i32,
    script_f: i32,
    script_g: u16,
    cmd: i32,
    shift: i32,
) -> ScriptKernAttachment {
    let (cur_f, nucleus) =
        crate::state::with_state(|st| (st.scanner.cur_f, glyph_nucleus(&st.mem, p)));

    if !font_is_ot_math_global(cur_f) {
        return ScriptKernAttachment {
            p,
            kern: 0,
            is_ot_math: false,
        };
    }

    let Some(nucleus) = nucleus else {
        return ScriptKernAttachment {
            p,
            kern: 0,
            is_ot_math: true,
        };
    };

    let kern = get_ot_math_kern(nucleus.font, nucleus.glyph, script_f, script_g, cmd, shift);
    ScriptKernAttachment {
        p: if kern == 0 {
            p
        } else {
            attach_hkern_to_new_hlist_global(q, kern)
        },
        kern,
        is_ot_math: true,
    }
}

fn add_script_space_and_restore_font(
    mem: &mut MemState,
    eqtb: &EqtbState,
    scanner: &mut ScannerState,
    box_node: i32,
    saved_font: i32,
) {
    scanner.cur_f = saved_font;
    mem.set_b32s1(
        box_node + 1,
        mem.b32s1(box_node + 1) + eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_SCRIPT_SPACE),
    );
}

#[derive(Clone, Copy)]
enum ScriptBoxAdjustment {
    Sub1,
    Sub2,
    Superscript,
}

struct ScriptBoxPrep {
    node: i32,
    shift: i32,
}

unsafe fn clean_script_box_global(
    script_field: i32,
    clean_style: i16,
    saved_font: i32,
    adjustment: ScriptBoxAdjustment,
) -> ScriptBoxPrep {
    let node = clean_box(script_field, clean_style);
    let shift = crate::state::with_state(|st| match adjustment {
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
    });
    crate::state::with_state_mut(|st| {
        add_script_space_and_restore_font(&mut st.mem, &st.eqtb, &mut st.scanner, node, saved_font);
    });

    ScriptBoxPrep { node, shift }
}

struct ScriptShifts {
    up: i32,
    down: i32,
}

unsafe fn initial_script_shifts_global(p: i32) -> ScriptShifts {
    if crate::state::with_state(|st| {
        crate::mem::is_char_node(&st.mem, p) || glyph_nucleus(&st.mem, p).is_some()
    }) {
        ScriptShifts { up: 0, down: 0 }
    } else {
        let z = crate::packaging::hpack(p, 0, ADDITIONAL);
        let shifts = crate::state::with_state(|st| {
            let t = if st.scanner.cur_style < SCRIPT_STYLE {
                SCRIPT_SIZE
            } else {
                SCRIPT_SCRIPT_SIZE
            };
            ScriptShifts {
                up: st.mem.b32s1(z + 3) - sup_drop(&st.fonts, &st.eqtb, t),
                down: st.mem.b32s1(z + 2) + sub_drop(&st.fonts, &st.eqtb, t),
            }
        });
        crate::state::with_state_mut(|st| free_node(&mut st.mem, z, BOX_NODE_SIZE));
        shifts
    }
}

fn append_script_box(mem: &mut MemState, q: i32, box_node: i32) {
    if mem.b32s1(q + 1) == TEX_NULL {
        mem.set_b32s1(q + 1, box_node);
    } else {
        let mut p = mem.b32s1(q + 1);
        while mem.b32s1(p) != TEX_NULL {
            p = mem.b32s1(p);
        }
        mem.set_b32s1(p, box_node);
    }
}

unsafe fn append_script_box_global(q: i32, box_node: i32) {
    crate::state::with_state_mut(|st| append_script_box(&mut st.mem, q, box_node));
}

struct SubSupStackInput {
    q: i32,
    p: i32,
    x: i32,
    y: i32,
    sub_f: i32,
    sub_g: u16,
    sup_f: i32,
    sup_g: u16,
    shift_down: i32,
    shift_up: i32,
    delta: i32,
}

unsafe fn finish_subsup_stack_global(input: SubSupStackInput) {
    let mut p = input.p;
    let mut sub_kern;
    let mut sup_kern = 0;
    let attachment = maybe_attach_script_kern_global(
        input.q,
        p,
        input.sub_f,
        input.sub_g,
        SUB_CMD,
        input.shift_down,
    );
    p = attachment.p;
    sub_kern = attachment.kern;
    if attachment.is_ot_math {
        let sub_empty = crate::state::with_state(|st| st.mem.b32s1(input.q + 3) == EMPTY);
        if sub_empty {
            let attachment = maybe_attach_script_kern_global(
                input.q,
                p,
                input.sup_f,
                input.sup_g,
                SUP_CMD,
                input.shift_up,
            );
            sup_kern = attachment.kern;
        }
    } else {
        sup_kern = 0;
        sub_kern = 0;
    }

    let kern_width = crate::state::with_state(|st| {
        (input.shift_up - st.mem.b32s1(input.x + 2))
            - (st.mem.b32s1(input.y + 3) - input.shift_down)
    });
    let kern = crate::state::with_state_mut(|st| {
        st.mem
            .set_b32s1(input.x + 4, sup_kern + input.delta - sub_kern);
        let kern = new_kern(&mut st.mem, kern_width);
        if kern != ALLOC_FAILED {
            st.mem.set_b32s1(input.x, kern);
            st.mem.set_b32s1(kern, input.y);
        }
        kern
    });
    if kern == ALLOC_FAILED {
        node_recycle_abort(NodeRecycleAbort::Overflow);
    }

    let packaged = crate::packaging::vpackage(input.x, 0, ADDITIONAL, MAX_HALFWORD);
    crate::state::with_state_mut(|st| st.mem.set_b32s1(packaged + 4, input.shift_down));
    append_script_box_global(input.q, packaged);
}

struct PreparedScript {
    node: i32,
    glyph: u16,
    font: i32,
    shift: i32,
}

unsafe fn prepare_subscript_global(
    script_field: i32,
    adjustment: ScriptBoxAdjustment,
    mut shift_down: i32,
    saved_font_before_glyph_probe: Option<i32>,
) -> PreparedScript {
    let (save_f, glyph, font) = if let Some(save_f) = saved_font_before_glyph_probe {
        let (glyph, font) = first_script_glyph_global(script_field);
        (save_f, glyph, font)
    } else {
        let (glyph, font) = first_script_glyph_global(script_field);
        let save_f = crate::state::with_state(|st| st.scanner.cur_f);
        (save_f, glyph, font)
    };
    let clean_style = crate::state::with_state(|st| 2 * (st.scanner.cur_style / 4) + 5);
    let prepared = clean_script_box_global(script_field, clean_style, save_f, adjustment);
    if shift_down < prepared.shift {
        shift_down = prepared.shift;
    }

    PreparedScript {
        node: prepared.node,
        glyph,
        font,
        shift: shift_down,
    }
}

unsafe fn finish_subscript_only_global(q: i32, p: i32, shift_down: i32) {
    let PreparedScript {
        node: x,
        glyph: sub_g,
        font: sub_f,
        shift: mut shift_down,
    } = prepare_subscript_global(q + 3, ScriptBoxAdjustment::Sub1, shift_down, None);

    let (cur_f, box_depth, x_height) = crate::state::with_state(|st| {
        (
            st.scanner.cur_f,
            st.mem.b32s1(x + 3),
            math_x_height(&st.fonts, &st.eqtb, st.scanner.cur_size),
        )
    });
    let clr = if font_is_ot_math_global(cur_f) {
        box_depth - get_ot_math_constant(cur_f, SUBSCRIPTTOPMAX)
    } else {
        box_depth - (x_height * 4).abs() / 5
    };

    if shift_down < clr {
        shift_down = clr;
    }

    crate::state::with_state_mut(|st| st.mem.set_b32s1(x + 4, shift_down));

    let _ = maybe_attach_script_kern_global(q, p, sub_f, sub_g, SUB_CMD, shift_down);
    append_script_box_global(q, x);
}

unsafe fn prepare_superscript_global(q: i32, mut shift_up: i32) -> PreparedScript {
    let (glyph, font) = first_script_glyph_global(q + 2);
    let (save_f, clean_style) = crate::state::with_state(|st| {
        (
            st.scanner.cur_f,
            2 * (st.scanner.cur_style / 4) + 4 + (st.scanner.cur_style % 2),
        )
    });
    let prepared =
        clean_script_box_global(q + 2, clean_style, save_f, ScriptBoxAdjustment::Superscript);
    if shift_up < prepared.shift {
        shift_up = prepared.shift;
    }

    let (cur_f, box_height, x_height) = crate::state::with_state(|st| {
        (
            st.scanner.cur_f,
            st.mem.b32s1(prepared.node + 2),
            math_x_height(&st.fonts, &st.eqtb, st.scanner.cur_size),
        )
    });
    let clr = if font_is_ot_math_global(cur_f) {
        box_height + get_ot_math_constant(cur_f, SUPERSCRIPTBOTTOMMIN)
    } else {
        box_height + x_height.abs() / 4
    };

    if shift_up < clr {
        shift_up = clr;
    }

    PreparedScript {
        node: prepared.node,
        glyph,
        font,
        shift: shift_up,
    }
}

unsafe fn finish_superscript_branch_global(
    q: i32,
    p: i32,
    superscript: PreparedScript,
    mut shift_down: i32,
    delta: i32,
) {
    let x = superscript.node;
    let shift_up = superscript.shift;
    let sub_empty = crate::state::with_state(|st| st.mem.b32s1(q + 3) == EMPTY);
    if sub_empty {
        let _ = maybe_attach_script_kern_global(
            q,
            p,
            superscript.font,
            superscript.glyph,
            SUP_CMD,
            shift_up,
        );
        crate::state::with_state_mut(|st| st.mem.set_b32s1(x + 4, -shift_up));
        append_script_box_global(q, x);
        return;
    }

    let save_f = crate::state::with_state(|st| st.scanner.cur_f);
    let PreparedScript {
        node: y,
        glyph: sub_g,
        font: sub_f,
        shift,
    } = prepare_subscript_global(q + 3, ScriptBoxAdjustment::Sub2, shift_down, Some(save_f));
    shift_down = shift;

    let (cur_f, x_height_node, y_depth, default_rule_thickness) = crate::state::with_state(|st| {
        (
            st.scanner.cur_f,
            st.mem.b32s1(x + 2),
            st.mem.b32s1(y + 3),
            default_rule_thickness(&st.fonts, &st.eqtb, st.scanner.cur_size),
        )
    });
    let mut clr = if font_is_ot_math_global(cur_f) {
        get_ot_math_constant(cur_f, SUBSUPERSCRIPTGAPMIN)
            - ((shift_up - x_height_node) - (y_depth - shift_down))
    } else {
        4 * default_rule_thickness - ((shift_up - x_height_node) - (y_depth - shift_down))
    };

    let mut shift_up = shift_up;
    if clr > 0 {
        shift_down += clr;
        let (cur_f, x_height_node, x_height) = crate::state::with_state(|st| {
            (
                st.scanner.cur_f,
                st.mem.b32s1(x + 2),
                math_x_height(&st.fonts, &st.eqtb, st.scanner.cur_size),
            )
        });
        clr = if font_is_ot_math_global(cur_f) {
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

    finish_subsup_stack_global(SubSupStackInput {
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
    });
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
            match mem.b16s1(script_ptr) {
                KERN_NODE | GLUE_NODE | CHOICE_NODE => {}
                STYLE_NODE => {
                    this_math_style = mem.b16s0(script_ptr) as i16;
                }
                ORD_NOAD | OP_NOAD | BIN_NOAD | REL_NOAD | OPEN_NOAD | CLOSE_NOAD | PUNCT_NOAD => {
                    script_head = script_ptr + 1;
                    script_ptr = TEX_NULL;
                }
                _ => {
                    script_ptr = TEX_NULL;
                }
            }

            if script_ptr >= 0 && script_ptr <= mem.mem_end {
                if mem.b16s1(script_ptr) == CHOICE_NODE {
                    script_ptr = match this_math_style / 2 {
                        0 => mem.b32s0(script_ptr + 1),
                        1 => mem.b32s1(script_ptr + 1),
                        2 => mem.b32s0(script_ptr + 2),
                        3 => mem.b32s1(script_ptr + 2),
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

unsafe fn first_script_glyph_global(script_head: i32) -> (u16, i32) {
    let mut script_g: u16 = 0;
    let mut script_f: i32 = 0;

    let initial_math_style = crate::state::with_state(|st| 2 * (st.scanner.cur_style / 4) + 5);
    let (script_head, this_math_style, is_math_char) = crate::state::with_state(|st| {
        let (script_head, this_math_style) =
            first_script_glyph_step(&st.mem, script_head, initial_math_style);
        (
            script_head,
            this_math_style,
            script_head >= 0
                && script_head <= st.mem.mem_end
                && st.mem.b32s1(script_head) == MATH_CHAR,
        )
    });

    if is_math_char {
        let (save_f, saved_math_style) =
            crate::state::with_state(|st| (st.scanner.cur_f, st.scanner.cur_style));
        let cur_size_for_script = if this_math_style < SCRIPT_STYLE {
            TEXT_SIZE
        } else {
            SCRIPT_SIZE * ((this_math_style as i32 - 2) / 2)
        };
        let script_mu = crate::scaled_math::x_over_n(math_quad_global(cur_size_for_script), 18);
        crate::state::with_state_mut(|st| {
            st.scanner.cur_style = this_math_style;
            st.scanner.cur_size = cur_size_for_script;
            st.scanner.cur_mu = script_mu;
        });
        fetch_global(script_head);

        let (cur_f_after_fetch, cur_c_after_fetch) =
            crate::state::with_state(|st| (st.scanner.cur_f, st.scanner.cur_c));
        if font_is_ot_math_global(cur_f_after_fetch) {
            let script_c =
                crate::fonts::new_native_character(cur_f_after_fetch, cur_c_after_fetch as u32);
            script_g = real_get_native_glyph(mem_node_addr(script_c), 0);
            script_f = cur_f_after_fetch;
        }

        let restored_size = if saved_math_style < SCRIPT_STYLE {
            TEXT_SIZE
        } else {
            SCRIPT_SIZE * ((saved_math_style as i32 - 2) / 2)
        };
        let restored_mu = crate::scaled_math::x_over_n(math_quad_global(restored_size), 18);
        crate::state::with_state_mut(|st| {
            st.scanner.cur_f = save_f;
            st.scanner.cur_style = saved_math_style;
            st.scanner.cur_size = restored_size;
            st.scanner.cur_mu = restored_mu;
        });
    }

    (script_g, script_f)
}

/// translated from xetex-math.c:84 (make_scripts)
unsafe fn make_scripts_global(q: i32, delta: i32) {
    let p = crate::state::with_state(|st| st.mem.b32s1(q + 1));
    let shifts = initial_script_shifts_global(p);

    if crate::state::with_state(|st| st.mem.b32s1(q + 2)) == EMPTY {
        finish_subscript_only_global(q, p, shifts.down);
    } else {
        let superscript = prepare_superscript_global(q, shifts.up);
        finish_superscript_branch_global(q, p, superscript, shifts.down, delta);
    }
}

/// translated from xetex-math.c:759 (make_left_right)
pub unsafe fn make_left_right_global(q: i32, style: i16, max_d: i32, max_h: i32) -> i16 {
    let size = if style < SCRIPT_STYLE {
        TEXT_SIZE
    } else {
        SCRIPT_SIZE * ((style as i32 - 2) / 2)
    };
    let math_quad = crate::state::with_state(|st| math_quad(&st.fonts, &st.eqtb, size));
    let mu = crate::scaled_math::x_over_n(math_quad, 18);
    crate::state::with_state_mut(|st| {
        st.scanner.cur_style = style;
        st.scanner.cur_size = size;
        st.scanner.cur_mu = mu;
    });

    let (axis_height, delimiter_factor, delimiter_shortfall) = crate::state::with_state(|st| {
        (
            axis_height(&st.fonts, &st.eqtb, size),
            st.eqtb.b32s1(INT_BASE + INT_PAR_DELIMITER_FACTOR),
            st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_DELIMITER_SHORTFALL),
        )
    });
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
    let delimiter = var_delimiter_global(q + 1, size, delta);
    crate::state::with_state_mut(|st| {
        st.mem.set_b32s1(q + 1, delimiter);
        (st.mem.b16s1(q) as i32 - (LEFT_NOAD as i32 - 20)) as i16
    })
}

#[inline]
unsafe fn update_math_style_params() {
    if cur_style() < SCRIPT_STYLE {
        set_cur_size(TEXT_SIZE);
    } else {
        set_cur_size(SCRIPT_SIZE * ((cur_style() as i32 - 2) / 2));
    }
    set_cur_mu(crate::scaled_math::x_over_n(
        math_quad_global(cur_size()),
        18,
    ));
}

/// translated from xetex-math.c:84 (mlist_to_hlist)
#[no_mangle]
pub unsafe extern "C" fn mlist_to_hlist() {
    let mlist = crate::globals::cur_mlist();
    let penalties = crate::globals::mlist_penalties();
    let style = cur_style();
    let mut q = mlist;
    let mut r = TEX_NULL;
    let mut r_type = OP_NOAD;
    let mut max_h = 0;
    let mut max_d = 0;

    update_math_style_params();

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
            match mem_b16s1(q) {
                BIN_NOAD => {
                    match r_type {
                        BIN_NOAD | OP_NOAD | REL_NOAD | OPEN_NOAD | PUNCT_NOAD | LEFT_NOAD => {
                            set_mem_b16s1(q, ORD_NOAD);
                            continue 'reswitch;
                        }
                        _ => {}
                    }
                    break 'reswitch FirstPass::ProcessNucleus;
                }
                REL_NOAD | CLOSE_NOAD | PUNCT_NOAD | RIGHT_NOAD => {
                    if r_type == BIN_NOAD {
                        set_mem_b16s1(r, ORD_NOAD);
                    }
                    if mem_b16s1(q) == RIGHT_NOAD {
                        break 'reswitch FirstPass::DoneWithNoad;
                    }
                    break 'reswitch FirstPass::ProcessNucleus;
                }
                LEFT_NOAD => break 'reswitch FirstPass::DoneWithNoad,
                FRACTION_NOAD => {
                    make_fraction_global(q);
                    break 'reswitch FirstPass::CheckDimensions;
                }
                OP_NOAD => {
                    delta = make_op_global(q);
                    if mem_b16s0(q) == LIMITS {
                        break 'reswitch FirstPass::CheckDimensions;
                    }
                    break 'reswitch FirstPass::ProcessNucleus;
                }
                ORD_NOAD => {
                    make_ord_global(q);
                    break 'reswitch FirstPass::ProcessNucleus;
                }
                OPEN_NOAD | INNER_NOAD => break 'reswitch FirstPass::ProcessNucleus,
                RADICAL_NOAD => {
                    make_radical_global(q);
                    break 'reswitch FirstPass::ProcessNucleus;
                }
                OVER_NOAD => {
                    make_over_global(q);
                    break 'reswitch FirstPass::ProcessNucleus;
                }
                UNDER_NOAD => {
                    make_under_global(q);
                    break 'reswitch FirstPass::ProcessNucleus;
                }
                ACCENT_NOAD => {
                    make_math_accent_global(q);
                    break 'reswitch FirstPass::ProcessNucleus;
                }
                VCENTER_NOAD => {
                    make_vcenter_global(q);
                    break 'reswitch FirstPass::ProcessNucleus;
                }
                STYLE_NODE => {
                    set_cur_style(mem_b16s0(q) as i16);
                    update_math_style_params();
                    break 'reswitch FirstPass::DoneWithNode;
                }
                CHOICE_NODE => {
                    let mut p = match cur_style() / 2 {
                        0 => {
                            let p = mem_b32s0(q + 1);
                            set_mem_b32s0(q + 1, TEX_NULL);
                            p
                        }
                        1 => {
                            let p = mem_b32s1(q + 1);
                            set_mem_b32s1(q + 1, TEX_NULL);
                            p
                        }
                        2 => {
                            let p = mem_b32s0(q + 2);
                            set_mem_b32s0(q + 2, TEX_NULL);
                            p
                        }
                        3 => {
                            let p = mem_b32s1(q + 2);
                            set_mem_b32s1(q + 2, TEX_NULL);
                            p
                        }
                        _ => TEX_NULL,
                    };
                    crate::packaging::flush_node_list_global(mem_b32s0(q + 1));
                    crate::packaging::flush_node_list_global(mem_b32s1(q + 1));
                    crate::packaging::flush_node_list_global(mem_b32s0(q + 2));
                    crate::packaging::flush_node_list_global(mem_b32s1(q + 2));
                    set_mem_b16s1(q, STYLE_NODE);
                    set_mem_b16s0(q, cur_style() as u16);
                    set_mem_b32s1(q + 1, 0);
                    set_mem_b32s1(q + 2, 0);
                    if p != TEX_NULL {
                        let z = mem_b32s1(q);
                        set_mem_b32s1(q, p);
                        while mem_b32s1(p) != TEX_NULL {
                            p = mem_b32s1(p);
                        }
                        set_mem_b32s1(p, z);
                    }
                    break 'reswitch FirstPass::DoneWithNode;
                }
                INS_NODE | MARK_NODE | ADJUST_NODE | WHATSIT_NODE | PENALTY_NODE | DISC_NODE => {
                    break 'reswitch FirstPass::DoneWithNode;
                }
                RULE_NODE => {
                    if mem_b32s1(q + 3) > max_h {
                        max_h = mem_b32s1(q + 3);
                    }
                    if mem_b32s1(q + 2) > max_d {
                        max_d = mem_b32s1(q + 2);
                    }
                    break 'reswitch FirstPass::DoneWithNode;
                }
                GLUE_NODE => {
                    if mem_b16s0(q) == MU_GLUE {
                        let x = mem_b32s0(q + 1);
                        let y = math_glue_global(x, cur_mu());
                        crate::mem::delete_glue_ref_global(x);
                        set_mem_b32s0(q + 1, y);
                        set_mem_b16s0(q, NORMAL);
                    } else if cur_size() != TEXT_SIZE && mem_b16s0(q) == COND_MATH_GLUE {
                        let p = mem_b32s1(q);
                        if p != TEX_NULL && (mem_b16s1(p) == GLUE_NODE || mem_b16s1(p) == KERN_NODE)
                        {
                            set_mem_b32s1(q, mem_b32s1(p));
                            set_mem_b32s1(p, TEX_NULL);
                            crate::packaging::flush_node_list_global(p);
                        }
                    }
                    break 'reswitch FirstPass::DoneWithNode;
                }
                KERN_NODE => {
                    math_kern_global(q, cur_mu());
                    break 'reswitch FirstPass::DoneWithNode;
                }
                _ => crate::errors::confusion(c"mlist1".as_ptr()),
            }
        };

        match action {
            FirstPass::ProcessNucleus => {
                let p = match mem_b32s1(q + 1) {
                    MATH_CHAR | MATH_TEXT_CHAR => {
                        fetch_global(q + 1);
                        let p;
                        if font_area_entry(cur_f()) == AAT_FONT_FLAG
                            || font_area_entry(cur_f()) == OTGR_FONT_FLAG
                        {
                            let z = crate::fonts::new_native_character(cur_f(), cur_c() as u32);
                            p = get_node_global(GLYPH_NODE_SIZE);
                            set_mem_b16s1(p, WHATSIT_NODE);
                            set_mem_b16s0(p, GLYPH_NODE);
                            set_mem_b16s2(p + 4, cur_f() as u16);
                            set_mem_b16s1(p + 4, real_get_native_glyph(mem_node_addr(z), 0));
                            crate::ext::measure_native_glyph(mem_node_addr(p), 1);
                            free_node_global(z, mem_b16s3(z + 4) as i32);
                            delta = get_ot_math_ital_corr(cur_f(), mem_b16s1(p + 4) as i32);
                            if mem_b32s1(q + 1) == MATH_TEXT_CHAR
                                && !font_is_ot_math_global(cur_f())
                            {
                                delta = 0;
                            }
                            if mem_b32s1(q + 3) == EMPTY && delta != 0 {
                                set_mem_b32s1(p, new_kern_global(delta));
                                delta = 0;
                            }
                        } else if cur_i().s3 > 0 {
                            delta =
                                font_info_b32s1(italic_base_entry(cur_f()) + cur_i().s1 as i32 / 4);
                            p = crate::main_control::new_character(cur_f(), cur_c() as u16);
                            if mem_b32s1(q + 1) == MATH_TEXT_CHAR
                                && font_info_b32s1(SPACE_CODE + param_base(cur_f())) != 0
                            {
                                delta = 0;
                            }
                            if mem_b32s1(q + 3) == EMPTY && delta != 0 {
                                set_mem_b32s1(p, new_kern_global(delta));
                                delta = 0;
                            }
                        } else {
                            p = TEX_NULL;
                        }
                        p
                    }
                    EMPTY => TEX_NULL,
                    SUB_BOX => mem_b32s0(q + 1),
                    SUB_MLIST => {
                        set_cur_mlist(mem_b32s0(q + 1));
                        let save_style = cur_style();
                        set_mlist_penalties(false);
                        mlist_to_hlist();
                        set_cur_style(save_style);
                        update_math_style_params();
                        crate::packaging::hpack(mem_b32s1(TEMP_HEAD), 0, ADDITIONAL)
                    }
                    _ => crate::errors::confusion(c"mlist2".as_ptr()),
                };
                set_mem_b32s1(q + 1, p);
                if mem_b32s1(q + 3) != EMPTY || mem_b32s1(q + 2) != EMPTY {
                    make_scripts_global(q, delta);
                }

                let z = crate::packaging::hpack(mem_b32s1(q + 1), 0, ADDITIONAL);
                if mem_b32s1(z + 3) > max_h {
                    max_h = mem_b32s1(z + 3);
                }
                if mem_b32s1(z + 2) > max_d {
                    max_d = mem_b32s1(z + 2);
                }
                free_node_global(z, BOX_NODE_SIZE);

                r = q;
                r_type = mem_b16s1(r);
                if r_type == RIGHT_NOAD {
                    r_type = LEFT_NOAD;
                    set_cur_style(style);
                    update_math_style_params();
                }
                q = mem_b32s1(q);
            }
            FirstPass::CheckDimensions => {
                let z = crate::packaging::hpack(mem_b32s1(q + 1), 0, ADDITIONAL);
                if mem_b32s1(z + 3) > max_h {
                    max_h = mem_b32s1(z + 3);
                }
                if mem_b32s1(z + 2) > max_d {
                    max_d = mem_b32s1(z + 2);
                }
                free_node_global(z, BOX_NODE_SIZE);

                r = q;
                r_type = mem_b16s1(r);
                if r_type == RIGHT_NOAD {
                    r_type = LEFT_NOAD;
                    set_cur_style(style);
                    update_math_style_params();
                }
                q = mem_b32s1(q);
            }
            FirstPass::DoneWithNoad => {
                r = q;
                r_type = mem_b16s1(r);
                if r_type == RIGHT_NOAD {
                    r_type = LEFT_NOAD;
                    set_cur_style(style);
                    update_math_style_params();
                }
                q = mem_b32s1(q);
            }
            FirstPass::DoneWithNode => {
                q = mem_b32s1(q);
            }
        }
    }

    if r_type == BIN_NOAD {
        set_mem_b16s1(r, ORD_NOAD);
    }

    let mut p = TEMP_HEAD;
    set_mem_b32s1(p, TEX_NULL);
    q = mlist;
    r_type = 0;
    set_cur_style(style);
    update_math_style_params();

    while q != TEX_NULL {
        let mut t = ORD_NOAD;
        let mut s = NOAD_SIZE;
        let mut pen = INF_PENALTY;
        let delete_q = match mem_b16s1(q) {
            OP_NOAD | OPEN_NOAD | CLOSE_NOAD | PUNCT_NOAD | INNER_NOAD => {
                t = mem_b16s1(q);
                true
            }
            BIN_NOAD => {
                t = BIN_NOAD;
                pen = eqtb_b32s1(INT_BASE + INT_PAR_BIN_OP_PENALTY);
                true
            }
            REL_NOAD => {
                t = REL_NOAD;
                pen = eqtb_b32s1(INT_BASE + INT_PAR_REL_PENALTY);
                true
            }
            ORD_NOAD | VCENTER_NOAD | OVER_NOAD | UNDER_NOAD => true,
            RADICAL_NOAD => {
                s = RADICAL_NOAD_SIZE;
                true
            }
            ACCENT_NOAD => {
                s = ACCENT_NOAD_SIZE;
                true
            }
            FRACTION_NOAD => {
                s = FRACTION_NOAD_SIZE;
                true
            }
            LEFT_NOAD | RIGHT_NOAD => {
                t = make_left_right_global(q, style, max_d, max_h) as u16;
                true
            }
            STYLE_NODE => {
                set_cur_style(mem_b16s0(q) as i16);
                s = STYLE_NODE_SIZE;
                update_math_style_params();
                true
            }
            WHATSIT_NODE | PENALTY_NODE | RULE_NODE | DISC_NODE | ADJUST_NODE | INS_NODE
            | MARK_NODE | GLUE_NODE | KERN_NODE => {
                set_mem_b32s1(p, q);
                p = q;
                q = mem_b32s1(q);
                set_mem_b32s1(p, TEX_NULL);
                false
            }
            _ => crate::errors::confusion(c"mlist3".as_ptr()),
        };

        if !delete_q {
            continue;
        }

        if mem_b16s1(q) != STYLE_NODE {
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
                let spacing = offset_table[(r_type - ORD_NOAD) as usize][(t - ORD_NOAD) as usize];
                let x = match spacing {
                    b'0' => 0,
                    b'1' => {
                        if cur_style() < SCRIPT_STYLE {
                            GLUE_PAR_THIN_MU_SKIP
                        } else {
                            0
                        }
                    }
                    b'2' => GLUE_PAR_THIN_MU_SKIP,
                    b'3' => {
                        if cur_style() < SCRIPT_STYLE {
                            GLUE_PAR_MED_MU_SKIP
                        } else {
                            0
                        }
                    }
                    b'4' => {
                        if cur_style() < SCRIPT_STYLE {
                            GLUE_PAR_THICK_MU_SKIP
                        } else {
                            0
                        }
                    }
                    _ => crate::errors::confusion(c"mlist4".as_ptr()),
                };
                if x != 0 {
                    let y = math_glue_global(eqtb_b32s1(GLUE_BASE + x), cur_mu());
                    let z = new_glue_global(y);
                    set_mem_b32s1(y, TEX_NULL);
                    set_mem_b32s1(p, z);
                    p = z;
                    set_mem_b16s0(z, (x + 1) as u16);
                }
            }

            if mem_b32s1(q + 1) != TEX_NULL {
                set_mem_b32s1(p, mem_b32s1(q + 1));
                loop {
                    p = mem_b32s1(p);
                    if mem_b32s1(p) == TEX_NULL {
                        break;
                    }
                }
            }

            if penalties && mem_b32s1(q) != TEX_NULL && pen < INF_PENALTY {
                let next_type = mem_b16s1(mem_b32s1(q));
                if next_type != PENALTY_NODE && next_type != REL_NOAD {
                    let z = crate::mem::new_penalty_global(pen);
                    set_mem_b32s1(p, z);
                    p = z;
                }
            }

            if mem_b16s1(q) == RIGHT_NOAD {
                t = OPEN_NOAD;
            }
            r_type = t;
        }

        r = q;
        q = mem_b32s1(q);
        free_node_global(r, s);
    }
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
fn char_box(mem: &mut MemState, input: CharBoxInput) -> Result<i32, NodeRecycleAbort> {
    let b = new_null_box(mem);
    if b == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    let p = match input {
        CharBoxInput::Native { p } => {
            let h = mem.b32s1(p + 3);
            let w = mem.b32s1(p + 1);
            let d = mem.b32s1(p + 2);
            mem.set_b32s1(b + 3, h);
            mem.set_b32s1(b + 1, w);
            mem.set_b32s1(b + 2, if d < 0 { 0 } else { d });
            p
        }
        CharBoxInput::Tfm {
            width,
            height,
            depth,
            c,
            f,
        } => {
            mem.set_b32s1(b + 1, width);
            mem.set_b32s1(b + 3, height);
            mem.set_b32s1(b + 2, depth);
            let p = get_avail(mem);
            if p == ALLOC_FAILED {
                return Err(NodeRecycleAbort::OverflowRunaway);
            }
            mem.set_b16s0(p, c);
            mem.set_b16s1(p, f as u16);
            p
        }
    };
    mem.set_b32s1(b + 5, p);
    Ok(b)
}

/// Transitional bridge-down wrapper for legacy `char_box` callers (`01b` §3,
/// §5). Performs the `with_state`-bearing native-character allocation / TFM
/// font-table reads, then builds the box under a single `mem` borrow.
pub unsafe fn char_box_global(f: i32, c: i32) -> i32 {
    let input = if font_area_entry(f) == AAT_FONT_FLAG || font_area_entry(f) == OTGR_FONT_FLAG {
        CharBoxInput::Native {
            p: crate::fonts::new_native_character(f, c as u32),
        }
    } else {
        let q = font_info_b16(
            char_base_entry(f) + crate::scan::effective_char_global(true, f, c as u16),
        );
        CharBoxInput::Tfm {
            width: font_info_b32s1(width_base_entry(f) + q.s3 as i32)
                + font_info_b32s1(italic_base_entry(f) + q.s1 as i32 / 4),
            height: font_info_b32s1(height_base_entry(f) + q.s2 as i32 / 16),
            depth: font_info_b32s1(depth_base_entry(f) + q.s2 as i32 % 16),
            c: c as u16,
            f,
        }
    };
    match crate::state::with_state_mut(|st| char_box(&mut st.mem, input)) {
        Ok(b) => b,
        Err(e) => node_recycle_abort(e),
    }
}

/// translated from xetex-math.c:1463 (stack_into_box)
///
/// Phase-C: threaded to `&mut MemState`; takes the character box `p` already
/// built by [`char_box_global`] (in [`stack_into_box_global`]) and links it.
fn stack_into_box(mem: &mut MemState, b: i32, p: i32) {
    let link = mem.b32s1(b + 5);
    mem.set_b32s1(p, link);
    mem.set_b32s1(b + 5, p);
    let ph = mem.b32s1(p + 3);
    mem.set_b32s1(b + 3, ph);
}

/// Transitional bridge-down wrapper for legacy `stack_into_box` callers
/// (`01b` §3). Builds the character box, then links it under one `mem` borrow.
pub unsafe fn stack_into_box_global(b: i32, f: i32, c: u16) {
    let p = char_box_global(f, c as i32);
    crate::state::with_state_mut(|st| stack_into_box(&mut st.mem, b, p));
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

fn new_stack_glyph(mem: &mut MemState, f: i32, g: i32) -> Result<i32, NodeRecycleAbort> {
    let p = crate::mem::get_node(mem, GLYPH_NODE_SIZE);
    if p == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b16s1(p, WHATSIT_NODE);
    mem.set_b16s0(p, GLYPH_NODE);
    mem.set_b16s2(p + 4, f as u16);
    mem.set_b16s1(p + 4, g as u16);
    Ok(p)
}

/// translated from xetex-math.c:1482 (stack_glyph_into_box)
///
/// Phase-C: the measured glyph node `p` is spliced under a narrow `MemState`
/// borrow. Legacy callers use [`stack_glyph_into_box_global`], which allocates
/// the node, drops the borrow for `measure_native_glyph`, then links it here.
pub fn stack_glyph_into_box(mem: &mut MemState, b: i32, p: i32) {
    if mem.b16s1(b) == HLIST_NODE {
        let mut q = mem.b32s1(b + 5);
        if q == TEX_NULL {
            mem.set_b32s1(b + 5, p);
        } else {
            while mem.b32s1(q) != TEX_NULL {
                q = mem.b32s1(q);
            }
            mem.set_b32s1(q, p);
            let glyph_height = mem.b32s1(p + 3);
            if mem.b32s1(b + 3) < glyph_height {
                mem.set_b32s1(b + 3, glyph_height);
            }
            let glyph_depth = mem.b32s1(p + 2);
            if mem.b32s1(b + 2) < glyph_depth {
                mem.set_b32s1(b + 2, glyph_depth);
            }
        }
    } else {
        mem.set_b32s1(p, mem.b32s1(b + 5));
        mem.set_b32s1(b + 5, p);
        mem.set_b32s1(b + 3, mem.b32s1(p + 3));
        let glyph_width = mem.b32s1(p + 1);
        if mem.b32s1(b + 1) < glyph_width {
            mem.set_b32s1(b + 1, glyph_width);
        }
    }
}

/// Transitional bridge-down wrapper for legacy `stack_glyph_into_box` callers.
/// The native-glyph measurement path reads font state via bridge accessors, so
/// it must run between the allocation and linking borrows.
pub unsafe fn stack_glyph_into_box_global(b: i32, f: i32, g: i32) {
    let p = match crate::state::with_state_mut(|st| new_stack_glyph(&mut st.mem, f, g)) {
        Ok(p) => p,
        Err(e) => node_recycle_abort(e),
    };
    crate::ext::measure_native_glyph(mem_node_addr(p), 1);
    crate::state::with_state_mut(|st| stack_glyph_into_box(&mut st.mem, b, p));
}

fn native_glyph_vbox_start(
    mem: &mut MemState,
    f: i32,
    g: u16,
) -> Result<(i32, i32), NodeRecycleAbort> {
    let b = new_null_box(mem);
    if b == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b16s1(b, VLIST_NODE);
    let p = new_stack_glyph(mem, f, g as i32)?;
    Ok((b, p))
}

fn native_glyph_vbox_finish(mem: &mut MemState, b: i32, p: i32) {
    mem.set_b32s1(b + 5, p);
    mem.set_b32s1(b + 1, mem.b32s1(p + 1));
    mem.set_b32s1(b + 3, mem.b32s1(p + 3));
    mem.set_b32s1(b + 2, mem.b32s1(p + 2));
}

unsafe fn native_glyph_vbox_global(f: i32, g: u16) -> i32 {
    let (b, p) = match crate::state::with_state_mut(|st| native_glyph_vbox_start(&mut st.mem, f, g))
    {
        Ok(nodes) => nodes,
        Err(e) => node_recycle_abort(e),
    };
    crate::ext::measure_native_glyph(mem_node_addr(p), 1);
    crate::state::with_state_mut(|st| native_glyph_vbox_finish(&mut st.mem, b, p));
    b
}

/// translated from xetex-math.c:1517 (stack_glue_into_box)
///
/// Phase-C: threaded to `&mut MemState`; bottoms out on the threaded
/// [`new_spec`]/[`new_glue`] and propagates their [`ALLOC_FAILED`] sentinel as
/// [`NodeRecycleAbort::Overflow`] (`01b` §5). Legacy callers use
/// [`stack_glue_into_box_global`].
fn stack_glue_into_box(
    mem: &mut MemState,
    b: i32,
    min: i32,
    max: i32,
) -> Result<(), NodeRecycleAbort> {
    let q_spec = new_spec(mem, 0);
    if q_spec == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(q_spec + 1, min);
    mem.set_b32s1(q_spec + 2, max - min);
    let p = new_glue(mem, q_spec);
    if p == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    if mem.b16s1(b) == HLIST_NODE {
        let mut q = mem.b32s1(b + 5);
        if q == TEX_NULL {
            mem.set_b32s1(b + 5, p);
        } else {
            while mem.b32s1(q) != TEX_NULL {
                q = mem.b32s1(q);
            }
            mem.set_b32s1(q, p);
        }
    } else {
        let link = mem.b32s1(b + 5);
        mem.set_b32s1(p, link);
        mem.set_b32s1(b + 5, p);
        let pd = mem.b32s1(p + 3);
        mem.set_b32s1(b + 3, pd);
        let pw = mem.b32s1(p + 1);
        mem.set_b32s1(b + 1, pw);
    }
    Ok(())
}

/// Transitional bridge-down wrapper for legacy `stack_glue_into_box` callers
/// (`01b` §3, §5).
pub unsafe fn stack_glue_into_box_global(b: i32, min: i32, max: i32) {
    if let Err(e) = crate::state::with_state_mut(|st| stack_glue_into_box(&mut st.mem, b, min, max))
    {
        node_recycle_abort(e);
    }
}

enum OtMathAccentPosStep {
    FetchMathChar,
    Recurse(i32),
    Infinity,
}

fn compute_ot_math_accent_pos_step(mem: &MemState, p: i32) -> OtMathAccentPosStep {
    if mem.b32s1(p + 1) == MATH_CHAR {
        OtMathAccentPosStep::FetchMathChar
    } else if mem.b32s1(p + 1) == SUB_MLIST {
        let r = mem.b32s0(p + 1);
        if r != TEX_NULL && mem.b16s1(r) == ACCENT_NOAD {
            OtMathAccentPosStep::Recurse(r)
        } else {
            OtMathAccentPosStep::Infinity
        }
    } else {
        OtMathAccentPosStep::Infinity
    }
}

/// translated from xetex-math.c:199 (compute_ot_math_accent_pos)
unsafe fn compute_ot_math_accent_pos_global(mut p: i32) -> i32 {
    loop {
        match crate::state::with_state(|st| compute_ot_math_accent_pos_step(&st.mem, p)) {
            OtMathAccentPosStep::FetchMathChar => {
                fetch_global(p + 1);
                let (f, c) = crate::state::with_state(|st| (st.scanner.cur_f, st.scanner.cur_c));
                let q = crate::fonts::new_native_character(f, c as u32);
                let g = real_get_native_glyph(mem_node_addr(q), 0);
                return get_ot_math_accent_pos(f, g as i32);
            }
            OtMathAccentPosStep::Recurse(r) => {
                p = r;
            }
            OtMathAccentPosStep::Infinity => return TEX_INFINITY,
        }
    }
}

/// translated from xetex-math.c:388 (scan_delimiter)
#[no_mangle]
pub unsafe extern "C" fn scan_delimiter(p: i32, r: bool) {
    if r {
        if cur_chr() == 1 {
            set_cur_val1(0x4000_0000);
            crate::scan::scan_math_fam_int();
            set_cur_val1(cur_val1() + cur_val() * 0x20_0000);
            crate::scan::scan_usv_num();
            set_cur_val(cur_val() + cur_val1());
        } else {
            crate::scan::scan_delimiter_int();
        }
    } else {
        loop {
            crate::scan::get_x_token();
            if cur_cmd() != SPACER && cur_cmd() != RELAX {
                break;
            }
        }

        match cur_cmd() {
            LETTER | OTHER_CHAR => set_cur_val(eqtb_b32s1(DEL_CODE_BASE + cur_chr())),
            DELIM_NUM => {
                if cur_chr() == 1 {
                    set_cur_val1(0x4000_0000);
                    crate::scan::scan_math_class_int();
                    crate::scan::scan_math_fam_int();
                    set_cur_val1(cur_val1() + cur_val() * 0x2_0000);
                    crate::scan::scan_usv_num();
                    set_cur_val(cur_val() + cur_val1());
                } else {
                    crate::scan::scan_delimiter_int();
                }
            }
            _ => set_cur_val(-1),
        }
    }

    if cur_val() < 0 {
        if file_line_error_style_p() != 0 {
            crate::output::print_file_line();
        } else {
            crate::output::print_nl_cstr(c"! ".as_ptr());
        }
        crate::output::print_cstr(c"Missing delimiter (. inserted)".as_ptr());
        set_help_ptr(6);
        set_help_line(
            5,
            c"I was expecting to see something like `(' or `\\{' or".as_ptr(),
        );
        set_help_line(
            4,
            c"`\\}' here. If you typed, e.g., `{' instead of `\\{', you".as_ptr(),
        );
        set_help_line(
            3,
            c"should probably delete the `{' by typing `1' now, so that".as_ptr(),
        );
        set_help_line(
            2,
            c"braces don't get unbalanced. Otherwise just proceed.".as_ptr(),
        );
        set_help_line(
            1,
            c"Acceptable delimiters are characters whose \\delcode is".as_ptr(),
        );
        set_help_line(
            0,
            c"nonnegative, or you can use `\\delimiter <delimiter code>'.".as_ptr(),
        );
        crate::input::back_error();
        set_cur_val(0);
    }

    if cur_val() >= 0x4000_0000 {
        set_mem_b16s3(
            p,
            (((cur_val() % 0x20_0000) / 0x1_0000) * 0x100 + (cur_val() / 0x20_0000) % 0x100) as u16,
        );
        set_mem_b16s2(p, (cur_val() % 0x1_0000) as u16);
        set_mem_b16s1(p, 0);
        set_mem_b16s0(p, 0);
    } else {
        set_mem_b16s3(p, ((cur_val() / 0x10_0000) % 16) as u16);
        set_mem_b16s2(p, ((cur_val() / 0x1000) % 0x100) as u16);
        set_mem_b16s1(p, ((cur_val() / 0x100) % 16) as u16);
        set_mem_b16s0(p, (cur_val() % 0x100) as u16);
    }
}

/// translated from xetex-math.c:354 (start_eq_no)
pub unsafe fn start_eq_no() {
    set_save_stack_b32s1(save_ptr(), cur_chr());
    set_save_ptr(save_ptr() + 1);

    crate::main_control::push_math(MATH_SHIFT_GROUP);
    crate::save::eq_word_define(INT_BASE + INT_PAR_CUR_FAM, -1);
    if insert_src_special_every_math() {
        crate::ext::insert_src_special();
    }
    let every_math = eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_MATH);
    if every_math != TEX_NULL {
        begin_token_list(every_math, EVERY_MATH_TEXT);
    }
}

/// translated from xetex-math.c:368 (math_limit_switch)
fn apply_math_limit_switch(mem: &mut MemState, limit: u16) -> bool {
    if cur_list_head() != cur_list_tail() && mem.b16s1(cur_list_tail()) == OP_NOAD {
        mem.set_b16s0(cur_list_tail(), limit);
        true
    } else {
        false
    }
}

pub unsafe fn math_limit_switch_global() {
    let applied =
        crate::state::with_state_mut(|st| apply_math_limit_switch(&mut st.mem, cur_chr() as u16));
    if applied {
        return;
    }
    crate::output::error_here_with_diagnostic(
        c"Limit controls must follow a math operator".as_ptr(),
    );
    crate::output::capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(1);
    set_help_line(
        0,
        c"I'm ignoring this misplaced \\limits or \\nolimits command.".as_ptr(),
    );
    crate::errors::error();
}

/// translated from xetex-math.c:461 (math_radical)
fn append_radical_noad(mem: &mut MemState) -> Result<i32, NodeRecycleAbort> {
    let node = get_node(mem, RADICAL_NOAD_SIZE);
    if node == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(cur_list_tail(), node);
    set_cur_list_tail(mem.b32s1(cur_list_tail()));
    let tail = cur_list_tail();
    mem.set_b16s1(tail, RADICAL_NOAD);
    mem.set_b16s0(tail, NORMAL);
    set_empty_math_field(mem, tail + 1);
    set_empty_math_field(mem, tail + 3);
    set_empty_math_field(mem, tail + 2);
    Ok(tail)
}

pub unsafe fn math_radical_global() {
    let tail = match crate::state::with_state_mut(|st| append_radical_noad(&mut st.mem)) {
        Ok(tail) => tail,
        Err(abort) => node_recycle_abort(abort),
    };
    scan_delimiter(tail + 4, true);
    crate::scan::scan_math(tail + 1);
}

/// translated from xetex-math.c:533 (append_choices)
fn append_choices_core(
    mem: &mut MemState,
    save_stack: &mut SaveStackState,
) -> Result<(), NodeRecycleAbort> {
    let choice = new_choice(mem);
    if choice == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(cur_list_tail(), choice);
    set_cur_list_tail(mem.b32s1(cur_list_tail()));
    save_stack.save_ptr += 1;
    save_stack.set_b32s1(save_stack.save_ptr - 1, 0);
    Ok(())
}

pub unsafe fn append_choices_global() {
    if let Err(abort) =
        crate::state::with_state_mut(|st| append_choices_core(&mut st.mem, &mut st.save_stack))
    {
        node_recycle_abort(abort);
    }
    crate::main_control::push_math(MATH_CHOICE_GROUP);
    crate::scan::scan_left_brace();
}

/// translated from xetex-math.c:545 (fin_mlist)
///
/// Phase C (`01b` §4a bucket C): the mem work + raw `cur_list` reads run under a
/// single `&mut MemState` borrow; the impossible corrupt-node `confusion`
/// (`q` not a `LEFT_NOAD`) propagates out as `Err` and the final `pop_nest` (which
/// is `with_state`-bearing through the `nest` array) is deferred to
/// [`fin_mlist_global`], both fired after the borrow drops. `pop_nest` only
/// recycles `cur_list.head`, which the core never modifies, so the original
/// ordering (`pop_nest` last) is preserved.
fn fin_mlist(mem: &mut MemState, p: i32) -> Result<i32, &'static std::ffi::CStr> {
    let q;
    let aux = cur_list_aux_b32s1();
    if aux != TEX_NULL {
        mem.set_b32s1(aux + 3, SUB_MLIST);
        let head_link = mem.b32s1(cur_list_head());
        mem.set_b32s0(aux + 3, head_link);
        if p == TEX_NULL {
            q = aux;
        } else {
            q = mem.b32s0(aux + 2);
            let etex_aux = cur_list_e_tex_aux();
            if mem.b16s1(q) != LEFT_NOAD || etex_aux == TEX_NULL {
                return Err(c"right");
            }
            let etex_link = mem.b32s1(etex_aux);
            mem.set_b32s0(aux + 2, etex_link);
            mem.set_b32s1(etex_aux, aux);
            mem.set_b32s1(aux, p);
        }
    } else {
        mem.set_b32s1(cur_list_tail(), p);
        q = mem.b32s1(cur_list_head());
    }
    Ok(q)
}

/// Bridge-down wrapper for legacy callers (`01b` §3): runs the threaded
/// [`fin_mlist`] under one borrow, fires the cold `confusion` if it hit the
/// corrupt-node arm, then runs the `with_state`-bearing `pop_nest` after the
/// borrow drops.
pub unsafe fn fin_mlist_global(p: i32) -> i32 {
    let q = match crate::state::with_state_mut(|st| fin_mlist(&mut st.mem, p)) {
        Ok(q) => q,
        Err(msg) => crate::errors::confusion(msg.as_ptr()),
    };
    crate::mem::pop_nest();
    q
}

/// translated from xetex-math.c:571 (build_choices)
fn build_choices_finish(mem: &mut MemState, save_stack: &mut SaveStackState, p: i32) -> bool {
    match save_stack.b32s1(save_stack.save_ptr - 1) {
        0 => mem.set_b32s0(cur_list_tail() + 1, p),
        1 => mem.set_b32s1(cur_list_tail() + 1, p),
        2 => mem.set_b32s0(cur_list_tail() + 2, p),
        3 => {
            mem.set_b32s1(cur_list_tail() + 2, p);
            save_stack.save_ptr -= 1;
            return true;
        }
        _ => {}
    }
    let choice_state = save_stack.b32s1(save_stack.save_ptr - 1);
    save_stack.set_b32s1(save_stack.save_ptr - 1, choice_state + 1);
    false
}

pub unsafe fn build_choices_global() {
    crate::save::unsave();
    let p = fin_mlist_global(TEX_NULL);
    let complete =
        crate::state::with_state_mut(|st| build_choices_finish(&mut st.mem, &mut st.save_stack, p));
    if complete {
        return;
    }
    crate::main_control::push_math(MATH_CHOICE_GROUP);
    crate::scan::scan_left_brace();
}

/// translated from xetex-math.c:599 (sub_sup)
enum ScriptDuplication {
    Superscript,
    Subscript,
}

fn prepare_sub_sup(
    mem: &mut MemState,
    cmd: u8,
) -> Result<(i32, Option<ScriptDuplication>), NodeRecycleAbort> {
    let mut t = EMPTY;
    let mut p = TEX_NULL;

    if cur_list_tail() != cur_list_head() {
        let node_type = mem.b16s1(cur_list_tail());
        if (16..LEFT_NOAD).contains(&node_type) {
            p = cur_list_tail() + 2 + cmd as i32 - 7;
            t = mem.b32s1(p);
        }
    }

    let mut duplicate = None;
    if p == TEX_NULL || t != EMPTY {
        let noad = new_noad(mem);
        if noad == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        mem.set_b32s1(cur_list_tail(), noad);
        set_cur_list_tail(mem.b32s1(cur_list_tail()));
        p = cur_list_tail() + 2 + cmd as i32 - 7;
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

pub unsafe fn sub_sup_global() {
    let cmd = cur_cmd();
    let (p, duplicate) = match crate::state::with_state_mut(|st| prepare_sub_sup(&mut st.mem, cmd))
    {
        Ok(result) => result,
        Err(abort) => node_recycle_abort(abort),
    };
    if let Some(duplicate) = duplicate {
        match duplicate {
            ScriptDuplication::Superscript => {
                crate::output::error_here_with_diagnostic(c"Double superscript".as_ptr());
                set_help_line(0, c"I treat `x^1^2' essentially like `x^1{}^2'.".as_ptr());
            }
            ScriptDuplication::Subscript => {
                crate::output::error_here_with_diagnostic(c"Double subscript".as_ptr());
                set_help_line(0, c"I treat `x_1_2' essentially like `x_1{}_2'.".as_ptr());
            }
        }
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(1);
        crate::errors::error();
    }
    crate::scan::scan_math(p);
}

/// translated from xetex-math.c:644 (math_fraction)
fn start_fraction_noad(mem: &mut MemState) -> Result<i32, NodeRecycleAbort> {
    let fraction = get_node(mem, FRACTION_NOAD_SIZE);
    if fraction == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    set_cur_list_aux_b32s1(fraction);
    let aux = cur_list_aux_b32s1();
    mem.set_b16s1(aux, FRACTION_NOAD);
    mem.set_b16s0(aux, NORMAL);
    mem.set_b32s1(aux + 2, SUB_MLIST);
    mem.set_b32s0(aux + 2, mem.b32s1(cur_list_head()));
    set_empty_math_field(mem, aux + 3);
    mem.set_b16s3(aux + 4, 0);
    mem.set_b16s2(aux + 4, 0);
    mem.set_b16s1(aux + 4, 0);
    mem.set_b16s0(aux + 4, 0);
    mem.set_b16s3(aux + 5, 0);
    mem.set_b16s2(aux + 5, 0);
    mem.set_b16s1(aux + 5, 0);
    mem.set_b16s0(aux + 5, 0);
    mem.set_b32s1(cur_list_head(), TEX_NULL);
    set_cur_list_tail(cur_list_head());
    Ok(aux)
}

fn finish_fraction_rule(mem: &mut MemState, c: i32, rule_thickness: i32) {
    match c % DELIMITED_CODE {
        ABOVE_CODE => mem.set_b32s1(cur_list_aux_b32s1() + 1, rule_thickness),
        OVER_CODE => mem.set_b32s1(cur_list_aux_b32s1() + 1, DEFAULT_CODE),
        ATOP_CODE => mem.set_b32s1(cur_list_aux_b32s1() + 1, 0),
        _ => {}
    }
}

pub unsafe fn math_fraction_global() {
    let c = cur_chr();

    if cur_list_aux_b32s1() != TEX_NULL {
        if c >= DELIMITED_CODE {
            scan_delimiter(GARBAGE, false);
            scan_delimiter(GARBAGE, false);
        }

        if c % DELIMITED_CODE == ABOVE_CODE {
            crate::scan::scan_dimen(false, false, false);
        }

        if file_line_error_style_p() != 0 {
            crate::output::print_file_line();
        } else {
            crate::output::print_nl_cstr(c"! ".as_ptr());
        }
        crate::output::print_cstr(c"Ambiguous; you need another { and }".as_ptr());
        set_help_ptr(3);
        set_help_line(
            2,
            c"I'm ignoring this fraction specification, since I don't".as_ptr(),
        );
        set_help_line(
            1,
            c"know whether a construction like `x \\over y \\over z'".as_ptr(),
        );
        set_help_line(
            0,
            c"means `{x \\over y} \\over z' or `x \\over {y \\over z}'.".as_ptr(),
        );
        crate::errors::error();
    } else {
        let aux = match crate::state::with_state_mut(|st| start_fraction_noad(&mut st.mem)) {
            Ok(aux) => aux,
            Err(abort) => node_recycle_abort(abort),
        };

        if c >= DELIMITED_CODE {
            scan_delimiter(aux + 4, false);
            scan_delimiter(aux + 5, false);
        }

        let rule_thickness = if c % DELIMITED_CODE == ABOVE_CODE {
            crate::scan::scan_dimen(false, false, false);
            cur_val()
        } else {
            0
        };
        crate::state::with_state_mut(|st| finish_fraction_rule(&mut st.mem, c, rule_thickness));
    }
}

fn append_accent_noad(mem: &mut MemState) -> Result<i32, NodeRecycleAbort> {
    let accent = get_node(mem, ACCENT_NOAD_SIZE);
    if accent == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(cur_list_tail(), accent);
    set_cur_list_tail(mem.b32s1(cur_list_tail()));
    let tail = cur_list_tail();
    mem.set_b16s1(tail, ACCENT_NOAD);
    mem.set_b16s0(tail, NORMAL);
    set_empty_math_field(mem, tail + 1);
    set_empty_math_field(mem, tail + 3);
    set_empty_math_field(mem, tail + 2);
    mem.set_b32s1(tail + 4, MATH_CHAR);
    Ok(tail)
}

fn finish_accent_math_field(mem: &mut MemState, eqtb: &EqtbState, tail: i32, value: i32) {
    mem.set_b16s0(tail + 4, (value % 65536) as u16);
    let intpar_cur_fam = eqtb.b32s1(INT_BASE + INT_PAR_CUR_FAM);
    if math_class_field(value) == 7 && (0..NUMBER_MATH_FAMILIES).contains(&intpar_cur_fam) {
        mem.set_b16s1(tail + 4, intpar_cur_fam as u16);
    } else {
        mem.set_b16s1(tail + 4, math_fam_field(value) as u16);
    }
    mem.set_b16s1(
        tail + 4,
        mem.b16s1(tail + 4) + (math_char_field(value) / 65536) as u16 * 256,
    );
}

fn set_accent_kind(mem: &mut MemState, tail: i32, kind: u16) {
    mem.set_b16s0(tail, kind);
}

/// translated from xetex-math.c:703 (math_left_right)
fn append_left_right_noad(mem: &mut MemState, node_type: u16) -> Result<i32, NodeRecycleAbort> {
    let p = new_noad(mem);
    if p == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b16s1(p, node_type);
    Ok(p)
}

fn mark_middle_noad(mem: &mut MemState, p: i32) {
    mem.set_b16s1(p, RIGHT_NOAD);
    mem.set_b16s0(p, 1);
}

fn finish_left_right_group(mem: &mut MemState, q: i32, p: i32) {
    mem.set_b32s1(cur_list_head(), q);
    set_cur_list_tail(p);
    set_cur_list_e_tex_aux(p);
}

fn append_inner_noad_for_right(mem: &mut MemState, q: i32) -> Result<(), NodeRecycleAbort> {
    let inner = new_noad(mem);
    if inner == ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(cur_list_tail(), inner);
    set_cur_list_tail(mem.b32s1(cur_list_tail()));
    let tail = cur_list_tail();
    mem.set_b16s1(tail, INNER_NOAD);
    mem.set_b32s1(tail + 1, SUB_MLIST);
    mem.set_b32s0(tail + 1, q);
    Ok(())
}

pub unsafe fn math_left_right_global() {
    let t = cur_chr() as u16;
    if t != LEFT_NOAD && cur_group() != MATH_LEFT_GROUP {
        if cur_group() == MATH_SHIFT_GROUP {
            scan_delimiter(GARBAGE, false);
            crate::output::error_here_with_diagnostic(c"Extra ".as_ptr());
            if t == 1 {
                crate::output::print_esc_cstr(c"middle".as_ptr());
                set_help_ptr(1);
                set_help_line(
                    0,
                    c"I'm ignoring a \\middle that had no matching \\left.".as_ptr(),
                );
            } else {
                crate::output::print_esc_cstr(c"right".as_ptr());
                set_help_ptr(1);
                set_help_line(
                    0,
                    c"I'm ignoring a \\right that had no matching \\left.".as_ptr(),
                );
            }
            crate::output::capture_to_diagnostic(std::ptr::null_mut());
            crate::errors::error();
        } else {
            crate::main_control::off_save();
        }
    } else {
        let p = match crate::state::with_state_mut(|st| append_left_right_noad(&mut st.mem, t)) {
            Ok(p) => p,
            Err(abort) => node_recycle_abort(abort),
        };
        scan_delimiter(p + 1, false);
        if t == 1 {
            crate::state::with_state_mut(|st| mark_middle_noad(&mut st.mem, p));
        }
        let q = if t == LEFT_NOAD {
            p
        } else {
            let q = fin_mlist_global(p);
            crate::save::unsave();
            q
        };
        if t != RIGHT_NOAD {
            crate::main_control::push_math(MATH_LEFT_GROUP);
            crate::state::with_state_mut(|st| finish_left_right_group(&mut st.mem, q, p));
        } else if let Err(abort) =
            crate::state::with_state_mut(|st| append_inner_noad_for_right(&mut st.mem, q))
        {
            node_recycle_abort(abort);
        }
    }
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

/// translated from xetex-math.c:355 (math_ac)
pub unsafe fn math_ac_global() {
    if cur_cmd() == ACCENT {
        crate::output::error_here_with_diagnostic(c"Please use ".as_ptr());
        crate::output::print_esc_cstr(c"mathaccent".as_ptr());
        crate::output::print_cstr(c" for accents in math mode".as_ptr());
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"I'm changing \\accent to \\mathaccent here; wish me luck.".as_ptr(),
        );
        set_help_line(
            0,
            c"(Accents are not the same in formulas as they are in text.)".as_ptr(),
        );
        crate::errors::error();
    }

    let tail = match crate::state::with_state_mut(|st| append_accent_noad(&mut st.mem)) {
        Ok(tail) => tail,
        Err(abort) => node_recycle_abort(abort),
    };

    if cur_chr() == 1 {
        if crate::scan::scan_keyword(c"fixed".as_ptr()) {
            crate::state::with_state_mut(|st| set_accent_kind(&mut st.mem, tail, FIXED_ACC));
        } else if crate::scan::scan_keyword(c"bottom".as_ptr()) {
            if crate::scan::scan_keyword(c"fixed".as_ptr()) {
                crate::state::with_state_mut(|st| {
                    set_accent_kind(&mut st.mem, tail, BOTTOM_ACC + 1)
                });
            } else {
                crate::state::with_state_mut(|st| set_accent_kind(&mut st.mem, tail, BOTTOM_ACC));
            }
        }
        crate::scan::scan_math_class_int();
        let mut c = set_class_bits(cur_val());
        crate::scan::scan_math_fam_int();
        c += set_family_bits(cur_val());
        crate::scan::scan_usv_num();
        set_cur_val(cur_val() + c);
    } else {
        crate::scan::scan_fifteen_bit_int();
        set_cur_val(
            set_class_bits(cur_val() / 4096)
                + set_family_bits((cur_val() % 4096) / 256)
                + (cur_val() % 256),
        );
    }

    let value = cur_val();
    crate::state::with_state_mut(|st| {
        finish_accent_math_field(&mut st.mem, &st.eqtb, tail, value);
    });
    crate::scan::scan_math(tail + 1);
}

/// translated from xetex-math.c:786 (resume_after_display)
pub unsafe fn resume_after_display() {
    if cur_group() != MATH_SHIFT_GROUP {
        crate::errors::confusion(c"display".as_ptr());
    }

    crate::save::unsave();
    set_cur_list_prev_graf(cur_list_prev_graf() + 3);
    crate::mem::push_nest();
    set_cur_list_mode(HMODE);
    set_cur_list_aux_b32s0(1000);

    let lang = eqtb_b32s1(INT_BASE + INT_PAR_LANGUAGE);
    if lang <= 0 || lang > BIGGEST_LANG {
        set_cur_lang(0);
    } else {
        set_cur_lang(lang as u8);
    }
    set_cur_list_aux_b32s1(cur_lang() as i32);

    let lhm = crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as i32;
    let rhm = crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as i32;
    set_cur_list_prev_graf((lhm * 64 + rhm) * 65536 + cur_lang() as i32);

    crate::scan::get_x_token();
    if cur_cmd() != SPACER {
        crate::input::back_input();
    }
    if nest_ptr() == 1 {
        crate::pagebuilder::build_page();
    }
}

#[inline]
fn sym_font_ok(fonts: &FontState, eqtb: &EqtbState, size: i32) -> bool {
    let f = eqtb.b32s1(MATH_FONT_BASE + 2 + size);
    fonts.font_params(f) >= TOTAL_MATHSY_PARAMS || font_is_ot_math(fonts, f)
}

#[inline]
fn sym_font_ok_global(size: i32) -> bool {
    crate::state::with_state(|st| sym_font_ok(&st.fonts, &st.eqtb, size))
}

#[inline]
fn ext_font_ok(fonts: &FontState, eqtb: &EqtbState, size: i32) -> bool {
    let f = eqtb.b32s1(MATH_FONT_BASE + 3 + size);
    fonts.font_params(f) >= TOTAL_MATHEX_PARAMS || font_is_ot_math(fonts, f)
}

#[inline]
fn ext_font_ok_global(size: i32) -> bool {
    crate::state::with_state(|st| ext_font_ok(&st.fonts, &st.eqtb, size))
}

/// translated from xetex-math.c:82 (app_display) — private helper
unsafe fn app_display(j: i32, b_in: i32, d_in: i32) {
    let mut b = b_in;
    let mut d = d_in;
    let s = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT);
    let x = eqtb_b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_DIRECTION);

    if x == 0 {
        set_mem_b32s1(b + 4, s + d);
    } else {
        let z = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_WIDTH);
        let p_orig = b;
        let mut e;
        if x > 0 {
            e = z - d - mem_b32s1(p_orig + 1);
        } else {
            e = d;
            d = z - e - mem_b32s1(p_orig + 1);
        }
        if j != TEX_NULL {
            b = crate::packaging::copy_node_list_global(j);
            set_mem_b32s1(b + 3, mem_b32s1(p_orig + 3));
            set_mem_b32s1(b + 2, mem_b32s1(p_orig + 2));
            let b4 = mem_b32s1(b + 4);
            let s2 = s - b4;
            d += s2;
            e = e + mem_b32s1(b + 1) - z - s2;
        }
        let (mut p, mut q);
        if mem_b16s0(p_orig) == DLIST {
            q = p_orig;
            p = p_orig;
        } else {
            let r0 = mem_b32s1(p_orig + 5);
            crate::mem::free_node_global(p_orig, BOX_NODE_SIZE);
            if r0 == TEX_NULL {
                crate::errors::confusion(c"LR4".as_ptr());
            }
            let mut r = r0;
            if x > 0 {
                p = r;
                loop {
                    q = r;
                    r = mem_b32s1(r);
                    if r == TEX_NULL {
                        break;
                    }
                }
            } else {
                p = TEX_NULL;
                q = r;
                loop {
                    let t = mem_b32s1(r);
                    set_mem_b32s1(r, p);
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
            r = crate::mem::new_kern_global(0);
            t = crate::mem::new_kern_global(0);
        } else {
            r = mem_b32s1(b + 5);
            t = mem_b32s1(r);
        }
        let u = crate::mem::new_math_global(0, END_M_CODE);
        if mem_b16s1(t) == GLUE_NODE {
            let jj = crate::mem::new_skip_param_global(GLUE_PAR_RIGHT_SKIP as i16);
            set_mem_b32s1(q, jj);
            set_mem_b32s1(jj, u);
            let jspec = mem_b32s0(t + 1);
            set_mem_b16s1(temp_ptr(), mem_b16s1(jspec));
            set_mem_b16s0(temp_ptr(), mem_b16s0(jspec));
            set_mem_b32s1(temp_ptr() + 1, e - mem_b32s1(jspec + 1));
            set_mem_b32s1(temp_ptr() + 2, -mem_b32s1(jspec + 2));
            set_mem_b32s1(temp_ptr() + 3, -mem_b32s1(jspec + 3));
            set_mem_b32s1(u, t);
        } else {
            set_mem_b32s1(t + 1, e);
            set_mem_b32s1(t, u);
            set_mem_b32s1(q, t);
        }
        let u2 = crate::mem::new_math_global(0, BEGIN_M_CODE);
        if mem_b16s1(r) == GLUE_NODE {
            let jj = crate::mem::new_skip_param_global(GLUE_PAR_LEFT_SKIP as i16);
            set_mem_b32s1(u2, jj);
            set_mem_b32s1(jj, p);
            let jspec = mem_b32s0(r + 1);
            set_mem_b16s1(temp_ptr(), mem_b16s1(jspec));
            set_mem_b16s0(temp_ptr(), mem_b16s0(jspec));
            set_mem_b32s1(temp_ptr() + 1, d - mem_b32s1(jspec + 1));
            set_mem_b32s1(temp_ptr() + 2, -mem_b32s1(jspec + 2));
            set_mem_b32s1(temp_ptr() + 3, -mem_b32s1(jspec + 3));
            set_mem_b32s1(r, u2);
        } else {
            set_mem_b32s1(r + 1, d);
            set_mem_b32s1(r, p);
            set_mem_b32s1(u2, r);
            if j == TEX_NULL {
                b = crate::packaging::hpack(u2, 0, ADDITIONAL);
                set_mem_b32s1(b + 4, s);
            } else {
                set_mem_b32s1(b + 5, u2);
            }
        }
    }
    crate::packaging::append_to_vlist(b);
}

/// Emit error for insufficient math fonts, flush the math list, set danger.
unsafe fn check_math_font_params(danger: &mut bool) {
    if !sym_font_ok_global(TEXT_SIZE)
        || !sym_font_ok_global(SCRIPT_SIZE)
        || !sym_font_ok_global(SCRIPT_SCRIPT_SIZE)
    {
        crate::output::error_here_with_diagnostic(
            c"Math formula deleted: Insufficient symbol fonts".as_ptr(),
        );
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(3);
        set_help_line(
            2,
            c"Sorry, but I can't typeset math unless \\textfont 2".as_ptr(),
        );
        set_help_line(
            1,
            c"and \\scriptfont 2 and \\scriptscriptfont 2 have all".as_ptr(),
        );
        set_help_line(
            0,
            c"the \\fontdimen values needed in math symbol fonts.".as_ptr(),
        );
        crate::errors::error();
        flush_math_global();
        *danger = true;
    } else if !ext_font_ok_global(TEXT_SIZE)
        || !ext_font_ok_global(SCRIPT_SIZE)
        || !ext_font_ok_global(SCRIPT_SCRIPT_SIZE)
    {
        crate::output::error_here_with_diagnostic(
            c"Math formula deleted: Insufficient extension fonts".as_ptr(),
        );
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(3);
        set_help_line(
            2,
            c"Sorry, but I can't typeset math unless \\textfont 3".as_ptr(),
        );
        set_help_line(
            1,
            c"and \\scriptfont 3 and \\scriptscriptfont 3 have all".as_ptr(),
        );
        set_help_line(
            0,
            c"the \\fontdimen values needed in math extension fonts.".as_ptr(),
        );
        crate::errors::error();
        flush_math_global();
        *danger = true;
    }
}

/// translated from xetex-math.c:194 (after_math)
pub unsafe fn after_math() {
    let mut danger = false;

    let mut j = if cur_list_mode() == MMODE {
        cur_list_e_tex_aux()
    } else {
        TEX_NULL
    };

    check_math_font_params(&mut danger);

    let mut m = cur_list_mode() as i32;
    let mut l = false;
    let mut p = fin_mlist_global(TEX_NULL);

    let a;
    if cur_list_mode() as i32 == -m {
        // eTeX display math: second `$$` ends an inner display
        crate::scan::get_x_token();
        if cur_cmd() != MATH_SHIFT {
            crate::output::error_here_with_diagnostic(c"Display math should end with $$".as_ptr());
            crate::output::capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(2);
            set_help_line(
                1,
                c"The `$' that I just saw supposedly matches a previous `$$'.".as_ptr(),
            );
            set_help_line(
                0,
                c"So I shall assume that you typed `$$' both times.".as_ptr(),
            );
            crate::input::back_error();
        }
        set_cur_mlist(p);
        set_cur_style(TEXT_STYLE);
        set_mlist_penalties(false);
        mlist_to_hlist();
        a = crate::packaging::hpack(mem_b32s1(TEMP_HEAD), 0, ADDITIONAL);
        set_mem_b16s0(a, DLIST);
        crate::save::unsave();
        set_save_ptr(save_ptr() - 1);
        if save_stack_b32s1(save_ptr()) == 1 {
            l = true;
        }
        danger = false;

        j = if cur_list_mode() == MMODE {
            cur_list_e_tex_aux()
        } else {
            TEX_NULL
        };

        check_math_font_params(&mut danger);

        m = cur_list_mode() as i32;
        p = fin_mlist_global(TEX_NULL);
    } else {
        a = TEX_NULL;
    }

    if m < 0 {
        // inline math (or inner/restricted hmode)
        let math_surround = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_MATH_SURROUND);
        let node = crate::mem::new_math_global(math_surround, BEFORE as i16);
        set_mem_b32s1(cur_list_tail(), node);
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        set_cur_mlist(p);
        set_cur_style(TEXT_STYLE);
        set_mlist_penalties(cur_list_mode() > 0);
        mlist_to_hlist();
        set_mem_b32s1(cur_list_tail(), mem_b32s1(TEMP_HEAD));
        while mem_b32s1(cur_list_tail()) != TEX_NULL {
            set_cur_list_tail(mem_b32s1(cur_list_tail()));
        }
        let node2 = crate::mem::new_math_global(math_surround, AFTER);
        set_mem_b32s1(cur_list_tail(), node2);
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        set_cur_list_aux_b32s0(1000);

        if semantic_pagination_enabled() {
            crate::ext::tt_insert_special(c"tdux:ce math".as_ptr());
        }

        crate::save::unsave();
    } else {
        // display math
        if a == TEX_NULL {
            crate::scan::get_x_token();
            if cur_cmd() != MATH_SHIFT {
                crate::output::error_here_with_diagnostic(
                    c"Display math should end with $$".as_ptr(),
                );
                crate::output::capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(
                    1,
                    c"The `$' that I just saw supposedly matches a previous `$$'.".as_ptr(),
                );
                set_help_line(
                    0,
                    c"So I shall assume that you typed `$$' both times.".as_ptr(),
                );
                crate::input::back_error();
            }
        }
        set_cur_mlist(p);
        set_cur_style(DISPLAY_STYLE);
        set_mlist_penalties(false);
        mlist_to_hlist();
        p = mem_b32s1(TEMP_HEAD);
        set_adjust_tail(ADJUST_HEAD);
        set_pre_adjust_tail(PRE_ADJUST_HEAD);
        let mut b = crate::packaging::hpack(p, 0, ADDITIONAL);
        p = mem_b32s1(b + 5);
        let t = adjust_tail();
        set_adjust_tail(TEX_NULL);
        let pre_t = pre_adjust_tail();
        set_pre_adjust_tail(TEX_NULL);
        let mut w = mem_b32s1(b + 1);
        let z = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_WIDTH);
        let mut s = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT);
        if eqtb_b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_DIRECTION) < 0 {
            s = -s - z;
        }
        let (mut e, q);
        if a == TEX_NULL || danger {
            e = 0;
            q = 0;
        } else {
            e = mem_b32s1(a + 1);
            q = e + math_quad_global(TEXT_SIZE);
        }
        if w + q > z {
            if e != 0
                && (w - total_shrink(0) + q <= z
                    || total_shrink(FIL) != 0
                    || total_shrink(FILL) != 0
                    || total_shrink(FILLL) != 0)
            {
                crate::mem::free_node_global(b, BOX_NODE_SIZE);
                b = crate::packaging::hpack(p, z - q, EXACTLY);
            } else {
                e = 0;
                if w > z {
                    crate::mem::free_node_global(b, BOX_NODE_SIZE);
                    b = crate::packaging::hpack(p, z, EXACTLY);
                }
            }
            w = mem_b32s1(b + 1);
        }
        set_mem_b16s0(b, DLIST);
        let mut d = crate::scaled_math::half(z - w);
        if e > 0 && d < 2 * e {
            d = crate::scaled_math::half(z - w - e);
            if p != TEX_NULL && p < crate::globals::hi_mem_min() && mem_b16s1(p) == GLUE_NODE {
                d = 0;
            }
        }

        let penalty_node =
            crate::mem::new_penalty_global(eqtb_b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_PENALTY));
        set_mem_b32s1(cur_list_tail(), penalty_node);
        set_cur_list_tail(mem_b32s1(cur_list_tail()));

        let pre_display_size = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PRE_DISPLAY_SIZE);
        let (g1, mut g2);
        if d + s <= pre_display_size || l {
            g1 = GLUE_PAR_ABOVE_DISPLAY_SKIP;
            g2 = GLUE_PAR_BELOW_DISPLAY_SKIP;
        } else {
            g1 = GLUE_PAR_ABOVE_DISPLAY_SHORT_SKIP;
            g2 = GLUE_PAR_BELOW_DISPLAY_SHORT_SKIP;
        }

        if l && e == 0 {
            app_display(j, a, 0);
            let inf_node = crate::mem::new_penalty_global(INF_PENALTY);
            set_mem_b32s1(cur_list_tail(), inf_node);
            set_cur_list_tail(mem_b32s1(cur_list_tail()));
        } else {
            let g1_node = crate::mem::new_param_glue_global(g1);
            set_mem_b32s1(cur_list_tail(), g1_node);
            set_cur_list_tail(mem_b32s1(cur_list_tail()));
        }

        if e != 0 {
            let r = crate::mem::new_kern_global(z - w - e - d);
            if l {
                set_mem_b32s1(a, r);
                set_mem_b32s1(r, b);
                b = a;
                d = 0;
            } else {
                set_mem_b32s1(b, r);
                set_mem_b32s1(r, a);
            }
            b = crate::packaging::hpack(b, 0, ADDITIONAL);
        }

        app_display(j, b, d);

        if a != TEX_NULL && e == 0 && !l {
            let inf_node2 = crate::mem::new_penalty_global(INF_PENALTY);
            set_mem_b32s1(cur_list_tail(), inf_node2);
            set_cur_list_tail(mem_b32s1(cur_list_tail()));
            app_display(j, a, z - mem_b32s1(a + 1));
            g2 = 0;
        }

        if t != ADJUST_HEAD {
            set_mem_b32s1(cur_list_tail(), mem_b32s1(ADJUST_HEAD));
            set_cur_list_tail(t);
        }
        if pre_t != PRE_ADJUST_HEAD {
            set_mem_b32s1(cur_list_tail(), mem_b32s1(PRE_ADJUST_HEAD));
            set_cur_list_tail(pre_t);
        }

        let post_pen =
            crate::mem::new_penalty_global(eqtb_b32s1(INT_BASE + INT_PAR_POST_DISPLAY_PENALTY));
        set_mem_b32s1(cur_list_tail(), post_pen);
        set_cur_list_tail(mem_b32s1(cur_list_tail()));

        if g2 > 0 {
            let g2_node = crate::mem::new_param_glue_global(g2);
            set_mem_b32s1(cur_list_tail(), g2_node);
            set_cur_list_tail(mem_b32s1(cur_list_tail()));
        }

        crate::packaging::flush_node_list_global(j);

        if semantic_pagination_enabled() {
            crate::ext::tt_insert_special(c"tdux:ce dmath".as_ptr());
        }

        resume_after_display();
    }
}

/// translated from xetex-math.c:81 (init_math)
pub unsafe fn init_math() {
    crate::scan::get_token();

    if cur_cmd() == MATH_SHIFT && cur_list_mode() > 0 {
        let mut j = TEX_NULL;
        let mut w = -MAX_HALFWORD;
        let x;

        if cur_list_head() == cur_list_tail() {
            crate::mem::pop_nest();
            x = if cur_list_e_tex_aux() == TEX_NULL {
                0
            } else if mem_b32s0(cur_list_e_tex_aux()) >= R_CODE {
                -1
            } else {
                1
            };
        } else {
            crate::linebreak::line_break(true);

            j = if eqtb_b32s1(GLUE_BASE + GLUE_PAR_RIGHT_SKIP) == 0 {
                crate::mem::new_kern_global(0)
            } else {
                crate::mem::new_param_glue_global(GLUE_PAR_RIGHT_SKIP as i16)
            };

            let p_glue = if eqtb_b32s1(GLUE_BASE + GLUE_PAR_LEFT_SKIP) == 0 {
                crate::mem::new_kern_global(0)
            } else {
                crate::mem::new_param_glue_global(GLUE_PAR_LEFT_SKIP as i16)
            };
            set_mem_b32s1(p_glue, j);

            j = crate::mem::new_null_box_global();
            set_mem_b32s1(j + 1, mem_b32s1(just_box() + 1));
            set_mem_b32s1(j + 4, mem_b32s1(just_box() + 4));
            set_mem_b32s1(j + 5, p_glue);
            set_mem_b16s0(j + 5, mem_b16s0(just_box() + 5));
            set_mem_b16s1(j + 5, mem_b16s1(just_box() + 5));
            set_mem_gr(j + 6, mem_gr(just_box() + 6));

            let mut v = mem_b32s1(just_box() + 4);
            x = if cur_list_e_tex_aux() == TEX_NULL {
                0
            } else if mem_b32s0(cur_list_e_tex_aux()) >= R_CODE {
                -1
            } else {
                1
            };

            let mut p;
            if x >= 0 {
                p = mem_b32s1(just_box() + 5);
                set_mem_b32s1(TEMP_HEAD, TEX_NULL);
            } else {
                v = -v - mem_b32s1(just_box() + 1);
                p = crate::mem::new_math_global(0, BEGIN_L_CODE);
                set_mem_b32s1(TEMP_HEAD, p);
                crate::packaging::just_copy(
                    mem_b32s1(just_box() + 5),
                    p,
                    crate::mem::new_math_global(0, END_L_CODE),
                );
                set_cur_dir(RIGHT_TO_LEFT);
            }

            v += 2 * font_info_b32s1(QUAD_CODE + param_base(eqtb_b32s1(CUR_FONT_LOC)));

            if eqtb_b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
                set_temp_ptr(crate::mem::get_avail_global());
                set_mem_b32s0(temp_ptr(), BEFORE);
                set_mem_b32s1(temp_ptr(), lr_ptr());
                set_lr_ptr(temp_ptr());
            }

            'outer: loop {
                if p == TEX_NULL {
                    break 'outer;
                }

                let is_found: bool;
                let d: i32;

                'reswitch: loop {
                    if p >= hi_mem_min() {
                        let f = mem_b16s1(p) as i32;
                        let c = crate::scan::effective_char_global(true, f, mem_b16s0(p));
                        d = font_info_b32s1(
                            width_base_entry(f) + font_info_b16(char_base_entry(f) + c).s3 as i32,
                        );
                        is_found = true;
                        break 'reswitch;
                    }
                    match mem_b16s1(p) {
                        0..=2 => {
                            d = mem_b32s1(p + 1);
                            is_found = true;
                            break 'reswitch;
                        }
                        6 => {
                            set_mem_word(GARBAGE, mem_word(p + 1));
                            set_mem_b32s1(GARBAGE, mem_b32s1(p));
                            p = GARBAGE;
                            set_xtx_ligature_present(true);
                            continue 'reswitch;
                        }
                        11 | 40 => {
                            d = mem_b32s1(p + 1);
                            is_found = false;
                            break 'reswitch;
                        }
                        9 => {
                            d = mem_b32s1(p + 1);
                            if eqtb_b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
                                if mem_b16s0(p) & 1 != 0 {
                                    if mem_b32s0(lr_ptr())
                                        == L_CODE * (mem_b16s0(p) as i32 / L_CODE) + 3
                                    {
                                        set_temp_ptr(lr_ptr());
                                        set_lr_ptr(mem_b32s1(temp_ptr()));
                                        set_mem_b32s1(temp_ptr(), avail());
                                        set_avail(temp_ptr());
                                    } else if mem_b16s0(p) as i32 > L_CODE {
                                        w = MAX_HALFWORD;
                                        break 'outer;
                                    }
                                } else {
                                    set_temp_ptr(crate::mem::get_avail_global());
                                    set_mem_b32s0(
                                        temp_ptr(),
                                        L_CODE * (mem_b16s0(p) as i32 / L_CODE) + 3,
                                    );
                                    set_mem_b32s1(temp_ptr(), lr_ptr());
                                    set_lr_ptr(temp_ptr());
                                    if mem_b16s0(p) as i32 / R_CODE != cur_dir() as i32 {
                                        crate::packaging::just_reverse(p);
                                        p = TEMP_HEAD;
                                    }
                                }
                            } else if mem_b16s0(p) as i32 >= L_CODE {
                                w = MAX_HALFWORD;
                                break 'outer;
                            }
                            is_found = false;
                            break 'reswitch;
                        }
                        14 => {
                            d = mem_b32s1(p + 1);
                            set_cur_dir(mem_b16s0(p) as i16);
                            is_found = false;
                            break 'reswitch;
                        }
                        10 => {
                            let q = mem_b32s0(p + 1);
                            d = mem_b32s1(q + 1);
                            if mem_b16s1(just_box() + 5) == STRETCHING {
                                if mem_b16s0(just_box() + 5) == mem_b16s1(q)
                                    && mem_b32s1(q + 2) != 0
                                {
                                    v = MAX_HALFWORD;
                                }
                            } else if mem_b16s1(just_box() + 5) == SHRINKING
                                && mem_b16s0(just_box() + 5) == mem_b16s0(q)
                                && mem_b32s1(q + 3) != 0
                            {
                                v = MAX_HALFWORD;
                            }
                            if mem_b16s0(p) >= A_LEADERS {
                                is_found = true;
                                break 'reswitch;
                            }
                            is_found = false;
                            break 'reswitch;
                        }
                        8 => {
                            let st = mem_b16s0(p);
                            if st == NATIVE_WORD_NODE
                                || st == NATIVE_WORD_NODE_AT
                                || st == GLYPH_NODE
                                || st == PIC_NODE
                                || st == PDF_NODE
                            {
                                d = mem_b32s1(p + 1);
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

                p = mem_b32s1(p);
            }

            if eqtb_b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
                while lr_ptr() != TEX_NULL {
                    set_temp_ptr(lr_ptr());
                    set_lr_ptr(mem_b32s1(temp_ptr()));
                    set_mem_b32s1(temp_ptr(), avail());
                    set_avail(temp_ptr());
                }
                if lr_problems() != 0 {
                    w = MAX_HALFWORD;
                    set_lr_problems(0);
                }
            }

            set_cur_dir(LEFT_TO_RIGHT);
            crate::packaging::flush_node_list_global(mem_b32s1(TEMP_HEAD));
        }

        let l;
        let s;
        let par_shape = eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
        if par_shape == TEX_NULL {
            let hang = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_HANG_INDENT);
            let hang_after = eqtb_b32s1(INT_BASE + INT_PAR_HANG_AFTER);
            let prev_graf = cur_list_prev_graf();
            if hang != 0
                && ((hang_after >= 0 && prev_graf + 2 > hang_after) || prev_graf + 1 < -hang_after)
            {
                l = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_HSIZE) - hang.abs();
                s = if hang > 0 { hang } else { 0 };
            } else {
                l = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_HSIZE);
                s = 0;
            }
        } else {
            let n = mem_b32s0(par_shape);
            let p = if cur_list_prev_graf() + 2 >= n {
                par_shape + 2 * n
            } else {
                par_shape + 2 * (cur_list_prev_graf() + 2)
            };
            s = mem_b32s1(p - 1);
            l = mem_b32s1(p);
        }

        crate::main_control::push_math(MATH_SHIFT_GROUP);
        set_cur_list_mode(MMODE);
        crate::save::eq_word_define(INT_BASE + INT_PAR_CUR_FAM, -1);
        crate::save::eq_word_define(DIMEN_BASE + DIMEN_PAR_PRE_DISPLAY_SIZE, w);
        set_cur_list_e_tex_aux(j);
        crate::save::eq_word_define(INT_BASE + INT_PAR_PRE_DISPLAY_DIRECTION, x);
        crate::save::eq_word_define(DIMEN_BASE + DIMEN_PAR_DISPLAY_WIDTH, l);
        crate::save::eq_word_define(DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT, s);

        if semantic_pagination_enabled() {
            crate::ext::tt_insert_special(c"tdux:cs dmath".as_ptr());
        }

        let every_display = eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_DISPLAY);
        if every_display != TEX_NULL {
            begin_token_list(every_display, EVERY_DISPLAY_TEXT);
        }
        if nest_ptr() == 1 {
            crate::pagebuilder::build_page();
        }
    } else {
        crate::input::back_input();
        crate::main_control::push_math(MATH_SHIFT_GROUP);
        crate::save::eq_word_define(INT_BASE + INT_PAR_CUR_FAM, -1);
        if insert_src_special_every_math() {
            crate::ext::insert_src_special();
        }
        if semantic_pagination_enabled() {
            crate::ext::tt_insert_special(c"tdux:cs math".as_ptr());
        }
        let every_math = eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_MATH);
        if every_math != TEX_NULL {
            begin_token_list(every_math, EVERY_MATH_TEXT);
        }
    }
}

/// translated from xetex-math.c:71 (initialize_math_variables)
#[no_mangle]
pub unsafe extern "C" fn initialize_math_variables() {
    // null_delimiter is a file-static b16x4 that is never read outside this
    // function; zeroing it is redundant (C zero-initializes statics) and a no-op.
}

/// translated from xetex-math.c:1356 (rebox)
enum ReboxResult {
    Done(i32),
    Hpack(i32),
}

fn rebox(
    mem: &mut MemState,
    b: i32,
    w: i32,
    char_kern_width: Option<i32>,
) -> Result<ReboxResult, NodeRecycleAbort> {
    let mut b = b;
    if mem.b32s1(b + 1) != w && mem.b32s1(b + 5) != TEX_NULL {
        let mut p = mem.b32s1(b + 5);
        if let Some(v) = char_kern_width {
            if v != mem.b32s1(b + 1) {
                let k = new_kern(mem, mem.b32s1(b + 1) - v);
                if k == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow);
                }
                mem.set_b32s1(p, k);
            }
        }
        free_node(mem, b, BOX_NODE_SIZE);
        b = new_glue(mem, 12);
        if b == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        mem.set_b32s1(b, p);
        while mem.b32s1(p) != TEX_NULL {
            p = mem.b32s1(p);
        }
        let tail_glue = new_glue(mem, 12);
        if tail_glue == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        mem.set_b32s1(p, tail_glue);
        Ok(ReboxResult::Hpack(b))
    } else {
        mem.set_b32s1(b + 1, w);
        Ok(ReboxResult::Done(b))
    }
}

pub unsafe fn rebox_global(b: i32, w: i32) -> i32 {
    let mut b = b;
    if mem_b32s1(b + 1) != w && mem_b32s1(b + 5) != TEX_NULL && mem_b16s1(b) == VLIST_NODE {
        b = crate::packaging::hpack(b, 0, ADDITIONAL);
    }

    let char_kern_width = if mem_b32s1(b + 1) != w && mem_b32s1(b + 5) != TEX_NULL {
        let p = mem_b32s1(b + 5);
        if is_char_node_global(p) && mem_b32s1(p) == TEX_NULL {
            let f = mem_b16s1(p) as i32;
            let c_raw = mem_b16s0(p);
            let q = font_info_b16(
                char_base_entry(f) + crate::scan::effective_char_global(true, f, c_raw),
            );
            Some(font_info_b32s1(width_base_entry(f) + q.s3 as i32))
        } else {
            None
        }
    } else {
        None
    };

    match crate::state::with_state_mut(|st| rebox(&mut st.mem, b, w, char_kern_width)) {
        Ok(ReboxResult::Done(b)) => b,
        Ok(ReboxResult::Hpack(b)) => crate::packaging::hpack(b, w, EXACTLY),
        Err(e) => node_recycle_abort(e),
    }
}

/// translated from xetex-math.c:1206 (build_opentype_assembly)
#[no_mangle]
pub unsafe extern "C" fn build_opentype_assembly(
    f: i32,
    a: *mut libc::c_void,
    s: i32,
    horiz: bool,
) -> i32 {
    let b = crate::mem::new_null_box_global();
    if horiz {
        set_mem_b16s1(b, HLIST_NODE);
    } else {
        set_mem_b16s1(b, VLIST_NODE);
    }
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
                    stack_glue_into_box_global(b, -oo, -o);
                }
                stack_glyph_into_box_global(b, f, ot_part_glyph(a, i));
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
                stack_glue_into_box_global(b, -oo, -o);
            }
            stack_glyph_into_box_global(b, f, ot_part_glyph(a, i));
            prev_o = ot_part_end_connector(f, a, i);
        }
    }
    // Measure natural size and total stretch.
    let mut p = mem_b32s1(b + 5);
    let mut nat: i32 = 0;
    let mut str: i32 = 0;
    while p != TEX_NULL {
        if mem_b16s1(p) == WHATSIT_NODE {
            if horiz {
                nat += mem_b32s1(p + 1);
            } else {
                nat += mem_b32s1(p + 3) + mem_b32s1(p + 2);
            }
        } else if mem_b16s1(p) == GLUE_NODE {
            let gptr = mem_b32s0(p + 1);
            nat += mem_b32s1(gptr + 1);
            str += mem_b32s1(gptr + 2);
        }
        p = mem_b32s1(p);
    }
    if s > nat && str > 0 {
        let o = if s - nat > str { str } else { s - nat };
        set_mem_b16s0(b + 5, NORMAL);
        set_mem_b16s1(b + 5, STRETCHING);
        set_mem_gr(b + 6, o as f64 / str as f64);
        let set_dim = nat + crate::scaled_math::tex_round(str as f64 * mem_gr(b + 6));
        if horiz {
            set_mem_b32s1(b + 1, set_dim);
        } else {
            set_mem_b32s1(b + 3, set_dim);
        }
    } else if horiz {
        set_mem_b32s1(b + 1, nat);
    } else {
        set_mem_b32s1(b + 3, nat);
    }
    b
}

/// Build the extensible (`EXT_TAG`) delimiter box for [`var_delimiter_global`].
///
/// Phase-C (`01b` §4a bucket C, session-81/83 style): the box allocation and
/// all the TFM font reads (the `exten` table row, the repeatable piece's width
/// and height-plus-depth, and the top/mid/bottom piece extents) run under a
/// single `mem`+`fonts` borrow island instead of the ~12 scattered bridge
/// closures the C translation left behind. The repeat-count arithmetic is pure,
/// and the piece stacking uses the already-threaded [`stack_into_box_global`]
/// (which builds each char box + links it under its own borrow). Allocation
/// order is preserved (the vlist box first, then each stacked char box), so the
/// mem image is byte-identical to the inline version.
unsafe fn build_ext_delimiter_box_global(f: i32, q: crate::mem_types::B16x4, v: i32) -> i32 {
    struct ExtGeom {
        b: i32,
        r: crate::mem_types::B16x4,
        u: i32,
        w: i32,
    }
    let ExtGeom { b, r, u, mut w } = match crate::state::with_state_mut(|st| {
        let st = &mut *st;
        let b = crate::mem::new_null_box(&mut st.mem);
        if b == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        st.mem.set_b16s1(b, VLIST_NODE);
        let r = st
            .fonts
            .font_info_b16(st.fonts.exten_base_entry(f) + q.s0 as i32);
        // The repeatable (s0) piece sets the box width and the per-step height.
        let u = height_plus_depth(&mut st.fonts, f, r.s0);
        let effective_c = crate::fonts::effective_char(&mut st.fonts, true, f, r.s0);
        let qq = st
            .fonts
            .font_info_b16(st.fonts.char_base_entry(f) + effective_c);
        let box_width = st
            .fonts
            .font_info_b32s1(st.fonts.width_base_entry(f) + qq.s3 as i32)
            + st.fonts
                .font_info_b32s1(st.fonts.italic_base_entry(f) + qq.s1 as i32 / 4);
        st.mem.set_b32s1(b + 1, box_width);
        let mut w = 0;
        if r.s1 != 0 {
            w += height_plus_depth(&mut st.fonts, f, r.s1);
        }
        if r.s2 != 0 {
            w += height_plus_depth(&mut st.fonts, f, r.s2);
        }
        if r.s3 != 0 {
            w += height_plus_depth(&mut st.fonts, f, r.s3);
        }
        Ok(ExtGeom { b, r, u, w })
    }) {
        Ok(geom) => geom,
        Err(e) => node_recycle_abort(e),
    };
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
        stack_into_box_global(b, f, r.s1);
    }
    for _ in 1..=n {
        stack_into_box_global(b, f, r.s0);
    }
    if r.s2 != 0 {
        stack_into_box_global(b, f, r.s2);
        for _ in 1..=n {
            stack_into_box_global(b, f, r.s0);
        }
    }
    if r.s3 != 0 {
        stack_into_box_global(b, f, r.s3);
    }
    crate::state::with_state_mut(|st| {
        let height = st.mem.b32s1(b + 3);
        st.mem.set_b32s1(b + 2, w - height);
    });
    b
}

/// translated from xetex-math.c:1017 (var_delimiter)
pub unsafe fn var_delimiter_global(d: i32, s: i32, v: i32) -> i32 {
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
    let d_s3 = mem_b16s3(d) as i32;
    let mut z = d_s3 % 256;
    let mut x = mem_b16s2(d) as i32 + (d_s3 / 256) * 65536;
    // Outer loop: try small delimiter family first, then large.
    // All exits break to 'found.
    'found: {
        loop {
            if z != 0 || x != 0 {
                z = z + s + 256;
                // Scan font sizes from s down to SCRIPT_SCRIPT_SIZE.
                loop {
                    z -= 256;
                    let g = crate::state::with_state(|st| math_font(&st.eqtb, z));
                    if g != FONT_BASE {
                        let (font_area, layout_engine) = crate::state::with_state(|st| {
                            (
                                st.fonts.font_area_entry(g),
                                st.fonts.font_layout_engine_ptr(g),
                            )
                        });
                        if font_area == AAT_FONT_FLAG
                            || (font_area == OTGR_FONT_FLAG && usingOpenType(layout_engine))
                        {
                            x = crate::ext::map_char_to_glyph(g, x) as i32;
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
                            let scan = crate::state::with_state(|st| {
                                let fonts = &st.fonts;
                                if y < fonts.font_bc_entry(g) || y > fonts.font_ec_entry(g) {
                                    return None;
                                }
                                // Follow character list chain, looking for tallest glyph.
                                loop {
                                    let q_tmp =
                                        fonts.font_info_b16(fonts.char_base_entry(g) + y as i32);
                                    q = q_tmp;
                                    if q.s3 > 0 {
                                        if q.s1 % 4 == EXT_TAG {
                                            return Some((g, y, w, q, true));
                                        }
                                        let u = fonts.font_info_b32s1(
                                            fonts.height_base_entry(g) + q.s2 as i32 / 16,
                                        ) + fonts.font_info_b32s1(
                                            fonts.depth_base_entry(g) + q.s2 as i32 % 16,
                                        );
                                        if u > w {
                                            if u >= v {
                                                return Some((g, y, u, q, true));
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
                            });
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
            (z, x) = crate::state::with_state(|st| {
                (
                    st.mem.b16s1(d) as i32 % 256,
                    st.mem.b16s0(d) as i32 + (st.mem.b16s1(d) as i32 / 256) * 65536,
                )
            });
        }
    }
    // found:
    let b;
    if f != FONT_BASE {
        let (font_area, layout_engine) = crate::state::with_state(|st| {
            (
                st.fonts.font_area_entry(f),
                st.fonts.font_layout_engine_ptr(f),
            )
        });
        if !(font_area == OTGR_FONT_FLAG && usingOpenType(layout_engine)) {
            if q.s1 % 4 == EXT_TAG {
                b = build_ext_delimiter_box_global(f, q, v);
            } else {
                b = char_box_global(f, c as i32);
            }
        } else if !ot_assembly_ptr.is_null() {
            b = build_opentype_assembly(f, ot_assembly_ptr, v, false);
        } else {
            b = native_glyph_vbox_global(f, c);
        }
    } else {
        // Null delimiter: an empty box of \nulldelimiterspace width. One borrow
        // island over disjoint `mem` (alloc) + `eqtb` (the dimen read).
        b = match crate::state::with_state_mut(|st| {
            let st = &mut *st;
            let bb = crate::mem::new_null_box(&mut st.mem);
            if bb == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow);
            }
            let width = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_NULL_DELIMITER_SPACE);
            st.mem.set_b32s1(bb + 1, width);
            Ok(bb)
        }) {
            Ok(bb) => bb,
            Err(e) => node_recycle_abort(e),
        };
    }
    let axis = axis_height_global(s);
    crate::state::with_state_mut(|st| {
        let state = &mut *st;
        let shift =
            crate::scaled_math::half(state.mem.b32s1(b + 3) - state.mem.b32s1(b + 2)) - axis;
        state.mem.set_b32s1(b + 4, shift);
    });
    free_ot_assembly(ot_assembly_ptr);
    b
}
