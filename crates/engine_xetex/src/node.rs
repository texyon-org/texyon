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
    BOX_NODE_SIZE, GLUE_SPEC_SIZE, HLIST_NODE, KERN_NODE, MEDIUM_NODE_SIZE, PENALTY_NODE,
};
use crate::mem::{get_node, ALLOC_FAILED};
use crate::state::MemState;
use crate::units::{InvalidNodeType, MemIdx, MemPtr, NodeType, Scaled};

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
impl HasNodeHeader for GlueNode {}
impl HasNodeHeader for KernNode {}
impl HasNodeHeader for PenaltyNode {}

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

impl MemState {
    /// Read the `NODE_type` (`mem[p].b16.s1`) of a lo-mem node as a checked
    /// [`NodeType`]. `p` must be a real lo-mem node index (the caller did
    /// the null / `is_char_node` check at the list-walk boundary, `01c` §7).
    #[inline]
    pub fn node_type(&self, p: MemIdx) -> Result<NodeType, InvalidNodeType> {
        NodeType::from_raw(self.b16s1(p.0))
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
}

// --- the glue family: glue nodes and glue_spec records ---
//
// Two distinct node shapes that are always reached together:
//
// `GlueNode` (`NODE_type` = `GLUE_NODE` = 10, a `SMALL_NODE_SIZE` = 2 node):
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
}
