//! Shipping boxes out to the XDV file.
use crate::consts::{
    A_LEADERS, BEFORE, COUNT_BASE, CS_TOKEN_FLAG, DIMEN_BASE, DLIST, FONT_BASE,
    FROZEN_CONTROL_SEQUENCE, GLUE_SPEC_SIZE, INSERTED, INT_BASE, INT_PAR_MAG,
    INT_PAR_TRACING_ONLINE, INVALID_HANDLE, LEFT_TO_RIGHT, MAX_HALFWORD, MEDIUM_NODE_SIZE, MEM_TOP,
    MIN_HALFWORD, NATIVE_GLYPH_INFO_SIZE, NULL_FLAG, RIGHT_TO_LEFT, SELECTOR_LOG_ONLY,
    SELECTOR_NEW_STRING, SELECTOR_TERM_AND_LOG, SELECTOR_TERM_ONLY, SMALL_NODE_SIZE,
    SPACE_ADJUSTMENT, TEX_NULL, TOO_BIG_CHAR, WRITE_TEXT,
};
use crate::ext::apply_tfm_font_mapping;

use crate::bridge::{
    ttstub_output_close, ttstub_output_flush, ttstub_output_open, ttstub_output_write,
    ttstub_shell_escape,
};
use crate::mem::LIG_TRICK;
use crate::node::{
    AnyBox, AnyNode, EdgeNode, GlueNode, GlueSpecNode, MathNode, MovementNode, NodePtr, RuleNode,
    WhatsitNode, WhatsitSubtype,
};
use crate::state::{EqtbState, FontState, MemState, ScannerState, ShipoutState, StringsState};
use crate::units::{GlueSign, MemIdx, MemPtr, Scaled};

const DVI_BUF_SIZE: i32 = 16384;
const HALF_BUF: i32 = 8192;
const TEX_INFINITY: i32 = 0x7FFF_FFFF;

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

const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 65534;

const MOV_NONE_SEEN: i32 = 0;
const MOV_Y_HERE: i32 = 1;
const MOV_Z_HERE: i32 = 2;
const MOV_YZ_OK: i32 = 3;
const MOV_Y_OK: i32 = 4;
const MOV_Z_OK: i32 = 5;
const MOV_Y_SEEN: i32 = 6;
const MOV_D_FIXED: i32 = 6;
const MOV_Z_SEEN: i32 = 12;

const FILLL: u16 = 3;
const NORMAL: u16 = 0;
const C_LEADERS: u16 = 101;

const L_CODE: u16 = 4;
const R_CODE: u16 = 8;
const END_M_CODE: u16 = 3;
const REVERSED: u16 = 1;

const INT_PAR_TRACING_OUTPUT: i32 = 34;
const INT_PAR_XETEX_INTERWORD_SPACE_SHAPING: i32 = 78;
const DIMEN_PAR_H_OFFSET: i32 = 18;
const DIMEN_PAR_V_OFFSET: i32 = 19;
const DIMEN_PAR_PDF_PAGE_WIDTH: i32 = 21;
const DIMEN_PAR_PDF_PAGE_HEIGHT: i32 = 22;
pub(crate) const ONE_INCH: i32 = 4_736_287;
const ONE_INCH_LESS_ONE: i32 = ONE_INCH - 1;

const LEFT_BRACE_TOKEN: i32 = 0x20_0000;
const RIGHT_BRACE_TOKEN: i32 = 0x40_0000;
const END_WRITE: i32 = FROZEN_CONTROL_SEQUENCE + 8;

pub fn new_edge(mem: &mut MemState, s: i16, w: i32) -> Option<NodePtr<EdgeNode>> {
    let p = NodePtr::<EdgeNode>::alloc(mem)?;
    p.set_subtype(mem, s as u16);
    p.set_width(mem, Scaled(w));
    p.set_edge_dist(mem, Scaled(0));
    Some(p)
}

pub(crate) enum ShipoutAbort {
    /// `overflow("pool size")` while merging native words or writing a special.
    PoolSizeOverflow,
    NodeRecycle(crate::scan::NodeRecycleAbort),
    Confusion(&'static std::ffi::CStr),
    /// Short write to the XDV file.
    DviWriteFailed,
    /// The DVI length exceeded 0x7FFFFFFF.
    DviLengthOverflow,
    /// `overflow("main memory size")`.
    MainMemoryOverflow,
    /// A scanner abort while expanding a `\write`.
    Scc(crate::scan::SccAbort),
    CannotOpenOutput,
    Name(crate::io::NameAbort),
    NativeFont(crate::ext::NativeFontAbort),
}

impl From<crate::ext::NativeFontAbort> for ShipoutAbort {
    fn from(e: crate::ext::NativeFontAbort) -> Self {
        ShipoutAbort::NativeFont(e)
    }
}

impl From<crate::scan::SccAbort> for ShipoutAbort {
    fn from(a: crate::scan::SccAbort) -> Self {
        ShipoutAbort::Scc(a)
    }
}

impl From<crate::input::InputAbort> for ShipoutAbort {
    fn from(a: crate::input::InputAbort) -> Self {
        ShipoutAbort::Scc(crate::scan::SccAbort::Input(a))
    }
}

impl From<crate::io::NameAbort> for ShipoutAbort {
    fn from(a: crate::io::NameAbort) -> Self {
        ShipoutAbort::Name(a)
    }
}

impl From<crate::io::OpenLogAbort> for ShipoutAbort {
    fn from(a: crate::io::OpenLogAbort) -> Self {
        ShipoutAbort::Scc(crate::scan::SccAbort::OpenLog(a))
    }
}

impl From<crate::texmfmp::MakeTexStringAbort> for ShipoutAbort {
    fn from(a: crate::texmfmp::MakeTexStringAbort) -> Self {
        ShipoutAbort::Scc(crate::scan::SccAbort::MakeTexString(a))
    }
}

/// `Continue` keeps going, any other error outcome aborts.
fn ship_err(o: crate::errors::ErrorOutcome) -> Result<(), ShipoutAbort> {
    crate::scan::scc_err(o).map_err(ShipoutAbort::Scc)
}

fn get_avail_ship(mem: &mut MemState) -> Result<crate::units::MemPtr, ShipoutAbort> {
    crate::mem::get_avail(mem).ok_or(ShipoutAbort::MainMemoryOverflow)
}

impl ShipoutAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            ShipoutAbort::PoolSizeOverflow => crate::errors::EngineAbort::PoolOverflow,
            ShipoutAbort::NodeRecycle(e) => e.into_engine_abort(),
            ShipoutAbort::Confusion(msg) => crate::errors::EngineAbort::Confusion(msg.as_ptr()),
            ShipoutAbort::DviWriteFailed => {
                crate::errors::EngineAbort::TtAbort(c"failed to write data to XDV file".as_ptr())
            }
            ShipoutAbort::DviLengthOverflow => {
                crate::errors::EngineAbort::Fatal(c"dvi length exceeds 0x7FFFFFFF".as_ptr())
            }
            ShipoutAbort::MainMemoryOverflow => crate::errors::EngineAbort::Overflow {
                what: c"main memory size".as_ptr(),
                n: MEM_TOP + 1,
            },
            ShipoutAbort::Scc(e) => e.into_engine_abort(),
            ShipoutAbort::CannotOpenOutput => crate::errors::EngineAbort::CannotOpenOutput,
            ShipoutAbort::Name(e) => e.into_engine_abort(),
            ShipoutAbort::NativeFont(e) => e.into_engine_abort(),
        }
    }
}

fn write_to_dvi(shipout: &ShipoutState, a: i32, b: i32) -> Result<(), ShipoutAbort> {
    let bytes = &shipout.dvi_buf[a as usize..=b as usize];
    // SAFETY: FFI write of a live byte slice, no engine re-entry.
    let written = unsafe {
        ttstub_output_write(
            shipout.dvi_file,
            bytes.as_ptr() as *const libc::c_char,
            bytes.len(),
        )
    };
    if written != bytes.len() {
        return Err(ShipoutAbort::DviWriteFailed);
    }
    Ok(())
}

fn dvi_swap(shipout: &mut ShipoutState) -> Result<(), ShipoutAbort> {
    if shipout.dvi_ptr > TEX_INFINITY - shipout.dvi_offset {
        shipout.cur_s = -2;
        return Err(ShipoutAbort::DviLengthOverflow);
    }
    if shipout.dvi_limit == DVI_BUF_SIZE {
        write_to_dvi(shipout, 0, HALF_BUF - 1)?;
        shipout.dvi_limit = HALF_BUF;
        shipout.dvi_offset += DVI_BUF_SIZE;
        shipout.dvi_ptr = 0;
    } else {
        write_to_dvi(shipout, HALF_BUF, DVI_BUF_SIZE - 1)?;
        shipout.dvi_limit = DVI_BUF_SIZE;
    }
    shipout.dvi_gone += HALF_BUF;
    Ok(())
}

#[inline]
fn dvi_out(shipout: &mut ShipoutState, c: u8) -> Result<(), ShipoutAbort> {
    shipout.page_manifest.record_byte(c);
    shipout.dvi_buf[shipout.dvi_ptr as usize] = c;
    shipout.dvi_ptr += 1;
    if shipout.dvi_ptr == shipout.dvi_limit {
        dvi_swap(shipout)?;
    }
    Ok(())
}

fn dvi_bytes(shipout: &mut ShipoutState, mut bytes: &[u8]) -> Result<(), ShipoutAbort> {
    while !bytes.is_empty() {
        let ptr = shipout.dvi_ptr as usize;
        let n = bytes.len().min(shipout.dvi_limit as usize - ptr);
        shipout.dvi_buf[ptr..ptr + n].copy_from_slice(&bytes[..n]);
        shipout.page_manifest.record_bytes(n);
        shipout.dvi_ptr += n as i32;
        bytes = &bytes[n..];
        if shipout.dvi_ptr == shipout.dvi_limit {
            dvi_swap(shipout)?;
        }
    }
    Ok(())
}

fn dvi_four(shipout: &mut ShipoutState, x: i32) -> Result<(), ShipoutAbort> {
    dvi_bytes(shipout, &x.to_be_bytes())
}

fn dvi_two(shipout: &mut ShipoutState, s: u16) -> Result<(), ShipoutAbort> {
    dvi_bytes(shipout, &s.to_be_bytes())
}

fn synch_h(st: &mut crate::state::EngineState) -> Result<(), ShipoutAbort> {
    if st.shipout.cur_h != st.shipout.dvi_h {
        let delta = st.shipout.cur_h - st.shipout.dvi_h;
        movement(&mut st.mem, &mut st.shipout, delta, RIGHT1)?;
        st.shipout.dvi_h = st.shipout.cur_h;
    }
    Ok(())
}

