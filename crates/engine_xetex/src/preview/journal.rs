//! Reverse journals for checkpointable state.
//!
//! Batch runs leave journals disarmed. While armed, each [`Journaled`] part of
//! the engine records how to undo the current epoch, and sealing hands that
//! record out as a [`Delta`] for the checkpoint ladder.

use std::mem;
use std::ops::{Deref, Range};

use bytemuck::{Pod, Zeroable};

use crate::base::cmd::Cmd;
use crate::base::command::Command;
use crate::memory::mem::MemDelta;
use crate::state::EngineState;
use crate::tables::eqtb::EqtbDelta;
use crate::tables::hash::{HashEntry, PrimEntry};

/// State the checkpoint ladder rewinds by undoing recorded changes.
/// Lengths are not part of it: checkpoint marks restore them.
pub(crate) trait Journaled {
    type Delta: Delta;
    fn arm(&mut self);
    fn disarm(&mut self);
    /// Closes the open epoch and starts the next.
    fn seal(&mut self) -> Self::Delta;
    /// The open epoch so far, leaving it open.
    fn snapshot(&self) -> Self::Delta;
    fn apply(&mut self, delta: &Self::Delta);
}

/// The undo record of one epoch.
pub(crate) trait Delta: Clone + Default {
    /// Merges in `older`, which restores further back and wins where both hold a value.
    fn compose_with_older(&mut self, older: Self);
    fn compact(&mut self);
    fn allocated_bytes(&self) -> usize;
}

