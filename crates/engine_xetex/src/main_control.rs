//! The main control loop, its command verbs, and the `\halign`/`\valign`
//! alignment family (= xetex0.c's main_control group).

use crate::consts::{
    ACC_KERN, ACTIVE_MATH_CHAR, A_LEADERS, BEGIN_M_CODE, BIGGEST_CHAR, BIGGEST_LANG, BIGGEST_USV,
    BOX_BASE, BOX_NODE_SIZE, CAT_CODE_BASE, CHAR_CLASS_LIMIT, COND_MATH_GLUE, COUNT_BASE,
    CS_TOKEN_FLAG, CUR_FONT_LOC, DEL_CODE_BASE, DIMEN_BASE, DIMEN_PAR_BOX_MAX_DEPTH,
    DIMEN_PAR_DISPLAY_INDENT, DIMEN_PAR_HANG_INDENT, DIMEN_PAR_HSIZE, DIMEN_PAR_OVERFULL_RULE,
    DIMEN_PAR_PAR_INDENT, DIMEN_PAR_SPLIT_MAX_DEPTH, DLIST, EMPTY, END_M_CODE, ETEX_PEN_BASE,
    EXPLICIT, FONT_BASE, FONT_ID_BASE, FROZEN_CR, FROZEN_END_GROUP, FROZEN_END_TEMPLATE,
    FROZEN_RIGHT, GLUE_BASE, GLUE_NODE, GLUE_PAR_ABOVE_DISPLAY_SKIP, GLUE_PAR_BELOW_DISPLAY_SKIP,
    GLUE_PAR_PAR_SKIP, GLUE_PAR_SPACE_SKIP, GLUE_PAR_SPLIT_TOP_SKIP, GLUE_PAR_TAB_SKIP,
    GLUE_PAR_XSPACE_SKIP, HASH_BASE, HMODE, IGNORE_DEPTH, INT_BASE, INT_PAR_FLOATING_PENALTY,
    INT_PAR_GLOBAL_DEFS, INT_PAR_HANG_AFTER, INT_PAR_LANGUAGE, INT_PAR_LEFT_HYPHEN_MIN,
    INT_PAR_LOOSENESS, INT_PAR_POST_DISPLAY_PENALTY, INT_PAR_PRE_DISPLAY_PENALTY,
    INT_PAR_RIGHT_HYPHEN_MIN, INT_PAR_TEXXET, INT_PAR_TRACING_COMMANDS, INT_PAR_TRACING_LOST_CHARS,
    INT_PAR_XETEX_DASH_BREAK, INT_PAR_XETEX_INTERWORD_SPACE_SHAPING,
    INT_PAR_XETEX_INTER_CHAR_TOKENS, INT_PAR_XETEX_UPWARDS, INT_PAR_XETEX_USE_GLYPH_METRICS,
    KERN_NODE, LEFT_SIDE, LOCAL_BASE, LOCAL_EVERY_CR, LOCAL_EVERY_HBOX, LOCAL_EVERY_JOB,
    LOCAL_EVERY_PAR, LOCAL_EVERY_VBOX, LOCAL_OUTPUT_ROUTINE, LOCAL_PAR_SHAPE,
    LOCAL_XETEX_INTER_CHAR_TOKS, MARGIN_KERN_NODE_SIZE, MATH_CODE_BASE, MAX_HALFWORD, MMODE,
    MU_GLUE, MU_SKIP_BASE, NATIVE_NODE_SIZE, NOAD_SIZE, NON_ADDRESS, NORMAL, NULL_CS, NULL_FLAG,
    PRIM_EQTB_BASE, RIGHT_SIDE, SCALED_BASE, SELECTOR_NEW_STRING, SELECTOR_NO_PRINT,
    SELECTOR_TERM_ONLY, SF_CODE_BASE, SINGLE_BASE, SKIP_BASE, SMALL_NODE_SIZE, SPACE_ADJUSTMENT,
    SUB_BOX, SUB_MLIST, TEX_INFINITY, TEX_NULL, TOKS_BASE, TOO_BIG_CHAR, TOO_BIG_USV,
    UNDEFINED_PRIMITIVE, VMODE,
};
use crate::ext::{append_src_special_in, apply_mapping, do_extension_in};
use crate::fonts::{
    read_font_info_in, AAT_FONT_FLAG, EXTRA_SPACE_CODE, LIG_TAG, OTGR_FONT_FLAG, SLANT_CODE,
    SPACE_CODE, X_HEIGHT_CODE,
};
use crate::hyphenation::{new_hyph_exceptions_in, new_patterns_in};
use crate::math::{after_math_in, init_math_in};
use crate::mem::{
    is_char_node, ADJUST_HEAD, ALIGN_HEAD, CONTRIB_HEAD, END_SPAN, FILL_GLUE, FIL_GLUE,
    FIL_NEG_GLUE, HOLD_HEAD, PAGE_HEAD, PRE_ADJUST_HEAD, SS_GLUE, TEMP_HEAD, ZERO_GLUE,
};
use crate::node::{
    AdjustNode, AnyBox, AnyNode, GlueSpecNode, InsNode, MarkNode, NodePtr, RuleDimen, SaNode,
    UnsetNode, WhatsitNode, WhatsitSubtype,
};
use crate::scan::{
    set_class_fn, set_family_fn, ALIGNING, BOX_VAL, DIMEN_VAL, GLUE_VAL, INTER_CHAR_VAL, INT_VAL,
    LEFT_BRACE_TOKEN, MATH_SHIFT_TOKEN, MAX_CHAR_VAL, MU_VAL, NORMAL_SCANNER, OTHER_TOKEN,
    PROTECTED_TOKEN, RIGHT_BRACE_TOKEN, TAB_TOKEN, TOK_VAL,
};
use crate::state::{EngineState, GroupCode, MemState};
use crate::units::{GlueSign, MemIdx, MemPtr, NodeType, Scaled};

use crate::bridge::diag_append_str;
use crate::cmd::{
    ABOVE, ACCENT, ADVANCE, AFTER_ASSIGNMENT, AFTER_GROUP, ASSIGN_DIMEN, ASSIGN_FONT_DIMEN,
    ASSIGN_FONT_INT, ASSIGN_GLUE, ASSIGN_INT, ASSIGN_MU_GLUE, ASSIGN_TOKS, BEGIN_GROUP, BOX_REF,
    BREAK_PENALTY, CALL, CAR_RET, CASE_SHIFT, CHAR_GIVEN, CHAR_NUM, DATA, DEF, DEF_CODE,
    DEF_FAMILY, DEF_FONT, DELIM_NUM, DISCRETIONARY, DIVIDE, ENDV, END_CS_NAME, END_GROUP, EQ_NO,
    EXTENSION, EX_SPACE, GLUE_REF, HALIGN, HMOVE, HRULE, HSKIP, HYPH_DATA, IGNORE_SPACES, INSERT,
    IN_STREAM, ITAL_CORR, KERN, LAST_ITEM, LEADER_SHIP, LEFT_BRACE, LEFT_RIGHT, LET, LETTER,
    LIMIT_SWITCH, MAC_PARAM, MAKE_BOX, MARK, MATH_ACCENT, MATH_CHAR_NUM, MATH_CHOICE, MATH_COMP,
    MATH_GIVEN, MATH_SHIFT, MATH_STYLE, MAX_CHAR_CODE, MAX_COMMAND, MAX_NON_PREFIXED_COMMAND,
    MESSAGE, MKERN, MSKIP, MULTIPLY, NON_SCRIPT, NO_ALIGN, NO_BOUNDARY, OMIT, OTHER_CHAR, PAR_END,
    PREFIX, RADICAL, READ_TO_CS, REGISTER, RELAX, REMOVE_ITEM, RIGHT_BRACE, SET_AUX, SET_BOX,
    SET_BOX_DIMEN, SET_FONT, SET_INTERACTION, SET_PAGE_DIMEN, SET_PAGE_INT, SET_PREV_GRAF,
    SET_SHAPE, SHAPE_REF, SHORTHAND_DEF, SPACER, START_PAR, STOP, SUB_MARK, SUP_MARK, TAB_MARK,
    TOKS_REGISTER, UNDEFINED_CS, UN_HBOX, UN_VBOX, VADJUST, VALIGN, VCENTER, VMOVE, VRULE, VSKIP,
    XETEX_DEF_CODE, XETEX_MATH_GIVEN, XRAY,
};
use crate::input::{
    BACKED_UP, BACKED_UP_CHAR, EVERY_CR_TEXT, EVERY_HBOX_TEXT, EVERY_JOB_TEXT, EVERY_PAR_TEXT,
    EVERY_VBOX_TEXT, INSERTED, INTER_CHAR_TEXT, OUTPUT_TEXT, TOKEN_LIST, U_TEMPLATE, V_TEMPLATE,
};
use crate::packaging::ADDITIONAL;

extern "C" {
    fn set_cp_code(font_num: i32, code: u32, side: i32, value: i32);
}

// Command codes (`cur_cmd`).

// Equivalent types beyond `MAX_COMMAND`.

// Value levels (`cur_val_level`).

const INNER_VMODE: i16 = -VMODE;
const INNER_HMODE: i16 = -HMODE;

const ALIGN_STACK_NODE_SIZE: i32 = 6;
const SPAN_NODE_SIZE: i32 = 2;

const TAB_SKIP_SUBTYPE: u16 = GLUE_PAR_TAB_SKIP as u16 + 1;

// Token-list types.

const GLUE_BASE_TAB_SKIP: i32 = GLUE_BASE + GLUE_PAR_TAB_SKIP as i32;

// `cur_chr` modifiers.
pub(crate) const BOX_CODE: i32 = 0;
pub(crate) const COPY_CODE: i32 = 1;
pub(crate) const LAST_BOX_CODE: i32 = 2;
pub(crate) const VSPLIT_CODE: i32 = 3;
pub(crate) const VTOP_CODE: i32 = 4;
pub(crate) const SKIP_CODE: i32 = 4;
pub(crate) const MSKIP_CODE: i32 = 5;
pub(crate) const CHAR_DEF_CODE: i32 = 0;
pub(crate) const MATH_CHAR_DEF_CODE: i32 = 1;
pub(crate) const COUNT_DEF_CODE: i32 = 2;
pub(crate) const DIMEN_DEF_CODE: i32 = 3;
pub(crate) const SKIP_DEF_CODE: i32 = 4;
pub(crate) const MU_SKIP_DEF_CODE: i32 = 5;
pub(crate) const TOKS_DEF_CODE: i32 = 6;
pub(crate) const CHAR_SUB_DEF_CODE: i32 = 7;
pub(crate) const XETEX_MATH_CHAR_NUM_DEF_CODE: i32 = 8;
pub(crate) const XETEX_MATH_CHAR_DEF_CODE: i32 = 9;
pub(crate) const LP_CODE_BASE: i32 = 2;
pub(crate) const RP_CODE_BASE: i32 = 3;
pub(crate) const SPAN_CODE: i32 = 1_114_113;
pub(crate) const CR_CODE: i32 = 1_114_114;
pub(crate) const CR_CR_CODE: i32 = 1_114_115;

pub(crate) const BOX_FLAG: i32 = 0x4000_0000;
pub(crate) const GLOBAL_BOX_FLAG: i32 = 0x4000_8000;
pub(crate) const SHIP_OUT_FLAG: i32 = 0x4001_0000;
pub(crate) const LEADER_FLAG: i32 = 0x4001_0001;

pub(crate) fn align_error_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.align_state.abs() > 2 {
        crate::output::error_here_with_diagnostic_in(st, c"Misplaced ".as_ptr());
        crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd, st.scanner.cur_chr);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        if st.scanner.cur_tok == TAB_TOKEN + 38 {
            help(
                &mut st.errors,
                &[
                    c"I can't figure out why you would want to use a tab mark",
                    c"here. If you just want an ampersand, the remedy is",
                    c"simple: Just type `I\\&' now. But if some right brace",
                    c"up above has ended a previous alignment prematurely,",
                    c"you're probably due for more error messages, and you",
                    c"might try typing `S' now just to see what is salvageable.",
                ],
            );
        } else {
            help(
                &mut st.errors,
                &[
                    c"I can't figure out why you would want to use a tab mark",
                    c"or \\cr or \\span just now. If something like a right brace",
                    c"up above has ended a previous alignment prematurely,",
                    c"you're probably due for more error messages, and you",
                    c"might try typing `S' now just to see what is salvageable.",
                ],
            );
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
        help(
            &mut st.errors,
            &[
                c"I've put in what seems to be necessary to fix",
                c"the current column of the current alignment.",
                c"Try to go on, since this might almost work.",
            ],
        );
        mc_propagate_error(crate::input::ins_error_in(st)?)?;
    }
    Ok(())
}
pub(crate) fn no_align_error_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::output::error_here_with_diagnostic_in(st, c"Misplaced ".as_ptr());
    crate::output::print_esc_cstr_in(st, c"noalign".as_ptr());
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    help(
        &mut st.errors,
        &[
            c"I expect to see \\noalign only after the \\cr of",
            c"an alignment. Proceed, and I'll ignore this case.",
        ],
    );
    mc_propagate_error(crate::errors::error_in(st))
}
pub(crate) fn omit_error_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::output::error_here_with_diagnostic_in(st, c"Misplaced ".as_ptr());
    crate::output::print_esc_cstr_in(st, c"omit".as_ptr());
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    help(
        &mut st.errors,
        &[
            c"I expect to see \\omit only after tab marks or the \\cr of",
            c"an alignment. Proceed, and I'll ignore this case.",
        ],
    );
    mc_propagate_error(crate::errors::error_in(st))
}
pub(crate) fn fin_align_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    use crate::mem::{
        alloc_glue, alloc_null_box, alloc_param_glue, alloc_penalty, delete_glue_ref, flush_list,
        free_node, pop_nest_in,
    };
    use crate::packaging::flush_node_list;
    use crate::scan::NodeRecycleAbort;

    if st.save_stack.cur_group != GroupCode::Align {
        return Err(MainControlAbort::Confusion(c"align1"));
    }
    crate::save::unsave_in(st)?;
    if st.save_stack.cur_group != GroupCode::Align {
        return Err(MainControlAbort::Confusion(c"align0"));
    }
    crate::save::unsave_in(st)?;

    let in_display = st.input.nest[(st.input.nest_ptr - 1) as usize].mode == MMODE;
    let o = if in_display {
        st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_DISPLAY_INDENT)
    } else {
        0
    };

    let mut q = st.mem.b32s1(st.mem.b32s1(ALIGN_HEAD));
    loop {
        let u_part = st.mem.b32s1(q + 3);
        flush_list(&mut st.mem, MemPtr::from_raw(u_part));
        let v_part = st.mem.b32s1(q + 2);
        flush_list(&mut st.mem, MemPtr::from_raw(v_part));
        let p = st.mem.b32s1(st.mem.b32s1(q));
        if st.mem.b32s1(q + 1) == NULL_FLAG {
            st.mem.set_b32s1(q + 1, 0);
            let r = st.mem.b32s1(q);
            let s = st.mem.b32s0(r + 1);
            if s != 0 {
                crate::mem::add_glue_ref(&mut st.mem, ZERO_GLUE);
                delete_glue_ref(&mut st.mem, s);
                st.mem.set_b32s0(r + 1, 0);
            }
        }
        if st.mem.b32s0(q) != END_SPAN.0 {
            let t = st.mem.b32s1(q + 1) + st.mem.b32s1(st.mem.b32s0(st.mem.b32s1(q) + 1) + 1);
            let mut r = st.mem.b32s0(q);
            let mut s = END_SPAN.0;
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
                    free_node(&mut st.mem, MemPtr::from_raw(r), SPAN_NODE_SIZE);
                }
                r = u;
                if r == END_SPAN.0 {
                    break;
                }
            }
        }
        st.mem.set_b16s1(q, NodeType::Unset.as_raw());
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

    st.save_stack.save_ptr -= 2;
    st.scanner.pack_begin_line = -st.input.cur_list.mode_line;
    let spec_size = st.save_stack.b32s1(st.save_stack.save_ptr + 1);
    let spec_mode = st.save_stack.b32s1(st.save_stack.save_ptr) as i16;
    let head = st.mem.link_ptr(ALIGN_HEAD);
    let p = if st.input.cur_list.mode == INNER_VMODE {
        let rule_save = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE);
        st.eqtb.set_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE, 0);
        let packed = crate::packaging::hpack_emit_in(st, head, spec_size, spec_mode)?;
        st.eqtb
            .set_b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE, rule_save);
        packed
    } else {
        move_preamble_dimen(&mut st.mem, 1, 3);
        let packed =
            crate::packaging::vpackage_emit_in(st, head, spec_size, spec_mode, MAX_HALFWORD)?;
        move_preamble_dimen(&mut st.mem, 3, 1);
        packed
    };
    st.scanner.pack_begin_line = 0;

    let mut q = st.mem.b32s1(st.input.cur_list.head);
    let mut s = st.input.cur_list.head.0;
    while q != TEX_NULL {
        if !is_char_node(&st.mem, q) {
            match st.mem.classify(MemIdx(q)) {
                Ok(AnyNode::Unset(row)) => {
                    let row = if st.input.cur_list.mode == INNER_VMODE {
                        let b = row.into_hlist(&mut st.mem);
                        let width = p.width(&st.mem);
                        b.set_width(&mut st.mem, width);
                        if in_display {
                            b.set_subtype(&mut st.mem, DLIST);
                        }
                        AnyBox::HList(b)
                    } else {
                        let b = row.into_vlist(&mut st.mem);
                        let height = p.height(&st.mem);
                        b.set_height(&mut st.mem, height);
                        AnyBox::VList(b)
                    };
                    let (order, sign, set) = (
                        p.glue_order(&st.mem),
                        p.glue_sign(&st.mem),
                        p.glue_set(&st.mem),
                    );
                    row.set_glue_order(&mut st.mem, order);
                    row.set_glue_sign(&mut st.mem, sign);
                    row.set_glue_set(&mut st.mem, set);
                    row.set_shift_amount(&mut st.mem, Scaled(o));

                    let mut r = row.list_ptr(&st.mem).link(&st.mem).0;
                    let mut ss = p.list_ptr(&st.mem).link(&st.mem).0;
                    loop {
                        let Ok(AnyNode::Unset(col)) = st.mem.classify(MemIdx(r)) else {
                            return Err(MainControlAbort::Confusion(c"align"));
                        };
                        let n = col.subtype(&st.mem);
                        let w = st.mem.b32s1(ss + 1);
                        let mut t = w;
                        let mut u = HOLD_HEAD;
                        col.set_subtype(&mut st.mem, 0);
                        for _ in 0..n {
                            ss = st.mem.b32s1(ss);
                            let v = st.mem.b32s0(ss + 1);
                            let spec = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(v))
                                .expect("non-null glue spec");
                            let g = alloc_glue(&mut st.mem, v).ok_or(NodeRecycleAbort::Overflow)?;
                            u.set_link(&mut st.mem, g.ptr());
                            u = g.ptr();
                            g.set_subtype(&mut st.mem, TAB_SKIP_SUBTYPE);
                            t += spec.width(&st.mem).0;
                            match p.glue_sign(&st.mem) {
                                GlueSign::Stretching
                                    if spec.stretch_order(&st.mem) == p.glue_order(&st.mem) =>
                                {
                                    t += crate::scaled_math::tex_round(
                                        p.glue_set(&st.mem) * spec.stretch(&st.mem).0 as f64,
                                    );
                                }
                                GlueSign::Shrinking
                                    if spec.shrink_order(&st.mem) == p.glue_order(&st.mem) =>
                                {
                                    t -= crate::scaled_math::tex_round(
                                        p.glue_set(&st.mem) * spec.shrink(&st.mem).0 as f64,
                                    );
                                }
                                _ => {}
                            }
                            ss = st.mem.b32s1(ss);
                            let nb =
                                alloc_null_box(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
                            u.set_link(&mut st.mem, nb.ptr());
                            u = nb.ptr();
                            let span_w = Scaled(st.mem.b32s1(ss + 1));
                            t += span_w.0;
                            if st.input.cur_list.mode == INNER_VMODE {
                                nb.set_width(&mut st.mem, span_w);
                            } else {
                                nb.into_vlist(&mut st.mem).set_height(&mut st.mem, span_w);
                            }
                        }
                        let col = if st.input.cur_list.mode == INNER_VMODE {
                            let (h, d) = (row.height(&st.mem), row.depth(&st.mem));
                            col.set_height(&mut st.mem, h);
                            col.set_depth(&mut st.mem, d);
                            let glue = unset_glue(&st.mem, col, t, col.width(&st.mem).0);
                            let b = col.into_hlist(&mut st.mem);
                            b.set_width(&mut st.mem, Scaled(w));
                            (AnyBox::HList(b), glue)
                        } else {
                            let width = row.width(&st.mem);
                            col.set_width(&mut st.mem, width);
                            let glue = unset_glue(&st.mem, col, t, col.height(&st.mem).0);
                            let b = col.into_vlist(&mut st.mem);
                            b.set_height(&mut st.mem, Scaled(w));
                            (AnyBox::VList(b), glue)
                        };
                        let (b, (sign, order, set)) = col;
                        b.set_glue_sign(&mut st.mem, sign);
                        b.set_glue_order(&mut st.mem, order);
                        b.set_glue_set(&mut st.mem, set);
                        b.set_shift_amount(&mut st.mem, Scaled(0));
                        if u != HOLD_HEAD {
                            st.mem.set_b32s1(u, st.mem.b32s1(r));
                            st.mem.set_b32s1(r, st.mem.b32s1(HOLD_HEAD));
                            r = u.0;
                        }
                        r = st.mem.b32s1(st.mem.b32s1(r));
                        ss = st.mem.b32s1(st.mem.b32s1(ss));
                        if r == TEX_NULL {
                            break;
                        }
                    }
                }
                Ok(AnyNode::Rule(rule)) => {
                    let (w, h, d) = (p.width(&st.mem), p.height(&st.mem), p.depth(&st.mem));
                    if rule.width(&st.mem) == RuleDimen::Running {
                        rule.set_width(&mut st.mem, RuleDimen::Set(w));
                    }
                    if rule.height(&st.mem) == RuleDimen::Running {
                        rule.set_height(&mut st.mem, RuleDimen::Set(h));
                    }
                    if rule.depth(&st.mem) == RuleDimen::Running {
                        rule.set_depth(&mut st.mem, RuleDimen::Set(d));
                    }
                    if o != 0 {
                        let next = rule.link(&st.mem);
                        rule.set_link(&mut st.mem, MemPtr::NULL);
                        let boxed = crate::packaging::hpack_emit_in(st, rule.ptr(), 0, ADDITIONAL)?;
                        boxed.set_shift_amount(&mut st.mem, Scaled(o));
                        boxed.set_link(&mut st.mem, next);
                        MemPtr(s).set_link(&mut st.mem, boxed.ptr());
                        q = boxed.idx().0;
                    }
                }
                _ => {}
            }
        }
        s = q;
        q = st.mem.b32s1(q);
    }

    flush_node_list(&mut st.mem, p.idx().0).map_err(MainControlAbort::Confusion)?;
    pop_alignment_in(st);

    let aux_save_s0 = st.input.cur_list.aux_s0();
    let aux_save_s1 = st.input.cur_list.aux_s1();
    let rows = st.mem.link_ptr(st.input.cur_list.head);
    let rows_tail = st.input.cur_list.tail;
    pop_nest_in(&mut st.mem, &mut st.input);

    if st.input.cur_list.mode == MMODE {
        do_assignments_in(st)?;
        if st.scanner.cur_cmd != MATH_SHIFT {
            crate::output::error_here_with_diagnostic_in(st, c"Missing $$ inserted".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"Displays can use special alignments (like \\eqalignno)",
                    c"only if nothing but the alignment itself is between $$'s.",
                ],
            );
            mc_propagate_error(crate::input::back_error_in(st)?)?;
        } else {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd != MATH_SHIFT {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Display math should end with $$".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"The `$' that I just saw supposedly matches a previous `$$'.",
                        c"So I shall assume that you typed `$$' both times.",
                    ],
                );
                mc_propagate_error(crate::input::back_error_in(st)?)?;
            }
        }
        let etex_aux = st.input.cur_list.e_tex_aux.0;
        flush_node_list(&mut st.mem, etex_aux).map_err(MainControlAbort::Confusion)?;
        pop_nest_in(&mut st.mem, &mut st.input);
        let pre_penalty = st.eqtb.b32s1(INT_BASE + INT_PAR_PRE_DISPLAY_PENALTY);
        let pen = alloc_penalty(&mut st.mem, pre_penalty).ok_or(NodeRecycleAbort::Overflow)?;
        tail_append(st, pen.ptr());
        let above = alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_ABOVE_DISPLAY_SKIP)
            .ok_or(NodeRecycleAbort::Overflow)?;
        tail_append(st, above.ptr());
        st.input.cur_list.tail.set_link(&mut st.mem, rows);
        if !rows.is_null() {
            st.input.cur_list.tail = rows_tail;
        }
        let post_penalty = st.eqtb.b32s1(INT_BASE + INT_PAR_POST_DISPLAY_PENALTY);
        let pen = alloc_penalty(&mut st.mem, post_penalty).ok_or(NodeRecycleAbort::Overflow)?;
        tail_append(st, pen.ptr());
        let below = alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_BELOW_DISPLAY_SKIP)
            .ok_or(NodeRecycleAbort::Overflow)?;
        tail_append(st, below.ptr());
        st.input.cur_list.set_aux_s1(aux_save_s1);
        crate::math::resume_after_display_in(st)?;
    } else {
        st.input.cur_list.set_aux_s0(aux_save_s0);
        st.input.cur_list.set_aux_s1(aux_save_s1);
        st.input.cur_list.tail.set_link(&mut st.mem, rows);
        if !rows.is_null() {
            st.input.cur_list.tail = rows_tail;
        }
        if st.input.cur_list.mode == VMODE {
            crate::pagebuilder::build_page_in(st)?;
        }
    }
    Ok(())
}

