//! Reverse-delta checkpoint ladder.

#![allow(dead_code)] // Wired into the resident session with the rung bundle.

use crate::journal::{PlainDelta, WordDelta};
use crate::mem_types::{B32x2, MemoryWord};
use crate::state::EngineState;

mod bundle;

pub(crate) use bundle::{EngineBundle, StringWatermark};

pub(crate) struct EngineDelta {
    mem: WordDelta,
    eqtb: WordDelta,
    xeq: PlainDelta<u16>,
    hash: PlainDelta<B32x2>,
    prim: PlainDelta<B32x2>,
    font_info: PlainDelta<MemoryWord>,
}

impl EngineDelta {
    pub(crate) fn seal(st: &mut EngineState) -> Self {
        let delta = Self {
            mem: st.mem.journal.seal(),
            eqtb: st.eqtb.journal.words.seal(),
            xeq: st.eqtb.journal.xeq.seal(),
            hash: st.eqtb.journal.hash.seal(),
            prim: st.eqtb.journal.prim.seal(),
            font_info: st.fonts.font_info_journal.seal(),
        };
        st.fonts.fmem_epoch_base = st.fonts.fmem_ptr;
        delta
    }

    pub(crate) fn apply(&self, st: &mut EngineState) {
        self.mem.apply_to(&mut st.mem.data);
        self.eqtb.apply_to(&mut st.eqtb.data);
        self.xeq.apply_to(&mut st.eqtb.xeq_level);
        self.hash.apply_to(&mut st.eqtb.hash);
        self.prim.apply_to(&mut st.eqtb.prim);
        self.font_info.apply_to(&mut st.fonts.font_info);
    }

    pub(crate) fn compose_with_older(&mut self, older: EngineDelta) {
        self.mem.compose_with_older(older.mem);
        self.eqtb.compose_with_older(older.eqtb);
        self.xeq.compose_with_older(older.xeq);
        self.hash.compose_with_older(older.hash);
        self.prim.compose_with_older(older.prim);
        self.font_info.compose_with_older(older.font_info);
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.mem.allocated_bytes()
            + self.eqtb.allocated_bytes()
            + self.xeq.allocated_bytes()
            + self.hash.allocated_bytes()
            + self.prim.allocated_bytes()
            + self.font_info.allocated_bytes()
    }
}

pub(crate) struct Rung<B> {
    /// Reverse delta from this rung to its predecessor; absent only on rung 0.
    undo: Option<EngineDelta>,
    pub(crate) bundle: B,
    pub(crate) clock: u64,
    pub(crate) resumable: bool,
    pub(crate) protected: bool,
}

pub(crate) struct CheckpointLadder<B> {
    rungs: Vec<Rung<B>>,
}

impl<B> CheckpointLadder<B> {
    pub(crate) fn new(base: B, clock: u64) -> Self {
        Self {
            rungs: vec![Rung {
                undo: None,
                bundle: base,
                clock,
                resumable: false,
                protected: true,
            }],
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.rungs.len()
    }

    pub(crate) fn push(
        &mut self,
        st: &mut EngineState,
        bundle: B,
        clock: u64,
        resumable: bool,
        protected: bool,
    ) -> usize {
        self.rungs.push(Rung {
            undo: Some(EngineDelta::seal(st)),
            bundle,
            clock,
            resumable,
            protected,
        });
        self.rungs.len() - 1
    }

    /// Rewind journaled arrays and restore the target's non-journaled bundle.
    /// The open epoch is undone first, then published rung deltas newest-first.
    pub(crate) fn rewind_to(
        &mut self,
        st: &mut EngineState,
        target: usize,
        restore: impl FnOnce(&mut EngineState, &B),
    ) {
        assert!(target < self.rungs.len(), "checkpoint target out of range");
        EngineDelta::seal(st).apply(st);
        for index in (target + 1..self.rungs.len()).rev() {
            self.rungs[index]
                .undo
                .as_ref()
                .expect("non-base rung without undo")
                .apply(st);
        }
        restore(st, &self.rungs[target].bundle);
        self.rungs.truncate(target + 1);
        st.fonts.fmem_epoch_base = st.fonts.fmem_ptr;
    }

    /// Remove an interior rung without breaking the reverse chain. The rung
    /// above it absorbs its older undo delta; the successor's own bundle and
    /// metadata remain the state it originally captured.
    pub(crate) fn decimate(&mut self, index: usize) -> bool {
        if index == 0 || index + 1 >= self.rungs.len() || self.rungs[index].protected {
            return false;
        }
        let removed = self.rungs.remove(index);
        let older = removed.undo.expect("interior rung without undo");
        self.rungs[index]
            .undo
            .as_mut()
            .expect("successor rung without undo")
            .compose_with_older(older);
        true
    }

    pub(crate) fn clear_protections(&mut self) {
        for rung in self.rungs.iter_mut().skip(1) {
            rung.protected = false;
        }
        if let Some(base) = self.rungs.first_mut() {
            base.protected = true;
        }
    }

    pub(crate) fn delta_bytes(&self) -> usize {
        self.rungs
            .iter()
            .filter_map(|rung| rung.undo.as_ref())
            .map(EngineDelta::allocated_bytes)
            .sum()
    }
}

impl CheckpointLadder<EngineBundle> {
    pub(crate) fn from_engine(st: &EngineState, strings_base: StringWatermark) -> Self {
        Self::new(EngineBundle::capture(st, strings_base, None), st.scanner.logical_clock)
    }

