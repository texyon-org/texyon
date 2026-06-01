//! The main control loop, its command verbs, and the `\halign`/`\valign`
//! alignment family. Relocated from `scan.rs` (= xetex0.c's main_control
//! group) by workstream 02 (module decomposition).
//!
//! This slice seeds the module with the alignment family
//! (`init_align`/`align_peek`/`fin_align`/`init_span`/`init_row`/`init_col`/
//! `push_alignment`/`pop_alignment`/`fin_col`/`fin_row`/`get_preamble_token`/
//! `align_error`/`no_align_error`/`omit_error`); the `main_control*` dispatch
//! and verbs follow in later slices.

use crate::consts::{
    BOX_BASE, BOX_NODE_SIZE, CS_TOKEN_FLAG, CUR_FONT_LOC, DIMEN_BASE, DISC_NODE, ETEX_PEN_BASE,
    GLUE_BASE, GLUE_NODE, HLIST_NODE, INT_BASE, KERN_NODE, LOCAL_BASE, MATH_NODE, MAX_HALFWORD,
    NULL_FLAG, RULE_NODE, SMALL_NODE_SIZE,
};
use crate::errors::{box_error, error, extra_right_brace, privileged, you_cant};
use crate::expand::expand;
use crate::ext::insert_src_special;
use crate::fonts::new_native_character;
use crate::globals::{
    align_state, cur_chr, cur_cmd, cur_group, cur_list_mode, cur_tok, cur_val, eqtb_b32s1,
    mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1, save_ptr, set_align_state, set_cur_input_index,
    set_cur_tok, set_cur_val, set_eqtb_b32s1, set_help_line, set_help_ptr, set_mem_b16s0,
    set_mem_b32s0, set_mem_b32s1, set_save_ptr, set_save_stack_b32s1,
};
use crate::input::begin_token_list;
use crate::linebreak::line_break;
use crate::math::{build_choices_global, fin_mlist_global, flush_math_global, resume_after_display};
use crate::output::{
    capture_to_diagnostic, char_warning, error_here_with_diagnostic, print_cmd_chr,
};
use crate::packaging::{
    append_to_vlist, copy_node_list_global, flush_node_list_global, hpack, vpackage, vsplit,
};
use crate::pagebuilder::build_page;
use crate::scan::{
    append_glue, build_discretionary, do_assignments, effective_char_global, get_token,
    get_x_or_protected, get_x_token, off_save, scan_dimen, scan_eight_bit_int, scan_glue, scan_int,
    scan_keyword, scan_left_brace, scan_optional_equals, scan_register_num, scan_rule_spec,
    scan_spec,
};
use crate::shipout::ship_out;

// Command/token/code consts that live module-private in scan.rs (duplicated
// here, same values — referenced bare by the moved alignment functions).
const RIGHT_BRACE_TOKEN: i32 = 0x40_0000; // xetex-constants.h:351
const LEFT_BRACE_TOKEN: i32 = 0x200000;
const TAB_TOKEN: i32 = 0x80_0000; // xetex-constants.h:354
const RIGHT_BRACE: u8 = 2;
const MAX_COMMAND: u8 = 102;
const ENDV: u8 = 9;
const GLUE_VAL: i16 = 2;
const ASSIGN_GLUE: u8 = 76;
const SPACER: u8 = 10;
const IGNORE_DEPTH: i32 = -65_536_000; // xetex_format.h:388
const AAT_FONT_FLAG: i32 = 0xFFFF;
const DIMEN_PAR_HANG_INDENT: i32 = 17; // xetex_format.h:139
const DIMEN_PAR_HSIZE: i32 = 3; // xetex_format.h:125
const END_M_CODE: u16 = 3;
const HMODE: i16 = 104; // xetex_format.h:369
const HMODE_I: i16 = 104; // xetex_format.h:369
const HRULE: u8 = 36; // xetex_format.h:274
const INSERTED: u16 = 5;
const INSERT_GROUP: u8 = 11; // xetex-constants.h:163
const INT_PAR_HANG_AFTER: i32 = 41; // xetex_format.h:76
const INT_PAR_LANGUAGE: i32 = 50; // xetex_format.h:85
const INT_PAR_LEFT_HYPHEN_MIN: i32 = 51; // xetex_format.h:86
const INT_PAR_LOOSENESS: i32 = 19; // xetex_format.h:54
const INT_PAR_RIGHT_HYPHEN_MIN: i32 = 52; // xetex_format.h:87
const OTGR_FONT_FLAG: i32 = 0xFFFE;
const PAGE_HEAD: i32 = 4_999_997; // MEM_TOP - 2 (xetex_format.h:30)
const RELAX: u8 = 0;
const SHAPE_REF: u16 = 120; // xetex_format.h:362
const SUB_BOX: i32 = 2; // xetex_format.h:283
const VADJUST: u8 = 38; // xetex_format.h:276
const VMODE: i16 = 1;

// ---- moved verbatim from scan.rs (workstream 02) ----

