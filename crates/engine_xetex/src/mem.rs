//! Newtype indices and semantic unit types for the TeX engine.
//!
//! This module intentionally has **no accessors** for memory words yet —
//! see `plan/03-memory-and-nodes.md` §"Typed node accessors" for when
//! those are added (Phase 2). For Session 1, we only need the unit
//! newtypes so that pure-arithmetic translations can be type-checked.

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
