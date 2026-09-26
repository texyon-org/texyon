//! Owned shaped-glyph payloads for native-word nodes.
//!
//! A native node stores a one-based handle in `mem[p + 5].b32.s0`. Zero means
//! no shaped glyphs. Each node owns one arena slot, while copied nodes may
//! share the immutable payload through `Arc` until one is mutated.

use std::sync::Arc;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FixedPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct GlyphBlob {
    pub locations: Vec<FixedPoint>,
    pub gids: Vec<u16>,
}

#[derive(Clone, Default)]
pub(crate) struct GlyphBlobArena {
    slots: Vec<Option<Arc<GlyphBlob>>>,
    free: Vec<usize>,
}

impl GlyphBlobArena {
    pub(crate) fn restore_from(&mut self, source: &Self) {
        self.slots.clone_from(&source.slots);
        self.free.clone_from(&source.free);
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.slots.capacity() * std::mem::size_of::<Option<Arc<GlyphBlob>>>()
            + self.free.capacity() * std::mem::size_of::<usize>()
    }

    pub(crate) fn slots(&self) -> impl Iterator<Item = Option<&GlyphBlob>> {
        self.slots.iter().map(|slot| slot.as_deref())
    }

    pub fn insert(&mut self, blob: GlyphBlob) -> i32 {
        self.adopt(Arc::new(blob))
    }

    pub fn clone_handle(&mut self, handle: i32) -> i32 {
        self.adopt(Arc::clone(self.slot(handle)))
    }

    pub fn get(&self, handle: i32) -> &GlyphBlob {
        self.slot(handle)
    }

    pub fn get_mut(&mut self, handle: i32) -> &mut GlyphBlob {
        Arc::make_mut(
            self.slots[(handle - 1) as usize]
                .as_mut()
                .expect("released glyph-blob handle"),
        )
    }

    pub fn release(&mut self, handle: i32) {
        assert!(handle > 0, "zero is not a live glyph-blob handle");
        let index = (handle - 1) as usize;
        assert!(
            self.slots[index].take().is_some(),
            "double release of glyph-blob handle"
        );
        self.free.push(index);
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.free.clear();
    }

    #[cfg(test)]
    fn live(&self) -> usize {
        self.slots.len() - self.free.len()
    }

    fn adopt(&mut self, blob: Arc<GlyphBlob>) -> i32 {
        let index = match self.free.pop() {
            Some(index) => {
                debug_assert!(self.slots[index].is_none());
                self.slots[index] = Some(blob);
                index
            }
            None => {
                self.slots.push(Some(blob));
                self.slots.len() - 1
            }
        };
        i32::try_from(index + 1).expect("glyph-blob handle overflow")
    }

    fn slot(&self, handle: i32) -> &Arc<GlyphBlob> {
        assert!(handle > 0, "zero is not a live glyph-blob handle");
        self.slots[(handle - 1) as usize]
            .as_ref()
            .expect("released glyph-blob handle")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(x: i32, gid: u16) -> GlyphBlob {
        GlyphBlob {
            locations: vec![FixedPoint { x, y: 0 }],
            gids: vec![gid],
        }
    }

    #[test]
    fn handles_are_nonzero_and_recycled() {
        let mut arena = GlyphBlobArena::default();
        let first = arena.insert(blob(1, 7));
        let second = arena.insert(blob(2, 8));
        assert_eq!((first, second), (1, 2));
        arena.release(first);
        assert_eq!(arena.insert(blob(3, 9)), first);
        assert_eq!(arena.live(), 2);
    }

    #[test]
    fn cloned_handles_copy_on_write() {
        let mut arena = GlyphBlobArena::default();
        let first = arena.insert(blob(5, 9));
        let second = arena.clone_handle(first);
        arena.get_mut(second).locations[0].x = 42;

        assert_eq!(arena.get(first).locations[0].x, 5);
        assert_eq!(arena.get(second).locations[0].x, 42);
        arena.release(first);
        assert_eq!(arena.get(second).gids, [9]);
    }

    #[test]
    #[should_panic(expected = "double release")]
    fn double_release_is_loud() {
        let mut arena = GlyphBlobArena::default();
        let handle = arena.insert(GlyphBlob::default());
        arena.release(handle);
        arena.release(handle);
    }
}
