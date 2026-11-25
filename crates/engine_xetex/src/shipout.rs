//! Shipping boxes out to the XDV file.

use crate::base::consts::{INVALID_HANDLE, MAX_DIMEN, MIN_HALFWORD, NULL_FONT};
use crate::base::params::{DimenPar, IntPar};
use crate::base::units::{Dir, FontId, GlueOrder, GlueSign, MemIdx, MemPtr, Scaled, StrNumber};
use crate::build::packaging::LrState;
use crate::build::pagebuilder::PageContents;
use crate::engine::output_file_extension;
use crate::env::EngineEnv;
use crate::ffi::bridge::{output_close, output_flush, output_open, strerror};
use crate::files::io::{make_name_string, open_log_file, pack_job_name};
use crate::fonts::FontState;
use crate::fonts::ext::apply_tfm_font_mapping;
use crate::memory::glue_store::GlueSpecId;
use crate::memory::glyph_arena::FixedPoint;
use crate::memory::mem::MemState;
use crate::memory::node::{
    AnyNode, BoxLr, BoxNode, CharNode, EdgeNode, GlueKind, GlueNode, GlyphNode, KernNode,
    KernSubtype, LeaderKind, LigatureNode, LinkSlot, MathCode, MathNode, NativeWordNode, NodePtr,
    RUNNING, RuleNode, Stored, View, WhatsitSubtype, WhatsitVariant,
};
use crate::print::diagnostic::{begin_warning, end_capture};
use crate::print::errors::{Abort, OrConfusion, error};
use crate::print::{
    Selector, begin_diagnostic, end_diagnostic, print, print_char, print_int, print_ln, print_nl,
    print_scaled, show_box,
};
use crate::shipout::dvi::{Axis, DviFontMetrics, DviWriter, MoveReg};
use crate::shipout::synctex::{
    synctex_current, synctex_hlist, synctex_horizontal_rule_or_glue, synctex_kern, synctex_math,
    synctex_sheet, synctex_teehs, synctex_tsilh, synctex_tsilv, synctex_vlist, synctex_void_hlist,
    synctex_void_vlist,
};
use crate::state::EngineState;
use crate::tables::cs::CsId;
use crate::tables::eqtb::EqtbState;
use crate::tables::save::prepare_mag;
use crate::tables::string_pool::StringsState;

pub(crate) mod dvi;
pub(crate) mod pic;
pub(crate) mod synctex;
mod whatsits;

use whatsits::pic_out;
pub(crate) use whatsits::{ensure_write_stream_open, out_what};

const XDV_ID_BYTE: u8 = 7;
const SPX_ID_BYTE: u8 = 100;

const MOV_NONE_SEEN: i32 = 0;
const MOV_Y_HERE: i32 = 1;
const MOV_Z_HERE: i32 = 2;
const MOV_YZ_OK: i32 = 3;
const MOV_Y_OK: i32 = 4;
const MOV_Z_OK: i32 = 5;
const MOV_Y_SEEN: i32 = 6;
const MOV_D_FIXED: i32 = 6;
const MOV_Z_SEEN: i32 = 12;

/// Slack TeX adds to leader glue against floating-point rounding.
const LEADER_SLOP: Scaled = Scaled::from_raw(10);

fn leader_count_and_rest(total: Scaled, size: Scaled) -> (i32, Scaled) {
    (total.raw() / size.raw(), total % size.raw())
}

pub(crate) const ONE_INCH: Scaled = Scaled::from_raw(4_736_287);
const ONE_INCH_LESS_ONE: Scaled = Scaled::from_raw(ONE_INCH.raw() - 1);

impl MemState {
    pub fn new_edge(&mut self, dir: Dir, w: Scaled) -> Result<NodePtr<EdgeNode>, Abort> {
        let p = NodePtr::<EdgeNode>::alloc(self)?;
        let node = &mut self[p];
        node.dir = dir.into();
        node.width = w;
        node.edge_dist = Scaled::ZERO;
        Ok(p)
    }
}

fn synch_h(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    if st.shipout.cur_h != st.shipout.dvi_h {
        let delta = st.shipout.cur_h - st.shipout.dvi_h;
        st.shipout.movement(env, delta, Axis::Right)?;
        st.shipout.dvi_h = st.shipout.cur_h;
    }
    Ok(())
}

fn synch_v(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    if st.shipout.cur_v != st.shipout.dvi_v {
        let delta = st.shipout.cur_v - st.shipout.dvi_v;
        st.shipout.movement(env, delta, Axis::Down)?;
        st.shipout.dvi_v = st.shipout.cur_v;
    }
    Ok(())
}

fn dvi_font_def(st: &mut EngineState, env: &mut EngineEnv, f: FontId) -> Result<(), Abort> {
    let k = f.dvi_number();
    if st.fonts.is_native_font(f) {
        let def = st.fonts.make_font_def(f)?;
        return st.shipout.dvi.native_font_def(env, k, &def);
    }
    let font_name = st.fonts.font_name[f];
    let name = &st.strings.str_pool[st.strings.str_range(font_name)];
    // Only the part before a `:features` suffix is written (a leading colon does not count).
    let name_len = name
        .iter()
        .skip(1)
        .position(|&c| c == u16::from(b':'))
        .map_or(name.len(), |l| l + 1);
    let area = &st.strings.str_pool[st.strings.str_range(st.fonts.font_area[f])];
    let metrics = DviFontMetrics {
        check: st.fonts.check_bytes(f),
        size: st.fonts.font_size[f],
        design_size: st.fonts.font_dsize[f],
    };
    st.shipout
        .dvi
        .fnt_def(env, k, metrics, area, &name[..name_len])
}

pub fn initialize_shipout_variables(st: &mut EngineState) {
    let s = &mut st.shipout;
    s.output_file_name = StrNumber::NONE;
    s.dvi.init();
    s.down_moves.clear();
    s.right_moves.clear();
}

fn dvi_special_from_cur_string(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    st.shipout.dvi.special(env, st.strings.cur_string())?;
    st.strings.discard_cur_string();
    Ok(())
}

pub(crate) fn ship_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    page: NodePtr<BoxNode>,
) -> Result<(), Abort> {
    let mut lr = LrState::START;
    let p = page.ptr();
    let mag = st.eqtb.int_par(IntPar::Mag);
    let job_name = st.strings.texstring_bytes(st.strings.job_name);
    synctex_sheet(st, env, mag, &job_name);

    if st.strings.job_name == StrNumber::NONE {
        open_log_file(st, env)?;
    }
    let tracing_output = st.eqtb.int_par(IntPar::TracingOutput) > 0;
    if tracing_output {
        print_nl(st, env, "");
        print_ln(st, env);
        print(st, env, "Completed box being shipped out");
    }
    if st.shipout.term_offset > st.config.max_print_line - 9 {
        print_ln(st, env);
    } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
        print_char(st, env, b' ');
    }

    print_char(st, env, b'[');
    let mut j: u8 = 9;
    while j > 0 && st.eqtb.count(j) == 0 {
        j -= 1;
    }
    for k in 0..=j {
        print_int(st, env, st.eqtb.count(k));
        if k < j {
            print_char(st, env, b'.');
        }
    }
    output_flush(env, st.shipout.rust_stdout);

    if tracing_output {
        print_char(st, env, b']');
        begin_diagnostic(st);
        show_box(st, env, p);
        end_diagnostic(st, env, true);
    }

    let (height, depth, width) = (st.mem[page].height, st.mem[page].depth, st.mem[page].width);
    let h_offset = st.eqtb.dimen_par(DimenPar::HOffset);
    let v_offset = st.eqtb.dimen_par(DimenPar::VOffset);
    if height > MAX_DIMEN
        || depth > MAX_DIMEN
        || height + depth + v_offset > MAX_DIMEN
        || width + h_offset > MAX_DIMEN
    {
        error(
            st,
            env,
            "Huge page cannot be shipped out",
            &[
                "The page just created is more than 18 feet tall or",
                "more than 18 feet wide, so I suspect something went wrong.",
            ],
        )?;

        if st.eqtb.int_par(IntPar::TracingOutput) <= 0 {
            begin_diagnostic(st);
            print_nl(st, env, "The following box has been deleted:");
            show_box(st, env, p);
            end_diagnostic(st, env, true);
        }
    } else {
        ship_page(st, env, &mut lr, page)?;
    }

    if lr.problems > 0 {
        print_ln(st, env);
        print_nl(st, env, "\\endL or \\endR problem (");
        print_int(st, env, lr.problems / 10000);
        print(st, env, " missing, ");
        print_int(st, env, lr.problems % 10000);
        print(st, env, " extra");
        print_char(st, env, b')');
        print_ln(st, env);
    }

    if !lr.ptr.is_none() || lr.dir != Dir::Ltr {
        return Err(Abort::Confusion("LR3"));
    }
    if st.eqtb.int_par(IntPar::TracingOutput) <= 0 {
        print_char(st, env, b']');
    }
    st.shipout.dead_cycles = 0;
    output_flush(env, st.shipout.rust_stdout);
    st.mem.flush_node_list(p)?;
    synctex_teehs(st, env);
    Ok(())
}

