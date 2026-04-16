//! Memory arena types and Phase-2 allocator functions translated from
//! `xetex/xetex-xetex0.c`.
//!
//! Phase 2 targets are the node allocation helpers (`get_avail`,
//! `flush_list`, `get_node`, `free_node`, `new_*`). During Phase A of
//! state migration (see `plan/02-state.md`), all arena access goes
//! through the raw C `mem[]` pointer wired up in `src/globals.rs`.

use crate::globals::{avail, hi_mem_min, lo_mem_max, mem_b32s1, mem_end, set_avail, set_hi_mem_min, set_mem_b32s1, set_mem_end};

// TeX constants (xetex-constants.h)
const TEX_NULL: i32 = -0x0FFF_FFFF; // = MIN_HALFWORD
const MEM_TOP: i32 = 4_999_999;

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
