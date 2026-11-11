//! The paragraph builder: `line_break`, `post_line_break`, `hyphenate` and
//! character protrusion.

use crate::consts::{
    AWFUL_BAD, EJECT_PENALTY, INF_BAD, INF_PENALTY, MAX_DIMEN, MAX_HALFWORD, NULL_FONT,
};
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::glue_store::GlueSpecId;
use crate::mem::TEMP_HEAD;
use crate::node::{
    AnyNode, BoxNode, CharNode, DiscNode, GlueNode, KernSubtype, NodePtr, WhatsitVariant,
};
use crate::packaging::PackOrigin;
use crate::params::{DimenPar, GluePar, IntPar};
use crate::scaled_math::fract;
use crate::state::{EngineState, EqtbState, ErrorState, FontState, MemState};
use crate::units::{FontId, GlueOrder, MemPtr, Scaled};

mod hyphenate;
mod lists;
mod locale;
mod post;
mod prehyph;
mod protrusion;

pub(crate) use self::lists::Widths;
use self::lists::{
    widths, ActiveEntry, BreakLists, BreakType, Entry, EntryId, Fitness, Passive, PassiveId,
};
pub(crate) use self::locale::do_locale_linebreaks;
use self::post::post_line_break;
use self::prehyph::{set_language, try_hyphenate};

/// The line breaker's state for one paragraph.
#[derive(Default)]
struct Breaker {
    /// The active list and passive breaks.
    lists: BreakLists,
    /// Scratch for the protrusion searches through nested hlists.
    hlist_stack: Vec<MemPtr>,
    first_p: MemPtr,
    global_prev_p: MemPtr,
    cur_p: MemPtr,
    last_special_line: i32,
    first_width: Scaled,
    second_width: Scaled,
    first_indent: Scaled,
    second_indent: Scaled,
    active_width: Widths,
    cur_active_width: Widths,
    background: Widths,
    best_place: [Option<PassiveId>; 4],
    best_pl_line: [i32; 4],
    final_pass: bool,
    threshold: i32,
    minimal_demerits: [i32; 4],
    minimum_demerits: i32,
    easy_line: i32,
    do_last_line_fit: bool,
    fill_width: [Scaled; 3],
    best_pl_short: [Scaled; 4],
    best_pl_glue: [Scaled; 4],
    no_shrink_error_yet: bool,
}

/// A hyphenated break at a discretionary, with the width of its pre-break text.
#[derive(Copy, Clone)]
struct DiscBreak {
    node: NodePtr<DiscNode>,
    width: Scaled,
}

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

impl MemState {
    #[inline]
    fn has_infinite_shrink(&self, spec: GlueSpecId) -> bool {
        self[spec].shrink_order != GlueOrder::Normal && self[spec].shrink != Scaled::ZERO
    }
}

impl MemState {
    /// A copy of `spec` with finite shrink, releasing `spec`.
    pub(crate) fn finite_shrink(&mut self, spec: GlueSpecId) -> Result<GlueSpecId, Abort> {
        let q = self.new_spec(spec)?;
        self[q].shrink_order = GlueOrder::Normal.into();
        self.delete_glue_ref(spec);
        Ok(q)
    }
}

impl Breaker {
    fn start_linebreak_pass(
        &mut self,
        mem: &MemState,
        errors: &mut ErrorState,
        prev_graf: i32,
    ) -> Result<(), Abort> {
        self.lists.clear();
        let q = self.lists.push(
            EntryId::LAST_ACTIVE,
            Entry::Active(ActiveEntry::start(prev_graf)),
        )?;
        self.lists.set_link(EntryId::ACTIVE_LIST, q);

        self.active_width[1..].copy_from_slice(&self.background[1..]);
        errors.font_in_short_display = NULL_FONT;
        self.cur_p = TEMP_HEAD.link(mem);
        self.global_prev_p = self.cur_p;
        self.first_p = self.cur_p;
        Ok(())
    }

