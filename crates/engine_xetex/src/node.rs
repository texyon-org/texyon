//! Typed node handles over the flat `mem` arena.
//!
//! [`NodePtr<K>`] is a `Copy` index plus a compile-time kind marker, so a field
//! access compiles to the same load/store as the raw `mem[p + k]` form. Fields
//! are read by passing the arena as context: `p.width(mem)`,
//! `p.set_width(mem, w)`.

use std::marker::PhantomData;

use crate::consts::{
    ACCENT_NOAD_SIZE, BOX_NODE_SIZE, EMPTY, FRACTION_NOAD_SIZE, GLUE_SPEC_SIZE, INS_NODE_SIZE,
    MARGIN_KERN_NODE_SIZE, MEDIUM_NODE_SIZE, NATIVE_NODE_SIZE, NOAD_SIZE, NORMAL, NULL_FLAG,
    PIC_NODE_SIZE, RADICAL_NOAD_SIZE, RULE_NODE_SIZE, SMALL_NODE_SIZE, STYLE_NODE_SIZE,
};
use crate::errors::Abort;
use crate::state::MemState;
use crate::units::{
    FontId, GlueSign, InvalidNodeType, MemIdx, MemPtr, NodeType, Scaled, StrNumber,
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
impl<K: NodeKind> std::fmt::Debug for NodePtr<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodePtr({})", self.idx.0)
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
        Scaled(raw)
    }
    #[inline(always)]
    fn encode(self) -> i32 {
        self.0
    }
}

impl Slot for MemPtr {
    type Raw = i32;
    #[inline(always)]
    fn decode(raw: i32) -> Self {
        MemPtr(raw)
    }
    #[inline(always)]
    fn encode(self) -> i32 {
        self.0
    }
}

impl Slot for NodePtr<GlueSpecNode> {
    type Raw = i32;
    #[inline(always)]
    fn decode(raw: i32) -> Self {
        NodePtr::from_glue_ptr(MemPtr(raw))
    }
    #[inline(always)]
    fn encode(self) -> i32 {
        self.idx.0
    }
}

impl Slot for StrNumber {
    type Raw = i32;
    #[inline(always)]
    fn decode(raw: i32) -> Self {
        StrNumber(raw)
    }
    #[inline(always)]
    fn encode(self) -> i32 {
        self.0
    }
}

impl Slot for FontId {
    type Raw = u16;
    #[inline(always)]
    fn decode(raw: u16) -> Self {
        FontId(raw as i32)
    }
    #[inline(always)]
    fn encode(self) -> u16 {
        self.0 as u16
    }
}

impl Slot for GlueSign {
    type Raw = u16;
    #[inline(always)]
    fn decode(raw: u16) -> Self {
        GlueSign::from_raw(raw)
    }
    #[inline(always)]
    fn encode(self) -> u16 {
        self.as_raw()
    }
}

impl Slot for RuleDimen {
    type Raw = i32;
    #[inline(always)]
    fn decode(raw: i32) -> Self {
        RuleDimen::from_raw(raw)
    }
    #[inline(always)]
    fn encode(self) -> i32 {
        self.raw()
    }
}

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
            <$ty as Slot>::decode(mem.$slot(self.idx.0 + $off))
        }
        $(
            #[inline]
            pub fn $set(self, mem: &mut MemState, v: $ty) {
                write_slot!(mem, $slot, self.idx.0 + $off, <$ty as Slot>::encode(v))
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
    HListNode = HList, BOX_NODE_SIZE;
    VListNode = VList, BOX_NODE_SIZE;
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

/// A `link` field in word 0.
pub trait HasLink: NodeKind {}
/// A list-node header: type in `b16.s1`, subtype in `b16.s0`, link in `b32.s1`.
pub trait HasNodeHeader: HasLink {}
/// A `Scaled` width at +1.
pub trait HasWidth: NodeKind {}
/// `Scaled` depth and height at +2 and +3.
pub trait HasHeightDepth: NodeKind {}
/// A child list at +5.
pub trait HasListPtr: NodeKind {}
/// The hlist/vlist box layout: shift at +4 and glue setting at +5 and +6.
pub trait IsBox: HasWidth + HasHeightDepth + HasListPtr {}
/// The SyncTeX tag and line that `get_node` stamps into a node's last word.
pub trait HasSyncFields: FixedNode {}

impl<K: HasNodeHeader> HasLink for K {}

macro_rules! impl_traits {
    ($trait:ident: $($kind:ident),*) => {$(impl $trait for $kind {})*};
}

impl_traits!(HasNodeHeader: HListNode, VListNode, RuleNode, InsNode, MarkNode, AdjustNode,
    LigatureNode, DiscNode, WhatsitNode, MathNode, GlueNode, KernNode, PenaltyNode, UnsetNode,
    StyleNode, ChoiceNode, EdgeNode, Noad, MarginKernNode, SaNode, PageInsNode, DeltaNode);
impl_traits!(HasLink: MovementNode, ActiveNode, PassiveNode, CondNode, CharNode,
    AlignRecord, AlignStackNode, ExprNode, PseudoLineNode, LigItem);
impl_traits!(HasWidth: HListNode, VListNode, UnsetNode, WhatsitNode, MathNode, KernNode,
    EdgeNode, MarginKernNode, GlueSpecNode, AlignRecord, SpanNode);
impl_traits!(HasHeightDepth: HListNode, VListNode, UnsetNode, WhatsitNode, InsNode);
impl_traits!(HasListPtr: HListNode, VListNode, UnsetNode);
impl_traits!(IsBox: HListNode, VListNode);
impl_traits!(HasSyncFields: HListNode, VListNode, RuleNode, GlueNode, KernNode, MathNode);

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

impl<K: ListRecord + FixedNode> NodePtr<K> {
    #[inline]
    pub fn free(self, mem: &mut MemState) {
        mem.free_node(self.ptr(), K::SIZE);
    }
}

impl<K: HasLink> NodePtr<K> {
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

impl<K: HasListPtr> NodePtr<K> {
    fields! {
        list_ptr / set_list_ptr: MemPtr = b32s1 @ 5;
    }
}

impl<K: IsBox> NodePtr<K> {
    fields! {
        shift_amount / set_shift_amount: Scaled = b32s1 @ 4;
        glue_sign / set_glue_sign: GlueSign = b16s1 @ 5;
        glue_order / set_glue_order: u16 = b16s0 @ 5;
    }

    #[inline]
    pub fn glue_set(self, mem: &MemState) -> f64 {
        mem.word(self.idx.0 + 6).gr()
    }
    #[inline]
    pub fn set_glue_set(self, mem: &mut MemState, v: f64) {
        mem.set_gr(self.idx.0 + 6, v);
    }
}

impl<K: HasSyncFields> NodePtr<K> {
    fields! {
        sync_tag / set_sync_tag: i32 = b32s0 @ K::SIZE - 1;
        sync_line: i32 = b32s1 @ K::SIZE - 1;
    }
}

impl MemPtr {
    #[inline]
    pub fn link(self, mem: &MemState) -> MemPtr {
        MemPtr(mem.b32s1(self.0))
    }
    #[inline]
    pub fn set_link(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.0, q.0);
    }
    /// Knuth's `info`, the payload half of a one-word node.
    #[inline]
    pub fn info(self, mem: &MemState) -> i32 {
        mem.b32s0(self.0)
    }
    #[inline]
    pub fn set_info(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self.0, v);
    }

    /// The token held by a token-list node.
    #[inline]
    pub fn token(self, mem: &MemState) -> i32 {
        mem.b32s0(self.0)
    }
    #[inline]
    pub fn set_token(self, mem: &mut MemState, t: i32) {
        mem.set_b32s0(self.0, t);
    }
    /// The reference count in a token list's head node, one less than the
    /// number of references.
    #[inline]
    pub fn token_ref_count(self, mem: &MemState) -> i32 {
        mem.b32s0(self.0)
    }
    #[inline]
    pub fn set_token_ref_count(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self.0, v);
    }
}

