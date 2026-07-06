//! Memory arena types and allocator functions translated from
//! `xetex/xetex-xetex0.c`.
use crate::consts::{
    GLUE_BASE, GLUE_SPEC_SIZE, MAX_HALFWORD, MEDIUM_NODE_SIZE, MEM_TOP, NATIVE_GLYPH_INFO_SIZE,
};
pub(crate) use crate::consts::TEX_NULL;
use crate::node::{
    ChoiceNode, DiscNode, GlueNode, GlueSpecNode, HListNode, KernNode, LigItem, LigatureNode,
    MarginKernNode, MathNode, Noad, NodePtr, PenaltyNode, RuleDimen, RuleNode, StyleNode,
};
use crate::state::{EqtbState, MemState};
use crate::units::{FontId, MemIdx, MemPtr, Scaled};


// TeX constants (xetex-constants.h / xetex_format.h)
const NORMAL: u16 = 0;
// Node type values (mem[p].b16.s1)
// eqtb base indices (xetex_format.h)

// empty.s1 = EMPTY = 0, empty.s0 = TEX_NULL — used to zero out math-noad fields.
const EMPTY: i32 = 0; // xetex-constants.h:230


// C functions not yet translated — still live in xetex-xetex0.c.
/// Non-longjmp replacements for bridge_core's `xmalloc`/`xcalloc`/`xrealloc`,
/// which `_tt_abort`/`longjmp` on OOM. These allocate via `libc` and **panic**
/// on exhaustion; `tt_run_engine`'s `catch_unwind` turns that panic into a
/// `HISTORY_FATAL_ERROR` return with a surfaced message — the same terminal
/// outcome as the old longjmp, but without any `longjmp` reaching the engine.
///
/// # Safety
/// The returned pointer is uninitialized C memory (`xcalloc` zeroes it); callers
/// own it and free it with `libc::free`, exactly as with the C allocators.
pub(crate) unsafe fn xmalloc(size: libc::size_t) -> *mut libc::c_void {
    // Mirror bridge_core: `malloc(size ? size : 1)` — a 0-size request still
    // yields a non-null pointer.
    let p = libc::malloc(if size == 0 { 1 } else { size });
    assert!(!p.is_null(), "out of memory: malloc({size} bytes) failed");
    p
}

pub(crate) unsafe fn xcalloc(nelem: libc::size_t, elsize: libc::size_t) -> *mut libc::c_void {
    let p = libc::calloc(if nelem == 0 { 1 } else { nelem }, if elsize == 0 { 1 } else { elsize });
    assert!(!p.is_null(), "out of memory: calloc({nelem}, {elsize}) failed");
    p
}

pub(crate) unsafe fn xrealloc(ptr: *mut libc::c_void, size: libc::size_t) -> *mut libc::c_void {
    if ptr.is_null() {
        return xmalloc(size);
    }
    let p = libc::realloc(ptr, if size == 0 { 1 } else { size });
    assert!(!p.is_null(), "out of memory: realloc({size} bytes) failed");
    p
}

// ---------------------------------------------------------------------------
// get_avail
// ---------------------------------------------------------------------------

/// Overflow sentinel returned by the i32-returning allocators ([`get_node`]
/// and the `new_*` wrappers) when the `mem` arena is exhausted. `i32::MIN`
/// lies outside every valid arena index, `TEX_NULL`, and `MAX_HALFWORD`, so
/// it can never collide with a real return value. ([`get_avail`] signals the
/// same exhaustion as `None`.)
pub(crate) const ALLOC_FAILED: i32 = i32::MIN;

/// Allocate one word from the single-word (hi-mem) free list.
///
/// Returns a pointer to the allocated word, whose link field
/// (`mem[p].b32.s1`) is set to `TEX_NULL` — or `None` if the arena is
/// exhausted (the caller propagates the `main memory size` overflow).
///
/// translated from xetex-xetex0.c:170 (get_avail)
#[inline]
pub fn get_avail(mem: &mut MemState) -> Option<MemPtr> {
    let p = mem.avail;

    if p != TEX_NULL {
        mem.avail = mem.b32s1(p);
    } else if mem.mem_end < MEM_TOP {
        mem.mem_end += 1;
        let p = mem.mem_end;
        mem.set_b32s1(p, TEX_NULL);
        return Some(MemPtr::from_raw(p));
    } else {
        mem.hi_mem_min -= 1;
        let p = mem.hi_mem_min;
        // is_char_node(lo_mem_max) ≡ lo_mem_max >= hi_mem_min
        if mem.lo_mem_max >= mem.hi_mem_min {
            return None;
        }
        mem.set_b32s1(p, TEX_NULL);
        return Some(MemPtr::from_raw(p));
    }

    mem.set_b32s1(p, TEX_NULL);
    Some(MemPtr::from_raw(p))
}

