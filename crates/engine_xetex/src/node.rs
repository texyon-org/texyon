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

impl HasDimensions for HListNode {}
impl HasDimensions for VListNode {}
impl HasListPtr for HListNode {}
impl HasListPtr for VListNode {}
impl HasGlueSet for HListNode {}
impl HasGlueSet for VListNode {}

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
}
