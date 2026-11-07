//! The paragraph builder: `line_break`, `post_line_break`, `hyphenate` and
//! character protrusion.

use crate::consts::{
    AWFUL_BAD, EJECT_PENALTY, EXPLICIT, INF_BAD, INF_PENALTY, L_CODE, MAX_DIMEN, MAX_HALFWORD,
    NULL_FONT,
};
use crate::errors::Abort;
use crate::mem::{ACTIVE_LIST, TEMP_HEAD};
use crate::node::{
    link_of, ActiveNode, AnyNode, BoxNode, CharNode, DeltaNode, DiscNode, GlueNode, GlueSpecNode,
    NodePtr, PassiveNode, WhatsitVariant,
};
use crate::params::{DimenPar, GluePar, IntPar};
use crate::state::{
    ConfigState, EngineState, EqtbState, ErrorState, FontState, LinebreakState, MemState,
    ScannerState,
};
use crate::units::{FontId, GlueOrder, MemAddr, MemPtr, NodeType, Scaled};

mod hyphenate;
mod locale;
mod post;
mod prehyph;
mod protrusion;

pub(crate) use self::locale::do_locale_linebreaks;
use self::post::post_line_break;
use self::prehyph::{set_language, try_hyphenate};

const LAST_ACTIVE: MemPtr = ACTIVE_LIST;

const DELTA_NODE: u16 = NodeType::Rule.as_raw();

pub(crate) const UNHYPHENATED: i16 = 0;
pub(crate) const HYPHENATED: i16 = 1;
const VERY_LOOSE_FIT: usize = 0;
const LOOSE_FIT: usize = 1;
const DECENT_FIT: usize = 2;
const TIGHT_FIT: usize = 3;

#[inline]
fn odd(v: u8) -> bool {
    v % 2 == 1
}

impl FontState {
    #[inline]
    fn effective_char_width(&mut self, f: FontId, c: u16) -> Scaled {
        let ec = self.effective_char(f, c);
        let info = self.char_info(f, ec);
        self.char_width(f, info)
    }
}

/// A node on the active list, which carries no `NODE_type` of its own.
#[inline]
fn active(p: impl MemAddr) -> NodePtr<ActiveNode> {
    NodePtr::at(p)
}

#[inline]
fn delta_node(p: impl MemAddr) -> NodePtr<DeltaNode> {
    NodePtr::at(p)
}

#[inline]
fn passive_node(p: impl MemAddr) -> NodePtr<PassiveNode> {
    NodePtr::at(p)
}

/// An active-list entry. Its type word holds the break type (0/1) or
/// `DELTA_NODE` (2), which `classify()` would misread as HList/VList/Rule.
#[derive(Copy, Clone)]
enum ActiveOrDelta {
    Active(NodePtr<ActiveNode>),
    Delta(NodePtr<DeltaNode>),
}

/// A delta node's six components, indexed like `active_width` (slot 0 unused).
#[inline]
fn delta_widths(d: NodePtr<DeltaNode>, mem: &MemState) -> [Scaled; 7] {
    [
        Scaled::ZERO,
        mem[d].dwidth,
        mem[d].dstretch0,
        mem[d].dstretch1,
        mem[d].dstretch2,
        mem[d].dstretch3,
        mem[d].dshrink,
    ]
}

#[inline]
fn set_delta_widths(d: NodePtr<DeltaNode>, mem: &mut MemState, w: &[Scaled; 7]) {
    mem[d].dwidth = w[1];
    mem[d].dstretch0 = w[2];
    mem[d].dstretch1 = w[3];
    mem[d].dstretch2 = w[4];
    mem[d].dstretch3 = w[5];
    mem[d].dshrink = w[6];
}

impl MemState {
    #[inline]
    fn has_infinite_shrink(&self, spec: NodePtr<GlueSpecNode>) -> bool {
        self[spec].shrink_order != GlueOrder::Normal && self[spec].shrink != Scaled::ZERO
    }

    #[inline]
    fn classify_active_or_delta(&self, p: impl MemAddr + Copy) -> ActiveOrDelta {
        if self[active(p)].break_type == DELTA_NODE {
            ActiveOrDelta::Delta(delta_node(p))
        } else {
            ActiveOrDelta::Active(active(p))
        }
    }

    /// A new active node before `link`, its `\lastlinefit` fields zeroed.
    fn new_active_node(&mut self, link: MemPtr) -> Result<NodePtr<ActiveNode>, Abort> {
        let p = NodePtr::<ActiveNode>::alloc(self)?;
        let n = &mut self[p];
        n.link = link;
        n.shortfall = Scaled::ZERO;
        n.glue = Scaled::ZERO;
        Ok(p)
    }

    /// A new delta node before `link` holding `w[1..=6]`.
    fn new_delta_node(
        &mut self,
        link: MemPtr,
        w: &[Scaled; 7],
    ) -> Result<NodePtr<DeltaNode>, Abort> {
        let p = NodePtr::<DeltaNode>::alloc(self)?;
        self[p].link = link;
        self[p].subtype = 0;
        set_delta_widths(p, self, w);
        Ok(p)
    }
}

