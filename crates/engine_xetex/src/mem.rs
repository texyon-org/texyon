//! The `mem` arena: the one-word and variable-size free lists, the fixed list
//! heads, and TeX's node constructors.

pub(crate) use crate::consts::TEX_NULL;
use crate::consts::{
    END_M_CODE, GLUE_SPEC_SIZE, L_CODE, MAX_HALFWORD, MEDIUM_NODE_SIZE, MEM_TOP, NORMAL,
};
use crate::mem_types::ListStateRecord;
use crate::node::{
    CharNode, ChoiceNode, DiscNode, FreeBlock, GlueNode, GlueSpecNode, HListNode, KernNode,
    LigItem, LigatureNode, MarginKernNode, MathNode, Noad, NodePtr, PenaltyNode, RuleDimen,
    RuleNode, SpanNode, StyleNode, WhatsitNode, WhatsitSubtype,
};
use crate::state::{EqtbState, MemState};
use crate::units::{FontId, GlueSign, MemAddr, MemIdx, MemPtr, Scaled};

pub(crate) const ZERO_GLUE: NodePtr<GlueSpecNode> = static_spec(0);
pub(crate) const FIL_GLUE: NodePtr<GlueSpecNode> = static_spec(1);
pub(crate) const FILL_GLUE: NodePtr<GlueSpecNode> = static_spec(2);
pub(crate) const SS_GLUE: NodePtr<GlueSpecNode> = static_spec(3);
pub(crate) const FIL_NEG_GLUE: NodePtr<GlueSpecNode> = static_spec(4);

/// The `k`th of the glue specs that INITEX builds at the bottom of `mem`.
const fn static_spec(k: i32) -> NodePtr<GlueSpecNode> {
    // SAFETY: those words hold glue specs for the whole run.
    unsafe { NodePtr::from_idx_unchecked(MemIdx(k * GLUE_SPEC_SIZE)) }
}

pub(crate) const PAGE_INS_HEAD: MemPtr = MemPtr(MEM_TOP);
pub(crate) const CONTRIB_HEAD: MemPtr = MemPtr(MEM_TOP - 1);
pub(crate) const PAGE_HEAD: MemPtr = MemPtr(MEM_TOP - 2);
pub(crate) const TEMP_HEAD: MemPtr = MemPtr(MEM_TOP - 3);
pub(crate) const HOLD_HEAD: MemPtr = MemPtr(MEM_TOP - 4);
pub(crate) const ADJUST_HEAD: MemPtr = MemPtr(MEM_TOP - 5);
pub(crate) const ACTIVE_LIST: MemPtr = MemPtr(MEM_TOP - 7);
pub(crate) const ALIGN_HEAD: MemPtr = MemPtr(MEM_TOP - 8);
// SAFETY: the static word at `MEM_TOP - 9` ends every span list.
pub(crate) const END_SPAN: NodePtr<SpanNode> =
    unsafe { NodePtr::from_idx_unchecked(MemIdx(MEM_TOP - 9)) };
pub(crate) const OMIT_TEMPLATE: MemPtr = MemPtr(MEM_TOP - 10);
pub(crate) const NULL_LIST: MemPtr = MemPtr(MEM_TOP - 11);
pub(crate) const LIG_TRICK: MemPtr = MemPtr(MEM_TOP - 12);
pub(crate) const GARBAGE: MemPtr = MemPtr(MEM_TOP - 12);
pub(crate) const BACKUP_HEAD: MemPtr = MemPtr(MEM_TOP - 13);
pub(crate) const PRE_ADJUST_HEAD: MemPtr = MemPtr(MEM_TOP - 14);
pub(crate) const HI_MEM_STAT_MIN: i32 = MEM_TOP - 14;
pub(crate) const HI_MEM_STAT_USAGE: i32 = 15;

/// `malloc` that panics instead of returning null, for C-owned buffers.
///
/// # Safety
/// The caller owns the uninitialised result and frees it with `libc::free`.
pub(crate) unsafe fn xmalloc(size: libc::size_t) -> *mut libc::c_void {
    let p = libc::malloc(size.max(1));
    assert!(!p.is_null(), "out of memory: malloc({size} bytes) failed");
    p
}

