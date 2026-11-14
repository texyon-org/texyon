//! Entering and leaving math mode: `init_math`, `after_math` and displays.

use super::build::fin_mlist;
use super::mlist::mlist_to_hlist;
use super::{MathStyle, SCRIPT_SCRIPT_SIZE};
use crate::base::cmd::Cmd;
use crate::base::consts::{INF_PENALTY, MAX_DIMEN, SCRIPT_SIZE, TEXT_SIZE};
use crate::base::mode::{Mode, ModeName};
use crate::base::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::base::units::{Dir, GlueOrder, GlueSign, MemIdx, NodeType, Scaled};
use crate::build::linebreak::line_break;
use crate::build::main_control::{push_math, start_paragraph_language};
use crate::build::packaging::{
    AdjustTails, LrState, PackOrigin, PackSpec, append_to_vlist, hpack, hpack_with,
};
use crate::build::pagebuilder::build_page;
use crate::fonts::ext::{insert_special, insert_src_special};
use crate::fonts::{FontState, QUAD_CODE};
use crate::memory::mem::{ADJUST_HEAD, MemState, PRE_ADJUST_HEAD, TEMP_HEAD, ZERO_GLUE, push_nest};
use crate::memory::node::{AnyNode, BoxLr, BoxNode, GlueNode, MathCode, NodePtr};
use crate::print::errors::{Abort, OrConfusion, back_error, error};
use crate::scanning::input::{TokenType, back_input, begin_token_list};
use crate::scanning::scan::{get_token, get_x_token};
use crate::state::EngineState;
use crate::tables::eqtb::{EqtbState, define};
use crate::tables::save::{GroupCode, Saved, unsave};

const TOTAL_MATHSY_PARAMS: i32 = 22;
const TOTAL_MATHEX_PARAMS: i32 = 13;

pub(crate) fn resume_after_display(st: &mut EngineState) -> Result<(), Abort> {
    if st.save_stack.cur_group != GroupCode::MathShift {
        return Err(Abort::Confusion("display"));
    }

    unsave(st)?;
    st.input.cur_list.prev_graf = st.input.cur_list.prev_graf.wrapping_add(3);
    push_nest(st)?;
    st.input.cur_list.mode = Mode::horizontal(false);

    start_paragraph_language(st)?;

    get_x_token(st)?;
    if st.scanner.cur.cmd() != Cmd::Spacer {
        back_input(st)?;
    }
    if st.input.nest.len() == 1 {
        build_page(st)?;
    }
    Ok(())
}

impl FontState {
    #[inline]
    fn sym_font_ok(&self, eqtb: &EqtbState, size: i32) -> bool {
        let f = eqtb.math_font(2 + size);
        self.param_count(f) >= TOTAL_MATHSY_PARAMS || self.font_is_ot_math(f)
    }

    #[inline]
    fn ext_font_ok(&self, eqtb: &EqtbState, size: i32) -> bool {
        let f = eqtb.math_font(3 + size);
        self.param_count(f) >= TOTAL_MATHEX_PARAMS || self.font_is_ot_math(f)
    }
}

impl MemState {
    /// Fills the fresh spec of `skip` with `w` minus the glue of `g`, negating its stretch and shrink.
    fn set_display_skip(&mut self, skip: NodePtr<GlueNode>, g: NodePtr<GlueNode>, w: Scaled) {
        let src = self[g].glue_ptr;
        let dst = self[skip].glue_ptr;
        let from = self[src];
        let node = &mut self[dst];
        node.stretch_order = from.stretch_order;
        node.shrink_order = from.shrink_order;
        node.width = w - from.width;
        node.stretch = -from.stretch;
        node.shrink = -from.shrink;
    }