impl MemState {
    /// A copy of `spec` with finite shrink, releasing `spec`.
    pub(crate) fn finite_shrink(
        &mut self,
        spec: NodePtr<GlueSpecNode>,
    ) -> Result<NodePtr<GlueSpecNode>, Abort> {
        let q = self.new_spec(spec)?;
        self[q].shrink_order = GlueOrder::Normal.into();
        self.delete_glue_ref(spec);
        Ok(q)
    }
}

impl MemState {
    fn start_linebreak_pass(
        &mut self,
        scanner: &mut ScannerState,
        linebreak: &mut LinebreakState,
        errors: &mut ErrorState,
        prev_graf: i32,
    ) -> Result<(), Abort> {
        let q = self.new_active_node(LAST_ACTIVE)?;
        let n = &mut self[q];
        n.break_type = UNHYPHENATED as u16;
        n.fitness = DECENT_FIT as u16;
        n.break_node = MemPtr::NULL;
        n.line_number = prev_graf.wrapping_add(1);
        n.total_demerits = 0;
        ACTIVE_LIST.set_link(self, q.ptr());

        scanner.active_width[1..].copy_from_slice(&linebreak.background[1..]);
        linebreak.passive = None;
        errors.font_in_short_display = NULL_FONT;
        linebreak.cur_p = TEMP_HEAD.link(self);
        linebreak.global_prev_p = linebreak.cur_p;
        linebreak.first_p = linebreak.cur_p;
        Ok(())
    }

    fn free_break_lists(&mut self, linebreak: &LinebreakState) {
        let mut q = ACTIVE_LIST.link(self);
        while q != LAST_ACTIVE {
            let next = q.link(self);
            match self.classify_active_or_delta(q) {
                ActiveOrDelta::Delta(d) => d.free(self),
                ActiveOrDelta::Active(a) => a.free(self),
            }
            q = next;
        }

        let mut passive = linebreak.passive;
        while let Some(p) = passive {
            passive = p.next(self);
            p.free(self);
        }
    }

    /// Pick the best active node and set `best_line`. `None` when the result
    /// misses `\looseness` and another pass is due.
    fn choose_best_break(
        &self,
        linebreak: &mut LinebreakState,
        eqtb: &EqtbState,
    ) -> Option<NodePtr<ActiveNode>> {
        if ACTIVE_LIST.link(self) == LAST_ACTIVE {
            return None;
        }

        let mut r = ACTIVE_LIST.link(self);
        let mut fewest_demerits = MAX_HALFWORD;
        let mut best = None;
        loop {
            if let ActiveOrDelta::Active(a) = self.classify_active_or_delta(r) {
                if self[a].total_demerits < fewest_demerits {
                    fewest_demerits = self[a].total_demerits;
                    best = Some(a);
                }
            }
            r = r.link(self);
            if r == LAST_ACTIVE {
                break;
            }
        }
        let mut best = best.expect("an active node has finite demerits");
        linebreak.best_line = self[best].line_number;

        let looseness = eqtb.int_par(IntPar::Looseness);
        if looseness == 0 {
            return Some(best);
        }

        let mut r = ACTIVE_LIST.link(self);
        let mut actual_looseness = 0;
        loop {
            if let ActiveOrDelta::Active(a) = self.classify_active_or_delta(r) {
                let line_diff = self[a].line_number.wrapping_sub(linebreak.best_line);
                if (line_diff < actual_looseness && looseness <= line_diff)
                    || (line_diff > actual_looseness && looseness >= line_diff)
                {
                    best = a;
                    actual_looseness = line_diff;
                    fewest_demerits = self[a].total_demerits;
                } else if line_diff == actual_looseness && self[a].total_demerits < fewest_demerits
                {
                    best = a;
                    fewest_demerits = self[a].total_demerits;
                }
            }
            r = r.link(self);
            if r == LAST_ACTIVE {
                break;
            }
        }

        linebreak.best_line = self[best].line_number;
        (actual_looseness == looseness || linebreak.final_pass).then_some(best)
    }

    /// Stretch the `\parfillskip` of the last line by the `\lastlinefit` adjustment.
    fn finish_last_line_fit(
        &mut self,
        linebreak: &mut LinebreakState,
        best: NodePtr<ActiveNode>,
        last_line_fill: NodePtr<GlueNode>,
    ) -> Result<(), Abort> {
        if !linebreak.do_last_line_fit {
            return Ok(());
        }

        if self[best].shortfall == Scaled::ZERO {
            linebreak.do_last_line_fit = false;
            return Ok(());
        }

        let old_spec = self[last_line_fill].glue_ptr;
        let spec = self.new_spec(old_spec)?;
        self.delete_glue_ref(old_spec);

        let extra = self[best].shortfall - self[best].glue;
        self[spec].width += extra;
        self[spec].stretch = Scaled::ZERO;
        self[last_line_fill].glue_ptr = spec;
        Ok(())
    }
}

