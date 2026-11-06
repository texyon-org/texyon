//! Boxes: `\hbox`, `\vbox`, `\box`, `\lastbox`, `\unhbox`, `\unskip` and friends.

use crate::cmd::Cmd;
use crate::codes::{BoxCode, ValueLevel};
use crate::consts::{
    A_LEADERS, BEGIN_M_CODE, BOX_BASE, BOX_NODE_SIZE, END_M_CODE, HMODE, IGNORE_DEPTH,
    MARGIN_KERN_NODE_SIZE, MMODE, SUB_BOX, VMODE,
};
use crate::errors::Abort;
use crate::input::TokenType;
use crate::mem::{ADJUST_HEAD, PRE_ADJUST_HEAD};
use crate::node::{AnyBox, AnyNode, BoxOrRule};
use crate::params::{DimenPar, IntPar, TokPar};
use crate::scan::fetch_box;
use crate::state::{EngineState, GroupCode, MemState};
use crate::units::{MemPtr, NodeType, Scaled};

use super::assign::{define, sa_define};
use super::lists::{append_glue, normal_paragraph};
use super::{
    append_list, begin_every, BOX_FLAG, GLOBAL_BOX_FLAG, INNER_HMODE, LEADER_FLAG, SHIP_OUT_FLAG,
};

pub(super) fn unpackage(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.cur_chr > BoxCode::Copy.raw() {
        let disc = &mut st.scanner.disc_ptr[st.scanner.cur_chr as usize];
        st.input.cur_list.tail.set_link(&mut st.mem, *disc);
        *disc = MemPtr::NULL;
    } else {
        let c = st.scanner.cur_chr;
        let n = crate::scan::scan_register_num(st)?;
        let Some(b) = fetch_box(st, n) else {
            return Ok(());
        };
        let mode = st.input.cur_list.mode.abs();
        if mode == MMODE
            || (mode == VMODE && !matches!(b, AnyBox::VList(_)))
            || (mode == HMODE && !matches!(b, AnyBox::HList(_)))
        {
            crate::output::print_err(st, "Incompatible list can't be unboxed");
            st.errors.help(&[
                "Sorry, Pandora. (You sneaky devil.)",
                r"I refuse to unbox an \hbox in vertical mode or vice versa.",
                "And I can't open any boxes in math mode.",
            ]);
            return crate::errors::error(st);
        }
        let list = b.list_ptr(&st.mem);
        if c == BoxCode::Copy.raw() {
            let copy = st.mem.copy_node_list(list)?;
            st.input.cur_list.tail.set_link(&mut st.mem, copy);
        } else {
            st.input.cur_list.tail.set_link(&mut st.mem, list);
            st.mem
                .change_box(&mut st.scanner, &mut st.eqtb, n, MemPtr::NULL);
            st.mem.free_node(b.ptr(), BOX_NODE_SIZE);
        }
    }

    loop {
        let r = st.input.cur_list.tail.link(&st.mem);
        if r.is_null() {
            return Ok(());
        }
        if !st.mem.is_char_node(r) && matches!(st.mem.classify(r), Ok(AnyNode::MarginKern(_))) {
            let next = r.link(&st.mem);
            st.input.cur_list.tail.set_link(&mut st.mem, next);
            st.mem.free_node(r, MARGIN_KERN_NODE_SIZE);
        } else {
            st.input.cur_list.tail = r;
        }
    }
}

pub(super) fn delete_last(st: &mut EngineState) -> Result<(), Abort> {
    if st.input.cur_list.mode == VMODE && st.input.cur_list.tail == st.input.cur_list.head {
        if st.scanner.cur_chr as u16 != NodeType::Glue.as_raw() || st.shipout.last_glue.is_some() {
            crate::errors::you_cant(st);
            let remedy = if st.scanner.cur_chr as u16 == NodeType::Kern.as_raw() {
                r"Try `I\kern-\lastkern' instead."
            } else if st.scanner.cur_chr as u16 != NodeType::Glue.as_raw() {
                "Perhaps you can make the output routine do it."
            } else {
                r"Try `I\vskip-\lastskip' instead."
            };
            st.errors.help(&[
                "Sorry...I usually can't take things from the current page.",
                remedy,
            ]);
            return crate::errors::error(st);
        }
        return Ok(());
    }
    let tx = effective_tail(st);
    if st.mem.is_char_node(tx)
        || st
            .mem
            .node_type(tx)
            .map_or(true, |t| i32::from(t.as_raw()) != st.scanner.cur_chr)
    {
        return Ok(());
    }
    if unlink_effective_tail(st, tx)? {
        st.mem.flush_node_list(tx)?;
    }
    Ok(())
}

