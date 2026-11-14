//! Character protrusion: margin kerns and the protruding chars at line ends.

use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::fonts::{QUAD_CODE, Side};
use crate::mem::{TEMP_HEAD, ZERO_GLUE};
use crate::node::{AnyNode, BoxNode, DiscNode, KernSubtype, NodePtr, WhatsitVariant};
use crate::state::{FontState, MemState};
use crate::units::MemIdx;
use crate::units::{FontId, MemPtr, Scaled};

use super::Breaker;
use super::lists::PassiveId;

/// The last index the hlist stack may push to, as in WEB.
const MAX_HLIST_STACK: usize = 512;

impl Breaker {
    fn push_node(&mut self, p: MemPtr) -> bool {
        if self.hlist_stack.len() > MAX_HLIST_STACK {
            return false;
        }
        self.hlist_stack.push(p);
        true
    }

    /// Total protrusion of the line from `break_node` (the paragraph start for
    /// `None`) to `cur_p`, which is `p_disc` for a hyphenated break.
    pub(super) fn total_pw(
        &mut self,
        mem: &MemState,
        fonts: &FontState,
        break_node: Option<PassiveId>,
        p_disc: Option<NodePtr<DiscNode>>,
    ) -> Option<Scaled> {
        let mut l = match break_node {
            None => self.first_p,
            Some(b) => self.lists.passive(b).cur_break,
        };

        let r = mem.prev_rightmost(self.global_prev_p, self.cur_p);
        let r = match p_disc.and_then(|d| mem[d].pre_break) {
            Some(pre) => Some(mem.list_tail(pre)),
            None => self.find_protchar_right(mem, l, r)?,
        };

        let l_disc = l.and_then(|l| mem.classify_disc(l).ok().flatten());
        l = match l_disc {
            Some(d) if !mem[d].post_break.is_none() => mem[d].post_break,
            Some(d) => {
                l = mem[d].link;
                for _ in 0..mem[d].replace_count {
                    if let Some(next) = l.and_then(|x| x.link(mem)) {
                        l = Some(next);
                    }
                }
                self.find_protchar_left(mem, l, true)?
            }
            None => self.find_protchar_left(mem, l, true)?,
        };

        Some(self.char_pw(mem, fonts, l, Side::Left) + self.char_pw(mem, fonts, r, Side::Right))
    }

    /// The protrusion (`\lpcode`/`\rpcode` scaled to the font quad) of `p`.
    fn char_pw(&self, mem: &MemState, fonts: &FontState, p: MemPtr, side: Side) -> Scaled {
        fn protrusion(fonts: &FontState, f: FontId, cp: i32) -> Scaled {
            let quad = fonts.param(f, QUAD_CODE);
            crate::scaled_math::round_xn_over_d(quad, cp, 1000)
        }

        let Some(p_idx) = p else {
            return Scaled::ZERO;
        };

        let (f, c) = if let Some(ch) = mem.as_char(p) {
            (mem[ch].font, mem[ch].character)
        } else {
            match mem.classify(p_idx) {
                Ok(AnyNode::Whatsit(w)) => match w.variant(mem) {
                    Some(WhatsitVariant::NativeWord(w)) => {
                        if mem[w].glyph_info == 0 {
                            return Scaled::ZERO;
                        }
                        let f = mem[w].font;
                        let cp = mem
                            .native_word_edge_glyph(w, side)
                            .map_or(0, |g| fonts.cp_codes.get(f, side, u32::from(g)));
                        return protrusion(fonts, f, cp);
                    }
                    Some(WhatsitVariant::Glyph(glyph)) => {
                        let glyph = mem[glyph];
                        let cp = fonts.cp_codes.get(glyph.font, side, u32::from(glyph.glyph));
                        return protrusion(fonts, glyph.font, cp);
                    }
                    _ => return Scaled::ZERO,
                },
                Ok(AnyNode::Ligature(lig)) => (mem[lig].font, mem[lig].character),
                _ => return Scaled::ZERO,
            }
        };

        let cp = fonts.cp_codes.get(f, side, u32::from(c));
        if cp == 0 {
            return Scaled::ZERO;
        }
        protrusion(fonts, f, cp)
    }

    /// Insert the right margin kern of the line ending at `q`, which is the last
    /// node of a discretionary's pre-break list when `pre_break_tail` is set.
    pub(super) fn insert_right_margin_kern(
        &mut self,
        mem: &mut MemState,
        fonts: &FontState,
        q: MemIdx,
        pre_break_tail: bool,
    ) -> Result<MemIdx, Abort> {
        let (p, ptmp) = if pre_break_tail {
            (Some(q), Some(q))
        } else {
            let head = TEMP_HEAD.link(mem);
            let ptmp = mem.prev_rightmost(head, Some(q));
            let p = self
                .find_protchar_right(mem, head, ptmp)
                .ok_or(Abort::PdfError(&[
                    "insert_right_margin_kern",
                    "hlist stack failure",
                ]))?;
            (p, ptmp)
        };

        let w = self.char_pw(mem, fonts, p, Side::Right);
        if w == Scaled::ZERO {
            return Ok(q);
        }

        let ptmp = ptmp.or_confusion("insert_right_margin_kern")?;
        let k = mem.new_margin_kern(-w, Side::Right)?;
        mem[k].link = ptmp.link(mem);
        ptmp.set_link(mem, k.ptr());
        Ok(if ptmp == q { k.idx() } else { q })
    }

