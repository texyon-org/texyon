//! Breaking vertical lists: `vert_break`, `vsplit`, `prune_page_top` and `do_marks`.

use super::{PackOrigin, PackOutcome, PackSpec, print_vpack_diag};
use crate::base::codes::{BoxCode, MarkCode};
use crate::base::consts::{EJECT_PENALTY, INF_BAD, INF_PENALTY, MAX_DIMEN, MAX_HALFWORD};
use crate::base::params::{DimenPar, GluePar, IntPar};
use crate::base::token::TokenList;
use crate::base::units::MemIdx;
use crate::base::units::{GlueOrder, MemPtr, Scaled};
use crate::memory::glue_store::GlueSpecId;
use crate::memory::mem::MemState;
use crate::memory::mem::TEMP_HEAD;
use crate::memory::node::{
    AnyNode, BoxNode, GlueNode, KernNode, MarkNode, NodePtr, WhatsitVariant,
};
use crate::print::errors::OrConfusion;
use crate::print::errors::{Abort, IgnoredError};
use crate::scanning::scan::ScannerState;
use crate::shipout::ShipoutState;
use crate::state::EngineState;
use crate::tables::eqtb::EqtbState;
use crate::tables::eqtb::Register;

/// What `do_marks` does to each mark class.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarksAction {
    VsplitInit,
    FireUpInit,
    FireUpDone,
    DestroyMarks,
}

/// Store `v` in the first empty slot of a two-slot record.
#[inline]
pub(super) fn push_slot<T>(slots: &mut [Option<T>; 2], v: T) {
    let i = usize::from(slots[0].is_some());
    slots[i] = Some(v);
}

struct VertBreakCursor {
    p: MemPtr,
    prev_p: MemPtr,
    /// Whether glue after the node at `p` is a legal breakpoint.
    breakable: bool,
    /// The same for `prev_p`.
    prev_breakable: bool,
    least_cost: i32,
    best_place: MemPtr,
    prev_dp: Scaled,
    finish_glue_after_shrinkage: Option<NodePtr<GlueNode>>,
    /// Height, stretch and shrink of the material so far, as in WEB's `active_height`.
    active_height: crate::build::linebreak::Widths,
    /// The natural height plus depth of the material before `best_place`.
    best_height_plus_depth: Scaled,
}

enum VertBreakStep {
    Done(MemPtr),
    InfiniteShrinkage {
        glue: NodePtr<GlueNode>,
        spec: GlueSpecId,
    },
}

enum VertBreakNext {
    NotFound,
    UpdateHeights(VertBreakUpdateSource),
    CheckPi {
        pi: i32,
        update_after: Option<VertBreakUpdateSource>,
    },
}

#[derive(Copy, Clone)]
enum VertBreakUpdateSource {
    Kern(NodePtr<KernNode>),
    Glue(NodePtr<GlueNode>),
}

impl VertBreakCursor {
    fn start(p: MemPtr) -> Self {
        Self {
            p,
            prev_p: p,
            breakable: false,
            prev_breakable: false,
            least_cost: MAX_HALFWORD,
            best_place: None,
            prev_dp: Scaled::ZERO,
            finish_glue_after_shrinkage: None,
            active_height: [Scaled::ZERO; 7],
            best_height_plus_depth: Scaled::ZERO,
        }
    }
}

impl MemState {
    fn advance_vert_break(&self, cursor: &mut VertBreakCursor, d: Scaled) {
        if cursor.prev_dp > d {
            cursor.active_height[1] += cursor.prev_dp - d;
            cursor.prev_dp = d;
        }
        cursor.prev_p = cursor.p;
        cursor.prev_breakable = cursor.breakable;
        cursor.p = cursor.p.and_then(|p| p.link(self));
    }