fn synch_v(st: &mut crate::state::EngineState) -> Result<(), ShipoutAbort> {
    if st.shipout.cur_v != st.shipout.dvi_v {
        let delta = st.shipout.cur_v - st.shipout.dvi_v;
        movement(&mut st.mem, &mut st.shipout, delta, DOWN1)?;
        st.shipout.dvi_v = st.shipout.cur_v;
    }
    Ok(())
}

fn dvi_native_font_def(st: &mut crate::state::EngineState, f: i32) -> Result<(), ShipoutAbort> {
    // SAFETY: FFI fill of `xdv_buffer`, returning the definition length.
    let font_def_length = unsafe { crate::ext::make_font_def(&mut st.fonts, f) }?;
    dvi_out(&mut st.shipout, DEFINE_NATIVE_FONT)?;
    dvi_four(&mut st.shipout, f - 1)?;
    dvi_bytes(
        &mut st.shipout,
        &st.fonts.xdv_buffer[..font_def_length as usize],
    )
}

fn dvi_font_def(st: &mut crate::state::EngineState, f: i32) -> Result<(), ShipoutAbort> {
    let font_area = st.fonts.font_area_entry(f);
    if font_area == AAT_FONT_FLAG || font_area == OTGR_FONT_FLAG {
        return dvi_native_font_def(st, f);
    }
    let fu = f as usize;
    if f <= 256 {
        dvi_bytes(&mut st.shipout, &[FNT_DEF1, (f - 1) as u8])?;
    } else {
        dvi_bytes(
            &mut st.shipout,
            &[FNT_DEF1 + 1, ((f - 1) / 256) as u8, ((f - 1) % 256) as u8],
        )?;
    }
    let check = st.fonts.font_check[fu];
    dvi_bytes(
        &mut st.shipout,
        &[
            check.s3 as u8,
            check.s2 as u8,
            check.s1 as u8,
            check.s0 as u8,
        ],
    )?;
    dvi_four(&mut st.shipout, st.fonts.font_size[fu])?;
    dvi_four(&mut st.shipout, st.fonts.font_dsize[fu])?;
    dvi_out(
        &mut st.shipout,
        crate::string_pool::length(&st.strings, font_area) as u8,
    )?;

    let font_name = st.fonts.font_name[fu];
    let name = str_range(&st.strings, font_name);
    // Only the part before a `:features` suffix is written (a leading colon does not count).
    let colon = st.strings.str_pool[name.clone()]
        .iter()
        .skip(1)
        .position(|&c| c == b':' as u16)
        .map(|l| l + 1);
    let name_len = colon.map_or_else(
        || crate::string_pool::length(&st.strings, font_name),
        |l| l as i32,
    );
    dvi_out(&mut st.shipout, name_len as u8)?;

    let area = str_range(&st.strings, font_area);
    for k in area {
        dvi_out(&mut st.shipout, st.strings.str_pool[k] as u8)?;
    }
    let name_start = name.start;
    for k in name_start..name_start + name_len as usize {
        dvi_out(&mut st.shipout, st.strings.str_pool[k] as u8)?;
    }
    Ok(())
}

fn str_range(strings: &StringsState, s: i32) -> std::ops::Range<usize> {
    let start = strings.str_start[(s - TOO_BIG_CHAR) as usize] as usize;
    let end = strings.str_start[(s + 1 - TOO_BIG_CHAR) as usize] as usize;
    start..end
}

pub fn initialize_shipout_variables(st: &mut crate::state::EngineState) {
    let s = &mut st.shipout;
    s.output_file_name = 0;
    s.dvi_buf = vec![0u8; (DVI_BUF_SIZE + 1) as usize];
    s.dvi_limit = DVI_BUF_SIZE;
    s.dvi_ptr = 0;
    s.dvi_offset = 0;
    s.dvi_gone = 0;
    s.down_ptr = MemPtr::NULL;
    s.right_ptr = MemPtr::NULL;
    s.cur_s = -1;
}

fn int_par(eqtb: &EqtbState, k: i32) -> i32 {
    eqtb.b32s1(INT_BASE + k)
}

fn dimen_par(eqtb: &EqtbState, k: i32) -> i32 {
    eqtb.b32s1(DIMEN_BASE + k)
}

/// Emit the current string (the tail of the pool) as a DVI special and drop it.
fn dvi_special_from_cur_string(st: &mut crate::state::EngineState) -> Result<(), ShipoutAbort> {
    let base = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
    let cur_len = st.strings.pool_ptr - base;
    if cur_len < 256 {
        dvi_bytes(&mut st.shipout, &[XXX1, cur_len as u8])?;
    } else {
        dvi_out(&mut st.shipout, XXX4)?;
        dvi_four(&mut st.shipout, cur_len)?;
    }
    for k in base..st.strings.pool_ptr {
        dvi_out(&mut st.shipout, st.strings.str_pool[k as usize] as u8)?;
    }
    st.strings.pool_ptr = base;
    Ok(())
}

pub(crate) unsafe fn ship_out_in(
    st: &mut crate::state::EngineState,
    p: i32,
) -> Result<(), ShipoutAbort> {
    let mag = int_par(&st.eqtb, INT_PAR_MAG);
    let job_name = crate::texmfmp::gettexstring_in(&st.strings, st.strings.job_name);
    crate::synctex::synctex_sheet(st, mag, job_name);

    if st.strings.job_name == 0 {
        crate::io::open_log_file_in(st)?;
    }
    let tracing_output = int_par(&st.eqtb, INT_PAR_TRACING_OUTPUT) > 0;
    if tracing_output {
        crate::output::print_nl_cstr_in(st, c"".as_ptr());
        crate::output::print_ln_in(st);
        crate::output::print_cstr_in(st, c"Completed box being shipped out".as_ptr());
    }
    if st.shipout.term_offset > st.config.max_print_line - 9 {
        crate::output::print_ln_in(st);
    } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
        crate::output::print_char_in(st, b' ' as i32);
    }

    crate::output::print_char_in(st, b'[' as i32);
    let mut j = 9;
    while j > 0 && st.eqtb.b32s1(COUNT_BASE + j) == 0 {
        j -= 1;
    }
    for k in 0..=j {
        crate::output::print_int_in(st, st.eqtb.b32s1(COUNT_BASE + k));
        if k < j {
            crate::output::print_char_in(st, b'.' as i32);
        }
    }
    ttstub_output_flush(st.shipout.rust_stdout);

    if tracing_output {
        crate::output::print_char_in(st, b']' as i32);
        crate::output::begin_diagnostic_in(st);
        crate::output::show_box_in(st, p);
        crate::output::end_diagnostic_in(st, true);
    }

    let page = st
        .mem
        .classify_box(MemIdx::new(p))
        .expect("valid shipped box")
        .expect("shipped box");
    let (height, depth, width) = (
        page.height(&st.mem).0,
        page.depth(&st.mem).0,
        page.width(&st.mem).0,
    );
    let h_offset = dimen_par(&st.eqtb, DIMEN_PAR_H_OFFSET);
    let v_offset = dimen_par(&st.eqtb, DIMEN_PAR_V_OFFSET);
    if height > MAX_HALFWORD
        || depth > MAX_HALFWORD
        || height + depth + v_offset > MAX_HALFWORD
        || width + h_offset > MAX_HALFWORD
    {
        crate::output::error_here_with_diagnostic_in(
            st,
            c"Huge page cannot be shipped out".as_ptr(),
        );
        crate::output::capture_to_diagnostic_in(st, core::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] = c"The page just created is more than 18 feet tall or".as_ptr();
        st.errors.help_line[0] =
            c"more than 18 feet wide, so I suspect something went wrong.".as_ptr();
        ship_err(crate::errors::error_in(st))?;

        if int_par(&st.eqtb, INT_PAR_TRACING_OUTPUT) <= 0 {
            crate::output::begin_diagnostic_in(st);
            crate::output::print_nl_cstr_in(st, c"The following box has been deleted:".as_ptr());
            crate::output::show_box_in(st, p);
            crate::output::end_diagnostic_in(st, true);
        }
    } else {
        ship_page(st, page)?;
    }

    if st.scanner.lr_problems > 0 {
        crate::output::print_ln_in(st);
        crate::output::print_nl_cstr_in(st, c"\\endL or \\endR problem (".as_ptr());
        crate::output::print_int_in(st, st.scanner.lr_problems / 10000);
        crate::output::print_cstr_in(st, c" missing, ".as_ptr());
        crate::output::print_int_in(st, st.scanner.lr_problems % 10000);
        crate::output::print_cstr_in(st, c" extra".as_ptr());
        st.scanner.lr_problems = 0;
        crate::output::print_char_in(st, b')' as i32);
        crate::output::print_ln_in(st);
    }

    if !st.scanner.lr_ptr.is_null() || st.scanner.cur_dir != LEFT_TO_RIGHT {
        return Err(ShipoutAbort::Confusion(c"LR3"));
    }
    if int_par(&st.eqtb, INT_PAR_TRACING_OUTPUT) <= 0 {
        crate::output::print_char_in(st, b']' as i32);
    }
    st.shipout.dead_cycles = 0;
    ttstub_output_flush(st.shipout.rust_stdout);
    crate::packaging::flush_node_list(&mut st.mem, p).map_err(ShipoutAbort::Confusion)?;
    crate::synctex::synctex_teehs(st);
    Ok(())
}

