//! The paragraph builder: `line_break`, `post_line_break`, `hyphenate` and
//! character protrusion.

use crate::consts::{
    AWFUL_BAD, DIMEN_PAR_EMERGENCY_STRETCH, DIMEN_PAR_HANG_INDENT, DIMEN_PAR_HSIZE, EJECT_PENALTY,
    ETEX_PENALTIES_PAR_CLUB_PENALTIES, ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES,
    ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES, ETEX_PENALTIES_PAR_WIDOW_PENALTIES, ETEX_PEN_BASE,
    EXPLICIT, GLUE_PAR_LEFT_SKIP, GLUE_PAR_PAR_FILL_SKIP, GLUE_PAR_RIGHT_SKIP,
    GLUE_PAR_XETEX_LINEBREAK_SKIP, INF_BAD, INF_PENALTY, INT_PAR_ADJ_DEMERITS,
    INT_PAR_BROKEN_PENALTY, INT_PAR_CLUB_PENALTY, INT_PAR_DISPLAY_WIDOW_PENALTY,
    INT_PAR_DOUBLE_HYPHEN_DEMERITS, INT_PAR_EX_HYPHEN_PENALTY, INT_PAR_FINAL_HYPHEN_DEMERITS,
    INT_PAR_HANG_AFTER, INT_PAR_HYPHEN_PENALTY, INT_PAR_INTER_LINE_PENALTY, INT_PAR_LAST_LINE_FIT,
    INT_PAR_LINE_PENALTY, INT_PAR_LOOSENESS, INT_PAR_PRETOLERANCE, INT_PAR_TEXXET,
    INT_PAR_TOLERANCE, INT_PAR_UC_HYPH, INT_PAR_WIDOW_PENALTY, INT_PAR_XETEX_LINEBREAK_LOCALE,
    INT_PAR_XETEX_LINEBREAK_PENALTY, INT_PAR_XETEX_PROTRUDE_CHARS, INT_PAR_XETEX_USE_GLYPH_METRICS,
    L_CODE, MAX_HALFWORD, MIN_TRIE_OP, NON_ADDRESS, NORMAL, SMALL_NODE_SIZE, SPACE_ADJUSTMENT,
    TEX_NULL, TOO_BIG_CHAR,
};
use crate::errors::Abort;
use crate::ext::linebreak_start;
use crate::fonts::{LIG_TAG, QUAD_CODE};
use crate::hyphenation::HYPH_PRIME;
use crate::mem::{ACTIVE_LIST, ADJUST_HEAD, HOLD_HEAD, PRE_ADJUST_HEAD, TEMP_HEAD, ZERO_GLUE};
use crate::mem_types::ListStateRecord;
use crate::node::{
    ActiveNode, AnyBox, AnyNode, CharNode, DeltaNode, DiscNode, FixedNode, GlueNode, GlueSpecNode,
    HListNode, LigItem, MathNode, NodePtr, PassiveNode, WhatsitNode, WhatsitSubtype,
};
use crate::packaging::{ADDITIONAL, EXACTLY};
use crate::state::{
    ConfigState, EngineState, EqtbState, ErrorState, FontState, LinebreakState, MemState,
    ScannerState,
};
use crate::units::{FontId, MemAddr, MemIdx, MemPtr, NodeType, Scaled};

const LAST_ACTIVE: MemPtr = ACTIVE_LIST;

const DELTA_NODE: u16 = NodeType::Rule.as_raw();
const ACTIVE_NODE_SIZE_NORMAL: i16 = 3;
const ACTIVE_NODE_SIZE_EXTENDED: i16 = 5;
const MAX_HLIST_STACK: i16 = 512;
const BEST_BET_SET: &str = "best_bet is set by choose_best_break before use";

pub(crate) const UNHYPHENATED: i16 = 0;
pub(crate) const HYPHENATED: i16 = 1;
const VERY_LOOSE_FIT: usize = 0;
const LOOSE_FIT: usize = 1;
const DECENT_FIT: usize = 2;
const TIGHT_FIT: usize = 3;

extern "C" {
    fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;
}

#[inline]
fn odd(v: u8) -> bool {
    v % 2 == 1
}

impl FontState {
    #[inline]
    fn effective_char_width(&mut self, f: i32, c: u16) -> i32 {
        let ec = self.effective_char(f, c);
        let info = self.char_info(f, ec);
        self.char_width(f, info)
    }
}

/// A node on the active list, which carries no `NODE_type` of its own.
#[inline]
fn active(p: impl MemAddr) -> NodePtr<ActiveNode> {
    NodePtr::at(p)
}

#[inline]
fn delta_node(p: impl MemAddr) -> NodePtr<DeltaNode> {
    NodePtr::at(p)
}

#[inline]
fn passive_node(p: impl MemAddr) -> NodePtr<PassiveNode> {
    NodePtr::at(p)
}

/// An active-list entry. Its type word holds the break type (0/1) or
/// `DELTA_NODE` (2), which `classify()` would misread as HList/VList/Rule.
#[derive(Copy, Clone)]
enum ActiveOrDelta {
    Active(NodePtr<ActiveNode>),
    Delta(NodePtr<DeltaNode>),
}

/// A delta node's six components, indexed like `active_width` (slot 0 unused).
#[inline]
fn delta_widths(d: NodePtr<DeltaNode>, mem: &MemState) -> [i32; 7] {
    [
        0,
        d.dwidth(mem).0,
        d.dstretch0(mem).0,
        d.dstretch1(mem).0,
        d.dstretch2(mem).0,
        d.dstretch3(mem).0,
        d.dshrink(mem).0,
    ]
}

#[inline]
fn set_delta_widths(d: NodePtr<DeltaNode>, mem: &mut MemState, w: &[i32; 7]) {
    d.set_dwidth(mem, Scaled(w[1]));
    d.set_dstretch0(mem, Scaled(w[2]));
    d.set_dstretch1(mem, Scaled(w[3]));
    d.set_dstretch2(mem, Scaled(w[4]));
    d.set_dstretch3(mem, Scaled(w[5]));
    d.set_dshrink(mem, Scaled(w[6]));
}

impl MemState {
    #[inline]
    fn has_infinite_shrink(&self, spec: NodePtr<GlueSpecNode>) -> bool {
        spec.shrink_order(self) != NORMAL && spec.shrink(self).0 != 0
    }

    #[inline]
    fn classify_active_or_delta(&self, p: impl MemAddr) -> ActiveOrDelta {
        let p = p.addr();
        if active(p).break_type(self) == DELTA_NODE {
            ActiveOrDelta::Delta(delta_node(p))
        } else {
            ActiveOrDelta::Active(active(p))
        }
    }

    fn new_active_node(
        &mut self,
        active_node_size: i32,
        link: MemPtr,
    ) -> Result<NodePtr<ActiveNode>, Abort> {
        let p = self.get_node(active_node_size).ok_or(Abort::NodeOverflow)?;
        let p = active(p);
        p.set_link(self, link);
        Ok(p)
    }

    /// A new delta node before `link` holding `w[1..=6]`.
    fn new_delta_node(&mut self, link: MemPtr, w: &[i32; 7]) -> Result<NodePtr<DeltaNode>, Abort> {
        let p = NodePtr::<DeltaNode>::alloc(self).ok_or(Abort::NodeOverflow)?;
        p.set_link(self, link);
        p.set_subtype(self, 0);
        set_delta_widths(p, self, w);
        Ok(p)
    }

    /// The Unicode scalar at UTF-16 offset `i` of `w`'s text.
    fn get_native_usv(&self, w: NodePtr<WhatsitNode>, i: i32) -> i32 {
        let text = w.native_text_ptr(self);
        // SAFETY: `i` lies within the node's text.
        let c = unsafe { *text.add(i as usize) };

        if (0xD800..0xDC00).contains(&c) {
            // SAFETY: a high surrogate is followed by its low surrogate.
            let lo = unsafe { *text.add(i as usize + 1) };
            return 0x10000 + (c as i32 - 0xD800) * 0x400 + lo as i32 - 0xDC00;
        }

        c as i32
    }

    /// Copy `len` UTF-16 units of `src`'s text, starting at `start`, into `dst`.
    fn copy_native_text(
        &mut self,
        dst: NodePtr<WhatsitNode>,
        src: NodePtr<WhatsitNode>,
        start: i32,
        len: i32,
    ) {
        if len <= 0 {
            return;
        }
        let dst = dst.native_text_ptr_mut(self);
        let src = src.native_text_ptr(self);
        // SAFETY: both nodes are sized for the copied range, are distinct, and the
        // arena is not reallocated in between.
        unsafe { std::ptr::copy_nonoverlapping(src.add(start as usize), dst, len as usize) };
    }

    pub(crate) fn set_native_metrics(
        &mut self,
        eqtb: &EqtbState,
        fonts: &FontState,
        w: NodePtr<WhatsitNode>,
    ) -> Result<(), Abort> {
        let use_glyph_metrics = eqtb.int_par(INT_PAR_XETEX_USE_GLYPH_METRICS) > 0;
        // SAFETY: the measurer only reads font tables over FFI.
        unsafe { fonts.measure_native_node(self, w, use_glyph_metrics) }
    }

    /// A native-word node of font `f` holding `len` units of `src`'s text from `start`.
    fn native_word_fragment(
        &mut self,
        eqtb: &EqtbState,
        fonts: &FontState,
        f: i32,
        src: NodePtr<WhatsitNode>,
        start: i32,
        len: i32,
    ) -> Result<NodePtr<WhatsitNode>, Abort> {
        let q = self.new_native_word_node(eqtb, f, len)?;
        q.set_subtype(self, src.subtype(self));
        self.copy_native_text(q, src, start, len);
        self.set_native_metrics(eqtb, fonts, q)?;
        Ok(q)
    }
}

/// Split the native word `ha` at UTF-16 offset `start`, returning the tail
/// linked after it.
fn split_native_word(
    st: &mut EngineState,
    ha: NodePtr<WhatsitNode>,
    start: i32,
) -> Result<NodePtr<WhatsitNode>, Abort> {
    let len = ha.native_length(&st.mem) as i32;
    let tail =
        st.mem
            .native_word_fragment(&st.eqtb, &st.fonts, st.hyph.hf, ha, start, len - start)?;
    let next = ha.link(&st.mem);
    tail.set_link(&mut st.mem, next);
    ha.set_link(&mut st.mem, tail.ptr());
    ha.set_native_length(&mut st.mem, start as u16);
    st.mem.set_native_metrics(&st.eqtb, &st.fonts, ha)?;
    Ok(tail)
}

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
    fn total_pw(
        &mut self,
        errors: &mut ErrorState,
        mem: &MemState,
        fonts: &FontState,
        q: NodePtr<ActiveNode>,
        p_disc: Option<NodePtr<DiscNode>>,
    ) -> Option<i32> {
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
            mem.classify_disc(MemIdx::new(l)).ok().flatten()
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
    ) -> i32 {
        fn protrusion(errors: &mut ErrorState, fonts: &FontState, f: i32, cp: i32) -> i32 {
            let quad = fonts.param(f, QUAD_CODE);
            errors.round_xn_over_d(quad, cp, 1000)
        }

        if side == 0 {
            self.last_leftmost_char = MemPtr::NULL;
        } else {
            self.last_rightmost_char = MemPtr::NULL;
        }
        if p.is_null() {
            return 0;
        }

        let (char_ptr, f, c) = if let Some(ch) = mem.as_char(p) {
            (p, ch.font(mem).0, ch.character(mem))
        } else {
            match mem.classify(MemIdx::new(p)) {
                Ok(AnyNode::Whatsit(w)) => match w.whatsit_subtype(mem) {
                    Some(s) if s.is_native_word() => {
                        if w.native_glyph_info_handle(mem) == 0 {
                            return 0;
                        }
                        let f = w.native_font(mem).0;
                        // SAFETY: `p` is a native word with glyph info.
                        let cp = unsafe { mem.real_get_native_word_cp(w, side as libc::c_int) };
                        return protrusion(errors, fonts, f, cp);
                    }
                    Some(WhatsitSubtype::Glyph) => {
                        let f = w.native_font(mem).0;
                        // SAFETY: plain FFI lookup by font and glyph code.
                        let cp = unsafe { get_cp_code(f, w.native_glyph(mem) as u32, side as i32) };
                        return protrusion(errors, fonts, f, cp);
                    }
                    _ => return 0,
                },
                Ok(AnyNode::Ligature(lig)) => (lig.ptr() + 1, lig.font(mem).0, lig.character(mem)),
                _ => return 0,
            }
        };

        // SAFETY: plain FFI lookup by font and character code.
        let cp = unsafe { get_cp_code(f, c as u32, side as i32) };
        if side == 0 {
            self.last_leftmost_char = char_ptr;
        } else {
            self.last_rightmost_char = char_ptr;
        }
        if cp == 0 {
            return 0;
        }
        protrusion(errors, fonts, f, cp)
    }

    /// Insert the right margin kern of the line ending at `q`, which is the last
    /// node of a discretionary's pre-break list when `pre_break_tail` is set.
    fn insert_right_margin_kern(
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
        if w == 0 {
            return Ok(q);
        }

        let k = mem.new_margin_kern(-w, 1).ok_or(Abort::NodeOverflow)?;
        k.set_link(mem, ptmp.link(mem));
        ptmp.set_link(mem, k.ptr());
        Ok(if ptmp == q { q.link(mem) } else { q })
    }

    fn insert_left_margin_kern(
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
        if w == 0 {
            return Ok(q);
        }

        let k = mem.new_margin_kern(-w, 0).ok_or(Abort::NodeOverflow)?;
        k.set_link(mem, q);
        Ok(k.ptr())
    }
}

