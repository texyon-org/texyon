//! Box packing (`hpack`, `vpackage`), vertical breaking (`vert_break`,
//! `vsplit`, `prune_page_top`, `do_marks`) and node-list copying and freeing.

use crate::base::consts::{IGNORE_DEPTH, NULL_FONT};
use crate::base::params::{DimenPar, GluePar, IntPar};
use crate::base::units::{
    Dir, GlueOrder, GlueSign, MemPtr, NodeType, OrderTotals, Orientation, Scaled,
};
use crate::fonts::FontState;
use crate::memory::mem::MemState;
use crate::memory::mem::{ADJUST_HEAD, PRE_ADJUST_HEAD};
use crate::memory::mem_types::ListStateRecord;
use crate::memory::node::LinkSlot;
use crate::memory::node::{
    AdjustPlacement, AnyNode, BoxLr, BoxNode, KernSubtype, MathCode, NativeWordNode, NodePtr,
    RuleDimen, WhatsitVariant,
};
use crate::print::errors::Abort;
use crate::print::errors::OrConfusion;
use crate::shipout::ShipoutState;
use crate::state::EngineState;
use crate::tables::eqtb::EqtbState;

mod copy;
mod vert_break;
pub(crate) use vert_break::{CurMarks, MarkClasses};

use self::vert_break::push_slot;
pub(crate) use self::vert_break::{MarksAction, vert_break, vsplit};

/// The size `hpack` and `vpack` aim for: exactly some size, or the natural size plus some.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PackSpec {
    Exactly(Scaled),
    Additional(Scaled),
}

impl PackSpec {
    /// The natural size.
    pub(crate) const NATURAL: PackSpec = PackSpec::Additional(Scaled::ZERO);

    /// The size to set a box of natural size `natural` to.
    #[inline]
    pub(crate) fn target(self, natural: Scaled) -> Scaled {
        match self {
            PackSpec::Exactly(size) => size,
            PackSpec::Additional(extra) => natural + extra,
        }
    }
}

impl ListStateRecord {
    /// Append box `b` to the current vertical list, preceded by interline glue.
    pub fn append_to_vlist(
        &mut self,
        mem: &mut MemState,
        eqtb: &EqtbState,
        b: NodePtr<BoxNode>,
    ) -> Result<(), Abort> {
        let upwards = eqtb.int_par(IntPar::XetexUpwards) > 0;
        let prev_depth = self.prev_depth()?;
        if prev_depth > IGNORE_DEPTH {
            let baseline_skip = eqtb.glue_par(GluePar::BaselineSkip);
            let d = mem[baseline_skip].width
                - prev_depth
                - if upwards { mem[b].depth } else { mem[b].height };
            let p = if d < eqtb.dimen_par(DimenPar::LineSkipLimit) {
                mem.new_param_glue(eqtb, GluePar::LineSkip)?
            } else {
                let p = mem.new_skip_param(eqtb, GluePar::BaselineSkip)?;
                let spec = mem[p].glue_ptr;
                mem[spec].width = d;
                p
            };
            self.tail_append(mem, p);
        }
        self.tail_append(mem, b);
        self.set_prev_depth(if upwards { mem[b].height } else { mem[b].depth })
    }
}

pub(crate) fn append_to_vlist(st: &mut EngineState, b: NodePtr<BoxNode>) -> Result<(), Abort> {
    st.input.cur_list.append_to_vlist(&mut st.mem, &st.eqtb, b)
}

#[derive(Copy, Clone)]
pub(crate) enum PackDiag {
    HUnderfull { badness: i32, loose: bool },
    HOverfull { excess: Scaled },
    HTight { badness: i32 },
    LrProblem { missing: i32, extra: i32 },
    VUnderfull { badness: i32, loose: bool },
    VOverfull { excess: Scaled },
    VTight { badness: i32 },
}

#[derive(Copy, Clone)]
struct PackOutcome {
    r_box: NodePtr<BoxNode>,
    diags: [Option<PackDiag>; 2],
    stretch: OrderTotals,
    shrink: OrderTotals,
}

