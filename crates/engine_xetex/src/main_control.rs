//! The main control loop, its command verbs, and the `\halign`/`\valign`
//! alignment family.

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
use crate::consts::{
    ACC_KERN, ACTIVE_MATH_CHAR, A_LEADERS, BEGIN_M_CODE, BIGGEST_CHAR, BIGGEST_USV, BOX_BASE,
    BOX_NODE_SIZE, CAT_CODE_BASE, CHAR_CLASS_LIMIT, COND_MATH_GLUE, COUNT_BASE, CS_TOKEN_FLAG,
    CUR_FONT_LOC, DEL_CODE_BASE, DIMEN_BASE, DIMEN_PAR_BOX_MAX_DEPTH, DIMEN_PAR_DISPLAY_INDENT,
    DIMEN_PAR_HANG_INDENT, DIMEN_PAR_HSIZE, DIMEN_PAR_OVERFULL_RULE, DIMEN_PAR_PAR_INDENT,
    DIMEN_PAR_SPLIT_MAX_DEPTH, DLIST, EMPTY, END_M_CODE, ETEX_PEN_BASE, EXPLICIT, FONT_BASE,
    FONT_ID_BASE, FROZEN_CR, FROZEN_END_GROUP, FROZEN_END_TEMPLATE, FROZEN_RIGHT, GLUE_BASE,
    GLUE_PAR_ABOVE_DISPLAY_SKIP, GLUE_PAR_BELOW_DISPLAY_SKIP, GLUE_PAR_PAR_SKIP,
    GLUE_PAR_SPACE_SKIP, GLUE_PAR_SPLIT_TOP_SKIP, GLUE_PAR_TAB_SKIP, GLUE_PAR_XSPACE_SKIP,
    HASH_BASE, HMODE, IGNORE_DEPTH, INT_BASE, INT_PAR_FLOATING_PENALTY, INT_PAR_GLOBAL_DEFS,
    INT_PAR_HANG_AFTER, INT_PAR_LANGUAGE, INT_PAR_LEFT_HYPHEN_MIN, INT_PAR_LOOSENESS,
    INT_PAR_POST_DISPLAY_PENALTY, INT_PAR_PRE_DISPLAY_PENALTY, INT_PAR_RIGHT_HYPHEN_MIN,
    INT_PAR_TEXXET, INT_PAR_TRACING_COMMANDS, INT_PAR_TRACING_LOST_CHARS, INT_PAR_XETEX_DASH_BREAK,
    INT_PAR_XETEX_INTERWORD_SPACE_SHAPING, INT_PAR_XETEX_INTER_CHAR_TOKENS, INT_PAR_XETEX_UPWARDS,
    INT_PAR_XETEX_USE_GLYPH_METRICS, LEFT_SIDE, LOCAL_BASE, LOCAL_EVERY_CR, LOCAL_EVERY_HBOX,
    LOCAL_EVERY_JOB, LOCAL_EVERY_PAR, LOCAL_EVERY_VBOX, LOCAL_OUTPUT_ROUTINE, LOCAL_PAR_SHAPE,
    LOCAL_XETEX_INTER_CHAR_TOKS, MARGIN_KERN_NODE_SIZE, MATH_CODE_BASE, MAX_HALFWORD, MMODE,
    MU_GLUE, MU_SKIP_BASE, NATIVE_NODE_SIZE, NOAD_SIZE, NON_ADDRESS, NORMAL, NULL_CS, NULL_FLAG,
    PRIM_EQTB_BASE, RIGHT_SIDE, SCALED_BASE, SELECTOR_NEW_STRING, SELECTOR_NO_PRINT,
    SELECTOR_TERM_ONLY, SF_CODE_BASE, SINGLE_BASE, SKIP_BASE, SMALL_NODE_SIZE, SPACE_ADJUSTMENT,
    SUB_BOX, SUB_MLIST, TEX_INFINITY, TEX_NULL, TOKS_BASE, TOO_BIG_CHAR, TOO_BIG_USV,
    UNDEFINED_PRIMITIVE, VMODE,
};
use crate::errors::Abort;
use crate::ext::{append_src_special, do_extension};
use crate::fonts::{
    read_font_info, EXTRA_SPACE_CODE, LIG_TAG, SLANT_CODE, SPACE_CODE, X_HEIGHT_CODE,
};
use crate::hyphenation::{new_hyph_exceptions, new_patterns};
use crate::input::{
    BACKED_UP, BACKED_UP_CHAR, EVERY_CR_TEXT, EVERY_HBOX_TEXT, EVERY_JOB_TEXT, EVERY_PAR_TEXT,
    EVERY_VBOX_TEXT, INSERTED, INTER_CHAR_TEXT, OUTPUT_TEXT, TOKEN_LIST, U_TEMPLATE, V_TEMPLATE,
};
use crate::math::{after_math, init_math};
use crate::mem::{
    ADJUST_HEAD, ALIGN_HEAD, CONTRIB_HEAD, END_SPAN, FILL_GLUE, FIL_GLUE, FIL_NEG_GLUE, HOLD_HEAD,
    PAGE_HEAD, PRE_ADJUST_HEAD, SS_GLUE, TEMP_HEAD, ZERO_GLUE,
};
use crate::mem_types::ListStateRecord;
use crate::node::{
    AdjustNode, AlignColumn, AlignRecord, AlignStackNode, AnyBox, AnyNode, BoxOrRule, CharNode,
    DiscNode, GlueNode, GlueSpecNode, InsNode, LigItem, MarkNode, MathField, NodePtr, ParShapeNode,
    PenaltiesNode, RuleDimen, SaNode, SpanNode, UnsetNode, WhatsitNode, WhatsitSubtype,
};
use crate::packaging::ADDITIONAL;
use crate::scan::{
    fetch_box, set_class_fn, set_family_fn, ALIGNING, BOX_VAL, DIMEN_VAL, GLUE_VAL, INTER_CHAR_VAL,
    INT_VAL, LEFT_BRACE_TOKEN, MATH_SHIFT_TOKEN, MAX_CHAR_VAL, MU_VAL, NORMAL_SCANNER, OTHER_TOKEN,
    PROTECTED_TOKEN, RIGHT_BRACE_TOKEN, TAB_TOKEN, TOK_VAL,
};
use crate::state::{EngineState, GroupCode, MemState};
use crate::units::{FontId, GlueSign, MemIdx, MemPtr, NodeType, Scaled};

use crate::layout::set_cp_code;

const INNER_VMODE: i16 = -VMODE;
const INNER_HMODE: i16 = -HMODE;

const TAB_SKIP_SUBTYPE: u16 = GLUE_PAR_TAB_SKIP as u16 + 1;

const GLUE_BASE_TAB_SKIP: i32 = GLUE_BASE + GLUE_PAR_TAB_SKIP as i32;

// `cur_chr` modifiers.
pub(crate) const BOX_CODE: i32 = 0;
pub(crate) const COPY_CODE: i32 = 1;
pub(crate) const LAST_BOX_CODE: i32 = 2;
pub(crate) const VSPLIT_CODE: i32 = 3;
pub(crate) const VTOP_CODE: i32 = 4;
pub(crate) const FIL_CODE: i32 = 0;
pub(crate) const FILL_CODE: i32 = 1;
pub(crate) const SS_CODE: i32 = 2;
pub(crate) const FIL_NEG_CODE: i32 = 3;
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

pub(crate) fn align_error(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.align_state.abs() > 2 {
        crate::output::error_here_with_diagnostic(st, "Misplaced ");
        crate::output::print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
        crate::output::capture_to_diagnostic(st, None);
        if st.scanner.cur_tok == TAB_TOKEN + 38 {
            st.errors.help(&[
                "I can't figure out why you would want to use a tab mark",
                "here. If you just want an ampersand, the remedy is",
                r"simple: Just type `I\&' now. But if some right brace",
                "up above has ended a previous alignment prematurely,",
                "you're probably due for more error messages, and you",
                "might try typing `S' now just to see what is salvageable.",
            ]);
        } else {
            st.errors.help(&[
                "I can't figure out why you would want to use a tab mark",
                r"or \cr or \span just now. If something like a right brace",
                "up above has ended a previous alignment prematurely,",
                "you're probably due for more error messages, and you",
                "might try typing `S' now just to see what is salvageable.",
            ]);
        }
        crate::errors::error(st)?;
    } else {
        crate::input::back_input(st)?;
        if st.scanner.align_state < 0 {
            crate::output::print_err(st, "Missing { inserted");
            st.scanner.align_state += 1;
            st.scanner.cur_tok = LEFT_BRACE_TOKEN + 123;
        } else {
            crate::output::print_err(st, "Missing } inserted");
            st.scanner.align_state -= 1;
            st.scanner.cur_tok = RIGHT_BRACE_TOKEN + 125;
        }
        st.errors.help(&[
            "I've put in what seems to be necessary to fix",
            "the current column of the current alignment.",
            "Try to go on, since this might almost work.",
        ]);
        crate::input::ins_error(st)?;
    }
    Ok(())
}
pub(crate) fn no_align_error(st: &mut EngineState) -> Result<(), Abort> {
    crate::output::error_here_with_diagnostic(st, "Misplaced ");
    crate::output::print_esc(st, "noalign");
    crate::output::capture_to_diagnostic(st, None);
    st.errors.help(&[
        r"I expect to see \noalign only after the \cr of",
        "an alignment. Proceed, and I'll ignore this case.",
    ]);
    crate::errors::error(st)
}
pub(crate) fn omit_error(st: &mut EngineState) -> Result<(), Abort> {
    crate::output::error_here_with_diagnostic(st, "Misplaced ");
    crate::output::print_esc(st, "omit");
    crate::output::capture_to_diagnostic(st, None);
    st.errors.help(&[
        r"I expect to see \omit only after tab marks or the \cr of",
        "an alignment. Proceed, and I'll ignore this case.",
    ]);
    crate::errors::error(st)
}
pub(crate) fn fin_align(st: &mut EngineState) -> Result<(), Abort> {
    if st.save_stack.cur_group != GroupCode::Align {
        return Err(Abort::Confusion("align1"));
    }
    crate::save::unsave(st)?;
    if st.save_stack.cur_group != GroupCode::Align {
        return Err(Abort::Confusion("align0"));
    }
    crate::save::unsave(st)?;

    let in_display = st.input.nest[(st.input.nest_ptr - 1) as usize].mode == MMODE;
    let o = if in_display {
        st.eqtb.dimen_par(DIMEN_PAR_DISPLAY_INDENT)
    } else {
        0
    };

    let mut column = st.mem.first_column::<AlignRecord>();
    while let Some(q) = column {
        let (u_part, v_part) = (q.u_part(&st.mem), q.v_part(&st.mem));
        st.mem.flush_list(u_part);
        st.mem.flush_list(v_part);
        let next = q.next_column(&st.mem);
        if q.width(&st.mem) == Scaled(NULL_FLAG) {
            q.set_width(&mut st.mem, Scaled(0));
            let glue = q.tab_skip(&st.mem);
            let spec = glue.glue_ptr(&st.mem);
            if spec != ZERO_GLUE {
                st.mem.add_glue_ref(ZERO_GLUE);
                st.mem.delete_glue_ref(spec);
                glue.set_glue_ptr(&mut st.mem, ZERO_GLUE);
            }
        }
        if let Some(next) = next {
            if q.span_head().next(&st.mem) != END_SPAN {
                st.mem.merge_spans(q, next);
            }
        }
        q.into_unset(&mut st.mem);
        column = next;
    }

    st.save_stack.save_ptr -= 2;
    st.scanner.pack_begin_line = -st.input.cur_list.mode_line;
    let spec_size = st.save_stack.saved(1);
    let spec_mode = st.save_stack.saved(0) as i16;
    let head = ALIGN_HEAD.link(&st.mem);
    let p = if st.input.cur_list.mode == INNER_VMODE {
        let rule_save = st.eqtb.dimen_par(DIMEN_PAR_OVERFULL_RULE);
        st.eqtb.set_dimen_par(DIMEN_PAR_OVERFULL_RULE, 0);
        let packed = crate::packaging::hpack(st, head, spec_size, spec_mode)?;
        st.eqtb.set_dimen_par(DIMEN_PAR_OVERFULL_RULE, rule_save);
        packed
    } else {
        st.mem.for_each_preamble_column(|mem, q| {
            q.set_height(mem, q.width(mem));
            q.set_width(mem, Scaled(0));
        });
        let packed = crate::packaging::vpackage(st, head, spec_size, spec_mode, MAX_HALFWORD)?;
        st.mem.for_each_preamble_column(|mem, q| {
            q.set_width(mem, q.height(mem));
            q.set_height(mem, Scaled(0));
        });
        packed
    };
    st.scanner.pack_begin_line = 0;

    let mut s = st.input.cur_list.head;
    let mut q = s.link(&st.mem);
    while !q.is_null() {
        match st.mem.classify_non_char(q) {
            Some(AnyNode::Unset(row)) => {
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

                let mut r = row.list_ptr(&st.mem).link(&st.mem);
                let mut proto_column =
                    NodePtr::<UnsetNode>::after_tab_skip(&st.mem, p.list_ptr(&st.mem));
                loop {
                    let (Ok(AnyNode::Unset(col)), Some(mut proto)) =
                        (st.mem.classify(MemIdx(r.0)), proto_column)
                    else {
                        return Err(Abort::Confusion("align"));
                    };
                    let n = col.subtype(&st.mem);
                    let w = proto.width(&st.mem);
                    let mut t = w.0;
                    let mut u = HOLD_HEAD;
                    col.set_subtype(&mut st.mem, 0);
                    for _ in 0..n {
                        let spec = proto.tab_skip(&st.mem).glue_ptr(&st.mem);
                        let g = st.mem.new_glue(spec)?;
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
                        let Some(spanned) = proto.next_column(&st.mem) else {
                            return Err(Abort::Confusion("align"));
                        };
                        proto = spanned;
                        let nb = st.mem.new_null_box()?;
                        u.set_link(&mut st.mem, nb.ptr());
                        u = nb.ptr();
                        let span_w = proto.width(&st.mem);
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
                        let glue = st.mem.unset_glue(col, t, col.width(&st.mem).0);
                        let b = col.into_hlist(&mut st.mem);
                        b.set_width(&mut st.mem, w);
                        (AnyBox::HList(b), glue)
                    } else {
                        let width = row.width(&st.mem);
                        col.set_width(&mut st.mem, width);
                        let glue = st.mem.unset_glue(col, t, col.height(&st.mem).0);
                        let b = col.into_vlist(&mut st.mem);
                        b.set_height(&mut st.mem, w);
                        (AnyBox::VList(b), glue)
                    };
                    let (b, (sign, order, set)) = col;
                    b.set_glue_sign(&mut st.mem, sign);
                    b.set_glue_order(&mut st.mem, order);
                    b.set_glue_set(&mut st.mem, set);
                    b.set_shift_amount(&mut st.mem, Scaled(0));
                    if u != HOLD_HEAD {
                        let (rest, spanned) = (r.link(&st.mem), HOLD_HEAD.link(&st.mem));
                        u.set_link(&mut st.mem, rest);
                        r.set_link(&mut st.mem, spanned);
                        r = u;
                    }
                    r = r.link(&st.mem).link(&st.mem);
                    proto_column = proto.next_column(&st.mem);
                    if r.is_null() {
                        break;
                    }
                }
            }
            Some(AnyNode::Rule(rule)) => {
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
                    let boxed = crate::packaging::hpack(st, rule.ptr(), 0, ADDITIONAL)?;
                    boxed.set_shift_amount(&mut st.mem, Scaled(o));
                    boxed.set_link(&mut st.mem, next);
                    s.set_link(&mut st.mem, boxed.ptr());
                    q = boxed.ptr();
                }
            }
            _ => {}
        }
        s = q;
        q = q.link(&st.mem);
    }

    st.mem.flush_node_list(p.idx().0)?;
    pop_alignment(st);

    let aux_save_s0 = st.input.cur_list.aux_s0();
    let aux_save_s1 = st.input.cur_list.aux_s1();
    let rows = st.input.cur_list.head.link(&st.mem);
    let rows_tail = st.input.cur_list.tail;
    st.mem.pop_nest(&mut st.input);

    if st.input.cur_list.mode == MMODE {
        do_assignments(st)?;
        if st.scanner.cur_cmd != MATH_SHIFT {
            crate::output::print_err(st, "Missing $$ inserted");
            st.errors.help(&[
                r"Displays can use special alignments (like \eqalignno)",
                "only if nothing but the alignment itself is between $$'s.",
            ]);
            crate::input::back_error(st)?;
        } else {
            crate::math::check_display_end(st)?;
        }
        let etex_aux = st.input.cur_list.e_tex_aux.0;
        st.mem.flush_node_list(etex_aux)?;
        st.mem.pop_nest(&mut st.input);
        let pre_penalty = st.eqtb.int_par(INT_PAR_PRE_DISPLAY_PENALTY);
        let pen = st.mem.new_penalty(pre_penalty)?;
        st.input.cur_list.tail_append(&mut st.mem, pen.ptr());
        let above = st
            .mem
            .new_param_glue(&st.eqtb, GLUE_PAR_ABOVE_DISPLAY_SKIP)?;
        st.input.cur_list.tail_append(&mut st.mem, above.ptr());
        st.input.cur_list.tail.set_link(&mut st.mem, rows);
        if !rows.is_null() {
            st.input.cur_list.tail = rows_tail;
        }
        let post_penalty = st.eqtb.int_par(INT_PAR_POST_DISPLAY_PENALTY);
        let pen = st.mem.new_penalty(post_penalty)?;
        st.input.cur_list.tail_append(&mut st.mem, pen.ptr());
        let below = st
            .mem
            .new_param_glue(&st.eqtb, GLUE_PAR_BELOW_DISPLAY_SKIP)?;
        st.input.cur_list.tail_append(&mut st.mem, below.ptr());
        st.input.cur_list.set_aux_s1(aux_save_s1);
        crate::math::resume_after_display(st)?;
    } else {
        st.input.cur_list.set_aux_s0(aux_save_s0);
        st.input.cur_list.set_aux_s1(aux_save_s1);
        st.input.cur_list.tail.set_link(&mut st.mem, rows);
        if !rows.is_null() {
            st.input.cur_list.tail = rows_tail;
        }
        if st.input.cur_list.mode == VMODE {
            crate::pagebuilder::build_page(st)?;
        }
    }
    Ok(())
}

impl MemState {
    fn first_column<K: AlignColumn>(&self) -> Option<NodePtr<K>> {
        NodePtr::after_tab_skip(self, ALIGN_HEAD.link(self))
    }

    fn for_each_preamble_column(&mut self, mut f: impl FnMut(&mut MemState, NodePtr<UnsetNode>)) {
        let mut column = self.first_column();
        while let Some(q) = column {
            f(self, q);
            column = q.next_column(self);
        }
    }

    /// Folds column `q`'s span widths, less its own width and tabskip, into `next`'s.
    fn merge_spans(&mut self, q: NodePtr<AlignRecord>, next: NodePtr<AlignRecord>) {
        let t = q.width(self).0 + q.tab_skip(self).glue_ptr(self).width(self).0;
        let mut r = q.span_head().next(self);
        let mut s = END_SPAN;
        s.set_next(self, next.span_head());
        let mut n = 1;
        loop {
            r.set_width(self, Scaled(r.width(self).0 - t));
            let u = r.next(self);
            while r.span_count(self) > n {
                s = s.next(self);
                n = s.next(self).span_count(self) + 1;
            }
            if r.span_count(self) < n {
                r.set_next(self, s.next(self));
                s.set_next(self, r);
                r.set_span_count(self, r.span_count(self) - 1);
                s = r;
            } else {
                let merged = s.next(self);
                if r.width(self) > merged.width(self) {
                    merged.set_width(self, r.width(self));
                }
                r.free(self);
            }
            r = u;
            if r == END_SPAN {
                break;
            }
        }
    }

