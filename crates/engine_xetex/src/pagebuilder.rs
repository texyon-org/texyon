//! Page builder functions translated from `xetex/xetex-pagebuilder.c`.
#![allow(non_upper_case_globals)]

use crate::consts::{
    AWFUL_BAD, BOX_BASE, BOX_NODE_SIZE, CONTRIB_HEAD, COUNT_BASE, DIMEN_BASE, EJECT_PENALTY,
    GLUE_BASE, IGNORE_DEPTH, INT_BASE, LOCAL_BASE, MAX_HALFWORD, MEM_TOP, OUTPUT_TEXT, SCALED_BASE,
    SKIP_BASE, TEX_NULL, VMODE,
};

use crate::errors::ErrorOutcome;
use crate::helpers::badness;
use crate::node::{AnyBox, AnyNode, GlueSpecNode, NodePtr, SaNode, WhatsitSubtype};
use crate::state::{EngineState, EqtbState, GroupCode, MemState, ShipoutState};
use crate::units::{MemIdx, MemPtr, Scaled};

// Node types (xetex_format.h)
const MATH_NODE: i32 = crate::consts::MATH_NODE as i32;

// Glue/page constants (xetex-constants.h)
const PAGE_INS_HEAD: i32 = MEM_TOP;
const PAGE_HEAD: i32 = MEM_TOP - 2;
const HOLD_HEAD: i32 = MEM_TOP - 4;
const NORMAL: i32 = 0;
const EMPTY: u8 = 0;
const INSERTS_ONLY: u8 = 1;
const BOX_THERE: u8 = 2;
const INSERTING: u16 = 0;
const SPLIT_UP: i32 = 1;
const MARK_VAL: usize = 7;
const FIRE_UP_INIT: i16 = 1;
const FIRE_UP_DONE: i16 = 2;
const PAGE_INS_NODE_SIZE: i32 = 4;
const INS_NODE_SIZE: i32 = 5;
const ADDITIONAL: i16 = 1;
const EXACTLY: i16 = 0;
const INF_BAD: i32 = 10_000;
const INF_PENALTY: i32 = 10_000;
const TOP_MARK_CODE: usize = 0;
const FIRST_MARK_CODE: usize = 1;
const BOT_MARK_CODE: usize = 2;
const LAST_BOX_CODE: usize = 2;
const COPY_CODE: usize = 1;

// eqtb bases and parameter indices (xetex_format.h)
const INT_PAR__output_penalty: i32 = 39;
const INT_PAR__max_dead_cycles: i32 = 40;
const INT_PAR__holding_inserts: i32 = 53;
const INT_PAR__saving_vdiscards: i32 = 63;
const INT_PAR__vbadness: i32 = 27;
const DIMEN_PAR__vsize: i32 = 4;
const DIMEN_PAR__max_depth: i32 = 5;
const DIMEN_PAR__vfuzz: i32 = 9;
const GLUE_PAR__top_skip: i32 = 9;
const GLUE_PAR__split_top_skip: i32 = 10;
const LOCAL__output_routine: i32 = 1;

extern "C" {}

pub fn initialize_pagebuilder_variables(st: &mut crate::state::EngineState) {
    st.shipout.page_max_depth = 0;
}

fn freeze_page_specs(eqtb: &EqtbState, shipout: &mut ShipoutState, s: u8) {
    shipout.set_page_contents(s);
    shipout.set_page_so_far(0, eqtb.b32s1(DIMEN_BASE + DIMEN_PAR__vsize));
    shipout.set_page_max_depth(eqtb.b32s1(DIMEN_BASE + DIMEN_PAR__max_depth));
    shipout.set_page_so_far(7, 0);
    shipout.set_page_so_far(1, 0);
    shipout.set_page_so_far(2, 0);
    shipout.set_page_so_far(3, 0);
    shipout.set_page_so_far(4, 0);
    shipout.set_page_so_far(5, 0);
    shipout.set_page_so_far(6, 0);
    shipout.set_least_page_cost(MAX_HALFWORD);
}

/// Decides whether box register `n` is an `\hbox` that must be flagged.
fn ensure_vbox_is_hbox(eqtb: &EqtbState, mem: &MemState, n: u8) -> bool {
    let p = eqtb.b32s1(BOX_BASE + n as i32);
    if p == TEX_NULL {
        return false;
    }
    matches!(mem.classify_box(MemIdx::new(p)), Ok(Some(AnyBox::HList(_))))
}