    /// Sets the width of `p`, an end of the display's kern frame that is not glue.
    fn set_kern_or_math_width(&mut self, p: MemIdx, w: Scaled) -> Result<(), Abort> {
        match self.classify(p) {
            Ok(AnyNode::Kern(k)) => self[k].width = w,
            Ok(AnyNode::Math(m)) => self[m].width = w,
            _ => return Err(Abort::Confusion("app_display")),
        }
        Ok(())
    }
}

fn app_display(
    st: &mut EngineState,
    j: Option<NodePtr<BoxNode>>,
    orig: NodePtr<BoxNode>,
    d: Scaled,
) -> Result<(), Abort> {
    let s = st.eqtb.dimen_par(DimenPar::DisplayIndent);
    let x = st.eqtb.int_par(IntPar::PreDisplayDirection);
    if x == 0 {
        st.mem[orig].shift_amount = s + d;
        append_to_vlist(st, orig)?;
        return Ok(());
    }

    let mut d = d;
    let z = st.eqtb.dimen_par(DimenPar::DisplayWidth);
    let mut e;
    if x > 0 {
        e = z - d - st.mem[orig].width;
    } else {
        e = d;
        d = z - e - st.mem[orig].width;
    }
    let copy = if let Some(j) = j {
        let bc = st.mem.copy_box(j)?;
        let (h, dp) = (st.mem[orig].height, st.mem[orig].depth);
        let node = &mut st.mem[bc];
        node.height = h;
        node.depth = dp;
        let s2 = s - st.mem[bc].shift_amount;
        d += s2;
        e = e + st.mem[bc].width - z - s2;
        Some(bc)
    } else {
        None
    };
    let mut b = copy.unwrap_or(orig);

    let (p, q);
    if st.mem[orig].lr() == BoxLr::Dlist {
        p = orig.idx();
        q = p;
    } else {
        let list = st.mem[orig].list_ptr;
        orig.free(&mut st.mem);
        let first = list.or_confusion("LR4")?;
        if x > 0 {
            p = first;
            q = st.mem.list_tail(first);
        } else {
            let mut rev = None;
            let mut r = list;
            q = first;
            while let Some(here) = r {
                let t = here.link(&st.mem);
                here.set_link(&mut st.mem, rev);
                rev = r;
                r = t;
            }
            p = rev.or_confusion("LR4")?;
        }
    }

    let (r, t) = match copy {
        None => (
            st.mem.new_kern(Scaled::ZERO)?.idx(),
            st.mem.new_kern(Scaled::ZERO)?.idx(),
        ),
        Some(bc) => {
            let r = st.mem[bc].list_ptr.or_confusion("app_display")?;
            (r, r.link(&st.mem).or_confusion("app_display")?)
        }
    };
    let u = st.mem.new_math(Scaled::ZERO, MathCode::EndM)?.idx();
    if let Ok(AnyNode::Glue(tg)) = st.mem.classify(t) {
        let skip = st.mem.new_skip_param(&st.eqtb, GluePar::RightSkip)?;
        q.set_link(&mut st.mem, skip.ptr());
        st.mem[skip].link = Some(u);
        st.mem.set_display_skip(skip, tg, e);
        u.set_link(&mut st.mem, Some(t));
    } else {
        st.mem.set_kern_or_math_width(t, e)?;
        t.set_link(&mut st.mem, Some(u));
        q.set_link(&mut st.mem, Some(t));
    }

    let u = st.mem.new_math(Scaled::ZERO, MathCode::BeginM)?.idx();
    if let Ok(AnyNode::Glue(rg)) = st.mem.classify(r) {
        let skip = st.mem.new_skip_param(&st.eqtb, GluePar::LeftSkip)?;
        u.set_link(&mut st.mem, skip.ptr());
        st.mem[skip].link = Some(p);
        st.mem.set_display_skip(skip, rg, d);
        r.set_link(&mut st.mem, Some(u));
    } else {
        st.mem.set_kern_or_math_width(r, d)?;
        r.set_link(&mut st.mem, Some(p));
        u.set_link(&mut st.mem, Some(r));
        match copy {
            None => {
                b = hpack(st, Some(u), PackSpec::NATURAL)?;
                st.mem[b].shift_amount = s;
            }
            Some(bc) => st.mem[bc].list_ptr = Some(u),
        }
    }
    append_to_vlist(st, b)?;
    Ok(())
}