pub(super) fn scan_box(st: &mut EngineState, box_context: i32) -> Result<(), Abort> {
    crate::scan::get_x_non_blank_non_relax(st)?;
    if st.scanner.cur_cmd == Cmd::MakeBox {
        begin_box(st, box_context)
    } else if box_context >= LEADER_FLAG
        && (st.scanner.cur_cmd == Cmd::Hrule || st.scanner.cur_cmd == Cmd::Vrule)
    {
        st.scanner.cur_box = Some(BoxOrRule::Rule(crate::scan::scan_rule_spec(st)?));
        box_end(st, box_context)
    } else {
        crate::output::print_err(st, "A <box> was supposed to be here");
        st.errors.help(&[
            r"I was expecting to see \hbox or \vbox or \copy or \box or",
            "something like that. So you might find something missing in",
            "your output. But keep trying; you can fix this later.",
        ]);
        crate::input::back_error(st)
    }
}

pub(super) fn package(st: &mut EngineState, vtop: bool) -> Result<(), Abort> {
    let d = st.eqtb.dimen_par(DimenPar::BoxMaxDepth);
    let u = st.eqtb.int_par(IntPar::XetexUpwards);
    crate::save::unsave(st)?;
    let spec = st.save_stack.pop_spec()?;
    st.save_stack.save_ptr -= 1;
    let v = st.eqtb.int_par(IntPar::XetexUpwards);
    st.eqtb.set_int_par(IntPar::XetexUpwards, u);
    let list = st.input.cur_list.head.link(&st.mem);
    if st.input.cur_list.mode == INNER_HMODE {
        let b = crate::packaging::hpack(st, list, spec)?;
        st.scanner.cur_box = Some(BoxOrRule::Box(b));
    } else {
        let b = crate::packaging::vpackage(st, list, spec, d)?;
        st.scanner.cur_box = Some(BoxOrRule::Box(b));
        if vtop {
            let h = match b.list_ptr(&st.mem).as_idx().map(|p| st.mem.classify(p)) {
                Some(Ok(AnyNode::HList(first))) => first.height(&st.mem),
                Some(Ok(AnyNode::VList(first))) => first.height(&st.mem),
                Some(Ok(AnyNode::Rule(first))) => first.height(&st.mem).to_scaled(),
                _ => Scaled::ZERO,
            };
            let depth = b.depth(&st.mem) - h + b.height(&st.mem);
            b.set_depth(&mut st.mem, depth);
            b.set_height(&mut st.mem, h);
        }
    }
    st.eqtb.set_int_par(IntPar::XetexUpwards, v);
    st.mem.pop_nest(&mut st.input)?;
    let box_context = st.save_stack.saved(0);
    box_end(st, box_context)
}

