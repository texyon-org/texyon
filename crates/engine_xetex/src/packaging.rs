//! Box packing (`hpack`, `vpackage`), vertical breaking (`vert_break`,
//! `vsplit`, `prune_page_top`, `do_marks`) and node-list copying and freeing.

use crate::consts::{A_LEADERS, BEFORE, EXPLICIT, IGNORE_DEPTH, NULL_FONT, SMALL_NODE_SIZE};
use crate::errors::Abort;
use crate::mem::{end_lr_type, GARBAGE};
use crate::mem_types::ListStateRecord;
use crate::node::{
    AnyBox, AnyNode, CharNode, HListNode, IsBox, NodePtr, RuleDimen, VListNode, WhatsitNode,
    WhatsitSubtype,
};
use crate::params::{DimenPar, GluePar, IntPar};
use crate::state::{EngineState, EqtbState, FontState, MemState, ScannerState, ShipoutState};
use crate::units::{GlueOrder, GlueSign, MemPtr, NodeType, OrderTotals, Scaled};

mod copy;
mod vert_break;

use self::vert_break::push_slot;
pub(crate) use self::vert_break::{vert_break, vsplit, MarksAction};

/// The word of a box whose link field is its `list_ptr`.
const LIST_OFFSET: i32 = 5;

pub(crate) const EXACTLY: i16 = 0;
pub(crate) const ADDITIONAL: i16 = 1;

impl ListStateRecord {
    /// Append box `b` to the current vertical list, preceded by interline glue.
    pub fn append_to_vlist(
        &mut self,
        mem: &mut MemState,
        eqtb: &EqtbState,
        b: AnyBox,
    ) -> Result<(), Abort> {
        let upwards = eqtb.int_par(IntPar::XetexUpwards) > 0;
        let prev_depth = self.prev_depth();
        if prev_depth > IGNORE_DEPTH {
            let baseline_skip = eqtb.glue_par(GluePar::BaselineSkip);
            let d = baseline_skip.width(mem)
                - prev_depth
                - if upwards { b.depth(mem) } else { b.height(mem) };
            let p = if d < eqtb.dimen_par(DimenPar::LineSkipLimit) {
                mem.new_param_glue(eqtb, GluePar::LineSkip)?
            } else {
                let p = mem.new_skip_param(eqtb, GluePar::BaselineSkip)?;
                p.glue_ptr(mem).set_width(mem, d);
                p
            };
            self.tail_append(mem, p.ptr());
        }
        self.tail_append(mem, b.ptr());
        self.set_prev_depth(if upwards { b.height(mem) } else { b.depth(mem) });
        Ok(())
    }
}

pub(crate) fn append_to_vlist(st: &mut EngineState, b: AnyBox) -> Result<(), Abort> {
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
    r_box: AnyBox,
    diags: [Option<PackDiag>; 2],
}

impl MemState {
    /// Set `r`'s glue to distribute `excess` over the highest-order `totals`,
    /// returning that order.
    fn set_box_glue<K: IsBox>(
        &mut self,
        r: NodePtr<K>,
        sign: GlueSign,
        excess: f64,
        totals: &OrderTotals,
    ) -> GlueOrder {
        let o = totals.max_order();
        r.set_glue_order(self, o);
        r.set_glue_sign(self, sign);
        if totals[o] != Scaled::ZERO {
            r.set_glue_set(self, excess / f64::from(totals[o].raw()));
        } else {
            r.set_glue_sign(self, GlueSign::Normal);
            r.set_glue_set(self, 0.0);
        }
        o
    }

    fn clear_box_glue<K: IsBox>(&mut self, r: NodePtr<K>) {
        r.set_glue_sign(self, GlueSign::Normal);
        r.set_glue_order(self, GlueOrder::Normal);
        r.set_glue_set(self, 0.0);
    }

    /// `replace_count(q)` if `type(q)=disc_node`, read like TeX does even when
    /// `q` is a char node (whose font sits in the type field).
    fn disc_replace_count(&self, q: MemPtr) -> Option<i32> {
        match self.as_char(q) {
            Some(ch) => (ch.font(self).raw() == NodeType::Disc as i32)
                .then(|| i32::from(ch.character(self))),
            None => self.as_disc(q).map(|disc| i32::from(disc.subtype(self))),
        }
    }

