//! The page builder: `build_page` and `fire_up`.

use crate::codes::{BoxCode, MarkCode, ValueLevel};
use crate::consts::{
    AWFUL_BAD, BOX_NODE_SIZE, EJECT_PENALTY, IGNORE_DEPTH, INF_BAD, INF_PENALTY, INS_NODE_SIZE,
    MAX_DIMEN, MAX_HALFWORD, PAGE_INS_NODE_SIZE, VMODE,
};
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::helpers::badness;
use crate::input::TokenType;
use crate::mem::{CONTRIB_HEAD, HOLD_HEAD, PAGE_HEAD, PAGE_INS_HEAD};
use crate::node::{AnyNode, InsNode, NodePtr, PageInsNode, SaLeaf, WhatsitVariant};
use crate::packaging::{MarksAction, PackSpec};
use crate::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::scan::fetch_box;
use crate::state::{EngineState, GroupCode, PageContents, ShipoutState};
use crate::token::TokenList;
use crate::units::{GlueOrder, MemPtr, Scaled};

// Indices into `page_so_far`, whose entries 2 to 5 hold the stretch of each order.
const PAGE_GOAL: usize = 0;
const PAGE_TOTAL: usize = 1;
const PAGE_SHRINK: usize = 6;
const PAGE_DEPTH: usize = 7;

const DEPLORABLE: i32 = 100_000;

pub fn initialize_pagebuilder_variables(st: &mut EngineState) {
    st.shipout.page_max_depth = Scaled::ZERO;
}

/// A record on the `page_ins_head` ring.
fn page_ins(p: MemPtr) -> NodePtr<PageInsNode> {
    NodePtr::at(p)
}

/// TeX's `contrib_tail`, the tail of the outermost vertical list.
fn set_contrib_tail(st: &mut EngineState, tail: MemPtr) {
    if st.input.nest.is_empty() {
        st.input.cur_list.tail = tail;
    } else {
        st.input.nest[0].tail = tail;
    }
}

fn freeze_page_specs(st: &mut EngineState, contents: PageContents) {
    let shipout = &mut st.shipout;
    shipout.page_contents = contents;
    shipout.page_so_far[PAGE_GOAL] = st.eqtb.dimen_par(DimenPar::Vsize);
    shipout.page_max_depth = st.eqtb.dimen_par(DimenPar::MaxDepth);
    shipout.page_so_far[PAGE_TOTAL..=PAGE_DEPTH].fill(Scaled::ZERO);
    shipout.least_page_cost = MAX_HALFWORD;
}

/// The box register of an insertion class, which `\insert` limits to 0..254.
fn ins_box(box_number: u16) -> Result<u8, Abort> {
    u8::try_from(box_number).or_confusion("insertion box")
}

fn ensure_vbox(st: &mut EngineState, n: u8) -> Result<(), Abort> {
    if !fetch_box(st, i32::from(n)).is_some_and(|b| st.mem[b].is_hlist()) {
        return Ok(());
    }
    crate::errors::box_error(
        st,
        n,
        "Insertions can only be added to a vbox",
        &[
            r"Tut tut: You're trying to \insert into a",
            r"\box register that now contains an \hbox.",
            "Proceed, and I'll discard its present contents.",
        ],
    )
}

/// Records the new mark `mark_ptr` as the first and bottom mark of class `class`.
fn update_marks(st: &mut EngineState, class: i32, mark_ptr: TokenList) -> Result<(), Abort> {
    if class != 0 {
        let SaLeaf::MarkClass(m) =
            st.mem
                .create_sa_element(&mut st.scanner, ValueLevel::Mark, class)?
        else {
            return Err(Abort::Confusion("marks"));
        };
        if st.mem[m].mark(MarkCode::First).is_null() {
            st.mem[m].set_mark(MarkCode::First, mark_ptr);
            st.mem.add_token_ref(mark_ptr);
        }
        let bot = st.mem[m].mark(MarkCode::Bot);
        if !bot.is_null() {
            st.mem.delete_token_ref(bot);
        }
        st.mem[m].set_mark(MarkCode::Bot, mark_ptr);
        st.mem.add_token_ref(mark_ptr);
        return Ok(());
    }
    let marks = &mut st.scanner.cur_mark;
    if marks[MarkCode::First].is_null() {
        marks[MarkCode::First] = mark_ptr;
        st.mem.add_token_ref(mark_ptr);
    }
    if !marks[MarkCode::Bot].is_null() {
        st.mem.delete_token_ref(marks[MarkCode::Bot]);
    }
    marks[MarkCode::Bot] = mark_ptr;
    st.mem.add_token_ref(mark_ptr);
    Ok(())
}