    /// Pick the best active entry. `None` when the result
    /// misses `\looseness` and another pass is due.
    fn choose_best_break(&mut self, eqtb: &EqtbState) -> Result<Option<ActiveEntry>, Abort> {
        if self.lists.is_empty() {
            return Ok(None);
        }

        let mut fewest_demerits = MAX_HALFWORD;
        let mut best = None;
        for a in self.lists.actives() {
            if a.total_demerits < fewest_demerits {
                fewest_demerits = a.total_demerits;
                best = Some(a);
            }
        }
        let mut best = best.or_confusion("line breaking")?;
        let best_line = best.line_number;

        let looseness = eqtb.int_par(IntPar::Looseness);
        if looseness == 0 {
            return Ok(Some(best));
        }

        let mut actual_looseness = 0;
        for a in self.lists.actives() {
            let line_diff = a.line_number.wrapping_sub(best_line);
            if (line_diff < actual_looseness && looseness <= line_diff)
                || (line_diff > actual_looseness && looseness >= line_diff)
            {
                best = a;
                actual_looseness = line_diff;
                fewest_demerits = a.total_demerits;
            } else if line_diff == actual_looseness && a.total_demerits < fewest_demerits {
                best = a;
                fewest_demerits = a.total_demerits;
            }
        }

        Ok((actual_looseness == looseness || self.final_pass).then_some(best))
    }

    /// Stretch the `\parfillskip` of the last line by the `\lastlinefit` adjustment.
    fn finish_last_line_fit(
        &mut self,
        mem: &mut MemState,
        best: &ActiveEntry,
        last_line_fill: NodePtr<GlueNode>,
    ) -> Result<(), Abort> {
        if !self.do_last_line_fit {
            return Ok(());
        }

        if best.shortfall == Scaled::ZERO {
            self.do_last_line_fit = false;
            return Ok(());
        }

        let old_spec = mem[last_line_fill].glue_ptr;
        let spec = mem.new_spec(old_spec)?;
        mem.delete_glue_ref(old_spec);

        let node = &mut mem[spec];
        node.width += best.shortfall - best.glue;
        node.stretch = Scaled::ZERO;
        mem[last_line_fill].glue_ptr = spec;
        Ok(())
    }
}

impl Breaker {
    /// Make `spec`'s shrink finite, reporting it on the paragraph's first offence.
    fn finite_shrink(
        &mut self,
        st: &mut EngineState,
        spec: GlueSpecId,
    ) -> Result<GlueSpecId, Abort> {
        if self.no_shrink_error_yet {
            self.no_shrink_error_yet = false;
            crate::errors::error(
                st,
                "Infinite glue shrinkage found in a paragraph",
                &[
                    "The paragraph just ended includes some glue that has",
                    r"infinite shrinkability, e.g., `\hskip 0pt minus 1fil'.",
                    "Such glue doesn't belong there---it allows a paragraph",
                    "of any length to fit on one line. But it's safe to proceed,",
                    "since the offensive shrinkability has been made finite.",
                ],
            )?;
        }
        st.mem.finite_shrink(spec)
    }

    /// TeX's `kern_break`: a kern or math node followed by glue is a legal breakpoint.
    #[inline]
    fn kern_break(
        &mut self,
        st: &mut EngineState,
        next: MemPtr,
        auto_breaking: bool,
    ) -> Result<(), Abort> {
        if !st.mem.is_char_node(next)
            && auto_breaking
            && matches!(st.mem.classify(next), Ok(AnyNode::Glue(_)))
        {
            self.try_break(st, 0, BreakType::Unhyphenated, None)?;
        }
        Ok(())
    }
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
            Ok(AnyNode::Whatsit(w)) => w.dims(self).map(|d| d.width).ok_or(true),
            _ => Err(false),
        }
    }
}

