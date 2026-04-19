//! Memory arena types and Phase-2 allocator functions translated from
//! `xetex/xetex-xetex0.c`.
//!
//! Phase 2 targets are the node allocation helpers (`get_avail`,
//! `flush_list`, `get_node`, `free_node`, `new_*`). During Phase A of
//! state migration (see `plan/02-state.md`), all arena access goes
//! through the raw C `mem[]` pointer wired up in `src/globals.rs`.

use crate::globals::{
    avail, cur_input_synctex_tag, eqtb_b32s1, hi_mem_min, line, lo_mem_max, max_nest_stack,
    mem_b16s0, mem_b32s0, mem_b32s1, mem_end, mem_ptr, mem_word, nest_ptr, nest_size, rover,
    restore_cur_list_from_nest, save_cur_list_to_nest, set_avail, set_cur_list_e_tex_aux,
    set_cur_list_head, set_cur_list_mode_line, set_cur_list_prev_graf, set_cur_list_tail,
    set_hi_mem_min, set_lo_mem_max, set_max_nest_stack, set_mem_b16s0, set_mem_b16s1,
    set_mem_b32s0, set_mem_b32s1, set_mem_end, set_mem_gr, set_mem_ptr, set_mem_word,
    set_nest_ptr, set_rover, set_temp_ptr, temp_ptr, cur_list_head,
};

// TeX constants (xetex-constants.h / xetex_format.h)
pub(crate) const TEX_NULL: i32 = -0x0FFF_FFFF;    // MIN_HALFWORD
const MEM_TOP: i32 = 4_999_999;
const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
const NULL_FLAG: i32 = -0x4000_0000;   // "missing item" in rule nodes
const NORMAL: u16 = 0;
const MEDIUM_NODE_SIZE: i32 = 3;
const SMALL_NODE_SIZE: i32 = 2;
const GLUE_SPEC_SIZE: i32 = 4;
const RULE_NODE_SIZE: i32 = 5;
const BOX_NODE_SIZE: i32 = 8;
// Node type values (mem[p].b16.s1)
const HLIST_NODE: u16 = 0;
const RULE_NODE: u16 = 2;
const LIGATURE_NODE: u16 = 6;
const DISC_NODE: u16 = 7;
const MATH_NODE: u16 = 9;
const GLUE_NODE: u16 = 10;
const KERN_NODE: u16 = 11;
const PENALTY_NODE: u16 = 12;
// eqtb base indices (xetex_format.h)
const GLUE_BASE: i32 = 2_254_340;
// native node glyph info size in bytes (xetex-constants.h:304)
const NATIVE_GLYPH_INFO_SIZE: usize = 10;