/// `calloc` that panics instead of returning null, for C-owned buffers.
///
/// # Safety
/// The caller owns the zeroed result and frees it with `libc::free`.
pub(crate) unsafe fn xcalloc(nelem: libc::size_t, elsize: libc::size_t) -> *mut libc::c_void {
    let p = libc::calloc(nelem.max(1), elsize.max(1));
    assert!(
        !p.is_null(),
        "out of memory: calloc({nelem}, {elsize}) failed"
    );
    p
}

// --- the one-word free list ---

impl MemState {
    /// TeX's `get_avail`, `None` when main memory is exhausted.
    #[inline(always)]
    pub fn get_avail(&mut self) -> Option<MemPtr> {
        let p = self.avail;
        if p != TEX_NULL {
            self.avail = self.b32s1(p);
            self.set_b32s1(p, TEX_NULL);
            Some(MemPtr(p))
        } else {
            self.get_avail_grow()
        }
    }

    #[cold]
    fn get_avail_grow(&mut self) -> Option<MemPtr> {
        let p = if self.mem_end < MEM_TOP {
            self.mem_end += 1;
            self.mem_end
        } else {
            self.hi_mem_min -= 1;
            if self.lo_mem_max >= self.hi_mem_min {
                return None;
            }
            self.hi_mem_min
        };
        self.set_b32s1(p, TEX_NULL);
        Some(MemPtr(p))
    }

    /// Returns the one-word node `p` to the free list.
    #[inline]
    pub fn free_avail(&mut self, p: MemPtr) {
        p.set_link(self, MemPtr(self.avail));
        self.avail = p.0;
    }

    /// Returns the one-word list `p` to the free list.
    pub fn flush_list(&mut self, p: MemPtr) {
        if p.is_null() {
            return;
        }
        self.list_tail(p).set_link(self, MemPtr(self.avail));
        self.avail = p.0;
    }

    /// TeX's `store_new_token`: appends `tok` after `tail` and makes it the tail,
    /// `None` when main memory is exhausted.
    #[inline(always)]
    pub(crate) fn store_new_token(&mut self, tail: &mut MemPtr, tok: i32) -> Option<()> {
        let q = self.get_avail()?;
        tail.set_link(self, q);
        q.set_token(self, tok);
        *tail = q;
        Some(())
    }

    #[inline]
    pub(crate) fn add_token_ref(&mut self, p: MemPtr) {
        p.set_token_ref_count(self, p.token_ref_count(self) + 1);
    }