/// Break the current paragraph into lines, returning the last line's box.
pub(crate) fn line_break(st: &mut EngineState, d: bool) -> Result<NodePtr<BoxNode>, Abort> {
    let origin = PackOrigin::from_begin_line(st.input.cur_list.mode_line);
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
    let init_l_hyf = st.input.cur_list.prev_graf / 0x0040_0000;
    let init_r_hyf = (st.input.cur_list.prev_graf / 65_536) % 64;
    let init_cur_lang = (st.input.cur_list.prev_graf % 65_536) as u8;

    st.mem.pop_nest(&mut st.input)?;
    let mut lb = Breaker {
        no_shrink_error_yet: true,
        ..Breaker::default()
    };

    for n in [GluePar::LeftSkip, GluePar::RightSkip] {
        let spec = st.eqtb.glue_par(n);
        if st.mem.has_infinite_shrink(spec) {
            let spec = lb.finite_shrink(st, spec)?;
            st.eqtb.set_glue_par(n, spec);
        }
    }

    let left_skip = st.eqtb.glue_par(GluePar::LeftSkip);
    let right_skip = st.eqtb.glue_par(GluePar::RightSkip);
    let mem = &st.mem;
    let background = &mut lb.background;
    background[1] = mem[left_skip].width + mem[right_skip].width;
    background[2..=5].fill(Scaled::ZERO);
    background[2 + mem[left_skip].stretch_order.index()] = mem[left_skip].stretch;
    background[2 + mem[right_skip].stretch_order.index()] += mem[right_skip].stretch;
    background[6] = mem[left_skip].shrink + mem[right_skip].shrink;

    if st.eqtb.int_par(IntPar::LastLineFit) > 0 {
        let spec = st.mem[last_line_fill].glue_ptr;
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

    lb.minimum_demerits = AWFUL_BAD;
    lb.minimal_demerits = [AWFUL_BAD; 4];

    let par_shape = st.eqtb.par_shape();
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
            // A line number wrapped negative by a huge \prevgraf still reads these.
            lb.first_width = hsize;
            lb.first_indent = Scaled::ZERO;
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
        lb.threshold = lb.threshold.min(INF_BAD);

        if second_pass {
            if st.hyph.trie_not_ready {
                st.hyph.init_trie()?;
            }
            st.hyph.cur_lang = init_cur_lang;
            st.hyph.l_hyf = init_l_hyf;
            st.hyph.r_hyf = init_r_hyf;
            st.hyph.set_hyph_index();
        }

        lb.start_linebreak_pass(&st.mem, &mut st.errors, st.input.cur_list.prev_graf)?;
        let mut auto_breaking = true;
        // Whether glue after the previous node is a legal breakpoint.
        let mut prev_breakable = false;

        while !lb.cur_p.is_null() && !lb.lists.is_empty() {
            let mut cur = lb.cur_p;

            if st.mem.is_char_node(cur) {
                prev_breakable = true;
                lb.global_prev_p = cur;
                loop {
                    let ch = NodePtr::<CharNode>::at(cur);
                    let f = st.mem[ch].font;
                    let c = st.mem[ch].character;
                    lb.active_width[1] += st.fonts.effective_char_width(f, c);
                    cur = cur.link(&st.mem);
                    if !st.mem.is_char_node(cur) {
                        break;
                    }
                }
                lb.cur_p = cur;
            }

            let Ok(node) = st.mem.classify(cur) else {
                return Err(Abort::Confusion("paragraph"));
            };
            match node {
                AnyNode::Box(_) | AnyNode::Rule(_) => {
                    lb.active_width[1] += node.hlist_width(&st.mem);
                }
                AnyNode::Whatsit(w) => {
                    if let Some(WhatsitVariant::Language(language)) = w.variant(&st.mem) {
                        set_language(st, language);
                    } else if let Some(dims) = w.dims(&st.mem) {
                        lb.active_width[1] += dims.width;
                    }
                }
                AnyNode::Glue(cur_glue) => {
                    if auto_breaking && prev_breakable {
                        lb.try_break(st, 0, BreakType::Unhyphenated, None)?;
                    }

                    let mut spec = st.mem[cur_glue].glue_ptr;
                    if st.mem.has_infinite_shrink(spec) {
                        spec = lb.finite_shrink(st, spec)?;
                        st.mem[cur_glue].glue_ptr = spec;
                    }
                    let aw = &mut lb.active_width;
                    aw[1] += st.mem[spec].width;
                    aw[2 + st.mem[spec].stretch_order.index()] += st.mem[spec].stretch;
                    aw[6] += st.mem[spec].shrink;

                    if second_pass && auto_breaking {
                        try_hyphenate(st, cur)?;
                    }
                }
                AnyNode::Kern(k) => {
                    if st.mem[k].subtype == KernSubtype::Explicit {
                        lb.kern_break(st, st.mem[k].link, auto_breaking)?;
                    }
                    lb.active_width[1] += st.mem[k].width;
                }
                AnyNode::Ligature(lig) => {
                    let f = st.mem[lig].font;
                    st.fonts.xtx_ligature_present = true;
                    let c = st.mem[lig].character;
                    lb.active_width[1] += st.fonts.effective_char_width(f, c);
                }
                AnyNode::Disc(d) => {
                    let pre_break = st.mem[d].pre_break;
                    if pre_break.is_null() {
                        let pi = st.eqtb.int_par(IntPar::ExHyphenPenalty);
                        let disc = DiscBreak {
                            node: d,
                            width: Scaled::ZERO,
                        };
                        lb.try_break(st, pi, BreakType::Hyphenated, Some(disc))?;
                    } else {
                        let mut width = Scaled::ZERO;
                        let mut s = pre_break;
                        while !s.is_null() {
                            width += st
                                .mem
                                .disc_node_width(&mut st.fonts, s)
                                .or_confusion("disc3")?;
                            s = s.link(&st.mem);
                        }
                        lb.active_width[1] += width;
                        let pi = st.eqtb.int_par(IntPar::HyphenPenalty);
                        let disc = DiscBreak { node: d, width };
                        lb.try_break(st, pi, BreakType::Hyphenated, Some(disc))?;
                        lb.active_width[1] -= width;
                    }

                    let mut s = st.mem[d].link;
                    for _ in 0..st.mem[d].replace_count {
                        lb.active_width[1] += st
                            .mem
                            .disc_node_width(&mut st.fonts, s)
                            .or_confusion("disc4")?;
                        s = s.link(&st.mem);
                    }

                    prev_breakable = true;
                    lb.global_prev_p = cur;
                    lb.cur_p = s;
                    continue;
                }
                AnyNode::Math(m) => {
                    let code = st.mem[m].code()?;
                    if !code.is_lr() {
                        auto_breaking = code.is_end();
                    }
                    lb.kern_break(st, st.mem[m].link, auto_breaking)?;
                    lb.active_width[1] += st.mem[m].width;
                }
                AnyNode::Penalty(pen) => {
                    lb.try_break(st, st.mem[pen].penalty, BreakType::Unhyphenated, None)?;
                }
                AnyNode::Mark(_) | AnyNode::Ins(_) | AnyNode::Adjust(_) => {}
                _ => return Err(Abort::Confusion("paragraph")),
            }

            prev_breakable = node.precedes_break()
                || matches!(node, AnyNode::Kern(k) if st.mem[k].subtype != KernSubtype::Explicit);
            lb.global_prev_p = cur;
            lb.cur_p = cur.link(&st.mem);
        }

        if lb.cur_p.is_null() {
            lb.try_break(st, EJECT_PENALTY, BreakType::Hyphenated, None)?;
            if let Some(best) = lb.choose_best_break(&st.eqtb)? {
                break best;
            }
        }

        if !second_pass {
            lb.threshold = st.eqtb.int_par(IntPar::Tolerance);
            second_pass = true;
            lb.final_pass = st.eqtb.dimen_par(DimenPar::EmergencyStretch) <= Scaled::ZERO;
        } else {
            lb.background[2] += st.eqtb.dimen_par(DimenPar::EmergencyStretch);
            lb.final_pass = true;
        }
    };

    lb.finish_last_line_fit(&mut st.mem, &best, last_line_fill)?;

    let last_line = post_line_break(st, &mut lb, &best, d, origin)?;
    Ok(last_line)
}

