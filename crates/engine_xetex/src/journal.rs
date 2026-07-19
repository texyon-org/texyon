//! Reverse journals for checkpointable flat arrays.
//!
//! Batch runs leave journals disarmed. An armed [`WordJournal`] records the
//! first pre-image of each touched word in an epoch; [`PlainTrail`] records
//! every write for smaller, colder arrays and relies on reverse replay.

#![allow(dead_code)] // The checkpoint ladder consumes the sealed-delta API next.

use crate::mem_types::MemoryWord;

#[derive(Default)]
pub(crate) struct WordJournal {
    armed: bool,
    touched: Vec<u64>,
    idx: Vec<u32>,
    old: Vec<MemoryWord>,
}

impl WordJournal {
    #[inline(always)]
    pub(crate) fn armed(&self) -> bool {
        self.armed
    }

    pub(crate) fn arm(&mut self, arena_len: usize) {
        assert!(u32::try_from(arena_len).is_ok(), "journal arena exceeds u32 indices");
        self.touched.clear();
        self.touched.resize(arena_len.div_ceil(64), 0);
        self.idx.clear();
        self.old.clear();
        self.armed = true;
    }

    pub(crate) fn disarm(&mut self) {
        *self = Self::default();
    }

    #[inline]
    pub(crate) fn note(&mut self, index: usize, old: MemoryWord) {
        debug_assert!(self.armed);
        let bit = 1u64 << (index & 63);
        let word = &mut self.touched[index >> 6];
        if *word & bit == 0 {
            *word |= bit;
            self.idx.push(index as u32);
            self.old.push(old);
        }
    }

    pub(crate) fn seal(&mut self) -> WordDelta {
        for &index in &self.idx {
            self.touched[index as usize >> 6] = 0;
        }
        WordDelta {
            idx: std::mem::take(&mut self.idx),
            old: std::mem::take(&mut self.old),
        }
    }

    #[cfg(test)]
    fn open_len(&self) -> usize {
        self.idx.len()
    }
}

#[derive(Default)]
pub(crate) struct WordDelta {
    idx: Vec<u32>,
    old: Vec<MemoryWord>,
}

impl WordDelta {
    pub(crate) fn apply_to(&self, data: &mut [MemoryWord]) {
        for i in (0..self.idx.len()).rev() {
            data[self.idx[i] as usize] = self.old[i];
        }
    }

    /// Compose two adjacent reverse deltas. `older` restores further back in
    /// time, so its pre-image wins for indices touched by both epochs.
    pub(crate) fn compose_with_older(&mut self, mut older: WordDelta) {
        let mut older_indices = older.idx.clone();
        older_indices.sort_unstable();
        for i in 0..self.idx.len() {
            if older_indices.binary_search(&self.idx[i]).is_err() {
                older.idx.push(self.idx[i]);
                older.old.push(self.old[i]);
            }
        }
        *self = older;
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.idx.capacity() * std::mem::size_of::<u32>()
            + self.old.capacity() * std::mem::size_of::<MemoryWord>()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.idx.len()
    }
}

pub(crate) struct PlainTrail<T> {
    armed: bool,
    idx: Vec<u32>,
    old: Vec<T>,
}

impl<T> Default for PlainTrail<T> {
    fn default() -> Self {
        Self { armed: false, idx: Vec::new(), old: Vec::new() }
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
}

pub(crate) struct PlainDelta<T> {
    idx: Vec<u32>,
    old: Vec<T>,
}

impl<T> Default for PlainDelta<T> {
    fn default() -> Self {
        Self { idx: Vec::new(), old: Vec::new() }
    }
}

impl<T: Copy> PlainDelta<T> {
    pub(crate) fn apply_to(&self, data: &mut [T]) {
        for i in (0..self.idx.len()).rev() {
            data[self.idx[i] as usize] = self.old[i];
        }
    }