    /// Drops a reference to token list `p`, freeing it with the last one.
    pub fn delete_token_ref(&mut self, p: MemPtr) {
        if p.token_ref_count(self) == TEX_NULL {
            self.flush_list(p);
        } else {
            p.set_token_ref_count(self, p.token_ref_count(self) - 1);
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
        p.addr() >= self.hi_mem_min
    }
}

/// TeX's `end_LR_type`: the LR entry that a math node of `subtype` closes.
#[inline]
pub(crate) fn end_lr_type(subtype: u16) -> i32 {
    (L_CODE * (subtype / L_CODE) + END_M_CODE) as i32
}

impl MemState {
    /// TeX's `push_LR`, `None` when main memory is exhausted.
    #[inline]
    pub(crate) fn push_lr(&mut self, lr_ptr: &mut MemPtr, info: i32) -> Option<()> {
        let t = self.get_avail()?;
        self.temp_ptr = t;
        t.set_info(self, info);
        t.set_link(self, *lr_ptr);
        *lr_ptr = t;
        Some(())
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

    /// TeX's `get_node`, `None` when main memory is exhausted. The request
    /// `0x4000_0000` is `sort_avail`'s probe and yields `MAX_HALFWORD`.
    pub fn get_node(&mut self, s: i32) -> Option<MemPtr> {
        let block = |p: i32| NodePtr::<FreeBlock>::at(p);
        let r = 'restart: loop {
            let mut p = self.rover;
            loop {
                // Merge the free blocks that follow `p`.
                let mut q = p + block(p).node_size(self);
                while block(q).is_empty(self) {
                    let t = block(q).rlink(self);
                    if q == self.rover {
                        self.rover = t;
                    }
                    let q_left = block(q).llink(self);
                    block(t).set_llink(self, q_left);
                    block(q_left).set_rlink(self, t);
                    q += block(q).node_size(self);
                }

                let r = q - s;
                if r > p + 1 {
                    block(p).set_node_size(self, r - p);
                    self.rover = p;
                    break 'restart r;
                }
                if r == p && block(p).rlink(self) != p {
                    let new_rover = block(p).rlink(self);
                    let t = block(p).llink(self);
                    self.rover = new_rover;
                    block(new_rover).set_llink(self, t);
                    block(t).set_rlink(self, new_rover);
                    break 'restart r;
                }

                block(p).set_node_size(self, q - p);
                p = block(p).rlink(self);
                if p == self.rover {
                    break;
                }
            }

            if s == 0x4000_0000 {
                return Some(MemPtr(MAX_HALFWORD));
            }

            if self.lo_mem_max + 2 < self.hi_mem_min && self.lo_mem_max + 2 <= MAX_HALFWORD {
                // Grow lo-mem by a new free block below `hi_mem_min`.
                let t = if self.hi_mem_min - self.lo_mem_max >= 1998 {
                    self.lo_mem_max + 1000
                } else {
                    self.lo_mem_max + 1 + (self.hi_mem_min - self.lo_mem_max) / 2
                };
                let t = t.min(MAX_HALFWORD);

                let rover = self.rover;
                let left_neighbor = block(rover).llink(self);
                let q = self.lo_mem_max;
                block(left_neighbor).set_rlink(self, q);
                block(rover).set_llink(self, q);
                block(q).set_rlink(self, rover);
                block(q).set_llink(self, left_neighbor);
                block(q).mark_empty(self);
                block(q).set_node_size(self, t - self.lo_mem_max);
                self.lo_mem_max = t;
                let end = MemPtr(t);
                end.set_link(self, MemPtr::NULL);
                end.set_info(self, TEX_NULL);
                self.rover = q;
                continue 'restart;
            }

            return None;
        };

        MemPtr(r).set_link(self, MemPtr::NULL);
        if s >= MEDIUM_NODE_SIZE {
            self.set_b32s0(r + s - 1, self.cur_synctex_tag);
            self.set_b32s1(r + s - 1, self.line);
        }
        Some(MemPtr(r))
    }

    /// Returns the `s`-word node `p` to the variable-size free list.
    pub fn free_node(&mut self, p: MemPtr, s: i32) {
        let block = |p: i32| NodePtr::<FreeBlock>::at(p);
        let freed = block(p.0);
        freed.set_node_size(self, s);
        freed.mark_empty(self);
        let rover = self.rover;
        let q = block(rover).llink(self);
        freed.set_llink(self, q);
        freed.set_rlink(self, rover);
        block(rover).set_llink(self, p.0);
        block(q).set_rlink(self, p.0);
    }

    // --- node constructors ---

    pub fn new_null_box(&mut self) -> Option<NodePtr<HListNode>> {
        let p = NodePtr::<HListNode>::alloc(self)?;
        p.set_subtype(self, 0);
        p.set_width(self, Scaled(0));
        p.set_depth(self, Scaled(0));
        p.set_height(self, Scaled(0));
        p.set_shift_amount(self, Scaled(0));
        p.set_list_ptr(self, MemPtr::NULL);
        p.set_glue_sign(self, GlueSign::Normal);
        p.set_glue_order(self, NORMAL);
        p.set_glue_set(self, 0.0);
        Some(p)
    }

    /// A rule with running dimensions.
    pub fn new_rule(&mut self) -> Option<NodePtr<RuleNode>> {
        let p = NodePtr::<RuleNode>::alloc(self)?;
        p.set_subtype(self, 0);
        p.set_width(self, RuleDimen::Running);
        p.set_depth(self, RuleDimen::Running);
        p.set_height(self, RuleDimen::Running);
        Some(p)
    }

