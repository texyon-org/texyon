//! Phase 3 — printing helpers translated from `xetex/xetex-output.c`.
use crate::consts::{
    ACTIVE_BASE, BIGGEST_CHAR, BOX_BASE, CAT_CODE_BASE, CS_TOKEN_FLAG, DISC_NODE, EQTB_SIZE,
    ERROR_STOP_MODE, GLUE_NODE, HLIST_NODE, INT_BASE, KERN_NODE, LIGATURE_NODE, LOCAL_BASE,
    MATH_NODE, NULL_FLAG, PENALTY_NODE, PRIM_EQTB_BASE, RULE_NODE, SELECTOR_TERM_AND_LOG,
    SELECTOR_TERM_ONLY, SINGLE_BASE, TEX_NULL, TOO_BIG_CHAR,
};
use crate::errors::error;
use crate::ext::print_utf8_str;
use crate::scan::{get_token, scan_register_num, scan_toks, the_toks};
use crate::state::EngineState;
use crate::texmfmp::gettexstring;

use crate::globals::{
    base_ptr, cond_ptr, cur_boundary, cur_chr, cur_cmd, cur_cs, cur_group, cur_if, cur_input,
    cur_level, cur_list_mode, cur_val, current_diagnostic, def_ref, eqtb_b32s1,
    file_line_error_style_p, file_offset, full_source_filename_stack, grp_stack, history, if_limit,
    if_line, if_stack, in_open, init_pool_ptr, input_ptr, input_stack, line, line_stack,
    max_print_line, mem_b16s0, mem_b16s1, mem_b32s1, pool_ptr, pool_size, rust_stdout, save_ptr,
    save_stack_b16s0, save_stack_b32s1, selector, set_base_ptr, set_cond_ptr, set_cur_group,
    set_cur_if, set_cur_level, set_current_diagnostic, set_grp_stack, set_help_line, set_help_ptr,
    set_history, set_if_limit, set_if_line, set_if_stack, set_input_stack, set_mem_b16s1,
    set_mem_b32s1, set_pool_ptr, set_save_ptr, set_selector, set_shown_mode, set_str_ptr,
    shown_mode, str_ptr, str_start, term_offset,
};

// Consts used by the show_* group, moved with it from scan.rs (workstream 02).
const TEMP_HEAD: i32 = 4_999_999 - 3;
const GARBAGE: i32 = 4_999_999 - 12;
const OTGR_FONT_FLAG: i32 = 0xFFFE;
const AAT_FONT_FLAG: i32 = 0xFFFF;
const LOCAL__ERR_HELP: i32 = 9; // xetex_format.h:181
const INT_PAR_SHOW_BOX_DEPTH: i32 = 25;
const INT_PAR_SHOW_BOX_BREADTH: i32 = 24;
const INT_PAR_END_LINE_CHAR: i32 = 48;

// Selector constants (xetex-core.h:42–51).
const SELECTOR_NO_PRINT: i32 = 16;
const SELECTOR_LOG_ONLY: i32 = 18;
const SELECTOR_PSEUDO: i32 = 20;
const SELECTOR_NEW_STRING: i32 = 21;

// Constants for INTPAR(escape_char) = eqtb[INT_BASE + INT_PAR__escape_char].b32.s1
// (xetex_format.h:219,80 and xetex-constants.h:83).
const INT_PAR_ESCAPE_CHAR: i32 = 45;
const INT_PAR_NEW_LINE_CHAR: i32 = 49;
const INT_PAR_TRACING_ONLINE: i32 = 29;

// History values (xetex-xetexd.h:355–356).
const HISTORY_SPOTLESS: i32 = 0;
const HISTORY_WARNING_ISSUED: i32 = 1;

// Mode divisor for print_mode / print_in_mode (xetex_format.h:345).
const MAX_COMMAND_PLUS_1: i32 = 103; // MAX_COMMAND + 1
const BIGGEST_USV: i32 = 0x10_FFFF;
#[allow(dead_code)]
const TEXT_SIZE: i32 = 0;
const SCRIPT_SIZE: i32 = 256;
// print_cs / sprint_cs constants (xetex_format.h).
const NULL_CS: i32 = 2_228_225;
const HASH_BASE: i32 = 2_228_226;
const FROZEN_NULL_FONT: i32 = 2_245_338;
const UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
const LETTER: i32 = 11;
// print_sa_num (xetex-constants.h:326).
const DIMEN_VAL_LIMIT: i32 = 128;
// FONT_ID_BASE = FROZEN_NULL_FONT (xetex-constants.h:68, xetex_format.h:203).
const FONT_ID_BASE: i32 = FROZEN_NULL_FONT;
// Token-state constants (xetex-constants.h:261).
const TOKEN_LIST: u16 = 0;
// Conditional command codes (xetex_format.h:349,355,537).
const IF_TEST: u16 = 107;
const TOP_BOT_MARK: u16 = 112; // xetex_format.h:354
const CALL: u16 = 113;
const FI_CODE: u8 = 2;

// INT_PAR__tracing_nesting (xetex_format.h:95).
const INT_PAR_TRACING_NESTING: i32 = 60;
// FI_OR_ELSE command code (xetex_format.h:350).
const FI_OR_ELSE: u8 = 108;
// INT_PAR__tracing_ifs (xetex_format.h:93).
const INT_PAR_TRACING_IFS: i32 = 58;

// C functions not yet translated — still live in xetex-output.c or bridge.
extern "C" {
    // tectonic_bridge_core output helpers.
    fn ttstub_output_putc(handle: usize, c: i32) -> i32;
    fn ttbc_diag_append(diag: *mut std::ffi::c_void, text: *const libc::c_char);
    fn ttstub_diag_finish(diag: *mut std::ffi::c_void);
    fn ttbc_diag_begin_warning() -> *mut std::ffi::c_void;
    fn ttbc_diag_begin_error() -> *mut std::ffi::c_void;
    fn ttstub_diag_printf(diag: *mut std::ffi::c_void, fmt: *const libc::c_char, ...);
    fn ttstub_issue_warning(fmt: *const libc::c_char, ...) -> libc::c_void;
    fn ttstub_output_flush(handle: usize) -> libc::c_int;

    // Still in C — complex context display.

}

// ---------------------------------------------------------------------------
// print_the_digs (static helper — not exported; all C callers translated here)
// ---------------------------------------------------------------------------

/// Print the digits in `dig[0..k]` in reverse order.
///
/// Corresponds to the static `print_the_digs` in xetex-output.c:350.
/// Not exported — only called from the Rust functions below.
pub(crate) fn print_the_digs_in(st: &mut EngineState, mut k: u8) {
    // translated from xetex-output.c:350 (print_the_digs)
    while k > 0 {
        k -= 1;
        let d = st.errors.dig[k as usize];
        if d < 10 {
            print_char_in(st, b'0' as i32 + d as i32);
        } else {
            print_char_in(st, 55 /* b'A' - 10 */ + d as i32);
        }
    }
}

// ---------------------------------------------------------------------------
// print_int
// ---------------------------------------------------------------------------

/// Print an integer in decimal.
///
/// translated from xetex-output.c:363 (print_int)
pub unsafe fn print_int(n: i32) {
    crate::state::with_state_mut(|st| print_int_in(st, n));
}

/// `&mut EngineState` core of [`print_int`].
pub(crate) fn print_int_in(st: &mut EngineState, mut n: i32) {
    let mut k: u8 = 0;

    if n < 0 {
        print_char_in(st, b'-' as i32);
        if n > -100_000_000 {
            n = -(n);
        } else {
            let m_val = -1 - n;
            n = m_val / 10;
            let m = (m_val % 10) + 1;
            k = 1;
            if m < 10 {
                st.errors.dig[0] = m as u8;
            } else {
                st.errors.dig[0] = 0;
                n += 1;
            }
        }
    }

    loop {
        st.errors.dig[k as usize] = (n % 10) as u8;
        n /= 10;
        k += 1;
        if n == 0 {
            break;
        }
    }

    print_the_digs_in(st, k);
}

// ---------------------------------------------------------------------------
// print_hex
// ---------------------------------------------------------------------------

/// Print an integer in hexadecimal with a leading `"`.
///
/// translated from xetex-output.c:658 (print_hex)
pub unsafe fn print_hex(n: i32) {
    crate::state::with_state_mut(|st| print_hex_in(st, n));
}

/// `&mut EngineState` core of [`print_hex`].
pub(crate) fn print_hex_in(st: &mut EngineState, mut n: i32) {
    let mut k: u8 = 0;
    print_char_in(st, b'"' as i32);
    loop {
        st.errors.dig[k as usize] = (n % 16) as u8;
        n /= 16;
        k += 1;
        if n == 0 {
            break;
        }
    }
    print_the_digs_in(st, k);
}

// ---------------------------------------------------------------------------
// print_ucs_code
// ---------------------------------------------------------------------------

/// Print a Unicode scalar value in `U+XXXX` form.
///
/// translated from xetex-output.c:749 (print_ucs_code)
pub unsafe fn print_ucs_code(n: i32) {
    crate::state::with_state_mut(|st| print_ucs_code_in(st, n));
}

/// `&mut EngineState` core of [`print_ucs_code`].
pub(crate) fn print_ucs_code_in(st: &mut EngineState, mut n: i32) {
    let mut k: u8 = 0;
    print_cstr_in(st, c"U+".as_ptr());
    loop {
        st.errors.dig[k as usize] = (n % 16) as u8;
        n /= 16;
        k += 1;
        if n == 0 {
            break;
        }
    }
    while k < 4 {
        st.errors.dig[k as usize] = 0;
        k += 1;
    }
    print_the_digs_in(st, k);
}

// ---------------------------------------------------------------------------
// print_cstr
// ---------------------------------------------------------------------------

/// Print a C string one character at a time via `print_char`.
///
/// translated from xetex-output.c:301 (print_cstr)
pub unsafe fn print_cstr(str_ptr: *const libc::c_char) {
    crate::state::with_state_mut(|st| print_cstr_in(st, str_ptr));
}

/// `&mut EngineState` core of [`print_cstr`].
pub(crate) fn print_cstr_in(st: &mut EngineState, str_ptr: *const libc::c_char) {
    let s = unsafe { std::ffi::CStr::from_ptr(str_ptr) };
    for &b in s.to_bytes() {
        print_char_in(st, b as i32);
    }
}

// ---------------------------------------------------------------------------
// print_nl_cstr
// ---------------------------------------------------------------------------

/// Start a new line if needed, then print a C string.
///
/// translated from xetex-output.c:319 (print_nl_cstr)
pub unsafe fn print_nl_cstr(str_ptr: *const libc::c_char) {
    crate::state::with_state_mut(|st| print_nl_cstr_in(st, str_ptr));
}

/// `&mut EngineState` core of [`print_nl_cstr`].
pub(crate) fn print_nl_cstr_in(st: &mut EngineState, str_ptr: *const libc::c_char) {
    let sel = st.shipout.selector;
    if ((st.shipout.term_offset > 0) && (sel & 1 != 0))
        || ((st.shipout.file_offset > 0) && (sel >= SELECTOR_LOG_ONLY))
    {
        print_ln_in(st);
    }
    print_cstr_in(st, str_ptr);
}

// ---------------------------------------------------------------------------
// print_esc_cstr
// ---------------------------------------------------------------------------

/// Print the escape character (if any) followed by a C string.
///
/// translated from xetex-output.c:338 (print_esc_cstr)
pub unsafe fn print_esc_cstr(s: *const libc::c_char) {
    crate::state::with_state_mut(|st| print_esc_cstr_in(st, s));
}

/// `&mut EngineState` core of [`print_esc_cstr`].
pub(crate) fn print_esc_cstr_in(st: &mut EngineState, s: *const libc::c_char) {
    let c = st.eqtb.b32s1(INT_BASE + INT_PAR_ESCAPE_CHAR);
    if (0..=BIGGEST_USV).contains(&c) {
        print_char_in(st, c);
    }
    print_cstr_in(st, s);
}

// ---------------------------------------------------------------------------
// print_nl
// ---------------------------------------------------------------------------

/// Start a new line if needed, then print string `s`.
///
/// translated from xetex-output.c:302 (print_nl)
pub unsafe fn print_nl(s: i32) {
    crate::state::with_state_mut(|st| print_nl_in(st, s));
}

/// `&mut EngineState` core of [`print_nl`].
pub(crate) fn print_nl_in(st: &mut EngineState, s: i32) {
    let sel = st.shipout.selector;
    if ((st.shipout.term_offset > 0) && (sel & 1 != 0))
        || ((st.shipout.file_offset > 0) && (sel >= SELECTOR_LOG_ONLY))
    {
        print_ln_in(st);
    }
    print_in(st, s);
}

// ---------------------------------------------------------------------------
// print_esc
// ---------------------------------------------------------------------------

/// Print the escape character (if any) followed by string `s`.
///
/// translated from xetex-output.c:310 (print_esc)
pub unsafe fn print_esc(s: i32) {
    crate::state::with_state_mut(|st| print_esc_in(st, s));
}

/// `&mut EngineState` core of [`print_esc`].
pub(crate) fn print_esc_in(st: &mut EngineState, s: i32) {
    let c = st.eqtb.b32s1(INT_BASE + INT_PAR_ESCAPE_CHAR);
    if (0..=BIGGEST_USV).contains(&c) {
        print_char_in(st, c);
    }
    print_in(st, s);
}

// ---------------------------------------------------------------------------
// print_roman_int
// ---------------------------------------------------------------------------

/// Print `n` as a Roman numeral (lowercase).
///
/// translated from xetex-output.c:563 (print_roman_int)
pub unsafe fn print_roman_int(n: i32) {
    crate::state::with_state_mut(|st| print_roman_int_in(st, n));
}

/// `&mut EngineState` core of [`print_roman_int`].
pub(crate) fn print_roman_int_in(st: &mut EngineState, mut n: i32) {
    // "m2d5c2l5x2v5i": alternating letter/divisor pairs encoding the
    // standard Roman numeral values and their subtraction thresholds.
    const ROMAN_DATA: &[u8] = b"m2d5c2l5x2v5i";
    let mut j: usize = 0;
    let mut v: i32 = 1000;
    loop {
        while n >= v {
            print_char_in(st, ROMAN_DATA[j] as i32);
            n -= v;
        }
        if n <= 0 {
            return;
        }
        let mut k = j + 2;
        let mut u = v / (ROMAN_DATA[k - 1] as i32 - b'0' as i32);
        if ROMAN_DATA[k - 1] == b'2' {
            k += 2;
            u /= ROMAN_DATA[k - 1] as i32 - b'0' as i32;
        }
        if n + u >= v {
            print_char_in(st, ROMAN_DATA[k] as i32);
            n += u;
        } else {
            j += 2;
            v /= ROMAN_DATA[j - 1] as i32 - b'0' as i32;
        }
    }
}

// ---------------------------------------------------------------------------
// print_scaled
// ---------------------------------------------------------------------------

/// Print a scaled integer (fixed-point with 16 fractional bits) as a decimal.
///
/// translated from xetex-output.c:612 (print_scaled)
pub unsafe fn print_scaled(s: i32) {
    crate::state::with_state_mut(|st| print_scaled_in(st, s));
}

