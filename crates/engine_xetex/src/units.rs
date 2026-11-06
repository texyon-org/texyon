//! Zero-cost scalar newtypes: arena indices and pointers, scaled lengths,
//! font numbers, and the node-type and glue-sign codes.

use crate::consts::TEX_NULL;

/// A valid, non-null index into the `mem` arena.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(transparent)]
pub struct MemIdx(i32);

impl MemIdx {
    /// The value from its raw form in a memory word, file or FFI call.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> MemIdx {
        MemIdx(raw)
    }

    /// The raw form for a memory word, file or FFI call.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl MemIdx {
    #[inline]
    pub fn new(idx: impl MemAddr) -> MemIdx {
        let idx = idx.addr();
        debug_assert!(idx != TEX_NULL, "MemIdx must not hold TEX_NULL");
        MemIdx(idx)
    }

    #[inline]
    pub fn offset(self, delta: i32) -> MemIdx {
        MemIdx(self.0 + delta)
    }
}

/// A TeX pointer: an arena index or `TEX_NULL` (which is not zero).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct MemPtr(i32);

impl MemPtr {
    /// The value from its raw form in a memory word, file or FFI call.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> MemPtr {
        MemPtr(raw)
    }

    /// The raw form for a memory word, file or FFI call.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl std::ops::Add<i32> for MemIdx {
    type Output = MemIdx;
    #[inline]
    fn add(self, rhs: i32) -> MemIdx {
        self.offset(rhs)
    }
}

impl std::ops::Sub<i32> for MemIdx {
    type Output = MemIdx;
    #[inline]
    fn sub(self, rhs: i32) -> MemIdx {
        self.offset(-rhs)
    }
}

/// The number of words from `rhs` to `self`.
impl std::ops::Sub for MemPtr {
    type Output = i32;
    #[inline]
    fn sub(self, rhs: MemPtr) -> i32 {
        self.0 - rhs.0
    }
}

impl std::ops::Sub<i32> for MemPtr {
    type Output = MemPtr;
    #[inline]
    fn sub(self, rhs: i32) -> MemPtr {
        MemPtr(self.0 - rhs)
    }
}

impl std::ops::AddAssign<i32> for MemPtr {
    #[inline]
    fn add_assign(&mut self, rhs: i32) {
        self.0 += rhs;
    }
}

impl std::ops::SubAssign<i32> for MemPtr {
    #[inline]
    fn sub_assign(&mut self, rhs: i32) {
        self.0 -= rhs;
    }
}

impl std::ops::Add<i32> for MemPtr {
    type Output = MemPtr;
    #[inline]
    fn add(self, rhs: i32) -> MemPtr {
        MemPtr(self.0 + rhs)
    }
}

/// Anything the raw word accessors accept as an arena address.
pub trait MemAddr {
    fn addr(self) -> i32;
}
impl MemAddr for MemPtr {
    #[inline]
    fn addr(self) -> i32 {
        self.0
    }
}
impl MemAddr for MemIdx {
    #[inline]
    fn addr(self) -> i32 {
        self.0
    }
}

impl MemPtr {
    pub const NULL: MemPtr = MemPtr(TEX_NULL);
    /// The lowest arena address, TeX's `mem_bot`.
    pub const MEM_BOT: MemPtr = MemPtr(0);

    #[inline]
    pub fn from_idx(idx: MemIdx) -> MemPtr {
        MemPtr(idx.0)
    }

    #[inline]
    pub fn is_null(self) -> bool {
        self == Self::NULL
    }

    #[inline]
    pub fn as_idx(self) -> Option<MemIdx> {
        if self.is_null() {
            None
        } else {
            Some(MemIdx(self.0))
        }
    }
}

/// A length in units of 2^-16 pt.
///
/// Arithmetic wraps like TeX's 32-bit integers: sums of legal dimensions can
/// exceed `max_dimen`, and TeX checks overflow only where WEB says so.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct Scaled(i32);

impl Scaled {
    /// The value from its raw form in a memory word, file or FFI call.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> Scaled {
        Scaled(raw)
    }

    /// The raw form for a memory word, file or FFI call.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl Scaled {
    pub const ZERO: Scaled = Scaled(0);
    /// One point, TeX's `unity`.
    pub const UNITY: Scaled = Scaled(0x10000);

    /// `r` scaled points rounded to the nearest, as TeX's `round`.
    #[inline]
    pub fn round(r: f64) -> Scaled {
        Scaled(crate::scaled_math::tex_round(r))
    }

    /// TeX's `half`, rounding odd values up.
    #[inline]
    pub fn half(self) -> Scaled {
        Scaled(crate::scaled_math::half(self.0))
    }

    #[inline]
    pub fn abs(self) -> Scaled {
        Scaled(self.0.wrapping_abs())
    }

    #[inline]
    pub fn signum(self) -> i32 {
        self.0.signum()
    }
}