/// Make `spec`'s shrink finite, reporting it on the paragraph's first offence.
fn finite_shrink(
    st: &mut EngineState,
    spec: NodePtr<GlueSpecNode>,
) -> Result<NodePtr<GlueSpecNode>, Abort> {
    if st.errors.no_shrink_error_yet {
        st.errors.no_shrink_error_yet = false;
        crate::output::print_err(st, "Infinite glue shrinkage found in a paragraph");
        st.errors.help(&[
            "The paragraph just ended includes some glue that has",
            r"infinite shrinkability, e.g., `\hskip 0pt minus 1fil'.",
            "Such glue doesn't belong there---it allows a paragraph",
            "of any length to fit on one line. But it's safe to proceed,",
            "since the offensive shrinkability has been made finite.",
        ]);
        crate::errors::error(st)?;
    }
    st.mem.finite_shrink(spec)
}

#[inline]
fn line_break_try_break(
    st: &mut EngineState,
    pi: i32,
    break_type: i16,
    disc: Option<NodePtr<DiscNode>>,
) -> Result<(), Abort> {
    let s = &mut *st;
    s.mem.try_break(
        &mut s.scanner,
        &mut s.linebreak,
        &s.eqtb,
        &mut s.fonts,
        &s.config,
        &mut s.errors,
        pi,
        break_type,
        disc,
    )
}

/// TeX's `kern_break`: a kern or math node followed by glue is a legal breakpoint.
#[inline]
fn kern_break(st: &mut EngineState, next: MemPtr, auto_breaking: bool) -> Result<(), Abort> {
    if !st.mem.is_char_node(next)
        && auto_breaking
        && matches!(st.mem.classify(next), Ok(AnyNode::Glue(_)))
    {
        line_break_try_break(st, 0, UNHYPHENATED, None)?;
    }
    Ok(())
}

impl MemState {
    /// The width of a node in a discretionary's list. `Err(true)` for a whatsit
    /// without dimensions, `Err(false)` for any other node that cannot occur there.
    #[inline]
    fn disc_node_width(&self, fonts: &mut FontState, p: MemPtr) -> Result<Scaled, bool> {
        if let Some(ch) = self.as_char(p) {
            return Ok(fonts.effective_char_width(self[ch].font, self[ch].character));
        }

        match self.classify(p) {
            Ok(AnyNode::Ligature(lig)) => {
                let f = self[lig].font;
                fonts.xtx_ligature_present = true;
                Ok(fonts.effective_char_width(f, self[lig].character))
            }
            Ok(node @ (AnyNode::Box(_) | AnyNode::Rule(_) | AnyNode::Kern(_))) => {
                Ok(node.hlist_width(self))
            }
            Ok(AnyNode::Whatsit(w)) => w.sized(self).map(|s| self[s].width).ok_or(true),
            _ => Err(false),
        }
    }
}