    /// The glue setting of unset node `col`, of natural size `natural`, when set to size `t`.
    fn unset_glue(&self, col: NodePtr<UnsetNode>, t: i32, natural: i32) -> (GlueSign, u16, f64) {
        if t == natural {
            return (GlueSign::Normal, NORMAL, 0.0);
        }
        if t > natural {
            let stretch = col.glue_stretch(self).0;
            let ratio = if stretch == 0 {
                0.0
            } else {
                (t - natural) as f64 / stretch as f64
            };
            return (GlueSign::Stretching, col.glue_order(self), ratio);
        }
        let order = col.glue_sign(self);
        let shrink = col.glue_shrink(self).0;
        let ratio = if shrink == 0 {
            0.0
        } else if order == NORMAL && natural - t > shrink {
            1.0
        } else {
            (natural - t) as f64 / shrink as f64
        };
        (GlueSign::Shrinking, order, ratio)
    }
}

pub(crate) fn align_peek(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        st.scanner.align_state = 1_000_000;
        get_x_non_blank_or_protected(st)?;
        match st.scanner.cur_cmd {
            NO_ALIGN => {
                crate::scan::scan_left_brace(st)?;
                st.save_stack
                    .new_save_level(GroupCode::NoAlign, st.mem.line)?;
                if st.input.cur_list.mode == INNER_VMODE {
                    normal_paragraph(st)?;
                }
                return Ok(());
            }
            RIGHT_BRACE => return fin_align(st),
            CAR_RET if st.scanner.cur_chr == CR_CR_CODE => {}
            _ => {
                init_row(st)?;
                return init_col(st);
            }
        }
    }
}

fn get_x_non_blank_or_protected(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        crate::scan::get_x_or_protected(st)?;
        if st.scanner.cur_cmd != SPACER {
            return Ok(());
        }
    }
}

pub(crate) fn get_preamble_token(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        crate::scan::get_token(st)?;
        while st.scanner.cur_chr == SPAN_CODE && st.scanner.cur_cmd == TAB_MARK {
            crate::scan::get_token(st)?;
            if st.scanner.cur_cmd > MAX_COMMAND {
                crate::expand::expand(st)?;
                crate::scan::get_token(st)?;
            }
        }
        if st.scanner.cur_cmd == ENDV {
            return Err(Abort::Fatal(
                "(interwoven alignment preambles are not allowed)",
            ));
        }
        if st.scanner.cur_cmd != ASSIGN_GLUE || st.scanner.cur_chr != GLUE_BASE_TAB_SKIP {
            return Ok(());
        }
        crate::scan::scan_optional_equals(st)?;
        crate::scan::scan_glue(st, GLUE_VAL)?;
        let global = st.eqtb.int_par(INT_PAR_GLOBAL_DEFS) > 0;
        define(st, global, GLUE_BASE_TAB_SKIP, GLUE_REF, st.scanner.cur_val)?;
    }
}

pub(crate) fn init_span(st: &mut EngineState, p: MemPtr) -> Result<(), Abort> {
    crate::mem::push_nest(st)?;
    if st.input.cur_list.mode == INNER_HMODE {
        st.input.cur_list.set_aux_s0(1000);
    } else {
        st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
        normal_paragraph(st)?;
    }
    st.scanner.cur_span = p;
    Ok(())
}

pub(crate) fn init_row(st: &mut EngineState) -> Result<(), Abort> {
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = -HMODE - VMODE - st.input.cur_list.mode;
    if st.input.cur_list.mode == INNER_HMODE {
        st.input.cur_list.set_aux_s0(0);
    } else {
        st.input.cur_list.set_aux_s1(0);
    }
    let tab_skip = NodePtr::<GlueNode>::tab_skip_after(&st.mem, ALIGN_HEAD);
    let spec = tab_skip.glue_ptr(&st.mem);
    let glue = st.mem.new_glue(spec)?;
    st.input.cur_list.tail_append(&mut st.mem, glue.ptr());
    glue.set_subtype(&mut st.mem, TAB_SKIP_SUBTYPE);
    st.scanner.cur_align = tab_skip.link(&st.mem);
    st.scanner.cur_tail = st.scanner.cur_head;
    st.scanner.cur_pre_tail = st.scanner.cur_pre_head;
    init_span(st, st.scanner.cur_align)
}

pub(crate) fn init_col(st: &mut EngineState) -> Result<(), Abort> {
    let column = NodePtr::<AlignRecord>::at(st.scanner.cur_align);
    column.set_extra_info(&mut st.mem, st.scanner.cur_cmd as i32);
    if st.scanner.cur_cmd == OMIT {
        st.scanner.align_state = 0;
    } else {
        crate::input::back_input(st)?;
        crate::input::begin_token_list(st, column.u_part(&st.mem), U_TEMPLATE)?;
    }
    Ok(())
}

pub(crate) fn push_alignment(st: &mut EngineState) -> Result<(), Abort> {
    let p = NodePtr::<AlignStackNode>::alloc(&mut st.mem)?;
    let (mem, sc) = (&mut st.mem, &st.scanner);
    p.set_link(mem, sc.align_ptr);
    p.set_cur_align(mem, sc.cur_align);
    p.set_preamble(mem, ALIGN_HEAD.link(mem));
    p.set_cur_span(mem, sc.cur_span);
    p.set_cur_loop(mem, sc.cur_loop);
    p.set_align_state(mem, sc.align_state);
    p.set_cur_head(mem, sc.cur_head);
    p.set_cur_tail(mem, sc.cur_tail);
    p.set_cur_pre_head(mem, sc.cur_pre_head);
    p.set_cur_pre_tail(mem, sc.cur_pre_tail);
    st.scanner.align_ptr = p.ptr();
    st.scanner.cur_head = st.mem.get_avail()?;
    st.scanner.cur_pre_head = st.mem.get_avail()?;
    Ok(())
}

pub(crate) fn pop_alignment(st: &mut EngineState) {
    st.mem.free_avail(st.scanner.cur_head);
    st.mem.free_avail(st.scanner.cur_pre_head);
    let p = NodePtr::<AlignStackNode>::at(st.scanner.align_ptr);
    let (mem, sc) = (&mut st.mem, &mut st.scanner);
    sc.cur_tail = p.cur_tail(mem);
    sc.cur_head = p.cur_head(mem);
    sc.cur_pre_tail = p.cur_pre_tail(mem);
    sc.cur_pre_head = p.cur_pre_head(mem);
    sc.align_state = p.align_state(mem);
    sc.cur_loop = p.cur_loop(mem);
    sc.cur_span = p.cur_span(mem);
    ALIGN_HEAD.set_link(mem, p.preamble(mem));
    sc.cur_align = p.cur_align(mem);
    sc.align_ptr = p.link(mem);
    p.free(mem);
}

impl MemState {
    /// Copies token list `r` through `HOLD_HEAD` and returns the copy.
    fn copy_token_list(&mut self, mut r: MemPtr) -> Result<MemPtr, Abort> {
        let mut q = HOLD_HEAD;
        while !r.is_null() {
            self.store_new_token(&mut q, r.token(self))?;
            r = r.link(self);
        }
        q.set_link(self, MemPtr::NULL);
        Ok(HOLD_HEAD.link(self))
    }

    /// A fresh preamble column with an empty span list and no entries yet.
    fn new_align_record(&mut self) -> Result<NodePtr<AlignRecord>, Abort> {
        let nb = self.new_null_box()?;
        let column = NodePtr::<AlignRecord>::at(nb.idx());
        column.span_head().set_next(self, END_SPAN);
        column.set_width(self, Scaled(NULL_FLAG));
        Ok(column)
    }
}

pub(crate) fn fin_col(st: &mut EngineState) -> Result<bool, Abort> {
    if st.scanner.cur_align.is_null() {
        return Err(Abort::Confusion("endv"));
    }
    let cur_align = NodePtr::<AlignRecord>::at(st.scanner.cur_align);
    let q = cur_align.link(&st.mem);
    if q.is_null() {
        return Err(Abort::Confusion("endv"));
    }
    if st.scanner.align_state < 500_000 {
        return Err(Abort::Fatal(
            "(interwoven alignment preambles are not allowed)",
        ));
    }
    let mut p = q.link(&st.mem);
    if p.is_null() && cur_align.extra_info(&st.mem) < CR_CODE {
        if !st.scanner.cur_loop.is_null() {
            let column = st.mem.new_align_record()?;
            q.set_link(&mut st.mem, column.ptr());
            p = column.ptr();
            let template = NodePtr::<AlignRecord>::at(st.scanner.cur_loop.link(&st.mem));
            let u_template = template.u_part(&st.mem);
            let u_part = st.mem.copy_token_list(u_template)?;
            column.set_u_part(&mut st.mem, u_part);
            let v_template = template.v_part(&st.mem);
            let v_part = st.mem.copy_token_list(v_template)?;
            column.set_v_part(&mut st.mem, v_part);
            let tab_skip = template.tab_skip(&st.mem);
            st.scanner.cur_loop = tab_skip.ptr();
            let spec = tab_skip.glue_ptr(&st.mem);
            let g = st.mem.new_glue(spec)?;
            column.set_link(&mut st.mem, g.ptr());
            g.set_subtype(&mut st.mem, TAB_SKIP_SUBTYPE);
        } else {
            crate::output::error_here_with_diagnostic(
                st,
                "Extra alignment tab has been changed to ",
            );
            crate::output::print_esc(st, "cr");
            crate::output::capture_to_diagnostic(st, None);
            st.errors.help(&[
                r"You have given more \span or & marks than there were",
                r"in the preamble to the \halign or \valign now in progress.",
                r"So I'll assume that you meant to type \cr instead.",
            ]);
            cur_align.set_extra_info(&mut st.mem, CR_CODE);
            crate::errors::error(st)?;
        }
    }
    if cur_align.extra_info(&st.mem) != SPAN_CODE {
        crate::save::unsave(st)?;
        st.save_stack
            .new_save_level(GroupCode::Align, st.mem.line)?;
        let head = st.input.cur_list.head.link(&st.mem);
        let (u, w) = if st.input.cur_list.mode == INNER_HMODE {
            st.scanner.adjust_tail = st.scanner.cur_tail;
            st.scanner.pre_adjust_tail = st.scanner.cur_pre_tail;
            let u = crate::packaging::hpack(st, head, 0, ADDITIONAL)?;
            st.scanner.cur_tail = st.scanner.adjust_tail;
            st.scanner.adjust_tail = MemPtr::NULL;
            st.scanner.cur_pre_tail = st.scanner.pre_adjust_tail;
            st.scanner.pre_adjust_tail = MemPtr::NULL;
            (u, u.width(&st.mem))
        } else {
            let u = crate::packaging::vpackage(st, head, 0, ADDITIONAL, 0)?;
            (u, u.height(&st.mem))
        };
        let mut n: i32 = 0;
        if st.scanner.cur_span != st.scanner.cur_align {
            let mut q = st.scanner.cur_span;
            loop {
                n += 1;
                q = q.link(&st.mem).link(&st.mem);
                if q == st.scanner.cur_align {
                    break;
                }
            }
            if n > u16::MAX as i32 {
                return Err(Abort::Confusion("too many spans"));
            }
            let mut q = NodePtr::<AlignRecord>::at(st.scanner.cur_span).span_head();
            while q.next(&st.mem).span_count(&st.mem) < n {
                q = q.next(&st.mem);
            }
            let next = q.next(&st.mem);
            if next.span_count(&st.mem) > n {
                let s = NodePtr::<SpanNode>::alloc(&mut st.mem)?;
                s.set_next(&mut st.mem, next);
                s.set_span_count(&mut st.mem, n);
                q.set_next(&mut st.mem, s);
                s.set_width(&mut st.mem, w);
            } else if next.width(&st.mem) < w {
                next.set_width(&mut st.mem, w);
            }
        } else if w > cur_align.width(&st.mem) {
            cur_align.set_width(&mut st.mem, w);
        }
        let u = u.into_unset(&mut st.mem);
        u.set_subtype(&mut st.mem, n as u16);
        let o = crate::packaging::glue_order(&st.scanner.total_stretch);
        u.set_glue_order(&mut st.mem, o);
        u.set_glue_stretch(&mut st.mem, Scaled(st.scanner.total_stretch[o as usize]));
        let o = crate::packaging::glue_order(&st.scanner.total_shrink);
        u.set_glue_sign(&mut st.mem, o);
        u.set_glue_shrink(&mut st.mem, Scaled(st.scanner.total_shrink[o as usize]));
        st.mem.pop_nest(&mut st.input);
        st.input.cur_list.tail_append(&mut st.mem, u.ptr());
        let spec = cur_align.tab_skip(&st.mem).glue_ptr(&st.mem);
        let g = st.mem.new_glue(spec)?;
        st.input.cur_list.tail_append(&mut st.mem, g.ptr());
        g.set_subtype(&mut st.mem, TAB_SKIP_SUBTYPE);
        if cur_align.extra_info(&st.mem) >= CR_CODE {
            return Ok(true);
        }
        init_span(st, p)?;
    }
    st.scanner.align_state = 1_000_000;
    get_x_non_blank_or_protected(st)?;
    st.scanner.cur_align = p;
    init_col(st)?;
    Ok(false)
}

fn append_list(st: &mut EngineState, head: MemPtr, tail: MemPtr) {
    let first = head.link(&st.mem);
    st.input.cur_list.tail.set_link(&mut st.mem, first);
    st.input.cur_list.tail = tail;
}

fn begin_every(st: &mut EngineState, local: i32, kind: u16) -> Result<(), Abort> {
    let list = st.eqtb.local(local);
    if !list.is_null() {
        crate::input::begin_token_list(st, list, kind)?;
    }
    Ok(())
}

pub(crate) fn fin_row(st: &mut EngineState) -> Result<(), Abort> {
    let head = st.input.cur_list.head.link(&st.mem);
    let p = if st.input.cur_list.mode == INNER_HMODE {
        let row = crate::packaging::hpack(st, head, 0, ADDITIONAL)?;
        st.mem.pop_nest(&mut st.input);
        if st.scanner.cur_pre_head != st.scanner.cur_pre_tail {
            append_list(st, st.scanner.cur_pre_head, st.scanner.cur_pre_tail);
        }
        st.input
            .cur_list
            .append_to_vlist(&mut st.mem, &st.eqtb, row)?;
        if st.scanner.cur_head != st.scanner.cur_tail {
            append_list(st, st.scanner.cur_head, st.scanner.cur_tail);
        }
        row
    } else {
        let row = crate::packaging::vpackage(st, head, 0, ADDITIONAL, MAX_HALFWORD)?;
        st.mem.pop_nest(&mut st.input);
        st.input.cur_list.tail_append(&mut st.mem, row.ptr());
        st.input.cur_list.set_aux_s0(1000);
        row
    };
    p.into_unset(&mut st.mem)
        .set_glue_stretch(&mut st.mem, Scaled(0));
    begin_every(st, LOCAL_EVERY_CR, EVERY_CR_TEXT)?;
    align_peek(st)
}

pub(crate) fn init_align(st: &mut EngineState) -> Result<(), Abort> {
    let save_cs_ptr = st.scanner.cur_cs;
    push_alignment(st)?;
    st.scanner.align_state = -1_000_000;

    if st.input.cur_list.mode == MMODE
        && (st.input.cur_list.tail != st.input.cur_list.head
            || st.input.cur_list.aux_s1() != TEX_NULL)
    {
        crate::output::error_here_with_diagnostic(st, "Improper ");
        crate::output::print_esc(st, "halign");
        crate::output::print(st, " inside $$'s");
        crate::output::capture_to_diagnostic(st, None);
        st.errors.help(&[
            r"Displays can use special alignments (like \eqalignno)",
            "only if nothing but the alignment itself is between $$'s.",
            "So I've deleted the formulas that preceded this alignment.",
        ]);
        crate::errors::error(st)?;
        st.input.cur_list.flush_math(&mut st.mem)?;
    }

    crate::mem::push_nest(st)?;
    if st.input.cur_list.mode == MMODE {
        st.input.cur_list.mode = INNER_VMODE;
        let prev_depth = st.input.nest[(st.input.nest_ptr - 2) as usize].aux_s1();
        st.input.cur_list.set_aux_s1(prev_depth);
    } else if st.input.cur_list.mode > 0 {
        st.input.cur_list.mode = -st.input.cur_list.mode;
    }

    crate::scan::scan_spec(st, GroupCode::Align, false)?;
    ALIGN_HEAD.set_link(&mut st.mem, MemPtr::NULL);
    st.scanner.cur_align = ALIGN_HEAD;
    st.scanner.cur_loop = MemPtr::NULL;
    st.scanner.scanner_status = ALIGNING;
    st.scanner.warning_index = save_cs_ptr;
    st.scanner.align_state = -1_000_000;

    loop {
        let g = st.mem.new_param_glue(&st.eqtb, GLUE_PAR_TAB_SKIP)?.ptr();
        st.scanner.cur_align.set_link(&mut st.mem, g);
        st.scanner.cur_align = g;
        if st.scanner.cur_cmd == CAR_RET {
            break;
        }

        let mut p = HOLD_HEAD;
        p.set_link(&mut st.mem, MemPtr::NULL);
        loop {
            get_preamble_token(st)?;
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
                    crate::output::print_err(st, "Missing # inserted in alignment preamble");
                    st.errors.help(&[
                        "There should be exactly one # between &'s, when an",
                        r"\halign or \valign is being set up. In this case you had",
                        "none, so I've put one in; maybe that will work.",
                    ]);
                    crate::input::back_error(st)?;
                    break;
                }
            } else if st.scanner.cur_cmd != SPACER || p != HOLD_HEAD {
                st.mem.store_new_token(&mut p, st.scanner.cur_tok)?;
            }
        }

        let column = st.mem.new_align_record()?;
        st.scanner.cur_align.set_link(&mut st.mem, column.ptr());
        st.scanner.cur_align = column.ptr();
        let u_part = HOLD_HEAD.link(&st.mem);
        column.set_u_part(&mut st.mem, u_part);

        p = HOLD_HEAD;
        p.set_link(&mut st.mem, MemPtr::NULL);
        loop {
            get_preamble_token(st)?;
            if (TAB_MARK..=CAR_RET).contains(&st.scanner.cur_cmd)
                && st.scanner.align_state == -1_000_000
            {
                break;
            }
            if st.scanner.cur_cmd == MAC_PARAM {
                crate::output::print_err(st, "Only one # is allowed per tab");
                st.errors.help(&[
                    "There should be exactly one # between &'s, when an",
                    r"\halign or \valign is being set up. In this case you had",
                    "more than one, so I'm ignoring all but the first.",
                ]);
                crate::errors::error(st)?;
                continue;
            }
            st.mem.store_new_token(&mut p, st.scanner.cur_tok)?;
        }
        st.mem
            .store_new_token(&mut p, CS_TOKEN_FLAG + FROZEN_END_TEMPLATE)?;
        let v_part = HOLD_HEAD.link(&st.mem);
        column.set_v_part(&mut st.mem, v_part);
    }

    st.scanner.scanner_status = NORMAL_SCANNER;
    st.save_stack
        .new_save_level(GroupCode::Align, st.mem.line)?;
    begin_every(st, LOCAL_EVERY_CR, EVERY_CR_TEXT)?;
    align_peek(st)
}