/// A packed box with the glue totals of its list.
#[derive(Copy, Clone)]
pub(crate) struct Packed {
    pub r_box: NodePtr<BoxNode>,
    pub stretch: OrderTotals,
    pub shrink: OrderTotals,
}

/// The tails of the lists `hpack` moves `\vadjust`, `\insert` and `\mark` material to.
#[derive(Copy, Clone)]
pub(crate) struct AdjustTails {
    pub post: MemPtr,
    pub pre: MemPtr,
}

impl AdjustTails {
    /// Empty lists at `ADJUST_HEAD` and `PRE_ADJUST_HEAD`.
    pub(crate) const HEADS: Self = Self {
        post: Some(ADJUST_HEAD),
        pre: Some(PRE_ADJUST_HEAD),
    };
}

/// One pass over an hlist for TeX--XeT: the LR stack, WEB's `LR_problems`
/// count of unmatched math nodes, and the current direction.
pub(crate) struct LrState {
    pub ptr: MemPtr,
    pub problems: i32,
    pub dir: Dir,
}

impl LrState {
    /// An empty stack, no problems, left to right.
    pub(crate) const START: Self = Self {
        ptr: None,
        problems: 0,
        dir: Dir::Ltr,
    };
}

/// Where the packed material began, named in over/underfull box messages.
/// The variant picks the message, so an alignment can yield `Paragraph`.
#[derive(Copy, Clone)]
pub(crate) enum PackOrigin {
    Here,
    Paragraph(i32),
    Alignment(i32),
}

impl PackOrigin {
    /// From WEB's `pack_begin_line`: positive for a paragraph, negative for an alignment.
    pub(crate) fn from_begin_line(line: i32) -> Self {
        match line.signum() {
            1 => Self::Paragraph(line),
            -1 => Self::Alignment(-line),
            _ => Self::Here,
        }
    }
}

impl PackOutcome {
    fn packed(&self) -> Packed {
        Packed {
            r_box: self.r_box,
            stretch: self.stretch,
            shrink: self.shrink,
        }
    }
}

impl MemState {
    /// Set `r`'s glue to distribute `excess` over the highest-order `totals`,
    /// returning that order.
    fn set_box_glue(
        &mut self,
        r: NodePtr<BoxNode>,
        sign: GlueSign,
        excess: f64,
        totals: &OrderTotals,
    ) -> GlueOrder {
        let o = totals.max_order();
        let node = &mut self[r];
        node.glue_order = o.into();
        if totals[o] != Scaled::ZERO {
            node.glue_sign = sign.into();
            node.glue_set =
                crate::memory::node::GlueRatio::new(excess / f64::from(totals[o].raw()));
        } else {
            node.glue_sign = GlueSign::Normal.into();
            node.glue_set = crate::memory::node::GlueRatio::new(0.0);
        }
        o
    }

    fn clear_box_glue(&mut self, r: NodePtr<BoxNode>) {
        let node = &mut self[r];
        node.glue_sign = GlueSign::Normal.into();
        node.glue_order = GlueOrder::Normal.into();
        node.glue_set = crate::memory::node::GlueRatio::new(0.0);
    }

    /// `replace_count(q)` if `type(q)=disc_node`, read like TeX does even when
    /// `q` is a char node (whose font sits in the type field).
    fn disc_replace_count(&self, q: MemPtr) -> Option<i32> {
        match self.as_char(q) {
            Some(ch) => (self[ch].font.raw() == NodeType::Disc as i32)
                .then(|| i32::from(self[ch].character)),
            None => self
                .as_disc(q)
                .map(|disc| i32::from(self[disc].replace_count)),
        }
    }