/// `&mut EngineState` core of [`print_scaled`].
pub(crate) fn print_scaled_in(st: &mut EngineState, mut s: i32) {
    if s < 0 {
        print_char_in(st, b'-' as i32);
        s = -s;
    }
    print_int_in(st, s / 0x10000);
    print_char_in(st, b'.' as i32);
    s = 10 * (s % 0x10000) + 5;
    let mut delta: i32 = 10;
    loop {
        if delta > 0x10000 {
            s = s + 0x8000 - 50000;
        }
        print_char_in(st, b'0' as i32 + (s / 0x10000));
        s = 10 * (s % 0x10000);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// warn_char
// ---------------------------------------------------------------------------

/// Append a single character to the active diagnostic (if any).
///
/// translated from xetex-output.c:75 (warn_char)
pub(crate) fn warn_char_in(st: &mut EngineState, c: i32) {
    let diag = st.shipout.current_diagnostic;
    if !diag.is_null() {
        let bytes: [u8; 2] = [c as u8, 0];
        unsafe { ttbc_diag_append(diag, bytes.as_ptr() as *const libc::c_char) };
    }
}

// ---------------------------------------------------------------------------
// print_ln
// ---------------------------------------------------------------------------

/// Emit a newline to the current output destination(s).
///
/// translated from xetex-output.c:84 (print_ln)
pub unsafe fn print_ln() {
    crate::state::with_state_mut(|st| print_ln_in(st));
}

/// `&mut EngineState` core of [`print_ln`].
pub(crate) fn print_ln_in(st: &mut EngineState) {
    match st.shipout.selector {
        SELECTOR_TERM_AND_LOG => {
            warn_char_in(st, b'\n' as i32);
            unsafe { ttstub_output_putc(st.shipout.rust_stdout, b'\n' as i32) };
            unsafe { ttstub_output_putc(st.shipout.log_file, b'\n' as i32) };
            st.shipout.term_offset = 0;
            st.shipout.file_offset = 0;
        }
        SELECTOR_LOG_ONLY => {
            warn_char_in(st, b'\n' as i32);
            unsafe { ttstub_output_putc(st.shipout.log_file, b'\n' as i32) };
            st.shipout.file_offset = 0;
        }
        SELECTOR_TERM_ONLY => {
            warn_char_in(st, b'\n' as i32);
            unsafe { ttstub_output_putc(st.shipout.rust_stdout, b'\n' as i32) };
            st.shipout.term_offset = 0;
        }
        SELECTOR_NO_PRINT | SELECTOR_PSEUDO | SELECTOR_NEW_STRING => {}
        sel => {
            unsafe { ttstub_output_putc(st.shipout.write_file[sel as usize], b'\n' as i32) };
        }
    }
}

// ---------------------------------------------------------------------------
// print_raw_char
// ---------------------------------------------------------------------------

/// Write one UTF-16 code unit to the current output destination(s).
///
/// translated from xetex-output.c:76 (print_raw_char)
pub unsafe fn print_raw_char(s: u16, incr_offset: bool) {
    crate::state::with_state_mut(|st| print_raw_char_in(st, s, incr_offset));
}

/// `&mut EngineState` core of [`print_raw_char`].
pub(crate) fn print_raw_char_in(st: &mut EngineState, s: u16, incr_offset: bool) {
    match st.shipout.selector {
        SELECTOR_TERM_AND_LOG => {
            warn_char_in(st, s as i32);
            unsafe { ttstub_output_putc(st.shipout.rust_stdout, s as i32) };
            unsafe { ttstub_output_putc(st.shipout.log_file, s as i32) };
            if incr_offset {
                st.shipout.term_offset += 1;
                st.shipout.file_offset += 1;
            }
            if st.shipout.term_offset == st.config.max_print_line {
                unsafe { ttstub_output_putc(st.shipout.rust_stdout, b'\n' as i32) };
                st.shipout.term_offset = 0;
            }
            if st.shipout.file_offset == st.config.max_print_line {
                unsafe { ttstub_output_putc(st.shipout.log_file, b'\n' as i32) };
                st.shipout.file_offset = 0;
            }
        }
        SELECTOR_LOG_ONLY => {
            warn_char_in(st, s as i32);
            unsafe { ttstub_output_putc(st.shipout.log_file, s as i32) };
            if incr_offset {
                st.shipout.file_offset += 1;
            }
            if st.shipout.file_offset == st.config.max_print_line {
                unsafe { ttstub_output_putc(st.shipout.log_file, b'\n' as i32) };
                st.shipout.file_offset = 0;
            }
        }
        SELECTOR_TERM_ONLY => {
            warn_char_in(st, s as i32);
            unsafe { ttstub_output_putc(st.shipout.rust_stdout, s as i32) };
            if incr_offset {
                st.shipout.term_offset += 1;
            }
            if st.shipout.term_offset == st.config.max_print_line {
                unsafe { ttstub_output_putc(st.shipout.rust_stdout, b'\n' as i32) };
                st.shipout.term_offset = 0;
            }
        }
        SELECTOR_NO_PRINT => {}
        SELECTOR_PSEUDO => {
            let t = st.errors.tally;
            if t < st.errors.trick_count {
                st.errors.trick_buf[(t % st.errors.error_line) as usize] = s;
            }
        }
        SELECTOR_NEW_STRING => {
            let pp = st.strings.pool_ptr;
            if pp < st.strings.pool_size {
                st.strings.str_pool[pp as usize] = s;
                st.strings.pool_ptr = pp + 1;
            }
        }
        sel => {
            unsafe { ttstub_output_putc(st.shipout.write_file[sel as usize], s as i32) };
        }
    }
    st.errors.tally += 1;
}

// ---------------------------------------------------------------------------
// print_char
// ---------------------------------------------------------------------------

/// Print a single Unicode code point `s` to the current output destination.
///
/// translated from xetex-output.c:76 (print_char)
pub unsafe fn print_char(s: i32) {
    crate::state::with_state_mut(|st| print_char_in(st, s));
}

/// `&mut EngineState` core of [`print_char`].
pub(crate) fn print_char_in(st: &mut EngineState, s: i32) {
    // Fast path for NEW_STRING: store as UTF-16 code unit(s).
    if st.shipout.selector > SELECTOR_PSEUDO && !st.shipout.doing_special {
        if s >= 0x10000 {
            print_raw_char_in(st, (0xD800 + (s - 0x10000) / 1024) as u16, true);
            print_raw_char_in(st, (0xDC00 + (s - 0x10000) % 1024) as u16, true);
        } else {
            print_raw_char_in(st, s as u16, true);
        }
        return;
    }

    // Newline character substitution.
    if s == st.eqtb.b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR) && st.shipout.selector < SELECTOR_PSEUDO
    {
        print_ln_in(st);
        return;
    }

    // Encode for terminal/log output.
    if s < 32 && !st.shipout.doing_special {
        // C0 controls: ^^X notation.
        print_raw_char_in(st, b'^' as u16, true);
        print_raw_char_in(st, b'^' as u16, true);
        print_raw_char_in(st, (s + 64) as u16, true);
    } else if s < 127 {
        print_raw_char_in(st, s as u16, true);
    } else if s == 127 {
        if !st.shipout.doing_special {
            print_raw_char_in(st, b'^' as u16, true);
            print_raw_char_in(st, b'^' as u16, true);
            print_raw_char_in(st, b'?' as u16, true);
        } else {
            print_raw_char_in(st, s as u16, true);
        }
    } else if s < 160 && !st.shipout.doing_special {
        // C1 controls and 128–159: ^^XX hex notation.
        print_raw_char_in(st, b'^' as u16, true);
        print_raw_char_in(st, b'^' as u16, true);
        let hi = (s % 256) / 16;
        if hi < 10 {
            print_raw_char_in(st, (b'0' as i32 + hi) as u16, true);
        } else {
            print_raw_char_in(st, (b'a' as i32 + hi - 10) as u16, true);
        }
        let lo = s % 16;
        if lo < 10 {
            print_raw_char_in(st, (b'0' as i32 + lo) as u16, true);
        } else {
            print_raw_char_in(st, (b'a' as i32 + lo - 10) as u16, true);
        }
    } else if st.shipout.selector == SELECTOR_PSEUDO {
        // In pseudo mode, store raw code unit regardless of value.
        print_raw_char_in(st, s as u16, true);
    } else {
        // Non-ASCII (s >= 160) heading to terminal/log: emit as UTF-8.
        if s < 2048 {
            print_raw_char_in(st, (192 + s / 64) as u16, false);
            print_raw_char_in(st, (128 + s % 64) as u16, true);
        } else if s < 0x10000 {
            print_raw_char_in(st, (224 + s / 4096) as u16, false);
            print_raw_char_in(st, (128 + (s % 4096) / 64) as u16, false);
            print_raw_char_in(st, (128 + s % 64) as u16, true);
        } else {
            print_raw_char_in(st, (240 + s / 0x40000) as u16, false);
            print_raw_char_in(st, (128 + (s % 0x40000) / 4096) as u16, false);
            print_raw_char_in(st, (128 + (s % 4096) / 64) as u16, false);
            print_raw_char_in(st, (128 + s % 64) as u16, true);
        }
    }
}

// ---------------------------------------------------------------------------
// print
// ---------------------------------------------------------------------------

/// Print string number `s` from the TeX string pool (or a single character).
///
/// translated from xetex-output.c:146 (print)
pub unsafe fn print(s: i32) {
    crate::state::with_state_mut(|st| print_in(st, s));
}

/// `&mut EngineState` core of [`print`].
pub(crate) fn print_in(st: &mut EngineState, s: i32) {
    if s >= st.strings.str_ptr {
        return print_cstr_in(st, c"???".as_ptr());
    }
    if s <= BIGGEST_CHAR {
        if s < 0 {
            return print_cstr_in(st, c"???".as_ptr());
        }
        // Single-character string number.
        if st.shipout.selector > SELECTOR_PSEUDO {
            print_char_in(st, s);
            return;
        }
        if s == st.eqtb.b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR)
            && st.shipout.selector < SELECTOR_PSEUDO
        {
            print_ln_in(st);
            return;
        }
        // Temporarily suppress newline-char substitution while printing s.
        let nl = st.eqtb.b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR);
        st.eqtb.set_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR, -1);
        print_char_in(st, s);
        st.eqtb.set_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR, nl);
        return;
    }

    // String pool reference: s >= 0x10000.
    let pool_idx = s - 0x10000;
    let mut i = st.strings.str_start[pool_idx as usize];
    let end = st.strings.str_start[(pool_idx + 1) as usize];
    while i < end {
        let c = st.strings.str_pool[i as usize];
        if (0xD800..0xDC00).contains(&c) && i + 1 < end {
            let c2 = st.strings.str_pool[(i + 1) as usize];
            if (0xDC00..0xE000).contains(&c2) {
                print_char_in(
                    st,
                    0x10000 + (c as i32 - 0xD800) * 1024 + c2 as i32 - 0xDC00,
                );
                i += 2;
                continue;
            }
        }
        print_char_in(st, c as i32);
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// print_current_string
// ---------------------------------------------------------------------------

/// Print the currently-accumulating string (in the string pool scratch area).
///
/// translated from xetex-output.c:328 (print_current_string)
pub(crate) fn print_current_string_in(st: &mut EngineState) {
    let mut j = st.strings.str_start[(st.strings.str_ptr - 0x10000) as usize];
    while j < st.strings.pool_ptr {
        let c = st.strings.str_pool[j as usize];
        print_char_in(st, c as i32);
        j += 1;
    }
}

// ---------------------------------------------------------------------------
// capture_to_diagnostic
// ---------------------------------------------------------------------------

/// Set the active diagnostic capture target, finishing the previous one.
///
/// translated from xetex-output.c:14 (capture_to_diagnostic)
pub unsafe fn capture_to_diagnostic(diagnostic: *mut std::ffi::c_void) {
    let cur = current_diagnostic();
    if !cur.is_null() {
        ttstub_diag_finish(cur);
    }
    set_current_diagnostic(diagnostic);
}

// ---------------------------------------------------------------------------
// diagnostic_print_file_line  (private Rust helper — C static, no export)
// ---------------------------------------------------------------------------

/// Append `"<file>:<line>: "` (or `"!"`) to `diagnostic`.
///
/// translated from xetex-output.c:24 (diagnostic_print_file_line — was static)
unsafe fn diagnostic_print_file_line(diagnostic: *mut std::ffi::c_void) {
    let mut level = in_open();
    while level > 0 && full_source_filename_stack(level) == 0 {
        level -= 1;
    }
    if level == 0 {
        unsafe { ttbc_diag_append(diagnostic, c"!".as_ptr()) };
    } else {
        let source_line = if level != in_open() {
            line_stack(level + 1)
        } else {
            line()
        };
        let filename = gettexstring(full_source_filename_stack(level));
        ttstub_diag_printf(diagnostic, c"%s:%d: ".as_ptr(), filename, source_line);
        libc::free(filename as *mut std::ffi::c_void);
    }
}

// ---------------------------------------------------------------------------
// diagnostic_begin_capture_warning_here
// ---------------------------------------------------------------------------

/// Begin a warning diagnostic capturing file/line context.
///
/// translated from xetex-output.c:48 (diagnostic_begin_capture_warning_here)
pub unsafe fn diagnostic_begin_capture_warning_here() -> *mut std::ffi::c_void {
    let warning = ttbc_diag_begin_warning();
    diagnostic_print_file_line(warning);
    capture_to_diagnostic(warning);
    warning
}

// ---------------------------------------------------------------------------
// error_here_with_diagnostic
// ---------------------------------------------------------------------------

/// Begin an error diagnostic with file/line context, and emit to TeX output.
///
/// translated from xetex-output.c:58 (error_here_with_diagnostic)
pub unsafe fn error_here_with_diagnostic(message: *const libc::c_char) -> *mut std::ffi::c_void {
    let error = ttbc_diag_begin_error();
    diagnostic_print_file_line(error);
    ttstub_diag_printf(error, c"%s".as_ptr(), message);

    if file_line_error_style_p() != 0 {
        print_file_line();
    } else {
        print_nl_cstr(c"! ".as_ptr());
    }
    print_cstr(message);

    capture_to_diagnostic(error);
    error
}

// ---------------------------------------------------------------------------
// print_file_line
// ---------------------------------------------------------------------------

/// Emit the current file/line prefix to the TeX output stream.
///
/// translated from xetex-output.c:304 (print_file_line)
pub unsafe fn print_file_line() {
    crate::state::with_state_mut(|st| print_file_line_in(st));
}

/// `&mut EngineState` core of [`print_file_line`].
pub(crate) fn print_file_line_in(st: &mut EngineState) {
    let mut level = st.input.in_open;
    while level > 0 && st.input.full_source_filename_stack[(level) as usize] == 0 {
        level -= 1;
    }
    if level == 0 {
        print_nl_cstr_in(st, c"! ".as_ptr());
    } else {
        print_nl_cstr_in(st, c"".as_ptr());
        print_in(st, st.input.full_source_filename_stack[(level) as usize]);
        print_char_in(st, b':' as i32);
        if level == st.input.in_open {
            print_int_in(st, line());
        } else {
            print_int_in(st, st.input.line_stack[(level + 1) as usize]);
        }
        print_cstr_in(st, c": ".as_ptr());
    }
}

// ---------------------------------------------------------------------------
// print_size
// ---------------------------------------------------------------------------

/// Print the name of a math font-size selector.
///
/// translated from xetex-output.c:233 (print_size)
pub unsafe fn print_size(s: i32) {
    crate::state::with_state_mut(|st| print_size_in(st, s));
}

/// `&mut EngineState` core of [`print_size`].
pub(crate) fn print_size_in(st: &mut EngineState, s: i32) {
    if s == TEXT_SIZE {
        print_esc_cstr_in(st, c"textfont".as_ptr());
    } else if s == SCRIPT_SIZE {
        print_esc_cstr_in(st, c"scriptfont".as_ptr());
    } else {
        print_esc_cstr_in(st, c"scriptscriptfont".as_ptr());
    }
}

// ---------------------------------------------------------------------------
// print_write_whatsit
// ---------------------------------------------------------------------------

/// Print a write-whatsit node's label and stream number.
///
/// translated from xetex-output.c:245 (print_write_whatsit)
pub(crate) fn print_write_whatsit_in(st: &mut EngineState, s: *const libc::c_char, p: i32) {
    print_esc_cstr_in(st, s);
    let v = st.mem.b32s0(p + 1);
    if v < 16 {
        print_int_in(st, v);
    } else if v == 16 {
        print_char_in(st, b'*' as i32);
    } else {
        print_char_in(st, b'-' as i32);
    }
}

// ---------------------------------------------------------------------------
// print_file_name
// ---------------------------------------------------------------------------

/// Print a filename composed of area `a`, name `n`, and extension `e`,
/// quoting if needed.
///
/// translated from xetex-output.c:128 (print_file_name)
pub unsafe fn print_file_name(n: i32, a: i32, e: i32) {
    crate::state::with_state_mut(|st| print_file_name_in(st, n, a, e));
}

/// `&mut EngineState` core of [`print_file_name`].
pub(crate) fn print_file_name_in(st: &mut EngineState, n: i32, a: i32, e: i32) {
    // Scan all three string components to decide if quoting is needed.
    let mut must_quote = false;
    let mut quote_char: i32 = 0;

    for s in [a, n, e] {
        if s != 0 {
            let mut j = st.strings.str_start[(s - 0x10000) as usize];
            let end = st.strings.str_start[((s + 1) - 0x10000) as usize];
            while (!must_quote || quote_char == 0) && j < end {
                let c = st.strings.str_pool[j as usize] as i32;
                if c == b' ' as i32 {
                    must_quote = true;
                } else if c == b'"' as i32 || c == b'\'' as i32 {
                    must_quote = true;
                    quote_char = 73 - c; // 73 = '"' + '\'' = 34 + 39
                }
                j += 1;
            }
        }
    }

    if must_quote {
        if quote_char == 0 {
            quote_char = b'"' as i32;
        }
        print_char_in(st, quote_char);
    }

    // Print each component with quote-char escaping.
    for s in [a, n, e] {
        if s != 0 {
            let mut j = st.strings.str_start[(s - 0x10000) as usize];
            let for_end = st.strings.str_start[((s + 1) - 0x10000) as usize] - 1;
            if j <= for_end {
                loop {
                    let c = st.strings.str_pool[j as usize] as i32;
                    if c == quote_char {
                        print_in(st, quote_char);
                        quote_char = 73 - quote_char;
                        print_in(st, quote_char);
                    }
                    print_in(st, c);
                    if j >= for_end {
                        break;
                    }
                    j += 1;
                }
            }
        }
    }

    if quote_char != 0 {
        print_char_in(st, quote_char);
    }
}

// ---------------------------------------------------------------------------
// print_cs
// ---------------------------------------------------------------------------

/// Print a control-sequence name for control-sequence pointer `p`.
///
/// translated from xetex-output.c:76 (print_cs)
pub unsafe fn print_cs(p: i32) {
    crate::state::with_state_mut(|st| print_cs_in(st, p));
}

/// `&mut EngineState` core of [`print_cs`].
pub(crate) fn print_cs_in(st: &mut EngineState, p: i32) {
    if p < HASH_BASE {
        if p >= SINGLE_BASE {
            if p == NULL_CS {
                print_esc_cstr_in(st, c"csname".as_ptr());
                print_esc_cstr_in(st, c"endcsname".as_ptr());
                print_char_in(st, b' ' as i32);
            } else {
                print_esc_in(st, p - SINGLE_BASE);
                if st.eqtb.b32s1(CAT_CODE_BASE + p - SINGLE_BASE) == LETTER {
                    print_char_in(st, b' ' as i32);
                }
            }
        } else if p < ACTIVE_BASE {
            print_esc_cstr_in(st, c"IMPOSSIBLE.".as_ptr());
        } else {
            print_char_in(st, p - 1);
        }
    } else if (UNDEFINED_CONTROL_SEQUENCE..=EQTB_SIZE).contains(&p) || p > st.eqtb.eqtb_top {
        print_esc_cstr_in(st, c"IMPOSSIBLE.".as_ptr());
    } else if st.eqtb.hash_s1(p) >= st.strings.str_ptr {
        print_esc_cstr_in(st, c"NONEXISTENT.".as_ptr());
    } else {
        if (PRIM_EQTB_BASE..FROZEN_NULL_FONT).contains(&p) {
            let s = st.eqtb.prim_s1(p - PRIM_EQTB_BASE) - 1;
            print_esc_in(st, s);
        } else {
            let s = st.eqtb.hash_s1(p);
            print_esc_in(st, s);
        }
        print_char_in(st, b' ' as i32);
    }
}

// ---------------------------------------------------------------------------
// sprint_cs
// ---------------------------------------------------------------------------

/// Print a control-sequence name without trailing space (used in error msgs).
///
/// translated from xetex-output.c:108 (sprint_cs)
pub unsafe fn sprint_cs(p: i32) {
    crate::state::with_state_mut(|st| sprint_cs_in(st, p));
}

/// `&mut EngineState` core of [`sprint_cs`].
pub(crate) fn sprint_cs_in(st: &mut EngineState, p: i32) {
    if p < HASH_BASE {
        if p < SINGLE_BASE {
            print_char_in(st, p - 1);
        } else if p < NULL_CS {
            print_esc_in(st, p - SINGLE_BASE);
        } else {
            print_esc_cstr_in(st, c"csname".as_ptr());
            print_esc_cstr_in(st, c"endcsname".as_ptr());
        }
    } else if (PRIM_EQTB_BASE..FROZEN_NULL_FONT).contains(&p) {
        let s = st.eqtb.prim_s1(p - PRIM_EQTB_BASE) - 1;
        print_esc_in(st, s);
    } else {
        let s = st.eqtb.hash_s1(p);
        print_esc_in(st, s);
    }
}

// ---------------------------------------------------------------------------
// print_native_word
// ---------------------------------------------------------------------------

/// Print the UTF-16 text of a native-word node.
///
/// translated from xetex-output.c:260 (print_native_word)
pub(crate) fn print_native_word_in(st: &mut EngineState, p: i32) {
    // NATIVE_NODE_text(p) = (unsigned short *) &mem[p + NATIVE_NODE_SIZE]
    // NATIVE_NODE_SIZE = 6 (xetex-constants.h:130)
    // mem[p+4].b16.s1 = character count
    let for_end = st.mem.b16s1(p + 4) as i32 - 1;
    let text_ptr = st.mem.data.as_ptr().wrapping_add((p + 6) as usize) as *const u16;
    let mut i: i32 = 0;
    while i <= for_end {
        let c = unsafe { *text_ptr.offset(i as isize) } as i32;
        if (0xD800..0xDC00).contains(&c) && i < for_end {
            let cc = unsafe { *text_ptr.offset(i as isize + 1) } as i32;
            if (0xDC00..0xE000).contains(&cc) {
                print_char_in(st, 0x10000 + (c - 0xD800) * 1024 + (cc - 0xDC00));
                i += 2;
                continue;
            }
        }
        if (0xD800..0xDC00).contains(&c) {
            print_in(st, b'.' as i32);
        } else {
            print_char_in(st, c);
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// print_sa_num
// ---------------------------------------------------------------------------

/// `&mut EngineState` core of [`print_sa_num`].
pub(crate) fn print_sa_num_in(st: &mut EngineState, q: i32) {
    let n = if (st.mem.b16s1(q) as i32) < DIMEN_VAL_LIMIT {
        st.mem.b32s1(q + 1)
    } else {
        let n0 = st.mem.b16s1(q) as i32 % 64;
        let q1 = st.mem.b32s1(q);
        let n1 = st.mem.b16s1(q1) as i32;
        let q2 = st.mem.b32s1(q1);
        let b16s1_q2 = st.mem.b16s1(q2) as i32;
        let q3 = st.mem.b32s1(q2);
        let b16s1_q3 = st.mem.b16s1(q3) as i32;
        n0 + 64 * n1 + 64 * 64 * (b16s1_q2 + 64 * b16s1_q3)
    };
    print_int_in(st, n);
}

// ---------------------------------------------------------------------------
// begin_diagnostic / end_diagnostic
// ---------------------------------------------------------------------------

/// Save selector and switch to log-only mode for diagnostic output.
///
/// translated from xetex-xetex0.c:1785 (begin_diagnostic)
pub fn begin_diagnostic() {
    crate::state::with_state_mut(|st| begin_diagnostic_in(st));
}

/// `&mut EngineState` core of [`begin_diagnostic`].
pub(crate) fn begin_diagnostic_in(st: &mut EngineState) {
    st.shipout.old_setting = st.shipout.selector as u8;
    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) <= 0
        && st.shipout.selector == SELECTOR_TERM_AND_LOG
    {
        st.shipout.selector = st.shipout.selector - 1;
        if st.errors.history == HISTORY_SPOTLESS {
            st.errors.history = HISTORY_WARNING_ISSUED;
        }
    }
}

/// Restore selector saved by begin_diagnostic.
///
/// translated from xetex-xetex0.c:1797 (end_diagnostic)
pub unsafe fn end_diagnostic(blank_line: bool) {
    crate::state::with_state_mut(|st| end_diagnostic_in(st, blank_line));
}

/// `&mut EngineState` core of [`end_diagnostic`].
pub(crate) fn end_diagnostic_in(st: &mut EngineState, blank_line: bool) {
    print_nl_cstr_in(st, c"".as_ptr());
    if blank_line {
        print_ln_in(st);
    }
    st.shipout.selector = st.shipout.old_setting as i32;
}

// ---------------------------------------------------------------------------
// print_mode / print_in_mode
// ---------------------------------------------------------------------------

/// Print a textual description of the current TeX mode m.
///
/// translated from xetex-xetex0.c:1327 (print_mode)
pub unsafe fn print_mode(m: i32) {
    crate::state::with_state_mut(|st| print_mode_in(st, m));
}

/// `&mut EngineState` core of [`print_mode`].
pub(crate) fn print_mode_in(st: &mut EngineState, m: i32) {
    if m > 0 {
        match m / MAX_COMMAND_PLUS_1 {
            0 => print_cstr_in(st, c"vertical mode".as_ptr()),
            1 => print_cstr_in(st, c"horizontal mode".as_ptr()),
            2 => print_cstr_in(st, c"display math mode".as_ptr()),
            _ => {}
        }
    } else if m == 0 {
        print_cstr_in(st, c"no mode".as_ptr());
    } else {
        match (-m) / MAX_COMMAND_PLUS_1 {
            0 => print_cstr_in(st, c"internal vertical mode".as_ptr()),
            1 => print_cstr_in(st, c"restricted horizontal mode".as_ptr()),
            2 => print_cstr_in(st, c"math mode".as_ptr()),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// print_group
// ---------------------------------------------------------------------------

// Group-code constants (xetex-constants.h:152–168; group_code = u8).
const BOTTOM_LEVEL: u8 = 0;
const SIMPLE_GROUP: u8 = 1;
const HBOX_GROUP: u8 = 2;
const ADJUSTED_HBOX_GROUP: u8 = 3;
const VBOX_GROUP: u8 = 4;
const VTOP_GROUP: u8 = 5;
const ALIGN_GROUP: u8 = 6;
const NO_ALIGN_GROUP: u8 = 7;
const OUTPUT_GROUP: u8 = 8;
const MATH_GROUP: u8 = 9;
const DISC_GROUP: u8 = 10;
const INSERT_GROUP: u8 = 11;
const VCENTER_GROUP: u8 = 12;
const MATH_CHOICE_GROUP: u8 = 13;
const SEMI_SIMPLE_GROUP: u8 = 14;
const MATH_SHIFT_GROUP: u8 = 15;
const MATH_LEFT_GROUP: u8 = 16;

/// Print a textual name for the current group, with level and entry line.
///
/// translated from xetex-xetex0.c:3464 (print_group)
pub unsafe fn print_group(e: bool) {
    crate::state::with_state_mut(|st| print_group_in(st, e));
}

/// `&mut EngineState` core of [`print_group`].
pub(crate) fn print_group_in(st: &mut EngineState, e: bool) {
    let g = st.save_stack.cur_group;
    match g {
        BOTTOM_LEVEL => {
            print_cstr_in(st, c"bottom level".as_ptr());
            return;
        }
        SIMPLE_GROUP | SEMI_SIMPLE_GROUP => {
            if g == SEMI_SIMPLE_GROUP {
                print_cstr_in(st, c"semi ".as_ptr());
            }
            print_cstr_in(st, c"simple".as_ptr());
        }
        HBOX_GROUP | ADJUSTED_HBOX_GROUP => {
            if g == ADJUSTED_HBOX_GROUP {
                print_cstr_in(st, c"adjusted ".as_ptr());
            }
            print_cstr_in(st, c"hbox".as_ptr());
        }
        VBOX_GROUP => print_cstr_in(st, c"vbox".as_ptr()),
        VTOP_GROUP => print_cstr_in(st, c"vtop".as_ptr()),
        ALIGN_GROUP | NO_ALIGN_GROUP => {
            if g == NO_ALIGN_GROUP {
                print_cstr_in(st, c"no ".as_ptr());
            }
            print_cstr_in(st, c"align".as_ptr());
        }
        OUTPUT_GROUP => print_cstr_in(st, c"output".as_ptr()),
        DISC_GROUP => print_cstr_in(st, c"disc".as_ptr()),
        INSERT_GROUP => print_cstr_in(st, c"insert".as_ptr()),
        VCENTER_GROUP => print_cstr_in(st, c"vcenter".as_ptr()),
        MATH_GROUP | MATH_CHOICE_GROUP | MATH_SHIFT_GROUP | MATH_LEFT_GROUP => {
            print_cstr_in(st, c"math".as_ptr());
            if g == MATH_CHOICE_GROUP {
                print_cstr_in(st, c" choice".as_ptr());
            } else if g == MATH_SHIFT_GROUP {
                print_cstr_in(st, c" shift".as_ptr());
            } else if g == MATH_LEFT_GROUP {
                print_cstr_in(st, c" left".as_ptr());
            }
        }
        _ => {}
    }
    print_cstr_in(st, c" group (level ".as_ptr());
    print_int_in(st, st.save_stack.cur_level as i32);
    print_char_in(st, ')' as i32);
    let line_at = st.save_stack.b32s1(st.save_stack.save_ptr - 1);
    if line_at != 0 {
        if e {
            print_cstr_in(st, c" entered at line ".as_ptr());
        } else {
            print_cstr_in(st, c" at line ".as_ptr());
        }
        print_int_in(st, line_at);
    }
}

/// Print "' in <mode>" for use in error messages.
///
/// translated from xetex-xetex0.c:1356 (print_in_mode)
pub unsafe fn print_in_mode(m: i32) {
    crate::state::with_state_mut(|st| print_in_mode_in(st, m));
}

/// `&mut EngineState` core of [`print_in_mode`].
pub(crate) fn print_in_mode_in(st: &mut EngineState, m: i32) {
    if m > 0 {
        match m / MAX_COMMAND_PLUS_1 {
            0 => print_cstr_in(st, c"' in vertical mode".as_ptr()),
            1 => print_cstr_in(st, c"' in horizontal mode".as_ptr()),
            2 => print_cstr_in(st, c"' in display math mode".as_ptr()),
            _ => {}
        }
    } else if m == 0 {
        print_cstr_in(st, c"' in no mode".as_ptr());
    } else {
        match (-m) / MAX_COMMAND_PLUS_1 {
            0 => print_cstr_in(st, c"' in internal vertical mode".as_ptr()),
            1 => print_cstr_in(st, c"' in restricted horizontal mode".as_ptr()),
            2 => print_cstr_in(st, c"' in math mode".as_ptr()),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// token_show
// ---------------------------------------------------------------------------

/// Display a token list headed at p.
///
/// translated from xetex-xetex0.c:3893 (token_show)
pub unsafe fn token_show(p: i32) {
    crate::state::with_state_mut(|st| token_show_in(st, p));
}

/// `&mut EngineState` core of [`token_show`].
pub(crate) fn token_show_in(st: &mut EngineState, p: i32) {
    if p != TEX_NULL {
        let head = st.mem.b32s1(p);
        show_token_list_in(st, head, TEX_NULL, 10_000_000);
    }
}

// ---------------------------------------------------------------------------
// print_spec
// ---------------------------------------------------------------------------

/// Print a glue spec node as "width plus stretch minus shrink".
///
/// translated from xetex-xetex0.c:274 (print_spec)
pub unsafe fn print_spec(p: i32, s: *const libc::c_char) {
    crate::state::with_state_mut(|st| print_spec_in(st, p, s));
}

/// `&mut EngineState` core of [`print_spec`].
pub(crate) fn print_spec_in(st: &mut EngineState, p: i32, s: *const libc::c_char) {
    if p < 0 || p >= st.mem.lo_mem_max {
        print_char_in(st, '*' as i32);
    } else {
        print_scaled_in(st, st.mem.b32s1(p + 1));
        if !s.is_null() {
            print_cstr_in(st, s);
        }
        if st.mem.b32s1(p + 2) != 0 {
            print_cstr_in(st, c" plus ".as_ptr());
            let d = st.mem.b32s1(p + 2);
            let o = st.mem.b16s1(p) as i32;
            print_glue_in(st, d, o, s);
        }
        if st.mem.b32s1(p + 3) != 0 {
            print_cstr_in(st, c" minus ".as_ptr());
            let d = st.mem.b32s1(p + 3);
            let o = st.mem.b16s0(p) as i32;
            print_glue_in(st, d, o, s);
        }
    }
}

// ---------------------------------------------------------------------------
// print_skip_param / print_param / print_length_param
// ---------------------------------------------------------------------------

/// `&mut EngineState` core of [`print_skip_param`].
pub(crate) fn print_skip_param_in(st: &mut EngineState, n: i32) {
    match n {
        0 => print_esc_cstr_in(st, c"lineskip".as_ptr()),
        1 => print_esc_cstr_in(st, c"baselineskip".as_ptr()),
        2 => print_esc_cstr_in(st, c"parskip".as_ptr()),
        3 => print_esc_cstr_in(st, c"abovedisplayskip".as_ptr()),
        4 => print_esc_cstr_in(st, c"belowdisplayskip".as_ptr()),
        5 => print_esc_cstr_in(st, c"abovedisplayshortskip".as_ptr()),
        6 => print_esc_cstr_in(st, c"belowdisplayshortskip".as_ptr()),
        7 => print_esc_cstr_in(st, c"leftskip".as_ptr()),
        8 => print_esc_cstr_in(st, c"rightskip".as_ptr()),
        9 => print_esc_cstr_in(st, c"topskip".as_ptr()),
        10 => print_esc_cstr_in(st, c"splittopskip".as_ptr()),
        11 => print_esc_cstr_in(st, c"tabskip".as_ptr()),
        12 => print_esc_cstr_in(st, c"spaceskip".as_ptr()),
        13 => print_esc_cstr_in(st, c"xspaceskip".as_ptr()),
        14 => print_esc_cstr_in(st, c"parfillskip".as_ptr()),
        15 => print_esc_cstr_in(st, c"XeTeXlinebreakskip".as_ptr()),
        16 => print_esc_cstr_in(st, c"thinmuskip".as_ptr()),
        17 => print_esc_cstr_in(st, c"medmuskip".as_ptr()),
        18 => print_esc_cstr_in(st, c"thickmuskip".as_ptr()),
        _ => print_cstr_in(st, c"[unknown glue parameter!]".as_ptr()),
    }
}

/// `&mut EngineState` core of [`print_param`].
pub(crate) fn print_param_in(st: &mut EngineState, n: i32) {
    match n {
        0 => print_esc_cstr_in(st, c"pretolerance".as_ptr()),
        1 => print_esc_cstr_in(st, c"tolerance".as_ptr()),
        2 => print_esc_cstr_in(st, c"linepenalty".as_ptr()),
        3 => print_esc_cstr_in(st, c"hyphenpenalty".as_ptr()),
        4 => print_esc_cstr_in(st, c"exhyphenpenalty".as_ptr()),
        5 => print_esc_cstr_in(st, c"clubpenalty".as_ptr()),
        6 => print_esc_cstr_in(st, c"widowpenalty".as_ptr()),
        7 => print_esc_cstr_in(st, c"displaywidowpenalty".as_ptr()),
        8 => print_esc_cstr_in(st, c"brokenpenalty".as_ptr()),
        9 => print_esc_cstr_in(st, c"binoppenalty".as_ptr()),
        10 => print_esc_cstr_in(st, c"relpenalty".as_ptr()),
        11 => print_esc_cstr_in(st, c"predisplaypenalty".as_ptr()),
        12 => print_esc_cstr_in(st, c"postdisplaypenalty".as_ptr()),
        13 => print_esc_cstr_in(st, c"interlinepenalty".as_ptr()),
        14 => print_esc_cstr_in(st, c"doublehyphendemerits".as_ptr()),
        15 => print_esc_cstr_in(st, c"finalhyphendemerits".as_ptr()),
        16 => print_esc_cstr_in(st, c"adjdemerits".as_ptr()),
        17 => print_esc_cstr_in(st, c"mag".as_ptr()),
        18 => print_esc_cstr_in(st, c"delimiterfactor".as_ptr()),
        19 => print_esc_cstr_in(st, c"looseness".as_ptr()),
        20 => print_esc_cstr_in(st, c"time".as_ptr()),
        21 => print_esc_cstr_in(st, c"day".as_ptr()),
        22 => print_esc_cstr_in(st, c"month".as_ptr()),
        23 => print_esc_cstr_in(st, c"year".as_ptr()),
        24 => print_esc_cstr_in(st, c"showboxbreadth".as_ptr()),
        25 => print_esc_cstr_in(st, c"showboxdepth".as_ptr()),
        26 => print_esc_cstr_in(st, c"hbadness".as_ptr()),
        27 => print_esc_cstr_in(st, c"vbadness".as_ptr()),
        28 => print_esc_cstr_in(st, c"pausing".as_ptr()),
        29 => print_esc_cstr_in(st, c"tracingonline".as_ptr()),
        30 => print_esc_cstr_in(st, c"tracingmacros".as_ptr()),
        31 => print_esc_cstr_in(st, c"tracingstats".as_ptr()),
        32 => print_esc_cstr_in(st, c"tracingparagraphs".as_ptr()),
        33 => print_esc_cstr_in(st, c"tracingpages".as_ptr()),
        34 => print_esc_cstr_in(st, c"tracingoutput".as_ptr()),
        35 => print_esc_cstr_in(st, c"tracinglostchars".as_ptr()),
        36 => print_esc_cstr_in(st, c"tracingcommands".as_ptr()),
        37 => print_esc_cstr_in(st, c"tracingrestores".as_ptr()),
        38 => print_esc_cstr_in(st, c"uchyph".as_ptr()),
        39 => print_esc_cstr_in(st, c"outputpenalty".as_ptr()),
        40 => print_esc_cstr_in(st, c"maxdeadcycles".as_ptr()),
        41 => print_esc_cstr_in(st, c"hangafter".as_ptr()),
        42 => print_esc_cstr_in(st, c"floatingpenalty".as_ptr()),
        43 => print_esc_cstr_in(st, c"globaldefs".as_ptr()),
        44 => print_esc_cstr_in(st, c"fam".as_ptr()),
        45 => print_esc_cstr_in(st, c"escapechar".as_ptr()),
        46 => print_esc_cstr_in(st, c"defaulthyphenchar".as_ptr()),
        47 => print_esc_cstr_in(st, c"defaultskewchar".as_ptr()),
        48 => print_esc_cstr_in(st, c"endlinechar".as_ptr()),
        49 => print_esc_cstr_in(st, c"newlinechar".as_ptr()),
        50 => print_esc_cstr_in(st, c"language".as_ptr()),
        51 => print_esc_cstr_in(st, c"lefthyphenmin".as_ptr()),
        52 => print_esc_cstr_in(st, c"righthyphenmin".as_ptr()),
        53 => print_esc_cstr_in(st, c"holdinginserts".as_ptr()),
        54 => print_esc_cstr_in(st, c"errorcontextlines".as_ptr()),
        55 => print_esc_cstr_in(st, c"tracingstacklevels".as_ptr()),
        56 => print_esc_cstr_in(st, c"tracingassigns".as_ptr()),
        57 => print_esc_cstr_in(st, c"tracinggroups".as_ptr()),
        58 => print_esc_cstr_in(st, c"tracingifs".as_ptr()),
        59 => print_esc_cstr_in(st, c"tracingscantokens".as_ptr()),
        60 => print_esc_cstr_in(st, c"tracingnesting".as_ptr()),
        61 => print_esc_cstr_in(st, c"predisplaydirection".as_ptr()),
        62 => print_esc_cstr_in(st, c"lastlinefit".as_ptr()),
        63 => print_esc_cstr_in(st, c"savingvdiscards".as_ptr()),
        64 => print_esc_cstr_in(st, c"savinghyphcodes".as_ptr()),
        65 => print_esc_cstr_in(st, c"suppressfontnotfounderror".as_ptr()),
        // 66 = xetex_linebreak_locale — not in C switch (no matching case)
        67 => print_esc_cstr_in(st, c"XeTeXlinebreakpenalty".as_ptr()),
        68 => print_esc_cstr_in(st, c"XeTeXprotrudechars".as_ptr()),
        69 => print_esc_cstr_in(st, c"TeXXeTstate".as_ptr()),
        70 => print_esc_cstr_in(st, c"XeTeXdashbreakstate".as_ptr()),
        71 => print_esc_cstr_in(st, c"XeTeXupwardsmode".as_ptr()),
        72 => print_esc_cstr_in(st, c"XeTeXuseglyphmetrics".as_ptr()),
        73 => print_esc_cstr_in(st, c"XeTeXinterchartokenstate".as_ptr()),
        74 => print_esc_cstr_in(st, c"XeTeXinputnormalization".as_ptr()),
        77 => print_esc_cstr_in(st, c"XeTeXtracingfonts".as_ptr()),
        78 => print_esc_cstr_in(st, c"XeTeXinterwordspaceshaping".as_ptr()),
        79 => print_esc_cstr_in(st, c"XeTeXgenerateactualtext".as_ptr()),
        80 => print_esc_cstr_in(st, c"XeTeXhyphenatablelength".as_ptr()),
        81 => print_esc_cstr_in(st, c"synctex".as_ptr()),
        82 => print_esc_cstr_in(st, c"pdfoutput".as_ptr()),
        _ => print_cstr_in(st, c"[unknown int32_t parameter!]".as_ptr()),
    }
}

// ---------------------------------------------------------------------------
// print_style / print_fam_and_char / print_delimiter
// ---------------------------------------------------------------------------

/// `&mut EngineState` core of [`print_style`].
pub(crate) fn print_style_in(st: &mut EngineState, c: i32) {
    match c / 2 {
        0 => print_esc_cstr_in(st, c"displaystyle".as_ptr()), // DISPLAY_STYLE / 2
        1 => print_esc_cstr_in(st, c"textstyle".as_ptr()),    // TEXT_STYLE / 2
        2 => print_esc_cstr_in(st, c"scriptstyle".as_ptr()),  // SCRIPT_STYLE / 2
        3 => print_esc_cstr_in(st, c"scriptscriptstyle".as_ptr()), // SCRIPT_SCRIPT_STYLE / 2
        _ => print_cstr_in(st, c"Unknown style!".as_ptr()),
    }
}

/// Print the fam number and character code stored in a math node p.
///
/// translated from xetex-xetex0.c:294 (print_fam_and_char)
pub(crate) fn print_fam_and_char_in(st: &mut EngineState, p: i32) {
    print_esc_cstr_in(st, c"fam".as_ptr());
    print_int_in(st, (st.mem.b16s1(p) % 256) as i32);
    print_char_in(st, ' ' as i32);
    let c = (st.mem.b16s0(p) as i64 + ((st.mem.b16s1(p) as i64 / 256) * 65536)) as i32;
    if c < 65536 {
        print_in(st, c);
    } else {
        print_char_in(st, c);
    }
}

/// Print a delimiter code (four 16-bit fields packed as a hex number).
///
/// translated from xetex-xetex0.c:307 (print_delimiter)
pub(crate) fn print_delimiter_in(st: &mut EngineState, p: i32) {
    let s3 = st.mem.b16s3(p) as i64;
    let s2 = st.mem.b16s2(p) as i64;
    let s1 = st.mem.b16s1(p) as i64;
    let s0 = st.mem.b16s0(p) as i64;
    let mut a = (s3 % 256) * 256 + (s2 + (s3 / 256) * 65536);
    a = a * 4096 + (s1 % 256) * 256 + (s0 + (s1 / 256) * 65536);
    let a32 = a as i32;
    if a32 < 0 {
        print_int_in(st, a32);
    } else {
        print_hex_in(st, a32);
    }
}

// ---------------------------------------------------------------------------
// print_rule_dimen / print_glue
// ---------------------------------------------------------------------------

// Glue order constants (xetex-constants.h:173–182).
const NORMAL: i32 = 0;
const FIL: i32 = 1;
const FILLL: i32 = 3;

/// Print a rule dimension (print '*' for a null-flag wildcard).
///
/// translated from xetex-xetex0.c:250 (print_rule_dimen)
pub(crate) fn print_rule_dimen_in(st: &mut EngineState, d: i32) {
    if d == NULL_FLAG {
        print_char_in(st, '*' as i32);
    } else {
        print_scaled_in(st, d);
    }
}

/// Print a glue dimension with optional fil/fill/filll suffix.
///
/// translated from xetex-xetex0.c:258 (print_glue)
pub(crate) fn print_glue_in(st: &mut EngineState, d: i32, order: i32, s: *const libc::c_char) {
    print_scaled_in(st, d);
    if !(NORMAL..=FILLL).contains(&order) {
        print_cstr_in(st, c"foul".as_ptr());
    } else if order > NORMAL {
        print_cstr_in(st, c"fil".as_ptr());
        let mut ord = order;
        while ord > FIL {
            print_char_in(st, 'l' as i32);
            ord -= 1;
        }
    } else if !s.is_null() {
        print_cstr_in(st, s);
    }
}

/// `&mut EngineState` core of [`print_length_param`].
pub(crate) fn print_length_param_in(st: &mut EngineState, n: i32) {
    match n {
        0 => print_esc_cstr_in(st, c"parindent".as_ptr()),
        1 => print_esc_cstr_in(st, c"mathsurround".as_ptr()),
        2 => print_esc_cstr_in(st, c"lineskiplimit".as_ptr()),
        3 => print_esc_cstr_in(st, c"hsize".as_ptr()),
        4 => print_esc_cstr_in(st, c"vsize".as_ptr()),
        5 => print_esc_cstr_in(st, c"maxdepth".as_ptr()),
        6 => print_esc_cstr_in(st, c"splitmaxdepth".as_ptr()),
        7 => print_esc_cstr_in(st, c"boxmaxdepth".as_ptr()),
        8 => print_esc_cstr_in(st, c"hfuzz".as_ptr()),
        9 => print_esc_cstr_in(st, c"vfuzz".as_ptr()),
        10 => print_esc_cstr_in(st, c"delimitershortfall".as_ptr()),
        11 => print_esc_cstr_in(st, c"nulldelimiterspace".as_ptr()),
        12 => print_esc_cstr_in(st, c"scriptspace".as_ptr()),
        13 => print_esc_cstr_in(st, c"predisplaysize".as_ptr()),
        14 => print_esc_cstr_in(st, c"displaywidth".as_ptr()),
        15 => print_esc_cstr_in(st, c"displayindent".as_ptr()),
        16 => print_esc_cstr_in(st, c"overfullrule".as_ptr()),
        17 => print_esc_cstr_in(st, c"hangindent".as_ptr()),
        18 => print_esc_cstr_in(st, c"hoffset".as_ptr()),
        19 => print_esc_cstr_in(st, c"voffset".as_ptr()),
        20 => print_esc_cstr_in(st, c"emergencystretch".as_ptr()),
        21 => print_esc_cstr_in(st, c"pdfpagewidth".as_ptr()),
        22 => print_esc_cstr_in(st, c"pdfpageheight".as_ptr()),
        _ => print_cstr_in(st, c"[unknown dimen parameter!]".as_ptr()),
    }
}

// ---------------------------------------------------------------------------
// print_font_and_char
// ---------------------------------------------------------------------------

/// Print the font escape-name and character for a char-node at `p`.
///
/// translated from xetex-xetex0.c:225 (print_font_and_char)
pub(crate) fn print_font_and_char_in(st: &mut EngineState, p: i32) {
    if p > st.mem.mem_end {
        print_esc_cstr_in(st, c"CLOBBERED.".as_ptr());
    } else {
        if st.mem.b16s1(p) > st.fonts.font_max as u16 {
            print_char_in(st, '*' as i32);
        } else {
            // FONT_ID_BASE + font_id → hash entry → string number of the font name.
            let s = st.eqtb.hash_s1(FONT_ID_BASE + st.mem.b16s1(p) as i32);
            print_esc_in(st, s);
        }
        print_char_in(st, ' ' as i32);
        print_in(st, st.mem.b16s0(p) as i32);
    }
}

// ---------------------------------------------------------------------------
// print_mark
// ---------------------------------------------------------------------------

/// Print a mark token list enclosed in `{…}`.
///
/// translated from xetex-xetex0.c:240 (print_mark)
pub(crate) fn print_mark_in(st: &mut EngineState, p: i32) {
    print_char_in(st, '{' as i32);
    if p < st.mem.hi_mem_min || p > st.mem.mem_end {
        print_esc_cstr_in(st, c"CLOBBERED.".as_ptr());
    } else {
        let head = st.mem.b32s1(p);
        let l = st.config.max_print_line - 10;
        show_token_list_in(st, head, TEX_NULL, l);
    }
    print_char_in(st, '}' as i32);
}

// ---------------------------------------------------------------------------
// print_cmd_chr
// ---------------------------------------------------------------------------

// Command codes (xetex_format.h:229–359)
const CMD_RELAX: u16 = 0;
const CMD_LEFT_BRACE: u16 = 1;
const CMD_RIGHT_BRACE: u16 = 2;
const CMD_MATH_SHIFT: u16 = 3;
const CMD_TAB_MARK: u16 = 4;
const CMD_CAR_RET: u16 = 5;
const CMD_MAC_PARAM: u16 = 6;
const CMD_SUP_MARK: u16 = 7;
const CMD_SUB_MARK: u16 = 8;
const CMD_ENDV: u16 = 9;
const CMD_SPACER: u16 = 10;
const CMD_LETTER: u16 = 11;
const CMD_OTHER_CHAR: u16 = 12;
const CMD_PAR_END: u16 = 13;
const CMD_STOP: u16 = 14;
const CMD_DELIM_NUM: u16 = 15;
const CMD_CHAR_NUM: u16 = 16;
const CMD_MATH_CHAR_NUM: u16 = 17;
const CMD_MARK: u16 = 18;
const CMD_XRAY: u16 = 19;
const CMD_MAKE_BOX: u16 = 20;
const CMD_HMOVE: u16 = 21;
const CMD_VMOVE: u16 = 22;
const CMD_UN_HBOX: u16 = 23;
const CMD_UN_VBOX: u16 = 24;
const CMD_REMOVE_ITEM: u16 = 25;
const CMD_HSKIP: u16 = 26;
const CMD_VSKIP: u16 = 27;
const CMD_MSKIP: u16 = 28;
const CMD_KERN: u16 = 29;
const CMD_MKERN: u16 = 30;
const CMD_LEADER_SHIP: u16 = 31;
const CMD_HALIGN: u16 = 32;
const CMD_VALIGN: u16 = 33;
const CMD_NO_ALIGN: u16 = 34;
const CMD_VRULE: u16 = 35;
const CMD_HRULE: u16 = 36;
const CMD_INSERT: u16 = 37;
const CMD_VADJUST: u16 = 38;
const CMD_IGNORE_SPACES: u16 = 39;
const CMD_AFTER_ASSIGNMENT: u16 = 40;
const CMD_AFTER_GROUP: u16 = 41;
const CMD_BREAK_PENALTY: u16 = 42;
const CMD_START_PAR: u16 = 43;
const CMD_ITAL_CORR: u16 = 44;
const CMD_ACCENT: u16 = 45;
const CMD_MATH_ACCENT: u16 = 46;
const CMD_DISCRETIONARY: u16 = 47;
const CMD_EQ_NO: u16 = 48;
const CMD_LEFT_RIGHT: u16 = 49;
const CMD_MATH_COMP: u16 = 50;
const CMD_LIMIT_SWITCH: u16 = 51;
const CMD_ABOVE: u16 = 52;
const CMD_MATH_STYLE: u16 = 53;
const CMD_MATH_CHOICE: u16 = 54;
const CMD_NON_SCRIPT: u16 = 55;
const CMD_VCENTER: u16 = 56;
const CMD_CASE_SHIFT: u16 = 57;
const CMD_MESSAGE: u16 = 58;
const CMD_EXTENSION: u16 = 59;
const CMD_IN_STREAM: u16 = 60;
const CMD_BEGIN_GROUP: u16 = 61;
const CMD_END_GROUP: u16 = 62;
const CMD_OMIT: u16 = 63;
const CMD_EX_SPACE: u16 = 64;
const CMD_NO_BOUNDARY: u16 = 65;
const CMD_RADICAL: u16 = 66;
const CMD_END_CS_NAME: u16 = 67;
const CMD_CHAR_GIVEN: u16 = 68;
const CMD_MATH_GIVEN: u16 = 69;
const CMD_XETEX_MATH_GIVEN: u16 = 70;
const CMD_LAST_ITEM: u16 = 71;
const CMD_TOKS_REGISTER: u16 = 72;
const CMD_ASSIGN_TOKS: u16 = 73;
const CMD_ASSIGN_INT: u16 = 74;
const CMD_ASSIGN_DIMEN: u16 = 75;
const CMD_ASSIGN_GLUE: u16 = 76;
const CMD_ASSIGN_MU_GLUE: u16 = 77;
const CMD_ASSIGN_FONT_DIMEN: u16 = 78;
const CMD_ASSIGN_FONT_INT: u16 = 79;
const CMD_SET_AUX: u16 = 80;
const CMD_SET_PREV_GRAF: u16 = 81;
const CMD_SET_PAGE_DIMEN: u16 = 82;
const CMD_SET_PAGE_INT: u16 = 83;
const CMD_SET_BOX_DIMEN: u16 = 84;
const CMD_SET_SHAPE: u16 = 85;
const CMD_DEF_CODE: u16 = 86;
const CMD_XETEX_DEF_CODE: u16 = 87;
const CMD_DEF_FAMILY: u16 = 88;
const CMD_SET_FONT: u16 = 89;
const CMD_DEF_FONT: u16 = 90;
const CMD_REGISTER: u16 = 91;
const CMD_ADVANCE: u16 = 92;
const CMD_MULTIPLY: u16 = 93;
const CMD_DIVIDE: u16 = 94;
const CMD_PREFIX: u16 = 95;
const CMD_LET: u16 = 96;
const CMD_SHORTHAND_DEF: u16 = 97;
const CMD_READ_TO_CS: u16 = 98;
const CMD_DEF: u16 = 99;
const CMD_SET_BOX: u16 = 100;
const CMD_HYPH_DATA: u16 = 101;
const CMD_SET_INTERACTION: u16 = 102;
const CMD_UNDEFINED_CS: u16 = 103;
const CMD_EXPAND_AFTER: u16 = 104;
const CMD_NO_EXPAND: u16 = 105;
const CMD_INPUT: u16 = 106;
const CMD_IF_TEST: u16 = 107;
const CMD_FI_OR_ELSE: u16 = 108;
const CMD_CS_NAME: u16 = 109;
const CMD_CONVERT: u16 = 110;
const CMD_THE: u16 = 111;
const CMD_TOP_BOT_MARK: u16 = 112;
const CMD_CALL: u16 = 113;
const CMD_LONG_CALL: u16 = 114;
const CMD_OUTER_CALL: u16 = 115;
const CMD_LONG_OUTER_CALL: u16 = 116;
const CMD_END_TEMPLATE: u16 = 117;
// Memory layout constants (xetex_format.h:205–213)
const CC_GLUE_BASE: i32 = 2_254_340;
const CC_SKIP_BASE: i32 = 2_254_359;
const CC_MU_SKIP_BASE: i32 = 2_254_615;
const CC_LOCAL_BASE: i32 = 2_254_871;
const CC_TOKS_BASE: i32 = 2_254_884;
const CC_ETEX_PEN_BASE: i32 = 2_255_140;
const CC_MATH_FONT_BASE: i32 = 2_255_401;
const CC_INT_BASE: i32 = 7_826_729;
const CC_COUNT_BASE: i32 = 7_826_812;
const CC_DIMEN_BASE: i32 = 8_941_180;
const CC_SCALED_BASE: i32 = 8_941_203;
const CC_LC_CODE_BASE: i32 = 3_370_281;
const CC_UC_CODE_BASE: i32 = 4_484_393;
const CC_SF_CODE_BASE: i32 = 5_598_505;
const CC_MATH_CODE_BASE: i32 = 6_712_617;
const CC_DEL_CODE_BASE: i32 = 7_827_068;
// LOCAL__ offsets (xetex_format.h:172–184)
const CC_LOCAL_PAR_SHAPE: i32 = 0;
const CC_LOCAL_OUTPUT_ROUTINE: i32 = 1;
const CC_LOCAL_EVERY_PAR: i32 = 2;
const CC_LOCAL_EVERY_MATH: i32 = 3;
const CC_LOCAL_EVERY_DISPLAY: i32 = 4;
const CC_LOCAL_EVERY_HBOX: i32 = 5;
const CC_LOCAL_EVERY_VBOX: i32 = 6;
const CC_LOCAL_EVERY_JOB: i32 = 7;
const CC_LOCAL_EVERY_CR: i32 = 8;
const CC_LOCAL_EVERY_EOF: i32 = 10;
const CC_LOCAL_XETEX_INTER_CHAR_TOKS: i32 = 11;
const CC_LOCAL_TECTONIC_CODA_TOKENS: i32 = 12;
// ETEX_PENALTIES_PAR__ offsets (xetex_format.h:189–192)
const CC_ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES: i32 = 0;
const CC_ETEX_PENALTIES_PAR_CLUB_PENALTIES: i32 = 1;
const CC_ETEX_PENALTIES_PAR_WIDOW_PENALTIES: i32 = 2;
const CC_ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES: i32 = 3;
// IF_TEST chr_codes (xetex_format.h:544–564)
const CC_IF_CAT_CODE: i32 = 1;
const CC_IF_INT_CODE: i32 = 2;
const CC_IF_DIM_CODE: i32 = 3;
const CC_IF_ODD_CODE: i32 = 4;
const CC_IF_VMODE_CODE: i32 = 5;
const CC_IF_HMODE_CODE: i32 = 6;
const CC_IF_MMODE_CODE: i32 = 7;
const CC_IF_INNER_CODE: i32 = 8;
const CC_IF_VOID_CODE: i32 = 9;
const CC_IF_HBOX_CODE: i32 = 10;
const CC_IF_VBOX_CODE: i32 = 11;
const CC_IFX_CODE: i32 = 12;
const CC_IF_EOF_CODE: i32 = 13;
const CC_IF_TRUE_CODE: i32 = 14;
const CC_IF_FALSE_CODE: i32 = 15;
const CC_IF_CASE_CODE: i32 = 16;
const CC_IF_DEF_CODE: i32 = 17;
const CC_IF_CS_CODE: i32 = 18;
const CC_IF_FONT_CHAR_CODE: i32 = 19;
const CC_IF_IN_CSNAME_CODE: i32 = 20;
const CC_IF_PRIMITIVE_CODE: i32 = 21;
const CC_UNLESS_CODE: i32 = 32;
// FI_OR_ELSE chr_codes
const CC_FI_CODE: i32 = 2;
const CC_OR_CODE: i32 = 4;
// TOP_BOT_MARK chr_codes (xetex_format.h:686–694)
const CC_FIRST_MARK_CODE: i32 = 1;
const CC_BOT_MARK_CODE: i32 = 2;
const CC_SPLIT_FIRST_MARK_CODE: i32 = 3;
const CC_SPLIT_BOT_MARK_CODE: i32 = 4;
const CC_MARKS_CODE: i32 = 5;
// LAST_ITEM chr_codes (xetex_format.h:568–635)
const CC_INT_VAL: i32 = 0;
const CC_DIMEN_VAL: i32 = 1;
const CC_GLUE_VAL: i32 = 2;
const CC_MU_VAL: i32 = 3;
const CC_LAST_NODE_TYPE_CODE: i32 = 3;
const CC_INPUT_LINE_NO_CODE: i32 = 4;
const CC_PDF_LAST_X_POS_CODE: i32 = 12;
const CC_PDF_LAST_Y_POS_CODE: i32 = 13;
const CC_ELAPSED_TIME_CODE: i32 = 16;
const CC_PDF_SHELL_ESCAPE_CODE: i32 = 17;
const CC_RANDOM_SEED_CODE: i32 = 18;
const CC_ETEX_VERSION_CODE: i32 = 19;
const CC_CURRENT_GROUP_LEVEL_CODE: i32 = 20;
const CC_CURRENT_GROUP_TYPE_CODE: i32 = 21;
const CC_CURRENT_IF_LEVEL_CODE: i32 = 22;
const CC_CURRENT_IF_TYPE_CODE: i32 = 23;
const CC_CURRENT_IF_BRANCH_CODE: i32 = 24;
const CC_GLUE_STRETCH_ORDER_CODE: i32 = 25;
const CC_GLUE_SHRINK_ORDER_CODE: i32 = 26;
const CC_XETEX_VERSION_CODE: i32 = 27;
const CC_XETEX_COUNT_GLYPHS_CODE: i32 = 28;
const CC_XETEX_COUNT_VARIATIONS_CODE: i32 = 29;
const CC_XETEX_VARIATION_CODE: i32 = 30;
const CC_XETEX_FIND_VARIATION_BY_NAME_CODE: i32 = 31;
const CC_XETEX_VARIATION_MIN_CODE: i32 = 32;
const CC_XETEX_VARIATION_MAX_CODE: i32 = 33;
const CC_XETEX_VARIATION_DEFAULT_CODE: i32 = 34;
const CC_XETEX_COUNT_FEATURES_CODE: i32 = 35;
const CC_XETEX_FEATURE_CODE_CODE: i32 = 36;
const CC_XETEX_FIND_FEATURE_BY_NAME_CODE: i32 = 37;
const CC_XETEX_IS_EXCLUSIVE_FEATURE_CODE: i32 = 38;
const CC_XETEX_COUNT_SELECTORS_CODE: i32 = 39;
const CC_XETEX_SELECTOR_CODE_CODE: i32 = 40;
const CC_XETEX_FIND_SELECTOR_BY_NAME_CODE: i32 = 41;
const CC_XETEX_IS_DEFAULT_SELECTOR_CODE: i32 = 42;
const CC_XETEX_OT_COUNT_SCRIPTS_CODE: i32 = 43;
const CC_XETEX_OT_COUNT_LANGUAGES_CODE: i32 = 44;
const CC_XETEX_OT_COUNT_FEATURES_CODE: i32 = 45;
const CC_XETEX_OT_SCRIPT_CODE: i32 = 46;
const CC_XETEX_OT_LANGUAGE_CODE: i32 = 47;
const CC_XETEX_OT_FEATURE_CODE: i32 = 48;
const CC_XETEX_MAP_CHAR_TO_GLYPH_CODE: i32 = 49;
const CC_XETEX_GLYPH_INDEX_CODE: i32 = 50;
const CC_XETEX_FONT_TYPE_CODE: i32 = 51;
const CC_XETEX_FIRST_CHAR_CODE: i32 = 52;
const CC_XETEX_LAST_CHAR_CODE: i32 = 53;
const CC_XETEX_PDF_PAGE_COUNT_CODE: i32 = 54;
const CC_XETEX_GLYPH_BOUNDS_CODE: i32 = 55;
const CC_FONT_CHAR_WD_CODE: i32 = 56;
const CC_FONT_CHAR_HT_CODE: i32 = 57;
const CC_FONT_CHAR_DP_CODE: i32 = 58;
const CC_FONT_CHAR_IC_CODE: i32 = 59;
const CC_PAR_SHAPE_LENGTH_CODE: i32 = 60;
const CC_PAR_SHAPE_INDENT_CODE: i32 = 61;
const CC_PAR_SHAPE_DIMEN_CODE: i32 = 62;
const CC_ETEX_EXPR: i32 = 67;
const CC_GLUE_STRETCH_CODE: i32 = 63;
const CC_GLUE_SHRINK_CODE: i32 = 64;
const CC_MU_TO_GLUE_CODE: i32 = 65;
const CC_GLUE_TO_MU_CODE: i32 = 66;
// CONVERT chr_codes (xetex_format.h:490–518)
const CC_NUMBER_CODE: i32 = 0;
const CC_ROMAN_NUMERAL_CODE: i32 = 1;
const CC_STRING_CODE: i32 = 2;
const CC_MEANING_CODE: i32 = 3;
const CC_FONT_NAME_CODE: i32 = 4;
const CC_ETEX_REVISION_CODE: i32 = 5;
const CC_EXPANDED_CODE: i32 = 6;
const CC_LEFT_MARGIN_KERN_CODE: i32 = 16;
const CC_RIGHT_MARGIN_KERN_CODE: i32 = 17;
const CC_PDF_STRCMP_CODE: i32 = 18;
const CC_PDF_CREATION_DATE_CODE: i32 = 22;
const CC_PDF_FILE_MOD_DATE_CODE: i32 = 23;
const CC_PDF_FILE_SIZE_CODE: i32 = 24;
const CC_PDF_MDFIVE_SUM_CODE: i32 = 25;
const CC_PDF_FILE_DUMP_CODE: i32 = 26;
const CC_UNIFORM_DEVIATE_CODE: i32 = 29;
const CC_NORMAL_DEVIATE_CODE: i32 = 30;
const CC_XETEX_REVISION_CODE: i32 = 33;
const CC_XETEX_VARIATION_NAME_CODE: i32 = 34;
const CC_XETEX_FEATURE_NAME_CODE: i32 = 35;
const CC_XETEX_SELECTOR_NAME_CODE: i32 = 36;
const CC_XETEX_GLYPH_NAME_CODE: i32 = 37;
const CC_XETEX_UCHAR_CODE: i32 = 38;
const CC_XETEX_UCHARCAT_CODE: i32 = 39;
// SET_SHAPE sub-codes
const CC_LOCAL_BASE_PAR_SHAPE: i32 = CC_LOCAL_BASE + CC_LOCAL_PAR_SHAPE;
const CC_ETEX_PEN_BASE_INTER_LINE: i32 =
    CC_ETEX_PEN_BASE + CC_ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES;
const CC_ETEX_PEN_BASE_CLUB: i32 = CC_ETEX_PEN_BASE + CC_ETEX_PENALTIES_PAR_CLUB_PENALTIES;
const CC_ETEX_PEN_BASE_WIDOW: i32 = CC_ETEX_PEN_BASE + CC_ETEX_PENALTIES_PAR_WIDOW_PENALTIES;
const CC_ETEX_PEN_BASE_DISPLAY_WIDOW: i32 =
    CC_ETEX_PEN_BASE + CC_ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES;
// MAKE_BOX sub-codes (xetex_format.h:480–484)
const CC_BOX_CODE: i32 = 0;
const CC_COPY_CODE: i32 = 1;
const CC_LAST_BOX_CODE: i32 = 2;
const CC_VSPLIT_CODE: i32 = 3;
const CC_VTOP_CODE: i32 = 4;
const CC_VMODE: i32 = 1;
// HSKIP/VSKIP sub-codes (xetex_format.h:670–674)
const CC_FIL_CODE: i32 = 0;
const CC_FILL_CODE: i32 = 1;
const CC_SS_CODE: i32 = 2;
const CC_SKIP_CODE: i32 = 4;
// LEADER_SHIP sub-codes (xetex_format.h:422–424)
const CC_A_LEADERS: i32 = 100;
const CC_C_LEADERS: i32 = 101;
const CC_X_LEADERS: i32 = 102;
// UN_VBOX sub-codes
const CC_VSPLIT_CODE2: i32 = 3;
// REMOVE_ITEM sub-codes (xetex_format.h:390–391)
const CC_GLUE_NODE: i32 = 10;
const CC_KERN_NODE: i32 = 11;
// SET_BOX_DIMEN sub-codes (xetex_format.h:651,653)
const CC_WIDTH_OFFSET: i32 = 1;
const CC_HEIGHT_OFFSET: i32 = 3;
// VALIGN sub-codes (xetex_format.h:438–441)
const CC_BEGIN_L_CODE: i32 = 6;
const CC_END_L_CODE: i32 = 7;
const CC_BEGIN_R_CODE: i32 = 10;
// MATH_COMP sub-codes (xetex_format.h:401–411)
const CC_ORD_NOAD: i32 = 16;
const CC_OP_NOAD: i32 = 17;
const CC_BIN_NOAD: i32 = 18;
const CC_REL_NOAD: i32 = 19;
const CC_OPEN_NOAD: i32 = 20;
const CC_CLOSE_NOAD: i32 = 21;
const CC_PUNCT_NOAD: i32 = 22;
const CC_INNER_NOAD: i32 = 23;
const CC_UNDER_NOAD: i32 = 26;
// LIMIT_SWITCH sub-codes (xetex_format.h:452–453)
const CC_LIMITS: i32 = 1;
const CC_NO_LIMITS: i32 = 2;
// ABOVE sub-codes (xetex_format.h:471–473,190)
const CC_ABOVE_CODE: i32 = 0;
const CC_OVER_CODE: i32 = 1;
const CC_ATOP_CODE: i32 = 2;
const CC_DELIMITED_CODE: i32 = 3;
// LEFT_RIGHT sub-codes (xetex_format.h:415,390)
const CC_LEFT_NOAD: i32 = 30;
const CC_MIDDLE_NOAD: i32 = 1;
// SHORTHAND_DEF sub-codes (xetex_format.h:657–666)
const CC_CHAR_DEF_CODE: i32 = 0;
const CC_MATH_CHAR_DEF_CODE: i32 = 1;
const CC_COUNT_DEF_CODE: i32 = 2;
const CC_DIMEN_DEF_CODE: i32 = 3;
const CC_SKIP_DEF_CODE: i32 = 4;
const CC_MU_SKIP_DEF_CODE: i32 = 5;
const CC_CHAR_SUB_DEF_CODE: i32 = 7;
const CC_XETEX_MATH_CHAR_NUM_DEF_CODE: i32 = 8;
const CC_XETEX_MATH_CHAR_DEF_CODE: i32 = 9;
// DEF sub-codes
const CC_NORMAL: i32 = 0;
// TAB_MARK special sub-code (xetex_format.h:679)
const CC_SPAN_CODE: i32 = 1_114_113;
// CAR_RET sub-codes (xetex_format.h)
const CC_CR_CODE: i32 = 1_114_114;
// XRAY sub-codes (xetex_format.h:699–704)
const CC_SHOW_BOX_CODE: i32 = 1;
const CC_SHOW_THE_CODE: i32 = 2;
const CC_SHOW_LISTS: i32 = 3;
const CC_SHOW_GROUPS: i32 = 4;
const CC_SHOW_TOKENS: i32 = 5;
const CC_SHOW_IFS: i32 = 6;
// EXTENSION sub-codes (xetex_format.h:457–533)
const CC_OPEN_NODE: i32 = 0;
const CC_WRITE_NODE: i32 = 1;
const CC_CLOSE_NODE: i32 = 2;
const CC_SPECIAL_NODE: i32 = 3;
const CC_IMMEDIATE_CODE: i32 = 4;
const CC_SET_LANGUAGE_CODE: i32 = 5;
const CC_PDF_SAVE_POS_NODE: i32 = 21;
const CC_RESET_TIMER_CODE: i32 = 31;
const CC_SET_RANDOM_SEED_CODE: i32 = 33;
const CC_PIC_FILE_CODE: i32 = 41;
const CC_PDF_FILE_CODE: i32 = 42;
const CC_GLYPH_CODE: i32 = 43;
const CC_XETEX_LINEBREAK_LOCALE_EXTENSION_CODE: i32 = 46;
const CC_XETEX_INPUT_ENCODING_EXTENSION_CODE: i32 = 44;
const CC_XETEX_DEFAULT_ENCODING_EXTENSION_CODE: i32 = 45;
// SET_INTERACTION sub-codes (xetex_format.h:644–646)
const CC_BATCH_MODE: i32 = 0;
const CC_NONSTOP_MODE: i32 = 1;
const CC_SCROLL_MODE: i32 = 2;
// ASSIGN_FONT_INT sub-codes (xetex-constants.h:377–378)
const CC_LP_CODE_BASE: i32 = 2;
const CC_RP_CODE_BASE: i32 = 3;
// SET_FONT flags (xetex-ext.h:47–48)
const CC_AAT_FONT_FLAG: i32 = 0xFFFF;
const CC_OTGR_FONT_FLAG: i32 = 0xFFFE;
// PROTECTED_TOKEN (xetex-constants.h:361)
const CC_PROTECTED_TOKEN: i32 = 0x1C00001;

/// Print the name of command code `cmd` with subtype `chr_code`.
///
/// translated from xetex-xetex0.c:1265 (print_cmd_chr)
pub unsafe fn print_cmd_chr(cmd: u16, chr_code: i32) {
    crate::state::with_state_mut(|st| print_cmd_chr_in(st, cmd, chr_code));
}

/// `&mut EngineState` core of [`print_cmd_chr`].
pub(crate) fn print_cmd_chr_in(st: &mut EngineState, cmd: u16, chr_code: i32) {
    // Helper: print a char/codepoint (> 65535 uses print_char for raw code).
    let print_char_or_wide = |st: &mut EngineState, code: i32| {
        if code < 65536 {
            print_in(st, code);
        } else {
            print_char_in(st, code);
        }
    };

    match cmd {
        CMD_LEFT_BRACE => {
            print_cstr_in(st, c"begin-group character ".as_ptr());
            print_char_or_wide(st, chr_code);
        }
        CMD_RIGHT_BRACE => {
            print_cstr_in(st, c"end-group character ".as_ptr());
            print_char_or_wide(st, chr_code);
        }
        CMD_MATH_SHIFT => {
            print_cstr_in(st, c"math shift character ".as_ptr());
            print_char_or_wide(st, chr_code);
        }
        CMD_MAC_PARAM => {
            print_cstr_in(st, c"macro parameter character ".as_ptr());
            print_char_or_wide(st, chr_code);
        }
        CMD_SUP_MARK => {
            print_cstr_in(st, c"superscript character ".as_ptr());
            print_char_or_wide(st, chr_code);
        }
        CMD_SUB_MARK => {
            print_cstr_in(st, c"subscript character ".as_ptr());
            print_char_or_wide(st, chr_code);
        }
        CMD_ENDV => {
            print_cstr_in(st, c"end of alignment template".as_ptr());
        }
        CMD_SPACER => {
            print_cstr_in(st, c"blank space ".as_ptr());
            print_char_or_wide(st, chr_code);
        }
        CMD_LETTER => {
            print_cstr_in(st, c"the letter ".as_ptr());
            print_char_or_wide(st, chr_code);
        }
        CMD_OTHER_CHAR => {
            print_cstr_in(st, c"the character ".as_ptr());
            print_char_or_wide(st, chr_code);
        }
        CMD_ASSIGN_GLUE | CMD_ASSIGN_MU_GLUE => {
            if chr_code < CC_SKIP_BASE {
                print_skip_param_in(st, chr_code - CC_GLUE_BASE);
            } else if chr_code < CC_MU_SKIP_BASE {
                print_esc_cstr_in(st, c"skip".as_ptr());
                print_int_in(st, chr_code - CC_SKIP_BASE);
            } else {
                print_esc_cstr_in(st, c"muskip".as_ptr());
                print_int_in(st, chr_code - CC_MU_SKIP_BASE);
            }
        }
        CMD_ASSIGN_TOKS => {
            if chr_code >= CC_TOKS_BASE {
                print_esc_cstr_in(st, c"toks".as_ptr());
                print_int_in(st, chr_code - CC_TOKS_BASE);
            } else {
                match chr_code {
                    x if x == CC_LOCAL_BASE + CC_LOCAL_OUTPUT_ROUTINE => {
                        print_esc_cstr_in(st, c"output".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL_EVERY_PAR => {
                        print_esc_cstr_in(st, c"everypar".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL_EVERY_MATH => {
                        print_esc_cstr_in(st, c"everymath".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL_EVERY_DISPLAY => {
                        print_esc_cstr_in(st, c"everydisplay".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL_EVERY_HBOX => {
                        print_esc_cstr_in(st, c"everyhbox".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL_EVERY_VBOX => {
                        print_esc_cstr_in(st, c"everyvbox".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL_EVERY_JOB => {
                        print_esc_cstr_in(st, c"everyjob".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL_EVERY_CR => {
                        print_esc_cstr_in(st, c"everycr".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL_EVERY_EOF => {
                        print_esc_cstr_in(st, c"everyeof".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL_XETEX_INTER_CHAR_TOKS => {
                        print_esc_cstr_in(st, c"XeTeXinterchartoks".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL_TECTONIC_CODA_TOKENS => {
                        print_esc_cstr_in(st, c"TectonicCodaTokens".as_ptr())
                    }
                    _ => print_esc_cstr_in(st, c"errhelp".as_ptr()),
                }
            }
        }
        CMD_ASSIGN_INT => {
            if chr_code < CC_COUNT_BASE {
                print_param_in(st, chr_code - CC_INT_BASE);
            } else {
                print_esc_cstr_in(st, c"count".as_ptr());
                print_int_in(st, chr_code - CC_COUNT_BASE);
            }
        }
        CMD_ASSIGN_DIMEN => {
            if chr_code < CC_SCALED_BASE {
                print_length_param_in(st, chr_code - CC_DIMEN_BASE);
            } else {
                print_esc_cstr_in(st, c"dimen".as_ptr());
                print_int_in(st, chr_code - CC_SCALED_BASE);
            }
        }
        CMD_ACCENT => print_esc_cstr_in(st, c"accent".as_ptr()),
        CMD_ADVANCE => print_esc_cstr_in(st, c"advance".as_ptr()),
        CMD_AFTER_ASSIGNMENT => print_esc_cstr_in(st, c"afterassignment".as_ptr()),
        CMD_AFTER_GROUP => print_esc_cstr_in(st, c"aftergroup".as_ptr()),
        CMD_ASSIGN_FONT_DIMEN => print_esc_cstr_in(st, c"fontdimen".as_ptr()),
        CMD_BEGIN_GROUP => print_esc_cstr_in(st, c"begingroup".as_ptr()),
        CMD_BREAK_PENALTY => print_esc_cstr_in(st, c"penalty".as_ptr()),
        CMD_CHAR_NUM => print_esc_cstr_in(st, c"char".as_ptr()),
        CMD_CS_NAME => print_esc_cstr_in(st, c"csname".as_ptr()),
        CMD_DEF_FONT => print_esc_cstr_in(st, c"font".as_ptr()),
        CMD_DELIM_NUM => {
            if chr_code == 1 {
                print_esc_cstr_in(st, c"Udelimiter".as_ptr());
            } else {
                print_esc_cstr_in(st, c"delimiter".as_ptr());
            }
        }
        CMD_DIVIDE => print_esc_cstr_in(st, c"divide".as_ptr()),
        CMD_END_CS_NAME => print_esc_cstr_in(st, c"endcsname".as_ptr()),
        CMD_END_GROUP => print_esc_cstr_in(st, c"endgroup".as_ptr()),
        CMD_EX_SPACE => print_esc_in(st, ' ' as i32),
        CMD_EXPAND_AFTER => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"expandafter".as_ptr());
            } else {
                print_esc_cstr_in(st, c"unless".as_ptr());
            }
        }
        CMD_HALIGN => print_esc_cstr_in(st, c"halign".as_ptr()),
        CMD_HRULE => print_esc_cstr_in(st, c"hrule".as_ptr()),
        CMD_IGNORE_SPACES => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"ignorespaces".as_ptr());
            } else {
                print_esc_cstr_in(st, c"primitive".as_ptr());
            }
        }
        CMD_INSERT => print_esc_cstr_in(st, c"insert".as_ptr()),
        CMD_ITAL_CORR => print_esc_in(st, '/' as i32),
        CMD_MARK => {
            print_esc_cstr_in(st, c"mark".as_ptr());
            if chr_code > 0 {
                print_char_in(st, 's' as i32);
            }
        }
        CMD_MATH_ACCENT => {
            if chr_code == 1 {
                print_esc_cstr_in(st, c"Umathaccent".as_ptr());
            } else {
                print_esc_cstr_in(st, c"mathaccent".as_ptr());
            }
        }
        CMD_MATH_CHAR_NUM => {
            if chr_code == 2 {
                print_esc_cstr_in(st, c"Umathchar".as_ptr());
            } else if chr_code == 1 {
                print_esc_cstr_in(st, c"Umathcharnum".as_ptr());
            } else {
                print_esc_cstr_in(st, c"mathchar".as_ptr());
            }
        }
        CMD_MATH_CHOICE => print_esc_cstr_in(st, c"mathchoice".as_ptr()),
        CMD_MULTIPLY => print_esc_cstr_in(st, c"multiply".as_ptr()),
        CMD_NO_ALIGN => print_esc_cstr_in(st, c"noalign".as_ptr()),
        CMD_NO_BOUNDARY => print_esc_cstr_in(st, c"noboundary".as_ptr()),
        CMD_NO_EXPAND => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"noexpand".as_ptr());
            } else {
                print_esc_cstr_in(st, c"primitive".as_ptr());
            }
        }
        CMD_NON_SCRIPT => print_esc_cstr_in(st, c"nonscript".as_ptr()),
        CMD_OMIT => print_esc_cstr_in(st, c"omit".as_ptr()),
        CMD_RADICAL => {
            if chr_code == 1 {
                print_esc_cstr_in(st, c"Uradical".as_ptr());
            } else {
                print_esc_cstr_in(st, c"radical".as_ptr());
            }
        }
        CMD_READ_TO_CS => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"read".as_ptr());
            } else {
                print_esc_cstr_in(st, c"readline".as_ptr());
            }
        }
        CMD_RELAX => print_esc_cstr_in(st, c"relax".as_ptr()),
        CMD_SET_BOX => print_esc_cstr_in(st, c"setbox".as_ptr()),
        CMD_SET_PREV_GRAF => print_esc_cstr_in(st, c"prevgraf".as_ptr()),
        CMD_SET_SHAPE => match chr_code {
            CC_LOCAL_BASE_PAR_SHAPE => print_esc_cstr_in(st, c"parshape".as_ptr()),
            CC_ETEX_PEN_BASE_INTER_LINE => print_esc_cstr_in(st, c"interlinepenalties".as_ptr()),
            CC_ETEX_PEN_BASE_CLUB => print_esc_cstr_in(st, c"clubpenalties".as_ptr()),
            CC_ETEX_PEN_BASE_WIDOW => print_esc_cstr_in(st, c"widowpenalties".as_ptr()),
            CC_ETEX_PEN_BASE_DISPLAY_WIDOW => {
                print_esc_cstr_in(st, c"displaywidowpenalties".as_ptr())
            }
            _ => {}
        },
        CMD_THE => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"the".as_ptr());
            } else if chr_code == 1 {
                print_esc_cstr_in(st, c"unexpanded".as_ptr());
            } else {
                print_esc_cstr_in(st, c"detokenize".as_ptr());
            }
        }
        CMD_TOKS_REGISTER => {
            print_esc_cstr_in(st, c"toks".as_ptr());
            if chr_code != 0 {
                print_sa_num_in(st, chr_code);
            }
        }
        CMD_VADJUST => print_esc_cstr_in(st, c"vadjust".as_ptr()),
        CMD_VALIGN => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"valign".as_ptr());
            } else {
                match chr_code {
                    CC_BEGIN_L_CODE => print_esc_cstr_in(st, c"beginL".as_ptr()),
                    CC_END_L_CODE => print_esc_cstr_in(st, c"endL".as_ptr()),
                    CC_BEGIN_R_CODE => print_esc_cstr_in(st, c"beginR".as_ptr()),
                    _ => print_esc_cstr_in(st, c"endR".as_ptr()),
                }
            }
        }
        CMD_VCENTER => print_esc_cstr_in(st, c"vcenter".as_ptr()),
        CMD_VRULE => print_esc_cstr_in(st, c"vrule".as_ptr()),
        CMD_PAR_END => print_esc_cstr_in(st, c"par".as_ptr()),
        CMD_INPUT => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"input".as_ptr());
            } else if chr_code == 2 {
                print_esc_cstr_in(st, c"scantokens".as_ptr());
            } else {
                print_esc_cstr_in(st, c"endinput".as_ptr());
            }
        }
        CMD_TOP_BOT_MARK => {
            match chr_code % CC_MARKS_CODE {
                CC_FIRST_MARK_CODE => print_esc_cstr_in(st, c"firstmark".as_ptr()),
                CC_BOT_MARK_CODE => print_esc_cstr_in(st, c"botmark".as_ptr()),
                CC_SPLIT_FIRST_MARK_CODE => print_esc_cstr_in(st, c"splitfirstmark".as_ptr()),
                CC_SPLIT_BOT_MARK_CODE => print_esc_cstr_in(st, c"splitbotmark".as_ptr()),
                _ => print_esc_cstr_in(st, c"topmark".as_ptr()),
            }
            if chr_code >= CC_MARKS_CODE {
                print_char_in(st, 's' as i32);
            }
        }
        CMD_REGISTER => {
            let (the_cmd, the_chr) = if !(0..=19).contains(&chr_code) {
                (st.mem.b16s1(chr_code) as i32 / 64, chr_code)
            } else {
                (chr_code, TEX_NULL)
            };
            if the_cmd == CC_INT_VAL {
                print_esc_cstr_in(st, c"count".as_ptr());
            } else if the_cmd == CC_DIMEN_VAL {
                print_esc_cstr_in(st, c"dimen".as_ptr());
            } else if the_cmd == CC_GLUE_VAL {
                print_esc_cstr_in(st, c"skip".as_ptr());
            } else {
                print_esc_cstr_in(st, c"muskip".as_ptr());
            }
            if the_chr != TEX_NULL {
                print_sa_num_in(st, the_chr);
            }
        }
        CMD_SET_AUX => {
            if chr_code == CC_VMODE {
                print_esc_cstr_in(st, c"prevdepth".as_ptr());
            } else {
                print_esc_cstr_in(st, c"spacefactor".as_ptr());
            }
        }
        CMD_SET_PAGE_INT => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"deadcycles".as_ptr());
            } else if chr_code == 2 {
                print_esc_cstr_in(st, c"interactionmode".as_ptr());
            } else {
                print_esc_cstr_in(st, c"insertpenalties".as_ptr());
            }
        }
        CMD_SET_BOX_DIMEN => {
            if chr_code == CC_WIDTH_OFFSET {
                print_esc_cstr_in(st, c"wd".as_ptr());
            } else if chr_code == CC_HEIGHT_OFFSET {
                print_esc_cstr_in(st, c"ht".as_ptr());
            } else {
                print_esc_cstr_in(st, c"dp".as_ptr());
            }
        }
        CMD_LAST_ITEM => match chr_code {
            CC_INT_VAL => print_esc_cstr_in(st, c"lastpenalty".as_ptr()),
            CC_DIMEN_VAL => print_esc_cstr_in(st, c"lastkern".as_ptr()),
            CC_GLUE_VAL => print_esc_cstr_in(st, c"lastskip".as_ptr()),
            CC_INPUT_LINE_NO_CODE => print_esc_cstr_in(st, c"inputlineno".as_ptr()),
            CC_LAST_NODE_TYPE_CODE => print_esc_cstr_in(st, c"lastnodetype".as_ptr()),
            CC_ETEX_VERSION_CODE => print_esc_cstr_in(st, c"eTeXversion".as_ptr()),
            CC_XETEX_VERSION_CODE => print_esc_cstr_in(st, c"XeTeXversion".as_ptr()),
            CC_XETEX_COUNT_GLYPHS_CODE => print_esc_cstr_in(st, c"XeTeXcountglyphs".as_ptr()),
            CC_XETEX_COUNT_VARIATIONS_CODE => {
                print_esc_cstr_in(st, c"XeTeXcountvariations".as_ptr())
            }
            CC_XETEX_VARIATION_CODE => print_esc_cstr_in(st, c"XeTeXvariation".as_ptr()),
            CC_XETEX_FIND_VARIATION_BY_NAME_CODE => {
                print_esc_cstr_in(st, c"XeTeXfindvariationbyname".as_ptr())
            }
            CC_XETEX_VARIATION_MIN_CODE => print_esc_cstr_in(st, c"XeTeXvariationmin".as_ptr()),
            CC_XETEX_VARIATION_MAX_CODE => print_esc_cstr_in(st, c"XeTeXvariationmax".as_ptr()),
            CC_XETEX_VARIATION_DEFAULT_CODE => {
                print_esc_cstr_in(st, c"XeTeXvariationdefault".as_ptr())
            }
            CC_XETEX_COUNT_FEATURES_CODE => print_esc_cstr_in(st, c"XeTeXcountfeatures".as_ptr()),
            CC_XETEX_FEATURE_CODE_CODE => print_esc_cstr_in(st, c"XeTeXfeaturecode".as_ptr()),
            CC_XETEX_FIND_FEATURE_BY_NAME_CODE => {
                print_esc_cstr_in(st, c"XeTeXfindfeaturebyname".as_ptr())
            }
            CC_XETEX_IS_EXCLUSIVE_FEATURE_CODE => {
                print_esc_cstr_in(st, c"XeTeXisexclusivefeature".as_ptr())
            }
            CC_XETEX_COUNT_SELECTORS_CODE => print_esc_cstr_in(st, c"XeTeXcountselectors".as_ptr()),
            CC_XETEX_SELECTOR_CODE_CODE => print_esc_cstr_in(st, c"XeTeXselectorcode".as_ptr()),
            CC_XETEX_FIND_SELECTOR_BY_NAME_CODE => {
                print_esc_cstr_in(st, c"XeTeXfindselectorbyname".as_ptr())
            }
            CC_XETEX_IS_DEFAULT_SELECTOR_CODE => {
                print_esc_cstr_in(st, c"XeTeXisdefaultselector".as_ptr())
            }
            CC_XETEX_OT_COUNT_SCRIPTS_CODE => {
                print_esc_cstr_in(st, c"XeTeXOTcountscripts".as_ptr())
            }
            CC_XETEX_OT_COUNT_LANGUAGES_CODE => {
                print_esc_cstr_in(st, c"XeTeXOTcountlanguages".as_ptr())
            }
            CC_XETEX_OT_COUNT_FEATURES_CODE => {
                print_esc_cstr_in(st, c"XeTeXOTcountfeatures".as_ptr())
            }
            CC_XETEX_OT_SCRIPT_CODE => print_esc_cstr_in(st, c"XeTeXOTscripttag".as_ptr()),
            CC_XETEX_OT_LANGUAGE_CODE => print_esc_cstr_in(st, c"XeTeXOTlanguagetag".as_ptr()),
            CC_XETEX_OT_FEATURE_CODE => print_esc_cstr_in(st, c"XeTeXOTfeaturetag".as_ptr()),
            CC_XETEX_MAP_CHAR_TO_GLYPH_CODE => print_esc_cstr_in(st, c"XeTeXcharglyph".as_ptr()),
            CC_XETEX_GLYPH_INDEX_CODE => print_esc_cstr_in(st, c"XeTeXglyphindex".as_ptr()),
            CC_XETEX_GLYPH_BOUNDS_CODE => print_esc_cstr_in(st, c"XeTeXglyphbounds".as_ptr()),
            CC_XETEX_FONT_TYPE_CODE => print_esc_cstr_in(st, c"XeTeXfonttype".as_ptr()),
            CC_XETEX_FIRST_CHAR_CODE => print_esc_cstr_in(st, c"XeTeXfirstfontchar".as_ptr()),
            CC_XETEX_LAST_CHAR_CODE => print_esc_cstr_in(st, c"XeTeXlastfontchar".as_ptr()),
            CC_XETEX_PDF_PAGE_COUNT_CODE => print_esc_cstr_in(st, c"XeTeXpdfpagecount".as_ptr()),
            CC_CURRENT_GROUP_LEVEL_CODE => print_esc_cstr_in(st, c"currentgrouplevel".as_ptr()),
            CC_CURRENT_GROUP_TYPE_CODE => print_esc_cstr_in(st, c"currentgrouptype".as_ptr()),
            CC_CURRENT_IF_LEVEL_CODE => print_esc_cstr_in(st, c"currentiflevel".as_ptr()),
            CC_CURRENT_IF_TYPE_CODE => print_esc_cstr_in(st, c"currentiftype".as_ptr()),
            CC_CURRENT_IF_BRANCH_CODE => print_esc_cstr_in(st, c"currentifbranch".as_ptr()),
            CC_FONT_CHAR_WD_CODE => print_esc_cstr_in(st, c"fontcharwd".as_ptr()),
            CC_FONT_CHAR_HT_CODE => print_esc_cstr_in(st, c"fontcharht".as_ptr()),
            CC_FONT_CHAR_DP_CODE => print_esc_cstr_in(st, c"fontchardp".as_ptr()),
            CC_FONT_CHAR_IC_CODE => print_esc_cstr_in(st, c"fontcharic".as_ptr()),
            CC_PAR_SHAPE_LENGTH_CODE => print_esc_cstr_in(st, c"parshapelength".as_ptr()),
            CC_PAR_SHAPE_INDENT_CODE => print_esc_cstr_in(st, c"parshapeindent".as_ptr()),
            CC_PAR_SHAPE_DIMEN_CODE => print_esc_cstr_in(st, c"parshapedimen".as_ptr()),
            x if x == CC_ETEX_EXPR - CC_INT_VAL + CC_INT_VAL => {
                print_esc_cstr_in(st, c"numexpr".as_ptr())
            }
            x if x == CC_ETEX_EXPR - CC_INT_VAL + CC_DIMEN_VAL => {
                print_esc_cstr_in(st, c"dimexpr".as_ptr())
            }
            x if x == CC_ETEX_EXPR - CC_INT_VAL + CC_GLUE_VAL => {
                print_esc_cstr_in(st, c"glueexpr".as_ptr())
            }
            x if x == CC_ETEX_EXPR - CC_INT_VAL + CC_MU_VAL => {
                print_esc_cstr_in(st, c"muexpr".as_ptr())
            }
            CC_GLUE_STRETCH_ORDER_CODE => print_esc_cstr_in(st, c"gluestretchorder".as_ptr()),
            CC_GLUE_SHRINK_ORDER_CODE => print_esc_cstr_in(st, c"glueshrinkorder".as_ptr()),
            CC_GLUE_STRETCH_CODE => print_esc_cstr_in(st, c"gluestretch".as_ptr()),
            CC_GLUE_SHRINK_CODE => print_esc_cstr_in(st, c"glueshrink".as_ptr()),
            CC_MU_TO_GLUE_CODE => print_esc_cstr_in(st, c"mutoglue".as_ptr()),
            CC_GLUE_TO_MU_CODE => print_esc_cstr_in(st, c"gluetomu".as_ptr()),
            CC_PDF_LAST_X_POS_CODE => print_esc_cstr_in(st, c"pdflastxpos".as_ptr()),
            CC_PDF_LAST_Y_POS_CODE => print_esc_cstr_in(st, c"pdflastypos".as_ptr()),
            CC_ELAPSED_TIME_CODE => print_esc_cstr_in(st, c"elapsedtime".as_ptr()),
            CC_PDF_SHELL_ESCAPE_CODE => print_esc_cstr_in(st, c"shellescape".as_ptr()),
            CC_RANDOM_SEED_CODE => print_esc_cstr_in(st, c"randomseed".as_ptr()),
            _ => print_esc_cstr_in(st, c"badness".as_ptr()),
        },
        CMD_CONVERT => match chr_code {
            CC_NUMBER_CODE => print_esc_cstr_in(st, c"number".as_ptr()),
            CC_ROMAN_NUMERAL_CODE => print_esc_cstr_in(st, c"romannumeral".as_ptr()),
            CC_STRING_CODE => print_esc_cstr_in(st, c"string".as_ptr()),
            CC_MEANING_CODE => print_esc_cstr_in(st, c"meaning".as_ptr()),
            CC_FONT_NAME_CODE => print_esc_cstr_in(st, c"fontname".as_ptr()),
            CC_ETEX_REVISION_CODE => print_esc_cstr_in(st, c"eTeXrevision".as_ptr()),
            CC_EXPANDED_CODE => print_esc_cstr_in(st, c"expanded".as_ptr()),
            CC_LEFT_MARGIN_KERN_CODE => print_esc_cstr_in(st, c"leftmarginkern".as_ptr()),
            CC_RIGHT_MARGIN_KERN_CODE => print_esc_cstr_in(st, c"rightmarginkern".as_ptr()),
            CC_PDF_CREATION_DATE_CODE => print_esc_cstr_in(st, c"creationdate".as_ptr()),
            CC_PDF_FILE_MOD_DATE_CODE => print_esc_cstr_in(st, c"filemoddate".as_ptr()),
            CC_PDF_FILE_SIZE_CODE => print_esc_cstr_in(st, c"filesize".as_ptr()),
            CC_PDF_MDFIVE_SUM_CODE => print_esc_cstr_in(st, c"mdfivesum".as_ptr()),
            CC_PDF_FILE_DUMP_CODE => print_esc_cstr_in(st, c"filedump".as_ptr()),
            CC_PDF_STRCMP_CODE => print_esc_cstr_in(st, c"strcmp".as_ptr()),
            CC_UNIFORM_DEVIATE_CODE => print_esc_cstr_in(st, c"uniformdeviate".as_ptr()),
            CC_NORMAL_DEVIATE_CODE => print_esc_cstr_in(st, c"normaldeviate".as_ptr()),
            CC_XETEX_REVISION_CODE => print_esc_cstr_in(st, c"XeTeXrevision".as_ptr()),
            CC_XETEX_VARIATION_NAME_CODE => print_esc_cstr_in(st, c"XeTeXvariationname".as_ptr()),
            CC_XETEX_FEATURE_NAME_CODE => print_esc_cstr_in(st, c"XeTeXfeaturename".as_ptr()),
            CC_XETEX_SELECTOR_NAME_CODE => print_esc_cstr_in(st, c"XeTeXselectorname".as_ptr()),
            CC_XETEX_GLYPH_NAME_CODE => print_esc_cstr_in(st, c"XeTeXglyphname".as_ptr()),
            CC_XETEX_UCHAR_CODE => print_esc_cstr_in(st, c"Uchar".as_ptr()),
            CC_XETEX_UCHARCAT_CODE => print_esc_cstr_in(st, c"Ucharcat".as_ptr()),
            _ => print_esc_cstr_in(st, c"jobname".as_ptr()),
        },
        CMD_IF_TEST => {
            if chr_code >= CC_UNLESS_CODE {
                print_esc_cstr_in(st, c"unless".as_ptr());
            }
            match chr_code % CC_UNLESS_CODE {
                CC_IF_CAT_CODE => print_esc_cstr_in(st, c"ifcat".as_ptr()),
                CC_IF_INT_CODE => print_esc_cstr_in(st, c"ifnum".as_ptr()),
                CC_IF_DIM_CODE => print_esc_cstr_in(st, c"ifdim".as_ptr()),
                CC_IF_ODD_CODE => print_esc_cstr_in(st, c"ifodd".as_ptr()),
                CC_IF_VMODE_CODE => print_esc_cstr_in(st, c"ifvmode".as_ptr()),
                CC_IF_HMODE_CODE => print_esc_cstr_in(st, c"ifhmode".as_ptr()),
                CC_IF_MMODE_CODE => print_esc_cstr_in(st, c"ifmmode".as_ptr()),
                CC_IF_INNER_CODE => print_esc_cstr_in(st, c"ifinner".as_ptr()),
                CC_IF_VOID_CODE => print_esc_cstr_in(st, c"ifvoid".as_ptr()),
                CC_IF_HBOX_CODE => print_esc_cstr_in(st, c"ifhbox".as_ptr()),
                CC_IF_VBOX_CODE => print_esc_cstr_in(st, c"ifvbox".as_ptr()),
                CC_IFX_CODE => print_esc_cstr_in(st, c"ifx".as_ptr()),
                CC_IF_EOF_CODE => print_esc_cstr_in(st, c"ifeof".as_ptr()),
                CC_IF_TRUE_CODE => print_esc_cstr_in(st, c"iftrue".as_ptr()),
                CC_IF_FALSE_CODE => print_esc_cstr_in(st, c"iffalse".as_ptr()),
                CC_IF_CASE_CODE => print_esc_cstr_in(st, c"ifcase".as_ptr()),
                CC_IF_PRIMITIVE_CODE => print_esc_cstr_in(st, c"ifprimitive".as_ptr()),
                CC_IF_DEF_CODE => print_esc_cstr_in(st, c"ifdefined".as_ptr()),
                CC_IF_CS_CODE => print_esc_cstr_in(st, c"ifcsname".as_ptr()),
                CC_IF_FONT_CHAR_CODE => print_esc_cstr_in(st, c"iffontchar".as_ptr()),
                CC_IF_IN_CSNAME_CODE => print_esc_cstr_in(st, c"ifincsname".as_ptr()),
                _ => print_esc_cstr_in(st, c"if".as_ptr()),
            }
        }
        CMD_FI_OR_ELSE => {
            if chr_code == CC_FI_CODE {
                print_esc_cstr_in(st, c"fi".as_ptr());
            } else if chr_code == CC_OR_CODE {
                print_esc_cstr_in(st, c"or".as_ptr());
            } else {
                print_esc_cstr_in(st, c"else".as_ptr());
            }
        }
        CMD_TAB_MARK => {
            if chr_code == CC_SPAN_CODE {
                print_esc_cstr_in(st, c"span".as_ptr());
            } else {
                print_cstr_in(st, c"alignment tab character ".as_ptr());
                print_char_or_wide(st, chr_code);
            }
        }
        CMD_CAR_RET => {
            if chr_code == CC_CR_CODE {
                print_esc_cstr_in(st, c"cr".as_ptr());
            } else {
                print_esc_cstr_in(st, c"crcr".as_ptr());
            }
        }
        CMD_SET_PAGE_DIMEN => match chr_code {
            0 => print_esc_cstr_in(st, c"pagegoal".as_ptr()),
            1 => print_esc_cstr_in(st, c"pagetotal".as_ptr()),
            2 => print_esc_cstr_in(st, c"pagestretch".as_ptr()),
            3 => print_esc_cstr_in(st, c"pagefilstretch".as_ptr()),
            4 => print_esc_cstr_in(st, c"pagefillstretch".as_ptr()),
            5 => print_esc_cstr_in(st, c"pagefilllstretch".as_ptr()),
            6 => print_esc_cstr_in(st, c"pageshrink".as_ptr()),
            _ => print_esc_cstr_in(st, c"pagedepth".as_ptr()),
        },
        CMD_STOP => {
            if chr_code == 1 {
                print_esc_cstr_in(st, c"dump".as_ptr());
            } else {
                print_esc_cstr_in(st, c"end".as_ptr());
            }
        }
        CMD_HSKIP => match chr_code {
            CC_SKIP_CODE => print_esc_cstr_in(st, c"hskip".as_ptr()),
            CC_FIL_CODE => print_esc_cstr_in(st, c"hfil".as_ptr()),
            CC_FILL_CODE => print_esc_cstr_in(st, c"hfill".as_ptr()),
            CC_SS_CODE => print_esc_cstr_in(st, c"hss".as_ptr()),
            _ => print_esc_cstr_in(st, c"hfilneg".as_ptr()),
        },
        CMD_VSKIP => match chr_code {
            CC_SKIP_CODE => print_esc_cstr_in(st, c"vskip".as_ptr()),
            CC_FIL_CODE => print_esc_cstr_in(st, c"vfil".as_ptr()),
            CC_FILL_CODE => print_esc_cstr_in(st, c"vfill".as_ptr()),
            CC_SS_CODE => print_esc_cstr_in(st, c"vss".as_ptr()),
            _ => print_esc_cstr_in(st, c"vfilneg".as_ptr()),
        },
        CMD_MSKIP => print_esc_cstr_in(st, c"mskip".as_ptr()),
        CMD_KERN => print_esc_cstr_in(st, c"kern".as_ptr()),
        CMD_MKERN => print_esc_cstr_in(st, c"mkern".as_ptr()),
        CMD_HMOVE => {
            if chr_code == 1 {
                print_esc_cstr_in(st, c"moveleft".as_ptr());
            } else {
                print_esc_cstr_in(st, c"moveright".as_ptr());
            }
        }
        CMD_VMOVE => {
            if chr_code == 1 {
                print_esc_cstr_in(st, c"raise".as_ptr());
            } else {
                print_esc_cstr_in(st, c"lower".as_ptr());
            }
        }
        CMD_MAKE_BOX => match chr_code {
            CC_BOX_CODE => print_esc_cstr_in(st, c"box".as_ptr()),
            CC_COPY_CODE => print_esc_cstr_in(st, c"copy".as_ptr()),
            CC_LAST_BOX_CODE => print_esc_cstr_in(st, c"lastbox".as_ptr()),
            CC_VSPLIT_CODE => print_esc_cstr_in(st, c"vsplit".as_ptr()),
            CC_VTOP_CODE => print_esc_cstr_in(st, c"vtop".as_ptr()),
            x if x == CC_VTOP_CODE + CC_VMODE => print_esc_cstr_in(st, c"vbox".as_ptr()),
            _ => print_esc_cstr_in(st, c"hbox".as_ptr()),
        },
        CMD_LEADER_SHIP => {
            if chr_code == CC_A_LEADERS {
                print_esc_cstr_in(st, c"leaders".as_ptr());
            } else if chr_code == CC_C_LEADERS {
                print_esc_cstr_in(st, c"cleaders".as_ptr());
            } else if chr_code == CC_X_LEADERS {
                print_esc_cstr_in(st, c"xleaders".as_ptr());
            } else {
                print_esc_cstr_in(st, c"shipout".as_ptr());
            }
        }
        CMD_START_PAR => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"noindent".as_ptr());
            } else {
                print_esc_cstr_in(st, c"indent".as_ptr());
            }
        }
        CMD_REMOVE_ITEM => {
            if chr_code == CC_GLUE_NODE {
                print_esc_cstr_in(st, c"unskip".as_ptr());
            } else if chr_code == CC_KERN_NODE {
                print_esc_cstr_in(st, c"unkern".as_ptr());
            } else {
                print_esc_cstr_in(st, c"unpenalty".as_ptr());
            }
        }
        CMD_UN_HBOX => {
            if chr_code == CC_COPY_CODE {
                print_esc_cstr_in(st, c"unhcopy".as_ptr());
            } else {
                print_esc_cstr_in(st, c"unhbox".as_ptr());
            }
        }
        CMD_UN_VBOX => {
            if chr_code == CC_COPY_CODE {
                print_esc_cstr_in(st, c"unvcopy".as_ptr());
            } else if chr_code == CC_LAST_BOX_CODE {
                print_esc_cstr_in(st, c"pagediscards".as_ptr());
            } else if chr_code == CC_VSPLIT_CODE2 {
                print_esc_cstr_in(st, c"splitdiscards".as_ptr());
            } else {
                print_esc_cstr_in(st, c"unvbox".as_ptr());
            }
        }
        CMD_DISCRETIONARY => {
            if chr_code == 1 {
                print_esc_in(st, '-' as i32);
            } else {
                print_esc_cstr_in(st, c"discretionary".as_ptr());
            }
        }
        CMD_EQ_NO => {
            if chr_code == 1 {
                print_esc_cstr_in(st, c"leqno".as_ptr());
            } else {
                print_esc_cstr_in(st, c"eqno".as_ptr());
            }
        }
        CMD_MATH_COMP => match chr_code {
            CC_ORD_NOAD => print_esc_cstr_in(st, c"mathord".as_ptr()),
            CC_OP_NOAD => print_esc_cstr_in(st, c"mathop".as_ptr()),
            CC_BIN_NOAD => print_esc_cstr_in(st, c"mathbin".as_ptr()),
            CC_REL_NOAD => print_esc_cstr_in(st, c"mathrel".as_ptr()),
            CC_OPEN_NOAD => print_esc_cstr_in(st, c"mathopen".as_ptr()),
            CC_CLOSE_NOAD => print_esc_cstr_in(st, c"mathclose".as_ptr()),
            CC_PUNCT_NOAD => print_esc_cstr_in(st, c"mathpunct".as_ptr()),
            CC_INNER_NOAD => print_esc_cstr_in(st, c"mathinner".as_ptr()),
            CC_UNDER_NOAD => print_esc_cstr_in(st, c"underline".as_ptr()),
            _ => print_esc_cstr_in(st, c"overline".as_ptr()),
        },
        CMD_LIMIT_SWITCH => {
            if chr_code == CC_LIMITS {
                print_esc_cstr_in(st, c"limits".as_ptr());
            } else if chr_code == CC_NO_LIMITS {
                print_esc_cstr_in(st, c"nolimits".as_ptr());
            } else {
                print_esc_cstr_in(st, c"displaylimits".as_ptr());
            }
        }
        CMD_MATH_STYLE => print_style_in(st, chr_code),
        CMD_ABOVE => match chr_code {
            CC_OVER_CODE => print_esc_cstr_in(st, c"over".as_ptr()),
            CC_ATOP_CODE => print_esc_cstr_in(st, c"atop".as_ptr()),
            x if x == CC_DELIMITED_CODE + CC_ABOVE_CODE => {
                print_esc_cstr_in(st, c"abovewithdelims".as_ptr())
            }
            x if x == CC_DELIMITED_CODE + CC_OVER_CODE => {
                print_esc_cstr_in(st, c"overwithdelims".as_ptr())
            }
            x if x == CC_DELIMITED_CODE + CC_ATOP_CODE => {
                print_esc_cstr_in(st, c"atopwithdelims".as_ptr())
            }
            _ => print_esc_cstr_in(st, c"above".as_ptr()),
        },
        CMD_LEFT_RIGHT => {
            if chr_code == CC_LEFT_NOAD {
                print_esc_cstr_in(st, c"left".as_ptr());
            } else if chr_code == CC_MIDDLE_NOAD {
                print_esc_cstr_in(st, c"middle".as_ptr());
            } else {
                print_esc_cstr_in(st, c"right".as_ptr());
            }
        }
        CMD_PREFIX => {
            if chr_code == 1 {
                print_esc_cstr_in(st, c"long".as_ptr());
            } else if chr_code == 2 {
                print_esc_cstr_in(st, c"outer".as_ptr());
            } else if chr_code == 8 {
                print_esc_cstr_in(st, c"protected".as_ptr());
            } else {
                print_esc_cstr_in(st, c"global".as_ptr());
            }
        }
        CMD_DEF => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"def".as_ptr());
            } else if chr_code == 1 {
                print_esc_cstr_in(st, c"gdef".as_ptr());
            } else if chr_code == 2 {
                print_esc_cstr_in(st, c"edef".as_ptr());
            } else {
                print_esc_cstr_in(st, c"xdef".as_ptr());
            }
        }
        CMD_LET => {
            if chr_code != CC_NORMAL {
                print_esc_cstr_in(st, c"futurelet".as_ptr());
            } else {
                print_esc_cstr_in(st, c"let".as_ptr());
            }
        }
        CMD_SHORTHAND_DEF => match chr_code {
            CC_CHAR_DEF_CODE => print_esc_cstr_in(st, c"chardef".as_ptr()),
            CC_MATH_CHAR_DEF_CODE => print_esc_cstr_in(st, c"mathchardef".as_ptr()),
            CC_XETEX_MATH_CHAR_DEF_CODE => print_esc_cstr_in(st, c"Umathchardef".as_ptr()),
            CC_XETEX_MATH_CHAR_NUM_DEF_CODE => print_esc_cstr_in(st, c"Umathcharnumdef".as_ptr()),
            CC_COUNT_DEF_CODE => print_esc_cstr_in(st, c"countdef".as_ptr()),
            CC_DIMEN_DEF_CODE => print_esc_cstr_in(st, c"dimendef".as_ptr()),
            CC_SKIP_DEF_CODE => print_esc_cstr_in(st, c"skipdef".as_ptr()),
            CC_MU_SKIP_DEF_CODE => print_esc_cstr_in(st, c"muskipdef".as_ptr()),
            CC_CHAR_SUB_DEF_CODE => print_esc_cstr_in(st, c"charsubdef".as_ptr()),
            _ => print_esc_cstr_in(st, c"toksdef".as_ptr()),
        },
        CMD_CHAR_GIVEN => {
            print_esc_cstr_in(st, c"char".as_ptr());
            print_hex_in(st, chr_code);
        }
        CMD_MATH_GIVEN => {
            print_esc_cstr_in(st, c"mathchar".as_ptr());
            print_hex_in(st, chr_code);
        }
        CMD_XETEX_MATH_GIVEN => {
            print_esc_cstr_in(st, c"Umathchar".as_ptr());
            print_hex_in(st, ((chr_code as u32 >> 21) & 0x07) as i32); // math_class
            print_hex_in(st, ((chr_code as u32 >> 24) & 0xFF) as i32); // math_fam
            print_hex_in(st, (chr_code as u32 & 0x1FFFFF) as i32); // math_char
        }
        CMD_DEF_CODE => {
            if chr_code == CC_LC_CODE_BASE {
                print_esc_cstr_in(st, c"lccode".as_ptr());
            } else if chr_code == CC_UC_CODE_BASE {
                print_esc_cstr_in(st, c"uccode".as_ptr());
            } else if chr_code == CC_SF_CODE_BASE {
                print_esc_cstr_in(st, c"sfcode".as_ptr());
            } else if chr_code == CC_MATH_CODE_BASE {
                print_esc_cstr_in(st, c"mathcode".as_ptr());
            } else if chr_code == CC_DEL_CODE_BASE {
                print_esc_cstr_in(st, c"delcode".as_ptr());
            } else {
                print_esc_cstr_in(st, c"catcode".as_ptr());
            }
        }
        CMD_XETEX_DEF_CODE => {
            if chr_code == CC_SF_CODE_BASE {
                print_esc_cstr_in(st, c"XeTeXcharclass".as_ptr());
            } else if chr_code == CC_MATH_CODE_BASE {
                print_esc_cstr_in(st, c"Umathcodenum".as_ptr());
            } else if chr_code == CC_MATH_CODE_BASE + 1 {
                print_esc_cstr_in(st, c"Umathcode".as_ptr());
            } else if chr_code == CC_DEL_CODE_BASE {
                print_esc_cstr_in(st, c"Udelcodenum".as_ptr());
            } else {
                print_esc_cstr_in(st, c"Udelcode".as_ptr());
            }
        }
        CMD_DEF_FAMILY => print_size_in(st, chr_code - CC_MATH_FONT_BASE),
        CMD_HYPH_DATA => {
            if chr_code == 1 {
                print_esc_cstr_in(st, c"patterns".as_ptr());
            } else {
                print_esc_cstr_in(st, c"hyphenation".as_ptr());
            }
        }
        CMD_ASSIGN_FONT_INT => match chr_code {
            0 => print_esc_cstr_in(st, c"hyphenchar".as_ptr()),
            1 => print_esc_cstr_in(st, c"skewchar".as_ptr()),
            CC_LP_CODE_BASE => print_esc_cstr_in(st, c"lpcode".as_ptr()),
            CC_RP_CODE_BASE => print_esc_cstr_in(st, c"rpcode".as_ptr()),
            _ => {}
        },
        CMD_SET_FONT => {
            print_cstr_in(st, c"select font ".as_ptr());
            let font_name_str = st.fonts.font_name[(chr_code) as usize];
            let fa = st.fonts.font_area[(chr_code) as usize];
            if fa == CC_AAT_FONT_FLAG || fa == CC_OTGR_FONT_FLAG {
                let len = crate::string_pool::length(&st.strings, font_name_str);
                let mut quote_char = b'"' as i32;
                for n in 0..len {
                    if st.strings.str_pool
                        [(st.strings.str_start[(font_name_str - 65536) as usize] + n) as usize]
                        == b'"' as u16
                    {
                        quote_char = b'\'' as i32;
                    }
                }
                print_char_in(st, quote_char);
                print_in(st, font_name_str);
                print_char_in(st, quote_char);
            } else {
                print_in(st, font_name_str);
            }
            if st.fonts.font_size[(chr_code) as usize] != st.fonts.font_dsize[(chr_code) as usize] {
                print_cstr_in(st, c" at ".as_ptr());
                print_scaled_in(st, st.fonts.font_size[(chr_code) as usize]);
                print_cstr_in(st, c"pt".as_ptr());
            }
        }
        CMD_SET_INTERACTION => match chr_code {
            CC_BATCH_MODE => print_esc_cstr_in(st, c"batchmode".as_ptr()),
            CC_NONSTOP_MODE => print_esc_cstr_in(st, c"nonstopmode".as_ptr()),
            CC_SCROLL_MODE => print_esc_cstr_in(st, c"scrollmode".as_ptr()),
            _ => print_esc_cstr_in(st, c"errorstopmode".as_ptr()),
        },
        CMD_IN_STREAM => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"closein".as_ptr());
            } else {
                print_esc_cstr_in(st, c"openin".as_ptr());
            }
        }
        CMD_MESSAGE => {
            if chr_code == 0 {
                print_esc_cstr_in(st, c"message".as_ptr());
            } else {
                print_esc_cstr_in(st, c"errmessage".as_ptr());
            }
        }
        CMD_CASE_SHIFT => {
            if chr_code == CC_LC_CODE_BASE {
                print_esc_cstr_in(st, c"lowercase".as_ptr());
            } else {
                print_esc_cstr_in(st, c"uppercase".as_ptr());
            }
        }
        CMD_XRAY => match chr_code {
            CC_SHOW_BOX_CODE => print_esc_cstr_in(st, c"showbox".as_ptr()),
            CC_SHOW_THE_CODE => print_esc_cstr_in(st, c"showthe".as_ptr()),
            CC_SHOW_LISTS => print_esc_cstr_in(st, c"showlists".as_ptr()),
            CC_SHOW_GROUPS => print_esc_cstr_in(st, c"showgroups".as_ptr()),
            CC_SHOW_TOKENS => print_esc_cstr_in(st, c"showtokens".as_ptr()),
            CC_SHOW_IFS => print_esc_cstr_in(st, c"showifs".as_ptr()),
            _ => print_esc_cstr_in(st, c"show".as_ptr()),
        },
        CMD_UNDEFINED_CS => print_cstr_in(st, c"undefined".as_ptr()),
        CMD_CALL | CMD_LONG_CALL | CMD_OUTER_CALL | CMD_LONG_OUTER_CALL => {
            let mut n = cmd as i32 - CMD_CALL as i32;
            if st.mem.b32s0(st.mem.b32s1(chr_code)) == CC_PROTECTED_TOKEN {
                n += 4;
            }
            if n / 4 % 2 != 0 {
                print_esc_cstr_in(st, c"protected".as_ptr());
            }
            if n % 2 != 0 {
                print_esc_cstr_in(st, c"long".as_ptr());
            }
            if n / 2 % 2 != 0 {
                print_esc_cstr_in(st, c"outer".as_ptr());
            }
            if n > 0 {
                print_char_in(st, ' ' as i32);
            }
            print_cstr_in(st, c"macro".as_ptr());
        }
        CMD_END_TEMPLATE => print_esc_cstr_in(st, c"outer endtemplate".as_ptr()),
        CMD_EXTENSION => match chr_code {
            CC_OPEN_NODE => print_esc_cstr_in(st, c"openout".as_ptr()),
            CC_WRITE_NODE => print_esc_cstr_in(st, c"write".as_ptr()),
            CC_CLOSE_NODE => print_esc_cstr_in(st, c"closeout".as_ptr()),
            CC_SPECIAL_NODE => print_esc_cstr_in(st, c"special".as_ptr()),
            CC_IMMEDIATE_CODE => print_esc_cstr_in(st, c"immediate".as_ptr()),
            CC_SET_LANGUAGE_CODE => print_esc_cstr_in(st, c"setlanguage".as_ptr()),
            CC_PDF_SAVE_POS_NODE => print_esc_cstr_in(st, c"pdfsavepos".as_ptr()),
            CC_RESET_TIMER_CODE => print_esc_cstr_in(st, c"resettimer".as_ptr()),
            CC_SET_RANDOM_SEED_CODE => print_esc_cstr_in(st, c"setrandomseed".as_ptr()),
            CC_PIC_FILE_CODE => print_esc_cstr_in(st, c"XeTeXpicfile".as_ptr()),
            CC_PDF_FILE_CODE => print_esc_cstr_in(st, c"XeTeXpdffile".as_ptr()),
            CC_GLYPH_CODE => print_esc_cstr_in(st, c"XeTeXglyph".as_ptr()),
            CC_XETEX_LINEBREAK_LOCALE_EXTENSION_CODE => {
                print_esc_cstr_in(st, c"XeTeXlinebreaklocale".as_ptr())
            }
            CC_XETEX_INPUT_ENCODING_EXTENSION_CODE => {
                print_esc_cstr_in(st, c"XeTeXinputencoding".as_ptr())
            }
            CC_XETEX_DEFAULT_ENCODING_EXTENSION_CODE => {
                print_esc_cstr_in(st, c"XeTeXdefaultencoding".as_ptr())
            }
            _ => print_cstr_in(st, c"[unknown extension!]".as_ptr()),
        },
        _ => print_cstr_in(st, c"[unknown command code!]".as_ptr()),
    }
}

