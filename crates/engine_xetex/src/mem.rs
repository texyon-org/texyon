//! The `mem` arena: the one-word and variable-size free lists, the fixed list
//! heads, and TeX's node constructors.

pub(crate) use crate::consts::TEX_NULL;
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::fonts::Side;
use crate::glue_store::GlueSpecId;
use crate::mem_types::ListStateRecord;
use crate::node::{
    BoxLr, BoxNode, CharNode, ChoiceNode, DiscNode, GlueKind, GlueNode, HeadNode, HyphPosNode,
    KernNode, KernSubtype, LigBoundary, LigItem, LigatureNode, ListRecord, LrNode, MarginKernNode,
    MathCode, MathNode, Noad, Node, NodePtr, PageInsNode, PenaltyNode, RuleDimen, RuleNode,
    SaIndexNode, SpanNode, StyleNode, View, WhatsitKind, WhatsitSubtype,
};
use crate::params::GluePar;
use crate::state::{EqtbState, MemState};
use crate::units::{
    FontId, GlueOrder, GlueSign, MemAddr, MemIdx, MemPtr, NodeType, Orientation, Scaled,
};

pub(crate) use crate::glue_store::{FILL_GLUE, FIL_GLUE, FIL_NEG_GLUE, SS_GLUE, ZERO_GLUE};

/// One more than the largest node size, so a node size indexes its free list.
pub(crate) const FREE_CLASSES: usize = SaIndexNode::SIZE as usize + 1;
/// `mem` may grow to this many words, half of `MAIN_MEMORY_WORDS`.
const MAX_MEM_WORDS: usize = 1 << 25;

/// The free list of each node size, by size.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct FreeLists(pub(crate) [MemPtr; FREE_CLASSES]);

impl Default for FreeLists {
    fn default() -> Self {
        FreeLists([MemPtr::NULL; FREE_CLASSES])
    }
}

/// The fixed node after `p`, which takes `words`.
const fn after(p: MemPtr, words: i32) -> MemPtr {
    MemPtr::from_raw(p.raw() + words)
}

// The fixed nodes after the unused word 0, each as large as the layout it is read as.
pub(crate) const PAGE_INS_HEAD: MemPtr = MemPtr::from_raw(1);
pub(crate) const CONTRIB_HEAD: MemPtr = after(PAGE_INS_HEAD, PageInsNode::SIZE);
pub(crate) const PAGE_HEAD: MemPtr = after(CONTRIB_HEAD, HeadNode::SIZE);
pub(crate) const TEMP_HEAD: MemPtr = after(PAGE_HEAD, GlueNode::SIZE);
pub(crate) const HOLD_HEAD: MemPtr = after(TEMP_HEAD, HeadNode::SIZE);
pub(crate) const ADJUST_HEAD: MemPtr = after(HOLD_HEAD, HeadNode::SIZE);
pub(crate) const ALIGN_HEAD: MemPtr = after(ADJUST_HEAD, HeadNode::SIZE);
const END_SPAN_AT: MemPtr = after(ALIGN_HEAD, HeadNode::SIZE);
// SAFETY: the fixed span node ends every span list.
pub(crate) const END_SPAN: NodePtr<SpanNode> =
    NodePtr::from_idx_unchecked(MemIdx::from_raw(END_SPAN_AT.raw()));
pub(crate) const NULL_LIST: MemPtr = after(END_SPAN_AT, SpanNode::SIZE);
/// The scratch char node of TeX's lig trick.
pub(crate) const LIG_TRICK: MemPtr = after(NULL_LIST, HeadNode::SIZE);
pub(crate) const PRE_ADJUST_HEAD: MemPtr = after(LIG_TRICK, CharNode::SIZE);
/// The fixed nodes laid out as a [`HeadNode`].
pub(crate) const FIXED_HEADS: [MemPtr; 7] = [
    CONTRIB_HEAD,
    TEMP_HEAD,
    HOLD_HEAD,
    ADJUST_HEAD,
    ALIGN_HEAD,
    NULL_LIST,
    PRE_ADJUST_HEAD,
];
/// The words the fixed nodes take, where allocation starts.
pub(crate) const RESERVED_WORDS: usize =
    after(PRE_ADJUST_HEAD, HeadNode::SIZE).raw().cast_unsigned() as usize;

