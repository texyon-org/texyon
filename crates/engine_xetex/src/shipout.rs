//! Shipout helpers translated from `xetex/xetex-shipout.c`.
use crate::consts::{
    CS_TOKEN_FLAG, DIMEN_BASE, GLUE_NODE, GLUE_SPEC_SIZE, HLIST_NODE, INT_BASE, KERN_NODE,
    LIGATURE_NODE, MATH_NODE, MAX_HALFWORD, MEDIUM_NODE_SIZE, MIN_HALFWORD, NULL_FLAG,
    PENALTY_NODE, RULE_NODE, SELECTOR_TERM_AND_LOG, SELECTOR_TERM_ONLY, SMALL_NODE_SIZE, TEX_NULL,
    TOO_BIG_CHAR,
};
use crate::ext::apply_tfm_font_mapping;
use crate::ext::makeXDVGlyphArrayData;
use crate::ext::make_font_def_global;

use crate::globals::{
    avail, char_base_entry, cur_area, cur_dir, cur_ext, cur_h, cur_list_mode, cur_name,
    cur_page_height, cur_tok, cur_v, def_ref, doing_leaders, eqtb_b32s1, file_offset,
    font_area_entry, font_bc_entry, font_ec_entry, font_info_b16, font_mapping_ptr, font_ptr,
    font_used_entry, init_pool_ptr, job_name, last_bop, log_opened, lr_problems, lr_ptr, max_h,
    max_print_line, max_push, max_v, mem_as_u16_ptr, mem_b16s0, mem_b16s1, mem_b16s2, mem_b32s0,
    mem_b32s1, mem_gr, mem_node_addr, mem_ptr, mem_word, name_of_file, pool_ptr, pool_size,
    rule_ht, rule_wd, rust_stdout, selector, semantic_pagination_enabled, set_avail, set_cur_area,
    set_cur_cs, set_cur_dir, set_cur_ext, set_cur_h, set_cur_h_offset, set_cur_list_mode,
    set_cur_name, set_cur_page_height, set_cur_page_width, set_cur_v, set_cur_v_offset,
    set_dead_cycles, set_doing_leaders, set_font_ptr, set_font_used_entry, set_help_line,
    set_help_ptr, set_last_bop, set_lr_problems, set_lr_ptr, set_max_h, set_max_push, set_max_v,
    set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1, set_mem_word, set_pdf_last_x_pos,
    set_pdf_last_y_pos, set_pool_ptr, set_rule_dp, set_rule_ht, set_rule_wd, set_selector,
    set_temp_ptr, set_total_pages, set_write_file, set_write_open, set_xtx_ligature_present,
    shell_escape_enabled, str_pool_entry, str_pool_ptr, str_ptr, str_start, temp_ptr, term_offset,
    total_pages, write_file, write_loc, write_open, xdv_buffer_entry,
};
use crate::mem::{free_node_global, get_avail_global, get_node_global, new_kern_global};
use crate::node::{EdgeNode, MovementNode, NodePtr};
use crate::state::{EqtbState, FontState, MemState, ScannerState, ShipoutState, StringsState};
use crate::units::{MemIdx, MemPtr, Scaled};

extern "C" {
    fn ttstub_output_write(
        handle: usize,
        data: *const libc::c_char,
        len: libc::size_t,
    ) -> libc::size_t;
    fn ttstub_output_close(handle: usize) -> libc::c_int;
    fn ttstub_output_open(path: *const libc::c_char, is_gz: libc::c_int) -> usize;
    fn ttstub_output_flush(handle: usize) -> libc::c_int;
    fn ttstub_shell_escape(cmd: *const u16, len: libc::size_t);
    fn _tt_abort(fmt: *const libc::c_char) -> !;

    static output_file_extension: *const libc::c_char;
}

// DVI output file state — positioning and output variables bridged to ShipoutState.
fn dvi_file() -> usize {
    crate::state::with_state(|s| s.shipout.dvi_file)
}
fn set_dvi_file(v: usize) {
    crate::state::with_state_mut(|s| s.shipout.dvi_file = v);
}

fn output_file_name() -> i32 {
    crate::state::with_state(|s| s.shipout.output_file_name)
}
fn set_output_file_name(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.output_file_name = v);
}

fn dvi_limit() -> i32 {
    crate::state::with_state(|s| s.shipout.dvi_limit)
}
fn set_dvi_limit(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.dvi_limit = v);
}

fn dvi_ptr() -> i32 {
    crate::state::with_state(|s| s.shipout.dvi_ptr)
}
fn set_dvi_ptr(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.dvi_ptr = v);
}

fn dvi_offset() -> i32 {
    crate::state::with_state(|s| s.shipout.dvi_offset)
}
fn set_dvi_offset(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.dvi_offset = v);
}

fn set_dvi_gone(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.dvi_gone = v);
}

fn cur_s() -> i32 {
    crate::state::with_state(|s| s.shipout.cur_s)
}
fn set_cur_s(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.cur_s = v);
}

fn dvi_h() -> i32 {
    crate::state::with_state(|s| s.shipout.dvi_h)
}
fn set_dvi_h(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.dvi_h = v);
}

fn dvi_v() -> i32 {
    crate::state::with_state(|s| s.shipout.dvi_v)
}
fn set_dvi_v(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.dvi_v = v);
}

fn dvi_f() -> i32 {
    crate::state::with_state(|s| s.shipout.dvi_f)
}
fn set_dvi_f(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.dvi_f = v);
}

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
const MARGIN_KERN_NODE: u16 = 40;
const MEM_TOP: i32 = 4_999_999;
const AAT_FONT_FLAG: i32 = 65533;
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

// Whatsit subtype codes — xetex_format.h:457–461
const OPEN_NODE: u16 = 0;
const WRITE_NODE: u16 = 1;
const CLOSE_NODE: u16 = 2;
const SPECIAL_NODE: u16 = 3;
const LANGUAGE_NODE: u16 = 4;
const NATIVE_WORD_NODE: u16 = 40;
const NATIVE_WORD_NODE_AT: u16 = 41;
const GLYPH_NODE: u16 = 42;
const PIC_NODE: u16 = 43;
const PDF_NODE: u16 = 44;
const PDF_SAVE_POS_NODE: u16 = 21;

// Glue and leader constants: xetex-constants.h:175,182,275,281; xetex_format.h:422.
const FILLL: u16 = 3;
const NORMAL: u16 = 0;
const STRETCHING: u16 = 1;
const SHRINKING: u16 = 2;
const A_LEADERS: u16 = 100;
const C_LEADERS: u16 = 101;
const SPACE_ADJUSTMENT: u16 = 3;

// MATH_NODE LR constants: xetex-constants.h:142-143; xetex_format.h:437.
const L_CODE: u16 = 4;
const R_CODE: u16 = 8;
const END_M_CODE: u16 = 3;
const BEFORE: i32 = 0;
const DLIST: u16 = 2;
const REVERSED: u16 = 1;
const LIG_TRICK: i32 = 4_999_987;

const INT_PAR_XETEX_INTERWORD_SPACE_SHAPING: i32 = 78;

// xetex-constants.h:133, tectonic_bridge_core_generated.h:12
const PIC_NODE_SIZE: i32 = 9;
const INVALID_HANDLE: usize = 0;

// INT_PAR index for \tracingonline — xetex_format.h
const INT_PAR_TRACING_ONLINE: i32 = 29;

// Token constants — xetex-constants.h:204,217,348–351
const INSERTED: u16 = 5;
const WRITE_TEXT: u16 = 18;
const LEFT_BRACE_TOKEN: i32 = 0x20_0000;
const RIGHT_BRACE_TOKEN: i32 = 0x40_0000;
// END_WRITE = FROZEN_CONTROL_SEQUENCE + 8 = 2_243_226 + 8
const END_WRITE: i32 = 2_243_234;

// Output selector values — xetex-core.h:43–50
const SELECTOR_LOG_ONLY: i32 = 18;
const SELECTOR_NEW_STRING: i32 = 21;

/// Create a new edge node of subtype `s` and width `w`.
///
/// translated from xetex-shipout.c:1708 (new_edge)
pub fn new_edge(mem: &mut MemState, s: i16, w: i32) -> i32 {
    let Some(p) = NodePtr::<EdgeNode>::alloc(mem) else {
        return crate::mem::ALLOC_FAILED;
    };
    p.set_subtype(mem, s as u16);
    p.set_width(mem, Scaled(w));
    p.set_edge_dist(mem, Scaled(0));
    p.idx().0
}

pub unsafe fn new_edge_global(s: i16, w: i32) -> i32 {
    let p = crate::state::with_state_mut(|st| new_edge(&mut st.mem, s, w));
    if p == crate::mem::ALLOC_FAILED {
        crate::errors::overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    p
}

/// What a threaded DVI byte write needs its bridge-down wrapper to do
/// *after* the `with_state_mut` borrow drops: file I/O and the cold overflow
/// abort must not run while the thread-local is borrowed (§5 sentinel rule).
enum DviFlush {
    /// Flush `dvi_buf[a..=b]` to the output file.
    Write(i32, i32),
    /// DVI length overflowed 0x7FFFFFFF — abort via `fatal_error`.
    Fatal,
}

/// Cold `-> !` abort sentinel for threaded shipout helpers: the work is done
/// under the live state borrow and the divergent abort is fired by the thin
/// `_global` wrapper after the borrow drops (§5 sentinel rule).
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
}

/// Fire a deferred shipout cold abort after the state borrow has dropped (§5).
unsafe fn fire_shipout_abort(a: ShipoutAbort) -> ! {
    match a {
        ShipoutAbort::ConfusionLr2 => crate::errors::confusion(c"LR2".as_ptr()),
        ShipoutAbort::PoolSizeOverflow => {
            crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr())
        }
        ShipoutAbort::NodeRecycle(e) => crate::scan::node_recycle_abort(e),
        ShipoutAbort::Confusion(msg) => crate::errors::confusion(msg.as_ptr()),
        ShipoutAbort::DviWriteFailed => _tt_abort(c"failed to write data to XDV file".as_ptr()),
        ShipoutAbort::DviLengthOverflow => {
            crate::errors::fatal_error(c"dvi length exceeds 0x7FFFFFFF".as_ptr());
            unreachable!()
        }
        ShipoutAbort::MainMemoryOverflow => {
            crate::errors::overflow(c"main memory size".as_ptr(), MEM_TOP + 1)
        }
    }
}

