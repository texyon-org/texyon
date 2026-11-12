//! Zero-cost scalar newtypes: arena indices and pointers, scaled lengths,
//! font numbers, and the node-type and glue-sign codes.

use std::num::NonZeroU32;

/// A non-null index into the `mem` arena. Word 0 is never a node, so a null
/// pointer is the zero word of an `Option<MemIdx>`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(transparent)]
pub struct MemIdx(NonZeroU32);

// SAFETY: `MemIdx` is transparent over `NonZeroU32`, so `None` is the zero word.
unsafe impl bytemuck::ZeroableInOption for MemIdx {}
// SAFETY: as above, and every non-zero word is a valid `MemIdx`.
unsafe impl bytemuck::PodInOption for MemIdx {}

/// A TeX pointer: a `mem` index, `None` for null.
pub type MemPtr = Option<MemIdx>;

impl MemIdx {
    /// The pointer a raw memory word, file or FFI value holds, `None` for null.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> MemPtr {
        if raw < 0 {
            return None;
        }
        match NonZeroU32::new(raw.cast_unsigned()) {
            Some(n) => Some(MemIdx(n)),
            None => None,
        }
    }

    /// The raw form for a memory word, file or FFI call.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0.get().cast_signed()
    }

    /// The `len` words starting here, as a range of `mem` indices.
    #[inline]
    pub fn words(self, len: i32) -> std::ops::Range<usize> {
        // An index past any arena, so slicing rejects it. Staying in i32 range
        // lets the compiler fold a constant-length slice check.
        const PAST_ANY_ARENA: usize = 0x7FFF_FFFF;
        let start = usize::try_from(self.0.get()).unwrap_or(PAST_ANY_ARENA);
        start..start + usize::try_from(len).unwrap_or(0)
    }

    /// The first word after the unused word 0.
    pub const FIRST: MemIdx = MemIdx(NonZeroU32::MIN);

    /// The word `delta` words on from here, within the same node.
    #[inline]
    pub const fn offset(self, delta: u32) -> MemIdx {
        MemIdx(self.0.saturating_add(delta))
    }
}

/// Anything that names a non-null `mem` address.
pub trait MemAddr {
    /// The address.
    fn addr(self) -> MemIdx;
}

impl MemAddr for MemIdx {
    #[inline(always)]
    fn addr(self) -> MemIdx {
        self
    }
}

/// The raw word form of a pointer, 0 for null.
pub trait RawPtr {
    /// The raw form for a memory word, file or FFI call.
    fn raw(self) -> i32;
}

impl RawPtr for MemPtr {
    #[inline(always)]
    fn raw(self) -> i32 {
        self.map_or(0, MemIdx::raw)
    }
}

/// The number of words from `rhs` to `self`.
impl std::ops::Sub for MemIdx {
    type Output = i32;
    #[inline]
    fn sub(self, rhs: MemIdx) -> i32 {
        self.raw() - rhs.raw()
    }
}

/// A length in units of 2^-16 pt.
///
/// Arithmetic wraps like TeX's 32-bit integers: sums of legal dimensions can
/// exceed `max_dimen`, and TeX checks overflow only where WEB says so.
#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Hash,
    Default,
    bytemuck::Pod,
    bytemuck::Zeroable,
)]
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

impl std::ops::Rem<i32> for Scaled {
    type Output = Scaled;
    #[inline(always)]
    fn rem(self, rhs: i32) -> Scaled {
        Scaled(self.0 % rhs)
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

/// A position in the input `buffer`. It can sit one before a line, as an empty
/// line's `limit` does.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct BufPos(i32);

impl BufPos {
    pub const ZERO: BufPos = BufPos(0);

    /// The value from its raw form in a memory word or a size setting.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> BufPos {
        BufPos(raw)
    }

    /// The raw form for a memory word.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0
    }

    #[inline(always)]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// The positions from `self` up to but not including `end`.
    pub fn until(self, end: BufPos) -> impl DoubleEndedIterator<Item = BufPos> + Clone {
        (self.0..end.0).map(BufPos)
    }
}

impl std::ops::Add<i32> for BufPos {
    type Output = BufPos;
    #[inline(always)]
    fn add(self, rhs: i32) -> BufPos {
        BufPos(self.0 + rhs)
    }
}

impl std::ops::Sub<i32> for BufPos {
    type Output = BufPos;
    #[inline(always)]
    fn sub(self, rhs: i32) -> BufPos {
        BufPos(self.0 - rhs)
    }
}

/// The number of positions from `rhs` to `self`.
impl std::ops::Sub for BufPos {
    type Output = i32;
    #[inline(always)]
    fn sub(self, rhs: BufPos) -> i32 {
        self.0 - rhs.0
    }
}