fn ship_page(
    st: &mut EngineState,
    env: &mut EngineEnv,
    lr: &mut LrState,
    page: NodePtr<BoxNode>,
) -> Result<(), Abort> {
    let (height, depth, width) = (st.mem[page].height, st.mem[page].depth, st.mem[page].width);
    let h_offset = st.eqtb.dimen_par(DimenPar::HOffset);
    let v_offset = st.eqtb.dimen_par(DimenPar::VOffset);
    let pdf_page_width = st.eqtb.dimen_par(DimenPar::PdfPageWidth);
    let pdf_page_height = st.eqtb.dimen_par(DimenPar::PdfPageHeight);

    st.shipout.max_v = st.shipout.max_v.max(height + depth + v_offset);
    st.shipout.max_h = st.shipout.max_h.max(width + h_offset);
    st.shipout.dvi_h = Scaled::ZERO;
    st.shipout.dvi_v = Scaled::ZERO;
    st.shipout.cur_h = h_offset;
    st.shipout.dvi_f = NULL_FONT;
    st.shipout.cur_h_offset = h_offset + ONE_INCH_LESS_ONE;
    st.shipout.cur_v_offset = v_offset + ONE_INCH_LESS_ONE;
    st.shipout.cur_page_width = if pdf_page_width != Scaled::ZERO {
        pdf_page_width
    } else {
        width + st.shipout.cur_h_offset * 2
    };
    st.shipout.cur_page_height = if pdf_page_height != Scaled::ZERO {
        pdf_page_height
    } else {
        height + depth + st.shipout.cur_v_offset * 2
    };

    if st.shipout.output_file_name == StrNumber::NONE {
        if st.strings.job_name == StrNumber::NONE {
            open_log_file(st, env)?;
        }
        let path = pack_job_name(st, output_file_extension(st))?;
        st.shipout.dvi.file = output_open(env, &path, false);
        if st.shipout.dvi.file == INVALID_HANDLE {
            return Err(Abort::CannotOpenOutput {
                name: path.as_slice().into(),
            });
        }
        st.shipout.output_file_name = make_name_string(st, &path)?;
    }

    if st.shipout.total_pages == 0 {
        let id = id_byte(st.config.semantic_pagination_enabled);
        st.shipout.dvi.begin_preamble(env, id)?;
        prepare_mag(st, env)?;
        let mag = st.eqtb.int_par(IntPar::Mag);
        st.shipout.dvi.end_preamble(env, mag, b"tectonic")?;
    }

    let counts = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9].map(|k| st.eqtb.count(k));
    st.shipout.dvi.bop(env, counts)?;

    let old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    print(st, env, "pdf:pagesize ");
    if pdf_page_width <= Scaled::ZERO || pdf_page_height <= Scaled::ZERO {
        print(st, env, "default");
    } else {
        print(st, env, "width");
        print(st, env, b' ');
        print_scaled(st, env, pdf_page_width);
        print(st, env, "pt");
        print(st, env, b' ');
        print(st, env, "height");
        print(st, env, b' ');
        print_scaled(st, env, pdf_page_height);
        print(st, env, "pt");
    }
    st.shipout.selector = old_setting;

    dvi_special_from_cur_string(st, env)?;

    st.shipout.cur_v = height + v_offset;
    if st.mem[page].is_vlist() {
        vlist_out(st, env, lr, page)?;
    } else {
        hlist_out(st, env, lr, page)?;
    }

    st.shipout.dvi.eop(env, st.scanner.logical_clock)?;
    st.shipout.total_pages += 1;
    st.shipout.dvi.depth = -1;
    Ok(())
}

fn id_byte(semantic_pagination: bool) -> u8 {
    if semantic_pagination {
        SPX_ID_BYTE
    } else {
        XDV_ID_BYTE
    }
}

fn dvi_font_select(st: &mut EngineState, env: &mut EngineEnv, f: FontId) -> Result<(), Abort> {
    if f == st.shipout.dvi_f {
        return Ok(());
    }
    if !st.fonts.font_used[f] {
        dvi_font_def(st, env, f)?;
        st.fonts.font_used[f] = true;
    }
    st.shipout.dvi.fnt(env, f.dvi_number())?;
    st.shipout.dvi_f = f;
    Ok(())
}

/// The glue-setting state of the box being shipped, carried across glue nodes
/// so rounding errors do not accumulate.
struct GlueRounding {
    order: Stored<GlueOrder>,
    sign: Stored<GlueSign>,
    set: f64,
    cur_g: Scaled,
    cur_glue: f64,
}

impl GlueRounding {
    fn new(mem: &MemState, this_box: NodePtr<BoxNode>) -> GlueRounding {
        GlueRounding {
            order: mem[this_box].glue_order,
            sign: mem[this_box].glue_sign,
            set: mem[this_box].glue_set.get(),
            cur_g: Scaled::ZERO,
            cur_glue: 0.0,
        }
    }

    fn is_set(&self, mem: &MemState, spec: GlueSpecId) -> bool {
        match self.sign.get() {
            Some(GlueSign::Stretching) => mem[spec].stretch_order == self.order,
            Some(GlueSign::Shrinking) => mem[spec].shrink_order == self.order,
            Some(GlueSign::Normal) | None => false,
        }
    }

    fn width(&mut self, mem: &MemState, spec: GlueSpecId) -> Scaled {
        let width = mem[spec].width - self.cur_g;
        if self.is_set(mem, spec) {
            if self.sign == GlueSign::Stretching {
                self.cur_glue += f64::from(mem[spec].stretch.raw());
            } else {
                self.cur_glue -= f64::from(mem[spec].shrink.raw());
            }
            let glue_temp = (self.set * self.cur_glue).clamp(-1_000_000_000.0, 1_000_000_000.0);
            self.cur_g = Scaled::round(glue_temp);
        }
        width + self.cur_g
    }
}

enum Leader {
    Rule(NodePtr<RuleNode>),
    Box(NodePtr<BoxNode>),
}

impl FontState {
    fn font_character_width(&self, f: FontId, c: i32) -> Scaled {
        self.char_metrics(f, c).width
    }
}

struct NativeWordRun {
    first: NodePtr<NativeWordNode>,
    last: NodePtr<NativeWordNode>,
    text_len: i32,
}

