//! Character protrusion: margin kerns and the protruding chars at line ends.

use crate::consts::NORMAL;
use crate::errors::Abort;
use crate::fonts::{Side, QUAD_CODE};
use crate::mem::{TEMP_HEAD, ZERO_GLUE};
use crate::node::{ActiveNode, AnyNode, BoxNode, DiscNode, NodePtr, WhatsitSubtype};
use crate::state::{ErrorState, FontState, LinebreakState, MemState};
use crate::units::{FontId, MemPtr, Scaled};

use super::passive_node;

const MAX_HLIST_STACK: i16 = 512;

impl LinebreakState {
    fn push_node(&mut self, p: MemPtr) -> bool {
        let level = self.hlist_stack_level;
        if level > MAX_HLIST_STACK {
            return false;
        }

        self.hlist_stack[level as usize] = p;
        self.hlist_stack_level = level + 1;
        true
    }

    fn pop_node(&mut self) -> Option<MemPtr> {
        let level = self.hlist_stack_level - 1;
        self.hlist_stack_level = level;
        if level < 0 {
            return None;
        }

        Some(self.hlist_stack[level as usize])
    }

    /// Total protrusion of the line from `q`'s break to `cur_p`, which is `p_disc`
    /// for a hyphenated break.
    pub(super) fn total_pw(
        &mut self,
        errors: &mut ErrorState,
        mem: &MemState,
        fonts: &FontState,
        q: NodePtr<ActiveNode>,
        p_disc: Option<NodePtr<DiscNode>>,
    ) -> Option<Scaled> {
        let break_node = q.break_node(mem);
        let mut l = if break_node.is_null() {
            self.first_p
        } else {
            mem[passive_node(break_node)].cur_break
        };

        let r = mem.prev_rightmost(self.global_prev_p, self.cur_p);
        let r = match p_disc {
            Some(d) if !mem[d].pre_break.is_null() => mem.list_tail(mem[d].pre_break),
            _ => mem.find_protchar_right(self, l, r)?,
        };

        let l_disc = if l.is_null() {
            None
        } else {
            mem.classify_disc(l).ok().flatten()
        };
        l = match l_disc {
            Some(d) if !mem[d].post_break.is_null() => mem[d].post_break,
            Some(d) => {
                l = mem[d].link;
                for _ in 0..mem[d].subtype {
                    if !l.link(mem).is_null() {
                        l = l.link(mem);
                    }
                }
                mem.find_protchar_left(self, l, true)?
            }
            None => mem.find_protchar_left(self, l, true)?,
        };

        Some(
            self.char_pw(errors, mem, fonts, l, Side::Left)
                + self.char_pw(errors, mem, fonts, r, Side::Right),
        )
    }

    /// The protrusion (`\lpcode`/`\rpcode` scaled to the font quad) of `p`.
    fn char_pw(
        &self,
        errors: &mut ErrorState,
        mem: &MemState,
        fonts: &FontState,
        p: MemPtr,
        side: Side,
    ) -> Scaled {
        fn protrusion(errors: &mut ErrorState, fonts: &FontState, f: FontId, cp: i32) -> Scaled {
            let quad = fonts.param(f, QUAD_CODE);
            errors.round_xn_over_d(quad, cp, 1000)
        }

        if p.is_null() {
            return Scaled::ZERO;
        }

        let (f, c) = if let Some(ch) = mem.as_char(p) {
            (ch.font(mem), ch.character(mem))
        } else {
            match mem.classify(p) {
                Ok(AnyNode::Whatsit(w)) => match w.whatsit_subtype(mem) {
                    Some(s) if s.is_native_word() => {
                        if w.native_glyph_info_handle(mem) == 0 {
                            return Scaled::ZERO;
                        }
                        let f = w.native_font(mem);
                        let cp = mem
                            .native_word_edge_glyph(w, side)
                            .map_or(0, |g| fonts.cp_codes.get(f, side, u32::from(g)));
                        return protrusion(errors, fonts, f, cp);
                    }
                    Some(WhatsitSubtype::Glyph) => {
                        let f = w.native_font(mem);
                        let cp = fonts.cp_codes.get(f, side, u32::from(w.native_glyph(mem)));
                        return protrusion(errors, fonts, f, cp);
                    }
                    _ => return Scaled::ZERO,
                },
                Ok(AnyNode::Ligature(lig)) => (lig.font(mem), lig.character(mem)),
                _ => return Scaled::ZERO,
            }
        };

        let cp = fonts.cp_codes.get(f, side, u32::from(c));
        if cp == 0 {
            return Scaled::ZERO;
        }
        protrusion(errors, fonts, f, cp)
    }

    /// Insert the right margin kern of the line ending at `q`, which is the last
    /// node of a discretionary's pre-break list when `pre_break_tail` is set.
    pub(super) fn insert_right_margin_kern(
        &mut self,
        errors: &mut ErrorState,
        mem: &mut MemState,
        fonts: &FontState,
        q: MemPtr,
        pre_break_tail: bool,
    ) -> Result<MemPtr, Abort> {
        let (p, ptmp) = if pre_break_tail {
            (q, q)
        } else {
            let head = TEMP_HEAD.link(mem);
            let ptmp = mem.prev_rightmost(head, q);
            let p = mem
                .find_protchar_right(self, head, ptmp)
                .ok_or(Abort::PdfError(&[
                    "insert_right_margin_kern",
                    "hlist stack failure",
                ]))?;
            (p, ptmp)
        };

        let w = self.char_pw(errors, mem, fonts, p, Side::Right);
        if w == Scaled::ZERO {
            return Ok(q);
        }

        let k = mem.new_margin_kern(-w, 1)?;
        mem[k].link = ptmp.link(mem);
        ptmp.set_link(mem, k.ptr());
        Ok(if ptmp == q { q.link(mem) } else { q })
    }