    /// Compose adjacent trails while canonicalizing duplicate indices. The
    /// first pre-image in `older` is the value furthest back in time.
    pub(crate) fn compose_with_older(&mut self, older: PlainDelta<T>) {
        let mut seen = std::collections::HashSet::new();
        let mut idx = Vec::with_capacity(older.idx.len() + self.idx.len());
        let mut old = Vec::with_capacity(older.old.len() + self.old.len());
        for i in 0..older.idx.len() {
            if seen.insert(older.idx[i]) {
                idx.push(older.idx[i]);
                old.push(older.old[i]);
            }
        }
        for i in 0..self.idx.len() {
            if seen.insert(self.idx[i]) {
                idx.push(self.idx[i]);
                old.push(self.old[i]);
            }
        }
        self.idx = idx;
        self.old = old;
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.idx.capacity() * std::mem::size_of::<u32>()
            + self.old.capacity() * std::mem::size_of::<T>()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.idx.len()
    }
}

#[derive(Default)]
pub(crate) struct EqtbJournal {
    pub(crate) words: WordJournal,
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
    st.eqtb.journal.words.arm(st.eqtb.data.len());
    st.eqtb.journal.xeq.arm();
    st.eqtb.journal.hash.arm();
    st.eqtb.journal.prim.arm();
    st.fonts.font_info_journal.arm();
    st.fonts.fmem_epoch_base = st.fonts.fmem_ptr;
}

pub(crate) fn disarm(st: &mut crate::state::EngineState) {
    st.mem.journal.disarm();
    st.eqtb.journal.disarm();
    st.fonts.font_info_journal.disarm();
    st.fonts.fmem_epoch_base = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem_types::B32x2;

    fn word(value: i32) -> MemoryWord {
        MemoryWord { b32: B32x2 { s0: value, s1: value } }
    }

    fn bits(value: MemoryWord) -> u64 {
        unsafe { std::mem::transmute(value) }
    }

    #[test]
    fn word_journal_records_only_the_first_touch() {
        let mut data = vec![word(0); 160];
        let mut journal = WordJournal::default();
        journal.arm(data.len());
        for value in 1..=3 {
            journal.note(7, data[7]);
            data[7] = word(value);
        }
        journal.note(130, data[130]);
        data[130] = word(9);
        assert_eq!(journal.open_len(), 2);

        let delta = journal.seal();
        assert_eq!(delta.len(), 2);
        delta.apply_to(&mut data);
        assert_eq!(bits(data[7]), bits(word(0)));
        assert_eq!(bits(data[130]), bits(word(0)));
    }

    #[test]
    fn sealing_starts_a_fresh_touch_epoch() {
        let mut data = vec![word(0); 64];
        let mut journal = WordJournal::default();
        journal.arm(data.len());
        journal.note(3, data[3]);
        data[3] = word(1);
        let first = journal.seal();
        journal.note(3, data[3]);
        data[3] = word(2);
        let second = journal.seal();

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
        let mut journal = WordJournal::default();
        journal.arm(data.len());
        journal.note(0, data[0]);
        data[0] = word(1);
        journal.note(1, data[1]);
        data[1] = word(1);
        let older = journal.seal();
        journal.note(0, data[0]);
        data[0] = word(2);
        journal.note(2, data[2]);
        data[2] = word(2);
        let mut newer = journal.seal();

        newer.compose_with_older(older);
        newer.apply_to(&mut data);
        assert_eq!(data.iter().map(|v| bits(*v)).collect::<Vec<_>>(), vec![0, 0, 0]);
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
    fn mem_setters_and_raw_ranges_capture_preimages() {
        let mut mem = crate::state::MemState::default();
        mem.data = vec![word(0); 4];
        mem.journal.arm(mem.data.len());

        mem.set_b32s1(0, 10);
        mem.set_b16s0(0, 11);
        mem.journal_mark_range(2, 2);
        mem.data[2] = word(12);
        mem.data[3] = word(13);

        let delta = mem.journal.seal();
        assert_eq!(delta.len(), 3);
        delta.apply_to(&mut mem.data);
        assert!(mem.data.iter().all(|value| bits(*value) == 0));
    }

    #[test]
    fn eqtb_and_font_setters_capture_preimages() {
        let mut eqtb = crate::state::EqtbState::default();
        eqtb.data = vec![word(0); 2];
        eqtb.hash = vec![B32x2 { s0: 0, s1: 0 }; 1];
        eqtb.journal.words.arm(eqtb.data.len());
        eqtb.journal.hash.arm();
        eqtb.set_b32s1(1, 5);
        eqtb.set_hash_s1(514, 6);
        eqtb.journal.words.seal().apply_to(&mut eqtb.data);
        eqtb.journal.hash.seal().apply_to(&mut eqtb.hash);
        assert_eq!(bits(eqtb.data[1]), 0);
        assert_eq!(eqtb.hash[0].s1, 0);

        let mut fonts = crate::state::FontState::default();
        fonts.font_info = vec![word(0); 3];
        fonts.fmem_ptr = 2;
        fonts.fmem_epoch_base = 2;
        fonts.font_info_journal.arm();
        fonts.set_font_info_b32s1(1, 7);
        fonts.set_font_info_b32s1(2, 8); // append region: dead after scalar rewind
        let delta = fonts.font_info_journal.seal();
        assert_eq!(delta.len(), 1);
        delta.apply_to(&mut fonts.font_info);
        assert_eq!(bits(fonts.font_info[1]), 0);
        assert_ne!(bits(fonts.font_info[2]), 0);
    }
}