/// The `main memory size` overflow as an [`EngineAbort`] value: run `runaway()`
/// (the runaway-diagnostic side effect) then hand back the `Overflow` terminal
/// for the caller to propagate.
pub(crate) fn main_memory_overflow_abort() -> crate::errors::EngineAbort {
    // runaway() + the "main memory size" report are deferred to into_history.
    crate::errors::EngineAbort::MainMemoryOverflow
}

// ---------------------------------------------------------------------------
// flush_list
// ---------------------------------------------------------------------------

/// Return an entire one-word linked list (headed at `p`) to the free list.
///
/// If `p` is `TEX_NULL`, does nothing. Otherwise walks the list to find
/// the tail, then splices the whole list onto the front of `avail`.
///
/// translated from xetex-xetex0.c:192 (flush_list)
pub fn flush_list(mem: &mut MemState, p: MemPtr) {
    if p.is_null() {
        return;
    }

    let mut r = p;
    let mut q;
    loop {
        q = r;
        r = r.link(mem);
        if r.is_null() {
            break;
        }
    }
    // q is now the tail; splice: tail.link = old_avail; avail = p
    let avail = mem.avail;
    q.set_link(mem, MemPtr::from_raw(avail));
    mem.avail = p.0;
}

// ---------------------------------------------------------------------------
// free_node
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// get_node
// ---------------------------------------------------------------------------