    pub(super) fn insert_left_margin_kern(
        &mut self,
        mem: &mut MemState,
        fonts: &FontState,
        q: MemPtr,
    ) -> Result<MemPtr, Abort> {
        let p = self
            .find_protchar_left(mem, q, false)
            .ok_or(Abort::PdfError(&[
                "insert_right_margin_kern",
                "hlist stack failure",
            ]))?;

        let w = self.char_pw(mem, fonts, p, Side::Left);
        if w == Scaled::ZERO {
            return Ok(q);
        }

        let k = mem.new_margin_kern(-w, Side::Left)?;
        mem[k].link = q;
        Ok(k.ptr())
    }
}

impl MemState {
    /// Whether `p` is a char node or a node that glue may break after.
    #[inline]
    fn is_non_discardable(&self, p: MemPtr) -> bool {
        p.is_some_and(|p| {
            self.is_char_node(p) || self.classify(p).is_ok_and(|n| n.precedes_break())
        })
    }

    #[inline]
    fn is_empty_hlist(&self, b: NodePtr<BoxNode>) -> bool {
        self[b].is_hlist()
            && self[b].width == Scaled::ZERO
            && self[b].height == Scaled::ZERO
            && self[b].depth == Scaled::ZERO
            && self[b].list_ptr.is_none()
    }

    #[inline]
    fn is_empty_box(&self, p: MemPtr) -> bool {
        matches!(p.map(|p| self.classify_box(p)), Some(Ok(Some(b))) if self.is_empty_hlist(b))
    }

    /// The list of `p` if `p` is an hlist with a nonempty list.
    #[inline]
    fn nonempty_hlist(&self, p: MemPtr) -> Option<MemIdx> {
        match p.map(|p| self.classify_box(p)) {
            Some(Ok(Some(b))) if self[b].is_hlist() => self[b].list_ptr,
            _ => None,
        }
    }

    fn ignorable_protrusion_node(&self, p: MemPtr) -> bool {
        let Some(p) = p.filter(|&p| !self.is_char_node(p)) else {
            return false;
        };
        match self.classify(p) {
            Ok(AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_) | AnyNode::Penalty(_)) => {
                true
            }
            Ok(AnyNode::Disc(d)) => {
                self[d].pre_break.is_none()
                    && self[d].post_break.is_none()
                    && self[d].replace_count == 0
            }
            Ok(AnyNode::Math(m)) => self[m].width == Scaled::ZERO,
            Ok(AnyNode::Kern(k)) => {
                self[k].width == Scaled::ZERO || self[k].subtype == KernSubtype::Normal
            }
            Ok(AnyNode::Glue(g)) => self[g].glue_ptr == ZERO_GLUE,
            Ok(AnyNode::Box(h)) => self.is_empty_hlist(h),
            _ => false,
        }
    }
}

impl Breaker {
    fn find_protchar_left(&mut self, mem: &MemState, mut l: MemPtr, d: bool) -> Option<MemPtr> {
        if mem.is_empty_box(l) {
            l = l.and_then(|x| x.link(mem));
        } else if d {
            while let Some(x) =
                l.filter(|&x| x.link(mem).is_some() && !mem.is_non_discardable(Some(x)))
            {
                l = x.link(mem);
            }
        }

        self.hlist_stack.clear();
        let mut run = true;
        loop {
            let t = l;
            if run {
                while let Some(list) = mem.nonempty_hlist(l) {
                    if !self.push_node(l) {
                        return None;
                    }
                    l = Some(list);
                }
            }

            while run && mem.ignorable_protrusion_node(l) {
                while l.is_some_and(|x| x.link(mem).is_none()) && !self.hlist_stack.is_empty() {
                    l = self.hlist_stack.pop()?;
                }

                if let Some(next) = l.and_then(|x| x.link(mem)) {
                    l = Some(next);
                } else if self.hlist_stack.is_empty() {
                    run = false;
                }
            }

            if t == l {
                return Some(l);
            }
        }
    }

    fn find_protchar_right(
        &mut self,
        mem: &MemState,
        mut l: MemPtr,
        mut r: MemPtr,
    ) -> Option<MemPtr> {
        if r.is_none() {
            return Some(None);
        }

        self.hlist_stack.clear();
        let mut run = true;
        loop {
            let t = r;
            if run {
                while let Some(list) = mem.nonempty_hlist(r) {
                    if !self.push_node(l) || !self.push_node(r) {
                        return None;
                    }
                    l = Some(list);
                    r = Some(mem.list_tail(list));
                }
            }

            while run && mem.ignorable_protrusion_node(r) {
                while r == l && !self.hlist_stack.is_empty() {
                    r = self.hlist_stack.pop()?;
                    l = self.hlist_stack.pop()?;
                }

                if r != l && !r.is_none() {
                    r = mem.prev_rightmost(l, r);
                } else if r == l && self.hlist_stack.is_empty() {
                    run = false;
                }
            }

            if t == r {
                return Some(r);
            }
        }
    }
}