    fn finish_vert_break_updated_height(
        &self,
        cursor: &mut VertBreakCursor,
        width: Scaled,
        d: Scaled,
    ) {
        cursor.active_height[1] += cursor.prev_dp + width;
        cursor.prev_dp = Scaled::ZERO;
        self.advance_vert_break(cursor, d);
    }

    fn classify_vert_break_next(
        &self,
        cursor: &mut VertBreakCursor,
    ) -> Result<VertBreakNext, Abort> {
        let Some(p) = cursor.p else {
            return Ok(VertBreakNext::CheckPi {
                pi: EJECT_PENALTY,
                update_after: None,
            });
        };

        let node = self.classify(p).or_confusion("vertbreak")?;
        cursor.breakable = node.precedes_break();
        match node {
            AnyNode::Box(b) => {
                cursor.active_height[1] += cursor.prev_dp + self[b].height;
                cursor.prev_dp = self[b].depth;
                Ok(VertBreakNext::NotFound)
            }
            AnyNode::Rule(rule) => {
                cursor.active_height[1] += cursor.prev_dp + self[rule].height.to_scaled();
                cursor.prev_dp = self[rule].depth.to_scaled();
                Ok(VertBreakNext::NotFound)
            }
            AnyNode::Whatsit(w) => {
                if let Some(WhatsitVariant::Pic(pic)) = w.variant(self) {
                    cursor.active_height[1] += cursor.prev_dp + self[pic].height;
                    cursor.prev_dp = self[pic].depth;
                }
                Ok(VertBreakNext::NotFound)
            }
            AnyNode::Glue(glue) => {
                if cursor.prev_breakable {
                    Ok(VertBreakNext::CheckPi {
                        pi: 0,
                        update_after: Some(VertBreakUpdateSource::Glue(glue)),
                    })
                } else {
                    Ok(VertBreakNext::UpdateHeights(VertBreakUpdateSource::Glue(
                        glue,
                    )))
                }
            }
            AnyNode::Kern(kern) => {
                let next = self[kern].link;
                if next.is_some_and(|n| matches!(self.classify(n), Ok(AnyNode::Glue(_)))) {
                    Ok(VertBreakNext::CheckPi {
                        pi: 0,
                        update_after: Some(VertBreakUpdateSource::Kern(kern)),
                    })
                } else {
                    Ok(VertBreakNext::UpdateHeights(VertBreakUpdateSource::Kern(
                        kern,
                    )))
                }
            }
            AnyNode::Penalty(penalty) => Ok(VertBreakNext::CheckPi {
                pi: self[penalty].penalty,
                update_after: None,
            }),
            AnyNode::Ins(_) | AnyNode::Mark(_) => Ok(VertBreakNext::NotFound),
            _ => Err(Abort::Confusion("vertbreak")),
        }
    }