    /// Merge the run of native words in `w`'s font that follows `w` (possibly
    /// across empty discretionaries) into one node. `q` trails `w`. Returns the
    /// new `q` and the node now in `w`'s place.
    fn merge_native_words(
        &mut self,
        eqtb: &EqtbState,
        fonts: &FontState,
        mut q: LinkSlot,
        w: NodePtr<NativeWordNode>,
    ) -> Result<(LinkSlot, NodePtr<NativeWordNode>), Abort> {
        let p = w.idx();
        let font = self[w].font;
        let same_font =
            |mem: &MemState, p: MemPtr| mem.as_native_word(p).filter(|&w| mem[w].font == font);

        let mut k = self.disc_replace_count(q.node()).unwrap_or(0);
        while q.get(self) != Some(p) {
            k -= 1;
            let next = q.get(self).or_confusion("native word")?;
            q = LinkSlot::Node(next);
            if let Some(count) = self.disc_replace_count(Some(next)) {
                k = count;
            }
        }

        let mut pp = p.link(self);
        while k <= 0 && pp.is_some_and(|x| !self.is_char_node(x)) {
            if let Some(next) = same_font(self, pp) {
                pp = self[next].link;
            } else if let Some(next) = self
                .as_disc(pp)
                .and_then(|disc| same_font(self, self[disc].link))
            {
                pp = self[next].link;
            } else {
                break;
            }
        }
        if pp == p.link(self) {
            return Ok((q, w));
        }

        let mut total_chars: i32 = 0;
        let mut s = Some(p);
        while s != pp {
            let here = s.or_confusion("native word")?;
            if let Some(sw) = self.as_native_word(s) {
                total_chars += i32::from(self[sw].length);
            }
            s = here.link(self);
        }

        let merged = self.new_native_word_node(eqtb, font, total_chars)?;
        self[merged].subtype = self[w].subtype;
        q.set(self, merged.ptr());
        let last = self.predecessor(p, pp)?;
        self[merged].link = last.link(self);
        last.set_link(self, None);

        let mut at = 0;
        let mut s = p;
        loop {
            if let Some(sw) = self.as_native_word(Some(s)) {
                let (to, from) = self.entries_mut_and(merged, sw)?;
                to[at..at + from.len()].copy_from_slice(from);
                at += from.len();
            }
            let Some(next) = s.link(self) else {
                break;
            };
            s = next;
        }
        self.flush_node_list(Some(p))?;
        self.set_native_metrics(eqtb, fonts, merged)?;
        Ok((q, merged))
    }

