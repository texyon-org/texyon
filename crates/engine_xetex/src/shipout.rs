//! Shipout helpers translated from `xetex/xetex-shipout.c`.
use crate::consts::{
    A_LEADERS, BEFORE, COUNT_BASE, CS_TOKEN_FLAG, DIMEN_BASE, DLIST, FONT_BASE, GLUE_NODE,
    GLUE_SPEC_SIZE, HLIST_NODE, INSERTED, INT_BASE, INT_PAR_MAG, INT_PAR_TRACING_ONLINE,
    INVALID_HANDLE, KERN_NODE, LEFT_TO_RIGHT, LIGATURE_NODE, MATH_NODE, MAX_HALFWORD,
    MEDIUM_NODE_SIZE, MEM_TOP, MIN_HALFWORD, NATIVE_GLYPH_INFO_SIZE, NULL_FLAG, PENALTY_NODE,
    PIC_NODE_SIZE, RIGHT_TO_LEFT, RULE_NODE, SELECTOR_LOG_ONLY, SELECTOR_NEW_STRING,
    SELECTOR_TERM_AND_LOG, SELECTOR_TERM_ONLY, SMALL_NODE_SIZE, SPACE_ADJUSTMENT, TEX_NULL,
    TOO_BIG_CHAR, WRITE_TEXT,
};
use crate::ext::apply_tfm_font_mapping;

use crate::bridge::{
    ttstub_output_close, ttstub_output_flush, ttstub_output_open, ttstub_output_write,
    ttstub_shell_escape,
};
use crate::engine::OUTPUT_FILE_EXTENSION;
use crate::node::{AnyBox, AnyNode, EdgeNode, MovementNode, NodePtr, WhatsitNode, WhatsitSubtype};
use crate::state::{EqtbState, FontState, MemState, ScannerState, ShipoutState, StringsState};
use crate::units::{GlueSign, MemIdx, MemPtr, Scaled};

// xetex-shipout.c:11–12
const DVI_BUF_SIZE: i32 = 16384;
const HALF_BUF: i32 = 8192;
const TEX_INFINITY: i32 = 0x7FFF_FFFF;
// xetex-constants.h:240
const POP: u8 = 142;
const PRE: u8 = 247;
const BOP: u8 = 139;
const PUSH: u8 = 141;
const SET1: u8 = 128;
const SET_RULE: u8 = 132;
const PUT_RULE: u8 = 137;
const FNT1: u8 = 235;
const SET_GLYPHS: u8 = 253;
const SET_TEXT_AND_GLYPHS: u8 = 254;
const FNT_NUM_0: u8 = 171;

// xetex-constants.h:105,114. EDGE_NODE is an alias of STYLE_NODE.
const VLIST_NODE: u16 = 1;
const INS_NODE: u16 = 3;
const MARK_NODE: u16 = 4;
const ADJUST_NODE: u16 = 5;
const WHATSIT_NODE: u16 = 8;
const EDGE_NODE: u16 = 14;
const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 65534;
const FNT_DEF1: u8 = 243;
const DEFINE_NATIVE_FONT: u8 = 252;

// DVI movement opcodes — xetex-constants.h:241–242,244–245
const DOWN1: u8 = 157;
const RIGHT1: u8 = 143;
const XXX1: u8 = 239;
const XXX4: u8 = 242;

// Movement node states — xetex-constants.h:393–401
const MOV_NONE_SEEN: i32 = 0;
const MOV_Y_HERE: i32 = 1;
const MOV_Z_HERE: i32 = 2;
const MOV_YZ_OK: i32 = 3;
const MOV_Y_OK: i32 = 4;
const MOV_Z_OK: i32 = 5;
const MOV_Y_SEEN: i32 = 6;
const MOV_D_FIXED: i32 = 6;
const MOV_Z_SEEN: i32 = 12;

// Glue and leader constants: xetex-constants.h:175,182,275,281; xetex_format.h:422.
const FILLL: u16 = 3;
const NORMAL: u16 = 0;
const C_LEADERS: u16 = 101;

// MATH_NODE LR constants: xetex-constants.h:142-143; xetex_format.h:437.
const L_CODE: u16 = 4;
const R_CODE: u16 = 8;
const END_M_CODE: u16 = 3;
const REVERSED: u16 = 1;
const LIG_TRICK: i32 = 4_999_987;

const INT_PAR_XETEX_INTERWORD_SPACE_SHAPING: i32 = 78;

// Token constants — xetex-constants.h:204,217,348–351
const LEFT_BRACE_TOKEN: i32 = 0x20_0000;
const RIGHT_BRACE_TOKEN: i32 = 0x40_0000;
// END_WRITE = FROZEN_CONTROL_SEQUENCE + 8 = 2_243_226 + 8
const END_WRITE: i32 = 2_243_234;

/// Create a new edge node of subtype `s` and width `w`. `None` is the
/// arena-exhaustion sentinel.
///
/// translated from xetex-shipout.c:1708 (new_edge)
pub fn new_edge(mem: &mut MemState, s: i16, w: i32) -> Option<NodePtr<EdgeNode>> {
    let p = NodePtr::<EdgeNode>::alloc(mem)?;
    p.set_subtype(mem, s as u16);
    p.set_width(mem, Scaled(w));
    p.set_edge_dist(mem, Scaled(0));
    Some(p)
}

enum DviFlush {
    /// Flush `dvi_buf[a..=b]` to the output file.
    Write(i32, i32),
    /// DVI length overflowed 0x7FFFFFFF — abort via `fatal_error`.
    Fatal,
}

pub(crate) enum ShipoutAbort {
    /// `confusion("LR2")` — internal LR-consistency failure in `reverse`.
    ConfusionLr2,
    /// `overflow("pool size", …)` — string-pool exhaustion in word merging.
    PoolSizeOverflow,
    /// A node-allocation overflow bubbled up from `new_native_word_node`.
    NodeRecycle(crate::scan::NodeRecycleAbort),
    /// A `confusion(msg)` bubbled up from `flush_node_list`.
    Confusion(&'static std::ffi::CStr),
    /// `_tt_abort` — short write to the XDV file (I/O failure flush).
    DviWriteFailed,
    /// `fatal_error` — DVI length exceeded 0x7FFFFFFF.
    DviLengthOverflow,
    /// `overflow("main memory size", …)` — movement-node allocation failure.
    MainMemoryOverflow,
    /// A scanner/tokenizer cold tail bubbled up from a SCC core
    /// (`scan_toks_in`/`get_token_in`/…) called from `write_out`.
    Scc(crate::scan::SccAbort),
    /// `_tt_abort("cannot open output file …")`
    CannotOpenOutput,
    /// A file-name pool overflow bubbled up from `make_name_string_in`.
    Name(crate::io::NameAbort),
    /// A native-font measurement cold abort (reached from `make_font_def`).
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

/// Propagate a recoverable-error outcome as a [`ShipoutAbort`]: `Continue`
/// keeps going, anything else became terminal.
fn ship_err(o: crate::errors::ErrorOutcome) -> Result<(), ShipoutAbort> {
    crate::scan::scc_err(o).map_err(ShipoutAbort::Scc)
}

/// `get_avail` with its exhaustion `None` lifted into the
/// movement/main-memory overflow tail.
fn get_avail_ship(mem: &mut MemState) -> Result<crate::units::MemPtr, ShipoutAbort> {
    crate::mem::get_avail(mem).ok_or(ShipoutAbort::MainMemoryOverflow)
}

/// `get_node` with its exhaustion `None` lifted into the
/// main-memory overflow tail.
fn get_node_ship(mem: &mut MemState, s: i32) -> Result<i32, ShipoutAbort> {
    let p = crate::mem::get_node(mem, s).ok_or(ShipoutAbort::MainMemoryOverflow)?;
    Ok(p.0)
}

impl ShipoutAbort {
    /// Map to the unified [`EngineAbort`] terminal. `MainMemoryOverflow`
    /// is a plain overflow (no `runaway`). `CannotOpenOutput` rebuilds the message
    /// from `name_of_file()` into a leaked heap buffer so the
    /// pointer stays valid through `into_history`.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            ShipoutAbort::ConfusionLr2 => crate::errors::EngineAbort::Confusion(c"LR2".as_ptr()),
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
            ShipoutAbort::CannotOpenOutput => {
                // The message is formatted in `into_history` from
                // `st.input.name_of_file`.
                crate::errors::EngineAbort::CannotOpenOutput
            }
            ShipoutAbort::Name(e) => e.into_engine_abort(),
            ShipoutAbort::NativeFont(e) => e.into_engine_abort(),
        }
    }
}

/// Flush `dvi_buf[a..=b]` to the
/// output file by reading the buffer fields directly.
fn write_to_dvi(shipout: &ShipoutState, a: i32, b: i32) -> Result<(), ShipoutAbort> {
    let n = (b - a + 1) as libc::size_t;
    let file = shipout.dvi_file;
    // SAFETY: the buffer Vec never reallocates; `a..=b` is a valid sub-range.
    let ptr = unsafe { shipout.dvi_buf.as_ptr().add(a as usize) };
    // SAFETY: FFI write of `n` bytes; does not re-enter the engine state.
    if unsafe { ttstub_output_write(file, ptr as *const libc::c_char, n) } != n {
        return Err(ShipoutAbort::DviWriteFailed);
    }
    Ok(())
}

/// Perform a deferred DVI buffer flush.
fn handle_dvi_flush(shipout: &ShipoutState, flush: Option<DviFlush>) -> Result<(), ShipoutAbort> {
    match flush {
        None => Ok(()),
        Some(DviFlush::Write(a, b)) => write_to_dvi(shipout, a, b),
        Some(DviFlush::Fatal) => Err(ShipoutAbort::DviLengthOverflow),
    }
}

fn emit_dvi_out(shipout: &mut ShipoutState, c: u8) -> Result<(), ShipoutAbort> {
    let flush = dvi_out(shipout, c);
    handle_dvi_flush(shipout, flush)
}

fn emit_dvi_four(shipout: &mut ShipoutState, x: i32) -> Result<(), ShipoutAbort> {
    let mut start = 0;
    while start < 4 {
        let result = dvi_four(shipout, x, start);
        start = result.next;
        handle_dvi_flush(shipout, result.flush)?;
    }
    Ok(())
}

fn emit_dvi_two(shipout: &mut ShipoutState, s: u16) -> Result<(), ShipoutAbort> {
    let mut start = 0;
    while start < 2 {
        let result = dvi_two(shipout, s, start);
        start = result.next;
        handle_dvi_flush(shipout, result.flush)?;
    }
    Ok(())
}

/// Build the movement bytes and emit them.
fn emit_movement(
    mem: &mut MemState,
    shipout: &mut ShipoutState,
    w: i32,
    o: u8,
) -> Result<(), ShipoutAbort> {
    let bytes = movement(mem, shipout, w, o);
    if bytes.is_empty() {
        return Err(ShipoutAbort::MainMemoryOverflow);
    }
    for byte in bytes {
        emit_dvi_out(shipout, byte)?;
    }
    Ok(())
}

struct DviEmit {
    next: usize,
    flush: Option<DviFlush>,
}

fn emit_dvi_bytes(shipout: &mut ShipoutState, bytes: &[u8], start: usize) -> DviEmit {
    for (idx, byte) in bytes.iter().enumerate().skip(start) {
        if let Some(flush) = dvi_out(shipout, *byte) {
            return DviEmit {
                next: idx + 1,
                flush: Some(flush),
            };
        }
    }

    DviEmit {
        next: bytes.len(),
        flush: None,
    }
}

fn dvi_four_bytes(mut x: i32) -> [u8; 4] {
    let first = if x >= 0 {
        (x / 0x1000000) as u8
    } else {
        x += 0x40000000;
        x += 0x40000000;
        ((x / 0x1000000) + 128) as u8
    };

    x %= 0x1000000;
    let second = (x / 0x10000) as u8;

    x %= 0x10000;
    [first, second, (x / 0x100) as u8, (x % 0x100) as u8]
}

/// translated from xetex-shipout.c:2403 (dvi_four)
fn dvi_four(shipout: &mut ShipoutState, x: i32, start: usize) -> DviEmit {
    let bytes = dvi_four_bytes(x);
    emit_dvi_bytes(shipout, &bytes, start)
}

/// translated from xetex-shipout.c:2423 (dvi_two)
fn dvi_two(shipout: &mut ShipoutState, s: u16, start: usize) -> DviEmit {
    emit_dvi_bytes(shipout, &[(s / 0x100) as u8, (s % 0x100) as u8], start)
}

fn strings_length(strings: &StringsState, s: i32) -> i32 {
    if s >= TOO_BIG_CHAR {
        return strings.str_start[(s + 1 - TOO_BIG_CHAR) as usize]
            - strings.str_start[(s - TOO_BIG_CHAR) as usize];
    }
    if (32..127).contains(&s) {
        return 1;
    }
    if s <= 127 {
        return 3;
    }
    if s < 256 {
        return 4;
    }
    5
}

/// translated from xetex-shipout.c:1784 (dvi_native_font_def)
fn dvi_native_font_def(fonts: &FontState, f: i32, font_def_length: i32) -> Vec<u8> {
    let mut bytes = vec![DEFINE_NATIVE_FONT];
    bytes.extend_from_slice(&dvi_four_bytes(f - 1));

    for i in 0..font_def_length {
        bytes.push(fonts.xdv_buffer[i as usize]);
    }
    bytes
}

