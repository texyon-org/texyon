//! The page builder: `build_page` and `fire_up`.

use crate::consts::{
    AWFUL_BAD, BOT_MARK_CODE, BOX_NODE_SIZE, DIMEN_PAR_MAX_DEPTH, DIMEN_PAR_VFUZZ, DIMEN_PAR_VSIZE,
    EJECT_PENALTY, FIRST_MARK_CODE, GLUE_PAR_SPLIT_TOP_SKIP, GLUE_PAR_TOP_SKIP, IGNORE_DEPTH,
    INF_BAD, INF_PENALTY, INS_NODE_SIZE, INT_BASE, INT_PAR_HOLDING_INSERTS,
    INT_PAR_MAX_DEAD_CYCLES, INT_PAR_OUTPUT_PENALTY, INT_PAR_SAVING_VDISCARDS, INT_PAR_VBADNESS,
    LOCAL_OUTPUT_ROUTINE, MAX_HALFWORD, MEM_TOP, NORMAL, TEX_NULL, TOP_MARK_CODE, VMODE,
};
use crate::errors::ErrorOutcome;
use crate::helpers::badness;
use crate::input::OUTPUT_TEXT;
use crate::main_control::{COPY_CODE, LAST_BOX_CODE};
use crate::mem::{CONTRIB_HEAD, HOLD_HEAD, PAGE_HEAD, PAGE_INS_HEAD};
use crate::node::{AnyBox, AnyNode, InsNode, NodePtr, PageInsNode, WhatsitSubtype};
use crate::packaging::{ADDITIONAL, EXACTLY};
use crate::scan::fetch_box;
use crate::scan::MARK_VAL;
use crate::state::{EngineState, GroupCode, PageContents, ShipoutState};
use crate::units::{MemIdx, MemPtr, Scaled};

// Indices into `page_so_far`, whose entries 2 to 5 hold the stretch of each order.
const PAGE_GOAL: usize = 0;
const PAGE_TOTAL: usize = 1;
const PAGE_SHRINK: usize = 6;
const PAGE_DEPTH: usize = 7;

pub(crate) const FIRE_UP_INIT: i16 = 1;
pub(crate) const FIRE_UP_DONE: i16 = 2;
const PAGE_INS_NODE_SIZE: i32 = 4;
const DEPLORABLE: i32 = 100_000;

pub(crate) enum PageBuildAbort {
    Input(crate::input::InputAbort),
    NodeRecycle(crate::scan::NodeRecycleAbort),
    Save(crate::save::SaveAbort),
    Scc(crate::scan::SccAbort),
    Shipout(crate::shipout::ShipoutAbort),
    VertBreak(crate::packaging::VertBreakAbort),
    MainMemoryOverflow,
    MainMemoryOverflowRunaway,
    Confusion(&'static str),
}

impl From<crate::input::InputAbort> for PageBuildAbort {
    fn from(a: crate::input::InputAbort) -> Self {
        PageBuildAbort::Input(a)
    }
}

impl From<crate::scan::NodeRecycleAbort> for PageBuildAbort {
    fn from(a: crate::scan::NodeRecycleAbort) -> Self {
        PageBuildAbort::NodeRecycle(a)
    }
}

impl From<crate::scan::SccAbort> for PageBuildAbort {
    fn from(a: crate::scan::SccAbort) -> Self {
        PageBuildAbort::Scc(a)
    }
}

impl From<crate::shipout::ShipoutAbort> for PageBuildAbort {
    fn from(a: crate::shipout::ShipoutAbort) -> Self {
        PageBuildAbort::Shipout(a)
    }
}

impl From<crate::packaging::VertBreakAbort> for PageBuildAbort {
    fn from(a: crate::packaging::VertBreakAbort) -> Self {
        PageBuildAbort::VertBreak(a)
    }
}

impl From<crate::save::SaveAbort> for PageBuildAbort {
    fn from(e: crate::save::SaveAbort) -> Self {
        PageBuildAbort::Save(e)
    }
}

impl PageBuildAbort {
    /// `MainMemoryOverflowRunaway` also reports the runaway text.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            PageBuildAbort::Input(a) => a.into_engine_abort(),
            PageBuildAbort::NodeRecycle(a) => a.into_engine_abort(),
            PageBuildAbort::Save(a) => a.into_engine_abort(),
            PageBuildAbort::Scc(a) => a.into_engine_abort(),
            PageBuildAbort::Shipout(a) => a.into_engine_abort(),
            PageBuildAbort::VertBreak(a) => a.into_engine_abort(),
            PageBuildAbort::MainMemoryOverflow => crate::errors::EngineAbort::Overflow {
                what: "main memory size",
                n: MEM_TOP + 1,
            },
            PageBuildAbort::MainMemoryOverflowRunaway => {
                crate::errors::EngineAbort::MainMemoryOverflow
            }
            PageBuildAbort::Confusion(s) => crate::errors::EngineAbort::Confusion(s),
        }
    }
}

