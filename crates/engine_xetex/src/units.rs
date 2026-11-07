//! Zero-cost scalar newtypes: arena indices and pointers, scaled lengths,
//! font numbers, and the node-type and glue-sign codes.

use crate::consts::TEX_NULL;

/// A valid, non-null index into the `mem` arena.
#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, bytemuck::Pod, bytemuck::Zeroable,
)]
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
    /// The `len` words starting here, as a range of `mem` indices.
    #[inline]
    pub fn words(self, len: i32) -> std::ops::Range<usize> {
        // A negative start becomes an index past any arena, so slicing rejects it.
        let start = usize::try_from(self.0).unwrap_or(usize::MAX);
        start..start.saturating_add(usize::try_from(len).unwrap_or(0))
    }

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
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct PrimNum(i32);

impl PrimNum {
    /// No primitive has this name.
    pub const UNDEFINED: PrimNum = PrimNum(0);

    /// The value from its raw form in a `prim` link.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> PrimNum {
        PrimNum(raw)
    }

    /// The raw form for a `prim` link.
    #[inline(always)]
    pub const fn raw(self) -> i32 {
        self.0
    }

    #[inline(always)]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub fn prev(self) -> PrimNum {
        PrimNum(self.0 - 1)
    }

    /// The frozen eqtb location holding this primitive's meaning.
    #[inline]
    pub fn eqtb_loc(self) -> EqtbPtr {
        crate::consts::PRIM_EQTB_BASE.plus(self.0)
    }

    /// The primitive whose meaning sits at frozen eqtb location `p`, if any.
    #[inline]
    pub fn at_eqtb_loc(p: EqtbPtr) -> Option<PrimNum> {
        (crate::consts::PRIM_EQTB_BASE..crate::consts::FROZEN_NULL_FONT)
            .contains(&p)
            .then(|| PrimNum(p - crate::consts::PRIM_EQTB_BASE))
    }

    /// All slots of the `prim` table.
    pub fn all() -> impl Iterator<Item = PrimNum> {
        (0..=crate::consts::PRIM_SIZE).map(PrimNum)
    }
}

/// An index into `font_info`, the packed TFM tables and parameters of all fonts.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct FontInfoIdx(i32);

impl FontInfoIdx {
    pub const ZERO: FontInfoIdx = FontInfoIdx(0);
    /// `bchar_label` of a font without a boundary-character program.
    pub const NON_ADDRESS: FontInfoIdx = FontInfoIdx(0);

    /// The value from its raw form in a format file or a size setting.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> FontInfoIdx {
        FontInfoIdx(raw)
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

    /// The indices from `self` up to but not including `end`.
    pub fn until(self, end: FontInfoIdx) -> impl DoubleEndedIterator<Item = FontInfoIdx> + Clone {
        (self.0..end.0).map(FontInfoIdx)
    }
}

impl std::ops::Add<i32> for FontInfoIdx {
    type Output = FontInfoIdx;
    #[inline(always)]
    fn add(self, rhs: i32) -> FontInfoIdx {
        FontInfoIdx(self.0 + rhs)
    }
}

impl std::ops::Sub<i32> for FontInfoIdx {
    type Output = FontInfoIdx;
    #[inline(always)]
    fn sub(self, rhs: i32) -> FontInfoIdx {
        FontInfoIdx(self.0 - rhs)
    }
}

/// The number of words from `rhs` to `self`.
impl std::ops::Sub for FontInfoIdx {
    type Output = i32;
    #[inline(always)]
    fn sub(self, rhs: FontInfoIdx) -> i32 {
        self.0 - rhs.0
    }
}

impl std::ops::AddAssign<i32> for FontInfoIdx {
    #[inline(always)]
    fn add_assign(&mut self, rhs: i32) {
        self.0 += rhs;
    }
}

impl std::ops::SubAssign<i32> for FontInfoIdx {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: i32) {
        self.0 -= rhs;
    }
}

/// A text direction, as TeX--XeT stores it in edge subtypes and math subtypes over `R_CODE`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Dir {
    #[default]
    Ltr = 0,
    Rtl = 1,
}

impl Dir {
    /// The direction stored as `code`, `None` for any other value.
    #[inline]
    pub fn from_code(code: u16) -> Option<Dir> {
        match code {
            0 => Some(Dir::Ltr),
            1 => Some(Dir::Rtl),
            _ => None,
        }
    }

    #[inline]
    pub fn code(self) -> u16 {
        self as u16
    }

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

    /// The value from its raw form in a memory word.
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> SavePtr {
        SavePtr(raw)
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
}

impl std::ops::Add<i32> for SavePtr {
    type Output = SavePtr;
    #[inline(always)]
    fn add(self, rhs: i32) -> SavePtr {
        SavePtr(self.0 + rhs)
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

    /// The `eqtb` location of the font's identifier, `font_id_base + f`.
    #[inline]
    pub fn id_loc(self) -> EqtbPtr {
        crate::consts::FONT_ID_BASE + self.raw()
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

/// Whether a box stacks its list horizontally (hlist) or vertically (vlist).
/// The raw values are the box's `NODE_type` codes.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum Orientation {
    Horizontal = 0,
    Vertical = 1,
}

impl Orientation {
    #[inline]
    pub const fn as_raw(self) -> u16 {
        self as u16
    }

    /// The box type code with this orientation.
    #[inline]
    pub const fn node_type(self) -> NodeType {
        match self {
            Orientation::Horizontal => NodeType::HList,
            Orientation::Vertical => NodeType::VList,
        }
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
    pub const fn as_raw(self) -> u16 {
        self as u16
    }
}

/// The order of infinity of a stretch or shrink: finite, fil, fill or filll.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
#[repr(u16)]
pub enum GlueOrder {
    #[default]
    Normal = 0,
    Fil = 1,
    Fill = 2,
    Filll = 3,
    /// `filll + 1`, which matches no box's glue order, so the glue stays rigid.
    Rigid = 4,
}

impl GlueOrder {
    /// The orders from finite up.
    pub const ALL: [GlueOrder; 4] = [
        GlueOrder::Normal,
        GlueOrder::Fil,
        GlueOrder::Fill,
        GlueOrder::Filll,
    ];

    #[inline]
    pub const fn as_raw(self) -> u16 {
        self as u16
    }

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
        &self.0[usize::from(o.as_raw())]
    }
}

impl std::ops::IndexMut<GlueOrder> for OrderTotals {
    #[inline(always)]
    fn index_mut(&mut self, o: GlueOrder) -> &mut Scaled {
        &mut self.0[usize::from(o.as_raw())]
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
                <GlueOrder as crate::node::StoredCode>::from_u16(o.as_raw()),
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