// ---------------------------------------------------------------------------
// print_meaning
// ---------------------------------------------------------------------------

/// Print the meaning of the current token (`cur_cmd`/`cur_chr`).
///
/// translated from xetex-xetex0.c:3383 (print_meaning)
pub unsafe fn print_meaning() {
    crate::state::with_state_mut(|st| print_meaning_in(st));
}

/// `&mut EngineState` core of [`print_meaning`].
pub(crate) fn print_meaning_in(st: &mut EngineState) {
    print_cmd_chr_in(st, cur_cmd() as u16, cur_chr());
    if cur_cmd() as u16 >= CALL {
        print_char_in(st, ':' as i32);
        print_ln_in(st);
        token_show_in(st, cur_chr());
    } else if cur_cmd() as u16 == TOP_BOT_MARK && (cur_chr() as usize) < 5 {
        print_char_in(st, ':' as i32);
        print_ln_in(st);
        token_show_in(st, st.scanner.cur_mark[cur_chr() as usize]);
    }
}

// ---------------------------------------------------------------------------
// group_warning
// ---------------------------------------------------------------------------

/// Warn if a group spans file boundaries (\tracingcommands > 0).
///
/// translated from xetex-xetex0.c:2961 (group_warning)
pub unsafe fn group_warning() {
    set_base_ptr(input_ptr());
    set_input_stack(base_ptr(), cur_input());
    let mut i = in_open();
    let mut w = false;
    while grp_stack(i) == cur_boundary() && i > 0 {
        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_NESTING) > 0 {
            let mut bp = base_ptr();
            while input_stack(bp).state == TOKEN_LIST || input_stack(bp).index as i32 > i {
                bp -= 1;
                set_base_ptr(bp);
            }
            if input_stack(base_ptr()).name > 17 {
                w = true;
            }
        }
        set_grp_stack(i, save_stack_b32s1(save_ptr()));
        i -= 1;
    }
    if w {
        print_nl_cstr(c"Warning: ".as_ptr());
        diagnostic_begin_capture_warning_here();
        print_cstr(c"end of ".as_ptr());
        print_group(true);
        print_cstr(c" of a different file".as_ptr());
        print_ln();
        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_NESTING) > 1 {
            show_context();
        }
        if history() == HISTORY_SPOTLESS {
            set_history(HISTORY_WARNING_ISSUED);
        }
        capture_to_diagnostic(std::ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// if_warning
// ---------------------------------------------------------------------------

/// Warn if an \if spans file boundaries.
///
/// translated from xetex-xetex0.c:2998 (if_warning)
pub unsafe fn if_warning() {
    set_base_ptr(input_ptr());
    set_input_stack(base_ptr(), cur_input());
    let mut i = in_open();
    let mut w = false;
    while if_stack(i) == cond_ptr() {
        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_NESTING) > 0 {
            let mut bp = base_ptr();
            while input_stack(bp).state == TOKEN_LIST || input_stack(bp).index as i32 > i {
                bp -= 1;
                set_base_ptr(bp);
            }
            if input_stack(base_ptr()).name > 17 {
                w = true;
            }
        }
        set_if_stack(i, mem_b32s1(cond_ptr()));
        i -= 1;
    }
    if w {
        print_nl_cstr(c"Warning: ".as_ptr());
        diagnostic_begin_capture_warning_here();
        print_cstr(c"end of ".as_ptr());
        print_cmd_chr(IF_TEST, cur_if() as i32);
        if if_line() != 0 {
            print_cstr(c" entered on line ".as_ptr());
            print_int(if_line());
        }
        print_cstr(c" of a different file".as_ptr());
        print_ln();
        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_NESTING) > 1 {
            show_context();
        }
        capture_to_diagnostic(std::ptr::null_mut());
        if history() == HISTORY_SPOTLESS {
            set_history(HISTORY_WARNING_ISSUED);
        }
    }
}

