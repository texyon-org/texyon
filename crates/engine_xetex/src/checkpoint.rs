//! Reverse-delta checkpoint ladder.

use crate::journal::{PlainDelta, WordDelta};
use crate::mem_types::{B32x2, MemoryWord};
use crate::replay::fence::ReplayFence;
use crate::replay::source::SourceSnapshot;
use crate::state::EngineState;

mod bundle;
mod oracle;

pub(crate) use bundle::{EngineBundle, StringWatermark};

#[derive(Clone)]
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
        let mut delta = Self {
            mem: st.mem.journal.seal(),
            eqtb: st.eqtb.journal.words.seal(),
            xeq: st.eqtb.journal.xeq.seal(),
            hash: st.eqtb.journal.hash.seal(),
            prim: st.eqtb.journal.prim.seal(),
            font_info: st.fonts.font_info_journal.seal(),
        };
        delta.compact();
        st.fonts.fmem_epoch_base = st.fonts.fmem_ptr;
        delta
    }

    fn snapshot_open(st: &EngineState) -> Self {
        Self {
            mem: st.mem.journal.snapshot(),
            eqtb: st.eqtb.journal.words.snapshot(),
            xeq: st.eqtb.journal.xeq.snapshot(),
            hash: st.eqtb.journal.hash.snapshot(),
            prim: st.eqtb.journal.prim.snapshot(),
            font_info: st.fonts.font_info_journal.snapshot(),
        }
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
        self.compact();
    }

    fn compact(&mut self) {
        self.mem.compact();
        self.eqtb.compact();
        self.xeq.compact();
        self.hash.compact();
        self.prim.compact();
        self.font_info.compact();
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
    fence: Option<ReplayFence>,
    digests: Vec<oracle::ComponentDigest>,
}

