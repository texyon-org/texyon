//! Alignment: `\halign`, `\valign`, `\noalign`, `\omit` and `\span`.

use crate::cmd::Cmd;
use crate::codes::{ScannerStatus, ValueLevel};
use crate::consts::{
    DLIST, FROZEN_END_TEMPLATE, HMODE, IGNORE_DEPTH, MAX_DIMEN, MMODE, TEX_NULL, VMODE,
};
use crate::errors::Abort;
use crate::input::{TokenType, TOKEN_LIST};
use crate::mem::{ALIGN_HEAD, END_SPAN, HOLD_HEAD, HOLD_HEAD_TOKS, ZERO_GLUE};
use crate::node::{
    AlignColumn, AlignRecord, AlignStackNode, AnyBox, AnyNode, GlueNode, NodePtr, RuleDimen,
    SpanNode, UnsetNode, RUNNING,
};
use crate::packaging::ADDITIONAL;
use crate::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::state::{EngineState, GroupCode, MemState};
use crate::token::{Token, TokenList};
use crate::units::{GlueOrder, GlueSign, MemPtr, Scaled};

use super::assign::{define, do_assignments};
use super::lists::{end_graf, normal_paragraph, off_save};
use super::{append_list, begin_every, CR_CODE, CR_CR_CODE, INNER_HMODE, INNER_VMODE, SPAN_CODE};