    fn vert_break_resume(
        &mut self,
        cursor: &mut VertBreakCursor,
        h: Scaled,
        d: Scaled,
    ) -> Result<VertBreakStep, Abort> {
        loop {
            if let Some(glue) = cursor.finish_glue_after_shrinkage.take() {
                let spec = self[glue].glue_ptr;
                self.finish_vert_break_updated_height(cursor, self[spec].width, d);
                continue;
            }

            let update_source = match self.classify_vert_break_next(cursor)? {
                VertBreakNext::NotFound => None,
                VertBreakNext::UpdateHeights(source) => Some(source),
                VertBreakNext::CheckPi { pi, update_after } => {
                    if pi < INF_PENALTY {
                        let aw = &cursor.active_height;
                        let b = if aw[1] < h {
                            if aw[3] != Scaled::ZERO
                                || aw[4] != Scaled::ZERO
                                || aw[5] != Scaled::ZERO
                            {
                                0
                            } else {
                                crate::base::helpers::badness(h - aw[1], aw[2])
                            }
                        } else if aw[1] - h > aw[6] {
                            MAX_HALFWORD
                        } else {
                            crate::base::helpers::badness(aw[1] - h, aw[6])
                        };
                        let b = if b < MAX_HALFWORD {
                            if pi <= EJECT_PENALTY {
                                pi
                            } else if b < INF_BAD {
                                b + pi
                            } else {
                                100_000
                            }
                        } else {
                            b
                        };
                        if b <= cursor.least_cost {
                            cursor.best_place = cursor.p;
                            cursor.least_cost = b;
                            cursor.best_height_plus_depth =
                                cursor.active_height[1] + cursor.prev_dp;
                        }
                        if b == MAX_HALFWORD || pi <= EJECT_PENALTY {
                            return Ok(VertBreakStep::Done(cursor.best_place));
                        }
                    }
                    update_after
                }
            };

            match update_source {
                Some(VertBreakUpdateSource::Kern(kern)) => {
                    self.finish_vert_break_updated_height(cursor, self[kern].width, d);
                }
                Some(VertBreakUpdateSource::Glue(glue)) => {
                    let spec = self[glue].glue_ptr;
                    cursor.active_height[2 + self[spec].stretch_order.index()] +=
                        self[spec].stretch;
                    cursor.active_height[6] += self[spec].shrink;
                    if self[spec].shrink_order != GlueOrder::Normal
                        && self[spec].shrink != Scaled::ZERO
                    {
                        cursor.finish_glue_after_shrinkage = Some(glue);
                        return Ok(VertBreakStep::InfiniteShrinkage { glue, spec });
                    }
                    self.finish_vert_break_updated_height(cursor, self[spec].width, d);
                }
                None => self.advance_vert_break(cursor, d),
            }
        }
    }
}

fn print_vert_break_infinite_shrinkage_error(st: &mut EngineState) -> Result<(), Abort> {
    const MESSAGE: &str = "Infinite glue shrinkage found in box being split";
    if st
        .eqtb
        .ignores_primitive_error(IgnoredError::VsplitInfiniteShrink)
    {
        crate::print::print_ignored_err(st, MESSAGE);
        return Ok(());
    }
    crate::print::errors::error(
        st,
        MESSAGE,
        &[
            r"The box you are \vsplitting contains some infinitely",
            r"shrinkable glue, e.g., `\vss' or `\vskip 0pt minus 1fil'.",
            "Such glue doesn't belong there; but you can safely proceed,",
            "since the offensive shrinkability has been made finite.",
        ],
    )
}

impl MemState {
    fn repair_vert_break_infinite_shrinkage(
        &mut self,
        glue: NodePtr<GlueNode>,
        spec: GlueSpecId,
    ) -> Result<(), Abort> {
        let spec = self.finite_shrink(spec)?;
        self[glue].glue_ptr = spec;
        Ok(())
    }
}

/// The best place to break the vlist `p` for a height `h` with maximum depth `d`,
/// and the natural height plus depth of the material before it.
pub(crate) fn vert_break(
    st: &mut EngineState,
    p: MemPtr,
    h: Scaled,
    d: Scaled,
) -> Result<(MemPtr, Scaled), Abort> {
    let mut cursor = VertBreakCursor::start(p);
    loop {
        match st.mem.vert_break_resume(&mut cursor, h, d)? {
            VertBreakStep::Done(best_place) => {
                return Ok((best_place, cursor.best_height_plus_depth));
            }
            VertBreakStep::InfiniteShrinkage { glue, spec } => {
                print_vert_break_infinite_shrinkage_error(st)?;
                st.mem.repair_vert_break_infinite_shrinkage(glue, spec)?;
            }
        }
    }
}

struct VsplitState {
    /// The box register being split.
    n: crate::tables::eqtb::Register,
    h: Scaled,
    v: NodePtr<BoxNode>,
    split_max_d: Scaled,
    vert_break: VertBreakCursor,
}

#[derive(Copy, Clone)]
struct VsplitOutcome {
    result: Option<NodePtr<BoxNode>>,
    packages: [Option<PackOutcome>; 2],
}