impl std::ops::Add for Scaled {
    type Output = Scaled;
    #[inline(always)]
    fn add(self, rhs: Scaled) -> Scaled {
        Scaled(self.0.wrapping_add(rhs.0))
    }
}

impl std::ops::Sub for Scaled {
    type Output = Scaled;
    #[inline(always)]
    fn sub(self, rhs: Scaled) -> Scaled {
        Scaled(self.0.wrapping_sub(rhs.0))
    }
}

impl std::ops::Neg for Scaled {
    type Output = Scaled;
    #[inline(always)]
    fn neg(self) -> Scaled {
        Scaled(self.0.wrapping_neg())
    }
}

impl std::ops::Mul<i32> for Scaled {
    type Output = Scaled;
    #[inline(always)]
    fn mul(self, rhs: i32) -> Scaled {
        Scaled(self.0.wrapping_mul(rhs))
    }
}

impl std::ops::Div<i32> for Scaled {
    type Output = Scaled;
    #[inline(always)]
    fn div(self, rhs: i32) -> Scaled {
        Scaled(self.0.wrapping_div(rhs))
    }
}

impl std::ops::AddAssign for Scaled {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Scaled) {
        *self = *self + rhs;
    }
}

impl std::ops::SubAssign for Scaled {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Scaled) {
        *self = *self - rhs;
    }
}

impl From<Scaled> for f64 {
    #[inline(always)]
    fn from(s: Scaled) -> f64 {
        f64::from(s.0)
    }
}

impl std::iter::Sum for Scaled {
    fn sum<I: Iterator<Item = Scaled>>(iter: I) -> Scaled {
        iter.fold(Scaled::ZERO, |a, b| a + b)
    }
}

/// A location in `eqtb`: a control sequence, or a glue, token, box, code,
/// integer or dimension entry.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct EqtbPtr(i32);

impl EqtbPtr {
    /// `cur_cs` of a character token.
    pub const NONE: EqtbPtr = EqtbPtr(0);

    /// The value from its raw form in a memory word, file or FFI call.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> EqtbPtr {
        EqtbPtr(raw)
    }

    /// The raw form for a memory word, file or FFI call.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// The location `n` entries past `self`, usable in constants.
    #[inline(always)]
    pub const fn plus(self, n: i32) -> EqtbPtr {
        EqtbPtr(self.0 + n)
    }

    #[inline(always)]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    #[inline(always)]
    pub fn is_none(self) -> bool {
        self.0 == 0
    }

    /// The locations from `self` up to but not including `end`.
    pub fn until(self, end: EqtbPtr) -> impl DoubleEndedIterator<Item = EqtbPtr> {
        (self.0..end.0).map(EqtbPtr)
    }

    /// The locations from `self` up to and including `end`.
    pub fn through(self, end: EqtbPtr) -> impl DoubleEndedIterator<Item = EqtbPtr> {
        (self.0..=end.0).map(EqtbPtr)
    }
}

impl std::ops::Add<i32> for EqtbPtr {
    type Output = EqtbPtr;
    #[inline(always)]
    fn add(self, rhs: i32) -> EqtbPtr {
        EqtbPtr(self.0 + rhs)
    }
}

impl std::ops::Sub<i32> for EqtbPtr {
    type Output = EqtbPtr;
    #[inline(always)]
    fn sub(self, rhs: i32) -> EqtbPtr {
        EqtbPtr(self.0 - rhs)
    }
}

impl std::ops::AddAssign<i32> for EqtbPtr {
    #[inline(always)]
    fn add_assign(&mut self, rhs: i32) {
        self.0 += rhs;
    }
}

impl std::ops::SubAssign<i32> for EqtbPtr {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: i32) {
        self.0 -= rhs;
    }
}

/// The number of entries from `rhs` to `self`.
impl std::ops::Sub for EqtbPtr {
    type Output = i32;
    #[inline(always)]
    fn sub(self, rhs: EqtbPtr) -> i32 {
        self.0 - rhs.0
    }
}

/// An internal font number.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct FontId(i32);

