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
    BATCH_MODE, BIGGEST_CHAR, BOX_BASE, BOX_NODE_SIZE, CAT_CODE_BASE, CS_TOKEN_FLAG, CUR_FONT_LOC,
    DEL_CODE_BASE, DIMEN_BASE, DISC_NODE, ERROR_STOP_MODE, ETEX_PEN_BASE, GLUE_BASE, GLUE_NODE,
    HLIST_NODE, INT_BASE, KERN_NODE, LIGATURE_NODE, LOCAL_BASE, MATH_CODE_BASE, MATH_NODE,
    MAX_HALFWORD, NULL_FLAG, PENALTY_NODE, PRIM_EQTB_BASE, RULE_NODE, SELECTOR_TERM_ONLY,
    SF_CODE_BASE, SINGLE_BASE, SMALL_NODE_SIZE, TEX_NULL, TOKS_BASE, TOO_BIG_CHAR,
};
use crate::ext::{append_src_special_in, apply_mapping, do_extension_in};
use crate::fonts::read_font_info_in;
use crate::globals::{
    cur_list_mode, set_cur_input_index, };
use crate::hyphenation::{new_hyph_exceptions_in, new_patterns_in};
use crate::math::{after_math_in, init_math_in};
use crate::node::{AnyNode, GlueSpecNode, NodePtr};
use crate::scan::{set_class_fn, set_family_fn};
use crate::units::{MemIdx, MemPtr, Scaled};

extern "C" {
    fn ttstub_diag_printf(d: *mut std::ffi::c_void, fmt: *const libc::c_char, ...) -> libc::c_int;
    fn set_cp_code(font_num: i32, code: u32, side: i32, value: i32);
}

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
const ASSIGN_INT: u8 = 74;
const ASSIGN_MU_GLUE: u8 = 77;
const BACKED_UP: u16 = 3;
const BOTTOM_LEVEL: u8 = 0; // xetex_format.h (group level 0)
const CHAR_GIVEN: u8 = 68;
const COUNT_BASE: i32 = 7_826_812;
const DISC_GROUP: u8 = 10; // xetex-constants.h:162
const FONT_BASE: i32 = 0;
const FONT_ID_BASE: i32 = 2_245_338; // FROZEN_NULL_FONT
const FROZEN_END_GROUP: i32 = 2_243_226 + 2; // xetex_format.h:201 + xetex-constants.h:57
const FROZEN_RIGHT: i32 = 2_243_226 + 3; // xetex_format.h:201 + xetex-constants.h:58
const GLYPH_NODE_BD: u16 = 42; // xetex_format.h:465
const HASH_BASE: i32 = 2_228_226;
const INT_VAL: i16 = 0;
const LANGUAGE_NODE_SUBTYPE: i16 = 4; // xetex_format.h:461 (whatsit subtype)
const MARK_NODE: u16 = 4; // xetex_format.h:384
const MATH_SHIFT_TOKEN: i32 = 0x600000;
const MAX_CHAR_VAL: i32 = 0x200000; // 1 << 21
const MMODE: i16 = 207; // xetex_format.h:370
const MU_GLUE: u16 = 99;
const MU_SKIP_BASE: i32 = 2_254_615;
const MU_VAL_SCAN: i16 = 3;
const NATIVE_WORD_NODE: u16 = 40; // xetex_format.h:463
const NATIVE_WORD_NODE_AT: u16 = 41; // xetex_format.h:464
const NULL_CS: i32 = 2_228_225; // xetex_format.h:199
const OTHER_CHAR: u8 = 12;
const OTHER_TOKEN: i32 = 0x1800000; // OTHER_CHAR << 21
const REGISTER: u8 = 91;
const SCALED_BASE: i32 = 8_941_203;
const SELECTOR_NEW_STRING: i32 = 21;
const SELECTOR_NO_PRINT: i32 = 16; // xetex-core.h:45
const SET_FONT: u8 = 89;
const SKIP_BASE: i32 = 2_254_359;
const SKIP_CODE: i16 = 4; // xetex_format.h:674
const TEMP_HEAD: i32 = 4_999_999 - 3;
const TEX_INFINITY: i32 = 0x7FFF_FFFF;
const WHATSIT_NODE: u16 = 8; // xetex_format.h:388
const SEMI_SIMPLE_GROUP: u8 = 14;
const MATH_SHIFT_GROUP: u8 = 15;
const MATH_LEFT_GROUP: u8 = 16;
const DIMEN_VAL: i16 = 1;
const MU_VAL: i16 = 3;
const CHAR_CLASS_LIMIT: i32 = 4096;
const INTER_CHAR_VAL: i16 = 6;
const UNDEFINED_PRIMITIVE: i32 = 0;
const INTER_CHAR_TEXT: u16 = 17; // xetex-constants.h:216
const BACKED_UP_CHAR: u16 = 4;
const TOKEN_LIST_INPUT: u16 = 0;
const INT_PAR_XETEX_INTER_CHAR_TOKENS: i32 = 73; // xetex_format.h:108

// ---- moved verbatim from scan.rs (workstream 02) ----

// translated from xetex-xetex0.c:10390 (align_error)
pub(crate) fn align_error_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.align_state.abs() > 2 {
        crate::output::error_here_with_diagnostic_in(st, c"Misplaced ".as_ptr());
        crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        if st.scanner.cur_tok == TAB_TOKEN + 38 {
            st.errors.help_ptr = 6;
            st.errors.help_line[5] =
                c"I can't figure out why you would want to use a tab mark".as_ptr();
            st.errors.help_line[4] = c"here. If you just want an ampersand, the remedy is".as_ptr();
            st.errors.help_line[3] =
                c"simple: Just type `I\\&' now. But if some right brace".as_ptr();
            st.errors.help_line[2] =
                c"up above has ended a previous alignment prematurely,".as_ptr();
            st.errors.help_line[1] =
                c"you're probably due for more error messages, and you".as_ptr();
            st.errors.help_line[0] =
                c"might try typing `S' now just to see what is salvageable.".as_ptr();
        } else {
            st.errors.help_ptr = 5;
            st.errors.help_line[4] =
                c"I can't figure out why you would want to use a tab mark".as_ptr();
            st.errors.help_line[3] =
                c"or \\cr or \\span just now. If something like a right brace".as_ptr();
            st.errors.help_line[2] =
                c"up above has ended a previous alignment prematurely,".as_ptr();
            st.errors.help_line[1] =
                c"you're probably due for more error messages, and you".as_ptr();
            st.errors.help_line[0] =
                c"might try typing `S' now just to see what is salvageable.".as_ptr();
        }
        mc_propagate_error(crate::errors::error_in(st))?;
    } else {
        crate::input::back_input_in(st)?;
        if st.scanner.align_state < 0 {
            crate::output::error_here_with_diagnostic_in(st, c"Missing { inserted".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.scanner.align_state += 1;
            st.scanner.cur_tok = LEFT_BRACE_TOKEN + 123;
        } else {
            crate::output::error_here_with_diagnostic_in(st, c"Missing } inserted".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.scanner.align_state -= 1;
            st.scanner.cur_tok = RIGHT_BRACE_TOKEN + 125;
        }
        st.errors.help_ptr = 3;
        st.errors.help_line[2] = c"I've put in what seems to be necessary to fix".as_ptr();
        st.errors.help_line[1] = c"the current column of the current alignment.".as_ptr();
        st.errors.help_line[0] = c"Try to go on, since this might almost work.".as_ptr();
        mc_propagate_error(crate::input::ins_error_in(st)?)?;
    }
    Ok(())
}
// translated from xetex-xetex0.c:10443 (no_align_error)
pub(crate) fn no_align_error_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    crate::output::error_here_with_diagnostic_in(st, c"Misplaced ".as_ptr());
    crate::output::print_esc_cstr_in(st, c"noalign".as_ptr());
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 2;
    st.errors.help_line[1] = c"I expect to see \\noalign only after the \\cr of".as_ptr();
    st.errors.help_line[0] = c"an alignment. Proceed, and I'll ignore this case.".as_ptr();
    mc_propagate_error(crate::errors::error_in(st))
}
// translated from xetex-xetex0.c:10457 (omit_error)
pub(crate) fn omit_error_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    crate::output::error_here_with_diagnostic_in(st, c"Misplaced ".as_ptr());
    crate::output::print_esc_cstr_in(st, c"omit".as_ptr());
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 2;
    st.errors.help_line[1] = c"I expect to see \\omit only after tab marks or the \\cr of".as_ptr();
    st.errors.help_line[0] = c"an alignment. Proceed, and I'll ignore this case.".as_ptr();
    mc_propagate_error(crate::errors::error_in(st))
}
// translated from xetex-xetex0.c:8074 (fin_align)
pub(crate) fn fin_align_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    use crate::globals::{
        cur_list_aux_b32s0, cur_list_aux_b32s1, cur_list_e_tex_aux, cur_list_head, cur_list_mode,
        cur_list_mode_line, cur_list_tail, set_cur_list_aux_b32s0, set_cur_list_aux_b32s1,
        set_cur_list_tail,
    };
    use crate::mem::{
        delete_glue_ref, flush_list, free_node, new_glue, new_null_box, new_param_glue,
        new_penalty, pop_nest_in, ALLOC_FAILED, TEX_NULL,
    };
    use crate::packaging::flush_node_list;
    use crate::scan::NodeRecycleAbort;

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

    if st.save_stack.cur_group != ALIGN_GROUP {
        return Err(MainControlAbort::Confusion(c"align1"));
    }
    crate::save::unsave_in(st)?;
    if st.save_stack.cur_group != ALIGN_GROUP {
        return Err(MainControlAbort::Confusion(c"align0"));
    }
    crate::save::unsave_in(st)?;

    let o: i32 = if st.input.nest[(st.input.nest_ptr - 1) as usize].mode == MMODE {
        st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT)
    } else {
        0
    };

    // §830: walk preamble, compute column widths, handle span nodes
    let mut q = st.mem.b32s1(st.mem.b32s1(ALIGN_HEAD));
    loop {
        let fl3 = st.mem.b32s1(q + 3);
        flush_list(&mut st.mem, fl3);
        let fl2 = st.mem.b32s1(q + 2);
        flush_list(&mut st.mem, fl2);
        let p = st.mem.b32s1(st.mem.b32s1(q));
        // §831: if NULL_FLAG width, clear the tab_skip glue leader
        if st.mem.b32s1(q + 1) == NULL_FLAG {
            st.mem.set_b32s1(q + 1, 0);
            let r = st.mem.b32s1(q);
            let s = st.mem.b32s0(r + 1);
            if s != 0 {
                st.mem.set_b32s1(0, st.mem.b32s1(0) + 1); // GLUE_SPEC_ref_count(0)++
                delete_glue_ref(&mut st.mem, s);
                st.mem.set_b32s0(r + 1, 0);
            }
        }
        // §832: handle span nodes — distribute width across spanned columns
        if st.mem.b32s0(q) != END_SPAN {
            let t = st.mem.b32s1(q + 1) + st.mem.b32s1(st.mem.b32s0(st.mem.b32s1(q) + 1) + 1);
            let mut r = st.mem.b32s0(q);
            let mut s = END_SPAN;
            st.mem.set_b32s0(s, p);
            let mut n: i32 = 1;
            loop {
                st.mem.set_b32s1(r + 1, st.mem.b32s1(r + 1) - t);
                let u = st.mem.b32s0(r);
                while st.mem.b32s1(r) > n {
                    s = st.mem.b32s0(s);
                    n = st.mem.b32s1(st.mem.b32s0(s)) + 1;
                }
                if st.mem.b32s1(r) < n {
                    st.mem.set_b32s0(r, st.mem.b32s0(s));
                    st.mem.set_b32s0(s, r);
                    st.mem.set_b32s1(r, st.mem.b32s1(r) - 1);
                    s = r;
                } else {
                    if st.mem.b32s1(r + 1) > st.mem.b32s1(st.mem.b32s0(s) + 1) {
                        st.mem.set_b32s1(st.mem.b32s0(s) + 1, st.mem.b32s1(r + 1));
                    }
                    free_node(&mut st.mem, r, SPAN_NODE_SIZE);
                }
                r = u;
                if r == END_SPAN {
                    break;
                }
            }
        }
        // Clear the preamble node fields (convert to UNSET_NODE)
        st.mem.set_b16s1(q, UNSET_NODE);
        st.mem.set_b16s0(q, 0);
        st.mem.set_b32s1(q + 3, 0);
        st.mem.set_b32s1(q + 2, 0);
        st.mem.set_b16s0(q + 5, NORMAL);
        st.mem.set_b16s1(q + 5, NORMAL);
        st.mem.set_b32s1(q + 6, 0);
        st.mem.set_b32s1(q + 4, 0);
        q = p;
        if q == TEX_NULL {
            break;
        }
    }

    // §833: pop the spec from save_stack and pack the whole alignment
    st.save_stack.save_ptr -= 2;
    st.scanner.pack_begin_line = -cur_list_mode_line();
    let p = if cur_list_mode() == INNER_VMODE {
        let rule_save = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE);
        st.eqtb.set_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE, 0);
        let head = st.mem.b32s1(ALIGN_HEAD);
        let w = st.save_stack.b32s1(st.save_stack.save_ptr + 1);
        let m = st.save_stack.b32s1(st.save_stack.save_ptr) as i16;
        let pp = crate::packaging::hpack_emit_in(st, head, w, m)?;
        st.eqtb
            .set_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE, rule_save);
        pp
    } else {
        // vmode: copy widths to depth, pack, restore
        let mut qq = st.mem.b32s1(st.mem.b32s1(ALIGN_HEAD));
        loop {
            st.mem.set_b32s1(qq + 3, st.mem.b32s1(qq + 1));
            st.mem.set_b32s1(qq + 1, 0);
            qq = st.mem.b32s1(st.mem.b32s1(qq));
            if qq == TEX_NULL {
                break;
            }
        }
        let head = st.mem.b32s1(ALIGN_HEAD);
        let w = st.save_stack.b32s1(st.save_stack.save_ptr + 1);
        let m = st.save_stack.b32s1(st.save_stack.save_ptr) as i16;
        let pp = crate::packaging::vpackage_emit_in(st, head, w, m, MAX_HALFWORD)?;
        qq = st.mem.b32s1(st.mem.b32s1(ALIGN_HEAD));
        loop {
            st.mem.set_b32s1(qq + 1, st.mem.b32s1(qq + 3));
            st.mem.set_b32s1(qq + 3, 0);
            qq = st.mem.b32s1(st.mem.b32s1(qq));
            if qq == TEX_NULL {
                break;
            }
        }
        pp
    };
    st.scanner.pack_begin_line = 0;

    // §836–840: walk the row list and resolve each UNSET_NODE
    q = st.mem.b32s1(cur_list_head());
    let mut s = cur_list_head();
    while q != TEX_NULL {
        if q < st.mem.hi_mem_min {
            // !is_char_node
            if st.mem.b16s1(q) == UNSET_NODE {
                // §836: set this row/column to its final type and dimensions
                if cur_list_mode() == INNER_VMODE {
                    st.mem.set_b16s1(q, HLIST_NODE);
                    st.mem.set_b32s1(q + 1, p.width(&st.mem).0);
                    if st.input.nest[(st.input.nest_ptr - 1) as usize].mode == MMODE {
                        st.mem.set_b16s0(q, DLIST);
                    }
                } else {
                    st.mem.set_b16s1(q, VLIST_NODE);
                    st.mem.set_b32s1(q + 3, p.height(&st.mem).0);
                }
                st.mem.set_b16s0(q + 5, p.glue_order(&st.mem));
                st.mem.set_b16s1(q + 5, p.glue_sign(&st.mem));
                st.mem.set_gr(q + 6, p.glue_set(&st.mem));
                st.mem.set_b32s1(q + 4, o);

                // Walk the column UNSET_NODEs inside this row
                let mut r = st.mem.b32s1(st.mem.b32s1(q + 5));
                let mut ss = st.mem.b32s1(p.list_ptr(&st.mem).0);
                loop {
                    // §837: for each column unset node r, distribute glue
                    let n = st.mem.b16s0(r);
                    let t_base = st.mem.b32s1(ss + 1);
                    let mut t = t_base;
                    let mut u = HOLD_HEAD;
                    st.mem.set_b16s0(r, 0);
                    // Insert inter-column glue for spanned columns
                    let mut span_count = n;
                    while span_count > 0 {
                        span_count -= 1;
                        ss = st.mem.b32s1(ss);
                        let v = st.mem.b32s0(ss + 1);
                        let v_p = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(v))
                            .expect("non-null glue spec");
                        let g = new_glue(&mut st.mem, v);
                        if g == ALLOC_FAILED {
                            return Err(NodeRecycleAbort::Overflow.into());
                        }
                        st.mem.set_b32s1(u, g);
                        u = st.mem.b32s1(u);
                        st.mem.set_b16s0(u, GLUE_PAR_TAB_SKIP);
                        t += v_p.width(&st.mem).0;
                        if p.glue_sign(&st.mem) == STRETCHING {
                            if v_p.stretch_order(&st.mem) == p.glue_order(&st.mem) {
                                t += crate::scaled_math::tex_round(
                                    p.glue_set(&st.mem) * v_p.stretch(&st.mem).0 as f64,
                                );
                            }
                        } else if p.glue_sign(&st.mem) == SHRINKING
                            && v_p.shrink_order(&st.mem) == p.glue_order(&st.mem)
                        {
                            t -= crate::scaled_math::tex_round(
                                p.glue_set(&st.mem) * v_p.shrink(&st.mem).0 as f64,
                            );
                        }
                        ss = st.mem.b32s1(ss);
                        let nb = new_null_box(&mut st.mem);
                        if nb == ALLOC_FAILED {
                            return Err(NodeRecycleAbort::Overflow.into());
                        }
                        st.mem.set_b32s1(u, nb);
                        u = st.mem.b32s1(u);
                        t += st.mem.b32s1(ss + 1);
                        if cur_list_mode() == INNER_VMODE {
                            st.mem.set_b32s1(u + 1, st.mem.b32s1(ss + 1));
                        } else {
                            st.mem.set_b16s1(u, VLIST_NODE);
                            st.mem.set_b32s1(u + 3, st.mem.b32s1(ss + 1));
                        }
                    }
                    // §839/840: set glue ratio for r and finalize node type
                    if cur_list_mode() == INNER_VMODE {
                        // §839: hbox column
                        st.mem.set_b32s1(r + 3, st.mem.b32s1(q + 3));
                        st.mem.set_b32s1(r + 2, st.mem.b32s1(q + 2));
                        if t == st.mem.b32s1(r + 1) {
                            st.mem.set_b16s1(r + 5, NORMAL);
                            st.mem.set_b16s0(r + 5, NORMAL);
                            st.mem.set_gr(r + 6, 0.0);
                        } else if t > st.mem.b32s1(r + 1) {
                            st.mem.set_b16s1(r + 5, STRETCHING);
                            if st.mem.b32s1(r + 6) == 0 {
                                st.mem.set_gr(r + 6, 0.0);
                            } else {
                                st.mem.set_gr(
                                    r + 6,
                                    (t - st.mem.b32s1(r + 1)) as f64 / st.mem.b32s1(r + 6) as f64,
                                );
                            }
                        } else {
                            st.mem.set_b16s0(r + 5, st.mem.b16s1(r + 5));
                            st.mem.set_b16s1(r + 5, SHRINKING);
                            if st.mem.b32s1(r + 4) == 0 {
                                st.mem.set_gr(r + 6, 0.0);
                            } else if st.mem.b16s0(r + 5) == NORMAL
                                && st.mem.b32s1(r + 1) - t > st.mem.b32s1(r + 4)
                            {
                                st.mem.set_gr(r + 6, 1.0);
                            } else {
                                st.mem.set_gr(
                                    r + 6,
                                    (st.mem.b32s1(r + 1) - t) as f64 / st.mem.b32s1(r + 4) as f64,
                                );
                            }
                        }
                        st.mem.set_b32s1(r + 1, t_base);
                        st.mem.set_b16s1(r, HLIST_NODE);
                    } else {
                        // §840: vbox column
                        st.mem.set_b32s1(r + 1, st.mem.b32s1(q + 1));
                        if t == st.mem.b32s1(r + 3) {
                            st.mem.set_b16s1(r + 5, NORMAL);
                            st.mem.set_b16s0(r + 5, NORMAL);
                            st.mem.set_gr(r + 6, 0.0);
                        } else if t > st.mem.b32s1(r + 3) {
                            st.mem.set_b16s1(r + 5, STRETCHING);
                            if st.mem.b32s1(r + 6) == 0 {
                                st.mem.set_gr(r + 6, 0.0);
                            } else {
                                st.mem.set_gr(
                                    r + 6,
                                    (t - st.mem.b32s1(r + 3)) as f64 / st.mem.b32s1(r + 6) as f64,
                                );
                            }
                        } else {
                            st.mem.set_b16s0(r + 5, st.mem.b16s1(r + 5));
                            st.mem.set_b16s1(r + 5, SHRINKING);
                            if st.mem.b32s1(r + 4) == 0 {
                                st.mem.set_gr(r + 6, 0.0);
                            } else if st.mem.b16s0(r + 5) == NORMAL
                                && st.mem.b32s1(r + 3) - t > st.mem.b32s1(r + 4)
                            {
                                st.mem.set_gr(r + 6, 1.0);
                            } else {
                                st.mem.set_gr(
                                    r + 6,
                                    (st.mem.b32s1(r + 3) - t) as f64 / st.mem.b32s1(r + 4) as f64,
                                );
                            }
                        }
                        st.mem.set_b32s1(r + 3, t_base);
                        st.mem.set_b16s1(r, VLIST_NODE);
                    }
                    st.mem.set_b32s1(r + 4, 0);
                    // Link any inserted inter-column glue nodes
                    if u != HOLD_HEAD {
                        st.mem.set_b32s1(u, st.mem.b32s1(r));
                        st.mem.set_b32s1(r, st.mem.b32s1(HOLD_HEAD));
                        r = u;
                    }
                    r = st.mem.b32s1(st.mem.b32s1(r));
                    ss = st.mem.b32s1(st.mem.b32s1(ss));
                    if r == TEX_NULL {
                        break;
                    }
                }
            } else if st.mem.b16s1(q) == RULE_NODE {
                // §835: rule node — fill in any NULL_FLAG dimensions from the packed box
                if st.mem.b32s1(q + 1) == NULL_FLAG {
                    st.mem.set_b32s1(q + 1, p.width(&st.mem).0);
                }
                if st.mem.b32s1(q + 3) == NULL_FLAG {
                    st.mem.set_b32s1(q + 3, p.height(&st.mem).0);
                }
                if st.mem.b32s1(q + 2) == NULL_FLAG {
                    st.mem.set_b32s1(q + 2, p.depth(&st.mem).0);
                }
                if o != 0 {
                    let r = st.mem.b32s1(q);
                    st.mem.set_b32s1(q, TEX_NULL);
                    let qq2 = crate::packaging::hpack_emit_in(st, q, 0, ADDITIONAL)?
                        .idx()
                        .0;
                    st.mem.set_b32s1(qq2 + 4, o);
                    st.mem.set_b32s1(qq2, r);
                    st.mem.set_b32s1(s, qq2);
                    q = qq2;
                }
            }
        } // end !is_char_node
        s = q;
        q = st.mem.b32s1(q);
    }

    flush_node_list(&mut st.mem, p.idx().0).map_err(MainControlAbort::Confusion)?;
    pop_alignment_in(st);

    // Save aux and pop nest to get back to the outer list
    let aux_save_s0 = cur_list_aux_b32s0();
    let aux_save_s1 = cur_list_aux_b32s1();
    let pp = st.mem.b32s1(cur_list_head());
    let qq = cur_list_tail();
    pop_nest_in(&mut st.mem, &mut st.input);

    if cur_list_mode() == MMODE {
        // §1241: display math alignment — add penalties + display glue
        do_assignments_in(st)?;
        if st.scanner.cur_cmd != MATH_SHIFT {
            crate::output::error_here_with_diagnostic_in(st, c"Missing $$ inserted".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 2;
            st.errors.help_line[1] =
                c"Displays can use special alignments (like \\eqalignno)".as_ptr();
            st.errors.help_line[0] =
                c"only if nothing but the alignment itself is between $$'s.".as_ptr();
            mc_propagate_error(crate::input::back_error_in(st)?)?;
        } else {
            // §1232: check for second $$
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
        let etex_aux = cur_list_e_tex_aux();
        flush_node_list(&mut st.mem, etex_aux).map_err(MainControlAbort::Confusion)?;
        pop_nest_in(&mut st.mem, &mut st.input);
        // Insert pre-display penalty and above-display skip
        let pre_pen = st.eqtb.b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_PENALTY);
        let pen = new_penalty(&mut st.mem, pre_pen);
        if pen == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(cur_list_tail(), pen);
        set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
        let ag = new_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_ABOVE_DISPLAY);
        if ag == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(cur_list_tail(), ag);
        set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
        // Attach alignment rows
        st.mem.set_b32s1(cur_list_tail(), pp);
        if pp != TEX_NULL {
            set_cur_list_tail(qq);
        }
        // Insert post-display penalty and below-display skip
        let post_pen = st.eqtb.b32s1(INT_BASE + INT_PAR_POST_DISPLAY_PENALTY);
        let pen2 = new_penalty(&mut st.mem, post_pen);
        if pen2 == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(cur_list_tail(), pen2);
        set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
        let bg = new_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_BELOW_DISPLAY);
        if bg == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(cur_list_tail(), bg);
        set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
        // Only prevdepth (aux.b32.s1) is restored for MMODE (C: cur_list.aux.b32.s1 = aux_save.b32.s1)
        set_cur_list_aux_b32s1(aux_save_s1);
        crate::math::resume_after_display_in(st)?;
    } else {
        // Regular vmode or hmode alignment — restore full aux
        set_cur_list_aux_b32s0(aux_save_s0);
        set_cur_list_aux_b32s1(aux_save_s1);
        st.mem.set_b32s1(cur_list_tail(), pp);
        if pp != TEX_NULL {
            set_cur_list_tail(qq);
        }
        if cur_list_mode() == VMODE {
            crate::pagebuilder::build_page_in(st)?;
        }
    }
    Ok(())
}
// translated from xetex-xetex0.c:8785 (align_peek)
pub(crate) fn align_peek_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    const NO_ALIGN: u8 = 34; // xetex_format.h:272
    const NO_ALIGN_GROUP: u8 = 7; // xetex-constants.h:159
    const CAR_RET: u8 = 5; // xetex_format.h:235
    const CR_CR_CODE: i32 = 1_114_115; // xetex-constants.h:681

    loop {
        st.scanner.align_state = 1_000_000;
        loop {
            crate::scan::get_x_or_protected_in(st)?;
            if st.scanner.cur_cmd != SPACER {
                break;
            }
        }
        if st.scanner.cur_cmd == NO_ALIGN {
            crate::scan::scan_left_brace_in(st)?;
            if let Some(a) = crate::save::new_save_level(&mut st.save_stack, NO_ALIGN_GROUP, st.mem.line) {
                return Err(MainControlAbort::Save(a));
            }
            if crate::globals::cur_list_mode() == -1 {
                if let Some(a) = normal_paragraph(
                    &mut st.eqtb,
                    &mut st.save_stack,
                    &mut st.mem,
                    &mut st.scanner,
                ) {
                    return Err(MainControlAbort::Save(a));
                }
            }
            return Ok(());
        } else if st.scanner.cur_cmd == RIGHT_BRACE {
            fin_align_in(st)?;
            return Ok(());
        } else if st.scanner.cur_cmd == CAR_RET && st.scanner.cur_chr == CR_CR_CODE {
            continue;
        } else {
            init_row_in(st)?;
            init_col_in(st)?;
            return Ok(());
        }
    }
}
pub(crate) fn get_preamble_token_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const TAB_MARK: u8 = 4; // xetex_format.h:234
    const SPAN_CODE_GPT: i32 = 1_114_113; // xetex_format.h:679
    const GLUE_REF_GPT: u16 = 119; // xetex_format.h:361
    const INT_PAR_GLOBAL_DEFS: i32 = 43; // xetex_format.h:78
    const GLUE_BASE_TAB_SKIP: i32 = GLUE_BASE + 11;

    'restart: loop {
        crate::scan::get_token_in(st)?;
        while st.scanner.cur_chr == SPAN_CODE_GPT && st.scanner.cur_cmd == TAB_MARK {
            crate::scan::get_token_in(st)?;
            if st.scanner.cur_cmd > MAX_COMMAND {
                crate::expand::expand_in(st)?;
                crate::scan::get_token_in(st)?;
            }
        }
        if st.scanner.cur_cmd == ENDV {
            // Interwoven alignment preamble: propagate as a Result instead of the
            // direct fatal_error longjmp under the &mut EngineState borrow, mirroring
            // the sibling sites at fin_align (MainControlAbort::Input -> InputAbort::
            // Fatal -> EngineAbort::Fatal; into_history prints "Emergency stop" + msg
            // and returns 3, same as the old fatal_error) — `01d` §5.6.
            return Err(MainControlAbort::Input(crate::input::InputAbort::Fatal(
                c"(interwoven alignment preambles are not allowed)".as_ptr(),
            )));
        }
        if st.scanner.cur_cmd == ASSIGN_GLUE && st.scanner.cur_chr == GLUE_BASE_TAB_SKIP {
            crate::scan::scan_optional_equals_in(st)?;
            crate::scan::scan_glue_in(st, GLUE_VAL)?;
            if st.eqtb.b32s1(INT_BASE + INT_PAR_GLOBAL_DEFS) > 0 {
                let __cv20 = st.scanner.cur_val;
                if let Some(a) = crate::save::geq_define(
                    &mut st.eqtb,
                    &mut st.mem,
                    &mut st.scanner,
                    GLUE_BASE_TAB_SKIP,
                    GLUE_REF_GPT,
                    __cv20,
                ) {
                    return Err(MainControlAbort::Save(a));
                }
            } else {
                let __cv19 = st.scanner.cur_val;
                if let Some(a) = crate::save::eq_define(
                    &mut st.eqtb,
                    &mut st.save_stack,
                    &mut st.mem,
                    &mut st.scanner,
                    GLUE_BASE_TAB_SKIP,
                    GLUE_REF_GPT,
                    __cv19,
                ) {
                    return Err(MainControlAbort::Save(a));
                }
            }
            continue 'restart;
        }
        break;
    }
    Ok(())
}
pub(crate) fn init_span_in(
    st: &mut crate::state::EngineState,
    p: i32,
) -> Result<(), MainControlAbort> {
    const INNER_HMODE: i16 = -104;

    crate::mem::push_nest_in(st)?;
    if cur_list_mode() == INNER_HMODE {
        crate::globals::set_cur_list_aux_b32s0(1000);
    } else {
        crate::globals::set_cur_list_aux_b32s1(IGNORE_DEPTH);
        if let Some(a) = normal_paragraph(
            &mut st.eqtb,
            &mut st.save_stack,
            &mut st.mem,
            &mut st.scanner,
        ) {
            return Err(MainControlAbort::Save(a));
        }
    }
    st.scanner.cur_span = p;
    Ok(())
}
pub(crate) fn init_row_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    const ALIGN_HEAD_IR: i32 = 4_999_991;
    const GLUE_PAR_TAB_SKIP: u16 = 11 + 1; // GLUE_PAR__tab_skip + 1

    crate::mem::push_nest_in(st)?;
    let new_mode = -105 - cur_list_mode() as i32;
    crate::globals::set_cur_list_mode(new_mode as i16);
    if new_mode == -104 {
        crate::globals::set_cur_list_aux_b32s0(0);
    } else {
        crate::globals::set_cur_list_aux_b32s1(0);
    }
    let align_head_link = st.mem.b32s1(ALIGN_HEAD_IR);
    let glue_spec = st.mem.b32s0(align_head_link + 1);
    let glue_node = crate::mem::new_glue(&mut st.mem, glue_spec);
    if glue_node == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    st.mem.set_b32s1(crate::globals::cur_list_tail(), glue_node);
    crate::globals::set_cur_list_tail(st.mem.b32s1(crate::globals::cur_list_tail()));
    st.mem
        .set_b16s0(crate::globals::cur_list_tail(), GLUE_PAR_TAB_SKIP);
    st.scanner.cur_align = st.mem.b32s1(align_head_link);
    st.scanner.cur_tail = st.scanner.cur_head;
    st.scanner.cur_pre_tail = st.scanner.cur_pre_head;
    init_span_in(st, st.scanner.cur_align)?;
    Ok(())
}
pub(crate) fn init_col_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    const OMIT_CMD: u8 = 63; // xetex_format.h:301
    const U_TEMPLATE: u16 = 1; // xetex-constants.h:200

    st.mem.set_b32s0(st.scanner.cur_align + 5, st.scanner.cur_cmd as i32);
    if st.scanner.cur_cmd == OMIT_CMD {
        st.scanner.align_state = 0;
    } else {
        crate::input::back_input_in(st)?;
        crate::input::begin_token_list_in(st, st.mem.b32s1(st.scanner.cur_align + 3), U_TEMPLATE)?;
    }
    Ok(())
}
pub(crate) fn push_alignment_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const ALIGN_HEAD: i32 = 4_999_991; // MEM_TOP - 8
    const ALIGN_STACK_NODE_SIZE: i32 = 6; // xetex-constants.h:129

    let p = crate::scan::get_node_scc(st, ALIGN_STACK_NODE_SIZE)?;
    st.mem.set_b32s1(p, st.scanner.align_ptr);
    st.mem.set_b32s0(p, st.scanner.cur_align);
    st.mem.set_b32s0(p + 1, st.mem.b32s1(ALIGN_HEAD));
    st.mem.set_b32s1(p + 1, st.scanner.cur_span);
    st.mem.set_b32s1(p + 2, st.scanner.cur_loop);
    st.mem.set_b32s1(p + 3, st.scanner.align_state);
    st.mem.set_b32s0(p + 4, st.scanner.cur_head);
    st.mem.set_b32s1(p + 4, st.scanner.cur_tail);
    st.mem.set_b32s0(p + 5, st.scanner.cur_pre_head);
    st.mem.set_b32s1(p + 5, st.scanner.cur_pre_tail);
    st.scanner.align_ptr = p;
    st.scanner.cur_head = crate::scan::get_avail_scc(st)?;
    st.scanner.cur_pre_head = crate::scan::get_avail_scc(st)?;
    Ok(())
}
pub(crate) fn pop_alignment_in(st: &mut crate::state::EngineState) {
    const ALIGN_HEAD: i32 = 4_999_991;
    const ALIGN_STACK_NODE_SIZE: i32 = 6;

    // free_avail(cur_head): mem[cur_head].b32.s1 = avail; avail = cur_head
    st.mem.set_b32s1(st.scanner.cur_head, st.mem.avail);
    st.mem.avail = st.scanner.cur_head;
    // free_avail(cur_pre_head)
    st.mem.set_b32s1(st.scanner.cur_pre_head, st.mem.avail);
    st.mem.avail = st.scanner.cur_pre_head;

    let p = st.scanner.align_ptr;
    st.scanner.cur_tail = st.mem.b32s1(p + 4);
    st.scanner.cur_head = st.mem.b32s0(p + 4);
    st.scanner.cur_pre_tail = st.mem.b32s1(p + 5);
    st.scanner.cur_pre_head = st.mem.b32s0(p + 5);
    st.scanner.align_state = st.mem.b32s1(p + 3);
    st.scanner.cur_loop = st.mem.b32s1(p + 2);
    st.scanner.cur_span = st.mem.b32s1(p + 1);
    st.mem.set_b32s1(ALIGN_HEAD, st.mem.b32s0(p + 1));
    st.scanner.cur_align = st.mem.b32s0(p);
    st.scanner.align_ptr = st.mem.b32s1(p);
    crate::mem::free_node(&mut st.mem, p, ALIGN_STACK_NODE_SIZE);
}
// translated from xetex-xetex0.c:7859 (fin_col)
pub(crate) fn fin_col_in(st: &mut crate::state::EngineState) -> Result<bool, MainControlAbort> {
    use crate::globals::{cur_list_head, cur_list_mode, cur_list_tail, set_cur_list_tail};
    use crate::mem::{get_avail, new_glue, new_null_box, ALLOC_FAILED, TEX_NULL};

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

    if st.scanner.cur_align == TEX_NULL {
        return Err(MainControlAbort::Confusion(c"endv"));
    }
    let q = st.mem.b32s1(st.scanner.cur_align);
    if q == TEX_NULL {
        return Err(MainControlAbort::Confusion(c"endv"));
    }
    if st.scanner.align_state < 500_000 {
        return Err(MainControlAbort::Input(crate::input::InputAbort::Fatal(
            c"(interwoven alignment preambles are not allowed)".as_ptr(),
        )));
    }
    let mut p = st.mem.b32s1(q);
    if p == TEX_NULL && st.mem.b32s0(st.scanner.cur_align + 5) < CR_CODE {
        if st.scanner.cur_loop != TEX_NULL {
            // §822: extend preamble by duplicating the loop template
            let nb = new_null_box(&mut st.mem);
            if nb == ALLOC_FAILED {
                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(q, nb);
            p = st.mem.b32s1(q);
            st.mem.set_b32s0(p, END_SPAN);
            st.mem.set_b32s1(p + 1, NULL_FLAG);
            st.scanner.cur_loop = st.mem.b32s1(st.scanner.cur_loop);
            // copy u-template from cur_loop+3 into HOLD_HEAD list → p+3
            let mut wq = HOLD_HEAD;
            let mut r = st.mem.b32s1(st.scanner.cur_loop + 3);
            while r != TEX_NULL {
                let av = get_avail(&mut st.mem);
                if av == ALLOC_FAILED {
                    return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                }
                st.mem.set_b32s1(wq, av);
                wq = st.mem.b32s1(wq);
                st.mem.set_b32s0(wq, st.mem.b32s0(r));
                r = st.mem.b32s1(r);
            }
            st.mem.set_b32s1(wq, TEX_NULL);
            st.mem.set_b32s1(p + 3, st.mem.b32s1(HOLD_HEAD));
            // copy v-template from cur_loop+2 into HOLD_HEAD list → p+2
            wq = HOLD_HEAD;
            r = st.mem.b32s1(st.scanner.cur_loop + 2);
            while r != TEX_NULL {
                let av = get_avail(&mut st.mem);
                if av == ALLOC_FAILED {
                    return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                }
                st.mem.set_b32s1(wq, av);
                wq = st.mem.b32s1(wq);
                st.mem.set_b32s0(wq, st.mem.b32s0(r));
                r = st.mem.b32s1(r);
            }
            st.mem.set_b32s1(wq, TEX_NULL);
            st.mem.set_b32s1(p + 2, st.mem.b32s1(HOLD_HEAD));
            st.scanner.cur_loop = st.mem.b32s1(st.scanner.cur_loop);
            let gv = st.mem.b32s0(st.scanner.cur_loop + 1);
            let g = new_glue(&mut st.mem, gv);
            if g == ALLOC_FAILED {
                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(p, g);
            let pp = st.mem.b32s1(p);
            st.mem.set_b16s0(pp, GLUE_PAR_TAB_SKIP);
        } else {
            crate::output::error_here_with_diagnostic_in(
                st,
                c"Extra alignment tab has been changed to ".as_ptr(),
            );
            crate::output::print_esc_cstr_in(st, c"cr".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 3;
            st.errors.help_line[2] =
                c"You have given more \\span or & marks than there were".as_ptr();
            st.errors.help_line[1] =
                c"in the preamble to the \\halign or \\valign now in progress.".as_ptr();
            st.errors.help_line[0] =
                c"So I'll assume that you meant to type \\cr instead.".as_ptr();
            st.mem.set_b32s0(st.scanner.cur_align + 5, CR_CODE);
            mc_propagate_error(crate::errors::error_in(st))?;
        }
    }
    if st.mem.b32s0(st.scanner.cur_align + 5) != SPAN_CODE {
        crate::save::unsave_in(st)?;
        if let Some(a) = crate::save::new_save_level(&mut st.save_stack, ALIGN_GROUP, st.mem.line) {
            return Err(MainControlAbort::Save(a));
        }
        let (u, w) = if cur_list_mode() == INNER_HMODE {
            st.scanner.adjust_tail = st.scanner.cur_tail;
            st.scanner.pre_adjust_tail = st.scanner.cur_pre_tail;
            let head = st.mem.b32s1(cur_list_head());
            let u = crate::packaging::hpack_emit_in(st, head, 0, ADDITIONAL)?
                .idx()
                .0;
            let w = st.mem.b32s1(u + 1);
            st.scanner.cur_tail = st.scanner.adjust_tail;
            st.scanner.adjust_tail = TEX_NULL;
            st.scanner.cur_pre_tail = st.scanner.pre_adjust_tail;
            st.scanner.pre_adjust_tail = TEX_NULL;
            (u, w)
        } else {
            let head = st.mem.b32s1(cur_list_head());
            let u = crate::packaging::vpackage_emit_in(st, head, 0, ADDITIONAL, 0)?
                .idx()
                .0;
            let w = st.mem.b32s1(u + 3);
            (u, w)
        };
        let mut n: i32 = 0;
        if st.scanner.cur_span != st.scanner.cur_align {
            // §827: compare accumulated span width with recorded widths
            let mut sq = st.scanner.cur_span;
            loop {
                n += 1;
                sq = st.mem.b32s1(st.mem.b32s1(sq));
                if sq == st.scanner.cur_align {
                    break;
                }
            }
            if n > u16::MAX as i32 {
                return Err(MainControlAbort::Confusion(c"too many spans"));
            }
            sq = st.scanner.cur_span;
            while st.mem.b32s1(st.mem.b32s0(sq)) < n {
                sq = st.mem.b32s0(sq);
            }
            if st.mem.b32s1(st.mem.b32s0(sq)) > n {
                let s = crate::mem::get_node(&mut st.mem, SPAN_NODE_SIZE);
                if s == ALLOC_FAILED {
                    return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                }
                st.mem.set_b32s0(s, st.mem.b32s0(sq));
                st.mem.set_b32s1(s, n);
                st.mem.set_b32s0(sq, s);
                st.mem.set_b32s1(s + 1, w);
            } else if st.mem.b32s1(st.mem.b32s0(sq) + 1) < w {
                st.mem.set_b32s1(st.mem.b32s0(sq) + 1, w);
            }
        } else if w > st.mem.b32s1(st.scanner.cur_align + 1) {
            st.mem.set_b32s1(st.scanner.cur_align + 1, w);
        }
        st.mem.set_b16s1(u, UNSET_NODE);
        st.mem.set_b16s0(u, n as u16);
        let o_str: usize = if st.scanner.total_stretch[FILLL] != 0 {
            FILLL
        } else if st.scanner.total_stretch[FILL] != 0 {
            FILL
        } else if st.scanner.total_stretch[FIL] != 0 {
            FIL
        } else {
            0
        };
        st.mem.set_b16s0(u + 5, o_str as u16);
        st.mem.set_b32s1(u + 6, st.scanner.total_stretch[o_str]);
        let o_shr: usize = if st.scanner.total_shrink[FILLL] != 0 {
            FILLL
        } else if st.scanner.total_shrink[FILL] != 0 {
            FILL
        } else if st.scanner.total_shrink[FIL] != 0 {
            FIL
        } else {
            0
        };
        st.mem.set_b16s1(u + 5, o_shr as u16);
        st.mem.set_b32s1(u + 4, st.scanner.total_shrink[o_shr]);
        crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
        st.mem.set_b32s1(cur_list_tail(), u);
        set_cur_list_tail(u);
        let gv = st.mem.b32s0(st.mem.b32s1(st.scanner.cur_align) + 1);
        let g = new_glue(&mut st.mem, gv);
        if g == ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(cur_list_tail(), g);
        set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
        st.mem.set_b16s0(cur_list_tail(), GLUE_PAR_TAB_SKIP);
        if st.mem.b32s0(st.scanner.cur_align + 5) >= CR_CODE {
            return Ok(true);
        }
        init_span_in(st, p)?;
    }
    st.scanner.align_state = 1_000_000;
    loop {
        crate::scan::get_x_or_protected_in(st)?;
        if st.scanner.cur_cmd != SPACER {
            break;
        }
    }
    st.scanner.cur_align = p;
    init_col_in(st)?;
    Ok(false)
}
// translated from xetex-xetex0.c:8009 (fin_row)
pub(crate) fn fin_row_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    use crate::globals::{
        cur_list_head, cur_list_mode, cur_list_tail, set_cur_list_aux_b32s0, set_cur_list_tail,
    };
    use crate::mem::{pop_nest_in, TEX_NULL};

    const ADDITIONAL: i16 = 1;
    const UNSET_NODE: u16 = 13;
    const INNER_HMODE: i16 = -104;
    const EVERY_CR_TEXT: u16 = 14; // xetex-constants.h:213
    const LOCAL__EVERY_CR: i32 = 8; // xetex_format.h:180

    let p = if cur_list_mode() == INNER_HMODE {
        let head = st.mem.b32s1(cur_list_head());
        let u = crate::packaging::hpack_emit_in(st, head, 0, ADDITIONAL)?;
        pop_nest_in(&mut st.mem, &mut st.input);
        if st.scanner.cur_pre_head != st.scanner.cur_pre_tail {
            let v = st.mem.b32s1(st.scanner.cur_pre_head);
            st.mem.set_b32s1(cur_list_tail(), v);
            set_cur_list_tail(st.scanner.cur_pre_tail);
        }
        crate::packaging::append_to_vlist(&mut st.mem, &st.eqtb, u)?;
        if st.scanner.cur_head != st.scanner.cur_tail {
            let v = st.mem.b32s1(st.scanner.cur_head);
            st.mem.set_b32s1(cur_list_tail(), v);
            set_cur_list_tail(st.scanner.cur_tail);
        }
        // `u` is re-stamped as an UNSET_NODE below, so drop to the raw index here.
        u.idx().0
    } else {
        let head = st.mem.b32s1(cur_list_head());
        let u = crate::packaging::vpackage_emit_in(st, head, 0, ADDITIONAL, MAX_HALFWORD)?
            .idx()
            .0;
        pop_nest_in(&mut st.mem, &mut st.input);
        st.mem.set_b32s1(cur_list_tail(), u);
        set_cur_list_tail(u);
        set_cur_list_aux_b32s0(1000);
        u
    };
    st.mem.set_b16s1(p, UNSET_NODE);
    st.mem.set_b32s1(p + 6, 0);
    let every_cr = st.eqtb.b32s1(LOCAL_BASE + LOCAL__EVERY_CR);
    if every_cr != TEX_NULL {
        crate::input::begin_token_list_in(st, every_cr, EVERY_CR_TEXT)?;
    }
    align_peek_in(st)?;
    Ok(())
}
// translated from xetex-xetex0.c:7732 (init_align)
pub(crate) fn init_align_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    use crate::globals::{
        cur_list_aux_b32s1, cur_list_head, cur_list_mode, cur_list_tail,
        set_cur_list_aux_b32s1, set_cur_list_mode,
    };
    use crate::mem::{get_avail, new_null_box, new_param_glue, ALLOC_FAILED, TEX_NULL};
    use crate::scan::NodeRecycleAbort;

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

    let save_cs_ptr = st.scanner.cur_cs;
    push_alignment_in(st)?;
    st.scanner.align_state = -1_000_000;

    if cur_list_mode() == MMODE
        && (cur_list_tail() != cur_list_head() || cur_list_aux_b32s1() != TEX_NULL)
    {
        crate::output::error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
        crate::output::print_esc_cstr_in(st, c"halign".as_ptr());
        crate::output::print_cstr_in(st, c" inside $$'s".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 3;
        st.errors.help_line[2] = c"Displays can use special alignments (like \\eqalignno)".as_ptr();
        st.errors.help_line[1] =
            c"only if nothing but the alignment itself is between $$'s.".as_ptr();
        st.errors.help_line[0] =
            c"So I've deleted the formulas that preceded this alignment.".as_ptr();
        mc_propagate_error(crate::errors::error_in(st))?;
        crate::math::flush_math(&mut st.mem).map_err(MainControlAbort::Confusion)?;
    }

    crate::mem::push_nest_in(st)?;

    if cur_list_mode() == MMODE {
        set_cur_list_mode(-1);
        let aux = unsafe { st.input.nest[(st.input.nest_ptr - 2) as usize].aux.b32.s1 };
        set_cur_list_aux_b32s1(aux);
    } else if cur_list_mode() > 0 {
        set_cur_list_mode(-(cur_list_mode() as i32) as i16);
    }

    crate::scan::scan_spec_in(st, ALIGN_GROUP, false)?;
    st.mem.set_b32s1(ALIGN_HEAD, TEX_NULL);
    st.scanner.cur_align = ALIGN_HEAD;
    st.scanner.cur_loop = TEX_NULL;
    st.scanner.scanner_status = ALIGNING;
    st.scanner.warning_index = save_cs_ptr;
    st.scanner.align_state = -1_000_000;

    'outer: loop {
        let g = new_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_TAB_SKIP);
        if g == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        let ca = st.scanner.cur_align;
        st.mem.set_b32s1(ca, g);
        st.scanner.cur_align = st.mem.b32s1(ca);
        if st.scanner.cur_cmd == CAR_RET {
            break 'outer;
        }

        let mut p = HOLD_HEAD;
        st.mem.set_b32s1(p, TEX_NULL);

        // scan u-template up to first #
        loop {
            get_preamble_token_in(st)?;
            if st.scanner.cur_cmd == MAC_PARAM {
                break; // done1
            }
            if st.scanner.cur_cmd <= CAR_RET && st.scanner.cur_cmd >= TAB_MARK && st.scanner.align_state == -1_000_000
            {
                if p == HOLD_HEAD && st.scanner.cur_loop == TEX_NULL && st.scanner.cur_cmd == TAB_MARK {
                    st.scanner.cur_loop = st.scanner.cur_align;
                } else {
                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"Missing # inserted in alignment preamble".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 3;
                    st.errors.help_line[2] =
                        c"There should be exactly one # between &'s, when an".as_ptr();
                    st.errors.help_line[1] =
                        c"\\halign or \\valign is being set up. In this case you had".as_ptr();
                    st.errors.help_line[0] =
                        c"none, so I've put one in; maybe that will work.".as_ptr();
                    mc_propagate_error(crate::input::back_error_in(st)?)?;
                    break; // done1
                }
            } else if st.scanner.cur_cmd != SPACER || p != HOLD_HEAD {
                let av = get_avail(&mut st.mem);
                if av == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow.into());
                }
                st.mem.set_b32s1(p, av);
                p = st.mem.b32s1(p);
                st.mem.set_b32s0(p, st.scanner.cur_tok);
            }
        }

        // done1: set up the column preamble node
        let nb = new_null_box(&mut st.mem);
        if nb == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        let ca = st.scanner.cur_align;
        st.mem.set_b32s1(ca, nb);
        st.scanner.cur_align = st.mem.b32s1(ca);
        let ca = st.scanner.cur_align;
        st.mem.set_b32s0(ca, END_SPAN);
        st.mem.set_b32s1(ca + 1, NULL_FLAG);
        let hh = st.mem.b32s1(HOLD_HEAD);
        st.mem.set_b32s1(ca + 3, hh);
        p = HOLD_HEAD;
        st.mem.set_b32s1(p, TEX_NULL);

        // scan v-template after #
        loop {
            get_preamble_token_in(st)?;
            if st.scanner.cur_cmd <= CAR_RET && st.scanner.cur_cmd >= TAB_MARK && st.scanner.align_state == -1_000_000
            {
                break; // done2
            }
            if st.scanner.cur_cmd == MAC_PARAM {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Only one # is allowed per tab".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 3;
                st.errors.help_line[2] =
                    c"There should be exactly one # between &'s, when an".as_ptr();
                st.errors.help_line[1] =
                    c"\\halign or \\valign is being set up. In this case you had".as_ptr();
                st.errors.help_line[0] =
                    c"more than one, so I'm ignoring all but the first.".as_ptr();
                mc_propagate_error(crate::errors::error_in(st))?;
                continue; // goto continue_: restart this inner loop
            }
            let av = get_avail(&mut st.mem);
            if av == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(p, av);
            p = st.mem.b32s1(p);
            st.mem.set_b32s0(p, st.scanner.cur_tok);
        }

        // done2: append FROZEN_END_TEMPLATE and store v-template
        let av = get_avail(&mut st.mem);
        if av == ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(p, av);
        p = st.mem.b32s1(p);
        st.mem.set_b32s0(p, CS_TOKEN_FLAG + FROZEN_END_TEMPLATE);
        let ca = st.scanner.cur_align;
        let hh = st.mem.b32s1(HOLD_HEAD);
        st.mem.set_b32s1(ca + 2, hh);
    }

    // done:
    st.scanner.scanner_status = NORMAL_SCANNER;
    if let Some(a) = crate::save::new_save_level(&mut st.save_stack, ALIGN_GROUP, st.mem.line) {
        return Err(MainControlAbort::Save(a));
    }
    let every_cr = st.eqtb.b32s1(LOCAL_BASE + LOCAL__EVERY_CR);
    if every_cr != TEX_NULL {
        crate::input::begin_token_list_in(st, every_cr, EVERY_CR_TEXT)?;
    }
    align_peek_in(st)?;
    Ok(())
}