impl std::ops::AddAssign<i32> for BufPos {
    #[inline(always)]
    fn add_assign(&mut self, rhs: i32) {
        self.0 += rhs;
    }
}

impl std::ops::SubAssign<i32> for BufPos {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: i32) {
        self.0 -= rhs;
    }
}

/// A position in `str_pool`, the UTF-16 units of all pool strings.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct PoolPos(i32);

impl PoolPos {
    pub const ZERO: PoolPos = PoolPos(0);

    /// The value from its raw form in a format file or a size setting.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> PoolPos {
        PoolPos(raw)
    }

    /// The raw form for a format file.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0
    }

    #[inline(always)]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// The positions from `self` up to but not including `end`.
    pub fn until(self, end: PoolPos) -> impl DoubleEndedIterator<Item = PoolPos> + Clone {
        (self.0..end.0).map(PoolPos)
    }
}

impl std::ops::Add<i32> for PoolPos {
    type Output = PoolPos;
    #[inline(always)]
    fn add(self, rhs: i32) -> PoolPos {
        PoolPos(self.0 + rhs)
    }
}

impl std::ops::Sub<i32> for PoolPos {
    type Output = PoolPos;
    #[inline(always)]
    fn sub(self, rhs: i32) -> PoolPos {
        PoolPos(self.0 - rhs)
    }
}

/// The number of units from `rhs` to `self`.
impl std::ops::Sub for PoolPos {
    type Output = i32;
    #[inline(always)]
    fn sub(self, rhs: PoolPos) -> i32 {
        self.0 - rhs.0
    }
}

impl std::ops::AddAssign<i32> for PoolPos {
    #[inline(always)]
    fn add_assign(&mut self, rhs: i32) {
        self.0 += rhs;
    }
}

impl std::ops::SubAssign<i32> for PoolPos {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: i32) {
        self.0 -= rhs;
    }
}

/// A primitive's slot in the `prim` table, which `\\primitive` looks names up in.
/// Slot 0 is TeX's end of a chain, `None` here.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(transparent)]
pub struct PrimNum(NonZeroU32);

impl PrimNum {
    pub const FIRST: PrimNum = PrimNum(NonZeroU32::MIN);
    /// `prim_size`, the last slot.
    pub const LAST: PrimNum = PrimNum::FIRST.plus(crate::consts::PRIM_SIZE.cast_unsigned() - 1);

    /// The slot `n` past `self`.
    #[inline(always)]
    pub const fn plus(self, n: u32) -> PrimNum {
        PrimNum(self.0.saturating_add(n))
    }

    #[inline(always)]
    pub fn index(self) -> usize {
        self.0.get() as usize
    }

    /// The raw form for a `prim` link.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0.get().cast_signed()
    }

    /// The slot a raw `prim` link holds, `None` for 0.
    #[inline(always)]
    pub fn from_raw(raw: i32) -> Option<PrimNum> {
        NonZeroU32::new(u32::try_from(raw).ok()?).map(PrimNum)
    }

    /// The slot before `self`, `None` before the first.
    #[inline]
    pub fn prev(self) -> Option<PrimNum> {
        NonZeroU32::new(self.0.get() - 1).map(PrimNum)
    }

    /// All slots of the `prim` table.
    pub fn all() -> impl Iterator<Item = PrimNum> {
        (1..=crate::consts::PRIM_SIZE.cast_unsigned())
            .filter_map(NonZeroU32::new)
            .map(PrimNum)
    }
}

crate::codes::code_enum! {
    /// A text direction, as TeX--XeT stores it in edge subtypes and math subtypes over `R_CODE`.
    #[derive(Default)]
    pub enum Dir: u16 {
        #[default]
        Ltr = 0,
        Rtl = 1,
    }
}

impl Dir {
    #[inline]
    pub fn reversed(self) -> Dir {
        match self {
            Dir::Ltr => Dir::Rtl,
            Dir::Rtl => Dir::Ltr,
        }
    }
}

/// A position on the save stack.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct SavePtr(i32);

impl SavePtr {
    /// The bottom of the save stack.
    pub const ZERO: SavePtr = SavePtr(0);

    /// The position as a number.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0
    }

    #[inline(always)]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl std::ops::Add<i32> for SavePtr {
    type Output = SavePtr;
    #[inline(always)]
    fn add(self, rhs: i32) -> SavePtr {
        SavePtr(self.0 + rhs)
    }
}

impl std::ops::Sub<i32> for SavePtr {
    type Output = SavePtr;
    #[inline(always)]
    fn sub(self, rhs: i32) -> SavePtr {
        SavePtr(self.0 - rhs)
    }
}