impl MemState {
    fn is_space_adjustment(&self, q: MemPtr) -> bool {
        matches!(self.classify_non_char(q), Some(AnyNode::Kern(k)) if self[k].subtype == KernSubtype::SpaceAdjustment)
    }

    /// Merge runs of native words separated by interword glue into single nodes
    /// so they can be justified as a whole.
    fn merge_justifiable_native_words(
        &mut self,
        strings: &mut StringsState,
        fonts: &FontState,
        eqtb: &EqtbState,
        this_box: NodePtr<BoxNode>,
    ) -> Result<(), Abort> {
        let mut prev_p = LinkSlot::List(this_box);
        let mut p = self[this_box].list_ptr;

        while let Some(here) = p {
            let mut here = here;
            let first = self.as_native_word(p).filter(|&r| {
                fonts.font_letter_space[self[r].font] == Scaled::ZERO && here.link(self).is_some()
            });
            if let Some(r) = first {
                let font = self[r].font;
                let same_font_word = |mem: &MemState, q: MemPtr| {
                    mem.as_native_word(q).filter(|&w| mem[w].font == font)
                };
                let mut last = r;
                let mut text_len = i32::from(self[r].length);
                let mut q = self[r].link;
                loop {
                    q = self.skip_interword_fillers(q);
                    match self.classify_non_char(q) {
                        Some(AnyNode::Glue(glue)) if self[glue].subtype == GlueKind::Normal => {
                            if Some(self[glue].glue_ptr) == fonts.font_glue[font] {
                                q = self.skip_interword_fillers(self[glue].link);
                                if let Some(w) = same_font_word(self, q) {
                                    last = w;
                                    text_len += 1 + i32::from(self[w].length);
                                    q = self[w].link;
                                    continue;
                                }
                            } else {
                                q = self[glue].link;
                            }
                            if self.is_space_adjustment(q) {
                                q = self.skip_interword_fillers(q.and_then(|k| k.link(self)));
                                if let Some(w) = same_font_word(self, q) {
                                    last = w;
                                    text_len += 1 + i32::from(self[w].length);
                                    q = self[w].link;
                                    continue;
                                }
                            }
                        }
                        Some(AnyNode::Whatsit(_)) => {
                            if let Some(w) = same_font_word(self, q) {
                                last = w;
                                q = self[w].link;
                                continue;
                            }
                        }
                        _ => {}
                    }
                    break;
                }

                if last != r {
                    let run = NativeWordRun {
                        first: r,
                        last,
                        text_len,
                    };
                    here =
                        self.merge_native_word_run(strings, fonts, eqtb, this_box, prev_p, run)?;
                }
            }
            prev_p = LinkSlot::Node(here);
            p = here.link(self);
        }
        Ok(())
    }

    fn merge_native_word_run(
        &mut self,
        strings: &mut StringsState,
        fonts: &FontState,
        eqtb: &EqtbState,
        this_box: NodePtr<BoxNode>,
        prev_p: LinkSlot,
        run: NativeWordRun,
    ) -> Result<MemIdx, Abort> {
        let NativeWordRun {
            first,
            last,
            text_len,
        } = run;
        if strings.pool_ptr + text_len > strings.pool_size {
            return Err(strings.pool_overflow());
        }

        let g_order = self[this_box].glue_order;
        let g_sign = self[this_box].glue_sign;
        let mut width = Scaled::ZERO;
        let mut q = first.idx();
        loop {
            match self.classify(q) {
                Ok(AnyNode::Whatsit(w)) => {
                    if let Some(WhatsitVariant::NativeWord(word)) = w.variant(self) {
                        for &unit in self.entries(word) {
                            strings.append_char(unit);
                        }
                        width += self[word].width;
                    }
                }
                Ok(AnyNode::Glue(glue)) => {
                    strings.append_char(u16::from(b' '));
                    let spec = self[glue].glue_ptr;
                    width += self[spec].width;
                    match g_sign.get() {
                        Some(GlueSign::Stretching) if self[spec].stretch_order == g_order => {
                            width += Scaled::round(
                                self[this_box].glue_set.get() * f64::from(self[spec].stretch.raw()),
                            );
                        }
                        Some(GlueSign::Shrinking) if self[spec].shrink_order == g_order => {
                            width -= Scaled::round(
                                self[this_box].glue_set.get() * f64::from(self[spec].shrink.raw()),
                            );
                        }
                        _ => {}
                    }
                }
                Ok(AnyNode::Kern(kern)) => width += self[kern].width,
                _ => {}
            }
            if q == last.idx() {
                break;
            }
            q = q.link(self).or_confusion("native word run")?;
        }

        let nlen = strings.cur_length();
        let word = self.new_native_word_node(eqtb, self[first].font, nlen)?;
        self[word].subtype = self[first].subtype;
        let base = strings.cur_string_start().index();
        self.entries_mut(word)
            .copy_from_slice(&strings.str_pool[base..base + nlen as usize]);
        self[word].width = width;
        fonts.store_justified_native_glyphs(self, word)?;

        prev_p.set(self, word.ptr());
        self[word].link = self[last].link;
        self[last].link = None;

        let mut tail = word.idx();
        let mut prev_p = first.idx();
        let mut p = self[first].link;
        while let Some(here) = p {
            if self
                .classify_non_char(p)
                .is_some_and(|n| n.is_interword_filler(self))
            {
                prev_p.set_link(self, here.link(self));
                here.set_link(self, tail.link(self));
                tail.set_link(self, p);
                tail = here;
            }
            prev_p = here;
            p = here.link(self);
        }

        self.flush_node_list(first.ptr())?;
        strings.discard_cur_string();
        Ok(tail)
    }

    /// Replace glue that the box stretched or shrank by a rigid equivalent of
    /// width `w`, so reflected or repeated output sees the width already rounded.
    fn freeze_set_glue(
        &mut self,
        glue: NodePtr<GlueNode>,
        spec: GlueSpecId,
        w: Scaled,
    ) -> Result<(), Abort> {
        self.delete_glue_ref(spec);
        if self[glue].leaders().is_none() {
            glue.into_kern(self, w);
        } else {
            let rigid = self.glue.alloc()?;
            let node = &mut self[rigid];
            node.stretch_order = GlueOrder::Rigid.into();
            node.shrink_order = GlueOrder::Rigid.into();
            node.width = w;
            node.stretch = Scaled::ZERO;
            node.shrink = Scaled::ZERO;
            self[glue].glue_ptr = rigid;
        }
        Ok(())
    }

    fn leader_of(&self, glue: NodePtr<GlueNode>) -> Leader {
        match self[glue].leader_ptr.map(|l| self.classify(l)) {
            Some(Ok(AnyNode::Rule(rule))) => Leader::Rule(rule),
            Some(Ok(AnyNode::Box(b))) => Leader::Box(b),
            other => unreachable!("leaders must be a rule or a box, found {other:?}"),
        }
    }

    fn lr_matches(&self, lr: &LrState, code: MathCode) -> bool {
        self.lr_code(lr.ptr) == Some(code.lr_end())
    }

    fn pop_dvi_level(
        &mut self,
        env: &mut EngineEnv,
        shipout: &mut ShipoutState,
        save_loc: i32,
    ) -> Result<(), Abort> {
        shipout.prune_movements(save_loc);
        shipout.dvi.leave_level(env, save_loc)
    }
}

fn ship_box(
    st: &mut EngineState,
    env: &mut EngineEnv,
    lr: &mut LrState,
    b: NodePtr<BoxNode>,
) -> Result<(), Abort> {
    if st.mem[b].is_vlist() {
        vlist_out(st, env, lr, b)
    } else {
        hlist_out(st, env, lr, b)
    }
}