unsafe fn ship_page(st: &mut crate::state::EngineState, page: AnyBox) -> Result<(), ShipoutAbort> {
    let (height, depth, width) = (
        page.height(&st.mem).0,
        page.depth(&st.mem).0,
        page.width(&st.mem).0,
    );
    let h_offset = dimen_par(&st.eqtb, DIMEN_PAR_H_OFFSET);
    let v_offset = dimen_par(&st.eqtb, DIMEN_PAR_V_OFFSET);
    let pdf_page_width = dimen_par(&st.eqtb, DIMEN_PAR_PDF_PAGE_WIDTH);
    let pdf_page_height = dimen_par(&st.eqtb, DIMEN_PAR_PDF_PAGE_HEIGHT);

    st.shipout.max_v = st.shipout.max_v.max(height + depth + v_offset);
    st.shipout.max_h = st.shipout.max_h.max(width + h_offset);
    st.shipout.dvi_h = 0;
    st.shipout.dvi_v = 0;
    st.shipout.cur_h = h_offset;
    st.shipout.dvi_f = FONT_BASE;
    st.shipout.cur_h_offset = h_offset + ONE_INCH;
    st.shipout.cur_v_offset = v_offset + ONE_INCH;
    st.shipout.cur_page_width = if pdf_page_width != 0 {
        pdf_page_width
    } else {
        width + 2 * (h_offset + ONE_INCH)
    };
    st.shipout.cur_page_height = if pdf_page_height != 0 {
        pdf_page_height
    } else {
        height + depth + 2 * (v_offset + ONE_INCH)
    };

    if st.shipout.output_file_name == 0 {
        if st.strings.job_name == 0 {
            crate::io::open_log_file_in(st)?;
        }
        crate::io::pack_job_name_in(st, crate::engine::output_file_extension(st))?;
        st.shipout.dvi_file = ttstub_output_open(st.input.name_of_file.as_ptr(), 0);
        if st.shipout.dvi_file == INVALID_HANDLE {
            return Err(ShipoutAbort::CannotOpenOutput);
        }
        st.shipout.output_file_name = crate::io::make_name_string_in(st)?;
    }

    if st.shipout.total_pages == 0 {
        dvi_out(&mut st.shipout, PRE)?;
        dvi_out(
            &mut st.shipout,
            id_byte(st.config.semantic_pagination_enabled),
        )?;
        dvi_four(&mut st.shipout, 25_400_000)?;
        dvi_four(&mut st.shipout, 473_628_672)?;
        ship_err(crate::save::prepare_mag_in(st))?;
        dvi_four(&mut st.shipout, int_par(&st.eqtb, INT_PAR_MAG))?;
        let comment = b"tectonic";
        dvi_out(&mut st.shipout, comment.len() as u8)?;
        dvi_bytes(&mut st.shipout, comment)?;
    }

    let page_loc = st.shipout.dvi_offset + st.shipout.dvi_ptr;
    st.shipout.page_manifest.begin_page(page_loc as u64);
    dvi_out(&mut st.shipout, BOP)?;
    for k in 0..10 {
        dvi_four(&mut st.shipout, st.eqtb.b32s1(COUNT_BASE + k))?;
    }
    let last_bop = st.shipout.last_bop;
    dvi_four(&mut st.shipout, last_bop)?;
    st.shipout.last_bop = page_loc;

    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    crate::output::print_cstr_in(st, c"pdf:pagesize ".as_ptr());
    if pdf_page_width <= 0 || pdf_page_height <= 0 {
        crate::output::print_cstr_in(st, c"default".as_ptr());
    } else {
        crate::output::print_cstr_in(st, c"width".as_ptr());
        crate::output::print_in(st, b' ' as i32);
        crate::output::print_scaled_in(st, pdf_page_width);
        crate::output::print_cstr_in(st, c"pt".as_ptr());
        crate::output::print_in(st, b' ' as i32);
        crate::output::print_cstr_in(st, c"height".as_ptr());
        crate::output::print_in(st, b' ' as i32);
        crate::output::print_scaled_in(st, pdf_page_height);
        crate::output::print_cstr_in(st, c"pt".as_ptr());
    }
    st.shipout.selector = old_setting;

    dvi_out(&mut st.shipout, XXX1)?;
    dvi_out(&mut st.shipout, crate::hash::cur_length(&st.strings) as u8)?;
    let base = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
    for k in base..st.strings.pool_ptr {
        dvi_out(&mut st.shipout, st.strings.str_pool[k as usize] as u8)?;
    }
    st.strings.pool_ptr = base;

    st.shipout.cur_v = height + v_offset;
    match page {
        AnyBox::VList(_) => vlist_out(st, page)?,
        AnyBox::HList(_) => hlist_out(st, page)?,
    }

    dvi_out(&mut st.shipout, EOP)?;
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

fn dvi_pop(shipout: &mut ShipoutState, l: i32) -> Result<(), ShipoutAbort> {
    if l == shipout.dvi_offset + shipout.dvi_ptr && shipout.dvi_ptr > 0 {
        shipout.dvi_ptr -= 1;
        shipout.page_manifest.pop_byte();
        Ok(())
    } else {
        dvi_out(shipout, POP)
    }
}

fn dvi_font_select(st: &mut crate::state::EngineState, f: i32) -> Result<(), ShipoutAbort> {
    if f == st.shipout.dvi_f {
        return Ok(());
    }
    if !st.fonts.font_used[f as usize] {
        dvi_font_def(st, f)?;
        st.fonts.font_used[f as usize] = true;
    }
    if f <= 64 {
        dvi_out(&mut st.shipout, f as u8 + FNT_NUM_0 - 1)?;
    } else if f <= 256 {
        dvi_bytes(&mut st.shipout, &[FNT1, (f - 1) as u8])?;
    } else {
        dvi_bytes(
            &mut st.shipout,
            &[FNT1 + 1, ((f - 1) / 256) as u8, ((f - 1) % 256) as u8],
        )?;
    }
    st.shipout.dvi_f = f;
    Ok(())
}

fn classify_non_char(mem: &MemState, p: MemPtr) -> Option<AnyNode> {
    if p.is_null() || crate::mem::is_char_node(mem, p) {
        return None;
    }
    mem.classify(MemIdx::new(p.0)).ok()
}

fn native_word_node(mem: &MemState, p: MemPtr) -> Option<NodePtr<WhatsitNode>> {
    match classify_non_char(mem, p)? {
        AnyNode::Whatsit(w) if w.whatsit_subtype(mem).is_some_and(|s| s.is_native_word()) => {
            Some(w)
        }
        _ => None,
    }
}

fn is_ignorable_interword_node(mem: &MemState, p: MemPtr) -> bool {
    match classify_non_char(mem, p) {
        Some(AnyNode::Penalty(_) | AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_)) => true,
        Some(AnyNode::Whatsit(w)) => w.whatsit_subtype(mem).is_some_and(|s| s.is_io()),
        _ => false,
    }
}

fn skip_ignorable_interword_nodes(mem: &MemState, mut q: MemPtr) -> MemPtr {
    while is_ignorable_interword_node(mem, q) {
        q = q.link(mem);
    }
    q
}

fn is_space_adjustment(mem: &MemState, q: MemPtr) -> bool {
    matches!(classify_non_char(mem, q), Some(AnyNode::Kern(k)) if k.subtype(mem) == SPACE_ADJUSTMENT)
}

/// Merge runs of native words separated by interword glue into single nodes
/// so they can be justified as a whole.
fn merge_justifiable_native_words(
    mem: &mut MemState,
    strings: &mut StringsState,
    fonts: &FontState,
    eqtb: &EqtbState,
    this_box: AnyBox,
) -> Result<(), ShipoutAbort> {
    let mut prev_p = this_box.list_head();
    let mut p = this_box.list_ptr(mem);

    while !p.is_null() {
        let first = native_word_node(mem, p).filter(|r| {
            fonts.font_letter_space_entry(r.native_font(mem).0) == 0 && !p.link(mem).is_null()
        });
        if let Some(r) = first {
            let font = r.native_font(mem);
            let same_font_word = |mem: &MemState, q: MemPtr| {
                native_word_node(mem, q).filter(|w| w.native_font(mem) == font)
            };
            let mut last = r;
            let mut text_len = r.native_length(mem) as i32;
            let mut q = r.link(mem);
            loop {
                q = skip_ignorable_interword_nodes(mem, q);
                match classify_non_char(mem, q) {
                    Some(AnyNode::Glue(glue)) if glue.subtype(mem) == NORMAL => {
                        if glue.glue_ptr(mem).0 == fonts.font_glue(font.0) {
                            q = skip_ignorable_interword_nodes(mem, glue.link(mem));
                            if let Some(w) = same_font_word(mem, q) {
                                last = w;
                                text_len += 1 + w.native_length(mem) as i32;
                                q = w.link(mem);
                                continue;
                            }
                        } else {
                            q = glue.link(mem);
                        }
                        if is_space_adjustment(mem, q) {
                            q = skip_ignorable_interword_nodes(mem, q.link(mem));
                            if let Some(w) = same_font_word(mem, q) {
                                last = w;
                                text_len += 1 + w.native_length(mem) as i32;
                                q = w.link(mem);
                                continue;
                            }
                        }
                    }
                    Some(AnyNode::Whatsit(_)) => {
                        if let Some(w) = same_font_word(mem, q) {
                            last = w;
                            q = w.link(mem);
                            continue;
                        }
                    }
                    _ => {}
                }
                break;
            }

            if last != r {
                p = merge_native_word_run(
                    mem, strings, fonts, eqtb, this_box, prev_p, r, last, text_len,
                )?;
            }
        }
        prev_p = p;
        p = p.link(mem);
    }
    Ok(())
}

/// Replace the run `first..=last` by one native word, moving the ignorable
/// nodes inside the run after it. Returns the last node of the replacement.
fn merge_native_word_run(
    mem: &mut MemState,
    strings: &mut StringsState,
    fonts: &FontState,
    eqtb: &EqtbState,
    this_box: AnyBox,
    prev_p: MemPtr,
    first: NodePtr<WhatsitNode>,
    last: NodePtr<WhatsitNode>,
    text_len: i32,
) -> Result<MemPtr, ShipoutAbort> {
    if strings.pool_ptr + text_len > strings.pool_size {
        return Err(ShipoutAbort::PoolSizeOverflow);
    }

    let g_order = this_box.glue_order(mem);
    let g_sign = this_box.glue_sign(mem);
    let mut width = 0;
    let mut q = first.ptr();
    loop {
        match mem.classify(MemIdx::new(q.0)) {
            Ok(AnyNode::Whatsit(w)) => {
                if w.whatsit_subtype(mem).is_some_and(|s| s.is_native_word()) {
                    let len = w.native_length(mem) as usize;
                    let text = w.native_text_ptr(mem);
                    for j in 0..len {
                        // SAFETY: a native word holds `native_length` UTF-16 units inline.
                        strings.str_pool[strings.pool_ptr as usize] = unsafe { *text.add(j) };
                        strings.pool_ptr += 1;
                    }
                    width += w.width(mem).0;
                }
            }
            Ok(AnyNode::Glue(glue)) => {
                strings.str_pool[strings.pool_ptr as usize] = b' ' as u16;
                strings.pool_ptr += 1;
                let spec = glue.glue_spec(mem).expect("glue node has a spec");
                width += spec.width(mem).0;
                match g_sign {
                    GlueSign::Stretching if spec.stretch_order(mem) == g_order => {
                        width += crate::scaled_math::tex_round(
                            this_box.glue_set(mem) * spec.stretch(mem).0 as f64,
                        );
                    }
                    GlueSign::Shrinking if spec.shrink_order(mem) == g_order => {
                        width -= crate::scaled_math::tex_round(
                            this_box.glue_set(mem) * spec.shrink(mem).0 as f64,
                        );
                    }
                    _ => {}
                }
            }
            Ok(AnyNode::Kern(kern)) => width += kern.width(mem).0,
            _ => {}
        }
        if q == last.ptr() {
            break;
        }
        q = q.link(mem);
    }

    let nlen = crate::hash::cur_length(strings);
    let word = crate::fonts::new_native_word_node(mem, eqtb, first.native_font(mem).0, nlen)
        .map_err(ShipoutAbort::NodeRecycle)?;
    word.set_subtype(mem, first.subtype(mem));
    let base = strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize] as usize;
    let text = &strings.str_pool[base..base + nlen as usize];
    let dst = word.native_text_ptr_mut(mem);
    // SAFETY: the new node holds `nlen` UTF-16 units inline, disjoint from the pool.
    unsafe { std::ptr::copy_nonoverlapping(text.as_ptr(), dst, text.len()) };
    word.set_width(mem, Scaled(width));
    // SAFETY: FFI re-measure of the node just built.
    unsafe { crate::ext::store_justified_native_glyphs(fonts, mem, word) }?;

    prev_p.set_link(mem, word.ptr());
    word.set_link(mem, last.link(mem));
    last.set_link(mem, MemPtr::NULL);

    let mut tail = word.ptr();
    let mut prev_p = first.ptr();
    let mut p = first.link(mem);
    while !p.is_null() {
        if is_ignorable_interword_node(mem, p) {
            prev_p.set_link(mem, p.link(mem));
            p.set_link(mem, tail.link(mem));
            tail.set_link(mem, p);
            tail = p;
        }
        prev_p = p;
        p = p.link(mem);
    }

    crate::packaging::flush_node_list(mem, first.ptr()).map_err(ShipoutAbort::Confusion)?;
    strings.pool_ptr = strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize];
    Ok(tail)
}

