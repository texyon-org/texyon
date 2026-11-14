//! Alignment: `\halign`, `\valign`, `\noalign`, `\omit` and `\span`.

use crate::cmd::Cmd;
use crate::codes::{ScannerStatus, ValueLevel};
use crate::command::Command;
use crate::command::TabMarkCode;
use crate::consts::{IGNORE_DEPTH, MAX_DIMEN};
use crate::cs::Frozen;
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::input::{TOKEN_LIST, TokenType};
use crate::mem::{ALIGN_HEAD, END_SPAN, HOLD_HEAD, ZERO_GLUE};
use crate::node::Stored;
use crate::node::{
    AlignColumn, AlignRecord, AnyNode, BoxLr, GlueKind, GlueNode, HeadNode, NodePtr, RUNNING,
    RuleDimen, SpanNode, UnsetNode,
};
use crate::packaging::{AdjustTails, PackOrigin, PackSpec};
use crate::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::state::{EngineState, GroupCode, MemState};
use crate::token::{Token, TokenList};
use crate::units::{GlueOrder, GlueSign, MemPtr, Orientation, Scaled};

use super::assign::do_assignments;
use super::lists::{end_graf, normal_paragraph, off_save};
use super::{CR_CODE, SPAN_CODE, append_list, begin_every};
use crate::mode::{Mode, ModeName};