fn dvi_font_def_bytes(
    fonts: &FontState,
    strings: &StringsState,
    f: i32,
    font_area: i32,
    font_area_len: i32,
    font_name: i32,
    font_name_len: i32,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    if f <= 256 {
        bytes.push(FNT_DEF1);
        bytes.push((f - 1) as u8);
    } else {
        bytes.push(FNT_DEF1 + 1);
        bytes.push(((f - 1) / 256) as u8);
        bytes.push(((f - 1) % 256) as u8);
    }

    let font_check = fonts.font_check[f as usize];
    bytes.push(font_check.s3 as u8);
    bytes.push(font_check.s2 as u8);
    bytes.push(font_check.s1 as u8);
    bytes.push(font_check.s0 as u8);
    bytes.extend_from_slice(&dvi_four_bytes(fonts.font_size[f as usize]));
    bytes.extend_from_slice(&dvi_four_bytes(fonts.font_dsize[f as usize]));
    bytes.push(font_area_len as u8);

    let mut l = 0;
    let mut k = strings.str_start[(font_name - TOO_BIG_CHAR) as usize];

    while l == 0 && k < strings.str_start[(font_name + 1 - TOO_BIG_CHAR) as usize] {
        if strings.str_pool[k as usize] == b':' as u16 {
            l = k - strings.str_start[(font_name - TOO_BIG_CHAR) as usize];
        }
        k += 1;
    }

    if l == 0 {
        l = font_name_len;
    }

    bytes.push(l as u8);

    k = strings.str_start[(font_area - TOO_BIG_CHAR) as usize];
    let mut for_end = strings.str_start[(font_area + 1 - TOO_BIG_CHAR) as usize] - 1;
    while k <= for_end {
        bytes.push(strings.str_pool[k as usize] as u8);
        k += 1;
    }

    k = strings.str_start[(font_name - TOO_BIG_CHAR) as usize];
    for_end = strings.str_start[(font_name - TOO_BIG_CHAR) as usize] + l - 1;
    while k <= for_end {
        bytes.push(strings.str_pool[k as usize] as u8);
        k += 1;
    }
    bytes
}

fn dvi_font_select_bytes(f: i32) -> Vec<u8> {
    if f <= 64 {
        vec![(f as u8) + FNT_NUM_0 - 1]
    } else if f <= 256 {
        vec![FNT1, (f - 1) as u8]
    } else {
        vec![FNT1 + 1, ((f - 1) / 256) as u8, ((f - 1) % 256) as u8]
    }
}

// translated from xetex-shipout.c:48 (initialize_shipout_variables)
pub fn initialize_shipout_variables(st: &mut crate::state::EngineState) {
    let s = &mut *st;
    s.shipout.output_file_name = 0;
    s.shipout.dvi_buf = vec![0u8; (DVI_BUF_SIZE + 1) as usize];
    s.shipout.dvi_limit = DVI_BUF_SIZE;
    s.shipout.dvi_ptr = 0;
    s.shipout.dvi_offset = 0;
    s.shipout.dvi_gone = 0;
    s.shipout.down_ptr.0 = TEX_NULL;
    s.shipout.right_ptr.0 = TEX_NULL;
    s.shipout.cur_s = -1;
}

// translated from xetex-shipout.c:2306 (dvi_swap)
//
// The state updates are reordered before the (deferred) flush, which
// is sound: `dvi_limit/offset/ptr/gone` do not affect the bytes being written.
fn dvi_swap(shipout: &mut ShipoutState) -> DviFlush {
    if shipout.dvi_ptr > TEX_INFINITY - shipout.dvi_offset {
        shipout.cur_s = -2;
        return DviFlush::Fatal;
    }
    let range = if shipout.dvi_limit == DVI_BUF_SIZE {
        shipout.dvi_limit = HALF_BUF;
        shipout.dvi_offset += DVI_BUF_SIZE;
        shipout.dvi_ptr = 0;
        DviFlush::Write(0, HALF_BUF - 1)
    } else {
        shipout.dvi_limit = DVI_BUF_SIZE;
        DviFlush::Write(HALF_BUF, DVI_BUF_SIZE - 1)
    };
    shipout.dvi_gone += HALF_BUF;
    range
}

// translated from xetex-shipout.c:71 (dvi_out)
//
// Byte write. Returns the deferred flush action
// (if the buffer filled).
fn dvi_out(shipout: &mut ShipoutState, c: u8) -> Option<DviFlush> {
    shipout.page_manifest.record_byte(c);
    let ptr = shipout.dvi_ptr;
    shipout.dvi_buf[ptr as usize] = c;
    shipout.dvi_ptr += 1;
    if shipout.dvi_ptr == shipout.dvi_limit {
        Some(dvi_swap(shipout))
    } else {
        None
    }
}

// translated from xetex-shipout.c:56 (ship_out)
pub(crate) unsafe fn ship_out_in(
    st: &mut crate::state::EngineState,
    p: i32,
) -> Result<(), ShipoutAbort> {
    let mag = st.eqtb.b32s1(INT_BASE + INT_PAR_MAG);
    let synctex_job_name = st.strings.job_name;
    let synctex_tmp = crate::texmfmp::gettexstring_in(&st.strings, synctex_job_name);
    crate::synctex::synctex_sheet(st, mag, synctex_tmp);

    if st.strings.job_name == 0 {
        crate::io::open_log_file_in(st)?;
    }

    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) > 0 {
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
    let mut j: u8 = 9;
    while j > 0 && st.eqtb.b32s1(COUNT_BASE + j as i32) == 0 {
        j -= 1;
    }

    for k in 0..=j {
        crate::output::print_int_in(st, st.eqtb.b32s1(COUNT_BASE + k as i32));
        if k < j {
            crate::output::print_char_in(st, b'.' as i32);
        }
    }

    ttstub_output_flush(st.shipout.rust_stdout);

    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) > 0 {
        crate::output::print_char_in(st, b']' as i32);
        crate::output::begin_diagnostic_in(st);
        crate::output::show_box_in(st, p);
        crate::output::end_diagnostic_in(st, true);
    }

    let p_box = st
        .mem
        .classify_box(MemIdx::new(p))
        .expect("valid shipped box")
        .expect("shipped box");
    if p_box.height(&st.mem).0 > MAX_HALFWORD
        || p_box.depth(&st.mem).0 > MAX_HALFWORD
        || p_box.height(&st.mem).0
            + p_box.depth(&st.mem).0
            + st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET)
            > MAX_HALFWORD
        || p_box.width(&st.mem).0 + st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET) > MAX_HALFWORD
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

        if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) <= 0 {
            crate::output::begin_diagnostic_in(st);
            crate::output::print_nl_cstr_in(st, c"The following box has been deleted:".as_ptr());
            crate::output::show_box_in(st, p);
            crate::output::end_diagnostic_in(st, true);
        }
    } else {
        let page_height = p_box.height(&st.mem).0
            + p_box.depth(&st.mem).0
            + st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET);
        if page_height > st.shipout.max_v {
            st.shipout.max_v = page_height;
        }

        let page_width = p_box.width(&st.mem).0 + st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET);
        if page_width > st.shipout.max_h {
            st.shipout.max_h = page_width;
        }

        st.shipout.dvi_h = 0;
        st.shipout.dvi_v = 0;
        st.shipout.cur_h = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET);
        st.shipout.dvi_f = FONT_BASE;

        st.shipout.cur_h_offset = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET) + ONE_INCH;
        st.shipout.cur_v_offset = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET) + ONE_INCH;

        if st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH) != 0 {
            st.shipout.cur_page_width = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH);
        } else {
            st.shipout.cur_page_width = p_box.width(&st.mem).0
                + 2 * (st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET) + ONE_INCH);
        }

        if st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT) != 0 {
            st.shipout.cur_page_height = st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT);
        } else {
            st.shipout.cur_page_height = p_box.height(&st.mem).0
                + p_box.depth(&st.mem).0
                + 2 * (st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET) + ONE_INCH);
        }

        if st.shipout.output_file_name == 0 {
            if st.strings.job_name == 0 {
                crate::io::open_log_file_in(st)?;
            }
            crate::io::pack_job_name_in(st, OUTPUT_FILE_EXTENSION)?;
            st.shipout.dvi_file = ttstub_output_open(st.input.name_of_file.as_ptr(), 0);
            if st.shipout.dvi_file == INVALID_HANDLE {
                return Err(ShipoutAbort::CannotOpenOutput);
            }
            st.shipout.output_file_name = crate::io::make_name_string_in(st)?;
        }

        if st.shipout.total_pages == 0 {
            emit_dvi_out(&mut st.shipout, PRE)?;
            if st.config.semantic_pagination_enabled {
                emit_dvi_out(&mut st.shipout, SPX_ID_BYTE)?;
            } else {
                emit_dvi_out(&mut st.shipout, XDV_ID_BYTE)?;
            }
            emit_dvi_four(&mut st.shipout, 25_400_000)?;
            emit_dvi_four(&mut st.shipout, 473_628_672)?;
            ship_err(crate::save::prepare_mag_in(st))?;
            emit_dvi_four(&mut st.shipout, st.eqtb.b32s1(INT_BASE + INT_PAR_MAG))?;

            let output_comment = c"tectonic";
            let l = output_comment.to_bytes().len() as u8;
            emit_dvi_out(&mut st.shipout, l)?;
            for s in 0..l {
                emit_dvi_out(
                    &mut st.shipout,
                    *output_comment.as_ptr().add(s as usize) as u8,
                )?;
            }
        }

        let page_loc = st.shipout.dvi_offset + st.shipout.dvi_ptr;
        st.shipout.page_manifest.begin_page(page_loc as u64);
        emit_dvi_out(&mut st.shipout, BOP)?;
        for k in 0..10 {
            emit_dvi_four(&mut st.shipout, st.eqtb.b32s1(COUNT_BASE + k))?;
        }
        let last_bop = st.shipout.last_bop;
        emit_dvi_four(&mut st.shipout, last_bop)?;
        st.shipout.last_bop = page_loc;

        let old_setting = st.shipout.selector;
        st.shipout.selector = SELECTOR_NEW_STRING;
        crate::output::print_cstr_in(st, c"pdf:pagesize ".as_ptr());
        if st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH) <= 0
            || st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT) <= 0
        {
            crate::output::print_cstr_in(st, c"default".as_ptr());
        } else {
            crate::output::print_cstr_in(st, c"width".as_ptr());
            crate::output::print_in(st, b' ' as i32);
            crate::output::print_scaled_in(
                st,
                st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH),
            );
            crate::output::print_cstr_in(st, c"pt".as_ptr());
            crate::output::print_in(st, b' ' as i32);
            crate::output::print_cstr_in(st, c"height".as_ptr());
            crate::output::print_in(st, b' ' as i32);
            crate::output::print_scaled_in(
                st,
                st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT),
            );
            crate::output::print_cstr_in(st, c"pt".as_ptr());
        }
        st.shipout.selector = old_setting;

        emit_dvi_out(&mut st.shipout, XXX1)?;
        emit_dvi_out(&mut st.shipout, crate::hash::cur_length(&st.strings) as u8)?;
        let start = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
        let end = st.strings.pool_ptr;
        for s in start..end {
            emit_dvi_out(&mut st.shipout, st.strings.str_pool[s as usize] as u8)?;
        }
        st.strings.pool_ptr = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];

        st.shipout.cur_v = p_box.height(&st.mem).0 + st.eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET);
        st.mem.temp_ptr.0 = p;
        if matches!(p_box, AnyBox::VList(_)) {
            vlist_out_in(st)?;
        } else {
            hlist_out_in(st)?;
        }

        emit_dvi_out(&mut st.shipout, EOP)?;
        st.shipout
            .page_manifest
            .finish_page(st.scanner.logical_clock);
        st.shipout.total_pages = st.shipout.total_pages + 1;
        st.shipout.cur_s = -1;
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

    if st.scanner.lr_ptr.0 != TEX_NULL || st.scanner.cur_dir != LEFT_TO_RIGHT {
        return Err(ShipoutAbort::Confusion(c"LR3"));
    }

    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) <= 0 {
        crate::output::print_char_in(st, b']' as i32);
    }

    st.shipout.dead_cycles = 0;
    ttstub_output_flush(st.shipout.rust_stdout);
    crate::packaging::flush_node_list(&mut st.mem, p).map_err(ShipoutAbort::Confusion)?;
    crate::synctex::synctex_teehs(st);

    Ok(())
}

// translated from xetex-shipout.c:2332 (dvi_pop)
pub fn dvi_pop(shipout: &mut ShipoutState, l: i32) -> bool {
    if l == shipout.dvi_offset + shipout.dvi_ptr && shipout.dvi_ptr > 0 {
        shipout.dvi_ptr -= 1;
        shipout.page_manifest.pop_byte();
        false
    } else {
        true
    }
}

/// Emit the font definition.
fn dvi_font_def_in(st: &mut crate::state::EngineState, f: i32) -> Result<(), ShipoutAbort> {
    let bytes = if st.fonts.font_area_entry(f) == AAT_FONT_FLAG
        || st.fonts.font_area_entry(f) == OTGR_FONT_FLAG
    {
        // SAFETY: FFI; fills st.fonts.xdv_buffer and returns the def length.
        let font_def_length = unsafe { crate::ext::make_font_def(&mut st.fonts, f) }?;
        dvi_native_font_def(&st.fonts, f, font_def_length)
    } else {
        let font_area = st.fonts.font_area_entry(f);
        let font_name = st.fonts.font_name[f as usize];
        dvi_font_def_bytes(
            &st.fonts,
            &st.strings,
            f,
            font_area,
            strings_length(&st.strings, font_area),
            font_name,
            strings_length(&st.strings, font_name),
        )
    };
    for byte in bytes {
        emit_dvi_out(&mut st.shipout, byte)?;
    }
    Ok(())
}

/// Select (defining if needed) the DVI font.
fn ensure_dvi_font_in(st: &mut crate::state::EngineState, f: i32) -> Result<(), ShipoutAbort> {
    if f == st.shipout.dvi_f {
        return Ok(());
    }
    if !st.fonts.font_used[f as usize] {
        dvi_font_def_in(st, f)?;
        st.fonts.font_used[f as usize] = true;
    }
    for byte in dvi_font_select_bytes(f) {
        emit_dvi_out(&mut st.shipout, byte)?;
    }
    st.shipout.dvi_f = f;
    Ok(())
}

