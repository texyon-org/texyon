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
    CS_TOKEN_FLAG, DIMEN_BASE, GLUE_BASE, HLIST_NODE, INT_BASE, LOCAL_BASE, MAX_HALFWORD,
    NULL_FLAG, RULE_NODE,
};
use crate::errors::error;
use crate::expand::expand;
use crate::globals::{
    align_state, cur_chr, cur_cmd, cur_tok, cur_val, eqtb_b32s1, mem_b32s0, mem_b32s1,
    set_align_state, set_cur_tok, set_help_line, set_help_ptr,
};
use crate::input::begin_token_list;
use crate::math::{flush_math_global, resume_after_display};
use crate::output::{capture_to_diagnostic, error_here_with_diagnostic, print_cmd_chr};
use crate::packaging::{append_to_vlist, flush_node_list_global, hpack, vpackage};
use crate::pagebuilder::build_page;
use crate::scan::{
    do_assignments, get_token, get_x_or_protected, get_x_token, normal_paragraph, scan_glue,
    scan_left_brace, scan_optional_equals, scan_spec,
};

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
