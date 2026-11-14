//! TeX's conditional stack, the `cond_ptr` list of saved `\if` states.

use crate::base::codes::{FiOrElseCode, IfTest};
use crate::memory::node::Stored;

/// One conditional on the stack. TeX tells them apart by node address, so each gets its own id.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct CondId(u32);

impl CondId {
    pub(crate) fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct SavedCond {
    pub id: CondId,
    pub cur_if: IfTest,
    pub if_limit: Stored<FiOrElseCode>,
    pub if_line: i32,
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct CondStack {
    entries: Vec<SavedCond>,
    next_id: u32,
}

impl CondStack {
    pub(crate) fn top(&self) -> Option<CondId> {
        self.entries.last().map(|c| c.id)
    }

    pub(crate) fn below_top(&self) -> Option<CondId> {
        let n = self.entries.len();
        n.checked_sub(2).map(|i| self.entries[i].id)
    }

    pub(crate) fn depth(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn push(
        &mut self,
        cur_if: IfTest,
        if_limit: Stored<FiOrElseCode>,
        if_line: i32,
    ) -> CondId {
        let id = CondId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        self.entries.push(SavedCond {
            id,
            cur_if,
            if_limit,
            if_line,
        });
        id
    }

    pub(crate) fn pop(&mut self) -> Option<SavedCond> {
        self.entries.pop()
    }

    pub(crate) fn iter_from_top(&self) -> impl Iterator<Item = &SavedCond> {
        self.entries.iter().rev()
    }

    /// The conditional pushed right after `p`, which holds the state `p` will return to.
    pub(crate) fn above_mut(&mut self, p: CondId) -> Option<&mut SavedCond> {
        let i = self.entries.iter().position(|c| c.id == p)?;
        self.entries.get_mut(i + 1)
    }

    pub(crate) fn next_id(&self) -> u32 {
        self.next_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_stay_distinct_after_a_pop() {
        let mut s = CondStack::default();
        let a = s.push(IfTest::default(), FiOrElseCode::If.into(), 10);
        s.pop();
        let b = s.push(IfTest::default(), FiOrElseCode::If.into(), 20);
        assert_ne!(a, b);
        assert_eq!((s.top(), s.below_top(), s.depth()), (Some(b), None, 1));
    }

    #[test]
    fn the_entry_above_holds_the_saved_limit() {
        let mut s = CondStack::default();
        let a = s.push(IfTest::default(), FiOrElseCode::If.into(), 10);
        let b = s.push(IfTest::default(), FiOrElseCode::Fi.into(), 20);
        assert_eq!(s.above_mut(a).map(|c| c.id), Some(b));
        assert!(s.above_mut(b).is_none());
        assert_eq!(s.below_top(), Some(a));
        let lines: Vec<i32> = s.iter_from_top().map(|c| c.if_line).collect();
        assert_eq!(lines, [20, 10]);
    }
}
