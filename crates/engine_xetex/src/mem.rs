//! Memory arena types and Phase-2 allocator functions translated from
//! `xetex/xetex-xetex0.c`.
//!
//! Phase 2 targets are the node allocation helpers (`get_avail`,
//! `flush_list`, `get_node`, `free_node`, `new_*`). During Phase A of
//! state migration (see `plan/02-state.md`), all arena access goes
//! through the raw C `mem[]` pointer wired up in `src/globals.rs`.
use crate::consts::{GLUE_BASE, GLUE_SPEC_SIZE, MAX_HALFWORD, MEDIUM_NODE_SIZE};
use crate::errors::overflow;
use crate::errors::runaway;
use crate::node::{
    ChoiceNode, DiscNode, GlueNode, GlueSpecNode, HListNode, KernNode, LigItem, LigatureNode,
    MarginKernNode, MathNode, Noad, NodePtr, PenaltyNode, RuleDimen, RuleNode, StyleNode,
};
use crate::state::{EqtbState, MemState};
use crate::units::{FontId, MemIdx, MemPtr, Scaled};

use crate::globals::{
    cur_input_synctex_tag, cur_list_head, line, nest_size, set_cur_list_e_tex_aux,
    set_cur_list_head, set_cur_list_mode_line, set_cur_list_prev_graf, set_cur_list_tail,
};

// TeX constants (xetex-constants.h / xetex_format.h)
pub(crate) const TEX_NULL: i32 = -0x0FFF_FFFF; // MIN_HALFWORD
const MEM_TOP: i32 = 4_999_999;
const NORMAL: u16 = 0;
// Node type values (mem[p].b16.s1)
// eqtb base indices (xetex_format.h)

// empty.s1 = EMPTY = 0, empty.s0 = TEX_NULL — used to zero out math-noad fields.
const EMPTY: i32 = 0; // xetex-constants.h:230

// native node glyph info size in bytes (xetex-constants.h:304)
const NATIVE_GLYPH_INFO_SIZE: usize = 10;

// C functions not yet translated — still live in xetex-xetex0.c.
extern "C" {

    // bridge_core: tectonic_bridge_core.h:155
    fn xmalloc(size: libc::size_t) -> *mut libc::c_void;
}

// The scalar memory newtypes (`MemIdx`, `MemPtr`, `Scaled`, `StrNumber`,
// `FontId`, `Token`) now live in `crate::units` — the typed-memory
// foundation module (`plan/idiom/01c` §2 Stage 1). The vestigial stubs
// that used to sit here (with the wrong `MemIdx::NULL = MemIdx(0)`) had no
// call sites and were superseded; see `units.rs` for the corrected forms.

// ---------------------------------------------------------------------------
// get_avail
// ---------------------------------------------------------------------------

/// Overflow sentinel returned by the threaded allocators ([`get_avail`],
/// [`get_node`]) when the `mem` arena is exhausted (`01b` §5). Because
/// `overflow()`/`runaway()` are `with_state`-bearing, they cannot be called
/// under the live borrow held while a threaded allocator runs; the allocator
/// returns this sentinel instead and the bridge-down wrapper aborts after the
/// borrow drops. `i32::MIN` lies outside every valid arena index, `TEX_NULL`,
/// and `MAX_HALFWORD`, so it can never collide with a real return value.
pub(crate) const ALLOC_FAILED: i32 = i32::MIN;

/// Allocate one word from the single-word (hi-mem) free list.
///
/// Returns the index of the allocated word. The word's link field
/// (`mem[p].b32.s1`) is set to `TEX_NULL`.
///
/// translated from xetex-xetex0.c:170 (get_avail)
///
/// Phase-C: threaded to `&mut MemState`. On arena exhaustion it returns
/// [`ALLOC_FAILED`] rather than calling `runaway()`/`overflow()` under the
/// borrow (`01b` §5); the [`get_avail_global`] wrapper performs the abort.
pub fn get_avail(mem: &mut MemState) -> i32 {
    let p = mem.avail;

    if p != TEX_NULL {
        mem.avail = mem.b32s1(p);
    } else if mem.mem_end < MEM_TOP {
        mem.mem_end += 1;
        let p = mem.mem_end;
        mem.set_b32s1(p, TEX_NULL);
        return p;
    } else {
        mem.hi_mem_min -= 1;
        let p = mem.hi_mem_min;
        // is_char_node(lo_mem_max) ≡ lo_mem_max >= hi_mem_min
        if mem.lo_mem_max >= mem.hi_mem_min {
            return ALLOC_FAILED;
        }
        mem.set_b32s1(p, TEX_NULL);
        return p;
    }

    mem.set_b32s1(p, TEX_NULL);
    p
}