fn box_end(st: &mut EngineState, box_context: i32) -> Result<(), Abort> {
    if box_context < BOX_FLAG {
        let b = match st.scanner.cur_box {
            None => return Ok(()),
            Some(BoxOrRule::Box(b)) => b,
            Some(BoxOrRule::Rule(_)) => unreachable!("only leaders take a rule"),
        };
        b.set_shift_amount(&mut st.mem, Scaled::from_raw(box_context));
        if st.input.cur_list.mode.abs() == VMODE {
            if !st.scanner.pre_adjust_tail.is_null() {
                if st.scanner.pre_adjust_tail != PRE_ADJUST_HEAD {
                    append_list(st, PRE_ADJUST_HEAD, st.scanner.pre_adjust_tail);
                }
                st.scanner.pre_adjust_tail = MemPtr::NULL;
            }
            st.input
                .cur_list
                .append_to_vlist(&mut st.mem, &st.eqtb, b)?;
            if !st.scanner.adjust_tail.is_null() {
                if st.scanner.adjust_tail != ADJUST_HEAD {
                    append_list(st, ADJUST_HEAD, st.scanner.adjust_tail);
                }
                st.scanner.adjust_tail = MemPtr::NULL;
            }
            if st.input.cur_list.mode > 0 {
                crate::pagebuilder::build_page(st)?;
            }
        } else {
            let node = if st.input.cur_list.mode.abs() == HMODE {
                st.input.cur_list.set_space_factor(1000);
                b.ptr()
            } else {
                let noad = st.mem.new_noad()?;
                noad.nucleus().set_math_type(&mut st.mem, SUB_BOX);
                noad.nucleus().set_info(&mut st.mem, b.ptr());
                noad.ptr()
            };
            st.input.cur_list.tail_append(&mut st.mem, node);
        }
    } else if box_context < SHIP_OUT_FLAG {
        let (n, global) = if box_context < GLOBAL_BOX_FLAG {
            (box_context - BOX_FLAG, false)
        } else {
            (box_context - GLOBAL_BOX_FLAG, true)
        };
        let cb = st.scanner.cur_box.map_or(MemPtr::NULL, |b| b.ptr());
        if n < 256 {
            define(st, global, BOX_BASE + n, Cmd::BoxRef, cb.raw())?;
        } else {
            let sa = st
                .mem
                .create_sa_element(&mut st.scanner, ValueLevel::BOX, n)?;
            sa_define(st, global, sa, cb)?;
        }
    } else if let Some(cb) = st.scanner.cur_box {
        if box_context == SHIP_OUT_FLAG {
            let BoxOrRule::Box(b) = cb else {
                unreachable!("only leaders take a rule");
            };
            crate::shipout::ship_out(st, b)?;
            return Ok(());
        }
        crate::scan::get_x_non_blank_non_relax(st)?;
        let vertical = st.input.cur_list.mode.abs() == VMODE;
        if (st.scanner.cur_cmd == Cmd::Hskip && !vertical)
            || (st.scanner.cur_cmd == Cmd::Vskip && vertical)
        {
            let g = append_glue(st)?;
            g.set_subtype(
                &mut st.mem,
                (box_context - (LEADER_FLAG - i32::from(A_LEADERS))) as u16,
            );
            g.set_leader_ptr(&mut st.mem, cb.ptr());
        } else {
            crate::output::print_err(st, "Leaders not followed by proper glue");
            st.errors.help(&[
                r"You should say `\leaders <box or rule><hskip or vskip>'.",
                "I found the <box or rule>, but there's no suitable",
                "<hskip or vskip>, so I'm ignoring these leaders.",
            ]);
            crate::input::back_error(st)?;
            st.mem.flush_node_list(cb.ptr())?;
        }
    }
    Ok(())
}

impl MemState {
    fn is_math_node(&self, p: MemPtr, subtype: u16) -> bool {
        !self.is_char_node(p)
            && matches!(self.classify(p), Ok(AnyNode::Math(m)) if m.subtype(self) == subtype)
    }
}

/// The last node of the current list, skipping a trailing end-of-math node.
fn effective_tail(st: &EngineState) -> MemPtr {
    let tail = st.input.cur_list.tail;
    if !st.mem.is_math_node(tail, END_M_CODE) {
        return tail;
    }
    st.mem.predecessor(st.input.cur_list.head, tail)
}

/// Unlinks node `tx` from the current list, unless it hides in a
/// discretionary's replacement text. Returns whether it was unlinked.
fn unlink_effective_tail(st: &mut EngineState, tx: MemPtr) -> Result<bool, Abort> {
    let mut q = st.input.cur_list.head;
    let mut p = MemPtr::NULL;
    let mut r;
    let mut fm;
    loop {
        r = p;
        p = q;
        fm = false;
        if !st.mem.is_char_node(q) {
            match st.mem.classify(q) {
                Ok(AnyNode::Disc(disc)) => {
                    for _ in 0..disc.subtype(&st.mem) {
                        p = p.link(&st.mem);
                    }
                    if p == tx {
                        return Ok(false);
                    }
                }
                Ok(AnyNode::Math(m)) => fm = m.subtype(&st.mem) == BEGIN_M_CODE,
                _ => {}
            }
        }
        q = p.link(&st.mem);
        if q == tx {
            break;
        }
    }
    let next = tx.link(&st.mem);
    p.set_link(&mut st.mem, next);
    tx.set_link(&mut st.mem, MemPtr::NULL);
    if next.is_null() {
        if fm {
            return Err(Abort::Confusion("tail1"));
        }
        st.input.cur_list.tail = p;
    } else if fm {
        st.input.cur_list.tail = r;
        r.set_link(&mut st.mem, MemPtr::NULL);
        st.mem.flush_node_list(p)?;
    }
    Ok(true)
}

