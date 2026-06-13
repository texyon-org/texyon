//! Typed node handles over the flat `mem` arena (`plan/idiom/01c` §3;
//! `plan/03-memory-and-nodes.md`).
//!
//! [`NodePtr<K>`] is a `#[repr(transparent)]` wrapper over a [`MemIdx`] plus
//! a compile-time-only `PhantomData<K>` kind marker. It is `Copy`, holds no
//! borrow of the arena, and — after inlining — compiles to the same
//! load/store as the raw `i32` index (the zero-cost gate, `01c` §4). Field
//! access is done by passing the [`MemState`] arena as a context parameter
//! (`p.width(mem)` / `p.set_width(mem, w)`); there are no view structs
//! (`03` §"Avoidance of View Structs").
//!
//! This is the first family layer (`01c` §1 build order: the shared header
//! word + the box family). It is built before its consumers — the Phase-C
//! type-as-you-thread slices (`01`-§4a) — so it is `#[allow(dead_code)]` in
//! `lib.rs` until a threading slice adopts it.
//!
//! ## Field offsets (verified against `xetex/xetex-xetexd.h:211–218`)
//!
//! | field          | offset.subfield | type     |
//! |----------------|-----------------|----------|
//! | `width`        | +1.b32.s1       | `Scaled` |
//! | `depth`        | +2.b32.s1       | `Scaled` |
//! | `height`       | +3.b32.s1       | `Scaled` |
//! | `shift_amount` | +4.b32.s1       | `Scaled` |
//! | `list_ptr`     | +5.b32.s1       | `MemPtr` (nullable: empty box → `TEX_NULL`) |
//! | `glue_sign`    | +5.b16.s1       | `u16`    |
//! | `glue_order`   | +5.b16.s0       | `u16`    |
//! | `glue_set`     | +6.gr           | `f64` (the glue ratio) |
//!
//! `BOX_NODE_SIZE` = 8 words (fixed). Field reads/writes go through the
//! audited `MemState` word accessors so the `MemoryWord` union projection
//! stays confined there (one exception: the `glue_set` read uses
//! `MemState::word(..).gr`, documented below).

use std::marker::PhantomData;

use crate::consts::{
    BOX_NODE_SIZE, DISC_NODE, GLUE_NODE, GLUE_SPEC_SIZE, HLIST_NODE, KERN_NODE, LIGATURE_NODE,
    MARGIN_KERN_NODE, MARGIN_KERN_NODE_SIZE, MATH_NODE, MEDIUM_NODE_SIZE, NULL_FLAG, PENALTY_NODE,
    RULE_NODE, RULE_NODE_SIZE, SMALL_NODE_SIZE,
};
use crate::mem::{free_node, get_node, ALLOC_FAILED};
use crate::state::MemState;
use crate::units::{FontId, InvalidNodeType, MemIdx, MemPtr, NodeType, Scaled};

/// Marker trait for node kinds. Implemented only by the zero-sized kind
/// markers below; never constructed.
pub trait NodeKind {}

/// A typed handle to a node of kind `K` in the `mem` arena.
///
/// Same runtime payload as [`MemIdx`]; the kind is compile-time only.
#[repr(transparent)]
pub struct NodePtr<K: NodeKind> {
    idx: MemIdx,
    _kind: PhantomData<fn() -> K>,
}

// Manual `Copy`/`Clone`/`PartialEq`/`Eq` so they don't require `K: Copy`
// (the marker `K` is never instantiated).
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
    /// Wrap an index as a `NodePtr<K>` **without** checking that it really
    /// references a `K` node.
    ///
    /// # Safety
    /// The caller must have classified `idx` as a `K`-kind node (e.g. via
    /// [`MemState::classify_box`] or after a `NODE_type` branch mirroring
    /// the C source). Constructing the wrong kind is not memory-unsafe (the
    /// field methods still index in bounds) but it is a *parity* bug — it
    /// reads a field that does not exist on the real node.
    #[inline]
    pub unsafe fn from_idx_unchecked(idx: MemIdx) -> Self {
        NodePtr {
            idx,
            _kind: PhantomData,
        }
    }

    /// The underlying arena index.
    #[inline]
    pub fn idx(self) -> MemIdx {
        self.idx
    }
}

// --- the box family: hlist and vlist nodes (NODE_type 0 and 1) ---

/// `HLIST_NODE` (`NODE_type` = 0).
pub enum HListNode {}
/// `VLIST_NODE` (`NODE_type` = 1).
pub enum VListNode {}

impl NodeKind for HListNode {}
impl NodeKind for VListNode {}

/// A rule dimension: either TeX's running-dimension sentinel (`NULL_FLAG`) or
/// a concrete scaled length.
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
}

/// `RULE_NODE` (`NODE_type` = 2): a rectangular rule with running dimensions.
pub enum RuleNode {}

impl NodeKind for RuleNode {}

// --- capability traits: shared field layouts (compile-time only) ---

/// Has the `width`/`depth`/`height`/`shift_amount` scaled dimensions at
/// offsets +1..+4 (`03` `HasDimensions`).
pub trait HasDimensions: NodeKind {}
/// Has the `list_ptr` child-list field at +5.b32.s1.
pub trait HasListPtr: NodeKind {}
/// Has the glue-set triple (`glue_sign` +5.b16.s1, `glue_order` +5.b16.s0,
/// `glue_set` +6.gr) — the box glue-setting fields.
pub trait HasGlueSet: NodeKind {}
/// Has the list-node header word (`NODE_type` in 0.b16.s1, subtype in
/// 0.b16.s0). Glue specs deliberately do not implement this: their header word
/// holds ref_count/stretch_order/shrink_order instead of a node type.
pub trait HasNodeHeader: NodeKind {}

impl HasDimensions for HListNode {}
impl HasDimensions for VListNode {}
impl HasListPtr for HListNode {}
impl HasListPtr for VListNode {}
impl HasGlueSet for HListNode {}
impl HasGlueSet for VListNode {}
impl HasNodeHeader for HListNode {}
impl HasNodeHeader for VListNode {}
impl HasNodeHeader for RuleNode {}
impl HasNodeHeader for DiscNode {}
impl HasNodeHeader for LigatureNode {}
impl HasNodeHeader for MathNode {}
impl HasNodeHeader for StyleNode {}
impl HasNodeHeader for ChoiceNode {}
impl HasNodeHeader for GlueNode {}
impl HasNodeHeader for KernNode {}
impl HasNodeHeader for PenaltyNode {}
impl HasNodeHeader for Noad {}
impl HasNodeHeader for MarginKernNode {}

impl<K: HasNodeHeader> NodePtr<K> {
    /// Shared list-node subtype = `mem[p].b16.s0`.
    #[inline]
    pub fn subtype(self, mem: &MemState) -> u16 {
        mem.b16s0(self.idx.0)
    }
    #[inline]
    pub fn set_subtype(self, mem: &mut MemState, v: u16) {
        mem.set_b16s0(self.idx.0, v);
    }

    /// Link pointer to the next list element = `mem[p].b32.s1`.
    #[inline]
    pub fn link(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.0))
    }
    #[inline]
    pub fn set_link(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.idx.0, q.0);
    }
}

impl NodePtr<HListNode> {
    /// Allocate a fresh hlist box node (`BOX_NODE_SIZE` words), type-stamped
    /// with `NODE_type = HLIST_NODE`, and return a typed handle to it — or
    /// `None` if the arena is exhausted.
    ///
    /// This is the box-family form of the typed allocation boundary (`01c`
    /// §8): it wraps TeX's free-list allocator, writes the node type tag, and
    /// confines the family's `from_idx_unchecked` construction to this single
    /// audited site. The caller fills the remaining box fields.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<HListNode>> {
        let p = get_node(mem, BOX_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        // type-stamp: an hlist node carries NODE_type = HLIST_NODE.
        mem.set_b16s1(p, HLIST_NODE);
        // SAFETY: `p` is a freshly allocated `BOX_NODE_SIZE` node just stamped
        // HLIST_NODE; this is the family's audited construction site.
        Some(unsafe { NodePtr::<HListNode>::from_idx_unchecked(MemIdx::new(p)) })
    }
}

impl NodePtr<VListNode> {
    /// Allocate a fresh vlist box node (`BOX_NODE_SIZE` words), type-stamped
    /// with `NODE_type = VLIST_NODE`, and return a typed handle to it — or
    /// `None` if the arena is exhausted.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<VListNode>> {
        let p = get_node(mem, BOX_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, NodeType::VList.as_raw());
        // SAFETY: `p` is a freshly allocated `BOX_NODE_SIZE` node just stamped
        // VLIST_NODE; this is the family's audited construction site.
        Some(unsafe { NodePtr::<VListNode>::from_idx_unchecked(MemIdx::new(p)) })
    }
}

impl<K: HasDimensions> NodePtr<K> {
    /// `BOX_width(p)` = `mem[(p) + 1].b32.s1`.
    #[inline]
    pub fn width(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_width(self, mem: &mut MemState, w: Scaled) {
        mem.set_b32s1(self.idx.offset(1).0, w.0);
    }

    /// `BOX_depth(p)` = `mem[(p) + 2].b32.s1`.
    #[inline]
    pub fn depth(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(2).0))
    }
    #[inline]
    pub fn set_depth(self, mem: &mut MemState, w: Scaled) {
        mem.set_b32s1(self.idx.offset(2).0, w.0);
    }

    /// `BOX_height(p)` = `mem[(p) + 3].b32.s1`.
    #[inline]
    pub fn height(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(3).0))
    }
    #[inline]
    pub fn set_height(self, mem: &mut MemState, w: Scaled) {
        mem.set_b32s1(self.idx.offset(3).0, w.0);
    }

    /// `BOX_shift_amount(p)` = `mem[(p) + 4].b32.s1`.
    #[inline]
    pub fn shift_amount(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(4).0))
    }
    #[inline]
    pub fn set_shift_amount(self, mem: &mut MemState, w: Scaled) {
        mem.set_b32s1(self.idx.offset(4).0, w.0);
    }
}

impl<K: HasListPtr> NodePtr<K> {
    /// `BOX_list_ptr(p)` = `mem[(p) + 5].b32.s1` — the child list head, or
    /// `TEX_NULL` for an empty box. Returns a nullable [`MemPtr`].
    #[inline]
    pub fn list_ptr(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(5).0))
    }
    #[inline]
    pub fn set_list_ptr(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.idx.offset(5).0, q.0);
    }
}

impl<K: HasGlueSet> NodePtr<K> {
    /// `BOX_glue_sign(p)` = `mem[(p) + 5].b16.s1`.
    #[inline]
    pub fn glue_sign(self, mem: &MemState) -> u16 {
        mem.b16s1(self.idx.offset(5).0)
    }
    #[inline]
    pub fn set_glue_sign(self, mem: &mut MemState, v: u16) {
        mem.set_b16s1(self.idx.offset(5).0, v);
    }

    /// `BOX_glue_order(p)` = `mem[(p) + 5].b16.s0`.
    #[inline]
    pub fn glue_order(self, mem: &MemState) -> u16 {
        mem.b16s0(self.idx.offset(5).0)
    }
    #[inline]
    pub fn set_glue_order(self, mem: &mut MemState, v: u16) {
        mem.set_b16s0(self.idx.offset(5).0, v);
    }

    /// `BOX_glue_set(p)` = `mem[(p) + 6].gr` (the `f64` glue ratio).
    #[inline]
    pub fn glue_set(self, mem: &MemState) -> f64 {
        // SAFETY: `MemoryWord` union projection of the `.gr` (f64) field;
        // `MemState::word` is the audited bounds-checked word read. The box
        // glue_set word is always written as `.gr` by the packers, so this
        // reads the field last written (`03` union-safety contract).
        unsafe { mem.word(self.idx.offset(6).0).gr }
    }
    #[inline]
    pub fn set_glue_set(self, mem: &mut MemState, v: f64) {
        mem.set_gr(self.idx.offset(6).0, v);
    }
}

/// A classified box: either an hlist or a vlist node.
#[derive(Copy, Clone, Debug)]
pub enum AnyBox {
    HList(NodePtr<HListNode>),
    VList(NodePtr<VListNode>),
}

impl AnyBox {
    #[inline]
    pub fn idx(self) -> MemIdx {
        match self {
            AnyBox::HList(p) => p.idx(),
            AnyBox::VList(p) => p.idx(),
        }
    }

    #[inline]
    pub fn width(self, mem: &MemState) -> Scaled {
        match self {
            AnyBox::HList(p) => p.width(mem),
            AnyBox::VList(p) => p.width(mem),
        }
    }

    #[inline]
    pub fn depth(self, mem: &MemState) -> Scaled {
        match self {
            AnyBox::HList(p) => p.depth(mem),
            AnyBox::VList(p) => p.depth(mem),
        }
    }

    #[inline]
    pub fn height(self, mem: &MemState) -> Scaled {
        match self {
            AnyBox::HList(p) => p.height(mem),
            AnyBox::VList(p) => p.height(mem),
        }
    }

    #[inline]
    pub fn set_width(self, mem: &mut MemState, w: Scaled) {
        match self {
            AnyBox::HList(p) => p.set_width(mem, w),
            AnyBox::VList(p) => p.set_width(mem, w),
        }
    }

    #[inline]
    pub fn set_depth(self, mem: &mut MemState, d: Scaled) {
        match self {
            AnyBox::HList(p) => p.set_depth(mem, d),
            AnyBox::VList(p) => p.set_depth(mem, d),
        }
    }

    #[inline]
    pub fn set_height(self, mem: &mut MemState, h: Scaled) {
        match self {
            AnyBox::HList(p) => p.set_height(mem, h),
            AnyBox::VList(p) => p.set_height(mem, h),
        }
    }

    #[inline]
    pub fn shift_amount(self, mem: &MemState) -> Scaled {
        match self {
            AnyBox::HList(p) => p.shift_amount(mem),
            AnyBox::VList(p) => p.shift_amount(mem),
        }
    }

    #[inline]
    pub fn set_shift_amount(self, mem: &mut MemState, s: Scaled) {
        match self {
            AnyBox::HList(p) => p.set_shift_amount(mem, s),
            AnyBox::VList(p) => p.set_shift_amount(mem, s),
        }
    }

    #[inline]
    pub fn list_ptr(self, mem: &MemState) -> MemPtr {
        match self {
            AnyBox::HList(p) => p.list_ptr(mem),
            AnyBox::VList(p) => p.list_ptr(mem),
        }
    }

    #[inline]
    pub fn set_list_ptr(self, mem: &mut MemState, q: MemPtr) {
        match self {
            AnyBox::HList(p) => p.set_list_ptr(mem, q),
            AnyBox::VList(p) => p.set_list_ptr(mem, q),
        }
    }

    #[inline]
    pub fn link(self, mem: &MemState) -> MemPtr {
        match self {
            AnyBox::HList(p) => p.link(mem),
            AnyBox::VList(p) => p.link(mem),
        }
    }

    #[inline]
    pub fn set_link(self, mem: &mut MemState, q: MemPtr) {
        match self {
            AnyBox::HList(p) => p.set_link(mem, q),
            AnyBox::VList(p) => p.set_link(mem, q),
        }
    }
}