#[inline]
fn is_ignorable_interword_node(mem: &MemState, p: impl crate::units::MemAddr) -> bool {
    let p = p.addr();
    !crate::mem::is_char_node(mem, p)
        && (mem.b16s1(p) == PENALTY_NODE
            || mem.b16s1(p) == INS_NODE
            || mem.b16s1(p) == MARK_NODE
            || mem.b16s1(p) == ADJUST_NODE
            || (mem.b16s1(p) == WHATSIT_NODE
                && WhatsitSubtype::from_raw(mem.b16s0(p)).is_some_and(|s| s.is_io())))
}

fn skip_ignorable_interword_nodes(mem: &MemState, q: impl crate::units::MemAddr) -> MemPtr {
    let mut q = MemPtr::from_raw(q.addr());
    while !q.is_null() && is_ignorable_interword_node(mem, q) {
        q = q.link(mem);
    }
    q
}

fn merge_justifiable_native_words(
    mem: &mut MemState,
    strings: &mut StringsState,
    fonts: &FontState,
    eqtb: &EqtbState,
    this_box: i32,
    g_order: u16,
    g_sign: GlueSign,
) -> Result<(), ShipoutAbort> {
    let mut p = MemPtr::from_raw(this_box + 5).link(mem);
    let mut prev_p = MemPtr::from_raw(this_box + 5);

    while !p.is_null() {
        if !p.link(mem).is_null()
            && !crate::mem::is_char_node(mem, p)
            && mem.b16s1(p) == WHATSIT_NODE
            && WhatsitSubtype::from_raw(mem.b16s0(p)).is_some_and(|s| s.is_native_word())
            && fonts.font_letter_space_entry(mem.b16s2(p + 4) as i32) == 0
        {
            let r = p;
            let mut k = mem.b16s1(r + 4) as i32;
            let mut q = p.link(mem);

            'check_next: loop {
                q = skip_ignorable_interword_nodes(mem, q);

                if !q.is_null() && !crate::mem::is_char_node(mem, q) {
                    if mem.b16s1(q) == GLUE_NODE && mem.b16s0(mem.b32s0(q + 1)) == NORMAL {
                        if mem.b32s0(q + 1) == fonts.font_glue(mem.b16s2(r + 4) as i32) {
                            q = skip_ignorable_interword_nodes(mem, q.link(mem));

                            if !q.is_null()
                                && !crate::mem::is_char_node(mem, q)
                                && mem.b16s1(q) == WHATSIT_NODE
                                && WhatsitSubtype::from_raw(mem.b16s0(q))
                                    .is_some_and(|s| s.is_native_word())
                                && mem.b16s2(q + 4) == mem.b16s2(r + 4)
                            {
                                p = q;
                                k += 1 + mem.b16s1(q + 4) as i32;
                                q = q.link(mem);
                                continue 'check_next;
                            }
                        } else {
                            q = q.link(mem);
                        }

                        if !q.is_null()
                            && !crate::mem::is_char_node(mem, q)
                            && mem.b16s1(q) == KERN_NODE
                            && mem.b16s0(q) == SPACE_ADJUSTMENT
                        {
                            q = skip_ignorable_interword_nodes(mem, q.link(mem));

                            if !q.is_null()
                                && !crate::mem::is_char_node(mem, q)
                                && mem.b16s1(q) == WHATSIT_NODE
                                && WhatsitSubtype::from_raw(mem.b16s0(q))
                                    .is_some_and(|s| s.is_native_word())
                                && mem.b16s2(q + 4) == mem.b16s2(r + 4)
                            {
                                p = q;
                                k += 1 + mem.b16s1(q + 4) as i32;
                                q = q.link(mem);
                                continue 'check_next;
                            }
                        }
                    } else if mem.b16s1(q) == WHATSIT_NODE
                        && WhatsitSubtype::from_raw(mem.b16s0(q))
                            .is_some_and(|s| s.is_native_word())
                        && mem.b16s2(q + 4) == mem.b16s2(r + 4)
                    {
                        p = q;
                        q = q.link(mem);
                        continue 'check_next;
                    }
                }

                break;
            }

            if p != r {
                if strings.pool_ptr + k > strings.pool_size {
                    return Err(ShipoutAbort::PoolSizeOverflow);
                }

                k = 0;
                q = r;
                loop {
                    if mem.b16s1(q) == WHATSIT_NODE {
                        if WhatsitSubtype::from_raw(mem.b16s0(q))
                            .is_some_and(|s| s.is_native_word())
                        {
                            let text = mem.as_u16_ptr(q + 6);
                            for j in 0..(mem.b16s1(q + 4) as i32) {
                                // SAFETY: native-word text spans `mem[q+6..]`, length b16s1(q+4).
                                strings.str_pool[strings.pool_ptr as usize] =
                                    unsafe { *text.add(j as usize) };
                                strings.pool_ptr += 1;
                            }

                            k += mem.b32s1(q + 1);
                        }
                    } else if mem.b16s1(q) == GLUE_NODE {
                        strings.str_pool[strings.pool_ptr as usize] = b' ' as u16;
                        strings.pool_ptr += 1;
                        let g = mem.b32s0(q + 1);
                        k += mem.b32s1(g + 1);

                        if g_sign != GlueSign::Normal {
                            if g_sign == GlueSign::Stretching {
                                if mem.b16s1(g) == g_order {
                                    k += crate::scaled_math::tex_round(
                                        mem.gr(this_box + 6) * mem.b32s1(g + 2) as f64,
                                    );
                                }
                            } else if mem.b16s0(g) == g_order {
                                k -= crate::scaled_math::tex_round(
                                    mem.gr(this_box + 6) * mem.b32s1(g + 3) as f64,
                                );
                            }
                        }
                    } else if mem.b16s1(q) == KERN_NODE {
                        k += mem.b32s1(q + 1);
                    }

                    if q == p {
                        break;
                    }
                    q = q.link(mem);
                }

                let nlen = crate::hash::cur_length(strings);
                q = MemPtr::from_raw(
                    crate::fonts::new_native_word_node(mem, eqtb, mem.b16s2(r + 4) as i32, nlen)
                        .map_err(ShipoutAbort::NodeRecycle)?,
                );
                mem.set_b16s0(q, mem.b16s0(r));

                let base = strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize];
                // SAFETY: copying `nlen` UTF-16 codes from the pool tail into the
                // freshly-allocated native-word node text region.
                let src = unsafe { strings.str_pool.as_ptr().add(base as usize) };
                mem.journal_mark_range(q.0 + 6, crate::state::native_text_words(nlen));
                let dst = mem.as_u16_ptr_mut(q + 6);
                for j in 0..nlen {
                    // SAFETY: both regions hold at least `nlen` u16 codes.
                    unsafe { *dst.add(j as usize) = *src.add(j as usize) };
                }

                mem.set_b32s1(q + 1, k);
                unsafe { crate::ext::store_justified_native_glyphs(fonts, mem, q.0) }?;
                prev_p.set_link(mem, q);
                q.set_link(mem, p.link(mem));
                p.set_link(mem, MemPtr::NULL);
                prev_p = r;
                p = r.link(mem);

                while !p.is_null() {
                    if is_ignorable_interword_node(mem, p) {
                        prev_p.set_link(mem, p.link(mem));
                        p.set_link(mem, q.link(mem));
                        q.set_link(mem, p);
                        q = p;
                    }

                    prev_p = p;
                    p = p.link(mem);
                }

                crate::packaging::flush_node_list(mem, r).map_err(ShipoutAbort::Confusion)?;
                strings.pool_ptr = strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize];
                p = q;
            }
        }

        prev_p = p;
        p = p.link(mem);
    }

    Ok(())
}

struct HListRuleAction {
    emit: bool,
    h_move: Option<i32>,
    v_move: Option<i32>,
    rule_ht: i32,
    rule_wd: i32,
    synctex_p: MemPtr,
}

fn finish_hlist_rule(
    mem: &MemState,
    shipout: &mut ShipoutState,
    this_box: AnyBox,
    base_line: i32,
    p: MemPtr,
) -> HListRuleAction {
    if shipout.rule_ht == NULL_FLAG {
        shipout.rule_ht = this_box.height(mem).0;
    }
    if shipout.rule_dp == NULL_FLAG {
        shipout.rule_dp = this_box.depth(mem).0;
    }
    shipout.rule_ht += shipout.rule_dp;

    let mut emit = false;
    let mut h_move = None;
    let mut v_move = None;

    if shipout.rule_ht > 0 && shipout.rule_wd > 0 {
        emit = true;
        if shipout.cur_h != shipout.dvi_h {
            h_move = Some(shipout.cur_h - shipout.dvi_h);
            shipout.dvi_h = shipout.cur_h;
        }

        shipout.cur_v = base_line + shipout.rule_dp;
        if shipout.cur_v != shipout.dvi_v {
            v_move = Some(shipout.cur_v - shipout.dvi_v);
            shipout.dvi_v = shipout.cur_v;
        }

        shipout.cur_v = base_line;
        shipout.dvi_h += shipout.rule_wd;
    }

    shipout.cur_h += shipout.rule_wd;

    HListRuleAction {
        emit,
        h_move,
        v_move,
        rule_ht: shipout.rule_ht,
        rule_wd: shipout.rule_wd,
        synctex_p: p,
    }
}

fn finish_hlist_rule_in(
    st: &mut crate::state::EngineState,
    this_box: AnyBox,
    base_line: i32,
    p: MemPtr,
) -> Result<(), ShipoutAbort> {
    let action = finish_hlist_rule(&st.mem, &mut st.shipout, this_box, base_line, p);
    if action.emit {
        if let Some(h_move) = action.h_move {
            emit_movement(&mut st.mem, &mut st.shipout, h_move, RIGHT1)?;
        }
        if let Some(v_move) = action.v_move {
            emit_movement(&mut st.mem, &mut st.shipout, v_move, DOWN1)?;
        }
        emit_dvi_out(&mut st.shipout, SET_RULE)?;
        emit_dvi_four(&mut st.shipout, action.rule_ht)?;
        emit_dvi_four(&mut st.shipout, action.rule_wd)?;
    }
    // SAFETY: synctex bookkeeping over the shared engine state.
    unsafe {
        crate::synctex::synctex_horizontal_rule_or_glue(st, action.synctex_p);
    }
    Ok(())
}

unsafe fn output_hlist_glue_in(
    st: &mut crate::state::EngineState,
    this_box: AnyBox,
    base_line: i32,
    left_edge: i32,
    p: i32,
    g_order: u16,
    g_sign: GlueSign,
    cur_g: &mut i32,
    cur_glue: &mut f64,
) -> Result<(), ShipoutAbort> {
    let mut g = st.mem.b32s0(p + 1);
    st.shipout.rule_wd = st.mem.b32s1(g + 1) - *cur_g;

    if g_sign != GlueSign::Normal {
        if g_sign == GlueSign::Stretching {
            if st.mem.b16s1(g) == g_order {
                *cur_glue += st.mem.b32s1(g + 2) as f64;
                *cur_g = crate::scaled_math::tex_round(
                    (this_box.glue_set(&st.mem) * *cur_glue)
                        .clamp(-1_000_000_000.0, 1_000_000_000.0),
                );
            }
        } else if st.mem.b16s0(g) == g_order {
            *cur_glue -= st.mem.b32s1(g + 3) as f64;
            *cur_g = crate::scaled_math::tex_round(
                (this_box.glue_set(&st.mem) * *cur_glue).clamp(-1_000_000_000.0, 1_000_000_000.0),
            );
        }
    }

    st.shipout.rule_wd += *cur_g;

    if (g_sign == GlueSign::Stretching && st.mem.b16s1(g) == g_order)
        || (g_sign == GlueSign::Shrinking && st.mem.b16s0(g) == g_order)
    {
        if st.mem.b32s1(g) == TEX_NULL {
            crate::mem::free_node(
                &mut st.mem,
                crate::units::MemPtr::from_raw(g),
                GLUE_SPEC_SIZE,
            );
        } else {
            st.mem.set_b32s1(g, st.mem.b32s1(g) - 1);
        }

        if st.mem.b16s0(p) < A_LEADERS {
            st.mem.set_b16s1(p, KERN_NODE);
            st.mem.set_b32s1(p + 1, st.shipout.rule_wd);
        } else {
            g = get_node_ship(&mut st.mem, GLUE_SPEC_SIZE)?;
            st.mem.set_b16s1(g, FILLL + 1);
            st.mem.set_b16s0(g, FILLL + 1);
            st.mem.set_b32s1(g + 1, st.shipout.rule_wd);
            st.mem.set_b32s1(g + 2, 0);
            st.mem.set_b32s1(g + 3, 0);
            st.mem.set_b32s0(p + 1, g);
        }
    }

    if st.mem.b16s0(p) >= A_LEADERS {
        let leader_box = st.mem.b32s1(p + 1);
        if st.mem.b16s1(leader_box) == RULE_NODE {
            st.shipout.rule_ht = st.mem.b32s1(leader_box + 3);
            st.shipout.rule_dp = st.mem.b32s1(leader_box + 2);
            finish_hlist_rule_in(st, this_box, base_line, MemPtr::from_raw(p))?;
            return Ok(());
        }

        let leader_wd = st.mem.b32s1(leader_box + 1);
        if leader_wd > 0 && st.shipout.rule_wd > 0 {
            st.shipout.rule_wd += 10;
            if st.scanner.cur_dir == RIGHT_TO_LEFT {
                st.shipout.cur_h -= 10;
            }
            let edge = st.shipout.cur_h + st.shipout.rule_wd;
            let mut lx = 0;

            if st.mem.b16s0(p) == A_LEADERS {
                let save_h = st.shipout.cur_h;
                st.shipout.cur_h =
                    left_edge + leader_wd * ((st.shipout.cur_h - left_edge) / leader_wd);
                if st.shipout.cur_h < save_h {
                    st.shipout.cur_h += leader_wd;
                }
            } else {
                let lq = st.shipout.rule_wd / leader_wd;
                let lr = st.shipout.rule_wd % leader_wd;
                if st.mem.b16s0(p) == C_LEADERS {
                    st.shipout.cur_h += lr / 2;
                } else {
                    lx = lr / (lq + 1);
                    st.shipout.cur_h += (lr - (lq - 1) * lx) / 2;
                }
            }

            while st.shipout.cur_h + leader_wd <= edge {
                st.shipout.cur_v = base_line + st.mem.b32s1(leader_box + 4);
                if st.shipout.cur_v != st.shipout.dvi_v {
                    let delta = st.shipout.cur_v - st.shipout.dvi_v;
                    emit_movement(&mut st.mem, &mut st.shipout, delta, DOWN1)?;
                    st.shipout.dvi_v = st.shipout.cur_v;
                }
                let save_v = st.shipout.dvi_v;

                if st.shipout.cur_h != st.shipout.dvi_h {
                    let delta = st.shipout.cur_h - st.shipout.dvi_h;
                    emit_movement(&mut st.mem, &mut st.shipout, delta, RIGHT1)?;
                    st.shipout.dvi_h = st.shipout.cur_h;
                }
                let save_h = st.shipout.dvi_h;

                st.mem.temp_ptr.0 = leader_box;
                if st.scanner.cur_dir == RIGHT_TO_LEFT {
                    st.shipout.cur_h += leader_wd;
                }

                let outer_doing_leaders = st.shipout.doing_leaders;
                st.shipout.doing_leaders = true;
                if st.mem.b16s1(leader_box) == VLIST_NODE {
                    vlist_out_in(st)?;
                } else {
                    hlist_out_in(st)?;
                }
                st.shipout.doing_leaders = outer_doing_leaders;

                st.shipout.dvi_v = save_v;
                st.shipout.dvi_h = save_h;
                st.shipout.cur_v = base_line;
                st.shipout.cur_h = save_h + leader_wd + lx;
            }

            if st.scanner.cur_dir == RIGHT_TO_LEFT {
                st.shipout.cur_h = edge;
            } else {
                st.shipout.cur_h = edge - 10;
            }
            return Ok(());
        }
    }

    st.shipout.cur_h += st.shipout.rule_wd;
    crate::synctex::synctex_horizontal_rule_or_glue(st, MemPtr::from_raw(p));
    Ok(())
}