/// A lig item on `mainloop.lig_stack`, which `reconstitute` fills only with lig items.
#[inline]
fn lig_item(p: MemPtr) -> NodePtr<LigItem> {
    NodePtr::at(p)
}

impl MemState {
    /// Whether `p` is a char node or a node that glue may break after.
    #[inline]
    fn is_non_discardable(&self, p: MemPtr) -> bool {
        self.is_char_node(p)
            || self
                .classify(MemIdx::new(p))
                .is_ok_and(|n| n.precedes_break())
    }

    #[inline]
    fn is_empty_hlist(&self, b: NodePtr<HListNode>) -> bool {
        b.width(self).0 == 0
            && b.height(self).0 == 0
            && b.depth(self).0 == 0
            && b.list_ptr(self).is_null()
    }

    #[inline]
    fn is_empty_box(&self, p: MemPtr) -> bool {
        matches!(self.classify_box(MemIdx::new(p)), Ok(Some(AnyBox::HList(b))) if self.is_empty_hlist(b))
    }

    /// The list of `p` if `p` is an hlist with a nonempty list.
    #[inline]
    fn nonempty_hlist(&self, p: MemPtr) -> Option<MemPtr> {
        match self.classify_box(MemIdx::new(p)) {
            Ok(Some(AnyBox::HList(b))) => Some(b.list_ptr(self)).filter(|list| !list.is_null()),
            _ => None,
        }
    }

    fn ignorable_protrusion_node(&self, p: MemPtr) -> bool {
        if self.is_char_node(p) {
            return false;
        }
        match self.classify(MemIdx::new(p)) {
            Ok(AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_) | AnyNode::Penalty(_)) => {
                true
            }
            Ok(AnyNode::Disc(d)) => {
                d.pre_break(self).is_null() && d.post_break(self).is_null() && d.subtype(self) == 0
            }
            Ok(AnyNode::Math(m)) => m.width(self).0 == 0,
            Ok(AnyNode::Kern(k)) => k.width(self).0 == 0 || k.subtype(self) == NORMAL,
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

    /// A copy of `spec` with finite shrink, releasing `spec`.
    pub(crate) fn finite_shrink(
        &mut self,
        spec: NodePtr<GlueSpecNode>,
    ) -> Result<NodePtr<GlueSpecNode>, Abort> {
        let q = self.new_spec(spec).ok_or(Abort::NodeOverflow)?;
        q.set_shrink_order(self, NORMAL);
        self.delete_glue_ref(spec);
        Ok(q)
    }

    /// A fresh char node of font `f` holding `c`, linked after `t`.
    fn append_char(&mut self, t: MemPtr, f: i32, c: u16) -> Result<MemPtr, Abort> {
        let q = self.new_char(FontId(f), c).ok_or(Abort::AvailOverflow)?;
        t.set_link(self, q.ptr());
        Ok(q.ptr())
    }
}

/// Load `cur_r` for position `j` and return the hyphen char still to be tried there.
fn update_right(st: &mut EngineState, j: i16, n: i16, bchar: i32, hchar: i32) -> i32 {
    st.mainloop.cur_r = if j < n {
        st.hyph.hu[(j + 1) as usize]
    } else {
        bchar
    };
    if odd(st.hyph.hyf[j as usize]) {
        hchar
    } else {
        TOO_BIG_CHAR
    }
}

/// Pop the top lig item, appending its char node after `t`, and load the next `cur_r`.
#[allow(clippy::too_many_arguments)]
fn pop_lig_stack(
    st: &mut EngineState,
    t: &mut MemPtr,
    j: &mut i16,
    n: i16,
    bchar: i32,
    hchar: i32,
    cur_rh: &mut i32,
) {
    let item = lig_item(st.mainloop.lig_stack);
    let component = item.component(&st.mem);
    if component.0 > TEX_NULL {
        t.set_link(&mut st.mem, component);
        *t = component;
        *j += 1;
    }
    st.mainloop.lig_stack = item.ptr().link(&st.mem);
    st.mem.free_node(item.ptr(), SMALL_NODE_SIZE);
    if st.mainloop.lig_stack.is_null() {
        *cur_rh = update_right(st, *j, n, bchar, hchar);
    } else {
        st.mainloop.cur_r = lig_item(st.mainloop.lig_stack).character(&st.mem) as i32;
    }
}

/// Rebuild `hu[j..]` after `HOLD_HEAD` as chars, ligatures and kerns, returning
/// the index of the last character consumed.
fn reconstitute(
    st: &mut EngineState,
    mut j: i16,
    n: i16,
    mut bchar: i32,
    mut hchar: i32,
) -> Result<i16, Abort> {
    st.hyph.hyphen_passed = 0;
    let mut t = HOLD_HEAD;
    let mut w = 0;
    HOLD_HEAD.set_link(&mut st.mem, MemPtr::NULL);
    st.mainloop.cur_l = st.hyph.hu[j as usize];
    st.mainloop.cur_q = t;

    if j == 0 {
        st.mainloop.ligature_present = st.hyph.init_lig;
        let mut p = st.hyph.init_list;
        if st.mainloop.ligature_present {
            st.mainloop.lft_hit = st.hyph.init_lft;
        }
        while p.0 > TEX_NULL {
            let c = NodePtr::<CharNode>::at(p).character(&st.mem);
            t = st.mem.append_char(t, st.hyph.hf, c)?;
            p = p.link(&st.mem);
        }
    } else if st.mainloop.cur_l < TOO_BIG_CHAR {
        t = st
            .mem
            .append_char(t, st.hyph.hf, st.mainloop.cur_l as u16)?;
    }

    st.mainloop.lig_stack = MemPtr::NULL;
    let mut cur_rh = update_right(st, j, n, bchar, hchar);

    loop {
        'continue_: loop {
            let mut k;
            let mut q;

            if st.mainloop.cur_l == TOO_BIG_CHAR {
                k = st.fonts.bchar_label_entry(st.hyph.hf);
                if k == NON_ADDRESS {
                    break 'continue_;
                }
                q = st.fonts.font_info_b16(k);
            } else {
                let char_base = st.fonts.char_base_entry(st.hyph.hf);
                let effective = st
                    .fonts
                    .effective_char(st.hyph.hf, st.mainloop.cur_l as u16);
                q = st.fonts.font_info_b16(char_base + effective);
                if q.s1 % 4 != LIG_TAG {
                    break 'continue_;
                }
                k = st.fonts.lig_kern_base_entry(st.hyph.hf) + q.s0 as i32;
                q = st.fonts.font_info_b16(k);
                if q.s3 > 128 {
                    k = st.fonts.lig_kern_base_entry(st.hyph.hf)
                        + 256 * q.s1 as i32
                        + q.s0 as i32
                        + 32768
                        - 256 * 128;
                    q = st.fonts.font_info_b16(k);
                }
            }

            let mut test_char = if cur_rh < TOO_BIG_CHAR {
                cur_rh
            } else {
                st.mainloop.cur_r
            };

            loop {
                if q.s2 as i32 == test_char && q.s3 <= 128 {
                    if cur_rh < TOO_BIG_CHAR {
                        st.hyph.hyphen_passed = j;
                        hchar = TOO_BIG_CHAR;
                        cur_rh = TOO_BIG_CHAR;
                        continue 'continue_;
                    }

                    if hchar < TOO_BIG_CHAR && odd(st.hyph.hyf[j as usize]) {
                        st.hyph.hyphen_passed = j;
                        hchar = TOO_BIG_CHAR;
                    }

                    if q.s1 < 128 {
                        if st.mainloop.cur_l == TOO_BIG_CHAR {
                            st.mainloop.lft_hit = true;
                        }
                        if j == n && st.mainloop.lig_stack.is_null() {
                            st.mainloop.rt_hit = true;
                        }

                        match q.s1 {
                            1 | 5 => {
                                st.mainloop.cur_l = q.s0 as i32;
                                st.mainloop.ligature_present = true;
                            }
                            2 | 6 => {
                                st.mainloop.cur_r = q.s0 as i32;
                                if st.mainloop.lig_stack.0 > TEX_NULL {
                                    lig_item(st.mainloop.lig_stack)
                                        .set_character(&mut st.mem, st.mainloop.cur_r as u16);
                                } else {
                                    let item = st
                                        .mem
                                        .new_lig_item(st.mainloop.cur_r as u16)
                                        .ok_or(Abort::NodeOverflow)?;
                                    st.mainloop.lig_stack = item.ptr();
                                    if j == n {
                                        bchar = TOO_BIG_CHAR;
                                    } else {
                                        let p = st
                                            .mem
                                            .new_char(
                                                FontId(st.hyph.hf),
                                                st.hyph.hu[(j + 1) as usize] as u16,
                                            )
                                            .ok_or(Abort::AvailOverflow)?;
                                        item.set_component(&mut st.mem, p.ptr());
                                    }
                                }
                            }
                            3 => {
                                st.mainloop.cur_r = q.s0 as i32;
                                let below = st.mainloop.lig_stack;
                                let item = st
                                    .mem
                                    .new_lig_item(st.mainloop.cur_r as u16)
                                    .ok_or(Abort::NodeOverflow)?;
                                st.mainloop.lig_stack = item.ptr();
                                item.ptr().set_link(&mut st.mem, below);
                            }
                            7 | 11 => {
                                if st.mainloop.ligature_present {
                                    t = wrap_lig(st, false)?;
                                }
                                st.mainloop.cur_q = t;
                                st.mainloop.cur_l = q.s0 as i32;
                                st.mainloop.ligature_present = true;
                            }
                            _ => {
                                st.mainloop.cur_l = q.s0 as i32;
                                st.mainloop.ligature_present = true;
                                if st.mainloop.lig_stack.0 > TEX_NULL {
                                    pop_lig_stack(st, &mut t, &mut j, n, bchar, hchar, &mut cur_rh);
                                } else if j == n {
                                    break 'continue_;
                                } else {
                                    t = st.mem.append_char(
                                        t,
                                        st.hyph.hf,
                                        st.mainloop.cur_r as u16,
                                    )?;
                                    j += 1;
                                    cur_rh = update_right(st, j, n, bchar, hchar);
                                }
                            }
                        }

                        if q.s1 > 4 && q.s1 != 7 {
                            break 'continue_;
                        }
                        continue 'continue_;
                    }

                    w = st.fonts.char_kern(st.hyph.hf, q);
                    break 'continue_;
                }

                if q.s3 >= 128 {
                    if cur_rh == TOO_BIG_CHAR {
                        break 'continue_;
                    }
                    cur_rh = TOO_BIG_CHAR;
                    continue 'continue_;
                }

                k += q.s3 as i32 + 1;
                q = st.fonts.font_info_b16(k);
                test_char = if cur_rh < TOO_BIG_CHAR {
                    cur_rh
                } else {
                    st.mainloop.cur_r
                };
            }
        }

        if st.mainloop.ligature_present {
            t = wrap_lig(st, true)?;
        }

        if w != 0 {
            let kern = st.mem.new_kern(w).ok_or(Abort::NodeOverflow)?;
            t.set_link(&mut st.mem, kern.ptr());
            t = kern.ptr();
            w = 0;
            kern.set_sync_tag(&mut st.mem, 0);
        }

        if st.mainloop.lig_stack.0 > TEX_NULL {
            st.mainloop.cur_q = t;
            st.mainloop.cur_l = lig_item(st.mainloop.lig_stack).character(&st.mem) as i32;
            st.mainloop.ligature_present = true;
            pop_lig_stack(st, &mut t, &mut j, n, bchar, hchar, &mut cur_rh);
            continue;
        }

        return Ok(j);
    }
}