/// Fire the cold `runaway()` + `overflow("main memory size")` abort that an
/// arena-exhausted [`get_avail`] / [`get_node`] defers to its caller via the
/// `ALLOC_FAILED` sentinel (`01b` §5). Must be called only **after** any live
/// `with_state*`/`&mut EngineState` borrow has dropped. Never returns.
pub(crate) unsafe fn fire_main_memory_overflow() -> ! {
    runaway();
    overflow(c"main memory size".as_ptr(), MEM_TOP + 1)
}

/// The `main memory size` overflow as an [`EngineAbort`] value (`01d` §5.5) —
/// the non-longjmp twin of [`fire_main_memory_overflow`]: run `runaway()` (the
/// runaway-diagnostic side effect) then hand back the `Overflow` terminal for
/// the caller to propagate. Call only *after* the state borrow drops (`runaway`
/// re-enters `with_state`).
pub(crate) fn main_memory_overflow_abort() -> crate::errors::EngineAbort {
    // SAFETY: post-borrow; `runaway` re-enters `with_state` freely here.
    unsafe { runaway() };
    crate::errors::EngineAbort::Overflow {
        what: c"main memory size".as_ptr(),
        n: MEM_TOP + 1,
    }
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
///
/// Phase-C: threaded to `&mut MemState`; touches the `mem` arena only (no cold
/// path, zero `with_state` calls). Legacy callers use [`flush_list_global`].
pub fn flush_list(mem: &mut MemState, p: i32) {
    if p == TEX_NULL {
        return;
    }

    let mut r = p;
    let mut q;
    loop {
        q = r;
        r = mem.b32s1(r);
        if r == TEX_NULL {
            break;
        }
    }
    // q is now the tail; splice: tail.link = old_avail; avail = p
    let avail = mem.avail;
    mem.set_b32s1(q, avail);
    mem.avail = p;
}

// ---------------------------------------------------------------------------
// free_node
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// get_node
// ---------------------------------------------------------------------------

/// Allocate a variable-size node of `s` words from the lo-mem free list.
///
/// Returns the index of the first word of the allocated block. Calls
/// `overflow()` (never returns) if memory is exhausted. Returns
/// `MAX_HALFWORD` for the special sentinel request `s == 0x40000000`.
///
/// The two C `goto` labels are handled with Rust's labeled-loop break:
///   - `goto found`   → `break 'search r`
///   - `goto restart` → `continue 'restart`
///
/// translated from xetex-xetex0.c:150 (get_node)
///
/// Phase-C: threaded to `&mut MemState`. The cold exhaustion path returns
/// [`ALLOC_FAILED`] instead of calling `overflow()` under the borrow (`01b`
/// §5); [`get_node_global`] performs the abort. The two raw-global reads
/// (`cur_input_synctex_tag()`/`line()`) read `mod raw`, not the thread-local,
/// so they are safe under a live borrow (`01b` §4).
pub fn get_node(mem: &mut MemState, s: i32) -> i32 {
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
            return MAX_HALFWORD;
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

        return ALLOC_FAILED;
    };

    // found:
    mem.set_b32s1(r, TEX_NULL);
    if s >= MEDIUM_NODE_SIZE {
        mem.set_b32s0(r + s - 1, cur_input_synctex_tag());
        mem.set_b32s1(r + s - 1, line());
    }
    r
}

// ---------------------------------------------------------------------------
// free_node
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// new_null_box
// ---------------------------------------------------------------------------