/// Threaded analog of `write_to_dvi_global`: flush `dvi_buf[a..=b]` to the
/// output file by reading the buffer fields directly under the held borrow.
/// The cold I/O-failure abort is deferred (§5).
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

/// Perform a deferred DVI buffer flush under the held borrow (§5).
fn handle_dvi_flush(shipout: &ShipoutState, flush: Option<DviFlush>) -> Result<(), ShipoutAbort> {
    match flush {
        None => Ok(()),
        Some(DviFlush::Write(a, b)) => write_to_dvi(shipout, a, b),
        Some(DviFlush::Fatal) => Err(ShipoutAbort::DviLengthOverflow),
    }
}

/// Threaded analog of `dvi_out_global`.
fn emit_dvi_out(shipout: &mut ShipoutState, c: u8) -> Result<(), ShipoutAbort> {
    let flush = dvi_out(shipout, c);
    handle_dvi_flush(shipout, flush)
}

/// Threaded analog of `dvi_four_global`.
fn emit_dvi_four(shipout: &mut ShipoutState, x: i32) -> Result<(), ShipoutAbort> {
    let mut start = 0;
    while start < 4 {
        let result = dvi_four(shipout, x, start);
        start = result.next;
        handle_dvi_flush(shipout, result.flush)?;
    }
    Ok(())
}

/// Threaded analog of `movement_global`: build the movement bytes and emit them.
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

unsafe fn dvi_four_global(x: i32) {
    let mut start = 0;
    while start < 4 {
        let result = crate::state::with_state_mut(|s| dvi_four(&mut s.shipout, x, start));
        start = result.next;
        match result.flush {
            None => {}
            Some(DviFlush::Write(a, b)) => write_to_dvi_global(a, b),
            Some(DviFlush::Fatal) => {
                crate::errors::fatal_error(c"dvi length exceeds 0x7FFFFFFF".as_ptr());
            }
        }
    }
}

/// translated from xetex-shipout.c:2423 (dvi_two)
fn dvi_two(shipout: &mut ShipoutState, s: u16, start: usize) -> DviEmit {
    emit_dvi_bytes(shipout, &[(s / 0x100) as u8, (s % 0x100) as u8], start)
}

unsafe fn dvi_two_global(s: u16) {
    let mut start = 0;
    while start < 2 {
        let result = crate::state::with_state_mut(|st| dvi_two(&mut st.shipout, s, start));
        start = result.next;
        match result.flush {
            None => {}
            Some(DviFlush::Write(a, b)) => write_to_dvi_global(a, b),
            Some(DviFlush::Fatal) => {
                crate::errors::fatal_error(c"dvi length exceeds 0x7FFFFFFF".as_ptr());
            }
        }
    }
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

/// translated from xetex-shipout.c:1798 (dvi_font_def)
fn dvi_font_def(shipout: &mut ShipoutState, bytes: &[u8], start: usize) -> DviEmit {
    emit_dvi_bytes(shipout, bytes, start)
}

unsafe fn dvi_font_def_global(f: i32) {
    let bytes = if font_area_entry(f) == AAT_FONT_FLAG || font_area_entry(f) == OTGR_FONT_FLAG {
        let font_def_length = make_font_def_global(f);
        crate::state::with_state(|s| dvi_native_font_def(&s.fonts, f, font_def_length))
    } else {
        crate::state::with_state(|s| {
            let font_area = s.fonts.font_area_entry(f);
            let font_name = s.fonts.font_name[f as usize];
            dvi_font_def_bytes(
                &s.fonts,
                &s.strings,
                f,
                font_area,
                strings_length(&s.strings, font_area),
                font_name,
                strings_length(&s.strings, font_name),
            )
        })
    };
    let mut start = 0;
    while start < bytes.len() {
        let result = crate::state::with_state_mut(|s| dvi_font_def(&mut s.shipout, &bytes, start));
        start = result.next;
        match result.flush {
            None => {}
            Some(DviFlush::Write(a, b)) => write_to_dvi_global(a, b),
            Some(DviFlush::Fatal) => {
                crate::errors::fatal_error(c"dvi length exceeds 0x7FFFFFFF".as_ptr());
            }
        }
    }
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
pub fn initialize_shipout_variables() {
    set_output_file_name(0);
    crate::state::with_state_mut(|s| {
        s.shipout.dvi_buf = vec![0u8; (DVI_BUF_SIZE + 1) as usize];
    });
    set_dvi_limit(DVI_BUF_SIZE);
    set_dvi_ptr(0);
    set_dvi_offset(0);
    set_dvi_gone(0);
    crate::state::with_state_mut(|s| {
        s.shipout.down_ptr = TEX_NULL;
        s.shipout.right_ptr = TEX_NULL;
    });
    set_cur_s(-1);
}

// translated from xetex-shipout.c:63 (deinitialize_shipout_variables)
pub fn deinitialize_shipout_variables() {
    crate::state::with_state_mut(|s| {
        s.shipout.dvi_buf = Vec::new();
    });
}

// translated from xetex-shipout.c:2296 (write_to_dvi)
//
// Borrow-dropping byte-range flush. The FFI write / `_tt_abort` must not run
// while the thread-local is borrowed (the abort longjmp would leak the
// `RefCell` guard), so it grabs (file, ptr) under a short `with_state` borrow
// and writes outside it — matching the pre-thread structure. Threads to a
// direct `&ShipoutState` helper once its callers (`finalize_dvi_file`,
// `dvi_swap`) are threaded.
pub unsafe fn write_to_dvi_global(a: i32, b: i32) {
    let n = (b - a + 1) as libc::size_t;
    let (file, ptr) = crate::state::with_state(|s| {
        // The buffer Vec never reallocates, so this raw ptr stays valid past
        // the borrow (same escaping-pointer idiom as `buffer_ptr`).
        (
            s.shipout.dvi_file,
            s.shipout.dvi_buf.as_ptr().add(a as usize),
        )
    });
    if ttstub_output_write(file, ptr as *const libc::c_char, n) != n {
        _tt_abort(c"failed to write data to XDV file".as_ptr());
    }
}

// translated from xetex-shipout.c:2306 (dvi_swap)
//
// Direct `&mut ShipoutState` buffer-swap bookkeeping. The byte flush and the
// cold overflow abort are returned to the wrapper instead of run under the
// borrow. The state updates are reordered before the (deferred) flush, which
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
// Direct `&mut ShipoutState` byte write. Returns the deferred flush action
// (if the buffer filled) for the bridge-down wrapper to perform after the
// borrow drops.
fn dvi_out(shipout: &mut ShipoutState, c: u8) -> Option<DviFlush> {
    let ptr = shipout.dvi_ptr;
    shipout.dvi_buf[ptr as usize] = c;
    shipout.dvi_ptr += 1;
    if shipout.dvi_ptr == shipout.dvi_limit {
        Some(dvi_swap(shipout))
    } else {
        None
    }
}

pub unsafe fn dvi_out_global(c: u8) {
    let flush = crate::state::with_state_mut(|s| dvi_out(&mut s.shipout, c));
    match flush {
        None => {}
        Some(DviFlush::Write(a, b)) => write_to_dvi_global(a, b),
        Some(DviFlush::Fatal) => {
            crate::errors::fatal_error(c"dvi length exceeds 0x7FFFFFFF".as_ptr());
        }
    }
}

// translated from xetex-shipout.c:56 (ship_out)
pub unsafe fn ship_out(p: i32) {
    let mag = eqtb_b32s1(INT_BASE + INT_PAR_MAG);
    let synctex_job_name = job_name();
    let synctex_tmp = crate::texmfmp::gettexstring(synctex_job_name);
    crate::state::with_state_mut(|st| crate::synctex::synctex_sheet(st, mag, synctex_tmp));

    if job_name() == 0 {
        crate::io::open_log_file();
    }

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) > 0 {
        crate::output::print_nl_cstr(c"".as_ptr());
        crate::output::print_ln();
        crate::output::print_cstr(c"Completed box being shipped out".as_ptr());
    }

    if term_offset() > max_print_line() - 9 {
        crate::output::print_ln();
    } else if term_offset() > 0 || file_offset() > 0 {
        crate::output::print_char(b' ' as i32);
    }

    crate::output::print_char(b'[' as i32);
    let mut j: u8 = 9;
    while j > 0 && eqtb_b32s1(COUNT_BASE + j as i32) == 0 {
        j -= 1;
    }

    for k in 0..=j {
        crate::output::print_int(eqtb_b32s1(COUNT_BASE + k as i32));
        if k < j {
            crate::output::print_char(b'.' as i32);
        }
    }

    ttstub_output_flush(rust_stdout());

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) > 0 {
        crate::output::print_char(b']' as i32);
        crate::output::begin_diagnostic();
        crate::output::show_box(p);
        crate::output::end_diagnostic(true);
    }

    if mem_b32s1(p + 3) > MAX_HALFWORD
        || mem_b32s1(p + 2) > MAX_HALFWORD
        || mem_b32s1(p + 3) + mem_b32s1(p + 2) + eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET)
            > MAX_HALFWORD
        || mem_b32s1(p + 1) + eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET) > MAX_HALFWORD
    {
        crate::output::error_here_with_diagnostic(c"Huge page cannot be shipped out".as_ptr());
        crate::output::capture_to_diagnostic(core::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"The page just created is more than 18 feet tall or".as_ptr(),
        );
        set_help_line(
            0,
            c"more than 18 feet wide, so I suspect something went wrong.".as_ptr(),
        );
        crate::errors::error();

        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) <= 0 {
            crate::output::begin_diagnostic();
            crate::output::print_nl_cstr(c"The following box has been deleted:".as_ptr());
            crate::output::show_box(p);
            crate::output::end_diagnostic(true);
        }
    } else {
        let page_height =
            mem_b32s1(p + 3) + mem_b32s1(p + 2) + eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET);
        if page_height > max_v() {
            set_max_v(page_height);
        }

        let page_width = mem_b32s1(p + 1) + eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET);
        if page_width > max_h() {
            set_max_h(page_width);
        }

        set_dvi_h(0);
        set_dvi_v(0);
        set_cur_h(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET));
        set_dvi_f(FONT_BASE);

        set_cur_h_offset(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET) + ONE_INCH);
        set_cur_v_offset(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET) + ONE_INCH);

        if eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH) != 0 {
            set_cur_page_width(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH));
        } else {
            set_cur_page_width(
                mem_b32s1(p + 1) + 2 * (eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET) + ONE_INCH),
            );
        }

        if eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT) != 0 {
            set_cur_page_height(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT));
        } else {
            set_cur_page_height(
                mem_b32s1(p + 3)
                    + mem_b32s1(p + 2)
                    + 2 * (eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET) + ONE_INCH),
            );
        }

        if output_file_name() == 0 {
            if job_name() == 0 {
                crate::io::open_log_file();
            }
            crate::io::pack_job_name(output_file_extension);
            set_dvi_file(ttstub_output_open(name_of_file(), 0));
            if dvi_file() == INVALID_HANDLE {
                let mut buf = [0u8; 512];
                libc::snprintf(
                    buf.as_mut_ptr() as *mut libc::c_char,
                    buf.len(),
                    c"cannot open output file \"%s\"".as_ptr(),
                    name_of_file(),
                );
                _tt_abort(buf.as_ptr() as *const libc::c_char);
            }
            set_output_file_name(crate::io::make_name_string());
        }

        if total_pages() == 0 {
            dvi_out_global(PRE);
            if semantic_pagination_enabled() {
                dvi_out_global(SPX_ID_BYTE);
            } else {
                dvi_out_global(XDV_ID_BYTE);
            }
            dvi_four_global(25_400_000);
            dvi_four_global(473_628_672);
            crate::save::prepare_mag();
            dvi_four_global(eqtb_b32s1(INT_BASE + INT_PAR_MAG));

            let output_comment = c"tectonic";
            let l = output_comment.to_bytes().len() as u8;
            dvi_out_global(l);
            for s in 0..l {
                dvi_out_global(*output_comment.as_ptr().add(s as usize) as u8);
            }
        }

        let page_loc = dvi_offset() + dvi_ptr();
        dvi_out_global(BOP);
        for k in 0..10 {
            dvi_four_global(eqtb_b32s1(COUNT_BASE + k));
        }
        dvi_four_global(last_bop());
        set_last_bop(page_loc);

        let old_setting = selector();
        set_selector(SELECTOR_NEW_STRING);
        crate::output::print_cstr(c"pdf:pagesize ".as_ptr());
        if eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH) <= 0
            || eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT) <= 0
        {
            crate::output::print_cstr(c"default".as_ptr());
        } else {
            crate::output::print_cstr(c"width".as_ptr());
            crate::output::print(b' ' as i32);
            crate::output::print_scaled(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH));
            crate::output::print_cstr(c"pt".as_ptr());
            crate::output::print(b' ' as i32);
            crate::output::print_cstr(c"height".as_ptr());
            crate::output::print(b' ' as i32);
            crate::output::print_scaled(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT));
            crate::output::print_cstr(c"pt".as_ptr());
        }
        set_selector(old_setting);

        dvi_out_global(XXX1);
        dvi_out_global(crate::hash::cur_length_global() as u8);
        let start = str_start(str_ptr() - TOO_BIG_CHAR);
        let end = pool_ptr();
        for s in start..end {
            dvi_out_global(str_pool_entry(s) as u8);
        }
        set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));

        set_cur_v(mem_b32s1(p + 3) + eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET));
        set_temp_ptr(p);
        if mem_b16s1(p) == VLIST_NODE {
            vlist_out();
        } else {
            hlist_out();
        }

        dvi_out_global(EOP);
        set_total_pages(total_pages() + 1);
        set_cur_s(-1);
    }

    if lr_problems() > 0 {
        crate::output::print_ln();
        crate::output::print_nl_cstr(c"\\endL or \\endR problem (".as_ptr());
        crate::output::print_int(lr_problems() / 10000);
        crate::output::print_cstr(c" missing, ".as_ptr());
        crate::output::print_int(lr_problems() % 10000);
        crate::output::print_cstr(c" extra".as_ptr());
        set_lr_problems(0);
        crate::output::print_char(b')' as i32);
        crate::output::print_ln();
    }

    if lr_ptr() != TEX_NULL || cur_dir() != LEFT_TO_RIGHT {
        crate::errors::confusion(c"LR3".as_ptr());
    }

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) <= 0 {
        crate::output::print_char(b']' as i32);
    }

    set_dead_cycles(0);
    ttstub_output_flush(rust_stdout());
    crate::packaging::flush_node_list_global(p);
    crate::state::with_state_mut(|st| crate::synctex::synctex_teehs(st));
}