/// Replace the chars after `cur_q` by a ligature of `cur_l`, returning it.
/// `rt` also records a right boundary hit when the lig stack is empty.
fn wrap_lig(st: &mut EngineState, rt: bool) -> Result<MemPtr, Abort> {
    let components = st.mainloop.cur_q.link(&st.mem);
    let lig = st
        .mem
        .new_ligature(FontId(st.hyph.hf), st.mainloop.cur_l as u16, components)
        .ok_or(Abort::NodeOverflow)?;
    if st.mainloop.lft_hit {
        lig.set_subtype(&mut st.mem, 2);
        st.mainloop.lft_hit = false;
    }
    if rt && st.mainloop.rt_hit && st.mainloop.lig_stack.is_null() {
        let s = lig.subtype(&st.mem);
        lig.set_subtype(&mut st.mem, s + 1);
        st.mainloop.rt_hit = false;
    }
    st.mainloop.cur_q.set_link(&mut st.mem, lig.ptr());
    st.mainloop.ligature_present = false;
    Ok(lig.ptr())
}

impl MemState {
    fn start_linebreak_pass(
        &mut self,
        scanner: &mut ScannerState,
        linebreak: &mut LinebreakState,
        errors: &mut ErrorState,
        prev_graf: i32,
    ) -> Result<(), Abort> {
        let q = self.new_active_node(linebreak.active_node_size as i32, LAST_ACTIVE)?;
        q.set_break_type(self, UNHYPHENATED as u16);
        q.set_fitness(self, DECENT_FIT as u16);
        q.set_break_node(self, MemPtr::NULL);
        q.set_line_number(self, prev_graf + 1);
        q.set_total_demerits(self, 0);
        ACTIVE_LIST.set_link(self, q.ptr());
        if linebreak.do_last_line_fit {
            q.set_shortfall(self, Scaled(0));
            q.set_glue(self, Scaled(0));
        }

        scanner.active_width[1..].copy_from_slice(&linebreak.background[1..]);
        linebreak.passive = MemPtr::NULL;
        errors.font_in_short_display = 0;
        linebreak.cur_p = TEMP_HEAD.link(self);
        linebreak.global_prev_p = linebreak.cur_p;
        linebreak.first_p = linebreak.cur_p;
        Ok(())
    }

    fn free_break_lists(&mut self, linebreak: &LinebreakState) {
        let mut q = ACTIVE_LIST.link(self);
        while q != LAST_ACTIVE {
            let next = q.link(self);
            let size = match self.classify_active_or_delta(q) {
                ActiveOrDelta::Delta(_) => <DeltaNode as FixedNode>::SIZE,
                ActiveOrDelta::Active(_) => linebreak.active_node_size as i32,
            };
            self.free_node(q, size);
            q = next;
        }

        q = linebreak.passive;
        while !q.is_null() {
            let next = passive_node(q).link(self);
            passive_node(q).free(self);
            q = next;
        }
    }

    /// Pick `best_bet` and `best_line`, returning whether the result satisfies `\looseness`.
    fn choose_best_break(&self, linebreak: &mut LinebreakState, eqtb: &EqtbState) -> bool {
        if ACTIVE_LIST.link(self) == LAST_ACTIVE {
            return false;
        }

        let mut r = ACTIVE_LIST.link(self);
        let mut fewest_demerits = MAX_HALFWORD;
        loop {
            if let ActiveOrDelta::Active(a) = self.classify_active_or_delta(r) {
                if a.total_demerits(self) < fewest_demerits {
                    fewest_demerits = a.total_demerits(self);
                    linebreak.best_bet = Some(a);
                }
            }
            r = r.link(self);
            if r == LAST_ACTIVE {
                break;
            }
        }
        linebreak.best_line = linebreak.best_bet.expect(BEST_BET_SET).line_number(self);

        let looseness = eqtb.int_par(INT_PAR_LOOSENESS);
        if looseness == 0 {
            return true;
        }

        let mut r = ACTIVE_LIST.link(self);
        let mut actual_looseness = 0;
        loop {
            if let ActiveOrDelta::Active(a) = self.classify_active_or_delta(r) {
                let line_diff = a.line_number(self) - linebreak.best_line;
                if (line_diff < actual_looseness && looseness <= line_diff)
                    || (line_diff > actual_looseness && looseness >= line_diff)
                {
                    linebreak.best_bet = Some(a);
                    actual_looseness = line_diff;
                    fewest_demerits = a.total_demerits(self);
                } else if line_diff == actual_looseness && a.total_demerits(self) < fewest_demerits
                {
                    linebreak.best_bet = Some(a);
                    fewest_demerits = a.total_demerits(self);
                }
            }
            r = r.link(self);
            if r == LAST_ACTIVE {
                break;
            }
        }

        linebreak.best_line = linebreak.best_bet.expect(BEST_BET_SET).line_number(self);
        actual_looseness == looseness || linebreak.final_pass
    }

    /// Stretch the `\parfillskip` of the last line by the `\lastlinefit` adjustment.
    fn finish_last_line_fit(
        &mut self,
        linebreak: &mut LinebreakState,
        last_line_fill: NodePtr<GlueNode>,
    ) -> Result<(), Abort> {
        if !linebreak.do_last_line_fit {
            return Ok(());
        }

        let best = linebreak.best_bet.expect(BEST_BET_SET);
        if best.shortfall(self).0 == 0 {
            linebreak.do_last_line_fit = false;
            return Ok(());
        }

        let old_spec = last_line_fill.glue_ptr(self);
        let spec = self.new_spec(old_spec).ok_or(Abort::NodeOverflow)?;
        self.delete_glue_ref(old_spec);

        spec.set_width(
            self,
            Scaled(spec.width(self).0 + best.shortfall(self).0 - best.glue(self).0),
        );
        spec.set_stretch(self, Scaled(0));
        last_line_fill.set_glue_ptr(self, spec);
        Ok(())
    }
}

/// Make `spec`'s shrink finite, reporting it on the paragraph's first offence.
fn finite_shrink(
    st: &mut EngineState,
    spec: NodePtr<GlueSpecNode>,
) -> Result<NodePtr<GlueSpecNode>, Abort> {
    if st.errors.no_shrink_error_yet {
        st.errors.no_shrink_error_yet = false;
        crate::output::print_err(st, "Infinite glue shrinkage found in a paragraph");
        st.errors.help(&[
            "The paragraph just ended includes some glue that has",
            r"infinite shrinkability, e.g., `\hskip 0pt minus 1fil'.",
            "Such glue doesn't belong there---it allows a paragraph",
            "of any length to fit on one line. But it's safe to proceed,",
            "since the offensive shrinkability has been made finite.",
        ]);
        crate::errors::error(st)?;
    }
    Ok(st.mem.finite_shrink(spec)?)
}

#[inline]
fn line_break_try_break(
    st: &mut EngineState,
    pi: i32,
    break_type: i16,
    disc: Option<NodePtr<DiscNode>>,
) -> Result<(), Abort> {
    let s = &mut *st;
    s.mem.try_break(
        &mut s.scanner,
        &mut s.linebreak,
        &s.eqtb,
        &mut s.fonts,
        &s.config,
        &mut s.errors,
        pi,
        break_type,
        disc,
    )
}

/// TeX's `kern_break`: a kern or math node followed by glue is a legal breakpoint.
#[inline]
fn kern_break(st: &mut EngineState, next: MemPtr, auto_breaking: bool) -> Result<(), Abort> {
    if !st.mem.is_char_node(next)
        && auto_breaking
        && matches!(st.mem.classify(MemIdx::new(next)), Ok(AnyNode::Glue(_)))
    {
        line_break_try_break(st, 0, UNHYPHENATED, None)?;
    }
    Ok(())
}

impl MemState {
    /// The width of a node in a discretionary's list. `Err(true)` for a whatsit
    /// without dimensions, `Err(false)` for any other node that cannot occur there.
    #[inline]
    fn disc_node_width(&self, fonts: &mut FontState, p: MemPtr) -> Result<i32, bool> {
        if let Some(ch) = self.as_char(p) {
            return Ok(fonts.effective_char_width(ch.font(self).0, ch.character(self)));
        }

        match self.classify(MemIdx::new(p)) {
            Ok(AnyNode::Ligature(lig)) => {
                let f = lig.font(self).0;
                fonts.xtx_ligature_present = true;
                Ok(fonts.effective_char_width(f, lig.character(self)))
            }
            Ok(
                node
                @ (AnyNode::HList(_) | AnyNode::VList(_) | AnyNode::Rule(_) | AnyNode::Kern(_)),
            ) => Ok(node.hlist_width(self)),
            Ok(AnyNode::Whatsit(w)) => w
                .whatsit_subtype(self)
                .is_some_and(|s| s.has_dimensions())
                .then(|| w.width(self).0)
                .ok_or(true),
            _ => Err(false),
        }
    }
}