/// Checks the family 2 and 3 fonts. On failure reports the error, flushes the
/// formula and returns `true` (TeX's `danger`).
fn check_math_fonts(st: &mut EngineState) -> Result<bool, Abort> {
    let sizes = [TEXT_SIZE, SCRIPT_SIZE, SCRIPT_SCRIPT_SIZE];
    let (msg, help) = if !sizes.iter().all(|&z| st.fonts.sym_font_ok(&st.eqtb, z)) {
        (
            "Math formula deleted: Insufficient symbol fonts",
            [
                r"Sorry, but I can't typeset math unless \textfont 2",
                r"and \scriptfont 2 and \scriptscriptfont 2 have all",
                r"the \fontdimen values needed in math symbol fonts.",
            ],
        )
    } else if !sizes.iter().all(|&z| st.fonts.ext_font_ok(&st.eqtb, z)) {
        (
            "Math formula deleted: Insufficient extension fonts",
            [
                r"Sorry, but I can't typeset math unless \textfont 3",
                r"and \scriptfont 3 and \scriptscriptfont 3 have all",
                r"the \fontdimen values needed in math extension fonts.",
            ],
        )
    } else {
        return Ok(false);
    };
    error(st, msg, &help)?;
    st.input.cur_list.flush_math(&mut st.mem)?;
    Ok(true)
}

pub(crate) fn check_display_end(st: &mut EngineState) -> Result<(), Abort> {
    get_x_token(st)?;
    if st.scanner.cur.cmd() != Cmd::MathShift {
        back_error(
            st,
            "Display math should end with $$",
            &[
                "The `$' that I just saw supposedly matches a previous `$$'.",
                "So I shall assume that you typed `$$' both times.",
            ],
        )?;
    }
    Ok(())
}

/// The box `init_math` saved for the paragraph line before a display, if any.
fn pre_display_box(st: &EngineState) -> Option<NodePtr<BoxNode>> {
    if st.input.cur_list.mode.name() != ModeName::DisplayMath {
        return None;
    }
    let j = st.input.cur_list.e_tex_aux?;
    st.mem.classify_box(j).ok().flatten()
}