impl std::ops::AddAssign<i32> for SavePtr {
    #[inline(always)]
    fn add_assign(&mut self, rhs: i32) {
        self.0 += rhs;
    }
}

impl std::ops::SubAssign<i32> for SavePtr {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: i32) {
        self.0 -= rhs;
    }
}

/// An internal font number. Nodes store it in 16 bits, as WEB's quarterword.
#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Hash,
    Default,
    bytemuck::Pod,
    bytemuck::Zeroable,
)]
#[repr(transparent)]
pub struct FontId(u16);

/// An eqtb word, format field or command code that is no font number.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct InvalidFontId(pub i32);

impl FontId {
    #[inline(always)]
    pub const fn new(n: u16) -> FontId {
        FontId(n)
    }

    #[inline(always)]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// The number widened for an eqtb word, format field or FFI call.
    #[inline(always)]
    pub fn raw(self) -> i32 {
        i32::from(self.0)
    }

    #[inline(always)]
    pub fn index(self) -> usize {
        usize::from(self.0)
    }

    /// The font's number in the DVI file, `f - font_base - 1`.
    #[inline]
    pub fn dvi_number(self) -> i32 {
        self.raw() - crate::consts::FONT_BASE - 1
    }

    #[inline]
    pub fn next(self) -> FontId {
        FontId(self.0 + 1)
    }

    /// The fonts after `null_font` up to `self`, the loaded ones when `self` is `font_ptr`.
    pub fn loaded(self) -> impl DoubleEndedIterator<Item = FontId> {
        (crate::consts::NULL_FONT.0 + 1..=self.0).map(FontId)
    }
}

impl TryFrom<i32> for FontId {
    type Error = InvalidFontId;

    #[inline]
    fn try_from(raw: i32) -> Result<FontId, InvalidFontId> {
        u16::try_from(raw)
            .map(FontId)
            .map_err(|_| InvalidFontId(raw))
    }
}

/// A string number: a single character below `TOO_BIG_CHAR`, else a pool string.
#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Hash,
    Default,
    bytemuck::Pod,
    bytemuck::Zeroable,
)]
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

    /// The single-character string of `c`, which WEB's `print(c)` prints.
    #[inline]
    pub const fn single_char(c: i32) -> StrNumber {
        StrNumber(c)
    }

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

    #[inline]
    pub fn prev(self) -> StrNumber {
        StrNumber(self.0 - 1)
    }

    /// The string `n` numbers after `self`.
    #[inline]
    pub fn offset(self, n: i32) -> StrNumber {
        StrNumber(self.0 + n)
    }
}

/// The number of strings from `rhs` to `self`.
impl std::ops::Sub for StrNumber {
    type Output = i32;
    #[inline(always)]
    fn sub(self, rhs: StrNumber) -> i32 {
        self.0 - rhs.0
    }
}

crate::codes::code_enum! {
    /// A node's type code. Delta nodes reuse the rule code and edge nodes the
    /// style code, their list context tells them apart.
    pub enum NodeType: u16 {
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
        /// A character node, which TeX told apart by its address in hi-mem.
        Char = 41,
        /// The head of a list being built, which is never part of a list.
        Head = 42,
    }
}

/// A type code outside the node types, as in the 32..=39 gap.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct InvalidNodeType(pub u16);

impl NodeType {
    /// The node type with code `raw`, an error for a code outside the node types.
    #[inline]
    pub fn of_code(raw: u16) -> Result<NodeType, InvalidNodeType> {
        NodeType::from_raw(i32::from(raw)).ok_or(InvalidNodeType(raw))
    }
}

crate::codes::code_enum! {
    /// Whether a box stacks its list horizontally (hlist) or vertically (vlist).
    /// The raw values are the box's `NODE_type` codes.
    pub enum Orientation: u16 {
        Horizontal = 0,
        Vertical = 1,
    }
}

impl Orientation {
    /// The box type code with this orientation.
    #[inline]
    pub const fn node_type(self) -> NodeType {
        match self {
            Orientation::Horizontal => NodeType::HList,
            Orientation::Vertical => NodeType::VList,
        }
    }
}

crate::codes::code_enum! {
    /// Whether a box's glue is at natural size, stretched or shrunk.
    pub enum GlueSign: u16 {
        Normal = 0,
        Stretching = 1,
        Shrinking = 2,
    }
}

impl GlueSign {}

crate::codes::code_enum! {
    /// The order of infinity of a stretch or shrink: finite, fil, fill or filll.
    #[derive(PartialOrd, Ord, Default)]
    pub enum GlueOrder: u16 {
        #[default]
        Normal = 0,
        Fil = 1,
        Fill = 2,
        Filll = 3,
        /// `filll + 1`, which matches no box's glue order, so the glue stays rigid.
        Rigid = 4,
    }
}

