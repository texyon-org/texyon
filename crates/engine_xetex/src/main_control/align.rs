//! Alignment: `\halign`, `\valign`, `\noalign`, `\omit` and `\span`.

use crate::cmd::Cmd;
use crate::codes::{ScannerStatus, ValueLevel};
use crate::command::Command;
use crate::command::TabMark;
use crate::consts::{DLIST, FROZEN_END_TEMPLATE, HMODE, IGNORE_DEPTH, MAX_DIMEN, MMODE, VMODE};
use crate::errors::Abort;
use crate::input::{TokenType, TOKEN_LIST};
use crate::mem::{ALIGN_HEAD, END_SPAN, HOLD_HEAD, HOLD_HEAD_TOKS, ZERO_GLUE};
use crate::node::Stored;
use crate::node::{
    link_of, AlignColumn, AlignRecord, AlignStackNode, AnyNode, GlueNode, NodePtr, RuleDimen,
    SpanNode, UnsetNode, RUNNING,
};
use crate::packaging::PackSpec;
use crate::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::state::{EngineState, GroupCode, MemState};
use crate::token::{Token, TokenList};
use crate::units::{GlueOrder, GlueSign, MemPtr, Orientation, Scaled};

use super::assign::{define, do_assignments};
use super::lists::{end_graf, normal_paragraph, off_save};
use super::{append_list, begin_every, CR_CODE, INNER_HMODE, INNER_VMODE, SPAN_CODE};

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

    let in_display = st.input.nest[st.input.nest.len() - 1].mode == MMODE;
    let o = if in_display {
        st.eqtb.dimen_par(DimenPar::DisplayIndent)
    } else {
        Scaled::ZERO
    };

    let mut column = st.mem.first_column::<AlignRecord>();
    while let Some(q) = column {
        let (u_part, v_part) = (st.mem[q].u_part, st.mem[q].v_part);
        st.mem.flush_token_list(u_part);
        st.mem.flush_token_list(v_part);
        let next = q.next_column(&st.mem);
        if st.mem[q].width == RUNNING {
            st.mem[q].width = Scaled::ZERO;
            let glue = q.tab_skip(&st.mem);
            let spec = st.mem[glue].glue_ptr;
            if spec != ZERO_GLUE {
                st.mem.add_glue_ref(ZERO_GLUE);
                st.mem.delete_glue_ref(spec);
                st.mem[glue].glue_ptr = ZERO_GLUE;
            }
        }
        if let Some(next) = next {
            if st.mem[q.span_head()].next != END_SPAN {
                st.mem.merge_spans(q, next);
            }
        }
        q.into_unset(&mut st.mem);
        column = next;
    }

    let spec = st.save_stack.pop_spec()?;
    st.scanner.pack_begin_line = -st.input.cur_list.mode_line;
    let head = ALIGN_HEAD.link(&st.mem);
    let p = if st.input.cur_list.mode == INNER_VMODE {
        let rule_save = st.eqtb.dimen_par(DimenPar::OverfullRule);
        st.eqtb.set_dimen_par(DimenPar::OverfullRule, Scaled::ZERO);
        let packed = crate::packaging::hpack(st, head, spec)?;
        st.eqtb.set_dimen_par(DimenPar::OverfullRule, rule_save);
        packed
    } else {
        st.mem.for_each_preamble_column(|mem, q| {
            mem[q].height = mem[q].width;
            mem[q].width = Scaled::ZERO;
        });
        let packed = crate::packaging::vpackage(st, head, spec, MAX_DIMEN)?;
        st.mem.for_each_preamble_column(|mem, q| {
            mem[q].width = mem[q].height;
            mem[q].height = Scaled::ZERO;
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
                    let b = row.into_box(&mut st.mem, Orientation::Horizontal);
                    let width = st.mem[p].width;
                    st.mem[b].width = width;
                    if in_display {
                        st.mem[b].subtype = DLIST;
                    }
                    b
                } else {
                    let b = row.into_box(&mut st.mem, Orientation::Vertical);
                    let height = st.mem[p].height;
                    st.mem[b].height = height;
                    b
                };
                let (order, sign, set) = (
                    st.mem[p].glue_order,
                    st.mem[p].glue_sign,
                    st.mem[p].glue_set,
                );
                st.mem[row].glue_order = order;
                st.mem[row].glue_sign = sign;
                st.mem[row].glue_set = set;
                st.mem[row].shift_amount = o;

                let mut r = st.mem[row].list_ptr.link(&st.mem);
                let mut proto_column =
                    NodePtr::<UnsetNode>::after_tab_skip(&st.mem, st.mem[p].list_ptr);
                loop {
                    let (Ok(AnyNode::Unset(col)), Some(mut proto)) =
                        (st.mem.classify(r), proto_column)
                    else {
                        return Err(Abort::Confusion("align"));
                    };
                    let n = st.mem[col].span_count;
                    let w = st.mem[proto].width;
                    let mut t = w;
                    let mut u = HOLD_HEAD;
                    st.mem[col].span_count = 0;
                    for _ in 0..n {
                        let spec = st.mem[proto.tab_skip(&st.mem)].glue_ptr;
                        let g = st.mem.new_glue(spec)?;
                        u.set_link(&mut st.mem, g.ptr());
                        u = g.ptr();
                        st.mem[g].subtype = GluePar::TabSkip.subtype();
                        t += st.mem[spec].width;
                        match st.mem[p].glue_sign.get() {
                            Some(GlueSign::Stretching)
                                if st.mem[spec].stretch_order == st.mem[p].glue_order =>
                            {
                                t += Scaled::round(
                                    st.mem[p].glue_set * f64::from(st.mem[spec].stretch),
                                );
                            }
                            Some(GlueSign::Shrinking)
                                if st.mem[spec].shrink_order == st.mem[p].glue_order =>
                            {
                                t -= Scaled::round(
                                    st.mem[p].glue_set * f64::from(st.mem[spec].shrink),
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
                        let span_w = st.mem[proto].width;
                        t += span_w;
                        if st.input.cur_list.mode == INNER_VMODE {
                            st.mem[nb].width = span_w;
                        } else {
                            let nb = &mut st.mem[nb];
                            nb.orientation = Orientation::Vertical.into();
                            nb.height = span_w;
                        }
                    }
                    let col = if st.input.cur_list.mode == INNER_VMODE {
                        let (h, d) = (st.mem[row].height, st.mem[row].depth);
                        st.mem[col].height = h;
                        st.mem[col].depth = d;
                        let glue = st.mem.unset_glue(col, t, st.mem[col].width);
                        let b = col.into_box(&mut st.mem, Orientation::Horizontal);
                        st.mem[b].width = w;
                        (b, glue)
                    } else {
                        let width = st.mem[row].width;
                        st.mem[col].width = width;
                        let glue = st.mem.unset_glue(col, t, st.mem[col].height);
                        let b = col.into_box(&mut st.mem, Orientation::Vertical);
                        st.mem[b].height = w;
                        (b, glue)
                    };
                    let (b, (sign, order, set)) = col;
                    st.mem[b].glue_sign = sign.into();
                    st.mem[b].glue_order = order;
                    st.mem[b].glue_set = set;
                    st.mem[b].shift_amount = Scaled::ZERO;
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
                let (w, h, d) = (st.mem[p].width, st.mem[p].height, st.mem[p].depth);
                if st.mem[rule].width.get() == RuleDimen::Running {
                    st.mem[rule].width = RuleDimen::Set(w).into();
                }
                if st.mem[rule].height.get() == RuleDimen::Running {
                    st.mem[rule].height = RuleDimen::Set(h).into();
                }
                if st.mem[rule].depth.get() == RuleDimen::Running {
                    st.mem[rule].depth = RuleDimen::Set(d).into();
                }
                if o != Scaled::ZERO {
                    let next = st.mem[rule].link;
                    st.mem[rule].link = MemPtr::NULL;
                    let boxed = crate::packaging::hpack(st, rule.ptr(), PackSpec::NATURAL)?;
                    st.mem[boxed].shift_amount = o;
                    st.mem[boxed].link = next;
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
    pop_alignment(st)?;

    let aux_save_s0 = st.input.cur_list.aux_s0();
    let aux_save_s1 = st.input.cur_list.aux_s1();
    let rows = st.input.cur_list.head.link(&st.mem);
    let rows_tail = st.input.cur_list.tail;
    st.mem.pop_nest(&mut st.input)?;

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
        st.mem.pop_nest(&mut st.input)?;
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
        let t = self[q].width + self[self[q.tab_skip(self)].glue_ptr].width;
        let mut r = self[q.span_head()].next;
        let mut s = END_SPAN;
        self[s].next = next.span_head();
        let mut n = 1;
        loop {
            self[r].width -= t;
            let u = self[r].next;
            while self[r].span_count > n {
                s = self[s].next;
                n = self[self[s].next].span_count + 1;
            }
            if self[r].span_count < n {
                self[r].next = self[s].next;
                self[s].next = r;
                self[r].span_count -= 1;
                s = r;
            } else {
                let merged = self[s].next;
                if self[r].width > self[merged].width {
                    self[merged].width = self[r].width;
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
    ) -> (GlueSign, Stored<GlueOrder>, f64) {
        if t == natural {
            return (GlueSign::Normal, GlueOrder::Normal.into(), 0.0);
        }
        if t > natural {
            let stretch = self[col].glue_stretch;
            let ratio = if stretch == Scaled::ZERO {
                0.0
            } else {
                f64::from(t - natural) / f64::from(stretch)
            };
            return (GlueSign::Stretching, self[col].glue_order, ratio);
        }
        let order = self[col].glue_sign;
        let shrink = self[col].glue_shrink;
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
            Cmd::CarRet if st.scanner.try_command() == Some(Command::CarRet { crcr: true }) => {}
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
        while st.scanner.try_command() == Some(Command::TabMark(TabMark::Span)) {
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
        if st.scanner.try_command() != Some(Command::AssignGlue(GluePar::TabSkip.loc())) {
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
    let spec = st.mem[tab_skip].glue_ptr;
    let glue = st.mem.new_glue(spec)?;
    st.input.cur_list.tail_append(&mut st.mem, glue.ptr());
    st.mem[glue].subtype = GluePar::TabSkip.subtype();
    st.scanner.cur_align = st.mem[tab_skip].link;
    st.scanner.cur_tail = st.scanner.cur_head;
    st.scanner.cur_pre_tail = st.scanner.cur_pre_head;
    init_span(st, st.scanner.cur_align)
}

fn init_col(st: &mut EngineState) -> Result<(), Abort> {
    let column = NodePtr::<AlignRecord>::at(st.scanner.cur_align);
    st.mem[column].extra_info = i32::from(st.scanner.cur_cmd);
    if st.scanner.cur_cmd == Cmd::Omit {
        st.scanner.align_state = 0;
    } else {
        crate::input::back_input(st)?;
        crate::input::begin_token_list(st, st.mem[column].u_part, TokenType::UTemplate)?;
    }
    Ok(())
}

fn push_alignment(st: &mut EngineState) -> Result<(), Abort> {
    let p = NodePtr::<AlignStackNode>::alloc(&mut st.mem)?;
    let (mem, sc) = (&mut st.mem, &st.scanner);
    mem[p].link = link_of(sc.align_ptr);
    mem[p].cur_align = sc.cur_align;
    mem[p].preamble = ALIGN_HEAD.link(mem);
    mem[p].cur_span = sc.cur_span;
    mem[p].cur_loop = sc.cur_loop;
    mem[p].align_state = sc.align_state;
    mem[p].cur_head = sc.cur_head;
    mem[p].cur_tail = sc.cur_tail;
    mem[p].cur_pre_head = sc.cur_pre_head;
    mem[p].cur_pre_tail = sc.cur_pre_tail;
    st.scanner.align_ptr = Some(p);
    st.scanner.cur_head = st.mem.get_avail()?;
    st.scanner.cur_pre_head = st.mem.get_avail()?;
    Ok(())
}

fn pop_alignment(st: &mut EngineState) -> Result<(), Abort> {
    let p = st.scanner.align_ptr.ok_or(Abort::Confusion("align"))?;
    st.mem.free_avail(st.scanner.cur_head);
    st.mem.free_avail(st.scanner.cur_pre_head);
    let (mem, sc) = (&mut st.mem, &mut st.scanner);
    sc.cur_tail = mem[p].cur_tail;
    sc.cur_head = mem[p].cur_head;
    sc.cur_pre_tail = mem[p].cur_pre_tail;
    sc.cur_pre_head = mem[p].cur_pre_head;
    sc.align_state = mem[p].align_state;
    sc.cur_loop = mem[p].cur_loop;
    sc.cur_span = mem[p].cur_span;
    ALIGN_HEAD.set_link(mem, mem[p].preamble);
    sc.cur_align = mem[p].cur_align;
    sc.align_ptr = p.next(mem);
    p.free(mem);
    Ok(())
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
        self[column.span_head()].next = END_SPAN;
        self[column].width = RUNNING;
        Ok(column)
    }
}

fn fin_col(st: &mut EngineState) -> Result<bool, Abort> {
    if st.scanner.cur_align.is_null() {
        return Err(Abort::Confusion("endv"));
    }
    let cur_align = NodePtr::<AlignRecord>::at(st.scanner.cur_align);
    let q = st.mem[cur_align].link;
    if q.is_null() {
        return Err(Abort::Confusion("endv"));
    }
    if st.scanner.align_state < 500_000 {
        return Err(Abort::Fatal(
            "(interwoven alignment preambles are not allowed)",
        ));
    }
    let mut p = q.link(&st.mem);
    if p.is_null() && st.mem[cur_align].extra_info < CR_CODE {
        if !st.scanner.cur_loop.is_null() {
            let column = st.mem.new_align_record()?;
            q.set_link(&mut st.mem, column.ptr());
            p = column.ptr();
            let template = NodePtr::<AlignRecord>::at(st.scanner.cur_loop.link(&st.mem));
            let u_template = st.mem[template].u_part;
            let u_part = st.mem.copy_token_list(u_template)?;
            st.mem[column].u_part = u_part;
            let v_template = st.mem[template].v_part;
            let v_part = st.mem.copy_token_list(v_template)?;
            st.mem[column].v_part = v_part;
            let tab_skip = template.tab_skip(&st.mem);
            st.scanner.cur_loop = tab_skip.ptr();
            let spec = st.mem[tab_skip].glue_ptr;
            let g = st.mem.new_glue(spec)?;
            st.mem[column].link = g.ptr();
            st.mem[g].subtype = GluePar::TabSkip.subtype();
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
            st.mem[cur_align].extra_info = CR_CODE;
            crate::errors::error(st)?;
        }
    }
    if st.mem[cur_align].extra_info != SPAN_CODE {
        crate::save::unsave(st)?;
        st.save_stack
            .new_save_level(GroupCode::Align, st.mem.line)?;
        let head = st.input.cur_list.head.link(&st.mem);
        let (u, w) = if st.input.cur_list.mode == INNER_HMODE {
            st.scanner.adjust_tail = st.scanner.cur_tail;
            st.scanner.pre_adjust_tail = st.scanner.cur_pre_tail;
            let u = crate::packaging::hpack(st, head, PackSpec::NATURAL)?;
            st.scanner.cur_tail = st.scanner.adjust_tail;
            st.scanner.adjust_tail = MemPtr::NULL;
            st.scanner.cur_pre_tail = st.scanner.pre_adjust_tail;
            st.scanner.pre_adjust_tail = MemPtr::NULL;
            (u, st.mem[u].width)
        } else {
            let u = crate::packaging::vpackage(st, head, PackSpec::NATURAL, Scaled::ZERO)?;
            (u, st.mem[u].height)
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
            while st.mem[st.mem[q].next].span_count < n {
                q = st.mem[q].next;
            }
            let next = st.mem[q].next;
            if st.mem[next].span_count > n {
                let s = NodePtr::<SpanNode>::alloc(&mut st.mem)?;
                st.mem[s].next = next;
                st.mem[s].span_count = n;
                st.mem[q].next = s;
                st.mem[s].width = w;
            } else if st.mem[next].width < w {
                st.mem[next].width = w;
            }
        } else if w > st.mem[cur_align].width {
            st.mem[cur_align].width = w;
        }
        let u = u.into_unset(&mut st.mem);
        st.mem[u].span_count = u16::try_from(n).map_err(|_| Abort::Confusion("span"))?;
        let o = st.scanner.total_stretch.max_order();
        st.mem[u].glue_order = o.into();
        st.mem[u].glue_stretch = st.scanner.total_stretch[o];
        let o = st.scanner.total_shrink.max_order();
        st.mem[u].glue_sign = o.into();
        st.mem[u].glue_shrink = st.scanner.total_shrink[o];
        st.mem.pop_nest(&mut st.input)?;
        st.input.cur_list.tail_append(&mut st.mem, u.ptr());
        let spec = st.mem[cur_align.tab_skip(&st.mem)].glue_ptr;
        let g = st.mem.new_glue(spec)?;
        st.input.cur_list.tail_append(&mut st.mem, g.ptr());
        st.mem[g].subtype = GluePar::TabSkip.subtype();
        if st.mem[cur_align].extra_info >= CR_CODE {
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
        let row = crate::packaging::hpack(st, head, PackSpec::NATURAL)?;
        st.mem.pop_nest(&mut st.input)?;
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
        let row = crate::packaging::vpackage(st, head, PackSpec::NATURAL, MAX_DIMEN)?;
        st.mem.pop_nest(&mut st.input)?;
        st.input.cur_list.tail_append(&mut st.mem, row.ptr());
        st.input.cur_list.set_aux_s0(1000);
        row
    };
    let unset = p.into_unset(&mut st.mem);
    st.mem[unset].glue_stretch = Scaled::ZERO;
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
        let prev_depth = st.input.nest[st.input.nest.len() - 2].prev_depth();
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
        st.mem[column].u_part = u_part;

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
        st.mem[column].v_part = v_part;
    }

    st.scanner.scanner_status = ScannerStatus::Normal;
    st.save_stack
        .new_save_level(GroupCode::Align, st.mem.line)?;
    begin_every(st, TokPar::EveryCr, TokenType::EveryCr)?;
    align_peek(st)
}

pub(super) fn do_endv(st: &mut EngineState) -> Result<(), Abort> {
    fn is_v_template_end(r: &crate::state::InputState) -> bool {
        r.state == TOKEN_LIST && r.token_loc().is_null() && r.token_type() == TokenType::VTemplate
    }

    st.input.base_ptr = st.input.input_stack.len();
    st.input.input_stack.park(st.input.cur_input);
    while st.input.input_stack[st.input.base_ptr].state == TOKEN_LIST
        && st.input.input_stack[st.input.base_ptr]
            .token_loc()
            .is_null()
        && st.input.input_stack[st.input.base_ptr].token_type() != TokenType::VTemplate
    {
        st.input.base_ptr -= 1;
    }
    if !is_v_template_end(&st.input.input_stack[st.input.base_ptr]) {
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
