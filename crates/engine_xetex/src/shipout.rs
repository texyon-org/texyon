//! Shipping boxes out to the XDV file.

use crate::bridge::{output_close, output_flush, output_open};
use crate::consts::{
    INVALID_HANDLE, MAX_DIMEN, MEDIUM_NODE_SIZE, MIN_HALFWORD, NULL_FONT, SMALL_NODE_SIZE,
};
use crate::dvi::{Axis, MoveReg};
use crate::errors::Abort;
use crate::ext::apply_tfm_font_mapping;
use crate::glyph_arena::FixedPoint;
use crate::mem::LIG_TRICK;
use crate::node::Stored;
use crate::node::{
    link_of, AnyNode, BoxLr, BoxNode, EdgeNode, GlueKind, GlueNode, GlueSpecNode, GlyphNode,
    KernSubtype, LeaderKind, MathCode, MathNode, MovementNode, NativeWordNode, NodePtr, RuleNode,
    WhatsitSubtype, WhatsitVariant, RUNNING,
};
use crate::params::{DimenPar, IntPar};
use crate::state::{
    EqtbState, FontState, MemState, ScannerState, Selector, ShipoutState, StringsState,
};
use crate::units::{Dir, FontId, GlueOrder, GlueSign, MemPtr, Scaled, StrNumber};

mod whatsits;

use self::whatsits::pic_out;
pub(crate) use self::whatsits::{ensure_write_stream_open, out_what};

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

/// How many leader boxes of size `size` fit in `total`, and the leftover space.
fn leader_count_and_rest(total: Scaled, size: Scaled) -> (i32, Scaled) {
    (
        total.raw() / size.raw(),
        Scaled::from_raw(total.raw() % size.raw()),
    )
}

pub(crate) const ONE_INCH: Scaled = Scaled::from_raw(4_736_287);
const ONE_INCH_LESS_ONE: Scaled = Scaled::from_raw(ONE_INCH.raw() - 1);

impl MemState {
    pub fn new_edge(&mut self, dir: Dir, w: Scaled) -> Result<NodePtr<EdgeNode>, Abort> {
        let p = NodePtr::<EdgeNode>::alloc(self)?;
        self[p].dir = dir.into();
        self[p].width = w;
        self[p].edge_dist = Scaled::ZERO;
        Ok(p)
    }
}

fn synch_h(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    if st.shipout.cur_h != st.shipout.dvi_h {
        let delta = st.shipout.cur_h - st.shipout.dvi_h;
        st.mem.movement(&mut st.shipout, delta, Axis::Right)?;
        st.shipout.dvi_h = st.shipout.cur_h;
    }
    Ok(())
}

fn synch_v(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    if st.shipout.cur_v != st.shipout.dvi_v {
        let delta = st.shipout.cur_v - st.shipout.dvi_v;
        st.mem.movement(&mut st.shipout, delta, Axis::Down)?;
        st.shipout.dvi_v = st.shipout.cur_v;
    }
    Ok(())
}

fn dvi_font_def(st: &mut crate::state::EngineState, f: FontId) -> Result<(), Abort> {
    let k = f.dvi_number();
    if st.fonts.is_native_font(f) {
        let def = st.fonts.make_font_def(f)?;
        return st.shipout.dvi.native_font_def(k, &def);
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
    st.shipout.dvi.fnt_def(
        k,
        st.fonts.check_bytes(f),
        st.fonts.font_size[f],
        st.fonts.font_dsize[f],
        area,
        &name[..name_len],
    )
}

pub fn initialize_shipout_variables(st: &mut crate::state::EngineState) {
    let s = &mut st.shipout;
    s.output_file_name = StrNumber::NONE;
    s.dvi.init();
    s.down_ptr = None;
    s.right_ptr = None;
}

/// Emit the current string (the tail of the pool) as a DVI special and drop it.
fn dvi_special_from_cur_string(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    st.shipout.dvi.special(st.strings.cur_string())?;
    st.strings.discard_cur_string();
    Ok(())
}

pub(crate) fn ship_out(
    st: &mut crate::state::EngineState,
    page: NodePtr<BoxNode>,
) -> Result<(), Abort> {
    let p = page.ptr();
    let mag = st.eqtb.int_par(IntPar::Mag);
    let job_name = st.strings.texstring_bytes(st.strings.job_name);
    crate::synctex::synctex_sheet(st, mag, &job_name);

    if st.strings.job_name == StrNumber::NONE {
        crate::io::open_log_file(st)?;
    }
    let tracing_output = st.eqtb.int_par(IntPar::TracingOutput) > 0;
    if tracing_output {
        crate::output::print_nl(st, "");
        crate::output::print_ln(st);
        crate::output::print(st, "Completed box being shipped out");
    }
    if st.shipout.term_offset > st.config.max_print_line - 9 {
        crate::output::print_ln(st);
    } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
        crate::output::print_char(st, b' ');
    }

    crate::output::print_char(st, b'[');
    let mut j = 9;
    while j > 0 && st.eqtb.count(j) == 0 {
        j -= 1;
    }
    for k in 0..=j {
        crate::output::print_int(st, st.eqtb.count(k));
        if k < j {
            crate::output::print_char(st, b'.');
        }
    }
    output_flush(st.shipout.rust_stdout);

    if tracing_output {
        crate::output::print_char(st, b']');
        crate::output::begin_diagnostic(st);
        crate::output::show_box(st, p);
        crate::output::end_diagnostic(st, true);
    }

    let (height, depth, width) = (st.mem[page].height, st.mem[page].depth, st.mem[page].width);
    let h_offset = st.eqtb.dimen_par(DimenPar::HOffset);
    let v_offset = st.eqtb.dimen_par(DimenPar::VOffset);
    if height > MAX_DIMEN
        || depth > MAX_DIMEN
        || height + depth + v_offset > MAX_DIMEN
        || width + h_offset > MAX_DIMEN
    {
        crate::output::error_here_with_diagnostic(st, "Huge page cannot be shipped out");
        crate::output::capture_to_diagnostic(st, None);
        st.errors.help(&[
            "The page just created is more than 18 feet tall or",
            "more than 18 feet wide, so I suspect something went wrong.",
        ]);
        crate::errors::error(st)?;

        if st.eqtb.int_par(IntPar::TracingOutput) <= 0 {
            crate::output::begin_diagnostic(st);
            crate::output::print_nl(st, "The following box has been deleted:");
            crate::output::show_box(st, p);
            crate::output::end_diagnostic(st, true);
        }
    } else {
        ship_page(st, page)?;
    }

    if st.scanner.lr_problems > 0 {
        crate::output::print_ln(st);
        crate::output::print_nl(st, "\\endL or \\endR problem (");
        crate::output::print_int(st, st.scanner.lr_problems / 10000);
        crate::output::print(st, " missing, ");
        crate::output::print_int(st, st.scanner.lr_problems % 10000);
        crate::output::print(st, " extra");
        st.scanner.lr_problems = 0;
        crate::output::print_char(st, b')');
        crate::output::print_ln(st);
    }

    if !st.scanner.lr_ptr.is_null() || st.scanner.cur_dir != Dir::Ltr {
        return Err(Abort::Confusion("LR3"));
    }
    if st.eqtb.int_par(IntPar::TracingOutput) <= 0 {
        crate::output::print_char(st, b']');
    }
    st.shipout.dead_cycles = 0;
    output_flush(st.shipout.rust_stdout);
    st.mem.flush_node_list(p)?;
    crate::synctex::synctex_teehs(st);
    Ok(())
}