/// Moves insertion `ins` at `p` into its box when it made it onto the page.
/// Returns whether it has to wait for the next page.
fn place_insertion(st: &mut EngineState, ins: NodePtr<InsNode>, p: MemPtr) -> Result<bool, Abort> {
    let mut r = PAGE_INS_HEAD.link(&st.mem);
    while st.mem[page_ins(r)].box_number != st.mem[ins].box_number {
        r = r.link(&st.mem);
    }
    let rec = page_ins(r);
    if st.mem[rec].best_ins_ptr.is_null() {
        return Ok(true);
    }
    let mut s = st.mem[rec].last_ins_ptr;
    let list = st.mem[ins].ins_ptr;
    s.set_link(&mut st.mem, list);
    if st.mem[rec].best_ins_ptr != p {
        while !s.link(&st.mem).is_null() {
            s = s.link(&st.mem);
        }
        st.mem[rec].last_ins_ptr = s;
        return Ok(false);
    }

    let mut wait = false;
    let broken_ptr = st.mem[rec].broken_ptr;
    if st.mem[rec].is_split_up() && st.mem[rec].broken_ins == p && !broken_ptr.is_null() {
        while s.link(&st.mem) != broken_ptr {
            s = s.link(&st.mem);
        }
        s.set_link(&mut st.mem, MemPtr::NULL);
        st.eqtb
            .set_glue_par(GluePar::SplitTopSkip, st.mem[ins].split_top_ptr);
        let pruned = st
            .mem
            .prune_page_top(&mut st.scanner, &st.eqtb, broken_ptr, false)?;
        st.mem[ins].ins_ptr = pruned;
        if !pruned.is_null() {
            let tmp = crate::packaging::vpackage(st, pruned, PackSpec::NATURAL, MAX_DIMEN)?;
            let size = st.mem[tmp].height + st.mem[tmp].depth;
            st.mem[ins].height = size;
            st.mem.free_node(tmp.ptr(), BOX_NODE_SIZE);
            wait = true;
        }
    }
    st.mem[rec].best_ins_ptr = MemPtr::NULL;
    let n = ins_box(st.mem[rec].box_number)?;
    let b = fetch_box(st, i32::from(n)).expect("insertion box made in fire_up");
    let list = st.mem[b].list_ptr;
    st.mem.free_node(b.ptr(), BOX_NODE_SIZE);
    let packed = crate::packaging::vpackage(st, list, PackSpec::NATURAL, MAX_DIMEN)?;
    st.eqtb.set_box_reg(i32::from(n), packed.ptr());
    Ok(wait)
}