impl GlueOrder {
    /// The orders from finite up.
    pub const ALL: [GlueOrder; 4] = [
        GlueOrder::Normal,
        GlueOrder::Fil,
        GlueOrder::Fill,
        GlueOrder::Filll,
    ];

    /// The next higher order, `None` above `filll`.
    #[inline]
    pub fn next(self) -> Option<GlueOrder> {
        match self {
            GlueOrder::Normal => Some(GlueOrder::Fil),
            GlueOrder::Fil => Some(GlueOrder::Fill),
            GlueOrder::Fill => Some(GlueOrder::Filll),
            GlueOrder::Filll | GlueOrder::Rigid => None,
        }
    }
}

/// A total per glue order, as `total_stretch` and `total_shrink` keep.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct OrderTotals([Scaled; 4]);

impl OrderTotals {
    pub const ZERO: OrderTotals = OrderTotals([Scaled::ZERO; 4]);

    /// The highest order with a nonzero total, `Normal` when all are zero.
    #[inline]
    pub fn max_order(&self) -> GlueOrder {
        GlueOrder::ALL
            .into_iter()
            .rev()
            .find(|&o| self[o] != Scaled::ZERO)
            .unwrap_or(GlueOrder::Normal)
    }
}

impl OrderTotals {
    #[inline(always)]
    pub(crate) fn as_array(&self) -> &[Scaled; 4] {
        &self.0
    }
    #[inline(always)]
    pub(crate) fn as_array_mut(&mut self) -> &mut [Scaled; 4] {
        &mut self.0
    }
}

impl std::ops::Index<GlueOrder> for OrderTotals {
    type Output = Scaled;
    #[inline(always)]
    fn index(&self, o: GlueOrder) -> &Scaled {
        &self.0[usize::from(o.to_u16())]
    }
}

impl std::ops::IndexMut<GlueOrder> for OrderTotals {
    #[inline(always)]
    fn index_mut(&mut self, o: GlueOrder) -> &mut Scaled {
        &mut self.0[usize::from(o.to_u16())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn glue_order_round_trips_through_memory() {
        for o in [
            GlueOrder::Normal,
            GlueOrder::Fil,
            GlueOrder::Fill,
            GlueOrder::Filll,
            GlueOrder::Rigid,
        ] {
            assert_eq!(
                <GlueOrder as crate::node::StoredCode>::from_u16(o.to_u16()),
                Some(o)
            );
        }
    }

    #[test]
    fn order_totals_pick_the_highest_nonzero_order() {
        let mut t = OrderTotals::ZERO;
        assert_eq!(t.max_order(), GlueOrder::Normal);
        t[GlueOrder::Normal] = Scaled::UNITY;
        t[GlueOrder::Fill] = Scaled::UNITY;
        assert_eq!(t.max_order(), GlueOrder::Fill);
    }

    #[test]
    fn newtypes_are_zero_cost_repr() {
        assert_eq!(size_of::<MemIdx>(), size_of::<i32>());
        assert_eq!(size_of::<MemPtr>(), size_of::<i32>());
        assert_eq!(size_of::<Scaled>(), size_of::<i32>());
        assert_eq!(size_of::<FontId>(), size_of::<u16>());
    }

    #[test]
    fn null_is_the_zero_word() {
        assert_eq!(MemIdx::from_raw(0), None);
        assert_eq!(MemIdx::from_raw(-1), None);
        assert_eq!(MemIdx::from_raw(7).map(MemIdx::raw), Some(7));
        assert_eq!(None::<MemIdx>.raw(), 0);
        assert_eq!(bytemuck::cast::<u32, MemPtr>(0), None);
        assert_eq!(bytemuck::cast::<u32, MemPtr>(9).raw(), 9);
    }

    #[test]
    fn memidx_offset_walks_node_fields() {
        let p = MemIdx::from_raw(100).unwrap();
        assert_eq!(p.offset(1).raw(), 101);
        assert_eq!(p.offset(5).words(1), 105..106);
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
            (Char, 41),
            (Head, 42),
        ];
        for (nt, code) in all {
            assert_eq!(nt.to_u16(), code);
            assert_eq!(NodeType::of_code(code), Ok(nt));
        }
    }

    #[test]
    fn nodetype_from_raw_rejects_invalid() {
        for raw in [32u16, 35, 39, 43, 44, 100, u16::MAX] {
            assert_eq!(NodeType::of_code(raw), Err(InvalidNodeType(raw)));
        }
    }

    #[test]
    fn nodetype_is_two_bytes() {
        assert_eq!(size_of::<NodeType>(), size_of::<u16>());
    }
}