// ---------------------------------------------------------------------------
// file_warning
// ---------------------------------------------------------------------------

/// Warn about unclosed groups and \if's when reaching end of file.
///
/// translated from xetex-xetex0.c:3039 (file_warning)
pub unsafe fn file_warning() {
    // Save and restore save-stack / level / group state while scanning.
    let p = save_ptr();
    let l = cur_level();
    let c = cur_group();
    // Use cur_boundary() as the traversal pointer for groups.
    set_save_ptr(cur_boundary());
    while grp_stack(in_open()) != save_ptr() {
        set_cur_level(cur_level() - 1);
        print_nl_cstr(c"Warning: ".as_ptr());
        diagnostic_begin_capture_warning_here();
        print_cstr(c"end of file when ".as_ptr());
        print_group(true);
        print_cstr(c" is incomplete".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_cur_group(save_stack_b16s0(save_ptr()) as u8);
        set_save_ptr(save_stack_b32s1(save_ptr()));
    }
    set_save_ptr(p);
    set_cur_level(l);
    set_cur_group(c);

    // Traverse cond_ptr chain.
    let cp = cond_ptr();
    let il = if_limit();
    let ci = cur_if();
    let iln = if_line();
    while if_stack(in_open()) != cond_ptr() {
        print_nl_cstr(c"Warning: ".as_ptr());
        diagnostic_begin_capture_warning_here();
        print_cstr(c"end of file when ".as_ptr());
        print_cmd_chr(IF_TEST, cur_if() as i32);
        if if_limit() == FI_CODE {
            print_esc_cstr(c"else".as_ptr());
        }
        if if_line() != 0 {
            print_cstr(c" entered on line ".as_ptr());
            print_int(if_line());
        }
        print_cstr(c" is incomplete".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_if_line(mem_b32s1(cond_ptr() + 1));
        set_cur_if(mem_b16s0(cond_ptr()) as i16);
        set_if_limit(mem_b16s1(cond_ptr()) as u8);
        set_cond_ptr(mem_b32s1(cond_ptr()));
    }
    set_cond_ptr(cp);
    set_if_limit(il);
    set_cur_if(ci);
    set_if_line(iln);

    print_ln();
    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_NESTING) > 1 {
        diagnostic_begin_capture_warning_here();
        show_context();
        capture_to_diagnostic(std::ptr::null_mut());
    }
    if history() == HISTORY_SPOTLESS {
        set_history(HISTORY_WARNING_ISSUED);
    }
}

// ---------------------------------------------------------------------------
// show_cur_cmd_chr
// ---------------------------------------------------------------------------

// translated from xetex-xetex0.c:2824 (show_cur_cmd_chr)
pub unsafe fn show_cur_cmd_chr() {
    begin_diagnostic();
    diagnostic_begin_capture_warning_here();
    print_nl(b'{' as i32);
    if cur_list_mode() != shown_mode() {
        print_mode(cur_list_mode() as i32);
        print_cstr(c": ".as_ptr());
        set_shown_mode(cur_list_mode());
    }
    print_cmd_chr(cur_cmd() as u16, cur_chr());
    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_IFS) > 0
        && cur_cmd() >= IF_TEST as u8
        && cur_cmd() <= FI_OR_ELSE
    {
        print_cstr(c": ".as_ptr());
        let (mut n, l) = if cur_cmd() == FI_OR_ELSE {
            print_cmd_chr(IF_TEST, cur_if() as i32);
            print_char(b' ' as i32);
            (0_i32, if_line())
        } else {
            (1_i32, line())
        };
        let mut p = cond_ptr();
        while p != TEX_NULL {
            n += 1;
            p = mem_b32s1(p);
        }
        print_cstr(c"(level ".as_ptr());
        print_int(n);
        print_char(b')' as i32);
        if l != 0 {
            print_cstr(c" entered on line ".as_ptr());
            print_int(l);
        }
    }
    print_char(b'}' as i32);
    capture_to_diagnostic(std::ptr::null_mut());
    end_diagnostic(false);
}