/// TeX's `fire_up`: puts the best page into `\box255` and runs the output routine.
fn fire_up(st: &mut EngineState, c: MemPtr) -> Result<(), Abort> {
    let best = st.shipout.best_page_break;
    let output_penalty = match best.as_idx().map(|b| st.mem.classify(b)) {
        Some(Ok(AnyNode::Penalty(pen))) => {
            let v = st.mem[pen].penalty;
            st.mem[pen].penalty = INF_PENALTY;
            v
        }
        _ => INF_PENALTY,
    };
    st.eqtb
        .geq_word_define(IntPar::OutputPenalty.loc(), output_penalty);

    if let Some(root) = st.scanner.sa_root.get(ValueLevel::Mark) {
        if st
            .mem
            .do_marks(&mut st.scanner, MarksAction::FireUpInit, 0, root)
        {
            st.scanner.sa_root.set(ValueLevel::Mark, None);
        }
    }
    if !st.scanner.cur_mark[MarkCode::Bot].is_null() {
        if !st.scanner.cur_mark[MarkCode::Top].is_null() {
            st.mem.delete_token_ref(st.scanner.cur_mark[MarkCode::Top]);
        }
        st.scanner.cur_mark[MarkCode::Top] = st.scanner.cur_mark[MarkCode::Bot];
        st.mem.add_token_ref(st.scanner.cur_mark[MarkCode::Top]);
        st.mem
            .delete_token_ref(st.scanner.cur_mark[MarkCode::First]);
        st.scanner.cur_mark[MarkCode::First] = TokenList::NULL;
    }

    if c == best {
        st.shipout.best_page_break = MemPtr::NULL;
    }
    if !st.eqtb.box_reg(255).is_null() {
        crate::errors::box_error(
            st,
            255,
            (crate::output::Esc("box"), "255 is not void"),
            &[
                r"You shouldn't use \box255 except in \output routines.",
                "Proceed, and I'll discard its present contents.",
            ],
        )?;
    }

    st.shipout.insert_penalties = 0;
    let save_split_top_skip = st.eqtb.glue_par(GluePar::SplitTopSkip);
    let process_inserts =
        st.eqtb.int_par(IntPar::HoldingInserts) <= 0 && !st.config.semantic_pagination_enabled;
    if process_inserts {
        let mut r = PAGE_INS_HEAD.link(&st.mem);
        while r != PAGE_INS_HEAD {
            let rec = page_ins(r);
            if !st.mem[rec].best_ins_ptr.is_null() {
                let n = ins_box(st.mem[rec].box_number)?;
                ensure_vbox(st, n)?;
                if st.eqtb.box_reg(i32::from(n)).is_null() {
                    let b = st.mem.new_null_box()?;
                    st.eqtb.set_box_reg(i32::from(n), b.ptr());
                }
                // The box's `list_ptr` word serves as the head link of its list.
                let mut p = st.eqtb.box_reg(i32::from(n)) + 5;
                while !p.link(&st.mem).is_null() {
                    p = p.link(&st.mem);
                }
                st.mem[rec].last_ins_ptr = p;
            }
            r = r.link(&st.mem);
        }
    }

    let mut hold_tail = HOLD_HEAD;
    hold_tail.set_link(&mut st.mem, MemPtr::NULL);
    let mut prev_p = PAGE_HEAD;
    let mut p = prev_p.link(&st.mem);
    while p != st.shipout.best_page_break {
        match st.mem.classify(p) {
            Ok(AnyNode::Ins(ins)) if process_inserts => {
                let wait = place_insertion(st, ins, p)?;
                let next = st.mem[ins].link;
                prev_p.set_link(&mut st.mem, next);
                st.mem[ins].link = MemPtr::NULL;
                if wait {
                    hold_tail.set_link(&mut st.mem, p);
                    hold_tail = p;
                    st.shipout.insert_penalties += 1;
                } else {
                    let split_top = st.mem[ins].split_top_ptr;
                    st.mem.delete_glue_ref(split_top);
                    st.mem.free_node(p, INS_NODE_SIZE);
                }
            }
            Ok(AnyNode::Mark(mark)) => {
                update_marks(st, st.mem[mark].mark_class, st.mem[mark].mark_ptr)?;
                prev_p = p;
            }
            _ => prev_p = p,
        }
        p = prev_p.link(&st.mem);
    }
    st.eqtb
        .set_glue_par(GluePar::SplitTopSkip, save_split_top_skip);

    if !p.is_null() {
        if CONTRIB_HEAD.link(&st.mem).is_null() {
            set_contrib_tail(st, st.shipout.page_tail);
        }
        let contrib = CONTRIB_HEAD.link(&st.mem);
        st.shipout.page_tail.set_link(&mut st.mem, contrib);
        CONTRIB_HEAD.set_link(&mut st.mem, p);
        prev_p.set_link(&mut st.mem, MemPtr::NULL);
    }

    let save_vbadness = st.eqtb.int_par(IntPar::Vbadness);
    st.eqtb.set_int_par(IntPar::Vbadness, INF_BAD);
    let save_vfuzz = st.eqtb.dimen_par(DimenPar::Vfuzz);
    st.eqtb.set_dimen_par(DimenPar::Vfuzz, MAX_DIMEN);
    let page = crate::packaging::vpackage(
        st,
        PAGE_HEAD.link(&st.mem),
        PackSpec::Exactly(st.shipout.best_size),
        st.shipout.page_max_depth,
    )?;
    st.eqtb.set_box_reg(255, page.ptr());
    st.eqtb.set_int_par(IntPar::Vbadness, save_vbadness);
    st.eqtb.set_dimen_par(DimenPar::Vfuzz, save_vfuzz);
    if let Some(spec) = st.shipout.last_glue {
        st.mem.delete_glue_ref(spec);
    }

    st.shipout.page_contents = PageContents::Empty;
    st.shipout.page_tail = PAGE_HEAD;
    PAGE_HEAD.set_link(&mut st.mem, MemPtr::NULL);
    st.shipout.last_glue = None;
    st.shipout.last_penalty = 0;
    st.shipout.last_kern = Scaled::ZERO;
    st.shipout.last_node_type = -1;
    st.shipout.page_so_far[PAGE_DEPTH] = Scaled::ZERO;
    st.shipout.page_max_depth = Scaled::ZERO;
    if hold_tail != HOLD_HEAD {
        let held = HOLD_HEAD.link(&st.mem);
        PAGE_HEAD.set_link(&mut st.mem, held);
        st.shipout.page_tail = hold_tail;
    }

    let mut r = PAGE_INS_HEAD.link(&st.mem);
    while r != PAGE_INS_HEAD {
        let next = r.link(&st.mem);
        st.mem.free_node(r, PAGE_INS_NODE_SIZE);
        r = next;
    }
    PAGE_INS_HEAD.set_link(&mut st.mem, PAGE_INS_HEAD);

    if let Some(root) = st.scanner.sa_root.get(ValueLevel::Mark) {
        if st
            .mem
            .do_marks(&mut st.scanner, MarksAction::FireUpDone, 0, root)
        {
            st.scanner.sa_root.set(ValueLevel::Mark, None);
        }
    }
    let marks = &mut st.scanner.cur_mark;
    if !marks[MarkCode::Top].is_null() && marks[MarkCode::First].is_null() {
        marks[MarkCode::First] = marks[MarkCode::Top];
        st.mem.add_token_ref(marks[MarkCode::Top]);
    }

    let output_routine = st.eqtb.tok_par(TokPar::OutputRoutine);
    if !output_routine.is_null() {
        if st.shipout.dead_cycles < st.eqtb.int_par(IntPar::MaxDeadCycles) {
            st.shipout.output_active = true;
            st.shipout.dead_cycles += 1;
            crate::mem::push_nest(st)?;
            st.input.cur_list.mode = -VMODE;
            st.input.cur_list.set_prev_depth(IGNORE_DEPTH);
            st.input.cur_list.mode_line = -st.mem.line;
            crate::input::begin_token_list(st, output_routine, TokenType::Output)?;
            st.save_stack
                .new_save_level(GroupCode::Output, st.mem.line)?;
            crate::main_control::normal_paragraph(st)?;
            crate::scan::scan_left_brace(st)?;
            return Ok(());
        }
        crate::errors::error(
            st,
            (
                "Output loop---",
                st.shipout.dead_cycles,
                " consecutive dead cycles",
            ),
            &[
                r"I've concluded that your \output is awry; it never does a",
                r"\shipout, so I'm shipping \box255 out myself. Next time",
                r"increase \maxdeadcycles if you want me to be more patient!",
            ],
        )?;
    }

    if !PAGE_HEAD.link(&st.mem).is_null() {
        let contrib = CONTRIB_HEAD.link(&st.mem);
        if contrib.is_null() {
            set_contrib_tail(st, st.shipout.page_tail);
        } else {
            st.shipout.page_tail.set_link(&mut st.mem, contrib);
        }
        let held = PAGE_HEAD.link(&st.mem);
        CONTRIB_HEAD.set_link(&mut st.mem, held);
        PAGE_HEAD.set_link(&mut st.mem, MemPtr::NULL);
        st.shipout.page_tail = PAGE_HEAD;
    }
    let discards = st.scanner.disc_ptr[BoxCode::LastBox as usize];
    st.mem.flush_node_list(discards)?;
    st.scanner.disc_ptr[BoxCode::LastBox as usize] = MemPtr::NULL;
    crate::shipout::ship_out(st, page)?;
    st.eqtb.set_box_reg(255, MemPtr::NULL);
    Ok(())
}

