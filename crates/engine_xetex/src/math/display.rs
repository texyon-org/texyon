//! Entering and leaving math mode: `init_math`, `after_math` and displays.

use crate::cmd::Cmd;
use crate::consts::{
    AFTER, A_LEADERS, BEFORE, BEGIN_L_CODE, BEGIN_M_CODE, BOX_NODE_SIZE, DLIST, END_L_CODE,
    END_M_CODE, HMODE, INF_PENALTY, L_CODE, MAX_DIMEN, MMODE, R_CODE, SCRIPT_SIZE, TEXT_SIZE,
};
use crate::errors::Abort;
use crate::fonts::QUAD_CODE;
use crate::input::TokenType;
use crate::mem::{ADJUST_HEAD, GARBAGE, PRE_ADJUST_HEAD, TEMP_HEAD, ZERO_GLUE};
use crate::node::{AnyNode, BoxNode, GlueNode, NodePtr};
use crate::packaging::PackSpec;
use crate::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::state::{EngineState, EqtbState, FontState, GroupCode, MemState};
use crate::units::{Dir, GlueOrder, GlueSign, MemPtr, NodeType, Scaled};

use super::build::{define_word, fin_mlist};
use super::mlist::mlist_to_hlist;
use super::{MathStyle, SCRIPT_SCRIPT_SIZE};

const TOTAL_MATHSY_PARAMS: i32 = 22;
const TOTAL_MATHEX_PARAMS: i32 = 13;

pub(crate) fn resume_after_display(st: &mut EngineState) -> Result<(), Abort> {
    if st.save_stack.cur_group != GroupCode::MathShift {
        return Err(Abort::Confusion("display"));
    }

    crate::save::unsave(st)?;
    st.input.cur_list.prev_graf = st.input.cur_list.prev_graf.wrapping_add(3);
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = HMODE;
    st.input.cur_list.set_space_factor(1000);

    crate::main_control::start_paragraph_language(st);

    crate::scan::get_x_token(st)?;
    if st.scanner.cur_cmd != Cmd::Spacer {
        crate::input::back_input(st)?;
    }
    if st.input.nest.len() == 1 {
        crate::pagebuilder::build_page(st)?;
    }
    Ok(())
}

impl FontState {
    #[inline]
    fn sym_font_ok(&self, eqtb: &EqtbState, size: i32) -> bool {
        let f = eqtb.math_font(2 + size);
        self.font_params[f] >= TOTAL_MATHSY_PARAMS || self.font_is_ot_math(f)
    }

    #[inline]
    fn ext_font_ok(&self, eqtb: &EqtbState, size: i32) -> bool {
        let f = eqtb.math_font(3 + size);
        self.font_params[f] >= TOTAL_MATHEX_PARAMS || self.font_is_ot_math(f)
    }
}

impl MemState {
    /// Fills the fresh spec of `skip` with `w` minus the glue of `g`, negating its stretch and shrink.
    fn set_display_skip(&mut self, skip: NodePtr<GlueNode>, g: NodePtr<GlueNode>, w: Scaled) {
        let src = self[g].glue_ptr;
        let dst = self[skip].glue_ptr;
        self[dst].stretch_order = self[src].stretch_order;
        self[dst].shrink_order = self[src].shrink_order;
        self[dst].width = w - self[src].width;
        self[dst].stretch = -self[src].stretch;
        self[dst].shrink = -self[src].shrink;
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
        crate::packaging::append_to_vlist(st, orig)?;
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
        st.mem[bc].height = h;
        st.mem[bc].depth = dp;
        let s2 = s - st.mem[bc].shift_amount;
        d += s2;
        e = e + st.mem[bc].width - z - s2;
        Some(bc)
    } else {
        None
    };
    let mut b = copy.unwrap_or(orig);

    let (p, q);
    if st.mem[orig].subtype == DLIST {
        p = orig.ptr();
        q = p;
    } else {
        let mut r = st.mem[orig].list_ptr;
        st.mem.free_node(orig.ptr(), BOX_NODE_SIZE);
        if r.is_null() {
            return Err(Abort::Confusion("LR4"));
        }
        if x > 0 {
            p = r;
            q = st.mem.list_tail(r);
        } else {
            let mut rev = MemPtr::NULL;
            q = r;
            while !r.is_null() {
                let t = r.link(&st.mem);
                r.set_link(&mut st.mem, rev);
                rev = r;
                r = t;
            }
            p = rev;
        }
    }