/// A fully classified lo-mem node: the typed handle for every family whose
/// `NodePtr<K>` layer is built, or a raw [`MemIdx`] for families not yet typed
/// (`01c` §1: native/whatsit, unset, ins/mark/adjust). This is the single
/// dispatch surface the plan calls for (`01c` §3, "classification happens
/// ONCE"): a consumer calls [`MemState::classify`] once at the list-walk
/// boundary and matches on the result, so the per-arm `from_idx_unchecked` that
/// a raw `NODE_type` match forces is gone — each arm carries its static type.
///
/// Context-disambiguated codes are classified by their **node-list** meaning:
/// type 2 → [`AnyNode::Rule`] (not the linebreak-active-list `DELTA_NODE`) and
/// type 14 → [`AnyNode::Style`] (not `EDGE_NODE`). Walkers of those specialized
/// lists carry their own context and do not call `classify`.
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
    Whatsit(MemIdx),
    Math(NodePtr<MathNode>),
    Glue(NodePtr<GlueNode>),
    Kern(NodePtr<KernNode>),
    Penalty(NodePtr<PenaltyNode>),
    Unset(NodePtr<UnsetNode>),
    Style(NodePtr<StyleNode>),
    Choice(NodePtr<ChoiceNode>),
    /// Any noad (`NODE_type` 16–31); refine with [`NodePtr::<Noad>::noad_type`].
    Noad(NodePtr<Noad>),
    MarginKern(NodePtr<MarginKernNode>),
}

impl AnyNode {
    /// The arena index of the classified node, regardless of family.
    #[inline]
    pub fn idx(self) -> MemIdx {
        match self {
            AnyNode::HList(p) => p.idx(),
            AnyNode::VList(p) => p.idx(),
            AnyNode::Rule(p) => p.idx(),
            AnyNode::Ligature(p) => p.idx(),
            AnyNode::Disc(p) => p.idx(),
            AnyNode::Math(p) => p.idx(),
            AnyNode::Glue(p) => p.idx(),
            AnyNode::Kern(p) => p.idx(),
            AnyNode::Penalty(p) => p.idx(),
            AnyNode::Style(p) => p.idx(),
            AnyNode::Choice(p) => p.idx(),
            AnyNode::Noad(p) => p.idx(),
            AnyNode::MarginKern(p) => p.idx(),
            AnyNode::Mark(p) => p.idx(),
            AnyNode::Adjust(p) => p.idx(),
            AnyNode::Ins(p) => p.idx(),
            AnyNode::Unset(p) => p.idx(),
            AnyNode::Whatsit(i) => i,
        }
    }
}

impl MemState {
    /// Read the `NODE_type` (`mem[p].b16.s1`) of a lo-mem node as a checked
    /// [`NodeType`]. `p` must be a real lo-mem node index (the caller did
    /// the null / `is_char_node` check at the list-walk boundary, `01c` §7).
    #[inline]
    pub fn node_type(&self, p: MemIdx) -> Result<NodeType, InvalidNodeType> {
        NodeType::from_raw(self.b16s1(p.0))
    }

    /// Classify a non-null lo-mem node into its typed handle ([`AnyNode`],
    /// `01c` §3). `p` must be a real lo-mem node index — the caller did the
    /// null / `is_char_node` check at the list-walk boundary (`01c` §7). `Err`
    /// if the `NODE_type` code is not valid at all (the 32–39 gap / 41+).
    ///
    /// This confines every built family's `from_idx_unchecked` assertion to one
    /// audited site; a consumer that matches the result writes no `unsafe`.
    #[inline]
    pub fn classify(&self, p: MemIdx) -> Result<AnyNode, InvalidNodeType> {
        use NodeType::*;
        // SAFETY (every typed arm): the `NODE_type` value matched just below was
        // read from `p`, which proves `p`'s family; `from_idx_unchecked` asserts
        // exactly that and nothing more. Its kind `K` is inferred from the
        // `AnyNode` variant being constructed.
        Ok(match self.node_type(p)? {
            HList => AnyNode::HList(unsafe { NodePtr::from_idx_unchecked(p) }),
            VList => AnyNode::VList(unsafe { NodePtr::from_idx_unchecked(p) }),
            Rule => AnyNode::Rule(unsafe { NodePtr::from_idx_unchecked(p) }),
            Ins => AnyNode::Ins(unsafe { NodePtr::from_idx_unchecked(p) }),
            Mark => AnyNode::Mark(unsafe { NodePtr::from_idx_unchecked(p) }),
            Adjust => AnyNode::Adjust(unsafe { NodePtr::from_idx_unchecked(p) }),
            Ligature => AnyNode::Ligature(unsafe { NodePtr::from_idx_unchecked(p) }),
            Disc => AnyNode::Disc(unsafe { NodePtr::from_idx_unchecked(p) }),
            Whatsit => AnyNode::Whatsit(p),
            Math => AnyNode::Math(unsafe { NodePtr::from_idx_unchecked(p) }),
            Glue => AnyNode::Glue(unsafe { NodePtr::from_idx_unchecked(p) }),
            Kern => AnyNode::Kern(unsafe { NodePtr::from_idx_unchecked(p) }),
            Penalty => AnyNode::Penalty(unsafe { NodePtr::from_idx_unchecked(p) }),
            Unset => AnyNode::Unset(unsafe { NodePtr::from_idx_unchecked(p) }),
            Style => AnyNode::Style(unsafe { NodePtr::from_idx_unchecked(p) }),
            Choice => AnyNode::Choice(unsafe { NodePtr::from_idx_unchecked(p) }),
            OrdNoad | OpNoad | BinNoad | RelNoad | OpenNoad | CloseNoad | PunctNoad | InnerNoad
            | RadicalNoad | FractionNoad | UnderNoad | OverNoad | AccentNoad | VcenterNoad
            | LeftNoad | RightNoad => AnyNode::Noad(unsafe { NodePtr::from_idx_unchecked(p) }),
            MarginKern => AnyNode::MarginKern(unsafe { NodePtr::from_idx_unchecked(p) }),
        })
    }

    /// Classify a known lo-mem node as a box, returning the typed handle.
    ///
    /// `Ok(Some(_))` if `p` is an hlist or vlist; `Ok(None)` if it is some
    /// other (valid) node type; `Err` if the type code is not a valid
    /// `NODE_type` at all (the 32–39 gap / 41+, `01c` §1). `p` must already
    /// be a non-null lo-mem index.
    #[inline]
    pub fn classify_box(&self, p: MemIdx) -> Result<Option<AnyBox>, InvalidNodeType> {
        Ok(match self.node_type(p)? {
            // SAFETY: the NODE_type branch just proved the kind.
            NodeType::HList => Some(AnyBox::HList(unsafe {
                NodePtr::<HListNode>::from_idx_unchecked(p)
            })),
            NodeType::VList => Some(AnyBox::VList(unsafe {
                NodePtr::<VListNode>::from_idx_unchecked(p)
            })),
            _ => None,
        })
    }

    /// Classify a known lo-mem node as a rule node.
    #[inline]
    pub fn classify_rule(&self, p: MemIdx) -> Result<Option<NodePtr<RuleNode>>, InvalidNodeType> {
        Ok(match self.node_type(p)? {
            // SAFETY: the NODE_type branch just proved the kind.
            NodeType::Rule => Some(unsafe { NodePtr::<RuleNode>::from_idx_unchecked(p) }),
            _ => None,
        })
    }
}

