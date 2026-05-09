//! Math-mode command helpers translated from `xetex/xetex-math.c`.

use crate::globals::{
    adjust_tail, avail, char_base_entry, cur_chr, cur_cmd, cur_dir, cur_group, cur_lang,
    cur_c, cur_f, cur_i, depth_base_entry, height_base_entry, italic_base_entry,
    kern_base_entry, lig_kern_base_entry, skew_char_entry,
    cur_list_aux_b32s1, cur_list_e_tex_aux, cur_list_head, cur_list_mode,
    cur_list_prev_graf, cur_list_tail, cur_size, cur_style, cur_val, cur_val1, eqtb_b32s1,
    file_line_error_style_p, font_area_entry, font_bc_entry, font_ec_entry, font_info_b16,
    font_info_b32s1, font_layout_engine_ptr, font_params, hi_mem_min,
    insert_src_special_every_math, just_box, lr_problems, lr_ptr, mem_b16s0, mem_b16s1,
    mem_b16s3, mem_b32s0, mem_b32s1, mem_gr, mem_node_addr, mem_word, nest_ptr, null_character,
    param_base, pre_adjust_tail, save_ptr, save_stack_b32s1, semantic_pagination_enabled,
    set_adjust_tail, set_avail, set_cur_c, set_cur_dir, set_cur_f, set_cur_i, set_cur_lang,
    set_cur_list_aux_b32s0, set_cur_list_aux_b32s1, set_cur_list_e_tex_aux, set_cur_list_mode,
    set_cur_list_prev_graf, set_cur_list_tail, set_cur_mlist, set_cur_mu, set_cur_size,
    set_cur_style, set_cur_val, set_cur_val1, set_help_line, set_help_ptr, set_lr_problems,
    set_lr_ptr, set_mem_b16s0, set_mem_b16s1, set_mem_b16s2, set_mem_b16s3, set_mem_b32s0,
    set_mem_b32s1, set_mem_gr, set_mem_word, set_mlist_penalties, set_pre_adjust_tail,
    set_save_ptr, set_save_stack_b32s1, set_temp_ptr, set_xtx_ligature_present, temp_ptr,
    tex_remainder, total_shrink, width_base_entry,
};
use crate::mem::{free_node, get_node, new_choice, new_glue, new_kern, new_noad, new_rule, new_spec, TEX_NULL};

const INT_BASE: i32 = 7_826_729;
const INT_PAR_CUR_FAM: i32 = 44;
const LOCAL_BASE: i32 = 2_254_871;
const LOCAL_EVERY_MATH: i32 = 3;
const DEL_CODE_BASE: i32 = 7_827_068;

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
const WHATSIT_NODE: u16 = 8;
const GLYPH_NODE_SIZE: i32 = 5;
const DISPLAYOPERATORMINHEIGHT: i32 = 3;
const RADICAL_NOAD: u16 = 24;
const FRACTION_NOAD: u16 = 25;
const OP_NOAD: u16 = 17;
const INNER_NOAD: u16 = 23;
const LEFT_NOAD: u16 = 30;
const RIGHT_NOAD: u16 = 31;
const ORD_NOAD: u16 = 16;
const PUNCT_NOAD: u16 = 22;
const LIG_TAG: u16 = 1;
const MATH_TEXT_CHAR: i32 = 4;
const NOAD_SIZE: i32 = 4;
const HLIST_NODE: u16 = 0;
const INT_PAR_DELIMITER_FACTOR: i32 = 18;
const DIMEN_PAR_DELIMITER_SHORTFALL: i32 = 10;
const ACCENTBASEHEIGHT: i32 = 6;
const X_HEIGHT_CODE: i32 = 5;
const RADICAL_NOAD_SIZE: i32 = 5;
const FRACTION_NOAD_SIZE: i32 = 6;

const MATH_SHIFT: u8 = 3;
const MMODE: i16 = 207;
const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
const GLUE_BASE: i32 = 2_254_340;
const DIMEN_BASE: i32 = 8_941_180;
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
const CUR_FONT_LOC: i32 = 2_255_400;
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

const MATH_FONT_BASE: i32 = 2_255_401;
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
const BEGIN_M_CODE: i16 = 2;
const END_M_CODE: i16 = 3;
const DISPLAY_STYLE: i16 = 0;
const TEXT_STYLE: i16 = 2;
const SCRIPT_STYLE: i16 = 4;
const BOX_NODE_SIZE: i32 = 8;
const ADJUST_HEAD: i32 = 4_999_994;
const PRE_ADJUST_HEAD: i32 = 4_999_985;
const FIL: usize = 1;
const FILL: usize = 2;
const FILLL: usize = 3;
const INF_PENALTY: i32 = 10000;
const DIMEN_PAR_MATH_SURROUND: i32 = 1;
const INT_PAR_PRE_DISPLAY_PENALTY: i32 = 11;
const INT_PAR_POST_DISPLAY_PENALTY: i32 = 12;
const GLUE_PAR_ABOVE_DISPLAY_SKIP: i16 = 3;
const GLUE_PAR_BELOW_DISPLAY_SKIP: i16 = 4;
const GLUE_PAR_ABOVE_DISPLAY_SHORT_SKIP: i16 = 5;
const GLUE_PAR_BELOW_DISPLAY_SHORT_SKIP: i16 = 6;
const ADDITIONAL: i16 = 1;
const EXACTLY: i16 = 0;
const AFTER: i16 = 1;
const GLUE_NODE: u16 = 10;
const DLIST: u16 = 2;
const GLUE_SPEC_SIZE: i32 = 4;
const MU_GLUE: u16 = 99;
const EXPLICIT: u16 = 1;
const VLIST_NODE: u16 = 1;
const KERN_NODE: u16 = 11;
const MEDIUM_NODE_SIZE: i32 = 3;
const SUB_BOX: i32 = 2;

extern "C" {
    fn begin_token_list(p: i32, t: u16);
    fn mlist_to_hlist();
    fn isOpenTypeMathFont(engine: *mut libc::c_void) -> bool;
    fn usingOpenType(engine: *mut libc::c_void) -> bool;
    fn get_native_mathsy_param(f: i32, n: i32) -> i32;
    fn get_native_mathex_param(f: i32, n: i32) -> i32;
    fn real_get_native_glyph(p_node: *mut libc::c_void, index: u32) -> u16;
    fn get_ot_math_accent_pos(f: i32, g: i32) -> i32;
    fn get_ot_math_constant(f: i32, n: i32) -> i32;
    fn get_ot_math_variant(f: i32, g: i32, v: i32, adv: *mut i32, horiz: i32) -> i32;
    fn get_ot_assembly_ptr(f: i32, g: i32, horiz: i32) -> *mut libc::c_void;
    fn get_ot_math_ital_corr(f: i32, g: i32) -> i32;
    fn free_ot_assembly(a: *mut libc::c_void);
    fn build_opentype_assembly(f: i32, a: *mut libc::c_void, s: i32, horiz: bool) -> i32;
    fn var_delimiter(d: i32, s: i32, v: i32) -> i32;
}

#[inline]
unsafe fn set_empty_math_field(p: i32) {
    set_mem_b32s1(p, EMPTY);
    set_mem_b32s0(p, TEX_NULL);
}

#[inline]
unsafe fn math_font(n: i32) -> i32 {
    eqtb_b32s1(MATH_FONT_BASE + n)
}

#[inline]
unsafe fn math_sy_param(size_code: i32, param: i32) -> i32 {
    let f = math_font(2 + size_code);
    if font_is_ot_math(f) {
        get_native_mathsy_param(f, param)
    } else {
        font_info_b32s1(param + param_base(f))
    }
}

#[inline]
unsafe fn math_ext_param(param: i32) -> i32 {
    let f = math_font(3 + cur_size());
    if font_is_ot_math(f) {
        get_native_mathex_param(f, param)
    } else {
        font_info_b32s1(param + param_base(f))
    }
}

/// translated from xetex-math.c:86 (math_x_height)
#[no_mangle]
pub unsafe extern "C" fn math_x_height(size_code: i32) -> i32 {
    math_sy_param(size_code, 5)
}

/// translated from xetex-math.c:101 (math_quad)
#[no_mangle]
pub unsafe extern "C" fn math_quad(size_code: i32) -> i32 {
    math_sy_param(size_code, 6)
}

/// translated from xetex-math.c:115 (num1)
#[no_mangle]
pub unsafe extern "C" fn num1(size_code: i32) -> i32 {
    math_sy_param(size_code, 8)
}

/// translated from xetex-math.c:130 (num2)
#[no_mangle]
pub unsafe extern "C" fn num2(size_code: i32) -> i32 {
    math_sy_param(size_code, 9)
}

/// translated from xetex-math.c:145 (num3)
#[no_mangle]
pub unsafe extern "C" fn num3(size_code: i32) -> i32 {
    math_sy_param(size_code, 10)
}

/// translated from xetex-math.c:160 (denom1)
#[no_mangle]
pub unsafe extern "C" fn denom1(size_code: i32) -> i32 {
    math_sy_param(size_code, 11)
}