/// Allocate and zero-initialize an empty hbox node (BOX_NODE_SIZE words).
///
/// translated from xetex-xetex0.c:147 (new_null_box)
///
/// Phase-C: threaded to `&mut MemState`. Calls the threaded [`get_node`]
/// directly and propagates its [`ALLOC_FAILED`] sentinel (`01b` §5).
pub fn new_null_box(mem: &mut MemState) -> i32 {
    let Some(p) = NodePtr::<HListNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_subtype(mem, 0);
    p.set_width(mem, Scaled(0));
    p.set_depth(mem, Scaled(0));
    p.set_height(mem, Scaled(0));
    p.set_shift_amount(mem, Scaled(0));
    p.set_list_ptr(mem, crate::units::MemPtr::NULL);
    p.set_glue_sign(mem, NORMAL);
    p.set_glue_order(mem, NORMAL);
    p.set_glue_set(mem, 0.0);
    p.idx().0
}

// ---------------------------------------------------------------------------
// new_rule
// ---------------------------------------------------------------------------

/// Allocate a rule node (RULE_NODE_SIZE words) with all dimensions NULL_FLAG.
///
/// translated from xetex-xetex0.c:162 (new_rule)
///
/// Phase-C: threaded to `&mut MemState` (callers use [`new_rule`] directly).
pub fn new_rule(mem: &mut MemState) -> i32 {
    let Some(p) = NodePtr::<RuleNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_subtype(mem, 0);
    p.set_width(mem, RuleDimen::Running);
    p.set_depth(mem, RuleDimen::Running);
    p.set_height(mem, RuleDimen::Running);
    p.idx().0
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
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_ligature_global`].
pub fn new_ligature(mem: &mut MemState, f: i32, c: u16, q: i32) -> i32 {
    let Some(p) = NodePtr::<LigatureNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_font(mem, FontId(f));
    p.set_character(mem, c);
    p.set_lig_ptr(mem, MemPtr::from_raw(q));
    p.set_subtype(mem, 0);
    p.idx().0
}

// ---------------------------------------------------------------------------
// new_lig_item
// ---------------------------------------------------------------------------

/// Allocate a lig-item node (SMALL_NODE_SIZE) for char `c`.
///
/// translated from xetex-xetex0.c:186 (new_lig_item)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_lig_item_global`].
pub fn new_lig_item(mem: &mut MemState, c: u16) -> i32 {
    let Some(p) = NodePtr::<LigItem>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_character(mem, c);
    p.set_component(mem, MemPtr::from_raw(TEX_NULL));
    p.idx().0
}

// ---------------------------------------------------------------------------
// new_disc
// ---------------------------------------------------------------------------

/// Allocate a discretionary node (SMALL_NODE_SIZE words) with null lists.
///
/// translated from xetex-xetex0.c:195 (new_disc)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_disc_global`].
pub fn new_disc(mem: &mut MemState) -> i32 {
    let Some(p) = NodePtr::<DiscNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_subtype(mem, 0);
    p.set_pre_break(mem, MemPtr::NULL);
    p.set_post_break(mem, MemPtr::NULL);
    p.idx().0
}

// ---------------------------------------------------------------------------
// new_math
// ---------------------------------------------------------------------------

/// Allocate a math-on/off node of width `w` and subtype `s`.
///
/// translated from xetex-xetex0.c:221 (new_math)
///
/// Phase-C: threaded to `&mut MemState`.
pub fn new_math(mem: &mut MemState, w: i32, s: i16) -> i32 {
    let Some(p) = NodePtr::<MathNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_subtype(mem, s as u16);
    p.set_width(mem, Scaled(w));
    p.idx().0
}

// ---------------------------------------------------------------------------
// new_spec
// ---------------------------------------------------------------------------

/// Allocate a glue-spec node (GLUE_SPEC_SIZE words) copied from the spec
/// at `p`, then clear the copy's reference-count / link field to TEX_NULL.
///
/// translated from xetex-xetex0.c:231 (new_spec)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_spec_global`].
pub fn new_spec(mem: &mut MemState, p: i32) -> i32 {
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

/// Allocate a glue node referencing parameter `n` from the format's glue
/// parameter table.
///
/// translated from xetex-xetex0.c:243 (new_param_glue)
///
/// Phase-C: threaded to `&mut MemState` and `&EqtbState`; legacy callers use [`new_param_glue_global`].
pub fn new_param_glue(mem: &mut MemState, eqtb: &EqtbState, n: i16) -> i32 {
    let Some(p) = NodePtr::<GlueNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_subtype(mem, n as u16 + 1); // subtype = n + 1
    p.set_leader_ptr(mem, MemPtr::NULL);
    // q = eqtb[GLUE_BASE + n].b32.s1
    let q = eqtb.b32s1(GLUE_BASE + n as i32);
    p.set_glue_ptr(mem, MemPtr::from_raw(q));
    // GLUE_SPEC_ref_count(q)++  →  mem[q].b32.s1 += 1
    let spec =
        NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(q)).expect("non-null glue spec");
    spec.set_ref_count(mem, spec.ref_count(mem) + 1);
    p.idx().0
}