    /// Merge the run of native words in `w`'s font that follows `w` (possibly
    /// across empty discretionaries) into one node. `q` trails `w`. Returns the
    /// new `q` and the node now in `w`'s place.
    fn merge_native_words(
        &mut self,
        eqtb: &EqtbState,
        fonts: &FontState,
        r_node: NodePtr<HListNode>,
        mut q: MemPtr,
        w: NodePtr<WhatsitNode>,
    ) -> Result<(MemPtr, NodePtr<WhatsitNode>), Abort> {
        let p = w.ptr();
        let font = w.native_font(self);
        let same_font = |mem: &MemState, p: MemPtr| {
            mem.as_native_word(p).filter(|w| w.native_font(mem) == font)
        };

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
                pp = next.link(self);
            } else if let Some(next) = self
                .as_disc(pp)
                .and_then(|disc| same_font(self, disc.link(self)))
            {
                pp = next.link(self);
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
                total_chars += i32::from(sw.native_length(self));
            }
            s = s.link(self);
        }

        let merged = self.new_native_word_node(eqtb, font, total_chars)?;
        merged.set_subtype(self, w.subtype(self));
        q.set_link(self, merged.ptr());
        let last = self.predecessor(p, pp);
        merged.set_link(self, last.link(self));
        last.set_link(self, MemPtr::NULL);

        total_chars = 0;
        let mut s = p;
        loop {
            if let Some(sw) = self.as_native_word(s) {
                let count = sw.native_length(self) as usize;
                merged.mark_native_text(self);
                let from = sw.native_text_start();
                let to = merged.native_text_start() + total_chars as usize;
                self.units_mut().copy_within(from..from + count, to);
                total_chars += count as i32;
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
        w: Scaled,
        m: i16,
    ) -> Result<PackOutcome, Abort> {
        shipout.last_badness = 0;
        let r_node = NodePtr::<HListNode>::alloc(self)?;
        r_node.set_subtype(self, 0);
        r_node.set_shift_amount(self, Scaled::ZERO);
        r_node.set_list_ptr(self, list);
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
            self.push_lr(&mut scanner.lr_ptr, BEFORE.into())?;
        }

        let mut p = list;
        'outer: while !p.is_null() {
            while let Some(ch) = self.as_char(p) {
                let f = ch.font(self);
                let c = fonts.effective_char(f, ch.character(self));
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
                Ok(AnyNode::HList(b)) => {
                    let s = b.shift_amount(self);
                    x += b.width(self);
                    h = h.max(b.height(self) - s);
                    d = d.max(b.depth(self) + s);
                }
                Ok(AnyNode::VList(b)) => {
                    let s = b.shift_amount(self);
                    x += b.width(self);
                    h = h.max(b.height(self) - s);
                    d = d.max(b.depth(self) + s);
                }
                Ok(AnyNode::Rule(rule)) => {
                    x += rule.width(self).to_scaled();
                    h = h.max(rule.height(self).to_scaled());
                    d = d.max(rule.depth(self).to_scaled());
                }
                Ok(AnyNode::Unset(u)) => {
                    x += u.width(self);
                    h = h.max(u.height(self));
                    d = d.max(u.depth(self));
                }
                Ok(AnyNode::Adjust(adjust)) => {
                    if !scanner.adjust_tail.is_null() || !scanner.pre_adjust_tail.is_null() {
                        q = self.predecessor(q, p);
                        let adjust_list = adjust.adjust_ptr(self);
                        let tail = if adjust.subtype(self) != 0 {
                            &mut scanner.pre_adjust_tail
                        } else {
                            &mut scanner.adjust_tail
                        };
                        if tail.is_null() {
                            return Err(Abort::Confusion("pre vadjust"));
                        }
                        tail.set_link(self, adjust_list);
                        *tail = self.list_tail(*tail);
                        p = adjust.link(self);
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
                Ok(AnyNode::Whatsit(w)) => match w.whatsit_subtype(self) {
                    Some(WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt) => {
                        let (new_q, native) = self.merge_native_words(eqtb, fonts, r_node, q, w)?;
                        q = new_q;
                        p = native.ptr();
                        h = h.max(native.height(self));
                        d = d.max(native.depth(self));
                        x += native.width(self);
                    }
                    Some(WhatsitSubtype::Glyph | WhatsitSubtype::Pic | WhatsitSubtype::Pdf) => {
                        h = h.max(w.height(self));
                        d = d.max(w.depth(self));
                        x += w.width(self);
                    }
                    _ => {}
                },
                Ok(AnyNode::Glue(glue)) => {
                    let spec = glue.glue_ptr(self);
                    x += spec.width(self);
                    scanner.total_stretch[spec.stretch_order(self)] += spec.stretch(self);
                    scanner.total_shrink[spec.shrink_order(self)] += spec.shrink(self);
                    if glue.subtype(self) >= A_LEADERS {
                        let leader = glue.leader_ptr(self);
                        let (leader_h, leader_d) = match self.classify(leader) {
                            Ok(AnyNode::HList(b)) => (b.height(self), b.depth(self)),
                            Ok(AnyNode::VList(b)) => (b.height(self), b.depth(self)),
                            Ok(AnyNode::Rule(rule)) => {
                                (rule.height(self).to_scaled(), rule.depth(self).to_scaled())
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
                    x += kern.width(self);
                }
                Ok(AnyNode::MarginKern(kern)) => {
                    x += kern.width(self);
                }
                Ok(AnyNode::Math(math)) => {
                    x += math.width(self);
                    if texxet {
                        let subtype = math.subtype(self);
                        if subtype % 2 == 1 {
                            if scanner.lr_ptr.info(self) == end_lr_type(subtype) {
                                self.pop_lr(&mut scanner.lr_ptr);
                            } else {
                                scanner.lr_problems += 1;
                                math.into_kern(self).set_subtype(self, EXPLICIT);
                            }
                        } else {
                            self.push_lr(&mut scanner.lr_ptr, end_lr_type(subtype))?;
                        }
                    }
                }
                Ok(AnyNode::Ligature(lig)) => {
                    // Measure the ligature as its character through the scratch char node.
                    let scratch = NodePtr::<CharNode>::at(GARBAGE);
                    scratch.set_font(self, lig.font(self));
                    scratch.set_character(self, lig.character(self));
                    GARBAGE.set_link(self, lig.link(self));
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
        r_node.set_height(self, h);
        r_node.set_depth(self, d);

        let w = if m == ADDITIONAL { x + w } else { w };
        r_node.set_width(self, w);
        x = w - x;

        let has_list = !r_node.list_ptr(self).is_null();
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
                r_node.set_glue_set(self, 1.0);
                let excess = (-x) - scanner.total_shrink[GlueOrder::Normal];
                let hfuzz = eqtb.dimen_par(DimenPar::Hfuzz);
                if excess > hfuzz || eqtb.int_par(IntPar::Hbadness) < 100 {
                    let overfull_rule = eqtb.dimen_par(DimenPar::OverfullRule);
                    if overfull_rule > Scaled::ZERO && excess > hfuzz {
                        q = self.list_tail(q);
                        let rule = self.new_rule()?;
                        q.set_link(self, rule.ptr());
                        rule.set_width(self, RuleDimen::Set(overfull_rule));
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
            if scanner.lr_ptr.info(self) != BEFORE.into() {
                q = self.list_tail(q);
                loop {
                    let math = self.new_math(Scaled::ZERO, scanner.lr_ptr.info(self) as u16)?;
                    q.set_link(self, math.ptr());
                    q = math.ptr();
                    scanner.lr_problems += 10000;
                    self.pop_lr(&mut scanner.lr_ptr);
                    if scanner.lr_ptr.info(self) == BEFORE.into() {
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
            r_box: AnyBox::HList(r_node),
            diags,
        })
    }
}

pub(crate) fn print_hpack_diag(st: &mut EngineState, box_ptr: AnyBox, diag: PackDiag) {
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

fn print_hpack_common_ending(st: &mut EngineState, box_ptr: AnyBox) {
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
    let list = box_ptr.list_ptr(&st.mem);
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
    w: Scaled,
    m: i16,
) -> Result<AnyBox, Abort> {
    let outcome = st.mem.hpack(
        &st.eqtb,
        &mut st.fonts,
        &mut st.scanner,
        &mut st.shipout,
        list,
        w,
        m,
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
        h: Scaled,
        m: i16,
        l: Scaled,
    ) -> Result<PackOutcome, Abort> {
        shipout.last_badness = 0;
        let r_node = NodePtr::<VListNode>::alloc(self)?;
        let upwards = eqtb.int_par(IntPar::XetexUpwards) > 0;
        r_node.set_subtype(self, u16::from(upwards));
        r_node.set_shift_amount(self, Scaled::ZERO);
        r_node.set_list_ptr(self, list);

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
                Ok(AnyNode::HList(b)) => {
                    x += d + b.height(self);
                    d = b.depth(self);
                    w = w.max(b.width(self) + b.shift_amount(self));
                }
                Ok(AnyNode::VList(b)) => {
                    x += d + b.height(self);
                    d = b.depth(self);
                    w = w.max(b.width(self) + b.shift_amount(self));
                }
                Ok(AnyNode::Rule(rule)) => {
                    x += d + rule.height(self).to_scaled();
                    d = rule.depth(self).to_scaled();
                    w = w.max(rule.width(self).to_scaled());
                }
                Ok(AnyNode::Unset(u)) => {
                    x += d + u.height(self);
                    d = u.depth(self);
                    w = w.max(u.width(self));
                }
                Ok(AnyNode::Whatsit(wn)) => {
                    if matches!(
                        wn.whatsit_subtype(self),
                        Some(WhatsitSubtype::Pic | WhatsitSubtype::Pdf)
                    ) {
                        x += d + wn.height(self);
                        d = wn.depth(self);
                        w = w.max(wn.width(self));
                    }
                }
                Ok(AnyNode::Glue(glue)) => {
                    x += d;
                    d = Scaled::ZERO;
                    let spec = glue.glue_ptr(self);
                    x += spec.width(self);
                    scanner.total_stretch[spec.stretch_order(self)] += spec.stretch(self);
                    scanner.total_shrink[spec.shrink_order(self)] += spec.shrink(self);
                    if glue.subtype(self) >= A_LEADERS {
                        let leader = glue.leader_ptr(self);
                        let leader_w = match self.classify(leader) {
                            Ok(AnyNode::HList(b)) => b.width(self),
                            Ok(AnyNode::VList(b)) => b.width(self),
                            Ok(AnyNode::Rule(rule)) => rule.width(self).to_scaled(),
                            _ => Scaled::from_raw(self.b32s1(leader + 1)),
                        };
                        w = w.max(leader_w);
                    }
                }
                Ok(AnyNode::Kern(kern)) => {
                    x += d + kern.width(self);
                    d = Scaled::ZERO;
                }
                _ => {}
            }
            p = p.link(self);
        }

        r_node.set_width(self, w);
        if d > l {
            x += d - l;
            r_node.set_depth(self, l);
        } else {
            r_node.set_depth(self, d);
        }

        let h = if m == ADDITIONAL { x + h } else { h };
        r_node.set_height(self, h);
        x = h - x;

        let has_list = !r_node.list_ptr(self).is_null();
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
                r_node.set_glue_set(self, 1.0);
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
            r_box: AnyBox::VList(r_node),
            diags,
        })
    }
}

pub(crate) fn print_vpack_diag(st: &mut EngineState, box_ptr: AnyBox, diag: PackDiag) {
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

fn print_vpack_common_ending(st: &mut EngineState, box_ptr: AnyBox) {
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
    h: Scaled,
    m: i16,
    l: Scaled,
) -> Result<AnyBox, Abort> {
    let outcome = st
        .mem
        .vpackage(&st.eqtb, &mut st.scanner, &mut st.shipout, list, h, m, l)?;
    for diag in outcome.diags.into_iter().flatten() {
        print_vpack_diag(st, outcome.r_box, diag);
    }
    Ok(outcome.r_box)
}