/// Break the current paragraph into lines, returning the last line's box.
pub(crate) fn line_break(st: &mut EngineState, d: bool) -> Result<NodePtr<BoxNode>, Abort> {
    st.scanner.pack_begin_line = st.input.cur_list.mode_line;
    let first = st.input.cur_list.head.link(&st.mem);
    TEMP_HEAD.set_link(&mut st.mem, first);

    let tail = st.input.cur_list.tail;
    let tail_glue = if st.mem.is_char_node(tail) {
        None
    } else if let Ok(AnyNode::Glue(g)) = st.mem.classify(tail) {
        Some(g)
    } else {
        None
    };
    match tail_glue {
        None => {
            let p = st.mem.new_penalty(INF_PENALTY)?;
            tail.set_link(&mut st.mem, p.ptr());
            st.input.cur_list.tail = p.ptr();
        }
        Some(tail_glue) => {
            // The trailing glue becomes the infinite penalty in place.
            let spec = st.mem[tail_glue].glue_ptr;
            let leaders = st.mem[tail_glue].leader_ptr;
            let tail_penalty = tail_glue.into_penalty(&mut st.mem);
            st.mem.delete_glue_ref(spec);
            st.mem.flush_node_list(leaders)?;
            st.mem[tail_penalty].penalty = INF_PENALTY;
        }
    }

    let last_line_fill = st.mem.new_param_glue(&st.eqtb, GluePar::ParFillSkip)?;
    st.input
        .cur_list
        .tail
        .set_link(&mut st.mem, last_line_fill.ptr());
    st.hyph.init_l_hyf = st.input.cur_list.prev_graf / 0x0040_0000;
    st.hyph.init_r_hyf = (st.input.cur_list.prev_graf / 65_536) % 64;
    let init_cur_lang = (st.input.cur_list.prev_graf % 65_536) as u8;

    st.mem.pop_nest(&mut st.input)?;
    st.errors.no_shrink_error_yet = true;

    for n in [GluePar::LeftSkip, GluePar::RightSkip] {
        let spec = st.eqtb.glue_par(n);
        if st.mem.has_infinite_shrink(spec) {
            let spec = finite_shrink(st, spec)?;
            st.eqtb.set_glue_par(n, spec);
        }
    }

    let left_skip = st.eqtb.glue_par(GluePar::LeftSkip);
    let right_skip = st.eqtb.glue_par(GluePar::RightSkip);
    let mem = &st.mem;
    let background = &mut st.linebreak.background;
    background[1] = mem[left_skip].width + mem[right_skip].width;
    background[2..=5].fill(Scaled::ZERO);
    background[2 + mem[left_skip].stretch_order.index()] = mem[left_skip].stretch;
    background[2 + mem[right_skip].stretch_order.index()] += mem[right_skip].stretch;
    background[6] = mem[left_skip].shrink + mem[right_skip].shrink;

    st.linebreak.do_last_line_fit = false;
    if st.eqtb.int_par(IntPar::LastLineFit) > 0 {
        let spec = st.mem[last_line_fill].glue_ptr;
        let lb = &mut st.linebreak;
        if st.mem[spec].stretch > Scaled::ZERO
            && st.mem[spec].stretch_order > GlueOrder::Normal
            && lb.background[3..=5] == [Scaled::ZERO; 3]
        {
            lb.do_last_line_fit = true;
            lb.fill_width = [Scaled::ZERO; 3];
            let order = st.mem[spec].stretch_order;
            lb.fill_width[order.index() - 1] = st.mem[spec].stretch;
        }
    }

    st.linebreak.minimum_demerits = AWFUL_BAD;
    st.linebreak.minimal_demerits = [AWFUL_BAD; 4];

    let par_shape = st.eqtb.par_shape();
    let lb = &mut st.linebreak;
    if let Some(shape) = par_shape {
        let lines = st.mem[shape].lines;
        let last = shape.line(&st.mem, lines);
        lb.last_special_line = lines - 1;
        lb.second_width = last.width;
        lb.second_indent = last.indent;
    } else {
        let hang_indent = st.eqtb.dimen_par(DimenPar::HangIndent);
        let hsize = st.eqtb.dimen_par(DimenPar::Hsize);
        if hang_indent == Scaled::ZERO {
            lb.last_special_line = 0;
            lb.second_width = hsize;
            lb.second_indent = Scaled::ZERO;
        } else {
            let hang_after = st.eqtb.int_par(IntPar::HangAfter);
            lb.last_special_line = hang_after.wrapping_abs();
            if hang_after < 0 {
                lb.first_width = hsize - hang_indent.abs();
                lb.first_indent = hang_indent.max(Scaled::ZERO);
                lb.second_width = hsize;
                lb.second_indent = Scaled::ZERO;
            } else {
                lb.first_width = hsize;
                lb.first_indent = Scaled::ZERO;
                lb.second_width = hsize - hang_indent.abs();
                lb.second_indent = hang_indent.max(Scaled::ZERO);
            }
        }
    }

    lb.easy_line = if st.eqtb.int_par(IntPar::Looseness) == 0 {
        lb.last_special_line
    } else {
        MAX_HALFWORD
    };

    lb.threshold = st.eqtb.int_par(IntPar::Pretolerance);
    let mut second_pass;
    if lb.threshold >= 0 {
        second_pass = false;
        lb.final_pass = false;
    } else {
        lb.threshold = st.eqtb.int_par(IntPar::Tolerance);
        second_pass = true;
        lb.final_pass = st.eqtb.dimen_par(DimenPar::EmergencyStretch) <= Scaled::ZERO;
    }

    let best = loop {
        st.linebreak.threshold = st.linebreak.threshold.min(INF_BAD);

        if second_pass {
            if st.hyph.trie_not_ready {
                st.hyph.init_trie()?;
            }
            st.hyph.cur_lang = init_cur_lang;
            st.hyph.l_hyf = st.hyph.init_l_hyf;
            st.hyph.r_hyf = st.hyph.init_r_hyf;
            st.hyph.set_hyph_index();
        }

        st.mem.start_linebreak_pass(
            &mut st.scanner,
            &mut st.linebreak,
            &mut st.errors,
            st.input.cur_list.prev_graf,
        )?;
        let mut auto_breaking = true;
        // Whether glue after the previous node is a legal breakpoint.
        let mut prev_breakable = false;

        while !st.linebreak.cur_p.is_null() && ACTIVE_LIST.link(&st.mem) != LAST_ACTIVE {
            let mut cur = st.linebreak.cur_p;

            if st.mem.is_char_node(cur) {
                prev_breakable = true;
                st.linebreak.global_prev_p = cur;
                loop {
                    let ch = NodePtr::<CharNode>::at(cur);
                    let f = st.mem[ch].font;
                    let c = st.mem[ch].character;
                    st.scanner.active_width[1] += st.fonts.effective_char_width(f, c);
                    cur = cur.link(&st.mem);
                    if !st.mem.is_char_node(cur) {
                        break;
                    }
                }
                st.linebreak.cur_p = cur;
            }

            let Ok(node) = st.mem.classify(cur) else {
                return Err(Abort::Confusion("paragraph"));
            };
            match node {
                AnyNode::Box(_) | AnyNode::Rule(_) => {
                    st.scanner.active_width[1] += node.hlist_width(&st.mem);
                }
                AnyNode::Whatsit(w) => {
                    if let Some(WhatsitVariant::Language(language)) = w.variant(&st.mem) {
                        set_language(st, language);
                    } else if let Some(sized) = w.sized(&st.mem) {
                        st.scanner.active_width[1] += st.mem[sized].width;
                    }
                }
                AnyNode::Glue(cur_glue) => {
                    if auto_breaking && prev_breakable {
                        line_break_try_break(st, 0, UNHYPHENATED, None)?;
                    }

                    let mut spec = st.mem[cur_glue].glue_ptr;
                    if st.mem.has_infinite_shrink(spec) {
                        spec = finite_shrink(st, spec)?;
                        st.mem[cur_glue].glue_ptr = spec;
                    }
                    let aw = &mut st.scanner.active_width;
                    aw[1] += st.mem[spec].width;
                    aw[2 + st.mem[spec].stretch_order.index()] += st.mem[spec].stretch;
                    aw[6] += st.mem[spec].shrink;

                    if second_pass && auto_breaking {
                        try_hyphenate(st, cur)?;
                    }
                }
                AnyNode::Kern(k) => {
                    if st.mem[k].subtype == EXPLICIT {
                        kern_break(st, st.mem[k].link, auto_breaking)?;
                    }
                    st.scanner.active_width[1] += st.mem[k].width;
                }
                AnyNode::Ligature(lig) => {
                    let f = st.mem[lig].font;
                    st.fonts.xtx_ligature_present = true;
                    let c = st.mem[lig].character;
                    st.scanner.active_width[1] += st.fonts.effective_char_width(f, c);
                }
                AnyNode::Disc(d) => {
                    let pre_break = st.mem[d].pre_break;
                    st.linebreak.disc_width = Scaled::ZERO;
                    if pre_break.is_null() {
                        let pi = st.eqtb.int_par(IntPar::ExHyphenPenalty);
                        line_break_try_break(st, pi, HYPHENATED, Some(d))?;
                    } else {
                        let mut s = pre_break;
                        while !s.is_null() {
                            st.linebreak.disc_width += st
                                .mem
                                .disc_node_width(&mut st.fonts, s)
                                .map_err(|_| Abort::Confusion("disc3"))?;
                            s = s.link(&st.mem);
                        }
                        st.scanner.active_width[1] += st.linebreak.disc_width;
                        let pi = st.eqtb.int_par(IntPar::HyphenPenalty);
                        line_break_try_break(st, pi, HYPHENATED, Some(d))?;
                        st.scanner.active_width[1] -= st.linebreak.disc_width;
                    }

                    let mut s = st.mem[d].link;
                    for _ in 0..st.mem[d].subtype {
                        st.scanner.active_width[1] += st
                            .mem
                            .disc_node_width(&mut st.fonts, s)
                            .map_err(|_| Abort::Confusion("disc4"))?;
                        s = s.link(&st.mem);
                    }

                    prev_breakable = true;
                    st.linebreak.global_prev_p = cur;
                    st.linebreak.cur_p = s;
                    continue;
                }
                AnyNode::Math(m) => {
                    if st.mem[m].subtype < L_CODE {
                        auto_breaking = odd(st.mem[m].subtype as u8);
                    }
                    kern_break(st, st.mem[m].link, auto_breaking)?;
                    st.scanner.active_width[1] += st.mem[m].width;
                }
                AnyNode::Penalty(pen) => {
                    line_break_try_break(st, st.mem[pen].penalty, UNHYPHENATED, None)?;
                }
                AnyNode::Mark(_) | AnyNode::Ins(_) | AnyNode::Adjust(_) => {}
                _ => return Err(Abort::Confusion("paragraph")),
            }

            prev_breakable = node.precedes_break()
                || matches!(node, AnyNode::Kern(k) if st.mem[k].subtype != EXPLICIT);
            st.linebreak.global_prev_p = cur;
            st.linebreak.cur_p = cur.link(&st.mem);
        }

        if st.linebreak.cur_p.is_null() {
            line_break_try_break(st, EJECT_PENALTY, HYPHENATED, None)?;
            if let Some(best) = st.mem.choose_best_break(&mut st.linebreak, &st.eqtb) {
                break best;
            }
        }

        st.mem.free_break_lists(&st.linebreak);

        if !second_pass {
            st.linebreak.threshold = st.eqtb.int_par(IntPar::Tolerance);
            second_pass = true;
            st.linebreak.final_pass = st.eqtb.dimen_par(DimenPar::EmergencyStretch) <= Scaled::ZERO;
        } else {
            st.linebreak.background[2] += st.eqtb.dimen_par(DimenPar::EmergencyStretch);
            st.linebreak.final_pass = true;
        }
    };

    st.mem
        .finish_last_line_fit(&mut st.linebreak, best, last_line_fill)?;

    let last_line = post_line_break(st, best, d)?;

    st.mem.free_break_lists(&st.linebreak);

    st.scanner.pack_begin_line = 0;
    Ok(last_line)
}