// ---------------------------------------------------------------------------
// new_glue
// ---------------------------------------------------------------------------

/// Allocate a glue node pointing to the existing glue spec `q`, and
/// bump the spec's reference count.
///
/// translated from xetex-xetex0.c:258 (new_glue)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_glue_global`].
pub fn new_glue(mem: &mut MemState, q: i32) -> i32 {
    let Some(p) = NodePtr::<GlueNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    // GLUE_SPEC_shrink_order(p) = NORMAL  →  mem[p].b16.s0 = NORMAL
    p.set_subtype(mem, NORMAL);
    p.set_leader_ptr(mem, MemPtr::NULL);
    p.set_glue_ptr(mem, MemPtr::from_raw(q));
    // GLUE_SPEC_ref_count(q)++
    let spec =
        NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(q)).expect("non-null glue spec");
    spec.set_ref_count(mem, spec.ref_count(mem) + 1);
    p.idx().0
}

// ---------------------------------------------------------------------------
// new_skip_param
// ---------------------------------------------------------------------------

/// Allocate a glue node whose spec is a fresh copy of parameter `n`,
/// setting `temp_ptr` to the copy.
///
/// translated from xetex-xetex0.c:270 (new_skip_param)
///
/// Phase-C: threaded to `&mut MemState` and `&EqtbState`; legacy callers use [`new_skip_param_global`].
pub fn new_skip_param(mem: &mut MemState, eqtb: &EqtbState, n: i16) -> i32 {
    let spec = eqtb.b32s1(GLUE_BASE + n as i32);
    let s = new_spec(mem, spec);
    if s == ALLOC_FAILED {
        return s;
    }
    crate::globals::set_temp_ptr(s);
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
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_kern_global`].
pub fn new_kern(mem: &mut MemState, w: i32) -> i32 {
    // Typesafe allocation: `alloc` type-stamps NODE_type = KERN_NODE and
    // confines the family's `from_idx_unchecked` (`01b` §3a / `01c` §8). This
    // body is now `unsafe`-free and fully typed; `None` is the §5 arena-
    // exhaustion sentinel the `new_kern_global` wrapper maps to `overflow()`.
    let Some(p) = NodePtr::<KernNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_subtype(mem, NORMAL);
    p.set_width(mem, Scaled(w));
    p.idx().0
}

// ---------------------------------------------------------------------------
// new_penalty
// ---------------------------------------------------------------------------

/// Allocate a penalty node with value `m`.
///
/// translated from xetex-xetex0.c:291 (new_penalty)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_penalty_global`].
pub fn new_penalty(mem: &mut MemState, m: i32) -> i32 {
    // Typesafe allocation: `alloc` type-stamps NODE_type = PENALTY_NODE and
    // confines the family's `from_idx_unchecked` (`01b` §3a / `01c` §8). This
    // body is now `unsafe`-free and fully typed; `None` is the §5 arena-
    // exhaustion sentinel the `new_penalty_global` wrapper maps to `overflow()`.
    let Some(p) = NodePtr::<PenaltyNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_subtype(mem, 0);
    p.set_penalty(mem, m);
    p.idx().0
}

// ---------------------------------------------------------------------------
// free_node
// ---------------------------------------------------------------------------