pub(crate) enum PageBuildAbort {
    Input(crate::input::InputAbort),
    NodeRecycle(crate::scan::NodeRecycleAbort),
    Save(crate::save::SaveAbort),
    Scc(crate::scan::SccAbort),
    Shipout(crate::shipout::ShipoutAbort),
    VertBreak(crate::packaging::VertBreakAbort),
    MainMemoryOverflow,
    MainMemoryOverflowRunaway,
    Confusion(&'static std::ffi::CStr),
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

fn page_error(outcome: ErrorOutcome) -> Result<(), PageBuildAbort> {
    crate::scan::scc_err(outcome).map_err(PageBuildAbort::Scc)
}

fn page_save(abort: Option<crate::save::SaveAbort>) -> Result<(), PageBuildAbort> {
    match abort {
        Some(a) => Err(PageBuildAbort::Save(a)),
        None => Ok(()),
    }
}

fn page_get_node(mem: &mut MemState, s: i32) -> Result<i32, PageBuildAbort> {
    let p = crate::mem::get_node(mem, s);
    if p == crate::mem::ALLOC_FAILED {
        return Err(PageBuildAbort::MainMemoryOverflow);
    }
    Ok(p)
}

fn page_new_null_box(mem: &mut MemState) -> Result<i32, PageBuildAbort> {
    let p = crate::mem::new_null_box(mem);
    if p == crate::mem::ALLOC_FAILED {
        return Err(PageBuildAbort::MainMemoryOverflow);
    }
    Ok(p)
}

fn page_new_skip_param(
    mem: &mut MemState,
    eqtb: &EqtbState,
    n: i16,
) -> Result<i32, PageBuildAbort> {
    let p = crate::mem::new_skip_param(mem, eqtb, n);
    if p == crate::mem::ALLOC_FAILED {
        return Err(PageBuildAbort::MainMemoryOverflow);
    }
    Ok(p)
}

fn page_new_spec(mem: &mut MemState, p: i32) -> Result<i32, PageBuildAbort> {
    let q = crate::mem::new_spec(mem, p);
    if q == crate::mem::ALLOC_FAILED {
        return Err(PageBuildAbort::MainMemoryOverflow);
    }
    Ok(q)
}

fn ensure_vbox_in(st: &mut EngineState, n: u8) -> Result<(), PageBuildAbort> {
    if !ensure_vbox_is_hbox(&st.eqtb, &st.mem, n) {
        return Ok(());
    }
    crate::output::error_here_with_diagnostic_in(
        st,
        c"Insertions can only be added to a vbox".as_ptr(),
    );
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 3;
    st.errors.help_line[2] = c"Tut tut: You're trying to \\insert into a".as_ptr();
    st.errors.help_line[1] = c"\\box register that now contains an \\hbox.".as_ptr();
    st.errors.help_line[0] = c"Proceed, and I'll discard its present contents.".as_ptr();
    page_error(crate::errors::box_error_in(st, n))
}

fn push_nest_pagebuilder_in(st: &mut EngineState) -> Result<(), PageBuildAbort> {
    if st.input.nest_ptr > st.input.max_nest_stack {
        st.input.max_nest_stack = st.input.nest_ptr;
        if st.input.nest_ptr == st.input.nest_size {
            return Err(PageBuildAbort::Save(crate::save::SaveAbort::Overflow(
                c"semantic nest size",
                st.input.nest_size,
            )));
        }
    }
    st.input.nest[st.input.nest_ptr as usize] = st.input.cur_list;
    st.input.nest_ptr += 1;
    let h = crate::mem::get_avail(&mut st.mem);
    if h == crate::mem::ALLOC_FAILED {
        return Err(PageBuildAbort::MainMemoryOverflowRunaway);
    }
    st.input.cur_list.head.0 = h;
    st.input.cur_list.tail.0 = h;
    st.input.cur_list.prev_graf = 0;
    st.input.cur_list.mode_line = st.mem.line;
    st.input.cur_list.e_tex_aux = crate::units::MemPtr::NULL;
    Ok(())
}

impl PageBuildAbort {
    /// Map to the unified [`EngineAbort`] terminal.
    /// `MainMemoryOverflow` is a plain overflow; `MainMemoryOverflowRunaway`
    /// runs `runaway` first.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            PageBuildAbort::Input(a) => a.into_engine_abort(),
            PageBuildAbort::NodeRecycle(a) => a.into_engine_abort(),
            PageBuildAbort::Save(a) => a.into_engine_abort(),
            PageBuildAbort::Scc(a) => a.into_engine_abort(),
            PageBuildAbort::Shipout(a) => a.into_engine_abort(),
            PageBuildAbort::VertBreak(a) => a.into_engine_abort(),
            PageBuildAbort::MainMemoryOverflow => crate::errors::EngineAbort::Overflow {
                what: c"main memory size".as_ptr(),
                n: MEM_TOP + 1,
            },
            PageBuildAbort::MainMemoryOverflowRunaway => crate::mem::main_memory_overflow_abort(),
            PageBuildAbort::Confusion(s) => crate::errors::EngineAbort::Confusion(s.as_ptr()),
        }
    }
}

