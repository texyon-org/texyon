//! Boxes: `\hbox`, `\vbox`, `\box`, `\lastbox`, `\unhbox`, `\unskip` and friends.

use crate::cmd::Cmd;
use crate::codes::{BoxCode, LeaderCode, ValueLevel};
use crate::command::Command;
use crate::consts::{
    BOX_BASE, BOX_NODE_SIZE, HMODE, IGNORE_DEPTH, MARGIN_KERN_NODE_SIZE, MMODE, SUB_BOX, VMODE,
};
use crate::errors::Abort;
use crate::input::TokenType;
use crate::mem::{ADJUST_HEAD, PRE_ADJUST_HEAD};
use crate::node::{AnyNode, BoxOrRule, GlueKind, MathCode, SaLeaf};
use crate::params::{DimenPar, IntPar, TokPar};
use crate::scan::fetch_box;
use crate::state::{EngineState, GroupCode, MemState};
use crate::units::{MemPtr, NodeType, Scaled};

use super::assign::{define, sa_define};
use super::lists::{append_glue, normal_paragraph};
use super::{
    append_list, begin_every, BOX_FLAG, GLOBAL_BOX_FLAG, INNER_HMODE, LEADER_FLAG, SHIP_OUT_FLAG,
};

pub(super) fn unpackage(st: &mut EngineState, c: BoxCode) -> Result<(), Abort> {
    if matches!(c, BoxCode::LastBox | BoxCode::Vsplit) {
        let disc = &mut st.scanner.disc_ptr[c as usize];
        st.input.cur_list.tail.set_link(&mut st.mem, *disc);
        *disc = MemPtr::NULL;
    } else {
        let n = crate::scan::scan_register_num(st)?;
        let Some(b) = fetch_box(st, n) else {
            return Ok(());
        };
        let mode = st.input.cur_list.mode.abs();
        if mode == MMODE
            || (mode == VMODE && !st.mem[b].is_vlist())
            || (mode == HMODE && !st.mem[b].is_hlist())
        {
            return crate::errors::error(
                st,
                "Incompatible list can't be unboxed",
                &[
                    "Sorry, Pandora. (You sneaky devil.)",
                    r"I refuse to unbox an \hbox in vertical mode or vice versa.",
                    "And I can't open any boxes in math mode.",
                ],
            );
        }
        let list = st.mem[b].list_ptr;
        if c == BoxCode::Copy {
            let copy = st.mem.copy_node_list(list)?;
            st.input.cur_list.tail.set_link(&mut st.mem, copy);
        } else {
            st.input.cur_list.tail.set_link(&mut st.mem, list);
            st.mem
                .change_box(&mut st.scanner, &mut st.eqtb, n, MemPtr::NULL)?;
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

pub(super) fn delete_last(st: &mut EngineState, t: NodeType) -> Result<(), Abort> {
    if st.input.cur_list.mode == VMODE && st.input.cur_list.tail == st.input.cur_list.head {
        if t != NodeType::Glue || st.shipout.last_glue.is_some() {
            let remedy = if t == NodeType::Kern {
                r"Try `I\kern-\lastkern' instead."
            } else if t != NodeType::Glue {
                "Perhaps you can make the output routine do it."
            } else {
                r"Try `I\vskip-\lastskip' instead."
            };
            return crate::errors::error(
                st,
                crate::errors::you_cant(st),
                &[
                    "Sorry...I usually can't take things from the current page.",
                    remedy,
                ],
            );
        }
        return Ok(());
    }
    let tx = effective_tail(st);
    if st.mem.is_char_node(tx) || st.mem.node_type(tx) != Ok(t) {
        return Ok(());
    }
    if unlink_effective_tail(st, tx)? {
        st.mem.flush_node_list(tx)?;
    }
    Ok(())
}

pub(super) fn scan_box(st: &mut EngineState, box_context: i32) -> Result<(), Abort> {
    crate::scan::get_x_non_blank_non_relax(st)?;
    if let Some(Command::MakeBox(code)) = st.scanner.try_command() {
        begin_box(st, box_context, code)
    } else if box_context >= LEADER_FLAG
        && (st.scanner.cur_cmd == Cmd::Hrule || st.scanner.cur_cmd == Cmd::Vrule)
    {
        st.scanner.cur_box = Some(BoxOrRule::Rule(crate::scan::scan_rule_spec(st)?));
        box_end(st, box_context)
    } else {
        crate::errors::back_error(
            st,
            "A <box> was supposed to be here",
            &[
                r"I was expecting to see \hbox or \vbox or \copy or \box or",
                "something like that. So you might find something missing in",
                "your output. But keep trying; you can fix this later.",
            ],
        )
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
            let h = match st.mem[b].list_ptr.as_idx().map(|p| st.mem.classify(p)) {
                Some(Ok(AnyNode::Box(first))) => st.mem[first].height,
                Some(Ok(AnyNode::Rule(first))) => st.mem[first].height.to_scaled(),
                _ => Scaled::ZERO,
            };
            let depth = st.mem[b].depth - h + st.mem[b].height;
            st.mem[b].depth = depth;
            st.mem[b].height = h;
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
        st.mem[b].shift_amount = Scaled::from_raw(box_context);
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
                st.mem[noad.nucleus()].math_type = SUB_BOX;
                st.mem[noad.nucleus()].set_info(b.ptr());
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
            let SaLeaf::Pointer(p) =
                st.mem
                    .create_sa_element(&mut st.scanner, ValueLevel::BOX, n)?
            else {
                return Err(Abort::Confusion("box"));
            };
            sa_define(st, global, p, cb)?;
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
        let skip = match st.scanner.try_command() {
            Some(Command::Hskip(s)) if !vertical => Some(s),
            Some(Command::Vskip(s)) if vertical => Some(s),
            _ => None,
        };
        if let Some(s) = skip {
            let g = append_glue(st, s)?;
            let kind = LeaderCode::from_raw(box_context - LEADER_FLAG + LeaderCode::ALeaders.raw())
                .and_then(LeaderCode::glue_leaders)
                .ok_or(Abort::Confusion("leaders"))?;
            st.mem[g].subtype = GlueKind::Leaders(kind).into();
            st.mem[g].leader_ptr = cb.ptr();
        } else {
            crate::errors::back_error(
                st,
                "Leaders not followed by proper glue",
                &[
                    r"You should say `\leaders <box or rule><hskip or vskip>'.",
                    "I found the <box or rule>, but there's no suitable",
                    "<hskip or vskip>, so I'm ignoring these leaders.",
                ],
            )?;
            st.mem.flush_node_list(cb.ptr())?;
        }
    }
    Ok(())
}

impl MemState {
    fn is_math_node(&self, p: MemPtr, code: MathCode) -> bool {
        !self.is_char_node(p)
            && matches!(self.classify(p), Ok(AnyNode::Math(m)) if self[m].subtype == code)
    }
}

/// The last node of the current list, skipping a trailing end-of-math node.
fn effective_tail(st: &EngineState) -> MemPtr {
    let tail = st.input.cur_list.tail;
    if !st.mem.is_math_node(tail, MathCode::EndM) {
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
                    for _ in 0..st.mem[disc].replace_count {
                        p = p.link(&st.mem);
                    }
                    if p == tx {
                        return Ok(false);
                    }
                }
                Ok(AnyNode::Math(m)) => fm = st.mem[m].subtype == MathCode::BeginM,
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

pub(super) fn begin_box(
    st: &mut EngineState,
    box_context: i32,
    code: BoxCode,
) -> Result<(), Abort> {
    match code {
        BoxCode::Box => {
            let n = crate::scan::scan_register_num(st)?;
            st.scanner.cur_box = fetch_box(st, n).map(BoxOrRule::Box);
            st.mem
                .change_box(&mut st.scanner, &mut st.eqtb, n, MemPtr::NULL)?;
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
                crate::errors::error(
                    st,
                    crate::errors::you_cant(st),
                    &[r"Sorry; this \lastbox will be void."],
                )?;
            } else if st.input.cur_list.mode == VMODE
                && st.input.cur_list.head == st.input.cur_list.tail
            {
                crate::errors::error(
                    st,
                    crate::errors::you_cant(st),
                    &[
                        "Sorry...I usually can't take things from the current page.",
                        r"This \lastbox will therefore be void.",
                    ],
                )?;
            } else {
                let tx = effective_tail(st);
                if !st.mem.is_char_node(tx) {
                    if let Ok(Some(b)) = st.mem.classify_box(tx) {
                        if unlink_effective_tail(st, tx)? {
                            st.scanner.cur_box = Some(BoxOrRule::Box(b));
                            st.mem[b].shift_amount = Scaled::ZERO;
                        }
                    }
                }
            }
        }
        BoxCode::Vsplit => {
            let n = crate::scan::scan_register_num(st)?;
            if !crate::scan::scan_keyword(st, "to")? {
                crate::errors::error(
                    st,
                    "Missing `to' inserted",
                    &[
                        r"I'm working on `\vsplit<box number> to <dimen>';",
                        "will look for the <dimen> next.",
                    ],
                )?;
            }
            let d = crate::scan::scan_dimen(st, false, false)?;
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