fn move_preamble_dimen(mem: &mut MemState, from: i32, to: i32) {
    let mut q = mem.b32s1(mem.b32s1(ALIGN_HEAD));
    loop {
        mem.set_b32s1(q + to, mem.b32s1(q + from));
        mem.set_b32s1(q + from, 0);
        q = mem.b32s1(mem.b32s1(q));
        if q == TEX_NULL {
            break;
        }
    }
}

/// The glue setting of unset node `col`, of natural size `natural`, when set to size `t`.
fn unset_glue(
    mem: &MemState,
    col: NodePtr<UnsetNode>,
    t: i32,
    natural: i32,
) -> (GlueSign, u16, f64) {
    if t == natural {
        return (GlueSign::Normal, NORMAL, 0.0);
    }
    if t > natural {
        let stretch = col.glue_stretch(mem).0;
        let ratio = if stretch == 0 {
            0.0
        } else {
            (t - natural) as f64 / stretch as f64
        };
        return (GlueSign::Stretching, col.glue_order(mem), ratio);
    }
    let order = col.glue_sign(mem);
    let shrink = col.glue_shrink(mem).0;
    let ratio = if shrink == 0 {
        0.0
    } else if order == NORMAL && natural - t > shrink {
        1.0
    } else {
        (natural - t) as f64 / shrink as f64
    };
    (GlueSign::Shrinking, order, ratio)
}

pub(crate) fn align_peek_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    loop {
        st.scanner.align_state = 1_000_000;
        get_x_non_blank_or_protected(st)?;
        match st.scanner.cur_cmd {
            NO_ALIGN => {
                crate::scan::scan_left_brace_in(st)?;
                crate::save::new_save_level(&mut st.save_stack, GroupCode::NoAlign, st.mem.line)?;
                if st.input.cur_list.mode == INNER_VMODE {
                    normal_paragraph(st)?;
                }
                return Ok(());
            }
            RIGHT_BRACE => return fin_align_in(st),
            CAR_RET if st.scanner.cur_chr == CR_CR_CODE => {}
            _ => {
                init_row_in(st)?;
                return init_col_in(st);
            }
        }
    }
}

fn get_x_non_blank_or_protected(st: &mut EngineState) -> Result<(), MainControlAbort> {
    loop {
        crate::scan::get_x_or_protected_in(st)?;
        if st.scanner.cur_cmd != SPACER {
            return Ok(());
        }
    }
}

pub(crate) fn get_preamble_token_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    loop {
        crate::scan::get_token_in(st)?;
        while st.scanner.cur_chr == SPAN_CODE && st.scanner.cur_cmd == TAB_MARK {
            crate::scan::get_token_in(st)?;
            if st.scanner.cur_cmd > MAX_COMMAND {
                crate::expand::expand_in(st)?;
                crate::scan::get_token_in(st)?;
            }
        }
        if st.scanner.cur_cmd == ENDV {
            return Err(MainControlAbort::Input(crate::input::InputAbort::Fatal(
                c"(interwoven alignment preambles are not allowed)".as_ptr(),
            )));
        }
        if st.scanner.cur_cmd != ASSIGN_GLUE || st.scanner.cur_chr != GLUE_BASE_TAB_SKIP {
            return Ok(());
        }
        crate::scan::scan_optional_equals_in(st)?;
        crate::scan::scan_glue_in(st, GLUE_VAL)?;
        let global = st.eqtb.b32s1(INT_BASE + INT_PAR_GLOBAL_DEFS) > 0;
        define(st, global, GLUE_BASE_TAB_SKIP, GLUE_REF, st.scanner.cur_val)?;
    }
}

pub(crate) fn init_span_in(st: &mut EngineState, p: MemPtr) -> Result<(), MainControlAbort> {
    crate::mem::push_nest_in(st)?;
    if st.input.cur_list.mode == INNER_HMODE {
        st.input.cur_list.set_aux_s0(1000);
    } else {
        st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
        normal_paragraph(st)?;
    }
    st.scanner.cur_span = p;
    Ok(())
}

pub(crate) fn init_row_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = -HMODE - VMODE - st.input.cur_list.mode;
    if st.input.cur_list.mode == INNER_HMODE {
        st.input.cur_list.set_aux_s0(0);
    } else {
        st.input.cur_list.set_aux_s1(0);
    }
    let tab_skip = st.mem.link_ptr(ALIGN_HEAD);
    let spec = st.mem.b32s0(tab_skip + 1);
    let glue =
        crate::mem::alloc_glue(&mut st.mem, spec).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    tail_append(st, glue.ptr());
    glue.set_subtype(&mut st.mem, TAB_SKIP_SUBTYPE);
    st.scanner.cur_align = tab_skip.link(&st.mem);
    st.scanner.cur_tail = st.scanner.cur_head;
    st.scanner.cur_pre_tail = st.scanner.cur_pre_head;
    init_span_in(st, st.scanner.cur_align)
}

pub(crate) fn init_col_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    st.mem
        .set_b32s0(st.scanner.cur_align + 5, st.scanner.cur_cmd as i32);
    if st.scanner.cur_cmd == OMIT {
        st.scanner.align_state = 0;
    } else {
        crate::input::back_input_in(st)?;
        crate::input::begin_token_list_in(st, st.mem.b32s1(st.scanner.cur_align + 3), U_TEMPLATE)?;
    }
    Ok(())
}

pub(crate) fn push_alignment_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let p = crate::scan::get_node_scc(st, ALIGN_STACK_NODE_SIZE)?;
    st.mem.set_b32s1(p, st.scanner.align_ptr.0);
    st.mem.set_b32s0(p, st.scanner.cur_align.0);
    st.mem.set_b32s0(p + 1, st.mem.b32s1(ALIGN_HEAD));
    st.mem.set_b32s1(p + 1, st.scanner.cur_span.0);
    st.mem.set_b32s1(p + 2, st.scanner.cur_loop.0);
    st.mem.set_b32s1(p + 3, st.scanner.align_state);
    st.mem.set_b32s0(p + 4, st.scanner.cur_head.0);
    st.mem.set_b32s1(p + 4, st.scanner.cur_tail.0);
    st.mem.set_b32s0(p + 5, st.scanner.cur_pre_head.0);
    st.mem.set_b32s1(p + 5, st.scanner.cur_pre_tail.0);
    st.scanner.align_ptr = p;
    st.scanner.cur_head = crate::scan::get_avail_scc(st)?;
    st.scanner.cur_pre_head = crate::scan::get_avail_scc(st)?;
    Ok(())
}

pub(crate) fn pop_alignment_in(st: &mut EngineState) {
    crate::mem::free_avail(&mut st.mem, st.scanner.cur_head);
    crate::mem::free_avail(&mut st.mem, st.scanner.cur_pre_head);
    let p = st.scanner.align_ptr;
    st.scanner.cur_tail.0 = st.mem.b32s1(p + 4);
    st.scanner.cur_head.0 = st.mem.b32s0(p + 4);
    st.scanner.cur_pre_tail.0 = st.mem.b32s1(p + 5);
    st.scanner.cur_pre_head.0 = st.mem.b32s0(p + 5);
    st.scanner.align_state = st.mem.b32s1(p + 3);
    st.scanner.cur_loop.0 = st.mem.b32s1(p + 2);
    st.scanner.cur_span.0 = st.mem.b32s1(p + 1);
    st.mem.set_b32s1(ALIGN_HEAD, st.mem.b32s0(p + 1));
    st.scanner.cur_align.0 = st.mem.b32s0(p);
    st.scanner.align_ptr.0 = st.mem.b32s1(p);
    crate::mem::free_node(&mut st.mem, p, ALIGN_STACK_NODE_SIZE);
}

/// TeX's `store_new_token`: appends `tok` after `p` and returns the new tail.
fn store_new_token(
    mem: &mut MemState,
    p: MemPtr,
    tok: i32,
) -> Result<MemPtr, crate::scan::NodeRecycleAbort> {
    let q = crate::mem::get_avail(mem).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    p.set_link(mem, q);
    q.set_info(mem, tok);
    Ok(q)
}

/// Copies token list `r` through `HOLD_HEAD` and returns the copy.
fn copy_token_list(mem: &mut MemState, mut r: i32) -> Result<i32, crate::scan::NodeRecycleAbort> {
    let mut q = HOLD_HEAD;
    while r != TEX_NULL {
        q = store_new_token(mem, q, mem.b32s0(r))?;
        r = mem.b32s1(r);
    }
    mem.set_b32s1(q, TEX_NULL);
    Ok(mem.b32s1(HOLD_HEAD))
}

pub(crate) fn fin_col_in(st: &mut EngineState) -> Result<bool, MainControlAbort> {
    use crate::scan::NodeRecycleAbort;

    if st.scanner.cur_align.is_null() {
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
        if !st.scanner.cur_loop.is_null() {
            let nb = crate::mem::alloc_null_box(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
            MemPtr(q).set_link(&mut st.mem, nb.ptr());
            p = nb.idx().0;
            st.mem.set_b32s0(p, END_SPAN.0);
            nb.set_width(&mut st.mem, Scaled(NULL_FLAG));
            st.scanner.cur_loop = st.scanner.cur_loop.link(&st.mem);
            let u_template = st.mem.b32s1(st.scanner.cur_loop + 3);
            let u_part = copy_token_list(&mut st.mem, u_template)?;
            st.mem.set_b32s1(p + 3, u_part);
            let v_template = st.mem.b32s1(st.scanner.cur_loop + 2);
            let v_part = copy_token_list(&mut st.mem, v_template)?;
            st.mem.set_b32s1(p + 2, v_part);
            st.scanner.cur_loop = st.scanner.cur_loop.link(&st.mem);
            let spec = st.mem.b32s0(st.scanner.cur_loop + 1);
            let g = crate::mem::alloc_glue(&mut st.mem, spec).ok_or(NodeRecycleAbort::Overflow)?;
            nb.set_link(&mut st.mem, g.ptr());
            g.set_subtype(&mut st.mem, TAB_SKIP_SUBTYPE);
        } else {
            crate::output::error_here_with_diagnostic_in(
                st,
                c"Extra alignment tab has been changed to ".as_ptr(),
            );
            crate::output::print_esc_cstr_in(st, c"cr".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"You have given more \\span or & marks than there were",
                    c"in the preamble to the \\halign or \\valign now in progress.",
                    c"So I'll assume that you meant to type \\cr instead.",
                ],
            );
            st.mem.set_b32s0(st.scanner.cur_align + 5, CR_CODE);
            mc_propagate_error(crate::errors::error_in(st))?;
        }
    }
    if st.mem.b32s0(st.scanner.cur_align + 5) != SPAN_CODE {
        crate::save::unsave_in(st)?;
        crate::save::new_save_level(&mut st.save_stack, GroupCode::Align, st.mem.line)?;
        let head = st.mem.link_ptr(st.input.cur_list.head);
        let (u, w) = if st.input.cur_list.mode == INNER_HMODE {
            st.scanner.adjust_tail = st.scanner.cur_tail;
            st.scanner.pre_adjust_tail = st.scanner.cur_pre_tail;
            let u = crate::packaging::hpack_emit_in(st, head, 0, ADDITIONAL)?;
            st.scanner.cur_tail = st.scanner.adjust_tail;
            st.scanner.adjust_tail = MemPtr::NULL;
            st.scanner.cur_pre_tail = st.scanner.pre_adjust_tail;
            st.scanner.pre_adjust_tail = MemPtr::NULL;
            (u, u.width(&st.mem).0)
        } else {
            let u = crate::packaging::vpackage_emit_in(st, head, 0, ADDITIONAL, 0)?;
            (u, u.height(&st.mem).0)
        };
        let mut n: i32 = 0;
        if st.scanner.cur_span != st.scanner.cur_align {
            let mut q = st.scanner.cur_span.0;
            loop {
                n += 1;
                q = st.mem.b32s1(st.mem.b32s1(q));
                if q == st.scanner.cur_align.0 {
                    break;
                }
            }
            if n > u16::MAX as i32 {
                return Err(MainControlAbort::Confusion(c"too many spans"));
            }
            q = st.scanner.cur_span.0;
            while st.mem.b32s1(st.mem.b32s0(q)) < n {
                q = st.mem.b32s0(q);
            }
            if st.mem.b32s1(st.mem.b32s0(q)) > n {
                let s = crate::mem::get_node(&mut st.mem, SPAN_NODE_SIZE)
                    .ok_or(NodeRecycleAbort::Overflow)?;
                let next = st.mem.b32s0(q);
                s.set_info(&mut st.mem, next);
                s.set_link(&mut st.mem, MemPtr(n));
                st.mem.set_b32s0(q, s.0);
                st.mem.set_b32s1(s + 1, w);
            } else if st.mem.b32s1(st.mem.b32s0(q) + 1) < w {
                st.mem.set_b32s1(st.mem.b32s0(q) + 1, w);
            }
        } else if w > st.mem.b32s1(st.scanner.cur_align + 1) {
            st.mem.set_b32s1(st.scanner.cur_align + 1, w);
        }
        let u = u.into_unset(&mut st.mem);
        u.set_subtype(&mut st.mem, n as u16);
        let o = crate::packaging::glue_order(&st.scanner.total_stretch);
        u.set_glue_order(&mut st.mem, o);
        u.set_glue_stretch(&mut st.mem, Scaled(st.scanner.total_stretch[o as usize]));
        let o = crate::packaging::glue_order(&st.scanner.total_shrink);
        u.set_glue_sign(&mut st.mem, o);
        u.set_glue_shrink(&mut st.mem, Scaled(st.scanner.total_shrink[o as usize]));
        crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
        tail_append(st, u.ptr());
        let spec = st.mem.b32s0(st.mem.b32s1(st.scanner.cur_align) + 1);
        let g = crate::mem::alloc_glue(&mut st.mem, spec).ok_or(NodeRecycleAbort::Overflow)?;
        tail_append(st, g.ptr());
        g.set_subtype(&mut st.mem, TAB_SKIP_SUBTYPE);
        if st.mem.b32s0(st.scanner.cur_align + 5) >= CR_CODE {
            return Ok(true);
        }
        init_span_in(st, MemPtr(p))?;
    }
    st.scanner.align_state = 1_000_000;
    get_x_non_blank_or_protected(st)?;
    st.scanner.cur_align = MemPtr(p);
    init_col_in(st)?;
    Ok(false)
}

fn append_list(st: &mut EngineState, head: MemPtr, tail: MemPtr) {
    let first = head.link(&st.mem);
    st.input.cur_list.tail.set_link(&mut st.mem, first);
    st.input.cur_list.tail = tail;
}

fn begin_every(st: &mut EngineState, local: i32, kind: u16) -> Result<(), MainControlAbort> {
    let list = st.eqtb.b32s1(LOCAL_BASE + local);
    if list != TEX_NULL {
        crate::input::begin_token_list_in(st, list, kind)?;
    }
    Ok(())
}

pub(crate) fn fin_row_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    use crate::mem::pop_nest_in;

    let head = st.mem.link_ptr(st.input.cur_list.head);
    let p = if st.input.cur_list.mode == INNER_HMODE {
        let row = crate::packaging::hpack_emit_in(st, head, 0, ADDITIONAL)?;
        pop_nest_in(&mut st.mem, &mut st.input);
        if st.scanner.cur_pre_head != st.scanner.cur_pre_tail {
            append_list(st, st.scanner.cur_pre_head, st.scanner.cur_pre_tail);
        }
        crate::packaging::append_to_vlist(&mut st.input.cur_list, &mut st.mem, &st.eqtb, row)?;
        if st.scanner.cur_head != st.scanner.cur_tail {
            append_list(st, st.scanner.cur_head, st.scanner.cur_tail);
        }
        row
    } else {
        let row = crate::packaging::vpackage_emit_in(st, head, 0, ADDITIONAL, MAX_HALFWORD)?;
        pop_nest_in(&mut st.mem, &mut st.input);
        tail_append(st, row.ptr());
        st.input.cur_list.set_aux_s0(1000);
        row
    };
    p.into_unset(&mut st.mem)
        .set_glue_stretch(&mut st.mem, Scaled(0));
    begin_every(st, LOCAL_EVERY_CR, EVERY_CR_TEXT)?;
    align_peek_in(st)
}

pub(crate) fn init_align_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    use crate::mem::{alloc_null_box, alloc_param_glue};
    use crate::scan::NodeRecycleAbort;

    let save_cs_ptr = st.scanner.cur_cs;
    push_alignment_in(st)?;
    st.scanner.align_state = -1_000_000;

    if st.input.cur_list.mode == MMODE
        && (st.input.cur_list.tail != st.input.cur_list.head
            || st.input.cur_list.aux_s1() != TEX_NULL)
    {
        crate::output::error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
        crate::output::print_esc_cstr_in(st, c"halign".as_ptr());
        crate::output::print_cstr_in(st, c" inside $$'s".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[
                c"Displays can use special alignments (like \\eqalignno)",
                c"only if nothing but the alignment itself is between $$'s.",
                c"So I've deleted the formulas that preceded this alignment.",
            ],
        );
        mc_propagate_error(crate::errors::error_in(st))?;
        crate::math::flush_math(&mut st.input.cur_list, &mut st.mem)
            .map_err(MainControlAbort::Confusion)?;
    }

    crate::mem::push_nest_in(st)?;
    if st.input.cur_list.mode == MMODE {
        st.input.cur_list.mode = INNER_VMODE;
        let prev_depth = st.input.nest[(st.input.nest_ptr - 2) as usize].aux_s1();
        st.input.cur_list.set_aux_s1(prev_depth);
    } else if st.input.cur_list.mode > 0 {
        st.input.cur_list.mode = -st.input.cur_list.mode;
    }

    crate::scan::scan_spec_in(st, GroupCode::Align, false)?;
    st.mem.set_b32s1(ALIGN_HEAD, TEX_NULL);
    st.scanner.cur_align = ALIGN_HEAD;
    st.scanner.cur_loop = MemPtr::NULL;
    st.scanner.scanner_status = ALIGNING;
    st.scanner.warning_index = save_cs_ptr;
    st.scanner.align_state = -1_000_000;

    loop {
        let g = alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_TAB_SKIP)
            .ok_or(NodeRecycleAbort::Overflow)?
            .ptr();
        st.scanner.cur_align.set_link(&mut st.mem, g);
        st.scanner.cur_align = g;
        if st.scanner.cur_cmd == CAR_RET {
            break;
        }

        let mut p = HOLD_HEAD;
        st.mem.set_b32s1(p, TEX_NULL);
        loop {
            get_preamble_token_in(st)?;
            if st.scanner.cur_cmd == MAC_PARAM {
                break;
            }
            if (TAB_MARK..=CAR_RET).contains(&st.scanner.cur_cmd)
                && st.scanner.align_state == -1_000_000
            {
                if p == HOLD_HEAD && st.scanner.cur_loop.is_null() && st.scanner.cur_cmd == TAB_MARK
                {
                    st.scanner.cur_loop = st.scanner.cur_align;
                } else {
                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"Missing # inserted in alignment preamble".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    help(
                        &mut st.errors,
                        &[
                            c"There should be exactly one # between &'s, when an",
                            c"\\halign or \\valign is being set up. In this case you had",
                            c"none, so I've put one in; maybe that will work.",
                        ],
                    );
                    mc_propagate_error(crate::input::back_error_in(st)?)?;
                    break;
                }
            } else if st.scanner.cur_cmd != SPACER || p != HOLD_HEAD {
                p = store_new_token(&mut st.mem, p, st.scanner.cur_tok)?;
            }
        }

        let nb = alloc_null_box(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
        st.scanner.cur_align.set_link(&mut st.mem, nb.ptr());
        st.scanner.cur_align = nb.ptr();
        st.mem.set_b32s0(nb.idx(), END_SPAN.0);
        nb.set_width(&mut st.mem, Scaled(NULL_FLAG));
        st.mem
            .set_b32s1(nb.idx().offset(3), st.mem.b32s1(HOLD_HEAD));

        p = HOLD_HEAD;
        st.mem.set_b32s1(p, TEX_NULL);
        loop {
            get_preamble_token_in(st)?;
            if (TAB_MARK..=CAR_RET).contains(&st.scanner.cur_cmd)
                && st.scanner.align_state == -1_000_000
            {
                break;
            }
            if st.scanner.cur_cmd == MAC_PARAM {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Only one # is allowed per tab".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"There should be exactly one # between &'s, when an",
                        c"\\halign or \\valign is being set up. In this case you had",
                        c"more than one, so I'm ignoring all but the first.",
                    ],
                );
                mc_propagate_error(crate::errors::error_in(st))?;
                continue;
            }
            p = store_new_token(&mut st.mem, p, st.scanner.cur_tok)?;
        }
        store_new_token(&mut st.mem, p, CS_TOKEN_FLAG + FROZEN_END_TEMPLATE)?;
        st.mem
            .set_b32s1(st.scanner.cur_align + 2, st.mem.b32s1(HOLD_HEAD));
    }

    st.scanner.scanner_status = NORMAL_SCANNER;
    crate::save::new_save_level(&mut st.save_stack, GroupCode::Align, st.mem.line)?;
    begin_every(st, LOCAL_EVERY_CR, EVERY_CR_TEXT)?;
    align_peek_in(st)
}

pub(crate) fn push_math_in(st: &mut EngineState, c: GroupCode) -> Result<(), MainControlAbort> {
    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = -MMODE;
    st.input.cur_list.set_aux_s1(TEX_NULL);
    crate::save::new_save_level(&mut st.save_stack, c, st.mem.line)?;
    Ok(())
}

pub(crate) fn end_graf_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.input.cur_list.mode != HMODE {
        return Ok(());
    }
    if st.input.cur_list.head == st.input.cur_list.tail {
        crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
    } else {
        match crate::linebreak::line_break_in(st, false) {
            crate::linebreak::LineBreakOutcome::Done => {}
            o => return Err(MainControlAbort::LineBreak(o)),
        }
    }
    let lr_save = st.input.cur_list.e_tex_aux;
    if !lr_save.is_null() {
        crate::mem::flush_list(&mut st.mem, lr_save);
        st.input.cur_list.e_tex_aux = MemPtr::NULL;
    }
    normal_paragraph(st)?;
    st.errors.error_count = 0;
    Ok(())
}

pub(crate) fn append_penalty_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::scan::scan_int_in(st)?;
    let p = crate::mem::alloc_penalty(&mut st.mem, st.scanner.cur_val)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    tail_append(st, p.ptr());
    if st.input.cur_list.mode == VMODE {
        crate::pagebuilder::build_page_in(st)?;
    }
    Ok(())
}

pub(crate) fn normal_paragraph(st: &mut EngineState) -> Result<(), crate::save::SaveAbort> {
    for (loc, value) in [
        (INT_BASE + INT_PAR_LOOSENESS, 0),
        (DIMEN_BASE + DIMEN_PAR_HANG_INDENT, 0),
        (INT_BASE + INT_PAR_HANG_AFTER, 1),
    ] {
        if st.eqtb.b32s1(loc) != value {
            word_define(st, false, loc, value)?;
        }
    }
    for loc in [LOCAL_BASE + LOCAL_PAR_SHAPE, ETEX_PEN_BASE] {
        if st.eqtb.b32s1(loc) != TEX_NULL {
            define(st, false, loc, SHAPE_REF, TEX_NULL)?;
        }
    }
    Ok(())
}