/// Allocate a variable-size node of `s` words from the lo-mem free list.
///
/// Returns a pointer to the first word of the allocated block, or `None`
/// if memory is exhausted (the caller propagates the `main memory size`
/// overflow). The special sentinel request `s == 0x40000000` (the
/// `sort_avail` coalescing probe) yields `Some(MAX_HALFWORD)`.
///
/// The two C `goto` labels are handled with Rust's labeled-loop break:
///   - `goto found`   → `break 'search r`
///   - `goto restart` → `continue 'restart`
///
/// translated from xetex-xetex0.c:150 (get_node)
pub fn get_node(mem: &mut MemState, s: i32) -> Option<MemPtr> {
    let r = 'restart: loop {
        let mut p = mem.rover;

        // do { … } while p != rover()
        loop {
            // Coalesce any immediately-following free blocks into one.
            let mut q = p + mem.b32s0(p);
            while mem.b32s1(q) == MAX_HALFWORD {
                let t = mem.b32s1(q + 1);
                if q == mem.rover {
                    mem.rover = t;
                }
                // Unlink q from the doubly-linked free list.
                let q_left = mem.b32s0(q + 1);
                mem.set_b32s0(t + 1, q_left);
                mem.set_b32s1(q_left + 1, t);
                q += mem.b32s0(q);
            }

            let r = q - s;

            if r > p + 1 {
                // Case 132: block large enough to split; keep left part as free.
                mem.set_b32s0(p, r - p);
                mem.rover = p;
                break 'restart r;
            }

            if r == p && mem.b32s1(p + 1) != p {
                // Case 133: exact fit; unlink p from the free list.
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

        // No suitable block found in one pass through the free list.
        if s == 0x4000_0000 {
            return Some(MemPtr::from_raw(MAX_HALFWORD));
        }

        if mem.lo_mem_max + 2 < mem.hi_mem_min && mem.lo_mem_max + 2 <= MAX_HALFWORD {
            // Grow lo-mem by inserting a new free block between lo_mem_max
            // and hi_mem_min, then retry.
            let t = if mem.hi_mem_min - mem.lo_mem_max >= 1998 {
                mem.lo_mem_max + 1000
            } else {
                mem.lo_mem_max + 1 + (mem.hi_mem_min - mem.lo_mem_max) / 2
            };
            let t = if t > MAX_HALFWORD { MAX_HALFWORD } else { t };

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

    // found:
    mem.set_b32s1(r, TEX_NULL);
    if s >= MEDIUM_NODE_SIZE {
        mem.set_b32s0(r + s - 1, mem.cur_synctex_tag);
        mem.set_b32s1(r + s - 1, mem.line);
    }
    Some(MemPtr::from_raw(r))
}

// ---------------------------------------------------------------------------
// free_node
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// new_null_box
// ---------------------------------------------------------------------------

/// Allocate and zero-initialize an empty hbox node (BOX_NODE_SIZE words),
/// returning its `NodePtr<HListNode>` handle (`None` on arena exhaustion).
///
/// translated from xetex-xetex0.c:147 (new_null_box)
pub fn alloc_null_box(mem: &mut MemState) -> Option<NodePtr<HListNode>> {
    let p = NodePtr::<HListNode>::alloc(mem)?;
    p.set_subtype(mem, 0);
    p.set_width(mem, Scaled(0));
    p.set_depth(mem, Scaled(0));
    p.set_height(mem, Scaled(0));
    p.set_shift_amount(mem, Scaled(0));
    p.set_list_ptr(mem, crate::units::MemPtr::NULL);
    p.set_glue_sign(mem, crate::units::GlueSign::Normal);
    p.set_glue_order(mem, NORMAL);
    p.set_glue_set(mem, 0.0);
    Some(p)
}

// ---------------------------------------------------------------------------
// new_rule
// ---------------------------------------------------------------------------

/// Allocate a rule node (RULE_NODE_SIZE words) with all dimensions NULL_FLAG.
///
/// translated from xetex-xetex0.c:162 (new_rule)
///
/// Typed twin of [`new_rule`]: allocate a running-dimension rule and return its
/// `NodePtr<RuleNode>` handle directly. `None` is the arena-exhaustion sentinel.
pub fn alloc_rule(mem: &mut MemState) -> Option<NodePtr<RuleNode>> {
    let p = NodePtr::<RuleNode>::alloc(mem)?;
    p.set_subtype(mem, 0);
    p.set_width(mem, RuleDimen::Running);
    p.set_depth(mem, RuleDimen::Running);
    p.set_height(mem, RuleDimen::Running);
    Some(p)
}


// ---------------------------------------------------------------------------
// new_ligature
// ---------------------------------------------------------------------------

/// Allocate a ligature node (SMALL_NODE_SIZE words) for font `f`, char `c`,
/// and successor list `q`.
///
/// Note: `f` (internal_font_number = i32) is cast to u16 to fit in b16.s1,
/// matching the C's implicit narrowing assignment.
///
/// translated from xetex-xetex0.c:174 (new_ligature)
pub fn alloc_ligature(
    mem: &mut MemState,
    f: i32,
    c: u16,
    q: impl crate::units::MemAddr,
) -> Option<NodePtr<LigatureNode>> {
    let q = q.addr();
    let p = NodePtr::<LigatureNode>::alloc(mem)?;
    p.set_font(mem, FontId(f));
    p.set_character(mem, c);
    p.set_lig_ptr(mem, MemPtr::from_raw(q));
    p.set_subtype(mem, 0);
    Some(p)
}

// ---------------------------------------------------------------------------
// new_lig_item
// ---------------------------------------------------------------------------

/// Allocate a lig-item node (SMALL_NODE_SIZE) for char `c`.
///
/// translated from xetex-xetex0.c:186 (new_lig_item)
pub fn alloc_lig_item(mem: &mut MemState, c: u16) -> Option<NodePtr<LigItem>> {
    let p = NodePtr::<LigItem>::alloc(mem)?;
    p.set_character(mem, c);
    p.set_component(mem, MemPtr::from_raw(TEX_NULL));
    Some(p)
}

// ---------------------------------------------------------------------------
// new_disc
// ---------------------------------------------------------------------------

/// Allocate an empty discretionary node (SMALL_NODE_SIZE words: subtype 0,
/// empty pre/post-break), returning its typed `NodePtr<DiscNode>` handle
/// (`None` on arena exhaustion).
///
/// Typed twin of [`new_disc`].
pub fn alloc_disc(mem: &mut MemState) -> Option<NodePtr<DiscNode>> {
    let p = NodePtr::<DiscNode>::alloc(mem)?;
    p.set_subtype(mem, 0);
    p.set_pre_break(mem, MemPtr::NULL);
    p.set_post_break(mem, MemPtr::NULL);
    Some(p)
}

// ---------------------------------------------------------------------------
// new_math
// ---------------------------------------------------------------------------

/// Allocate a math-on/off node of width `w` and subtype `s`, returning its
/// typed `NodePtr<MathNode>` handle (`None` on arena exhaustion).
///
/// translated from xetex-xetex0.c:221 (new_math)
pub fn alloc_math(mem: &mut MemState, w: i32, s: i16) -> Option<NodePtr<MathNode>> {
    let p = NodePtr::<MathNode>::alloc(mem)?;
    p.set_subtype(mem, s as u16);
    p.set_width(mem, Scaled(w));
    Some(p)
}

// ---------------------------------------------------------------------------
// new_spec
// ---------------------------------------------------------------------------

/// Allocate a glue-spec node (GLUE_SPEC_SIZE words) copied from the spec
/// at `p`, then clear the copy's reference-count / link field to TEX_NULL.
///
/// translated from xetex-xetex0.c:231 (new_spec)
pub fn new_spec(mem: &mut MemState, p: impl crate::units::MemAddr) -> i32 {
    let p = p.addr();
    let Some(q) = NodePtr::<GlueSpecNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    let p_spec =
        NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(p)).expect("non-null glue spec");
    let word = mem.word(p);
    mem.set_word(q.idx().0, word); // copy the whole first word (type + ref_count)
    q.set_ref_count(mem, TEX_NULL); // clear ref_count / link
    q.set_width(mem, p_spec.width(mem));
    q.set_stretch(mem, p_spec.stretch(mem));
    q.set_shrink(mem, p_spec.shrink(mem));
    q.idx().0
}

// ---------------------------------------------------------------------------
// new_param_glue
// ---------------------------------------------------------------------------

/// Allocate a glue node for glue parameter `n` from the format's glue
/// parameter table, returning its typed `NodePtr<GlueNode>` handle (`None` on
/// arena exhaustion).
///
/// Typed twin of [`new_param_glue`].
pub fn alloc_param_glue(mem: &mut MemState, eqtb: &EqtbState, n: i16) -> Option<NodePtr<GlueNode>> {
    let p = NodePtr::<GlueNode>::alloc(mem)?;
    p.set_subtype(mem, n as u16 + 1); // subtype = n + 1
    p.set_leader_ptr(mem, MemPtr::NULL);
    // q = eqtb[GLUE_BASE + n].b32.s1
    let q = eqtb.b32s1(GLUE_BASE + n as i32);
    p.set_glue_ptr(mem, MemPtr::from_raw(q));
    // GLUE_SPEC_ref_count(q)++  →  mem[q].b32.s1 += 1
    let spec =
        NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(q)).expect("non-null glue spec");
    spec.set_ref_count(mem, spec.ref_count(mem) + 1);
    Some(p)
}

/// translated from xetex-xetex0.c:243 (new_param_glue)
pub fn new_param_glue(mem: &mut MemState, eqtb: &EqtbState, n: i16) -> i32 {
    alloc_param_glue(mem, eqtb, n).map_or(ALLOC_FAILED, |p| p.idx().0)
}

// ---------------------------------------------------------------------------
// new_glue
// ---------------------------------------------------------------------------

/// Allocate a glue node pointing to the existing glue spec `q`, and
/// bump the spec's reference count.
///
/// translated from xetex-xetex0.c:258 (new_glue)
///
/// Typed twin of [`new_glue`]: allocate a glue node pointing at spec `q` and
/// return its `NodePtr<GlueNode>` handle directly. Callers that immediately need
/// the typed node (to set its subtype / read its spec) use this instead of
/// re-deriving it from `new_glue`'s raw index via `classify_glue().expect()`.
/// `None` is the arena-exhaustion sentinel.
pub fn alloc_glue(mem: &mut MemState, q: impl crate::units::MemAddr) -> Option<NodePtr<GlueNode>> {
    let q = q.addr();
    let p = NodePtr::<GlueNode>::alloc(mem)?;
    // GLUE_SPEC_shrink_order(p) = NORMAL  →  mem[p].b16.s0 = NORMAL
    p.set_subtype(mem, NORMAL);
    p.set_leader_ptr(mem, MemPtr::NULL);
    p.set_glue_ptr(mem, MemPtr::from_raw(q));
    // GLUE_SPEC_ref_count(q)++
    let spec =
        NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(q)).expect("non-null glue spec");
    spec.set_ref_count(mem, spec.ref_count(mem) + 1);
    Some(p)
}

