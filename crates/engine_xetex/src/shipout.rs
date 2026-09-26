//! Shipping boxes out to the XDV file.
use crate::consts::{
    A_LEADERS, BEFORE, CS_TOKEN_FLAG, C_LEADERS, DIMEN_PAR_H_OFFSET, DIMEN_PAR_PDF_PAGE_HEIGHT,
    DIMEN_PAR_PDF_PAGE_WIDTH, DIMEN_PAR_V_OFFSET, DLIST, END_WRITE, FILLL, INT_PAR_MAG,
    INT_PAR_TRACING_ONLINE, INT_PAR_TRACING_OUTPUT, INT_PAR_XETEX_INTERWORD_SPACE_SHAPING,
    INVALID_HANDLE, LEFT_TO_RIGHT, L_CODE, MAX_DIMEN, MEDIUM_NODE_SIZE, MIN_HALFWORD,
    NATIVE_GLYPH_INFO_SIZE, NORMAL, NULL_FONT, RIGHT_TO_LEFT, R_CODE, SELECTOR_LOG_ONLY,
    SELECTOR_NEW_STRING, SELECTOR_TERM_AND_LOG, SELECTOR_TERM_ONLY, SMALL_NODE_SIZE,
    SPACE_ADJUSTMENT, TEX_INFINITY, TEX_NULL,
};
use crate::ext::apply_tfm_font_mapping;

use crate::bridge::{output_close, output_flush, output_open, output_write, shell_escape};
use crate::errors::Abort;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::input::{INSERTED, WRITE_TEXT};
use crate::mem::{end_lr_type, LIG_TRICK};
use crate::node::{
    AnyBox, AnyNode, EdgeNode, GlueNode, GlueSpecNode, MathNode, MovementNode, NodePtr, RuleNode,
    WhatsitNode, WhatsitSubtype, RUNNING,
};
use crate::scan::{LEFT_BRACE_TOKEN, RIGHT_BRACE_TOKEN};
use crate::state::{EqtbState, FontState, MemState, ScannerState, ShipoutState, StringsState};
use crate::units::{FontId, GlueSign, MemIdx, MemPtr, Scaled, StrNumber};

const DVI_BUF_SIZE: i32 = 16384;
const HALF_BUF: i32 = 8192;

const SET1: u8 = 128;
const SET_RULE: u8 = 132;
const PUT_RULE: u8 = 137;
const BOP: u8 = 139;
const EOP: u8 = 140;
const PUSH: u8 = 141;
const POP: u8 = 142;
const RIGHT1: u8 = 143;
const DOWN1: u8 = 157;
const FNT_NUM_0: u8 = 171;
const FNT1: u8 = 235;
const XXX1: u8 = 239;
const XXX4: u8 = 242;
const FNT_DEF1: u8 = 243;
const PRE: u8 = 247;
const POST: u8 = 248;
const POST_POST: u8 = 249;
const DEFINE_NATIVE_FONT: u8 = 252;
const SET_GLYPHS: u8 = 253;
const SET_TEXT_AND_GLYPHS: u8 = 254;
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

const REVERSED: u16 = 1;

/// Slack TeX adds to leader glue against floating-point rounding.
const LEADER_SLOP: Scaled = Scaled(10);

/// How many leader boxes of size `size` fit in `total`, and the leftover space.
fn leader_count_and_rest(total: Scaled, size: Scaled) -> (i32, Scaled) {
    (total.0 / size.0, Scaled(total.0 % size.0))
}

pub(crate) const ONE_INCH: Scaled = Scaled(4_736_287);
const ONE_INCH_LESS_ONE: Scaled = Scaled(ONE_INCH.0 - 1);

impl MemState {
    pub fn new_edge(&mut self, s: i16, w: Scaled) -> Result<NodePtr<EdgeNode>, Abort> {
        let p = NodePtr::<EdgeNode>::alloc(self)?;
        p.set_subtype(self, s as u16);
        p.set_width(self, w);
        p.set_edge_dist(self, Scaled::ZERO);
        Ok(p)
    }
}

impl ShipoutState {
    fn write_to_dvi(&self, a: i32, b: i32) -> Result<(), Abort> {
        let bytes = &self.dvi_buf[a as usize..=b as usize];
        let written = output_write(self.dvi_file, bytes);
        if written != bytes.len() {
            return Err(Abort::Internal("failed to write data to XDV file"));
        }
        Ok(())
    }

    fn dvi_swap(&mut self) -> Result<(), Abort> {
        if self.dvi_ptr > TEX_INFINITY - self.dvi_offset {
            self.cur_s = -2;
            return Err(Abort::Fatal("dvi length exceeds 0x7FFFFFFF"));
        }
        if self.dvi_limit == DVI_BUF_SIZE {
            self.write_to_dvi(0, HALF_BUF - 1)?;
            self.dvi_limit = HALF_BUF;
            self.dvi_offset += DVI_BUF_SIZE;
            self.dvi_ptr = 0;
        } else {
            self.write_to_dvi(HALF_BUF, DVI_BUF_SIZE - 1)?;
            self.dvi_limit = DVI_BUF_SIZE;
        }
        self.dvi_gone += HALF_BUF;
        Ok(())
    }

    #[inline]
    fn dvi_out(&mut self, c: u8) -> Result<(), Abort> {
        self.page_manifest.record_byte(c);
        self.dvi_buf[self.dvi_ptr as usize] = c;
        self.dvi_ptr += 1;
        if self.dvi_ptr == self.dvi_limit {
            self.dvi_swap()?;
        }
        Ok(())
    }

    fn dvi_bytes(&mut self, mut bytes: &[u8]) -> Result<(), Abort> {
        while !bytes.is_empty() {
            let ptr = self.dvi_ptr as usize;
            let n = bytes.len().min(self.dvi_limit as usize - ptr);
            self.dvi_buf[ptr..ptr + n].copy_from_slice(&bytes[..n]);
            self.page_manifest.record_bytes(n);
            self.dvi_ptr += n as i32;
            bytes = &bytes[n..];
            if self.dvi_ptr == self.dvi_limit {
                self.dvi_swap()?;
            }
        }
        Ok(())
    }

    fn dvi_four(&mut self, x: i32) -> Result<(), Abort> {
        self.dvi_bytes(&x.to_be_bytes())
    }

    fn dvi_two(&mut self, s: u16) -> Result<(), Abort> {
        self.dvi_bytes(&s.to_be_bytes())
    }
}

fn synch_h(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    if st.shipout.cur_h != st.shipout.dvi_h {
        let delta = st.shipout.cur_h - st.shipout.dvi_h;
        st.mem.movement(&mut st.shipout, delta, RIGHT1)?;
        st.shipout.dvi_h = st.shipout.cur_h;
    }
    Ok(())
}

fn synch_v(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    if st.shipout.cur_v != st.shipout.dvi_v {
        let delta = st.shipout.cur_v - st.shipout.dvi_v;
        st.mem.movement(&mut st.shipout, delta, DOWN1)?;
        st.shipout.dvi_v = st.shipout.cur_v;
    }
    Ok(())
}

fn dvi_native_font_def(st: &mut crate::state::EngineState, f: FontId) -> Result<(), Abort> {
    let font_def_length = st.fonts.make_font_def(f)?;
    st.shipout.dvi_out(DEFINE_NATIVE_FONT)?;
    st.shipout.dvi_four(f.dvi_number())?;
    st.shipout
        .dvi_bytes(&st.fonts.xdv_buffer[..font_def_length as usize])
}

fn dvi_font_def(st: &mut crate::state::EngineState, f: FontId) -> Result<(), Abort> {
    let font_area = st.fonts.font_area[f];
    if font_area == AAT_FONT_FLAG || font_area == OTGR_FONT_FLAG {
        return dvi_native_font_def(st, f);
    }
    let k = f.dvi_number();
    if k < 256 {
        st.shipout.dvi_bytes(&[FNT_DEF1, k as u8])?;
    } else {
        st.shipout
            .dvi_bytes(&[FNT_DEF1 + 1, (k / 256) as u8, (k % 256) as u8])?;
    }
    let check = st.fonts.font_check[f];
    st.shipout.dvi_bytes(&[
        check.s3 as u8,
        check.s2 as u8,
        check.s1 as u8,
        check.s0 as u8,
    ])?;
    st.shipout.dvi_four(st.fonts.font_size[f].0)?;
    st.shipout.dvi_four(st.fonts.font_dsize[f].0)?;
    st.shipout.dvi_out(st.strings.length(font_area) as u8)?;

    let font_name = st.fonts.font_name[f];
    let name = st.strings.str_range(font_name);
    // Only the part before a `:features` suffix is written (a leading colon does not count).
    let colon = st.strings.str_pool[name.clone()]
        .iter()
        .skip(1)
        .position(|&c| c == u16::from(b':'))
        .map(|l| l + 1);
    let name_len = colon.map_or_else(|| st.strings.length(font_name), |l| l as i32);
    st.shipout.dvi_out(name_len as u8)?;

    let area = st.strings.str_range(font_area);
    for k in area {
        st.shipout.dvi_out(st.strings.str_pool[k] as u8)?;
    }
    let name_start = name.start;
    for k in name_start..name_start + name_len as usize {
        st.shipout.dvi_out(st.strings.str_pool[k] as u8)?;
    }
    Ok(())
}

