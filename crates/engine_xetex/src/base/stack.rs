//! TeX's fixed-size stacks: entries, a pointer, a size limit and a high-water mark.

use std::ops::{Index, IndexMut};

use crate::print::errors::{Abort, Capacity};

/// A push past a stack's size.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct StackFull;

/// A TeX stack of at most `size` entries that remembers the deepest it has been.
///
/// Like WEB's arrays it keeps one slot past the top, where a caller can [`park`]
/// the current level (`cur_input`, `cur_list`) to walk every level by index.
///
/// [`park`]: TexStack::park
#[derive(Clone, Debug, Default)]
pub(crate) struct TexStack<T> {
    slots: Vec<T>,
    len: usize,
    max_len: usize,
}

impl<T: Copy + Default> TexStack<T> {
    /// An empty stack that holds up to `size` entries.
    pub(crate) fn with_size(size: usize) -> Self {
        TexStack {
            slots: vec![T::default(); size + 1],
            len: 0,
            max_len: 0,
        }
    }

    /// TeX's overflow error for this stack, reporting its size.
    pub(crate) fn overflow(&self, capacity: Capacity) -> Abort {
        Abort::overflow(capacity, self.size())
    }

    /// The most entries the stack holds.
    #[inline]
    pub(crate) fn size(&self) -> usize {
        self.slots.len().saturating_sub(1)
    }

    /// The number of entries, WEB's `*_ptr`.
    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The deepest the stack has been, WEB's `max_*_stack`.
    #[inline]
    pub(crate) fn max_len(&self) -> usize {
        self.max_len
    }

    /// Pushes `v`, checking the size only when the stack grows past its old depth, as WEB does.
    #[inline]
    pub(crate) fn push(&mut self, v: T) -> Result<(), StackFull> {
        if self.len > self.max_len {
            self.max_len = self.len;
            if self.len == self.size() {
                return Err(StackFull);
            }
        }
        self.slots[self.len] = v;
        self.len += 1;
        Ok(())
    }

    /// Pushes all of `vs` at once, as `macro_call` stores a macro's parameters.
    pub(crate) fn push_all(&mut self, vs: &[T]) -> Result<(), StackFull> {
        let end = self.len + vs.len();
        if end > self.max_len {
            self.max_len = end;
            if end > self.size() {
                return Err(StackFull);
            }
        }
        self.slots[self.len..end].copy_from_slice(vs);
        self.len = end;
        Ok(())
    }

    /// Pops the top entry, `None` when empty.
    #[inline]
    pub(crate) fn pop(&mut self) -> Option<T> {
        self.len = self.len.checked_sub(1)?;
        Some(self.slots[self.len])
    }

    /// Stores `top` in the slot past the top entry, so `self[self.len()]` reads it.
    #[inline]
    pub(crate) fn park(&mut self, top: T) {
        self.slots[self.len] = top;
    }

    /// Slot `i`, which may lie past the parked one and hold a stale entry, as WEB's arrays allow.
    #[inline]
    pub(crate) fn stale_slot(&self, i: usize) -> &T {
        &self.slots[i]
    }

    /// The entries and the parked slot, as a checkpoint keeps them. Empty before allocation.
    pub(crate) fn with_parked(&self) -> &[T] {
        &self.slots[..(self.len + 1).min(self.slots.len())]
    }

    /// The entries from the bottom up.
    pub(crate) fn entries(&self) -> &[T] {
        &self.slots[..self.len]
    }

    /// Restores entries captured by [`entries`](TexStack::entries) or
    /// [`with_parked`](TexStack::with_parked), with their depth and high-water mark.
    pub(crate) fn restore(&mut self, slots: &[T], len: usize, max_len: usize) {
        self.slots[..slots.len()].copy_from_slice(slots);
        self.len = len;
        self.max_len = max_len;
    }

    /// Empties the stack and forgets its high-water mark.
    pub(crate) fn reset(&mut self) {
        self.len = 0;
        self.max_len = 0;
    }

    /// Frees the storage.
    pub(crate) fn release(&mut self) {
        *self = TexStack::default();
    }
}

impl<T> Index<usize> for TexStack<T> {
    type Output = T;
    #[inline(always)]
    fn index(&self, i: usize) -> &T {
        &self.slots[..=self.len][i]
    }
}

impl<T> IndexMut<usize> for TexStack<T> {
    #[inline(always)]
    fn index_mut(&mut self, i: usize) -> &mut T {
        &mut self.slots[..=self.len][i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_fails_only_past_the_size() {
        let mut s = TexStack::<i32>::with_size(2);
        assert_eq!(s.push(1), Ok(()));
        assert_eq!(s.push(2), Ok(()));
        assert_eq!(s.push(3), Err(StackFull));
        assert_eq!(s.len(), 2);
        assert_eq!(s.max_len(), 2);
    }

    #[test]
    fn pop_returns_entries_and_keeps_the_high_water_mark() {
        let mut s = TexStack::<i32>::with_size(4);
        s.push(1).ok();
        s.push(2).ok();
        s.push(3).ok();
        assert_eq!(s.pop(), Some(3));
        assert_eq!(s.pop(), Some(2));
        assert_eq!(s.entries(), &[1]);
        s.push(9).ok();
        assert_eq!(s.max_len(), 2);
        s.pop();
        s.pop();
        assert_eq!(s.pop(), None);
    }

    #[test]
    fn parked_top_is_indexable() {
        let mut s = TexStack::<i32>::with_size(3);
        s.push(1).ok();
        s.park(7);
        assert_eq!(s[1], 7);
        assert_eq!(s.with_parked(), &[1, 7]);
    }

    #[test]
    fn an_unallocated_stack_has_nothing_parked() {
        let s = TexStack::<i32>::default();
        assert_eq!(s.with_parked(), &[] as &[i32]);
    }

    #[test]
    fn push_all_checks_the_whole_run() {
        let mut s = TexStack::<i32>::with_size(3);
        assert_eq!(s.push_all(&[1, 2]), Ok(()));
        assert_eq!(s.push_all(&[3, 4]), Err(StackFull));
        assert_eq!(s.entries(), &[1, 2]);
    }
}