pub(crate) fn after_math(st: &mut EngineState) -> Result<(), Abort> {
    let mut j = pre_display_box(st);
    let mut danger = check_math_fonts(st)?;
    let mut inline = st.input.cur_list.mode.is_inner();
    let mut l = false;
    let mut p = fin_mlist(st, None)?;

    let a;
    if inline && st.input.cur_list.mode.name() == ModeName::DisplayMath {
        check_display_end(st)?;
        let hlist = mlist_to_hlist(st, p, MathStyle::TEXT, false)?;
        let eqno = hpack(st, hlist, PackSpec::NATURAL)?;
        st.mem[eqno].set_lr(BoxLr::Dlist);
        a = Some(eqno);
        unsave(st)?;
        match st.save_stack.pop_saved() {
            Some(Saved::EqNo { left }) => l = left,
            _ => return Err(Abort::Confusion("eqno")),
        }

        j = pre_display_box(st);
        danger = check_math_fonts(st)?;
        inline = st.input.cur_list.mode.is_inner();
        p = fin_mlist(st, None)?;
    } else {
        a = None;
    }

    if inline {
        let math_surround = st.eqtb.dimen_par(DimenPar::MathSurround);
        let node = st.mem.new_math(math_surround, MathCode::Before)?;
        st.input.cur_list.tail_append(&mut st.mem, node);
        let penalties = st.input.cur_list.mode.is_outer();
        let hlist = mlist_to_hlist(st, p, MathStyle::TEXT, penalties)?;
        let cl = &mut st.input.cur_list;
        cl.tail.set_link(&mut st.mem, hlist);
        cl.tail = st.mem.list_tail(cl.tail);
        let node = st.mem.new_math(math_surround, MathCode::After)?;
        st.input.cur_list.tail_append(&mut st.mem, node);
        st.input.cur_list.set_space_factor(1000)?;

        if st.config.semantic_pagination_enabled {
            insert_special(st, "tdux:ce math")?;
        }

        unsave(st)?;
        return Ok(());
    }

    if a.is_none() {
        check_display_end(st)?;
    }
    let mut p = mlist_to_hlist(st, p, MathStyle::DISPLAY, false)?;
    let mut adjust = AdjustTails::HEADS;
    let packed = hpack_with(
        st,
        p,
        PackSpec::NATURAL,
        PackOrigin::Here,
        Some(&mut adjust),
    )?;
    let mut b = packed.r_box;
    p = st.mem[b].list_ptr;
    let t = adjust.post;
    let pre_t = adjust.pre;
    let mut w = st.mem[b].width;
    let z = st.eqtb.dimen_par(DimenPar::DisplayWidth);
    let mut s = st.eqtb.dimen_par(DimenPar::DisplayIndent);
    if st.eqtb.int_par(IntPar::PreDisplayDirection) < 0 {
        s = -s - z;
    }
    let (mut e, q);
    match a {
        Some(eqno) if !danger => {
            e = st.mem[eqno].width;
            q = e + st.fonts.math_quad(&st.eqtb, TEXT_SIZE);
        }
        _ => {
            e = Scaled::ZERO;
            q = Scaled::ZERO;
        }
    }
    if w + q > z {
        if e != Scaled::ZERO
            && (w - packed.shrink[GlueOrder::Normal] + q <= z
                || packed.shrink[GlueOrder::Fil] != Scaled::ZERO
                || packed.shrink[GlueOrder::Fill] != Scaled::ZERO
                || packed.shrink[GlueOrder::Filll] != Scaled::ZERO)
        {
            b.free(&mut st.mem);
            b = hpack(st, p, PackSpec::Exactly(z - q))?;
        } else {
            e = Scaled::ZERO;
            if w > z {
                b.free(&mut st.mem);
                b = hpack(st, p, PackSpec::Exactly(z))?;
            }
        }
        w = st.mem[b].width;
    }
    st.mem[b].set_lr(BoxLr::Dlist);
    let mut d = (z - w).half();
    if e > Scaled::ZERO && d < e * 2 {
        d = (z - w - e).half();
        if let Some(first) = p
            && !st.mem.is_char_node(first)
            && st.mem.node_type(first) == Ok(NodeType::Glue)
        {
            d = Scaled::ZERO;
        }
    }

    let pen = st
        .mem
        .new_penalty(st.eqtb.int_par(IntPar::PreDisplayPenalty))?;
    st.input.cur_list.tail_append(&mut st.mem, pen);

    let pre_display_size = st.eqtb.dimen_par(DimenPar::PreDisplaySize);
    let (g1, mut g2) = if d + s <= pre_display_size || l {
        (GluePar::AboveDisplaySkip, Some(GluePar::BelowDisplaySkip))
    } else {
        (
            GluePar::AboveDisplayShortSkip,
            Some(GluePar::BelowDisplayShortSkip),
        )
    };

    if let Some(eqno) = a.filter(|_| l && e == Scaled::ZERO) {
        app_display(st, j, eqno, Scaled::ZERO)?;
        let pen = st.mem.new_penalty(INF_PENALTY)?;
        st.input.cur_list.tail_append(&mut st.mem, pen);
    } else {
        let glue = st.mem.new_param_glue(&st.eqtb, g1)?;
        st.input.cur_list.tail_append(&mut st.mem, glue);
    }

    if let Some(eqno) = a.filter(|_| e != Scaled::ZERO) {
        let r = st.mem.new_kern(z - w - e - d)?;
        if l {
            st.mem[eqno].link = r.ptr();
            st.mem[r].link = b.ptr();
            b = eqno;
            d = Scaled::ZERO;
        } else {
            st.mem[b].link = r.ptr();
            st.mem[r].link = eqno.ptr();
        }
        b = hpack(st, b.ptr(), PackSpec::NATURAL)?;
    }

    app_display(st, j, b, d)?;

    if let Some(eqno) = a.filter(|_| e == Scaled::ZERO && !l) {
        let pen = st.mem.new_penalty(INF_PENALTY)?;
        st.input.cur_list.tail_append(&mut st.mem, pen);
        let a_width = st.mem[eqno].width;
        app_display(st, j, eqno, z - a_width)?;
        g2 = None;
    }

    let cl = &mut st.input.cur_list;
    if t != Some(ADJUST_HEAD) {
        let adjusts = ADJUST_HEAD.link(&st.mem);
        cl.tail.set_link(&mut st.mem, adjusts);
        cl.tail = t.or_confusion("adjust_tail")?;
    }
    if pre_t != Some(PRE_ADJUST_HEAD) {
        let pre_adjusts = PRE_ADJUST_HEAD.link(&st.mem);
        cl.tail.set_link(&mut st.mem, pre_adjusts);
        cl.tail = pre_t.or_confusion("pre_adjust_tail")?;
    }

    let pen = st
        .mem
        .new_penalty(st.eqtb.int_par(IntPar::PostDisplayPenalty))?;
    st.input.cur_list.tail_append(&mut st.mem, pen);

    if let Some(g2) = g2 {
        let glue = st.mem.new_param_glue(&st.eqtb, g2)?;
        st.input.cur_list.tail_append(&mut st.mem, glue);
    }

    if let Some(j) = j {
        st.mem.flush_node_list(j.ptr())?;
    }

    if st.config.semantic_pagination_enabled {
        insert_special(st, "tdux:ce dmath")?;
    }

    resume_after_display(st)
}

