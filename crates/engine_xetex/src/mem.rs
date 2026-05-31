//! Memory arena types and Phase-2 allocator functions translated from
//! `xetex/xetex-xetex0.c`.
//!
//! Phase 2 targets are the node allocation helpers (`get_avail`,
//! `flush_list`, `get_node`, `free_node`, `new_*`). During Phase A of
//! state migration (see `plan/02-state.md`), all arena access goes
//! through the raw C `mem[]` pointer wired up in `src/globals.rs`.
use crate::consts::{
    BOX_NODE_SIZE, DISC_NODE, GLUE_BASE, GLUE_NODE, GLUE_SPEC_SIZE, HLIST_NODE, KERN_NODE,
    LIGATURE_NODE, MATH_NODE, MAX_HALFWORD, MEDIUM_NODE_SIZE, NULL_FLAG, PENALTY_NODE, RULE_NODE,
    RULE_NODE_SIZE, SMALL_NODE_SIZE,
};
use crate::errors::overflow;
use crate::scan::runaway;
use crate::state::{EqtbState, MemState};

use crate::globals::{
    avail, cur_input_synctex_tag, cur_list_head, eqtb_b32s1, line, max_nest_stack, mem_b16s0,
    mem_b32s0, mem_b32s1, mem_ptr, nest_ptr, nest_size, restore_cur_list_from_nest,
    save_cur_list_to_nest, set_avail, set_cur_list_e_tex_aux, set_cur_list_head,
    set_cur_list_mode_line, set_cur_list_prev_graf, set_cur_list_tail, set_max_nest_stack,
    set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1, set_mem_ptr, set_nest_ptr,
    set_temp_ptr, temp_ptr,
};

// TeX constants (xetex-constants.h / xetex_format.h)
pub(crate) const TEX_NULL: i32 = -0x0FFF_FFFF; // MIN_HALFWORD
const MEM_TOP: i32 = 4_999_999;
const NORMAL: u16 = 0;
// Node type values (mem[p].b16.s1)
// eqtb base indices (xetex_format.h)

// Math node type and size constants (xetex_format.h:394–401; xetex-constants.h:119,136).
const NOAD_SIZE: i32 = 4;
const STYLE_NODE_SIZE: i32 = 3;
const ORD_NOAD: u16 = 16; // xetex_format.h:401
const STYLE_NODE: u16 = 14; // xetex_format.h:394
const CHOICE_NODE: u16 = 15; // xetex_format.h:395
                             // empty.s1 = EMPTY = 0, empty.s0 = TEX_NULL — used to zero out math-noad fields.
const EMPTY: i32 = 0; // xetex-constants.h:230

// Margin kern node (xetex-constants.h:115; xetex_format.h:396).
const MARGIN_KERN_NODE_SIZE: i32 = 3;
const MARGIN_KERN_NODE: u16 = 40;
// native node glyph info size in bytes (xetex-constants.h:304)
const NATIVE_GLYPH_INFO_SIZE: usize = 10;

// C functions not yet translated — still live in xetex-xetex0.c.
extern "C" {

    // bridge_core: tectonic_bridge_core.h:155
    fn xmalloc(size: libc::size_t) -> *mut libc::c_void;
}

/// Index into the TeX memory arena (`mem[]`).
///
/// Kept as signed `i32` for parity with the C `int32_t p` variables,
/// some of which store negative sentinel values alongside real indices.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct MemIdx(pub i32);

impl MemIdx {
    pub const NULL: MemIdx = MemIdx(0);

    #[inline]
    pub fn is_null(self) -> bool {
        self == Self::NULL
    }

    #[inline]
    pub fn offset(self, delta: i32) -> MemIdx {
        MemIdx(self.0 + delta)
    }
}

/// TeX's basic length unit: 1/65536 of a point.
///
/// The C engine uses `scaled_t` (which is `int32_t`) everywhere a length
/// appears. We keep the same representation but distinguish it from
/// plain counts with a newtype.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub struct Scaled(pub i32);

/// Internal font number.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FontId(pub u16);

/// Packed `(cmd, chr)` token, as stored in `cur_tok` and on the input
/// stack.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Token(pub i32);

/// Index into the string pool.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct StrNumber(pub i32);

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