// ---- moved verbatim from scan.rs (workstream 02) ----

/// `&mut EngineState` core of [`push_math`]: enters math mode under one borrow.
pub(crate) fn push_math_in(
    st: &mut crate::state::EngineState,
    c: u8,
) -> Result<(), MainControlAbort> {
    crate::mem::push_nest_in(st)?;
    crate::globals::set_cur_list_mode(-207); // -MMODE (xetex_format.h:370)
    crate::globals::set_cur_list_aux_b32s1(crate::mem::TEX_NULL);
    if let Some(a) = crate::save::new_save_level(&mut st.save_stack, c, st.mem.line) {
        return Err(MainControlAbort::Save(a));
    }
    Ok(())
}
pub(crate) fn end_graf_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    if cur_list_mode() == HMODE {
        if crate::globals::cur_list_head() == crate::globals::cur_list_tail() {
            crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
        } else {
            match crate::linebreak::line_break_in(st, false) {
                crate::linebreak::LineBreakOutcome::Done => {}
                o => return Err(MainControlAbort::LineBreak(o)),
            }
        }
        let aux = crate::globals::cur_list_e_tex_aux();
        if aux != crate::mem::TEX_NULL {
            crate::mem::flush_list(&mut st.mem, aux);
            crate::globals::set_cur_list_e_tex_aux(crate::mem::TEX_NULL);
        }
        if let Some(a) = normal_paragraph(
            &mut st.eqtb,
            &mut st.save_stack,
            &mut st.mem,
            &mut st.scanner,
        ) {
            return Err(MainControlAbort::Save(a));
        }
        st.errors.error_count = 0;
    }
    Ok(())
}
// translated from xetex-xetex0.c:10016 (append_penalty)
fn append_penalty_node(mem: &mut crate::state::MemState, tail: i32, penalty: i32) -> i32 {
    let p_raw = crate::mem::new_penalty(mem, penalty);
    if p_raw == crate::mem::ALLOC_FAILED {
        return p_raw;
    }
    let p = mem
        .classify_penalty(MemIdx::new(p_raw))
        .expect("new_penalty stamped a valid node")
        .expect("new_penalty returned a penalty node");
    mem.set_b32s1(tail, p.idx().0);
    p.idx().0
}

pub(crate) fn append_penalty_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    crate::scan::scan_int_in(st)?;
    let tail = crate::globals::cur_list_tail();
    let p = append_penalty_node(&mut st.mem, tail, st.scanner.cur_val);
    if p == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    crate::globals::set_cur_list_tail(p);
    if cur_list_mode() == VMODE {
        crate::pagebuilder::build_page_in(st)?;
    }
    Ok(())
}
// translated from xetex-xetex0.c:9496 (normal_paragraph)
///
/// Phase-C: threaded to disjoint `&mut EqtbState`/`&mut SaveStackState`/`&mut
/// MemState`/`&mut ScannerState` borrows (`01b` §3). The `eq_word_define`/
/// `eq_define` calls return the `01b` §5 [`crate::save::SaveAbort`] sentinel on
/// a cold save-stack overflow; this fn short-circuits on the first one and
/// [`normal_paragraph_global`] raises it after the borrow drops.
pub(crate) fn normal_paragraph(
    eqtb: &mut crate::state::EqtbState,
    save_stack: &mut crate::state::SaveStackState,
    mem: &mut crate::state::MemState,
    scanner: &mut crate::state::ScannerState,
) -> Option<crate::save::SaveAbort> {
    if eqtb.b32s1(INT_BASE + INT_PAR_LOOSENESS) != 0 {
        let a = crate::save::eq_word_define(eqtb, save_stack, INT_BASE + INT_PAR_LOOSENESS, 0);
        if a.is_some() {
            return a;
        }
    }
    if eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_HANG_INDENT) != 0 {
        let a =
            crate::save::eq_word_define(eqtb, save_stack, DIMEN_BASE + DIMEN_PAR_HANG_INDENT, 0);
        if a.is_some() {
            return a;
        }
    }
    if eqtb.b32s1(INT_BASE + INT_PAR_HANG_AFTER) != 1 {
        let a = crate::save::eq_word_define(eqtb, save_stack, INT_BASE + INT_PAR_HANG_AFTER, 1);
        if a.is_some() {
            return a;
        }
    }
    if eqtb.b32s1(LOCAL_BASE) != crate::mem::TEX_NULL {
        let a = crate::save::eq_define(
            eqtb,
            save_stack,
            mem,
            scanner,
            LOCAL_BASE,
            SHAPE_REF,
            crate::mem::TEX_NULL,
        );
        if a.is_some() {
            return a;
        }
    }
    if eqtb.b32s1(ETEX_PEN_BASE) != crate::mem::TEX_NULL {
        let a = crate::save::eq_define(
            eqtb,
            save_stack,
            mem,
            scanner,
            ETEX_PEN_BASE,
            SHAPE_REF,
            crate::mem::TEX_NULL,
        );
        if a.is_some() {
            return a;
        }
    }
    None
}

pub(crate) fn its_all_over_in(
    st: &mut crate::state::EngineState,
) -> Result<bool, MainControlAbort> {
    if crate::errors::privileged_in(st).map_err(MainControlAbort::Error)? {
        if st.shipout.page_tail == PAGE_HEAD
            && crate::globals::cur_list_head() == crate::globals::cur_list_tail()
            && st.shipout.dead_cycles == 0
        {
            return Ok(true);
        }
        crate::input::back_input_in(st)?;
        let nb = crate::mem::new_null_box(&mut st.mem);
        if nb == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(crate::globals::cur_list_tail(), nb);
        crate::globals::set_cur_list_tail(nb);
        st.mem.set_b32s1(
            crate::globals::cur_list_tail() + 1,
            st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_HSIZE),
        );
        let ng = crate::mem::new_glue(&mut st.mem, 8);
        if ng == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(crate::globals::cur_list_tail(), ng);
        crate::globals::set_cur_list_tail(ng);
        let np = crate::mem::new_penalty(&mut st.mem, NULL_FLAG);
        if np == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(crate::globals::cur_list_tail(), np);
        crate::globals::set_cur_list_tail(np);
        crate::pagebuilder::build_page_in(st)?;
    }
    Ok(false)
}
// translated from xetex-xetex0.c:9808 (indent_in_hmode)
///
/// Phase-C: threaded to `&mut MemState` + `&EqtbState` (`01b` §3). The cold
/// arena-exhaustion branches of `new_null_box`/`new_noad` return the `01b` §5
/// [`crate::scan::NodeRecycleAbort`] sentinel; [`indent_in_hmode_global`] runs
/// the `overflow` abort after the borrow drops.
pub(crate) fn indent_in_hmode(
    st: &mut crate::state::EngineState,
) -> Result<(), crate::scan::NodeRecycleAbort> {
    if st.scanner.cur_chr > 0 {
        let p = crate::mem::new_null_box(&mut st.mem);
        if p == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow);
        }
        let dimen = st.eqtb.b32s1(DIMEN_BASE);
        st.mem.set_b32s1(p + 1, dimen);
        let p = if cur_list_mode().abs() == HMODE_I {
            crate::globals::set_cur_list_aux_b32s0(1000);
            p
        } else {
            let q = crate::mem::new_noad(&mut st.mem);
            if q == crate::mem::ALLOC_FAILED {
                return Err(crate::scan::NodeRecycleAbort::Overflow);
            }
            st.mem.set_b32s1(q + 1, SUB_BOX);
            st.mem.set_b32s0(q + 1, p);
            q
        };
        let tail = crate::globals::cur_list_tail();
        st.mem.set_b32s1(tail, p);
        let v = st.mem.b32s1(tail);
        crate::globals::set_cur_list_tail(v);
    }
    Ok(())
}

// translated from xetex-xetex0.c:9831 (head_for_vmode)
pub(crate) fn head_for_vmode_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    if cur_list_mode() < 0 {
        if st.scanner.cur_cmd != HRULE {
            off_save_in(st)?;
        } else {
            crate::output::error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
            crate::output::print_esc_cstr_in(st, c"hrule".as_ptr());
            crate::output::print_cstr_in(st, c"' here except with leaders".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 2;
            st.errors.help_line[1] =
                c"To put a horizontal rule in an hbox or an alignment,".as_ptr();
            st.errors.help_line[0] =
                c"you should use \\leaders or \\hrulefill (see The TeXbook).".as_ptr();
            mc_propagate_error(crate::errors::error_in(st))?;
        }
    } else {
        crate::input::back_input_in(st)?;
        st.scanner.cur_tok = st.input.par_token;
        crate::input::back_input_in(st)?;
        set_cur_input_index(INSERTED);
    }
    Ok(())
}
pub(crate) fn begin_insert_or_adjust_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    if st.scanner.cur_cmd == VADJUST {
        st.scanner.cur_val = 255;
    } else {
        crate::scan::scan_eight_bit_int_in(st)?;
        if st.scanner.cur_val == 255 {
            crate::output::error_here_with_diagnostic_in(st, c"You can't ".as_ptr());
            crate::output::print_esc_cstr_in(st, c"insert".as_ptr());
            crate::output::print_int_in(st, st.scanner.cur_val);
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 1;
            st.errors.help_line[0] = c"I'm changing to \\insert0; box 255 is special.".as_ptr();
            mc_propagate_error(crate::errors::error_in(st))?;
            st.scanner.cur_val = 0;
        }
    }
    let sp = st.save_stack.save_ptr;
    st.save_stack.set_b32s1(sp, st.scanner.cur_val);
    if st.scanner.cur_cmd == VADJUST && crate::scan::scan_keyword_in(st, c"pre".as_ptr())? {
        st.save_stack.set_b32s1(sp + 1, 1);
    } else {
        st.save_stack.set_b32s1(sp + 1, 0);
    }
    st.save_stack.save_ptr = sp + 2;
    if let Some(a) = crate::save::new_save_level(&mut st.save_stack, INSERT_GROUP, st.mem.line) {
        return Err(MainControlAbort::Save(a));
    }
    crate::scan::scan_left_brace_in(st)?;
    if let Some(a) = normal_paragraph(
        &mut st.eqtb,
        &mut st.save_stack,
        &mut st.mem,
        &mut st.scanner,
    ) {
        return Err(MainControlAbort::Save(a));
    }
    crate::mem::push_nest_in(st)?;
    crate::globals::set_cur_list_mode(-1); // -VMODE
    crate::globals::set_cur_list_aux_b32s1(IGNORE_DEPTH);
    Ok(())
}
// translated from xetex-xetex0.c:10161 (do_endv)
pub(crate) fn do_endv_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    const V_TEMPLATE: u16 = 2; // xetex-constants.h:201
    const TOKEN_LIST: u16 = 0; // xetex-constants.h:261
    const ALIGN_GROUP: u8 = 6; // xetex-constants.h:158

    st.input.base_ptr = st.input.input_ptr;
    st.input.input_stack[st.input.base_ptr as usize] = crate::globals::cur_input();
    while st.input.input_stack[st.input.base_ptr as usize].index != V_TEMPLATE
        && st.input.input_stack[st.input.base_ptr as usize].loc == crate::mem::TEX_NULL
        && st.input.input_stack[st.input.base_ptr as usize].state == TOKEN_LIST
    {
        st.input.base_ptr -= 1;
    }
    if st.input.input_stack[st.input.base_ptr as usize].index != V_TEMPLATE
        || st.input.input_stack[st.input.base_ptr as usize].loc != crate::mem::TEX_NULL
        || st.input.input_stack[st.input.base_ptr as usize].state != TOKEN_LIST
    {
        return Err(MainControlAbort::Input(crate::input::InputAbort::Fatal(
            c"(interwoven alignment preambles are not allowed)".as_ptr(),
        )));
    }
    if st.save_stack.cur_group == ALIGN_GROUP {
        end_graf_in(st)?;
        if fin_col_in(st)? {
            fin_row_in(st)?;
        }
    } else {
        off_save_in(st)?;
    }
    Ok(())
}
pub(crate) fn unpackage_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    const COPY_CODE: i32 = 1; // xetex_format.h:481
    const VLIST_NODE: u16 = 1; // xetex_format.h:381
    const MARGIN_KERN_NODE: u16 = 40; // xetex_format.h:396
    const MARGIN_KERN_NODE_SIZE: i32 = 3; // xetex-constants.h:115
    const HMODE: i16 = 104; // xetex_format.h:369
    const MMODE: i16 = 207; // xetex_format.h:370

    if st.scanner.cur_chr > COPY_CODE {
        let tail = crate::globals::cur_list_tail();
        st.mem
            .set_b32s1(tail, st.scanner.disc_ptr[st.scanner.cur_chr as usize]);
        st.scanner.disc_ptr[st.scanner.cur_chr as usize] = crate::mem::TEX_NULL;
        // goto done — jump to tail-walk loop
    } else {
        let c = st.scanner.cur_chr;
        crate::scan::scan_register_num_in(st)?;
        let p = if st.scanner.cur_val < 256 {
            st.eqtb.b32s1(BOX_BASE + st.scanner.cur_val)
        } else {
            let __cv18 = st.scanner.cur_val;
            crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, __cv18, false);
            if st.scanner.cur_ptr == crate::mem::TEX_NULL {
                crate::mem::TEX_NULL
            } else {
                st.mem.b32s1(st.scanner.cur_ptr + 1)
            }
        };

        if p == crate::mem::TEX_NULL {
            return Ok(());
        }

        let mode = cur_list_mode().abs();
        let ntype = st.mem.b16s1(p);
        if mode == MMODE
            || (mode == VMODE && ntype != VLIST_NODE)
            || (mode == HMODE && ntype != HLIST_NODE)
        {
            crate::output::error_here_with_diagnostic_in(
                st,
                c"Incompatible list can't be unboxed".as_ptr(),
            );
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 3;
            st.errors.help_line[2] = c"Sorry, Pandora. (You sneaky devil.)".as_ptr();
            st.errors.help_line[1] =
                c"I refuse to unbox an \\hbox in vertical mode or vice versa.".as_ptr();
            st.errors.help_line[0] = c"And I can't open any boxes in math mode.".as_ptr();
            return mc_propagate_error(crate::errors::error_in(st));
        }

        let list_head = st.mem.b32s1(p + 5);
        let tail = crate::globals::cur_list_tail();
        if c == COPY_CODE {
            let copy = crate::packaging::copy_node_list(&mut st.mem, list_head)?;
            st.mem.set_b32s1(tail, copy);
        } else {
            st.mem.set_b32s1(tail, list_head);
            if st.scanner.cur_val < 256 {
                st.eqtb
                    .set_b32s1(BOX_BASE + st.scanner.cur_val, crate::mem::TEX_NULL);
            } else {
                let __cv17 = st.scanner.cur_val;
                crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, __cv17, false);
                let cp = st.scanner.cur_ptr;
                if cp != crate::mem::TEX_NULL {
                    st.mem.set_b32s1(cp + 1, crate::mem::TEX_NULL);
                    let rc = st.mem.b32s0(cp + 1) + 1;
                    st.mem.set_b32s0(cp + 1, rc);
                    crate::save::delete_sa_ref(&mut st.mem, &mut st.scanner, cp);
                }
            }
            crate::mem::free_node(&mut st.mem, p, BOX_NODE_SIZE);
        }
    }

    // done: advance cur_list.tail past any MARGIN_KERN_NODEs
    while st.mem.b32s1(crate::globals::cur_list_tail()) != crate::mem::TEX_NULL {
        let r = st.mem.b32s1(crate::globals::cur_list_tail());
        if r < st.mem.hi_mem_min && st.mem.b16s1(r) == MARGIN_KERN_NODE {
            let tail = crate::globals::cur_list_tail();
            st.mem.set_b32s1(tail, st.mem.b32s1(r));
            crate::mem::free_node(&mut st.mem, r, MARGIN_KERN_NODE_SIZE);
        } else {
            crate::globals::set_cur_list_tail(st.mem.b32s1(crate::globals::cur_list_tail()));
        }
    }
    Ok(())
}
pub(crate) fn delete_last_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    const BEGIN_M_CODE: u16 = 2; // xetex_format.h:436

    if cur_list_mode() == VMODE
        && crate::globals::cur_list_tail() == crate::globals::cur_list_head()
    {
        // On page: only allowed for \lastskip when last_glue != MAX_HALFWORD
        if st.scanner.cur_chr as u16 != GLUE_NODE || st.shipout.last_glue != MAX_HALFWORD {
            crate::errors::you_cant_in(st);
            st.errors.help_ptr = 2;
            st.errors.help_line[1] =
                c"Sorry...I usually can't take things from the current page.".as_ptr();
            st.errors.help_line[0] = c"Try `I\\vskip-\\lastskip' instead.".as_ptr();
            if st.scanner.cur_chr as u16 == KERN_NODE {
                st.errors.help_line[0] = c"Try `I\\kern-\\lastkern' instead.".as_ptr();
            } else if st.scanner.cur_chr as u16 != GLUE_NODE {
                st.errors.help_line[0] = c"Perhaps you can make the output routine do it.".as_ptr();
            }
            return mc_propagate_error(crate::errors::error_in(st));
        }
        return Ok(());
    }

    let mut tx = crate::globals::cur_list_tail();

    // If tail is MATH END, back up one to find the actual last "deletable" node
    if tx < st.mem.hi_mem_min && st.mem.b16s1(tx) == MATH_NODE && st.mem.b16s0(tx) == END_M_CODE {
        let mut r = crate::globals::cur_list_head();
        loop {
            let next_r = st.mem.b32s1(r);
            if next_r == tx {
                tx = r;
                break;
            }
            r = next_r;
        }
    }

    if tx >= st.mem.hi_mem_min || st.mem.b16s1(tx) as i32 != st.scanner.cur_chr {
        return Ok(());
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
        if q < st.mem.hi_mem_min {
            if st.mem.b16s1(q) == DISC_NODE {
                let count = st.mem.b16s0(q);
                for _ in 0..count {
                    p = st.mem.b32s1(p);
                }
                if p == tx {
                    return Ok(());
                }
            } else if st.mem.b16s1(q) == MATH_NODE && st.mem.b16s0(q) == BEGIN_M_CODE {
                fm = true;
            }
        }
        q = st.mem.b32s1(p);
        if q == tx {
            break;
        }
    }

    // q == tx; p is the node just before tx; r_save is the one before p
    let q_next = st.mem.b32s1(tx);
    st.mem.set_b32s1(p, q_next);
    st.mem.set_b32s1(tx, crate::mem::TEX_NULL);
    if q_next == crate::mem::TEX_NULL {
        if fm {
            return Err(MainControlAbort::Confusion(c"tail1"));
        } else {
            crate::globals::set_cur_list_tail(p);
        }
    } else if fm {
        crate::globals::set_cur_list_tail(r_save);
        st.mem.set_b32s1(r_save, crate::mem::TEX_NULL);
        crate::packaging::flush_node_list(&mut st.mem, p).map_err(MainControlAbort::Confusion)?;
    }
    crate::packaging::flush_node_list(&mut st.mem, tx).map_err(MainControlAbort::Confusion)?;
    Ok(())
}
// translated from xetex-xetex0.c:9223 (app_space)
// Phase-C: threaded to `&mut MemState`/`&EqtbState`/`&mut FontState`; legacy
// entry points use [`app_space_in`]. Returns the deferred `xn_over_d` side
// effects (`tex_remainder`, plus `arith_error` on overflow); `None` when the
// spacing branch taken did no scaling arithmetic.
pub(crate) fn app_space(
    st: &mut crate::state::EngineState,
) -> Result<Option<(i32, bool)>, crate::scan::NodeRecycleAbort> {
    use crate::globals::{main_k, main_p, set_cur_list_tail, set_main_k, set_main_p};
    const GLUE_PAR_SPACE_SKIP: i32 = 12; // xetex_format.h:161
    const GLUE_PAR_XSPACE_SKIP: i32 = 13; // xetex_format.h:162
    const EXTRA_SPACE_CODE: i32 = 7; // xetex-constants.h:299

    let space_factor = crate::globals::cur_list_aux_b32s0();
    let cf = st.eqtb.b32s1(CUR_FONT_LOC);
    let q;
    let mut side_effects = None;
    if space_factor >= 2000 && st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_XSPACE_SKIP) != 0 {
        q = crate::mem::new_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_XSPACE_SKIP as i16);
        if q == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow);
        }
    } else {
        if st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP) != 0 {
            set_main_p(st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP));
        } else {
            set_main_p(st.fonts.font_glue(cf));
            if main_p() == crate::mem::TEX_NULL {
                set_main_p(crate::mem::new_spec(&mut st.mem, 0));
                if main_p() == crate::mem::ALLOC_FAILED {
                    return Err(crate::scan::NodeRecycleAbort::Overflow);
                }
                set_main_k(st.fonts.param_base(cf) + 2);
                // width = space, stretch = space_stretch, shrink = space_shrink
                st.mem.set_b32s1(main_p() + 1, st.fonts.font_info_b32s1(main_k()));
                st.mem.set_b32s1(main_p() + 2, st.fonts.font_info_b32s1(main_k() + 1));
                st.mem.set_b32s1(main_p() + 3, st.fonts.font_info_b32s1(main_k() + 2));
                st.fonts.set_font_glue(cf, main_p());
            }
        }
        set_main_p(crate::mem::new_spec(&mut st.mem, main_p()));
        if main_p() == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow);
        }
        if space_factor >= 2000 {
            // add extra_space to width
            let extra = st.fonts.font_info_b32s1(EXTRA_SPACE_CODE + st.fonts.param_base(cf));
            st.mem.set_b32s1(main_p() + 1, st.mem.b32s1(main_p() + 1) + extra);
        }
        // scale stretch by space_factor / 1000
        let (new_stretch, _r1, ov1) =
            crate::scaled_math::xn_over_d_parts(st.mem.b32s1(main_p() + 2), space_factor, 1000);
        st.mem.set_b32s1(main_p() + 2, new_stretch);
        // scale shrink by 1000 / space_factor
        let (new_shrink, r2, ov2) =
            crate::scaled_math::xn_over_d_parts(st.mem.b32s1(main_p() + 3), 1000, space_factor);
        st.mem.set_b32s1(main_p() + 3, new_shrink);
        // `xn_over_d` sets `tex_remainder` (last call wins) and `arith_error`
        // on any overflow; defer both to after the borrow drops (§129).
        side_effects = Some((r2, ov1 || ov2));
        q = crate::mem::new_glue(&mut st.mem, main_p());
        if q == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow);
        }
        st.mem.set_b32s1(main_p(), crate::mem::TEX_NULL);
    }
    st.mem.set_b32s1(crate::globals::cur_list_tail(), q);
    set_cur_list_tail(q);
    Ok(side_effects)
}