#[inline]
fn stretch_fitness(b: i32) -> usize {
    if b > 99 {
        VERY_LOOSE_FIT
    } else if b > 12 {
        LOOSE_FIT
    } else {
        DECENT_FIT
    }
}

#[inline]
fn shrink_fitness(b: i32) -> usize {
    if b > 12 {
        TIGHT_FIT
    } else {
        DECENT_FIT
    }
}

/// Badness and fitness class of a line that is `shortfall` short with `stretch` available.
#[inline]
fn stretch_badness(shortfall: Scaled, stretch: Scaled) -> (i32, usize) {
    if shortfall > Scaled::from_raw(7_230_584) && stretch < Scaled::from_raw(1_663_497) {
        (INF_BAD, VERY_LOOSE_FIT)
    } else {
        let b = crate::helpers::badness(shortfall, stretch);
        (b, stretch_fitness(b))
    }
}

impl MemState {
    /// Set `break_width` to the width of the line material after a break at
    /// `cur_p`, which is `disc` for a hyphenated break.
    fn compute_break_width(
        &self,
        fonts: &mut FontState,
        linebreak: &mut LinebreakState,
        disc: Option<NodePtr<DiscNode>>,
    ) -> Result<(), Abort> {
        linebreak.break_width[1..].copy_from_slice(&linebreak.background[1..]);
        let mut s = linebreak.cur_p;

        if let Some(cur_disc) = disc {
            let mut v = cur_disc.ptr();
            s = self[cur_disc].post_break;

            for _ in 0..self[cur_disc].subtype {
                v = v.link(self);
                linebreak.break_width[1] -= self.disc_node_width(fonts, v).map_err(|whatsit| {
                    Abort::Confusion(if whatsit { "disc1a" } else { "disc1" })
                })?;
            }

            while !s.is_null() {
                linebreak.break_width[1] += self.disc_node_width(fonts, s).map_err(|whatsit| {
                    Abort::Confusion(if whatsit { "disc2a" } else { "disc2" })
                })?;
                s = s.link(self);
            }

            linebreak.break_width[1] += linebreak.disc_width;
            if self[cur_disc].post_break.is_null() {
                s = v.link(self);
            }
        }

        while !s.is_null() && !self.is_char_node(s) {
            match self.classify(s) {
                Ok(AnyNode::Glue(g)) => {
                    let spec = self[g].glue_ptr;
                    linebreak.break_width[1] -= self[spec].width;
                    linebreak.break_width[2 + self[spec].stretch_order.index()] -=
                        self[spec].stretch;
                    linebreak.break_width[6] -= self[spec].shrink;
                }
                Ok(AnyNode::Penalty(_)) => {}
                Ok(AnyNode::Math(m)) => linebreak.break_width[1] -= self[m].width,
                Ok(AnyNode::Kern(k)) if self[k].subtype == EXPLICIT => {
                    linebreak.break_width[1] -= self[k].width;
                }
                _ => break,
            }
            s = s.link(self);
        }
        Ok(())
    }