impl NodePtr<RuleNode> {
    /// Allocate a fresh rule node (`RULE_NODE_SIZE` words), type-stamped with
    /// `NODE_type = RULE_NODE`, returning `None` on arena exhaustion.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<RuleNode>> {
        let p = get_node(mem, RULE_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, RULE_NODE);
        // SAFETY: `p` is freshly allocated as a rule-sized node and just
        // stamped RULE_NODE.
        Some(unsafe { NodePtr::<RuleNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    #[inline]
    pub fn width(self, mem: &MemState) -> RuleDimen {
        RuleDimen::from_raw(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_width(self, mem: &mut MemState, v: RuleDimen) {
        mem.set_b32s1(self.idx.offset(1).0, v.raw());
    }

    #[inline]
    pub fn depth(self, mem: &MemState) -> RuleDimen {
        RuleDimen::from_raw(mem.b32s1(self.idx.offset(2).0))
    }
    #[inline]
    pub fn set_depth(self, mem: &mut MemState, v: RuleDimen) {
        mem.set_b32s1(self.idx.offset(2).0, v.raw());
    }

    #[inline]
    pub fn height(self, mem: &MemState) -> RuleDimen {
        RuleDimen::from_raw(mem.b32s1(self.idx.offset(3).0))
    }
    #[inline]
    pub fn set_height(self, mem: &mut MemState, v: RuleDimen) {
        mem.set_b32s1(self.idx.offset(3).0, v.raw());
    }
}

// --- the ligature family: ligature nodes (NODE_type 6) ---

/// `LIGATURE_NODE` (`NODE_type` = 6): a ligature carrying font/character and a
/// successor list pointer in word +1.
pub enum LigatureNode {}

impl NodeKind for LigatureNode {}

impl NodePtr<LigatureNode> {
    /// Allocate a fresh ligature node (`SMALL_NODE_SIZE` words), type-stamped
    /// with `NODE_type = LIGATURE_NODE`, returning `None` on arena exhaustion.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<LigatureNode>> {
        let p = get_node(mem, SMALL_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, LIGATURE_NODE);
        // SAFETY: `p` is freshly allocated as a small node and just stamped
        // LIGATURE_NODE.
        Some(unsafe { NodePtr::<LigatureNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Ligature font = `mem[p + 1].b16.s1`.
    #[inline]
    pub fn font(self, mem: &MemState) -> FontId {
        FontId(mem.b16s1(self.idx.offset(1).0) as i32)
    }
    #[inline]
    pub fn set_font(self, mem: &mut MemState, v: FontId) {
        mem.set_b16s1(self.idx.offset(1).0, v.0 as u16);
    }

    /// Ligature character = `mem[p + 1].b16.s0`.
    #[inline]
    pub fn character(self, mem: &MemState) -> u16 {
        mem.b16s0(self.idx.offset(1).0)
    }
    #[inline]
    pub fn set_character(self, mem: &mut MemState, v: u16) {
        mem.set_b16s0(self.idx.offset(1).0, v);
    }

    /// Ligature successor list = `mem[p + 1].b32.s1`.
    #[inline]
    pub fn lig_ptr(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_lig_ptr(self, mem: &mut MemState, v: MemPtr) {
        mem.set_b32s1(self.idx.offset(1).0, v.0);
    }
}

impl MemState {
    /// Classify a known lo-mem node as a ligature node.
    #[inline]
    pub fn classify_ligature(
        &self,
        p: MemIdx,
    ) -> Result<Option<NodePtr<LigatureNode>>, InvalidNodeType> {
        Ok(match self.node_type(p)? {
            // SAFETY: the NODE_type branch just proved the kind.
            NodeType::Ligature => Some(unsafe { NodePtr::<LigatureNode>::from_idx_unchecked(p) }),
            _ => None,
        })
    }
}

// --- the ligature-item record (part of the ligature stack) ---

/// A ligature stack item record. It is a temporary record of `SMALL_NODE_SIZE` (2)
/// that represents a character in the ligature processing stack.
/// It carries no `NODE_type` header (its word 0 holds character and link).
pub enum LigItem {}

impl NodeKind for LigItem {}

impl NodePtr<LigItem> {
    /// Allocate a fresh, uninitialised ligature item record on the `mem`
    /// free-list arena and return a typed handle to it, or `None` on arena
    /// exhaustion.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<LigItem>> {
        let p = get_node(mem, SMALL_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        // SAFETY: `p` is a freshly allocated `SMALL_NODE_SIZE` record; this is
        // the family's single audited construction site.
        Some(unsafe { NodePtr::<LigItem>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Link pointer to the next stack element = `mem[p].b32.s1`.
    #[inline]
    pub fn link(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.0))
    }
    #[inline]
    pub fn set_link(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.idx.0, q.0);
    }

    /// Ligature item character code = `mem[p].b16.s0`.
    #[inline]
    pub fn character(self, mem: &MemState) -> u16 {
        mem.b16s0(self.idx.0)
    }
    #[inline]
    pub fn set_character(self, mem: &mut MemState, c: u16) {
        mem.set_b16s0(self.idx.0, c);
    }

    /// Ligature item component pointer = `mem[p + 1].b32.s1`.
    #[inline]
    pub fn component(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_component(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.idx.offset(1).0, q.0);
    }
}

// --- the discretionary family: discretionary break nodes (NODE_type 7) ---

/// `DISC_NODE` (`NODE_type` = 7): discretionary break with pre/post lists.
pub enum DiscNode {}

impl NodeKind for DiscNode {}

impl NodePtr<DiscNode> {
    /// Allocate a fresh discretionary node (`SMALL_NODE_SIZE` words),
    /// type-stamped with `NODE_type = DISC_NODE`, returning `None` on arena
    /// exhaustion.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<DiscNode>> {
        let p = get_node(mem, SMALL_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, DISC_NODE);
        // SAFETY: `p` is freshly allocated as a small node and just stamped
        // DISC_NODE.
        Some(unsafe { NodePtr::<DiscNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Pre-break list = `mem[p + 1].b32.s0`.
    #[inline]
    pub fn pre_break(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s0(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_pre_break(self, mem: &mut MemState, v: MemPtr) {
        mem.set_b32s0(self.idx.offset(1).0, v.0);
    }

    /// Post-break list = `mem[p + 1].b32.s1`.
    #[inline]
    pub fn post_break(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_post_break(self, mem: &mut MemState, v: MemPtr) {
        mem.set_b32s1(self.idx.offset(1).0, v.0);
    }
}

impl MemState {
    /// Classify a known lo-mem node as a discretionary node.
    #[inline]
    pub fn classify_disc(&self, p: MemIdx) -> Result<Option<NodePtr<DiscNode>>, InvalidNodeType> {
        Ok(match self.node_type(p)? {
            // SAFETY: the NODE_type branch just proved the kind.
            NodeType::Disc => Some(unsafe { NodePtr::<DiscNode>::from_idx_unchecked(p) }),
            _ => None,
        })
    }
}

// --- the math-node family: math-on/off nodes (NODE_type 9) ---

/// `MATH_NODE` (`NODE_type` = 9): an inline math boundary node with a scaled
/// width and an LR/math subtype in the shared header's subtype field.
pub enum MathNode {}

impl NodeKind for MathNode {}

impl NodePtr<MathNode> {
    /// Allocate a fresh math node (`MEDIUM_NODE_SIZE` words), type-stamped
    /// with `NODE_type = MATH_NODE`, returning `None` on arena exhaustion.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<MathNode>> {
        let p = get_node(mem, MEDIUM_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, MATH_NODE);
        // SAFETY: `p` is a freshly allocated `MEDIUM_NODE_SIZE` node just
        // stamped MATH_NODE; this is the family's audited construction site.
        Some(unsafe { NodePtr::<MathNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Math-node width = `mem[(p) + 1].b32.s1`.
    #[inline]
    pub fn width(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_width(self, mem: &mut MemState, w: Scaled) {
        mem.set_b32s1(self.idx.offset(1).0, w.0);
    }
}

impl MemState {
    /// Classify a known lo-mem node as a math node.
    #[inline]
    pub fn classify_math(&self, p: MemIdx) -> Result<Option<NodePtr<MathNode>>, InvalidNodeType> {
        Ok(match self.node_type(p)? {
            // SAFETY: the NODE_type branch just proved the kind.
            NodeType::Math => Some(unsafe { NodePtr::<MathNode>::from_idx_unchecked(p) }),
            _ => None,
        })
    }
}

// --- math style/choice nodes (NODE_type 14 and 15) ---

const STYLE_NODE_SIZE: i32 = 3;

/// `STYLE_NODE` (`NODE_type` = 14): changes the current math style.
pub enum StyleNode {}
/// `EDGE_NODE` (`NODE_type` = 14 in LR-edge context): a shipout edge record.
pub enum EdgeNode {}
/// `CHOICE_NODE` (`NODE_type` = 15): four nullable math-list alternatives.
pub enum ChoiceNode {}

impl NodeKind for StyleNode {}
impl NodeKind for EdgeNode {}
impl NodeKind for ChoiceNode {}
impl HasNodeHeader for EdgeNode {}

impl NodePtr<StyleNode> {
    /// Allocate a fresh style node (`STYLE_NODE_SIZE` words), type-stamped
    /// with `NODE_type = STYLE_NODE`, returning `None` on arena exhaustion.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<StyleNode>> {
        let p = get_node(mem, STYLE_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, NodeType::Style.as_raw());
        // SAFETY: `p` is freshly allocated and just stamped STYLE_NODE.
        Some(unsafe { NodePtr::<StyleNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Zero the two payload words used by the C constructor.
    #[inline]
    pub fn clear_payload(self, mem: &mut MemState) {
        mem.set_b32s1(self.idx.offset(1).0, 0);
        mem.set_b32s1(self.idx.offset(2).0, 0);
    }
}

impl NodePtr<EdgeNode> {
    /// Allocate a fresh edge node (`STYLE_NODE_SIZE` words), type-stamped with
    /// `NODE_type = STYLE_NODE` because XeTeX aliases `EDGE_NODE` to code 14.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<EdgeNode>> {
        let p = get_node(mem, STYLE_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, NodeType::Style.as_raw());
        // SAFETY: `p` is freshly allocated and just stamped with the edge
        // context's NODE_type code.
        Some(unsafe { NodePtr::<EdgeNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Set edge width = `mem[p + 1].b32.s1`.
    #[inline]
    pub fn set_width(self, mem: &mut MemState, w: Scaled) {
        mem.set_b32s1(self.idx.offset(1).0, w.0);
    }

    /// Set edge distance = `mem[p + 2].b32.s1`.
    #[inline]
    pub fn set_edge_dist(self, mem: &mut MemState, w: Scaled) {
        mem.set_b32s1(self.idx.offset(2).0, w.0);
    }
}

impl NodePtr<ChoiceNode> {
    /// Allocate a fresh choice node (`STYLE_NODE_SIZE` words), type-stamped
    /// with `NODE_type = CHOICE_NODE`, returning `None` on arena exhaustion.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<ChoiceNode>> {
        let p = get_node(mem, STYLE_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, NodeType::Choice.as_raw());
        // SAFETY: `p` is freshly allocated and just stamped CHOICE_NODE.
        Some(unsafe { NodePtr::<ChoiceNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Display-style branch pointer = `mem[p + 1].b32.s0`.
    #[inline]
    pub fn display_mlist(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s0(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_display_mlist(self, mem: &mut MemState, v: MemPtr) {
        mem.set_b32s0(self.idx.offset(1).0, v.0);
    }

    /// Text-style branch pointer = `mem[p + 1].b32.s1`.
    #[inline]
    pub fn text_mlist(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_text_mlist(self, mem: &mut MemState, v: MemPtr) {
        mem.set_b32s1(self.idx.offset(1).0, v.0);
    }

    /// Script-style branch pointer = `mem[p + 2].b32.s0`.
    #[inline]
    pub fn script_mlist(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s0(self.idx.offset(2).0))
    }
    #[inline]
    pub fn set_script_mlist(self, mem: &mut MemState, v: MemPtr) {
        mem.set_b32s0(self.idx.offset(2).0, v.0);
    }

    /// Script-script-style branch pointer = `mem[p + 2].b32.s1`.
    #[inline]
    pub fn script_script_mlist(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(2).0))
    }
    #[inline]
    pub fn set_script_script_mlist(self, mem: &mut MemState, v: MemPtr) {
        mem.set_b32s1(self.idx.offset(2).0, v.0);
    }
}

impl MemState {
    /// Classify a known lo-mem node as a style node.
    #[inline]
    pub fn classify_style(&self, p: MemIdx) -> Result<Option<NodePtr<StyleNode>>, InvalidNodeType> {
        Ok(match self.node_type(p)? {
            // SAFETY: the NODE_type branch just proved the kind.
            NodeType::Style => Some(unsafe { NodePtr::<StyleNode>::from_idx_unchecked(p) }),
            _ => None,
        })
    }

    /// Classify a known lo-mem node as a choice node.
    #[inline]
    pub fn classify_choice(
        &self,
        p: MemIdx,
    ) -> Result<Option<NodePtr<ChoiceNode>>, InvalidNodeType> {
        Ok(match self.node_type(p)? {
            // SAFETY: the NODE_type branch just proved the kind.
            NodeType::Choice => Some(unsafe { NodePtr::<ChoiceNode>::from_idx_unchecked(p) }),
            _ => None,
        })
    }
}

// --- the math noad family (NODE_type 16 to 31) ---

const NOAD_SIZE: i32 = 4;
const RADICAL_NOAD_SIZE: i32 = 5;
const ACCENT_NOAD_SIZE: i32 = 5;
const FRACTION_NOAD_SIZE: i32 = 6;

/// `Noad` (math noads, `NODE_type` between 16 and 31 inclusive): represents a math noad structure.
pub enum Noad {}

impl NodeKind for Noad {}

impl NodePtr<Noad> {
    /// Allocate a fresh ordinary noad (`NOAD_SIZE` = 4 words), type-stamped
    /// with `NODE_type = OrdNoad`, returning `None` on arena exhaustion.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<Noad>> {
        let p = get_node(mem, NOAD_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, NodeType::OrdNoad.as_raw());
        // SAFETY: `p` is freshly allocated and type-stamped.
        Some(unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Allocate a fresh radical noad (`RADICAL_NOAD_SIZE` = 5 words), type-stamped
    /// with `NODE_type = RadicalNoad`, returning `None` on arena exhaustion.
    #[inline]
    pub fn alloc_radical(mem: &mut MemState) -> Option<NodePtr<Noad>> {
        let p = get_node(mem, RADICAL_NOAD_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, NodeType::RadicalNoad.as_raw());
        Some(unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Allocate a fresh accent noad (`ACCENT_NOAD_SIZE` = 5 words), type-stamped
    /// with `NODE_type = AccentNoad`, returning `None` on arena exhaustion.
    #[inline]
    pub fn alloc_accent(mem: &mut MemState) -> Option<NodePtr<Noad>> {
        let p = get_node(mem, ACCENT_NOAD_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, NodeType::AccentNoad.as_raw());
        Some(unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Allocate a fresh fraction noad (`FRACTION_NOAD_SIZE` = 6 words), type-stamped
    /// with `NODE_type = FractionNoad`, returning `None` on arena exhaustion.
    #[inline]
    pub fn alloc_fraction(mem: &mut MemState) -> Option<NodePtr<Noad>> {
        let p = get_node(mem, FRACTION_NOAD_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, NodeType::FractionNoad.as_raw());
        Some(unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Set the noad's type code = `mem[p].b16.s1`.
    #[inline]
    pub fn set_noad_type(self, mem: &mut MemState, nt: NodeType) {
        mem.set_b16s1(self.idx.0, nt.as_raw());
    }

    /// Get the noad's type code = `mem[p].b16.s1`.
    #[inline]
    pub fn noad_type(self, mem: &MemState) -> NodeType {
        NodeType::from_raw(mem.b16s1(self.idx.0)).expect("valid noad type")
    }

    /// Nucleus type = `mem[p + 1].b32.s1`
    #[inline]
    pub fn nucleus_type(self, mem: &MemState) -> i32 {
        mem.b32s1(self.idx.offset(1).0)
    }
    #[inline]
    pub fn set_nucleus_type(self, mem: &mut MemState, t: i32) {
        mem.set_b32s1(self.idx.offset(1).0, t);
    }
    /// Nucleus field = `mem[p + 1].b32.s0`
    #[inline]
    pub fn nucleus_field(self, mem: &MemState) -> i32 {
        mem.b32s0(self.idx.offset(1).0)
    }
    #[inline]
    pub fn set_nucleus_field(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self.idx.offset(1).0, v);
    }

    /// Nucleus character = `mem[p + 1].b16.s0`
    #[inline]
    pub fn nucleus_char(self, mem: &MemState) -> u16 {
        mem.b16s0(self.idx.offset(1).0)
    }
    #[inline]
    pub fn set_nucleus_char(self, mem: &mut MemState, c: u16) {
        mem.set_b16s0(self.idx.offset(1).0, c);
    }
    /// Nucleus family = `mem[p + 1].b16.s1`
    #[inline]
    pub fn nucleus_fam(self, mem: &MemState) -> u16 {
        mem.b16s1(self.idx.offset(1).0)
    }
    #[inline]
    pub fn set_nucleus_fam(self, mem: &mut MemState, f: u16) {
        mem.set_b16s1(self.idx.offset(1).0, f);
    }

    // Noad script layout (matches TeX/this build): superscript = offset +2,
    // subscript = offset +3 (cf. `prepare_subscript_global(q + 3)` in math.rs).
    // Session 163 swapped the previously-inverted `subscr_*`/`supscr_*` names so
    // they now match their offsets; the swap was offset-preserving at every call
    // site (names and bodies moved together).

    /// Superscript type = `mem[p + 2].b32.s1`
    #[inline]
    pub fn supscr_type(self, mem: &MemState) -> i32 {
        mem.b32s1(self.idx.offset(2).0)
    }
    #[inline]
    pub fn set_supscr_type(self, mem: &mut MemState, t: i32) {
        mem.set_b32s1(self.idx.offset(2).0, t);
    }
    /// Superscript field = `mem[p + 2].b32.s0`
    #[inline]
    pub fn supscr_field(self, mem: &MemState) -> i32 {
        mem.b32s0(self.idx.offset(2).0)
    }
    #[inline]
    pub fn set_supscr_field(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self.idx.offset(2).0, v);
    }

    /// Superscript character = `mem[p + 2].b16.s0`
    #[inline]
    pub fn supscr_char(self, mem: &MemState) -> u16 {
        mem.b16s0(self.idx.offset(2).0)
    }
    #[inline]
    pub fn set_supscr_char(self, mem: &mut MemState, c: u16) {
        mem.set_b16s0(self.idx.offset(2).0, c);
    }
    /// Superscript family = `mem[p + 2].b16.s1`
    #[inline]
    pub fn supscr_fam(self, mem: &MemState) -> u16 {
        mem.b16s1(self.idx.offset(2).0)
    }
    #[inline]
    pub fn set_supscr_fam(self, mem: &mut MemState, f: u16) {
        mem.set_b16s1(self.idx.offset(2).0, f);
    }

    /// Subscript type = `mem[p + 3].b32.s1`
    #[inline]
    pub fn subscr_type(self, mem: &MemState) -> i32 {
        mem.b32s1(self.idx.offset(3).0)
    }
    #[inline]
    pub fn set_subscr_type(self, mem: &mut MemState, t: i32) {
        mem.set_b32s1(self.idx.offset(3).0, t);
    }
    /// Subscript field = `mem[p + 3].b32.s0`
    #[inline]
    pub fn subscr_field(self, mem: &MemState) -> i32 {
        mem.b32s0(self.idx.offset(3).0)
    }
    #[inline]
    pub fn set_subscr_field(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self.idx.offset(3).0, v);
    }

    /// Subscript character = `mem[p + 3].b16.s0`
    #[inline]
    pub fn subscr_char(self, mem: &MemState) -> u16 {
        mem.b16s0(self.idx.offset(3).0)
    }
    #[inline]
    pub fn set_subscr_char(self, mem: &mut MemState, c: u16) {
        mem.set_b16s0(self.idx.offset(3).0, c);
    }
    /// Subscript family = `mem[p + 3].b16.s1`
    #[inline]
    pub fn subscr_fam(self, mem: &MemState) -> u16 {
        mem.b16s1(self.idx.offset(3).0)
    }
    #[inline]
    pub fn set_subscr_fam(self, mem: &mut MemState, f: u16) {
        mem.set_b16s1(self.idx.offset(3).0, f);
    }

    /// Thickness (Fraction only) = `mem[p + 1].b32.s1`
    #[inline]
    pub fn thickness(self, mem: &MemState) -> i32 {
        mem.b32s1(self.idx.offset(1).0)
    }
    #[inline]
    pub fn set_thickness(self, mem: &mut MemState, v: i32) {
        mem.set_b32s1(self.idx.offset(1).0, v);
    }

    /// Accent character type (Accent only) = `mem[p + 4].b32.s1`
    #[inline]
    pub fn accent_char_type(self, mem: &MemState) -> i32 {
        mem.b32s1(self.idx.offset(4).0)
    }
    #[inline]
    pub fn set_accent_char_type(self, mem: &mut MemState, t: i32) {
        mem.set_b32s1(self.idx.offset(4).0, t);
    }
    /// Accent character field (Accent only) = `mem[p + 4].b32.s0`
    #[inline]
    pub fn accent_char_field(self, mem: &MemState) -> i32 {
        mem.b32s0(self.idx.offset(4).0)
    }
    #[inline]
    pub fn set_accent_char_field(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self.idx.offset(4).0, v);
    }

    /// Accent character code = `mem[p + 4].b16.s0`
    #[inline]
    pub fn accent_char_val(self, mem: &MemState) -> u16 {
        mem.b16s0(self.idx.offset(4).0)
    }
    #[inline]
    pub fn set_accent_char_val(self, mem: &mut MemState, c: u16) {
        mem.set_b16s0(self.idx.offset(4).0, c);
    }
    /// Accent character family = `mem[p + 4].b16.s1`
    #[inline]
    pub fn accent_char_fam(self, mem: &MemState) -> u16 {
        mem.b16s1(self.idx.offset(4).0)
    }
    #[inline]
    pub fn set_accent_char_fam(self, mem: &mut MemState, f: u16) {
        mem.set_b16s1(self.idx.offset(4).0, f);
    }

    /// Delimiter field address (Radical: word 4, LeftRight: word 1)
    #[inline]
    pub fn delimiter_addr(self) -> i32 {
        self.idx.offset(4).0
    }

    /// Left/right delimiter address (LeftRight: word 1)
    #[inline]
    pub fn left_right_delimiter_addr(self) -> i32 {
        self.idx.offset(1).0
    }

    /// Fraction left delimiter address (Fraction: word 4)
    #[inline]
    pub fn fraction_left_delimiter_addr(self) -> i32 {
        self.idx.offset(4).0
    }

    /// Fraction right delimiter address (Fraction: word 5)
    #[inline]
    pub fn fraction_right_delimiter_addr(self) -> i32 {
        self.idx.offset(5).0
    }

    /// Get the nucleus field address (word 1)
    #[inline]
    pub fn nucleus_addr(self) -> i32 {
        self.idx.offset(1).0
    }

    /// Get the supscr field address (word 2)
    #[inline]
    pub fn supscr_addr(self) -> i32 {
        self.idx.offset(2).0
    }

    /// Get the subscr field address (word 3)
    #[inline]
    pub fn subscr_addr(self) -> i32 {
        self.idx.offset(3).0
    }
}

impl MemState {
    /// Classify a known lo-mem node as a noad.
    #[inline]
    pub fn classify_noad(&self, p: MemIdx) -> Result<Option<NodePtr<Noad>>, InvalidNodeType> {
        let nt = self.node_type(p)?;
        Ok(match nt {
            NodeType::OrdNoad
            | NodeType::OpNoad
            | NodeType::BinNoad
            | NodeType::RelNoad
            | NodeType::OpenNoad
            | NodeType::CloseNoad
            | NodeType::PunctNoad
            | NodeType::InnerNoad
            | NodeType::RadicalNoad
            | NodeType::FractionNoad
            | NodeType::UnderNoad
            | NodeType::OverNoad
            | NodeType::AccentNoad
            | NodeType::VcenterNoad
            | NodeType::LeftNoad
            | NodeType::RightNoad => Some(unsafe { NodePtr::<Noad>::from_idx_unchecked(p) }),
            _ => None,
        })
    }
}

// --- the glue family: glue nodes and glue_spec records ---
//
// Two distinct node shapes that are always reached together:
//
// `GlueNode` (`NODE_type` = `GLUE_NODE` = 10; allocated `MEDIUM_NODE_SIZE` = 3
// in this build because SyncTeX rides in the extra word):
//
// | field        | offset.subfield | type                          |
// |--------------|-----------------|-------------------------------|
// | `subtype`    | 0.b16.s0        | `u16` (the header word)        |
// | `glue_ptr`   | +1.b32.s0       | `MemPtr` → the glue_spec       |
// | `leader_ptr` | +1.b32.s1       | `MemPtr` (nullable: no leaders) |
//
// `GlueSpecNode` (`GLUE_SPEC_SIZE` = 4 — a *record*, never a list node):
//
// | field           | offset.subfield | type                              |
// |-----------------|-----------------|-----------------------------------|
// | `ref_count`     | 0.b32.s1        | `i32` — a REFERENCE COUNT, not a link |
// | `stretch_order` | 0.b16.s1        | `u16` (the fil order of `stretch`) |
// | `shrink_order`  | 0.b16.s0        | `u16` (the fil order of `shrink`)  |
// | `width`         | +1.b32.s1       | `Scaled` (the natural glue amount) |
// | `stretch`       | +2.b32.s1       | `Scaled`                          |
// | `shrink`        | +3.b32.s1       | `Scaled`                          |
//
// (`width` shares the generic `BOX_width` +1.b32.s1 offset, so there is no
// dedicated `GLUE_SPEC_width` C macro — verified against `new_spec`/
// `math_glue` in `src/mem.rs`/`src/math.rs`, which read/write `mem[p+1]` as
// the spec width.)
//
// **Classification differs from the box family.** A glue_spec is *not*
// found via `NODE_type`: its `0.b16.s1` word holds `stretch_order`, not a
// type code. A glue_spec is only ever reached through a glue node's
// `glue_ptr` (or a parameter-glue pointer), so its constructor is
// [`NodePtr::<GlueSpecNode>::from_glue_ptr`] (a from-`MemPtr` boundary),
// **not** a `node_type` branch. Only the glue *node* is classified by
// `NODE_type` (== `GLUE_NODE`), via [`MemState::classify_glue`].
//
// **The ref_count trap (`01c` §3).** `GLUE_SPEC_ref_count` lives in the
// `0.b32.s1` word — the same word that holds `link()` in a real list node.
// On a glue_spec it is a reference count, so it is exposed as `ref_count() ->
// i32`, never as `link()`/`MemPtr`. Treating it as a pointer is a silent
// parity break.

/// `GLUE_NODE` (`NODE_type` = 10): inter-element glue, pointing at a shared
/// `GlueSpecNode` plus an optional leader box/rule list.
pub enum GlueNode {}
/// `GLUE_SPEC` record (`GLUE_SPEC_SIZE` = 4): the shared, reference-counted
/// stretch/shrink specification a glue node refers to. Not a list node — it
/// carries no `NODE_type` and never appears on a horizontal/vertical list.
pub enum GlueSpecNode {}

impl NodeKind for GlueNode {}
impl NodeKind for GlueSpecNode {}

impl NodePtr<GlueNode> {
    /// Allocate a fresh glue node on the `mem` free-list arena, type-stamped
    /// with `NODE_type = GLUE_NODE`, returning `None` on arena exhaustion.
    ///
    /// This preserves the current XeTeX allocation size (`MEDIUM_NODE_SIZE`,
    /// not the generic-TeX small-node size) and confines the family's typed
    /// construction to this single audited site. The caller fills
    /// `subtype`/`glue_ptr`/`leader_ptr`.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<GlueNode>> {
        let p = get_node(mem, MEDIUM_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, GLUE_NODE);
        // SAFETY: `p` is a freshly allocated `MEDIUM_NODE_SIZE` node just
        // stamped GLUE_NODE; this is the family's audited construction site.
        Some(unsafe { NodePtr::<GlueNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// `GLUE_NODE_glue_ptr(p)` = `mem[(p) + 1].b32.s0` — the pointer to this
    /// node's [`GlueSpecNode`]. Returned as a raw [`MemPtr`]; the caller
    /// classifies it (typically via [`NodePtr::<GlueSpecNode>::from_glue_ptr`]
    /// or the [`glue_spec`](Self::glue_spec) helper).
    #[inline]
    pub fn glue_ptr(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s0(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_glue_ptr(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s0(self.idx.offset(1).0, q.0);
    }

    /// `GLUE_NODE_leader_ptr(p)` = `mem[(p) + 1].b32.s1` — the leader
    /// box/rule list head, or `TEX_NULL` for plain glue. Nullable [`MemPtr`].
    #[inline]
    pub fn leader_ptr(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_leader_ptr(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.idx.offset(1).0, q.0);
    }

    /// Classify this glue node's `glue_ptr` as a [`GlueSpecNode`] handle, or
    /// `None` if the pointer is null. Convenience over
    /// `NodePtr::<GlueSpecNode>::from_glue_ptr(p.glue_ptr(mem))`.
    #[inline]
    pub fn glue_spec(self, mem: &MemState) -> Option<NodePtr<GlueSpecNode>> {
        NodePtr::<GlueSpecNode>::from_glue_ptr(self.glue_ptr(mem))
    }
}

impl NodePtr<GlueSpecNode> {
    /// Classify a glue node's `glue_ptr` (or a parameter-glue pointer) as a
    /// `GlueSpecNode` handle. This is the **only** entry into the family — a
    /// glue_spec carries no `NODE_type`, so it is never produced by
    /// [`MemState::classify_box`]/[`node_type`](MemState::node_type). Returns
    /// `None` if `p` is null.
    ///
    /// In practice every glue node points at a real spec, so `None` signals a
    /// caller bug (a null glue_ptr); the caller may `expect` it.
    #[inline]
    pub fn from_glue_ptr(p: MemPtr) -> Option<NodePtr<GlueSpecNode>> {
        // SAFETY: a non-null glue_ptr always references a GLUE_SPEC record by
        // construction (it is only ever written by the spec allocators).
        p.as_idx()
            .map(|idx| unsafe { NodePtr::<GlueSpecNode>::from_idx_unchecked(idx) })
    }

    /// Allocate a fresh, **uninitialised** glue_spec record on the `mem`
    /// free-list arena and return a typed handle to it, or `None` if the
    /// arena is exhausted (the §5 overflow condition — the caller maps it to
    /// its abort sentinel after the borrow drops).
    ///
    /// This is the **typesafe node-allocation boundary** for the family
    /// (`01b` §3a / `01c` §8): it confines the one `from_idx_unchecked` to a
    /// single audited site (a node *being initialised* has no `NODE_type` to
    /// classify yet), so call sites never write `unsafe`. It keeps TeX's
    /// free-list arena (no per-node heap alloc, `01c` §8) — it is exactly the
    /// `get_node(GLUE_SPEC_SIZE)` the C `new_spec`/`math_glue` do, lifted to a
    /// typed return. A glue_spec carries no `NODE_type`, so nothing is
    /// stamped; the caller fills `width`/`stretch`/`shrink`/orders. (A
    /// type-stamping family's `alloc` would additionally `set_b16s1(NODE_type)`
    /// before returning — the pattern extends per family as consumers appear.)
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<GlueSpecNode>> {
        let p = get_node(mem, GLUE_SPEC_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        // SAFETY: `p` is a freshly allocated `GLUE_SPEC_SIZE` record; this is
        // the family's single audited construction site (the node is being
        // initialised by the caller, so there is no tag to classify).
        Some(unsafe { NodePtr::<GlueSpecNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// `GLUE_SPEC_ref_count(p)` = `mem[p].b32.s1`.
    ///
    /// **This is a reference count, not a link** (`01c` §3). It occupies the
    /// `0.b32.s1` word — the slot that holds the successor pointer on a real
    /// list node — but on a glue_spec it counts the glue nodes sharing this
    /// record. Exposed as a plain `i32`; never as `link()`/[`MemPtr`].
    #[inline]
    pub fn ref_count(self, mem: &MemState) -> i32 {
        mem.b32s1(self.idx.0)
    }
    #[inline]
    pub fn set_ref_count(self, mem: &mut MemState, v: i32) {
        mem.set_b32s1(self.idx.0, v);
    }

    /// `GLUE_SPEC_stretch_order(p)` = `mem[p].b16.s1` — the fil order
    /// (`NORMAL`/`FIL`/`FILL`/`FILLL`) of the stretch component. Lives in the
    /// header word's high half, where a list node holds `NODE_type`.
    #[inline]
    pub fn stretch_order(self, mem: &MemState) -> u16 {
        mem.b16s1(self.idx.0)
    }
    #[inline]
    pub fn set_stretch_order(self, mem: &mut MemState, v: u16) {
        mem.set_b16s1(self.idx.0, v);
    }

    /// `GLUE_SPEC_shrink_order(p)` = `mem[p].b16.s0` — the fil order of the
    /// shrink component. Header word's low half.
    #[inline]
    pub fn shrink_order(self, mem: &MemState) -> u16 {
        mem.b16s0(self.idx.0)
    }
    #[inline]
    pub fn set_shrink_order(self, mem: &mut MemState, v: u16) {
        mem.set_b16s0(self.idx.0, v);
    }

    /// Glue-spec `width` = `mem[(p) + 1].b32.s1` — the natural glue amount.
    /// Shares the generic `BOX_width` +1.b32.s1 slot (no dedicated
    /// `GLUE_SPEC_width` macro), but on a glue_spec it is the rigid width.
    #[inline]
    pub fn width(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_width(self, mem: &mut MemState, v: Scaled) {
        mem.set_b32s1(self.idx.offset(1).0, v.0);
    }

    /// `GLUE_SPEC_stretch(p)` = `mem[(p) + 2].b32.s1` — the stretchability.
    #[inline]
    pub fn stretch(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(2).0))
    }
    #[inline]
    pub fn set_stretch(self, mem: &mut MemState, v: Scaled) {
        mem.set_b32s1(self.idx.offset(2).0, v.0);
    }

    /// `GLUE_SPEC_shrink(p)` = `mem[(p) + 3].b32.s1` — the shrinkability.
    #[inline]
    pub fn shrink(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(3).0))
    }
    #[inline]
    pub fn set_shrink(self, mem: &mut MemState, v: Scaled) {
        mem.set_b32s1(self.idx.offset(3).0, v.0);
    }
}

impl MemState {
    /// Classify a known lo-mem node as a glue node, returning the typed
    /// handle. `Ok(Some(_))` if `p` is a `GLUE_NODE`; `Ok(None)` if it is some
    /// other (valid) node type; `Err` if the type code is invalid. `p` must
    /// already be a non-null lo-mem index (the caller did the list-walk
    /// null/`is_char_node` check, `01c` §7).
    ///
    /// Note there is no analogous `classify_glue_spec`: a glue_spec carries no
    /// `NODE_type`, so it is reached only through
    /// [`NodePtr::<GlueSpecNode>::from_glue_ptr`].
    #[inline]
    pub fn classify_glue(&self, p: MemIdx) -> Result<Option<NodePtr<GlueNode>>, InvalidNodeType> {
        Ok(match self.node_type(p)? {
            // SAFETY: the NODE_type branch just proved the kind.
            NodeType::Glue => Some(unsafe { NodePtr::<GlueNode>::from_idx_unchecked(p) }),
            _ => None,
        })
    }
}

// --- the kern family: kern nodes (NODE_type 11) ---
//
// `KernNode` (`NODE_type` = `KERN_NODE` = 11; allocated `MEDIUM_NODE_SIZE` =
// 3 in this build — the generic-TeX "small (2)" grows to medium when SyncTeX
// fields ride along):
//
// | field     | offset.subfield | type                              |
// |-----------|-----------------|-----------------------------------|
// | `subtype` | 0.b16.s0        | `u16` (NORMAL/EXPLICIT/ACC_KERN/MU_GLUE) |
// | `width`   | +1.b32.s1       | `Scaled`                          |
//
// `width` is the same +1.b32.s1 `Scaled` slot as the box family's `width`,
// but a kern node has **no** depth/height/shift_amount — those offsets hold
// other data (e.g. the SyncTeX word) — so kern uses a **concrete** `width`
// method, *not* the `HasDimensions` capability trait (which would falsely
// expose +2..+4).

/// `KERN_NODE` (`NODE_type` = 11): inter-element kern (a single signed
/// width).
pub enum KernNode {}

impl NodeKind for KernNode {}

impl NodePtr<KernNode> {
    /// Allocate a fresh kern node on the `mem` free-list arena, **type-stamped**
    /// with `NODE_type = KERN_NODE`, and return a typed handle to it — or `None`
    /// if the arena is exhausted (the §5 overflow condition, mapped by the
    /// caller to its abort sentinel after the borrow drops).
    ///
    /// This is the **type-stamping** form of the typesafe node-allocation
    /// boundary (`01b` §3a / `01c` §8): unlike
    /// [`NodePtr::<GlueSpecNode>::alloc`] (a glue_spec carries no `NODE_type`,
    /// so it stamps nothing), a kern node *is* a list node, so its `alloc`
    /// additionally writes `NODE_type` into the header before returning. It
    /// confines the family's one `from_idx_unchecked` to this single audited
    /// site (the node is being initialised, so there is no tag to *classify* —
    /// `alloc` *writes* the tag), keeping TeX's free-list arena (no per-node
    /// heap alloc). It is exactly the `get_node(MEDIUM_NODE_SIZE)` +
    /// `type(p) = kern_node` the C `new_kern` does, lifted to a typed return;
    /// the caller fills `subtype`/`width`.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<KernNode>> {
        let p = get_node(mem, MEDIUM_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        // type-stamp: a kern node carries NODE_type = KERN_NODE in its header.
        mem.set_b16s1(p, KERN_NODE);
        // SAFETY: `p` is a freshly allocated `MEDIUM_NODE_SIZE` node just stamped
        // KERN_NODE; this is the family's single audited construction site.
        Some(unsafe { NodePtr::<KernNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// `KERN_NODE` width = `mem[(p) + 1].b32.s1` — the kern amount. Shares the
    /// box `width` slot/type but is a one-field family (concrete impl).
    #[inline]
    pub fn width(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_width(self, mem: &mut MemState, w: Scaled) {
        mem.set_b32s1(self.idx.offset(1).0, w.0);
    }
}

impl MemState {
    /// Classify a known lo-mem node as a kern node. `Ok(Some(_))` if `p` is a
    /// `KERN_NODE`; `Ok(None)` if some other valid type; `Err` if invalid.
    /// `p` must already be a non-null lo-mem index (`01c` §7).
    #[inline]
    pub fn classify_kern(&self, p: MemIdx) -> Result<Option<NodePtr<KernNode>>, InvalidNodeType> {
        Ok(match self.node_type(p)? {
            // SAFETY: the NODE_type branch just proved the kind.
            NodeType::Kern => Some(unsafe { NodePtr::<KernNode>::from_idx_unchecked(p) }),
            _ => None,
        })
    }
}

// --- the penalty family: penalty nodes (NODE_type 12) ---
//
// `PenaltyNode` (`NODE_type` = `PENALTY_NODE` = 12; allocated
// `MEDIUM_NODE_SIZE` = 3 in this build, like kern):
//
// | field     | offset.subfield | type                       |
// |-----------|-----------------|----------------------------|
// | `penalty` | +1.b32.s1       | **`i32`** (NOT `Scaled`)    |
//
// **The penalty trap (`01c` §3).** `PENALTY_NODE_penalty` occupies the same
// +1.b32.s1 slot as the box/kern `width` (a `Scaled`), but it is a plain
// **integer breakpoint penalty** (`int`), not a scaled length. Returning
// `Scaled` here would be a silent type/parity error — it is exposed as
// `penalty() -> i32`.

/// `PENALTY_NODE` (`NODE_type` = 12): a breakpoint penalty (a plain `i32`,
/// `INF_PENALTY`/`EJECT_PENALTY` and everything between).
pub enum PenaltyNode {}

impl NodeKind for PenaltyNode {}

impl NodePtr<PenaltyNode> {
    /// Allocate a fresh penalty node on the `mem` free-list arena,
    /// **type-stamped** with `NODE_type = PENALTY_NODE`, and return a typed
    /// handle to it — or `None` on arena exhaustion (the §5 overflow condition,
    /// mapped by the caller to its abort sentinel after the borrow drops).
    ///
    /// The type-stamping `alloc` form (`01b` §3a / `01c` §8), like
    /// [`NodePtr::<KernNode>::alloc`]: it writes `NODE_type` and confines the
    /// family's one `from_idx_unchecked` to this single audited site, keeping
    /// TeX's free-list arena. It is the `get_node(MEDIUM_NODE_SIZE)` +
    /// `type(p) = penalty_node` the C `new_penalty` does; the caller fills
    /// `subtype`/`penalty`.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<PenaltyNode>> {
        let p = get_node(mem, MEDIUM_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        // type-stamp: a penalty node carries NODE_type = PENALTY_NODE.
        mem.set_b16s1(p, PENALTY_NODE);
        // SAFETY: `p` is a freshly allocated `MEDIUM_NODE_SIZE` node just stamped
        // PENALTY_NODE; this is the family's single audited construction site.
        Some(unsafe { NodePtr::<PenaltyNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// `PENALTY_NODE_penalty(p)` = `mem[(p) + 1].b32.s1`.
    ///
    /// **A plain `i32`, not `Scaled`** (`01c` §3) — the breakpoint penalty,
    /// even though it shares the +1.b32.s1 slot with box/kern `width`.
    #[inline]
    pub fn penalty(self, mem: &MemState) -> i32 {
        mem.b32s1(self.idx.offset(1).0)
    }
    #[inline]
    pub fn set_penalty(self, mem: &mut MemState, v: i32) {
        mem.set_b32s1(self.idx.offset(1).0, v);
    }
}

impl MemState {
    /// Classify a known lo-mem node as a penalty node. `Ok(Some(_))` if `p`
    /// is a `PENALTY_NODE`; `Ok(None)` if some other valid type; `Err` if
    /// invalid. `p` must already be a non-null lo-mem index (`01c` §7).
    #[inline]
    pub fn classify_penalty(
        &self,
        p: MemIdx,
    ) -> Result<Option<NodePtr<PenaltyNode>>, InvalidNodeType> {
        Ok(match self.node_type(p)? {
            // SAFETY: the NODE_type branch just proved the kind.
            NodeType::Penalty => Some(unsafe { NodePtr::<PenaltyNode>::from_idx_unchecked(p) }),
            _ => None,
        })
    }
}

// --- the margin kern family: margin kern nodes (NODE_type 40) ---

/// `MARGIN_KERN_NODE` (`NODE_type` = 40): a margin-protrusion kern node.
pub enum MarginKernNode {}

impl NodeKind for MarginKernNode {}

impl NodePtr<MarginKernNode> {
    /// Allocate a fresh margin kern node on the `mem` free-list arena,
    /// type-stamped with `NODE_type = MARGIN_KERN_NODE`, and return a typed
    /// handle to it — or `None` on arena exhaustion.
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<MarginKernNode>> {
        let p = get_node(mem, MARGIN_KERN_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        mem.set_b16s1(p, MARGIN_KERN_NODE);
        Some(unsafe { NodePtr::<MarginKernNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// `MARGIN_KERN_NODE` width = `mem[(p) + 1].b32.s1`.
    #[inline]
    pub fn width(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_width(self, mem: &mut MemState, w: Scaled) {
        mem.set_b32s1(self.idx.offset(1).0, w.0);
    }
}

impl MemState {
    /// Classify a known lo-mem node as a margin kern node.
    #[inline]
    pub fn classify_margin_kern(
        &self,
        p: MemIdx,
    ) -> Result<Option<NodePtr<MarginKernNode>>, InvalidNodeType> {
        Ok(match self.node_type(p)? {
            NodeType::MarginKern => {
                Some(unsafe { NodePtr::<MarginKernNode>::from_idx_unchecked(p) })
            }
            _ => None,
        })
    }
}

// --- mark / adjust families (NODE_type 4, 5) ---

/// `MARK_NODE` (`NODE_type` = 4): a `\mark` whose token list is carried into
/// the page stream. `SMALL_NODE_SIZE` (2). Field offsets verified against
/// `xetex/xetex-xetexd.h:258–259` (`MARK_NODE_ptr` / `MARK_NODE_class`).
pub enum MarkNode {}

impl NodeKind for MarkNode {}
impl HasNodeHeader for MarkNode {}

impl NodePtr<MarkNode> {
    /// Mark token list = `mem[(p) + 1].b32.s1` (`MARK_NODE_ptr`).
    #[inline]
    pub fn mark_ptr(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_mark_ptr(self, mem: &mut MemState, v: MemPtr) {
        mem.set_b32s1(self.idx.offset(1).0, v.0);
    }

    /// Mark class = `mem[(p) + 1].b32.s0` (`MARK_NODE_class`).
    #[inline]
    pub fn mark_class(self, mem: &MemState) -> i32 {
        mem.b32s0(self.idx.offset(1).0)
    }
    #[inline]
    pub fn set_mark_class(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self.idx.offset(1).0, v);
    }
}

/// `ADJUST_NODE` (`NODE_type` = 5): a `\vadjust` list moved to the enclosing
/// vertical list. `SMALL_NODE_SIZE` (2); the subtype selects pre-/post-adjust
/// placement. The adjustment list pointer reuses the box `width` slot
/// (`mem[(p) + 1].b32.s1`).
pub enum AdjustNode {}

impl NodeKind for AdjustNode {}
impl HasNodeHeader for AdjustNode {}

impl NodePtr<AdjustNode> {
    /// Adjustment list = `mem[(p) + 1].b32.s1` (`ADJUST_NODE_ptr`).
    #[inline]
    pub fn adjust_ptr(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_adjust_ptr(self, mem: &mut MemState, v: MemPtr) {
        mem.set_b32s1(self.idx.offset(1).0, v.0);
    }
}

/// `INS_NODE` (`NODE_type` = 3): an insertion (`\insert`). `INS_NODE_SIZE` (5).
/// The subtype holds the insertion class. Field offsets verified against
/// `xetex/xetex-xetexd.h:246–248` (`INSERTION_NODE_float_cost` /
/// `INSERTION_NODE_split_top_ptr` / `INSERTION_NODE_ins_ptr`); height/depth
/// share the box `height`/`depth` slots.
pub enum InsNode {}

impl NodeKind for InsNode {}
impl HasNodeHeader for InsNode {}

impl NodePtr<InsNode> {
    /// Vlist to be inserted = `mem[(p) + 4].b32.s0` (`INSERTION_NODE_ins_ptr`).
    #[inline]
    pub fn ins_ptr(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s0(self.idx.offset(4).0))
    }
    #[inline]
    pub fn set_ins_ptr(self, mem: &mut MemState, v: MemPtr) {
        mem.set_b32s0(self.idx.offset(4).0, v.0);
    }

    /// `\splittopskip` glue spec = `mem[(p) + 4].b32.s1`
    /// (`INSERTION_NODE_split_top_ptr`). Points at a glue spec, not a list.
    #[inline]
    pub fn split_top_ptr(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(4).0))
    }
    #[inline]
    pub fn set_split_top_ptr(self, mem: &mut MemState, v: MemPtr) {
        mem.set_b32s1(self.idx.offset(4).0, v.0);
    }
}

/// `UNSET_NODE` (`NODE_type` = 13): an unset box produced while building an
/// alignment, before column widths are known. It reuses the box layout
/// (`BOX_NODE_SIZE` = 8) for `width`/`depth`/`height`/`list_ptr`, but its `+4`
/// and `+5`/`+6` words hold alignment bookkeeping (`glue_shrink`/`span_count`/
/// `glue_stretch`), so this family deliberately does **not** implement
/// [`HasDimensions`] (its `shift_amount` slot is `glue_shrink` here). Only the
/// box dimension/list fields shared verbatim with hlist/vlist are exposed.
pub enum UnsetNode {}

impl NodeKind for UnsetNode {}
impl HasNodeHeader for UnsetNode {}
impl HasListPtr for UnsetNode {}

impl NodePtr<UnsetNode> {
    /// Natural width = `mem[(p) + 1].b32.s1` (box `width`).
    #[inline]
    pub fn width(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(1).0))
    }
    /// Depth = `mem[(p) + 2].b32.s1` (box `depth`).
    #[inline]
    pub fn depth(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(2).0))
    }
    /// Height = `mem[(p) + 3].b32.s1` (box `height`).
    #[inline]
    pub fn height(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(3).0))
    }
}

// --- shipout movement records ---

const MOVEMENT_NODE_SIZE: i32 = 3;

/// A shipout movement record in the `down_ptr` / `right_ptr` lists.
///
/// These records are not ordinary list nodes: word 0 is both the linked-list
/// successor (`b32.s1`) and movement state (`b32.s0`), with no `NODE_type`.
pub enum MovementNode {}

impl NodeKind for MovementNode {}

impl NodePtr<MovementNode> {
    /// Allocate a fresh movement record (`MOVEMENT_NODE_SIZE` words).
    #[inline]
    pub fn alloc(mem: &mut MemState) -> Option<NodePtr<MovementNode>> {
        let p = get_node(mem, MOVEMENT_NODE_SIZE);
        if p == ALLOC_FAILED {
            return None;
        }
        // SAFETY: `p` is a freshly allocated movement-sized record; it is
        // classified by the shipout movement-list context.
        Some(unsafe { NodePtr::<MovementNode>::from_idx_unchecked(MemIdx::new(p)) })
    }

    /// Convert a nullable movement-list pointer to a typed record handle.
    #[inline]
    pub fn from_ptr(p: MemPtr) -> Option<NodePtr<MovementNode>> {
        p.as_idx()
            .map(|idx| unsafe { NodePtr::<MovementNode>::from_idx_unchecked(idx) })
    }

    /// Free this movement record back to the node arena.
    #[inline]
    pub fn free(self, mem: &mut MemState) {
        free_node(mem, self.idx.0, MOVEMENT_NODE_SIZE);
    }

    /// Movement-list successor = `mem[p].b32.s1`.
    #[inline]
    pub fn link(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.0))
    }
    #[inline]
    pub fn set_link(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.idx.0, q.0);
    }

    /// Movement reuse state = `mem[p].b32.s0`.
    #[inline]
    pub fn state(self, mem: &MemState) -> i32 {
        mem.b32s0(self.idx.0)
    }
    #[inline]
    pub fn set_state(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self.idx.0, v);
    }

    /// Movement distance = `mem[p + 1].b32.s1`.
    #[inline]
    pub fn distance(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_distance(self, mem: &mut MemState, w: Scaled) {
        mem.set_b32s1(self.idx.offset(1).0, w.0);
    }

    /// DVI location where this movement was emitted = `mem[p + 2].b32.s1`.
    #[inline]
    pub fn location(self, mem: &MemState) -> i32 {
        mem.b32s1(self.idx.offset(2).0)
    }
    #[inline]
    pub fn set_location(self, mem: &mut MemState, v: i32) {
        mem.set_b32s1(self.idx.offset(2).0, v);
    }
}

// --- the line-breaker active list: active + delta nodes ---
//
// The line breaker's *active list* (`linebreak.rs`) is a SEPARATE list from
// the main vlist/hlist, so it reuses the `NODE_type` word with its own
// codes: an active node carries `break_type` = `UNHYPHENATED`(0) /
// `HYPHENATED`(1) in `0.b16.s1`, and a delta node carries
// `DELTA_NODE` = `RULE_NODE`(2) there (the context alias documented on
// [`NodeType`]). They are therefore classified by *list context* — you are
// walking the active list — **never** through `NodeType::from_raw`, which
// would (correctly, for the *main* list) map 0/1/2 to HList/VList/Rule. The
// list is headed/terminated by the `LAST_ACTIVE` sentinel; the field
// accessors below are only valid on real active/delta nodes a walk has
// already established (so the constructor is `from_idx_unchecked`, the
// context boundary — there is no `classify_*` for these, by design).

/// `ACTIVE_NODE` (line-breaker active list; `break_type` = `UNHYPHENATED`(0)
/// or `HYPHENATED`(1) lives in the `NODE_type` word, with `fitness` in the
/// subtype word). Allocated `ACTIVE_NODE_SIZE_NORMAL`(3) or `_EXTENDED`(5)
/// — the `shortfall`/`glue` fields only exist on the extended form
/// (`\lastlinefit`). Fields verified against `xetex/xetex-xetexd.h:220-225`.
pub enum ActiveNode {}

impl NodeKind for ActiveNode {}

impl NodePtr<ActiveNode> {
    /// Active-list successor = `mem[p].b32.s1`.
    #[inline]
    pub fn link(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.0))
    }
    #[inline]
    pub fn set_link(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.idx.0, q.0);
    }

    /// `ACTIVE_NODE_fitness(p)` = `mem[p].b16.s0` — the fitness class
    /// (`VERY_LOOSE_FIT`/`LOOSE_FIT`/`DECENT_FIT`/`TIGHT_FIT`), stored in the
    /// node's subtype word.
    #[inline]
    pub fn fitness(self, mem: &MemState) -> u16 {
        mem.b16s0(self.idx.0)
    }
    #[inline]
    pub fn set_fitness(self, mem: &mut MemState, v: u16) {
        mem.set_b16s0(self.idx.0, v);
    }

    /// The break type (`UNHYPHENATED`(0)/`HYPHENATED`(1)), stored in the
    /// `NODE_type` word `mem[p].b16.s1` — an active-list-private use of the
    /// type slot (not a [`NodeType`]; see the module note above).
    #[inline]
    pub fn break_type(self, mem: &MemState) -> u16 {
        mem.b16s1(self.idx.0)
    }
    #[inline]
    pub fn set_break_type(self, mem: &mut MemState, v: u16) {
        mem.set_b16s1(self.idx.0, v);
    }

    /// `ACTIVE_NODE_break_node(p)` = `mem[(p) + 1].b32.s1` (the "rlink" slot)
    /// — the passive node recording this breakpoint (nullable).
    #[inline]
    pub fn break_node(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_break_node(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.idx.offset(1).0, q.0);
    }

    /// `ACTIVE_NODE_line_number(p)` = `mem[(p) + 1].b32.s0` (the "llink"
    /// slot) — the line number this breakpoint would end. A plain `i32`.
    #[inline]
    pub fn line_number(self, mem: &MemState) -> i32 {
        mem.b32s0(self.idx.offset(1).0)
    }
    #[inline]
    pub fn set_line_number(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self.idx.offset(1).0, v);
    }

    /// `ACTIVE_NODE_total_demerits(p)` = `mem[(p) + 2].b32.s1` — the minimum
    /// total demerits to reach this breakpoint. A plain `i32` (demerits, not
    /// a scaled length).
    #[inline]
    pub fn total_demerits(self, mem: &MemState) -> i32 {
        mem.b32s1(self.idx.offset(2).0)
    }
    #[inline]
    pub fn set_total_demerits(self, mem: &mut MemState, v: i32) {
        mem.set_b32s1(self.idx.offset(2).0, v);
    }

    /// `ACTIVE_NODE_shortfall(p)` = `mem[(p) + 3].b32.s1` (the WEB
    /// "active_short"; extended active nodes only) — a [`Scaled`].
    #[inline]
    pub fn shortfall(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(3).0))
    }
    #[inline]
    pub fn set_shortfall(self, mem: &mut MemState, v: Scaled) {
        mem.set_b32s1(self.idx.offset(3).0, v.0);
    }

    /// `ACTIVE_NODE_glue(p)` = `mem[(p) + 4].b32.s1` (extended active nodes
    /// only) — a [`Scaled`].
    #[inline]
    pub fn glue(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(4).0))
    }
    #[inline]
    pub fn set_glue(self, mem: &mut MemState, v: Scaled) {
        mem.set_b32s1(self.idx.offset(4).0, v.0);
    }
}

/// `DELTA_NODE` (line-breaker active list; `DELTA_NODE` = `RULE_NODE`(2) in
/// the `NODE_type` word — the context alias documented on [`NodeType`]).
/// A delta node sits *between* active nodes and records the running
/// width/stretch/shrink change from one breakpoint to the next.
/// `DELTA_NODE_SIZE` = 7. Fields verified against `xetex/xetex-xetexd.h:230-235`;
/// all six are [`Scaled`] glue components (`dwidth`, the four stretch orders,
/// `dshrink`). Context-classified like [`ActiveNode`] (see the module note).
pub enum DeltaNode {}

impl NodeKind for DeltaNode {}

impl NodePtr<DeltaNode> {
    /// Active-list successor = `mem[p].b32.s1`.
    #[inline]
    pub fn link(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.0))
    }
    #[inline]
    pub fn set_link(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.idx.0, q.0);
    }

    /// `DELTA_NODE_dwidth(p)` = `mem[(p) + 1].b32.s1` — the natural-width
    /// difference.
    #[inline]
    pub fn dwidth(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_dwidth(self, mem: &mut MemState, v: Scaled) {
        mem.set_b32s1(self.idx.offset(1).0, v.0);
    }

    /// `DELTA_NODE_dstretch0(p)` = `mem[(p) + 2].b32.s1` — the stretch
    /// difference in points (order 0, `normal`).
    #[inline]
    pub fn dstretch0(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(2).0))
    }
    #[inline]
    pub fn set_dstretch0(self, mem: &mut MemState, v: Scaled) {
        mem.set_b32s1(self.idx.offset(2).0, v.0);
    }

    /// `DELTA_NODE_dstretch1(p)` = `mem[(p) + 3].b32.s1` — stretch difference
    /// in `fil`.
    #[inline]
    pub fn dstretch1(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(3).0))
    }
    #[inline]
    pub fn set_dstretch1(self, mem: &mut MemState, v: Scaled) {
        mem.set_b32s1(self.idx.offset(3).0, v.0);
    }

    /// `DELTA_NODE_dstretch2(p)` = `mem[(p) + 4].b32.s1` — stretch difference
    /// in `fill`.
    #[inline]
    pub fn dstretch2(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(4).0))
    }
    #[inline]
    pub fn set_dstretch2(self, mem: &mut MemState, v: Scaled) {
        mem.set_b32s1(self.idx.offset(4).0, v.0);
    }

    /// `DELTA_NODE_dstretch3(p)` = `mem[(p) + 5].b32.s1` — stretch difference
    /// in `filll`.
    #[inline]
    pub fn dstretch3(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(5).0))
    }
    #[inline]
    pub fn set_dstretch3(self, mem: &mut MemState, v: Scaled) {
        mem.set_b32s1(self.idx.offset(5).0, v.0);
    }

    /// `DELTA_NODE_dshrink(p)` = `mem[(p) + 6].b32.s1` — the shrink
    /// difference.
    #[inline]
    pub fn dshrink(self, mem: &MemState) -> Scaled {
        Scaled(mem.b32s1(self.idx.offset(6).0))
    }
    #[inline]
    pub fn set_dshrink(self, mem: &mut MemState, v: Scaled) {
        mem.set_b32s1(self.idx.offset(6).0, v.0);
    }
}

// --- the passive node: the line-breaker's passive list (PASSIVE_NODE_SIZE=2) ---
//
// A passive node records a feasible breakpoint for the second pass. Like
// [`GlueSpecNode`], it deliberately does **not** implement [`HasNodeHeader`]:
// its `serial` number occupies `0.b32.s0`, which overlaps the `b16`
// type/subtype halves, so a passive node carries no `NODE_type`. It is
// identified by *list context* (it is in the passive list), and its `link`
// (`0.b32.s1`) chains the list — exactly as the C builder writes it
// (`linebreak.rs` post-line-break loop). Fields verified against
// `xetex/xetex-xetexd.h:278-281`.

/// `PASSIVE_NODE`: a recorded feasible breakpoint (the line-breaker passive
/// list). No `NODE_type` (the type/subtype slot is the `serial` number).
pub enum PassiveNode {}

impl NodeKind for PassiveNode {}

impl NodePtr<PassiveNode> {
    /// The list link `mem[p].b32.s1` chaining the passive list (nullable).
    /// (Passive nodes have no `NODE_type`, so this is a concrete impl, not
    /// the [`HasNodeHeader`] `link`.)
    #[inline]
    pub fn link(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.0))
    }
    #[inline]
    pub fn set_link(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.idx.0, q.0);
    }

    /// `PASSIVE_NODE_serial(p)` = `mem[p].b32.s0` (the "info" word) — the
    /// 1-based serial number assigned as breakpoints are discovered.
    #[inline]
    pub fn serial(self, mem: &MemState) -> i32 {
        mem.b32s0(self.idx.0)
    }
    #[inline]
    pub fn set_serial(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self.idx.0, v);
    }

    /// `PASSIVE_NODE_cur_break(p)` = `mem[(p) + 1].b32.s1` (the "rlink"
    /// slot) — the node where this breakpoint occurs (nullable).
    #[inline]
    pub fn cur_break(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s1(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_cur_break(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s1(self.idx.offset(1).0, q.0);
    }

    /// `PASSIVE_NODE_prev_break(p)` = `mem[(p) + 1].b32.s0` (the "llink"
    /// slot) — the previous passive breakpoint on the best path (nullable).
    ///
    /// **Aliased:** the C macro `PASSIVE_NODE_next_break` names the *same*
    /// slot (`#define …_next_break(p) …_prev_break(p)`); use [`next_break`]
    /// for the reading sense. They read/write identical bits.
    ///
    /// [`next_break`]: NodePtr::<PassiveNode>::next_break
    #[inline]
    pub fn prev_break(self, mem: &MemState) -> MemPtr {
        MemPtr::from_raw(mem.b32s0(self.idx.offset(1).0))
    }
    #[inline]
    pub fn set_prev_break(self, mem: &mut MemState, q: MemPtr) {
        mem.set_b32s0(self.idx.offset(1).0, q.0);
    }

    /// `PASSIVE_NODE_next_break(p)` — the C alias of [`prev_break`] (same
    /// `+1.b32.s0` slot), used when the list is walked forward.
    ///
    /// [`prev_break`]: NodePtr::<PassiveNode>::prev_break
    #[inline]
    pub fn next_break(self, mem: &MemState) -> MemPtr {
        self.prev_break(mem)
    }
    #[inline]
    pub fn set_next_break(self, mem: &mut MemState, q: MemPtr) {
        self.set_prev_break(mem, q);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem_types::{B32x2, MemoryWord};
    use std::mem::size_of;

    fn zero_word() -> MemoryWord {
        MemoryWord {
            b32: B32x2 { s0: 0, s1: 0 },
        }
    }

    fn fresh_mem(n: usize) -> MemState {
        MemState {
            data: vec![zero_word(); n],
            lo_mem_max: 0,
            hi_mem_min: 1_000_000, // every test index is lo-mem
            avail: 0,
            mem_end: 0,
            rover: 0,
            var_used: 0,
            dyn_used: 0,
        }
    }

    #[test]
    fn nodeptr_is_zero_cost() {
        assert_eq!(size_of::<NodePtr<HListNode>>(), size_of::<i32>());
        assert_eq!(size_of::<NodePtr<VListNode>>(), size_of::<i32>());
    }

    #[test]
    fn box_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        // tag it an hlist so classify_box finds it
        mem.set_b16s1(p.0, NodeType::HList.as_raw());
        let b = unsafe { NodePtr::<HListNode>::from_idx_unchecked(p) };

        b.set_width(&mut mem, Scaled(0x1111));
        b.set_depth(&mut mem, Scaled(0x2222));
        b.set_height(&mut mem, Scaled(0x3333));
        b.set_shift_amount(&mut mem, Scaled(0x4444));
        b.set_list_ptr(&mut mem, MemPtr::from_raw(150));
        b.set_glue_sign(&mut mem, 2);
        b.set_glue_order(&mut mem, 1);
        b.set_glue_set(&mut mem, 1.5_f64);

        assert_eq!(b.width(&mem), Scaled(0x1111));
        assert_eq!(b.depth(&mem), Scaled(0x2222));
        assert_eq!(b.height(&mem), Scaled(0x3333));
        assert_eq!(b.shift_amount(&mem), Scaled(0x4444));
        assert_eq!(b.list_ptr(&mem), MemPtr::from_raw(150));
        assert_eq!(b.glue_sign(&mem), 2);
        assert_eq!(b.glue_order(&mem), 1);
        assert_eq!(b.glue_set(&mem), 1.5_f64);

        // each field landed in the C-exact raw word/subfield
        assert_eq!(mem.b32s1(p.0 + 1), 0x1111); // width
        assert_eq!(mem.b32s1(p.0 + 2), 0x2222); // depth
        assert_eq!(mem.b32s1(p.0 + 3), 0x3333); // height
        assert_eq!(mem.b32s1(p.0 + 4), 0x4444); // shift_amount
        assert_eq!(mem.b32s1(p.0 + 5), 150); // list_ptr
        assert_eq!(mem.b16s1(p.0 + 5), 2); // glue_sign
        assert_eq!(mem.b16s0(p.0 + 5), 1); // glue_order
    }

    #[test]
    fn empty_box_list_ptr_is_null() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b32s1((p.0) + 5, crate::consts::TEX_NULL);
        let b = unsafe { NodePtr::<HListNode>::from_idx_unchecked(p) };
        assert!(b.list_ptr(&mem).is_null());
        assert_eq!(b.list_ptr(&mem).as_idx(), None);
    }

    #[test]
    fn alloc_hlist_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let b = NodePtr::<HListNode>::alloc(&mut mem).expect("arena has room");
        // alloc type-stamped NODE_type = HLIST_NODE, so it classifies back:
        assert!(matches!(
            mem.classify_box(b.idx()),
            Ok(Some(AnyBox::HList(_)))
        ));
        b.set_subtype(&mut mem, 0);
        b.set_width(&mut mem, Scaled(0x1111));
        b.set_list_ptr(&mut mem, MemPtr::NULL);
        assert_eq!(b.width(&mem), Scaled(0x1111));
        assert!(b.list_ptr(&mem).is_null());
    }

    #[test]
    fn classify_box_dispatches_on_node_type() {
        let mut mem = fresh_mem(200);
        let h = MemIdx::new(40);
        let v = MemIdx::new(60);
        let other = MemIdx::new(80);
        mem.set_b16s1(h.0, NodeType::HList.as_raw());
        mem.set_b16s1(v.0, NodeType::VList.as_raw());
        mem.set_b16s1(other.0, NodeType::Glue.as_raw());

        assert!(matches!(mem.classify_box(h), Ok(Some(AnyBox::HList(_)))));
        assert!(matches!(mem.classify_box(v), Ok(Some(AnyBox::VList(_)))));
        assert!(matches!(mem.classify_box(other), Ok(None))); // valid, not a box

        // an invalid NODE_type code (the 32–39 gap) is an error
        let bad = MemIdx::new(90);
        mem.set_b16s1(bad.0, 35);
        assert!(mem.classify_box(bad).is_err());
    }

    #[test]
    fn classify_dispatches_to_typed_handles() {
        let mut mem = fresh_mem(300);
        let cases: &[(i32, NodeType)] = &[
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
        for &(at, nt) in cases {
            mem.set_b16s1(at, nt.as_raw());
        }
        use AnyNode::*;
        assert!(matches!(mem.classify(MemIdx::new(40)), Ok(HList(_))));
        assert!(matches!(mem.classify(MemIdx::new(50)), Ok(VList(_))));
        assert!(matches!(mem.classify(MemIdx::new(60)), Ok(Rule(_))));
        assert!(matches!(mem.classify(MemIdx::new(70)), Ok(Glue(_))));
        assert!(matches!(mem.classify(MemIdx::new(80)), Ok(Kern(_))));
        assert!(matches!(mem.classify(MemIdx::new(90)), Ok(Math(_))));
        assert!(matches!(mem.classify(MemIdx::new(100)), Ok(Penalty(_))));
        assert!(matches!(mem.classify(MemIdx::new(110)), Ok(Disc(_))));
        assert!(matches!(mem.classify(MemIdx::new(120)), Ok(Choice(_))));
        assert!(matches!(mem.classify(MemIdx::new(130)), Ok(MarginKern(_))));
        // every noad code collapses to the single Noad handle
        assert!(matches!(mem.classify(MemIdx::new(140)), Ok(Noad(_))));
        assert!(matches!(mem.classify(MemIdx::new(150)), Ok(Noad(_))));
        // not-yet-typed families carry the raw index
        assert!(matches!(mem.classify(MemIdx::new(160)), Ok(Whatsit(_))));
        assert!(matches!(mem.classify(MemIdx::new(170)), Ok(Unset(_))));

        // the carried handle/index points back at the same node
        assert_eq!(mem.classify(MemIdx::new(40)).unwrap().idx().0, 40);
        assert_eq!(mem.classify(MemIdx::new(160)).unwrap().idx().0, 160);

        // an invalid NODE_type (the 32–39 gap) is an error
        mem.set_b16s1(180, 35);
        assert!(mem.classify(MemIdx::new(180)).is_err());
    }

    #[test]
    fn rule_dimensions_model_the_running_sentinel() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Rule.as_raw());
        let r = unsafe { NodePtr::<RuleNode>::from_idx_unchecked(p) };

        r.set_width(&mut mem, RuleDimen::Running);
        r.set_depth(&mut mem, RuleDimen::Set(Scaled(0)));
        r.set_height(&mut mem, RuleDimen::Set(Scaled(0x2222)));

        assert_eq!(r.width(&mem), RuleDimen::Running);
        assert_eq!(r.depth(&mem), RuleDimen::Set(Scaled(0)));
        assert_eq!(r.height(&mem), RuleDimen::Set(Scaled(0x2222)));
        assert_eq!(mem.b32s1(p.0 + 1), crate::consts::NULL_FLAG);
        assert_eq!(mem.b32s1(p.0 + 2), 0);
        assert_eq!(mem.b32s1(p.0 + 3), 0x2222);
    }

    #[test]
    fn alloc_rule_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let r = NodePtr::<RuleNode>::alloc(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_rule(r.idx()), Ok(Some(_))));
        r.set_subtype(&mut mem, 0);
        r.set_width(&mut mem, RuleDimen::Running);
        r.set_height(&mut mem, RuleDimen::Set(Scaled(99)));
        assert_eq!(r.width(&mem), RuleDimen::Running);
        assert_eq!(r.height(&mem), RuleDimen::Set(Scaled(99)));
    }

    #[test]
    fn ligature_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Ligature.as_raw());
        let lig = unsafe { NodePtr::<LigatureNode>::from_idx_unchecked(p) };

        lig.set_subtype(&mut mem, 0);
        lig.set_font(&mut mem, FontId(77));
        lig.set_character(&mut mem, 0x1234);
        lig.set_lig_ptr(&mut mem, MemPtr::from_raw(150));

        assert_eq!(lig.subtype(&mem), 0);
        assert_eq!(lig.font(&mem), FontId(77));
        assert_eq!(lig.character(&mem), 0x1234);
        assert_eq!(lig.lig_ptr(&mem), MemPtr::from_raw(150));
        assert_eq!(mem.b16s1(p.0 + 1), 77);
        assert_eq!(mem.b16s0(p.0 + 1), 0x1234);
        assert_eq!(mem.b32s1(p.0 + 1), 150);
    }

    #[test]
    fn alloc_ligature_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let lig = NodePtr::<LigatureNode>::alloc(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_ligature(lig.idx()), Ok(Some(_))));
        lig.set_font(&mut mem, FontId(5));
        lig.set_character(&mut mem, 42);
        lig.set_lig_ptr(&mut mem, MemPtr::NULL);
        assert_eq!(lig.font(&mem), FontId(5));
        assert_eq!(lig.character(&mem), 42);
        assert!(lig.lig_ptr(&mem).is_null());
    }

    #[test]
    fn disc_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Disc.as_raw());
        let disc = unsafe { NodePtr::<DiscNode>::from_idx_unchecked(p) };