/// translated from xetex-math.c:175 (denom2)
#[no_mangle]
pub unsafe extern "C" fn denom2(size_code: i32) -> i32 {
    math_sy_param(size_code, 12)
}

/// translated from xetex-math.c:190 (sup1)
#[no_mangle]
pub unsafe extern "C" fn sup1(size_code: i32) -> i32 {
    math_sy_param(size_code, 13)
}

/// translated from xetex-math.c:205 (sup2)
#[no_mangle]
pub unsafe extern "C" fn sup2(size_code: i32) -> i32 {
    math_sy_param(size_code, 14)
}

/// translated from xetex-math.c:219 (sup3)
#[no_mangle]
pub unsafe extern "C" fn sup3(size_code: i32) -> i32 {
    math_sy_param(size_code, 15)
}

/// translated from xetex-math.c:233 (sub1)
#[no_mangle]
pub unsafe extern "C" fn sub1(size_code: i32) -> i32 {
    math_sy_param(size_code, 16)
}

/// translated from xetex-math.c:247 (sub2)
#[no_mangle]
pub unsafe extern "C" fn sub2(size_code: i32) -> i32 {
    math_sy_param(size_code, 17)
}

/// translated from xetex-math.c:261 (sup_drop)
#[no_mangle]
pub unsafe extern "C" fn sup_drop(size_code: i32) -> i32 {
    math_sy_param(size_code, 18)
}

/// translated from xetex-math.c:275 (sub_drop)
#[no_mangle]
pub unsafe extern "C" fn sub_drop(size_code: i32) -> i32 {
    math_sy_param(size_code, 19)
}

/// translated from xetex-math.c:289 (delim1)
#[no_mangle]
pub unsafe extern "C" fn delim1(size_code: i32) -> i32 {
    math_sy_param(size_code, 20)
}

/// translated from xetex-math.c:303 (delim2)
#[no_mangle]
pub unsafe extern "C" fn delim2(size_code: i32) -> i32 {
    math_sy_param(size_code, 21)
}

/// translated from xetex-math.c:317 (axis_height)
#[no_mangle]
pub unsafe extern "C" fn axis_height(size_code: i32) -> i32 {
    math_sy_param(size_code, 22)
}

/// translated from xetex-math.c:88 (default_rule_thickness)
#[no_mangle]
pub unsafe extern "C" fn default_rule_thickness() -> i32 {
    math_ext_param(8)
}

/// translated from xetex-math.c:102 (big_op_spacing1)
#[no_mangle]
pub unsafe extern "C" fn big_op_spacing1() -> i32 {
    math_ext_param(9)
}

/// translated from xetex-math.c:116 (big_op_spacing2)
#[no_mangle]
pub unsafe extern "C" fn big_op_spacing2() -> i32 {
    math_ext_param(10)
}

/// translated from xetex-math.c:130 (big_op_spacing3)
#[no_mangle]
pub unsafe extern "C" fn big_op_spacing3() -> i32 {
    math_ext_param(11)
}

/// translated from xetex-math.c:144 (big_op_spacing4)
#[no_mangle]
pub unsafe extern "C" fn big_op_spacing4() -> i32 {
    math_ext_param(12)
}

/// translated from xetex-math.c:158 (big_op_spacing5)
#[no_mangle]
pub unsafe extern "C" fn big_op_spacing5() -> i32 {
    math_ext_param(13)
}

/// translated from xetex-math.c:170 (fraction_rule)
#[no_mangle]
pub unsafe extern "C" fn fraction_rule(t: i32) -> i32 {
    let p = new_rule();
    set_mem_b32s1(p + 3, t);
    set_mem_b32s1(p + 2, 0);
    p
}

/// translated from xetex-math.c:181 (overbar)
#[no_mangle]
pub unsafe extern "C" fn overbar(b: i32, k: i32, t: i32) -> i32 {
    let mut p = new_kern(k);
    set_mem_b32s1(p, b);
    let q = fraction_rule(t);
    set_mem_b32s1(q, p);
    p = new_kern(t);
    set_mem_b32s1(p, q);
    crate::scan::vpackage(p, 0, ADDITIONAL, MAX_HALFWORD)
}

/// translated from xetex-math.c:195 (math_glue)
#[no_mangle]
pub unsafe extern "C" fn math_glue(g: i32, m: i32) -> i32 {
    let mut n = crate::scaled_math::x_over_n(m, 65_536);
    let mut f = tex_remainder();
    if f < 0 {
        n -= 1;
        f += 65_536;
    }

    let p = get_node(GLUE_SPEC_SIZE);
    set_mem_b32s1(
        p + 1,
        crate::scaled_math::mult_and_add(
            n,
            mem_b32s1(g + 1),
            crate::scaled_math::xn_over_d(mem_b32s1(g + 1), f, 65_536),
            MAX_HALFWORD,
        ),
    );
    set_mem_b16s1(p, mem_b16s1(g));
    if mem_b16s1(p) == NORMAL {
        set_mem_b32s1(
            p + 2,
            crate::scaled_math::mult_and_add(
                n,
                mem_b32s1(g + 2),
                crate::scaled_math::xn_over_d(mem_b32s1(g + 2), f, 65_536),
                MAX_HALFWORD,
            ),
        );
    } else {
        set_mem_b32s1(p + 2, mem_b32s1(g + 2));
    }
    set_mem_b16s0(p, mem_b16s0(g));
    if mem_b16s0(p) == NORMAL {
        set_mem_b32s1(
            p + 3,
            crate::scaled_math::mult_and_add(
                n,
                mem_b32s1(g + 3),
                crate::scaled_math::xn_over_d(mem_b32s1(g + 3), f, 65_536),
                MAX_HALFWORD,
            ),
        );
    } else {
        set_mem_b32s1(p + 3, mem_b32s1(g + 3));
    }
    p
}

/// translated from xetex-math.c:223 (math_kern)
#[no_mangle]
pub unsafe extern "C" fn math_kern(p: i32, m: i32) {
    if mem_b16s0(p) == MU_GLUE {
        let mut n = crate::scaled_math::x_over_n(m, 65_536);
        let mut f = tex_remainder();
        if f < 0 {
            n -= 1;
            f += 65_536;
        }
        set_mem_b32s1(
            p + 1,
            crate::scaled_math::mult_and_add(
                n,
                mem_b32s1(p + 1),
                crate::scaled_math::xn_over_d(mem_b32s1(p + 1), f, 65_536),
                MAX_HALFWORD,
            ),
        );
        set_mem_b16s0(p, EXPLICIT);
    }
}

/// translated from xetex-math.c:80 (flush_math)
#[no_mangle]
pub unsafe extern "C" fn flush_math() {
    crate::scan::flush_node_list(mem_b32s1(cur_list_head()));
    crate::scan::flush_node_list(cur_list_aux_b32s1());
    set_mem_b32s1(cur_list_head(), TEX_NULL);
    set_cur_list_tail(cur_list_head());
    set_cur_list_aux_b32s1(TEX_NULL);
}

#[inline]
unsafe fn is_char_node(p: i32) -> bool {
    p >= hi_mem_min()
}

/// translated from xetex-math.c:91 (clean_box)
#[no_mangle]
pub unsafe extern "C" fn clean_box(p: i32, s: i16) -> i32 {
    let q;
    match mem_b32s1(p) {
        MATH_CHAR => {
            let mlist = new_noad();
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
            set_cur_mu(crate::scaled_math::x_over_n(math_quad(cur_size()), 18));
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
            set_cur_mu(crate::scaled_math::x_over_n(math_quad(cur_size()), 18));
        }
        _ => {
            q = crate::mem::new_null_box();
        }
    }

    let x = if is_char_node(q) || q == TEX_NULL {
        crate::scan::hpack(q, 0, ADDITIONAL)
    } else if mem_b32s1(q) == TEX_NULL && mem_b16s1(q) <= VLIST_NODE && mem_b32s1(q + 4) == 0 {
        q
    } else {
        crate::scan::hpack(q, 0, ADDITIONAL)
    };

    let q = mem_b32s1(x + 5);
    if is_char_node(q) {
        let r = mem_b32s1(q);
        if r != TEX_NULL && mem_b32s1(r) == TEX_NULL && !is_char_node(r) && mem_b16s1(r) == KERN_NODE {
            free_node(r, MEDIUM_NODE_SIZE);
            set_mem_b32s1(q, TEX_NULL);
        }
    }
    x
}