enum VsplitStart {
    Done(VsplitOutcome),
    NeedsVboxDiagnostic,
    Ready(VsplitState),
}

enum VsplitStep {
    Done(VsplitOutcome),
    InfiniteShrinkage {
        glue: NodePtr<GlueNode>,
        spec: GlueSpecId,
    },
}

impl MemState {
    /// The contents of box register `n`, leaving `n` in `cur_val`.
    fn reset_vsplit_marks(&mut self, scanner: &mut ScannerState) -> Result<(), Abort> {
        self.flush_node_list(scanner.disc_ptr[BoxCode::Vsplit as usize])?;
        scanner.disc_ptr[BoxCode::Vsplit as usize] = None;

        self.do_marks(&mut scanner.mark_classes, MarksAction::VsplitInit);

        if !scanner.cur_mark[MarkCode::SplitFirst].is_null() {
            self.tokens
                .delete_ref(scanner.cur_mark[MarkCode::SplitFirst]);
            scanner.cur_mark[MarkCode::SplitFirst] = TokenList::NULL;
            self.tokens.delete_ref(scanner.cur_mark[MarkCode::SplitBot]);
            scanner.cur_mark[MarkCode::SplitBot] = TokenList::NULL;
        }

        Ok(())
    }
}

fn start_vsplit(
    st: &mut EngineState,
    n: crate::tables::eqtb::Register,
    h: Scaled,
) -> Result<VsplitStart, Abort> {
    let v = crate::scanning::scan::fetch_box(st, n);
    st.mem.reset_vsplit_marks(&mut st.scanner)?;

    let v = match v {
        None => {
            return Ok(VsplitStart::Done(VsplitOutcome {
                result: None,
                packages: [None, None],
            }));
        }
        Some(v) if st.mem[v].is_vlist() => v,
        Some(_) => return Ok(VsplitStart::NeedsVboxDiagnostic),
    };

    Ok(VsplitStart::Ready(VsplitState {
        n,
        h,
        v,
        split_max_d: st.eqtb.dimen_par(DimenPar::SplitMaxDepth),
        vert_break: VertBreakCursor::start(st.mem[v].list_ptr),
    }))
}

impl MemState {
    fn collect_split_mark(
        &mut self,
        scanner: &mut ScannerState,
        mark: NodePtr<MarkNode>,
    ) -> Result<(), Abort> {
        let mark_class = self[mark].mark_class;
        let mark_ptr = self[mark].mark_ptr;
        if mark_class != 0 {
            let class = Register::new(mark_class).or_confusion("marks")?;
            let m = scanner.mark_classes.entry(class);
            if m[MarkCode::SplitFirst].is_null() {
                m[MarkCode::SplitFirst] = mark_ptr;
                self.tokens.add_ref(mark_ptr);
            } else {
                self.tokens.delete_ref(m[MarkCode::SplitBot]);
            }
            m[MarkCode::SplitBot] = mark_ptr;
            self.tokens.add_ref(mark_ptr);
        } else if scanner.cur_mark[MarkCode::SplitFirst].is_null() {
            scanner.cur_mark[MarkCode::SplitFirst] = mark_ptr;
            scanner.cur_mark[MarkCode::SplitBot] = mark_ptr;
            self.tokens.add_ref(mark_ptr);
            self.tokens.add_ref(mark_ptr);
        } else {
            self.tokens.delete_ref(scanner.cur_mark[MarkCode::SplitBot]);
            scanner.cur_mark[MarkCode::SplitBot] = mark_ptr;
            self.tokens.add_ref(mark_ptr);
        }
        Ok(())
    }