fn update_last_node_info(st: &mut EngineState, node: AnyNode, node_type: i32) {
    st.shipout.last_penalty = 0;
    st.shipout.last_kern = Scaled::ZERO;
    st.shipout.last_node_type = node_type + 1;
    st.shipout.last_glue = None;
    match node {
        AnyNode::Glue(glue) => {
            let spec = st.mem[glue].glue_ptr;
            st.shipout.last_glue = Some(spec);
            let refs = st.mem[spec].ref_count;
            st.mem[spec].ref_count = refs + 1;
        }
        AnyNode::Penalty(penalty) => st.shipout.last_penalty = st.mem[penalty].penalty,
        AnyNode::Kern(kern) => st.shipout.last_kern = st.mem[kern].width,
        _ => {}
    }
}

impl ShipoutState {
    fn page_badness(&self) -> i32 {
        let so_far = &self.page_so_far;
        if so_far[PAGE_TOTAL] < so_far[PAGE_GOAL] {
            if so_far[3] != Scaled::ZERO || so_far[4] != Scaled::ZERO || so_far[5] != Scaled::ZERO {
                0
            } else {
                badness(so_far[PAGE_GOAL] - so_far[PAGE_TOTAL], so_far[2])
            }
        } else if so_far[PAGE_TOTAL] - so_far[PAGE_GOAL] > so_far[PAGE_SHRINK] {
            AWFUL_BAD
        } else {
            badness(so_far[PAGE_TOTAL] - so_far[PAGE_GOAL], so_far[PAGE_SHRINK])
        }
    }
}

