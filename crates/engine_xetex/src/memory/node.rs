//! Typed node handles over the flat `mem` arena.
//!
//! Every node kind is a `#[repr(C)]` struct in [`kinds`], laid over its words
//! in `mem`. A [`NodePtr<K>`] is a `Copy` index typed by its kind, so `mem[p]`
//! borrows the node as a `K` and a field access is one load or store.
//! `mem.entries(p)` gives the text or array an [`ArrayNode`] keeps after its struct.

use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Index, IndexMut};

use crate::base::codes::code_enum;
use crate::base::consts::NULL_FLAG;
use crate::base::params::GluePar;
use crate::base::units::{
    Dir, GlueOrder, InvalidNodeType, MemAddr, MemIdx, MemPtr, NodeType, OrderTotals, Orientation,
    Scaled,
};
use crate::build::math::MathStyle;
use crate::memory::mem::MemState;
use crate::print::errors::{Abort, OrConfusion};

mod kinds;

pub use kinds::*;

/// Marker for node kinds, implemented by the uninhabited kind enums below.
/// A struct laid over consecutive words of `mem`, read as `mem[p]` through a [`NodePtr`].
pub trait View: bytemuck::Pod {
    const SIZE: i32 = word_offset(mem::size_of::<Self>());
}

#[repr(transparent)]
pub struct NodePtr<K: View> {
    idx: MemIdx,
    _kind: PhantomData<fn() -> K>,
}

impl<K: View> Copy for NodePtr<K> {}
impl<K: View> Clone for NodePtr<K> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<K: View> PartialEq for NodePtr<K> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}
impl<K: View> Eq for NodePtr<K> {}
impl<K: View> Hash for NodePtr<K> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
    }
}
// SAFETY: transparent over a `MemIdx`, so `None` is the zero word.
unsafe impl<K: View + 'static> bytemuck::ZeroableInOption for NodePtr<K> {}
// SAFETY: as above, and every non-zero word is a valid `MemIdx`.
unsafe impl<K: View + 'static> bytemuck::PodInOption for NodePtr<K> {}
impl<K: View> MemAddr for NodePtr<K> {
    #[inline(always)]
    fn addr(self) -> MemIdx {
        self.idx
    }
}
impl<K: View> Debug for NodePtr<K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodePtr({})", self.idx.raw())
    }
}

impl<K: View> NodePtr<K> {
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

    #[inline]
    pub fn ptr(self) -> MemPtr {
        Some(self.idx)
    }
}

impl Index<Stored<GlueOrder>> for OrderTotals {
    type Output = Scaled;
    #[inline(always)]
    fn index(&self, o: Stored<GlueOrder>) -> &Scaled {
        &self.as_array()[o.index()]
    }
}

impl IndexMut<Stored<GlueOrder>> for OrderTotals {
    #[inline(always)]
    fn index_mut(&mut self, o: Stored<GlueOrder>) -> &mut Scaled {
        &mut self.as_array_mut()[o.index()]
    }
}

pub trait StoredCode: Copy {
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
impl<T> Debug for Stored<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

    #[inline]
    pub fn get(self) -> Option<T> {
        T::from_u16(self.0)
    }

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
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Stored<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}
impl<T: StoredCode> PartialOrd<T> for Stored<T> {
    #[inline]
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        Some(self.0.cmp(&other.to_u16()))
    }
}

code_enum! {
    pub enum KernSubtype: u16 {
        Normal = 0,
        Explicit = 1,
        AccKern = 2,
        /// XeTeX's adjustment after an interword space.
        SpaceAdjustment = 3,
        /// `\mkern`, in mu until `mlist_to_hlist` converts it.
        MuGlue = 99,
    }
}

code_enum! {
    pub enum AdjustPlacement: u16 {
        Post = 0,
        Pre = 1,
    }
}