/// translated from xetex-math.c:115 (fetch)
#[no_mangle]
pub unsafe extern "C" fn fetch(a: i32) {
    set_cur_c(mem_b16s0(a) as i32);
    set_cur_f(math_font((mem_b16s1(a) as i32 % 256) + cur_size()));
    set_cur_c(cur_c() + (mem_b16s1(a) as i32 / 256) * 65_536);

    if cur_f() == FONT_BASE {
        crate::output::error_here_with_diagnostic(c"".as_ptr());
        crate::output::print_size(cur_size());
        crate::output::print_char(' ' as i32);
        crate::output::print_int(mem_b16s1(a) as i32 % 256);
        crate::output::print_cstr(c" is undefined (character ".as_ptr());
        crate::output::print(cur_c());
        crate::output::print_char(')' as i32);
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(4);
        set_help_line(3, c"Somewhere in the math formula just ended, you used the".as_ptr());
        set_help_line(2, c"stated character from an undefined font family. For example,".as_ptr());
        set_help_line(1, c"plain TeX doesn't allow \\it or \\sl in subscripts. Proceed,".as_ptr());
        set_help_line(0, c"and I'll try to forget that I needed that character.".as_ptr());
        crate::errors::error();
        set_cur_i(null_character());
        set_mem_b32s1(a, EMPTY);
    } else if font_area_entry(cur_f()) == AAT_FONT_FLAG || font_area_entry(cur_f()) == OTGR_FONT_FLAG {
        set_cur_i(null_character());
    } else {
        if cur_c() >= font_bc_entry(cur_f()) as i32 && cur_c() <= font_ec_entry(cur_f()) as i32 {
            set_cur_i(font_info_b16(char_base_entry(cur_f()) + cur_c()));
        } else {
            set_cur_i(null_character());
        }
        if cur_i().s3 == 0 {
            crate::scan::char_warning(cur_f(), cur_c());
            set_mem_b32s1(a, EMPTY);
            set_cur_i(null_character());
        }
    }
}

/// translated from xetex-math.c:198 (make_over)
#[no_mangle]
pub unsafe extern "C" fn make_over(q: i32) {
    set_mem_b32s0(
        q + 1,
        overbar(
            clean_box(q + 1, 2 * (cur_style() / 2) + 1),
            3 * default_rule_thickness(),
            default_rule_thickness(),
        ),
    );
    set_mem_b32s1(q + 1, SUB_BOX);
}

/// translated from xetex-math.c:207 (make_under)
#[no_mangle]
pub unsafe extern "C" fn make_under(q: i32) {
    let x = clean_box(q + 1, cur_style());
    let p = new_kern(3 * default_rule_thickness());
    set_mem_b32s1(x, p);
    set_mem_b32s1(p, fraction_rule(default_rule_thickness()));
    let y = crate::scan::vpackage(x, 0, ADDITIONAL, MAX_HALFWORD);
    let delta = mem_b32s1(y + 3) + mem_b32s1(y + 2) + default_rule_thickness();
    set_mem_b32s1(y + 3, mem_b32s1(x + 3));
    set_mem_b32s1(y + 2, delta - mem_b32s1(y + 3));
    set_mem_b32s0(q + 1, y);
    set_mem_b32s1(q + 1, SUB_BOX);
}

/// translated from xetex-math.c:225 (make_vcenter)
#[no_mangle]
pub unsafe extern "C" fn make_vcenter(q: i32) {
    let v = mem_b32s0(q + 1);
    if mem_b16s1(v) != VLIST_NODE {
        crate::errors::confusion(c"vcenter".as_ptr());
    }
    let delta = mem_b32s1(v + 3) + mem_b32s1(v + 2);
    set_mem_b32s1(v + 3, axis_height(cur_size()) + crate::scaled_math::half(delta));
    set_mem_b32s1(v + 2, delta - mem_b32s1(v + 3));
}

/// translated from xetex-math.c:80 (make_radical)
#[no_mangle]
pub unsafe extern "C" fn make_radical(q: i32) {
    let f = math_font((mem_b16s3(q + 4) as i32 % 256) + cur_size());
    let rule_thickness = if font_is_ot_math(f) {
        get_ot_math_constant(f, RADICALRULETHICKNESS)
    } else {
        default_rule_thickness()
    };

    let x = clean_box(q + 1, 2 * (cur_style() / 2) + 1);
    let mut clr = if font_is_ot_math(f) {
        if cur_style() < TEXT_STYLE {
            get_ot_math_constant(f, RADICALDISPLAYSTYLEVERTICALGAP)
        } else {
            get_ot_math_constant(f, RADICALVERTICALGAP)
        }
    } else if cur_style() < TEXT_STYLE {
        rule_thickness + math_x_height(cur_size()).abs() / 4
    } else {
        rule_thickness + rule_thickness.abs() / 4
    };

    let y = var_delimiter(
        q + 4,
        cur_size(),
        mem_b32s1(x + 3) + mem_b32s1(x + 2) + clr + rule_thickness,
    );
    if font_is_ot_math(f) {
        set_mem_b32s1(y + 2, mem_b32s1(y + 3) + mem_b32s1(y + 2) - rule_thickness);
        set_mem_b32s1(y + 3, rule_thickness);
    }

    let delta = mem_b32s1(y + 2) - (mem_b32s1(x + 3) + mem_b32s1(x + 2) + clr);
    if delta > 0 {
        clr += crate::scaled_math::half(delta);
    }
    set_mem_b32s1(y + 4, -(mem_b32s1(x + 3) + clr));
    set_mem_b32s1(y, overbar(x, clr, mem_b32s1(y + 3)));
    set_mem_b32s0(q + 1, crate::scan::hpack(y, 0, ADDITIONAL));
    set_mem_b32s1(q + 1, SUB_BOX);
}

/// translated from xetex-math.c:313 (make_fraction)
#[no_mangle]
pub unsafe extern "C" fn make_fraction(q: i32) {
    if mem_b32s1(q + 1) == DEFAULT_CODE {
        set_mem_b32s1(q + 1, default_rule_thickness());
    }

    let mut x = clean_box(q + 2, cur_style() + 2 - 2 * (cur_style() / 6));
    let mut z = clean_box(q + 3, 2 * (cur_style() / 2) + 3 - 2 * (cur_style() / 6));

    if mem_b32s1(x + 1) < mem_b32s1(z + 1) {
        x = rebox(x, mem_b32s1(z + 1));
    } else {
        z = rebox(z, mem_b32s1(x + 1));
    }

    let (mut shift_up, mut shift_down) = if cur_style() < TEXT_STYLE {
        (num1(cur_size()), denom1(cur_size()))
    } else {
        (
            if mem_b32s1(q + 1) != 0 {
                num2(cur_size())
            } else {
                num3(cur_size())
            },
            denom2(cur_size()),
        )
    };

    let mut delta = 0;
    if mem_b32s1(q + 1) == 0 {
        let clr = if font_is_ot_math(cur_f()) {
            if cur_style() < TEXT_STYLE {
                get_ot_math_constant(cur_f(), STACKDISPLAYSTYLEGAPMIN)
            } else {
                get_ot_math_constant(cur_f(), STACKGAPMIN)
            }
        } else if cur_style() < TEXT_STYLE {
            7 * default_rule_thickness()
        } else {
            3 * default_rule_thickness()
        };

        delta = crate::scaled_math::half(
            clr - ((shift_up - mem_b32s1(x + 2)) - (mem_b32s1(z + 3) - shift_down)),
        );
        if delta > 0 {
            shift_up += delta;
            shift_down += delta;
        }
    } else {
        let (delta1, delta2) = if font_is_ot_math(cur_f()) {
            delta = crate::scaled_math::half(mem_b32s1(q + 1));
            let clr_num = if cur_style() < TEXT_STYLE {
                get_ot_math_constant(cur_f(), FRACTIONNUMDISPLAYSTYLEGAPMIN)
            } else {
                get_ot_math_constant(cur_f(), FRACTIONNUMERATORGAPMIN)
            };
            let delta1 =
                clr_num - ((shift_up - mem_b32s1(x + 2)) - (axis_height(cur_size()) + delta));

            let clr_den = if cur_style() < TEXT_STYLE {
                get_ot_math_constant(cur_f(), FRACTIONDENOMDISPLAYSTYLEGAPMIN)
            } else {
                get_ot_math_constant(cur_f(), FRACTIONDENOMINATORGAPMIN)
            };
            let delta2 =
                clr_den - ((axis_height(cur_size()) - delta) - (mem_b32s1(z + 3) - shift_down));
            (delta1, delta2)
        } else {
            let clr = if cur_style() < TEXT_STYLE {
                3 * mem_b32s1(q + 1)
            } else {
                mem_b32s1(q + 1)
            };
            delta = crate::scaled_math::half(mem_b32s1(q + 1));
            (
                clr - ((shift_up - mem_b32s1(x + 2)) - (axis_height(cur_size()) + delta)),
                clr - ((axis_height(cur_size()) - delta) - (mem_b32s1(z + 3) - shift_down)),
            )
        };

        if delta1 > 0 {
            shift_up += delta1;
        }
        if delta2 > 0 {
            shift_down += delta2;
        }
    }

    let v = crate::mem::new_null_box();
    set_mem_b16s1(v, VLIST_NODE);
    set_mem_b32s1(v + 3, shift_up + mem_b32s1(x + 3));
    set_mem_b32s1(v + 2, mem_b32s1(z + 2) + shift_down);
    set_mem_b32s1(v + 1, mem_b32s1(x + 1));

    let mut p;
    if mem_b32s1(q + 1) == 0 {
        p = new_kern((shift_up - mem_b32s1(x + 2)) - (mem_b32s1(z + 3) - shift_down));
        set_mem_b32s1(p, z);
    } else {
        let y = fraction_rule(mem_b32s1(q + 1));
        p = new_kern((axis_height(cur_size()) - delta) - (mem_b32s1(z + 3) - shift_down));
        set_mem_b32s1(y, p);
        set_mem_b32s1(p, z);
        p = new_kern((shift_up - mem_b32s1(x + 2)) - (axis_height(cur_size()) + delta));
        set_mem_b32s1(p, y);
    }

    set_mem_b32s1(x, p);
    set_mem_b32s1(v + 5, x);

    delta = if cur_style() < TEXT_STYLE {
        delim1(cur_size())
    } else {
        delim2(cur_size())
    };

    x = var_delimiter(q + 4, cur_size(), delta);
    set_mem_b32s1(x, v);
    z = var_delimiter(q + 5, cur_size(), delta);
    set_mem_b32s1(v, z);
    set_mem_b32s1(q + 1, crate::scan::hpack(x, 0, ADDITIONAL));
}