fn ship_leader_box(
    st: &mut EngineState,
    env: &mut EngineEnv,
    lr: &mut LrState,
    leader: NodePtr<BoxNode>,
) -> Result<(), Abort> {
    let outer_doing_leaders = st.shipout.doing_leaders;
    st.shipout.doing_leaders = true;
    ship_box(st, env, lr, leader)?;
    st.shipout.doing_leaders = outer_doing_leaders;
    Ok(())
}

/// The size of a rule being shipped (WEB's `rule_ht`, `rule_dp`, `rule_wd`).
/// `RUNNING` takes that dimension from the enclosing box.
#[derive(Copy, Clone)]
pub(crate) struct RuleDims {
    pub ht: Scaled,
    pub dp: Scaled,
    pub wd: Scaled,
}

impl RuleDims {
    fn of(mem: &MemState, rule: NodePtr<RuleNode>) -> Self {
        Self {
            ht: mem[rule].height.to_scaled(),
            dp: mem[rule].depth.to_scaled(),
            wd: mem[rule].width.to_scaled(),
        }
    }
}

fn hlist_rule_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    this_box: NodePtr<BoxNode>,
    base_line: Scaled,
    p: MemIdx,
    mut rule: RuleDims,
) -> Result<(), Abort> {
    if rule.ht == RUNNING {
        rule.ht = st.mem[this_box].height;
    }
    if rule.dp == RUNNING {
        rule.dp = st.mem[this_box].depth;
    }
    rule.ht += rule.dp;
    if rule.ht > Scaled::ZERO && rule.wd > Scaled::ZERO {
        synch_h(st, env)?;
        st.shipout.cur_v = base_line + rule.dp;
        synch_v(st, env)?;
        st.shipout.dvi.set_rule(env, rule.ht, rule.wd)?;
        st.shipout.cur_v = base_line;
        st.shipout.dvi_h += rule.wd;
    }
    st.shipout.cur_h += rule.wd;
    synctex_horizontal_rule_or_glue(st, env, p, Some(rule));
    Ok(())
}

fn vlist_rule_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    lr: &mut LrState,
    this_box: NodePtr<BoxNode>,
    left_edge: Scaled,
    upwards: bool,
    mut rule: RuleDims,
) -> Result<(), Abort> {
    if rule.wd == RUNNING {
        rule.wd = st.mem[this_box].width;
    }
    rule.ht += rule.dp;
    st.shipout.move_past_vlist_rule(upwards, rule.ht);
    if rule.ht > Scaled::ZERO && rule.wd > Scaled::ZERO {
        if lr.dir == Dir::Rtl {
            st.shipout.cur_h -= rule.wd;
        }
        synch_h(st, env)?;
        synch_v(st, env)?;
        st.shipout.dvi.put_rule(env, rule.ht, rule.wd)?;
        st.shipout.cur_h = left_edge;
    }
    Ok(())
}

impl ShipoutState {
    fn move_past_vlist_rule(&mut self, upwards: bool, height: Scaled) {
        if upwards {
            self.cur_v -= height;
        } else {
            self.cur_v += height;
        }
    }
}

struct BoxLeaders {
    leader: NodePtr<BoxNode>,
    kind: LeaderKind,
    size: Scaled,
}

/// The box an hlist belongs to and where its baseline and left edge are.
#[derive(Clone, Copy)]
struct HlistFrame {
    this_box: NodePtr<BoxNode>,
    base_line: Scaled,
    left_edge: Scaled,
}

fn hlist_glue_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    lr: &mut LrState,
    frame: HlistFrame,
    glue: NodePtr<GlueNode>,
    rounding: &mut GlueRounding,
) -> Result<(), Abort> {
    let HlistFrame {
        this_box,
        base_line,
        left_edge,
    } = frame;
    let spec = st.mem[glue].glue_ptr;
    let mut wd = rounding.width(&st.mem, spec);
    if rounding.is_set(&st.mem, spec) {
        st.mem.freeze_set_glue(glue, spec, wd)?;
    }

    if let Some(kind) = st.mem[glue].leaders() {
        let leader = match st.mem.leader_of(glue) {
            Leader::Rule(rule) => {
                let rule = RuleDims {
                    wd,
                    ..RuleDims::of(&st.mem, rule)
                };
                return hlist_rule_out(st, env, this_box, base_line, glue.idx(), rule);
            }
            Leader::Box(leader) => leader,
        };
        let leader_wd = st.mem[leader].width;
        if leader_wd > Scaled::ZERO && wd > Scaled::ZERO {
            wd += LEADER_SLOP;
            if lr.dir == Dir::Rtl {
                st.shipout.cur_h -= LEADER_SLOP;
            }
            let edge = st.shipout.cur_h + wd;
            let mut lx = Scaled::ZERO;
            if kind == LeaderKind::Aligned {
                let save_h = st.shipout.cur_h;
                st.shipout.cur_h = left_edge
                    + leader_wd * ((st.shipout.cur_h - left_edge).raw() / leader_wd.raw());
                if st.shipout.cur_h < save_h {
                    st.shipout.cur_h += leader_wd;
                }
            } else {
                let (lq, rest) = leader_count_and_rest(wd, leader_wd);
                if kind == LeaderKind::Centered {
                    st.shipout.cur_h += rest / 2;
                } else {
                    lx = rest / (lq + 1);
                    st.shipout.cur_h += (rest - lx * (lq - 1)) / 2;
                }
            }

            while st.shipout.cur_h + leader_wd <= edge {
                st.shipout.cur_v = base_line + st.mem[leader].shift_amount;
                synch_v(st, env)?;
                synch_h(st, env)?;
                let (save_h, _) = with_saved_dvi_pos(st, env, |st, env| {
                    if lr.dir == Dir::Rtl {
                        st.shipout.cur_h += leader_wd;
                    }
                    ship_leader_box(st, env, lr, leader)
                })?;
                st.shipout.cur_v = base_line;
                st.shipout.cur_h = save_h + leader_wd + lx;
            }

            st.shipout.cur_h = if lr.dir == Dir::Rtl {
                edge
            } else {
                edge - LEADER_SLOP
            };
            return Ok(());
        }
    }

    st.shipout.cur_h += wd;
    synctex_horizontal_rule_or_glue(st, env, glue, None);
    Ok(())
}

fn glyph_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    glyph: NodePtr<GlyphNode>,
) -> Result<(), Abort> {
    synch_h(st, env)?;
    synch_v(st, env)?;
    let glyph = st.mem[glyph];
    dvi_font_select(st, env, glyph.font)?;
    st.shipout.dvi.set_glyph(env, glyph.width, glyph.glyph)?;
    st.shipout.cur_h += glyph.width;
    st.shipout.dvi_h = st.shipout.cur_h;
    Ok(())
}

fn native_word_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    word: NodePtr<NativeWordNode>,
    actual_text: bool,
) -> Result<(), Abort> {
    synch_h(st, env)?;
    synch_v(st, env)?;
    dvi_font_select(st, env, st.mem[word].font)?;
    if actual_text {
        if st.mem[word].length > 0 || st.mem[word].glyph_info != 0 {
            let (width, locations, gids) = glyph_array(&st.mem, word);
            let text = st.mem.entries(word);
            st.shipout
                .dvi
                .set_text_and_glyphs(env, text, width, locations, gids)?;
        }
    } else if st.mem[word].glyph_info != 0 {
        let (width, locations, gids) = glyph_array(&st.mem, word);
        st.shipout.dvi.set_glyphs(env, width, locations, gids)?;
    }
    st.shipout.cur_h += st.mem[word].width;
    st.shipout.dvi_h = st.shipout.cur_h;
    Ok(())
}

