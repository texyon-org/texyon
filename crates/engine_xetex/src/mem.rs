//! The `mem` arena: the one-word and variable-size free lists, the fixed list
//! heads, and TeX's node constructors.

pub(crate) use crate::consts::TEX_NULL;
use crate::consts::{GLUE_SPEC_SIZE, MAX_HALFWORD, MEDIUM_NODE_SIZE, MEM_TOP};
use crate::errors::Abort;
use crate::fonts::Side;
use crate::mem_types::ListStateRecord;
use crate::node::{
    BoxLr, BoxNode, CharNode, ChoiceNode, DiscNode, FixedNode, FreeBlock, GlueKind, GlueNode,
    GlueSpecNode, KernNode, KernSubtype, LigBoundary, LigItem, LigatureNode, MarginKernNode,
    MathCode, MathNode, Noad, NodePtr, PenaltyNode, RuleDimen, RuleNode, SpanNode, Stored,
    StoredCode, StyleNode, WhatsitKind, WhatsitSubtype,
};
use crate::params::GluePar;
use crate::state::{EqtbState, MemState};
use crate::token::TokenList;
use crate::units::{FontId, GlueOrder, GlueSign, MemAddr, MemIdx, MemPtr, Orientation, Scaled};

pub(crate) const ZERO_GLUE: NodePtr<GlueSpecNode> = static_spec(0);
pub(crate) const FIL_GLUE: NodePtr<GlueSpecNode> = static_spec(1);
pub(crate) const FILL_GLUE: NodePtr<GlueSpecNode> = static_spec(2);
pub(crate) const SS_GLUE: NodePtr<GlueSpecNode> = static_spec(3);
pub(crate) const FIL_NEG_GLUE: NodePtr<GlueSpecNode> = static_spec(4);

/// The `k`th of the glue specs that INITEX builds at the bottom of `mem`.
const fn static_spec(k: i32) -> NodePtr<GlueSpecNode> {
    NodePtr::from_idx_unchecked(MemIdx::from_raw(k * GLUE_SPEC_SIZE))
}

/// The arena length: `MEM_TOP` and the words below it, then layout slack.
pub(crate) const MEM_WORDS: usize = (MEM_TOP + crate::overlay::MAX_LAYOUT_WORDS) as usize;

pub(crate) const PAGE_INS_HEAD: MemPtr = MemPtr::from_raw(MEM_TOP);
pub(crate) const CONTRIB_HEAD: MemPtr = MemPtr::from_raw(MEM_TOP - 1);
pub(crate) const PAGE_HEAD: MemPtr = MemPtr::from_raw(MEM_TOP - 2);
pub(crate) const TEMP_HEAD: MemPtr = MemPtr::from_raw(MEM_TOP - 3);
pub(crate) const HOLD_HEAD: MemPtr = MemPtr::from_raw(MEM_TOP - 4);
/// `temp_head` and `hold_head` as heads of token lists.
pub(crate) const TEMP_HEAD_TOKS: TokenList = TokenList::from_ptr(TEMP_HEAD);
pub(crate) const HOLD_HEAD_TOKS: TokenList = TokenList::from_ptr(HOLD_HEAD);
pub(crate) const ADJUST_HEAD: MemPtr = MemPtr::from_raw(MEM_TOP - 5);
pub(crate) const ACTIVE_LIST: MemPtr = MemPtr::from_raw(MEM_TOP - 7);
pub(crate) const ALIGN_HEAD: MemPtr = MemPtr::from_raw(MEM_TOP - 8);
// SAFETY: the static word at `MEM_TOP - 9` ends every span list.
pub(crate) const END_SPAN: NodePtr<SpanNode> =
    NodePtr::from_idx_unchecked(MemIdx::from_raw(MEM_TOP - 9));
pub(crate) const OMIT_TEMPLATE: TokenList = TokenList::from_ptr(MemPtr::from_raw(MEM_TOP - 10));
pub(crate) const NULL_LIST: MemPtr = MemPtr::from_raw(MEM_TOP - 11);
pub(crate) const LIG_TRICK: MemPtr = MemPtr::from_raw(MEM_TOP - 12);
pub(crate) const GARBAGE: MemPtr = MemPtr::from_raw(MEM_TOP - 12);
pub(crate) const BACKUP_HEAD: TokenList = TokenList::from_ptr(MemPtr::from_raw(MEM_TOP - 13));
pub(crate) const PRE_ADJUST_HEAD: MemPtr = MemPtr::from_raw(MEM_TOP - 14);
pub(crate) const HI_MEM_STAT_MIN: i32 = MEM_TOP - 14;
pub(crate) const HI_MEM_STAT_USAGE: i32 = 15;