pub fn initialize_shipout_variables(st: &mut crate::state::EngineState) {
    let s = &mut st.shipout;
    s.output_file_name = StrNumber::NONE;
    s.dvi_buf = vec![0u8; (DVI_BUF_SIZE + 1) as usize];
    s.dvi_limit = DVI_BUF_SIZE;
    s.dvi_ptr = 0;
    s.dvi_offset = 0;
    s.dvi_gone = 0;
    s.down_ptr = MemPtr::NULL;
    s.right_ptr = MemPtr::NULL;
    s.cur_s = -1;
}

/// Emit the current string (the tail of the pool) as a DVI special and drop it.
fn dvi_special_from_cur_string(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    let base = st.strings.cur_string_start();
    let cur_len = st.strings.pool_ptr - base;
    if cur_len < 256 {
        st.shipout.dvi_bytes(&[XXX1, cur_len as u8])?;
    } else {
        st.shipout.dvi_out(XXX4)?;
        st.shipout.dvi_four(cur_len)?;
    }
    for k in base..st.strings.pool_ptr {
        st.shipout.dvi_out(st.strings.str_pool[k as usize] as u8)?;
    }
    st.strings.pool_ptr = base;
    Ok(())
}

pub(crate) fn ship_out(st: &mut crate::state::EngineState, page: AnyBox) -> Result<(), Abort> {
    let p = page.ptr();
    let mag = st.eqtb.int_par(INT_PAR_MAG);
    let job_name = st.strings.texstring_bytes(st.strings.job_name);
    crate::synctex::synctex_sheet(st, mag, &job_name);

    if st.strings.job_name == StrNumber::NONE {
        crate::io::open_log_file(st)?;
    }
    let tracing_output = st.eqtb.int_par(INT_PAR_TRACING_OUTPUT) > 0;
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

    let (height, depth, width) = (
        page.height(&st.mem),
        page.depth(&st.mem),
        page.width(&st.mem),
    );
    let h_offset = st.eqtb.dimen_par(DIMEN_PAR_H_OFFSET);
    let v_offset = st.eqtb.dimen_par(DIMEN_PAR_V_OFFSET);
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

        if st.eqtb.int_par(INT_PAR_TRACING_OUTPUT) <= 0 {
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

    if !st.scanner.lr_ptr.is_null() || st.scanner.cur_dir != LEFT_TO_RIGHT {
        return Err(Abort::Confusion("LR3"));
    }
    if st.eqtb.int_par(INT_PAR_TRACING_OUTPUT) <= 0 {
        crate::output::print_char(st, b']');
    }
    st.shipout.dead_cycles = 0;
    output_flush(st.shipout.rust_stdout);
    st.mem.flush_node_list(p)?;
    crate::synctex::synctex_teehs(st);
    Ok(())
}

fn ship_page(st: &mut crate::state::EngineState, page: AnyBox) -> Result<(), Abort> {
    let (height, depth, width) = (
        page.height(&st.mem),
        page.depth(&st.mem),
        page.width(&st.mem),
    );
    let h_offset = st.eqtb.dimen_par(DIMEN_PAR_H_OFFSET);
    let v_offset = st.eqtb.dimen_par(DIMEN_PAR_V_OFFSET);
    let pdf_page_width = st.eqtb.dimen_par(DIMEN_PAR_PDF_PAGE_WIDTH);
    let pdf_page_height = st.eqtb.dimen_par(DIMEN_PAR_PDF_PAGE_HEIGHT);

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
        st.shipout.dvi_file = output_open(st.input.name_of_file_bytes(), false);
        if st.shipout.dvi_file == INVALID_HANDLE {
            return Err(Abort::CannotOpenOutput);
        }
        st.shipout.output_file_name = crate::io::make_name_string(st)?;
    }

    if st.shipout.total_pages == 0 {
        st.shipout.dvi_out(PRE)?;
        st.shipout
            .dvi_out(id_byte(st.config.semantic_pagination_enabled))?;
        st.shipout.dvi_four(25_400_000)?;
        st.shipout.dvi_four(473_628_672)?;
        crate::save::prepare_mag(st)?;
        st.shipout.dvi_four(st.eqtb.int_par(INT_PAR_MAG))?;
        let comment = b"tectonic";
        st.shipout.dvi_out(comment.len() as u8)?;
        st.shipout.dvi_bytes(comment)?;
    }

    let page_loc = st.shipout.dvi_offset + st.shipout.dvi_ptr;
    st.shipout.page_manifest.begin_page(page_loc as u64);
    st.shipout.dvi_out(BOP)?;
    for k in 0..10 {
        st.shipout.dvi_four(st.eqtb.count(k))?;
    }
    let last_bop = st.shipout.last_bop;
    st.shipout.dvi_four(last_bop)?;
    st.shipout.last_bop = page_loc;

    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
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

    st.shipout.dvi_out(XXX1)?;
    st.shipout.dvi_out(st.strings.cur_length() as u8)?;
    let base = st.strings.cur_string_start();
    for k in base..st.strings.pool_ptr {
        st.shipout.dvi_out(st.strings.str_pool[k as usize] as u8)?;
    }
    st.strings.pool_ptr = base;

    st.shipout.cur_v = height + v_offset;
    match page {
        AnyBox::VList(_) => vlist_out(st, page)?,
        AnyBox::HList(_) => hlist_out(st, page)?,
    }

    st.shipout.dvi_out(EOP)?;
    st.shipout
        .page_manifest
        .finish_page(st.scanner.logical_clock);
    st.shipout.total_pages += 1;
    st.shipout.cur_s = -1;
    Ok(())
}

fn id_byte(semantic_pagination: bool) -> u8 {
    if semantic_pagination {
        SPX_ID_BYTE
    } else {
        XDV_ID_BYTE
    }
}

impl ShipoutState {
    fn dvi_pop(&mut self, l: i32) -> Result<(), Abort> {
        if l == self.dvi_offset + self.dvi_ptr && self.dvi_ptr > 0 {
            self.dvi_ptr -= 1;
            self.page_manifest.pop_byte();
            Ok(())
        } else {
            self.dvi_out(POP)
        }
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
    let k = f.dvi_number();
    if k < 64 {
        st.shipout.dvi_out(k as u8 + FNT_NUM_0)?;
    } else if k < 256 {
        st.shipout.dvi_bytes(&[FNT1, k as u8])?;
    } else {
        st.shipout
            .dvi_bytes(&[FNT1 + 1, (k / 256) as u8, (k % 256) as u8])?;
    }
    st.shipout.dvi_f = f;
    Ok(())
}

/// The glue-setting state of the box being shipped, carried across glue nodes
/// so rounding errors do not accumulate.
struct GlueRounding {
    order: u16,
    sign: GlueSign,
    set: f64,
    cur_g: Scaled,
    cur_glue: f64,
}

impl GlueRounding {
    fn new(mem: &MemState, this_box: AnyBox) -> GlueRounding {
        GlueRounding {
            order: this_box.glue_order(mem),
            sign: this_box.glue_sign(mem),
            set: this_box.glue_set(mem),
            cur_g: Scaled::ZERO,
            cur_glue: 0.0,
        }
    }

    /// Whether the box stretches or shrinks `spec`'s component.
    fn is_set(&self, mem: &MemState, spec: NodePtr<GlueSpecNode>) -> bool {
        match self.sign {
            GlueSign::Stretching => spec.stretch_order(mem) == self.order,
            GlueSign::Shrinking => spec.shrink_order(mem) == self.order,
            GlueSign::Normal => false,
        }
    }

