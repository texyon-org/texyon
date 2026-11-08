//! Typed node handles over the flat `mem` arena.
//!
//! [`NodePtr<K>`] is a `Copy` index plus a compile-time kind marker, so a field
//! access compiles to the same load/store as the raw `mem[p + k]` form. Every
//! kind's layout lives in [`crate::overlay`] and indexes the arena:
//! `mem[p].width`, `mem.entries(p)` for the text or array after a header.

use crate::fonts::Side;
use crate::math::MathStyle;
use crate::params::GluePar;
use std::marker::PhantomData;

use crate::consts::{
    ACCENT_NOAD_SIZE, BOX_NODE_SIZE, FRACTION_NOAD_SIZE, GLUE_SPEC_SIZE, GLYPH_NODE_SIZE,
    INDEX_NODE_SIZE, INS_NODE_SIZE, MARGIN_KERN_NODE_SIZE, MARK_CLASS_NODE_SIZE, MEDIUM_NODE_SIZE,
    NOAD_SIZE, NULL_FLAG, OPEN_NODE_SIZE, PAGE_INS_NODE_SIZE, RADICAL_NOAD_SIZE, RULE_NODE_SIZE,
    SMALL_NODE_SIZE, STYLE_NODE_SIZE, WRITE_NODE_SIZE,
};
use crate::errors::Abort;
use crate::state::MemState;
use crate::units::{
    Dir, GlueOrder, GlueSign, InvalidNodeType, MemAddr, MemIdx, MemPtr, NodeType, Orientation,
    Scaled,
};

/// Marker for node kinds, implemented by the uninhabited kind enums below.
pub trait NodeKind {}

/// A typed handle to a node of kind `K` in the `mem` arena.
#[repr(transparent)]
pub struct NodePtr<K: NodeKind> {
    idx: MemIdx,
    _kind: PhantomData<fn() -> K>,
}

impl<K: NodeKind> Copy for NodePtr<K> {}
impl<K: NodeKind> Clone for NodePtr<K> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<K: NodeKind> PartialEq for NodePtr<K> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}
impl<K: NodeKind> Eq for NodePtr<K> {}
// SAFETY: a `MemIdx` and a zero-sized marker, valid at all-zero bits.
unsafe impl<K: NodeKind + 'static> bytemuck::Zeroable for NodePtr<K> {}
// SAFETY: a `MemIdx` and a zero-sized marker, so no padding and any bits are valid.
unsafe impl<K: NodeKind + 'static> bytemuck::Pod for NodePtr<K> {}
impl<K: NodeKind> std::fmt::Debug for NodePtr<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodePtr({})", self.idx.raw())
    }
}

impl<K: NodeKind> NodePtr<K> {
    /// Wraps `idx` without checking that it holds a `K` node.
    ///
    /// # Safety
    /// `idx` must reference a `K` node. A wrong kind is not memory-unsafe (all
    /// fields index in bounds) but reads fields the real node does not have.
    #[inline]
    pub const fn from_idx_unchecked(idx: MemIdx) -> Self {
        NodePtr {
            idx,
            _kind: PhantomData,
        }
    }

    #[inline]
    pub fn idx(self) -> MemIdx {
        self.idx
    }

    /// The node's address as an untyped link value.
    #[inline]
    pub fn ptr(self) -> MemPtr {
        MemPtr::from_idx(self.idx)
    }
}

/// A nullable pointer to a `K` as a node stores it: the raw half-word,
/// `TEX_NULL` for none.
#[repr(transparent)]
pub struct Link<K: NodeKind>(MemPtr, PhantomData<fn() -> K>);

impl<K: NodeKind> Copy for Link<K> {}
impl<K: NodeKind> Clone for Link<K> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<K: NodeKind> PartialEq for Link<K> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<K: NodeKind> Eq for Link<K> {}
// SAFETY: a `MemPtr` and a zero-sized marker, valid at all-zero bits.
unsafe impl<K: NodeKind + 'static> bytemuck::Zeroable for Link<K> {}
// SAFETY: a `MemPtr` and a zero-sized marker, so no padding and any bits are valid.
unsafe impl<K: NodeKind + 'static> bytemuck::Pod for Link<K> {}
impl<K: NodeKind> std::fmt::Debug for Link<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Link({})", self.0.raw())
    }
}

impl<K: NodeKind> Link<K> {
    /// The node, `None` for a null link.
    #[inline]
    pub fn get(self) -> Option<NodePtr<K>> {
        self.0.as_idx().map(NodePtr::from_idx_unchecked)
    }
}

impl<K: NodeKind> From<Option<NodePtr<K>>> for Link<K> {
    #[inline]
    fn from(p: Option<NodePtr<K>>) -> Self {
        Link(link_of(p), PhantomData)
    }
}

impl<K: NodeKind> From<NodePtr<K>> for Link<K> {
    #[inline]
    fn from(p: NodePtr<K>) -> Self {
        Link(p.ptr(), PhantomData)
    }
}

impl std::ops::Index<Stored<GlueOrder>> for crate::units::OrderTotals {
    type Output = Scaled;
    #[inline(always)]
    fn index(&self, o: Stored<GlueOrder>) -> &Scaled {
        &self.as_array()[o.index()]
    }
}

impl std::ops::IndexMut<Stored<GlueOrder>> for crate::units::OrderTotals {
    #[inline(always)]
    fn index_mut(&mut self, o: Stored<GlueOrder>) -> &mut Scaled {
        &mut self.as_array_mut()[o.index()]
    }
}

/// A code set a node stores in a 16-bit subfield.
pub trait StoredCode: Copy {
    /// The code `raw` stands for, `None` when it is none of them.
    fn from_u16(raw: u16) -> Option<Self>;
    fn to_u16(self) -> u16;
}

/// A code as a node stores it. The raw bits are kept, so a value outside the
/// code set (a WEB sentinel, a fresh node's leftovers) reads back unchanged.
#[repr(transparent)]
pub struct Stored<T>(u16, PhantomData<fn() -> T>);

impl<T> Copy for Stored<T> {}
impl<T> Clone for Stored<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> PartialEq for Stored<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T> Eq for Stored<T> {}
// SAFETY: a `u16` and a zero-sized marker, valid at all-zero bits.
unsafe impl<T: 'static> bytemuck::Zeroable for Stored<T> {}
// SAFETY: a `u16` and a zero-sized marker, so no padding and any bits are valid.
unsafe impl<T: 'static> bytemuck::Pod for Stored<T> {}

impl<T> Default for Stored<T> {
    #[inline]
    fn default() -> Self {
        Stored(0, PhantomData)
    }
}
impl<T> std::fmt::Debug for Stored<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stored({})", self.0)
    }
}

impl<T: StoredCode> Stored<T> {
    #[inline]
    pub const fn from_raw(raw: u16) -> Self {
        Stored(raw, PhantomData)
    }

    #[inline]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// The code, `None` when the stored bits are not one.
    #[inline]
    pub fn get(self) -> Option<T> {
        T::from_u16(self.0)
    }

    /// The raw code as an array index, as WEB indexes by it.
    #[inline]
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

impl<T: StoredCode> From<T> for Stored<T> {
    #[inline]
    fn from(code: T) -> Self {
        Stored(code.to_u16(), PhantomData)
    }
}

impl<T: StoredCode> PartialEq<T> for Stored<T> {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        self.0 == other.to_u16()
    }
}

// Codes compare as WEB compares them, by their raw numbers.
impl<T> PartialOrd for Stored<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Stored<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
impl<T: StoredCode> PartialOrd<T> for Stored<T> {
    #[inline]
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.to_u16()))
    }
}