    /// Record the feasible breaks found so far as new active nodes before `r`, with
    /// delta nodes around them (TeX's "insert new active nodes").
    #[allow(clippy::too_many_arguments)]
    fn insert_new_active_nodes(
        &mut self,
        scanner: &mut ScannerState,
        linebreak: &mut LinebreakState,
        eqtb: &EqtbState,
        r: MemPtr,
        prev_r: &mut MemPtr,
        prev_prev_r: &mut MemPtr,
        break_type: i16,
    ) -> Result<(), Abort> {
        let bw = linebreak.break_width;
        let caw = linebreak.cur_active_width;
        if let ActiveOrDelta::Delta(prev_delta) = self.classify_active_or_delta(*prev_r) {
            let old = delta_widths(prev_delta, self);
            let w = std::array::from_fn(|k| old[k] - caw[k] + bw[k]);
            set_delta_widths(prev_delta, self, &w);
        } else if *prev_r == ACTIVE_LIST {
            scanner.active_width[1..].copy_from_slice(&bw[1..]);
        } else {
            let w = std::array::from_fn(|k| bw[k] - caw[k]);
            let q = self.new_delta_node(r, &w)?;
            prev_r.set_link(self, q.ptr());
            *prev_prev_r = *prev_r;
            *prev_r = q.ptr();
        }

        let adj = eqtb.int_par(IntPar::AdjDemerits).wrapping_abs();
        if adj >= MAX_HALFWORD.wrapping_sub(linebreak.minimum_demerits) {
            linebreak.minimum_demerits = AWFUL_BAD - 1;
        } else {
            linebreak.minimum_demerits += adj;
        }

        for fit_class in VERY_LOOSE_FIT..=TIGHT_FIT {
            if linebreak.minimal_demerits[fit_class] <= linebreak.minimum_demerits {
                let passive = NodePtr::<PassiveNode>::alloc(self)?;
                self[passive].link = link_of(linebreak.passive);
                linebreak.passive = Some(passive);
                self[passive].cur_break = linebreak.cur_p;
                self[passive].next_break = linebreak.best_place[fit_class];

                let q = self.new_active_node(r)?;
                let n = &mut self[q];
                n.break_node = passive.ptr();
                n.line_number = linebreak.best_pl_line[fit_class].wrapping_add(1);
                n.fitness = fit_class as u16;
                n.break_type = break_type as u16;
                n.total_demerits = linebreak.minimal_demerits[fit_class];
                if linebreak.do_last_line_fit {
                    n.shortfall = linebreak.best_pl_short[fit_class];
                    n.glue = linebreak.best_pl_glue[fit_class];
                }

                prev_r.set_link(self, q.ptr());
                *prev_r = q.ptr();
            }
            linebreak.minimal_demerits[fit_class] = MAX_HALFWORD;
        }
        linebreak.minimum_demerits = MAX_HALFWORD;

        if r != LAST_ACTIVE {
            let w = std::array::from_fn(|k| caw[k] - bw[k]);
            let q = self.new_delta_node(r, &w)?;
            prev_r.set_link(self, q.ptr());
            *prev_prev_r = *prev_r;
            *prev_r = q.ptr();
        }
        Ok(())
    }