pub fn new_glue(mem: &mut MemState, q: impl crate::units::MemAddr) -> i32 {
    alloc_glue(mem, q).map_or(ALLOC_FAILED, |p| p.idx().0)
}

// ---------------------------------------------------------------------------
// new_skip_param
// ---------------------------------------------------------------------------

/// Allocate a glue node whose spec is a fresh copy of parameter `n`,
/// setting `temp_ptr` to the copy.
///
/// translated from xetex-xetex0.c:270 (new_skip_param)
pub fn new_skip_param(mem: &mut MemState, eqtb: &EqtbState, n: i16) -> i32 {
    let spec = eqtb.b32s1(GLUE_BASE + n as i32);
    let s = new_spec(mem, spec);
    if s == ALLOC_FAILED {
        return s;
    }
    mem.temp_ptr.0 = s;
    let p = new_glue(mem, s);
    if p == ALLOC_FAILED {
        return p;
    }
    let s_spec =
        NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(s)).expect("non-null glue spec");
    s_spec.set_ref_count(mem, TEX_NULL); // clear the spec's ref_count link
    let p_glue = unsafe { NodePtr::<GlueNode>::from_idx_unchecked(MemIdx::new(p)) };
    p_glue.set_subtype(mem, n as u16 + 1); // subtype = n + 1
    p
}