pub(crate) fn app_space_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    if let Some((remainder, overflowed)) = app_space(st)? {
        if overflowed {
            st.errors.arith_error = true;
        }
        st.errors.tex_remainder = remainder;
    }
    Ok(())
}
pub(crate) fn scan_box_in(
    st: &mut crate::state::EngineState,
    box_context: i32,
) -> Result<(), MainControlAbort> {
    const MAKE_BOX: u8 = 20; // xetex_format.h:258
    const HRULE: u8 = 36; // xetex_format.h:274
    const VRULE: u8 = 35; // xetex_format.h:273
    const LEADER_FLAG: i32 = 0x40010001; // xetex-constants.h:375
    loop {
        crate::scan::get_x_token_in(st)?;
        if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
            break;
        }
    }
    if st.scanner.cur_cmd == MAKE_BOX {
        begin_box_in(st, box_context)?;
    } else if box_context >= LEADER_FLAG && (st.scanner.cur_cmd == HRULE || st.scanner.cur_cmd == VRULE) {
        st.scanner.cur_box = crate::scan::scan_rule_spec_in(st)?;
        box_end_in(st, box_context)?;
    } else {
        crate::output::error_here_with_diagnostic_in(
            st,
            c"A <box> was supposed to be here".as_ptr(),
        );
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 3;
        st.errors.help_line[2] =
            c"I was expecting to see \\hbox or \\vbox or \\copy or \\box or".as_ptr();
        st.errors.help_line[1] =
            c"something like that. So you might find something missing in".as_ptr();
        st.errors.help_line[0] = c"your output. But keep trying; you can fix this later.".as_ptr();
        let outcome = crate::input::back_error_in(st)?;
        if !matches!(outcome, crate::errors::ErrorOutcome::Continue) {
            return Err(MainControlAbort::Error(outcome));
        }
    }
    Ok(())
}
// translated from xetex-xetex0.c:9509 (package)
/// `&mut EngineState` core of [`package`]: closes a box group, packs the inner
/// list, and dispatches via [`box_end_in`] — all under one borrow. `cur_list_*`
/// stay raw `mod raw`; `cur_box` is `st.scanner.cur_box`. Cold aborts ride
/// [`MainControlAbort`].
///
/// translated from xetex-xetex0.c:9509 (package)
pub(crate) fn package_in(
    st: &mut crate::state::EngineState,
    c: i16,
) -> Result<(), MainControlAbort> {
    use crate::globals::{cur_list_head, cur_list_mode};
    use crate::mem::TEX_NULL;
    const DIMEN_PAR_BOX_MAX_DEPTH: i32 = 7; // xetex_format.h:129
    const INT_PAR_XETEX_UPWARDS: i32 = 71; // xetex_format.h:106
    const VTOP_CODE: i16 = 4; // xetex_format.h:484
    const INNER_HMODE: i16 = -104; // xetex_format.h:369 negated

    let d = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_BOX_MAX_DEPTH);
    let u = st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS);
    crate::save::unsave_in(st)?;
    st.save_stack.save_ptr -= 3;
    let v = st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS);
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS, u);
    if cur_list_mode() == INNER_HMODE {
        let sp = st.save_stack.save_ptr;
        let list = st.mem.b32s1(cur_list_head());
        let w = st.save_stack.b32s1(sp + 2);
        let m = st.save_stack.b32s1(sp + 1) as i16;
        st.scanner.cur_box = crate::packaging::hpack_emit_in(st, list, w, m)?.idx().0;
    } else {
        let sp = st.save_stack.save_ptr;
        let list = st.mem.b32s1(cur_list_head());
        let h = st.save_stack.b32s1(sp + 2);
        let m = st.save_stack.b32s1(sp + 1) as i16;
        let cb = crate::packaging::vpackage_emit_in(st, list, h, m, d)?;
        st.scanner.cur_box = cb.idx().0;
        if c == VTOP_CODE {
            let mut hh = 0i32;
            let p = cb.list_ptr(&st.mem).0;
            if p != TEX_NULL && st.mem.b16s1(p) <= 2 {
                hh = st.mem.b32s1(p + 3);
            }
            let new_depth = cb.depth(&st.mem).0 - hh + cb.height(&st.mem).0;
            cb.set_depth(&mut st.mem, Scaled(new_depth));
            cb.set_height(&mut st.mem, Scaled(hh));
        }
    }
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS, v);
    crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
    let ctx = st.save_stack.b32s1(st.save_stack.save_ptr);
    box_end_in(st, ctx)?;
    Ok(())
}
/// `&mut EngineState` core of [`new_graf`]: starts a new paragraph (par-skip
/// glue, nest push, hyphenation/language setup, optional indent box +
/// src-special, `\everypar`, page build) under one borrow. `cur_list_*` stay
/// raw `mod raw`. Cold aborts ride [`MainControlAbort`].
///
/// translated from xetex-xetex0.c:9549 (new_graf)
pub(crate) fn new_graf_in(
    st: &mut crate::state::EngineState,
    indented: bool,
) -> Result<(), MainControlAbort> {
    use crate::globals::{
        cur_list_head, cur_list_mode, cur_list_tail, set_cur_list_aux_b32s0,
        set_cur_list_aux_b32s1, set_cur_list_mode, set_cur_list_prev_graf, set_cur_list_tail,
    };
    use crate::mem::TEX_NULL;
    const GLUE_PAR_PAR_SKIP: i16 = 2; // xetex_format.h:151
    const VMODE_I16: i16 = 1; // xetex_format.h:369
    const HMODE_I16: i16 = 104; // xetex_format.h:369
    const BIGGEST_LANG: i32 = 255; // xetex-constants.h:327
    const LOCAL__EVERY_PAR: i32 = 2; // xetex_format.h:174
    const EVERY_PAR_TEXT: u16 = 8; // xetex-constants.h:207

    set_cur_list_prev_graf(0);

    if cur_list_mode() == VMODE_I16 || cur_list_head() != cur_list_tail() {
        let pg = crate::mem::new_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_PAR_SKIP);
        if pg == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(cur_list_tail(), pg);
        set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
    }

    crate::mem::push_nest_in(st)?;
    set_cur_list_mode(HMODE_I16);
    set_cur_list_aux_b32s0(1000);

    let lang_val = st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE);
    let l: u8 = if lang_val <= 0 || lang_val > BIGGEST_LANG {
        0
    } else {
        lang_val as u8
    };
    st.hyph.cur_lang = l;
    set_cur_list_aux_b32s1(l as i32);
    let lhm = crate::helpers::norm_min(st.eqtb.b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as i32;
    let rhm = crate::helpers::norm_min(st.eqtb.b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as i32;
    set_cur_list_prev_graf((lhm * 64 + rhm) * 65536 + l as i32);

    if indented {
        let nb = crate::mem::new_null_box(&mut st.mem);
        if nb == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
        }
        set_cur_list_tail(nb);
        st.mem.set_b32s1(cur_list_head(), nb);
        st.mem.set_b32s1(nb + 1, st.eqtb.b32s1(DIMEN_BASE));
        if st.shipout.insert_src_special_every_par {
            crate::ext::insert_src_special_in(st)?;
        }
    }

    let every_par = st.eqtb.b32s1(LOCAL_BASE + LOCAL__EVERY_PAR);
    if every_par != TEX_NULL {
        crate::input::begin_token_list_in(st, every_par, EVERY_PAR_TEXT)?;
    }

    if st.input.nest_ptr == 1 {
        crate::pagebuilder::build_page_in(st)?;
    }
    Ok(())
}
pub(crate) enum NewCharacterAbort {
    Native(crate::fonts::NativeCharAbort),
    Error(crate::errors::ErrorOutcome),
    MainMemoryOverflow,
}

impl NewCharacterAbort {
    /// Map to the unified [`EngineAbort`] terminal (`01d` §5.5), mirroring the
    /// former `finish()` cold-abort tail without longjmp-ing. `MainMemoryOverflow`
    /// runs `runaway` first (via `main_memory_overflow_abort`).
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            NewCharacterAbort::Native(a) => a.into_engine_abort(),
            NewCharacterAbort::Error(outcome) => outcome.into_engine_abort(),
            NewCharacterAbort::MainMemoryOverflow => crate::mem::main_memory_overflow_abort(),
        }
    }
}

pub(crate) enum MainControlAbort {
    Scanner(crate::scan::SccAbort),
    NodeRecycle(crate::scan::NodeRecycleAbort),
    PageBuild(crate::pagebuilder::PageBuildAbort),
    LineBreak(crate::linebreak::LineBreakOutcome),
    PushNest(crate::mem::PushNestAbort),
    Vsplit(crate::packaging::VsplitAbort),
    Input(crate::input::InputAbort),
    Shipout(crate::shipout::ShipoutAbort),
    Save(crate::save::SaveAbort),
    SaMutation(crate::save::SaMutationAbort),
    Unsave(crate::save::UnsaveAbort),
    InsertSrc(crate::ext::InsertSrcAbort),
    IssueMessage(crate::output::IssueMessageAbort),
    ShowWhatever(crate::output::ShowWhateverAbort),
    OpenLog(crate::io::OpenLogAbort),
    OpenOrClose(crate::io::OpenOrCloseAbort),
    MakeTexString(crate::texmfmp::MakeTexStringAbort),
    ReadFontInfo(crate::fonts::ReadFontInfoAbort),
    Error(crate::errors::ErrorOutcome),
    Confusion(&'static std::ffi::CStr),
    NewCharacter(NewCharacterAbort),
    Trie(crate::hyphenation::TrieAbort),
    /// Serious internal error closing an input file (the C `ttstub_input_close`
    /// wrapper `longjmp`'d here); reached from `load_picture_in` (`01d` §2.1).
    InputClose,
    /// A native-font measurement cold abort (`01d` §5.6).
    NativeFont(crate::ext::NativeFontAbort),
}

impl From<crate::ext::NativeFontAbort> for MainControlAbort {
    fn from(e: crate::ext::NativeFontAbort) -> Self {
        MainControlAbort::NativeFont(e)
    }
}

impl From<crate::hyphenation::TrieAbort> for MainControlAbort {
    fn from(e: crate::hyphenation::TrieAbort) -> Self {
        MainControlAbort::Trie(e)
    }
}

impl From<NewCharacterAbort> for MainControlAbort {
    fn from(e: NewCharacterAbort) -> Self {
        MainControlAbort::NewCharacter(e)
    }
}

impl From<crate::scan::SccAbort> for MainControlAbort {
    fn from(e: crate::scan::SccAbort) -> Self {
        MainControlAbort::Scanner(e)
    }
}

impl From<crate::scan::NodeRecycleAbort> for MainControlAbort {
    fn from(e: crate::scan::NodeRecycleAbort) -> Self {
        MainControlAbort::NodeRecycle(e)
    }
}

impl From<crate::pagebuilder::PageBuildAbort> for MainControlAbort {
    fn from(e: crate::pagebuilder::PageBuildAbort) -> Self {
        MainControlAbort::PageBuild(e)
    }
}

impl From<crate::mem::PushNestAbort> for MainControlAbort {
    fn from(e: crate::mem::PushNestAbort) -> Self {
        MainControlAbort::PushNest(e)
    }
}

impl From<crate::packaging::VsplitAbort> for MainControlAbort {
    fn from(e: crate::packaging::VsplitAbort) -> Self {
        MainControlAbort::Vsplit(e)
    }
}

impl From<crate::input::InputAbort> for MainControlAbort {
    fn from(e: crate::input::InputAbort) -> Self {
        MainControlAbort::Input(e)
    }
}

impl From<crate::shipout::ShipoutAbort> for MainControlAbort {
    fn from(e: crate::shipout::ShipoutAbort) -> Self {
        MainControlAbort::Shipout(e)
    }
}

impl From<crate::save::SaMutationAbort> for MainControlAbort {
    fn from(e: crate::save::SaMutationAbort) -> Self {
        MainControlAbort::SaMutation(e)
    }
}

impl From<crate::save::UnsaveAbort> for MainControlAbort {
    fn from(e: crate::save::UnsaveAbort) -> Self {
        MainControlAbort::Unsave(e)
    }
}

impl From<crate::ext::InsertSrcAbort> for MainControlAbort {
    fn from(e: crate::ext::InsertSrcAbort) -> Self {
        MainControlAbort::InsertSrc(e)
    }
}

impl From<crate::output::IssueMessageAbort> for MainControlAbort {
    fn from(e: crate::output::IssueMessageAbort) -> Self {
        MainControlAbort::IssueMessage(e)
    }
}

impl From<crate::output::ShowWhateverAbort> for MainControlAbort {
    fn from(e: crate::output::ShowWhateverAbort) -> Self {
        MainControlAbort::ShowWhatever(e)
    }
}

impl From<crate::io::OpenLogAbort> for MainControlAbort {
    fn from(e: crate::io::OpenLogAbort) -> Self {
        MainControlAbort::OpenLog(e)
    }
}

impl From<crate::io::OpenOrCloseAbort> for MainControlAbort {
    fn from(e: crate::io::OpenOrCloseAbort) -> Self {
        MainControlAbort::OpenOrClose(e)
    }
}

impl From<crate::texmfmp::MakeTexStringAbort> for MainControlAbort {
    fn from(e: crate::texmfmp::MakeTexStringAbort) -> Self {
        MainControlAbort::MakeTexString(e)
    }
}

impl From<crate::fonts::ReadFontInfoAbort> for MainControlAbort {
    fn from(e: crate::fonts::ReadFontInfoAbort) -> Self {
        MainControlAbort::ReadFontInfo(e)
    }
}

impl MainControlAbort {
    /// Map the aggregated main-control cold abort to the unified [`EngineAbort`]
    /// terminal (`01d` §5.5), delegating to each leaf's `into_engine_abort`. Runs
    /// *after* the `main_control_in` borrow drops (fired at [`main_control`]);
    /// `tt_run_engine` then prints + cleans via [`EngineAbort::into_history`]
    /// instead of longjmp-ing. Replaces the former `fire()` longjmp dispatcher.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            MainControlAbort::Scanner(e) => e.into_engine_abort(),
            MainControlAbort::NodeRecycle(e) => e.into_engine_abort(),
            MainControlAbort::PageBuild(e) => e.into_engine_abort(),
            MainControlAbort::LineBreak(o) => o.into_engine_abort(),
            MainControlAbort::PushNest(e) => e.into_engine_abort(),
            MainControlAbort::Vsplit(a) => a.into_engine_abort(),
            MainControlAbort::Input(e) => e.into_engine_abort(),
            MainControlAbort::Shipout(e) => e.into_engine_abort(),
            MainControlAbort::Save(a) => a.into_engine_abort(),
            MainControlAbort::SaMutation(a) => a.into_engine_abort(),
            MainControlAbort::Unsave(a) => a.into_engine_abort(),
            MainControlAbort::InsertSrc(a) => a.into_engine_abort(),
            MainControlAbort::IssueMessage(a) => a.into_engine_abort(),
            MainControlAbort::ShowWhatever(a) => a.into_engine_abort(),
            MainControlAbort::OpenLog(a) => a.into_engine_abort(),
            MainControlAbort::OpenOrClose(a) => a.into_engine_abort(),
            MainControlAbort::MakeTexString(a) => a.into_engine_abort(),
            MainControlAbort::ReadFontInfo(a) => a.into_engine_abort(),
            MainControlAbort::Error(o) => o.into_engine_abort(),
            MainControlAbort::Confusion(m) => crate::errors::EngineAbort::Confusion(m.as_ptr()),
            MainControlAbort::NewCharacter(a) => a.into_engine_abort(),
            MainControlAbort::Trie(e) => e.into_engine_abort(),
            MainControlAbort::InputClose => crate::errors::EngineAbort::TtAbort(
                c"serious internal error closing an input file".as_ptr(),
            ),
            MainControlAbort::NativeFont(e) => e.into_engine_abort(),
        }
    }
}

/// Propagate a recoverable [`crate::errors::ErrorOutcome`] from a threaded
/// main-control verb: `Continue` returns `Ok(())`, any cold tail rides
/// [`MainControlAbort::Error`] to be `finish()`ed after the borrow drops.
#[inline]
pub(crate) fn mc_propagate_error(o: crate::errors::ErrorOutcome) -> Result<(), MainControlAbort> {
    if matches!(o, crate::errors::ErrorOutcome::Continue) {
        Ok(())
    } else {
        Err(MainControlAbort::Error(o))
    }
}

// translated from xetex-xetex0.c:7048 (new_character)
pub(crate) fn new_character_in(
    st: &mut crate::state::EngineState,
    f: i32,
    c: u16,
) -> Result<i32, NewCharacterAbort> {
    let fa = st.fonts.font_area_entry(f);
    if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
        return crate::fonts::new_native_character_in(st, f, c as u32)
            .map_err(NewCharacterAbort::Native);
    }
    let ec = crate::fonts::effective_char(&mut st.fonts, false, f, c) as u16;
    if st.fonts.font_bc_entry(f) <= ec && st.fonts.font_ec_entry(f) >= ec {
        let info = st
            .fonts
            .font_info_b16(st.fonts.char_base_entry(f) + ec as i32);
        if info.s3 > 0 {
            let p = crate::mem::get_avail(&mut st.mem);
            if p == crate::mem::ALLOC_FAILED {
                return Err(NewCharacterAbort::MainMemoryOverflow);
            }
            st.mem.set_b16s1(p, f as u16);
            st.mem.set_b16s0(p, c);
            return Ok(p);
        }
    }
    let outcome = crate::output::char_warning_in(st, f, c as i32);
    if !matches!(outcome, crate::errors::ErrorOutcome::Continue) {
        return Err(NewCharacterAbort::Error(outcome));
    }
    Ok(crate::mem::TEX_NULL)
}

// translated from xetex-xetex0.c:7797 (box_end)
pub(crate) fn box_end_in(
    st: &mut crate::state::EngineState,
    box_context: i32,
) -> Result<(), MainControlAbort> {
    use crate::globals::{
        cur_list_mode, cur_list_tail, set_cur_list_aux_b32s0, set_cur_list_tail,
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
        let cb = st.scanner.cur_box;
        if cb != TEX_NULL {
            st.mem.set_b32s1(cb + 4, box_context);

            if cur_list_mode().abs() == VMODE {
                if st.scanner.pre_adjust_tail != TEX_NULL {
                    if PRE_ADJUST_HEAD != st.scanner.pre_adjust_tail {
                        let tail = cur_list_tail();
                        st.mem.set_b32s1(tail, st.mem.b32s1(PRE_ADJUST_HEAD));
                        set_cur_list_tail(st.scanner.pre_adjust_tail);
                    }
                    st.scanner.pre_adjust_tail = TEX_NULL;
                }

                let b = st
                    .mem
                    .classify_box(MemIdx::new(cb))
                    .expect("valid node type for append_to_vlist")
                    .expect("append_to_vlist receives an hlist or vlist node");
                crate::packaging::append_to_vlist(&mut st.mem, &st.eqtb, b)?;

                if st.scanner.adjust_tail != TEX_NULL {
                    if ADJUST_HEAD != st.scanner.adjust_tail {
                        let tail = cur_list_tail();
                        st.mem.set_b32s1(tail, st.mem.b32s1(ADJUST_HEAD));
                        set_cur_list_tail(st.scanner.adjust_tail);
                    }
                    st.scanner.adjust_tail = TEX_NULL;
                }

                if cur_list_mode() > 0 {
                    crate::pagebuilder::build_page_in(st)?;
                }
            } else {
                if cur_list_mode().abs() == HMODE {
                    set_cur_list_aux_b32s0(1000); // space_factor
                } else {
                    let p = crate::mem::new_noad(&mut st.mem);
                    if p == crate::mem::ALLOC_FAILED {
                        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                    }
                    st.mem.set_b32s1(p + 1, SUB_BOX);
                    st.mem.set_b32s0(p + 1, cb);
                    st.scanner.cur_box = p;
                }

                let cb2 = st.scanner.cur_box;
                let tail = cur_list_tail();
                st.mem.set_b32s1(tail, cb2);
                set_cur_list_tail(cb2);
            }
        }
    } else if box_context < SHIP_OUT_FLAG {
        let (val, a) = if box_context < GLOBAL_BOX_FLAG {
            (box_context - BOX_FLAG, 0i16)
        } else {
            (box_context - GLOBAL_BOX_FLAG, 4i16)
        };
        st.scanner.cur_val = val;
        let cb = st.scanner.cur_box;

        if st.scanner.cur_val < 256 {
            let abort = if a >= 4 {
                let __cv16 = st.scanner.cur_val;
                crate::save::geq_define(
                    &mut st.eqtb,
                    &mut st.mem,
                    &mut st.scanner,
                    BOX_BASE + __cv16,
                    BOX_REF,
                    cb,
                )
            } else {
                let __cv15 = st.scanner.cur_val;
                crate::save::eq_define(
                    &mut st.eqtb,
                    &mut st.save_stack,
                    &mut st.mem,
                    &mut st.scanner,
                    BOX_BASE + __cv15,
                    BOX_REF,
                    cb,
                )
            };
            if let Some(sa) = abort {
                return Err(MainControlAbort::Save(sa));
            }
        } else {
            let __cv14 = st.scanner.cur_val;
            let r = crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, __cv14, true);
            if r == crate::mem::ALLOC_FAILED {
                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
            }
            let p = st.scanner.cur_ptr;
            let result = if a >= 4 {
                crate::save::gsa_def(&mut st.mem, p, cb)
            } else {
                crate::save::sa_def(&mut st.mem, &mut st.scanner, &mut st.save_stack, p, cb)
            };
            crate::save::finish_sa_mutation_in(st, p, result)?;
        }
    } else if st.scanner.cur_box != TEX_NULL {
        if box_context > SHIP_OUT_FLAG {
            loop {
                crate::scan::get_x_token_in(st)?;
                if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
                    break;
                }
            }

            if (st.scanner.cur_cmd == HSKIP && cur_list_mode().abs() != VMODE)
                || (st.scanner.cur_cmd == VSKIP && cur_list_mode().abs() == VMODE)
            {
                append_glue_in(st)?;
                let tail = cur_list_tail();
                st.mem
                    .set_b16s0(tail, (box_context - (LEADER_FLAG - A_LEADERS)) as u16);
                let cb = st.scanner.cur_box;
                st.mem.set_b32s1(tail + 1, cb);
            } else {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Leaders not followed by proper glue".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 3;
                st.errors.help_line[2] =
                    c"You should say `\\leaders <box or rule><hskip or vskip>'.".as_ptr();
                st.errors.help_line[1] =
                    c"I found the <box or rule>, but there's no suitable".as_ptr();
                st.errors.help_line[0] =
                    c"<hskip or vskip>, so I'm ignoring these leaders.".as_ptr();
                let outcome = crate::input::back_error_in(st)?;
                if !matches!(outcome, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(outcome));
                }
                let cb = st.scanner.cur_box;
                crate::packaging::flush_node_list(&mut st.mem, cb)
                    .map_err(MainControlAbort::Confusion)?;
            }
        } else {
            let cb = st.scanner.cur_box;
            unsafe {
                crate::shipout::ship_out_in(st, cb)?;
            }
        }
    }
    Ok(())
}
pub(crate) fn begin_box_in(
    st: &mut crate::state::EngineState,
    box_context: i32,
) -> Result<(), MainControlAbort> {
    use crate::globals::{
        cur_list_head, cur_list_mode, cur_list_tail, set_cur_list_aux_b32s0,
        set_cur_list_aux_b32s1, set_cur_list_mode, set_cur_list_tail,
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

    match st.scanner.cur_chr {
        BOX_CODE => {
            crate::scan::scan_register_num_in(st)?;
            let v = st.scanner.cur_val;
            if v < 256 {
                st.scanner.cur_box = st.eqtb.b32s1(BOX_BASE + v);
            } else {
                let r = crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, v, false);
                if r == crate::mem::ALLOC_FAILED {
                    return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                }
                let cp = st.scanner.cur_ptr;
                st.scanner.cur_box = if cp == TEX_NULL {
                    TEX_NULL
                } else {
                    st.mem.b32s1(cp + 1)
                };
            }
            if v < 256 {
                st.eqtb.set_b32s1(BOX_BASE + v, TEX_NULL);
            } else {
                let r = crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, v, false);
                if r == crate::mem::ALLOC_FAILED {
                    return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                }
                let cp = st.scanner.cur_ptr;
                if cp != TEX_NULL {
                    st.mem.set_b32s1(cp + 1, TEX_NULL);
                    st.mem.set_b32s0(cp + 1, st.mem.b32s0(cp + 1) + 1);
                    crate::save::delete_sa_ref(&mut st.mem, &mut st.scanner, cp);
                }
            }
        }

        COPY_CODE => {
            crate::scan::scan_register_num_in(st)?;
            let v = st.scanner.cur_val;
            let q = if v < 256 {
                st.eqtb.b32s1(BOX_BASE + v)
            } else {
                let r = crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, v, false);
                if r == crate::mem::ALLOC_FAILED {
                    return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                }
                let cp = st.scanner.cur_ptr;
                if cp == TEX_NULL {
                    TEX_NULL
                } else {
                    st.mem.b32s1(cp + 1)
                }
            };
            st.scanner.cur_box = crate::packaging::copy_node_list(&mut st.mem, q)?;
        }

        LAST_BOX_CODE => {
            st.scanner.cur_box = TEX_NULL;

            if cur_list_mode().abs() == MMODE {
                crate::errors::you_cant_in(st);
                st.errors.help_ptr = 1;
                st.errors.help_line[0] = c"Sorry; this \\lastbox will be void.".as_ptr();
                let o = crate::errors::error_in(st);
                if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(o));
                }
            } else if cur_list_mode() == VMODE && cur_list_head() == cur_list_tail() {
                crate::errors::you_cant_in(st);
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"Sorry...I usually can't take things from the current page.".as_ptr();
                st.errors.help_line[0] = c"This \\lastbox will therefore be void.".as_ptr();
                let o = crate::errors::error_in(st);
                if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(o));
                }
            } else {
                let mut tx = cur_list_tail();
                if tx < st.mem.hi_mem_min
                    && st.mem.b16s1(tx) == MATH_NODE
                    && st.mem.b16s0(tx) == END_M_CODE
                {
                    let mut r = cur_list_head();
                    loop {
                        let prev = r;
                        r = st.mem.b32s1(prev);
                        if r == tx {
                            tx = prev;
                            break;
                        }
                    }
                }
                'last_box_done: {
                    if tx < st.mem.hi_mem_min {
                        let nt = st.mem.b16s1(tx);
                        if nt == HLIST_NODE || nt == VLIST_NODE {
                            let mut q = cur_list_head();
                            let mut p = TEX_NULL;
                            let mut r = TEX_NULL;
                            let mut fm = false;
                            loop {
                                r = p;
                                p = q;
                                fm = false;
                                if q < st.mem.hi_mem_min {
                                    if st.mem.b16s1(q) == DISC_NODE {
                                        for _ in 1..=st.mem.b16s0(q) {
                                            p = st.mem.b32s1(p);
                                        }
                                        if p == tx {
                                            break 'last_box_done;
                                        }
                                    } else if st.mem.b16s1(q) == MATH_NODE
                                        && st.mem.b16s0(q) == BEGIN_M_CODE
                                    {
                                        fm = true;
                                    }
                                }
                                q = st.mem.b32s1(p);
                                if q == tx {
                                    break;
                                }
                            }
                            let q2 = st.mem.b32s1(tx);
                            st.mem.set_b32s1(p, q2);
                            st.mem.set_b32s1(tx, TEX_NULL);
                            if q2 == TEX_NULL {
                                if fm {
                                    return Err(MainControlAbort::Confusion(c"tail1"));
                                } else {
                                    set_cur_list_tail(p);
                                }
                            } else if fm {
                                set_cur_list_tail(r);
                                st.mem.set_b32s1(r, TEX_NULL);
                                crate::packaging::flush_node_list(&mut st.mem, p)
                                    .map_err(MainControlAbort::Confusion)?;
                            }
                            st.scanner.cur_box = tx;
                            st.mem.set_b32s1(tx + 4, 0);
                        }
                    }
                }
            }
        }

        VSPLIT_CODE_I => {
            crate::scan::scan_register_num_in(st)?;
            let n = st.scanner.cur_val;
            if !crate::scan::scan_keyword_in(st, c"to".as_ptr())? {
                crate::output::error_here_with_diagnostic_in(st, c"Missing `to' inserted".as_ptr());
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"I'm working on `\\vsplit<box number> to <dimen>';".as_ptr();
                st.errors.help_line[0] = c"will look for the <dimen> next.".as_ptr();
                let o = crate::errors::error_in(st);
                if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(o));
                }
            }
            crate::scan::scan_dimen_in(st, false, false, false)?;
            st.scanner.cur_box = crate::packaging::vsplit_in(st, n, st.scanner.cur_val)?;
        }

        _ => {
            let k = st.scanner.cur_chr - 4;
            let sp = st.save_stack.save_ptr;
            st.save_stack.set_b32s1(sp, box_context);
            if k == HMODE as i32 {
                if box_context < BOX_FLAG && cur_list_mode().abs() == VMODE {
                    crate::scan::scan_spec_in(st, ADJUSTED_HBOX_GROUP, true)?;
                } else {
                    crate::scan::scan_spec_in(st, HBOX_GROUP, true)?;
                }
            } else {
                if k == VMODE as i32 {
                    crate::scan::scan_spec_in(st, VBOX_GROUP, true)?;
                } else {
                    crate::scan::scan_spec_in(st, VTOP_GROUP, true)?;
                    // k = VMODE for the nest push below
                }
                if let Some(a) = normal_paragraph(
                    &mut st.eqtb,
                    &mut st.save_stack,
                    &mut st.mem,
                    &mut st.scanner,
                ) {
                    return Err(MainControlAbort::Save(a));
                }
            }
            crate::mem::push_nest_in(st)?;
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
                let every_vbox = st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_VBOX);
                if every_vbox != TEX_NULL {
                    crate::input::begin_token_list_in(st, every_vbox, EVERY_VBOX_TEXT)?;
                }
            } else {
                set_cur_list_aux_b32s0(1000);
                let every_hbox = st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_HBOX);
                if every_hbox != TEX_NULL {
                    crate::input::begin_token_list_in(st, every_hbox, EVERY_HBOX_TEXT)?;
                }
            }
            return Ok(()); // no box_end call for new-group cases
        }
    }

    box_end_in(st, box_context)
}
// translated from xetex-xetex0.c:8995 (handle_right_brace)
pub(crate) fn handle_right_brace_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    use crate::globals::{cur_input, cur_list_head, cur_list_tail, set_cur_list_tail};
    use crate::mem::{
        delete_glue_ref, free_node, get_node, new_noad, pop_nest_in, ALLOC_FAILED, TEX_NULL,
    };
    use crate::packaging::flush_node_list;
    use crate::scan::NodeRecycleAbort;

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

    match st.save_stack.cur_group {
        SIMPLE_GROUP => {
            crate::save::unsave_in(st)?;
        }

        BOTTOM_LEVEL => {
            crate::output::error_here_with_diagnostic_in(st, c"Too many }'s".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 2;
            st.errors.help_line[1] = c"You've closed more groups than you opened.".as_ptr();
            st.errors.help_line[0] =
                c"Such booboos are generally harmless, so keep going.".as_ptr();
            mc_propagate_error(crate::errors::error_in(st))?;
        }

        SEMI_SIMPLE_GROUP | MATH_SHIFT_GROUP | MATH_LEFT_GROUP => {
            mc_propagate_error(crate::errors::extra_right_brace_in(st))?;
        }

        HBOX_GROUP => {
            package_in(st, 0)?;
        }

        ADJUSTED_HBOX_GROUP => {
            st.scanner.adjust_tail = ADJUST_HEAD;
            st.scanner.pre_adjust_tail = PRE_ADJUST_HEAD;
            package_in(st, 0)?;
        }

        VBOX_GROUP => {
            end_graf_in(st)?;
            package_in(st, 0)?;
        }

        VTOP_GROUP => {
            end_graf_in(st)?;
            package_in(st, VTOP_CODE)?;
        }

        INSERT_GROUP => {
            end_graf_in(st)?;
            // GLUEPAR(split_top_skip)
            let q = st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_SPLIT_TOP_SKIP);
            // GLUE_SPEC_ref_count(q)++
            st.mem.set_b32s1(q, st.mem.b32s1(q) + 1);
            let d = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_SPLIT_MAX_DEPTH);
            let f = st.eqtb.b32s1(INT_BASE + INT_PAR_FLOATING_PENALTY);
            crate::save::unsave_in(st)?;
            st.save_stack.save_ptr -= 2;
            let head = st.mem.b32s1(cur_list_head());
            let p = crate::packaging::vpackage_emit_in(st, head, 0, ADDITIONAL, MAX_HALFWORD)?;
            pop_nest_in(&mut st.mem, &mut st.input);

            if st.save_stack.b32s1(st.save_stack.save_ptr) < 255 {
                let new_node = get_node(&mut st.mem, INS_NODE_SIZE);
                if new_node == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow.into());
                }
                st.mem.set_b32s1(cur_list_tail(), new_node);
                set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
                let tail = cur_list_tail();
                st.mem.set_b16s1(tail, INS_NODE);
                let ins = match st.mem.classify(MemIdx::new(tail)) {
                    Ok(AnyNode::Ins(ins)) => ins,
                    _ => unreachable!("freshly stamped INS_NODE"),
                };
                let sp = st.save_stack.b32s1(st.save_stack.save_ptr) as u16;
                ins.set_subtype(&mut st.mem, sp);
                let hd = p.height(&st.mem).0 + p.depth(&st.mem).0;
                ins.set_height(&mut st.mem, Scaled(hd));
                let lp = p.list_ptr(&st.mem); // ins list ptr
                ins.set_ins_ptr(&mut st.mem, lp);
                ins.set_split_top_ptr(&mut st.mem, MemPtr::from_raw(q));
                ins.set_depth(&mut st.mem, Scaled(d));
                ins.set_float_cost(&mut st.mem, f);
            } else {
                let new_node = get_node(&mut st.mem, SMALL_NODE_SIZE);
                if new_node == ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow.into());
                }
                st.mem.set_b32s1(cur_list_tail(), new_node);
                set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
                let tail = cur_list_tail();
                st.mem.set_b16s1(tail, ADJUST_NODE);
                let adj = match st.mem.classify(MemIdx::new(tail)) {
                    Ok(AnyNode::Adjust(adj)) => adj,
                    _ => unreachable!("freshly stamped ADJUST_NODE"),
                };
                let sp = st.save_stack.b32s1(st.save_stack.save_ptr + 1) as u16;
                adj.set_subtype(&mut st.mem, sp);
                let lp = p.list_ptr(&st.mem);
                adj.set_adjust_ptr(&mut st.mem, lp);
                delete_glue_ref(&mut st.mem, q);
            }

            free_node(&mut st.mem, p.idx().0, BOX_NODE_SIZE);
            if st.input.nest_ptr == 0 {
                crate::pagebuilder::build_page_in(st)?;
            }
        }

        OUTPUT_GROUP => {
            if cur_input().loc != TEX_NULL
                || (cur_input().index != OUTPUT_TEXT && cur_input().index != BACKED_UP)
            {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Unbalanced output routine".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"Your sneaky output routine has problematic {'s and/or }'s.".as_ptr();
                st.errors.help_line[0] = c"I can't handle that very well; good luck.".as_ptr();
                mc_propagate_error(crate::errors::error_in(st))?;
                loop {
                    crate::scan::get_token_in(st)?;
                    if cur_input().loc == TEX_NULL {
                        break;
                    }
                }
            }

            crate::input::end_token_list_in(st)?;
            end_graf_in(st)?;
            crate::save::unsave_in(st)?;
            st.shipout.output_active = false;
            st.shipout.insert_penalties = 0;

            if st.eqtb.b32s1(BOX_BASE + 255) != TEX_NULL {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Output routine didn't use all of ".as_ptr(),
                );
                crate::output::print_esc_cstr_in(st, c"box".as_ptr());
                crate::output::print_int_in(st, 255);
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 3;
                st.errors.help_line[2] = c"Your \\output commands should empty \\box255,".as_ptr();
                st.errors.help_line[1] = c"e.g., by saying `\\shipout\\box255'.".as_ptr();
                st.errors.help_line[0] = c"Proceed; I'll discard its present contents.".as_ptr();
                mc_propagate_error(crate::errors::box_error_in(st, 255))?;
            }

            if cur_list_tail() != cur_list_head() {
                let head_link = st.mem.b32s1(cur_list_head());
                st.mem.set_b32s1(st.shipout.page_tail, head_link);
                st.shipout.page_tail = cur_list_tail();
            }

            if st.mem.b32s1(PAGE_HEAD) != TEX_NULL {
                if st.mem.b32s1(CONTRIB_HEAD) == TEX_NULL {
                    st.input.nest[0].tail = st.shipout.page_tail;
                }
                let contrib = st.mem.b32s1(CONTRIB_HEAD);
                st.mem.set_b32s1(st.shipout.page_tail, contrib);
                st.mem.set_b32s1(CONTRIB_HEAD, st.mem.b32s1(PAGE_HEAD));
                st.mem.set_b32s1(PAGE_HEAD, TEX_NULL);
                st.shipout.page_tail = PAGE_HEAD;
            }

            let dp = st.scanner.disc_ptr[LAST_BOX_CODE];
            flush_node_list(&mut st.mem, dp).map_err(MainControlAbort::Confusion)?;
            st.scanner.disc_ptr[LAST_BOX_CODE] = TEX_NULL;
            pop_nest_in(&mut st.mem, &mut st.input);
            crate::pagebuilder::build_page_in(st)?;
        }

        DISC_GROUP => {
            build_discretionary_in(st)?;
        }

        ALIGN_GROUP => {
            crate::input::back_input_in(st)?;
            st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_CR;
            crate::output::error_here_with_diagnostic_in(st, c"Missing ".as_ptr());
            crate::output::print_esc_cstr_in(st, c"cr".as_ptr());
            crate::output::print_cstr_in(st, c" inserted".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 1;
            st.errors.help_line[0] =
                c"I'm guessing that you meant to end an alignment here.".as_ptr();
            mc_propagate_error(crate::input::ins_error_in(st)?)?;
        }

        NO_ALIGN_GROUP => {
            end_graf_in(st)?;
            crate::save::unsave_in(st)?;
            align_peek_in(st)?;
        }

        VCENTER_GROUP => {
            end_graf_in(st)?;
            crate::save::unsave_in(st)?;
            st.save_stack.save_ptr -= 2;
            let head = st.mem.b32s1(cur_list_head());
            let w = st.save_stack.b32s1(st.save_stack.save_ptr + 1);
            let m = st.save_stack.b32s1(st.save_stack.save_ptr) as i16;
            let p = crate::packaging::vpackage_emit_in(st, head, w, m, MAX_HALFWORD)?;
            pop_nest_in(&mut st.mem, &mut st.input);
            let noad = new_noad(&mut st.mem);
            if noad == ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(cur_list_tail(), noad);
            set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
            st.mem.set_b16s1(cur_list_tail(), VCENTER_NOAD);
            st.mem.set_b32s1(cur_list_tail() + 1, SUB_BOX);
            st.mem.set_b32s0(cur_list_tail() + 1, p.idx().0);
        }

        MATH_CHOICE_GROUP => {
            crate::math::build_choices_in(st)?;
        }

        MATH_GROUP => {
            crate::save::unsave_in(st)?;
            st.save_stack.save_ptr -= 1;
            let save_loc = st.save_stack.b32s1(st.save_stack.save_ptr);
            st.mem.set_b32s1(save_loc, SUB_MLIST);
            let p = crate::math::fin_mlist_in(st, TEX_NULL)?;
            st.mem.set_b32s0(save_loc, p);

            if p != TEX_NULL && st.mem.b32s1(p) == TEX_NULL {
                if st.mem.b16s1(p) == ORD_NOAD
                    && st.mem.b32s1(p + 3) == EMPTY
                    && st.mem.b32s1(p + 2) == EMPTY
                {
                    // collapse to inline field
                    st.mem.set_b32s0(save_loc, st.mem.b32s0(p + 1));
                    st.mem.set_b32s1(save_loc, st.mem.b32s1(p + 1));
                    free_node(&mut st.mem, p, NOAD_SIZE);
                } else if st.mem.b16s1(p) == ACCENT_NOAD
                    && save_loc == cur_list_tail() + 1
                    && st.mem.b16s1(cur_list_tail()) == ORD_NOAD
                {
                    // merge accent noad into ord
                    let mut q = cur_list_head();
                    while st.mem.b32s1(q) != cur_list_tail() {
                        q = st.mem.b32s1(q);
                    }
                    st.mem.set_b32s1(q, p);
                    free_node(&mut st.mem, cur_list_tail(), NOAD_SIZE);
                    set_cur_list_tail(p);
                }
            }
        }

        _ => {
            return Err(MainControlAbort::Confusion(c"rightbrace"));
        }
    }
    Ok(())
}