// ---------------------------------------------------------------------------
// change_if_limit
// ---------------------------------------------------------------------------

/// Update the if-limit for a specific conditional pointer.
///
/// translated from xetex-xetex0.c:7889 (change_if_limit)
pub unsafe fn change_if_limit(l: i16, p: i32) {
    if p == cond_ptr() {
        set_if_limit(l as u8);
    } else {
        let mut q = cond_ptr();
        loop {
            if q == crate::mem::TEX_NULL {
                crate::errors::confusion(c"if".as_ptr());
            }
            if mem_b32s1(q) == p {
                set_mem_b16s1(q, l as u16);
                return;
            }
            q = mem_b32s1(q); // LLIST_link(q)
        }
    }
}

extern "C" {}

// ---------------------------------------------------------------------------
// font_feature_warning / font_mapping_warning / graphite_warning
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:8712 (font_feature_warning)
#[no_mangle]
pub unsafe extern "C" fn font_feature_warning(
    feature_name_p: *const libc::c_void,
    feat_len: i32,
    setting_name_p: *const libc::c_void,
    set_len: i32,
) {
    begin_diagnostic();
    diagnostic_begin_capture_warning_here();
    print_nl_cstr(c"Unknown ".as_ptr());
    if set_len > 0 {
        print_cstr(c"selector `".as_ptr());
        print_utf8_str(setting_name_p as *const u8, set_len);
        print_cstr(c"' for ".as_ptr());
    }
    print_cstr(c"feature `".as_ptr());
    print_utf8_str(feature_name_p as *const u8, feat_len);
    print_cstr(c"' in font `".as_ptr());
    let nof = crate::globals::name_of_file();
    let mut i = 0;
    while *nof.offset(i) != 0 {
        print_raw_char(*nof.offset(i) as u16, true);
        i += 1;
    }
    print_cstr(c"'.".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    end_diagnostic(false);
}

/// translated from xetex-xetex0.c:8732 (font_mapping_warning)
pub unsafe fn font_mapping_warning(
    mapping_name_p: *const libc::c_void,
    mapping_name_len: i32,
    warning_type: i32,
) {
    begin_diagnostic();
    diagnostic_begin_capture_warning_here();
    if warning_type == 0 {
        print_nl_cstr(c"Loaded mapping `".as_ptr());
    } else {
        print_nl_cstr(c"Font mapping `".as_ptr());
    }
    print_utf8_str(mapping_name_p as *const u8, mapping_name_len);
    print_cstr(c"' for font `".as_ptr());
    let nof = crate::globals::name_of_file();
    let mut i = 0;
    while *nof.offset(i) != 0 {
        print_raw_char(*nof.offset(i) as u16, true);
        i += 1;
    }
    match warning_type {
        1 => print_cstr(c"' not found.".as_ptr()),
        2 => {
            print_cstr(c"' not usable;".as_ptr());
            print_nl_cstr(c"bad mapping file or incorrect mapping type.".as_ptr());
        }
        _ => print_cstr(c"'.".as_ptr()),
    }
    capture_to_diagnostic(std::ptr::null_mut());
    end_diagnostic(false);
}

// ---------------------------------------------------------------------------
// bad_utf8_warning
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:8970 (bad_utf8_warning)
pub unsafe fn bad_utf8_warning() {
    begin_diagnostic();
    diagnostic_begin_capture_warning_here();
    print_nl_cstr(c"Invalid UTF-8 byte or sequence".as_ptr());
    if crate::globals::cur_input().name == 0 {
        print_cstr(c" in terminal input".as_ptr());
    } else {
        print_cstr(c" at line ".as_ptr());
        print_int(crate::globals::line());
    }
    print_cstr(c" replaced by U+FFFD.".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    end_diagnostic(false);
}

// ---------------------------------------------------------------------------
// show_token_list
// ---------------------------------------------------------------------------

// Token-word layout (xetex-constants.h:347).
const MAX_CHAR_VAL_STL: i32 = 0x200_000; // 1 << 21

// Category-code discriminants used in token words (xetex_format.h:231–250).
const LEFT_BRACE_STL: i32 = 1;
const RIGHT_BRACE_STL: i32 = 2;
const MATH_SHIFT_STL: i32 = 3;
const TAB_MARK_STL: i32 = 4;
const OUT_PARAM_STL: i32 = 5;
const MAC_PARAM_STL: i32 = 6;
const SUP_MARK_STL: i32 = 7;
const SUB_MARK_STL: i32 = 8;
const SPACER_STL: i32 = 10;
const LETTER_STL: i32 = 11;
const OTHER_CHAR_STL: i32 = 12;
const MATCH_STL: i32 = 13;
const END_MATCH_STL: i32 = 14;

/// translated from xetex-xetex0.c:20 (show_token_list)
pub unsafe fn show_token_list(p: i32, q: i32, l: i32) {
    crate::state::with_state_mut(|st| show_token_list_in(st, p, q, l));
}

/// `&mut EngineState` core of [`show_token_list`].
pub(crate) fn show_token_list_in(st: &mut EngineState, mut p: i32, q: i32, l: i32) {
    let mut match_chr: i32 = '#' as i32;
    let mut n: i32 = '0' as i32;
    st.errors.tally = 0;
    while p != crate::mem::TEX_NULL && st.errors.tally < l {
        if p == q {
            st.errors.first_count = st.errors.tally;
            let tc = st.errors.tally + 1 + st.errors.error_line - st.errors.half_error_line;
            st.errors.trick_count = tc.max(st.errors.error_line);
        }
        if p < st.mem.hi_mem_min || p > st.mem.mem_end {
            print_esc_cstr_in(st, c"CLOBBERED.".as_ptr());
            return;
        }
        let w = st.mem.b32s0(p);
        if w >= CS_TOKEN_FLAG {
            print_cs_in(st, w - CS_TOKEN_FLAG);
        } else if w < 0 {
            print_esc_cstr_in(st, c"BAD.".as_ptr());
        } else {
            let m = w / MAX_CHAR_VAL_STL;
            let c = w % MAX_CHAR_VAL_STL;
            match m {
                LEFT_BRACE_STL | RIGHT_BRACE_STL | MATH_SHIFT_STL | TAB_MARK_STL | SUP_MARK_STL
                | SUB_MARK_STL | SPACER_STL | LETTER_STL | OTHER_CHAR_STL => {
                    print_char_in(st, c);
                }
                MAC_PARAM_STL => {
                    print_char_in(st, c);
                    print_char_in(st, c);
                }
                OUT_PARAM_STL => {
                    print_char_in(st, match_chr);
                    if c <= 9 {
                        print_char_in(st, c + 48);
                    } else {
                        print_char_in(st, '!' as i32);
                        return;
                    }
                }
                MATCH_STL => {
                    match_chr = c;
                    print_char_in(st, c);
                    n += 1;
                    print_char_in(st, n);
                    if n > '9' as i32 {
                        return;
                    }
                }
                END_MATCH_STL => {
                    if c == 0 {
                        print_cstr_in(st, c"->".as_ptr());
                    }
                }
                _ => {
                    print_esc_cstr_in(st, c"BAD.".as_ptr());
                }
            }
        }
        p = st.mem.b32s1(p);
    }
    if p != crate::mem::TEX_NULL {
        print_esc_cstr_in(st, c"ETC.".as_ptr());
    }
}

/// `&mut EngineState` core of [`print_totals`].
pub(crate) fn print_totals_in(st: &mut EngineState) {
    print_scaled_in(st, st.shipout.page_so_far[1]);
    if st.shipout.page_so_far[2] != 0 {
        print_cstr_in(st, c" plus ".as_ptr());
        print_scaled_in(st, st.shipout.page_so_far[2]);
        print_cstr_in(st, c"".as_ptr());
    }
    if st.shipout.page_so_far[3] != 0 {
        print_cstr_in(st, c" plus ".as_ptr());
        print_scaled_in(st, st.shipout.page_so_far[3]);
        print_cstr_in(st, c"fil".as_ptr());
    }
    if st.shipout.page_so_far[4] != 0 {
        print_cstr_in(st, c" plus ".as_ptr());
        print_scaled_in(st, st.shipout.page_so_far[4]);
        print_cstr_in(st, c"fill".as_ptr());
    }
    if st.shipout.page_so_far[5] != 0 {
        print_cstr_in(st, c" plus ".as_ptr());
        print_scaled_in(st, st.shipout.page_so_far[5]);
        print_cstr_in(st, c"filll".as_ptr());
    }
    if st.shipout.page_so_far[6] != 0 {
        print_cstr_in(st, c" minus ".as_ptr());
        print_scaled_in(st, st.shipout.page_so_far[6]);
    }
}

// ---- moved verbatim from scan.rs (workstream 02) ----

/// translated from xetex-xetex0.c:9942 (show_info)
pub(crate) fn show_info_in(st: &mut EngineState) {
    let tp = crate::globals::temp_ptr();
    let p = st.mem.b32s0(tp);
    show_node_list_in(st, p);
}
// translated from xetex-xetex0.c:10708 (issue_message)
pub unsafe fn issue_message() {
    let c = cur_chr() as u8;
    set_mem_b32s1(GARBAGE, scan_toks(false, true));
    let old_setting = selector() as u8;
    set_selector(SELECTOR_NEW_STRING);
    crate::output::token_show(def_ref());
    set_selector(old_setting as i32);
    crate::mem::flush_list_global(def_ref());
    if pool_ptr() + 1 > pool_size() {
        crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr());
    }
    let s = crate::string_pool::make_string_global();
    if c == 0 {
        let len = crate::string_pool::length_global(s);
        if term_offset() + len > max_print_line() - 2 {
            crate::output::print_ln();
        } else if term_offset() > 0 || file_offset() > 0 {
            crate::output::print_char(b' ' as i32);
        }
        crate::output::print(s);
        ttstub_output_flush(rust_stdout());
    } else {
        error_here_with_diagnostic(c"".as_ptr());
        crate::output::print(s);
        capture_to_diagnostic(std::ptr::null_mut());
        let err_help = eqtb_b32s1(LOCAL_BASE + LOCAL__ERR_HELP);
        if err_help != crate::mem::TEX_NULL {
            crate::globals::set_use_err_help(true);
        } else if crate::globals::long_help_seen() {
            set_help_ptr(1);
            set_help_line(0, c"(That was another \\errmessage.)".as_ptr());
        } else {
            if crate::globals::interaction() < ERROR_STOP_MODE {
                crate::globals::set_long_help_seen(true);
            }
            set_help_ptr(4);
            set_help_line(
                3,
                c"This error message was generated by an \\errmessage".as_ptr(),
            );
            set_help_line(2, c"command, so I can't give any explicit help.".as_ptr());
            set_help_line(
                1,
                c"Pretend that you're Hercule Poirot: Examine all clues,".as_ptr(),
            );
            set_help_line(0, c"and deduce the truth by order and method.".as_ptr());
        }
        error();
        crate::globals::set_use_err_help(false);
    }
    // pop the temporary string (equivalent to flush_str(s) when s == str_ptr-1)
    set_str_ptr(str_ptr() - 1);
    set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
}
// translated from xetex-xetex0.c:7752 (show_save_groups)
pub unsafe fn show_save_groups() {
    crate::state::with_state_mut(|st| show_save_groups_in(st));
}

/// `&mut EngineState` core of [`show_save_groups`].
pub(crate) fn show_save_groups_in(st: &mut EngineState) {
    const BOTTOM_LEVEL: u8 = 0;
    const SIMPLE_GROUP: u8 = 1;
    const HBOX_GROUP: u8 = 2;
    const ADJUSTED_HBOX_GROUP: u8 = 3;
    const VBOX_GROUP: u8 = 4;
    const VTOP_GROUP: u8 = 5;
    const ALIGN_GROUP: u8 = 6;
    const NO_ALIGN_GROUP: u8 = 7;
    const OUTPUT_GROUP: u8 = 8;
    const MATH_GROUP: u8 = 9;
    const DISC_GROUP: u8 = 10;
    const INSERT_GROUP: u8 = 11;
    const VCENTER_GROUP: u8 = 12;
    const MATH_CHOICE_GROUP: u8 = 13;
    const SEMI_SIMPLE_GROUP: u8 = 14;
    const MATH_SHIFT_GROUP: u8 = 15;
    const MATH_LEFT_GROUP: u8 = 16;
    const EXACTLY: i32 = 0; // xetex-constants.h:256
    const BOX_FLAG: i32 = 0x4000_0000; // xetex-constants.h:372
    const GLOBAL_BOX_FLAG: i32 = 0x4000_8000; // xetex-constants.h:373
    const SHIP_OUT_FLAG: i32 = 0x4001_0000; // xetex-constants.h:374
    const LEADER_FLAG: i32 = 0x4001_0001; // xetex-constants.h:375
    const A_LEADERS: i32 = 100; // xetex_format.h:422
    const HMOVE: u16 = 21; // xetex_format.h:259
    const VMOVE: u16 = 22; // xetex_format.h:260
    const LEADER_SHIP: u16 = 31; // xetex_format.h:269
    const EQ_NO: u16 = 48; // xetex_format.h:286
    const LEFT_NOAD: u16 = 30; // xetex_format.h:415
    const VMODE: i16 = 1;
    const HMODE: i16 = 104;
    const MMODE: i16 = 207;

    // Save and override state for walking the group stack
    let mut p = st.input.nest_ptr;
    st.input.nest[(p) as usize] = crate::globals::cur_list();
    let v = st.save_stack.save_ptr;
    let l = st.save_stack.cur_level;
    let c = st.save_stack.cur_group;
    st.save_stack.save_ptr = st.save_stack.cur_boundary;
    st.save_stack.cur_level = st.save_stack.cur_level - 1;
    let mut a: i8 = 1;

    crate::output::print_nl_cstr_in(st, c"".as_ptr());
    crate::output::print_ln_in(st);

    'outer: loop {
        crate::output::print_nl_cstr_in(st, c"### ".as_ptr());
        crate::output::print_group_in(st, true);

        if st.save_stack.cur_group == BOTTOM_LEVEL {
            break 'outer;
        }

        // C do-while: advance p until we find a non-HMODE nest entry.
        // After the loop, m = the mode that stopped it, p = one below that entry.
        let m: i16;
        loop {
            let cur_m = st.input.nest[(p) as usize].mode;
            if p > 0 {
                p -= 1;
            } else {
                m = VMODE;
                break;
            }
            if cur_m != HMODE {
                m = cur_m;
                break;
            }
        }

        crate::output::print_cstr_in(st, c" (".as_ptr());

        // Determine which continuation block to execute
        // After::Preamble = execute the i-box preamble block, then found1→found2→found
        // After::Found1   = skip preamble, execute found1→found2→found
        // After::Found2   = skip preamble+found1, execute found2→found
        // After::Found    = skip to found only
        enum After {
            Preamble,
            Found1,
            Found2,
            Found,
        }
        let mut s: *const libc::c_char = std::ptr::null();

        let after = match st.save_stack.cur_group {
            SIMPLE_GROUP => {
                p += 1;
                After::Found2
            }
            HBOX_GROUP | ADJUSTED_HBOX_GROUP => {
                s = c"hbox".as_ptr();
                After::Preamble
            }
            VBOX_GROUP => {
                s = c"vbox".as_ptr();
                After::Preamble
            }
            VTOP_GROUP => {
                s = c"vtop".as_ptr();
                After::Preamble
            }
            ALIGN_GROUP => {
                if a == 0 {
                    s = if m == -VMODE {
                        c"halign".as_ptr()
                    } else {
                        c"valign".as_ptr()
                    };
                    a = 1;
                    After::Found1
                } else {
                    if a == 1 {
                        crate::output::print_cstr_in(st, c"align entry".as_ptr());
                    } else {
                        crate::output::print_esc_cstr_in(st, c"cr".as_ptr());
                    }
                    if p >= a as i32 {
                        p -= a as i32;
                    }
                    a = 0;
                    After::Found
                }
            }
            NO_ALIGN_GROUP => {
                p += 1;
                a = -1;
                crate::output::print_esc_cstr_in(st, c"noalign".as_ptr());
                After::Found2
            }
            OUTPUT_GROUP => {
                crate::output::print_esc_cstr_in(st, c"output".as_ptr());
                After::Found
            }
            MATH_GROUP => After::Found2,
            DISC_GROUP | MATH_CHOICE_GROUP => {
                if st.save_stack.cur_group == DISC_GROUP {
                    crate::output::print_esc_cstr_in(st, c"discretionary".as_ptr());
                } else {
                    crate::output::print_esc_cstr_in(st, c"mathchoice".as_ptr());
                }
                let limit = st.save_stack.b32s1(st.save_stack.save_ptr - 2);
                for i in 1..=3_i32 {
                    if i <= limit {
                        crate::output::print_cstr_in(st, c"{}".as_ptr());
                    }
                }
                After::Found2
            }
            INSERT_GROUP => {
                let val = st.save_stack.b32s1(st.save_stack.save_ptr - 2);
                if val == 255 {
                    crate::output::print_esc_cstr_in(st, c"vadjust".as_ptr());
                } else {
                    crate::output::print_esc_cstr_in(st, c"insert".as_ptr());
                    crate::output::print_int_in(st, val);
                }
                After::Found2
            }
            VCENTER_GROUP => {
                s = c"vcenter".as_ptr();
                After::Found1
            }
            SEMI_SIMPLE_GROUP => {
                p += 1;
                crate::output::print_esc_cstr_in(st, c"begingroup".as_ptr());
                After::Found
            }
            MATH_SHIFT_GROUP => {
                // C: if m==MMODE print '$' (fall through to another '$' = "$$")
                //    else if nest[p].mode==MMODE: print EQ_NO, goto found
                //    else: (fall through to '$' + goto found)
                if m == MMODE {
                    crate::output::print_char_in(st, '$' as i32);
                } else if st.input.nest[(p) as usize].mode == MMODE {
                    print_cmd_chr_in(st, EQ_NO, st.save_stack.b32s1(st.save_stack.save_ptr - 2));
                    // goto found (no '$')
                    crate::output::print_char_in(st, ')' as i32);
                    st.save_stack.cur_level = st.save_stack.cur_level - 1;
                    st.save_stack.cur_group =
                        unsafe { st.save_stack.data[(st.save_stack.save_ptr) as usize].b16.s0 }
                            as u8;
                    st.save_stack.save_ptr = st.save_stack.b32s1(st.save_stack.save_ptr);
                    continue 'outer;
                }
                crate::output::print_char_in(st, '$' as i32);
                After::Found
            }
            MATH_LEFT_GROUP => {
                if st.mem.b16s1(st.input.nest[(p + 1) as usize].e_tex_aux) == LEFT_NOAD {
                    crate::output::print_esc_cstr_in(st, c"left".as_ptr());
                } else {
                    crate::output::print_esc_cstr_in(st, c"middle".as_ptr());
                }
                After::Found
            }
            _ => After::Found,
        };

        // found1's preamble: print box context for hbox/vbox/vtop (move/setbox/leaders)
        if matches!(after, After::Preamble) {
            let i = st.save_stack.b32s1(st.save_stack.save_ptr - 4);
            if i != 0 {
                if i < BOX_FLAG {
                    let j = if st.input.nest[(p) as usize].mode.abs() == VMODE {
                        HMOVE
                    } else {
                        VMOVE
                    };
                    if i > 0 {
                        print_cmd_chr_in(st, j, 0);
                    } else {
                        print_cmd_chr_in(st, j, 1);
                    }
                    crate::output::print_scaled_in(st, i.abs());
                    crate::output::print_cstr_in(st, c"pt".as_ptr());
                } else if i < SHIP_OUT_FLAG {
                    let mut ii = i;
                    if ii >= GLOBAL_BOX_FLAG {
                        crate::output::print_esc_cstr_in(st, c"global".as_ptr());
                        ii -= GLOBAL_BOX_FLAG - BOX_FLAG;
                    }
                    crate::output::print_esc_cstr_in(st, c"setbox".as_ptr());
                    crate::output::print_int_in(st, ii - BOX_FLAG);
                    crate::output::print_char_in(st, '=' as i32);
                } else {
                    print_cmd_chr_in(st, LEADER_SHIP, i - (LEADER_FLAG - A_LEADERS));
                }
            }
        }

        // found1: print \s and optional size spec
        if matches!(after, After::Preamble | After::Found1) {
            crate::output::print_esc_cstr_in(st, s);
            let spec = st.save_stack.b32s1(st.save_stack.save_ptr - 2);
            if spec != 0 {
                crate::output::print_char_in(st, ' ' as i32);
                // C: save_stack[save_ptr - 3].b32.s1 == EXACTLY
                if st.save_stack.b32s1(st.save_stack.save_ptr - 3) == EXACTLY {
                    crate::output::print_cstr_in(st, c"to".as_ptr());
                } else {
                    crate::output::print_cstr_in(st, c"spread".as_ptr());
                }
                crate::output::print_scaled_in(st, spec);
                crate::output::print_cstr_in(st, c"pt".as_ptr());
            }
        }

        // found2: print '{'
        if !matches!(after, After::Found) {
            crate::output::print_char_in(st, '{' as i32);
        }

        // found: print ')', advance group-boundary chain
        crate::output::print_char_in(st, ')' as i32);
        st.save_stack.cur_level = st.save_stack.cur_level - 1;
        st.save_stack.cur_group =
            unsafe { st.save_stack.data[(st.save_stack.save_ptr) as usize].b16.s0 } as u8;
        st.save_stack.save_ptr = st.save_stack.b32s1(st.save_stack.save_ptr);
    }

    // done: restore state
    st.save_stack.save_ptr = v;
    st.save_stack.cur_level = l;
    st.save_stack.cur_group = c;
}
// translated from xetex-xetex0.c:10561 (show_whatever)
pub unsafe fn show_whatever() {
    const INT_PAR_TRACING_ONLINE: i32 = 29; // xetex_format.h:33
    const IF_TEST: u16 = 107; // xetex_format.h:349
    const FI_CODE: u8 = 2; // xetex_format.h:537

    crate::output::diagnostic_begin_capture_warning_here();

    let goto_common_ending = match cur_chr() {
        3 => {
            crate::output::begin_diagnostic();
            show_activities();
            false
        }
        1 => {
            scan_register_num();
            let p = if cur_val() < 256 {
                eqtb_b32s1(BOX_BASE + cur_val())
            } else {
                crate::save::find_sa_element_global(4, cur_val(), false);
                if crate::globals::cur_ptr() == crate::mem::TEX_NULL {
                    crate::mem::TEX_NULL
                } else {
                    mem_b32s1(crate::globals::cur_ptr() + 1)
                }
            };
            crate::output::begin_diagnostic();
            crate::output::print_nl_cstr(c"> \\box".as_ptr());
            crate::output::print_int(cur_val());
            crate::output::print_char('=' as i32);
            if p == crate::mem::TEX_NULL {
                crate::output::print_cstr(c"void".as_ptr());
            } else {
                show_box(p);
            }
            false
        }
        0 => {
            get_token();
            crate::output::print_nl_cstr(c"> ".as_ptr());
            if cur_cs() != 0 {
                crate::output::sprint_cs(cur_cs());
                crate::output::print_char('=' as i32);
            }
            crate::output::print_meaning();
            true
        }
        4 => {
            crate::output::begin_diagnostic();
            show_save_groups();
            false
        }
        6 => {
            crate::output::begin_diagnostic();
            crate::output::print_nl_cstr(c"".as_ptr());
            crate::output::print_ln();
            if crate::globals::cond_ptr() == crate::mem::TEX_NULL {
                crate::output::print_nl_cstr(c"### ".as_ptr());
                crate::output::print_cstr(c"no active conditionals".as_ptr());
            } else {
                let mut p = crate::globals::cond_ptr();
                let mut n: i32 = 0;
                loop {
                    n += 1;
                    p = mem_b32s1(p);
                    if p == crate::mem::TEX_NULL {
                        break;
                    }
                }
                p = crate::globals::cond_ptr();
                let mut t = crate::globals::cur_if();
                let mut l = crate::globals::if_line();
                let mut m = crate::globals::if_limit();
                loop {
                    crate::output::print_nl_cstr(c"### level ".as_ptr());
                    crate::output::print_int(n);
                    crate::output::print_cstr(c": ".as_ptr());
                    print_cmd_chr(IF_TEST, t as i32);
                    if m == FI_CODE {
                        crate::output::print_esc_cstr(c"else".as_ptr());
                    }
                    if l != 0 {
                        crate::output::print_cstr(c" entered on line ".as_ptr());
                        crate::output::print_int(l);
                    }
                    n -= 1;
                    t = mem_b16s0(p) as i16;
                    l = mem_b32s1(p + 1);
                    m = mem_b16s1(p) as u8;
                    p = mem_b32s1(p);
                    if p == crate::mem::TEX_NULL {
                        break;
                    }
                }
            }
            false
        }
        _ => {
            the_toks(); // the_toks populates TEMP_HEAD
            crate::output::print_nl_cstr(c"> ".as_ptr());
            crate::output::token_show(TEMP_HEAD);
            crate::mem::flush_list_global(mem_b32s1(TEMP_HEAD));
            true
        }
    };

    if !goto_common_ending {
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::end_diagnostic(true);
        if crate::globals::file_line_error_style_p() != 0 {
            crate::output::print_file_line();
        } else {
            crate::output::print_nl_cstr(c"! ".as_ptr());
        }
        crate::output::print_cstr(c"OK".as_ptr());
        if crate::globals::selector() == SELECTOR_TERM_AND_LOG
            && eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) <= 0
        {
            crate::globals::set_selector(SELECTOR_TERM_ONLY);
            crate::output::print_cstr(c" (see the transcript file)".as_ptr());
            crate::globals::set_selector(SELECTOR_TERM_AND_LOG);
        }
    }

    // common_ending:
    capture_to_diagnostic(std::ptr::null_mut());
    if crate::globals::interaction() < ERROR_STOP_MODE {
        set_help_ptr(0);
        crate::globals::set_error_count(crate::globals::error_count() - 1);
    } else if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) > 0 {
        set_help_ptr(3);
        set_help_line(
            2,
            c"This isn't an error message; I'm just \\showing something.".as_ptr(),
        );
        set_help_line(
            1,
            c"Type `I\\show...' to show more (e.g., \\show\\cs,".as_ptr(),
        );
        set_help_line(
            0,
            c"\\showthe\\count10, \\showbox255, \\showlists).".as_ptr(),
        );
    } else {
        set_help_ptr(5);
        set_help_line(
            4,
            c"This isn't an error message; I'm just \\showing something.".as_ptr(),
        );
        set_help_line(
            3,
            c"Type `I\\show...' to show more (e.g., \\show\\cs,".as_ptr(),
        );
        set_help_line(
            2,
            c"\\showthe\\count10, \\showbox255, \\showlists).".as_ptr(),
        );
        set_help_line(
            1,
            c"And type `I\\tracingonline=1\\show...' to show boxes and".as_ptr(),
        );
        set_help_line(
            0,
            c"lists on your terminal as well as in the transcript file.".as_ptr(),
        );
    }
    error();
}
// translated from xetex-xetex0.c:6428 (char_warning)
pub unsafe fn char_warning(f: i32, c: i32) {
    const INT_PAR_TRACING_LOST_CHARS: i32 = 35; // xetex_format.h:70
    const INT_PAR_TRACING_ONLINE: i32 = 29; // xetex_format.h:64

    let tracing_lost = eqtb_b32s1(INT_BASE + INT_PAR_TRACING_LOST_CHARS);
    if tracing_lost > 0 {
        let old_setting = eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE);
        if tracing_lost > 1 {
            crate::globals::set_eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE, 1);
        }
        if tracing_lost > 2 {
            if crate::globals::file_line_error_style_p() != 0 {
                crate::output::print_file_line();
            } else {
                crate::output::print_nl_cstr(c"! ".as_ptr());
            }
            crate::output::print_cstr(c"Missing character: There is no ".as_ptr());
        } else {
            crate::output::begin_diagnostic();
            crate::output::diagnostic_begin_capture_warning_here();
            crate::output::print_nl_cstr(c"Missing character: There is no ".as_ptr());
        }
        if c < 65536 {
            crate::output::print(c);
        } else {
            crate::output::print_char(c);
        }
        crate::output::print_cstr(c" (".as_ptr());
        let fa = crate::globals::font_area_entry(f);
        if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
            crate::output::print_ucs_code(c);
        } else {
            crate::output::print_hex(c);
        }
        crate::output::print_char(b')' as i32);
        crate::output::print_cstr(c" in font ".as_ptr());
        crate::output::print(crate::globals::font_name_entry(f));
        if tracing_lost < 3 {
            crate::output::print_char(b'!' as i32);
        }
        crate::globals::set_eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE, old_setting);
        if tracing_lost > 2 {
            crate::globals::set_help_ptr(0);
            error();
        } else {
            capture_to_diagnostic(std::ptr::null_mut());
            crate::output::end_diagnostic(false);
        }
    }

    // Emit a machine-readable warning via ttstub_issue_warning.
    let font_name_c = crate::texmfmp::gettexstring(crate::globals::font_name_entry(f));
    let prev_sel = crate::globals::selector();
    crate::globals::set_selector(SELECTOR_NEW_STRING);
    if c < 0x10000 {
        crate::output::print(c);
    } else {
        crate::output::print_char(c);
    }
    crate::globals::set_selector(prev_sel);
    let s = crate::string_pool::make_string_global();
    let chr_c = crate::texmfmp::gettexstring(s);
    // flush_string: str_ptr--, pool_ptr = str_start[str_ptr - TOO_BIG_CHAR]
    set_str_ptr(str_ptr() - 1);
    set_pool_ptr(crate::globals::str_start(str_ptr() - TOO_BIG_CHAR));
    ttstub_issue_warning(
        c"could not represent character \"%s\" (0x%x) in font \"%s\"".as_ptr(),
        chr_c,
        c as u32,
        font_name_c,
    );
    libc::free(font_name_c as *mut libc::c_void);
    libc::free(chr_c as *mut libc::c_void);
    if !crate::globals::gave_char_warning_help() {
        ttstub_issue_warning(
            c"  you may need to load the `fontspec` package and use (e.g.) \\setmainfont to"
                .as_ptr(),
        );
        ttstub_issue_warning(
            c"  choose a different font that covers the unrepresentable character(s)".as_ptr(),
        );
        crate::globals::set_gave_char_warning_help(true);
    }
}
/// translated from xetex-xetex0.c:735 (show_box)
pub unsafe fn show_box(p: i32) {
    crate::state::with_state_mut(|st| show_box_in(st, p));
}

