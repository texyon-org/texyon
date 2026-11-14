//! Boxes: `\hbox`, `\vbox`, `\box`, `\lastbox`, `\unhbox`, `\unskip` and friends.

use crate::base::codes::BoxCode;
use crate::base::command::Command;
use crate::base::consts::SUB_BOX;
use crate::base::mode::{Mode, ModeName};
use crate::base::params::{DimenPar, IntPar, TokPar};
use crate::base::units::MemIdx;
use crate::base::units::{NodeType, Scaled};
use crate::build::packaging::{AdjustTails, PackOrigin};
use crate::memory::mem::{ADJUST_HEAD, PRE_ADJUST_HEAD};
use crate::memory::node::LeaderKind;
use crate::memory::node::{
    AnyNode, BoxNode, BoxOrRule, GlueKind, MarginKernNode, MathCode, NodePtr, View,
};
use crate::print::errors::Abort;
use crate::print::errors::OrConfusion;
use crate::scanning::input::TokenType;
use crate::scanning::scan::fetch_box;
use crate::state::Saved;
use crate::state::{EngineState, GroupCode, MemState};
use crate::tables::eqtb::BoxLoc;

use super::lists::{append_glue, normal_paragraph};
use super::{BoxContext, append_list, begin_every};

pub(super) fn unpackage(st: &mut EngineState, c: BoxCode) -> Result<(), Abort> {
    if matches!(c, BoxCode::LastBox | BoxCode::Vsplit) {
        let disc = &mut st.scanner.disc_ptr[c as usize];
        st.input.cur_list.tail.set_link(&mut st.mem, *disc);
        *disc = None;
    } else {
        let n = crate::scanning::scan::scan_register_num(st)?;
        let Some(b) = fetch_box(st, n) else {
            return Ok(());
        };
        let mode = st.input.cur_list.mode;
        if mode.is_math()
            || (mode.is_vertical() && !st.mem[b].is_vlist())
            || (mode.is_horizontal() && !st.mem[b].is_hlist())
        {
            return crate::print::errors::error(
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
            st.eqtb.set_box_reg(n, None);
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
    if st.input.cur_list.mode.name() == ModeName::Vertical
        && st.input.cur_list.tail == st.input.cur_list.head
    {
        if t != NodeType::Glue || st.shipout.last_glue.is_some() {
            let remedy = if t == NodeType::Kern {
                r"Try `I\kern-\lastkern' instead."
            } else if t != NodeType::Glue {
                "Perhaps you can make the output routine do it."
            } else {
                r"Try `I\vskip-\lastskip' instead."
            };
            return crate::print::errors::error(
                st,
                crate::print::errors::you_cant(st),
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

pub(super) fn scan_box(st: &mut EngineState, box_context: BoxContext) -> Result<(), Abort> {
    crate::scanning::scan::get_x_non_blank_non_relax(st)?;
    match (box_context, st.scanner.cur) {
        (_, Command::MakeBox(code)) => begin_box(st, box_context, code),
        (BoxContext::Leaders(kind), Command::Hrule | Command::Vrule) => {
            let rule = crate::scanning::scan::scan_rule_spec(st)?;
            leaders_end(st, kind, BoxOrRule::Rule(rule))
        }
        _ => crate::print::errors::back_error(
            st,
            "A <box> was supposed to be here",
            &[
                r"I was expecting to see \hbox or \vbox or \copy or \box or",
                "something like that. So you might find something missing in",
                "your output. But keep trying; you can fix this later.",
            ],
        ),
    }
}

/// Package the box closed by `group`, an `\hbox`, `\vbox` or `\vtop` group.
pub(super) fn package(st: &mut EngineState, group: GroupCode) -> Result<(), Abort> {
    let d = st.eqtb.dimen_par(DimenPar::BoxMaxDepth);
    let u = st.eqtb.int_par(IntPar::XetexUpwards);
    crate::tables::save::unsave(st)?;
    let spec = st.save_stack.pop_spec()?;
    let Some(Saved::BoxContext(box_context)) = st.save_stack.pop_saved() else {
        return Err(Abort::Confusion("box_context"));
    };
    let v = st.eqtb.int_par(IntPar::XetexUpwards);
    st.eqtb.set_int_par(IntPar::XetexUpwards, u);
    let list = st.input.cur_list.head.link(&st.mem);
    let mut adjust = (group == GroupCode::AdjustedHbox).then_some(AdjustTails::HEADS);
    let b = if st.input.cur_list.mode.name() == ModeName::RestrictedHorizontal {
        crate::build::packaging::hpack_with(st, list, spec, PackOrigin::Here, adjust.as_mut())?
            .r_box
    } else {
        let b = crate::build::packaging::vpackage(st, list, spec, d)?;
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
    box_end(st, box_context, Some(b), adjust)
}

/// Put `cur_box`, possibly void, where `box_context` says, with the material
/// `hpack` moved out of it to `adjust` placed around it in a vertical list.
fn box_end(
    st: &mut EngineState,
    box_context: BoxContext,
    cur_box: Option<NodePtr<BoxNode>>,
    adjust: Option<AdjustTails>,
) -> Result<(), Abort> {
    match box_context {
        BoxContext::Shift(shift) => {
            let Some(b) = cur_box else {
                return Ok(());
            };
            st.mem[b].shift_amount = shift;
            if st.input.cur_list.mode.is_vertical() {
                if let Some(adjust) = adjust.filter(|a| a.pre != Some(PRE_ADJUST_HEAD)) {
                    append_list(st, Some(PRE_ADJUST_HEAD), adjust.pre)?;
                }
                st.input
                    .cur_list
                    .append_to_vlist(&mut st.mem, &st.eqtb, b)?;
                if let Some(adjust) = adjust.filter(|a| a.post != Some(ADJUST_HEAD)) {
                    append_list(st, Some(ADJUST_HEAD), adjust.post)?;
                }
                if st.input.cur_list.mode.is_outer() {
                    crate::build::pagebuilder::build_page(st)?;
                }
            } else {
                let node =
                    if let Mode::Horizontal { space_factor, .. } = &mut st.input.cur_list.mode {
                        *space_factor = 1000;
                        b.idx()
                    } else {
                        let noad = st.mem.new_noad()?;
                        st.mem[noad].nucleus.math_type = SUB_BOX;
                        st.mem[noad].nucleus.set_info(b.ptr());
                        noad.idx()
                    };
                st.input.cur_list.tail_append(&mut st.mem, node);
            }
        }
        BoxContext::SetBox { register, global } => {
            crate::tables::eqtb::define(st, global, BoxLoc::register(register), cur_box)?;
        }
        BoxContext::ShipOut => {
            if let Some(b) = cur_box {
                crate::shipout::ship_out(st, b)?;
            }
        }
        BoxContext::Leaders(kind) => {
            if let Some(b) = cur_box {
                leaders_end(st, kind, BoxOrRule::Box(b))?;
            }
        }
    }
    Ok(())
}

/// Make `content` the leaders of the glue that follows, for a leaders `box_context`.
fn leaders_end(st: &mut EngineState, kind: LeaderKind, content: BoxOrRule) -> Result<(), Abort> {
    crate::scanning::scan::get_x_non_blank_non_relax(st)?;
    let vertical = st.input.cur_list.mode.is_vertical();
    let skip = match st.scanner.cur {
        Command::Hskip(s) if !vertical => Some(s),
        Command::Vskip(s) if vertical => Some(s),
        _ => None,
    };
    if let Some(s) = skip {
        let g = append_glue(st, s)?;
        let node = &mut st.mem[g];
        node.subtype = GlueKind::Leaders(kind).into();
        node.leader_ptr = content.ptr();
    } else {
        crate::print::errors::back_error(
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
    box_context: BoxContext,
    code: BoxCode,
) -> Result<(), Abort> {
    let cur_box = match code {
        BoxCode::Box => {
            let n = crate::scanning::scan::scan_register_num(st)?;
            let b = fetch_box(st, n);
            st.eqtb.set_box_reg(n, None);
            b
        }
        BoxCode::Copy => {
            let n = crate::scanning::scan::scan_register_num(st)?;
            match fetch_box(st, n) {
                Some(b) => Some(st.mem.copy_box(b)?),
                None => None,
            }
        }
        BoxCode::LastBox => {
            let mut last = None;
            if st.input.cur_list.mode.is_math() {
                crate::print::errors::error(
                    st,
                    crate::print::errors::you_cant(st),
                    &[r"Sorry; this \lastbox will be void."],
                )?;
            } else if st.input.cur_list.mode.name() == ModeName::Vertical
                && st.input.cur_list.head == st.input.cur_list.tail
            {
                crate::print::errors::error(
                    st,
                    crate::print::errors::you_cant(st),
                    &[
                        "Sorry...I usually can't take things from the current page.",
                        r"This \lastbox will therefore be void.",
                    ],
                )?;
            } else {
                let tx = effective_tail(st)?;
                if !st.mem.is_char_node(tx)
                    && let Ok(Some(b)) = st.mem.classify_box(tx)
                    && unlink_effective_tail(st, tx)?
                {
                    last = Some(b);
                    st.mem[b].shift_amount = Scaled::ZERO;
                }
            }
            last
        }
        BoxCode::Vsplit => {
            let n = crate::scanning::scan::scan_register_num(st)?;
            if !crate::scanning::scan::scan_keyword(st, "to")? {
                crate::print::errors::error(
                    st,
                    "Missing `to' inserted",
                    &[
                        r"I'm working on `\vsplit<box number> to <dimen>';",
                        "will look for the <dimen> next.",
                    ],
                )?;
            }
            let d = crate::scanning::scan::scan_dimen(st, false)?;
            crate::build::packaging::vsplit(st, n, d)?
        }
        BoxCode::Vtop | BoxCode::Vbox | BoxCode::Hbox => {
            let horizontal = code == BoxCode::Hbox;
            if horizontal {
                let group = if matches!(box_context, BoxContext::Shift(_))
                    && st.input.cur_list.mode.is_vertical()
                {
                    GroupCode::AdjustedHbox
                } else {
                    GroupCode::Hbox
                };
                crate::scanning::scan::scan_spec(st, group, Some(box_context))?;
            } else {
                let group = if code == BoxCode::Vbox {
                    GroupCode::Vbox
                } else {
                    GroupCode::Vtop
                };
                crate::scanning::scan::scan_spec(st, group, Some(box_context))?;
                normal_paragraph(st)?;
            }
            crate::memory::mem::push_nest(st)?;
            if horizontal {
                st.input.cur_list.mode = Mode::horizontal(true);
                begin_every(st, TokPar::EveryHbox, TokenType::EveryHbox)?;
            } else {
                st.input.cur_list.mode = Mode::vertical(true);
                begin_every(st, TokPar::EveryVbox, TokenType::EveryVbox)?;
            }
            return Ok(());
        }
    };
    box_end(st, box_context, cur_box, None)
}