/// Scales `x` by the insertion magnification `count`, in thousandths.
fn scale_by_count(x: Scaled, count: i32) -> Scaled {
    if count == 1000 {
        x
    } else {
        crate::scaled_math::x_over_n(x, 1000).quotient * count
    }
}

/// Adds insertion `ins` at `p` to the current page, splitting it if needed.
fn append_insertion(st: &mut EngineState, ins: NodePtr<InsNode>, p: MemPtr) -> Result<(), Abort> {
    if st.shipout.page_contents == PageContents::Empty {
        freeze_page_specs(st, PageContents::InsertsOnly);
    }
    let n = ins_box(st.mem[ins].box_number)?;
    let mut r = PAGE_INS_HEAD;
    while u16::from(n) >= st.mem[page_ins(r.link(&st.mem))].box_number {
        r = r.link(&st.mem);
    }
    if st.mem[page_ins(r)].box_number != u16::from(n) {
        let q = st.mem.get_node(PAGE_INS_NODE_SIZE)?;
        let next = r.link(&st.mem);
        q.set_link(&mut st.mem, next);
        r.set_link(&mut st.mem, q);
        r = q;
        let rec = page_ins(r);
        st.mem[rec].box_number = u16::from(n);
        st.mem[rec].set_split_up(false);
        ensure_vbox(st, n)?;
        let height = match fetch_box(st, i32::from(n)) {
            Some(b) => st.mem[b].height + st.mem[b].depth,
            None => Scaled::ZERO,
        };
        st.mem[rec].height = height;
        st.mem[rec].best_ins_ptr = MemPtr::NULL;
        let skip = st.eqtb.skip(i32::from(n));
        let count = st.eqtb.count(i32::from(n));
        let h = scale_by_count(height, count);
        st.shipout.page_so_far[PAGE_GOAL] -= h + st.mem[skip].width;
        let order = st.mem[skip].stretch_order.index();
        st.shipout.page_so_far[2 + order] += st.mem[skip].stretch;
        st.shipout.page_so_far[PAGE_SHRINK] += st.mem[skip].shrink;
        if st.mem[skip].shrink_order != GlueOrder::Normal && st.mem[skip].shrink != Scaled::ZERO {
            crate::errors::error(
                st,
                (
                    "Infinite glue shrinkage inserted from ",
                    crate::output::Esc("skip"),
                    i32::from(n),
                ),
                &[
                    "The correction glue for page breaking with insertions",
                    "must have finite shrinkability. But you may proceed,",
                    "since the offensive shrinkability has been made finite.",
                ],
            )?;
        }
    }

    let rec = page_ins(r);
    if st.mem[rec].is_split_up() {
        st.shipout.insert_penalties = st
            .shipout
            .insert_penalties
            .wrapping_add(st.mem[ins].float_cost);
        return Ok(());
    }
    st.mem[rec].last_ins_ptr = p;
    let so_far = st.shipout.page_so_far;
    let delta = so_far[PAGE_GOAL] - so_far[PAGE_TOTAL] - so_far[PAGE_DEPTH] + so_far[PAGE_SHRINK];
    let count = st.eqtb.count(i32::from(n));
    let ins_height = st.mem[ins].height;
    let h = scale_by_count(ins_height, count);
    let max_height = st.eqtb.dimen(i32::from(n));
    if (h <= Scaled::ZERO || h <= delta) && ins_height + st.mem[rec].height <= max_height {
        st.shipout.page_so_far[PAGE_GOAL] -= h;
        let height = st.mem[rec].height + ins_height;
        st.mem[rec].height = height;
        return Ok(());
    }

    let mut w = if count <= 0 {
        MAX_DIMEN
    } else {
        let w = so_far[PAGE_GOAL] - so_far[PAGE_TOTAL] - so_far[PAGE_DEPTH];
        if count == 1000 {
            w
        } else {
            crate::scaled_math::x_over_n(w, count).quotient * 1000
        }
    };
    let room = max_height - st.mem[rec].height;
    if w > room {
        w = room;
    }
    let (q, mut best_height_plus_depth) =
        crate::packaging::vert_break(st, st.mem[ins].ins_ptr, w, st.mem[ins].depth)?;
    let height = st.mem[rec].height + best_height_plus_depth;
    st.mem[rec].height = height;
    if count != 1000 {
        best_height_plus_depth = scale_by_count(best_height_plus_depth, count);
    }
    st.shipout.page_so_far[PAGE_GOAL] -= best_height_plus_depth;
    st.mem[rec].set_split_up(true);
    st.mem[rec].broken_ptr = q;
    st.mem[rec].broken_ins = p;
    if q.is_null() {
        st.shipout.insert_penalties = st.shipout.insert_penalties.wrapping_add(EJECT_PENALTY);
    } else if let Ok(AnyNode::Penalty(pen)) = st.mem.classify(q) {
        st.shipout.insert_penalties = st
            .shipout
            .insert_penalties
            .wrapping_add(st.mem[pen].penalty);
    }
    Ok(())
}