fn fire_up_in(st: &mut EngineState, c: i32) -> Result<(), PageBuildAbort> {
    // 1048: Set output_penalty
    let best_penalty = (st.shipout.best_page_break.0 != TEX_NULL)
        .then(|| {
            st.mem
                .classify_penalty(MemIdx::new(st.shipout.best_page_break.0))
                .expect("valid best page break")
        })
        .flatten();
    if let Some(best_penalty) = best_penalty {
        crate::save::geq_word_define(
            &mut st.eqtb,
            INT_BASE + INT_PAR__output_penalty,
            best_penalty.penalty(&st.mem),
        );
        best_penalty.set_penalty(&mut st.mem, INF_PENALTY);
    } else {
        crate::save::geq_word_define(
            &mut st.eqtb,
            INT_BASE + INT_PAR__output_penalty,
            INF_PENALTY,
        );
    }

    // Update extended marks
    let mark_root = st.scanner.sa_root[MARK_VAL];
    if mark_root != TEX_NULL
        && crate::packaging::do_marks(&mut st.mem, &mut st.scanner, FIRE_UP_INIT, 0, mark_root)
    {
        st.scanner.sa_root[MARK_VAL] = TEX_NULL;
    }

    // 1047: Update top_mark / first_mark / bot_mark
    if st.scanner.cur_mark[BOT_MARK_CODE] != TEX_NULL {
        if st.scanner.cur_mark[TOP_MARK_CODE] != TEX_NULL {
            crate::mem::delete_token_ref(&mut st.mem, st.scanner.cur_mark[TOP_MARK_CODE]);
        }
        st.scanner.cur_mark[TOP_MARK_CODE] = st.scanner.cur_mark[BOT_MARK_CODE];
        let top = st.scanner.cur_mark[TOP_MARK_CODE];
        st.mem.set_b32s0(top, st.mem.b32s0(top) + 1); // TOKEN_LIST_ref_count++
        crate::mem::delete_token_ref(&mut st.mem, st.scanner.cur_mark[FIRST_MARK_CODE]);
        st.scanner.cur_mark[FIRST_MARK_CODE] = TEX_NULL;
    }

    // 1049: Put optimal page into box 255
    if c == st.shipout.best_page_break.0 {
        st.shipout.best_page_break.0 = TEX_NULL;
    }

    if st.eqtb.b32s1(BOX_BASE + 255) != TEX_NULL {
        // 1050: box 255 is not void — error and discard
        crate::output::error_here_with_diagnostic_in(st, c"".as_ptr());
        crate::output::print_esc_cstr_in(st, c"box".as_ptr());
        crate::output::print_cstr_in(st, c"255 is not void".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] =
            c"You shouldn't use \\box255 except in \\output routines.".as_ptr();
        st.errors.help_line[0] = c"Proceed, and I'll discard its present contents.".as_ptr();
        page_error(crate::errors::box_error_in(st, 255))?;
    }

    st.shipout.insert_penalties = 0;
    let save_split_top_skip = st.eqtb.b32s1(GLUE_BASE + GLUE_PAR__split_top_skip);
    let process_inserts = st.eqtb.b32s1(INT_BASE + INT_PAR__holding_inserts) <= 0
        && !st.config.semantic_pagination_enabled;

    if process_inserts {
        // 1053: Prepare insertion boxes as queues
        let mut r = st.mem.b32s1(PAGE_INS_HEAD);
        while r != PAGE_INS_HEAD {
            if st.mem.b32s0(r + 2) != TEX_NULL {
                // PAGE_INS_NODE_best_ins_ptr != TEX_NULL
                let n = st.mem.b16s0(r) as u8; // NODE_subtype
                ensure_vbox_in(st, n)?;
                if st.eqtb.b32s1(BOX_BASE + n as i32) == TEX_NULL {
                    let b = page_new_null_box(&mut st.mem)?;
                    st.eqtb.set_b32s1(BOX_BASE + n as i32, b);
                }
                let mut p = st.eqtb.b32s1(BOX_BASE + n as i32) + 5;
                while st.mem.b32s1(p) != TEX_NULL {
                    p = st.mem.b32s1(p);
                }
                st.mem.set_b32s1(r + 2, p); // PAGE_INS_NODE_last_ins_ptr = p
            }
            r = st.mem.b32s1(r); // LLIST_link
        }
    }

    let mut hold_q = HOLD_HEAD;
    st.mem.set_b32s1(hold_q, TEX_NULL);
    let mut prev_p = PAGE_HEAD;
    let mut p = st.mem.b32s1(prev_p);

    // Walk page list up to best_page_break, processing insertions and marks
    while p != st.shipout.best_page_break.0 {
        let node = st.mem.classify(MemIdx::new(p)).expect("valid page node");
        let reset_p = if let (AnyNode::Ins(ins), true) = (node, process_inserts) {
            // 1055: Process insertion node
            let mut r = st.mem.b32s1(PAGE_INS_HEAD);
            while st.mem.b16s0(r) != ins.subtype(&st.mem) {
                // NODE_subtype(r) != NODE_subtype(p)
                r = st.mem.b32s1(r);
            }

            let wait = if st.mem.b32s0(r + 2) == TEX_NULL {
                // PAGE_INS_NODE_best_ins_ptr == TEX_NULL
                true
            } else {
                let mut s = st.mem.b32s1(r + 2); // PAGE_INS_NODE_last_ins_ptr
                st.mem.set_b32s1(s, ins.ins_ptr(&st.mem).0);

                if st.mem.b32s0(r + 2) == p {
                    // 1056: best_ins_ptr == p — wrap up the box
                    let mut inner_wait = false;
                    if st.mem.b16s1(r) as i32 == SPLIT_UP
                        && st.mem.b32s0(r + 1) == p
                        && st.mem.b32s1(r + 1) != TEX_NULL
                    {
                        // broken_ins == p && broken_ptr != TEX_NULL
                        while st.mem.b32s1(s) != st.mem.b32s1(r + 1) {
                            s = st.mem.b32s1(s);
                        }
                        st.mem.set_b32s1(s, TEX_NULL);
                        st.eqtb.set_b32s1(
                            GLUE_BASE + GLUE_PAR__split_top_skip,
                            ins.split_top_ptr(&st.mem).0,
                        );
                        let broken_ptr = st.mem.b32s1(r + 1);
                        let pruned = crate::packaging::prune_page_top(
                            &mut st.mem,
                            &mut st.scanner,
                            &st.eqtb,
                            broken_ptr,
                            false,
                        )?;
                        ins.set_ins_ptr(&mut st.mem, MemPtr::from_raw(pruned));
                        if ins.ins_ptr(&st.mem) != MemPtr::NULL {
                            // INSERTION_NODE_ins_ptr != TEX_NULL
                            let tmp_box = crate::packaging::vpackage_emit_in(
                                st,
                                ins.ins_ptr(&st.mem).0,
                                0,
                                ADDITIONAL,
                                MAX_HALFWORD,
                            )?;
                            let height_plus_depth =
                                tmp_box.height(&st.mem).0 + tmp_box.depth(&st.mem).0;
                            ins.set_height(&mut st.mem, Scaled(height_plus_depth));
                            crate::mem::free_node(&mut st.mem, tmp_box.idx().0, BOX_NODE_SIZE);
                            inner_wait = true;
                        }
                    }
                    st.mem.set_b32s0(r + 2, TEX_NULL); // best_ins_ptr = TEX_NULL
                    let n2 = st.mem.b16s0(r) as u8;
                    let box_n = st.eqtb.b32s1(BOX_BASE + n2 as i32);
                    let box_lp = st
                        .mem
                        .classify_box(MemIdx::new(box_n))
                        .expect("valid insertion box")
                        .expect("insertion box")
                        .list_ptr(&st.mem)
                        .0;
                    crate::mem::free_node(&mut st.mem, box_n, BOX_NODE_SIZE);
                    let packed =
                        crate::packaging::vpackage_emit_in(st, box_lp, 0, ADDITIONAL, MAX_HALFWORD);
                    st.eqtb.set_b32s1(BOX_BASE + n2 as i32, packed?.idx().0);
                    inner_wait
                } else {
                    // best_ins_ptr != p — advance s to end of list
                    while st.mem.b32s1(s) != TEX_NULL {
                        s = st.mem.b32s1(s);
                    }
                    st.mem.set_b32s1(r + 2, s); // PAGE_INS_NODE_last_ins_ptr = s
                    false
                }
            };

            // 1057: Append to hold list or delete node p
            st.mem.set_b32s1(prev_p, ins.link(&st.mem).0); // unlink p
            ins.set_link(&mut st.mem, MemPtr::NULL);
            if wait {
                st.mem.set_b32s1(hold_q, p);
                hold_q = p;
                st.shipout.insert_penalties += 1;
            } else {
                let split_top_ptr = ins.split_top_ptr(&st.mem).0;
                crate::mem::delete_glue_ref(&mut st.mem, split_top_ptr); // INSERTION_NODE_split_top_ptr
                crate::mem::free_node(&mut st.mem, p, INS_NODE_SIZE);
            }
            true // reset p to prev_p
        } else {
            if let AnyNode::Mark(mark_node) = node {
                if mark_node.mark_class(&st.mem) != 0 {
                    // MARK_NODE_class != 0
                    // 1618: Update current marks (extended)
                    let mark_class = mark_node.mark_class(&st.mem);
                    let r = crate::save::find_sa_element(
                        &mut st.mem,
                        &mut st.scanner,
                        MARK_VAL as u16,
                        mark_class,
                        true,
                    );
                    if r == crate::mem::ALLOC_FAILED {
                        return Err(PageBuildAbort::MainMemoryOverflow);
                    }
                    let sa = NodePtr::<SaNode>::from_ptr(MemPtr::from_raw(st.scanner.cur_ptr.0))
                        .expect("find_sa_element returned a mark-class element");
                    if sa.child(&st.mem, 1).is_null() {
                        // ETEX_MARK_sa_first_mark == TEX_NULL
                        let mp = mark_node.mark_ptr(&st.mem);
                        sa.set_child(&mut st.mem, 1, mp);
                        let mark = mp.0;
                        st.mem.set_b32s0(mark, st.mem.b32s0(mark) + 1);
                        // TOKEN_LIST_ref_count++
                    }
                    if !sa.child(&st.mem, 2).is_null() {
                        // ETEX_MARK_sa_bot_mark != TEX_NULL
                        let old_bot = sa.child(&st.mem, 2).0;
                        crate::mem::delete_token_ref(&mut st.mem, old_bot);
                    }
                    let mp = mark_node.mark_ptr(&st.mem);
                    sa.set_child(&mut st.mem, 2, mp);
                    let mark = mark_node.mark_ptr(&st.mem).0;
                    st.mem.set_b32s0(mark, st.mem.b32s0(mark) + 1);
                // TOKEN_LIST_ref_count++
                } else {
                    // 1051: Update first_mark and bot_mark (default)
                    if st.scanner.cur_mark[FIRST_MARK_CODE] == TEX_NULL {
                        st.scanner.cur_mark[FIRST_MARK_CODE] = mark_node.mark_ptr(&st.mem).0;
                        let first = st.scanner.cur_mark[FIRST_MARK_CODE];
                        st.mem.set_b32s0(first, st.mem.b32s0(first) + 1);
                    }
                    if st.scanner.cur_mark[BOT_MARK_CODE] != TEX_NULL {
                        crate::mem::delete_token_ref(
                            &mut st.mem,
                            st.scanner.cur_mark[BOT_MARK_CODE],
                        );
                    }
                    st.scanner.cur_mark[BOT_MARK_CODE] = mark_node.mark_ptr(&st.mem).0;
                    let bot = st.scanner.cur_mark[BOT_MARK_CODE];
                    st.mem.set_b32s0(bot, st.mem.b32s0(bot) + 1);
                }
            }
            false
        };

        if !reset_p {
            prev_p = p;
        }
        p = st.mem.b32s1(prev_p);
    }

    st.eqtb
        .set_b32s1(GLUE_BASE + GLUE_PAR__split_top_skip, save_split_top_skip);

    // 1052: Break page at p, put into box 255, put rest on contribution list
    if p != TEX_NULL {
        if st.mem.b32s1(CONTRIB_HEAD) == TEX_NULL {
            if st.input.nest_ptr == 0 {
                st.input.cur_list.tail.0 = st.shipout.page_tail.0;
            } else {
                st.input.nest[0].tail.0 = st.shipout.page_tail.0;
            }
        }
        st.mem
            .set_b32s1(st.shipout.page_tail.0, st.mem.b32s1(CONTRIB_HEAD));
        st.mem.set_b32s1(CONTRIB_HEAD, p);
        st.mem.set_b32s1(prev_p, TEX_NULL);
    }

    // Temporarily suppress badness errors
    let save_vbadness = st.eqtb.b32s1(INT_BASE + INT_PAR__vbadness);
    st.eqtb.set_b32s1(INT_BASE + INT_PAR__vbadness, INF_BAD);
    let save_vfuzz = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR__vfuzz);
    st.eqtb
        .set_b32s1(DIMEN_BASE + DIMEN_PAR__vfuzz, MAX_HALFWORD);
    let packed_255 = crate::packaging::vpackage_emit_in(
        st,
        st.mem.b32s1(PAGE_HEAD),
        st.shipout.best_size,
        EXACTLY,
        st.shipout.page_max_depth,
    )?;
    st.eqtb.set_b32s1(BOX_BASE + 255, packed_255.idx().0);
    st.eqtb
        .set_b32s1(INT_BASE + INT_PAR__vbadness, save_vbadness);
    st.eqtb.set_b32s1(DIMEN_BASE + DIMEN_PAR__vfuzz, save_vfuzz);

    if st.shipout.last_glue.0 != MAX_HALFWORD {
        crate::mem::delete_glue_ref(&mut st.mem, st.shipout.last_glue.0);
    }

    // 1026: Start a new current page
    st.shipout.page_contents = EMPTY;
    st.shipout.page_tail.0 = PAGE_HEAD;
    st.mem.set_b32s1(PAGE_HEAD, TEX_NULL);
    st.shipout.last_glue.0 = MAX_HALFWORD;
    st.shipout.last_penalty = 0;
    st.shipout.last_kern = 0;
    st.shipout.last_node_type = -1;
    st.shipout.page_so_far[7] = 0;
    st.shipout.page_max_depth = 0;

    if hold_q != HOLD_HEAD {
        st.mem.set_b32s1(PAGE_HEAD, st.mem.b32s1(HOLD_HEAD));
        st.shipout.page_tail.0 = hold_q;
    }

    // 1054: Delete page-insertion nodes
    let mut r = st.mem.b32s1(PAGE_INS_HEAD);
    while r != PAGE_INS_HEAD {
        let next = st.mem.b32s1(r);
        crate::mem::free_node(&mut st.mem, r, PAGE_INS_NODE_SIZE);
        r = next;
    }
    st.mem.set_b32s1(PAGE_INS_HEAD, PAGE_INS_HEAD);

    // Handle extended marks after new-page setup
    let mark_root = st.scanner.sa_root[MARK_VAL];
    if mark_root != TEX_NULL
        && crate::packaging::do_marks(&mut st.mem, &mut st.scanner, FIRE_UP_DONE, 0, mark_root)
    {
        st.scanner.sa_root[MARK_VAL] = TEX_NULL;
    }

    if st.scanner.cur_mark[TOP_MARK_CODE] != TEX_NULL
        && st.scanner.cur_mark[FIRST_MARK_CODE] == TEX_NULL
    {
        st.scanner.cur_mark[FIRST_MARK_CODE] = st.scanner.cur_mark[TOP_MARK_CODE];
        let top = st.scanner.cur_mark[TOP_MARK_CODE];
        st.mem.set_b32s0(top, st.mem.b32s0(top) + 1);
    }

    if st.eqtb.b32s1(LOCAL_BASE + LOCAL__output_routine) != TEX_NULL {
        if st.shipout.dead_cycles >= st.eqtb.b32s1(INT_BASE + INT_PAR__max_dead_cycles) {
            // 1059: Too many dead cycles
            crate::output::error_here_with_diagnostic_in(st, c"Output loop---".as_ptr());
            crate::output::print_int_in(st, st.shipout.dead_cycles);
            crate::output::print_cstr_in(st, c" consecutive dead cycles".as_ptr());
            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
            st.errors.help_ptr = 3;
            st.errors.help_line[2] =
                c"I've concluded that your \\output is awry; it never does a".as_ptr();
            st.errors.help_line[1] =
                c"\\shipout, so I'm shipping \\box255 out myself. Next time".as_ptr();
            st.errors.help_line[0] =
                c"increase \\maxdeadcycles if you want me to be more patient!".as_ptr();
            page_error(crate::errors::error_in(st))?;
        } else {
            // 1060: Fire up the user's output routine
            st.shipout.output_active = true;
            st.shipout.dead_cycles += 1;
            push_nest_pagebuilder_in(st)?;
            st.input.cur_list.mode = -VMODE;
            st.input.cur_list.set_aux_s1(IGNORE_DEPTH); // prev_depth
            st.input.cur_list.mode_line = -st.mem.line;
            crate::input::begin_token_list_in(
                st,
                st.eqtb.b32s1(LOCAL_BASE + LOCAL__output_routine),
                OUTPUT_TEXT,
            )?;
            page_save(crate::save::new_save_level(
                &mut st.save_stack,
                GroupCode::Output,
                st.mem.line,
            ))?;
            page_save(crate::main_control::normal_paragraph(
                &mut st.eqtb,
                &mut st.save_stack,
                &mut st.mem,
                &mut st.scanner,
            ))?;
            crate::scan::scan_left_brace_in(st)?;
            return Ok(());
        }
    }

    // 1058: Default output routine
    if st.mem.b32s1(PAGE_HEAD) != TEX_NULL {
        if st.mem.b32s1(CONTRIB_HEAD) == TEX_NULL {
            if st.input.nest_ptr == 0 {
                st.input.cur_list.tail.0 = st.shipout.page_tail.0;
            } else {
                st.input.nest[0].tail.0 = st.shipout.page_tail.0;
            }
        } else {
            st.mem
                .set_b32s1(st.shipout.page_tail.0, st.mem.b32s1(CONTRIB_HEAD));
        }
        st.mem.set_b32s1(CONTRIB_HEAD, st.mem.b32s1(PAGE_HEAD));
        st.mem.set_b32s1(PAGE_HEAD, TEX_NULL);
        st.shipout.page_tail.0 = PAGE_HEAD;
    }

    crate::packaging::flush_node_list(&mut st.mem, st.scanner.disc_ptr[LAST_BOX_CODE])
        .map_err(PageBuildAbort::Confusion)?;
    st.scanner.disc_ptr[LAST_BOX_CODE] = TEX_NULL;
    unsafe { crate::shipout::ship_out_in(st, st.eqtb.b32s1(BOX_BASE + 255))? };
    st.eqtb.set_b32s1(BOX_BASE + 255, TEX_NULL);
    Ok(())
}

