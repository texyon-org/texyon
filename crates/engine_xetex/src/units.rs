//! Scalar newtypes for the typed memory foundation.
//!
//! These are the zero-cost semantic wrappers: a `mem` arena index, a
//! nullable TeX halfword, a scaled length, a string number, a font number,
//! and a packed token. Every one is `#[repr(transparent)]` over its C
//! representation and `Copy`, so it compiles to the same load/store as the
//! raw integer.
//!
//! 1. **`TEX_NULL = MIN_HALFWORD = -0x0FFFFFFF`** (`crate::consts::TEX_NULL`,
//!    `xetex-constants.h:14`), *not* `0`. Null-ness lives on [`MemPtr`]
//!    keyed to `TEX_NULL`; [`MemIdx`] has no null.
//! 2. **`FontId` wraps `i32`**, not `u16`: `internal_font_number` is
//!    `int32_t` (`xetex-xetexd.h:326`). Do not narrow before a dedicated
//!    audit.

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
    /// classified it as a real index).
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
/// index happens once, at a checked boundary, via [`as_idx`].
///
/// [`as_idx`]: MemPtr::as_idx
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Default)]
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

    /// A non-null pointer to a known-valid node. The idiomatic way to store a
    /// typed node handle (`p.idx()`) into another node's pointer field without
    /// dropping back to a raw `i32`.
    #[inline]
    pub fn from_idx(idx: MemIdx) -> MemPtr {
        MemPtr(idx.0)
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
    /// field that can hold a sentinel.
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

/// The kind of a node, as stored in `mem[p].b16.s1` (the `NODE_type`
/// field). Codes mirror `xetex/xetex_format.h:380–416` exactly.
///
/// This is the *first* classification layer. Two subtleties:
///
/// - **Context aliases, not separate codes.** `DELTA_NODE` reuses
///   [`Rule`](NodeType::Rule)'s code `2` and `EDGE_NODE` reuses
///   [`Style`](NodeType::Style)'s code `14` (`xetex-constants.h:104–105`).
///   They are disambiguated by *list context* (linebreaker / bidi), not by
///   the type field, so they are not distinct discriminants here.
/// - **Whatsit subtypes are a second layer.** A [`Whatsit`](NodeType::Whatsit)
///   node's `NODE_subtype` (`b16.s0`) selects `OPEN_NODE`/`WRITE_NODE`/…/
///   `NATIVE_WORD_NODE`(40)/`GLYPH_NODE`(42)/`PIC_NODE`(43)/`PDF_NODE`(44)
///   (`xetex_format.h:457–467`). Those are *subtype* codes in a different
///   field, classified separately, and must not
///   be confused with the `MARGIN_KERN_NODE`(40) *type* code below.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(u16)]
pub enum NodeType {
    HList = 0,
    VList = 1,
    Rule = 2, // also DELTA_NODE (context-disambiguated)
    Ins = 3,
    Mark = 4,
    Adjust = 5,
    Ligature = 6,
    Disc = 7,
    Whatsit = 8,
    Math = 9,
    Glue = 10,
    Kern = 11,
    Penalty = 12,
    Unset = 13,
    Style = 14, // also EDGE_NODE (context-disambiguated)
    Choice = 15,
    OrdNoad = 16,
    OpNoad = 17,
    BinNoad = 18,
    RelNoad = 19,
    OpenNoad = 20,
    CloseNoad = 21,
    PunctNoad = 22,
    InnerNoad = 23,
    RadicalNoad = 24,
    FractionNoad = 25,
    UnderNoad = 26,
    OverNoad = 27,
    AccentNoad = 28,
    VcenterNoad = 29,
    LeftNoad = 30,
    RightNoad = 31,
    MarginKern = 40,
}

/// Error from [`NodeType::from_raw`]: the `b16.s1` value is not a valid
/// node-type code (e.g. the 32–39 gap, or 41+).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct InvalidNodeType(pub u16);

impl NodeType {
    /// Convert a raw `mem[p].b16.s1` value to a [`NodeType`] with a
    /// **checked** path (never `transmute` — an invalid discriminant is
    /// Rust UB).
    #[inline]
    pub fn from_raw(raw: u16) -> Result<NodeType, InvalidNodeType> {
        use NodeType::*;
        Ok(match raw {
            0 => HList,
            1 => VList,
            2 => Rule,
            3 => Ins,
            4 => Mark,
            5 => Adjust,
            6 => Ligature,
            7 => Disc,
            8 => Whatsit,
            9 => Math,
            10 => Glue,
            11 => Kern,
            12 => Penalty,
            13 => Unset,
            14 => Style,
            15 => Choice,
            16 => OrdNoad,
            17 => OpNoad,
            18 => BinNoad,
            19 => RelNoad,
            20 => OpenNoad,
            21 => CloseNoad,
            22 => PunctNoad,
            23 => InnerNoad,
            24 => RadicalNoad,
            25 => FractionNoad,
            26 => UnderNoad,
            27 => OverNoad,
            28 => AccentNoad,
            29 => VcenterNoad,
            30 => LeftNoad,
            31 => RightNoad,
            40 => MarginKern,
            other => return Err(InvalidNodeType(other)),
        })
    }

    /// The raw `b16.s1` code for this node type (zero-cost: `self as u16`).
    #[inline]
    pub fn as_raw(self) -> u16 {
        self as u16
    }
}

/// A box's glue-set sign (`BOX_glue_sign` = `mem[p+5].b16.s1`): whether the
/// box's glue is left at natural size, stretched, or shrunk. Only these three
/// codes are ever stored, so [`from_raw`](Self::from_raw) is infallible.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum GlueSign {
    Normal = 0,
    Stretching = 1,
    Shrinking = 2,
}