/// The glue-setting state of the box being shipped, carried across glue nodes
/// so rounding errors do not accumulate.
struct GlueRounding {
    order: u16,
    sign: GlueSign,
    set: f64,
    cur_g: i32,
    cur_glue: f64,
}

impl GlueRounding {
    fn new(mem: &MemState, this_box: AnyBox) -> GlueRounding {
        GlueRounding {
            order: this_box.glue_order(mem),
            sign: this_box.glue_sign(mem),
            set: this_box.glue_set(mem),
            cur_g: 0,
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
    fn width(&mut self, mem: &MemState, spec: NodePtr<GlueSpecNode>) -> i32 {
        let width = spec.width(mem).0 - self.cur_g;
        if self.is_set(mem, spec) {
            if self.sign == GlueSign::Stretching {
                self.cur_glue += spec.stretch(mem).0 as f64;
            } else {
                self.cur_glue -= spec.shrink(mem).0 as f64;
            }
            let glue_temp = (self.set * self.cur_glue).clamp(-1_000_000_000.0, 1_000_000_000.0);
            self.cur_g = crate::scaled_math::tex_round(glue_temp);
        }
        width + self.cur_g
    }
}

fn glue_spec(mem: &MemState, glue: NodePtr<GlueNode>) -> NodePtr<GlueSpecNode> {
    glue.glue_spec(mem).expect("glue node has a glue spec")
}

/// Replace glue that the box stretched or shrank by a rigid equivalent of
/// width `w`, so reflected or repeated output sees the width already rounded.
fn freeze_set_glue(
    mem: &mut MemState,
    glue: NodePtr<GlueNode>,
    spec: NodePtr<GlueSpecNode>,
    w: i32,
) -> Result<(), ShipoutAbort> {
    if spec.ref_count(mem) == TEX_NULL {
        crate::mem::free_node(mem, spec.ptr(), GLUE_SPEC_SIZE);
    } else {
        spec.set_ref_count(mem, spec.ref_count(mem) - 1);
    }
    if glue.subtype(mem) < A_LEADERS {
        glue.into_kern(mem, Scaled(w));
    } else {
        let rigid = NodePtr::<GlueSpecNode>::alloc(mem).ok_or(ShipoutAbort::MainMemoryOverflow)?;
        rigid.set_stretch_order(mem, FILLL + 1);
        rigid.set_shrink_order(mem, FILLL + 1);
        rigid.set_width(mem, Scaled(w));
        rigid.set_stretch(mem, Scaled(0));
        rigid.set_shrink(mem, Scaled(0));
        glue.set_glue_ptr(mem, rigid.ptr());
    }
    Ok(())
}

enum Leader {
    Rule(NodePtr<RuleNode>),
    Box(AnyBox),
}

fn leader_of(mem: &MemState, glue: NodePtr<GlueNode>) -> Leader {
    match mem.classify(MemIdx::new(glue.leader_ptr(mem).0)) {
        Ok(AnyNode::Rule(rule)) => Leader::Rule(rule),
        Ok(AnyNode::HList(h)) => Leader::Box(AnyBox::HList(h)),
        Ok(AnyNode::VList(v)) => Leader::Box(AnyBox::VList(v)),
        other => unreachable!("leaders must be a rule or a box, found {other:?}"),
    }
}

fn push_lr(mem: &mut MemState, scanner: &mut ScannerState, info: i32) -> Result<(), ShipoutAbort> {
    let node = get_avail_ship(mem)?;
    mem.temp_ptr = node;
    node.set_info(mem, info);
    node.set_link(mem, scanner.lr_ptr);
    scanner.lr_ptr = node;
    Ok(())
}

fn pop_lr(mem: &mut MemState, scanner: &mut ScannerState) {
    let node = scanner.lr_ptr;
    mem.temp_ptr = node;
    scanner.lr_ptr = node.link(mem);
    let avail = MemPtr::from_raw(mem.avail);
    node.set_link(mem, avail);
    mem.avail = node.0;
}

fn lr_dir(mem: &MemState, math: NodePtr<MathNode>) -> i16 {
    (math.subtype(mem) / R_CODE) as i16
}

fn end_lr_type(mem: &MemState, math: NodePtr<MathNode>) -> u16 {
    L_CODE * (math.subtype(mem) / L_CODE) + END_M_CODE
}

/// The LR stack top matches the end code of `math`.
fn lr_matches(mem: &MemState, scanner: &ScannerState, math: NodePtr<MathNode>) -> bool {
    scanner.lr_ptr.info(mem) as u16 == end_lr_type(mem, math)
}

fn font_character_width(fonts: &FontState, f: i32, c: i32) -> i32 {
    let info = fonts.font_info_b16(fonts.char_base_entry(f) + c);
    fonts.font_info_b32s1(fonts.width_base_entry(f) + info.s3 as i32)
}

fn push_dvi_level(shipout: &mut ShipoutState) -> Result<i32, ShipoutAbort> {
    shipout.cur_s += 1;
    if shipout.cur_s > 0 {
        dvi_out(shipout, PUSH)?;
    }
    shipout.max_push = shipout.max_push.max(shipout.cur_s);
    Ok(shipout.dvi_offset + shipout.dvi_ptr)
}

fn pop_dvi_level(
    mem: &mut MemState,
    shipout: &mut ShipoutState,
    save_loc: i32,
) -> Result<(), ShipoutAbort> {
    prune_movements(mem, shipout, save_loc);
    if shipout.cur_s > 0 {
        dvi_pop(shipout, save_loc)?;
    }
    shipout.cur_s -= 1;
    Ok(())
}

fn ship_box(st: &mut crate::state::EngineState, b: AnyBox) -> Result<(), ShipoutAbort> {
    match b {
        AnyBox::HList(_) => hlist_out(st, b),
        AnyBox::VList(_) => vlist_out(st, b),
    }
}

/// Ship one copy of a leader box with `doing_leaders` set.
fn ship_leader_box(st: &mut crate::state::EngineState, leader: AnyBox) -> Result<(), ShipoutAbort> {
    let outer_doing_leaders = st.shipout.doing_leaders;
    st.shipout.doing_leaders = true;
    ship_box(st, leader)?;
    st.shipout.doing_leaders = outer_doing_leaders;
    Ok(())
}

fn hlist_rule_out(
    st: &mut crate::state::EngineState,
    this_box: AnyBox,
    base_line: i32,
    p: MemPtr,
) -> Result<(), ShipoutAbort> {
    if st.shipout.rule_ht == NULL_FLAG {
        st.shipout.rule_ht = this_box.height(&st.mem).0;
    }
    if st.shipout.rule_dp == NULL_FLAG {
        st.shipout.rule_dp = this_box.depth(&st.mem).0;
    }
    st.shipout.rule_ht += st.shipout.rule_dp;
    if st.shipout.rule_ht > 0 && st.shipout.rule_wd > 0 {
        synch_h(st)?;
        st.shipout.cur_v = base_line + st.shipout.rule_dp;
        synch_v(st)?;
        dvi_rule(&mut st.shipout, SET_RULE)?;
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
    left_edge: i32,
    upwards: bool,
) -> Result<(), ShipoutAbort> {
    if st.shipout.rule_wd == NULL_FLAG {
        st.shipout.rule_wd = this_box.width(&st.mem).0;
    }
    st.shipout.rule_ht += st.shipout.rule_dp;
    move_past_vlist_rule(&mut st.shipout, upwards);
    if st.shipout.rule_ht > 0 && st.shipout.rule_wd > 0 {
        if st.scanner.cur_dir == RIGHT_TO_LEFT {
            st.shipout.cur_h -= st.shipout.rule_wd;
        }
        synch_h(st)?;
        synch_v(st)?;
        dvi_rule(&mut st.shipout, PUT_RULE)?;
        st.shipout.cur_h = left_edge;
    }
    Ok(())
}

fn dvi_rule(shipout: &mut ShipoutState, opcode: u8) -> Result<(), ShipoutAbort> {
    let (ht, wd) = (shipout.rule_ht, shipout.rule_wd);
    dvi_out(shipout, opcode)?;
    dvi_four(shipout, ht)?;
    dvi_four(shipout, wd)
}

fn move_past_vlist_rule(shipout: &mut ShipoutState, upwards: bool) {
    if upwards {
        shipout.cur_v -= shipout.rule_ht;
    } else {
        shipout.cur_v += shipout.rule_ht;
    }
}

/// Move right past glue `glue` or output its leaders.
fn hlist_glue_out(
    st: &mut crate::state::EngineState,
    this_box: AnyBox,
    base_line: i32,
    left_edge: i32,
    glue: NodePtr<GlueNode>,
    rounding: &mut GlueRounding,
) -> Result<(), ShipoutAbort> {
    let spec = glue_spec(&st.mem, glue);
    st.shipout.rule_wd = rounding.width(&st.mem, spec);
    if rounding.is_set(&st.mem, spec) {
        freeze_set_glue(&mut st.mem, glue, spec, st.shipout.rule_wd)?;
    }

    let subtype = glue.subtype(&st.mem);
    if subtype >= A_LEADERS {
        let leader = match leader_of(&st.mem, glue) {
            Leader::Rule(rule) => {
                st.shipout.rule_ht = rule.height(&st.mem).raw();
                st.shipout.rule_dp = rule.depth(&st.mem).raw();
                return hlist_rule_out(st, this_box, base_line, glue.ptr());
            }
            Leader::Box(leader) => leader,
        };
        let leader_wd = leader.width(&st.mem).0;
        if leader_wd > 0 && st.shipout.rule_wd > 0 {
            st.shipout.rule_wd += 10;
            if st.scanner.cur_dir == RIGHT_TO_LEFT {
                st.shipout.cur_h -= 10;
            }
            let edge = st.shipout.cur_h + st.shipout.rule_wd;
            let mut lx = 0;
            if subtype == A_LEADERS {
                let save_h = st.shipout.cur_h;
                st.shipout.cur_h =
                    left_edge + leader_wd * ((st.shipout.cur_h - left_edge) / leader_wd);
                if st.shipout.cur_h < save_h {
                    st.shipout.cur_h += leader_wd;
                }
            } else {
                let lq = st.shipout.rule_wd / leader_wd;
                let lr = st.shipout.rule_wd % leader_wd;
                if subtype == C_LEADERS {
                    st.shipout.cur_h += lr / 2;
                } else {
                    lx = lr / (lq + 1);
                    st.shipout.cur_h += (lr - (lq - 1) * lx) / 2;
                }
            }

            while st.shipout.cur_h + leader_wd <= edge {
                st.shipout.cur_v = base_line + leader.shift_amount(&st.mem).0;
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
                edge - 10
            };
            return Ok(());
        }
    }

    st.shipout.cur_h += st.shipout.rule_wd;
    crate::synctex::synctex_horizontal_rule_or_glue(st, glue.ptr());
    Ok(())
}

fn dvi_xdv_buffer(st: &mut crate::state::EngineState, len: usize) -> Result<(), ShipoutAbort> {
    dvi_bytes(&mut st.shipout, &st.fonts.xdv_buffer[..len])
}

fn native_word_out(
    st: &mut crate::state::EngineState,
    word: NodePtr<WhatsitNode>,
    subtype: WhatsitSubtype,
) -> Result<(), ShipoutAbort> {
    synch_h(st)?;
    synch_v(st)?;
    dvi_font_select(st, word.native_font(&st.mem).0)?;
    match subtype {
        WhatsitSubtype::Glyph => {
            dvi_out(&mut st.shipout, SET_GLYPHS)?;
            dvi_four(&mut st.shipout, word.width(&st.mem).0)?;
            dvi_two(&mut st.shipout, 1)?;
            dvi_four(&mut st.shipout, 0)?;
            dvi_four(&mut st.shipout, 0)?;
            dvi_two(&mut st.shipout, word.native_glyph(&st.mem))?;
        }
        WhatsitSubtype::NativeWordAt => {
            let len = word.native_length(&st.mem);
            if len > 0 || word.native_glyph_info_handle(&st.mem) != 0 {
                dvi_out(&mut st.shipout, SET_TEXT_AND_GLYPHS)?;
                dvi_two(&mut st.shipout, len)?;
                let text = word.native_text_ptr(&st.mem);
                for k in 0..len as usize {
                    // SAFETY: the node holds `native_length` UTF-16 units inline.
                    dvi_two(&mut st.shipout, unsafe { *text.add(k) })?;
                }
                let len = make_xdv_glyph_array_data(st, word);
                dvi_xdv_buffer(st, len)?;
            }
        }
        _ => {
            if word.native_glyph_info_handle(&st.mem) != 0 {
                dvi_out(&mut st.shipout, SET_GLYPHS)?;
                let len = make_xdv_glyph_array_data(st, word);
                dvi_xdv_buffer(st, len)?;
            }
        }
    }
    st.shipout.cur_h += word.width(&st.mem).0;
    st.shipout.dvi_h = st.shipout.cur_h;
    Ok(())
}

fn pdf_save_pos(shipout: &mut ShipoutState) {
    shipout.pdf_last_x_pos = shipout.cur_h + ONE_INCH_LESS_ONE;
    shipout.pdf_last_y_pos = shipout.cur_page_height - shipout.cur_v - ONE_INCH_LESS_ONE;
}

/// Output the characters starting at `p`, returning the first non-character
/// node and the matching advance of `prev_p`.
fn chars_out(
    st: &mut crate::state::EngineState,
    mut p: MemPtr,
    mut prev_p: MemPtr,
) -> Result<(MemPtr, MemPtr), ShipoutAbort> {
    synch_h(st)?;
    synch_v(st)?;
    while crate::mem::is_char_node(&st.mem, p) {
        let f = st.mem.b16s1(p.0) as i32;
        let mut c = st.mem.b16s0(p.0);
        let mapping = st.fonts.font_mapping_ptr(f);
        if p != LIG_TRICK && !mapping.is_null() {
            // SAFETY: `mapping` is the font's live TECkit converter.
            c = unsafe { apply_tfm_font_mapping(mapping, c as libc::c_int) } as u16;
        }
        dvi_font_select(st, f)?;
        if st.fonts.font_ec_entry(f) >= c
            && st.fonts.font_bc_entry(f) <= c
            && st
                .fonts
                .font_info_b16(st.fonts.char_base_entry(f) + c as i32)
                .s3
                > 0
        {
            if c >= 128 {
                dvi_out(&mut st.shipout, SET1)?;
            }
            dvi_out(&mut st.shipout, c as u8)?;
            st.shipout.cur_h += font_character_width(&st.fonts, f, c as i32);
        }
        prev_p = prev_p.link(&st.mem);
        p = p.link(&st.mem);
    }
    crate::synctex::synctex_current(st);
    st.shipout.dvi_h = st.shipout.cur_h;
    Ok((p, prev_p))
}

fn hlist_out(st: &mut crate::state::EngineState, this_box: AnyBox) -> Result<(), ShipoutAbort> {
    st.mem.temp_ptr = this_box.ptr();
    let mut rounding = GlueRounding::new(&st.mem, this_box);
    if int_par(&st.eqtb, INT_PAR_XETEX_INTERWORD_SPACE_SHAPING) > 1 {
        merge_justifiable_native_words(
            &mut st.mem,
            &mut st.strings,
            &st.fonts,
            &st.eqtb,
            this_box,
        )?;
    }

    // A raw cursor: it can be LIG_TRICK, and nodes get retyped in place.
    let mut p = this_box.list_ptr(&st.mem);
    let save_loc = push_dvi_level(&mut st.shipout)?;
    let base_line = st.shipout.cur_v;
    let mut prev_p = this_box.list_head();
    push_lr(&mut st.mem, &mut st.scanner, BEFORE)?;

    if this_box.subtype(&st.mem) == DLIST {
        if st.scanner.cur_dir == RIGHT_TO_LEFT {
            st.scanner.cur_dir = LEFT_TO_RIGHT;
            st.shipout.cur_h -= this_box.width(&st.mem).0;
        } else {
            this_box.set_subtype(&mut st.mem, 0);
        }
    }

    if st.scanner.cur_dir == RIGHT_TO_LEFT && this_box.subtype(&st.mem) != REVERSED {
        let save_h = st.shipout.cur_h;
        st.mem.temp_ptr = p;
        let kern =
            crate::mem::alloc_kern(&mut st.mem, 0).ok_or(ShipoutAbort::MainMemoryOverflow)?;
        kern.set_sync_tag(&mut st.mem, 0);
        prev_p.set_link(&mut st.mem, kern.ptr());
        st.shipout.cur_h = 0;
        let reversed = reverse(
            &mut st.mem,
            &mut st.shipout,
            &mut st.scanner,
            &mut st.fonts,
            None,
            &mut rounding,
        )?;
        kern.set_link(&mut st.mem, reversed);
        kern.set_width(&mut st.mem, Scaled(-st.shipout.cur_h));
        st.shipout.cur_h = save_h;
        this_box.set_subtype(&mut st.mem, REVERSED);
        p = kern.ptr();
    }

    let mut left_edge = st.shipout.cur_h;
    crate::synctex::synctex_hlist(st, this_box);

    while !p.is_null() {
        if crate::mem::is_char_node(&st.mem, p) {
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
                st.shipout.rule_ht = rule.height(&st.mem).raw();
                st.shipout.rule_dp = rule.depth(&st.mem).raw();
                st.shipout.rule_wd = rule.width(&st.mem).raw();
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
                    let edge = st.shipout.cur_h + whatsit.width(&st.mem).0;
                    pic_out(st, whatsit)?;
                    st.shipout.dvi_h = save_h;
                    st.shipout.dvi_v = save_v;
                    st.shipout.cur_h = edge;
                    st.shipout.cur_v = base_line;
                }
                Some(WhatsitSubtype::PdfSavePos) => pdf_save_pos(&mut st.shipout),
                _ => out_what(st, whatsit)?,
            },
            AnyNode::Glue(glue) => {
                hlist_glue_out(st, this_box, base_line, left_edge, glue, &mut rounding)?
            }
            AnyNode::MarginKern(kern) => st.shipout.cur_h += kern.width(&st.mem).0,
            AnyNode::Kern(kern) => {
                crate::synctex::synctex_kern(st, kern, this_box);
                st.shipout.cur_h += kern.width(&st.mem).0;
            }
            AnyNode::Math(math) => {
                crate::synctex::synctex_math(st, math);
                let math_width = math.width(&st.mem).0;
                if !math.subtype(&st.mem).is_multiple_of(2) {
                    if lr_matches(&st.mem, &st.scanner, math) {
                        pop_lr(&mut st.mem, &mut st.scanner);
                    } else if math.subtype(&st.mem) > L_CODE {
                        st.scanner.lr_problems += 1;
                    }
                } else {
                    let end_type = end_lr_type(&st.mem, math) as i32;
                    push_lr(&mut st.mem, &mut st.scanner, end_type)?;
                    if lr_dir(&st.mem, math) != st.scanner.cur_dir {
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
                st.fonts.set_xtx_ligature_present(true);
                continue;
            }
            AnyNode::Style(style) => {
                let edge = style.as_edge();
                st.shipout.cur_h += edge.width(&st.mem).0;
                left_edge = st.shipout.cur_h + edge.edge_dist(&st.mem).0;
                st.scanner.cur_dir = edge.subtype(&st.mem) as i16;
            }
            _ => {}
        }
        prev_p = p;
        p = p.link(&st.mem);
    }

    crate::synctex::synctex_tsilh(st, this_box);

    while st.scanner.lr_ptr.info(&st.mem) != BEFORE {
        if st.scanner.lr_ptr.info(&st.mem) as u16 > L_CODE {
            st.scanner.lr_problems += 10000;
        }
        pop_lr(&mut st.mem, &mut st.scanner);
    }
    pop_lr(&mut st.mem, &mut st.scanner);
    if this_box.subtype(&st.mem) == DLIST {
        st.scanner.cur_dir = RIGHT_TO_LEFT;
    }
    pop_dvi_level(&mut st.mem, &mut st.shipout, save_loc)
}

fn hlist_box_out(
    st: &mut crate::state::EngineState,
    b: AnyBox,
    base_line: i32,
) -> Result<(), ShipoutAbort> {
    if b.list_ptr(&st.mem).is_null() {
        match b {
            AnyBox::VList(v) => crate::synctex::synctex_void_vlist(st, v),
            AnyBox::HList(h) => crate::synctex::synctex_void_hlist(st, h),
        }
        st.shipout.cur_h += b.width(&st.mem).0;
        return Ok(());
    }
    let save_h = st.shipout.dvi_h;
    let save_v = st.shipout.dvi_v;
    st.shipout.cur_v = base_line + b.shift_amount(&st.mem).0;
    let edge = st.shipout.cur_h + b.width(&st.mem).0;
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
    left_edge: i32,
    math_width: i32,
    rounding: &mut GlueRounding,
) -> Result<NodePtr<EdgeNode>, ShipoutAbort> {
    let save_h = st.shipout.cur_h;
    st.mem.temp_ptr = math.link(&st.mem);
    st.shipout.rule_wd = math_width;
    crate::mem::free_node(&mut st.mem, math.ptr(), MEDIUM_NODE_SIZE);
    st.scanner.cur_dir = 1 - st.scanner.cur_dir;
    let edge = new_edge(&mut st.mem, st.scanner.cur_dir, st.shipout.rule_wd)
        .ok_or(ShipoutAbort::MainMemoryOverflow)?;
    prev_p.set_link(&mut st.mem, edge.ptr());
    st.shipout.cur_h = st.shipout.cur_h - left_edge + st.shipout.rule_wd;
    let end =
        new_edge(&mut st.mem, 1 - st.scanner.cur_dir, 0).ok_or(ShipoutAbort::MainMemoryOverflow)?;
    let reversed = reverse(
        &mut st.mem,
        &mut st.shipout,
        &mut st.scanner,
        &mut st.fonts,
        Some(end),
        rounding,
    )?;
    edge.set_link(&mut st.mem, reversed);
    edge.set_edge_dist(&mut st.mem, Scaled(st.shipout.cur_h));
    st.scanner.cur_dir = 1 - st.scanner.cur_dir;
    st.shipout.cur_h = save_h;
    Ok(edge)
}

fn vlist_out(st: &mut crate::state::EngineState, this_box: AnyBox) -> Result<(), ShipoutAbort> {
    st.mem.temp_ptr = this_box.ptr();
    let mut rounding = GlueRounding::new(&st.mem, this_box);
    let mut p = this_box.list_ptr(&st.mem);
    let upwards = this_box.subtype(&st.mem) == 1;
    let save_loc = push_dvi_level(&mut st.shipout)?;
    let left_edge = st.shipout.cur_h;
    crate::synctex::synctex_vlist(st, this_box);
    if upwards {
        st.shipout.cur_v += this_box.depth(&st.mem).0;
    } else {
        st.shipout.cur_v -= this_box.height(&st.mem).0;
    }
    let top_edge = st.shipout.cur_v;

    while !p.is_null() {
        if crate::mem::is_char_node(&st.mem, p) {
            return Err(ShipoutAbort::Confusion(c"vlistout"));
        }
        match st
            .mem
            .classify(MemIdx::new(p.0))
            .expect("valid node type in vlist_out")
        {
            AnyNode::HList(h) => vlist_box_out(st, AnyBox::HList(h), left_edge, upwards)?,
            AnyNode::VList(v) => vlist_box_out(st, AnyBox::VList(v), left_edge, upwards)?,
            AnyNode::Rule(rule) => {
                st.shipout.rule_ht = rule.height(&st.mem).raw();
                st.shipout.rule_dp = rule.depth(&st.mem).raw();
                st.shipout.rule_wd = rule.width(&st.mem).raw();
                vlist_rule_out(st, this_box, left_edge, upwards)?;
            }
            AnyNode::Whatsit(whatsit) => match whatsit.whatsit_subtype(&st.mem) {
                Some(WhatsitSubtype::Glyph) => {
                    st.shipout.cur_v += whatsit.height(&st.mem).0;
                    st.shipout.cur_h = left_edge;
                    synch_h(st)?;
                    synch_v(st)?;
                    dvi_font_select(st, whatsit.native_font(&st.mem).0)?;
                    dvi_out(&mut st.shipout, SET_GLYPHS)?;
                    dvi_four(&mut st.shipout, 0)?;
                    dvi_two(&mut st.shipout, 1)?;
                    dvi_four(&mut st.shipout, 0)?;
                    dvi_four(&mut st.shipout, 0)?;
                    dvi_two(&mut st.shipout, whatsit.native_glyph(&st.mem))?;
                    st.shipout.cur_v += whatsit.depth(&st.mem).0;
                    st.shipout.cur_h = left_edge;
                }
                Some(WhatsitSubtype::Pic | WhatsitSubtype::Pdf) => {
                    let save_h = st.shipout.dvi_h;
                    let save_v = st.shipout.dvi_v;
                    st.shipout.cur_v += whatsit.height(&st.mem).0;
                    pic_out(st, whatsit)?;
                    st.shipout.dvi_h = save_h;
                    st.shipout.dvi_v = save_v;
                    st.shipout.cur_v = save_v + whatsit.depth(&st.mem).0;
                    st.shipout.cur_h = left_edge;
                }
                Some(WhatsitSubtype::PdfSavePos) => pdf_save_pos(&mut st.shipout),
                _ => out_what(st, whatsit)?,
            },
            AnyNode::Glue(glue) => {
                let spec = glue_spec(&st.mem, glue);
                st.shipout.rule_ht = rounding.width(&st.mem, spec);
                let subtype = glue.subtype(&st.mem);
                if subtype < A_LEADERS {
                    move_past_vlist_rule(&mut st.shipout, upwards);
                } else {
                    match leader_of(&st.mem, glue) {
                        Leader::Rule(rule) => {
                            st.shipout.rule_wd = rule.width(&st.mem).raw();
                            st.shipout.rule_dp = 0;
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
                    st.shipout.cur_v -= kern.width(&st.mem).0;
                } else {
                    st.shipout.cur_v += kern.width(&st.mem).0;
                }
            }
            _ => {}
        }
        p = p.link(&st.mem);
    }

    crate::synctex::synctex_tsilv(st, this_box);
    pop_dvi_level(&mut st.mem, &mut st.shipout, save_loc)
}

fn vlist_box_out(
    st: &mut crate::state::EngineState,
    b: AnyBox,
    left_edge: i32,
    upwards: bool,
) -> Result<(), ShipoutAbort> {
    let (height, depth) = (b.height(&st.mem).0, b.depth(&st.mem).0);
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
    let shift = b.shift_amount(&st.mem).0;
    st.shipout.cur_h = if st.scanner.cur_dir == RIGHT_TO_LEFT {
        left_edge - shift
    } else {
        left_edge + shift
    };
    ship_box(st, b)?;
    st.shipout.dvi_h = save_h;
    st.shipout.dvi_v = save_v;
    st.shipout.cur_v = if upwards {
        save_v - b.height(&st.mem).0
    } else {
        save_v + b.depth(&st.mem).0
    };
    st.shipout.cur_h = left_edge;
    Ok(())
}

fn vlist_leaders_out(
    st: &mut crate::state::EngineState,
    leader: AnyBox,
    subtype: u16,
    left_edge: i32,
    top_edge: i32,
    upwards: bool,
) -> Result<(), ShipoutAbort> {
    let leader_ht = leader.height(&st.mem).0 + leader.depth(&st.mem).0;
    if leader_ht <= 0 || st.shipout.rule_ht <= 0 {
        move_past_vlist_rule(&mut st.shipout, upwards);
        return Ok(());
    }
    st.shipout.rule_ht += 10;
    let edge = st.shipout.cur_v + st.shipout.rule_ht;
    let mut lx = 0;
    if subtype == A_LEADERS {
        let save_v = st.shipout.cur_v;
        st.shipout.cur_v = top_edge + leader_ht * ((st.shipout.cur_v - top_edge) / leader_ht);
        if st.shipout.cur_v < save_v {
            st.shipout.cur_v += leader_ht;
        }
    } else {
        let lq = st.shipout.rule_ht / leader_ht;
        let lr = st.shipout.rule_ht % leader_ht;
        if subtype == C_LEADERS {
            st.shipout.cur_v += lr / 2;
        } else {
            lx = lr / (lq + 1);
            st.shipout.cur_v += (lr - (lq - 1) * lx) / 2;
        }
    }

    while st.shipout.cur_v + leader_ht <= edge {
        let shift = leader.shift_amount(&st.mem).0;
        st.shipout.cur_h = if st.scanner.cur_dir == RIGHT_TO_LEFT {
            left_edge - shift
        } else {
            left_edge + shift
        };
        synch_h(st)?;
        let save_h = st.shipout.dvi_h;
        st.shipout.cur_v += leader.height(&st.mem).0;
        synch_v(st)?;
        let save_v = st.shipout.dvi_v;
        ship_leader_box(st, leader)?;
        st.shipout.dvi_v = save_v;
        st.shipout.dvi_h = save_h;
        st.shipout.cur_h = left_edge;
        st.shipout.cur_v = save_v - leader.height(&st.mem).0 + leader_ht + lx;
    }
    st.shipout.cur_v = edge - 10;
    Ok(())
}

fn prune_movements(mem: &mut MemState, shipout: &mut ShipoutState, l: i32) {
    prune_movement_list(mem, &mut shipout.down_ptr, l);
    prune_movement_list(mem, &mut shipout.right_ptr, l);
}

fn prune_movement_list(mem: &mut MemState, head: &mut MemPtr, l: i32) {
    while let Some(p) = NodePtr::<MovementNode>::from_ptr(*head) {
        if p.location(mem) < l {
            return;
        }
        *head = p.link(mem);
        p.free(mem);
    }
}

pub(crate) fn finalize_dvi_file_in(st: &mut crate::state::EngineState) -> Result<(), ShipoutAbort> {
    while st.shipout.cur_s > -1 {
        if st.shipout.cur_s > 0 {
            dvi_out(&mut st.shipout, POP)?;
        } else {
            dvi_out(&mut st.shipout, EOP)?;
            st.shipout
                .page_manifest
                .finish_page(st.scanner.logical_clock);
            st.shipout.total_pages += 1;
        }
        st.shipout.cur_s -= 1;
    }

    if st.shipout.total_pages == 0 {
        crate::output::print_nl_cstr_in(st, c"No pages of output.".as_ptr());
        return Ok(());
    }
    // The DVI grew too big and the message was already printed.
    if st.shipout.cur_s == -2 {
        return Ok(());
    }

    dvi_out(&mut st.shipout, POST)?;
    let last_bop = st.shipout.last_bop;
    dvi_four(&mut st.shipout, last_bop)?;
    st.shipout.last_bop = st.shipout.dvi_offset + st.shipout.dvi_ptr - 5;
    dvi_four(&mut st.shipout, 25_400_000)?;
    dvi_four(&mut st.shipout, 473_628_672)?;
    ship_err(crate::save::prepare_mag_in(st))?;
    dvi_four(&mut st.shipout, int_par(&st.eqtb, INT_PAR_MAG))?;
    let (max_v, max_h, max_push, total_pages) = (
        st.shipout.max_v,
        st.shipout.max_h,
        st.shipout.max_push,
        st.shipout.total_pages,
    );
    dvi_four(&mut st.shipout, max_v)?;
    dvi_four(&mut st.shipout, max_h)?;
    dvi_bytes(
        &mut st.shipout,
        &[
            (max_push / 256) as u8,
            (max_push % 256) as u8,
            ((total_pages / 256) % 256) as u8,
            (total_pages % 256) as u8,
        ],
    )?;

    let mut f = st.fonts.font_ptr;
    while f > FONT_BASE {
        if st.fonts.font_used[f as usize] {
            dvi_font_def(st, f)?;
        }
        f -= 1;
    }
    st.fonts.font_ptr = f;

    dvi_out(&mut st.shipout, POST_POST)?;
    let last_bop = st.shipout.last_bop;
    dvi_four(&mut st.shipout, last_bop)?;
    dvi_out(
        &mut st.shipout,
        id_byte(st.config.semantic_pagination_enabled),
    )?;
    let padding = 4 + (DVI_BUF_SIZE - st.shipout.dvi_ptr) % 4;
    for _ in 0..padding {
        dvi_out(&mut st.shipout, 223)?;
    }

    if st.shipout.dvi_limit == HALF_BUF {
        write_to_dvi(&st.shipout, HALF_BUF, DVI_BUF_SIZE - 1)?;
    }
    if st.shipout.dvi_ptr > TEX_INFINITY - st.shipout.dvi_offset {
        st.shipout.cur_s = -2;
        return Err(ShipoutAbort::DviLengthOverflow);
    }
    if st.shipout.dvi_ptr > 0 {
        write_to_dvi(&st.shipout, 0, st.shipout.dvi_ptr - 1)?;
    }

    // SAFETY: FFI close of the output file.
    let k = unsafe { ttstub_output_close(st.shipout.dvi_file) as u8 };
    if k == 0 {
        crate::output::print_nl_cstr_in(st, c"Output written on ".as_ptr());
        crate::output::print_in(st, st.shipout.output_file_name);
        crate::output::print_cstr_in(st, c" (".as_ptr());
        crate::output::print_int_in(st, st.shipout.total_pages);
        if st.shipout.total_pages != 1 {
            crate::output::print_cstr_in(st, c" pages".as_ptr());
        } else {
            crate::output::print_cstr_in(st, c" page".as_ptr());
        }
        crate::output::print_cstr_in(st, c", ".as_ptr());
        crate::output::print_int_in(st, st.shipout.dvi_offset + st.shipout.dvi_ptr);
        crate::output::print_cstr_in(st, c" bytes).".as_ptr());
    } else {
        crate::output::print_nl_cstr_in(st, c"Error ".as_ptr());
        crate::output::print_int_in(st, k as i32);
        crate::output::print_cstr_in(st, c" (".as_ptr());
        // SAFETY: strerror returns a static NUL-terminated message.
        let message = unsafe { std::ffi::CStr::from_ptr(libc::strerror(k as libc::c_int)) };
        for &c in message.to_bytes() {
            crate::output::print_char_in(st, c as libc::c_char as i32);
        }
        crate::output::print_cstr_in(st, c") generating output;".as_ptr());
        crate::output::print_nl_cstr_in(st, c"file ".as_ptr());
        crate::output::print_in(st, st.shipout.output_file_name);
        crate::output::print_cstr_in(st, c" may not be valid.".as_ptr());
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
        put(&loc.x.to_be_bytes());
        put(&loc.y.to_be_bytes());
    }
    for gid in &glyphs.gids[..glyph_count] {
        put(&gid.to_be_bytes());
    }
    len
}

/// Reverse the hlist starting at `temp_ptr` onto `t` (the segment's closing
/// edge, or `None` for a whole list), returning the new head.
fn reverse(
    mem: &mut MemState,
    shipout: &mut ShipoutState,
    scanner: &mut ScannerState,
    fonts: &mut FontState,
    t: Option<NodePtr<EdgeNode>>,
    rounding: &mut GlueRounding,
) -> Result<MemPtr, ShipoutAbort> {
    let mut l = t.map_or(MemPtr::NULL, |t| t.ptr());
    let mut p = mem.temp_ptr;
    let mut m = MIN_HALFWORD;
    let mut n = MIN_HALFWORD;

    loop {
        while !p.is_null() {
            if crate::mem::is_char_node(mem, p) {
                while crate::mem::is_char_node(mem, p) {
                    let f = mem.b16s1(p.0) as i32;
                    let c = crate::fonts::effective_char(fonts, true, f, mem.b16s0(p.0));
                    shipout.cur_h += font_character_width(fonts, f, c);
                    let q = p.link(mem);
                    p.set_link(mem, l);
                    l = p;
                    p = q;
                }
                continue;
            }

            let q = p.link(mem);
            let mut add_width = true;
            let mut now_kern = false;
            match mem.classify(MemIdx::new(p.0)) {
                Ok(node @ (AnyNode::HList(_) | AnyNode::VList(_) | AnyNode::Rule(_))) => {
                    shipout.rule_wd = node.hlist_width(mem);
                }
                Ok(AnyNode::Kern(kern)) => {
                    shipout.rule_wd = kern.width(mem).0;
                    now_kern = true;
                }
                Ok(AnyNode::Whatsit(w)) => {
                    if w.whatsit_subtype(mem).is_some_and(|s| s.has_dimensions()) {
                        shipout.rule_wd = w.width(mem).0;
                    } else {
                        add_width = false;
                    }
                }
                Ok(AnyNode::Glue(glue)) => {
                    let spec = glue_spec(mem, glue);
                    shipout.rule_wd = rounding.width(mem, spec);
                    if rounding.is_set(mem, spec) {
                        freeze_set_glue(mem, glue, spec, shipout.rule_wd)?;
                        now_kern = glue.subtype(mem) < A_LEADERS;
                    }
                }
                Ok(AnyNode::Ligature(ligature)) => {
                    crate::mem::flush_list(mem, ligature.lig_ptr(mem));
                    mem.temp_ptr = ligature.ptr();
                    let c = get_avail_ship(mem)?;
                    mem.set_word(c.0, mem.word(ligature.idx().offset(1).0));
                    c.set_link(mem, q);
                    crate::mem::free_node(mem, ligature.ptr(), SMALL_NODE_SIZE);
                    p = c;
                    continue;
                }
                Ok(AnyNode::Math(math)) => {
                    shipout.rule_wd = math.width(mem).0;
                    if !math.subtype(mem).is_multiple_of(2) {
                        if !lr_matches(mem, scanner, math) {
                            math.into_kern(mem);
                            now_kern = true;
                            scanner.lr_problems += 1;
                        } else {
                            pop_lr(mem, scanner);
                            if n > MIN_HALFWORD {
                                n -= 1;
                                math.set_subtype(mem, math.subtype(mem) - 1);
                            } else {
                                math.into_kern(mem);
                                now_kern = true;
                                if m > MIN_HALFWORD {
                                    m -= 1;
                                } else {
                                    crate::mem::free_node(mem, math.ptr(), MEDIUM_NODE_SIZE);
                                    let t = t.expect("a segment reversal ends at its edge");
                                    t.set_link(mem, q);
                                    t.set_width(mem, Scaled(shipout.rule_wd));
                                    t.set_edge_dist(mem, Scaled(-shipout.cur_h - shipout.rule_wd));
                                    return Ok(l);
                                }
                            }
                        }
                    } else {
                        let end_type = end_lr_type(mem, math) as i32;
                        push_lr(mem, scanner, end_type)?;
                        if n > MIN_HALFWORD || lr_dir(mem, math) != scanner.cur_dir {
                            n += 1;
                            math.set_subtype(mem, math.subtype(mem) + 1);
                        } else {
                            math.into_kern(mem);
                            now_kern = true;
                            m += 1;
                        }
                    }
                }
                Ok(AnyNode::Style(_)) => return Err(ShipoutAbort::Confusion(c"LR2")),
                _ => add_width = false,
            }

            if add_width {
                shipout.cur_h += shipout.rule_wd;
            }
            p.set_link(mem, l);
            if now_kern && (shipout.rule_wd == 0 || l.is_null()) {
                crate::mem::free_node(mem, p, MEDIUM_NODE_SIZE);
                p = l;
            }
            l = p;
            p = q;
        }

        if t.is_none() && m == MIN_HALFWORD && n == MIN_HALFWORD {
            return Ok(l);
        }
        let dir = scanner.lr_ptr.info(mem) as i16;
        p = crate::mem::alloc_math(mem, 0, dir)
            .ok_or(ShipoutAbort::MainMemoryOverflow)?
            .ptr();
        scanner.lr_problems += 10000;
    }
}

fn movement(
    mem: &mut MemState,
    shipout: &mut ShipoutState,
    w: i32,
    o: u8,
) -> Result<(), ShipoutAbort> {
    let mut q = NodePtr::<MovementNode>::alloc(mem).ok_or(ShipoutAbort::MainMemoryOverflow)?;
    q.set_distance(mem, Scaled(w));
    q.set_location(mem, shipout.dvi_offset + shipout.dvi_ptr);
    let head = if o == DOWN1 {
        &mut shipout.down_ptr
    } else {
        &mut shipout.right_ptr
    };
    q.set_link(mem, *head);
    *head = q.ptr();

    let mut p = NodePtr::<MovementNode>::from_ptr(q.link(mem));
    let mut mstate = MOV_NONE_SEEN;
    let found = 'search: {
        while let Some(node) = p {
            p = NodePtr::<MovementNode>::from_ptr(node.link(mem));
            if node.distance(mem).0 != w {
                match mstate + node.state(mem) {
                    1 => mstate = MOV_Y_SEEN,
                    2 => mstate = MOV_Z_SEEN,
                    8 | 13 => break 'search None,
                    _ => {}
                }
                continue;
            }
            let (delta, here) = match mstate + node.state(mem) {
                // NONE_SEEN or Z_SEEN meeting YZ_OK or Y_OK
                3 | 4 | 15 | 16 => (5, MOV_Y_HERE),
                // NONE_SEEN meeting Z_OK, Y_SEEN meeting YZ_OK or Z_OK
                5 | 9 | 11 => (10, MOV_Z_HERE),
                // a y or z that is already here
                1 | 2 | 8 | 13 => break 'search Some(node),
                _ => continue,
            };
            let location = node.location(mem);
            if location < shipout.dvi_gone {
                break 'search None;
            }
            let mut k = location - shipout.dvi_offset;
            if k < 0 {
                k += DVI_BUF_SIZE;
            }
            shipout.dvi_buf[k as usize] += delta;
            shipout.page_manifest.patch_byte(location as u64, delta);
            node.set_state(mem, here);
            break 'search Some(node);
        }
        None
    };

    let Some(found) = found else {
        q.set_state(mem, MOV_YZ_OK);
        let len = if w.abs() >= 0x800000 {
            4
        } else if w.abs() >= 0x8000 {
            3
        } else if w.abs() >= 128 {
            2
        } else {
            1
        };
        dvi_out(shipout, o + len as u8 - 1)?;
        return dvi_bytes(shipout, &w.to_be_bytes()[4 - len..]);
    };

    q.set_state(mem, found.state(mem));
    let (opcode, yz_becomes, fixed_from) = if q.state(mem) == MOV_Y_HERE {
        (o + 4, MOV_Z_OK, MOV_Y_OK)
    } else {
        (o + 9, MOV_Y_OK, MOV_Z_OK)
    };
    while q.link(mem) != found.ptr() {
        q = NodePtr::<MovementNode>::from_ptr(q.link(mem))
            .expect("found movement record is linked from the new record");
        let state = q.state(mem);
        if state == MOV_YZ_OK {
            q.set_state(mem, yz_becomes);
        } else if state == fixed_from {
            q.set_state(mem, MOV_D_FIXED);
        }
    }
    dvi_out(shipout, opcode)
}

fn special_out(
    st: &mut crate::state::EngineState,
    special: NodePtr<WhatsitNode>,
) -> Result<(), ShipoutAbort> {
    synch_h(st)?;
    synch_v(st)?;
    st.shipout.doing_special = true;
    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    let tokens = special.write_tokens(&st.mem).link(&st.mem).0;
    let avail = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list_in(st, tokens, TEX_NULL, avail);
    st.shipout.selector = old_setting;
    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(ShipoutAbort::PoolSizeOverflow);
    }
    dvi_special_from_cur_string(st)?;
    st.shipout.doing_special = false;
    Ok(())
}

/// Log only, or terminal and log when `\tracingonline` is positive.
fn log_selector(eqtb: &EqtbState) -> i32 {
    if int_par(eqtb, INT_PAR_TRACING_ONLINE) <= 0 {
        SELECTOR_LOG_ONLY
    } else {
        SELECTOR_TERM_AND_LOG
    }
}

fn write_out(
    st: &mut crate::state::EngineState,
    write: NodePtr<WhatsitNode>,
) -> Result<(), ShipoutAbort> {
    let q = get_avail_ship(&mut st.mem)?;
    q.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
    let r = get_avail_ship(&mut st.mem)?;
    q.set_link(&mut st.mem, r);
    r.set_info(&mut st.mem, CS_TOKEN_FLAG + END_WRITE);
    crate::input::begin_token_list_in(st, q.0, INSERTED)?;
    crate::input::begin_token_list_in(st, write.write_tokens(&st.mem).0, WRITE_TEXT)?;
    let q = get_avail_ship(&mut st.mem)?;
    q.set_info(&mut st.mem, LEFT_BRACE_TOKEN + b'{' as i32);
    crate::input::begin_token_list_in(st, q.0, INSERTED)?;

    let old_mode = st.input.cur_list.mode;
    st.input.cur_list.mode = 0;
    st.scanner.cur_cs = st.shipout.write_loc;
    crate::scan::scan_toks_in(st, false, true)?;
    crate::scan::get_token_in(st)?;
    if st.scanner.cur_tok != CS_TOKEN_FLAG + END_WRITE {
        crate::output::error_here_with_diagnostic_in(st, c"Unbalanced write command".as_ptr());
        crate::output::capture_to_diagnostic_in(st, core::ptr::null_mut());
        st.errors.help_ptr = 2;
        st.errors.help_line[1] =
            c"On this page there's a \\write with fewer real {'s than }'s.".as_ptr();
        st.errors.help_line[0] = c"I can't handle that very well; good luck.".as_ptr();
        ship_err(crate::errors::error_in(st))?;
        loop {
            crate::scan::get_token_in(st)?;
            if st.scanner.cur_tok == CS_TOKEN_FLAG + END_WRITE {
                break;
            }
        }
    }
    st.input.cur_list.mode = old_mode;
    crate::input::end_token_list_in(st)?;

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
        crate::output::print_nl_cstr_in(st, c"".as_ptr());
    }
    let def_ref = st.scanner.def_ref;
    crate::output::token_show_in(st, def_ref.0);
    crate::output::print_ln_in(st);
    crate::mem::flush_list(&mut st.mem, def_ref);

    if j == 18 {
        st.shipout.selector = log_selector(&st.eqtb);
        if !st.errors.log_opened {
            st.shipout.selector = SELECTOR_TERM_ONLY;
        }
        // Tectonic leaves shell-escape warnings to the driver when it is enabled.
        if !st.config.shell_escape_enabled {
            crate::output::diagnostic_begin_capture_warning_here_in(st);
            crate::output::print_nl_cstr_in(st, c"runsystem(".as_ptr());
            let base = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            for k in base..st.strings.pool_ptr {
                crate::output::print_in(st, st.strings.str_pool[k as usize] as i32);
            }
            crate::output::print_cstr_in(st, c")...".as_ptr());
            crate::output::print_cstr_in(st, c"disabled".as_ptr());
            crate::output::print_char_in(st, b'.' as i32);
            crate::output::capture_to_diagnostic_in(st, core::ptr::null_mut());
            crate::output::print_nl_cstr_in(st, c"".as_ptr());
            crate::output::print_ln_in(st);
        } else {
            let base = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            let cmd = &st.strings.str_pool[base as usize..st.strings.pool_ptr as usize];
            // SAFETY: FFI hands the command's UTF-16 units to the driver.
            unsafe { ttstub_shell_escape(cmd.as_ptr(), cmd.len()) };
        }
        st.strings.pool_ptr = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
    }
    st.shipout.selector = old_setting;
    Ok(())
}

fn pic_out(
    st: &mut crate::state::EngineState,
    pic: NodePtr<WhatsitNode>,
) -> Result<(), ShipoutAbort> {
    synch_h(st)?;
    synch_v(st)?;
    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    crate::output::print_cstr_in(st, c"pdf:image ".as_ptr());
    crate::output::print_cstr_in(st, c"matrix ".as_ptr());
    for v in pic.pic_transform(&st.mem) {
        crate::output::print_scaled_in(st, v);
        crate::output::print_in(st, b' ' as i32);
    }
    crate::output::print_cstr_in(st, c"page ".as_ptr());
    crate::output::print_int_in(st, pic.pic_page(&st.mem) as i32);
    crate::output::print_in(st, b' ' as i32);
    if let Some(label) =
        crate::pic::PageboxType::from_raw(pic.pic_pagebox(&st.mem)).and_then(|pb| pb.label())
    {
        crate::output::print_cstr_in(st, label.as_ptr());
    }
    crate::output::print_in(st, b'(' as i32);
    let path = pic.pic_path(&st.mem).to_vec();
    for byte in path {
        crate::output::print_raw_char_in(st, byte as u16, true);
    }
    crate::output::print_in(st, b')' as i32);
    st.shipout.selector = old_setting;
    dvi_special_from_cur_string(st)
}

/// `out_what` for the node at the tail of the current list (`\immediate`).
pub(crate) fn out_what_in(st: &mut crate::state::EngineState, p: i32) -> Result<(), ShipoutAbort> {
    let whatsit = st
        .mem
        .classify_whatsit(MemIdx::new(p))
        .expect("valid whatsit")
        .expect("immediate whatsit");
    out_what(st, whatsit)
}

fn out_what(
    st: &mut crate::state::EngineState,
    w: NodePtr<WhatsitNode>,
) -> Result<(), ShipoutAbort> {
    let subtype = w.whatsit_subtype(&st.mem);
    match subtype {
        Some(WhatsitSubtype::Open | WhatsitSubtype::Write | WhatsitSubtype::Close) => {
            if !st.shipout.doing_leaders {
                out_write_stream(st, w, subtype)?;
            }
        }
        Some(WhatsitSubtype::Special) => special_out(st, w)?,
        Some(WhatsitSubtype::Language) => {}
        _ => return Err(ShipoutAbort::Confusion(c"ext4")),
    }
    Ok(())
}

fn out_write_stream(
    st: &mut crate::state::EngineState,
    w: NodePtr<WhatsitNode>,
    subtype: Option<WhatsitSubtype>,
) -> Result<(), ShipoutAbort> {
    if subtype == Some(WhatsitSubtype::Write) {
        return write_out(st, w);
    }
    let j = w.write_stream(&st.mem);
    let stream = j as usize;
    if st.shipout.write_open[stream] {
        ensure_write_stream_open(st, stream)?;
        // SAFETY: FFI close of the write-stream output handle.
        unsafe { ttstub_output_close(st.shipout.write_file[stream]) };
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
    if crate::string_pool::length(&st.strings, st.scanner.cur_ext) == 0 {
        // SAFETY: FFI string interning.
        st.scanner.cur_ext =
            unsafe { crate::texmfmp::maketexstring_in(&mut st.strings, c".tex".as_ptr()) }?;
    }
    let (n, a, e) = (st.scanner.cur_name, st.scanner.cur_area, st.scanner.cur_ext);
    crate::io::pack_file_name_in(st, n, a, e);
    // SAFETY: FFI open with the packed name buffer.
    let handle = unsafe { ttstub_output_open(st.input.name_of_file.as_ptr(), 0) };
    if handle == INVALID_HANDLE {
        return Err(ShipoutAbort::CannotOpenOutput);
    }
    st.shipout.write_file[stream] = handle;
    st.shipout.write_open[stream] = true;
    let name_len = st.input.name_length as usize;
    st.shipout.write_name[stream].clear();
    st.shipout.write_name[stream].extend_from_slice(&st.input.name_of_file[..=name_len]);
    st.shipout.write_prefix[stream].clear();
    if st.errors.log_opened {
        let old_setting = st.shipout.selector;
        st.shipout.selector = log_selector(&st.eqtb);
        crate::output::print_nl_cstr_in(st, c"\\openout".as_ptr());
        crate::output::print_int_in(st, j);
        crate::output::print_cstr_in(st, c" = `".as_ptr());
        crate::output::print_file_name_in(st, n, a, e);
        crate::output::print_cstr_in(st, c"'.".as_ptr());
        crate::output::print_nl_cstr_in(st, c"".as_ptr());
        crate::output::print_ln_in(st);
        st.shipout.selector = old_setting;
    }
    Ok(())
}

/// Lazily recreate a checkpoint-restored TeX output stream and its prefix.
pub(crate) fn ensure_write_stream_open(
    st: &mut crate::state::EngineState,
    stream: usize,
) -> Result<(), ShipoutAbort> {
    if st.shipout.write_file[stream] != INVALID_HANDLE {
        return Ok(());
    }
    let name = &st.shipout.write_name[stream];
    if name.last() != Some(&0) {
        return Err(ShipoutAbort::CannotOpenOutput);
    }
    let handle = unsafe { ttstub_output_open(name.as_ptr(), 0) };
    if handle == INVALID_HANDLE {
        return Err(ShipoutAbort::CannotOpenOutput);
    }
    for &code_unit in &st.shipout.write_prefix[stream] {
        unsafe { crate::bridge::ttstub_output_putc(handle, code_unit) };
    }
    st.shipout.write_file[stream] = handle;
    Ok(())
}
