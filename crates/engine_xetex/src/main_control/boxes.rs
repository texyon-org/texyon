//! Boxes: `\hbox`, `\vbox`, `\box`, `\lastbox`, `\unhbox`, `\unskip` and friends.

use crate::cmd::Cmd;
use crate::codes::{BoxCode, LeaderCode, ValueLevel};
use crate::command::Command;
use crate::consts::{BOX_BASE, HMODE, IGNORE_DEPTH, MMODE, SUB_BOX, VMODE};
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::input::TokenType;
use crate::mem::{ADJUST_HEAD, PRE_ADJUST_HEAD};
use crate::node::{
    AnyNode, BoxNode, BoxOrRule, GlueKind, MarginKernNode, MathCode, NodePtr, SaLeaf, View,
};
use crate::packaging::{AdjustTails, PackOrigin};
use crate::params::{DimenPar, IntPar, TokPar};
use crate::scan::fetch_box;
use crate::state::{EngineState, GroupCode, MemState};
use crate::units::MemIdx;
use crate::units::RawPtr;
use crate::units::{NodeType, Scaled};

use super::assign::{define, sa_define};
use super::lists::{append_glue, normal_paragraph};
use super::{
    append_list, begin_every, BOX_FLAG, GLOBAL_BOX_FLAG, INNER_HMODE, LEADER_FLAG, SHIP_OUT_FLAG,
};