        disc.set_subtype(&mut mem, 0);
        disc.set_pre_break(&mut mem, MemPtr::from_raw(150));
        disc.set_post_break(&mut mem, MemPtr::from_raw(160));

        assert_eq!(disc.subtype(&mem), 0);
        assert_eq!(disc.pre_break(&mem), MemPtr::from_raw(150));
        assert_eq!(disc.post_break(&mem), MemPtr::from_raw(160));
        assert_eq!(mem.b32s0(p.0 + 1), 150);
        assert_eq!(mem.b32s1(p.0 + 1), 160);
    }

    #[test]
    fn alloc_disc_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let disc = NodePtr::<DiscNode>::alloc(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_disc(disc.idx()), Ok(Some(_))));
        disc.set_pre_break(&mut mem, MemPtr::NULL);
        disc.set_post_break(&mut mem, MemPtr::NULL);
        assert!(disc.pre_break(&mem).is_null());
        assert!(disc.post_break(&mem).is_null());
    }

    #[test]
    fn math_node_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Math.as_raw());
        let math = unsafe { NodePtr::<MathNode>::from_idx_unchecked(p) };

        math.set_subtype(&mut mem, 3);
        math.set_width(&mut mem, Scaled(0x1234));

        assert_eq!(math.subtype(&mem), 3);
        assert_eq!(math.width(&mem), Scaled(0x1234));
        assert_eq!(mem.b16s0(p.0), 3);
        assert_eq!(mem.b32s1(p.0 + 1), 0x1234);
    }

    #[test]
    fn alloc_math_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let math = NodePtr::<MathNode>::alloc(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_math(math.idx()), Ok(Some(_))));
        math.set_subtype(&mut mem, 1);
        math.set_width(&mut mem, Scaled(-99));
        assert_eq!(math.subtype(&mem), 1);
        assert_eq!(math.width(&mem), Scaled(-99));
    }

    #[test]
    fn style_node_constructor_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Style.as_raw());
        let style = unsafe { NodePtr::<StyleNode>::from_idx_unchecked(p) };

        style.set_subtype(&mut mem, 4);
        style.clear_payload(&mut mem);

        assert_eq!(style.subtype(&mem), 4);
        assert_eq!(mem.b16s0(p.0), 4);
        assert_eq!(mem.b32s1(p.0 + 1), 0);
        assert_eq!(mem.b32s1(p.0 + 2), 0);
    }

    #[test]
    fn alloc_style_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let style = NodePtr::<StyleNode>::alloc(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_style(style.idx()), Ok(Some(_))));
        style.set_subtype(&mut mem, 2);
        style.clear_payload(&mut mem);
        assert_eq!(style.subtype(&mem), 2);
    }

    #[test]
    fn choice_node_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Choice.as_raw());
        let choice = unsafe { NodePtr::<ChoiceNode>::from_idx_unchecked(p) };

        choice.set_subtype(&mut mem, 0);
        choice.set_display_mlist(&mut mem, MemPtr::from_raw(11));
        choice.set_text_mlist(&mut mem, MemPtr::from_raw(22));
        choice.set_script_mlist(&mut mem, MemPtr::from_raw(33));
        choice.set_script_script_mlist(&mut mem, MemPtr::from_raw(44));

        assert_eq!(choice.subtype(&mem), 0);
        assert_eq!(choice.display_mlist(&mem), MemPtr::from_raw(11));
        assert_eq!(choice.text_mlist(&mem), MemPtr::from_raw(22));
        assert_eq!(choice.script_mlist(&mem), MemPtr::from_raw(33));
        assert_eq!(choice.script_script_mlist(&mem), MemPtr::from_raw(44));
        assert_eq!(mem.b32s0(p.0 + 1), 11);
        assert_eq!(mem.b32s1(p.0 + 1), 22);
        assert_eq!(mem.b32s0(p.0 + 2), 33);
        assert_eq!(mem.b32s1(p.0 + 2), 44);
    }

    #[test]
    fn alloc_choice_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let choice = NodePtr::<ChoiceNode>::alloc(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_choice(choice.idx()), Ok(Some(_))));
        choice.set_display_mlist(&mut mem, MemPtr::NULL);
        choice.set_text_mlist(&mut mem, MemPtr::NULL);
        choice.set_script_mlist(&mut mem, MemPtr::NULL);
        choice.set_script_script_mlist(&mut mem, MemPtr::NULL);
        assert!(choice.display_mlist(&mem).is_null());
        assert!(choice.text_mlist(&mem).is_null());
        assert!(choice.script_mlist(&mem).is_null());
        assert!(choice.script_script_mlist(&mem).is_null());
    }

    #[test]
    fn mark_node_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Mark.as_raw());
        let m = match mem.classify(p) {
            Ok(AnyNode::Mark(m)) => m,
            other => panic!("expected Mark, got {other:?}"),
        };
        m.set_mark_ptr(&mut mem, MemPtr::from_raw(77));
        m.set_mark_class(&mut mem, 3);
        assert_eq!(m.mark_ptr(&mem), MemPtr::from_raw(77));
        assert_eq!(m.mark_class(&mem), 3);
        assert_eq!(mem.b32s1(p.0 + 1), 77);
        assert_eq!(mem.b32s0(p.0 + 1), 3);
    }

    #[test]
    fn adjust_node_ptr_roundtrips_at_the_right_offset() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Adjust.as_raw());
        let a = match mem.classify(p) {
            Ok(AnyNode::Adjust(a)) => a,
            other => panic!("expected Adjust, got {other:?}"),
        };
        a.set_adjust_ptr(&mut mem, MemPtr::from_raw(88));
        assert_eq!(a.adjust_ptr(&mem), MemPtr::from_raw(88));
        assert_eq!(mem.b32s1(p.0 + 1), 88);
    }

    #[test]
    fn ins_node_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Ins.as_raw());
        let ins = match mem.classify(p) {
            Ok(AnyNode::Ins(ins)) => ins,
            other => panic!("expected Ins, got {other:?}"),
        };
        ins.set_ins_ptr(&mut mem, MemPtr::from_raw(55));
        ins.set_split_top_ptr(&mut mem, MemPtr::from_raw(66));
        assert_eq!(ins.ins_ptr(&mem), MemPtr::from_raw(55));
        assert_eq!(ins.split_top_ptr(&mem), MemPtr::from_raw(66));
        assert_eq!(mem.b32s0(p.0 + 4), 55);
        assert_eq!(mem.b32s1(p.0 + 4), 66);
    }

    #[test]
    fn unset_node_box_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Unset.as_raw());
        let u = match mem.classify(p) {
            Ok(AnyNode::Unset(u)) => u,
            other => panic!("expected Unset, got {other:?}"),
        };
        mem.set_b32s1(p.0 + 1, 0x111);
        mem.set_b32s1(p.0 + 2, 0x222);
        mem.set_b32s1(p.0 + 3, 0x333);
        u.set_list_ptr(&mut mem, MemPtr::from_raw(0x444));
        assert_eq!(u.width(&mem), Scaled(0x111));
        assert_eq!(u.depth(&mem), Scaled(0x222));
        assert_eq!(u.height(&mem), Scaled(0x333));
        assert_eq!(u.list_ptr(&mem), MemPtr::from_raw(0x444));
        assert_eq!(mem.b32s1(p.0 + 5), 0x444);
    }

    #[test]
    fn glue_family_is_zero_cost() {
        assert_eq!(size_of::<NodePtr<GlueNode>>(), size_of::<i32>());
        assert_eq!(size_of::<NodePtr<GlueSpecNode>>(), size_of::<i32>());
    }

    #[test]
    fn glue_node_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Glue.as_raw());
        let g = unsafe { NodePtr::<GlueNode>::from_idx_unchecked(p) };

        g.set_subtype(&mut mem, 7);
        g.set_glue_ptr(&mut mem, MemPtr::from_raw(150));
        g.set_leader_ptr(&mut mem, MemPtr::from_raw(160));

        assert_eq!(g.subtype(&mem), 7);
        assert_eq!(g.glue_ptr(&mem), MemPtr::from_raw(150));
        assert_eq!(g.leader_ptr(&mem), MemPtr::from_raw(160));

        // each field landed in the C-exact raw word/subfield:
        // subtype  -> mem[p].b16.s0
        // glue_ptr -> mem[p+1].b32.s0  (note: the *s0* half, not s1)
        // leader_ptr -> mem[p+1].b32.s1
        assert_eq!(mem.b16s0(p.0), 7);
        assert_eq!(mem.b32s0(p.0 + 1), 150);
        assert_eq!(mem.b32s1(p.0 + 1), 160);
    }

    #[test]
    fn alloc_glue_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let g = NodePtr::<GlueNode>::alloc(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_glue(g.idx()), Ok(Some(_))));
        g.set_subtype(&mut mem, 0);
        g.set_glue_ptr(&mut mem, MemPtr::from_raw(22));
        g.set_leader_ptr(&mut mem, MemPtr::NULL);
        assert_eq!(g.glue_ptr(&mem), MemPtr::from_raw(22));
        assert!(g.leader_ptr(&mem).is_null());
    }

    #[test]
    fn glue_spec_fields_roundtrip_and_ref_count_is_not_a_link() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        let s = unsafe { NodePtr::<GlueSpecNode>::from_idx_unchecked(p) };

        s.set_ref_count(&mut mem, 42);
        s.set_stretch_order(&mut mem, 2);
        s.set_shrink_order(&mut mem, 1);
        s.set_width(&mut mem, Scaled(0x4444));
        s.set_stretch(&mut mem, Scaled(0x5555));
        s.set_shrink(&mut mem, Scaled(0x6666));

        assert_eq!(s.ref_count(&mem), 42);
        assert_eq!(s.stretch_order(&mem), 2);
        assert_eq!(s.shrink_order(&mem), 1);
        assert_eq!(s.width(&mem), Scaled(0x4444));
        assert_eq!(s.stretch(&mem), Scaled(0x5555));
        assert_eq!(s.shrink(&mem), Scaled(0x6666));

        // The ref_count occupies the 0.b32.s1 word — the slot that holds the
        // link/successor on a real list node — but here it is a count. The
        // box `list_ptr`-style `MemPtr` reading of this word would be a parity
        // bug; confirm the raw word holds the count value verbatim.
        assert_eq!(mem.b32s1(p.0), 42);
        assert_eq!(mem.b16s1(p.0), 2); // stretch_order
        assert_eq!(mem.b16s0(p.0), 1); // shrink_order
        assert_eq!(mem.b32s1(p.0 + 1), 0x4444); // width
        assert_eq!(mem.b32s1(p.0 + 2), 0x5555); // stretch
        assert_eq!(mem.b32s1(p.0 + 3), 0x6666); // shrink
    }

    #[test]
    fn glue_ptr_classifies_to_a_spec_handle_and_handles_null() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Glue.as_raw());
        let g = unsafe { NodePtr::<GlueNode>::from_idx_unchecked(p) };

        // a real glue_ptr classifies to a spec handle at the same index
        g.set_glue_ptr(&mut mem, MemPtr::from_raw(150));
        let spec = g.glue_spec(&mem).expect("non-null glue_ptr");
        assert_eq!(spec.idx(), MemIdx::new(150));
        assert_eq!(
            NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(150)).map(|s| s.idx()),
            Some(MemIdx::new(150))
        );

        // a null glue_ptr classifies to None (a caller bug in practice)
        g.set_glue_ptr(&mut mem, MemPtr::NULL);
        assert!(g.glue_spec(&mem).is_none());
        assert!(NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::NULL).is_none());
    }

    #[test]
    fn alloc_glue_spec_carves_a_usable_typed_handle() {
        let mut mem = fresh_mem(64);
        // Build a single free block spanning [10, 30) on the variable-size
        // free list: header `.s0` = block size, `.s1` = MAX_HALFWORD (the free
        // marker); the block is its own left/right neighbour (rover → self).
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60; // no growth needed; keeps the carved node in bounds
        mem.set_b32s0(rover, 20); // block size 20 → [10, 30)
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD); // free
        mem.set_b32s1(rover + 1, rover); // rlink → self
        mem.set_b32s0(rover + 1, rover); // llink → self
        mem.set_b32s1(30, 0); // word past the block is not a free marker (stop coalescing)

        let spec = NodePtr::<GlueSpecNode>::alloc(&mut mem).expect("arena has room");
        // the returned handle is a real, in-bounds, writable glue_spec
        spec.set_width(&mut mem, Scaled(0x1234));
        spec.set_stretch(&mut mem, Scaled(0x5678));
        assert_eq!(spec.width(&mem), Scaled(0x1234));
        assert_eq!(spec.stretch(&mem), Scaled(0x5678));
        // and `alloc` confined the construction — no `unsafe` at this call site
    }

    #[test]
    fn classify_glue_dispatches_on_node_type() {
        let mut mem = fresh_mem(200);
        let g = MemIdx::new(40);
        let other = MemIdx::new(60);
        mem.set_b16s1(g.0, NodeType::Glue.as_raw());
        mem.set_b16s1(other.0, NodeType::HList.as_raw());

        assert!(matches!(mem.classify_glue(g), Ok(Some(_))));
        assert!(matches!(mem.classify_glue(other), Ok(None))); // valid, not glue

        let bad = MemIdx::new(80);
        mem.set_b16s1(bad.0, 37); // the 32–39 gap
        assert!(mem.classify_glue(bad).is_err());
    }

    #[test]
    fn kern_is_zero_cost() {
        assert_eq!(size_of::<NodePtr<KernNode>>(), size_of::<i32>());
    }

    #[test]
    fn kern_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Kern.as_raw());
        let k = unsafe { NodePtr::<KernNode>::from_idx_unchecked(p) };

        k.set_subtype(&mut mem, 3);
        k.set_width(&mut mem, Scaled(0x7777));

        assert_eq!(k.subtype(&mem), 3);
        assert_eq!(k.width(&mem), Scaled(0x7777));

        // each field landed in the C-exact raw word/subfield
        assert_eq!(mem.b16s0(p.0), 3); // subtype
        assert_eq!(mem.b32s1(p.0 + 1), 0x7777); // width
    }

    #[test]
    fn alloc_kern_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        // single free block [10, 30) on the variable-size free list (see
        // `alloc_glue_spec_carves_a_usable_typed_handle` for the layout).
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let k = NodePtr::<KernNode>::alloc(&mut mem).expect("arena has room");
        // alloc type-stamped NODE_type = KERN_NODE (the type-stamping form,
        // unlike glue_spec::alloc which stamps nothing) — so it classifies back:
        assert!(matches!(mem.classify_kern(k.idx()), Ok(Some(_))));
        // and the handle is a real, in-bounds, writable kern node
        k.set_subtype(&mut mem, 0);
        k.set_width(&mut mem, Scaled(0x2468));
        assert_eq!(k.width(&mem), Scaled(0x2468));
    }

    #[test]
    fn classify_kern_dispatches_on_node_type() {
        let mut mem = fresh_mem(200);
        let k = MemIdx::new(40);
        let other = MemIdx::new(60);
        mem.set_b16s1(k.0, NodeType::Kern.as_raw());
        mem.set_b16s1(other.0, NodeType::Glue.as_raw());

        assert!(matches!(mem.classify_kern(k), Ok(Some(_))));
        assert!(matches!(mem.classify_kern(other), Ok(None))); // valid, not kern

        let bad = MemIdx::new(80);
        mem.set_b16s1(bad.0, 38); // the 32–39 gap
        assert!(mem.classify_kern(bad).is_err());
    }

    #[test]
    fn penalty_is_zero_cost() {
        assert_eq!(size_of::<NodePtr<PenaltyNode>>(), size_of::<i32>());
    }

    #[test]
    fn penalty_roundtrips_as_i32_not_scaled() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::Penalty.as_raw());
        let pen = unsafe { NodePtr::<PenaltyNode>::from_idx_unchecked(p) };

        // a negative penalty (eject-ish) round-trips with i32 sign semantics
        pen.set_penalty(&mut mem, -10000);
        assert_eq!(pen.penalty(&mem), -10000);
        // the value lands verbatim in the +1.b32.s1 slot
        assert_eq!(mem.b32s1(p.0 + 1), -10000);

        // the return type is i32, NOT Scaled — this would not type-check if
        // `penalty()` returned `Scaled` (the trap encoded in the type):
        let v: i32 = pen.penalty(&mem);
        assert_eq!(v, -10000);
    }

    #[test]
    fn alloc_penalty_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let pen = NodePtr::<PenaltyNode>::alloc(&mut mem).expect("arena has room");
        // alloc type-stamped NODE_type = PENALTY_NODE, so it classifies back:
        assert!(matches!(mem.classify_penalty(pen.idx()), Ok(Some(_))));
        pen.set_subtype(&mut mem, 0);
        pen.set_penalty(&mut mem, -10000);
        assert_eq!(pen.penalty(&mem), -10000);
    }

    #[test]
    fn classify_penalty_dispatches_on_node_type() {
        let mut mem = fresh_mem(200);
        let pen = MemIdx::new(40);
        let other = MemIdx::new(60);
        mem.set_b16s1(pen.0, NodeType::Penalty.as_raw());
        mem.set_b16s1(other.0, NodeType::Kern.as_raw());

        assert!(matches!(mem.classify_penalty(pen), Ok(Some(_))));
        assert!(matches!(mem.classify_penalty(other), Ok(None))); // valid, not penalty

        let bad = MemIdx::new(80);
        mem.set_b16s1(bad.0, 39); // the 32–39 gap
        assert!(mem.classify_penalty(bad).is_err());
    }

    #[test]
    fn noad_is_zero_cost() {
        assert_eq!(size_of::<NodePtr<Noad>>(), size_of::<i32>());
    }

    #[test]
    fn alloc_noad_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let noad = NodePtr::<Noad>::alloc(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_noad(noad.idx()), Ok(Some(_))));
        noad.set_subtype(&mut mem, 5);
        noad.set_nucleus_type(&mut mem, 1);
        noad.set_nucleus_field(&mut mem, 123);
        assert_eq!(noad.subtype(&mem), 5);
        assert_eq!(noad.nucleus_type(&mem), 1);
        assert_eq!(noad.nucleus_field(&mem), 123);

        noad.set_nucleus_char(&mut mem, 45);
        noad.set_nucleus_fam(&mut mem, 2);
        assert_eq!(noad.nucleus_char(&mem), 45);
        assert_eq!(noad.nucleus_fam(&mem), 2);

        noad.set_subscr_char(&mut mem, 46);
        noad.set_subscr_fam(&mut mem, 3);
        assert_eq!(noad.subscr_char(&mem), 46);
        assert_eq!(noad.subscr_fam(&mem), 3);

        noad.set_supscr_char(&mut mem, 47);
        noad.set_supscr_fam(&mut mem, 4);
        assert_eq!(noad.supscr_char(&mem), 47);
        assert_eq!(noad.supscr_fam(&mem), 4);

        noad.set_accent_char_val(&mut mem, 48);
        noad.set_accent_char_fam(&mut mem, 5);
        assert_eq!(noad.accent_char_val(&mem), 48);
        assert_eq!(noad.accent_char_fam(&mem), 5);

        noad.set_link(&mut mem, MemPtr::from_raw(1234));
        assert_eq!(noad.link(&mem), MemPtr::from_raw(1234));

        let radical = NodePtr::<Noad>::alloc_radical(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_noad(radical.idx()), Ok(Some(_))));

        let accent = NodePtr::<Noad>::alloc_accent(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_noad(accent.idx()), Ok(Some(_))));

        let fraction = NodePtr::<Noad>::alloc_fraction(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_noad(fraction.idx()), Ok(Some(_))));
    }

    #[test]
    fn lig_item_is_zero_cost() {
        assert_eq!(size_of::<NodePtr<LigItem>>(), size_of::<i32>());
    }

    #[test]
    fn alloc_lig_item_carves_a_usable_typed_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let item = NodePtr::<LigItem>::alloc(&mut mem).expect("arena has room");
        item.set_character(&mut mem, 45);
        item.set_link(&mut mem, MemPtr::from_raw(12));
        item.set_component(&mut mem, MemPtr::from_raw(34));

        assert_eq!(item.character(&mem), 45);
        assert_eq!(item.link(&mem), MemPtr::from_raw(12));
        assert_eq!(item.component(&mem), MemPtr::from_raw(34));
    }

    #[test]
    fn margin_kern_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        mem.set_b16s1(p.0, NodeType::MarginKern.as_raw());
        let k = unsafe { NodePtr::<MarginKernNode>::from_idx_unchecked(p) };

        k.set_subtype(&mut mem, 1);
        k.set_width(&mut mem, Scaled(0x1234));

        assert_eq!(k.subtype(&mem), 1);
        assert_eq!(k.width(&mem), Scaled(0x1234));
        assert_eq!(mem.b16s0(p.0), 1);
        assert_eq!(mem.b32s1(p.0 + 1), 0x1234);
    }

    #[test]
    fn alloc_margin_kern_type_stamps_and_carves_a_usable_handle() {
        let mut mem = fresh_mem(64);
        let rover = 10;
        mem.rover = rover;
        mem.lo_mem_max = 30;
        mem.hi_mem_min = 60;
        mem.set_b32s0(rover, 20);
        mem.set_b32s1(rover, crate::consts::MAX_HALFWORD);
        mem.set_b32s1(rover + 1, rover);
        mem.set_b32s0(rover + 1, rover);
        mem.set_b32s1(30, 0);

        let k = NodePtr::<MarginKernNode>::alloc(&mut mem).expect("arena has room");
        assert!(matches!(mem.classify_margin_kern(k.idx()), Ok(Some(_))));
        k.set_subtype(&mut mem, 2);
        k.set_width(&mut mem, Scaled(-123));
        assert_eq!(k.subtype(&mem), 2);
        assert_eq!(k.width(&mem), Scaled(-123));
    }

    #[test]
    fn active_node_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        // No classify boundary: active/delta nodes are context-classified
        // (the active-list walk established the kind).
        let a = unsafe { NodePtr::<ActiveNode>::from_idx_unchecked(p) };

        a.set_break_type(&mut mem, 1); // HYPHENATED, in the NODE_type word
        a.set_fitness(&mut mem, 2); // DECENT_FIT, in the subtype word
        a.set_break_node(&mut mem, MemPtr::from_raw(150));
        a.set_line_number(&mut mem, 7);
        a.set_total_demerits(&mut mem, 0x5151);
        a.set_shortfall(&mut mem, Scaled(-0x20));
        a.set_glue(&mut mem, Scaled(0x30));

        assert_eq!(a.break_type(&mem), 1);
        assert_eq!(a.fitness(&mem), 2);
        assert_eq!(a.break_node(&mem), MemPtr::from_raw(150));
        assert_eq!(a.line_number(&mem), 7);
        assert_eq!(a.total_demerits(&mem), 0x5151);
        assert_eq!(a.shortfall(&mem), Scaled(-0x20));
        assert_eq!(a.glue(&mem), Scaled(0x30));

        // C-exact raw word/subfield placement (xetex-xetexd.h:220-225).
        assert_eq!(mem.b16s1(p.0), 1); // break_type (type word)
        assert_eq!(mem.b16s0(p.0), 2); // fitness (subtype word)
        assert_eq!(mem.b32s1(p.0 + 1), 150); // break_node (rlink)
        assert_eq!(mem.b32s0(p.0 + 1), 7); // line_number (llink)
        assert_eq!(mem.b32s1(p.0 + 2), 0x5151); // total_demerits
        assert_eq!(mem.b32s1(p.0 + 3), -0x20); // shortfall
        assert_eq!(mem.b32s1(p.0 + 4), 0x30); // glue
    }

    #[test]
    fn delta_node_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        let d = unsafe { NodePtr::<DeltaNode>::from_idx_unchecked(p) };

        d.set_dwidth(&mut mem, Scaled(0x11));
        d.set_dstretch0(&mut mem, Scaled(0x22));
        d.set_dstretch1(&mut mem, Scaled(0x33));
        d.set_dstretch2(&mut mem, Scaled(0x44));
        d.set_dstretch3(&mut mem, Scaled(0x55));
        d.set_dshrink(&mut mem, Scaled(-0x66));

        assert_eq!(d.dwidth(&mem), Scaled(0x11));
        assert_eq!(d.dstretch0(&mem), Scaled(0x22));
        assert_eq!(d.dstretch1(&mem), Scaled(0x33));
        assert_eq!(d.dstretch2(&mem), Scaled(0x44));
        assert_eq!(d.dstretch3(&mem), Scaled(0x55));
        assert_eq!(d.dshrink(&mem), Scaled(-0x66));

        // C-exact placement: all six are b32.s1 at +1..+6 (xetex-xetexd.h:230-235).
        assert_eq!(mem.b32s1(p.0 + 1), 0x11);
        assert_eq!(mem.b32s1(p.0 + 2), 0x22);
        assert_eq!(mem.b32s1(p.0 + 3), 0x33);
        assert_eq!(mem.b32s1(p.0 + 4), 0x44);
        assert_eq!(mem.b32s1(p.0 + 5), 0x55);
        assert_eq!(mem.b32s1(p.0 + 6), -0x66);
    }

    #[test]
    fn passive_node_fields_roundtrip_at_the_right_offsets() {
        let mut mem = fresh_mem(200);
        let p = MemIdx::new(100);
        let pn = unsafe { NodePtr::<PassiveNode>::from_idx_unchecked(p) };

        pn.set_link(&mut mem, MemPtr::from_raw(90));
        pn.set_serial(&mut mem, 5);
        pn.set_cur_break(&mut mem, MemPtr::from_raw(150));
        pn.set_prev_break(&mut mem, MemPtr::from_raw(80));

        assert_eq!(pn.link(&mem), MemPtr::from_raw(90));
        assert_eq!(pn.serial(&mem), 5);
        assert_eq!(pn.cur_break(&mem), MemPtr::from_raw(150));
        assert_eq!(pn.prev_break(&mem), MemPtr::from_raw(80));
        // next_break aliases prev_break (same +1.b32.s0 slot).
        assert_eq!(pn.next_break(&mem), MemPtr::from_raw(80));

        // C-exact placement (xetex-xetexd.h:278-281).
        assert_eq!(mem.b32s1(p.0), 90); // link
        assert_eq!(mem.b32s0(p.0), 5); // serial (info word)
        assert_eq!(mem.b32s1(p.0 + 1), 150); // cur_break (rlink)
        assert_eq!(mem.b32s0(p.0 + 1), 80); // prev_break (llink)

        // the alias setter writes the same slot
        pn.set_next_break(&mut mem, MemPtr::from_raw(70));
        assert_eq!(pn.prev_break(&mem), MemPtr::from_raw(70));
        assert_eq!(mem.b32s0(p.0 + 1), 70);
    }
}
