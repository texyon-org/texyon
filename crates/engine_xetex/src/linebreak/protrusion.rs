//! Character protrusion: margin kerns and the protruding chars at line ends.

use crate::consts::NORMAL;
use crate::errors::Abort;
use crate::fonts::QUAD_CODE;
use crate::mem::{TEMP_HEAD, ZERO_GLUE};
use crate::node::{ActiveNode, AnyBox, AnyNode, DiscNode, HListNode, NodePtr, WhatsitSubtype};
use crate::state::{ErrorState, FontState, LinebreakState, MemState};
use crate::units::{FontId, MemPtr, Scaled};
use texyon_xetex_layout::get_cp_code;

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
            passive_node(break_node).cur_break(mem)
        };

        let r = mem.prev_rightmost(self.global_prev_p, self.cur_p);
        let r = match p_disc {
            Some(d) if !d.pre_break(mem).is_null() => mem.list_tail(d.pre_break(mem)),
            _ => mem.find_protchar_right(self, l, r)?,
        };

        let l_disc = if l.is_null() {
            None
        } else {
            mem.classify_disc(l).ok().flatten()
        };
        l = match l_disc {
            Some(d) if !d.post_break(mem).is_null() => d.post_break(mem),
            Some(d) => {
                l = d.link(mem);
                for _ in 0..d.subtype(mem) {
                    if !l.link(mem).is_null() {
                        l = l.link(mem);
                    }
                }
                mem.find_protchar_left(self, l, true)?
            }
            None => mem.find_protchar_left(self, l, true)?,
        };

        Some(self.char_pw(errors, mem, fonts, l, 0) + self.char_pw(errors, mem, fonts, r, 1))
    }

    /// The protrusion (`\lpcode`/`\rpcode` scaled to the font quad) of `p`, recording
    /// the protruding char in `last_leftmost_char`/`last_rightmost_char`.
    fn char_pw(
        &mut self,
        errors: &mut ErrorState,
        mem: &MemState,
        fonts: &FontState,
        p: MemPtr,
        side: i16,
    ) -> Scaled {
        fn protrusion(errors: &mut ErrorState, fonts: &FontState, f: FontId, cp: i32) -> Scaled {
            let quad = fonts.param(f, QUAD_CODE);
            errors.round_xn_over_d(quad, cp, 1000)
        }

        if side == 0 {
            self.last_leftmost_char = MemPtr::NULL;
        } else {
            self.last_rightmost_char = MemPtr::NULL;
        }
        if p.is_null() {
            return Scaled::ZERO;
        }

        let (char_ptr, f, c) = if let Some(ch) = mem.as_char(p) {
            (p, ch.font(mem), ch.character(mem))
        } else {
            match mem.classify(p) {
                Ok(AnyNode::Whatsit(w)) => match w.whatsit_subtype(mem) {
                    Some(s) if s.is_native_word() => {
                        if w.native_glyph_info_handle(mem) == 0 {
                            return Scaled::ZERO;
                        }
                        let f = w.native_font(mem);
                        let cp = mem.get_native_word_cp(w, side as libc::c_int);
                        return protrusion(errors, fonts, f, cp);
                    }
                    Some(WhatsitSubtype::Glyph) => {
                        let f = w.native_font(mem);
                        let cp =
                            get_cp_code(f.raw(), u32::from(w.native_glyph(mem)), i32::from(side));
                        return protrusion(errors, fonts, f, cp);
                    }
                    _ => return Scaled::ZERO,
                },
                Ok(AnyNode::Ligature(lig)) => (lig.ptr() + 1, lig.font(mem), lig.character(mem)),
                _ => return Scaled::ZERO,
            }
        };

        let cp = get_cp_code(f.raw(), u32::from(c), i32::from(side));
        if side == 0 {
            self.last_leftmost_char = char_ptr;
        } else {
            self.last_rightmost_char = char_ptr;
        }
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

        let w = self.char_pw(errors, mem, fonts, p, 1);
        if w == Scaled::ZERO {
            return Ok(q);
        }

        let k = mem.new_margin_kern(-w, 1)?;
        k.set_link(mem, ptmp.link(mem));
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

        let w = self.char_pw(errors, mem, fonts, p, 0);
        if w == Scaled::ZERO {
            return Ok(q);
        }

        let k = mem.new_margin_kern(-w, 0)?;
        k.set_link(mem, q);
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
    fn is_empty_hlist(&self, b: NodePtr<HListNode>) -> bool {
        b.width(self) == Scaled::ZERO
            && b.height(self) == Scaled::ZERO
            && b.depth(self) == Scaled::ZERO
            && b.list_ptr(self).is_null()
    }

    #[inline]
    fn is_empty_box(&self, p: MemPtr) -> bool {
        matches!(self.classify_box(p), Ok(Some(AnyBox::HList(b))) if self.is_empty_hlist(b))
    }

    /// The list of `p` if `p` is an hlist with a nonempty list.
    #[inline]
    fn nonempty_hlist(&self, p: MemPtr) -> Option<MemPtr> {
        match self.classify_box(p) {
            Ok(Some(AnyBox::HList(b))) => Some(b.list_ptr(self)).filter(|list| !list.is_null()),
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
                d.pre_break(self).is_null() && d.post_break(self).is_null() && d.subtype(self) == 0
            }
            Ok(AnyNode::Math(m)) => m.width(self) == Scaled::ZERO,
            Ok(AnyNode::Kern(k)) => k.width(self) == Scaled::ZERO || k.subtype(self) == NORMAL,
            Ok(AnyNode::Glue(g)) => g.glue_ptr(self) == ZERO_GLUE,
            Ok(AnyNode::HList(h)) => self.is_empty_hlist(h),
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