pub(super) fn align_error(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.align_state.abs() > 2 {
        let help: &[&'static str] = if st.scanner.cur_tok == Token::tab(b'&') {
            &[
                "I can't figure out why you would want to use a tab mark",
                "here. If you just want an ampersand, the remedy is",
                r"simple: Just type `I\&' now. But if some right brace",
                "up above has ended a previous alignment prematurely,",
                "you're probably due for more error messages, and you",
                "might try typing `S' now just to see what is salvageable.",
            ]
        } else {
            &[
                "I can't figure out why you would want to use a tab mark",
                r"or \cr or \span just now. If something like a right brace",
                "up above has ended a previous alignment prematurely,",
                "you're probably due for more error messages, and you",
                "might try typing `S' now just to see what is salvageable.",
            ]
        };
        crate::errors::error(st, ("Misplaced ", st.scanner.cur), help)?;
    } else {
        crate::input::back_input(st)?;
        let message = if st.scanner.align_state < 0 {
            st.scanner.align_state += 1;
            st.scanner.cur_tok = Token::left_brace(b'{');
            "Missing { inserted"
        } else {
            st.scanner.align_state -= 1;
            st.scanner.cur_tok = Token::right_brace(b'}');
            "Missing } inserted"
        };
        crate::errors::ins_error(
            st,
            message,
            &[
                "I've put in what seems to be necessary to fix",
                "the current column of the current alignment.",
                "Try to go on, since this might almost work.",
            ],
        )?;
    }
    Ok(())
}
pub(super) fn no_align_error(st: &mut EngineState) -> Result<(), Abort> {
    crate::errors::error(
        st,
        ("Misplaced ", crate::output::Esc("noalign")),
        &[
            r"I expect to see \noalign only after the \cr of",
            "an alignment. Proceed, and I'll ignore this case.",
        ],
    )
}
pub(super) fn omit_error(st: &mut EngineState) -> Result<(), Abort> {
    crate::errors::error(
        st,
        ("Misplaced ", crate::output::Esc("omit")),
        &[
            r"I expect to see \omit only after tab marks or the \cr of",
            "an alignment. Proceed, and I'll ignore this case.",
        ],
    )
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

    let in_display = st.input.nest[st.input.nest.len() - 1].mode.name() == ModeName::DisplayMath;
    let o = if in_display {
        st.eqtb.dimen_par(DimenPar::DisplayIndent)
    } else {
        Scaled::ZERO
    };

    let mut column = st.mem.first_column::<AlignRecord>();
    while let Some(q) = column {
        let (u_part, v_part) = (st.mem[q].u_part, st.mem[q].v_part);
        st.mem.tokens.flush(u_part);
        st.mem.tokens.flush(v_part);
        let next = q.next_column(&st.mem);
        if st.mem[q].width == RUNNING {
            st.mem[q].width = Scaled::ZERO;
            let glue = q.tab_skip(&st.mem)?;
            let spec = st.mem[glue].glue_ptr;
            if spec != ZERO_GLUE {
                st.mem.add_glue_ref(ZERO_GLUE);
                st.mem.delete_glue_ref(spec);
                st.mem[glue].glue_ptr = ZERO_GLUE;
            }
        }
        if let Some(next) = next
            && st.mem[q].span_list != Some(END_SPAN)
        {
            st.mem.merge_spans(q, next)?;
        }
        q.into_unset(&mut st.mem);
        column = next;
    }

    let spec = st.save_stack.pop_spec()?;
    let origin = PackOrigin::from_begin_line(-st.input.cur_list.mode_line);
    let head = ALIGN_HEAD.link(&st.mem);
    let p = if st.input.cur_list.mode.name() == ModeName::InternalVertical {
        let rule_save = st.eqtb.dimen_par(DimenPar::OverfullRule);
        st.eqtb.set_dimen_par(DimenPar::OverfullRule, Scaled::ZERO);
        let packed = crate::packaging::hpack_with(st, head, spec, origin, None)?.r_box;
        st.eqtb.set_dimen_par(DimenPar::OverfullRule, rule_save);
        packed
    } else {
        st.mem.for_each_preamble_column(|mem, q| {
            mem[q].height = mem[q].width;
            mem[q].width = Scaled::ZERO;
        });
        let packed = crate::packaging::vpackage_with(st, head, spec, MAX_DIMEN, origin)?.r_box;
        st.mem.for_each_preamble_column(|mem, q| {
            mem[q].width = mem[q].height;
            mem[q].height = Scaled::ZERO;
        });
        packed
    };

    let mut s = st.input.cur_list.head;
    let mut q = s.link(&st.mem);
    while let Some(mut cur) = q {
        match st.mem.classify_non_char(q) {
            Some(AnyNode::Unset(row)) => {
                let row = if st.input.cur_list.mode.name() == ModeName::InternalVertical {
                    let b = row.into_box(&mut st.mem, Orientation::Horizontal);
                    let width = st.mem[p].width;
                    st.mem[b].width = width;
                    if in_display {
                        st.mem[b].set_lr(BoxLr::Dlist);
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
                    st.mem[p].glue_set.get(),
                );
                let node = &mut st.mem[row];
                node.glue_order = order;
                node.glue_sign = sign;
                node.glue_set = crate::node::GlueRatio::new(set);
                node.shift_amount = o;

                let mut r = st.mem[row].list_ptr.or_confusion("align")?.link(&st.mem);
                let first_proto = st.mem[p].list_ptr.or_confusion("align")?;
                let mut proto_column = NodePtr::<UnsetNode>::after_tab_skip(&st.mem, first_proto);
                loop {
                    let (Some(mut cell), Some(mut proto)) = (r, proto_column) else {
                        return Err(Abort::Confusion("align"));
                    };
                    let Ok(AnyNode::Unset(col)) = st.mem.classify(cell) else {
                        return Err(Abort::Confusion("align"));
                    };
                    let n = st.mem[col].span_count;
                    let w = st.mem[proto].width;
                    let mut t = w;
                    let mut u = HOLD_HEAD;
                    st.mem[col].span_count = 0;
                    for _ in 0..n {
                        let spec = st.mem[proto.tab_skip(&st.mem)?].glue_ptr;
                        let g = st.mem.new_glue(spec)?;
                        u.set_link(&mut st.mem, g.ptr());
                        u = g.idx();
                        st.mem[g].subtype = GlueKind::Param(GluePar::TabSkip).into();
                        t += st.mem[spec].width;
                        match st.mem[p].glue_sign.get() {
                            Some(GlueSign::Stretching)
                                if st.mem[spec].stretch_order == st.mem[p].glue_order =>
                            {
                                t += Scaled::round(
                                    st.mem[p].glue_set.get() * f64::from(st.mem[spec].stretch),
                                );
                            }
                            Some(GlueSign::Shrinking)
                                if st.mem[spec].shrink_order == st.mem[p].glue_order =>
                            {
                                t -= Scaled::round(
                                    st.mem[p].glue_set.get() * f64::from(st.mem[spec].shrink),
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
                        u = nb.idx();
                        let span_w = st.mem[proto].width;
                        t += span_w;
                        if st.input.cur_list.mode.name() == ModeName::InternalVertical {
                            st.mem[nb].width = span_w;
                        } else {
                            let nb = &mut st.mem[nb];
                            nb.orientation = Orientation::Vertical.into();
                            nb.height = span_w;
                        }
                    }
                    let col = if st.input.cur_list.mode.name() == ModeName::InternalVertical {
                        let (h, d) = (st.mem[row].height, st.mem[row].depth);
                        let node = &mut st.mem[col];
                        node.height = h;
                        node.depth = d;
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
                    let node = &mut st.mem[b];
                    node.glue_sign = sign.into();
                    node.glue_order = order;
                    node.glue_set = crate::node::GlueRatio::new(set);
                    node.shift_amount = Scaled::ZERO;
                    if u != HOLD_HEAD {
                        let (rest, spanned) = (cell.link(&st.mem), HOLD_HEAD.link(&st.mem));
                        u.set_link(&mut st.mem, rest);
                        cell.set_link(&mut st.mem, spanned);
                        cell = u;
                    }
                    r = cell.link(&st.mem).or_confusion("align")?.link(&st.mem);
                    proto_column = proto.next_column(&st.mem);
                    if r.is_none() {
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
                    st.mem[rule].link = None;
                    let boxed = crate::packaging::hpack(st, rule.ptr(), PackSpec::NATURAL)?;
                    let node = &mut st.mem[boxed];
                    node.shift_amount = o;
                    node.link = next;
                    s.set_link(&mut st.mem, boxed.ptr());
                    cur = boxed.idx();
                }
            }
            _ => {}
        }
        s = cur;
        q = cur.link(&st.mem);
    }

    st.mem.flush_node_list(p.ptr())?;
    pop_alignment(st)?;

    let aux_save = st.input.cur_list.mode;
    let rows = st.input.cur_list.head.link(&st.mem);
    let rows_tail = st.input.cur_list.tail;
    st.mem.pop_nest(&mut st.input)?;

    if st.input.cur_list.mode.name() == ModeName::DisplayMath {
        do_assignments(st)?;
        if st.scanner.cur.cmd() != Cmd::MathShift {
            crate::errors::back_error(
                st,
                "Missing $$ inserted",
                &[
                    r"Displays can use special alignments (like \eqalignno)",
                    "only if nothing but the alignment itself is between $$'s.",
                ],
            )?;
        } else {
            crate::math::check_display_end(st)?;
        }
        st.mem.flush_node_list(st.input.cur_list.e_tex_aux)?;
        st.mem.pop_nest(&mut st.input)?;
        let pre_penalty = st.eqtb.int_par(IntPar::PreDisplayPenalty);
        let pen = st.mem.new_penalty(pre_penalty)?;
        st.input.cur_list.tail_append(&mut st.mem, pen);
        let above = st.mem.new_param_glue(&st.eqtb, GluePar::AboveDisplaySkip)?;
        st.input.cur_list.tail_append(&mut st.mem, above);
        st.input.cur_list.tail.set_link(&mut st.mem, rows);
        if !rows.is_none() {
            st.input.cur_list.tail = rows_tail;
        }
        let post_penalty = st.eqtb.int_par(IntPar::PostDisplayPenalty);
        let pen = st.mem.new_penalty(post_penalty)?;
        st.input.cur_list.tail_append(&mut st.mem, pen);
        let below = st.mem.new_param_glue(&st.eqtb, GluePar::BelowDisplaySkip)?;
        st.input.cur_list.tail_append(&mut st.mem, below);
        let Mode::Vertical { prev_depth, .. } = aux_save else {
            return Err(Abort::Confusion("align aux"));
        };
        st.input.cur_list.set_prev_depth(prev_depth)?;
        crate::math::resume_after_display(st)?;
    } else {
        match (&mut st.input.cur_list.mode, aux_save) {
            (
                Mode::Vertical { prev_depth, .. },
                Mode::Vertical {
                    prev_depth: saved, ..
                },
            ) => *prev_depth = saved,
            (
                Mode::Horizontal {
                    space_factor,
                    clang,
                    ..
                },
                Mode::Horizontal {
                    space_factor: saved_factor,
                    clang: saved_clang,
                    ..
                },
            ) => {
                *space_factor = saved_factor;
                *clang = saved_clang;
            }
            _ => return Err(Abort::Confusion("align aux")),
        }
        st.input.cur_list.tail.set_link(&mut st.mem, rows);
        if !rows.is_none() {
            st.input.cur_list.tail = rows_tail;
        }
        if st.input.cur_list.mode.name() == ModeName::Vertical {
            crate::pagebuilder::build_page(st)?;
        }
    }
    Ok(())
}

impl MemState {
    fn first_column<K: AlignColumn>(&self) -> Option<NodePtr<K>> {
        NodePtr::after_tab_skip(self, ALIGN_HEAD.link(self)?)
    }

    fn for_each_preamble_column(&mut self, mut f: impl FnMut(&mut MemState, NodePtr<UnsetNode>)) {
        let mut column = self.first_column();
        while let Some(q) = column {
            f(self, q);
            column = q.next_column(self);
        }
    }

    /// Folds column `q`'s span widths, less its own width and tabskip, into `next`'s.
    /// A width left spanning one column is `next`'s own width.
    fn merge_spans(
        &mut self,
        q: NodePtr<AlignRecord>,
        next: NodePtr<AlignRecord>,
    ) -> Result<(), Abort> {
        let t = self[q].width + self[self[q.tab_skip(self)?].glue_ptr].width;
        let mut r = self.span_next(SpanAt::Front(q))?;
        // `None` while still before `next` itself, which counts as spanning one column.
        let mut s: Option<SpanAt> = None;
        let mut n = 1;
        loop {
            self[r].width -= t;
            let u = self.span_next(SpanAt::After(r))?;
            while self[r].span_count > n {
                let at = match s {
                    None => SpanAt::Front(next),
                    Some(at) => SpanAt::After(self.span_next(at)?),
                };
                s = Some(at);
                n = self[self.span_next(at)?].span_count + 1;
            }
            if self[r].span_count < n {
                let at = s.or_confusion("span")?;
                self[r].next = Some(self.span_next(at)?);
                self.set_span_next(at, r);
                self[r].span_count -= 1;
                s = Some(SpanAt::After(r));
            } else {
                let w = self[r].width;
                match s {
                    None => self[next].width = self[next].width.max(w),
                    Some(at) => {
                        let merged = self.span_next(at)?;
                        self[merged].width = self[merged].width.max(w);
                    }
                }
                r.free(self);
            }
            r = u;
            if r == END_SPAN {
                break;
            }
        }
        Ok(())
    }

    /// The span node after `at`, which does not end its span list.
    fn span_next(&self, at: SpanAt) -> Result<NodePtr<SpanNode>, Abort> {
        match at {
            SpanAt::Front(column) => self[column].span_list,
            SpanAt::After(s) => self[s].next,
        }
        .or_confusion("span list")
    }

    fn set_span_next(&mut self, at: SpanAt, s: NodePtr<SpanNode>) {
        match at {
            SpanAt::Front(column) => self[column].span_list = Some(s),
            SpanAt::After(p) => self[p].next = Some(s),
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
        match st.scanner.cur.cmd() {
            Cmd::NoAlign => {
                crate::scan::scan_left_brace(st)?;
                st.save_stack
                    .new_save_level(GroupCode::NoAlign, st.mem.line)?;
                if st.input.cur_list.mode.name() == ModeName::InternalVertical {
                    normal_paragraph(st)?;
                }
                return Ok(());
            }
            Cmd::RightBrace => return fin_align(st),
            Cmd::CarRet if st.scanner.cur == Command::CarRet { crcr: true } => {}
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
        if st.scanner.cur.cmd() != Cmd::Spacer {
            return Ok(());
        }
    }
}

fn get_preamble_token(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        crate::scan::get_token(st)?;
        while st.scanner.cur == Command::TabMark(TabMarkCode::SPAN) {
            crate::scan::get_token(st)?;
            if st.scanner.cur.cmd() > Cmd::MAX_COMMAND {
                crate::expand::expand(st)?;
                crate::scan::get_token(st)?;
            }
        }
        if st.scanner.cur.cmd() == Cmd::Endv {
            return Err(Abort::Fatal(
                "(interwoven alignment preambles are not allowed)",
            ));
        }
        if st.scanner.cur != Command::AssignGlue(GluePar::TabSkip.loc()) {
            return Ok(());
        }
        crate::scan::scan_optional_equals(st)?;
        let g = crate::scan::scan_glue(st, ValueLevel::Glue)?;
        let global = st.eqtb.int_par(IntPar::GlobalDefs) > 0;
        crate::eqtb::define(st, global, GluePar::TabSkip.loc(), g)?;
    }
}

fn init_span(st: &mut EngineState, p: MemPtr) -> Result<(), Abort> {
    crate::mem::push_nest(st)?;
    if let Mode::Horizontal { space_factor, .. } = &mut st.input.cur_list.mode {
        *space_factor = 1000;
    } else {
        st.input.cur_list.set_prev_depth(IGNORE_DEPTH)?;
        normal_paragraph(st)?;
    }
    st.scanner.cur_span = p;
    Ok(())
}

fn init_row(st: &mut EngineState) -> Result<(), Abort> {
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = match st.input.cur_list.mode {
        Mode::Vertical { .. } => Mode::Horizontal {
            inner: true,
            space_factor: 0,
            clang: 0,
        },
        Mode::Horizontal { .. } => Mode::Vertical {
            inner: true,
            prev_depth: Scaled::ZERO,
        },
        Mode::Math { .. } | Mode::None => return Err(Abort::Confusion("align row")),
    };
    let tab_skip = NodePtr::<GlueNode>::tab_skip_after(&st.mem, ALIGN_HEAD)?;
    let spec = st.mem[tab_skip].glue_ptr;
    let glue = st.mem.new_glue(spec)?;
    st.input.cur_list.tail_append(&mut st.mem, glue);
    st.mem[glue].subtype = GlueKind::Param(GluePar::TabSkip).into();
    st.scanner.cur_align = st.mem[tab_skip].link;
    st.scanner.cur_tail = st.scanner.cur_head;
    st.scanner.cur_pre_tail = st.scanner.cur_pre_head;
    init_span(st, st.scanner.cur_align)
}

fn init_col(st: &mut EngineState) -> Result<(), Abort> {
    let column = NodePtr::<AlignRecord>::from_ptr(st.scanner.cur_align).or_confusion("align")?;
    st.mem[column].extra_info = i32::from(st.scanner.cur.cmd());
    if st.scanner.cur.cmd() == Cmd::Omit {
        st.scanner.align_state = 0;
    } else {
        crate::input::back_input(st)?;
        crate::input::begin_token_list(st, st.mem[column].u_part, TokenType::UTemplate)?;
    }
    Ok(())
}

/// The state of an enclosing alignment, saved while an inner one runs.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct SavedAlign {
    pub cur_align: MemPtr,
    pub preamble: MemPtr,
    pub cur_span: MemPtr,
    pub cur_loop: MemPtr,
    pub align_state: i32,
    pub cur_head: MemPtr,
    pub cur_tail: MemPtr,
    pub cur_pre_head: MemPtr,
    pub cur_pre_tail: MemPtr,
}

fn push_alignment(st: &mut EngineState) -> Result<(), Abort> {
    let sc = &st.scanner;
    let saved = SavedAlign {
        cur_align: sc.cur_align,
        preamble: ALIGN_HEAD.link(&st.mem),
        cur_span: sc.cur_span,
        cur_loop: sc.cur_loop,
        align_state: sc.align_state,
        cur_head: sc.cur_head,
        cur_tail: sc.cur_tail,
        cur_pre_head: sc.cur_pre_head,
        cur_pre_tail: sc.cur_pre_tail,
    };
    st.scanner.align_stack.push(saved);
    st.scanner.cur_head = NodePtr::<HeadNode>::alloc(&mut st.mem)?.ptr();
    st.scanner.cur_pre_head = NodePtr::<HeadNode>::alloc(&mut st.mem)?.ptr();
    Ok(())
}

fn pop_alignment(st: &mut EngineState) -> Result<(), Abort> {
    let p = st.scanner.align_stack.pop().or_confusion("align")?;
    for head in [st.scanner.cur_head, st.scanner.cur_pre_head] {
        NodePtr::<HeadNode>::from_ptr(head)
            .or_confusion("align")?
            .free(&mut st.mem);
    }
    let sc = &mut st.scanner;
    sc.cur_tail = p.cur_tail;
    sc.cur_head = p.cur_head;
    sc.cur_pre_tail = p.cur_pre_tail;
    sc.cur_pre_head = p.cur_pre_head;
    sc.align_state = p.align_state;
    sc.cur_loop = p.cur_loop;
    sc.cur_span = p.cur_span;
    ALIGN_HEAD.set_link(&mut st.mem, p.preamble);
    st.scanner.cur_align = p.cur_align;
    Ok(())
}

impl MemState {
    /// A copy of token list `r`.
    fn copy_token_list(&mut self, r: TokenList) -> Result<TokenList, Abort> {
        let copy = self.tokens.begin()?;
        let mut c = self.tokens.cursor(r);
        while let Some(t) = self.tokens.read(&mut c) {
            self.tokens.push(&copy, t)?;
        }
        self.tokens.finish(copy)
    }

    /// A fresh preamble column with an empty span list and no entries yet.
    fn new_align_record(&mut self) -> Result<NodePtr<AlignRecord>, Abort> {
        let nb = self.new_null_box()?;
        let column = NodePtr::<AlignRecord>::at(nb.idx());
        self[column].span_list = Some(END_SPAN);
        self[column].width = RUNNING;
        Ok(column)
    }
}

/// A place in a column's span list: its front, or just after a span node.
#[derive(Copy, Clone)]
enum SpanAt {
    Front(NodePtr<AlignRecord>),
    After(NodePtr<SpanNode>),
}

fn fin_col(st: &mut EngineState) -> Result<bool, Abort> {
    let cur_align = NodePtr::<AlignRecord>::from_ptr(st.scanner.cur_align).or_confusion("endv")?;
    let q = st.mem[cur_align].link.or_confusion("endv")?;
    if st.scanner.align_state < 500_000 {
        return Err(Abort::Fatal(
            "(interwoven alignment preambles are not allowed)",
        ));
    }
    let mut p = q.link(&st.mem);
    if p.is_none() && st.mem[cur_align].extra_info < CR_CODE {
        if let Some(cur_loop) = st.scanner.cur_loop {
            let column = st.mem.new_align_record()?;
            q.set_link(&mut st.mem, column.ptr());
            p = column.ptr();
            let template =
                NodePtr::<AlignRecord>::from_ptr(cur_loop.link(&st.mem)).or_confusion("align")?;
            let u_template = st.mem[template].u_part;
            let u_part = st.mem.copy_token_list(u_template)?;
            st.mem[column].u_part = u_part;
            let v_template = st.mem[template].v_part;
            let v_part = st.mem.copy_token_list(v_template)?;
            st.mem[column].v_part = v_part;
            let tab_skip = template.tab_skip(&st.mem)?;
            st.scanner.cur_loop = tab_skip.ptr();
            let spec = st.mem[tab_skip].glue_ptr;
            let g = st.mem.new_glue(spec)?;
            st.mem[column].link = g.ptr();
            st.mem[g].subtype = GlueKind::Param(GluePar::TabSkip).into();
        } else {
            st.mem[cur_align].extra_info = CR_CODE;
            crate::errors::error(
                st,
                (
                    "Extra alignment tab has been changed to ",
                    crate::output::Esc("cr"),
                ),
                &[
                    r"You have given more \span or & marks than there were",
                    r"in the preamble to the \halign or \valign now in progress.",
                    r"So I'll assume that you meant to type \cr instead.",
                ],
            )?;
        }
    }
    if st.mem[cur_align].extra_info != SPAN_CODE {
        crate::save::unsave(st)?;
        st.save_stack
            .new_save_level(GroupCode::Align, st.mem.line)?;
        let head = st.input.cur_list.head.link(&st.mem);
        let (packed, w) = if st.input.cur_list.mode.name() == ModeName::RestrictedHorizontal {
            let mut adjust = AdjustTails {
                post: st.scanner.cur_tail,
                pre: st.scanner.cur_pre_tail,
            };
            let packed = crate::packaging::hpack_with(
                st,
                head,
                PackSpec::NATURAL,
                PackOrigin::Here,
                Some(&mut adjust),
            )?;
            st.scanner.cur_tail = adjust.post;
            st.scanner.cur_pre_tail = adjust.pre;
            (packed, st.mem[packed.r_box].width)
        } else {
            let packed = crate::packaging::vpackage_with(
                st,
                head,
                PackSpec::NATURAL,
                Scaled::ZERO,
                PackOrigin::Here,
            )?;
            (packed, st.mem[packed.r_box].height)
        };
        let u = packed.r_box;
        let mut n: i32 = 0;
        if st.scanner.cur_span != st.scanner.cur_align {
            let mut q = st.scanner.cur_span;
            loop {
                n += 1;
                let glue = q.or_confusion("span")?.link(&st.mem);
                q = glue.or_confusion("span")?.link(&st.mem);
                if q == st.scanner.cur_align {
                    break;
                }
            }
            if n > i32::from(u16::MAX) {
                return Err(Abort::Confusion("too many spans"));
            }
            let column =
                NodePtr::<AlignRecord>::from_ptr(st.scanner.cur_span).or_confusion("span")?;
            let mut q = SpanAt::Front(column);
            while st.mem[st.mem.span_next(q)?].span_count < n {
                q = SpanAt::After(st.mem.span_next(q)?);
            }
            let next = st.mem.span_next(q)?;
            if st.mem[next].span_count > n {
                let s = NodePtr::<SpanNode>::alloc(&mut st.mem)?;
                let node = &mut st.mem[s];
                node.next = Some(next);
                node.span_count = n;
                st.mem.set_span_next(q, s);
                st.mem[s].width = w;
            } else if st.mem[next].width < w {
                st.mem[next].width = w;
            }
        } else if w > st.mem[cur_align].width {
            st.mem[cur_align].width = w;
        }
        let u = u.into_unset(&mut st.mem);
        st.mem[u].span_count = u16::try_from(n).or_confusion("span")?;
        let o = packed.stretch.max_order();
        st.mem[u].glue_order = o.into();
        st.mem[u].glue_stretch = packed.stretch[o];
        let o = packed.shrink.max_order();
        st.mem[u].glue_sign = o.into();
        st.mem[u].glue_shrink = packed.shrink[o];
        st.mem.pop_nest(&mut st.input)?;
        st.input.cur_list.tail_append(&mut st.mem, u);
        let spec = st.mem[cur_align.tab_skip(&st.mem)?].glue_ptr;
        let g = st.mem.new_glue(spec)?;
        st.input.cur_list.tail_append(&mut st.mem, g);
        st.mem[g].subtype = GlueKind::Param(GluePar::TabSkip).into();
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
    let p = if st.input.cur_list.mode.name() == ModeName::RestrictedHorizontal {
        let row = crate::packaging::hpack(st, head, PackSpec::NATURAL)?;
        st.mem.pop_nest(&mut st.input)?;
        if st.scanner.cur_pre_head != st.scanner.cur_pre_tail {
            append_list(st, st.scanner.cur_pre_head, st.scanner.cur_pre_tail)?;
        }
        st.input
            .cur_list
            .append_to_vlist(&mut st.mem, &st.eqtb, row)?;
        if st.scanner.cur_head != st.scanner.cur_tail {
            append_list(st, st.scanner.cur_head, st.scanner.cur_tail)?;
        }
        row
    } else {
        let row = crate::packaging::vpackage(st, head, PackSpec::NATURAL, MAX_DIMEN)?;
        st.mem.pop_nest(&mut st.input)?;
        st.input.cur_list.tail_append(&mut st.mem, row);
        st.input.cur_list.set_space_factor(1000)?;
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

    if let Mode::Math {
        inner: false,
        incompleat_noad,
    } = st.input.cur_list.mode
        && (st.input.cur_list.tail != st.input.cur_list.head || incompleat_noad.is_some())
    {
        crate::errors::error(
            st,
            ("Improper ", crate::output::Esc("halign"), " inside $$'s"),
            &[
                r"Displays can use special alignments (like \eqalignno)",
                "only if nothing but the alignment itself is between $$'s.",
                "So I've deleted the formulas that preceded this alignment.",
            ],
        )?;
        st.input.cur_list.flush_math(&mut st.mem)?;
    }

    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = if st.input.cur_list.mode.name() == ModeName::DisplayMath {
        Mode::Vertical {
            inner: true,
            prev_depth: st.input.nest[st.input.nest.len() - 2].prev_depth()?,
        }
    } else {
        st.input.cur_list.mode.inner()
    };

    crate::scan::scan_spec(st, GroupCode::Align, None)?;
    ALIGN_HEAD.set_link(&mut st.mem, None);
    st.scanner.cur_align = Some(ALIGN_HEAD);
    st.scanner.cur_loop = None;
    st.scanner.scanner_status = ScannerStatus::Aligning;
    st.scanner.warning_index = save_cs_ptr;
    st.scanner.align_state = -1_000_000;

    loop {
        let g = st.mem.new_param_glue(&st.eqtb, GluePar::TabSkip)?.ptr();
        let last = st.scanner.cur_align.or_confusion("align")?;
        last.set_link(&mut st.mem, g);
        st.scanner.cur_align = g;
        if st.scanner.cur.cmd() == Cmd::CarRet {
            break;
        }

        let u_part = st.mem.tokens.begin()?;
        loop {
            get_preamble_token(st)?;
            if st.scanner.cur.cmd() == Cmd::MacParam {
                break;
            }
            let empty = st.mem.tokens.last_pushed(&u_part).is_none();
            if (Cmd::TabMark..=Cmd::CarRet).contains(&st.scanner.cur.cmd())
                && st.scanner.align_state == -1_000_000
            {
                if empty && st.scanner.cur_loop.is_none() && st.scanner.cur.cmd() == Cmd::TabMark {
                    st.scanner.cur_loop = st.scanner.cur_align;
                } else {
                    crate::errors::back_error(
                        st,
                        "Missing # inserted in alignment preamble",
                        &[
                            "There should be exactly one # between &'s, when an",
                            r"\halign or \valign is being set up. In this case you had",
                            "none, so I've put one in; maybe that will work.",
                        ],
                    )?;
                    break;
                }
            } else if st.scanner.cur.cmd() != Cmd::Spacer || !empty {
                st.mem.tokens.push(&u_part, st.scanner.cur_tok)?;
            }
        }

        let column = st.mem.new_align_record()?;
        let last = st.scanner.cur_align.or_confusion("align")?;
        last.set_link(&mut st.mem, column.ptr());
        st.scanner.cur_align = column.ptr();
        st.mem[column].u_part = st.mem.tokens.finish(u_part)?;

        let v_part = st.mem.tokens.begin()?;
        loop {
            get_preamble_token(st)?;
            if (Cmd::TabMark..=Cmd::CarRet).contains(&st.scanner.cur.cmd())
                && st.scanner.align_state == -1_000_000
            {
                break;
            }
            if st.scanner.cur.cmd() == Cmd::MacParam {
                crate::errors::error(
                    st,
                    "Only one # is allowed per tab",
                    &[
                        "There should be exactly one # between &'s, when an",
                        r"\halign or \valign is being set up. In this case you had",
                        "more than one, so I'm ignoring all but the first.",
                    ],
                )?;
                continue;
            }
            st.mem.tokens.push(&v_part, st.scanner.cur_tok)?;
        }
        st.mem
            .tokens
            .push(&v_part, Token::from_cs(Frozen::EndTemplate.cs()))?;
        st.mem[column].v_part = st.mem.tokens.finish(v_part)?;
    }

    st.scanner.scanner_status = ScannerStatus::Normal;
    st.save_stack
        .new_save_level(GroupCode::Align, st.mem.line)?;
    begin_every(st, TokPar::EveryCr, TokenType::EveryCr)?;
    align_peek(st)
}

pub(super) fn do_endv(st: &mut EngineState) -> Result<(), Abort> {
    fn is_v_template_end(r: &crate::state::InputState) -> bool {
        r.state == TOKEN_LIST && r.token_loc().is_end() && r.token_type() == TokenType::VTemplate
    }

    st.input.base_ptr = st.input.input_stack.len();
    st.input.input_stack.park(st.input.cur_input);
    while st.input.input_stack[st.input.base_ptr].state == TOKEN_LIST
        && st.input.input_stack[st.input.base_ptr].token_loc().is_end()
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