/// `&mut EngineState` core of [`show_box`].
pub(crate) fn show_box_in(st: &mut EngineState, p: i32) {
    st.errors.depth_threshold = st.eqtb.b32s1(INT_BASE + INT_PAR_SHOW_BOX_DEPTH);
    st.errors.breadth_max = st.eqtb.b32s1(INT_BASE + INT_PAR_SHOW_BOX_BREADTH);
    if st.errors.breadth_max <= 0 {
        st.errors.breadth_max = 5;
    }
    let dt = st.strings.pool_size - st.strings.pool_ptr - 1;
    if st.errors.depth_threshold > dt {
        st.errors.depth_threshold = dt;
    }
    show_node_list_in(st, p);
    print_ln_in(st);
}

/// translated from xetex-xetex0.c:117 (short_display)
pub unsafe fn short_display(p: i32) {
    crate::state::with_state_mut(|st| short_display_in(st, p));
}

/// `&mut EngineState` core of [`short_display`].
pub(crate) fn short_display_in(st: &mut EngineState, mut p: i32) {
    while p > 0 {
        if p >= st.mem.hi_mem_min {
            // is_char_node(p)
            if p <= st.mem.mem_end {
                let fnt = st.mem.b16s1(p) as i32;
                if fnt != st.errors.font_in_short_display {
                    if fnt > st.fonts.font_max {
                        print_char_in(st, '*' as i32);
                    } else {
                        let s = st.eqtb.hash_s1(FONT_ID_BASE_SD + fnt);
                        print_esc_in(st, s);
                    }
                    print_char_in(st, ' ' as i32);
                    st.errors.font_in_short_display = fnt;
                }
                print_in(st, st.mem.b16s0(p) as i32);
            }
        } else {
            match st.mem.b16s1(p) {
                HLIST_NODE | VLIST_NODE_SD | INS_NODE_SD | MARK_NODE_SD | ADJUST_NODE_SD
                | UNSET_NODE_SD => {
                    print_cstr_in(st, c"[]".as_ptr());
                }
                WHATSIT_NODE_SD => match st.mem.b16s0(p) {
                    NATIVE_WORD_NODE_SD | NATIVE_WORD_NODE_AT_SD => {
                        let fnt = st.mem.b16s2(p + 4) as i32;
                        if fnt != st.errors.font_in_short_display {
                            let s = st.eqtb.hash_s1(FONT_ID_BASE_SD + fnt);
                            print_esc_in(st, s);
                            print_char_in(st, ' ' as i32);
                            st.errors.font_in_short_display = fnt;
                        }
                        print_native_word_in(st, p);
                    }
                    _ => {
                        print_cstr_in(st, c"[]".as_ptr());
                    }
                },
                RULE_NODE => {
                    print_char_in(st, '|' as i32);
                }
                GLUE_NODE => {
                    if st.mem.b32s0(p + 1) != 0 {
                        print_char_in(st, ' ' as i32);
                    }
                }
                MATH_NODE => {
                    if st.mem.b16s0(p) >= L_CODE_SD {
                        print_cstr_in(st, c"[]".as_ptr());
                    } else {
                        print_char_in(st, '$' as i32);
                    }
                }
                LIGATURE_NODE => {
                    short_display_in(st, st.mem.b32s1(p + 1));
                }
                DISC_NODE => {
                    short_display_in(st, st.mem.b32s0(p + 1));
                    short_display_in(st, st.mem.b32s1(p + 1));
                    let mut n = st.mem.b16s0(p) as i32;
                    while n > 0 {
                        if st.mem.b32s1(p) != crate::mem::TEX_NULL {
                            p = st.mem.b32s1(p);
                        }
                        n -= 1;
                    }
                }
                _ => {}
            }
        }
        p = st.mem.b32s1(p);
    }
}
/// translated from xetex-xetex0.c:1123 (show_activities)
pub unsafe fn show_activities() {
    crate::state::with_state_mut(|st| show_activities_in(st));
}

