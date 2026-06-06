//! Scalar newtypes for the typed memory foundation (`plan/idiom/01c` §2
//! Stage 1; `plan/03-memory-and-nodes.md`).
//!
//! These are the zero-cost semantic wrappers that the Phase-C
//! "type-as-you-thread" slices adopt (`01b` §3a): a `mem` arena index, a
//! nullable TeX halfword, a scaled length, a string number, a font number,
//! and a packed token. Every one is `#[repr(transparent)]` over its C
//! representation and `Copy`, so it compiles to the same load/store as the
//! raw integer (the zero-cost gate, `01c` §4 — exercised for real once the
//! `NodePtr<K>` field methods land in Stage 3).
//!
//! This module is built *before* its consumers exist (the foundation-first
//! prerequisite, `01`-§4a), so its items are intentionally unused for now;
//! see the `#[allow(dead_code)]` on `mod units;` in `lib.rs`.
//!
//! ## Two corrections vs. the vestigial stubs + the parent sketch (`01c` §2)
//!
//! 1. **`TEX_NULL = MIN_HALFWORD = -0x0FFFFFFF`** (`crate::consts::TEX_NULL`,
//!    `xetex-constants.h:14`), *not* `0`. Null-ness lives on [`MemPtr`]
//!    keyed to `TEX_NULL`; [`MemIdx`] has no null. (The earlier
//!    `mem.rs` `MemIdx::NULL = MemIdx(0)` and the `03-memory-and-nodes.md`
//!    `MemPtr::NULL = MemPtr(0)` sketch were both wrong for this codebase.)
//! 2. **`FontId` wraps `i32`**, not `u16`: `internal_font_number` is
//!    `int32_t` (`xetex-xetexd.h:326`). Do not narrow before a dedicated
//!    audit (`01`-§4).

use crate::consts::TEX_NULL;

/// A **valid** index into the `mem` arena (`mem.data[idx]`).
///
/// A `MemIdx` never holds `TEX_NULL` or any other negative sentinel — that
/// is what distinguishes it from [`MemPtr`]. Nullable TeX halfwords are
/// [`MemPtr`] until checked and converted with [`MemPtr::as_idx`].
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(transparent)]
pub struct MemIdx(pub i32);

impl MemIdx {
    /// Wrap a known-valid arena index. In debug builds this asserts the
    /// value is not the `TEX_NULL` sentinel (the caller must have already
    /// classified it as a real index — `01c` §3).
    #[inline]
    pub fn new(idx: i32) -> MemIdx {
        debug_assert!(idx != TEX_NULL, "MemIdx must not hold TEX_NULL");
        MemIdx(idx)
    }

    /// The `k`-th word after this node base (`mem[(p) + delta]`). Stays a
    /// valid index because `delta` is a fixed field offset within a node.
    #[inline]
    pub fn offset(self, delta: i32) -> MemIdx {
        MemIdx(self.0 + delta)
    }

    /// The raw index as a `usize` for slice indexing into `mem.data`.
    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A nullable TeX halfword: either a real arena index or `TEX_NULL`.
///
/// TeX's "pointer" fields (`link`, `glue_ptr`, `list_ptr`, …) store either
/// a `mem` index or `TEX_NULL = -0x0FFFFFFF`. Classification to a real
/// index happens once, at a checked boundary (`01c` §3), via [`as_idx`].
///
/// [`as_idx`]: MemPtr::as_idx
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(transparent)]
pub struct MemPtr(pub i32);

impl MemPtr {
    /// The TeX null pointer (`TEX_NULL = MIN_HALFWORD = -0x0FFFFFFF`).
    pub const NULL: MemPtr = MemPtr(TEX_NULL);

    /// Wrap a raw halfword that may be `TEX_NULL` or a real index.
    #[inline]
    pub fn from_raw(raw: i32) -> MemPtr {
        MemPtr(raw)
    }

    /// Is this the TeX null pointer?
    #[inline]
    pub fn is_null(self) -> bool {
        self == Self::NULL
    }

    /// Convert to a valid [`MemIdx`], or `None` if null.
    ///
    /// This is the single checked boundary where a nullable halfword
    /// becomes a real index — never construct a `MemIdx` directly from a
    /// field that can hold a sentinel (`01`-§10 sentinel hazard).
    #[inline]
    pub fn as_idx(self) -> Option<MemIdx> {
        if self.is_null() {
            None
        } else {
            Some(MemIdx(self.0))
        }
    }
}

/// TeX's basic length unit: 1/65536 of a point (`scaled_t`, an `int32_t`).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct Scaled(pub i32);

/// An index into the string pool (`str_number`).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(transparent)]
pub struct StrNumber(pub i32);

/// An internal font number (`internal_font_number`, an `int32_t` —
/// `xetex-xetexd.h:326`; kept `i32`, not narrowed to `u16`).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(transparent)]
pub struct FontId(pub i32);

/// A packed `(cmd, chr)` token, as stored in `cur_tok` and on the input
/// stack.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(transparent)]
pub struct Token(pub i32);

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    /// `#[repr(transparent)]` guarantee: every newtype is exactly its inner
    /// representation, so it is zero-cost vs. the raw integer (`01c` §4).
    #[test]
    fn newtypes_are_zero_cost_repr() {
        assert_eq!(size_of::<MemIdx>(), size_of::<i32>());
        assert_eq!(size_of::<MemPtr>(), size_of::<i32>());
        assert_eq!(size_of::<Scaled>(), size_of::<i32>());
        assert_eq!(size_of::<StrNumber>(), size_of::<i32>());
        assert_eq!(size_of::<FontId>(), size_of::<i32>());
        assert_eq!(size_of::<Token>(), size_of::<i32>());
        // Option<MemIdx> is *not* niche-optimized (i32 has no niche), but
        // MemPtr carries null in-band, so the classified path stays i32.
        assert_eq!(size_of::<MemPtr>(), size_of::<i32>());
    }

    #[test]
    fn memptr_null_is_tex_null_not_zero() {
        assert_eq!(MemPtr::NULL.0, TEX_NULL);
        assert_ne!(MemPtr::NULL.0, 0);
        assert!(MemPtr::NULL.is_null());
        assert!(MemPtr::NULL.as_idx().is_none());
        // zero is a real arena index, not null
        assert!(!MemPtr::from_raw(0).is_null());
        assert_eq!(MemPtr::from_raw(0).as_idx(), Some(MemIdx(0)));
    }

    #[test]
    fn memidx_offset_walks_node_fields() {
        let p = MemIdx::new(100);
        assert_eq!(p.offset(1), MemIdx(101));
        assert_eq!(p.offset(5).index(), 105);
    }
}