// ---- moved verbatim from scan.rs (workstream 02) ----

// translated from xetex-xetex0.c:9358 (append_glue)
fn append_glue_node(
    mem: &mut crate::state::MemState,
    tail: i32,
    glue_spec: i32,
    subtype: u16,
    decrement_spec_ref: bool,
) -> i32 {
    let p_raw = crate::mem::new_glue(mem, glue_spec);
    if p_raw == crate::mem::ALLOC_FAILED {
        return p_raw;
    }
    let p = mem
        .classify_glue(MemIdx::new(p_raw))
        .expect("new_glue stamped a valid node")
        .expect("new_glue returned a glue node");
    mem.set_b32s1(tail, p.idx().0);
    if decrement_spec_ref {
        let spec = p.glue_spec(mem).expect("non-null glue spec");
        spec.set_ref_count(mem, spec.ref_count(mem) - 1);
        p.set_subtype(mem, subtype);
    }
    p.idx().0
}

pub(crate) fn append_glue_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    let s = st.scanner.cur_chr as i16;
    match s {
        0 => st.scanner.cur_val = 4,
        1 => st.scanner.cur_val = 8,
        2 => st.scanner.cur_val = 12,
        3 => st.scanner.cur_val = 16,
        4 => crate::scan::scan_glue_in(st, GLUE_VAL)?, // SKIP_CODE
        5 => crate::scan::scan_glue_in(st, MU_VAL_SCAN)?, // MSKIP_CODE
        _ => {}
    }
    let subtype = if s > SKIP_CODE { MU_GLUE } else { 0 };
    let tail = crate::globals::cur_list_tail();
    let p = append_glue_node(&mut st.mem, tail, st.scanner.cur_val, subtype, s >= SKIP_CODE);
    if p == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    crate::globals::set_cur_list_tail(p);
    Ok(())
}
// translated from xetex-xetex0.c:9395 (append_kern)
fn append_kern_node(mem: &mut crate::state::MemState, tail: i32, width: i32, subtype: u16) -> i32 {
    let p_raw = crate::mem::new_kern(mem, width);
    if p_raw == crate::mem::ALLOC_FAILED {
        return p_raw;
    }
    let p = mem
        .classify_kern(MemIdx::new(p_raw))
        .expect("new_kern stamped a valid node")
        .expect("new_kern returned a kern node");
    mem.set_b32s1(tail, p.idx().0);
    p.set_subtype(mem, subtype);
    p.idx().0
}

pub(crate) fn append_kern_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    let s = st.scanner.cur_chr as u16;
    crate::scan::scan_dimen_in(st, s == MU_GLUE, false, false)?;
    let tail = crate::globals::cur_list_tail();
    let p = append_kern_node(&mut st.mem, tail, st.scanner.cur_val, s);
    if p == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    crate::globals::set_cur_list_tail(p);
    Ok(())
}
pub(crate) fn make_mark_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    let c = if st.scanner.cur_chr == 0 {
        0
    } else {
        crate::scan::scan_register_num_in(st)?;
        st.scanner.cur_val
    };
    crate::scan::scan_toks_in(st, false, true)?; // side effect: sets def_ref; return value discarded
    let p = crate::mem::get_node(&mut st.mem, SMALL_NODE_SIZE);
    if p == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    st.mem.set_b16s1(p, MARK_NODE); // NODE_type(p)
    let mark = match st.mem.classify(MemIdx::new(p)) {
        Ok(AnyNode::Mark(mark)) => mark,
        _ => unreachable!("freshly stamped MARK_NODE"),
    };
    mark.set_mark_class(&mut st.mem, c);
    mark.set_subtype(&mut st.mem, 0);
    mark.set_mark_ptr(&mut st.mem, MemPtr::from_raw(st.scanner.def_ref));
    let tail = crate::globals::cur_list_tail();
    st.mem.set_b32s1(tail, p);
    crate::globals::set_cur_list_tail(p);
    Ok(())
}
pub(crate) fn new_interaction_in(st: &mut crate::state::EngineState) {
    crate::output::print_ln_in(st);
    st.errors.interaction = st.scanner.cur_chr as u8;
    if st.errors.interaction == BATCH_MODE {
        st.shipout.selector = SELECTOR_NO_PRINT;
    } else {
        st.shipout.selector = SELECTOR_TERM_ONLY;
    }
    if st.errors.log_opened {
        st.shipout.selector += 2;
    }
}
pub(crate) fn alter_aux_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.cur_chr != cur_list_mode().unsigned_abs() as i32 {
        return mc_propagate_error(crate::errors::report_illegal_case_in(st));
    }
    let c = st.scanner.cur_chr;
    crate::scan::scan_optional_equals_in(st)?;
    if c == VMODE as i32 {
        crate::scan::scan_dimen_in(st, false, false, false)?;
        crate::globals::set_cur_list_aux_b32s1(st.scanner.cur_val);
    } else {
        crate::scan::scan_int_in(st)?;
        if st.scanner.cur_val <= 0 || st.scanner.cur_val > 32767 {
            let errmsg =
                crate::output::error_here_with_diagnostic_in(st, c"Bad space factor".as_ptr());
            unsafe { ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), st.scanner.cur_val) };
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 1;
            st.errors.help_line[0] = c"I allow only values in the range 1..32767 here.".as_ptr();
            return mc_propagate_error(crate::errors::int_error_in(st, st.scanner.cur_val));
        }
        crate::globals::set_cur_list_aux_b32s0(st.scanner.cur_val);
    }
    Ok(())
}
pub(crate) fn alter_prev_graf_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    let np = st.input.nest_ptr;
    st.input.nest[np as usize] = crate::globals::cur_list();
    let mut p = np;
    while st.input.nest[p as usize].mode.unsigned_abs() != VMODE as u16 {
        p -= 1;
    }
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_int_in(st)?;
    if st.scanner.cur_val < 0 {
        let errmsg = crate::output::error_here_with_diagnostic_in(st, c"Bad ".as_ptr());
        unsafe { ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), st.scanner.cur_val) };
        crate::output::print_esc_cstr_in(st, c"prevgraf".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 1;
        st.errors.help_line[0] = c"I allow only nonnegative values here.".as_ptr();
        return mc_propagate_error(crate::errors::int_error_in(st, st.scanner.cur_val));
    }
    st.input.nest[p as usize].prev_graf = st.scanner.cur_val;
    crate::globals::set_cur_list_record(st.input.nest[np as usize]);
    Ok(())
}
pub(crate) fn alter_page_so_far_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    let c = st.scanner.cur_chr as usize;
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_dimen_in(st, false, false, false)?;
    st.shipout.page_so_far[c] = st.scanner.cur_val;
    Ok(())
}
pub(crate) fn alter_integer_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    let c = st.scanner.cur_chr as u8;
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_int_in(st)?;
    if c == 0 {
        st.shipout.dead_cycles = st.scanner.cur_val;
    } else if c == 2 {
        if st.scanner.cur_val < BATCH_MODE as i32 || st.scanner.cur_val > ERROR_STOP_MODE as i32 {
            let errmsg =
                crate::output::error_here_with_diagnostic_in(st, c"Bad interaction mode".as_ptr());
            unsafe { ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), st.scanner.cur_val) };
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 2;
            st.errors.help_line[1] = c"Modes are 0=batch, 1=nonstop, 2=scroll, and".as_ptr();
            st.errors.help_line[0] = c"3=errorstop. Proceed, and I'll ignore this case.".as_ptr();
            return mc_propagate_error(crate::errors::int_error_in(st, st.scanner.cur_val));
        }
        st.scanner.cur_chr = st.scanner.cur_val;
        new_interaction_in(st);
    } else {
        st.shipout.insert_penalties = st.scanner.cur_val;
    }
    Ok(())
}
pub(crate) fn alter_box_dimen_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    let c = st.scanner.cur_chr;
    crate::scan::scan_register_num_in(st)?;
    let b = if st.scanner.cur_val < 256 {
        st.eqtb.b32s1(BOX_BASE + st.scanner.cur_val)
    } else {
        let __cv13 = st.scanner.cur_val;
        crate::save::find_sa_element(&mut st.mem, &mut st.scanner, 4, __cv13, false);
        if st.scanner.cur_ptr == crate::mem::TEX_NULL {
            crate::mem::TEX_NULL
        } else {
            st.mem.b32s1(st.scanner.cur_ptr + 1)
        }
    };
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_dimen_in(st, false, false, false)?;
    if b != crate::mem::TEX_NULL {
        st.mem.set_b32s1(b + c, st.scanner.cur_val);
    }
    Ok(())
}
// translated from xetex-xetex0.c:10570 (trap_zero_glue)
///
/// Phase-C: threaded to `&mut MemState` (`01b` §3); legacy callers use
/// [`trap_zero_glue_global`]. No allocation, so no cold-path sentinel.
pub(crate) fn trap_zero_glue(st: &mut crate::state::EngineState) {
    let cv = st.scanner.cur_val;
    if st.mem.b32s1(cv + 1) == 0 && st.mem.b32s1(cv + 2) == 0 && st.mem.b32s1(cv + 3) == 0 {
        // GLUE_SPEC_ref_count(0)++ = mem[0].b32.s1++
        let r = st.mem.b32s1(0) + 1;
        st.mem.set_b32s1(0, r);
        crate::mem::delete_glue_ref(&mut st.mem, cv);
        st.scanner.cur_val = 0;
    }
}

// translated from xetex-xetex0.c:11357 (fix_language)
///
/// Phase-C: threaded to `&mut MemState` + `&EqtbState` (`01b` §3). The cold
/// arena-exhaustion branch of `new_whatsit` returns the `01b` §5
/// [`crate::scan::NodeRecycleAbort`] sentinel; [`fix_language_global`] runs the
/// `overflow` abort after the borrow drops.
pub(crate) fn fix_language(
    mem: &mut crate::state::MemState,
    eqtb: &crate::state::EqtbState,
) -> Result<(), crate::scan::NodeRecycleAbort> {
    let lang = eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE);
    let l = if lang <= 0 || lang > 255 { 0i32 } else { lang };
    if l != crate::globals::cur_list_aux_b32s1() {
        let p = crate::mem::new_whatsit(mem, LANGUAGE_NODE_SUBTYPE, SMALL_NODE_SIZE as i16);
        if p == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow);
        }
        let tail = crate::globals::cur_list_tail();
        mem.set_b32s1(tail + 1, l);
        crate::globals::set_cur_list_aux_b32s1(l);
        mem.set_b16s1(
            tail + 1,
            crate::helpers::norm_min(eqtb.b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as u16,
        );
        mem.set_b16s0(
            tail + 1,
            crate::helpers::norm_min(eqtb.b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as u16,
        );
    }
    Ok(())
}

pub(crate) fn off_save_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    if st.save_stack.cur_group == BOTTOM_LEVEL {
        crate::output::error_here_with_diagnostic_in(st, c"Extra ".as_ptr());
        crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 1;
        st.errors.help_line[0] =
            c"Things are pretty mixed up, but I think the worst is over.".as_ptr();
        mc_propagate_error(crate::errors::error_in(st))?;
    } else {
        crate::input::back_input_in(st)?;
        let p = crate::scan::get_avail_scc(st)?;
        st.mem.set_b32s1(TEMP_HEAD, p);
        crate::output::error_here_with_diagnostic_in(st, c"Missing ".as_ptr());
        match st.save_stack.cur_group {
            SEMI_SIMPLE_GROUP => {
                st.mem.set_b32s0(p, CS_TOKEN_FLAG + FROZEN_END_GROUP);
                crate::output::print_esc_cstr_in(st, c"endgroup".as_ptr());
            }
            MATH_SHIFT_GROUP => {
                st.mem.set_b32s0(p, MATH_SHIFT_TOKEN + b'$' as i32);
                crate::output::print_char_in(st, b'$' as i32);
            }
            MATH_LEFT_GROUP => {
                st.mem.set_b32s0(p, CS_TOKEN_FLAG + FROZEN_RIGHT);
                let q = crate::scan::get_avail_scc(st)?;
                st.mem.set_b32s1(p, q);
                let p2 = st.mem.b32s1(p); // = LLIST_link(p)
                st.mem.set_b32s0(p2, OTHER_TOKEN + b'.' as i32);
                crate::output::print_esc_cstr_in(st, c"right.".as_ptr());
            }
            _ => {
                st.mem.set_b32s0(p, RIGHT_BRACE_TOKEN + b'}' as i32);
                crate::output::print_char_in(st, b'}' as i32);
            }
        }
        crate::output::print_cstr_in(st, c" inserted".as_ptr());
        crate::input::begin_token_list_in(st, st.mem.b32s1(TEMP_HEAD), INSERTED)?;
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 5;
        st.errors.help_line[4] = c"I've inserted something that you may have forgotten.".as_ptr();
        st.errors.help_line[3] = c"(See the <inserted text> above.)".as_ptr();
        st.errors.help_line[2] = c"With luck, this will get me unwedged. But if you".as_ptr();
        st.errors.help_line[1] =
            c"really didn't forget anything, try typing `2' now; then".as_ptr();
        st.errors.help_line[0] =
            c"my insertion and my current dilemma will both disappear.".as_ptr();
        mc_propagate_error(crate::errors::error_in(st))?;
    }
    Ok(())
}
pub(crate) fn append_discretionary_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    let tail = crate::globals::cur_list_tail();
    let d = crate::mem::new_disc(&mut st.mem);
    if d == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    st.mem.set_b32s1(tail, d);
    crate::globals::set_cur_list_tail(st.mem.b32s1(tail));

    if st.scanner.cur_chr == 1 {
        let f = st.eqtb.b32s1(CUR_FONT_LOC);
        let c = st.fonts.hyphen_char[f as usize];
        if (0..=BIGGEST_CHAR).contains(&c) {
            let ch = new_character_in(st, f, c as u16)?;
            let tail2 = crate::globals::cur_list_tail();
            st.mem.set_b32s0(tail2 + 1, ch);
        }
    } else {
        let sp = st.save_stack.save_ptr;
        st.save_stack.set_b32s1(sp, 0);
        st.save_stack.save_ptr = sp + 1;
        if let Some(a) = crate::save::new_save_level(&mut st.save_stack, DISC_GROUP, st.mem.line) {
            return Err(MainControlAbort::Save(a));
        }
        crate::scan::scan_left_brace_in(st)?;
        crate::mem::push_nest_in(st)?;
        crate::globals::set_cur_list_mode(-104); // -HMODE
        crate::globals::set_cur_list_aux_b32s0(1000);
    }
    Ok(())
}
// translated from xetex-xetex0.c:10765 (shift_case)
pub(crate) fn shift_case_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    let b = st.scanner.cur_chr;
    crate::scan::scan_toks_in(st, false, false)?;
    let mut p = st.mem.b32s1(st.scanner.def_ref);
    while p != crate::mem::TEX_NULL {
        let t = st.mem.b32s0(p);
        if t < CS_TOKEN_FLAG + SINGLE_BASE {
            let c = t % MAX_CHAR_VAL;
            if st.eqtb.b32s1(b + c) != 0 {
                st.mem.set_b32s0(p, t - c + st.eqtb.b32s1(b + c));
            }
        }
        p = st.mem.b32s1(p);
    }
    let def_ref = st.scanner.def_ref;
    crate::input::begin_token_list_in(st, st.mem.b32s1(def_ref), BACKED_UP)?;
    st.mem.set_b32s1(def_ref, st.mem.avail);
    st.mem.avail = def_ref;
    Ok(())
}
// translated from xetex-xetex0.c:9958 (build_discretionary)
/// `&mut EngineState` core of [`build_discretionary`]: validates and stitches the
/// three discretionary sublists under one borrow. `cur_list_*` stay raw `mod
/// raw`; the diagnostics/error/scan callees run threaded. Cold aborts ride
/// [`MainControlAbort`].
///
/// translated from xetex-xetex0.c:9958 (build_discretionary)
pub(crate) fn build_discretionary_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    use crate::globals::{
        cur_list_head, cur_list_mode, cur_list_tail, set_cur_list_aux_b32s0, set_cur_list_mode,
        set_cur_list_tail,
    };
    use crate::mem::TEX_NULL;

    crate::save::unsave_in(st)?;
    let q_head = cur_list_head();
    let mut q = q_head;
    let mut p = st.mem.b32s1(q);
    let mut n: i32 = 0;

    'walk: loop {
        if p == TEX_NULL {
            break;
        }
        if p < st.mem.hi_mem_min {
            // not a char node — check node type
            let ntype = st.mem.b16s1(p);
            if ntype > RULE_NODE {
                let ok = ntype == KERN_NODE   // 11
                    || ntype == LIGATURE_NODE  // 6
                    || ntype == WHATSIT_NODE && {
                        let sub = st.mem.b16s0(p);
                        sub == NATIVE_WORD_NODE || sub == NATIVE_WORD_NODE_AT || sub == GLYPH_NODE_BD
                    };
                if !ok {
                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"Improper discretionary list".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 1;
                    st.errors.help_line[0] =
                        c"Discretionary lists must contain only boxes and kerns.".as_ptr();
                    let o = crate::errors::error_in(st);
                    if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                        return Err(MainControlAbort::Error(o));
                    }

                    crate::output::begin_diagnostic_in(st);
                    crate::output::diagnostic_begin_capture_warning_here_in(st);
                    crate::output::print_nl_cstr_in(
                        st,
                        c"The following discretionary sublist has been deleted:".as_ptr(),
                    );
                    crate::output::show_box_in(st, p);
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    crate::output::end_diagnostic_in(st, true);

                    crate::packaging::flush_node_list(&mut st.mem, p)
                        .map_err(MainControlAbort::Confusion)?;
                    st.mem.set_b32s1(q, TEX_NULL);
                    break 'walk;
                }
            }
        }
        q = p;
        p = st.mem.b32s1(q);
        n += 1;
    }

    p = st.mem.b32s1(q_head);
    crate::mem::pop_nest_in(&mut st.mem, &mut st.input);

    let slot = st.save_stack.b32s1(st.save_stack.save_ptr - 1);
    match slot {
        0 => {
            st.mem.set_b32s0(cur_list_tail() + 1, p);
        }
        1 => {
            st.mem.set_b32s1(cur_list_tail() + 1, p);
        }
        2 => {
            if n > 0 && cur_list_mode().abs() == MMODE {
                crate::output::error_here_with_diagnostic_in(st, c"Illegal math ".as_ptr());
                crate::output::print_esc_cstr_in(st, c"discretionary".as_ptr());
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"Sorry: The third part of a discretionary break must be".as_ptr();
                st.errors.help_line[0] =
                    c"empty, in math formulas. I had to delete your third part.".as_ptr();
                crate::packaging::flush_node_list(&mut st.mem, p)
                    .map_err(MainControlAbort::Confusion)?;
                n = 0;
                let o = crate::errors::error_in(st);
                if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(o));
                }
            } else {
                st.mem.set_b32s1(cur_list_tail(), p);
            }
            if n <= u16::MAX as i32 {
                st.mem.set_b16s0(cur_list_tail(), n as u16);
            } else {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Discretionary list is too long".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"Wow---I never thought anybody would tweak me here.".as_ptr();
                st.errors.help_line[0] =
                    c"You can't seriously need such a huge discretionary list?".as_ptr();
                let o = crate::errors::error_in(st);
                if !matches!(o, crate::errors::ErrorOutcome::Continue) {
                    return Err(MainControlAbort::Error(o));
                }
            }
            if n > 0 {
                set_cur_list_tail(q);
            }
            st.save_stack.save_ptr -= 1;
            return Ok(());
        }
        _ => {}
    }
    st.save_stack
        .set_b32s1(st.save_stack.save_ptr - 1, slot + 1);
    if let Some(a) = crate::save::new_save_level(&mut st.save_stack, DISC_GROUP, st.mem.line) {
        return Err(MainControlAbort::Save(a));
    }
    crate::scan::scan_left_brace_in(st)?;
    crate::mem::push_nest_in(st)?;
    set_cur_list_mode(-104); // -HMODE
    set_cur_list_aux_b32s0(1000);
    Ok(())
}
pub(crate) fn append_italic_correction_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const EXPLICIT: u16 = 1; // xetex_format.h:428
    const GLYPH_NODE: u16 = 42; // xetex_format.h:465

    // get_native_italic_correction(p) = real_get_native_italic_correction(&mem[p])
    let native_kern = |st: &mut crate::state::EngineState,
                       tail: i32,
                       glyph: bool|
     -> Result<(), MainControlAbort> {
        let node_addr =
            unsafe { st.mem.data.as_mut_ptr().add(tail as usize) as *mut std::ffi::c_void };
        let ic = unsafe {
            if glyph {
                crate::ext::real_get_native_glyph_italic_correction(&st.fonts, node_addr)
            } else {
                crate::ext::real_get_native_italic_correction(&st.fonts, node_addr)
            }
        };
        let kern = crate::mem::new_kern(&mut st.mem, ic);
        if kern == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b32s1(tail, kern);
        crate::globals::set_cur_list_tail(kern);
        st.mem.set_b16s0(kern, EXPLICIT);
        Ok(())
    };

    let tail = crate::globals::cur_list_tail();
    if tail == crate::globals::cur_list_head() {
        return Ok(());
    }

    let p = if tail >= st.mem.hi_mem_min {
        // is_char_node
        tail
    } else if st.mem.b16s1(tail) == LIGATURE_NODE {
        tail + 1
    } else if st.mem.b16s1(tail) == WHATSIT_NODE {
        let sub = st.mem.b16s0(tail);
        if sub == NATIVE_WORD_NODE || sub == NATIVE_WORD_NODE_AT {
            native_kern(st, tail, false)?;
        } else if sub == GLYPH_NODE {
            native_kern(st, tail, true)?;
        }
        return Ok(());
    } else {
        return Ok(());
    };

    let f = st.mem.b16s1(p) as i32; // CHAR_NODE_font(p)
    let c = crate::fonts::effective_char(&mut st.fonts, true, f, st.mem.b16s0(p));
    let info = st.fonts.font_info_b16(st.fonts.char_base_entry(f) + c);
    let italic = st
        .fonts
        .font_info_b32s1(st.fonts.italic_base_entry(f) + (info.s1 / 4) as i32);
    let kern = crate::mem::new_kern(&mut st.mem, italic);
    if kern == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    let tail2 = crate::globals::cur_list_tail();
    st.mem.set_b32s1(tail2, kern);
    crate::globals::set_cur_list_tail(kern);
    st.mem.set_b16s0(kern, EXPLICIT);
    Ok(())
}
// translated from xetex-xetex0.c:9910 (make_accent)
pub(crate) fn make_accent_in(st: &mut crate::state::EngineState) -> Result<(), MainControlAbort> {
    const SLANT_CODE: i32 = 1; // xetex-constants.h:273
    const X_HEIGHT_CODE: i32 = 5; // xetex-constants.h:294
    const ACC_KERN: u16 = 2; // xetex_format.h:429
    const ADDITIONAL: i16 = 1; // xetex-constants.h:264
    const LETTER: u8 = 11; // xetex_format.h:243
    const CHAR_NUM: u8 = 16; // xetex_format.h:253

    fn is_native(fonts: &crate::state::FontState, fnt: i32) -> bool {
        fonts.font_area_entry(fnt) == AAT_FONT_FLAG || fonts.font_area_entry(fnt) == OTGR_FONT_FLAG
    }

    crate::scan::scan_char_num_in(st)?;
    let mut f = st.eqtb.b32s1(CUR_FONT_LOC);
    let p = new_character_in(st, f, st.scanner.cur_val as u16)?;

    if p == crate::mem::TEX_NULL {
        return Ok(());
    }

    let x = st
        .fonts
        .font_info_b32s1(X_HEIGHT_CODE + st.fonts.param_base(f));
    let s = st
        .fonts
        .font_info_b32s1(SLANT_CODE + st.fonts.param_base(f)) as f64
        / 65536.0;
    let mut lsb: i32 = 0;
    let mut rsb: i32 = 0;
    let a = if is_native(&st.fonts, f) {
        let av = st.mem.b32s1(p + 1);
        if av == 0 {
            unsafe {
                crate::ext::get_native_char_sidebearings(
                    &st.fonts,
                    f,
                    st.scanner.cur_val,
                    &mut lsb,
                    &mut rsb,
                )?;
            }
        }
        av
    } else {
        let ch = st.mem.b16s0(p);
        let c = crate::fonts::effective_char(&mut st.fonts, true, f, ch);
        let info = st.fonts.font_info_b16(st.fonts.char_base_entry(f) + c);
        st.fonts
            .font_info_b32s1(st.fonts.width_base_entry(f) + info.s3 as i32)
    };

    do_assignments_in(st)?;
    let mut q = crate::mem::TEX_NULL;
    f = st.eqtb.b32s1(CUR_FONT_LOC);
    if st.scanner.cur_cmd == LETTER || st.scanner.cur_cmd == OTHER_CHAR || st.scanner.cur_cmd == CHAR_GIVEN {
        q = new_character_in(st, f, st.scanner.cur_chr as u16)?;
        st.scanner.cur_val = st.scanner.cur_chr;
    } else if st.scanner.cur_cmd == CHAR_NUM {
        crate::scan::scan_char_num_in(st)?;
        q = new_character_in(st, f, st.scanner.cur_val as u16)?;
    } else {
        crate::input::back_input_in(st)?;
    }

    if q != crate::mem::TEX_NULL {
        let t = st
            .fonts
            .font_info_b32s1(SLANT_CODE + st.fonts.param_base(f)) as f64
            / 65536.0;
        let (w, h) = if is_native(&st.fonts, f) {
            let wv = st.mem.b32s1(q + 1);
            let mut hv: i32 = 0;
            let mut dv: i32 = 0;
            unsafe {
                crate::ext::get_native_char_height_depth(
                    &st.fonts,
                    f,
                    st.scanner.cur_val,
                    &mut hv,
                    &mut dv,
                )?;
            }
            (wv, hv)
        } else {
            let ch = st.mem.b16s0(q);
            let c = crate::fonts::effective_char(&mut st.fonts, true, f, ch);
            let info = st.fonts.font_info_b16(st.fonts.char_base_entry(f) + c);
            let wv = st
                .fonts
                .font_info_b32s1(st.fonts.width_base_entry(f) + info.s3 as i32);
            let hv = st
                .fonts
                .font_info_b32s1(st.fonts.height_base_entry(f) + (info.s2 / 16) as i32);
            (wv, hv)
        };
        let mut pa = p;
        if h != x {
            pa = crate::packaging::hpack_emit_in(st, p, 0, ADDITIONAL)?
                .idx()
                .0;
            st.mem.set_b32s1(pa + 4, x - h);
        }
        let delta = if is_native(&st.fonts, f) && a == 0 {
            crate::scaled_math::tex_round(
                (w - lsb + rsb) as f64 / 2.0 + h as f64 * t - x as f64 * s,
            )
        } else {
            crate::scaled_math::tex_round((w - a) as f64 / 2.0 + h as f64 * t - x as f64 * s)
        };
        let r = crate::mem::new_kern(&mut st.mem, delta);
        if r == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
        }
        st.mem.set_b16s0(r, ACC_KERN);
        st.mem.set_b32s1(crate::globals::cur_list_tail(), r);
        st.mem.set_b32s1(r, pa);
        let new_tail = crate::mem::new_kern(&mut st.mem, -a - delta);
        if new_tail == crate::mem::ALLOC_FAILED {
            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
        }
        crate::globals::set_cur_list_tail(new_tail);
        st.mem.set_b16s0(new_tail, ACC_KERN);
        st.mem.set_b32s1(pa, new_tail);
        // p = q (in C); we use q directly below
    }

    // Append p (q if base was found, else accent) to hlist.
    let p_final = if q != crate::mem::TEX_NULL { q } else { p };
    st.mem.set_b32s1(crate::globals::cur_list_tail(), p_final);
    crate::globals::set_cur_list_tail(p_final);
    crate::globals::set_cur_list_aux_b32s0(1000);
    Ok(())
}
pub(crate) unsafe fn new_font_in(
    st: &mut crate::state::EngineState,
    a: i16,
) -> Result<(), MainControlAbort> {
    if st.strings.job_name == 0 {
        crate::io::open_log_file_in(st)?;
    }

    crate::scan::get_r_token_in(st)?;
    let u = st.scanner.cur_cs;
    let t = if u >= HASH_BASE {
        st.eqtb.hash_s1(u)
    } else if u >= SINGLE_BASE {
        if u == NULL_CS {
            crate::texmfmp::maketexstring_in(&mut st.strings, c"FONT".as_ptr())?
        } else {
            u - SINGLE_BASE
        }
    } else {
        let old_setting = st.shipout.selector;
        st.shipout.selector = SELECTOR_NEW_STRING;
        crate::output::print_cstr_in(st, c"FONT".as_ptr());
        crate::output::print_in(st, u - 1);
        st.shipout.selector = old_setting;
        if st.strings.pool_ptr + 1 > st.strings.pool_size {
            return Err(MainControlAbort::Input(
                crate::input::InputAbort::Overflow {
                    what: c"pool size".as_ptr(),
                    n: st.strings.pool_size - st.strings.init_pool_ptr,
                },
            ));
        }
        match crate::string_pool::make_string(&mut st.strings) {
            Ok(s) => s,
            Err(crate::scan::NodeRecycleAbort::Overflow) => {
                return Err(MainControlAbort::Input(
                    crate::input::InputAbort::Overflow {
                        what: c"number of strings".as_ptr(),
                        n: st.strings.max_strings - st.strings.init_str_ptr,
                    },
                ));
            }
            Err(e) => return Err(MainControlAbort::NodeRecycle(e)),
        }
    };

    if a >= 4 {
        if let Some(ab) = crate::save::geq_define(
            &mut st.eqtb,
            &mut st.mem,
            &mut st.scanner,
            u,
            SET_FONT as u16,
            FONT_BASE,
        ) {
            return Err(MainControlAbort::Save(ab));
        }
    } else {
        if let Some(ab) = crate::save::eq_define(
            &mut st.eqtb,
            &mut st.save_stack,
            &mut st.mem,
            &mut st.scanner,
            u,
            SET_FONT as u16,
            FONT_BASE,
        ) {
            return Err(MainControlAbort::Save(ab));
        }
    }
    crate::scan::scan_optional_equals_in(st)?;
    crate::io::scan_file_name_in(st)?;
    st.scanner.name_in_progress = true;

    let s = if crate::scan::scan_keyword_in(st, c"at".as_ptr())? {
        crate::scan::scan_dimen_in(st, false, false, false)?;
        let sv = st.scanner.cur_val;
        if sv <= 0 || sv >= 0x800_0000 {
            crate::output::error_here_with_diagnostic_in(st, c"Improper `at' size (".as_ptr());
            crate::output::print_scaled_in(st, sv);
            crate::output::print_cstr_in(st, c"pt), replaced by 10pt".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 2;
            st.errors.help_line[1] = c"I can only handle fonts at positive sizes that are".as_ptr();
            st.errors.help_line[0] =
                c"less than 2048pt, so I've changed what you said to 10pt.".as_ptr();
            mc_propagate_error(crate::errors::error_in(st))?;
            10 * 65536_i32
        } else {
            sv
        }
    } else if crate::scan::scan_keyword_in(st, c"scaled".as_ptr())? {
        crate::scan::scan_int_in(st)?;
        let sv = st.scanner.cur_val;
        if sv <= 0 || sv > 32768 {
            let errmsg = crate::output::error_here_with_diagnostic_in(
                st,
                c"Illegal magnification has been changed to 1000".as_ptr(),
            );
            ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), sv);
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 1;
            st.errors.help_line[0] =
                c"The magnification ratio must be between 1 and 32768.".as_ptr();
            mc_propagate_error(crate::errors::int_error_in(st, sv))?;
            -1000_i32
        } else {
            -sv
        }
    } else {
        -1000_i32
    };

    st.scanner.name_in_progress = false;

    let f = 'search: {
        let mut fi = FONT_BASE + 1;
        while fi <= st.fonts.font_ptr {
            if crate::string_pool::str_eq_str(
                &st.strings,
                st.fonts.font_name[fi as usize],
                st.scanner.cur_name,
            ) && ((crate::string_pool::length(&st.strings, st.scanner.cur_area) == 0
                && (st.fonts.font_area[fi as usize] == AAT_FONT_FLAG
                    || st.fonts.font_area[fi as usize] == OTGR_FONT_FLAG))
                || crate::string_pool::str_eq_str(
                    &st.strings,
                    st.fonts.font_area[fi as usize],
                    st.scanner.cur_area,
                ))
            {
                if s > 0 {
                    if s == st.fonts.font_size[fi as usize] {
                        break 'search fi;
                    }
                } else if st.fonts.font_size[fi as usize]
                    == crate::scaled_math::xn_over_d_in(
                        &mut st.errors,
                        st.fonts.font_dsize[fi as usize],
                        -s,
                        1000,
                    )
                {
                    break 'search fi;
                }
            }
            crate::string_pool::append_str(&mut st.strings, st.scanner.cur_area).map_err(|e| {
                match e {
                    crate::scan::NodeRecycleAbort::Overflow => {
                        MainControlAbort::Input(crate::input::InputAbort::Overflow {
                            what: c"pool size".as_ptr(),
                            n: st.strings.pool_size - st.strings.init_pool_ptr,
                        })
                    }
                    e => MainControlAbort::NodeRecycle(e),
                }
            })?;
            crate::string_pool::append_str(&mut st.strings, st.scanner.cur_name).map_err(|e| {
                match e {
                    crate::scan::NodeRecycleAbort::Overflow => {
                        MainControlAbort::Input(crate::input::InputAbort::Overflow {
                            what: c"pool size".as_ptr(),
                            n: st.strings.pool_size - st.strings.init_pool_ptr,
                        })
                    }
                    e => MainControlAbort::NodeRecycle(e),
                }
            })?;
            crate::string_pool::append_str(&mut st.strings, st.scanner.cur_ext).map_err(
                |e| match e {
                    crate::scan::NodeRecycleAbort::Overflow => {
                        MainControlAbort::Input(crate::input::InputAbort::Overflow {
                            what: c"pool size".as_ptr(),
                            n: st.strings.pool_size - st.strings.init_pool_ptr,
                        })
                    }
                    e => MainControlAbort::NodeRecycle(e),
                },
            )?;
            let ms = match crate::string_pool::make_string(&mut st.strings) {
                Ok(ms) => ms,
                Err(crate::scan::NodeRecycleAbort::Overflow) => {
                    return Err(MainControlAbort::Input(
                        crate::input::InputAbort::Overflow {
                            what: c"number of strings".as_ptr(),
                            n: st.strings.max_strings - st.strings.init_str_ptr,
                        },
                    ));
                }
                Err(e) => return Err(MainControlAbort::NodeRecycle(e)),
            };
            if crate::string_pool::str_eq_str(&st.strings, st.fonts.font_name[fi as usize], ms) {
                st.strings.str_ptr -= 1;
                st.strings.pool_ptr =
                    st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
                if st.fonts.font_area[fi as usize] == AAT_FONT_FLAG
                    || st.fonts.font_area[fi as usize] == OTGR_FONT_FLAG
                {
                    if s > 0 {
                        if s == st.fonts.font_size[fi as usize] {
                            break 'search fi;
                        }
                    } else if st.fonts.font_size[fi as usize]
                        == crate::scaled_math::xn_over_d_in(
                            &mut st.errors,
                            st.fonts.font_dsize[fi as usize],
                            -s,
                            1000,
                        )
                    {
                        break 'search fi;
                    }
                }
            } else {
                st.strings.str_ptr -= 1;
                st.strings.pool_ptr =
                    st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            }
            fi += 1;
        }
        read_font_info_in(st, u, st.scanner.cur_name, st.scanner.cur_area, s)?
    };

    // common_ending:
    if a >= 4 {
        if let Some(ab) = crate::save::geq_define(
            &mut st.eqtb,
            &mut st.mem,
            &mut st.scanner,
            u,
            SET_FONT as u16,
            f,
        ) {
            return Err(MainControlAbort::Save(ab));
        }
    } else {
        if let Some(ab) = crate::save::eq_define(
            &mut st.eqtb,
            &mut st.save_stack,
            &mut st.mem,
            &mut st.scanner,
            u,
            SET_FONT as u16,
            f,
        ) {
            return Err(MainControlAbort::Save(ab));
        }
    }
    st.eqtb.data[(FONT_ID_BASE + f) as usize] = st.eqtb.word(u);
    st.eqtb.set_hash_s1(FONT_ID_BASE + f, t);
    Ok(())
}
// translated from xetex-xetex0.c:10220 (do_register_command)
pub(crate) fn do_register_command_in(
    st: &mut crate::state::EngineState,
    a: i16,
) -> Result<(), MainControlAbort> {
    const ADVANCE: u8 = 92; // xetex_format.h:332
    const MULTIPLY: u8 = 93; // xetex_format.h:334
    const GLUE_REF: u16 = 119; // xetex_format.h:361
    const NORMAL: u16 = 0; // xetex_format.h:420

    let q = st.scanner.cur_cmd;
    let mut w: i32 = 0;
    let mut s: i32 = crate::mem::TEX_NULL;

    // Determine (l, p, e): register address, value type, is_extended_SA
    let (l, p, e) = 'determine: {
        if q != REGISTER {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd >= ASSIGN_INT && st.scanner.cur_cmd <= ASSIGN_MU_GLUE {
                break 'determine (st.scanner.cur_chr, (st.scanner.cur_cmd - ASSIGN_INT) as i16, false);
            }
            if st.scanner.cur_cmd != REGISTER {
                crate::output::error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
                crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
                crate::output::print_cstr_in(st, c"' after ".as_ptr());
                crate::output::print_cmd_chr_in(st, q as u16, 0);
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 1;
                st.errors.help_line[0] =
                    c"I'm forgetting what you said and not changing anything.".as_ptr();
                return mc_propagate_error(crate::errors::error_in(st));
            }
        }
        if st.scanner.cur_chr < 0 || st.scanner.cur_chr > 19 {
            let l = st.scanner.cur_chr;
            let p = (st.mem.b16s1(l) / 64) as i16;
            (l, p, true)
        } else {
            let p = st.scanner.cur_chr as i16;
            crate::scan::scan_register_num_in(st)?;
            if st.scanner.cur_val > 255 {
                let __cv12 = st.scanner.cur_val;
                crate::save::find_sa_element(
                    &mut st.mem,
                    &mut st.scanner,
                    p as u16,
                    __cv12,
                    true,
                );
                (st.scanner.cur_ptr, p, true)
            } else {
                let l = match p {
                    INT_VAL => st.scanner.cur_val + COUNT_BASE,
                    DIMEN_VAL => st.scanner.cur_val + SCALED_BASE,
                    GLUE_VAL => st.scanner.cur_val + SKIP_BASE,
                    MU_VAL => st.scanner.cur_val + MU_SKIP_BASE,
                    _ => unreachable!(),
                };
                (l, p, false)
            }
        }
    };

    // found: read current value into w (int/dimen) or s (glue/mu)
    if p < GLUE_VAL {
        w = if e {
            st.mem.b32s1(l + 2)
        } else {
            unsafe { st.eqtb.data[l as usize].b32.s1 }
        };
    } else if e {
        s = st.mem.b32s1(l + 1);
    } else {
        s = unsafe { st.eqtb.data[l as usize].b32.s1 };
    }

    if q == REGISTER {
        crate::scan::scan_optional_equals_in(st)?;
    } else {
        crate::scan::scan_keyword_in(st, c"by".as_ptr())?;
    }

    st.errors.arith_error = false;

    if q < MULTIPLY {
        if p < GLUE_VAL {
            if p == INT_VAL {
                crate::scan::scan_int_in(st)?;
            } else {
                crate::scan::scan_dimen_in(st, false, false, false)?;
            }
            if q == ADVANCE {
                st.scanner.cur_val = st.scanner.cur_val + w;
            }
        } else {
            crate::scan::scan_glue_in(st, p)?;
            if q == ADVANCE {
                let qs = crate::mem::new_spec(&mut st.mem, st.scanner.cur_val);
                if qs == crate::mem::ALLOC_FAILED {
                    return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                }
                let r = s;
                crate::mem::delete_glue_ref(&mut st.mem, st.scanner.cur_val);
                let qs_p = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(qs))
                    .expect("non-null glue spec");
                let r_p = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(r))
                    .expect("non-null glue spec");
                let w = qs_p.width(&st.mem).0 + r_p.width(&st.mem).0;
                qs_p.set_width(&mut st.mem, Scaled(w));
                if qs_p.stretch(&st.mem).0 == 0 {
                    qs_p.set_stretch_order(&mut st.mem, NORMAL);
                }
                if qs_p.stretch_order(&st.mem) == r_p.stretch_order(&st.mem) {
                    let v = qs_p.stretch(&st.mem).0 + r_p.stretch(&st.mem).0;
                    qs_p.set_stretch(&mut st.mem, Scaled(v));
                } else if qs_p.stretch_order(&st.mem) < r_p.stretch_order(&st.mem)
                    && r_p.stretch(&st.mem).0 != 0
                {
                    let (rs, ro) = (r_p.stretch(&st.mem), r_p.stretch_order(&st.mem));
                    qs_p.set_stretch(&mut st.mem, rs);
                    qs_p.set_stretch_order(&mut st.mem, ro);
                }
                if qs_p.shrink(&st.mem).0 == 0 {
                    qs_p.set_shrink_order(&mut st.mem, NORMAL);
                }
                if qs_p.shrink_order(&st.mem) == r_p.shrink_order(&st.mem) {
                    let v = qs_p.shrink(&st.mem).0 + r_p.shrink(&st.mem).0;
                    qs_p.set_shrink(&mut st.mem, Scaled(v));
                } else if qs_p.shrink_order(&st.mem) < r_p.shrink_order(&st.mem)
                    && r_p.shrink(&st.mem).0 != 0
                {
                    let (rs, ro) = (r_p.shrink(&st.mem), r_p.shrink_order(&st.mem));
                    qs_p.set_shrink(&mut st.mem, rs);
                    qs_p.set_shrink_order(&mut st.mem, ro);
                }
                st.scanner.cur_val = qs;
            }
        }
    } else {
        crate::scan::scan_int_in(st)?;
        if p < GLUE_VAL {
            if q == MULTIPLY {
                if p == INT_VAL {
                    st.scanner.cur_val = crate::scaled_math::mult_and_add_in(
                        &mut st.errors,
                        w,
                        st.scanner.cur_val,
                        0,
                        TEX_INFINITY,
                    );
                } else {
                    st.scanner.cur_val = crate::scaled_math::mult_and_add_in(
                        &mut st.errors,
                        w,
                        st.scanner.cur_val,
                        0,
                        MAX_HALFWORD,
                    );
                }
            } else {
                st.scanner.cur_val = crate::scaled_math::x_over_n_in(
                    &mut st.errors,
                    w,
                    st.scanner.cur_val,
                );
            }
        } else {
            let r = crate::mem::new_spec(&mut st.mem, s);
            if r == crate::mem::ALLOC_FAILED {
                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
            }
            let s_p = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(s))
                .expect("non-null glue spec");
            let r_p = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(r))
                .expect("non-null glue spec");
            let (sw, sst, ssh) = (
                s_p.width(&st.mem),
                s_p.stretch(&st.mem),
                s_p.shrink(&st.mem),
            );
            if q == MULTIPLY {
                let v = crate::scaled_math::mult_and_add_in(
                    &mut st.errors,
                    sw.0,
                    st.scanner.cur_val,
                    0,
                    MAX_HALFWORD,
                );
                r_p.set_width(&mut st.mem, Scaled(v));
                let v = crate::scaled_math::mult_and_add_in(
                    &mut st.errors,
                    sst.0,
                    st.scanner.cur_val,
                    0,
                    MAX_HALFWORD,
                );
                r_p.set_stretch(&mut st.mem, Scaled(v));
                let v = crate::scaled_math::mult_and_add_in(
                    &mut st.errors,
                    ssh.0,
                    st.scanner.cur_val,
                    0,
                    MAX_HALFWORD,
                );
                r_p.set_shrink(&mut st.mem, Scaled(v));
            } else {
                let v = crate::scaled_math::x_over_n_in(&mut st.errors, sw.0, st.scanner.cur_val);
                r_p.set_width(&mut st.mem, Scaled(v));
                let v = crate::scaled_math::x_over_n_in(&mut st.errors, sst.0, st.scanner.cur_val);
                r_p.set_stretch(&mut st.mem, Scaled(v));
                let v = crate::scaled_math::x_over_n_in(&mut st.errors, ssh.0, st.scanner.cur_val);
                r_p.set_shrink(&mut st.mem, Scaled(v));
            }
            st.scanner.cur_val = r;
        }
    }

    if st.errors.arith_error {
        crate::output::error_here_with_diagnostic_in(st, c"Arithmetic overflow".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] = c"I can't carry out that multiplication or division,".as_ptr();
        st.errors.help_line[0] = c"since the result is out of range.".as_ptr();
        if p >= GLUE_VAL {
            crate::mem::delete_glue_ref(&mut st.mem, st.scanner.cur_val);
        }
        return mc_propagate_error(crate::errors::error_in(st));
    }

    if p < GLUE_VAL {
        if e {
            let result = if a >= 4 {
                crate::save::gsa_w_def(&mut st.mem, l, st.scanner.cur_val)
            } else {
                let __cv11 = st.scanner.cur_val;
                crate::save::sa_w_def(
                    &mut st.mem,
                    &mut st.scanner,
                    &mut st.save_stack,
                    l,
                    __cv11,
                )
            };
            crate::save::finish_sa_mutation_in(st, l, result)?;
        } else if a >= 4 {
            crate::save::geq_word_define(&mut st.eqtb, l, st.scanner.cur_val);
        } else if let Some(ab) =
            crate::save::eq_word_define(&mut st.eqtb, &mut st.save_stack, l, st.scanner.cur_val)
        {
            return Err(MainControlAbort::Save(ab));
        }
    } else {
        crate::main_control::trap_zero_glue(st);
        if e {
            let result = if a >= 4 {
                crate::save::gsa_def(&mut st.mem, l, st.scanner.cur_val)
            } else {
                let __cv10 = st.scanner.cur_val;
                crate::save::sa_def(
                    &mut st.mem,
                    &mut st.scanner,
                    &mut st.save_stack,
                    l,
                    __cv10,
                )
            };
            crate::save::finish_sa_mutation_in(st, l, result)?;
        } else if a >= 4 {
            let __cv9 = st.scanner.cur_val;
            if let Some(ab) = crate::save::geq_define(
                &mut st.eqtb,
                &mut st.mem,
                &mut st.scanner,
                l,
                GLUE_REF,
                __cv9,
            ) {
                return Err(MainControlAbort::Save(ab));
            }
        } else {
            let __cv8 = st.scanner.cur_val;
            if let Some(ab) = crate::save::eq_define(
                &mut st.eqtb,
                &mut st.save_stack,
                &mut st.mem,
                &mut st.scanner,
                l,
                GLUE_REF,
                __cv8,
            ) {
                return Err(MainControlAbort::Save(ab));
            }
        }
    }
    Ok(())
}

