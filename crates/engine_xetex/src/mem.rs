//! The `mem` arena: the one-word and variable-size free lists, the fixed list
//! heads, and TeX's node constructors.

pub(crate) use crate::consts::TEX_NULL;
use crate::consts::{
    END_M_CODE, GLUE_SPEC_SIZE, L_CODE, MAX_HALFWORD, MEDIUM_NODE_SIZE, MEM_TOP, NORMAL,
};
use crate::mem_types::ListStateRecord;
use crate::node::{
    ChoiceNode, DiscNode, GlueNode, GlueSpecNode, HListNode, KernNode, LigItem, LigatureNode,
    MarginKernNode, MathNode, Noad, NodePtr, PenaltyNode, RuleDimen, RuleNode, StyleNode,
    WhatsitNode, WhatsitSubtype,
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
pub(crate) const END_SPAN: MemPtr = MemPtr(MEM_TOP - 9);
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

/// TeX's `get_avail`, `None` when main memory is exhausted.
#[inline(always)]
pub fn get_avail(mem: &mut MemState) -> Option<MemPtr> {
    let p = mem.avail;
    if p != TEX_NULL {
        mem.avail = mem.b32s1(p);
        mem.set_b32s1(p, TEX_NULL);
        Some(MemPtr(p))
    } else {
        get_avail_grow(mem)
    }
}

#[cold]
fn get_avail_grow(mem: &mut MemState) -> Option<MemPtr> {
    let p = if mem.mem_end < MEM_TOP {
        mem.mem_end += 1;
        mem.mem_end
    } else {
        mem.hi_mem_min -= 1;
        if mem.lo_mem_max >= mem.hi_mem_min {
            return None;
        }
        mem.hi_mem_min
    };
    mem.set_b32s1(p, TEX_NULL);
    Some(MemPtr(p))
}

/// Returns the one-word node `p` to the free list.
#[inline]
pub fn free_avail(mem: &mut MemState, p: MemPtr) {
    p.set_link(mem, MemPtr(mem.avail));
    mem.avail = p.0;
}

/// Returns the one-word list `p` to the free list.
pub fn flush_list(mem: &mut MemState, p: MemPtr) {
    if p.is_null() {
        return;
    }
    list_tail(mem, p).set_link(mem, MemPtr(mem.avail));
    mem.avail = p.0;
}

#[inline]
pub(crate) fn add_token_ref(mem: &mut MemState, p: MemPtr) {
    p.set_info(mem, p.info(mem) + 1);
}

/// Drops a reference to token list `p`, freeing it with the last one.
pub fn delete_token_ref(mem: &mut MemState, p: MemPtr) {
    if p.info(mem) == TEX_NULL {
        flush_list(mem, p);
    } else {
        p.set_info(mem, p.info(mem) - 1);
    }
}

/// The last node of the non-empty list starting at `p`.
#[inline]
pub(crate) fn list_tail(mem: &MemState, mut p: MemPtr) -> MemPtr {
    while !p.link(mem).is_null() {
        p = p.link(mem);
    }
    p
}

/// The node whose link is `target`, searching forward from `p`.
#[inline]
pub(crate) fn predecessor(mem: &MemState, mut p: MemPtr, target: MemPtr) -> MemPtr {
    while p.link(mem) != target {
        p = p.link(mem);
    }
    p
}

/// The node whose link is `e`, searching forward from `s`, or null if the
/// list ends first.
pub fn prev_rightmost(mem: &MemState, s: MemPtr, e: MemPtr) -> MemPtr {
    let mut p = s;
    if p.is_null() {
        return MemPtr::NULL;
    }
    while p.link(mem) != e {
        p = p.link(mem);
        if p.is_null() {
            return MemPtr::NULL;
        }
    }
    p
}

/// Skips the nodes that may sit between the words of an interword space.
pub(crate) fn skip_interword_fillers(mem: &MemState, mut p: MemPtr) -> MemPtr {
    while mem
        .classify_non_char(p)
        .is_some_and(|n| n.is_interword_filler(mem))
    {
        p = p.link(mem);
    }
    p
}

/// Whether `p` lies in the one-word region, where every node is a character.
#[inline]
pub fn is_char_node(mem: &MemState, p: impl MemAddr) -> bool {
    p.addr() >= mem.hi_mem_min
}

/// TeX's `end_LR_type`: the LR entry that a math node of `subtype` closes.
#[inline]
pub(crate) fn end_lr_type(subtype: u16) -> i32 {
    (L_CODE * (subtype / L_CODE) + END_M_CODE) as i32
}

/// TeX's `push_LR`, `None` when main memory is exhausted.
#[inline]
pub(crate) fn push_lr(mem: &mut MemState, lr_ptr: &mut MemPtr, info: i32) -> Option<()> {
    let t = get_avail(mem)?;
    mem.temp_ptr = t;
    t.set_info(mem, info);
    t.set_link(mem, *lr_ptr);
    *lr_ptr = t;
    Some(())
}

/// TeX's `pop_LR`.
#[inline]
pub(crate) fn pop_lr(mem: &mut MemState, lr_ptr: &mut MemPtr) {
    let t = *lr_ptr;
    mem.temp_ptr = t;
    *lr_ptr = t.link(mem);
    free_avail(mem, t);
}

// --- the variable-size free list ---

/// TeX's `get_node`, `None` when main memory is exhausted. The request
/// `0x4000_0000` is `sort_avail`'s probe and yields `MAX_HALFWORD`.
pub fn get_node(mem: &mut MemState, s: i32) -> Option<MemPtr> {
    let r = 'restart: loop {
        let mut p = mem.rover;
        loop {
            // Merge the free blocks that follow `p`.
            let mut q = p + mem.b32s0(p);
            while mem.b32s1(q) == MAX_HALFWORD {
                let t = mem.b32s1(q + 1);
                if q == mem.rover {
                    mem.rover = t;
                }
                let q_left = mem.b32s0(q + 1);
                mem.set_b32s0(t + 1, q_left);
                mem.set_b32s1(q_left + 1, t);
                q += mem.b32s0(q);
            }

            let r = q - s;
            if r > p + 1 {
                mem.set_b32s0(p, r - p);
                mem.rover = p;
                break 'restart r;
            }
            if r == p && mem.b32s1(p + 1) != p {
                let new_rover = mem.b32s1(p + 1);
                let t = mem.b32s0(p + 1);
                mem.rover = new_rover;
                mem.set_b32s0(new_rover + 1, t);
                mem.set_b32s1(t + 1, new_rover);
                break 'restart r;
            }

            mem.set_b32s0(p, q - p);
            p = mem.b32s1(p + 1);
            if p == mem.rover {
                break;
            }
        }

        if s == 0x4000_0000 {
            return Some(MemPtr(MAX_HALFWORD));
        }

        if mem.lo_mem_max + 2 < mem.hi_mem_min && mem.lo_mem_max + 2 <= MAX_HALFWORD {
            // Grow lo-mem by a new free block below `hi_mem_min`.
            let t = if mem.hi_mem_min - mem.lo_mem_max >= 1998 {
                mem.lo_mem_max + 1000
            } else {
                mem.lo_mem_max + 1 + (mem.hi_mem_min - mem.lo_mem_max) / 2
            };
            let t = t.min(MAX_HALFWORD);

            let rover = mem.rover;
            let left_neighbor = mem.b32s0(rover + 1);
            let q = mem.lo_mem_max;
            mem.set_b32s1(left_neighbor + 1, q);
            mem.set_b32s0(rover + 1, q);
            mem.set_b32s1(q + 1, rover);
            mem.set_b32s0(q + 1, left_neighbor);
            mem.set_b32s1(q, MAX_HALFWORD);
            mem.set_b32s0(q, t - mem.lo_mem_max);
            mem.lo_mem_max = t;
            mem.set_b32s1(mem.lo_mem_max, TEX_NULL);
            mem.set_b32s0(mem.lo_mem_max, TEX_NULL);
            mem.rover = q;
            continue 'restart;
        }

        return None;
    };

    mem.set_b32s1(r, TEX_NULL);
    if s >= MEDIUM_NODE_SIZE {
        mem.set_b32s0(r + s - 1, mem.cur_synctex_tag);
        mem.set_b32s1(r + s - 1, mem.line);
    }
    Some(MemPtr(r))
}