    fn hpack(
        &mut self,
        eqtb: &EqtbState,
        fonts: &mut FontState,
        shipout: &mut ShipoutState,
        list: MemPtr,
        spec: PackSpec,
        adjust: Option<&mut AdjustTails>,
    ) -> Result<PackOutcome, Abort> {
        let mut no_adjust = AdjustTails {
            post: None,
            pre: None,
        };
        let tails = adjust.unwrap_or(&mut no_adjust);
        let mut lr_ptr = None;
        let mut lr_problems = 0;
        shipout.last_badness = 0;
        let r_node = NodePtr::<BoxNode>::alloc(self)?;
        let node = &mut self[r_node];
        node.set_lr(BoxLr::Normal);
        node.shift_amount = Scaled::ZERO;
        node.list_ptr = list;
        // `q` trails `p`, starting at the box's list pointer.
        let mut q = LinkSlot::List(r_node);
        let mut h = Scaled::ZERO;
        let mut d = Scaled::ZERO;
        let mut x = Scaled::ZERO;
        let mut total_stretch = OrderTotals::ZERO;
        let mut total_shrink = OrderTotals::ZERO;
        let mut diags = [None, None];
        let texxet = eqtb.int_par(IntPar::Texxet) > 0;

        if texxet {
            self.push_lr(&mut lr_ptr, MathCode::Before)?;
        }

        let mut p = list;
        while p.is_some() {
            while let Some(ch) = self.as_char(p) {
                let (cw, ch_h, ch_d) = fonts.char_box(self[ch].font, self[ch].character);
                x += cw;
                h = h.max(ch_h);
                d = d.max(ch_d);
                p = ch.idx().link(self);
            }

            let Some(mut here) = p else {
                break;
            };

            match self.classify(here) {
                Ok(AnyNode::Box(b)) => {
                    let s = self[b].shift_amount;
                    x += self[b].width;
                    h = h.max(self[b].height - s);
                    d = d.max(self[b].depth + s);
                }
                Ok(AnyNode::Rule(rule)) => {
                    x += self[rule].width.to_scaled();
                    h = h.max(self[rule].height.to_scaled());
                    d = d.max(self[rule].depth.to_scaled());
                }
                Ok(AnyNode::Unset(u)) => {
                    x += self[u].width;
                    h = h.max(self[u].height);
                    d = d.max(self[u].depth);
                }
                Ok(AnyNode::Adjust(adjust)) => {
                    if !tails.post.is_none() || !tails.pre.is_none() {
                        q = q.before(self, p)?;
                        let adjust_list = self[adjust].adjust_ptr;
                        let tail = if self[adjust].placement == AdjustPlacement::Pre {
                            &mut tails.pre
                        } else {
                            &mut tails.post
                        };
                        let t = tail.or_confusion("pre vadjust")?;
                        t.set_link(self, adjust_list);
                        *tail = Some(self.list_tail(t));
                        let next = self[adjust].link;
                        adjust.free(self);
                        q.set(self, next);
                        p = next;
                        continue;
                    }
                }
                Ok(AnyNode::Ins(_) | AnyNode::Mark(_)) => {
                    if !tails.post.is_none() || !tails.pre.is_none() {
                        q = q.before(self, p)?;
                        tails.post.or_confusion("adjust")?.set_link(self, p);
                        tails.post = p;
                        let next = here.link(self);
                        q.set(self, next);
                        p = next;
                        continue;
                    }
                }
                Ok(AnyNode::Whatsit(w)) => match w.variant(self) {
                    Some(WhatsitVariant::NativeWord(word)) => {
                        let (new_q, native) = self.merge_native_words(eqtb, fonts, q, word)?;
                        q = new_q;
                        here = native.idx();
                        h = h.max(self[native].height);
                        d = d.max(self[native].depth);
                        x += self[native].width;
                    }
                    _ => {
                        if let Some(sized) = w.dims(self) {
                            h = h.max(sized.height);
                            d = d.max(sized.depth);
                            x += sized.width;
                        }
                    }
                },
                Ok(AnyNode::Glue(glue)) => {
                    let spec = self[glue].glue_ptr;
                    x += self[spec].width;
                    total_stretch[self[spec].stretch_order] += self[spec].stretch;
                    total_shrink[self[spec].shrink_order] += self[spec].shrink;
                    if self[glue].leaders().is_some() {
                        let leader = self[glue].leader_ptr;
                        let (leader_h, leader_d) = match leader.map(|l| self.classify(l)) {
                            Some(Ok(AnyNode::Box(b))) => (self[b].height, self[b].depth),
                            Some(Ok(AnyNode::Rule(rule))) => {
                                (self[rule].height.to_scaled(), self[rule].depth.to_scaled())
                            }
                            _ => return Err(Abort::Confusion("leader box")),
                        };
                        h = h.max(leader_h);
                        d = d.max(leader_d);
                    }
                }
                Ok(AnyNode::Kern(kern)) => {
                    x += self[kern].width;
                }
                Ok(AnyNode::MarginKern(kern)) => {
                    x += self[kern].width;
                }
                Ok(AnyNode::Math(math)) => {
                    x += self[math].width;
                    if texxet {
                        let code = self[math].code()?;
                        if code.is_end() {
                            if self.lr_code(lr_ptr) == Some(code.lr_end()) {
                                self.pop_lr(&mut lr_ptr);
                            } else {
                                lr_problems += 1;
                                let kern = math.into_kern(self);
                                self[kern].subtype = KernSubtype::Explicit.into();
                            }
                        } else {
                            self.push_lr(&mut lr_ptr, code.lr_end())?;
                        }
                    }
                }
                Ok(AnyNode::Ligature(lig)) => {
                    fonts.xtx_ligature_present = true;
                    let (cw, ch_h, ch_d) = fonts.char_box(self[lig].font, self[lig].character);
                    x += cw;
                    h = h.max(ch_h);
                    d = d.max(ch_d);
                }
                _ => {}
            }
            p = here.link(self);
        }

        if let Some(post) = tails.post {
            post.set_link(self, None);
        }
        if let Some(pre) = tails.pre {
            pre.set_link(self, None);
        }
        let w = spec.target(x);
        let node = &mut self[r_node];
        node.height = h;
        node.depth = d;
        node.width = w;
        x = w - x;

        let has_list = !self[r_node].list_ptr.is_none();
        if x == Scaled::ZERO {
            self.clear_box_glue(r_node);
        } else if x > Scaled::ZERO {
            let o = self.set_box_glue(
                r_node,
                GlueSign::Stretching,
                f64::from(x.raw()),
                &total_stretch,
            );
            if o == GlueOrder::Normal && has_list {
                shipout.last_badness =
                    crate::base::helpers::badness(x, total_stretch[GlueOrder::Normal]);
                if shipout.last_badness > eqtb.int_par(IntPar::Hbadness) {
                    push_slot(
                        &mut diags,
                        PackDiag::HUnderfull {
                            badness: shipout.last_badness,
                            loose: shipout.last_badness <= 100,
                        },
                    );
                }
            }
        } else {
            let o = self.set_box_glue(
                r_node,
                GlueSign::Shrinking,
                f64::from((-x).raw()),
                &total_shrink,
            );
            if total_shrink[o] < (-x) && o == GlueOrder::Normal && has_list {
                shipout.last_badness = 1_000_000;
                self[r_node].glue_set = crate::memory::node::GlueRatio::new(1.0);
                let excess = (-x) - total_shrink[GlueOrder::Normal];
                let hfuzz = eqtb.dimen_par(DimenPar::Hfuzz);
                if excess > hfuzz || eqtb.int_par(IntPar::Hbadness) < 100 {
                    let overfull_rule = eqtb.dimen_par(DimenPar::OverfullRule);
                    if overfull_rule > Scaled::ZERO && excess > hfuzz {
                        q = q.tail(self);
                        let rule = self.new_rule()?;
                        q.set(self, rule.ptr());
                        self[rule].width = RuleDimen::Set(overfull_rule).into();
                    }
                    push_slot(&mut diags, PackDiag::HOverfull { excess });
                }
            } else if o == GlueOrder::Normal && has_list {
                shipout.last_badness =
                    crate::base::helpers::badness(-x, total_shrink[GlueOrder::Normal]);
                if shipout.last_badness > eqtb.int_par(IntPar::Hbadness) {
                    push_slot(
                        &mut diags,
                        PackDiag::HTight {
                            badness: shipout.last_badness,
                        },
                    );
                }
            }
        }

        if texxet {
            if self.lr_code(lr_ptr) != Some(MathCode::Before) {
                q = q.tail(self);
                loop {
                    let code = self.lr_code(lr_ptr).or_confusion("LR")?;
                    let math = self.new_math(Scaled::ZERO, code)?;
                    q.set(self, math.ptr());
                    q = LinkSlot::Node(math.idx());
                    lr_problems += 10000;
                    self.pop_lr(&mut lr_ptr);
                    if self.lr_code(lr_ptr) == Some(MathCode::Before) {
                        break;
                    }
                }
            }
            if lr_problems > 0 {
                push_slot(
                    &mut diags,
                    PackDiag::LrProblem {
                        missing: lr_problems / 10000,
                        extra: lr_problems % 10000,
                    },
                );
            }
            self.pop_lr(&mut lr_ptr);
            if !lr_ptr.is_none() {
                return Err(Abort::Confusion("LR1"));
            }
        }

        Ok(PackOutcome {
            r_box: r_node,
            diags,
            stretch: total_stretch,
            shrink: total_shrink,
        })
    }
}