// --- boxes and rules ---

impl NodePtr<HListNode> {
    #[inline]
    pub fn into_vlist(self, mem: &mut MemState) -> NodePtr<VListNode> {
        retype(self, mem, NodeType::VList)
    }
}

/// Stamps a new type code and rewraps the handle.
#[inline]
fn retype<K: NodeKind, L: NodeKind>(p: NodePtr<K>, mem: &mut MemState, ty: NodeType) -> NodePtr<L> {
    mem.set_b16s1(p.idx.0, ty.as_raw());
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
            RuleDimen::Set(Scaled(raw))
        }
    }

    #[inline]
    pub fn raw(self) -> i32 {
        match self {
            RuleDimen::Running => NULL_FLAG,
            RuleDimen::Set(v) => v.0,
        }
    }

    /// The stored word as a length, `RUNNING` for a running dimension.
    #[inline]
    pub fn to_scaled(self) -> Scaled {
        Scaled(self.raw())
    }
}

/// A running rule dimension seen as a length, as TeX's arithmetic sees it.
pub(crate) const RUNNING: Scaled = Scaled(NULL_FLAG);

impl NodePtr<RuleNode> {
    fields! {
        width / set_width: RuleDimen = b32s1 @ 1;
        depth / set_depth: RuleDimen = b32s1 @ 2;
        height / set_height: RuleDimen = b32s1 @ 3;
    }
}

/// A classified box: either an hlist or a vlist node.
#[derive(Copy, Clone, Debug)]
pub enum AnyBox {
    HList(NodePtr<HListNode>),
    VList(NodePtr<VListNode>),
}

macro_rules! box_delegate {
    ($($name:ident($($arg:ident: $t:ty),*) $(-> $ret:ty)?;)*) => {$(
        #[inline]
        pub fn $name(self, $($arg: $t),*) $(-> $ret)? {
            match self {
                AnyBox::HList(p) => p.$name($($arg),*),
                AnyBox::VList(p) => p.$name($($arg),*),
            }
        }
    )*};
}

impl AnyBox {
    box_delegate! {
        idx() -> MemIdx;
        ptr() -> MemPtr;
        sync_tag(mem: &MemState) -> i32;
        sync_line(mem: &MemState) -> i32;
        width(mem: &MemState) -> Scaled;
        depth(mem: &MemState) -> Scaled;
        height(mem: &MemState) -> Scaled;
        shift_amount(mem: &MemState) -> Scaled;
        list_ptr(mem: &MemState) -> MemPtr;
        subtype(mem: &MemState) -> u16;
        glue_sign(mem: &MemState) -> GlueSign;
        glue_order(mem: &MemState) -> u16;
        glue_set(mem: &MemState) -> f64;
        set_width(mem: &mut MemState, w: Scaled);
        set_depth(mem: &mut MemState, d: Scaled);
        set_height(mem: &mut MemState, h: Scaled);
        set_shift_amount(mem: &mut MemState, s: Scaled);
        set_list_ptr(mem: &mut MemState, q: MemPtr);
        set_link(mem: &mut MemState, q: MemPtr);
        set_subtype(mem: &mut MemState, v: u16);
        set_glue_sign(mem: &mut MemState, v: GlueSign);
        set_glue_order(mem: &mut MemState, v: u16);
        set_glue_set(mem: &mut MemState, v: f64);
    }

    #[inline]
    pub fn into_unset(self, mem: &mut MemState) -> NodePtr<UnsetNode> {
        match self {
            AnyBox::HList(p) => retype(p, mem, NodeType::Unset),
            AnyBox::VList(p) => retype(p, mem, NodeType::Unset),
        }
    }

    /// The pseudo-node whose `link` is `list_ptr`, for splicing at the list head.
    #[inline]
    pub fn list_head(self) -> MemPtr {
        MemPtr::from_idx(self.idx().offset(5))
    }
}

/// What `\box`, `\hrule` and friends leave in `cur_box`: a rule only where
/// leaders accept one.
#[derive(Copy, Clone, Debug)]
pub enum BoxOrRule {
    Box(AnyBox),
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
    HList(NodePtr<HListNode>),
    VList(NodePtr<VListNode>),
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
            AnyNode::HList(p) => p.idx(),
            AnyNode::VList(p) => p.idx(),
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
            AnyNode::HList(_)
                | AnyNode::VList(_)
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
            AnyNode::HList(p) => p.width(mem),
            AnyNode::VList(p) => p.width(mem),
            AnyNode::Rule(p) => p.width(mem).to_scaled(),
            AnyNode::Kern(p) => p.width(mem),
            AnyNode::Math(p) => p.width(mem),
            AnyNode::Whatsit(p) => p.width(mem),
            _ => Scaled::ZERO,
        }
    }
}

impl MemState {
    /// The raw type code, for `\lastnodetype` arithmetic and diagnostics.
    #[inline]
    pub fn type_code(&self, p: MemIdx) -> u16 {
        self.b16s1(p.0)
    }

    /// The checked `NODE_type` of the non-null lo-mem node `p`.
    #[inline]
    pub fn node_type(&self, p: MemIdx) -> Result<NodeType, InvalidNodeType> {
        NodeType::from_raw(self.b16s1(p.0))
    }

