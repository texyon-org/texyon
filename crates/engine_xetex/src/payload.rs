//! The variable-length payloads of array nodes, kept outside `mem` so every node there has a fixed
//! size. A payload is a run of words shared copy-on-write, so copied nodes and checkpoints keep
//! their contents while the live one changes.

use std::sync::Arc;

use crate::errors::Abort;

/// A one-based handle to a payload, zero for none.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub struct PayloadId(u32);

impl PayloadId {
    pub const NONE: PayloadId = PayloadId(0);

    fn slot(self) -> Option<usize> {
        usize::try_from(self.0).ok()?.checked_sub(1)
    }
}

/// The payloads of all array nodes.
#[derive(Clone, Default)]
pub(crate) struct PayloadStore {
    slots: Vec<Option<Arc<[u32]>>>,
    free: Vec<usize>,
}

impl PayloadStore {
    /// A fresh payload of `words` zero words.
    pub(crate) fn insert(&mut self, words: usize) -> Result<PayloadId, Abort> {
        self.adopt(Arc::from(vec![0; words]))
    }

    /// A second handle on the payload `id`, which shares its words until one side writes.
    pub(crate) fn share(&mut self, id: PayloadId) -> Result<PayloadId, Abort> {
        match self.arc(id) {
            Some(words) => self.adopt(Arc::clone(words)),
            None => Ok(PayloadId::NONE),
        }
    }

    pub(crate) fn release(&mut self, id: PayloadId) {
        if let Some(i) = id.slot() {
            if self.slots.get_mut(i).and_then(Option::take).is_some() {
                self.free.push(i);
            }
        }
    }

    /// The words of `id`, empty for none.
    pub(crate) fn get(&self, id: PayloadId) -> &[u32] {
        self.arc(id).map_or(&[], |words| words)
    }

    /// The words of `id`, unshared first.
    pub(crate) fn get_mut(&mut self, id: PayloadId) -> &mut [u32] {
        match id.slot().and_then(|i| self.slots.get_mut(i)) {
            Some(Some(words)) => Arc::make_mut(words),
            _ => &mut [],
        }
    }

    /// The words of `dst`, unshared first, beside those of a different payload `src`.
    pub(crate) fn get_mut_and(
        &mut self,
        dst: PayloadId,
        src: PayloadId,
    ) -> Option<(&mut [u32], &[u32])> {
        let [d, s] = self
            .slots
            .get_disjoint_mut([dst.slot()?, src.slot()?])
            .ok()?;
        Some((Arc::make_mut(d.as_mut()?), s.as_deref()?))
    }

    pub(crate) fn restore_from(&mut self, source: &Self) {
        self.slots.clone_from(&source.slots);
        self.free.clone_from(&source.free);
    }

    pub(crate) fn clear(&mut self) {
        self.slots.clear();
        self.free.clear();
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.slots.capacity() * std::mem::size_of::<Option<Arc<[u32]>>>()
            + self.free.capacity() * std::mem::size_of::<usize>()
    }

    /// Every slot in handle order, `None` for a free one, for digests and formats.
    pub(crate) fn slots(&self) -> impl Iterator<Item = Option<&[u32]>> {
        self.slots.iter().map(|slot| slot.as_deref())
    }

    /// Rebuilds the store from slots in handle order, for a format load.
    pub(crate) fn load(&mut self, slots: Vec<Option<Vec<u32>>>) {
        self.slots = slots.into_iter().map(|s| s.map(Arc::from)).collect();
        self.free = (0..self.slots.len())
            .filter(|&i| self.slots[i].is_none())
            .collect();
    }

    fn arc(&self, id: PayloadId) -> Option<&Arc<[u32]>> {
        self.slots.get(id.slot()?)?.as_ref()
    }

    fn adopt(&mut self, words: Arc<[u32]>) -> Result<PayloadId, Abort> {
        let i = match self.free.pop() {
            Some(i) => {
                self.slots[i] = Some(words);
                i
            }
            None => {
                self.slots.push(Some(words));
                self.slots.len() - 1
            }
        };
        u32::try_from(i + 1)
            .map(PayloadId)
            .map_err(|_| Abort::Internal("too many node payloads"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_are_nonzero_and_recycled() {
        let mut store = PayloadStore::default();
        let a = store.insert(2).unwrap();
        let b = store.insert(3).unwrap();
        assert_eq!((a.0, b.0), (1, 2));
        store.release(a);
        assert_eq!(store.insert(1).unwrap(), a);
        assert_eq!(store.get(b), [0, 0, 0]);
        assert_eq!(store.get(PayloadId::NONE), [] as [u32; 0]);
    }

    #[test]
    fn shared_payloads_copy_on_write() {
        let mut store = PayloadStore::default();
        let a = store.insert(2).unwrap();
        store.get_mut(a)[0] = 5;
        let b = store.share(a).unwrap();
        store.get_mut(b)[0] = 9;
        assert_eq!((store.get(a), store.get(b)), (&[5, 0][..], &[9, 0][..]));

        let saved = store.clone();
        store.get_mut(a)[1] = 7;
        assert_eq!(saved.get(a), [5, 0]);
        store.restore_from(&saved);
        assert_eq!(store.get(a), [5, 0]);
    }

    #[test]
    fn get_mut_and_refuses_one_payload_twice() {
        let mut store = PayloadStore::default();
        let a = store.insert(1).unwrap();
        let b = store.insert(1).unwrap();
        store.get_mut(b)[0] = 4;
        let (to, from) = store.get_mut_and(a, b).unwrap();
        to.copy_from_slice(from);
        assert_eq!(store.get(a), [4]);
        assert!(store.get_mut_and(a, a).is_none());
    }
}
