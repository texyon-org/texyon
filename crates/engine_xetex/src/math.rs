//! Math-mode command helpers translated from `xetex/xetex-math.c`.

use crate::globals::{
    avail, char_base_entry, cur_chr, cur_cmd, cur_dir, cur_group, cur_lang, cur_list_aux_b32s1,
    cur_list_e_tex_aux, cur_list_head, cur_list_mode, cur_list_prev_graf, cur_list_tail, cur_val,
    cur_val1, eqtb_b32s1, file_line_error_style_p, font_info_b16, font_info_b32s1, hi_mem_min,
    insert_src_special_every_math, just_box, lr_problems, lr_ptr, mem_b16s0, mem_b16s1,
    mem_b32s0, mem_b32s1, mem_gr, mem_word, nest_ptr, param_base, save_ptr, save_stack_b32s1,
    semantic_pagination_enabled, set_avail, set_cur_dir, set_cur_lang, set_cur_list_aux_b32s0,
    set_cur_list_aux_b32s1, set_cur_list_e_tex_aux, set_cur_list_mode, set_cur_list_prev_graf,
    set_cur_list_tail, set_cur_val, set_cur_val1, set_help_line, set_help_ptr, set_lr_problems,
    set_lr_ptr, set_mem_b16s0, set_mem_b16s1, set_mem_b16s2, set_mem_b16s3, set_mem_b32s0,
    set_mem_b32s1, set_mem_gr, set_mem_word, set_save_ptr, set_save_stack_b32s1, set_temp_ptr,
    set_xtx_ligature_present, temp_ptr, width_base_entry,
};
use crate::mem::{TEX_NULL, get_node, new_choice, new_noad};

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
const RADICAL_NOAD: u16 = 24;
const FRACTION_NOAD: u16 = 25;
const OP_NOAD: u16 = 17;
const INNER_NOAD: u16 = 23;
const LEFT_NOAD: u16 = 30;
const RIGHT_NOAD: u16 = 31;
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

extern "C" {
    fn begin_token_list(p: i32, t: u16);
}

#[inline]
unsafe fn set_empty_math_field(p: i32) {
    set_mem_b32s1(p, EMPTY);
    set_mem_b32s0(p, TEX_NULL);
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
                            width_base_entry(f)
                                + font_info_b16(char_base_entry(f) + c).s3 as i32,
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
                && ((hang_after >= 0 && prev_graf + 2 > hang_after)
                    || prev_graf + 1 < -hang_after)
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