    pub(crate) fn push_engine(
        &mut self,
        st: &mut EngineState,
        strings_base: StringWatermark,
        resumable: bool,
        protected: bool,
    ) -> usize {
        let bundle = EngineBundle::capture(
            st,
            strings_base,
            self.rungs.last().map(|rung| &rung.bundle),
        );
        self.push(st, bundle, st.scanner.logical_clock, resumable, protected)
    }

    pub(crate) fn rewind_engine(&mut self, st: &mut EngineState, target: usize) {
        self.rewind_to(st, target, |state, bundle| bundle.restore(state));
    }

    /// Count rung-private allocations and each shared hyphen table exactly
    /// once, using the Arc allocation address as its stable identity.
    pub(crate) fn accounted_bytes(&self) -> usize {
        let mut total = self.delta_bytes();
        let mut shared = std::collections::HashSet::new();
        for rung in &self.rungs {
            total += rung.bundle.allocated_bytes();
            let (identity, bytes) = rung.bundle.shared_hyph_tables();
            if shared.insert(identity as usize) {
                total += bytes;
            }
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Bundle {
        mem_end: i32,
        fmem_ptr: i32,
    }

    fn bundle(st: &EngineState) -> Bundle {
        Bundle { mem_end: st.mem.mem_end, fmem_ptr: st.fonts.fmem_ptr }
    }

    fn restore(st: &mut EngineState, bundle: &Bundle) {
        st.mem.mem_end = bundle.mem_end;
        st.fonts.fmem_ptr = bundle.fmem_ptr;
    }

    fn value(word: MemoryWord) -> i32 {
        unsafe { word.b32.s1 }
    }

    fn state() -> EngineState {
        let mut st = EngineState::default();
        st.mem.data = crate::mem_types::zeroed_vec(4);
        st.eqtb.data = crate::mem_types::zeroed_vec(4);
        st.eqtb.hash = vec![B32x2 { s0: 0, s1: 0 }; 1];
        st.fonts.font_info = crate::mem_types::zeroed_vec(4);
        crate::journal::arm(&mut st);
        st
    }

    #[test]
    fn multi_rung_rewind_restores_arrays_and_scalars() {
        let mut st = state();
        let mut ladder = CheckpointLadder::new(bundle(&st), 0);

        st.mem.set_b32s1(0, 10);
        st.eqtb.set_b32s1(1, 11);
        st.mem.mem_end = 12;
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 1, true, false);

        st.mem.set_b32s1(0, 20);
        st.mem.set_b32s1(2, 21);
        st.eqtb.set_b32s1(1, 22);
        st.fonts.fmem_ptr = 2;
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 2, true, false);

        st.mem.set_b32s1(3, 30); // open epoch after the newest rung
        st.mem.mem_end = 31;
        ladder.rewind_to(&mut st, 1, restore);

        assert_eq!(value(st.mem.data[0]), 10);
        assert_eq!(value(st.mem.data[2]), 0);
        assert_eq!(value(st.mem.data[3]), 0);
        assert_eq!(value(st.eqtb.data[1]), 11);
        assert_eq!(st.mem.mem_end, 12);
        assert_eq!(st.fonts.fmem_ptr, 0);
        assert_eq!(ladder.len(), 2);
    }

    #[test]
    fn decimation_preserves_rewind_truth() {
        let mut st = state();
        let mut ladder = CheckpointLadder::new(bundle(&st), 0);
        st.mem.set_b32s1(0, 1);
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 1, false, false);
        st.mem.set_b32s1(0, 2);
        st.mem.set_b32s1(1, 2);
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 2, false, false);

        assert!(ladder.decimate(1));
        assert_eq!(ladder.len(), 2);
        ladder.rewind_to(&mut st, 0, restore);
        assert_eq!(value(st.mem.data[0]), 0);
        assert_eq!(value(st.mem.data[1]), 0);
    }

    #[test]
    fn protected_and_edge_rungs_cannot_be_decimated() {
        let mut st = state();
        let mut ladder = CheckpointLadder::new(bundle(&st), 0);
        st.mem.set_b32s1(0, 1);
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 1, false, true);
        st.mem.set_b32s1(0, 2);
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 2, false, false);
        assert!(!ladder.decimate(0));
        assert!(!ladder.decimate(1));
        assert!(!ladder.decimate(2));
        ladder.clear_protections();
        assert!(ladder.decimate(1));
    }

    #[test]
    fn engine_bundle_ladder_restores_clock_and_scalar_state() {
        let mut st = state();
        st.strings.str_pool = vec![0; 4];
        st.strings.str_start = vec![0; 4];
        let strings_base = StringWatermark::capture(&st);
        let mut ladder = CheckpointLadder::from_engine(&st, strings_base);

        st.scanner.logical_clock = 10;
        st.mem.mem_end = 20;
        st.mem.set_b32s1(0, 30);
        ladder.push_engine(&mut st, strings_base, true, true);
        st.scanner.logical_clock = 40;
        st.mem.mem_end = 50;
        st.mem.set_b32s1(0, 60);

        ladder.rewind_engine(&mut st, 1);
        assert_eq!(st.scanner.logical_clock, 10);
        assert_eq!(st.mem.mem_end, 20);
        assert_eq!(value(st.mem.data[0]), 30);
        assert!(ladder.accounted_bytes() > 0);
    }
}