pub(crate) struct CheckpointLadder<B> {
    rungs: Vec<Rung<B>>,
    verify_rewinds: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RestoreMismatch {
    pub(crate) rung: usize,
    pub(crate) component: &'static str,
    pub(crate) expected: u64,
    pub(crate) actual: u64,
}

/// Accounted-memory policy for a checkpoint ladder.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LadderBudget {
    pub(crate) target_bytes: usize,
    pub(crate) hard_bytes: usize,
    pub(crate) min_recent_rungs: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BudgetError {
    TargetExceedsHardCap,
    RungZeroTooLarge { minimum_bytes: usize },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RungAdmission {
    Admitted {
        index: usize,
        dropped: usize,
        accounted_bytes: usize,
    },
    RungNotAdmitted {
        minimum_bytes: usize,
    },
}

// Covers Vec/Rung headers, allocator rounding, and small owned strings that
// are deliberately not individually inspected. This is charged per survivor
// in addition to all measured capacities.
const RUNG_CONTAINER_ALLOWANCE: usize = 4096;

impl<B> CheckpointLadder<B> {
    pub(crate) fn new(base: B, clock: u64) -> Self {
        Self {
            rungs: vec![Rung {
                undo: None,
                bundle: base,
                clock,
                resumable: false,
                protected: true,
                fence: None,
                digests: Vec::new(),
            }],
            verify_rewinds: false,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.rungs.len()
    }

    #[cfg(test)]
    pub(crate) fn push(
        &mut self,
        st: &mut EngineState,
        bundle: B,
        clock: u64,
        resumable: bool,
        protected: bool,
    ) -> usize {
        self.push_with_fence(st, bundle, clock, resumable, protected, None)
    }

    fn push_with_fence(
        &mut self,
        st: &mut EngineState,
        bundle: B,
        clock: u64,
        resumable: bool,
        protected: bool,
        fence: Option<ReplayFence>,
    ) -> usize {
        self.rungs.push(Rung {
            undo: Some(EngineDelta::seal(st)),
            bundle,
            clock,
            resumable,
            protected,
            fence,
            digests: Vec::new(),
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

    /// Protect every rung through `index` against budget-driven eviction.
    ///
    /// Background continuation work advances the clock far past the edit that
    /// produced these rungs, which makes them the oldest and therefore the
    /// cheapest to drop — exactly backwards, because the next edit replays from
    /// one of them. Protecting the whole prefix leaves the continuation's own
    /// rungs as the only eviction candidates under the soft budget.
    pub(crate) fn protect_rungs_through(&mut self, index: usize) -> bool {
        if index >= self.rungs.len() {
            return false;
        }
        for rung in self.rungs.iter_mut().take(index + 1) {
            rung.protected = true;
        }
        true
    }

    pub(crate) fn rung_clock(&self, index: usize) -> Option<u64> {
        self.rungs.get(index).map(|rung| rung.clock)
    }

    pub(crate) fn rung_at_clock(&self, clock: u64) -> Option<usize> {
        self.rungs.iter().position(|rung| rung.clock == clock)
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
        Self::new(
            EngineBundle::capture(st, strings_base, None),
            st.scanner.logical_clock,
        )
    }

    #[cfg(test)]
    pub(crate) fn from_engine_verified(st: &EngineState, strings_base: StringWatermark) -> Self {
        let mut ladder = Self::from_engine(st, strings_base);
        ladder.verify_rewinds = true;
        ladder.rungs[0].digests = oracle::digest_state(st);
        ladder
    }

    pub(crate) fn enable_verification(&mut self, st: &EngineState) {
        assert_eq!(
            self.rungs.len(),
            1,
            "oracle enabled after document execution"
        );
        self.verify_rewinds = true;
        self.rungs[0].digests = oracle::digest_state(st);
    }

    pub(crate) fn from_engine_with_budget(
        st: &EngineState,
        strings_base: StringWatermark,
        budget: LadderBudget,
    ) -> Result<Self, BudgetError> {
        if budget.target_bytes > budget.hard_bytes {
            return Err(BudgetError::TargetExceedsHardCap);
        }
        let ladder = Self::from_engine(st, strings_base);
        let minimum = ladder.accounted_bytes();
        if minimum > budget.hard_bytes {
            return Err(BudgetError::RungZeroTooLarge {
                minimum_bytes: minimum,
            });
        }
        Ok(ladder)
    }

    #[cfg(test)]
    pub(crate) fn push_engine(
        &mut self,
        st: &mut EngineState,
        strings_base: StringWatermark,
        resumable: bool,
        protected: bool,
    ) -> usize {
        let bundle =
            EngineBundle::capture(st, strings_base, self.rungs.last().map(|rung| &rung.bundle));
        let index = self.push(st, bundle, st.scanner.logical_clock, resumable, protected);
        if self.verify_rewinds {
            self.rungs[index].digests = oracle::digest_state(st);
        }
        index
    }

    /// Deepest resumable fence whose source constraints remain sound.
    pub(crate) fn select_replay_fence(
        &self,
        snapshot: &SourceSnapshot,
    ) -> Option<(usize, ReplayFence)> {
        self.rungs
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, rung)| {
                (rung.resumable)
                    .then_some(rung.fence.as_ref())
                    .flatten()
                    .filter(|fence| fence.is_eligible(snapshot))
                    .cloned()
                    .map(|fence| (index, fence))
            })
    }

    pub(crate) fn replay_fence(&self, index: usize) -> Option<ReplayFence> {
        self.rungs.get(index)?.fence.clone()
    }

    pub(crate) fn rewind_engine(&mut self, st: &mut EngineState, target: usize) {
        if let Err(mismatch) = self.rewind_engine_checked(st, target) {
            panic!(
                "checkpoint rung {} component `{}` restored to {:016x}, expected {:016x}",
                mismatch.rung, mismatch.component, mismatch.actual, mismatch.expected
            );
        }
    }

    pub(crate) fn rewind_engine_checked(
        &mut self,
        st: &mut EngineState,
        target: usize,
    ) -> Result<(), RestoreMismatch> {
        self.rewind_to(st, target, |state, bundle| bundle.restore(state));
        self.verify_rung(st, target)
    }

    pub(crate) fn verify_rung(
        &self,
        st: &EngineState,
        target: usize,
    ) -> Result<(), RestoreMismatch> {
        if !self.verify_rewinds {
            return Ok(());
        }
        let expected = &self.rungs[target].digests;
        let actual = oracle::digest_state(st);
        for (expected, actual) in expected.iter().zip(&actual) {
            if expected != actual {
                return Err(RestoreMismatch {
                    rung: target,
                    component: expected.name,
                    expected: expected.value,
                    actual: actual.value,
                });
            }
        }
        assert_eq!(
            expected.len(),
            actual.len(),
            "checkpoint digest schema changed"
        );
        Ok(())
    }

    /// Destructively verify every surviving rung, newest to oldest.
    #[cfg(test)]
    pub(crate) fn verify_all_rewinds(
        &mut self,
        st: &mut EngineState,
    ) -> Result<(), RestoreMismatch> {
        for target in (0..self.rungs.len()).rev() {
            self.rewind_engine_checked(st, target)?;
        }
        Ok(())
    }

    /// Count rung-private allocations and each shared hyphen table exactly
    /// once, using the Arc allocation address as its stable identity.
    pub(crate) fn accounted_bytes(&self) -> usize {
        let mut total = self.delta_bytes() + self.rungs.len() * RUNG_CONTAINER_ALLOWANCE;
        let mut shared = std::collections::HashSet::new();
        for rung in &self.rungs {
            total += rung.bundle.allocated_bytes();
            total += rung.fence.as_ref().map_or(0, ReplayFence::allocated_bytes);
            let (identity, bytes) = rung.bundle.shared_hyph_tables();
            if shared.insert(identity as usize) {
                total += bytes;
            }
        }
        total
    }

    #[cfg(test)]
    pub(crate) fn admit_engine_rung(
        &mut self,
        st: &mut EngineState,
        strings_base: StringWatermark,
        resumable: bool,
        protected: bool,
        budget: LadderBudget,
    ) -> RungAdmission {
        self.admit_engine_rung_with_fence(st, strings_base, resumable, protected, None, budget)
    }

    pub(crate) fn admit_engine_fence(
        &mut self,
        st: &mut EngineState,
        strings_base: StringWatermark,
        fence: ReplayFence,
        protected: bool,
        budget: LadderBudget,
    ) -> RungAdmission {
        self.admit_engine_rung_with_fence(st, strings_base, true, protected, Some(fence), budget)
    }

    fn admit_engine_rung_with_fence(
        &mut self,
        st: &mut EngineState,
        strings_base: StringWatermark,
        resumable: bool,
        protected: bool,
        fence: Option<ReplayFence>,
        budget: LadderBudget,
    ) -> RungAdmission {
        assert!(
            budget.target_bytes <= budget.hard_bytes,
            "invalid ladder budget"
        );
        let candidate =
            EngineBundle::capture(st, strings_base, self.rungs.last().map(|rung| &rung.bundle));
        let mut survivors: Vec<Option<usize>> = (0..self.rungs.len()).map(Some).collect();
        survivors.push(None);
        let mut clocks: Vec<u64> = self.rungs.iter().map(|rung| rung.clock).collect();
        clocks.push(st.scanner.logical_clock);
        let mut protections: Vec<bool> = self.rungs.iter().map(|rung| rung.protected).collect();
        protections.push(protected);
        let mut deltas: Vec<Option<EngineDelta>> =
            self.rungs.iter().map(|rung| rung.undo.clone()).collect();
        deltas.push(Some(EngineDelta::snapshot_open(st)));

        let mut dropped = 0;
        while planned_bytes(self, &candidate, fence.as_ref(), &survivors, &deltas)
            > budget.target_bytes
        {
            let Some(index) =
                pick_rung_to_drop(&clocks, &protections, budget.min_recent_rungs, false)
            else {
                break;
            };
            drop_planned_rung(
                index,
                &mut survivors,
                &mut clocks,
                &mut protections,
                &mut deltas,
            );
            dropped += 1;
        }

        // The soft target never overrides fresh protections or the recent
        // window. The hard cap does: progressively thin any interior rung.
        while planned_bytes(self, &candidate, fence.as_ref(), &survivors, &deltas)
            > budget.hard_bytes
        {
            let Some(index) = pick_rung_to_drop(&clocks, &protections, 0, true) else {
                return RungAdmission::RungNotAdmitted {
                    minimum_bytes: planned_bytes(
                        self,
                        &candidate,
                        fence.as_ref(),
                        &survivors,
                        &deltas,
                    ),
                };
            };
            drop_planned_rung(
                index,
                &mut survivors,
                &mut clocks,
                &mut protections,
                &mut deltas,
            );
            dropped += 1;
        }

        let surviving_existing: std::collections::HashSet<usize> =
            survivors.iter().filter_map(|index| *index).collect();
        self.push_with_fence(
            st,
            candidate,
            st.scanner.logical_clock,
            resumable,
            protected,
            fence,
        );
        if self.verify_rewinds {
            let top = self.rungs.len() - 1;
            self.rungs[top].digests = oracle::digest_state(st);
        }
        for index in (1..self.rungs.len() - 1).rev() {
            if !surviving_existing.contains(&index) {
                // Hard-cap planning is allowed to relax stale protection.
                self.rungs[index].protected = false;
                assert!(self.decimate(index));
            }
        }
        let accounted_bytes = self.accounted_bytes();
        assert!(
            accounted_bytes <= budget.hard_bytes,
            "published ladder exceeded its staged hard cap"
        );
        RungAdmission::Admitted {
            index: self.rungs.len() - 1,
            dropped,
            accounted_bytes,
        }
    }
}

fn planned_bytes(
    ladder: &CheckpointLadder<EngineBundle>,
    candidate: &EngineBundle,
    candidate_fence: Option<&ReplayFence>,
    survivors: &[Option<usize>],
    deltas: &[Option<EngineDelta>],
) -> usize {
    let mut total = survivors.len() * RUNG_CONTAINER_ALLOWANCE
        + deltas
            .iter()
            .filter_map(Option::as_ref)
            .map(EngineDelta::allocated_bytes)
            .sum::<usize>();
    total += candidate_fence.map_or(0, ReplayFence::allocated_bytes);
    let mut shared = std::collections::HashSet::new();
    for survivor in survivors {
        let bundle = survivor.map_or(candidate, |index| &ladder.rungs[index].bundle);
        total += bundle.allocated_bytes();
        if let Some(index) = survivor {
            total += ladder.rungs[*index]
                .fence
                .as_ref()
                .map_or(0, ReplayFence::allocated_bytes);
        }
        let (identity, bytes) = bundle.shared_hyph_tables();
        if shared.insert(identity as usize) {
            total += bytes;
        }
    }
    total
}

fn pick_rung_to_drop(
    clocks: &[u64],
    protected: &[bool],
    min_recent_rungs: usize,
    relax: bool,
) -> Option<usize> {
    if clocks.len() <= 2 {
        return None;
    }
    let recent_start = clocks.len().saturating_sub(min_recent_rungs.max(1));
    let top = clocks[clocks.len() - 1];
    let mut best: Option<(u64, u64, usize)> = None;
    for index in 1..clocks.len() - 1 {
        if !relax && (protected[index] || index >= recent_start) {
            continue;
        }
        let gap = clocks[index + 1].saturating_sub(clocks[index - 1]);
        let age = top.saturating_sub(clocks[index]).saturating_add(1);
        if best.is_none_or(|(best_gap, best_age, _)| {
            u128::from(gap) * u128::from(best_age) < u128::from(best_gap) * u128::from(age)
        }) {
            best = Some((gap, age, index));
        }
    }
    best.map(|(_, _, index)| index)
}

fn drop_planned_rung(
    index: usize,
    survivors: &mut Vec<Option<usize>>,
    clocks: &mut Vec<u64>,
    protected: &mut Vec<bool>,
    deltas: &mut Vec<Option<EngineDelta>>,
) {
    survivors.remove(index);
    clocks.remove(index);
    protected.remove(index);
    let older = deltas
        .remove(index)
        .expect("interior planned rung without undo");
    deltas[index]
        .as_mut()
        .expect("planned successor without undo")
        .compose_with_older(older);
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
        Bundle {
            mem_end: st.mem.mem_end,
            fmem_ptr: st.fonts.fmem_ptr,
        }
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

    #[test]
    fn too_small_budget_is_rejected_at_rung_zero() {
        let st = state();
        let strings_base = StringWatermark::capture(&st);
        let minimum = CheckpointLadder::from_engine(&st, strings_base).accounted_bytes();
        let error = CheckpointLadder::from_engine_with_budget(
            &st,
            strings_base,
            LadderBudget {
                target_bytes: minimum - 1,
                hard_bytes: minimum - 1,
                min_recent_rungs: 2,
            },
        )
        .err()
        .expect("undersized budget accepted");
        assert_eq!(
            error,
            BudgetError::RungZeroTooLarge {
                minimum_bytes: minimum
            }
        );
    }

    #[test]
    fn rejected_candidate_does_not_seal_the_open_epoch() {
        let mut st = state();
        let strings_base = StringWatermark::capture(&st);
        let mut ladder = CheckpointLadder::from_engine(&st, strings_base);
        let minimum = ladder.accounted_bytes();
        st.mem.set_b32s1(0, 7);
        let rejected = ladder.admit_engine_rung(
            &mut st,
            strings_base,
            true,
            false,
            LadderBudget {
                target_bytes: minimum,
                hard_bytes: minimum,
                min_recent_rungs: 1,
            },
        );
        assert!(matches!(rejected, RungAdmission::RungNotAdmitted { .. }));
        assert_eq!(ladder.len(), 1);

        let admitted = ladder.admit_engine_rung(
            &mut st,
            strings_base,
            true,
            false,
            LadderBudget {
                target_bytes: usize::MAX,
                hard_bytes: usize::MAX,
                min_recent_rungs: 1,
            },
        );
        assert!(matches!(admitted, RungAdmission::Admitted { .. }));
        st.mem.set_b32s1(0, 8);
        ladder.rewind_engine(&mut st, 0);
        assert_eq!(value(st.mem.data[0]), 0);
    }

    #[test]
    fn admission_decimates_to_the_hard_byte_cap() {
        let mut st = state();
        let strings_base = StringWatermark::capture(&st);
        let mut ladder = CheckpointLadder::from_engine(&st, strings_base);
        let unlimited = LadderBudget {
            target_bytes: usize::MAX,
            hard_bytes: usize::MAX,
            min_recent_rungs: 1,
        };
        st.scanner.logical_clock = 1;
        ladder.admit_engine_rung(&mut st, strings_base, true, true, unlimited);
        let two_rungs = ladder.accounted_bytes();
        let one_rung = {
            let base = CheckpointLadder::from_engine(&st, strings_base);
            base.accounted_bytes()
        };
        let per_rung = two_rungs - one_rung;
        let budget = LadderBudget {
            target_bytes: two_rungs + per_rung,
            hard_bytes: two_rungs + per_rung,
            min_recent_rungs: 3,
        };
        for clock in 2..10 {
            st.scanner.logical_clock = clock;
            st.mem.set_b32s1(0, clock as i32);
            let admission = ladder.admit_engine_rung(&mut st, strings_base, true, true, budget);
            assert!(matches!(admission, RungAdmission::Admitted { .. }));
            assert!(ladder.accounted_bytes() <= budget.hard_bytes);
        }
        assert!(ladder.len() < 10);
    }

    #[test]
    fn a_protected_prefix_outlives_later_rungs_under_budget_pressure() {
        let mut st = state();
        let strings_base = StringWatermark::capture(&st);
        let mut ladder = CheckpointLadder::from_engine(&st, strings_base);
        let unlimited = LadderBudget {
            target_bytes: usize::MAX,
            hard_bytes: usize::MAX,
            min_recent_rungs: 1,
        };
        for clock in 1..=4 {
            st.scanner.logical_clock = clock;
            st.mem.set_b32s1(0, clock as i32);
            ladder.admit_engine_rung(&mut st, strings_base, true, false, unlimited);
        }
        let horizon = ladder.len() - 1;
        assert!(ladder.protect_rungs_through(horizon));
        let clocks: Vec<u64> = (0..=horizon)
            .map(|index| ladder.rung_clock(index).unwrap())
            .collect();

        // Continuation work advances the clock far past the protected prefix
        // and must stay under a budget that cannot hold every rung.
        let budget = LadderBudget {
            target_bytes: ladder.accounted_bytes(),
            hard_bytes: usize::MAX,
            min_recent_rungs: 1,
        };
        for clock in 100..120 {
            st.scanner.logical_clock = clock;
            st.mem.set_b32s1(0, clock as i32);
            let admission = ladder.admit_engine_rung(&mut st, strings_base, true, false, budget);
            assert!(matches!(admission, RungAdmission::Admitted { .. }));
        }

        for (index, clock) in clocks.iter().enumerate() {
            assert_eq!(
                ladder.rung_clock(index),
                Some(*clock),
                "protected rung {index} was evicted by continuation work"
            );
        }
    }

    #[test]
    fn oracle_names_a_write_that_bypassed_the_mem_journal() {
        let mut st = state();
        st.strings.str_pool = vec![0; 4];
        st.strings.str_start = vec![0; 4];
        let strings_base = StringWatermark::capture(&st);
        let mut ladder = CheckpointLadder::from_engine_verified(&st, strings_base);

        st.mem.data[0].b32.s1 = 99;
        ladder.push_engine(&mut st, strings_base, false, false);
        let mismatch = ladder
            .rewind_engine_checked(&mut st, 0)
            .expect_err("oracle accepted an unjournaled write");
        assert_eq!(mismatch.component, "mem");
    }

    #[test]
    fn oracle_verifies_survivors_after_decimation() {
        let mut st = state();
        st.strings.str_pool = vec![0; 4];
        st.strings.str_start = vec![0; 4];
        let strings_base = StringWatermark::capture(&st);
        let mut ladder = CheckpointLadder::from_engine_verified(&st, strings_base);
        for clock in 1..=4 {
            st.scanner.logical_clock = clock;
            st.mem.set_b32s1(0, clock as i32);
            ladder.push_engine(&mut st, strings_base, false, false);
        }
        assert!(ladder.decimate(2));
        ladder.verify_all_rewinds(&mut st).unwrap();
        assert_eq!(value(st.mem.data[0]), 0);
    }

    #[test]
    fn prefix_detour_and_rewind_reproduce_state_and_output() {
        fn run_prefix(st: &mut EngineState, value: i32) {
            st.mem.set_b32s1(0, value);
            st.shipout.dvi_buf.clear();
            st.shipout.dvi_buf.extend_from_slice(&value.to_le_bytes());
            st.shipout.dvi_ptr = 4;
            st.scanner.logical_clock += 1;
        }

        let mut st = state();
        st.strings.str_pool = vec![0; 4];
        st.strings.str_start = vec![0; 4];
        let strings_base = StringWatermark::capture(&st);
        let mut ladder = CheckpointLadder::from_engine_verified(&st, strings_base);
        run_prefix(&mut st, 17);
        let expected_output = st.shipout.dvi_buf.clone();
        let expected_state = oracle::digest_state(&st);

        ladder.rewind_engine(&mut st, 0);
        run_prefix(&mut st, 23);
        ladder.push_engine(&mut st, strings_base, false, false);
        ladder.rewind_engine(&mut st, 0);
        run_prefix(&mut st, 17);

        assert_eq!(st.shipout.dvi_buf, expected_output);
        assert_eq!(oracle::digest_state(&st), expected_state);
    }
}