/// `&mut EngineState` core of [`show_activities`].
pub(crate) fn show_activities_in(st: &mut EngineState) {
    st.input.nest[(st.input.nest_ptr) as usize] = crate::globals::cur_list();
    crate::output::print_nl_cstr_in(st, c"".as_ptr());
    crate::output::print_ln_in(st);
    let np = st.input.nest_ptr;
    for p in (0..=np).rev() {
        let m = st.input.nest[(p) as usize].mode;
        let a_s0 = unsafe { st.input.nest[(p) as usize].aux.b32.s0 };
        let a_s1 = unsafe { st.input.nest[(p) as usize].aux.b32.s1 };
        crate::output::print_nl_cstr_in(st, c"### ".as_ptr());
        crate::output::print_mode_in(st, m as i32);
        crate::output::print_cstr_in(st, c" entered at line ".as_ptr());
        crate::output::print_int_in(st, st.input.nest[(p) as usize].mode_line.abs());
        if m == HMODE_SA {
            let pg = st.input.nest[(p) as usize].prev_graf;
            if pg != 0x83_0000 {
                crate::output::print_cstr_in(st, c" (language".as_ptr());
                crate::output::print_int_in(st, pg % 65536);
                crate::output::print_cstr_in(st, c":hyphenmin".as_ptr());
                crate::output::print_int_in(st, pg / 0x40_0000);
                crate::output::print_char_in(st, ',' as i32);
                crate::output::print_int_in(st, (pg / 65536) % 64);
                crate::output::print_char_in(st, ')' as i32);
            }
        }
        if st.input.nest[(p) as usize].mode_line < 0 {
            crate::output::print_cstr_in(st, c" (\\output routine)".as_ptr());
        }
        if p == 0 {
            if PAGE_HEAD_SA != st.shipout.page_tail {
                crate::output::print_nl_cstr_in(st, c"### current page:".as_ptr());
                if st.shipout.output_active {
                    crate::output::print_cstr_in(st, c" (held over for next output)".as_ptr());
                }
                show_box_in(st, st.mem.b32s1(PAGE_HEAD_SA));
                if st.shipout.page_contents > EMPTY_SA {
                    crate::output::print_nl_cstr_in(st, c"total height ".as_ptr());
                    crate::output::print_totals_in(st);
                    crate::output::print_nl_cstr_in(st, c" goal height ".as_ptr());
                    crate::output::print_scaled_in(st, st.shipout.page_so_far[0]);
                    let mut r = st.mem.b32s1(PAGE_INS_HEAD_SA);
                    while r != PAGE_INS_HEAD_SA {
                        crate::output::print_ln_in(st);
                        crate::output::print_esc_cstr_in(st, c"insert".as_ptr());
                        let mut t = st.mem.b16s0(r) as i32;
                        crate::output::print_int_in(st, t);
                        crate::output::print_cstr_in(st, c" adds ".as_ptr());
                        let count_t = st.eqtb.b32s1(COUNT_BASE_SA + t);
                        if count_t == 1000 {
                            t = st.mem.b32s1(r + 3);
                        } else {
                            let xn = st.mem.b32s1(r + 3);
                            t = crate::scaled_math::x_over_n_in(&mut st.errors, xn, 1000) * count_t;
                        }
                        crate::output::print_scaled_in(st, t);
                        if st.mem.b16s1(r) == SPLIT_UP_SA {
                            let mut q = PAGE_HEAD_SA;
                            t = 0;
                            loop {
                                q = st.mem.b32s1(q);
                                if st.mem.b16s1(q) == INS_NODE_SA
                                    && st.mem.b16s0(q) == st.mem.b16s0(r)
                                {
                                    t += 1;
                                }
                                if q == st.mem.b32s0(r + 1) {
                                    break;
                                }
                            }
                            crate::output::print_cstr_in(st, c", #".as_ptr());
                            crate::output::print_int_in(st, t);
                            crate::output::print_cstr_in(st, c" might split".as_ptr());
                        }
                        r = st.mem.b32s1(r);
                    }
                }
            }
            if st.mem.b32s1(CONTRIB_HEAD_SA) != crate::mem::TEX_NULL {
                crate::output::print_nl_cstr_in(st, c"### recent contributions:".as_ptr());
            }
        }
        show_box_in(st, st.mem.b32s1(st.input.nest[(p) as usize].head));
        match (m.abs() / (MAX_COMMAND_SA + 1)) as i32 {
            0 => {
                crate::output::print_nl_cstr_in(st, c"prevdepth ".as_ptr());
                if a_s1 <= IGNORE_DEPTH_SA {
                    crate::output::print_cstr_in(st, c"ignored".as_ptr());
                } else {
                    crate::output::print_scaled_in(st, a_s1);
                }
                let pg = st.input.nest[(p) as usize].prev_graf;
                if pg != 0 {
                    crate::output::print_cstr_in(st, c", prevgraf ".as_ptr());
                    crate::output::print_int_in(st, pg);
                    if pg != 1 {
                        crate::output::print_cstr_in(st, c" lines".as_ptr());
                    } else {
                        crate::output::print_cstr_in(st, c" line".as_ptr());
                    }
                }
            }
            1 => {
                crate::output::print_nl_cstr_in(st, c"spacefactor ".as_ptr());
                crate::output::print_int_in(st, a_s0);
                if m > 0 && a_s1 > 0 {
                    crate::output::print_cstr_in(st, c", current language ".as_ptr());
                    crate::output::print_int_in(st, a_s1);
                }
            }
            2 => {
                if a_s1 != crate::mem::TEX_NULL {
                    crate::output::print_cstr_in(st, c"this will begin denominator of:".as_ptr());
                    show_box_in(st, a_s1);
                }
            }
            _ => {}
        }
    }
}
/// translated from xetex-xetex0.c:215 (print_subsidiary_data)
/// `&mut EngineState` core of `print_subsidiary_data`.
fn print_subsidiary_data_in(st: &mut EngineState, p: i32, c: u16) {
    if crate::hash::cur_length(&st.strings) >= st.errors.depth_threshold {
        if st.mem.b32s1(p) != 0 {
            // EMPTY = 0
            print_cstr_in(st, c" []".as_ptr());
        }
    } else {
        st.strings.str_pool[st.strings.pool_ptr as usize] = c;
        st.strings.pool_ptr += 1;
        crate::globals::set_temp_ptr(p);
        match st.mem.b32s1(p) {
            MATH_CHAR_SNL => {
                print_ln_in(st);
                print_current_string_in(st);
                print_fam_and_char_in(st, p);
            }
            SUB_BOX_SNL => {
                show_info_in(st);
            }
            SUB_MLIST_SNL => {
                if st.mem.b32s0(p) == crate::mem::TEX_NULL {
                    print_ln_in(st);
                    print_current_string_in(st);
                    print_cstr_in(st, c"{}".as_ptr());
                } else {
                    show_info_in(st);
                }
            }
            _ => {}
        }
        st.strings.pool_ptr -= 1;
    }
}
/// translated from xetex-xetex0.c:259 (show_node_list)
pub(crate) fn show_node_list_in(st: &mut EngineState, mut p: i32) {
    if crate::hash::cur_length(&st.strings) > st.errors.depth_threshold {
        if p > crate::mem::TEX_NULL {
            print_cstr_in(st, c" []".as_ptr());
        }
        return;
    }
    let mut n = 0i32;
    while p > 0 {
        print_ln_in(st);
        print_current_string_in(st);
        if p > st.mem.mem_end {
            print_cstr_in(st, c"Bad link, display aborted.".as_ptr());
            return;
        }
        n += 1;
        if n > st.errors.breadth_max {
            print_cstr_in(st, c"etc.".as_ptr());
            return;
        }
        if p >= st.mem.hi_mem_min {
            print_font_and_char_in(st, p);
        } else {
            match st.mem.b16s1(p) {
                HLIST_NODE | VLIST_NODE_SNL | UNSET_NODE_SNL => {
                    let ntype = st.mem.b16s1(p);
                    if ntype == HLIST_NODE {
                        print_esc_in(st, 'h' as i32);
                    } else if ntype == VLIST_NODE_SNL {
                        print_esc_in(st, 'v' as i32);
                    } else {
                        print_esc_cstr_in(st, c"unset".as_ptr());
                    }
                    print_cstr_in(st, c"box(".as_ptr());
                    print_scaled_in(st, st.mem.b32s1(p + 3));
                    print_char_in(st, '+' as i32);
                    print_scaled_in(st, st.mem.b32s1(p + 2));
                    print_cstr_in(st, c")x".as_ptr());
                    print_scaled_in(st, st.mem.b32s1(p + 1));
                    if ntype == UNSET_NODE_SNL {
                        if st.mem.b16s0(p) != 0 {
                            print_cstr_in(st, c" (".as_ptr());
                            print_int_in(st, st.mem.b16s0(p) as i32 + 1);
                            print_cstr_in(st, c" columns)".as_ptr());
                        }
                        if st.mem.b32s1(p + 6) != 0 {
                            print_cstr_in(st, c", stretch ".as_ptr());
                            print_glue_in(
                                st,
                                st.mem.b32s1(p + 6),
                                st.mem.b16s0(p + 5) as i32,
                                std::ptr::null(),
                            );
                        }
                        if st.mem.b32s1(p + 4) != 0 {
                            print_cstr_in(st, c", shrink ".as_ptr());
                            print_glue_in(
                                st,
                                st.mem.b32s1(p + 4),
                                st.mem.b16s1(p + 5) as i32,
                                std::ptr::null(),
                            );
                        }
                    } else {
                        let g = st.mem.gr(p + 6);
                        if g != 0.0 && st.mem.b16s1(p + 5) != NORMAL_SNL {
                            print_cstr_in(st, c", glue set ".as_ptr());
                            if st.mem.b16s1(p + 5) == SHRINKING_SNL {
                                print_cstr_in(st, c"- ".as_ptr());
                            }
                            if g.abs() > 20000.0 {
                                if g > 0.0 {
                                    print_char_in(st, '>' as i32);
                                } else {
                                    print_cstr_in(st, c"< -".as_ptr());
                                }
                                print_glue_in(
                                    st,
                                    20000 * 65536,
                                    st.mem.b16s0(p + 5) as i32,
                                    std::ptr::null(),
                                );
                            } else {
                                print_glue_in(
                                    st,
                                    crate::scaled_math::tex_round(65536.0 * g),
                                    st.mem.b16s0(p + 5) as i32,
                                    std::ptr::null(),
                                );
                            }
                        }
                        if st.mem.b32s1(p + 4) != 0 {
                            print_cstr_in(st, c", shifted ".as_ptr());
                            print_scaled_in(st, st.mem.b32s1(p + 4));
                        }
                        if ntype == HLIST_NODE && st.mem.b16s0(p) == DLIST_SNL {
                            print_cstr_in(st, c", display".as_ptr());
                        }
                    }
                    st.strings.str_pool[st.strings.pool_ptr as usize] = '.' as u16;
                    st.strings.pool_ptr += 1;
                    show_node_list_in(st, st.mem.b32s1(p + 5));
                    st.strings.pool_ptr -= 1;
                }
                RULE_NODE => {
                    print_esc_cstr_in(st, c"rule(".as_ptr());
                    print_rule_dimen_in(st, st.mem.b32s1(p + 3));
                    print_char_in(st, '+' as i32);
                    print_rule_dimen_in(st, st.mem.b32s1(p + 2));
                    print_cstr_in(st, c")x".as_ptr());
                    print_rule_dimen_in(st, st.mem.b32s1(p + 1));
                }
                INS_NODE_SNL => {
                    print_esc_cstr_in(st, c"insert".as_ptr());
                    print_int_in(st, st.mem.b16s0(p) as i32);
                    print_cstr_in(st, c", natural size ".as_ptr());
                    print_scaled_in(st, st.mem.b32s1(p + 3));
                    print_cstr_in(st, c"; split(".as_ptr());
                    print_spec_in(st, st.mem.b32s1(p + 4), std::ptr::null());
                    print_char_in(st, ',' as i32);
                    print_scaled_in(st, st.mem.b32s1(p + 2));
                    print_cstr_in(st, c"); float cost ".as_ptr());
                    print_int_in(st, st.mem.b32s1(p + 1));
                    st.strings.str_pool[st.strings.pool_ptr as usize] = '.' as u16;
                    st.strings.pool_ptr += 1;
                    show_node_list_in(st, st.mem.b32s0(p + 4));
                    st.strings.pool_ptr -= 1;
                }
                WHATSIT_NODE_SNL => {
                    match st.mem.b16s0(p) {
                        OPEN_NODE_SNL => {
                            print_write_whatsit_in(st, c"openout".as_ptr(), p);
                            print_char_in(st, '=' as i32);
                            print_file_name_in(
                                st,
                                st.mem.b32s1(p + 1),
                                st.mem.b32s0(p + 2),
                                st.mem.b32s1(p + 2),
                            );
                        }
                        WRITE_NODE_SNL => {
                            print_write_whatsit_in(st, c"write".as_ptr(), p);
                            print_mark_in(st, st.mem.b32s1(p + 1));
                        }
                        CLOSE_NODE_SNL => {
                            print_write_whatsit_in(st, c"closeout".as_ptr(), p);
                        }
                        SPECIAL_NODE_SNL => {
                            print_esc_cstr_in(st, c"special".as_ptr());
                            print_mark_in(st, st.mem.b32s1(p + 1));
                        }
                        LANGUAGE_NODE_SNL => {
                            print_esc_cstr_in(st, c"setlanguage".as_ptr());
                            print_int_in(st, st.mem.b32s1(p + 1));
                            print_cstr_in(st, c" (hyphenmin ".as_ptr());
                            print_int_in(st, st.mem.b16s1(p + 1) as i32);
                            print_char_in(st, ',' as i32);
                            print_int_in(st, st.mem.b16s0(p + 1) as i32);
                            print_char_in(st, ')' as i32);
                        }
                        PDF_SAVE_POS_NODE_SNL => {
                            print_esc_cstr_in(st, c"pdfsavepos".as_ptr());
                        }
                        NATIVE_WORD_NODE_SNL | NATIVE_WORD_NODE_AT_SNL => {
                            // hash[FONT_ID_BASE + mem[p+4].b16.s2].s1
                            print_esc_in(
                                st,
                                st.eqtb
                                    .hash_s1(FONT_ID_BASE_SNL + st.mem.b16s2(p + 4) as i32),
                            );
                            print_char_in(st, ' ' as i32);
                            print_native_word_in(st, p);
                        }
                        GLYPH_NODE_SNL => {
                            print_esc_in(
                                st,
                                st.eqtb
                                    .hash_s1(FONT_ID_BASE_SNL + st.mem.b16s2(p + 4) as i32),
                            );
                            print_cstr_in(st, c" glyph#".as_ptr());
                            print_int_in(st, st.mem.b16s1(p + 4) as i32);
                        }
                        PIC_NODE_SNL | PDF_NODE_SNL => {
                            if st.mem.b16s0(p) == PIC_NODE_SNL {
                                print_esc_cstr_in(st, c"XeTeXpicfile".as_ptr());
                            } else {
                                print_esc_cstr_in(st, c"XeTeXpdffile".as_ptr());
                            }
                            print_cstr_in(st, c"( ".as_ptr());
                            let path_len = st.mem.b16s1(p + 4) as i32;
                            let path_ptr = st.mem.node_addr(p + PIC_NODE_SIZE_SNL) as *const u8;
                            for i in 0..path_len {
                                print_raw_char_in(
                                    st,
                                    unsafe { *path_ptr.add(i as usize) } as u16,
                                    true,
                                );
                            }
                            print_in(st, '"' as i32);
                        }
                        _ => {
                            print_cstr_in(st, c"whatsit?".as_ptr());
                        }
                    }
                }
                GLUE_NODE => {
                    let sub = st.mem.b16s0(p);
                    if sub >= A_LEADERS_SNL {
                        print_esc_cstr_in(st, c"".as_ptr());
                        if sub == C_LEADERS_SNL {
                            print_char_in(st, 'c' as i32);
                        } else if sub == X_LEADERS_SNL {
                            print_char_in(st, 'x' as i32);
                        }
                        print_cstr_in(st, c"leaders ".as_ptr());
                        print_spec_in(st, st.mem.b32s0(p + 1), std::ptr::null());
                        st.strings.str_pool[st.strings.pool_ptr as usize] = '.' as u16;
                        st.strings.pool_ptr += 1;
                        show_node_list_in(st, st.mem.b32s1(p + 1));
                        st.strings.pool_ptr -= 1;
                    } else {
                        print_esc_cstr_in(st, c"glue".as_ptr());
                        if sub != NORMAL_SNL {
                            // GLUE_SPEC_shrink_order(p) != NORMAL
                            print_char_in(st, '(' as i32);
                            if sub < COND_MATH_GLUE_SNL {
                                print_skip_param_in(st, sub as i32 - 1);
                            } else if sub == COND_MATH_GLUE_SNL {
                                print_esc_cstr_in(st, c"nonscript".as_ptr());
                            } else {
                                print_esc_cstr_in(st, c"mskip".as_ptr());
                            }
                            print_char_in(st, ')' as i32);
                        }
                        if sub != COND_MATH_GLUE_SNL {
                            print_char_in(st, ' ' as i32);
                            if sub < COND_MATH_GLUE_SNL {
                                print_spec_in(st, st.mem.b32s0(p + 1), std::ptr::null());
                            } else {
                                print_spec_in(st, st.mem.b32s0(p + 1), c"mu".as_ptr());
                            }
                        }
                    }
                }
                KERN_NODE => {
                    if st.mem.b16s0(p) != MU_GLUE_SNL {
                        print_esc_cstr_in(st, c"kern".as_ptr());
                        if st.mem.b16s0(p) != NORMAL_SNL {
                            print_char_in(st, ' ' as i32);
                        }
                        print_scaled_in(st, st.mem.b32s1(p + 1));
                        let sub = st.mem.b16s0(p);
                        if sub == ACC_KERN_SNL {
                            print_cstr_in(st, c" (for accent)".as_ptr());
                        } else if sub == SPACE_ADJUSTMENT_SNL {
                            print_cstr_in(st, c" (space adjustment)".as_ptr());
                        }
                    } else {
                        print_esc_cstr_in(st, c"mkern".as_ptr());
                        print_scaled_in(st, st.mem.b32s1(p + 1));
                        print_cstr_in(st, c"mu".as_ptr());
                    }
                }
                MARGIN_KERN_NODE_SNL => {
                    print_esc_cstr_in(st, c"kern".as_ptr());
                    print_scaled_in(st, st.mem.b32s1(p + 1));
                    if st.mem.b16s0(p) == 0 {
                        print_cstr_in(st, c" (left margin)".as_ptr());
                    } else {
                        print_cstr_in(st, c" (right margin)".as_ptr());
                    }
                }
                MATH_NODE => {
                    let sub = st.mem.b16s0(p);
                    if sub > AFTER_SNL {
                        if sub % 2 == 1 {
                            print_esc_cstr_in(st, c"end".as_ptr());
                        } else {
                            print_esc_cstr_in(st, c"begin".as_ptr());
                        }
                        if sub > R_CODE_SNL {
                            print_char_in(st, 'R' as i32);
                        } else if sub > L_CODE_SNL {
                            print_char_in(st, 'L' as i32);
                        } else {
                            print_char_in(st, 'M' as i32);
                        }
                    } else {
                        print_esc_cstr_in(st, c"math".as_ptr());
                        if sub == BEFORE_SNL {
                            print_cstr_in(st, c"on".as_ptr());
                        } else {
                            print_cstr_in(st, c"off".as_ptr());
                        }
                        if st.mem.b32s1(p + 1) != 0 {
                            print_cstr_in(st, c", surrounded ".as_ptr());
                            print_scaled_in(st, st.mem.b32s1(p + 1));
                        }
                    }
                }
                LIGATURE_NODE => {
                    print_font_and_char_in(st, p + 1);
                    print_cstr_in(st, c" (ligature ".as_ptr());
                    if st.mem.b16s0(p) > 1 {
                        print_char_in(st, '|' as i32);
                    }
                    st.errors.font_in_short_display = st.mem.b16s1(p + 1) as i32;
                    short_display_in(st, st.mem.b32s1(p + 1));
                    if st.mem.b16s0(p) % 2 == 1 {
                        print_char_in(st, '|' as i32);
                    }
                    print_char_in(st, ')' as i32);
                }
                PENALTY_NODE => {
                    print_esc_cstr_in(st, c"penalty ".as_ptr());
                    print_int_in(st, st.mem.b32s1(p + 1));
                }
                DISC_NODE => {
                    print_esc_cstr_in(st, c"discretionary".as_ptr());
                    if st.mem.b16s0(p) > 0 {
                        print_cstr_in(st, c" replacing ".as_ptr());
                        print_int_in(st, st.mem.b16s0(p) as i32);
                    }
                    st.strings.str_pool[st.strings.pool_ptr as usize] = '.' as u16;
                    st.strings.pool_ptr += 1;
                    show_node_list_in(st, st.mem.b32s0(p + 1));
                    st.strings.pool_ptr -= 1;
                    st.strings.str_pool[st.strings.pool_ptr as usize] = '|' as u16;
                    st.strings.pool_ptr += 1;
                    show_node_list_in(st, st.mem.b32s1(p + 1));
                    st.strings.pool_ptr -= 1;
                }
                MARK_NODE_SNL => {
                    print_esc_cstr_in(st, c"mark".as_ptr());
                    if st.mem.b32s0(p + 1) != 0 {
                        print_char_in(st, 's' as i32);
                        print_int_in(st, st.mem.b32s0(p + 1));
                    }
                    print_mark_in(st, st.mem.b32s1(p + 1));
                }
                ADJUST_NODE_SNL => {
                    print_esc_cstr_in(st, c"vadjust".as_ptr());
                    if st.mem.b16s0(p) != 0 {
                        print_cstr_in(st, c" pre ".as_ptr());
                    }
                    st.strings.str_pool[st.strings.pool_ptr as usize] = '.' as u16;
                    st.strings.pool_ptr += 1;
                    show_node_list_in(st, st.mem.b32s1(p + 1));
                    st.strings.pool_ptr -= 1;
                }
                STYLE_NODE_SNL => {
                    print_style_in(st, st.mem.b16s0(p) as i32);
                }
                CHOICE_NODE_SNL => {
                    print_esc_cstr_in(st, c"mathchoice".as_ptr());
                    st.strings.str_pool[st.strings.pool_ptr as usize] = 'D' as u16;
                    st.strings.pool_ptr += 1;
                    show_node_list_in(st, st.mem.b32s0(p + 1));
                    st.strings.pool_ptr -= 1;
                    st.strings.str_pool[st.strings.pool_ptr as usize] = 'T' as u16;
                    st.strings.pool_ptr += 1;
                    show_node_list_in(st, st.mem.b32s1(p + 1));
                    st.strings.pool_ptr -= 1;
                    st.strings.str_pool[st.strings.pool_ptr as usize] = 'S' as u16;
                    st.strings.pool_ptr += 1;
                    show_node_list_in(st, st.mem.b32s0(p + 2));
                    st.strings.pool_ptr -= 1;
                    st.strings.str_pool[st.strings.pool_ptr as usize] = 's' as u16;
                    st.strings.pool_ptr += 1;
                    show_node_list_in(st, st.mem.b32s1(p + 2));
                    st.strings.pool_ptr -= 1;
                }
                ORD_NOAD_SNL | OP_NOAD_SNL | BIN_NOAD_SNL | REL_NOAD_SNL | OPEN_NOAD_SNL
                | CLOSE_NOAD_SNL | PUNCT_NOAD_SNL | INNER_NOAD_SNL | RADICAL_NOAD_SNL
                | OVER_NOAD_SNL | UNDER_NOAD_SNL | VCENTER_NOAD_SNL | ACCENT_NOAD_SNL
                | LEFT_NOAD_SNL | RIGHT_NOAD_SNL => {
                    let ntype = st.mem.b16s1(p);
                    match ntype {
                        ORD_NOAD_SNL => {
                            print_esc_cstr_in(st, c"mathord".as_ptr());
                        }
                        OP_NOAD_SNL => {
                            print_esc_cstr_in(st, c"mathop".as_ptr());
                        }
                        BIN_NOAD_SNL => {
                            print_esc_cstr_in(st, c"mathbin".as_ptr());
                        }
                        REL_NOAD_SNL => {
                            print_esc_cstr_in(st, c"mathrel".as_ptr());
                        }
                        OPEN_NOAD_SNL => {
                            print_esc_cstr_in(st, c"mathopen".as_ptr());
                        }
                        CLOSE_NOAD_SNL => {
                            print_esc_cstr_in(st, c"mathclose".as_ptr());
                        }
                        PUNCT_NOAD_SNL => {
                            print_esc_cstr_in(st, c"mathpunct".as_ptr());
                        }
                        INNER_NOAD_SNL => {
                            print_esc_cstr_in(st, c"mathinner".as_ptr());
                        }
                        OVER_NOAD_SNL => {
                            print_esc_cstr_in(st, c"overline".as_ptr());
                        }
                        UNDER_NOAD_SNL => {
                            print_esc_cstr_in(st, c"underline".as_ptr());
                        }
                        VCENTER_NOAD_SNL => {
                            print_esc_cstr_in(st, c"vcenter".as_ptr());
                        }
                        RADICAL_NOAD_SNL => {
                            print_esc_cstr_in(st, c"radical".as_ptr());
                            print_delimiter_in(st, p + 4);
                        }
                        ACCENT_NOAD_SNL => {
                            print_esc_cstr_in(st, c"accent".as_ptr());
                            print_fam_and_char_in(st, p + 4);
                        }
                        LEFT_NOAD_SNL => {
                            print_esc_cstr_in(st, c"left".as_ptr());
                            print_delimiter_in(st, p + 1);
                        }
                        RIGHT_NOAD_SNL => {
                            if st.mem.b16s0(p) == NORMAL_SNL {
                                print_esc_cstr_in(st, c"right".as_ptr());
                            } else {
                                print_esc_cstr_in(st, c"middle".as_ptr());
                            }
                            print_delimiter_in(st, p + 1);
                        }
                        _ => {}
                    }
                    if ntype < LEFT_NOAD_SNL {
                        if st.mem.b16s0(p) != NORMAL_SNL {
                            if st.mem.b16s0(p) == LIMITS_SNL {
                                print_esc_cstr_in(st, c"limits".as_ptr());
                            } else {
                                print_esc_cstr_in(st, c"nolimits".as_ptr());
                            }
                        }
                        print_subsidiary_data_in(st, p + 1, '.' as u16);
                    }
                    print_subsidiary_data_in(st, p + 2, '^' as u16);
                    print_subsidiary_data_in(st, p + 3, '_' as u16);
                }
                FRACTION_NOAD_SNL => {
                    print_esc_cstr_in(st, c"fraction, thickness ".as_ptr());
                    if st.mem.b32s1(p + 1) == DEFAULT_CODE_SNL {
                        print_cstr_in(st, c"= default".as_ptr());
                    } else {
                        print_scaled_in(st, st.mem.b32s1(p + 1));
                    }
                    // left delimiter check
                    if !st.mem.b16s3(p + 4).is_multiple_of(256)
                        || (st.mem.b16s2(p + 4) as i32 + (st.mem.b16s3(p + 4) as i32 / 256) * 65536)
                            != 0
                        || !st.mem.b16s1(p + 4).is_multiple_of(256)
                        || (st.mem.b16s0(p + 4) as i32 + (st.mem.b16s1(p + 4) as i32 / 256) * 65536)
                            != 0
                    {
                        print_cstr_in(st, c", left-delimiter ".as_ptr());
                        print_delimiter_in(st, p + 4);
                    }
                    // right delimiter check
                    if !st.mem.b16s3(p + 5).is_multiple_of(256)
                        || (st.mem.b16s2(p + 5) as i32 + (st.mem.b16s3(p + 5) as i32 / 256) * 65536)
                            != 0
                        || !st.mem.b16s1(p + 5).is_multiple_of(256)
                        || (st.mem.b16s0(p + 5) as i32 + (st.mem.b16s1(p + 5) as i32 / 256) * 65536)
                            != 0
                    {
                        print_cstr_in(st, c", right-delimiter ".as_ptr());
                        print_delimiter_in(st, p + 5);
                    }
                    print_subsidiary_data_in(st, p + 2, '\\' as u16);
                    print_subsidiary_data_in(st, p + 3, '/' as u16);
                }
                _ => {
                    print_cstr_in(st, c"Unknown node type!".as_ptr());
                }
            }
        }
        p = st.mem.b32s1(p);
    }
}
// translated from xetex-xetex0.c:2764 (show_context)
pub unsafe fn show_context() {
    crate::state::with_state_mut(|st| show_context_in(st));
}