// ---------------------------------------------------------------------------
// new_kern
// ---------------------------------------------------------------------------

/// Allocate a kern node of width `w`.
///
/// translated from xetex-xetex0.c:281 (new_kern)
///
/// Allocate a normal kern of width `w` and return its `NodePtr<KernNode>`
/// handle. `None` is the arena-exhaustion sentinel.
pub fn alloc_kern(mem: &mut MemState, w: i32) -> Option<NodePtr<KernNode>> {
    // Typesafe allocation: `alloc` type-stamps NODE_type = KERN_NODE and
    // confines the family's `from_idx_unchecked`.
    let p = NodePtr::<KernNode>::alloc(mem)?;
    p.set_subtype(mem, NORMAL);
    p.set_width(mem, Scaled(w));
    Some(p)
}

// ---------------------------------------------------------------------------
// new_penalty
// ---------------------------------------------------------------------------

/// Allocate a penalty node with value `m`.
///
/// translated from xetex-xetex0.c:291 (new_penalty)
///
/// Allocate a penalty node of value `m` and return its `NodePtr<PenaltyNode>`
/// handle. `None` is the arena-exhaustion sentinel.
pub fn alloc_penalty(mem: &mut MemState, m: i32) -> Option<NodePtr<PenaltyNode>> {
    // Typesafe allocation: `alloc` type-stamps NODE_type = PENALTY_NODE and
    // confines the family's `from_idx_unchecked`.
    let p = NodePtr::<PenaltyNode>::alloc(mem)?;
    p.set_subtype(mem, 0);
    p.set_penalty(mem, m);
    Some(p)
}

// ---------------------------------------------------------------------------
// free_node
// ---------------------------------------------------------------------------

/// Return a variable-size node of size `s` words (headed at `p`) to the
/// variable-size free list rooted at `rover`.
///
/// translated from xetex-xetex0.c:284 (free_node)
pub fn free_node(mem: &mut MemState, p: MemPtr, s: i32) {
    // Record node size and mark it as a free block (MAX_HALFWORD sentinel).
    mem.set_b32s0(p, s);
    mem.set_b32s1(p, MAX_HALFWORD);

    // Insert p between rover and rover's left-neighbor (rover+1.s0).
    let rover = mem.rover;
    let q = mem.b32s0(rover + 1);
    mem.set_b32s0(p + 1, q);
    mem.set_b32s1(p + 1, rover);
    mem.set_b32s0(rover + 1, p.0);
    mem.set_b32s1(q + 1, p.0);
}

// ---------------------------------------------------------------------------
// copy_native_glyph_info
// ---------------------------------------------------------------------------

