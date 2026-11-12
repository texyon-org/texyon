//! Reverse-delta checkpoint ladder.

use std::borrow::Cow;
use std::collections::HashSet;

use crate::glue_store::GlueSpec;
use crate::journal::{PlainDelta, WordDelta};
use crate::replay::fence::ReplayFence;
use crate::replay::source::SourceSnapshot;
use crate::state::EngineState;

mod bundle;
mod oracle;

pub(crate) use bundle::{EngineBundle, StringWatermark};

#[derive(Clone)]
struct EngineDelta {
    mem: WordDelta<u32>,
    tokens: crate::token::TokenDelta,
    glue: WordDelta<GlueSpec>,
    tables: crate::eqtb::TablesDelta,
    hash: PlainDelta<crate::hash::HashEntry>,
    prim: PlainDelta<crate::hash::PrimEntry>,
}

impl EngineDelta {
    fn seal(st: &mut EngineState) -> Self {
        let mut delta = Self::take_open(st);
        delta.compact();
        delta
    }

    fn take_open(st: &mut EngineState) -> Self {
        Self {
            mem: st.mem.journal.seal(&st.mem.data),
            tokens: st.mem.tokens.seal_journal(),
            glue: st.mem.glue.seal_journal(),
            tables: st.eqtb.seal_tables(),
            hash: st.eqtb.journal.hash.seal(),
            prim: st.eqtb.journal.prim.seal(),
        }
    }

    fn snapshot_open(st: &EngineState) -> Self {
        Self {
            mem: st.mem.journal.snapshot(),
            tokens: st.mem.tokens.snapshot_journal(),
            glue: st.mem.glue.journal.snapshot(),
            tables: st.eqtb.snapshot_tables(),
            hash: st.eqtb.journal.hash.snapshot(),
            prim: st.eqtb.journal.prim.snapshot(),
        }
    }

    fn apply(&self, st: &mut EngineState) {
        self.mem.apply_to(&mut st.mem.data);
        st.mem.tokens.apply(&self.tokens);
        self.glue.apply_to(st.mem.glue.specs_mut());
        self.tables.apply_to(&mut st.eqtb);
        self.hash.apply_to(&mut st.eqtb.hash);
        self.prim.apply_to(&mut st.eqtb.prim);
    }

    fn compose_with_older(&mut self, older: EngineDelta) {
        self.mem.compose_with_older(older.mem);
        self.tokens.compose_with_older(older.tokens);
        self.glue.compose_with_older(older.glue);
        self.tables.compose_with_older(older.tables);
        self.hash.compose_with_older(older.hash);
        self.prim.compose_with_older(older.prim);
        self.compact();
    }

    fn compact(&mut self) {
        self.mem.compact();
        self.tokens.compact();
        self.glue.compact();
        self.tables.compact();
        self.hash.compact();
        self.prim.compact();
    }

    fn allocated_bytes(&self) -> usize {
        self.mem.allocated_bytes()
            + self.tokens.allocated_bytes()
            + self.glue.allocated_bytes()
            + self.tables.allocated_bytes()
            + self.hash.allocated_bytes()
            + self.prim.allocated_bytes()
    }
}

struct Rung<B> {
    /// Reverse delta to the predecessor, absent only on rung 0.
    undo: Option<EngineDelta>,
    bundle: B,
    clock: u64,
    resumable: bool,
    protected: bool,
    fence: Option<ReplayFence>,
    digests: Vec<oracle::ComponentDigest>,
}