/// A `u16` code set a node subfield stores, with its [`StoredCode`] impl.
macro_rules! stored_code {
    ($(#[$m:meta])* $vis:vis enum $name:ident {
        $($(#[$vm:meta])* $v:ident = $n:literal),+ $(,)?
    }) => {
        $(#[$m])*
        #[derive(Copy, Clone, PartialEq, Eq, Debug)]
        #[repr(u16)]
        $vis enum $name {
            $($(#[$vm])* $v = $n),+
        }

        impl StoredCode for $name {
            #[inline]
            fn from_u16(raw: u16) -> Option<Self> {
                match raw {
                    $($n => Some($name::$v),)+
                    _ => None,
                }
            }
            #[inline]
            fn to_u16(self) -> u16 {
                self as u16
            }
        }
    };
}

stored_code! {
    /// What made a kern, which decides where a line may break and how it shows.
    pub enum KernSubtype {
        /// A font kern.
        Normal = 0,
        /// `\kern` or an italic correction.
        Explicit = 1,
        /// Around an accent.
        AccKern = 2,
        /// XeTeX's adjustment after an interword space.
        SpaceAdjustment = 3,
        /// `\mkern`, in mu until `mlist_to_hlist` converts it.
        MuGlue = 99,
    }
}

stored_code! {
    /// Where a `\vadjust` list goes around the line that holds it.
    pub enum AdjustPlacement {
        /// After the line.
        Post = 0,
        /// Before the line, `\vadjust pre`.
        Pre = 1,
    }
}

stored_code! {
    /// A math accent's placement, set by `\Umathaccent fixed`, `bottom` or `bottom fixed`.
    pub enum MathAccentKind {
        /// Above the nucleus, stretched to its width.
        Top = 0,
        /// Above, at the accent's own width.
        TopFixed = 1,
        /// Below the nucleus, stretched.
        Bottom = 2,
        /// Below, at the accent's own width.
        BottomFixed = 3,
    }
}

impl MathAccentKind {
    #[inline]
    pub fn is_bottom(self) -> bool {
        matches!(self, MathAccentKind::Bottom | MathAccentKind::BottomFixed)
    }

    #[inline]
    pub fn is_fixed(self) -> bool {
        matches!(self, MathAccentKind::TopFixed | MathAccentKind::BottomFixed)
    }
}

stored_code! {
    /// A right noad's subtype, which tells `\middle` from `\right`.
    pub enum FenceSubtype {
        /// `\left` or `\right`.
        Normal = 0,
        /// `\middle`.
        Middle = 1,
    }
}

stored_code! {
    /// The implicit boundary characters a ligature absorbed, shown as `|` around it.
    pub enum LigBoundary {
        /// None.
        None = 0,
        /// The right boundary character.
        Right = 1,
        /// The left boundary character.
        Left = 2,
        /// Both.
        Both = 3,
    }
}

impl LigBoundary {
    /// The same boundaries plus the right one.
    #[inline]
    pub fn with_right(self) -> LigBoundary {
        match self {
            LigBoundary::None | LigBoundary::Right => LigBoundary::Right,
            LigBoundary::Left | LigBoundary::Both => LigBoundary::Both,
        }
    }
}

// The bits WEB tests, so any stored value reads as WEB reads it.
impl Stored<LigBoundary> {
    #[inline]
    pub fn has_left(self) -> bool {
        self.raw() > 1
    }

    #[inline]
    pub fn has_right(self) -> bool {
        self.raw() % 2 == 1
    }
}

stored_code! {
    /// A math node's code: math-on/off, the `\beginM`/`\endM` pair around a display, or a
    /// TeX--XeT boundary. The LR stack holds these codes too.
    pub enum MathCode {
        /// `\mathon`.
        Before = 0,
        /// `\mathoff`.
        After = 1,
        /// Opens the segment around a display.
        BeginM = 2,
        /// Closes it.
        EndM = 3,
        /// `\beginL`.
        BeginL = 6,
        /// `\endL`.
        EndL = 7,
        /// `\beginR`.
        BeginR = 10,
        /// `\endR`.
        EndR = 11,
    }
}

impl MathCode {
    /// Closes a segment: `\mathoff` or an end code.
    #[inline]
    pub fn is_end(self) -> bool {
        matches!(
            self,
            MathCode::After | MathCode::EndM | MathCode::EndL | MathCode::EndR
        )
    }

    /// A TeX--XeT `\beginL`..`\endR` boundary.
    #[inline]
    pub fn is_lr(self) -> bool {
        matches!(
            self,
            MathCode::BeginL | MathCode::EndL | MathCode::BeginR | MathCode::EndR
        )
    }

    /// The direction of the segment it bounds, left to right unless it is an R code.
    #[inline]
    pub fn dir(self) -> Dir {
        match self {
            MathCode::BeginR | MathCode::EndR => Dir::Rtl,
            _ => Dir::Ltr,
        }
    }

    /// TeX's `end_LR_type`: the LR stack entry this code opens or closes.
    #[inline]
    pub fn lr_end(self) -> MathCode {
        match self {
            MathCode::Before | MathCode::After | MathCode::BeginM | MathCode::EndM => {
                MathCode::EndM
            }
            MathCode::BeginL | MathCode::EndL => MathCode::EndL,
            MathCode::BeginR | MathCode::EndR => MathCode::EndR,
        }
    }

    /// The opening code of its pair, WEB's `subtype - 1` on an end code.
    #[inline]
    pub fn as_begin(self) -> MathCode {
        match self {
            MathCode::Before | MathCode::After => MathCode::Before,
            MathCode::BeginM | MathCode::EndM => MathCode::BeginM,
            MathCode::BeginL | MathCode::EndL => MathCode::BeginL,
            MathCode::BeginR | MathCode::EndR => MathCode::BeginR,
        }
    }

    /// The closing code of its pair, WEB's `subtype + 1` on a begin code.
    #[inline]
    pub fn as_end(self) -> MathCode {
        match self {
            MathCode::Before | MathCode::After => MathCode::After,
            MathCode::BeginM | MathCode::EndM => MathCode::EndM,
            MathCode::BeginL | MathCode::EndL => MathCode::EndL,
            MathCode::BeginR | MathCode::EndR => MathCode::EndR,
        }
    }
}

stored_code! {
    /// How leaders repeat their box or rule.
    pub enum LeaderKind {
        /// `\leaders`, aligned on the enclosing box.
        Aligned = 100,
        /// `\cleaders`, centred.
        Centered = 101,
        /// `\xleaders`, expanded.
        Expanded = 102,
    }
}

/// What a glue node is: plain, made from a parameter, math glue or leaders.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum GlueKind {
    /// Plain glue.
    Normal,
    /// From a glue parameter, so `\showbox` names it.
    Param(GluePar),
    /// Glue that `\nonscript` cancels in script styles.
    CondMath,
    /// `\mskip`, in mu until `mlist_to_hlist` converts it.
    Mu,
    /// `\leaders`, `\cleaders` or `\xleaders`.
    Leaders(LeaderKind),
}

const COND_MATH_GLUE: u16 = 98;
const MU_GLUE: u16 = 99;

impl StoredCode for GlueKind {
    #[inline]
    fn from_u16(raw: u16) -> Option<Self> {
        Some(match raw {
            0 => GlueKind::Normal,
            COND_MATH_GLUE => GlueKind::CondMath,
            MU_GLUE => GlueKind::Mu,
            _ => match LeaderKind::from_u16(raw) {
                Some(kind) => GlueKind::Leaders(kind),
                None => GlueKind::Param(GluePar::from_raw(i32::from(raw) - 1)?),
            },
        })
    }
    #[inline]
    fn to_u16(self) -> u16 {
        match self {
            GlueKind::Normal => 0,
            GlueKind::Param(p) => p as u16 + 1,
            GlueKind::CondMath => COND_MATH_GLUE,
            GlueKind::Mu => MU_GLUE,
            GlueKind::Leaders(kind) => kind.to_u16(),
        }
    }
}

stored_code! {
    /// TeX--XeT's `box_lr` of an hlist.
    pub enum BoxLr {
        /// Not reversed yet.
        Normal = 0,
        /// Reversed for right-to-left output.
        Reversed = 1,
        /// A display whose lines run right to left.
        Dlist = 2,
    }
}

impl StoredCode for Side {
    #[inline]
    fn from_u16(raw: u16) -> Option<Self> {
        match raw {
            0 => Some(Side::Left),
            1 => Some(Side::Right),
            _ => None,
        }
    }
    #[inline]
    fn to_u16(self) -> u16 {
        match self {
            Side::Left => 0,
            Side::Right => 1,
        }
    }
}

impl StoredCode for Dir {
    #[inline]
    fn from_u16(raw: u16) -> Option<Self> {
        Dir::from_code(raw)
    }
    #[inline]
    fn to_u16(self) -> u16 {
        self.code()
    }
}

impl StoredCode for GlueSign {
    #[inline]
    fn from_u16(raw: u16) -> Option<Self> {
        match raw {
            0 => Some(GlueSign::Normal),
            1 => Some(GlueSign::Stretching),
            2 => Some(GlueSign::Shrinking),
            _ => None,
        }
    }
    #[inline]
    fn to_u16(self) -> u16 {
        self.as_raw()
    }
}

impl StoredCode for GlueOrder {
    #[inline]
    fn from_u16(raw: u16) -> Option<Self> {
        match raw {
            0 => Some(GlueOrder::Normal),
            1 => Some(GlueOrder::Fil),
            2 => Some(GlueOrder::Fill),
            3 => Some(GlueOrder::Filll),
            4 => Some(GlueOrder::Rigid),
            _ => None,
        }
    }
    #[inline]
    fn to_u16(self) -> u16 {
        self.as_raw()
    }
}

impl StoredCode for NodeType {
    #[inline]
    fn from_u16(raw: u16) -> Option<Self> {
        NodeType::from_raw(raw).ok()
    }
    #[inline]
    fn to_u16(self) -> u16 {
        self.as_raw()
    }
}

impl StoredCode for Orientation {
    #[inline]
    fn from_u16(raw: u16) -> Option<Self> {
        match raw {
            0 => Some(Orientation::Horizontal),
            1 => Some(Orientation::Vertical),
            _ => None,
        }
    }
    #[inline]
    fn to_u16(self) -> u16 {
        self.as_raw()
    }
}

const _: () = assert!(
    Orientation::Horizontal.node_type().as_raw() == Orientation::Horizontal.as_raw()
        && Orientation::Vertical.node_type().as_raw() == Orientation::Vertical.as_raw()
);

/// A kind with a fixed allocation size and header type code.
pub trait FixedNode: NodeKind {
    const SIZE: i32;
    /// The `NODE_type` stamped at allocation, `None` for headerless records.
    const TYPE: Option<NodeType>;
}

impl MemState {
    #[inline]
    fn alloc_node<K: NodeKind>(
        &mut self,
        size: i32,
        ty: Option<NodeType>,
    ) -> Result<NodePtr<K>, Abort> {
        let p = self.get_node(size)?;
        if let Some(ty) = ty {
            self.set_b16s1(p, ty.as_raw());
            self.set_b16s0(p, 0);
        }
        Ok(NodePtr::from_idx_unchecked(MemIdx::new(p)))
    }
}

impl<K: FixedNode> NodePtr<K> {
    /// Carves a fresh node with its type stamped and its fields uninitialised.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Result<Self, Abort> {
        mem.alloc_node(K::SIZE, K::TYPE)
    }
}

macro_rules! node_kinds {
    ($($(#[$m:meta])* $kind:ident $(= $ty:ident, $size:expr)?;)*) => {$(
        $(#[$m])*
        pub enum $kind {}
        impl NodeKind for $kind {}
        $(
            impl FixedNode for $kind {
                const SIZE: i32 = $size;
                const TYPE: Option<NodeType> = Some(NodeType::$ty);
            }
        )?
    )*};
}

node_kinds! {
    /// An hlist or vlist box. `alloc` makes an hlist.
    BoxNode = HList, BOX_NODE_SIZE;
    RuleNode = Rule, RULE_NODE_SIZE;
    InsNode = Ins, INS_NODE_SIZE;
    MarkNode = Mark, SMALL_NODE_SIZE;
    /// A `\vadjust` list.
    AdjustNode = Adjust, SMALL_NODE_SIZE;
    LigatureNode = Ligature, SMALL_NODE_SIZE;
    DiscNode = Disc, SMALL_NODE_SIZE;
    MathNode = Math, MEDIUM_NODE_SIZE;
    GlueNode = Glue, MEDIUM_NODE_SIZE;
    KernNode = Kern, MEDIUM_NODE_SIZE;
    PenaltyNode = Penalty, MEDIUM_NODE_SIZE;
    StyleNode = Style, STYLE_NODE_SIZE;
    ChoiceNode = Choice, STYLE_NODE_SIZE;
    /// An LR edge in a shipped list. XeTeX aliases its type code to style.
    EdgeNode = Style, STYLE_NODE_SIZE;
    /// A noad with a nucleus and two scripts: ord through inner, overline,
    /// underline and vcenter. `alloc` makes an ordinary one.
    Noad = OrdNoad, NOAD_SIZE;
    /// A radical: a noad, then its delimiter.
    RadicalNoad = RadicalNoad, RADICAL_NOAD_SIZE;
    /// A math accent: a noad, then its accent character.
    AccentNoad = AccentNoad, ACCENT_NOAD_SIZE;
    /// A generalized fraction.
    FractionNoad = FractionNoad, FRACTION_NOAD_SIZE;
    /// A `\left`, `\middle` or `\right` delimiter. `alloc` makes a left one.
    FenceNoad = LeftNoad, NOAD_SIZE;
    /// Any noad, through the words every noad shares.
    AnyNoad;
    MarginKernNode = MarginKern, MARGIN_KERN_NODE_SIZE;
    /// Any whatsit, through its header. [`NodePtr::variant`] gives its layout.
    WhatsitNode;
    /// `\openout`: a stream and a file name.
    OpenNode = Whatsit, OPEN_NODE_SIZE;
    /// `\write`, `\closeout` or `\special`: a stream and a token list.
    WriteNode = Whatsit, WRITE_NODE_SIZE;
    /// A language change: the language and its hyphenation minima.
    LanguageNode = Whatsit, SMALL_NODE_SIZE;
    /// One glyph of a native font, by glyph id.
    GlyphNode = Whatsit, GLYPH_NODE_SIZE;
    /// A `\XeTeXpicfile` or `\XeTeXpdffile` image, its path stored inline.
    PicNode;
    /// A run of text in one native font, stored inline as UTF-16.
    NativeWordNode;
    /// A native word, glyph or picture, through the dimensions they share.
    SizedWhatsit;
    /// An alignment box before its column widths are known. Words +4..+6 hold
    /// the accumulated shrink, span count and stretch instead of box glue.
    UnsetNode;
    /// A glue specification, reached only through a glue pointer. Its header
    /// word holds the reference count and fill orders, not a type code.
    GlueSpecNode;
    /// An entry of the ligature reconstitution stack.
    LigItem;
    /// An eTeX sparse-array index node: 64 children, two per word.
    SaIndexNode;
    /// A sparse-array element, the leaf below the last index level, read
    /// through its header until its type picks the layout.
    SaElement;
    /// A count or dimension register element.
    SaWordNode;
    /// A glue, muglue, box or token-list register element.
    SaPointerNode;
    /// The marks of one mark class.
    SaMarkClassNode;
    /// An entry of the sparse-array save chain, read through its header.
    SaSaved;
    /// A saved count or dimension value.
    SaSavedWord;
    /// A saved pointer value.
    SaSavedPointer;
    /// A record on the page builder's insertion ring. Its type code is
    /// `inserting` or `split_up`, its subtype the insertion's box number.
    PageInsNode;
    /// A shipout `down_ptr`/`right_ptr` movement record.
    MovementNode;
    /// A line-breaker active node. Its type word holds the break type.
    ActiveNode;
    /// A line-breaker delta node, sitting between active nodes.
    DeltaNode;
    /// A line-breaker passive node. Its info word is a serial number.
    PassiveNode;
    /// An entry of the conditional stack, saving the enclosing `\if`.
    CondNode;
    /// A one-word character node. A ligature's +1 word has the same layout.
    CharNode;
    /// An alignment preamble column: a null box holding the column's
    /// templates, natural width and span list.
    AlignRecord;
    /// An entry of a column's span list, ended by `END_SPAN`. The list head is
    /// the align record itself, which shares the `info` and `width` words.
    SpanNode;
    /// A saved outer alignment on the `align_ptr` stack.
    AlignStackNode;
    /// A noad's nucleus, script or accent character word: its math type,
    /// then a character and family or a sub-list.
    MathField;
    /// A noad's delimiter word, holding a small and a large variant.
    DelimiterField;
    /// A `\parshape` array: the line count, then an indent and width per line.
    ParShapeNode;
    /// An e-TeX penalty array: the entry count at +1, then the entries.
    PenaltiesNode;
    /// A suspended level of `\numexpr`-style expression scanning.
    ExprNode;
    /// A line of a `\scantokens` pseudo file: its size, then four characters per word.
    PseudoLineNode;
    /// A block on the variable-size free list, marked by an empty link word.
    FreeBlock;
}

impl FixedNode for GlueSpecNode {
    const SIZE: i32 = GLUE_SPEC_SIZE;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for LigItem {
    const SIZE: i32 = SMALL_NODE_SIZE;
    const TYPE: Option<NodeType> = None;
}
/// Always the extended `\lastlinefit` size, unlike WEB's 3 or 5 words per paragraph.
impl FixedNode for ActiveNode {
    const SIZE: i32 = 5;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for SaIndexNode {
    const SIZE: i32 = INDEX_NODE_SIZE;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for SaWordNode {
    const SIZE: i32 = 3;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for SaPointerNode {
    const SIZE: i32 = 2;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for SaMarkClassNode {
    const SIZE: i32 = MARK_CLASS_NODE_SIZE;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for SaSavedWord {
    const SIZE: i32 = SaWordNode::SIZE;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for SaSavedPointer {
    const SIZE: i32 = SaPointerNode::SIZE;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for PassiveNode {
    const SIZE: i32 = 2;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for DeltaNode {
    const SIZE: i32 = 7;
    const TYPE: Option<NodeType> = Some(NodeType::Rule);
}
impl FixedNode for CondNode {
    const SIZE: i32 = 2;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for MovementNode {
    const SIZE: i32 = MOVEMENT_NODE_SIZE;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for SpanNode {
    const SIZE: i32 = 2;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for AlignRecord {
    const SIZE: i32 = BOX_NODE_SIZE;
    const TYPE: Option<NodeType> = Some(NodeType::HList);
}
impl FixedNode for UnsetNode {
    const SIZE: i32 = BOX_NODE_SIZE;
    const TYPE: Option<NodeType> = Some(NodeType::Unset);
}
impl FixedNode for PageInsNode {
    const SIZE: i32 = PAGE_INS_NODE_SIZE;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for AlignStackNode {
    const SIZE: i32 = 6;
    const TYPE: Option<NodeType> = None;
}
impl FixedNode for ExprNode {
    const SIZE: i32 = 4;
    const TYPE: Option<NodeType> = None;
}

const MOVEMENT_NODE_SIZE: i32 = 3;

// --- shared layouts ---

/// A `link` in word 0, which generic list walks read as an untyped word.
pub trait HasLink: NodeKind {}

macro_rules! impl_traits {
    ($trait:ident: $($kind:ident),*) => {$(impl $trait for $kind {})*};
}

impl_traits!(HasLink: BoxNode, RuleNode, InsNode, MarkNode, AdjustNode, LigatureNode, DiscNode,
    WhatsitNode, MathNode, GlueNode, KernNode, PenaltyNode, UnsetNode, StyleNode, ChoiceNode,
    EdgeNode, Noad, RadicalNoad, AccentNoad, FractionNoad, FenceNoad, AnyNoad, MarginKernNode, SaIndexNode, SaElement, SaWordNode, SaPointerNode, SaMarkClassNode,
    SaSaved, SaSavedWord, SaSavedPointer, PageInsNode, ActiveNode, DeltaNode, PassiveNode,
    MovementNode, CondNode, CharNode, AlignRecord, AlignStackNode, ExprNode, PseudoLineNode,
    LigItem);

/// A record kept only on its own list, which therefore proves its kind.
pub trait ListRecord: NodeKind {}

impl_traits!(ListRecord: SaIndexNode, SaElement, SaWordNode, SaPointerNode, SaMarkClassNode,
    SaSaved, SaSavedWord, SaSavedPointer, PageInsNode, MovementNode, ActiveNode, DeltaNode, PassiveNode,
    LigItem, CondNode, CharNode, AlignRecord, SpanNode, AlignStackNode, ParShapeNode, PenaltiesNode,
    ExprNode, PseudoLineNode, MathField, DelimiterField, FreeBlock);

impl<K: ListRecord> NodePtr<K> {
    /// The record at `p`, a non-null link of a list of `K` records.
    #[inline]
    pub fn at(p: impl crate::units::MemAddr) -> Self {
        NodePtr::from_idx_unchecked(MemIdx::new(p))
    }

    /// The record at `p`, `None` when the link is null.
    #[inline]
    pub fn from_ptr(p: MemPtr) -> Option<Self> {
        (!p.is_null()).then(|| Self::at(p))
    }
}

impl<K: ListRecord + HasLink> NodePtr<K> {
    /// The next record on the list, `None` at its end.
    #[inline]
    pub fn next(self, mem: &MemState) -> Option<Self> {
        Self::from_ptr(self.ptr().link(mem))
    }
}

/// The link value of an optional node, null for `None`.
#[inline]
pub fn link_of<K: NodeKind>(p: Option<NodePtr<K>>) -> MemPtr {
    p.map_or(MemPtr::NULL, NodePtr::ptr)
}

impl<K: ListRecord + FixedNode> NodePtr<K> {
    #[inline]
    pub fn free(self, mem: &mut MemState) {
        mem.free_node(self.ptr(), K::SIZE);
    }
}

impl MemPtr {
    #[inline]
    pub fn link(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self))
    }
    #[inline]
    pub fn set_link(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self, q.raw());
    }
    /// Knuth's `info`, the payload half of a one-word node.
    #[inline]
    pub fn info(self, mem: &MemState) -> i32 {
        mem.b32s0(self)
    }
    /// `info` of a node whose payload is a pointer.
    #[inline]
    pub fn info_ptr(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(self.info(mem))
    }
    #[inline]
    pub fn set_info(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self, v);
    }
}

// --- boxes and rules ---

impl NodePtr<BoxNode> {
    #[inline]
    pub fn into_unset(self, mem: &mut MemState) -> NodePtr<UnsetNode> {
        retype(self, mem, NodeType::Unset)
    }

    /// The pseudo-node whose `link` is `list_ptr`, for splicing at the list head.
    #[inline]
    pub fn list_head(self) -> MemPtr {
        MemPtr::from_idx(self.idx.offset(5))
    }
}

/// Stamps a new type code and rewraps the handle.
#[inline]
fn retype<K: NodeKind, L: NodeKind>(p: NodePtr<K>, mem: &mut MemState, ty: NodeType) -> NodePtr<L> {
    mem.set_b16s1(p.idx, ty.as_raw());
    NodePtr::from_idx_unchecked(p.idx)
}

/// A rule dimension: the running sentinel (`NULL_FLAG`) or a concrete length.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum RuleDimen {
    Running,
    Set(Scaled),
}

impl RuleDimen {
    #[inline]
    pub fn from_raw(raw: i32) -> RuleDimen {
        if raw == NULL_FLAG {
            RuleDimen::Running
        } else {
            RuleDimen::Set(Scaled::from_raw(raw))
        }
    }

    #[inline]
    pub fn raw(self) -> i32 {
        match self {
            RuleDimen::Running => NULL_FLAG,
            RuleDimen::Set(v) => v.raw(),
        }
    }
}

/// A running rule dimension seen as a length, as TeX's arithmetic sees it.
pub(crate) const RUNNING: Scaled = Scaled::from_raw(NULL_FLAG);

/// A rule dimension as a node stores it, the raw word decoded by `get`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub struct StoredDimen(i32);

impl StoredDimen {
    #[inline]
    pub fn get(self) -> RuleDimen {
        RuleDimen::from_raw(self.0)
    }

    /// The word as a length, `RUNNING` for a running dimension.
    #[inline]
    pub fn to_scaled(self) -> Scaled {
        Scaled::from_raw(self.0)
    }
}

impl From<RuleDimen> for StoredDimen {
    #[inline]
    fn from(d: RuleDimen) -> Self {
        StoredDimen(d.raw())
    }
}

/// What `\box`, `\hrule` and friends leave in `cur_box`: a rule only where
/// leaders accept one.
#[derive(Copy, Clone, Debug)]
pub enum BoxOrRule {
    Box(NodePtr<BoxNode>),
    Rule(NodePtr<RuleNode>),
}

impl BoxOrRule {
    #[inline]
    pub fn ptr(self) -> MemPtr {
        match self {
            BoxOrRule::Box(b) => b.ptr(),
            BoxOrRule::Rule(r) => r.ptr(),
        }
    }
}

// --- classification ---

/// A classified lo-mem node. Type 2 is read as a rule (not a delta node) and
/// type 14 as a style node (not an edge): walkers of those special lists
/// know their context and do not classify.
#[derive(Copy, Clone, Debug)]
pub enum AnyNode {
    /// An hlist or vlist box.
    Box(NodePtr<BoxNode>),
    Rule(NodePtr<RuleNode>),
    Ins(NodePtr<InsNode>),
    Mark(NodePtr<MarkNode>),
    Adjust(NodePtr<AdjustNode>),
    Ligature(NodePtr<LigatureNode>),
    Disc(NodePtr<DiscNode>),
    Whatsit(NodePtr<WhatsitNode>),
    Math(NodePtr<MathNode>),
    Glue(NodePtr<GlueNode>),
    Kern(NodePtr<KernNode>),
    Penalty(NodePtr<PenaltyNode>),
    Unset(NodePtr<UnsetNode>),
    Style(NodePtr<StyleNode>),
    Choice(NodePtr<ChoiceNode>),
    /// Any noad, refine with [`NodePtr::<AnyNoad>::variant`].
    Noad(NodePtr<AnyNoad>),
    MarginKern(NodePtr<MarginKernNode>),
}

impl AnyNode {
    #[inline]
    pub fn idx(self) -> MemIdx {
        match self {
            AnyNode::Box(p) => p.idx(),
            AnyNode::Rule(p) => p.idx(),
            AnyNode::Ins(p) => p.idx(),
            AnyNode::Mark(p) => p.idx(),
            AnyNode::Adjust(p) => p.idx(),
            AnyNode::Ligature(p) => p.idx(),
            AnyNode::Disc(p) => p.idx(),
            AnyNode::Whatsit(p) => p.idx(),
            AnyNode::Math(p) => p.idx(),
            AnyNode::Glue(p) => p.idx(),
            AnyNode::Kern(p) => p.idx(),
            AnyNode::Penalty(p) => p.idx(),
            AnyNode::Unset(p) => p.idx(),
            AnyNode::Style(p) => p.idx(),
            AnyNode::Choice(p) => p.idx(),
            AnyNode::Noad(p) => p.idx(),
            AnyNode::MarginKern(p) => p.idx(),
        }
    }

    /// TeX's `precedes_break`: a type below `math_node`, after which glue is a
    /// legal breakpoint.
    #[inline]
    pub fn precedes_break(self) -> bool {
        matches!(
            self,
            AnyNode::Box(_)
                | AnyNode::Rule(_)
                | AnyNode::Ins(_)
                | AnyNode::Mark(_)
                | AnyNode::Adjust(_)
                | AnyNode::Ligature(_)
                | AnyNode::Disc(_)
                | AnyNode::Whatsit(_)
        )
    }

    /// A node that may sit between the words of an interword space: a
    /// penalty, insertion, mark, adjust or I/O whatsit.
    #[inline]
    pub fn is_interword_filler(self, mem: &MemState) -> bool {
        match self {
            AnyNode::Penalty(_) | AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_) => true,
            AnyNode::Whatsit(w) => w.whatsit_subtype(mem).is_some_and(|s| s.is_io()),
            _ => false,
        }
    }

    /// The +1 width word of the kinds that add their width to a line, else 0.
    #[inline]
    pub fn hlist_width(self, mem: &MemState) -> Scaled {
        match self {
            AnyNode::Box(p) => mem[p].width,
            AnyNode::Rule(p) => mem[p].width.to_scaled(),
            AnyNode::Kern(p) => mem[p].width,
            AnyNode::Math(p) => mem[p].width,
            AnyNode::Whatsit(p) => p.sized(mem).map_or(Scaled::ZERO, |s| mem[s].width),
            _ => Scaled::ZERO,
        }
    }
}

impl MemState {
    /// The raw type code, for `\lastnodetype` arithmetic and diagnostics.
    #[inline]
    pub fn type_code(&self, p: impl MemAddr) -> u16 {
        self.b16s1(MemIdx::new(p))
    }

    /// The checked `NODE_type` of the non-null lo-mem node `p`.
    #[inline]
    pub fn node_type(&self, p: impl MemAddr) -> Result<NodeType, InvalidNodeType> {
        NodeType::from_raw(self.b16s1(MemIdx::new(p)))
    }

    /// Classifies the non-null lo-mem node `p`. `Err` for an invalid type code.
    #[inline]
    pub fn classify(&self, p: impl MemAddr) -> Result<AnyNode, InvalidNodeType> {
        use NodeType::*;
        let p = MemIdx::new(p);
        // SAFETY (every arm): the type code just read from `p` proves its kind.
        Ok(match self.node_type(p)? {
            HList | VList => AnyNode::Box(NodePtr::from_idx_unchecked(p)),
            Rule => AnyNode::Rule(NodePtr::from_idx_unchecked(p)),
            Ins => AnyNode::Ins(NodePtr::from_idx_unchecked(p)),
            Mark => AnyNode::Mark(NodePtr::from_idx_unchecked(p)),
            Adjust => AnyNode::Adjust(NodePtr::from_idx_unchecked(p)),
            Ligature => AnyNode::Ligature(NodePtr::from_idx_unchecked(p)),
            Disc => AnyNode::Disc(NodePtr::from_idx_unchecked(p)),
            Whatsit => AnyNode::Whatsit(NodePtr::from_idx_unchecked(p)),
            Math => AnyNode::Math(NodePtr::from_idx_unchecked(p)),
            Glue => AnyNode::Glue(NodePtr::from_idx_unchecked(p)),
            Kern => AnyNode::Kern(NodePtr::from_idx_unchecked(p)),
            Penalty => AnyNode::Penalty(NodePtr::from_idx_unchecked(p)),
            Unset => AnyNode::Unset(NodePtr::from_idx_unchecked(p)),
            Style => AnyNode::Style(NodePtr::from_idx_unchecked(p)),
            Choice => AnyNode::Choice(NodePtr::from_idx_unchecked(p)),
            OrdNoad | OpNoad | BinNoad | RelNoad | OpenNoad | CloseNoad | PunctNoad | InnerNoad
            | RadicalNoad | FractionNoad | UnderNoad | OverNoad | AccentNoad | VcenterNoad
            | LeftNoad | RightNoad => AnyNode::Noad(NodePtr::from_idx_unchecked(p)),
            MarginKern => AnyNode::MarginKern(NodePtr::from_idx_unchecked(p)),
        })
    }

    /// Classifies `p` unless it is null or a character.
    #[inline]
    pub fn classify_non_char(&self, p: MemPtr) -> Option<AnyNode> {
        if p.is_null() || p >= self.hi_mem_min {
            return None;
        }
        self.classify(p).ok()
    }

    /// `p` as a native-word whatsit, if it is one.
    #[inline]
    pub fn as_native_word(&self, p: MemPtr) -> Option<NodePtr<NativeWordNode>> {
        match self.classify_non_char(p)? {
            AnyNode::Whatsit(w) => match w.variant(self)? {
                WhatsitVariant::NativeWord(word) => Some(word),
                _ => None,
            },
            _ => None,
        }
    }

    /// `Ok(None)` for a valid non-box node.
    #[inline]
    pub fn classify_box(
        &self,
        p: impl MemAddr,
    ) -> Result<Option<NodePtr<BoxNode>>, InvalidNodeType> {
        Ok(match self.classify(p)? {
            AnyNode::Box(b) => Some(b),
            _ => None,
        })
    }

    #[inline]
    pub fn classify_disc(
        &self,
        p: impl MemAddr,
    ) -> Result<Option<NodePtr<DiscNode>>, InvalidNodeType> {
        Ok(match self.classify(p)? {
            AnyNode::Disc(d) => Some(d),
            _ => None,
        })
    }

    #[inline]
    pub fn classify_noad(
        &self,
        p: impl MemAddr,
    ) -> Result<Option<NodePtr<AnyNoad>>, InvalidNodeType> {
        Ok(match self.classify(p)? {
            AnyNode::Noad(n) => Some(n),
            _ => None,
        })
    }
}

// --- character-bearing nodes ---

impl MemState {
    /// `p` as a character node, if it lies in the one-word region.
    #[inline]
    pub fn as_char(&self, p: MemPtr) -> Option<NodePtr<CharNode>> {
        (p >= self.hi_mem_min).then(|| NodePtr::at(p))
    }
}

impl NodePtr<LigatureNode> {
    /// The font and character word, laid out like a character node.
    #[inline]
    pub fn char_word(self) -> NodePtr<CharNode> {
        NodePtr::at(self.idx + 1)
    }
}

// --- math, glue, kern, penalty ---

impl NodePtr<MathNode> {
    /// Retypes the node as a kern of the same width.
    #[inline]
    pub fn into_kern(self, mem: &mut MemState) -> NodePtr<KernNode> {
        retype(self, mem, NodeType::Kern)
    }
}

impl NodePtr<StyleNode> {
    /// Reads a type-14 node of a shipped list as the edge it is.
    #[inline]
    pub fn as_edge(self) -> NodePtr<EdgeNode> {
        NodePtr::from_idx_unchecked(self.idx)
    }
}

impl NodePtr<ChoiceNode> {
    /// Retypes the node as a style node for `style`.
    #[inline]
    pub fn into_style(self, mem: &mut MemState, style: MathStyle) -> NodePtr<StyleNode> {
        let p: NodePtr<StyleNode> = retype(self, mem, NodeType::Style);
        let layout = &mut mem[p];
        layout.style = style;
        layout.clear_payload();
        p
    }
}

/// A noad kind, which widens to [`AnyNoad`] for the words every noad shares.
pub trait NoadKind: NodeKind {}

impl_traits!(NoadKind: Noad, RadicalNoad, AccentNoad, FractionNoad, FenceNoad);

impl<K: NoadKind> NodePtr<K> {
    /// The noad as any noad, for its header and translated hlist.
    #[inline]
    pub fn any(self) -> NodePtr<AnyNoad> {
        NodePtr::from_idx_unchecked(self.idx)
    }
}

/// A noad by its layout, read off its type code.
#[derive(Copy, Clone, Debug)]
pub enum NoadVariant {
    Noad(NodePtr<Noad>),
    Radical(NodePtr<RadicalNoad>),
    Accent(NodePtr<AccentNoad>),
    Fraction(NodePtr<FractionNoad>),
    Fence(NodePtr<FenceNoad>),
}

impl NoadVariant {
    /// The nucleus and scripts of a noad that has them, all but fractions and fences.
    #[inline]
    pub fn as_noad(self) -> Option<NodePtr<Noad>> {
        match self {
            NoadVariant::Noad(n) => Some(n),
            NoadVariant::Radical(r) => Some(r.as_noad()),
            NoadVariant::Accent(a) => Some(a.as_noad()),
            NoadVariant::Fraction(_) | NoadVariant::Fence(_) => None,
        }
    }

    /// The words the noad occupies.
    #[inline]
    pub fn size(self) -> i32 {
        match self {
            NoadVariant::Noad(_) => Noad::SIZE,
            NoadVariant::Radical(_) => RadicalNoad::SIZE,
            NoadVariant::Accent(_) => AccentNoad::SIZE,
            NoadVariant::Fraction(_) => FractionNoad::SIZE,
            NoadVariant::Fence(_) => FenceNoad::SIZE,
        }
    }
}

impl NodePtr<AnyNoad> {
    /// The noad's layout, from its type code.
    #[inline]
    pub fn variant(self, mem: &MemState) -> NoadVariant {
        let p = self.idx;
        match mem[self].noad_type.get() {
            Some(NodeType::RadicalNoad) => NoadVariant::Radical(NodePtr::from_idx_unchecked(p)),
            Some(NodeType::AccentNoad) => NoadVariant::Accent(NodePtr::from_idx_unchecked(p)),
            Some(NodeType::FractionNoad) => NoadVariant::Fraction(NodePtr::from_idx_unchecked(p)),
            Some(NodeType::LeftNoad | NodeType::RightNoad) => {
                NoadVariant::Fence(NodePtr::from_idx_unchecked(p))
            }
            _ => NoadVariant::Noad(NodePtr::from_idx_unchecked(p)),
        }
    }
}

impl NodePtr<Noad> {
    #[inline]
    pub fn nucleus(self) -> NodePtr<MathField> {
        NodePtr::at(self.idx + 1)
    }
    #[inline]
    pub fn supscr(self) -> NodePtr<MathField> {
        NodePtr::at(self.idx + 2)
    }
    #[inline]
    pub fn subscr(self) -> NodePtr<MathField> {
        NodePtr::at(self.idx + 3)
    }
}

impl NodePtr<RadicalNoad> {
    /// The radical's nucleus and scripts, laid out as a plain noad.
    #[inline]
    pub fn as_noad(self) -> NodePtr<Noad> {
        NodePtr::from_idx_unchecked(self.idx)
    }
    #[inline]
    pub fn left_delimiter(self) -> NodePtr<DelimiterField> {
        NodePtr::at(self.idx + 4)
    }
}

impl NodePtr<AccentNoad> {
    /// The accentee and scripts, laid out as a plain noad.
    #[inline]
    pub fn as_noad(self) -> NodePtr<Noad> {
        NodePtr::from_idx_unchecked(self.idx)
    }
    #[inline]
    pub fn accent_chr(self) -> NodePtr<MathField> {
        NodePtr::at(self.idx + 4)
    }
}

impl NodePtr<FractionNoad> {
    #[inline]
    pub fn numerator(self) -> NodePtr<MathField> {
        NodePtr::at(self.idx + 2)
    }
    #[inline]
    pub fn denominator(self) -> NodePtr<MathField> {
        NodePtr::at(self.idx + 3)
    }
    #[inline]
    pub fn left_delimiter(self) -> NodePtr<DelimiterField> {
        NodePtr::at(self.idx + 4)
    }
    #[inline]
    pub fn right_delimiter(self) -> NodePtr<DelimiterField> {
        NodePtr::at(self.idx + 5)
    }
}

impl NodePtr<FenceNoad> {
    #[inline]
    pub fn delimiter(self) -> NodePtr<DelimiterField> {
        NodePtr::at(self.idx + 1)
    }
    #[inline]
    pub fn supscr(self) -> NodePtr<MathField> {
        NodePtr::at(self.idx + 2)
    }
    #[inline]
    pub fn subscr(self) -> NodePtr<MathField> {
        NodePtr::at(self.idx + 3)
    }
}

impl NodePtr<GlueNode> {
    #[inline]
    pub fn into_penalty(self, mem: &mut MemState) -> NodePtr<PenaltyNode> {
        retype(self, mem, NodeType::Penalty)
    }

    /// Retypes the node as a kern of width `w`.
    #[inline]
    pub fn into_kern(self, mem: &mut MemState, w: Scaled) -> NodePtr<KernNode> {
        let kern: NodePtr<KernNode> = retype(self, mem, NodeType::Kern);
        mem[kern].width = w;
        kern
    }
}

impl NodePtr<GlueSpecNode> {
    /// The spec at `p`, `None` when the pointer is null.
    #[inline]
    pub fn from_spec_link(p: MemPtr) -> Option<NodePtr<GlueSpecNode>> {
        p.as_idx().map(NodePtr::from_idx_unchecked)
    }

    /// The spec a glue pointer references. Glue pointers are never null.
    #[inline]
    pub fn from_glue_ptr(p: MemPtr) -> NodePtr<GlueSpecNode> {
        let idx = p.as_idx().expect("null glue spec");
        NodePtr::from_idx_unchecked(idx)
    }
}

// --- page-builder nodes ---

impl NodePtr<UnsetNode> {
    /// Turns the finished unset node into a box of orientation `o`.
    #[inline]
    pub fn into_box(self, mem: &mut MemState, o: Orientation) -> NodePtr<BoxNode> {
        retype(self, mem, o.node_type())
    }
}

// --- alignments ---

/// A column of an alignment preamble or row. Columns alternate with tabskip glue.
pub trait AlignColumn: HasLink {}

impl_traits!(AlignColumn: AlignRecord, UnsetNode);

impl<K: AlignColumn> NodePtr<K> {
    /// The column after tabskip glue `g`, `None` at the end of the list.
    #[inline]
    pub fn after_tab_skip(mem: &MemState, g: MemPtr) -> Option<Self> {
        // SAFETY: tabskip glue is followed by a column or ends the list.
        g.link(mem)
            .as_idx()
            .map(|idx| NodePtr::from_idx_unchecked(idx))
    }

    #[inline]
    pub fn tab_skip(self, mem: &MemState) -> NodePtr<GlueNode> {
        NodePtr::tab_skip_after(mem, self.ptr())
    }

    #[inline]
    pub fn next_column(self, mem: &MemState) -> Option<Self> {
        Self::after_tab_skip(mem, self.ptr().link(mem))
    }
}

impl NodePtr<GlueNode> {
    /// The tabskip glue linked from `ALIGN_HEAD` or an alignment column `p`.
    #[inline]
    pub fn tab_skip_after(mem: &MemState, p: MemPtr) -> Self {
        NodePtr::from_idx_unchecked(MemIdx::new(p.link(mem)))
    }
}

impl NodePtr<AlignRecord> {
    /// The column viewed as the head of its span list.
    #[inline]
    pub fn span_head(self) -> NodePtr<SpanNode> {
        NodePtr::at(self.idx)
    }

    /// Turns the record into an empty unset box of its final width.
    pub fn into_unset(self, mem: &mut MemState) -> NodePtr<UnsetNode> {
        let u: NodePtr<UnsetNode> = retype(self, mem, NodeType::Unset);
        let node = &mut mem[u];
        node.span_count = 0;
        node.height = Scaled::ZERO;
        node.depth = Scaled::ZERO;
        node.glue_order = (GlueOrder::Normal).into();
        node.glue_sign = (GlueOrder::Normal).into();
        node.glue_stretch = Scaled::ZERO;
        node.glue_shrink = Scaled::ZERO;
        u
    }
}

impl NodePtr<ParShapeNode> {
    /// Line `k`, counting from 1. Panics outside the shape.
    #[inline]
    pub fn line(self, mem: &MemState, k: i32) -> crate::overlay::ParShapeLine {
        mem.entries(self)[usize::try_from(k - 1).unwrap_or(usize::MAX)]
    }
}

impl NodePtr<PenaltiesNode> {
    /// Entry `k`, counting from 1, or the count for 0 as e-TeX reads it.
    /// Panics past the entries.
    #[inline]
    pub fn penalty(self, mem: &MemState, k: i32) -> i32 {
        if k == 0 {
            return mem[self].count;
        }
        mem.entries(self)[usize::try_from(k - 1).unwrap_or(usize::MAX)].value
    }
}

// --- whatsits ---

/// The subtype of a whatsit node. Only `NativeWord`..`Pdf` carry dimensions.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum WhatsitSubtype {
    Open = 0,
    Write = 1,
    Close = 2,
    Special = 3,
    Language = 4,
    PdfSavePos = 21,
    NativeWord = 40,
    NativeWordAt = 41,
    Glyph = 42,
    Pic = 43,
    Pdf = 44,
}

impl WhatsitSubtype {
    pub fn from_raw(v: u16) -> Option<WhatsitSubtype> {
        Some(match v {
            0 => WhatsitSubtype::Open,
            1 => WhatsitSubtype::Write,
            2 => WhatsitSubtype::Close,
            3 => WhatsitSubtype::Special,
            4 => WhatsitSubtype::Language,
            21 => WhatsitSubtype::PdfSavePos,
            40 => WhatsitSubtype::NativeWord,
            41 => WhatsitSubtype::NativeWordAt,
            42 => WhatsitSubtype::Glyph,
            43 => WhatsitSubtype::Pic,
            44 => WhatsitSubtype::Pdf,
            _ => return None,
        })
    }

    pub fn has_dimensions(self) -> bool {
        matches!(
            self,
            WhatsitSubtype::NativeWord
                | WhatsitSubtype::NativeWordAt
                | WhatsitSubtype::Glyph
                | WhatsitSubtype::Pic
                | WhatsitSubtype::Pdf
        )
    }

    pub fn is_native_word(self) -> bool {
        matches!(
            self,
            WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt
        )
    }

    /// `\openout`, `\write`, `\closeout`, `\special` and `\setlanguage`.
    pub fn is_io(self) -> bool {
        matches!(
            self,
            WhatsitSubtype::Open
                | WhatsitSubtype::Write
                | WhatsitSubtype::Close
                | WhatsitSubtype::Special
                | WhatsitSubtype::Language
        )
    }
}

impl StoredCode for WhatsitSubtype {
    #[inline]
    fn from_u16(raw: u16) -> Option<Self> {
        WhatsitSubtype::from_raw(raw)
    }
    #[inline]
    fn to_u16(self) -> u16 {
        self as u16
    }
}

/// A whatsit kind, which widens to [`WhatsitNode`].
pub trait WhatsitKind: NodeKind {}

impl_traits!(WhatsitKind: WhatsitNode, OpenNode, WriteNode, LanguageNode, GlyphNode, PicNode,
    NativeWordNode);

impl<K: WhatsitKind> NodePtr<K> {
    /// A fresh `size`-word whatsit of the given subtype, other fields uninitialised.
    #[inline]
    pub fn alloc_sized(
        mem: &mut MemState,
        size: i32,
        subtype: WhatsitSubtype,
    ) -> Result<Self, Abort> {
        let p: Self = mem.alloc_node(size, Some(NodeType::Whatsit))?;
        mem[p.whatsit()].subtype = subtype.into();
        Ok(p)
    }

    /// The whatsit as any whatsit, for its header.
    #[inline]
    pub fn whatsit(self) -> NodePtr<WhatsitNode> {
        NodePtr::from_idx_unchecked(self.idx)
    }
}

impl<K: WhatsitKind + FixedNode> NodePtr<K> {
    /// A fresh whatsit of the given subtype, other fields uninitialised.
    #[inline]
    pub fn alloc_whatsit(mem: &mut MemState, subtype: WhatsitSubtype) -> Result<Self, Abort> {
        Self::alloc_sized(mem, K::SIZE, subtype)
    }
}

/// A whatsit by its layout, read off its subtype.
#[derive(Copy, Clone, Debug)]
pub enum WhatsitVariant {
    Open(NodePtr<OpenNode>),
    Write(NodePtr<WriteNode>),
    Close(NodePtr<WriteNode>),
    Special(NodePtr<WriteNode>),
    Language(NodePtr<LanguageNode>),
    SavePos,
    NativeWord(NodePtr<NativeWordNode>),
    Glyph(NodePtr<GlyphNode>),
    Pic(NodePtr<PicNode>),
}

impl NodePtr<WhatsitNode> {
    /// The whatsit's layout, from its subtype. `None` for an unknown subtype.
    #[inline]
    pub fn variant(self, mem: &MemState) -> Option<WhatsitVariant> {
        use WhatsitSubtype as S;
        let p = self.idx;
        Some(match mem[self].subtype.get()? {
            S::Open => WhatsitVariant::Open(NodePtr::from_idx_unchecked(p)),
            S::Write => WhatsitVariant::Write(NodePtr::from_idx_unchecked(p)),
            S::Close => WhatsitVariant::Close(NodePtr::from_idx_unchecked(p)),
            S::Special => WhatsitVariant::Special(NodePtr::from_idx_unchecked(p)),
            S::Language => WhatsitVariant::Language(NodePtr::from_idx_unchecked(p)),
            S::PdfSavePos => WhatsitVariant::SavePos,
            S::NativeWord | S::NativeWordAt => {
                WhatsitVariant::NativeWord(NodePtr::from_idx_unchecked(p))
            }
            S::Glyph => WhatsitVariant::Glyph(NodePtr::from_idx_unchecked(p)),
            S::Pic | S::Pdf => WhatsitVariant::Pic(NodePtr::from_idx_unchecked(p)),
        })
    }

    /// The whatsit's dimensions, for a native word, glyph or picture.
    #[inline]
    pub fn sized(self, mem: &MemState) -> Option<NodePtr<SizedWhatsit>> {
        mem[self]
            .subtype
            .get()?
            .has_dimensions()
            .then_some(NodePtr::from_idx_unchecked(self.idx))
    }

    #[inline]
    pub fn whatsit_subtype(self, mem: &MemState) -> Option<WhatsitSubtype> {
        WhatsitSubtype::from_raw(mem.b16s0(self.idx))
    }
}

impl NodePtr<NativeWordNode> {
    /// Drops the shaped-glyph payload of a native word.
    pub fn release_native_glyphs(self, mem: &mut MemState) {
        let handle = mem[self].glyph_info;
        if handle != 0 {
            mem.glyph_blobs.release(handle);
            mem[self].clear_glyphs();
        }
    }

    /// Gives this copy of `src` its own handle on `src`'s shaped glyphs.
    pub fn share_native_glyphs(self, mem: &mut MemState, src: Self) {
        let src = mem[src];
        if src.glyph_info != 0 {
            let copy = mem.glyph_blobs.clone_handle(src.glyph_info);
            let word = &mut mem[self];
            word.glyph_info = copy;
            word.glyph_count = src.glyph_count;
        }
    }

    /// Frees a native word together with its shaped glyphs.
    pub fn free_native(self, mem: &mut MemState) {
        self.release_native_glyphs(mem);
        mem.free_node(self.ptr(), i32::from(mem[self].size));
    }
}

// --- eTeX sparse arrays ---

/// A sparse-array kind that hangs in a tree slot.
pub trait SaChild: NodeKind {}

impl_traits!(SaChild: SaIndexNode, SaElement, SaWordNode, SaPointerNode, SaMarkClassNode);

/// A sparse-array element kind, which widens to [`SaElement`].
pub trait SaElementKind: NodeKind {}

impl_traits!(SaElementKind: SaWordNode, SaPointerNode, SaMarkClassNode);

impl<K: SaElementKind> NodePtr<K> {
    /// The element as any element, for its header.
    #[inline]
    pub fn element(self) -> NodePtr<SaElement> {
        NodePtr::from_idx_unchecked(self.idx)
    }
}

/// A sparse-array element by its layout, read off its type.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SaLeaf {
    Word(NodePtr<SaWordNode>),
    Pointer(NodePtr<SaPointerNode>),
    MarkClass(NodePtr<SaMarkClassNode>),
}

impl SaLeaf {
    /// The leaf layout of an element of type `t` at `idx`.
    #[inline]
    pub fn of_type(t: crate::codes::ValueLevel, idx: MemIdx) -> SaLeaf {
        use crate::codes::ValueLevel;
        match t {
            ValueLevel::Int | ValueLevel::Dimen => SaLeaf::Word(NodePtr::from_idx_unchecked(idx)),
            ValueLevel::Mark => SaLeaf::MarkClass(NodePtr::from_idx_unchecked(idx)),
            _ => SaLeaf::Pointer(NodePtr::from_idx_unchecked(idx)),
        }
    }

    #[inline]
    pub fn element(self) -> NodePtr<SaElement> {
        match self {
            SaLeaf::Word(p) => p.element(),
            SaLeaf::Pointer(p) => p.element(),
            SaLeaf::MarkClass(p) => p.element(),
        }
    }
}

impl NodePtr<SaElement> {
    /// The element's layout, from the type in its header. `None` for a type
    /// outside the sparse-array set.
    #[inline]
    pub fn leaf(self, mem: &MemState) -> Option<SaLeaf> {
        Some(SaLeaf::of_type(mem[self].sa_type().get()?, self.idx))
    }

    #[inline]
    pub fn word(self, mem: &MemState) -> Option<NodePtr<SaWordNode>> {
        match self.leaf(mem)? {
            SaLeaf::Word(p) => Some(p),
            _ => None,
        }
    }
}

/// A save-chain entry by its layout.
#[derive(Copy, Clone, Debug)]
pub enum SaSavedEntry {
    Word(NodePtr<SaSavedWord>),
    Pointer(NodePtr<SaSavedPointer>),
}

impl NodePtr<SaSaved> {
    /// The entry's layout, from the type in its header.
    #[inline]
    pub fn entry(self, mem: &MemState) -> SaSavedEntry {
        if mem[self].sa_index < crate::save::DIMEN_VAL_LIMIT {
            SaSavedEntry::Word(NodePtr::from_idx_unchecked(self.idx))
        } else {
            SaSavedEntry::Pointer(NodePtr::from_idx_unchecked(self.idx))
        }
    }
}

impl NodePtr<SaSavedWord> {
    #[inline]
    pub fn saved(self) -> NodePtr<SaSaved> {
        NodePtr::from_idx_unchecked(self.idx)
    }
}

impl NodePtr<SaSavedPointer> {
    #[inline]
    pub fn saved(self) -> NodePtr<SaSaved> {
        NodePtr::from_idx_unchecked(self.idx)
    }
}

// --- the line breaker's active, delta and passive lists ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::LimitCode;
    use crate::consts::MAX_HALFWORD;
    use crate::token::TokenList;
    use crate::units::FontId;
    use std::mem::size_of;

    fn fresh_mem(n: usize) -> MemState {
        MemState {
            data: vec![Default::default(); n],
            hi_mem_min: MemPtr::from_raw(1_000_000),
            ..Default::default()
        }
    }

    /// Checks a code set against WEB's numbers, which formats and checkpoints store raw.
    fn assert_web_codes<T: StoredCode + PartialEq + std::fmt::Debug>(codes: &[(T, u16)]) {
        for &(code, raw) in codes {
            assert_eq!(code.to_u16(), raw, "{code:?}");
            assert_eq!(T::from_u16(raw), Some(code), "{raw}");
        }
        let unknown = (0..=u16::MAX).find(|n| codes.iter().all(|&(_, raw)| raw != *n));
        if let Some(n) = unknown {
            assert_eq!(T::from_u16(n), None);
            assert_eq!(Stored::<T>::from_raw(n).raw(), n);
        }
    }

    #[test]
    fn kern_subtypes_keep_web_codes() {
        assert_web_codes(&[
            (KernSubtype::Normal, 0),
            (KernSubtype::Explicit, 1),
            (KernSubtype::AccKern, 2),
            (KernSubtype::SpaceAdjustment, 3),
            (KernSubtype::MuGlue, 99),
        ]);
    }

    #[test]
    fn adjust_placements_keep_web_codes() {
        assert_web_codes(&[(AdjustPlacement::Post, 0), (AdjustPlacement::Pre, 1)]);
    }

    #[test]
    fn edge_directions_keep_web_codes() {
        assert_web_codes(&[(Dir::Ltr, 0), (Dir::Rtl, 1)]);
    }

    #[test]
    fn noad_subtypes_keep_web_codes() {
        assert_web_codes(&[
            (LimitCode::Normal, 0),
            (LimitCode::Limits, 1),
            (LimitCode::NoLimits, 2),
        ]);
        assert_web_codes(&[
            (MathAccentKind::Top, 0),
            (MathAccentKind::TopFixed, 1),
            (MathAccentKind::Bottom, 2),
            (MathAccentKind::BottomFixed, 3),
        ]);
        assert_web_codes(&[(FenceSubtype::Normal, 0), (FenceSubtype::Middle, 1)]);
    }

    #[test]
    fn ligature_boundaries_keep_web_codes_and_bits() {
        use LigBoundary as B;
        assert_web_codes(&[(B::None, 0), (B::Right, 1), (B::Left, 2), (B::Both, 3)]);
        let added = [B::None, B::Right, B::Left, B::Both].map(B::with_right);
        assert_eq!(added, [B::Right, B::Right, B::Both, B::Both]);
        let bits = (0..6).map(|raw| {
            let b = Stored::<B>::from_raw(raw);
            (b.has_left(), b.has_right())
        });
        assert_eq!(
            bits.collect::<Vec<_>>(),
            [
                (false, false),
                (false, true),
                (true, false),
                (true, true),
                (true, false),
                (true, true)
            ]
        );
    }

    #[test]
    fn math_codes_keep_web_codes_and_arithmetic() {
        use MathCode as M;
        let all = [
            (M::Before, 0),
            (M::After, 1),
            (M::BeginM, 2),
            (M::EndM, 3),
            (M::BeginL, 6),
            (M::EndL, 7),
            (M::BeginR, 10),
            (M::EndR, 11),
        ];
        assert_web_codes(&all);
        for (code, raw) in all {
            assert_eq!(code.is_end(), raw % 2 == 1, "{code:?}");
            assert_eq!(code.is_lr(), raw >= 4, "{code:?}");
            assert_eq!(
                code.dir(),
                if raw >= 8 { Dir::Rtl } else { Dir::Ltr },
                "{code:?}"
            );
            // WEB's end_LR_type and the `subtype - 1`/`+ 1` flips, on the raw codes.
            assert_eq!(
                Stored::from(code.lr_end()).raw(),
                4 * (raw / 4) + 3,
                "{code:?}"
            );
            assert_eq!(
                Stored::from(code.as_begin()).raw(),
                raw - raw % 2,
                "{code:?}"
            );
            assert_eq!(Stored::from(code.as_end()).raw(), raw | 1, "{code:?}");
        }
    }

    #[test]
    fn glue_kinds_keep_web_codes() {
        use LeaderKind as L;
        assert_web_codes(&[(L::Aligned, 100), (L::Centered, 101), (L::Expanded, 102)]);
        let pairs = [
            (GlueKind::Normal, 0),
            (GlueKind::Param(GluePar::LineSkip), 1),
            (GlueKind::Param(GluePar::TabSkip), 12),
            (GlueKind::Param(GluePar::ThickMuSkip), 19),
            (GlueKind::CondMath, 98),
            (GlueKind::Mu, 99),
            (GlueKind::Leaders(L::Aligned), 100),
            (GlueKind::Leaders(L::Centered), 101),
            (GlueKind::Leaders(L::Expanded), 102),
        ];
        for (kind, raw) in pairs {
            assert_eq!(kind.to_u16(), raw, "{kind:?}");
            assert_eq!(GlueKind::from_u16(raw), Some(kind), "{raw}");
        }
        let valid: Vec<u16> = (0..=u16::MAX)
            .filter(|&n| GlueKind::from_u16(n).is_some())
            .collect();
        let expected: Vec<u16> = (0..=19).chain(98..=102).collect();
        assert_eq!(valid, expected);
        for n in valid {
            assert_eq!(GlueKind::from_u16(n).map(GlueKind::to_u16), Some(n));
        }
    }

    #[test]
    fn box_and_margin_codes_keep_web_codes() {
        assert_web_codes(&[(BoxLr::Normal, 0), (BoxLr::Reversed, 1), (BoxLr::Dlist, 2)]);
        assert_web_codes(&[(Side::Left, 0), (Side::Right, 1)]);
    }

    #[test]
    fn alloc_zeroes_the_subtype_with_the_type() {
        let mut mem = arena_with_free_block();
        for i in 20..30 {
            mem.set_b16s0(MemPtr::from_raw(i), 0xBEEF);
        }
        let p = NodePtr::<PenaltyNode>::alloc(&mut mem).unwrap();
        assert_eq!(mem.b16s0(p.idx()), 0);
        assert_eq!(mem.b16s1(p.idx()), NodeType::Penalty.as_raw());
    }

    /// An arena whose free list is the single block `[10, 30)`.
    fn arena_with_free_block() -> MemState {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = MemPtr::from_raw(rover);
        mem.lo_mem_max = MemPtr::from_raw(30);
        mem.hi_mem_min = MemPtr::from_raw(60);
        mem.set_b32s0(MemPtr::from_raw(rover), 20);
        mem.set_b32s1(MemPtr::from_raw(rover), MAX_HALFWORD);
        mem.set_b32s1(MemPtr::from_raw(rover + 1), rover);
        mem.set_b32s0(MemPtr::from_raw(rover + 1), rover);
        mem.set_b32s1(MemPtr::from_raw(30), 0);
        mem
    }

    fn at<K: NodeKind>(mem: &mut MemState, p: i32, ty: Option<NodeType>) -> NodePtr<K> {
        if let Some(ty) = ty {
            mem.set_b16s1(MemPtr::from_raw(p), ty.as_raw());
        }
        NodePtr::from_idx_unchecked(MemIdx::new(MemPtr::from_raw(p)))
    }

    #[test]
    fn handles_are_zero_cost() {
        assert_eq!(size_of::<NodePtr<BoxNode>>(), size_of::<i32>());
        assert_eq!(size_of::<NodePtr<GlueSpecNode>>(), size_of::<i32>());
        assert_eq!(size_of::<NodePtr<Noad>>(), size_of::<i32>());
    }

    #[test]
    fn alloc_stamps_the_type_that_classify_reads_back() {
        let mut mem = arena_with_free_block();
        let h = NodePtr::<BoxNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(h.idx()), Ok(AnyNode::Box(b)) if mem[b].is_hlist()));
        let mut mem = arena_with_free_block();
        let r = NodePtr::<RuleNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(r.idx()), Ok(AnyNode::Rule(_))));
        let l = NodePtr::<LigatureNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(l.idx()), Ok(AnyNode::Ligature(_))));
        let d = NodePtr::<DiscNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(d.idx()), Ok(AnyNode::Disc(_))));
        let m = NodePtr::<MathNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(m.idx()), Ok(AnyNode::Math(_))));
        let s = NodePtr::<StyleNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(s.idx()), Ok(AnyNode::Style(_))));

        let mut mem = arena_with_free_block();
        let c = NodePtr::<ChoiceNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(c.idx()), Ok(AnyNode::Choice(_))));
        let g = NodePtr::<GlueNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(g.idx()), Ok(AnyNode::Glue(_))));
        let k = NodePtr::<KernNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(k.idx()), Ok(AnyNode::Kern(_))));
        let p = NodePtr::<PenaltyNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(p.idx()), Ok(AnyNode::Penalty(_))));
        let mk = NodePtr::<MarginKernNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(mk.idx()), Ok(AnyNode::MarginKern(_))));

        let mut mem = arena_with_free_block();
        let n = NodePtr::<Noad>::alloc(&mut mem).unwrap();
        assert_eq!(mem[n].noad_type, NodeType::OrdNoad);
        assert!(matches!(n.any().variant(&mem), NoadVariant::Noad(_)));
        let r = NodePtr::<RadicalNoad>::alloc(&mut mem).unwrap();
        assert!(matches!(r.any().variant(&mem), NoadVariant::Radical(_)));
        let a = NodePtr::<AccentNoad>::alloc(&mut mem).unwrap();
        assert!(matches!(a.any().variant(&mem), NoadVariant::Accent(_)));
        let f = NodePtr::<FractionNoad>::alloc(&mut mem).unwrap();
        assert!(matches!(
            mem.classify(f.idx()),
            Ok(AnyNode::Noad(any)) if matches!(any.variant(&mem), NoadVariant::Fraction(_))
        ));

        let mut mem = arena_with_free_block();
        let l = NodePtr::<FenceNoad>::alloc(&mut mem).unwrap();
        assert_eq!(mem[l].noad_type, NodeType::LeftNoad);
        assert!(matches!(l.any().variant(&mem), NoadVariant::Fence(_)));
        mem[l].noad_type = NodeType::RightNoad.into();
        assert!(matches!(l.any().variant(&mem), NoadVariant::Fence(_)));
    }

    #[test]
    fn stored_codes_keep_bits_outside_their_code_set() {
        let mut mem = arena_with_free_block();
        let s = NodePtr::<GlueSpecNode>::alloc(&mut mem).unwrap();
        mem[s].stretch_order = Stored::from_raw(7);
        mem[s].shrink_order = GlueOrder::Fill.into();
        let stretch = mem[s].stretch_order;
        assert_eq!(stretch.raw(), 7);
        assert_eq!(stretch.get(), None);
        assert!(stretch > GlueOrder::Filll);
        assert_eq!(mem[s].shrink_order, GlueOrder::Fill);
        assert_eq!(mem[s].shrink_order.get(), Some(GlueOrder::Fill));
        assert!(mem[s].shrink_order < stretch);
    }

    #[test]
    fn headerless_records_alloc_without_stamping() {
        let mut mem = arena_with_free_block();
        let spec = NodePtr::<GlueSpecNode>::alloc(&mut mem).unwrap();
        mem[spec].width = Scaled::from_raw(0x1234);
        assert_eq!(mem[spec].width, Scaled::from_raw(0x1234));
        let item = NodePtr::<LigItem>::alloc(&mut mem).unwrap();
        mem[item].character = 45;
        mem[item].component = MemPtr::from_raw(34);
        assert_eq!(mem.b16s0(item.idx()), 45);
        assert_eq!(mem.b32s1(item.idx() + 1), 34);
    }

    #[test]
    fn sync_trailer_is_the_last_word() {
        let mut mem = fresh_mem(200);
        let b: NodePtr<BoxNode> = at(&mut mem, 100, Some(NodeType::HList));
        mem.set_b32s0(MemPtr::from_raw(100 + BOX_NODE_SIZE - 1), 42);
        mem.set_b32s1(MemPtr::from_raw(100 + BOX_NODE_SIZE - 1), 7);
        assert_eq!((mem[b].sync_tag, mem[b].sync_line), (42, 7));

        let k: NodePtr<KernNode> = at(&mut mem, 100, Some(NodeType::Kern));
        mem.set_b32s0(MemPtr::from_raw(100 + MEDIUM_NODE_SIZE - 1), 9);
        mem.set_b32s1(MemPtr::from_raw(100 + MEDIUM_NODE_SIZE - 1), 3);
        assert_eq!(mem[k].sync_tag, 9);
        assert_eq!(mem[k].sync_line, 3);
    }

    #[test]
    fn classify_dispatches_to_typed_handles() {
        let mut mem = fresh_mem(300);
        let cases = [
            (40, NodeType::HList),
            (50, NodeType::VList),
            (60, NodeType::Rule),
            (70, NodeType::Glue),
            (80, NodeType::Kern),
            (90, NodeType::Math),
            (100, NodeType::Penalty),
            (110, NodeType::Disc),
            (120, NodeType::Choice),
            (130, NodeType::MarginKern),
            (140, NodeType::OpNoad),
            (150, NodeType::FractionNoad),
            (160, NodeType::Whatsit),
            (170, NodeType::Unset),
        ];
        for (p, ty) in cases {
            mem.set_b16s1(MemPtr::from_raw(p), ty.as_raw());
        }
        mem.set_b16s1(MemPtr::from_raw(180), 35);
        let c = |p| mem.classify(MemPtr::from_raw(p));
        use AnyNode::*;
        assert!(matches!(c(40), Ok(Box(b)) if mem[b].is_hlist()));
        assert!(matches!(c(50), Ok(Box(b)) if mem[b].is_vlist()));
        assert!(matches!(c(60), Ok(Rule(_))));
        assert!(matches!(c(70), Ok(Glue(_))));
        assert!(matches!(c(80), Ok(Kern(_))));
        assert!(matches!(c(90), Ok(Math(_))));
        assert!(matches!(c(100), Ok(Penalty(_))));
        assert!(matches!(c(110), Ok(Disc(_))));
        assert!(matches!(c(120), Ok(Choice(_))));
        assert!(matches!(c(130), Ok(MarginKern(_))));
        assert!(matches!(c(140), Ok(Noad(_))));
        assert!(matches!(c(150), Ok(Noad(_))));
        assert!(matches!(c(160), Ok(Whatsit(_))));
        assert!(matches!(c(170), Ok(Unset(_))));
        assert!(c(180).is_err());
        assert_eq!(c(40).unwrap().idx().raw(), 40);

        assert!(matches!(
            mem.classify_box(MemPtr::from_raw(50)),
            Ok(Some(b)) if mem[b].is_vlist()
        ));
        assert!(matches!(mem.classify_box(MemPtr::from_raw(70)), Ok(None)));
        assert!(mem.classify_box(MemPtr::from_raw(180)).is_err());
    }

    #[test]
    fn rule_dimensions_model_the_running_sentinel() {
        let mut mem = fresh_mem(200);
        let r: NodePtr<RuleNode> = at(&mut mem, 100, Some(NodeType::Rule));
        mem[r].width = RuleDimen::Running.into();
        mem[r].depth = RuleDimen::Set(Scaled::ZERO).into();
        mem[r].height = RuleDimen::Set(Scaled::from_raw(0x2222)).into();
        assert_eq!(mem[r].width.get(), RuleDimen::Running);
        assert_eq!(mem[r].depth.get(), RuleDimen::Set(Scaled::ZERO));
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), NULL_FLAG);
        assert_eq!(mem.b32s1(MemPtr::from_raw(103)), 0x2222);
    }

    #[test]
    fn small_node_fields_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let lig: NodePtr<LigatureNode> = at(&mut mem, 100, Some(NodeType::Ligature));
        mem[lig].font = FontId::new(77);
        mem[lig].character = 0x1234;
        assert_eq!(mem[lig].font, FontId::new(77));
        assert_eq!(mem.b16s1(MemPtr::from_raw(101)), 77);
        assert_eq!(mem.b16s0(MemPtr::from_raw(101)), 0x1234);
        mem[lig].lig_ptr = MemPtr::from_raw(150);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 150);
        mem[lig].boundary = LigBoundary::Left.into();
        assert_eq!(mem.b16s0(MemPtr::from_raw(100)), 2);

        let disc: NodePtr<DiscNode> = at(&mut mem, 100, Some(NodeType::Disc));
        mem[disc].pre_break = MemPtr::from_raw(150);
        mem[disc].post_break = MemPtr::from_raw(160);
        assert_eq!(mem.b32s0(MemPtr::from_raw(101)), 150);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 160);

        let math: NodePtr<MathNode> = at(&mut mem, 100, Some(NodeType::Math));
        mem[math].subtype = MathCode::EndM.into();
        mem[math].width = Scaled::from_raw(0x1234);
        assert_eq!(mem.b16s0(MemPtr::from_raw(100)), 3);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 0x1234);

        let style: NodePtr<StyleNode> = at(&mut mem, 100, Some(NodeType::Style));
        mem.set_b32s1(MemPtr::from_raw(101), 9);
        mem.set_b32s1(MemPtr::from_raw(102), 9);
        mem[style].clear_payload();
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 0);
        assert_eq!(mem.b32s1(MemPtr::from_raw(102)), 0);
        mem[style].style = MathStyle::SCRIPT.cramped();
        assert_eq!(mem.b16s0(MemPtr::from_raw(100)), 5);

        let pen: NodePtr<PenaltyNode> = at(&mut mem, 100, Some(NodeType::Penalty));
        mem[pen].penalty = -10000;
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), -10000);

        let mark: NodePtr<MarkNode> = at(&mut mem, 100, Some(NodeType::Mark));
        mem[mark].mark_ptr = TokenList::from_ptr(MemPtr::from_raw(77));
        mem[mark].mark_class = 3;
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 77);
        assert_eq!(mem.b32s0(MemPtr::from_raw(101)), 3);

        let adj: NodePtr<AdjustNode> = at(&mut mem, 100, Some(NodeType::Adjust));
        mem[adj].adjust_ptr = MemPtr::from_raw(88);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 88);
        mem[adj].placement = AdjustPlacement::Pre.into();
        assert_eq!(mem.b16s0(MemPtr::from_raw(100)), 1);

        let ins: NodePtr<InsNode> = at(&mut mem, 100, Some(NodeType::Ins));
        mem[ins].ins_ptr = MemPtr::from_raw(55);
        mem[ins].split_top_ptr = NodePtr::from_glue_ptr(MemPtr::from_raw(66));
        assert_eq!(mem.b32s0(MemPtr::from_raw(104)), 55);
        assert_eq!(mem.b32s1(MemPtr::from_raw(104)), 66);
    }

    #[test]
    fn choice_branches_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let c: NodePtr<ChoiceNode> = at(&mut mem, 100, Some(NodeType::Choice));
        mem[c].display_mlist = MemPtr::from_raw(11);
        mem[c].text_mlist = MemPtr::from_raw(22);
        mem[c].script_mlist = MemPtr::from_raw(33);
        mem[c].script_script_mlist = MemPtr::from_raw(44);
        assert_eq!(mem.b32s0(MemPtr::from_raw(101)), 11);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 22);
        assert_eq!(mem.b32s0(MemPtr::from_raw(102)), 33);
        assert_eq!(mem.b32s1(MemPtr::from_raw(102)), 44);
    }

    #[test]
    fn alignment_records_walk_the_preamble_and_become_unset() {
        let mut mem = fresh_mem(200);
        let (head, g1, col, g2) = (
            MemPtr::from_raw(10),
            MemPtr::from_raw(20),
            MemPtr::from_raw(100),
            MemPtr::from_raw(30),
        );
        head.set_link(&mut mem, g1);
        g1.set_link(&mut mem, col);
        col.set_link(&mut mem, g2);
        g2.set_link(&mut mem, MemPtr::NULL);
        let rec = NodePtr::<AlignRecord>::after_tab_skip(&mem, g1).unwrap();
        assert_eq!(rec.ptr(), col);
        assert_eq!(rec.tab_skip(&mem).ptr(), g2);
        assert_eq!(NodePtr::<GlueNode>::tab_skip_after(&mem, head).ptr(), g1);
        assert!(rec.next_column(&mem).is_none());

        mem[rec].v_part = TokenList::from_ptr(MemPtr::from_raw(0x22));
        mem[rec].u_part = TokenList::from_ptr(MemPtr::from_raw(0x33));
        mem[rec].extra_info = 0x55;
        let head = rec.span_head();
        mem[head].next = NodePtr::at(MemPtr::from_raw(40));
        assert_eq!(
            (
                mem.b32s1(MemPtr::from_raw(102)),
                mem.b32s1(MemPtr::from_raw(103))
            ),
            (0x22, 0x33)
        );
        assert_eq!(
            (
                mem.b32s0(MemPtr::from_raw(105)),
                mem.b32s0(MemPtr::from_raw(100))
            ),
            (0x55, 40)
        );

        let u = rec.into_unset(&mut mem);
        assert_eq!(mem.b16s1(MemPtr::from_raw(100)), NodeType::Unset.as_raw());
        assert_eq!((mem[u].height, mem[u].depth), (Scaled::ZERO, Scaled::ZERO));
        assert_eq!(
            (
                mem.b16s0(MemPtr::from_raw(100)),
                mem.b32s1(MemPtr::from_raw(104)),
                mem.b32s1(MemPtr::from_raw(106))
            ),
            (0, 0, 0)
        );
        assert_eq!(mem[u].link, g2);

        let span: NodePtr<SpanNode> = NodePtr::at(MemPtr::from_raw(40));
        mem[span].span_count = 3;
        mem[span].width = Scaled::from_raw(0x66);
        assert_eq!(
            (
                mem.b32s1(MemPtr::from_raw(40)),
                mem.b32s1(MemPtr::from_raw(41))
            ),
            (3, 0x66)
        );
    }

    #[test]
    fn unset_and_whatsit_fields_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let u: NodePtr<UnsetNode> = at(&mut mem, 100, Some(NodeType::Unset));
        mem.set_b32s1(MemPtr::from_raw(101), 0x111);
        mem.set_b32s1(MemPtr::from_raw(103), 0x333);
        mem[u].list_ptr = MemPtr::from_raw(0x444);
        assert_eq!(mem[u].width, Scaled::from_raw(0x111));
        assert_eq!(mem[u].height, Scaled::from_raw(0x333));
        assert_eq!(mem.b32s1(MemPtr::from_raw(105)), 0x444);
    }

    #[test]
    fn sparse_array_records_land_in_their_words() {
        use crate::codes::{MarkCode, ValueLevel};
        let w = |p: i32| MemPtr::from_raw(p);
        let mut mem = fresh_mem(200);
        let q: NodePtr<SaIndexNode> = at(&mut mem, 100, None);
        let leaf: NodePtr<SaElement> = at(&mut mem, 150, None);
        let b = &mut mem[q];
        b.used = 7;
        b.sa_index = 5;
        b.link = Link::from(Some(q));
        b.set_child(0, Some(leaf));
        b.set_child(1, Some(q));
        b.set_child(63, Some(leaf));
        assert_eq!([mem.b16s0(w(100)), mem.b16s1(w(100))], [7, 5]);
        assert_eq!(mem.b32s1(w(100)), 100);
        assert_eq!([mem.b32s0(w(101)), mem.b32s1(w(101))], [150, 100]);
        assert_eq!(mem.b32s1(w(132)), 150);
        assert_eq!(mem[q].child_index(1), Some(q));
        assert_eq!(mem[q].child_element(63), Some(leaf));
        mem[q].clear_child(63);
        assert_eq!(mem.b32s1(w(132)), MemPtr::NULL.raw());
        assert_eq!(mem[q].child_element(63), None);

        let c: NodePtr<SaWordNode> = at(&mut mem, 150, None);
        let b = &mut mem[c];
        b.sa_lev = 3;
        b.sa_index = 64 + 9;
        b.link = q;
        b.sa_ref = -4;
        b.sa_num = 300;
        b.sa_int = 0x51;
        assert_eq!([mem.b16s0(w(150)), mem.b16s1(w(150))], [3, 64 + 9]);
        assert_eq!(mem.b32s1(w(150)), 100);
        assert_eq!([mem.b32s0(w(151)), mem.b32s1(w(151))], [-4, 300]);
        assert_eq!(mem.b32s1(w(152)), 0x51);
        assert_eq!(mem[leaf].sa_type(), ValueLevel::Dimen);
        assert_eq!(mem[leaf].slot(), 9);
        assert_eq!(leaf.leaf(&mem), Some(SaLeaf::Word(c)));

        let p: NodePtr<SaPointerNode> = at(&mut mem, 150, None);
        mem[p].sa_index = 64 * 5;
        mem[p].sa_ptr = w(77);
        assert_eq!(mem.b32s1(w(151)), 77);
        assert_eq!(mem[p].toks(), TokenList::from_ptr(w(77)));
        assert_eq!(leaf.leaf(&mem), Some(SaLeaf::Pointer(p)));

        let m: NodePtr<SaMarkClassNode> = at(&mut mem, 150, None);
        mem[m].sa_index = 64 * 7;
        for (k, code) in MarkCode::ALL.into_iter().enumerate() {
            let k = i32::try_from(k).unwrap();
            mem[m].set_mark(code, TokenList::from_ptr(w(10 + k)));
        }
        assert_eq!([mem.b32s0(w(151)), mem.b32s1(w(151))], [10, 11]);
        assert_eq!([mem.b32s0(w(152)), mem.b32s1(w(152))], [12, 13]);
        assert_eq!(mem.b32s0(w(153)), 14);
        assert_eq!(leaf.leaf(&mem), Some(SaLeaf::MarkClass(m)));

        let s: NodePtr<SaSavedWord> = at(&mut mem, 160, None);
        mem[s].sa_index = 64 + 9;
        mem[s].sa_loc = c;
        mem[s].sa_int = -9;
        assert_eq!(mem.b32s0(w(161)), 150);
        assert_eq!(mem.b32s1(w(162)), -9);
        assert!(matches!(s.saved().entry(&mem), SaSavedEntry::Word(e) if e == s));
        let s: NodePtr<SaSavedPointer> = at(&mut mem, 160, None);
        mem[s].sa_index = 64 * 2;
        mem[s].sa_ptr = w(88);
        assert_eq!([mem.b32s0(w(161)), mem.b32s1(w(161))], [150, 88]);
        assert!(matches!(s.saved().entry(&mem), SaSavedEntry::Pointer(e) if e == s));
    }

    #[test]
    fn glue_node_and_spec_fields_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let g: NodePtr<GlueNode> = at(&mut mem, 100, Some(NodeType::Glue));
        mem[g].subtype = GlueKind::Param(GluePar::LeftSkip).into();
        mem[g].glue_ptr = NodePtr::from_glue_ptr(MemPtr::from_raw(150));
        mem[g].leader_ptr = MemPtr::from_raw(160);
        assert_eq!(mem.b16s0(MemPtr::from_raw(100)), 8);
        assert_eq!(mem.b32s0(MemPtr::from_raw(101)), 150);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 160);
        assert_eq!(mem[g].glue_ptr.idx(), MemIdx::from_raw(150));

        let s: NodePtr<GlueSpecNode> = at(&mut mem, 150, None);
        mem[s].ref_count = 42;
        mem[s].stretch_order = GlueOrder::Fill.into();
        mem[s].shrink_order = GlueOrder::Fil.into();
        mem[s].width = Scaled::from_raw(0x4444);
        mem[s].stretch = Scaled::from_raw(0x5555);
        mem[s].shrink = Scaled::from_raw(0x6666);
        assert_eq!(mem.b32s1(MemPtr::from_raw(150)), 42);
        assert_eq!(mem.b16s1(MemPtr::from_raw(150)), 2);
        assert_eq!(mem.b16s0(MemPtr::from_raw(150)), 1);
        assert_eq!(mem.b32s1(MemPtr::from_raw(151)), 0x4444);
        assert_eq!(mem.b32s1(MemPtr::from_raw(152)), 0x5555);
        assert_eq!(mem.b32s1(MemPtr::from_raw(153)), 0x6666);
    }

    #[test]
    fn noad_fields_land_in_their_words() {
        let w = |p: i32| MemPtr::from_raw(p);
        let mut mem = fresh_mem(200);
        let n: NodePtr<Noad> = at(&mut mem, 100, Some(NodeType::OrdNoad));
        let b = &mut mem[n];
        b.set_limits(LimitCode::NoLimits);
        b.nucleus.math_type = 1;
        b.supscr.math_type = 2;
        b.subscr.math_type = 3;
        assert_eq!(mem.b16s0(w(100)), 2);
        assert_eq!(mem.b16s1(w(100)), NodeType::OrdNoad.as_raw());
        assert_eq!(
            [mem.b32s1(w(101)), mem.b32s1(w(102)), mem.b32s1(w(103))],
            [1, 2, 3]
        );
        assert_eq!(mem[n.supscr()].math_type, 2);
        mem[n.any()].new_hlist = w(77);
        assert_eq!(mem.b32s1(w(101)), 77);
        assert_eq!(mem[n.any()].link, mem[n].link);

        let a: NodePtr<AccentNoad> = at(&mut mem, 100, Some(NodeType::AccentNoad));
        mem[a].set_kind(MathAccentKind::TopFixed);
        mem[a].accent_chr.set_character(48);
        mem[a].accent_chr.set_plane_and_fam(5);
        assert_eq!(mem.b16s0(w(100)), 1);
        assert_eq!([mem.b16s0(w(104)), mem.b16s1(w(104))], [48, 5]);
        assert_eq!(mem[a.accent_chr()].character(), 48);
        assert_eq!(a.as_noad().nucleus().idx(), n.nucleus().idx());

        let r: NodePtr<RadicalNoad> = at(&mut mem, 100, Some(NodeType::RadicalNoad));
        mem[r].left_delimiter.set_small(3, 4);
        assert_eq!([mem.b16s2(w(104)), mem.b16s3(w(104))], [4, 3]);
        assert_eq!(mem[r.left_delimiter()].small(), (3, 4));

        let f: NodePtr<FractionNoad> = at(&mut mem, 100, Some(NodeType::FractionNoad));
        let b = &mut mem[f];
        b.thickness = Scaled::from_raw(0x51);
        b.numerator.math_type = 6;
        b.denominator.math_type = 7;
        b.left_delimiter.set_large(8, 9);
        b.right_delimiter.set_small(1 + 256, 7);
        b.right_delimiter.set_large(2, 9);
        assert_eq!(mem.b32s1(w(101)), 0x51);
        assert_eq!([mem.b32s1(w(102)), mem.b32s1(w(103))], [6, 7]);
        assert_eq!([mem.b16s0(w(104)), mem.b16s1(w(104))], [9, 8]);
        assert_eq!(mem.b16s3(w(105)), 1 + 256);
        assert_eq!(mem.b16s0(w(105)), 9);
        assert_eq!(mem[f.right_delimiter()].small(), (1, 7 + 65_536));
        assert_eq!(mem[f.numerator()].math_type, 6);
        assert_eq!(mem[f.denominator()].math_type, 7);

        let e: NodePtr<FenceNoad> = at(&mut mem, 100, Some(NodeType::RightNoad));
        mem[e].delimiter.set_large(2, 9);
        mem[e].subscr.math_type = 5;
        assert_eq!([mem.b16s0(w(101)), mem.b16s1(w(101))], [9, 2]);
        assert_eq!(mem.b32s1(w(103)), 5);
        assert_eq!(mem[e.delimiter()].large(), (2, 9));
    }

    #[test]
    fn math_and_delimiter_words_land_in_their_halves() {
        let mut mem = fresh_mem(200);
        let w = MemPtr::from_raw(100);
        let f: NodePtr<MathField> = NodePtr::at(w);
        mem[f].math_type = 3;
        mem[f].set_character(0x1234);
        mem[f].set_plane_and_fam(0x0205);
        assert_eq!(mem.b32s1(w), 3);
        assert_eq!(mem.b16s0(w), 0x1234);
        assert_eq!(mem.b16s1(w), 0x0205);
        assert_eq!(mem[f].fam(), 5);
        assert_eq!(mem[f].full_char(), 0x2_1234);
        mem[f].set_info(MemPtr::from_raw(0x0012_3456));
        assert_eq!(mem.b32s0(w), 0x0012_3456);
        mem[f].set_info(MemPtr::NULL);
        assert_eq!(mem.b32s0(w), MemPtr::NULL.raw());
        assert_eq!(mem[f].info(), MemPtr::NULL);
        assert_eq!(mem.b32s1(w), 3);

        let d: NodePtr<DelimiterField> = NodePtr::at(w);
        mem[d].set_small(0x0103, 0x41);
        mem[d].set_large(0x0204, 0x42);
        assert_eq!(
            [mem.b16s0(w), mem.b16s1(w), mem.b16s2(w), mem.b16s3(w)],
            [0x42, 0x0204, 0x41, 0x0103]
        );
        assert_eq!(mem[d].small(), (3, 0x1_0041));
        assert_eq!(mem[d].large(), (4, 0x2_0042));
        assert!(!mem[d].is_null());
        mem[d].clear();
        assert!(mem[d].is_null());
    }

    #[test]
    fn line_breaker_records_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let a: NodePtr<ActiveNode> = at(&mut mem, 100, None);
        let n = &mut mem[a];
        n.break_type = 1;
        n.fitness = 2;
        n.link = MemPtr::from_raw(160);
        n.break_node = MemPtr::from_raw(150);
        n.line_number = 7;
        n.total_demerits = 0x5151;
        n.shortfall = Scaled::from_raw(-0x20);
        n.glue = Scaled::from_raw(0x30);
        assert_eq!(MemPtr::from_raw(100).link(&mem), MemPtr::from_raw(160));
        assert_eq!(mem.b16s1(MemPtr::from_raw(100)), 1);
        assert_eq!(mem.b16s0(MemPtr::from_raw(100)), 2);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 150);
        assert_eq!(mem.b32s0(MemPtr::from_raw(101)), 7);
        assert_eq!(mem.b32s1(MemPtr::from_raw(102)), 0x5151);
        assert_eq!(mem.b32s1(MemPtr::from_raw(103)), -0x20);
        assert_eq!(mem.b32s1(MemPtr::from_raw(104)), 0x30);

        let d: NodePtr<DeltaNode> = at(&mut mem, 100, None);
        mem[d].dwidth = Scaled::from_raw(0x11);
        mem[d].dstretch3 = Scaled::from_raw(0x55);
        mem[d].dshrink = Scaled::from_raw(-0x66);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 0x11);
        assert_eq!(mem.b32s1(MemPtr::from_raw(105)), 0x55);
        assert_eq!(mem.b32s1(MemPtr::from_raw(106)), -0x66);

        let pn: NodePtr<PassiveNode> = at(&mut mem, 100, None);
        mem[pn].link = MemPtr::from_raw(90);
        mem[pn].cur_break = MemPtr::from_raw(150);
        mem[pn].next_break = MemPtr::from_raw(80);
        assert_eq!(mem[pn].next_break, MemPtr::from_raw(80));
        assert_eq!(mem.b32s1(MemPtr::from_raw(100)), 90);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 150);
        mem[pn].next_break = MemPtr::from_raw(70);
        assert_eq!(mem.b32s0(MemPtr::from_raw(101)), 70);
    }
}
