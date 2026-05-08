//! Math-mode command helpers translated from `xetex/xetex-math.c`.

use crate::globals::{
    cur_chr, cur_cmd, cur_group, cur_list_aux_b32s1, cur_list_e_tex_aux, cur_list_head,
    cur_list_tail, cur_val, cur_val1, eqtb_b32s1, file_line_error_style_p,
    insert_src_special_every_math, mem_b16s1, mem_b32s0, mem_b32s1, save_ptr, save_stack_b32s1,
    set_cur_list_aux_b32s1, set_cur_list_e_tex_aux, set_cur_list_tail, set_cur_val, set_cur_val1,
    set_help_line, set_help_ptr, set_mem_b16s0, set_mem_b16s1, set_mem_b16s2, set_mem_b16s3,
    set_mem_b32s0, set_mem_b32s1, set_save_ptr, set_save_stack_b32s1,
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