    /// The rounded width of `spec` as set in this box.
    fn width(&mut self, mem: &MemState, spec: NodePtr<GlueSpecNode>) -> Scaled {
        let width = spec.width(mem) - self.cur_g;
        if self.is_set(mem, spec) {
            if self.sign == GlueSign::Stretching {
                self.cur_glue += f64::from(spec.stretch(mem).0);
            } else {
                self.cur_glue -= f64::from(spec.shrink(mem).0);
            }
            let glue_temp = (self.set * self.cur_glue).clamp(-1_000_000_000.0, 1_000_000_000.0);
            self.cur_g = Scaled(crate::scaled_math::tex_round(glue_temp));
        }
        width + self.cur_g
    }
}

enum Leader {
    Rule(NodePtr<RuleNode>),
    Box(AnyBox),
}

impl FontState {
    fn font_character_width(&self, f: FontId, c: i32) -> Scaled {
        let info = self.char_info(f, c);
        self.char_width(f, info)
    }
}

impl ShipoutState {
    fn push_dvi_level(&mut self) -> Result<i32, Abort> {
        self.cur_s += 1;
        if self.cur_s > 0 {
            self.dvi_out(PUSH)?;
        }
        self.max_push = self.max_push.max(self.cur_s);
        Ok(self.dvi_offset + self.dvi_ptr)
    }
}

impl MemState {
    fn is_space_adjustment(&self, q: MemPtr) -> bool {
        matches!(self.classify_non_char(q), Some(AnyNode::Kern(k)) if k.subtype(self) == SPACE_ADJUSTMENT)
    }