pub(super) fn align_error(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.align_state.abs() > 2 {
        crate::output::error_here_with_diagnostic(st, "Misplaced ");
        crate::output::print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
        crate::output::capture_to_diagnostic(st, None);
        if st.scanner.cur_tok == Token::tab(b'&') {
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
            st.scanner.cur_tok = Token::left_brace(b'{');
        } else {
            crate::output::print_err(st, "Missing } inserted");
            st.scanner.align_state -= 1;
            st.scanner.cur_tok = Token::right_brace(b'}');
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
pub(super) fn no_align_error(st: &mut EngineState) -> Result<(), Abort> {
    crate::output::error_here_with_diagnostic(st, "Misplaced ");
    crate::output::print_esc(st, "noalign");
    crate::output::capture_to_diagnostic(st, None);
    st.errors.help(&[
        r"I expect to see \noalign only after the \cr of",
        "an alignment. Proceed, and I'll ignore this case.",
    ]);
    crate::errors::error(st)
}
pub(super) fn omit_error(st: &mut EngineState) -> Result<(), Abort> {
    crate::output::error_here_with_diagnostic(st, "Misplaced ");
    crate::output::print_esc(st, "omit");
    crate::output::capture_to_diagnostic(st, None);
    st.errors.help(&[
        r"I expect to see \omit only after tab marks or the \cr of",
        "an alignment. Proceed, and I'll ignore this case.",
    ]);
    crate::errors::error(st)
}
fn fin_align(st: &mut EngineState) -> Result<(), Abort> {
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
        st.eqtb.dimen_par(DimenPar::DisplayIndent)
    } else {
        Scaled::ZERO
    };

    let mut column = st.mem.first_column::<AlignRecord>();
    while let Some(q) = column {
        let (u_part, v_part) = (q.u_part(&st.mem), q.v_part(&st.mem));
        st.mem.flush_token_list(u_part);
        st.mem.flush_token_list(v_part);
        let next = q.next_column(&st.mem);
        if q.width(&st.mem) == RUNNING {
            q.set_width(&mut st.mem, Scaled::ZERO);
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
    let spec_size = Scaled::from_raw(st.save_stack.saved(1));
    let spec_mode = st.save_stack.saved(0) as i16;
    let head = ALIGN_HEAD.link(&st.mem);
    let p = if st.input.cur_list.mode == INNER_VMODE {
        let rule_save = st.eqtb.dimen_par(DimenPar::OverfullRule);
        st.eqtb.set_dimen_par(DimenPar::OverfullRule, Scaled::ZERO);
        let packed = crate::packaging::hpack(st, head, spec_size, spec_mode)?;
        st.eqtb.set_dimen_par(DimenPar::OverfullRule, rule_save);
        packed
    } else {
        st.mem.for_each_preamble_column(|mem, q| {
            q.set_height(mem, q.width(mem));
            q.set_width(mem, Scaled::ZERO);
        });
        let packed = crate::packaging::vpackage(st, head, spec_size, spec_mode, MAX_DIMEN)?;
        st.mem.for_each_preamble_column(|mem, q| {
            q.set_width(mem, q.height(mem));
            q.set_height(mem, Scaled::ZERO);
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
                row.set_shift_amount(&mut st.mem, o);

                let mut r = row.list_ptr(&st.mem).link(&st.mem);
                let mut proto_column =
                    NodePtr::<UnsetNode>::after_tab_skip(&st.mem, p.list_ptr(&st.mem));
                loop {
                    let (Ok(AnyNode::Unset(col)), Some(mut proto)) =
                        (st.mem.classify(r), proto_column)
                    else {
                        return Err(Abort::Confusion("align"));
                    };
                    let n = col.subtype(&st.mem);
                    let w = proto.width(&st.mem);
                    let mut t = w;
                    let mut u = HOLD_HEAD;
                    col.set_subtype(&mut st.mem, 0);
                    for _ in 0..n {
                        let spec = proto.tab_skip(&st.mem).glue_ptr(&st.mem);
                        let g = st.mem.new_glue(spec)?;
                        u.set_link(&mut st.mem, g.ptr());
                        u = g.ptr();
                        g.set_subtype(&mut st.mem, GluePar::TabSkip.subtype());
                        t += spec.width(&st.mem);
                        match p.glue_sign(&st.mem) {
                            GlueSign::Stretching
                                if spec.stretch_order(&st.mem) == p.glue_order(&st.mem) =>
                            {
                                t += Scaled::round(
                                    p.glue_set(&st.mem) * f64::from(spec.stretch(&st.mem)),
                                );
                            }
                            GlueSign::Shrinking
                                if spec.shrink_order(&st.mem) == p.glue_order(&st.mem) =>
                            {
                                t -= Scaled::round(
                                    p.glue_set(&st.mem) * f64::from(spec.shrink(&st.mem)),
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
                        t += span_w;
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
                        let glue = st.mem.unset_glue(col, t, col.width(&st.mem));
                        let b = col.into_hlist(&mut st.mem);
                        b.set_width(&mut st.mem, w);
                        (AnyBox::HList(b), glue)
                    } else {
                        let width = row.width(&st.mem);
                        col.set_width(&mut st.mem, width);
                        let glue = st.mem.unset_glue(col, t, col.height(&st.mem));
                        let b = col.into_vlist(&mut st.mem);
                        b.set_height(&mut st.mem, w);
                        (AnyBox::VList(b), glue)
                    };
                    let (b, (sign, order, set)) = col;
                    b.set_glue_sign(&mut st.mem, sign);
                    b.set_glue_order(&mut st.mem, order);
                    b.set_glue_set(&mut st.mem, set);
                    b.set_shift_amount(&mut st.mem, Scaled::ZERO);
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
                if o != Scaled::ZERO {
                    let next = rule.link(&st.mem);
                    rule.set_link(&mut st.mem, MemPtr::NULL);
                    let boxed = crate::packaging::hpack(st, rule.ptr(), Scaled::ZERO, ADDITIONAL)?;
                    boxed.set_shift_amount(&mut st.mem, o);
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

    st.mem.flush_node_list(p.ptr())?;
    pop_alignment(st);

    let aux_save_s0 = st.input.cur_list.aux_s0();
    let aux_save_s1 = st.input.cur_list.aux_s1();
    let rows = st.input.cur_list.head.link(&st.mem);
    let rows_tail = st.input.cur_list.tail;
    st.mem.pop_nest(&mut st.input);

    if st.input.cur_list.mode == MMODE {
        do_assignments(st)?;
        if st.scanner.cur_cmd != Cmd::MathShift {
            crate::output::print_err(st, "Missing $$ inserted");
            st.errors.help(&[
                r"Displays can use special alignments (like \eqalignno)",
                "only if nothing but the alignment itself is between $$'s.",
            ]);
            crate::input::back_error(st)?;
        } else {
            crate::math::check_display_end(st)?;
        }
        st.mem.flush_node_list(st.input.cur_list.e_tex_aux)?;
        st.mem.pop_nest(&mut st.input);
        let pre_penalty = st.eqtb.int_par(IntPar::PreDisplayPenalty);
        let pen = st.mem.new_penalty(pre_penalty)?;
        st.input.cur_list.tail_append(&mut st.mem, pen.ptr());
        let above = st.mem.new_param_glue(&st.eqtb, GluePar::AboveDisplaySkip)?;
        st.input.cur_list.tail_append(&mut st.mem, above.ptr());
        st.input.cur_list.tail.set_link(&mut st.mem, rows);
        if !rows.is_null() {
            st.input.cur_list.tail = rows_tail;
        }
        let post_penalty = st.eqtb.int_par(IntPar::PostDisplayPenalty);
        let pen = st.mem.new_penalty(post_penalty)?;
        st.input.cur_list.tail_append(&mut st.mem, pen.ptr());
        let below = st.mem.new_param_glue(&st.eqtb, GluePar::BelowDisplaySkip)?;
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
        let t = q.width(self) + q.tab_skip(self).glue_ptr(self).width(self);
        let mut r = q.span_head().next(self);
        let mut s = END_SPAN;
        s.set_next(self, next.span_head());
        let mut n = 1;
        loop {
            r.set_width(self, r.width(self) - t);
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
    fn unset_glue(
        &self,
        col: NodePtr<UnsetNode>,
        t: Scaled,
        natural: Scaled,
    ) -> (GlueSign, GlueOrder, f64) {
        if t == natural {
            return (GlueSign::Normal, GlueOrder::Normal, 0.0);
        }
        if t > natural {
            let stretch = col.glue_stretch(self);
            let ratio = if stretch == Scaled::ZERO {
                0.0
            } else {
                f64::from(t - natural) / f64::from(stretch)
            };
            return (GlueSign::Stretching, col.glue_order(self), ratio);
        }
        let order = col.glue_sign(self);
        let shrink = col.glue_shrink(self);
        let ratio = if shrink == Scaled::ZERO {
            0.0
        } else if order == GlueOrder::Normal && natural - t > shrink {
            1.0
        } else {
            f64::from(natural - t) / f64::from(shrink)
        };
        (GlueSign::Shrinking, order, ratio)
    }
}

pub(super) fn align_peek(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        st.scanner.align_state = 1_000_000;
        get_x_non_blank_or_protected(st)?;
        match st.scanner.cur_cmd {
            Cmd::NoAlign => {
                crate::scan::scan_left_brace(st)?;
                st.save_stack
                    .new_save_level(GroupCode::NoAlign, st.mem.line)?;
                if st.input.cur_list.mode == INNER_VMODE {
                    normal_paragraph(st)?;
                }
                return Ok(());
            }
            Cmd::RightBrace => return fin_align(st),
            Cmd::CarRet if st.scanner.cur_chr == CR_CR_CODE => {}
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
        if st.scanner.cur_cmd != Cmd::Spacer {
            return Ok(());
        }
    }
}

fn get_preamble_token(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        crate::scan::get_token(st)?;
        while st.scanner.cur_chr == SPAN_CODE && st.scanner.cur_cmd == Cmd::TabMark {
            crate::scan::get_token(st)?;
            if st.scanner.cur_cmd > Cmd::MAX_COMMAND {
                crate::expand::expand(st)?;
                crate::scan::get_token(st)?;
            }
        }
        if st.scanner.cur_cmd == Cmd::Endv {
            return Err(Abort::Fatal(
                "(interwoven alignment preambles are not allowed)",
            ));
        }
        if st.scanner.cur_cmd != Cmd::AssignGlue || st.scanner.chr_loc() != GluePar::TabSkip.loc() {
            return Ok(());
        }
        crate::scan::scan_optional_equals(st)?;
        let g = crate::scan::scan_glue(st, ValueLevel::Glue)?;
        let global = st.eqtb.int_par(IntPar::GlobalDefs) > 0;
        define(
            st,
            global,
            GluePar::TabSkip.loc(),
            Cmd::GlueRef,
            g.ptr().raw(),
        )?;
    }
}

fn init_span(st: &mut EngineState, p: MemPtr) -> Result<(), Abort> {
    crate::mem::push_nest(st)?;
    if st.input.cur_list.mode == INNER_HMODE {
        st.input.cur_list.set_aux_s0(1000);
    } else {
        st.input.cur_list.set_prev_depth(IGNORE_DEPTH);
        normal_paragraph(st)?;
    }
    st.scanner.cur_span = p;
    Ok(())
}

fn init_row(st: &mut EngineState) -> Result<(), Abort> {
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
    glue.set_subtype(&mut st.mem, GluePar::TabSkip.subtype());
    st.scanner.cur_align = tab_skip.link(&st.mem);
    st.scanner.cur_tail = st.scanner.cur_head;
    st.scanner.cur_pre_tail = st.scanner.cur_pre_head;
    init_span(st, st.scanner.cur_align)
}

fn init_col(st: &mut EngineState) -> Result<(), Abort> {
    let column = NodePtr::<AlignRecord>::at(st.scanner.cur_align);
    column.set_extra_info(&mut st.mem, i32::from(st.scanner.cur_cmd));
    if st.scanner.cur_cmd == Cmd::Omit {
        st.scanner.align_state = 0;
    } else {
        crate::input::back_input(st)?;
        crate::input::begin_token_list(st, column.u_part(&st.mem), TokenType::UTemplate)?;
    }
    Ok(())
}

fn push_alignment(st: &mut EngineState) -> Result<(), Abort> {
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

fn pop_alignment(st: &mut EngineState) {
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
    /// Copies token list `r` through `HOLD_HEAD_TOKS` and returns the copy.
    fn copy_token_list(&mut self, mut r: TokenList) -> Result<TokenList, Abort> {
        let mut q = HOLD_HEAD_TOKS;
        while !r.is_null() {
            self.store_new_token(&mut q, r.token(self))?;
            r = r.link(self);
        }
        q.set_link(self, TokenList::NULL);
        Ok(HOLD_HEAD_TOKS.link(self))
    }

    /// A fresh preamble column with an empty span list and no entries yet.
    fn new_align_record(&mut self) -> Result<NodePtr<AlignRecord>, Abort> {
        let nb = self.new_null_box()?;
        let column = NodePtr::<AlignRecord>::at(nb.idx());
        column.span_head().set_next(self, END_SPAN);
        column.set_width(self, RUNNING);
        Ok(column)
    }
}

fn fin_col(st: &mut EngineState) -> Result<bool, Abort> {
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
            g.set_subtype(&mut st.mem, GluePar::TabSkip.subtype());
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
            let u = crate::packaging::hpack(st, head, Scaled::ZERO, ADDITIONAL)?;
            st.scanner.cur_tail = st.scanner.adjust_tail;
            st.scanner.adjust_tail = MemPtr::NULL;
            st.scanner.cur_pre_tail = st.scanner.pre_adjust_tail;
            st.scanner.pre_adjust_tail = MemPtr::NULL;
            (u, u.width(&st.mem))
        } else {
            let u = crate::packaging::vpackage(st, head, Scaled::ZERO, ADDITIONAL, Scaled::ZERO)?;
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
            if n > i32::from(u16::MAX) {
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
        let o = st.scanner.total_stretch.max_order();
        u.set_glue_order(&mut st.mem, o);
        u.set_glue_stretch(&mut st.mem, st.scanner.total_stretch[o]);
        let o = st.scanner.total_shrink.max_order();
        u.set_glue_sign(&mut st.mem, o);
        u.set_glue_shrink(&mut st.mem, st.scanner.total_shrink[o]);
        st.mem.pop_nest(&mut st.input);
        st.input.cur_list.tail_append(&mut st.mem, u.ptr());
        let spec = cur_align.tab_skip(&st.mem).glue_ptr(&st.mem);
        let g = st.mem.new_glue(spec)?;
        st.input.cur_list.tail_append(&mut st.mem, g.ptr());
        g.set_subtype(&mut st.mem, GluePar::TabSkip.subtype());
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

fn fin_row(st: &mut EngineState) -> Result<(), Abort> {
    let head = st.input.cur_list.head.link(&st.mem);
    let p = if st.input.cur_list.mode == INNER_HMODE {
        let row = crate::packaging::hpack(st, head, Scaled::ZERO, ADDITIONAL)?;
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
        let row = crate::packaging::vpackage(st, head, Scaled::ZERO, ADDITIONAL, MAX_DIMEN)?;
        st.mem.pop_nest(&mut st.input);
        st.input.cur_list.tail_append(&mut st.mem, row.ptr());
        st.input.cur_list.set_aux_s0(1000);
        row
    };
    p.into_unset(&mut st.mem)
        .set_glue_stretch(&mut st.mem, Scaled::ZERO);
    begin_every(st, TokPar::EveryCr, TokenType::EveryCr)?;
    align_peek(st)
}

pub(super) fn init_align(st: &mut EngineState) -> Result<(), Abort> {
    let save_cs_ptr = st.scanner.cur_cs;
    push_alignment(st)?;
    st.scanner.align_state = -1_000_000;

    if st.input.cur_list.mode == MMODE
        && (st.input.cur_list.tail != st.input.cur_list.head
            || st.input.cur_list.incompleat_noad().is_some())
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
        let prev_depth = st.input.nest[(st.input.nest_ptr - 2) as usize].prev_depth();
        st.input.cur_list.set_prev_depth(prev_depth);
    } else if st.input.cur_list.mode > 0 {
        st.input.cur_list.mode = -st.input.cur_list.mode;
    }

    crate::scan::scan_spec(st, GroupCode::Align, false)?;
    ALIGN_HEAD.set_link(&mut st.mem, MemPtr::NULL);
    st.scanner.cur_align = ALIGN_HEAD;
    st.scanner.cur_loop = MemPtr::NULL;
    st.scanner.scanner_status = ScannerStatus::Aligning;
    st.scanner.warning_index = save_cs_ptr;
    st.scanner.align_state = -1_000_000;

    loop {
        let g = st.mem.new_param_glue(&st.eqtb, GluePar::TabSkip)?.ptr();
        st.scanner.cur_align.set_link(&mut st.mem, g);
        st.scanner.cur_align = g;
        if st.scanner.cur_cmd == Cmd::CarRet {
            break;
        }

        let mut p = HOLD_HEAD_TOKS;
        p.set_link(&mut st.mem, TokenList::NULL);
        loop {
            get_preamble_token(st)?;
            if st.scanner.cur_cmd == Cmd::MacParam {
                break;
            }
            if (Cmd::TabMark..=Cmd::CarRet).contains(&st.scanner.cur_cmd)
                && st.scanner.align_state == -1_000_000
            {
                if p == HOLD_HEAD_TOKS
                    && st.scanner.cur_loop.is_null()
                    && st.scanner.cur_cmd == Cmd::TabMark
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
            } else if st.scanner.cur_cmd != Cmd::Spacer || p != HOLD_HEAD_TOKS {
                st.mem.store_new_token(&mut p, st.scanner.cur_tok)?;
            }
        }

        let column = st.mem.new_align_record()?;
        st.scanner.cur_align.set_link(&mut st.mem, column.ptr());
        st.scanner.cur_align = column.ptr();
        let u_part = HOLD_HEAD_TOKS.link(&st.mem);
        column.set_u_part(&mut st.mem, u_part);

        p = HOLD_HEAD_TOKS;
        p.set_link(&mut st.mem, TokenList::NULL);
        loop {
            get_preamble_token(st)?;
            if (Cmd::TabMark..=Cmd::CarRet).contains(&st.scanner.cur_cmd)
                && st.scanner.align_state == -1_000_000
            {
                break;
            }
            if st.scanner.cur_cmd == Cmd::MacParam {
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
            .store_new_token(&mut p, Token::from_cs(FROZEN_END_TEMPLATE))?;
        let v_part = HOLD_HEAD_TOKS.link(&st.mem);
        column.set_v_part(&mut st.mem, v_part);
    }

    st.scanner.scanner_status = ScannerStatus::Normal;
    st.save_stack
        .new_save_level(GroupCode::Align, st.mem.line)?;
    begin_every(st, TokPar::EveryCr, TokenType::EveryCr)?;
    align_peek(st)
}

pub(super) fn do_endv(st: &mut EngineState) -> Result<(), Abort> {
    fn is_v_template_end(r: &crate::state::InputState) -> bool {
        r.state == TOKEN_LIST && r.loc == TEX_NULL && r.token_type() == TokenType::VTemplate
    }

    st.input.base_ptr = st.input.input_ptr;
    st.input.input_stack[st.input.base_ptr as usize] = st.input.cur_input;
    while st.input.input_stack[st.input.base_ptr as usize].state == TOKEN_LIST
        && st.input.input_stack[st.input.base_ptr as usize].loc == TEX_NULL
        && st.input.input_stack[st.input.base_ptr as usize].token_type() != TokenType::VTemplate
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
