//! Breaking a paragraph into lines, continued: `post_line_break`.

use crate::consts::{
    ETEX_PENALTIES_PAR_CLUB_PENALTIES, ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES,
    ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES, ETEX_PENALTIES_PAR_WIDOW_PENALTIES, ETEX_PEN_BASE,
};
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::mem::{ADJUST_HEAD, PRE_ADJUST_HEAD, TEMP_HEAD, ZERO_GLUE};
use crate::node::{AnyNode, BoxNode, GlueKind, KernSubtype, MathNode, NodePtr};
use crate::packaging::{AdjustTails, PackOrigin, PackSpec};
use crate::params::{GluePar, IntPar};
use crate::state::{EngineState, EqtbState, MemState};
use crate::units::{MemPtr, Scaled};

use super::lists::{ActiveEntry, PassiveId};
use super::Breaker;

/// Where `post_line_break` is in the paragraph.
struct PostLines {
    cur_line: i32,
    /// One more than the paragraph's last line number.
    best_line: i32,
    /// The LR stack carried from line to line.
    lr_ptr: MemPtr,
    /// Whether a display follows the paragraph.
    display: bool,
    /// Where the paragraph began, for over/underfull line messages.
    origin: PackOrigin,
}

struct PostLineHpack {
    /// The break that ends this line.
    brk: PassiveId,
    q: MemPtr,
    cur_width: Scaled,
    cur_indent: Scaled,
    semantic_pagination: bool,
    disc_break: bool,
    post_disc_break: bool,
}

impl Breaker {
    /// Break off the next line of the paragraph after `TEMP_HEAD`, ready for `hpack`.
    fn post_line_break_prefix(
        &mut self,
        st: &mut EngineState,
        lines: &mut PostLines,
        brk: PassiveId,
    ) -> Result<PostLineHpack, Abort> {
        let cur_line = lines.cur_line;
        let lr_ptr = &mut lines.lr_ptr;
        let EngineState {
            mem,
            eqtb,
            fonts,
            config,
            ..
        } = st;
        let texxet = eqtb.int_par(IntPar::Texxet) > 0;
        let protrude = eqtb.int_par(IntPar::XetexProtrudeChars) > 0;
        let cur_break = self.lists.passive(brk).cur_break;

        if texxet {
            let mut q = TEMP_HEAD.link(mem);
            if !lr_ptr.is_null() {
                let mut temp_ptr = *lr_ptr;
                let mut r = q;
                loop {
                    let code = mem.lr_code(temp_ptr).or_confusion("LR")?;
                    let s = mem.new_math(Scaled::ZERO, code.as_begin())?;
                    mem[s].link = r;
                    r = s.ptr();
                    temp_ptr = temp_ptr.link(mem);
                    if temp_ptr.is_null() {
                        break;
                    }
                }
                TEMP_HEAD.set_link(mem, r);
            }

            while q != cur_break {
                if !mem.is_char_node(q) {
                    if let Ok(AnyNode::Math(m)) = mem.classify(q) {
                        mem.update_lr_stack(m, lr_ptr)?;
                    }
                }
                q = q.link(mem);
            }
        }

        let mut q = cur_break;
        let mut disc_break = false;
        let mut post_disc_break = false;
        let mut glue_break = false;
        let mut pre_break_tail = false;

        if q.is_null() {
            q = mem.list_tail(TEMP_HEAD);
        } else {
            match mem.classify(q) {
                Ok(AnyNode::Glue(g)) => {
                    let old_spec = mem[g].glue_ptr;
                    mem.delete_glue_ref(old_spec);
                    let right_skip = eqtb.glue_par(GluePar::RightSkip);
                    let node = &mut mem[g];
                    node.glue_ptr = right_skip;
                    node.subtype = GlueKind::Param(GluePar::RightSkip).into();
                    mem.add_glue_ref(right_skip);
                    glue_break = true;
                }
                Ok(AnyNode::Disc(d)) => {
                    let mut r;
                    let t = mem[d].replace_count;
                    if t == 0 {
                        r = mem[d].link;
                    } else {
                        r = q;
                        for _ in 1..t {
                            r = r.link(mem);
                        }
                        let s = r.link(mem);
                        r = s.link(mem);
                        s.set_link(mem, MemPtr::NULL);
                        mem.flush_node_list(mem[d].link)?;
                        mem[d].replace_count = 0;
                    }

                    let post_break = mem[d].post_break;
                    if !post_break.is_null() {
                        mem.list_tail(post_break).set_link(mem, r);
                        r = post_break;
                        mem[d].post_break = MemPtr::NULL;
                        post_disc_break = true;
                    }

                    let pre_break = mem[d].pre_break;
                    if !pre_break.is_null() {
                        mem[d].link = pre_break;
                        q = mem.list_tail(pre_break);
                        mem[d].pre_break = MemPtr::NULL;
                        pre_break_tail = true;
                    }

                    q.set_link(mem, r);
                    disc_break = true;
                }
                Ok(AnyNode::Kern(k)) => mem[k].width = Scaled::ZERO,
                Ok(AnyNode::Math(m)) => {
                    mem[m].width = Scaled::ZERO;
                    if texxet {
                        mem.update_lr_stack(m, lr_ptr)?;
                    }
                }
                _ => {}
            }
        }

        if protrude {
            q = self.insert_right_margin_kern(mem, fonts, q, pre_break_tail)?;
        }

        if !glue_break {
            let r = mem.new_param_glue(eqtb, GluePar::RightSkip)?;
            mem[r].link = q.link(mem);
            q.set_link(mem, r.ptr());
            q = r.ptr();
        }

        if texxet && !lr_ptr.is_null() {
            let mut s = TEMP_HEAD;
            let mut r = s.link(mem);
            while r != q {
                s = r;
                r = s.link(mem);
            }

            r = *lr_ptr;
            while !r.is_null() {
                let code = mem.lr_code(r).or_confusion("LR")?;
                let m = mem.new_math(Scaled::ZERO, code)?;
                s.set_link(mem, m.ptr());
                s = m.ptr();
                r = r.link(mem);
            }
            s.set_link(mem, q);
        }

        let r = q.link(mem);
        q.set_link(mem, MemPtr::NULL);
        q = TEMP_HEAD.link(mem);
        TEMP_HEAD.set_link(mem, r);

        if protrude {
            q = self.insert_left_margin_kern(mem, fonts, q)?;
        }

        if eqtb.glue_par(GluePar::LeftSkip) != ZERO_GLUE {
            let r = mem.new_param_glue(eqtb, GluePar::LeftSkip)?;
            mem[r].link = q;
            q = r.ptr();
        }

        let (cur_width, cur_indent) = if cur_line > self.last_special_line {
            (self.second_width, self.second_indent)
        } else if let Some(shape) = eqtb.par_shape() {
            // A huge \prevgraf wraps line numbers negative, past the shape.
            if cur_line > 0 {
                let line = shape.line(mem, cur_line);
                (line.width, line.indent)
            } else {
                (self.second_width, self.second_indent)
            }
        } else {
            (self.first_width, self.first_indent)
        };

        Ok(PostLineHpack {
            brk,
            q,
            cur_width,
            cur_indent,
            semantic_pagination: config.semantic_pagination_enabled,
            disc_break,
            post_disc_break,
        })
    }
}