pub(super) fn unpackage(st: &mut EngineState, c: BoxCode) -> Result<(), Abort> {
    if matches!(c, BoxCode::LastBox | BoxCode::Vsplit) {
        let disc = &mut st.scanner.disc_ptr[c as usize];
        st.input.cur_list.tail.set_link(&mut st.mem, *disc);
        *disc = None;
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
            st.mem.change_box(&mut st.scanner, &mut st.eqtb, n, None)?;
            b.free(&mut st.mem);
        }
    }

    loop {
        let tail = st.input.cur_list.tail;
        let Some(r) = tail.link(&st.mem) else {
            return Ok(());
        };
        if matches!(
            st.mem.classify_non_char(Some(r)),
            Some(AnyNode::MarginKern(_))
        ) {
            let next = r.link(&st.mem);
            tail.set_link(&mut st.mem, next);
            st.mem.free_node(r, MarginKernNode::SIZE);
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
    let tx = effective_tail(st)?;
    if st.mem.is_char_node(tx) || st.mem.node_type(tx) != Ok(t) {
        return Ok(());
    }
    if unlink_effective_tail(st, tx)? {
        st.mem.flush_node_list(Some(tx))?;
    }
    Ok(())
}

pub(super) fn scan_box(st: &mut EngineState, box_context: i32) -> Result<(), Abort> {
    crate::scan::get_x_non_blank_non_relax(st)?;
    if let Command::MakeBox(code) = st.scanner.cur {
        begin_box(st, box_context, code)
    } else if box_context >= LEADER_FLAG
        && (st.scanner.cur.cmd() == Cmd::Hrule || st.scanner.cur.cmd() == Cmd::Vrule)
    {
        let rule = crate::scan::scan_rule_spec(st)?;
        leaders_end(st, box_context, BoxOrRule::Rule(rule))
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

/// Package the box closed by `group`, an `\hbox`, `\vbox` or `\vtop` group.
pub(super) fn package(st: &mut EngineState, group: GroupCode) -> Result<(), Abort> {
    let d = st.eqtb.dimen_par(DimenPar::BoxMaxDepth);
    let u = st.eqtb.int_par(IntPar::XetexUpwards);
    crate::save::unsave(st)?;
    let spec = st.save_stack.pop_spec()?;
    st.save_stack.save_ptr -= 1;
    let v = st.eqtb.int_par(IntPar::XetexUpwards);
    st.eqtb.set_int_par(IntPar::XetexUpwards, u);
    let list = st.input.cur_list.head.link(&st.mem);
    let mut adjust = (group == GroupCode::AdjustedHbox).then_some(AdjustTails::HEADS);
    let b = if st.input.cur_list.mode == INNER_HMODE {
        crate::packaging::hpack_with(st, list, spec, PackOrigin::Here, adjust.as_mut())?.r_box
    } else {
        let b = crate::packaging::vpackage(st, list, spec, d)?;
        if group == GroupCode::Vtop {
            let h = match st.mem[b].list_ptr.map(|p| st.mem.classify(p)) {
                Some(Ok(AnyNode::Box(first))) => st.mem[first].height,
                Some(Ok(AnyNode::Rule(first))) => st.mem[first].height.to_scaled(),
                _ => Scaled::ZERO,
            };
            let depth = st.mem[b].depth - h + st.mem[b].height;
            let node = &mut st.mem[b];
            node.depth = depth;
            node.height = h;
        }
        b
    };
    st.eqtb.set_int_par(IntPar::XetexUpwards, v);
    st.mem.pop_nest(&mut st.input)?;
    let box_context = st.save_stack.saved(0);
    box_end(st, box_context, Some(b), adjust)
}

/// Put `cur_box`, possibly void, where `box_context` says, with the material
/// `hpack` moved out of it to `adjust` placed around it in a vertical list.
fn box_end(
    st: &mut EngineState,
    box_context: i32,
    cur_box: Option<NodePtr<BoxNode>>,
    adjust: Option<AdjustTails>,
) -> Result<(), Abort> {
    if box_context < BOX_FLAG {
        let Some(b) = cur_box else {
            return Ok(());
        };
        st.mem[b].shift_amount = Scaled::from_raw(box_context);
        if st.input.cur_list.mode.abs() == VMODE {
            if let Some(adjust) = adjust.filter(|a| a.pre != Some(PRE_ADJUST_HEAD)) {
                append_list(st, Some(PRE_ADJUST_HEAD), adjust.pre)?;
            }
            st.input
                .cur_list
                .append_to_vlist(&mut st.mem, &st.eqtb, b)?;
            if let Some(adjust) = adjust.filter(|a| a.post != Some(ADJUST_HEAD)) {
                append_list(st, Some(ADJUST_HEAD), adjust.post)?;
            }
            if st.input.cur_list.mode > 0 {
                crate::pagebuilder::build_page(st)?;
            }
        } else {
            let node = if st.input.cur_list.mode.abs() == HMODE {
                st.input.cur_list.set_space_factor(1000);
                b.idx()
            } else {
                let noad = st.mem.new_noad()?;
                st.mem[noad].nucleus.math_type = SUB_BOX;
                st.mem[noad].nucleus.set_info(b.ptr());
                noad.idx()
            };
            st.input.cur_list.tail_append(&mut st.mem, node);
        }
    } else if box_context < SHIP_OUT_FLAG {
        let (n, global) = if box_context < GLOBAL_BOX_FLAG {
            (box_context - BOX_FLAG, false)
        } else {
            (box_context - GLOBAL_BOX_FLAG, true)
        };
        let cb = cur_box.map(NodePtr::idx);
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
    } else if let Some(b) = cur_box {
        if box_context == SHIP_OUT_FLAG {
            crate::shipout::ship_out(st, b)?;
        } else {
            leaders_end(st, box_context, BoxOrRule::Box(b))?;
        }
    }
    Ok(())
}

/// Make `content` the leaders of the glue that follows, for a leaders `box_context`.
fn leaders_end(st: &mut EngineState, box_context: i32, content: BoxOrRule) -> Result<(), Abort> {
    crate::scan::get_x_non_blank_non_relax(st)?;
    let vertical = st.input.cur_list.mode.abs() == VMODE;
    let skip = match st.scanner.cur {
        Command::Hskip(s) if !vertical => Some(s),
        Command::Vskip(s) if vertical => Some(s),
        _ => None,
    };
    if let Some(s) = skip {
        let g = append_glue(st, s)?;
        let kind = LeaderCode::from_raw(box_context - LEADER_FLAG + LeaderCode::ALeaders.raw())
            .and_then(LeaderCode::glue_leaders)
            .or_confusion("leaders")?;
        let node = &mut st.mem[g];
        node.subtype = GlueKind::Leaders(kind).into();
        node.leader_ptr = content.ptr();
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
        st.mem.flush_node_list(content.ptr())?;
    }
    Ok(())
}

impl MemState {
    fn is_math_node(&self, p: MemIdx, code: MathCode) -> bool {
        matches!(self.classify_non_char(Some(p)), Some(AnyNode::Math(m)) if self[m].subtype == code)
    }
}

/// The last node of the current list, skipping a trailing end-of-math node.
fn effective_tail(st: &EngineState) -> Result<MemIdx, Abort> {
    let tail = st.input.cur_list.tail;
    if !st.mem.is_math_node(tail, MathCode::EndM) {
        return Ok(tail);
    }
    st.mem.predecessor(st.input.cur_list.head, Some(tail))
}

/// Unlinks node `tx` from the current list, unless it hides in a
/// discretionary's replacement text. Returns whether it was unlinked.
fn unlink_effective_tail(st: &mut EngineState, tx: MemIdx) -> Result<bool, Abort> {
    let mut q = st.input.cur_list.head;
    let mut p = None;
    let mut r;
    let mut fm;
    let p = loop {
        r = p;
        let mut here = q;
        fm = false;
        if !st.mem.is_char_node(q) {
            match st.mem.classify(q) {
                Ok(AnyNode::Disc(disc)) => {
                    for _ in 0..st.mem[disc].replace_count {
                        here = here.link(&st.mem).or_confusion("tail1")?;
                    }
                    if here == tx {
                        return Ok(false);
                    }
                }
                Ok(AnyNode::Math(m)) => fm = st.mem[m].subtype == MathCode::BeginM,
                _ => {}
            }
        }
        p = Some(here);
        q = here.link(&st.mem).or_confusion("tail1")?;
        if q == tx {
            break here;
        }
    };
    let next = tx.link(&st.mem);
    p.set_link(&mut st.mem, next);
    tx.set_link(&mut st.mem, None);
    if next.is_none() {
        if fm {
            return Err(Abort::Confusion("tail1"));
        }
        st.input.cur_list.tail = p;
    } else if fm {
        let r = r.or_confusion("tail1")?;
        st.input.cur_list.tail = r;
        r.set_link(&mut st.mem, None);
        st.mem.flush_node_list(Some(p))?;
    }
    Ok(true)
}

pub(super) fn begin_box(
    st: &mut EngineState,
    box_context: i32,
    code: BoxCode,
) -> Result<(), Abort> {
    let cur_box = match code {
        BoxCode::Box => {
            let n = crate::scan::scan_register_num(st)?;
            let b = fetch_box(st, n);
            st.mem.change_box(&mut st.scanner, &mut st.eqtb, n, None)?;
            b
        }
        BoxCode::Copy => {
            let n = crate::scan::scan_register_num(st)?;
            match fetch_box(st, n) {
                Some(b) => Some(st.mem.copy_box(b)?),
                None => None,
            }
        }
        BoxCode::LastBox => {
            let mut last = None;
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
                let tx = effective_tail(st)?;
                if !st.mem.is_char_node(tx) {
                    if let Ok(Some(b)) = st.mem.classify_box(tx) {
                        if unlink_effective_tail(st, tx)? {
                            last = Some(b);
                            st.mem[b].shift_amount = Scaled::ZERO;
                        }
                    }
                }
            }
            last
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
            let d = crate::scan::scan_dimen(st, false)?;
            crate::packaging::vsplit(st, n, d)?
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
    };
    box_end(st, box_context, cur_box, None)
}