    /// Classifies the non-null lo-mem node `p`. `Err` for an invalid type code.
    #[inline]
    pub fn classify(&self, p: MemIdx) -> Result<AnyNode, InvalidNodeType> {
        use NodeType::*;
        // SAFETY (every arm): the type code just read from `p` proves its kind.
        Ok(match self.node_type(p)? {
            HList => AnyNode::HList(NodePtr::from_idx_unchecked(p)),
            VList => AnyNode::VList(NodePtr::from_idx_unchecked(p)),
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
        if p.is_null() || p.0 >= self.hi_mem_min {
            return None;
        }
        self.classify(MemIdx(p.0)).ok()
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
    pub fn classify_box(&self, p: MemIdx) -> Result<Option<AnyBox>, InvalidNodeType> {
        Ok(match self.classify(p)? {
            AnyNode::HList(b) => Some(AnyBox::HList(b)),
            AnyNode::VList(b) => Some(AnyBox::VList(b)),
            _ => None,
        })
    }

    #[inline]
    pub fn classify_disc(&self, p: MemIdx) -> Result<Option<NodePtr<DiscNode>>, InvalidNodeType> {
        Ok(match self.classify(p)? {
            AnyNode::Disc(d) => Some(d),
            _ => None,
        })
    }

    #[inline]
    pub fn classify_noad(&self, p: MemIdx) -> Result<Option<NodePtr<Noad>>, InvalidNodeType> {
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
        (p.0 >= self.hi_mem_min).then(|| NodePtr::at(p))
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
        NodePtr::at(self.idx.0 + 1)
    }
}

impl NodePtr<LigItem> {
    fields! {
        character / set_character: u16 = b16s0 @ 0;
        component / set_component: MemPtr = b32s1 @ 1;
    }
}

impl NodePtr<DiscNode> {
    fields! {
        pre_break / set_pre_break: MemPtr = b32s0 @ 1;
        post_break / set_post_break: MemPtr = b32s1 @ 1;
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
    /// Zeroes the two payload words, as `new_style` does.
    #[inline]
    pub fn clear_payload(self, mem: &mut MemState) {
        mem.set_b32s1(self.idx.0 + 1, 0);
        mem.set_b32s1(self.idx.0 + 2, 0);
    }

    /// Reads a type-14 node of a shipped list as the edge it is.
    #[inline]
    pub fn as_edge(self) -> NodePtr<EdgeNode> {
        NodePtr::from_idx_unchecked(self.idx)
    }
}

impl NodePtr<EdgeNode> {
    fields! {
        edge_dist / set_edge_dist: Scaled = b32s1 @ 2;
    }
}

impl NodePtr<ChoiceNode> {
    fields! {
        display_mlist / set_display_mlist: MemPtr = b32s0 @ 1;
        text_mlist / set_text_mlist: MemPtr = b32s1 @ 1;
        script_mlist / set_script_mlist: MemPtr = b32s0 @ 2;
        script_script_mlist / set_script_script_mlist: MemPtr = b32s1 @ 2;
    }

    /// Retypes the node as a style node for `style`.
    #[inline]
    pub fn into_style(self, mem: &mut MemState, style: u16) -> NodePtr<StyleNode> {
        let p: NodePtr<StyleNode> = retype(self, mem, NodeType::Style);
        p.set_subtype(mem, style);
        p.clear_payload(mem);
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
        NodeType::from_raw(mem.b16s1(self.idx.0)).expect("valid noad type")
    }
    #[inline]
    pub fn set_noad_type(self, mem: &mut MemState, nt: NodeType) {
        mem.set_b16s1(self.idx.0, nt.as_raw());
    }
    /// Stamps a type code taken from `cur_chr`, which a caller may fix up after.
    #[inline]
    pub fn set_type_code(self, mem: &mut MemState, code: u16) {
        mem.set_b16s1(self.idx.0, code);
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
        NodePtr::at(self.idx.0 + 1)
    }
    #[inline]
    pub fn supscr(self) -> NodePtr<MathField> {
        NodePtr::at(self.idx.0 + 2)
    }
    #[inline]
    pub fn subscr(self) -> NodePtr<MathField> {
        NodePtr::at(self.idx.0 + 3)
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
        NodePtr::at(self.idx.0 + 4)
    }
    /// A left or right noad's delimiter, in the nucleus word.
    #[inline]
    pub fn delimiter(self) -> NodePtr<DelimiterField> {
        NodePtr::at(self.idx.0 + 1)
    }
    /// A radical's delimiter, or a fraction's left one.
    #[inline]
    pub fn left_delimiter(self) -> NodePtr<DelimiterField> {
        NodePtr::at(self.idx.0 + 4)
    }
    #[inline]
    pub fn right_delimiter(self) -> NodePtr<DelimiterField> {
        NodePtr::at(self.idx.0 + 5)
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
        self.plane_and_fam(mem) as i32 % 256
    }
    /// The character code with its plane.
    #[inline]
    pub fn full_char(self, mem: &MemState) -> i32 {
        self.character(mem) as i32 + (self.plane_and_fam(mem) as i32 / 256) * 65_536
    }
    #[inline]
    pub fn clear(self, mem: &mut MemState) {
        self.set_math_type(mem, EMPTY);
        self.set_info(mem, MemPtr::NULL);
    }
    #[inline]
    pub fn copy_from(self, mem: &mut MemState, src: NodePtr<MathField>) {
        mem.set_word(self.idx.0, mem.word(src.idx.0));
    }
}

/// A delimiter's small and large variants, each a family and a character.
impl NodePtr<DelimiterField> {
    #[inline]
    fn variant(plane_and_fam: u16, char: u16) -> (i32, i32) {
        let plane_and_fam = plane_and_fam as i32;
        (
            plane_and_fam % 256,
            char as i32 + (plane_and_fam / 256) * 65_536,
        )
    }
    /// The small variant's `(family, character)`.
    #[inline]
    pub fn small(self, mem: &MemState) -> (i32, i32) {
        Self::variant(mem.b16s3(self.idx.0), mem.b16s2(self.idx.0))
    }
    /// The large variant's `(family, character)`.
    #[inline]
    pub fn large(self, mem: &MemState) -> (i32, i32) {
        Self::variant(mem.b16s1(self.idx.0), mem.b16s0(self.idx.0))
    }
    #[inline]
    pub fn set_small(self, mem: &mut MemState, plane_and_fam: u16, char: u16) {
        mem.set_b16s3(self.idx.0, plane_and_fam);
        mem.set_b16s2(self.idx.0, char);
    }
    #[inline]
    pub fn set_large(self, mem: &mut MemState, plane_and_fam: u16, char: u16) {
        mem.set_b16s1(self.idx.0, plane_and_fam);
        mem.set_b16s0(self.idx.0, char);
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
    fields! {
        glue_ptr / set_glue_ptr: NodePtr<GlueSpecNode> = b32s0 @ 1;
        leader_ptr / set_leader_ptr: MemPtr = b32s1 @ 1;
    }

    #[inline]
    pub fn into_penalty(self, mem: &mut MemState) -> NodePtr<PenaltyNode> {
        retype(self, mem, NodeType::Penalty)
    }

    /// Retypes the node as a kern of width `w`.
    #[inline]
    pub fn into_kern(self, mem: &mut MemState, w: Scaled) -> NodePtr<KernNode> {
        let kern: NodePtr<KernNode> = retype(self, mem, NodeType::Kern);
        kern.set_width(mem, w);
        kern
    }
}

impl NodePtr<GlueSpecNode> {
    /// The spec a glue pointer references. Glue pointers are never null.
    #[inline]
    pub fn from_glue_ptr(p: MemPtr) -> NodePtr<GlueSpecNode> {
        let idx = p.as_idx().expect("null glue spec");
        NodePtr::from_idx_unchecked(idx)
    }

    fields! {
        /// A reference count, not a link, although it shares the link word.
        ref_count / set_ref_count: i32 = b32s1 @ 0;
        stretch_order / set_stretch_order: u16 = b16s1 @ 0;
        shrink_order / set_shrink_order: u16 = b16s0 @ 0;
        stretch / set_stretch: Scaled = b32s1 @ 2;
        shrink / set_shrink: Scaled = b32s1 @ 3;
    }
}

impl NodePtr<PenaltyNode> {
    fields! {
        penalty / set_penalty: i32 = b32s1 @ 1;
    }
}

// --- page-builder nodes ---

impl NodePtr<MarkNode> {
    fields! {
        mark_ptr / set_mark_ptr: MemPtr = b32s1 @ 1;
        mark_class / set_mark_class: i32 = b32s0 @ 1;
    }
}

impl NodePtr<AdjustNode> {
    fields! {
        adjust_ptr / set_adjust_ptr: MemPtr = b32s1 @ 1;
    }
}

impl NodePtr<InsNode> {
    fields! {
        float_cost / set_float_cost: i32 = b32s1 @ 1;
        ins_ptr / set_ins_ptr: MemPtr = b32s0 @ 4;
        /// The `\splittopskip` glue spec.
        split_top_ptr / set_split_top_ptr: NodePtr<GlueSpecNode> = b32s1 @ 4;
    }
}

impl NodePtr<PageInsNode> {
    #[inline]
    pub fn is_split_up(self, mem: &MemState) -> bool {
        mem.b16s1(self.idx.0) == 1
    }
    #[inline]
    pub fn set_split_up(self, mem: &mut MemState, split: bool) {
        mem.set_b16s1(self.idx.0, split as u16);
    }

    fields! {
        broken_ins / set_broken_ins: MemPtr = b32s0 @ 1;
        broken_ptr / set_broken_ptr: MemPtr = b32s1 @ 1;
        best_ins_ptr / set_best_ins_ptr: MemPtr = b32s0 @ 2;
        last_ins_ptr / set_last_ins_ptr: MemPtr = b32s1 @ 2;
        height / set_height: Scaled = b32s1 @ 3;
    }
}

impl NodePtr<UnsetNode> {
    fields! {
        glue_shrink / set_glue_shrink: Scaled = b32s1 @ 4;
        /// The shrink order.
        glue_sign / set_glue_sign: u16 = b16s1 @ 5;
        /// The stretch order.
        glue_order / set_glue_order: u16 = b16s0 @ 5;
        glue_stretch / set_glue_stretch: Scaled = b32s1 @ 6;
    }

    #[inline]
    pub fn into_hlist(self, mem: &mut MemState) -> NodePtr<HListNode> {
        retype(self, mem, NodeType::HList)
    }

    #[inline]
    pub fn into_vlist(self, mem: &mut MemState) -> NodePtr<VListNode> {
        retype(self, mem, NodeType::VList)
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
        Self::after_tab_skip(mem, self.link(mem))
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
    fields! {
        v_part / set_v_part: MemPtr = b32s1 @ 2;
        u_part / set_u_part: MemPtr = b32s1 @ 3;
        /// The command that ended the current entry of this column.
        extra_info / set_extra_info: i32 = b32s0 @ 5;
    }

    /// The column viewed as the head of its span list.
    #[inline]
    pub fn span_head(self) -> NodePtr<SpanNode> {
        NodePtr::at(self.idx)
    }

    /// Turns the record into an empty unset box of its final width.
    pub fn into_unset(self, mem: &mut MemState) -> NodePtr<UnsetNode> {
        let u: NodePtr<UnsetNode> = retype(self, mem, NodeType::Unset);
        u.set_subtype(mem, 0);
        u.set_height(mem, Scaled(0));
        u.set_depth(mem, Scaled(0));
        u.set_glue_order(mem, NORMAL);
        u.set_glue_sign(mem, NORMAL);
        u.set_glue_stretch(mem, Scaled(0));
        u.set_glue_shrink(mem, Scaled(0));
        u
    }
}

impl NodePtr<SpanNode> {
    fields! {
        /// Columns spanned beyond the first. `END_SPAN` holds a larger sentinel.
        span_count / set_span_count: i32 = b32s1 @ 0;
    }

    #[inline]
    pub fn next(self, mem: &MemState) -> NodePtr<SpanNode> {
        NodePtr::at(mem.b32s0(self.idx.0))
    }
    #[inline]
    pub fn set_next(self, mem: &mut MemState, v: NodePtr<SpanNode>) {
        mem.set_b32s0(self.idx.0, v.idx.0);
    }
}

impl NodePtr<AlignStackNode> {
    fields! {
        cur_align / set_cur_align: MemPtr = b32s0 @ 0;
        preamble / set_preamble: MemPtr = b32s0 @ 1;
        cur_span / set_cur_span: MemPtr = b32s1 @ 1;
        cur_loop / set_cur_loop: MemPtr = b32s1 @ 2;
        align_state / set_align_state: i32 = b32s1 @ 3;
        cur_head / set_cur_head: MemPtr = b32s0 @ 4;
        cur_tail / set_cur_tail: MemPtr = b32s1 @ 4;
        cur_pre_head / set_cur_pre_head: MemPtr = b32s0 @ 5;
        cur_pre_tail / set_cur_pre_tail: MemPtr = b32s1 @ 5;
    }
}

impl NodePtr<ParShapeNode> {
    fields! {
        lines / set_lines: i32 = b32s0 @ 0;
    }

    #[inline]
    pub fn indent(self, mem: &MemState, line: i32) -> Scaled {
        Scaled(mem.b32s1(self.idx.0 + 2 * line - 1))
    }
    #[inline]
    pub fn set_indent(self, mem: &mut MemState, line: i32, v: Scaled) {
        mem.set_b32s1(self.idx.0 + 2 * line - 1, v.0);
    }
    #[inline]
    pub fn line_width(self, mem: &MemState, line: i32) -> Scaled {
        Scaled(mem.b32s1(self.idx.0 + 2 * line))
    }
    #[inline]
    pub fn set_line_width(self, mem: &mut MemState, line: i32, v: Scaled) {
        mem.set_b32s1(self.idx.0 + 2 * line, v.0);
    }
}

impl NodePtr<PenaltiesNode> {
    fields! {
        count / set_count: i32 = b32s1 @ 1;
    }

    /// Stores half the node size, as a `\parshape` array does.
    #[inline]
    pub fn set_words(self, mem: &mut MemState, words: i32) {
        mem.set_b32s0(self.idx.0, words);
    }

    /// Entry `k`, counting from 1.
    #[inline]
    pub fn penalty(self, mem: &MemState, k: i32) -> i32 {
        mem.b32s1(self.idx.0 + 1 + k)
    }
    #[inline]
    pub fn set_penalty(self, mem: &mut MemState, k: i32, v: i32) {
        mem.set_b32s1(self.idx.0 + 1 + k, v);
    }
}

impl NodePtr<PseudoLineNode> {
    fields! {
        size / set_size: i32 = b32s0 @ 0;
    }

    /// The four characters in word `k`, counting from 1.
    #[inline]
    pub fn chars(self, mem: &MemState, k: i32) -> [u16; 4] {
        let w = self.idx.0 + k;
        [mem.b16s3(w), mem.b16s2(w), mem.b16s1(w), mem.b16s0(w)]
    }
    #[inline]
    pub fn set_chars(self, mem: &mut MemState, k: i32, chars: [u16; 4]) {
        let w = self.idx.0 + k;
        mem.set_b16s3(w, chars[0]);
        mem.set_b16s2(w, chars[1]);
        mem.set_b16s1(w, chars[2]);
        mem.set_b16s0(w, chars[3]);
    }
}

impl NodePtr<FreeBlock> {
    fields! {
        node_size / set_node_size: i32 = b32s0 @ 0;
        llink / set_llink: i32 = b32s0 @ 1;
        rlink / set_rlink: i32 = b32s1 @ 1;
    }

    #[inline]
    pub fn is_empty(self, mem: &MemState) -> bool {
        mem.b32s1(self.idx.0) == crate::consts::MAX_HALFWORD
    }
    #[inline]
    pub fn mark_empty(self, mem: &mut MemState) {
        mem.set_b32s1(self.idx.0, crate::consts::MAX_HALFWORD);
    }
}

impl NodePtr<ExprNode> {
    fields! {
        /// The value level of the suspended expression.
        level / set_level: u16 = b16s1 @ 0;
        /// The pending operators, as `4 * s + r`.
        state / set_state: u16 = b16s0 @ 0;
        e / set_e: i32 = b32s1 @ 1;
        t / set_t: i32 = b32s1 @ 2;
        n / set_n: i32 = b32s1 @ 3;
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
        WhatsitSubtype::from_raw(mem.b16s0(self.idx.0))
    }

    fields! {
        write_tokens / set_write_tokens: MemPtr = b32s1 @ 1;
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
        mem.b16s3(self.idx.0 + 4) as i32
    }
    #[inline]
    pub fn set_native_size(self, mem: &mut MemState, words: i32) {
        mem.set_b16s3(self.idx.0 + 4, words as u16);
    }

    #[inline]
    pub fn set_native_glyph_info_handle(self, mem: &mut MemState, handle: i32) {
        mem.set_b32s0(self.idx.0 + 5, handle);
        mem.set_b32s1(self.idx.0 + 5, 0);
    }

    /// The picture transform `[a, b, c, d, x, y]`.
    #[inline]
    pub fn pic_transform(self, mem: &MemState) -> [Scaled; 6] {
        std::array::from_fn(|k| {
            let q = self.idx.0 + 5 + k as i32 / 2;
            Scaled(if k % 2 == 0 {
                mem.b32s0(q)
            } else {
                mem.b32s1(q)
            })
        })
    }
    #[inline]
    pub fn set_pic_transform(self, mem: &mut MemState, t: [Scaled; 6]) {
        for (k, v) in (0..).zip(t) {
            let q = self.idx.0 + 5 + k / 2;
            if k % 2 == 0 {
                mem.set_b32s0(q, v.0);
            } else {
                mem.set_b32s1(q, v.0);
            }
        }
    }

    #[inline]
    pub fn pic_path(self, mem: &MemState) -> &[u8] {
        let start = (self.idx.0 + PIC_NODE_SIZE) as usize * 8;
        &mem.bytes()[start..start + self.pic_path_len(mem) as usize]
    }
    /// Stores `path` inline, the node must have room for it.
    #[inline]
    pub fn set_pic_path(self, mem: &mut MemState, path: &[u8]) {
        let start = self.idx.0 + PIC_NODE_SIZE;
        mem.journal_mark_range(start, path.len().div_ceil(8));
        let at = start as usize * 8;
        mem.bytes_mut()[at..at + path.len()].copy_from_slice(path);
    }

    /// The index in `mem.units()` where the text starts.
    #[inline]
    pub fn native_text_start(self) -> usize {
        (self.idx.0 + NATIVE_NODE_SIZE) as usize * 4
    }

    #[inline]
    pub fn native_text(self, mem: &MemState) -> &[u16] {
        let start = self.native_text_start();
        &mem.units()[start..start + self.native_length(mem) as usize]
    }

    /// Journals the words of the text before it is written.
    #[inline]
    pub fn mark_native_text(self, mem: &mut MemState) {
        let words = crate::state::native_text_words(self.native_length(mem) as i32);
        mem.journal_mark_range(self.idx.0 + NATIVE_NODE_SIZE, words);
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
        /// A count or dimension value.
        sa_int / set_sa_int: i32 = b32s1 @ 2;
    }

    #[inline]
    pub fn sa_type(self, mem: &MemState) -> u16 {
        self.sa_index(mem) / 64
    }

    /// Child or mark slot `i`, packed two per word from +1.
    #[inline]
    pub fn child(self, mem: &MemState, i: i32) -> MemPtr {
        let w = self.idx.0 + i / 2 + 1;
        MemPtr::from_raw(if i % 2 != 0 {
            mem.b32s1(w)
        } else {
            mem.b32s0(w)
        })
    }
    #[inline]
    pub fn set_child(self, mem: &mut MemState, i: i32, v: MemPtr) {
        let w = self.idx.0 + i / 2 + 1;
        if i % 2 != 0 {
            mem.set_b32s1(w, v.0);
        } else {
            mem.set_b32s0(w, v.0);
        }
    }

    /// The parent index node, non-null below the root.
    #[inline]
    pub fn parent(self, mem: &MemState) -> NodePtr<SaNode> {
        Self::at(self.link(mem))
    }
}

// --- the conditional stack ---

impl NodePtr<CondNode> {
    fields! {
        if_limit / set_if_limit: u16 = b16s1 @ 0;
        cur_if / set_cur_if: u16 = b16s0 @ 0;
        if_line / set_if_line: i32 = b32s1 @ 1;
    }

    /// The saved `(cur_if, if_line, if_limit)`.
    #[inline]
    pub fn saved_state(self, mem: &MemState) -> (i16, i32, u8) {
        (
            self.cur_if(mem) as i16,
            self.if_line(mem),
            self.if_limit(mem) as u8,
        )
    }
}

// --- shipout movement records ---

impl NodePtr<MovementNode> {
    fields! {
        /// Whether the movement may be reused, and as which command.
        state / set_state: i32 = b32s0 @ 0;
        distance / set_distance: Scaled = b32s1 @ 1;
        /// The DVI offset where the movement was emitted.
        location / set_location: i32 = b32s1 @ 2;
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

impl NodePtr<DeltaNode> {
    fields! {
        dwidth / set_dwidth: Scaled = b32s1 @ 1;
        dstretch0 / set_dstretch0: Scaled = b32s1 @ 2;
        dstretch1 / set_dstretch1: Scaled = b32s1 @ 3;
        dstretch2 / set_dstretch2: Scaled = b32s1 @ 4;
        dstretch3 / set_dstretch3: Scaled = b32s1 @ 5;
        dshrink / set_dshrink: Scaled = b32s1 @ 6;
    }
}

impl NodePtr<PassiveNode> {
    fields! {
        cur_break / set_cur_break: MemPtr = b32s1 @ 1;
        next_break / set_next_break: MemPtr = b32s0 @ 1;
    }

    /// The previous break on the best path, reversed into `next_break` later.
    #[inline]
    pub fn set_prev_break(self, mem: &mut MemState, v: MemPtr) {
        self.set_next_break(mem, v);
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
            hi_mem_min: 1_000_000,
            ..Default::default()
        }
    }

    /// An arena whose free list is the single block `[10, 30)`.
    fn arena_with_free_block() -> MemState {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);
        mem
    }

    fn at<K: NodeKind>(mem: &mut MemState, p: i32, ty: Option<NodeType>) -> NodePtr<K> {
        if let Some(ty) = ty {
            mem.set_b16s1(p, ty.as_raw());
        }
        NodePtr::from_idx_unchecked(MemIdx::new(p))
    }

    #[test]
    fn handles_are_zero_cost() {
        assert_eq!(size_of::<NodePtr<HListNode>>(), size_of::<i32>());
        assert_eq!(size_of::<NodePtr<GlueSpecNode>>(), size_of::<i32>());
        assert_eq!(size_of::<NodePtr<Noad>>(), size_of::<i32>());
    }

    #[test]
    fn alloc_stamps_the_type_that_classify_reads_back() {
        let mut mem = arena_with_free_block();
        let h = NodePtr::<HListNode>::alloc(&mut mem).unwrap();
        assert!(matches!(mem.classify(h.idx()), Ok(AnyNode::HList(_))));
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
    fn headerless_records_alloc_without_stamping() {
        let mut mem = arena_with_free_block();
        let spec = NodePtr::<GlueSpecNode>::alloc(&mut mem).unwrap();
        spec.set_width(&mut mem, Scaled(0x1234));
        assert_eq!(spec.width(&mem), Scaled(0x1234));
        let item = NodePtr::<LigItem>::alloc(&mut mem).unwrap();
        item.set_character(&mut mem, 45);
        item.set_component(&mut mem, MemPtr(34));
        assert_eq!(mem.b16s0(item.idx().0), 45);
        assert_eq!(mem.b32s1(item.idx().0 + 1), 34);
    }

    #[test]
    fn sync_trailer_is_the_last_word() {
        let mut mem = fresh_mem(200);
        let b: NodePtr<HListNode> = at(&mut mem, 100, Some(NodeType::HList));
        mem.set_b32s0(100 + BOX_NODE_SIZE - 1, 42);
        mem.set_b32s1(100 + BOX_NODE_SIZE - 1, 7);
        assert_eq!(b.sync_tag(&mem), 42);
        assert_eq!(AnyBox::HList(b).sync_line(&mem), 7);

        let k: NodePtr<KernNode> = at(&mut mem, 100, Some(NodeType::Kern));
        mem.set_b32s0(100 + MEDIUM_NODE_SIZE - 1, 9);
        mem.set_b32s1(100 + MEDIUM_NODE_SIZE - 1, 3);
        assert_eq!(k.sync_tag(&mem), 9);
        assert_eq!(k.sync_line(&mem), 3);
    }

    #[test]
    fn box_fields_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let b: NodePtr<HListNode> = at(&mut mem, 100, Some(NodeType::HList));
        b.set_width(&mut mem, Scaled(0x1111));
        b.set_depth(&mut mem, Scaled(0x2222));
        b.set_height(&mut mem, Scaled(0x3333));
        b.set_shift_amount(&mut mem, Scaled(0x4444));
        b.set_list_ptr(&mut mem, MemPtr(150));
        b.set_glue_sign(&mut mem, GlueSign::Shrinking);
        b.set_glue_order(&mut mem, 1);
        b.set_glue_set(&mut mem, 1.5);
        assert_eq!(b.glue_sign(&mem), GlueSign::Shrinking);
        assert_eq!(b.glue_set(&mem), 1.5);
        assert_eq!(mem.b32s1(101), 0x1111);
        assert_eq!(mem.b32s1(102), 0x2222);
        assert_eq!(mem.b32s1(103), 0x3333);
        assert_eq!(mem.b32s1(104), 0x4444);
        assert_eq!(mem.b32s1(105), 150);
        assert_eq!(mem.b16s1(105), 2);
        assert_eq!(mem.b16s0(105), 1);

        b.set_list_ptr(&mut mem, MemPtr::NULL);
        assert!(b.list_ptr(&mem).as_idx().is_none());
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
            mem.set_b16s1(p, ty.as_raw());
        }
        mem.set_b16s1(180, 35);
        let c = |p| mem.classify(MemIdx::new(p));
        use AnyNode::*;
        assert!(matches!(c(40), Ok(HList(_))));
        assert!(matches!(c(50), Ok(VList(_))));
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
        assert_eq!(c(40).unwrap().idx().0, 40);

        assert!(matches!(
            mem.classify_box(MemIdx(50)),
            Ok(Some(AnyBox::VList(_)))
        ));
        assert!(matches!(mem.classify_box(MemIdx(70)), Ok(None)));
        assert!(mem.classify_box(MemIdx(180)).is_err());
    }

    #[test]
    fn rule_dimensions_model_the_running_sentinel() {
        let mut mem = fresh_mem(200);
        let r: NodePtr<RuleNode> = at(&mut mem, 100, Some(NodeType::Rule));
        r.set_width(&mut mem, RuleDimen::Running);
        r.set_depth(&mut mem, RuleDimen::Set(Scaled(0)));
        r.set_height(&mut mem, RuleDimen::Set(Scaled(0x2222)));
        assert_eq!(r.width(&mem), RuleDimen::Running);
        assert_eq!(r.depth(&mem), RuleDimen::Set(Scaled(0)));
        assert_eq!(mem.b32s1(101), NULL_FLAG);
        assert_eq!(mem.b32s1(103), 0x2222);
    }

    #[test]
    fn small_node_fields_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let lig: NodePtr<LigatureNode> = at(&mut mem, 100, Some(NodeType::Ligature));
        lig.set_font(&mut mem, FontId(77));
        lig.set_character(&mut mem, 0x1234);
        assert_eq!(lig.font(&mem), FontId(77));
        assert_eq!(mem.b16s1(101), 77);
        assert_eq!(mem.b16s0(101), 0x1234);
        lig.set_lig_ptr(&mut mem, MemPtr(150));
        assert_eq!(mem.b32s1(101), 150);

        let disc: NodePtr<DiscNode> = at(&mut mem, 100, Some(NodeType::Disc));
        disc.set_pre_break(&mut mem, MemPtr(150));
        disc.set_post_break(&mut mem, MemPtr(160));
        assert_eq!(mem.b32s0(101), 150);
        assert_eq!(mem.b32s1(101), 160);

        let math: NodePtr<MathNode> = at(&mut mem, 100, Some(NodeType::Math));
        math.set_subtype(&mut mem, 3);
        math.set_width(&mut mem, Scaled(0x1234));
        assert_eq!(mem.b16s0(100), 3);
        assert_eq!(mem.b32s1(101), 0x1234);

        let style: NodePtr<StyleNode> = at(&mut mem, 100, Some(NodeType::Style));
        style.clear_payload(&mut mem);
        assert_eq!(mem.b32s1(101), 0);
        assert_eq!(mem.b32s1(102), 0);

        let pen: NodePtr<PenaltyNode> = at(&mut mem, 100, Some(NodeType::Penalty));
        pen.set_penalty(&mut mem, -10000);
        assert_eq!(mem.b32s1(101), -10000);

        let mark: NodePtr<MarkNode> = at(&mut mem, 100, Some(NodeType::Mark));
        mark.set_mark_ptr(&mut mem, MemPtr(77));
        mark.set_mark_class(&mut mem, 3);
        assert_eq!(mem.b32s1(101), 77);
        assert_eq!(mem.b32s0(101), 3);

        let adj: NodePtr<AdjustNode> = at(&mut mem, 100, Some(NodeType::Adjust));
        adj.set_adjust_ptr(&mut mem, MemPtr(88));
        assert_eq!(mem.b32s1(101), 88);

        let ins: NodePtr<InsNode> = at(&mut mem, 100, Some(NodeType::Ins));
        ins.set_ins_ptr(&mut mem, MemPtr(55));
        ins.set_split_top_ptr(&mut mem, NodePtr::from_glue_ptr(MemPtr(66)));
        assert_eq!(mem.b32s0(104), 55);
        assert_eq!(mem.b32s1(104), 66);
    }

    #[test]
    fn choice_branches_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let c: NodePtr<ChoiceNode> = at(&mut mem, 100, Some(NodeType::Choice));
        c.set_display_mlist(&mut mem, MemPtr(11));
        c.set_text_mlist(&mut mem, MemPtr(22));
        c.set_script_mlist(&mut mem, MemPtr(33));
        c.set_script_script_mlist(&mut mem, MemPtr(44));
        assert_eq!(mem.b32s0(101), 11);
        assert_eq!(mem.b32s1(101), 22);
        assert_eq!(mem.b32s0(102), 33);
        assert_eq!(mem.b32s1(102), 44);
    }

    #[test]
    fn alignment_records_walk_the_preamble_and_become_unset() {
        let mut mem = fresh_mem(200);
        let (head, g1, col, g2) = (MemPtr(10), MemPtr(20), MemPtr(100), MemPtr(30));
        head.set_link(&mut mem, g1);
        g1.set_link(&mut mem, col);
        col.set_link(&mut mem, g2);
        g2.set_link(&mut mem, MemPtr::NULL);
        let rec = NodePtr::<AlignRecord>::after_tab_skip(&mem, g1).unwrap();
        assert_eq!(rec.ptr(), col);
        assert_eq!(rec.tab_skip(&mem).ptr(), g2);
        assert_eq!(NodePtr::<GlueNode>::tab_skip_after(&mem, head).ptr(), g1);
        assert!(rec.next_column(&mem).is_none());

        rec.set_v_part(&mut mem, MemPtr(0x22));
        rec.set_u_part(&mut mem, MemPtr(0x33));
        rec.set_extra_info(&mut mem, 0x55);
        rec.span_head().set_next(&mut mem, NodePtr::at(40));
        assert_eq!((mem.b32s1(102), mem.b32s1(103)), (0x22, 0x33));
        assert_eq!((mem.b32s0(105), mem.b32s0(100)), (0x55, 40));

        let u = rec.into_unset(&mut mem);
        assert_eq!(mem.b16s1(100), NodeType::Unset.as_raw());
        assert_eq!((u.height(&mem), u.depth(&mem)), (Scaled(0), Scaled(0)));
        assert_eq!((mem.b16s0(100), mem.b32s1(104), mem.b32s1(106)), (0, 0, 0));
        assert_eq!(u.link(&mem), g2);

        let span: NodePtr<SpanNode> = NodePtr::at(40);
        span.set_span_count(&mut mem, 3);
        span.set_width(&mut mem, Scaled(0x66));
        assert_eq!((mem.b32s1(40), mem.b32s1(41)), (3, 0x66));
    }

    #[test]
    fn unset_and_whatsit_fields_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let u: NodePtr<UnsetNode> = at(&mut mem, 100, Some(NodeType::Unset));
        mem.set_b32s1(101, 0x111);
        mem.set_b32s1(103, 0x333);
        u.set_list_ptr(&mut mem, MemPtr(0x444));
        assert_eq!(u.width(&mem), Scaled(0x111));
        assert_eq!(u.height(&mem), Scaled(0x333));
        assert_eq!(mem.b32s1(105), 0x444);

        let w: NodePtr<WhatsitNode> = at(&mut mem, 100, Some(NodeType::Whatsit));
        w.set_subtype(&mut mem, 40);
        assert_eq!(w.whatsit_subtype(&mem), Some(WhatsitSubtype::NativeWord));
        mem.set_b16s3(104, 6);
        mem.set_b16s2(104, 7);
        mem.set_b16s1(104, 9);
        assert_eq!(w.native_size(&mem), 6);
        assert_eq!(w.native_font(&mem), FontId(7));
        assert_eq!(w.native_length(&mem), 9);
        assert_eq!(w.pic_path_len(&mem), 9);
        w.set_native_glyph_count(&mut mem, 3);
        assert_eq!(mem.b16s0(104), 3);
        w.set_native_glyph_info_handle(&mut mem, 7);
        assert_eq!(w.native_glyph_info_handle(&mem), 7);
        assert_eq!(mem.b32s1(105), 0);

        mem.set_b32s1(101, 0x55);
        assert_eq!(w.write_tokens(&mem), MemPtr(0x55));
        assert_eq!(w.what_lang(&mem), 0x55);
        mem.set_b16s1(101, 4);
        mem.set_b16s0(101, 5);
        assert_eq!(w.what_lhm(&mem), 4);
        assert_eq!(w.what_rhm(&mem), 5);
    }

    #[test]
    fn sa_children_pack_two_per_word() {
        let mut mem = fresh_mem(200);
        let node: NodePtr<SaNode> = at(&mut mem, 100, None);
        for i in 0..6 {
            node.set_child(&mut mem, i, MemPtr(1000 + i));
        }
        for i in 0..6 {
            assert_eq!(node.child(&mem, i), MemPtr(1000 + i));
        }
        assert_eq!(mem.b32s0(101), 1000);
        assert_eq!(mem.b32s1(101), 1001);
        assert_eq!(mem.b32s1(103), 1005);
        node.set_ref_count(&mut mem, 7);
        assert_eq!(mem.b16s0(100), 7);
    }

    #[test]
    fn glue_node_and_spec_fields_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let g: NodePtr<GlueNode> = at(&mut mem, 100, Some(NodeType::Glue));
        g.set_subtype(&mut mem, 7);
        g.set_glue_ptr(&mut mem, NodePtr::from_glue_ptr(MemPtr(150)));
        g.set_leader_ptr(&mut mem, MemPtr(160));
        assert_eq!(mem.b16s0(100), 7);
        assert_eq!(mem.b32s0(101), 150);
        assert_eq!(mem.b32s1(101), 160);
        assert_eq!(g.glue_ptr(&mem).idx(), MemIdx(150));

        let s: NodePtr<GlueSpecNode> = at(&mut mem, 150, None);
        s.set_ref_count(&mut mem, 42);
        s.set_stretch_order(&mut mem, 2);
        s.set_shrink_order(&mut mem, 1);
        s.set_width(&mut mem, Scaled(0x4444));
        s.set_stretch(&mut mem, Scaled(0x5555));
        s.set_shrink(&mut mem, Scaled(0x6666));
        assert_eq!(mem.b32s1(150), 42);
        assert_eq!(mem.b16s1(150), 2);
        assert_eq!(mem.b16s0(150), 1);
        assert_eq!(mem.b32s1(151), 0x4444);
        assert_eq!(mem.b32s1(152), 0x5555);
        assert_eq!(mem.b32s1(153), 0x6666);
    }

    #[test]
    fn noad_fields_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let n: NodePtr<Noad> = at(&mut mem, 100, Some(NodeType::OrdNoad));
        n.nucleus().set_math_type(&mut mem, 1);
        n.nucleus().set_info(&mut mem, MemPtr(123));
        assert_eq!(mem.b32s1(101), 1);
        assert_eq!(mem.b32s0(101), 123);
        n.nucleus().set_character(&mut mem, 45);
        n.nucleus().set_plane_and_fam(&mut mem, 2 + 3 * 256);
        assert_eq!(n.nucleus().fam(&mem), 2);
        assert_eq!(n.nucleus().full_char(&mem), 45 + 3 * 65_536);
        n.accent_chr().set_character(&mut mem, 48);
        n.accent_chr().set_plane_and_fam(&mut mem, 5);
        assert_eq!(mem.b16s0(104), 48);
        assert_eq!(mem.b16s1(104), 5);
        n.right_delimiter().set_small(&mut mem, 1 + 256, 7);
        n.right_delimiter().set_large(&mut mem, 2, 9);
        assert_eq!(n.right_delimiter().small(&mem), (1, 7 + 65_536));
        assert_eq!(n.right_delimiter().large(&mem), (2, 9));
        assert_eq!(mem.b16s3(105), 1 + 256);
        assert_eq!(mem.b16s0(105), 9);
    }