impl ListStateRecord {
    /// A `size`-word whatsit appended to the current list.
    pub fn new_whatsit<K: WhatsitKind>(
        &mut self,
        mem: &mut MemState,
        subtype: WhatsitSubtype,
        size: i32,
    ) -> Result<NodePtr<K>, Abort> {
        let p = NodePtr::<K>::alloc_sized(mem, size, subtype)?;
        self.tail_append(mem, p.ptr());
        Ok(p)
    }

    /// A fixed-layout whatsit appended to the current list.
    pub fn append_whatsit<K: WhatsitKind + FixedNode>(
        &mut self,
        mem: &mut MemState,
        subtype: WhatsitSubtype,
    ) -> Result<NodePtr<K>, Abort> {
        let p = NodePtr::<K>::alloc_whatsit(mem, subtype)?;
        self.tail_append(mem, p.ptr());
        Ok(p)
    }
}

// --- the one-word free list ---

impl MemState {
    /// TeX's `get_avail`.
    #[inline(always)]
    pub fn get_avail(&mut self) -> Result<MemPtr, Abort> {
        let p = self.avail;
        if !p.is_null() {
            self.avail = p.link(self);
            p.set_link(self, MemPtr::NULL);
            Ok(p)
        } else {
            self.get_avail_grow()
        }
    }

    #[cold]
    fn get_avail_grow(&mut self) -> Result<MemPtr, Abort> {
        let p = if self.mem_end < MemPtr::from_raw(MEM_TOP) {
            self.mem_end += 1;
            self.mem_end
        } else {
            self.hi_mem_min -= 1;
            if self.lo_mem_max >= self.hi_mem_min {
                return Err(Abort::AvailOverflow);
            }
            self.hi_mem_min
        };
        p.set_link(self, MemPtr::NULL);
        Ok(p)
    }

    /// Returns the one-word node `p` to the free list.
    #[inline]
    pub fn free_avail(&mut self, p: MemPtr) {
        p.set_link(self, self.avail);
        self.avail = p;
    }

    /// Returns the one-word list `p` to the free list.
    pub fn flush_list(&mut self, p: MemPtr) {
        if p.is_null() {
            return;
        }
        self.list_tail(p).set_link(self, self.avail);
        self.avail = p;
    }

    /// `flush_list_with_tail` for a token list.
    #[inline]
    pub(crate) fn flush_token_list_with_tail(&mut self, p: TokenList, tail: TokenList) {
        self.flush_list_with_tail(p.ptr(), tail.ptr());
    }

    /// `flush_list` for a list whose last node `tail` the caller already knows.
    #[inline]
    pub fn flush_list_with_tail(&mut self, p: MemPtr, tail: MemPtr) {
        if p.is_null() {
            return;
        }
        debug_assert_eq!(self.list_tail(p), tail);
        tail.set_link(self, self.avail);
        self.avail = p;
    }

    /// TeX's `store_new_token`: appends `tok` after `tail` and makes it the tail.
    #[inline(always)]
    pub(crate) fn store_new_token(
        &mut self,
        tail: &mut TokenList,
        tok: crate::token::Token,
    ) -> Result<(), Abort> {
        let q = self.get_avail_token()?;
        tail.set_link(self, q);
        q.set_token(self, tok);
        *tail = q;
        Ok(())
    }

    /// `get_avail` for a token node.
    #[inline(always)]
    pub(crate) fn get_avail_token(&mut self) -> Result<TokenList, Abort> {
        self.get_avail().map(TokenList::from_ptr)
    }

    /// Returns the token list `p` to the free list.
    #[inline]
    pub(crate) fn flush_token_list(&mut self, p: TokenList) {
        self.flush_list(p.ptr());
    }

    #[inline]
    pub(crate) fn add_token_ref(&mut self, p: TokenList) {
        p.set_ref_count(self, p.ref_count(self) + 1);
    }

    /// Drops a reference to token list `p`, freeing it with the last one.
    pub(crate) fn delete_token_ref(&mut self, p: TokenList) {
        if p.ref_count(self) == TEX_NULL {
            self.flush_token_list(p);
        } else {
            p.set_ref_count(self, p.ref_count(self) - 1);
        }
    }

    /// The last node of the non-empty list starting at `p`.
    #[inline]
    pub(crate) fn list_tail(&self, mut p: MemPtr) -> MemPtr {
        while !p.link(self).is_null() {
            p = p.link(self);
        }
        p
    }

    /// The node whose link is `target`, searching forward from `p`.
    #[inline]
    pub(crate) fn predecessor(&self, mut p: MemPtr, target: MemPtr) -> MemPtr {
        while p.link(self) != target {
            p = p.link(self);
        }
        p
    }