    /// Detach the part of `state.v`'s list before `q`, recording its marks.
    fn split_vlist_at_break(
        &mut self,
        scanner: &mut ScannerState,
        state: &VsplitState,
        q: MemPtr,
    ) -> Result<MemPtr, Abort> {
        let mut p = self[state.v].list_ptr;
        if p == q {
            self[state.v].list_ptr = None;
        } else {
            loop {
                let here = p.or_confusion("vsplit")?;
                if let AnyNode::Mark(mark) = self.classify(here).or_confusion("vsplit")? {
                    self.collect_split_mark(scanner, mark)?;
                }
                if here.link(self) == q {
                    here.set_link(self, None);
                    break;
                }
                p = here.link(self);
            }
        }

        Ok(self[state.v].list_ptr)
    }

    fn finish_vsplit_after_break(
        &mut self,
        eqtb: &mut EqtbState,
        scanner: &mut ScannerState,
        shipout: &mut ShipoutState,
        state: &VsplitState,
        q: MemPtr,
    ) -> Result<VsplitOutcome, Abort> {
        let mut packages = [None, None];
        let p = self.split_vlist_at_break(scanner, state, q)?;
        let save_discards = eqtb.int_par(IntPar::SavingVdiscards) > 0;
        let q = self.prune_page_top(scanner, eqtb, q, save_discards)?;

        state.v.free(self);
        let mut rest = None;
        if !q.is_none() {
            let packed = self.vpackage(eqtb, shipout, q, PackSpec::NATURAL, MAX_DIMEN)?;
            rest = Some(packed.r_box);
            push_slot(&mut packages, packed);
        }
        eqtb.set_box_reg(state.n, rest);

        let packed = self.vpackage(
            eqtb,
            shipout,
            p,
            PackSpec::Exactly(state.h),
            state.split_max_d,
        )?;
        let result = Some(packed.r_box);
        push_slot(&mut packages, packed);

        Ok(VsplitOutcome { result, packages })
    }

    fn vsplit_resume(
        &mut self,
        eqtb: &mut EqtbState,
        scanner: &mut ScannerState,
        shipout: &mut ShipoutState,
        state: &mut VsplitState,
    ) -> Result<VsplitStep, Abort> {
        match self.vert_break_resume(&mut state.vert_break, state.h, state.split_max_d)? {
            VertBreakStep::Done(q) => {
                let outcome = self.finish_vsplit_after_break(eqtb, scanner, shipout, state, q)?;
                Ok(VsplitStep::Done(outcome))
            }
            VertBreakStep::InfiniteShrinkage { glue, spec } => {
                Ok(VsplitStep::InfiniteShrinkage { glue, spec })
            }
        }
    }
}

fn print_vsplit_needs_vbox_error(st: &mut EngineState) -> Result<(), Abort> {
    crate::print::errors::error(
        st,
        (
            "",
            crate::print::Esc("vsplit"),
            " needs a ",
            crate::print::Esc("vbox"),
        ),
        &[
            r"The box you are trying to split is an \hbox.",
            "I can't split such a box, so I'll leave it alone.",
        ],
    )
}

fn print_vsplit_packages(st: &mut EngineState, outcome: VsplitOutcome) {
    for package in outcome.packages.into_iter().flatten() {
        for diag in package.diags.into_iter().flatten() {
            print_vpack_diag(st, package.r_box, diag, PackOrigin::Here);
        }
    }
}

/// `\vsplit n to h`: the box split off the top of box register `n`.
pub(crate) fn vsplit(
    st: &mut EngineState,
    n: crate::tables::eqtb::Register,
    h: Scaled,
) -> Result<Option<NodePtr<BoxNode>>, Abort> {
    let mut state = match start_vsplit(st, n, h)? {
        VsplitStart::Done(outcome) => {
            print_vsplit_packages(st, outcome);
            return Ok(outcome.result);
        }
        VsplitStart::NeedsVboxDiagnostic => {
            print_vsplit_needs_vbox_error(st)?;
            return Ok(None);
        }
        VsplitStart::Ready(state) => state,
    };

    loop {
        match st
            .mem
            .vsplit_resume(&mut st.eqtb, &mut st.scanner, &mut st.shipout, &mut state)?
        {
            VsplitStep::Done(outcome) => {
                print_vsplit_packages(st, outcome);
                return Ok(outcome.result);
            }
            VsplitStep::InfiniteShrinkage { glue, spec } => {
                print_vert_break_infinite_shrinkage_error(st)?;
                st.mem.repair_vert_break_infinite_shrinkage(glue, spec)?;
            }
        }
    }
}