/// Returns the `s`-word node `p` to the variable-size free list.
pub fn free_node(mem: &mut MemState, p: MemPtr, s: i32) {
    mem.set_b32s0(p, s);
    mem.set_b32s1(p, MAX_HALFWORD);
    let rover = mem.rover;
    let q = mem.b32s0(rover + 1);
    mem.set_b32s0(p + 1, q);
    mem.set_b32s1(p + 1, rover);
    mem.set_b32s0(rover + 1, p.0);
    mem.set_b32s1(q + 1, p.0);
}

// --- node constructors ---

pub fn new_null_box(mem: &mut MemState) -> Option<NodePtr<HListNode>> {
    let p = NodePtr::<HListNode>::alloc(mem)?;
    p.set_subtype(mem, 0);
    p.set_width(mem, Scaled(0));
    p.set_depth(mem, Scaled(0));
    p.set_height(mem, Scaled(0));
    p.set_shift_amount(mem, Scaled(0));
    p.set_list_ptr(mem, MemPtr::NULL);
    p.set_glue_sign(mem, GlueSign::Normal);
    p.set_glue_order(mem, NORMAL);
    p.set_glue_set(mem, 0.0);
    Some(p)
}

/// A rule with running dimensions.
pub fn new_rule(mem: &mut MemState) -> Option<NodePtr<RuleNode>> {
    let p = NodePtr::<RuleNode>::alloc(mem)?;
    p.set_subtype(mem, 0);
    p.set_width(mem, RuleDimen::Running);
    p.set_depth(mem, RuleDimen::Running);
    p.set_height(mem, RuleDimen::Running);
    Some(p)
}