    /// The node whose link is `e`, searching forward from `s`, or null if the
    /// list ends first.
    pub fn prev_rightmost(&self, s: MemPtr, e: MemPtr) -> MemPtr {
        let mut p = s;
        if p.is_null() {
            return MemPtr::NULL;
        }
        while p.link(self) != e {
            p = p.link(self);
            if p.is_null() {
                return MemPtr::NULL;
            }
        }
        p
    }

    /// Skips the nodes that may sit between the words of an interword space.
    pub(crate) fn skip_interword_fillers(&self, mut p: MemPtr) -> MemPtr {
        while self
            .classify_non_char(p)
            .is_some_and(|n| n.is_interword_filler(self))
        {
            p = p.link(self);
        }
        p
    }

    /// Whether `p` lies in the one-word region, where every node is a character.
    #[inline]
    pub fn is_char_node(&self, p: impl MemAddr) -> bool {
        p.addr() >= self.hi_mem_min.raw()
    }

    /// TeX's `push_LR`.
    #[inline]
    pub(crate) fn push_lr(&mut self, lr_ptr: &mut MemPtr, code: MathCode) -> Result<(), Abort> {
        let t = self.get_avail()?;
        self.temp_ptr = t;
        t.set_info(self, i32::from(Stored::from(code).raw()));
        t.set_link(self, *lr_ptr);
        *lr_ptr = t;
        Ok(())
    }

    /// The code in LR stack entry `p`, `None` for anything else.
    #[inline]
    pub(crate) fn lr_code(&self, p: MemPtr) -> Option<MathCode> {
        u16::try_from(p.info(self))
            .ok()
            .and_then(MathCode::from_u16)
    }

    /// TeX's `pop_LR`.
    #[inline]
    pub(crate) fn pop_lr(&mut self, lr_ptr: &mut MemPtr) {
        let t = *lr_ptr;
        self.temp_ptr = t;
        *lr_ptr = t.link(self);
        self.free_avail(t);
    }

    // --- the variable-size free list ---

    /// TeX's `get_node`. The request `0x4000_0000` is `sort_avail`'s probe
    /// and yields `MAX_HALFWORD`.
    pub fn get_node(&mut self, s: i32) -> Result<MemPtr, Abort> {
        let block = NodePtr::<FreeBlock>::at;
        let r = 'restart: loop {
            let mut p = self.rover;
            loop {
                // Merge the free blocks that follow `p`.
                let mut q = p + self[block(p)].node_size;
                while self[block(q)].is_empty() {
                    let t = self[block(q)].rlink;
                    if q == self.rover {
                        self.rover = t;
                    }
                    let q_left = self[block(q)].llink;
                    self[block(t)].llink = q_left;
                    self[block(q_left)].rlink = t;
                    q += self[block(q)].node_size;
                }

                let r = q - s;
                if r > p + 1 {
                    self[block(p)].node_size = r - p;
                    self.rover = p;
                    break 'restart r;
                }
                if r == p && self[block(p)].rlink != p {
                    let new_rover = self[block(p)].rlink;
                    let t = self[block(p)].llink;
                    self.rover = new_rover;
                    self[block(new_rover)].llink = t;
                    self[block(t)].rlink = new_rover;
                    break 'restart r;
                }

                self[block(p)].node_size = q - p;
                p = self[block(p)].rlink;
                if p == self.rover {
                    break;
                }
            }

            if s == 0x4000_0000 {
                return Ok(MemPtr::from_raw(MAX_HALFWORD));
            }

            let max_halfword = MemPtr::from_raw(MAX_HALFWORD);
            if self.lo_mem_max + 2 < self.hi_mem_min && self.lo_mem_max + 2 <= max_halfword {
                // Grow lo-mem by a new free block below `hi_mem_min`.
                let t = if self.hi_mem_min - self.lo_mem_max >= 1998 {
                    self.lo_mem_max + 1000
                } else {
                    self.lo_mem_max + 1 + (self.hi_mem_min - self.lo_mem_max) / 2
                };
                let t = t.min(max_halfword);

                let rover = self.rover;
                let left_neighbor = self[block(rover)].llink;
                let q = self.lo_mem_max;
                self[block(left_neighbor)].rlink = q;
                self[block(rover)].llink = q;
                self[block(q)].rlink = rover;
                self[block(q)].llink = left_neighbor;
                self[block(q)].mark_empty();
                self[block(q)].node_size = t - self.lo_mem_max;
                self.lo_mem_max = t;
                t.set_link(self, MemPtr::NULL);
                t.set_info(self, TEX_NULL);
                self.rover = q;
                continue 'restart;
            }

            return Err(Abort::NodeOverflow);
        };

