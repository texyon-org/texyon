//! Typed views of fixed-size nodes, read and written as `mem[p].field`.
//!
//! A layout struct lies over a node's words in the flat `mem` arena. Indexing
//! `mem` with a [`NodePtr`] borrows the node as its layout, and a mutable
//! borrow first notes the node's words in the journal.

use bytemuck::{Pod, Zeroable};

use crate::node::{FixedNode, GlueSpecNode, NodePtr, Stored};
use crate::state::MemState;
use crate::units::{GlueOrder, Scaled};

#[cfg(not(target_endian = "little"))]
compile_error!("node layouts assume little-endian memory words");

/// A node kind whose words read as one `#[repr(C)]` layout struct.
pub trait Overlay: FixedNode {
    /// The struct laid over the node's `SIZE` words.
    type Layout: Pod;
}

impl<K: Overlay> std::ops::Index<NodePtr<K>> for MemState {
    type Output = K::Layout;

    #[inline]
    fn index(&self, p: NodePtr<K>) -> &K::Layout {
        bytemuck::from_bytes(bytemuck::cast_slice(&self.data[p.idx().words(K::SIZE)]))
    }
}

impl<K: Overlay> std::ops::IndexMut<NodePtr<K>> for MemState {
    #[inline]
    fn index_mut(&mut self, p: NodePtr<K>) -> &mut K::Layout {
        let words = p.idx().words(K::SIZE);
        if self.journal.armed() {
            for index in words.clone() {
                self.journal.note(index, self.data[index]);
            }
        }
        bytemuck::from_bytes_mut(bytemuck::cast_slice_mut(&mut self.data[words]))
    }
}

/// Declares the layout of a fixed node kind and checks it covers the node exactly.
macro_rules! overlay {
    ($kind:ty => $layout:ty) => {
        impl Overlay for $kind {
            type Layout = $layout;
        }
        const _: () = assert!(
            std::mem::size_of::<$layout>()
                == std::mem::size_of::<crate::mem_types::MemoryWord>()
                    * <$kind as FixedNode>::SIZE as usize
        );
    };
}

/// A glue specification. Its first word holds the fill orders and a reference
/// count in place of a type code and link.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GlueSpec {
    pub shrink_order: Stored<GlueOrder>,
    pub stretch_order: Stored<GlueOrder>,
    /// The reference count, `TEX_NULL` for a single reference.
    pub ref_count: i32,
    _unused1: i32,
    pub width: Scaled,
    _unused2: i32,
    pub stretch: Scaled,
    _unused3: i32,
    pub shrink: Scaled,
}

overlay!(GlueSpecNode => GlueSpec);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::MemIdx;

    fn arena() -> MemState {
        MemState {
            data: vec![Default::default(); 16],
            ..Default::default()
        }
    }

    fn spec_at(idx: i32) -> NodePtr<GlueSpecNode> {
        NodePtr::from_idx_unchecked(MemIdx::from_raw(idx))
    }

    #[test]
    fn glue_spec_fields_land_on_their_words() {
        let mut mem = arena();
        let s = spec_at(4);
        let spec = &mut mem[s];
        spec.shrink_order = GlueOrder::Fil.into();
        spec.stretch_order = GlueOrder::Fill.into();
        spec.ref_count = 7;
        spec.width = Scaled::from_raw(11);
        spec.stretch = Scaled::from_raw(12);
        spec.shrink = Scaled::from_raw(13);
        let at = MemIdx::from_raw;
        assert_eq!((mem.b16s0(at(4)), mem.b16s1(at(4))), (1, 2));
        assert_eq!([4, 5, 6, 7].map(|i| mem.b32s1(at(i))), [7, 11, 12, 13]);
        assert_eq!([3, 5, 6, 7, 8].map(|i| mem.b32s0(at(i))), [0; 5]);
        assert_eq!((mem.b32s1(at(3)), mem.b32s1(at(8))), (0, 0));
    }

    #[test]
    fn a_mutable_borrow_journals_every_word_of_the_node() {
        let mut mem = arena();
        let s = spec_at(4);
        mem[s].shrink = Scaled::from_raw(5);
        mem.journal.arm(16);
        let spec = &mut mem[s];
        spec.ref_count = 1;
        spec.width = Scaled::from_raw(2);
        spec.stretch = Scaled::from_raw(3);
        spec.shrink = Scaled::from_raw(4);
        let delta = mem.journal.seal();
        delta.apply_to(&mut mem.data);
        let spec = mem[s];
        assert_eq!(
            (spec.ref_count, spec.width, spec.stretch, spec.shrink),
            (0, Scaled::ZERO, Scaled::ZERO, Scaled::from_raw(5))
        );
    }
}