pub(crate) fn its_all_over_in(st: &mut EngineState) -> Result<bool, MainControlAbort> {
    use crate::scan::NodeRecycleAbort;

    if !crate::errors::privileged_in(st).map_err(MainControlAbort::Error)? {
        return Ok(false);
    }
    if st.shipout.page_tail == PAGE_HEAD
        && st.input.cur_list.head == st.input.cur_list.tail
        && st.shipout.dead_cycles == 0
    {
        return Ok(true);
    }
    crate::input::back_input_in(st)?;
    let b = crate::mem::alloc_null_box(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
    tail_append(st, b.ptr());
    b.set_width(
        &mut st.mem,
        Scaled(st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_HSIZE)),
    );
    let g = crate::mem::alloc_glue(&mut st.mem, FILL_GLUE).ok_or(NodeRecycleAbort::Overflow)?;
    tail_append(st, g.ptr());
    let p = crate::mem::alloc_penalty(&mut st.mem, NULL_FLAG).ok_or(NodeRecycleAbort::Overflow)?;
    tail_append(st, p.ptr());
    crate::pagebuilder::build_page_in(st)?;
    Ok(false)
}

pub(crate) fn indent_in_hmode(st: &mut EngineState) -> Result<(), crate::scan::NodeRecycleAbort> {
    use crate::scan::NodeRecycleAbort;

    if st.scanner.cur_chr <= 0 {
        return Ok(());
    }
    let b = crate::mem::alloc_null_box(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
    b.set_width(
        &mut st.mem,
        Scaled(st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_PAR_INDENT)),
    );
    let p = if st.input.cur_list.mode.abs() == HMODE {
        st.input.cur_list.set_aux_s0(1000);
        b.ptr()
    } else {
        let q = crate::mem::alloc_noad(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
        q.set_nucleus_type(&mut st.mem, SUB_BOX);
        q.set_nucleus_field(&mut st.mem, b.idx().0);
        q.ptr()
    };
    tail_append(st, p);
    Ok(())
}

pub(crate) fn head_for_vmode_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.input.cur_list.mode >= 0 {
        crate::input::back_input_in(st)?;
        st.scanner.cur_tok = st.input.par_token;
        crate::input::back_input_in(st)?;
        st.input.cur_input.index = INSERTED;
        return Ok(());
    }
    if st.scanner.cur_cmd != HRULE {
        return off_save_in(st);
    }
    crate::output::error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
    crate::output::print_esc_cstr_in(st, c"hrule".as_ptr());
    crate::output::print_cstr_in(st, c"' here except with leaders".as_ptr());
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    help(
        &mut st.errors,
        &[
            c"To put a horizontal rule in an hbox or an alignment,",
            c"you should use \\leaders or \\hrulefill (see The TeXbook).",
        ],
    );
    mc_propagate_error(crate::errors::error_in(st))
}

pub(crate) fn begin_insert_or_adjust_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.cur_cmd == VADJUST {
        st.scanner.cur_val = 255;
    } else {
        crate::scan::scan_eight_bit_int_in(st)?;
        if st.scanner.cur_val == 255 {
            crate::output::error_here_with_diagnostic_in(st, c"You can't ".as_ptr());
            crate::output::print_esc_cstr_in(st, c"insert".as_ptr());
            crate::output::print_int_in(st, st.scanner.cur_val);
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[c"I'm changing to \\insert0; box 255 is special."],
            );
            mc_propagate_error(crate::errors::error_in(st))?;
            st.scanner.cur_val = 0;
        }
    }
    let sp = st.save_stack.save_ptr;
    st.save_stack.set_b32s1(sp, st.scanner.cur_val);
    let pre = st.scanner.cur_cmd == VADJUST && crate::scan::scan_keyword_in(st, c"pre".as_ptr())?;
    st.save_stack.set_b32s1(sp + 1, pre as i32);
    st.save_stack.save_ptr = sp + 2;
    crate::save::new_save_level(&mut st.save_stack, GroupCode::Insert, st.mem.line)?;
    crate::scan::scan_left_brace_in(st)?;
    normal_paragraph(st)?;
    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = INNER_VMODE;
    st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
    Ok(())
}

pub(crate) fn do_endv_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    fn is_v_template_end(r: &crate::globals::InputState) -> bool {
        r.index == V_TEMPLATE && r.loc == TEX_NULL && r.state == TOKEN_LIST
    }

    st.input.base_ptr = st.input.input_ptr;
    st.input.input_stack[st.input.base_ptr as usize] = st.input.cur_input;
    while st.input.input_stack[st.input.base_ptr as usize].index != V_TEMPLATE
        && st.input.input_stack[st.input.base_ptr as usize].loc == TEX_NULL
        && st.input.input_stack[st.input.base_ptr as usize].state == TOKEN_LIST
    {
        st.input.base_ptr -= 1;
    }
    if !is_v_template_end(&st.input.input_stack[st.input.base_ptr as usize]) {
        return Err(MainControlAbort::Input(crate::input::InputAbort::Fatal(
            c"(interwoven alignment preambles are not allowed)".as_ptr(),
        )));
    }
    if st.save_stack.cur_group == GroupCode::Align {
        end_graf_in(st)?;
        if fin_col_in(st)? {
            fin_row_in(st)?;
        }
        Ok(())
    } else {
        off_save_in(st)
    }
}

pub(crate) fn fetch_box(st: &mut EngineState, n: i32) -> Option<AnyBox> {
    let p = if n < 256 {
        MemPtr::from_raw(st.eqtb.b32s1(BOX_BASE + n))
    } else {
        crate::save::find_sa_element(&mut st.mem, &mut st.scanner, BOX_VAL, n, false);
        NodePtr::<SaNode>::from_ptr(st.scanner.cur_ptr)?.sa_ptr(&st.mem)
    };
    st.mem.classify_box(p.as_idx()?).ok().flatten()
}

/// TeX's `change_box`: sets box register `n` to `v` without saving.
fn change_box(st: &mut EngineState, n: i32, v: i32) {
    if n < 256 {
        st.eqtb.set_b32s1(BOX_BASE + n, v);
        return;
    }
    crate::save::find_sa_element(&mut st.mem, &mut st.scanner, BOX_VAL, n, false);
    if let Some(sa) = NodePtr::<SaNode>::from_ptr(st.scanner.cur_ptr) {
        sa.set_sa_ptr(&mut st.mem, MemPtr::from_raw(v));
        let refs = sa.sa_ref(&st.mem);
        sa.set_sa_ref(&mut st.mem, refs + 1);
        crate::save::delete_sa_ref(&mut st.mem, &mut st.scanner, sa.idx());
    }
}

pub(crate) fn unpackage_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.cur_chr > COPY_CODE {
        let disc = &mut st.scanner.disc_ptr[st.scanner.cur_chr as usize];
        st.input.cur_list.tail.set_link(&mut st.mem, *disc);
        *disc = MemPtr::NULL;
    } else {
        let c = st.scanner.cur_chr;
        crate::scan::scan_register_num_in(st)?;
        let Some(b) = fetch_box(st, st.scanner.cur_val) else {
            return Ok(());
        };
        let mode = st.input.cur_list.mode.abs();
        if mode == MMODE
            || (mode == VMODE && !matches!(b, AnyBox::VList(_)))
            || (mode == HMODE && !matches!(b, AnyBox::HList(_)))
        {
            crate::output::error_here_with_diagnostic_in(
                st,
                c"Incompatible list can't be unboxed".as_ptr(),
            );
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"Sorry, Pandora. (You sneaky devil.)",
                    c"I refuse to unbox an \\hbox in vertical mode or vice versa.",
                    c"And I can't open any boxes in math mode.",
                ],
            );
            return mc_propagate_error(crate::errors::error_in(st));
        }
        let list = b.list_ptr(&st.mem);
        if c == COPY_CODE {
            let copy = crate::packaging::copy_node_list(&mut st.mem, list.0)?;
            st.input
                .cur_list
                .tail
                .set_link(&mut st.mem, MemPtr::from_raw(copy));
        } else {
            st.input.cur_list.tail.set_link(&mut st.mem, list);
            change_box(st, st.scanner.cur_val, TEX_NULL);
            crate::mem::free_node(&mut st.mem, b.ptr(), BOX_NODE_SIZE);
        }
    }

    loop {
        let r = st.input.cur_list.tail.link(&st.mem);
        if r.is_null() {
            return Ok(());
        }
        if !is_char_node(&st.mem, r)
            && matches!(st.mem.classify(MemIdx(r.0)), Ok(AnyNode::MarginKern(_)))
        {
            let next = r.link(&st.mem);
            st.input.cur_list.tail.set_link(&mut st.mem, next);
            crate::mem::free_node(&mut st.mem, r, MARGIN_KERN_NODE_SIZE);
        } else {
            st.input.cur_list.tail = r;
        }
    }
}

pub(crate) fn delete_last_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.input.cur_list.mode == VMODE && st.input.cur_list.tail == st.input.cur_list.head {
        if st.scanner.cur_chr as u16 != GLUE_NODE || st.shipout.last_glue.0 != MAX_HALFWORD {
            crate::errors::you_cant_in(st);
            help(
                &mut st.errors,
                &[
                    c"Sorry...I usually can't take things from the current page.",
                    c"Try `I\\vskip-\\lastskip' instead.",
                ],
            );
            if st.scanner.cur_chr as u16 == KERN_NODE {
                st.errors.help_line[0] = c"Try `I\\kern-\\lastkern' instead.".as_ptr();
            } else if st.scanner.cur_chr as u16 != GLUE_NODE {
                st.errors.help_line[0] = c"Perhaps you can make the output routine do it.".as_ptr();
            }
            return mc_propagate_error(crate::errors::error_in(st));
        }
        return Ok(());
    }
    let tx = effective_tail(st);
    if is_char_node(&st.mem, tx) || st.mem.b16s1(tx) as i32 != st.scanner.cur_chr {
        return Ok(());
    }
    if unlink_effective_tail(st, tx)? {
        crate::packaging::flush_node_list(&mut st.mem, tx).map_err(MainControlAbort::Confusion)?;
    }
    Ok(())
}

/// Appends interword glue for the current space factor. Returns the
/// `xn_over_d` side effects (`tex_remainder` and overflow) when it scaled.
pub(crate) fn app_space(
    st: &mut EngineState,
) -> Result<Option<(i32, bool)>, crate::scan::NodeRecycleAbort> {
    use crate::scan::NodeRecycleAbort;

    let space_factor = st.input.cur_list.aux_s0();
    let q;
    let mut side_effects = None;
    if space_factor >= 2000 && st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_XSPACE_SKIP as i32) != 0 {
        q = crate::mem::alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_XSPACE_SKIP)
            .ok_or(NodeRecycleAbort::Overflow)?
            .ptr();
    } else {
        let space_skip = st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP as i32);
        st.mainloop.main_p.0 = if space_skip != 0 {
            space_skip
        } else {
            font_space_glue(st)?
        };
        let spec = crate::mem::alloc_spec(&mut st.mem, st.mainloop.main_p.0)
            .ok_or(NodeRecycleAbort::Overflow)?;
        st.mainloop.main_p = spec.ptr();
        if space_factor >= 2000 {
            let cf = st.eqtb.b32s1(CUR_FONT_LOC);
            let extra = st
                .fonts
                .font_info_b32s1(EXTRA_SPACE_CODE + st.fonts.param_base(cf));
            let width = spec.width(&st.mem).0 + extra;
            spec.set_width(&mut st.mem, Scaled(width));
        }
        let (stretch, _, stretch_overflow) =
            crate::scaled_math::xn_over_d_parts(spec.stretch(&st.mem).0, space_factor, 1000);
        spec.set_stretch(&mut st.mem, Scaled(stretch));
        let (shrink, remainder, shrink_overflow) =
            crate::scaled_math::xn_over_d_parts(spec.shrink(&st.mem).0, 1000, space_factor);
        spec.set_shrink(&mut st.mem, Scaled(shrink));
        side_effects = Some((remainder, stretch_overflow || shrink_overflow));
        q = crate::mem::alloc_glue(&mut st.mem, spec.idx())
            .ok_or(NodeRecycleAbort::Overflow)?
            .ptr();
        spec.set_ref_count(&mut st.mem, TEX_NULL);
    }
    tail_append(st, q);
    Ok(side_effects)
}

/// The current font's interword glue spec, built from its parameters on first use.
fn font_space_glue(st: &mut EngineState) -> Result<i32, crate::scan::NodeRecycleAbort> {
    let cf = st.eqtb.b32s1(CUR_FONT_LOC);
    let glue = st.fonts.font_glue(cf);
    if glue != TEX_NULL {
        return Ok(glue);
    }
    let spec =
        crate::mem::alloc_spec(&mut st.mem, 0).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    st.mainloop.main_k = st.fonts.param_base(cf) + SPACE_CODE;
    let k = st.mainloop.main_k;
    spec.set_width(&mut st.mem, Scaled(st.fonts.font_info_b32s1(k)));
    spec.set_stretch(&mut st.mem, Scaled(st.fonts.font_info_b32s1(k + 1)));
    spec.set_shrink(&mut st.mem, Scaled(st.fonts.font_info_b32s1(k + 2)));
    st.fonts.set_font_glue(cf, spec.idx().0);
    Ok(spec.idx().0)
}

pub(crate) fn app_space_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if let Some((remainder, overflowed)) = app_space(st)? {
        if overflowed {
            st.errors.arith_error = true;
        }
        st.errors.tex_remainder = remainder;
    }
    Ok(())
}

fn get_x_non_blank_non_relax(st: &mut EngineState) -> Result<(), MainControlAbort> {
    loop {
        crate::scan::get_x_token_in(st)?;
        if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
            return Ok(());
        }
    }
}

pub(crate) fn scan_box_in(st: &mut EngineState, box_context: i32) -> Result<(), MainControlAbort> {
    get_x_non_blank_non_relax(st)?;
    if st.scanner.cur_cmd == MAKE_BOX {
        begin_box_in(st, box_context)
    } else if box_context >= LEADER_FLAG
        && (st.scanner.cur_cmd == HRULE || st.scanner.cur_cmd == VRULE)
    {
        st.scanner.cur_box = crate::scan::scan_rule_spec_in(st)?.ptr();
        box_end_in(st, box_context)
    } else {
        crate::output::error_here_with_diagnostic_in(
            st,
            c"A <box> was supposed to be here".as_ptr(),
        );
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[
                c"I was expecting to see \\hbox or \\vbox or \\copy or \\box or",
                c"something like that. So you might find something missing in",
                c"your output. But keep trying; you can fix this later.",
            ],
        );
        mc_propagate_error(crate::input::back_error_in(st)?)
    }
}

pub(crate) fn package_in(st: &mut EngineState, c: i32) -> Result<(), MainControlAbort> {
    let d = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_BOX_MAX_DEPTH);
    let u = st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS);
    crate::save::unsave_in(st)?;
    st.save_stack.save_ptr -= 3;
    let v = st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS);
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS, u);
    let sp = st.save_stack.save_ptr;
    let list = st.mem.link_ptr(st.input.cur_list.head);
    let size = st.save_stack.b32s1(sp + 2);
    let spec_mode = st.save_stack.b32s1(sp + 1) as i16;
    if st.input.cur_list.mode == INNER_HMODE {
        st.scanner.cur_box = crate::packaging::hpack_emit_in(st, list, size, spec_mode)?.ptr();
    } else {
        let b = crate::packaging::vpackage_emit_in(st, list, size, spec_mode, d)?;
        st.scanner.cur_box = b.ptr();
        if c == VTOP_CODE {
            let h = match b.list_ptr(&st.mem).as_idx().map(|p| st.mem.classify(p)) {
                Some(Ok(AnyNode::HList(first))) => first.height(&st.mem).0,
                Some(Ok(AnyNode::VList(first))) => first.height(&st.mem).0,
                Some(Ok(AnyNode::Rule(first))) => first.height(&st.mem).raw(),
                _ => 0,
            };
            let depth = b.depth(&st.mem).0 - h + b.height(&st.mem).0;
            b.set_depth(&mut st.mem, Scaled(depth));
            b.set_height(&mut st.mem, Scaled(h));
        }
    }
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS, v);
    crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
    let box_context = st.save_stack.b32s1(st.save_stack.save_ptr);
    box_end_in(st, box_context)
}

pub(crate) fn new_graf_in(st: &mut EngineState, indented: bool) -> Result<(), MainControlAbort> {
    st.input.cur_list.prev_graf = 0;
    if st.input.cur_list.mode == VMODE || st.input.cur_list.head != st.input.cur_list.tail {
        let g = crate::mem::alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_PAR_SKIP)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
        tail_append(st, g.ptr());
    }
    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = HMODE;
    st.input.cur_list.set_aux_s0(1000);

    let language = st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE);
    let l: u8 = if language <= 0 || language > BIGGEST_LANG {
        0
    } else {
        language as u8
    };
    st.hyph.cur_lang = l;
    st.input.cur_list.set_aux_s1(l as i32);
    let lhm = crate::helpers::norm_min(st.eqtb.b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as i32;
    let rhm = crate::helpers::norm_min(st.eqtb.b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as i32;
    st.input.cur_list.prev_graf = (lhm * 64 + rhm) * 65536 + l as i32;

    if indented {
        let b = crate::mem::alloc_null_box(&mut st.mem)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
        st.input.cur_list.tail = b.ptr();
        st.input.cur_list.head.set_link(&mut st.mem, b.ptr());
        b.set_width(
            &mut st.mem,
            Scaled(st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_PAR_INDENT)),
        );
        if st.shipout.insert_src_special_every_par {
            crate::ext::insert_src_special_in(st)?;
        }
    }
    begin_every(st, LOCAL_EVERY_PAR, EVERY_PAR_TEXT)?;
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
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            NewCharacterAbort::Native(a) => a.into_engine_abort(),
            NewCharacterAbort::Error(outcome) => outcome.into_engine_abort(),
            NewCharacterAbort::MainMemoryOverflow => crate::mem::main_memory_overflow_abort(),
        }
    }
}

pub(crate) enum MainControlAbort {
    /// Cooperative resident-operation cancellation.
    Cancelled,
    /// Viewport horizon reached at a resumable command boundary.
    Horizoned,
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
    /// Serious internal error closing an input file, reached from `load_picture_in`.
    InputClose,
    NativeFont(crate::ext::NativeFontAbort),
}

macro_rules! main_control_abort_from {
    ($($source:ty => $variant:ident),* $(,)?) => {
        $(
            impl From<$source> for MainControlAbort {
                fn from(e: $source) -> Self {
                    MainControlAbort::$variant(e)
                }
            }
        )*
    };
}

main_control_abort_from! {
    crate::ext::NativeFontAbort => NativeFont,
    crate::hyphenation::TrieAbort => Trie,
    NewCharacterAbort => NewCharacter,
    crate::scan::SccAbort => Scanner,
    crate::scan::NodeRecycleAbort => NodeRecycle,
    crate::pagebuilder::PageBuildAbort => PageBuild,
    crate::mem::PushNestAbort => PushNest,
    crate::packaging::VsplitAbort => Vsplit,
    crate::input::InputAbort => Input,
    crate::shipout::ShipoutAbort => Shipout,
    crate::save::SaMutationAbort => SaMutation,
    crate::save::SaveAbort => Save,
    crate::save::UnsaveAbort => Unsave,
    crate::ext::InsertSrcAbort => InsertSrc,
    crate::output::IssueMessageAbort => IssueMessage,
    crate::output::ShowWhateverAbort => ShowWhatever,
    crate::io::OpenLogAbort => OpenLog,
    crate::io::OpenOrCloseAbort => OpenOrClose,
    crate::texmfmp::MakeTexStringAbort => MakeTexString,
    crate::fonts::ReadFontInfoAbort => ReadFontInfo,
}

impl MainControlAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            MainControlAbort::Cancelled => crate::errors::EngineAbort::Cancelled,
            MainControlAbort::Horizoned => crate::errors::EngineAbort::Horizoned,
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

/// Continues on a recoverable error outcome, otherwise aborts with it.
#[inline]
pub(crate) fn mc_propagate_error(o: crate::errors::ErrorOutcome) -> Result<(), MainControlAbort> {
    if matches!(o, crate::errors::ErrorOutcome::Continue) {
        Ok(())
    } else {
        Err(MainControlAbort::Error(o))
    }
}

#[inline]
fn tail_append(st: &mut EngineState, p: MemPtr) {
    st.input.cur_list.tail.set_link(&mut st.mem, p);
    st.input.cur_list.tail = p;
}

/// TeX's `help0`..`help6`: `lines` in reading order.
pub(crate) fn help(errors: &mut crate::state::ErrorState, lines: &[&'static std::ffi::CStr]) {
    errors.help_ptr = lines.len() as u8;
    for (slot, line) in errors.help_line.iter_mut().zip(lines.iter().rev()) {
        *slot = line.as_ptr();
    }
}

pub(crate) fn new_character_in(
    st: &mut EngineState,
    f: i32,
    c: u16,
) -> Result<i32, NewCharacterAbort> {
    if is_native_font(&st.fonts, f) {
        return crate::fonts::new_native_character_in(st, f, c as u32)
            .map(|w| w.ptr().0)
            .map_err(NewCharacterAbort::Native);
    }
    let ec = crate::fonts::effective_char(&mut st.fonts, false, f, c) as u16;
    if st.fonts.font_bc_entry(f) <= ec && st.fonts.font_ec_entry(f) >= ec {
        let info = st
            .fonts
            .font_info_b16(st.fonts.char_base_entry(f) + ec as i32);
        if info.s3 > 0 {
            let p =
                crate::mem::get_avail(&mut st.mem).ok_or(NewCharacterAbort::MainMemoryOverflow)?;
            st.mem.set_b16s1(p, f as u16);
            st.mem.set_b16s0(p, c);
            return Ok(p.0);
        }
    }
    let outcome = crate::output::char_warning_in(st, f, c as i32);
    if !matches!(outcome, crate::errors::ErrorOutcome::Continue) {
        return Err(NewCharacterAbort::Error(outcome));
    }
    Ok(TEX_NULL)
}

pub(crate) fn box_end_in(st: &mut EngineState, box_context: i32) -> Result<(), MainControlAbort> {
    if box_context < BOX_FLAG {
        let cb = st.scanner.cur_box;
        if cb.is_null() {
            return Ok(());
        }
        st.mem.set_b32s1(cb + 4, box_context);
        if st.input.cur_list.mode.abs() == VMODE {
            if !st.scanner.pre_adjust_tail.is_null() {
                if st.scanner.pre_adjust_tail != PRE_ADJUST_HEAD {
                    append_list(st, PRE_ADJUST_HEAD, st.scanner.pre_adjust_tail);
                }
                st.scanner.pre_adjust_tail = MemPtr::NULL;
            }
            let b = st
                .mem
                .classify_box(MemIdx::new(cb))
                .expect("valid node type for append_to_vlist")
                .expect("append_to_vlist receives an hlist or vlist node");
            crate::packaging::append_to_vlist(&mut st.input.cur_list, &mut st.mem, &st.eqtb, b)?;
            if !st.scanner.adjust_tail.is_null() {
                if st.scanner.adjust_tail != ADJUST_HEAD {
                    append_list(st, ADJUST_HEAD, st.scanner.adjust_tail);
                }
                st.scanner.adjust_tail = MemPtr::NULL;
            }
            if st.input.cur_list.mode > 0 {
                crate::pagebuilder::build_page_in(st)?;
            }
        } else {
            if st.input.cur_list.mode.abs() == HMODE {
                st.input.cur_list.set_aux_s0(1000);
            } else {
                let noad = crate::mem::alloc_noad(&mut st.mem)
                    .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                noad.set_nucleus_type(&mut st.mem, SUB_BOX);
                noad.set_nucleus_field(&mut st.mem, cb.0);
                st.scanner.cur_box = noad.ptr();
            }
            tail_append(st, st.scanner.cur_box);
        }
    } else if box_context < SHIP_OUT_FLAG {
        let (n, global) = if box_context < GLOBAL_BOX_FLAG {
            (box_context - BOX_FLAG, false)
        } else {
            (box_context - GLOBAL_BOX_FLAG, true)
        };
        st.scanner.cur_val = n;
        let cb = st.scanner.cur_box.0;
        if n < 256 {
            define(st, global, BOX_BASE + n, BOX_REF, cb)?;
        } else {
            crate::save::find_sa_element(&mut st.mem, &mut st.scanner, BOX_VAL, n, true)
                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
            sa_define(st, global, st.scanner.cur_ptr.0, cb)?;
        }
    } else if !st.scanner.cur_box.is_null() {
        if box_context == SHIP_OUT_FLAG {
            let cb = st.scanner.cur_box.0;
            unsafe { crate::shipout::ship_out_in(st, cb)? };
            return Ok(());
        }
        get_x_non_blank_non_relax(st)?;
        let vertical = st.input.cur_list.mode.abs() == VMODE;
        if (st.scanner.cur_cmd == HSKIP && !vertical) || (st.scanner.cur_cmd == VSKIP && vertical) {
            append_glue_in(st)?;
            let tail = st.input.cur_list.tail;
            st.mem.set_b16s0(
                tail,
                (box_context - (LEADER_FLAG - A_LEADERS as i32)) as u16,
            );
            st.mem.set_b32s1(tail + 1, st.scanner.cur_box.0);
        } else {
            crate::output::error_here_with_diagnostic_in(
                st,
                c"Leaders not followed by proper glue".as_ptr(),
            );
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"You should say `\\leaders <box or rule><hskip or vskip>'.",
                    c"I found the <box or rule>, but there's no suitable",
                    c"<hskip or vskip>, so I'm ignoring these leaders.",
                ],
            );
            mc_propagate_error(crate::input::back_error_in(st)?)?;
            crate::packaging::flush_node_list(&mut st.mem, st.scanner.cur_box.0)
                .map_err(MainControlAbort::Confusion)?;
        }
    }
    Ok(())
}