    /// Consider a break at `cur_p` with penalty `pi`, updating the active list.
    #[allow(clippy::too_many_arguments)]
    fn try_break(
        &mut self,
        scanner: &mut ScannerState,
        linebreak: &mut LinebreakState,
        eqtb: &EqtbState,
        fonts: &mut FontState,
        config: &ConfigState,
        errors: &mut ErrorState,
        mut pi: i32,
        break_type: i16,
        disc: Option<NodePtr<DiscNode>>,
    ) -> Result<(), Abort> {
        if config.semantic_pagination_enabled && !linebreak.cur_p.is_null() {
            return Ok(());
        }

        if pi.abs() >= INF_PENALTY {
            if pi > 0 {
                return Ok(());
            }
            pi = EJECT_PENALTY;
        }

        let mut no_break_yet = true;
        let mut prev_r = ACTIVE_LIST;
        let mut prev_prev_r = MemPtr::NULL;
        let mut old_l = 0;
        let mut line_width = Scaled::ZERO;
        let mut g = Scaled::ZERO;

        linebreak.cur_active_width[1..].copy_from_slice(&scanner.active_width[1..]);

        loop {
            let r = prev_r.link(self);

            if let ActiveOrDelta::Delta(r_delta) = self.classify_active_or_delta(r) {
                let w = delta_widths(r_delta, self);
                for (width, delta) in linebreak.cur_active_width.iter_mut().zip(w) {
                    *width += delta;
                }
                prev_prev_r = prev_r;
                prev_r = r;
                continue;
            }

            let r_active = active(r);
            let l = self[r_active].line_number;
            if l > old_l {
                if linebreak.minimum_demerits < AWFUL_BAD
                    && (old_l != linebreak.easy_line || r == LAST_ACTIVE)
                {
                    if no_break_yet {
                        no_break_yet = false;
                        self.compute_break_width(fonts, linebreak, disc)?;
                    }
                    self.insert_new_active_nodes(
                        scanner,
                        linebreak,
                        eqtb,
                        r,
                        &mut prev_r,
                        &mut prev_prev_r,
                        break_type,
                    )?;
                }

                if r == LAST_ACTIVE {
                    return Ok(());
                }

                if l > linebreak.easy_line {
                    line_width = linebreak.second_width;
                    old_l = MAX_HALFWORD - 1;
                } else {
                    old_l = l;
                    line_width = if l > linebreak.last_special_line {
                        linebreak.second_width
                    } else if let Some(shape) = eqtb.par_shape() {
                        // A huge \prevgraf wraps line numbers negative, past the shape.
                        if l > 0 {
                            shape.line(self, l).width
                        } else {
                            linebreak.second_width
                        }
                    } else {
                        linebreak.first_width
                    };
                }
            }

            let caw = &linebreak.cur_active_width;
            let mut artificial_demerits;
            let mut shortfall;
            if config.semantic_pagination_enabled {
                line_width = caw[1];
                artificial_demerits = true;
                shortfall = Scaled::ZERO;
            } else {
                artificial_demerits = false;
                shortfall = line_width - caw[1];
                if eqtb.int_par(IntPar::XetexProtrudeChars) > 1 {
                    shortfall += linebreak
                        .total_pw(errors, self, fonts, r_active, disc)
                        .ok_or(Abort::PdfError(&["total_pw", "hlist stack failure"]))?;
                }
            }

            let caw = linebreak.cur_active_width;
            let mut b = 0;
            let mut fit_class = DECENT_FIT;
            let mut last_line_fit_found = false;

            if shortfall > Scaled::ZERO {
                if caw[3] != Scaled::ZERO || caw[4] != Scaled::ZERO || caw[5] != Scaled::ZERO {
                    if linebreak.do_last_line_fit
                        && linebreak.cur_p.is_null()
                        && self[r_active].shortfall != Scaled::ZERO
                        && self[r_active].glue > Scaled::ZERO
                        && caw[3..6] == linebreak.fill_width
                    {
                        g = if self[r_active].shortfall > Scaled::ZERO {
                            caw[2]
                        } else {
                            caw[6]
                        };

                        if g > Scaled::ZERO {
                            errors.arith_error = false;
                            g = errors.fract(
                                g,
                                self[r_active].shortfall.raw(),
                                self[r_active].glue.raw(),
                                MAX_DIMEN,
                            );
                            let last_line_fit = eqtb.int_par(IntPar::LastLineFit);
                            if last_line_fit < 1000 {
                                g = errors.fract(g, last_line_fit, 1000, MAX_DIMEN);
                            }
                            if errors.arith_error {
                                g = if self[r_active].shortfall > Scaled::ZERO {
                                    MAX_DIMEN
                                } else {
                                    -MAX_DIMEN
                                };
                            }

                            if g > Scaled::ZERO {
                                g = g.min(shortfall);
                                (b, fit_class) = stretch_badness(g, caw[2]);
                                last_line_fit_found = true;
                            } else if g < Scaled::ZERO {
                                g = g.max(-caw[6]);
                                b = crate::helpers::badness(-g, caw[6]);
                                fit_class = shrink_fitness(b);
                                last_line_fit_found = true;
                            }
                        }
                    }

                    if !last_line_fit_found {
                        shortfall = Scaled::ZERO;
                        b = 0;
                        fit_class = DECENT_FIT;
                    }
                } else {
                    (b, fit_class) = stretch_badness(shortfall, caw[2]);
                }
            } else {
                b = if -shortfall > caw[6] {
                    INF_BAD + 1
                } else {
                    crate::helpers::badness(-shortfall, caw[6])
                };
                fit_class = shrink_fitness(b);
            }

            if linebreak.do_last_line_fit && !last_line_fit_found {
                if linebreak.cur_p.is_null() {
                    shortfall = Scaled::ZERO;
                }
                g = match shortfall.signum() {
                    1 => caw[2],
                    -1 => caw[6],
                    _ => Scaled::ZERO,
                };
            }

            let node_r_stays_active = if b > INF_BAD || pi == EJECT_PENALTY {
                if linebreak.final_pass
                    && linebreak.minimum_demerits == AWFUL_BAD
                    && self[r_active].link == LAST_ACTIVE
                    && prev_r == ACTIVE_LIST
                {
                    artificial_demerits = true;
                } else if b > linebreak.threshold {
                    self.deactivate_node(scanner, linebreak, r, &mut prev_r, &mut prev_prev_r);
                    continue;
                }
                false
            } else {
                prev_r = r;
                if b > linebreak.threshold {
                    continue;
                }
                true
            };

            let mut d = if artificial_demerits {
                0
            } else {
                let mut d = eqtb.int_par(IntPar::LinePenalty).wrapping_add(b);
                if d.wrapping_abs() >= 10_000 {
                    d = 100_000_000;
                } else {
                    d *= d;
                }

                if pi > 0 {
                    d += pi * pi;
                } else if pi < 0 && pi > EJECT_PENALTY {
                    d -= pi * pi;
                }

                if break_type == HYPHENATED && self[r_active].break_type as i16 == HYPHENATED {
                    d = d.wrapping_add(if linebreak.cur_p.is_null() {
                        eqtb.int_par(IntPar::FinalHyphenDemerits)
                    } else {
                        eqtb.int_par(IntPar::DoubleHyphenDemerits)
                    });
                }

                if (fit_class as i32 - i32::from(self[r_active].fitness)).abs() > 1 {
                    d = d.wrapping_add(eqtb.int_par(IntPar::AdjDemerits));
                }

                d
            };

            d = d.wrapping_add(self[r_active].total_demerits);
            if d <= linebreak.minimal_demerits[fit_class] {
                linebreak.minimal_demerits[fit_class] = d;
                linebreak.best_place[fit_class] = self[r_active].break_node;
                linebreak.best_pl_line[fit_class] = l;
                if linebreak.do_last_line_fit {
                    linebreak.best_pl_short[fit_class] = shortfall;
                    linebreak.best_pl_glue[fit_class] = g;
                }
                linebreak.minimum_demerits = linebreak.minimum_demerits.min(d);
            }

            if !node_r_stays_active {
                self.deactivate_node(scanner, linebreak, r, &mut prev_r, &mut prev_prev_r);
            }
        }
    }