impl MemState {
    /// Insert `\splittopskip` glue between `prev_p` and the box or rule `p`, less its `top`
    /// (height, or depth in upwards mode).
    fn insert_split_top_skip_before(
        &mut self,
        eqtb: &EqtbState,
        prev_p: MemIdx,
        p: MemPtr,
        top: Scaled,
    ) -> Result<(), Abort> {
        let q = self.new_skip_param(eqtb, GluePar::SplitTopSkip)?;
        prev_p.set_link(self, q.ptr());
        self[q].link = p;
        let spec = self[q].glue_ptr;
        let avail_ht = self[spec].width;
        if avail_ht > top {
            self[spec].width = avail_ht - top;
        } else {
            self[spec].width = Scaled::ZERO;
        }
        Ok(())
    }

    /// Unlink the discardable `node` after `prev_p`, saving it after `r` (or
    /// flushing it), and return its successor.
    fn prune_discard_node(
        &mut self,
        scanner: &mut ScannerState,
        prev_p: MemIdx,
        r: &mut MemPtr,
        node: MemIdx,
        save_discards: bool,
    ) -> Result<MemPtr, Abort> {
        let q = node.link(self);
        node.set_link(self, None);
        prev_p.set_link(self, q);
        if save_discards {
            if scanner.disc_ptr[BoxCode::Vsplit as usize].is_none() {
                scanner.disc_ptr[BoxCode::Vsplit as usize] = Some(node);
            } else {
                r.or_confusion("discards")?.set_link(self, Some(node));
            }
            *r = Some(node);
        } else {
            self.flush_node_list(Some(node))?;
        }
        Ok(q)
    }

    /// Remove the discardable items from the top of the vlist `p`, putting
    /// `\splittopskip` before its first box or rule.
    pub fn prune_page_top(
        &mut self,
        scanner: &mut ScannerState,
        eqtb: &EqtbState,
        mut p: MemPtr,
        s: bool,
    ) -> Result<MemPtr, Abort> {
        let mut prev_p = TEMP_HEAD;
        let mut r = None;
        TEMP_HEAD.set_link(self, p);
        let upwards = eqtb.int_par(IntPar::XetexUpwards) > 0;

        while let Some(here) = p {
            let node = self.classify(here).or_confusion("pruning")?;
            let top = match node {
                AnyNode::Box(b) if upwards => self[b].depth,
                AnyNode::Box(b) => self[b].height,
                AnyNode::Rule(rule) if upwards => self[rule].depth.to_scaled(),
                AnyNode::Rule(rule) => self[rule].height.to_scaled(),
                AnyNode::Whatsit(_) | AnyNode::Mark(_) | AnyNode::Ins(_) => {
                    prev_p = here;
                    p = here.link(self);
                    continue;
                }
                AnyNode::Glue(_) | AnyNode::Kern(_) | AnyNode::Penalty(_) => {
                    p = self.prune_discard_node(scanner, prev_p, &mut r, here, s)?;
                    continue;
                }
                _ => return Err(Abort::Confusion("pruning")),
            };
            self.insert_split_top_skip_before(eqtb, prev_p, p, top)?;
            p = None;
        }

        Ok(TEMP_HEAD.link(self))
    }

    /// Applies mark action `a` to every mark class above 0, in ascending order,
    /// dropping the classes left without a bottom or split bottom mark.
    pub fn do_marks(&mut self, classes: &mut MarkClasses, a: MarksAction) {
        classes.retain(|marks| self.do_mark_class(a, marks));
    }