// translated from xetex-xetex0.c:10745 (do_assignments)
pub(crate) fn do_assignments_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const MAX_NON_PREFIXED_COMMAND: u8 = 71; // xetex_format.h:312
    const RELAX: u8 = 0; // xetex_format.h:229

    loop {
        loop {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
                break;
            }
        }
        if st.scanner.cur_cmd <= MAX_NON_PREFIXED_COMMAND {
            return Ok(());
        }
        st.scanner.set_box_allowed = false;
        prefixed_command_in(st)?;
        st.scanner.set_box_allowed = true;
    }
}

// ---- moved verbatim from scan.rs (workstream 02) ----

// translated from xetex-xetex0.c:8221 (main_control append_normal_space label)
pub(crate) fn main_control_append_normal_space_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const GLUE_PAR_SPACE_SKIP_MC: i16 = 12; // xetex_format.h:161
    const CHAR_NUM_MC: u8 = 16; // xetex_format.h:253

    if inter_char_boundary_after_space_action(st, CHAR_NUM_MC).execute_in(st)? {
        return Ok(());
    }

    let cur_font = st.eqtb.b32s1(CUR_FONT_LOC);
    if st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP_MC as i32) == 0 {
        crate::globals::set_main_p(st.fonts.font_glue(cur_font));
        if crate::globals::main_p() == crate::mem::TEX_NULL {
            crate::globals::set_main_p(crate::mem::new_spec(&mut st.mem, 0));
            if crate::globals::main_p() == crate::mem::ALLOC_FAILED {
                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
            }
            crate::globals::set_main_k(st.fonts.param_base(cur_font) + 2);
            st.mem.set_b32s1(
                crate::globals::main_p() + 1,
                st.fonts.font_info_b32s1(crate::globals::main_k()),
            );
            st.mem.set_b32s1(
                crate::globals::main_p() + 2,
                st.fonts.font_info_b32s1(crate::globals::main_k() + 1),
            );
            st.mem.set_b32s1(
                crate::globals::main_p() + 3,
                st.fonts.font_info_b32s1(crate::globals::main_k() + 2),
            );
            st.fonts.set_font_glue(cur_font, crate::globals::main_p());
        }
        st.mem.temp_ptr = crate::mem::new_glue(&mut st.mem, crate::globals::main_p());
    } else {
        st.mem.temp_ptr = crate::mem::new_param_glue(
            &mut st.mem,
            &st.eqtb,
            GLUE_PAR_SPACE_SKIP_MC,
        );
    }
    if st.mem.temp_ptr == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }

    st.mem
        .set_b32s1(crate::globals::cur_list_tail(), st.mem.temp_ptr);
    crate::globals::set_cur_list_tail(st.mem.temp_ptr);
    Ok(())
}

pub(crate) fn main_control_mode_independent_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const AFTER_ASSIGNMENT_MC: u8 = 40;
    const AFTER_GROUP_MC: u8 = 41;
    const XRAY_MC: u8 = 19;
    const CASE_SHIFT_MC: u8 = 57;
    const MESSAGE_MC: u8 = 58;
    const EXTENSION_MC: u8 = 59;
    const IN_STREAM_MC: u8 = 60;
    const TOKS_REGISTER_MC: u8 = 72;
    const ASSIGN_TOKS_MC: u8 = 73;
    const ASSIGN_INT_MC: u8 = 74;
    const ASSIGN_DIMEN_MC: u8 = 75;
    const ASSIGN_GLUE_MC: u8 = 76;
    const ASSIGN_MU_GLUE_MC: u8 = 77;
    const ASSIGN_FONT_DIMEN_MC: u8 = 78;
    const ASSIGN_FONT_INT_MC: u8 = 79;
    const SET_AUX_MC: u8 = 80;
    const SET_PREV_GRAF_MC: u8 = 81;
    const SET_PAGE_DIMEN_MC: u8 = 82;
    const SET_PAGE_INT_MC: u8 = 83;
    const SET_BOX_DIMEN_MC: u8 = 84;
    const SET_SHAPE_MC: u8 = 85;
    const DEF_CODE_MC: u8 = 86;
    const XETEX_DEF_CODE_MC: u8 = 87;
    const DEF_FAMILY_MC: u8 = 88;
    const SET_FONT_MC: u8 = 89;
    const DEF_FONT_MC: u8 = 90;
    const REGISTER_MC: u8 = 91;
    const ADVANCE_MC: u8 = 92;
    const MULTIPLY_MC: u8 = 93;
    const DIVIDE_MC: u8 = 94;
    const PREFIX_MC: u8 = 95;
    const LET_MC: u8 = 96;
    const SHORTHAND_DEF_MC: u8 = 97;
    const READ_TO_CS_MC: u8 = 98;
    const DEF_MC: u8 = 99;
    const SET_BOX_MC: u8 = 100;
    const HYPH_DATA_MC: u8 = 101;
    const SET_INTERACTION_MC: u8 = 102;

    match st.scanner.cur_cmd {
        TOKS_REGISTER_MC | ASSIGN_TOKS_MC | ASSIGN_INT_MC | ASSIGN_DIMEN_MC | ASSIGN_GLUE_MC
        | ASSIGN_MU_GLUE_MC | ASSIGN_FONT_DIMEN_MC | ASSIGN_FONT_INT_MC | SET_AUX_MC
        | SET_PREV_GRAF_MC | SET_PAGE_DIMEN_MC | SET_PAGE_INT_MC | SET_BOX_DIMEN_MC
        | SET_SHAPE_MC | DEF_CODE_MC | XETEX_DEF_CODE_MC | DEF_FAMILY_MC | SET_FONT_MC
        | DEF_FONT_MC | REGISTER_MC | ADVANCE_MC | MULTIPLY_MC | DIVIDE_MC | PREFIX_MC | LET_MC
        | SHORTHAND_DEF_MC | READ_TO_CS_MC | DEF_MC | SET_BOX_MC | HYPH_DATA_MC
        | SET_INTERACTION_MC => prefixed_command_in(st)?,
        AFTER_ASSIGNMENT_MC => {
            crate::scan::get_token_in(st)?;
            st.scanner.after_token = st.scanner.cur_tok;
        }
        AFTER_GROUP_MC => {
            crate::scan::get_token_in(st)?;
            if let Some(a) = crate::save::save_for_after(&mut st.save_stack, st.scanner.cur_tok) {
                return Err(MainControlAbort::Save(a));
            }
        }
        IN_STREAM_MC => unsafe { crate::io::open_or_close_in(st)? },
        MESSAGE_MC => crate::output::issue_message_in(st)?,
        CASE_SHIFT_MC => shift_case_in(st)?,
        XRAY_MC => crate::output::show_whatever_in(st)?,
        EXTENSION_MC => do_extension_in(st)?,
        _ => {}
    }
    Ok(())
}

pub(crate) fn main_control_mode_error_cases_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const VMODE_MC: u16 = 1;
    const HMODE_MC: u16 = 104;
    const MMODE_MC: u16 = 207;
    const MAC_PARAM_MC: u8 = 6;
    const SUP_MARK_MC: u8 = 7;
    const SUB_MARK_MC: u8 = 8;
    const ENDV_MC: u8 = 9;
    const PAR_END_MC: u8 = 13;
    const STOP_MC: u8 = 14;
    const DELIM_NUM_MC: u8 = 15;
    const MATH_CHAR_NUM_MC: u8 = 17;
    const HMOVE_MC: u8 = 21;
    const VMOVE_MC: u8 = 22;
    const UN_VBOX_MC: u8 = 24;
    const VSKIP_MC: u8 = 27;
    const MSKIP_MC: u8 = 28;
    const MKERN_MC: u8 = 30;
    const VALIGN_MC: u8 = 33;
    const HRULE_MC: u8 = 36;
    const VADJUST_MC: u8 = 38;
    const ITAL_CORR_MC: u8 = 44;
    const MATH_ACCENT_MC: u8 = 46;
    const EQ_NO_MC: u8 = 48;
    const LEFT_RIGHT_MC: u8 = 49;
    const MATH_COMP_MC: u8 = 50;
    const LIMIT_SWITCH_MC: u8 = 51;
    const ABOVE_MC: u8 = 52;
    const MATH_STYLE_MC: u8 = 53;
    const MATH_CHOICE_MC: u8 = 54;
    const NON_SCRIPT_MC: u8 = 55;
    const VCENTER_MC: u8 = 56;
    const RADICAL_MC: u8 = 66;
    const MATH_GIVEN_MC: u8 = 69;
    const XETEX_MATH_GIVEN_MC: u8 = 70;
    const LAST_ITEM_MC: u8 = 71;

    match (crate::globals::cur_list_mode().unsigned_abs(), st.scanner.cur_cmd) {
        (VMODE_MC, VMOVE_MC)
        | (HMODE_MC, HMOVE_MC)
        | (MMODE_MC, HMOVE_MC)
        | (VMODE_MC, LAST_ITEM_MC)
        | (HMODE_MC, LAST_ITEM_MC)
        | (MMODE_MC, LAST_ITEM_MC)
        | (VMODE_MC, VADJUST_MC)
        | (VMODE_MC, ITAL_CORR_MC)
        | (VMODE_MC, EQ_NO_MC)
        | (HMODE_MC, EQ_NO_MC)
        | (VMODE_MC, MAC_PARAM_MC)
        | (HMODE_MC, MAC_PARAM_MC)
        | (MMODE_MC, MAC_PARAM_MC) => {
            mc_propagate_error(crate::errors::report_illegal_case_in(st))?;
        }
        (VMODE_MC, SUP_MARK_MC)
        | (HMODE_MC, SUP_MARK_MC)
        | (VMODE_MC, SUB_MARK_MC)
        | (HMODE_MC, SUB_MARK_MC)
        | (VMODE_MC, MATH_CHAR_NUM_MC)
        | (HMODE_MC, MATH_CHAR_NUM_MC)
        | (VMODE_MC, MATH_GIVEN_MC)
        | (HMODE_MC, MATH_GIVEN_MC)
        | (VMODE_MC, XETEX_MATH_GIVEN_MC)
        | (HMODE_MC, XETEX_MATH_GIVEN_MC)
        | (VMODE_MC, MATH_COMP_MC)
        | (HMODE_MC, MATH_COMP_MC)
        | (VMODE_MC, DELIM_NUM_MC)
        | (HMODE_MC, DELIM_NUM_MC)
        | (VMODE_MC, LEFT_RIGHT_MC)
        | (HMODE_MC, LEFT_RIGHT_MC)
        | (VMODE_MC, ABOVE_MC)
        | (HMODE_MC, ABOVE_MC)
        | (VMODE_MC, RADICAL_MC)
        | (HMODE_MC, RADICAL_MC)
        | (VMODE_MC, MATH_STYLE_MC)
        | (HMODE_MC, MATH_STYLE_MC)
        | (VMODE_MC, MATH_CHOICE_MC)
        | (HMODE_MC, MATH_CHOICE_MC)
        | (VMODE_MC, VCENTER_MC)
        | (HMODE_MC, VCENTER_MC)
        | (VMODE_MC, NON_SCRIPT_MC)
        | (HMODE_MC, NON_SCRIPT_MC)
        | (VMODE_MC, MKERN_MC)
        | (HMODE_MC, MKERN_MC)
        | (VMODE_MC, LIMIT_SWITCH_MC)
        | (HMODE_MC, LIMIT_SWITCH_MC)
        | (VMODE_MC, MSKIP_MC)
        | (HMODE_MC, MSKIP_MC)
        | (VMODE_MC, MATH_ACCENT_MC)
        | (HMODE_MC, MATH_ACCENT_MC)
        | (MMODE_MC, ENDV_MC)
        | (MMODE_MC, PAR_END_MC)
        | (MMODE_MC, STOP_MC)
        | (MMODE_MC, VSKIP_MC)
        | (MMODE_MC, UN_VBOX_MC)
        | (MMODE_MC, VALIGN_MC)
        | (MMODE_MC, HRULE_MC) => crate::scan::insert_dollar_sign_in(st)?,
        _ => {}
    }
    Ok(())
}