impl ShipoutState {
    fn pdf_save_pos(&mut self) {
        self.pdf_last_x_pos = (self.cur_h + ONE_INCH_LESS_ONE).raw();
        self.pdf_last_y_pos = (self.cur_page_height - self.cur_v - ONE_INCH_LESS_ONE).raw();
    }
}

/// A character to output: a char node's, or a ligature's, which TFM font mappings skip.
struct OutChar {
    node: MemIdx,
    font: FontId,
    chr: u16,
    ligature: bool,
    next: MemPtr,
}

impl OutChar {
    fn of_char(mem: &MemState, ch: NodePtr<CharNode>) -> OutChar {
        let n = mem[ch];
        OutChar {
            node: ch.idx(),
            font: n.font,
            chr: n.character,
            ligature: false,
            next: n.link,
        }
    }

    fn of_ligature(mem: &MemState, lig: NodePtr<LigatureNode>) -> OutChar {
        let n = mem[lig];
        OutChar {
            node: lig.idx(),
            font: n.font,
            chr: n.character,
            ligature: true,
            next: n.link,
        }
    }
}

fn chars_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    first: OutChar,
) -> Result<(MemPtr, LinkSlot), Abort> {
    synch_h(st, env)?;
    synch_v(st, env)?;
    let mut cur = first;
    loop {
        let f = cur.font;
        let mut c = cur.chr;
        if !cur.ligature
            && let Some(mapping) = st.fonts.font_mapping[f].as_deref()
        {
            c = apply_tfm_font_mapping(mapping, c as libc::c_int) as u16;
        }
        dvi_font_select(st, env, f)?;
        if st.fonts.char_exists(f, i32::from(c)) {
            st.shipout.dvi.set_char(env, c)?;
            st.shipout.cur_h += st.fonts.font_character_width(f, i32::from(c));
        }
        match st.mem.as_char(cur.next) {
            Some(ch) => cur = OutChar::of_char(&st.mem, ch),
            None => break,
        }
    }
    synctex_current(st, env);
    st.shipout.dvi_h = st.shipout.cur_h;
    Ok((cur.next, LinkSlot::Node(cur.node)))
}

fn hlist_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    lr: &mut LrState,
    this_box: NodePtr<BoxNode>,
) -> Result<(), Abort> {
    st.mem.temp_ptr = this_box.ptr();
    let mut rounding = GlueRounding::new(&st.mem, this_box);
    if st.eqtb.int_par(IntPar::XetexInterwordSpaceShaping) > 1 {
        st.mem
            .merge_justifiable_native_words(&mut st.strings, &st.fonts, &st.eqtb, this_box)?;
    }

    // A raw cursor: nodes get retyped in place.
    let mut p = st.mem[this_box].list_ptr;
    let save_loc = st.shipout.dvi.enter_level(env)?;
    let base_line = st.shipout.cur_v;
    let mut prev_p = LinkSlot::List(this_box);
    st.mem.push_lr(&mut lr.ptr, MathCode::Before)?;

    if st.mem[this_box].lr() == BoxLr::Dlist {
        if lr.dir == Dir::Rtl {
            lr.dir = Dir::Ltr;
            st.shipout.cur_h -= st.mem[this_box].width;
        } else {
            st.mem[this_box].set_lr(BoxLr::Normal);
        }
    }

    if lr.dir == Dir::Rtl && st.mem[this_box].lr() != BoxLr::Reversed {
        let save_h = st.shipout.cur_h;
        st.mem.temp_ptr = p;
        let kern = st.mem.new_kern(Scaled::ZERO)?;
        st.mem[kern].sync.tag = 0;
        prev_p.set(&mut st.mem, kern.ptr());
        st.shipout.cur_h = Scaled::ZERO;
        let reversed = st
            .mem
            .reverse(&mut st.shipout, lr, &mut st.fonts, None, &mut rounding)?;
        let node = &mut st.mem[kern];
        node.link = reversed;
        node.width = -st.shipout.cur_h;
        st.shipout.cur_h = save_h;
        st.mem[this_box].set_lr(BoxLr::Reversed);
        p = kern.ptr();
    }

    let mut left_edge = st.shipout.cur_h;
    synctex_hlist(st, env, this_box);

    while let Some(here) = p {
        if let Some(ch) = st.mem.as_char(p) {
            (p, prev_p) = chars_out(st, env, OutChar::of_char(&st.mem, ch))?;
            continue;
        }
        match st.mem.classify(here).or_confusion("hlistout")? {
            AnyNode::Box(b) => hlist_box_out(st, env, lr, b, base_line)?,
            AnyNode::Rule(rule) => {
                let dims = RuleDims::of(&st.mem, rule);
                hlist_rule_out(st, env, this_box, base_line, rule.idx(), dims)?;
            }
            AnyNode::Whatsit(whatsit) => match whatsit.variant(&st.mem) {
                Some(WhatsitVariant::NativeWord(word)) => {
                    let actual_text =
                        whatsit.whatsit_subtype(&st.mem) == Some(WhatsitSubtype::NativeWordAt);
                    native_word_out(st, env, word, actual_text)?
                }
                Some(WhatsitVariant::Glyph(glyph)) => glyph_out(st, env, glyph)?,
                Some(WhatsitVariant::Pic(pic)) => {
                    st.shipout.cur_v = base_line;
                    let edge = st.shipout.cur_h + st.mem[pic].width;
                    with_saved_dvi_pos(st, env, |st, env| pic_out(st, env, pic))?;
                    st.shipout.cur_h = edge;
                    st.shipout.cur_v = base_line;
                }
                Some(WhatsitVariant::SavePos(_)) => st.shipout.pdf_save_pos(),
                _ => out_what(st, env, whatsit)?,
            },
            AnyNode::Glue(glue) => {
                let frame = HlistFrame {
                    this_box,
                    base_line,
                    left_edge,
                };
                hlist_glue_out(st, env, lr, frame, glue, &mut rounding)?;
            }
            AnyNode::MarginKern(kern) => st.shipout.cur_h += st.mem[kern].width,
            AnyNode::Kern(kern) => {
                synctex_kern(st, env, kern, this_box);
                st.shipout.cur_h += st.mem[kern].width;
            }
            AnyNode::Math(math) => {
                synctex_math(st, env, math);
                let math_width = st.mem[math].width;
                let code = st.mem[math].code()?;
                if code.is_end() {
                    if st.mem.lr_matches(lr, code) {
                        st.mem.pop_lr(&mut lr.ptr);
                    } else if code.is_lr() {
                        lr.problems += 1;
                    }
                } else {
                    st.mem.push_lr(&mut lr.ptr, code.lr_end())?;
                    if code.dir() != lr.dir {
                        let edge = reverse_segment(
                            st,
                            lr,
                            math,
                            prev_p,
                            left_edge,
                            math_width,
                            &mut rounding,
                        )?;
                        p = edge.ptr();
                        continue;
                    }
                }
                math.into_kern(&mut st.mem);
                st.shipout.cur_h += math_width;
            }
            AnyNode::Ligature(ligature) => {
                st.fonts.xtx_ligature_present = true;
                let first = OutChar::of_ligature(&st.mem, ligature);
                (p, prev_p) = chars_out(st, env, first)?;
                continue;
            }
            AnyNode::Style(style) => {
                let edge = style.as_edge();
                st.shipout.cur_h += st.mem[edge].width;
                left_edge = st.shipout.cur_h + st.mem[edge].edge_dist;
                lr.dir = st.mem[edge].dir.get().or_confusion("LR")?;
            }
            _ => {}
        }
        prev_p = LinkSlot::Node(here);
        p = here.link(&st.mem);
    }

    synctex_tsilh(st, env, this_box);

    while st.mem.lr_code(lr.ptr) != Some(MathCode::Before) {
        if st.mem.lr_code(lr.ptr).is_some_and(MathCode::is_lr) {
            lr.problems += 10000;
        }
        st.mem.pop_lr(&mut lr.ptr);
    }
    st.mem.pop_lr(&mut lr.ptr);
    if st.mem[this_box].lr() == BoxLr::Dlist {
        lr.dir = Dir::Rtl;
    }
    st.mem.pop_dvi_level(env, &mut st.shipout, save_loc)
}

