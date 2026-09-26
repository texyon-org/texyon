//! Breaking vertical lists: `vert_break`, `vsplit`, `prune_page_top` and `do_marks`.

use crate::codes::{BoxCode, MarkCode, ValueLevel};
use crate::consts::{
    BOX_NODE_SIZE, EJECT_PENALTY, INDEX_NODE_SIZE, INF_BAD, INF_PENALTY, MARK_CLASS_NODE_SIZE,
    MAX_DIMEN, MAX_HALFWORD, NORMAL,
};
use crate::errors::Abort;
use crate::mem::TEMP_HEAD;
use crate::node::{
    AnyBox, AnyNode, GlueNode, GlueSpecNode, KernNode, MarkNode, NodePtr, SaNode, VListNode,
    WhatsitSubtype,
};
use crate::params::{DimenPar, GluePar, IntPar};
use crate::state::{EngineState, EqtbState, MemState, ScannerState, ShipoutState};
use crate::units::{MemIdx, MemPtr, Scaled};

use super::{print_vpack_diag, PackOutcome, ADDITIONAL, EXACTLY};

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
}

enum VertBreakStep {
    Done(MemPtr),
    InfiniteShrinkage {
        glue: NodePtr<GlueNode>,
        spec: NodePtr<GlueSpecNode>,
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

impl ScannerState {
    fn start_vert_break(&mut self, p: MemPtr) -> VertBreakCursor {
        self.active_width[1..].fill(Scaled::ZERO);

        VertBreakCursor {
            p,
            prev_p: p,
            breakable: false,
            prev_breakable: false,
            least_cost: MAX_HALFWORD,
            best_place: MemPtr::NULL,
            prev_dp: Scaled::ZERO,
            finish_glue_after_shrinkage: None,
        }
    }
}

impl MemState {
    fn advance_vert_break(
        &self,
        scanner: &mut ScannerState,
        cursor: &mut VertBreakCursor,
        d: Scaled,
    ) {
        if cursor.prev_dp > d {
            scanner.active_width[1] += cursor.prev_dp - d;
            cursor.prev_dp = d;
        }
        cursor.prev_p = cursor.p;
        cursor.prev_breakable = cursor.breakable;
        cursor.p = cursor.prev_p.link(self);
    }

    fn finish_vert_break_updated_height(
        &self,
        scanner: &mut ScannerState,
        cursor: &mut VertBreakCursor,
        width: Scaled,
        d: Scaled,
    ) {
        scanner.active_width[1] += cursor.prev_dp + width;
        cursor.prev_dp = Scaled::ZERO;
        self.advance_vert_break(scanner, cursor, d);
    }

    fn classify_vert_break_next(
        &self,
        scanner: &mut ScannerState,
        cursor: &mut VertBreakCursor,
    ) -> Result<VertBreakNext, Abort> {
        if cursor.p.is_null() {
            return Ok(VertBreakNext::CheckPi {
                pi: EJECT_PENALTY,
                update_after: None,
            });
        }

        let node = self
            .classify(MemIdx::new(cursor.p))
            .map_err(|_| Abort::Confusion("vertbreak"))?;
        cursor.breakable = node.precedes_break();
        match node {
            AnyNode::HList(b) => {
                scanner.active_width[1] += cursor.prev_dp + b.height(self);
                cursor.prev_dp = b.depth(self);
                Ok(VertBreakNext::NotFound)
            }
            AnyNode::VList(b) => {
                scanner.active_width[1] += cursor.prev_dp + b.height(self);
                cursor.prev_dp = b.depth(self);
                Ok(VertBreakNext::NotFound)
            }
            AnyNode::Rule(rule) => {
                scanner.active_width[1] += cursor.prev_dp + rule.height(self).to_scaled();
                cursor.prev_dp = rule.depth(self).to_scaled();
                Ok(VertBreakNext::NotFound)
            }
            AnyNode::Whatsit(w) => {
                if matches!(
                    w.whatsit_subtype(self),
                    Some(WhatsitSubtype::Pic | WhatsitSubtype::Pdf)
                ) {
                    scanner.active_width[1] += cursor.prev_dp + w.height(self);
                    cursor.prev_dp = w.depth(self);
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
                let next = kern.link(self);
                if !next.is_null()
                    && matches!(self.classify(MemIdx::new(next)), Ok(AnyNode::Glue(_)))
                {
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
                pi: penalty.penalty(self),
                update_after: None,
            }),
            AnyNode::Ins(_) | AnyNode::Mark(_) => Ok(VertBreakNext::NotFound),
            _ => Err(Abort::Confusion("vertbreak")),
        }
    }

    fn vert_break_resume(
        &mut self,
        scanner: &mut ScannerState,
        cursor: &mut VertBreakCursor,
        h: Scaled,
        d: Scaled,
    ) -> Result<VertBreakStep, Abort> {
        loop {
            if let Some(glue) = cursor.finish_glue_after_shrinkage.take() {
                let spec = glue.glue_ptr(self);
                self.finish_vert_break_updated_height(scanner, cursor, spec.width(self), d);
                continue;
            }

            let update_source = match self.classify_vert_break_next(scanner, cursor)? {
                VertBreakNext::NotFound => None,
                VertBreakNext::UpdateHeights(source) => Some(source),
                VertBreakNext::CheckPi { pi, update_after } => {
                    if pi < INF_PENALTY {
                        let aw = &scanner.active_width;
                        let b = if aw[1] < h {
                            if aw[3] != Scaled::ZERO
                                || aw[4] != Scaled::ZERO
                                || aw[5] != Scaled::ZERO
                            {
                                0
                            } else {
                                crate::helpers::badness(h - aw[1], aw[2])
                            }
                        } else if aw[1] - h > aw[6] {
                            MAX_HALFWORD
                        } else {
                            crate::helpers::badness(aw[1] - h, aw[6])
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
                            scanner.best_height_plus_depth =
                                scanner.active_width[1] + cursor.prev_dp;
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
                    self.finish_vert_break_updated_height(scanner, cursor, kern.width(self), d);
                }
                Some(VertBreakUpdateSource::Glue(glue)) => {
                    let spec = glue.glue_ptr(self);
                    scanner.active_width[2 + spec.stretch_order(self) as usize] +=
                        spec.stretch(self);
                    scanner.active_width[6] += spec.shrink(self);
                    if spec.shrink_order(self) != NORMAL && spec.shrink(self) != Scaled::ZERO {
                        cursor.finish_glue_after_shrinkage = Some(glue);
                        return Ok(VertBreakStep::InfiniteShrinkage { glue, spec });
                    }
                    self.finish_vert_break_updated_height(scanner, cursor, spec.width(self), d);
                }
                None => self.advance_vert_break(scanner, cursor, d),
            }
        }
    }
}

fn print_vert_break_infinite_shrinkage_error(st: &mut EngineState) -> Result<(), Abort> {
    crate::output::print_err(st, "Infinite glue shrinkage found in box being split");
    st.errors.help(&[
        r"The box you are \vsplitting contains some infinitely",
        r"shrinkable glue, e.g., `\vss' or `\vskip 0pt minus 1fil'.",
        "Such glue doesn't belong there; but you can safely proceed,",
        "since the offensive shrinkability has been made finite.",
    ]);
    crate::errors::error(st)
}

impl MemState {
    fn repair_vert_break_infinite_shrinkage(
        &mut self,
        glue: NodePtr<GlueNode>,
        spec: NodePtr<GlueSpecNode>,
    ) -> Result<(), Abort> {
        let spec = self.finite_shrink(spec)?;
        glue.set_glue_ptr(self, spec);
        Ok(())
    }
}

/// The best place to break the vlist `p` for a height `h` with maximum depth `d`.
pub(crate) fn vert_break(
    st: &mut EngineState,
    p: MemPtr,
    h: Scaled,
    d: Scaled,
) -> Result<MemPtr, Abort> {
    let mut cursor = st.scanner.start_vert_break(p);
    loop {
        match st
            .mem
            .vert_break_resume(&mut st.scanner, &mut cursor, h, d)?
        {
            VertBreakStep::Done(best_place) => return Ok(best_place),
            VertBreakStep::InfiniteShrinkage { glue, spec } => {
                print_vert_break_infinite_shrinkage_error(st)?;
                st.mem.repair_vert_break_infinite_shrinkage(glue, spec)?;
            }
        }
    }
}

struct VsplitState {
    h: Scaled,
    v: NodePtr<VListNode>,
    split_max_d: Scaled,
    vert_break: VertBreakCursor,
}

#[derive(Copy, Clone)]
struct VsplitOutcome {
    result: Option<AnyBox>,
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
        spec: NodePtr<GlueSpecNode>,
    },
}

impl MemState {
    /// The contents of box register `n`, leaving `n` in `cur_val`.
    fn reset_vsplit_marks(&mut self, scanner: &mut ScannerState) -> Result<(), Abort> {
        self.flush_node_list(scanner.disc_ptr[BoxCode::Vsplit as usize])?;
        scanner.disc_ptr[BoxCode::Vsplit as usize] = MemPtr::NULL;

        if let Some(root) = scanner.sa_root[ValueLevel::Mark].as_idx() {
            if self.do_marks(scanner, MarksAction::VsplitInit, 0, root) {
                scanner.sa_root[ValueLevel::Mark] = MemPtr::NULL;
            }
        }

        if !scanner.cur_mark[MarkCode::SplitFirst].is_null() {
            self.delete_token_ref(scanner.cur_mark[MarkCode::SplitFirst]);
            scanner.cur_mark[MarkCode::SplitFirst] = MemPtr::NULL;
            self.delete_token_ref(scanner.cur_mark[MarkCode::SplitBot]);
            scanner.cur_mark[MarkCode::SplitBot] = MemPtr::NULL;
        }

        Ok(())
    }
}

fn start_vsplit(st: &mut EngineState, n: i32, h: Scaled) -> Result<VsplitStart, Abort> {
    st.scanner.cur_val = n;
    let v = crate::scan::fetch_box(st, n);
    st.mem.reset_vsplit_marks(&mut st.scanner)?;

    let v = match v {
        None => {
            return Ok(VsplitStart::Done(VsplitOutcome {
                result: None,
                packages: [None, None],
            }))
        }
        Some(AnyBox::VList(v)) => v,
        Some(AnyBox::HList(_)) => return Ok(VsplitStart::NeedsVboxDiagnostic),
    };

    Ok(VsplitStart::Ready(VsplitState {
        h,
        v,
        split_max_d: st.eqtb.dimen_par(DimenPar::SplitMaxDepth),
        vert_break: st.scanner.start_vert_break(v.list_ptr(&st.mem)),
    }))
}

impl MemState {
    fn collect_split_mark(
        &mut self,
        scanner: &mut ScannerState,
        mark: NodePtr<MarkNode>,
    ) -> Result<(), Abort> {
        let mark_class = mark.mark_class(self);
        let mark_ptr = mark.mark_ptr(self);
        if mark_class != 0 {
            let element = self.create_sa_element(scanner, ValueLevel::Mark, mark_class)?;
            if element.mark(self, MarkCode::SplitFirst).is_null() {
                element.set_mark(self, MarkCode::SplitFirst, mark_ptr);
                self.add_token_ref(mark_ptr);
            } else {
                self.delete_token_ref(element.mark(self, MarkCode::SplitBot));
            }
            element.set_mark(self, MarkCode::SplitBot, mark_ptr);
            self.add_token_ref(mark_ptr);
        } else if scanner.cur_mark[MarkCode::SplitFirst].is_null() {
            scanner.cur_mark[MarkCode::SplitFirst] = mark_ptr;
            scanner.cur_mark[MarkCode::SplitBot] = mark_ptr;
            mark_ptr.set_info(self, mark_ptr.info(self) + 2);
        } else {
            self.delete_token_ref(scanner.cur_mark[MarkCode::SplitBot]);
            scanner.cur_mark[MarkCode::SplitBot] = mark_ptr;
            self.add_token_ref(mark_ptr);
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
        let mut p = state.v.list_ptr(self);
        if p == q {
            state.v.set_list_ptr(self, MemPtr::NULL);
        } else {
            loop {
                if let AnyNode::Mark(mark) = self
                    .classify(MemIdx::new(p))
                    .map_err(|_| Abort::Confusion("vsplit"))?
                {
                    self.collect_split_mark(scanner, mark)?;
                }
                if p.link(self) == q {
                    p.set_link(self, MemPtr::NULL);
                    break;
                }
                p = p.link(self);
            }
        }

        Ok(state.v.list_ptr(self))
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
        let mut q = self.prune_page_top(scanner, eqtb, q, save_discards)?;

        self.free_node(state.v.ptr(), BOX_NODE_SIZE);
        if !q.is_null() {
            let packed = self.vpackage(
                eqtb,
                scanner,
                shipout,
                q,
                Scaled::ZERO,
                ADDITIONAL,
                MAX_DIMEN,
            )?;
            q = packed.r_box.ptr();
            push_slot(&mut packages, packed);
        }
        let n = scanner.cur_val;
        self.change_box(scanner, eqtb, n, q);

        let packed = self.vpackage(
            eqtb,
            scanner,
            shipout,
            p,
            state.h,
            EXACTLY,
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
        match self.vert_break_resume(scanner, &mut state.vert_break, state.h, state.split_max_d)? {
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
    crate::output::error_here_with_diagnostic(st, "");
    crate::output::print_esc(st, "vsplit");
    crate::output::print(st, " needs a ");
    crate::output::print_esc(st, "vbox");
    crate::output::capture_to_diagnostic(st, None);
    st.errors.help(&[
        r"The box you are trying to split is an \hbox.",
        "I can't split such a box, so I'll leave it alone.",
    ]);
    crate::errors::error(st)
}

fn print_vsplit_packages(st: &mut EngineState, outcome: VsplitOutcome) {
    for package in outcome.packages.into_iter().flatten() {
        for diag in package.diags.into_iter().flatten() {
            print_vpack_diag(st, package.r_box, diag);
        }
    }
}

/// `\vsplit n to h`: the box split off the top of box register `n`.
pub(crate) fn vsplit(st: &mut EngineState, n: i32, h: Scaled) -> Result<Option<AnyBox>, Abort> {
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
    /// Insert `\splittopskip` glue between `prev_p` and the box or rule `p` of `height`.
    fn insert_split_top_skip_before(
        &mut self,
        eqtb: &EqtbState,
        prev_p: MemPtr,
        p: MemPtr,
        height: Scaled,
    ) -> Result<(), Abort> {
        let q = self.new_skip_param(eqtb, GluePar::SplitTopSkip)?;
        prev_p.set_link(self, q.ptr());
        q.set_link(self, p);
        let spec = q.glue_ptr(self);
        let avail_ht = spec.width(self);
        if avail_ht > height {
            spec.set_width(self, avail_ht - height);
        } else {
            spec.set_width(self, Scaled::ZERO);
        }
        Ok(())
    }

    /// Unlink the discardable `node` after `prev_p`, saving it after `r` (or
    /// flushing it), and return its successor.
    fn prune_discard_node(
        &mut self,
        scanner: &mut ScannerState,
        prev_p: MemPtr,
        r: &mut MemPtr,
        node: MemPtr,
        save_discards: bool,
    ) -> Result<MemPtr, Abort> {
        let q = node.link(self);
        node.set_link(self, MemPtr::NULL);
        prev_p.set_link(self, q);
        if save_discards {
            if scanner.disc_ptr[BoxCode::Vsplit as usize].is_null() {
                scanner.disc_ptr[BoxCode::Vsplit as usize] = node;
            } else {
                r.set_link(self, node);
            }
            *r = node;
        } else {
            self.flush_node_list(node)?;
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
        let mut r = MemPtr::NULL;
        TEMP_HEAD.set_link(self, p);

        while !p.is_null() {
            let node = self
                .classify(MemIdx::new(p))
                .map_err(|_| Abort::Confusion("pruning"))?;
            let height = match node {
                AnyNode::HList(b) => b.height(self),
                AnyNode::VList(b) => b.height(self),
                AnyNode::Rule(rule) => rule.height(self).to_scaled(),
                AnyNode::Whatsit(_) | AnyNode::Mark(_) | AnyNode::Ins(_) => {
                    prev_p = p;
                    p = prev_p.link(self);
                    continue;
                }
                AnyNode::Glue(_) | AnyNode::Kern(_) | AnyNode::Penalty(_) => {
                    p = self.prune_discard_node(scanner, prev_p, &mut r, p, s)?;
                    continue;
                }
                _ => return Err(Abort::Confusion("pruning")),
            };
            self.insert_split_top_skip_before(eqtb, prev_p, p, height)?;
            p = MemPtr::NULL;
        }

        Ok(TEMP_HEAD.link(self))
    }

    /// Apply mark action `a` to the sparse-array mark tree `q` at level `l`.
    /// Returns whether the record at `q` was freed.
    pub fn do_marks(
        &mut self,
        scanner: &mut ScannerState,
        a: MarksAction,
        l: i16,
        q: MemIdx,
    ) -> bool {
        let node = NodePtr::<SaNode>::at(q);

        if l < 4 {
            for i in 0..=15 {
                scanner.cur_ptr = node.child(self, i);
                if let Some(cur) = scanner.cur_ptr.as_idx() {
                    if self.do_marks(scanner, a, l + 1, cur) {
                        node.set_child(self, i, MemPtr::NULL);
                        node.set_ref_count(self, node.ref_count(self) - 1);
                    }
                }
            }

            if node.ref_count(self) == 0 {
                self.free_node(MemPtr::from_idx(q), INDEX_NODE_SIZE);
                return true;
            }
            return false;
        }

        match a {
            MarksAction::VsplitInit => {
                if !node.mark(self, MarkCode::SplitFirst).is_null() {
                    self.delete_token_ref(node.mark(self, MarkCode::SplitFirst));
                    node.set_mark(self, MarkCode::SplitFirst, MemPtr::NULL);
                    self.delete_token_ref(node.mark(self, MarkCode::SplitBot));
                    node.set_mark(self, MarkCode::SplitBot, MemPtr::NULL);
                }
            }
            MarksAction::FireUpInit => {
                if !node.mark(self, MarkCode::Bot).is_null() {
                    if !node.mark(self, MarkCode::Top).is_null() {
                        self.delete_token_ref(node.mark(self, MarkCode::Top));
                    }
                    self.delete_token_ref(node.mark(self, MarkCode::First));
                    node.set_mark(self, MarkCode::First, MemPtr::NULL);
                    let bot = node.mark(self, MarkCode::Bot);
                    if bot.link(self).is_null() {
                        self.delete_token_ref(bot);
                        node.set_mark(self, MarkCode::Bot, MemPtr::NULL);
                    } else {
                        self.add_token_ref(bot);
                    }
                    node.set_mark(self, MarkCode::Top, node.mark(self, MarkCode::Bot));
                }
            }
            MarksAction::FireUpDone => {
                let top = node.mark(self, MarkCode::Top);
                if !top.is_null() && node.mark(self, MarkCode::First).is_null() {
                    node.set_mark(self, MarkCode::First, top);
                    self.add_token_ref(top);
                }
            }
            MarksAction::DestroyMarks => {
                for mark in MarkCode::ALL {
                    scanner.cur_ptr = node.mark(self, mark);
                    if !scanner.cur_ptr.is_null() {
                        self.delete_token_ref(scanner.cur_ptr);
                        node.set_mark(self, mark, MemPtr::NULL);
                    }
                }
            }
        }

        if node.mark(self, MarkCode::Bot).is_null() && node.mark(self, MarkCode::SplitBot).is_null()
        {
            self.free_node(MemPtr::from_idx(q), MARK_CLASS_NODE_SIZE);
            return true;
        }
        false
    }
}