code_enum! {
    /// A math accent's placement, set by `\Umathaccent fixed`, `bottom` or `bottom fixed`.
    pub enum MathAccentKind: u16 {
        Top = 0,
        TopFixed = 1,
        Bottom = 2,
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

code_enum! {
    /// A right noad's subtype, which tells `\middle` from `\right`.
    pub enum FenceSubtype: u16 {
        Normal = 0,
        Middle = 1,
    }
}

code_enum! {
    /// The implicit boundary characters a ligature absorbed, shown as `|` around it.
    pub enum LigBoundary: u16 {
        None = 0,
        Right = 1,
        Left = 2,
        Both = 3,
    }
}

impl LigBoundary {
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

code_enum! {
    /// A math node's code: math-on/off, the `\beginM`/`\endM` pair around a display, or a
    /// TeX--XeT boundary. The LR stack holds these codes too.
    pub enum MathCode: u16 {
        Before = 0,
        After = 1,
        /// Opens the segment around a display.
        BeginM = 2,
        /// Closes it.
        EndM = 3,
        BeginL = 6,
        EndL = 7,
        BeginR = 10,
        EndR = 11,
    }
}

impl MathCode {
    #[inline]
    pub fn is_end(self) -> bool {
        matches!(
            self,
            MathCode::After | MathCode::EndM | MathCode::EndL | MathCode::EndR
        )
    }

    #[inline]
    pub fn is_lr(self) -> bool {
        matches!(
            self,
            MathCode::BeginL | MathCode::EndL | MathCode::BeginR | MathCode::EndR
        )
    }

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

code_enum! {
    pub enum LeaderKind: u16 {
        Aligned = 100,
        Centered = 101,
        Expanded = 102,
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum GlueKind {
    Normal,
    /// From a glue parameter, so `\showbox` names it.
    Param(GluePar),
    /// Glue that `\nonscript` cancels in script styles.
    CondMath,
    /// `\mskip`, in mu until `mlist_to_hlist` converts it.
    Mu,
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

code_enum! {
    pub enum BoxLr: u16 {
        Normal = 0,
        Reversed = 1,
        /// A display whose lines run right to left.
        Dlist = 2,
    }
}

const _: () = assert!(
    Orientation::Horizontal.node_type().to_u16() == Orientation::Horizontal.to_u16()
        && Orientation::Vertical.node_type().to_u16() == Orientation::Vertical.to_u16()
);

pub trait Node: View {
    /// The `NODE_type` stamped at allocation, `None` for headerless records.
    const TYPE: Option<NodeType>;
    /// Whether entries follow the struct, so the node is sized by `ArrayNode`.
    const ARRAY: bool = false;

    #[inline]
    fn init(_mem: &mut MemState, _p: NodePtr<Self>) {}
}

impl MemState {
    #[inline]
    fn alloc_node<K: View>(
        &mut self,
        size: i32,
        ty: Option<NodeType>,
    ) -> Result<NodePtr<K>, Abort> {
        let p = self.get_node(size)?;
        if let Some(ty) = ty {
            self.set_header(p, ty, 0);
        }
        Ok(NodePtr::from_idx_unchecked(p))
    }
}

impl<K: Node> NodePtr<K> {
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Result<Self, Abort> {
        const { assert!(!K::ARRAY) };
        let p = mem.alloc_node(K::SIZE, K::TYPE)?;
        K::init(mem, p);
        Ok(p)
    }
}

/// A kind with a `link`, which generic list walks read through the node header.
pub trait HasLink: View {}

impl HasLink for BoxNode {}
impl HasLink for RuleNode {}
impl HasLink for InsNode {}
impl HasLink for MarkNode {}
impl HasLink for AdjustNode {}
impl HasLink for LigatureNode {}
impl HasLink for DiscNode {}
impl HasLink for WhatsitNode {}
impl HasLink for MathNode {}
impl HasLink for GlueNode {}
impl HasLink for KernNode {}
impl HasLink for PenaltyNode {}
impl HasLink for UnsetNode {}
impl HasLink for StyleNode {}
impl HasLink for ChoiceNode {}
impl HasLink for EdgeNode {}
impl HasLink for Noad {}
impl HasLink for RadicalNoad {}
impl HasLink for AccentNoad {}
impl HasLink for FractionNoad {}
impl HasLink for FenceNoad {}
impl HasLink for AnyNoad {}
impl HasLink for MarginKernNode {}
impl HasLink for PageInsNode {}
impl HasLink for CharNode {}
impl HasLink for HeadNode {}
impl HasLink for LrNode {}
impl HasLink for AlignRecord {}
impl HasLink for PseudoLineNode {}
impl HasLink for LigItem {}

/// A record kept only on its own list, which therefore proves its kind.
pub trait ListRecord: View {}

impl ListRecord for PageInsNode {}
impl ListRecord for LigItem {}
impl ListRecord for CharNode {}
impl ListRecord for HeadNode {}
impl ListRecord for LrNode {}
impl ListRecord for PseudoFileNode {}
impl ListRecord for HyphPosNode {}
impl ListRecord for AlignRecord {}
impl ListRecord for SpanNode {}
impl ListRecord for ParShapeNode {}
impl ListRecord for PenaltiesNode {}
impl ListRecord for PseudoLineNode {}

impl<K: ListRecord> NodePtr<K> {
    #[inline]
    pub fn at(p: impl MemAddr) -> Self {
        NodePtr::from_idx_unchecked(p.addr())
    }

    #[inline]
    pub fn from_ptr(p: MemPtr) -> Option<Self> {
        p.map(Self::at)
    }
}

#[inline]
pub fn link_of<K: View>(p: Option<NodePtr<K>>) -> MemPtr {
    p.map(NodePtr::idx)
}

impl<K: Node> NodePtr<K> {
    #[inline]
    pub fn free(self, mem: &mut MemState) {
        const { assert!(!K::ARRAY) };
        mem.free_node(self.idx(), K::SIZE);
    }
}

#[inline]
fn header(p: impl MemAddr) -> NodePtr<NodeHeader> {
    NodePtr::from_idx_unchecked(p.addr())
}

impl MemIdx {
    #[inline]
    pub fn link(self, mem: &MemState) -> MemPtr {
        mem[header(self)].link
    }
    #[inline]
    pub fn set_link(self, mem: &mut MemState, q: MemPtr) {
        mem[header(self)].link = q;
    }
}

impl NodePtr<BoxNode> {
    #[inline]
    pub fn into_unset(self, mem: &mut MemState) -> NodePtr<UnsetNode> {
        retype(self, mem, NodeType::Unset)
    }
}

/// A field holding a list pointer: a box's `list_ptr`, or a node's `link`.
/// Cursors that splice a box's list trail its nodes with one of these.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LinkSlot {
    List(NodePtr<BoxNode>),
    Node(MemIdx),
}

impl LinkSlot {
    #[inline]
    pub fn get(self, mem: &MemState) -> MemPtr {
        match self {
            LinkSlot::List(b) => mem[b].list_ptr,
            LinkSlot::Node(p) => p.link(mem),
        }
    }

    #[inline]
    pub fn set(self, mem: &mut MemState, q: MemPtr) {
        match self {
            LinkSlot::List(b) => mem[b].list_ptr = q,
            LinkSlot::Node(p) => p.set_link(mem, q),
        }
    }

    #[inline]
    pub fn node(self) -> Option<MemIdx> {
        match self {
            LinkSlot::List(_) => None,
            LinkSlot::Node(p) => Some(p),
        }
    }

    pub fn tail(mut self, mem: &MemState) -> LinkSlot {
        while let Some(q) = self.get(mem) {
            self = LinkSlot::Node(q);
        }
        self
    }

    pub fn before(mut self, mem: &MemState, target: MemPtr) -> Result<LinkSlot, Abort> {
        while self.get(mem) != target {
            self = LinkSlot::Node(self.get(mem).or_confusion("predecessor")?);
        }
        Ok(self)
    }
}

// A node retyped in place is freed by its new type, so both kinds have one size.
const _: () = assert!(
    UnsetNode::SIZE == BoxNode::SIZE
        && AlignRecord::SIZE == UnsetNode::SIZE
        && MathNode::SIZE == KernNode::SIZE
        && GlueNode::SIZE == KernNode::SIZE
        && GlueNode::SIZE == PenaltyNode::SIZE
        && ChoiceNode::SIZE == StyleNode::SIZE
        && EdgeNode::SIZE == StyleNode::SIZE
);

#[inline]
fn retype<K: View, L: View>(p: NodePtr<K>, mem: &mut MemState, ty: NodeType) -> NodePtr<L> {
    mem[header(p.idx)].ty = ty.to_u16();
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

#[derive(Copy, Clone, PartialEq, Eq, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub struct StoredDimen(i32);

impl StoredDimen {
    #[inline]
    pub fn get(self) -> RuleDimen {
        RuleDimen::from_raw(self.0)
    }

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

/// A classified lo-mem node. Type 2 is read as a rule (not a delta node) and
/// type 14 as a style node (not an edge): walkers of those special lists
/// know their context and do not classify.
#[derive(Copy, Clone, Debug)]
pub enum AnyNode {
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
    Noad(NodePtr<AnyNoad>),
    MarginKern(NodePtr<MarginKernNode>),
    Char(NodePtr<CharNode>),
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
            AnyNode::Char(p) => p.idx(),
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
            AnyNode::Whatsit(p) => p.dims(mem).map_or(Scaled::ZERO, |d| d.width),
            _ => Scaled::ZERO,
        }
    }
}

impl MemState {
    #[inline]
    pub fn set_header(&mut self, p: impl MemAddr, ty: NodeType, subtype: u16) {
        let h = &mut self[header(p)];
        h.ty = ty.to_u16();
        h.subtype = subtype;
    }

    /// The raw type code, for `\lastnodetype` arithmetic and diagnostics.
    #[inline]
    pub fn type_code(&self, p: impl MemAddr) -> u16 {
        self[header(p)].ty
    }

    #[inline]
    pub fn node_type(&self, p: impl MemAddr) -> Result<NodeType, InvalidNodeType> {
        NodeType::of_code(self[header(p)].ty)
    }

    #[inline]
    pub fn classify(&self, p: impl MemAddr) -> Result<AnyNode, InvalidNodeType> {
        use NodeType::*;
        let p = p.addr();
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
            Char => AnyNode::Char(NodePtr::from_idx_unchecked(p)),
            Head => return Err(InvalidNodeType(Head.to_u16())),
        })
    }

    #[inline]
    pub fn classify_non_char(&self, p: MemPtr) -> Option<AnyNode> {
        let p = p?;
        if self.is_char_node(p) {
            return None;
        }
        self.classify(p).ok()
    }

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

impl MemState {
    #[inline]
    pub fn as_char(&self, p: MemPtr) -> Option<NodePtr<CharNode>> {
        p.filter(|&p| self.is_char_node(p)).map(NodePtr::at)
    }
}

impl NodePtr<MathNode> {
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
    #[inline]
    pub fn into_style(self, mem: &mut MemState, style: MathStyle) -> NodePtr<StyleNode> {
        let p: NodePtr<StyleNode> = retype(self, mem, NodeType::Style);
        let node = &mut mem[p];
        node.style = style;
        node.clear_payload();
        p
    }
}

pub trait NoadKind: View {}

impl NoadKind for Noad {}
impl NoadKind for RadicalNoad {}
impl NoadKind for AccentNoad {}
impl NoadKind for FractionNoad {}
impl NoadKind for FenceNoad {}

impl<K: NoadKind> NodePtr<K> {
    #[inline]
    pub fn any(self) -> NodePtr<AnyNoad> {
        NodePtr::from_idx_unchecked(self.idx)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum NoadVariant {
    Noad(NodePtr<Noad>),
    Radical(NodePtr<RadicalNoad>),
    Accent(NodePtr<AccentNoad>),
    Fraction(NodePtr<FractionNoad>),
    Fence(NodePtr<FenceNoad>),
}

impl NoadVariant {
    #[inline]
    pub fn as_noad(self) -> Option<NodePtr<Noad>> {
        match self {
            NoadVariant::Noad(n) => Some(n),
            NoadVariant::Radical(r) => Some(r.as_noad()),
            NoadVariant::Accent(a) => Some(a.as_noad()),
            NoadVariant::Fraction(_) | NoadVariant::Fence(_) => None,
        }
    }

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

impl NodePtr<RadicalNoad> {
    #[inline]
    pub fn as_noad(self) -> NodePtr<Noad> {
        NodePtr::from_idx_unchecked(self.idx)
    }
}

impl NodePtr<AccentNoad> {
    #[inline]
    pub fn as_noad(self) -> NodePtr<Noad> {
        NodePtr::from_idx_unchecked(self.idx)
    }
}

/// A math field of a noad that something writes into: the target of `scan_math`,
/// which a math group fills when it ends, or a character `fetch` may find missing.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum MathSlot {
    Nucleus(NodePtr<Noad>),
    Supscr(NodePtr<Noad>),
    Subscr(NodePtr<Noad>),
    /// A fraction's numerator, where WEB's `supscr` lands on a fraction.
    Numerator(NodePtr<FractionNoad>),
    /// A fraction's denominator, where WEB's `subscr` lands on a fraction.
    Denominator(NodePtr<FractionNoad>),
    AccentChr(NodePtr<AccentNoad>),
}

impl Index<MathSlot> for MemState {
    type Output = MathWord;

    #[inline]
    fn index(&self, s: MathSlot) -> &MathWord {
        match s {
            MathSlot::Nucleus(n) => &self[n].nucleus,
            MathSlot::Supscr(n) => &self[n].supscr,
            MathSlot::Subscr(n) => &self[n].subscr,
            MathSlot::Numerator(f) => &self[f].numerator,
            MathSlot::Denominator(f) => &self[f].denominator,
            MathSlot::AccentChr(a) => &self[a].accent_chr,
        }
    }
}

impl IndexMut<MathSlot> for MemState {
    #[inline]
    fn index_mut(&mut self, s: MathSlot) -> &mut MathWord {
        match s {
            MathSlot::Nucleus(n) => &mut self[n].nucleus,
            MathSlot::Supscr(n) => &mut self[n].supscr,
            MathSlot::Subscr(n) => &mut self[n].subscr,
            MathSlot::Numerator(f) => &mut self[f].numerator,
            MathSlot::Denominator(f) => &mut self[f].denominator,
            MathSlot::AccentChr(a) => &mut self[a].accent_chr,
        }
    }
}

impl NodePtr<GlueNode> {
    #[inline]
    pub fn into_penalty(self, mem: &mut MemState) -> NodePtr<PenaltyNode> {
        retype(self, mem, NodeType::Penalty)
    }

    #[inline]
    pub fn into_kern(self, mem: &mut MemState, w: Scaled) -> NodePtr<KernNode> {
        let kern: NodePtr<KernNode> = retype(self, mem, NodeType::Kern);
        mem[kern].width = w;
        kern
    }
}

impl NodePtr<UnsetNode> {
    #[inline]
    pub fn into_box(self, mem: &mut MemState, o: Orientation) -> NodePtr<BoxNode> {
        retype(self, mem, o.node_type())
    }
}

/// A column of an alignment preamble or row. Columns alternate with tabskip glue.
pub trait AlignColumn: HasLink {}

impl AlignColumn for AlignRecord {}
impl AlignColumn for UnsetNode {}

impl<K: AlignColumn> NodePtr<K> {
    #[inline]
    pub fn after_tab_skip(mem: &MemState, g: impl MemAddr) -> Option<Self> {
        // SAFETY: tabskip glue is followed by a column or ends the list.
        g.addr().link(mem).map(NodePtr::from_idx_unchecked)
    }

    #[inline]
    pub fn tab_skip(self, mem: &MemState) -> Result<NodePtr<GlueNode>, Abort> {
        NodePtr::tab_skip_after(mem, self)
    }

    #[inline]
    pub fn next_column(self, mem: &MemState) -> Option<Self> {
        Self::after_tab_skip(mem, self.idx().link(mem)?)
    }
}

impl NodePtr<GlueNode> {
    #[inline]
    pub fn tab_skip_after(mem: &MemState, p: impl MemAddr) -> Result<Self, Abort> {
        // SAFETY: tabskip glue follows the align head and every column.
        p.addr()
            .link(mem)
            .map(NodePtr::from_idx_unchecked)
            .or_confusion("tabskip glue")
    }
}

impl NodePtr<AlignRecord> {
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

// Either array takes one word more than twice its first word.
const _: () = assert!(ParShapeNode::SIZE == 1 && PenaltiesNode::SIZE == 2);

impl MemState {
    /// Frees the `\parshape` or e-TeX penalty array `p` an eqtb shape entry holds. The shape
    /// view frees either, since both sizes follow from the first word alike.
    pub(crate) fn free_shape_ref(&mut self, p: MemPtr) {
        if let Some(shape) = NodePtr::<ParShapeNode>::from_ptr(p) {
            self.free_array(shape);
        }
    }
}

impl NodePtr<ParShapeNode> {
    /// Line `k`, counting from 1. Panics outside the shape.
    #[inline]
    pub fn line(self, mem: &MemState, k: i32) -> ParShapeLine {
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

code_enum! {
    /// The subtype of a whatsit node. Only `NativeWord`..`Pdf` carry dimensions.
    pub enum WhatsitSubtype: u16 {
        Open = 0,
        Write = 1,
        Close = 2,
        Special = 3,
        /// `\special shipout`, expanded when shipped out.
        LateSpecial = 4,
        Language = 5,
        PdfSavePos = 23,
        NativeWord = 40,
        NativeWordAt = 41,
        Glyph = 42,
        Pic = 43,
        Pdf = 44,
    }
}

impl WhatsitSubtype {
    pub fn is_native_word(self) -> bool {
        matches!(
            self,
            WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt
        )
    }

    pub fn is_io(self) -> bool {
        matches!(
            self,
            WhatsitSubtype::Open
                | WhatsitSubtype::Write
                | WhatsitSubtype::Close
                | WhatsitSubtype::Special
                | WhatsitSubtype::LateSpecial
                | WhatsitSubtype::Language
        )
    }
}

pub trait WhatsitKind: View {}

impl WhatsitKind for WhatsitNode {}
impl WhatsitKind for OpenNode {}
impl WhatsitKind for WriteNode {}
impl WhatsitKind for LanguageNode {}
impl WhatsitKind for GlyphNode {}
impl WhatsitKind for PicNode {}
impl WhatsitKind for NativeWordNode {}
impl WhatsitKind for SavePosNode {}

impl<K: WhatsitKind> NodePtr<K> {
    #[inline]
    pub fn whatsit(self) -> NodePtr<WhatsitNode> {
        NodePtr::from_idx_unchecked(self.idx)
    }
}

impl<K: WhatsitKind + Node> NodePtr<K> {
    #[inline]
    pub fn alloc_whatsit(mem: &mut MemState, subtype: WhatsitSubtype) -> Result<Self, Abort> {
        let p = Self::alloc(mem)?;
        mem[p.whatsit()].subtype = subtype.into();
        Ok(p)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum WhatsitVariant {
    Open(NodePtr<OpenNode>),
    Write(NodePtr<WriteNode>),
    Close(NodePtr<WriteNode>),
    Special(NodePtr<WriteNode>),
    LateSpecial(NodePtr<WriteNode>),
    Language(NodePtr<LanguageNode>),
    SavePos(NodePtr<SavePosNode>),
    NativeWord(NodePtr<NativeWordNode>),
    Glyph(NodePtr<GlyphNode>),
    Pic(NodePtr<PicNode>),
}

impl NodePtr<WhatsitNode> {
    #[inline]
    pub fn variant(self, mem: &MemState) -> Option<WhatsitVariant> {
        use WhatsitSubtype as S;
        let p = self.idx;
        Some(match mem[self].subtype.get()? {
            S::Open => WhatsitVariant::Open(NodePtr::from_idx_unchecked(p)),
            S::Write => WhatsitVariant::Write(NodePtr::from_idx_unchecked(p)),
            S::Close => WhatsitVariant::Close(NodePtr::from_idx_unchecked(p)),
            S::Special => WhatsitVariant::Special(NodePtr::from_idx_unchecked(p)),
            S::LateSpecial => WhatsitVariant::LateSpecial(NodePtr::from_idx_unchecked(p)),
            S::Language => WhatsitVariant::Language(NodePtr::from_idx_unchecked(p)),
            S::PdfSavePos => WhatsitVariant::SavePos(NodePtr::from_idx_unchecked(p)),
            S::NativeWord | S::NativeWordAt => {
                WhatsitVariant::NativeWord(NodePtr::from_idx_unchecked(p))
            }
            S::Glyph => WhatsitVariant::Glyph(NodePtr::from_idx_unchecked(p)),
            S::Pic | S::Pdf => WhatsitVariant::Pic(NodePtr::from_idx_unchecked(p)),
        })
    }

    #[inline]
    pub fn dims(self, mem: &MemState) -> Option<Dims> {
        let dims = |width, depth, height| Dims {
            width,
            depth,
            height,
        };
        match self.variant(mem)? {
            WhatsitVariant::NativeWord(w) => Some(dims(mem[w].width, mem[w].depth, mem[w].height)),
            WhatsitVariant::Glyph(g) => Some(dims(mem[g].width, mem[g].depth, mem[g].height)),
            WhatsitVariant::Pic(p) => Some(dims(mem[p].width, mem[p].depth, mem[p].height)),
            _ => None,
        }
    }

    #[inline]
    pub fn whatsit_subtype(self, mem: &MemState) -> Option<WhatsitSubtype> {
        WhatsitSubtype::from_u16(mem[header(self.idx)].subtype)
    }
}

impl NodePtr<NativeWordNode> {
    pub fn release_native_glyphs(self, mem: &mut MemState) {
        let handle = mem[self].glyph_info;
        if handle != 0 {
            mem.glyph_blobs.release(handle);
            mem[self].clear_glyphs();
        }
    }

    pub fn share_native_glyphs(self, mem: &mut MemState, src: Self) {
        let src = mem[src];
        if src.glyph_info != 0 {
            let copy = mem.glyph_blobs.clone_handle(src.glyph_info);
            let word = &mut mem[self];
            word.glyph_info = copy;
            word.glyph_count = src.glyph_count;
        }
    }

    pub fn free_native(self, mem: &mut MemState) {
        self.release_native_glyphs(mem);
        mem.free_array(self);
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;
    use crate::base::codes::LimitCode;
    use crate::base::token::TokenList;
    use crate::fonts::Side;
    use crate::memory::glue_store::GlueSpecId;
    use crate::memory::mem::RESERVED_WORDS;
    use crate::memory::mem_types::zeroed_vec;

    fn fresh_mem(n: usize) -> MemState {
        MemState {
            data: zeroed_vec(n).into(),
            ..Default::default()
        }
    }

    /// Checks a code set against WEB's numbers, which formats and checkpoints store raw.
    fn assert_web_codes<T: StoredCode + PartialEq + Debug>(codes: &[(T, u16)]) {
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
        let old = NodePtr::<PenaltyNode>::alloc(&mut mem).unwrap();
        mem[header(old.idx())].subtype = 0xBEEF;
        old.free(&mut mem);
        let p = NodePtr::<PenaltyNode>::alloc(&mut mem).unwrap();
        assert_eq!(p, old);
        assert_eq!(mem[header(p.idx())].subtype, 0);
        assert_eq!(mem.node_type(p), Ok(NodeType::Penalty));
    }

    #[test]
    fn freed_nodes_return_to_the_list_of_their_size() {
        let mut mem = arena_with_free_block();
        let start = RESERVED_WORDS as i32;
        let a = mem.get_node(3).unwrap();
        let b = mem.get_node(3).unwrap();
        let c = mem.get_node(2).unwrap();
        assert_eq!([a, b, c].map(MemIdx::raw), [start, start + 3, start + 6]);
        mem.free_node(a, 3);
        mem.free_node(b, 3);
        assert_eq!(mem.get_node(2).unwrap().raw(), start + 8);
        assert_eq!(mem.get_node(3).unwrap(), b);
        assert_eq!(mem.get_node(3).unwrap(), a);
        assert_eq!(mem.data.len(), RESERVED_WORDS + 10);
    }

    fn arena_with_free_block() -> MemState {
        fresh_mem(RESERVED_WORDS)
    }

    fn at<K: View>(mem: &mut MemState, p: i32, ty: Option<NodeType>) -> NodePtr<K> {
        if let Some(ty) = ty {
            mem[header(MemIdx::from_raw(p).unwrap())].ty = ty.to_u16();
        }
        NodePtr::from_idx_unchecked(MemIdx::from_raw(p).unwrap())
    }

    #[test]
    fn handles_are_zero_cost() {
        assert_eq!(size_of::<NodePtr<BoxNode>>(), size_of::<i32>());
        assert_eq!(size_of::<GlueSpecId>(), size_of::<i32>());
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
        let s = mem.glue.alloc().unwrap();
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
    fn alloc_stamps_the_sync_fields_of_synced_kinds() {
        let mut mem = arena_with_free_block();
        (mem.cur_synctex_tag, mem.line) = (42, 7);
        let b = NodePtr::<BoxNode>::alloc(&mut mem).unwrap();
        assert_eq!((mem[b].sync.tag, mem[b].sync.line), (42, 7));
        let k = NodePtr::<KernNode>::alloc(&mut mem).unwrap();
        assert_eq!((mem[k].sync.tag, mem[k].sync.line), (42, 7));
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
            mem[header(MemIdx::from_raw(p).unwrap())].ty = ty.to_u16();
        }
        mem[header(MemIdx::from_raw(180).unwrap())].ty = 35;
        let c = |p| mem.classify(MemIdx::from_raw(p).unwrap());
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
            mem.classify_box(MemIdx::from_raw(50).unwrap()),
            Ok(Some(b)) if mem[b].is_vlist()
        ));
        assert!(matches!(
            mem.classify_box(MemIdx::from_raw(70).unwrap()),
            Ok(None)
        ));
        assert!(mem.classify_box(MemIdx::from_raw(180).unwrap()).is_err());
    }

    #[test]
    fn rule_dimensions_model_the_running_sentinel() {
        let mut mem = fresh_mem(200);
        let r: NodePtr<RuleNode> = at(&mut mem, 100, Some(NodeType::Rule));
        mem[r].width = RuleDimen::Running.into();
        mem[r].depth = RuleDimen::Set(Scaled::ZERO).into();
        mem[r].height = RuleDimen::Set(Scaled::from_raw(NULL_FLAG + 1)).into();
        assert_eq!(mem[r].width.get(), RuleDimen::Running);
        assert_eq!(mem[r].depth.get(), RuleDimen::Set(Scaled::ZERO));
        assert_eq!(
            mem[r].height.get(),
            RuleDimen::Set(Scaled::from_raw(NULL_FLAG + 1))
        );
    }

    #[test]
    fn alignment_records_walk_the_preamble_and_become_unset() {
        let mut mem = fresh_mem(200);
        let idx = |raw| MemIdx::from_raw(raw).unwrap();
        let (head, g1, col, g2) = (idx(10), idx(20), idx(100), idx(30));
        head.set_link(&mut mem, Some(g1));
        g1.set_link(&mut mem, Some(col));
        col.set_link(&mut mem, Some(g2));
        g2.set_link(&mut mem, None);
        let rec = NodePtr::<AlignRecord>::after_tab_skip(&mem, g1).unwrap();
        assert_eq!(rec.idx(), col);
        assert_eq!(rec.tab_skip(&mem).unwrap().idx(), g2);
        assert_eq!(
            NodePtr::<GlueNode>::tab_skip_after(&mem, head)
                .unwrap()
                .idx(),
            g1
        );
        assert!(rec.next_column(&mem).is_none());

        mem[rec].width = Scaled::from_raw(0x44);
        mem[rec].v_part = TokenList::from_raw(0x22);
        mem[rec].u_part = TokenList::from_raw(0x33);
        mem[rec].extra_info = 0x55;
        mem[rec].span_list = Some(NodePtr::at(MemIdx::from_raw(40).unwrap()));

        let u = rec.into_unset(&mut mem);
        let unset = mem[u];
        assert_eq!(unset.link, Some(g2));
        assert_eq!(unset.width, Scaled::from_raw(0x44));
        assert_eq!(unset.span_count, 0);
        assert_eq!((unset.height, unset.depth), (Scaled::ZERO, Scaled::ZERO));
        assert_eq!(
            (unset.glue_stretch, unset.glue_shrink),
            (Scaled::ZERO, Scaled::ZERO)
        );
        assert_eq!(
            (unset.glue_order.get(), unset.glue_sign.get()),
            (Some(GlueOrder::Normal), Some(GlueOrder::Normal))
        );
    }

    #[test]
    fn math_slots_reach_their_fields() {
        let mut mem = fresh_mem(200);
        let n: NodePtr<Noad> = at(&mut mem, 100, Some(NodeType::OrdNoad));
        let f: NodePtr<FractionNoad> = at(&mut mem, 120, Some(NodeType::FractionNoad));
        let a: NodePtr<AccentNoad> = at(&mut mem, 140, Some(NodeType::AccentNoad));
        let slots = [
            MathSlot::Nucleus(n),
            MathSlot::Supscr(n),
            MathSlot::Subscr(n),
            MathSlot::Numerator(f),
            MathSlot::Denominator(f),
            MathSlot::AccentChr(a),
        ];
        for (k, slot) in (1..).zip(slots) {
            mem[slot].math_type = k;
        }
        assert_eq!(
            [mem[n].nucleus, mem[n].supscr, mem[n].subscr].map(|w| w.math_type),
            [1, 2, 3]
        );
        assert_eq!(
            [mem[f].numerator, mem[f].denominator].map(|w| w.math_type),
            [4, 5]
        );
        assert_eq!(mem[a].accent_chr.math_type, 6);
        assert_eq!(
            mem[a.as_noad()].nucleus.math_type,
            mem[a].base.nucleus.math_type
        );
    }

    #[test]
    fn math_and_delimiter_fields_pack_their_codes() {
        let mut f = <MathWord as bytemuck::Zeroable>::zeroed();
        f.math_type = 3;
        f.set_character(0x1234);
        f.set_plane_and_fam(0x0205);
        assert_eq!(f.fam(), 5);
        assert_eq!(f.full_char(), 0x2_1234);
        f.set_info(MemIdx::from_raw(0x0012_3456));
        assert_eq!(f.info(), MemIdx::from_raw(0x0012_3456));
        f.set_info(None);
        assert_eq!(f.info(), None);
        assert_eq!(f.math_type, 3);

        let mut d = <DelimiterWord as bytemuck::Zeroable>::zeroed();
        d.set_small(0x0103, 0x41);
        d.set_large(0x0204, 0x42);
        assert_eq!(d.small(), (3, 0x1_0041));
        assert_eq!(d.large(), (4, 0x2_0042));
        assert!(!d.is_null());
        d.clear();
        assert!(d.is_null());
    }
}