pub(crate) fn line_break(st: &mut EngineState, d: bool) -> Result<(), Abort> {
    st.scanner.pack_begin_line = st.input.cur_list.mode_line;
    let first = st.input.cur_list.head.link(&st.mem);
    TEMP_HEAD.set_link(&mut st.mem, first);

    let tail = st.input.cur_list.tail;
    let tail_glue = if st.mem.is_char_node(tail) {
        None
    } else if let Ok(AnyNode::Glue(g)) = st.mem.classify(MemIdx::new(tail)) {
        Some(g)
    } else {
        None
    };
    match tail_glue {
        None => {
            let p = st.mem.new_penalty(INF_PENALTY).ok_or(Abort::NodeOverflow)?;
            tail.set_link(&mut st.mem, p.ptr());
            st.input.cur_list.tail = p.ptr();
        }
        Some(tail_glue) => {
            // The trailing glue becomes the infinite penalty in place.
            let spec = tail_glue.glue_ptr(&st.mem);
            let leaders = tail_glue.leader_ptr(&st.mem);
            let tail_penalty = tail_glue.into_penalty(&mut st.mem);
            st.mem.delete_glue_ref(spec);
            st.mem.flush_node_list(leaders)?;
            tail_penalty.set_penalty(&mut st.mem, INF_PENALTY);
        }
    }

    let last_line_fill = st
        .mem
        .new_param_glue(&st.eqtb, GLUE_PAR_PAR_FILL_SKIP)
        .ok_or(Abort::NodeOverflow)?;
    st.input
        .cur_list
        .tail
        .set_link(&mut st.mem, last_line_fill.ptr());
    st.hyph.init_l_hyf = st.input.cur_list.prev_graf / 0x0040_0000;
    st.hyph.init_r_hyf = (st.input.cur_list.prev_graf / 65_536) % 64;
    let init_cur_lang = (st.input.cur_list.prev_graf % 65_536) as u8;

    st.mem.pop_nest(&mut st.input);
    st.errors.no_shrink_error_yet = true;

    for n in [GLUE_PAR_LEFT_SKIP, GLUE_PAR_RIGHT_SKIP] {
        let spec = st.eqtb.glue_par(n);
        if st.mem.has_infinite_shrink(spec) {
            let spec = finite_shrink(st, spec)?;
            st.eqtb.set_glue_par(n, spec);
        }
    }

    let left_skip = st.eqtb.glue_par(GLUE_PAR_LEFT_SKIP);
    let right_skip = st.eqtb.glue_par(GLUE_PAR_RIGHT_SKIP);
    let mem = &st.mem;
    let background = &mut st.linebreak.background;
    background[1] = left_skip.width(mem).0 + right_skip.width(mem).0;
    background[2..=5].fill(0);
    background[2 + left_skip.stretch_order(mem) as usize] = left_skip.stretch(mem).0;
    background[2 + right_skip.stretch_order(mem) as usize] += right_skip.stretch(mem).0;
    background[6] = left_skip.shrink(mem).0 + right_skip.shrink(mem).0;

    st.linebreak.do_last_line_fit = false;
    st.linebreak.active_node_size = ACTIVE_NODE_SIZE_NORMAL;
    if st.eqtb.int_par(INT_PAR_LAST_LINE_FIT) > 0 {
        let spec = last_line_fill.glue_ptr(&st.mem);
        let lb = &mut st.linebreak;
        if spec.stretch(&st.mem).0 > 0
            && spec.stretch_order(&st.mem) > NORMAL
            && lb.background[3..=5] == [0, 0, 0]
        {
            lb.do_last_line_fit = true;
            lb.active_node_size = ACTIVE_NODE_SIZE_EXTENDED;
            lb.fill_width = [0; 3];
            lb.fill_width[(spec.stretch_order(&st.mem) - 1) as usize] = spec.stretch(&st.mem).0;
        }
    }

    st.linebreak.minimum_demerits = AWFUL_BAD;
    st.linebreak.minimal_demerits = [AWFUL_BAD; 4];

    let par_shape = st.eqtb.par_shape();
    let lb = &mut st.linebreak;
    if let Some(shape) = par_shape {
        lb.last_special_line = shape.lines(&st.mem) - 1;
        lb.second_width = shape.line_width(&st.mem, lb.last_special_line + 1);
        lb.second_indent = shape.indent(&st.mem, lb.last_special_line + 1);
    } else {
        let hang_indent = st.eqtb.dimen_par(DIMEN_PAR_HANG_INDENT);
        let hsize = st.eqtb.dimen_par(DIMEN_PAR_HSIZE);
        if hang_indent == 0 {
            lb.last_special_line = 0;
            lb.second_width = hsize;
            lb.second_indent = 0;
        } else {
            let hang_after = st.eqtb.int_par(INT_PAR_HANG_AFTER);
            lb.last_special_line = hang_after.abs();
            if hang_after < 0 {
                lb.first_width = hsize - hang_indent.abs();
                lb.first_indent = hang_indent.max(0);
                lb.second_width = hsize;
                lb.second_indent = 0;
            } else {
                lb.first_width = hsize;
                lb.first_indent = 0;
                lb.second_width = hsize - hang_indent.abs();
                lb.second_indent = hang_indent.max(0);
            }
        }
    }

    lb.easy_line = if st.eqtb.int_par(INT_PAR_LOOSENESS) == 0 {
        lb.last_special_line
    } else {
        MAX_HALFWORD
    };

    lb.threshold = st.eqtb.int_par(INT_PAR_PRETOLERANCE);
    let mut second_pass;
    if lb.threshold >= 0 {
        second_pass = false;
        lb.final_pass = false;
    } else {
        lb.threshold = st.eqtb.int_par(INT_PAR_TOLERANCE);
        second_pass = true;
        lb.final_pass = st.eqtb.dimen_par(DIMEN_PAR_EMERGENCY_STRETCH) <= 0;
    }

    loop {
        st.linebreak.threshold = st.linebreak.threshold.min(INF_BAD);

        if second_pass {
            if st.hyph.trie_not_ready {
                st.hyph.init_trie()?;
            }
            st.hyph.cur_lang = init_cur_lang;
            st.hyph.l_hyf = st.hyph.init_l_hyf;
            st.hyph.r_hyf = st.hyph.init_r_hyf;
            st.hyph.set_hyph_index();
        }

        st.mem.start_linebreak_pass(
            &mut st.scanner,
            &mut st.linebreak,
            &mut st.errors,
            st.input.cur_list.prev_graf,
        )?;
        let mut auto_breaking = true;
        // Whether glue after the previous node is a legal breakpoint.
        let mut prev_breakable = false;

        while !st.linebreak.cur_p.is_null() && ACTIVE_LIST.link(&st.mem) != LAST_ACTIVE {
            let mut cur = st.linebreak.cur_p;

            if st.mem.is_char_node(cur) {
                prev_breakable = true;
                st.linebreak.global_prev_p = cur;
                loop {
                    let ch = NodePtr::<CharNode>::at(cur);
                    let f = ch.font(&st.mem).0;
                    let c = ch.character(&st.mem);
                    st.scanner.active_width[1] += st.fonts.effective_char_width(f, c);
                    cur = cur.link(&st.mem);
                    if !st.mem.is_char_node(cur) {
                        break;
                    }
                }
                st.linebreak.cur_p = cur;
            }

            let Ok(node) = st.mem.classify(MemIdx::new(cur)) else {
                return Err(Abort::Confusion("paragraph"));
            };
            match node {
                AnyNode::HList(_) | AnyNode::VList(_) | AnyNode::Rule(_) => {
                    st.scanner.active_width[1] += node.hlist_width(&st.mem);
                }
                AnyNode::Whatsit(w) => {
                    let subtype = w.whatsit_subtype(&st.mem);
                    if subtype == Some(WhatsitSubtype::Language) {
                        set_language(st, w);
                    } else if subtype.is_some_and(|s| s.has_dimensions()) {
                        st.scanner.active_width[1] += w.width(&st.mem).0;
                    }
                }
                AnyNode::Glue(cur_glue) => {
                    if auto_breaking && prev_breakable {
                        line_break_try_break(st, 0, UNHYPHENATED, None)?;
                    }

                    let mut spec = cur_glue.glue_ptr(&st.mem);
                    if st.mem.has_infinite_shrink(spec) {
                        spec = finite_shrink(st, spec)?;
                        cur_glue.set_glue_ptr(&mut st.mem, spec);
                    }
                    let aw = &mut st.scanner.active_width;
                    aw[1] += spec.width(&st.mem).0;
                    aw[2 + spec.stretch_order(&st.mem) as usize] += spec.stretch(&st.mem).0;
                    aw[6] += spec.shrink(&st.mem).0;

                    if second_pass && auto_breaking {
                        try_hyphenate(st, cur)?;
                    }
                }
                AnyNode::Kern(k) => {
                    if k.subtype(&st.mem) == EXPLICIT {
                        kern_break(st, k.link(&st.mem), auto_breaking)?;
                    }
                    st.scanner.active_width[1] += k.width(&st.mem).0;
                }
                AnyNode::Ligature(lig) => {
                    let f = lig.font(&st.mem).0;
                    st.fonts.xtx_ligature_present = true;
                    let c = lig.character(&st.mem);
                    st.scanner.active_width[1] += st.fonts.effective_char_width(f, c);
                }
                AnyNode::Disc(d) => {
                    let pre_break = d.pre_break(&st.mem);
                    st.linebreak.disc_width = 0;
                    if pre_break.is_null() {
                        let pi = st.eqtb.int_par(INT_PAR_EX_HYPHEN_PENALTY);
                        line_break_try_break(st, pi, HYPHENATED, Some(d))?;
                    } else {
                        let mut s = pre_break;
                        while !s.is_null() {
                            st.linebreak.disc_width += st
                                .mem
                                .disc_node_width(&mut st.fonts, s)
                                .map_err(|_| Abort::Confusion("disc3"))?;
                            s = s.link(&st.mem);
                        }
                        st.scanner.active_width[1] += st.linebreak.disc_width;
                        let pi = st.eqtb.int_par(INT_PAR_HYPHEN_PENALTY);
                        line_break_try_break(st, pi, HYPHENATED, Some(d))?;
                        st.scanner.active_width[1] -= st.linebreak.disc_width;
                    }

                    let mut s = d.link(&st.mem);
                    for _ in 0..d.subtype(&st.mem) {
                        st.scanner.active_width[1] += st
                            .mem
                            .disc_node_width(&mut st.fonts, s)
                            .map_err(|_| Abort::Confusion("disc4"))?;
                        s = s.link(&st.mem);
                    }

                    prev_breakable = true;
                    st.linebreak.global_prev_p = cur;
                    st.linebreak.cur_p = s;
                    continue;
                }
                AnyNode::Math(m) => {
                    if m.subtype(&st.mem) < L_CODE {
                        auto_breaking = odd(m.subtype(&st.mem) as u8);
                    }
                    kern_break(st, m.link(&st.mem), auto_breaking)?;
                    st.scanner.active_width[1] += m.width(&st.mem).0;
                }
                AnyNode::Penalty(pen) => {
                    line_break_try_break(st, pen.penalty(&st.mem), UNHYPHENATED, None)?;
                }
                AnyNode::Mark(_) | AnyNode::Ins(_) | AnyNode::Adjust(_) => {}
                _ => return Err(Abort::Confusion("paragraph")),
            }

            prev_breakable = node.precedes_break()
                || matches!(node, AnyNode::Kern(k) if k.subtype(&st.mem) != EXPLICIT);
            st.linebreak.global_prev_p = cur;
            st.linebreak.cur_p = cur.link(&st.mem);
        }

        if st.linebreak.cur_p.is_null() {
            line_break_try_break(st, EJECT_PENALTY, HYPHENATED, None)?;
            if st.mem.choose_best_break(&mut st.linebreak, &st.eqtb) {
                break;
            }
        }

        st.mem.free_break_lists(&st.linebreak);

        if !second_pass {
            st.linebreak.threshold = st.eqtb.int_par(INT_PAR_TOLERANCE);
            second_pass = true;
            st.linebreak.final_pass = st.eqtb.dimen_par(DIMEN_PAR_EMERGENCY_STRETCH) <= 0;
        } else {
            st.linebreak.background[2] += st.eqtb.dimen_par(DIMEN_PAR_EMERGENCY_STRETCH);
            st.linebreak.final_pass = true;
        }
    }

    st.mem
        .finish_last_line_fit(&mut st.linebreak, last_line_fill)?;

    post_line_break(st, d)?;

    st.mem.free_break_lists(&st.linebreak);

    st.scanner.pack_begin_line = 0;
    Ok(())
}

/// Switch hyphenation to the language of a `\language` whatsit.
fn set_language(st: &mut EngineState, w: NodePtr<WhatsitNode>) {
    st.hyph.cur_lang = w.what_lang(&st.mem) as u8;
    st.hyph.l_hyf = w.what_lhm(&st.mem) as i32;
    st.hyph.r_hyf = w.what_rhm(&st.mem) as i32;
    st.hyph.set_hyph_index();
}

impl MemState {
    /// Scan from `s` to the end of the word, returning the node that ends it, or
    /// `None` if a node there forbids hyphenation. `math_ok` lets a math node end it.
    fn find_word_end(&self, mut s: MemPtr, math_ok: bool) -> Option<MemPtr> {
        loop {
            if !self.is_char_node(s) {
                match self.classify(MemIdx::new(s)) {
                    Ok(AnyNode::Ligature(_)) => {}
                    Ok(AnyNode::Kern(k)) => {
                        if k.subtype(self) != NORMAL {
                            return Some(s);
                        }
                    }
                    Ok(
                        AnyNode::Whatsit(_)
                        | AnyNode::Glue(_)
                        | AnyNode::Penalty(_)
                        | AnyNode::Ins(_)
                        | AnyNode::Adjust(_)
                        | AnyNode::Mark(_),
                    ) => return Some(s),
                    Ok(AnyNode::Math(m)) if math_ok => {
                        return (m.subtype(self) >= L_CODE).then_some(s);
                    }
                    _ => return None,
                }
            }
            s = s.link(self);
        }
    }
}