#[inline]
fn stretch_fitness(b: i32) -> Fitness {
    if b > 99 {
        Fitness::VeryLoose
    } else if b > 12 {
        Fitness::Loose
    } else {
        Fitness::Decent
    }
}

#[inline]
fn shrink_fitness(b: i32) -> Fitness {
    if b > 12 {
        Fitness::Tight
    } else {
        Fitness::Decent
    }
}

/// Badness and fitness class of a line that is `shortfall` short with `stretch` available.
#[inline]
fn stretch_badness(shortfall: Scaled, stretch: Scaled) -> (i32, Fitness) {
    if shortfall > Scaled::from_raw(7_230_584) && stretch < Scaled::from_raw(1_663_497) {
        (INF_BAD, Fitness::VeryLoose)
    } else {
        let b = crate::helpers::badness(shortfall, stretch);
        (b, stretch_fitness(b))
    }
}

impl Breaker {
    /// The width of the line material after a break at `cur_p`, which is `disc`
    /// for a hyphenated break.
    fn compute_break_width(
        &self,
        mem: &MemState,
        fonts: &mut FontState,
        disc: Option<DiscBreak>,
    ) -> Result<Widths, Abort> {
        let mut break_width = self.background;
        let mut s = self.cur_p;

        if let Some(DiscBreak {
            node: cur_disc,
            width: disc_width,
        }) = disc
        {
            let mut v = cur_disc.ptr();
            s = mem[cur_disc].post_break;

            for _ in 0..mem[cur_disc].replace_count {
                v = v.link(mem);
                break_width[1] -= mem.disc_node_width(fonts, v).map_err(|whatsit| {
                    Abort::Confusion(if whatsit { "disc1a" } else { "disc1" })
                })?;
            }

            while !s.is_null() {
                break_width[1] += mem.disc_node_width(fonts, s).map_err(|whatsit| {
                    Abort::Confusion(if whatsit { "disc2a" } else { "disc2" })
                })?;
                s = s.link(mem);
            }

            break_width[1] += disc_width;
            if mem[cur_disc].post_break.is_null() {
                s = v.link(mem);
            }
        }

        while !s.is_null() && !mem.is_char_node(s) {
            match mem.classify(s) {
                Ok(AnyNode::Glue(g)) => {
                    let spec = mem[g].glue_ptr;
                    break_width[1] -= mem[spec].width;
                    break_width[2 + mem[spec].stretch_order.index()] -= mem[spec].stretch;
                    break_width[6] -= mem[spec].shrink;
                }
                Ok(AnyNode::Penalty(_)) => {}
                Ok(AnyNode::Math(m)) => break_width[1] -= mem[m].width,
                Ok(AnyNode::Kern(k)) if mem[k].subtype == KernSubtype::Explicit => {
                    break_width[1] -= mem[k].width;
                }
                _ => break,
            }
            s = s.link(mem);
        }
        Ok(break_width)
    }
}

