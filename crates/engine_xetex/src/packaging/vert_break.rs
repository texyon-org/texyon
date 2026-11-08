//! Breaking vertical lists: `vert_break`, `vsplit`, `prune_page_top` and `do_marks`.

use crate::codes::{BoxCode, MarkCode, ValueLevel};
use crate::consts::{BOX_NODE_SIZE, EJECT_PENALTY, INF_BAD, INF_PENALTY, MAX_DIMEN, MAX_HALFWORD};
use crate::errors::OrConfusion;
use crate::errors::{Abort, IgnoredError};
use crate::mem::TEMP_HEAD;
use crate::node::{
    link_of, AnyNode, BoxNode, FixedNode, GlueNode, GlueSpecNode, KernNode, MarkNode, NodePtr,
    SaIndexNode, SaLeaf, SaMarkClassNode, WhatsitVariant,
};
use crate::params::{DimenPar, GluePar, IntPar};
use crate::state::{EngineState, EqtbState, MemState, ScannerState, ShipoutState};
use crate::token::TokenList;
use crate::units::{GlueOrder, MemPtr, Scaled};

use super::{print_vpack_diag, PackOutcome, PackSpec};

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

        let node = self.classify(cursor.p).or_confusion("vertbreak")?;
        cursor.breakable = node.precedes_break();
        match node {
            AnyNode::Box(b) => {
                scanner.active_width[1] += cursor.prev_dp + self[b].height;
                cursor.prev_dp = self[b].depth;
                Ok(VertBreakNext::NotFound)
            }
            AnyNode::Rule(rule) => {
                scanner.active_width[1] += cursor.prev_dp + self[rule].height.to_scaled();
                cursor.prev_dp = self[rule].depth.to_scaled();
                Ok(VertBreakNext::NotFound)
            }
            AnyNode::Whatsit(w) => {
                if let Some(WhatsitVariant::Pic(pic)) = w.variant(self) {
                    scanner.active_width[1] += cursor.prev_dp + self[pic].height;
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
                if !next.is_null() && matches!(self.classify(next), Ok(AnyNode::Glue(_))) {
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
        scanner: &mut ScannerState,
        cursor: &mut VertBreakCursor,
        h: Scaled,
        d: Scaled,
    ) -> Result<VertBreakStep, Abort> {
        loop {
            if let Some(glue) = cursor.finish_glue_after_shrinkage.take() {
                let spec = self[glue].glue_ptr;
                self.finish_vert_break_updated_height(scanner, cursor, self[spec].width, d);
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
                    self.finish_vert_break_updated_height(scanner, cursor, self[kern].width, d);
                }
                Some(VertBreakUpdateSource::Glue(glue)) => {
                    let spec = self[glue].glue_ptr;
                    scanner.active_width[2 + self[spec].stretch_order.index()] +=
                        self[spec].stretch;
                    scanner.active_width[6] += self[spec].shrink;
                    if self[spec].shrink_order != GlueOrder::Normal
                        && self[spec].shrink != Scaled::ZERO
                    {
                        cursor.finish_glue_after_shrinkage = Some(glue);
                        return Ok(VertBreakStep::InfiniteShrinkage { glue, spec });
                    }
                    self.finish_vert_break_updated_height(scanner, cursor, self[spec].width, d);
                }
                None => self.advance_vert_break(scanner, cursor, d),
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
        crate::output::print_ignored_err(st, MESSAGE);
        return Ok(());
    }
    crate::errors::error(
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
        spec: NodePtr<GlueSpecNode>,
    ) -> Result<(), Abort> {
        let spec = self.finite_shrink(spec)?;
        self[glue].glue_ptr = spec;
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
    /// The box register being split.
    n: i32,
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
        spec: NodePtr<GlueSpecNode>,
    },
}

impl MemState {
    /// The contents of box register `n`, leaving `n` in `cur_val`.
    fn reset_vsplit_marks(&mut self, scanner: &mut ScannerState) -> Result<(), Abort> {
        self.flush_node_list(scanner.disc_ptr[BoxCode::Vsplit as usize])?;
        scanner.disc_ptr[BoxCode::Vsplit as usize] = MemPtr::NULL;

        if let Some(root) = scanner.sa_root.get(ValueLevel::Mark) {
            if self.do_marks(scanner, MarksAction::VsplitInit, 0, root) {
                scanner.sa_root.set(ValueLevel::Mark, None);
            }
        }

        if !scanner.cur_mark[MarkCode::SplitFirst].is_null() {
            self.delete_token_ref(scanner.cur_mark[MarkCode::SplitFirst]);
            scanner.cur_mark[MarkCode::SplitFirst] = TokenList::NULL;
            self.delete_token_ref(scanner.cur_mark[MarkCode::SplitBot]);
            scanner.cur_mark[MarkCode::SplitBot] = TokenList::NULL;
        }

        Ok(())
    }
}

fn start_vsplit(st: &mut EngineState, n: i32, h: Scaled) -> Result<VsplitStart, Abort> {
    let v = crate::scan::fetch_box(st, n);
    st.mem.reset_vsplit_marks(&mut st.scanner)?;

    let v = match v {
        None => {
            return Ok(VsplitStart::Done(VsplitOutcome {
                result: None,
                packages: [None, None],
            }))
        }
        Some(v) if st.mem[v].is_vlist() => v,
        Some(_) => return Ok(VsplitStart::NeedsVboxDiagnostic),
    };

    Ok(VsplitStart::Ready(VsplitState {
        n,
        h,
        v,
        split_max_d: st.eqtb.dimen_par(DimenPar::SplitMaxDepth),
        vert_break: st.scanner.start_vert_break(st.mem[v].list_ptr),
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
            let SaLeaf::MarkClass(m) =
                self.create_sa_element(scanner, ValueLevel::Mark, mark_class)?
            else {
                return Err(Abort::Confusion("marks"));
            };
            if self[m].mark(MarkCode::SplitFirst).is_null() {
                self[m].set_mark(MarkCode::SplitFirst, mark_ptr);
                self.add_token_ref(mark_ptr);
            } else {
                self.delete_token_ref(self[m].mark(MarkCode::SplitBot));
            }
            self[m].set_mark(MarkCode::SplitBot, mark_ptr);
            self.add_token_ref(mark_ptr);
        } else if scanner.cur_mark[MarkCode::SplitFirst].is_null() {
            scanner.cur_mark[MarkCode::SplitFirst] = mark_ptr;
            scanner.cur_mark[MarkCode::SplitBot] = mark_ptr;
            mark_ptr.set_ref_count(self, mark_ptr.ref_count(self) + 2);
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
        let mut p = self[state.v].list_ptr;
        if p == q {
            self[state.v].list_ptr = MemPtr::NULL;
        } else {
            loop {
                if let AnyNode::Mark(mark) = self.classify(p).or_confusion("vsplit")? {
                    self.collect_split_mark(scanner, mark)?;
                }
                if p.link(self) == q {
                    p.set_link(self, MemPtr::NULL);
                    break;
                }
                p = p.link(self);
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
        let mut q = self.prune_page_top(scanner, eqtb, q, save_discards)?;

        self.free_node(state.v.ptr(), BOX_NODE_SIZE);
        if !q.is_null() {
            let packed = self.vpackage(eqtb, scanner, shipout, q, PackSpec::NATURAL, MAX_DIMEN)?;
            q = packed.r_box.ptr();
            push_slot(&mut packages, packed);
        }
        self.change_box(scanner, eqtb, state.n, q)?;

        let packed = self.vpackage(
            eqtb,
            scanner,
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
    crate::errors::error(
        st,
        (
            "",
            crate::output::Esc("vsplit"),
            " needs a ",
            crate::output::Esc("vbox"),
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
            print_vpack_diag(st, package.r_box, diag);
        }
    }
}

/// `\vsplit n to h`: the box split off the top of box register `n`.
pub(crate) fn vsplit(
    st: &mut EngineState,
    n: i32,
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
        prev_p: MemPtr,
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
        let upwards = eqtb.int_par(IntPar::XetexUpwards) > 0;

        while !p.is_null() {
            let node = self.classify(p).or_confusion("pruning")?;
            let top = match node {
                AnyNode::Box(b) if upwards => self[b].depth,
                AnyNode::Box(b) => self[b].height,
                AnyNode::Rule(rule) if upwards => self[rule].depth.to_scaled(),
                AnyNode::Rule(rule) => self[rule].height.to_scaled(),
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
            self.insert_split_top_skip_before(eqtb, prev_p, p, top)?;
            p = MemPtr::NULL;
        }

        Ok(TEMP_HEAD.link(self))
    }

    /// Apply mark action `a` to the mark-class tree below index node `q` at
    /// level `l`. Returns whether `q` was freed.
    pub fn do_marks(
        &mut self,
        scanner: &mut ScannerState,
        a: MarksAction,
        l: i16,
        q: NodePtr<SaIndexNode>,
    ) -> bool {
        // XeTeX widened index nodes to 64 children but its do_marks still visits 16.
        for i in 0..16 {
            let freed = if l < 3 {
                let child = self[q].child_index(i);
                scanner.cur_ptr = link_of(child);
                child.is_some_and(|c| self.do_marks(scanner, a, l + 1, c))
            } else {
                let child = self[q].child_leaf(i, ValueLevel::Mark);
                scanner.cur_ptr = link_of(child.map(SaLeaf::element));
                match child {
                    Some(SaLeaf::MarkClass(m)) => self.do_mark_class(scanner, a, m),
                    _ => false,
                }
            };
            if freed {
                let n = &mut self[q];
                n.clear_child(i);
                n.used -= 1;
            }
        }
        if self[q].used == 0 {
            self.free_node(q.ptr(), SaIndexNode::SIZE);
            return true;
        }
        false
    }

    /// Apply mark action `a` to mark class `node`. Returns whether it was freed.
    fn do_mark_class(
        &mut self,
        scanner: &mut ScannerState,
        a: MarksAction,
        node: NodePtr<SaMarkClassNode>,
    ) -> bool {
        match a {
            MarksAction::VsplitInit => {
                if !self[node].mark(MarkCode::SplitFirst).is_null() {
                    self.delete_token_ref(self[node].mark(MarkCode::SplitFirst));
                    self[node].set_mark(MarkCode::SplitFirst, TokenList::NULL);
                    self.delete_token_ref(self[node].mark(MarkCode::SplitBot));
                    self[node].set_mark(MarkCode::SplitBot, TokenList::NULL);
                }
            }
            MarksAction::FireUpInit => {
                if !self[node].mark(MarkCode::Bot).is_null() {
                    if !self[node].mark(MarkCode::Top).is_null() {
                        self.delete_token_ref(self[node].mark(MarkCode::Top));
                    }
                    self.delete_token_ref(self[node].mark(MarkCode::First));
                    self[node].set_mark(MarkCode::First, TokenList::NULL);
                    let bot = self[node].mark(MarkCode::Bot);
                    if bot.link(self).is_null() {
                        self.delete_token_ref(bot);
                        self[node].set_mark(MarkCode::Bot, TokenList::NULL);
                    } else {
                        self.add_token_ref(bot);
                    }
                    let bot = self[node].mark(MarkCode::Bot);
                    self[node].set_mark(MarkCode::Top, bot);
                }
            }
            MarksAction::FireUpDone => {
                let top = self[node].mark(MarkCode::Top);
                if !top.is_null() && self[node].mark(MarkCode::First).is_null() {
                    self[node].set_mark(MarkCode::First, top);
                    self.add_token_ref(top);
                }
            }
            MarksAction::DestroyMarks => {
                for mark in MarkCode::ALL {
                    scanner.cur_ptr = self[node].mark(mark).ptr();
                    if !scanner.cur_ptr.is_null() {
                        self.delete_token_ref(TokenList::from_ptr(scanner.cur_ptr));
                        self[node].set_mark(mark, TokenList::NULL);
                    }
                }
            }
        }

        if self[node].mark(MarkCode::Bot).is_null() && self[node].mark(MarkCode::SplitBot).is_null()
        {
            self.free_node(node.ptr(), SaMarkClassNode::SIZE);
            return true;
        }
        false
    }
}