        r.set_link(self, MemPtr::NULL);
        if s >= MEDIUM_NODE_SIZE {
            self.set_b32s0(r + s - 1, self.cur_synctex_tag);
            self.set_b32s1(r + s - 1, self.line);
        }
        Ok(r)
    }

    /// Returns the `s`-word node `p` to the variable-size free list.
    pub fn free_node(&mut self, p: MemPtr, s: i32) {
        let block = NodePtr::<FreeBlock>::at;
        let rover = self.rover;
        let q = self[block(rover)].llink;
        let freed = &mut self[block(p)];
        freed.node_size = s;
        freed.mark_empty();
        freed.llink = q;
        freed.rlink = rover;
        self[block(rover)].llink = p;
        self[block(q)].rlink = p;
    }

    // --- node constructors ---

    /// An empty vlist, WEB's `new_null_box` retyped to `vlist_node`.
    pub fn new_null_vbox(&mut self) -> Result<NodePtr<BoxNode>, Abort> {
        let b = self.new_null_box()?;
        self[b].orientation = Orientation::Vertical.into();
        Ok(b)
    }

    pub fn new_null_box(&mut self) -> Result<NodePtr<BoxNode>, Abort> {
        let p = NodePtr::<BoxNode>::alloc(self)?;
        let node = &mut self[p];
        node.set_lr(BoxLr::Normal);
        node.width = Scaled::ZERO;
        node.depth = Scaled::ZERO;
        node.height = Scaled::ZERO;
        node.shift_amount = Scaled::ZERO;
        node.list_ptr = MemPtr::NULL;
        node.glue_sign = GlueSign::Normal.into();
        node.glue_order = GlueOrder::Normal.into();
        node.glue_set = 0.0;
        Ok(p)
    }

    /// A rule with running dimensions.
    pub fn new_rule(&mut self) -> Result<NodePtr<RuleNode>, Abort> {
        let p = NodePtr::<RuleNode>::alloc(self)?;
        let node = &mut self[p];
        node.width = RuleDimen::Running.into();
        node.depth = RuleDimen::Running.into();
        node.height = RuleDimen::Running.into();
        Ok(p)
    }

    /// A character node for `c` in font `f`.
    #[inline]
    pub fn new_char(&mut self, f: FontId, c: u16) -> Result<NodePtr<CharNode>, Abort> {
        let p = NodePtr::<CharNode>::at(self.get_avail()?);
        self[p].font = f;
        self[p].character = c;
        Ok(p)
    }

    pub fn new_ligature(
        &mut self,
        f: FontId,
        c: u16,
        q: MemPtr,
    ) -> Result<NodePtr<LigatureNode>, Abort> {
        let p = NodePtr::<LigatureNode>::alloc(self)?;
        let node = &mut self[p];
        node.font = f;
        node.character = c;
        node.lig_ptr = q;
        node.boundary = LigBoundary::None.into();
        Ok(p)
    }

    pub fn new_lig_item(&mut self, c: u16) -> Result<NodePtr<LigItem>, Abort> {
        let p = NodePtr::<LigItem>::alloc(self)?;
        self[p].character = c;
        self[p].component = MemPtr::NULL;
        Ok(p)
    }

    pub fn new_disc(&mut self) -> Result<NodePtr<DiscNode>, Abort> {
        let p = NodePtr::<DiscNode>::alloc(self)?;
        let node = &mut self[p];
        node.replace_count = 0;
        node.pre_break = MemPtr::NULL;
        node.post_break = MemPtr::NULL;
        Ok(p)
    }

    pub fn new_math(&mut self, w: Scaled, code: MathCode) -> Result<NodePtr<MathNode>, Abort> {
        let p = NodePtr::<MathNode>::alloc(self)?;
        self[p].subtype = code.into();
        self[p].width = w;
        Ok(p)
    }

    /// An unshared copy of the glue spec `src`.
    pub fn new_spec(&mut self, src: NodePtr<GlueSpecNode>) -> Result<NodePtr<GlueSpecNode>, Abort> {
        let q = NodePtr::<GlueSpecNode>::alloc(self)?;
        self.set_word(q.idx(), self.word(src.idx()));
        let from = self[src];
        let node = &mut self[q];
        node.ref_count = TEX_NULL;
        node.width = from.width;
        node.stretch = from.stretch;
        node.shrink = from.shrink;
        Ok(q)
    }

    /// Glue for parameter `n`, sharing its spec.
    pub fn new_param_glue(
        &mut self,
        eqtb: &EqtbState,
        n: GluePar,
    ) -> Result<NodePtr<GlueNode>, Abort> {
        let p = NodePtr::<GlueNode>::alloc(self)?;
        self[p].subtype = GlueKind::Param(n).into();
        self[p].leader_ptr = MemPtr::NULL;
        let spec = eqtb.glue_par(n);
        self[p].glue_ptr = spec;
        self.add_glue_ref(spec);
        Ok(p)
    }

    /// Glue sharing the spec `q`.
    pub fn new_glue(&mut self, q: NodePtr<GlueSpecNode>) -> Result<NodePtr<GlueNode>, Abort> {
        let p = NodePtr::<GlueNode>::alloc(self)?;
        let node = &mut self[p];
        node.subtype = GlueKind::Normal.into();
        node.leader_ptr = MemPtr::NULL;
        node.glue_ptr = q;
        self.add_glue_ref(q);
        Ok(p)
    }

    /// Glue for parameter `n` with a private copy of its spec, left in `temp_ptr`.
    pub fn new_skip_param(
        &mut self,
        eqtb: &EqtbState,
        n: GluePar,
    ) -> Result<NodePtr<GlueNode>, Abort> {
        let s = self.new_spec(eqtb.glue_par(n))?;
        self.temp_ptr = s.ptr();
        let p = self.new_glue(s)?;
        self[s].ref_count = TEX_NULL;
        self[p].subtype = GlueKind::Param(n).into();
        Ok(p)
    }

    pub fn new_kern(&mut self, w: Scaled) -> Result<NodePtr<KernNode>, Abort> {
        let p = NodePtr::<KernNode>::alloc(self)?;
        self[p].subtype = KernSubtype::Normal.into();
        self[p].width = w;
        Ok(p)
    }

    pub fn new_penalty(&mut self, m: i32) -> Result<NodePtr<PenaltyNode>, Abort> {
        let p = NodePtr::<PenaltyNode>::alloc(self)?;
        self[p].penalty = m;
        Ok(p)
    }

    pub fn new_noad(&mut self) -> Result<NodePtr<Noad>, Abort> {
        let p = NodePtr::<Noad>::alloc(self)?;
        self[p].clear_fields();
        Ok(p)
    }

    pub fn new_style(&mut self, s: crate::math::MathStyle) -> Result<NodePtr<StyleNode>, Abort> {
        let p = NodePtr::<StyleNode>::alloc(self)?;
        self[p].style = s;
        self[p].clear_payload();
        Ok(p)
    }

    pub fn new_choice(&mut self) -> Result<NodePtr<ChoiceNode>, Abort> {
        let p = NodePtr::<ChoiceNode>::alloc(self)?;
        let node = &mut self[p];
        node.display_mlist = MemPtr::NULL;
        node.text_mlist = MemPtr::NULL;
        node.script_mlist = MemPtr::NULL;
        node.script_script_mlist = MemPtr::NULL;
        Ok(p)
    }

    pub fn new_margin_kern(
        &mut self,
        w: Scaled,
        side: Side,
    ) -> Result<NodePtr<MarginKernNode>, Abort> {
        let k = NodePtr::<MarginKernNode>::alloc(self)?;
        self[k].side = side.into();
        self[k].width = w;
        Ok(k)
    }

    #[inline]
    pub fn add_glue_ref(&mut self, spec: NodePtr<GlueSpecNode>) {
        self[spec].ref_count += 1;
    }

    /// Drops a reference to `spec`, freeing it with the last one.
    pub fn delete_glue_ref(&mut self, spec: NodePtr<GlueSpecNode>) {
        if self[spec].ref_count == TEX_NULL {
            self.free_node(spec.ptr(), GLUE_SPEC_SIZE);
        } else {
            self[spec].ref_count -= 1;
        }
    }
}

// --- the semantic nest ---

pub(crate) fn push_nest(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    st.input
        .nest
        .push(st.input.cur_list)
        .map_err(|_| Abort::NestOverflow)?;
    let h = st.mem.get_avail()?;
    let cl = &mut st.input.cur_list;
    cl.head = h;
    cl.tail = h;
    cl.prev_graf = 0;
    cl.mode_line = st.mem.line;
    cl.e_tex_aux = MemPtr::NULL;
    Ok(())
}

impl MemState {
    pub fn pop_nest(&mut self, input: &mut crate::state::InputClusterState) -> Result<(), Abort> {
        self.free_avail(input.cur_list.head);
        input.cur_list = input.nest.pop().ok_or(Abort::Confusion("pop_nest"))?;
        Ok(())
    }
}