/// What to do with the node at the head of the contribution list.
enum PageStep {
    /// Move it to the current page.
    Contribute,
    /// Account for a glue or kern, then move it to the current page.
    UpdateHeights,
    /// Discard it: nothing has been put on the page yet.
    Discard,
    /// A legal breakpoint with penalty `pi`.
    Break(i32),
}

pub(crate) fn build_page(st: &mut EngineState) -> Result<(), Abort> {
    if CONTRIB_HEAD.link(&st.mem).is_null() || st.shipout.output_active {
        return Ok(());
    }
    while let Some(idx) = CONTRIB_HEAD.link(&st.mem).as_idx() {
        let p = MemPtr::from_idx(idx);
        let node_type = i32::from(st.mem.type_code(idx));
        let node = st.mem.classify(idx).or_confusion("page")?;
        if let Some(spec) = st.shipout.last_glue {
            st.mem.delete_glue_ref(spec);
        }
        update_last_node_info(st, node, node_type);

        let below_box = st.shipout.page_contents < PageContents::BoxThere;
        let step = match node {
            AnyNode::Box(_) | AnyNode::Rule(_) => {
                let (height, depth) = match node {
                    AnyNode::Box(b) => (st.mem[b].height, st.mem[b].depth),
                    AnyNode::Rule(r) => (st.mem[r].height.to_scaled(), st.mem[r].depth.to_scaled()),
                    _ => unreachable!(),
                };
                if below_box {
                    if st.shipout.page_contents == PageContents::Empty {
                        freeze_page_specs(st, PageContents::BoxThere);
                    } else {
                        st.shipout.page_contents = PageContents::BoxThere;
                    }
                    let q = st.mem.new_skip_param(&st.eqtb, GluePar::TopSkip)?;
                    let spec = st.mem[q].glue_ptr;
                    let width = st.mem[spec].width;
                    let top = if st.eqtb.int_par(IntPar::XetexUpwards) > 0 {
                        depth
                    } else {
                        height
                    };
                    let width = if width > top {
                        width - top
                    } else {
                        Scaled::ZERO
                    };
                    st.mem[spec].width = width;
                    st.mem[q].link = p;
                    CONTRIB_HEAD.set_link(&mut st.mem, q.ptr());
                    continue;
                }
                st.shipout.page_so_far[PAGE_TOTAL] += st.shipout.page_so_far[PAGE_DEPTH] + height;
                st.shipout.page_so_far[PAGE_DEPTH] = depth;
                PageStep::Contribute
            }
            AnyNode::Whatsit(w) => {
                if let Some(WhatsitVariant::Pic(pic)) = w.variant(&st.mem) {
                    let pic = st.mem[pic];
                    st.shipout.page_so_far[PAGE_TOTAL] +=
                        st.shipout.page_so_far[PAGE_DEPTH] + pic.height;
                    st.shipout.page_so_far[PAGE_DEPTH] = pic.depth;
                }
                PageStep::Contribute
            }
            AnyNode::Glue(_) if below_box => PageStep::Discard,
            AnyNode::Glue(_) => {
                let tail = st.mem.classify(st.shipout.page_tail);
                if tail.is_ok_and(AnyNode::precedes_break) {
                    PageStep::Break(0)
                } else {
                    PageStep::UpdateHeights
                }
            }
            AnyNode::Kern(_) if below_box => PageStep::Discard,
            AnyNode::Kern(kern) => {
                let next = st.mem[kern].link;
                if next.is_null() {
                    return Ok(());
                }
                if matches!(st.mem.classify(next), Ok(AnyNode::Glue(_))) {
                    PageStep::Break(0)
                } else {
                    PageStep::UpdateHeights
                }
            }
            AnyNode::Penalty(_) if below_box => PageStep::Discard,
            AnyNode::Penalty(pen) => PageStep::Break(st.mem[pen].penalty),
            AnyNode::Mark(_) => PageStep::Contribute,
            AnyNode::Ins(ins) => {
                append_insertion(st, ins, p)?;
                PageStep::Contribute
            }
            _ => return Err(Abort::Confusion("page")),
        };

        let step = match step {
            PageStep::Break(pi) => {
                if pi < INF_PENALTY {
                    let b = st.shipout.page_badness();
                    let mut c = if b >= AWFUL_BAD {
                        b
                    } else if pi <= EJECT_PENALTY {
                        pi
                    } else if b < INF_BAD {
                        (b + pi).wrapping_add(st.shipout.insert_penalties)
                    } else {
                        DEPLORABLE
                    };
                    if st.shipout.insert_penalties >= 10_000 {
                        c = MAX_HALFWORD;
                    }
                    if c <= st.shipout.least_page_cost {
                        st.shipout.best_page_break = p;
                        st.shipout.best_size = st.shipout.page_so_far[PAGE_GOAL];
                        st.shipout.least_page_cost = c;
                        let mut r = PAGE_INS_HEAD.link(&st.mem);
                        while r != PAGE_INS_HEAD {
                            let rec = page_ins(r);
                            let last = st.mem[rec].last_ins_ptr;
                            st.mem[rec].best_ins_ptr = last;
                            r = r.link(&st.mem);
                        }
                    }
                    if c == AWFUL_BAD || pi <= EJECT_PENALTY {
                        fire_up(st, p)?;
                        if st.shipout.output_active {
                            return Ok(());
                        }
                        continue;
                    }
                }
                if matches!(node, AnyNode::Glue(_) | AnyNode::Kern(_)) {
                    PageStep::UpdateHeights
                } else {
                    PageStep::Contribute
                }
            }
            step => step,
        };

        match step {
            PageStep::UpdateHeights => {
                let width = match node {
                    AnyNode::Kern(kern) => st.mem[kern].width,
                    AnyNode::Glue(glue) => page_glue_width(st, glue)?,
                    _ => unreachable!("only glue and kern update the page heights"),
                };
                st.shipout.page_so_far[PAGE_TOTAL] += st.shipout.page_so_far[PAGE_DEPTH] + width;
                st.shipout.page_so_far[PAGE_DEPTH] = Scaled::ZERO;
                contribute(st, p);
            }
            PageStep::Contribute => contribute(st, p),
            PageStep::Discard => {
                let next = p.link(&st.mem);
                CONTRIB_HEAD.set_link(&mut st.mem, next);
                p.set_link(&mut st.mem, MemPtr::NULL);
                if st.eqtb.int_par(IntPar::SavingVdiscards) <= 0 {
                    st.mem.flush_node_list(p)?;
                } else {
                    let discards = &mut st.scanner.disc_ptr;
                    if discards[BoxCode::LastBox as usize].is_null() {
                        discards[BoxCode::LastBox as usize] = p;
                    } else {
                        discards[BoxCode::Copy as usize].set_link(&mut st.mem, p);
                    }
                    discards[BoxCode::Copy as usize] = p;
                }
            }
            PageStep::Break(_) => unreachable!("breakpoints are resolved above"),
        }
    }
    set_contrib_tail(st, CONTRIB_HEAD);
    Ok(())
}