// translated from xetex-shipout.c:2332 (dvi_pop)
pub fn dvi_pop(shipout: &mut ShipoutState, l: i32) -> bool {
    if l == shipout.dvi_offset + shipout.dvi_ptr && shipout.dvi_ptr > 0 {
        shipout.dvi_ptr -= 1;
        false
    } else {
        true
    }
}

pub unsafe fn dvi_pop_global(l: i32) {
    let emit_pop = crate::state::with_state_mut(|st| dvi_pop(&mut st.shipout, l));
    if emit_pop {
        dvi_out_global(POP);
    }
}

fn ensure_dvi_font(shipout: &mut ShipoutState, bytes: &[u8], start: usize) -> DviEmit {
    emit_dvi_bytes(shipout, bytes, start)
}

unsafe fn ensure_dvi_font_global(f: i32) {
    if f == dvi_f() {
        return;
    }

    if !font_used_entry(f) {
        dvi_font_def_global(f);
        set_font_used_entry(f, true);
    }

    let bytes = dvi_font_select_bytes(f);
    let mut start = 0;
    while start < bytes.len() {
        let result =
            crate::state::with_state_mut(|s| ensure_dvi_font(&mut s.shipout, &bytes, start));
        start = result.next;
        match result.flush {
            None => {}
            Some(DviFlush::Write(a, b)) => write_to_dvi_global(a, b),
            Some(DviFlush::Fatal) => {
                crate::errors::fatal_error(c"dvi length exceeds 0x7FFFFFFF".as_ptr());
            }
        }
    }
    crate::state::with_state_mut(|s| {
        s.shipout.dvi_f = f;
    });
}

#[inline]
fn is_ignorable_interword_node(mem: &MemState, p: i32) -> bool {
    !crate::mem::is_char_node(mem, p)
        && (mem.b16s1(p) == PENALTY_NODE
            || mem.b16s1(p) == INS_NODE
            || mem.b16s1(p) == MARK_NODE
            || mem.b16s1(p) == ADJUST_NODE
            || (mem.b16s1(p) == WHATSIT_NODE && mem.b16s0(p) <= LANGUAGE_NODE))
}