    let (r, t) = match copy {
        None => (
            st.mem.new_kern(Scaled::ZERO)?.ptr(),
            st.mem.new_kern(Scaled::ZERO)?.ptr(),
        ),
        Some(bc) => {
            let r = st.mem[bc].list_ptr;
            (r, r.link(&st.mem))
        }
    };
    let u = st.mem.new_math(Scaled::ZERO, END_M_CODE)?.ptr();
    if let Ok(AnyNode::Glue(tg)) = st.mem.classify(t) {
        let skip = st.mem.new_skip_param(&st.eqtb, GluePar::RightSkip)?;
        q.set_link(&mut st.mem, skip.ptr());
        st.mem[skip].link = u;
        st.mem.set_display_skip(skip, tg, e);
        u.set_link(&mut st.mem, t);
    } else {
        st.mem.set_b32s1(t + 1, e.raw());
        t.set_link(&mut st.mem, u);
        q.set_link(&mut st.mem, t);
    }

    let u = st.mem.new_math(Scaled::ZERO, BEGIN_M_CODE)?.ptr();
    if let Ok(AnyNode::Glue(rg)) = st.mem.classify(r) {
        let skip = st.mem.new_skip_param(&st.eqtb, GluePar::LeftSkip)?;
        u.set_link(&mut st.mem, skip.ptr());
        st.mem[skip].link = p;
        st.mem.set_display_skip(skip, rg, d);
        r.set_link(&mut st.mem, u);
    } else {
        st.mem.set_b32s1(r + 1, d.raw());
        r.set_link(&mut st.mem, p);
        u.set_link(&mut st.mem, r);
        match copy {
            None => {
                b = crate::packaging::hpack(st, u, PackSpec::NATURAL)?;
                st.mem[b].shift_amount = s;
            }
            Some(bc) => st.mem[bc].list_ptr = u,
        }
    }
    crate::packaging::append_to_vlist(st, b)?;
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
    crate::output::print_err(st, msg);
    st.errors.help(&help);
    crate::errors::error(st)?;
    st.input.cur_list.flush_math(&mut st.mem)?;
    Ok(true)
}

pub(crate) fn check_display_end(st: &mut EngineState) -> Result<(), Abort> {
    crate::scan::get_x_token(st)?;
    if st.scanner.cur_cmd != Cmd::MathShift {
        crate::output::print_err(st, "Display math should end with $$");
        st.errors.help(&[
            "The `$' that I just saw supposedly matches a previous `$$'.",
            "So I shall assume that you typed `$$' both times.",
        ]);
        crate::input::back_error(st)?;
    }
    Ok(())
}

/// The box `init_math` saved for the paragraph line before a display, if any.
fn pre_display_box(st: &EngineState) -> Option<NodePtr<BoxNode>> {
    if st.input.cur_list.mode != MMODE {
        return None;
    }
    let j = st.input.cur_list.e_tex_aux.as_idx()?;
    st.mem.classify_box(j).ok().flatten()
}