/// translated from xetex-math.c:270 (make_op)
#[no_mangle]
pub unsafe extern "C" fn make_op(q: i32) -> i32 {
    let mut delta: i32 = 0;
    let mut ot_assembly_ptr: *mut libc::c_void = std::ptr::null_mut();

    if mem_b16s0(q) == NORMAL && cur_style() < TEXT_STYLE {
        set_mem_b16s0(q, LIMITS);
    }

    if mem_b32s1(q + 1) == MATH_CHAR {
        fetch(q + 1);
        if !(font_area_entry(cur_f()) == OTGR_FONT_FLAG
            && usingOpenType(font_layout_engine_ptr(cur_f())))
        {
            if cur_style() < TEXT_STYLE && cur_i().s1 % 4 == LIST_TAG {
                let c = cur_i().s0;
                let i_info = font_info_b16(char_base_entry(cur_f()) + c as i32);
                if i_info.s3 > 0 {
                    set_cur_c(c as i32);
                    set_cur_i(i_info);
                    set_mem_b16s0(q + 1, c);
                }
            }
            delta = font_info_b32s1(italic_base_entry(cur_f()) + cur_i().s1 as i32 / 4);
        }
        let mut x = clean_box(q + 1, cur_style());
        if font_area_entry(cur_f()) == OTGR_FONT_FLAG
            && isOpenTypeMathFont(font_layout_engine_ptr(cur_f()))
        {
            let mut p = mem_b32s1(x + 5);
            if p != TEX_NULL
                && !is_char_node(p)
                && mem_b16s1(p) == WHATSIT_NODE
                && mem_b16s0(p) == GLYPH_NODE
            {
                'found: {
                    if cur_style() < TEXT_STYLE {
                        let mut h1 = get_ot_math_constant(cur_f(), DISPLAYOPERATORMINHEIGHT);
                        let size_h =
                            (mem_b32s1(p + 3) + mem_b32s1(p + 2)) as f64 * 5.0 / 4.0;
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
                                free_node(p, GLYPH_NODE_SIZE);
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
        if mem_b32s1(q + 3) != EMPTY && mem_b16s0(q) != LIMITS {
            set_mem_b32s1(x + 1, mem_b32s1(x + 1) - delta);
        }
        set_mem_b32s1(
            x + 4,
            crate::scaled_math::half(mem_b32s1(x + 3) - mem_b32s1(x + 2))
                - axis_height(cur_size()),
        );
        set_mem_b32s1(q + 1, SUB_BOX);
        set_mem_b32s0(q + 1, x);
    }

    let save_f = cur_f();
    if mem_b16s0(q) == LIMITS {
        let mut x = clean_box(q + 2, 2 * (cur_style() / 4) + 4 + (cur_style() % 2));
        let mut y = clean_box(q + 1, cur_style());
        let mut z = clean_box(q + 3, 2 * (cur_style() / 4) + 5);
        let v = crate::mem::new_null_box();
        set_mem_b16s1(v, VLIST_NODE);
        set_mem_b32s1(v + 1, mem_b32s1(y + 1));
        if mem_b32s1(x + 1) > mem_b32s1(v + 1) {
            set_mem_b32s1(v + 1, mem_b32s1(x + 1));
        }
        if mem_b32s1(z + 1) > mem_b32s1(v + 1) {
            set_mem_b32s1(v + 1, mem_b32s1(z + 1));
        }
        x = rebox(x, mem_b32s1(v + 1));
        y = rebox(y, mem_b32s1(v + 1));
        z = rebox(z, mem_b32s1(v + 1));
        set_mem_b32s1(x + 4, crate::scaled_math::half(delta));
        set_mem_b32s1(z + 4, -mem_b32s1(x + 4));
        set_mem_b32s1(v + 3, mem_b32s1(y + 3));
        set_mem_b32s1(v + 2, mem_b32s1(y + 2));
        set_cur_f(save_f);
        if mem_b32s1(q + 2) == EMPTY {
            free_node(x, BOX_NODE_SIZE);
            set_mem_b32s1(v + 5, y);
        } else {
            let mut shift_up = big_op_spacing3() - mem_b32s1(x + 2);
            if shift_up < big_op_spacing1() {
                shift_up = big_op_spacing1();
            }
            let mut p = new_kern(shift_up);
            set_mem_b32s1(p, y);
            set_mem_b32s1(x, p);
            p = new_kern(big_op_spacing5());
            set_mem_b32s1(p, x);
            set_mem_b32s1(v + 5, p);
            set_mem_b32s1(
                v + 3,
                mem_b32s1(v + 3)
                    + big_op_spacing5()
                    + mem_b32s1(x + 3)
                    + mem_b32s1(x + 2)
                    + shift_up,
            );
        }
        if mem_b32s1(q + 3) == EMPTY {
            free_node(z, BOX_NODE_SIZE);
        } else {
            let mut shift_down = big_op_spacing4() - mem_b32s1(z + 3);
            if shift_down < big_op_spacing2() {
                shift_down = big_op_spacing2();
            }
            let mut p = new_kern(shift_down);
            set_mem_b32s1(y, p);
            set_mem_b32s1(p, z);
            p = new_kern(big_op_spacing5());
            set_mem_b32s1(z, p);
            set_mem_b32s1(
                v + 2,
                mem_b32s1(v + 2)
                    + big_op_spacing5()
                    + mem_b32s1(z + 3)
                    + mem_b32s1(z + 2)
                    + shift_down,
            );
        }
        set_mem_b32s1(q + 1, v);
    }

    free_ot_assembly(ot_assembly_ptr);
    delta
}

/// translated from xetex-math.c:272 (make_ord)
#[no_mangle]
pub unsafe extern "C" fn make_ord(q: i32) {
    'restart: loop {
        if mem_b32s1(q + 3) != EMPTY { break 'restart; }
        if mem_b32s1(q + 2) != EMPTY { break 'restart; }
        if mem_b32s1(q + 1) != MATH_CHAR { break 'restart; }
        let p = mem_b32s1(q);
        if p == TEX_NULL { break 'restart; }
        if mem_b16s1(p) < ORD_NOAD || mem_b16s1(p) > PUNCT_NOAD { break 'restart; }
        if mem_b32s1(p + 1) != MATH_CHAR { break 'restart; }
        if mem_b16s1(p + 1) % 256 != mem_b16s1(q + 1) % 256 { break 'restart; }

        set_mem_b32s1(q + 1, MATH_TEXT_CHAR);
        fetch(q + 1);
        if cur_i().s1 % 4 != LIG_TAG { break 'restart; }

        let mut a = lig_kern_base_entry(cur_f()) + cur_i().s0 as i32;
        set_cur_c(mem_b16s0(p + 1) as i32);
        set_cur_i(font_info_b16(a));
        if cur_i().s3 > 128 {
            a = lig_kern_base_entry(cur_f()) + 256 * cur_i().s1 as i32 + cur_i().s0 as i32 + 32768 - 256 * 128;
            set_cur_i(font_info_b16(a));
        }
        loop {
            if cur_i().s2 == cur_c() as u16 {
                if cur_i().s3 <= 128 {
                    if cur_i().s1 >= 128 {
                        let p_kern = new_kern(font_info_b32s1(
                            kern_base_entry(cur_f()) + 256 * cur_i().s1 as i32 + cur_i().s0 as i32,
                        ));
                        set_mem_b32s1(p_kern, mem_b32s1(q));
                        set_mem_b32s1(q, p_kern);
                        return;
                    } else {
                        match cur_i().s1 {
                            1 | 5 => {
                                set_mem_b16s0(q + 1, cur_i().s0);
                            }
                            2 | 6 => {
                                set_mem_b16s0(p + 1, cur_i().s0);
                            }
                            3 | 7 | 11 => {
                                let r = new_noad();
                                set_mem_b16s0(r + 1, cur_i().s0);
                                set_mem_b16s1(r + 1, mem_b16s1(q + 1) % 256);
                                set_mem_b32s1(q, r);
                                set_mem_b32s1(r, p);
                                if cur_i().s1 < 11 {
                                    set_mem_b32s1(r + 1, MATH_CHAR);
                                } else {
                                    set_mem_b32s1(r + 1, MATH_TEXT_CHAR);
                                }
                            }
                            _ => {
                                set_mem_b32s1(q, mem_b32s1(p));
                                set_mem_b16s0(q + 1, cur_i().s0);
                                set_mem_word(q + 3, mem_word(p + 3));
                                set_mem_word(q + 2, mem_word(p + 2));
                                free_node(p, NOAD_SIZE);
                            }
                        }
                        if cur_i().s1 > 3 {
                            return;
                        }
                        set_mem_b32s1(q + 1, MATH_CHAR);
                        continue 'restart;
                    }
                }
            }
            if cur_i().s3 >= 128 {
                return;
            }
            a += cur_i().s3 as i32 + 1;
            set_cur_i(font_info_b16(a));
        }
    }
}

/// translated from xetex-math.c:79 (make_math_accent)
#[no_mangle]
pub unsafe extern "C" fn make_math_accent(q: i32) {
    fetch(q + 4);
    let mut x: i32 = TEX_NULL;
    let mut ot_assembly_ptr: *mut libc::c_void = std::ptr::null_mut();
    let mut c: i32;
    let mut f: i32;
    let mut s: i32;
    let mut w: i32;
    let mut h: i32;

    if font_area_entry(cur_f()) == AAT_FONT_FLAG || font_area_entry(cur_f()) == OTGR_FONT_FLAG {
        c = cur_c();
        f = cur_f();
        if !(mem_b16s0(q) == BOTTOM_ACC || mem_b16s0(q) == BOTTOM_ACC + 1) {
            s = compute_ot_math_accent_pos(q);
        } else {
            s = 0;
        }
        x = clean_box(q + 1, 2 * (cur_style() / 2) + 1);
        w = mem_b32s1(x + 1);
        h = mem_b32s1(x + 3);
    } else if cur_i().s3 > 0 {
        let mut acc_i = cur_i();
        c = cur_c();
        f = cur_f();
        s = 0;
        'done1: {
            if mem_b32s1(q + 1) == MATH_CHAR {
                fetch(q + 1);
                if cur_i().s1 % 4 == LIG_TAG {
                    let mut a = lig_kern_base_entry(cur_f()) + cur_i().s0 as i32;
                    set_cur_i(font_info_b16(a));
                    if cur_i().s3 > 128 {
                        a = lig_kern_base_entry(cur_f())
                            + 256 * cur_i().s1 as i32
                            + cur_i().s0 as i32
                            + 32768 - 256 * 128;
                        set_cur_i(font_info_b16(a));
                    }
                    loop {
                        if cur_i().s2 == skew_char_entry(cur_f()) as u16 {
                            if cur_i().s1 >= 128 && cur_i().s3 <= 128 {
                                s = font_info_b32s1(
                                    kern_base_entry(cur_f())
                                        + 256 * cur_i().s1 as i32
                                        + cur_i().s0 as i32,
                                );
                            }
                            break 'done1;
                        }
                        if cur_i().s3 >= 128 {
                            break 'done1;
                        }
                        a += cur_i().s3 as i32 + 1;
                        set_cur_i(font_info_b16(a));
                    }
                }
            }
        }
        // done1:
        x = clean_box(q + 1, 2 * (cur_style() / 2) + 1);
        w = mem_b32s1(x + 1);
        h = mem_b32s1(x + 3);
        loop {
            if acc_i.s1 % 4 != LIST_TAG { break; }
            let y_tmp = acc_i.s0 as i32;
            acc_i = font_info_b16(char_base_entry(f) + y_tmp);
            if !(acc_i.s3 > 0) { break; }
            if font_info_b32s1(width_base_entry(f) + acc_i.s3 as i32) > w { break; }
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

    if mem_b32s1(q + 2) != EMPTY || mem_b32s1(q + 3) != EMPTY {
        if mem_b32s1(q + 1) == MATH_CHAR {
            crate::scan::flush_node_list(x);
            x = new_noad();
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
    }

    let y = char_box(f, c);
    let mut p: i32;
    if font_area_entry(f) == AAT_FONT_FLAG || font_area_entry(f) == OTGR_FONT_FLAG {
        p = get_node(GLYPH_NODE_SIZE);
        set_mem_b16s1(p, WHATSIT_NODE);
        set_mem_b16s0(p, GLYPH_NODE);
        set_mem_b16s2(p + 4, f as u16);
        set_mem_b16s1(p + 4, real_get_native_glyph(mem_node_addr(mem_b32s1(y + 5)), 0));
        crate::ext::measure_native_glyph(mem_node_addr(p), 1);
        let old_node = mem_b32s1(y + 5);
        free_node(old_node, mem_b16s3(old_node + 4) as i32);
        set_mem_b32s1(y + 5, p);
        if mem_b16s0(q) % 2 != 0 {
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
                if w2 < 0 || w2 >= w { break; }
            }
            'found: {
                if w2 < 0 {
                    let assembly_ptr = get_ot_assembly_ptr(f, c_glyph, 1);
                    if !assembly_ptr.is_null() {
                        ot_assembly_ptr = assembly_ptr;
                        free_node(p, GLYPH_NODE_SIZE);
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
            if mem_b32s1(y + 3) < 0 { set_mem_b32s1(y + 3, 0); }
        } else if mem_b32s1(y + 2) < 0 {
            set_mem_b32s1(y + 2, 0);
        }
        let sa: i32;
        if p != TEX_NULL && !is_char_node(p) && mem_b16s1(p) == WHATSIT_NODE && mem_b16s0(p) == GLYPH_NODE {
            sa = get_ot_math_accent_pos(f, mem_b16s1(p + 4) as i32);
            let sa = if sa == TEX_INFINITY { crate::scaled_math::half(mem_b32s1(y + 1)) } else { sa };
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
        let y = crate::scan::vpackage(x, 0, ADDITIONAL, MAX_HALFWORD);
        set_mem_b32s1(y + 4, -(h - mem_b32s1(y + 3)));
        set_mem_b32s1(y + 1, mem_b32s1(x + 1));
        set_mem_b32s0(q + 1, y);
        set_mem_b32s1(q + 1, SUB_BOX);
    } else {
        p = new_kern(-delta);
        set_mem_b32s1(p, x);
        set_mem_b32s1(y, p);
        let y = crate::scan::vpackage(y, 0, ADDITIONAL, MAX_HALFWORD);
        if mem_b32s1(y + 3) < h {
            p = new_kern(h - mem_b32s1(y + 3));
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
#[no_mangle]
pub unsafe extern "C" fn attach_hkern_to_new_hlist(q: i32, delta: i32) -> i32 {
    let z = new_kern(delta);
    if mem_b32s1(q + 1) == TEX_NULL {
        set_mem_b32s1(q + 1, z);
    } else {
        let mut y = mem_b32s1(q + 1);
        while mem_b32s1(y) != TEX_NULL {
            y = mem_b32s1(y);
        }
        set_mem_b32s1(y, z);
    }
    mem_b32s1(q + 1)
}

/// translated from xetex-math.c:759 (make_left_right)
#[no_mangle]
pub unsafe extern "C" fn make_left_right(q: i32, style: i16, max_d: i32, max_h: i32) -> i16 {
    set_cur_style(style);
    if cur_style() < SCRIPT_STYLE {
        set_cur_size(TEXT_SIZE);
    } else {
        set_cur_size(SCRIPT_SIZE * ((cur_style() as i32 - 2) / 2));
    }
    set_cur_mu(crate::scaled_math::x_over_n(math_quad(cur_size()), 18));
    let mut delta2 = max_d + axis_height(cur_size());
    let mut delta1 = max_h + max_d - delta2;
    if delta2 > delta1 {
        delta1 = delta2;
    }
    let mut delta = (delta1 / 500) * eqtb_b32s1(INT_BASE + INT_PAR_DELIMITER_FACTOR);
    delta2 = delta1 + delta1 - eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_DELIMITER_SHORTFALL);
    if delta < delta2 {
        delta = delta2;
    }
    set_mem_b32s1(q + 1, var_delimiter(q + 1, cur_size(), delta));
    (mem_b16s1(q) as i32 - (LEFT_NOAD as i32 - 20)) as i16
}

/// translated from xetex-math.c:1432 (char_box)
#[no_mangle]
pub unsafe extern "C" fn char_box(f: i32, c: i32) -> i32 {
    let b;
    let p;
    if font_area_entry(f) == AAT_FONT_FLAG || font_area_entry(f) == OTGR_FONT_FLAG {
        b = crate::mem::new_null_box();
        p = crate::scan::new_native_character(f, c as u32);
        set_mem_b32s1(b + 3, mem_b32s1(p + 3));
        set_mem_b32s1(b + 1, mem_b32s1(p + 1));
        if mem_b32s1(p + 2) < 0 {
            set_mem_b32s1(b + 2, 0);
        } else {
            set_mem_b32s1(b + 2, mem_b32s1(p + 2));
        }
    } else {
        let q = font_info_b16(
            char_base_entry(f) + crate::scan::effective_char(true, f, c as u16) as i32,
        );
        b = crate::mem::new_null_box();
        set_mem_b32s1(
            b + 1,
            font_info_b32s1(width_base_entry(f) + q.s3 as i32)
                + font_info_b32s1(italic_base_entry(f) + q.s1 as i32 / 4),
        );
        set_mem_b32s1(b + 3, font_info_b32s1(height_base_entry(f) + q.s2 as i32 / 16));
        set_mem_b32s1(b + 2, font_info_b32s1(depth_base_entry(f) + q.s2 as i32 % 16));
        p = crate::mem::get_avail();
        set_mem_b16s0(p, c as u16);
        set_mem_b16s1(p, f as u16);
    }
    set_mem_b32s1(b + 5, p);
    b
}

/// translated from xetex-math.c:1463 (stack_into_box)
#[no_mangle]
pub unsafe extern "C" fn stack_into_box(b: i32, f: i32, c: u16) {
    let p = char_box(f, c as i32);
    set_mem_b32s1(p, mem_b32s1(b + 5));
    set_mem_b32s1(b + 5, p);
    set_mem_b32s1(b + 3, mem_b32s1(p + 3));
}

/// translated from xetex-math.c:1474 (height_plus_depth)
#[no_mangle]
pub unsafe extern "C" fn height_plus_depth(f: i32, c: u16) -> i32 {
    let q = font_info_b16(
        char_base_entry(f) + crate::scan::effective_char(true, f, c) as i32,
    );
    font_info_b32s1(height_base_entry(f) + q.s2 as i32 / 16)
        + font_info_b32s1(depth_base_entry(f) + q.s2 as i32 % 16)
}

/// translated from xetex-math.c:1482 (stack_glyph_into_box)
#[no_mangle]
pub unsafe extern "C" fn stack_glyph_into_box(b: i32, f: i32, g: i32) {
    let p = get_node(GLYPH_NODE_SIZE);
    set_mem_b16s1(p, WHATSIT_NODE);
    set_mem_b16s0(p, GLYPH_NODE);
    set_mem_b16s2(p + 4, f as u16);
    set_mem_b16s1(p + 4, g as u16);
    crate::ext::measure_native_glyph(mem_node_addr(p), 1);
    if mem_b16s1(b) == HLIST_NODE {
        let mut q = mem_b32s1(b + 5);
        if q == TEX_NULL {
            set_mem_b32s1(b + 5, p);
        } else {
            while mem_b32s1(q) != TEX_NULL {
                q = mem_b32s1(q);
            }
            set_mem_b32s1(q, p);
            if mem_b32s1(b + 3) < mem_b32s1(p + 3) {
                set_mem_b32s1(b + 3, mem_b32s1(p + 3));
            }
            if mem_b32s1(b + 2) < mem_b32s1(p + 2) {
                set_mem_b32s1(b + 2, mem_b32s1(p + 2));
            }
        }
    } else {
        set_mem_b32s1(p, mem_b32s1(b + 5));
        set_mem_b32s1(b + 5, p);
        set_mem_b32s1(b + 3, mem_b32s1(p + 3));
        if mem_b32s1(b + 1) < mem_b32s1(p + 1) {
            set_mem_b32s1(b + 1, mem_b32s1(p + 1));
        }
    }
}

/// translated from xetex-math.c:1517 (stack_glue_into_box)
#[no_mangle]
pub unsafe extern "C" fn stack_glue_into_box(b: i32, min: i32, max: i32) {
    let q_spec = new_spec(0);
    set_mem_b32s1(q_spec + 1, min);
    set_mem_b32s1(q_spec + 2, max - min);
    let p = new_glue(q_spec);
    if mem_b16s1(b) == HLIST_NODE {
        let mut q = mem_b32s1(b + 5);
        if q == TEX_NULL {
            set_mem_b32s1(b + 5, p);
        } else {
            while mem_b32s1(q) != TEX_NULL {
                q = mem_b32s1(q);
            }
            set_mem_b32s1(q, p);
        }
    } else {
        set_mem_b32s1(p, mem_b32s1(b + 5));
        set_mem_b32s1(b + 5, p);
        set_mem_b32s1(b + 3, mem_b32s1(p + 3));
        set_mem_b32s1(b + 1, mem_b32s1(p + 1));
    }
}

/// translated from xetex-math.c:199 (compute_ot_math_accent_pos)
#[no_mangle]
pub unsafe extern "C" fn compute_ot_math_accent_pos(p: i32) -> i32 {
    if mem_b32s1(p + 1) == MATH_CHAR {
        fetch(p + 1);
        let q = crate::scan::new_native_character(cur_f(), cur_c() as u32);
        let g = real_get_native_glyph(mem_node_addr(q), 0);
        get_ot_math_accent_pos(cur_f(), g as i32)
    } else if mem_b32s1(p + 1) == SUB_MLIST {
        let r = mem_b32s0(p + 1);
        if r != TEX_NULL && mem_b16s1(r) == ACCENT_NOAD {
            compute_ot_math_accent_pos(r)
        } else {
            TEX_INFINITY
        }
    } else {
        TEX_INFINITY
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
#[no_mangle]
pub unsafe extern "C" fn start_eq_no() {
    set_save_stack_b32s1(save_ptr(), cur_chr());
    set_save_ptr(save_ptr() + 1);

    crate::scan::push_math(MATH_SHIFT_GROUP);
    crate::save::eq_word_define(INT_BASE + INT_PAR_CUR_FAM, -1);
    if insert_src_special_every_math() {
        crate::scan::insert_src_special();
    }
    let every_math = eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_MATH);
    if every_math != TEX_NULL {
        begin_token_list(every_math, EVERY_MATH_TEXT);
    }
}

/// translated from xetex-math.c:368 (math_limit_switch)
#[no_mangle]
pub unsafe extern "C" fn math_limit_switch() {
    if cur_list_head() != cur_list_tail() && mem_b16s1(cur_list_tail()) == OP_NOAD {
        set_mem_b16s0(cur_list_tail(), cur_chr() as u16);
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
#[no_mangle]
pub unsafe extern "C" fn math_radical() {
    set_mem_b32s1(cur_list_tail(), get_node(RADICAL_NOAD_SIZE));
    set_cur_list_tail(mem_b32s1(cur_list_tail()));
    set_mem_b16s1(cur_list_tail(), RADICAL_NOAD);
    set_mem_b16s0(cur_list_tail(), NORMAL);
    set_empty_math_field(cur_list_tail() + 1);
    set_empty_math_field(cur_list_tail() + 3);
    set_empty_math_field(cur_list_tail() + 2);
    scan_delimiter(cur_list_tail() + 4, true);
    crate::scan::scan_math(cur_list_tail() + 1);
}

/// translated from xetex-math.c:533 (append_choices)
#[no_mangle]
pub unsafe extern "C" fn append_choices() {
    set_mem_b32s1(cur_list_tail(), new_choice());
    set_cur_list_tail(mem_b32s1(cur_list_tail()));
    set_save_ptr(save_ptr() + 1);
    set_save_stack_b32s1(save_ptr() - 1, 0);
    crate::scan::push_math(MATH_CHOICE_GROUP);
    crate::scan::scan_left_brace();
}

/// translated from xetex-math.c:545 (fin_mlist)
#[no_mangle]
pub unsafe extern "C" fn fin_mlist(p: i32) -> i32 {
    let q;
    if cur_list_aux_b32s1() != TEX_NULL {
        set_mem_b32s1(cur_list_aux_b32s1() + 3, SUB_MLIST);
        set_mem_b32s0(cur_list_aux_b32s1() + 3, mem_b32s1(cur_list_head()));
        if p == TEX_NULL {
            q = cur_list_aux_b32s1();
        } else {
            q = mem_b32s0(cur_list_aux_b32s1() + 2);
            if mem_b16s1(q) != LEFT_NOAD || cur_list_e_tex_aux() == TEX_NULL {
                crate::errors::confusion(c"right".as_ptr());
            }
            set_mem_b32s0(cur_list_aux_b32s1() + 2, mem_b32s1(cur_list_e_tex_aux()));
            set_mem_b32s1(cur_list_e_tex_aux(), cur_list_aux_b32s1());
            set_mem_b32s1(cur_list_aux_b32s1(), p);
        }
    } else {
        set_mem_b32s1(cur_list_tail(), p);
        q = mem_b32s1(cur_list_head());
    }
    crate::mem::pop_nest();
    q
}

/// translated from xetex-math.c:571 (build_choices)
#[no_mangle]
pub unsafe extern "C" fn build_choices() {
    crate::save::unsave();
    let p = fin_mlist(TEX_NULL);
    match save_stack_b32s1(save_ptr() - 1) {
        0 => set_mem_b32s0(cur_list_tail() + 1, p),
        1 => set_mem_b32s1(cur_list_tail() + 1, p),
        2 => set_mem_b32s0(cur_list_tail() + 2, p),
        3 => {
            set_mem_b32s1(cur_list_tail() + 2, p);
            set_save_ptr(save_ptr() - 1);
            return;
        }
        _ => {}
    }
    set_save_stack_b32s1(save_ptr() - 1, save_stack_b32s1(save_ptr() - 1) + 1);
    crate::scan::push_math(MATH_CHOICE_GROUP);
    crate::scan::scan_left_brace();
}

/// translated from xetex-math.c:599 (sub_sup)
#[no_mangle]
pub unsafe extern "C" fn sub_sup() {
    let mut t = EMPTY;
    let mut p = TEX_NULL;

    if cur_list_tail() != cur_list_head() {
        let node_type = mem_b16s1(cur_list_tail());
        if node_type >= 16 && node_type < LEFT_NOAD {
            p = cur_list_tail() + 2 + cur_cmd() as i32 - 7;
            t = mem_b32s1(p);
        }
    }

    if p == TEX_NULL || t != EMPTY {
        set_mem_b32s1(cur_list_tail(), new_noad());
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        p = cur_list_tail() + 2 + cur_cmd() as i32 - 7;
        if t != EMPTY {
            if cur_cmd() == SUP_MARK {
                crate::output::error_here_with_diagnostic(c"Double superscript".as_ptr());
                crate::output::capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(1);
                set_help_line(0, c"I treat `x^1^2' essentially like `x^1{}^2'.".as_ptr());
            } else {
                crate::output::error_here_with_diagnostic(c"Double subscript".as_ptr());
                crate::output::capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(1);
                set_help_line(0, c"I treat `x_1_2' essentially like `x_1{}_2'.".as_ptr());
            }
            crate::errors::error();
        }
    }
    crate::scan::scan_math(p);
}

/// translated from xetex-math.c:644 (math_fraction)
#[no_mangle]
pub unsafe extern "C" fn math_fraction() {
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
        set_cur_list_aux_b32s1(get_node(FRACTION_NOAD_SIZE));
        set_mem_b16s1(cur_list_aux_b32s1(), FRACTION_NOAD);
        set_mem_b16s0(cur_list_aux_b32s1(), NORMAL);
        set_mem_b32s1(cur_list_aux_b32s1() + 2, SUB_MLIST);
        set_mem_b32s0(cur_list_aux_b32s1() + 2, mem_b32s1(cur_list_head()));
        set_empty_math_field(cur_list_aux_b32s1() + 3);
        set_mem_b16s3(cur_list_aux_b32s1() + 4, 0);
        set_mem_b16s2(cur_list_aux_b32s1() + 4, 0);
        set_mem_b16s1(cur_list_aux_b32s1() + 4, 0);
        set_mem_b16s0(cur_list_aux_b32s1() + 4, 0);
        set_mem_b16s3(cur_list_aux_b32s1() + 5, 0);
        set_mem_b16s2(cur_list_aux_b32s1() + 5, 0);
        set_mem_b16s1(cur_list_aux_b32s1() + 5, 0);
        set_mem_b16s0(cur_list_aux_b32s1() + 5, 0);
        set_mem_b32s1(cur_list_head(), TEX_NULL);

        set_cur_list_tail(cur_list_head());

        if c >= DELIMITED_CODE {
            scan_delimiter(cur_list_aux_b32s1() + 4, false);
            scan_delimiter(cur_list_aux_b32s1() + 5, false);
        }

        match c % DELIMITED_CODE {
            ABOVE_CODE => {
                crate::scan::scan_dimen(false, false, false);
                set_mem_b32s1(cur_list_aux_b32s1() + 1, cur_val());
            }
            OVER_CODE => set_mem_b32s1(cur_list_aux_b32s1() + 1, DEFAULT_CODE),
            ATOP_CODE => set_mem_b32s1(cur_list_aux_b32s1() + 1, 0),
            _ => {}
        }
    }
}

/// translated from xetex-math.c:703 (math_left_right)
#[no_mangle]
pub unsafe extern "C" fn math_left_right() {
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
            crate::scan::off_save();
        }
    } else {
        let p = new_noad();
        set_mem_b16s1(p, t);
        scan_delimiter(p + 1, false);
        if t == 1 {
            set_mem_b16s1(p, RIGHT_NOAD);
            set_mem_b16s0(p, 1);
        }
        let q = if t == LEFT_NOAD {
            p
        } else {
            let q = fin_mlist(p);
            crate::save::unsave();
            q
        };
        if t != RIGHT_NOAD {
            crate::scan::push_math(MATH_LEFT_GROUP);
            set_mem_b32s1(cur_list_head(), q);
            set_cur_list_tail(p);
            set_cur_list_e_tex_aux(p);
        } else {
            set_mem_b32s1(cur_list_tail(), new_noad());
            set_cur_list_tail(mem_b32s1(cur_list_tail()));
            set_mem_b16s1(cur_list_tail(), INNER_NOAD);
            set_mem_b32s1(cur_list_tail() + 1, SUB_MLIST);
            set_mem_b32s0(cur_list_tail() + 1, q);
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
#[no_mangle]
pub unsafe extern "C" fn math_ac() {
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

    set_mem_b32s1(cur_list_tail(), get_node(ACCENT_NOAD_SIZE));
    set_cur_list_tail(mem_b32s1(cur_list_tail()));
    set_mem_b16s1(cur_list_tail(), ACCENT_NOAD);
    set_mem_b16s0(cur_list_tail(), NORMAL);
    set_empty_math_field(cur_list_tail() + 1);
    set_empty_math_field(cur_list_tail() + 3);
    set_empty_math_field(cur_list_tail() + 2);
    set_mem_b32s1(cur_list_tail() + 4, MATH_CHAR);

    if cur_chr() == 1 {
        if crate::scan::scan_keyword(c"fixed".as_ptr()) {
            set_mem_b16s0(cur_list_tail(), FIXED_ACC);
        } else if crate::scan::scan_keyword(c"bottom".as_ptr()) {
            if crate::scan::scan_keyword(c"fixed".as_ptr()) {
                set_mem_b16s0(cur_list_tail(), BOTTOM_ACC + 1);
            } else {
                set_mem_b16s0(cur_list_tail(), BOTTOM_ACC);
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

    set_mem_b16s0(cur_list_tail() + 4, (cur_val() % 65536) as u16);
    let intpar_cur_fam = eqtb_b32s1(INT_BASE + INT_PAR_CUR_FAM);
    if math_class_field(cur_val()) == 7
        && intpar_cur_fam >= 0
        && intpar_cur_fam < NUMBER_MATH_FAMILIES
    {
        set_mem_b16s1(cur_list_tail() + 4, intpar_cur_fam as u16);
    } else {
        set_mem_b16s1(cur_list_tail() + 4, math_fam_field(cur_val()) as u16);
    }
    set_mem_b16s1(
        cur_list_tail() + 4,
        mem_b16s1(cur_list_tail() + 4) + (math_char_field(cur_val()) / 65536) as u16 * 256,
    );
    crate::scan::scan_math(cur_list_tail() + 1);
}

/// translated from xetex-math.c:786 (resume_after_display)
#[no_mangle]
pub unsafe extern "C" fn resume_after_display() {
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
unsafe fn font_is_ot_math(f: i32) -> bool {
    font_area_entry(f) == OTGR_FONT_FLAG && isOpenTypeMathFont(font_layout_engine_ptr(f))
}

#[inline]
unsafe fn sym_font_ok(size: i32) -> bool {
    let f = eqtb_b32s1(MATH_FONT_BASE + 2 + size);
    font_params(f) >= TOTAL_MATHSY_PARAMS || font_is_ot_math(f)
}

#[inline]
unsafe fn ext_font_ok(size: i32) -> bool {
    let f = eqtb_b32s1(MATH_FONT_BASE + 3 + size);
    font_params(f) >= TOTAL_MATHEX_PARAMS || font_is_ot_math(f)
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
            b = crate::scan::copy_node_list(j);
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
            crate::mem::free_node(p_orig, BOX_NODE_SIZE);
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
            r = crate::mem::new_kern(0);
            t = crate::mem::new_kern(0);
        } else {
            r = mem_b32s1(b + 5);
            t = mem_b32s1(r);
        }
        let u = crate::mem::new_math(0, END_M_CODE);
        if mem_b16s1(t) == GLUE_NODE {
            let jj = crate::mem::new_skip_param(GLUE_PAR_RIGHT_SKIP as i16);
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
        let u2 = crate::mem::new_math(0, BEGIN_M_CODE);
        if mem_b16s1(r) == GLUE_NODE {
            let jj = crate::mem::new_skip_param(GLUE_PAR_LEFT_SKIP as i16);
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
                b = crate::scan::hpack(u2, 0, ADDITIONAL);
                set_mem_b32s1(b + 4, s);
            } else {
                set_mem_b32s1(b + 5, u2);
            }
        }
    }
    crate::scan::append_to_vlist(b);
}

/// Emit error for insufficient math fonts, flush the math list, set danger.
unsafe fn check_math_font_params(danger: &mut bool) {
    if !sym_font_ok(TEXT_SIZE) || !sym_font_ok(SCRIPT_SIZE) || !sym_font_ok(SCRIPT_SCRIPT_SIZE) {
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
        flush_math();
        *danger = true;
    } else if !ext_font_ok(TEXT_SIZE)
        || !ext_font_ok(SCRIPT_SIZE)
        || !ext_font_ok(SCRIPT_SCRIPT_SIZE)
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
        flush_math();
        *danger = true;
    }
}

/// translated from xetex-math.c:194 (after_math)
#[no_mangle]
pub unsafe extern "C" fn after_math() {
    let mut danger = false;

    let mut j = if cur_list_mode() == MMODE {
        cur_list_e_tex_aux()
    } else {
        TEX_NULL
    };

    check_math_font_params(&mut danger);

    let mut m = cur_list_mode() as i32;
    let mut l = false;
    let mut p = fin_mlist(TEX_NULL);

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
        a = crate::scan::hpack(mem_b32s1(TEMP_HEAD), 0, ADDITIONAL);
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
        p = fin_mlist(TEX_NULL);
    } else {
        a = TEX_NULL;
    }

    if m < 0 {
        // inline math (or inner/restricted hmode)
        let math_surround = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_MATH_SURROUND);
        let node = crate::mem::new_math(math_surround, BEFORE as i16);
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
        let node2 = crate::mem::new_math(math_surround, AFTER);
        set_mem_b32s1(cur_list_tail(), node2);
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        set_cur_list_aux_b32s0(1000);

        if semantic_pagination_enabled() {
            crate::scan::tt_insert_special(c"tdux:ce math".as_ptr());
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
        let mut b = crate::scan::hpack(p, 0, ADDITIONAL);
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
            q = e + math_quad(TEXT_SIZE as i32);
        }
        if w + q > z {
            if e != 0
                && (w - total_shrink(0) + q <= z
                    || total_shrink(FIL) != 0
                    || total_shrink(FILL) != 0
                    || total_shrink(FILLL) != 0)
            {
                crate::mem::free_node(b, BOX_NODE_SIZE);
                b = crate::scan::hpack(p, z - q, EXACTLY);
            } else {
                e = 0;
                if w > z {
                    crate::mem::free_node(b, BOX_NODE_SIZE);
                    b = crate::scan::hpack(p, z, EXACTLY);
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
            crate::mem::new_penalty(eqtb_b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_PENALTY));
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
            let inf_node = crate::mem::new_penalty(INF_PENALTY);
            set_mem_b32s1(cur_list_tail(), inf_node);
            set_cur_list_tail(mem_b32s1(cur_list_tail()));
        } else {
            let g1_node = crate::mem::new_param_glue(g1);
            set_mem_b32s1(cur_list_tail(), g1_node);
            set_cur_list_tail(mem_b32s1(cur_list_tail()));
        }

        if e != 0 {
            let r = crate::mem::new_kern(z - w - e - d);
            if l {
                set_mem_b32s1(a, r);
                set_mem_b32s1(r, b);
                b = a;
                d = 0;
            } else {
                set_mem_b32s1(b, r);
                set_mem_b32s1(r, a);
            }
            b = crate::scan::hpack(b, 0, ADDITIONAL);
        }

        app_display(j, b, d);

        if a != TEX_NULL && e == 0 && !l {
            let inf_node2 = crate::mem::new_penalty(INF_PENALTY);
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

        let post_pen = crate::mem::new_penalty(eqtb_b32s1(INT_BASE + INT_PAR_POST_DISPLAY_PENALTY));
        set_mem_b32s1(cur_list_tail(), post_pen);
        set_cur_list_tail(mem_b32s1(cur_list_tail()));

        if g2 > 0 {
            let g2_node = crate::mem::new_param_glue(g2);
            set_mem_b32s1(cur_list_tail(), g2_node);
            set_cur_list_tail(mem_b32s1(cur_list_tail()));
        }

        crate::scan::flush_node_list(j);

        if semantic_pagination_enabled() {
            crate::scan::tt_insert_special(c"tdux:ce dmath".as_ptr());
        }

        resume_after_display();
    }
}

/// translated from xetex-math.c:81 (init_math)
#[no_mangle]
pub unsafe extern "C" fn init_math() {
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
                crate::mem::new_kern(0)
            } else {
                crate::mem::new_param_glue(GLUE_PAR_RIGHT_SKIP as i16)
            };

            let p_glue = if eqtb_b32s1(GLUE_BASE + GLUE_PAR_LEFT_SKIP) == 0 {
                crate::mem::new_kern(0)
            } else {
                crate::mem::new_param_glue(GLUE_PAR_LEFT_SKIP as i16)
            };
            set_mem_b32s1(p_glue, j);

            j = crate::mem::new_null_box();
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
                p = crate::mem::new_math(0, BEGIN_L_CODE);
                set_mem_b32s1(TEMP_HEAD, p);
                crate::scan::just_copy(
                    mem_b32s1(just_box() + 5),
                    p,
                    crate::mem::new_math(0, END_L_CODE),
                );
                set_cur_dir(RIGHT_TO_LEFT);
            }

            v += 2 * font_info_b32s1(QUAD_CODE + param_base(eqtb_b32s1(CUR_FONT_LOC)));

            if eqtb_b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
                set_temp_ptr(crate::mem::get_avail());
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
                        let c = crate::scan::effective_char(true, f, mem_b16s0(p));
                        d = font_info_b32s1(
                            width_base_entry(f) + font_info_b16(char_base_entry(f) + c).s3 as i32,
                        );
                        is_found = true;
                        break 'reswitch;
                    }
                    match mem_b16s1(p) {
                        0 | 1 | 2 => {
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
                                    set_temp_ptr(crate::mem::get_avail());
                                    set_mem_b32s0(
                                        temp_ptr(),
                                        L_CODE * (mem_b16s0(p) as i32 / L_CODE) + 3,
                                    );
                                    set_mem_b32s1(temp_ptr(), lr_ptr());
                                    set_lr_ptr(temp_ptr());
                                    if mem_b16s0(p) as i32 / R_CODE != cur_dir() as i32 {
                                        crate::scan::just_reverse(p);
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
                            } else if mem_b16s1(just_box() + 5) == SHRINKING {
                                if mem_b16s0(just_box() + 5) == mem_b16s0(q)
                                    && mem_b32s1(q + 3) != 0
                                {
                                    v = MAX_HALFWORD;
                                }
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
            crate::scan::flush_node_list(mem_b32s1(TEMP_HEAD));
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

        crate::scan::push_math(MATH_SHIFT_GROUP);
        set_cur_list_mode(MMODE);
        crate::save::eq_word_define(INT_BASE + INT_PAR_CUR_FAM, -1);
        crate::save::eq_word_define(DIMEN_BASE + DIMEN_PAR_PRE_DISPLAY_SIZE, w);
        set_cur_list_e_tex_aux(j);
        crate::save::eq_word_define(INT_BASE + INT_PAR_PRE_DISPLAY_DIRECTION, x);
        crate::save::eq_word_define(DIMEN_BASE + DIMEN_PAR_DISPLAY_WIDTH, l);
        crate::save::eq_word_define(DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT, s);

        if semantic_pagination_enabled() {
            crate::scan::tt_insert_special(c"tdux:cs dmath".as_ptr());
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
        crate::scan::push_math(MATH_SHIFT_GROUP);
        crate::save::eq_word_define(INT_BASE + INT_PAR_CUR_FAM, -1);
        if insert_src_special_every_math() {
            crate::scan::insert_src_special();
        }
        if semantic_pagination_enabled() {
            crate::scan::tt_insert_special(c"tdux:cs math".as_ptr());
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
#[no_mangle]
pub unsafe extern "C" fn rebox(b: i32, w: i32) -> i32 {
    let mut b = b;
    if mem_b32s1(b + 1) != w && mem_b32s1(b + 5) != TEX_NULL {
        if mem_b16s1(b) == VLIST_NODE {
            b = crate::scan::hpack(b, 0, ADDITIONAL);
        }
        let mut p = mem_b32s1(b + 5);
        if is_char_node(p) && mem_b32s1(p) == TEX_NULL {
            let f = mem_b16s1(p) as i32;
            let c_raw = mem_b16s0(p);
            let q = font_info_b16(
                char_base_entry(f) + crate::scan::effective_char(true, f, c_raw),
            );
            let v = font_info_b32s1(width_base_entry(f) + q.s3 as i32);
            if v != mem_b32s1(b + 1) {
                set_mem_b32s1(p, new_kern(mem_b32s1(b + 1) - v));
            }
        }
        free_node(b, BOX_NODE_SIZE);
        b = new_glue(12);
        set_mem_b32s1(b, p);
        while mem_b32s1(p) != TEX_NULL {
            p = mem_b32s1(p);
        }
        set_mem_b32s1(p, new_glue(12));
        crate::scan::hpack(b, w, EXACTLY)
    } else {
        set_mem_b32s1(b + 1, w);
        b
    }
}