fn print_hpack_diag(
    st: &mut EngineState,
    box_ptr: NodePtr<BoxNode>,
    diag: PackDiag,
    origin: PackOrigin,
) {
    use crate::print::{print, print_int, print_ln, print_nl, print_scaled};
    print_ln(st);
    crate::print::diagnostic::begin_warning(st);
    match diag {
        PackDiag::HUnderfull { badness, loose } => {
            if loose {
                print_nl(st, "Loose");
            } else {
                print_nl(st, "Underfull");
            }
            print(st, " \\hbox (badness ");
            print_int(st, badness);
        }
        PackDiag::HOverfull { excess } => {
            print_nl(st, "Overfull \\hbox (");
            print_scaled(st, excess);
            print(st, "pt too wide");
        }
        PackDiag::HTight { badness } => {
            print_nl(st, "Tight \\hbox (badness ");
            print_int(st, badness);
        }
        PackDiag::LrProblem { missing, extra } => {
            print_nl(st, "\\endL or \\endR problem (");
            print_int(st, missing);
            print(st, " missing, ");
            print_int(st, extra);
            print(st, " extra");
        }
        PackDiag::VUnderfull { .. } | PackDiag::VOverfull { .. } | PackDiag::VTight { .. } => {
            unreachable!("vpack diagnostic passed to hpack printer")
        }
    }
    print_hpack_common_ending(st, box_ptr, origin);
}