fn hlist_box_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    lr: &mut LrState,
    b: NodePtr<BoxNode>,
    base_line: Scaled,
) -> Result<(), Abort> {
    if st.mem[b].list_ptr.is_none() {
        if st.mem[b].is_vlist() {
            synctex_void_vlist(st, env, b);
        } else {
            synctex_void_hlist(st, env, b);
        }
        st.shipout.cur_h += st.mem[b].width;
        return Ok(());
    }
    st.shipout.cur_v = base_line + st.mem[b].shift_amount;
    let edge = st.shipout.cur_h + st.mem[b].width;
    if lr.dir == Dir::Rtl {
        st.shipout.cur_h = edge;
    }
    with_saved_dvi_pos(st, env, |st, env| ship_box(st, env, lr, b))?;
    st.shipout.cur_h = edge;
    st.shipout.cur_v = base_line;
    Ok(())
}

/// Reverse the hlist segment that starts after `math`, replacing `math` by an
/// edge node that the caller dispatches next.
fn reverse_segment(
    st: &mut EngineState,
    lr: &mut LrState,
    math: NodePtr<MathNode>,
    prev_p: LinkSlot,
    left_edge: Scaled,
    math_width: Scaled,
    rounding: &mut GlueRounding,
) -> Result<NodePtr<EdgeNode>, Abort> {
    let save_h = st.shipout.cur_h;
    st.mem.temp_ptr = st.mem[math].link;
    math.free(&mut st.mem);
    lr.dir = lr.dir.reversed();
    let edge = st.mem.new_edge(lr.dir, math_width)?;
    prev_p.set(&mut st.mem, edge.ptr());
    st.shipout.cur_h = st.shipout.cur_h - left_edge + math_width;
    let end = st.mem.new_edge(lr.dir.reversed(), Scaled::ZERO)?;
    let reversed = st
        .mem
        .reverse(&mut st.shipout, lr, &mut st.fonts, Some(end), rounding)?;
    let node = &mut st.mem[edge];
    node.link = reversed;
    node.edge_dist = st.shipout.cur_h;
    lr.dir = lr.dir.reversed();
    st.shipout.cur_h = save_h;
    Ok(edge)
}

fn vlist_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    lr: &mut LrState,
    this_box: NodePtr<BoxNode>,
) -> Result<(), Abort> {
    st.mem.temp_ptr = this_box.ptr();
    let mut rounding = GlueRounding::new(&st.mem, this_box);
    let mut p = st.mem[this_box].list_ptr;
    let upwards = st.mem[this_box].upwards();
    let save_loc = st.shipout.dvi.enter_level(env)?;
    let left_edge = st.shipout.cur_h;
    synctex_vlist(st, env, this_box);
    if upwards {
        st.shipout.cur_v += st.mem[this_box].depth;
    } else {
        st.shipout.cur_v -= st.mem[this_box].height;
    }
    let top_edge = st.shipout.cur_v;

    while let Some(here) = p {
        if st.mem.is_char_node(here) {
            return Err(Abort::Confusion("vlistout"));
        }
        match st.mem.classify(here).or_confusion("vlistout")? {
            AnyNode::Box(b) => vlist_box_out(st, env, lr, b, left_edge, upwards)?,
            AnyNode::Rule(rule) => {
                let dims = RuleDims::of(&st.mem, rule);
                vlist_rule_out(st, env, lr, this_box, left_edge, upwards, dims)?;
            }
            AnyNode::Whatsit(whatsit) => match whatsit.variant(&st.mem) {
                Some(WhatsitVariant::Glyph(glyph)) => {
                    let glyph = st.mem[glyph];
                    st.shipout.cur_v += glyph.height;
                    st.shipout.cur_h = left_edge;
                    synch_h(st, env)?;
                    synch_v(st, env)?;
                    dvi_font_select(st, env, glyph.font)?;
                    st.shipout.dvi.set_glyph(env, Scaled::ZERO, glyph.glyph)?;
                    st.shipout.cur_v += glyph.depth;
                    st.shipout.cur_h = left_edge;
                }
                Some(WhatsitVariant::Pic(pic)) => {
                    let (_, save_v) = with_saved_dvi_pos(st, env, |st, env| {
                        st.shipout.cur_v += st.mem[pic].height;
                        pic_out(st, env, pic)
                    })?;
                    st.shipout.cur_v = save_v + st.mem[pic].depth;
                    st.shipout.cur_h = left_edge;
                }
                Some(WhatsitVariant::SavePos(_)) => st.shipout.pdf_save_pos(),
                _ => out_what(st, env, whatsit)?,
            },
            AnyNode::Glue(glue) => {
                let spec = st.mem[glue].glue_ptr;
                let ht = rounding.width(&st.mem, spec);
                if let Some(kind) = st.mem[glue].leaders() {
                    match st.mem.leader_of(glue) {
                        Leader::Rule(rule) => {
                            let dims = RuleDims {
                                ht,
                                dp: Scaled::ZERO,
                                wd: st.mem[rule].width.to_scaled(),
                            };
                            vlist_rule_out(st, env, lr, this_box, left_edge, upwards, dims)?;
                        }
                        Leader::Box(leader) => {
                            let leaders = BoxLeaders {
                                leader,
                                kind,
                                size: ht,
                            };
                            vlist_leaders_out(st, env, lr, leaders, left_edge, top_edge, upwards)?
                        }
                    }
                } else {
                    st.shipout.move_past_vlist_rule(upwards, ht);
                }
            }
            AnyNode::Kern(kern) => {
                if upwards {
                    st.shipout.cur_v -= st.mem[kern].width;
                } else {
                    st.shipout.cur_v += st.mem[kern].width;
                }
            }
            _ => {}
        }
        p = here.link(&st.mem);
    }

    synctex_tsilv(st, env, this_box);
    st.mem.pop_dvi_level(env, &mut st.shipout, save_loc)
}

fn vlist_box_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    lr: &mut LrState,
    b: NodePtr<BoxNode>,
    left_edge: Scaled,
    upwards: bool,
) -> Result<(), Abort> {
    let (height, depth) = (st.mem[b].height, st.mem[b].depth);
    if upwards {
        st.shipout.cur_v -= depth;
    } else {
        st.shipout.cur_v += height;
    }
    if st.mem[b].list_ptr.is_none() {
        if st.mem[b].is_vlist() {
            synctex_void_vlist(st, env, b);
        } else {
            synctex_void_hlist(st, env, b);
        }
        if upwards {
            st.shipout.cur_v -= height;
        } else {
            st.shipout.cur_v += depth;
        }
        return Ok(());
    }
    synch_v(st, env)?;
    let shift = st.mem[b].shift_amount;
    st.shipout.cur_h = if lr.dir == Dir::Rtl {
        left_edge - shift
    } else {
        left_edge + shift
    };
    let (_, save_v) = with_saved_dvi_pos(st, env, |st, env| ship_box(st, env, lr, b))?;
    st.shipout.cur_v = if upwards {
        save_v - st.mem[b].height
    } else {
        save_v + st.mem[b].depth
    };
    st.shipout.cur_h = left_edge;
    Ok(())
}