pub(crate) struct CheckpointLadder<B> {
    rungs: Vec<Rung<B>>,
    verify_rewinds: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct RestoreMismatch {
    rung: usize,
    component: &'static str,
    expected: u64,
    actual: u64,
}

/// Accounted-memory policy for a checkpoint ladder.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LadderBudget {
    pub(crate) target_bytes: usize,
    pub(crate) hard_bytes: usize,
    pub(crate) min_recent_rungs: usize,
    /// Thin by spacing alone rather than keeping rungs near the latest clock.
    pub(crate) spread_evenly: bool,
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

/// Per-rung charge for container headers, allocator rounding and small
/// uninspected strings.
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
    fn push(
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

    /// Undo the open epoch, then published rung deltas newest-first, and
    /// restore the target's non-journaled bundle.
    fn rewind_to(
        &mut self,
        st: &mut EngineState,
        target: usize,
        restore: impl FnOnce(&mut EngineState, &B),
    ) {
        assert!(target < self.rungs.len(), "checkpoint target out of range");
        EngineDelta::take_open(st).apply(st);
        for rung in self.rungs[target + 1..].iter().rev() {
            rung.undo
                .as_ref()
                .expect("non-base rung without undo")
                .apply(st);
        }
        restore(st, &self.rungs[target].bundle);
        self.rungs.truncate(target + 1);
    }

    /// Remove an interior rung. Its successor absorbs the older undo delta
    /// but keeps its own bundle and metadata.
    fn decimate(&mut self, index: usize) -> bool {
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

    /// Protect the `count` rungs up to and including `index` against
    /// budget-driven eviction.
    ///
    /// Continuation work advances the clock far past these rungs, which would
    /// make them the cheapest to drop although the next edit replays from one.
    pub(crate) fn protect_rungs_before(&mut self, index: usize, count: usize) -> bool {
        if index >= self.rungs.len() {
            return false;
        }
        for rung in &mut self.rungs[(index + 1).saturating_sub(count)..=index] {
            rung.protected = true;
        }
        true
    }

    /// Let budget pressure evict `index` again. The base rung stays protected.
    pub(crate) fn unprotect(&mut self, index: usize) {
        if index > 0 {
            if let Some(rung) = self.rungs.get_mut(index) {
                rung.protected = false;
            }
        }
    }

    pub(crate) fn rung_clock(&self, index: usize) -> Option<u64> {
        self.rungs.get(index).map(|rung| rung.clock)
    }

    pub(crate) fn rung_at_clock(&self, clock: u64) -> Option<usize> {
        self.rungs.iter().position(|rung| rung.clock == clock)
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
    fn from_engine_verified(st: &EngineState, strings_base: StringWatermark) -> Self {
        let mut ladder = Self::from_engine(st, strings_base);
        ladder.enable_verification(st);
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

    fn capture_candidate(&self, st: &EngineState, strings_base: StringWatermark) -> EngineBundle {
        EngineBundle::capture(st, strings_base, self.rungs.last().map(|rung| &rung.bundle))
    }

    fn record_digests(&mut self, st: &EngineState, index: usize) {
        if self.verify_rewinds {
            self.rungs[index].digests = oracle::digest_state(st);
        }
    }

    #[cfg(test)]
    fn push_engine(
        &mut self,
        st: &mut EngineState,
        strings_base: StringWatermark,
        resumable: bool,
        protected: bool,
    ) -> usize {
        let bundle = self.capture_candidate(st, strings_base);
        let index = self.push(st, bundle, st.scanner.logical_clock, resumable, protected);
        self.record_digests(st, index);
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
                let fence = rung.fence.as_ref()?;
                (rung.resumable && fence.is_eligible(snapshot)).then(|| (index, fence.clone()))
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

    fn rewind_engine_checked(
        &mut self,
        st: &mut EngineState,
        target: usize,
    ) -> Result<(), RestoreMismatch> {
        self.rewind_to(st, target, |state, bundle| bundle.restore(state));
        self.verify_rung(st, target)
    }

    fn verify_rung(&self, st: &EngineState, target: usize) -> Result<(), RestoreMismatch> {
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
    fn verify_all_rewinds(&mut self, st: &mut EngineState) -> Result<(), RestoreMismatch> {
        for target in (0..self.rungs.len()).rev() {
            self.rewind_engine_checked(st, target)?;
        }
        Ok(())
    }

    pub(crate) fn accounted_bytes(&self) -> usize {
        ladder_bytes(
            self.rungs
                .iter()
                .map(|rung| (&rung.bundle, rung.fence.as_ref(), rung.undo.as_ref())),
        )
    }

    #[cfg(test)]
    fn admit_engine_rung(
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
        let candidate = self.capture_candidate(st, strings_base);
        let mut plan: Vec<PlannedRung> = self
            .rungs
            .iter()
            .enumerate()
            .map(|(index, rung)| PlannedRung {
                source: Some(index),
                clock: rung.clock,
                protected: rung.protected,
                undo: rung.undo.as_ref().map(Cow::Borrowed),
            })
            .collect();
        plan.push(PlannedRung {
            source: None,
            clock: st.scanner.logical_clock,
            protected,
            undo: Some(Cow::Owned(EngineDelta::snapshot_open(st))),
        });
        let planned_bytes = |plan: &[PlannedRung]| {
            ladder_bytes(plan.iter().map(|planned| {
                let undo = planned.undo.as_deref();
                match planned.source {
                    Some(index) => {
                        let rung = &self.rungs[index];
                        (&rung.bundle, rung.fence.as_ref(), undo)
                    }
                    None => (&candidate, fence.as_ref(), undo),
                }
            }))
        };

        let mut dropped = 0;
        while planned_bytes(&plan) > budget.target_bytes {
            let Some(index) =
                pick_rung_to_drop(&plan, budget.min_recent_rungs, false, budget.spread_evenly)
            else {
                break;
            };
            drop_planned_rung(&mut plan, index);
            dropped += 1;
        }

        // The soft target never overrides fresh protections or the recent
        // window. The hard cap does: progressively thin any interior rung.
        while planned_bytes(&plan) > budget.hard_bytes {
            let Some(index) = pick_rung_to_drop(&plan, 0, true, budget.spread_evenly) else {
                return RungAdmission::RungNotAdmitted {
                    minimum_bytes: planned_bytes(&plan),
                };
            };
            drop_planned_rung(&mut plan, index);
            dropped += 1;
        }

        let mut kept = vec![false; self.rungs.len()];
        for index in plan.iter().filter_map(|planned| planned.source) {
            kept[index] = true;
        }
        drop(plan);

        let top = self.push_with_fence(
            st,
            candidate,
            st.scanner.logical_clock,
            resumable,
            protected,
            fence,
        );
        self.record_digests(st, top);
        for index in (1..top).rev() {
            if !kept[index] {
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

/// A rung as admission planning sees it. `source` is `None` for the candidate.
struct PlannedRung<'a> {
    source: Option<usize>,
    clock: u64,
    protected: bool,
    undo: Option<Cow<'a, EngineDelta>>,
}

/// Counts each shared hyphen table once, keyed by its Arc allocation.
fn ladder_bytes<'a>(
    rungs: impl Iterator<
        Item = (
            &'a EngineBundle,
            Option<&'a ReplayFence>,
            Option<&'a EngineDelta>,
        ),
    >,
) -> usize {
    let mut total = 0;
    let mut shared = HashSet::new();
    for (bundle, fence, undo) in rungs {
        total += RUNG_CONTAINER_ALLOWANCE
            + bundle.allocated_bytes()
            + fence.map_or(0, ReplayFence::allocated_bytes)
            + undo.map_or(0, EngineDelta::allocated_bytes);
        let (identity, bytes) = bundle.shared_hyph_tables();
        if shared.insert(identity) {
            total += bytes;
        }
    }
    total
}

fn pick_rung_to_drop(
    plan: &[PlannedRung],
    min_recent_rungs: usize,
    relax: bool,
    spread_evenly: bool,
) -> Option<usize> {
    if plan.len() <= 2 {
        return None;
    }
    let recent_start = plan.len().saturating_sub(min_recent_rungs.max(1));
    let top = plan[plan.len() - 1].clock;
    let mut best: Option<(u64, u64, usize)> = None;
    for index in 1..plan.len() - 1 {
        if !relax && (plan[index].protected || index >= recent_start) {
            continue;
        }
        let gap = plan[index + 1].clock.saturating_sub(plan[index - 1].clock);
        let age = if spread_evenly {
            1
        } else {
            top.saturating_sub(plan[index].clock).saturating_add(1)
        };
        if best.is_none_or(|(best_gap, best_age, _)| {
            u128::from(gap) * u128::from(best_age) < u128::from(best_gap) * u128::from(age)
        }) {
            best = Some((gap, age, index));
        }
    }
    best.map(|(_, _, index)| index)
}

fn drop_planned_rung(plan: &mut Vec<PlannedRung>, index: usize) {
    let older = plan
        .remove(index)
        .undo
        .expect("interior planned rung without undo")
        .into_owned();
    plan[index]
        .undo
        .as_mut()
        .expect("planned successor without undo")
        .to_mut()
        .compose_with_older(older);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::units::MemPtr;

    #[derive(Clone, Copy)]
    struct Bundle {
        free_pairs: MemPtr,
        fmem_ptr: i32,
        tokens: crate::token::TokenStoreMarks,
    }

    fn bundle(st: &EngineState) -> Bundle {
        Bundle {
            free_pairs: st.mem.free.0[2],
            fmem_ptr: st.fonts.fmem_ptr,
            tokens: st.mem.tokens.marks(),
        }
    }

    fn restore(st: &mut EngineState, bundle: &Bundle) {
        st.mem.free.0[2] = bundle.free_pairs;
        st.fonts.fmem_ptr = bundle.fmem_ptr;
        st.mem.tokens.restore_marks(bundle.tokens);
    }

    const CS: crate::cs::CsId = crate::cs::CsId::active(crate::command::Usv::nth(1));

    const COUNT1: crate::eqtb::IntLoc = crate::eqtb::IntLoc::count(1);

    fn state() -> EngineState {
        let mut st = EngineState::default();
        st.mem.data = crate::mem_types::zeroed_vec(8);
        st.eqtb.hash = vec![crate::hash::HashEntry::default(); 1];
        st.eqtb.meanings = crate::eqtb::Table::at_level(4, Command::UndefinedCs, 0);
        st.eqtb.ints = crate::eqtb::Table::new(crate::eqtb::IntLoc::LEN, 0);
        crate::journal::arm(&mut st);
        st
    }

    #[test]
    fn multi_rung_rewind_restores_arrays_and_scalars() {
        let mut st = state();
        let mut ladder = CheckpointLadder::new(bundle(&st), 0);

        st.mem.set_word(1, 10);
        st.eqtb.ints.set(COUNT1, 13, 2);
        st.eqtb.set_meaning(CS, Command::Halign, 2);
        st.mem.free.0[2] = crate::units::MemIdx::from_raw(12);
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 1, true, false);

        st.mem.set_word(1, 20);
        st.mem.set_word(3, 21);
        st.eqtb.ints.set(COUNT1, 23, 3);
        st.eqtb.set_meaning(CS, Command::Vrule, 3);
        st.fonts.fmem_ptr = 2;
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 2, true, false);

        st.mem.set_word(4, 30); // open epoch after the newest rung
        st.mem.free.0[2] = crate::units::MemIdx::from_raw(31);
        ladder.rewind_to(&mut st, 1, restore);

        assert_eq!(st.mem.word(1), 10);
        assert_eq!(st.mem.word(3), 0);
        assert_eq!(st.mem.word(4), 0);
        assert_eq!(
            (st.eqtb.ints.get(COUNT1), st.eqtb.ints.level(COUNT1)),
            (13, 2)
        );
        assert_eq!(
            (st.eqtb.meaning(CS), st.eqtb.cs_level(CS)),
            (Command::Halign, 2)
        );
        assert_eq!(st.mem.free.0[2], crate::units::MemIdx::from_raw(12));
        assert_eq!(st.fonts.fmem_ptr, 0);
        assert_eq!(ladder.len(), 2);
    }

    #[test]
    fn rewind_restores_the_token_store_across_growth() {
        use crate::token::Token;
        let mut st = state();
        let mut ladder = CheckpointLadder::new(bundle(&st), 0);

        let a = st.mem.tokens.single(Token(5)).unwrap();
        let marks = st.mem.tokens.marks();
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 1, true, false);

        st.mem.tokens.append(a, Token(7)).unwrap();
        st.mem.tokens.map_tokens(a, |_| Token(9));
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 2, true, false);

        st.mem.tokens.flush(a);
        let reused = st.mem.tokens.single(Token(11)).unwrap();
        assert_eq!(reused, a, "the freed run and entry are reused");
        ladder.rewind_to(&mut st, 1, restore);

        assert_eq!(st.mem.tokens.marks(), marks);
        assert_eq!(st.mem.tokens.iter(a).collect::<Vec<_>>(), [Token(5)]);
        let d = st.mem.tokens.single(Token(3)).unwrap();
        assert_eq!(st.mem.tokens.iter(d).collect::<Vec<_>>(), [Token(3)]);
        assert_eq!(st.mem.tokens.iter(a).collect::<Vec<_>>(), [Token(5)]);
    }

    #[test]
    fn decimation_preserves_rewind_truth() {
        let mut st = state();
        let mut ladder = CheckpointLadder::new(bundle(&st), 0);
        st.mem.set_word(1, 1);
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 1, false, false);
        st.mem.set_word(1, 2);
        st.mem.set_word(2, 2);
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 2, false, false);

        assert!(ladder.decimate(1));
        assert_eq!(ladder.len(), 2);
        ladder.rewind_to(&mut st, 0, restore);
        assert_eq!(st.mem.word(1), 0);
        assert_eq!(st.mem.word(2), 0);
    }

    #[test]
    fn spreading_evenly_drops_the_closest_rung_however_recent() {
        let plan = [0, 100, 200, 900, 950, 1000].map(|clock| PlannedRung {
            source: None,
            clock,
            protected: false,
            undo: None,
        });
        // By age the oldest close pair goes, evenly the tightest pair near the top.
        assert_eq!(pick_rung_to_drop(&plan, 0, false, false), Some(1));
        assert_eq!(pick_rung_to_drop(&plan, 0, false, true), Some(4));
    }

    #[test]
    fn protected_and_edge_rungs_cannot_be_decimated() {
        let mut st = state();
        let mut ladder = CheckpointLadder::new(bundle(&st), 0);
        st.mem.set_word(1, 1);
        let rung = bundle(&st);
        ladder.push(&mut st, rung, 1, false, true);
        st.mem.set_word(1, 2);
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
        st.strings.str_start = vec![crate::units::PoolPos::ZERO; 4];
        let strings_base = StringWatermark::capture(&st);
        let mut ladder = CheckpointLadder::from_engine(&st, strings_base);

        st.scanner.logical_clock = 10;
        st.mem.free.0[2] = crate::units::MemIdx::from_raw(20);
        st.mem.set_word(1, 30);
        ladder.push_engine(&mut st, strings_base, true, true);
        st.scanner.logical_clock = 40;
        st.mem.free.0[2] = crate::units::MemIdx::from_raw(50);
        st.mem.set_word(1, 60);

        ladder.rewind_engine(&mut st, 1);
        assert_eq!(st.scanner.logical_clock, 10);
        assert_eq!(st.mem.free.0[2], crate::units::MemIdx::from_raw(20));
        assert_eq!(st.mem.word(1), 30);
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
                spread_evenly: false,
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
        st.mem.set_word(1, 7);
        let rejected = ladder.admit_engine_rung(
            &mut st,
            strings_base,
            true,
            false,
            LadderBudget {
                target_bytes: minimum,
                hard_bytes: minimum,
                min_recent_rungs: 1,
                spread_evenly: false,
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
                spread_evenly: false,
            },
        );
        assert!(matches!(admitted, RungAdmission::Admitted { .. }));
        st.mem.set_word(1, 8);
        ladder.rewind_engine(&mut st, 0);
        assert_eq!(st.mem.word(1), 0);
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
            spread_evenly: false,
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
            spread_evenly: false,
        };
        for clock in 2..10 {
            st.scanner.logical_clock = clock;
            st.mem.set_word(1, clock as i32);
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
            spread_evenly: false,
        };
        for clock in 1..=4 {
            st.scanner.logical_clock = clock;
            st.mem.set_word(1, clock as i32);
            ladder.admit_engine_rung(&mut st, strings_base, true, false, unlimited);
        }
        let horizon = ladder.len() - 1;
        assert!(ladder.protect_rungs_before(horizon, horizon + 1));
        let clocks: Vec<u64> = (0..=horizon)
            .map(|index| ladder.rung_clock(index).unwrap())
            .collect();

        // Continuation work advances the clock far past the protected prefix
        // and must stay under a budget that cannot hold every rung.
        let budget = LadderBudget {
            target_bytes: ladder.accounted_bytes(),
            hard_bytes: usize::MAX,
            min_recent_rungs: 1,
            spread_evenly: false,
        };
        for clock in 100..120 {
            st.scanner.logical_clock = clock;
            st.mem.set_word(1, clock as i32);
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
        st.strings.str_start = vec![crate::units::PoolPos::ZERO; 4];
        let strings_base = StringWatermark::capture(&st);
        let mut ladder = CheckpointLadder::from_engine_verified(&st, strings_base);

        st.mem.data[1] = 99;
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
        st.strings.str_start = vec![crate::units::PoolPos::ZERO; 4];
        let strings_base = StringWatermark::capture(&st);
        let mut ladder = CheckpointLadder::from_engine_verified(&st, strings_base);
        for clock in 1..=4 {
            st.scanner.logical_clock = clock;
            st.mem.set_word(1, clock as i32);
            ladder.push_engine(&mut st, strings_base, false, false);
        }
        assert!(ladder.decimate(2));
        ladder.verify_all_rewinds(&mut st).unwrap();
        assert_eq!(st.mem.word(1), 0);
    }

    #[test]
    fn prefix_detour_and_rewind_reproduce_state_and_output() {
        fn run_prefix(st: &mut EngineState, value: i32) {
            st.mem.set_word(1, value);
            st.shipout.dvi.init();
            for b in value.to_le_bytes() {
                st.shipout.dvi.set_char(u16::from(b)).unwrap();
            }
            st.scanner.logical_clock += 1;
        }

        let mut st = state();
        st.strings.str_pool = vec![0; 4];
        st.strings.str_start = vec![crate::units::PoolPos::ZERO; 4];
        let strings_base = StringWatermark::capture(&st);
        let mut ladder = CheckpointLadder::from_engine_verified(&st, strings_base);
        run_prefix(&mut st, 17);
        let expected_output = st.shipout.dvi.digest_view().1.to_vec();
        let expected_state = oracle::digest_state(&st);

        ladder.rewind_engine(&mut st, 0);
        run_prefix(&mut st, 23);
        ladder.push_engine(&mut st, strings_base, false, false);
        ladder.rewind_engine(&mut st, 0);
        run_prefix(&mut st, 17);

        assert_eq!(st.shipout.dvi.digest_view().1, expected_output);
        assert_eq!(oracle::digest_state(&st), expected_state);
    }
}
