//! The line breaker's active list and passive breaks. They live only within
//! one `line_break` call, so unlike WEB they are kept outside `mem`.

use std::array;

use crate::base::consts::MAX_HALFWORD;
use crate::base::units::{MemPtr, Scaled};
use crate::print::errors::Abort;

/// Whether a break is at a discretionary.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum BreakType {
    Unhyphenated,
    Hyphenated,
}

/// A line's fitness class, from very loose to tight.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Fitness {
    VeryLoose,
    Loose,
    Decent,
    Tight,
}

impl Fitness {
    /// Every class in WEB's order, which is also the index order.
    pub(crate) const ALL: [Self; 4] = [Self::VeryLoose, Self::Loose, Self::Decent, Self::Tight];

    /// The index into per-class arrays such as `minimal_demerits`.
    #[inline]
    pub(crate) fn index(self) -> usize {
        self as usize
    }

    /// Whether lines of these classes are not adjacent, which costs `\adjdemerits`.
    #[inline]
    pub(crate) fn far_from(self, other: Self) -> bool {
        self.index().abs_diff(other.index()) > 1
    }
}

/// An entry of the active list.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct EntryId(u32);

impl EntryId {
    /// The list head.
    pub(crate) const ACTIVE_LIST: Self = Self(0);
    /// The end sentinel, the same entry as the head as in WEB.
    pub(crate) const LAST_ACTIVE: Self = Self(0);
}

/// A feasible break on the passive list.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct PassiveId(u32);

/// A feasible break still in play.
#[derive(Copy, Clone, Debug)]
pub(crate) struct ActiveEntry {
    pub fitness: Fitness,
    pub break_type: BreakType,
    pub line_number: i32,
    /// `None` for the paragraph start.
    pub break_node: Option<PassiveId>,
    pub total_demerits: i32,
    /// `\lastlinefit` only, zero otherwise.
    pub shortfall: Scaled,
    /// `\lastlinefit` only, zero otherwise.
    pub glue: Scaled,
}

impl ActiveEntry {
    /// The paragraph start, before line `prev_graf + 1`.
    pub(crate) fn start(prev_graf: i32) -> Self {
        Self {
            fitness: Fitness::Decent,
            break_type: BreakType::Unhyphenated,
            line_number: prev_graf.wrapping_add(1),
            break_node: None,
            total_demerits: 0,
            shortfall: Scaled::ZERO,
            glue: Scaled::ZERO,
        }
    }

    /// WEB's `last_active`, whose line number ends every scan of the list.
    const SENTINEL: Self = Self {
        fitness: Fitness::VeryLoose,
        break_type: BreakType::Hyphenated,
        line_number: MAX_HALFWORD,
        break_node: None,
        total_demerits: 0,
        shortfall: Scaled::ZERO,
        glue: Scaled::ZERO,
    };
}

/// The change in the totals between two active entries, indexed like
/// `active_width` (slot 0 unused).
pub(crate) type Widths = [Scaled; 7];

/// The widths `f(1..=6)`, slot 0 zero.
#[inline]
pub(crate) fn widths(f: impl Fn(usize) -> Scaled) -> Widths {
    array::from_fn(|k| if k == 0 { Scaled::ZERO } else { f(k) })
}

/// What an entry of the active list holds.
#[derive(Copy, Clone, Debug)]
pub(crate) enum Entry {
    Active(ActiveEntry),
    Delta(Widths),
}

#[derive(Copy, Clone, Debug)]
struct Linked {
    link: EntryId,
    entry: Entry,
}

/// A feasible break.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Passive {
    pub cur_break: MemPtr,
    /// WEB's `prev_break` until `post_line_break` reverses the chain.
    pub next_break: Option<PassiveId>,
}

/// The active list and passive breaks of the paragraph being broken.
#[derive(Clone, Debug)]
pub(crate) struct BreakLists {
    entries: Vec<Linked>,
    passives: Vec<Passive>,
}

impl Default for BreakLists {
    fn default() -> Self {
        let mut lists = Self {
            entries: Vec::new(),
            passives: Vec::new(),
        };
        lists.clear();
        lists
    }
}