// translated from xetex-shipout.c:63 (hlist_out)
unsafe fn hlist_out_in(st: &mut crate::state::EngineState) -> Result<(), ShipoutAbort> {
    let mut cur_g = 0;
    let mut cur_glue = 0.0;
    let this_box = st.mem.temp_ptr.0;
    let this_box_b = st
        .mem
        .classify_box(MemIdx::new(this_box))
        .expect("valid hlist box")
        .expect("hlist box");
    let g_order = this_box_b.glue_order(&st.mem);
    let g_sign = this_box_b.glue_sign(&st.mem);

    if st
        .eqtb
        .b32s1(INT_BASE + INT_PAR_XETEX_INTERWORD_SPACE_SHAPING)
        > 1
    {
        merge_justifiable_native_words(
            &mut st.mem,
            &mut st.strings,
            &st.fonts,
            &st.eqtb,
            this_box,
            g_order,
            g_sign,
        )?;
    }

    let mut p = this_box_b.list_ptr(&st.mem).0;
    st.shipout.cur_s += 1;
    if st.shipout.cur_s > 0 {
        emit_dvi_out(&mut st.shipout, PUSH)?;
    }
    if st.shipout.cur_s > st.shipout.max_push {
        st.shipout.max_push = st.shipout.cur_s;
    }

    let save_loc = st.shipout.dvi_offset + st.shipout.dvi_ptr;
    let base_line = st.shipout.cur_v;
    let mut prev_p = this_box + 5;

    let lr_node = get_avail_ship(&mut st.mem)?;
    st.mem.temp_ptr = lr_node;
    lr_node.set_info(&mut st.mem, BEFORE);
    lr_node.set_link(&mut st.mem, st.scanner.lr_ptr);
    st.scanner.lr_ptr = lr_node;

    if this_box_b.subtype(&st.mem) == DLIST {
        if st.scanner.cur_dir == RIGHT_TO_LEFT {
            st.scanner.cur_dir = LEFT_TO_RIGHT;
            st.shipout.cur_h -= this_box_b.width(&st.mem).0;
        } else {
            this_box_b.set_subtype(&mut st.mem, 0);
        }
    }

    if st.scanner.cur_dir == RIGHT_TO_LEFT && this_box_b.subtype(&st.mem) != REVERSED {
        let save_h = st.shipout.cur_h;
        st.mem.temp_ptr.0 = p;
        p = crate::mem::alloc_kern(&mut st.mem, 0)
            .ok_or(ShipoutAbort::MainMemoryOverflow)?
            .ptr()
            .0;
        st.mem.set_b32s0(p + 2, 0);
        st.mem.set_b32s1(prev_p, p);
        st.shipout.cur_h = 0;
        let reversed = reverse(
            &mut st.mem,
            &mut st.shipout,
            &mut st.scanner,
            &mut st.fonts,
            this_box,
            TEX_NULL,
            &mut cur_g,
            &mut cur_glue,
        )?;
        st.mem.set_b32s1(p, reversed);
        st.mem.set_b32s1(p + 1, -st.shipout.cur_h);
        st.shipout.cur_h = save_h;
        this_box_b.set_subtype(&mut st.mem, REVERSED);
    }

    let mut left_edge = st.shipout.cur_h;
    crate::synctex::synctex_hlist(st, this_box_b);

    while p != TEX_NULL {
        'reswitch: loop {
            if crate::mem::is_char_node(&st.mem, p) {
                if st.shipout.cur_h != st.shipout.dvi_h {
                    let delta = st.shipout.cur_h - st.shipout.dvi_h;
                    emit_movement(&mut st.mem, &mut st.shipout, delta, RIGHT1)?;
                    st.shipout.dvi_h = st.shipout.cur_h;
                }
                if st.shipout.cur_v != st.shipout.dvi_v {
                    let delta = st.shipout.cur_v - st.shipout.dvi_v;
                    emit_movement(&mut st.mem, &mut st.shipout, delta, DOWN1)?;
                    st.shipout.dvi_v = st.shipout.cur_v;
                }

                while crate::mem::is_char_node(&st.mem, p) {
                    let f = st.mem.b16s1(p) as i32;
                    let mut c = st.mem.b16s0(p);
                    let mapping = st.fonts.font_mapping_ptr(f);
                    if p != LIG_TRICK && !mapping.is_null() {
                        c = apply_tfm_font_mapping(mapping, c as libc::c_int) as u16;
                    }

                    ensure_dvi_font_in(st, f)?;
                    if st.fonts.font_ec_entry(f) >= c && st.fonts.font_bc_entry(f) <= c {
                        let info = st
                            .fonts
                            .font_info_b16(st.fonts.char_base_entry(f) + c as i32);
                        if info.s3 > 0 {
                            if c >= 128 {
                                emit_dvi_out(&mut st.shipout, SET1)?;
                            }
                            emit_dvi_out(&mut st.shipout, c as u8)?;
                            st.shipout.cur_h += font_character_width(&st.fonts, f, c as i32);
                        }
                    }

                    prev_p = st.mem.b32s1(prev_p);
                    p = st.mem.b32s1(p);
                }

                crate::synctex::synctex_current(st);
                st.shipout.dvi_h = st.shipout.cur_h;
            } else {
                let node = st
                    .mem
                    .classify(MemIdx::new(p))
                    .expect("valid node type in hlist_out");
                let box_node = match node {
                    AnyNode::HList(q) => Some((AnyBox::HList(q), false)),
                    AnyNode::VList(q) => Some((AnyBox::VList(q), true)),
                    _ => None,
                };

                if let Some((bb, is_vlist)) = box_node {
                    if bb.list_ptr(&st.mem).is_null() {
                        match bb {
                            AnyBox::VList(v) => crate::synctex::synctex_void_vlist(st, v),
                            AnyBox::HList(h) => crate::synctex::synctex_void_hlist(st, h),
                        }
                        st.shipout.cur_h += bb.width(&st.mem).0;
                    } else {
                        let save_h = st.shipout.dvi_h;
                        let save_v = st.shipout.dvi_v;
                        st.shipout.cur_v = base_line + bb.shift_amount(&st.mem).0;
                        st.mem.temp_ptr.0 = p;
                        let edge = st.shipout.cur_h + bb.width(&st.mem).0;
                        if st.scanner.cur_dir == RIGHT_TO_LEFT {
                            st.shipout.cur_h = edge;
                        }
                        if is_vlist {
                            vlist_out_in(st)?;
                        } else {
                            hlist_out_in(st)?;
                        }
                        st.shipout.dvi_h = save_h;
                        st.shipout.dvi_v = save_v;
                        st.shipout.cur_h = edge;
                        st.shipout.cur_v = base_line;
                    }
                } else {
                    match node {
                        AnyNode::Rule(rule) => {
                            st.shipout.rule_ht = rule.height(&st.mem).raw();
                            st.shipout.rule_dp = rule.depth(&st.mem).raw();
                            st.shipout.rule_wd = rule.width(&st.mem).raw();
                            finish_hlist_rule_in(st, this_box_b, base_line, rule.ptr())?;
                        }
                        AnyNode::Whatsit(whatsit) => {
                            let subtype = whatsit.whatsit_subtype(&st.mem);
                            match subtype {
                                Some(WhatsitSubtype::NativeWord)
                                | Some(WhatsitSubtype::NativeWordAt)
                                | Some(WhatsitSubtype::Glyph) => {
                                    if st.shipout.cur_h != st.shipout.dvi_h {
                                        let delta = st.shipout.cur_h - st.shipout.dvi_h;
                                        emit_movement(&mut st.mem, &mut st.shipout, delta, RIGHT1)?;
                                        st.shipout.dvi_h = st.shipout.cur_h;
                                    }
                                    if st.shipout.cur_v != st.shipout.dvi_v {
                                        let delta = st.shipout.cur_v - st.shipout.dvi_v;
                                        emit_movement(&mut st.mem, &mut st.shipout, delta, DOWN1)?;
                                        st.shipout.dvi_v = st.shipout.cur_v;
                                    }
                                    let f = whatsit.native_font(&st.mem).0;
                                    ensure_dvi_font_in(st, f)?;

                                    if subtype == Some(WhatsitSubtype::Glyph) {
                                        emit_dvi_out(&mut st.shipout, SET_GLYPHS)?;
                                        emit_dvi_four(&mut st.shipout, whatsit.width(&st.mem).0)?;
                                        emit_dvi_two(&mut st.shipout, 1)?;
                                        emit_dvi_four(&mut st.shipout, 0)?;
                                        emit_dvi_four(&mut st.shipout, 0)?;
                                        emit_dvi_two(
                                            &mut st.shipout,
                                            whatsit.native_glyph(&st.mem),
                                        )?;
                                        st.shipout.cur_h += whatsit.width(&st.mem).0;
                                    } else {
                                        if subtype == Some(WhatsitSubtype::NativeWordAt) {
                                            if whatsit.native_length(&st.mem) > 0
                                                || whatsit.native_glyph_info_handle(&st.mem) != 0
                                            {
                                                emit_dvi_out(&mut st.shipout, SET_TEXT_AND_GLYPHS)?;
                                                let len = whatsit.native_length(&st.mem) as i32;
                                                emit_dvi_two(&mut st.shipout, len as u16)?;
                                                let text = whatsit.native_text_ptr(&st.mem);
                                                for k in 0..len {
                                                    emit_dvi_two(
                                                        &mut st.shipout,
                                                        *text.add(k as usize),
                                                    )?;
                                                }
                                                let len = make_xdv_glyph_array_data_in(st, whatsit);
                                                for k in 0..len {
                                                    emit_dvi_out(
                                                        &mut st.shipout,
                                                        st.fonts.xdv_buffer[k as usize],
                                                    )?;
                                                }
                                            }
                                        } else if whatsit.native_glyph_info_handle(&st.mem) != 0 {
                                            emit_dvi_out(&mut st.shipout, SET_GLYPHS)?;
                                            let len = make_xdv_glyph_array_data_in(st, whatsit);
                                            for k in 0..len {
                                                emit_dvi_out(
                                                    &mut st.shipout,
                                                    st.fonts.xdv_buffer[k as usize],
                                                )?;
                                            }
                                        }
                                        st.shipout.cur_h += whatsit.width(&st.mem).0;
                                    }
                                    st.shipout.dvi_h = st.shipout.cur_h;
                                }
                                Some(WhatsitSubtype::Pic) | Some(WhatsitSubtype::Pdf) => {
                                    let save_h = st.shipout.dvi_h;
                                    let save_v = st.shipout.dvi_v;
                                    st.shipout.cur_v = base_line;
                                    let edge = st.shipout.cur_h + whatsit.width(&st.mem).0;
                                    pic_out_in(st, p)?;
                                    st.shipout.dvi_h = save_h;
                                    st.shipout.dvi_v = save_v;
                                    st.shipout.cur_h = edge;
                                    st.shipout.cur_v = base_line;
                                }
                                Some(WhatsitSubtype::PdfSavePos) => {
                                    st.shipout.pdf_last_x_pos =
                                        st.shipout.cur_h + ONE_INCH_LESS_ONE;
                                    st.shipout.pdf_last_y_pos = st.shipout.cur_page_height
                                        - st.shipout.cur_v
                                        - ONE_INCH_LESS_ONE;
                                }
                                _ => out_what_in(st, p)?,
                            }
                        }
                        AnyNode::Glue(glue) => {
                            output_hlist_glue_in(
                                st,
                                this_box_b,
                                base_line,
                                left_edge,
                                glue.idx().0,
                                g_order,
                                g_sign,
                                &mut cur_g,
                                &mut cur_glue,
                            )?;
                        }
                        AnyNode::MarginKern(kern) => st.shipout.cur_h += kern.width(&st.mem).0,
                        AnyNode::Kern(kern) => {
                            crate::synctex::synctex_kern(st, kern, this_box_b);
                            st.shipout.cur_h += kern.width(&st.mem).0;
                        }
                        AnyNode::Math(math) => {
                            crate::synctex::synctex_math(st, math);
                            let math_width = math.width(&st.mem).0;
                            if !math.subtype(&st.mem).is_multiple_of(2) {
                                if st.mem.b32s0(st.scanner.lr_ptr) as u16
                                    == math_node_end_lr_type(&st.mem, p)
                                {
                                    st.mem.temp_ptr.0 = st.scanner.lr_ptr.0;
                                    st.scanner.lr_ptr.0 = st.mem.b32s1(st.mem.temp_ptr);
                                    st.mem.set_b32s1(st.mem.temp_ptr, st.mem.avail);
                                    st.mem.avail = st.mem.temp_ptr.0;
                                } else if math.subtype(&st.mem) > L_CODE {
                                    st.scanner.lr_problems += 1;
                                }
                            } else {
                                let lr_node = get_avail_ship(&mut st.mem)?;
                                st.mem.temp_ptr = lr_node;
                                let v = math_node_end_lr_type(&st.mem, p) as i32;
                                lr_node.set_info(&mut st.mem, v);
                                lr_node.set_link(&mut st.mem, st.scanner.lr_ptr);
                                st.scanner.lr_ptr = lr_node;

                                if math_node_lr_dir(&st.mem, p) as i16 != st.scanner.cur_dir {
                                    let save_h = st.shipout.cur_h;
                                    st.mem.temp_ptr.0 = math.link(&st.mem).0;
                                    st.shipout.rule_wd = math_width;
                                    crate::mem::free_node(
                                        &mut st.mem,
                                        crate::units::MemPtr::from_raw(p),
                                        MEDIUM_NODE_SIZE,
                                    );
                                    st.scanner.cur_dir = 1 - st.scanner.cur_dir;
                                    p = new_edge(
                                        &mut st.mem,
                                        st.scanner.cur_dir,
                                        st.shipout.rule_wd,
                                    )
                                    .ok_or(ShipoutAbort::MainMemoryOverflow)?
                                    .ptr()
                                    .0;
                                    st.mem.set_b32s1(prev_p, p);
                                    st.shipout.cur_h =
                                        st.shipout.cur_h - left_edge + st.shipout.rule_wd;
                                    let edge = new_edge(&mut st.mem, 1 - st.scanner.cur_dir, 0)
                                        .ok_or(ShipoutAbort::MainMemoryOverflow)?
                                        .ptr()
                                        .0;
                                    let reversed = reverse(
                                        &mut st.mem,
                                        &mut st.shipout,
                                        &mut st.scanner,
                                        &mut st.fonts,
                                        this_box,
                                        edge,
                                        &mut cur_g,
                                        &mut cur_glue,
                                    )?;
                                    st.mem.set_b32s1(p, reversed);
                                    st.mem.set_b32s1(p + 2, st.shipout.cur_h);
                                    st.scanner.cur_dir = 1 - st.scanner.cur_dir;
                                    st.shipout.cur_h = save_h;
                                    continue 'reswitch;
                                }
                            }
                            st.mem.set_b16s1(p, KERN_NODE);
                            st.shipout.cur_h += math_width;
                        }
                        AnyNode::Ligature(ligature) => {
                            st.mem
                                .set_word(LIG_TRICK, st.mem.word(ligature.idx().offset(1).0));
                            st.mem.set_b32s1(LIG_TRICK, ligature.link(&st.mem).0);
                            p = LIG_TRICK;
                            st.fonts.set_xtx_ligature_present(true);
                            continue 'reswitch;
                        }
                        AnyNode::Style(_) => {
                            // NODE_type 14 is an edge in this context. Its typed
                            // family currently exposes writes but not payload reads.
                            st.shipout.cur_h += st.mem.b32s1(p + 1);
                            left_edge = st.shipout.cur_h + st.mem.b32s1(p + 2);
                            st.scanner.cur_dir = st.mem.b16s0(p) as i16;
                        }
                        _ => {}
                    }
                }

                prev_p = p;
                p = st.mem.b32s1(p);
            }
            break;
        }
    }

    crate::synctex::synctex_tsilh(st, this_box_b);

    while st.mem.b32s0(st.scanner.lr_ptr) != BEFORE {
        if st.mem.b32s0(st.scanner.lr_ptr) as u16 > L_CODE {
            st.scanner.lr_problems += 10000;
        }
        st.mem.temp_ptr.0 = st.scanner.lr_ptr.0;
        st.scanner.lr_ptr.0 = st.mem.b32s1(st.mem.temp_ptr);
        st.mem.set_b32s1(st.mem.temp_ptr, st.mem.avail);
        st.mem.avail = st.mem.temp_ptr.0;
    }

    st.mem.temp_ptr.0 = st.scanner.lr_ptr.0;
    st.scanner.lr_ptr.0 = st.mem.b32s1(st.mem.temp_ptr);
    st.mem.set_b32s1(st.mem.temp_ptr, st.mem.avail);
    st.mem.avail = st.mem.temp_ptr.0;

    if this_box_b.subtype(&st.mem) == DLIST {
        st.scanner.cur_dir = RIGHT_TO_LEFT;
    }

    prune_movements(&mut st.mem, &mut st.shipout, save_loc);
    if st.shipout.cur_s > 0 && dvi_pop(&mut st.shipout, save_loc) {
        emit_dvi_out(&mut st.shipout, POP)?;
    }
    st.shipout.cur_s -= 1;
    Ok(())
}