    /// A character node for `c` in font `f`.
    #[inline]
    pub fn new_char(&mut self, f: FontId, c: u16) -> Option<NodePtr<CharNode>> {
        let p = NodePtr::<CharNode>::at(self.get_avail()?);
        p.set_font(self, f);
        p.set_character(self, c);
        Some(p)
    }

    pub fn new_ligature(&mut self, f: FontId, c: u16, q: MemPtr) -> Option<NodePtr<LigatureNode>> {
        let p = NodePtr::<LigatureNode>::alloc(self)?;
        p.set_font(self, f);
        p.set_character(self, c);
        p.set_lig_ptr(self, q);
        p.set_subtype(self, 0);
        Some(p)
    }

    pub fn new_lig_item(&mut self, c: u16) -> Option<NodePtr<LigItem>> {
        let p = NodePtr::<LigItem>::alloc(self)?;
        p.set_character(self, c);
        p.set_component(self, MemPtr::NULL);
        Some(p)
    }

    pub fn new_disc(&mut self) -> Option<NodePtr<DiscNode>> {
        let p = NodePtr::<DiscNode>::alloc(self)?;
        p.set_subtype(self, 0);
        p.set_pre_break(self, MemPtr::NULL);
        p.set_post_break(self, MemPtr::NULL);
        Some(p)
    }

    pub fn new_math(&mut self, w: i32, s: u16) -> Option<NodePtr<MathNode>> {
        let p = NodePtr::<MathNode>::alloc(self)?;
        p.set_subtype(self, s);
        p.set_width(self, Scaled(w));
        Some(p)
    }

    /// An unshared copy of the glue spec `src`.
    pub fn new_spec(&mut self, src: NodePtr<GlueSpecNode>) -> Option<NodePtr<GlueSpecNode>> {
        let q = NodePtr::<GlueSpecNode>::alloc(self)?;
        self.set_word(q.idx().0, self.word(src.idx().0));
        q.set_ref_count(self, TEX_NULL);
        q.set_width(self, src.width(self));
        q.set_stretch(self, src.stretch(self));
        q.set_shrink(self, src.shrink(self));
        Some(q)
    }

    /// Glue for parameter `n`, sharing its spec.
    pub fn new_param_glue(&mut self, eqtb: &EqtbState, n: i16) -> Option<NodePtr<GlueNode>> {
        let p = NodePtr::<GlueNode>::alloc(self)?;
        p.set_subtype(self, n as u16 + 1);
        p.set_leader_ptr(self, MemPtr::NULL);
        let spec = eqtb.glue_par(n);
        p.set_glue_ptr(self, spec);
        self.add_glue_ref(spec);
        Some(p)
    }

    /// Glue sharing the spec `q`.
    pub fn new_glue(&mut self, q: NodePtr<GlueSpecNode>) -> Option<NodePtr<GlueNode>> {
        let p = NodePtr::<GlueNode>::alloc(self)?;
        p.set_subtype(self, NORMAL);
        p.set_leader_ptr(self, MemPtr::NULL);
        p.set_glue_ptr(self, q);
        self.add_glue_ref(q);
        Some(p)
    }

    /// Glue for parameter `n` with a private copy of its spec, left in `temp_ptr`.
    pub fn new_skip_param(&mut self, eqtb: &EqtbState, n: i16) -> Option<NodePtr<GlueNode>> {
        let s = self.new_spec(eqtb.glue_par(n))?;
        self.temp_ptr = s.ptr();
        let p = self.new_glue(s)?;
        s.set_ref_count(self, TEX_NULL);
        p.set_subtype(self, n as u16 + 1);
        Some(p)
    }

    pub fn new_kern(&mut self, w: i32) -> Option<NodePtr<KernNode>> {
        let p = NodePtr::<KernNode>::alloc(self)?;
        p.set_subtype(self, NORMAL);
        p.set_width(self, Scaled(w));
        Some(p)
    }