pub(crate) fn main_control_simple_list_group_cases_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const VMODE_MC: u16 = 1;
    const HMODE_MC: u16 = 104;
    const MMODE_MC: u16 = 207;
    const LEFT_BRACE_MC: u8 = 1;
    const KERN_MC: u8 = 29;
    const MKERN_MC: u8 = 30;
    const HSKIP_MC: u8 = 26;
    const VSKIP_MC: u8 = 27;
    const MSKIP_MC: u8 = 28;
    const BEGIN_GROUP_MC: u8 = 61;
    const END_GROUP_MC: u8 = 62;
    const HRULE_MC: u8 = 36;
    const VRULE_MC: u8 = 35;
    const SIMPLE_GROUP_MC: u8 = 1;
    const SEMI_SIMPLE_GROUP_MC: u8 = 14;
    const IGNORE_DEPTH_MC: i32 = -65_536_000;

    let mode = crate::globals::cur_list_mode().unsigned_abs();
    match (mode, st.scanner.cur_cmd) {
        (VMODE_MC, HRULE_MC) | (HMODE_MC, VRULE_MC) | (MMODE_MC, VRULE_MC) => {
            let p = crate::scan::scan_rule_spec_in(st)?;
            st.mem.set_b32s1(crate::globals::cur_list_tail(), p);
            crate::globals::set_cur_list_tail(p);
            if mode == VMODE_MC {
                crate::globals::set_cur_list_aux_b32s1(IGNORE_DEPTH_MC);
            } else if mode == HMODE_MC {
                crate::globals::set_cur_list_aux_b32s0(1000);
            }
        }
        (VMODE_MC, VSKIP_MC)
        | (HMODE_MC, HSKIP_MC)
        | (MMODE_MC, HSKIP_MC)
        | (MMODE_MC, MSKIP_MC) => append_glue_in(st)?,
        (VMODE_MC, KERN_MC) | (HMODE_MC, KERN_MC) | (MMODE_MC, KERN_MC) | (MMODE_MC, MKERN_MC) => {
            append_kern_in(st)?
        }
        (VMODE_MC, LEFT_BRACE_MC) | (HMODE_MC, LEFT_BRACE_MC) => {
            if let Some(a) = crate::save::new_save_level(&mut st.save_stack, SIMPLE_GROUP_MC, st.mem.line) {
                return Err(MainControlAbort::Save(a));
            }
        }
        (VMODE_MC, BEGIN_GROUP_MC) | (HMODE_MC, BEGIN_GROUP_MC) | (MMODE_MC, BEGIN_GROUP_MC) => {
            if let Some(a) = crate::save::new_save_level(&mut st.save_stack, SEMI_SIMPLE_GROUP_MC, st.mem.line) {
                return Err(MainControlAbort::Save(a));
            }
        }
        (VMODE_MC, END_GROUP_MC) | (HMODE_MC, END_GROUP_MC) | (MMODE_MC, END_GROUP_MC) => {
            if st.save_stack.cur_group == SEMI_SIMPLE_GROUP_MC {
                crate::save::unsave_in(st)?;
            } else {
                off_save_in(st)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn main_control_box_list_cases_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const VMODE_MC: u16 = 1;
    const HMODE_MC: u16 = 104;
    const MMODE_MC: u16 = 207;
    const RIGHT_BRACE_MC: u8 = 2;
    const MATH_SHIFT_MC: u8 = 3;
    const TAB_MARK_MC: u8 = 4;
    const CAR_RET_MC: u8 = 5;
    const LETTER_MC: u8 = 11;
    const OTHER_CHAR_MC: u8 = 12;
    const PAR_END_MC: u8 = 13;
    const STOP_MC: u8 = 14;
    const CHAR_NUM_MC: u8 = 16;
    const MARK_MC: u8 = 18;
    const MAKE_BOX_MC: u8 = 20;
    const HMOVE_MC: u8 = 21;
    const VMOVE_MC: u8 = 22;
    const UN_HBOX_MC: u8 = 23;
    const UN_VBOX_MC: u8 = 24;
    const REMOVE_ITEM_MC: u8 = 25;
    const HSKIP_MC: u8 = 26;
    const VSKIP_MC: u8 = 27;
    const LEADER_SHIP_MC: u8 = 31;
    const HALIGN_MC: u8 = 32;
    const VALIGN_MC: u8 = 33;
    const VRULE_MC: u8 = 35;
    const HRULE_MC: u8 = 36;
    const INSERT_MC: u8 = 37;
    const VADJUST_MC: u8 = 38;
    const BREAK_PENALTY_MC: u8 = 42;
    const START_PAR_MC: u8 = 43;
    const ITAL_CORR_MC: u8 = 44;
    const ACCENT_MC: u8 = 45;
    const DISCRETIONARY_MC: u8 = 47;
    const EX_SPACE_MC: u8 = 64;
    const NO_BOUNDARY_MC: u8 = 65;
    const CHAR_GIVEN_MC: u8 = 68;
    const LEADER_FLAG_MC: i32 = 0x4001_0001;
    const A_LEADERS_MC: i32 = 100;

    match (crate::globals::cur_list_mode().unsigned_abs(), st.scanner.cur_cmd) {
        (VMODE_MC, RIGHT_BRACE_MC) | (HMODE_MC, RIGHT_BRACE_MC) | (MMODE_MC, RIGHT_BRACE_MC) => {
            handle_right_brace_in(st)?
        }
        (VMODE_MC, HMOVE_MC) | (HMODE_MC, VMOVE_MC) | (MMODE_MC, VMOVE_MC) => {
            let t = st.scanner.cur_chr;
            crate::scan::scan_dimen_in(st, false, false, false)?;
            if t == 0 {
                scan_box_in(st, st.scanner.cur_val)?;
            } else {
                scan_box_in(st, -st.scanner.cur_val)?;
            }
        }
        (VMODE_MC, LEADER_SHIP_MC) | (HMODE_MC, LEADER_SHIP_MC) | (MMODE_MC, LEADER_SHIP_MC) => {
            scan_box_in(st, LEADER_FLAG_MC - A_LEADERS_MC + st.scanner.cur_chr)?;
        }
        (VMODE_MC, MAKE_BOX_MC) | (HMODE_MC, MAKE_BOX_MC) | (MMODE_MC, MAKE_BOX_MC) => {
            begin_box_in(st, 0)?
        }
        (VMODE_MC, START_PAR_MC) => new_graf_in(st, st.scanner.cur_chr > 0)?,
        (VMODE_MC, LETTER_MC)
        | (VMODE_MC, OTHER_CHAR_MC)
        | (VMODE_MC, CHAR_NUM_MC)
        | (VMODE_MC, CHAR_GIVEN_MC)
        | (VMODE_MC, MATH_SHIFT_MC)
        | (VMODE_MC, UN_HBOX_MC)
        | (VMODE_MC, VRULE_MC)
        | (VMODE_MC, ACCENT_MC)
        | (VMODE_MC, DISCRETIONARY_MC)
        | (VMODE_MC, HSKIP_MC)
        | (VMODE_MC, VALIGN_MC)
        | (VMODE_MC, EX_SPACE_MC)
        | (VMODE_MC, NO_BOUNDARY_MC) => {
            crate::input::back_input_in(st)?;
            new_graf_in(st, true)?;
        }
        (HMODE_MC, START_PAR_MC) | (MMODE_MC, START_PAR_MC) => {
            indent_in_hmode(st)?;
        }
        (VMODE_MC, PAR_END_MC) => {
            if let Some(a) = normal_paragraph(
                &mut st.eqtb,
                &mut st.save_stack,
                &mut st.mem,
                &mut st.scanner,
            ) {
                return Err(MainControlAbort::Save(a));
            }
            if crate::globals::cur_list_mode() > 0 {
                crate::pagebuilder::build_page_in(st)?;
            }
        }
        (HMODE_MC, PAR_END_MC) => {
            if st.scanner.align_state < 0 {
                off_save_in(st)?;
            }
            end_graf_in(st)?;
            if crate::globals::cur_list_mode() == VMODE_MC as i16 {
                crate::pagebuilder::build_page_in(st)?;
            }
        }
        (HMODE_MC, STOP_MC)
        | (HMODE_MC, VSKIP_MC)
        | (HMODE_MC, HRULE_MC)
        | (HMODE_MC, UN_VBOX_MC)
        | (HMODE_MC, HALIGN_MC) => head_for_vmode_in(st)?,
        (VMODE_MC, INSERT_MC)
        | (HMODE_MC, INSERT_MC)
        | (MMODE_MC, INSERT_MC)
        | (HMODE_MC, VADJUST_MC)
        | (MMODE_MC, VADJUST_MC) => begin_insert_or_adjust_in(st)?,
        (VMODE_MC, MARK_MC) | (HMODE_MC, MARK_MC) | (MMODE_MC, MARK_MC) => make_mark_in(st)?,
        (VMODE_MC, BREAK_PENALTY_MC)
        | (HMODE_MC, BREAK_PENALTY_MC)
        | (MMODE_MC, BREAK_PENALTY_MC) => append_penalty_in(st)?,
        (VMODE_MC, REMOVE_ITEM_MC) | (HMODE_MC, REMOVE_ITEM_MC) | (MMODE_MC, REMOVE_ITEM_MC) => {
            delete_last_in(st)?
        }
        (VMODE_MC, UN_VBOX_MC) | (HMODE_MC, UN_HBOX_MC) | (MMODE_MC, UN_HBOX_MC) => {
            unpackage_in(st)?
        }
        (HMODE_MC, ITAL_CORR_MC) => append_italic_correction_in(st)?,
        (MMODE_MC, ITAL_CORR_MC) => {
            let p = crate::mem::new_kern(&mut st.mem, 0);
            if p == crate::mem::ALLOC_FAILED {
                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(crate::globals::cur_list_tail(), p);
            crate::globals::set_cur_list_tail(p);
        }
        (HMODE_MC, DISCRETIONARY_MC) | (MMODE_MC, DISCRETIONARY_MC) => append_discretionary_in(st)?,
        (HMODE_MC, ACCENT_MC) => make_accent_in(st)?,
        (VMODE_MC, CAR_RET_MC)
        | (HMODE_MC, CAR_RET_MC)
        | (MMODE_MC, CAR_RET_MC)
        | (VMODE_MC, TAB_MARK_MC)
        | (HMODE_MC, TAB_MARK_MC)
        | (MMODE_MC, TAB_MARK_MC) => align_error_in(st)?,
        _ => {}
    }
    Ok(())
}

pub(crate) fn main_control_alignment_cases_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const VMODE_MC: u16 = 1;
    const HMODE_MC: u16 = 104;
    const MMODE_MC: u16 = 207;
    const ENDV_MC: u8 = 9;
    const HALIGN_MC: u8 = 32;
    const VALIGN_MC: u8 = 33;
    const NO_ALIGN_MC: u8 = 34;
    const OMIT_MC: u8 = 63;
    const END_CS_NAME_MC: u8 = 67;
    const INT_PAR_TEXXET_MC: i32 = 69;
    const MATH_SHIFT_GROUP_MC: u8 = 15;

    match (crate::globals::cur_list_mode().unsigned_abs(), st.scanner.cur_cmd) {
        (VMODE_MC, NO_ALIGN_MC) | (HMODE_MC, NO_ALIGN_MC) | (MMODE_MC, NO_ALIGN_MC) => {
            no_align_error_in(st)?
        }
        (VMODE_MC, OMIT_MC) | (HMODE_MC, OMIT_MC) | (MMODE_MC, OMIT_MC) => omit_error_in(st)?,
        (VMODE_MC, HALIGN_MC) => init_align_in(st)?,
        (HMODE_MC, VALIGN_MC) => {
            if st.scanner.cur_chr > 0 {
                let enabled = crate::scan::eTeX_enabled_in(
                    st,
                    st.eqtb.b32s1(INT_BASE + INT_PAR_TEXXET_MC) > 0,
                    st.scanner.cur_cmd as u16,
                    st.scanner.cur_chr,
                )?;
                if enabled {
                    let p = crate::mem::new_math(&mut st.mem, 0, st.scanner.cur_chr as i16);
                    if p == crate::mem::ALLOC_FAILED {
                        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                    }
                    st.mem.set_b32s1(crate::globals::cur_list_tail(), p);
                    crate::globals::set_cur_list_tail(p);
                }
            } else {
                init_align_in(st)?;
            }
        }
        (MMODE_MC, HALIGN_MC) => {
            if crate::errors::privileged_in(st).map_err(MainControlAbort::Error)? {
                if st.save_stack.cur_group == MATH_SHIFT_GROUP_MC {
                    init_align_in(st)?;
                } else {
                    off_save_in(st)?;
                }
            }
        }
        (VMODE_MC, ENDV_MC) | (HMODE_MC, ENDV_MC) => do_endv_in(st)?,
        (VMODE_MC, END_CS_NAME_MC) | (HMODE_MC, END_CS_NAME_MC) | (MMODE_MC, END_CS_NAME_MC) => {
            mc_propagate_error(crate::errors::cs_error_in(st))?
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn main_control_math_cases_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    extern "C" {}

    const HMODE_MC: u16 = 104; // xetex_format.h:369
    const MMODE_MC: u16 = 207; // xetex_format.h:370
    const MATH_SHIFT_MC: u8 = 3; // xetex_format.h:233
    const LEFT_BRACE_MC: u8 = 1; // xetex_format.h:231
    const SUP_MARK_MC: u8 = 7; // xetex_format.h:238
    const SUB_MARK_MC: u8 = 8; // xetex_format.h:239
    const LETTER_MC: u8 = 11; // xetex_format.h:243
    const OTHER_CHAR_MC: u8 = 12; // xetex_format.h:244
    const DELIM_NUM_MC: u8 = 15; // xetex_format.h:251
    const CHAR_NUM_MC: u8 = 16; // xetex_format.h:253
    const MATH_CHAR_NUM_MC: u8 = 17; // xetex_format.h:255
    const ACCENT_MC: u8 = 45; // xetex_format.h:283
    const MATH_ACCENT_MC: u8 = 46; // xetex_format.h:284
    const EQ_NO_MC: u8 = 48; // xetex_format.h:286
    const LEFT_RIGHT_MC: u8 = 49; // xetex_format.h:287
    const MATH_COMP_MC: u8 = 50; // xetex_format.h:288
    const LIMIT_SWITCH_MC: u8 = 51; // xetex_format.h:289
    const ABOVE_MC: u8 = 52; // xetex_format.h:290
    const MATH_STYLE_MC: u8 = 53; // xetex_format.h:291
    const MATH_CHOICE_MC: u8 = 54; // xetex_format.h:292
    const NON_SCRIPT_MC: u8 = 55; // xetex_format.h:293
    const VCENTER_MC: u8 = 56; // xetex_format.h:294
    const RADICAL_MC: u8 = 66; // xetex_format.h:304
    const CHAR_GIVEN_MC: u8 = 68; // xetex_format.h:306
    const MATH_GIVEN_MC: u8 = 69; // xetex_format.h:308
    const XETEX_MATH_GIVEN_MC: u8 = 70; // xetex_format.h:309

    const MATH_SHIFT_GROUP_MC: u8 = 15; // xetex-constants.h:167
    const COND_MATH_GLUE_MC: u16 = 98; // xetex-constants.h:324
    const VCENTER_GROUP_MC: u8 = 12; // xetex-constants.h:164
    const IGNORE_DEPTH_MC: i32 = -65_536_000; // xetex_format.h:388
    const EVERY_VBOX_TEXT_MC: u16 = 12; // xetex-constants.h:211
    const LOCAL_EVERY_VBOX_MC: i32 = 6; // xetex_format.h:178

    use crate::globals::{
        cur_list_mode, cur_list_tail, set_cur_list_tail,
    };

    match (cur_list_mode().unsigned_abs(), st.scanner.cur_cmd) {
        (HMODE_MC, MATH_SHIFT_MC) => init_math_in(st)?,

        (MMODE_MC, EQ_NO_MC) => {
            if crate::errors::privileged_in(st).map_err(MainControlAbort::Error)? {
                if st.save_stack.cur_group == MATH_SHIFT_GROUP_MC {
                    crate::math::start_eq_no_in(st)?;
                } else {
                    off_save_in(st)?;
                }
            }
        }

        (MMODE_MC, LEFT_BRACE_MC) => {
            let n = crate::mem::new_noad(&mut st.mem);
            if n == crate::mem::ALLOC_FAILED {
                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(cur_list_tail(), n);
            set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
            crate::input::back_input_in(st)?;
            crate::scan::scan_math_in(st, cur_list_tail() + 1)?;
        }

        (MMODE_MC, LETTER_MC) | (MMODE_MC, OTHER_CHAR_MC) | (MMODE_MC, CHAR_GIVEN_MC) => {
            crate::scan::set_math_char_in(st, st.eqtb.b32s1(MATH_CODE_BASE + st.scanner.cur_chr))?;
        }

        (MMODE_MC, CHAR_NUM_MC) => {
            crate::scan::scan_char_num_in(st)?;
            st.scanner.cur_chr = st.scanner.cur_val;
            crate::scan::set_math_char_in(st, st.eqtb.b32s1(MATH_CODE_BASE + st.scanner.cur_chr))?;
        }

        (MMODE_MC, MATH_CHAR_NUM_MC) => {
            if st.scanner.cur_chr == 2 {
                // \Umathchar
                crate::scan::scan_math_class_int_in(st)?;
                let mut t = set_class_fn(st.scanner.cur_val);
                crate::scan::scan_math_fam_int_in(st)?;
                t += set_family_fn(st.scanner.cur_val);
                crate::scan::scan_usv_num_in(st)?;
                t += st.scanner.cur_val;
                crate::scan::set_math_char_in(st, t)?;
            } else if st.scanner.cur_chr == 1 {
                // \Umathcharnum
                crate::scan::scan_xetex_math_char_int_in(st)?;
                crate::scan::set_math_char_in(st, st.scanner.cur_val)?;
            } else {
                crate::scan::scan_fifteen_bit_int_in(st)?;
                let v = st.scanner.cur_val;
                crate::scan::set_math_char_in(
                    st,
                    set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256),
                )?;
            }
        }

        (MMODE_MC, MATH_GIVEN_MC) => {
            let c = st.scanner.cur_chr;
            crate::scan::set_math_char_in(
                st,
                set_class_fn(c / 4096) + set_family_fn((c % 4096) / 256) + (c % 256),
            )?;
        }

        (MMODE_MC, XETEX_MATH_GIVEN_MC) => {
            crate::scan::set_math_char_in(st, st.scanner.cur_chr)?;
        }

        (MMODE_MC, DELIM_NUM_MC) => {
            if st.scanner.cur_chr == 1 {
                // \Udelimiter
                crate::scan::scan_math_class_int_in(st)?;
                let mut t = set_class_fn(st.scanner.cur_val);
                crate::scan::scan_math_fam_int_in(st)?;
                t += set_family_fn(st.scanner.cur_val);
                crate::scan::scan_usv_num_in(st)?;
                t += st.scanner.cur_val;
                crate::scan::set_math_char_in(st, t)?;
            } else {
                crate::scan::scan_delimiter_int_in(st)?;
                let v = st.scanner.cur_val / 4096;
                crate::scan::set_math_char_in(
                    st,
                    set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256),
                )?;
            }
        }

        (MMODE_MC, MATH_COMP_MC) => {
            let n = crate::mem::new_noad(&mut st.mem);
            if n == crate::mem::ALLOC_FAILED {
                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(cur_list_tail(), n);
            set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
            st.mem.set_b16s1(cur_list_tail(), st.scanner.cur_chr as u16);
            crate::scan::scan_math_in(st, cur_list_tail() + 1)?;
        }

        (MMODE_MC, LIMIT_SWITCH_MC) => crate::math::math_limit_switch_in(st)?,
        (MMODE_MC, RADICAL_MC) => crate::math::math_radical_in(st)?,
        (MMODE_MC, ACCENT_MC) | (MMODE_MC, MATH_ACCENT_MC) => crate::math::math_ac_in(st)?,

        (MMODE_MC, VCENTER_MC) => {
            crate::scan::scan_spec_in(st, VCENTER_GROUP_MC, false)?;
            let save_abort = normal_paragraph(
                &mut st.eqtb,
                &mut st.save_stack,
                &mut st.mem,
                &mut st.scanner,
            );
            if let Some(a) = save_abort {
                return Err(MainControlAbort::Save(a));
            }
            crate::mem::push_nest_in(st)?;
            crate::globals::set_cur_list_mode(-1);
            crate::globals::set_cur_list_aux_b32s1(IGNORE_DEPTH_MC);
            if st.shipout.insert_src_special_every_vbox {
                crate::ext::insert_src_special_in(st)?;
            }
            let every_vbox = st.eqtb.b32s1(LOCAL_BASE + LOCAL_EVERY_VBOX_MC);
            if every_vbox != TEX_NULL {
                crate::input::begin_token_list_in(st, every_vbox, EVERY_VBOX_TEXT_MC)?;
            }
        }

        (MMODE_MC, MATH_STYLE_MC) => {
            let s = crate::mem::new_style(&mut st.mem, st.scanner.cur_chr as i16);
            if s == crate::mem::ALLOC_FAILED {
                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(cur_list_tail(), s);
            set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
        }

        (MMODE_MC, NON_SCRIPT_MC) => {
            let g = crate::mem::new_glue(&mut st.mem, 0);
            if g == crate::mem::ALLOC_FAILED {
                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
            }
            st.mem.set_b32s1(cur_list_tail(), g);
            set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
            st.mem.set_b16s0(cur_list_tail(), COND_MATH_GLUE_MC);
        }

        (MMODE_MC, MATH_CHOICE_MC) => crate::math::append_choices_in(st)?,
        (MMODE_MC, SUB_MARK_MC) | (MMODE_MC, SUP_MARK_MC) => crate::math::sub_sup_in(st)?,
        (MMODE_MC, ABOVE_MC) => crate::math::math_fraction_in(st)?,
        (MMODE_MC, LEFT_RIGHT_MC) => crate::math::math_left_right_in(st)?,

        (MMODE_MC, MATH_SHIFT_MC) => {
            if st.save_stack.cur_group == MATH_SHIFT_GROUP_MC {
                after_math_in(st)?;
            } else {
                off_save_in(st)?;
            }
        }

        _ => {}
    }
    Ok(())
}

pub(crate) fn main_control_ignore_spaces_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const SPACER_MC: u8 = 10;
    const NORMAL_SCANNER_MC: u8 = 0;

    if st.scanner.cur_chr == 0 {
        loop {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd != SPACER_MC {
                break;
            }
        }
    } else {
        let saved = st.scanner.scanner_status;
        st.scanner.scanner_status = NORMAL_SCANNER_MC;
        let get_next_outcome = crate::scan::get_next_in(st);
        st.scanner.scanner_status = saved;
        let get_next_result: Result<(), crate::scan::SccAbort> = get_next_outcome.into();
        get_next_result?;

        if st.scanner.cur_cs < HASH_BASE {
            let v = crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, st.scanner.cur_cs - SINGLE_BASE)
                .map_err(crate::scan::hash_err_abort)?;
            st.scanner.cur_cs = v;
        } else {
            let s = st.eqtb.hash_s1(st.scanner.cur_cs);
            let v = crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, s)
                .map_err(crate::scan::hash_err_abort)?;
            st.scanner.cur_cs = v;
        }

        if st.scanner.cur_cs != UNDEFINED_PRIMITIVE {
            st.scanner.cur_cmd = st.eqtb.b16s1(PRIM_EQTB_BASE + st.scanner.cur_cs) as u8;
            st.scanner.cur_chr = st.eqtb.b32s1(PRIM_EQTB_BASE + st.scanner.cur_cs);
            st.scanner.cur_tok = CS_TOKEN_FLAG + PRIM_EQTB_BASE + st.scanner.cur_cs;
        }
    }
    // Caller must do: goto reswitch
    Ok(())
}

enum InterCharInputAction {
    None,
    BackInputThenBeginTokenList {
        token_list: i32,
        force_cur_input_index: Option<u16>,
    },
}

impl InterCharInputAction {
    #[inline]
    fn execute_in(self, st: &mut crate::state::EngineState) -> Result<bool, MainControlAbort> {
        match self {
            InterCharInputAction::None => Ok(false),
            InterCharInputAction::BackInputThenBeginTokenList {
                token_list,
                force_cur_input_index,
            } => {
                crate::input::back_input_in(st)?;
                if let Some(index) = force_cur_input_index {
                    set_cur_input_index(index);
                }
                crate::input::begin_token_list_in(st, token_list, INTER_CHAR_TEXT)?;
                Ok(true)
            }
        }
    }
}

fn set_current_token_for_inter_char_boundary(
    st: &mut crate::state::EngineState,
    char_num_cmd: u8,
) {
    if st.scanner.cur_cs == 0 {
        if st.scanner.cur_cmd == char_num_cmd {
            st.scanner.cur_cmd = OTHER_CHAR;
        }
        st.scanner.cur_tok = st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
    } else {
        st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
    }
}

/// Probe `\XeTeXinterchartoks` for the current space-class followed by the
/// boundary class. The input-stack mutation is returned as an action so callers
/// can run `back_input`/`begin_token_list` after the state borrow drops.
fn inter_char_boundary_after_space_action(
    st: &mut crate::state::EngineState,
    char_num_cmd: u8,
) -> InterCharInputAction {
    if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) <= 0 {
        return InterCharInputAction::None;
    }
    let sc = crate::globals::space_class();
    if sc == CHAR_CLASS_LIMIT || crate::globals::prev_class() == CHAR_CLASS_LIMIT - 1 {
        return InterCharInputAction::None;
    }

    crate::globals::set_prev_class(CHAR_CLASS_LIMIT - 1);
    crate::save::find_sa_element(
        &mut st.mem,
        &mut st.scanner,
        INTER_CHAR_VAL as u16,
        sc * CHAR_CLASS_LIMIT + CHAR_CLASS_LIMIT - 1,
        false,
    );
    let cur_ptr = st.scanner.cur_ptr;
    if cur_ptr == crate::mem::TEX_NULL {
        return InterCharInputAction::None;
    }
    let token_list = st.mem.b32s1(cur_ptr + 1);
    if token_list == crate::mem::TEX_NULL {
        return InterCharInputAction::None;
    }

    set_current_token_for_inter_char_boundary(st, char_num_cmd);
    InterCharInputAction::BackInputThenBeginTokenList {
        token_list,
        force_cur_input_index: None,
    }
}

fn inter_char_before_current_char_action(
    st: &mut crate::state::EngineState,
    sc: i32,
    letter_cmd: u8,
    other_cmd: u8,
) -> InterCharInputAction {
    if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) <= 0 || sc == CHAR_CLASS_LIMIT {
        return InterCharInputAction::None;
    }

    let prev = crate::globals::prev_class();
    if prev == CHAR_CLASS_LIMIT - 1 {
        let cur_input = crate::globals::cur_input();
        if cur_input.state == TOKEN_LIST_INPUT && cur_input.index == BACKED_UP_CHAR {
            return InterCharInputAction::None;
        }
        crate::save::find_sa_element(
            &mut st.mem,
            &mut st.scanner,
            INTER_CHAR_VAL as u16,
            (CHAR_CLASS_LIMIT - 1) * CHAR_CLASS_LIMIT + sc,
            false,
        );
    } else {
        crate::save::find_sa_element(
            &mut st.mem,
            &mut st.scanner,
            INTER_CHAR_VAL as u16,
            prev * CHAR_CLASS_LIMIT + sc,
            false,
        );
    }

    let cur_ptr = st.scanner.cur_ptr;
    if cur_ptr == crate::mem::TEX_NULL {
        return InterCharInputAction::None;
    }
    let token_list = st.mem.b32s1(cur_ptr + 1);
    if token_list == crate::mem::TEX_NULL {
        return InterCharInputAction::None;
    }

    if st.scanner.cur_cmd != letter_cmd {
        st.scanner.cur_cmd = other_cmd;
    }
    st.scanner.cur_tok = st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
    InterCharInputAction::BackInputThenBeginTokenList {
        token_list,
        force_cur_input_index: Some(BACKED_UP_CHAR),
    }
}

// `is_char_node` for the threaded main loop: reads `hi_mem_min` directly under
// the held borrow (it can shrink on the get_avail hi-mem-exhaustion path, so it
// must be read fresh, not captured).
#[inline(always)]
fn mc_is_char_node(st: &crate::state::EngineState, p: i32) -> bool {
    p >= st.mem.hi_mem_min
}

// Node-allocation helpers for the threaded main loop: allocate under the held
// borrow and lift the cold `ALLOC_FAILED` overflow into `MainControlAbort`.
#[inline]
fn mc_get_avail(st: &mut crate::state::EngineState) -> Result<i32, MainControlAbort> {
    let p = crate::mem::get_avail(&mut st.mem);
    if p == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    Ok(p)
}
#[inline]
fn mc_new_disc(st: &mut crate::state::EngineState) -> Result<i32, MainControlAbort> {
    let p = crate::mem::new_disc(&mut st.mem);
    if p == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    Ok(p)
}
#[inline]
fn mc_new_kern(st: &mut crate::state::EngineState, w: i32) -> Result<i32, MainControlAbort> {
    let p = crate::mem::new_kern(&mut st.mem, w);
    if p == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    Ok(p)
}
#[inline]
fn mc_new_ligature(
    st: &mut crate::state::EngineState,
    f: i32,
    c: u16,
    q: i32,
) -> Result<i32, MainControlAbort> {
    let p = crate::mem::new_ligature(&mut st.mem, f, c, q);
    if p == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    Ok(p)
}
#[inline]
fn mc_new_lig_item(st: &mut crate::state::EngineState, c: u16) -> Result<i32, MainControlAbort> {
    let p = crate::mem::new_lig_item(&mut st.mem, c);
    if p == crate::mem::ALLOC_FAILED {
        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
    }
    Ok(p)
}

// translated from xetex-xetex0.c:6804 (main_control HMODE inter-char pre-dispatch check)
// Returns true if the caller should goto big_switch (inter-char token was injected).
pub(crate) fn main_control_hmode_inter_char_check_in(
    st: &mut crate::state::EngineState,
) -> Result<bool, MainControlAbort> {
    const HMODE_MC: i16 = 104;
    const CHAR_NUM_MC: u8 = 16;
    if crate::globals::cur_list_mode().abs() != HMODE_MC {
        return Ok(false);
    }
    inter_char_boundary_after_space_action(st, CHAR_NUM_MC).execute_in(st)
}