impl Breaker {
    /// Record the feasible breaks found so far as new active entries before `r`, with
    /// delta entries around them (TeX's "insert new active nodes").
    fn insert_new_active_nodes(
        &mut self,
        eqtb: &EqtbState,
        bw: &Widths,
        r: EntryId,
        prev_r: &mut EntryId,
        prev_prev_r: &mut Option<EntryId>,
        break_type: BreakType,
    ) -> Result<(), Abort> {
        let caw = self.cur_active_width;
        if let Entry::Delta(old) = self.lists.get_mut(*prev_r) {
            *old = widths(|k| old[k] - caw[k] + bw[k]);
        } else if *prev_r == EntryId::ACTIVE_LIST {
            self.active_width[1..].copy_from_slice(&bw[1..]);
        } else {
            let q = self
                .lists
                .push(r, Entry::Delta(widths(|k| bw[k] - caw[k])))?;
            self.lists.set_link(*prev_r, q);
            *prev_prev_r = Some(*prev_r);
            *prev_r = q;
        }

        let adj = eqtb.int_par(IntPar::AdjDemerits).wrapping_abs();
        if adj >= MAX_HALFWORD.wrapping_sub(self.minimum_demerits) {
            self.minimum_demerits = AWFUL_BAD - 1;
        } else {
            self.minimum_demerits += adj;
        }

        for fitness in Fitness::ALL {
            let i = fitness.index();
            if self.minimal_demerits[i] <= self.minimum_demerits {
                let passive = self.lists.push_passive(Passive {
                    cur_break: self.cur_p,
                    next_break: self.best_place[i],
                })?;
                let (shortfall, glue) = if self.do_last_line_fit {
                    (self.best_pl_short[i], self.best_pl_glue[i])
                } else {
                    (Scaled::ZERO, Scaled::ZERO)
                };
                let q = self.lists.push(
                    r,
                    Entry::Active(ActiveEntry {
                        fitness,
                        break_type,
                        line_number: self.best_pl_line[i].wrapping_add(1),
                        break_node: Some(passive),
                        total_demerits: self.minimal_demerits[i],
                        shortfall,
                        glue,
                    }),
                )?;
                self.lists.set_link(*prev_r, q);
                *prev_r = q;
            }
            self.minimal_demerits[i] = MAX_HALFWORD;
        }
        self.minimum_demerits = MAX_HALFWORD;

        if r != EntryId::LAST_ACTIVE {
            let q = self
                .lists
                .push(r, Entry::Delta(widths(|k| caw[k] - bw[k])))?;
            self.lists.set_link(*prev_r, q);
            *prev_prev_r = Some(*prev_r);
            *prev_r = q;
        }
        Ok(())
    }