/// Try to hyphenate the word following the glue node `cur`.
fn try_hyphenate(st: &mut EngineState, cur: MemPtr) -> Result<(), Abort> {
    let max_len = st.eqtb.max_hyphenatable_length();
    let uc_hyph = st.eqtb.int_par(INT_PAR_UC_HYPH) > 0;
    let mut prev_s = cur;
    let mut s = prev_s.link(&st.mem);
    if s.is_null() {
        return Ok(());
    }

    // Skip to the first letter, setting `hf`.
    loop {
        let c;
        if let Some(ch) = st.mem.as_char(s) {
            c = ch.character(&st.mem) as i32;
            st.hyph.hf = ch.font(&st.mem).0;
        } else {
            match st.mem.classify(MemIdx::new(s)) {
                Ok(AnyNode::Ligature(lig)) if !lig.lig_ptr(&st.mem).is_null() => {
                    let first = NodePtr::<CharNode>::at(lig.lig_ptr(&st.mem));
                    c = first.character(&st.mem) as i32;
                    st.hyph.hf = first.font(&st.mem).0;
                }
                Ok(AnyNode::Ligature(_)) => {
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                Ok(AnyNode::Kern(k)) if k.subtype(&st.mem) == NORMAL => {
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                Ok(AnyNode::Math(m)) if m.subtype(&st.mem) >= L_CODE => {
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                Ok(AnyNode::Whatsit(w)) => {
                    let subtype = w.whatsit_subtype(&st.mem);
                    if subtype.is_some_and(|sub| sub.is_native_word()) {
                        let len = w.native_length(&st.mem) as i32;
                        let mut l = 0;
                        while l < len {
                            let c = st.mem.get_native_usv(w, l);
                            if st.eqtb.lc_code(c) != 0 {
                                st.hyph.hf = w.native_font(&st.mem).0;
                                prev_s = s;
                                if st.eqtb.lc_code(c) == c || uc_hyph {
                                    break;
                                }
                                return Ok(());
                            }
                            if c >= 65_536 {
                                l += 1;
                            }
                            l += 1;
                        }
                        if l < len {
                            break;
                        }
                    }

                    if subtype == Some(WhatsitSubtype::Language) {
                        set_language(st, w);
                    }
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                _ => return Ok(()),
            }
        }

        st.hyph.hc[0] = st.hyph.hyph_code(&st.eqtb, c);
        if st.hyph.hc[0] != 0 {
            if st.hyph.hc[0] == c || uc_hyph {
                break;
            }
            return Ok(());
        }

        prev_s = s;
        s = prev_s.link(&st.mem);
    }

    st.hyph.hyf_char = st.fonts.hyphen_char[st.hyph.hf as usize];
    if st.hyph.hyf_char < 0 || st.hyph.hyf_char > TOO_BIG_CHAR - 1 {
        return Ok(());
    }
    st.linebreak.ha = prev_s;
    if st.hyph.l_hyf + st.hyph.r_hyf > max_len {
        return Ok(());
    }

    let native_ha = match st.mem.as_native_word(st.linebreak.ha) {
        Some(ha) => {
            let Some(end) = st.mem.find_word_end(ha.link(&st.mem), false) else {
                return Ok(());
            };
            s = end;
            Some(collect_native_word(st, ha, max_len)?)
        }
        None => {
            s = collect_word(st, s, max_len);
            None
        }
    };

    if (st.linebreak.hn as i32) < st.hyph.l_hyf + st.hyph.r_hyf
        || st.mem.find_word_end(s, true).is_none()
    {
        return Ok(());
    }

    Ok(hyphenate(st, native_ha)?)
}

/// Collect the letters of the native word `ha` into `hu`/`hc`, splitting it
/// where the hyphenatable part begins or ends. Returns the word's final node.
fn collect_native_word(
    st: &mut EngineState,
    mut ha: NodePtr<WhatsitNode>,
    max_len: i32,
) -> Result<NodePtr<WhatsitNode>, Abort> {
    st.linebreak.hn = 0;
    'restart: loop {
        let len = ha.native_length(&st.mem) as i32;
        let mut l = 0;
        while l < len {
            let c = st.mem.get_native_usv(ha, l);
            let hc0 = st.hyph.hyph_code(&st.eqtb, c);
            st.hyph.hc[0] = hc0;

            if hc0 == 0 {
                if st.linebreak.hn > 0 {
                    split_native_word(st, ha, l)?;
                    return Ok(ha);
                }
            } else if st.linebreak.hn == 0 && l > 0 {
                ha = split_native_word(st, ha, l)?;
                st.linebreak.ha = ha.ptr();
                continue 'restart;
            } else if st.linebreak.hn as i32 == max_len {
                return Ok(ha);
            } else {
                st.linebreak.hn += 1;
                let hn = st.linebreak.hn as usize;
                if c < 65_536 {
                    st.hyph.hu[hn] = c;
                    st.hyph.hc[hn] = hc0;
                } else {
                    st.hyph.hu[hn] = (c - 65_536) / 1024 + 0xD800;
                    st.hyph.hc[hn] = (hc0 - 65_536) / 1024 + 0xD800;
                    st.linebreak.hn += 1;
                    st.hyph.hu[hn + 1] = c % 1024 + 0xDC00;
                    st.hyph.hc[hn + 1] = hc0 % 1024 + 0xDC00;
                    l += 1;
                }
                st.hyph.hyf_bchar = TOO_BIG_CHAR;
            }
            l += 1;
        }
        return Ok(ha);
    }
}

/// Collect the letters of the word starting at `s` into `hu`/`hc`, setting
/// `hb` and `hyf_bchar`. Returns the node after the collected letters.
fn collect_word(st: &mut EngineState, mut s: MemPtr, max_len: i32) -> MemPtr {
    st.linebreak.hn = 0;
    loop {
        if let Some(ch) = st.mem.as_char(s) {
            if ch.font(&st.mem).0 != st.hyph.hf {
                return s;
            }
            let c = ch.character(&st.mem) as i32;
            st.hyph.hyf_bchar = c;
            let hc0 = st.hyph.hyph_code(&st.eqtb, c);
            st.hyph.hc[0] = hc0;
            if hc0 == 0 || hc0 > st.hyph.max_hyph_char || st.linebreak.hn as i32 == max_len {
                return s;
            }
            st.linebreak.hb = s;
            st.linebreak.hn += 1;
            st.hyph.hu[st.linebreak.hn as usize] = c;
            st.hyph.hc[st.linebreak.hn as usize] = hc0;
            st.hyph.hyf_bchar = TOO_BIG_CHAR;
        } else {
            match st.mem.classify(MemIdx::new(s)) {
                Ok(AnyNode::Ligature(lig)) => {
                    if lig.font(&st.mem).0 != st.hyph.hf {
                        return s;
                    }
                    let mut j = st.linebreak.hn;
                    let mut q = lig.lig_ptr(&st.mem);
                    if q.0 > TEX_NULL {
                        st.hyph.hyf_bchar = NodePtr::<CharNode>::at(q).character(&st.mem) as i32;
                    }
                    while q.0 > TEX_NULL {
                        let c = NodePtr::<CharNode>::at(q).character(&st.mem) as i32;
                        let hc0 = st.hyph.hyph_code(&st.eqtb, c);
                        st.hyph.hc[0] = hc0;
                        if hc0 == 0 || hc0 > st.hyph.max_hyph_char || j as i32 == max_len {
                            return s;
                        }
                        j += 1;
                        st.hyph.hu[j as usize] = c;
                        st.hyph.hc[j as usize] = hc0;
                        q = q.link(&st.mem);
                    }
                    st.linebreak.hb = s;
                    st.linebreak.hn = j;
                    st.hyph.hyf_bchar = if odd(lig.subtype(&st.mem) as u8) {
                        st.fonts.font_bchar[st.hyph.hf as usize]
                    } else {
                        TOO_BIG_CHAR
                    };
                }
                Ok(AnyNode::Kern(k)) if k.subtype(&st.mem) == NORMAL => {
                    st.linebreak.hb = s;
                    st.hyph.hyf_bchar = st.fonts.font_bchar[st.hyph.hf as usize];
                }
                _ => return s,
            }
        }
        s = s.link(&st.mem);
    }
}

struct PostLineHpack {
    q: MemPtr,
    cur_width: i32,
    cur_indent: i32,
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
        let texxet = eqtb.int_par(INT_PAR_TEXXET) > 0;
        let protrude = eqtb.int_par(INT_PAR_XETEX_PROTRUDE_CHARS) > 0;
        let cur_break = passive_node(self.cur_p).cur_break(mem);

        if texxet {
            let mut q = TEMP_HEAD.link(mem);
            if !lr_ptr.is_null() {
                let mut temp_ptr = *lr_ptr;
                let mut r = q;
                loop {
                    let s = mem
                        .new_math(0, (temp_ptr.info(mem) - 1) as u16)
                        .ok_or(Abort::NodeOverflow)?;
                    s.set_link(mem, r);
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
                    if let Ok(AnyNode::Math(m)) = mem.classify(MemIdx::new(q)) {
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
            match mem.classify(MemIdx::new(q)) {
                Ok(AnyNode::Glue(g)) => {
                    let old_spec = g.glue_ptr(mem);
                    mem.delete_glue_ref(old_spec);
                    let right_skip = eqtb.glue_par(GLUE_PAR_RIGHT_SKIP);
                    g.set_glue_ptr(mem, right_skip);
                    g.set_subtype(mem, GLUE_PAR_RIGHT_SKIP as u16 + 1);
                    mem.add_glue_ref(right_skip);
                    glue_break = true;
                }
                Ok(AnyNode::Disc(d)) => {
                    let mut r;
                    let t = d.subtype(mem);
                    if t == 0 {
                        r = d.link(mem);
                    } else {
                        r = q;
                        for _ in 1..t {
                            r = r.link(mem);
                        }
                        let s = r.link(mem);
                        r = s.link(mem);
                        s.set_link(mem, MemPtr::NULL);
                        mem.flush_node_list(d.link(mem))?;
                        d.set_subtype(mem, 0);
                    }

                    let post_break = d.post_break(mem);
                    if !post_break.is_null() {
                        mem.list_tail(post_break).set_link(mem, r);
                        r = post_break;
                        d.set_post_break(mem, MemPtr::NULL);
                        post_disc_break = true;
                    }

                    let pre_break = d.pre_break(mem);
                    if !pre_break.is_null() {
                        d.set_link(mem, pre_break);
                        q = mem.list_tail(pre_break);
                        d.set_pre_break(mem, MemPtr::NULL);
                        pre_break_tail = true;
                    }

                    q.set_link(mem, r);
                    disc_break = true;
                }
                Ok(AnyNode::Kern(k)) => k.set_width(mem, Scaled(0)),
                Ok(AnyNode::Math(m)) => {
                    m.set_width(mem, Scaled(0));
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
            let r = mem
                .new_param_glue(eqtb, GLUE_PAR_RIGHT_SKIP)
                .ok_or(Abort::NodeOverflow)?;
            r.set_link(mem, q.link(mem));
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
                let m = mem
                    .new_math(0, r.info(mem) as u16)
                    .ok_or(Abort::NodeOverflow)?;
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

        if eqtb.glue_par(GLUE_PAR_LEFT_SKIP) != ZERO_GLUE {
            let r = mem
                .new_param_glue(eqtb, GLUE_PAR_LEFT_SKIP)
                .ok_or(Abort::NodeOverflow)?;
            r.set_link(mem, q);
            q = r.ptr();
        }

        let (cur_width, cur_indent) = if cur_line > self.last_special_line {
            (self.second_width, self.second_indent)
        } else if let Some(shape) = eqtb.par_shape() {
            (shape.line_width(mem, cur_line), shape.indent(mem, cur_line))
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
    fn reverse_best_breaks(&mut self, linebreak: &mut LinebreakState) {
        let mut q = linebreak.best_bet.expect(BEST_BET_SET).break_node(self);
        linebreak.cur_p = MemPtr::NULL;
        loop {
            let r = passive_node(q);
            q = r.next_break(self);
            r.set_next_break(self, linebreak.cur_p);
            linebreak.cur_p = r.ptr();
            if q.is_null() {
                break;
            }
        }
    }

    /// Track the LR nesting of a math node on the `lr_ptr` stack.
    fn update_lr_stack(&mut self, q: NodePtr<MathNode>, lr_ptr: &mut MemPtr) -> Result<(), Abort> {
        let subtype = q.subtype(self);
        let end_lr = crate::mem::end_lr_type(subtype);
        if subtype % 2 == 1 {
            if !lr_ptr.is_null() && lr_ptr.info(self) == end_lr {
                self.pop_lr(lr_ptr);
            }
            Ok(())
        } else {
            self.push_lr(lr_ptr, end_lr).ok_or(Abort::AvailOverflow)
        }
    }

    /// Entry `n` (clamped to the last) of the eTeX penalty array `k`, if it is set.
    fn etex_penalty(&self, eqtb: &EqtbState, k: i32, n: i32) -> Option<i32> {
        let penalties = eqtb.penalties(ETEX_PEN_BASE + k)?;
        Some(penalties.penalty(self, n.min(penalties.count(self))))
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
        just_box: AnyBox,
        hpack: PostLineHpack,
        d: bool,
        lr_ptr: &mut MemPtr,
        cur_line: &mut i32,
    ) -> Result<bool, Abort> {
        scanner.just_box = Some(just_box);
        just_box.set_shift_amount(mem, Scaled(hpack.cur_indent));

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

        if *cur_line + 1 != linebreak.best_line {
            let mut pen = mem
                .etex_penalty(eqtb, ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES, *cur_line)
                .unwrap_or_else(|| eqtb.int_par(INT_PAR_INTER_LINE_PENALTY));

            match mem.etex_penalty(
                eqtb,
                ETEX_PENALTIES_PAR_CLUB_PENALTIES,
                *cur_line - self.prev_graf,
            ) {
                Some(p) => pen += p,
                None if *cur_line == self.prev_graf + 1 => {
                    pen += eqtb.int_par(INT_PAR_CLUB_PENALTY)
                }
                None => {}
            }

            let (widow_array, widow_penalty) = if d {
                (
                    ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES,
                    INT_PAR_DISPLAY_WIDOW_PENALTY,
                )
            } else {
                (ETEX_PENALTIES_PAR_WIDOW_PENALTIES, INT_PAR_WIDOW_PENALTY)
            };
            match mem.etex_penalty(eqtb, widow_array, linebreak.best_line - *cur_line - 1) {
                Some(p) => pen += p,
                None if *cur_line + 2 == linebreak.best_line => pen += eqtb.int_par(widow_penalty),
                None => {}
            }

            if hpack.disc_break {
                pen += eqtb.int_par(INT_PAR_BROKEN_PENALTY);
            }

            if pen != 0 {
                let r = mem.new_penalty(pen).ok_or(Abort::NodeOverflow)?;
                self.tail_append(mem, r.ptr());
            }
        }

        *cur_line += 1;
        linebreak.cur_p = passive_node(linebreak.cur_p).next_break(mem);

        if !linebreak.cur_p.is_null() && !hpack.post_disc_break {
            let texxet = eqtb.int_par(INT_PAR_TEXXET) > 0;
            let cur_break = passive_node(linebreak.cur_p).cur_break(mem);
            let mut r = TEMP_HEAD;
            let mut q;
            loop {
                q = r.link(mem);
                if q == cur_break || mem.is_char_node(q) {
                    break;
                }
                let node = mem.classify(MemIdx::new(q));
                match node {
                    Ok(node) if node.precedes_break() => break,
                    Ok(AnyNode::Kern(k))
                        if k.subtype(mem) != EXPLICIT && k.subtype(mem) != SPACE_ADJUSTMENT =>
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

/// Break the paragraph into lines along the chosen breakpoints and append them.
fn post_line_break(st: &mut EngineState, d: bool) -> Result<(), Abort> {
    let mut lr_ptr = st.input.cur_list.e_tex_aux;

    st.mem.reverse_best_breaks(&mut st.linebreak);

    let mut cur_line = st.input.cur_list.prev_graf + 1;
    loop {
        let hpack = match st.linebreak.post_line_break_prefix(
            &mut st.errors,
            &mut st.mem,
            &mut st.scanner,
            &st.eqtb,
            &st.fonts,
            &st.config,
            &mut lr_ptr,
            cur_line,
        ) {
            Ok(hpack) => hpack,
            Err(e) => return Err(e),
        };

        let (w, m) = if hpack.semantic_pagination {
            (0, ADDITIONAL)
        } else {
            (hpack.cur_width, EXACTLY)
        };
        let just_box = match crate::packaging::hpack(st, hpack.q, w, m) {
            Ok(b) => b,
            Err(e) => return Err(e),
        };

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
            Ok(true) => break,
            Ok(false) => {}
            Err(e) => return Err(e),
        }
    }

    if cur_line != st.linebreak.best_line || !TEMP_HEAD.link(&st.mem).is_null() {
        return Err(Abort::Confusion("line breaking"));
    }

    st.input.cur_list.prev_graf = st.linebreak.best_line - 1;
    st.input.cur_list.e_tex_aux = lr_ptr;
    Ok(())
}

#[inline]
fn stretch_fitness(b: i32) -> usize {
    if b > 99 {
        VERY_LOOSE_FIT
    } else if b > 12 {
        LOOSE_FIT
    } else {
        DECENT_FIT
    }
}

#[inline]
fn shrink_fitness(b: i32) -> usize {
    if b > 12 {
        TIGHT_FIT
    } else {
        DECENT_FIT
    }
}

/// Badness and fitness class of a line that is `shortfall` short with `stretch` available.
#[inline]
fn stretch_badness(shortfall: i32, stretch: i32) -> (i32, usize) {
    if shortfall > 7_230_584 && stretch < 1_663_497 {
        (INF_BAD, VERY_LOOSE_FIT)
    } else {
        let b = crate::helpers::badness(shortfall, stretch);
        (b, stretch_fitness(b))
    }
}

impl MemState {
    /// Set `break_width` to the width of the line material after a break at
    /// `cur_p`, which is `disc` for a hyphenated break.
    fn compute_break_width(
        &self,
        fonts: &mut FontState,
        linebreak: &mut LinebreakState,
        disc: Option<NodePtr<DiscNode>>,
    ) -> Result<(), Abort> {
        linebreak.break_width[1..].copy_from_slice(&linebreak.background[1..]);
        let mut s = linebreak.cur_p;

        if let Some(cur_disc) = disc {
            let mut v = cur_disc.ptr();
            s = cur_disc.post_break(self);

            for _ in 0..cur_disc.subtype(self) {
                v = v.link(self);
                linebreak.break_width[1] -= self.disc_node_width(fonts, v).map_err(|whatsit| {
                    Abort::Confusion(if whatsit { "disc1a" } else { "disc1" })
                })?;
            }

            while !s.is_null() {
                linebreak.break_width[1] += self.disc_node_width(fonts, s).map_err(|whatsit| {
                    Abort::Confusion(if whatsit { "disc2a" } else { "disc2" })
                })?;
                s = s.link(self);
            }

            linebreak.break_width[1] += linebreak.disc_width;
            if cur_disc.post_break(self).is_null() {
                s = v.link(self);
            }
        }

        while !s.is_null() && !self.is_char_node(s) {
            match self.classify(MemIdx::new(s)) {
                Ok(AnyNode::Glue(g)) => {
                    let spec = g.glue_ptr(self);
                    linebreak.break_width[1] -= spec.width(self).0;
                    linebreak.break_width[2 + spec.stretch_order(self) as usize] -=
                        spec.stretch(self).0;
                    linebreak.break_width[6] -= spec.shrink(self).0;
                }
                Ok(AnyNode::Penalty(_)) => {}
                Ok(AnyNode::Math(m)) => linebreak.break_width[1] -= m.width(self).0,
                Ok(AnyNode::Kern(k)) if k.subtype(self) == EXPLICIT => {
                    linebreak.break_width[1] -= k.width(self).0;
                }
                _ => break,
            }
            s = s.link(self);
        }
        Ok(())
    }

    /// Record the feasible breaks found so far as new active nodes before `r`, with
    /// delta nodes around them (TeX's "insert new active nodes").
    #[allow(clippy::too_many_arguments)]
    fn insert_new_active_nodes(
        &mut self,
        scanner: &mut ScannerState,
        linebreak: &mut LinebreakState,
        eqtb: &EqtbState,
        r: MemPtr,
        prev_r: &mut MemPtr,
        prev_prev_r: &mut MemPtr,
        break_type: i16,
    ) -> Result<(), Abort> {
        let bw = linebreak.break_width;
        let caw = linebreak.cur_active_width;
        if let ActiveOrDelta::Delta(prev_delta) = self.classify_active_or_delta(*prev_r) {
            let old = delta_widths(prev_delta, self);
            let w = std::array::from_fn(|k| old[k] - caw[k] + bw[k]);
            set_delta_widths(prev_delta, self, &w);
        } else if *prev_r == ACTIVE_LIST {
            scanner.active_width[1..].copy_from_slice(&bw[1..]);
        } else {
            let w = std::array::from_fn(|k| bw[k] - caw[k]);
            let q = self.new_delta_node(r, &w)?;
            prev_r.set_link(self, q.ptr());
            *prev_prev_r = *prev_r;
            *prev_r = q.ptr();
        }

        let adj = eqtb.int_par(INT_PAR_ADJ_DEMERITS).abs();
        if adj >= MAX_HALFWORD - linebreak.minimum_demerits {
            linebreak.minimum_demerits = AWFUL_BAD - 1;
        } else {
            linebreak.minimum_demerits += adj;
        }

        for fit_class in VERY_LOOSE_FIT..=TIGHT_FIT {
            if linebreak.minimal_demerits[fit_class] <= linebreak.minimum_demerits {
                let passive = NodePtr::<PassiveNode>::alloc(self).ok_or(Abort::NodeOverflow)?;
                passive.set_link(self, linebreak.passive);
                linebreak.passive = passive.ptr();
                passive.set_cur_break(self, linebreak.cur_p);
                passive.set_prev_break(self, linebreak.best_place[fit_class]);

                let q = self.new_active_node(linebreak.active_node_size as i32, r)?;
                q.set_break_node(self, linebreak.passive);
                q.set_line_number(self, linebreak.best_pl_line[fit_class] + 1);
                q.set_fitness(self, fit_class as u16);
                q.set_break_type(self, break_type as u16);
                q.set_total_demerits(self, linebreak.minimal_demerits[fit_class]);
                if linebreak.do_last_line_fit {
                    q.set_shortfall(self, Scaled(linebreak.best_pl_short[fit_class]));
                    q.set_glue(self, Scaled(linebreak.best_pl_glue[fit_class]));
                }

                prev_r.set_link(self, q.ptr());
                *prev_r = q.ptr();
            }
            linebreak.minimal_demerits[fit_class] = MAX_HALFWORD;
        }
        linebreak.minimum_demerits = MAX_HALFWORD;

        if r != LAST_ACTIVE {
            let w = std::array::from_fn(|k| caw[k] - bw[k]);
            let q = self.new_delta_node(r, &w)?;
            prev_r.set_link(self, q.ptr());
            *prev_prev_r = *prev_r;
            *prev_r = q.ptr();
        }
        Ok(())
    }

    /// Consider a break at `cur_p` with penalty `pi`, updating the active list.
    #[allow(clippy::too_many_arguments)]
    fn try_break(
        &mut self,
        scanner: &mut ScannerState,
        linebreak: &mut LinebreakState,
        eqtb: &EqtbState,
        fonts: &mut FontState,
        config: &ConfigState,
        errors: &mut ErrorState,
        mut pi: i32,
        break_type: i16,
        disc: Option<NodePtr<DiscNode>>,
    ) -> Result<(), Abort> {
        if config.semantic_pagination_enabled && !linebreak.cur_p.is_null() {
            return Ok(());
        }

        if pi.abs() >= INF_PENALTY {
            if pi > 0 {
                return Ok(());
            }
            pi = EJECT_PENALTY;
        }

        let mut no_break_yet = true;
        let mut prev_r = ACTIVE_LIST;
        let mut prev_prev_r = MemPtr::NULL;
        let mut old_l = 0;
        let mut line_width = 0;
        let mut g = 0;

        linebreak.cur_active_width[1..].copy_from_slice(&scanner.active_width[1..]);

        loop {
            let r = prev_r.link(self);

            if let ActiveOrDelta::Delta(r_delta) = self.classify_active_or_delta(r) {
                let w = delta_widths(r_delta, self);
                for (width, delta) in linebreak.cur_active_width.iter_mut().zip(w) {
                    *width += delta;
                }
                prev_prev_r = prev_r;
                prev_r = r;
                continue;
            }

            let r_active = active(r);
            let l = r_active.line_number(self);
            if l > old_l {
                if linebreak.minimum_demerits < AWFUL_BAD
                    && (old_l != linebreak.easy_line || r == LAST_ACTIVE)
                {
                    if no_break_yet {
                        no_break_yet = false;
                        self.compute_break_width(fonts, linebreak, disc)?;
                    }
                    self.insert_new_active_nodes(
                        scanner,
                        linebreak,
                        eqtb,
                        r,
                        &mut prev_r,
                        &mut prev_prev_r,
                        break_type,
                    )?;
                }

                if r == LAST_ACTIVE {
                    return Ok(());
                }

                if l > linebreak.easy_line {
                    line_width = linebreak.second_width;
                    old_l = MAX_HALFWORD - 1;
                } else {
                    old_l = l;
                    line_width = if l > linebreak.last_special_line {
                        linebreak.second_width
                    } else if let Some(shape) = eqtb.par_shape() {
                        shape.line_width(self, l)
                    } else {
                        linebreak.first_width
                    };
                }
            }

            let caw = &linebreak.cur_active_width;
            let mut artificial_demerits;
            let mut shortfall;
            if config.semantic_pagination_enabled {
                line_width = caw[1];
                artificial_demerits = true;
                shortfall = 0;
            } else {
                artificial_demerits = false;
                shortfall = line_width - caw[1];
                if eqtb.int_par(INT_PAR_XETEX_PROTRUDE_CHARS) > 1 {
                    shortfall += linebreak
                        .total_pw(errors, self, fonts, r_active, disc)
                        .ok_or(Abort::PdfError(&["total_pw", "hlist stack failure"]))?;
                }
            }

            let caw = linebreak.cur_active_width;
            let mut b = 0;
            let mut fit_class = DECENT_FIT;
            let mut last_line_fit_found = false;

            if shortfall > 0 {
                if caw[3] != 0 || caw[4] != 0 || caw[5] != 0 {
                    if linebreak.do_last_line_fit
                        && linebreak.cur_p.is_null()
                        && r_active.shortfall(self).0 != 0
                        && r_active.glue(self).0 > 0
                        && caw[3..6] == linebreak.fill_width
                    {
                        g = if r_active.shortfall(self).0 > 0 {
                            caw[2]
                        } else {
                            caw[6]
                        };

                        if g > 0 {
                            errors.arith_error = false;
                            g = errors.fract(
                                g,
                                r_active.shortfall(self).0,
                                r_active.glue(self).0,
                                MAX_HALFWORD,
                            );
                            let last_line_fit = eqtb.int_par(INT_PAR_LAST_LINE_FIT);
                            if last_line_fit < 1000 {
                                g = errors.fract(g, last_line_fit, 1000, MAX_HALFWORD);
                            }
                            if errors.arith_error {
                                g = if r_active.shortfall(self).0 > 0 {
                                    MAX_HALFWORD
                                } else {
                                    -MAX_HALFWORD
                                };
                            }

                            if g > 0 {
                                g = g.min(shortfall);
                                (b, fit_class) = stretch_badness(g, caw[2]);
                                last_line_fit_found = true;
                            } else if g < 0 {
                                g = g.max(-caw[6]);
                                b = crate::helpers::badness(-g, caw[6]);
                                fit_class = shrink_fitness(b);
                                last_line_fit_found = true;
                            }
                        }
                    }

                    if !last_line_fit_found {
                        shortfall = 0;
                        b = 0;
                        fit_class = DECENT_FIT;
                    }
                } else {
                    (b, fit_class) = stretch_badness(shortfall, caw[2]);
                }
            } else {
                b = if -shortfall > caw[6] {
                    INF_BAD + 1
                } else {
                    crate::helpers::badness(-shortfall, caw[6])
                };
                fit_class = shrink_fitness(b);
            }

            if linebreak.do_last_line_fit && !last_line_fit_found {
                if linebreak.cur_p.is_null() {
                    shortfall = 0;
                }
                g = match shortfall.signum() {
                    1 => caw[2],
                    -1 => caw[6],
                    _ => 0,
                };
            }

            let node_r_stays_active;
            if b > INF_BAD || pi == EJECT_PENALTY {
                if linebreak.final_pass
                    && linebreak.minimum_demerits == AWFUL_BAD
                    && r_active.link(self) == LAST_ACTIVE
                    && prev_r == ACTIVE_LIST
                {
                    artificial_demerits = true;
                } else if b > linebreak.threshold {
                    self.deactivate_node(scanner, linebreak, r, &mut prev_r, &mut prev_prev_r);
                    continue;
                }
                node_r_stays_active = false;
            } else {
                prev_r = r;
                if b > linebreak.threshold {
                    continue;
                }
                node_r_stays_active = true;
            }

            let mut d = if artificial_demerits {
                0
            } else {
                let mut d = eqtb.int_par(INT_PAR_LINE_PENALTY) + b;
                if d.abs() >= 10_000 {
                    d = 100_000_000;
                } else {
                    d *= d;
                }

                if pi > 0 {
                    d += pi * pi;
                } else if pi < 0 && pi > EJECT_PENALTY {
                    d -= pi * pi;
                }

                if break_type == HYPHENATED && r_active.break_type(self) as i16 == HYPHENATED {
                    d += if linebreak.cur_p.is_null() {
                        eqtb.int_par(INT_PAR_FINAL_HYPHEN_DEMERITS)
                    } else {
                        eqtb.int_par(INT_PAR_DOUBLE_HYPHEN_DEMERITS)
                    };
                }

                if (fit_class as i32 - r_active.fitness(self) as i32).abs() > 1 {
                    d += eqtb.int_par(INT_PAR_ADJ_DEMERITS);
                }

                d
            };

            d += r_active.total_demerits(self);
            if d <= linebreak.minimal_demerits[fit_class] {
                linebreak.minimal_demerits[fit_class] = d;
                linebreak.best_place[fit_class] = r_active.break_node(self);
                linebreak.best_pl_line[fit_class] = l;
                if linebreak.do_last_line_fit {
                    linebreak.best_pl_short[fit_class] = shortfall;
                    linebreak.best_pl_glue[fit_class] = g;
                }
                linebreak.minimum_demerits = linebreak.minimum_demerits.min(d);
            }

            if !node_r_stays_active {
                self.deactivate_node(scanner, linebreak, r, &mut prev_r, &mut prev_prev_r);
            }
        }
    }

    /// Unlink and free the active node `r`, merging the delta nodes around it.
    fn deactivate_node(
        &mut self,
        scanner: &mut ScannerState,
        linebreak: &mut LinebreakState,
        r: MemPtr,
        prev_r: &mut MemPtr,
        prev_prev_r: &mut MemPtr,
    ) {
        let next = r.link(self);
        prev_r.set_link(self, next);
        self.free_node(r, linebreak.active_node_size as i32);

        if *prev_r == ACTIVE_LIST {
            let r = ACTIVE_LIST.link(self);
            if let ActiveOrDelta::Delta(r_delta) = self.classify_active_or_delta(r) {
                let w = delta_widths(r_delta, self);
                for (width, delta) in scanner.active_width.iter_mut().zip(w) {
                    *width += delta;
                }
                linebreak.cur_active_width[1..].copy_from_slice(&scanner.active_width[1..]);
                ACTIVE_LIST.set_link(self, r_delta.link(self));
                delta_node(r).free(self);
            }
        } else if let ActiveOrDelta::Delta(prev_delta) = self.classify_active_or_delta(*prev_r) {
            let r = prev_r.link(self);
            if r == LAST_ACTIVE {
                let w = delta_widths(prev_delta, self);
                for (width, delta) in linebreak.cur_active_width.iter_mut().zip(w) {
                    *width -= delta;
                }
                prev_prev_r.set_link(self, LAST_ACTIVE);
                delta_node(*prev_r).free(self);
                *prev_r = *prev_prev_r;
            } else if let ActiveOrDelta::Delta(r_delta) = self.classify_active_or_delta(r) {
                let rw = delta_widths(r_delta, self);
                let pw = delta_widths(prev_delta, self);
                for (width, delta) in linebreak.cur_active_width.iter_mut().zip(rw) {
                    *width += delta;
                }
                set_delta_widths(prev_delta, self, &std::array::from_fn(|k| pw[k] + rw[k]));
                prev_delta.set_link(self, r_delta.link(self));
                delta_node(r).free(self);
            }
        }
    }
}

/// Hyphenate the native word `ha` by splitting it into fragments separated by
/// discretionaries.
fn hyphenate_native_word(
    st: &mut EngineState,
    ha: NodePtr<WhatsitNode>,
    hn: i16,
) -> Result<(), Abort> {
    let hf = st.hyph.hf;
    let hyf_char = st.hyph.hyf_char;
    let mut s = st.mem.predecessor(st.linebreak.cur_p, ha.ptr());

    st.hyph.hyphen_passed = 0;
    for j in st.hyph.l_hyf as i16..=(hn - st.hyph.r_hyf as i16) {
        if odd(st.hyph.hyf[j as usize]) {
            let hp = st.hyph.hyphen_passed;
            let q = st.mem.native_word_fragment(
                &st.eqtb,
                &st.fonts,
                hf,
                ha,
                hp as i32,
                (j - hp) as i32,
            )?;
            s.set_link(&mut st.mem, q.ptr());

            let disc = st.mem.new_disc().ok_or(Abort::NodeOverflow)?;
            let hyphen = crate::fonts::new_native_character(st, hf, hyf_char as u32)?;
            disc.set_pre_break(&mut st.mem, hyphen.ptr());
            q.set_link(&mut st.mem, disc.ptr());
            s = disc.ptr();
            st.hyph.hyphen_passed = j;
        }
    }

    let hn = ha.native_length(&st.mem) as i16;
    st.linebreak.hn = hn;
    let hp = st.hyph.hyphen_passed;
    let q =
        st.mem
            .native_word_fragment(&st.eqtb, &st.fonts, hf, ha, hp as i32, (hn - hp) as i32)?;
    s.set_link(&mut st.mem, q.ptr());
    let next = ha.link(&st.mem);
    q.set_link(&mut st.mem, next);
    ha.set_link(&mut st.mem, MemPtr::NULL);
    st.mem.flush_node_list(ha.ptr())
}

/// Find the hyphen positions of the word in `hc[1..=hn]` (from the exception
/// dictionary or the patterns) and insert the discretionaries. `native_ha` is
/// the word's node when it is a native word.
fn hyphenate(st: &mut EngineState, native_ha: Option<NodePtr<WhatsitNode>>) -> Result<(), Abort> {
    let mut hn = st.linebreak.hn;
    st.hyph.hyf[..=hn as usize].fill(0);

    let mut h = st.hyph.hc[1];
    hn += 1;
    st.hyph.hc[hn as usize] = st.hyph.cur_lang as i32;
    for j in 2..=hn as usize {
        h = (h + h + st.hyph.hc[j]) % HYPH_PRIME;
    }

    loop {
        let k = st.hyph.hyph_word[h as usize];
        if k == 0 {
            break;
        }

        if st.strings.length(k) == hn as i32 {
            let start = st.strings.str_start[(k - TOO_BIG_CHAR) as usize] as usize;
            let word = &st.strings.str_pool[start..start + hn as usize];
            if word
                .iter()
                .zip(&st.hyph.hc[1..=hn as usize])
                .all(|(&u, &c)| u as i32 == c)
            {
                let mut s = MemPtr(st.hyph.hyph_list[h as usize]);
                while !s.is_null() {
                    st.hyph.hyf[s.info(&st.mem) as usize] = 1;
                    s = s.link(&st.mem);
                }
                hn -= 1;
                st.linebreak.hn = hn;
                return insert_hyphens(st, hn, native_ha);
            }
        }

        let link = st.hyph.hyph_link[h as usize];
        if link == 0 {
            break;
        }
        h = link as i32 - 1;
    }

    hn -= 1;
    st.linebreak.hn = hn;
    let lang = st.hyph.cur_lang as usize;
    if st.hyph.trie_trc[lang + 1] != st.hyph.cur_lang as u16 {
        return Ok(());
    }

    let hyph = &mut st.hyph;
    hyph.hc[0] = 0;
    hyph.hc[(hn + 1) as usize] = 0;
    hyph.hc[(hn + 2) as usize] = hyph.max_hyph_char;

    for j in 0..=(hn - hyph.r_hyf as i16 + 1) {
        let mut z = hyph.trie_trl[lang + 1] + hyph.hc[j as usize];
        let mut l = j;
        while hyph.hc[l as usize] == hyph.trie_trc[z as usize] as i32 {
            if hyph.trie_tro[z as usize] != MIN_TRIE_OP {
                let mut v = hyph.trie_tro[z as usize];
                loop {
                    v += hyph.op_start[lang];
                    let i = l - hyph.hyf_distance[v as usize];
                    if hyph.hyf_num[v as usize] as u8 > hyph.hyf[i as usize] {
                        hyph.hyf[i as usize] = hyph.hyf_num[v as usize] as u8;
                    }
                    v = hyph.hyf_next[v as usize] as i32;
                    if v == MIN_TRIE_OP {
                        break;
                    }
                }
            }
            l += 1;
            z = hyph.trie_trl[z as usize] + hyph.hc[l as usize];
        }
    }

    insert_hyphens(st, hn, native_ha)
}

impl MemState {
    /// Append the list after `HOLD_HEAD` to `disc`'s pre- or post-break list,
    /// whose current tail is `minor_tail`.
    fn append_to_disc_list(
        &mut self,
        disc: NodePtr<DiscNode>,
        post: bool,
        minor_tail: &mut MemPtr,
    ) {
        let hold = HOLD_HEAD.link(self);
        if hold.0 <= TEX_NULL {
            return;
        }
        if !minor_tail.is_null() {
            minor_tail.set_link(self, hold);
        } else if post {
            disc.set_post_break(self, hold);
        } else {
            disc.set_pre_break(self, hold);
        }
        *minor_tail = self.list_tail(hold);
    }
}

/// Replace the word `ha..hb` by a reconstituted list with a discretionary at
/// every odd `hyf` position.
fn insert_hyphens(
    st: &mut EngineState,
    hn: i16,
    native_ha: Option<NodePtr<WhatsitNode>>,
) -> Result<(), Abort> {
    let l_hyf = st.hyph.l_hyf as i16;
    let r_hyf = st.hyph.r_hyf as i16;
    for j in 0..l_hyf {
        st.hyph.hyf[j as usize] = 0;
    }
    for j in 0..r_hyf {
        st.hyph.hyf[(hn - j) as usize] = 0;
    }

    if !(l_hyf..=hn - r_hyf).any(|j| odd(st.hyph.hyf[j as usize])) {
        return Ok(());
    }

    if let Some(ha) = native_ha {
        return hyphenate_native_word(st, ha, hn);
    }

    let hb = st.linebreak.hb;
    let ha = st.linebreak.ha;
    let q_final = hb.link(&st.mem);
    hb.set_link(&mut st.mem, MemPtr::NULL);
    let r = ha.link(&st.mem);
    ha.set_link(&mut st.mem, MemPtr::NULL);
    let bchar = st.hyph.hyf_bchar;

    let found2 = |st: &mut EngineState| {
        st.hyph.hu[0] = st.hyph.max_hyph_char;
        st.hyph.init_lig = false;
        st.hyph.init_list = MemPtr::NULL;
        (ha, 0)
    };
    let (mut s, mut j) = if let Some(ha_ch) = st.mem.as_char(ha) {
        if ha_ch.font(&st.mem).0 != st.hyph.hf {
            found2(st)
        } else {
            st.hyph.init_list = ha;
            st.hyph.init_lig = false;
            st.hyph.hu[0] = ha_ch.character(&st.mem) as i32;
            (st.mem.predecessor(st.linebreak.cur_p, ha), 0)
        }
    } else if let Ok(AnyNode::Ligature(ha_lig)) = st.mem.classify(MemIdx::new(ha)) {
        if ha_lig.font(&st.mem).0 != st.hyph.hf {
            found2(st)
        } else {
            st.hyph.init_list = ha_lig.lig_ptr(&st.mem);
            st.hyph.init_lig = true;
            st.hyph.init_lft = ha_lig.subtype(&st.mem) > 1;
            st.hyph.hu[0] = ha_lig.character(&st.mem) as i32;
            if st.hyph.init_list.is_null() && st.hyph.init_lft {
                st.hyph.hu[0] = st.hyph.max_hyph_char;
                st.hyph.init_lig = false;
            }
            st.mem.free_node(ha, SMALL_NODE_SIZE);
            (st.mem.predecessor(st.linebreak.cur_p, ha), 0)
        }
    } else if !st.mem.is_char_node(r)
        && matches!(
            st.mem.classify(MemIdx::new(r)),
            Ok(AnyNode::Ligature(lig)) if lig.subtype(&st.mem) > 1
        )
    {
        found2(st)
    } else {
        st.hyph.init_list = MemPtr::NULL;
        (ha, 1)
    };

    st.mem.flush_node_list(r)?;

    loop {
        let mut l = j;
        j = reconstitute(st, j, hn, bchar, st.hyph.hyf_char)? + 1;
        if st.hyph.hyphen_passed == 0 {
            let hold = HOLD_HEAD.link(&st.mem);
            s.set_link(&mut st.mem, hold);
            s = st.mem.list_tail(s);
            if odd(st.hyph.hyf[(j - 1) as usize]) {
                l = j;
                st.hyph.hyphen_passed = j - 1;
                HOLD_HEAD.set_link(&mut st.mem, MemPtr::NULL);
            }
        }

        if st.hyph.hyphen_passed > 0 {
            loop {
                let disc = NodePtr::<DiscNode>::alloc(&mut st.mem).ok_or(Abort::NodeOverflow)?;
                let hold = HOLD_HEAD.link(&st.mem);
                disc.set_link(&mut st.mem, hold);
                let mut major_tail = disc.ptr();
                let mut r_count = 0;
                while major_tail.link(&st.mem).0 > TEX_NULL {
                    major_tail = major_tail.link(&st.mem);
                    r_count += 1;
                }

                let mut i = st.hyph.hyphen_passed;
                st.hyph.hyf[i as usize] = 0;
                let mut minor_tail = MemPtr::NULL;
                disc.set_pre_break(&mut st.mem, MemPtr::NULL);
                let hyf_node =
                    crate::main_control::new_character(st, st.hyph.hf, st.hyph.hyf_char as u16)?;
                let mut saved_c = 0;
                if let Some(node) = hyf_node {
                    i += 1;
                    saved_c = st.hyph.hu[i as usize];
                    st.hyph.hu[i as usize] = st.hyph.hyf_char;
                    st.mem.free_avail(node.ptr());
                }

                while l <= i {
                    let font_bchar = st.fonts.font_bchar_entry(st.hyph.hf);
                    l = reconstitute(st, l, i, font_bchar, TOO_BIG_CHAR)? + 1;
                    st.mem.append_to_disc_list(disc, false, &mut minor_tail);
                }

                if hyf_node.is_some() {
                    st.hyph.hu[i as usize] = saved_c;
                    l = i;
                }

                minor_tail = MemPtr::NULL;
                disc.set_post_break(&mut st.mem, MemPtr::NULL);
                let mut c_loc = 0;
                let mut c = 0;
                if st.fonts.bchar_label_entry(st.hyph.hf) != NON_ADDRESS {
                    l -= 1;
                    c = st.hyph.hu[l as usize];
                    c_loc = l;
                    st.hyph.hu[l as usize] = st.hyph.max_hyph_char;
                }

                while l < j {
                    loop {
                        l = reconstitute(st, l, hn, bchar, TOO_BIG_CHAR)? + 1;
                        if c_loc > 0 {
                            st.hyph.hu[c_loc as usize] = c;
                            c_loc = 0;
                        }
                        st.mem.append_to_disc_list(disc, true, &mut minor_tail);
                        if l >= j {
                            break;
                        }
                    }

                    while l > j {
                        j = reconstitute(st, j, hn, bchar, TOO_BIG_CHAR)? + 1;
                        let hold = HOLD_HEAD.link(&st.mem);
                        major_tail.set_link(&mut st.mem, hold);
                        while major_tail.link(&st.mem).0 > TEX_NULL {
                            major_tail = major_tail.link(&st.mem);
                            r_count += 1;
                        }
                    }
                }

                if r_count > 127 {
                    let replacement = disc.link(&st.mem);
                    s.set_link(&mut st.mem, replacement);
                    disc.set_link(&mut st.mem, MemPtr::NULL);
                    st.mem.flush_node_list(disc.ptr())?;
                } else {
                    s.set_link(&mut st.mem, disc.ptr());
                    disc.set_subtype(&mut st.mem, r_count as u16);
                }
                s = major_tail;
                st.hyph.hyphen_passed = j - 1;
                HOLD_HEAD.set_link(&mut st.mem, MemPtr::NULL);

                if !odd(st.hyph.hyf[(j - 1) as usize]) {
                    break;
                }
            }
        }

        if j > hn {
            break;
        }
    }

    s.set_link(&mut st.mem, q_final);
    st.mem.flush_list(st.hyph.init_list);
    Ok(())
}

/// Append a native word of font `f` holding `text[..len]` to the current list.
///
/// # Safety
/// `text` must be valid for `len` reads.
unsafe fn append_native_word(
    st: &mut EngineState,
    f: i32,
    text: *const u16,
    len: i32,
) -> Result<(), Abort> {
    let w = st.mem.new_native_word_node(&st.eqtb, f, len)?;
    st.input.cur_list.tail_append(&mut st.mem, w.ptr());
    if len > 0 {
        let dst = w.native_text_ptr_mut(&mut st.mem);
        std::ptr::copy_nonoverlapping(text, dst, len as usize);
    }
    st.mem.set_native_metrics(&st.eqtb, &st.fonts, w)?;
    Ok(())
}

/// Append `native_text[s..s + len]` as native words, split at the locale's
/// line-break opportunities when `\XeTeXlinebreaklocale` is set.
///
/// # Safety
/// `s..s + len` must lie within `scanner.native_text`.
pub(crate) unsafe fn do_locale_linebreaks(
    st: &mut EngineState,
    s: i32,
    len: i32,
) -> Result<(), Abort> {
    let main_f = st.mainloop.main_f;
    // `native_text` is not resized here, so the pointer stays valid across the allocations.
    let native_text = st.scanner.native_text.as_mut_ptr();

    let locale = st.eqtb.int_par(INT_PAR_XETEX_LINEBREAK_LOCALE);
    if locale == 0 || len == 1 {
        return append_native_word(st, main_f, native_text.add(s as usize), len);
    }

    let use_skip = st.eqtb.glue_par(GLUE_PAR_XETEX_LINEBREAK_SKIP) != ZERO_GLUE;
    let penalty = st.eqtb.int_par(INT_PAR_XETEX_LINEBREAK_PENALTY);
    let use_penalty = penalty != 0 || !use_skip;
    linebreak_start(st, main_f, locale, native_text.add(s as usize), len)?;
    let mut offs = 0;
    loop {
        let prev_offs = offs;
        offs = st.fonts.linebreak_next(main_f);
        if offs > 0 {
            if prev_offs != 0 {
                if use_penalty {
                    let node = st.mem.new_penalty(penalty).ok_or(Abort::NodeOverflow)?;
                    st.input.cur_list.tail_append(&mut st.mem, node.ptr());
                }
                if use_skip {
                    let node = st
                        .mem
                        .new_param_glue(&st.eqtb, GLUE_PAR_XETEX_LINEBREAK_SKIP)
                        .ok_or(Abort::NodeOverflow)?;
                    st.input.cur_list.tail_append(&mut st.mem, node.ptr());
                }
            }
            append_native_word(
                st,
                main_f,
                native_text.add((s + prev_offs) as usize),
                offs - prev_offs,
            )?;
        }
        if offs < 0 {
            return Ok(());
        }
    }
}