// translated from xetex-xetex0.c:10390 (align_error)
#[no_mangle]
pub unsafe extern "C" fn align_error() {
    if align_state().abs() > 2 {
        error_here_with_diagnostic(c"Misplaced ".as_ptr());
        print_cmd_chr(cur_cmd() as u16, cur_chr());
        capture_to_diagnostic(std::ptr::null_mut());
        if cur_tok() == TAB_TOKEN + 38 {
            set_help_ptr(6);
            set_help_line(
                5,
                c"I can't figure out why you would want to use a tab mark".as_ptr(),
            );
            set_help_line(
                4,
                c"here. If you just want an ampersand, the remedy is".as_ptr(),
            );
            set_help_line(
                3,
                c"simple: Just type `I\\&' now. But if some right brace".as_ptr(),
            );
            set_help_line(
                2,
                c"up above has ended a previous alignment prematurely,".as_ptr(),
            );
            set_help_line(
                1,
                c"you're probably due for more error messages, and you".as_ptr(),
            );
            set_help_line(
                0,
                c"might try typing `S' now just to see what is salvageable.".as_ptr(),
            );
        } else {
            set_help_ptr(5);
            set_help_line(
                4,
                c"I can't figure out why you would want to use a tab mark".as_ptr(),
            );
            set_help_line(
                3,
                c"or \\cr or \\span just now. If something like a right brace".as_ptr(),
            );
            set_help_line(
                2,
                c"up above has ended a previous alignment prematurely,".as_ptr(),
            );
            set_help_line(
                1,
                c"you're probably due for more error messages, and you".as_ptr(),
            );
            set_help_line(
                0,
                c"might try typing `S' now just to see what is salvageable.".as_ptr(),
            );
        }
        error();
    } else {
        crate::input::back_input();
        if align_state() < 0 {
            error_here_with_diagnostic(c"Missing { inserted".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_align_state(align_state() + 1);
            set_cur_tok(LEFT_BRACE_TOKEN + 123);
        } else {
            error_here_with_diagnostic(c"Missing } inserted".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_align_state(align_state() - 1);
            set_cur_tok(RIGHT_BRACE_TOKEN + 125);
        }
        set_help_ptr(3);
        set_help_line(2, c"I've put in what seems to be necessary to fix".as_ptr());
        set_help_line(1, c"the current column of the current alignment.".as_ptr());
        set_help_line(0, c"Try to go on, since this might almost work.".as_ptr());
        crate::input::ins_error();
    }
}
// translated from xetex-xetex0.c:10443 (no_align_error)
#[no_mangle]
pub unsafe extern "C" fn no_align_error() {
    error_here_with_diagnostic(c"Misplaced ".as_ptr());
    crate::output::print_esc_cstr(c"noalign".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(2);
    set_help_line(
        1,
        c"I expect to see \\noalign only after the \\cr of".as_ptr(),
    );
    set_help_line(
        0,
        c"an alignment. Proceed, and I'll ignore this case.".as_ptr(),
    );
    error();
}
// translated from xetex-xetex0.c:10457 (omit_error)
#[no_mangle]
pub unsafe extern "C" fn omit_error() {
    error_here_with_diagnostic(c"Misplaced ".as_ptr());
    crate::output::print_esc_cstr(c"omit".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(2);
    set_help_line(
        1,
        c"I expect to see \\omit only after tab marks or the \\cr of".as_ptr(),
    );
    set_help_line(
        0,
        c"an alignment. Proceed, and I'll ignore this case.".as_ptr(),
    );
    error();
}
// translated from xetex-xetex0.c:8040 (fin_align)
#[no_mangle]
pub unsafe extern "C" fn fin_align() {
    use crate::globals::{
        cur_group, cur_list_aux_b32s0, cur_list_aux_b32s1, cur_list_e_tex_aux, cur_list_head,
        cur_list_mode, cur_list_mode_line, cur_list_tail, mem_b16s0, mem_b16s1, mem_b32s0,
        mem_b32s1, mem_gr, nest_mode, nest_ptr, save_ptr, save_stack_b32s1, set_cur_list_aux_b32s0,
        set_cur_list_aux_b32s1, set_cur_list_tail, set_eqtb_b32s1, set_mem_b16s0, set_mem_b16s1,
        set_mem_b32s0, set_mem_b32s1, set_mem_gr, set_pack_begin_line, set_save_ptr,
    };
    use crate::mem::TEX_NULL;

    const ALIGN_GROUP: u8 = 6; // xetex-constants.h:158
    const ALIGN_HEAD: i32 = 4_999_991; // MEM_TOP - 8
    const END_SPAN: i32 = 4_999_990; // MEM_TOP - 9
    const HOLD_HEAD: i32 = 4_999_995; // MEM_TOP - 4
    const MMODE: i16 = 207;
    const INNER_VMODE: i16 = -1; // -VMODE
    const VMODE: i16 = 1;
    const VLIST_NODE: u16 = 1; // xetex_format.h:381
    const UNSET_NODE: u16 = 13;
    const SPAN_NODE_SIZE: i32 = 2;
    const ADDITIONAL: i16 = 1;
    const STRETCHING: u16 = 1; // xetex-constants.h:275
    const SHRINKING: u16 = 2; // xetex-constants.h:281
    const DLIST: u16 = 2; // xetex-constants.h:278
    const NORMAL: u16 = 0;
    // GLUE_PAR__above_display_skip = 3, GLUE_PAR__below_display_skip = 4
    const GLUE_PAR_ABOVE_DISPLAY: i16 = 3;
    const GLUE_PAR_BELOW_DISPLAY: i16 = 4;
    const DIMEN_PAR_DISPLAY_INDENT: i32 = 15;
    const DIMEN_PAR_OVERFULL_RULE: i32 = 16;
    const INT_PAR_PRE_DISPLAY_PENALTY: i32 = 11;
    const INT_PAR_POST_DISPLAY_PENALTY: i32 = 12;
    const MATH_SHIFT: u8 = 3; // xetex_format.h:233
    const GLUE_PAR_TAB_SKIP: u16 = 12; // GLUE_PAR__tab_skip + 1

    if cur_group() != ALIGN_GROUP {
        crate::errors::confusion(c"align1".as_ptr());
    }
    crate::save::unsave();
    if cur_group() != ALIGN_GROUP {
        crate::errors::confusion(c"align0".as_ptr());
    }
    crate::save::unsave();

    let o: i32 = if nest_mode(nest_ptr() - 1) == MMODE {
        eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT)
    } else {
        0
    };

    // §830: walk preamble, compute column widths, handle span nodes
    let mut q = mem_b32s1(mem_b32s1(ALIGN_HEAD));
    loop {
        crate::mem::flush_list_global(mem_b32s1(q + 3));
        crate::mem::flush_list_global(mem_b32s1(q + 2));
        let p = mem_b32s1(mem_b32s1(q));
        // §831: if NULL_FLAG width, clear the tab_skip glue leader
        if mem_b32s1(q + 1) == NULL_FLAG {
            set_mem_b32s1(q + 1, 0);
            let r = mem_b32s1(q);
            let s = mem_b32s0(r + 1);
            if s != 0 {
                set_mem_b32s1(0, mem_b32s1(0) + 1); // GLUE_SPEC_ref_count(0)++
                crate::mem::delete_glue_ref_global(s);
                set_mem_b32s0(r + 1, 0);
            }
        }
        // §832: handle span nodes — distribute width across spanned columns
        if mem_b32s0(q) != END_SPAN {
            let t = mem_b32s1(q + 1) + mem_b32s1(mem_b32s0(mem_b32s1(q) + 1) + 1);
            let mut r = mem_b32s0(q);
            let mut s = END_SPAN;
            set_mem_b32s0(s, p);
            let mut n: i32 = 1;
            loop {
                set_mem_b32s1(r + 1, mem_b32s1(r + 1) - t);
                let u = mem_b32s0(r);
                while mem_b32s1(r) > n {
                    s = mem_b32s0(s);
                    n = mem_b32s1(mem_b32s0(s)) + 1;
                }
                if mem_b32s1(r) < n {
                    set_mem_b32s0(r, mem_b32s0(s));
                    set_mem_b32s0(s, r);
                    set_mem_b32s1(r, mem_b32s1(r) - 1);
                    s = r;
                } else {
                    if mem_b32s1(r + 1) > mem_b32s1(mem_b32s0(s) + 1) {
                        set_mem_b32s1(mem_b32s0(s) + 1, mem_b32s1(r + 1));
                    }
                    crate::mem::free_node_global(r, SPAN_NODE_SIZE);
                }
                r = u;
                if r == END_SPAN {
                    break;
                }
            }
        }
        // Clear the preamble node fields (convert to UNSET_NODE)
        set_mem_b16s1(q, UNSET_NODE);
        set_mem_b16s0(q, 0);
        set_mem_b32s1(q + 3, 0);
        set_mem_b32s1(q + 2, 0);
        set_mem_b16s0(q + 5, NORMAL);
        set_mem_b16s1(q + 5, NORMAL);
        set_mem_b32s1(q + 6, 0);
        set_mem_b32s1(q + 4, 0);
        q = p;
        if q == TEX_NULL {
            break;
        }
    }

    // §833: pop the spec from save_stack and pack the whole alignment
    set_save_ptr(save_ptr() - 2);
    set_pack_begin_line(-cur_list_mode_line());
    let p = if cur_list_mode() == INNER_VMODE {
        let rule_save = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE);
        set_eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE, 0);
        let pp = hpack(
            mem_b32s1(ALIGN_HEAD),
            save_stack_b32s1(save_ptr() + 1),
            save_stack_b32s1(save_ptr()) as i16,
        );
        set_eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE, rule_save);
        pp
    } else {
        // vmode: copy widths to depth, pack, restore
        let mut qq = mem_b32s1(mem_b32s1(ALIGN_HEAD));
        loop {
            set_mem_b32s1(qq + 3, mem_b32s1(qq + 1));
            set_mem_b32s1(qq + 1, 0);
            qq = mem_b32s1(mem_b32s1(qq));
            if qq == TEX_NULL {
                break;
            }
        }
        let pp = vpackage(
            mem_b32s1(ALIGN_HEAD),
            save_stack_b32s1(save_ptr() + 1),
            save_stack_b32s1(save_ptr()) as i16,
            MAX_HALFWORD,
        );
        qq = mem_b32s1(mem_b32s1(ALIGN_HEAD));
        loop {
            set_mem_b32s1(qq + 1, mem_b32s1(qq + 3));
            set_mem_b32s1(qq + 3, 0);
            qq = mem_b32s1(mem_b32s1(qq));
            if qq == TEX_NULL {
                break;
            }
        }
        pp
    };
    set_pack_begin_line(0);

    // §836–840: walk the row list and resolve each UNSET_NODE
    q = mem_b32s1(cur_list_head());
    let mut s = cur_list_head();
    while q != TEX_NULL {
        if q < crate::globals::hi_mem_min() {
            // !is_char_node
            if mem_b16s1(q) == UNSET_NODE {
                // §836: set this row/column to its final type and dimensions
                if cur_list_mode() == INNER_VMODE {
                    set_mem_b16s1(q, HLIST_NODE);
                    set_mem_b32s1(q + 1, mem_b32s1(p + 1));
                    if nest_mode(nest_ptr() - 1) == MMODE {
                        set_mem_b16s0(q, DLIST);
                    }
                } else {
                    set_mem_b16s1(q, VLIST_NODE);
                    set_mem_b32s1(q + 3, mem_b32s1(p + 3));
                }
                set_mem_b16s0(q + 5, mem_b16s0(p + 5));
                set_mem_b16s1(q + 5, mem_b16s1(p + 5));
                set_mem_gr(q + 6, mem_gr(p + 6));
                set_mem_b32s1(q + 4, o);

                // Walk the column UNSET_NODEs inside this row
                let mut r = mem_b32s1(mem_b32s1(q + 5));
                let mut ss = mem_b32s1(mem_b32s1(p + 5));
                loop {
                    // §837: for each column unset node r, distribute glue
                    let n = mem_b16s0(r);
                    let t_base = mem_b32s1(ss + 1);
                    let mut t = t_base;
                    let mut u = HOLD_HEAD;
                    set_mem_b16s0(r, 0);
                    // Insert inter-column glue for spanned columns
                    let mut span_count = n;
                    while span_count > 0 {
                        span_count -= 1;
                        ss = mem_b32s1(ss);
                        let v = mem_b32s0(ss + 1);
                        set_mem_b32s1(u, crate::mem::new_glue_global(v));
                        u = mem_b32s1(u);
                        set_mem_b16s0(u, GLUE_PAR_TAB_SKIP);
                        t += mem_b32s1(v + 1);
                        if mem_b16s1(p + 5) == STRETCHING {
                            if mem_b16s1(v) == mem_b16s0(p + 5) {
                                t += crate::scaled_math::tex_round(
                                    mem_gr(p + 6) * mem_b32s1(v + 2) as f64,
                                );
                            }
                        } else if mem_b16s1(p + 5) == SHRINKING && mem_b16s0(v) == mem_b16s0(p + 5)
                        {
                            t -= crate::scaled_math::tex_round(
                                mem_gr(p + 6) * mem_b32s1(v + 3) as f64,
                            );
                        }
                        ss = mem_b32s1(ss);
                        set_mem_b32s1(u, crate::mem::new_null_box_global());
                        u = mem_b32s1(u);
                        t += mem_b32s1(ss + 1);
                        if cur_list_mode() == INNER_VMODE {
                            set_mem_b32s1(u + 1, mem_b32s1(ss + 1));
                        } else {
                            set_mem_b16s1(u, VLIST_NODE);
                            set_mem_b32s1(u + 3, mem_b32s1(ss + 1));
                        }
                    }
                    // §839/840: set glue ratio for r and finalize node type
                    if cur_list_mode() == INNER_VMODE {
                        // §839: hbox column
                        set_mem_b32s1(r + 3, mem_b32s1(q + 3));
                        set_mem_b32s1(r + 2, mem_b32s1(q + 2));
                        if t == mem_b32s1(r + 1) {
                            set_mem_b16s1(r + 5, NORMAL);
                            set_mem_b16s0(r + 5, NORMAL);
                            set_mem_gr(r + 6, 0.0);
                        } else if t > mem_b32s1(r + 1) {
                            set_mem_b16s1(r + 5, STRETCHING);
                            if mem_b32s1(r + 6) == 0 {
                                set_mem_gr(r + 6, 0.0);
                            } else {
                                set_mem_gr(
                                    r + 6,
                                    (t - mem_b32s1(r + 1)) as f64 / mem_b32s1(r + 6) as f64,
                                );
                            }
                        } else {
                            set_mem_b16s0(r + 5, mem_b16s1(r + 5));
                            set_mem_b16s1(r + 5, SHRINKING);
                            if mem_b32s1(r + 4) == 0 {
                                set_mem_gr(r + 6, 0.0);
                            } else if mem_b16s0(r + 5) == NORMAL
                                && mem_b32s1(r + 1) - t > mem_b32s1(r + 4)
                            {
                                set_mem_gr(r + 6, 1.0);
                            } else {
                                set_mem_gr(
                                    r + 6,
                                    (mem_b32s1(r + 1) - t) as f64 / mem_b32s1(r + 4) as f64,
                                );
                            }
                        }
                        set_mem_b32s1(r + 1, t_base);
                        set_mem_b16s1(r, HLIST_NODE);
                    } else {
                        // §840: vbox column
                        set_mem_b32s1(r + 1, mem_b32s1(q + 1));
                        if t == mem_b32s1(r + 3) {
                            set_mem_b16s1(r + 5, NORMAL);
                            set_mem_b16s0(r + 5, NORMAL);
                            set_mem_gr(r + 6, 0.0);
                        } else if t > mem_b32s1(r + 3) {
                            set_mem_b16s1(r + 5, STRETCHING);
                            if mem_b32s1(r + 6) == 0 {
                                set_mem_gr(r + 6, 0.0);
                            } else {
                                set_mem_gr(
                                    r + 6,
                                    (t - mem_b32s1(r + 3)) as f64 / mem_b32s1(r + 6) as f64,
                                );
                            }
                        } else {
                            set_mem_b16s0(r + 5, mem_b16s1(r + 5));
                            set_mem_b16s1(r + 5, SHRINKING);
                            if mem_b32s1(r + 4) == 0 {
                                set_mem_gr(r + 6, 0.0);
                            } else if mem_b16s0(r + 5) == NORMAL
                                && mem_b32s1(r + 3) - t > mem_b32s1(r + 4)
                            {
                                set_mem_gr(r + 6, 1.0);
                            } else {
                                set_mem_gr(
                                    r + 6,
                                    (mem_b32s1(r + 3) - t) as f64 / mem_b32s1(r + 4) as f64,
                                );
                            }
                        }
                        set_mem_b32s1(r + 3, t_base);
                        set_mem_b16s1(r, VLIST_NODE);
                    }
                    set_mem_b32s1(r + 4, 0);
                    // Link any inserted inter-column glue nodes
                    if u != HOLD_HEAD {
                        set_mem_b32s1(u, mem_b32s1(r));
                        set_mem_b32s1(r, mem_b32s1(HOLD_HEAD));
                        r = u;
                    }
                    r = mem_b32s1(mem_b32s1(r));
                    ss = mem_b32s1(mem_b32s1(ss));
                    if r == TEX_NULL {
                        break;
                    }
                }
            } else if mem_b16s1(q) == RULE_NODE {
                // §835: rule node — fill in any NULL_FLAG dimensions from the packed box
                if mem_b32s1(q + 1) == NULL_FLAG {
                    set_mem_b32s1(q + 1, mem_b32s1(p + 1));
                }
                if mem_b32s1(q + 3) == NULL_FLAG {
                    set_mem_b32s1(q + 3, mem_b32s1(p + 3));
                }
                if mem_b32s1(q + 2) == NULL_FLAG {
                    set_mem_b32s1(q + 2, mem_b32s1(p + 2));
                }
                if o != 0 {
                    let r = mem_b32s1(q);
                    set_mem_b32s1(q, TEX_NULL);
                    let qq2 = hpack(q, 0, ADDITIONAL);
                    set_mem_b32s1(qq2 + 4, o);
                    set_mem_b32s1(qq2, r);
                    set_mem_b32s1(s, qq2);
                    q = qq2;
                }
            }
        } // end !is_char_node
        s = q;
        q = mem_b32s1(q);
    }

    flush_node_list_global(p);
    pop_alignment();

    // Save aux and pop nest to get back to the outer list
    let aux_save_s0 = cur_list_aux_b32s0();
    let aux_save_s1 = cur_list_aux_b32s1();
    let pp = mem_b32s1(cur_list_head());
    let qq = cur_list_tail();
    crate::mem::pop_nest();

    if cur_list_mode() == MMODE {
        // §1241: display math alignment — add penalties + display glue
        do_assignments();
        if cur_cmd() != MATH_SHIFT {
            error_here_with_diagnostic(c"Missing $$ inserted".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(2);
            set_help_line(
                1,
                c"Displays can use special alignments (like \\eqalignno)".as_ptr(),
            );
            set_help_line(
                0,
                c"only if nothing but the alignment itself is between $$'s.".as_ptr(),
            );
            crate::input::back_error();
        } else {
            // §1232: check for second $$
            get_x_token();
            if cur_cmd() != MATH_SHIFT {
                error_here_with_diagnostic(c"Display math should end with $$".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
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
        flush_node_list_global(cur_list_e_tex_aux());
        crate::mem::pop_nest();
        // Insert pre-display penalty and above-display skip
        set_mem_b32s1(
            cur_list_tail(),
            crate::mem::new_penalty_global(eqtb_b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_PENALTY)),
        );
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        set_mem_b32s1(
            cur_list_tail(),
            crate::mem::new_param_glue_global(GLUE_PAR_ABOVE_DISPLAY),
        );
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        // Attach alignment rows
        set_mem_b32s1(cur_list_tail(), pp);
        if pp != TEX_NULL {
            set_cur_list_tail(qq);
        }
        // Insert post-display penalty and below-display skip
        set_mem_b32s1(
            cur_list_tail(),
            crate::mem::new_penalty_global(eqtb_b32s1(INT_BASE + INT_PAR_POST_DISPLAY_PENALTY)),
        );
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        set_mem_b32s1(
            cur_list_tail(),
            crate::mem::new_param_glue_global(GLUE_PAR_BELOW_DISPLAY),
        );
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        // Only prevdepth (aux.b32.s1) is restored for MMODE (C: cur_list.aux.b32.s1 = aux_save.b32.s1)
        set_cur_list_aux_b32s1(aux_save_s1);
        resume_after_display();
    } else {
        // Regular vmode or hmode alignment — restore full aux
        set_cur_list_aux_b32s0(aux_save_s0);
        set_cur_list_aux_b32s1(aux_save_s1);
        set_mem_b32s1(cur_list_tail(), pp);
        if pp != TEX_NULL {
            set_cur_list_tail(qq);
        }
        if cur_list_mode() == VMODE {
            build_page();
        }
    }
}
// translated from xetex-xetex0.c:8785 (align_peek)
#[no_mangle]
pub unsafe extern "C" fn align_peek() {
    const NO_ALIGN: u8 = 34; // xetex_format.h:272
    const NO_ALIGN_GROUP: u8 = 7; // xetex-constants.h:159
    const CAR_RET: u8 = 5; // xetex_format.h:235
    const CR_CR_CODE: i32 = 1_114_115; // xetex-constants.h:681

    loop {
        crate::globals::set_align_state(1_000_000);
        loop {
            get_x_or_protected();
            if cur_cmd() != SPACER {
                break;
            }
        }
        if cur_cmd() == NO_ALIGN {
            scan_left_brace();
            crate::save::new_save_level(NO_ALIGN_GROUP);
            if crate::globals::cur_list_mode() == -1 {
                normal_paragraph();
            }
            return;
        } else if cur_cmd() == RIGHT_BRACE {
            fin_align();
            return;
        } else if cur_cmd() == CAR_RET && cur_chr() == CR_CR_CODE {
            continue;
        } else {
            init_row();
            init_col();
            return;
        }
    }
}
// translated from xetex-xetex0.c:7728 (get_preamble_token)
#[no_mangle]
pub unsafe extern "C" fn get_preamble_token() {
    use crate::globals::{cur_chr, cur_cmd};
    const TAB_MARK: u8 = 4; // xetex_format.h:234
    const SPAN_CODE_GPT: i32 = 1_114_113; // xetex_format.h:679
    const GLUE_REF_GPT: u16 = 119; // xetex_format.h:361
    const INT_PAR_GLOBAL_DEFS: i32 = 43; // xetex_format.h:78
    const GLUE_BASE_TAB_SKIP: i32 = GLUE_BASE + 11;

    'restart: loop {
        get_token();
        while cur_chr() == SPAN_CODE_GPT && cur_cmd() == TAB_MARK {
            get_token();
            if cur_cmd() > MAX_COMMAND {
                expand();
                get_token();
            }
        }
        if cur_cmd() == ENDV {
            crate::errors::fatal_error(
                c"(interwoven alignment preambles are not allowed)".as_ptr(),
            );
        }
        if cur_cmd() == ASSIGN_GLUE && cur_chr() == GLUE_BASE_TAB_SKIP {
            scan_optional_equals();
            scan_glue(GLUE_VAL);
            if eqtb_b32s1(INT_BASE + INT_PAR_GLOBAL_DEFS) > 0 {
                crate::save::geq_define(GLUE_BASE_TAB_SKIP, GLUE_REF_GPT, cur_val());
            } else {
                crate::save::eq_define(GLUE_BASE_TAB_SKIP, GLUE_REF_GPT, cur_val());
            }
            continue 'restart;
        }
        break;
    }
}
// translated from xetex-xetex0.c:7880 (init_span)
#[no_mangle]
pub unsafe extern "C" fn init_span(p: i32) {
    use crate::globals::{
        cur_list_mode, set_cur_list_aux_b32s0, set_cur_list_aux_b32s1, set_cur_span,
    };
    const INNER_HMODE: i16 = -104;

    crate::mem::push_nest();
    if cur_list_mode() == INNER_HMODE {
        set_cur_list_aux_b32s0(1000);
    } else {
        set_cur_list_aux_b32s1(IGNORE_DEPTH);
        normal_paragraph();
    }
    set_cur_span(p);
}
// translated from xetex-xetex0.c:7893 (init_row)
#[no_mangle]
pub unsafe extern "C" fn init_row() {
    use crate::globals::{
        cur_head, cur_list_mode, cur_list_tail, cur_pre_head, set_cur_align,
        set_cur_list_aux_b32s0, set_cur_list_aux_b32s1, set_cur_list_mode, set_cur_list_tail,
        set_cur_pre_tail, set_cur_tail_a, set_mem_b16s0, set_mem_b32s1,
    };
    const ALIGN_HEAD_IR: i32 = 4_999_991;
    const GLUE_PAR_TAB_SKIP: u16 = 11 + 1; // GLUE_PAR__tab_skip + 1

    crate::mem::push_nest();
    let new_mode = -105 - cur_list_mode() as i32;
    set_cur_list_mode(new_mode as i16);
    if new_mode == -104 {
        set_cur_list_aux_b32s0(0);
    } else {
        set_cur_list_aux_b32s1(0);
    }
    let align_head_link = mem_b32s1(ALIGN_HEAD_IR);
    let glue_node = crate::mem::new_glue_global(mem_b32s0(align_head_link + 1));
    set_mem_b32s1(cur_list_tail(), glue_node);
    set_cur_list_tail(mem_b32s1(cur_list_tail()));
    set_mem_b16s0(cur_list_tail(), GLUE_PAR_TAB_SKIP);
    set_cur_align(mem_b32s1(align_head_link));
    set_cur_tail_a(cur_head());
    set_cur_pre_tail(cur_pre_head());
    init_span(crate::globals::cur_align());
}
// translated from xetex-xetex0.c:7912 (init_col)
#[no_mangle]
pub unsafe extern "C" fn init_col() {
    use crate::globals::{cur_align, cur_cmd, set_align_state, set_mem_b32s0};
    const OMIT_CMD: u8 = 63; // xetex_format.h:301
    const U_TEMPLATE: u16 = 1; // xetex-constants.h:200

    set_mem_b32s0(cur_align() + 5, cur_cmd() as i32);
    if cur_cmd() == OMIT_CMD {
        set_align_state(0);
    } else {
        crate::input::back_input();
        begin_token_list(mem_b32s1(cur_align() + 3), U_TEMPLATE);
    }
}
// translated from xetex-xetex0.c:7724 (push_alignment)
#[no_mangle]
pub unsafe extern "C" fn push_alignment() {
    use crate::globals::{
        align_ptr, align_state, cur_align, cur_head, cur_loop, cur_pre_head, cur_pre_tail,
        cur_span, cur_tail_a, set_align_ptr, set_cur_head, set_cur_pre_head, set_mem_b32s0,
        set_mem_b32s1,
    };
    const ALIGN_HEAD: i32 = 4_999_991; // MEM_TOP - 8
    const ALIGN_STACK_NODE_SIZE: i32 = 6; // xetex-constants.h:129

    let p = crate::mem::get_node_global(ALIGN_STACK_NODE_SIZE);
    set_mem_b32s1(p, align_ptr());
    set_mem_b32s0(p, cur_align());
    set_mem_b32s0(p + 1, mem_b32s1(ALIGN_HEAD));
    set_mem_b32s1(p + 1, cur_span());
    set_mem_b32s1(p + 2, cur_loop());
    set_mem_b32s1(p + 3, align_state());
    set_mem_b32s0(p + 4, cur_head());
    set_mem_b32s1(p + 4, cur_tail_a());
    set_mem_b32s0(p + 5, cur_pre_head());
    set_mem_b32s1(p + 5, cur_pre_tail());
    set_align_ptr(p);
    set_cur_head(crate::mem::get_avail_global());
    set_cur_pre_head(crate::mem::get_avail_global());
}
// translated from xetex-xetex0.c:7743 (pop_alignment)
#[no_mangle]
pub unsafe extern "C" fn pop_alignment() {
    use crate::globals::{
        align_ptr, avail, cur_head, cur_pre_head, mem_b32s0, set_align_ptr, set_align_state,
        set_cur_align, set_cur_head, set_cur_loop, set_cur_pre_head, set_cur_pre_tail,
        set_cur_span, set_cur_tail_a, set_mem_b32s1,
    };
    const ALIGN_HEAD: i32 = 4_999_991;
    const ALIGN_STACK_NODE_SIZE: i32 = 6;

    // free_avail(cur_head): mem[cur_head].b32.s1 = avail; avail = cur_head
    crate::globals::set_mem_b32s1(cur_head(), avail());
    crate::globals::set_avail(cur_head());
    // free_avail(cur_pre_head)
    crate::globals::set_mem_b32s1(cur_pre_head(), avail());
    crate::globals::set_avail(cur_pre_head());

    let p = align_ptr();
    set_cur_tail_a(mem_b32s1(p + 4));
    set_cur_head(mem_b32s0(p + 4));
    set_cur_pre_tail(mem_b32s1(p + 5));
    set_cur_pre_head(mem_b32s0(p + 5));
    set_align_state(mem_b32s1(p + 3));
    set_cur_loop(mem_b32s1(p + 2));
    set_cur_span(mem_b32s1(p + 1));
    set_mem_b32s1(ALIGN_HEAD, mem_b32s0(p + 1));
    set_cur_align(mem_b32s0(p));
    set_align_ptr(mem_b32s1(p));
    crate::mem::free_node_global(p, ALIGN_STACK_NODE_SIZE);
}
// translated from xetex-xetex0.c:7859 (fin_col)
#[no_mangle]
pub unsafe extern "C" fn fin_col() -> bool {
    use crate::globals::{
        adjust_tail, cur_align, cur_list_head, cur_list_mode, cur_list_tail, cur_loop,
        cur_pre_tail, cur_span, cur_tail_a, mem_b32s0, mem_b32s1, pre_adjust_tail, set_adjust_tail,
        set_cur_align, set_cur_list_tail, set_cur_loop, set_cur_pre_tail, set_cur_tail_a,
        set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1, set_pre_adjust_tail,
        total_shrink, total_stretch,
    };
    use crate::mem::TEX_NULL;

    const ADDITIONAL: i16 = 1;
    const SPAN_CODE: i32 = 1_114_113; // xetex_format.h:679
    const CR_CODE: i32 = 1_114_114; // xetex_format.h:680
    const SPAN_NODE_SIZE: i32 = 2; // xetex-constants.h:111
    const ALIGN_GROUP: u8 = 6; // xetex-constants.h:158
    const HOLD_HEAD: i32 = 4_999_995; // MEM_TOP - 4
    const END_SPAN: i32 = 4_999_990; // MEM_TOP - 9
    const GLUE_PAR_TAB_SKIP: u16 = 12; // GLUE_PAR__tab_skip + 1
    const INNER_HMODE: i16 = -104;
    const FIL: usize = 1;
    const FILL: usize = 2;
    const FILLL: usize = 3;
    const UNSET_NODE: u16 = 13; // xetex_format.h

    if cur_align() == TEX_NULL {
        crate::errors::confusion(c"endv".as_ptr());
    }
    let q = mem_b32s1(cur_align());
    if q == TEX_NULL {
        crate::errors::confusion(c"endv".as_ptr());
    }
    if align_state() < 500_000 {
        crate::errors::fatal_error(c"(interwoven alignment preambles are not allowed)".as_ptr());
    }
    let mut p = mem_b32s1(q);
    if p == TEX_NULL && mem_b32s0(cur_align() + 5) < CR_CODE {
        if cur_loop() != TEX_NULL {
            // §822: extend preamble by duplicating the loop template
            set_mem_b32s1(q, crate::mem::new_null_box_global());
            p = mem_b32s1(q);
            set_mem_b32s0(p, END_SPAN);
            set_mem_b32s1(p + 1, NULL_FLAG);
            set_cur_loop(mem_b32s1(cur_loop()));
            // copy u-template from cur_loop+3 into HOLD_HEAD list → p+3
            let mut wq = HOLD_HEAD;
            let mut r = mem_b32s1(cur_loop() + 3);
            while r != TEX_NULL {
                set_mem_b32s1(wq, crate::mem::get_avail_global());
                wq = mem_b32s1(wq);
                set_mem_b32s0(wq, mem_b32s0(r));
                r = mem_b32s1(r);
            }
            set_mem_b32s1(wq, TEX_NULL);
            set_mem_b32s1(p + 3, mem_b32s1(HOLD_HEAD));
            // copy v-template from cur_loop+2 into HOLD_HEAD list → p+2
            wq = HOLD_HEAD;
            r = mem_b32s1(cur_loop() + 2);
            while r != TEX_NULL {
                set_mem_b32s1(wq, crate::mem::get_avail_global());
                wq = mem_b32s1(wq);
                set_mem_b32s0(wq, mem_b32s0(r));
                r = mem_b32s1(r);
            }
            set_mem_b32s1(wq, TEX_NULL);
            set_mem_b32s1(p + 2, mem_b32s1(HOLD_HEAD));
            set_cur_loop(mem_b32s1(cur_loop()));
            set_mem_b32s1(p, crate::mem::new_glue_global(mem_b32s0(cur_loop() + 1)));
            set_mem_b16s0(mem_b32s1(p), GLUE_PAR_TAB_SKIP);
        } else {
            error_here_with_diagnostic(c"Extra alignment tab has been changed to ".as_ptr());
            crate::output::print_esc_cstr(c"cr".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(3);
            set_help_line(
                2,
                c"You have given more \\span or & marks than there were".as_ptr(),
            );
            set_help_line(
                1,
                c"in the preamble to the \\halign or \\valign now in progress.".as_ptr(),
            );
            set_help_line(
                0,
                c"So I'll assume that you meant to type \\cr instead.".as_ptr(),
            );
            set_mem_b32s0(cur_align() + 5, CR_CODE);
            error();
        }
    }
    if mem_b32s0(cur_align() + 5) != SPAN_CODE {
        crate::save::unsave();
        crate::save::new_save_level(ALIGN_GROUP);
        let (u, w) = if cur_list_mode() == INNER_HMODE {
            set_adjust_tail(cur_tail_a());
            set_pre_adjust_tail(cur_pre_tail());
            let u = hpack(mem_b32s1(cur_list_head()), 0, ADDITIONAL);
            let w = mem_b32s1(u + 1);
            set_cur_tail_a(adjust_tail());
            set_adjust_tail(TEX_NULL);
            set_cur_pre_tail(pre_adjust_tail());
            set_pre_adjust_tail(TEX_NULL);
            (u, w)
        } else {
            let u = vpackage(mem_b32s1(cur_list_head()), 0, ADDITIONAL, 0);
            let w = mem_b32s1(u + 3);
            (u, w)
        };
        let mut n: i32 = 0;
        if cur_span() != cur_align() {
            // §827: compare accumulated span width with recorded widths
            let mut sq = cur_span();
            loop {
                n += 1;
                sq = mem_b32s1(mem_b32s1(sq));
                if sq == cur_align() {
                    break;
                }
            }
            if n > u16::MAX as i32 {
                crate::errors::confusion(c"too many spans".as_ptr());
            }
            sq = cur_span();
            while mem_b32s1(mem_b32s0(sq)) < n {
                sq = mem_b32s0(sq);
            }
            if mem_b32s1(mem_b32s0(sq)) > n {
                let s = crate::mem::get_node_global(SPAN_NODE_SIZE);
                set_mem_b32s0(s, mem_b32s0(sq));
                set_mem_b32s1(s, n);
                set_mem_b32s0(sq, s);
                set_mem_b32s1(s + 1, w);
            } else if mem_b32s1(mem_b32s0(sq) + 1) < w {
                set_mem_b32s1(mem_b32s0(sq) + 1, w);
            }
        } else if w > mem_b32s1(cur_align() + 1) {
            set_mem_b32s1(cur_align() + 1, w);
        }
        set_mem_b16s1(u, UNSET_NODE);
        set_mem_b16s0(u, n as u16);
        let o_str: usize = if total_stretch(FILLL) != 0 {
            FILLL
        } else if total_stretch(FILL) != 0 {
            FILL
        } else if total_stretch(FIL) != 0 {
            FIL
        } else {
            0
        };
        set_mem_b16s0(u + 5, o_str as u16);
        set_mem_b32s1(u + 6, total_stretch(o_str));
        let o_shr: usize = if total_shrink(FILLL) != 0 {
            FILLL
        } else if total_shrink(FILL) != 0 {
            FILL
        } else if total_shrink(FIL) != 0 {
            FIL
        } else {
            0
        };
        set_mem_b16s1(u + 5, o_shr as u16);
        set_mem_b32s1(u + 4, total_shrink(o_shr));
        crate::mem::pop_nest();
        set_mem_b32s1(cur_list_tail(), u);
        set_cur_list_tail(u);
        set_mem_b32s1(
            cur_list_tail(),
            crate::mem::new_glue_global(mem_b32s0(mem_b32s1(cur_align()) + 1)),
        );
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        set_mem_b16s0(cur_list_tail(), GLUE_PAR_TAB_SKIP);
        if mem_b32s0(cur_align() + 5) >= CR_CODE {
            return true;
        }
        init_span(p);
    }
    set_align_state(1_000_000);
    loop {
        get_x_or_protected();
        if cur_cmd() != SPACER {
            break;
        }
    }
    set_cur_align(p);
    init_col();
    false
}
// translated from xetex-xetex0.c:8009 (fin_row)
#[no_mangle]
pub unsafe extern "C" fn fin_row() {
    use crate::globals::{
        cur_head, cur_list_head, cur_list_mode, cur_list_tail, cur_pre_head, cur_pre_tail,
        cur_tail_a, mem_b32s1, set_cur_list_aux_b32s0, set_cur_list_tail, set_mem_b16s1,
        set_mem_b32s1,
    };
    use crate::mem::TEX_NULL;

    const ADDITIONAL: i16 = 1;
    const UNSET_NODE: u16 = 13;
    const INNER_HMODE: i16 = -104;
    const EVERY_CR_TEXT: u16 = 14; // xetex-constants.h:213
    const LOCAL__EVERY_CR: i32 = 8; // xetex_format.h:180

    let p = if cur_list_mode() == INNER_HMODE {
        let u = hpack(mem_b32s1(cur_list_head()), 0, ADDITIONAL);
        crate::mem::pop_nest();
        if cur_pre_head() != cur_pre_tail() {
            set_mem_b32s1(cur_list_tail(), mem_b32s1(cur_pre_head()));
            set_cur_list_tail(cur_pre_tail());
        }
        append_to_vlist(u);
        if cur_head() != cur_tail_a() {
            set_mem_b32s1(cur_list_tail(), mem_b32s1(cur_head()));
            set_cur_list_tail(cur_tail_a());
        }
        u
    } else {
        let u = vpackage(mem_b32s1(cur_list_head()), 0, ADDITIONAL, MAX_HALFWORD);
        crate::mem::pop_nest();
        set_mem_b32s1(cur_list_tail(), u);
        set_cur_list_tail(u);
        set_cur_list_aux_b32s0(1000);
        u
    };
    set_mem_b16s1(p, UNSET_NODE);
    set_mem_b32s1(p + 6, 0);
    let every_cr = eqtb_b32s1(LOCAL_BASE + LOCAL__EVERY_CR);
    if every_cr != TEX_NULL {
        begin_token_list(every_cr, EVERY_CR_TEXT);
    }
    align_peek();
}
// translated from xetex-xetex0.c:7732 (init_align)
#[no_mangle]
pub unsafe extern "C" fn init_align() {
    use crate::globals::{
        cur_align, cur_cs, cur_list_aux_b32s1, cur_list_head, cur_list_mode, cur_list_tail,
        cur_loop, mem_b32s1, set_align_state, set_cur_align, set_cur_list_aux_b32s1,
        set_cur_list_mode, set_cur_loop, set_mem_b32s0, set_mem_b32s1, set_scanner_status,
        set_warning_index,
    };
    use crate::mem::TEX_NULL;

    const ALIGN_HEAD: i32 = 4_999_991; // MEM_TOP - 8
    const END_SPAN: i32 = 4_999_990; // MEM_TOP - 9
    const HOLD_HEAD: i32 = 4_999_995; // MEM_TOP - 4
    const FROZEN_END_TEMPLATE: i32 = 2_243_226 + 5; // FROZEN_CONTROL_SEQUENCE + 5
    const ALIGN_GROUP: u8 = 6;
    const EVERY_CR_TEXT: u16 = 14;
    const LOCAL__EVERY_CR: i32 = 8;
    const GLUE_PAR_TAB_SKIP: i16 = 11; // GLUE_PAR__tab_skip
    const MMODE: i16 = 207;
    const ALIGNING: u8 = 4;
    const NORMAL_SCANNER: u8 = 0;
    const TAB_MARK: u8 = 4;
    const CAR_RET: u8 = 5;
    const MAC_PARAM: u8 = 6;
    const SPACER: u8 = 10;

    let save_cs_ptr = cur_cs();
    push_alignment();
    set_align_state(-1_000_000);

    if cur_list_mode() == MMODE
        && (cur_list_tail() != cur_list_head() || cur_list_aux_b32s1() != TEX_NULL)
    {
        error_here_with_diagnostic(c"Improper ".as_ptr());
        crate::output::print_esc_cstr(c"halign".as_ptr());
        crate::output::print_cstr(c" inside $$'s".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(3);
        set_help_line(
            2,
            c"Displays can use special alignments (like \\eqalignno)".as_ptr(),
        );
        set_help_line(
            1,
            c"only if nothing but the alignment itself is between $$'s.".as_ptr(),
        );
        set_help_line(
            0,
            c"So I've deleted the formulas that preceded this alignment.".as_ptr(),
        );
        error();
        flush_math_global();
    }

    crate::mem::push_nest();

    if cur_list_mode() == MMODE {
        set_cur_list_mode(-1);
        let aux = crate::globals::nest_aux_b32s1(crate::globals::nest_ptr() - 2);
        set_cur_list_aux_b32s1(aux);
    } else if cur_list_mode() > 0 {
        set_cur_list_mode(-(cur_list_mode() as i32) as i16);
    }

    scan_spec(ALIGN_GROUP, false);
    set_mem_b32s1(ALIGN_HEAD, TEX_NULL);
    set_cur_align(ALIGN_HEAD);
    set_cur_loop(TEX_NULL);
    set_scanner_status(ALIGNING);
    set_warning_index(save_cs_ptr);
    set_align_state(-1_000_000);

    'outer: loop {
        set_mem_b32s1(
            cur_align(),
            crate::mem::new_param_glue_global(GLUE_PAR_TAB_SKIP),
        );
        set_cur_align(mem_b32s1(cur_align()));
        if cur_cmd() == CAR_RET {
            break 'outer;
        }

        let mut p = HOLD_HEAD;
        set_mem_b32s1(p, TEX_NULL);

        // scan u-template up to first #
        loop {
            get_preamble_token();
            if cur_cmd() == MAC_PARAM {
                break; // done1
            }
            if cur_cmd() <= CAR_RET && cur_cmd() >= TAB_MARK && align_state() == -1_000_000 {
                if p == HOLD_HEAD && cur_loop() == TEX_NULL && cur_cmd() == TAB_MARK {
                    set_cur_loop(cur_align());
                } else {
                    error_here_with_diagnostic(
                        c"Missing # inserted in alignment preamble".as_ptr(),
                    );
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(3);
                    set_help_line(
                        2,
                        c"There should be exactly one # between &'s, when an".as_ptr(),
                    );
                    set_help_line(
                        1,
                        c"\\halign or \\valign is being set up. In this case you had".as_ptr(),
                    );
                    set_help_line(
                        0,
                        c"none, so I've put one in; maybe that will work.".as_ptr(),
                    );
                    crate::input::back_error();
                    break; // done1
                }
            } else if cur_cmd() != SPACER || p != HOLD_HEAD {
                set_mem_b32s1(p, crate::mem::get_avail_global());
                p = mem_b32s1(p);
                set_mem_b32s0(p, cur_tok());
            }
        }

        // done1: set up the column preamble node
        set_mem_b32s1(cur_align(), crate::mem::new_null_box_global());
        set_cur_align(mem_b32s1(cur_align()));
        set_mem_b32s0(cur_align(), END_SPAN);
        set_mem_b32s1(cur_align() + 1, NULL_FLAG);
        set_mem_b32s1(cur_align() + 3, mem_b32s1(HOLD_HEAD));
        p = HOLD_HEAD;
        set_mem_b32s1(p, TEX_NULL);

        // scan v-template after #
        loop {
            get_preamble_token();
            if cur_cmd() <= CAR_RET && cur_cmd() >= TAB_MARK && align_state() == -1_000_000 {
                break; // done2
            }
            if cur_cmd() == MAC_PARAM {
                error_here_with_diagnostic(c"Only one # is allowed per tab".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(3);
                set_help_line(
                    2,
                    c"There should be exactly one # between &'s, when an".as_ptr(),
                );
                set_help_line(
                    1,
                    c"\\halign or \\valign is being set up. In this case you had".as_ptr(),
                );
                set_help_line(
                    0,
                    c"more than one, so I'm ignoring all but the first.".as_ptr(),
                );
                error();
                continue; // goto continue_: restart this inner loop
            }
            set_mem_b32s1(p, crate::mem::get_avail_global());
            p = mem_b32s1(p);
            set_mem_b32s0(p, cur_tok());
        }

        // done2: append FROZEN_END_TEMPLATE and store v-template
        set_mem_b32s1(p, crate::mem::get_avail_global());
        p = mem_b32s1(p);
        set_mem_b32s0(p, CS_TOKEN_FLAG + FROZEN_END_TEMPLATE);
        set_mem_b32s1(cur_align() + 2, mem_b32s1(HOLD_HEAD));
    }

    // done:
    set_scanner_status(NORMAL_SCANNER);
    crate::save::new_save_level(ALIGN_GROUP);
    let every_cr = eqtb_b32s1(LOCAL_BASE + LOCAL__EVERY_CR);
    if every_cr != TEX_NULL {
        begin_token_list(every_cr, EVERY_CR_TEXT);
    }
    align_peek();
}

// ---- moved verbatim from scan.rs (workstream 02) ----

// translated from xetex-xetex0.c:10416 (push_math)
// group_code = unsigned char = u8 (xetex-xetexd.h:325)
#[no_mangle]
pub unsafe extern "C" fn push_math(c: u8) {
    crate::mem::push_nest();
    crate::globals::set_cur_list_mode(-207); // -MMODE (xetex_format.h:370)
    crate::globals::set_cur_list_aux_b32s1(crate::mem::TEX_NULL);
    crate::save::new_save_level(c);
}
// translated from xetex-xetex0.c:9942 (end_graf)
#[no_mangle]
pub unsafe extern "C" fn end_graf() {
    if cur_list_mode() == HMODE {
        if crate::globals::cur_list_head() == crate::globals::cur_list_tail() {
            crate::mem::pop_nest();
        } else {
            line_break(false);
        }
        if crate::globals::cur_list_e_tex_aux() != crate::mem::TEX_NULL {
            crate::mem::flush_list_global(crate::globals::cur_list_e_tex_aux());
            crate::globals::set_cur_list_e_tex_aux(crate::mem::TEX_NULL);
        }
        normal_paragraph();
        crate::globals::set_error_count(0);
    }
}
// translated from xetex-xetex0.c:10016 (append_penalty)
#[no_mangle]
pub unsafe extern "C" fn append_penalty() {
    scan_int();
    let p = crate::mem::new_penalty_global(crate::globals::cur_val());
    let tail = crate::globals::cur_list_tail();
    set_mem_b32s1(tail, p);
    crate::globals::set_cur_list_tail(p);
    if cur_list_mode() == VMODE {
        build_page();
    }
}
// translated from xetex-xetex0.c:9496 (normal_paragraph)
pub unsafe fn normal_paragraph() {
    if eqtb_b32s1(INT_BASE + INT_PAR_LOOSENESS) != 0 {
        crate::save::eq_word_define(INT_BASE + INT_PAR_LOOSENESS, 0);
    }
    if eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_HANG_INDENT) != 0 {
        crate::save::eq_word_define(DIMEN_BASE + DIMEN_PAR_HANG_INDENT, 0);
    }
    if eqtb_b32s1(INT_BASE + INT_PAR_HANG_AFTER) != 1 {
        crate::save::eq_word_define(INT_BASE + INT_PAR_HANG_AFTER, 1);
    }
    if eqtb_b32s1(LOCAL_BASE) != crate::mem::TEX_NULL {
        crate::save::eq_define(LOCAL_BASE, SHAPE_REF, crate::mem::TEX_NULL);
    }
    if eqtb_b32s1(ETEX_PEN_BASE) != crate::mem::TEX_NULL {
        crate::save::eq_define(ETEX_PEN_BASE, SHAPE_REF, crate::mem::TEX_NULL);
    }
}
// translated from xetex-xetex0.c:9332 (its_all_over)
#[no_mangle]
pub unsafe extern "C" fn its_all_over() -> bool {
    if privileged() {
        if crate::globals::page_tail() == PAGE_HEAD
            && crate::globals::cur_list_head() == crate::globals::cur_list_tail()
            && crate::globals::dead_cycles() == 0
        {
            return true;
        }
        crate::input::back_input();
        let nb = crate::mem::new_null_box_global();
        set_mem_b32s1(crate::globals::cur_list_tail(), nb);
        crate::globals::set_cur_list_tail(nb);
        set_mem_b32s1(
            crate::globals::cur_list_tail() + 1,
            eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_HSIZE),
        );
        let ng = crate::mem::new_glue_global(8);
        set_mem_b32s1(crate::globals::cur_list_tail(), ng);
        crate::globals::set_cur_list_tail(ng);
        let np = crate::mem::new_penalty_global(NULL_FLAG);
        set_mem_b32s1(crate::globals::cur_list_tail(), np);
        crate::globals::set_cur_list_tail(np);
        build_page();
    }
    false
}
// translated from xetex-xetex0.c:9808 (indent_in_hmode)
#[no_mangle]
pub unsafe extern "C" fn indent_in_hmode() {
    if cur_chr() > 0 {
        let p = crate::mem::new_null_box_global();
        set_mem_b32s1(p + 1, eqtb_b32s1(DIMEN_BASE));
        let p = if cur_list_mode().abs() == HMODE_I {
            crate::globals::set_cur_list_aux_b32s0(1000);
            p
        } else {
            let q = crate::mem::new_noad_global();
            set_mem_b32s1(q + 1, SUB_BOX);
            set_mem_b32s0(q + 1, p);
            q
        };
        let tail = crate::globals::cur_list_tail();
        set_mem_b32s1(tail, p);
        crate::globals::set_cur_list_tail(mem_b32s1(tail));
    }
}
// translated from xetex-xetex0.c:9831 (head_for_vmode)
#[no_mangle]
pub unsafe extern "C" fn head_for_vmode() {
    if cur_list_mode() < 0 {
        if cur_cmd() != HRULE {
            off_save();
        } else {
            error_here_with_diagnostic(c"You can't use `".as_ptr());
            crate::output::print_esc_cstr(c"hrule".as_ptr());
            crate::output::print_cstr(c"' here except with leaders".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(2);
            set_help_line(
                1,
                c"To put a horizontal rule in an hbox or an alignment,".as_ptr(),
            );
            set_help_line(
                0,
                c"you should use \\leaders or \\hrulefill (see The TeXbook).".as_ptr(),
            );
            error();
        }
    } else {
        crate::input::back_input();
        set_cur_tok(crate::globals::par_token());
        crate::input::back_input();
        set_cur_input_index(INSERTED);
    }
}
// translated from xetex-xetex0.c:9760 (begin_insert_or_adjust)
#[no_mangle]
pub unsafe extern "C" fn begin_insert_or_adjust() {
    if cur_cmd() == VADJUST {
        set_cur_val(255);
    } else {
        scan_eight_bit_int();
        if cur_val() == 255 {
            error_here_with_diagnostic(c"You can't ".as_ptr());
            crate::output::print_esc_cstr(c"insert".as_ptr());
            crate::output::print_int(cur_val());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(1);
            set_help_line(
                0,
                c"I'm changing to \\insert0; box 255 is special.".as_ptr(),
            );
            error();
            set_cur_val(0);
        }
    }
    set_save_stack_b32s1(save_ptr(), cur_val());
    if cur_cmd() == VADJUST && scan_keyword(c"pre".as_ptr()) {
        set_save_stack_b32s1(save_ptr() + 1, 1);
    } else {
        set_save_stack_b32s1(save_ptr() + 1, 0);
    }
    set_save_ptr(save_ptr() + 2);
    crate::save::new_save_level(INSERT_GROUP);
    scan_left_brace();
    normal_paragraph();
    crate::mem::push_nest();
    crate::globals::set_cur_list_mode(-1); // -VMODE
    crate::globals::set_cur_list_aux_b32s1(IGNORE_DEPTH);
}
// translated from xetex-xetex0.c:10161 (do_endv)
#[no_mangle]
pub unsafe extern "C" fn do_endv() {
    const V_TEMPLATE: u16 = 2; // xetex-constants.h:201
    const TOKEN_LIST: u16 = 0; // xetex-constants.h:261
    const ALIGN_GROUP: u8 = 6; // xetex-constants.h:158

    crate::globals::set_base_ptr(crate::globals::input_ptr());
    crate::globals::set_input_stack(crate::globals::base_ptr(), crate::globals::cur_input());
    while crate::globals::input_stack(crate::globals::base_ptr()).index != V_TEMPLATE
        && crate::globals::input_stack(crate::globals::base_ptr()).loc == crate::mem::TEX_NULL
        && crate::globals::input_stack(crate::globals::base_ptr()).state == TOKEN_LIST
    {
        crate::globals::set_base_ptr(crate::globals::base_ptr() - 1);
    }
    if crate::globals::input_stack(crate::globals::base_ptr()).index != V_TEMPLATE
        || crate::globals::input_stack(crate::globals::base_ptr()).loc != crate::mem::TEX_NULL
        || crate::globals::input_stack(crate::globals::base_ptr()).state != TOKEN_LIST
    {
        crate::errors::fatal_error(c"(interwoven alignment preambles are not allowed)".as_ptr());
    }
    if cur_group() == ALIGN_GROUP {
        end_graf();
        if fin_col() {
            fin_row();
        }
    } else {
        off_save();
    }
}
// translated from xetex-xetex0.c:9799 (unpackage)
#[no_mangle]
pub unsafe extern "C" fn unpackage() {
    const COPY_CODE: i32 = 1; // xetex_format.h:481
    const VLIST_NODE: u16 = 1; // xetex_format.h:381
    const MARGIN_KERN_NODE: u16 = 40; // xetex_format.h:396
    const MARGIN_KERN_NODE_SIZE: i32 = 3; // xetex-constants.h:115
    const HMODE: i16 = 104; // xetex_format.h:369
    const MMODE: i16 = 207; // xetex_format.h:370

    if cur_chr() > COPY_CODE {
        set_mem_b32s1(
            crate::globals::cur_list_tail(),
            crate::globals::disc_ptr(cur_chr() as usize),
        );
        crate::globals::set_disc_ptr(cur_chr() as usize, crate::mem::TEX_NULL);
        // goto done — jump to tail-walk loop
    } else {
        let c = cur_chr();
        scan_register_num();
        let p = if cur_val() < 256 {
            eqtb_b32s1(BOX_BASE + cur_val())
        } else {
            crate::save::find_sa_element_global(4, cur_val(), false);
            if crate::globals::cur_ptr() == crate::mem::TEX_NULL {
                crate::mem::TEX_NULL
            } else {
                mem_b32s1(crate::globals::cur_ptr() + 1)
            }
        };

        if p == crate::mem::TEX_NULL {
            return;
        }

        let mode = cur_list_mode().abs();
        let ntype = mem_b16s1(p);
        if mode == MMODE
            || (mode == VMODE && ntype != VLIST_NODE)
            || (mode == HMODE && ntype != HLIST_NODE)
        {
            error_here_with_diagnostic(c"Incompatible list can't be unboxed".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(3);
            set_help_line(2, c"Sorry, Pandora. (You sneaky devil.)".as_ptr());
            set_help_line(
                1,
                c"I refuse to unbox an \\hbox in vertical mode or vice versa.".as_ptr(),
            );
            set_help_line(0, c"And I can't open any boxes in math mode.".as_ptr());
            error();
            return;
        }

        let list_head = mem_b32s1(p + 5);
        if c == COPY_CODE {
            set_mem_b32s1(
                crate::globals::cur_list_tail(),
                copy_node_list_global(list_head),
            );
        } else {
            set_mem_b32s1(crate::globals::cur_list_tail(), list_head);
            if cur_val() < 256 {
                crate::globals::set_eqtb_b32s1(BOX_BASE + cur_val(), crate::mem::TEX_NULL);
            } else {
                crate::save::find_sa_element_global(4, cur_val(), false);
                if crate::globals::cur_ptr() != crate::mem::TEX_NULL {
                    set_mem_b32s1(crate::globals::cur_ptr() + 1, crate::mem::TEX_NULL);
                    let rc = mem_b32s0(crate::globals::cur_ptr() + 1) + 1;
                    set_mem_b32s0(crate::globals::cur_ptr() + 1, rc);
                    crate::save::delete_sa_ref_global(crate::globals::cur_ptr());
                }
            }
            crate::mem::free_node_global(p, BOX_NODE_SIZE);
        }
    }

    // done: advance cur_list.tail past any MARGIN_KERN_NODEs
    while mem_b32s1(crate::globals::cur_list_tail()) != crate::mem::TEX_NULL {
        let r = mem_b32s1(crate::globals::cur_list_tail());
        if r < crate::globals::hi_mem_min() && mem_b16s1(r) == MARGIN_KERN_NODE {
            set_mem_b32s1(crate::globals::cur_list_tail(), mem_b32s1(r));
            crate::mem::free_node_global(r, MARGIN_KERN_NODE_SIZE);
        } else {
            crate::globals::set_cur_list_tail(mem_b32s1(crate::globals::cur_list_tail()));
        }
    }
}
// translated from xetex-xetex0.c:9716 (delete_last)
#[no_mangle]
pub unsafe extern "C" fn delete_last() {
    const BEGIN_M_CODE: u16 = 2; // xetex_format.h:436

    if cur_list_mode() == VMODE
        && crate::globals::cur_list_tail() == crate::globals::cur_list_head()
    {
        // On page: only allowed for \lastskip when last_glue != MAX_HALFWORD
        if cur_chr() as u16 != GLUE_NODE || crate::globals::last_glue() != MAX_HALFWORD {
            you_cant();
            set_help_ptr(2);
            set_help_line(
                1,
                c"Sorry...I usually can't take things from the current page.".as_ptr(),
            );
            set_help_line(0, c"Try `I\\vskip-\\lastskip' instead.".as_ptr());
            if cur_chr() as u16 == KERN_NODE {
                set_help_line(0, c"Try `I\\kern-\\lastkern' instead.".as_ptr());
            } else if cur_chr() as u16 != GLUE_NODE {
                set_help_line(
                    0,
                    c"Perhaps you can make the output routine do it.".as_ptr(),
                );
            }
            error();
        }
        return;
    }

    let mut tx = crate::globals::cur_list_tail();

    // If tail is MATH END, back up one to find the actual last "deletable" node
    if tx < crate::globals::hi_mem_min()
        && mem_b16s1(tx) == MATH_NODE
        && mem_b16s0(tx) == END_M_CODE
    {
        let mut r = crate::globals::cur_list_head();
        loop {
            let next_r = mem_b32s1(r);
            if next_r == tx {
                tx = r;
                break;
            }
            r = next_r;
        }
    }

    if tx >= crate::globals::hi_mem_min() || mem_b16s1(tx) as i32 != cur_chr() {
        return;
    }

    // Walk the list to find the node before tx
    let mut q = crate::globals::cur_list_head();
    let mut p = crate::mem::TEX_NULL;
    let mut r_save = p;
    let mut fm = false;
    loop {
        r_save = p;
        p = q;
        fm = false;
        if q < crate::globals::hi_mem_min() {
            if mem_b16s1(q) == DISC_NODE {
                let count = mem_b16s0(q);
                for _ in 0..count {
                    p = mem_b32s1(p);
                }
                if p == tx {
                    return;
                }
            } else if mem_b16s1(q) == MATH_NODE && mem_b16s0(q) == BEGIN_M_CODE {
                fm = true;
            }
        }
        q = mem_b32s1(p);
        if q == tx {
            break;
        }
    }

    // q == tx; p is the node just before tx; r_save is the one before p
    let q_next = mem_b32s1(tx);
    set_mem_b32s1(p, q_next);
    set_mem_b32s1(tx, crate::mem::TEX_NULL);
    if q_next == crate::mem::TEX_NULL {
        if fm {
            crate::errors::confusion(c"tail1".as_ptr());
        } else {
            crate::globals::set_cur_list_tail(p);
        }
    } else if fm {
        crate::globals::set_cur_list_tail(r_save);
        set_mem_b32s1(r_save, crate::mem::TEX_NULL);
        flush_node_list_global(p);
    }
    flush_node_list_global(tx);
}
// translated from xetex-xetex0.c:9223 (app_space)
#[no_mangle]
pub unsafe extern "C" fn app_space() {
    use crate::globals::{
        font_glue, font_info_b32s1, main_k, main_p, param_base, set_cur_list_tail, set_font_glue,
        set_main_k, set_main_p,
    };
    const GLUE_PAR_SPACE_SKIP: i32 = 12; // xetex_format.h:161
    const GLUE_PAR_XSPACE_SKIP: i32 = 13; // xetex_format.h:162
    const EXTRA_SPACE_CODE: i32 = 7; // xetex-constants.h:299

    let space_factor = crate::globals::cur_list_aux_b32s0();
    let cf = eqtb_b32s1(CUR_FONT_LOC);
    let q;
    if space_factor >= 2000 && eqtb_b32s1(GLUE_BASE + GLUE_PAR_XSPACE_SKIP) != 0 {
        q = crate::mem::new_param_glue_global(GLUE_PAR_XSPACE_SKIP as i16);
    } else {
        if eqtb_b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP) != 0 {
            set_main_p(eqtb_b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP));
        } else {
            set_main_p(font_glue(cf));
            if main_p() == crate::mem::TEX_NULL {
                set_main_p(crate::mem::new_spec_global(0));
                set_main_k(param_base(cf) + 2);
                // width = space, stretch = space_stretch, shrink = space_shrink
                crate::globals::set_mem_b32s1(main_p() + 1, font_info_b32s1(main_k()));
                crate::globals::set_mem_b32s1(main_p() + 2, font_info_b32s1(main_k() + 1));
                crate::globals::set_mem_b32s1(main_p() + 3, font_info_b32s1(main_k() + 2));
                set_font_glue(cf, main_p());
            }
        }
        set_main_p(crate::mem::new_spec_global(main_p()));
        if space_factor >= 2000 {
            // add extra_space to width
            let extra = font_info_b32s1(EXTRA_SPACE_CODE + param_base(cf));
            crate::globals::set_mem_b32s1(main_p() + 1, mem_b32s1(main_p() + 1) + extra);
        }
        // scale stretch by space_factor / 1000
        let new_stretch =
            crate::scaled_math::xn_over_d(mem_b32s1(main_p() + 2), space_factor, 1000);
        crate::globals::set_mem_b32s1(main_p() + 2, new_stretch);
        // scale shrink by 1000 / space_factor
        let new_shrink = crate::scaled_math::xn_over_d(mem_b32s1(main_p() + 3), 1000, space_factor);
        crate::globals::set_mem_b32s1(main_p() + 3, new_shrink);
        q = crate::mem::new_glue_global(main_p());
        crate::globals::set_mem_b32s1(main_p(), crate::mem::TEX_NULL);
    }
    crate::globals::set_mem_b32s1(crate::globals::cur_list_tail(), q);
    set_cur_list_tail(q);
}
// translated from xetex-xetex0.c:9485 (scan_box)
#[no_mangle]
pub unsafe extern "C" fn scan_box(box_context: i32) {
    use crate::globals::{cur_cmd, set_cur_box};
    const MAKE_BOX: u8 = 20; // xetex_format.h:258
    const HRULE: u8 = 36; // xetex_format.h:274
    const VRULE: u8 = 35; // xetex_format.h:273
    const LEADER_FLAG: i32 = 0x40010001; // xetex-constants.h:375
    loop {
        get_x_token();
        if cur_cmd() != SPACER && cur_cmd() != RELAX {
            break;
        }
    }
    if cur_cmd() == MAKE_BOX {
        begin_box(box_context);
    } else if box_context >= LEADER_FLAG && (cur_cmd() == HRULE || cur_cmd() == VRULE) {
        set_cur_box(scan_rule_spec());
        box_end(box_context);
    } else {
        error_here_with_diagnostic(c"A <box> was supposed to be here".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(3);
        set_help_line(
            2,
            c"I was expecting to see \\hbox or \\vbox or \\copy or \\box or".as_ptr(),
        );
        set_help_line(
            1,
            c"something like that. So you might find something missing in".as_ptr(),
        );
        set_help_line(
            0,
            c"your output. But keep trying; you can fix this later.".as_ptr(),
        );
        crate::input::back_error();
    }
}
// translated from xetex-xetex0.c:9509 (package)
#[no_mangle]
pub unsafe extern "C" fn package(c: i16) {
    use crate::globals::{
        cur_box, cur_list_head, cur_list_mode, save_ptr, save_stack_b32s1, set_cur_box,
        set_save_ptr,
    };
    const DIMEN_PAR_BOX_MAX_DEPTH: i32 = 7; // xetex_format.h:129
    const INT_PAR_XETEX_UPWARDS: i32 = 71; // xetex_format.h:106
    const VTOP_CODE: i16 = 4; // xetex_format.h:484
    const INNER_HMODE: i16 = -104; // xetex_format.h:369 negated

    let d = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_BOX_MAX_DEPTH);
    let u = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS);
    crate::save::unsave();
    set_save_ptr(save_ptr() - 3);
    let v = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS);
    crate::globals::set_eqtb_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS, u);
    if cur_list_mode() == INNER_HMODE {
        set_cur_box(hpack(
            mem_b32s1(cur_list_head()),
            save_stack_b32s1(save_ptr() + 2),
            save_stack_b32s1(save_ptr() + 1) as i16,
        ));
    } else {
        set_cur_box(vpackage(
            mem_b32s1(cur_list_head()),
            save_stack_b32s1(save_ptr() + 2),
            save_stack_b32s1(save_ptr() + 1) as i16,
            d,
        ));
        if c == VTOP_CODE {
            let mut h = 0i32;
            let p = mem_b32s1(cur_box() + 5);
            if p != crate::mem::TEX_NULL && mem_b16s1(p) <= 2 {
                h = mem_b32s1(p + 3);
            }
            crate::globals::set_mem_b32s1(
                cur_box() + 2,
                mem_b32s1(cur_box() + 2) - h + mem_b32s1(cur_box() + 3),
            );
            crate::globals::set_mem_b32s1(cur_box() + 3, h);
        }
    }
    crate::globals::set_eqtb_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS, v);
    crate::mem::pop_nest();
    box_end(save_stack_b32s1(save_ptr()));
}
// translated from xetex-xetex0.c:9549 (new_graf)
#[no_mangle]
pub unsafe extern "C" fn new_graf(indented: bool) {
    use crate::globals::{
        cur_list_head, cur_list_mode, cur_list_tail, insert_src_special_every_par, nest_ptr,
        set_cur_lang, set_cur_list_aux_b32s0, set_cur_list_aux_b32s1, set_cur_list_mode,
        set_cur_list_prev_graf, set_cur_list_tail, set_mem_b32s1,
    };
    const GLUE_PAR_PAR_SKIP: i16 = 2; // xetex_format.h:151
    const VMODE_I16: i16 = 1; // xetex_format.h:369
    const HMODE_I16: i16 = 104; // xetex_format.h:369
    const BIGGEST_LANG: i32 = 255; // xetex-constants.h:327
    const LOCAL__EVERY_PAR: i32 = 2; // xetex_format.h:174
    const EVERY_PAR_TEXT: u16 = 8; // xetex-constants.h:207

    set_cur_list_prev_graf(0);

    if cur_list_mode() == VMODE_I16 || cur_list_head() != cur_list_tail() {
        let pg = crate::mem::new_param_glue_global(GLUE_PAR_PAR_SKIP);
        set_mem_b32s1(cur_list_tail(), pg);
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
    }

    crate::mem::push_nest();
    set_cur_list_mode(HMODE_I16);
    set_cur_list_aux_b32s0(1000);

    let lang_val = eqtb_b32s1(INT_BASE + INT_PAR_LANGUAGE);
    let l: u8 = if lang_val <= 0 || lang_val > BIGGEST_LANG {
        0
    } else {
        lang_val as u8
    };
    set_cur_lang(l);
    set_cur_list_aux_b32s1(l as i32);
    let lhm = crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as i32;
    let rhm = crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as i32;
    set_cur_list_prev_graf((lhm * 64 + rhm) * 65536 + l as i32);

    if indented {
        let nb = crate::mem::new_null_box_global();
        set_cur_list_tail(nb);
        set_mem_b32s1(cur_list_head(), nb);
        set_mem_b32s1(nb + 1, eqtb_b32s1(DIMEN_BASE));
        if insert_src_special_every_par() {
            insert_src_special();
        }
    }

    let every_par = eqtb_b32s1(LOCAL_BASE + LOCAL__EVERY_PAR);
    if every_par != crate::mem::TEX_NULL {
        begin_token_list(every_par, EVERY_PAR_TEXT);
    }

    if nest_ptr() == 1 {
        build_page();
    }
}
// translated from xetex-xetex0.c:7048 (new_character)
pub unsafe fn new_character(f: i32, c: u16) -> i32 {
    extern "C" {}
    use crate::globals::{char_base_entry, font_bc_entry, font_ec_entry, font_info_b16};

    let fa = crate::globals::font_area_entry(f);
    if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
        return new_native_character(f, c as u32);
    }
    let ec = effective_char_global(false, f, c) as u16;
    if font_bc_entry(f) <= ec && font_ec_entry(f) >= ec {
        let info = font_info_b16(char_base_entry(f) + ec as i32);
        if info.s3 > 0 {
            let p = crate::mem::get_avail_global();
            crate::globals::set_mem_b16s1(p, f as u16);
            crate::globals::set_mem_b16s0(p, c);
            return p;
        }
    }
    char_warning(f, c as i32);
    crate::mem::TEX_NULL
}
// translated from xetex-xetex0.c:7797 (box_end)
#[no_mangle]
pub unsafe extern "C" fn box_end(box_context: i32) {
    use crate::globals::{
        adjust_tail, cur_box, cur_list_mode, cur_list_tail, cur_ptr, pre_adjust_tail,
        set_adjust_tail, set_cur_box, set_cur_list_aux_b32s0, set_cur_list_tail, set_cur_val,
        set_pre_adjust_tail,
    };
    use crate::mem::TEX_NULL;

    const VMODE: i16 = 1;
    const HMODE: i16 = 104;
    const BOX_FLAG: i32 = 0x4000_0000; // xetex-constants.h:372
    const GLOBAL_BOX_FLAG: i32 = 0x4000_8000; // xetex-constants.h:373
    const SHIP_OUT_FLAG: i32 = 0x4001_0000; // xetex-constants.h:374
    const LEADER_FLAG: i32 = 0x4001_0001; // xetex-constants.h:375
    const A_LEADERS: i32 = 100; // xetex_format.h:422
    const ADJUST_HEAD: i32 = 4_999_994; // MEM_TOP - 5 (xetex-constants.h:42)
    const PRE_ADJUST_HEAD: i32 = 4_999_985; // MEM_TOP - 14 (xetex-constants.h:51)
    const SUB_BOX: i32 = 2; // xetex-constants.h:283
    const BOX_REF: u16 = 121; // xetex_format.h:363
    const SPACER: u8 = 10; // xetex_format.h:242
    const RELAX: u8 = 0; // xetex_format.h:229
    const HSKIP: u8 = 26; // xetex_format.h:264
    const VSKIP: u8 = 27; // xetex_format.h:265

    if box_context < BOX_FLAG {
        if cur_box() != TEX_NULL {
            set_mem_b32s1(cur_box() + 4, box_context);

            if cur_list_mode().abs() == VMODE {
                if pre_adjust_tail() != TEX_NULL {
                    if PRE_ADJUST_HEAD != pre_adjust_tail() {
                        set_mem_b32s1(cur_list_tail(), mem_b32s1(PRE_ADJUST_HEAD));
                        set_cur_list_tail(pre_adjust_tail());
                    }
                    set_pre_adjust_tail(TEX_NULL);
                }

                append_to_vlist(cur_box());

                if adjust_tail() != TEX_NULL {
                    if ADJUST_HEAD != adjust_tail() {
                        set_mem_b32s1(cur_list_tail(), mem_b32s1(ADJUST_HEAD));
                        set_cur_list_tail(adjust_tail());
                    }
                    set_adjust_tail(TEX_NULL);
                }

                if cur_list_mode() > 0 {
                    build_page();
                }
            } else {
                if cur_list_mode().abs() == HMODE {
                    set_cur_list_aux_b32s0(1000); // space_factor
                } else {
                    let p = crate::mem::new_noad_global();
                    set_mem_b32s1(p + 1, SUB_BOX);
                    set_mem_b32s0(p + 1, cur_box());
                    set_cur_box(p);
                }

                set_mem_b32s1(cur_list_tail(), cur_box());
                set_cur_list_tail(cur_box());
            }
        }
    } else if box_context < SHIP_OUT_FLAG {
        let (val, a) = if box_context < GLOBAL_BOX_FLAG {
            (box_context - BOX_FLAG, 0i16)
        } else {
            (box_context - GLOBAL_BOX_FLAG, 4i16)
        };
        set_cur_val(val);

        if cur_val() < 256 {
            if a >= 4 {
                crate::save::geq_define(BOX_BASE + cur_val(), BOX_REF, cur_box());
            } else {
                crate::save::eq_define(BOX_BASE + cur_val(), BOX_REF, cur_box());
            }
        } else {
            crate::save::find_sa_element_global(4, cur_val(), true);
            if a >= 4 {
                crate::save::gsa_def_global(cur_ptr(), cur_box());
            } else {
                crate::save::sa_def_global(cur_ptr(), cur_box());
            }
        }
    } else if cur_box() != TEX_NULL {
        if box_context > SHIP_OUT_FLAG {
            loop {
                get_x_token();
                if cur_cmd() != SPACER && cur_cmd() != RELAX {
                    break;
                }
            }

            if (cur_cmd() == HSKIP && cur_list_mode().abs() != VMODE)
                || (cur_cmd() == VSKIP && cur_list_mode().abs() == VMODE)
            {
                append_glue();
                set_mem_b16s0(
                    cur_list_tail(),
                    (box_context - (LEADER_FLAG - A_LEADERS)) as u16,
                );
                set_mem_b32s1(cur_list_tail() + 1, cur_box());
            } else {
                error_here_with_diagnostic(c"Leaders not followed by proper glue".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(3);
                set_help_line(
                    2,
                    c"You should say `\\leaders <box or rule><hskip or vskip>'.".as_ptr(),
                );
                set_help_line(
                    1,
                    c"I found the <box or rule>, but there's no suitable".as_ptr(),
                );
                set_help_line(
                    0,
                    c"<hskip or vskip>, so I'm ignoring these leaders.".as_ptr(),
                );
                crate::input::back_error();
                flush_node_list_global(cur_box());
            }
        } else {
            ship_out(cur_box());
        }
    }
}
// translated from xetex-xetex0.c:7794 (begin_box)
#[no_mangle]
pub unsafe extern "C" fn begin_box(box_context: i32) {
    use crate::globals::{
        cur_box, cur_list_head, cur_list_mode, cur_list_tail, cur_ptr, hi_mem_min, set_cur_box,
        set_cur_list_aux_b32s0, set_cur_list_aux_b32s1, set_cur_list_mode, set_cur_list_tail,
        set_save_stack_b32s1,
    };
    use crate::mem::TEX_NULL;

    const BOX_CODE: i32 = 0; // xetex_format.h:480
    const COPY_CODE: i32 = 1; // xetex_format.h:481
    const LAST_BOX_CODE: i32 = 2; // xetex_format.h:482
    const VSPLIT_CODE_I: i32 = 3; // xetex_format.h:483
    const BOX_FLAG: i32 = 0x4000_0000; // xetex-constants.h:372
    const VLIST_NODE: u16 = 1; // xetex_format.h:381
    const MMODE: i16 = 207; // xetex_format.h:370
    const VMODE: i16 = 1; // xetex_format.h:368
    const HMODE: i16 = 104; // xetex_format.h:369
    const BEGIN_M_CODE: u16 = 2; // xetex_format.h:436
    const END_M_CODE: u16 = 3; // xetex_format.h:437
    const HBOX_GROUP: u8 = 2; // xetex-constants.h:154
    const ADJUSTED_HBOX_GROUP: u8 = 3; // xetex-constants.h:155
    const VBOX_GROUP: u8 = 4; // xetex-constants.h:156
    const VTOP_GROUP: u8 = 5; // xetex-constants.h:157
    const EVERY_HBOX_TEXT: u16 = 11; // xetex-constants.h:210
    const EVERY_VBOX_TEXT: u16 = 12; // xetex-constants.h:211
    const IGNORE_DEPTH: i32 = -65_536_000; // xetex-constants.h:388
    const LOCAL_EVERY_HBOX: i32 = 5; // xetex_format.h:177
    const LOCAL_EVERY_VBOX: i32 = 6; // xetex_format.h:178

    match cur_chr() {
        BOX_CODE => {
            scan_register_num();
            let v = cur_val();
            if v < 256 {
                set_cur_box(eqtb_b32s1(BOX_BASE + v));
            } else {
                crate::save::find_sa_element_global(4, v, false);
                set_cur_box(if cur_ptr() == TEX_NULL {
                    TEX_NULL
                } else {
                    mem_b32s1(cur_ptr() + 1)
                });
            }
            if v < 256 {
                set_eqtb_b32s1(BOX_BASE + v, TEX_NULL);
            } else {
                crate::save::find_sa_element_global(4, v, false);
                if cur_ptr() != TEX_NULL {
                    set_mem_b32s1(cur_ptr() + 1, TEX_NULL);
                    set_mem_b32s0(cur_ptr() + 1, mem_b32s0(cur_ptr() + 1) + 1);
                    crate::save::delete_sa_ref_global(cur_ptr());
                }
            }
        }

        COPY_CODE => {
            scan_register_num();
            let v = cur_val();
            let q = if v < 256 {
                eqtb_b32s1(BOX_BASE + v)
            } else {
                crate::save::find_sa_element_global(4, v, false);
                if cur_ptr() == TEX_NULL {
                    TEX_NULL
                } else {
                    mem_b32s1(cur_ptr() + 1)
                }
            };
            set_cur_box(copy_node_list_global(q));
        }

        LAST_BOX_CODE => {
            set_cur_box(TEX_NULL);

            if cur_list_mode().abs() == MMODE {
                you_cant();
                set_help_ptr(1);
                set_help_line(0, c"Sorry; this \\lastbox will be void.".as_ptr());
                error();
            } else if cur_list_mode() == VMODE && cur_list_head() == cur_list_tail() {
                you_cant();
                set_help_ptr(2);
                set_help_line(
                    1,
                    c"Sorry...I usually can't take things from the current page.".as_ptr(),
                );
                set_help_line(0, c"This \\lastbox will therefore be void.".as_ptr());
                error();
            } else {
                let mut tx = cur_list_tail();
                if tx < hi_mem_min() && mem_b16s1(tx) == MATH_NODE && mem_b16s0(tx) == END_M_CODE {
                    let mut r = cur_list_head();
                    loop {
                        let prev = r;
                        r = mem_b32s1(prev);
                        if r == tx {
                            tx = prev;
                            break;
                        }
                    }
                }
                'last_box_done: {
                    if tx < hi_mem_min() {
                        let nt = mem_b16s1(tx);
                        if nt == HLIST_NODE || nt == VLIST_NODE {
                            let mut q = cur_list_head();
                            let mut p = TEX_NULL;
                            let mut r = TEX_NULL;
                            let mut fm = false;
                            loop {
                                r = p;
                                p = q;
                                fm = false;
                                if q < hi_mem_min() {
                                    if mem_b16s1(q) == DISC_NODE {
                                        for _ in 1..=mem_b16s0(q) {
                                            p = mem_b32s1(p);
                                        }
                                        if p == tx {
                                            break 'last_box_done;
                                        }
                                    } else if mem_b16s1(q) == MATH_NODE
                                        && mem_b16s0(q) == BEGIN_M_CODE
                                    {
                                        fm = true;
                                    }
                                }
                                q = mem_b32s1(p);
                                if q == tx {
                                    break;
                                }
                            }
                            let q2 = mem_b32s1(tx);
                            set_mem_b32s1(p, q2);
                            set_mem_b32s1(tx, TEX_NULL);
                            if q2 == TEX_NULL {
                                if fm {
                                    crate::errors::confusion(c"tail1".as_ptr());
                                } else {
                                    set_cur_list_tail(p);
                                }
                            } else if fm {
                                set_cur_list_tail(r);
                                set_mem_b32s1(r, TEX_NULL);
                                flush_node_list_global(p);
                            }
                            set_cur_box(tx);
                            set_mem_b32s1(cur_box() + 4, 0);
                        }
                    }
                }
            }
        }

        VSPLIT_CODE_I => {
            scan_register_num();
            let n = cur_val();
            if !scan_keyword(c"to".as_ptr()) {
                error_here_with_diagnostic(c"Missing `to' inserted".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(
                    1,
                    c"I'm working on `\\vsplit<box number> to <dimen>';".as_ptr(),
                );
                set_help_line(0, c"will look for the <dimen> next.".as_ptr());
                error();
            }
            scan_dimen(false, false, false);
            set_cur_box(vsplit(n, cur_val()));
        }

        _ => {
            let k = cur_chr() - 4;
            set_save_stack_b32s1(save_ptr(), box_context);
            if k == HMODE as i32 {
                if box_context < BOX_FLAG && cur_list_mode().abs() == VMODE {
                    scan_spec(ADJUSTED_HBOX_GROUP, true);
                } else {
                    scan_spec(HBOX_GROUP, true);
                }
            } else {
                if k == VMODE as i32 {
                    scan_spec(VBOX_GROUP, true);
                } else {
                    scan_spec(VTOP_GROUP, true);
                    // k = VMODE for the nest push below
                }
                normal_paragraph();
            }
            crate::mem::push_nest();
            // If k was adjusted to VTOP_GROUP branch: k was neither HMODE nor VMODE,
            // so the else branch above ran and after scan_spec(VTOP_GROUP), we want k=VMODE.
            let k_mode = if k != HMODE as i32 && k != VMODE as i32 {
                VMODE as i32
            } else {
                k
            };
            set_cur_list_mode(-(k_mode as i16));
            if k_mode == VMODE as i32 {
                set_cur_list_aux_b32s1(IGNORE_DEPTH);
                let every_vbox = eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_VBOX);
                if every_vbox != TEX_NULL {
                    begin_token_list(every_vbox, EVERY_VBOX_TEXT);
                }
            } else {
                set_cur_list_aux_b32s0(1000);
                let every_hbox = eqtb_b32s1(LOCAL_BASE + LOCAL_EVERY_HBOX);
                if every_hbox != TEX_NULL {
                    begin_token_list(every_hbox, EVERY_HBOX_TEXT);
                }
            }
            return; // no box_end call for new-group cases
        }
    }

    box_end(box_context);
}
// translated from xetex-xetex0.c:8265 (handle_right_brace)
#[no_mangle]
pub unsafe extern "C" fn handle_right_brace() {
    use crate::globals::{
        cur_group, cur_input, cur_list_head, cur_list_tail, disc_ptr, nest_ptr, page_tail,
        save_ptr, save_stack_b32s1, set_adjust_tail, set_cur_list_tail, set_cur_tok, set_disc_ptr,
        set_insert_penalties, set_mem_b16s0, set_mem_b16s1, set_nest_tail, set_output_active,
        set_page_tail, set_pre_adjust_tail, set_save_ptr,
    };
    use crate::mem::TEX_NULL;

    const BOTTOM_LEVEL: u8 = 0; // xetex-constants.h:152
    const SIMPLE_GROUP: u8 = 1;
    const HBOX_GROUP: u8 = 2;
    const ADJUSTED_HBOX_GROUP: u8 = 3;
    const VBOX_GROUP: u8 = 4;
    const VTOP_GROUP: u8 = 5;
    const ALIGN_GROUP: u8 = 6;
    const NO_ALIGN_GROUP: u8 = 7;
    const OUTPUT_GROUP: u8 = 8;
    const MATH_GROUP: u8 = 9;
    const DISC_GROUP: u8 = 10;
    const INSERT_GROUP: u8 = 11;
    const VCENTER_GROUP: u8 = 12;
    const MATH_CHOICE_GROUP: u8 = 13;
    const SEMI_SIMPLE_GROUP: u8 = 14;
    const MATH_SHIFT_GROUP: u8 = 15;
    const MATH_LEFT_GROUP: u8 = 16;

    const INS_NODE_SIZE: i32 = 5;
    const NOAD_SIZE: i32 = 4;
    const ADDITIONAL: i16 = 1;
    const VTOP_CODE: i16 = 4; // xetex_format.h:484
    const ADJUST_HEAD: i32 = 4_999_994; // MEM_TOP - 5
    const PRE_ADJUST_HEAD: i32 = 4_999_985; // MEM_TOP - 14
    const CONTRIB_HEAD: i32 = 4_999_998; // MEM_TOP - 1
    const PAGE_HEAD: i32 = 4_999_997; // MEM_TOP - 2
    const LAST_BOX_CODE: usize = 2; // xetex_format.h:482
    const OUTPUT_TEXT: u16 = 7; // xetex-constants.h:206
    const BACKED_UP: u16 = 3; // xetex-constants.h:202
    const DIMEN_PAR_SPLIT_MAX_DEPTH: i32 = 6; // xetex_format.h:128
    const INT_PAR_FLOATING_PENALTY: i32 = 42; // xetex_format.h:77
    const GLUE_PAR_SPLIT_TOP_SKIP: i32 = 10; // xetex_format.h:159
    const INS_NODE: u16 = 3; // xetex_format.h:383
    const ADJUST_NODE: u16 = 5; // xetex_format.h:385
    const VCENTER_NOAD: u16 = 29; // xetex_format.h:414
    const ORD_NOAD: u16 = 16; // xetex_format.h:401
    const ACCENT_NOAD: u16 = 28; // xetex_format.h:413
    const SUB_BOX: i32 = 2; // xetex-constants.h:283
    const SUB_MLIST: i32 = 3; // xetex-constants.h:287
    const EMPTY: i32 = 0; // xetex-constants.h:230
    const FROZEN_CR: i32 = 2_243_226 + 1; // FROZEN_CONTROL_SEQUENCE + 1

    match cur_group() {
        SIMPLE_GROUP => {
            crate::save::unsave();
        }

        BOTTOM_LEVEL => {
            error_here_with_diagnostic(c"Too many }'s".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(2);
            set_help_line(1, c"You've closed more groups than you opened.".as_ptr());
            set_help_line(
                0,
                c"Such booboos are generally harmless, so keep going.".as_ptr(),
            );
            error();
        }

        SEMI_SIMPLE_GROUP | MATH_SHIFT_GROUP | MATH_LEFT_GROUP => {
            extra_right_brace();
        }

        HBOX_GROUP => {
            package(0);
        }

        ADJUSTED_HBOX_GROUP => {
            set_adjust_tail(ADJUST_HEAD);
            set_pre_adjust_tail(PRE_ADJUST_HEAD);
            package(0);
        }

        VBOX_GROUP => {
            end_graf();
            package(0);
        }

        VTOP_GROUP => {
            end_graf();
            package(VTOP_CODE);
        }

        INSERT_GROUP => {
            end_graf();
            // GLUEPAR(split_top_skip)
            let q = eqtb_b32s1(GLUE_BASE + GLUE_PAR_SPLIT_TOP_SKIP);
            // GLUE_SPEC_ref_count(q)++
            set_mem_b32s1(q, mem_b32s1(q) + 1);
            let d = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_SPLIT_MAX_DEPTH);
            let f = eqtb_b32s1(INT_BASE + INT_PAR_FLOATING_PENALTY);
            crate::save::unsave();
            set_save_ptr(save_ptr() - 2);
            let p = vpackage(mem_b32s1(cur_list_head()), 0, ADDITIONAL, MAX_HALFWORD);
            crate::mem::pop_nest();

            if save_stack_b32s1(save_ptr()) < 255 {
                let new_node = crate::mem::get_node_global(INS_NODE_SIZE);
                set_mem_b32s1(cur_list_tail(), new_node);
                set_cur_list_tail(mem_b32s1(cur_list_tail()));
                set_mem_b16s1(cur_list_tail(), INS_NODE);
                set_mem_b16s0(cur_list_tail(), save_stack_b32s1(save_ptr()) as u16);
                set_mem_b32s1(cur_list_tail() + 3, mem_b32s1(p + 3) + mem_b32s1(p + 2));
                set_mem_b32s0(cur_list_tail() + 4, mem_b32s1(p + 5)); // ins list ptr
                set_mem_b32s1(cur_list_tail() + 4, q);
                set_mem_b32s1(cur_list_tail() + 2, d);
                set_mem_b32s1(cur_list_tail() + 1, f);
            } else {
                let new_node = crate::mem::get_node_global(SMALL_NODE_SIZE);
                set_mem_b32s1(cur_list_tail(), new_node);
                set_cur_list_tail(mem_b32s1(cur_list_tail()));
                set_mem_b16s1(cur_list_tail(), ADJUST_NODE);
                set_mem_b16s0(cur_list_tail(), save_stack_b32s1(save_ptr() + 1) as u16);
                set_mem_b32s1(cur_list_tail() + 1, mem_b32s1(p + 5));
                crate::mem::delete_glue_ref_global(q);
            }

            crate::mem::free_node_global(p, BOX_NODE_SIZE);
            if nest_ptr() == 0 {
                build_page();
            }
        }

        OUTPUT_GROUP => {
            if cur_input().loc != TEX_NULL
                || (cur_input().index != OUTPUT_TEXT && cur_input().index != BACKED_UP)
            {
                error_here_with_diagnostic(c"Unbalanced output routine".as_ptr());
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(
                    1,
                    c"Your sneaky output routine has problematic {'s and/or }'s.".as_ptr(),
                );
                set_help_line(0, c"I can't handle that very well; good luck.".as_ptr());
                error();
                loop {
                    get_token();
                    if cur_input().loc == TEX_NULL {
                        break;
                    }
                }
            }

            crate::input::end_token_list();
            end_graf();
            crate::save::unsave();
            set_output_active(false);
            set_insert_penalties(0);

            if eqtb_b32s1(BOX_BASE + 255) != TEX_NULL {
                error_here_with_diagnostic(c"Output routine didn't use all of ".as_ptr());
                crate::output::print_esc_cstr(c"box".as_ptr());
                crate::output::print_int(255);
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(3);
                set_help_line(2, c"Your \\output commands should empty \\box255,".as_ptr());
                set_help_line(1, c"e.g., by saying `\\shipout\\box255'.".as_ptr());
                set_help_line(0, c"Proceed; I'll discard its present contents.".as_ptr());
                box_error(255);
            }

            if cur_list_tail() != cur_list_head() {
                set_mem_b32s1(page_tail(), mem_b32s1(cur_list_head()));
                set_page_tail(cur_list_tail());
            }

            if mem_b32s1(PAGE_HEAD) != TEX_NULL {
                if mem_b32s1(CONTRIB_HEAD) == TEX_NULL {
                    set_nest_tail(0, page_tail());
                }
                set_mem_b32s1(page_tail(), mem_b32s1(CONTRIB_HEAD));
                set_mem_b32s1(CONTRIB_HEAD, mem_b32s1(PAGE_HEAD));
                set_mem_b32s1(PAGE_HEAD, TEX_NULL);
                set_page_tail(PAGE_HEAD);
            }

            flush_node_list_global(disc_ptr(LAST_BOX_CODE));
            set_disc_ptr(LAST_BOX_CODE, TEX_NULL);
            crate::mem::pop_nest();
            build_page();
        }

        DISC_GROUP => {
            build_discretionary();
        }

        ALIGN_GROUP => {
            crate::input::back_input();
            set_cur_tok(CS_TOKEN_FLAG + FROZEN_CR);
            error_here_with_diagnostic(c"Missing ".as_ptr());
            crate::output::print_esc_cstr(c"cr".as_ptr());
            crate::output::print_cstr(c" inserted".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(1);
            set_help_line(
                0,
                c"I'm guessing that you meant to end an alignment here.".as_ptr(),
            );
            crate::input::ins_error();
        }

        NO_ALIGN_GROUP => {
            end_graf();
            crate::save::unsave();
            align_peek();
        }

        VCENTER_GROUP => {
            end_graf();
            crate::save::unsave();
            set_save_ptr(save_ptr() - 2);
            let p = vpackage(
                mem_b32s1(cur_list_head()),
                save_stack_b32s1(save_ptr() + 1),
                save_stack_b32s1(save_ptr()) as i16,
                MAX_HALFWORD,
            );
            crate::mem::pop_nest();
            let new_noad_global = crate::mem::new_noad_global();
            set_mem_b32s1(cur_list_tail(), new_noad_global);
            set_cur_list_tail(mem_b32s1(cur_list_tail()));
            set_mem_b16s1(cur_list_tail(), VCENTER_NOAD);
            set_mem_b32s1(cur_list_tail() + 1, SUB_BOX);
            set_mem_b32s0(cur_list_tail() + 1, p);
        }

        MATH_CHOICE_GROUP => {
            build_choices_global();
        }

        MATH_GROUP => {
            crate::save::unsave();
            set_save_ptr(save_ptr() - 1);
            let save_loc = save_stack_b32s1(save_ptr());
            set_mem_b32s1(save_loc, SUB_MLIST);
            let p = fin_mlist_global(TEX_NULL);
            set_mem_b32s0(save_loc, p);

            if p != TEX_NULL && mem_b32s1(p) == TEX_NULL {
                if mem_b16s1(p) == ORD_NOAD
                    && mem_b32s1(p + 3) == EMPTY
                    && mem_b32s1(p + 2) == EMPTY
                {
                    // collapse to inline field
                    set_mem_b32s0(save_loc, mem_b32s0(p + 1));
                    set_mem_b32s1(save_loc, mem_b32s1(p + 1));
                    crate::mem::free_node_global(p, NOAD_SIZE);
                } else if mem_b16s1(p) == ACCENT_NOAD
                    && save_loc == cur_list_tail() + 1
                    && mem_b16s1(cur_list_tail()) == ORD_NOAD
                {
                    // merge accent noad into ord
                    let mut q = cur_list_head();
                    while mem_b32s1(q) != cur_list_tail() {
                        q = mem_b32s1(q);
                    }
                    set_mem_b32s1(q, p);
                    crate::mem::free_node_global(cur_list_tail(), NOAD_SIZE);
                    set_cur_list_tail(p);
                }
            }
        }

        _ => {
            crate::errors::confusion(c"rightbrace".as_ptr());
        }
    }
}