/// Copy glyph layout info from native node `src` to `dest`, allocating a
/// fresh heap buffer for the destination.
///
/// `NATIVE_NODE_glyph_info_ptr(p)` = `mem[p+5].ptr`
/// `NATIVE_NODE_glyph_count(p)`    = `mem[p+4].b16.s0`
///
/// translated from xetex-xetex0.c:146 (copy_native_glyph_info)
pub fn copy_native_glyph_info(mem: &mut MemState, src: impl crate::units::MemAddr, dest: impl crate::units::MemAddr) {
    let src = src.addr();
    let dest = dest.addr();
    let src_ptr = mem.ptr(src + 5);
    if !src_ptr.is_null() {
        let glyph_count = mem.b16s0(src + 4) as usize;
        let byte_count = glyph_count * NATIVE_GLYPH_INFO_SIZE;
        // xmalloc_array(char, glyph_count * NATIVE_GLYPH_INFO_SIZE)
        //   = xmalloc((glyph_count * NATIVE_GLYPH_INFO_SIZE + 1) * sizeof(char))
        // SAFETY: `xmalloc` returns an owned buffer of `byte_count + 1` bytes;
        // `src_ptr` (checked non-null) holds at least `byte_count` bytes.
        let new_ptr = unsafe { xmalloc(byte_count + 1) };
        unsafe {
            std::ptr::copy_nonoverlapping(src_ptr as *const u8, new_ptr as *mut u8, byte_count);
        }
        mem.set_ptr(dest + 5, new_ptr);
        mem.set_b16s0(dest + 4, glyph_count as u16);
    }
}

// ---------------------------------------------------------------------------
// delete_token_ref / delete_glue_ref
// ---------------------------------------------------------------------------

/// Decrement the reference count of a token list; free if it reaches TEX_NULL.
///
/// translated from xetex-xetex0.c:976 (delete_token_ref)
pub fn delete_token_ref(mem: &mut MemState, p: impl crate::units::MemAddr) {
    let p = p.addr();
    if mem.b32s0(p) == TEX_NULL {
        flush_list(mem, crate::units::MemPtr::from_raw(p));
    } else {
        mem.set_b32s0(p, mem.b32s0(p) - 1);
    }
}

/// Decrement the reference count of a glue spec; free if it reaches TEX_NULL.
///
/// translated from xetex-xetex0.c:984 (delete_glue_ref)
pub fn delete_glue_ref(mem: &mut MemState, p: impl crate::units::MemAddr) {
    let p = p.addr();
    if let Some(spec) = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(p)) {
        if spec.ref_count(mem) == TEX_NULL {
            free_node(mem, crate::units::MemPtr::from_idx(spec.idx()), GLUE_SPEC_SIZE);
        } else {
            spec.set_ref_count(mem, spec.ref_count(mem) - 1);
        }
    }
}

// ---------------------------------------------------------------------------
// push_nest / pop_nest
// ---------------------------------------------------------------------------

/// Cold abort sentinel for [`push_nest_in`]: either the semantic
/// nest stack is full or the avail list is exhausted.
pub(crate) enum PushNestAbort {
    SemanticNest,
    MainMemory,
}

impl PushNestAbort {
    /// Map to the [`EngineAbort`] terminal. `MainMemory` runs
    /// `runaway` first (via `main_memory_overflow_abort`).
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            PushNestAbort::SemanticNest => crate::errors::EngineAbort::NestOverflow,
            PushNestAbort::MainMemory => main_memory_overflow_abort(),
        }
    }
}

/// Core of [`push_nest`].
///
/// translated from xetex-xetex0.c:1323 (push_nest)
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
    st.input.cur_list.head = h;
    st.input.cur_list.tail = h;
    st.input.cur_list.prev_graf = 0;
    st.input.cur_list.mode_line = st.mem.line;
    st.input.cur_list.e_tex_aux = crate::units::MemPtr::NULL;
    Ok(())
}

/// Core of [`pop_nest`].
pub fn pop_nest_in(mem: &mut MemState, input: &mut crate::state::InputClusterState) {
    let head = input.cur_list.head.0;
    mem.set_b32s1(head, mem.avail);
    mem.avail = head;
    input.nest_ptr -= 1;
    let rec = input.nest[input.nest_ptr as usize];
    input.cur_list = rec;
}