fn print_hpack_common_ending(st: &mut EngineState, box_ptr: NodePtr<BoxNode>, origin: PackOrigin) {
    use crate::print::{
        begin_diagnostic, end_diagnostic, print, print_int, print_ln, short_display, show_box,
    };
    if st.shipout.output_active {
        print(st, ") has occurred while \\output is active");
    } else {
        match origin {
            PackOrigin::Here => print(st, ") detected at line "),
            PackOrigin::Paragraph(line) => print(st, (") in paragraph at lines ", line, "--")),
            PackOrigin::Alignment(line) => print(st, (") in alignment at lines ", line, "--")),
        }
        print_int(st, st.mem.line);
    }
    crate::print::diagnostic::end_capture(st);
    print_ln(st);
    st.errors.font_in_short_display = NULL_FONT;
    let list = st.mem[box_ptr].list_ptr;
    short_display(st, list);
    print_ln(st);
    begin_diagnostic(st);
    show_box(st, box_ptr.ptr());
    end_diagnostic(st, true);
}

/// `hpack`, printing its over/underfull diagnostics in order.
pub(crate) fn hpack(
    st: &mut EngineState,
    list: MemPtr,
    spec: PackSpec,
) -> Result<NodePtr<BoxNode>, Abort> {
    Ok(hpack_with(st, list, spec, PackOrigin::Here, None)?.r_box)
}

/// `hpack` for material from `origin`, moving adjustments to `adjust` if given.
pub(crate) fn hpack_with(
    st: &mut EngineState,
    list: MemPtr,
    spec: PackSpec,
    origin: PackOrigin,
    adjust: Option<&mut AdjustTails>,
) -> Result<Packed, Abort> {
    let outcome = st
        .mem
        .hpack(&st.eqtb, &mut st.fonts, &mut st.shipout, list, spec, adjust)?;
    for diag in outcome.diags.into_iter().flatten() {
        print_hpack_diag(st, outcome.r_box, diag, origin);
    }
    Ok(outcome.packed())
}

