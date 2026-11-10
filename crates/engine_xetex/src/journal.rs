//! Reverse journals for checkpointable flat arrays.
//!
//! Batch runs leave journals disarmed. An armed [`WordJournal`] records the
//! first pre-image of each touched word in an epoch. [`PlainTrail`] records
//! every write for smaller, colder arrays and relies on reverse replay.

use bytemuck::Pod;

use crate::mem_types::MemoryWord;

/// A pre-image: word `idx` held `old` before the epoch touched it.
#[derive(Clone, Copy)]
#[repr(C, packed(4))]
struct Preimage<T: Copy> {
    idx: u32,
    old: T,
}

const _: () = assert!(std::mem::size_of::<Preimage<MemoryWord>>() == 12);

/// The first-touch journal of one flat array of `T` words.
///
/// Words at or past the epoch's `base` did not exist when it began, so they need no pre-image.
pub(crate) struct WordJournal<T: Copy> {
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

impl<T: Pod> WordJournal<T> {
    #[inline(always)]
    pub(crate) fn armed(&self) -> bool {
        self.armed
    }

    /// Starts journaling the first `base` words.
    pub(crate) fn arm(&mut self, base: usize) {
        assert!(
            u32::try_from(base).is_ok(),
            "journal arena exceeds u32 indices"
        );
        self.touched.clear();
        self.entries.clear();
        self.armed = true;
        self.rebase(base);
    }

    /// Moves the epoch's base, the length the array had when the epoch began.
    pub(crate) fn rebase(&mut self, base: usize) {
        self.base = base;
        let len = base.div_ceil(64);
        if self.touched.len() < len {
            self.touched.resize(len, 0);
        }
    }

    pub(crate) fn disarm(&mut self) {
        *self = Self::default();
    }