fn ship_page(st: &mut crate::state::EngineState, page: NodePtr<BoxNode>) -> Result<(), Abort> {
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
            crate::io::open_log_file(st)?;
        }
        crate::io::pack_job_name(st, crate::engine::output_file_extension(st))?;
        st.shipout.dvi.file = output_open(st.input.name_of_file_bytes(), false);
        if st.shipout.dvi.file == INVALID_HANDLE {
            return Err(Abort::CannotOpenOutput);
        }
        st.shipout.output_file_name = crate::io::make_name_string(st)?;
    }

    if st.shipout.total_pages == 0 {
        let id = id_byte(st.config.semantic_pagination_enabled);
        st.shipout.dvi.begin_preamble(id)?;
        crate::save::prepare_mag(st)?;
        let mag = st.eqtb.int_par(IntPar::Mag);
        st.shipout.dvi.end_preamble(mag, b"tectonic")?;
    }

    let counts = std::array::from_fn(|k| st.eqtb.count(k as i32));
    st.shipout.dvi.bop(counts)?;

    let old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    crate::output::print(st, "pdf:pagesize ");
    if pdf_page_width <= Scaled::ZERO || pdf_page_height <= Scaled::ZERO {
        crate::output::print(st, "default");
    } else {
        crate::output::print(st, "width");
        crate::output::print(st, b' ');
        crate::output::print_scaled(st, pdf_page_width);
        crate::output::print(st, "pt");
        crate::output::print(st, b' ');
        crate::output::print(st, "height");
        crate::output::print(st, b' ');
        crate::output::print_scaled(st, pdf_page_height);
        crate::output::print(st, "pt");
    }
    st.shipout.selector = old_setting;

    dvi_special_from_cur_string(st)?;

    st.shipout.cur_v = height + v_offset;
    if st.mem[page].is_vlist() {
        vlist_out(st, page)?;
    } else {
        hlist_out(st, page)?;
    }

    st.shipout.dvi.eop(st.scanner.logical_clock)?;
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

fn dvi_font_select(st: &mut crate::state::EngineState, f: FontId) -> Result<(), Abort> {
    if f == st.shipout.dvi_f {
        return Ok(());
    }
    if !st.fonts.font_used[f] {
        dvi_font_def(st, f)?;
        st.fonts.font_used[f] = true;
    }
    st.shipout.dvi.fnt(f.dvi_number())?;
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
            set: mem[this_box].glue_set,
            cur_g: Scaled::ZERO,
            cur_glue: 0.0,
        }
    }

    /// Whether the box stretches or shrinks `spec`'s component.
    fn is_set(&self, mem: &MemState, spec: NodePtr<GlueSpecNode>) -> bool {
        match self.sign.get() {
            Some(GlueSign::Stretching) => mem[spec].stretch_order == self.order,
            Some(GlueSign::Shrinking) => mem[spec].shrink_order == self.order,
            Some(GlueSign::Normal) | None => false,
        }
    }

    /// The rounded width of `spec` as set in this box.
    fn width(&mut self, mem: &MemState, spec: NodePtr<GlueSpecNode>) -> Scaled {
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
        let info = self.char_info(f, c);
        self.char_width(f, info)
    }
}