    /// Unlink and free the active node `r`, merging the delta nodes around it.
    fn deactivate_node(
        &mut self,
        scanner: &mut ScannerState,
        linebreak: &mut LinebreakState,
        r: MemPtr,
        prev_r: &mut MemPtr,
        prev_prev_r: &mut MemPtr,
    ) {
        let next = r.link(self);
        prev_r.set_link(self, next);
        active(r).free(self);

        if *prev_r == ACTIVE_LIST {
            let r = ACTIVE_LIST.link(self);
            if let ActiveOrDelta::Delta(r_delta) = self.classify_active_or_delta(r) {
                let w = delta_widths(r_delta, self);
                for (width, delta) in scanner.active_width.iter_mut().zip(w) {
                    *width += delta;
                }
                linebreak.cur_active_width[1..].copy_from_slice(&scanner.active_width[1..]);
                ACTIVE_LIST.set_link(self, self[r_delta].link);
                delta_node(r).free(self);
            }
        } else if let ActiveOrDelta::Delta(prev_delta) = self.classify_active_or_delta(*prev_r) {
            let r = prev_r.link(self);
            if r == LAST_ACTIVE {
                let w = delta_widths(prev_delta, self);
                for (width, delta) in linebreak.cur_active_width.iter_mut().zip(w) {
                    *width -= delta;
                }
                prev_prev_r.set_link(self, LAST_ACTIVE);
                delta_node(*prev_r).free(self);
                *prev_r = *prev_prev_r;
            } else if let ActiveOrDelta::Delta(r_delta) = self.classify_active_or_delta(r) {
                let rw = delta_widths(r_delta, self);
                let pw = delta_widths(prev_delta, self);
                for (width, delta) in linebreak.cur_active_width.iter_mut().zip(rw) {
                    *width += delta;
                }
                set_delta_widths(prev_delta, self, &std::array::from_fn(|k| pw[k] + rw[k]));
                self[prev_delta].link = self[r_delta].link;
                delta_node(r).free(self);
            }
        }
    }
}
