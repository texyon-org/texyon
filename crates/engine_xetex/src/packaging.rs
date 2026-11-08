//! Box packing (`hpack`, `vpackage`), vertical breaking (`vert_break`,
//! `vsplit`, `prune_page_top`, `do_marks`) and node-list copying and freeing.

use crate::consts::{IGNORE_DEPTH, NULL_FONT, SMALL_NODE_SIZE};
use crate::errors::Abort;
use crate::mem::GARBAGE;
use crate::mem_types::ListStateRecord;
use crate::node::{
    AdjustPlacement, AnyNode, BoxLr, BoxNode, CharNode, KernSubtype, MathCode, NativeWordNode,
    NodePtr, RuleDimen, WhatsitVariant,
};
use crate::params::{DimenPar, GluePar, IntPar};
use crate::state::{EngineState, EqtbState, FontState, MemState, ScannerState, ShipoutState};
use crate::units::{GlueOrder, GlueSign, MemPtr, NodeType, OrderTotals, Orientation, Scaled};

mod copy;
mod vert_break;

use self::vert_break::push_slot;
pub(crate) use self::vert_break::{vert_break, vsplit, MarksAction};

/// The word of a box whose link field is its `list_ptr`.
const LIST_OFFSET: i32 = 5;

/// The size `hpack` and `vpack` aim for: exactly some size, or the natural size plus some.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

    /// WEB's `exactly`/`additional` code and the amount, as the save stack keeps them.
    pub(crate) fn to_words(self) -> (i32, i32) {
        match self {
            PackSpec::Exactly(size) => (0, size.raw()),
            PackSpec::Additional(extra) => (1, extra.raw()),
        }
    }

    pub(crate) fn from_words(code: i32, amount: i32) -> Option<PackSpec> {
        let amount = Scaled::from_raw(amount);
        match code {
            0 => Some(PackSpec::Exactly(amount)),
            1 => Some(PackSpec::Additional(amount)),
            _ => None,
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
        let prev_depth = self.prev_depth();
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
            self.tail_append(mem, p.ptr());
        }
        self.tail_append(mem, b.ptr());
        self.set_prev_depth(if upwards { mem[b].height } else { mem[b].depth });
        Ok(())
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
        self[r].glue_order = o.into();
        self[r].glue_sign = sign.into();
        if totals[o] != Scaled::ZERO {
            self[r].glue_set = excess / f64::from(totals[o].raw());
        } else {
            self[r].glue_sign = GlueSign::Normal.into();
            self[r].glue_set = 0.0;
        }
        o
    }

    fn clear_box_glue(&mut self, r: NodePtr<BoxNode>) {
        let node = &mut self[r];
        node.glue_sign = GlueSign::Normal.into();
        node.glue_order = GlueOrder::Normal.into();
        node.glue_set = 0.0;
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
        r_node: NodePtr<BoxNode>,
        mut q: MemPtr,
        w: NodePtr<NativeWordNode>,
    ) -> Result<(MemPtr, NodePtr<NativeWordNode>), Abort> {
        let p = w.ptr();
        let font = self[w].font;
        let same_font =
            |mem: &MemState, p: MemPtr| mem.as_native_word(p).filter(|&w| mem[w].font == font);

        let mut k = if q != r_node.ptr() + LIST_OFFSET {
            self.disc_replace_count(q).unwrap_or(0)
        } else {
            0
        };
        while q.link(self) != p {
            k -= 1;
            q = q.link(self);
            if let Some(count) = self.disc_replace_count(q) {
                k = count;
            }
        }

        let mut pp = p.link(self);
        while k <= 0 && !pp.is_null() && !self.is_char_node(pp) {
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
        let mut s = p;
        while s != pp {
            if let Some(sw) = self.as_native_word(s) {
                total_chars += i32::from(self[sw].length);
            }
            s = s.link(self);
        }

        let merged = self.new_native_word_node(eqtb, font, total_chars)?;
        self[merged].subtype = self[w].subtype;
        q.set_link(self, merged.ptr());
        let last = self.predecessor(p, pp);
        self[merged].link = last.link(self);
        last.set_link(self, MemPtr::NULL);

        let mut at = 0;
        let mut s = p;
        loop {
            if let Some(sw) = self.as_native_word(s) {
                let (to, from) = self.entries_mut_and(merged, sw)?;
                to[at..at + from.len()].copy_from_slice(from);
                at += from.len();
            }
            if s.link(self).is_null() {
                break;
            }
            s = s.link(self);
        }
        self.flush_node_list(p)?;
        self.set_native_metrics(eqtb, fonts, merged)?;
        Ok((q, merged))
    }

    #[allow(clippy::too_many_arguments)]
    fn hpack(
        &mut self,
        eqtb: &EqtbState,
        fonts: &mut FontState,
        scanner: &mut ScannerState,
        shipout: &mut ShipoutState,
        list: MemPtr,
        spec: PackSpec,
    ) -> Result<PackOutcome, Abort> {
        shipout.last_badness = 0;
        let r_node = NodePtr::<BoxNode>::alloc(self)?;
        self[r_node].set_lr(BoxLr::Normal);
        self[r_node].shift_amount = Scaled::ZERO;
        self[r_node].list_ptr = list;
        // `q` trails `p`, starting at the word whose link is the list pointer.
        let mut q = r_node.ptr() + LIST_OFFSET;
        let mut h = Scaled::ZERO;
        let mut d = Scaled::ZERO;
        let mut x = Scaled::ZERO;
        scanner.total_stretch = OrderTotals::ZERO;
        scanner.total_shrink = OrderTotals::ZERO;
        let mut diags = [None, None];
        let texxet = eqtb.int_par(IntPar::Texxet) > 0;

        if texxet {
            self.push_lr(&mut scanner.lr_ptr, MathCode::Before)?;
        }

        let mut p = list;
        'outer: while !p.is_null() {
            while let Some(ch) = self.as_char(p) {
                let f = self[ch].font;
                let c = fonts.effective_char(f, self[ch].character);
                let info = fonts.char_info(f, c);
                x += fonts.char_width(f, info);
                h = h.max(fonts.char_height(f, info));
                d = d.max(fonts.char_depth(f, info));
                p = p.link(self);
            }

            if p.is_null() {
                break;
            }

            match self.classify(p) {
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
                    if !scanner.adjust_tail.is_null() || !scanner.pre_adjust_tail.is_null() {
                        q = self.predecessor(q, p);
                        let adjust_list = self[adjust].adjust_ptr;
                        let tail = if self[adjust].placement == AdjustPlacement::Pre {
                            &mut scanner.pre_adjust_tail
                        } else {
                            &mut scanner.adjust_tail
                        };
                        if tail.is_null() {
                            return Err(Abort::Confusion("pre vadjust"));
                        }
                        tail.set_link(self, adjust_list);
                        *tail = self.list_tail(*tail);
                        p = self[adjust].link;
                        self.free_node(adjust.ptr(), SMALL_NODE_SIZE);
                        q.set_link(self, p);
                        p = q;
                    }
                }
                Ok(AnyNode::Ins(_) | AnyNode::Mark(_)) => {
                    if !scanner.adjust_tail.is_null() || !scanner.pre_adjust_tail.is_null() {
                        q = self.predecessor(q, p);
                        scanner.adjust_tail.set_link(self, p);
                        scanner.adjust_tail = p;
                        p = p.link(self);
                        q.set_link(self, p);
                        p = q;
                    }
                }
                Ok(AnyNode::Whatsit(w)) => match w.variant(self) {
                    Some(WhatsitVariant::NativeWord(word)) => {
                        let (new_q, native) =
                            self.merge_native_words(eqtb, fonts, r_node, q, word)?;
                        q = new_q;
                        p = native.ptr();
                        h = h.max(self[native].height);
                        d = d.max(self[native].depth);
                        x += self[native].width;
                    }
                    _ => {
                        if let Some(sized) = w.sized(self) {
                            let sized = self[sized];
                            h = h.max(sized.height);
                            d = d.max(sized.depth);
                            x += sized.width;
                        }
                    }
                },
                Ok(AnyNode::Glue(glue)) => {
                    let spec = self[glue].glue_ptr;
                    x += self[spec].width;
                    scanner.total_stretch[self[spec].stretch_order] += self[spec].stretch;
                    scanner.total_shrink[self[spec].shrink_order] += self[spec].shrink;
                    if self[glue].leaders().is_some() {
                        let leader = self[glue].leader_ptr;
                        let (leader_h, leader_d) = match self.classify(leader) {
                            Ok(AnyNode::Box(b)) => (self[b].height, self[b].depth),
                            Ok(AnyNode::Rule(rule)) => {
                                (self[rule].height.to_scaled(), self[rule].depth.to_scaled())
                            }
                            _ => (
                                Scaled::from_raw(self.b32s1(leader + 3)),
                                Scaled::from_raw(self.b32s1(leader + 2)),
                            ),
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
                            if self.lr_code(scanner.lr_ptr) == Some(code.lr_end()) {
                                self.pop_lr(&mut scanner.lr_ptr);
                            } else {
                                scanner.lr_problems += 1;
                                let kern = math.into_kern(self);
                                self[kern].subtype = KernSubtype::Explicit.into();
                            }
                        } else {
                            self.push_lr(&mut scanner.lr_ptr, code.lr_end())?;
                        }
                    }
                }
                Ok(AnyNode::Ligature(lig)) => {
                    // Measure the ligature as its character through the scratch char node.
                    let scratch = NodePtr::<CharNode>::at(GARBAGE);
                    self[scratch].font = self[lig].font;
                    self[scratch].character = self[lig].character;
                    GARBAGE.set_link(self, self[lig].link);
                    p = GARBAGE;
                    fonts.xtx_ligature_present = true;
                    continue 'outer;
                }
                _ => {}
            }
            p = p.link(self);
        }

        if !scanner.adjust_tail.is_null() {
            scanner.adjust_tail.set_link(self, MemPtr::NULL);
        }
        if !scanner.pre_adjust_tail.is_null() {
            scanner.pre_adjust_tail.set_link(self, MemPtr::NULL);
        }
        self[r_node].height = h;
        self[r_node].depth = d;

        let w = spec.target(x);
        self[r_node].width = w;
        x = w - x;

        let has_list = !self[r_node].list_ptr.is_null();
        if x == Scaled::ZERO {
            self.clear_box_glue(r_node);
        } else if x > Scaled::ZERO {
            let o = self.set_box_glue(
                r_node,
                GlueSign::Stretching,
                f64::from(x.raw()),
                &scanner.total_stretch,
            );
            if o == GlueOrder::Normal && has_list {
                shipout.last_badness =
                    crate::helpers::badness(x, scanner.total_stretch[GlueOrder::Normal]);
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
                &scanner.total_shrink,
            );
            if scanner.total_shrink[o] < (-x) && o == GlueOrder::Normal && has_list {
                shipout.last_badness = 1_000_000;
                self[r_node].glue_set = 1.0;
                let excess = (-x) - scanner.total_shrink[GlueOrder::Normal];
                let hfuzz = eqtb.dimen_par(DimenPar::Hfuzz);
                if excess > hfuzz || eqtb.int_par(IntPar::Hbadness) < 100 {
                    let overfull_rule = eqtb.dimen_par(DimenPar::OverfullRule);
                    if overfull_rule > Scaled::ZERO && excess > hfuzz {
                        q = self.list_tail(q);
                        let rule = self.new_rule()?;
                        q.set_link(self, rule.ptr());
                        self[rule].width = RuleDimen::Set(overfull_rule).into();
                    }
                    push_slot(&mut diags, PackDiag::HOverfull { excess });
                }
            } else if o == GlueOrder::Normal && has_list {
                shipout.last_badness =
                    crate::helpers::badness(-x, scanner.total_shrink[GlueOrder::Normal]);
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
            if self.lr_code(scanner.lr_ptr) != Some(MathCode::Before) {
                q = self.list_tail(q);
                loop {
                    let code = self.lr_code(scanner.lr_ptr).ok_or(Abort::Confusion("LR"))?;
                    let math = self.new_math(Scaled::ZERO, code)?;
                    q.set_link(self, math.ptr());
                    q = math.ptr();
                    scanner.lr_problems += 10000;
                    self.pop_lr(&mut scanner.lr_ptr);
                    if self.lr_code(scanner.lr_ptr) == Some(MathCode::Before) {
                        break;
                    }
                }
            }
            if scanner.lr_problems > 0 {
                push_slot(
                    &mut diags,
                    PackDiag::LrProblem {
                        missing: scanner.lr_problems / 10000,
                        extra: scanner.lr_problems % 10000,
                    },
                );
                scanner.lr_problems = 0;
            }
            self.pop_lr(&mut scanner.lr_ptr);
            if !scanner.lr_ptr.is_null() {
                return Err(Abort::Confusion("LR1"));
            }
        }

        Ok(PackOutcome {
            r_box: r_node,
            diags,
        })
    }
}