impl MemState {
    fn vpackage(
        &mut self,
        eqtb: &EqtbState,
        shipout: &mut ShipoutState,
        list: MemPtr,
        spec: PackSpec,
        l: Scaled,
    ) -> Result<PackOutcome, Abort> {
        shipout.last_badness = 0;
        let r_node = NodePtr::<BoxNode>::alloc(self)?;
        let upwards = eqtb.int_par(IntPar::XetexUpwards) > 0;
        let node = &mut self[r_node];
        node.orientation = Orientation::Vertical.into();
        node.set_upwards(upwards);
        node.shift_amount = Scaled::ZERO;
        node.list_ptr = list;

        let mut w = Scaled::ZERO;
        let mut d = Scaled::ZERO;
        let mut x = Scaled::ZERO;
        let mut total_stretch = OrderTotals::ZERO;
        let mut total_shrink = OrderTotals::ZERO;
        let mut diags = [None, None];

        let mut p = list;
        while let Some(here) = p {
            if self.is_char_node(here) {
                return Err(Abort::Confusion("vpack"));
            }
            match self.classify(here) {
                Ok(AnyNode::Box(b)) => {
                    x += d + self[b].height;
                    d = self[b].depth;
                    w = w.max(self[b].width + self[b].shift_amount);
                }
                Ok(AnyNode::Rule(rule)) => {
                    x += d + self[rule].height.to_scaled();
                    d = self[rule].depth.to_scaled();
                    w = w.max(self[rule].width.to_scaled());
                }
                Ok(AnyNode::Unset(u)) => {
                    x += d + self[u].height;
                    d = self[u].depth;
                    w = w.max(self[u].width);
                }
                Ok(AnyNode::Whatsit(wn)) => {
                    if let Some(WhatsitVariant::Pic(pic)) = wn.variant(self) {
                        let pic = self[pic];
                        x += d + pic.height;
                        d = pic.depth;
                        w = w.max(pic.width);
                    }
                }
                Ok(AnyNode::Glue(glue)) => {
                    x += d;
                    d = Scaled::ZERO;
                    let spec = self[glue].glue_ptr;
                    x += self[spec].width;
                    total_stretch[self[spec].stretch_order] += self[spec].stretch;
                    total_shrink[self[spec].shrink_order] += self[spec].shrink;
                    if self[glue].leaders().is_some() {
                        let leader = self[glue].leader_ptr;
                        let leader_w = match leader.map(|l| self.classify(l)) {
                            Some(Ok(AnyNode::Box(b))) => self[b].width,
                            Some(Ok(AnyNode::Rule(rule))) => self[rule].width.to_scaled(),
                            _ => return Err(Abort::Confusion("leader box")),
                        };
                        w = w.max(leader_w);
                    }
                }
                Ok(AnyNode::Kern(kern)) => {
                    x += d + self[kern].width;
                    d = Scaled::ZERO;
                }
                _ => {}
            }
            p = here.link(self);
        }

        self[r_node].width = w;
        if d > l {
            x += d - l;
            self[r_node].depth = l;
        } else {
            self[r_node].depth = d;
        }

        let h = spec.target(x);
        self[r_node].height = h;
        x = h - x;

        let has_list = !self[r_node].list_ptr.is_none();
        if x == Scaled::ZERO {
            self.clear_box_glue(r_node);
        } else if x > Scaled::ZERO {
            let o = self.set_box_glue(
                r_node,
                GlueSign::Stretching,
                f64::from(x.raw()),
                &total_stretch,
            );
            if o == GlueOrder::Normal && has_list {
                shipout.last_badness =
                    crate::base::helpers::badness(x, total_stretch[GlueOrder::Normal]);
                if shipout.last_badness > eqtb.int_par(IntPar::Vbadness) {
                    push_slot(
                        &mut diags,
                        PackDiag::VUnderfull {
                            badness: shipout.last_badness,
                            loose: shipout.last_badness <= 100,
                        },
                    );
                }
            }
        } else {
            let o = self.set_box_glue(
                r_node,
                GlueSign::Shrinking,
                (-i64::from(x.raw())) as f64,
                &total_shrink,
            );
            if total_shrink[o] < (-x) && o == GlueOrder::Normal && has_list {
                shipout.last_badness = 1_000_000;
                self[r_node].glue_set = crate::memory::node::GlueRatio::new(1.0);
                let excess = (-x) - total_shrink[GlueOrder::Normal];
                if excess > eqtb.dimen_par(DimenPar::Vfuzz) || eqtb.int_par(IntPar::Vbadness) < 100
                {
                    push_slot(&mut diags, PackDiag::VOverfull { excess });
                }
            } else if o == GlueOrder::Normal && has_list {
                shipout.last_badness =
                    crate::base::helpers::badness(-x, total_shrink[GlueOrder::Normal]);
                if shipout.last_badness > eqtb.int_par(IntPar::Vbadness) {
                    push_slot(
                        &mut diags,
                        PackDiag::VTight {
                            badness: shipout.last_badness,
                        },
                    );
                }
            }
        }

        Ok(PackOutcome {
            r_box: r_node,
            diags,
            stretch: total_stretch,
            shrink: total_shrink,
        })
    }
}