// translated from xetex-shipout.c:866 (vlist_out)
unsafe fn vlist_out_in(st: &mut crate::state::EngineState) -> Result<(), ShipoutAbort> {
    let mut cur_g = 0;
    let mut cur_glue = 0.0;
    let this_box = st.mem.temp_ptr.0;
    let this_box_b = st
        .mem
        .classify_box(MemIdx::new(this_box))
        .expect("valid vlist box")
        .expect("vlist box");
    let g_order = this_box_b.glue_order(&st.mem);
    let g_sign = this_box_b.glue_sign(&st.mem);
    let mut p = this_box_b.list_ptr(&st.mem).0;
    let upwards = this_box_b.subtype(&st.mem) == 1;

    st.shipout.cur_s = st.shipout.cur_s + 1;
    if st.shipout.cur_s > 0 {
        emit_dvi_out(&mut st.shipout, PUSH)?;
    }

    if st.shipout.cur_s > st.shipout.max_push {
        st.shipout.max_push = st.shipout.cur_s;
    }

    let save_loc = st.shipout.dvi_offset + st.shipout.dvi_ptr;
    let left_edge = st.shipout.cur_h;
    crate::synctex::synctex_vlist(st, this_box_b);

    if upwards {
        st.shipout.cur_v = st.shipout.cur_v + this_box_b.depth(&st.mem).0;
    } else {
        st.shipout.cur_v = st.shipout.cur_v - this_box_b.height(&st.mem).0;
    }

    let top_edge = st.shipout.cur_v;

    while p != TEX_NULL {
        if crate::mem::is_char_node(&st.mem, p) {
            return Err(ShipoutAbort::Confusion(c"vlistout"));
        }

        let mut go_next = true;
        let node = st
            .mem
            .classify(MemIdx::new(p))
            .expect("valid node type in vlist_out");
        let box_node = match node {
            AnyNode::HList(q) => Some((AnyBox::HList(q), false)),
            AnyNode::VList(q) => Some((AnyBox::VList(q), true)),
            _ => None,
        };

        if let Some((bb, is_vlist)) = box_node {
            if bb.list_ptr(&st.mem).is_null() {
                if upwards {
                    st.shipout.cur_v -= bb.depth(&st.mem).0;
                } else {
                    st.shipout.cur_v += bb.height(&st.mem).0;
                }

                match bb {
                    AnyBox::VList(v) => crate::synctex::synctex_void_vlist(st, v),
                    AnyBox::HList(h) => crate::synctex::synctex_void_hlist(st, h),
                }

                if upwards {
                    st.shipout.cur_v -= bb.height(&st.mem).0;
                } else {
                    st.shipout.cur_v += bb.depth(&st.mem).0;
                }
            } else {
                if upwards {
                    st.shipout.cur_v -= bb.depth(&st.mem).0;
                } else {
                    st.shipout.cur_v += bb.height(&st.mem).0;
                }

                if st.shipout.cur_v != st.shipout.dvi_v {
                    let d = st.shipout.cur_v - st.shipout.dvi_v;
                    emit_movement(&mut st.mem, &mut st.shipout, d, DOWN1)?;
                    st.shipout.dvi_v = st.shipout.cur_v;
                }

                let save_h = st.shipout.dvi_h;
                let save_v = st.shipout.dvi_v;

                if st.scanner.cur_dir == RIGHT_TO_LEFT {
                    st.shipout.cur_h = left_edge - bb.shift_amount(&st.mem).0;
                } else {
                    st.shipout.cur_h = left_edge + bb.shift_amount(&st.mem).0;
                }

                st.mem.temp_ptr.0 = p;
                if is_vlist {
                    vlist_out_in(st)?;
                } else {
                    hlist_out_in(st)?;
                }

                st.shipout.dvi_h = save_h;
                st.shipout.dvi_v = save_v;

                if upwards {
                    st.shipout.cur_v = save_v - bb.height(&st.mem).0;
                } else {
                    st.shipout.cur_v = save_v + bb.depth(&st.mem).0;
                }

                st.shipout.cur_h = left_edge;
            }
        } else {
            match node {
                AnyNode::Rule(rule) => {
                    st.shipout.rule_ht = rule.height(&st.mem).raw();
                    st.shipout.rule_dp = rule.depth(&st.mem).raw();
                    st.shipout.rule_wd = rule.width(&st.mem).raw();
                    finish_vlist_rule_in(st, this_box, left_edge, upwards)?;
                }
                AnyNode::Whatsit(whatsit) => match whatsit.whatsit_subtype(&st.mem) {
                    Some(WhatsitSubtype::Glyph) => {
                        st.shipout.cur_v += whatsit.height(&st.mem).0;
                        st.shipout.cur_h = left_edge;

                        if st.shipout.cur_h != st.shipout.dvi_h {
                            let d = st.shipout.cur_h - st.shipout.dvi_h;
                            emit_movement(&mut st.mem, &mut st.shipout, d, RIGHT1)?;
                            st.shipout.dvi_h = st.shipout.cur_h;
                        }

                        if st.shipout.cur_v != st.shipout.dvi_v {
                            let d = st.shipout.cur_v - st.shipout.dvi_v;
                            emit_movement(&mut st.mem, &mut st.shipout, d, DOWN1)?;
                            st.shipout.dvi_v = st.shipout.cur_v;
                        }

                        let f = whatsit.native_font(&st.mem).0;
                        ensure_dvi_font_in(st, f)?;

                        emit_dvi_out(&mut st.shipout, SET_GLYPHS)?;
                        emit_dvi_four(&mut st.shipout, 0)?;
                        emit_dvi_two(&mut st.shipout, 1)?;
                        emit_dvi_four(&mut st.shipout, 0)?;
                        emit_dvi_four(&mut st.shipout, 0)?;
                        emit_dvi_two(&mut st.shipout, whatsit.native_glyph(&st.mem))?;

                        st.shipout.cur_v += whatsit.depth(&st.mem).0;
                        st.shipout.cur_h = left_edge;
                    }
                    Some(WhatsitSubtype::Pic) | Some(WhatsitSubtype::Pdf) => {
                        let save_h = st.shipout.dvi_h;
                        let save_v = st.shipout.dvi_v;
                        st.shipout.cur_v += whatsit.height(&st.mem).0;
                        pic_out_in(st, p)?;
                        st.shipout.dvi_h = save_h;
                        st.shipout.dvi_v = save_v;
                        st.shipout.cur_v = save_v + whatsit.depth(&st.mem).0;
                        st.shipout.cur_h = left_edge;
                    }
                    Some(WhatsitSubtype::PdfSavePos) => {
                        st.shipout.pdf_last_x_pos = st.shipout.cur_h + ONE_INCH_LESS_ONE;
                        st.shipout.pdf_last_y_pos =
                            st.shipout.cur_page_height - st.shipout.cur_v - ONE_INCH_LESS_ONE;
                    }
                    _ => out_what_in(st, p)?,
                },
                AnyNode::Glue(glue) => {
                    let spec = glue
                        .glue_spec(&st.mem)
                        .expect("glue node in vlist_out has a glue spec");
                    st.shipout.rule_ht = spec.width(&st.mem).0 - cur_g;

                    if g_sign != GlueSign::Normal {
                        if g_sign == GlueSign::Stretching {
                            if spec.stretch_order(&st.mem) == g_order {
                                cur_glue += spec.stretch(&st.mem).0 as f64;
                                let glue_temp = (this_box_b.glue_set(&st.mem) * cur_glue)
                                    .clamp(-1_000_000_000.0, 1_000_000_000.0);
                                cur_g = crate::scaled_math::tex_round(glue_temp);
                            }
                        } else if spec.shrink_order(&st.mem) == g_order {
                            cur_glue -= spec.shrink(&st.mem).0 as f64;
                            let glue_temp = (this_box_b.glue_set(&st.mem) * cur_glue)
                                .clamp(-1_000_000_000.0, 1_000_000_000.0);
                            cur_g = crate::scaled_math::tex_round(glue_temp);
                        }
                    }

                    st.shipout.rule_ht = st.shipout.rule_ht + cur_g;

                    if glue.subtype(&st.mem) >= A_LEADERS {
                        let leader_box = glue.leader_ptr(&st.mem).0;
                        let leader = st
                            .mem
                            .classify(MemIdx::new(leader_box))
                            .expect("valid leader node type in vlist_out");

                        if let AnyNode::Rule(rule) = leader {
                            st.shipout.rule_wd = rule.width(&st.mem).raw();
                            st.shipout.rule_dp = 0;
                            finish_vlist_rule_in(st, this_box, left_edge, upwards)?;
                        } else {
                            let (leader_bb, leader_is_vlist) = match leader {
                                AnyNode::HList(q) => (AnyBox::HList(q), false),
                                AnyNode::VList(q) => (AnyBox::VList(q), true),
                                _ => panic!("non-box leader in vlist_out"),
                            };
                            let leader_ht =
                                leader_bb.height(&st.mem).0 + leader_bb.depth(&st.mem).0;

                            if leader_ht > 0 && st.shipout.rule_ht > 0 {
                                st.shipout.rule_ht = st.shipout.rule_ht + 10;
                                let edge = st.shipout.cur_v + st.shipout.rule_ht;
                                let mut lx = 0;

                                if glue.subtype(&st.mem) == A_LEADERS {
                                    let save_v = st.shipout.cur_v;
                                    st.shipout.cur_v = top_edge
                                        + leader_ht * ((st.shipout.cur_v - top_edge) / leader_ht);
                                    if st.shipout.cur_v < save_v {
                                        st.shipout.cur_v = st.shipout.cur_v + leader_ht;
                                    }
                                } else {
                                    let lq = st.shipout.rule_ht / leader_ht;
                                    let lr = st.shipout.rule_ht % leader_ht;

                                    if glue.subtype(&st.mem) == C_LEADERS {
                                        st.shipout.cur_v = st.shipout.cur_v + (lr / 2);
                                    } else {
                                        lx = lr / (lq + 1);
                                        st.shipout.cur_v =
                                            st.shipout.cur_v + ((lr - (lq - 1) * lx) / 2);
                                    }
                                }

                                while st.shipout.cur_v + leader_ht <= edge {
                                    if st.scanner.cur_dir == RIGHT_TO_LEFT {
                                        st.shipout.cur_h =
                                            left_edge - leader_bb.shift_amount(&st.mem).0;
                                    } else {
                                        st.shipout.cur_h =
                                            left_edge + leader_bb.shift_amount(&st.mem).0;
                                    }

                                    if st.shipout.cur_h != st.shipout.dvi_h {
                                        let d = st.shipout.cur_h - st.shipout.dvi_h;
                                        emit_movement(&mut st.mem, &mut st.shipout, d, RIGHT1)?;
                                        st.shipout.dvi_h = st.shipout.cur_h;
                                    }

                                    let save_h = st.shipout.dvi_h;
                                    st.shipout.cur_v += leader_bb.height(&st.mem).0;

                                    if st.shipout.cur_v != st.shipout.dvi_v {
                                        let d = st.shipout.cur_v - st.shipout.dvi_v;
                                        emit_movement(&mut st.mem, &mut st.shipout, d, DOWN1)?;
                                        st.shipout.dvi_v = st.shipout.cur_v;
                                    }

                                    let save_v = st.shipout.dvi_v;
                                    st.mem.temp_ptr.0 = leader_box;
                                    let outer_doing_leaders = st.shipout.doing_leaders;
                                    st.shipout.doing_leaders = true;

                                    if leader_is_vlist {
                                        vlist_out_in(st)?;
                                    } else {
                                        hlist_out_in(st)?;
                                    }

                                    st.shipout.doing_leaders = outer_doing_leaders;
                                    st.shipout.dvi_v = save_v;
                                    st.shipout.dvi_h = save_h;
                                    st.shipout.cur_h = left_edge;
                                    st.shipout.cur_v =
                                        save_v - leader_bb.height(&st.mem).0 + leader_ht + lx;
                                }

                                st.shipout.cur_v = edge - 10;
                                go_next = true;
                            } else {
                                move_past_vlist_rule(&mut st.shipout, upwards);
                            }
                        }
                    } else {
                        move_past_vlist_rule(&mut st.shipout, upwards);
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
        }

        if go_next {
            p = st.mem.b32s1(p);
        }
    }

    crate::synctex::synctex_tsilv(st, this_box_b);
    prune_movements(&mut st.mem, &mut st.shipout, save_loc);

    if st.shipout.cur_s > 0 {
        if dvi_pop(&mut st.shipout, save_loc) {
            emit_dvi_out(&mut st.shipout, POP)?;
        }
    }
    st.shipout.cur_s = st.shipout.cur_s - 1;

    Ok(())
}

struct VListRuleAction {
    emit: bool,
    h_move: Option<i32>,
    v_move: Option<i32>,
    rule_ht: i32,
    rule_wd: i32,
}

fn finish_vlist_rule(
    mem: &MemState,
    shipout: &mut ShipoutState,
    this_box: i32,
    left_edge: i32,
    upwards: bool,
    cur_dir: i16,
) -> VListRuleAction {
    if shipout.rule_wd == NULL_FLAG {
        let bb = mem
            .classify_box(MemIdx::new(this_box))
            .expect("valid node type for vlist_out box")
            .expect("vlist_out operates on a box");
        shipout.rule_wd = bb.width(mem).0;
    }

    shipout.rule_ht += shipout.rule_dp;

    if upwards {
        shipout.cur_v -= shipout.rule_ht;
    } else {
        shipout.cur_v += shipout.rule_ht;
    }

    let mut emit = false;
    let mut h_move = None;
    let mut v_move = None;

    if shipout.rule_ht > 0 && shipout.rule_wd > 0 {
        emit = true;
        if cur_dir == RIGHT_TO_LEFT {
            shipout.cur_h -= shipout.rule_wd;
        }

        if shipout.cur_h != shipout.dvi_h {
            h_move = Some(shipout.cur_h - shipout.dvi_h);
            shipout.dvi_h = shipout.cur_h;
        }

        if shipout.cur_v != shipout.dvi_v {
            v_move = Some(shipout.cur_v - shipout.dvi_v);
            shipout.dvi_v = shipout.cur_v;
        }

        shipout.cur_h = left_edge;
    }

    VListRuleAction {
        emit,
        h_move,
        v_move,
        rule_ht: shipout.rule_ht,
        rule_wd: shipout.rule_wd,
    }
}

fn finish_vlist_rule_in(
    st: &mut crate::state::EngineState,
    this_box: i32,
    left_edge: i32,
    upwards: bool,
) -> Result<(), ShipoutAbort> {
    let cur_dir_val = st.scanner.cur_dir;
    let action = finish_vlist_rule(
        &st.mem,
        &mut st.shipout,
        this_box,
        left_edge,
        upwards,
        cur_dir_val,
    );
    if action.emit {
        if let Some(h_move) = action.h_move {
            emit_movement(&mut st.mem, &mut st.shipout, h_move, RIGHT1)?;
        }
        if let Some(v_move) = action.v_move {
            emit_movement(&mut st.mem, &mut st.shipout, v_move, DOWN1)?;
        }
        emit_dvi_out(&mut st.shipout, PUT_RULE)?;
        emit_dvi_four(&mut st.shipout, action.rule_ht)?;
        emit_dvi_four(&mut st.shipout, action.rule_wd)?;
    }
    Ok(())
}

fn move_past_vlist_rule(shipout: &mut ShipoutState, upwards: bool) {
    if upwards {
        shipout.cur_v -= shipout.rule_ht;
    } else {
        shipout.cur_v += shipout.rule_ht;
    }
}

// xetex-constants.h:238,240,248,249,226,227
const EOP: u8 = 140;
const POST: u8 = 248;
const POST_POST: u8 = 249;
const XDV_ID_BYTE: u8 = 7;
const SPX_ID_BYTE: u8 = 100;
// xetex-constants.h:117
// xetex_format.h:52; xetex-xetexd.h INT_BASE
const INT_PAR_TRACING_OUTPUT: i32 = 34;
const DIMEN_PAR_H_OFFSET: i32 = 18;
const DIMEN_PAR_V_OFFSET: i32 = 19;
const DIMEN_PAR_PDF_PAGE_WIDTH: i32 = 21;
const DIMEN_PAR_PDF_PAGE_HEIGHT: i32 = 22;
const ONE_INCH: i32 = 4_736_287;
const ONE_INCH_LESS_ONE: i32 = 4_736_286;

// translated from xetex-shipout.c:1930 (prune_movements)
pub fn prune_movements(mem: &mut MemState, shipout: &mut ShipoutState, l: i32) {
    while let Some(p) = NodePtr::<MovementNode>::from_ptr(MemPtr::from_raw(shipout.down_ptr.0)) {
        if p.location(mem) < l {
            break;
        }
        shipout.down_ptr.0 = p.link(mem).0;
        p.free(mem);
    }
    while let Some(p) = NodePtr::<MovementNode>::from_ptr(MemPtr::from_raw(shipout.right_ptr.0)) {
        if p.location(mem) < l {
            return;
        }
        shipout.right_ptr.0 = p.link(mem).0;
        p.free(mem);
    }
}

unsafe fn print_c_string_here_in(st: &mut crate::state::EngineState, s: *const libc::c_char) {
    let mut p = s;
    while *p != 0 {
        crate::output::print_char_in(st, *p as i32);
        p = p.add(1);
    }
}

// translated from xetex-shipout.c:2173 (finalize_dvi_file)
pub(crate) fn finalize_dvi_file_in(st: &mut crate::state::EngineState) -> Result<(), ShipoutAbort> {
    while st.shipout.cur_s > -1 {
        if st.shipout.cur_s > 0 {
            emit_dvi_out(&mut st.shipout, POP)?;
        } else {
            emit_dvi_out(&mut st.shipout, EOP)?;
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

    if st.shipout.cur_s == -2 {
        // DVI got too big; message already printed
        return Ok(());
    }

    emit_dvi_out(&mut st.shipout, POST)?;
    let last_bop = st.shipout.last_bop;
    emit_dvi_four(&mut st.shipout, last_bop)?;
    st.shipout.last_bop = st.shipout.dvi_offset + st.shipout.dvi_ptr - 5;
    emit_dvi_four(&mut st.shipout, 25_400_000)?; // magic: conversion ratio for sp
    emit_dvi_four(&mut st.shipout, 473_628_672)?; // magic: conversion ratio for sp
    ship_err(crate::save::prepare_mag_in(st))?;
    emit_dvi_four(&mut st.shipout, st.eqtb.b32s1(INT_BASE + INT_PAR_MAG))?;
    let max_v = st.shipout.max_v;
    let max_h = st.shipout.max_h;
    let max_push = st.shipout.max_push;
    let total_pages = st.shipout.total_pages;
    emit_dvi_four(&mut st.shipout, max_v)?;
    emit_dvi_four(&mut st.shipout, max_h)?;
    emit_dvi_out(&mut st.shipout, (max_push / 256) as u8)?;
    emit_dvi_out(&mut st.shipout, (max_push % 256) as u8)?;
    emit_dvi_out(&mut st.shipout, ((total_pages / 256) % 256) as u8)?;
    emit_dvi_out(&mut st.shipout, (total_pages % 256) as u8)?;

    let mut fp = st.fonts.font_ptr;
    while fp > FONT_BASE {
        if st.fonts.font_used[fp as usize] {
            dvi_font_def_in(st, fp)?;
        }
        fp -= 1;
    }
    st.fonts.font_ptr = fp;

    emit_dvi_out(&mut st.shipout, POST_POST)?;
    let last_bop = st.shipout.last_bop;
    emit_dvi_four(&mut st.shipout, last_bop)?;

    if st.config.semantic_pagination_enabled {
        emit_dvi_out(&mut st.shipout, SPX_ID_BYTE)?;
    } else {
        emit_dvi_out(&mut st.shipout, XDV_ID_BYTE)?;
    }

    let mut k = (4 + (DVI_BUF_SIZE - st.shipout.dvi_ptr) % 4) as u8;
    while k > 0 {
        emit_dvi_out(&mut st.shipout, 223)?;
        k -= 1;
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
        unsafe { print_c_string_here_in(st, libc::strerror(k as libc::c_int)) };
        crate::output::print_cstr_in(st, c") generating output;".as_ptr());
        crate::output::print_nl_cstr_in(st, c"file ".as_ptr());
        crate::output::print_in(st, st.shipout.output_file_name);
        crate::output::print_cstr_in(st, c" may not be valid.".as_ptr());
        // XeTeX adds history = OUTPUT_FAILURE = 4 here; not implemented.
    }
    Ok(())
}

#[inline]
fn math_node_lr_dir(mem: &MemState, p: impl crate::units::MemAddr) -> u16 {
    let p = p.addr();
    mem.b16s0(p) / R_CODE
}

#[inline]
fn math_node_end_lr_type(mem: &MemState, p: impl crate::units::MemAddr) -> u16 {
    let p = p.addr();
    L_CODE * (mem.b16s0(p) / L_CODE) + END_M_CODE
}

#[inline]
fn font_character_width(fonts: &FontState, f: i32, c: i32) -> i32 {
    let info = fonts.font_info_b16(fonts.char_base_entry(f) + c);
    fonts.font_info_b32s1(fonts.width_base_entry(f) + info.s3 as i32)
}

unsafe fn make_xdv_glyph_array_data_in(
    st: &mut crate::state::EngineState,
    p: NodePtr<WhatsitNode>,
) -> i32 {
    let glyph_count = p.native_glyph_count(&st.mem) as usize;
    let needed = glyph_count * NATIVE_GLYPH_INFO_SIZE + 8;
    if st.fonts.xdv_buffer.len() < needed {
        st.fonts.xdv_buffer.resize(needed, 0);
    }

    let glyphs = st.mem.glyph_blobs.get(p.native_glyph_info_handle(&st.mem));

    let xdv_ptr = st.fonts.xdv_buffer.as_mut_ptr();
    let mut cp = xdv_ptr;
    let width = p.width(&st.mem).0 as u32;

    *cp = (width >> 24) as u8;
    cp = cp.add(1);
    *cp = (width >> 16) as u8;
    cp = cp.add(1);
    *cp = (width >> 8) as u8;
    cp = cp.add(1);
    *cp = width as u8;
    cp = cp.add(1);

    *cp = (glyph_count >> 8) as u8;
    cp = cp.add(1);
    *cp = glyph_count as u8;
    cp = cp.add(1);

    for i in 0..glyph_count {
        let x = glyphs.locations[i].x as u32;
        let y = glyphs.locations[i].y as u32;
        *cp = (x >> 24) as u8;
        cp = cp.add(1);
        *cp = (x >> 16) as u8;
        cp = cp.add(1);
        *cp = (x >> 8) as u8;
        cp = cp.add(1);
        *cp = x as u8;
        cp = cp.add(1);
        *cp = (y >> 24) as u8;
        cp = cp.add(1);
        *cp = (y >> 16) as u8;
        cp = cp.add(1);
        *cp = (y >> 8) as u8;
        cp = cp.add(1);
        *cp = y as u8;
        cp = cp.add(1);
    }

    for i in 0..glyph_count {
        let g = glyphs.gids[i];
        *cp = (g >> 8) as u8;
        cp = cp.add(1);
        *cp = g as u8;
        cp = cp.add(1);
    }

    cp.offset_from(xdv_ptr) as i32
}

// translated from xetex-shipout.c:1470 (reverse)
pub fn reverse(
    mem: &mut MemState,
    shipout: &mut ShipoutState,
    scanner: &mut ScannerState,
    fonts: &mut FontState,
    this_box: i32,
    t: i32,
    cur_g: &mut i32,
    cur_glue: &mut f64,
) -> Result<i32, ShipoutAbort> {
    // `this_box` is the box being reversed (always an h/vlist).
    let bb = mem
        .classify_box(MemIdx::new(this_box))
        .expect("valid box")
        .expect("reverse box");
    let g_order = bb.glue_order(mem);
    let g_sign = bb.glue_sign(mem);
    let mut l = t;
    let mut p = mem.temp_ptr.0;
    let mut m = MIN_HALFWORD;
    let mut n = MIN_HALFWORD;

    loop {
        'nodes: while p != TEX_NULL {
            if crate::mem::is_char_node(mem, p) {
                loop {
                    let f = mem.b16s1(p) as i32;
                    let c = mem.b16s0(p);
                    let ec = crate::fonts::effective_char(fonts, true, f, c);
                    shipout.cur_h += font_character_width(fonts, f, ec);
                    let q = mem.b32s1(p);
                    mem.set_b32s1(p, l);
                    l = p;
                    p = q;
                    if !crate::mem::is_char_node(mem, p) {
                        break;
                    }
                }
                continue;
            }

            let q = mem.b32s1(p);
            let mut add_width = true;

            loop {
                match mem.b16s1(p) {
                    HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                        shipout.rule_wd = mem.b32s1(p + 1);
                    }
                    WHATSIT_NODE => {
                        if WhatsitSubtype::from_raw(mem.b16s0(p))
                            .is_some_and(|s| s.has_dimensions())
                        {
                            shipout.rule_wd = mem.b32s1(p + 1);
                        } else {
                            add_width = false;
                        }
                    }
                    GLUE_NODE => {
                        let mut g = mem.b32s0(p + 1);
                        shipout.rule_wd = mem.b32s1(g + 1) - *cur_g;

                        if g_sign != GlueSign::Normal {
                            if g_sign == GlueSign::Stretching {
                                if mem.b16s1(g) == g_order {
                                    *cur_glue += mem.b32s1(g + 2) as f64;
                                    let mut glue_temp = bb.glue_set(mem) * *cur_glue;
                                    glue_temp = glue_temp.clamp(-1_000_000_000.0, 1_000_000_000.0);
                                    *cur_g = crate::scaled_math::tex_round(glue_temp);
                                }
                            } else if mem.b16s0(g) == g_order {
                                *cur_glue -= mem.b32s1(g + 3) as f64;
                                let mut glue_temp = bb.glue_set(mem) * *cur_glue;
                                glue_temp = glue_temp.clamp(-1_000_000_000.0, 1_000_000_000.0);
                                *cur_g = crate::scaled_math::tex_round(glue_temp);
                            }
                        }

                        shipout.rule_wd += *cur_g;

                        if (g_sign == GlueSign::Stretching && mem.b16s1(g) == g_order)
                            || (g_sign == GlueSign::Shrinking && mem.b16s0(g) == g_order)
                        {
                            if mem.b32s1(g) == TEX_NULL {
                                crate::mem::free_node(
                                    mem,
                                    crate::units::MemPtr::from_raw(g),
                                    GLUE_SPEC_SIZE,
                                );
                            } else {
                                mem.set_b32s1(g, mem.b32s1(g) - 1);
                            }

                            if mem.b16s0(p) < A_LEADERS {
                                mem.set_b16s1(p, KERN_NODE);
                                mem.set_b32s1(p + 1, shipout.rule_wd);
                            } else {
                                g = get_node_ship(mem, GLUE_SPEC_SIZE)?;
                                mem.set_b16s1(g, FILLL + 1);
                                mem.set_b16s0(g, FILLL + 1);
                                mem.set_b32s1(g + 1, shipout.rule_wd);
                                mem.set_b32s1(g + 2, 0);
                                mem.set_b32s1(g + 3, 0);
                                mem.set_b32s0(p + 1, g);
                            }
                        }
                    }
                    LIGATURE_NODE => {
                        crate::mem::flush_list(
                            mem,
                            crate::units::MemPtr::from_raw(mem.b32s1(p + 1)),
                        );
                        mem.temp_ptr.0 = p;
                        p = get_avail_ship(mem)?.0;
                        mem.set_word(p, mem.word(mem.temp_ptr + 1));
                        mem.set_b32s1(p, q);
                        crate::mem::free_node(mem, mem.temp_ptr, SMALL_NODE_SIZE);
                        continue 'nodes;
                    }
                    MATH_NODE => {
                        shipout.rule_wd = mem.b32s1(p + 1);

                        if !mem.b16s0(p).is_multiple_of(2) {
                            if mem.b32s0(scanner.lr_ptr) as u16 != math_node_end_lr_type(mem, p) {
                                mem.set_b16s1(p, KERN_NODE);
                                scanner.lr_problems += 1;
                            } else {
                                mem.temp_ptr.0 = scanner.lr_ptr.0;
                                scanner.lr_ptr.0 = mem.b32s1(mem.temp_ptr);
                                mem.set_b32s1(mem.temp_ptr, mem.avail);
                                mem.avail = mem.temp_ptr.0;

                                if n > MIN_HALFWORD {
                                    n -= 1;
                                    mem.set_b16s0(p, mem.b16s0(p) - 1);
                                } else {
                                    mem.set_b16s1(p, KERN_NODE);

                                    if m > MIN_HALFWORD {
                                        m -= 1;
                                    } else {
                                        crate::mem::free_node(
                                            mem,
                                            crate::units::MemPtr::from_raw(p),
                                            MEDIUM_NODE_SIZE,
                                        );
                                        mem.set_b32s1(t, q);
                                        mem.set_b32s1(t + 1, shipout.rule_wd);
                                        mem.set_b32s1(t + 2, -shipout.cur_h - shipout.rule_wd);
                                        return Ok(l);
                                    }
                                }
                            }
                        } else {
                            let tp = get_avail_ship(mem)?;
                            mem.temp_ptr = tp;
                            let v = math_node_end_lr_type(mem, p) as i32;
                            tp.set_info(mem, v);
                            tp.set_link(mem, scanner.lr_ptr);
                            scanner.lr_ptr = tp;

                            if n > MIN_HALFWORD
                                || math_node_lr_dir(mem, p) as i16 != scanner.cur_dir
                            {
                                n += 1;
                                mem.set_b16s0(p, mem.b16s0(p) + 1);
                            } else {
                                mem.set_b16s1(p, KERN_NODE);
                                m += 1;
                            }
                        }
                    }
                    EDGE_NODE => {
                        return Err(ShipoutAbort::ConfusionLr2);
                    }
                    _ => add_width = false,
                }
                break;
            }

            if add_width {
                shipout.cur_h += shipout.rule_wd;
            }

            mem.set_b32s1(p, l);

            if mem.b16s1(p) == KERN_NODE && (shipout.rule_wd == 0 || l == TEX_NULL) {
                crate::mem::free_node(mem, crate::units::MemPtr::from_raw(p), MEDIUM_NODE_SIZE);
                p = l;
            }

            l = p;
            p = q;
        }

        if t == TEX_NULL && m == MIN_HALFWORD && n == MIN_HALFWORD {
            return Ok(l);
        }

        p = crate::mem::alloc_math(mem, 0, mem.b32s0(scanner.lr_ptr) as i16)
            .ok_or(ShipoutAbort::MainMemoryOverflow)?
            .ptr()
            .0;
        scanner.lr_problems += 10000;
    }
}

fn movement_bytes(mut w: i32, o: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(5);
    if w.abs() >= 0x800000 {
        out.push(o.wrapping_add(3));
        if w >= 0 {
            out.push((w / 0x1000000) as u8);
        } else {
            w += 0x40000000;
            w += 0x40000000;
            out.push(((w / 0x1000000) + 128) as u8);
        }
        w %= 0x1000000;
        out.push((w / 0x10000) as u8);
        w %= 0x10000;
        out.push((w / 0x100) as u8);
        out.push((w % 0x100) as u8);
    } else if w.abs() >= 0x8000 {
        out.push(o.wrapping_add(2));
        if w < 0 {
            w += 0x1000000;
        }
        out.push((w / 0x10000) as u8);
        w %= 0x10000;
        out.push((w / 256) as u8);
        out.push((w % 256) as u8);
    } else if w.abs() >= 128 {
        out.push(o.wrapping_add(1));
        if w < 0 {
            w += 0x10000;
        }
        out.push((w / 256) as u8);
        out.push((w % 256) as u8);
    } else {
        out.push(o);
        if w < 0 {
            w += 256;
        }
        out.push((w % 256) as u8);
    }
    out
}

// translated from xetex-shipout.c:1763 (movement)
pub fn movement(mem: &mut MemState, shipout: &mut ShipoutState, w: i32, o: u8) -> Vec<u8> {
    let Some(mut q) = NodePtr::<MovementNode>::alloc(mem) else {
        return Vec::new();
    };
    q.set_distance(mem, Scaled(w));
    q.set_location(mem, shipout.dvi_offset + shipout.dvi_ptr);

    if o == DOWN1 {
        q.set_link(mem, MemPtr::from_raw(shipout.down_ptr.0));
        shipout.down_ptr.0 = q.idx().0;
    } else {
        q.set_link(mem, MemPtr::from_raw(shipout.right_ptr.0));
        shipout.right_ptr.0 = q.idx().0;
    }

    let mut p = NodePtr::<MovementNode>::from_ptr(q.link(mem));
    let mut mstate: i32 = MOV_NONE_SEEN;

    // 'search either breaks with Some(found_node) or falls through with None.
    let found_p: Option<NodePtr<MovementNode>> = 'search: {
        while let Some(p_node) = p {
            if p_node.distance(mem).0 == w {
                match mstate + p_node.state(mem) {
                    // NONE_SEEN+YZ_OK=3, NONE_SEEN+Y_OK=4, Z_SEEN+YZ_OK=15, Z_SEEN+Y_OK=16
                    3 | 4 | 15 | 16 => {
                        if p_node.location(mem) < shipout.dvi_gone {
                            break 'search None;
                        }
                        let mut k = p_node.location(mem) - shipout.dvi_offset;
                        if k < 0 {
                            k += DVI_BUF_SIZE;
                        }
                        shipout.dvi_buf[k as usize] += 5;
                        shipout
                            .page_manifest
                            .patch_byte(p_node.location(mem) as u64, 5);
                        p_node.set_state(mem, MOV_Y_HERE);
                        break 'search Some(p_node);
                    }
                    // NONE_SEEN+Z_OK=5, Y_SEEN+YZ_OK=9, Y_SEEN+Z_OK=11
                    5 | 9 | 11 => {
                        if p_node.location(mem) < shipout.dvi_gone {
                            break 'search None;
                        }
                        let mut k = p_node.location(mem) - shipout.dvi_offset;
                        if k < 0 {
                            k += DVI_BUF_SIZE;
                        }
                        shipout.dvi_buf[k as usize] += 10;
                        shipout
                            .page_manifest
                            .patch_byte(p_node.location(mem) as u64, 10);
                        p_node.set_state(mem, MOV_Z_HERE);
                        break 'search Some(p_node);
                    }
                    // NONE_SEEN+Y_HERE=1, NONE_SEEN+Z_HERE=2, Y_SEEN+Z_HERE=8, Z_SEEN+Y_HERE=13
                    1 | 2 | 8 | 13 => break 'search Some(p_node),
                    _ => {}
                }
            } else {
                match mstate + p_node.state(mem) {
                    1 => mstate = MOV_Y_SEEN,     // NONE_SEEN+Y_HERE
                    2 => mstate = MOV_Z_SEEN,     // NONE_SEEN+Z_HERE
                    8 | 13 => break 'search None, // Y_SEEN+Z_HERE, Z_SEEN+Y_HERE
                    _ => {}
                }
            }
            p = NodePtr::<MovementNode>::from_ptr(p_node.link(mem));
        }
        None
    };

    match found_p {
        None => {
            // not_found: emit the shortest encoding
            q.set_state(mem, MOV_YZ_OK);
            movement_bytes(w, o)
        }
        Some(p) => {
            // found: reuse previous y/z command
            q.set_state(mem, p.state(mem));
            if q.state(mem) == MOV_Y_HERE {
                let out = vec![o.wrapping_add(4)];
                while q.link(mem) != MemPtr::from_idx(p.idx()) {
                    q = NodePtr::<MovementNode>::from_ptr(q.link(mem))
                        .expect("found movement record is linked from the new record");
                    match q.state(mem) {
                        MOV_YZ_OK => q.set_state(mem, MOV_Z_OK),
                        MOV_Y_OK => q.set_state(mem, MOV_D_FIXED),
                        _ => {}
                    }
                }
                out
            } else {
                let out = vec![o.wrapping_add(9)];
                while q.link(mem) != MemPtr::from_idx(p.idx()) {
                    q = NodePtr::<MovementNode>::from_ptr(q.link(mem))
                        .expect("found movement record is linked from the new record");
                    match q.state(mem) {
                        MOV_YZ_OK => q.set_state(mem, MOV_Y_OK),
                        MOV_Z_OK => q.set_state(mem, MOV_D_FIXED),
                        _ => {}
                    }
                }
                out
            }
        }
    }
}

// translated from xetex-shipout.c:1930 (special_out)
fn special_out_in(
    st: &mut crate::state::EngineState,
    p: impl crate::units::MemAddr,
) -> Result<(), ShipoutAbort> {
    let p = p.addr();
    if st.shipout.cur_h != st.shipout.dvi_h {
        let d = st.shipout.cur_h - st.shipout.dvi_h;
        emit_movement(&mut st.mem, &mut st.shipout, d, RIGHT1)?;
        st.shipout.dvi_h = st.shipout.cur_h;
    }
    if st.shipout.cur_v != st.shipout.dvi_v {
        let d = st.shipout.cur_v - st.shipout.dvi_v;
        emit_movement(&mut st.mem, &mut st.shipout, d, DOWN1)?;
        st.shipout.dvi_v = st.shipout.cur_v;
    }
    st.shipout.doing_special = true;
    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    let w = st
        .mem
        .classify_whatsit(MemIdx::new(p))
        .expect("valid whatsit")
        .expect("special node");
    let tok = st.mem.b32s1(w.write_tokens(&st.mem).0);
    let avail = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list_in(st, tok, TEX_NULL, avail);
    st.shipout.selector = old_setting;

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(ShipoutAbort::PoolSizeOverflow);
    }

    let base = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
    let cur_len = st.strings.pool_ptr - base;
    if cur_len < 256 {
        emit_dvi_out(&mut st.shipout, XXX1)?;
        emit_dvi_out(&mut st.shipout, cur_len as u8)?;
    } else {
        emit_dvi_out(&mut st.shipout, XXX4)?;
        emit_dvi_four(&mut st.shipout, cur_len)?;
    }

    let mut k = base;
    let for_end = st.strings.pool_ptr - 1;
    while k <= for_end {
        let byte = st.strings.str_pool[k as usize] as u8;
        emit_dvi_out(&mut st.shipout, byte)?;
        k += 1;
    }
    st.strings.pool_ptr = base;
    st.shipout.doing_special = false;
    Ok(())
}

// translated from xetex-shipout.c:1976 (write_out)
fn write_out_in(
    st: &mut crate::state::EngineState,
    p: impl crate::units::MemAddr,
) -> Result<(), ShipoutAbort> {
    let p = p.addr();
    let q = get_avail_ship(&mut st.mem)?;
    q.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
    let r = get_avail_ship(&mut st.mem)?;
    q.set_link(&mut st.mem, r);
    r.set_info(&mut st.mem, CS_TOKEN_FLAG + END_WRITE);
    crate::input::begin_token_list_in(st, q.0, INSERTED)?;
    let write_text_ptr = st
        .mem
        .classify_whatsit(MemIdx::new(p))
        .expect("valid whatsit")
        .expect("write node")
        .write_tokens(&st.mem)
        .0;
    crate::input::begin_token_list_in(st, write_text_ptr, WRITE_TEXT)?;
    let q2 = get_avail_ship(&mut st.mem)?;
    q2.set_info(&mut st.mem, LEFT_BRACE_TOKEN + b'{' as i32);
    crate::input::begin_token_list_in(st, q2.0, INSERTED)?;

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
    let w = st
        .mem
        .classify_whatsit(MemIdx::new(p))
        .expect("valid whatsit")
        .expect("write whatsit");
    let j = w.write_stream(&st.mem);

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

    let def_ref = st.scanner.def_ref.0;
    crate::output::token_show_in(st, def_ref);
    crate::output::print_ln_in(st);
    crate::mem::flush_list(&mut st.mem, crate::units::MemPtr::from_raw(def_ref));

    if j == 18 {
        if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) <= 0 {
            st.shipout.selector = SELECTOR_LOG_ONLY;
        } else {
            st.shipout.selector = SELECTOR_TERM_AND_LOG;
        }
        if !st.errors.log_opened {
            st.shipout.selector = SELECTOR_TERM_ONLY;
        }

        // Tectonic: don't emit warnings for shell-escape when enabled;
        // the driver provides a better UX.
        if !st.config.shell_escape_enabled {
            crate::output::diagnostic_begin_capture_warning_here_in(st);
            crate::output::print_nl_cstr_in(st, c"runsystem(".as_ptr());
            let base = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            let cur_len = st.strings.pool_ptr - base;
            for d in 0..cur_len {
                crate::output::print_in(st, st.strings.str_pool[(base + d) as usize] as i32);
            }
            crate::output::print_cstr_in(st, c")...".as_ptr());
            crate::output::print_cstr_in(st, c"disabled".as_ptr());
            crate::output::print_char_in(st, b'.' as i32);
            crate::output::capture_to_diagnostic_in(st, core::ptr::null_mut());
            crate::output::print_nl_cstr_in(st, c"".as_ptr());
            crate::output::print_ln_in(st);
        } else {
            let base = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            let cur_len = st.strings.pool_ptr - base;
            // SAFETY: FFI; the str_pool Vec never reallocates across this call.
            let ptr = unsafe { st.strings.str_pool.as_ptr().add(base as usize) };
            // SAFETY: hands `cur_len` UTF-16 units to the shell-escape driver.
            unsafe { ttstub_shell_escape(ptr, cur_len as libc::size_t) };
        }

        st.strings.pool_ptr = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
    }

    st.shipout.selector = old_setting;
    Ok(())
}