impl GlueSign {
    /// Convert a raw `b16.s1` glue-sign value. Any value other than
    /// `1`/`2` maps to `Normal` (TeX only ever stores 0/1/2).
    #[inline]
    pub fn from_raw(raw: u16) -> GlueSign {
        match raw {
            1 => GlueSign::Stretching,
            2 => GlueSign::Shrinking,
            _ => GlueSign::Normal,
        }
    }

    /// The raw `b16.s1` code for this glue sign (zero-cost: `self as u16`).
    #[inline]
    pub fn as_raw(self) -> u16 {
        self as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    /// `#[repr(transparent)]` guarantee: every newtype is exactly its inner
    /// representation, so it is zero-cost vs. the raw integer.
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

    /// Every discriminant must equal the C `NODE_type` code
    /// (`xetex_format.h:380–416`), and `from_raw` must round-trip it.
    #[test]
    fn nodetype_codes_match_c_and_roundtrip() {
        use NodeType::*;
        let all = [
            (HList, 0u16),
            (VList, 1),
            (Rule, 2),
            (Ins, 3),
            (Mark, 4),
            (Adjust, 5),
            (Ligature, 6),
            (Disc, 7),
            (Whatsit, 8),
            (Math, 9),
            (Glue, 10),
            (Kern, 11),
            (Penalty, 12),
            (Unset, 13),
            (Style, 14),
            (Choice, 15),
            (OrdNoad, 16),
            (OpNoad, 17),
            (BinNoad, 18),
            (RelNoad, 19),
            (OpenNoad, 20),
            (CloseNoad, 21),
            (PunctNoad, 22),
            (InnerNoad, 23),
            (RadicalNoad, 24),
            (FractionNoad, 25),
            (UnderNoad, 26),
            (OverNoad, 27),
            (AccentNoad, 28),
            (VcenterNoad, 29),
            (LeftNoad, 30),
            (RightNoad, 31),
            (MarginKern, 40),
        ];
        for (nt, code) in all {
            assert_eq!(nt.as_raw(), code);
            assert_eq!(NodeType::from_raw(code), Ok(nt));
        }
        // MARGIN_KERN_NODE type code is 40; the whatsit *subtype* codes
        // (GLYPH_NODE 42, PIC_NODE 43, PDF_NODE 44) are NOT node types.
        assert_eq!(NodeType::from_raw(40), Ok(MarginKern));
    }

    #[test]
    fn nodetype_from_raw_rejects_invalid() {
        // the 32..=39 gap, and 41+ (incl. whatsit subtype codes 42/43/44)
        for raw in [32u16, 35, 39, 41, 42, 43, 44, 100, u16::MAX] {
            assert_eq!(NodeType::from_raw(raw), Err(InvalidNodeType(raw)));
        }
    }

    #[test]
    fn nodetype_is_two_bytes() {
        assert_eq!(size_of::<NodeType>(), size_of::<u16>());
    }
}