    /// Consider a break at `cur_p` with penalty `pi`, updating the active list.
    fn try_break(
        &mut self,
        st: &mut EngineState,
        mut pi: i32,
        break_type: BreakType,
        disc: Option<DiscBreak>,
    ) -> Result<(), Abort> {
        let EngineState {
            mem,
            eqtb,
            fonts,
            config,
            ..
        } = st;
        if config.semantic_pagination_enabled && !self.cur_p.is_null() {
            return Ok(());
        }

        if pi.abs() >= INF_PENALTY {
            if pi > 0 {
                return Ok(());
            }
            pi = EJECT_PENALTY;
        }

        let mut break_width = None;
        let mut prev_r = EntryId::ACTIVE_LIST;
        let mut prev_prev_r = None;
        let mut old_l = 0;
        let mut line_width = Scaled::ZERO;
        let mut g = Scaled::ZERO;

        self.cur_active_width[1..].copy_from_slice(&self.active_width[1..]);

        loop {
            let r = self.lists.link(prev_r);

            let r_active = match self.lists.get(r) {
                Entry::Delta(w) => {
                    for (width, delta) in self.cur_active_width.iter_mut().zip(w) {
                        *width += delta;
                    }
                    prev_prev_r = Some(prev_r);
                    prev_r = r;
                    continue;
                }
                Entry::Active(a) => a,
            };

            let l = r_active.line_number;
            if l > old_l {
                if self.minimum_demerits < AWFUL_BAD
                    && (old_l != self.easy_line || r == EntryId::LAST_ACTIVE)
                {
                    let bw = match break_width {
                        Some(bw) => bw,
                        None => *break_width.insert(self.compute_break_width(mem, fonts, disc)?),
                    };
                    self.insert_new_active_nodes(
                        eqtb,
                        &bw,
                        r,
                        &mut prev_r,
                        &mut prev_prev_r,
                        break_type,
                    )?;
                }

                if r == EntryId::LAST_ACTIVE {
                    return Ok(());
                }

                if l > self.easy_line {
                    line_width = self.second_width;
                    old_l = MAX_HALFWORD - 1;
                } else {
                    old_l = l;
                    line_width = if l > self.last_special_line {
                        self.second_width
                    } else if let Some(shape) = eqtb.par_shape() {
                        // A huge \prevgraf wraps line numbers negative, past the shape.
                        if l > 0 {
                            shape.line(mem, l).width
                        } else {
                            self.second_width
                        }
                    } else {
                        self.first_width
                    };
                }
            }

            let caw = &self.cur_active_width;
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
                    shortfall += self
                        .total_pw(mem, fonts, r_active.break_node, disc.map(|d| d.node))
                        .ok_or(Abort::PdfError(&["total_pw", "hlist stack failure"]))?;
                }
            }

            let caw = self.cur_active_width;
            let mut b = 0;
            let mut fit_class = Fitness::Decent;
            let mut last_line_fit_found = false;