/// Native words of one font joined by interword glue, and their text length
/// with a space for each glue.
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
        let mut prev_p = this_box.list_head();
        let mut p = self[this_box].list_ptr;

        while !p.is_null() {
            let first = self.as_native_word(p).filter(|&r| {
                fonts.font_letter_space[self[r].font] == Scaled::ZERO && !p.link(self).is_null()
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
                                q = self.skip_interword_fillers(q.link(self));
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
                    p = self.merge_native_word_run(strings, fonts, eqtb, this_box, prev_p, run)?;
                }
            }
            prev_p = p;
            p = p.link(self);
        }
        Ok(())
    }

    /// Replace the run `first..=last` by one native word, moving the ignorable
    /// nodes inside the run after it. Returns the last node of the replacement.
    fn merge_native_word_run(
        &mut self,
        strings: &mut StringsState,
        fonts: &FontState,
        eqtb: &EqtbState,
        this_box: NodePtr<BoxNode>,
        prev_p: MemPtr,
        run: NativeWordRun,
    ) -> Result<MemPtr, Abort> {
        let NativeWordRun {
            first,
            last,
            text_len,
        } = run;
        if strings.pool_ptr + text_len > strings.pool_size {
            return Err(Abort::PoolOverflow);
        }

        let g_order = self[this_box].glue_order;
        let g_sign = self[this_box].glue_sign;
        let mut width = Scaled::ZERO;
        let mut q = first.ptr();
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
                                self[this_box].glue_set * f64::from(self[spec].stretch.raw()),
                            );
                        }
                        Some(GlueSign::Shrinking) if self[spec].shrink_order == g_order => {
                            width -= Scaled::round(
                                self[this_box].glue_set * f64::from(self[spec].shrink.raw()),
                            );
                        }
                        _ => {}
                    }
                }
                Ok(AnyNode::Kern(kern)) => width += self[kern].width,
                _ => {}
            }
            if q == last.ptr() {
                break;
            }
            q = q.link(self);
        }

        let nlen = strings.cur_length();
        let word = self.new_native_word_node(eqtb, self[first].font, nlen)?;
        self[word].subtype = self[first].subtype;
        let base = strings.cur_string_start().index();
        self.entries_mut(word)
            .copy_from_slice(&strings.str_pool[base..base + nlen as usize]);
        self[word].width = width;
        fonts.store_justified_native_glyphs(self, word)?;

        prev_p.set_link(self, word.ptr());
        self[word].link = self[last].link;
        self[last].link = MemPtr::NULL;

        let mut tail = word.ptr();
        let mut prev_p = first.ptr();
        let mut p = self[first].link;
        while !p.is_null() {
            if self
                .classify_non_char(p)
                .is_some_and(|n| n.is_interword_filler(self))
            {
                prev_p.set_link(self, p.link(self));
                p.set_link(self, tail.link(self));
                tail.set_link(self, p);
                tail = p;
            }
            prev_p = p;
            p = p.link(self);
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
        spec: NodePtr<GlueSpecNode>,
        w: Scaled,
    ) -> Result<(), Abort> {
        self.delete_glue_ref(spec);
        if self[glue].leaders().is_none() {
            glue.into_kern(self, w);
        } else {
            let rigid = NodePtr::<GlueSpecNode>::alloc(self)?;
            self[rigid].stretch_order = GlueOrder::Rigid.into();
            self[rigid].shrink_order = GlueOrder::Rigid.into();
            self[rigid].width = w;
            self[rigid].stretch = Scaled::ZERO;
            self[rigid].shrink = Scaled::ZERO;
            self[glue].glue_ptr = rigid;
        }
        Ok(())
    }

    fn leader_of(&self, glue: NodePtr<GlueNode>) -> Leader {
        match self.classify(self[glue].leader_ptr) {
            Ok(AnyNode::Rule(rule)) => Leader::Rule(rule),
            Ok(AnyNode::Box(b)) => Leader::Box(b),
            other => unreachable!("leaders must be a rule or a box, found {other:?}"),
        }
    }

    /// The LR stack top matches the end code of `code`.
    fn lr_matches(&self, scanner: &ScannerState, code: MathCode) -> bool {
        self.lr_code(scanner.lr_ptr) == Some(code.lr_end())
    }

    fn pop_dvi_level(&mut self, shipout: &mut ShipoutState, save_loc: i32) -> Result<(), Abort> {
        self.prune_movements(shipout, save_loc);
        shipout.dvi.leave_level(save_loc)
    }
}

fn ship_box(st: &mut crate::state::EngineState, b: NodePtr<BoxNode>) -> Result<(), Abort> {
    if st.mem[b].is_vlist() {
        vlist_out(st, b)
    } else {
        hlist_out(st, b)
    }
}

/// Ship one copy of a leader box with `doing_leaders` set.
fn ship_leader_box(
    st: &mut crate::state::EngineState,
    leader: NodePtr<BoxNode>,
) -> Result<(), Abort> {
    let outer_doing_leaders = st.shipout.doing_leaders;
    st.shipout.doing_leaders = true;
    ship_box(st, leader)?;
    st.shipout.doing_leaders = outer_doing_leaders;
    Ok(())
}

fn hlist_rule_out(
    st: &mut crate::state::EngineState,
    this_box: NodePtr<BoxNode>,
    base_line: Scaled,
    p: MemPtr,
) -> Result<(), Abort> {
    if st.shipout.rule_ht == RUNNING {
        st.shipout.rule_ht = st.mem[this_box].height;
    }
    if st.shipout.rule_dp == RUNNING {
        st.shipout.rule_dp = st.mem[this_box].depth;
    }
    st.shipout.rule_ht += st.shipout.rule_dp;
    if st.shipout.rule_ht > Scaled::ZERO && st.shipout.rule_wd > Scaled::ZERO {
        synch_h(st)?;
        st.shipout.cur_v = base_line + st.shipout.rule_dp;
        synch_v(st)?;
        st.shipout
            .dvi
            .set_rule(st.shipout.rule_ht, st.shipout.rule_wd)?;
        st.shipout.cur_v = base_line;
        st.shipout.dvi_h += st.shipout.rule_wd;
    }
    st.shipout.cur_h += st.shipout.rule_wd;
    crate::synctex::synctex_horizontal_rule_or_glue(st, p);
    Ok(())
}

fn vlist_rule_out(
    st: &mut crate::state::EngineState,
    this_box: NodePtr<BoxNode>,
    left_edge: Scaled,
    upwards: bool,
) -> Result<(), Abort> {
    if st.shipout.rule_wd == RUNNING {
        st.shipout.rule_wd = st.mem[this_box].width;
    }
    st.shipout.rule_ht += st.shipout.rule_dp;
    st.shipout.move_past_vlist_rule(upwards);
    if st.shipout.rule_ht > Scaled::ZERO && st.shipout.rule_wd > Scaled::ZERO {
        if st.scanner.cur_dir == Dir::Rtl {
            st.shipout.cur_h -= st.shipout.rule_wd;
        }
        synch_h(st)?;
        synch_v(st)?;
        st.shipout
            .dvi
            .put_rule(st.shipout.rule_ht, st.shipout.rule_wd)?;
        st.shipout.cur_h = left_edge;
    }
    Ok(())
}

impl ShipoutState {
    fn move_past_vlist_rule(&mut self, upwards: bool) {
        if upwards {
            self.cur_v -= self.rule_ht;
        } else {
            self.cur_v += self.rule_ht;
        }
    }
}