    /// Applies mark action `a` to one class's `marks`. Returns whether it keeps any.
    fn do_mark_class(&mut self, a: MarksAction, marks: &mut CurMarks) -> bool {
        match a {
            MarksAction::VsplitInit => {
                if !marks[MarkCode::SplitFirst].is_null() {
                    self.tokens.delete_ref(marks[MarkCode::SplitFirst]);
                    marks[MarkCode::SplitFirst] = TokenList::NULL;
                    self.tokens.delete_ref(marks[MarkCode::SplitBot]);
                    marks[MarkCode::SplitBot] = TokenList::NULL;
                }
            }
            MarksAction::FireUpInit => {
                if !marks[MarkCode::Bot].is_null() {
                    if !marks[MarkCode::Top].is_null() {
                        self.tokens.delete_ref(marks[MarkCode::Top]);
                    }
                    self.tokens.delete_ref(marks[MarkCode::First]);
                    marks[MarkCode::First] = TokenList::NULL;
                    let bot = marks[MarkCode::Bot];
                    if self.tokens.is_empty(bot) {
                        self.tokens.delete_ref(bot);
                        marks[MarkCode::Bot] = TokenList::NULL;
                    } else {
                        self.tokens.add_ref(bot);
                    }
                    marks[MarkCode::Top] = marks[MarkCode::Bot];
                }
            }
            MarksAction::FireUpDone => {
                let top = marks[MarkCode::Top];
                if !top.is_null() && marks[MarkCode::First].is_null() {
                    marks[MarkCode::First] = top;
                    self.tokens.add_ref(top);
                }
            }
            MarksAction::DestroyMarks => {
                for mark in MarkCode::ALL {
                    if !marks[mark].is_null() {
                        self.tokens.delete_ref(marks[mark]);
                        marks[mark] = TokenList::NULL;
                    }
                }
            }
        }
        !(marks[MarkCode::Bot].is_null() && marks[MarkCode::SplitBot].is_null())
    }
}

/// `cur_mark`, the marks of the current page and split.
#[derive(Default, Clone, Copy)]
pub(crate) struct CurMarks([crate::base::token::TokenList; 5]);

impl CurMarks {
    pub(crate) const NULL: CurMarks = CurMarks([crate::base::token::TokenList::NULL; 5]);
}

impl std::ops::Index<crate::base::codes::MarkCode> for CurMarks {
    type Output = crate::base::token::TokenList;
    #[inline]
    fn index(&self, m: crate::base::codes::MarkCode) -> &crate::base::token::TokenList {
        &self.0[m as usize]
    }
}

impl std::ops::IndexMut<crate::base::codes::MarkCode> for CurMarks {
    #[inline]
    fn index_mut(&mut self, m: crate::base::codes::MarkCode) -> &mut crate::base::token::TokenList {
        &mut self.0[m as usize]
    }
}

/// The marks of the `\marks` classes above 0, class 0 being `cur_mark`.
#[derive(Default, Clone)]
pub(crate) struct MarkClasses(std::collections::BTreeMap<crate::tables::eqtb::Register, CurMarks>);

impl MarkClasses {
    /// The marks of `class`, all null when it has none.
    pub(crate) fn get(&self, class: crate::tables::eqtb::Register) -> CurMarks {
        self.0.get(&class).copied().unwrap_or(CurMarks::NULL)
    }

    /// The marks of `class`, created null.
    pub(crate) fn entry(&mut self, class: crate::tables::eqtb::Register) -> &mut CurMarks {
        self.0.entry(class).or_insert(CurMarks::NULL)
    }

    /// Keeps the classes `keep` returns true for, in ascending order.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&mut CurMarks) -> bool) {
        self.0.retain(|_, marks| keep(marks));
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (crate::tables::eqtb::Register, &CurMarks)> {
        self.0.iter().map(|(&class, marks)| (class, marks))
    }
}