/// Return a variable-size node of size `s` words (headed at `p`) to the
/// variable-size free list rooted at `rover`.
///
/// translated from xetex-xetex0.c:284 (free_node)
///
/// Phase-C: threaded to take `&mut MemState` directly (no thread-local
/// bridge). Touches the `mem` arena only — zero `with_state`-bearing calls,
/// so it is safe under any borrow (`01b` §4).
pub fn free_node(mem: &mut MemState, p: i32, s: i32) {
    // Record node size and mark it as a free block (MAX_HALFWORD sentinel).
    mem.set_b32s0(p, s);
    mem.set_b32s1(p, MAX_HALFWORD);

    // Insert p between rover and rover's left-neighbor (rover+1.s0).
    let rover = mem.rover;
    let q = mem.b32s0(rover + 1);
    mem.set_b32s0(p + 1, q);
    mem.set_b32s1(p + 1, rover);
    mem.set_b32s0(rover + 1, p);
    mem.set_b32s1(q + 1, p);
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
///
/// Phase-C: threaded to `&mut MemState`; touches the `mem` arena only plus a
/// raw `xmalloc`/`copy_nonoverlapping` of the native-glyph buffer (no
/// `with_state` calls, no cold path). Called by the threaded `copy_node_list`
/// and `just_copy`.
pub fn copy_native_glyph_info(mem: &mut MemState, src: i32, dest: i32) {
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
///
/// Phase-C: threaded to `&mut MemState`; touches the `mem` arena only and calls
/// the threaded [`flush_list`] (no cold path, zero `with_state` calls).
pub fn delete_token_ref(mem: &mut MemState, p: i32) {
    if mem.b32s0(p) == TEX_NULL {
        flush_list(mem, p);
    } else {
        mem.set_b32s0(p, mem.b32s0(p) - 1);
    }
}

/// Decrement the reference count of a glue spec; free if it reaches TEX_NULL.
///
/// translated from xetex-xetex0.c:984 (delete_glue_ref)
///
/// Phase-C: threaded to `&mut MemState`; touches the `mem` arena only and calls
/// the threaded [`free_node`] (no cold path, zero `with_state` calls).
pub fn delete_glue_ref(mem: &mut MemState, p: i32) {
    if let Some(spec) = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(p)) {
        if spec.ref_count(mem) == TEX_NULL {
            free_node(mem, spec.idx().0, GLUE_SPEC_SIZE);
        } else {
            spec.set_ref_count(mem, spec.ref_count(mem) - 1);
        }
    }
}

// ---------------------------------------------------------------------------
// push_nest / pop_nest
// ---------------------------------------------------------------------------

/// Cold abort sentinel for [`push_nest_in`] (`01b` §5): either the semantic
/// nest stack is full or the avail list is exhausted. Both `-> !` after the
/// borrow drops.
pub(crate) enum PushNestAbort {
    SemanticNest,
    MainMemory,
}

impl PushNestAbort {
    /// Map to the unified [`EngineAbort`] terminal (`01d` §5.5), mirroring the
    /// former `fire()` cold-abort tail without longjmp-ing. `MainMemory` runs
    /// `runaway` first (via `main_memory_overflow_abort`).
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            PushNestAbort::SemanticNest => crate::errors::EngineAbort::Overflow {
                what: c"semantic nest size".as_ptr(),
                n: nest_size(),
            },
            PushNestAbort::MainMemory => main_memory_overflow_abort(),
        }
    }
}

/// Threaded (Phase C) core of [`push_nest`]: the nest stack lives in
/// `st.input`; `cur_list` is the raw `mod raw` record (safe under the borrow).
///
/// translated from xetex-xetex0.c:1323 (push_nest)
pub(crate) fn push_nest_in(st: &mut crate::state::EngineState) -> Result<(), PushNestAbort> {
    let np = st.input.nest_ptr;
    if np > st.input.max_nest_stack {
        st.input.max_nest_stack = np;
        if np == crate::globals::nest_size() {
            return Err(PushNestAbort::SemanticNest);
        }
    }
    st.input.nest[np as usize] = crate::globals::cur_list();
    st.input.nest_ptr = np + 1;
    let h = get_avail(&mut st.mem);
    if h == ALLOC_FAILED {
        return Err(PushNestAbort::MainMemory);
    }
    set_cur_list_head(h);
    set_cur_list_tail(h);
    set_cur_list_prev_graf(0);
    set_cur_list_mode_line(line());
    set_cur_list_e_tex_aux(TEX_NULL);
    Ok(())
}