    /// Merge runs of native words separated by interword glue into single nodes
    /// so they can be justified as a whole.
    fn merge_justifiable_native_words(
        &mut self,
        strings: &mut StringsState,
        fonts: &FontState,
        eqtb: &EqtbState,
        this_box: AnyBox,
    ) -> Result<(), Abort> {
        let mut prev_p = this_box.list_head();
        let mut p = this_box.list_ptr(self);

        while !p.is_null() {
            let first = self.as_native_word(p).filter(|r| {
                fonts.font_letter_space[r.native_font(self)] == Scaled::ZERO
                    && !p.link(self).is_null()
            });
            if let Some(r) = first {
                let font = r.native_font(self);
                let same_font_word = |mem: &MemState, q: MemPtr| {
                    mem.as_native_word(q).filter(|w| w.native_font(mem) == font)
                };
                let mut last = r;
                let mut text_len = i32::from(r.native_length(self));
                let mut q = r.link(self);
                loop {
                    q = self.skip_interword_fillers(q);
                    match self.classify_non_char(q) {
                        Some(AnyNode::Glue(glue)) if glue.subtype(self) == NORMAL => {
                            if glue.glue_ptr(self).ptr().0 == fonts.font_glue[font] {
                                q = self.skip_interword_fillers(glue.link(self));
                                if let Some(w) = same_font_word(self, q) {
                                    last = w;
                                    text_len += 1 + i32::from(w.native_length(self));
                                    q = w.link(self);
                                    continue;
                                }
                            } else {
                                q = glue.link(self);
                            }
                            if self.is_space_adjustment(q) {
                                q = self.skip_interword_fillers(q.link(self));
                                if let Some(w) = same_font_word(self, q) {
                                    last = w;
                                    text_len += 1 + i32::from(w.native_length(self));
                                    q = w.link(self);
                                    continue;
                                }
                            }
                        }
                        Some(AnyNode::Whatsit(_)) => {
                            if let Some(w) = same_font_word(self, q) {
                                last = w;
                                q = w.link(self);
                                continue;
                            }
                        }
                        _ => {}
                    }
                    break;
                }

                if last != r {
                    p = self.merge_native_word_run(
                        strings, fonts, eqtb, this_box, prev_p, r, last, text_len,
                    )?;
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
        this_box: AnyBox,
        prev_p: MemPtr,
        first: NodePtr<WhatsitNode>,
        last: NodePtr<WhatsitNode>,
        text_len: i32,
    ) -> Result<MemPtr, Abort> {
        if strings.pool_ptr + text_len > strings.pool_size {
            return Err(Abort::PoolOverflow);
        }

        let g_order = this_box.glue_order(self);
        let g_sign = this_box.glue_sign(self);
        let mut width = Scaled::ZERO;
        let mut q = first.ptr();
        loop {
            match self.classify(MemIdx::new(q.0)) {
                Ok(AnyNode::Whatsit(w)) => {
                    if w.whatsit_subtype(self).is_some_and(|s| s.is_native_word()) {
                        for &unit in w.native_text(self) {
                            strings.str_pool[strings.pool_ptr as usize] = unit;
                            strings.pool_ptr += 1;
                        }
                        width += w.width(self);
                    }
                }
                Ok(AnyNode::Glue(glue)) => {
                    strings.str_pool[strings.pool_ptr as usize] = u16::from(b' ');
                    strings.pool_ptr += 1;
                    let spec = glue.glue_ptr(self);
                    width += spec.width(self);
                    match g_sign {
                        GlueSign::Stretching if spec.stretch_order(self) == g_order => {
                            width += Scaled(crate::scaled_math::tex_round(
                                this_box.glue_set(self) * f64::from(spec.stretch(self).0),
                            ));
                        }
                        GlueSign::Shrinking if spec.shrink_order(self) == g_order => {
                            width -= Scaled(crate::scaled_math::tex_round(
                                this_box.glue_set(self) * f64::from(spec.shrink(self).0),
                            ));
                        }
                        _ => {}
                    }
                }
                Ok(AnyNode::Kern(kern)) => width += kern.width(self),
                _ => {}
            }
            if q == last.ptr() {
                break;
            }
            q = q.link(self);
        }

        let nlen = strings.cur_length();
        let word = self.new_native_word_node(eqtb, first.native_font(self), nlen)?;
        word.set_subtype(self, first.subtype(self));
        let base = strings.cur_string_start() as usize;
        word.native_text_mut(self)
            .copy_from_slice(&strings.str_pool[base..base + nlen as usize]);
        word.set_width(self, width);
        fonts.store_justified_native_glyphs(self, word)?;

        prev_p.set_link(self, word.ptr());
        word.set_link(self, last.link(self));
        last.set_link(self, MemPtr::NULL);

        let mut tail = word.ptr();
        let mut prev_p = first.ptr();
        let mut p = first.link(self);
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
        if glue.subtype(self) < A_LEADERS {
            glue.into_kern(self, w);
        } else {
            let rigid = NodePtr::<GlueSpecNode>::alloc(self)?;
            rigid.set_stretch_order(self, FILLL + 1);
            rigid.set_shrink_order(self, FILLL + 1);
            rigid.set_width(self, w);
            rigid.set_stretch(self, Scaled::ZERO);
            rigid.set_shrink(self, Scaled::ZERO);
            glue.set_glue_ptr(self, rigid);
        }
        Ok(())
    }

    fn leader_of(&self, glue: NodePtr<GlueNode>) -> Leader {
        match self.classify(MemIdx::new(glue.leader_ptr(self).0)) {
            Ok(AnyNode::Rule(rule)) => Leader::Rule(rule),
            Ok(AnyNode::HList(h)) => Leader::Box(AnyBox::HList(h)),
            Ok(AnyNode::VList(v)) => Leader::Box(AnyBox::VList(v)),
            other => unreachable!("leaders must be a rule or a box, found {other:?}"),
        }
    }

    fn lr_dir(&self, math: NodePtr<MathNode>) -> i16 {
        (math.subtype(self) / R_CODE) as i16
    }

    /// The LR stack top matches the end code of `math`.
    fn lr_matches(&self, scanner: &ScannerState, math: NodePtr<MathNode>) -> bool {
        scanner.lr_ptr.info(self) == end_lr_type(math.subtype(self))
    }

    fn pop_dvi_level(&mut self, shipout: &mut ShipoutState, save_loc: i32) -> Result<(), Abort> {
        self.prune_movements(shipout, save_loc);
        if shipout.cur_s > 0 {
            shipout.dvi_pop(save_loc)?;
        }
        shipout.cur_s -= 1;
        Ok(())
    }
}

fn ship_box(st: &mut crate::state::EngineState, b: AnyBox) -> Result<(), Abort> {
    match b {
        AnyBox::HList(_) => hlist_out(st, b),
        AnyBox::VList(_) => vlist_out(st, b),
    }
}

/// Ship one copy of a leader box with `doing_leaders` set.
fn ship_leader_box(st: &mut crate::state::EngineState, leader: AnyBox) -> Result<(), Abort> {
    let outer_doing_leaders = st.shipout.doing_leaders;
    st.shipout.doing_leaders = true;
    ship_box(st, leader)?;
    st.shipout.doing_leaders = outer_doing_leaders;
    Ok(())
}

fn hlist_rule_out(
    st: &mut crate::state::EngineState,
    this_box: AnyBox,
    base_line: Scaled,
    p: MemPtr,
) -> Result<(), Abort> {
    if st.shipout.rule_ht == RUNNING {
        st.shipout.rule_ht = this_box.height(&st.mem);
    }
    if st.shipout.rule_dp == RUNNING {
        st.shipout.rule_dp = this_box.depth(&st.mem);
    }
    st.shipout.rule_ht += st.shipout.rule_dp;
    if st.shipout.rule_ht > Scaled::ZERO && st.shipout.rule_wd > Scaled::ZERO {
        synch_h(st)?;
        st.shipout.cur_v = base_line + st.shipout.rule_dp;
        synch_v(st)?;
        st.shipout.dvi_rule(SET_RULE)?;
        st.shipout.cur_v = base_line;
        st.shipout.dvi_h += st.shipout.rule_wd;
    }
    st.shipout.cur_h += st.shipout.rule_wd;
    crate::synctex::synctex_horizontal_rule_or_glue(st, p);
    Ok(())
}

fn vlist_rule_out(
    st: &mut crate::state::EngineState,
    this_box: AnyBox,
    left_edge: Scaled,
    upwards: bool,
) -> Result<(), Abort> {
    if st.shipout.rule_wd == RUNNING {
        st.shipout.rule_wd = this_box.width(&st.mem);
    }
    st.shipout.rule_ht += st.shipout.rule_dp;
    st.shipout.move_past_vlist_rule(upwards);
    if st.shipout.rule_ht > Scaled::ZERO && st.shipout.rule_wd > Scaled::ZERO {
        if st.scanner.cur_dir == RIGHT_TO_LEFT {
            st.shipout.cur_h -= st.shipout.rule_wd;
        }
        synch_h(st)?;
        synch_v(st)?;
        st.shipout.dvi_rule(PUT_RULE)?;
        st.shipout.cur_h = left_edge;
    }
    Ok(())
}

impl ShipoutState {
    fn dvi_rule(&mut self, opcode: u8) -> Result<(), Abort> {
        let (ht, wd) = (self.rule_ht, self.rule_wd);
        self.dvi_out(opcode)?;
        self.dvi_four(ht.0)?;
        self.dvi_four(wd.0)
    }

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
    this_box: AnyBox,
    base_line: Scaled,
    left_edge: Scaled,
    glue: NodePtr<GlueNode>,
    rounding: &mut GlueRounding,
) -> Result<(), Abort> {
    let spec = glue.glue_ptr(&st.mem);
    st.shipout.rule_wd = rounding.width(&st.mem, spec);
    if rounding.is_set(&st.mem, spec) {
        st.mem.freeze_set_glue(glue, spec, st.shipout.rule_wd)?;
    }

    let subtype = glue.subtype(&st.mem);
    if subtype >= A_LEADERS {
        let leader = match st.mem.leader_of(glue) {
            Leader::Rule(rule) => {
                st.shipout.rule_ht = rule.height(&st.mem).to_scaled();
                st.shipout.rule_dp = rule.depth(&st.mem).to_scaled();
                return hlist_rule_out(st, this_box, base_line, glue.ptr());
            }
            Leader::Box(leader) => leader,
        };
        let leader_wd = leader.width(&st.mem);
        if leader_wd > Scaled::ZERO && st.shipout.rule_wd > Scaled::ZERO {
            st.shipout.rule_wd += LEADER_SLOP;
            if st.scanner.cur_dir == RIGHT_TO_LEFT {
                st.shipout.cur_h -= LEADER_SLOP;
            }
            let edge = st.shipout.cur_h + st.shipout.rule_wd;
            let mut lx = Scaled::ZERO;
            if subtype == A_LEADERS {
                let save_h = st.shipout.cur_h;
                st.shipout.cur_h =
                    left_edge + leader_wd * ((st.shipout.cur_h - left_edge).0 / leader_wd.0);
                if st.shipout.cur_h < save_h {
                    st.shipout.cur_h += leader_wd;
                }
            } else {
                let (lq, lr) = leader_count_and_rest(st.shipout.rule_wd, leader_wd);
                if subtype == C_LEADERS {
                    st.shipout.cur_h += lr / 2;
                } else {
                    lx = lr / (lq + 1);
                    st.shipout.cur_h += (lr - lx * (lq - 1)) / 2;
                }
            }

            while st.shipout.cur_h + leader_wd <= edge {
                st.shipout.cur_v = base_line + leader.shift_amount(&st.mem);
                synch_v(st)?;
                let save_v = st.shipout.dvi_v;
                synch_h(st)?;
                let save_h = st.shipout.dvi_h;
                if st.scanner.cur_dir == RIGHT_TO_LEFT {
                    st.shipout.cur_h += leader_wd;
                }
                ship_leader_box(st, leader)?;
                st.shipout.dvi_v = save_v;
                st.shipout.dvi_h = save_h;
                st.shipout.cur_v = base_line;
                st.shipout.cur_h = save_h + leader_wd + lx;
            }

            st.shipout.cur_h = if st.scanner.cur_dir == RIGHT_TO_LEFT {
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

fn dvi_xdv_buffer(st: &mut crate::state::EngineState, len: usize) -> Result<(), Abort> {
    st.shipout.dvi_bytes(&st.fonts.xdv_buffer[..len])
}

fn native_word_out(
    st: &mut crate::state::EngineState,
    word: NodePtr<WhatsitNode>,
    subtype: WhatsitSubtype,
) -> Result<(), Abort> {
    synch_h(st)?;
    synch_v(st)?;
    dvi_font_select(st, word.native_font(&st.mem))?;
    match subtype {
        WhatsitSubtype::Glyph => {
            st.shipout.dvi_out(SET_GLYPHS)?;
            st.shipout.dvi_four(word.width(&st.mem).0)?;
            st.shipout.dvi_two(1)?;
            st.shipout.dvi_four(0)?;
            st.shipout.dvi_four(0)?;
            st.shipout.dvi_two(word.native_glyph(&st.mem))?;
        }
        WhatsitSubtype::NativeWordAt => {
            let len = word.native_length(&st.mem);
            if len > 0 || word.native_glyph_info_handle(&st.mem) != 0 {
                st.shipout.dvi_out(SET_TEXT_AND_GLYPHS)?;
                st.shipout.dvi_two(len)?;
                for &unit in word.native_text(&st.mem) {
                    st.shipout.dvi_two(unit)?;
                }
                let len = make_xdv_glyph_array_data(st, word);
                dvi_xdv_buffer(st, len)?;
            }
        }
        _ => {
            if word.native_glyph_info_handle(&st.mem) != 0 {
                st.shipout.dvi_out(SET_GLYPHS)?;
                let len = make_xdv_glyph_array_data(st, word);
                dvi_xdv_buffer(st, len)?;
            }
        }
    }
    st.shipout.cur_h += word.width(&st.mem);
    st.shipout.dvi_h = st.shipout.cur_h;
    Ok(())
}

impl ShipoutState {
    fn pdf_save_pos(&mut self) {
        self.pdf_last_x_pos = (self.cur_h + ONE_INCH_LESS_ONE).0;
        self.pdf_last_y_pos = (self.cur_page_height - self.cur_v - ONE_INCH_LESS_ONE).0;
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
        let f = ch.font(&st.mem);
        let mut c = ch.character(&st.mem);
        if p != LIG_TRICK {
            if let Some(mapping) = st.fonts.font_mapping[f].as_deref() {
                c = apply_tfm_font_mapping(mapping, c as libc::c_int) as u16;
            }
        }
        dvi_font_select(st, f)?;
        if st.fonts.font_ec[f] >= c
            && st.fonts.font_bc[f] <= c
            && st
                .fonts
                .font_info_b16(st.fonts.char_base[f] + i32::from(c))
                .s3
                > 0
        {
            if c >= 128 {
                st.shipout.dvi_out(SET1)?;
            }
            st.shipout.dvi_out(c as u8)?;
            st.shipout.cur_h += st.fonts.font_character_width(f, i32::from(c));
        }
        prev_p = prev_p.link(&st.mem);
        p = p.link(&st.mem);
    }
    crate::synctex::synctex_current(st);
    st.shipout.dvi_h = st.shipout.cur_h;
    Ok((p, prev_p))
}

fn hlist_out(st: &mut crate::state::EngineState, this_box: AnyBox) -> Result<(), Abort> {
    st.mem.temp_ptr = this_box.ptr();
    let mut rounding = GlueRounding::new(&st.mem, this_box);
    if st.eqtb.int_par(INT_PAR_XETEX_INTERWORD_SPACE_SHAPING) > 1 {
        st.mem
            .merge_justifiable_native_words(&mut st.strings, &st.fonts, &st.eqtb, this_box)?;
    }

    // A raw cursor: it can be LIG_TRICK, and nodes get retyped in place.
    let mut p = this_box.list_ptr(&st.mem);
    let save_loc = st.shipout.push_dvi_level()?;
    let base_line = st.shipout.cur_v;
    let mut prev_p = this_box.list_head();
    st.mem.push_lr(&mut st.scanner.lr_ptr, BEFORE.into())?;

    if this_box.subtype(&st.mem) == DLIST {
        if st.scanner.cur_dir == RIGHT_TO_LEFT {
            st.scanner.cur_dir = LEFT_TO_RIGHT;
            st.shipout.cur_h -= this_box.width(&st.mem);
        } else {
            this_box.set_subtype(&mut st.mem, 0);
        }
    }

    if st.scanner.cur_dir == RIGHT_TO_LEFT && this_box.subtype(&st.mem) != REVERSED {
        let save_h = st.shipout.cur_h;
        st.mem.temp_ptr = p;
        let kern = st.mem.new_kern(Scaled::ZERO)?;
        kern.set_sync_tag(&mut st.mem, 0);
        prev_p.set_link(&mut st.mem, kern.ptr());
        st.shipout.cur_h = Scaled::ZERO;
        let reversed = st.mem.reverse(
            &mut st.shipout,
            &mut st.scanner,
            &mut st.fonts,
            None,
            &mut rounding,
        )?;
        kern.set_link(&mut st.mem, reversed);
        kern.set_width(&mut st.mem, -st.shipout.cur_h);
        st.shipout.cur_h = save_h;
        this_box.set_subtype(&mut st.mem, REVERSED);
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
            .classify(MemIdx::new(p.0))
            .expect("valid node type in hlist_out")
        {
            AnyNode::HList(h) => hlist_box_out(st, AnyBox::HList(h), base_line)?,
            AnyNode::VList(v) => hlist_box_out(st, AnyBox::VList(v), base_line)?,
            AnyNode::Rule(rule) => {
                st.shipout.rule_ht = rule.height(&st.mem).to_scaled();
                st.shipout.rule_dp = rule.depth(&st.mem).to_scaled();
                st.shipout.rule_wd = rule.width(&st.mem).to_scaled();
                hlist_rule_out(st, this_box, base_line, rule.ptr())?;
            }
            AnyNode::Whatsit(whatsit) => match whatsit.whatsit_subtype(&st.mem) {
                Some(
                    subtype @ (WhatsitSubtype::NativeWord
                    | WhatsitSubtype::NativeWordAt
                    | WhatsitSubtype::Glyph),
                ) => native_word_out(st, whatsit, subtype)?,
                Some(WhatsitSubtype::Pic | WhatsitSubtype::Pdf) => {
                    let save_h = st.shipout.dvi_h;
                    let save_v = st.shipout.dvi_v;
                    st.shipout.cur_v = base_line;
                    let edge = st.shipout.cur_h + whatsit.width(&st.mem);
                    pic_out(st, whatsit)?;
                    st.shipout.dvi_h = save_h;
                    st.shipout.dvi_v = save_v;
                    st.shipout.cur_h = edge;
                    st.shipout.cur_v = base_line;
                }
                Some(WhatsitSubtype::PdfSavePos) => st.shipout.pdf_save_pos(),
                _ => out_what(st, whatsit)?,
            },
            AnyNode::Glue(glue) => {
                hlist_glue_out(st, this_box, base_line, left_edge, glue, &mut rounding)?
            }
            AnyNode::MarginKern(kern) => st.shipout.cur_h += kern.width(&st.mem),
            AnyNode::Kern(kern) => {
                crate::synctex::synctex_kern(st, kern, this_box);
                st.shipout.cur_h += kern.width(&st.mem);
            }
            AnyNode::Math(math) => {
                crate::synctex::synctex_math(st, math);
                let math_width = math.width(&st.mem);
                if !math.subtype(&st.mem).is_multiple_of(2) {
                    if st.mem.lr_matches(&st.scanner, math) {
                        st.mem.pop_lr(&mut st.scanner.lr_ptr);
                    } else if math.subtype(&st.mem) > L_CODE {
                        st.scanner.lr_problems += 1;
                    }
                } else {
                    let end_type = end_lr_type(math.subtype(&st.mem));
                    st.mem.push_lr(&mut st.scanner.lr_ptr, end_type)?;
                    if st.mem.lr_dir(math) != st.scanner.cur_dir {
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
                st.mem
                    .set_word(LIG_TRICK, st.mem.word(ligature.idx().offset(1).0));
                let next = ligature.link(&st.mem);
                p = LIG_TRICK;
                p.set_link(&mut st.mem, next);
                st.fonts.xtx_ligature_present = true;
                continue;
            }
            AnyNode::Style(style) => {
                let edge = style.as_edge();
                st.shipout.cur_h += edge.width(&st.mem);
                left_edge = st.shipout.cur_h + edge.edge_dist(&st.mem);
                st.scanner.cur_dir = edge.subtype(&st.mem) as i16;
            }
            _ => {}
        }
        prev_p = p;
        p = p.link(&st.mem);
    }

    crate::synctex::synctex_tsilh(st, this_box);

    while st.scanner.lr_ptr.info(&st.mem) != BEFORE.into() {
        if st.scanner.lr_ptr.info(&st.mem) as u16 > L_CODE {
            st.scanner.lr_problems += 10000;
        }
        st.mem.pop_lr(&mut st.scanner.lr_ptr);
    }
    st.mem.pop_lr(&mut st.scanner.lr_ptr);
    if this_box.subtype(&st.mem) == DLIST {
        st.scanner.cur_dir = RIGHT_TO_LEFT;
    }
    st.mem.pop_dvi_level(&mut st.shipout, save_loc)
}

fn hlist_box_out(
    st: &mut crate::state::EngineState,
    b: AnyBox,
    base_line: Scaled,
) -> Result<(), Abort> {
    if b.list_ptr(&st.mem).is_null() {
        match b {
            AnyBox::VList(v) => crate::synctex::synctex_void_vlist(st, v),
            AnyBox::HList(h) => crate::synctex::synctex_void_hlist(st, h),
        }
        st.shipout.cur_h += b.width(&st.mem);
        return Ok(());
    }
    let save_h = st.shipout.dvi_h;
    let save_v = st.shipout.dvi_v;
    st.shipout.cur_v = base_line + b.shift_amount(&st.mem);
    let edge = st.shipout.cur_h + b.width(&st.mem);
    if st.scanner.cur_dir == RIGHT_TO_LEFT {
        st.shipout.cur_h = edge;
    }
    ship_box(st, b)?;
    st.shipout.dvi_h = save_h;
    st.shipout.dvi_v = save_v;
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
    st.mem.temp_ptr = math.link(&st.mem);
    st.shipout.rule_wd = math_width;
    st.mem.free_node(math.ptr(), MEDIUM_NODE_SIZE);
    st.scanner.cur_dir = 1 - st.scanner.cur_dir;
    let edge = st.mem.new_edge(st.scanner.cur_dir, st.shipout.rule_wd)?;
    prev_p.set_link(&mut st.mem, edge.ptr());
    st.shipout.cur_h = st.shipout.cur_h - left_edge + st.shipout.rule_wd;
    let end = st.mem.new_edge(1 - st.scanner.cur_dir, Scaled::ZERO)?;
    let reversed = st.mem.reverse(
        &mut st.shipout,
        &mut st.scanner,
        &mut st.fonts,
        Some(end),
        rounding,
    )?;
    edge.set_link(&mut st.mem, reversed);
    edge.set_edge_dist(&mut st.mem, st.shipout.cur_h);
    st.scanner.cur_dir = 1 - st.scanner.cur_dir;
    st.shipout.cur_h = save_h;
    Ok(edge)
}

fn vlist_out(st: &mut crate::state::EngineState, this_box: AnyBox) -> Result<(), Abort> {
    st.mem.temp_ptr = this_box.ptr();
    let mut rounding = GlueRounding::new(&st.mem, this_box);
    let mut p = this_box.list_ptr(&st.mem);
    let upwards = this_box.subtype(&st.mem) == 1;
    let save_loc = st.shipout.push_dvi_level()?;
    let left_edge = st.shipout.cur_h;
    crate::synctex::synctex_vlist(st, this_box);
    if upwards {
        st.shipout.cur_v += this_box.depth(&st.mem);
    } else {
        st.shipout.cur_v -= this_box.height(&st.mem);
    }
    let top_edge = st.shipout.cur_v;

    while !p.is_null() {
        if st.mem.is_char_node(p) {
            return Err(Abort::Confusion("vlistout"));
        }
        match st
            .mem
            .classify(MemIdx::new(p.0))
            .expect("valid node type in vlist_out")
        {
            AnyNode::HList(h) => vlist_box_out(st, AnyBox::HList(h), left_edge, upwards)?,
            AnyNode::VList(v) => vlist_box_out(st, AnyBox::VList(v), left_edge, upwards)?,
            AnyNode::Rule(rule) => {
                st.shipout.rule_ht = rule.height(&st.mem).to_scaled();
                st.shipout.rule_dp = rule.depth(&st.mem).to_scaled();
                st.shipout.rule_wd = rule.width(&st.mem).to_scaled();
                vlist_rule_out(st, this_box, left_edge, upwards)?;
            }
            AnyNode::Whatsit(whatsit) => match whatsit.whatsit_subtype(&st.mem) {
                Some(WhatsitSubtype::Glyph) => {
                    st.shipout.cur_v += whatsit.height(&st.mem);
                    st.shipout.cur_h = left_edge;
                    synch_h(st)?;
                    synch_v(st)?;
                    dvi_font_select(st, whatsit.native_font(&st.mem))?;
                    st.shipout.dvi_out(SET_GLYPHS)?;
                    st.shipout.dvi_four(0)?;
                    st.shipout.dvi_two(1)?;
                    st.shipout.dvi_four(0)?;
                    st.shipout.dvi_four(0)?;
                    st.shipout.dvi_two(whatsit.native_glyph(&st.mem))?;
                    st.shipout.cur_v += whatsit.depth(&st.mem);
                    st.shipout.cur_h = left_edge;
                }
                Some(WhatsitSubtype::Pic | WhatsitSubtype::Pdf) => {
                    let save_h = st.shipout.dvi_h;
                    let save_v = st.shipout.dvi_v;
                    st.shipout.cur_v += whatsit.height(&st.mem);
                    pic_out(st, whatsit)?;
                    st.shipout.dvi_h = save_h;
                    st.shipout.dvi_v = save_v;
                    st.shipout.cur_v = save_v + whatsit.depth(&st.mem);
                    st.shipout.cur_h = left_edge;
                }
                Some(WhatsitSubtype::PdfSavePos) => st.shipout.pdf_save_pos(),
                _ => out_what(st, whatsit)?,
            },
            AnyNode::Glue(glue) => {
                let spec = glue.glue_ptr(&st.mem);
                st.shipout.rule_ht = rounding.width(&st.mem, spec);
                let subtype = glue.subtype(&st.mem);
                if subtype < A_LEADERS {
                    st.shipout.move_past_vlist_rule(upwards);
                } else {
                    match st.mem.leader_of(glue) {
                        Leader::Rule(rule) => {
                            st.shipout.rule_wd = rule.width(&st.mem).to_scaled();
                            st.shipout.rule_dp = Scaled::ZERO;
                            vlist_rule_out(st, this_box, left_edge, upwards)?;
                        }
                        Leader::Box(leader) => {
                            vlist_leaders_out(st, leader, subtype, left_edge, top_edge, upwards)?
                        }
                    }
                }
            }
            AnyNode::Kern(kern) => {
                if upwards {
                    st.shipout.cur_v -= kern.width(&st.mem);
                } else {
                    st.shipout.cur_v += kern.width(&st.mem);
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
    b: AnyBox,
    left_edge: Scaled,
    upwards: bool,
) -> Result<(), Abort> {
    let (height, depth) = (b.height(&st.mem), b.depth(&st.mem));
    if upwards {
        st.shipout.cur_v -= depth;
    } else {
        st.shipout.cur_v += height;
    }
    if b.list_ptr(&st.mem).is_null() {
        match b {
            AnyBox::VList(v) => crate::synctex::synctex_void_vlist(st, v),
            AnyBox::HList(h) => crate::synctex::synctex_void_hlist(st, h),
        }
        if upwards {
            st.shipout.cur_v -= height;
        } else {
            st.shipout.cur_v += depth;
        }
        return Ok(());
    }
    synch_v(st)?;
    let save_h = st.shipout.dvi_h;
    let save_v = st.shipout.dvi_v;
    let shift = b.shift_amount(&st.mem);
    st.shipout.cur_h = if st.scanner.cur_dir == RIGHT_TO_LEFT {
        left_edge - shift
    } else {
        left_edge + shift
    };
    ship_box(st, b)?;
    st.shipout.dvi_h = save_h;
    st.shipout.dvi_v = save_v;
    st.shipout.cur_v = if upwards {
        save_v - b.height(&st.mem)
    } else {
        save_v + b.depth(&st.mem)
    };
    st.shipout.cur_h = left_edge;
    Ok(())
}

fn vlist_leaders_out(
    st: &mut crate::state::EngineState,
    leader: AnyBox,
    subtype: u16,
    left_edge: Scaled,
    top_edge: Scaled,
    upwards: bool,
) -> Result<(), Abort> {
    let leader_ht = leader.height(&st.mem) + leader.depth(&st.mem);
    if leader_ht <= Scaled::ZERO || st.shipout.rule_ht <= Scaled::ZERO {
        st.shipout.move_past_vlist_rule(upwards);
        return Ok(());
    }
    st.shipout.rule_ht += LEADER_SLOP;
    let edge = st.shipout.cur_v + st.shipout.rule_ht;
    let mut lx = Scaled::ZERO;
    if subtype == A_LEADERS {
        let save_v = st.shipout.cur_v;
        st.shipout.cur_v = top_edge + leader_ht * ((st.shipout.cur_v - top_edge).0 / leader_ht.0);
        if st.shipout.cur_v < save_v {
            st.shipout.cur_v += leader_ht;
        }
    } else {
        let (lq, lr) = leader_count_and_rest(st.shipout.rule_ht, leader_ht);
        if subtype == C_LEADERS {
            st.shipout.cur_v += lr / 2;
        } else {
            lx = lr / (lq + 1);
            st.shipout.cur_v += (lr - lx * (lq - 1)) / 2;
        }
    }

    while st.shipout.cur_v + leader_ht <= edge {
        let shift = leader.shift_amount(&st.mem);
        st.shipout.cur_h = if st.scanner.cur_dir == RIGHT_TO_LEFT {
            left_edge - shift
        } else {
            left_edge + shift
        };
        synch_h(st)?;
        let save_h = st.shipout.dvi_h;
        st.shipout.cur_v += leader.height(&st.mem);
        synch_v(st)?;
        let save_v = st.shipout.dvi_v;
        ship_leader_box(st, leader)?;
        st.shipout.dvi_v = save_v;
        st.shipout.dvi_h = save_h;
        st.shipout.cur_h = left_edge;
        st.shipout.cur_v = save_v - leader.height(&st.mem) + leader_ht + lx;
    }
    st.shipout.cur_v = edge - LEADER_SLOP;
    Ok(())
}

impl MemState {
    fn prune_movements(&mut self, shipout: &mut ShipoutState, l: i32) {
        self.prune_movement_list(&mut shipout.down_ptr, l);
        self.prune_movement_list(&mut shipout.right_ptr, l);
    }

    fn prune_movement_list(&mut self, head: &mut MemPtr, l: i32) {
        while let Some(p) = NodePtr::<MovementNode>::from_ptr(*head) {
            if p.location(self) < l {
                return;
            }
            *head = p.link(self);
            p.free(self);
        }
    }
}

pub(crate) fn finalize_dvi_file(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    while st.shipout.cur_s > -1 {
        if st.shipout.cur_s > 0 {
            st.shipout.dvi_out(POP)?;
        } else {
            st.shipout.dvi_out(EOP)?;
            st.shipout
                .page_manifest
                .finish_page(st.scanner.logical_clock);
            st.shipout.total_pages += 1;
        }
        st.shipout.cur_s -= 1;
    }

    if st.shipout.total_pages == 0 {
        crate::output::print_nl(st, "No pages of output.");
        return Ok(());
    }
    // The DVI grew too big and the message was already printed.
    if st.shipout.cur_s == -2 {
        return Ok(());
    }

    st.shipout.dvi_out(POST)?;
    let last_bop = st.shipout.last_bop;
    st.shipout.dvi_four(last_bop)?;
    st.shipout.last_bop = st.shipout.dvi_offset + st.shipout.dvi_ptr - 5;
    st.shipout.dvi_four(25_400_000)?;
    st.shipout.dvi_four(473_628_672)?;
    crate::save::prepare_mag(st)?;
    st.shipout.dvi_four(st.eqtb.int_par(INT_PAR_MAG))?;
    let (max_v, max_h, max_push, total_pages) = (
        st.shipout.max_v,
        st.shipout.max_h,
        st.shipout.max_push,
        st.shipout.total_pages,
    );
    st.shipout.dvi_four(max_v.0)?;
    st.shipout.dvi_four(max_h.0)?;
    st.shipout.dvi_bytes(&[
        (max_push / 256) as u8,
        (max_push % 256) as u8,
        ((total_pages / 256) % 256) as u8,
        (total_pages % 256) as u8,
    ])?;

    for f in st.fonts.font_ptr.loaded().rev() {
        if st.fonts.font_used[f] {
            dvi_font_def(st, f)?;
        }
    }
    st.fonts.font_ptr = NULL_FONT;

    st.shipout.dvi_out(POST_POST)?;
    let last_bop = st.shipout.last_bop;
    st.shipout.dvi_four(last_bop)?;
    st.shipout
        .dvi_out(id_byte(st.config.semantic_pagination_enabled))?;
    let padding = 4 + (DVI_BUF_SIZE - st.shipout.dvi_ptr) % 4;
    for _ in 0..padding {
        st.shipout.dvi_out(223)?;
    }

    if st.shipout.dvi_limit == HALF_BUF {
        st.shipout.write_to_dvi(HALF_BUF, DVI_BUF_SIZE - 1)?;
    }
    if st.shipout.dvi_ptr > TEX_INFINITY - st.shipout.dvi_offset {
        st.shipout.cur_s = -2;
        return Err(Abort::Fatal("dvi length exceeds 0x7FFFFFFF"));
    }
    if st.shipout.dvi_ptr > 0 {
        st.shipout.write_to_dvi(0, st.shipout.dvi_ptr - 1)?;
    }

    let k = output_close(st.shipout.dvi_file) as u8;
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
        crate::output::print_int(st, st.shipout.dvi_offset + st.shipout.dvi_ptr);
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

/// Fill `xdv_buffer` with the glyph array of `word`, returning its length.
fn make_xdv_glyph_array_data(
    st: &mut crate::state::EngineState,
    word: NodePtr<WhatsitNode>,
) -> usize {
    let glyph_count = word.native_glyph_count(&st.mem) as usize;
    let needed = glyph_count * NATIVE_GLYPH_INFO_SIZE + 8;
    if st.fonts.xdv_buffer.len() < needed {
        st.fonts.xdv_buffer.resize(needed, 0);
    }
    let width = word.width(&st.mem).0;
    let glyphs = st
        .mem
        .glyph_blobs
        .get(word.native_glyph_info_handle(&st.mem));
    let buf = &mut st.fonts.xdv_buffer;
    let mut len = 0;
    let mut put = |bytes: &[u8]| {
        buf[len..len + bytes.len()].copy_from_slice(bytes);
        len += bytes.len();
    };
    put(&width.to_be_bytes());
    put(&(glyph_count as u16).to_be_bytes());
    for loc in &glyphs.locations[..glyph_count] {
        put(&loc.x.0.to_be_bytes());
        put(&loc.y.0.to_be_bytes());
    }
    for gid in &glyphs.gids[..glyph_count] {
        put(&gid.to_be_bytes());
    }
    len
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
                        let f = ch.font(self);
                        let c = fonts.effective_char(f, ch.character(self));
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
                match self.classify(MemIdx::new(p.0)) {
                    Ok(node @ (AnyNode::HList(_) | AnyNode::VList(_) | AnyNode::Rule(_))) => {
                        shipout.rule_wd = node.hlist_width(self);
                    }
                    Ok(AnyNode::Kern(kern)) => {
                        shipout.rule_wd = kern.width(self);
                        now_kern = true;
                    }
                    Ok(AnyNode::Whatsit(w)) => {
                        if w.whatsit_subtype(self).is_some_and(|s| s.has_dimensions()) {
                            shipout.rule_wd = w.width(self);
                        } else {
                            add_width = false;
                        }
                    }
                    Ok(AnyNode::Glue(glue)) => {
                        let spec = glue.glue_ptr(self);
                        shipout.rule_wd = rounding.width(self, spec);
                        if rounding.is_set(self, spec) {
                            self.freeze_set_glue(glue, spec, shipout.rule_wd)?;
                            now_kern = glue.subtype(self) < A_LEADERS;
                        }
                    }
                    Ok(AnyNode::Ligature(ligature)) => {
                        self.flush_list(ligature.lig_ptr(self));
                        self.temp_ptr = ligature.ptr();
                        let c = self.get_avail()?;
                        self.set_word(c.0, self.word(ligature.idx().offset(1).0));
                        c.set_link(self, q);
                        self.free_node(ligature.ptr(), SMALL_NODE_SIZE);
                        p = c;
                        continue;
                    }
                    Ok(AnyNode::Math(math)) => {
                        shipout.rule_wd = math.width(self);
                        if !math.subtype(self).is_multiple_of(2) {
                            if !self.lr_matches(scanner, math) {
                                math.into_kern(self);
                                now_kern = true;
                                scanner.lr_problems += 1;
                            } else {
                                self.pop_lr(&mut scanner.lr_ptr);
                                if n > MIN_HALFWORD {
                                    n -= 1;
                                    math.set_subtype(self, math.subtype(self) - 1);
                                } else {
                                    math.into_kern(self);
                                    now_kern = true;
                                    if m > MIN_HALFWORD {
                                        m -= 1;
                                    } else {
                                        self.free_node(math.ptr(), MEDIUM_NODE_SIZE);
                                        let t = t.expect("a segment reversal ends at its edge");
                                        t.set_link(self, q);
                                        t.set_width(self, shipout.rule_wd);
                                        t.set_edge_dist(self, -shipout.cur_h - shipout.rule_wd);
                                        return Ok(l);
                                    }
                                }
                            }
                        } else {
                            let end_type = end_lr_type(math.subtype(self));
                            self.push_lr(&mut scanner.lr_ptr, end_type)?;
                            if n > MIN_HALFWORD || self.lr_dir(math) != scanner.cur_dir {
                                n += 1;
                                math.set_subtype(self, math.subtype(self) + 1);
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
            let dir = scanner.lr_ptr.info(self) as u16;
            p = self.new_math(Scaled::ZERO, dir)?.ptr();
            scanner.lr_problems += 10000;
        }
    }

    fn movement(&mut self, shipout: &mut ShipoutState, w: Scaled, o: u8) -> Result<(), Abort> {
        let mut q = NodePtr::<MovementNode>::alloc(self)?;
        q.set_distance(self, w);
        q.set_location(self, shipout.dvi_offset + shipout.dvi_ptr);
        let head = if o == DOWN1 {
            &mut shipout.down_ptr
        } else {
            &mut shipout.right_ptr
        };
        q.set_link(self, *head);
        *head = q.ptr();

        let mut p = NodePtr::<MovementNode>::from_ptr(q.link(self));
        let mut mstate = MOV_NONE_SEEN;
        let found = 'search: {
            while let Some(node) = p {
                p = NodePtr::<MovementNode>::from_ptr(node.link(self));
                if node.distance(self) != w {
                    match mstate + node.state(self) {
                        1 => mstate = MOV_Y_SEEN,
                        2 => mstate = MOV_Z_SEEN,
                        8 | 13 => break 'search None,
                        _ => {}
                    }
                    continue;
                }
                let (delta, here) = match mstate + node.state(self) {
                    // NONE_SEEN or Z_SEEN meeting YZ_OK or Y_OK
                    3 | 4 | 15 | 16 => (5, MOV_Y_HERE),
                    // NONE_SEEN meeting Z_OK, Y_SEEN meeting YZ_OK or Z_OK
                    5 | 9 | 11 => (10, MOV_Z_HERE),
                    // a y or z that is already here
                    1 | 2 | 8 | 13 => break 'search Some(node),
                    _ => continue,
                };
                let location = node.location(self);
                if location < shipout.dvi_gone {
                    break 'search None;
                }
                let mut k = location - shipout.dvi_offset;
                if k < 0 {
                    k += DVI_BUF_SIZE;
                }
                shipout.dvi_buf[k as usize] += delta;
                shipout.page_manifest.patch_byte(location as u64, delta);
                node.set_state(self, here);
                break 'search Some(node);
            }
            None
        };

        let Some(found) = found else {
            q.set_state(self, MOV_YZ_OK);
            let w = w.0;
            let len = if w.abs() >= 0x800000 {
                4
            } else if w.abs() >= 0x8000 {
                3
            } else if w.abs() >= 128 {
                2
            } else {
                1
            };
            shipout.dvi_out(o + len as u8 - 1)?;
            return shipout.dvi_bytes(&w.to_be_bytes()[4 - len..]);
        };

        q.set_state(self, found.state(self));
        let (opcode, yz_becomes, fixed_from) = if q.state(self) == MOV_Y_HERE {
            (o + 4, MOV_Z_OK, MOV_Y_OK)
        } else {
            (o + 9, MOV_Y_OK, MOV_Z_OK)
        };
        while q.link(self) != found.ptr() {
            q = NodePtr::<MovementNode>::from_ptr(q.link(self))
                .expect("found movement record is linked from the new record");
            let state = q.state(self);
            if state == MOV_YZ_OK {
                q.set_state(self, yz_becomes);
            } else if state == fixed_from {
                q.set_state(self, MOV_D_FIXED);
            }
        }
        shipout.dvi_out(opcode)
    }
}

fn special_out(
    st: &mut crate::state::EngineState,
    special: NodePtr<WhatsitNode>,
) -> Result<(), Abort> {
    synch_h(st)?;
    synch_v(st)?;
    st.shipout.doing_special = true;
    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    let tokens = special.write_tokens(&st.mem).link(&st.mem).0;
    let avail = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list(st, tokens, TEX_NULL, avail);
    st.shipout.selector = old_setting;
    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(Abort::PoolOverflow);
    }
    dvi_special_from_cur_string(st)?;
    st.shipout.doing_special = false;
    Ok(())
}

impl EqtbState {
    /// Log only, or terminal and log when `\tracingonline` is positive.
    fn log_selector(&self) -> i32 {
        if self.int_par(INT_PAR_TRACING_ONLINE) <= 0 {
            SELECTOR_LOG_ONLY
        } else {
            SELECTOR_TERM_AND_LOG
        }
    }
}

fn write_out(st: &mut crate::state::EngineState, write: NodePtr<WhatsitNode>) -> Result<(), Abort> {
    let q = st.mem.get_avail()?;
    q.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + i32::from(b'}'));
    let r = st.mem.get_avail()?;
    q.set_link(&mut st.mem, r);
    r.set_info(&mut st.mem, CS_TOKEN_FLAG + END_WRITE);
    crate::input::begin_token_list(st, q, INSERTED)?;
    crate::input::begin_token_list(st, write.write_tokens(&st.mem), WRITE_TEXT)?;
    let q = st.mem.get_avail()?;
    q.set_info(&mut st.mem, LEFT_BRACE_TOKEN + i32::from(b'{'));
    crate::input::begin_token_list(st, q, INSERTED)?;

    let old_mode = st.input.cur_list.mode;
    st.input.cur_list.mode = 0;
    st.scanner.cur_cs = st.shipout.write_loc;
    crate::scan::scan_toks(st, false, true)?;
    crate::scan::get_token(st)?;
    if st.scanner.cur_tok != CS_TOKEN_FLAG + END_WRITE {
        crate::output::error_here_with_diagnostic(st, "Unbalanced write command");
        crate::output::capture_to_diagnostic(st, None);
        st.errors.help(&[
            r"On this page there's a \write with fewer real {'s than }'s.",
            "I can't handle that very well; good luck.",
        ]);
        crate::errors::error(st)?;
        loop {
            crate::scan::get_token(st)?;
            if st.scanner.cur_tok == CS_TOKEN_FLAG + END_WRITE {
                break;
            }
        }
    }
    st.input.cur_list.mode = old_mode;
    crate::input::end_token_list(st)?;

    let old_setting = st.shipout.selector;
    let j = write.write_stream(&st.mem);
    if j == 18 {
        st.shipout.selector = SELECTOR_NEW_STRING;
    } else if st.shipout.write_open[j as usize] {
        ensure_write_stream_open(st, j as usize)?;
        st.shipout.selector = j;
    } else {
        if j == 17 && st.shipout.selector == SELECTOR_TERM_AND_LOG {
            st.shipout.selector = SELECTOR_LOG_ONLY;
        }
        crate::output::print_nl(st, "");
    }
    let def_ref = st.scanner.def_ref;
    crate::output::token_show(st, def_ref.0);
    crate::output::print_ln(st);
    st.mem.flush_list(def_ref);

    if j == 18 {
        st.shipout.selector = st.eqtb.log_selector();
        if !st.errors.log_opened {
            st.shipout.selector = SELECTOR_TERM_ONLY;
        }
        // Tectonic leaves shell-escape warnings to the driver when it is enabled.
        if !st.config.shell_escape_enabled {
            crate::output::diagnostic_begin_capture_warning_here(st);
            crate::output::print_nl(st, "runsystem(");
            let base = st.strings.cur_string_start();
            for k in base..st.strings.pool_ptr {
                crate::output::print(st, i32::from(st.strings.str_pool[k as usize]));
            }
            crate::output::print(st, ")...");
            crate::output::print(st, "disabled");
            crate::output::print_char(st, b'.');
            crate::output::capture_to_diagnostic(st, None);
            crate::output::print_nl(st, "");
            crate::output::print_ln(st);
        } else {
            let base = st.strings.cur_string_start();
            let cmd = &st.strings.str_pool[base as usize..st.strings.pool_ptr as usize];
            shell_escape(cmd);
        }
        st.strings.discard_cur_string();
    }
    st.shipout.selector = old_setting;
    Ok(())
}

fn pic_out(st: &mut crate::state::EngineState, pic: NodePtr<WhatsitNode>) -> Result<(), Abort> {
    synch_h(st)?;
    synch_v(st)?;
    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    crate::output::print(st, "pdf:image ");
    crate::output::print(st, "matrix ");
    for v in pic.pic_transform(&st.mem) {
        crate::output::print_scaled(st, v);
        crate::output::print(st, b' ');
    }
    crate::output::print(st, "page ");
    crate::output::print_int(st, i32::from(pic.pic_page(&st.mem)));
    crate::output::print(st, b' ');
    if let Some(label) =
        crate::pic::PageboxType::from_raw(pic.pic_pagebox(&st.mem)).and_then(|pb| pb.label())
    {
        crate::output::print(st, label);
    }
    crate::output::print(st, b'(');
    let path = pic.pic_path(&st.mem).to_vec();
    for byte in path {
        crate::output::print_raw_char(st, i32::from(byte), true);
    }
    crate::output::print(st, b')');
    st.shipout.selector = old_setting;
    dvi_special_from_cur_string(st)
}

/// `out_what` for the node at the tail of the current list (`\immediate`).
pub(crate) fn out_what(
    st: &mut crate::state::EngineState,
    w: NodePtr<WhatsitNode>,
) -> Result<(), Abort> {
    let subtype = w.whatsit_subtype(&st.mem);
    match subtype {
        Some(WhatsitSubtype::Open | WhatsitSubtype::Write | WhatsitSubtype::Close) => {
            if !st.shipout.doing_leaders {
                out_write_stream(st, w, subtype)?;
            }
        }
        Some(WhatsitSubtype::Special) => special_out(st, w)?,
        Some(WhatsitSubtype::Language) => {}
        _ => return Err(Abort::Confusion("ext4")),
    }
    Ok(())
}

fn out_write_stream(
    st: &mut crate::state::EngineState,
    w: NodePtr<WhatsitNode>,
    subtype: Option<WhatsitSubtype>,
) -> Result<(), Abort> {
    if subtype == Some(WhatsitSubtype::Write) {
        return write_out(st, w);
    }
    let j = w.write_stream(&st.mem);
    let stream = j as usize;
    if st.shipout.write_open[stream] {
        ensure_write_stream_open(st, stream)?;
        output_close(st.shipout.write_file[stream]);
    }
    if subtype == Some(WhatsitSubtype::Close) {
        st.shipout.write_open[stream] = false;
        if j < 16 {
            st.shipout.write_file[stream] = INVALID_HANDLE;
        }
        return Ok(());
    }
    if j >= 16 {
        return Ok(());
    }

    st.scanner.cur_name = w.open_name(&st.mem);
    st.scanner.cur_area = w.open_area(&st.mem);
    st.scanner.cur_ext = w.open_ext(&st.mem);
    if st.strings.length(st.scanner.cur_ext) == 0 {
        st.scanner.cur_ext = st.strings.make_tex_string(".tex")?;
    }
    let (n, a, e) = (st.scanner.cur_name, st.scanner.cur_area, st.scanner.cur_ext);
    crate::io::pack_file_name(st, n, a, e);
    let handle = output_open(st.input.name_of_file_bytes(), false);
    if handle == INVALID_HANDLE {
        return Err(Abort::CannotOpenOutput);
    }
    st.shipout.write_file[stream] = handle;
    st.shipout.write_open[stream] = true;
    st.shipout.write_name[stream] = Some(st.input.name_of_file.clone());
    st.shipout.write_prefix[stream].clear();
    if st.errors.log_opened {
        let old_setting = st.shipout.selector;
        st.shipout.selector = st.eqtb.log_selector();
        crate::output::print_nl(st, "\\openout");
        crate::output::print_int(st, j);
        crate::output::print(st, " = `");
        crate::output::print_file_name(st, n, a, e);
        crate::output::print(st, "'.");
        crate::output::print_nl(st, "");
        crate::output::print_ln(st);
        st.shipout.selector = old_setting;
    }
    Ok(())
}

/// Lazily recreate a checkpoint-restored TeX output stream and its prefix.
pub(crate) fn ensure_write_stream_open(
    st: &mut crate::state::EngineState,
    stream: usize,
) -> Result<(), Abort> {
    if st.shipout.write_file[stream] != INVALID_HANDLE {
        return Ok(());
    }
    let Some(name) = &st.shipout.write_name[stream] else {
        return Err(Abort::CannotOpenOutput);
    };
    let handle = output_open(name, false);
    if handle == INVALID_HANDLE {
        return Err(Abort::CannotOpenOutput);
    }
    for &code_unit in &st.shipout.write_prefix[stream] {
        crate::bridge::output_putc(handle, code_unit);
    }
    st.shipout.write_file[stream] = handle;
    Ok(())
}