fn skip_ignorable_interword_nodes(mem: &MemState, mut q: i32) -> i32 {
    while q != TEX_NULL && is_ignorable_interword_node(mem, q) {
        q = mem.b32s1(q);
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
    g_sign: u16,
) -> Result<(), ShipoutAbort> {
    let mut p = mem.b32s1(this_box + 5);
    let mut prev_p = this_box + 5;

    while p != TEX_NULL {
        if mem.b32s1(p) != TEX_NULL
            && !crate::mem::is_char_node(mem, p)
            && mem.b16s1(p) == WHATSIT_NODE
            && (mem.b16s0(p) == NATIVE_WORD_NODE || mem.b16s0(p) == NATIVE_WORD_NODE_AT)
            && fonts.font_letter_space_entry(mem.b16s2(p + 4) as i32) == 0
        {
            let r = p;
            let mut k = mem.b16s1(r + 4) as i32;
            let mut q = mem.b32s1(p);

            'check_next: loop {
                q = skip_ignorable_interword_nodes(mem, q);

                if q != TEX_NULL && !crate::mem::is_char_node(mem, q) {
                    if mem.b16s1(q) == GLUE_NODE && mem.b16s0(mem.b32s0(q + 1)) == NORMAL {
                        if mem.b32s0(q + 1) == fonts.font_glue(mem.b16s2(r + 4) as i32) {
                            q = skip_ignorable_interword_nodes(mem, mem.b32s1(q));

                            if q != TEX_NULL
                                && !crate::mem::is_char_node(mem, q)
                                && mem.b16s1(q) == WHATSIT_NODE
                                && (mem.b16s0(q) == NATIVE_WORD_NODE
                                    || mem.b16s0(q) == NATIVE_WORD_NODE_AT)
                                && mem.b16s2(q + 4) == mem.b16s2(r + 4)
                            {
                                p = q;
                                k += 1 + mem.b16s1(q + 4) as i32;
                                q = mem.b32s1(q);
                                continue 'check_next;
                            }
                        } else {
                            q = mem.b32s1(q);
                        }

                        if q != TEX_NULL
                            && !crate::mem::is_char_node(mem, q)
                            && mem.b16s1(q) == KERN_NODE
                            && mem.b16s0(q) == SPACE_ADJUSTMENT
                        {
                            q = skip_ignorable_interword_nodes(mem, mem.b32s1(q));

                            if q != TEX_NULL
                                && !crate::mem::is_char_node(mem, q)
                                && mem.b16s1(q) == WHATSIT_NODE
                                && (mem.b16s0(q) == NATIVE_WORD_NODE
                                    || mem.b16s0(q) == NATIVE_WORD_NODE_AT)
                                && mem.b16s2(q + 4) == mem.b16s2(r + 4)
                            {
                                p = q;
                                k += 1 + mem.b16s1(q + 4) as i32;
                                q = mem.b32s1(q);
                                continue 'check_next;
                            }
                        }
                    } else if mem.b16s1(q) == WHATSIT_NODE
                        && (mem.b16s0(q) == NATIVE_WORD_NODE || mem.b16s0(q) == NATIVE_WORD_NODE_AT)
                        && mem.b16s2(q + 4) == mem.b16s2(r + 4)
                    {
                        p = q;
                        q = mem.b32s1(q);
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
                        if mem.b16s0(q) == NATIVE_WORD_NODE || mem.b16s0(q) == NATIVE_WORD_NODE_AT {
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

                        if g_sign != NORMAL {
                            if g_sign == STRETCHING {
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
                    q = mem.b32s1(q);
                }

                let nlen = crate::hash::cur_length(strings);
                q = crate::fonts::new_native_word_node(mem, eqtb, mem.b16s2(r + 4) as i32, nlen)
                    .map_err(ShipoutAbort::NodeRecycle)?;
                mem.set_b16s0(q, mem.b16s0(r));

                let base = strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize];
                // SAFETY: copying `nlen` UTF-16 codes from the pool tail into the
                // freshly-allocated native-word node text region.
                let src = unsafe { strings.str_pool.as_ptr().add(base as usize) };
                let dst = mem.as_u16_ptr_mut(q + 6);
                for j in 0..nlen {
                    // SAFETY: both regions hold at least `nlen` u16 codes.
                    unsafe { *dst.add(j as usize) = *src.add(j as usize) };
                }

                mem.set_b32s1(q + 1, k);
                let addr = mem.node_addr(q);
                // SAFETY: `addr` points at the native-word node; FFI does not re-enter.
                unsafe { crate::ext::store_justified_native_glyphs(fonts, addr) };
                mem.set_b32s1(prev_p, q);
                mem.set_b32s1(q, mem.b32s1(p));
                mem.set_b32s1(p, TEX_NULL);
                prev_p = r;
                p = mem.b32s1(r);

                while p != TEX_NULL {
                    if is_ignorable_interword_node(mem, p) {
                        mem.set_b32s1(prev_p, mem.b32s1(p));
                        mem.set_b32s1(p, mem.b32s1(q));
                        mem.set_b32s1(q, p);
                        q = p;
                    }

                    prev_p = p;
                    p = mem.b32s1(p);
                }

                crate::packaging::flush_node_list(mem, r).map_err(ShipoutAbort::Confusion)?;
                strings.pool_ptr = strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize];
                p = q;
            }
        }

        prev_p = p;
        p = mem.b32s1(p);
    }

    Ok(())
}

// Bridge-down wrapper; retires when `hlist_out` is threaded.
unsafe fn merge_justifiable_native_words_global(this_box: i32, g_order: u16, g_sign: u16) {
    let outcome = crate::state::with_state_mut(|st| {
        let s = &mut *st;
        merge_justifiable_native_words(
            &mut s.mem,
            &mut s.strings,
            &s.fonts,
            &s.eqtb,
            this_box,
            g_order,
            g_sign,
        )
    });
    if let Err(a) = outcome {
        fire_shipout_abort(a);
    }
}

struct HListRuleAction {
    emit: bool,
    h_move: Option<i32>,
    v_move: Option<i32>,
    rule_ht: i32,
    rule_wd: i32,
    synctex_p: i32,
    synctex_box: i32,
}

fn finish_hlist_rule(
    mem: &MemState,
    shipout: &mut ShipoutState,
    this_box: i32,
    base_line: i32,
    p: i32,
) -> HListRuleAction {
    let bb = mem
        .classify_box(MemIdx::new(this_box))
        .expect("valid node type for hlist_out box")
        .expect("hlist_out operates on a box");
    if shipout.rule_ht == NULL_FLAG {
        shipout.rule_ht = bb.height(mem).0;
    }
    if shipout.rule_dp == NULL_FLAG {
        shipout.rule_dp = bb.depth(mem).0;
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
        synctex_box: this_box,
    }
}

pub unsafe fn finish_hlist_rule_global(this_box: i32, base_line: i32, p: i32) {
    let action = crate::state::with_state_mut(|st| {
        finish_hlist_rule(&st.mem, &mut st.shipout, this_box, base_line, p)
    });
    if action.emit {
        if let Some(h_move) = action.h_move {
            movement_global(h_move, RIGHT1);
        }
        if let Some(v_move) = action.v_move {
            movement_global(v_move, DOWN1);
        }
        dvi_out_global(SET_RULE);
        dvi_four_global(action.rule_ht);
        dvi_four_global(action.rule_wd);
    }
    crate::state::with_state_mut(|st| {
        crate::synctex::synctex_horizontal_rule_or_glue(st, action.synctex_p, action.synctex_box)
    });
}

unsafe fn output_hlist_glue(
    this_box: i32,
    base_line: i32,
    left_edge: i32,
    p: i32,
    g_order: u16,
    g_sign: u16,
    cur_g: &mut i32,
    cur_glue: &mut f64,
) {
    let mut g = mem_b32s0(p + 1);
    set_rule_wd(mem_b32s1(g + 1) - *cur_g);

    if g_sign != NORMAL {
        if g_sign == STRETCHING {
            if mem_b16s1(g) == g_order {
                *cur_glue += mem_b32s1(g + 2) as f64;
                *cur_g = crate::scaled_math::tex_round(
                    (mem_gr(this_box + 6) * *cur_glue).clamp(-1_000_000_000.0, 1_000_000_000.0),
                );
            }
        } else if mem_b16s0(g) == g_order {
            *cur_glue -= mem_b32s1(g + 3) as f64;
            *cur_g = crate::scaled_math::tex_round(
                (mem_gr(this_box + 6) * *cur_glue).clamp(-1_000_000_000.0, 1_000_000_000.0),
            );
        }
    }

    set_rule_wd(rule_wd() + *cur_g);

    if (g_sign == STRETCHING && mem_b16s1(g) == g_order)
        || (g_sign == SHRINKING && mem_b16s0(g) == g_order)
    {
        if mem_b32s1(g) == TEX_NULL {
            free_node_global(g, GLUE_SPEC_SIZE);
        } else {
            set_mem_b32s1(g, mem_b32s1(g) - 1);
        }

        if mem_b16s0(p) < A_LEADERS {
            set_mem_b16s1(p, KERN_NODE);
            set_mem_b32s1(p + 1, rule_wd());
        } else {
            g = get_node_global(GLUE_SPEC_SIZE);
            set_mem_b16s1(g, FILLL + 1);
            set_mem_b16s0(g, FILLL + 1);
            set_mem_b32s1(g + 1, rule_wd());
            set_mem_b32s1(g + 2, 0);
            set_mem_b32s1(g + 3, 0);
            set_mem_b32s0(p + 1, g);
        }
    }

    if mem_b16s0(p) >= A_LEADERS {
        let leader_box = mem_b32s1(p + 1);
        if mem_b16s1(leader_box) == RULE_NODE {
            set_rule_ht(mem_b32s1(leader_box + 3));
            set_rule_dp(mem_b32s1(leader_box + 2));
            finish_hlist_rule_global(this_box, base_line, p);
            return;
        }

        let leader_wd = mem_b32s1(leader_box + 1);
        if leader_wd > 0 && rule_wd() > 0 {
            set_rule_wd(rule_wd() + 10);
            if cur_dir() == RIGHT_TO_LEFT {
                set_cur_h(cur_h() - 10);
            }
            let edge = cur_h() + rule_wd();
            let mut lx = 0;

            if mem_b16s0(p) == A_LEADERS {
                let save_h = cur_h();
                set_cur_h(left_edge + leader_wd * ((cur_h() - left_edge) / leader_wd));
                if cur_h() < save_h {
                    set_cur_h(cur_h() + leader_wd);
                }
            } else {
                let lq = rule_wd() / leader_wd;
                let lr = rule_wd() % leader_wd;
                if mem_b16s0(p) == C_LEADERS {
                    set_cur_h(cur_h() + (lr / 2));
                } else {
                    lx = lr / (lq + 1);
                    set_cur_h(cur_h() + ((lr - (lq - 1) * lx) / 2));
                }
            }

            while cur_h() + leader_wd <= edge {
                set_cur_v(base_line + mem_b32s1(leader_box + 4));
                if cur_v() != dvi_v() {
                    movement_global(cur_v() - dvi_v(), DOWN1);
                    set_dvi_v(cur_v());
                }
                let save_v = dvi_v();

                if cur_h() != dvi_h() {
                    movement_global(cur_h() - dvi_h(), RIGHT1);
                    set_dvi_h(cur_h());
                }
                let save_h = dvi_h();

                set_temp_ptr(leader_box);
                if cur_dir() == RIGHT_TO_LEFT {
                    set_cur_h(cur_h() + leader_wd);
                }

                let outer_doing_leaders = doing_leaders();
                set_doing_leaders(true);
                if mem_b16s1(leader_box) == VLIST_NODE {
                    vlist_out();
                } else {
                    hlist_out();
                }
                set_doing_leaders(outer_doing_leaders);

                set_dvi_v(save_v);
                set_dvi_h(save_h);
                set_cur_v(base_line);
                set_cur_h(save_h + leader_wd + lx);
            }

            if cur_dir() == RIGHT_TO_LEFT {
                set_cur_h(edge);
            } else {
                set_cur_h(edge - 10);
            }
            return;
        }
    }

    set_cur_h(cur_h() + rule_wd());
    crate::state::with_state_mut(|st| {
        crate::synctex::synctex_horizontal_rule_or_glue(st, p, this_box)
    });
}

// translated from xetex-shipout.c:63 (hlist_out)
pub unsafe fn hlist_out() {
    let mut cur_g = 0;
    let mut cur_glue = 0.0;
    let this_box = temp_ptr();
    let g_order = mem_b16s0(this_box + 5);
    let g_sign = mem_b16s1(this_box + 5);

    if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_INTERWORD_SPACE_SHAPING) > 1 {
        merge_justifiable_native_words_global(this_box, g_order, g_sign);
    }

    let mut p = mem_b32s1(this_box + 5);
    set_cur_s(cur_s() + 1);
    if cur_s() > 0 {
        dvi_out_global(PUSH);
    }
    if cur_s() > max_push() {
        set_max_push(cur_s());
    }

    let save_loc = dvi_offset() + dvi_ptr();
    let base_line = cur_v();
    let mut prev_p = this_box + 5;

    set_temp_ptr(get_avail_global());
    set_mem_b32s0(temp_ptr(), BEFORE);
    set_mem_b32s1(temp_ptr(), lr_ptr());
    set_lr_ptr(temp_ptr());

    if mem_b16s0(this_box) == DLIST {
        if cur_dir() == RIGHT_TO_LEFT {
            set_cur_dir(LEFT_TO_RIGHT);
            set_cur_h(cur_h() - mem_b32s1(this_box + 1));
        } else {
            set_mem_b16s0(this_box, 0);
        }
    }

    if cur_dir() == RIGHT_TO_LEFT && mem_b16s0(this_box) != REVERSED {
        let save_h = cur_h();
        set_temp_ptr(p);
        p = new_kern_global(0);
        set_mem_b32s0(p + 2, 0);
        set_mem_b32s1(prev_p, p);
        set_cur_h(0);
        set_mem_b32s1(
            p,
            reverse_global(this_box, TEX_NULL, &mut cur_g, &mut cur_glue),
        );
        set_mem_b32s1(p + 1, -cur_h());
        set_cur_h(save_h);
        set_mem_b16s0(this_box, REVERSED);
    }

    let mut left_edge = cur_h();
    crate::state::with_state_mut(|st| crate::synctex::synctex_hlist(st, this_box));

    while p != TEX_NULL {
        'reswitch: loop {
            if is_char_node_global(p) {
                if cur_h() != dvi_h() {
                    movement_global(cur_h() - dvi_h(), RIGHT1);
                    set_dvi_h(cur_h());
                }
                if cur_v() != dvi_v() {
                    movement_global(cur_v() - dvi_v(), DOWN1);
                    set_dvi_v(cur_v());
                }

                while is_char_node_global(p) {
                    let f = mem_b16s1(p) as i32;
                    let mut c = mem_b16s0(p);
                    if p != LIG_TRICK && !font_mapping_ptr(f).is_null() {
                        c = apply_tfm_font_mapping(font_mapping_ptr(f), c as libc::c_int) as u16;
                    }

                    ensure_dvi_font_global(f);
                    if font_ec_entry(f) >= c && font_bc_entry(f) <= c {
                        let info = font_info_b16(char_base_entry(f) + c as i32);
                        if info.s3 > 0 {
                            if c >= 128 {
                                dvi_out_global(SET1);
                            }
                            dvi_out_global(c as u8);
                            set_cur_h(cur_h() + font_character_width_global(f, c as i32));
                        }
                    }

                    prev_p = mem_b32s1(prev_p);
                    p = mem_b32s1(p);
                }

                crate::state::with_state_mut(|st| crate::synctex::synctex_current(st));
                set_dvi_h(cur_h());
            } else {
                match mem_b16s1(p) {
                    HLIST_NODE | VLIST_NODE => {
                        if mem_b32s1(p + 5) == TEX_NULL {
                            if mem_b16s1(p) == VLIST_NODE {
                                crate::state::with_state_mut(|st| {
                                    crate::synctex::synctex_void_vlist(st, p, this_box)
                                });
                            } else {
                                crate::state::with_state_mut(|st| {
                                    crate::synctex::synctex_void_hlist(st, p, this_box)
                                });
                            }
                            set_cur_h(cur_h() + mem_b32s1(p + 1));
                        } else {
                            let save_h = dvi_h();
                            let save_v = dvi_v();
                            set_cur_v(base_line + mem_b32s1(p + 4));
                            set_temp_ptr(p);
                            let edge = cur_h() + mem_b32s1(p + 1);
                            if cur_dir() == RIGHT_TO_LEFT {
                                set_cur_h(edge);
                            }
                            if mem_b16s1(p) == VLIST_NODE {
                                vlist_out();
                            } else {
                                hlist_out();
                            }
                            set_dvi_h(save_h);
                            set_dvi_v(save_v);
                            set_cur_h(edge);
                            set_cur_v(base_line);
                        }
                    }
                    RULE_NODE => {
                        set_rule_ht(mem_b32s1(p + 3));
                        set_rule_dp(mem_b32s1(p + 2));
                        set_rule_wd(mem_b32s1(p + 1));
                        finish_hlist_rule_global(this_box, base_line, p);
                    }
                    WHATSIT_NODE => match mem_b16s0(p) {
                        NATIVE_WORD_NODE | NATIVE_WORD_NODE_AT | GLYPH_NODE => {
                            if cur_h() != dvi_h() {
                                movement_global(cur_h() - dvi_h(), RIGHT1);
                                set_dvi_h(cur_h());
                            }
                            if cur_v() != dvi_v() {
                                movement_global(cur_v() - dvi_v(), DOWN1);
                                set_dvi_v(cur_v());
                            }
                            let f = mem_b16s2(p + 4) as i32;
                            ensure_dvi_font_global(f);

                            if mem_b16s0(p) == GLYPH_NODE {
                                dvi_out_global(SET_GLYPHS);
                                dvi_four_global(mem_b32s1(p + 1));
                                dvi_two_global(1);
                                dvi_four_global(0);
                                dvi_four_global(0);
                                dvi_two_global(mem_b16s1(p + 4));
                                set_cur_h(cur_h() + mem_b32s1(p + 1));
                            } else {
                                if mem_b16s0(p) == NATIVE_WORD_NODE_AT {
                                    if mem_b16s1(p + 4) > 0 || !mem_ptr(p + 5).is_null() {
                                        dvi_out_global(SET_TEXT_AND_GLYPHS);
                                        let len = mem_b16s1(p + 4) as i32;
                                        dvi_two_global(len as u16);
                                        let text = mem_as_u16_ptr(p + 6);
                                        for k in 0..len {
                                            dvi_two_global(*text.add(k as usize));
                                        }
                                        let len = makeXDVGlyphArrayData(mem_node_addr(p));
                                        for k in 0..len {
                                            dvi_out_global(xdv_buffer_entry(k));
                                        }
                                    }
                                } else if !mem_ptr(p + 5).is_null() {
                                    dvi_out_global(SET_GLYPHS);
                                    let len = makeXDVGlyphArrayData(mem_node_addr(p));
                                    for k in 0..len {
                                        dvi_out_global(xdv_buffer_entry(k));
                                    }
                                }
                                set_cur_h(cur_h() + mem_b32s1(p + 1));
                            }
                            set_dvi_h(cur_h());
                        }
                        PIC_NODE | PDF_NODE => {
                            let save_h = dvi_h();
                            let save_v = dvi_v();
                            set_cur_v(base_line);
                            let edge = cur_h() + mem_b32s1(p + 1);
                            pic_out(p);
                            set_dvi_h(save_h);
                            set_dvi_v(save_v);
                            set_cur_h(edge);
                            set_cur_v(base_line);
                        }
                        PDF_SAVE_POS_NODE => {
                            set_pdf_last_x_pos(cur_h() + ONE_INCH_LESS_ONE);
                            set_pdf_last_y_pos(cur_page_height() - cur_v() - ONE_INCH_LESS_ONE);
                        }
                        _ => out_what(p),
                    },
                    GLUE_NODE => {
                        output_hlist_glue(
                            this_box,
                            base_line,
                            left_edge,
                            p,
                            g_order,
                            g_sign,
                            &mut cur_g,
                            &mut cur_glue,
                        );
                    }
                    MARGIN_KERN_NODE => set_cur_h(cur_h() + mem_b32s1(p + 1)),
                    KERN_NODE => {
                        crate::state::with_state_mut(|st| {
                            crate::synctex::synctex_kern(st, p, this_box)
                        });
                        set_cur_h(cur_h() + mem_b32s1(p + 1));
                    }
                    MATH_NODE => {
                        crate::state::with_state_mut(|st| {
                            crate::synctex::synctex_math(st, p, this_box)
                        });
                        if !mem_b16s0(p).is_multiple_of(2) {
                            if mem_b32s0(lr_ptr()) as u16 == math_node_end_lr_type_global(p) {
                                set_temp_ptr(lr_ptr());
                                set_lr_ptr(mem_b32s1(temp_ptr()));
                                set_mem_b32s1(temp_ptr(), avail());
                                set_avail(temp_ptr());
                            } else if mem_b16s0(p) > L_CODE {
                                set_lr_problems(lr_problems() + 1);
                            }
                        } else {
                            set_temp_ptr(get_avail_global());
                            set_mem_b32s0(temp_ptr(), math_node_end_lr_type_global(p) as i32);
                            set_mem_b32s1(temp_ptr(), lr_ptr());
                            set_lr_ptr(temp_ptr());

                            if math_node_lr_dir_global(p) as i16 != cur_dir() {
                                let save_h = cur_h();
                                set_temp_ptr(mem_b32s1(p));
                                set_rule_wd(mem_b32s1(p + 1));
                                free_node_global(p, MEDIUM_NODE_SIZE);
                                set_cur_dir(1 - cur_dir());
                                p = new_edge_global(cur_dir(), rule_wd());
                                set_mem_b32s1(prev_p, p);
                                set_cur_h(cur_h() - left_edge + rule_wd());
                                set_mem_b32s1(
                                    p,
                                    reverse_global(
                                        this_box,
                                        new_edge_global(1 - cur_dir(), 0),
                                        &mut cur_g,
                                        &mut cur_glue,
                                    ),
                                );
                                set_mem_b32s1(p + 2, cur_h());
                                set_cur_dir(1 - cur_dir());
                                set_cur_h(save_h);
                                continue 'reswitch;
                            }
                        }
                        set_mem_b16s1(p, KERN_NODE);
                        set_cur_h(cur_h() + mem_b32s1(p + 1));
                    }
                    LIGATURE_NODE => {
                        set_mem_word(LIG_TRICK, mem_word(p + 1));
                        set_mem_b32s1(LIG_TRICK, mem_b32s1(p));
                        p = LIG_TRICK;
                        set_xtx_ligature_present(true);
                        continue 'reswitch;
                    }
                    EDGE_NODE => {
                        set_cur_h(cur_h() + mem_b32s1(p + 1));
                        left_edge = cur_h() + mem_b32s1(p + 2);
                        set_cur_dir(mem_b16s0(p) as i16);
                    }
                    _ => {}
                }

                prev_p = p;
                p = mem_b32s1(p);
            }
            break;
        }
    }

    crate::state::with_state_mut(|st| crate::synctex::synctex_tsilh(st, this_box));

    while mem_b32s0(lr_ptr()) != BEFORE {
        if mem_b32s0(lr_ptr()) as u16 > L_CODE {
            set_lr_problems(lr_problems() + 10000);
        }
        set_temp_ptr(lr_ptr());
        set_lr_ptr(mem_b32s1(temp_ptr()));
        set_mem_b32s1(temp_ptr(), avail());
        set_avail(temp_ptr());
    }

    set_temp_ptr(lr_ptr());
    set_lr_ptr(mem_b32s1(temp_ptr()));
    set_mem_b32s1(temp_ptr(), avail());
    set_avail(temp_ptr());

    if mem_b16s0(this_box) == DLIST {
        set_cur_dir(RIGHT_TO_LEFT);
    }

    prune_movements_global(save_loc);
    if cur_s() > 0 {
        dvi_pop_global(save_loc);
    }
    set_cur_s(cur_s() - 1);
}