pub(crate) fn main_control_spacer_cases_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    const HMODE_MC: u16 = 104; // xetex_format.h:369
    const MMODE_MC: u16 = 207; // xetex_format.h:370
    const SPACER_MC: u8 = 10; // xetex_format.h:242
    const EX_SPACE_MC: u8 = 64; // xetex_format.h:302

    match (crate::globals::cur_list_mode().unsigned_abs(), st.scanner.cur_cmd) {
        (HMODE_MC, SPACER_MC) => {
            if crate::globals::cur_list_aux_b32s0() == 1000 {
                main_control_append_normal_space_in(st)?;
            } else {
                app_space_in(st)?;
            }
        }
        (HMODE_MC, EX_SPACE_MC) | (MMODE_MC, EX_SPACE_MC) => {
            main_control_append_normal_space_in(st)?;
        }
        _ => {}
    }
    Ok(())
}
// translated from xetex-xetex0.c:7030 (main_loop through main_loop_move_lig)
// Returns true → caller must goto big_switch; false → caller must goto reswitch.
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub(crate) unsafe fn main_control_main_loop_in(
    st: &mut crate::state::EngineState,
) -> Result<bool, MainControlAbort> {
    use crate::globals::{
        bchar, cancel_boundary, cur_l, cur_list_aux_b32s0, cur_list_aux_b32s1,
        cur_list_head, cur_list_mode, cur_list_tail, cur_q, cur_r, ins_disc, lft_hit,
        lig_stack, ligature_present, main_f, main_h, main_p, main_pp, main_ppp, prev_class, rt_hit,
        set_bchar, set_cancel_boundary, set_cur_l, set_cur_list_aux_b32s0,
        set_cur_list_tail, set_cur_q, set_cur_r, set_false_bchar, set_ins_disc,
        set_lft_hit, set_lig_stack, set_ligature_present, set_main_f, set_main_h, set_main_p,
        set_main_pp, set_main_ppp, set_main_s, set_prev_class, set_rt_hit, set_space_class,
        space_class,
    };
    use crate::mem::TEX_NULL;

    // Constants
    const INT_PAR_LANGUAGE: i32 = 50;
    const INT_PAR_TRACING_LOST_CHARS: i32 = 35;
    const INT_PAR_XETEX_INTERWORD_SPACE_SHAPING: i32 = 78;
    const INT_PAR_XETEX_DASH_BREAK: i32 = 70;
    const INT_PAR_XETEX_USE_GLYPH_METRICS: i32 = 72;
    const LIG_TAG: u16 = 1;
    const NON_ADDRESS: i32 = 0;
    const NATIVE_NODE_SIZE: i32 = 6;
    const SPACE_ADJUSTMENT: u16 = 3;
    const WHATSIT_NODE: u16 = 8;
    const NATIVE_WORD_NODE: u16 = 40;
    const NATIVE_WORD_NODE_AT: u16 = 41;
    const INS_NODE: u16 = 3;
    const MARK_NODE: u16 = 4;
    const ADJUST_NODE: u16 = 5;
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const CHAR_GIVEN: u8 = 68;
    const CHAR_NUM: u8 = 16;

    // === main_loop: entry point ===
    // SyncTeX: append special at start of new paragraph
    if cur_list_head() == cur_list_tail()
        && cur_list_mode() > 0
        && st.shipout.insert_src_special_auto()
    {
        append_src_special_in(st)?;
    }
    set_prev_class(CHAR_CLASS_LIMIT - 1);

    let mf = st.eqtb.b32s1(CUR_FONT_LOC);
    let fa = st.fonts.font_area_entry(mf);

    // =========================================================================
    // AAT / OTGR native font path (collect_native loop)
    // =========================================================================
    if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
        if cur_list_mode() > 0 && st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE) != cur_list_aux_b32s1()
        {
            fix_language(&mut st.mem, &st.eqtb)?;
        }
        set_main_h(0);
        set_main_f(mf);
        st.scanner.set_native_len(0);

        // collect_native loop: accumulate consecutive chars into native_text buffer
        'collect: loop {
            // Update space factor from SF_CODE(cur_chr)
            let sf = st.eqtb.b32s1(SF_CODE_BASE + st.scanner.cur_chr);
            let ms = sf % 65536;
            set_main_s(ms);
            if ms == 1000 {
                set_cur_list_aux_b32s0(1000);
            } else if ms < 1000 {
                if ms > 0 {
                    set_cur_list_aux_b32s0(ms);
                }
            } else if cur_list_aux_b32s0() < 1000 {
                set_cur_list_aux_b32s0(1000);
            } else {
                set_cur_list_aux_b32s0(ms);
            }

            st.scanner.cur_ptr = TEX_NULL;
            set_space_class(sf / 65536);
            let sc = space_class();

            let was_boundary = prev_class() == CHAR_CLASS_LIMIT - 1;
            if inter_char_before_current_char_action(st, sc, LETTER, OTHER_CHAR).execute_in(st)? {
                if was_boundary {
                    return Ok(true); // goto big_switch
                }
                set_prev_class(CHAR_CLASS_LIMIT - 1);
                break 'collect; // goto collected
            }
            if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0
                && sc != CHAR_CLASS_LIMIT
            {
                set_prev_class(sc);
            }

            // Append cur_chr to native_text buffer
            let ch = st.scanner.cur_chr;
            if ch > 65535 {
                while st.scanner.native_text_size() <= st.scanner.native_len() + 2 {
                    let size = st.scanner.native_text_size + 128;
                    st.scanner.native_text_size = size;
                    st.scanner.native_text.resize(size as usize, 0);
                }
                *st.scanner
                    .native_text
                    .as_mut_ptr()
                    .add(st.scanner.native_len() as usize) = ((ch - 65536) / 1024 + 0xD800) as u16;
                st.scanner.set_native_len(st.scanner.native_len() + 1);
                *st.scanner
                    .native_text
                    .as_mut_ptr()
                    .add(st.scanner.native_len() as usize) = ((ch - 65536) % 1024 + 0xDC00) as u16;
                st.scanner.set_native_len(st.scanner.native_len() + 1);
            } else {
                while st.scanner.native_text_size() <= st.scanner.native_len() + 1 {
                    let size = st.scanner.native_text_size + 128;
                    st.scanner.native_text_size = size;
                    st.scanner.native_text.resize(size as usize, 0);
                }
                *st.scanner
                    .native_text
                    .as_mut_ptr()
                    .add(st.scanner.native_len() as usize) = ch as u16;
                st.scanner.set_native_len(st.scanner.native_len() + 1);
            }

            // Check if this char is a hyphen/dash
            let is_h = ch == st.fonts.hyphen_char_entry(main_f())
                || (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) > 0
                    && (ch == 8212 || ch == 8211));
            st.hyph.set_is_hyph(is_h);
            if main_h() == 0 && is_h {
                set_main_h(st.scanner.native_len());
            }

            // Look ahead for more chars (same font)
            crate::scan::get_next_scc(st)?;
            if st.scanner.cur_cmd == LETTER || st.scanner.cur_cmd == OTHER_CHAR || st.scanner.cur_cmd == CHAR_GIVEN {
                continue 'collect;
            }
            crate::scan::x_token_in(st)?;
            if st.scanner.cur_cmd == LETTER || st.scanner.cur_cmd == OTHER_CHAR || st.scanner.cur_cmd == CHAR_GIVEN {
                continue 'collect;
            }
            if st.scanner.cur_cmd == CHAR_NUM {
                crate::scan::scan_usv_num_in(st)?;
                st.scanner.cur_chr = st.scanner.cur_val;
                continue 'collect;
            }

            // End of char run: inter-char suffix check
            if inter_char_boundary_after_space_action(st, CHAR_NUM).execute_in(st)? {
                break 'collect; // goto collected (then process mapping)
            }
            break 'collect; // natural end of collect_native
        } // end 'collect

        // collected:
        // Apply font mapping if present
        let fm = st.fonts.font_mapping_ptr(main_f());
        if !fm.is_null() {
            // Evaluate the with_state-bearing `native_text`/`native_len` reads
            // before taking the `apply_mapping` borrow (it now threads `&mut
            // FontState`); they would nest-panic inside the closure.
            let nt = st.scanner.native_text.as_mut_ptr();
            let nl = st.scanner.native_len();
            let mk = apply_mapping(&mut st.fonts, fm, nt, nl);
            st.scanner.set_native_len(0);
            while st.scanner.native_text_size() <= st.scanner.native_len() + mk {
                let size = st.scanner.native_text_size + 128;
                st.scanner.native_text_size = size;
                st.scanner.native_text.resize(size as usize, 0);
            }
            set_main_h(0);
            for mp in 0..mk {
                let c = st.fonts.mapped_text_entry(mp);
                *st.scanner
                    .native_text
                    .as_mut_ptr()
                    .add(st.scanner.native_len() as usize) = c;
                st.scanner.set_native_len(st.scanner.native_len() + 1);
                if main_h() == 0
                    && (c == st.fonts.hyphen_char_entry(main_f()) as u16
                        || (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) > 0
                            && (c == 8212 || c == 8211)))
                {
                    set_main_h(st.scanner.native_len());
                }
            }
        }

        // Tracing lost chars
        if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_LOST_CHARS) > 0 {
            let mut tp = 0i32;
            while tp < st.scanner.native_len() {
                let mut mc = *st.scanner.native_text.as_mut_ptr().add(tp as usize) as i32;
                tp += 1;
                if (0xD800..0xDC00).contains(&mc) {
                    mc = 65536 + (mc - 0xD800) * 1024;
                    mc += *st.scanner.native_text.as_mut_ptr().add(tp as usize) as i32 - 0xDC00;
                    tp += 1;
                }
                if crate::ext::map_char_to_glyph(&st.fonts, main_f(), mc)? == 0 {
                    mc_propagate_error(crate::output::char_warning_in(st, main_f(), mc))?;
                }
            }
        }

        set_main_p(cur_list_tail());
        let mk = st.scanner.native_len();
        set_main_pp(cur_list_tail());

        if cur_list_mode() == 104
        /* HMODE */
        {
            set_main_ppp(cur_list_head());
            // Advance main_ppp to find node just before main_pp
            while main_ppp() != main_pp() && st.mem.b32s1(main_ppp()) != main_pp() {
                if !mc_is_char_node(st, main_ppp()) && st.mem.b16s1(main_ppp()) == DISC_NODE {
                    let tp = main_ppp();
                    for _ in 1..=st.mem.b16s0(tp) as i32 {
                        set_main_ppp(st.mem.b32s1(main_ppp()));
                    }
                }
                if main_ppp() != main_pp() {
                    set_main_ppp(st.mem.b32s1(main_ppp()));
                }
            }

            // C uses `main_k` as the running remaining-length and `temp_ptr` as
            // the offset into native_text of the current segment; both mutate
            // across the do-while. `main_h` (global) is the next hyphen offset.
            let mut temp_ptr = 0i32;
            let mut main_k = mk;
            loop {
                if main_h() == 0 {
                    set_main_h(main_k);
                }

                let tail_is_native = main_pp() != TEX_NULL
                    && !mc_is_char_node(st, main_pp())
                    && st.mem.b16s1(main_pp()) == WHATSIT_NODE
                    && (st.mem.b16s0(main_pp()) == NATIVE_WORD_NODE
                        || st.mem.b16s0(main_pp()) == NATIVE_WORD_NODE_AT)
                    && st.mem.b16s2(main_pp() + 4) == main_f() as u16
                    && main_ppp() != main_pp()
                    && !mc_is_char_node(st, main_ppp())
                    && st.mem.b16s1(main_ppp()) != DISC_NODE;

                if tail_is_native {
                    let old_len = st.mem.b16s1(main_pp() + 4) as i32;
                    main_k = main_h() + old_len;
                    while st.scanner.native_text_size() <= st.scanner.native_len() + main_k {
                        let size = st.scanner.native_text_size + 128;
                        st.scanner.native_text_size = size;
                        st.scanner.native_text.resize(size as usize, 0);
                    }
                    st.scanner.set_save_native_len(st.scanner.native_len());
                    for i in 0..old_len {
                        *st.scanner
                            .native_text
                            .as_mut_ptr()
                            .add(st.scanner.native_len() as usize) = *st
                            .mem
                            .as_u16_ptr_mut(main_pp() + NATIVE_NODE_SIZE)
                            .add(i as usize);
                        st.scanner.set_native_len(st.scanner.native_len() + 1);
                    }
                    for i in 0..main_h() {
                        *st.scanner
                            .native_text
                            .as_mut_ptr()
                            .add(st.scanner.native_len() as usize) = *st
                            .scanner
                            .native_text
                            .as_mut_ptr()
                            .add((temp_ptr + i) as usize);
                        st.scanner.set_native_len(st.scanner.native_len() + 1);
                    }
                    crate::linebreak::do_locale_linebreaks_in(
                        st,
                        st.scanner.save_native_len(),
                        main_k,
                    )?;
                    st.scanner.set_native_len(st.scanner.save_native_len());
                    main_k = st.scanner.native_len() - main_h() - temp_ptr;
                    temp_ptr = main_h();
                    set_main_h(0);
                    while main_h() < main_k
                        && (*st
                            .scanner
                            .native_text
                            .as_mut_ptr()
                            .add((temp_ptr + main_h()) as usize) as i32)
                            != st.fonts.hyphen_char_entry(main_f())
                        && (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) <= 0
                            || ((*st
                                .scanner
                                .native_text
                                .as_mut_ptr()
                                .add((temp_ptr + main_h()) as usize)
                                as i32)
                                != 8212
                                && (*st
                                    .scanner
                                    .native_text
                                    .as_mut_ptr()
                                    .add((temp_ptr + main_h()) as usize)
                                    as i32)
                                    != 8211))
                    {
                        set_main_h(main_h() + 1);
                    }
                    if main_h() < main_k {
                        set_main_h(main_h() + 1);
                    }
                    st.mem.set_b32s1(main_ppp(), st.mem.b32s1(main_pp()));
                    st.mem.set_b32s1(main_pp(), TEX_NULL);
                    crate::packaging::flush_node_list(&mut st.mem, main_pp())
                        .map_err(MainControlAbort::Confusion)?;
                    set_main_pp(cur_list_tail());
                    while st.mem.b32s1(main_ppp()) != main_pp() {
                        set_main_ppp(st.mem.b32s1(main_ppp()));
                    }
                } else {
                    crate::linebreak::do_locale_linebreaks_in(st, temp_ptr, main_h())?;
                    temp_ptr += main_h();
                    main_k -= main_h();
                    set_main_h(0);
                    while main_h() < main_k
                        && (*st
                            .scanner
                            .native_text
                            .as_mut_ptr()
                            .add((temp_ptr + main_h()) as usize) as i32)
                            != st.fonts.hyphen_char_entry(main_f())
                        && (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) <= 0
                            || ((*st
                                .scanner
                                .native_text
                                .as_mut_ptr()
                                .add((temp_ptr + main_h()) as usize)
                                as i32)
                                != 8212
                                && (*st
                                    .scanner
                                    .native_text
                                    .as_mut_ptr()
                                    .add((temp_ptr + main_h()) as usize)
                                    as i32)
                                    != 8211))
                    {
                        set_main_h(main_h() + 1);
                    }
                    if main_h() < main_k {
                        set_main_h(main_h() + 1);
                    }
                }

                if main_k > 0 || st.hyph.is_hyph() {
                    let nd = mc_new_disc(st)?;
                    st.mem.set_b32s1(cur_list_tail(), nd);
                    set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
                    set_main_pp(cur_list_tail());
                }

                if main_k == 0 {
                    break;
                }
            }
        } else {
            // MMODE (or not HMODE): simple native word node creation
            set_main_ppp(cur_list_head());
            while main_ppp() != main_pp() && st.mem.b32s1(main_ppp()) != main_pp() {
                if !mc_is_char_node(st, main_ppp()) && st.mem.b16s1(main_ppp()) == DISC_NODE {
                    let tp = main_ppp();
                    for _ in 1..=st.mem.b16s0(tp) as i32 {
                        set_main_ppp(st.mem.b32s1(main_ppp()));
                    }
                }
                if main_ppp() != main_pp() {
                    set_main_ppp(st.mem.b32s1(main_ppp()));
                }
            }

            let tail_is_native = main_pp() != TEX_NULL
                && !mc_is_char_node(st, main_pp())
                && st.mem.b16s1(main_pp()) == WHATSIT_NODE
                && (st.mem.b16s0(main_pp()) == NATIVE_WORD_NODE
                    || st.mem.b16s0(main_pp()) == NATIVE_WORD_NODE_AT)
                && st.mem.b16s2(main_pp() + 4) == mf as u16
                && main_ppp() != main_pp()
                && !mc_is_char_node(st, main_ppp())
                && st.mem.b16s1(main_ppp()) != DISC_NODE;

            if tail_is_native {
                let old_len = st.mem.b16s1(main_pp() + 4) as i32;
                let new_node =
                    crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, mf, mk + old_len)?;
                st.mem.set_b32s1(main_pp(), new_node);
                set_cur_list_tail(st.mem.b32s1(main_pp()));
                for i in 0..old_len {
                    *st.mem
                        .as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE)
                        .add(i as usize) = *st
                        .mem
                        .as_u16_ptr_mut(main_pp() + NATIVE_NODE_SIZE)
                        .add(i as usize);
                }
                for i in 0..mk {
                    *st.mem
                        .as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE)
                        .add((i + old_len) as usize) =
                        *st.scanner.native_text.as_mut_ptr().add(i as usize);
                }
                crate::ext::measure_native_node(
                    &st.fonts,
                    st.mem.node_addr(cur_list_tail()),
                    (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0) as libc::c_int,
                )?;
                let mut prev = cur_list_head();
                if prev != main_pp() {
                    while st.mem.b32s1(prev) != main_pp() {
                        prev = st.mem.b32s1(prev);
                    }
                }
                st.mem.set_b32s1(prev, st.mem.b32s1(main_pp()));
                st.mem.set_b32s1(main_pp(), TEX_NULL);
                let fnsz = st.mem.b16s3(main_pp() + 4) as i32;
                crate::mem::free_node(&mut st.mem, main_pp(), fnsz);
            } else {
                let new_node = crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, mf, mk)?;
                st.mem.set_b32s1(main_pp(), new_node);
                set_cur_list_tail(st.mem.b32s1(main_pp()));
                for i in 0..mk {
                    *st.mem
                        .as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE)
                        .add(i as usize) = *st.scanner.native_text.as_mut_ptr().add(i as usize);
                }
                crate::ext::measure_native_node(
                    &st.fonts,
                    st.mem.node_addr(cur_list_tail()),
                    (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0) as libc::c_int,
                )?;
            }
        }

        // Interword space shaping (both HMODE and MMODE)
        if st
            .eqtb
            .b32s1(INT_BASE + INT_PAR_XETEX_INTERWORD_SPACE_SHAPING)
            > 0
        {
            let mut p = cur_list_head();
            let mut pp = TEX_NULL;
            while p != cur_list_tail() {
                if p != TEX_NULL
                    && !mc_is_char_node(st, p)
                    && st.mem.b16s1(p) == WHATSIT_NODE
                    && (st.mem.b16s0(p) == NATIVE_WORD_NODE
                        || st.mem.b16s0(p) == NATIVE_WORD_NODE_AT)
                {
                    pp = p;
                }
                p = st.mem.b32s1(p);
            }
            if pp != TEX_NULL && st.mem.b16s2(pp + 4) == mf as u16 {
                let mut p2 = st.mem.b32s1(pp);
                while !mc_is_char_node(st, p2)
                    && (st.mem.b16s1(p2) == PENALTY_NODE
                        || st.mem.b16s1(p2) == INS_NODE
                        || st.mem.b16s1(p2) == MARK_NODE
                        || st.mem.b16s1(p2) == ADJUST_NODE
                        || (st.mem.b16s1(p2) == WHATSIT_NODE && st.mem.b16s0(p2) <= 4))
                {
                    p2 = st.mem.b32s1(p2);
                }
                if !mc_is_char_node(st, p2) && st.mem.b16s1(p2) == GLUE_NODE {
                    let p3 = st.mem.b32s1(p2);
                    let mut p3m = p3;
                    while !mc_is_char_node(st, p3m)
                        && (st.mem.b16s1(p3m) == PENALTY_NODE
                            || st.mem.b16s1(p3m) == INS_NODE
                            || st.mem.b16s1(p3m) == MARK_NODE
                            || st.mem.b16s1(p3m) == ADJUST_NODE
                            || (st.mem.b16s1(p3m) == WHATSIT_NODE && st.mem.b16s0(p3m) <= 4))
                    {
                        p3m = st.mem.b32s1(p3m);
                    }
                    if p3m == cur_list_tail() {
                        let pp_len = st.mem.b16s1(pp + 4) as i32;
                        let tail_len = st.mem.b16s1(cur_list_tail() + 4) as i32;
                        let tmp = crate::fonts::new_native_word_node(
                            &mut st.mem,
                            &st.eqtb,
                            mf,
                            pp_len + 1 + tail_len,
                        )?;
                        let mut ki = 0i32;
                        for t in 0..pp_len {
                            *st.mem
                                .as_u16_ptr_mut(tmp + NATIVE_NODE_SIZE)
                                .add(ki as usize) =
                                *st.mem.as_u16_ptr_mut(pp + NATIVE_NODE_SIZE).add(t as usize);
                            ki += 1;
                        }
                        *st.mem
                            .as_u16_ptr_mut(tmp + NATIVE_NODE_SIZE)
                            .add(ki as usize) = b' ' as u16;
                        ki += 1;
                        for t in 0..tail_len {
                            *st.mem
                                .as_u16_ptr_mut(tmp + NATIVE_NODE_SIZE)
                                .add(ki as usize) = *st
                                .mem
                                .as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE)
                                .add(t as usize);
                            ki += 1;
                        }
                        crate::ext::measure_native_node(
                            &st.fonts,
                            st.mem.node_addr(tmp),
                            (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0)
                                as libc::c_int,
                        )?;
                        let t_val = st.mem.b32s1(tmp + 1)
                            - st.mem.b32s1(pp + 1)
                            - st.mem.b32s1(cur_list_tail() + 1);
                        let fnsz = st.mem.b16s3(tmp + 4) as i32;
                        crate::mem::free_node(&mut st.mem, tmp, fnsz);
                        if t_val != st.mem.b32s1(st.fonts.font_glue(mf) + 1) {
                            let kw = t_val - st.mem.b32s1(st.fonts.font_glue(mf) + 1);
                            let kn = mc_new_kern(st, kw)?;
                            st.mem.set_b16s0(kn, SPACE_ADJUSTMENT);
                            st.mem.set_b32s1(kn, st.mem.b32s1(p2));
                            st.mem.set_b32s1(p2, kn);
                        }
                    }
                }
            }
        }

        // Exit: cur_ptr set iff inter-char token injected
        return Ok(st.scanner.cur_ptr != TEX_NULL);
    }

    // =========================================================================
    // Regular TFM font path
    // =========================================================================

    // Update space factor from SF_CODE(cur_chr)
    let sf = st.eqtb.b32s1(SF_CODE_BASE + st.scanner.cur_chr);
    let ms = sf % 65536;
    set_main_s(ms);
    if ms == 1000 {
        set_cur_list_aux_b32s0(1000);
    } else if ms < 1000 {
        if ms > 0 {
            set_cur_list_aux_b32s0(ms);
        }
    } else if cur_list_aux_b32s0() < 1000 {
        set_cur_list_aux_b32s0(1000);
    } else {
        set_cur_list_aux_b32s0(ms);
    }

    st.scanner.cur_ptr = TEX_NULL;
    set_space_class(sf / 65536);
    let sc = space_class();

    if inter_char_before_current_char_action(st, sc, LETTER, OTHER_CHAR).execute_in(st)? {
        return Ok(true); // goto big_switch
    }
    if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0 && sc != CHAR_CLASS_LIMIT {
        set_prev_class(sc);
    }

    // Set up font state
    set_main_f(mf);
    set_bchar(st.fonts.font_bchar_entry(mf));
    set_false_bchar(st.fonts.font_false_bchar_entry(mf));

    if cur_list_mode() > 0 && st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE) != cur_list_aux_b32s1() {
        fix_language(&mut st.mem, &st.eqtb)?;
    }

    // Allocate lig_stack node from avail list
    let ls = if st.mem.avail() == TEX_NULL {
        mc_get_avail(st)?
    } else {
        let a = st.mem.avail();
        st.mem.set_avail(st.mem.b32s1(a));
        st.mem.set_b32s1(a, TEX_NULL);
        a
    };
    set_lig_stack(ls);
    st.mem.set_b16s1(ls, mf as u16);
    set_cur_l(st.scanner.cur_chr);
    st.mem.set_b16s0(ls, cur_l() as u16);
    set_cur_q(cur_list_tail());

    // Determine entry point: NON_ADDRESS → move_2; else → lig_loop_1 (bchar)
    let bchar_lbl = if cancel_boundary() {
        set_cancel_boundary(false);
        NON_ADDRESS
    } else {
        st.fonts.bchar_label_entry(mf)
    };

    // `enter_lig1_k`: if Some(k), enter lig loop at _1 with that k; else enter at move_2
    let mut enter_lig1_k: Option<i32> = if bchar_lbl != NON_ADDRESS {
        set_cur_r(cur_l());
        set_cur_l(TOO_BIG_CHAR);
        Some(bchar_lbl)
    } else {
        None
    };

    // main_i: char info for cur_l (set at move_2 or when cur_l changes in lig loop)
    let mut main_i = crate::mem_types::B16x4 {
        s0: 0,
        s1: 0,
        s2: 0,
        s3: 0,
    };

    // -------------------------------------------------------------------------
    // Outer char-processing loop.
    // Each iteration: run the lig/kern table, then advance to the next char.
    // -------------------------------------------------------------------------
    'char_proc: loop {
        // Determine how to enter the lig/kern loop this iteration:
        // - enter_lig1_k = Some(k): start at lig_loop_1 (skips the precondition check)
        // - enter_lig1_k = None: start at lig_loop top (checks LIG_TAG and cur_r)

        // "lig_result": 0=wrapup, 1=goto_move (skip wrapup), 2=goto_move1 (skip wrapup+move-check)
        let lig_result: u8;

        if let Some(init_k) = enter_lig1_k.take() {
            // Enter at main_lig_loop_1 directly (bchar case or after lig advance)
            lig_result = lig_inner_loop(st, init_k, &mut main_i, &mut enter_lig1_k)?;
        } else {
            // Enter at main_lig_loop top: check preconditions
            if (main_i.s1 % 4) as u8 != LIG_TAG as u8 || cur_r() == TOO_BIG_CHAR {
                lig_result = 0; // goto main_loop_wrapup
            } else {
                let init_k = st.fonts.lig_kern_base_entry(main_f()) + main_i.s0 as i32;
                let init_j = st.fonts.font_info_b16(init_k);
                let real_k = if init_j.s3 > 128 {
                    st.fonts.lig_kern_base_entry(main_f())
                        + 256 * init_j.s1 as i32
                        + init_j.s0 as i32
                        + 32768
                        - 256 * 128
                } else {
                    init_k
                };
                lig_result = lig_inner_loop(st, real_k, &mut main_i, &mut enter_lig1_k)?;
            }
        }

        // -----------------------------------------------------------------------
        // Handle lig_result: wrapup, move, or move1
        // -----------------------------------------------------------------------
        // 0xFF: lig type 1/2/3/7 advanced cur_l; re-enter lig_loop top (no wrapup/move)
        if lig_result == 0xFF {
            continue 'char_proc;
        }

        if lig_result == 0 {
            // main_loop_wrapup: only when lig_result==0 (result 1=goto_move, 2=goto_move1 both skip wrapup)
            if cur_l() < TOO_BIG_CHAR {
                if st.mem.b32s1(cur_q()) > TEX_NULL
                    && st.mem.b16s0(cur_list_tail()) == st.fonts.hyphen_char_entry(main_f()) as u16
                {
                    set_ins_disc(true);
                }
                if ligature_present() {
                    let lq = st.mem.b32s1(cur_q());
                    let mp = mc_new_ligature(st, main_f(), cur_l() as u16, lq)?;
                    if lft_hit() {
                        st.mem.set_b16s0(mp, 2);
                        set_lft_hit(false);
                    }
                    if rt_hit() && lig_stack() == TEX_NULL {
                        st.mem.set_b16s0(mp, st.mem.b16s0(mp) + 1);
                        set_rt_hit(false);
                    }
                    st.mem.set_b32s1(cur_q(), mp);
                    set_cur_list_tail(mp);
                    set_ligature_present(false);
                }
                if ins_disc() {
                    set_ins_disc(false);
                    if cur_list_mode() > 0 {
                        let nd = mc_new_disc(st)?;
                        st.mem.set_b32s1(cur_list_tail(), nd);
                        set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
                    }
                }
            }
        }
        // If lig_result == 1: skip wrapup, go directly to move

        // main_loop_move: check if lig_stack is exhausted
        // (for lig_result==2 / move1: skip this check, cur_l already set)
        if lig_result != 2 {
            if lig_stack() == TEX_NULL {
                return Ok(false); // goto reswitch
            }
            set_cur_q(cur_list_tail());
            set_cur_l(st.mem.b16s0(lig_stack()) as i32);
        }

        // main_loop_move_1: is lig_stack a char node or a lig item?
        if !mc_is_char_node(st, lig_stack()) {
            // main_loop_move_lig: handle lig item in lig_stack
            set_main_p(st.mem.b32s1(lig_stack() + 1));
            if main_p() > TEX_NULL {
                st.mem.set_b32s1(cur_list_tail(), main_p());
                set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
            }
            let temp = lig_stack();
            set_lig_stack(st.mem.b32s1(temp));
            crate::mem::free_node(&mut st.mem, temp, SMALL_NODE_SIZE);
            let ec = crate::fonts::effective_char(&mut st.fonts, true, main_f(), cur_l() as u16);
            main_i = st
                .fonts
                .font_info_b16(st.fonts.char_base_entry(main_f()) + ec);
            set_ligature_present(true);

            if lig_stack() == TEX_NULL {
                if main_p() > TEX_NULL {
                    // goto main_loop_lookahead (enter with main_i set, skip move_2)
                    // We do the lookahead inline here:
                    if !do_lookahead(st, &mut main_i, &mut enter_lig1_k, mf)? {
                        return Ok(true); // inter-char token injected → big_switch
                    }
                    continue 'char_proc;
                } else {
                    set_cur_r(bchar());
                }
            } else {
                set_cur_r(st.mem.b16s0(lig_stack()) as i32);
            }
            // goto main_lig_loop (outer top) — continue 'char_proc
            continue 'char_proc;
        }

        // main_loop_move_2: validate and append the char node (lig_stack is char)
        let eff = crate::fonts::effective_char(&mut st.fonts, false, main_f(), st.scanner.cur_chr as u16);
        if eff > st.fonts.font_ec_entry(main_f()) as i32
            || eff < st.fonts.font_bc_entry(main_f()) as i32
        {
            mc_propagate_error(crate::output::char_warning_in(st, main_f(), st.scanner.cur_chr))?;
            st.mem.set_b32s1(lig_stack(), st.mem.avail());
            st.mem.set_avail(lig_stack());
            return Ok(true); // goto big_switch
        }
        main_i = crate::fonts::effective_char_info(&mut st.fonts, main_f(), cur_l() as u16);
        if main_i.s3 == 0 {
            mc_propagate_error(crate::output::char_warning_in(st, main_f(), st.scanner.cur_chr))?;
            st.mem.set_b32s1(lig_stack(), st.mem.avail());
            st.mem.set_avail(lig_stack());
            return Ok(true); // goto big_switch
        }
        st.mem.set_b32s1(cur_list_tail(), lig_stack());
        set_cur_list_tail(lig_stack());

        // main_loop_lookahead: get next token, set up lig_stack for it
        if !do_lookahead(st, &mut main_i, &mut enter_lig1_k, mf)? {
            return Ok(true); // inter-char token injected → big_switch
        }
        // continue 'char_proc → back to top of lig loop
    } // end 'char_proc
}
// Inner lig/kern table scan (main_lig_loop_1 / main_lig_loop_2).
// Returns: 0=wrapup, 1=goto_move (skip wrapup), 2=goto_move1 (lig type 0/4 with stack)
// Also sets enter_lig1_k if we need to re-enter lig_loop_1 (bchar advance).
#[inline(always)]
unsafe fn lig_inner_loop(
    st: &mut crate::state::EngineState,
    init_k: i32,
    main_i: &mut crate::mem_types::B16x4,
    enter_lig1_k: &mut Option<i32>,
) -> Result<u8, MainControlAbort> {
    use crate::globals::{
        cur_l, cur_list_mode, cur_list_tail, cur_q, cur_r, ins_disc, lft_hit, lig_stack,
        ligature_present, main_f, set_bchar, set_cur_l, set_cur_list_tail, set_cur_q, set_cur_r,
        set_ins_disc, set_lft_hit, set_lig_stack, set_ligature_present, set_rt_hit,
    };

    let mut mk = init_k;

    loop {
        // main_lig_loop_1 / main_lig_loop_2
        let mj = st.fonts.font_info_b16(mk);

        if mj.s2 as i32 == cur_r() {
            if mj.s3 <= 128 {
                if mj.s1 >= 128 {
                    // Kern case: partial wrapup + append kern
                    if cur_l() < TOO_BIG_CHAR {
                        if st.mem.b32s1(cur_q()) > TEX_NULL
                            && st.mem.b16s0(cur_list_tail())
                                == st.fonts.hyphen_char_entry(main_f()) as u16
                        {
                            set_ins_disc(true);
                        }
                        if ligature_present() {
                            let lq = st.mem.b32s1(cur_q());
                            let mp = mc_new_ligature(st, main_f(), cur_l() as u16, lq)?;
                            if lft_hit() {
                                st.mem.set_b16s0(mp, 2);
                                set_lft_hit(false);
                            }
                            // rt_hit: only if lig_stack == NULL (not applicable here since we have kern)
                            st.mem.set_b32s1(cur_q(), mp);
                            set_cur_list_tail(mp);
                            set_ligature_present(false);
                        }
                        if ins_disc() {
                            set_ins_disc(false);
                            if cur_list_mode() > 0 {
                                let nd = mc_new_disc(st)?;
                                st.mem.set_b32s1(cur_list_tail(), nd);
                                set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
                            }
                        }
                    }
                    let kval = st.fonts.font_info_b32s1(
                        st.fonts.kern_base_entry(main_f()) + 256 * mj.s1 as i32 + mj.s0 as i32,
                    );
                    let kn = mc_new_kern(st, kval)?;
                    st.mem.set_b32s1(cur_list_tail(), kn);
                    set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
                    return Ok(1); // goto main_loop_move (skip wrapup)
                }

                // Ligature case
                if cur_l() == TOO_BIG_CHAR {
                    set_lft_hit(true);
                } else if lig_stack() == TEX_NULL {
                    set_rt_hit(true);
                }

                match mj.s1 {
                    1 | 5 => {
                        set_cur_l(mj.s0 as i32);
                        let ec = crate::fonts::effective_char(
                            &mut st.fonts,
                            true,
                            main_f(),
                            cur_l() as u16,
                        );
                        *main_i = st
                            .fonts
                            .font_info_b16(st.fonts.char_base_entry(main_f()) + ec);
                        set_ligature_present(true);
                    }
                    2 | 6 => {
                        set_cur_r(mj.s0 as i32);
                        if lig_stack() == TEX_NULL {
                            set_lig_stack(mc_new_lig_item(st, cur_r() as u16)?);
                            set_bchar(TOO_BIG_CHAR);
                        } else if mc_is_char_node(st, lig_stack()) {
                            let prev = lig_stack();
                            set_lig_stack(mc_new_lig_item(st, cur_r() as u16)?);
                            st.mem.set_b32s1(lig_stack() + 1, prev);
                        } else {
                            st.mem.set_b16s0(lig_stack(), cur_r() as u16);
                        }
                    }
                    3 => {
                        set_cur_r(mj.s0 as i32);
                        let prev = lig_stack();
                        set_lig_stack(mc_new_lig_item(st, cur_r() as u16)?);
                        st.mem.set_b32s1(lig_stack(), prev);
                    }
                    7 | 11 => {
                        // Partial wrapup
                        if cur_l() < TOO_BIG_CHAR {
                            if st.mem.b32s1(cur_q()) > TEX_NULL
                                && st.mem.b16s0(cur_list_tail())
                                    == st.fonts.hyphen_char_entry(main_f()) as u16
                            {
                                set_ins_disc(true);
                            }
                            if ligature_present() {
                                let lq = st.mem.b32s1(cur_q());
                                let mp = mc_new_ligature(st, main_f(), cur_l() as u16, lq)?;
                                if lft_hit() {
                                    st.mem.set_b16s0(mp, 2);
                                    set_lft_hit(false);
                                }
                                // no rt_hit for type 7/11
                                st.mem.set_b32s1(cur_q(), mp);
                                set_cur_list_tail(mp);
                                set_ligature_present(false);
                            }
                            if ins_disc() {
                                set_ins_disc(false);
                                if cur_list_mode() > 0 {
                                    let nd = mc_new_disc(st)?;
                                    st.mem.set_b32s1(cur_list_tail(), nd);
                                    set_cur_list_tail(st.mem.b32s1(cur_list_tail()));
                                }
                            }
                        }
                        set_cur_q(cur_list_tail());
                        set_cur_l(mj.s0 as i32);
                        let ec = crate::fonts::effective_char(
                            &mut st.fonts,
                            true,
                            main_f(),
                            cur_l() as u16,
                        );
                        *main_i = st
                            .fonts
                            .font_info_b16(st.fonts.char_base_entry(main_f()) + ec);
                        set_ligature_present(true);
                    }
                    _ => {
                        // Type 0 or 4 (default): =: or |=:|
                        set_cur_l(mj.s0 as i32);
                        set_ligature_present(true);
                        if lig_stack() == TEX_NULL {
                            return Ok(0); // goto main_loop_wrapup
                        } else {
                            return Ok(2); // goto main_loop_move_1
                        }
                    }
                }

                // Post-switch tail for types 1,2,3,5,6,7,11:
                if mj.s1 > 4 && mj.s1 != 7 {
                    return Ok(0); // types 5,6,11 → wrapup
                }
                // Types 1,2,3,7: re-enter lig loop
                if cur_l() < TOO_BIG_CHAR {
                    // re-enter main_lig_loop (outer): signal caller via enter_lig1_k = None
                    // The outer loop will re-enter at top (checking LIG_TAG)
                    // We need to break out and let caller restart 'char_proc at lig_loop top
                    return Ok(0xFF); // sentinel: re-enter lig_loop (not wrapup!)
                }
                // cur_l == TOO_BIG_CHAR: re-enter at lig_loop_1 with bchar_label
                mk = st.fonts.bchar_label_entry(main_f());
                // continue inner loop (main_lig_loop_1)
            }
            // mj.s3 > 128: this entry's match byte > 128 means we skip (it's a stop/redirect)
            // but we already handled the redirect at the top. If we reach here with s3 > 128
            // in the inner loop, it means no match → go to wrapup.
            // Actually: if s3 > 128 when s2 == cur_r, that would be a redirect, but redirects
            // are only in the first entry and we've already computed the real index.
            // So s3 > 128 here when s2 == cur_r can't happen after the first entry redirect.
            // The s3 > 128 at line 7722 means "first entry has large byte → use redirect".
            // After redirect, s3 values in subsequent entries are normal (0-128).
        } else {
            // No match for this lig/kern table entry: advance
            if mj.s3 == 0 {
                mk += 1;
            } else if mj.s3 >= 128 {
                return Ok(0); // no more entries → wrapup
            } else {
                mk += mj.s3 as i32 + 1;
            }
            // continue inner loop (main_lig_loop_1)
        }
    }
}
// Lookahead: get next token and set up lig_stack for it.
// Returns false if an inter-char token was injected (caller should return true = big_switch).
// On normal return, cur_r and lig_stack are set for the next lig/kern iteration.
#[inline(always)]
unsafe fn do_lookahead(
    st: &mut crate::state::EngineState,
    main_i: &mut crate::mem_types::B16x4,
    enter_lig1_k: &mut Option<i32>,
    _mf: i32,
) -> Result<bool, MainControlAbort> {
    use crate::globals::{
        bchar, cur_list_aux_b32s0, false_bchar, main_f, prev_class, set_bchar,
        set_cur_list_aux_b32s0, set_cur_r, set_lig_stack, set_prev_class,
        set_space_class, space_class,
    };

    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const CHAR_GIVEN: u8 = 68;
    const CHAR_NUM: u8 = 16;
    const NO_BOUNDARY: u8 = 65;

    crate::scan::get_next_scc(st)?;

    let is_char_cmd = |cmd: u8| cmd == LETTER || cmd == OTHER_CHAR || cmd == CHAR_GIVEN;

    if !is_char_cmd(st.scanner.cur_cmd) {
        crate::scan::x_token_in(st)?;
    }
    if !is_char_cmd(st.scanner.cur_cmd) {
        if st.scanner.cur_cmd == CHAR_NUM {
            crate::scan::scan_char_num_in(st)?;
            st.scanner.cur_chr = st.scanner.cur_val;
            // fall through to is_char_cmd path
        } else {
            // Not a char: set cur_r = bchar, lig_stack = NULL
            if st.scanner.cur_cmd == NO_BOUNDARY {
                set_bchar(TOO_BIG_CHAR);
            }
            set_cur_r(bchar());
            set_lig_stack(crate::mem::TEX_NULL);
            return Ok(true); // normal: re-enter lig loop at top
        }
    }

    // main_loop_lookahead_1: we have a char in cur_chr
    let sf = st.eqtb.b32s1(SF_CODE_BASE + st.scanner.cur_chr);
    let ms = sf % 65536;
    if ms == 1000 {
        set_cur_list_aux_b32s0(1000);
    } else if ms < 1000 {
        if ms > 0 {
            set_cur_list_aux_b32s0(ms);
        }
    } else if cur_list_aux_b32s0() < 1000 {
        set_cur_list_aux_b32s0(1000);
    } else {
        set_cur_list_aux_b32s0(ms);
    }
    st.scanner.cur_ptr = crate::mem::TEX_NULL;
    set_space_class(sf / 65536);
    let sc = space_class();

    // Inter-char check (lookahead_1 variant)
    let was_boundary = prev_class() == CHAR_CLASS_LIMIT - 1;
    if inter_char_before_current_char_action(st, sc, LETTER, OTHER_CHAR).execute_in(st)? {
        if !was_boundary {
            set_prev_class(CHAR_CLASS_LIMIT - 1);
        }
        return Ok(false); // inter-char injected → caller returns true (big_switch)
    }
    if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0 && sc != CHAR_CLASS_LIMIT {
        set_prev_class(sc);
    }

    // Allocate lig_stack for the new char
    let ls = if st.mem.avail() == crate::mem::TEX_NULL {
        mc_get_avail(st)?
    } else {
        let a = st.mem.avail();
        st.mem.set_avail(st.mem.b32s1(a));
        st.mem.set_b32s1(a, crate::mem::TEX_NULL);
        a
    };
    set_lig_stack(ls);
    st.mem.set_b16s1(ls, main_f() as u16);
    let cr = st.scanner.cur_chr;
    set_cur_r(cr);
    st.mem.set_b16s0(ls, cr as u16);
    if cr == false_bchar() {
        set_cur_r(TOO_BIG_CHAR);
    }

    Ok(true) // normal lookahead complete, continue lig loop
}
// translated from xetex-xetex0.c:6766 (main_control)
// Top-level TeX command dispatcher: big_switch / reswitch loop.
//
// The public wrapper is the thread-local anchor; the full command loop holds
// one state borrow and threads it through every dispatcher below this point.
pub(crate) unsafe fn main_control_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    // Mode constants (xetex_format.h)
    const VMODE: u16 = 1;
    const HMODE: u16 = 104;
    const MMODE: u16 = 207;

    // Command constants (xetex_format.h)
    const RELAX: u8 = 0;
    const LEFT_BRACE: u8 = 1;
    const RIGHT_BRACE: u8 = 2;
    const MATH_SHIFT: u8 = 3;
    const TAB_MARK: u8 = 4;
    const CAR_RET: u8 = 5;
    const MAC_PARAM: u8 = 6;
    const SUP_MARK: u8 = 7;
    const SUB_MARK: u8 = 8;
    const ENDV: u8 = 9;
    const SPACER: u8 = 10;
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const PAR_END: u8 = 13;
    const STOP: u8 = 14;
    const DELIM_NUM: u8 = 15;
    const CHAR_NUM: u8 = 16;
    const MATH_CHAR_NUM: u8 = 17;
    const MARK: u8 = 18;
    const XRAY: u8 = 19;
    const MAKE_BOX: u8 = 20;
    const HMOVE: u8 = 21;
    const VMOVE: u8 = 22;
    const UN_HBOX: u8 = 23;
    const UN_VBOX: u8 = 24;
    const REMOVE_ITEM: u8 = 25;
    const HSKIP: u8 = 26;
    const VSKIP: u8 = 27;
    const MSKIP: u8 = 28;
    const KERN: u8 = 29;
    const MKERN: u8 = 30;
    const LEADER_SHIP: u8 = 31;
    const HALIGN: u8 = 32;
    const VALIGN: u8 = 33;
    const NO_ALIGN: u8 = 34;
    const VRULE: u8 = 35;
    const HRULE: u8 = 36;
    const INSERT: u8 = 37;
    const VADJUST: u8 = 38;
    const IGNORE_SPACES: u8 = 39;
    const AFTER_ASSIGNMENT: u8 = 40;
    const AFTER_GROUP: u8 = 41;
    const BREAK_PENALTY: u8 = 42;
    const START_PAR: u8 = 43;
    const ITAL_CORR: u8 = 44;
    const ACCENT: u8 = 45;
    const MATH_ACCENT: u8 = 46;
    const DISCRETIONARY: u8 = 47;
    const EQ_NO: u8 = 48;
    const LEFT_RIGHT: u8 = 49;
    const MATH_COMP: u8 = 50;
    const LIMIT_SWITCH: u8 = 51;
    const ABOVE: u8 = 52;
    const MATH_STYLE: u8 = 53;
    const MATH_CHOICE: u8 = 54;
    const NON_SCRIPT: u8 = 55;
    const VCENTER: u8 = 56;
    const CASE_SHIFT: u8 = 57;
    const MESSAGE: u8 = 58;
    const EXTENSION: u8 = 59;
    const IN_STREAM: u8 = 60;
    const BEGIN_GROUP: u8 = 61;
    const END_GROUP: u8 = 62;
    const OMIT: u8 = 63;
    const EX_SPACE: u8 = 64;
    const NO_BOUNDARY: u8 = 65;
    const RADICAL: u8 = 66;
    const END_CS_NAME: u8 = 67;
    const CHAR_GIVEN: u8 = 68;
    const MATH_GIVEN: u8 = 69;
    const XETEX_MATH_GIVEN: u8 = 70;
    const LAST_ITEM: u8 = 71;
    const TOKS_REGISTER: u8 = 72;
    const ASSIGN_TOKS: u8 = 73;
    const ASSIGN_INT: u8 = 74;
    const ASSIGN_DIMEN: u8 = 75;
    const ASSIGN_GLUE: u8 = 76;
    const ASSIGN_MU_GLUE: u8 = 77;
    const ASSIGN_FONT_DIMEN: u8 = 78;
    const ASSIGN_FONT_INT: u8 = 79;
    const SET_AUX: u8 = 80;
    const SET_PREV_GRAF: u8 = 81;
    const SET_PAGE_DIMEN: u8 = 82;
    const SET_PAGE_INT: u8 = 83;
    const SET_BOX_DIMEN: u8 = 84;
    const SET_SHAPE: u8 = 85;
    const DEF_CODE: u8 = 86;
    const XETEX_DEF_CODE: u8 = 87;
    const DEF_FAMILY: u8 = 88;
    const SET_FONT: u8 = 89;
    const DEF_FONT: u8 = 90;
    const REGISTER: u8 = 91;
    const ADVANCE: u8 = 92;
    const MULTIPLY: u8 = 93;
    const DIVIDE: u8 = 94;
    const PREFIX: u8 = 95;
    const LET: u8 = 96;
    const SHORTHAND_DEF: u8 = 97;
    const READ_TO_CS: u8 = 98;
    const DEF: u8 = 99;
    const SET_BOX: u8 = 100;
    const HYPH_DATA: u8 = 101;
    const SET_INTERACTION: u8 = 102;

    const CC_LOCAL_BASE: i32 = 2_254_871; // xetex_format.h:208
    const CC_LOCAL_EVERY_JOB: i32 = 7; // xetex_format.h:179
    const EVERY_JOB_TEXT: u16 = 13; // xetex-constants.h:212
    const INT_PAR_TRACING_COMMANDS: i32 = 36; // xetex_format.h:71

    use crate::globals::{cur_list_mode, set_cancel_boundary};
    use crate::mem::TEX_NULL;

    let every_job = st.eqtb.b32s1(CC_LOCAL_BASE + CC_LOCAL_EVERY_JOB);
    if every_job != TEX_NULL {
        crate::input::begin_token_list_in(st, every_job, EVERY_JOB_TEXT)?;
    }

    loop {
        crate::scan::get_x_token_in(st)?;

        'reswitch: loop {
            if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 0 {
                crate::output::show_cur_cmd_chr_in(st);
            }

            let mode = cur_list_mode().unsigned_abs();
            let cmd = st.scanner.cur_cmd;

            // Fast char path and boundary check (first switch in C, before inter-char check)
            match (mode, cmd) {
                (HMODE, LETTER) | (HMODE, OTHER_CHAR) | (HMODE, CHAR_GIVEN) => {
                    if main_control_main_loop_in(st)? {
                        break 'reswitch;
                    }
                    continue 'reswitch;
                }
                (HMODE, CHAR_NUM) => {
                    crate::scan::scan_usv_num_in(st)?;
                    st.scanner.cur_chr = st.scanner.cur_val;
                    if main_control_main_loop_in(st)? {
                        break 'reswitch;
                    }
                    continue 'reswitch;
                }
                (HMODE, NO_BOUNDARY) => {
                    crate::scan::get_x_token_in(st)?;
                    if matches!(st.scanner.cur_cmd, LETTER | OTHER_CHAR | CHAR_GIVEN | CHAR_NUM) {
                        set_cancel_boundary(true);
                    }
                    continue 'reswitch;
                }
                _ => {}
            }

            if main_control_hmode_inter_char_check_in(st)? {
                break 'reswitch; // → big_switch
            }

            // Main command dispatch (second switch in C)
            match (mode, cmd) {
                // Spacer/ex_space → spacer_cases
                (HMODE, SPACER) | (HMODE, EX_SPACE) | (MMODE, EX_SPACE) => {
                    main_control_spacer_cases_in(st)?;
                }
                // No-op: ANY_MODE(RELAX), VMODE+SPACER, MMODE+SPACER, MMODE+NO_BOUNDARY
                (HMODE, RELAX)
                | (VMODE, RELAX)
                | (MMODE, RELAX)
                | (VMODE, SPACER)
                | (MMODE, SPACER)
                | (MMODE, NO_BOUNDARY) => {}
                // ANY_MODE(IGNORE_SPACES) → call helper then reswitch
                (HMODE, IGNORE_SPACES) | (VMODE, IGNORE_SPACES) | (MMODE, IGNORE_SPACES) => {
                    main_control_ignore_spaces_in(st)?;
                    continue 'reswitch;
                }
                // VMODE+STOP → "the only way out"
                (VMODE, STOP) => {
                    if its_all_over_in(st)? {
                        return Ok(());
                    }
                }
                // Forbidden / math-mode-mismatch error cases
                (VMODE, VMOVE)
                | (HMODE, HMOVE)
                | (MMODE, HMOVE)
                | (VMODE, LAST_ITEM)
                | (HMODE, LAST_ITEM)
                | (MMODE, LAST_ITEM)
                | (VMODE, VADJUST)
                | (VMODE, ITAL_CORR)
                | (VMODE, EQ_NO)
                | (HMODE, EQ_NO)
                | (VMODE, MAC_PARAM)
                | (HMODE, MAC_PARAM)
                | (MMODE, MAC_PARAM)
                | (VMODE, SUP_MARK)
                | (HMODE, SUP_MARK)
                | (VMODE, SUB_MARK)
                | (HMODE, SUB_MARK)
                | (VMODE, MATH_CHAR_NUM)
                | (HMODE, MATH_CHAR_NUM)
                | (VMODE, MATH_GIVEN)
                | (HMODE, MATH_GIVEN)
                | (VMODE, XETEX_MATH_GIVEN)
                | (HMODE, XETEX_MATH_GIVEN)
                | (VMODE, MATH_COMP)
                | (HMODE, MATH_COMP)
                | (VMODE, DELIM_NUM)
                | (HMODE, DELIM_NUM)
                | (VMODE, LEFT_RIGHT)
                | (HMODE, LEFT_RIGHT)
                | (VMODE, ABOVE)
                | (HMODE, ABOVE)
                | (VMODE, RADICAL)
                | (HMODE, RADICAL)
                | (VMODE, MATH_STYLE)
                | (HMODE, MATH_STYLE)
                | (VMODE, MATH_CHOICE)
                | (HMODE, MATH_CHOICE)
                | (VMODE, VCENTER)
                | (HMODE, VCENTER)
                | (VMODE, NON_SCRIPT)
                | (HMODE, NON_SCRIPT)
                | (VMODE, MKERN)
                | (HMODE, MKERN)
                | (VMODE, LIMIT_SWITCH)
                | (HMODE, LIMIT_SWITCH)
                | (VMODE, MSKIP)
                | (HMODE, MSKIP)
                | (VMODE, MATH_ACCENT)
                | (HMODE, MATH_ACCENT)
                | (MMODE, ENDV)
                | (MMODE, PAR_END)
                | (MMODE, STOP)
                | (MMODE, VSKIP)
                | (MMODE, UN_VBOX)
                | (MMODE, VALIGN)
                | (MMODE, HRULE) => {
                    main_control_mode_error_cases_in(st)?;
                }
                // Rule/skip/kern/group cases
                (VMODE, HRULE)
                | (HMODE, VRULE)
                | (MMODE, VRULE)
                | (VMODE, VSKIP)
                | (HMODE, HSKIP)
                | (MMODE, HSKIP)
                | (MMODE, MSKIP)
                | (VMODE, KERN)
                | (HMODE, KERN)
                | (MMODE, KERN)
                | (MMODE, MKERN)
                | (VMODE, LEFT_BRACE)
                | (HMODE, LEFT_BRACE)
                | (VMODE, BEGIN_GROUP)
                | (HMODE, BEGIN_GROUP)
                | (MMODE, BEGIN_GROUP)
                | (VMODE, END_GROUP)
                | (HMODE, END_GROUP)
                | (MMODE, END_GROUP) => {
                    main_control_simple_list_group_cases_in(st)?;
                }
                // Box/list construction cases
                (VMODE, RIGHT_BRACE)
                | (HMODE, RIGHT_BRACE)
                | (MMODE, RIGHT_BRACE)
                | (VMODE, HMOVE)
                | (HMODE, VMOVE)
                | (MMODE, VMOVE)
                | (VMODE, LEADER_SHIP)
                | (HMODE, LEADER_SHIP)
                | (MMODE, LEADER_SHIP)
                | (VMODE, MAKE_BOX)
                | (HMODE, MAKE_BOX)
                | (MMODE, MAKE_BOX)
                | (VMODE, START_PAR)
                | (VMODE, LETTER)
                | (VMODE, OTHER_CHAR)
                | (VMODE, CHAR_NUM)
                | (VMODE, CHAR_GIVEN)
                | (VMODE, MATH_SHIFT)
                | (VMODE, UN_HBOX)
                | (VMODE, VRULE)
                | (VMODE, ACCENT)
                | (VMODE, DISCRETIONARY)
                | (VMODE, HSKIP)
                | (VMODE, VALIGN)
                | (VMODE, EX_SPACE)
                | (VMODE, NO_BOUNDARY)
                | (HMODE, START_PAR)
                | (MMODE, START_PAR)
                | (VMODE, PAR_END)
                | (HMODE, PAR_END)
                | (HMODE, STOP)
                | (HMODE, VSKIP)
                | (HMODE, HRULE)
                | (HMODE, UN_VBOX)
                | (HMODE, HALIGN)
                | (VMODE, INSERT)
                | (HMODE, INSERT)
                | (MMODE, INSERT)
                | (HMODE, VADJUST)
                | (MMODE, VADJUST)
                | (VMODE, MARK)
                | (HMODE, MARK)
                | (MMODE, MARK)
                | (VMODE, BREAK_PENALTY)
                | (HMODE, BREAK_PENALTY)
                | (MMODE, BREAK_PENALTY)
                | (VMODE, REMOVE_ITEM)
                | (HMODE, REMOVE_ITEM)
                | (MMODE, REMOVE_ITEM)
                | (VMODE, UN_VBOX)
                | (HMODE, UN_HBOX)
                | (MMODE, UN_HBOX)
                | (HMODE, ITAL_CORR)
                | (MMODE, ITAL_CORR)
                | (HMODE, DISCRETIONARY)
                | (MMODE, DISCRETIONARY)
                | (HMODE, ACCENT)
                | (VMODE, CAR_RET)
                | (HMODE, CAR_RET)
                | (MMODE, CAR_RET)
                | (VMODE, TAB_MARK)
                | (HMODE, TAB_MARK)
                | (MMODE, TAB_MARK) => {
                    main_control_box_list_cases_in(st)?;
                }
                // Alignment / endcsname cases
                (VMODE, NO_ALIGN)
                | (HMODE, NO_ALIGN)
                | (MMODE, NO_ALIGN)
                | (VMODE, OMIT)
                | (HMODE, OMIT)
                | (MMODE, OMIT)
                | (VMODE, HALIGN)
                | (HMODE, VALIGN)
                | (MMODE, HALIGN)
                | (VMODE, ENDV)
                | (HMODE, ENDV)
                | (VMODE, END_CS_NAME)
                | (HMODE, END_CS_NAME)
                | (MMODE, END_CS_NAME) => {
                    main_control_alignment_cases_in(st)?;
                }
                // Math entry/exit cases
                (HMODE, MATH_SHIFT)
                | (MMODE, EQ_NO)
                | (MMODE, LEFT_BRACE)
                | (MMODE, LETTER)
                | (MMODE, OTHER_CHAR)
                | (MMODE, CHAR_GIVEN)
                | (MMODE, CHAR_NUM)
                | (MMODE, MATH_CHAR_NUM)
                | (MMODE, MATH_GIVEN)
                | (MMODE, XETEX_MATH_GIVEN)
                | (MMODE, DELIM_NUM)
                | (MMODE, MATH_COMP)
                | (MMODE, LIMIT_SWITCH)
                | (MMODE, RADICAL)
                | (MMODE, ACCENT)
                | (MMODE, MATH_ACCENT)
                | (MMODE, VCENTER)
                | (MMODE, MATH_STYLE)
                | (MMODE, NON_SCRIPT)
                | (MMODE, MATH_CHOICE)
                | (MMODE, SUB_MARK)
                | (MMODE, SUP_MARK)
                | (MMODE, ABOVE)
                | (MMODE, LEFT_RIGHT)
                | (MMODE, MATH_SHIFT) => {
                    main_control_math_cases_in(st)?;
                }
                // Mode-independent cases (ANY_MODE): assignments, defs, etc.
                _ if matches!(
                    cmd,
                    TOKS_REGISTER
                        | ASSIGN_TOKS
                        | ASSIGN_INT
                        | ASSIGN_DIMEN
                        | ASSIGN_GLUE
                        | ASSIGN_MU_GLUE
                        | ASSIGN_FONT_DIMEN
                        | ASSIGN_FONT_INT
                        | SET_AUX
                        | SET_PREV_GRAF
                        | SET_PAGE_DIMEN
                        | SET_PAGE_INT
                        | SET_BOX_DIMEN
                        | SET_SHAPE
                        | DEF_CODE
                        | XETEX_DEF_CODE
                        | DEF_FAMILY
                        | SET_FONT
                        | DEF_FONT
                        | REGISTER
                        | ADVANCE
                        | MULTIPLY
                        | DIVIDE
                        | PREFIX
                        | LET
                        | SHORTHAND_DEF
                        | READ_TO_CS
                        | DEF
                        | SET_BOX
                        | HYPH_DATA
                        | SET_INTERACTION
                        | AFTER_ASSIGNMENT
                        | AFTER_GROUP
                        | IN_STREAM
                        | MESSAGE
                        | CASE_SHIFT
                        | XRAY
                        | EXTENSION
                ) =>
                {
                    main_control_mode_independent_in(st)?;
                }
                // Default: unrecognized / already handled → goto big_switch
                _ => {}
            }

            break 'reswitch; // → big_switch (get_x_token at top of loop)
        }
    }
}