/// Move right past glue `glue` or output its leaders.
fn hlist_glue_out(
    st: &mut crate::state::EngineState,
    this_box: NodePtr<BoxNode>,
    base_line: Scaled,
    left_edge: Scaled,
    glue: NodePtr<GlueNode>,
    rounding: &mut GlueRounding,
) -> Result<(), Abort> {
    let spec = st.mem[glue].glue_ptr;
    st.shipout.rule_wd = rounding.width(&st.mem, spec);
    if rounding.is_set(&st.mem, spec) {
        st.mem.freeze_set_glue(glue, spec, st.shipout.rule_wd)?;
    }

    if let Some(kind) = st.mem[glue].leaders() {
        let leader = match st.mem.leader_of(glue) {
            Leader::Rule(rule) => {
                st.shipout.rule_ht = st.mem[rule].height.to_scaled();
                st.shipout.rule_dp = st.mem[rule].depth.to_scaled();
                return hlist_rule_out(st, this_box, base_line, glue.ptr());
            }
            Leader::Box(leader) => leader,
        };
        let leader_wd = st.mem[leader].width;
        if leader_wd > Scaled::ZERO && st.shipout.rule_wd > Scaled::ZERO {
            st.shipout.rule_wd += LEADER_SLOP;
            if st.scanner.cur_dir == Dir::Rtl {
                st.shipout.cur_h -= LEADER_SLOP;
            }
            let edge = st.shipout.cur_h + st.shipout.rule_wd;
            let mut lx = Scaled::ZERO;
            if kind == LeaderKind::Aligned {
                let save_h = st.shipout.cur_h;
                st.shipout.cur_h = left_edge
                    + leader_wd * ((st.shipout.cur_h - left_edge).raw() / leader_wd.raw());
                if st.shipout.cur_h < save_h {
                    st.shipout.cur_h += leader_wd;
                }
            } else {
                let (lq, lr) = leader_count_and_rest(st.shipout.rule_wd, leader_wd);
                if kind == LeaderKind::Centered {
                    st.shipout.cur_h += lr / 2;
                } else {
                    lx = lr / (lq + 1);
                    st.shipout.cur_h += (lr - lx * (lq - 1)) / 2;
                }
            }

            while st.shipout.cur_h + leader_wd <= edge {
                st.shipout.cur_v = base_line + st.mem[leader].shift_amount;
                synch_v(st)?;
                synch_h(st)?;
                let (save_h, _) = with_saved_dvi_pos(st, |st| {
                    if st.scanner.cur_dir == Dir::Rtl {
                        st.shipout.cur_h += leader_wd;
                    }
                    ship_leader_box(st, leader)
                })?;
                st.shipout.cur_v = base_line;
                st.shipout.cur_h = save_h + leader_wd + lx;
            }

            st.shipout.cur_h = if st.scanner.cur_dir == Dir::Rtl {
                edge
            } else {
                edge - LEADER_SLOP
            };
            return Ok(());
        }
    }

    st.shipout.cur_h += st.shipout.rule_wd;
    crate::synctex::synctex_horizontal_rule_or_glue(st, glue.ptr());
    Ok(())
}

fn glyph_out(st: &mut crate::state::EngineState, glyph: NodePtr<GlyphNode>) -> Result<(), Abort> {
    synch_h(st)?;
    synch_v(st)?;
    let glyph = st.mem[glyph];
    dvi_font_select(st, glyph.font)?;
    st.shipout.dvi.set_glyph(glyph.width, glyph.glyph)?;
    st.shipout.cur_h += glyph.width;
    st.shipout.dvi_h = st.shipout.cur_h;
    Ok(())
}