fn page_error(outcome: ErrorOutcome) -> Result<(), PageBuildAbort> {
    crate::scan::scc_err(outcome).map_err(PageBuildAbort::Scc)
}

pub fn initialize_pagebuilder_variables(st: &mut EngineState) {
    st.shipout.page_max_depth = 0;
}

/// A record on the `page_ins_head` ring.
fn page_ins(p: MemPtr) -> NodePtr<PageInsNode> {
    NodePtr::at(p)
}

/// TeX's `contrib_tail`, the tail of the outermost vertical list.
fn set_contrib_tail(st: &mut EngineState, tail: MemPtr) {
    if st.input.nest_ptr == 0 {
        st.input.cur_list.tail = tail;
    } else {
        st.input.nest[0].tail = tail;
    }
}

fn freeze_page_specs(st: &mut EngineState, contents: PageContents) {
    let shipout = &mut st.shipout;
    shipout.page_contents = contents;
    shipout.page_so_far[PAGE_GOAL] = st.eqtb.dimen_par(DIMEN_PAR_VSIZE);
    shipout.page_max_depth = st.eqtb.dimen_par(DIMEN_PAR_MAX_DEPTH);
    shipout.page_so_far[PAGE_TOTAL..=PAGE_DEPTH].fill(0);
    shipout.least_page_cost = MAX_HALFWORD;
}

fn ensure_vbox(st: &mut EngineState, n: u8) -> Result<(), PageBuildAbort> {
    if !matches!(fetch_box(st, n as i32), Some(AnyBox::HList(_))) {
        return Ok(());
    }
    crate::output::print_err(st, "Insertions can only be added to a vbox");
    st.errors.help(&[
        r"Tut tut: You're trying to \insert into a",
        r"\box register that now contains an \hbox.",
        "Proceed, and I'll discard its present contents.",
    ]);
    page_error(crate::errors::box_error(st, n))
}

fn push_nest_pagebuilder(st: &mut EngineState) -> Result<(), PageBuildAbort> {
    crate::mem::push_nest(st).map_err(|e| match e {
        crate::mem::PushNestAbort::SemanticNest => PageBuildAbort::Save(
            crate::save::SaveAbort::Overflow("semantic nest size", st.input.nest_size),
        ),
        crate::mem::PushNestAbort::MainMemory => PageBuildAbort::MainMemoryOverflowRunaway,
    })
}

/// Records the new mark `mark_ptr` as the first and bottom mark of class `class`.
fn update_marks(st: &mut EngineState, class: i32, mark_ptr: MemPtr) -> Result<(), PageBuildAbort> {
    if class != 0 {
        let sa = st
            .mem
            .create_sa_element(&mut st.scanner, MARK_VAL, class)
            .ok_or(PageBuildAbort::MainMemoryOverflow)?;
        if sa.child(&st.mem, 1).is_null() {
            sa.set_child(&mut st.mem, 1, mark_ptr);
            st.mem.add_token_ref(mark_ptr);
        }
        let bot = sa.child(&st.mem, 2);
        if !bot.is_null() {
            st.mem.delete_token_ref(bot);
        }
        sa.set_child(&mut st.mem, 2, mark_ptr);
        st.mem.add_token_ref(mark_ptr);
        return Ok(());
    }
    let marks = &mut st.scanner.cur_mark;
    if marks[FIRST_MARK_CODE].is_null() {
        marks[FIRST_MARK_CODE] = mark_ptr;
        st.mem.add_token_ref(mark_ptr);
    }
    if !marks[BOT_MARK_CODE].is_null() {
        st.mem.delete_token_ref(marks[BOT_MARK_CODE]);
    }
    marks[BOT_MARK_CODE] = mark_ptr;
    st.mem.add_token_ref(mark_ptr);
    Ok(())
}