pub(crate) fn push_math(st: &mut EngineState, c: GroupCode) -> Result<(), Abort> {
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = -MMODE;
    st.input.cur_list.set_aux_s1(TEX_NULL);
    st.save_stack.new_save_level(c, st.mem.line)?;
    Ok(())
}

pub(crate) fn end_graf(st: &mut EngineState) -> Result<(), Abort> {
    if st.input.cur_list.mode != HMODE {
        return Ok(());
    }
    if st.input.cur_list.head == st.input.cur_list.tail {
        st.mem.pop_nest(&mut st.input);
    } else {
        crate::linebreak::line_break(st, false)?;
    }
    let lr_save = st.input.cur_list.e_tex_aux;
    if !lr_save.is_null() {
        st.mem.flush_list(lr_save);
        st.input.cur_list.e_tex_aux = MemPtr::NULL;
    }
    normal_paragraph(st)?;
    st.errors.error_count = 0;
    Ok(())
}

pub(crate) fn append_penalty(st: &mut EngineState) -> Result<(), Abort> {
    crate::scan::scan_int(st)?;
    let p = st.mem.new_penalty(st.scanner.cur_val)?;
    st.input.cur_list.tail_append(&mut st.mem, p.ptr());
    if st.input.cur_list.mode == VMODE {
        crate::pagebuilder::build_page(st)?;
    }
    Ok(())
}

pub(crate) fn normal_paragraph(st: &mut EngineState) -> Result<(), Abort> {
    for (loc, value) in [
        (INT_BASE + INT_PAR_LOOSENESS, 0),
        (DIMEN_BASE + DIMEN_PAR_HANG_INDENT, 0),
        (INT_BASE + INT_PAR_HANG_AFTER, 1),
    ] {
        if st.eqtb.int_at(loc) != value {
            word_define(st, false, loc, value)?;
        }
    }
    for loc in [LOCAL_BASE + LOCAL_PAR_SHAPE, ETEX_PEN_BASE] {
        if st.eqtb.equiv(loc) != TEX_NULL {
            define(st, false, loc, SHAPE_REF, TEX_NULL)?;
        }
    }
    Ok(())
}

pub(crate) fn its_all_over(st: &mut EngineState) -> Result<bool, Abort> {
    if !crate::errors::privileged(st)? {
        return Ok(false);
    }
    if st.shipout.page_tail == PAGE_HEAD
        && st.input.cur_list.head == st.input.cur_list.tail
        && st.shipout.dead_cycles == 0
    {
        return Ok(true);
    }
    crate::input::back_input(st)?;
    let b = st.mem.new_null_box()?;
    st.input.cur_list.tail_append(&mut st.mem, b.ptr());
    b.set_width(&mut st.mem, Scaled(st.eqtb.dimen_par(DIMEN_PAR_HSIZE)));
    let g = st.mem.new_glue(FILL_GLUE)?;
    st.input.cur_list.tail_append(&mut st.mem, g.ptr());
    let p = st.mem.new_penalty(NULL_FLAG)?;
    st.input.cur_list.tail_append(&mut st.mem, p.ptr());
    crate::pagebuilder::build_page(st)?;
    Ok(false)
}

pub(crate) fn indent_in_hmode(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.cur_chr <= 0 {
        return Ok(());
    }
    let b = st.mem.new_null_box()?;
    b.set_width(&mut st.mem, Scaled(st.eqtb.dimen_par(DIMEN_PAR_PAR_INDENT)));
    let p = if st.input.cur_list.mode.abs() == HMODE {
        st.input.cur_list.set_aux_s0(1000);
        b.ptr()
    } else {
        let q = st.mem.new_noad()?;
        q.nucleus().set_math_type(&mut st.mem, SUB_BOX);
        q.nucleus().set_info(&mut st.mem, b.ptr());
        q.ptr()
    };
    st.input.cur_list.tail_append(&mut st.mem, p);
    Ok(())
}

pub(crate) fn head_for_vmode(st: &mut EngineState) -> Result<(), Abort> {
    if st.input.cur_list.mode >= 0 {
        crate::input::back_input(st)?;
        st.scanner.cur_tok = st.input.par_token;
        crate::input::back_input(st)?;
        st.input.cur_input.index = INSERTED;
        return Ok(());
    }
    if st.scanner.cur_cmd != HRULE {
        return off_save(st);
    }
    crate::output::error_here_with_diagnostic(st, "You can't use `");
    crate::output::print_esc(st, "hrule");
    crate::output::print(st, "' here except with leaders");
    crate::output::capture_to_diagnostic(st, None);
    st.errors.help(&[
        "To put a horizontal rule in an hbox or an alignment,",
        r"you should use \leaders or \hrulefill (see The TeXbook).",
    ]);
    crate::errors::error(st)
}

pub(crate) fn begin_insert_or_adjust(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.cur_cmd == VADJUST {
        st.scanner.cur_val = 255;
    } else {
        crate::scan::scan_eight_bit_int(st)?;
        if st.scanner.cur_val == 255 {
            crate::output::error_here_with_diagnostic(st, "You can't ");
            crate::output::print_esc(st, "insert");
            crate::output::print_int(st, st.scanner.cur_val);
            crate::output::capture_to_diagnostic(st, None);
            st.errors
                .help(&[r"I'm changing to \insert0; box 255 is special."]);
            crate::errors::error(st)?;
            st.scanner.cur_val = 0;
        }
    }
    st.save_stack.set_saved(0, st.scanner.cur_val);
    let pre = st.scanner.cur_cmd == VADJUST && crate::scan::scan_keyword(st, "pre")?;
    st.save_stack.set_saved(1, pre as i32);
    st.save_stack.save_ptr += 2;
    st.save_stack
        .new_save_level(GroupCode::Insert, st.mem.line)?;
    crate::scan::scan_left_brace(st)?;
    normal_paragraph(st)?;
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = INNER_VMODE;
    st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
    Ok(())
}

pub(crate) fn do_endv(st: &mut EngineState) -> Result<(), Abort> {
    fn is_v_template_end(r: &crate::state::InputState) -> bool {
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
        return Err(Abort::Fatal(
            "(interwoven alignment preambles are not allowed)",
        ));
    }
    if st.save_stack.cur_group == GroupCode::Align {
        end_graf(st)?;
        if fin_col(st)? {
            fin_row(st)?;
        }
        Ok(())
    } else {
        off_save(st)
    }
}

pub(crate) fn unpackage(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.cur_chr > COPY_CODE {
        let disc = &mut st.scanner.disc_ptr[st.scanner.cur_chr as usize];
        st.input.cur_list.tail.set_link(&mut st.mem, *disc);
        *disc = MemPtr::NULL;
    } else {
        let c = st.scanner.cur_chr;
        crate::scan::scan_register_num(st)?;
        let Some(b) = fetch_box(st, st.scanner.cur_val) else {
            return Ok(());
        };
        let mode = st.input.cur_list.mode.abs();
        if mode == MMODE
            || (mode == VMODE && !matches!(b, AnyBox::VList(_)))
            || (mode == HMODE && !matches!(b, AnyBox::HList(_)))
        {
            crate::output::print_err(st, "Incompatible list can't be unboxed");
            st.errors.help(&[
                "Sorry, Pandora. (You sneaky devil.)",
                r"I refuse to unbox an \hbox in vertical mode or vice versa.",
                "And I can't open any boxes in math mode.",
            ]);
            return crate::errors::error(st);
        }
        let list = b.list_ptr(&st.mem);
        if c == COPY_CODE {
            let copy = st.mem.copy_node_list(list.0)?;
            st.input
                .cur_list
                .tail
                .set_link(&mut st.mem, MemPtr::from_raw(copy));
        } else {
            st.input.cur_list.tail.set_link(&mut st.mem, list);
            let n = st.scanner.cur_val;
            st.mem
                .change_box(&mut st.scanner, &mut st.eqtb, n, MemPtr::NULL);
            st.mem.free_node(b.ptr(), BOX_NODE_SIZE);
        }
    }

    loop {
        let r = st.input.cur_list.tail.link(&st.mem);
        if r.is_null() {
            return Ok(());
        }
        if !st.mem.is_char_node(r)
            && matches!(st.mem.classify(MemIdx(r.0)), Ok(AnyNode::MarginKern(_)))
        {
            let next = r.link(&st.mem);
            st.input.cur_list.tail.set_link(&mut st.mem, next);
            st.mem.free_node(r, MARGIN_KERN_NODE_SIZE);
        } else {
            st.input.cur_list.tail = r;
        }
    }
}

pub(crate) fn delete_last(st: &mut EngineState) -> Result<(), Abort> {
    if st.input.cur_list.mode == VMODE && st.input.cur_list.tail == st.input.cur_list.head {
        if st.scanner.cur_chr as u16 != NodeType::Glue.as_raw() || st.shipout.last_glue.is_some() {
            crate::errors::you_cant(st);
            let remedy = if st.scanner.cur_chr as u16 == NodeType::Kern.as_raw() {
                r"Try `I\kern-\lastkern' instead."
            } else if st.scanner.cur_chr as u16 != NodeType::Glue.as_raw() {
                "Perhaps you can make the output routine do it."
            } else {
                r"Try `I\vskip-\lastskip' instead."
            };
            st.errors.help(&[
                "Sorry...I usually can't take things from the current page.",
                remedy,
            ]);
            return crate::errors::error(st);
        }
        return Ok(());
    }
    let tx = effective_tail(st);
    if st.mem.is_char_node(tx)
        || st
            .mem
            .node_type(MemIdx::new(tx))
            .map_or(true, |t| t.as_raw() as i32 != st.scanner.cur_chr)
    {
        return Ok(());
    }
    if unlink_effective_tail(st, tx)? {
        st.mem.flush_node_list(tx)?;
    }
    Ok(())
}