pub fn new_ligature(
    mem: &mut MemState,
    f: FontId,
    c: u16,
    q: MemPtr,
) -> Option<NodePtr<LigatureNode>> {
    let p = NodePtr::<LigatureNode>::alloc(mem)?;
    p.set_font(mem, f);
    p.set_character(mem, c);
    p.set_lig_ptr(mem, q);
    p.set_subtype(mem, 0);
    Some(p)
}

pub fn new_lig_item(mem: &mut MemState, c: u16) -> Option<NodePtr<LigItem>> {
    let p = NodePtr::<LigItem>::alloc(mem)?;
    p.set_character(mem, c);
    p.set_component(mem, MemPtr::NULL);
    Some(p)
}

pub fn new_disc(mem: &mut MemState) -> Option<NodePtr<DiscNode>> {
    let p = NodePtr::<DiscNode>::alloc(mem)?;
    p.set_subtype(mem, 0);
    p.set_pre_break(mem, MemPtr::NULL);
    p.set_post_break(mem, MemPtr::NULL);
    Some(p)
}

pub fn new_math(mem: &mut MemState, w: i32, s: u16) -> Option<NodePtr<MathNode>> {
    let p = NodePtr::<MathNode>::alloc(mem)?;
    p.set_subtype(mem, s);
    p.set_width(mem, Scaled(w));
    Some(p)
}

/// An unshared copy of the glue spec `src`.
pub fn new_spec(mem: &mut MemState, src: NodePtr<GlueSpecNode>) -> Option<NodePtr<GlueSpecNode>> {
    let q = NodePtr::<GlueSpecNode>::alloc(mem)?;
    mem.set_word(q.idx().0, mem.word(src.idx().0));
    q.set_ref_count(mem, TEX_NULL);
    q.set_width(mem, src.width(mem));
    q.set_stretch(mem, src.stretch(mem));
    q.set_shrink(mem, src.shrink(mem));
    Some(q)
}

/// Glue for parameter `n`, sharing its spec.
pub fn new_param_glue(mem: &mut MemState, eqtb: &EqtbState, n: i16) -> Option<NodePtr<GlueNode>> {
    let p = NodePtr::<GlueNode>::alloc(mem)?;
    p.set_subtype(mem, n as u16 + 1);
    p.set_leader_ptr(mem, MemPtr::NULL);
    let spec = eqtb.glue_par(n);
    p.set_glue_ptr(mem, spec);
    add_glue_ref(mem, spec);
    Some(p)
}

/// Glue sharing the spec `q`.
pub fn new_glue(mem: &mut MemState, q: NodePtr<GlueSpecNode>) -> Option<NodePtr<GlueNode>> {
    let p = NodePtr::<GlueNode>::alloc(mem)?;
    p.set_subtype(mem, NORMAL);
    p.set_leader_ptr(mem, MemPtr::NULL);
    p.set_glue_ptr(mem, q);
    add_glue_ref(mem, q);
    Some(p)
}

