//! Memory arena types and Phase-2 allocator functions translated from
//! `xetex/xetex-xetex0.c`.
//!
//! Phase 2 targets are the node allocation helpers (`get_avail`,
//! `flush_list`, `get_node`, `free_node`, `new_*`). During Phase A of
//! state migration (see `plan/02-state.md`), all arena access goes
//! through the raw C `mem[]` pointer wired up in `src/globals.rs`.

use crate::globals::{
    avail, cur_input_synctex_tag, hi_mem_min, line, lo_mem_max, mem_b32s0, mem_b32s1, mem_end,
    rover, set_avail, set_hi_mem_min, set_lo_mem_max, set_mem_b32s0, set_mem_b32s1, set_mem_end,
    set_rover,
};

// TeX constants (xetex-constants.h)
const TEX_NULL: i32 = -0x0FFF_FFFF; // = MIN_HALFWORD
const MEM_TOP: i32 = 4_999_999;
const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
const MEDIUM_NODE_SIZE: i32 = 3;

// C functions not yet translated — still live in xetex-xetex0.c.
extern "C" {
    fn runaway();
    fn overflow(s: *const libc::c_char, n: i32) -> !;
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

/// Allocate one word from the single-word (hi-mem) free list.
///
/// Returns the index of the allocated word. The word's link field
/// (`mem[p].b32.s1`) is set to `TEX_NULL`. Calls `runaway()` + `overflow()`
/// if memory is exhausted (those functions never return).
///
/// translated from xetex-xetex0.c:170 (get_avail)
#[no_mangle]
pub unsafe extern "C" fn get_avail() -> i32 {
    let p = avail();

    if p != TEX_NULL {
        set_avail(mem_b32s1(avail()));
    } else if mem_end() < MEM_TOP {
        set_mem_end(mem_end() + 1);
        // p already holds old mem_end; we return the incremented value
        let p = mem_end();
        set_mem_b32s1(p, TEX_NULL);
        return p;
    } else {
        set_hi_mem_min(hi_mem_min() - 1);
        let p = hi_mem_min();
        // is_char_node(lo_mem_max) ≡ lo_mem_max >= hi_mem_min
        if lo_mem_max() >= hi_mem_min() {
            runaway();
            overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
        }
        set_mem_b32s1(p, TEX_NULL);
        return p;
    }

    set_mem_b32s1(p, TEX_NULL);
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
#[no_mangle]
pub unsafe extern "C" fn flush_list(p: i32) {
    if p == TEX_NULL {
        return;
    }

    let mut r = p;
    let mut q;
    loop {
        q = r;
        r = mem_b32s1(r);
        if r == TEX_NULL {
            break;
        }
    }
    // q is now the tail; splice: tail.link = old_avail; avail = p
    set_mem_b32s1(q, avail());
    set_avail(p);
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
#[no_mangle]
pub unsafe extern "C" fn get_node(s: i32) -> i32 {
    let r = 'restart: loop {
        let mut p = rover();

        // do { … } while p != rover()
        loop {
            // Coalesce any immediately-following free blocks into one.
            let mut q = p + mem_b32s0(p);
            while mem_b32s1(q) == MAX_HALFWORD {
                let t = mem_b32s1(q + 1);
                if q == rover() {
                    set_rover(t);
                }
                // Unlink q from the doubly-linked free list.
                set_mem_b32s0(t + 1, mem_b32s0(q + 1));
                set_mem_b32s1(mem_b32s0(q + 1) + 1, t);
                q = q + mem_b32s0(q);
            }

            let r = q - s;

            if r > p + 1 {
                // Case 132: block large enough to split; keep left part as free.
                set_mem_b32s0(p, r - p);
                set_rover(p);
                break 'restart r;
            }

            if r == p && mem_b32s1(p + 1) != p {
                // Case 133: exact fit; unlink p from the free list.
                let new_rover = mem_b32s1(p + 1);
                let t = mem_b32s0(p + 1);
                set_rover(new_rover);
                set_mem_b32s0(new_rover + 1, t);
                set_mem_b32s1(t + 1, new_rover);
                break 'restart r;
            }

            set_mem_b32s0(p, q - p);
            p = mem_b32s1(p + 1);
            if p == rover() {
                break;
            }
        }

        // No suitable block found in one pass through the free list.
        if s == 0x4000_0000 {
            return MAX_HALFWORD;
        }

        if lo_mem_max() + 2 < hi_mem_min() && lo_mem_max() + 2 <= MAX_HALFWORD {
            // Grow lo-mem by inserting a new free block between lo_mem_max
            // and hi_mem_min, then retry.
            let t = if hi_mem_min() - lo_mem_max() >= 1998 {
                lo_mem_max() + 1000
            } else {
                lo_mem_max() + 1 + (hi_mem_min() - lo_mem_max()) / 2
            };
            let t = if t > MAX_HALFWORD { MAX_HALFWORD } else { t };

            let left_neighbor = mem_b32s0(rover() + 1);
            let q = lo_mem_max();
            set_mem_b32s1(left_neighbor + 1, q);
            set_mem_b32s0(rover() + 1, q);
            set_mem_b32s1(q + 1, rover());
            set_mem_b32s0(q + 1, left_neighbor);
            set_mem_b32s1(q, MAX_HALFWORD);
            set_mem_b32s0(q, t - lo_mem_max());
            set_lo_mem_max(t);
            set_mem_b32s1(lo_mem_max(), TEX_NULL);
            set_mem_b32s0(lo_mem_max(), TEX_NULL);
            set_rover(q);
            continue 'restart;
        }

        overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    };

    // found:
    set_mem_b32s1(r, TEX_NULL);
    if s >= MEDIUM_NODE_SIZE {
        set_mem_b32s0(r + s - 1, cur_input_synctex_tag());
        set_mem_b32s1(r + s - 1, line());
    }
    r
}

// ---------------------------------------------------------------------------
// free_node
// ---------------------------------------------------------------------------

/// Return a variable-size node of size `s` words (headed at `p`) to the
/// variable-size free list rooted at `rover`.
///
/// translated from xetex-xetex0.c:284 (free_node)
#[no_mangle]
pub unsafe extern "C" fn free_node(p: i32, s: i32) {
    // Record node size and mark it as a free block (MAX_HALFWORD sentinel).
    set_mem_b32s0(p, s);
    set_mem_b32s1(p, MAX_HALFWORD);

    // Insert p between rover and rover's left-neighbor (rover+1.s0).
    let q = mem_b32s0(rover() + 1);
    set_mem_b32s0(p + 1, q);
    set_mem_b32s1(p + 1, rover());
    set_mem_b32s0(rover() + 1, p);
    set_mem_b32s1(q + 1, p);
}