fn update_last_node_info(st: &mut EngineState, node: AnyNode, nt: i32) {
    st.shipout.last_penalty = 0;
    st.shipout.last_kern = 0;
    st.shipout.last_node_type = nt + 1;

    match node {
        AnyNode::Glue(glue) => {
            let spec = glue.glue_spec(&st.mem).expect("non-null glue spec");
            st.shipout.last_glue.0 = spec.idx().0;
            let ref_count = spec.ref_count(&st.mem);
            spec.set_ref_count(&mut st.mem, ref_count + 1);
        }
        AnyNode::Penalty(penalty) => {
            st.shipout.last_glue.0 = MAX_HALFWORD;
            st.shipout.last_penalty = penalty.penalty(&st.mem);
        }
        AnyNode::Kern(kern) => {
            st.shipout.last_glue.0 = MAX_HALFWORD;
            st.shipout.last_kern = kern.width(&st.mem).0;
        }
        _ => {
            st.shipout.last_glue.0 = MAX_HALFWORD;
        }
    }
}

pub(crate) fn build_page_in(st: &mut EngineState) -> Result<(), PageBuildAbort> {
    if st.mem.b32s1(CONTRIB_HEAD) == TEX_NULL || st.shipout.output_active {
        return Ok(());
    }

    'outer: loop {
        let p = st.mem.b32s1(CONTRIB_HEAD);
        let nt = st.mem.b16s1(p) as i32;
        let node = st.mem.classify(MemIdx::new(p)).expect("valid page node");

        // 1031: Update last_glue, last_penalty, last_kern, last_node_type
        if st.shipout.last_glue.0 != MAX_HALFWORD {
            crate::mem::delete_glue_ref(&mut st.mem, st.shipout.last_glue.0);
        }
        update_last_node_info(st, node, nt);

        // 1032/1035: Dispatch on node type
        let mut pi: i32 = INF_PENALTY;
        let mut need_contribute = false;
        let mut need_update_heights = false;
        let mut need_done1 = false;
        let mut page_glue = None;
        let mut page_kern = None;

        match node {
            AnyNode::HList(_) | AnyNode::VList(_) | AnyNode::Rule(_) => {
                let (height, depth) = match node {
                    AnyNode::HList(node) => (node.height(&st.mem).0, node.depth(&st.mem).0),
                    AnyNode::VList(node) => (node.height(&st.mem).0, node.depth(&st.mem).0),
                    AnyNode::Rule(node) => (node.height(&st.mem).raw(), node.depth(&st.mem).raw()),
                    _ => unreachable!(),
                };
                if st.shipout.page_contents < BOX_THERE {
                    // 1036: Initialize page, insert \topskip, continue loop
                    if st.shipout.page_contents == EMPTY {
                        freeze_page_specs(&st.eqtb, &mut st.shipout, BOX_THERE);
                    } else {
                        st.shipout.page_contents = BOX_THERE;
                    }
                    let q = page_new_skip_param(&mut st.mem, &st.eqtb, GLUE_PAR__top_skip as i16)?;
                    let top_skip_spec =
                        NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(st.mem.temp_ptr.0))
                            .expect("non-null top skip spec");
                    let top_skip_width = top_skip_spec.width(&st.mem).0;
                    if top_skip_width > height {
                        // BOX_width(temp_ptr) > BOX_height(p)
                        top_skip_spec
                            .set_width(&mut st.mem, crate::units::Scaled(top_skip_width - height));
                    } else {
                        top_skip_spec.set_width(&mut st.mem, crate::units::Scaled(0));
                    }
                    let AnyNode::Glue(top_skip_node) = st
                        .mem
                        .classify(MemIdx::new(q))
                        .expect("valid top skip node")
                    else {
                        unreachable!("top skip is glue")
                    };
                    top_skip_node.set_link(&mut st.mem, MemPtr::from_raw(p));
                    st.mem.set_b32s1(CONTRIB_HEAD, q);
                    if st.mem.b32s1(CONTRIB_HEAD) == TEX_NULL {
                        break 'outer;
                    }
                    continue 'outer;
                } else {
                    // 1037: Move box/rule to current page
                    st.shipout.page_so_far[1] += st.shipout.page_so_far[7] + height;
                    st.shipout.page_so_far[7] = depth;
                    need_contribute = true;
                }
            }
            AnyNode::Whatsit(whatsit) => {
                // 1401
                let sub = whatsit.whatsit_subtype(&st.mem);
                if sub == Some(WhatsitSubtype::Pic) || sub == Some(WhatsitSubtype::Pdf) {
                    st.shipout.page_so_far[1] +=
                        st.shipout.page_so_far[7] + whatsit.height(&st.mem).0;
                    st.shipout.page_so_far[7] = whatsit.depth(&st.mem).0;
                }
                need_contribute = true;
            }
            AnyNode::Glue(glue) => {
                page_glue = Some(glue);
                if st.shipout.page_contents < BOX_THERE {
                    need_done1 = true;
                } else if (st.mem.b16s1(st.shipout.page_tail.0) as i32) < MATH_NODE {
                    // is_non_discardable_node(page_tail)
                    pi = 0;
                } else {
                    need_update_heights = true;
                }
            }
            AnyNode::Kern(kern) => {
                page_kern = Some(kern);
                if st.shipout.page_contents < BOX_THERE {
                    need_done1 = true;
                } else if kern.link(&st.mem) == MemPtr::NULL {
                    // LLIST_link(p) == TEX_NULL — kern at list end; wait
                    return Ok(());
                } else if matches!(
                    st.mem
                        .classify(kern.link(&st.mem).as_idx().expect("non-null kern link")),
                    Ok(AnyNode::Glue(_))
                ) {
                    pi = 0;
                } else {
                    need_update_heights = true;
                }
            }
            AnyNode::Penalty(penalty) => {
                if st.shipout.page_contents < BOX_THERE {
                    need_done1 = true;
                } else {
                    pi = penalty.penalty(&st.mem);
                }
            }
            AnyNode::Mark(_) => {
                need_contribute = true;
            }
            AnyNode::Ins(ins) => {
                // 1043: Append insertion to current page
                if st.shipout.page_contents == EMPTY {
                    freeze_page_specs(&st.eqtb, &mut st.shipout, INSERTS_ONLY);
                }
                let n = ins.subtype(&st.mem) as u8;
                let mut r = PAGE_INS_HEAD;
                while n as u16 >= st.mem.b16s0(st.mem.b32s1(r)) {
                    r = st.mem.b32s1(r);
                }

                if st.mem.b16s0(r) as u8 != n {
                    // 1044: Create page-insertion node
                    let q2 = page_get_node(&mut st.mem, PAGE_INS_NODE_SIZE)?;
                    st.mem.set_b32s1(q2, st.mem.b32s1(r));
                    st.mem.set_b32s1(r, q2);
                    r = q2;
                    st.mem.set_b16s0(r, n as u16); // NODE_subtype = n
                    st.mem.set_b16s1(r, INSERTING); // NODE_type = INSERTING
                    ensure_vbox_in(st, n)?;
                    if st.eqtb.b32s1(BOX_BASE + n as i32) == TEX_NULL {
                        st.mem.set_b32s1(r + 3, 0); // BOX_height(r) = 0
                    } else {
                        let br = st.eqtb.b32s1(BOX_BASE + n as i32);
                        let box_node = st
                            .mem
                            .classify_box(MemIdx::new(br))
                            .expect("valid insertion box")
                            .expect("insertion box");
                        st.mem.set_b32s1(
                            r + 3,
                            box_node.height(&st.mem).0 + box_node.depth(&st.mem).0,
                        );
                    }
                    st.mem.set_b32s0(r + 2, TEX_NULL); // best_ins_ptr = TEX_NULL
                    let q2 = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(
                        st.eqtb.b32s1(SKIP_BASE + n as i32),
                    ))
                    .expect("non-null insertion skip spec");
                    let h = if st.eqtb.b32s1(COUNT_BASE + n as i32) == 1000 {
                        st.mem.b32s1(r + 3)
                    } else {
                        crate::scaled_math::x_over_n_in(&mut st.errors, st.mem.b32s1(r + 3), 1000)
                            * st.eqtb.b32s1(COUNT_BASE + n as i32)
                    };
                    st.shipout.page_so_far[0] -= h + q2.width(&st.mem).0;
                    let stretch_ord = q2.stretch_order(&st.mem) as usize;
                    st.shipout.page_so_far[2 + stretch_ord] += q2.stretch(&st.mem).0;
                    st.shipout.page_so_far[6] += q2.shrink(&st.mem).0;
                    if q2.shrink_order(&st.mem) as i32 != NORMAL && q2.shrink(&st.mem).0 != 0 {
                        crate::output::error_here_with_diagnostic_in(
                            st,
                            c"Infinite glue shrinkage inserted from ".as_ptr(),
                        );
                        crate::output::print_esc_cstr_in(st, c"skip".as_ptr());
                        crate::output::print_int_in(st, n as i32);
                        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                        st.errors.help_ptr = 3;
                        st.errors.help_line[2] =
                            c"The correction glue for page breaking with insertions".as_ptr();
                        st.errors.help_line[1] =
                            c"must have finite shrinkability. But you may proceed,".as_ptr();
                        st.errors.help_line[0] =
                            c"since the offensive shrinkability has been made finite.".as_ptr();
                        page_error(crate::errors::error_in(st))?;
                    }
                }

                if st.mem.b16s1(r) as i32 == SPLIT_UP {
                    st.shipout.insert_penalties += ins.float_cost(&st.mem);
                } else {
                    st.mem.set_b32s1(r + 2, p); // PAGE_INS_NODE_last_ins_ptr = p
                    let delta = st.shipout.page_so_far[0]
                        - st.shipout.page_so_far[1]
                        - st.shipout.page_so_far[7]
                        + st.shipout.page_so_far[6];
                    let count_n = st.eqtb.b32s1(COUNT_BASE + n as i32);
                    let h = if count_n == 1000 {
                        ins.height(&st.mem).0 // BOX_height(p)
                    } else {
                        crate::scaled_math::x_over_n_in(&mut st.errors, ins.height(&st.mem).0, 1000)
                            * count_n
                    };

                    if (h <= 0 || h <= delta)
                        && ins.height(&st.mem).0 + st.mem.b32s1(r + 3)
                            <= st.eqtb.b32s1(SCALED_BASE + n as i32)
                    {
                        st.shipout.page_so_far[0] -= h;
                        st.mem
                            .set_b32s1(r + 3, st.mem.b32s1(r + 3) + ins.height(&st.mem).0);
                    } else {
                        // 1045: Split the insertion
                        let w = if count_n <= 0 {
                            MAX_HALFWORD
                        } else {
                            let mut w = st.shipout.page_so_far[0]
                                - st.shipout.page_so_far[1]
                                - st.shipout.page_so_far[7];
                            if count_n != 1000 {
                                w = crate::scaled_math::x_over_n_in(&mut st.errors, w, count_n)
                                    * 1000;
                            }
                            w
                        };
                        let max_h = st.eqtb.b32s1(SCALED_BASE + n as i32) - st.mem.b32s1(r + 3);
                        let w = if w > max_h { max_h } else { w };
                        let q2 = crate::packaging::vert_break_in(
                            st,
                            ins.ins_ptr(&st.mem).0,
                            w,
                            ins.depth(&st.mem).0,
                        )?;
                        // BOX_height(r) += best_height_plus_depth
                        st.mem.set_b32s1(
                            r + 3,
                            st.mem.b32s1(r + 3) + st.scanner.best_height_plus_depth,
                        );
                        if count_n != 1000 {
                            st.scanner.best_height_plus_depth = crate::scaled_math::x_over_n_in(
                                &mut st.errors,
                                st.scanner.best_height_plus_depth,
                                1000,
                            ) * count_n;
                        }
                        st.shipout.page_so_far[0] -= st.scanner.best_height_plus_depth;
                        st.mem.set_b16s1(r, SPLIT_UP as u16); // NODE_type = SPLIT_UP
                        st.mem.set_b32s1(r + 1, q2); // broken_ptr = q
                        st.mem.set_b32s0(r + 1, p); // broken_ins = p
                        if q2 == TEX_NULL {
                            st.shipout.insert_penalties += EJECT_PENALTY;
                        } else if let Some(penalty) = st
                            .mem
                            .classify_penalty(MemIdx::new(q2))
                            .expect("valid insertion break node")
                        {
                            st.shipout.insert_penalties += penalty.penalty(&st.mem);
                        }
                    }
                }
                need_contribute = true;
            }
            _ => return Err(PageBuildAbort::Confusion(c"page")),
        }

        // 1040: Break check — reached for GLUE/KERN/PENALTY that didn't goto contribute/done1
        if !need_contribute && !need_update_heights && !need_done1 {
            if pi < INF_PENALTY {
                // 1042: Compute badness b of current page
                let b = if st.shipout.page_so_far[1] < st.shipout.page_so_far[0] {
                    if st.shipout.page_so_far[3] != 0
                        || st.shipout.page_so_far[4] != 0
                        || st.shipout.page_so_far[5] != 0
                    {
                        0
                    } else {
                        badness(
                            st.shipout.page_so_far[0] - st.shipout.page_so_far[1],
                            st.shipout.page_so_far[2],
                        )
                    }
                } else if st.shipout.page_so_far[1] - st.shipout.page_so_far[0]
                    > st.shipout.page_so_far[6]
                {
                    AWFUL_BAD
                } else {
                    badness(
                        st.shipout.page_so_far[1] - st.shipout.page_so_far[0],
                        st.shipout.page_so_far[6],
                    )
                };

                let mut c = if b < AWFUL_BAD {
                    if pi <= EJECT_PENALTY {
                        pi
                    } else if b < INF_BAD {
                        b + pi + st.shipout.insert_penalties
                    } else {
                        100_000 // DEPLORABLE
                    }
                } else {
                    b
                };

                if st.shipout.insert_penalties >= 10_000 {
                    c = MAX_HALFWORD;
                }

                if c <= st.shipout.least_page_cost {
                    st.shipout.best_page_break.0 = p;
                    st.shipout.best_size = st.shipout.page_so_far[0];
                    st.shipout.least_page_cost = c;
                    let mut r = st.mem.b32s1(PAGE_INS_HEAD);
                    while r != PAGE_INS_HEAD {
                        // best_ins_ptr = last_ins_ptr
                        st.mem.set_b32s0(r + 2, st.mem.b32s1(r + 2));
                        r = st.mem.b32s1(r);
                    }
                }

                if c == AWFUL_BAD || pi <= EJECT_PENALTY {
                    fire_up_in(st, p)?;
                    if st.shipout.output_active {
                        return Ok(());
                    }
                    if st.mem.b32s1(CONTRIB_HEAD) == TEX_NULL {
                        break 'outer;
                    }
                    continue 'outer;
                }
            }

            // After break check: decide contribute vs update_heights
            if page_glue.is_none() && page_kern.is_none() {
                need_contribute = true;
            } else {
                need_update_heights = true;
            }
        }

        // 1039: update_heights — update page measurements for glue/kern
        if need_update_heights {
            let width = if let Some(kern) = page_kern {
                kern.width(&st.mem).0
            } else {
                let glue = page_glue.expect("height update is glue or kern");
                let mut spec = glue.glue_spec(&st.mem).expect("non-null page glue spec");
                let stretch_ord = spec.stretch_order(&st.mem) as usize;
                st.shipout.page_so_far[2 + stretch_ord] += spec.stretch(&st.mem).0;
                st.shipout.page_so_far[6] += spec.shrink(&st.mem).0;
                if spec.shrink_order(&st.mem) as i32 != NORMAL && spec.shrink(&st.mem).0 != 0 {
                    crate::output::error_here_with_diagnostic_in(
                        st,
                        c"Infinite glue shrinkage found on current page".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 4;
                    st.errors.help_line[3] =
                        c"The page about to be output contains some infinitely".as_ptr();
                    st.errors.help_line[2] =
                        c"shrinkable glue, e.g., `\\vss' or `\\vskip 0pt minus 1fil'.".as_ptr();
                    st.errors.help_line[1] =
                        c"Such glue doesn't belong there; but you can safely proceed,".as_ptr();
                    st.errors.help_line[0] =
                        c"since the offensive shrinkability has been made finite.".as_ptr();
                    page_error(crate::errors::error_in(st))?;
                    let rr = page_new_spec(&mut st.mem, spec.idx().0)?;
                    let rr = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(rr))
                        .expect("non-null copied glue spec");
                    rr.set_shrink_order(&mut st.mem, NORMAL as u16);
                    crate::mem::delete_glue_ref(&mut st.mem, spec.idx().0);
                    glue.set_glue_ptr(&mut st.mem, MemPtr::from_idx(rr.idx()));
                    spec = rr;
                }
                spec.width(&st.mem).0
            };
            st.shipout.page_so_far[1] += st.shipout.page_so_far[7] + width;
            st.shipout.page_so_far[7] = 0;
            need_contribute = true;
        }

        // 1038/1033: contribute — check depth limit, link p into page, goto done
        if need_contribute {
            if st.shipout.page_so_far[7] > st.shipout.page_max_depth {
                st.shipout.page_so_far[1] += st.shipout.page_so_far[7] - st.shipout.page_max_depth;
                st.shipout.page_so_far[7] = st.shipout.page_max_depth;
            }
            st.mem.set_b32s1(st.shipout.page_tail.0, p);
            st.shipout.page_tail.0 = p;
            st.mem.set_b32s1(CONTRIB_HEAD, st.mem.b32s1(p));
            st.mem.set_b32s1(p, TEX_NULL);
            if st.mem.b32s1(CONTRIB_HEAD) == TEX_NULL {
                break 'outer;
            }
            continue 'outer;
        }

        // 1034: done1 — recycle node p (glue/kern/penalty before any box)
        if need_done1 {
            st.mem.set_b32s1(CONTRIB_HEAD, st.mem.b32s1(p));
            st.mem.set_b32s1(p, TEX_NULL);
            if st.eqtb.b32s1(INT_BASE + INT_PAR__saving_vdiscards) <= 0 {
                crate::packaging::flush_node_list(&mut st.mem, p)
                    .map_err(PageBuildAbort::Confusion)?;
            } else {
                if st.scanner.disc_ptr[LAST_BOX_CODE] == TEX_NULL {
                    st.scanner.disc_ptr[LAST_BOX_CODE] = p;
                } else {
                    st.mem.set_b32s1(st.scanner.disc_ptr[COPY_CODE], p);
                }
                st.scanner.disc_ptr[COPY_CODE] = p;
            }
            if st.mem.b32s1(CONTRIB_HEAD) == TEX_NULL {
                break 'outer;
            }
            continue 'outer;
        }

        // done: re-evaluate while condition
        if st.mem.b32s1(CONTRIB_HEAD) == TEX_NULL {
            break 'outer;
        }
    }

    // After loop: update nest tail to point at contribution list head
    if st.input.nest_ptr == 0 {
        st.input.cur_list.tail.0 = CONTRIB_HEAD;
    } else {
        st.input.nest[0].tail.0 = CONTRIB_HEAD;
    }
    Ok(())
}