impl Breaker {
    /// Reverse the chain of breaks ending at `last`, returning the first line's break.
    fn reverse_best_breaks(&mut self, last: Option<PassiveId>) -> Option<PassiveId> {
        let mut q = last;
        let mut first = None;
        while let Some(r) = q {
            q = self.lists.passive(r).next_break;
            self.lists.passive_mut(r).next_break = first;
            first = Some(r);
        }
        first
    }
}

impl MemState {
    /// Track the LR nesting of a math node on the `lr_ptr` stack.
    fn update_lr_stack(&mut self, q: NodePtr<MathNode>, lr_ptr: &mut MemPtr) -> Result<(), Abort> {
        let code = self[q].code()?;
        let end_lr = code.lr_end();
        if code.is_end() {
            if !lr_ptr.is_null() && self.lr_code(*lr_ptr) == Some(end_lr) {
                self.pop_lr(lr_ptr);
            }
            Ok(())
        } else {
            self.push_lr(lr_ptr, end_lr)
        }
    }

    /// Entry `n` (clamped to the last) of the eTeX penalty array `k`, if it is set.
    /// A line number wrapped negative by a huge `\prevgraf` also gets the last.
    fn etex_penalty(&self, eqtb: &EqtbState, k: i32, n: i32) -> Option<i32> {
        let penalties = eqtb.penalties(ETEX_PEN_BASE + k)?;
        let count = self[penalties].count;
        let n = if n > 0 { n.min(count) } else { count };
        Some(penalties.penalty(self, n))
    }
}