/// Moves insertion `ins` at `p` into its box when it made it onto the page.
/// Returns whether it has to wait for the next page.
fn place_insertion(
    st: &mut EngineState,
    ins: NodePtr<InsNode>,
    p: MemPtr,
) -> Result<bool, PageBuildAbort> {
    let mut r = PAGE_INS_HEAD.link(&st.mem);
    while page_ins(r).subtype(&st.mem) != ins.subtype(&st.mem) {
        r = r.link(&st.mem);
    }
    let rec = page_ins(r);
    if rec.best_ins_ptr(&st.mem).is_null() {
        return Ok(true);
    }
    let mut s = rec.last_ins_ptr(&st.mem);
    let list = ins.ins_ptr(&st.mem);
    s.set_link(&mut st.mem, list);
    if rec.best_ins_ptr(&st.mem) != p {
        while !s.link(&st.mem).is_null() {
            s = s.link(&st.mem);
        }
        rec.set_last_ins_ptr(&mut st.mem, s);
        return Ok(false);
    }

    let mut wait = false;
    let broken_ptr = rec.broken_ptr(&st.mem);
    if rec.is_split_up(&st.mem) && rec.broken_ins(&st.mem) == p && !broken_ptr.is_null() {
        while s.link(&st.mem) != broken_ptr {
            s = s.link(&st.mem);
        }
        s.set_link(&mut st.mem, MemPtr::NULL);
        st.eqtb
            .set_glue_par(GLUE_PAR_SPLIT_TOP_SKIP, ins.split_top_ptr(&st.mem));
        let pruned = st
            .mem
            .prune_page_top(&mut st.scanner, &st.eqtb, broken_ptr.0, false)?;
        ins.set_ins_ptr(&mut st.mem, pruned);
        if !pruned.is_null() {
            let tmp = crate::packaging::vpackage(st, pruned, 0, ADDITIONAL, MAX_HALFWORD)?;
            let size = tmp.height(&st.mem).0 + tmp.depth(&st.mem).0;
            ins.set_height(&mut st.mem, Scaled(size));
            st.mem.free_node(tmp.ptr(), BOX_NODE_SIZE);
            wait = true;
        }
    }
    rec.set_best_ins_ptr(&mut st.mem, MemPtr::NULL);
    let n = rec.subtype(&st.mem) as u8;
    let b = fetch_box(st, n as i32).expect("insertion box made in fire_up");
    let list = b.list_ptr(&st.mem);
    st.mem.free_node(b.ptr(), BOX_NODE_SIZE);
    let packed = crate::packaging::vpackage(st, list, 0, ADDITIONAL, MAX_HALFWORD)?;
    st.eqtb.set_box_reg(n as i32, packed.ptr());
    Ok(wait)
}

