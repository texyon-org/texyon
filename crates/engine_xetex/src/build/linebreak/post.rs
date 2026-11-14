//! Breaking a paragraph into lines, continued: `post_line_break`.

use crate::base::params::{GluePar, IntPar};
use crate::base::units::{MemPtr, Scaled};
use crate::build::packaging::{AdjustTails, PackOrigin, PackSpec};
use crate::memory::mem::{ADJUST_HEAD, PRE_ADJUST_HEAD, TEMP_HEAD, ZERO_GLUE};
use crate::memory::node::{AnyNode, BoxNode, GlueKind, KernSubtype, MathNode, NodePtr};
use crate::print::errors::Abort;
use crate::print::errors::OrConfusion;
use crate::state::{EngineState, EqtbState, MemState};
use crate::tables::eqtb::ShapeLoc;

use super::Breaker;
use super::lists::{ActiveEntry, PassiveId};

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
            if !lr_ptr.is_none() {
                let mut temp_ptr = *lr_ptr;
                let mut r = q;
                loop {
                    let code = mem.lr_code(temp_ptr).or_confusion("LR")?;
                    let s = mem.new_math(Scaled::ZERO, code.as_begin())?;
                    mem[s].link = r;
                    r = s.ptr();
                    temp_ptr = temp_ptr.and_then(|t| t.link(mem));
                    if temp_ptr.is_none() {
                        break;
                    }
                }
                TEMP_HEAD.set_link(mem, r);
            }

            while q != cur_break {
                let here = q.or_confusion("post_line_break")?;
                if let Some(AnyNode::Math(m)) = mem.classify_non_char(q) {
                    mem.update_lr_stack(m, lr_ptr)?;
                }
                q = here.link(mem);
            }
        }

        let mut disc_break = false;
        let mut post_disc_break = false;
        let mut glue_break = false;
        let mut pre_break_tail = false;

        let mut q = match cur_break {
            None => mem.list_tail(TEMP_HEAD),
            Some(cb) => cb,
        };
        if let Some(cb) = cur_break {
            match mem.classify(cb) {
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
                        let mut last = cb;
                        for _ in 1..t {
                            last = last.link(mem).or_confusion("disc")?;
                        }
                        let s = last.link(mem).or_confusion("disc")?;
                        r = s.link(mem);
                        s.set_link(mem, None);
                        mem.flush_node_list(mem[d].link)?;
                        mem[d].replace_count = 0;
                    }

                    if let Some(post_break) = mem[d].post_break {
                        mem.list_tail(post_break).set_link(mem, r);
                        r = Some(post_break);
                        mem[d].post_break = None;
                        post_disc_break = true;
                    }

                    if let Some(pre_break) = mem[d].pre_break {
                        mem[d].link = Some(pre_break);
                        q = mem.list_tail(pre_break);
                        mem[d].pre_break = None;
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
            q = r.idx();
        }

        if texxet && !lr_ptr.is_none() {
            let mut s = mem.predecessor(TEMP_HEAD, Some(q))?;
            let mut r = *lr_ptr;
            while let Some(entry) = r {
                let code = mem.lr_code(r).or_confusion("LR")?;
                let m = mem.new_math(Scaled::ZERO, code)?;
                s.set_link(mem, m.ptr());
                s = m.idx();
                r = entry.link(mem);
            }
            s.set_link(mem, Some(q));
        }

        let r = q.link(mem);
        q.set_link(mem, None);
        let mut q = TEMP_HEAD.link(mem);
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
            if !lr_ptr.is_none() && self.lr_code(*lr_ptr) == Some(end_lr) {
                self.pop_lr(lr_ptr);
            }
            Ok(())
        } else {
            self.push_lr(lr_ptr, end_lr)
        }
    }

    /// Entry `n` (clamped to the last) of the eTeX penalty array `k`, if it is set.
    /// A line number wrapped negative by a huge `\prevgraf` also gets the last.
    fn etex_penalty(&self, eqtb: &EqtbState, k: ShapeLoc, n: i32) -> Option<i32> {
        let penalties = eqtb.penalties(k)?;
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

        if adjust.pre != Some(PRE_ADJUST_HEAD) {
            list.tail.set_link(mem, PRE_ADJUST_HEAD.link(mem));
            list.tail = adjust.pre.or_confusion("pre_adjust_tail")?;
        }

        list.append_to_vlist(mem, eqtb, just_box)?;

        if adjust.post != Some(ADJUST_HEAD) {
            list.tail.set_link(mem, ADJUST_HEAD.link(mem));
            list.tail = adjust.post.or_confusion("adjust_tail")?;
        }

        if cur_line.wrapping_add(1) != best_line {
            let mut pen = mem
                .etex_penalty(eqtb, ShapeLoc::InterLinePenalties, *cur_line)
                .unwrap_or_else(|| eqtb.int_par(IntPar::InterLinePenalty));

            match mem.etex_penalty(
                eqtb,
                ShapeLoc::ClubPenalties,
                cur_line.wrapping_sub(list.prev_graf),
            ) {
                Some(p) => pen = pen.wrapping_add(p),
                None if *cur_line == list.prev_graf.wrapping_add(1) => {
                    pen = pen.wrapping_add(eqtb.int_par(IntPar::ClubPenalty))
                }
                None => {}
            }

            let (widow_array, widow_penalty) = if d {
                (ShapeLoc::DisplayWidowPenalties, IntPar::DisplayWidowPenalty)
            } else {
                (ShapeLoc::WidowPenalties, IntPar::WidowPenalty)
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
                list.tail_append(mem, r);
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
                if q == cur_break {
                    break;
                }
                let here = q.or_confusion("post_line_break")?;
                if mem.is_char_node(here) {
                    break;
                }
                let node = mem.classify(here);
                match node {
                    Ok(node) if node.precedes_break() => break,
                    Ok(AnyNode::Kern(k))
                        if mem[k].subtype != KernSubtype::Explicit
                            && mem[k].subtype != KernSubtype::SpaceAdjustment =>
                    {
                        break;
                    }
                    _ => {}
                }

                r = here;
                if texxet && let Ok(AnyNode::Math(m)) = node {
                    mem.update_lr_stack(m, lr_ptr)?;
                }
            }

            if r != TEMP_HEAD {
                r.set_link(mem, None);
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
        let just_box = crate::build::packaging::hpack_with(
            st,
            hpack.q,
            spec,
            lines.origin,
            Some(&mut adjust),
        )?
        .r_box;

        match lb.post_line_break_suffix(st, just_box, &mut lines, hpack, adjust)? {
            None => break just_box,
            Some(next) => brk = next,
        }
    };

    if lines.cur_line != lines.best_line || !TEMP_HEAD.link(&st.mem).is_none() {
        return Err(Abort::Confusion("line breaking"));
    }

    st.input.cur_list.prev_graf = lines.best_line.wrapping_sub(1);
    st.input.cur_list.e_tex_aux = lines.lr_ptr;
    Ok(last_line)
}