/// TeX's `\predisplaydirection` for the paragraph just ended.
fn pre_display_direction(st: &EngineState) -> i32 {
    let aux = st.input.cur_list.e_tex_aux;
    if aux.is_none() {
        0
    } else if st.mem.lr_code(aux).map(MathCode::dir) == Some(Dir::Rtl) {
        -1
    } else {
        1
    }
}

/// A kern, or the parameter glue when that skip is nonzero.
fn side_skip(st: &mut EngineState, param: GluePar) -> Result<MemIdx, Abort> {
    if st.eqtb.glue_par(param) == ZERO_GLUE {
        st.mem.new_kern(Scaled::ZERO).map(|k| k.idx())
    } else {
        st.mem.new_param_glue(&st.eqtb, param).map(|g| g.idx())
    }
}

pub(crate) fn init_math(st: &mut EngineState) -> Result<(), Abort> {
    get_token(st)?;

    if st.scanner.cur.cmd() == Cmd::MathShift && st.input.cur_list.mode.is_outer() {
        let mut j = None;
        let mut w = -MAX_DIMEN;
        let x;

        if st.input.cur_list.head == st.input.cur_list.tail {
            st.mem.pop_nest(&mut st.input)?;
            x = pre_display_direction(st);
        } else {
            let jb = line_break(st, true)?;

            let right = side_skip(st, GluePar::RightSkip)?;
            let left = side_skip(st, GluePar::LeftSkip)?;
            left.set_link(&mut st.mem, Some(right));

            let jbox = st.mem.new_null_box()?;
            j = jbox.ptr();
            let mem = &mut st.mem;
            let src = mem[jb];
            let node = &mut mem[jbox];
            node.width = src.width;
            node.shift_amount = src.shift_amount;
            node.list_ptr = Some(left);
            node.glue_order = src.glue_order;
            node.glue_sign = src.glue_sign;
            node.glue_set = src.glue_set;

            let mut v = st.mem[jb].shift_amount;
            x = pre_display_direction(st);

            let mut lr = LrState::START;
            let mut p;
            if x >= 0 {
                p = st.mem[jb].list_ptr;
                TEMP_HEAD.set_link(&mut st.mem, None);
            } else {
                v = -v - st.mem[jb].width;
                let begin = st.mem.new_math(Scaled::ZERO, MathCode::BeginL)?.idx();
                TEMP_HEAD.set_link(&mut st.mem, Some(begin));
                let end = st.mem.new_math(Scaled::ZERO, MathCode::EndL)?.ptr();
                let list = st.mem[jb].list_ptr;
                st.mem.just_copy(list, begin, end)?;
                lr.dir = Dir::Rtl;
                p = Some(begin);
            }

            let cur_font = st.eqtb.cur_font();
            v += st.fonts.param(cur_font, QUAD_CODE) * 2;

            let texxet = st.eqtb.int_par(IntPar::Texxet) > 0;
            if texxet {
                st.mem.push_lr(&mut lr.ptr, MathCode::Before)?;
            }

            'outer: while let Some(mut here) = p {
                let (d, found) = {
                    let shown = match st.mem.as_char(Some(here)) {
                        Some(ch) => Some((st.mem[ch].font, st.mem[ch].character)),
                        None => match st.mem.classify(here) {
                            Ok(AnyNode::Ligature(lig)) => {
                                st.fonts.xtx_ligature_present = true;
                                Some((st.mem[lig].font, st.mem[lig].character))
                            }
                            _ => None,
                        },
                    };
                    if let Some((f, c)) = shown {
                        let c = st.fonts.effective_char(f, c);
                        (st.fonts.char_metrics(f, c).width, true)
                    } else {
                        match st.mem.classify(here) {
                            Ok(node @ (AnyNode::Box(_) | AnyNode::Rule(_))) => {
                                (node.hlist_width(&st.mem), true)
                            }
                            Ok(AnyNode::Kern(k)) => (st.mem[k].width, false),
                            Ok(AnyNode::MarginKern(k)) => (st.mem[k].width, false),
                            Ok(AnyNode::Math(m)) => {
                                let d = st.mem[m].width;
                                let code = st.mem[m].code()?;
                                if texxet {
                                    if code.is_end() {
                                        if st.mem.lr_code(lr.ptr) == Some(code.lr_end()) {
                                            st.mem.pop_lr(&mut lr.ptr);
                                        } else if code.is_lr() {
                                            w = MAX_DIMEN;
                                            break 'outer;
                                        }
                                    } else {
                                        st.mem.push_lr(&mut lr.ptr, code.lr_end())?;
                                        if code.dir() != lr.dir {
                                            st.mem.just_reverse(&mut lr, here)?;
                                            here = TEMP_HEAD;
                                        }
                                    }
                                } else if code.is_lr() {
                                    w = MAX_DIMEN;
                                    break 'outer;
                                }
                                (d, false)
                            }
                            // An EDGE_NODE, which shares type 14 with STYLE_NODE.
                            Ok(AnyNode::Style(edge)) => {
                                lr.dir = st.mem[edge.as_edge()].dir.get().or_confusion("LR")?;
                                (st.mem[edge.as_edge()].width, false)
                            }
                            Ok(AnyNode::Glue(g)) => {
                                let spec = st.mem[g].glue_ptr;
                                let order = st.mem[jb].glue_order;
                                match st.mem[jb].glue_sign.get() {
                                    Some(GlueSign::Stretching)
                                        if order == st.mem[spec].stretch_order
                                            && st.mem[spec].stretch != Scaled::ZERO =>
                                    {
                                        v = MAX_DIMEN;
                                    }
                                    Some(GlueSign::Shrinking)
                                        if order == st.mem[spec].shrink_order
                                            && st.mem[spec].shrink != Scaled::ZERO =>
                                    {
                                        v = MAX_DIMEN;
                                    }
                                    _ => {}
                                }
                                (st.mem[spec].width, st.mem[g].leaders().is_some())
                            }
                            Ok(AnyNode::Whatsit(wn)) => match wn.dims(&st.mem) {
                                Some(dims) => (dims.width, true),
                                None => (Scaled::ZERO, false),
                            },
                            _ => (Scaled::ZERO, false),
                        }
                    }
                };

                if found {
                    if v < MAX_DIMEN {
                        v += d;
                        w = v;
                    } else {
                        w = MAX_DIMEN;
                        break 'outer;
                    }
                } else if v < MAX_DIMEN {
                    v += d;
                }

                p = here.link(&st.mem);
            }

            if texxet {
                while !lr.ptr.is_none() {
                    st.mem.pop_lr(&mut lr.ptr);
                }
                if lr.problems != 0 {
                    w = MAX_DIMEN;
                }
            }

            let temp_head = TEMP_HEAD.link(&st.mem);
            st.mem.flush_node_list(temp_head)?;
        }

        let l;
        let s;
        if let Some(shape) = st.eqtb.par_shape() {
            let line = st
                .input
                .cur_list
                .prev_graf
                .saturating_add(2)
                .min(st.mem[shape].lines);
            let line = shape.line(&st.mem, line);
            s = line.indent;
            l = line.width;
        } else {
            let hang = st.eqtb.dimen_par(DimenPar::HangIndent);
            let hang_after = st.eqtb.int_par(IntPar::HangAfter);
            let prev_graf = st.input.cur_list.prev_graf;
            if hang != Scaled::ZERO
                && ((hang_after >= 0 && prev_graf.wrapping_add(2) > hang_after)
                    || prev_graf.wrapping_add(1) < hang_after.wrapping_neg())
            {
                l = st.eqtb.dimen_par(DimenPar::Hsize) - hang.abs();
                s = if hang > Scaled::ZERO {
                    hang
                } else {
                    Scaled::ZERO
                };
            } else {
                l = st.eqtb.dimen_par(DimenPar::Hsize);
                s = Scaled::ZERO;
            }
        }

        push_math(st, GroupCode::MathShift)?;
        st.input.cur_list.mode = Mode::math(false);
        define(st, false, IntPar::CurFam.loc(), -1)?;
        define(st, false, DimenPar::PreDisplaySize.loc(), w)?;
        st.input.cur_list.e_tex_aux = j;
        define(st, false, IntPar::PreDisplayDirection.loc(), x)?;
        define(st, false, DimenPar::DisplayWidth.loc(), l)?;
        define(st, false, DimenPar::DisplayIndent.loc(), s)?;

        if st.config.semantic_pagination_enabled {
            insert_special(st, "tdux:cs dmath")?;
        }

        let every_display = st.eqtb.tok_par(TokPar::EveryDisplay);
        if !every_display.is_null() {
            begin_token_list(st, every_display, TokenType::EveryDisplay)?;
        }
        if st.input.nest.len() == 1 {
            build_page(st)?;
        }
    } else {
        back_input(st)?;
        push_math(st, GroupCode::MathShift)?;
        define(st, false, IntPar::CurFam.loc(), -1)?;
        if st.shipout.insert_src_special_every_math {
            insert_src_special(st)?;
        }
        if st.config.semantic_pagination_enabled {
            insert_special(st, "tdux:cs math")?;
        }
        let every_math = st.eqtb.tok_par(TokPar::EveryMath);
        if !every_math.is_null() {
            begin_token_list(st, every_math, TokenType::EveryMath)?;
        }
    }
    Ok(())
}