pub(crate) fn after_math(st: &mut EngineState) -> Result<(), Abort> {
    let mut j = pre_display_box(st);
    let mut danger = check_math_fonts(st)?;
    let mut m = i32::from(st.input.cur_list.mode);
    let mut l = false;
    let mut p = fin_mlist(st, MemPtr::NULL)?;

    let a;
    if i32::from(st.input.cur_list.mode) == -m {
        check_display_end(st)?;
        st.scanner.cur_mlist = p;
        st.scanner.cur_style = MathStyle::TEXT;
        st.scanner.mlist_penalties = false;
        mlist_to_hlist(st)?;
        let eqno = crate::packaging::hpack(st, TEMP_HEAD.link(&st.mem), PackSpec::NATURAL)?;
        st.mem[eqno].subtype = DLIST;
        a = Some(eqno);
        crate::save::unsave(st)?;
        st.save_stack.save_ptr -= 1;
        if st.save_stack.saved(0) == 1 {
            l = true;
        }

        j = pre_display_box(st);
        danger = check_math_fonts(st)?;
        m = i32::from(st.input.cur_list.mode);
        p = fin_mlist(st, MemPtr::NULL)?;
    } else {
        a = None;
    }

    if m < 0 {
        let math_surround = st.eqtb.dimen_par(DimenPar::MathSurround);
        let node = st.mem.new_math(math_surround, BEFORE)?;
        st.input.cur_list.tail_append(&mut st.mem, node.ptr());
        st.scanner.cur_mlist = p;
        st.scanner.cur_style = MathStyle::TEXT;
        st.scanner.mlist_penalties = st.input.cur_list.mode > 0;
        mlist_to_hlist(st)?;
        let hlist = TEMP_HEAD.link(&st.mem);
        let cl = &mut st.input.cur_list;
        cl.tail.set_link(&mut st.mem, hlist);
        while !cl.tail.link(&st.mem).is_null() {
            cl.tail = cl.tail.link(&st.mem);
        }
        let node = st.mem.new_math(math_surround, AFTER)?;
        st.input.cur_list.tail_append(&mut st.mem, node.ptr());
        st.input.cur_list.set_space_factor(1000);

        if st.config.semantic_pagination_enabled {
            crate::ext::insert_special(st, "tdux:ce math")?;
        }

        crate::save::unsave(st)?;
        return Ok(());
    }

    if a.is_none() {
        check_display_end(st)?;
    }
    st.scanner.cur_mlist = p;
    st.scanner.cur_style = MathStyle::DISPLAY;
    st.scanner.mlist_penalties = false;
    mlist_to_hlist(st)?;
    let mut p = TEMP_HEAD.link(&st.mem);
    st.scanner.adjust_tail = ADJUST_HEAD;
    st.scanner.pre_adjust_tail = PRE_ADJUST_HEAD;
    let mut b = crate::packaging::hpack(st, p, PackSpec::NATURAL)?;
    p = st.mem[b].list_ptr;
    let t = st.scanner.adjust_tail;
    st.scanner.adjust_tail = MemPtr::NULL;
    let pre_t = st.scanner.pre_adjust_tail;
    st.scanner.pre_adjust_tail = MemPtr::NULL;
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
            && (w - st.scanner.total_shrink[GlueOrder::Normal] + q <= z
                || st.scanner.total_shrink[GlueOrder::Fil] != Scaled::ZERO
                || st.scanner.total_shrink[GlueOrder::Fill] != Scaled::ZERO
                || st.scanner.total_shrink[GlueOrder::Filll] != Scaled::ZERO)
        {
            st.mem.free_node(b.ptr(), BOX_NODE_SIZE);
            b = crate::packaging::hpack(st, p, PackSpec::Exactly(z - q))?;
        } else {
            e = Scaled::ZERO;
            if w > z {
                st.mem.free_node(b.ptr(), BOX_NODE_SIZE);
                b = crate::packaging::hpack(st, p, PackSpec::Exactly(z))?;
            }
        }
        w = st.mem[b].width;
    }
    st.mem[b].subtype = DLIST;
    let mut d = (z - w).half();
    if e > Scaled::ZERO && d < e * 2 {
        d = (z - w - e).half();
        if let Some(first) = p.as_idx() {
            if !st.mem.is_char_node(p) && st.mem.node_type(first) == Ok(NodeType::Glue) {
                d = Scaled::ZERO;
            }
        }
    }

    let pen = st
        .mem
        .new_penalty(st.eqtb.int_par(IntPar::PreDisplayPenalty))?;
    st.input.cur_list.tail_append(&mut st.mem, pen.ptr());

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
        st.input.cur_list.tail_append(&mut st.mem, pen.ptr());
    } else {
        let glue = st.mem.new_param_glue(&st.eqtb, g1)?;
        st.input.cur_list.tail_append(&mut st.mem, glue.ptr());
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
        b = crate::packaging::hpack(st, b.ptr(), PackSpec::NATURAL)?;
    }

    app_display(st, j, b, d)?;

    if let Some(eqno) = a.filter(|_| e == Scaled::ZERO && !l) {
        let pen = st.mem.new_penalty(INF_PENALTY)?;
        st.input.cur_list.tail_append(&mut st.mem, pen.ptr());
        let a_width = st.mem[eqno].width;
        app_display(st, j, eqno, z - a_width)?;
        g2 = None;
    }

    let cl = &mut st.input.cur_list;
    if t != ADJUST_HEAD {
        let adjusts = ADJUST_HEAD.link(&st.mem);
        cl.tail.set_link(&mut st.mem, adjusts);
        cl.tail = t;
    }
    if pre_t != PRE_ADJUST_HEAD {
        let pre_adjusts = PRE_ADJUST_HEAD.link(&st.mem);
        cl.tail.set_link(&mut st.mem, pre_adjusts);
        cl.tail = pre_t;
    }

    let pen = st
        .mem
        .new_penalty(st.eqtb.int_par(IntPar::PostDisplayPenalty))?;
    st.input.cur_list.tail_append(&mut st.mem, pen.ptr());

    if let Some(g2) = g2 {
        let glue = st.mem.new_param_glue(&st.eqtb, g2)?;
        st.input.cur_list.tail_append(&mut st.mem, glue.ptr());
    }

    if let Some(j) = j {
        st.mem.flush_node_list(j.ptr())?;
    }

    if st.config.semantic_pagination_enabled {
        crate::ext::insert_special(st, "tdux:ce dmath")?;
    }

    resume_after_display(st)
}