fn with_saved_dvi_pos(
    st: &mut EngineState,
    env: &mut EngineEnv,
    out: impl FnOnce(&mut EngineState, &mut EngineEnv) -> Result<(), Abort>,
) -> Result<(Scaled, Scaled), Abort> {
    let (h, v) = (st.shipout.dvi_h, st.shipout.dvi_v);
    out(st, env)?;
    st.shipout.dvi_h = h;
    st.shipout.dvi_v = v;
    Ok((h, v))
}

fn vlist_leaders_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    lr: &mut LrState,
    leaders: BoxLeaders,
    left_edge: Scaled,
    top_edge: Scaled,
    upwards: bool,
) -> Result<(), Abort> {
    let BoxLeaders { leader, kind, size } = leaders;
    let leader_ht = st.mem[leader].height + st.mem[leader].depth;
    if leader_ht <= Scaled::ZERO || size <= Scaled::ZERO {
        st.shipout.move_past_vlist_rule(upwards, size);
        return Ok(());
    }
    let size = size + LEADER_SLOP;
    let edge = st.shipout.cur_v + size;
    let mut lx = Scaled::ZERO;
    if kind == LeaderKind::Aligned {
        let save_v = st.shipout.cur_v;
        st.shipout.cur_v =
            top_edge + leader_ht * ((st.shipout.cur_v - top_edge).raw() / leader_ht.raw());
        if st.shipout.cur_v < save_v {
            st.shipout.cur_v += leader_ht;
        }
    } else {
        let (lq, rest) = leader_count_and_rest(size, leader_ht);
        if kind == LeaderKind::Centered {
            st.shipout.cur_v += rest / 2;
        } else {
            lx = rest / (lq + 1);
            st.shipout.cur_v += (rest - lx * (lq - 1)) / 2;
        }
    }

    while st.shipout.cur_v + leader_ht <= edge {
        let shift = st.mem[leader].shift_amount;
        st.shipout.cur_h = if lr.dir == Dir::Rtl {
            left_edge - shift
        } else {
            left_edge + shift
        };
        synch_h(st, env)?;
        st.shipout.cur_v += st.mem[leader].height;
        synch_v(st, env)?;
        let (_, save_v) =
            with_saved_dvi_pos(st, env, |st, env| ship_leader_box(st, env, lr, leader))?;
        st.shipout.cur_h = left_edge;
        st.shipout.cur_v = save_v - st.mem[leader].height + leader_ht + lx;
    }
    st.shipout.cur_v = edge - LEADER_SLOP;
    Ok(())
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct Movement {
    /// Whether the movement may be reused, and as which command.
    pub state: i32,
    pub distance: Scaled,
    pub location: i32,
}

impl ShipoutState {
    fn movement(&mut self, env: &mut EngineEnv, w: Scaled, axis: Axis) -> Result<(), Abort> {
        let location = self.dvi.pos();
        let moves = match axis {
            Axis::Down => &mut self.down_moves,
            Axis::Right => &mut self.right_moves,
        };
        let top = moves.len();
        moves.push(Movement {
            state: MOV_NONE_SEEN,
            distance: w,
            location,
        });

        let mut mstate = MOV_NONE_SEEN;
        let found = 'search: {
            for i in (0..top).rev() {
                let node = moves[i];
                if node.distance != w {
                    match mstate + node.state {
                        1 => mstate = MOV_Y_SEEN,
                        2 => mstate = MOV_Z_SEEN,
                        8 | 13 => break 'search None,
                        _ => {}
                    }
                    continue;
                }
                let (reg, here) = match mstate + node.state {
                    // NONE_SEEN or Z_SEEN meeting YZ_OK or Y_OK
                    3 | 4 | 15 | 16 => (MoveReg::Y, MOV_Y_HERE),
                    // NONE_SEEN meeting Z_OK, Y_SEEN meeting YZ_OK or Z_OK
                    5 | 9 | 11 => (MoveReg::Z, MOV_Z_HERE),
                    // a y or z that is already here
                    1 | 2 | 8 | 13 => break 'search Some(i),
                    _ => continue,
                };
                if !self.dvi.set_reg_at(node.location, reg) {
                    break 'search None;
                }
                moves[i].state = here;
                break 'search Some(i);
            }
            None
        };

        let Some(found) = found else {
            moves[top].state = MOV_YZ_OK;
            return self.dvi.movement(env, axis, w);
        };

        moves[top].state = moves[found].state;
        let (reg, yz_becomes, fixed_from) = if moves[top].state == MOV_Y_HERE {
            (MoveReg::Y, MOV_Z_OK, MOV_Y_OK)
        } else {
            (MoveReg::Z, MOV_Y_OK, MOV_Z_OK)
        };
        for m in &mut moves[found + 1..top] {
            if m.state == MOV_YZ_OK {
                m.state = yz_becomes;
            } else if m.state == fixed_from {
                m.state = MOV_D_FIXED;
            }
        }
        self.dvi.movement_by_reg(env, axis, reg)
    }

    fn prune_movements(&mut self, l: i32) {
        for moves in [&mut self.down_moves, &mut self.right_moves] {
            while moves.last().is_some_and(|m| m.location >= l) {
                moves.pop();
            }
        }
    }
}

pub(crate) fn finalize_dvi_file(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    if st
        .shipout
        .dvi
        .close_open_page(env, st.scanner.logical_clock)?
    {
        st.shipout.total_pages += 1;
    }

    if st.shipout.total_pages == 0 {
        print_nl(st, env, "");
        begin_warning(st, env);
        print(st, env, "No pages of output");
        end_capture(st, env);
        print_char(st, env, b'.');
        return Ok(());
    }
    // The DVI grew too big and the message was already printed.
    if st.shipout.dvi.depth == -2 {
        return Ok(());
    }

    st.shipout.dvi.begin_postamble(env)?;
    prepare_mag(st, env)?;
    let mag = st.eqtb.int_par(IntPar::Mag);
    let s = &mut st.shipout;
    s.dvi
        .end_postamble(env, mag, s.max_v, s.max_h, s.total_pages)?;

    for f in st.fonts.font_ptr.loaded().rev() {
        if st.fonts.font_used[f] {
            dvi_font_def(st, env, f)?;
        }
    }
    st.fonts.font_ptr = NULL_FONT;

    st.shipout
        .dvi
        .finish(env, id_byte(st.config.semantic_pagination_enabled))?;

    let k = u8::from(output_close(env, st.shipout.dvi.file));
    if k == 0 {
        print_nl(st, env, "Output written on ");
        print(st, env, st.shipout.output_file_name);
        print(st, env, " (");
        print_int(st, env, st.shipout.total_pages);
        if st.shipout.total_pages != 1 {
            print(st, env, " pages");
        } else {
            print(st, env, " page");
        }
        print(st, env, ", ");
        print_int(st, env, st.shipout.dvi.pos());
        print(st, env, " bytes).");
    } else {
        print_nl(st, env, "Error ");
        print_int(st, env, i32::from(k));
        print(st, env, " (");
        print(st, env, &strerror(i32::from(k))[..]);
        print(st, env, ") generating output;");
        print_nl(st, env, "file ");
        print(st, env, st.shipout.output_file_name);
        print(st, env, " may not be valid.");
        // XeTeX also sets history to OUTPUT_FAILURE here.
    }
    Ok(())
}

fn glyph_array(mem: &MemState, word: NodePtr<NativeWordNode>) -> (Scaled, &[FixedPoint], &[u16]) {
    let count = mem[word].glyph_count as usize;
    let glyphs = mem.glyph_blobs.get(mem[word].glyph_info);
    (
        mem[word].width,
        &glyphs.locations[..count],
        &glyphs.gids[..count],
    )
}