/// Adds the stretch and shrink of page glue `glue` to the page totals and
/// returns its width, making infinite shrinkability finite.
fn page_glue_width(
    st: &mut EngineState,
    glue: NodePtr<crate::node::GlueNode>,
) -> Result<Scaled, Abort> {
    let spec = st.mem[glue].glue_ptr;
    let order = st.mem[spec].stretch_order.index();
    st.shipout.page_so_far[2 + order] += st.mem[spec].stretch;
    st.shipout.page_so_far[PAGE_SHRINK] += st.mem[spec].shrink;
    if st.mem[spec].shrink_order == GlueOrder::Normal || st.mem[spec].shrink == Scaled::ZERO {
        return Ok(st.mem[spec].width);
    }
    crate::errors::error(
        st,
        "Infinite glue shrinkage found on current page",
        &[
            "The page about to be output contains some infinitely",
            r"shrinkable glue, e.g., `\vss' or `\vskip 0pt minus 1fil'.",
            "Such glue doesn't belong there; but you can safely proceed,",
            "since the offensive shrinkability has been made finite.",
        ],
    )?;
    let finite = st.mem.new_spec(spec)?;
    st.mem[finite].shrink_order = GlueOrder::Normal.into();
    st.mem.delete_glue_ref(spec);
    st.mem[glue].glue_ptr = finite;
    Ok(st.mem[finite].width)
}

/// Moves `p` from the contribution list to the current page.
fn contribute(st: &mut EngineState, p: MemPtr) {
    let so_far = &mut st.shipout.page_so_far;
    if so_far[PAGE_DEPTH] > st.shipout.page_max_depth {
        so_far[PAGE_TOTAL] += so_far[PAGE_DEPTH] - st.shipout.page_max_depth;
        so_far[PAGE_DEPTH] = st.shipout.page_max_depth;
    }
    st.shipout.page_tail.set_link(&mut st.mem, p);
    st.shipout.page_tail = p;
    let next = p.link(&st.mem);
    CONTRIB_HEAD.set_link(&mut st.mem, next);
    p.set_link(&mut st.mem, MemPtr::NULL);
}