pub(super) fn begin_box(st: &mut EngineState, box_context: i32) -> Result<(), Abort> {
    let code = st
        .scanner
        .chr_code::<BoxCode>()
        .ok_or(Abort::Confusion("begin_box"))?;
    match code {
        BoxCode::Box => {
            let n = crate::scan::scan_register_num(st)?;
            st.scanner.cur_box = fetch_box(st, n).map(BoxOrRule::Box);
            st.mem
                .change_box(&mut st.scanner, &mut st.eqtb, n, MemPtr::NULL);
        }
        BoxCode::Copy => {
            let n = crate::scan::scan_register_num(st)?;
            st.scanner.cur_box = match fetch_box(st, n) {
                Some(b) => Some(BoxOrRule::Box(st.mem.copy_box(b)?)),
                None => None,
            };
        }
        BoxCode::LastBox => {
            st.scanner.cur_box = None;
            if st.input.cur_list.mode.abs() == MMODE {
                crate::errors::you_cant(st);
                st.errors.help(&[r"Sorry; this \lastbox will be void."]);
                crate::errors::error(st)?;
            } else if st.input.cur_list.mode == VMODE
                && st.input.cur_list.head == st.input.cur_list.tail
            {
                crate::errors::you_cant(st);
                st.errors.help(&[
                    "Sorry...I usually can't take things from the current page.",
                    r"This \lastbox will therefore be void.",
                ]);
                crate::errors::error(st)?;
            } else {
                let tx = effective_tail(st);
                if !st.mem.is_char_node(tx) {
                    if let Ok(Some(b)) = st.mem.classify_box(tx) {
                        if unlink_effective_tail(st, tx)? {
                            st.scanner.cur_box = Some(BoxOrRule::Box(b));
                            b.set_shift_amount(&mut st.mem, Scaled::ZERO);
                        }
                    }
                }
            }
        }
        BoxCode::Vsplit => {
            let n = crate::scan::scan_register_num(st)?;
            if !crate::scan::scan_keyword(st, "to")? {
                crate::output::print_err(st, "Missing `to' inserted");
                st.errors.help(&[
                    r"I'm working on `\vsplit<box number> to <dimen>';",
                    "will look for the <dimen> next.",
                ]);
                crate::errors::error(st)?;
            }
            let d = crate::scan::scan_dimen(st, false, false, false)?;
            st.scanner.cur_box = crate::packaging::vsplit(st, n, d)?.map(BoxOrRule::Box);
        }
        BoxCode::Vtop | BoxCode::Vbox | BoxCode::Hbox => {
            st.save_stack.set_saved(0, box_context);
            let mode = if code == BoxCode::Hbox {
                let group = if box_context < BOX_FLAG && st.input.cur_list.mode.abs() == VMODE {
                    GroupCode::AdjustedHbox
                } else {
                    GroupCode::Hbox
                };
                crate::scan::scan_spec(st, group, true)?;
                HMODE
            } else {
                let group = if code == BoxCode::Vbox {
                    GroupCode::Vbox
                } else {
                    GroupCode::Vtop
                };
                crate::scan::scan_spec(st, group, true)?;
                normal_paragraph(st)?;
                VMODE
            };
            crate::mem::push_nest(st)?;
            st.input.cur_list.mode = -mode;
            if mode == VMODE {
                st.input.cur_list.set_prev_depth(IGNORE_DEPTH);
                begin_every(st, TokPar::EveryVbox, TokenType::EveryVbox)?;
            } else {
                st.input.cur_list.set_space_factor(1000);
                begin_every(st, TokPar::EveryHbox, TokenType::EveryHbox)?;
            }
            return Ok(());
        }
    }
    box_end(st, box_context)
}