fn is_math_node(mem: &MemState, p: i32, subtype: u16) -> bool {
    !is_char_node(mem, p)
        && matches!(mem.classify(MemIdx(p)), Ok(AnyNode::Math(m)) if m.subtype(mem) == subtype)
}

/// The last node of the current list, skipping a trailing end-of-math node.
fn effective_tail(st: &EngineState) -> i32 {
    let tail = st.input.cur_list.tail.0;
    if !is_math_node(&st.mem, tail, END_M_CODE) {
        return tail;
    }
    let mut r = st.input.cur_list.head.0;
    loop {
        let next = st.mem.b32s1(r);
        if next == tail {
            return r;
        }
        r = next;
    }
}

/// Unlinks node `tx` from the current list, unless it hides in a
/// discretionary's replacement text. Returns whether it was unlinked.
fn unlink_effective_tail(st: &mut EngineState, tx: i32) -> Result<bool, MainControlAbort> {
    let mut q = st.input.cur_list.head.0;
    let mut p = TEX_NULL;
    let mut r;
    let mut fm;
    loop {
        r = p;
        p = q;
        fm = false;
        if !is_char_node(&st.mem, q) {
            match st.mem.classify(MemIdx(q)) {
                Ok(AnyNode::Disc(disc)) => {
                    for _ in 0..disc.subtype(&st.mem) {
                        p = st.mem.b32s1(p);
                    }
                    if p == tx {
                        return Ok(false);
                    }
                }
                Ok(AnyNode::Math(m)) => fm = m.subtype(&st.mem) == BEGIN_M_CODE,
                _ => {}
            }
        }
        q = st.mem.b32s1(p);
        if q == tx {
            break;
        }
    }
    let next = st.mem.b32s1(tx);
    st.mem.set_b32s1(p, next);
    st.mem.set_b32s1(tx, TEX_NULL);
    if next == TEX_NULL {
        if fm {
            return Err(MainControlAbort::Confusion(c"tail1"));
        }
        st.input.cur_list.tail.0 = p;
    } else if fm {
        st.input.cur_list.tail.0 = r;
        st.mem.set_b32s1(r, TEX_NULL);
        crate::packaging::flush_node_list(&mut st.mem, p).map_err(MainControlAbort::Confusion)?;
    }
    Ok(true)
}

pub(crate) fn begin_box_in(st: &mut EngineState, box_context: i32) -> Result<(), MainControlAbort> {
    match st.scanner.cur_chr {
        BOX_CODE => {
            crate::scan::scan_register_num_in(st)?;
            let n = st.scanner.cur_val;
            st.scanner.cur_box = fetch_box(st, n).map_or(MemPtr::NULL, AnyBox::ptr);
            change_box(st, n, TEX_NULL);
        }
        COPY_CODE => {
            crate::scan::scan_register_num_in(st)?;
            let q = fetch_box(st, st.scanner.cur_val).map_or(TEX_NULL, |b| b.idx().0);
            st.scanner.cur_box.0 = crate::packaging::copy_node_list(&mut st.mem, q)?;
        }
        LAST_BOX_CODE => {
            st.scanner.cur_box = MemPtr::NULL;
            if st.input.cur_list.mode.abs() == MMODE {
                crate::errors::you_cant_in(st);
                help(&mut st.errors, &[c"Sorry; this \\lastbox will be void."]);
                mc_propagate_error(crate::errors::error_in(st))?;
            } else if st.input.cur_list.mode == VMODE
                && st.input.cur_list.head == st.input.cur_list.tail
            {
                crate::errors::you_cant_in(st);
                help(
                    &mut st.errors,
                    &[
                        c"Sorry...I usually can't take things from the current page.",
                        c"This \\lastbox will therefore be void.",
                    ],
                );
                mc_propagate_error(crate::errors::error_in(st))?;
            } else {
                let tx = effective_tail(st);
                if !is_char_node(&st.mem, tx) {
                    if let Ok(Some(b)) = st.mem.classify_box(MemIdx(tx)) {
                        if unlink_effective_tail(st, tx)? {
                            st.scanner.cur_box = b.ptr();
                            b.set_shift_amount(&mut st.mem, Scaled(0));
                        }
                    }
                }
            }
        }
        VSPLIT_CODE => {
            crate::scan::scan_register_num_in(st)?;
            let n = st.scanner.cur_val;
            if !crate::scan::scan_keyword_in(st, c"to".as_ptr())? {
                crate::output::error_here_with_diagnostic_in(st, c"Missing `to' inserted".as_ptr());
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"I'm working on `\\vsplit<box number> to <dimen>';",
                        c"will look for the <dimen> next.",
                    ],
                );
                mc_propagate_error(crate::errors::error_in(st))?;
            }
            crate::scan::scan_dimen_in(st, false, false, false)?;
            st.scanner.cur_box = crate::packaging::vsplit_in(st, n, st.scanner.cur_val)?;
        }
        _ => {
            let k = st.scanner.cur_chr - VTOP_CODE as i32;
            st.save_stack.set_b32s1(st.save_stack.save_ptr, box_context);
            let mode = if k == HMODE as i32 {
                let group = if box_context < BOX_FLAG && st.input.cur_list.mode.abs() == VMODE {
                    GroupCode::AdjustedHbox
                } else {
                    GroupCode::Hbox
                };
                crate::scan::scan_spec_in(st, group, true)?;
                HMODE
            } else {
                let group = if k == VMODE as i32 {
                    GroupCode::Vbox
                } else {
                    GroupCode::Vtop
                };
                crate::scan::scan_spec_in(st, group, true)?;
                normal_paragraph(st)?;
                VMODE
            };
            crate::mem::push_nest_in(st)?;
            st.input.cur_list.mode = -mode;
            if mode == VMODE {
                st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
                begin_every(st, LOCAL_EVERY_VBOX, EVERY_VBOX_TEXT)?;
            } else {
                st.input.cur_list.set_aux_s0(1000);
                begin_every(st, LOCAL_EVERY_HBOX, EVERY_HBOX_TEXT)?;
            }
            return Ok(());
        }
    }
    box_end_in(st, box_context)
}

pub(crate) fn handle_right_brace_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    use crate::mem::{alloc_noad, delete_glue_ref, free_node, pop_nest_in};
    use crate::packaging::flush_node_list;
    use crate::scan::NodeRecycleAbort;

    match st.save_stack.cur_group {
        GroupCode::Simple => crate::save::unsave_in(st)?,
        GroupCode::BottomLevel => {
            crate::output::error_here_with_diagnostic_in(st, c"Too many }'s".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"You've closed more groups than you opened.",
                    c"Such booboos are generally harmless, so keep going.",
                ],
            );
            mc_propagate_error(crate::errors::error_in(st))?;
        }
        GroupCode::SemiSimple | GroupCode::MathShift | GroupCode::MathLeft => {
            mc_propagate_error(crate::errors::extra_right_brace_in(st))?;
        }
        GroupCode::Hbox => package_in(st, 0)?,
        GroupCode::AdjustedHbox => {
            st.scanner.adjust_tail = ADJUST_HEAD;
            st.scanner.pre_adjust_tail = PRE_ADJUST_HEAD;
            package_in(st, 0)?;
        }
        GroupCode::Vbox => {
            end_graf_in(st)?;
            package_in(st, 0)?;
        }
        GroupCode::Vtop => {
            end_graf_in(st)?;
            package_in(st, VTOP_CODE)?;
        }
        GroupCode::Insert => {
            end_graf_in(st)?;
            let split_top_skip = st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_SPLIT_TOP_SKIP as i32);
            crate::mem::add_glue_ref(&mut st.mem, split_top_skip);
            let d = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_SPLIT_MAX_DEPTH);
            let f = st.eqtb.b32s1(INT_BASE + INT_PAR_FLOATING_PENALTY);
            crate::save::unsave_in(st)?;
            st.save_stack.save_ptr -= 2;
            let head = st.mem.link_ptr(st.input.cur_list.head);
            let p = crate::packaging::vpackage_emit_in(st, head, 0, ADDITIONAL, MAX_HALFWORD)?;
            pop_nest_in(&mut st.mem, &mut st.input);
            let n = st.save_stack.b32s1(st.save_stack.save_ptr);
            if n < 255 {
                let ins =
                    NodePtr::<InsNode>::alloc(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
                tail_append(st, ins.ptr());
                ins.set_subtype(&mut st.mem, n as u16);
                let height = p.height(&st.mem).0 + p.depth(&st.mem).0;
                ins.set_height(&mut st.mem, Scaled(height));
                let list = p.list_ptr(&st.mem);
                ins.set_ins_ptr(&mut st.mem, list);
                ins.set_split_top_ptr(&mut st.mem, MemPtr::from_raw(split_top_skip));
                ins.set_depth(&mut st.mem, Scaled(d));
                ins.set_float_cost(&mut st.mem, f);
            } else {
                let adj =
                    NodePtr::<AdjustNode>::alloc(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
                tail_append(st, adj.ptr());
                let pre = st.save_stack.b32s1(st.save_stack.save_ptr + 1);
                adj.set_subtype(&mut st.mem, pre as u16);
                let list = p.list_ptr(&st.mem);
                adj.set_adjust_ptr(&mut st.mem, list);
                delete_glue_ref(&mut st.mem, split_top_skip);
            }
            free_node(&mut st.mem, p.ptr(), BOX_NODE_SIZE);
            if st.input.nest_ptr == 0 {
                crate::pagebuilder::build_page_in(st)?;
            }
        }
        GroupCode::Output => {
            if st.input.cur_input.loc != TEX_NULL
                || (st.input.cur_input.index != OUTPUT_TEXT
                    && st.input.cur_input.index != BACKED_UP)
            {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Unbalanced output routine".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"Your sneaky output routine has problematic {'s and/or }'s.",
                        c"I can't handle that very well; good luck.",
                    ],
                );
                mc_propagate_error(crate::errors::error_in(st))?;
                loop {
                    crate::scan::get_token_in(st)?;
                    if st.input.cur_input.loc == TEX_NULL {
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
                help(
                    &mut st.errors,
                    &[
                        c"Your \\output commands should empty \\box255,",
                        c"e.g., by saying `\\shipout\\box255'.",
                        c"Proceed; I'll discard its present contents.",
                    ],
                );
                mc_propagate_error(crate::errors::box_error_in(st, 255))?;
            }

            if st.input.cur_list.tail != st.input.cur_list.head {
                let first = st.input.cur_list.head.link(&st.mem);
                st.shipout.page_tail.set_link(&mut st.mem, first);
                st.shipout.page_tail = st.input.cur_list.tail;
            }
            if !st.mem.link_ptr(PAGE_HEAD).is_null() {
                if st.mem.link_ptr(CONTRIB_HEAD).is_null() {
                    st.input.nest[0].tail = st.shipout.page_tail;
                }
                let contrib = st.mem.link_ptr(CONTRIB_HEAD);
                st.shipout.page_tail.set_link(&mut st.mem, contrib);
                st.mem.set_b32s1(CONTRIB_HEAD, st.mem.b32s1(PAGE_HEAD));
                st.mem.set_b32s1(PAGE_HEAD, TEX_NULL);
                st.shipout.page_tail = PAGE_HEAD;
            }

            let last_box = st.scanner.disc_ptr[LAST_BOX_CODE as usize];
            flush_node_list(&mut st.mem, last_box.0).map_err(MainControlAbort::Confusion)?;
            st.scanner.disc_ptr[LAST_BOX_CODE as usize] = MemPtr::NULL;
            pop_nest_in(&mut st.mem, &mut st.input);
            crate::pagebuilder::build_page_in(st)?;
        }
        GroupCode::Disc => build_discretionary_in(st)?,
        GroupCode::Align => {
            crate::input::back_input_in(st)?;
            st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_CR;
            crate::output::error_here_with_diagnostic_in(st, c"Missing ".as_ptr());
            crate::output::print_esc_cstr_in(st, c"cr".as_ptr());
            crate::output::print_cstr_in(st, c" inserted".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[c"I'm guessing that you meant to end an alignment here."],
            );
            mc_propagate_error(crate::input::ins_error_in(st)?)?;
        }
        GroupCode::NoAlign => {
            end_graf_in(st)?;
            crate::save::unsave_in(st)?;
            align_peek_in(st)?;
        }
        GroupCode::Vcenter => {
            end_graf_in(st)?;
            crate::save::unsave_in(st)?;
            st.save_stack.save_ptr -= 2;
            let head = st.mem.link_ptr(st.input.cur_list.head);
            let size = st.save_stack.b32s1(st.save_stack.save_ptr + 1);
            let spec_mode = st.save_stack.b32s1(st.save_stack.save_ptr) as i16;
            let p = crate::packaging::vpackage_emit_in(st, head, size, spec_mode, MAX_HALFWORD)?;
            pop_nest_in(&mut st.mem, &mut st.input);
            let noad = alloc_noad(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
            tail_append(st, noad.ptr());
            noad.set_noad_type(&mut st.mem, NodeType::VcenterNoad);
            noad.set_nucleus_type(&mut st.mem, SUB_BOX);
            noad.set_nucleus_field(&mut st.mem, p.idx().0);
        }
        GroupCode::MathChoice => crate::math::build_choices_in(st)?,
        GroupCode::Math => {
            crate::save::unsave_in(st)?;
            st.save_stack.save_ptr -= 1;
            let save_loc = st.save_stack.b32s1(st.save_stack.save_ptr);
            st.mem.set_b32s1(save_loc, SUB_MLIST);
            let p = crate::math::fin_mlist_in(st, TEX_NULL)?;
            st.mem.set_b32s0(save_loc, p);
            if p == TEX_NULL || st.mem.b32s1(p) != TEX_NULL {
                return Ok(());
            }
            let Ok(AnyNode::Noad(noad)) = st.mem.classify(MemIdx(p)) else {
                return Ok(());
            };
            match noad.noad_type(&st.mem) {
                NodeType::OrdNoad
                    if noad.subscr_type(&st.mem) == EMPTY && noad.supscr_type(&st.mem) == EMPTY =>
                {
                    st.mem.set_word(save_loc, st.mem.word(noad.nucleus_addr()));
                    free_node(&mut st.mem, noad.ptr(), NOAD_SIZE);
                }
                NodeType::AccentNoad
                    if save_loc == st.input.cur_list.tail.0 + 1
                        && matches!(
                            st.mem.classify(MemIdx(st.input.cur_list.tail.0)),
                            Ok(AnyNode::Noad(t)) if t.noad_type(&st.mem) == NodeType::OrdNoad
                        ) =>
                {
                    let mut q = st.input.cur_list.head;
                    while q.link(&st.mem) != st.input.cur_list.tail {
                        q = q.link(&st.mem);
                    }
                    q.set_link(&mut st.mem, noad.ptr());
                    free_node(&mut st.mem, st.input.cur_list.tail, NOAD_SIZE);
                    st.input.cur_list.tail = noad.ptr();
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub(crate) fn append_glue_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let s = st.scanner.cur_chr;
    match s {
        0 => st.scanner.cur_val = FIL_GLUE.0,
        1 => st.scanner.cur_val = FILL_GLUE.0,
        2 => st.scanner.cur_val = SS_GLUE.0,
        3 => st.scanner.cur_val = FIL_NEG_GLUE.0,
        SKIP_CODE => crate::scan::scan_glue_in(st, GLUE_VAL)?,
        MSKIP_CODE => crate::scan::scan_glue_in(st, MU_VAL)?,
        _ => {}
    }
    let g = crate::mem::alloc_glue(&mut st.mem, st.scanner.cur_val)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    tail_append(st, g.ptr());
    if s >= SKIP_CODE {
        let spec = g.glue_spec(&st.mem).expect("non-null glue spec");
        let refs = spec.ref_count(&st.mem);
        spec.set_ref_count(&mut st.mem, refs - 1);
        if s > SKIP_CODE {
            g.set_subtype(&mut st.mem, MU_GLUE);
        }
    }
    Ok(())
}

pub(crate) fn append_kern_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let s = st.scanner.cur_chr as u16;
    crate::scan::scan_dimen_in(st, s == MU_GLUE, false, false)?;
    let p = crate::mem::alloc_kern(&mut st.mem, st.scanner.cur_val)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    p.set_subtype(&mut st.mem, s);
    tail_append(st, p.ptr());
    Ok(())
}

pub(crate) fn make_mark_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let c = if st.scanner.cur_chr == 0 {
        0
    } else {
        crate::scan::scan_register_num_in(st)?;
        st.scanner.cur_val
    };
    crate::scan::scan_toks_in(st, false, true)?;
    let mark = overflow_err(NodePtr::<MarkNode>::alloc(&mut st.mem))?;
    mark.set_mark_class(&mut st.mem, c);
    mark.set_subtype(&mut st.mem, 0);
    mark.set_mark_ptr(&mut st.mem, st.scanner.def_ref);
    tail_append(st, mark.ptr());
    Ok(())
}

pub(crate) fn new_interaction_in(st: &mut EngineState) {
    crate::output::print_ln_in(st);
    st.errors.interaction = crate::state::InteractionMode::from_raw(st.scanner.cur_chr)
        .expect("interaction primitive chr is 0..=3");
    st.shipout.selector = if st.errors.interaction == crate::state::InteractionMode::Batch {
        SELECTOR_NO_PRINT
    } else {
        SELECTOR_TERM_ONLY
    };
    if st.errors.log_opened {
        st.shipout.selector += 2;
    }
}

pub(crate) fn alter_aux_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.cur_chr != st.input.cur_list.mode.abs() as i32 {
        return mc_propagate_error(crate::errors::report_illegal_case_in(st));
    }
    let c = st.scanner.cur_chr;
    crate::scan::scan_optional_equals_in(st)?;
    if c == VMODE as i32 {
        crate::scan::scan_dimen_in(st, false, false, false)?;
        st.input.cur_list.set_aux_s1(st.scanner.cur_val);
        return Ok(());
    }
    crate::scan::scan_int_in(st)?;
    if st.scanner.cur_val <= 0 || st.scanner.cur_val > 32767 {
        let errmsg = crate::output::error_here_with_diagnostic_in(st, c"Bad space factor".as_ptr());
        unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val)) };
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[c"I allow only values in the range 1..32767 here."],
        );
        return mc_propagate_error(crate::errors::int_error_in(st, st.scanner.cur_val));
    }
    st.input.cur_list.set_aux_s0(st.scanner.cur_val);
    Ok(())
}

pub(crate) fn alter_prev_graf_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let np = st.input.nest_ptr as usize;
    st.input.nest[np] = st.input.cur_list;
    let mut p = np;
    while st.input.nest[p].mode.abs() != VMODE {
        p -= 1;
    }
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_int_in(st)?;
    if st.scanner.cur_val < 0 {
        let errmsg = crate::output::error_here_with_diagnostic_in(st, c"Bad ".as_ptr());
        unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val)) };
        crate::output::print_esc_cstr_in(st, c"prevgraf".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(&mut st.errors, &[c"I allow only nonnegative values here."]);
        return mc_propagate_error(crate::errors::int_error_in(st, st.scanner.cur_val));
    }
    st.input.nest[p].prev_graf = st.scanner.cur_val;
    st.input.cur_list = st.input.nest[np];
    Ok(())
}

pub(crate) fn alter_page_so_far_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let c = st.scanner.cur_chr as usize;
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_dimen_in(st, false, false, false)?;
    st.shipout.page_so_far[c] = st.scanner.cur_val;
    Ok(())
}

pub(crate) fn alter_integer_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let c = st.scanner.cur_chr;
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_int_in(st)?;
    match c {
        0 => st.shipout.dead_cycles = st.scanner.cur_val,
        2 => {
            if crate::state::InteractionMode::from_raw(st.scanner.cur_val).is_none() {
                let errmsg = crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Bad interaction mode".as_ptr(),
                );
                unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val)) };
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"Modes are 0=batch, 1=nonstop, 2=scroll, and",
                        c"3=errorstop. Proceed, and I'll ignore this case.",
                    ],
                );
                return mc_propagate_error(crate::errors::int_error_in(st, st.scanner.cur_val));
            }
            st.scanner.cur_chr = st.scanner.cur_val;
            new_interaction_in(st);
        }
        _ => st.shipout.insert_penalties = st.scanner.cur_val,
    }
    Ok(())
}

pub(crate) fn alter_box_dimen_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let c = st.scanner.cur_chr;
    crate::scan::scan_register_num_in(st)?;
    let b = fetch_box(st, st.scanner.cur_val);
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_dimen_in(st, false, false, false)?;
    if let Some(b) = b {
        st.mem.set_b32s1(b.idx().offset(c), st.scanner.cur_val);
    }
    Ok(())
}

/// Replaces an all-zero scanned glue spec in `cur_val` by the shared `zero_glue`.
pub(crate) fn trap_zero_glue(st: &mut EngineState) {
    let spec = st.scanner.cur_val;
    if st.mem.b32s1(spec + 1) == 0 && st.mem.b32s1(spec + 2) == 0 && st.mem.b32s1(spec + 3) == 0 {
        crate::mem::add_glue_ref(&mut st.mem, ZERO_GLUE);
        crate::mem::delete_glue_ref(&mut st.mem, spec);
        st.scanner.cur_val = ZERO_GLUE.0;
    }
}

pub(crate) fn fix_language(
    cl: &mut crate::mem_types::ListStateRecord,
    mem: &mut MemState,
    eqtb: &crate::state::EqtbState,
) -> Result<(), crate::scan::NodeRecycleAbort> {
    let language = eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE);
    let l = if language <= 0 || language > BIGGEST_LANG {
        0
    } else {
        language
    };
    if l == cl.aux_s1() {
        return Ok(());
    }
    let w = crate::mem::new_whatsit(cl, mem, WhatsitSubtype::Language as i16, SMALL_NODE_SIZE)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    w.set_what_lang(mem, l);
    cl.set_aux_s1(l);
    let lhm = crate::helpers::norm_min(eqtb.b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN)) as u16;
    w.set_what_lhm(mem, lhm);
    let rhm = crate::helpers::norm_min(eqtb.b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN)) as u16;
    w.set_what_rhm(mem, rhm);
    Ok(())
}

pub(crate) fn off_save_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.save_stack.cur_group == GroupCode::BottomLevel {
        crate::output::error_here_with_diagnostic_in(st, c"Extra ".as_ptr());
        crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd, st.scanner.cur_chr);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[c"Things are pretty mixed up, but I think the worst is over."],
        );
        return mc_propagate_error(crate::errors::error_in(st));
    }
    crate::input::back_input_in(st)?;
    let p = crate::scan::get_avail_scc(st)?;
    st.mem.set_b32s1(TEMP_HEAD, p.0);
    crate::output::error_here_with_diagnostic_in(st, c"Missing ".as_ptr());
    match st.save_stack.cur_group {
        GroupCode::SemiSimple => {
            p.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_END_GROUP);
            crate::output::print_esc_cstr_in(st, c"endgroup".as_ptr());
        }
        GroupCode::MathShift => {
            p.set_info(&mut st.mem, MATH_SHIFT_TOKEN + b'$' as i32);
            crate::output::print_char_in(st, b'$' as i32);
        }
        GroupCode::MathLeft => {
            p.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_RIGHT);
            let q = crate::scan::get_avail_scc(st)?;
            p.set_link(&mut st.mem, q);
            q.set_info(&mut st.mem, OTHER_TOKEN + b'.' as i32);
            crate::output::print_esc_cstr_in(st, c"right.".as_ptr());
        }
        _ => {
            p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
            crate::output::print_char_in(st, b'}' as i32);
        }
    }
    crate::output::print_cstr_in(st, c" inserted".as_ptr());
    crate::input::begin_token_list_in(st, st.mem.b32s1(TEMP_HEAD), INSERTED)?;
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    help(
        &mut st.errors,
        &[
            c"I've inserted something that you may have forgotten.",
            c"(See the <inserted text> above.)",
            c"With luck, this will get me unwedged. But if you",
            c"really didn't forget anything, try typing `2' now; then",
            c"my insertion and my current dilemma will both disappear.",
        ],
    );
    mc_propagate_error(crate::errors::error_in(st))
}