// translated from xetex-shipout.c:866 (vlist_out)
pub unsafe fn vlist_out() {
    let mut cur_g = 0;
    let mut cur_glue = 0.0;
    let this_box = temp_ptr();
    let g_order = mem_b16s0(this_box + 5);
    let g_sign = mem_b16s1(this_box + 5);
    let mut p = mem_b32s1(this_box + 5);
    let upwards = mem_b16s0(this_box) == 1;

    set_cur_s(cur_s() + 1);
    if cur_s() > 0 {
        dvi_out_global(PUSH);
    }

    if cur_s() > max_push() {
        set_max_push(cur_s());
    }

    let save_loc = dvi_offset() + dvi_ptr();
    let left_edge = cur_h();
    crate::state::with_state_mut(|st| crate::synctex::synctex_vlist(st, this_box));

    if upwards {
        set_cur_v(cur_v() + mem_b32s1(this_box + 2));
    } else {
        set_cur_v(cur_v() - mem_b32s1(this_box + 3));
    }

    let top_edge = cur_v();

    while p != TEX_NULL {
        if is_char_node_global(p) {
            crate::errors::confusion(c"vlistout".as_ptr());
        }

        let mut go_next = true;
        match mem_b16s1(p) {
            HLIST_NODE | VLIST_NODE => {
                if mem_b32s1(p + 5) == TEX_NULL {
                    if upwards {
                        set_cur_v(cur_v() - mem_b32s1(p + 2));
                    } else {
                        set_cur_v(cur_v() + mem_b32s1(p + 3));
                    }

                    if mem_b16s1(p) == VLIST_NODE {
                        crate::state::with_state_mut(|st| {
                            crate::synctex::synctex_void_vlist(st, p, this_box)
                        });
                    } else {
                        crate::state::with_state_mut(|st| {
                            crate::synctex::synctex_void_hlist(st, p, this_box)
                        });
                    }

                    if upwards {
                        set_cur_v(cur_v() - mem_b32s1(p + 3));
                    } else {
                        set_cur_v(cur_v() + mem_b32s1(p + 2));
                    }
                } else {
                    if upwards {
                        set_cur_v(cur_v() - mem_b32s1(p + 2));
                    } else {
                        set_cur_v(cur_v() + mem_b32s1(p + 3));
                    }

                    if cur_v() != dvi_v() {
                        movement_global(cur_v() - dvi_v(), DOWN1);
                        set_dvi_v(cur_v());
                    }

                    let save_h = dvi_h();
                    let save_v = dvi_v();

                    if cur_dir() == RIGHT_TO_LEFT {
                        set_cur_h(left_edge - mem_b32s1(p + 4));
                    } else {
                        set_cur_h(left_edge + mem_b32s1(p + 4));
                    }

                    set_temp_ptr(p);
                    if mem_b16s1(p) == VLIST_NODE {
                        vlist_out();
                    } else {
                        hlist_out();
                    }

                    set_dvi_h(save_h);
                    set_dvi_v(save_v);

                    if upwards {
                        set_cur_v(save_v - mem_b32s1(p + 3));
                    } else {
                        set_cur_v(save_v + mem_b32s1(p + 2));
                    }

                    set_cur_h(left_edge);
                }
            }
            RULE_NODE => {
                set_rule_ht(mem_b32s1(p + 3));
                set_rule_dp(mem_b32s1(p + 2));
                set_rule_wd(mem_b32s1(p + 1));
                finish_vlist_rule_global(this_box, left_edge, upwards);
            }
            WHATSIT_NODE => match mem_b16s0(p) {
                GLYPH_NODE => {
                    set_cur_v(cur_v() + mem_b32s1(p + 3));
                    set_cur_h(left_edge);

                    if cur_h() != dvi_h() {
                        movement_global(cur_h() - dvi_h(), RIGHT1);
                        set_dvi_h(cur_h());
                    }

                    if cur_v() != dvi_v() {
                        movement_global(cur_v() - dvi_v(), DOWN1);
                        set_dvi_v(cur_v());
                    }

                    let f = mem_b16s2(p + 4) as i32;
                    ensure_dvi_font_global(f);

                    dvi_out_global(SET_GLYPHS);
                    dvi_four_global(0);
                    dvi_two_global(1);
                    dvi_four_global(0);
                    dvi_four_global(0);
                    dvi_two_global(mem_b16s1(p + 4));

                    set_cur_v(cur_v() + mem_b32s1(p + 2));
                    set_cur_h(left_edge);
                }
                PIC_NODE | PDF_NODE => {
                    let save_h = dvi_h();
                    let save_v = dvi_v();
                    set_cur_v(cur_v() + mem_b32s1(p + 3));
                    pic_out(p);
                    set_dvi_h(save_h);
                    set_dvi_v(save_v);
                    set_cur_v(save_v + mem_b32s1(p + 2));
                    set_cur_h(left_edge);
                }
                PDF_SAVE_POS_NODE => {
                    set_pdf_last_x_pos(cur_h() + ONE_INCH_LESS_ONE);
                    set_pdf_last_y_pos(cur_page_height() - cur_v() - ONE_INCH_LESS_ONE);
                }
                _ => out_what(p),
            },
            GLUE_NODE => {
                let g = mem_b32s0(p + 1);
                set_rule_ht(mem_b32s1(g + 1) - cur_g);

                if g_sign != NORMAL {
                    if g_sign == STRETCHING {
                        if mem_b16s1(g) == g_order {
                            cur_glue += mem_b32s1(g + 2) as f64;
                            let glue_temp = (mem_gr(this_box + 6) * cur_glue)
                                .clamp(-1_000_000_000.0, 1_000_000_000.0);
                            cur_g = crate::scaled_math::tex_round(glue_temp);
                        }
                    } else if mem_b16s0(g) == g_order {
                        cur_glue -= mem_b32s1(g + 3) as f64;
                        let glue_temp = (mem_gr(this_box + 6) * cur_glue)
                            .clamp(-1_000_000_000.0, 1_000_000_000.0);
                        cur_g = crate::scaled_math::tex_round(glue_temp);
                    }
                }

                set_rule_ht(rule_ht() + cur_g);

                if mem_b16s0(p) >= A_LEADERS {
                    let leader_box = mem_b32s1(p + 1);

                    if mem_b16s1(leader_box) == RULE_NODE {
                        set_rule_wd(mem_b32s1(leader_box + 1));
                        set_rule_dp(0);
                        finish_vlist_rule_global(this_box, left_edge, upwards);
                    } else {
                        let leader_ht = mem_b32s1(leader_box + 3) + mem_b32s1(leader_box + 2);

                        if leader_ht > 0 && rule_ht() > 0 {
                            set_rule_ht(rule_ht() + 10);
                            let edge = cur_v() + rule_ht();
                            let mut lx = 0;

                            if mem_b16s0(p) == A_LEADERS {
                                let save_v = cur_v();
                                set_cur_v(
                                    top_edge + leader_ht * ((cur_v() - top_edge) / leader_ht),
                                );
                                if cur_v() < save_v {
                                    set_cur_v(cur_v() + leader_ht);
                                }
                            } else {
                                let lq = rule_ht() / leader_ht;
                                let lr = rule_ht() % leader_ht;

                                if mem_b16s0(p) == C_LEADERS {
                                    set_cur_v(cur_v() + (lr / 2));
                                } else {
                                    lx = lr / (lq + 1);
                                    set_cur_v(cur_v() + ((lr - (lq - 1) * lx) / 2));
                                }
                            }

                            while cur_v() + leader_ht <= edge {
                                if cur_dir() == RIGHT_TO_LEFT {
                                    set_cur_h(left_edge - mem_b32s1(leader_box + 4));
                                } else {
                                    set_cur_h(left_edge + mem_b32s1(leader_box + 4));
                                }

                                if cur_h() != dvi_h() {
                                    movement_global(cur_h() - dvi_h(), RIGHT1);
                                    set_dvi_h(cur_h());
                                }

                                let save_h = dvi_h();
                                set_cur_v(cur_v() + mem_b32s1(leader_box + 3));

                                if cur_v() != dvi_v() {
                                    movement_global(cur_v() - dvi_v(), DOWN1);
                                    set_dvi_v(cur_v());
                                }

                                let save_v = dvi_v();
                                set_temp_ptr(leader_box);
                                let outer_doing_leaders = doing_leaders();
                                set_doing_leaders(true);

                                if mem_b16s1(leader_box) == VLIST_NODE {
                                    vlist_out();
                                } else {
                                    hlist_out();
                                }

                                set_doing_leaders(outer_doing_leaders);
                                set_dvi_v(save_v);
                                set_dvi_h(save_h);
                                set_cur_h(left_edge);
                                set_cur_v(save_v - mem_b32s1(leader_box + 3) + leader_ht + lx);
                            }

                            set_cur_v(edge - 10);
                            go_next = true;
                        } else {
                            move_past_vlist_rule_global(upwards);
                        }
                    }
                } else {
                    move_past_vlist_rule_global(upwards);
                }
            }
            KERN_NODE => {
                if upwards {
                    set_cur_v(cur_v() - mem_b32s1(p + 1));
                } else {
                    set_cur_v(cur_v() + mem_b32s1(p + 1));
                }
            }
            _ => {}
        }

        if go_next {
            p = mem_b32s1(p);
        }
    }

    crate::state::with_state_mut(|st| crate::synctex::synctex_tsilv(st, this_box));
    prune_movements_global(save_loc);

    if cur_s() > 0 {
        dvi_pop_global(save_loc);
    }
    set_cur_s(cur_s() - 1);
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

pub unsafe fn finish_vlist_rule_global(this_box: i32, left_edge: i32, upwards: bool) {
    let cur_dir_val = cur_dir();
    let action = crate::state::with_state_mut(|st| {
        finish_vlist_rule(
            &st.mem,
            &mut st.shipout,
            this_box,
            left_edge,
            upwards,
            cur_dir_val,
        )
    });
    if action.emit {
        if let Some(h_move) = action.h_move {
            movement_global(h_move, RIGHT1);
        }
        if let Some(v_move) = action.v_move {
            movement_global(v_move, DOWN1);
        }
        dvi_out_global(PUT_RULE);
        dvi_four_global(action.rule_ht);
        dvi_four_global(action.rule_wd);
    }
}

fn move_past_vlist_rule(shipout: &mut ShipoutState, upwards: bool) {
    if upwards {
        shipout.cur_v -= shipout.rule_ht;
    } else {
        shipout.cur_v += shipout.rule_ht;
    }
}

fn move_past_vlist_rule_global(upwards: bool) {
    crate::state::with_state_mut(|st| move_past_vlist_rule(&mut st.shipout, upwards));
}

// xetex-constants.h:238,240,248,249,226,227
const EOP: u8 = 140;
const POST: u8 = 248;
const POST_POST: u8 = 249;
const XDV_ID_BYTE: u8 = 7;
const SPX_ID_BYTE: u8 = 100;
// xetex-constants.h:117
// xetex_format.h:52; xetex-xetexd.h INT_BASE
const INT_PAR_MAG: i32 = 17;
const INT_PAR_TRACING_OUTPUT: i32 = 34;
const DIMEN_PAR_H_OFFSET: i32 = 18;
const DIMEN_PAR_V_OFFSET: i32 = 19;
const DIMEN_PAR_PDF_PAGE_WIDTH: i32 = 21;
const DIMEN_PAR_PDF_PAGE_HEIGHT: i32 = 22;
const COUNT_BASE: i32 = 7_826_812;
// scan.rs FONT_BASE
const FONT_BASE: i32 = 0;
const LEFT_TO_RIGHT: i16 = 0;
const RIGHT_TO_LEFT: i16 = 1;
const ONE_INCH: i32 = 4_736_287;
const ONE_INCH_LESS_ONE: i32 = 4_736_286;

// translated from xetex-shipout.c:1930 (prune_movements)
pub fn prune_movements(mem: &mut MemState, shipout: &mut ShipoutState, l: i32) {
    while let Some(p) = NodePtr::<MovementNode>::from_ptr(MemPtr::from_raw(shipout.down_ptr)) {
        if p.location(mem) < l {
            break;
        }
        shipout.down_ptr = p.link(mem).0;
        p.free(mem);
    }
    while let Some(p) = NodePtr::<MovementNode>::from_ptr(MemPtr::from_raw(shipout.right_ptr)) {
        if p.location(mem) < l {
            return;
        }
        shipout.right_ptr = p.link(mem).0;
        p.free(mem);
    }
}

pub fn prune_movements_global(l: i32) {
    crate::state::with_state_mut(|st| prune_movements(&mut st.mem, &mut st.shipout, l));
}

// print_c_string: local equivalent of xetex-xetexd.h static inline print_c_string.
unsafe fn print_c_string_here(s: *const libc::c_char) {
    let mut p = s;
    while *p != 0 {
        crate::output::print_char(*p as i32);
        p = p.add(1);
    }
}

// translated from xetex-shipout.c:2173 (finalize_dvi_file)
pub unsafe fn finalize_dvi_file() {
    while cur_s() > -1 {
        if cur_s() > 0 {
            dvi_out_global(POP);
        } else {
            dvi_out_global(EOP);
            set_total_pages(total_pages() + 1);
        }
        set_cur_s(cur_s() - 1);
    }

    if total_pages() == 0 {
        crate::output::print_nl_cstr(c"No pages of output.".as_ptr());
        return;
    }

    if cur_s() == -2 {
        // DVI got too big; message already printed
        return;
    }

    dvi_out_global(POST);
    dvi_four_global(last_bop());
    set_last_bop(dvi_offset() + dvi_ptr() - 5);
    dvi_four_global(25_400_000); // magic: conversion ratio for sp
    dvi_four_global(473_628_672); // magic: conversion ratio for sp
    crate::save::prepare_mag();
    dvi_four_global(eqtb_b32s1(INT_BASE + INT_PAR_MAG));
    dvi_four_global(max_v());
    dvi_four_global(max_h());
    dvi_out_global((max_push() / 256) as u8);
    dvi_out_global((max_push() % 256) as u8);
    dvi_out_global(((total_pages() / 256) % 256) as u8);
    dvi_out_global((total_pages() % 256) as u8);

    let mut fp = font_ptr();
    while fp > FONT_BASE {
        if font_used_entry(fp) {
            dvi_font_def_global(fp);
        }
        fp -= 1;
    }
    set_font_ptr(fp);

    dvi_out_global(POST_POST);
    dvi_four_global(last_bop());

    if semantic_pagination_enabled() {
        dvi_out_global(SPX_ID_BYTE);
    } else {
        dvi_out_global(XDV_ID_BYTE);
    }

    let mut k = (4 + (DVI_BUF_SIZE - dvi_ptr()) % 4) as u8;
    while k > 0 {
        dvi_out_global(223);
        k -= 1;
    }

    if dvi_limit() == HALF_BUF {
        write_to_dvi_global(HALF_BUF, DVI_BUF_SIZE - 1);
    }

    if dvi_ptr() > TEX_INFINITY - dvi_offset() {
        set_cur_s(-2);
        crate::errors::fatal_error(c"dvi length exceeds 0x7FFFFFFF".as_ptr());
    }

    if dvi_ptr() > 0 {
        write_to_dvi_global(0, dvi_ptr() - 1);
    }

    let k = ttstub_output_close(dvi_file()) as u8;

    if k == 0 {
        crate::output::print_nl_cstr(c"Output written on ".as_ptr());
        crate::output::print(output_file_name());
        crate::output::print_cstr(c" (".as_ptr());
        crate::output::print_int(total_pages());
        if total_pages() != 1 {
            crate::output::print_cstr(c" pages".as_ptr());
        } else {
            crate::output::print_cstr(c" page".as_ptr());
        }
        crate::output::print_cstr(c", ".as_ptr());
        crate::output::print_int(dvi_offset() + dvi_ptr());
        crate::output::print_cstr(c" bytes).".as_ptr());
    } else {
        crate::output::print_nl_cstr(c"Error ".as_ptr());
        crate::output::print_int(k as i32);
        crate::output::print_cstr(c" (".as_ptr());
        print_c_string_here(libc::strerror(k as libc::c_int));
        crate::output::print_cstr(c") generating output;".as_ptr());
        crate::output::print_nl_cstr(c"file ".as_ptr());
        crate::output::print(output_file_name());
        crate::output::print_cstr(c" may not be valid.".as_ptr());
        // XeTeX adds history = OUTPUT_FAILURE = 4 here; not implemented.
    }
}

#[inline]
fn is_char_node_global(p: i32) -> bool {
    crate::state::with_state(|st| crate::mem::is_char_node(&st.mem, p))
}

#[inline]
fn math_node_lr_dir(mem: &MemState, p: i32) -> u16 {
    mem.b16s0(p) / R_CODE
}

#[inline]
fn math_node_lr_dir_global(p: i32) -> u16 {
    crate::state::with_state(|st| math_node_lr_dir(&st.mem, p))
}

#[inline]
fn math_node_end_lr_type(mem: &MemState, p: i32) -> u16 {
    L_CODE * (mem.b16s0(p) / L_CODE) + END_M_CODE
}

#[inline]
fn math_node_end_lr_type_global(p: i32) -> u16 {
    crate::state::with_state(|st| math_node_end_lr_type(&st.mem, p))
}

#[inline]
fn font_character_width(fonts: &FontState, f: i32, c: i32) -> i32 {
    let info = fonts.font_info_b16(fonts.char_base_entry(f) + c);
    fonts.font_info_b32s1(fonts.width_base_entry(f) + info.s3 as i32)
}

#[inline]
fn font_character_width_global(f: i32, c: i32) -> i32 {
    crate::state::with_state(|st| font_character_width(&st.fonts, f, c))
}

// translated from xetex-shipout.c:1470 (reverse)
// Called by the (now Rust) hlist_out; internal Rust fn.
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
    let g_order = mem.b16s0(this_box + 5);
    let g_sign = mem.b16s1(this_box + 5);
    let mut l = t;
    let mut p = temp_ptr();
    let mut m = MIN_HALFWORD;
    let mut n = MIN_HALFWORD;

    loop {
        while p != TEX_NULL {
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

            'reswitch: loop {
                match mem.b16s1(p) {
                    HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                        shipout.rule_wd = mem.b32s1(p + 1);
                    }
                    WHATSIT_NODE => match mem.b16s0(p) {
                        NATIVE_WORD_NODE | NATIVE_WORD_NODE_AT | GLYPH_NODE | PIC_NODE
                        | PDF_NODE => {
                            shipout.rule_wd = mem.b32s1(p + 1);
                        }
                        _ => add_width = false,
                    },
                    GLUE_NODE => {
                        let mut g = mem.b32s0(p + 1);
                        shipout.rule_wd = mem.b32s1(g + 1) - *cur_g;

                        if g_sign != NORMAL {
                            if g_sign == STRETCHING {
                                if mem.b16s1(g) == g_order {
                                    *cur_glue += mem.b32s1(g + 2) as f64;
                                    let mut glue_temp = mem.gr(this_box + 6) * *cur_glue;
                                    glue_temp = glue_temp.clamp(-1_000_000_000.0, 1_000_000_000.0);
                                    *cur_g = crate::scaled_math::tex_round(glue_temp);
                                }
                            } else if mem.b16s0(g) == g_order {
                                *cur_glue -= mem.b32s1(g + 3) as f64;
                                let mut glue_temp = mem.gr(this_box + 6) * *cur_glue;
                                glue_temp = glue_temp.clamp(-1_000_000_000.0, 1_000_000_000.0);
                                *cur_g = crate::scaled_math::tex_round(glue_temp);
                            }
                        }

                        shipout.rule_wd += *cur_g;

                        if (g_sign == STRETCHING && mem.b16s1(g) == g_order)
                            || (g_sign == SHRINKING && mem.b16s0(g) == g_order)
                        {
                            if mem.b32s1(g) == TEX_NULL {
                                crate::mem::free_node(mem, g, GLUE_SPEC_SIZE);
                            } else {
                                mem.set_b32s1(g, mem.b32s1(g) - 1);
                            }

                            if mem.b16s0(p) < A_LEADERS {
                                mem.set_b16s1(p, KERN_NODE);
                                mem.set_b32s1(p + 1, shipout.rule_wd);
                            } else {
                                g = crate::mem::get_node(mem, GLUE_SPEC_SIZE);
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
                        crate::mem::flush_list(mem, mem.b32s1(p + 1));
                        set_temp_ptr(p);
                        p = crate::mem::get_avail(mem);
                        mem.set_word(p, mem.word(temp_ptr() + 1));
                        mem.set_b32s1(p, q);
                        crate::mem::free_node(mem, temp_ptr(), SMALL_NODE_SIZE);
                        continue 'reswitch;
                    }
                    MATH_NODE => {
                        shipout.rule_wd = mem.b32s1(p + 1);

                        if !mem.b16s0(p).is_multiple_of(2) {
                            if mem.b32s0(scanner.LR_ptr) as u16 != math_node_end_lr_type(mem, p) {
                                mem.set_b16s1(p, KERN_NODE);
                                scanner.LR_problems += 1;
                            } else {
                                set_temp_ptr(scanner.LR_ptr);
                                scanner.LR_ptr = mem.b32s1(temp_ptr());
                                mem.set_b32s1(temp_ptr(), mem.avail);
                                mem.avail = temp_ptr();

                                if n > MIN_HALFWORD {
                                    n -= 1;
                                    mem.set_b16s0(p, mem.b16s0(p) - 1);
                                } else {
                                    mem.set_b16s1(p, KERN_NODE);

                                    if m > MIN_HALFWORD {
                                        m -= 1;
                                    } else {
                                        crate::mem::free_node(mem, p, MEDIUM_NODE_SIZE);
                                        mem.set_b32s1(t, q);
                                        mem.set_b32s1(t + 1, shipout.rule_wd);
                                        mem.set_b32s1(t + 2, -shipout.cur_h - shipout.rule_wd);
                                        return Ok(l);
                                    }
                                }
                            }
                        } else {
                            set_temp_ptr(crate::mem::get_avail(mem));
                            mem.set_b32s0(temp_ptr(), math_node_end_lr_type(mem, p) as i32);
                            mem.set_b32s1(temp_ptr(), scanner.LR_ptr);
                            scanner.LR_ptr = temp_ptr();

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
                crate::mem::free_node(mem, p, MEDIUM_NODE_SIZE);
                p = l;
            }

            l = p;
            p = q;
        }

        if t == TEX_NULL && m == MIN_HALFWORD && n == MIN_HALFWORD {
            return Ok(l);
        }

        p = crate::mem::new_math(mem, 0, mem.b32s0(scanner.LR_ptr) as i16);
        scanner.LR_problems += 10000;
    }
}

// Bridge-down wrapper: borrows the anchor once and fires the cold abort after
// the borrow drops (§5). Retires when both callers (`hlist_out`/`vlist_out`)
// are threaded.
pub unsafe fn reverse_global(this_box: i32, t: i32, cur_g: *mut i32, cur_glue: *mut f64) -> i32 {
    let outcome = crate::state::with_state_mut(|st| {
        reverse(
            &mut st.mem,
            &mut st.shipout,
            &mut st.scanner,
            &mut st.fonts,
            this_box,
            t,
            &mut *cur_g,
            &mut *cur_glue,
        )
    });
    match outcome {
        Ok(l) => l,
        Err(a) => fire_shipout_abort(a),
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
        q.set_link(mem, MemPtr::from_raw(shipout.down_ptr));
        shipout.down_ptr = q.idx().0;
    } else {
        q.set_link(mem, MemPtr::from_raw(shipout.right_ptr));
        shipout.right_ptr = q.idx().0;
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
                while q.link(mem).0 != p.idx().0 {
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
                while q.link(mem).0 != p.idx().0 {
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

pub unsafe fn movement_global(w: i32, o: u8) {
    let bytes = crate::state::with_state_mut(|st| movement(&mut st.mem, &mut st.shipout, w, o));
    if bytes.is_empty() {
        crate::errors::overflow(c"main memory size".as_ptr(), MEM_TOP + 1);
    }
    for byte in bytes {
        dvi_out_global(byte);
    }
}

// translated from xetex-shipout.c:1930 (special_out)
// Only called from out_what (Rust) → private.
fn special_out_in(st: &mut crate::state::EngineState, p: i32) -> Result<(), ShipoutAbort> {
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
    let tok = st.mem.b32s1(st.mem.b32s1(p + 1));
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

// Bridge-down wrapper; retires when `out_what` is threaded.
unsafe fn special_out(p: i32) {
    let outcome = crate::state::with_state_mut(|st| special_out_in(st, p));
    if let Err(a) = outcome {
        fire_shipout_abort(a);
    }
}

// translated from xetex-shipout.c:1976 (write_out)
// Only called from out_what (Rust) → private.
unsafe fn write_out(p: i32) {
    let q = crate::mem::get_avail_global();
    set_mem_b32s0(q, RIGHT_BRACE_TOKEN + b'}' as i32);
    let r = crate::mem::get_avail_global();
    set_mem_b32s1(q, r);
    set_mem_b32s0(r, CS_TOKEN_FLAG + END_WRITE);
    crate::input::begin_token_list(q, INSERTED);
    crate::input::begin_token_list(mem_b32s1(p + 1), WRITE_TEXT);
    let q2 = crate::mem::get_avail_global();
    set_mem_b32s0(q2, LEFT_BRACE_TOKEN + b'{' as i32);
    crate::input::begin_token_list(q2, INSERTED);

    let old_mode = cur_list_mode();
    set_cur_list_mode(0);
    set_cur_cs(write_loc());
    crate::scan::scan_toks(false, true);
    crate::scan::get_token();

    if cur_tok() != CS_TOKEN_FLAG + END_WRITE {
        crate::output::error_here_with_diagnostic(c"Unbalanced write command".as_ptr());
        crate::output::capture_to_diagnostic(core::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"On this page there's a \\write with fewer real {'s than }'s.".as_ptr(),
        );
        set_help_line(0, c"I can't handle that very well; good luck.".as_ptr());
        crate::errors::error();
        loop {
            crate::scan::get_token();
            if cur_tok() == CS_TOKEN_FLAG + END_WRITE {
                break;
            }
        }
    }

    set_cur_list_mode(old_mode);
    crate::input::end_token_list();
    let old_setting = selector();
    let j = mem_b32s0(p + 1);

    if j == 18 {
        set_selector(SELECTOR_NEW_STRING);
    } else if write_open(j as usize) {
        set_selector(j);
    } else {
        if j == 17 && selector() == SELECTOR_TERM_AND_LOG {
            set_selector(SELECTOR_LOG_ONLY);
        }
        crate::output::print_nl_cstr(c"".as_ptr());
    }

    crate::output::token_show(def_ref());
    crate::output::print_ln();
    crate::mem::flush_list_global(def_ref());

    if j == 18 {
        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) <= 0 {
            set_selector(SELECTOR_LOG_ONLY);
        } else {
            set_selector(SELECTOR_TERM_AND_LOG);
        }
        if !log_opened() {
            set_selector(SELECTOR_TERM_ONLY);
        }

        // Tectonic: don't emit warnings for shell-escape when enabled;
        // the driver provides a better UX.
        if !shell_escape_enabled() {
            crate::output::diagnostic_begin_capture_warning_here();
            crate::output::print_nl_cstr(c"runsystem(".as_ptr());
            let base = str_start(str_ptr() - TOO_BIG_CHAR);
            let cur_len = pool_ptr() - base;
            for d in 0..cur_len {
                crate::output::print(str_pool_entry(base + d) as i32);
            }
            crate::output::print_cstr(c")...".as_ptr());
            crate::output::print_cstr(c"disabled".as_ptr());
            crate::output::print_char(b'.' as i32);
            crate::output::capture_to_diagnostic(core::ptr::null_mut());
            crate::output::print_nl_cstr(c"".as_ptr());
            crate::output::print_ln();
        } else {
            let base = str_start(str_ptr() - TOO_BIG_CHAR);
            let cur_len = pool_ptr() - base;
            ttstub_shell_escape(str_pool_ptr(base), cur_len as libc::size_t);
        }

        set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
    }

    set_selector(old_setting);
}

// translated from xetex-shipout.c:2071 (pic_out)
// Called from the (now Rust) hlist_out; internal Rust fn.
pub(crate) fn pic_out_in(st: &mut crate::state::EngineState, p: i32) -> Result<(), ShipoutAbort> {
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

    match st.mem.b16s1(p + 8) {
        1 => crate::output::print_cstr_in(st, c"pagebox cropbox ".as_ptr()),
        2 => crate::output::print_cstr_in(st, c"pagebox mediabox ".as_ptr()),
        3 => crate::output::print_cstr_in(st, c"pagebox bleedbox ".as_ptr()),
        4 => crate::output::print_cstr_in(st, c"pagebox trimbox ".as_ptr()),
        5 => crate::output::print_cstr_in(st, c"pagebox artbox ".as_ptr()),
        _ => {}
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

// Bridge-down wrapper; retires when `hlist_out` is threaded.
pub unsafe fn pic_out(p: i32) {
    let outcome = crate::state::with_state_mut(|st| pic_out_in(st, p));
    if let Err(a) = outcome {
        fire_shipout_abort(a);
    }
}

// translated from xetex-shipout.c:1682 (out_what)
// Only a dead prototype remains in xetex-xetexd.h; internal Rust fn.
pub unsafe fn out_what(p: i32) {
    match mem_b16s0(p) {
        OPEN_NODE | WRITE_NODE | CLOSE_NODE => {
            if !doing_leaders() {
                let j = mem_b32s0(p + 1);
                if mem_b16s0(p) == WRITE_NODE {
                    write_out(p);
                    return;
                }
                if write_open(j as usize) {
                    ttstub_output_close(write_file(j as usize));
                }
                if mem_b16s0(p) == CLOSE_NODE {
                    set_write_open(j as usize, false);
                    return;
                }
                // OPEN_NODE
                if j < 16 {
                    set_cur_name(mem_b32s1(p + 1));
                    set_cur_area(mem_b32s0(p + 2));
                    set_cur_ext(mem_b32s1(p + 2));
                    if crate::string_pool::length_global(cur_ext()) == 0 {
                        set_cur_ext(crate::texmfmp::maketexstring(c".tex".as_ptr()));
                    }
                    crate::io::pack_file_name(cur_name(), cur_area(), cur_ext());
                    let handle = ttstub_output_open(name_of_file(), 0);
                    if handle == INVALID_HANDLE {
                        let mut buf = [0u8; 512];
                        libc::snprintf(
                            buf.as_mut_ptr() as *mut libc::c_char,
                            buf.len(),
                            c"cannot open output file \"%s\"".as_ptr(),
                            name_of_file(),
                        );
                        _tt_abort(buf.as_ptr() as *const libc::c_char);
                    }
                    set_write_file(j as usize, handle);
                    set_write_open(j as usize, true);
                    if log_opened() {
                        let old_setting = selector();
                        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) <= 0 {
                            set_selector(SELECTOR_LOG_ONLY);
                        } else {
                            set_selector(SELECTOR_TERM_AND_LOG);
                        }
                        crate::output::print_nl_cstr(c"\\openout".as_ptr());
                        crate::output::print_int(j);
                        crate::output::print_cstr(c" = `".as_ptr());
                        crate::output::print_file_name(cur_name(), cur_area(), cur_ext());
                        crate::output::print_cstr(c"'.".as_ptr());
                        crate::output::print_nl_cstr(c"".as_ptr());
                        crate::output::print_ln();
                        set_selector(old_setting);
                    }
                }
            }
        }
        SPECIAL_NODE => special_out(p),
        LANGUAGE_NODE => {}
        _ => crate::errors::confusion(c"ext4".as_ptr()),
    }
}