    #[test]
    fn line_breaker_records_land_in_their_words() {
        let mut mem = fresh_mem(200);
        let a: NodePtr<ActiveNode> = at(&mut mem, 100, None);
        a.set_break_type(&mut mem, 1);
        a.set_fitness(&mut mem, 2);
        a.set_break_node(&mut mem, MemPtr(150));
        a.set_line_number(&mut mem, 7);
        a.set_total_demerits(&mut mem, 0x5151);
        a.set_shortfall(&mut mem, Scaled(-0x20));
        a.set_glue(&mut mem, Scaled(0x30));
        assert_eq!(mem.b16s1(100), 1);
        assert_eq!(mem.b16s0(100), 2);
        assert_eq!(mem.b32s1(101), 150);
        assert_eq!(mem.b32s0(101), 7);
        assert_eq!(mem.b32s1(102), 0x5151);
        assert_eq!(mem.b32s1(103), -0x20);
        assert_eq!(mem.b32s1(104), 0x30);

        let d: NodePtr<DeltaNode> = at(&mut mem, 100, None);
        d.set_dwidth(&mut mem, Scaled(0x11));
        d.set_dstretch3(&mut mem, Scaled(0x55));
        d.set_dshrink(&mut mem, Scaled(-0x66));
        assert_eq!(mem.b32s1(101), 0x11);
        assert_eq!(mem.b32s1(105), 0x55);
        assert_eq!(mem.b32s1(106), -0x66);

        let pn: NodePtr<PassiveNode> = at(&mut mem, 100, None);
        pn.set_link(&mut mem, MemPtr(90));
        pn.set_cur_break(&mut mem, MemPtr(150));
        pn.set_prev_break(&mut mem, MemPtr(80));
        assert_eq!(pn.next_break(&mem), MemPtr(80));
        assert_eq!(mem.b32s1(100), 90);
        assert_eq!(mem.b32s1(101), 150);
        pn.set_next_break(&mut mem, MemPtr(70));
        assert_eq!(mem.b32s0(101), 70);
    }
}