// C functions not yet translated — still live in xetex-xetex0.c.
extern "C" {
    fn runaway();
    fn overflow(s: *const libc::c_char, n: i32) -> !;
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

// ---------------------------------------------------------------------------
// new_null_box
// ---------------------------------------------------------------------------

/// Allocate and zero-initialize an empty hbox node (BOX_NODE_SIZE words).
///
/// translated from xetex-xetex0.c:147 (new_null_box)
#[no_mangle]
pub unsafe extern "C" fn new_null_box() -> i32 {
    let p = get_node(BOX_NODE_SIZE);
    // NODE_type(p) = HLIST_NODE  →  mem[p].b16.s1 = HLIST_NODE
    set_mem_b16s1(p, HLIST_NODE);
    set_mem_b16s0(p, 0);           // subtype
    set_mem_b32s1(p + 1, 0);       // width
    set_mem_b32s1(p + 2, 0);       // depth
    set_mem_b32s1(p + 3, 0);       // height
    set_mem_b32s1(p + 4, 0);       // shift_amount
    set_mem_b32s1(p + 5, TEX_NULL); // list_ptr
    set_mem_b16s1(p + 5, NORMAL);  // glue_sign (b16.s1, bytes 2-3 of word 5)
    set_mem_b16s0(p + 5, NORMAL);  // glue_order (b16.s0, bytes 0-1 of word 5)
    set_mem_gr(p + 6, 0.0);        // BOX_glue_set: mem[(p)+6].gr
    p
}

// ---------------------------------------------------------------------------
// new_rule
// ---------------------------------------------------------------------------

/// Allocate a rule node (RULE_NODE_SIZE words) with all dimensions NULL_FLAG.
///
/// translated from xetex-xetex0.c:162 (new_rule)
#[no_mangle]
pub unsafe extern "C" fn new_rule() -> i32 {
    let p = get_node(RULE_NODE_SIZE);
    set_mem_b16s1(p, RULE_NODE);
    set_mem_b16s0(p, 0);
    set_mem_b32s1(p + 1, NULL_FLAG); // width
    set_mem_b32s1(p + 2, NULL_FLAG); // depth
    set_mem_b32s1(p + 3, NULL_FLAG); // height
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
#[no_mangle]
pub unsafe extern "C" fn new_ligature(f: i32, c: u16, q: i32) -> i32 {
    let p = get_node(SMALL_NODE_SIZE);
    set_mem_b16s1(p, LIGATURE_NODE);
    // word p+1: b16.s1 = font, b16.s0 = char, b32.s1 = successor link.
    // b16.s1 and b16.s0 together form b32.s0; b32.s1 is independent.
    set_mem_b16s1(p + 1, f as u16);
    set_mem_b16s0(p + 1, c);
    set_mem_b32s1(p + 1, q);
    set_mem_b16s0(p, 0); // subtype
    p
}

// ---------------------------------------------------------------------------
// new_lig_item
// ---------------------------------------------------------------------------

/// Allocate a lig-item node (SMALL_NODE_SIZE) for char `c`.
///
/// translated from xetex-xetex0.c:186 (new_lig_item)
#[no_mangle]
pub unsafe extern "C" fn new_lig_item(c: u16) -> i32 {
    let p = get_node(SMALL_NODE_SIZE);
    set_mem_b16s0(p, c);
    set_mem_b32s1(p + 1, TEX_NULL);
    p
}

// ---------------------------------------------------------------------------
// new_disc
// ---------------------------------------------------------------------------

/// Allocate a discretionary node (SMALL_NODE_SIZE words) with null lists.
///
/// translated from xetex-xetex0.c:195 (new_disc)
#[no_mangle]
pub unsafe extern "C" fn new_disc() -> i32 {
    let p = get_node(SMALL_NODE_SIZE);
    set_mem_b16s1(p, DISC_NODE);
    set_mem_b16s0(p, 0);
    set_mem_b32s0(p + 1, TEX_NULL); // pre-break list
    set_mem_b32s1(p + 1, TEX_NULL); // post-break list
    p
}

// ---------------------------------------------------------------------------
// new_math
// ---------------------------------------------------------------------------

/// Allocate a math-on/off node of width `w` and subtype `s`.
///
/// translated from xetex-xetex0.c:221 (new_math)
#[no_mangle]
pub unsafe extern "C" fn new_math(w: i32, s: i16) -> i32 {
    let p = get_node(MEDIUM_NODE_SIZE);
    set_mem_b16s1(p, MATH_NODE);
    set_mem_b16s0(p, s as u16);
    set_mem_b32s1(p + 1, w);
    p
}

// ---------------------------------------------------------------------------
// new_spec
// ---------------------------------------------------------------------------

/// Allocate a glue-spec node (GLUE_SPEC_SIZE words) copied from the spec
/// at `p`, then clear the copy's reference-count / link field to TEX_NULL.
///
/// translated from xetex-xetex0.c:231 (new_spec)
#[no_mangle]
pub unsafe extern "C" fn new_spec(p: i32) -> i32 {
    let q = get_node(GLUE_SPEC_SIZE);
    set_mem_word(q, mem_word(p));   // copy the whole first word (type + ref_count)
    set_mem_b32s1(q, TEX_NULL);     // clear ref_count / link
    set_mem_b32s1(q + 1, mem_b32s1(p + 1)); // width
    set_mem_b32s1(q + 2, mem_b32s1(p + 2)); // stretch
    set_mem_b32s1(q + 3, mem_b32s1(p + 3)); // shrink
    q
}

// ---------------------------------------------------------------------------
// new_param_glue
// ---------------------------------------------------------------------------

/// Allocate a glue node referencing parameter `n` from the format's glue
/// parameter table.
///
/// translated from xetex-xetex0.c:243 (new_param_glue)
#[no_mangle]
pub unsafe extern "C" fn new_param_glue(n: i16) -> i32 {
    let p = get_node(MEDIUM_NODE_SIZE);
    set_mem_b16s1(p, GLUE_NODE);
    set_mem_b16s0(p, n as u16 + 1); // subtype = n + 1
    set_mem_b32s1(p + 1, TEX_NULL);
    // q = eqtb[GLUE_BASE + n].b32.s1
    let q = eqtb_b32s1(GLUE_BASE + n as i32);
    set_mem_b32s0(p + 1, q);
    // GLUE_SPEC_ref_count(q)++  →  mem[q].b32.s1 += 1
    set_mem_b32s1(q, mem_b32s1(q) + 1);
    p
}

// ---------------------------------------------------------------------------
// new_glue
// ---------------------------------------------------------------------------

/// Allocate a glue node pointing to the existing glue spec `q`, and
/// bump the spec's reference count.
///
/// translated from xetex-xetex0.c:258 (new_glue)
#[no_mangle]
pub unsafe extern "C" fn new_glue(q: i32) -> i32 {
    let p = get_node(MEDIUM_NODE_SIZE);
    set_mem_b16s1(p, GLUE_NODE);
    // GLUE_SPEC_shrink_order(p) = NORMAL  →  mem[p].b16.s0 = NORMAL
    set_mem_b16s0(p, NORMAL);
    set_mem_b32s1(p + 1, TEX_NULL);
    set_mem_b32s0(p + 1, q);
    // GLUE_SPEC_ref_count(q)++
    set_mem_b32s1(q, mem_b32s1(q) + 1);
    p
}

// ---------------------------------------------------------------------------
// new_skip_param
// ---------------------------------------------------------------------------

/// Allocate a glue node whose spec is a fresh copy of parameter `n`,
/// setting `temp_ptr` to the copy.
///
/// translated from xetex-xetex0.c:270 (new_skip_param)
#[no_mangle]
pub unsafe extern "C" fn new_skip_param(n: i16) -> i32 {
    set_temp_ptr(new_spec(eqtb_b32s1(GLUE_BASE + n as i32)));
    let p = new_glue(temp_ptr());
    set_mem_b32s1(temp_ptr(), TEX_NULL); // clear the spec's ref_count link
    set_mem_b16s0(p, n as u16 + 1);     // subtype = n + 1
    p
}

// ---------------------------------------------------------------------------
// new_kern
// ---------------------------------------------------------------------------

/// Allocate a kern node of width `w`.
///
/// translated from xetex-xetex0.c:281 (new_kern)
#[no_mangle]
pub unsafe extern "C" fn new_kern(w: i32) -> i32 {
    let p = get_node(MEDIUM_NODE_SIZE);
    set_mem_b16s1(p, KERN_NODE);
    set_mem_b16s0(p, NORMAL);
    set_mem_b32s1(p + 1, w);
    p
}

// ---------------------------------------------------------------------------
// new_penalty
// ---------------------------------------------------------------------------

/// Allocate a penalty node with value `m`.
///
/// translated from xetex-xetex0.c:291 (new_penalty)
#[no_mangle]
pub unsafe extern "C" fn new_penalty(m: i32) -> i32 {
    let p = get_node(MEDIUM_NODE_SIZE);
    set_mem_b16s1(p, PENALTY_NODE);
    set_mem_b16s0(p, 0);
    set_mem_b32s1(p + 1, m);
    p
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
#[no_mangle]
pub unsafe extern "C" fn delete_token_ref(p: i32) {
    if mem_b32s0(p) == TEX_NULL {
        flush_list(p);
    } else {
        set_mem_b32s0(p, mem_b32s0(p) - 1);
    }
}

/// Decrement the reference count of a glue spec; free if it reaches TEX_NULL.
///
/// translated from xetex-xetex0.c:984 (delete_glue_ref)
#[no_mangle]
pub unsafe extern "C" fn delete_glue_ref(p: i32) {
    if mem_b32s1(p) == TEX_NULL {
        free_node(p, GLUE_SPEC_SIZE);
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
#[no_mangle]
pub unsafe extern "C" fn push_nest() {
    if nest_ptr() > max_nest_stack() {
        set_max_nest_stack(nest_ptr());
        if nest_ptr() == nest_size() {
            crate::errors::overflow(c"semantic nest size".as_ptr(), nest_size());
        }
    }
    save_cur_list_to_nest(nest_ptr());
    set_nest_ptr(nest_ptr() + 1);
    let h = get_avail();
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
#[no_mangle]
pub unsafe extern "C" fn prev_rightmost(s: i32, e: i32) -> i32 {
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