/// Glue for parameter `n` with a private copy of its spec, left in `temp_ptr`.
pub fn new_skip_param(mem: &mut MemState, eqtb: &EqtbState, n: i16) -> Option<NodePtr<GlueNode>> {
    let s = new_spec(mem, eqtb.glue_par(n))?;
    mem.temp_ptr = s.ptr();
    let p = new_glue(mem, s)?;
    s.set_ref_count(mem, TEX_NULL);
    p.set_subtype(mem, n as u16 + 1);
    Some(p)
}

pub fn new_kern(mem: &mut MemState, w: i32) -> Option<NodePtr<KernNode>> {
    let p = NodePtr::<KernNode>::alloc(mem)?;
    p.set_subtype(mem, NORMAL);
    p.set_width(mem, Scaled(w));
    Some(p)
}

pub fn new_penalty(mem: &mut MemState, m: i32) -> Option<NodePtr<PenaltyNode>> {
    let p = NodePtr::<PenaltyNode>::alloc(mem)?;
    p.set_subtype(mem, 0);
    p.set_penalty(mem, m);
    Some(p)
}

pub fn new_noad(mem: &mut MemState) -> Option<NodePtr<Noad>> {
    let p = NodePtr::<Noad>::alloc(mem)?;
    p.clear_fields(mem);
    Some(p)
}

pub fn new_style(mem: &mut MemState, s: i16) -> Option<NodePtr<StyleNode>> {
    let p = NodePtr::<StyleNode>::alloc(mem)?;
    p.set_subtype(mem, s as u16);
    p.clear_payload(mem);
    Some(p)
}

pub fn new_choice(mem: &mut MemState) -> Option<NodePtr<ChoiceNode>> {
    let p = NodePtr::<ChoiceNode>::alloc(mem)?;
    p.set_subtype(mem, 0);
    p.set_display_mlist(mem, MemPtr::NULL);
    p.set_text_mlist(mem, MemPtr::NULL);
    p.set_script_mlist(mem, MemPtr::NULL);
    p.set_script_script_mlist(mem, MemPtr::NULL);
    Some(p)
}

pub fn new_margin_kern(mem: &mut MemState, w: i32, side: i16) -> Option<NodePtr<MarginKernNode>> {
    let k = NodePtr::<MarginKernNode>::alloc(mem)?;
    k.set_subtype(mem, side as u16);
    k.set_width(mem, Scaled(w));
    Some(k)
}

/// A `size`-word whatsit appended to the current list.
pub fn new_whatsit(
    cl: &mut ListStateRecord,
    mem: &mut MemState,
    subtype: WhatsitSubtype,
    size: i32,
) -> Option<NodePtr<WhatsitNode>> {
    let p = NodePtr::<WhatsitNode>::alloc(mem, size, subtype)?;
    cl.tail_append(mem, p.ptr());
    Some(p)
}

// --- glue spec references ---

#[inline]
pub fn add_glue_ref(mem: &mut MemState, spec: NodePtr<GlueSpecNode>) {
    spec.set_ref_count(mem, spec.ref_count(mem) + 1);
}

/// Drops a reference to `spec`, freeing it with the last one.
pub fn delete_glue_ref(mem: &mut MemState, spec: NodePtr<GlueSpecNode>) {
    if spec.ref_count(mem) == TEX_NULL {
        free_node(mem, spec.ptr(), GLUE_SPEC_SIZE);
    } else {
        spec.set_ref_count(mem, spec.ref_count(mem) - 1);
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

pub(crate) fn push_nest_in(st: &mut crate::state::EngineState) -> Result<(), PushNestAbort> {
    let np = st.input.nest_ptr;
    if np > st.input.max_nest_stack {
        st.input.max_nest_stack = np;
        if np == st.input.nest_size {
            return Err(PushNestAbort::SemanticNest);
        }
    }
    st.input.nest[np as usize] = st.input.cur_list;
    st.input.nest_ptr = np + 1;
    let h = get_avail(&mut st.mem).ok_or(PushNestAbort::MainMemory)?;
    let cl = &mut st.input.cur_list;
    cl.head = h;
    cl.tail = h;
    cl.prev_graf = 0;
    cl.mode_line = st.mem.line;
    cl.e_tex_aux = MemPtr::NULL;
    Ok(())
}

pub fn pop_nest_in(mem: &mut MemState, input: &mut crate::state::InputClusterState) {
    free_avail(mem, input.cur_list.head);
    input.nest_ptr -= 1;
    input.cur_list = input.nest[input.nest_ptr as usize];
}