pub(crate) fn print_vpack_diag(
    st: &mut EngineState,
    box_ptr: NodePtr<BoxNode>,
    diag: PackDiag,
    origin: PackOrigin,
) {
    use crate::print::{print, print_int, print_ln, print_nl, print_scaled};
    print_ln(st);
    crate::print::diagnostic::begin_warning(st);
    match diag {
        PackDiag::VUnderfull { badness, loose } => {
            if loose {
                print_nl(st, "Loose");
            } else {
                print_nl(st, "Underfull");
            }
            print(st, " \\vbox (badness ");
            print_int(st, badness);
        }
        PackDiag::VOverfull { excess } => {
            print_nl(st, "Overfull \\vbox (");
            print_scaled(st, excess);
            print(st, "pt too high");
        }
        PackDiag::VTight { badness } => {
            print_nl(st, "Tight \\vbox (badness ");
            print_int(st, badness);
        }
        PackDiag::HUnderfull { .. }
        | PackDiag::HOverfull { .. }
        | PackDiag::HTight { .. }
        | PackDiag::LrProblem { .. } => {
            unreachable!("hpack diagnostic passed to vpack printer")
        }
    }
    print_vpack_common_ending(st, box_ptr, origin);
}

fn print_vpack_common_ending(st: &mut EngineState, box_ptr: NodePtr<BoxNode>, origin: PackOrigin) {
    use crate::print::{begin_diagnostic, end_diagnostic, print, print_int, print_ln, show_box};
    if st.shipout.output_active {
        print(st, ") has occurred while \\output is active");
    } else {
        match origin {
            PackOrigin::Here => print(st, ") detected at line "),
            // WEB's vpack names an alignment for any nonzero `pack_begin_line`.
            PackOrigin::Paragraph(line) | PackOrigin::Alignment(line) => {
                print(st, (") in alignment at lines ", line, "--"))
            }
        }
        print_int(st, st.mem.line);
        print_ln(st);
    }
    crate::print::diagnostic::end_capture(st);
    begin_diagnostic(st);
    show_box(st, box_ptr.ptr());
    end_diagnostic(st, true);
}

/// `vpackage`, printing its over/underfull diagnostics in order.
pub(crate) fn vpackage(
    st: &mut EngineState,
    list: MemPtr,
    spec: PackSpec,
    l: Scaled,
) -> Result<NodePtr<BoxNode>, Abort> {
    Ok(vpackage_with(st, list, spec, l, PackOrigin::Here)?.r_box)
}

/// `vpackage` for material from `origin`.
pub(crate) fn vpackage_with(
    st: &mut EngineState,
    list: MemPtr,
    spec: PackSpec,
    l: Scaled,
    origin: PackOrigin,
) -> Result<Packed, Abort> {
    let outcome = st.mem.vpackage(&st.eqtb, &mut st.shipout, list, spec, l)?;
    for diag in outcome.diags.into_iter().flatten() {
        print_vpack_diag(st, outcome.r_box, diag, origin);
    }
    Ok(outcome.packed())
}

#[cfg(test)]
mod pack_spec_tests {
    use super::PackSpec;
    use crate::base::units::Scaled;

    #[test]
    fn target_adds_only_for_additional() {
        let natural = Scaled::from_raw(100);
        let w = Scaled::from_raw(7);
        assert_eq!(PackSpec::Exactly(w).target(natural), w);
        assert_eq!(
            PackSpec::Additional(w).target(natural),
            Scaled::from_raw(107)
        );
    }
}