/// `&mut EngineState` core of [`show_context`].
pub(crate) fn show_context_in(st: &mut EngineState) {
    use crate::globals::{line, set_cur_input};

    const TOKEN_LIST: u16 = 0;
    const BACKED_UP: u16 = 3;
    const BACKED_UP_CHAR: u16 = 4;
    const PARAMETER: u16 = 0;
    const U_TEMPLATE: u16 = 1;
    const V_TEMPLATE: u16 = 2;
    const INSERTED: u16 = 5;
    const MACRO: u16 = 6;
    const OUTPUT_TEXT: u16 = 7;
    const EVERY_PAR_TEXT: u16 = 8;
    const EVERY_MATH_TEXT: u16 = 9;
    const EVERY_DISPLAY_TEXT: u16 = 10;
    const EVERY_HBOX_TEXT: u16 = 11;
    const EVERY_VBOX_TEXT: u16 = 12;
    const EVERY_JOB_TEXT: u16 = 13;
    const EVERY_CR_TEXT: u16 = 14;
    const MARK_TEXT: u16 = 15;
    const EVERY_EOF_TEXT: u16 = 16;
    const INTER_CHAR_TEXT: u16 = 17;
    const WRITE_TEXT: u16 = 18;
    const TECTONIC_CODA_TEXT: u16 = 19;
    const SELECTOR_PSEUDO: i32 = 20;
    const INT_PAR_ERROR_CONTEXT_LINES: i32 = 54;

    st.input.base_ptr = st.input.input_ptr;
    st.input.input_stack[st.input.base_ptr as usize] = cur_input();
    let mut nn: i32 = -1;
    let mut bottom_line = false;

    'outer: loop {
        set_cur_input(st.input.input_stack[(st.input.base_ptr) as usize]);
        let ci = cur_input();
        if ci.state != TOKEN_LIST && (ci.name > 19 || st.input.base_ptr == 0) {
            bottom_line = true;
        }
        let error_context_lines = st.eqtb.b32s1(INT_BASE + INT_PAR_ERROR_CONTEXT_LINES);
        if st.input.base_ptr == st.input.input_ptr || bottom_line || nn < error_context_lines {
            let ci = cur_input();
            if st.input.base_ptr == st.input.input_ptr
                || ci.state != TOKEN_LIST
                || ci.index != BACKED_UP
                || ci.loc != crate::mem::TEX_NULL
            {
                st.errors.tally = 0;
                let old_setting = st.shipout.selector;
                let l;
                if ci.state != TOKEN_LIST {
                    if ci.name <= 17 {
                        if ci.name == 0 {
                            if st.input.base_ptr == 0 {
                                crate::output::print_nl_cstr_in(st, c"<*>".as_ptr());
                            } else {
                                crate::output::print_nl_cstr_in(st, c"<insert> ".as_ptr());
                            }
                        } else {
                            crate::output::print_nl_cstr_in(st, c"<read ".as_ptr());
                            if ci.name == 17 {
                                crate::output::print_char_in(st, '*' as i32);
                            } else {
                                crate::output::print_int_in(st, ci.name - 1);
                            }
                            crate::output::print_char_in(st, '>' as i32);
                        }
                    } else {
                        crate::output::print_nl_cstr_in(st, c"l.".as_ptr());
                        if ci.index as i32 == st.input.in_open {
                            crate::output::print_int_in(st, line());
                        } else {
                            crate::output::print_int_in(
                                st,
                                st.input.line_stack[(ci.index as i32 + 1) as usize],
                            );
                        }
                    }
                    crate::output::print_char_in(st, ' ' as i32);
                    l = st.errors.tally;
                    st.errors.tally = 0;
                    st.shipout.selector = SELECTOR_PSEUDO;
                    st.errors.trick_count = 1_000_000;
                    let end_char = st.eqtb.b32s1(INT_BASE + INT_PAR_END_LINE_CHAR);
                    let j = if st.input.buffer[(ci.limit) as usize] == end_char {
                        ci.limit
                    } else {
                        ci.limit + 1
                    };
                    if j > 0 {
                        let mut i = ci.start;
                        let for_end = j - 1;
                        if i <= for_end {
                            loop {
                                if i == ci.loc {
                                    st.errors.first_count = st.errors.tally;
                                    let tc = st.errors.tally + 1 + st.errors.error_line
                                        - st.errors.half_error_line;
                                    st.errors.trick_count = tc.max(st.errors.error_line);
                                }
                                crate::output::print_char_in(st, st.input.buffer[(i) as usize]);
                                if i >= for_end {
                                    break;
                                }
                                i += 1;
                            }
                        }
                    }
                } else {
                    match ci.index {
                        PARAMETER => crate::output::print_nl_cstr_in(st, c"<argument> ".as_ptr()),
                        U_TEMPLATE | V_TEMPLATE => {
                            crate::output::print_nl_cstr_in(st, c"<template> ".as_ptr())
                        }
                        BACKED_UP | BACKED_UP_CHAR => {
                            if ci.loc == crate::mem::TEX_NULL {
                                crate::output::print_nl_cstr_in(st, c"<recently read> ".as_ptr());
                            } else {
                                crate::output::print_nl_cstr_in(
                                    st,
                                    c"<to be read again> ".as_ptr(),
                                );
                            }
                        }
                        INSERTED => {
                            crate::output::print_nl_cstr_in(st, c"<inserted text> ".as_ptr())
                        }
                        MACRO => {
                            crate::output::print_ln_in(st);
                            crate::output::print_cs_in(st, ci.name);
                        }
                        OUTPUT_TEXT => crate::output::print_nl_cstr_in(st, c"<output> ".as_ptr()),
                        EVERY_PAR_TEXT => {
                            crate::output::print_nl_cstr_in(st, c"<everypar> ".as_ptr())
                        }
                        EVERY_MATH_TEXT => {
                            crate::output::print_nl_cstr_in(st, c"<everymath> ".as_ptr())
                        }
                        EVERY_DISPLAY_TEXT => {
                            crate::output::print_nl_cstr_in(st, c"<everydisplay> ".as_ptr())
                        }
                        EVERY_HBOX_TEXT => {
                            crate::output::print_nl_cstr_in(st, c"<everyhbox> ".as_ptr())
                        }
                        EVERY_VBOX_TEXT => {
                            crate::output::print_nl_cstr_in(st, c"<everyvbox> ".as_ptr())
                        }
                        EVERY_JOB_TEXT => {
                            crate::output::print_nl_cstr_in(st, c"<everyjob> ".as_ptr())
                        }
                        EVERY_CR_TEXT => {
                            crate::output::print_nl_cstr_in(st, c"<everycr> ".as_ptr())
                        }
                        MARK_TEXT => crate::output::print_nl_cstr_in(st, c"<mark> ".as_ptr()),
                        EVERY_EOF_TEXT => {
                            crate::output::print_nl_cstr_in(st, c"<everyeof> ".as_ptr())
                        }
                        INTER_CHAR_TEXT => {
                            crate::output::print_nl_cstr_in(st, c"<XeTeXinterchartoks> ".as_ptr())
                        }
                        WRITE_TEXT => crate::output::print_nl_cstr_in(st, c"<write> ".as_ptr()),
                        TECTONIC_CODA_TEXT => {
                            crate::output::print_nl_cstr_in(st, c"<TectonicCodaTokens> ".as_ptr())
                        }
                        _ => crate::output::print_nl_in(st, '?' as i32),
                    }
                    l = st.errors.tally;
                    st.errors.tally = 0;
                    st.shipout.selector = SELECTOR_PSEUDO;
                    st.errors.trick_count = 1_000_000;
                    if ci.index < MACRO {
                        crate::output::show_token_list_in(st, ci.start, ci.loc, 100_000);
                    } else {
                        crate::output::show_token_list_in(
                            st,
                            st.mem.b32s1(ci.start),
                            ci.loc,
                            100_000,
                        );
                    }
                }
                st.shipout.selector = old_setting;
                if st.errors.trick_count == 1_000_000 {
                    st.errors.first_count = st.errors.tally;
                    let tc = st.errors.tally + 1 + st.errors.error_line - st.errors.half_error_line;
                    st.errors.trick_count = tc.max(st.errors.error_line);
                }
                let m = if st.errors.tally < st.errors.trick_count {
                    st.errors.tally - st.errors.first_count
                } else {
                    st.errors.trick_count - st.errors.first_count
                };
                let (mut p, n) = if l + st.errors.first_count <= st.errors.half_error_line {
                    (0i32, l + st.errors.first_count)
                } else {
                    crate::output::print_cstr_in(st, c"...".as_ptr());
                    (
                        l + st.errors.first_count - st.errors.half_error_line + 3,
                        st.errors.half_error_line,
                    )
                };
                let fc = st.errors.first_count;
                let mut q = p;
                while q < fc {
                    crate::output::print_char_in(
                        st,
                        st.errors.trick_buf[(q % st.errors.error_line) as usize] as i32,
                    );
                    q += 1;
                }
                crate::output::print_ln_in(st);
                let mut q2 = 1;
                while q2 <= n {
                    crate::output::print_raw_char_in(st, ' ' as u16, true);
                    q2 += 1;
                }
                p = if m + n <= st.errors.error_line {
                    fc + m
                } else {
                    fc + (st.errors.error_line - n - 3)
                };
                let mut q3 = fc;
                while q3 < p {
                    crate::output::print_char_in(
                        st,
                        st.errors.trick_buf[(q3 % st.errors.error_line) as usize] as i32,
                    );
                    q3 += 1;
                }
                if m + n > st.errors.error_line {
                    crate::output::print_cstr_in(st, c"...".as_ptr());
                }
                nn += 1;
            }
        } else if nn == error_context_lines {
            crate::output::print_nl_cstr_in(st, c"...".as_ptr());
            nn += 1;
        }
        if bottom_line {
            break 'outer;
        }
        st.input.base_ptr = st.input.base_ptr - 1;
    }
    set_cur_input(st.input.input_stack[(st.input.input_ptr) as usize]);
}

// ---- consts moved verbatim from scan.rs with the show_* group (workstream 02) ----

// Node-type constants (xetex_format.h:380–390).
const VLIST_NODE_SD: u16 = 1;
const INS_NODE_SD: u16 = 3;
const MARK_NODE_SD: u16 = 4;
const ADJUST_NODE_SD: u16 = 5;
const WHATSIT_NODE_SD: u16 = 8;
const UNSET_NODE_SD: u16 = 13;
const NATIVE_WORD_NODE_SD: u16 = 40;
const NATIVE_WORD_NODE_AT_SD: u16 = 41;
const L_CODE_SD: u16 = 4; // xetex-constants.h:142
const FONT_ID_BASE_SD: i32 = 2_245_338; // xetex_format.h:188

// MEM_TOP = 4999999; PAGE_INS_HEAD = MEM_TOP, CONTRIB_HEAD = MEM_TOP-1, PAGE_HEAD = MEM_TOP-2
const MEM_TOP_SA: i32 = 4_999_999;
const PAGE_INS_HEAD_SA: i32 = MEM_TOP_SA;
const CONTRIB_HEAD_SA: i32 = MEM_TOP_SA - 1;
const PAGE_HEAD_SA: i32 = MEM_TOP_SA - 2;
const MAX_COMMAND_SA: i16 = 102; // xetex-constants.h:325
const HMODE_SA: i16 = 104; // xetex-constants.h:369
const IGNORE_DEPTH_SA: i32 = -65_536_000; // xetex-constants.h:388
const EMPTY_SA: u8 = 0; // xetex-constants.h:230
const SPLIT_UP_SA: u16 = 1; // xetex-constants.h:274
const INS_NODE_SA: u16 = 3; // xetex_format.h:383
const COUNT_BASE_SA: i32 = 7_826_812; // xetex-constants.h:220

// Constants for show_node_list / print_subsidiary_data.
const NORMAL_SNL: u16 = 0;
const SHRINKING_SNL: u16 = 2; // xetex-constants.h:281
const DLIST_SNL: u16 = 2; // xetex-constants.h:278
const A_LEADERS_SNL: u16 = 100; // xetex_format.h:422
const C_LEADERS_SNL: u16 = 101; // xetex_format.h:423
const X_LEADERS_SNL: u16 = 102; // xetex_format.h:424
const MU_GLUE_SNL: u16 = 99; // xetex_format.h:421
const COND_MATH_GLUE_SNL: u16 = 98; // xetex-constants.h:324
const BEFORE_SNL: u16 = 0; // xetex_format.h:434
const AFTER_SNL: u16 = 1; // xetex_format.h:435
const L_CODE_SNL: u16 = 4; // xetex-constants.h:142
const R_CODE_SNL: u16 = 8; // xetex-constants.h:143
const ACC_KERN_SNL: u16 = 2; // xetex_format.h:429
const SPACE_ADJUSTMENT_SNL: u16 = 3; // xetex_format.h:430
const LIMITS_SNL: u16 = 1; // xetex_format.h:452
const LEFT_NOAD_SNL: u16 = 30; // xetex_format.h:415
const VLIST_NODE_SNL: u16 = 1;
const INS_NODE_SNL: u16 = 3;
const WHATSIT_NODE_SNL: u16 = 8;
const MARGIN_KERN_NODE_SNL: u16 = 40;
const UNSET_NODE_SNL: u16 = 13;
const STYLE_NODE_SNL: u16 = 14;
const CHOICE_NODE_SNL: u16 = 15;
const ORD_NOAD_SNL: u16 = 16;
const OP_NOAD_SNL: u16 = 17;
const BIN_NOAD_SNL: u16 = 18;
const REL_NOAD_SNL: u16 = 19;
const OPEN_NOAD_SNL: u16 = 20;
const CLOSE_NOAD_SNL: u16 = 21;
const PUNCT_NOAD_SNL: u16 = 22;
const INNER_NOAD_SNL: u16 = 23;
const RADICAL_NOAD_SNL: u16 = 24;
const FRACTION_NOAD_SNL: u16 = 25;
const UNDER_NOAD_SNL: u16 = 26;
const OVER_NOAD_SNL: u16 = 27;
const ACCENT_NOAD_SNL: u16 = 28;
const VCENTER_NOAD_SNL: u16 = 29;
const RIGHT_NOAD_SNL: u16 = 31;
const MARK_NODE_SNL: u16 = 4;
const ADJUST_NODE_SNL: u16 = 5;
// Whatsit subtypes.
const OPEN_NODE_SNL: u16 = 0;
const WRITE_NODE_SNL: u16 = 1;
const CLOSE_NODE_SNL: u16 = 2;
const SPECIAL_NODE_SNL: u16 = 3;
const LANGUAGE_NODE_SNL: u16 = 4;
const PDF_SAVE_POS_NODE_SNL: u16 = 21;
const NATIVE_WORD_NODE_SNL: u16 = 40;
const NATIVE_WORD_NODE_AT_SNL: u16 = 41;
const GLYPH_NODE_SNL: u16 = 42;
const PIC_NODE_SNL: u16 = 43;
const PDF_NODE_SNL: u16 = 44;
// print_subsidiary_data: mem[p].b32.s1 cases.
const MATH_CHAR_SNL: i32 = 1; // xetex-constants.h:269
const SUB_BOX_SNL: i32 = 2; // xetex-constants.h:283
const SUB_MLIST_SNL: i32 = 3; // xetex-constants.h:287
                              // Misc constants.
const DEFAULT_CODE_SNL: i32 = 0x4000_0000; // xetex-constants.h:17
const PIC_NODE_SIZE_SNL: i32 = 9; // xetex-constants.h
                                  // FONT_ID_BASE = FROZEN_NULL_FONT = 2245338
const FONT_ID_BASE_SNL: i32 = 2_245_338;