    #[inline]
    pub(crate) fn note(&mut self, index: usize, old: T) {
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
    pub(crate) fn note_run(&mut self, start: usize, words: &[T]) {
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
    pub(crate) fn seal(&mut self, data: &[T]) -> WordDelta<T> {
        for e in &self.entries {
            self.touched[e.idx as usize >> 6] = 0;
        }
        self.entries.retain(|e| {
            let old = e.old;
            bytemuck::bytes_of(&data[e.idx as usize]) != bytemuck::bytes_of(&old)
        });
        self.rebase(data.len());
        WordDelta {
            entries: std::mem::take(&mut self.entries),
        }
    }

    pub(crate) fn snapshot(&self) -> WordDelta<T> {
        WordDelta {
            entries: self.entries.clone(),
        }
    }

    #[cfg(test)]
    fn open_len(&self) -> usize {
        self.entries.len()
    }
}

/// The pre-images of one epoch, at most one per word.
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
    pub(crate) fn apply_to(&self, data: &mut [T]) {
        for e in self.entries.iter().rev() {
            data[e.idx as usize] = e.old;
        }
    }

    /// `older` restores further back in time, so its pre-image wins on overlap.
    pub(crate) fn compose_with_older(&mut self, mut older: WordDelta<T>) {
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

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.entries.capacity() * std::mem::size_of::<Preimage<T>>()
    }

    pub(crate) fn compact(&mut self) {
        self.entries.shrink_to_fit();
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

pub(crate) struct PlainTrail<T> {
    armed: bool,
    idx: Vec<u32>,
    old: Vec<T>,
}

impl<T> Default for PlainTrail<T> {
    fn default() -> Self {
        Self {
            armed: false,
            idx: Vec::new(),
            old: Vec::new(),
        }
    }
}

impl<T: Copy> PlainTrail<T> {
    #[inline(always)]
    pub(crate) fn armed(&self) -> bool {
        self.armed
    }

    pub(crate) fn arm(&mut self) {
        self.idx.clear();
        self.old.clear();
        self.armed = true;
    }

    pub(crate) fn disarm(&mut self) {
        *self = Self::default();
    }

    #[inline]
    pub(crate) fn note(&mut self, index: usize, old: T) {
        debug_assert!(self.armed);
        self.idx.push(index as u32);
        self.old.push(old);
    }

    pub(crate) fn seal(&mut self) -> PlainDelta<T> {
        PlainDelta {
            idx: std::mem::take(&mut self.idx),
            old: std::mem::take(&mut self.old),
        }
    }

    pub(crate) fn snapshot(&self) -> PlainDelta<T> {
        PlainDelta {
            idx: self.idx.clone(),
            old: self.old.clone(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct PlainDelta<T> {
    idx: Vec<u32>,
    old: Vec<T>,
}

impl<T> Default for PlainDelta<T> {
    fn default() -> Self {
        Self {
            idx: Vec::new(),
            old: Vec::new(),
        }
    }
}

impl<T: Copy> PlainDelta<T> {
    pub(crate) fn apply_to(&self, data: &mut [T]) {
        for (&index, &old) in self.idx.iter().zip(&self.old).rev() {
            data[index as usize] = old;
        }
    }

    /// Keeps only the first pre-image per index, which is the furthest back in time.
    pub(crate) fn compose_with_older(&mut self, mut older: PlainDelta<T>) {
        COMPOSE_SEEN.with_borrow_mut(|seen| {
            let mut kept = 0;
            for k in 0..older.idx.len() {
                let index = older.idx[k];
                if first_sighting(seen, index) {
                    older.idx[kept] = index;
                    older.old[kept] = older.old[k];
                    kept += 1;
                }
            }
            older.idx.truncate(kept);
            older.old.truncate(kept);
            for (&index, &value) in self.idx.iter().zip(&self.old) {
                if first_sighting(seen, index) {
                    older.idx.push(index);
                    older.old.push(value);
                }
            }
            for &index in &older.idx {
                seen[index as usize >> 6] = 0;
            }
        });
        *self = older;
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.idx.capacity() * std::mem::size_of::<u32>()
            + self.old.capacity() * std::mem::size_of::<T>()
    }

    pub(crate) fn compact(&mut self) {
        self.idx.shrink_to_fit();
        self.old.shrink_to_fit();
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.idx.len()
    }
}

thread_local! {
    /// Index bitmap for [`PlainDelta::compose_with_older`], all clear between calls.
    static COMPOSE_SEEN: std::cell::RefCell<Vec<u64>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Marks `index` in `seen`, reporting whether it was clear.
#[inline]
fn first_sighting(seen: &mut Vec<u64>, index: u32) -> bool {
    let word = index as usize >> 6;
    if word >= seen.len() {
        seen.resize(word + 1, 0);
    }
    let bit = 1u64 << (index & 63);
    let fresh = seen[word] & bit == 0;
    seen[word] |= bit;
    fresh
}

#[derive(Default)]
pub(crate) struct EqtbJournal {
    pub(crate) words: WordJournal<MemoryWord>,
    pub(crate) xeq: PlainTrail<u16>,
    pub(crate) hash: PlainTrail<crate::mem_types::B32x2>,
    pub(crate) prim: PlainTrail<crate::mem_types::B32x2>,
}

impl EqtbJournal {
    fn disarm(&mut self) {
        self.words.disarm();
        self.xeq.disarm();
        self.hash.disarm();
        self.prim.disarm();
    }
}

pub(crate) fn arm(st: &mut crate::state::EngineState) {
    st.mem.journal.arm(st.mem.data.len());
    st.mem.tokens.journal.arm(st.mem.tokens.nodes().len());
    st.mem.glue.journal.arm(st.mem.glue.specs().len());
    st.eqtb.journal.words.arm(st.eqtb.data.len());
    st.eqtb.journal.xeq.arm();
    st.eqtb.journal.hash.arm();
    st.eqtb.journal.prim.arm();
    st.fonts.font_info_journal.arm();
    st.fonts.fmem_epoch_base = st.fonts.fmem_ptr;
}

pub(crate) fn disarm(st: &mut crate::state::EngineState) {
    st.mem.journal.disarm();
    st.mem.tokens.journal.disarm();
    st.mem.glue.journal.disarm();
    st.eqtb.journal.disarm();
    st.fonts.font_info_journal.disarm();
    st.fonts.fmem_epoch_base = crate::units::FontInfoIdx::ZERO;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem_types::B32x2;

    fn word(value: i32) -> MemoryWord {
        MemoryWord {
            b32: B32x2 {
                s0: value,
                s1: value,
            },
        }
    }

    fn bits(value: MemoryWord) -> u64 {
        let b = value.b32();
        u64::from(b.s0 as u32) | (u64::from(b.s1 as u32) << 32)
    }

    #[test]
    fn word_journal_skips_words_past_the_epoch_base() {
        let mut data = vec![word(0); 4];
        let mut journal = WordJournal::<MemoryWord>::default();
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
        assert_eq!(bits(data[2]), bits(word(0)));
        assert_eq!(bits(data[70]), bits(word(7)));
        journal.seal(&data).apply_to(&mut data);
        assert_eq!(bits(data[70]), bits(word(5)));
    }

    #[test]
    fn word_journal_records_only_the_first_touch() {
        let mut data = vec![word(0); 160];
        let mut journal = WordJournal::<MemoryWord>::default();
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
        assert_eq!(bits(data[7]), bits(word(0)));
        assert_eq!(bits(data[130]), bits(word(0)));
    }

    #[test]
    fn sealing_drops_words_back_at_their_preimage() {
        let mut data = vec![word(0); 4];
        let mut journal = WordJournal::<MemoryWord>::default();
        journal.arm(data.len());
        journal.note_run(0, &data[..3]);
        data[1] = word(1);
        data[2] = word(1);
        data[2] = word(0);

        let delta = journal.seal(&data);
        assert_eq!(delta.len(), 1);
        delta.apply_to(&mut data);
        assert_eq!(bits(data[1]), bits(word(0)));
    }

    #[test]
    fn note_run_saves_each_word_once_across_a_bitmap_boundary() {
        let data: Vec<_> = (0..128).map(word).collect();
        let mut journal = WordJournal::<MemoryWord>::default();
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
        let mut journal = WordJournal::<MemoryWord>::default();
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
        let mut journal = WordJournal::<MemoryWord>::default();
        journal.arm(data.len());
        journal.note(3, data[3]);
        data[3] = word(1);
        let first = journal.seal(&data);
        journal.note(3, data[3]);
        data[3] = word(2);
        let second = journal.seal(&data);

        second.apply_to(&mut data);
        assert_eq!(bits(data[3]), bits(word(1)));
        first.apply_to(&mut data);
        assert_eq!(bits(data[3]), bits(word(0)));
    }

    #[test]
    fn plain_trail_replays_duplicate_writes_in_reverse() {
        let mut data = vec![10u16, 20, 30];
        let mut trail = PlainTrail::default();
        trail.arm();
        trail.note(1, data[1]);
        data[1] = 21;
        trail.note(1, data[1]);
        data[1] = 22;
        trail.note(2, data[2]);
        data[2] = 31;
        let delta = trail.seal();
        assert_eq!(delta.len(), 3);
        delta.apply_to(&mut data);
        assert_eq!(data, vec![10, 20, 30]);
    }

    #[test]
    fn word_delta_composition_keeps_the_older_preimage() {
        let mut data = vec![word(0); 3];
        let mut journal = WordJournal::<MemoryWord>::default();
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
        assert_eq!(
            data.iter().map(|v| bits(*v)).collect::<Vec<_>>(),
            vec![0, 0, 0]
        );
    }

    #[test]
    fn plain_delta_composition_canonicalizes_duplicates() {
        let mut data = vec![0i32, 0];
        let mut trail = PlainTrail::default();
        trail.arm();
        trail.note(0, 0);
        data[0] = 1;
        trail.note(0, 1);
        data[0] = 2;
        let older = trail.seal();
        trail.note(0, 2);
        data[0] = 3;
        trail.note(1, 0);
        data[1] = 4;
        let mut newer = trail.seal();

        newer.compose_with_older(older);
        assert_eq!(newer.len(), 2);
        newer.apply_to(&mut data);
        assert_eq!(data, vec![0, 0]);
    }

    #[test]
    fn plain_delta_composition_keeps_first_sightings_in_order() {
        let delta = |entries: &[(u32, i32)]| PlainDelta {
            idx: entries.iter().map(|e| e.0).collect(),
            old: entries.iter().map(|e| e.1).collect(),
        };
        let pairs = |d: &PlainDelta<i32>| -> Vec<(u32, i32)> {
            d.idx.iter().copied().zip(d.old.iter().copied()).collect()
        };

        let older = delta(&[(900, 1), (5, 2), (900, 3), (64, 4)]);
        let mut newer = delta(&[(64, 5), (7, 6), (5, 7), (7, 8), (1000, 9)]);
        newer.compose_with_older(older);
        assert_eq!(
            pairs(&newer),
            vec![(900, 1), (5, 2), (64, 4), (7, 6), (1000, 9)]
        );

        // The scratch bitmap is left clear, so a later merge sees fresh indices.
        let mut again = delta(&[(5, 10), (900, 11)]);
        again.compose_with_older(delta(&[(7, 12)]));
        assert_eq!(pairs(&again), vec![(7, 12), (5, 10), (900, 11)]);

        let mut empty = PlainDelta::<i32>::default();
        empty.compose_with_older(PlainDelta::default());
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn mem_setters_capture_preimages() {
        let mut mem = crate::state::MemState {
            data: vec![0; 4],
            ..Default::default()
        };
        mem.journal.arm(mem.data.len());

        mem.set_word(1, 10);
        mem.set_word(0, 11);
        mem.set_word(2, 12);
        mem.set_word(0, 13);
        assert_eq!(mem.data, [13, 10, 12, 0]);

        let delta = mem.journal.seal(&mem.data);
        assert_eq!(delta.len(), 3);
        delta.apply_to(&mut mem.data);
        assert_eq!(mem.data, [0; 4]);
    }

    #[test]
    fn eqtb_and_font_setters_capture_preimages() {
        let mut eqtb = crate::state::EqtbState {
            data: vec![word(0); 2],
            hash: vec![B32x2 { s0: 0, s1: 0 }; 1],
            ..Default::default()
        };
        eqtb.journal.words.arm(eqtb.data.len());
        eqtb.journal.hash.arm();
        eqtb.set_equiv(crate::units::EqtbPtr::from_raw(1), 5);
        eqtb.set_text(
            crate::units::EqtbPtr::from_raw(514),
            crate::units::StrNumber::from_raw(6),
        );
        eqtb.journal.words.seal(&eqtb.data).apply_to(&mut eqtb.data);
        eqtb.journal.hash.seal().apply_to(&mut eqtb.hash);
        assert_eq!(bits(eqtb.data[1]), 0);
        assert_eq!(eqtb.hash[0].s1, 0);

        let mut fonts = crate::state::FontState {
            font_info: vec![word(0); 3],
            fmem_ptr: crate::units::FontInfoIdx::from_raw(2),
            fmem_epoch_base: crate::units::FontInfoIdx::from_raw(2),
            ..Default::default()
        };
        fonts.font_info_journal.arm();
        fonts.set_font_info_sc(
            crate::units::FontInfoIdx::from_raw(1),
            crate::units::Scaled::from_raw(7),
        );
        fonts.set_font_info_sc(
            crate::units::FontInfoIdx::from_raw(2),
            crate::units::Scaled::from_raw(8),
        ); // append region: dead after scalar rewind
        let delta = fonts.font_info_journal.seal();
        assert_eq!(delta.len(), 1);
        delta.apply_to(&mut fonts.font_info);
        assert_eq!(bits(fonts.font_info[1]), 0);
        assert_ne!(bits(fonts.font_info[2]), 0);
    }
}