    pub(super) fn insert_left_margin_kern(
        &mut self,
        errors: &mut ErrorState,
        mem: &mut MemState,
        fonts: &FontState,
        q: MemPtr,
    ) -> Result<MemPtr, Abort> {
        let p = mem
            .find_protchar_left(self, q, false)
            .ok_or(Abort::PdfError(&[
                "insert_right_margin_kern",
                "hlist stack failure",
            ]))?;

        let w = self.char_pw(errors, mem, fonts, p, Side::Left);
        if w == Scaled::ZERO {
            return Ok(q);
        }

        let k = mem.new_margin_kern(-w, 0)?;
        mem[k].link = q;
        Ok(k.ptr())
    }
}

impl MemState {
    /// Whether `p` is a char node or a node that glue may break after.
    #[inline]
    fn is_non_discardable(&self, p: MemPtr) -> bool {
        self.is_char_node(p) || self.classify(p).is_ok_and(|n| n.precedes_break())
    }

    #[inline]
    fn is_empty_hlist(&self, b: NodePtr<BoxNode>) -> bool {
        self[b].is_hlist()
            && self[b].width == Scaled::ZERO
            && self[b].height == Scaled::ZERO
            && self[b].depth == Scaled::ZERO
            && self[b].list_ptr.is_null()
    }

    #[inline]
    fn is_empty_box(&self, p: MemPtr) -> bool {
        matches!(self.classify_box(p), Ok(Some(b)) if self.is_empty_hlist(b))
    }

    /// The list of `p` if `p` is an hlist with a nonempty list.
    #[inline]
    fn nonempty_hlist(&self, p: MemPtr) -> Option<MemPtr> {
        match self.classify_box(p) {
            Ok(Some(b)) if self[b].is_hlist() => {
                Some(self[b].list_ptr).filter(|list| !list.is_null())
            }
            _ => None,
        }
    }

    fn ignorable_protrusion_node(&self, p: MemPtr) -> bool {
        if self.is_char_node(p) {
            return false;
        }
        match self.classify(p) {
            Ok(AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_) | AnyNode::Penalty(_)) => {
                true
            }
            Ok(AnyNode::Disc(d)) => {
                self[d].pre_break.is_null() && self[d].post_break.is_null() && self[d].subtype == 0
            }
            Ok(AnyNode::Math(m)) => self[m].width == Scaled::ZERO,
            Ok(AnyNode::Kern(k)) => self[k].width == Scaled::ZERO || self[k].subtype == NORMAL,
            Ok(AnyNode::Glue(g)) => self[g].glue_ptr == ZERO_GLUE,
            Ok(AnyNode::Box(h)) => self.is_empty_hlist(h),
            _ => false,
        }
    }

    fn find_protchar_left(
        &self,
        linebreak: &mut LinebreakState,
        mut l: MemPtr,
        d: bool,
    ) -> Option<MemPtr> {
        if self.is_empty_box(l) {
            l = l.link(self);
        } else if d {
            while !l.link(self).is_null() && !self.is_non_discardable(l) {
                l = l.link(self);
            }
        }

        linebreak.hlist_stack_level = 0;
        let mut run = true;
        loop {
            let t = l;
            if run {
                while let Some(list) = self.nonempty_hlist(l) {
                    if !linebreak.push_node(l) {
                        return None;
                    }
                    l = list;
                }
            }

            while run && self.ignorable_protrusion_node(l) {
                while l.link(self).is_null() && linebreak.hlist_stack_level > 0 {
                    l = linebreak.pop_node()?;
                }

                if !l.link(self).is_null() {
                    l = l.link(self);
                } else if linebreak.hlist_stack_level == 0 {
                    run = false;
                }
            }

            if t == l {
                return Some(l);
            }
        }
    }

    fn find_protchar_right(
        &self,
        linebreak: &mut LinebreakState,
        mut l: MemPtr,
        mut r: MemPtr,
    ) -> Option<MemPtr> {
        if r.is_null() {
            return Some(MemPtr::NULL);
        }

        linebreak.hlist_stack_level = 0;
        let mut run = true;
        loop {
            let t = r;
            if run {
                while let Some(list) = self.nonempty_hlist(r) {
                    if !linebreak.push_node(l) || !linebreak.push_node(r) {
                        return None;
                    }
                    l = list;
                    r = self.list_tail(l);
                }
            }

            while run && self.ignorable_protrusion_node(r) {
                while r == l && linebreak.hlist_stack_level > 0 {
                    r = linebreak.pop_node()?;
                    l = linebreak.pop_node()?;
                }

                if r != l && !r.is_null() {
                    r = self.prev_rightmost(l, r);
                } else if r == l && linebreak.hlist_stack_level == 0 {
                    run = false;
                }
            }

            if t == r {
                return Some(r);
            }
        }
    }
}