// ---------------------------------------------------------------------------
// prev_rightmost
// ---------------------------------------------------------------------------

pub fn prev_rightmost(mem: &MemState, s: impl crate::units::MemAddr, e: impl crate::units::MemAddr) -> MemPtr {
    let e = e.addr();
    let mut p = MemPtr::from_raw(s.addr());
    if p.is_null() {
        return MemPtr::NULL;
    }
    while p.link(mem).0 != e {
        p = p.link(mem);
        if p.is_null() {
            return MemPtr::NULL;
        }
    }
    p
}

// ---------------------------------------------------------------------------
// is_char_node
// ---------------------------------------------------------------------------

/// `is_char_node(p)` ≡ `p >= hi_mem_min`: a node pointer that lives in the
/// upper "one-word" region of `mem` is a character node.
#[inline]
pub fn is_char_node(mem: &MemState, p: impl crate::units::MemAddr) -> bool {
    p.addr() >= mem.hi_mem_min
}

// ---------------------------------------------------------------------------
// new_noad / new_style / new_choice
// ---------------------------------------------------------------------------

/// Allocate and zero-initialize an ordinary noad (NOAD_SIZE = 4 words),
/// returning its `NodePtr<Noad>` handle (`None` on arena exhaustion).
///
/// translated from xetex-xetex0.c:10542 (new_noad)
pub fn alloc_noad(mem: &mut MemState) -> Option<NodePtr<Noad>> {
    let p = NodePtr::<Noad>::alloc(mem)?;
    p.set_subtype(mem, NORMAL);
    p.set_nucleus_type(mem, EMPTY);
    p.set_nucleus_field(mem, TEX_NULL);
    p.set_supscr_type(mem, EMPTY);
    p.set_supscr_field(mem, TEX_NULL);
    p.set_subscr_type(mem, EMPTY);
    p.set_subscr_field(mem, TEX_NULL);
    Some(p)
}

/// Allocate a style node (STYLE_NODE_SIZE = 3 words).
///
/// translated from xetex-xetex0.c:10554 (new_style)
pub fn alloc_style(mem: &mut MemState, s: i16) -> Option<NodePtr<StyleNode>> {
    let p = NodePtr::<StyleNode>::alloc(mem)?;
    p.set_subtype(mem, s as u16);
    p.clear_payload(mem);
    Some(p)
}

/// Allocate a choice node (STYLE_NODE_SIZE = 3 words).
///
/// translated from xetex-xetex0.c:10565 (new_choice)
pub fn alloc_choice(mem: &mut MemState) -> Option<NodePtr<ChoiceNode>> {
    let p = NodePtr::<ChoiceNode>::alloc(mem)?;
    p.set_subtype(mem, 0);
    p.set_display_mlist(mem, MemPtr::NULL);
    p.set_text_mlist(mem, MemPtr::NULL);
    p.set_script_mlist(mem, MemPtr::NULL);
    p.set_script_script_mlist(mem, MemPtr::NULL);
    Some(p)
}

/// Allocate a margin kern node (MARGIN_KERN_NODE_SIZE = 3 words).
///
/// translated from xetex-xetex0.c:9838 (new_margin_kern)
///
/// Typed twin of [`new_margin_kern`]: allocate a margin kern of width `w` on the
/// given `side` and return its `NodePtr<MarginKernNode>` handle directly. `None`
/// is the arena-exhaustion sentinel.
pub fn alloc_margin_kern(
    mem: &mut MemState,
    w: i32,
    _p: i32,
    side: i16,
) -> Option<NodePtr<MarginKernNode>> {
    let k = NodePtr::<MarginKernNode>::alloc(mem)?;
    k.set_subtype(mem, side as u16);
    k.set_width(mem, Scaled(w));
    Some(k)
}

// translated from xetex-xetex0.c:13763 (new_whatsit)
pub fn new_whatsit(
    cl: &mut crate::mem_types::ListStateRecord,
    mem: &mut MemState,
    s: i16,
    w: i16,
) -> Option<MemPtr> {
    let p = get_node(mem, w as i32)?;
    mem.set_b16s1(p, 8); // NODE_type(p) = WHATSIT_NODE (xetex_format.h:388)
    mem.set_b16s0(p, s as u16);
    cl.tail.set_link(mem, p);
    cl.tail = p;
    Some(p)
}