    pub fn new_penalty(&mut self, m: i32) -> Option<NodePtr<PenaltyNode>> {
        let p = NodePtr::<PenaltyNode>::alloc(self)?;
        p.set_subtype(self, 0);
        p.set_penalty(self, m);
        Some(p)
    }

    pub fn new_noad(&mut self) -> Option<NodePtr<Noad>> {
        let p = NodePtr::<Noad>::alloc(self)?;
        p.clear_fields(self);
        Some(p)
    }

    pub fn new_style(&mut self, s: i16) -> Option<NodePtr<StyleNode>> {
        let p = NodePtr::<StyleNode>::alloc(self)?;
        p.set_subtype(self, s as u16);
        p.clear_payload(self);
        Some(p)
    }

    pub fn new_choice(&mut self) -> Option<NodePtr<ChoiceNode>> {
        let p = NodePtr::<ChoiceNode>::alloc(self)?;
        p.set_subtype(self, 0);
        p.set_display_mlist(self, MemPtr::NULL);
        p.set_text_mlist(self, MemPtr::NULL);
        p.set_script_mlist(self, MemPtr::NULL);
        p.set_script_script_mlist(self, MemPtr::NULL);
        Some(p)
    }

    pub fn new_margin_kern(&mut self, w: i32, side: i16) -> Option<NodePtr<MarginKernNode>> {
        let k = NodePtr::<MarginKernNode>::alloc(self)?;
        k.set_subtype(self, side as u16);
        k.set_width(self, Scaled(w));
        Some(k)
    }
}

impl ListStateRecord {
    /// A `size`-word whatsit appended to the current list.
    pub fn new_whatsit(
        &mut self,
        mem: &mut MemState,
        subtype: WhatsitSubtype,
        size: i32,
    ) -> Option<NodePtr<WhatsitNode>> {
        let p = NodePtr::<WhatsitNode>::alloc(mem, size, subtype)?;
        self.tail_append(mem, p.ptr());
        Some(p)
    }
}

// --- glue spec references ---

impl MemState {
    #[inline]
    pub fn add_glue_ref(&mut self, spec: NodePtr<GlueSpecNode>) {
        spec.set_ref_count(self, spec.ref_count(self) + 1);
    }

    /// Drops a reference to `spec`, freeing it with the last one.
    pub fn delete_glue_ref(&mut self, spec: NodePtr<GlueSpecNode>) {
        if spec.ref_count(self) == TEX_NULL {
            self.free_node(spec.ptr(), GLUE_SPEC_SIZE);
        } else {
            spec.set_ref_count(self, spec.ref_count(self) - 1);
        }
    }
}

// --- the semantic nest ---

pub(crate) enum PushNestAbort {
    SemanticNest,
    MainMemory,
}

impl PushNestAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            PushNestAbort::SemanticNest => crate::errors::EngineAbort::NestOverflow,
            PushNestAbort::MainMemory => crate::errors::EngineAbort::MainMemoryOverflow,
        }
    }
}

pub(crate) fn push_nest(st: &mut crate::state::EngineState) -> Result<(), PushNestAbort> {
    let np = st.input.nest_ptr;
    if np > st.input.max_nest_stack {
        st.input.max_nest_stack = np;
        if np == st.input.nest_size {
            return Err(PushNestAbort::SemanticNest);
        }
    }
    st.input.nest[np as usize] = st.input.cur_list;
    st.input.nest_ptr = np + 1;
    let h = st.mem.get_avail().ok_or(PushNestAbort::MainMemory)?;
    let cl = &mut st.input.cur_list;
    cl.head = h;
    cl.tail = h;
    cl.prev_graf = 0;
    cl.mode_line = st.mem.line;
    cl.e_tex_aux = MemPtr::NULL;
    Ok(())
}

impl MemState {
    pub fn pop_nest(&mut self, input: &mut crate::state::InputClusterState) {
        self.free_avail(input.cur_list.head);
        input.nest_ptr -= 1;
        input.cur_list = input.nest[input.nest_ptr as usize];
    }
}