// translated from xetex-shipout.c:2071 (pic_out)
pub(crate) fn pic_out_in(
    st: &mut crate::state::EngineState,
    p: impl crate::units::MemAddr,
) -> Result<(), ShipoutAbort> {
    let p = p.addr();
    if st.shipout.cur_h != st.shipout.dvi_h {
        let d = st.shipout.cur_h - st.shipout.dvi_h;
        emit_movement(&mut st.mem, &mut st.shipout, d, RIGHT1)?;
        st.shipout.dvi_h = st.shipout.cur_h;
    }
    if st.shipout.cur_v != st.shipout.dvi_v {
        let d = st.shipout.cur_v - st.shipout.dvi_v;
        emit_movement(&mut st.mem, &mut st.shipout, d, DOWN1)?;
        st.shipout.dvi_v = st.shipout.cur_v;
    }

    let old_setting = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    crate::output::print_cstr_in(st, c"pdf:image ".as_ptr());
    crate::output::print_cstr_in(st, c"matrix ".as_ptr());
    crate::output::print_scaled_in(st, st.mem.b32s0(p + 5));
    crate::output::print_in(st, b' ' as i32);
    crate::output::print_scaled_in(st, st.mem.b32s1(p + 5));
    crate::output::print_in(st, b' ' as i32);
    crate::output::print_scaled_in(st, st.mem.b32s0(p + 6));
    crate::output::print_in(st, b' ' as i32);
    crate::output::print_scaled_in(st, st.mem.b32s1(p + 6));
    crate::output::print_in(st, b' ' as i32);
    crate::output::print_scaled_in(st, st.mem.b32s0(p + 7));
    crate::output::print_in(st, b' ' as i32);
    crate::output::print_scaled_in(st, st.mem.b32s1(p + 7));
    crate::output::print_in(st, b' ' as i32);
    crate::output::print_cstr_in(st, c"page ".as_ptr());
    crate::output::print_int_in(st, st.mem.b16s0(p + 4) as i32);
    crate::output::print_in(st, b' ' as i32);

    if let Some(label) =
        crate::pic::PageboxType::from_raw(st.mem.b16s1(p + 8)).and_then(|pb| pb.label())
    {
        crate::output::print_cstr_in(st, label.as_ptr());
    }

    crate::output::print_in(st, b'(' as i32);
    let path_len = st.mem.b16s1(p + 4) as i32;
    let path_ptr = st.mem.node_addr(p + PIC_NODE_SIZE) as *const u8;
    for i in 0..path_len {
        // SAFETY: the path bytes follow the PIC node; `path_ptr` stays valid
        // (the arena never reallocates) across the print reborrow.
        let ch = unsafe { *path_ptr.add(i as usize) } as u16;
        crate::output::print_raw_char_in(st, ch, true);
    }
    crate::output::print_in(st, b')' as i32);

    st.shipout.selector = old_setting;
    let base = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
    let cur_len = st.strings.pool_ptr - base;
    if cur_len < 256 {
        emit_dvi_out(&mut st.shipout, XXX1)?;
        emit_dvi_out(&mut st.shipout, cur_len as u8)?;
    } else {
        emit_dvi_out(&mut st.shipout, XXX4)?;
        emit_dvi_four(&mut st.shipout, cur_len)?;
    }

    let mut k = base;
    while k < st.strings.pool_ptr {
        let byte = st.strings.str_pool[k as usize] as u8;
        emit_dvi_out(&mut st.shipout, byte)?;
        k += 1;
    }
    st.strings.pool_ptr = base;
    Ok(())
}