impl BreakLists {
    /// Drop every entry and passive break, keeping the capacity.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.entries.push(Linked {
            link: EntryId::LAST_ACTIVE,
            entry: Entry::Active(ActiveEntry::SENTINEL),
        });
        self.passives.clear();
    }

    /// Whether the active list holds no entries.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.link(EntryId::ACTIVE_LIST) == EntryId::LAST_ACTIVE
    }

    #[inline]
    pub(crate) fn link(&self, p: EntryId) -> EntryId {
        self.entries[p.0 as usize].link
    }

    #[inline]
    pub(crate) fn set_link(&mut self, p: EntryId, link: EntryId) {
        self.entries[p.0 as usize].link = link;
    }

    #[inline]
    pub(crate) fn get(&self, p: EntryId) -> Entry {
        self.entries[p.0 as usize].entry
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, p: EntryId) -> &mut Entry {
        &mut self.entries[p.0 as usize].entry
    }

    /// A new entry before `link`.
    pub(crate) fn push(&mut self, link: EntryId, entry: Entry) -> Result<EntryId, Abort> {
        let id = EntryId(u32::try_from(self.entries.len()).map_err(|_| Abort::NODE_OVERFLOW)?);
        self.entries.push(Linked { link, entry });
        Ok(id)
    }

    /// The entries from the head to the end, in list order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = Entry> + '_ {
        let mut p = self.link(EntryId::ACTIVE_LIST);
        std::iter::from_fn(move || {
            (p != EntryId::LAST_ACTIVE).then(|| {
                let entry = self.get(p);
                p = self.link(p);
                entry
            })
        })
    }

    /// The active entries, in list order.
    pub(crate) fn actives(&self) -> impl Iterator<Item = ActiveEntry> + '_ {
        self.iter().filter_map(|e| match e {
            Entry::Active(a) => Some(a),
            Entry::Delta(_) => None,
        })
    }

    pub(crate) fn push_passive(&mut self, passive: Passive) -> Result<PassiveId, Abort> {
        let id = PassiveId(u32::try_from(self.passives.len()).map_err(|_| Abort::NODE_OVERFLOW)?);
        self.passives.push(passive);
        Ok(id)
    }

    #[inline]
    pub(crate) fn passive(&self, p: PassiveId) -> Passive {
        self.passives[p.0 as usize]
    }

    #[inline]
    pub(crate) fn passive_mut(&mut self, p: PassiveId) -> &mut Passive {
        &mut self.passives[p.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::units::MemIdx;

    fn delta(w: i32) -> Entry {
        let mut widths = [Scaled::ZERO; 7];
        widths[1] = Scaled::from_raw(w);
        Entry::Delta(widths)
    }

    fn widths(lists: &BreakLists) -> Vec<i32> {
        lists
            .iter()
            .map(|e| match e {
                Entry::Delta(w) => w[1].raw(),
                Entry::Active(a) => -a.line_number,
            })
            .collect()
    }

    #[test]
    fn entries_link_before_their_successor() {
        let mut lists = BreakLists::default();
        assert!(lists.is_empty());
        let a = lists
            .push(EntryId::LAST_ACTIVE, Entry::Active(ActiveEntry::start(0)))
            .unwrap();
        lists.set_link(EntryId::ACTIVE_LIST, a);
        let d = lists.push(a, delta(5)).unwrap();
        lists.set_link(EntryId::ACTIVE_LIST, d);
        assert_eq!(widths(&lists), [5, -1]);
        assert_eq!(lists.actives().count(), 1);

        lists.set_link(EntryId::ACTIVE_LIST, lists.link(d));
        assert_eq!(widths(&lists), [-1]);

        lists.clear();
        assert!(lists.is_empty());
        assert_eq!(lists.iter().count(), 0);
    }

    #[test]
    fn the_sentinel_ends_every_scan() {
        let lists = BreakLists::default();
        let Entry::Active(end) = lists.get(EntryId::LAST_ACTIVE) else {
            panic!("the sentinel is an active entry");
        };
        assert_eq!(end.line_number, MAX_HALFWORD);
        assert_eq!(end.break_type, BreakType::Hyphenated);
    }

    #[test]
    fn passive_ids_follow_push_order() {
        let mut lists = BreakLists::default();
        let p = lists
            .push_passive(Passive {
                cur_break: MemIdx::from_raw(10),
                next_break: None,
            })
            .unwrap();
        let q = lists
            .push_passive(Passive {
                cur_break: MemIdx::from_raw(20),
                next_break: Some(p),
            })
            .unwrap();
        assert_eq!(lists.passive(q).next_break, Some(p));
        assert_eq!(lists.passive(p).cur_break, MemIdx::from_raw(10));
        lists.passive_mut(p).next_break = Some(q);
        assert_eq!(lists.passive(p).next_break, Some(q));
    }

    #[test]
    fn fitness_classes_two_apart_are_far() {
        assert!(Fitness::VeryLoose.far_from(Fitness::Decent));
        assert!(Fitness::Tight.far_from(Fitness::Loose));
        assert!(!Fitness::Decent.far_from(Fitness::Tight));
        assert!(!Fitness::Loose.far_from(Fitness::Loose));
    }
}