/// Transitional bridge-down wrapper for legacy `get_avail` callers (`01b` §3,
/// §5). Evaluates the threaded allocator under a single borrow, then — after
/// the borrow drops — runs the cold `runaway()`+`overflow()` abort if the arena
/// was exhausted. Removed when the last legacy caller is itself threaded.
pub unsafe fn get_avail_global() -> i32 {
    let p = crate::state::with_state_mut(|st| get_avail(&mut st.mem));
    if p == ALLOC_FAILED {
        runaway();
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
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

/// Transitional bridge-down wrapper for legacy `flush_list` callers (`01b` §3).
pub unsafe fn flush_list_global(p: i32) {
    crate::state::with_state_mut(|st| flush_list(&mut st.mem, p));
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

/// Transitional bridge-down wrapper for legacy `get_node` callers (`01b` §3,
/// §5). Runs the threaded allocator under a single borrow, then — after the
/// borrow drops — performs the cold `overflow()` abort if the arena was
/// exhausted. Removed when the last legacy caller is itself threaded.
pub unsafe fn get_node_global(s: i32) -> i32 {
    let r = crate::state::with_state_mut(|st| get_node(&mut st.mem, s));
    if r == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
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
/// directly and propagates its [`ALLOC_FAILED`] sentinel (`01b` §5); the
/// [`new_null_box_global`] wrapper performs the cold `overflow()` abort.
pub fn new_null_box(mem: &mut MemState) -> i32 {
    let p = get_node(mem, BOX_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    // NODE_type(p) = HLIST_NODE  →  mem[p].b16.s1 = HLIST_NODE
    mem.set_b16s1(p, HLIST_NODE);
    mem.set_b16s0(p, 0); // subtype
    mem.set_b32s1(p + 1, 0); // width
    mem.set_b32s1(p + 2, 0); // depth
    mem.set_b32s1(p + 3, 0); // height
    mem.set_b32s1(p + 4, 0); // shift_amount
    mem.set_b32s1(p + 5, TEX_NULL); // list_ptr
    mem.set_b16s1(p + 5, NORMAL); // glue_sign (b16.s1, bytes 2-3 of word 5)
    mem.set_b16s0(p + 5, NORMAL); // glue_order (b16.s0, bytes 0-1 of word 5)
    mem.set_gr(p + 6, 0.0); // BOX_glue_set: mem[(p)+6].gr
    p
}

/// Transitional bridge-down wrapper for legacy `new_null_box` callers (`01b`
/// §3, §5). Runs the threaded constructor under a single borrow, then performs
/// the cold `overflow()` abort after the borrow drops if the arena was
/// exhausted. Removed when the last legacy caller is itself threaded.
pub unsafe fn new_null_box_global() -> i32 {
    let p = crate::state::with_state_mut(|st| new_null_box(&mut st.mem));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
}

// ---------------------------------------------------------------------------
// new_rule
// ---------------------------------------------------------------------------

/// Allocate a rule node (RULE_NODE_SIZE words) with all dimensions NULL_FLAG.
///
/// translated from xetex-xetex0.c:162 (new_rule)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_rule_global`].
pub fn new_rule(mem: &mut MemState) -> i32 {
    let p = get_node(mem, RULE_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, RULE_NODE);
    mem.set_b16s0(p, 0);
    mem.set_b32s1(p + 1, NULL_FLAG); // width
    mem.set_b32s1(p + 2, NULL_FLAG); // depth
    mem.set_b32s1(p + 3, NULL_FLAG); // height
    p
}

/// Transitional bridge-down wrapper for legacy `new_rule` callers (`01b` §3, §5).
pub unsafe fn new_rule_global() -> i32 {
    let p = crate::state::with_state_mut(|st| new_rule(&mut st.mem));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
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
    let p = get_node(mem, SMALL_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, LIGATURE_NODE);
    // word p+1: b16.s1 = font, b16.s0 = char, b32.s1 = successor link.
    // b16.s1 and b16.s0 together form b32.s0; b32.s1 is independent.
    mem.set_b16s1(p + 1, f as u16);
    mem.set_b16s0(p + 1, c);
    mem.set_b32s1(p + 1, q);
    mem.set_b16s0(p, 0); // subtype
    p
}

/// Transitional bridge-down wrapper for legacy `new_ligature` callers (`01b` §3, §5).
pub unsafe fn new_ligature_global(f: i32, c: u16, q: i32) -> i32 {
    let p = crate::state::with_state_mut(|st| new_ligature(&mut st.mem, f, c, q));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
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
    let p = get_node(mem, SMALL_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s0(p, c);
    mem.set_b32s1(p + 1, TEX_NULL);
    p
}

/// Transitional bridge-down wrapper for legacy `new_lig_item` callers (`01b` §3, §5).
pub unsafe fn new_lig_item_global(c: u16) -> i32 {
    let p = crate::state::with_state_mut(|st| new_lig_item(&mut st.mem, c));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
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
    let p = get_node(mem, SMALL_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, DISC_NODE);
    mem.set_b16s0(p, 0);
    mem.set_b32s0(p + 1, TEX_NULL); // pre-break list
    mem.set_b32s1(p + 1, TEX_NULL); // post-break list
    p
}

/// Transitional bridge-down wrapper for legacy `new_disc` callers (`01b` §3, §5).
pub unsafe fn new_disc_global() -> i32 {
    let p = crate::state::with_state_mut(|st| new_disc(&mut st.mem));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
}

// ---------------------------------------------------------------------------
// new_math
// ---------------------------------------------------------------------------

/// Allocate a math-on/off node of width `w` and subtype `s`.
///
/// translated from xetex-xetex0.c:221 (new_math)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_math_global`].
pub fn new_math(mem: &mut MemState, w: i32, s: i16) -> i32 {
    let p = get_node(mem, MEDIUM_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, MATH_NODE);
    mem.set_b16s0(p, s as u16);
    mem.set_b32s1(p + 1, w);
    p
}

/// Transitional bridge-down wrapper for legacy `new_math` callers (`01b` §3, §5).
pub unsafe fn new_math_global(w: i32, s: i16) -> i32 {
    let p = crate::state::with_state_mut(|st| new_math(&mut st.mem, w, s));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
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
    let q = get_node(mem, GLUE_SPEC_SIZE);
    if q == ALLOC_FAILED {
        return q;
    }
    let word = mem.word(p);
    mem.set_word(q, word); // copy the whole first word (type + ref_count)
    mem.set_b32s1(q, TEX_NULL); // clear ref_count / link
    let w = mem.b32s1(p + 1);
    mem.set_b32s1(q + 1, w); // width
    let st = mem.b32s1(p + 2);
    mem.set_b32s1(q + 2, st); // stretch
    let sh = mem.b32s1(p + 3);
    mem.set_b32s1(q + 3, sh); // shrink
    q
}

/// Transitional bridge-down wrapper for legacy `new_spec` callers (`01b` §3, §5).
pub unsafe fn new_spec_global(p: i32) -> i32 {
    let q = crate::state::with_state_mut(|st| new_spec(&mut st.mem, p));
    if q == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    q
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
    let p = get_node(mem, MEDIUM_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, GLUE_NODE);
    mem.set_b16s0(p, n as u16 + 1); // subtype = n + 1
    mem.set_b32s1(p + 1, TEX_NULL);
    // q = eqtb[GLUE_BASE + n].b32.s1
    let q = eqtb.b32s1(GLUE_BASE + n as i32);
    mem.set_b32s0(p + 1, q);
    // GLUE_SPEC_ref_count(q)++  →  mem[q].b32.s1 += 1
    let rc = mem.b32s1(q);
    mem.set_b32s1(q, rc + 1);
    p
}

/// Transitional bridge-down wrapper for legacy `new_param_glue` callers (`01b` §3, §5).
pub unsafe fn new_param_glue_global(n: i16) -> i32 {
    let p = crate::state::with_state_mut(|st| {
        let s = &mut *st;
        new_param_glue(&mut s.mem, &s.eqtb, n)
    });
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
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
    let p = get_node(mem, MEDIUM_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, GLUE_NODE);
    // GLUE_SPEC_shrink_order(p) = NORMAL  →  mem[p].b16.s0 = NORMAL
    mem.set_b16s0(p, NORMAL);
    mem.set_b32s1(p + 1, TEX_NULL);
    mem.set_b32s0(p + 1, q);
    // GLUE_SPEC_ref_count(q)++
    let rc = mem.b32s1(q);
    mem.set_b32s1(q, rc + 1);
    p
}

/// Transitional bridge-down wrapper for legacy `new_glue` callers (`01b` §3, §5).
pub unsafe fn new_glue_global(q: i32) -> i32 {
    let p = crate::state::with_state_mut(|st| new_glue(&mut st.mem, q));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
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
    mem.set_b32s1(s, TEX_NULL); // clear the spec's ref_count link
    mem.set_b16s0(p, n as u16 + 1); // subtype = n + 1
    p
}

/// Transitional bridge-down wrapper for legacy `new_skip_param` callers (`01b` §3, §5).
pub unsafe fn new_skip_param_global(n: i16) -> i32 {
    let p = crate::state::with_state_mut(|st| {
        let s = &mut *st;
        new_skip_param(&mut s.mem, &s.eqtb, n)
    });
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
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
    let p = get_node(mem, MEDIUM_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, KERN_NODE);
    mem.set_b16s0(p, NORMAL);
    mem.set_b32s1(p + 1, w);
    p
}

/// Transitional bridge-down wrapper for legacy `new_kern` callers (`01b` §3, §5).
pub unsafe fn new_kern_global(w: i32) -> i32 {
    let p = crate::state::with_state_mut(|st| new_kern(&mut st.mem, w));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
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
    let p = get_node(mem, MEDIUM_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, PENALTY_NODE);
    mem.set_b16s0(p, 0);
    mem.set_b32s1(p + 1, m);
    p
}

/// Transitional bridge-down wrapper for legacy `new_penalty` callers (`01b` §3, §5).
pub unsafe fn new_penalty_global(m: i32) -> i32 {
    let p = crate::state::with_state_mut(|st| new_penalty(&mut st.mem, m));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
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
/// so it is safe under any borrow (`01b` §4). Legacy (un-threaded) callers
/// reach it through the transitional [`free_node_global`] bridge-down wrapper.
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

/// Transitional bridge-down wrapper for legacy (un-threaded) `free_node`
/// callers (`01b` §3). Its arguments are evaluated by the caller *before* the
/// thread-local borrow is taken, so it is free of the argument nest-panic
/// hazard an inline `with_state_mut(|s| …)` closure would have. Removed when
/// the last legacy caller is itself threaded.
pub unsafe fn free_node_global(p: i32, s: i32) {
    crate::state::with_state_mut(|st| free_node(&mut st.mem, p, s));
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
#[no_mangle]
pub unsafe extern "C" fn copy_native_glyph_info(src: i32, dest: i32) {
    let src_ptr = mem_ptr(src + 5);
    if !src_ptr.is_null() {
        let glyph_count = mem_b16s0(src + 4) as usize;
        let byte_count = glyph_count * NATIVE_GLYPH_INFO_SIZE;
        // xmalloc_array(char, glyph_count * NATIVE_GLYPH_INFO_SIZE)
        //   = xmalloc((glyph_count * NATIVE_GLYPH_INFO_SIZE + 1) * sizeof(char))
        let new_ptr = xmalloc(byte_count + 1);
        std::ptr::copy_nonoverlapping(src_ptr as *const u8, new_ptr as *mut u8, byte_count);
        set_mem_ptr(dest + 5, new_ptr);
        set_mem_b16s0(dest + 4, glyph_count as u16);
    }
}

// ---------------------------------------------------------------------------
// delete_token_ref / delete_glue_ref
// ---------------------------------------------------------------------------

/// Decrement the reference count of a token list; free if it reaches TEX_NULL.
///
/// translated from xetex-xetex0.c:976 (delete_token_ref)
pub unsafe fn delete_token_ref(p: i32) {
    if mem_b32s0(p) == TEX_NULL {
        flush_list_global(p);
    } else {
        set_mem_b32s0(p, mem_b32s0(p) - 1);
    }
}

/// Decrement the reference count of a glue spec; free if it reaches TEX_NULL.
///
/// translated from xetex-xetex0.c:984 (delete_glue_ref)
pub unsafe fn delete_glue_ref(p: i32) {
    if mem_b32s1(p) == TEX_NULL {
        free_node_global(p, GLUE_SPEC_SIZE);
    } else {
        set_mem_b32s1(p, mem_b32s1(p) - 1);
    }
}

// ---------------------------------------------------------------------------
// push_nest / pop_nest
// ---------------------------------------------------------------------------

/// Push the current list state onto the nest stack and start a new list.
///
/// translated from xetex-xetex0.c:1323 (push_nest)
pub unsafe fn push_nest() {
    if nest_ptr() > max_nest_stack() {
        set_max_nest_stack(nest_ptr());
        if nest_ptr() == nest_size() {
            crate::errors::overflow(c"semantic nest size".as_ptr(), nest_size());
        }
    }
    save_cur_list_to_nest(nest_ptr());
    set_nest_ptr(nest_ptr() + 1);
    let h = get_avail_global();
    set_cur_list_head(h);
    set_cur_list_tail(h);
    set_cur_list_prev_graf(0);
    set_cur_list_mode_line(line());
    set_cur_list_e_tex_aux(TEX_NULL);
}

/// Pop the top nest entry back into cur_list, freeing the old head token.
///
/// translated from xetex-xetex0.c:1339 (pop_nest)
#[no_mangle]
pub unsafe extern "C" fn pop_nest() {
    // Recycle old cur_list.head into avail.
    set_mem_b32s1(cur_list_head(), avail());
    set_avail(cur_list_head());
    set_nest_ptr(nest_ptr() - 1);
    restore_cur_list_from_nest(nest_ptr());
}

// ---------------------------------------------------------------------------
// prev_rightmost
// ---------------------------------------------------------------------------

/// Return the last node in [s..e) whose successor is e, or TEX_NULL if none.
///
/// translated from xetex-xetex0.c:140 (prev_rightmost)
pub unsafe fn prev_rightmost(s: i32, e: i32) -> i32 {
    let mut p = s;
    if p == TEX_NULL {
        return TEX_NULL;
    }
    while mem_b32s1(p) != e {
        p = mem_b32s1(p);
        if p == TEX_NULL {
            return TEX_NULL;
        }
    }
    p
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
    let p = get_node(mem, NOAD_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, ORD_NOAD);
    mem.set_b16s0(p, NORMAL);
    // mem[p+1].b32 = empty  ↔  s1=EMPTY=0, s0=TEX_NULL
    mem.set_b32s1(p + 1, EMPTY);
    mem.set_b32s0(p + 1, TEX_NULL);
    // mem[p+3].b32 = empty
    mem.set_b32s1(p + 3, EMPTY);
    mem.set_b32s0(p + 3, TEX_NULL);
    // mem[p+2].b32 = empty
    mem.set_b32s1(p + 2, EMPTY);
    mem.set_b32s0(p + 2, TEX_NULL);
    p
}

/// Transitional bridge-down wrapper for legacy `new_noad` callers (`01b` §3, §5).
pub unsafe fn new_noad_global() -> i32 {
    let p = crate::state::with_state_mut(|st| new_noad(&mut st.mem));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
}

/// Allocate a style node (STYLE_NODE_SIZE = 3 words).
///
/// translated from xetex-xetex0.c:10554 (new_style)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_style_global`].
pub fn new_style(mem: &mut MemState, s: i16) -> i32 {
    let p = get_node(mem, STYLE_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, STYLE_NODE); // NODE_type(p)
    mem.set_b16s0(p, s as u16);
    mem.set_b32s1(p + 1, 0);
    mem.set_b32s1(p + 2, 0);
    p
}

/// Transitional bridge-down wrapper for legacy `new_style` callers (`01b` §3, §5).
pub unsafe fn new_style_global(s: i16) -> i32 {
    let p = crate::state::with_state_mut(|st| new_style(&mut st.mem, s));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
}

/// Allocate a choice node (STYLE_NODE_SIZE = 3 words).
///
/// translated from xetex-xetex0.c:10565 (new_choice)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use [`new_choice_global`].
pub fn new_choice(mem: &mut MemState) -> i32 {
    let p = get_node(mem, STYLE_NODE_SIZE);
    if p == ALLOC_FAILED {
        return p;
    }
    mem.set_b16s1(p, CHOICE_NODE); // NODE_type(p)
    mem.set_b16s0(p, 0);
    mem.set_b32s0(p + 1, TEX_NULL);
    mem.set_b32s1(p + 1, TEX_NULL);
    mem.set_b32s0(p + 2, TEX_NULL);
    mem.set_b32s1(p + 2, TEX_NULL);
    p
}

/// Transitional bridge-down wrapper for legacy `new_choice` callers (`01b` §3, §5).
pub unsafe fn new_choice_global() -> i32 {
    let p = crate::state::with_state_mut(|st| new_choice(&mut st.mem));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
}

/// Allocate a margin kern node (MARGIN_KERN_NODE_SIZE = 3 words).
///
/// translated from xetex-xetex0.c:9838 (new_margin_kern)
///
/// Phase-C: threaded to `&mut MemState`.
pub fn new_margin_kern(mem: &mut MemState, w: i32, _p: i32, side: i16) -> i32 {
    let k = get_node(mem, MARGIN_KERN_NODE_SIZE);
    if k == ALLOC_FAILED {
        return k;
    }
    mem.set_b16s1(k, MARGIN_KERN_NODE); // NODE_type(k)
    mem.set_b16s0(k, side as u16);
    mem.set_b32s1(k + 1, w);
    k
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

/// Transitional bridge-down wrapper for legacy `new_whatsit` callers (`01b` §3, §5).
pub unsafe fn new_whatsit_global(s: i16, w: i16) {
    let p = crate::state::with_state_mut(|st| new_whatsit(&mut st.mem, s, w));
    if p == ALLOC_FAILED {
        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
}