/// TeX's `fire_up`: puts the best page into `\box255` and runs the output routine.
fn fire_up(st: &mut EngineState, c: MemPtr) -> Result<(), PageBuildAbort> {
    let best = st.shipout.best_page_break;
    let output_penalty = match best.as_idx().map(|b| st.mem.classify(b)) {
        Some(Ok(AnyNode::Penalty(pen))) => {
            let v = pen.penalty(&st.mem);
            pen.set_penalty(&mut st.mem, INF_PENALTY);
            v
        }
        _ => INF_PENALTY,
    };
    st.eqtb
        .geq_word_define(INT_BASE + INT_PAR_OUTPUT_PENALTY, output_penalty);

    if let Some(root) = st.scanner.sa_root[MARK_VAL as usize].as_idx() {
        if st.mem.do_marks(&mut st.scanner, FIRE_UP_INIT, 0, root) {
            st.scanner.sa_root[MARK_VAL as usize] = MemPtr::NULL;
        }
    }
    if !st.scanner.cur_mark[BOT_MARK_CODE].is_null() {
        if !st.scanner.cur_mark[TOP_MARK_CODE].is_null() {
            st.mem.delete_token_ref(st.scanner.cur_mark[TOP_MARK_CODE]);
        }
        st.scanner.cur_mark[TOP_MARK_CODE] = st.scanner.cur_mark[BOT_MARK_CODE];
        st.mem.add_token_ref(st.scanner.cur_mark[TOP_MARK_CODE]);
        st.mem
            .delete_token_ref(st.scanner.cur_mark[FIRST_MARK_CODE]);
        st.scanner.cur_mark[FIRST_MARK_CODE] = MemPtr::NULL;
    }

    if c == best {
        st.shipout.best_page_break = MemPtr::NULL;
    }
    if !st.eqtb.box_reg(255).is_null() {
        crate::output::error_here_with_diagnostic(st, "");
        crate::output::print_esc(st, "box");
        crate::output::print(st, "255 is not void");
        crate::output::capture_to_diagnostic(st, std::ptr::null_mut());
        st.errors.help(&[
            r"You shouldn't use \box255 except in \output routines.",
            "Proceed, and I'll discard its present contents.",
        ]);
        page_error(crate::errors::box_error(st, 255))?;
    }

    st.shipout.insert_penalties = 0;
    let save_split_top_skip = st.eqtb.glue_par(GLUE_PAR_SPLIT_TOP_SKIP);
    let process_inserts =
        st.eqtb.int_par(INT_PAR_HOLDING_INSERTS) <= 0 && !st.config.semantic_pagination_enabled;
    if process_inserts {
        let mut r = PAGE_INS_HEAD.link(&st.mem);
        while r != PAGE_INS_HEAD {
            let rec = page_ins(r);
            if !rec.best_ins_ptr(&st.mem).is_null() {
                let n = rec.subtype(&st.mem) as u8;
                ensure_vbox(st, n)?;
                if st.eqtb.box_reg(n as i32).is_null() {
                    let b = st
                        .mem
                        .new_null_box()
                        .ok_or(PageBuildAbort::MainMemoryOverflow)?;
                    st.eqtb.set_box_reg(n as i32, b.ptr());
                }
                // The box's `list_ptr` word serves as the head link of its list.
                let mut p = st.eqtb.box_reg(n as i32) + 5;
                while !p.link(&st.mem).is_null() {
                    p = p.link(&st.mem);
                }
                rec.set_last_ins_ptr(&mut st.mem, p);
            }
            r = r.link(&st.mem);
        }
    }

    let mut hold_tail = HOLD_HEAD;
    hold_tail.set_link(&mut st.mem, MemPtr::NULL);
    let mut prev_p = PAGE_HEAD;
    let mut p = prev_p.link(&st.mem);
    while p != st.shipout.best_page_break {
        match st.mem.classify(MemIdx::new(p)) {
            Ok(AnyNode::Ins(ins)) if process_inserts => {
                let wait = place_insertion(st, ins, p)?;
                let next = ins.link(&st.mem);
                prev_p.set_link(&mut st.mem, next);
                ins.set_link(&mut st.mem, MemPtr::NULL);
                if wait {
                    hold_tail.set_link(&mut st.mem, p);
                    hold_tail = p;
                    st.shipout.insert_penalties += 1;
                } else {
                    let split_top = ins.split_top_ptr(&st.mem);
                    st.mem.delete_glue_ref(split_top);
                    st.mem.free_node(p, INS_NODE_SIZE);
                }
            }
            Ok(AnyNode::Mark(mark)) => {
                update_marks(st, mark.mark_class(&st.mem), mark.mark_ptr(&st.mem))?;
                prev_p = p;
            }
            _ => prev_p = p,
        }
        p = prev_p.link(&st.mem);
    }
    st.eqtb
        .set_glue_par(GLUE_PAR_SPLIT_TOP_SKIP, save_split_top_skip);

    if !p.is_null() {
        if CONTRIB_HEAD.link(&st.mem).is_null() {
            set_contrib_tail(st, st.shipout.page_tail);
        }
        let contrib = CONTRIB_HEAD.link(&st.mem);
        st.shipout.page_tail.set_link(&mut st.mem, contrib);
        CONTRIB_HEAD.set_link(&mut st.mem, p);
        prev_p.set_link(&mut st.mem, MemPtr::NULL);
    }

    let save_vbadness = st.eqtb.int_par(INT_PAR_VBADNESS);
    st.eqtb.set_int_par(INT_PAR_VBADNESS, INF_BAD);
    let save_vfuzz = st.eqtb.dimen_par(DIMEN_PAR_VFUZZ);
    st.eqtb.set_dimen_par(DIMEN_PAR_VFUZZ, MAX_HALFWORD);
    let page = crate::packaging::vpackage(
        st,
        PAGE_HEAD.link(&st.mem),
        st.shipout.best_size,
        EXACTLY,
        st.shipout.page_max_depth,
    )?;
    st.eqtb.set_box_reg(255, page.ptr());
    st.eqtb.set_int_par(INT_PAR_VBADNESS, save_vbadness);
    st.eqtb.set_dimen_par(DIMEN_PAR_VFUZZ, save_vfuzz);
    if let Some(spec) = st.shipout.last_glue {
        st.mem.delete_glue_ref(spec);
    }

    st.shipout.page_contents = PageContents::Empty;
    st.shipout.page_tail = PAGE_HEAD;
    PAGE_HEAD.set_link(&mut st.mem, MemPtr::NULL);
    st.shipout.last_glue = None;
    st.shipout.last_penalty = 0;
    st.shipout.last_kern = 0;
    st.shipout.last_node_type = -1;
    st.shipout.page_so_far[PAGE_DEPTH] = 0;
    st.shipout.page_max_depth = 0;
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

    if let Some(root) = st.scanner.sa_root[MARK_VAL as usize].as_idx() {
        if st.mem.do_marks(&mut st.scanner, FIRE_UP_DONE, 0, root) {
            st.scanner.sa_root[MARK_VAL as usize] = MemPtr::NULL;
        }
    }
    let marks = &mut st.scanner.cur_mark;
    if !marks[TOP_MARK_CODE].is_null() && marks[FIRST_MARK_CODE].is_null() {
        marks[FIRST_MARK_CODE] = marks[TOP_MARK_CODE];
        st.mem.add_token_ref(marks[TOP_MARK_CODE]);
    }

    let output_routine = st.eqtb.local(LOCAL_OUTPUT_ROUTINE);
    if !output_routine.is_null() {
        if st.shipout.dead_cycles < st.eqtb.int_par(INT_PAR_MAX_DEAD_CYCLES) {
            st.shipout.output_active = true;
            st.shipout.dead_cycles += 1;
            push_nest_pagebuilder(st)?;
            st.input.cur_list.mode = -VMODE;
            st.input.cur_list.set_aux_s1(IGNORE_DEPTH);
            st.input.cur_list.mode_line = -st.mem.line;
            crate::input::begin_token_list(st, output_routine, OUTPUT_TEXT)?;
            st.save_stack
                .new_save_level(GroupCode::Output, st.mem.line)?;
            crate::main_control::normal_paragraph(st)?;
            crate::scan::scan_left_brace(st)?;
            return Ok(());
        }
        crate::output::error_here_with_diagnostic(st, "Output loop---");
        crate::output::print_int(st, st.shipout.dead_cycles);
        crate::output::print(st, " consecutive dead cycles");
        crate::output::capture_to_diagnostic(st, std::ptr::null_mut());
        st.errors.help(&[
            r"I've concluded that your \output is awry; it never does a",
            r"\shipout, so I'm shipping \box255 out myself. Next time",
            r"increase \maxdeadcycles if you want me to be more patient!",
        ]);
        page_error(crate::errors::error(st))?;
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
    let discards = st.scanner.disc_ptr[LAST_BOX_CODE as usize];
    st.mem
        .flush_node_list(discards.0)
        .map_err(PageBuildAbort::Confusion)?;
    st.scanner.disc_ptr[LAST_BOX_CODE as usize] = MemPtr::NULL;
    unsafe { crate::shipout::ship_out(st, page)? };
    st.eqtb.set_box_reg(255, MemPtr::NULL);
    Ok(())
}

fn update_last_node_info(st: &mut EngineState, node: AnyNode, node_type: i32) {
    st.shipout.last_penalty = 0;
    st.shipout.last_kern = 0;
    st.shipout.last_node_type = node_type + 1;
    st.shipout.last_glue = None;
    match node {
        AnyNode::Glue(glue) => {
            let spec = glue.glue_ptr(&st.mem);
            st.shipout.last_glue = Some(spec);
            let refs = spec.ref_count(&st.mem);
            spec.set_ref_count(&mut st.mem, refs + 1);
        }
        AnyNode::Penalty(penalty) => st.shipout.last_penalty = penalty.penalty(&st.mem),
        AnyNode::Kern(kern) => st.shipout.last_kern = kern.width(&st.mem).0,
        _ => {}
    }
}

impl ShipoutState {
    fn page_badness(&self) -> i32 {
        let so_far = &self.page_so_far;
        if so_far[PAGE_TOTAL] < so_far[PAGE_GOAL] {
            if so_far[3] != 0 || so_far[4] != 0 || so_far[5] != 0 {
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
fn scale_by_count(st: &mut EngineState, x: i32, count: i32) -> i32 {
    if count == 1000 {
        x
    } else {
        st.errors.x_over_n(x, 1000) * count
    }
}

/// Adds insertion `ins` at `p` to the current page, splitting it if needed.
fn append_insertion(
    st: &mut EngineState,
    ins: NodePtr<InsNode>,
    p: MemPtr,
) -> Result<(), PageBuildAbort> {
    if st.shipout.page_contents == PageContents::Empty {
        freeze_page_specs(st, PageContents::InsertsOnly);
    }
    let n = ins.subtype(&st.mem) as u8;
    let mut r = PAGE_INS_HEAD;
    while n as u16 >= page_ins(r.link(&st.mem)).subtype(&st.mem) {
        r = r.link(&st.mem);
    }
    if page_ins(r).subtype(&st.mem) as u8 != n {
        let q = st
            .mem
            .get_node(PAGE_INS_NODE_SIZE)
            .ok_or(PageBuildAbort::MainMemoryOverflow)?;
        let next = r.link(&st.mem);
        q.set_link(&mut st.mem, next);
        r.set_link(&mut st.mem, q);
        r = q;
        let rec = page_ins(r);
        rec.set_subtype(&mut st.mem, n as u16);
        rec.set_split_up(&mut st.mem, false);
        ensure_vbox(st, n)?;
        let height = match fetch_box(st, n as i32) {
            Some(b) => b.height(&st.mem).0 + b.depth(&st.mem).0,
            None => 0,
        };
        rec.set_height(&mut st.mem, Scaled(height));
        rec.set_best_ins_ptr(&mut st.mem, MemPtr::NULL);
        let skip = st.eqtb.skip(n as i32);
        let count = st.eqtb.count(n as i32);
        let h = scale_by_count(st, height, count);
        st.shipout.page_so_far[PAGE_GOAL] -= h + skip.width(&st.mem).0;
        let order = skip.stretch_order(&st.mem) as usize;
        st.shipout.page_so_far[2 + order] += skip.stretch(&st.mem).0;
        st.shipout.page_so_far[PAGE_SHRINK] += skip.shrink(&st.mem).0;
        if skip.shrink_order(&st.mem) != NORMAL && skip.shrink(&st.mem).0 != 0 {
            crate::output::error_here_with_diagnostic(st, "Infinite glue shrinkage inserted from ");
            crate::output::print_esc(st, "skip");
            crate::output::print_int(st, n as i32);
            crate::output::capture_to_diagnostic(st, std::ptr::null_mut());
            st.errors.help(&[
                "The correction glue for page breaking with insertions",
                "must have finite shrinkability. But you may proceed,",
                "since the offensive shrinkability has been made finite.",
            ]);
            page_error(crate::errors::error(st))?;
        }
    }

    let rec = page_ins(r);
    if rec.is_split_up(&st.mem) {
        st.shipout.insert_penalties += ins.float_cost(&st.mem);
        return Ok(());
    }
    rec.set_last_ins_ptr(&mut st.mem, p);
    let so_far = st.shipout.page_so_far;
    let delta = so_far[PAGE_GOAL] - so_far[PAGE_TOTAL] - so_far[PAGE_DEPTH] + so_far[PAGE_SHRINK];
    let count = st.eqtb.count(n as i32);
    let ins_height = ins.height(&st.mem).0;
    let h = scale_by_count(st, ins_height, count);
    let max_height = st.eqtb.dimen(n as i32);
    if (h <= 0 || h <= delta) && ins_height + rec.height(&st.mem).0 <= max_height {
        st.shipout.page_so_far[PAGE_GOAL] -= h;
        let height = rec.height(&st.mem).0 + ins_height;
        rec.set_height(&mut st.mem, Scaled(height));
        return Ok(());
    }

    let mut w = if count <= 0 {
        MAX_HALFWORD
    } else {
        let w = so_far[PAGE_GOAL] - so_far[PAGE_TOTAL] - so_far[PAGE_DEPTH];
        if count == 1000 {
            w
        } else {
            st.errors.x_over_n(w, count) * 1000
        }
    };
    let room = max_height - rec.height(&st.mem).0;
    if w > room {
        w = room;
    }
    let q = crate::packaging::vert_break(st, ins.ins_ptr(&st.mem).0, w, ins.depth(&st.mem).0)?;
    let height = rec.height(&st.mem).0 + st.scanner.best_height_plus_depth;
    rec.set_height(&mut st.mem, Scaled(height));
    if count != 1000 {
        st.scanner.best_height_plus_depth =
            scale_by_count(st, st.scanner.best_height_plus_depth, count);
    }
    st.shipout.page_so_far[PAGE_GOAL] -= st.scanner.best_height_plus_depth;
    rec.set_split_up(&mut st.mem, true);
    rec.set_broken_ptr(&mut st.mem, MemPtr(q));
    rec.set_broken_ins(&mut st.mem, p);
    if q == TEX_NULL {
        st.shipout.insert_penalties += EJECT_PENALTY;
    } else if let Ok(AnyNode::Penalty(pen)) = st.mem.classify(MemIdx::new(q)) {
        st.shipout.insert_penalties += pen.penalty(&st.mem);
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

pub(crate) fn build_page(st: &mut EngineState) -> Result<(), PageBuildAbort> {
    if CONTRIB_HEAD.link(&st.mem).is_null() || st.shipout.output_active {
        return Ok(());
    }
    while let Some(idx) = CONTRIB_HEAD.link(&st.mem).as_idx() {
        let p = MemPtr::from_idx(idx);
        let node_type = st.mem.type_code(idx) as i32;
        let node = st.mem.classify(idx).expect("valid page node");
        if let Some(spec) = st.shipout.last_glue {
            st.mem.delete_glue_ref(spec);
        }
        update_last_node_info(st, node, node_type);

        let below_box = st.shipout.page_contents < PageContents::BoxThere;
        let step = match node {
            AnyNode::HList(_) | AnyNode::VList(_) | AnyNode::Rule(_) => {
                let (height, depth) = match node {
                    AnyNode::HList(b) => (b.height(&st.mem).0, b.depth(&st.mem).0),
                    AnyNode::VList(b) => (b.height(&st.mem).0, b.depth(&st.mem).0),
                    AnyNode::Rule(r) => (r.height(&st.mem).raw(), r.depth(&st.mem).raw()),
                    _ => unreachable!(),
                };
                if below_box {
                    if st.shipout.page_contents == PageContents::Empty {
                        freeze_page_specs(st, PageContents::BoxThere);
                    } else {
                        st.shipout.page_contents = PageContents::BoxThere;
                    }
                    let q = st
                        .mem
                        .new_skip_param(&st.eqtb, GLUE_PAR_TOP_SKIP)
                        .ok_or(PageBuildAbort::MainMemoryOverflow)?;
                    let spec = q.glue_ptr(&st.mem);
                    let width = spec.width(&st.mem).0;
                    let width = if width > height { width - height } else { 0 };
                    spec.set_width(&mut st.mem, Scaled(width));
                    q.set_link(&mut st.mem, p);
                    CONTRIB_HEAD.set_link(&mut st.mem, q.ptr());
                    continue;
                }
                st.shipout.page_so_far[PAGE_TOTAL] += st.shipout.page_so_far[PAGE_DEPTH] + height;
                st.shipout.page_so_far[PAGE_DEPTH] = depth;
                PageStep::Contribute
            }
            AnyNode::Whatsit(w) => {
                if matches!(
                    w.whatsit_subtype(&st.mem),
                    Some(WhatsitSubtype::Pic | WhatsitSubtype::Pdf)
                ) {
                    st.shipout.page_so_far[PAGE_TOTAL] +=
                        st.shipout.page_so_far[PAGE_DEPTH] + w.height(&st.mem).0;
                    st.shipout.page_so_far[PAGE_DEPTH] = w.depth(&st.mem).0;
                }
                PageStep::Contribute
            }
            AnyNode::Glue(_) if below_box => PageStep::Discard,
            AnyNode::Glue(_) => {
                let tail = st.mem.classify(MemIdx::new(st.shipout.page_tail));
                if tail.is_ok_and(AnyNode::precedes_break) {
                    PageStep::Break(0)
                } else {
                    PageStep::UpdateHeights
                }
            }
            AnyNode::Kern(_) if below_box => PageStep::Discard,
            AnyNode::Kern(kern) => {
                let next = kern.link(&st.mem);
                if next.is_null() {
                    return Ok(());
                }
                if matches!(st.mem.classify(MemIdx::new(next)), Ok(AnyNode::Glue(_))) {
                    PageStep::Break(0)
                } else {
                    PageStep::UpdateHeights
                }
            }
            AnyNode::Penalty(_) if below_box => PageStep::Discard,
            AnyNode::Penalty(pen) => PageStep::Break(pen.penalty(&st.mem)),
            AnyNode::Mark(_) => PageStep::Contribute,
            AnyNode::Ins(ins) => {
                append_insertion(st, ins, p)?;
                PageStep::Contribute
            }
            _ => return Err(PageBuildAbort::Confusion("page")),
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
                        b + pi + st.shipout.insert_penalties
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
                            let last = rec.last_ins_ptr(&st.mem);
                            rec.set_best_ins_ptr(&mut st.mem, last);
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
                    AnyNode::Kern(kern) => kern.width(&st.mem).0,
                    AnyNode::Glue(glue) => page_glue_width(st, glue)?,
                    _ => unreachable!("only glue and kern update the page heights"),
                };
                st.shipout.page_so_far[PAGE_TOTAL] += st.shipout.page_so_far[PAGE_DEPTH] + width;
                st.shipout.page_so_far[PAGE_DEPTH] = 0;
                contribute(st, p);
            }
            PageStep::Contribute => contribute(st, p),
            PageStep::Discard => {
                let next = p.link(&st.mem);
                CONTRIB_HEAD.set_link(&mut st.mem, next);
                p.set_link(&mut st.mem, MemPtr::NULL);
                if st.eqtb.int_par(INT_PAR_SAVING_VDISCARDS) <= 0 {
                    st.mem
                        .flush_node_list(p.0)
                        .map_err(PageBuildAbort::Confusion)?;
                } else {
                    let discards = &mut st.scanner.disc_ptr;
                    if discards[LAST_BOX_CODE as usize].is_null() {
                        discards[LAST_BOX_CODE as usize] = p;
                    } else {
                        discards[COPY_CODE as usize].set_link(&mut st.mem, p);
                    }
                    discards[COPY_CODE as usize] = p;
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
) -> Result<i32, PageBuildAbort> {
    let spec = glue.glue_ptr(&st.mem);
    let order = spec.stretch_order(&st.mem) as usize;
    st.shipout.page_so_far[2 + order] += spec.stretch(&st.mem).0;
    st.shipout.page_so_far[PAGE_SHRINK] += spec.shrink(&st.mem).0;
    if spec.shrink_order(&st.mem) == NORMAL || spec.shrink(&st.mem).0 == 0 {
        return Ok(spec.width(&st.mem).0);
    }
    crate::output::print_err(st, "Infinite glue shrinkage found on current page");
    st.errors.help(&[
        "The page about to be output contains some infinitely",
        r"shrinkable glue, e.g., `\vss' or `\vskip 0pt minus 1fil'.",
        "Such glue doesn't belong there; but you can safely proceed,",
        "since the offensive shrinkability has been made finite.",
    ]);
    page_error(crate::errors::error(st))?;
    let finite = st
        .mem
        .new_spec(spec)
        .ok_or(PageBuildAbort::MainMemoryOverflow)?;
    finite.set_shrink_order(&mut st.mem, NORMAL);
    st.mem.delete_glue_ref(spec);
    glue.set_glue_ptr(&mut st.mem, finite);
    Ok(finite.width(&st.mem).0)
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