/// Appends interword glue for the current space factor.
pub(crate) fn app_space(st: &mut EngineState) -> Result<(), Abort> {
    let space_factor = st.input.cur_list.aux_s0();
    let q;
    let mut side_effects = None;
    if space_factor >= 2000 && st.eqtb.glue_par(GLUE_PAR_XSPACE_SKIP) != ZERO_GLUE {
        q = st.mem.new_param_glue(&st.eqtb, GLUE_PAR_XSPACE_SKIP)?.ptr();
    } else {
        let space_skip = st.eqtb.glue_par(GLUE_PAR_SPACE_SKIP);
        let base = if space_skip != ZERO_GLUE {
            space_skip
        } else {
            font_space_glue(st)?
        };
        st.mainloop.main_p = base.ptr();
        let spec = st.mem.new_spec(base)?;
        st.mainloop.main_p = spec.ptr();
        if space_factor >= 2000 {
            let cf = st.eqtb.cur_font();
            let extra = st.fonts.param(cf, EXTRA_SPACE_CODE);
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
        q = st.mem.new_glue(spec)?.ptr();
        spec.set_ref_count(&mut st.mem, TEX_NULL);
    }
    st.input.cur_list.tail_append(&mut st.mem, q);
    if let Some((remainder, overflowed)) = side_effects {
        if overflowed {
            st.errors.arith_error = true;
        }
        st.errors.tex_remainder = remainder;
    }
    Ok(())
}

/// The current font's interword glue spec, built from its parameters on first use.
fn font_space_glue(st: &mut EngineState) -> Result<NodePtr<GlueSpecNode>, Abort> {
    let cf = st.eqtb.cur_font();
    let glue = st.fonts.font_glue(cf);
    if glue != TEX_NULL {
        return Ok(crate::node::NodePtr::from_glue_ptr(MemPtr(glue)));
    }
    let spec = st.mem.new_spec(ZERO_GLUE)?;
    st.mainloop.main_k = st.fonts.param_base(cf) + SPACE_CODE;
    let k = st.mainloop.main_k;
    spec.set_width(&mut st.mem, Scaled(st.fonts.font_info_sc(k)));
    spec.set_stretch(&mut st.mem, Scaled(st.fonts.font_info_sc(k + 1)));
    spec.set_shrink(&mut st.mem, Scaled(st.fonts.font_info_sc(k + 2)));
    st.fonts.set_font_glue(cf, spec.idx().0);
    Ok(spec)
}

pub(crate) fn scan_box(st: &mut EngineState, box_context: i32) -> Result<(), Abort> {
    crate::scan::get_x_non_blank_non_relax(st)?;
    if st.scanner.cur_cmd == MAKE_BOX {
        begin_box(st, box_context)
    } else if box_context >= LEADER_FLAG
        && (st.scanner.cur_cmd == HRULE || st.scanner.cur_cmd == VRULE)
    {
        st.scanner.cur_box = Some(BoxOrRule::Rule(crate::scan::scan_rule_spec(st)?));
        box_end(st, box_context)
    } else {
        crate::output::print_err(st, "A <box> was supposed to be here");
        st.errors.help(&[
            r"I was expecting to see \hbox or \vbox or \copy or \box or",
            "something like that. So you might find something missing in",
            "your output. But keep trying; you can fix this later.",
        ]);
        crate::input::back_error(st)
    }
}

pub(crate) fn package(st: &mut EngineState, c: i32) -> Result<(), Abort> {
    let d = st.eqtb.dimen_par(DIMEN_PAR_BOX_MAX_DEPTH);
    let u = st.eqtb.int_par(INT_PAR_XETEX_UPWARDS);
    crate::save::unsave(st)?;
    st.save_stack.save_ptr -= 3;
    let v = st.eqtb.int_par(INT_PAR_XETEX_UPWARDS);
    st.eqtb.set_int_par(INT_PAR_XETEX_UPWARDS, u);
    let list = st.input.cur_list.head.link(&st.mem);
    let size = st.save_stack.saved(2);
    let spec_mode = st.save_stack.saved(1) as i16;
    if st.input.cur_list.mode == INNER_HMODE {
        let b = crate::packaging::hpack(st, list, size, spec_mode)?;
        st.scanner.cur_box = Some(BoxOrRule::Box(b));
    } else {
        let b = crate::packaging::vpackage(st, list, size, spec_mode, d)?;
        st.scanner.cur_box = Some(BoxOrRule::Box(b));
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
    st.eqtb.set_int_par(INT_PAR_XETEX_UPWARDS, v);
    st.mem.pop_nest(&mut st.input);
    let box_context = st.save_stack.saved(0);
    box_end(st, box_context)
}

/// Sets `cur_lang` and the list's language and hyphen minima for a new paragraph.
pub(crate) fn start_paragraph_language(st: &mut EngineState) {
    let l = crate::helpers::norm_lang(st.eqtb.int_par(INT_PAR_LANGUAGE));
    st.hyph.cur_lang = l;
    st.input.cur_list.set_aux_s1(l as i32);
    let lhm = crate::helpers::norm_min(st.eqtb.int_par(INT_PAR_LEFT_HYPHEN_MIN)) as i32;
    let rhm = crate::helpers::norm_min(st.eqtb.int_par(INT_PAR_RIGHT_HYPHEN_MIN)) as i32;
    st.input.cur_list.prev_graf = (lhm * 64 + rhm) * 65536 + l as i32;
}

pub(crate) fn new_graf(st: &mut EngineState, indented: bool) -> Result<(), Abort> {
    st.input.cur_list.prev_graf = 0;
    if st.input.cur_list.mode == VMODE || st.input.cur_list.head != st.input.cur_list.tail {
        let g = st.mem.new_param_glue(&st.eqtb, GLUE_PAR_PAR_SKIP)?;
        st.input.cur_list.tail_append(&mut st.mem, g.ptr());
    }
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = HMODE;
    st.input.cur_list.set_aux_s0(1000);

    start_paragraph_language(st);

    if indented {
        let b = st.mem.new_null_box()?;
        st.input.cur_list.tail = b.ptr();
        st.input.cur_list.head.set_link(&mut st.mem, b.ptr());
        b.set_width(&mut st.mem, Scaled(st.eqtb.dimen_par(DIMEN_PAR_PAR_INDENT)));
        if st.shipout.insert_src_special_every_par {
            crate::ext::insert_src_special(st)?;
        }
    }
    begin_every(st, LOCAL_EVERY_PAR, EVERY_PAR_TEXT)?;
    if st.input.nest_ptr == 1 {
        crate::pagebuilder::build_page(st)?;
    }
    Ok(())
}

/// What `new_character` makes: a character node, or a glyph for a native font.
#[derive(Copy, Clone)]
pub(crate) enum NewChar {
    Char(NodePtr<CharNode>),
    Native(NodePtr<WhatsitNode>),
}

impl NewChar {
    pub(crate) fn ptr(self) -> MemPtr {
        match self {
            NewChar::Char(c) => c.ptr(),
            NewChar::Native(w) => w.ptr(),
        }
    }
}

/// TeX's `new_character`, `None` for a character the font lacks.
pub(crate) fn new_character(
    st: &mut EngineState,
    f: i32,
    c: u16,
) -> Result<Option<NewChar>, Abort> {
    if st.fonts.is_native_font(f) {
        return crate::fonts::new_native_character(st, f, c as u32)
            .map(|w| Some(NewChar::Native(w)));
    }
    let ec = st.fonts.effective_char(f, c) as u16;
    if st.fonts.font_bc_entry(f) <= ec && st.fonts.font_ec_entry(f) >= ec {
        let info = st
            .fonts
            .font_info_b16(st.fonts.char_base_entry(f) + ec as i32);
        if info.s3 > 0 {
            let p = st.mem.new_char(FontId(f), c)?;
            return Ok(Some(NewChar::Char(p)));
        }
    }
    crate::output::char_warning(st, f, c as i32)?;
    Ok(None)
}

pub(crate) fn box_end(st: &mut EngineState, box_context: i32) -> Result<(), Abort> {
    if box_context < BOX_FLAG {
        let b = match st.scanner.cur_box {
            None => return Ok(()),
            Some(BoxOrRule::Box(b)) => b,
            Some(BoxOrRule::Rule(_)) => unreachable!("only leaders take a rule"),
        };
        b.set_shift_amount(&mut st.mem, Scaled(box_context));
        if st.input.cur_list.mode.abs() == VMODE {
            if !st.scanner.pre_adjust_tail.is_null() {
                if st.scanner.pre_adjust_tail != PRE_ADJUST_HEAD {
                    append_list(st, PRE_ADJUST_HEAD, st.scanner.pre_adjust_tail);
                }
                st.scanner.pre_adjust_tail = MemPtr::NULL;
            }
            st.input
                .cur_list
                .append_to_vlist(&mut st.mem, &st.eqtb, b)?;
            if !st.scanner.adjust_tail.is_null() {
                if st.scanner.adjust_tail != ADJUST_HEAD {
                    append_list(st, ADJUST_HEAD, st.scanner.adjust_tail);
                }
                st.scanner.adjust_tail = MemPtr::NULL;
            }
            if st.input.cur_list.mode > 0 {
                crate::pagebuilder::build_page(st)?;
            }
        } else {
            let node = if st.input.cur_list.mode.abs() == HMODE {
                st.input.cur_list.set_aux_s0(1000);
                b.ptr()
            } else {
                let noad = st.mem.new_noad()?;
                noad.nucleus().set_math_type(&mut st.mem, SUB_BOX);
                noad.nucleus().set_info(&mut st.mem, b.ptr());
                noad.ptr()
            };
            st.input.cur_list.tail_append(&mut st.mem, node);
        }
    } else if box_context < SHIP_OUT_FLAG {
        let (n, global) = if box_context < GLOBAL_BOX_FLAG {
            (box_context - BOX_FLAG, false)
        } else {
            (box_context - GLOBAL_BOX_FLAG, true)
        };
        st.scanner.cur_val = n;
        let cb = st.scanner.cur_box.map_or(TEX_NULL, |b| b.ptr().0);
        if n < 256 {
            define(st, global, BOX_BASE + n, BOX_REF, cb)?;
        } else {
            let sa = st.mem.create_sa_element(&mut st.scanner, BOX_VAL, n)?;
            sa_define(st, global, sa.idx().0, cb)?;
        }
    } else if let Some(cb) = st.scanner.cur_box {
        if box_context == SHIP_OUT_FLAG {
            let BoxOrRule::Box(b) = cb else {
                unreachable!("only leaders take a rule");
            };
            crate::shipout::ship_out(st, b)?;
            return Ok(());
        }
        crate::scan::get_x_non_blank_non_relax(st)?;
        let vertical = st.input.cur_list.mode.abs() == VMODE;
        if (st.scanner.cur_cmd == HSKIP && !vertical) || (st.scanner.cur_cmd == VSKIP && vertical) {
            let g = append_glue(st)?;
            g.set_subtype(
                &mut st.mem,
                (box_context - (LEADER_FLAG - A_LEADERS as i32)) as u16,
            );
            g.set_leader_ptr(&mut st.mem, cb.ptr());
        } else {
            crate::output::print_err(st, "Leaders not followed by proper glue");
            st.errors.help(&[
                r"You should say `\leaders <box or rule><hskip or vskip>'.",
                "I found the <box or rule>, but there's no suitable",
                "<hskip or vskip>, so I'm ignoring these leaders.",
            ]);
            crate::input::back_error(st)?;
            st.mem.flush_node_list(cb.ptr())?;
        }
    }
    Ok(())
}

impl MemState {
    fn is_math_node(&self, p: MemPtr, subtype: u16) -> bool {
        !self.is_char_node(p)
            && matches!(self.classify(MemIdx::new(p)), Ok(AnyNode::Math(m)) if m.subtype(self) == subtype)
    }
}

/// The last node of the current list, skipping a trailing end-of-math node.
fn effective_tail(st: &EngineState) -> MemPtr {
    let tail = st.input.cur_list.tail;
    if !st.mem.is_math_node(tail, END_M_CODE) {
        return tail;
    }
    st.mem.predecessor(st.input.cur_list.head, tail)
}

/// Unlinks node `tx` from the current list, unless it hides in a
/// discretionary's replacement text. Returns whether it was unlinked.
fn unlink_effective_tail(st: &mut EngineState, tx: MemPtr) -> Result<bool, Abort> {
    let mut q = st.input.cur_list.head;
    let mut p = MemPtr::NULL;
    let mut r;
    let mut fm;
    loop {
        r = p;
        p = q;
        fm = false;
        if !st.mem.is_char_node(q) {
            match st.mem.classify(MemIdx::new(q)) {
                Ok(AnyNode::Disc(disc)) => {
                    for _ in 0..disc.subtype(&st.mem) {
                        p = p.link(&st.mem);
                    }
                    if p == tx {
                        return Ok(false);
                    }
                }
                Ok(AnyNode::Math(m)) => fm = m.subtype(&st.mem) == BEGIN_M_CODE,
                _ => {}
            }
        }
        q = p.link(&st.mem);
        if q == tx {
            break;
        }
    }
    let next = tx.link(&st.mem);
    p.set_link(&mut st.mem, next);
    tx.set_link(&mut st.mem, MemPtr::NULL);
    if next.is_null() {
        if fm {
            return Err(Abort::Confusion("tail1"));
        }
        st.input.cur_list.tail = p;
    } else if fm {
        st.input.cur_list.tail = r;
        r.set_link(&mut st.mem, MemPtr::NULL);
        st.mem.flush_node_list(p)?;
    }
    Ok(true)
}

pub(crate) fn begin_box(st: &mut EngineState, box_context: i32) -> Result<(), Abort> {
    match st.scanner.cur_chr {
        BOX_CODE => {
            crate::scan::scan_register_num(st)?;
            let n = st.scanner.cur_val;
            st.scanner.cur_box = fetch_box(st, n).map(BoxOrRule::Box);
            st.mem
                .change_box(&mut st.scanner, &mut st.eqtb, n, MemPtr::NULL);
        }
        COPY_CODE => {
            crate::scan::scan_register_num(st)?;
            st.scanner.cur_box = match fetch_box(st, st.scanner.cur_val) {
                Some(b) => Some(BoxOrRule::Box(st.mem.copy_box(b)?)),
                None => None,
            };
        }
        LAST_BOX_CODE => {
            st.scanner.cur_box = None;
            if st.input.cur_list.mode.abs() == MMODE {
                crate::errors::you_cant(st);
                st.errors.help(&[r"Sorry; this \lastbox will be void."]);
                crate::errors::error(st)?;
            } else if st.input.cur_list.mode == VMODE
                && st.input.cur_list.head == st.input.cur_list.tail
            {
                crate::errors::you_cant(st);
                st.errors.help(&[
                    "Sorry...I usually can't take things from the current page.",
                    r"This \lastbox will therefore be void.",
                ]);
                crate::errors::error(st)?;
            } else {
                let tx = effective_tail(st);
                if !st.mem.is_char_node(tx) {
                    if let Ok(Some(b)) = st.mem.classify_box(MemIdx::new(tx)) {
                        if unlink_effective_tail(st, tx)? {
                            st.scanner.cur_box = Some(BoxOrRule::Box(b));
                            b.set_shift_amount(&mut st.mem, Scaled(0));
                        }
                    }
                }
            }
        }
        VSPLIT_CODE => {
            crate::scan::scan_register_num(st)?;
            let n = st.scanner.cur_val;
            if !crate::scan::scan_keyword(st, "to")? {
                crate::output::print_err(st, "Missing `to' inserted");
                st.errors.help(&[
                    r"I'm working on `\vsplit<box number> to <dimen>';",
                    "will look for the <dimen> next.",
                ]);
                crate::errors::error(st)?;
            }
            crate::scan::scan_dimen(st, false, false, false)?;
            st.scanner.cur_box =
                crate::packaging::vsplit(st, n, st.scanner.cur_val)?.map(BoxOrRule::Box);
        }
        _ => {
            let k = st.scanner.cur_chr - VTOP_CODE as i32;
            st.save_stack.set_saved(0, box_context);
            let mode = if k == HMODE as i32 {
                let group = if box_context < BOX_FLAG && st.input.cur_list.mode.abs() == VMODE {
                    GroupCode::AdjustedHbox
                } else {
                    GroupCode::Hbox
                };
                crate::scan::scan_spec(st, group, true)?;
                HMODE
            } else {
                let group = if k == VMODE as i32 {
                    GroupCode::Vbox
                } else {
                    GroupCode::Vtop
                };
                crate::scan::scan_spec(st, group, true)?;
                normal_paragraph(st)?;
                VMODE
            };
            crate::mem::push_nest(st)?;
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
    box_end(st, box_context)
}

pub(crate) fn handle_right_brace(st: &mut EngineState) -> Result<(), Abort> {
    match st.save_stack.cur_group {
        GroupCode::Simple => crate::save::unsave(st)?,
        GroupCode::BottomLevel => {
            crate::output::print_err(st, "Too many }'s");
            st.errors.help(&[
                "You've closed more groups than you opened.",
                "Such booboos are generally harmless, so keep going.",
            ]);
            crate::errors::error(st)?;
        }
        GroupCode::SemiSimple | GroupCode::MathShift | GroupCode::MathLeft => {
            crate::errors::extra_right_brace(st)?;
        }
        GroupCode::Hbox => package(st, 0)?,
        GroupCode::AdjustedHbox => {
            st.scanner.adjust_tail = ADJUST_HEAD;
            st.scanner.pre_adjust_tail = PRE_ADJUST_HEAD;
            package(st, 0)?;
        }
        GroupCode::Vbox => {
            end_graf(st)?;
            package(st, 0)?;
        }
        GroupCode::Vtop => {
            end_graf(st)?;
            package(st, VTOP_CODE)?;
        }
        GroupCode::Insert => {
            end_graf(st)?;
            let split_top_skip = st.eqtb.glue_par(GLUE_PAR_SPLIT_TOP_SKIP);
            st.mem.add_glue_ref(split_top_skip);
            let d = st.eqtb.dimen_par(DIMEN_PAR_SPLIT_MAX_DEPTH);
            let f = st.eqtb.int_par(INT_PAR_FLOATING_PENALTY);
            crate::save::unsave(st)?;
            st.save_stack.save_ptr -= 2;
            let head = st.input.cur_list.head.link(&st.mem);
            let p = crate::packaging::vpackage(st, head, 0, ADDITIONAL, MAX_HALFWORD)?;
            st.mem.pop_nest(&mut st.input);
            let n = st.save_stack.saved(0);
            if n < 255 {
                let ins = NodePtr::<InsNode>::alloc(&mut st.mem)?;
                st.input.cur_list.tail_append(&mut st.mem, ins.ptr());
                ins.set_subtype(&mut st.mem, n as u16);
                let height = p.height(&st.mem).0 + p.depth(&st.mem).0;
                ins.set_height(&mut st.mem, Scaled(height));
                let list = p.list_ptr(&st.mem);
                ins.set_ins_ptr(&mut st.mem, list);
                ins.set_split_top_ptr(&mut st.mem, split_top_skip);
                ins.set_depth(&mut st.mem, Scaled(d));
                ins.set_float_cost(&mut st.mem, f);
            } else {
                let adj = NodePtr::<AdjustNode>::alloc(&mut st.mem)?;
                st.input.cur_list.tail_append(&mut st.mem, adj.ptr());
                let pre = st.save_stack.saved(1);
                adj.set_subtype(&mut st.mem, pre as u16);
                let list = p.list_ptr(&st.mem);
                adj.set_adjust_ptr(&mut st.mem, list);
                st.mem.delete_glue_ref(split_top_skip);
            }
            st.mem.free_node(p.ptr(), BOX_NODE_SIZE);
            if st.input.nest_ptr == 0 {
                crate::pagebuilder::build_page(st)?;
            }
        }
        GroupCode::Output => {
            if st.input.cur_input.loc != TEX_NULL
                || (st.input.cur_input.index != OUTPUT_TEXT
                    && st.input.cur_input.index != BACKED_UP)
            {
                crate::output::print_err(st, "Unbalanced output routine");
                st.errors.help(&[
                    "Your sneaky output routine has problematic {'s and/or }'s.",
                    "I can't handle that very well; good luck.",
                ]);
                crate::errors::error(st)?;
                loop {
                    crate::scan::get_token(st)?;
                    if st.input.cur_input.loc == TEX_NULL {
                        break;
                    }
                }
            }
            crate::input::end_token_list(st)?;
            end_graf(st)?;
            crate::save::unsave(st)?;
            st.shipout.output_active = false;
            st.shipout.insert_penalties = 0;

            if !st.eqtb.box_reg(255).is_null() {
                crate::output::error_here_with_diagnostic(st, "Output routine didn't use all of ");
                crate::output::print_esc(st, "box");
                crate::output::print_int(st, 255);
                crate::output::capture_to_diagnostic(st, None);
                st.errors.help(&[
                    r"Your \output commands should empty \box255,",
                    r"e.g., by saying `\shipout\box255'.",
                    "Proceed; I'll discard its present contents.",
                ]);
                crate::errors::box_error(st, 255)?;
            }

            if st.input.cur_list.tail != st.input.cur_list.head {
                let first = st.input.cur_list.head.link(&st.mem);
                st.shipout.page_tail.set_link(&mut st.mem, first);
                st.shipout.page_tail = st.input.cur_list.tail;
            }
            if !PAGE_HEAD.link(&st.mem).is_null() {
                if CONTRIB_HEAD.link(&st.mem).is_null() {
                    st.input.nest[0].tail = st.shipout.page_tail;
                }
                let contrib = CONTRIB_HEAD.link(&st.mem);
                st.shipout.page_tail.set_link(&mut st.mem, contrib);
                let page = PAGE_HEAD.link(&st.mem);
                CONTRIB_HEAD.set_link(&mut st.mem, page);
                PAGE_HEAD.set_link(&mut st.mem, MemPtr::NULL);
                st.shipout.page_tail = PAGE_HEAD;
            }

            let last_box = st.scanner.disc_ptr[LAST_BOX_CODE as usize];
            st.mem.flush_node_list(last_box.0)?;
            st.scanner.disc_ptr[LAST_BOX_CODE as usize] = MemPtr::NULL;
            st.mem.pop_nest(&mut st.input);
            crate::pagebuilder::build_page(st)?;
        }
        GroupCode::Disc => build_discretionary(st)?,
        GroupCode::Align => {
            crate::input::back_input(st)?;
            st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_CR;
            crate::output::error_here_with_diagnostic(st, "Missing ");
            crate::output::print_esc(st, "cr");
            crate::output::print(st, " inserted");
            crate::output::capture_to_diagnostic(st, None);
            st.errors
                .help(&["I'm guessing that you meant to end an alignment here."]);
            crate::input::ins_error(st)?;
        }
        GroupCode::NoAlign => {
            end_graf(st)?;
            crate::save::unsave(st)?;
            align_peek(st)?;
        }
        GroupCode::Vcenter => {
            end_graf(st)?;
            crate::save::unsave(st)?;
            st.save_stack.save_ptr -= 2;
            let head = st.input.cur_list.head.link(&st.mem);
            let size = st.save_stack.saved(1);
            let spec_mode = st.save_stack.saved(0) as i16;
            let p = crate::packaging::vpackage(st, head, size, spec_mode, MAX_HALFWORD)?;
            st.mem.pop_nest(&mut st.input);
            let noad = st.mem.new_noad()?;
            st.input.cur_list.tail_append(&mut st.mem, noad.ptr());
            noad.set_noad_type(&mut st.mem, NodeType::VcenterNoad);
            noad.nucleus().set_math_type(&mut st.mem, SUB_BOX);
            noad.nucleus().set_info(&mut st.mem, p.ptr());
        }
        GroupCode::MathChoice => crate::math::build_choices(st)?,
        GroupCode::Math => {
            crate::save::unsave(st)?;
            st.save_stack.save_ptr -= 1;
            let save_loc = NodePtr::<MathField>::at(st.save_stack.saved(0));
            save_loc.set_math_type(&mut st.mem, SUB_MLIST);
            let p = MemPtr(crate::math::fin_mlist(st, TEX_NULL)?);
            save_loc.set_info(&mut st.mem, p);
            if p.is_null() || !p.link(&st.mem).is_null() {
                return Ok(());
            }
            let Ok(AnyNode::Noad(noad)) = st.mem.classify(MemIdx::new(p)) else {
                return Ok(());
            };
            match noad.noad_type(&st.mem) {
                NodeType::OrdNoad
                    if noad.subscr().math_type(&st.mem) == EMPTY
                        && noad.supscr().math_type(&st.mem) == EMPTY =>
                {
                    save_loc.copy_from(&mut st.mem, noad.nucleus());
                    st.mem.free_node(noad.ptr(), NOAD_SIZE);
                }
                NodeType::AccentNoad
                    if save_loc.idx().0 == st.input.cur_list.tail.0 + 1
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
                    st.mem.free_node(st.input.cur_list.tail, NOAD_SIZE);
                    st.input.cur_list.tail = noad.ptr();
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub(crate) fn append_glue(st: &mut EngineState) -> Result<NodePtr<GlueNode>, Abort> {
    let s = st.scanner.cur_chr;
    match s {
        FIL_CODE => st.scanner.cur_val = FIL_GLUE.idx().0,
        FILL_CODE => st.scanner.cur_val = FILL_GLUE.idx().0,
        SS_CODE => st.scanner.cur_val = SS_GLUE.idx().0,
        FIL_NEG_CODE => st.scanner.cur_val = FIL_NEG_GLUE.idx().0,
        SKIP_CODE => crate::scan::scan_glue(st, GLUE_VAL)?,
        MSKIP_CODE => crate::scan::scan_glue(st, MU_VAL)?,
        _ => {}
    }
    let g = st.mem.new_glue(st.scanner.cur_glue())?;
    st.input.cur_list.tail_append(&mut st.mem, g.ptr());
    if s >= SKIP_CODE {
        let spec = g.glue_ptr(&st.mem);
        let refs = spec.ref_count(&st.mem);
        spec.set_ref_count(&mut st.mem, refs - 1);
        if s > SKIP_CODE {
            g.set_subtype(&mut st.mem, MU_GLUE);
        }
    }
    Ok(g)
}

pub(crate) fn append_kern(st: &mut EngineState) -> Result<(), Abort> {
    let s = st.scanner.cur_chr as u16;
    crate::scan::scan_dimen(st, s == MU_GLUE, false, false)?;
    let p = st.mem.new_kern(st.scanner.cur_val)?;
    p.set_subtype(&mut st.mem, s);
    st.input.cur_list.tail_append(&mut st.mem, p.ptr());
    Ok(())
}

pub(crate) fn make_mark(st: &mut EngineState) -> Result<(), Abort> {
    let c = if st.scanner.cur_chr == 0 {
        0
    } else {
        crate::scan::scan_register_num(st)?;
        st.scanner.cur_val
    };
    crate::scan::scan_toks(st, false, true)?;
    let mark = NodePtr::<MarkNode>::alloc(&mut st.mem)?;
    mark.set_mark_class(&mut st.mem, c);
    mark.set_subtype(&mut st.mem, 0);
    mark.set_mark_ptr(&mut st.mem, st.scanner.def_ref);
    st.input.cur_list.tail_append(&mut st.mem, mark.ptr());
    Ok(())
}

pub(crate) fn new_interaction(st: &mut EngineState) {
    crate::output::print_ln(st);
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

pub(crate) fn alter_aux(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.cur_chr != st.input.cur_list.mode.abs() as i32 {
        return crate::errors::report_illegal_case(st);
    }
    let c = st.scanner.cur_chr;
    crate::scan::scan_optional_equals(st)?;
    if c == VMODE as i32 {
        crate::scan::scan_dimen(st, false, false, false)?;
        st.input.cur_list.set_aux_s1(st.scanner.cur_val);
        return Ok(());
    }
    crate::scan::scan_int(st)?;
    if st.scanner.cur_val <= 0 || st.scanner.cur_val > 32767 {
        crate::output::error_here_with_diagnostic(st, "Bad space factor");
        crate::output::append_to_diagnostic(st, &format!(" ({})", st.scanner.cur_val));
        crate::output::capture_to_diagnostic(st, None);
        st.errors
            .help(&["I allow only values in the range 1..32767 here."]);
        return crate::errors::int_error(st, st.scanner.cur_val);
    }
    st.input.cur_list.set_aux_s0(st.scanner.cur_val);
    Ok(())
}

pub(crate) fn alter_prev_graf(st: &mut EngineState) -> Result<(), Abort> {
    let np = st.input.nest_ptr as usize;
    st.input.nest[np] = st.input.cur_list;
    let mut p = np;
    while st.input.nest[p].mode.abs() != VMODE {
        p -= 1;
    }
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_int(st)?;
    if st.scanner.cur_val < 0 {
        crate::output::error_here_with_diagnostic(st, "Bad ");
        crate::output::append_to_diagnostic(st, &format!(" ({})", st.scanner.cur_val));
        crate::output::print_esc(st, "prevgraf");
        crate::output::capture_to_diagnostic(st, None);
        st.errors.help(&["I allow only nonnegative values here."]);
        return crate::errors::int_error(st, st.scanner.cur_val);
    }
    st.input.nest[p].prev_graf = st.scanner.cur_val;
    st.input.cur_list = st.input.nest[np];
    Ok(())
}

pub(crate) fn alter_page_so_far(st: &mut EngineState) -> Result<(), Abort> {
    let c = st.scanner.cur_chr as usize;
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_dimen(st, false, false, false)?;
    st.shipout.page_so_far[c] = st.scanner.cur_val;
    Ok(())
}

pub(crate) fn alter_integer(st: &mut EngineState) -> Result<(), Abort> {
    let c = st.scanner.cur_chr;
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_int(st)?;
    match c {
        0 => st.shipout.dead_cycles = st.scanner.cur_val,
        2 => {
            if crate::state::InteractionMode::from_raw(st.scanner.cur_val).is_none() {
                crate::output::error_here_with_diagnostic(st, "Bad interaction mode");
                crate::output::append_to_diagnostic(st, &format!(" ({})", st.scanner.cur_val));
                crate::output::capture_to_diagnostic(st, None);
                st.errors.help(&[
                    "Modes are 0=batch, 1=nonstop, 2=scroll, and",
                    "3=errorstop. Proceed, and I'll ignore this case.",
                ]);
                return crate::errors::int_error(st, st.scanner.cur_val);
            }
            st.scanner.cur_chr = st.scanner.cur_val;
            new_interaction(st);
        }
        _ => st.shipout.insert_penalties = st.scanner.cur_val,
    }
    Ok(())
}

pub(crate) fn alter_box_dimen(st: &mut EngineState) -> Result<(), Abort> {
    let c = st.scanner.cur_chr;
    crate::scan::scan_register_num(st)?;
    let b = fetch_box(st, st.scanner.cur_val);
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_dimen(st, false, false, false)?;
    if let Some(b) = b {
        let v = Scaled(st.scanner.cur_val);
        match c {
            crate::scan::WIDTH_OFFSET => b.set_width(&mut st.mem, v),
            crate::scan::DEPTH_OFFSET => b.set_depth(&mut st.mem, v),
            _ => b.set_height(&mut st.mem, v),
        }
    }
    Ok(())
}

/// Replaces an all-zero scanned glue spec in `cur_val` by the shared `zero_glue`.
pub(crate) fn trap_zero_glue(st: &mut EngineState) {
    let spec = st.scanner.cur_glue();
    let mem = &mut st.mem;
    if spec.width(mem).0 == 0 && spec.stretch(mem).0 == 0 && spec.shrink(mem).0 == 0 {
        mem.add_glue_ref(ZERO_GLUE);
        mem.delete_glue_ref(spec);
        st.scanner.cur_val = ZERO_GLUE.idx().0;
    }
}

impl ListStateRecord {
    pub(crate) fn fix_language(
        &mut self,
        mem: &mut MemState,
        eqtb: &crate::state::EqtbState,
    ) -> Result<(), Abort> {
        let l = crate::helpers::norm_lang(eqtb.int_par(INT_PAR_LANGUAGE)) as i32;
        if l == self.aux_s1() {
            return Ok(());
        }
        let w = self.new_whatsit(mem, WhatsitSubtype::Language, SMALL_NODE_SIZE)?;
        w.set_what_lang(mem, l);
        self.set_aux_s1(l);
        let lhm = crate::helpers::norm_min(eqtb.int_par(INT_PAR_LEFT_HYPHEN_MIN)) as u16;
        w.set_what_lhm(mem, lhm);
        let rhm = crate::helpers::norm_min(eqtb.int_par(INT_PAR_RIGHT_HYPHEN_MIN)) as u16;
        w.set_what_rhm(mem, rhm);
        Ok(())
    }
}

pub(crate) fn off_save(st: &mut EngineState) -> Result<(), Abort> {
    if st.save_stack.cur_group == GroupCode::BottomLevel {
        crate::output::error_here_with_diagnostic(st, "Extra ");
        crate::output::print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
        crate::output::capture_to_diagnostic(st, None);
        st.errors
            .help(&["Things are pretty mixed up, but I think the worst is over."]);
        return crate::errors::error(st);
    }
    crate::input::back_input(st)?;
    let p = st.mem.get_avail()?;
    TEMP_HEAD.set_link(&mut st.mem, p);
    crate::output::error_here_with_diagnostic(st, "Missing ");
    match st.save_stack.cur_group {
        GroupCode::SemiSimple => {
            p.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_END_GROUP);
            crate::output::print_esc(st, "endgroup");
        }
        GroupCode::MathShift => {
            p.set_info(&mut st.mem, MATH_SHIFT_TOKEN + b'$' as i32);
            crate::output::print_char(st, b'$' as i32);
        }
        GroupCode::MathLeft => {
            p.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_RIGHT);
            let q = st.mem.get_avail()?;
            p.set_link(&mut st.mem, q);
            q.set_info(&mut st.mem, OTHER_TOKEN + b'.' as i32);
            crate::output::print_esc(st, "right.");
        }
        _ => {
            p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
            crate::output::print_char(st, b'}' as i32);
        }
    }
    crate::output::print(st, " inserted");
    crate::input::begin_token_list(st, TEMP_HEAD.link(&st.mem), INSERTED)?;
    crate::output::capture_to_diagnostic(st, None);
    st.errors.help(&[
        "I've inserted something that you may have forgotten.",
        "(See the <inserted text> above.)",
        "With luck, this will get me unwedged. But if you",
        "really didn't forget anything, try typing `2' now; then",
        "my insertion and my current dilemma will both disappear.",
    ]);
    crate::errors::error(st)
}

fn begin_disc_group(st: &mut EngineState) -> Result<(), Abort> {
    st.save_stack.new_save_level(GroupCode::Disc, st.mem.line)?;
    crate::scan::scan_left_brace(st)?;
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = INNER_HMODE;
    st.input.cur_list.set_aux_s0(1000);
    Ok(())
}

pub(crate) fn append_discretionary(st: &mut EngineState) -> Result<(), Abort> {
    let d = st.mem.new_disc()?;
    st.input.cur_list.tail_append(&mut st.mem, d.ptr());
    if st.scanner.cur_chr == 1 {
        let f = st.eqtb.cur_font();
        let c = st.fonts.hyphen_char[f as usize];
        if (0..=BIGGEST_CHAR).contains(&c) {
            let ch = new_character(st, f, c as u16)?;
            d.set_pre_break(&mut st.mem, ch.map_or(MemPtr::NULL, NewChar::ptr));
        }
        return Ok(());
    }
    st.save_stack.set_saved(0, 0);
    st.save_stack.save_ptr += 1;
    begin_disc_group(st)
}

pub(crate) fn shift_case(st: &mut EngineState) -> Result<(), Abort> {
    let b = st.scanner.cur_chr;
    crate::scan::scan_toks(st, false, false)?;
    let mut p = st.scanner.def_ref.link(&st.mem);
    while !p.is_null() {
        let t = p.token(&st.mem);
        if t < CS_TOKEN_FLAG + SINGLE_BASE {
            let c = t % MAX_CHAR_VAL;
            let mapped = st.eqtb.equiv(b + c);
            if mapped != 0 {
                p.set_token(&mut st.mem, t - c + mapped);
            }
        }
        p = p.link(&st.mem);
    }
    let def_ref = st.scanner.def_ref;
    crate::input::begin_token_list(st, def_ref.link(&st.mem), BACKED_UP)?;
    st.mem.free_avail(def_ref);
    Ok(())
}

pub(crate) fn build_discretionary(st: &mut EngineState) -> Result<(), Abort> {
    crate::save::unsave(st)?;
    let head = st.input.cur_list.head;
    let mut q = head;
    let mut p = q.link(&st.mem);
    let mut n: i32 = 0;
    while !p.is_null() {
        if !st.mem.is_char_node(p) {
            let allowed = match st.mem.classify(MemIdx::new(p)) {
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
                crate::output::print_err(st, "Improper discretionary list");
                st.errors
                    .help(&["Discretionary lists must contain only boxes and kerns."]);
                crate::errors::error(st)?;
                crate::output::begin_diagnostic(st);
                crate::output::diagnostic_begin_capture_warning_here(st);
                crate::output::print_nl(
                    st,
                    "The following discretionary sublist has been deleted:",
                );
                crate::output::show_box(st, p);
                crate::output::capture_to_diagnostic(st, None);
                crate::output::end_diagnostic(st, true);
                st.mem.flush_node_list(p)?;
                q.set_link(&mut st.mem, MemPtr::NULL);
                break;
            }
        }
        q = p;
        p = q.link(&st.mem);
        n += 1;
    }

    p = head.link(&st.mem);
    st.mem.pop_nest(&mut st.input);
    // SAFETY: `append_discretionary` left the disc node as the enclosing list's tail.
    let disc = NodePtr::<DiscNode>::from_idx_unchecked(MemIdx::new(st.input.cur_list.tail));
    let part = st.save_stack.saved(-1);
    match part {
        0 => disc.set_pre_break(&mut st.mem, p),
        1 => disc.set_post_break(&mut st.mem, p),
        2 => {
            if n > 0 && st.input.cur_list.mode.abs() == MMODE {
                crate::output::error_here_with_diagnostic(st, "Illegal math ");
                crate::output::print_esc(st, "discretionary");
                crate::output::capture_to_diagnostic(st, None);
                st.errors.help(&[
                    "Sorry: The third part of a discretionary break must be",
                    "empty, in math formulas. I had to delete your third part.",
                ]);
                st.mem.flush_node_list(p)?;
                n = 0;
                crate::errors::error(st)?;
            } else {
                disc.set_link(&mut st.mem, p);
            }
            if n <= u16::MAX as i32 {
                disc.set_subtype(&mut st.mem, n as u16);
            } else {
                crate::output::print_err(st, "Discretionary list is too long");
                st.errors.help(&[
                    "Wow---I never thought anybody would tweak me here.",
                    "You can't seriously need such a huge discretionary list?",
                ]);
                crate::errors::error(st)?;
            }
            if n > 0 {
                st.input.cur_list.tail = q;
            }
            st.save_stack.save_ptr -= 1;
            return Ok(());
        }
        _ => {}
    }
    st.save_stack.set_saved(-1, part + 1);
    begin_disc_group(st)
}

/// The italic correction of character `ch`.
fn char_italic(st: &mut EngineState, ch: NodePtr<CharNode>) -> i32 {
    let f = ch.font(&st.mem).0;
    let c = st.fonts.effective_char(f, ch.character(&st.mem));
    let info = st.fonts.char_info(f, c);
    st.fonts.char_italic(f, info)
}

pub(crate) fn append_italic_correction(st: &mut EngineState) -> Result<(), Abort> {
    let tail = st.input.cur_list.tail.0;
    if tail == st.input.cur_list.head.0 {
        return Ok(());
    }
    let italic = if let Some(ch) = st.mem.as_char(MemPtr(tail)) {
        char_italic(st, ch)
    } else {
        match st.mem.classify(MemIdx(tail)) {
            Ok(AnyNode::Ligature(lig)) => char_italic(st, lig.char_word()),
            Ok(AnyNode::Whatsit(w)) => match w.whatsit_subtype(&st.mem) {
                Some(s) if s.is_native_word() => {
                    st.fonts.real_get_native_italic_correction(&st.mem, w)
                }
                Some(WhatsitSubtype::Glyph) => {
                    st.fonts.real_get_native_glyph_italic_correction(&st.mem, w)
                }
                _ => return Ok(()),
            },
            _ => return Ok(()),
        }
    };
    let kern = st.mem.new_kern(italic)?;
    st.input.cur_list.tail_append(&mut st.mem, kern.ptr());
    kern.set_subtype(&mut st.mem, EXPLICIT);
    Ok(())
}

pub(crate) fn make_accent(st: &mut EngineState) -> Result<(), Abort> {
    crate::scan::scan_char_num(st)?;
    let f = st.eqtb.cur_font();
    let Some(p) = new_character(st, f, st.scanner.cur_val as u16)? else {
        return Ok(());
    };
    let x = st.fonts.param(f, X_HEIGHT_CODE);
    let s = st.fonts.param(f, SLANT_CODE) as f64 / 65536.0;
    let mut lsb: i32 = 0;
    let mut rsb: i32 = 0;
    let a = match p {
        NewChar::Native(w) => {
            let a = w.width(&st.mem).0;
            if a == 0 {
                (lsb, rsb) = st
                    .fonts
                    .get_native_char_sidebearings(f, st.scanner.cur_val)?;
            }
            a
        }
        NewChar::Char(ch) => {
            let c = st.fonts.effective_char(f, ch.character(&st.mem));
            let info = st.fonts.char_info(f, c);
            st.fonts.char_width(f, info)
        }
    };

    do_assignments(st)?;
    let f = st.eqtb.cur_font();
    let q = match st.scanner.cur_cmd {
        LETTER | OTHER_CHAR | CHAR_GIVEN => {
            let q = new_character(st, f, st.scanner.cur_chr as u16)?;
            st.scanner.cur_val = st.scanner.cur_chr;
            q
        }
        CHAR_NUM => {
            crate::scan::scan_char_num(st)?;
            new_character(st, f, st.scanner.cur_val as u16)?
        }
        _ => {
            crate::input::back_input(st)?;
            None
        }
    };
    let Some(q) = q else {
        st.input.cur_list.tail_append(&mut st.mem, p.ptr());
        st.input.cur_list.set_aux_s0(1000);
        return Ok(());
    };

    let t = st.fonts.param(f, SLANT_CODE) as f64 / 65536.0;
    let (w, h) = match q {
        NewChar::Native(w) => {
            let w = w.width(&st.mem).0;
            let (h, _) = st
                .fonts
                .get_native_char_height_depth(f, st.scanner.cur_val)?;
            (w, h)
        }
        NewChar::Char(ch) => {
            let c = st.fonts.effective_char(f, ch.character(&st.mem));
            let info = st.fonts.char_info(f, c);
            let w = st.fonts.char_width(f, info);
            let h = st.fonts.char_height(f, info);
            (w, h)
        }
    };
    let accent = if h == x {
        p.ptr()
    } else {
        let b = crate::packaging::hpack(st, p.ptr(), 0, ADDITIONAL)?;
        b.set_shift_amount(&mut st.mem, Scaled(x - h));
        b.ptr()
    };
    let delta = if st.fonts.is_native_font(f) && a == 0 {
        crate::scaled_math::tex_round((w - lsb + rsb) as f64 / 2.0 + h as f64 * t - x as f64 * s)
    } else {
        crate::scaled_math::tex_round((w - a) as f64 / 2.0 + h as f64 * t - x as f64 * s)
    };
    let r = st.mem.new_kern(delta)?;
    r.set_subtype(&mut st.mem, ACC_KERN);
    st.input.cur_list.tail.set_link(&mut st.mem, r.ptr());
    r.set_link(&mut st.mem, accent);
    let back = st.mem.new_kern(-a - delta)?;
    st.input.cur_list.tail = back.ptr();
    back.set_subtype(&mut st.mem, ACC_KERN);
    accent.set_link(&mut st.mem, back.ptr());
    st.input.cur_list.tail_append(&mut st.mem, q.ptr());
    st.input.cur_list.set_aux_s0(1000);
    Ok(())
}

fn flush_last_string(st: &mut EngineState) {
    st.strings.str_ptr -= 1;
    st.strings.discard_cur_string();
}

pub(crate) fn new_font(st: &mut EngineState, a: i16) -> Result<(), Abort> {
    if st.strings.job_name == 0 {
        crate::io::open_log_file(st)?;
    }
    crate::scan::get_r_token(st)?;
    let u = st.scanner.cur_cs;
    let t = if u >= HASH_BASE {
        st.eqtb.hash_s1(u)
    } else if u >= SINGLE_BASE {
        if u == NULL_CS {
            st.strings.make_tex_string("FONT")?
        } else {
            u - SINGLE_BASE
        }
    } else {
        let old_setting = st.shipout.selector;
        st.shipout.selector = SELECTOR_NEW_STRING;
        crate::output::print(st, "FONT");
        crate::output::print(st, u - 1);
        st.shipout.selector = old_setting;
        if st.strings.pool_ptr + 1 > st.strings.pool_size {
            return Err(Abort::PoolOverflow);
        }
        st.strings.make_string()?
    };

    let global = a >= 4;
    define(st, global, u, SET_FONT, FONT_BASE)?;
    crate::scan::scan_optional_equals(st)?;
    crate::io::scan_file_name(st)?;
    st.scanner.name_in_progress = true;
    let s = if crate::scan::scan_keyword(st, "at")? {
        crate::scan::scan_dimen(st, false, false, false)?;
        let size = st.scanner.cur_val;
        if size <= 0 || size >= 0x800_0000 {
            crate::output::error_here_with_diagnostic(st, "Improper `at' size (");
            crate::output::print_scaled(st, size);
            crate::output::print(st, "pt), replaced by 10pt");
            crate::output::capture_to_diagnostic(st, None);
            st.errors.help(&[
                "I can only handle fonts at positive sizes that are",
                "less than 2048pt, so I've changed what you said to 10pt.",
            ]);
            crate::errors::error(st)?;
            10 * 65536
        } else {
            size
        }
    } else if crate::scan::scan_keyword(st, "scaled")? {
        crate::scan::scan_int(st)?;
        let mag = st.scanner.cur_val;
        if mag <= 0 || mag > 32768 {
            crate::output::error_here_with_diagnostic(
                st,
                "Illegal magnification has been changed to 1000",
            );
            crate::output::append_to_diagnostic(st, &format!(" ({})", mag));
            crate::output::capture_to_diagnostic(st, None);
            st.errors
                .help(&["The magnification ratio must be between 1 and 32768."]);
            crate::errors::int_error(st, mag)?;
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
            st.fonts.font_size[f] == st.errors.xn_over_d(st.fonts.font_dsize[f], -s, 1000)
        }
    };
    let f = 'search: {
        for f in FONT_BASE + 1..=st.fonts.font_ptr {
            let fi = f as usize;
            let area_matches = (st.strings.length(st.scanner.cur_area) == 0
                && st.fonts.is_native_font(f))
                || st
                    .strings
                    .str_eq_str(st.fonts.font_area[fi], st.scanner.cur_area);
            if st
                .strings
                .str_eq_str(st.fonts.font_name[fi], st.scanner.cur_name)
                && area_matches
                && size_matches(st, fi)
            {
                break 'search f;
            }
            for part in [st.scanner.cur_area, st.scanner.cur_name, st.scanner.cur_ext] {
                st.strings.append_str(part)?;
            }
            let full_name = st.strings.make_string()?;
            let same_name = st.strings.str_eq_str(st.fonts.font_name[fi], full_name);
            flush_last_string(st);
            if same_name && st.fonts.is_native_font(f) && size_matches(st, fi) {
                break 'search f;
            }
        }
        read_font_info(st, u, st.scanner.cur_name, st.scanner.cur_area, s)?
    };

    define(st, global, u, SET_FONT, f)?;
    let word = st.eqtb.word(u);
    st.eqtb.set_word(FONT_ID_BASE + f, word);
    st.eqtb.set_hash_s1(FONT_ID_BASE + f, t);
    Ok(())
}

pub(crate) fn do_register_command(st: &mut EngineState, a: i16) -> Result<(), Abort> {
    let q = st.scanner.cur_cmd;
    let (l, p, e) = 'found: {
        if q != REGISTER {
            crate::scan::get_x_token(st)?;
            if (ASSIGN_INT..=ASSIGN_MU_GLUE).contains(&st.scanner.cur_cmd) {
                break 'found (st.scanner.cur_chr, (st.scanner.cur_cmd - ASSIGN_INT), false);
            }
            if st.scanner.cur_cmd != REGISTER {
                crate::output::error_here_with_diagnostic(st, "You can't use `");
                crate::output::print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
                crate::output::print(st, "' after ");
                crate::output::print_cmd_chr(st, q, 0);
                crate::output::capture_to_diagnostic(st, None);
                st.errors
                    .help(&["I'm forgetting what you said and not changing anything."]);
                return crate::errors::error(st);
            }
        }
        if !(0..=19).contains(&st.scanner.cur_chr) {
            let l = st.scanner.cur_chr;
            break 'found (l, NodePtr::<SaNode>::at(l).sa_type(&st.mem) as u8, true);
        }
        let p = st.scanner.cur_chr as u8;
        crate::scan::scan_register_num(st)?;
        let n = st.scanner.cur_val;
        if n > 255 {
            let sa = st.mem.create_sa_element(&mut st.scanner, p, n)?;
            break 'found (sa.idx().0, p, true);
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
        (true, true) => (NodePtr::<SaNode>::at(l).sa_int(&st.mem), TEX_NULL),
        (true, false) => (st.eqtb.equiv(l), TEX_NULL),
        (false, true) => (0, NodePtr::<SaNode>::at(l).sa_ptr(&st.mem).0),
        (false, false) => (0, st.eqtb.equiv(l)),
    };

    if q == REGISTER {
        crate::scan::scan_optional_equals(st)?;
    } else {
        crate::scan::scan_keyword(st, "by")?;
    }
    st.errors.arith_error = false;

    if q < MULTIPLY {
        if p < GLUE_VAL {
            if p == INT_VAL {
                crate::scan::scan_int(st)?;
            } else {
                crate::scan::scan_dimen(st, false, false, false)?;
            }
            if q == ADVANCE {
                st.scanner.cur_val += w;
            }
        } else {
            crate::scan::scan_glue(st, p)?;
            if q == ADVANCE {
                let sum = st.mem.new_spec(st.scanner.cur_glue())?;
                st.mem.delete_glue_ref(st.scanner.cur_glue());
                let old = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr(s));
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
        crate::scan::scan_int(st)?;
        let n = st.scanner.cur_val;
        let scale = |errors: &mut crate::state::ErrorState, x: i32, max_answer: i32| {
            if q == MULTIPLY {
                errors.mult_and_add(x, n, 0, max_answer)
            } else {
                errors.x_over_n(x, n)
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
            let old = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr(s));
            let r = st.mem.new_spec(old)?;
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
        crate::output::print_err(st, "Arithmetic overflow");
        st.errors.help(&[
            "I can't carry out that multiplication or division,",
            "since the result is out of range.",
        ]);
        if p >= GLUE_VAL {
            st.mem.delete_glue_ref(st.scanner.cur_glue());
        }
        return crate::errors::error(st);
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

pub(crate) fn do_assignments(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        crate::scan::get_x_non_blank_non_relax(st)?;
        if st.scanner.cur_cmd <= MAX_NON_PREFIXED_COMMAND {
            return Ok(());
        }
        st.scanner.set_box_allowed = false;
        prefixed_command(st)?;
        st.scanner.set_box_allowed = true;
    }
}

pub(crate) fn main_control_append_normal_space(st: &mut EngineState) -> Result<(), Abort> {
    if insert_boundary_inter_char_toks(st)? {
        return Ok(());
    }
    let glue = if st.eqtb.glue_par(GLUE_PAR_SPACE_SKIP) == ZERO_GLUE {
        let spec = font_space_glue(st)?;
        st.mainloop.main_p = spec.ptr();
        st.mem.new_glue(spec)?
    } else {
        st.mem.new_param_glue(&st.eqtb, GLUE_PAR_SPACE_SKIP)?
    };
    st.mem.temp_ptr = glue.ptr();
    st.input.cur_list.tail_append(&mut st.mem, glue.ptr());
    Ok(())
}

fn append_rule(st: &mut EngineState) -> Result<(), Abort> {
    let mode = st.input.cur_list.mode.abs();
    let rule = crate::scan::scan_rule_spec(st)?;
    st.input.cur_list.tail_append(&mut st.mem, rule.ptr());
    if mode == VMODE {
        st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
    } else if mode == HMODE {
        st.input.cur_list.set_aux_s0(1000);
    }
    Ok(())
}

fn valign_or_lr(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.cur_chr <= 0 {
        return init_align(st);
    }
    let enabled = crate::scan::eTeX_enabled(
        st,
        st.eqtb.int_par(INT_PAR_TEXXET) > 0,
        st.scanner.cur_cmd,
        st.scanner.cur_chr,
    )?;
    if enabled {
        let m = st.mem.new_math(0, st.scanner.cur_chr as u16)?;
        st.input.cur_list.tail_append(&mut st.mem, m.ptr());
    }
    Ok(())
}

/// Runs `action` if the math-shift group is open, else inserts what closes the current group.
fn in_math_shift_group(
    st: &mut EngineState,
    action: fn(&mut EngineState) -> Result<(), Abort>,
) -> Result<(), Abort> {
    if st.save_stack.cur_group == GroupCode::MathShift {
        action(st)
    } else {
        off_save(st)
    }
}

/// Scans the class, family and code of `\Umathchar` or `\Udelimiter`.
fn scan_umath_char(st: &mut EngineState) -> Result<i32, Abort> {
    crate::scan::scan_math_class_int(st)?;
    let mut t = set_class_fn(st.scanner.cur_val);
    crate::scan::scan_math_fam_int(st)?;
    t += set_family_fn(st.scanner.cur_val);
    crate::scan::scan_usv_num(st)?;
    Ok(t + st.scanner.cur_val)
}

/// Converts a classic 15-bit `\mathchar` code to the XeTeX layout.
fn math_char_from_tex(v: i32) -> i32 {
    set_class_fn(v / 4096) + set_family_fn((v % 4096) / 256) + (v % 256)
}

fn math_char_num(st: &mut EngineState) -> Result<(), Abort> {
    let c = match st.scanner.cur_chr {
        2 => scan_umath_char(st)?,
        1 => {
            crate::scan::scan_xetex_math_char_int(st)?;
            st.scanner.cur_val
        }
        _ => {
            crate::scan::scan_fifteen_bit_int(st)?;
            math_char_from_tex(st.scanner.cur_val)
        }
    };
    crate::scan::set_math_char(st, c)?;
    Ok(())
}

fn math_delim_num(st: &mut EngineState) -> Result<(), Abort> {
    let c = if st.scanner.cur_chr == 1 {
        scan_umath_char(st)?
    } else {
        crate::scan::scan_delimiter_int(st)?;
        math_char_from_tex(st.scanner.cur_val / 4096)
    };
    crate::scan::set_math_char(st, c)?;
    Ok(())
}

/// Appends a noad of type `cur_chr` (or an ordinary one for `{`) and scans its nucleus.
fn append_noad(st: &mut EngineState, backup: bool) -> Result<(), Abort> {
    let n = st.mem.new_noad()?;
    st.input.cur_list.tail_append(&mut st.mem, n.ptr());
    if backup {
        crate::input::back_input(st)?;
    } else {
        n.set_type_code(&mut st.mem, st.scanner.cur_chr as u16);
    }
    crate::scan::scan_math(st, n.nucleus())?;
    Ok(())
}

fn math_vcenter(st: &mut EngineState) -> Result<(), Abort> {
    crate::scan::scan_spec(st, GroupCode::Vcenter, false)?;
    normal_paragraph(st)?;
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = INNER_VMODE;
    st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
    if st.shipout.insert_src_special_every_vbox {
        crate::ext::insert_src_special(st)?;
    }
    begin_every(st, LOCAL_EVERY_VBOX, EVERY_VBOX_TEXT)
}

pub(crate) fn main_control_ignore_spaces(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.cur_chr == 0 {
        loop {
            crate::scan::get_x_token(st)?;
            if st.scanner.cur_cmd != SPACER {
                break;
            }
        }
    } else {
        let saved = st.scanner.scanner_status;
        st.scanner.scanner_status = NORMAL_SCANNER;
        let outcome = crate::scan::get_next(st);
        st.scanner.scanner_status = saved;
        Result::<(), Abort>::from(outcome)?;

        st.scanner.cur_cs = st.eqtb.prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)?;

        if st.scanner.cur_cs != UNDEFINED_PRIMITIVE {
            st.scanner.cur_cmd = st.eqtb.eq_type(PRIM_EQTB_BASE + st.scanner.cur_cs);
            st.scanner.cur_chr = st.eqtb.equiv(PRIM_EQTB_BASE + st.scanner.cur_cs);
            st.scanner.cur_tok = CS_TOKEN_FLAG + PRIM_EQTB_BASE + st.scanner.cur_cs;
        }
    }
    Ok(())
}

fn inter_char_tokens_enabled(st: &EngineState) -> bool {
    st.eqtb.int_par(INT_PAR_XETEX_INTER_CHAR_TOKENS) > 0
}

/// The `\XeTeXinterchartoks` list for class pair `n`, if set. Leaves the
/// pair's trie record, possibly null, in `cur_ptr`.
fn inter_char_toks(st: &mut EngineState, n: i32) -> Option<MemPtr> {
    let list = st
        .mem
        .find_sa_element(&mut st.scanner, INTER_CHAR_VAL, n)?
        .sa_ptr(&st.mem);
    (!list.is_null()).then_some(list)
}

/// Inserts the inter-character tokens between the last character's class and
/// the boundary class, ahead of the current token. Returns whether it did.
fn insert_boundary_inter_char_toks(st: &mut EngineState) -> Result<bool, Abort> {
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
    crate::input::back_input(st)?;
    crate::input::begin_token_list(st, list, INTER_CHAR_TEXT)?;
    Ok(true)
}

/// Inserts the inter-character tokens between the previous character's class
/// and class `sc` of the current character. Returns whether it did.
fn insert_inter_char_toks_before(st: &mut EngineState, sc: i32) -> Result<bool, Abort> {
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
    crate::input::back_input(st)?;
    st.input.cur_input.index = BACKED_UP_CHAR;
    crate::input::begin_token_list(st, list, INTER_CHAR_TEXT)?;
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
    let sf_code = st.eqtb.sf_code(st.scanner.cur_chr);
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

fn is_hyphen_unit(st: &EngineState, c: i32, hyphen: i32) -> bool {
    c == hyphen || (st.eqtb.int_par(INT_PAR_XETEX_DASH_BREAK) > 0 && (c == 8212 || c == 8211))
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
        if !st.mem.is_char_node(ppp) {
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

/// The native word `main_pp` if new text in font `f` can extend it.
fn extendable_native_word(st: &EngineState, f: i32) -> Option<NodePtr<WhatsitNode>> {
    let (pp, ppp) = (st.mainloop.main_pp, st.mainloop.main_ppp);
    if pp.is_null() {
        return None;
    }
    let word = st.mem.as_native_word(pp)?;
    let extendable = word.native_font(&st.mem).0 == f
        && ppp != pp
        && !st.mem.is_char_node(ppp)
        && !matches!(st.mem.classify(MemIdx(ppp.0)), Ok(AnyNode::Disc(_)));
    extendable.then_some(word)
}

impl MemState {
    /// Writes `text` into `node`'s text at `at`, unjournaled.
    fn write_native_text(&mut self, node: NodePtr<WhatsitNode>, at: i32, text: &[u16]) {
        let to = node.native_text_start() + at as usize;
        self.units_mut()[to..to + text.len()].copy_from_slice(text);
    }

    /// Copies the text of `src` into `node`'s text at `at`, unjournaled.
    fn copy_native_text_into(
        &mut self,
        node: NodePtr<WhatsitNode>,
        at: i32,
        src: NodePtr<WhatsitNode>,
    ) {
        let from = src.native_text_start();
        let len = src.native_length(self) as usize;
        self.units_mut()
            .copy_within(from..from + len, node.native_text_start() + at as usize);
    }
}

fn use_glyph_metrics(st: &EngineState) -> bool {
    st.eqtb.int_par(INT_PAR_XETEX_USE_GLYPH_METRICS) > 0
}

/// The main loop for an AAT or OpenType font: collects a run of characters
/// into native word nodes. Returns whether to go to `big_switch`.
fn main_loop_native(st: &mut EngineState, mf: i32) -> Result<bool, Abort> {
    if st.input.cur_list.mode > 0 && st.eqtb.int_par(INT_PAR_LANGUAGE) != st.input.cur_list.aux_s1()
    {
        st.input.cur_list.fix_language(&mut st.mem, &st.eqtb)?;
    }
    st.mainloop.main_h = 0;
    st.mainloop.main_f = mf;
    st.scanner.native_len = 0;

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
            st.scanner.reserve_native_text(2);
            st.scanner
                .push_native_text(((ch - 65536) / 1024 + 0xD800) as u16);
            st.scanner
                .push_native_text(((ch - 65536) % 1024 + 0xDC00) as u16);
        } else {
            st.scanner.reserve_native_text(1);
            st.scanner.push_native_text(ch as u16);
        }
        let is_hyph = is_hyphen_unit(st, ch, st.fonts.hyphen_char_entry(mf));
        st.hyph.is_hyph = is_hyph;
        if st.mainloop.main_h == 0 && is_hyph {
            st.mainloop.main_h = st.scanner.native_len;
        }

        crate::scan::get_next(st)?;
        if is_char_cmd(st.scanner.cur_cmd) {
            continue;
        }
        crate::scan::x_token(st)?;
        if is_char_cmd(st.scanner.cur_cmd) {
            continue;
        }
        if st.scanner.cur_cmd == CHAR_NUM {
            crate::scan::scan_usv_num(st)?;
            st.scanner.cur_chr = st.scanner.cur_val;
            continue;
        }
        insert_boundary_inter_char_toks(st)?;
        break;
    }

    if let Some(mapping) = st.fonts.font_mapping[mf as usize].clone() {
        let text = &st.scanner.native_text[..st.scanner.native_len as usize];
        let mapped_len = st.fonts.apply_mapping(&mapping, text);
        st.scanner.native_len = 0;
        st.scanner.reserve_native_text(mapped_len);
        st.mainloop.main_h = 0;
        let hyphen = st.fonts.hyphen_char_entry(mf);
        for i in 0..mapped_len {
            let c = st.fonts.mapped_text[i as usize];
            st.scanner.push_native_text(c);
            if st.mainloop.main_h == 0 && is_hyphen_unit(st, c as i32, hyphen) {
                st.mainloop.main_h = st.scanner.native_len;
            }
        }
    }

    if st.eqtb.int_par(INT_PAR_TRACING_LOST_CHARS) > 0 {
        let mut i = 0;
        while i < st.scanner.native_len {
            let mut c = st.scanner.native_text[i as usize] as i32;
            i += 1;
            if (0xD800..0xDC00).contains(&c) {
                c = 65536 + (c - 0xD800) * 1024 + st.scanner.native_text[i as usize] as i32
                    - 0xDC00;
                i += 1;
            }
            if st.fonts.map_char_to_glyph(mf, c)? == 0 {
                crate::output::char_warning(st, mf, c)?;
            }
        }
    }

    let len = st.scanner.native_len;
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
                st.scanner.reserve_native_text(remaining);
                st.scanner.save_native_len = st.scanner.native_len;
                for &c in word.native_text(&st.mem) {
                    st.scanner.push_native_text(c);
                }
                for i in 0..st.mainloop.main_h {
                    let c = st.scanner.native_text[(start + i) as usize];
                    st.scanner.push_native_text(c);
                }
                crate::linebreak::do_locale_linebreaks(st, st.scanner.save_native_len, remaining)?;
                st.scanner.native_len = st.scanner.save_native_len;
                remaining = st.scanner.native_len - st.mainloop.main_h - start;
                start = st.mainloop.main_h;
                st.mainloop.main_h = 0;
                advance_past_hyphen(st, start, remaining);
                let next = word.link(&st.mem);
                st.mainloop.main_ppp.set_link(&mut st.mem, next);
                word.set_link(&mut st.mem, MemPtr::NULL);
                st.mem.flush_node_list(word.idx().0)?;
                st.mainloop.main_pp = st.input.cur_list.tail;
                while st.mainloop.main_ppp.link(&st.mem) != st.mainloop.main_pp {
                    st.mainloop.main_ppp = st.mainloop.main_ppp.link(&st.mem);
                }
            } else {
                crate::linebreak::do_locale_linebreaks(st, start, st.mainloop.main_h)?;
                start += st.mainloop.main_h;
                remaining -= st.mainloop.main_h;
                st.mainloop.main_h = 0;
                advance_past_hyphen(st, start, remaining);
            }
            if remaining > 0 || st.hyph.is_hyph {
                let disc = st.mem.new_disc()?;
                st.input.cur_list.tail_append(&mut st.mem, disc.ptr());
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
        let node = st.mem.new_native_word_node(&st.eqtb, mf, len + old_len)?;
        pp.set_link(&mut st.mem, node.ptr());
        st.input.cur_list.tail = node.ptr();
        st.mem.journal_mark_range(
            node.idx().offset(NATIVE_NODE_SIZE),
            crate::state::native_text_words(len + old_len),
        );
        if let Some(word) = old {
            st.mem.copy_native_text_into(node, 0, word);
        }
        st.mem
            .write_native_text(node, old_len, &st.scanner.native_text[..len as usize]);
        let metrics = use_glyph_metrics(st);
        st.fonts.measure_native_node(&mut st.mem, node, metrics)?;
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
            st.mem.flush_node_list(word.idx().0)?;
        }
    }

    if st.eqtb.int_par(INT_PAR_XETEX_INTERWORD_SPACE_SHAPING) > 0 {
        shape_interword_space(st, mf)?;
    }
    Ok(!st.scanner.cur_ptr.is_null())
}

/// Kerns the interword glue before the current word by what the font's shaping
/// of `word space word` implies.
fn shape_interword_space(st: &mut EngineState, mf: i32) -> Result<(), Abort> {
    let tail = st.input.cur_list.tail;
    let mut p = st.input.cur_list.head;
    let mut prev_word = None;
    while p != tail {
        if !p.is_null() {
            prev_word = st.mem.as_native_word(p).or(prev_word);
        }
        p = p.link(&st.mem);
    }
    let Some(prev_word) = prev_word.filter(|w| w.native_font(&st.mem).0 == mf) else {
        return Ok(());
    };
    let glue = st.mem.skip_interword_fillers(prev_word.link(&st.mem));
    if st.mem.is_char_node(glue) {
        return Ok(());
    }
    let Ok(AnyNode::Glue(glue)) = st.mem.classify(MemIdx(glue.0)) else {
        return Ok(());
    };
    if st.mem.skip_interword_fillers(glue.link(&st.mem)) != tail {
        return Ok(());
    }
    let tail_word = NodePtr::<WhatsitNode>::from_idx_unchecked(MemIdx::new(tail));
    let prev_len = prev_word.native_length(&st.mem) as i32;
    let tail_len = tail_word.native_length(&st.mem) as i32;
    let len = prev_len + 1 + tail_len;
    let joined = st.mem.new_native_word_node(&st.eqtb, mf, len)?;
    st.mem.journal_mark_range(
        joined.idx().offset(NATIVE_NODE_SIZE),
        crate::state::native_text_words(len),
    );
    st.mem.copy_native_text_into(joined, 0, prev_word);
    st.mem.write_native_text(joined, prev_len, &[b' ' as u16]);
    st.mem
        .copy_native_text_into(joined, prev_len + 1, tail_word);
    let metrics = use_glyph_metrics(st);
    st.fonts.measure_native_node(&mut st.mem, joined, metrics)?;
    let t = joined.width(&st.mem).0 - prev_word.width(&st.mem).0 - tail_word.width(&st.mem).0;
    joined.free_native(&mut st.mem);
    // TeX reads `width(font_glue[f])`, which stays null while `\spaceskip` supplies the glue.
    let space = st.fonts.param(mf, SPACE_CODE);
    if t != space {
        let kern = st.mem.new_kern(t - space)?;
        kern.set_subtype(&mut st.mem, SPACE_ADJUSTMENT);
        let next = glue.link(&st.mem);
        kern.set_link(&mut st.mem, next);
        glue.set_link(&mut st.mem, kern.ptr());
    }
    Ok(())
}

/// The character on top of `lig_stack`, a char node or a lig item alike.
#[inline(always)]
fn lig_stack_char(st: &EngineState) -> i32 {
    NodePtr::<CharNode>::at(st.mainloop.lig_stack).character(&st.mem) as i32
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
fn wrapup(st: &mut EngineState, rt_hit: bool) -> Result<(), Abort> {
    if st.mainloop.cur_l >= TOO_BIG_CHAR {
        return Ok(());
    }
    let main_f = st.mainloop.main_f;
    if !st.mainloop.cur_q.link(&st.mem).is_null()
        && NodePtr::<CharNode>::at(st.input.cur_list.tail).character(&st.mem) as i32
            == st.fonts.hyphen_char_entry(main_f)
    {
        st.mainloop.ins_disc = true;
    }
    if st.mainloop.ligature_present {
        let lig_list = st.mainloop.cur_q.link(&st.mem);
        let lig = st
            .mem
            .new_ligature(FontId(main_f), st.mainloop.cur_l as u16, lig_list)?;
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
            let disc = st.mem.new_disc()?;
            st.input.cur_list.tail_append(&mut st.mem, disc.ptr());
        }
    }
    Ok(())
}

#[inline(always)]
fn cur_l_info(st: &mut EngineState) -> crate::mem_types::B16x4 {
    let main_f = st.mainloop.main_f;
    let c = st.fonts.effective_char(main_f, st.mainloop.cur_l as u16);
    st.fonts.char_info(main_f, c)
}

/// Returns true to go to `big_switch`, false to go to `reswitch`.
pub(crate) fn main_control_main_loop(st: &mut EngineState) -> Result<bool, Abort> {
    if st.input.cur_list.head == st.input.cur_list.tail
        && st.input.cur_list.mode > 0
        && st.shipout.insert_src_special_auto
    {
        append_src_special(st)?;
    }
    st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;

    let mf = st.eqtb.cur_font();
    if st.fonts.is_native_font(mf) {
        return main_loop_native(st, mf);
    }

    let sc = adjust_space_factor(st);
    if insert_inter_char_toks_before(st, sc)? {
        return Ok(true);
    }
    note_char_class(st, sc);

    st.mainloop.main_f = mf;
    st.mainloop.bchar = st.fonts.font_bchar_entry(mf);
    st.mainloop.false_bchar = st.fonts.font_false_bchar_entry(mf);
    if st.input.cur_list.mode > 0 && st.eqtb.int_par(INT_PAR_LANGUAGE) != st.input.cur_list.aux_s1()
    {
        st.input.cur_list.fix_language(&mut st.mem, &st.eqtb)?;
    }
    st.mainloop.cur_l = st.scanner.cur_chr;
    let ls = st.mem.new_char(FontId(mf), st.mainloop.cur_l as u16)?;
    st.mainloop.lig_stack = ls.ptr();
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
                st.mainloop.cur_l = lig_stack_char(st);
            }

            if !st.mem.is_char_node(st.mainloop.lig_stack) {
                let item = NodePtr::<LigItem>::at(st.mainloop.lig_stack);
                let lig_ptr = item.component(&st.mem);
                if lig_ptr.0 > TEX_NULL {
                    st.input.cur_list.tail_append(&mut st.mem, lig_ptr);
                }
                st.mainloop.lig_stack = item.link(&st.mem);
                item.free(&mut st.mem);
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
                    st.mainloop.cur_r = lig_stack_char(st);
                }
                continue;
            }
        }

        let main_f = st.mainloop.main_f;
        let cur_chr = st.scanner.cur_chr as u16;
        // Each call resets `xtx_ligature_present`, so the two bounds may see different codes.
        let exists = if st.fonts.effective_char(main_f, cur_chr)
            > st.fonts.font_ec_entry(main_f) as i32
            || st.fonts.effective_char(main_f, cur_chr) < st.fonts.font_bc_entry(main_f) as i32
        {
            false
        } else {
            main_i = st
                .fonts
                .effective_char_info(main_f, st.mainloop.cur_l as u16);
            main_i.s3 > 0
        };
        if !exists {
            crate::output::char_warning(st, main_f, st.scanner.cur_chr)?;
            st.mem.free_avail(st.mainloop.lig_stack);
            return Ok(true);
        }
        st.input
            .cur_list
            .tail_append(&mut st.mem, st.mainloop.lig_stack);
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
) -> Result<LigExit, Abort> {
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
            let kern = st.fonts.char_kern(main_f, step);
            let kern = st.mem.new_kern(kern)?;
            st.input.cur_list.tail_append(&mut st.mem, kern.ptr());
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
                    st.mainloop.lig_stack = st.mem.new_lig_item(c)?.ptr();
                    st.mainloop.bchar = TOO_BIG_CHAR;
                } else if st.mem.is_char_node(st.mainloop.lig_stack) {
                    let prev = st.mainloop.lig_stack;
                    let item = st.mem.new_lig_item(c)?;
                    st.mainloop.lig_stack = item.ptr();
                    item.set_component(&mut st.mem, prev);
                } else {
                    NodePtr::<LigItem>::at(st.mainloop.lig_stack).set_character(&mut st.mem, c);
                }
            }
            3 => {
                st.mainloop.cur_r = step.s0 as i32;
                let prev = st.mainloop.lig_stack;
                let item = st.mem.new_lig_item(st.mainloop.cur_r as u16)?;
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
fn lookahead(st: &mut EngineState) -> Result<bool, Abort> {
    crate::scan::get_next(st)?;
    if !is_char_cmd(st.scanner.cur_cmd) {
        crate::scan::x_token(st)?;
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
        crate::scan::scan_char_num(st)?;
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

    let c = st.scanner.cur_chr;
    let ls = st.mem.new_char(FontId(st.mainloop.main_f), c as u16)?;
    st.mainloop.lig_stack = ls.ptr();
    st.mainloop.cur_r = c;
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
fn dispatch_command(st: &mut EngineState) -> Result<Next, Abort> {
    if st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) > 0 {
        crate::output::show_cur_cmd_chr(st);
    }
    let mode = st.input.cur_list.mode.abs();
    let cmd = st.scanner.cur_cmd;
    if mode == HMODE {
        match cmd {
            LETTER | OTHER_CHAR | CHAR_GIVEN | CHAR_NUM => {
                if cmd == CHAR_NUM {
                    crate::scan::scan_usv_num(st)?;
                    st.scanner.cur_chr = st.scanner.cur_val;
                }
                return Ok(if main_control_main_loop(st)? {
                    Next::BigSwitch
                } else {
                    Next::Reswitch
                });
            }
            NO_BOUNDARY => {
                crate::scan::get_x_token(st)?;
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
                main_control_append_normal_space(st)?;
            } else {
                app_space(st)?;
            }
        }
        (HMODE | MMODE, EX_SPACE) => main_control_append_normal_space(st)?,
        (_, RELAX) | (VMODE | MMODE, SPACER) | (MMODE, NO_BOUNDARY) => {}
        (_, IGNORE_SPACES) => {
            main_control_ignore_spaces(st)?;
            return Ok(Next::Reswitch);
        }
        (VMODE, STOP) => {
            if its_all_over(st)? {
                return Ok(Next::Stop);
            }
        }

        (VMODE, VMOVE)
        | (HMODE | MMODE, HMOVE)
        | (_, LAST_ITEM)
        | (VMODE, VADJUST | ITAL_CORR)
        | (VMODE | HMODE, EQ_NO)
        | (_, MAC_PARAM) => crate::errors::report_illegal_case(st)?,
        (
            VMODE | HMODE,
            SUP_MARK | SUB_MARK | MATH_CHAR_NUM | MATH_GIVEN | XETEX_MATH_GIVEN | MATH_COMP
            | DELIM_NUM | LEFT_RIGHT | ABOVE | RADICAL | MATH_STYLE | MATH_CHOICE | VCENTER
            | NON_SCRIPT | MKERN | LIMIT_SWITCH | MSKIP | MATH_ACCENT,
        )
        | (MMODE, ENDV | PAR_END | STOP | VSKIP | UN_VBOX | VALIGN | HRULE) => {
            crate::scan::insert_dollar_sign(st)?
        }

        (VMODE, HRULE) | (HMODE | MMODE, VRULE) => append_rule(st)?,
        (VMODE, VSKIP) | (HMODE | MMODE, HSKIP) | (MMODE, MSKIP) => {
            append_glue(st)?;
        }
        (_, KERN) | (MMODE, MKERN) => append_kern(st)?,
        (VMODE | HMODE, LEFT_BRACE) => st
            .save_stack
            .new_save_level(GroupCode::Simple, st.mem.line)?,
        (_, BEGIN_GROUP) => st
            .save_stack
            .new_save_level(GroupCode::SemiSimple, st.mem.line)?,
        (_, END_GROUP) => {
            if st.save_stack.cur_group == GroupCode::SemiSimple {
                crate::save::unsave(st)?;
            } else {
                off_save(st)?;
            }
        }

        (_, RIGHT_BRACE) => handle_right_brace(st)?,
        (VMODE, HMOVE) | (HMODE | MMODE, VMOVE) => {
            let t = st.scanner.cur_chr;
            crate::scan::scan_dimen(st, false, false, false)?;
            let shift = if t == 0 {
                st.scanner.cur_val
            } else {
                -st.scanner.cur_val
            };
            scan_box(st, shift)?;
        }
        (_, LEADER_SHIP) => scan_box(st, LEADER_FLAG - A_LEADERS as i32 + st.scanner.cur_chr)?,
        (_, MAKE_BOX) => begin_box(st, 0)?,
        (VMODE, START_PAR) => new_graf(st, st.scanner.cur_chr > 0)?,
        (
            VMODE,
            LETTER | OTHER_CHAR | CHAR_NUM | CHAR_GIVEN | MATH_SHIFT | UN_HBOX | VRULE | ACCENT
            | DISCRETIONARY | HSKIP | VALIGN | EX_SPACE | NO_BOUNDARY,
        ) => {
            crate::input::back_input(st)?;
            new_graf(st, true)?;
        }
        (HMODE | MMODE, START_PAR) => indent_in_hmode(st)?,
        (VMODE, PAR_END) => {
            normal_paragraph(st)?;
            if st.input.cur_list.mode > 0 {
                crate::pagebuilder::build_page(st)?;
            }
        }
        (HMODE, PAR_END) => {
            if st.scanner.align_state < 0 {
                off_save(st)?;
            }
            end_graf(st)?;
            if st.input.cur_list.mode == VMODE {
                crate::pagebuilder::build_page(st)?;
            }
        }
        (HMODE, STOP | VSKIP | HRULE | UN_VBOX | HALIGN) => head_for_vmode(st)?,
        (_, INSERT) | (HMODE | MMODE, VADJUST) => begin_insert_or_adjust(st)?,
        (_, MARK) => make_mark(st)?,
        (_, BREAK_PENALTY) => append_penalty(st)?,
        (_, REMOVE_ITEM) => delete_last(st)?,
        (VMODE, UN_VBOX) | (HMODE | MMODE, UN_HBOX) => unpackage(st)?,
        (HMODE, ITAL_CORR) => append_italic_correction(st)?,
        (MMODE, ITAL_CORR) => {
            let k = st.mem.new_kern(0)?;
            st.input.cur_list.tail_append(&mut st.mem, k.ptr());
        }
        (HMODE | MMODE, DISCRETIONARY) => append_discretionary(st)?,
        (HMODE, ACCENT) => make_accent(st)?,
        (_, CAR_RET | TAB_MARK) => align_error(st)?,
        (_, NO_ALIGN) => no_align_error(st)?,
        (_, OMIT) => omit_error(st)?,
        (VMODE, HALIGN) => init_align(st)?,
        (HMODE, VALIGN) => valign_or_lr(st)?,
        (MMODE, HALIGN) => {
            if crate::errors::privileged(st)? {
                in_math_shift_group(st, init_align)?;
            }
        }
        (VMODE | HMODE, ENDV) => do_endv(st)?,
        (_, END_CS_NAME) => crate::errors::cs_error(st)?,

        (HMODE, MATH_SHIFT) => init_math(st)?,
        (MMODE, EQ_NO) => {
            if crate::errors::privileged(st)? {
                in_math_shift_group(st, |st| Ok(crate::math::start_eq_no(st)?))?;
            }
        }
        (MMODE, LEFT_BRACE) => append_noad(st, true)?,
        (MMODE, LETTER | OTHER_CHAR | CHAR_GIVEN) => {
            crate::scan::set_math_char(st, st.eqtb.math_code(st.scanner.cur_chr))?
        }
        (MMODE, CHAR_NUM) => {
            crate::scan::scan_char_num(st)?;
            st.scanner.cur_chr = st.scanner.cur_val;
            crate::scan::set_math_char(st, st.eqtb.math_code(st.scanner.cur_chr))?;
        }
        (MMODE, MATH_CHAR_NUM) => math_char_num(st)?,
        (MMODE, MATH_GIVEN) => {
            crate::scan::set_math_char(st, math_char_from_tex(st.scanner.cur_chr))?
        }
        (MMODE, XETEX_MATH_GIVEN) => crate::scan::set_math_char(st, st.scanner.cur_chr)?,
        (MMODE, DELIM_NUM) => math_delim_num(st)?,
        (MMODE, MATH_COMP) => append_noad(st, false)?,
        (MMODE, LIMIT_SWITCH) => crate::math::math_limit_switch(st)?,
        (MMODE, RADICAL) => crate::math::math_radical(st)?,
        (MMODE, ACCENT | MATH_ACCENT) => crate::math::math_ac(st)?,
        (MMODE, VCENTER) => math_vcenter(st)?,
        (MMODE, MATH_STYLE) => {
            let s = st.mem.new_style(st.scanner.cur_chr as i16)?;
            st.input.cur_list.tail_append(&mut st.mem, s.ptr());
        }
        (MMODE, NON_SCRIPT) => {
            let g = st.mem.new_glue(ZERO_GLUE)?;
            st.input.cur_list.tail_append(&mut st.mem, g.ptr());
            g.set_subtype(&mut st.mem, COND_MATH_GLUE);
        }
        (MMODE, MATH_CHOICE) => crate::math::append_choices(st)?,
        (MMODE, SUB_MARK | SUP_MARK) => crate::math::sub_sup(st)?,
        (MMODE, ABOVE) => crate::math::math_fraction(st)?,
        (MMODE, LEFT_RIGHT) => crate::math::math_left_right(st)?,
        (MMODE, MATH_SHIFT) => in_math_shift_group(st, |st| Ok(after_math(st)?))?,

        (_, TOKS_REGISTER..=SET_INTERACTION) => prefixed_command(st)?,
        (_, AFTER_ASSIGNMENT) => {
            crate::scan::get_token(st)?;
            st.scanner.after_token = st.scanner.cur_tok;
        }
        (_, AFTER_GROUP) => {
            crate::scan::get_token(st)?;
            st.save_stack.save_for_after(st.scanner.cur_tok)?;
        }
        (_, IN_STREAM) => crate::io::open_or_close(st)?,
        (_, MESSAGE) => crate::output::issue_message(st)?,
        (_, CASE_SHIFT) => shift_case(st)?,
        (_, XRAY) => crate::output::show_whatever(st)?,
        (_, EXTENSION) => do_extension(st)?,
        _ => {}
    }
    Ok(Next::BigSwitch)
}

fn main_control_from(
    st: &mut EngineState,
    run_every_job: bool,
    mut resume_pending_command: bool,
) -> Result<(), Abort> {
    if run_every_job {
        begin_every(st, LOCAL_EVERY_JOB, EVERY_JOB_TEXT)?;
    }
    // Cancellation and fence hooks are for resident sessions only, batch runs
    // must not pay for them per command.
    let resident = st.config.execution_mode == crate::state::EngineExecutionMode::Resident;
    loop {
        if resident && st.cancellation.at_command_boundary() {
            return Err(Abort::Cancelled);
        }
        if resume_pending_command {
            resume_pending_command = false;
        } else {
            if resident {
                crate::session::capture_prefetch_fence(st);
            }
            crate::scan::get_x_token(st)?;
        }
        if resident && crate::session::capture_command_fence(st) {
            return Err(Abort::Horizoned);
        }
        loop {
            match dispatch_command(st)? {
                Next::BigSwitch => break,
                Next::Reswitch => {}
                Next::Stop => return Ok(()),
            }
        }
    }
}

/// Dispatch a fresh document, including `\everyjob` and the first token fetch.
pub(crate) fn main_control(st: &mut EngineState) -> Result<(), Abort> {
    main_control_from(st, true, false)
}

/// Resume at a restored command boundary, optionally with a command already fetched.
pub(crate) fn main_control_resume(
    st: &mut EngineState,
    resume_pending_command: bool,
) -> Result<(), Abort> {
    main_control_from(st, false, resume_pending_command)
}

fn define(st: &mut EngineState, global: bool, p: i32, t: u8, e: i32) -> Result<(), Abort> {
    if global {
        st.eqtb.geq_define(&mut st.mem, &mut st.scanner, p, t, e)
    } else {
        st.eqtb
            .eq_define(&mut st.save_stack, &mut st.mem, &mut st.scanner, p, t, e)
    }
}

fn word_define(st: &mut EngineState, global: bool, p: i32, w: i32) -> Result<(), Abort> {
    if global {
        st.eqtb.geq_word_define(p, w);
        Ok(())
    } else {
        st.eqtb.eq_word_define(&mut st.save_stack, p, w)
    }
}

fn sa_define(st: &mut EngineState, global: bool, p: i32, e: i32) -> Result<(), Abort> {
    let result = if global {
        st.mem.gsa_def(p, e)
    } else {
        st.mem.sa_def(&mut st.scanner, &mut st.save_stack, p, e)?
    };
    crate::save::finish_sa_mutation(st, p, result)?;
    Ok(())
}

fn sa_word_define(st: &mut EngineState, global: bool, p: i32, w: i32) -> Result<(), Abort> {
    let result = if global {
        st.mem.gsa_w_def(p, w)
    } else {
        st.mem.sa_w_def(&mut st.scanner, &mut st.save_stack, p, w)?
    };
    crate::save::finish_sa_mutation(st, p, result)?;
    Ok(())
}

impl MemState {
    fn add_sa_ref(&mut self, p: MemPtr) {
        if let Some(sa) = NodePtr::<SaNode>::from_ptr(p) {
            let refs = sa.sa_ref(self);
            sa.set_sa_ref(self, refs + 1);
        }
    }
}

/// The `sa_ptr` of a sparse-array record, or null without one.
/// Scans a pair of character classes into its `\XeTeXinterchartoks` index.
fn scan_inter_char_pair(st: &mut EngineState) -> Result<i32, Abort> {
    crate::scan::scan_char_class_not_ignored(st)?;
    // TeX parks the first class in `cur_ptr` while scanning the second.
    st.scanner.cur_ptr.0 = st.scanner.cur_val;
    crate::scan::scan_char_class_not_ignored(st)?;
    Ok(st.scanner.cur_ptr.0 * CHAR_CLASS_LIMIT + st.scanner.cur_val)
}

/// Defines token list register `p`, a sparse-array entry if `e`, as `list`.
fn define_toks(
    st: &mut EngineState,
    e: bool,
    global: bool,
    p: i32,
    list: MemPtr,
) -> Result<(), Abort> {
    if e {
        return sa_define(st, global, p, list.0);
    }
    let t = if list.is_null() { UNDEFINED_CS } else { CALL };
    define(st, global, p, t, list.0)?;
    Ok(())
}

pub(crate) fn prefixed_command(st: &mut EngineState) -> Result<(), Abort> {
    let mut a: i32 = 0;
    while st.scanner.cur_cmd == PREFIX {
        if (a / st.scanner.cur_chr) & 1 == 0 {
            a += st.scanner.cur_chr;
        }
        crate::scan::get_x_non_blank_non_relax(st)?;
        if st.scanner.cur_cmd <= MAX_NON_PREFIXED_COMMAND {
            crate::output::error_here_with_diagnostic(st, "You can't use a prefix with `");
            crate::output::print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
            crate::output::print_char(st, '\'' as i32);
            crate::output::capture_to_diagnostic(st, None);
            st.errors
                .help(&[r"I'll pretend you didn't say \long or \outer or \global or \protected."]);
            return crate::input::back_error(st);
        }
        if st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) > 2 {
            crate::output::show_cur_cmd_chr(st);
        }
    }

    let protected = if a >= 8 {
        a -= 8;
        PROTECTED_TOKEN
    } else {
        0
    };
    if st.scanner.cur_cmd != DEF && (a % 4 != 0 || protected != 0) {
        crate::output::error_here_with_diagnostic(st, "You can't use `");
        crate::output::print_esc(st, "long");
        crate::output::print(st, "' or `");
        crate::output::print_esc(st, "outer");
        crate::output::print(st, "' or `");
        crate::output::print_esc(st, "protected");
        crate::output::print(st, "' with `");
        crate::output::print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
        crate::output::print_char(st, '\'' as i32);
        crate::output::capture_to_diagnostic(st, None);
        st.errors
            .help(&[r"I'll pretend you didn't say \long or \outer or \protected here."]);
        crate::errors::error(st)?;
    }
    let global_defs = st.eqtb.int_par(INT_PAR_GLOBAL_DEFS);
    if global_defs < 0 && a >= 4 {
        a -= 4;
    } else if global_defs > 0 && a < 4 {
        a += 4;
    }

    match st.scanner.cur_cmd {
        SET_FONT => define(st, a >= 4, CUR_FONT_LOC, DATA, st.scanner.cur_chr)?,
        DEF => {
            if st.scanner.cur_chr & 1 != 0 && a < 4 && st.eqtb.int_par(INT_PAR_GLOBAL_DEFS) >= 0 {
                a += 4;
            }
            let expand = st.scanner.cur_chr >= 2;
            crate::scan::get_r_token(st)?;
            let p = st.scanner.cur_cs;
            crate::scan::scan_toks(st, true, expand)?;
            if protected != 0 {
                let q = st.mem.get_avail()?;
                q.set_info(&mut st.mem, protected);
                let body = st.scanner.def_ref.link(&st.mem);
                q.set_link(&mut st.mem, body);
                st.scanner.def_ref.set_link(&mut st.mem, q);
            }
            define(st, a >= 4, p, CALL + (a % 4) as u8, st.scanner.def_ref.0)?;
        }
        LET => {
            let n = st.scanner.cur_chr;
            crate::scan::get_r_token(st)?;
            let p = st.scanner.cur_cs;
            if n == NORMAL as i32 {
                loop {
                    crate::scan::get_token(st)?;
                    if st.scanner.cur_cmd != SPACER {
                        break;
                    }
                }
                if st.scanner.cur_tok == OTHER_TOKEN + '=' as i32 {
                    crate::scan::get_token(st)?;
                    if st.scanner.cur_cmd == SPACER {
                        crate::scan::get_token(st)?;
                    }
                }
            } else {
                crate::scan::get_token(st)?;
                let q = st.scanner.cur_tok;
                crate::scan::get_token(st)?;
                crate::input::back_input(st)?;
                st.scanner.cur_tok = q;
                crate::input::back_input(st)?;
            }
            if st.scanner.cur_cmd >= CALL {
                st.mem.add_token_ref(MemPtr(st.scanner.cur_chr));
            } else if (st.scanner.cur_cmd == REGISTER || st.scanner.cur_cmd == TOKS_REGISTER)
                && !(0..=19).contains(&st.scanner.cur_chr)
            {
                st.mem.add_sa_ref(MemPtr::from_raw(st.scanner.cur_chr));
            }
            define(st, a >= 4, p, st.scanner.cur_cmd, st.scanner.cur_chr)?;
        }
        SHORTHAND_DEF => {
            if st.scanner.cur_chr == CHAR_SUB_DEF_CODE {
                crate::output::print_err(st, "vestigial MLTeX shorthand encountered??");
                st.errors
                    .help(&["This should never happen in Tectonic, where MLTeX has been excised."]);
                crate::errors::error(st)?;
            } else {
                shorthand_def(st, a >= 4)?;
            }
        }
        READ_TO_CS => {
            let j = st.scanner.cur_chr;
            crate::scan::scan_int(st)?;
            let n = st.scanner.cur_val;
            if !crate::scan::scan_keyword(st, "to")? {
                crate::output::print_err(st, "Missing `to' inserted");
                st.errors.help(&[
                    r"You should have said `\read<number> to \cs'.",
                    r"I'm going to look for the \cs now.",
                ]);
                crate::errors::error(st)?;
            }
            crate::scan::get_r_token(st)?;
            let p = st.scanner.cur_cs;
            crate::scan::read_toks(st, n, p, j)?;
            define(st, a >= 4, p, CALL, st.scanner.cur_val)?;
        }
        TOKS_REGISTER | ASSIGN_TOKS => assign_toks(st, a >= 4)?,
        ASSIGN_INT => {
            let p = st.scanner.cur_chr;
            crate::scan::scan_optional_equals(st)?;
            crate::scan::scan_int(st)?;
            word_define(st, a >= 4, p, st.scanner.cur_val)?;
        }
        ASSIGN_DIMEN => {
            let p = st.scanner.cur_chr;
            crate::scan::scan_optional_equals(st)?;
            crate::scan::scan_dimen(st, false, false, false)?;
            word_define(st, a >= 4, p, st.scanner.cur_val)?;
        }
        ASSIGN_GLUE | ASSIGN_MU_GLUE => {
            let p = st.scanner.cur_chr;
            let level = if st.scanner.cur_cmd == ASSIGN_MU_GLUE {
                MU_VAL
            } else {
                GLUE_VAL
            };
            crate::scan::scan_optional_equals(st)?;
            crate::scan::scan_glue(st, level)?;
            trap_zero_glue(st);
            define(st, a >= 4, p, GLUE_REF, st.scanner.cur_val)?;
        }
        XETEX_DEF_CODE => xetex_def_code(st, a >= 4)?,
        DEF_CODE => def_code(st, a >= 4)?,
        DEF_FAMILY => {
            let base = st.scanner.cur_chr;
            crate::scan::scan_math_fam_int(st)?;
            let p = base + st.scanner.cur_val;
            crate::scan::scan_optional_equals(st)?;
            crate::scan::scan_font_ident(st)?;
            define(st, a >= 4, p, DATA, st.scanner.cur_val)?;
        }
        REGISTER | ADVANCE | MULTIPLY | DIVIDE => do_register_command(st, a as i16)?,
        SET_BOX => {
            crate::scan::scan_register_num(st)?;
            let n = if a >= 4 {
                GLOBAL_BOX_FLAG + st.scanner.cur_val
            } else {
                BOX_FLAG + st.scanner.cur_val
            };
            crate::scan::scan_optional_equals(st)?;
            if st.scanner.set_box_allowed {
                scan_box(st, n)?;
            } else {
                crate::output::error_here_with_diagnostic(st, "Improper ");
                crate::output::print_esc(st, "setbox");
                crate::output::capture_to_diagnostic(st, None);
                st.errors.help(&[
                    r"Sorry, \setbox is not allowed after \halign in a display,",
                    r"or between \accent and an accented character.",
                ]);
                crate::errors::error(st)?;
            }
        }
        SET_AUX => alter_aux(st)?,
        SET_PREV_GRAF => alter_prev_graf(st)?,
        SET_PAGE_DIMEN => alter_page_so_far(st)?,
        SET_PAGE_INT => alter_integer(st)?,
        SET_BOX_DIMEN => alter_box_dimen(st)?,
        SET_SHAPE => set_shape(st, a >= 4)?,
        HYPH_DATA => {
            if st.scanner.cur_chr != 1 {
                new_hyph_exceptions(st)?;
            } else if st.config.in_initex_mode {
                new_patterns(st)?;
            } else {
                crate::output::print_err(st, "Patterns can be loaded only by INITEX");
                st.errors.help(&[]);
                crate::errors::error(st)?;
                loop {
                    crate::scan::get_token(st)?;
                    if st.scanner.cur_cmd == RIGHT_BRACE {
                        return Ok(());
                    }
                }
            }
        }
        ASSIGN_FONT_DIMEN => {
            crate::scan::find_font_dimen(st, true)?;
            let k = st.scanner.cur_val;
            crate::scan::scan_optional_equals(st)?;
            crate::scan::scan_dimen(st, false, false, false)?;
            st.fonts.set_font_info_sc(k, st.scanner.cur_val);
        }
        ASSIGN_FONT_INT => {
            let n = st.scanner.cur_chr;
            crate::scan::scan_font_ident(st)?;
            let f = st.scanner.cur_val;
            if n < 2 {
                crate::scan::scan_optional_equals(st)?;
                crate::scan::scan_int(st)?;
                if n == 0 {
                    st.fonts.set_hyphen_char_entry(f, st.scanner.cur_val);
                } else {
                    st.fonts.set_skew_char_entry(f, st.scanner.cur_val);
                }
            } else {
                if st.fonts.is_native_font(f) {
                    crate::scan::scan_glyph_number(st, f)?;
                } else {
                    crate::scan::scan_char_num(st)?;
                }
                let p = st.scanner.cur_val;
                crate::scan::scan_optional_equals(st)?;
                crate::scan::scan_int(st)?;
                match n {
                    LP_CODE_BASE => set_cp_code(f, p as u32, LEFT_SIDE, st.scanner.cur_val),
                    RP_CODE_BASE => set_cp_code(f, p as u32, RIGHT_SIDE, st.scanner.cur_val),
                    _ => {}
                }
            }
        }
        DEF_FONT => new_font(st, a as i16)?,
        SET_INTERACTION => new_interaction(st),
        _ => return Err(Abort::Confusion("prefix")),
    }

    if st.scanner.after_token != 0 {
        st.scanner.cur_tok = st.scanner.after_token;
        crate::input::back_input(st)?;
        st.scanner.after_token = 0;
    }
    Ok(())
}

fn shorthand_def(st: &mut EngineState, global: bool) -> Result<(), Abort> {
    let n = st.scanner.cur_chr;
    crate::scan::get_r_token(st)?;
    let p = st.scanner.cur_cs;
    define(st, global, p, RELAX, TOO_BIG_USV)?;
    crate::scan::scan_optional_equals(st)?;
    let (t, e) = match n {
        CHAR_DEF_CODE => {
            crate::scan::scan_usv_num(st)?;
            (CHAR_GIVEN, st.scanner.cur_val)
        }
        MATH_CHAR_DEF_CODE => {
            crate::scan::scan_fifteen_bit_int(st)?;
            (MATH_GIVEN, st.scanner.cur_val)
        }
        XETEX_MATH_CHAR_NUM_DEF_CODE => {
            crate::scan::scan_xetex_math_char_int(st)?;
            (XETEX_MATH_GIVEN, st.scanner.cur_val)
        }
        XETEX_MATH_CHAR_DEF_CODE => (XETEX_MATH_GIVEN, scan_umath_char(st)?),
        _ => {
            crate::scan::scan_register_num(st)?;
            if st.scanner.cur_val > 255 {
                let level = if n - 2 > MU_VAL as i32 {
                    TOK_VAL
                } else {
                    (n - 2) as u8
                };
                let reg = st.scanner.cur_val;
                let sa = st.mem.create_sa_element(&mut st.scanner, level, reg)?;
                st.mem.add_sa_ref(sa.ptr());
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

fn assign_toks(st: &mut EngineState, global: bool) -> Result<(), Abort> {
    let cs = st.scanner.cur_cs;
    let mut e = false;
    if st.scanner.cur_cmd == TOKS_REGISTER {
        if st.scanner.cur_chr == 0 {
            crate::scan::scan_register_num(st)?;
            if st.scanner.cur_val > 255 {
                let reg = st.scanner.cur_val;
                let sa = st.mem.create_sa_element(&mut st.scanner, TOK_VAL, reg)?;
                st.scanner.cur_chr = sa.idx().0;
                e = true;
            } else {
                st.scanner.cur_chr = TOKS_BASE + st.scanner.cur_val;
            }
        } else {
            e = true;
        }
    } else if st.scanner.cur_chr == LOCAL_BASE + LOCAL_XETEX_INTER_CHAR_TOKS {
        let n = scan_inter_char_pair(st)?;
        let sa = st
            .mem
            .create_sa_element(&mut st.scanner, INTER_CHAR_VAL, n)?;
        st.scanner.cur_chr = sa.idx().0;
        e = true;
    }
    let p = st.scanner.cur_chr;
    crate::scan::scan_optional_equals(st)?;
    crate::scan::get_x_non_blank_non_relax(st)?;

    if st.scanner.cur_cmd != LEFT_BRACE
        && (st.scanner.cur_cmd == TOKS_REGISTER || st.scanner.cur_cmd == ASSIGN_TOKS)
    {
        let list = if st.scanner.cur_cmd == TOKS_REGISTER {
            if st.scanner.cur_chr == 0 {
                crate::scan::scan_register_num(st)?;
                if st.scanner.cur_val < 256 {
                    st.eqtb.toks(st.scanner.cur_val)
                } else {
                    let reg = st.scanner.cur_val;
                    let sa = st.mem.find_sa_element(&mut st.scanner, TOK_VAL, reg);
                    crate::scan::sa_token_list(st, sa)
                }
            } else {
                NodePtr::<SaNode>::at(st.scanner.cur_chr).sa_ptr(&st.mem)
            }
        } else if st.scanner.cur_chr == LOCAL_BASE + LOCAL_XETEX_INTER_CHAR_TOKS {
            let n = scan_inter_char_pair(st)?;
            let sa = st.mem.find_sa_element(&mut st.scanner, INTER_CHAR_VAL, n);
            crate::scan::sa_token_list(st, sa)
        } else {
            MemPtr(st.eqtb.equiv(st.scanner.cur_chr))
        };
        if !list.is_null() {
            st.mem.add_token_ref(list);
        }
        return define_toks(st, e, global, p, list);
    }

    crate::input::back_input(st)?;
    st.scanner.cur_cs = cs;
    let tail = crate::scan::scan_toks(st, false, false)?;
    let def_ref = st.scanner.def_ref;
    if def_ref.link(&st.mem).is_null() {
        define_toks(st, e, global, p, MemPtr::NULL)?;
        st.mem.free_avail(def_ref);
        return Ok(());
    }
    if p == LOCAL_BASE + LOCAL_OUTPUT_ROUTINE && !e {
        let mut tail = tail;
        st.mem
            .store_new_token(&mut tail, RIGHT_BRACE_TOKEN + '}' as i32)?;
        let q = st.mem.get_avail()?;
        q.set_info(&mut st.mem, LEFT_BRACE_TOKEN + '{' as i32);
        let body = def_ref.link(&st.mem);
        q.set_link(&mut st.mem, body);
        def_ref.set_link(&mut st.mem, q);
    }
    define_toks(st, e, global, p, def_ref)
}

fn xetex_def_code(st: &mut EngineState, global: bool) -> Result<(), Abort> {
    let chr = st.scanner.cur_chr;
    let base = if chr == SF_CODE_BASE || chr == MATH_CODE_BASE || chr == DEL_CODE_BASE {
        chr
    } else {
        chr - 1
    };
    crate::scan::scan_usv_num(st)?;
    let p = base + st.scanner.cur_val;
    if chr == SF_CODE_BASE {
        let sf = st.eqtb.sf_code(st.scanner.cur_val) % 65536;
        crate::scan::scan_optional_equals(st)?;
        crate::scan::scan_char_class(st)?;
        let v = st.scanner.cur_val.wrapping_mul(65536).wrapping_add(sf);
        define(st, global, p, DATA, v)?;
    } else if chr == MATH_CODE_BASE {
        crate::scan::scan_optional_equals(st)?;
        crate::scan::scan_xetex_math_char_int(st)?;
        define(st, global, p, DATA, st.scanner.cur_val)?;
    } else if chr == MATH_CODE_BASE + 1 {
        crate::scan::scan_optional_equals(st)?;
        let v = scan_umath_char(st)?;
        define(st, global, p, DATA, v)?;
    } else if chr == DEL_CODE_BASE {
        crate::scan::scan_optional_equals(st)?;
        crate::scan::scan_int(st)?;
        word_define(st, global, p, st.scanner.cur_val)?;
    } else {
        crate::scan::scan_optional_equals(st)?;
        // Extended delimiter code flag.
        let mut n: i32 = 0x4000_0000;
        crate::scan::scan_math_fam_int(st)?;
        n = n.wrapping_add(st.scanner.cur_val.wrapping_mul(0x20_0000));
        crate::scan::scan_usv_num(st)?;
        n = n.wrapping_add(st.scanner.cur_val);
        word_define(st, global, p, n)?;
    }
    Ok(())
}

fn def_code(st: &mut EngineState, global: bool) -> Result<(), Abort> {
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
    crate::scan::scan_usv_num(st)?;
    let p = chr + st.scanner.cur_val;
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_int(st)?;
    if (st.scanner.cur_val < 0 && p < DEL_CODE_BASE) || st.scanner.cur_val > max {
        crate::output::error_here_with_diagnostic(st, "Invalid code (");
        crate::output::print_int(st, st.scanner.cur_val);
        if p < DEL_CODE_BASE {
            crate::output::print(st, "), should be in the range 0..");
        } else {
            crate::output::print(st, "), should be at most ");
        }
        crate::output::print_int(st, max);
        crate::output::capture_to_diagnostic(st, None);
        st.errors
            .help(&["I'm going to use 0 instead of that illegal code value."]);
        crate::errors::error(st)?;
        st.scanner.cur_val = 0;
    }
    if p < MATH_CODE_BASE {
        let v = if p >= SF_CODE_BASE {
            (st.eqtb.int_at(p) / 65536)
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

fn set_shape(st: &mut EngineState, global: bool) -> Result<(), Abort> {
    let q = st.scanner.cur_chr;
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_int(st)?;
    let n = st.scanner.cur_val;
    let p = if n <= 0 {
        MemPtr::NULL
    } else if q > LOCAL_BASE + LOCAL_PAR_SHAPE {
        let words = n / 2 + 1;
        let p = st.mem.get_node(2 * words + 1)?;
        let penalties = NodePtr::<PenaltiesNode>::at(p);
        penalties.set_words(&mut st.mem, words);
        penalties.set_count(&mut st.mem, n);
        for k in 1..=n {
            crate::scan::scan_int(st)?;
            penalties.set_penalty(&mut st.mem, k, st.scanner.cur_val);
        }
        if n & 1 == 0 {
            penalties.set_penalty(&mut st.mem, n + 1, 0);
        }
        p
    } else {
        let p = st.mem.get_node(2 * n + 1)?;
        let shape = NodePtr::<ParShapeNode>::at(p);
        shape.set_lines(&mut st.mem, n);
        for line in 1..=n {
            crate::scan::scan_dimen(st, false, false, false)?;
            shape.set_indent(&mut st.mem, line, st.scanner.cur_val);
            crate::scan::scan_dimen(st, false, false, false)?;
            shape.set_line_width(&mut st.mem, line, st.scanner.cur_val);
        }
        p
    };
    define(st, global, q, SHAPE_REF, p.0)?;
    Ok(())
}
