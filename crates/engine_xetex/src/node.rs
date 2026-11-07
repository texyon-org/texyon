//! Typed node handles over the flat `mem` arena.
//!
//! [`NodePtr<K>`] is a `Copy` index plus a compile-time kind marker, so a field
//! access compiles to the same load/store as the raw `mem[p + k]` form. Kinds
//! with a layout in [`crate::overlay`] index the arena (`mem[p].width`), the
//! others pass it as context: `p.width(mem)`, `p.set_width(mem, w)`.

use std::marker::PhantomData;

use crate::consts::{
    ACCENT_NOAD_SIZE, BOX_NODE_SIZE, EMPTY, FRACTION_NOAD_SIZE, GLUE_SPEC_SIZE, INS_NODE_SIZE,
    MARGIN_KERN_NODE_SIZE, MEDIUM_NODE_SIZE, NATIVE_NODE_SIZE, NOAD_SIZE, NORMAL, NULL_FLAG,
    PAGE_INS_NODE_SIZE, PIC_NODE_SIZE, RADICAL_NOAD_SIZE, RULE_NODE_SIZE, SMALL_NODE_SIZE,
    STYLE_NODE_SIZE,
};
use crate::errors::Abort;
use crate::state::MemState;
use crate::token::TokenList;
use crate::units::{
    FontId, GlueOrder, GlueSign, InvalidNodeType, MemAddr, MemIdx, MemPtr, NodeType, Orientation,
    Scaled, StrNumber,
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

/// A value stored in one subfield of a memory word.
trait Slot: Copy {
    type Raw;
    fn decode(raw: Self::Raw) -> Self;
    fn encode(self) -> Self::Raw;
}

macro_rules! identity_slot {
    ($($t:ty),*) => {$(
        impl Slot for $t {
            type Raw = $t;
            #[inline(always)]
            fn decode(raw: $t) -> $t {
                raw
            }
            #[inline(always)]
            fn encode(self) -> $t {
                self
            }
        }
    )*};
}
identity_slot!(i32, u16);

impl Slot for Scaled {
    type Raw = i32;
    #[inline(always)]
    fn decode(raw: i32) -> Self {
        Scaled::from_raw(raw)
    }
    #[inline(always)]
    fn encode(self) -> i32 {
        self.raw()
    }
}

impl Slot for TokenList {
    type Raw = i32;
    #[inline(always)]
    fn decode(raw: i32) -> Self {
        TokenList::from_ptr(MemPtr::from_raw(raw))
    }
    #[inline(always)]
    fn encode(self) -> i32 {
        self.ptr().raw()
    }
}

impl Slot for MemPtr {
    type Raw = i32;
    #[inline(always)]
    fn decode(raw: i32) -> Self {
        MemPtr::from_raw(raw)
    }
    #[inline(always)]
    fn encode(self) -> i32 {
        self.raw()
    }
}

impl Slot for StrNumber {
    type Raw = i32;
    #[inline(always)]
    fn decode(raw: i32) -> Self {
        StrNumber::from_raw(raw)
    }
    #[inline(always)]
    fn encode(self) -> i32 {
        self.raw()
    }
}

impl Slot for FontId {
    type Raw = u16;
    #[inline(always)]
    fn decode(raw: u16) -> Self {
        FontId::new(raw)
    }
    #[inline(always)]
    fn encode(self) -> u16 {
        self.get()
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

impl<T: StoredCode> Slot for Stored<T> {
    type Raw = u16;
    #[inline(always)]
    fn decode(raw: u16) -> Self {
        Stored::from_raw(raw)
    }
    #[inline(always)]
    fn encode(self) -> u16 {
        self.0
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

macro_rules! write_slot {
    ($mem:ident, b32s0, $p:expr, $v:expr) => {
        $mem.set_b32s0($p, $v)
    };
    ($mem:ident, b32s1, $p:expr, $v:expr) => {
        $mem.set_b32s1($p, $v)
    };
    ($mem:ident, b16s0, $p:expr, $v:expr) => {
        $mem.set_b16s0($p, $v)
    };
    ($mem:ident, b16s1, $p:expr, $v:expr) => {
        $mem.set_b16s1($p, $v)
    };
    ($mem:ident, b16s2, $p:expr, $v:expr) => {
        $mem.set_b16s2($p, $v)
    };
    ($mem:ident, b16s3, $p:expr, $v:expr) => {
        $mem.set_b16s3($p, $v)
    };
}

/// Field accessors: `getter / setter: Type = subfield @ word_offset;`.
macro_rules! fields {
    ($($(#[$m:meta])* $get:ident $(/ $set:ident)?: $ty:ty = $slot:ident @ $off:expr;)*) => {$(
        $(#[$m])*
        #[inline]
        pub fn $get(self, mem: &MemState) -> $ty {
            <$ty as Slot>::decode(mem.$slot(self.idx + $off))
        }
        $(
            #[inline]
            pub fn $set(self, mem: &mut MemState, v: $ty) {
                write_slot!(mem, $slot, self.idx + $off, <$ty as Slot>::encode(v))
            }
        )?
    )*};
}

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
    /// A `\vadjust` list, the subtype selects pre or post placement.
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
    /// Any noad. `alloc` makes an ordinary noad of `NOAD_SIZE`.
    Noad = OrdNoad, NOAD_SIZE;
    MarginKernNode = MarginKern, MARGIN_KERN_NODE_SIZE;
    /// Variable-size, sub-classified by [`WhatsitSubtype`].
    WhatsitNode;
    /// An alignment box before its column widths are known. Words +4..+6 hold
    /// the accumulated shrink, span count and stretch instead of box glue.
    UnsetNode;
    /// A glue specification, reached only through a glue pointer. Its header
    /// word holds the reference count and fill orders, not a type code.
    GlueSpecNode;
    /// An entry of the ligature reconstitution stack.
    LigItem;
    /// An eTeX sparse-array record: an index node or a register value.
    SaNode;
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
/// A kind still read through accessors rather than a layout.
pub trait Accessed: HasLink {}
/// A list-node header: type in `b16.s1`, subtype in `b16.s0`, link in `b32.s1`.
pub trait HasNodeHeader: Accessed {}
/// A `Scaled` width at +1.
pub trait HasWidth: NodeKind {}
/// `Scaled` depth and height at +2 and +3.
pub trait HasHeightDepth: NodeKind {}

macro_rules! impl_traits {
    ($trait:ident: $($kind:ident),*) => {$(impl $trait for $kind {})*};
}

impl_traits!(HasLink: BoxNode, RuleNode, InsNode, MarkNode, AdjustNode, LigatureNode, DiscNode,
    WhatsitNode, MathNode, GlueNode, KernNode, PenaltyNode, UnsetNode, StyleNode, ChoiceNode,
    EdgeNode, Noad, MarginKernNode, SaNode, PageInsNode, ActiveNode, DeltaNode, PassiveNode,
    MovementNode, CondNode, CharNode, AlignRecord, AlignStackNode, ExprNode, PseudoLineNode,
    LigItem);
impl_traits!(Accessed: LigatureNode, WhatsitNode, Noad, SaNode, ActiveNode, CharNode,
    PseudoLineNode);
impl_traits!(HasNodeHeader: LigatureNode, WhatsitNode, Noad, SaNode);
impl_traits!(HasWidth: WhatsitNode);
impl_traits!(HasHeightDepth: WhatsitNode);

/// A record kept only on its own list, which therefore proves its kind.
pub trait ListRecord: NodeKind {}

impl_traits!(ListRecord: SaNode, PageInsNode, MovementNode, ActiveNode, DeltaNode, PassiveNode,
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

impl<K: Accessed> NodePtr<K> {
    fields! {
        link / set_link: MemPtr = b32s1 @ 0;
    }
}

impl<K: HasNodeHeader> NodePtr<K> {
    fields! {
        subtype / set_subtype: u16 = b16s0 @ 0;
    }
}

impl<K: HasWidth> NodePtr<K> {
    fields! {
        width / set_width: Scaled = b32s1 @ 1;
    }
}

impl<K: HasHeightDepth> NodePtr<K> {
    fields! {
        depth / set_depth: Scaled = b32s1 @ 2;
        height / set_height: Scaled = b32s1 @ 3;
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
    /// Any noad, refine with [`NodePtr::<Noad>::noad_type`].
    Noad(NodePtr<Noad>),
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
            AnyNode::Whatsit(p) => p.width(mem),
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
    pub fn as_native_word(&self, p: MemPtr) -> Option<NodePtr<WhatsitNode>> {
        match self.classify_non_char(p)? {
            AnyNode::Whatsit(w) if w.whatsit_subtype(self).is_some_and(|s| s.is_native_word()) => {
                Some(w)
            }
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
    pub fn classify_noad(&self, p: impl MemAddr) -> Result<Option<NodePtr<Noad>>, InvalidNodeType> {
        Ok(match self.classify(p)? {
            AnyNode::Noad(n) => Some(n),
            _ => None,
        })
    }
}

// --- character-bearing nodes ---

impl NodePtr<CharNode> {
    fields! {
        font / set_font: FontId = b16s1 @ 0;
        character / set_character: u16 = b16s0 @ 0;
    }
}

impl MemState {
    /// `p` as a character node, if it lies in the one-word region.
    #[inline]
    pub fn as_char(&self, p: MemPtr) -> Option<NodePtr<CharNode>> {
        (p >= self.hi_mem_min).then(|| NodePtr::at(p))
    }
}

impl NodePtr<LigatureNode> {
    fields! {
        font / set_font: FontId = b16s1 @ 1;
        character / set_character: u16 = b16s0 @ 1;
        /// The original characters the ligature was formed from.
        lig_ptr / set_lig_ptr: MemPtr = b32s1 @ 1;
    }

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
    pub fn into_style(self, mem: &mut MemState, style: u16) -> NodePtr<StyleNode> {
        let p: NodePtr<StyleNode> = retype(self, mem, NodeType::Style);
        let layout = &mut mem[p];
        layout.subtype = style;
        layout.clear_payload();
        p
    }
}

impl NodePtr<Noad> {
    #[inline]
    pub fn alloc_radical(mem: &mut MemState) -> Result<NodePtr<Noad>, Abort> {
        mem.alloc_node(RADICAL_NOAD_SIZE, Some(NodeType::RadicalNoad))
    }

    #[inline]
    pub fn alloc_accent(mem: &mut MemState) -> Result<NodePtr<Noad>, Abort> {
        mem.alloc_node(ACCENT_NOAD_SIZE, Some(NodeType::AccentNoad))
    }

    #[inline]
    pub fn alloc_fraction(mem: &mut MemState) -> Result<NodePtr<Noad>, Abort> {
        mem.alloc_node(FRACTION_NOAD_SIZE, Some(NodeType::FractionNoad))
    }

    #[inline]
    pub fn noad_type(self, mem: &MemState) -> NodeType {
        NodeType::from_raw(mem.b16s1(self.idx)).expect("valid noad type")
    }
    #[inline]
    pub fn set_noad_type(self, mem: &mut MemState, nt: NodeType) {
        mem.set_b16s1(self.idx, nt.as_raw());
    }
    /// Stamps a type code taken from `cur_chr`, which a caller may fix up after.
    #[inline]
    pub fn set_type_code(self, mem: &mut MemState, code: u16) {
        mem.set_b16s1(self.idx, code);
    }

    fields! {
        /// The translated hlist, which reuses the nucleus type word once
        /// `mlist_to_hlist` has converted the nucleus.
        new_hlist / set_new_hlist: MemPtr = b32s1 @ 1;
        /// A fraction's rule thickness, in the nucleus type word.
        thickness / set_thickness: Scaled = b32s1 @ 1;
    }

    /// The empty fields and normal subtype of a fresh noad.
    #[inline]
    pub fn clear_fields(self, mem: &mut MemState) {
        self.set_subtype(mem, NORMAL);
        for field in [self.nucleus(), self.supscr(), self.subscr()] {
            field.clear(mem);
        }
    }

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
    /// A fraction's numerator, which shares the superscript word.
    #[inline]
    pub fn numerator(self) -> NodePtr<MathField> {
        self.supscr()
    }
    /// A fraction's denominator, which shares the subscript word.
    #[inline]
    pub fn denominator(self) -> NodePtr<MathField> {
        self.subscr()
    }
    #[inline]
    pub fn accent_chr(self) -> NodePtr<MathField> {
        NodePtr::at(self.idx + 4)
    }
    /// A left or right noad's delimiter, in the nucleus word.
    #[inline]
    pub fn delimiter(self) -> NodePtr<DelimiterField> {
        NodePtr::at(self.idx + 1)
    }
    /// A radical's delimiter, or a fraction's left one.
    #[inline]
    pub fn left_delimiter(self) -> NodePtr<DelimiterField> {
        NodePtr::at(self.idx + 4)
    }
    #[inline]
    pub fn right_delimiter(self) -> NodePtr<DelimiterField> {
        NodePtr::at(self.idx + 5)
    }
}

impl NodePtr<MathField> {
    fields! {
        math_type / set_math_type: i32 = b32s1 @ 0;
        /// The list of a `SUB_BOX` or `SUB_MLIST` field.
        info / set_info: MemPtr = b32s0 @ 0;
        character / set_character: u16 = b16s0 @ 0;
        /// The family in the low byte and the character's plane above it.
        plane_and_fam / set_plane_and_fam: u16 = b16s1 @ 0;
    }

    #[inline]
    pub fn fam(self, mem: &MemState) -> i32 {
        i32::from(self.plane_and_fam(mem)) % 256
    }
    /// The character code with its plane.
    #[inline]
    pub fn full_char(self, mem: &MemState) -> i32 {
        i32::from(self.character(mem)) + (i32::from(self.plane_and_fam(mem)) / 256) * 65_536
    }
    #[inline]
    pub fn clear(self, mem: &mut MemState) {
        self.set_math_type(mem, EMPTY);
        self.set_info(mem, MemPtr::NULL);
    }
    #[inline]
    pub fn copy_from(self, mem: &mut MemState, src: NodePtr<MathField>) {
        mem.set_word(self.idx, mem.word(src.idx));
    }
}

/// A delimiter's small and large variants, each a family and a character.
impl NodePtr<DelimiterField> {
    #[inline]
    fn variant(plane_and_fam: u16, char: u16) -> (i32, i32) {
        let plane_and_fam = i32::from(plane_and_fam);
        (
            plane_and_fam % 256,
            i32::from(char) + (plane_and_fam / 256) * 65_536,
        )
    }
    /// The small variant's `(family, character)`.
    #[inline]
    pub fn small(self, mem: &MemState) -> (i32, i32) {
        Self::variant(mem.b16s3(self.idx), mem.b16s2(self.idx))
    }
    /// The large variant's `(family, character)`.
    #[inline]
    pub fn large(self, mem: &MemState) -> (i32, i32) {
        Self::variant(mem.b16s1(self.idx), mem.b16s0(self.idx))
    }
    #[inline]
    pub fn set_small(self, mem: &mut MemState, plane_and_fam: u16, char: u16) {
        mem.set_b16s3(self.idx, plane_and_fam);
        mem.set_b16s2(self.idx, char);
    }
    #[inline]
    pub fn set_large(self, mem: &mut MemState, plane_and_fam: u16, char: u16) {
        mem.set_b16s1(self.idx, plane_and_fam);
        mem.set_b16s0(self.idx, char);
    }
    #[inline]
    pub fn clear(self, mem: &mut MemState) {
        self.set_small(mem, 0, 0);
        self.set_large(mem, 0, 0);
    }
    #[inline]
    pub fn is_null(self, mem: &MemState) -> bool {
        self.small(mem) == (0, 0) && self.large(mem) == (0, 0)
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
        mem[u].subtype = 0;
        mem[u].height = Scaled::ZERO;
        mem[u].depth = Scaled::ZERO;
        mem[u].glue_order = (GlueOrder::Normal).into();
        mem[u].glue_sign = (GlueOrder::Normal).into();
        mem[u].glue_stretch = Scaled::ZERO;
        mem[u].glue_shrink = Scaled::ZERO;
        u
    }
}

impl NodePtr<ParShapeNode> {
    fields! {
        lines / set_lines: i32 = b32s0 @ 0;
    }

    #[inline]
    pub fn indent(self, mem: &MemState, line: i32) -> Scaled {
        Scaled::from_raw(mem.b32s1(self.idx + 2 * line - 1))
    }
    #[inline]
    pub fn set_indent(self, mem: &mut MemState, line: i32, v: Scaled) {
        mem.set_b32s1(self.idx + 2 * line - 1, v.raw());
    }
    #[inline]
    pub fn line_width(self, mem: &MemState, line: i32) -> Scaled {
        Scaled::from_raw(mem.b32s1(self.idx + 2 * line))
    }
    #[inline]
    pub fn set_line_width(self, mem: &mut MemState, line: i32, v: Scaled) {
        mem.set_b32s1(self.idx + 2 * line, v.raw());
    }
}

impl NodePtr<PenaltiesNode> {
    fields! {
        count / set_count: i32 = b32s1 @ 1;
    }

    /// Stores half the node size, as a `\parshape` array does.
    #[inline]
    pub fn set_words(self, mem: &mut MemState, words: i32) {
        mem.set_b32s0(self.idx, words);
    }

    /// Entry `k`, counting from 1.
    #[inline]
    pub fn penalty(self, mem: &MemState, k: i32) -> i32 {
        mem.b32s1(self.idx + 1 + k)
    }
    #[inline]
    pub fn set_penalty(self, mem: &mut MemState, k: i32, v: i32) {
        mem.set_b32s1(self.idx + 1 + k, v);
    }
}

impl NodePtr<PseudoLineNode> {
    fields! {
        size / set_size: i32 = b32s0 @ 0;
    }

    /// The four characters in word `k`, counting from 1.
    #[inline]
    pub fn chars(self, mem: &MemState, k: i32) -> [u16; 4] {
        let w = self.idx.raw() + k;
        [
            mem.b16s3(MemPtr::from_raw(w)),
            mem.b16s2(MemPtr::from_raw(w)),
            mem.b16s1(MemPtr::from_raw(w)),
            mem.b16s0(MemPtr::from_raw(w)),
        ]
    }
    #[inline]
    pub fn set_chars(self, mem: &mut MemState, k: i32, chars: [u16; 4]) {
        let w = self.idx.raw() + k;
        mem.set_b16s3(MemPtr::from_raw(w), chars[0]);
        mem.set_b16s2(MemPtr::from_raw(w), chars[1]);
        mem.set_b16s1(MemPtr::from_raw(w), chars[2]);
        mem.set_b16s0(MemPtr::from_raw(w), chars[3]);
    }
}

impl NodePtr<FreeBlock> {
    fields! {
        node_size / set_node_size: i32 = b32s0 @ 0;
        llink / set_llink: MemPtr = b32s0 @ 1;
        rlink / set_rlink: MemPtr = b32s1 @ 1;
    }

    #[inline]
    pub fn is_empty(self, mem: &MemState) -> bool {
        mem.b32s1(self.idx) == crate::consts::MAX_HALFWORD
    }
    #[inline]
    pub fn mark_empty(self, mem: &mut MemState) {
        mem.set_b32s1(self.idx, crate::consts::MAX_HALFWORD);
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

impl NodePtr<WhatsitNode> {
    /// A fresh `size`-word whatsit of the given subtype, fields uninitialised.
    #[inline]
    pub fn alloc(mem: &mut MemState, size: i32, subtype: WhatsitSubtype) -> Result<Self, Abort> {
        let p: Self = mem.alloc_node(size, Some(NodeType::Whatsit))?;
        p.set_subtype(mem, subtype as u16);
        Ok(p)
    }

    #[inline]
    pub fn whatsit_subtype(self, mem: &MemState) -> Option<WhatsitSubtype> {
        WhatsitSubtype::from_raw(mem.b16s0(self.idx))
    }

    fields! {
        write_tokens / set_write_tokens: TokenList = b32s1 @ 1;
        write_stream / set_write_stream: i32 = b32s0 @ 1;
        open_name / set_open_name: StrNumber = b32s1 @ 1;
        open_area / set_open_area: StrNumber = b32s0 @ 2;
        open_ext / set_open_ext: StrNumber = b32s1 @ 2;
        what_lang / set_what_lang: i32 = b32s1 @ 1;
        what_lhm / set_what_lhm: u16 = b16s1 @ 1;
        what_rhm / set_what_rhm: u16 = b16s0 @ 1;
        native_font / set_native_font: FontId = b16s2 @ 4;
        /// UTF-16 length of a native word.
        native_length / set_native_length: u16 = b16s1 @ 4;
        /// Glyph id of a glyph node, in the native length slot.
        native_glyph / set_native_glyph: u16 = b16s1 @ 4;
        native_glyph_count / set_native_glyph_count: u16 = b16s0 @ 4;
        /// One-based shaped-glyph arena handle, zero when unshaped.
        native_glyph_info_handle: i32 = b32s0 @ 5;
        /// Path byte length of a picture, in the native length slot.
        pic_path_len / set_pic_path_len: u16 = b16s1 @ 4;
        pic_page / set_pic_page: u16 = b16s0 @ 4;
        pic_pagebox / set_pic_pagebox: u16 = b16s1 @ 8;
    }

    /// Node size in words, used to free or copy a native node.
    #[inline]
    pub fn native_size(self, mem: &MemState) -> i32 {
        i32::from(mem.b16s3(self.idx + 4))
    }
    #[inline]
    pub fn set_native_size(self, mem: &mut MemState, words: i32) {
        mem.set_b16s3(self.idx + 4, words as u16);
    }

    #[inline]
    pub fn set_native_glyph_info_handle(self, mem: &mut MemState, handle: i32) {
        mem.set_b32s0(self.idx + 5, handle);
        mem.set_b32s1(self.idx + 5, 0);
    }

    /// The picture transform `[a, b, c, d, x, y]`.
    #[inline]
    pub fn pic_transform(self, mem: &MemState) -> [Scaled; 6] {
        std::array::from_fn(|k| {
            let q = self.idx.raw() + 5 + k as i32 / 2;
            Scaled::from_raw(if k % 2 == 0 {
                mem.b32s0(MemPtr::from_raw(q))
            } else {
                mem.b32s1(MemPtr::from_raw(q))
            })
        })
    }
    #[inline]
    pub fn set_pic_transform(self, mem: &mut MemState, t: [Scaled; 6]) {
        for (k, v) in (0..).zip(t) {
            let q = self.idx.raw() + 5 + k / 2;
            if k % 2 == 0 {
                mem.set_b32s0(MemPtr::from_raw(q), v.raw());
            } else {
                mem.set_b32s1(MemPtr::from_raw(q), v.raw());
            }
        }
    }

    #[inline]
    pub fn pic_path(self, mem: &MemState) -> &[u8] {
        let start = (self.idx.raw() + PIC_NODE_SIZE) as usize * 8;
        &mem.bytes()[start..start + self.pic_path_len(mem) as usize]
    }
    /// Stores `path` inline, the node must have room for it.
    #[inline]
    pub fn set_pic_path(self, mem: &mut MemState, path: &[u8]) {
        let start = self.idx.raw() + PIC_NODE_SIZE;
        mem.journal_mark_range(MemPtr::from_raw(start), path.len().div_ceil(8));
        let at = start as usize * 8;
        mem.bytes_mut()[at..at + path.len()].copy_from_slice(path);
    }

    /// The index in `mem.units()` where the text starts.
    #[inline]
    pub fn native_text_start(self) -> usize {
        (self.idx.raw() + NATIVE_NODE_SIZE) as usize * 4
    }

    #[inline]
    pub fn native_text(self, mem: &MemState) -> &[u16] {
        let start = self.native_text_start();
        &mem.units()[start..start + self.native_length(mem) as usize]
    }

    /// Journals the words of the text before it is written.
    #[inline]
    pub fn mark_native_text(self, mem: &mut MemState) {
        let words = crate::state::native_text_words(i32::from(self.native_length(mem)));
        mem.journal_mark_range(self.idx + NATIVE_NODE_SIZE, words);
    }

    /// The text, journaled for writing.
    #[inline]
    pub fn native_text_mut(self, mem: &mut MemState) -> &mut [u16] {
        self.mark_native_text(mem);
        let start = self.native_text_start();
        let len = self.native_length(mem) as usize;
        &mut mem.units_mut()[start..start + len]
    }

    /// Drops the shaped-glyph payload of a native word.
    pub fn release_native_glyphs(self, mem: &mut MemState) {
        let handle = self.native_glyph_info_handle(mem);
        if handle != 0 {
            mem.glyph_blobs.release(handle);
            self.set_native_glyph_info_handle(mem, 0);
            self.set_native_glyph_count(mem, 0);
        }
    }

    /// Gives this copy of `src` its own handle on `src`'s shaped glyphs.
    pub fn share_native_glyphs(self, mem: &mut MemState, src: Self) {
        let handle = src.native_glyph_info_handle(mem);
        if handle != 0 {
            let count = src.native_glyph_count(mem);
            let copy = mem.glyph_blobs.clone_handle(handle);
            self.set_native_glyph_info_handle(mem, copy);
            self.set_native_glyph_count(mem, count);
        }
    }

    /// Frees a native word together with its shaped glyphs.
    pub fn free_native(self, mem: &mut MemState) {
        self.release_native_glyphs(mem);
        mem.free_node(self.ptr(), self.native_size(mem));
    }
}

// --- eTeX sparse arrays ---

impl NodePtr<SaNode> {
    fields! {
        /// An index node's count of populated children.
        ref_count / set_ref_count: u16 = b16s0 @ 0;
        /// Type and index code.
        sa_index / set_sa_index: u16 = b16s1 @ 0;
        sa_lev / set_sa_lev: u16 = b16s0 @ 0;
        sa_ref / set_sa_ref: i32 = b32s0 @ 1;
        /// A box or token-list value, in child slot 1.
        sa_ptr / set_sa_ptr: MemPtr = b32s1 @ 1;
        /// A token-list register's value, in the same slot.
        sa_toks: TokenList = b32s1 @ 1;
        /// A count or dimension register's number, in the same slot.
        sa_num / set_sa_num: i32 = b32s1 @ 1;
        /// A count or dimension value.
        sa_int / set_sa_int: i32 = b32s1 @ 2;
    }

    /// The register type, from the high bits of the index code.
    #[inline]
    pub fn sa_type(self, mem: &MemState) -> Stored<crate::codes::ValueLevel> {
        Stored::from_raw(self.sa_index(mem) / 64)
    }

    /// Mark `m` of a mark class record.
    #[inline]
    pub fn mark(self, mem: &MemState, m: crate::codes::MarkCode) -> TokenList {
        TokenList::from_ptr(self.child(mem, m.raw()))
    }
    #[inline]
    pub fn set_mark(self, mem: &mut MemState, m: crate::codes::MarkCode, v: TokenList) {
        self.set_child(mem, m.raw(), v.ptr());
    }

    /// Child or mark slot `i`, packed two per word from +1.
    #[inline]
    pub fn child(self, mem: &MemState, i: i32) -> MemPtr {
        let w = self.idx.raw() + i / 2 + 1;
        MemPtr::from_raw(if i % 2 != 0 {
            mem.b32s1(MemPtr::from_raw(w))
        } else {
            mem.b32s0(MemPtr::from_raw(w))
        })
    }
    #[inline]
    pub fn set_child(self, mem: &mut MemState, i: i32, v: MemPtr) {
        let w = self.idx.raw() + i / 2 + 1;
        if i % 2 != 0 {
            mem.set_b32s1(MemPtr::from_raw(w), v.raw());
        } else {
            mem.set_b32s0(MemPtr::from_raw(w), v.raw());
        }
    }

    /// The parent index node, non-null below the root.
    #[inline]
    pub fn parent(self, mem: &MemState) -> NodePtr<SaNode> {
        Self::at(self.link(mem))
    }
}

// --- the line breaker's active, delta and passive lists ---

impl NodePtr<ActiveNode> {
    fields! {
        fitness / set_fitness: u16 = b16s0 @ 0;
        /// Unhyphenated or hyphenated, in the type word.
        break_type / set_break_type: u16 = b16s1 @ 0;
        break_node / set_break_node: MemPtr = b32s1 @ 1;
        line_number / set_line_number: i32 = b32s0 @ 1;
        total_demerits / set_total_demerits: i32 = b32s1 @ 2;
        /// Extended (`\lastlinefit`) active nodes only.
        shortfall / set_shortfall: Scaled = b32s1 @ 3;
        /// Extended (`\lastlinefit`) active nodes only.
        glue / set_glue: Scaled = b32s1 @ 4;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::MAX_HALFWORD;
    use std::mem::size_of;

    fn fresh_mem(n: usize) -> MemState {
        MemState {
            data: vec![Default::default(); n],
            hi_mem_min: MemPtr::from_raw(1_000_000),
            ..Default::default()
        }
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
        assert_eq!(n.noad_type(&mem), NodeType::OrdNoad);
        let n = NodePtr::<Noad>::alloc_radical(&mut mem).unwrap();
        assert_eq!(n.noad_type(&mem), NodeType::RadicalNoad);
        let n = NodePtr::<Noad>::alloc_accent(&mut mem).unwrap();
        assert_eq!(n.noad_type(&mem), NodeType::AccentNoad);
        let n = NodePtr::<Noad>::alloc_fraction(&mut mem).unwrap();
        assert!(matches!(mem.classify(n.idx()), Ok(AnyNode::Noad(_))));
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
        lig.set_font(&mut mem, FontId::new(77));
        lig.set_character(&mut mem, 0x1234);
        assert_eq!(lig.font(&mem), FontId::new(77));
        assert_eq!(mem.b16s1(MemPtr::from_raw(101)), 77);
        assert_eq!(mem.b16s0(MemPtr::from_raw(101)), 0x1234);
        lig.set_lig_ptr(&mut mem, MemPtr::from_raw(150));
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 150);

        let disc: NodePtr<DiscNode> = at(&mut mem, 100, Some(NodeType::Disc));
        mem[disc].pre_break = MemPtr::from_raw(150);
        mem[disc].post_break = MemPtr::from_raw(160);
        assert_eq!(mem.b32s0(MemPtr::from_raw(101)), 150);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 160);

        let math: NodePtr<MathNode> = at(&mut mem, 100, Some(NodeType::Math));
        mem[math].subtype = 3;
        mem[math].width = Scaled::from_raw(0x1234);
        assert_eq!(mem.b16s0(MemPtr::from_raw(100)), 3);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 0x1234);

        let style: NodePtr<StyleNode> = at(&mut mem, 100, Some(NodeType::Style));
        mem.set_b32s1(MemPtr::from_raw(101), 9);
        mem.set_b32s1(MemPtr::from_raw(102), 9);
        mem[style].clear_payload();
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 0);
        assert_eq!(mem.b32s1(MemPtr::from_raw(102)), 0);

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

        let w: NodePtr<WhatsitNode> = at(&mut mem, 100, Some(NodeType::Whatsit));
        w.set_subtype(&mut mem, 40);
        assert_eq!(w.whatsit_subtype(&mem), Some(WhatsitSubtype::NativeWord));
        mem.set_b16s3(MemPtr::from_raw(104), 6);
        mem.set_b16s2(MemPtr::from_raw(104), 7);
        mem.set_b16s1(MemPtr::from_raw(104), 9);
        assert_eq!(w.native_size(&mem), 6);
        assert_eq!(w.native_font(&mem), FontId::new(7));
        assert_eq!(w.native_length(&mem), 9);
        assert_eq!(w.pic_path_len(&mem), 9);
        w.set_native_glyph_count(&mut mem, 3);
        assert_eq!(mem.b16s0(MemPtr::from_raw(104)), 3);
        w.set_native_glyph_info_handle(&mut mem, 7);
        assert_eq!(w.native_glyph_info_handle(&mem), 7);
        assert_eq!(mem.b32s1(MemPtr::from_raw(105)), 0);

        mem.set_b32s1(MemPtr::from_raw(101), 0x55);
        assert_eq!(
            w.write_tokens(&mem),
            TokenList::from_ptr(MemPtr::from_raw(0x55))
        );
        assert_eq!(w.what_lang(&mem), 0x55);
        mem.set_b16s1(MemPtr::from_raw(101), 4);
        mem.set_b16s0(MemPtr::from_raw(101), 5);
        assert_eq!(w.what_lhm(&mem), 4);
        assert_eq!(w.what_rhm(&mem), 5);
    }

    #[test]
    fn sa_children_pack_two_per_word() {
        let mut mem = fresh_mem(200);
        let node: NodePtr<SaNode> = at(&mut mem, 100, None);
        for i in 0..6 {
            node.set_child(&mut mem, i, MemPtr::from_raw(1000 + i));
        }
        for i in 0..6 {
            assert_eq!(node.child(&mem, i), MemPtr::from_raw(1000 + i));
        }
        assert_eq!(mem.b32s0(MemPtr::from_raw(101)), 1000);
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 1001);
        assert_eq!(mem.b32s1(MemPtr::from_raw(103)), 1005);
        node.set_ref_count(&mut mem, 7);
        assert_eq!(mem.b16s0(MemPtr::from_raw(100)), 7);
    }

    #[test]
    fn glue_node_and_spec_fields_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let g: NodePtr<GlueNode> = at(&mut mem, 100, Some(NodeType::Glue));
        mem[g].subtype = 7;
        mem[g].glue_ptr = NodePtr::from_glue_ptr(MemPtr::from_raw(150));
        mem[g].leader_ptr = MemPtr::from_raw(160);
        assert_eq!(mem.b16s0(MemPtr::from_raw(100)), 7);
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
        let mut mem = fresh_mem(200);
        let n: NodePtr<Noad> = at(&mut mem, 100, Some(NodeType::OrdNoad));
        n.nucleus().set_math_type(&mut mem, 1);
        n.nucleus().set_info(&mut mem, MemPtr::from_raw(123));
        assert_eq!(mem.b32s1(MemPtr::from_raw(101)), 1);
        assert_eq!(mem.b32s0(MemPtr::from_raw(101)), 123);
        n.nucleus().set_character(&mut mem, 45);
        n.nucleus().set_plane_and_fam(&mut mem, 2 + 3 * 256);
        assert_eq!(n.nucleus().fam(&mem), 2);
        assert_eq!(n.nucleus().full_char(&mem), 45 + 3 * 65_536);
        n.accent_chr().set_character(&mut mem, 48);
        n.accent_chr().set_plane_and_fam(&mut mem, 5);
        assert_eq!(mem.b16s0(MemPtr::from_raw(104)), 48);
        assert_eq!(mem.b16s1(MemPtr::from_raw(104)), 5);
        n.right_delimiter().set_small(&mut mem, 1 + 256, 7);
        n.right_delimiter().set_large(&mut mem, 2, 9);
        assert_eq!(n.right_delimiter().small(&mem), (1, 7 + 65_536));
        assert_eq!(n.right_delimiter().large(&mem), (2, 9));
        assert_eq!(mem.b16s3(MemPtr::from_raw(105)), 1 + 256);
        assert_eq!(mem.b16s0(MemPtr::from_raw(105)), 9);
    }

    #[test]
    fn line_breaker_records_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let a: NodePtr<ActiveNode> = at(&mut mem, 100, None);
        a.set_break_type(&mut mem, 1);
        a.set_fitness(&mut mem, 2);
        a.set_break_node(&mut mem, MemPtr::from_raw(150));
        a.set_line_number(&mut mem, 7);
        a.set_total_demerits(&mut mem, 0x5151);
        a.set_shortfall(&mut mem, Scaled::from_raw(-0x20));
        a.set_glue(&mut mem, Scaled::from_raw(0x30));
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
