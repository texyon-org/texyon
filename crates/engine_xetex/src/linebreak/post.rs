//! Breaking a paragraph into lines, continued: `post_line_break`.

use crate::consts::{
    ETEX_PENALTIES_PAR_CLUB_PENALTIES, ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES,
    ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES, ETEX_PENALTIES_PAR_WIDOW_PENALTIES, ETEX_PEN_BASE,
    EXPLICIT, SPACE_ADJUSTMENT,
};
use crate::errors::Abort;
use crate::mem::{ADJUST_HEAD, PRE_ADJUST_HEAD, TEMP_HEAD, ZERO_GLUE};
use crate::mem_types::ListStateRecord;
use crate::node::{ActiveNode, AnyNode, BoxNode, MathNode, NodePtr};
use crate::packaging::PackSpec;
use crate::params::{GluePar, IntPar};
use crate::state::{
    ConfigState, EngineState, EqtbState, ErrorState, FontState, LinebreakState, MemState,
    ScannerState,
};
use crate::units::{MemPtr, Scaled};

use super::passive_node;

struct PostLineHpack {
    q: MemPtr,
    cur_width: Scaled,
    cur_indent: Scaled,
    semantic_pagination: bool,
    disc_break: bool,
    post_disc_break: bool,
}

impl LinebreakState {
    /// Break off the next line of the paragraph after `TEMP_HEAD`, ready for `hpack`.
    #[allow(clippy::too_many_arguments)]
    fn post_line_break_prefix(
        &mut self,
        errors: &mut ErrorState,
        mem: &mut MemState,
        scanner: &mut ScannerState,
        eqtb: &EqtbState,
        fonts: &FontState,
        config: &ConfigState,
        lr_ptr: &mut MemPtr,
        cur_line: i32,
    ) -> Result<PostLineHpack, Abort> {
        let texxet = eqtb.int_par(IntPar::Texxet) > 0;
        let protrude = eqtb.int_par(IntPar::XetexProtrudeChars) > 0;
        let cur_break = mem[passive_node(self.cur_p)].cur_break;

        if texxet {
            let mut q = TEMP_HEAD.link(mem);
            if !lr_ptr.is_null() {
                let mut temp_ptr = *lr_ptr;
                let mut r = q;
                loop {
                    let s = mem.new_math(Scaled::ZERO, (temp_ptr.info(mem) - 1) as u16)?;
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
                    mem[g].glue_ptr = right_skip;
                    mem[g].subtype = GluePar::RightSkip.subtype();
                    mem.add_glue_ref(right_skip);
                    glue_break = true;
                }
                Ok(AnyNode::Disc(d)) => {
                    let mut r;
                    let t = mem[d].subtype;
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
                        mem[d].subtype = 0;
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
            q = self.insert_right_margin_kern(errors, mem, fonts, q, pre_break_tail)?;
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
                let m = mem.new_math(Scaled::ZERO, r.info(mem) as u16)?;
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
            q = self.insert_left_margin_kern(errors, mem, fonts, q)?;
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

        scanner.adjust_tail = ADJUST_HEAD;
        scanner.pre_adjust_tail = PRE_ADJUST_HEAD;

        Ok(PostLineHpack {
            q,
            cur_width,
            cur_indent,
            semantic_pagination: config.semantic_pagination_enabled,
            disc_break,
            post_disc_break,
        })
    }
}

impl MemState {
    fn reverse_best_breaks(&mut self, linebreak: &mut LinebreakState, best: NodePtr<ActiveNode>) {
        let mut q = self[best].break_node;
        linebreak.cur_p = MemPtr::NULL;
        loop {
            let r = passive_node(q);
            q = self[r].next_break;
            self[r].next_break = linebreak.cur_p;
            linebreak.cur_p = r.ptr();
            if q.is_null() {
                break;
            }
        }
    }

    /// Track the LR nesting of a math node on the `lr_ptr` stack.
    fn update_lr_stack(&mut self, q: NodePtr<MathNode>, lr_ptr: &mut MemPtr) -> Result<(), Abort> {
        let subtype = self[q].subtype;
        let end_lr = crate::mem::end_lr_type(subtype);
        if subtype % 2 == 1 {
            if !lr_ptr.is_null() && lr_ptr.info(self) == end_lr {
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
        let count = penalties.count(self);
        let n = if n > 0 { n.min(count) } else { count };
        Some(penalties.penalty(self, n))
    }
}

impl ListStateRecord {
    /// Append the packed line and the interline penalty, then prune discardable
    /// nodes from the start of the next line. Returns whether the paragraph is done.
    #[allow(clippy::too_many_arguments)]
    fn post_line_break_suffix(
        &mut self,
        mem: &mut MemState,
        scanner: &mut ScannerState,
        linebreak: &mut LinebreakState,
        eqtb: &EqtbState,
        just_box: NodePtr<BoxNode>,
        hpack: PostLineHpack,
        d: bool,
        lr_ptr: &mut MemPtr,
        cur_line: &mut i32,
    ) -> Result<bool, Abort> {
        mem[just_box].shift_amount = hpack.cur_indent;

        if scanner.pre_adjust_tail != PRE_ADJUST_HEAD {
            self.tail.set_link(mem, PRE_ADJUST_HEAD.link(mem));
            self.tail = scanner.pre_adjust_tail;
        }
        scanner.pre_adjust_tail = MemPtr::NULL;

        self.append_to_vlist(mem, eqtb, just_box)?;

        if scanner.adjust_tail != ADJUST_HEAD {
            self.tail.set_link(mem, ADJUST_HEAD.link(mem));
            self.tail = scanner.adjust_tail;
        }
        scanner.adjust_tail = MemPtr::NULL;

        if cur_line.wrapping_add(1) != linebreak.best_line {
            let mut pen = mem
                .etex_penalty(eqtb, ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES, *cur_line)
                .unwrap_or_else(|| eqtb.int_par(IntPar::InterLinePenalty));

            match mem.etex_penalty(
                eqtb,
                ETEX_PENALTIES_PAR_CLUB_PENALTIES,
                cur_line.wrapping_sub(self.prev_graf),
            ) {
                Some(p) => pen = pen.wrapping_add(p),
                None if *cur_line == self.prev_graf.wrapping_add(1) => {
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
                linebreak.best_line.wrapping_sub(*cur_line).wrapping_sub(1),
            ) {
                Some(p) => pen = pen.wrapping_add(p),
                None if cur_line.wrapping_add(2) == linebreak.best_line => {
                    pen = pen.wrapping_add(eqtb.int_par(widow_penalty))
                }
                None => {}
            }

            if hpack.disc_break {
                pen = pen.wrapping_add(eqtb.int_par(IntPar::BrokenPenalty));
            }

            if pen != 0 {
                let r = mem.new_penalty(pen)?;
                self.tail_append(mem, r.ptr());
            }
        }

        *cur_line = cur_line.wrapping_add(1);
        linebreak.cur_p = mem[passive_node(linebreak.cur_p)].next_break;

        if !linebreak.cur_p.is_null() && !hpack.post_disc_break {
            let texxet = eqtb.int_par(IntPar::Texxet) > 0;
            let cur_break = mem[passive_node(linebreak.cur_p)].cur_break;
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
                        if mem[k].subtype != EXPLICIT && mem[k].subtype != SPACE_ADJUSTMENT =>
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

        Ok(linebreak.cur_p.is_null())
    }
}

/// Break the paragraph into lines along the breakpoints ending at `best` and
/// append them, returning the last line's box.
pub(super) fn post_line_break(
    st: &mut EngineState,
    best: NodePtr<ActiveNode>,
    d: bool,
) -> Result<NodePtr<BoxNode>, Abort> {
    let mut lr_ptr = st.input.cur_list.e_tex_aux;

    st.mem.reverse_best_breaks(&mut st.linebreak, best);

    let mut cur_line = st.input.cur_list.prev_graf.wrapping_add(1);
    let last_line = loop {
        let hpack = st.linebreak.post_line_break_prefix(
            &mut st.errors,
            &mut st.mem,
            &mut st.scanner,
            &st.eqtb,
            &st.fonts,
            &st.config,
            &mut lr_ptr,
            cur_line,
        )?;

        let spec = if hpack.semantic_pagination {
            PackSpec::NATURAL
        } else {
            PackSpec::Exactly(hpack.cur_width)
        };
        let just_box = crate::packaging::hpack(st, hpack.q, spec)?;

        match st.input.cur_list.post_line_break_suffix(
            &mut st.mem,
            &mut st.scanner,
            &mut st.linebreak,
            &st.eqtb,
            just_box,
            hpack,
            d,
            &mut lr_ptr,
            &mut cur_line,
        ) {
            Ok(true) => break just_box,
            Ok(false) => {}
            Err(e) => return Err(e),
        }
    };

    if cur_line != st.linebreak.best_line || !TEMP_HEAD.link(&st.mem).is_null() {
        return Err(Abort::Confusion("line breaking"));
    }

    st.input.cur_list.prev_graf = st.linebreak.best_line.wrapping_sub(1);
    st.input.cur_list.e_tex_aux = lr_ptr;
    Ok(last_line)
}