            if shortfall > Scaled::ZERO {
                if caw[3] != Scaled::ZERO || caw[4] != Scaled::ZERO || caw[5] != Scaled::ZERO {
                    if self.do_last_line_fit
                        && self.cur_p.is_null()
                        && r_active.shortfall != Scaled::ZERO
                        && r_active.glue > Scaled::ZERO
                        && caw[3..6] == self.fill_width
                    {
                        g = if r_active.shortfall > Scaled::ZERO {
                            caw[2]
                        } else {
                            caw[6]
                        };

                        if g > Scaled::ZERO {
                            let mut overflowed = false;
                            g = fract(g, r_active.shortfall.raw(), r_active.glue.raw(), MAX_DIMEN)
                                .track(&mut overflowed);
                            let last_line_fit = eqtb.int_par(IntPar::LastLineFit);
                            if last_line_fit < 1000 {
                                g = fract(g, last_line_fit, 1000, MAX_DIMEN).track(&mut overflowed);
                            }
                            if overflowed {
                                g = if r_active.shortfall > Scaled::ZERO {
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
                        fit_class = Fitness::Decent;
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

            if self.do_last_line_fit && !last_line_fit_found {
                if self.cur_p.is_null() {
                    shortfall = Scaled::ZERO;
                }
                g = match shortfall.signum() {
                    1 => caw[2],
                    -1 => caw[6],
                    _ => Scaled::ZERO,
                };
            }

            let node_r_stays_active = if b > INF_BAD || pi == EJECT_PENALTY {
                if self.final_pass
                    && self.minimum_demerits == AWFUL_BAD
                    && self.lists.link(r) == EntryId::LAST_ACTIVE
                    && prev_r == EntryId::ACTIVE_LIST
                {
                    artificial_demerits = true;
                } else if b > self.threshold {
                    self.deactivate_node(r, &mut prev_r, &mut prev_prev_r)?;
                    continue;
                }
                false
            } else {
                prev_r = r;
                if b > self.threshold {
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

                if break_type == BreakType::Hyphenated
                    && r_active.break_type == BreakType::Hyphenated
                {
                    d = d.wrapping_add(if self.cur_p.is_null() {
                        eqtb.int_par(IntPar::FinalHyphenDemerits)
                    } else {
                        eqtb.int_par(IntPar::DoubleHyphenDemerits)
                    });
                }

                if fit_class.far_from(r_active.fitness) {
                    d = d.wrapping_add(eqtb.int_par(IntPar::AdjDemerits));
                }

                d
            };

            d = d.wrapping_add(r_active.total_demerits);
            let i = fit_class.index();
            if d <= self.minimal_demerits[i] {
                self.minimal_demerits[i] = d;
                self.best_place[i] = r_active.break_node;
                self.best_pl_line[i] = l;
                if self.do_last_line_fit {
                    self.best_pl_short[i] = shortfall;
                    self.best_pl_glue[i] = g;
                }
                self.minimum_demerits = self.minimum_demerits.min(d);
            }

            if !node_r_stays_active {
                self.deactivate_node(r, &mut prev_r, &mut prev_prev_r)?;
            }
        }
    }

    /// Unlink the active entry `r`, merging the delta entries around it.
    fn deactivate_node(
        &mut self,
        r: EntryId,
        prev_r: &mut EntryId,
        prev_prev_r: &mut Option<EntryId>,
    ) -> Result<(), Abort> {
        let lists = &mut self.lists;
        lists.set_link(*prev_r, lists.link(r));

        if *prev_r == EntryId::ACTIVE_LIST {
            let r = lists.link(EntryId::ACTIVE_LIST);
            if let Entry::Delta(w) = lists.get(r) {
                for (width, delta) in self.active_width.iter_mut().zip(w) {
                    *width += delta;
                }
                self.cur_active_width[1..].copy_from_slice(&self.active_width[1..]);
                lists.set_link(EntryId::ACTIVE_LIST, lists.link(r));
            }
        } else if let Entry::Delta(pw) = lists.get(*prev_r) {
            let r = lists.link(*prev_r);
            if r == EntryId::LAST_ACTIVE {
                for (width, delta) in self.cur_active_width.iter_mut().zip(pw) {
                    *width -= delta;
                }
                let before = prev_prev_r.or_confusion("line breaking")?;
                lists.set_link(before, EntryId::LAST_ACTIVE);
                *prev_r = before;
            } else if let Entry::Delta(rw) = lists.get(r) {
                for (width, delta) in self.cur_active_width.iter_mut().zip(rw) {
                    *width += delta;
                }
                *lists.get_mut(*prev_r) = Entry::Delta(widths(|k| pw[k] + rw[k]));
                lists.set_link(*prev_r, lists.link(r));
            }
        }
        Ok(())
    }
}