impl Breaker {
    /// Append the packed line and the interline penalty, then prune discardable
    /// nodes from the start of the next line. Returns the next line's break, if any.
    fn post_line_break_suffix(
        &self,
        st: &mut EngineState,
        just_box: NodePtr<BoxNode>,
        lines: &mut PostLines,
        hpack: PostLineHpack,
        adjust: AdjustTails,
    ) -> Result<Option<PassiveId>, Abort> {
        let best_line = lines.best_line;
        let d = lines.display;
        let lr_ptr = &mut lines.lr_ptr;
        let cur_line = &mut lines.cur_line;
        let EngineState {
            mem, eqtb, input, ..
        } = st;
        let list = &mut input.cur_list;
        mem[just_box].shift_amount = hpack.cur_indent;

        if adjust.pre != PRE_ADJUST_HEAD {
            list.tail.set_link(mem, PRE_ADJUST_HEAD.link(mem));
            list.tail = adjust.pre;
        }

        list.append_to_vlist(mem, eqtb, just_box)?;

        if adjust.post != ADJUST_HEAD {
            list.tail.set_link(mem, ADJUST_HEAD.link(mem));
            list.tail = adjust.post;
        }

        if cur_line.wrapping_add(1) != best_line {
            let mut pen = mem
                .etex_penalty(eqtb, ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES, *cur_line)
                .unwrap_or_else(|| eqtb.int_par(IntPar::InterLinePenalty));

            match mem.etex_penalty(
                eqtb,
                ETEX_PENALTIES_PAR_CLUB_PENALTIES,
                cur_line.wrapping_sub(list.prev_graf),
            ) {
                Some(p) => pen = pen.wrapping_add(p),
                None if *cur_line == list.prev_graf.wrapping_add(1) => {
                    pen = pen.wrapping_add(eqtb.int_par(IntPar::ClubPenalty))
                }
                None => {}
            }

            let (widow_array, widow_penalty) = if d {
                (
                    ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES,
                    IntPar::DisplayWidowPenalty,
                )
            } else {
                (ETEX_PENALTIES_PAR_WIDOW_PENALTIES, IntPar::WidowPenalty)
            };
            match mem.etex_penalty(
                eqtb,
                widow_array,
                best_line.wrapping_sub(*cur_line).wrapping_sub(1),
            ) {
                Some(p) => pen = pen.wrapping_add(p),
                None if cur_line.wrapping_add(2) == best_line => {
                    pen = pen.wrapping_add(eqtb.int_par(widow_penalty))
                }
                None => {}
            }

            if hpack.disc_break {
                pen = pen.wrapping_add(eqtb.int_par(IntPar::BrokenPenalty));
            }

            if pen != 0 {
                let r = mem.new_penalty(pen)?;
                list.tail_append(mem, r.ptr());
            }
        }

        *cur_line = cur_line.wrapping_add(1);
        let next = self.lists.passive(hpack.brk).next_break;

        if let Some(next) = next.filter(|_| !hpack.post_disc_break) {
            let texxet = eqtb.int_par(IntPar::Texxet) > 0;
            let cur_break = self.lists.passive(next).cur_break;
            let mut r = TEMP_HEAD;
            let mut q;
            loop {
                q = r.link(mem);
                if q == cur_break || mem.is_char_node(q) {
                    break;
                }
                let node = mem.classify(q);
                match node {
                    Ok(node) if node.precedes_break() => break,
                    Ok(AnyNode::Kern(k))
                        if mem[k].subtype != KernSubtype::Explicit
                            && mem[k].subtype != KernSubtype::SpaceAdjustment =>
                    {
                        break
                    }
                    _ => {}
                }

                r = q;
                if texxet {
                    if let Ok(AnyNode::Math(m)) = node {
                        mem.update_lr_stack(m, lr_ptr)?;
                    }
                }
            }

            if r != TEMP_HEAD {
                r.set_link(mem, MemPtr::NULL);
                mem.flush_node_list(TEMP_HEAD.link(mem))?;
                TEMP_HEAD.set_link(mem, q);
            }
        }

        Ok(next)
    }
}

/// Break the paragraph into lines along the breakpoints ending at `best` and
/// append them, returning the last line's box. `display` says a display follows.
pub(super) fn post_line_break(
    st: &mut EngineState,
    lb: &mut Breaker,
    best: &ActiveEntry,
    display: bool,
    origin: PackOrigin,
) -> Result<NodePtr<BoxNode>, Abort> {
    let mut brk = lb
        .reverse_best_breaks(best.break_node)
        .or_confusion("line breaking")?;

    let mut lines = PostLines {
        cur_line: st.input.cur_list.prev_graf.wrapping_add(1),
        best_line: best.line_number,
        lr_ptr: st.input.cur_list.e_tex_aux,
        display,
        origin,
    };
    let last_line = loop {
        let hpack = lb.post_line_break_prefix(st, &mut lines, brk)?;

        let spec = if hpack.semantic_pagination {
            PackSpec::NATURAL
        } else {
            PackSpec::Exactly(hpack.cur_width)
        };
        let mut adjust = AdjustTails::HEADS;
        let just_box =
            crate::packaging::hpack_with(st, hpack.q, spec, lines.origin, Some(&mut adjust))?.r_box;

        match lb.post_line_break_suffix(st, just_box, &mut lines, hpack, adjust)? {
            None => break just_box,
            Some(next) => brk = next,
        }
    };

    if lines.cur_line != lines.best_line || !TEMP_HEAD.link(&st.mem).is_null() {
        return Err(Abort::Confusion("line breaking"));
    }

    st.input.cur_list.prev_graf = lines.best_line.wrapping_sub(1);
    st.input.cur_list.e_tex_aux = lines.lr_ptr;
    Ok(last_line)
}