fn begin_disc_group(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::save::new_save_level(&mut st.save_stack, GroupCode::Disc, st.mem.line)?;
    crate::scan::scan_left_brace_in(st)?;
    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = INNER_HMODE;
    st.input.cur_list.set_aux_s0(1000);
    Ok(())
}

pub(crate) fn append_discretionary_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let d = crate::mem::alloc_disc(&mut st.mem).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    tail_append(st, d.ptr());
    if st.scanner.cur_chr == 1 {
        let f = st.eqtb.b32s1(CUR_FONT_LOC);
        let c = st.fonts.hyphen_char[f as usize];
        if (0..=BIGGEST_CHAR).contains(&c) {
            let ch = new_character_in(st, f, c as u16)?;
            d.set_pre_break(&mut st.mem, MemPtr::from_raw(ch));
        }
        return Ok(());
    }
    st.save_stack.set_b32s1(st.save_stack.save_ptr, 0);
    st.save_stack.save_ptr += 1;
    begin_disc_group(st)
}

pub(crate) fn shift_case_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let b = st.scanner.cur_chr;
    crate::scan::scan_toks_in(st, false, false)?;
    let mut p = st.mem.b32s1(st.scanner.def_ref);
    while p != TEX_NULL {
        let t = st.mem.b32s0(p);
        if t < CS_TOKEN_FLAG + SINGLE_BASE {
            let c = t % MAX_CHAR_VAL;
            let mapped = st.eqtb.b32s1(b + c);
            if mapped != 0 {
                st.mem.set_b32s0(p, t - c + mapped);
            }
        }
        p = st.mem.b32s1(p);
    }
    let def_ref = st.scanner.def_ref;
    crate::input::begin_token_list_in(st, st.mem.b32s1(def_ref), BACKED_UP)?;
    crate::mem::free_avail(&mut st.mem, def_ref);
    Ok(())
}

pub(crate) fn build_discretionary_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::save::unsave_in(st)?;
    let head = st.input.cur_list.head.0;
    let mut q = head;
    let mut p = st.mem.b32s1(q);
    let mut n: i32 = 0;
    while p != TEX_NULL {
        if !is_char_node(&st.mem, p) {
            let allowed = match st.mem.classify(MemIdx(p)) {
                Ok(
                    AnyNode::HList(_)
                    | AnyNode::VList(_)
                    | AnyNode::Rule(_)
                    | AnyNode::Kern(_)
                    | AnyNode::Ligature(_),
                ) => true,
                Ok(AnyNode::Whatsit(w)) => w
                    .whatsit_subtype(&st.mem)
                    .is_some_and(|s| s.is_native_word() || s == WhatsitSubtype::Glyph),
                _ => false,
            };
            if !allowed {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Improper discretionary list".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[c"Discretionary lists must contain only boxes and kerns."],
                );
                mc_propagate_error(crate::errors::error_in(st))?;
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
                break;
            }
        }
        q = p;
        p = st.mem.b32s1(q);
        n += 1;
    }

    p = st.mem.b32s1(head);
    crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
    let tail = st.input.cur_list.tail;
    let part = st.save_stack.b32s1(st.save_stack.save_ptr - 1);
    match part {
        0 => st.mem.set_b32s0(tail + 1, p),
        1 => st.mem.set_b32s1(tail + 1, p),
        2 => {
            if n > 0 && st.input.cur_list.mode.abs() == MMODE {
                crate::output::error_here_with_diagnostic_in(st, c"Illegal math ".as_ptr());
                crate::output::print_esc_cstr_in(st, c"discretionary".as_ptr());
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"Sorry: The third part of a discretionary break must be",
                        c"empty, in math formulas. I had to delete your third part.",
                    ],
                );
                crate::packaging::flush_node_list(&mut st.mem, p)
                    .map_err(MainControlAbort::Confusion)?;
                n = 0;
                mc_propagate_error(crate::errors::error_in(st))?;
            } else {
                st.mem.set_b32s1(tail, p);
            }
            if n <= u16::MAX as i32 {
                st.mem.set_b16s0(tail, n as u16);
            } else {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Discretionary list is too long".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"Wow---I never thought anybody would tweak me here.",
                        c"You can't seriously need such a huge discretionary list?",
                    ],
                );
                mc_propagate_error(crate::errors::error_in(st))?;
            }
            if n > 0 {
                st.input.cur_list.tail.0 = q;
            }
            st.save_stack.save_ptr -= 1;
            return Ok(());
        }
        _ => {}
    }
    st.save_stack
        .set_b32s1(st.save_stack.save_ptr - 1, part + 1);
    begin_disc_group(st)
}

/// The italic correction of the character at `p` (a char node or a ligature's char word).
fn char_italic(st: &mut EngineState, p: i32) -> i32 {
    let f = st.mem.b16s1(p) as i32;
    let c = crate::fonts::effective_char(&mut st.fonts, true, f, st.mem.b16s0(p));
    let info = st.fonts.font_info_b16(st.fonts.char_base_entry(f) + c);
    st.fonts
        .font_info_b32s1(st.fonts.italic_base_entry(f) + (info.s1 / 4) as i32)
}

pub(crate) fn append_italic_correction_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let tail = st.input.cur_list.tail.0;
    if tail == st.input.cur_list.head.0 {
        return Ok(());
    }
    let italic = if is_char_node(&st.mem, tail) {
        char_italic(st, tail)
    } else {
        match st.mem.classify(MemIdx(tail)) {
            Ok(AnyNode::Ligature(lig)) => char_italic(st, lig.idx().0 + 1),
            Ok(AnyNode::Whatsit(w)) => match w.whatsit_subtype(&st.mem) {
                Some(s) if s.is_native_word() => unsafe {
                    crate::ext::real_get_native_italic_correction(&st.fonts, &st.mem, w)
                },
                Some(WhatsitSubtype::Glyph) => unsafe {
                    crate::ext::real_get_native_glyph_italic_correction(&st.fonts, &st.mem, w)
                },
                _ => return Ok(()),
            },
            _ => return Ok(()),
        }
    };
    let kern = crate::mem::alloc_kern(&mut st.mem, italic)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    tail_append(st, kern.ptr());
    kern.set_subtype(&mut st.mem, EXPLICIT);
    Ok(())
}

fn is_native_font(fonts: &crate::state::FontState, f: i32) -> bool {
    let area = fonts.font_area_entry(f);
    area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG
}

pub(crate) fn make_accent_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::scan::scan_char_num_in(st)?;
    let f = st.eqtb.b32s1(CUR_FONT_LOC);
    let p = new_character_in(st, f, st.scanner.cur_val as u16)?;
    if p == TEX_NULL {
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
    let a = if is_native_font(&st.fonts, f) {
        let a = st.mem.b32s1(p + 1);
        if a == 0 {
            (lsb, rsb) = unsafe {
                crate::ext::get_native_char_sidebearings(&st.fonts, f, st.scanner.cur_val)?
            };
        }
        a
    } else {
        let c = crate::fonts::effective_char(&mut st.fonts, true, f, st.mem.b16s0(p));
        let info = st.fonts.font_info_b16(st.fonts.char_base_entry(f) + c);
        st.fonts
            .font_info_b32s1(st.fonts.width_base_entry(f) + info.s3 as i32)
    };

    do_assignments_in(st)?;
    let f = st.eqtb.b32s1(CUR_FONT_LOC);
    let q = match st.scanner.cur_cmd {
        LETTER | OTHER_CHAR | CHAR_GIVEN => {
            let q = new_character_in(st, f, st.scanner.cur_chr as u16)?;
            st.scanner.cur_val = st.scanner.cur_chr;
            q
        }
        CHAR_NUM => {
            crate::scan::scan_char_num_in(st)?;
            new_character_in(st, f, st.scanner.cur_val as u16)?
        }
        _ => {
            crate::input::back_input_in(st)?;
            TEX_NULL
        }
    };
    if q == TEX_NULL {
        tail_append(st, MemPtr::from_raw(p));
        st.input.cur_list.set_aux_s0(1000);
        return Ok(());
    }

    let t = st
        .fonts
        .font_info_b32s1(SLANT_CODE + st.fonts.param_base(f)) as f64
        / 65536.0;
    let (w, h) = if is_native_font(&st.fonts, f) {
        let w = st.mem.b32s1(q + 1);
        let (h, _) =
            unsafe { crate::ext::get_native_char_height_depth(&st.fonts, f, st.scanner.cur_val)? };
        (w, h)
    } else {
        let c = crate::fonts::effective_char(&mut st.fonts, true, f, st.mem.b16s0(q));
        let info = st.fonts.font_info_b16(st.fonts.char_base_entry(f) + c);
        let w = st
            .fonts
            .font_info_b32s1(st.fonts.width_base_entry(f) + info.s3 as i32);
        let h = st
            .fonts
            .font_info_b32s1(st.fonts.height_base_entry(f) + (info.s2 / 16) as i32);
        (w, h)
    };
    let accent = if h == x {
        MemPtr::from_raw(p)
    } else {
        let b = crate::packaging::hpack_emit_in(st, MemPtr::from_raw(p), 0, ADDITIONAL)?;
        b.set_shift_amount(&mut st.mem, Scaled(x - h));
        b.ptr()
    };
    let delta = if is_native_font(&st.fonts, f) && a == 0 {
        crate::scaled_math::tex_round((w - lsb + rsb) as f64 / 2.0 + h as f64 * t - x as f64 * s)
    } else {
        crate::scaled_math::tex_round((w - a) as f64 / 2.0 + h as f64 * t - x as f64 * s)
    };
    let r = crate::mem::alloc_kern(&mut st.mem, delta)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    r.set_subtype(&mut st.mem, ACC_KERN);
    st.input.cur_list.tail.set_link(&mut st.mem, r.ptr());
    r.set_link(&mut st.mem, accent);
    let back = crate::mem::alloc_kern(&mut st.mem, -a - delta)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    st.input.cur_list.tail = back.ptr();
    back.set_subtype(&mut st.mem, ACC_KERN);
    accent.set_link(&mut st.mem, back.ptr());
    tail_append(st, MemPtr::from_raw(q));
    st.input.cur_list.set_aux_s0(1000);
    Ok(())
}

fn string_overflow(st: &EngineState, e: crate::scan::NodeRecycleAbort) -> MainControlAbort {
    match e {
        crate::scan::NodeRecycleAbort::Overflow => {
            MainControlAbort::Input(crate::input::InputAbort::Overflow {
                what: c"pool size".as_ptr(),
                n: st.strings.pool_size - st.strings.init_pool_ptr,
            })
        }
        e => MainControlAbort::NodeRecycle(e),
    }
}

fn make_string_in(st: &mut EngineState) -> Result<i32, MainControlAbort> {
    crate::string_pool::make_string(&mut st.strings).map_err(|e| match e {
        crate::scan::NodeRecycleAbort::Overflow => {
            MainControlAbort::Input(crate::input::InputAbort::Overflow {
                what: c"number of strings".as_ptr(),
                n: st.strings.max_strings - st.strings.init_str_ptr,
            })
        }
        e => MainControlAbort::NodeRecycle(e),
    })
}

fn flush_last_string(st: &mut EngineState) {
    st.strings.str_ptr -= 1;
    st.strings.pool_ptr = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
}

pub(crate) unsafe fn new_font_in(st: &mut EngineState, a: i16) -> Result<(), MainControlAbort> {
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
        make_string_in(st)?
    };

    let global = a >= 4;
    define(st, global, u, SET_FONT, FONT_BASE)?;
    crate::scan::scan_optional_equals_in(st)?;
    crate::io::scan_file_name_in(st)?;
    st.scanner.name_in_progress = true;
    let s = if crate::scan::scan_keyword_in(st, c"at".as_ptr())? {
        crate::scan::scan_dimen_in(st, false, false, false)?;
        let size = st.scanner.cur_val;
        if size <= 0 || size >= 0x800_0000 {
            crate::output::error_here_with_diagnostic_in(st, c"Improper `at' size (".as_ptr());
            crate::output::print_scaled_in(st, size);
            crate::output::print_cstr_in(st, c"pt), replaced by 10pt".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[
                    c"I can only handle fonts at positive sizes that are",
                    c"less than 2048pt, so I've changed what you said to 10pt.",
                ],
            );
            mc_propagate_error(crate::errors::error_in(st))?;
            10 * 65536
        } else {
            size
        }
    } else if crate::scan::scan_keyword_in(st, c"scaled".as_ptr())? {
        crate::scan::scan_int_in(st)?;
        let mag = st.scanner.cur_val;
        if mag <= 0 || mag > 32768 {
            let errmsg = crate::output::error_here_with_diagnostic_in(
                st,
                c"Illegal magnification has been changed to 1000".as_ptr(),
            );
            diag_append_str(errmsg, &format!(" ({})", mag));
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[c"The magnification ratio must be between 1 and 32768."],
            );
            mc_propagate_error(crate::errors::int_error_in(st, mag))?;
            -1000
        } else {
            -mag
        }
    } else {
        -1000
    };
    st.scanner.name_in_progress = false;

    let size_matches = |st: &mut EngineState, f: usize| -> bool {
        if s > 0 {
            s == st.fonts.font_size[f]
        } else {
            st.fonts.font_size[f]
                == crate::scaled_math::xn_over_d_in(
                    &mut st.errors,
                    st.fonts.font_dsize[f],
                    -s,
                    1000,
                )
        }
    };
    let f = 'search: {
        for f in FONT_BASE + 1..=st.fonts.font_ptr {
            let fi = f as usize;
            let area_matches = (crate::string_pool::length(&st.strings, st.scanner.cur_area) == 0
                && is_native_font(&st.fonts, f))
                || crate::string_pool::str_eq_str(
                    &st.strings,
                    st.fonts.font_area[fi],
                    st.scanner.cur_area,
                );
            if crate::string_pool::str_eq_str(
                &st.strings,
                st.fonts.font_name[fi],
                st.scanner.cur_name,
            ) && area_matches
                && size_matches(st, fi)
            {
                break 'search f;
            }
            for part in [st.scanner.cur_area, st.scanner.cur_name, st.scanner.cur_ext] {
                crate::string_pool::append_str(&mut st.strings, part)
                    .map_err(|e| string_overflow(st, e))?;
            }
            let full_name = make_string_in(st)?;
            let same_name =
                crate::string_pool::str_eq_str(&st.strings, st.fonts.font_name[fi], full_name);
            flush_last_string(st);
            if same_name && is_native_font(&st.fonts, f) && size_matches(st, fi) {
                break 'search f;
            }
        }
        read_font_info_in(st, u, st.scanner.cur_name, st.scanner.cur_area, s)?
    };

    define(st, global, u, SET_FONT, f)?;
    let word = st.eqtb.word(u);
    st.eqtb.set_word(FONT_ID_BASE + f, word);
    st.eqtb.set_hash_s1(FONT_ID_BASE + f, t);
    Ok(())
}

pub(crate) fn do_register_command_in(st: &mut EngineState, a: i16) -> Result<(), MainControlAbort> {
    use crate::scaled_math::{mult_and_add_in, x_over_n_in};

    let q = st.scanner.cur_cmd;
    let (l, p, e) = 'found: {
        if q != REGISTER {
            crate::scan::get_x_token_in(st)?;
            if (ASSIGN_INT..=ASSIGN_MU_GLUE).contains(&st.scanner.cur_cmd) {
                break 'found (st.scanner.cur_chr, (st.scanner.cur_cmd - ASSIGN_INT), false);
            }
            if st.scanner.cur_cmd != REGISTER {
                crate::output::error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
                crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd, st.scanner.cur_chr);
                crate::output::print_cstr_in(st, c"' after ".as_ptr());
                crate::output::print_cmd_chr_in(st, q, 0);
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[c"I'm forgetting what you said and not changing anything."],
                );
                return mc_propagate_error(crate::errors::error_in(st));
            }
        }
        if !(0..=19).contains(&st.scanner.cur_chr) {
            let l = st.scanner.cur_chr;
            break 'found (l, (st.mem.b16s1(l) / 64) as u8, true);
        }
        let p = st.scanner.cur_chr as u8;
        crate::scan::scan_register_num_in(st)?;
        let n = st.scanner.cur_val;
        if n > 255 {
            crate::save::find_sa_element(&mut st.mem, &mut st.scanner, p, n, true);
            break 'found (st.scanner.cur_ptr.0, p, true);
        }
        let base = match p {
            INT_VAL => COUNT_BASE,
            DIMEN_VAL => SCALED_BASE,
            GLUE_VAL => SKIP_BASE,
            MU_VAL => MU_SKIP_BASE,
            _ => unreachable!("register value level"),
        };
        (n + base, p, false)
    };

    let (w, s) = match (p < GLUE_VAL, e) {
        (true, true) => (st.mem.b32s1(l + 2), TEX_NULL),
        (true, false) => (st.eqtb.b32s1(l), TEX_NULL),
        (false, true) => (0, st.mem.b32s1(l + 1)),
        (false, false) => (0, st.eqtb.b32s1(l)),
    };

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
                st.scanner.cur_val += w;
            }
        } else {
            crate::scan::scan_glue_in(st, p)?;
            if q == ADVANCE {
                let sum = crate::mem::alloc_spec(&mut st.mem, st.scanner.cur_val)
                    .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                crate::mem::delete_glue_ref(&mut st.mem, st.scanner.cur_val);
                let old = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(s))
                    .expect("non-null glue spec");
                let width = sum.width(&st.mem).0 + old.width(&st.mem).0;
                sum.set_width(&mut st.mem, Scaled(width));
                if sum.stretch(&st.mem).0 == 0 {
                    sum.set_stretch_order(&mut st.mem, NORMAL);
                }
                if sum.stretch_order(&st.mem) == old.stretch_order(&st.mem) {
                    let v = sum.stretch(&st.mem).0 + old.stretch(&st.mem).0;
                    sum.set_stretch(&mut st.mem, Scaled(v));
                } else if sum.stretch_order(&st.mem) < old.stretch_order(&st.mem)
                    && old.stretch(&st.mem).0 != 0
                {
                    let (v, order) = (old.stretch(&st.mem), old.stretch_order(&st.mem));
                    sum.set_stretch(&mut st.mem, v);
                    sum.set_stretch_order(&mut st.mem, order);
                }
                if sum.shrink(&st.mem).0 == 0 {
                    sum.set_shrink_order(&mut st.mem, NORMAL);
                }
                if sum.shrink_order(&st.mem) == old.shrink_order(&st.mem) {
                    let v = sum.shrink(&st.mem).0 + old.shrink(&st.mem).0;
                    sum.set_shrink(&mut st.mem, Scaled(v));
                } else if sum.shrink_order(&st.mem) < old.shrink_order(&st.mem)
                    && old.shrink(&st.mem).0 != 0
                {
                    let (v, order) = (old.shrink(&st.mem), old.shrink_order(&st.mem));
                    sum.set_shrink(&mut st.mem, v);
                    sum.set_shrink_order(&mut st.mem, order);
                }
                st.scanner.cur_val = sum.ptr().0;
            }
        }
    } else {
        crate::scan::scan_int_in(st)?;
        let n = st.scanner.cur_val;
        let scale = |errors: &mut crate::state::ErrorState, x: i32, max_answer: i32| {
            if q == MULTIPLY {
                mult_and_add_in(errors, x, n, 0, max_answer)
            } else {
                x_over_n_in(errors, x, n)
            }
        };
        if p < GLUE_VAL {
            let max_answer = if p == INT_VAL {
                TEX_INFINITY
            } else {
                MAX_HALFWORD
            };
            st.scanner.cur_val = scale(&mut st.errors, w, max_answer);
        } else {
            let r = crate::mem::alloc_spec(&mut st.mem, s)
                .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
            let old = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(s))
                .expect("non-null glue spec");
            let (width, stretch, shrink) = (
                old.width(&st.mem).0,
                old.stretch(&st.mem).0,
                old.shrink(&st.mem).0,
            );
            let v = scale(&mut st.errors, width, MAX_HALFWORD);
            r.set_width(&mut st.mem, Scaled(v));
            let v = scale(&mut st.errors, stretch, MAX_HALFWORD);
            r.set_stretch(&mut st.mem, Scaled(v));
            let v = scale(&mut st.errors, shrink, MAX_HALFWORD);
            r.set_shrink(&mut st.mem, Scaled(v));
            st.scanner.cur_val = r.ptr().0;
        }
    }

    if st.errors.arith_error {
        crate::output::error_here_with_diagnostic_in(st, c"Arithmetic overflow".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[
                c"I can't carry out that multiplication or division,",
                c"since the result is out of range.",
            ],
        );
        if p >= GLUE_VAL {
            crate::mem::delete_glue_ref(&mut st.mem, st.scanner.cur_val);
        }
        return mc_propagate_error(crate::errors::error_in(st));
    }

    let global = a >= 4;
    if p < GLUE_VAL {
        let value = st.scanner.cur_val;
        if e {
            sa_word_define(st, global, l, value)?;
        } else {
            word_define(st, global, l, value)?;
        }
    } else {
        trap_zero_glue(st);
        let value = st.scanner.cur_val;
        if e {
            sa_define(st, global, l, value)?;
        } else {
            define(st, global, l, GLUE_REF, value)?;
        }
    }
    Ok(())
}

pub(crate) fn do_assignments_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    loop {
        get_x_non_blank_non_relax(st)?;
        if st.scanner.cur_cmd <= MAX_NON_PREFIXED_COMMAND {
            return Ok(());
        }
        st.scanner.set_box_allowed = false;
        prefixed_command_in(st)?;
        st.scanner.set_box_allowed = true;
    }
}

pub(crate) fn main_control_append_normal_space_in(
    st: &mut EngineState,
) -> Result<(), MainControlAbort> {
    use crate::scan::NodeRecycleAbort;

    if insert_boundary_inter_char_toks(st)? {
        return Ok(());
    }
    let glue = if st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_SPACE_SKIP as i32) == ZERO_GLUE.0 {
        st.mainloop.main_p.0 = font_space_glue(st)?;
        crate::mem::alloc_glue(&mut st.mem, st.mainloop.main_p.0)
            .ok_or(NodeRecycleAbort::Overflow)?
    } else {
        crate::mem::alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_SPACE_SKIP)
            .ok_or(NodeRecycleAbort::Overflow)?
    };
    st.mem.temp_ptr = glue.ptr();
    tail_append(st, glue.ptr());
    Ok(())
}

fn append_rule_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let mode = st.input.cur_list.mode.abs();
    let rule = crate::scan::scan_rule_spec_in(st)?;
    tail_append(st, rule.ptr());
    if mode == VMODE {
        st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
    } else if mode == HMODE {
        st.input.cur_list.set_aux_s0(1000);
    }
    Ok(())
}

fn valign_or_lr_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.cur_chr <= 0 {
        return init_align_in(st);
    }
    let enabled = crate::scan::eTeX_enabled_in(
        st,
        st.eqtb.b32s1(INT_BASE + INT_PAR_TEXXET) > 0,
        st.scanner.cur_cmd,
        st.scanner.cur_chr,
    )?;
    if enabled {
        let m = crate::mem::alloc_math(&mut st.mem, 0, st.scanner.cur_chr as u16)
            .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
        tail_append(st, m.ptr());
    }
    Ok(())
}

/// Runs `action` if the math-shift group is open, else inserts what closes the current group.
fn in_math_shift_group(
    st: &mut EngineState,
    action: fn(&mut EngineState) -> Result<(), MainControlAbort>,
) -> Result<(), MainControlAbort> {
    if st.save_stack.cur_group == GroupCode::MathShift {
        action(st)
    } else {
        off_save_in(st)
    }
}

/// Scans the class, family and code of `\Umathchar` or `\Udelimiter`.
fn scan_umath_char(st: &mut EngineState) -> Result<i32, MainControlAbort> {
    crate::scan::scan_math_class_int_in(st)?;
    let mut t = set_class_fn(st.scanner.cur_val);
    crate::scan::scan_math_fam_int_in(st)?;
    t += set_family_fn(st.scanner.cur_val);
    crate::scan::scan_usv_num_in(st)?;
    Ok(t + st.scanner.cur_val)
}

/// Converts a classic 15-bit `\mathchar` code to the XeTeX layout.
fn math_char_from_tex(v: i32) -> i32 {
    set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256)
}