// translated from xetex-shipout.c:1682 (out_what)
pub(crate) fn out_what_in(
    st: &mut crate::state::EngineState,
    p: impl crate::units::MemAddr,
) -> Result<(), ShipoutAbort> {
    let p = p.addr();
    let sub = WhatsitSubtype::from_raw(st.mem.b16s0(p));
    match sub {
        Some(WhatsitSubtype::Open) | Some(WhatsitSubtype::Write) | Some(WhatsitSubtype::Close) => {
            if !st.shipout.doing_leaders {
                let w = st
                    .mem
                    .classify_whatsit(MemIdx::new(p))
                    .expect("valid whatsit")
                    .expect("open/write/close whatsit");
                let j = w.write_stream(&st.mem);
                if sub == Some(WhatsitSubtype::Write) {
                    write_out_in(st, p)?;
                    return Ok(());
                }
                if st.shipout.write_open[j as usize] {
                    ensure_write_stream_open(st, j as usize)?;
                    // SAFETY: FFI close of the write-stream output handle.
                    unsafe { ttstub_output_close(st.shipout.write_file[j as usize]) };
                }
                if sub == Some(WhatsitSubtype::Close) {
                    st.shipout.write_open[j as usize] = false;
                    if j < 16 {
                        st.shipout.write_file[j as usize] = INVALID_HANDLE;
                    }
                    return Ok(());
                }
                // OPEN_NODE
                if j < 16 {
                    st.scanner.cur_name = w.open_name(&st.mem);
                    st.scanner.cur_area = w.open_area(&st.mem);
                    st.scanner.cur_ext = w.open_ext(&st.mem);
                    if crate::string_pool::length(&st.strings, st.scanner.cur_ext) == 0 {
                        // SAFETY: FFI string interning; deferred overflow tail.
                        st.scanner.cur_ext = unsafe {
                            crate::texmfmp::maketexstring_in(&mut st.strings, c".tex".as_ptr())
                        }
                        .map_err(|e| ShipoutAbort::Scc(crate::scan::SccAbort::MakeTexString(e)))?;
                    }
                    let (n, a, e) = (st.scanner.cur_name, st.scanner.cur_area, st.scanner.cur_ext);
                    // SAFETY: packs the file name into the shared name buffer.
                    unsafe { crate::io::pack_file_name_in(st, n, a, e) };
                    // SAFETY: FFI open with the packed name buffer.
                    let handle = unsafe { ttstub_output_open(st.input.name_of_file.as_ptr(), 0) };
                    if handle == INVALID_HANDLE {
                        return Err(ShipoutAbort::CannotOpenOutput);
                    }
                    st.shipout.write_file[j as usize] = handle;
                    st.shipout.write_open[j as usize] = true;
                    let name_len = st.input.name_length as usize;
                    st.shipout.write_name[j as usize].clear();
                    st.shipout.write_name[j as usize]
                        .extend_from_slice(&st.input.name_of_file[..=name_len]);
                    st.shipout.write_prefix[j as usize].clear();
                    if st.errors.log_opened {
                        let old_setting = st.shipout.selector;
                        if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) <= 0 {
                            st.shipout.selector = SELECTOR_LOG_ONLY;
                        } else {
                            st.shipout.selector = SELECTOR_TERM_AND_LOG;
                        }
                        crate::output::print_nl_cstr_in(st, c"\\openout".as_ptr());
                        crate::output::print_int_in(st, j);
                        crate::output::print_cstr_in(st, c" = `".as_ptr());
                        let (n, a, e) =
                            (st.scanner.cur_name, st.scanner.cur_area, st.scanner.cur_ext);
                        crate::output::print_file_name_in(st, n, a, e);
                        crate::output::print_cstr_in(st, c"'.".as_ptr());
                        crate::output::print_nl_cstr_in(st, c"".as_ptr());
                        crate::output::print_ln_in(st);
                        st.shipout.selector = old_setting;
                    }
                }
            }
        }
        Some(WhatsitSubtype::Special) => special_out_in(st, p)?,
        Some(WhatsitSubtype::Language) => {}
        _ => return Err(ShipoutAbort::Confusion(c"ext4")),
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