impl ListStateRecord {
    /// A fixed-layout whatsit appended to the current list.
    pub fn append_whatsit<K: WhatsitKind + Node>(
        &mut self,
        mem: &mut MemState,
        subtype: WhatsitSubtype,
    ) -> Result<NodePtr<K>, Abort> {
        let p = NodePtr::<K>::alloc_whatsit(mem, subtype)?;
        self.tail_append(mem, p.ptr());
        Ok(p)
    }
}

// --- lists ---

impl MemState {
    /// Frees the list `p` of `K` records, all of one size.
    pub(crate) fn flush_records<K: ListRecord + Node>(&mut self, mut p: MemPtr) {
        while let Some(r) = NodePtr::<K>::from_ptr(p) {
            p = r.ptr().link(self);
            r.free(self);
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

    /// Whether the non-null node `p` is a character node.
    #[inline]
    pub fn is_char_node(&self, p: impl MemAddr) -> bool {
        let p = p.addr();
        p != TEX_NULL && self.type_code(MemPtr::from_raw(p)) == NodeType::Char.to_u16()
    }

    /// A hyphen position `n` linked in front of the positions `rest`.
    pub(crate) fn new_hyph_pos(&mut self, n: usize, rest: MemPtr) -> Result<MemPtr, Abort> {
        let p = NodePtr::<HyphPosNode>::alloc(self)?;
        let pos = i32::try_from(n).or_confusion("hyphen position")?;
        let node = &mut self[p];
        node.pos = pos;
        node.link = rest;
        Ok(p.ptr())
    }

    /// TeX's `push_LR`.
    #[inline]
    pub(crate) fn push_lr(&mut self, lr_ptr: &mut MemPtr, code: MathCode) -> Result<(), Abort> {
        let t = NodePtr::<LrNode>::alloc(self)?;
        self.temp_ptr = t.ptr();
        let node = &mut self[t];
        node.code = code.into();
        node.link = *lr_ptr;
        *lr_ptr = t.ptr();
        Ok(())
    }

    /// The code in LR stack entry `p`, `None` for anything else.
    #[inline]
    pub(crate) fn lr_code(&self, p: MemPtr) -> Option<MathCode> {
        NodePtr::<LrNode>::from_ptr(p).and_then(|t| self[t].code.get())
    }

    /// TeX's `pop_LR`.
    #[inline]
    pub(crate) fn pop_lr(&mut self, lr_ptr: &mut MemPtr) {
        let t = *lr_ptr;
        self.temp_ptr = t;
        *lr_ptr = t.link(self);
        if let Some(t) = NodePtr::<LrNode>::from_ptr(t) {
            t.free(self);
        }
    }

    // --- the node allocator ---

    /// Whether `p` points into the allocated part of `mem`, for printing a possibly clobbered list.
    pub(crate) fn contains(&self, p: MemPtr) -> bool {
        usize::try_from(p.raw()).is_ok_and(|i| (1..self.data.len()).contains(&i))
    }

    /// TeX's `get_node`: a node of `s` words, from its size's free list or fresh from the end.
    pub fn get_node(&mut self, s: i32) -> Result<MemPtr, Abort> {
        let class = usize::try_from(s)
            .ok()
            .filter(|c| (1..FREE_CLASSES).contains(c))
            .or_confusion("node size")?;
        let head = self.free.0[class];
        let r = if head.is_null() {
            let start = self.data.len();
            let end = start + class;
            if end > MAX_MEM_WORDS {
                return Err(Abort::NODE_OVERFLOW);
            }
            let r = i32::try_from(start).map_err(|_| Abort::NODE_OVERFLOW)?;
            self.data.resize(end, 0);
            MemPtr::from_raw(r)
        } else {
            self.free.0[class] = head.link(self);
            head
        };
        r.set_link(self, MemPtr::NULL);
        Ok(r)
    }

    /// Returns the `s`-word node `p` to its size's free list.
    pub fn free_node(&mut self, p: MemPtr, s: i32) {
        let class = usize::try_from(s)
            .ok()
            .filter(|c| (1..FREE_CLASSES).contains(c));
        debug_assert!(class.is_some(), "freeing a node of size {s}");
        if let Some(class) = class {
            p.set_link(self, self.free.0[class]);
            self.free.0[class] = p;
        }
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
        node.glue_set = crate::node::GlueRatio::new(0.0);
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
        let p = NodePtr::<CharNode>::alloc(self)?;
        let node = &mut self[p];
        node.link = MemPtr::NULL;
        node.font = f;
        node.character = c;
        Ok(p)
    }

    /// TeX's lig trick: the scratch char node holding `lig`'s character, linked to what follows `lig`.
    pub fn lig_trick(&mut self, lig: NodePtr<LigatureNode>) -> MemPtr {
        let scratch = NodePtr::<CharNode>::at(LIG_TRICK);
        let LigatureNode {
            font,
            character,
            link,
            ..
        } = self[lig];
        self.set_header(LIG_TRICK, NodeType::Char, character);
        let node = &mut self[scratch];
        node.font = font;
        node.link = link;
        LIG_TRICK
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
        let node = &mut self[p];
        node.character = c;
        node.component = MemPtr::NULL;
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
        let node = &mut self[p];
        node.subtype = code.into();
        node.width = w;
        Ok(p)
    }

    /// An unshared copy of the glue spec `src`.
    pub fn new_spec(&mut self, src: GlueSpecId) -> Result<GlueSpecId, Abort> {
        let q = self.glue.alloc()?;
        let from = self[src];
        let node = &mut self[q];
        node.stretch_order = from.stretch_order;
        node.shrink_order = from.shrink_order;
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
        let spec = eqtb.glue_par(n);
        let node = &mut self[p];
        node.subtype = GlueKind::Param(n).into();
        node.leader_ptr = MemPtr::NULL;
        node.glue_ptr = spec;
        self.add_glue_ref(spec);
        Ok(p)
    }

    /// Glue sharing the spec `q`.
    pub fn new_glue(&mut self, q: GlueSpecId) -> Result<NodePtr<GlueNode>, Abort> {
        let p = NodePtr::<GlueNode>::alloc(self)?;
        let node = &mut self[p];
        node.subtype = GlueKind::Normal.into();
        node.leader_ptr = MemPtr::NULL;
        node.glue_ptr = q;
        self.add_glue_ref(q);
        Ok(p)
    }

    /// Glue for parameter `n` with a private copy of its spec.
    pub fn new_skip_param(
        &mut self,
        eqtb: &EqtbState,
        n: GluePar,
    ) -> Result<NodePtr<GlueNode>, Abort> {
        let s = self.new_spec(eqtb.glue_par(n))?;
        let p = self.new_glue(s)?;
        self[s].ref_count = TEX_NULL;
        self[p].subtype = GlueKind::Param(n).into();
        Ok(p)
    }

    pub fn new_kern(&mut self, w: Scaled) -> Result<NodePtr<KernNode>, Abort> {
        let p = NodePtr::<KernNode>::alloc(self)?;
        let node = &mut self[p];
        node.subtype = KernSubtype::Normal.into();
        node.width = w;
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
        let node = &mut self[p];
        node.style = s;
        node.clear_payload();
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
        let node = &mut self[k];
        node.side = side.into();
        node.width = w;
        Ok(k)
    }

    #[inline]
    pub fn add_glue_ref(&mut self, spec: GlueSpecId) {
        self[spec].ref_count += 1;
    }

    /// Drops a reference to `spec`, freeing it with the last one.
    pub fn delete_glue_ref(&mut self, spec: GlueSpecId) {
        if self[spec].ref_count == TEX_NULL {
            self.glue.free(spec);
        } else {
            self[spec].ref_count -= 1;
        }
    }
}

// --- the semantic nest ---

pub(crate) fn push_nest(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    st.input.nest.push(st.input.cur_list).map_err(|_| {
        Abort::overflow(crate::errors::Capacity::SemanticNest, st.input.nest.size())
    })?;
    let h = NodePtr::<HeadNode>::alloc(&mut st.mem)?.ptr();
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
        NodePtr::<HeadNode>::at(input.cur_list.head).free(self);
        input.cur_list = input.nest.pop().or_confusion("pop_nest")?;
        Ok(())
    }
}