fn math_char_num_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let c = match st.scanner.cur_chr {
        2 => scan_umath_char(st)?,
        1 => {
            crate::scan::scan_xetex_math_char_int_in(st)?;
            st.scanner.cur_val
        }
        _ => {
            crate::scan::scan_fifteen_bit_int_in(st)?;
            math_char_from_tex(st.scanner.cur_val)
        }
    };
    crate::scan::set_math_char_in(st, c)?;
    Ok(())
}

fn math_delim_num_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    let c = if st.scanner.cur_chr == 1 {
        scan_umath_char(st)?
    } else {
        crate::scan::scan_delimiter_int_in(st)?;
        math_char_from_tex(st.scanner.cur_val / 4096)
    };
    crate::scan::set_math_char_in(st, c)?;
    Ok(())
}

/// Appends a noad of type `cur_chr` (or an ordinary one for `{`) and scans its nucleus.
fn append_noad_in(st: &mut EngineState, backup: bool) -> Result<(), MainControlAbort> {
    let n = crate::mem::alloc_noad(&mut st.mem).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    tail_append(st, n.ptr());
    if backup {
        crate::input::back_input_in(st)?;
    } else {
        st.mem.set_b16s1(n.idx(), st.scanner.cur_chr as u16);
    }
    crate::scan::scan_math_in(st, n.nucleus_addr())?;
    Ok(())
}

fn math_vcenter_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    crate::scan::scan_spec_in(st, GroupCode::Vcenter, false)?;
    normal_paragraph(st)?;
    crate::mem::push_nest_in(st)?;
    st.input.cur_list.mode = INNER_VMODE;
    st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
    if st.shipout.insert_src_special_every_vbox {
        crate::ext::insert_src_special_in(st)?;
    }
    begin_every(st, LOCAL_EVERY_VBOX, EVERY_VBOX_TEXT)
}

pub(crate) fn main_control_ignore_spaces_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    if st.scanner.cur_chr == 0 {
        loop {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd != SPACER {
                break;
            }
        }
    } else {
        let saved = st.scanner.scanner_status;
        st.scanner.scanner_status = NORMAL_SCANNER;
        let outcome = crate::scan::get_next_in(st);
        st.scanner.scanner_status = saved;
        Result::<(), crate::scan::SccAbort>::from(outcome)?;

        let name = if st.scanner.cur_cs < HASH_BASE {
            st.scanner.cur_cs - SINGLE_BASE
        } else {
            st.eqtb.hash_s1(st.scanner.cur_cs)
        };
        st.scanner.cur_cs = crate::hash::prim_lookup(&mut st.eqtb, &mut st.strings, name)
            .map_err(crate::scan::hash_err_abort)?;

        if st.scanner.cur_cs != UNDEFINED_PRIMITIVE {
            st.scanner.cur_cmd = st.eqtb.eq_type(PRIM_EQTB_BASE + st.scanner.cur_cs);
            st.scanner.cur_chr = st.eqtb.b32s1(PRIM_EQTB_BASE + st.scanner.cur_cs);
            st.scanner.cur_tok = CS_TOKEN_FLAG + PRIM_EQTB_BASE + st.scanner.cur_cs;
        }
    }
    Ok(())
}

fn inter_char_tokens_enabled(st: &EngineState) -> bool {
    st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0
}

/// The `\XeTeXinterchartoks` list for class pair `n`, if set. Leaves the
/// pair's trie record, possibly null, in `cur_ptr`.
fn inter_char_toks(st: &mut EngineState, n: i32) -> Option<i32> {
    crate::save::find_sa_element(&mut st.mem, &mut st.scanner, INTER_CHAR_VAL, n, false);
    let list = NodePtr::<SaNode>::from_ptr(st.scanner.cur_ptr)?.sa_ptr(&st.mem);
    (!list.is_null()).then_some(list.0)
}

/// Inserts the inter-character tokens between the last character's class and
/// the boundary class, ahead of the current token. Returns whether it did.
fn insert_boundary_inter_char_toks(st: &mut EngineState) -> Result<bool, MainControlAbort> {
    if !inter_char_tokens_enabled(st) {
        return Ok(false);
    }
    let sc = st.mainloop.space_class;
    if sc == CHAR_CLASS_LIMIT || st.mainloop.prev_class == CHAR_CLASS_LIMIT - 1 {
        return Ok(false);
    }
    st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;
    let Some(list) = inter_char_toks(st, sc * CHAR_CLASS_LIMIT + CHAR_CLASS_LIMIT - 1) else {
        return Ok(false);
    };
    if st.scanner.cur_cs == 0 {
        if st.scanner.cur_cmd == CHAR_NUM {
            st.scanner.cur_cmd = OTHER_CHAR;
        }
        st.scanner.cur_tok = st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
    } else {
        st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
    }
    crate::input::back_input_in(st)?;
    crate::input::begin_token_list_in(st, list, INTER_CHAR_TEXT)?;
    Ok(true)
}

/// Inserts the inter-character tokens between the previous character's class
/// and class `sc` of the current character. Returns whether it did.
fn insert_inter_char_toks_before(st: &mut EngineState, sc: i32) -> Result<bool, MainControlAbort> {
    if !inter_char_tokens_enabled(st) || sc == CHAR_CLASS_LIMIT {
        return Ok(false);
    }
    let prev = st.mainloop.prev_class;
    if prev == CHAR_CLASS_LIMIT - 1
        && st.input.cur_input.state == TOKEN_LIST
        && st.input.cur_input.index == BACKED_UP_CHAR
    {
        return Ok(false);
    }
    let Some(list) = inter_char_toks(st, prev * CHAR_CLASS_LIMIT + sc) else {
        return Ok(false);
    };
    if st.scanner.cur_cmd != LETTER {
        st.scanner.cur_cmd = OTHER_CHAR;
    }
    st.scanner.cur_tok = st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
    crate::input::back_input_in(st)?;
    st.input.cur_input.index = BACKED_UP_CHAR;
    crate::input::begin_token_list_in(st, list, INTER_CHAR_TEXT)?;
    Ok(true)
}

fn note_char_class(st: &mut EngineState, sc: i32) {
    if inter_char_tokens_enabled(st) && sc != CHAR_CLASS_LIMIT {
        st.mainloop.prev_class = sc;
    }
}

/// TeX's `adjust_space_factor` for `cur_chr`. Also resets `cur_ptr` and
/// records and returns the character's space class.
#[inline(always)]
fn adjust_space_factor(st: &mut EngineState) -> i32 {
    let sf_code = st.eqtb.b32s1(SF_CODE_BASE + st.scanner.cur_chr);
    let s = sf_code % 65536;
    if s == 1000 {
        st.input.cur_list.set_aux_s0(1000);
    } else if s < 1000 {
        if s > 0 {
            st.input.cur_list.set_aux_s0(s);
        }
    } else if st.input.cur_list.aux_s0() < 1000 {
        st.input.cur_list.set_aux_s0(1000);
    } else {
        st.input.cur_list.set_aux_s0(s);
    }
    st.scanner.cur_ptr = MemPtr::NULL;
    st.mainloop.space_class = sf_code / 65536;
    st.mainloop.space_class
}

fn is_char_cmd(cmd: u8) -> bool {
    matches!(cmd, LETTER | OTHER_CHAR | CHAR_GIVEN)
}

#[inline]
fn overflow_err<T>(p: Option<T>) -> Result<T, MainControlAbort> {
    p.ok_or(MainControlAbort::NodeRecycle(
        crate::scan::NodeRecycleAbort::Overflow,
    ))
}

#[inline(always)]
fn native_text_reserve(scanner: &mut crate::state::ScannerState, extra: i32) {
    while scanner.native_text_size() <= scanner.native_len() + extra {
        scanner.native_text_size += 128;
        scanner
            .native_text
            .resize(scanner.native_text_size as usize, 0);
    }
}

#[inline(always)]
fn native_text_push(scanner: &mut crate::state::ScannerState, c: u16) {
    let len = scanner.native_len();
    scanner.native_text[len as usize] = c;
    scanner.set_native_len(len + 1);
}

fn is_hyphen_unit(st: &EngineState, c: i32, hyphen: i32) -> bool {
    c == hyphen
        || (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_DASH_BREAK) > 0 && (c == 8212 || c == 8211))
}

/// Advances `main_h` just past the next hyphen in `len` units of native text at `start`.
fn advance_past_hyphen(st: &mut EngineState, start: i32, len: i32) {
    let hyphen = st.fonts.hyphen_char_entry(st.mainloop.main_f);
    while st.mainloop.main_h < len
        && !is_hyphen_unit(
            st,
            st.scanner.native_text[(start + st.mainloop.main_h) as usize] as i32,
            hyphen,
        )
    {
        st.mainloop.main_h += 1;
    }
    if st.mainloop.main_h < len {
        st.mainloop.main_h += 1;
    }
}

/// Points `main_ppp` at the node before `main_pp`, skipping discretionary replacement lists.
fn find_main_ppp(st: &mut EngineState) {
    let pp = st.mainloop.main_pp;
    let mut ppp = st.input.cur_list.head;
    while ppp != pp && ppp.link(&st.mem) != pp {
        if !is_char_node(&st.mem, ppp) {
            if let Ok(AnyNode::Disc(disc)) = st.mem.classify(MemIdx(ppp.0)) {
                for _ in 0..disc.subtype(&st.mem) {
                    ppp = ppp.link(&st.mem);
                }
            }
        }
        if ppp != pp {
            ppp = ppp.link(&st.mem);
        }
    }
    st.mainloop.main_ppp = ppp;
}

fn as_native_word(mem: &MemState, p: MemPtr) -> Option<NodePtr<WhatsitNode>> {
    if is_char_node(mem, p) {
        return None;
    }
    match mem.classify(MemIdx(p.0)) {
        Ok(AnyNode::Whatsit(w)) if w.whatsit_subtype(mem).is_some_and(|s| s.is_native_word()) => {
            Some(w)
        }
        _ => None,
    }
}

/// The native word `main_pp` if new text in font `f` can extend it.
fn extendable_native_word(st: &EngineState, f: i32) -> Option<NodePtr<WhatsitNode>> {
    let (pp, ppp) = (st.mainloop.main_pp, st.mainloop.main_ppp);
    if pp.is_null() {
        return None;
    }
    let word = as_native_word(&st.mem, pp)?;
    let extendable = word.native_font(&st.mem).0 == f
        && ppp != pp
        && !is_char_node(&st.mem, ppp)
        && !matches!(st.mem.classify(MemIdx(ppp.0)), Ok(AnyNode::Disc(_)));
    extendable.then_some(word)
}

/// # Safety
/// `src` must hold `len` units and `node` must have room for `at + len`.
unsafe fn copy_to_native_node(
    mem: &mut MemState,
    node: NodePtr<WhatsitNode>,
    at: i32,
    src: *const u16,
    len: i32,
) {
    let dst = mem
        .as_u16_ptr_mut(node.idx().offset(NATIVE_NODE_SIZE))
        .add(at as usize);
    std::ptr::copy_nonoverlapping(src, dst, len.max(0) as usize);
}

fn use_glyph_metrics(st: &EngineState) -> bool {
    st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0
}

/// Skips the nodes allowed between two words of an interword space.
fn skip_interword_items(mem: &MemState, mut p: MemPtr) -> MemPtr {
    while !is_char_node(mem, p)
        && match mem.classify(MemIdx(p.0)) {
            Ok(AnyNode::Penalty(_) | AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_)) => {
                true
            }
            Ok(AnyNode::Whatsit(w)) => w.whatsit_subtype(mem).is_some_and(|s| s.is_io()),
            _ => false,
        }
    {
        p = p.link(mem);
    }
    p
}

/// The main loop for an AAT or OpenType font: collects a run of characters
/// into native word nodes. Returns whether to go to `big_switch`.
unsafe fn main_loop_native_in(st: &mut EngineState, mf: i32) -> Result<bool, MainControlAbort> {
    if st.input.cur_list.mode > 0
        && st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE) != st.input.cur_list.aux_s1()
    {
        fix_language(&mut st.input.cur_list, &mut st.mem, &st.eqtb)?;
    }
    st.mainloop.main_h = 0;
    st.mainloop.main_f = mf;
    st.scanner.set_native_len(0);

    loop {
        let sc = adjust_space_factor(st);
        let was_boundary = st.mainloop.prev_class == CHAR_CLASS_LIMIT - 1;
        if insert_inter_char_toks_before(st, sc)? {
            if was_boundary {
                return Ok(true);
            }
            st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;
            break;
        }
        note_char_class(st, sc);

        let ch = st.scanner.cur_chr;
        if ch > 65535 {
            native_text_reserve(&mut st.scanner, 2);
            native_text_push(&mut st.scanner, ((ch - 65536) / 1024 + 0xD800) as u16);
            native_text_push(&mut st.scanner, ((ch - 65536) % 1024 + 0xDC00) as u16);
        } else {
            native_text_reserve(&mut st.scanner, 1);
            native_text_push(&mut st.scanner, ch as u16);
        }
        let is_hyph = is_hyphen_unit(st, ch, st.fonts.hyphen_char_entry(mf));
        st.hyph.set_is_hyph(is_hyph);
        if st.mainloop.main_h == 0 && is_hyph {
            st.mainloop.main_h = st.scanner.native_len();
        }

        crate::scan::get_next_scc(st)?;
        if is_char_cmd(st.scanner.cur_cmd) {
            continue;
        }
        crate::scan::x_token_in(st)?;
        if is_char_cmd(st.scanner.cur_cmd) {
            continue;
        }
        if st.scanner.cur_cmd == CHAR_NUM {
            crate::scan::scan_usv_num_in(st)?;
            st.scanner.cur_chr = st.scanner.cur_val;
            continue;
        }
        insert_boundary_inter_char_toks(st)?;
        break;
    }

    let mapping = st.fonts.font_mapping_ptr(mf);
    if !mapping.is_null() {
        let text = st.scanner.native_text.as_mut_ptr();
        let len = st.scanner.native_len();
        let mapped_len = apply_mapping(&mut st.fonts, mapping, text, len);
        st.scanner.set_native_len(0);
        native_text_reserve(&mut st.scanner, mapped_len);
        st.mainloop.main_h = 0;
        let hyphen = st.fonts.hyphen_char_entry(mf);
        for i in 0..mapped_len {
            let c = st.fonts.mapped_text_entry(i);
            native_text_push(&mut st.scanner, c);
            if st.mainloop.main_h == 0 && is_hyphen_unit(st, c as i32, hyphen) {
                st.mainloop.main_h = st.scanner.native_len();
            }
        }
    }

    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_LOST_CHARS) > 0 {
        let mut i = 0;
        while i < st.scanner.native_len() {
            let mut c = st.scanner.native_text[i as usize] as i32;
            i += 1;
            if (0xD800..0xDC00).contains(&c) {
                c = 65536 + (c - 0xD800) * 1024 + st.scanner.native_text[i as usize] as i32
                    - 0xDC00;
                i += 1;
            }
            if crate::ext::map_char_to_glyph(&st.fonts, mf, c)? == 0 {
                mc_propagate_error(crate::output::char_warning_in(st, mf, c))?;
            }
        }
    }

    let len = st.scanner.native_len();
    st.mainloop.main_pp = st.input.cur_list.tail;
    find_main_ppp(st);
    if st.input.cur_list.mode == HMODE {
        let mut start = 0;
        let mut remaining = len;
        loop {
            if st.mainloop.main_h == 0 {
                st.mainloop.main_h = remaining;
            }
            if let Some(word) = extendable_native_word(st, mf) {
                let old_len = word.native_length(&st.mem) as i32;
                remaining = st.mainloop.main_h + old_len;
                native_text_reserve(&mut st.scanner, remaining);
                st.scanner.set_save_native_len(st.scanner.native_len());
                // SAFETY: a native word stores `native_length` text units after its header.
                let old_text =
                    std::slice::from_raw_parts(word.native_text_ptr(&st.mem), old_len as usize);
                for &c in old_text {
                    native_text_push(&mut st.scanner, c);
                }
                for i in 0..st.mainloop.main_h {
                    let c = st.scanner.native_text[(start + i) as usize];
                    native_text_push(&mut st.scanner, c);
                }
                crate::linebreak::do_locale_linebreaks_in(
                    st,
                    st.scanner.save_native_len(),
                    remaining,
                )?;
                st.scanner.set_native_len(st.scanner.save_native_len());
                remaining = st.scanner.native_len() - st.mainloop.main_h - start;
                start = st.mainloop.main_h;
                st.mainloop.main_h = 0;
                advance_past_hyphen(st, start, remaining);
                let next = word.link(&st.mem);
                st.mainloop.main_ppp.set_link(&mut st.mem, next);
                word.set_link(&mut st.mem, MemPtr::NULL);
                crate::packaging::flush_node_list(&mut st.mem, word.idx().0)
                    .map_err(MainControlAbort::Confusion)?;
                st.mainloop.main_pp = st.input.cur_list.tail;
                while st.mainloop.main_ppp.link(&st.mem) != st.mainloop.main_pp {
                    st.mainloop.main_ppp = st.mainloop.main_ppp.link(&st.mem);
                }
            } else {
                crate::linebreak::do_locale_linebreaks_in(st, start, st.mainloop.main_h)?;
                start += st.mainloop.main_h;
                remaining -= st.mainloop.main_h;
                st.mainloop.main_h = 0;
                advance_past_hyphen(st, start, remaining);
            }
            if remaining > 0 || st.hyph.is_hyph() {
                let disc = overflow_err(crate::mem::alloc_disc(&mut st.mem))?;
                tail_append(st, disc.ptr());
                st.mainloop.main_pp = st.input.cur_list.tail;
            }
            if remaining == 0 {
                break;
            }
        }
    } else {
        let pp = st.mainloop.main_pp;
        let old = extendable_native_word(st, mf);
        let old_len = old.map_or(0, |w| w.native_length(&st.mem) as i32);
        let node = crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, mf, len + old_len)?;
        pp.set_link(&mut st.mem, node.ptr());
        st.input.cur_list.tail = node.ptr();
        st.mem.journal_mark_range(
            node.idx().offset(NATIVE_NODE_SIZE),
            crate::state::native_text_words(len + old_len),
        );
        if let Some(word) = old {
            let old_text = word.native_text_ptr(&st.mem);
            copy_to_native_node(&mut st.mem, node, 0, old_text, old_len);
        }
        let text = st.scanner.native_text.as_ptr();
        copy_to_native_node(&mut st.mem, node, old_len, text, len);
        let metrics = use_glyph_metrics(st);
        crate::ext::measure_native_node(&st.fonts, &mut st.mem, node, metrics)?;
        if let Some(word) = old {
            let mut prev = st.input.cur_list.head;
            if prev != word.ptr() {
                while prev.link(&st.mem) != word.ptr() {
                    prev = prev.link(&st.mem);
                }
            }
            let next = word.link(&st.mem);
            prev.set_link(&mut st.mem, next);
            word.set_link(&mut st.mem, MemPtr::NULL);
            crate::packaging::flush_node_list(&mut st.mem, word.idx().0)
                .map_err(MainControlAbort::Confusion)?;
        }
    }

    if st
        .eqtb
        .b32s1(INT_BASE + INT_PAR_XETEX_INTERWORD_SPACE_SHAPING)
        > 0
    {
        shape_interword_space(st, mf)?;
    }
    Ok(!st.scanner.cur_ptr.is_null())
}

/// Kerns the interword glue before the current word by what the font's shaping
/// of `word space word` implies.
unsafe fn shape_interword_space(st: &mut EngineState, mf: i32) -> Result<(), MainControlAbort> {
    let tail = st.input.cur_list.tail;
    let mut p = st.input.cur_list.head;
    let mut prev_word = None;
    while p != tail {
        if !p.is_null() {
            prev_word = as_native_word(&st.mem, p).or(prev_word);
        }
        p = p.link(&st.mem);
    }
    let Some(prev_word) = prev_word.filter(|w| w.native_font(&st.mem).0 == mf) else {
        return Ok(());
    };
    let glue = skip_interword_items(&st.mem, prev_word.link(&st.mem));
    if is_char_node(&st.mem, glue) {
        return Ok(());
    }
    let Ok(AnyNode::Glue(glue)) = st.mem.classify(MemIdx(glue.0)) else {
        return Ok(());
    };
    if skip_interword_items(&st.mem, glue.link(&st.mem)) != tail {
        return Ok(());
    }
    // The tail is normally the new native word but is read unchecked, as in TeX.
    let prev_len = prev_word.native_length(&st.mem) as i32;
    let tail_len = st.mem.b16s1(tail + 4) as i32;
    let len = prev_len + 1 + tail_len;
    let joined = crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, mf, len)?;
    st.mem.journal_mark_range(
        joined.idx().offset(NATIVE_NODE_SIZE),
        crate::state::native_text_words(len),
    );
    let prev_text = prev_word.native_text_ptr(&st.mem);
    copy_to_native_node(&mut st.mem, joined, 0, prev_text, prev_len);
    copy_to_native_node(&mut st.mem, joined, prev_len, &(b' ' as u16), 1);
    let tail_text = st.mem.as_u16_ptr(tail + NATIVE_NODE_SIZE);
    copy_to_native_node(&mut st.mem, joined, prev_len + 1, tail_text, tail_len);
    let metrics = use_glyph_metrics(st);
    crate::ext::measure_native_node(&st.fonts, &mut st.mem, joined, metrics)?;
    let t = joined.width(&st.mem).0 - prev_word.width(&st.mem).0 - st.mem.b32s1(tail + 1);
    joined.free_native(&mut st.mem);
    // TeX reads `width(font_glue[f])`, which stays null while `\spaceskip` supplies the glue.
    let space = st
        .fonts
        .font_info_b32s1(st.fonts.param_base(mf) + SPACE_CODE);
    if t != space {
        let kern = overflow_err(crate::mem::alloc_kern(&mut st.mem, t - space))?;
        kern.set_subtype(&mut st.mem, SPACE_ADJUSTMENT);
        let next = glue.link(&st.mem);
        kern.set_link(&mut st.mem, next);
        glue.set_link(&mut st.mem, kern.ptr());
    }
    Ok(())
}

/// Where the ligature/kern program scan ended.
enum LigExit {
    /// `main_loop_wrapup`.
    Wrapup,
    /// `main_loop_move`, skipping the wrapup.
    Move,
    /// `main_loop_move+1`.
    Move1,
    /// Back to `main_lig_loop` with a new `cur_l`.
    Restart,
}

/// Where the next pass of the TFM main loop starts.
enum Resume {
    /// `main_lig_loop`.
    LigLoop,
    /// `main_lig_loop+1` at lig/kern program index `k`.
    LigProgram(i32),
    /// `main_loop_move+2`.
    Move2,
}

/// TeX's `wrapup`: packs a pending ligature and inserts a pending
/// discretionary. `rt_hit` says whether a right boundary hit may count.
#[inline(always)]
fn wrapup(st: &mut EngineState, rt_hit: bool) -> Result<(), MainControlAbort> {
    if st.mainloop.cur_l >= TOO_BIG_CHAR {
        return Ok(());
    }
    let main_f = st.mainloop.main_f;
    if st.mem.b32s1(st.mainloop.cur_q) > TEX_NULL
        && st.mem.b16s0(st.input.cur_list.tail) as i32 == st.fonts.hyphen_char_entry(main_f)
    {
        st.mainloop.ins_disc = true;
    }
    if st.mainloop.ligature_present {
        let lig_list = st.mem.b32s1(st.mainloop.cur_q);
        let lig = overflow_err(crate::mem::alloc_ligature(
            &mut st.mem,
            main_f,
            st.mainloop.cur_l as u16,
            lig_list,
        ))?;
        if st.mainloop.lft_hit {
            lig.set_subtype(&mut st.mem, 2);
            st.mainloop.lft_hit = false;
        }
        if rt_hit && st.mainloop.rt_hit && st.mainloop.lig_stack.is_null() {
            let s = lig.subtype(&st.mem);
            lig.set_subtype(&mut st.mem, s + 1);
            st.mainloop.rt_hit = false;
        }
        st.mainloop.cur_q.set_link(&mut st.mem, lig.ptr());
        st.input.cur_list.tail = lig.ptr();
        st.mainloop.ligature_present = false;
    }
    if st.mainloop.ins_disc {
        st.mainloop.ins_disc = false;
        if st.input.cur_list.mode > 0 {
            let disc = overflow_err(crate::mem::alloc_disc(&mut st.mem))?;
            tail_append(st, disc.ptr());
        }
    }
    Ok(())
}

#[inline(always)]
fn cur_l_info(st: &mut EngineState) -> crate::mem_types::B16x4 {
    let main_f = st.mainloop.main_f;
    let c = crate::fonts::effective_char(&mut st.fonts, true, main_f, st.mainloop.cur_l as u16);
    st.fonts.font_info_b16(st.fonts.char_base_entry(main_f) + c)
}