impl MemState {
    /// Reverse the hlist starting at `temp_ptr` onto `t` (the segment's closing
    /// edge, or `None` for a whole list), returning the new head.
    fn reverse(
        &mut self,
        shipout: &mut ShipoutState,
        lr: &mut LrState,
        fonts: &mut FontState,
        t: Option<NodePtr<EdgeNode>>,
        rounding: &mut GlueRounding,
    ) -> Result<MemPtr, Abort> {
        let mut l = t.map(NodePtr::idx);
        let mut p = self.temp_ptr;
        let mut m = MIN_HALFWORD;
        let mut n = MIN_HALFWORD;

        loop {
            while let Some(here) = p {
                if self.is_char_node(here) {
                    while let Some(ch) = self.as_char(p) {
                        let f = self[ch].font;
                        let c = fonts.effective_char(f, self[ch].character);
                        shipout.cur_h += fonts.font_character_width(f, c);
                        let q = ch.idx().link(self);
                        ch.idx().set_link(self, l);
                        l = p;
                        p = q;
                    }
                    continue;
                }

                let q = here.link(self);
                let mut add_width = true;
                let mut now_kern = false;
                let mut wd = Scaled::ZERO;
                match self.classify(here) {
                    Ok(node @ (AnyNode::Box(_) | AnyNode::Rule(_))) => {
                        wd = node.hlist_width(self);
                    }
                    Ok(AnyNode::Kern(kern)) => {
                        wd = self[kern].width;
                        now_kern = true;
                    }
                    Ok(AnyNode::Whatsit(w)) => {
                        if let Some(dims) = w.dims(self) {
                            wd = dims.width;
                        } else {
                            add_width = false;
                        }
                    }
                    Ok(AnyNode::Glue(glue)) => {
                        let spec = self[glue].glue_ptr;
                        wd = rounding.width(self, spec);
                        if rounding.is_set(self, spec) {
                            self.freeze_set_glue(glue, spec, wd)?;
                            now_kern = self[glue].leaders().is_none();
                        }
                    }
                    Ok(AnyNode::Ligature(ligature)) => {
                        self.flush_records::<CharNode>(self[ligature].lig_ptr);
                        self.temp_ptr = ligature.ptr();
                        let (f, ch) = (self[ligature].font, self[ligature].character);
                        let c = self.new_char(f, ch)?;
                        c.idx().set_link(self, q);
                        ligature.free(self);
                        p = c.ptr();
                        continue;
                    }
                    Ok(AnyNode::Math(math)) => {
                        wd = self[math].width;
                        let code = self[math].code()?;
                        if code.is_end() {
                            if !self.lr_matches(lr, code) {
                                math.into_kern(self);
                                now_kern = true;
                                lr.problems += 1;
                            } else {
                                self.pop_lr(&mut lr.ptr);
                                if n > MIN_HALFWORD {
                                    n -= 1;
                                    self[math].subtype = code.as_begin().into();
                                } else {
                                    math.into_kern(self);
                                    now_kern = true;
                                    if m > MIN_HALFWORD {
                                        m -= 1;
                                    } else {
                                        math.free(self);
                                        let t = t.expect("a segment reversal ends at its edge");
                                        let node = &mut self[t];
                                        node.link = q;
                                        node.width = wd;
                                        node.edge_dist = -shipout.cur_h - wd;
                                        return Ok(l);
                                    }
                                }
                            }
                        } else {
                            self.push_lr(&mut lr.ptr, code.lr_end())?;
                            if n > MIN_HALFWORD || code.dir() != lr.dir {
                                n += 1;
                                self[math].subtype = code.as_end().into();
                            } else {
                                math.into_kern(self);
                                now_kern = true;
                                m += 1;
                            }
                        }
                    }
                    Ok(AnyNode::Style(_)) => return Err(Abort::Confusion("LR2")),
                    _ => add_width = false,
                }

                if add_width {
                    shipout.cur_h += wd;
                }
                here.set_link(self, l);
                if now_kern && (wd == Scaled::ZERO || l.is_none()) {
                    self.free_node(here, KernNode::SIZE);
                } else {
                    l = p;
                }
                p = q;
            }

            if t.is_none() && m == MIN_HALFWORD && n == MIN_HALFWORD {
                return Ok(l);
            }
            let code = self.lr_code(lr.ptr).or_confusion("LR")?;
            p = self.new_math(Scaled::ZERO, code)?.ptr();
            lr.problems += 10000;
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct ShipoutState {
    /// `D:YYYYmmddHHMMSS+HH'MM'`.
    pub start_time_str: Vec<u8>,
    pub dvi: DviWriter,
    pub insert_src_special_every_par: bool,
    pub insert_src_special_every_math: bool,
    pub insert_src_special_every_vbox: bool,
    pub insert_src_special_auto: bool,
    pub selector: Selector,
    pub doing_special: bool,
    pub term_offset: i32,
    pub file_offset: i32,
    pub old_setting: Selector,
    pub epochseconds: i32,
    pub microseconds: i32,
    pub dead_cycles: i32,
    pub page_tail: MemPtr,
    pub last_badness: i32,
    pub page_contents: PageContents,
    pub page_so_far: [Scaled; 8],
    pub last_glue: Option<GlueSpecId>,
    pub last_penalty: i32,
    pub last_kern: Scaled,
    pub last_node_type: i32,
    pub insert_penalties: i32,
    pub output_active: bool,
    /// Set only while the finished `\output` level is popped.
    pub output_can_end: bool,
    pub pdf_last_x_pos: i32,
    pub pdf_last_y_pos: i32,
    pub write_file: [usize; 16],
    pub write_open: [bool; 18],
    pub write_name: [Option<Vec<u8>>; 16],
    /// Code units emitted before the current replay fence.
    pub write_prefix: [Vec<i32>; 16],
    pub rust_stdout: usize,
    pub log_file: usize,
    /// Transcript bytes written so far, captured with every checkpoint.
    pub log_bytes: u64,
    /// Where this operation's transcript continues the previous one, if it
    /// resumed. Operation-scoped, so checkpoint restores keep it.
    pub log_resumed_at: Option<u64>,
    /// Diagnostics reported so far, captured with every checkpoint.
    pub diagnostics_issued: usize,
    pub best_page_break: MemPtr,
    pub best_size: Scaled,
    pub least_page_cost: i32,
    pub page_max_depth: Scaled,
    pub total_pages: i32,
    pub max_v: Scaled,
    pub max_h: Scaled,
    pub doing_leaders: bool,
    pub cur_h: Scaled,
    pub cur_v: Scaled,
    pub cur_page_width: Scaled,
    pub cur_page_height: Scaled,
    pub cur_h_offset: Scaled,
    pub cur_v_offset: Scaled,
    pub write_loc: Option<CsId>,
    pub output_file_name: StrNumber,
    /// The vertical and horizontal DVI movements of the page so far, TeX's `down_ptr` and
    /// `right_ptr` lists, the latest last.
    pub down_moves: Vec<Movement>,
    pub right_moves: Vec<Movement>,
    pub dvi_h: Scaled,
    pub dvi_v: Scaled,
    pub dvi_f: FontId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::test_env;

    #[test]
    fn a_repeated_movement_reuses_a_register() {
        let env = &mut test_env();
        let mut s = ShipoutState::default();
        s.dvi.init();
        for w in [5, 7, 5] {
            s.movement(env, Scaled::from_raw(w), Axis::Down).unwrap();
        }
        // down1 5 became y1 5, down1 7 stays, the repeat is y0.
        assert_eq!(&s.dvi.digest_view().1[..5], [162, 5, 157, 7, 161]);
        let states: Vec<i32> = s.down_moves.iter().map(|m| m.state).collect();
        assert_eq!(states, [MOV_Y_HERE, MOV_Z_OK, MOV_Y_HERE]);
        s.prune_movements(2);
        assert_eq!(s.down_moves.len(), 1);
    }
}