/// Declares the delta of a struct as one delta per journaled field, matched by name.
macro_rules! journaled_fields {
    ($(#[$meta:meta])* $vis:vis struct $delta:ident for $owner:ty {
        $($field:ident: $ty:ty),* $(,)?
    }) => {
        $(#[$meta])*
        #[derive(Clone, Default)]
        $vis struct $delta {
            $($field: $ty,)*
        }

        impl $crate::preview::journal::Delta for $delta {
            fn compose_with_older(&mut self, older: Self) {
                $($crate::preview::journal::Delta::compose_with_older(&mut self.$field, older.$field);)*
            }

            fn compact(&mut self) {
                $($crate::preview::journal::Delta::compact(&mut self.$field);)*
            }

            fn allocated_bytes(&self) -> usize {
                0 $(+ $crate::preview::journal::Delta::allocated_bytes(&self.$field))*
            }
        }

        impl $crate::preview::journal::Journaled for $owner {
            type Delta = $delta;

            fn arm(&mut self) {
                $($crate::preview::journal::Journaled::arm(&mut self.$field);)*
            }

            fn disarm(&mut self) {
                $($crate::preview::journal::Journaled::disarm(&mut self.$field);)*
            }

            fn seal(&mut self) -> $delta {
                $delta {
                    $($field: $crate::preview::journal::Journaled::seal(&mut self.$field),)*
                }
            }

            fn snapshot(&self) -> $delta {
                $delta {
                    $($field: $crate::preview::journal::Journaled::snapshot(&self.$field),)*
                }
            }

            fn apply(&mut self, delta: &$delta) {
                $($crate::preview::journal::Journaled::apply(&mut self.$field, &delta.$field);)*
            }
        }
    };
}
pub(crate) use journaled_fields;

journaled_fields! {
    /// Undoes one epoch of the whole engine.
    pub(crate) struct EngineDelta for EngineState {
        mem: MemDelta,
        eqtb: EqtbDelta,
    }
}

#[derive(Clone, Copy)]
#[repr(C, packed(4))]
struct Preimage<T: Copy> {
    idx: u32,
    old: T,
}

const _: () = assert!(mem::size_of::<Preimage<u64>>() == 12);

/// The first-touch journal of one flat array of `T` words.
///
/// Words at or past the epoch's `base` did not exist when it began, so they need no pre-image.
struct WordJournal<T: Copy> {
    armed: bool,
    base: usize,
    touched: Vec<u64>,
    entries: Vec<Preimage<T>>,
}

impl<T: Copy> Default for WordJournal<T> {
    fn default() -> Self {
        Self {
            armed: false,
            base: 0,
            touched: Vec::new(),
            entries: Vec::new(),
        }
    }
}

/// A word a journal can hold, and whether two of them restore to the same state.
pub(crate) trait JournalWord: Copy {
    fn same(&self, other: &Self) -> bool;
}

impl<T: Pod> JournalWord for T {
    #[inline]
    fn same(&self, other: &Self) -> bool {
        bytemuck::bytes_of(self) == bytemuck::bytes_of(other)
    }
}

impl JournalWord for Cmd {
    #[inline]
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl JournalWord for Command {
    #[inline]
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl<T: JournalWord> WordJournal<T> {
    #[inline(always)]
    fn armed(&self) -> bool {
        self.armed
    }

    fn arm(&mut self, base: usize) {
        assert!(
            u32::try_from(base).is_ok(),
            "journal arena exceeds u32 indices"
        );
        self.touched.clear();
        self.entries.clear();
        self.armed = true;
        self.rebase(base);
    }

    fn rebase(&mut self, base: usize) {
        self.base = base;
        let len = base.div_ceil(64);
        if self.touched.len() < len {
            self.touched.resize(len, 0);
        }
    }

    fn disarm(&mut self) {
        *self = Self::default();
    }

    #[inline]
    fn note(&mut self, index: usize, old: T) {
        debug_assert!(self.armed);
        if index >= self.base {
            return;
        }
        let bit = 1u64 << (index & 63);
        let word = &mut self.touched[index >> 6];
        if *word & bit == 0 {
            *word |= bit;
            self.entries.push(Preimage {
                idx: index as u32,
                old,
            });
        }
    }

    /// Notes every word of a node. One mask test settles the usual case, where
    /// all of them are already noted, at any node size.
    #[inline]
    fn note_run(&mut self, start: usize, words: &[T]) {
        debug_assert!(self.armed);
        let shift = start & 63;
        if shift + words.len() <= 64 && start + words.len() <= self.base {
            let mask = (u64::MAX >> ((64 - words.len()) & 63)) << shift;
            if self.touched[start >> 6] & mask == mask {
                return;
            }
        }
        self.note_each(start, words);
    }

    #[inline(never)]
    fn note_each(&mut self, start: usize, words: &[T]) {
        for (index, &old) in (start..).zip(words) {
            self.note(index, old);
        }
    }

    /// Closes the epoch. A word back at its pre-image needs no restoring, so it
    /// is dropped, which also undoes the cost of noting a node's unwritten words.
    fn seal(&mut self, data: &[T]) -> WordDelta<T> {
        for e in &self.entries {
            self.touched[e.idx as usize >> 6] = 0;
        }
        self.entries.retain(|e| {
            let old = e.old;
            !data[e.idx as usize].same(&old)
        });
        self.rebase(data.len());
        WordDelta {
            entries: mem::take(&mut self.entries),
        }
    }

    fn snapshot(&self) -> WordDelta<T> {
        WordDelta {
            entries: self.entries.clone(),
        }
    }

    #[cfg(test)]
    fn open_len(&self) -> usize {
        self.entries.len()
    }
}

/// Undoes one epoch of a [`JournaledVec`].
#[derive(Clone)]
pub(crate) struct WordDelta<T: Copy> {
    entries: Vec<Preimage<T>>,
}

impl<T: Copy> Default for WordDelta<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T: Copy> WordDelta<T> {
    fn apply_to(&self, data: &mut [T]) {
        for e in self.entries.iter().rev() {
            data[e.idx as usize] = e.old;
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

impl<T: Copy> Delta for WordDelta<T> {
    /// `older` restores further back in time, so its pre-image wins on overlap.
    fn compose_with_older(&mut self, mut older: WordDelta<T>) {
        // Each delta holds a word at most once, so reordering is harmless.
        older.entries.sort_unstable_by_key(|e| e.idx);
        let known = older.entries.len();
        for &e in &self.entries {
            if older.entries[..known]
                .binary_search_by_key(&e.idx, |o| o.idx)
                .is_err()
            {
                older.entries.push(e);
            }
        }
        *self = older;
    }

    fn compact(&mut self) {
        self.entries.shrink_to_fit();
    }

    fn allocated_bytes(&self) -> usize {
        self.entries.capacity() * mem::size_of::<Preimage<T>>()
    }
}

impl JournalWord for HashEntry {
    #[inline]
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl JournalWord for PrimEntry {
    #[inline]
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

/// A flat array that notes each word's pre-image before writing it while armed.
/// Reads go through `Deref`, and every write goes through a noting method.
pub(crate) struct JournaledVec<T: Copy> {
    data: Vec<T>,
    journal: WordJournal<T>,
}

impl<T: Copy> Default for JournaledVec<T> {
    fn default() -> Self {
        Vec::new().into()
    }
}

impl<T: Copy> From<Vec<T>> for JournaledVec<T> {
    /// A disarmed array holding `data`.
    fn from(data: Vec<T>) -> Self {
        Self {
            data,
            journal: WordJournal::default(),
        }
    }
}

impl<T: Copy> Deref for JournaledVec<T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &[T] {
        &self.data
    }
}

impl<T: JournalWord> JournaledVec<T> {
    #[inline(always)]
    pub(crate) fn set(&mut self, i: usize, value: T) {
        *self.at_mut(i) = value;
    }

    #[inline(always)]
    pub(crate) fn at_mut(&mut self, i: usize) -> &mut T {
        if self.journal.armed() {
            self.journal.note(i, self.data[i]);
        }
        &mut self.data[i]
    }

    #[inline(always)]
    pub(crate) fn run_mut(&mut self, run: Range<usize>) -> &mut [T] {
        let start = run.start;
        let words = &mut self.data[run];
        if self.journal.armed() {
            self.journal.note_run(start, words);
        }
        words
    }

    /// The run `dst` to write, and the disjoint run `src` to read.
    #[inline]
    pub(crate) fn run_mut_and(
        &mut self,
        dst: Range<usize>,
        src: Range<usize>,
    ) -> Option<(&mut [T], &[T])> {
        if self.journal.armed() {
            self.journal.note_run(dst.start, &self.data[dst.clone()]);
        }
        let [dst, src] = self.data.get_disjoint_mut([dst, src]).ok()?;
        Some((dst, src))
    }

    /// Words past the epoch's start need no pre-image, so growing is never noted.
    pub(crate) fn push(&mut self, value: T) {
        self.data.push(value);
    }

    pub(crate) fn grow(&mut self, len: usize, value: T) {
        debug_assert!(len >= self.data.len(), "a journaled array only grows");
        self.data.resize(len, value);
    }

    /// Writes that a checkpoint never needs to undo, such as loading a format.
    pub(crate) fn unjournaled_mut(&mut self) -> &mut [T] {
        &mut self.data
    }
}

impl<T: JournalWord + Zeroable> JournaledVec<T> {
    /// Sets the length a checkpoint recorded, after its deltas restored the words.
    pub(crate) fn restore_len(&mut self, len: usize) {
        self.data.resize(len, T::zeroed());
        if self.journal.armed() {
            self.journal.rebase(len);
        }
    }
}

impl<T: JournalWord> Journaled for JournaledVec<T> {
    type Delta = WordDelta<T>;

    fn arm(&mut self) {
        self.journal.arm(self.data.len());
    }

    fn disarm(&mut self) {
        self.journal.disarm();
    }

    fn seal(&mut self) -> WordDelta<T> {
        self.journal.seal(&self.data)
    }

    fn snapshot(&self) -> WordDelta<T> {
        self.journal.snapshot()
    }

    fn apply(&mut self, delta: &WordDelta<T>) {
        delta.apply_to(&mut self.data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::command::Command;
    use crate::base::units::StrNumber;
    use crate::memory::mem::MemState;
    use crate::tables::cs::NameSlot;
    use crate::tables::eqtb::{EqtbState, Table};
    fn word(value: i32) -> u64 {
        let half = u64::from(value.cast_unsigned());
        half | half << 32
    }

    #[test]
    fn word_journal_skips_words_past_the_epoch_base() {
        let mut data = vec![word(0); 4];
        let mut journal = WordJournal::<u64>::default();
        journal.arm(data.len());
        data.resize(100, word(0));
        journal.note(70, data[70]);
        data[70] = word(5);
        journal.note(2, data[2]);
        data[2] = word(6);
        assert_eq!(journal.open_len(), 1);

        let delta = journal.seal(&data);
        journal.rebase(data.len());
        journal.note(70, data[70]);
        data[70] = word(7);
        assert_eq!(journal.open_len(), 1);

        delta.apply_to(&mut data);
        assert_eq!(data[2], word(0));
        assert_eq!(data[70], word(7));
        journal.seal(&data).apply_to(&mut data);
        assert_eq!(data[70], word(5));
    }

    #[test]
    fn word_journal_records_only_the_first_touch() {
        let mut data = vec![word(0); 160];
        let mut journal = WordJournal::<u64>::default();
        journal.arm(data.len());
        for value in 1..=3 {
            journal.note(7, data[7]);
            data[7] = word(value);
        }
        journal.note(130, data[130]);
        data[130] = word(9);
        assert_eq!(journal.open_len(), 2);

        let delta = journal.seal(&data);
        assert_eq!(delta.len(), 2);
        delta.apply_to(&mut data);
        assert_eq!(data[7], word(0));
        assert_eq!(data[130], word(0));
    }

    #[test]
    fn sealing_drops_words_back_at_their_preimage() {
        let mut data = vec![word(0); 4];
        let mut journal = WordJournal::<u64>::default();
        journal.arm(data.len());
        journal.note_run(0, &data[..3]);
        data[1] = word(1);
        data[2] = word(1);
        data[2] = word(0);

        let delta = journal.seal(&data);
        assert_eq!(delta.len(), 1);
        delta.apply_to(&mut data);
        assert_eq!(data[1], word(0));
    }

    #[test]
    fn note_run_saves_each_word_once_across_a_bitmap_boundary() {
        let data: Vec<_> = (0..128).map(word).collect();
        let mut journal = WordJournal::<u64>::default();
        journal.arm(data.len());
        journal.note(62, data[62]);
        journal.note(63, data[63]);
        journal.note_run(62, &data[62..66]);
        assert_eq!(journal.open_len(), 4);
        journal.note_run(62, &data[62..66]);
        assert_eq!(journal.open_len(), 4);
    }

    #[test]
    fn note_run_saves_the_unnoted_words_of_a_partly_noted_node() {
        let data: Vec<_> = (0..16).map(word).collect();
        let mut journal = WordJournal::<u64>::default();
        journal.arm(data.len());
        journal.note(5, data[5]);
        journal.note_run(4, &data[4..6]);
        assert_eq!(journal.open_len(), 2);
        journal.note_run(4, &data[4..6]);
        assert_eq!(journal.open_len(), 2);
    }

    #[test]
    fn sealing_starts_a_fresh_touch_epoch() {
        let mut data = vec![word(0); 64];
        let mut journal = WordJournal::<u64>::default();
        journal.arm(data.len());
        journal.note(3, data[3]);
        data[3] = word(1);
        let first = journal.seal(&data);
        journal.note(3, data[3]);
        data[3] = word(2);
        let second = journal.seal(&data);

        second.apply_to(&mut data);
        assert_eq!(data[3], word(1));
        first.apply_to(&mut data);
        assert_eq!(data[3], word(0));
    }

    #[test]
    fn word_delta_composition_keeps_the_older_preimage() {
        let mut data = vec![word(0); 3];
        let mut journal = WordJournal::<u64>::default();
        journal.arm(data.len());
        journal.note(0, data[0]);
        data[0] = word(1);
        journal.note(1, data[1]);
        data[1] = word(1);
        let older = journal.seal(&data);
        journal.note(0, data[0]);
        data[0] = word(2);
        journal.note(2, data[2]);
        data[2] = word(2);
        let mut newer = journal.seal(&data);

        newer.compose_with_older(older);
        newer.apply_to(&mut data);
        assert_eq!(data, vec![word(0); 3]);
    }

    #[test]
    fn mem_setters_capture_preimages() {
        let mut mem = MemState {
            data: vec![0; 4].into(),
            ..Default::default()
        };
        mem.data.arm();

        mem.set_word(1, 10);
        mem.set_word(0, 11);
        mem.set_word(2, 12);
        mem.set_word(0, 13);
        assert_eq!(*mem.data, [13, 10, 12, 0]);

        let delta = mem.data.seal();
        assert_eq!(delta.len(), 3);
        mem.data.apply(&delta);
        assert_eq!(*mem.data, [0; 4]);
    }

    #[test]
    fn eqtb_setters_capture_preimages() {
        let cs = NameSlot::bucket(0).cs();
        let mut eqtb = EqtbState {
            hash: vec![HashEntry::default(); 1].into(),
            meanings: Table::at_level(cs.index() + 1, Command::UndefinedCs, 0),
            ..Default::default()
        };
        eqtb.arm();
        eqtb.set_text(cs, StrNumber::from_raw(6));
        eqtb.set_meaning(cs, Command::Halign, 3);
        let delta = eqtb.seal();
        eqtb.apply(&delta);
        assert_eq!(eqtb.hash[0].text, StrNumber::NONE);
        assert_eq!(
            (eqtb.meaning(cs), eqtb.cs_level(cs)),
            (Command::UndefinedCs, 0)
        );
    }

    #[test]
    fn composed_deltas_rewind_the_hash_to_its_oldest_preimage() {
        let cs = NameSlot::bucket(0).cs();
        let mut eqtb = EqtbState {
            hash: vec![HashEntry::default(); 1].into(),
            ..Default::default()
        };
        eqtb.arm();
        eqtb.set_text(cs, StrNumber::from_raw(6));
        eqtb.set_text(cs, StrNumber::from_raw(7));
        let older = eqtb.seal();
        eqtb.set_text(cs, StrNumber::from_raw(8));
        let mut newer = eqtb.seal();
        newer.compose_with_older(older);
        eqtb.apply(&newer);
        assert_eq!(eqtb.hash[0].text, StrNumber::NONE);
    }
}