pub(crate) fn print_hpack_diag(st: &mut EngineState, box_ptr: NodePtr<BoxNode>, diag: PackDiag) {
    use crate::output::{
        diagnostic_begin_capture_warning_here, print, print_int, print_ln, print_nl, print_scaled,
    };
    print_ln(st);
    diagnostic_begin_capture_warning_here(st);
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
    print_hpack_common_ending(st, box_ptr);
}

fn print_hpack_common_ending(st: &mut EngineState, box_ptr: NodePtr<BoxNode>) {
    use crate::output::{
        begin_diagnostic, capture_to_diagnostic, end_diagnostic, print, print_int, print_ln,
        short_display, show_box,
    };
    if st.shipout.output_active {
        print(st, ") has occurred while \\output is active");
    } else {
        let pack_begin_line = st.scanner.pack_begin_line;
        if pack_begin_line != 0 {
            if pack_begin_line > 0 {
                print(st, ") in paragraph at lines ");
            } else {
                print(st, ") in alignment at lines ");
            }
            print_int(st, pack_begin_line.abs());
            print(st, "--");
        } else {
            print(st, ") detected at line ");
        }
        print_int(st, st.mem.line);
    }
    capture_to_diagnostic(st, None);
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
    let outcome = st.mem.hpack(
        &st.eqtb,
        &mut st.fonts,
        &mut st.scanner,
        &mut st.shipout,
        list,
        spec,
    )?;
    for diag in outcome.diags.into_iter().flatten() {
        print_hpack_diag(st, outcome.r_box, diag);
    }
    Ok(outcome.r_box)
}