/// TeX's `\predisplaydirection` for the paragraph just ended.
fn pre_display_direction(st: &EngineState) -> i32 {
    let aux = st.input.cur_list.e_tex_aux;
    if aux.is_null() {
        0
    } else if aux.info(&st.mem) >= R_CODE.into() {
        -1
    } else {
        1
    }
}

/// A kern, or the parameter glue when that skip is nonzero.
fn side_skip(st: &mut EngineState, param: GluePar) -> Result<MemPtr, Abort> {
    if st.eqtb.glue_par(param) == ZERO_GLUE {
        st.mem.new_kern(Scaled::ZERO).map(|k| k.ptr())
    } else {
        st.mem.new_param_glue(&st.eqtb, param).map(|g| g.ptr())
    }
}

pub(crate) fn init_math(st: &mut EngineState) -> Result<(), Abort> {
    crate::scan::get_token(st)?;

    if st.scanner.cur_cmd == Cmd::MathShift && st.input.cur_list.mode > 0 {
        let mut j = MemPtr::NULL;
        let mut w = -MAX_DIMEN;
        let x;

        if st.input.cur_list.head == st.input.cur_list.tail {
            st.mem.pop_nest(&mut st.input)?;
            x = pre_display_direction(st);
        } else {
            let jb = crate::linebreak::line_break(st, true)?;

            let right = side_skip(st, GluePar::RightSkip)?;
            let left = side_skip(st, GluePar::LeftSkip)?;
            left.set_link(&mut st.mem, right);

            let jbox = st.mem.new_null_box()?;
            j = jbox.ptr();
            let mem = &mut st.mem;
            mem[jbox].width = mem[jb].width;
            mem[jbox].shift_amount = mem[jb].shift_amount;
            mem[jbox].list_ptr = left;
            mem[jbox].glue_order = mem[jb].glue_order;
            mem[jbox].glue_sign = mem[jb].glue_sign;
            mem[jbox].glue_set = mem[jb].glue_set;

            let mut v = st.mem[jb].shift_amount;
            x = pre_display_direction(st);

            let mut p;
            if x >= 0 {
                p = st.mem[jb].list_ptr;
                TEMP_HEAD.set_link(&mut st.mem, MemPtr::NULL);
            } else {
                v = -v - st.mem[jb].width;
                let begin = st.mem.new_math(Scaled::ZERO, BEGIN_L_CODE)?.ptr();
                TEMP_HEAD.set_link(&mut st.mem, begin);
                let end = st.mem.new_math(Scaled::ZERO, END_L_CODE)?.ptr();
                let list = st.mem[jb].list_ptr;
                st.mem.just_copy(list, begin, end)?;
                st.scanner.cur_dir = Dir::Rtl;
                p = begin;
            }

            let cur_font = st.eqtb.cur_font();
            v += st.fonts.param(cur_font, QUAD_CODE) * 2;

            let texxet = st.eqtb.int_par(IntPar::Texxet) > 0;
            if texxet {
                st.mem.push_lr(&mut st.scanner.lr_ptr, BEFORE.into())?;
            }

            'outer: while !p.is_null() {
                let (d, found) = 'reswitch: loop {
                    if let Some(ch) = st.mem.as_char(p) {
                        let f = ch.font(&st.mem);
                        let c = st.fonts.effective_char(f, ch.character(&st.mem));
                        let d = st.fonts.char_width(f, st.fonts.char_info(f, c));
                        break (d, true);
                    }
                    break match st.mem.classify(p) {
                        Ok(node @ (AnyNode::Box(_) | AnyNode::Rule(_))) => {
                            (node.hlist_width(&st.mem), true)
                        }
                        Ok(AnyNode::Ligature(lig)) => {
                            st.mem.set_word(GARBAGE, st.mem.word(lig.char_word().idx()));
                            let next = lig.link(&st.mem);
                            GARBAGE.set_link(&mut st.mem, next);
                            p = GARBAGE;
                            st.fonts.xtx_ligature_present = true;
                            continue 'reswitch;
                        }
                        Ok(AnyNode::Kern(k)) => (st.mem[k].width, false),
                        Ok(AnyNode::MarginKern(k)) => (st.mem[k].width, false),
                        Ok(AnyNode::Math(m)) => {
                            let d = st.mem[m].width;
                            let subtype = st.mem[m].subtype;
                            if texxet {
                                if subtype & 1 != 0 {
                                    if st.scanner.lr_ptr.info(&st.mem)
                                        == crate::mem::end_lr_type(subtype)
                                    {
                                        st.mem.pop_lr(&mut st.scanner.lr_ptr);
                                    } else if subtype > L_CODE {
                                        w = MAX_DIMEN;
                                        break 'outer;
                                    }
                                } else {
                                    st.mem.push_lr(
                                        &mut st.scanner.lr_ptr,
                                        crate::mem::end_lr_type(subtype),
                                    )?;
                                    if Dir::from_code(subtype / R_CODE) != Some(st.scanner.cur_dir)
                                    {
                                        st.mem.just_reverse(&mut st.scanner, p)?;
                                        p = TEMP_HEAD;
                                    }
                                }
                            } else if subtype >= L_CODE {
                                w = MAX_DIMEN;
                                break 'outer;
                            }
                            (d, false)
                        }
                        // An EDGE_NODE, which shares type 14 with STYLE_NODE.
                        Ok(AnyNode::Style(edge)) => {
                            st.scanner.cur_dir = Dir::from_code(st.mem[edge].subtype)
                                .ok_or(Abort::Confusion("LR"))?;
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
                            (st.mem[spec].width, st.mem[g].subtype >= A_LEADERS)
                        }
                        Ok(AnyNode::Whatsit(wn))
                            if wn
                                .whatsit_subtype(&st.mem)
                                .is_some_and(|s| s.has_dimensions()) =>
                        {
                            (wn.width(&st.mem), true)
                        }
                        _ => (Scaled::ZERO, false),
                    };
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

                p = p.link(&st.mem);
            }

            if texxet {
                while !st.scanner.lr_ptr.is_null() {
                    st.mem.pop_lr(&mut st.scanner.lr_ptr);
                }
                if st.scanner.lr_problems != 0 {
                    w = MAX_DIMEN;
                    st.scanner.lr_problems = 0;
                }
            }

            st.scanner.cur_dir = Dir::Ltr;
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
                .wrapping_add(2)
                .min(shape.lines(&st.mem));
            s = shape.indent(&st.mem, line);
            l = shape.line_width(&st.mem, line);
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

        crate::main_control::push_math(st, GroupCode::MathShift)?;
        st.input.cur_list.mode = MMODE;
        define_word(st, IntPar::CurFam.loc(), -1)?;
        define_word(st, DimenPar::PreDisplaySize.loc(), w.raw())?;
        st.input.cur_list.e_tex_aux = j;
        define_word(st, IntPar::PreDisplayDirection.loc(), x)?;
        define_word(st, DimenPar::DisplayWidth.loc(), l.raw())?;
        define_word(st, DimenPar::DisplayIndent.loc(), s.raw())?;

        if st.config.semantic_pagination_enabled {
            crate::ext::insert_special(st, "tdux:cs dmath")?;
        }

        let every_display = st.eqtb.tok_par(TokPar::EveryDisplay);
        if !every_display.is_null() {
            crate::input::begin_token_list(st, every_display, TokenType::EveryDisplay)?;
        }
        if st.input.nest.len() == 1 {
            crate::pagebuilder::build_page(st)?;
        }
    } else {
        crate::input::back_input(st)?;
        crate::main_control::push_math(st, GroupCode::MathShift)?;
        define_word(st, IntPar::CurFam.loc(), -1)?;
        if st.shipout.insert_src_special_every_math {
            crate::ext::insert_src_special(st)?;
        }
        if st.config.semantic_pagination_enabled {
            crate::ext::insert_special(st, "tdux:cs math")?;
        }
        let every_math = st.eqtb.tok_par(TokPar::EveryMath);
        if !every_math.is_null() {
            crate::input::begin_token_list(st, every_math, TokenType::EveryMath)?;
        }
    }
    Ok(())
}