impl FontId {
    /// The value from its raw form in a memory word, file or FFI call.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> FontId {
        FontId(raw)
    }

    /// The raw form for a memory word, file or FFI call.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl FontId {
    #[inline(always)]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// The `eqtb` location of the font's identifier, `font_id_base + f`.
    #[inline]
    pub fn id_loc(self) -> EqtbPtr {
        crate::consts::FONT_ID_BASE + self.0
    }

    /// The font's number in the DVI file, `f - font_base - 1`.
    #[inline]
    pub fn dvi_number(self) -> i32 {
        self.0 - crate::consts::FONT_BASE - 1
    }

    #[inline]
    pub fn next(self) -> FontId {
        FontId(self.0 + 1)
    }

    /// The fonts after `null_font` up to `self`, the loaded ones when `self` is `font_ptr`.
    pub fn loaded(self) -> impl DoubleEndedIterator<Item = FontId> {
        (crate::consts::FONT_BASE + 1..=self.0).map(FontId)
    }
}

/// A string number: a single character below `TOO_BIG_CHAR`, else a pool string.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct StrNumber(i32);

impl StrNumber {
    /// The value from its raw form in a memory word, file or FFI call.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> StrNumber {
        StrNumber(raw)
    }

    /// The raw form for a memory word, file or FFI call.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl StrNumber {
    /// `0`, which TeX uses for "no string" in `job_name` and friends.
    pub const NONE: StrNumber = StrNumber(0);

    /// Whether this is a pool string rather than a single character.
    #[inline]
    pub fn is_pool_string(self) -> bool {
        self.0 >= crate::consts::TOO_BIG_CHAR
    }

    /// The `str_start` slot of this pool string.
    #[inline]
    pub fn slot(self) -> usize {
        (self.0 - crate::consts::TOO_BIG_CHAR) as usize
    }

    #[inline]
    pub fn next(self) -> StrNumber {
        StrNumber(self.0 + 1)
    }
}

/// A node's type code. Delta nodes reuse the rule code and edge nodes the
/// style code, their list context tells them apart.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(u16)]
pub enum NodeType {
    HList = 0,
    VList = 1,
    Rule = 2,
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
    Style = 14,
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

/// A type code outside the node types, as in the 32..=39 gap.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct InvalidNodeType(pub u16);

impl NodeType {
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

    #[inline]
    pub const fn as_raw(self) -> u16 {
        self as u16
    }
}

/// Whether a box's glue is at natural size, stretched or shrunk.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum GlueSign {
    Normal = 0,
    Stretching = 1,
    Shrinking = 2,
}

impl GlueSign {
    #[inline]
    pub fn from_raw(raw: u16) -> GlueSign {
        match raw {
            1 => GlueSign::Stretching,
            2 => GlueSign::Shrinking,
            _ => GlueSign::Normal,
        }
    }

    #[inline]
    pub const fn as_raw(self) -> u16 {
        self as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn newtypes_are_zero_cost_repr() {
        assert_eq!(size_of::<MemIdx>(), size_of::<i32>());
        assert_eq!(size_of::<MemPtr>(), size_of::<i32>());
        assert_eq!(size_of::<Scaled>(), size_of::<i32>());
        assert_eq!(size_of::<FontId>(), size_of::<i32>());
    }

    #[test]
    fn memptr_null_is_tex_null_not_zero() {
        assert_eq!(MemPtr::NULL.0, TEX_NULL);
        assert_ne!(MemPtr::NULL.0, 0);
        assert!(MemPtr::NULL.is_null());
        assert!(MemPtr::NULL.as_idx().is_none());
        assert!(!MemPtr::MEM_BOT.is_null());
        assert_eq!(MemPtr::MEM_BOT.as_idx(), Some(MemIdx(0)));
    }

    #[test]
    fn memidx_offset_walks_node_fields() {
        let p = MemIdx::new(MemPtr::from_raw(100));
        assert_eq!(p.offset(1), MemIdx(101));
        assert_eq!(p.offset(5).0 as usize, 105);
    }

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
    }

    #[test]
    fn nodetype_from_raw_rejects_invalid() {
        for raw in [32u16, 35, 39, 41, 42, 43, 44, 100, u16::MAX] {
            assert_eq!(NodeType::from_raw(raw), Err(InvalidNodeType(raw)));
        }
    }

    #[test]
    fn nodetype_is_two_bytes() {
        assert_eq!(size_of::<NodeType>(), size_of::<u16>());
    }
}