impl MemState {
    #[allow(clippy::too_many_arguments)]
    fn vpackage(
        &mut self,
        eqtb: &EqtbState,
        scanner: &mut ScannerState,
        shipout: &mut ShipoutState,
        list: MemPtr,
        spec: PackSpec,
        l: Scaled,
    ) -> Result<PackOutcome, Abort> {
        shipout.last_badness = 0;
        let r_node = NodePtr::<BoxNode>::alloc(self)?;
        self[r_node].orientation = Orientation::Vertical.into();
        let upwards = eqtb.int_par(IntPar::XetexUpwards) > 0;
        self[r_node].set_upwards(upwards);
        self[r_node].shift_amount = Scaled::ZERO;
        self[r_node].list_ptr = list;

        let mut w = Scaled::ZERO;
        let mut d = Scaled::ZERO;
        let mut x = Scaled::ZERO;
        scanner.total_stretch = OrderTotals::ZERO;
        scanner.total_shrink = OrderTotals::ZERO;
        let mut diags = [None, None];

        let mut p = list;
        while !p.is_null() {
            if self.is_char_node(p) {
                return Err(Abort::Confusion("vpack"));
            }
            match self.classify(p) {
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
                    scanner.total_stretch[self[spec].stretch_order] += self[spec].stretch;
                    scanner.total_shrink[self[spec].shrink_order] += self[spec].shrink;
                    if self[glue].leaders().is_some() {
                        let leader = self[glue].leader_ptr;
                        let leader_w = match self.classify(leader) {
                            Ok(AnyNode::Box(b)) => self[b].width,
                            Ok(AnyNode::Rule(rule)) => self[rule].width.to_scaled(),
                            _ => Scaled::from_raw(self.b32s1(leader + 1)),
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
            p = p.link(self);
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

        let has_list = !self[r_node].list_ptr.is_null();
        if x == Scaled::ZERO {
            self.clear_box_glue(r_node);
        } else if x > Scaled::ZERO {
            let o = self.set_box_glue(
                r_node,
                GlueSign::Stretching,
                f64::from(x.raw()),
                &scanner.total_stretch,
            );
            if o == GlueOrder::Normal && has_list {
                shipout.last_badness =
                    crate::helpers::badness(x, scanner.total_stretch[GlueOrder::Normal]);
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
                &scanner.total_shrink,
            );
            if scanner.total_shrink[o] < (-x) && o == GlueOrder::Normal && has_list {
                shipout.last_badness = 1_000_000;
                self[r_node].glue_set = 1.0;
                let excess = (-x) - scanner.total_shrink[GlueOrder::Normal];
                if excess > eqtb.dimen_par(DimenPar::Vfuzz) || eqtb.int_par(IntPar::Vbadness) < 100
                {
                    push_slot(&mut diags, PackDiag::VOverfull { excess });
                }
            } else if o == GlueOrder::Normal && has_list {
                shipout.last_badness =
                    crate::helpers::badness(-x, scanner.total_shrink[GlueOrder::Normal]);
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
        })
    }
}

pub(crate) fn print_vpack_diag(st: &mut EngineState, box_ptr: NodePtr<BoxNode>, diag: PackDiag) {
    use crate::output::{
        diagnostic_begin_capture_warning_here, print, print_int, print_ln, print_nl, print_scaled,
    };
    print_ln(st);
    diagnostic_begin_capture_warning_here(st);
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
    print_vpack_common_ending(st, box_ptr);
}

fn print_vpack_common_ending(st: &mut EngineState, box_ptr: NodePtr<BoxNode>) {
    use crate::output::{
        begin_diagnostic, capture_to_diagnostic, end_diagnostic, print, print_int, print_ln,
        show_box,
    };
    if st.shipout.output_active {
        print(st, ") has occurred while \\output is active");
    } else {
        let pack_begin_line = st.scanner.pack_begin_line;
        if pack_begin_line != 0 {
            print(st, ") in alignment at lines ");
            print_int(st, pack_begin_line.abs());
            print(st, "--");
        } else {
            print(st, ") detected at line ");
        }
        print_int(st, st.mem.line);
        print_ln(st);
    }
    capture_to_diagnostic(st, None);
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
    let outcome = st
        .mem
        .vpackage(&st.eqtb, &mut st.scanner, &mut st.shipout, list, spec, l)?;
    for diag in outcome.diags.into_iter().flatten() {
        print_vpack_diag(st, outcome.r_box, diag);
    }
    Ok(outcome.r_box)
}

#[cfg(test)]
mod pack_spec_tests {
    use super::PackSpec;
    use crate::units::Scaled;

    #[test]
    fn spec_round_trips_through_save_stack_words() {
        let size = Scaled::from_raw(12_345);
        for spec in [PackSpec::Exactly(size), PackSpec::Additional(size)] {
            let (code, amount) = spec.to_words();
            assert_eq!(PackSpec::from_words(code, amount), Some(spec));
        }
        assert_eq!(PackSpec::from_words(2, 0), None);
    }

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