// ---- moved verbatim from ini.rs (workstream 02) ----

// translated from xetex-ini.c:583 (prefixed_command)
#[inline]
fn pc_define(
    st: &mut crate::state::EngineState,
    global: bool,
    p: i32,
    t: u16,
    e: i32,
) -> Result<(), MainControlAbort> {
    let abort = if global {
        crate::save::geq_define(&mut st.eqtb, &mut st.mem, &mut st.scanner, p, t, e)
    } else {
        crate::save::eq_define(
            &mut st.eqtb,
            &mut st.save_stack,
            &mut st.mem,
            &mut st.scanner,
            p,
            t,
            e,
        )
    };
    if let Some(a) = abort {
        return Err(MainControlAbort::Save(a));
    }
    Ok(())
}

#[inline]
fn pc_word_define(
    st: &mut crate::state::EngineState,
    global: bool,
    p: i32,
    w: i32,
) -> Result<(), MainControlAbort> {
    if global {
        crate::save::geq_word_define(&mut st.eqtb, p, w);
    } else if let Some(a) = crate::save::eq_word_define(&mut st.eqtb, &mut st.save_stack, p, w) {
        return Err(MainControlAbort::Save(a));
    }
    Ok(())
}

#[inline]
fn pc_sa_def(
    st: &mut crate::state::EngineState,
    global: bool,
    p: i32,
    e: i32,
) -> Result<(), MainControlAbort> {
    let result = if global {
        crate::save::gsa_def(&mut st.mem, p, e)
    } else {
        crate::save::sa_def(&mut st.mem, &mut st.scanner, &mut st.save_stack, p, e)
    };
    crate::save::finish_sa_mutation_in(st, p, result)?;
    Ok(())
}

pub(crate) fn prefixed_command_in(
    st: &mut crate::state::EngineState,
) -> Result<(), MainControlAbort> {
    fn set_class(x: i32) -> i32 {
        (((x as u32) & 0x07) << 21) as i32
    }
    fn set_family(x: i32) -> i32 {
        (((x as u32) & 0xFF) << 24) as i32
    }

    // Command/eqtb constants used by `prefixed_command` (xetex-constants.h).
    const PREFIX: i32 = 95;
    const MAX_NON_PREFIXED_COMMAND: i32 = 71;
    const SPACER: i32 = 10;
    const RELAX_CMD: i32 = 0;
    const RIGHT_BRACE: i32 = 2;
    const LEFT_BRACE: i32 = 1;
    const TOKS_REGISTER: i32 = 72;
    const ASSIGN_TOKS: i32 = 73;
    const ASSIGN_INT: i32 = 74;
    const ASSIGN_DIMEN: i32 = 75;
    const ASSIGN_GLUE: i32 = 76;
    const ASSIGN_MU_GLUE: i32 = 77;
    const ASSIGN_FONT_DIMEN: i32 = 78;
    const ASSIGN_FONT_INT: i32 = 79;
    const SET_AUX: i32 = 80;
    const SET_PREV_GRAF: i32 = 81;
    const SET_PAGE_DIMEN: i32 = 82;
    const SET_PAGE_INT: i32 = 83;
    const SET_BOX_DIMEN: i32 = 84;
    const SET_SHAPE: i32 = 85;
    const DEF_CODE: i32 = 86;
    const XETEX_DEF_CODE: i32 = 87;
    const DEF_FAMILY: i32 = 88;
    const SET_FONT: i32 = 89;
    const DEF_FONT: i32 = 90;
    const REGISTER: i32 = 91;
    const ADVANCE: i32 = 92;
    const MULTIPLY: i32 = 93;
    const DIVIDE: i32 = 94;
    const LET: i32 = 96;
    const SHORTHAND_DEF: i32 = 97;
    const READ_TO_CS: i32 = 98;
    const DEF: i32 = 99;
    const SET_BOX: i32 = 100;
    const HYPH_DATA: i32 = 101;
    const SET_INTERACTION: i32 = 102;
    const UNDEFINED_CS: i32 = 103;
    const CALL: i32 = 113;
    const GLUE_REF: i32 = 119;
    const SHAPE_REF: i32 = 120;
    const DATA: i32 = 122;
    const CHAR_GIVEN: i32 = 68;
    const MATH_GIVEN: i32 = 69;
    const XETEX_MATH_GIVEN: i32 = 70;
    const NORMAL: i32 = 0;

    const CHAR_DEF_CODE: i32 = 0;
    const MATH_CHAR_DEF_CODE: i32 = 1;
    const COUNT_DEF_CODE: i32 = 2;
    const DIMEN_DEF_CODE: i32 = 3;
    const SKIP_DEF_CODE: i32 = 4;
    const MU_SKIP_DEF_CODE: i32 = 5;
    const TOKS_DEF_CODE: i32 = 6;
    const CHAR_SUB_DEF_CODE: i32 = 7;
    const XETEX_MATH_CHAR_NUM_DEF_CODE: i32 = 8;
    const XETEX_MATH_CHAR_DEF_CODE: i32 = 9;

    const GLUE_VAL: i32 = 2;
    const MU_VAL: i32 = 3;
    const TOK_VAL: i32 = 5;
    const INTER_CHAR_VAL: i32 = 6;

    const PROTECTED_TOKEN: i32 = 0x1C0_0000 + 1; // END_MATCH_TOKEN + 1
    const LEFT_BRACE_TOKEN: i32 = 0x20_0000;
    const RIGHT_BRACE_TOKEN: i32 = 0x40_0000;
    const OTHER_TOKEN: i32 = 0x180_0000;

    const COUNT_BASE: i32 = 7_826_812;
    const SCALED_BASE: i32 = 8_941_203;
    const SKIP_BASE: i32 = 2_254_359;
    const MU_SKIP_BASE: i32 = 2_254_615;
    const PC_LOCAL_BASE: i32 = 2_254_871;
    const LOCAL__PAR_SHAPE: i32 = 0;
    const LOCAL__OUTPUT_ROUTINE: i32 = 1;
    const LOCAL__XETEX_INTER_CHAR_TOKS: i32 = 11;
    const INT_PAR__TRACING_COMMANDS: i32 = 36;
    const INT_PAR__GLOBAL_DEFS: i32 = 43;
    const CHAR_CLASS_LIMIT: i32 = 4096;

    const TOO_BIG_USV: i32 = 1_114_112;
    const ACTIVE_MATH_CHAR: i32 = 0x1F_FFFF;
    const MAX_CHAR_CODE: i32 = 15;
    const BIGGEST_USV: i32 = 0x10_FFFF;
    const BOX_FLAG: i32 = 0x4000_0000;
    const GLOBAL_BOX_FLAG: i32 = 0x4000_8000;
    const AAT_FONT_FLAG: i32 = 0xFFFF;
    const OTGR_FONT_FLAG: i32 = 0xFFFE;
    const LP_CODE_BASE: i32 = 2;
    const RP_CODE_BASE: i32 = 3;
    const PC_LEFT_SIDE: i32 = 0;
    const PC_RIGHT_SIDE: i32 = 1;

    let mut a: i32 = 0;
    let f: i32;
    let mut j: i32;
    let k: i32;
    let mut p: i32 = 0;
    let mut q: i32;
    let mut n: i32;
    let mut e: bool;

    while st.scanner.cur_cmd as i32 == PREFIX {
        if (a / st.scanner.cur_chr) & 1 == 0 {
            a += st.scanner.cur_chr;
        }

        loop {
            crate::scan::get_x_token_in(st)?;
            if !(st.scanner.cur_cmd as i32 == SPACER || st.scanner.cur_cmd as i32 == RELAX_CMD) {
                break;
            }
        }

        if (st.scanner.cur_cmd as i32) <= MAX_NON_PREFIXED_COMMAND {
            crate::output::error_here_with_diagnostic_in(
                st,
                c"You can't use a prefix with `".as_ptr(),
            );
            crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
            crate::output::print_char_in(st, '\'' as i32);
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 1;
            st.errors.help_line[0] =
                c"I'll pretend you didn't say \\long or \\outer or \\global or \\protected."
                    .as_ptr();
            let outcome = crate::input::back_error_in(st)?;
            mc_propagate_error(outcome)?;
            return Ok(());
        }

        if st.eqtb.b32s1(INT_BASE + INT_PAR__TRACING_COMMANDS) > 2 {
            crate::output::show_cur_cmd_chr_in(st);
        }
    }

    if a >= 8 {
        j = PROTECTED_TOKEN;
        a -= 8;
    } else {
        j = 0;
    }

    if st.scanner.cur_cmd as i32 != DEF && (a % 4 != 0 || j != 0) {
        crate::output::error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
        crate::output::print_esc_cstr_in(st, c"long".as_ptr());
        crate::output::print_cstr_in(st, c"' or `".as_ptr());
        crate::output::print_esc_cstr_in(st, c"outer".as_ptr());
        crate::output::print_cstr_in(st, c"' or `".as_ptr());
        crate::output::print_esc_cstr_in(st, c"protected".as_ptr());
        crate::output::print_cstr_in(st, c"' with `".as_ptr());
        crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
        crate::output::print_char_in(st, '\'' as i32);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 1;
        st.errors.help_line[0] =
            c"I'll pretend you didn't say \\long or \\outer or \\protected here.".as_ptr();
        mc_propagate_error(crate::errors::error_in(st))?;
    }

    if st.eqtb.b32s1(INT_BASE + INT_PAR__GLOBAL_DEFS) != 0 {
        if st.eqtb.b32s1(INT_BASE + INT_PAR__GLOBAL_DEFS) < 0 {
            if a >= 4 {
                a -= 4;
            }
        } else if a < 4 {
            a += 4;
        }
    }

    'sw: {
        match st.scanner.cur_cmd as i32 {
            SET_FONT => {
                pc_define(st, a >= 4, CUR_FONT_LOC, DATA as u16, st.scanner.cur_chr)?;
            }

            DEF => {
                if st.scanner.cur_chr & 1 != 0
                    && a < 4
                    && st.eqtb.b32s1(INT_BASE + INT_PAR__GLOBAL_DEFS) >= 0
                {
                    a += 4;
                }

                e = st.scanner.cur_chr >= 2;
                crate::scan::get_r_token_in(st)?;
                p = st.scanner.cur_cs;
                q = crate::scan::scan_toks_in(st, true, e)?;

                if j != 0 {
                    q = crate::mem::get_avail(&mut st.mem);
                    if q == crate::mem::ALLOC_FAILED {
                        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                    }
                    st.mem.set_b32s0(q, j);
                    st.mem.set_b32s1(q, st.mem.b32s1(st.scanner.def_ref));
                    st.mem.set_b32s1(st.scanner.def_ref, q);
                }

                pc_define(st, a >= 4, p, (CALL + (a % 4)) as u16, st.scanner.def_ref)?;
            }

            LET => {
                n = st.scanner.cur_chr;
                crate::scan::get_r_token_in(st)?;
                p = st.scanner.cur_cs;

                if n == NORMAL {
                    loop {
                        crate::scan::get_token_in(st)?;
                        if st.scanner.cur_cmd as i32 != SPACER {
                            break;
                        }
                    }

                    if st.scanner.cur_tok == OTHER_TOKEN + '=' as i32 {
                        crate::scan::get_token_in(st)?;
                        if st.scanner.cur_cmd as i32 == SPACER {
                            crate::scan::get_token_in(st)?;
                        }
                    }
                } else {
                    crate::scan::get_token_in(st)?;
                    q = st.scanner.cur_tok;
                    crate::scan::get_token_in(st)?;
                    crate::input::back_input_in(st)?;
                    st.scanner.cur_tok = q;
                    crate::input::back_input_in(st)?;
                }

                if st.scanner.cur_cmd as i32 >= CALL {
                    st.mem.set_b32s0(st.scanner.cur_chr, st.mem.b32s0(st.scanner.cur_chr) + 1);
                } else if (st.scanner.cur_cmd as i32 == REGISTER || st.scanner.cur_cmd as i32 == TOKS_REGISTER)
                    && (st.scanner.cur_chr < 0 || st.scanner.cur_chr > 19)
                {
                    st.mem
                        .set_b32s0(st.scanner.cur_chr + 1, st.mem.b32s0(st.scanner.cur_chr + 1) + 1);
                }

                pc_define(st, a >= 4, p, st.scanner.cur_cmd as u16, st.scanner.cur_chr)?;
            }

            SHORTHAND_DEF => {
                if st.scanner.cur_chr == CHAR_SUB_DEF_CODE {
                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"vestigial MLTeX shorthand encountered??".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 1;
                    st.errors.help_line[0] =
                        c"This should never happen in Tectonic, where MLTeX has been excised."
                            .as_ptr();
                    mc_propagate_error(crate::errors::error_in(st))?;
                } else {
                    n = st.scanner.cur_chr;
                    crate::scan::get_r_token_in(st)?;
                    p = st.scanner.cur_cs;

                    pc_define(st, a >= 4, p, RELAX_CMD as u16, TOO_BIG_USV)?;

                    crate::scan::scan_optional_equals_in(st)?;

                    match n {
                        CHAR_DEF_CODE => {
                            crate::scan::scan_usv_num_in(st)?;
                            pc_define(st, a >= 4, p, CHAR_GIVEN as u16, st.scanner.cur_val)?;
                        }
                        MATH_CHAR_DEF_CODE => {
                            crate::scan::scan_fifteen_bit_int_in(st)?;
                            pc_define(st, a >= 4, p, MATH_GIVEN as u16, st.scanner.cur_val)?;
                        }
                        XETEX_MATH_CHAR_NUM_DEF_CODE => {
                            crate::scan::scan_xetex_math_char_int_in(st)?;
                            pc_define(st, a >= 4, p, XETEX_MATH_GIVEN as u16, st.scanner.cur_val)?;
                        }
                        XETEX_MATH_CHAR_DEF_CODE => {
                            crate::scan::scan_math_class_int_in(st)?;
                            n = set_class(st.scanner.cur_val);
                            crate::scan::scan_math_fam_int_in(st)?;
                            n = n.wrapping_add(set_family(st.scanner.cur_val));
                            crate::scan::scan_usv_num_in(st)?;
                            n = n.wrapping_add(st.scanner.cur_val);
                            pc_define(st, a >= 4, p, XETEX_MATH_GIVEN as u16, n)?;
                        }
                        _ => {
                            crate::scan::scan_register_num_in(st)?;

                            if st.scanner.cur_val > 255 {
                                j = n - 2;

                                if j > MU_VAL {
                                    j = TOK_VAL;
                                }

                                let __cv7 = st.scanner.cur_val;
                                let found = crate::save::find_sa_element(
                                    &mut st.mem,
                                    &mut st.scanner,
                                    j as u16,
                                    __cv7,
                                    true,
                                );
                                if found == crate::mem::ALLOC_FAILED {
                                    return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                                }
                                st.mem
                                    .set_b32s0(st.scanner.cur_ptr + 1, st.mem.b32s0(st.scanner.cur_ptr + 1) + 1);

                                if j == TOK_VAL {
                                    j = TOKS_REGISTER;
                                } else {
                                    j = REGISTER;
                                }

                                pc_define(st, a >= 4, p, j as u16, st.scanner.cur_ptr)?;
                            } else {
                                match n {
                                    COUNT_DEF_CODE => {
                                        pc_define(
                                            st,
                                            a >= 4,
                                            p,
                                            ASSIGN_INT as u16,
                                            COUNT_BASE + st.scanner.cur_val,
                                        )?;
                                    }
                                    DIMEN_DEF_CODE => {
                                        pc_define(
                                            st,
                                            a >= 4,
                                            p,
                                            ASSIGN_DIMEN as u16,
                                            SCALED_BASE + st.scanner.cur_val,
                                        )?;
                                    }
                                    SKIP_DEF_CODE => {
                                        pc_define(
                                            st,
                                            a >= 4,
                                            p,
                                            ASSIGN_GLUE as u16,
                                            SKIP_BASE + st.scanner.cur_val,
                                        )?;
                                    }
                                    MU_SKIP_DEF_CODE => {
                                        pc_define(
                                            st,
                                            a >= 4,
                                            p,
                                            ASSIGN_MU_GLUE as u16,
                                            MU_SKIP_BASE + st.scanner.cur_val,
                                        )?;
                                    }
                                    TOKS_DEF_CODE => {
                                        pc_define(
                                            st,
                                            a >= 4,
                                            p,
                                            ASSIGN_TOKS as u16,
                                            TOKS_BASE + st.scanner.cur_val,
                                        )?;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            READ_TO_CS => {
                j = st.scanner.cur_chr;
                crate::scan::scan_int_in(st)?;
                n = st.scanner.cur_val;
                if !crate::scan::scan_keyword_in(st, c"to".as_ptr())? {
                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"Missing `to' inserted".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 2;
                    st.errors.help_line[1] =
                        c"You should have said `\\read<number> to \\cs'.".as_ptr();
                    st.errors.help_line[0] = c"I'm going to look for the \\cs now.".as_ptr();
                    mc_propagate_error(crate::errors::error_in(st))?;
                }

                crate::scan::get_r_token_in(st)?;
                p = st.scanner.cur_cs;
                crate::scan::read_toks_in(st, n, p, j)?;

                pc_define(st, a >= 4, p, CALL as u16, st.scanner.cur_val)?;
            }

            TOKS_REGISTER | ASSIGN_TOKS => {
                q = st.scanner.cur_cs;
                e = false;

                if st.scanner.cur_cmd as i32 == TOKS_REGISTER {
                    if st.scanner.cur_chr == 0 {
                        crate::scan::scan_register_num_in(st)?;
                        if st.scanner.cur_val > 255 {
                            let __cv6 = st.scanner.cur_val;
                            let found = crate::save::find_sa_element(
                                &mut st.mem,
                                &mut st.scanner,
                                TOK_VAL as u16,
                                __cv6,
                                true,
                            );
                            if found == crate::mem::ALLOC_FAILED {
                                return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                            }
                            st.scanner.cur_chr = st.scanner.cur_ptr;
                            e = true;
                        } else {
                            st.scanner.cur_chr = TOKS_BASE + st.scanner.cur_val;
                        }
                    } else {
                        e = true;
                    }
                } else if st.scanner.cur_chr == PC_LOCAL_BASE + LOCAL__XETEX_INTER_CHAR_TOKS {
                    crate::scan::scan_char_class_not_ignored_in(st)?;
                    st.scanner.cur_ptr = st.scanner.cur_val;
                    crate::scan::scan_char_class_not_ignored_in(st)?;
                    let __cv5 = st.scanner.cur_val;
                    let __cp5 = st.scanner.cur_ptr;
                    let found = crate::save::find_sa_element(
                        &mut st.mem,
                        &mut st.scanner,
                        INTER_CHAR_VAL as u16,
                        __cp5 * CHAR_CLASS_LIMIT + __cv5,
                        true,
                    );
                    if found == crate::mem::ALLOC_FAILED {
                        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                    }
                    st.scanner.cur_chr = st.scanner.cur_ptr;
                    e = true;
                }

                p = st.scanner.cur_chr;
                crate::scan::scan_optional_equals_in(st)?;

                loop {
                    crate::scan::get_x_token_in(st)?;
                    if !(st.scanner.cur_cmd as i32 == SPACER || st.scanner.cur_cmd as i32 == RELAX_CMD) {
                        break;
                    }
                }

                if st.scanner.cur_cmd as i32 != LEFT_BRACE
                    && (st.scanner.cur_cmd as i32 == TOKS_REGISTER || st.scanner.cur_cmd as i32 == ASSIGN_TOKS)
                {
                    if st.scanner.cur_cmd as i32 == TOKS_REGISTER {
                        if st.scanner.cur_chr == 0 {
                            crate::scan::scan_register_num_in(st)?;
                            if st.scanner.cur_val < 256 {
                                q = st.eqtb.b32s1(TOKS_BASE + st.scanner.cur_val);
                            } else {
                                let __cv4 = st.scanner.cur_val;
                                crate::save::find_sa_element(
                                    &mut st.mem,
                                    &mut st.scanner,
                                    TOK_VAL as u16,
                                    __cv4,
                                    false,
                                );
                                if st.scanner.cur_ptr == TEX_NULL {
                                    q = TEX_NULL;
                                } else {
                                    q = st.mem.b32s1(st.scanner.cur_ptr + 1);
                                }
                            }
                        } else {
                            q = st.mem.b32s1(st.scanner.cur_chr + 1);
                        }
                    } else if st.scanner.cur_chr == PC_LOCAL_BASE + LOCAL__XETEX_INTER_CHAR_TOKS {
                        crate::scan::scan_char_class_not_ignored_in(st)?;
                        st.scanner.cur_ptr = st.scanner.cur_val;
                        crate::scan::scan_char_class_not_ignored_in(st)?;
                        let __cv3 = st.scanner.cur_val;
                        let __cp3 = st.scanner.cur_ptr;
                        crate::save::find_sa_element(
                            &mut st.mem,
                            &mut st.scanner,
                            INTER_CHAR_VAL as u16,
                            __cp3 * CHAR_CLASS_LIMIT + __cv3,
                            false,
                        );
                        if st.scanner.cur_ptr == TEX_NULL {
                            q = TEX_NULL;
                        } else {
                            q = st.mem.b32s1(st.scanner.cur_ptr + 1);
                        }
                    } else {
                        q = st.eqtb.b32s1(st.scanner.cur_chr);
                    }

                    if q == TEX_NULL {
                        if e {
                            pc_sa_def(st, a >= 4, p, TEX_NULL)?;
                        } else if a >= 4 {
                            pc_define(st, true, p, UNDEFINED_CS as u16, TEX_NULL)?;
                        } else {
                            pc_define(st, false, p, UNDEFINED_CS as u16, TEX_NULL)?;
                        }
                    } else {
                        st.mem.set_b32s0(q, st.mem.b32s0(q) + 1);
                        if e {
                            pc_sa_def(st, a >= 4, p, q)?;
                        } else if a >= 4 {
                            pc_define(st, true, p, CALL as u16, q)?;
                        } else {
                            pc_define(st, false, p, CALL as u16, q)?;
                        }
                    }

                    break 'sw;
                }

                crate::input::back_input_in(st)?;
                st.scanner.cur_cs = q;
                q = crate::scan::scan_toks_in(st, false, false)?;

                if st.mem.b32s1(st.scanner.def_ref) == TEX_NULL {
                    if e {
                        pc_sa_def(st, a >= 4, p, TEX_NULL)?;
                    } else if a >= 4 {
                        pc_define(st, true, p, UNDEFINED_CS as u16, TEX_NULL)?;
                    } else {
                        pc_define(st, false, p, UNDEFINED_CS as u16, TEX_NULL)?;
                    }

                    st.mem.set_b32s1(st.scanner.def_ref, st.mem.avail);
                    st.mem.avail = st.scanner.def_ref;
                } else {
                    if p == PC_LOCAL_BASE + LOCAL__OUTPUT_ROUTINE && !e {
                        let new_avail = crate::mem::get_avail(&mut st.mem);
                        if new_avail == crate::mem::ALLOC_FAILED {
                            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                        }
                        st.mem.set_b32s1(q, new_avail);
                        q = st.mem.b32s1(q);
                        st.mem.set_b32s0(q, RIGHT_BRACE_TOKEN + 125);
                        q = crate::mem::get_avail(&mut st.mem);
                        if q == crate::mem::ALLOC_FAILED {
                            return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                        }
                        st.mem.set_b32s0(q, LEFT_BRACE_TOKEN + 123);
                        st.mem.set_b32s1(q, st.mem.b32s1(st.scanner.def_ref));
                        st.mem.set_b32s1(st.scanner.def_ref, q);
                    }

                    if e {
                        pc_sa_def(st, a >= 4, p, st.scanner.def_ref)?;
                    } else if a >= 4 {
                        pc_define(st, true, p, CALL as u16, st.scanner.def_ref)?;
                    } else {
                        pc_define(st, false, p, CALL as u16, st.scanner.def_ref)?;
                    }
                }
            }

            ASSIGN_INT => {
                p = st.scanner.cur_chr;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_int_in(st)?;
                pc_word_define(st, a >= 4, p, st.scanner.cur_val)?;
            }

            ASSIGN_DIMEN => {
                p = st.scanner.cur_chr;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_dimen_in(st, false, false, false)?;
                pc_word_define(st, a >= 4, p, st.scanner.cur_val)?;
            }

            ASSIGN_GLUE | ASSIGN_MU_GLUE => {
                p = st.scanner.cur_chr;
                n = st.scanner.cur_cmd as i32;
                crate::scan::scan_optional_equals_in(st)?;
                if n == ASSIGN_MU_GLUE {
                    crate::scan::scan_glue_in(st, MU_VAL as i16)?;
                } else {
                    crate::scan::scan_glue_in(st, GLUE_VAL as i16)?;
                }
                crate::main_control::trap_zero_glue(st);
                pc_define(st, a >= 4, p, GLUE_REF as u16, st.scanner.cur_val)?;
            }

            XETEX_DEF_CODE => {
                if st.scanner.cur_chr == SF_CODE_BASE {
                    p = st.scanner.cur_chr;
                    crate::scan::scan_usv_num_in(st)?;
                    p += st.scanner.cur_val;
                    n = st.eqtb.b32s1(SF_CODE_BASE + st.scanner.cur_val) % 65536;
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_char_class_in(st)?;
                    let v = st.scanner.cur_val.wrapping_mul(65536).wrapping_add(n);
                    pc_define(st, a >= 4, p, DATA as u16, v)?;
                } else if st.scanner.cur_chr == MATH_CODE_BASE {
                    p = st.scanner.cur_chr;
                    crate::scan::scan_usv_num_in(st)?;
                    p += st.scanner.cur_val;
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_xetex_math_char_int_in(st)?;
                    pc_define(st, a >= 4, p, DATA as u16, st.scanner.cur_val)?;
                } else if st.scanner.cur_chr == MATH_CODE_BASE + 1 {
                    p = st.scanner.cur_chr - 1;
                    crate::scan::scan_usv_num_in(st)?;
                    p += st.scanner.cur_val;
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_math_class_int_in(st)?;
                    n = set_class(st.scanner.cur_val);
                    crate::scan::scan_math_fam_int_in(st)?;
                    n = n.wrapping_add(set_family(st.scanner.cur_val));
                    crate::scan::scan_usv_num_in(st)?;
                    n = n.wrapping_add(st.scanner.cur_val);
                    pc_define(st, a >= 4, p, DATA as u16, n)?;
                } else if st.scanner.cur_chr == DEL_CODE_BASE {
                    p = st.scanner.cur_chr;
                    crate::scan::scan_usv_num_in(st)?;
                    p += st.scanner.cur_val;
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_int_in(st)?;
                    pc_word_define(st, a >= 4, p, st.scanner.cur_val)?;
                } else {
                    p = st.scanner.cur_chr - 1;
                    crate::scan::scan_usv_num_in(st)?;
                    p += st.scanner.cur_val;
                    crate::scan::scan_optional_equals_in(st)?;
                    n = 0x4000_0000; // "extended delimiter code flag"
                    crate::scan::scan_math_fam_int_in(st)?;
                    n = n.wrapping_add(st.scanner.cur_val.wrapping_mul(0x20_0000)); // extended delimiter family
                    crate::scan::scan_usv_num_in(st)?;
                    n = n.wrapping_add(st.scanner.cur_val);
                    pc_word_define(st, a >= 4, p, n)?;
                }
            }

            DEF_CODE => {
                if st.scanner.cur_chr == CAT_CODE_BASE {
                    n = MAX_CHAR_CODE;
                } else if st.scanner.cur_chr == MATH_CODE_BASE {
                    n = 0x8000;
                } else if st.scanner.cur_chr == SF_CODE_BASE {
                    n = 0x7FFF;
                } else if st.scanner.cur_chr == DEL_CODE_BASE {
                    n = 0xFFFFFF;
                } else {
                    n = BIGGEST_USV;
                }

                p = st.scanner.cur_chr;
                crate::scan::scan_usv_num_in(st)?;
                p += st.scanner.cur_val;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_int_in(st)?;

                if (st.scanner.cur_val < 0 && p < DEL_CODE_BASE) || st.scanner.cur_val > n {
                    crate::output::error_here_with_diagnostic_in(st, c"Invalid code (".as_ptr());
                    crate::output::print_int_in(st, st.scanner.cur_val);
                    if p < DEL_CODE_BASE {
                        crate::output::print_cstr_in(st, c"), should be in the range 0..".as_ptr());
                    } else {
                        crate::output::print_cstr_in(st, c"), should be at most ".as_ptr());
                    }
                    crate::output::print_int_in(st, n);
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 1;
                    st.errors.help_line[0] =
                        c"I'm going to use 0 instead of that illegal code value.".as_ptr();
                    mc_propagate_error(crate::errors::error_in(st))?;
                    st.scanner.cur_val = 0;
                }

                if p < MATH_CODE_BASE {
                    if p >= SF_CODE_BASE {
                        n = st.eqtb.b32s1(p) / 65536;
                        let v = n.wrapping_mul(65536).wrapping_add(st.scanner.cur_val);
                        pc_define(st, a >= 4, p, DATA as u16, v)?;
                    } else {
                        pc_define(st, a >= 4, p, DATA as u16, st.scanner.cur_val)?;
                    }
                } else if p < DEL_CODE_BASE {
                    if st.scanner.cur_val == 32768 {
                        st.scanner.cur_val = ACTIVE_MATH_CHAR;
                    } else {
                        let v = set_class(st.scanner.cur_val / 4096)
                            .wrapping_add(set_family((st.scanner.cur_val % 4096) / 256))
                            .wrapping_add(st.scanner.cur_val % 256);
                        st.scanner.cur_val = v;
                    }

                    pc_define(st, a >= 4, p, DATA as u16, st.scanner.cur_val)?;
                } else {
                    pc_word_define(st, a >= 4, p, st.scanner.cur_val)?;
                }
            }

            DEF_FAMILY => {
                p = st.scanner.cur_chr;
                crate::scan::scan_math_fam_int_in(st)?;
                p += st.scanner.cur_val;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_font_ident_in(st)?;
                pc_define(st, a >= 4, p, DATA as u16, st.scanner.cur_val)?;
            }

            REGISTER | ADVANCE | MULTIPLY | DIVIDE => {
                crate::main_control::do_register_command_in(st, a as i16)?;
            }

            SET_BOX => {
                crate::scan::scan_register_num_in(st)?;
                if a >= 4 {
                    n = GLOBAL_BOX_FLAG + st.scanner.cur_val;
                } else {
                    n = BOX_FLAG + st.scanner.cur_val;
                }

                crate::scan::scan_optional_equals_in(st)?;

                if st.scanner.set_box_allowed {
                    crate::main_control::scan_box_in(st, n)?;
                } else {
                    crate::output::error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
                    crate::output::print_esc_cstr_in(st, c"setbox".as_ptr());
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 2;
                    st.errors.help_line[1] =
                        c"Sorry, \\setbox is not allowed after \\halign in a display,".as_ptr();
                    st.errors.help_line[0] =
                        c"or between \\accent and an accented character.".as_ptr();
                    mc_propagate_error(crate::errors::error_in(st))?;
                }
            }

            SET_AUX => {
                crate::main_control::alter_aux_in(st)?;
            }

            SET_PREV_GRAF => {
                crate::main_control::alter_prev_graf_in(st)?;
            }

            SET_PAGE_DIMEN => {
                crate::main_control::alter_page_so_far_in(st)?;
            }

            SET_PAGE_INT => {
                crate::main_control::alter_integer_in(st)?;
            }

            SET_BOX_DIMEN => {
                crate::main_control::alter_box_dimen_in(st)?;
            }

            SET_SHAPE => {
                q = st.scanner.cur_chr;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_int_in(st)?;
                n = st.scanner.cur_val;

                if n <= 0 {
                    p = TEX_NULL;
                } else if q > PC_LOCAL_BASE + LOCAL__PAR_SHAPE {
                    n = (st.scanner.cur_val / 2) + 1;
                    p = crate::mem::get_node(&mut st.mem, 2 * n + 1);
                    if p == crate::mem::ALLOC_FAILED {
                        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                    }
                    st.mem.set_b32s0(p, n);
                    n = st.scanner.cur_val;
                    st.mem.set_b32s1(p + 1, n);

                    j = p + 2;
                    while j <= p + n + 1 {
                        crate::scan::scan_int_in(st)?;
                        st.mem.set_b32s1(j, st.scanner.cur_val);
                        j += 1;
                    }

                    if n & 1 == 0 {
                        st.mem.set_b32s1(p + n + 2, 0);
                    }
                } else {
                    p = crate::mem::get_node(&mut st.mem, 2 * n + 1);
                    if p == crate::mem::ALLOC_FAILED {
                        return Err(crate::scan::NodeRecycleAbort::Overflow.into());
                    }
                    st.mem.set_b32s0(p, n);

                    j = 1;
                    while j <= n {
                        crate::scan::scan_dimen_in(st, false, false, false)?;
                        st.mem.set_b32s1(p + 2 * j - 1, st.scanner.cur_val);
                        crate::scan::scan_dimen_in(st, false, false, false)?;
                        st.mem.set_b32s1(p + 2 * j, st.scanner.cur_val);
                        j += 1;
                    }
                }

                pc_define(st, a >= 4, q, SHAPE_REF as u16, p)?;
            }

            HYPH_DATA => {
                if st.scanner.cur_chr == 1 {
                    if st.config.in_initex_mode {
                        new_patterns_in(st)?;
                        break 'sw;
                    }

                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"Patterns can be loaded only by INITEX".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 0;
                    mc_propagate_error(crate::errors::error_in(st))?;

                    loop {
                        crate::scan::get_token_in(st)?;
                        if st.scanner.cur_cmd as i32 == RIGHT_BRACE {
                            break;
                        }
                    }

                    return Ok(());
                } else {
                    new_hyph_exceptions_in(st)?;
                    break 'sw;
                }
            }

            ASSIGN_FONT_DIMEN => {
                crate::scan::find_font_dimen_in(st, true)?;
                k = st.scanner.cur_val;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_dimen_in(st, false, false, false)?;
                st.fonts.set_font_info_b32s1(k, st.scanner.cur_val);
            }

            ASSIGN_FONT_INT => {
                n = st.scanner.cur_chr;
                crate::scan::scan_font_ident_in(st)?;
                f = st.scanner.cur_val;

                if n < 2 {
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_int_in(st)?;
                    if n == 0 {
                        st.fonts.set_hyphen_char_entry(f, st.scanner.cur_val);
                    } else {
                        st.fonts.set_skew_char_entry(f, st.scanner.cur_val);
                    }
                } else {
                    if st.fonts.font_area_entry(f) == AAT_FONT_FLAG
                        || st.fonts.font_area_entry(f) == OTGR_FONT_FLAG
                    {
                        crate::scan::scan_glyph_number_in(st, f)?;
                    } else {
                        crate::scan::scan_char_num_in(st)?;
                    }

                    p = st.scanner.cur_val;
                    crate::scan::scan_optional_equals_in(st)?;
                    crate::scan::scan_int_in(st)?;
                    match n {
                        LP_CODE_BASE => unsafe {
                            set_cp_code(f, p as u32, PC_LEFT_SIDE, st.scanner.cur_val)
                        },
                        RP_CODE_BASE => unsafe {
                            set_cp_code(f, p as u32, PC_RIGHT_SIDE, st.scanner.cur_val)
                        },
                        _ => {}
                    }
                }
            }

            DEF_FONT => {
                unsafe { crate::main_control::new_font_in(st, a as i16)? };
            }

            SET_INTERACTION => {
                crate::main_control::new_interaction_in(st);
            }

            _ => {
                return Err(MainControlAbort::Confusion(c"prefix"));
            }
        }
    }

    // done:
    if st.scanner.after_token != 0 {
        st.scanner.cur_tok = st.scanner.after_token;
        crate::input::back_input_in(st)?;
        st.scanner.after_token = 0;
    }
    Ok(())
}