/// Returns true to go to `big_switch`, false to go to `reswitch`.
pub(crate) unsafe fn main_control_main_loop_in(
    st: &mut EngineState,
) -> Result<bool, MainControlAbort> {
    if st.input.cur_list.head == st.input.cur_list.tail
        && st.input.cur_list.mode > 0
        && st.shipout.insert_src_special_auto()
    {
        append_src_special_in(st)?;
    }
    st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;

    let mf = st.eqtb.b32s1(CUR_FONT_LOC);
    if is_native_font(&st.fonts, mf) {
        return main_loop_native_in(st, mf);
    }

    let sc = adjust_space_factor(st);
    if insert_inter_char_toks_before(st, sc)? {
        return Ok(true);
    }
    note_char_class(st, sc);

    st.mainloop.main_f = mf;
    st.mainloop.bchar = st.fonts.font_bchar_entry(mf);
    st.mainloop.false_bchar = st.fonts.font_false_bchar_entry(mf);
    if st.input.cur_list.mode > 0
        && st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE) != st.input.cur_list.aux_s1()
    {
        fix_language(&mut st.input.cur_list, &mut st.mem, &st.eqtb)?;
    }
    let ls = overflow_err(crate::mem::get_avail(&mut st.mem))?;
    st.mainloop.lig_stack = ls;
    st.mem.set_b16s1(ls, mf as u16);
    st.mainloop.cur_l = st.scanner.cur_chr;
    st.mem.set_b16s0(ls, st.mainloop.cur_l as u16);
    st.mainloop.cur_q = st.input.cur_list.tail;

    let bchar_label = if st.mainloop.cancel_boundary {
        st.mainloop.cancel_boundary = false;
        NON_ADDRESS
    } else {
        st.fonts.bchar_label_entry(mf)
    };
    let mut resume = if bchar_label != NON_ADDRESS {
        st.mainloop.cur_r = st.mainloop.cur_l;
        st.mainloop.cur_l = TOO_BIG_CHAR;
        Resume::LigProgram(bchar_label)
    } else {
        Resume::Move2
    };
    let mut main_i = crate::mem_types::B16x4::default();

    loop {
        let exit = match std::mem::replace(&mut resume, Resume::LigLoop) {
            Resume::Move2 => None,
            Resume::LigProgram(k) => Some(lig_kern_program(st, k, &mut main_i)?),
            Resume::LigLoop if main_i.s1 % 4 != LIG_TAG || st.mainloop.cur_r == TOO_BIG_CHAR => {
                Some(LigExit::Wrapup)
            }
            Resume::LigLoop => {
                let base = st.fonts.lig_kern_base_entry(st.mainloop.main_f);
                let first = st.fonts.font_info_b16(base + main_i.s0 as i32);
                let k = if first.s3 > 128 {
                    base + 256 * first.s1 as i32 + first.s0 as i32 + 32768 - 256 * 128
                } else {
                    base + main_i.s0 as i32
                };
                Some(lig_kern_program(st, k, &mut main_i)?)
            }
        };
        if let Some(exit) = exit {
            match exit {
                LigExit::Restart => continue,
                LigExit::Wrapup => wrapup(st, true)?,
                LigExit::Move | LigExit::Move1 => {}
            }
            if !matches!(exit, LigExit::Move1) {
                if st.mainloop.lig_stack.is_null() {
                    return Ok(false);
                }
                st.mainloop.cur_q = st.input.cur_list.tail;
                st.mainloop.cur_l = st.mem.b16s0(st.mainloop.lig_stack) as i32;
            }

            if !is_char_node(&st.mem, st.mainloop.lig_stack) {
                let item = st.mainloop.lig_stack;
                let lig_ptr = st.mem.link_ptr(item + 1);
                if lig_ptr.0 > TEX_NULL {
                    tail_append(st, lig_ptr);
                }
                st.mainloop.lig_stack = item.link(&st.mem);
                crate::mem::free_node(&mut st.mem, item, SMALL_NODE_SIZE);
                main_i = cur_l_info(st);
                st.mainloop.ligature_present = true;
                if st.mainloop.lig_stack.is_null() {
                    if lig_ptr.0 > TEX_NULL {
                        if !lookahead(st)? {
                            return Ok(true);
                        }
                        continue;
                    }
                    st.mainloop.cur_r = st.mainloop.bchar;
                } else {
                    st.mainloop.cur_r = st.mem.b16s0(st.mainloop.lig_stack) as i32;
                }
                continue;
            }
        }

        let main_f = st.mainloop.main_f;
        let cur_chr = st.scanner.cur_chr as u16;
        // Each call resets `xtx_ligature_present`, so the two bounds may see different codes.
        let exists = if crate::fonts::effective_char(&mut st.fonts, false, main_f, cur_chr)
            > st.fonts.font_ec_entry(main_f) as i32
            || crate::fonts::effective_char(&mut st.fonts, false, main_f, cur_chr)
                < st.fonts.font_bc_entry(main_f) as i32
        {
            false
        } else {
            main_i =
                crate::fonts::effective_char_info(&mut st.fonts, main_f, st.mainloop.cur_l as u16);
            main_i.s3 > 0
        };
        if !exists {
            mc_propagate_error(crate::output::char_warning_in(
                st,
                main_f,
                st.scanner.cur_chr,
            ))?;
            crate::mem::free_avail(&mut st.mem, st.mainloop.lig_stack);
            return Ok(true);
        }
        tail_append(st, st.mainloop.lig_stack);
        if !lookahead(st)? {
            return Ok(true);
        }
    }
}

/// Runs the font's ligature/kern program for `cur_l` and `cur_r` from index `k`.
#[inline(always)]
fn lig_kern_program(
    st: &mut EngineState,
    mut k: i32,
    main_i: &mut crate::mem_types::B16x4,
) -> Result<LigExit, MainControlAbort> {
    loop {
        let step = st.fonts.font_info_b16(k);
        if step.s2 as i32 != st.mainloop.cur_r || step.s3 > 128 {
            if step.s3 == 0 {
                k += 1;
            } else if step.s3 >= 128 {
                return Ok(LigExit::Wrapup);
            } else {
                k += step.s3 as i32 + 1;
            }
            continue;
        }
        if step.s1 >= 128 {
            wrapup(st, true)?;
            let main_f = st.mainloop.main_f;
            let kern = st.fonts.font_info_b32s1(
                st.fonts.kern_base_entry(main_f) + 256 * step.s1 as i32 + step.s0 as i32,
            );
            let kern = overflow_err(crate::mem::alloc_kern(&mut st.mem, kern))?;
            tail_append(st, kern.ptr());
            return Ok(LigExit::Move);
        }
        if st.mainloop.cur_l == TOO_BIG_CHAR {
            st.mainloop.lft_hit = true;
        } else if st.mainloop.lig_stack.is_null() {
            st.mainloop.rt_hit = true;
        }
        match step.s1 {
            1 | 5 => {
                st.mainloop.cur_l = step.s0 as i32;
                *main_i = cur_l_info(st);
                st.mainloop.ligature_present = true;
            }
            2 | 6 => {
                st.mainloop.cur_r = step.s0 as i32;
                let c = st.mainloop.cur_r as u16;
                if st.mainloop.lig_stack.is_null() {
                    st.mainloop.lig_stack =
                        overflow_err(crate::mem::alloc_lig_item(&mut st.mem, c))?.ptr();
                    st.mainloop.bchar = TOO_BIG_CHAR;
                } else if is_char_node(&st.mem, st.mainloop.lig_stack) {
                    let prev = st.mainloop.lig_stack;
                    let item = overflow_err(crate::mem::alloc_lig_item(&mut st.mem, c))?;
                    st.mainloop.lig_stack = item.ptr();
                    st.mem.set_b32s1(item.idx().offset(1), prev.0);
                } else {
                    st.mem.set_b16s0(st.mainloop.lig_stack, c);
                }
            }
            3 => {
                st.mainloop.cur_r = step.s0 as i32;
                let prev = st.mainloop.lig_stack;
                let item = overflow_err(crate::mem::alloc_lig_item(
                    &mut st.mem,
                    st.mainloop.cur_r as u16,
                ))?;
                st.mainloop.lig_stack = item.ptr();
                item.ptr().set_link(&mut st.mem, prev);
            }
            7 | 11 => {
                wrapup(st, false)?;
                st.mainloop.cur_q = st.input.cur_list.tail;
                st.mainloop.cur_l = step.s0 as i32;
                *main_i = cur_l_info(st);
                st.mainloop.ligature_present = true;
            }
            _ => {
                st.mainloop.cur_l = step.s0 as i32;
                st.mainloop.ligature_present = true;
                return Ok(if st.mainloop.lig_stack.is_null() {
                    LigExit::Wrapup
                } else {
                    LigExit::Move1
                });
            }
        }
        if step.s1 > 4 && step.s1 != 7 {
            return Ok(LigExit::Wrapup);
        }
        if st.mainloop.cur_l < TOO_BIG_CHAR {
            return Ok(LigExit::Restart);
        }
        k = st.fonts.bchar_label_entry(st.mainloop.main_f);
    }
}

/// TeX's `main_loop_lookahead`: fetches the next character into a new
/// `lig_stack` and `cur_r`. Returns false when inter-character tokens were
/// inserted instead.
#[inline(always)]
fn lookahead(st: &mut EngineState) -> Result<bool, MainControlAbort> {
    crate::scan::get_next_scc(st)?;
    if !is_char_cmd(st.scanner.cur_cmd) {
        crate::scan::x_token_in(st)?;
    }
    if !is_char_cmd(st.scanner.cur_cmd) {
        if st.scanner.cur_cmd != CHAR_NUM {
            if st.scanner.cur_cmd == NO_BOUNDARY {
                st.mainloop.bchar = TOO_BIG_CHAR;
            }
            st.mainloop.cur_r = st.mainloop.bchar;
            st.mainloop.lig_stack = MemPtr::NULL;
            return Ok(true);
        }
        crate::scan::scan_char_num_in(st)?;
        st.scanner.cur_chr = st.scanner.cur_val;
    }

    let sc = adjust_space_factor(st);
    let was_boundary = st.mainloop.prev_class == CHAR_CLASS_LIMIT - 1;
    if insert_inter_char_toks_before(st, sc)? {
        if !was_boundary {
            st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;
        }
        return Ok(false);
    }
    note_char_class(st, sc);

    let ls = overflow_err(crate::mem::get_avail(&mut st.mem))?;
    st.mainloop.lig_stack = ls;
    st.mem.set_b16s1(ls, st.mainloop.main_f as u16);
    let c = st.scanner.cur_chr;
    st.mainloop.cur_r = c;
    st.mem.set_b16s0(ls, c as u16);
    if c == st.mainloop.false_bchar {
        st.mainloop.cur_r = TOO_BIG_CHAR;
    }
    Ok(true)
}

/// Where `main_control` continues after a command.
enum Next {
    /// `big_switch`: fetch a new token.
    BigSwitch,
    /// `reswitch`: act on the current token again.
    Reswitch,
    /// `\end` or `\dump` finished the job.
    Stop,
}

/// Acts on the current command in the current mode, TeX's `abs(mode)+cur_cmd` case.
#[inline]
unsafe fn dispatch_command_in(st: &mut EngineState) -> Result<Next, MainControlAbort> {
    use crate::scan::NodeRecycleAbort;

    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 0 {
        crate::output::show_cur_cmd_chr_in(st);
    }
    let mode = st.input.cur_list.mode.abs();
    let cmd = st.scanner.cur_cmd;
    if mode == HMODE {
        match cmd {
            LETTER | OTHER_CHAR | CHAR_GIVEN | CHAR_NUM => {
                if cmd == CHAR_NUM {
                    crate::scan::scan_usv_num_in(st)?;
                    st.scanner.cur_chr = st.scanner.cur_val;
                }
                return Ok(if main_control_main_loop_in(st)? {
                    Next::BigSwitch
                } else {
                    Next::Reswitch
                });
            }
            NO_BOUNDARY => {
                crate::scan::get_x_token_in(st)?;
                if matches!(
                    st.scanner.cur_cmd,
                    LETTER | OTHER_CHAR | CHAR_GIVEN | CHAR_NUM
                ) {
                    st.mainloop.cancel_boundary = true;
                }
                return Ok(Next::Reswitch);
            }
            _ => {}
        }
        if insert_boundary_inter_char_toks(st)? {
            return Ok(Next::BigSwitch);
        }
    }

    match (mode, cmd) {
        (HMODE, SPACER) => {
            if st.input.cur_list.aux_s0() == 1000 {
                main_control_append_normal_space_in(st)?;
            } else {
                app_space_in(st)?;
            }
        }
        (HMODE | MMODE, EX_SPACE) => main_control_append_normal_space_in(st)?,
        (_, RELAX) | (VMODE | MMODE, SPACER) | (MMODE, NO_BOUNDARY) => {}
        (_, IGNORE_SPACES) => {
            main_control_ignore_spaces_in(st)?;
            return Ok(Next::Reswitch);
        }
        (VMODE, STOP) => {
            if its_all_over_in(st)? {
                return Ok(Next::Stop);
            }
        }

        (VMODE, VMOVE)
        | (HMODE | MMODE, HMOVE)
        | (_, LAST_ITEM)
        | (VMODE, VADJUST | ITAL_CORR)
        | (VMODE | HMODE, EQ_NO)
        | (_, MAC_PARAM) => mc_propagate_error(crate::errors::report_illegal_case_in(st))?,
        (
            VMODE | HMODE,
            SUP_MARK | SUB_MARK | MATH_CHAR_NUM | MATH_GIVEN | XETEX_MATH_GIVEN | MATH_COMP
            | DELIM_NUM | LEFT_RIGHT | ABOVE | RADICAL | MATH_STYLE | MATH_CHOICE | VCENTER
            | NON_SCRIPT | MKERN | LIMIT_SWITCH | MSKIP | MATH_ACCENT,
        )
        | (MMODE, ENDV | PAR_END | STOP | VSKIP | UN_VBOX | VALIGN | HRULE) => {
            crate::scan::insert_dollar_sign_in(st)?
        }

        (VMODE, HRULE) | (HMODE | MMODE, VRULE) => append_rule_in(st)?,
        (VMODE, VSKIP) | (HMODE | MMODE, HSKIP) | (MMODE, MSKIP) => append_glue_in(st)?,
        (_, KERN) | (MMODE, MKERN) => append_kern_in(st)?,
        (VMODE | HMODE, LEFT_BRACE) => {
            crate::save::new_save_level(&mut st.save_stack, GroupCode::Simple, st.mem.line)?
        }
        (_, BEGIN_GROUP) => {
            crate::save::new_save_level(&mut st.save_stack, GroupCode::SemiSimple, st.mem.line)?
        }
        (_, END_GROUP) => {
            if st.save_stack.cur_group == GroupCode::SemiSimple {
                crate::save::unsave_in(st)?;
            } else {
                off_save_in(st)?;
            }
        }

        (_, RIGHT_BRACE) => handle_right_brace_in(st)?,
        (VMODE, HMOVE) | (HMODE | MMODE, VMOVE) => {
            let t = st.scanner.cur_chr;
            crate::scan::scan_dimen_in(st, false, false, false)?;
            let shift = if t == 0 {
                st.scanner.cur_val
            } else {
                -st.scanner.cur_val
            };
            scan_box_in(st, shift)?;
        }
        (_, LEADER_SHIP) => scan_box_in(st, LEADER_FLAG - A_LEADERS as i32 + st.scanner.cur_chr)?,
        (_, MAKE_BOX) => begin_box_in(st, 0)?,
        (VMODE, START_PAR) => new_graf_in(st, st.scanner.cur_chr > 0)?,
        (
            VMODE,
            LETTER | OTHER_CHAR | CHAR_NUM | CHAR_GIVEN | MATH_SHIFT | UN_HBOX | VRULE | ACCENT
            | DISCRETIONARY | HSKIP | VALIGN | EX_SPACE | NO_BOUNDARY,
        ) => {
            crate::input::back_input_in(st)?;
            new_graf_in(st, true)?;
        }
        (HMODE | MMODE, START_PAR) => indent_in_hmode(st)?,
        (VMODE, PAR_END) => {
            normal_paragraph(st)?;
            if st.input.cur_list.mode > 0 {
                crate::pagebuilder::build_page_in(st)?;
            }
        }
        (HMODE, PAR_END) => {
            if st.scanner.align_state < 0 {
                off_save_in(st)?;
            }
            end_graf_in(st)?;
            if st.input.cur_list.mode == VMODE {
                crate::pagebuilder::build_page_in(st)?;
            }
        }
        (HMODE, STOP | VSKIP | HRULE | UN_VBOX | HALIGN) => head_for_vmode_in(st)?,
        (_, INSERT) | (HMODE | MMODE, VADJUST) => begin_insert_or_adjust_in(st)?,
        (_, MARK) => make_mark_in(st)?,
        (_, BREAK_PENALTY) => append_penalty_in(st)?,
        (_, REMOVE_ITEM) => delete_last_in(st)?,
        (VMODE, UN_VBOX) | (HMODE | MMODE, UN_HBOX) => unpackage_in(st)?,
        (HMODE, ITAL_CORR) => append_italic_correction_in(st)?,
        (MMODE, ITAL_CORR) => {
            let k = crate::mem::alloc_kern(&mut st.mem, 0).ok_or(NodeRecycleAbort::Overflow)?;
            tail_append(st, k.ptr());
        }
        (HMODE | MMODE, DISCRETIONARY) => append_discretionary_in(st)?,
        (HMODE, ACCENT) => make_accent_in(st)?,
        (_, CAR_RET | TAB_MARK) => align_error_in(st)?,
        (_, NO_ALIGN) => no_align_error_in(st)?,
        (_, OMIT) => omit_error_in(st)?,
        (VMODE, HALIGN) => init_align_in(st)?,
        (HMODE, VALIGN) => valign_or_lr_in(st)?,
        (MMODE, HALIGN) => {
            if crate::errors::privileged_in(st).map_err(MainControlAbort::Error)? {
                in_math_shift_group(st, init_align_in)?;
            }
        }
        (VMODE | HMODE, ENDV) => do_endv_in(st)?,
        (_, END_CS_NAME) => mc_propagate_error(crate::errors::cs_error_in(st))?,

        (HMODE, MATH_SHIFT) => init_math_in(st)?,
        (MMODE, EQ_NO) => {
            if crate::errors::privileged_in(st).map_err(MainControlAbort::Error)? {
                in_math_shift_group(st, |st| Ok(crate::math::start_eq_no_in(st)?))?;
            }
        }
        (MMODE, LEFT_BRACE) => append_noad_in(st, true)?,
        (MMODE, LETTER | OTHER_CHAR | CHAR_GIVEN) => {
            crate::scan::set_math_char_in(st, st.eqtb.b32s1(MATH_CODE_BASE + st.scanner.cur_chr))?
        }
        (MMODE, CHAR_NUM) => {
            crate::scan::scan_char_num_in(st)?;
            st.scanner.cur_chr = st.scanner.cur_val;
            crate::scan::set_math_char_in(st, st.eqtb.b32s1(MATH_CODE_BASE + st.scanner.cur_chr))?;
        }
        (MMODE, MATH_CHAR_NUM) => math_char_num_in(st)?,
        (MMODE, MATH_GIVEN) => {
            crate::scan::set_math_char_in(st, math_char_from_tex(st.scanner.cur_chr))?
        }
        (MMODE, XETEX_MATH_GIVEN) => crate::scan::set_math_char_in(st, st.scanner.cur_chr)?,
        (MMODE, DELIM_NUM) => math_delim_num_in(st)?,
        (MMODE, MATH_COMP) => append_noad_in(st, false)?,
        (MMODE, LIMIT_SWITCH) => crate::math::math_limit_switch_in(st)?,
        (MMODE, RADICAL) => crate::math::math_radical_in(st)?,
        (MMODE, ACCENT | MATH_ACCENT) => crate::math::math_ac_in(st)?,
        (MMODE, VCENTER) => math_vcenter_in(st)?,
        (MMODE, MATH_STYLE) => {
            let s = crate::mem::alloc_style(&mut st.mem, st.scanner.cur_chr as i16)
                .ok_or(NodeRecycleAbort::Overflow)?;
            tail_append(st, s.ptr());
        }
        (MMODE, NON_SCRIPT) => {
            let g =
                crate::mem::alloc_glue(&mut st.mem, ZERO_GLUE).ok_or(NodeRecycleAbort::Overflow)?;
            tail_append(st, g.ptr());
            g.set_subtype(&mut st.mem, COND_MATH_GLUE);
        }
        (MMODE, MATH_CHOICE) => crate::math::append_choices_in(st)?,
        (MMODE, SUB_MARK | SUP_MARK) => crate::math::sub_sup_in(st)?,
        (MMODE, ABOVE) => crate::math::math_fraction_in(st)?,
        (MMODE, LEFT_RIGHT) => crate::math::math_left_right_in(st)?,
        (MMODE, MATH_SHIFT) => in_math_shift_group(st, |st| Ok(after_math_in(st)?))?,

        (_, TOKS_REGISTER..=SET_INTERACTION) => prefixed_command_in(st)?,
        (_, AFTER_ASSIGNMENT) => {
            crate::scan::get_token_in(st)?;
            st.scanner.after_token = st.scanner.cur_tok;
        }
        (_, AFTER_GROUP) => {
            crate::scan::get_token_in(st)?;
            crate::save::save_for_after(&mut st.save_stack, st.scanner.cur_tok)?;
        }
        (_, IN_STREAM) => crate::io::open_or_close_in(st)?,
        (_, MESSAGE) => crate::output::issue_message_in(st)?,
        (_, CASE_SHIFT) => shift_case_in(st)?,
        (_, XRAY) => crate::output::show_whatever_in(st)?,
        (_, EXTENSION) => do_extension_in(st)?,
        _ => {}
    }
    Ok(Next::BigSwitch)
}

unsafe fn main_control_from_in(
    st: &mut EngineState,
    run_every_job: bool,
    mut resume_pending_command: bool,
) -> Result<(), MainControlAbort> {
    if run_every_job {
        begin_every(st, LOCAL_EVERY_JOB, EVERY_JOB_TEXT)?;
    }
    // Cancellation and fence hooks are for resident sessions only, batch runs
    // must not pay for them per command.
    let resident = st.config.execution_mode == crate::state::EngineExecutionMode::Resident;
    loop {
        if resident && st.cancellation.at_command_boundary() {
            return Err(MainControlAbort::Cancelled);
        }
        if resume_pending_command {
            resume_pending_command = false;
        } else {
            if resident {
                crate::session::capture_prefetch_fence(st);
            }
            crate::scan::get_x_token_in(st)?;
        }
        if resident && crate::session::capture_command_fence(st) {
            return Err(MainControlAbort::Horizoned);
        }
        loop {
            match dispatch_command_in(st)? {
                Next::BigSwitch => break,
                Next::Reswitch => {}
                Next::Stop => return Ok(()),
            }
        }
    }
}

/// Dispatch a fresh document, including `\everyjob` and the first token fetch.
pub(crate) unsafe fn main_control_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    main_control_from_in(st, true, false)
}

/// Resume at a restored command boundary, optionally with a command already fetched.
pub(crate) unsafe fn main_control_resume_in(
    st: &mut EngineState,
    resume_pending_command: bool,
) -> Result<(), MainControlAbort> {
    main_control_from_in(st, false, resume_pending_command)
}

fn define(
    st: &mut EngineState,
    global: bool,
    p: i32,
    t: u8,
    e: i32,
) -> Result<(), crate::save::SaveAbort> {
    if global {
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
    }
}

fn word_define(
    st: &mut EngineState,
    global: bool,
    p: i32,
    w: i32,
) -> Result<(), crate::save::SaveAbort> {
    if global {
        crate::save::geq_word_define(&mut st.eqtb, p, w);
        Ok(())
    } else {
        crate::save::eq_word_define(&mut st.eqtb, &mut st.save_stack, p, w)
    }
}

fn sa_define(st: &mut EngineState, global: bool, p: i32, e: i32) -> Result<(), MainControlAbort> {
    let result = if global {
        crate::save::gsa_def(&mut st.mem, p, e)
    } else {
        crate::save::sa_def(&mut st.mem, &mut st.scanner, &mut st.save_stack, p, e)?
    };
    crate::save::finish_sa_mutation_in(st, p, result)?;
    Ok(())
}

fn sa_word_define(
    st: &mut EngineState,
    global: bool,
    p: i32,
    w: i32,
) -> Result<(), MainControlAbort> {
    let result = if global {
        crate::save::gsa_w_def(&mut st.mem, p, w)
    } else {
        crate::save::sa_w_def(&mut st.mem, &mut st.scanner, &mut st.save_stack, p, w)?
    };
    crate::save::finish_sa_mutation_in(st, p, result)?;
    Ok(())
}