/// A native word, with its text as ActualText for `actual_text`.
fn native_word_out(
    st: &mut crate::state::EngineState,
    word: NodePtr<NativeWordNode>,
    actual_text: bool,
) -> Result<(), Abort> {
    synch_h(st)?;
    synch_v(st)?;
    dvi_font_select(st, st.mem[word].font)?;
    if actual_text {
        if st.mem[word].length > 0 || st.mem[word].glyph_info != 0 {
            let (width, locations, gids) = glyph_array(&st.mem, word);
            let text = st.mem.entries(word);
            st.shipout
                .dvi
                .set_text_and_glyphs(text, width, locations, gids)?;
        }
    } else if st.mem[word].glyph_info != 0 {
        let (width, locations, gids) = glyph_array(&st.mem, word);
        st.shipout.dvi.set_glyphs(width, locations, gids)?;
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

/// Output the characters starting at `p`, returning the first non-character
/// node and the matching advance of `prev_p`.
fn chars_out(
    st: &mut crate::state::EngineState,
    mut p: MemPtr,
    mut prev_p: MemPtr,
) -> Result<(MemPtr, MemPtr), Abort> {
    synch_h(st)?;
    synch_v(st)?;
    while let Some(ch) = st.mem.as_char(p) {
        let f = st.mem[ch].font;
        let mut c = st.mem[ch].character;
        if p != LIG_TRICK {
            if let Some(mapping) = st.fonts.font_mapping[f].as_deref() {
                c = apply_tfm_font_mapping(mapping, c as libc::c_int) as u16;
            }
        }
        dvi_font_select(st, f)?;
        if st.fonts.char_exists(f, i32::from(c)) {
            st.shipout.dvi.set_char(c)?;
            st.shipout.cur_h += st.fonts.font_character_width(f, i32::from(c));
        }
        prev_p = prev_p.link(&st.mem);
        p = p.link(&st.mem);
    }
    crate::synctex::synctex_current(st);
    st.shipout.dvi_h = st.shipout.cur_h;
    Ok((p, prev_p))
}

fn hlist_out(st: &mut crate::state::EngineState, this_box: NodePtr<BoxNode>) -> Result<(), Abort> {
    st.mem.temp_ptr = this_box.ptr();
    let mut rounding = GlueRounding::new(&st.mem, this_box);
    if st.eqtb.int_par(IntPar::XetexInterwordSpaceShaping) > 1 {
        st.mem
            .merge_justifiable_native_words(&mut st.strings, &st.fonts, &st.eqtb, this_box)?;
    }

    // A raw cursor: it can be LIG_TRICK, and nodes get retyped in place.
    let mut p = st.mem[this_box].list_ptr;
    let save_loc = st.shipout.dvi.enter_level()?;
    let base_line = st.shipout.cur_v;
    let mut prev_p = this_box.list_head();
    st.mem.push_lr(&mut st.scanner.lr_ptr, MathCode::Before)?;

    if st.mem[this_box].lr() == BoxLr::Dlist {
        if st.scanner.cur_dir == Dir::Rtl {
            st.scanner.cur_dir = Dir::Ltr;
            st.shipout.cur_h -= st.mem[this_box].width;
        } else {
            st.mem[this_box].set_lr(BoxLr::Normal);
        }
    }

    if st.scanner.cur_dir == Dir::Rtl && st.mem[this_box].lr() != BoxLr::Reversed {
        let save_h = st.shipout.cur_h;
        st.mem.temp_ptr = p;
        let kern = st.mem.new_kern(Scaled::ZERO)?;
        st.mem[kern].sync_tag = 0;
        prev_p.set_link(&mut st.mem, kern.ptr());
        st.shipout.cur_h = Scaled::ZERO;
        let reversed = st.mem.reverse(
            &mut st.shipout,
            &mut st.scanner,
            &mut st.fonts,
            None,
            &mut rounding,
        )?;
        st.mem[kern].link = reversed;
        st.mem[kern].width = -st.shipout.cur_h;
        st.shipout.cur_h = save_h;
        st.mem[this_box].set_lr(BoxLr::Reversed);
        p = kern.ptr();
    }

    let mut left_edge = st.shipout.cur_h;
    crate::synctex::synctex_hlist(st, this_box);

    while !p.is_null() {
        if st.mem.is_char_node(p) {
            (p, prev_p) = chars_out(st, p, prev_p)?;
            continue;
        }
        match st
            .mem
            .classify(p)
            .map_err(|_| Abort::Confusion("hlistout"))?
        {
            AnyNode::Box(b) => hlist_box_out(st, b, base_line)?,
            AnyNode::Rule(rule) => {
                st.shipout.rule_ht = st.mem[rule].height.to_scaled();
                st.shipout.rule_dp = st.mem[rule].depth.to_scaled();
                st.shipout.rule_wd = st.mem[rule].width.to_scaled();
                hlist_rule_out(st, this_box, base_line, rule.ptr())?;
            }
            AnyNode::Whatsit(whatsit) => match whatsit.variant(&st.mem) {
                Some(WhatsitVariant::NativeWord(word)) => {
                    let actual_text =
                        whatsit.whatsit_subtype(&st.mem) == Some(WhatsitSubtype::NativeWordAt);
                    native_word_out(st, word, actual_text)?
                }
                Some(WhatsitVariant::Glyph(glyph)) => glyph_out(st, glyph)?,
                Some(WhatsitVariant::Pic(pic)) => {
                    st.shipout.cur_v = base_line;
                    let edge = st.shipout.cur_h + st.mem[pic].width;
                    with_saved_dvi_pos(st, |st| pic_out(st, pic))?;
                    st.shipout.cur_h = edge;
                    st.shipout.cur_v = base_line;
                }
                Some(WhatsitVariant::SavePos) => st.shipout.pdf_save_pos(),
                _ => out_what(st, whatsit)?,
            },
            AnyNode::Glue(glue) => {
                hlist_glue_out(st, this_box, base_line, left_edge, glue, &mut rounding)?
            }
            AnyNode::MarginKern(kern) => st.shipout.cur_h += st.mem[kern].width,
            AnyNode::Kern(kern) => {
                crate::synctex::synctex_kern(st, kern, this_box);
                st.shipout.cur_h += st.mem[kern].width;
            }
            AnyNode::Math(math) => {
                crate::synctex::synctex_math(st, math);
                let math_width = st.mem[math].width;
                let code = st.mem[math].code()?;
                if code.is_end() {
                    if st.mem.lr_matches(&st.scanner, code) {
                        st.mem.pop_lr(&mut st.scanner.lr_ptr);
                    } else if code.is_lr() {
                        st.scanner.lr_problems += 1;
                    }
                } else {
                    st.mem.push_lr(&mut st.scanner.lr_ptr, code.lr_end())?;
                    if code.dir() != st.scanner.cur_dir {
                        let edge = reverse_segment(
                            st,
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
                st.mem.set_word(LIG_TRICK, st.mem.word(ligature.idx() + 1));
                let next = st.mem[ligature].link;
                p = LIG_TRICK;
                p.set_link(&mut st.mem, next);
                st.fonts.xtx_ligature_present = true;
                continue;
            }
            AnyNode::Style(style) => {
                let edge = style.as_edge();
                st.shipout.cur_h += st.mem[edge].width;
                left_edge = st.shipout.cur_h + st.mem[edge].edge_dist;
                st.scanner.cur_dir = st.mem[edge].dir.get().ok_or(Abort::Confusion("LR"))?;
            }
            _ => {}
        }
        prev_p = p;
        p = p.link(&st.mem);
    }

    crate::synctex::synctex_tsilh(st, this_box);

    while st.mem.lr_code(st.scanner.lr_ptr) != Some(MathCode::Before) {
        if st
            .mem
            .lr_code(st.scanner.lr_ptr)
            .is_some_and(MathCode::is_lr)
        {
            st.scanner.lr_problems += 10000;
        }
        st.mem.pop_lr(&mut st.scanner.lr_ptr);
    }
    st.mem.pop_lr(&mut st.scanner.lr_ptr);
    if st.mem[this_box].lr() == BoxLr::Dlist {
        st.scanner.cur_dir = Dir::Rtl;
    }
    st.mem.pop_dvi_level(&mut st.shipout, save_loc)
}

fn hlist_box_out(
    st: &mut crate::state::EngineState,
    b: NodePtr<BoxNode>,
    base_line: Scaled,
) -> Result<(), Abort> {
    if st.mem[b].list_ptr.is_null() {
        if st.mem[b].is_vlist() {
            crate::synctex::synctex_void_vlist(st, b);
        } else {
            crate::synctex::synctex_void_hlist(st, b);
        }
        st.shipout.cur_h += st.mem[b].width;
        return Ok(());
    }
    st.shipout.cur_v = base_line + st.mem[b].shift_amount;
    let edge = st.shipout.cur_h + st.mem[b].width;
    if st.scanner.cur_dir == Dir::Rtl {
        st.shipout.cur_h = edge;
    }
    with_saved_dvi_pos(st, |st| ship_box(st, b))?;
    st.shipout.cur_h = edge;
    st.shipout.cur_v = base_line;
    Ok(())
}

/// Reverse the hlist segment that starts after `math`, replacing `math` by an
/// edge node that the caller dispatches next.
fn reverse_segment(
    st: &mut crate::state::EngineState,
    math: NodePtr<MathNode>,
    prev_p: MemPtr,
    left_edge: Scaled,
    math_width: Scaled,
    rounding: &mut GlueRounding,
) -> Result<NodePtr<EdgeNode>, Abort> {
    let save_h = st.shipout.cur_h;
    st.mem.temp_ptr = st.mem[math].link;
    st.shipout.rule_wd = math_width;
    st.mem.free_node(math.ptr(), MEDIUM_NODE_SIZE);
    st.scanner.cur_dir = st.scanner.cur_dir.reversed();
    let edge = st.mem.new_edge(st.scanner.cur_dir, st.shipout.rule_wd)?;
    prev_p.set_link(&mut st.mem, edge.ptr());
    st.shipout.cur_h = st.shipout.cur_h - left_edge + st.shipout.rule_wd;
    let end = st
        .mem
        .new_edge(st.scanner.cur_dir.reversed(), Scaled::ZERO)?;
    let reversed = st.mem.reverse(
        &mut st.shipout,
        &mut st.scanner,
        &mut st.fonts,
        Some(end),
        rounding,
    )?;
    st.mem[edge].link = reversed;
    st.mem[edge].edge_dist = st.shipout.cur_h;
    st.scanner.cur_dir = st.scanner.cur_dir.reversed();
    st.shipout.cur_h = save_h;
    Ok(edge)
}

fn vlist_out(st: &mut crate::state::EngineState, this_box: NodePtr<BoxNode>) -> Result<(), Abort> {
    st.mem.temp_ptr = this_box.ptr();
    let mut rounding = GlueRounding::new(&st.mem, this_box);
    let mut p = st.mem[this_box].list_ptr;
    let upwards = st.mem[this_box].upwards();
    let save_loc = st.shipout.dvi.enter_level()?;
    let left_edge = st.shipout.cur_h;
    crate::synctex::synctex_vlist(st, this_box);
    if upwards {
        st.shipout.cur_v += st.mem[this_box].depth;
    } else {
        st.shipout.cur_v -= st.mem[this_box].height;
    }
    let top_edge = st.shipout.cur_v;

    while !p.is_null() {
        if st.mem.is_char_node(p) {
            return Err(Abort::Confusion("vlistout"));
        }
        match st
            .mem
            .classify(p)
            .map_err(|_| Abort::Confusion("vlistout"))?
        {
            AnyNode::Box(b) => vlist_box_out(st, b, left_edge, upwards)?,
            AnyNode::Rule(rule) => {
                st.shipout.rule_ht = st.mem[rule].height.to_scaled();
                st.shipout.rule_dp = st.mem[rule].depth.to_scaled();
                st.shipout.rule_wd = st.mem[rule].width.to_scaled();
                vlist_rule_out(st, this_box, left_edge, upwards)?;
            }
            AnyNode::Whatsit(whatsit) => match whatsit.variant(&st.mem) {
                Some(WhatsitVariant::Glyph(glyph)) => {
                    let glyph = st.mem[glyph];
                    st.shipout.cur_v += glyph.height;
                    st.shipout.cur_h = left_edge;
                    synch_h(st)?;
                    synch_v(st)?;
                    dvi_font_select(st, glyph.font)?;
                    st.shipout.dvi.set_glyph(Scaled::ZERO, glyph.glyph)?;
                    st.shipout.cur_v += glyph.depth;
                    st.shipout.cur_h = left_edge;
                }
                Some(WhatsitVariant::Pic(pic)) => {
                    let (_, save_v) = with_saved_dvi_pos(st, |st| {
                        st.shipout.cur_v += st.mem[pic].height;
                        pic_out(st, pic)
                    })?;
                    st.shipout.cur_v = save_v + st.mem[pic].depth;
                    st.shipout.cur_h = left_edge;
                }
                Some(WhatsitVariant::SavePos) => st.shipout.pdf_save_pos(),
                _ => out_what(st, whatsit)?,
            },
            AnyNode::Glue(glue) => {
                let spec = st.mem[glue].glue_ptr;
                st.shipout.rule_ht = rounding.width(&st.mem, spec);
                if let Some(kind) = st.mem[glue].leaders() {
                    match st.mem.leader_of(glue) {
                        Leader::Rule(rule) => {
                            st.shipout.rule_wd = st.mem[rule].width.to_scaled();
                            st.shipout.rule_dp = Scaled::ZERO;
                            vlist_rule_out(st, this_box, left_edge, upwards)?;
                        }
                        Leader::Box(leader) => {
                            vlist_leaders_out(st, leader, kind, left_edge, top_edge, upwards)?
                        }
                    }
                } else {
                    st.shipout.move_past_vlist_rule(upwards);
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
        p = p.link(&st.mem);
    }

    crate::synctex::synctex_tsilv(st, this_box);
    st.mem.pop_dvi_level(&mut st.shipout, save_loc)
}

fn vlist_box_out(
    st: &mut crate::state::EngineState,
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
    if st.mem[b].list_ptr.is_null() {
        if st.mem[b].is_vlist() {
            crate::synctex::synctex_void_vlist(st, b);
        } else {
            crate::synctex::synctex_void_hlist(st, b);
        }
        if upwards {
            st.shipout.cur_v -= height;
        } else {
            st.shipout.cur_v += depth;
        }
        return Ok(());
    }
    synch_v(st)?;
    let shift = st.mem[b].shift_amount;
    st.shipout.cur_h = if st.scanner.cur_dir == Dir::Rtl {
        left_edge - shift
    } else {
        left_edge + shift
    };
    let (_, save_v) = with_saved_dvi_pos(st, |st| ship_box(st, b))?;
    st.shipout.cur_v = if upwards {
        save_v - st.mem[b].height
    } else {
        save_v + st.mem[b].depth
    };
    st.shipout.cur_h = left_edge;
    Ok(())
}

/// Runs `out`, then puts `dvi_h` and `dvi_v` back to where they were,
/// returning that position.
fn with_saved_dvi_pos(
    st: &mut crate::state::EngineState,
    out: impl FnOnce(&mut crate::state::EngineState) -> Result<(), Abort>,
) -> Result<(Scaled, Scaled), Abort> {
    let (h, v) = (st.shipout.dvi_h, st.shipout.dvi_v);
    out(st)?;
    st.shipout.dvi_h = h;
    st.shipout.dvi_v = v;
    Ok((h, v))
}

fn vlist_leaders_out(
    st: &mut crate::state::EngineState,
    leader: NodePtr<BoxNode>,
    kind: LeaderKind,
    left_edge: Scaled,
    top_edge: Scaled,
    upwards: bool,
) -> Result<(), Abort> {
    let leader_ht = st.mem[leader].height + st.mem[leader].depth;
    if leader_ht <= Scaled::ZERO || st.shipout.rule_ht <= Scaled::ZERO {
        st.shipout.move_past_vlist_rule(upwards);
        return Ok(());
    }
    st.shipout.rule_ht += LEADER_SLOP;
    let edge = st.shipout.cur_v + st.shipout.rule_ht;
    let mut lx = Scaled::ZERO;
    if kind == LeaderKind::Aligned {
        let save_v = st.shipout.cur_v;
        st.shipout.cur_v =
            top_edge + leader_ht * ((st.shipout.cur_v - top_edge).raw() / leader_ht.raw());
        if st.shipout.cur_v < save_v {
            st.shipout.cur_v += leader_ht;
        }
    } else {
        let (lq, lr) = leader_count_and_rest(st.shipout.rule_ht, leader_ht);
        if kind == LeaderKind::Centered {
            st.shipout.cur_v += lr / 2;
        } else {
            lx = lr / (lq + 1);
            st.shipout.cur_v += (lr - lx * (lq - 1)) / 2;
        }
    }

    while st.shipout.cur_v + leader_ht <= edge {
        let shift = st.mem[leader].shift_amount;
        st.shipout.cur_h = if st.scanner.cur_dir == Dir::Rtl {
            left_edge - shift
        } else {
            left_edge + shift
        };
        synch_h(st)?;
        st.shipout.cur_v += st.mem[leader].height;
        synch_v(st)?;
        let (_, save_v) = with_saved_dvi_pos(st, |st| ship_leader_box(st, leader))?;
        st.shipout.cur_h = left_edge;
        st.shipout.cur_v = save_v - st.mem[leader].height + leader_ht + lx;
    }
    st.shipout.cur_v = edge - LEADER_SLOP;
    Ok(())
}

impl MemState {
    fn prune_movements(&mut self, shipout: &mut ShipoutState, l: i32) {
        self.prune_movement_list(&mut shipout.down_ptr, l);
        self.prune_movement_list(&mut shipout.right_ptr, l);
    }

    fn prune_movement_list(&mut self, head: &mut Option<NodePtr<MovementNode>>, l: i32) {
        while let Some(p) = *head {
            if self[p].location < l {
                return;
            }
            *head = p.next(self);
            p.free(self);
        }
    }
}

pub(crate) fn finalize_dvi_file(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    if st.shipout.dvi.close_open_page(st.scanner.logical_clock)? {
        st.shipout.total_pages += 1;
    }

    if st.shipout.total_pages == 0 {
        crate::output::print_nl(st, "No pages of output.");
        return Ok(());
    }
    // The DVI grew too big and the message was already printed.
    if st.shipout.dvi.depth == -2 {
        return Ok(());
    }

    st.shipout.dvi.begin_postamble()?;
    crate::save::prepare_mag(st)?;
    let mag = st.eqtb.int_par(IntPar::Mag);
    let s = &mut st.shipout;
    s.dvi.end_postamble(mag, s.max_v, s.max_h, s.total_pages)?;

    for f in st.fonts.font_ptr.loaded().rev() {
        if st.fonts.font_used[f] {
            dvi_font_def(st, f)?;
        }
    }
    st.fonts.font_ptr = NULL_FONT;

    st.shipout
        .dvi
        .finish(id_byte(st.config.semantic_pagination_enabled))?;

    let k = u8::from(output_close(st.shipout.dvi.file));
    if k == 0 {
        crate::output::print_nl(st, "Output written on ");
        crate::output::print(st, st.shipout.output_file_name);
        crate::output::print(st, " (");
        crate::output::print_int(st, st.shipout.total_pages);
        if st.shipout.total_pages != 1 {
            crate::output::print(st, " pages");
        } else {
            crate::output::print(st, " page");
        }
        crate::output::print(st, ", ");
        crate::output::print_int(st, st.shipout.dvi.pos());
        crate::output::print(st, " bytes).");
    } else {
        crate::output::print_nl(st, "Error ");
        crate::output::print_int(st, i32::from(k));
        crate::output::print(st, " (");
        crate::output::print(st, &crate::bridge::strerror(i32::from(k))[..]);
        crate::output::print(st, ") generating output;");
        crate::output::print_nl(st, "file ");
        crate::output::print(st, st.shipout.output_file_name);
        crate::output::print(st, " may not be valid.");
        // XeTeX also sets history to OUTPUT_FAILURE here.
    }
    Ok(())
}

/// The glyph array of `word`: its width, glyph positions and glyph ids.
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
        scanner: &mut ScannerState,
        fonts: &mut FontState,
        t: Option<NodePtr<EdgeNode>>,
        rounding: &mut GlueRounding,
    ) -> Result<MemPtr, Abort> {
        let mut l = t.map_or(MemPtr::NULL, |t| t.ptr());
        let mut p = self.temp_ptr;
        let mut m = MIN_HALFWORD;
        let mut n = MIN_HALFWORD;

        loop {
            while !p.is_null() {
                if self.is_char_node(p) {
                    while let Some(ch) = self.as_char(p) {
                        let f = self[ch].font;
                        let c = fonts.effective_char(f, self[ch].character);
                        shipout.cur_h += fonts.font_character_width(f, c);
                        let q = p.link(self);
                        p.set_link(self, l);
                        l = p;
                        p = q;
                    }
                    continue;
                }

                let q = p.link(self);
                let mut add_width = true;
                let mut now_kern = false;
                match self.classify(p) {
                    Ok(node @ (AnyNode::Box(_) | AnyNode::Rule(_))) => {
                        shipout.rule_wd = node.hlist_width(self);
                    }
                    Ok(AnyNode::Kern(kern)) => {
                        shipout.rule_wd = self[kern].width;
                        now_kern = true;
                    }
                    Ok(AnyNode::Whatsit(w)) => {
                        if let Some(sized) = w.sized(self) {
                            shipout.rule_wd = self[sized].width;
                        } else {
                            add_width = false;
                        }
                    }
                    Ok(AnyNode::Glue(glue)) => {
                        let spec = self[glue].glue_ptr;
                        shipout.rule_wd = rounding.width(self, spec);
                        if rounding.is_set(self, spec) {
                            self.freeze_set_glue(glue, spec, shipout.rule_wd)?;
                            now_kern = self[glue].leaders().is_none();
                        }
                    }
                    Ok(AnyNode::Ligature(ligature)) => {
                        self.flush_list(self[ligature].lig_ptr);
                        self.temp_ptr = ligature.ptr();
                        let c = self.get_avail()?;
                        self.set_word(c, self.word(ligature.idx() + 1));
                        c.set_link(self, q);
                        self.free_node(ligature.ptr(), SMALL_NODE_SIZE);
                        p = c;
                        continue;
                    }
                    Ok(AnyNode::Math(math)) => {
                        shipout.rule_wd = self[math].width;
                        let code = self[math].code()?;
                        if code.is_end() {
                            if !self.lr_matches(scanner, code) {
                                math.into_kern(self);
                                now_kern = true;
                                scanner.lr_problems += 1;
                            } else {
                                self.pop_lr(&mut scanner.lr_ptr);
                                if n > MIN_HALFWORD {
                                    n -= 1;
                                    self[math].subtype = code.as_begin().into();
                                } else {
                                    math.into_kern(self);
                                    now_kern = true;
                                    if m > MIN_HALFWORD {
                                        m -= 1;
                                    } else {
                                        self.free_node(math.ptr(), MEDIUM_NODE_SIZE);
                                        let t = t.expect("a segment reversal ends at its edge");
                                        self[t].link = q;
                                        self[t].width = shipout.rule_wd;
                                        self[t].edge_dist = -shipout.cur_h - shipout.rule_wd;
                                        return Ok(l);
                                    }
                                }
                            }
                        } else {
                            self.push_lr(&mut scanner.lr_ptr, code.lr_end())?;
                            if n > MIN_HALFWORD || code.dir() != scanner.cur_dir {
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
                    shipout.cur_h += shipout.rule_wd;
                }
                p.set_link(self, l);
                if now_kern && (shipout.rule_wd == Scaled::ZERO || l.is_null()) {
                    self.free_node(p, MEDIUM_NODE_SIZE);
                    p = l;
                }
                l = p;
                p = q;
            }

            if t.is_none() && m == MIN_HALFWORD && n == MIN_HALFWORD {
                return Ok(l);
            }
            let code = self.lr_code(scanner.lr_ptr).ok_or(Abort::Confusion("LR"))?;
            p = self.new_math(Scaled::ZERO, code)?.ptr();
            scanner.lr_problems += 10000;
        }
    }

    fn movement(&mut self, shipout: &mut ShipoutState, w: Scaled, axis: Axis) -> Result<(), Abort> {
        let mut q = NodePtr::<MovementNode>::alloc(self)?;
        self[q].distance = w;
        self[q].location = shipout.dvi.pos();
        let head = match axis {
            Axis::Down => &mut shipout.down_ptr,
            Axis::Right => &mut shipout.right_ptr,
        };
        self[q].link = link_of(*head);
        *head = Some(q);

        let mut p = q.next(self);
        let mut mstate = MOV_NONE_SEEN;
        let found = 'search: {
            while let Some(node) = p {
                p = node.next(self);
                if self[node].distance != w {
                    match mstate + self[node].state {
                        1 => mstate = MOV_Y_SEEN,
                        2 => mstate = MOV_Z_SEEN,
                        8 | 13 => break 'search None,
                        _ => {}
                    }
                    continue;
                }
                let (reg, here) = match mstate + self[node].state {
                    // NONE_SEEN or Z_SEEN meeting YZ_OK or Y_OK
                    3 | 4 | 15 | 16 => (MoveReg::Y, MOV_Y_HERE),
                    // NONE_SEEN meeting Z_OK, Y_SEEN meeting YZ_OK or Z_OK
                    5 | 9 | 11 => (MoveReg::Z, MOV_Z_HERE),
                    // a y or z that is already here
                    1 | 2 | 8 | 13 => break 'search Some(node),
                    _ => continue,
                };
                if !shipout.dvi.set_reg_at(self[node].location, reg) {
                    break 'search None;
                }
                self[node].state = here;
                break 'search Some(node);
            }
            None
        };

        let Some(found) = found else {
            self[q].state = MOV_YZ_OK;
            return shipout.dvi.movement(axis, w);
        };

        self[q].state = self[found].state;
        let (reg, yz_becomes, fixed_from) = if self[q].state == MOV_Y_HERE {
            (MoveReg::Y, MOV_Z_OK, MOV_Y_OK)
        } else {
            (MoveReg::Z, MOV_Y_OK, MOV_Z_OK)
        };
        while self[q].link != found.ptr() {
            q = NodePtr::<MovementNode>::from_ptr(self[q].link)
                .expect("found movement record is linked from the new record");
            let state = self[q].state;
            if state == MOV_YZ_OK {
                self[q].state = yz_becomes;
            } else if state == fixed_from {
                self[q].state = MOV_D_FIXED;
            }
        }
        shipout.dvi.movement_by_reg(axis, reg)
    }
}