/// Threaded (Phase C) core of [`pop_nest`]: touches the `mem` arena and the
/// `nest` stack directly through the borrowed clusters, so it is safe to call
/// under a live `&mut EngineState` borrow. `cur_list` is still the deferred
/// `mod raw` static, so reading/writing it (via `cur_list_head` /
/// `set_cur_list_record`) is a raw access that does not re-enter `with_state`.
pub fn pop_nest_in(mem: &mut MemState, input: &mut crate::state::InputClusterState) {
    let head = cur_list_head();
    mem.set_b32s1(head, mem.avail);
    mem.avail = head;
    input.nest_ptr -= 1;
    let rec = input.nest[input.nest_ptr as usize];
    crate::globals::set_cur_list_record(rec);
}

// ---------------------------------------------------------------------------
// prev_rightmost
// ---------------------------------------------------------------------------

pub fn prev_rightmost(mem: &MemState, s: i32, e: i32) -> i32 {
    let mut p = s;
    if p == TEX_NULL {
        return TEX_NULL;
    }
    while mem.b32s1(p) != e {
        p = mem.b32s1(p);
        if p == TEX_NULL {
            return TEX_NULL;
        }
    }
    p
}

// ---------------------------------------------------------------------------
// is_char_node
// ---------------------------------------------------------------------------

/// `is_char_node(p)` ≡ `p >= hi_mem_min`: a node pointer that lives in the
/// upper "one-word" region of `mem` is a character node. Canonical threaded
/// form, shared by linebreak / math / shipout (was duplicated as
/// `is_char_node_in`).
#[inline]
pub fn is_char_node(mem: &MemState, p: i32) -> bool {
    p >= mem.hi_mem_min
}

// ---------------------------------------------------------------------------
// new_noad / new_style / new_choice
// ---------------------------------------------------------------------------

/// Allocate and zero-initialize an ordinary noad (NOAD_SIZE = 4 words).
///
/// translated from xetex-xetex0.c:10542 (new_noad)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_noad_global`].
pub fn new_noad(mem: &mut MemState) -> i32 {
    let Some(p) = NodePtr::<Noad>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_subtype(mem, NORMAL);
    p.set_nucleus_type(mem, EMPTY);
    p.set_nucleus_field(mem, TEX_NULL);
    p.set_supscr_type(mem, EMPTY);
    p.set_supscr_field(mem, TEX_NULL);
    p.set_subscr_type(mem, EMPTY);
    p.set_subscr_field(mem, TEX_NULL);
    p.idx().0
}

/// Allocate a style node (STYLE_NODE_SIZE = 3 words).
///
/// translated from xetex-xetex0.c:10554 (new_style)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_style_global`].
pub fn new_style(mem: &mut MemState, s: i16) -> i32 {
    let Some(p) = NodePtr::<StyleNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_subtype(mem, s as u16);
    p.clear_payload(mem);
    p.idx().0
}

/// Allocate a choice node (STYLE_NODE_SIZE = 3 words).
///
/// translated from xetex-xetex0.c:10565 (new_choice)
///
/// Phase-C: threaded to `&mut MemState`.
pub fn new_choice(mem: &mut MemState) -> i32 {
    let Some(p) = NodePtr::<ChoiceNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    p.set_subtype(mem, 0);
    p.set_display_mlist(mem, MemPtr::NULL);
    p.set_text_mlist(mem, MemPtr::NULL);
    p.set_script_mlist(mem, MemPtr::NULL);
    p.set_script_script_mlist(mem, MemPtr::NULL);
    p.idx().0
}

/// Allocate a margin kern node (MARGIN_KERN_NODE_SIZE = 3 words).
///
/// translated from xetex-xetex0.c:9838 (new_margin_kern)
///
/// Phase-C: threaded to `&mut MemState`.
pub fn new_margin_kern(mem: &mut MemState, w: i32, _p: i32, side: i16) -> i32 {
    let Some(k) = NodePtr::<MarginKernNode>::alloc(mem) else {
        return ALLOC_FAILED;
    };
    k.set_subtype(mem, side as u16);
    k.set_width(mem, Scaled(w));
    k.idx().0
}

// translated from xetex-xetex0.c:13763 (new_whatsit)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_whatsit_global`].
pub fn new_whatsit(mem: &mut MemState, s: i16, w: i16) -> i32 {
    let p = get_node(mem, w as i32);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, 8); // NODE_type(p) = WHATSIT_NODE (xetex_format.h:388)
    mem.set_b16s0(p, s as u16);
    mem.set_b32s1(crate::globals::cur_list_tail(), p);
    crate::globals::set_cur_list_tail(p);
    p
}