pub(crate) fn add_token_ref(mem: &mut MemState, p: impl crate::units::MemAddr) {
    let p = p.addr();
    mem.set_b32s0(p, mem.b32s0(p) + 1);
}

fn add_sa_ref(mem: &mut MemState, p: MemPtr) {
    if let Some(sa) = NodePtr::<SaNode>::from_ptr(p) {
        let refs = sa.sa_ref(mem);
        sa.set_sa_ref(mem, refs + 1);
    }
}

/// `sa_ptr(cur_ptr)`, or null when `cur_ptr` is null.
fn cur_sa_ptr(st: &EngineState) -> i32 {
    NodePtr::<SaNode>::from_ptr(st.scanner.cur_ptr).map_or(TEX_NULL, |sa| sa.sa_ptr(&st.mem).0)
}

/// Scans a pair of character classes and finds (with `w`, creates) its
/// `\XeTeXinterchartoks` record in `cur_ptr`.
fn find_inter_char_toks(st: &mut EngineState, w: bool) -> Result<(), MainControlAbort> {
    crate::scan::scan_char_class_not_ignored_in(st)?;
    // TeX parks the first class in `cur_ptr` while scanning the second.
    st.scanner.cur_ptr.0 = st.scanner.cur_val;
    crate::scan::scan_char_class_not_ignored_in(st)?;
    let n = st.scanner.cur_ptr.0 * CHAR_CLASS_LIMIT + st.scanner.cur_val;
    crate::save::find_sa_element(&mut st.mem, &mut st.scanner, INTER_CHAR_VAL, n, w)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    Ok(())
}

/// Defines token list register `p`, a sparse-array entry if `e`, as `list`.
fn define_toks(
    st: &mut EngineState,
    e: bool,
    global: bool,
    p: i32,
    list: i32,
) -> Result<(), MainControlAbort> {
    if e {
        return sa_define(st, global, p, list);
    }
    let t = if list == TEX_NULL { UNDEFINED_CS } else { CALL };
    define(st, global, p, t, list)?;
    Ok(())
}

pub(crate) fn prefixed_command_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    use crate::scan::NodeRecycleAbort;

    let mut a: i32 = 0;
    while st.scanner.cur_cmd == PREFIX {
        if (a / st.scanner.cur_chr) & 1 == 0 {
            a += st.scanner.cur_chr;
        }
        get_x_non_blank_non_relax(st)?;
        if st.scanner.cur_cmd <= MAX_NON_PREFIXED_COMMAND {
            crate::output::error_here_with_diagnostic_in(
                st,
                c"You can't use a prefix with `".as_ptr(),
            );
            crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd, st.scanner.cur_chr);
            crate::output::print_char_in(st, '\'' as i32);
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            help(
                &mut st.errors,
                &[c"I'll pretend you didn't say \\long or \\outer or \\global or \\protected."],
            );
            return mc_propagate_error(crate::input::back_error_in(st)?);
        }
        if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_COMMANDS) > 2 {
            crate::output::show_cur_cmd_chr_in(st);
        }
    }

    let protected = if a >= 8 {
        a -= 8;
        PROTECTED_TOKEN
    } else {
        0
    };
    if st.scanner.cur_cmd != DEF && (a % 4 != 0 || protected != 0) {
        crate::output::error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
        crate::output::print_esc_cstr_in(st, c"long".as_ptr());
        crate::output::print_cstr_in(st, c"' or `".as_ptr());
        crate::output::print_esc_cstr_in(st, c"outer".as_ptr());
        crate::output::print_cstr_in(st, c"' or `".as_ptr());
        crate::output::print_esc_cstr_in(st, c"protected".as_ptr());
        crate::output::print_cstr_in(st, c"' with `".as_ptr());
        crate::output::print_cmd_chr_in(st, st.scanner.cur_cmd, st.scanner.cur_chr);
        crate::output::print_char_in(st, '\'' as i32);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[c"I'll pretend you didn't say \\long or \\outer or \\protected here."],
        );
        mc_propagate_error(crate::errors::error_in(st))?;
    }
    let global_defs = st.eqtb.b32s1(INT_BASE + INT_PAR_GLOBAL_DEFS);
    if global_defs < 0 && a >= 4 {
        a -= 4;
    } else if global_defs > 0 && a < 4 {
        a += 4;
    }

    match st.scanner.cur_cmd {
        SET_FONT => define(st, a >= 4, CUR_FONT_LOC, DATA, st.scanner.cur_chr)?,
        DEF => {
            if st.scanner.cur_chr & 1 != 0
                && a < 4
                && st.eqtb.b32s1(INT_BASE + INT_PAR_GLOBAL_DEFS) >= 0
            {
                a += 4;
            }
            let expand = st.scanner.cur_chr >= 2;
            crate::scan::get_r_token_in(st)?;
            let p = st.scanner.cur_cs;
            crate::scan::scan_toks_in(st, true, expand)?;
            if protected != 0 {
                let q = crate::mem::get_avail(&mut st.mem).ok_or(NodeRecycleAbort::Overflow)?;
                q.set_info(&mut st.mem, protected);
                let body = st.scanner.def_ref.link(&st.mem);
                q.set_link(&mut st.mem, body);
                st.scanner.def_ref.set_link(&mut st.mem, q);
            }
            define(st, a >= 4, p, CALL + (a % 4) as u8, st.scanner.def_ref.0)?;
        }
        LET => {
            let n = st.scanner.cur_chr;
            crate::scan::get_r_token_in(st)?;
            let p = st.scanner.cur_cs;
            if n == NORMAL as i32 {
                loop {
                    crate::scan::get_token_in(st)?;
                    if st.scanner.cur_cmd != SPACER {
                        break;
                    }
                }
                if st.scanner.cur_tok == OTHER_TOKEN + '=' as i32 {
                    crate::scan::get_token_in(st)?;
                    if st.scanner.cur_cmd == SPACER {
                        crate::scan::get_token_in(st)?;
                    }
                }
            } else {
                crate::scan::get_token_in(st)?;
                let q = st.scanner.cur_tok;
                crate::scan::get_token_in(st)?;
                crate::input::back_input_in(st)?;
                st.scanner.cur_tok = q;
                crate::input::back_input_in(st)?;
            }
            if st.scanner.cur_cmd >= CALL {
                add_token_ref(&mut st.mem, st.scanner.cur_chr);
            } else if (st.scanner.cur_cmd == REGISTER || st.scanner.cur_cmd == TOKS_REGISTER)
                && !(0..=19).contains(&st.scanner.cur_chr)
            {
                add_sa_ref(&mut st.mem, MemPtr::from_raw(st.scanner.cur_chr));
            }
            define(st, a >= 4, p, st.scanner.cur_cmd, st.scanner.cur_chr)?;
        }
        SHORTHAND_DEF => {
            if st.scanner.cur_chr == CHAR_SUB_DEF_CODE {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"vestigial MLTeX shorthand encountered??".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[c"This should never happen in Tectonic, where MLTeX has been excised."],
                );
                mc_propagate_error(crate::errors::error_in(st))?;
            } else {
                shorthand_def_in(st, a >= 4)?;
            }
        }
        READ_TO_CS => {
            let j = st.scanner.cur_chr;
            crate::scan::scan_int_in(st)?;
            let n = st.scanner.cur_val;
            if !crate::scan::scan_keyword_in(st, c"to".as_ptr())? {
                crate::output::error_here_with_diagnostic_in(st, c"Missing `to' inserted".as_ptr());
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"You should have said `\\read<number> to \\cs'.",
                        c"I'm going to look for the \\cs now.",
                    ],
                );
                mc_propagate_error(crate::errors::error_in(st))?;
            }
            crate::scan::get_r_token_in(st)?;
            let p = st.scanner.cur_cs;
            crate::scan::read_toks_in(st, n, p, j)?;
            define(st, a >= 4, p, CALL, st.scanner.cur_val)?;
        }
        TOKS_REGISTER | ASSIGN_TOKS => assign_toks_in(st, a >= 4)?,
        ASSIGN_INT => {
            let p = st.scanner.cur_chr;
            crate::scan::scan_optional_equals_in(st)?;
            crate::scan::scan_int_in(st)?;
            word_define(st, a >= 4, p, st.scanner.cur_val)?;
        }
        ASSIGN_DIMEN => {
            let p = st.scanner.cur_chr;
            crate::scan::scan_optional_equals_in(st)?;
            crate::scan::scan_dimen_in(st, false, false, false)?;
            word_define(st, a >= 4, p, st.scanner.cur_val)?;
        }
        ASSIGN_GLUE | ASSIGN_MU_GLUE => {
            let p = st.scanner.cur_chr;
            let level = if st.scanner.cur_cmd == ASSIGN_MU_GLUE {
                MU_VAL
            } else {
                GLUE_VAL
            };
            crate::scan::scan_optional_equals_in(st)?;
            crate::scan::scan_glue_in(st, level)?;
            trap_zero_glue(st);
            define(st, a >= 4, p, GLUE_REF, st.scanner.cur_val)?;
        }
        XETEX_DEF_CODE => xetex_def_code_in(st, a >= 4)?,
        DEF_CODE => def_code_in(st, a >= 4)?,
        DEF_FAMILY => {
            let base = st.scanner.cur_chr;
            crate::scan::scan_math_fam_int_in(st)?;
            let p = base + st.scanner.cur_val;
            crate::scan::scan_optional_equals_in(st)?;
            crate::scan::scan_font_ident_in(st)?;
            define(st, a >= 4, p, DATA, st.scanner.cur_val)?;
        }
        REGISTER | ADVANCE | MULTIPLY | DIVIDE => do_register_command_in(st, a as i16)?,
        SET_BOX => {
            crate::scan::scan_register_num_in(st)?;
            let n = if a >= 4 {
                GLOBAL_BOX_FLAG + st.scanner.cur_val
            } else {
                BOX_FLAG + st.scanner.cur_val
            };
            crate::scan::scan_optional_equals_in(st)?;
            if st.scanner.set_box_allowed {
                scan_box_in(st, n)?;
            } else {
                crate::output::error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
                crate::output::print_esc_cstr_in(st, c"setbox".as_ptr());
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(
                    &mut st.errors,
                    &[
                        c"Sorry, \\setbox is not allowed after \\halign in a display,",
                        c"or between \\accent and an accented character.",
                    ],
                );
                mc_propagate_error(crate::errors::error_in(st))?;
            }
        }
        SET_AUX => alter_aux_in(st)?,
        SET_PREV_GRAF => alter_prev_graf_in(st)?,
        SET_PAGE_DIMEN => alter_page_so_far_in(st)?,
        SET_PAGE_INT => alter_integer_in(st)?,
        SET_BOX_DIMEN => alter_box_dimen_in(st)?,
        SET_SHAPE => set_shape_in(st, a >= 4)?,
        HYPH_DATA => {
            if st.scanner.cur_chr != 1 {
                new_hyph_exceptions_in(st)?;
            } else if st.config.in_initex_mode {
                new_patterns_in(st)?;
            } else {
                crate::output::error_here_with_diagnostic_in(
                    st,
                    c"Patterns can be loaded only by INITEX".as_ptr(),
                );
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                help(&mut st.errors, &[]);
                mc_propagate_error(crate::errors::error_in(st))?;
                loop {
                    crate::scan::get_token_in(st)?;
                    if st.scanner.cur_cmd == RIGHT_BRACE {
                        return Ok(());
                    }
                }
            }
        }
        ASSIGN_FONT_DIMEN => {
            crate::scan::find_font_dimen_in(st, true)?;
            let k = st.scanner.cur_val;
            crate::scan::scan_optional_equals_in(st)?;
            crate::scan::scan_dimen_in(st, false, false, false)?;
            st.fonts.set_font_info_b32s1(k, st.scanner.cur_val);
        }
        ASSIGN_FONT_INT => {
            let n = st.scanner.cur_chr;
            crate::scan::scan_font_ident_in(st)?;
            let f = st.scanner.cur_val;
            if n < 2 {
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_int_in(st)?;
                if n == 0 {
                    st.fonts.set_hyphen_char_entry(f, st.scanner.cur_val);
                } else {
                    st.fonts.set_skew_char_entry(f, st.scanner.cur_val);
                }
            } else {
                if is_native_font(&st.fonts, f) {
                    crate::scan::scan_glyph_number_in(st, f)?;
                } else {
                    crate::scan::scan_char_num_in(st)?;
                }
                let p = st.scanner.cur_val;
                crate::scan::scan_optional_equals_in(st)?;
                crate::scan::scan_int_in(st)?;
                match n {
                    LP_CODE_BASE => unsafe {
                        set_cp_code(f, p as u32, LEFT_SIDE, st.scanner.cur_val)
                    },
                    RP_CODE_BASE => unsafe {
                        set_cp_code(f, p as u32, RIGHT_SIDE, st.scanner.cur_val)
                    },
                    _ => {}
                }
            }
        }
        DEF_FONT => unsafe { new_font_in(st, a as i16)? },
        SET_INTERACTION => new_interaction_in(st),
        _ => return Err(MainControlAbort::Confusion(c"prefix")),
    }

    if st.scanner.after_token != 0 {
        st.scanner.cur_tok = st.scanner.after_token;
        crate::input::back_input_in(st)?;
        st.scanner.after_token = 0;
    }
    Ok(())
}

fn shorthand_def_in(st: &mut EngineState, global: bool) -> Result<(), MainControlAbort> {
    let n = st.scanner.cur_chr;
    crate::scan::get_r_token_in(st)?;
    let p = st.scanner.cur_cs;
    define(st, global, p, RELAX, TOO_BIG_USV)?;
    crate::scan::scan_optional_equals_in(st)?;
    let (t, e) = match n {
        CHAR_DEF_CODE => {
            crate::scan::scan_usv_num_in(st)?;
            (CHAR_GIVEN, st.scanner.cur_val)
        }
        MATH_CHAR_DEF_CODE => {
            crate::scan::scan_fifteen_bit_int_in(st)?;
            (MATH_GIVEN, st.scanner.cur_val)
        }
        XETEX_MATH_CHAR_NUM_DEF_CODE => {
            crate::scan::scan_xetex_math_char_int_in(st)?;
            (XETEX_MATH_GIVEN, st.scanner.cur_val)
        }
        XETEX_MATH_CHAR_DEF_CODE => (XETEX_MATH_GIVEN, scan_umath_char(st)?),
        _ => {
            crate::scan::scan_register_num_in(st)?;
            if st.scanner.cur_val > 255 {
                let level = if n - 2 > MU_VAL as i32 {
                    TOK_VAL
                } else {
                    (n - 2) as u8
                };
                let reg = st.scanner.cur_val;
                crate::save::find_sa_element(&mut st.mem, &mut st.scanner, level, reg, true)
                    .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                add_sa_ref(&mut st.mem, st.scanner.cur_ptr);
                let t = if level == TOK_VAL {
                    TOKS_REGISTER
                } else {
                    REGISTER
                };
                (t, st.scanner.cur_ptr.0)
            } else {
                let (t, base) = match n {
                    COUNT_DEF_CODE => (ASSIGN_INT, COUNT_BASE),
                    DIMEN_DEF_CODE => (ASSIGN_DIMEN, SCALED_BASE),
                    SKIP_DEF_CODE => (ASSIGN_GLUE, SKIP_BASE),
                    MU_SKIP_DEF_CODE => (ASSIGN_MU_GLUE, MU_SKIP_BASE),
                    TOKS_DEF_CODE => (ASSIGN_TOKS, TOKS_BASE),
                    _ => return Ok(()),
                };
                (t, base + st.scanner.cur_val)
            }
        }
    };
    define(st, global, p, t, e)?;
    Ok(())
}

fn assign_toks_in(st: &mut EngineState, global: bool) -> Result<(), MainControlAbort> {
    let cs = st.scanner.cur_cs;
    let mut e = false;
    if st.scanner.cur_cmd == TOKS_REGISTER {
        if st.scanner.cur_chr == 0 {
            crate::scan::scan_register_num_in(st)?;
            if st.scanner.cur_val > 255 {
                let reg = st.scanner.cur_val;
                crate::save::find_sa_element(&mut st.mem, &mut st.scanner, TOK_VAL, reg, true)
                    .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
                st.scanner.cur_chr = st.scanner.cur_ptr.0;
                e = true;
            } else {
                st.scanner.cur_chr = TOKS_BASE + st.scanner.cur_val;
            }
        } else {
            e = true;
        }
    } else if st.scanner.cur_chr == LOCAL_BASE + LOCAL_XETEX_INTER_CHAR_TOKS {
        find_inter_char_toks(st, true)?;
        st.scanner.cur_chr = st.scanner.cur_ptr.0;
        e = true;
    }
    let p = st.scanner.cur_chr;
    crate::scan::scan_optional_equals_in(st)?;
    get_x_non_blank_non_relax(st)?;

    if st.scanner.cur_cmd != LEFT_BRACE
        && (st.scanner.cur_cmd == TOKS_REGISTER || st.scanner.cur_cmd == ASSIGN_TOKS)
    {
        let list = if st.scanner.cur_cmd == TOKS_REGISTER {
            if st.scanner.cur_chr == 0 {
                crate::scan::scan_register_num_in(st)?;
                if st.scanner.cur_val < 256 {
                    st.eqtb.b32s1(TOKS_BASE + st.scanner.cur_val)
                } else {
                    let reg = st.scanner.cur_val;
                    crate::save::find_sa_element(&mut st.mem, &mut st.scanner, TOK_VAL, reg, false);
                    cur_sa_ptr(st)
                }
            } else {
                st.mem.b32s1(st.scanner.cur_chr + 1)
            }
        } else if st.scanner.cur_chr == LOCAL_BASE + LOCAL_XETEX_INTER_CHAR_TOKS {
            find_inter_char_toks(st, false)?;
            cur_sa_ptr(st)
        } else {
            st.eqtb.b32s1(st.scanner.cur_chr)
        };
        if list != TEX_NULL {
            add_token_ref(&mut st.mem, list);
        }
        return define_toks(st, e, global, p, list);
    }

    crate::input::back_input_in(st)?;
    st.scanner.cur_cs = cs;
    let tail = crate::scan::scan_toks_in(st, false, false)?;
    let def_ref = st.scanner.def_ref;
    if def_ref.link(&st.mem).is_null() {
        define_toks(st, e, global, p, TEX_NULL)?;
        crate::mem::free_avail(&mut st.mem, def_ref);
        return Ok(());
    }
    if p == LOCAL_BASE + LOCAL_OUTPUT_ROUTINE && !e {
        store_new_token(&mut st.mem, MemPtr(tail), RIGHT_BRACE_TOKEN + '}' as i32)?;
        let q =
            crate::mem::get_avail(&mut st.mem).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
        q.set_info(&mut st.mem, LEFT_BRACE_TOKEN + '{' as i32);
        let body = def_ref.link(&st.mem);
        q.set_link(&mut st.mem, body);
        def_ref.set_link(&mut st.mem, q);
    }
    define_toks(st, e, global, p, def_ref.0)
}

fn xetex_def_code_in(st: &mut EngineState, global: bool) -> Result<(), MainControlAbort> {
    let chr = st.scanner.cur_chr;
    let base = if chr == SF_CODE_BASE || chr == MATH_CODE_BASE || chr == DEL_CODE_BASE {
        chr
    } else {
        chr - 1
    };
    crate::scan::scan_usv_num_in(st)?;
    let p = base + st.scanner.cur_val;
    if chr == SF_CODE_BASE {
        let sf = st.eqtb.b32s1(SF_CODE_BASE + st.scanner.cur_val) % 65536;
        crate::scan::scan_optional_equals_in(st)?;
        crate::scan::scan_char_class_in(st)?;
        let v = st.scanner.cur_val.wrapping_mul(65536).wrapping_add(sf);
        define(st, global, p, DATA, v)?;
    } else if chr == MATH_CODE_BASE {
        crate::scan::scan_optional_equals_in(st)?;
        crate::scan::scan_xetex_math_char_int_in(st)?;
        define(st, global, p, DATA, st.scanner.cur_val)?;
    } else if chr == MATH_CODE_BASE + 1 {
        crate::scan::scan_optional_equals_in(st)?;
        let v = scan_umath_char(st)?;
        define(st, global, p, DATA, v)?;
    } else if chr == DEL_CODE_BASE {
        crate::scan::scan_optional_equals_in(st)?;
        crate::scan::scan_int_in(st)?;
        word_define(st, global, p, st.scanner.cur_val)?;
    } else {
        crate::scan::scan_optional_equals_in(st)?;
        // Extended delimiter code flag.
        let mut n: i32 = 0x4000_0000;
        crate::scan::scan_math_fam_int_in(st)?;
        n = n.wrapping_add(st.scanner.cur_val.wrapping_mul(0x20_0000));
        crate::scan::scan_usv_num_in(st)?;
        n = n.wrapping_add(st.scanner.cur_val);
        word_define(st, global, p, n)?;
    }
    Ok(())
}

fn def_code_in(st: &mut EngineState, global: bool) -> Result<(), MainControlAbort> {
    let chr = st.scanner.cur_chr;
    let max = if chr == CAT_CODE_BASE {
        MAX_CHAR_CODE.into()
    } else if chr == MATH_CODE_BASE {
        0x8000
    } else if chr == SF_CODE_BASE {
        0x7FFF
    } else if chr == DEL_CODE_BASE {
        0xFF_FFFF
    } else {
        BIGGEST_USV
    };
    crate::scan::scan_usv_num_in(st)?;
    let p = chr + st.scanner.cur_val;
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_int_in(st)?;
    if (st.scanner.cur_val < 0 && p < DEL_CODE_BASE) || st.scanner.cur_val > max {
        crate::output::error_here_with_diagnostic_in(st, c"Invalid code (".as_ptr());
        crate::output::print_int_in(st, st.scanner.cur_val);
        if p < DEL_CODE_BASE {
            crate::output::print_cstr_in(st, c"), should be in the range 0..".as_ptr());
        } else {
            crate::output::print_cstr_in(st, c"), should be at most ".as_ptr());
        }
        crate::output::print_int_in(st, max);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        help(
            &mut st.errors,
            &[c"I'm going to use 0 instead of that illegal code value."],
        );
        mc_propagate_error(crate::errors::error_in(st))?;
        st.scanner.cur_val = 0;
    }
    if p < MATH_CODE_BASE {
        let v = if p >= SF_CODE_BASE {
            (st.eqtb.b32s1(p) / 65536)
                .wrapping_mul(65536)
                .wrapping_add(st.scanner.cur_val)
        } else {
            st.scanner.cur_val
        };
        define(st, global, p, DATA, v)?;
    } else if p < DEL_CODE_BASE {
        st.scanner.cur_val = if st.scanner.cur_val == 32768 {
            ACTIVE_MATH_CHAR
        } else {
            math_char_from_tex(st.scanner.cur_val)
        };
        define(st, global, p, DATA, st.scanner.cur_val)?;
    } else {
        word_define(st, global, p, st.scanner.cur_val)?;
    }
    Ok(())
}

fn set_shape_in(st: &mut EngineState, global: bool) -> Result<(), MainControlAbort> {
    use crate::scan::NodeRecycleAbort;

    let q = st.scanner.cur_chr;
    crate::scan::scan_optional_equals_in(st)?;
    crate::scan::scan_int_in(st)?;
    let n = st.scanner.cur_val;
    let p = if n <= 0 {
        TEX_NULL
    } else if q > LOCAL_BASE + LOCAL_PAR_SHAPE {
        let words = n / 2 + 1;
        let p = crate::mem::get_node(&mut st.mem, 2 * words + 1)
            .ok_or(NodeRecycleAbort::Overflow)?
            .0;
        st.mem.set_b32s0(p, words);
        st.mem.set_b32s1(p + 1, n);
        for j in p + 2..=p + n + 1 {
            crate::scan::scan_int_in(st)?;
            st.mem.set_b32s1(j, st.scanner.cur_val);
        }
        if n & 1 == 0 {
            st.mem.set_b32s1(p + n + 2, 0);
        }
        p
    } else {
        let p = crate::mem::get_node(&mut st.mem, 2 * n + 1)
            .ok_or(NodeRecycleAbort::Overflow)?
            .0;
        st.mem.set_b32s0(p, n);
        for j in 1..=n {
            crate::scan::scan_dimen_in(st, false, false, false)?;
            st.mem.set_b32s1(p + 2 * j - 1, st.scanner.cur_val);
            crate::scan::scan_dimen_in(st, false, false, false)?;
            st.mem.set_b32s1(p + 2 * j, st.scanner.cur_val);
        }
        p
    };
    define(st, global, q, SHAPE_REF, p)?;
    Ok(())
}
