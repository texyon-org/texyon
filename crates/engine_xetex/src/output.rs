//! Phase 3 — printing helpers translated from `xetex/xetex-output.c`.

use crate::globals::{
    base_ptr, cond_ptr, cur_boundary, cur_chr, cur_cmd, cur_group, cur_if, cur_input, cur_level,
    cur_list_mode, cur_mark, current_diagnostic, dig_entry, doing_special, eqtb_b32s1, eqtb_top,
    error_line, file_line_error_style_p, file_offset, font_max, full_source_filename_stack,
    grp_stack, hash_s1, hi_mem_min, history, if_limit, if_line, if_stack, in_open, input_ptr,
    input_stack, line, line_stack, lo_mem_max, log_file, max_print_line, mem_as_u16_ptr, mem_b16s0,
    mem_b16s1, mem_b16s2, mem_b16s3, mem_b32s0, mem_b32s1, mem_end, old_setting, pool_ptr,
    pool_size, prim_s1, rust_stdout, save_ptr, save_stack_b16s0, save_stack_b32s1, selector,
    set_base_ptr, set_cond_ptr, set_cur_group, set_cur_if, set_cur_level, set_current_diagnostic,
    set_dig_entry, set_eqtb_b32s1, set_file_offset, set_grp_stack, set_history, set_if_limit,
    set_if_line, set_if_stack, set_input_stack, set_mem_b16s1, set_old_setting, set_pool_ptr,
    set_save_ptr, set_selector, set_shown_mode, set_str_pool, set_tally, set_term_offset,
    set_trick_buf, shown_mode, str_pool_entry, str_ptr, str_start, tally, term_offset, trick_count,
    write_file,
};

// Selector constants (xetex-core.h:42–51).
const SELECTOR_NO_PRINT: i32 = 16;
const SELECTOR_TERM_ONLY: i32 = 17;
const SELECTOR_LOG_ONLY: i32 = 18;
const SELECTOR_TERM_AND_LOG: i32 = 19;
const SELECTOR_PSEUDO: i32 = 20;
const SELECTOR_NEW_STRING: i32 = 21;

// Constants for INTPAR(escape_char) = eqtb[INT_BASE + INT_PAR__escape_char].b32.s1
// (xetex_format.h:219,80 and xetex-constants.h:83).
const INT_BASE: i32 = 7_826_729;
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
const BIGGEST_CHAR: i32 = 0xFFFF;
const TEXT_SIZE: i32 = 0;
const SCRIPT_SIZE: i32 = 256;
// print_cs / sprint_cs constants (xetex_format.h).
const ACTIVE_BASE: i32 = 1;
const SINGLE_BASE: i32 = 1_114_113;
const NULL_CS: i32 = 2_228_225;
const HASH_BASE: i32 = 2_228_226;
const PRIM_EQTB_BASE: i32 = 2_243_238;
const FROZEN_NULL_FONT: i32 = 2_245_338;
const UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
const EQTB_SIZE: i32 = 8_941_458;
const CAT_CODE_BASE: i32 = 2_256_169;
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
    fn gettexstring(s: i32) -> *mut libc::c_char;
    // Still in C — complex context display.
    fn show_context();
}

// ---------------------------------------------------------------------------
// print_the_digs (static helper — not exported; all C callers translated here)
// ---------------------------------------------------------------------------

/// Print the digits in `dig[0..k]` in reverse order.
///
/// Corresponds to the static `print_the_digs` in xetex-output.c:350.
/// Not exported — only called from the Rust functions below.
fn print_the_digs(mut k: u8) {
    // translated from xetex-output.c:350 (print_the_digs)
    while k > 0 {
        k -= 1;
        let d = dig_entry(k as usize);
        // SAFETY: print_char is always safe to call during engine execution.
        unsafe {
            if d < 10 {
                print_char(b'0' as i32 + d as i32);
            } else {
                print_char(55 /* b'A' - 10 */ + d as i32);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// print_int
// ---------------------------------------------------------------------------

/// Print an integer in decimal.
///
/// translated from xetex-output.c:363 (print_int)
#[no_mangle]
pub unsafe extern "C" fn print_int(mut n: i32) {
    let mut k: u8 = 0;

    if n < 0 {
        print_char(b'-' as i32);
        if n > -100_000_000 {
            n = -(n);
        } else {
            let m_val = -1 - n;
            n = m_val / 10;
            let m = (m_val % 10) + 1;
            k = 1;
            if m < 10 {
                set_dig_entry(0, m as u8);
            } else {
                set_dig_entry(0, 0);
                n += 1;
            }
        }
    }

    loop {
        set_dig_entry(k as usize, (n % 10) as u8);
        n /= 10;
        k += 1;
        if n == 0 {
            break;
        }
    }

    print_the_digs(k);
}

// ---------------------------------------------------------------------------
// print_hex
// ---------------------------------------------------------------------------

/// Print an integer in hexadecimal with a leading `"`.
///
/// translated from xetex-output.c:658 (print_hex)
#[no_mangle]
pub unsafe extern "C" fn print_hex(mut n: i32) {
    let mut k: u8 = 0;
    print_char(b'"' as i32);
    loop {
        set_dig_entry(k as usize, (n % 16) as u8);
        n /= 16;
        k += 1;
        if n == 0 {
            break;
        }
    }
    print_the_digs(k);
}

// ---------------------------------------------------------------------------
// print_ucs_code
// ---------------------------------------------------------------------------

/// Print a Unicode scalar value in `U+XXXX` form.
///
/// translated from xetex-output.c:749 (print_ucs_code)
#[no_mangle]
pub unsafe extern "C" fn print_ucs_code(mut n: i32) {
    let mut k: u8 = 0;
    print_cstr(c"U+".as_ptr());
    loop {
        set_dig_entry(k as usize, (n % 16) as u8);
        n /= 16;
        k += 1;
        if n == 0 {
            break;
        }
    }
    while k < 4 {
        set_dig_entry(k as usize, 0);
        k += 1;
    }
    print_the_digs(k);
}

// ---------------------------------------------------------------------------
// print_cstr
// ---------------------------------------------------------------------------

/// Print a C string one character at a time via `print_char`.
///
/// translated from xetex-output.c:301 (print_cstr)
#[no_mangle]
pub unsafe extern "C" fn print_cstr(str_ptr: *const libc::c_char) {
    let s = std::ffi::CStr::from_ptr(str_ptr);
    for &b in s.to_bytes() {
        print_char(b as i32);
    }
}

// ---------------------------------------------------------------------------
// print_nl_cstr
// ---------------------------------------------------------------------------

/// Start a new line if needed, then print a C string.
///
/// translated from xetex-output.c:319 (print_nl_cstr)
#[no_mangle]
pub unsafe extern "C" fn print_nl_cstr(str_ptr: *const libc::c_char) {
    let sel = selector();
    if ((term_offset() > 0) && (sel & 1 != 0))
        || ((file_offset() > 0) && (sel >= SELECTOR_LOG_ONLY))
    {
        print_ln();
    }
    print_cstr(str_ptr);
}

// ---------------------------------------------------------------------------
// print_esc_cstr
// ---------------------------------------------------------------------------

/// Print the escape character (if any) followed by a C string.
///
/// translated from xetex-output.c:338 (print_esc_cstr)
#[no_mangle]
pub unsafe extern "C" fn print_esc_cstr(s: *const libc::c_char) {
    let c = eqtb_b32s1(INT_BASE + INT_PAR_ESCAPE_CHAR);
    if (0..=BIGGEST_USV).contains(&c) {
        print_char(c);
    }
    print_cstr(s);
}

// ---------------------------------------------------------------------------
// print_nl
// ---------------------------------------------------------------------------

/// Start a new line if needed, then print string `s`.
///
/// translated from xetex-output.c:302 (print_nl)
#[no_mangle]
pub unsafe extern "C" fn print_nl(s: i32) {
    let sel = selector();
    if ((term_offset() > 0) && (sel & 1 != 0))
        || ((file_offset() > 0) && (sel >= SELECTOR_LOG_ONLY))
    {
        print_ln();
    }
    print(s);
}

// ---------------------------------------------------------------------------
// print_esc
// ---------------------------------------------------------------------------

/// Print the escape character (if any) followed by string `s`.
///
/// translated from xetex-output.c:310 (print_esc)
#[no_mangle]
pub unsafe extern "C" fn print_esc(s: i32) {
    let c = eqtb_b32s1(INT_BASE + INT_PAR_ESCAPE_CHAR);
    if (0..=BIGGEST_USV).contains(&c) {
        print_char(c);
    }
    print(s);
}

// ---------------------------------------------------------------------------
// print_two
// ---------------------------------------------------------------------------

/// Print a two-digit decimal number (last two decimal digits of |n|).
///
/// translated from xetex-output.c:554 (print_two)
#[no_mangle]
pub unsafe extern "C" fn print_two(n: i32) {
    let n = n.abs() % 100;
    print_char(b'0' as i32 + n / 10);
    print_char(b'0' as i32 + n % 10);
}

// ---------------------------------------------------------------------------
// print_roman_int
// ---------------------------------------------------------------------------

/// Print `n` as a Roman numeral (lowercase).
///
/// translated from xetex-output.c:563 (print_roman_int)
#[no_mangle]
pub unsafe extern "C" fn print_roman_int(mut n: i32) {
    // "m2d5c2l5x2v5i": alternating letter/divisor pairs encoding the
    // standard Roman numeral values and their subtraction thresholds.
    const ROMAN_DATA: &[u8] = b"m2d5c2l5x2v5i";
    let mut j: usize = 0;
    let mut v: i32 = 1000;
    loop {
        while n >= v {
            print_char(ROMAN_DATA[j] as i32);
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
            print_char(ROMAN_DATA[k] as i32);
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
#[no_mangle]
pub unsafe extern "C" fn print_scaled(mut s: i32) {
    if s < 0 {
        print_char(b'-' as i32);
        s = -s;
    }
    print_int(s / 0x10000);
    print_char(b'.' as i32);
    s = 10 * (s % 0x10000) + 5;
    let mut delta: i32 = 10;
    loop {
        if delta > 0x10000 {
            s = s + 0x8000 - 50000;
        }
        print_char(b'0' as i32 + (s / 0x10000));
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
#[no_mangle]
pub unsafe extern "C" fn warn_char(c: i32) {
    let diag = current_diagnostic();
    if !diag.is_null() {
        let bytes: [u8; 2] = [c as u8, 0];
        ttbc_diag_append(diag, bytes.as_ptr() as *const libc::c_char);
    }
}

// ---------------------------------------------------------------------------
// print_ln
// ---------------------------------------------------------------------------

/// Emit a newline to the current output destination(s).
///
/// translated from xetex-output.c:84 (print_ln)
#[no_mangle]
pub unsafe extern "C" fn print_ln() {
    match selector() {
        SELECTOR_TERM_AND_LOG => {
            warn_char(b'\n' as i32);
            ttstub_output_putc(rust_stdout(), b'\n' as i32);
            ttstub_output_putc(log_file(), b'\n' as i32);
            set_term_offset(0);
            set_file_offset(0);
        }
        SELECTOR_LOG_ONLY => {
            warn_char(b'\n' as i32);
            ttstub_output_putc(log_file(), b'\n' as i32);
            set_file_offset(0);
        }
        SELECTOR_TERM_ONLY => {
            warn_char(b'\n' as i32);
            ttstub_output_putc(rust_stdout(), b'\n' as i32);
            set_term_offset(0);
        }
        SELECTOR_NO_PRINT | SELECTOR_PSEUDO | SELECTOR_NEW_STRING => {}
        sel => {
            ttstub_output_putc(write_file(sel as usize), b'\n' as i32);
        }
    }
}

// ---------------------------------------------------------------------------
// print_raw_char
// ---------------------------------------------------------------------------

/// Write one UTF-16 code unit to the current output destination(s).
///
/// translated from xetex-output.c:76 (print_raw_char)
#[no_mangle]
pub unsafe extern "C" fn print_raw_char(s: u16, incr_offset: bool) {
    match selector() {
        SELECTOR_TERM_AND_LOG => {
            warn_char(s as i32);
            ttstub_output_putc(rust_stdout(), s as i32);
            ttstub_output_putc(log_file(), s as i32);
            if incr_offset {
                set_term_offset(term_offset() + 1);
                set_file_offset(file_offset() + 1);
            }
            if term_offset() == max_print_line() {
                ttstub_output_putc(rust_stdout(), b'\n' as i32);
                set_term_offset(0);
            }
            if file_offset() == max_print_line() {
                ttstub_output_putc(log_file(), b'\n' as i32);
                set_file_offset(0);
            }
        }
        SELECTOR_LOG_ONLY => {
            warn_char(s as i32);
            ttstub_output_putc(log_file(), s as i32);
            if incr_offset {
                set_file_offset(file_offset() + 1);
            }
            if file_offset() == max_print_line() {
                ttstub_output_putc(log_file(), b'\n' as i32);
                set_file_offset(0);
            }
        }
        SELECTOR_TERM_ONLY => {
            warn_char(s as i32);
            ttstub_output_putc(rust_stdout(), s as i32);
            if incr_offset {
                set_term_offset(term_offset() + 1);
            }
            if term_offset() == max_print_line() {
                ttstub_output_putc(rust_stdout(), b'\n' as i32);
                set_term_offset(0);
            }
        }
        SELECTOR_NO_PRINT => {}
        SELECTOR_PSEUDO => {
            let t = tally();
            if t < trick_count() {
                set_trick_buf((t % error_line()) as usize, s);
            }
        }
        SELECTOR_NEW_STRING => {
            let pp = pool_ptr();
            if pp < pool_size() {
                set_str_pool(pp, s);
                set_pool_ptr(pp + 1);
            }
        }
        sel => {
            ttstub_output_putc(write_file(sel as usize), s as i32);
        }
    }
    set_tally(tally() + 1);
}

// ---------------------------------------------------------------------------
// print_char
// ---------------------------------------------------------------------------

/// Print a single Unicode code point `s` to the current output destination.
///
/// translated from xetex-output.c:76 (print_char)
#[no_mangle]
pub unsafe extern "C" fn print_char(s: i32) {
    // Fast path for NEW_STRING: store as UTF-16 code unit(s).
    if selector() > SELECTOR_PSEUDO && !doing_special() {
        if s >= 0x10000 {
            print_raw_char((0xD800 + (s - 0x10000) / 1024) as u16, true);
            print_raw_char((0xDC00 + (s - 0x10000) % 1024) as u16, true);
        } else {
            print_raw_char(s as u16, true);
        }
        return;
    }

    // Newline character substitution.
    if s == eqtb_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR) && selector() < SELECTOR_PSEUDO {
        print_ln();
        return;
    }

    // Encode for terminal/log output.
    if s < 32 && !doing_special() {
        // C0 controls: ^^X notation.
        print_raw_char(b'^' as u16, true);
        print_raw_char(b'^' as u16, true);
        print_raw_char((s + 64) as u16, true);
    } else if s < 127 {
        print_raw_char(s as u16, true);
    } else if s == 127 {
        if !doing_special() {
            print_raw_char(b'^' as u16, true);
            print_raw_char(b'^' as u16, true);
            print_raw_char(b'?' as u16, true);
        } else {
            print_raw_char(s as u16, true);
        }
    } else if s < 160 && !doing_special() {
        // C1 controls and 128–159: ^^XX hex notation.
        print_raw_char(b'^' as u16, true);
        print_raw_char(b'^' as u16, true);
        let hi = (s % 256) / 16;
        if hi < 10 {
            print_raw_char((b'0' as i32 + hi) as u16, true);
        } else {
            print_raw_char((b'a' as i32 + hi - 10) as u16, true);
        }
        let lo = s % 16;
        if lo < 10 {
            print_raw_char((b'0' as i32 + lo) as u16, true);
        } else {
            print_raw_char((b'a' as i32 + lo - 10) as u16, true);
        }
    } else if selector() == SELECTOR_PSEUDO {
        // In pseudo mode, store raw code unit regardless of value.
        print_raw_char(s as u16, true);
    } else {
        // Non-ASCII (s >= 160) heading to terminal/log: emit as UTF-8.
        if s < 2048 {
            print_raw_char((192 + s / 64) as u16, false);
            print_raw_char((128 + s % 64) as u16, true);
        } else if s < 0x10000 {
            print_raw_char((224 + s / 4096) as u16, false);
            print_raw_char((128 + (s % 4096) / 64) as u16, false);
            print_raw_char((128 + s % 64) as u16, true);
        } else {
            print_raw_char((240 + s / 0x40000) as u16, false);
            print_raw_char((128 + (s % 0x40000) / 4096) as u16, false);
            print_raw_char((128 + (s % 4096) / 64) as u16, false);
            print_raw_char((128 + s % 64) as u16, true);
        }
    }
}

// ---------------------------------------------------------------------------
// print
// ---------------------------------------------------------------------------

/// Print string number `s` from the TeX string pool (or a single character).
///
/// translated from xetex-output.c:146 (print)
#[no_mangle]
pub unsafe extern "C" fn print(s: i32) {
    if s >= str_ptr() {
        return print_cstr(c"???".as_ptr());
    }
    if s <= BIGGEST_CHAR {
        if s < 0 {
            return print_cstr(c"???".as_ptr());
        }
        // Single-character string number.
        if selector() > SELECTOR_PSEUDO {
            print_char(s);
            return;
        }
        if s == eqtb_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR) && selector() < SELECTOR_PSEUDO {
            print_ln();
            return;
        }
        // Temporarily suppress newline-char substitution while printing s.
        let nl = eqtb_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR);
        set_eqtb_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR, -1);
        print_char(s);
        set_eqtb_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR, nl);
        return;
    }

    // String pool reference: s >= 0x10000.
    let pool_idx = s - 0x10000;
    let mut i = str_start(pool_idx);
    let end = str_start(pool_idx + 1);
    while i < end {
        let c = str_pool_entry(i);
        if (0xD800..0xDC00).contains(&c) && i + 1 < end {
            let c2 = str_pool_entry(i + 1);
            if (0xDC00..0xE000).contains(&c2) {
                print_char(0x10000 + (c as i32 - 0xD800) * 1024 + c2 as i32 - 0xDC00);
                i += 2;
                continue;
            }
        }
        print_char(c as i32);
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// print_current_string
// ---------------------------------------------------------------------------

/// Print the currently-accumulating string (in the string pool scratch area).
///
/// translated from xetex-output.c:328 (print_current_string)
#[no_mangle]
pub unsafe extern "C" fn print_current_string() {
    let mut j = str_start(str_ptr() - 0x10000);
    while j < pool_ptr() {
        print_char(str_pool_entry(j) as i32);
        j += 1;
    }
}

// ---------------------------------------------------------------------------
// capture_to_diagnostic
// ---------------------------------------------------------------------------

/// Set the active diagnostic capture target, finishing the previous one.
///
/// translated from xetex-output.c:14 (capture_to_diagnostic)
#[no_mangle]
pub unsafe extern "C" fn capture_to_diagnostic(diagnostic: *mut std::ffi::c_void) {
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
        ttbc_diag_append(diagnostic, c"!".as_ptr());
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
#[no_mangle]
pub unsafe extern "C" fn diagnostic_begin_capture_warning_here() -> *mut std::ffi::c_void {
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
#[no_mangle]
pub unsafe extern "C" fn error_here_with_diagnostic(
    message: *const libc::c_char,
) -> *mut std::ffi::c_void {
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
#[no_mangle]
pub unsafe extern "C" fn print_file_line() {
    let mut level = in_open();
    while level > 0 && full_source_filename_stack(level) == 0 {
        level -= 1;
    }
    if level == 0 {
        print_nl_cstr(c"! ".as_ptr());
    } else {
        print_nl_cstr(c"".as_ptr());
        print(full_source_filename_stack(level));
        print_char(b':' as i32);
        if level == in_open() {
            print_int(line());
        } else {
            print_int(line_stack(level + 1));
        }
        print_cstr(c": ".as_ptr());
    }
}

// ---------------------------------------------------------------------------
// print_size
// ---------------------------------------------------------------------------

/// Print the name of a math font-size selector.
///
/// translated from xetex-output.c:233 (print_size)
#[no_mangle]
pub unsafe extern "C" fn print_size(s: i32) {
    if s == TEXT_SIZE {
        print_esc_cstr(c"textfont".as_ptr());
    } else if s == SCRIPT_SIZE {
        print_esc_cstr(c"scriptfont".as_ptr());
    } else {
        print_esc_cstr(c"scriptscriptfont".as_ptr());
    }
}

// ---------------------------------------------------------------------------
// print_write_whatsit
// ---------------------------------------------------------------------------

/// Print a write-whatsit node's label and stream number.
///
/// translated from xetex-output.c:245 (print_write_whatsit)
#[no_mangle]
pub unsafe extern "C" fn print_write_whatsit(s: *const libc::c_char, p: i32) {
    print_esc_cstr(s);
    let v = mem_b32s0(p + 1);
    if v < 16 {
        print_int(v);
    } else if v == 16 {
        print_char(b'*' as i32);
    } else {
        print_char(b'-' as i32);
    }
}

// ---------------------------------------------------------------------------
// print_file_name
// ---------------------------------------------------------------------------

/// Print a filename composed of area `a`, name `n`, and extension `e`,
/// quoting if needed.
///
/// translated from xetex-output.c:128 (print_file_name)
#[no_mangle]
pub unsafe extern "C" fn print_file_name(n: i32, a: i32, e: i32) {
    // Scan all three string components to decide if quoting is needed.
    let mut must_quote = false;
    let mut quote_char: i32 = 0;

    for s in [a, n, e] {
        if s != 0 {
            let mut j = str_start(s - 0x10000);
            let end = str_start((s + 1) - 0x10000);
            while (!must_quote || quote_char == 0) && j < end {
                let c = str_pool_entry(j) as i32;
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
        print_char(quote_char);
    }

    // Print each component with quote-char escaping.
    for s in [a, n, e] {
        if s != 0 {
            let mut j = str_start(s - 0x10000);
            let for_end = str_start((s + 1) - 0x10000) - 1;
            if j <= for_end {
                loop {
                    let c = str_pool_entry(j) as i32;
                    if c == quote_char {
                        print(quote_char);
                        quote_char = 73 - quote_char;
                        print(quote_char);
                    }
                    print(c);
                    if j >= for_end {
                        break;
                    }
                    j += 1;
                }
            }
        }
    }

    if quote_char != 0 {
        print_char(quote_char);
    }
}

// ---------------------------------------------------------------------------
// print_cs
// ---------------------------------------------------------------------------

/// Print a control-sequence name for control-sequence pointer `p`.
///
/// translated from xetex-output.c:76 (print_cs)
#[no_mangle]
pub unsafe extern "C" fn print_cs(p: i32) {
    if p < HASH_BASE {
        if p >= SINGLE_BASE {
            if p == NULL_CS {
                print_esc_cstr(c"csname".as_ptr());
                print_esc_cstr(c"endcsname".as_ptr());
                print_char(b' ' as i32);
            } else {
                print_esc(p - SINGLE_BASE);
                if eqtb_b32s1(CAT_CODE_BASE + p - SINGLE_BASE) == LETTER {
                    print_char(b' ' as i32);
                }
            }
        } else if p < ACTIVE_BASE {
            print_esc_cstr(c"IMPOSSIBLE.".as_ptr());
        } else {
            print_char(p - 1);
        }
    } else if (UNDEFINED_CONTROL_SEQUENCE..=EQTB_SIZE).contains(&p) || p > eqtb_top() {
        print_esc_cstr(c"IMPOSSIBLE.".as_ptr());
    } else if hash_s1(p) >= str_ptr() {
        print_esc_cstr(c"NONEXISTENT.".as_ptr());
    } else {
        if (PRIM_EQTB_BASE..FROZEN_NULL_FONT).contains(&p) {
            print_esc(prim_s1(p - PRIM_EQTB_BASE) - 1);
        } else {
            print_esc(hash_s1(p));
        }
        print_char(b' ' as i32);
    }
}

// ---------------------------------------------------------------------------
// sprint_cs
// ---------------------------------------------------------------------------

/// Print a control-sequence name without trailing space (used in error msgs).
///
/// translated from xetex-output.c:108 (sprint_cs)
#[no_mangle]
pub unsafe extern "C" fn sprint_cs(p: i32) {
    if p < HASH_BASE {
        if p < SINGLE_BASE {
            print_char(p - 1);
        } else if p < NULL_CS {
            print_esc(p - SINGLE_BASE);
        } else {
            print_esc_cstr(c"csname".as_ptr());
            print_esc_cstr(c"endcsname".as_ptr());
        }
    } else if (PRIM_EQTB_BASE..FROZEN_NULL_FONT).contains(&p) {
        print_esc(prim_s1(p - PRIM_EQTB_BASE) - 1);
    } else {
        print_esc(hash_s1(p));
    }
}

// ---------------------------------------------------------------------------
// print_native_word
// ---------------------------------------------------------------------------

/// Print the UTF-16 text of a native-word node.
///
/// translated from xetex-output.c:260 (print_native_word)
#[no_mangle]
pub unsafe extern "C" fn print_native_word(p: i32) {
    // NATIVE_NODE_text(p) = (unsigned short *) &mem[p + NATIVE_NODE_SIZE]
    // NATIVE_NODE_SIZE = 6 (xetex-constants.h:130)
    // mem[p+4].b16.s1 = character count
    let for_end = mem_b16s1(p + 4) as i32 - 1;
    let text_ptr = mem_as_u16_ptr(p + 6);
    let mut i: i32 = 0;
    while i <= for_end {
        let c = *text_ptr.offset(i as isize) as i32;
        if (0xD800..0xDC00).contains(&c) && i < for_end {
            let cc = *text_ptr.offset(i as isize + 1) as i32;
            if (0xDC00..0xE000).contains(&cc) {
                print_char(0x10000 + (c - 0xD800) * 1024 + (cc - 0xDC00));
                i += 2;
                continue;
            }
        }
        if (0xD800..0xDC00).contains(&c) {
            print(b'.' as i32);
        } else {
            print_char(c);
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// print_sa_num
// ---------------------------------------------------------------------------

/// Print the sparse-array register number for node `q`.
///
/// translated from xetex-output.c:285 (print_sa_num)
#[no_mangle]
pub unsafe extern "C" fn print_sa_num(q: i32) {
    let n = if (mem_b16s1(q) as i32) < DIMEN_VAL_LIMIT {
        mem_b32s1(q + 1)
    } else {
        let n0 = mem_b16s1(q) as i32 % 64;
        let q1 = mem_b32s1(q);
        let n1 = mem_b16s1(q1) as i32;
        let q2 = mem_b32s1(q1);
        let b16s1_q2 = mem_b16s1(q2) as i32;
        let q3 = mem_b32s1(q2);
        let b16s1_q3 = mem_b16s1(q3) as i32;
        n0 + 64 * n1 + 64 * 64 * (b16s1_q2 + 64 * b16s1_q3)
    };
    print_int(n);
}

// ---------------------------------------------------------------------------
// begin_diagnostic / end_diagnostic
// ---------------------------------------------------------------------------

/// Save selector and switch to log-only mode for diagnostic output.
///
/// translated from xetex-xetex0.c:1785 (begin_diagnostic)
#[no_mangle]
pub unsafe extern "C" fn begin_diagnostic() {
    set_old_setting(selector() as u8);
    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) <= 0 && selector() == SELECTOR_TERM_AND_LOG {
        set_selector(selector() - 1);
        if history() == HISTORY_SPOTLESS {
            set_history(HISTORY_WARNING_ISSUED);
        }
    }
}

/// Restore selector saved by begin_diagnostic.
///
/// translated from xetex-xetex0.c:1797 (end_diagnostic)
#[no_mangle]
pub unsafe extern "C" fn end_diagnostic(blank_line: bool) {
    print_nl_cstr(c"".as_ptr());
    if blank_line {
        print_ln();
    }
    set_selector(old_setting() as i32);
}

// ---------------------------------------------------------------------------
// print_mode / print_in_mode
// ---------------------------------------------------------------------------

/// Print a textual description of the current TeX mode m.
///
/// translated from xetex-xetex0.c:1327 (print_mode)
#[no_mangle]
pub unsafe extern "C" fn print_mode(m: i32) {
    if m > 0 {
        match m / MAX_COMMAND_PLUS_1 {
            0 => print_cstr(c"vertical mode".as_ptr()),
            1 => print_cstr(c"horizontal mode".as_ptr()),
            2 => print_cstr(c"display math mode".as_ptr()),
            _ => {}
        }
    } else if m == 0 {
        print_cstr(c"no mode".as_ptr());
    } else {
        match (-m) / MAX_COMMAND_PLUS_1 {
            0 => print_cstr(c"internal vertical mode".as_ptr()),
            1 => print_cstr(c"restricted horizontal mode".as_ptr()),
            2 => print_cstr(c"math mode".as_ptr()),
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
#[no_mangle]
pub unsafe extern "C" fn print_group(e: bool) {
    let g = cur_group();
    match g {
        BOTTOM_LEVEL => {
            print_cstr(c"bottom level".as_ptr());
            return;
        }
        SIMPLE_GROUP | SEMI_SIMPLE_GROUP => {
            if g == SEMI_SIMPLE_GROUP {
                print_cstr(c"semi ".as_ptr());
            }
            print_cstr(c"simple".as_ptr());
        }
        HBOX_GROUP | ADJUSTED_HBOX_GROUP => {
            if g == ADJUSTED_HBOX_GROUP {
                print_cstr(c"adjusted ".as_ptr());
            }
            print_cstr(c"hbox".as_ptr());
        }
        VBOX_GROUP => print_cstr(c"vbox".as_ptr()),
        VTOP_GROUP => print_cstr(c"vtop".as_ptr()),
        ALIGN_GROUP | NO_ALIGN_GROUP => {
            if g == NO_ALIGN_GROUP {
                print_cstr(c"no ".as_ptr());
            }
            print_cstr(c"align".as_ptr());
        }
        OUTPUT_GROUP => print_cstr(c"output".as_ptr()),
        DISC_GROUP => print_cstr(c"disc".as_ptr()),
        INSERT_GROUP => print_cstr(c"insert".as_ptr()),
        VCENTER_GROUP => print_cstr(c"vcenter".as_ptr()),
        MATH_GROUP | MATH_CHOICE_GROUP | MATH_SHIFT_GROUP | MATH_LEFT_GROUP => {
            print_cstr(c"math".as_ptr());
            if g == MATH_CHOICE_GROUP {
                print_cstr(c" choice".as_ptr());
            } else if g == MATH_SHIFT_GROUP {
                print_cstr(c" shift".as_ptr());
            } else if g == MATH_LEFT_GROUP {
                print_cstr(c" left".as_ptr());
            }
        }
        _ => {}
    }
    print_cstr(c" group (level ".as_ptr());
    print_int(cur_level() as i32);
    print_char(')' as i32);
    let line_at = save_stack_b32s1(save_ptr() - 1);
    if line_at != 0 {
        if e {
            print_cstr(c" entered at line ".as_ptr());
        } else {
            print_cstr(c" at line ".as_ptr());
        }
        print_int(line_at);
    }
}

/// Print "' in <mode>" for use in error messages.
///
/// translated from xetex-xetex0.c:1356 (print_in_mode)
#[no_mangle]
pub unsafe extern "C" fn print_in_mode(m: i32) {
    if m > 0 {
        match m / MAX_COMMAND_PLUS_1 {
            0 => print_cstr(c"' in vertical mode".as_ptr()),
            1 => print_cstr(c"' in horizontal mode".as_ptr()),
            2 => print_cstr(c"' in display math mode".as_ptr()),
            _ => {}
        }
    } else if m == 0 {
        print_cstr(c"' in no mode".as_ptr());
    } else {
        match (-m) / MAX_COMMAND_PLUS_1 {
            0 => print_cstr(c"' in internal vertical mode".as_ptr()),
            1 => print_cstr(c"' in restricted horizontal mode".as_ptr()),
            2 => print_cstr(c"' in math mode".as_ptr()),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// token_show
// ---------------------------------------------------------------------------

const TEX_NULL: i32 = -0x0FFF_FFFF;

/// Display a token list headed at p.
///
/// translated from xetex-xetex0.c:3893 (token_show)
#[no_mangle]
pub unsafe extern "C" fn token_show(p: i32) {
    if p != TEX_NULL {
        show_token_list(mem_b32s1(p), TEX_NULL, 10_000_000);
    }
}

// ---------------------------------------------------------------------------
// print_spec
// ---------------------------------------------------------------------------

/// Print a glue spec node as "width plus stretch minus shrink".
///
/// translated from xetex-xetex0.c:274 (print_spec)
#[no_mangle]
pub unsafe extern "C" fn print_spec(p: i32, s: *const libc::c_char) {
    if p < 0 || p >= lo_mem_max() {
        print_char('*' as i32);
    } else {
        print_scaled(mem_b32s1(p + 1));
        if !s.is_null() {
            print_cstr(s);
        }
        if mem_b32s1(p + 2) != 0 {
            print_cstr(c" plus ".as_ptr());
            print_glue(mem_b32s1(p + 2), mem_b16s1(p) as i32, s);
        }
        if mem_b32s1(p + 3) != 0 {
            print_cstr(c" minus ".as_ptr());
            print_glue(mem_b32s1(p + 3), mem_b16s0(p) as i32, s);
        }
    }
}

// ---------------------------------------------------------------------------
// print_skip_param / print_param / print_length_param
// ---------------------------------------------------------------------------

/// Print the name of glue parameter n (GLUE_PAR__*).
///
/// translated from xetex-xetex0.c:379 (print_skip_param)
#[no_mangle]
pub unsafe extern "C" fn print_skip_param(n: i32) {
    match n {
        0 => print_esc_cstr(c"lineskip".as_ptr()),
        1 => print_esc_cstr(c"baselineskip".as_ptr()),
        2 => print_esc_cstr(c"parskip".as_ptr()),
        3 => print_esc_cstr(c"abovedisplayskip".as_ptr()),
        4 => print_esc_cstr(c"belowdisplayskip".as_ptr()),
        5 => print_esc_cstr(c"abovedisplayshortskip".as_ptr()),
        6 => print_esc_cstr(c"belowdisplayshortskip".as_ptr()),
        7 => print_esc_cstr(c"leftskip".as_ptr()),
        8 => print_esc_cstr(c"rightskip".as_ptr()),
        9 => print_esc_cstr(c"topskip".as_ptr()),
        10 => print_esc_cstr(c"splittopskip".as_ptr()),
        11 => print_esc_cstr(c"tabskip".as_ptr()),
        12 => print_esc_cstr(c"spaceskip".as_ptr()),
        13 => print_esc_cstr(c"xspaceskip".as_ptr()),
        14 => print_esc_cstr(c"parfillskip".as_ptr()),
        15 => print_esc_cstr(c"XeTeXlinebreakskip".as_ptr()),
        16 => print_esc_cstr(c"thinmuskip".as_ptr()),
        17 => print_esc_cstr(c"medmuskip".as_ptr()),
        18 => print_esc_cstr(c"thickmuskip".as_ptr()),
        _ => print_cstr(c"[unknown glue parameter!]".as_ptr()),
    }
}

/// Print the name of integer parameter n (INT_PAR__*).
///
/// translated from xetex-xetex0.c:1423 (print_param)
#[no_mangle]
pub unsafe extern "C" fn print_param(n: i32) {
    match n {
        0 => print_esc_cstr(c"pretolerance".as_ptr()),
        1 => print_esc_cstr(c"tolerance".as_ptr()),
        2 => print_esc_cstr(c"linepenalty".as_ptr()),
        3 => print_esc_cstr(c"hyphenpenalty".as_ptr()),
        4 => print_esc_cstr(c"exhyphenpenalty".as_ptr()),
        5 => print_esc_cstr(c"clubpenalty".as_ptr()),
        6 => print_esc_cstr(c"widowpenalty".as_ptr()),
        7 => print_esc_cstr(c"displaywidowpenalty".as_ptr()),
        8 => print_esc_cstr(c"brokenpenalty".as_ptr()),
        9 => print_esc_cstr(c"binoppenalty".as_ptr()),
        10 => print_esc_cstr(c"relpenalty".as_ptr()),
        11 => print_esc_cstr(c"predisplaypenalty".as_ptr()),
        12 => print_esc_cstr(c"postdisplaypenalty".as_ptr()),
        13 => print_esc_cstr(c"interlinepenalty".as_ptr()),
        14 => print_esc_cstr(c"doublehyphendemerits".as_ptr()),
        15 => print_esc_cstr(c"finalhyphendemerits".as_ptr()),
        16 => print_esc_cstr(c"adjdemerits".as_ptr()),
        17 => print_esc_cstr(c"mag".as_ptr()),
        18 => print_esc_cstr(c"delimiterfactor".as_ptr()),
        19 => print_esc_cstr(c"looseness".as_ptr()),
        20 => print_esc_cstr(c"time".as_ptr()),
        21 => print_esc_cstr(c"day".as_ptr()),
        22 => print_esc_cstr(c"month".as_ptr()),
        23 => print_esc_cstr(c"year".as_ptr()),
        24 => print_esc_cstr(c"showboxbreadth".as_ptr()),
        25 => print_esc_cstr(c"showboxdepth".as_ptr()),
        26 => print_esc_cstr(c"hbadness".as_ptr()),
        27 => print_esc_cstr(c"vbadness".as_ptr()),
        28 => print_esc_cstr(c"pausing".as_ptr()),
        29 => print_esc_cstr(c"tracingonline".as_ptr()),
        30 => print_esc_cstr(c"tracingmacros".as_ptr()),
        31 => print_esc_cstr(c"tracingstats".as_ptr()),
        32 => print_esc_cstr(c"tracingparagraphs".as_ptr()),
        33 => print_esc_cstr(c"tracingpages".as_ptr()),
        34 => print_esc_cstr(c"tracingoutput".as_ptr()),
        35 => print_esc_cstr(c"tracinglostchars".as_ptr()),
        36 => print_esc_cstr(c"tracingcommands".as_ptr()),
        37 => print_esc_cstr(c"tracingrestores".as_ptr()),
        38 => print_esc_cstr(c"uchyph".as_ptr()),
        39 => print_esc_cstr(c"outputpenalty".as_ptr()),
        40 => print_esc_cstr(c"maxdeadcycles".as_ptr()),
        41 => print_esc_cstr(c"hangafter".as_ptr()),
        42 => print_esc_cstr(c"floatingpenalty".as_ptr()),
        43 => print_esc_cstr(c"globaldefs".as_ptr()),
        44 => print_esc_cstr(c"fam".as_ptr()),
        45 => print_esc_cstr(c"escapechar".as_ptr()),
        46 => print_esc_cstr(c"defaulthyphenchar".as_ptr()),
        47 => print_esc_cstr(c"defaultskewchar".as_ptr()),
        48 => print_esc_cstr(c"endlinechar".as_ptr()),
        49 => print_esc_cstr(c"newlinechar".as_ptr()),
        50 => print_esc_cstr(c"language".as_ptr()),
        51 => print_esc_cstr(c"lefthyphenmin".as_ptr()),
        52 => print_esc_cstr(c"righthyphenmin".as_ptr()),
        53 => print_esc_cstr(c"holdinginserts".as_ptr()),
        54 => print_esc_cstr(c"errorcontextlines".as_ptr()),
        55 => print_esc_cstr(c"tracingstacklevels".as_ptr()),
        56 => print_esc_cstr(c"tracingassigns".as_ptr()),
        57 => print_esc_cstr(c"tracinggroups".as_ptr()),
        58 => print_esc_cstr(c"tracingifs".as_ptr()),
        59 => print_esc_cstr(c"tracingscantokens".as_ptr()),
        60 => print_esc_cstr(c"tracingnesting".as_ptr()),
        61 => print_esc_cstr(c"predisplaydirection".as_ptr()),
        62 => print_esc_cstr(c"lastlinefit".as_ptr()),
        63 => print_esc_cstr(c"savingvdiscards".as_ptr()),
        64 => print_esc_cstr(c"savinghyphcodes".as_ptr()),
        65 => print_esc_cstr(c"suppressfontnotfounderror".as_ptr()),
        // 66 = xetex_linebreak_locale — not in C switch (no matching case)
        67 => print_esc_cstr(c"XeTeXlinebreakpenalty".as_ptr()),
        68 => print_esc_cstr(c"XeTeXprotrudechars".as_ptr()),
        69 => print_esc_cstr(c"TeXXeTstate".as_ptr()),
        70 => print_esc_cstr(c"XeTeXdashbreakstate".as_ptr()),
        71 => print_esc_cstr(c"XeTeXupwardsmode".as_ptr()),
        72 => print_esc_cstr(c"XeTeXuseglyphmetrics".as_ptr()),
        73 => print_esc_cstr(c"XeTeXinterchartokenstate".as_ptr()),
        74 => print_esc_cstr(c"XeTeXinputnormalization".as_ptr()),
        77 => print_esc_cstr(c"XeTeXtracingfonts".as_ptr()),
        78 => print_esc_cstr(c"XeTeXinterwordspaceshaping".as_ptr()),
        79 => print_esc_cstr(c"XeTeXgenerateactualtext".as_ptr()),
        80 => print_esc_cstr(c"XeTeXhyphenatablelength".as_ptr()),
        81 => print_esc_cstr(c"synctex".as_ptr()),
        82 => print_esc_cstr(c"pdfoutput".as_ptr()),
        _ => print_cstr(c"[unknown int32_t parameter!]".as_ptr()),
    }
}

// ---------------------------------------------------------------------------
// print_style / print_fam_and_char / print_delimiter
// ---------------------------------------------------------------------------

/// Print the math style name for style code c.
///
/// translated from xetex-xetex0.c:319 (print_style)
#[no_mangle]
pub unsafe extern "C" fn print_style(c: i32) {
    match c / 2 {
        0 => print_esc_cstr(c"displaystyle".as_ptr()), // DISPLAY_STYLE / 2
        1 => print_esc_cstr(c"textstyle".as_ptr()),    // TEXT_STYLE / 2
        2 => print_esc_cstr(c"scriptstyle".as_ptr()),  // SCRIPT_STYLE / 2
        3 => print_esc_cstr(c"scriptscriptstyle".as_ptr()), // SCRIPT_SCRIPT_STYLE / 2
        _ => print_cstr(c"Unknown style!".as_ptr()),
    }
}

/// Print the fam number and character code stored in a math node p.
///
/// translated from xetex-xetex0.c:294 (print_fam_and_char)
#[no_mangle]
pub unsafe extern "C" fn print_fam_and_char(p: i32) {
    print_esc_cstr(c"fam".as_ptr());
    print_int((mem_b16s1(p) % 256) as i32);
    print_char(' ' as i32);
    let c = (mem_b16s0(p) as i64 + ((mem_b16s1(p) as i64 / 256) * 65536)) as i32;
    if c < 65536 {
        print(c);
    } else {
        print_char(c);
    }
}

/// Print a delimiter code (four 16-bit fields packed as a hex number).
///
/// translated from xetex-xetex0.c:307 (print_delimiter)
#[no_mangle]
pub unsafe extern "C" fn print_delimiter(p: i32) {
    let s3 = mem_b16s3(p) as i64;
    let s2 = mem_b16s2(p) as i64;
    let s1 = mem_b16s1(p) as i64;
    let s0 = mem_b16s0(p) as i64;
    let mut a = (s3 % 256) * 256 + (s2 + (s3 / 256) * 65536);
    a = a * 4096 + (s1 % 256) * 256 + (s0 + (s1 / 256) * 65536);
    let a32 = a as i32;
    if a32 < 0 {
        print_int(a32);
    } else {
        print_hex(a32);
    }
}

// ---------------------------------------------------------------------------
// print_rule_dimen / print_glue
// ---------------------------------------------------------------------------

// Glue order constants (xetex-constants.h:173–182).
const NORMAL: i32 = 0;
const FIL: i32 = 1;
const FILLL: i32 = 3;
const NULL_FLAG: i32 = -0x4000_0000;

/// Print a rule dimension (print '*' for a null-flag wildcard).
///
/// translated from xetex-xetex0.c:250 (print_rule_dimen)
#[no_mangle]
pub unsafe extern "C" fn print_rule_dimen(d: i32) {
    if d == NULL_FLAG {
        print_char('*' as i32);
    } else {
        print_scaled(d);
    }
}

/// Print a glue dimension with optional fil/fill/filll suffix.
///
/// translated from xetex-xetex0.c:258 (print_glue)
#[no_mangle]
pub unsafe extern "C" fn print_glue(d: i32, order: i32, s: *const libc::c_char) {
    print_scaled(d);
    if order < NORMAL || order > FILLL {
        print_cstr(c"foul".as_ptr());
    } else if order > NORMAL {
        print_cstr(c"fil".as_ptr());
        let mut ord = order;
        while ord > FIL {
            print_char('l' as i32);
            ord -= 1;
        }
    } else if !s.is_null() {
        print_cstr(s);
    }
}

/// Print the name of dimen parameter n (DIMEN_PAR__*).
///
/// translated from xetex-xetex0.c:1676 (print_length_param)
#[no_mangle]
pub unsafe extern "C" fn print_length_param(n: i32) {
    match n {
        0 => print_esc_cstr(c"parindent".as_ptr()),
        1 => print_esc_cstr(c"mathsurround".as_ptr()),
        2 => print_esc_cstr(c"lineskiplimit".as_ptr()),
        3 => print_esc_cstr(c"hsize".as_ptr()),
        4 => print_esc_cstr(c"vsize".as_ptr()),
        5 => print_esc_cstr(c"maxdepth".as_ptr()),
        6 => print_esc_cstr(c"splitmaxdepth".as_ptr()),
        7 => print_esc_cstr(c"boxmaxdepth".as_ptr()),
        8 => print_esc_cstr(c"hfuzz".as_ptr()),
        9 => print_esc_cstr(c"vfuzz".as_ptr()),
        10 => print_esc_cstr(c"delimitershortfall".as_ptr()),
        11 => print_esc_cstr(c"nulldelimiterspace".as_ptr()),
        12 => print_esc_cstr(c"scriptspace".as_ptr()),
        13 => print_esc_cstr(c"predisplaysize".as_ptr()),
        14 => print_esc_cstr(c"displaywidth".as_ptr()),
        15 => print_esc_cstr(c"displayindent".as_ptr()),
        16 => print_esc_cstr(c"overfullrule".as_ptr()),
        17 => print_esc_cstr(c"hangindent".as_ptr()),
        18 => print_esc_cstr(c"hoffset".as_ptr()),
        19 => print_esc_cstr(c"voffset".as_ptr()),
        20 => print_esc_cstr(c"emergencystretch".as_ptr()),
        21 => print_esc_cstr(c"pdfpagewidth".as_ptr()),
        22 => print_esc_cstr(c"pdfpageheight".as_ptr()),
        _ => print_cstr(c"[unknown dimen parameter!]".as_ptr()),
    }
}

// ---------------------------------------------------------------------------
// print_font_and_char
// ---------------------------------------------------------------------------

/// Print the font escape-name and character for a char-node at `p`.
///
/// translated from xetex-xetex0.c:225 (print_font_and_char)
#[no_mangle]
pub unsafe extern "C" fn print_font_and_char(p: i32) {
    if p > mem_end() {
        print_esc_cstr(c"CLOBBERED.".as_ptr());
    } else {
        if mem_b16s1(p) > font_max() as u16 {
            print_char('*' as i32);
        } else {
            // FONT_ID_BASE + font_id → hash entry → string number of the font name.
            print_esc(hash_s1(FONT_ID_BASE + mem_b16s1(p) as i32));
        }
        print_char(' ' as i32);
        print(mem_b16s0(p) as i32);
    }
}

// ---------------------------------------------------------------------------
// print_mark
// ---------------------------------------------------------------------------

/// Print a mark token list enclosed in `{…}`.
///
/// translated from xetex-xetex0.c:240 (print_mark)
#[no_mangle]
pub unsafe extern "C" fn print_mark(p: i32) {
    print_char('{' as i32);
    if p < hi_mem_min() || p > mem_end() {
        print_esc_cstr(c"CLOBBERED.".as_ptr());
    } else {
        show_token_list(mem_b32s1(p), TEX_NULL, max_print_line() - 10);
    }
    print_char('}' as i32);
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
const CC_LOCAL__par_shape: i32 = 0;
const CC_LOCAL__output_routine: i32 = 1;
const CC_LOCAL__every_par: i32 = 2;
const CC_LOCAL__every_math: i32 = 3;
const CC_LOCAL__every_display: i32 = 4;
const CC_LOCAL__every_hbox: i32 = 5;
const CC_LOCAL__every_vbox: i32 = 6;
const CC_LOCAL__every_job: i32 = 7;
const CC_LOCAL__every_cr: i32 = 8;
const CC_LOCAL__every_eof: i32 = 10;
const CC_LOCAL__xetex_inter_char_toks: i32 = 11;
const CC_LOCAL__tectonic_coda_tokens: i32 = 12;
// ETEX_PENALTIES_PAR__ offsets (xetex_format.h:189–192)
const CC_ETEX_PENALTIES_PAR__inter_line_penalties: i32 = 0;
const CC_ETEX_PENALTIES_PAR__club_penalties: i32 = 1;
const CC_ETEX_PENALTIES_PAR__widow_penalties: i32 = 2;
const CC_ETEX_PENALTIES_PAR__display_widow_penalties: i32 = 3;
// IF_TEST chr_codes (xetex_format.h:544–564)
const CC_IF_CHAR_CODE: i32 = 0;
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
const CC_LOCAL_BASE_PAR_SHAPE: i32 = CC_LOCAL_BASE + CC_LOCAL__par_shape;
const CC_ETEX_PEN_BASE_INTER_LINE: i32 =
    CC_ETEX_PEN_BASE + CC_ETEX_PENALTIES_PAR__inter_line_penalties;
const CC_ETEX_PEN_BASE_CLUB: i32 = CC_ETEX_PEN_BASE + CC_ETEX_PENALTIES_PAR__club_penalties;
const CC_ETEX_PEN_BASE_WIDOW: i32 = CC_ETEX_PEN_BASE + CC_ETEX_PENALTIES_PAR__widow_penalties;
const CC_ETEX_PEN_BASE_DISPLAY_WIDOW: i32 =
    CC_ETEX_PEN_BASE + CC_ETEX_PENALTIES_PAR__display_widow_penalties;
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
#[no_mangle]
pub unsafe extern "C" fn print_cmd_chr(cmd: u16, chr_code: i32) {
    use crate::globals::{font_area_entry, font_dsize_entry, font_name_entry, font_size_entry};

    // Helper: print a char/codepoint (> 65535 uses print_char for raw code).
    let print_char_or_wide = |code: i32| {
        if code < 65536 {
            print(code);
        } else {
            print_char(code);
        }
    };

    match cmd {
        CMD_LEFT_BRACE => {
            print_cstr(c"begin-group character ".as_ptr());
            print_char_or_wide(chr_code);
        }
        CMD_RIGHT_BRACE => {
            print_cstr(c"end-group character ".as_ptr());
            print_char_or_wide(chr_code);
        }
        CMD_MATH_SHIFT => {
            print_cstr(c"math shift character ".as_ptr());
            print_char_or_wide(chr_code);
        }
        CMD_MAC_PARAM => {
            print_cstr(c"macro parameter character ".as_ptr());
            print_char_or_wide(chr_code);
        }
        CMD_SUP_MARK => {
            print_cstr(c"superscript character ".as_ptr());
            print_char_or_wide(chr_code);
        }
        CMD_SUB_MARK => {
            print_cstr(c"subscript character ".as_ptr());
            print_char_or_wide(chr_code);
        }
        CMD_ENDV => {
            print_cstr(c"end of alignment template".as_ptr());
        }
        CMD_SPACER => {
            print_cstr(c"blank space ".as_ptr());
            print_char_or_wide(chr_code);
        }
        CMD_LETTER => {
            print_cstr(c"the letter ".as_ptr());
            print_char_or_wide(chr_code);
        }
        CMD_OTHER_CHAR => {
            print_cstr(c"the character ".as_ptr());
            print_char_or_wide(chr_code);
        }
        CMD_ASSIGN_GLUE | CMD_ASSIGN_MU_GLUE => {
            if chr_code < CC_SKIP_BASE {
                print_skip_param(chr_code - CC_GLUE_BASE);
            } else if chr_code < CC_MU_SKIP_BASE {
                print_esc_cstr(c"skip".as_ptr());
                print_int(chr_code - CC_SKIP_BASE);
            } else {
                print_esc_cstr(c"muskip".as_ptr());
                print_int(chr_code - CC_MU_SKIP_BASE);
            }
        }
        CMD_ASSIGN_TOKS => {
            if chr_code >= CC_TOKS_BASE {
                print_esc_cstr(c"toks".as_ptr());
                print_int(chr_code - CC_TOKS_BASE);
            } else {
                match chr_code {
                    x if x == CC_LOCAL_BASE + CC_LOCAL__output_routine => {
                        print_esc_cstr(c"output".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL__every_par => {
                        print_esc_cstr(c"everypar".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL__every_math => {
                        print_esc_cstr(c"everymath".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL__every_display => {
                        print_esc_cstr(c"everydisplay".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL__every_hbox => {
                        print_esc_cstr(c"everyhbox".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL__every_vbox => {
                        print_esc_cstr(c"everyvbox".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL__every_job => {
                        print_esc_cstr(c"everyjob".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL__every_cr => {
                        print_esc_cstr(c"everycr".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL__every_eof => {
                        print_esc_cstr(c"everyeof".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL__xetex_inter_char_toks => {
                        print_esc_cstr(c"XeTeXinterchartoks".as_ptr())
                    }
                    x if x == CC_LOCAL_BASE + CC_LOCAL__tectonic_coda_tokens => {
                        print_esc_cstr(c"TectonicCodaTokens".as_ptr())
                    }
                    _ => print_esc_cstr(c"errhelp".as_ptr()),
                }
            }
        }
        CMD_ASSIGN_INT => {
            if chr_code < CC_COUNT_BASE {
                print_param(chr_code - CC_INT_BASE);
            } else {
                print_esc_cstr(c"count".as_ptr());
                print_int(chr_code - CC_COUNT_BASE);
            }
        }
        CMD_ASSIGN_DIMEN => {
            if chr_code < CC_SCALED_BASE {
                print_length_param(chr_code - CC_DIMEN_BASE);
            } else {
                print_esc_cstr(c"dimen".as_ptr());
                print_int(chr_code - CC_SCALED_BASE);
            }
        }
        CMD_ACCENT => print_esc_cstr(c"accent".as_ptr()),
        CMD_ADVANCE => print_esc_cstr(c"advance".as_ptr()),
        CMD_AFTER_ASSIGNMENT => print_esc_cstr(c"afterassignment".as_ptr()),
        CMD_AFTER_GROUP => print_esc_cstr(c"aftergroup".as_ptr()),
        CMD_ASSIGN_FONT_DIMEN => print_esc_cstr(c"fontdimen".as_ptr()),
        CMD_BEGIN_GROUP => print_esc_cstr(c"begingroup".as_ptr()),
        CMD_BREAK_PENALTY => print_esc_cstr(c"penalty".as_ptr()),
        CMD_CHAR_NUM => print_esc_cstr(c"char".as_ptr()),
        CMD_CS_NAME => print_esc_cstr(c"csname".as_ptr()),
        CMD_DEF_FONT => print_esc_cstr(c"font".as_ptr()),
        CMD_DELIM_NUM => {
            if chr_code == 1 {
                print_esc_cstr(c"Udelimiter".as_ptr());
            } else {
                print_esc_cstr(c"delimiter".as_ptr());
            }
        }
        CMD_DIVIDE => print_esc_cstr(c"divide".as_ptr()),
        CMD_END_CS_NAME => print_esc_cstr(c"endcsname".as_ptr()),
        CMD_END_GROUP => print_esc_cstr(c"endgroup".as_ptr()),
        CMD_EX_SPACE => print_esc(' ' as i32),
        CMD_EXPAND_AFTER => {
            if chr_code == 0 {
                print_esc_cstr(c"expandafter".as_ptr());
            } else {
                print_esc_cstr(c"unless".as_ptr());
            }
        }
        CMD_HALIGN => print_esc_cstr(c"halign".as_ptr()),
        CMD_HRULE => print_esc_cstr(c"hrule".as_ptr()),
        CMD_IGNORE_SPACES => {
            if chr_code == 0 {
                print_esc_cstr(c"ignorespaces".as_ptr());
            } else {
                print_esc_cstr(c"primitive".as_ptr());
            }
        }
        CMD_INSERT => print_esc_cstr(c"insert".as_ptr()),
        CMD_ITAL_CORR => print_esc('/' as i32),
        CMD_MARK => {
            print_esc_cstr(c"mark".as_ptr());
            if chr_code > 0 {
                print_char('s' as i32);
            }
        }
        CMD_MATH_ACCENT => {
            if chr_code == 1 {
                print_esc_cstr(c"Umathaccent".as_ptr());
            } else {
                print_esc_cstr(c"mathaccent".as_ptr());
            }
        }
        CMD_MATH_CHAR_NUM => {
            if chr_code == 2 {
                print_esc_cstr(c"Umathchar".as_ptr());
            } else if chr_code == 1 {
                print_esc_cstr(c"Umathcharnum".as_ptr());
            } else {
                print_esc_cstr(c"mathchar".as_ptr());
            }
        }
        CMD_MATH_CHOICE => print_esc_cstr(c"mathchoice".as_ptr()),
        CMD_MULTIPLY => print_esc_cstr(c"multiply".as_ptr()),
        CMD_NO_ALIGN => print_esc_cstr(c"noalign".as_ptr()),
        CMD_NO_BOUNDARY => print_esc_cstr(c"noboundary".as_ptr()),
        CMD_NO_EXPAND => {
            if chr_code == 0 {
                print_esc_cstr(c"noexpand".as_ptr());
            } else {
                print_esc_cstr(c"primitive".as_ptr());
            }
        }
        CMD_NON_SCRIPT => print_esc_cstr(c"nonscript".as_ptr()),
        CMD_OMIT => print_esc_cstr(c"omit".as_ptr()),
        CMD_RADICAL => {
            if chr_code == 1 {
                print_esc_cstr(c"Uradical".as_ptr());
            } else {
                print_esc_cstr(c"radical".as_ptr());
            }
        }
        CMD_READ_TO_CS => {
            if chr_code == 0 {
                print_esc_cstr(c"read".as_ptr());
            } else {
                print_esc_cstr(c"readline".as_ptr());
            }
        }
        CMD_RELAX => print_esc_cstr(c"relax".as_ptr()),
        CMD_SET_BOX => print_esc_cstr(c"setbox".as_ptr()),
        CMD_SET_PREV_GRAF => print_esc_cstr(c"prevgraf".as_ptr()),
        CMD_SET_SHAPE => match chr_code {
            CC_LOCAL_BASE_PAR_SHAPE => print_esc_cstr(c"parshape".as_ptr()),
            CC_ETEX_PEN_BASE_INTER_LINE => print_esc_cstr(c"interlinepenalties".as_ptr()),
            CC_ETEX_PEN_BASE_CLUB => print_esc_cstr(c"clubpenalties".as_ptr()),
            CC_ETEX_PEN_BASE_WIDOW => print_esc_cstr(c"widowpenalties".as_ptr()),
            CC_ETEX_PEN_BASE_DISPLAY_WIDOW => print_esc_cstr(c"displaywidowpenalties".as_ptr()),
            _ => {}
        },
        CMD_THE => {
            if chr_code == 0 {
                print_esc_cstr(c"the".as_ptr());
            } else if chr_code == 1 {
                print_esc_cstr(c"unexpanded".as_ptr());
            } else {
                print_esc_cstr(c"detokenize".as_ptr());
            }
        }
        CMD_TOKS_REGISTER => {
            print_esc_cstr(c"toks".as_ptr());
            if chr_code != 0 {
                print_sa_num(chr_code);
            }
        }
        CMD_VADJUST => print_esc_cstr(c"vadjust".as_ptr()),
        CMD_VALIGN => {
            if chr_code == 0 {
                print_esc_cstr(c"valign".as_ptr());
            } else {
                match chr_code {
                    CC_BEGIN_L_CODE => print_esc_cstr(c"beginL".as_ptr()),
                    CC_END_L_CODE => print_esc_cstr(c"endL".as_ptr()),
                    CC_BEGIN_R_CODE => print_esc_cstr(c"beginR".as_ptr()),
                    _ => print_esc_cstr(c"endR".as_ptr()),
                }
            }
        }
        CMD_VCENTER => print_esc_cstr(c"vcenter".as_ptr()),
        CMD_VRULE => print_esc_cstr(c"vrule".as_ptr()),
        CMD_PAR_END => print_esc_cstr(c"par".as_ptr()),
        CMD_INPUT => {
            if chr_code == 0 {
                print_esc_cstr(c"input".as_ptr());
            } else if chr_code == 2 {
                print_esc_cstr(c"scantokens".as_ptr());
            } else {
                print_esc_cstr(c"endinput".as_ptr());
            }
        }
        CMD_TOP_BOT_MARK => {
            match chr_code % CC_MARKS_CODE {
                CC_FIRST_MARK_CODE => print_esc_cstr(c"firstmark".as_ptr()),
                CC_BOT_MARK_CODE => print_esc_cstr(c"botmark".as_ptr()),
                CC_SPLIT_FIRST_MARK_CODE => print_esc_cstr(c"splitfirstmark".as_ptr()),
                CC_SPLIT_BOT_MARK_CODE => print_esc_cstr(c"splitbotmark".as_ptr()),
                _ => print_esc_cstr(c"topmark".as_ptr()),
            }
            if chr_code >= CC_MARKS_CODE {
                print_char('s' as i32);
            }
        }
        CMD_REGISTER => {
            let (the_cmd, the_chr) = if chr_code < 0 || chr_code > 19 {
                (mem_b16s1(chr_code) as i32 / 64, chr_code)
            } else {
                (chr_code, TEX_NULL)
            };
            if the_cmd == CC_INT_VAL {
                print_esc_cstr(c"count".as_ptr());
            } else if the_cmd == CC_DIMEN_VAL {
                print_esc_cstr(c"dimen".as_ptr());
            } else if the_cmd == CC_GLUE_VAL {
                print_esc_cstr(c"skip".as_ptr());
            } else {
                print_esc_cstr(c"muskip".as_ptr());
            }
            if the_chr != TEX_NULL {
                print_sa_num(the_chr);
            }
        }
        CMD_SET_AUX => {
            if chr_code == CC_VMODE {
                print_esc_cstr(c"prevdepth".as_ptr());
            } else {
                print_esc_cstr(c"spacefactor".as_ptr());
            }
        }
        CMD_SET_PAGE_INT => {
            if chr_code == 0 {
                print_esc_cstr(c"deadcycles".as_ptr());
            } else if chr_code == 2 {
                print_esc_cstr(c"interactionmode".as_ptr());
            } else {
                print_esc_cstr(c"insertpenalties".as_ptr());
            }
        }
        CMD_SET_BOX_DIMEN => {
            if chr_code == CC_WIDTH_OFFSET {
                print_esc_cstr(c"wd".as_ptr());
            } else if chr_code == CC_HEIGHT_OFFSET {
                print_esc_cstr(c"ht".as_ptr());
            } else {
                print_esc_cstr(c"dp".as_ptr());
            }
        }
        CMD_LAST_ITEM => match chr_code {
            CC_INT_VAL => print_esc_cstr(c"lastpenalty".as_ptr()),
            CC_DIMEN_VAL => print_esc_cstr(c"lastkern".as_ptr()),
            CC_GLUE_VAL => print_esc_cstr(c"lastskip".as_ptr()),
            CC_INPUT_LINE_NO_CODE => print_esc_cstr(c"inputlineno".as_ptr()),
            CC_LAST_NODE_TYPE_CODE => print_esc_cstr(c"lastnodetype".as_ptr()),
            CC_ETEX_VERSION_CODE => print_esc_cstr(c"eTeXversion".as_ptr()),
            CC_XETEX_VERSION_CODE => print_esc_cstr(c"XeTeXversion".as_ptr()),
            CC_XETEX_COUNT_GLYPHS_CODE => print_esc_cstr(c"XeTeXcountglyphs".as_ptr()),
            CC_XETEX_COUNT_VARIATIONS_CODE => print_esc_cstr(c"XeTeXcountvariations".as_ptr()),
            CC_XETEX_VARIATION_CODE => print_esc_cstr(c"XeTeXvariation".as_ptr()),
            CC_XETEX_FIND_VARIATION_BY_NAME_CODE => {
                print_esc_cstr(c"XeTeXfindvariationbyname".as_ptr())
            }
            CC_XETEX_VARIATION_MIN_CODE => print_esc_cstr(c"XeTeXvariationmin".as_ptr()),
            CC_XETEX_VARIATION_MAX_CODE => print_esc_cstr(c"XeTeXvariationmax".as_ptr()),
            CC_XETEX_VARIATION_DEFAULT_CODE => print_esc_cstr(c"XeTeXvariationdefault".as_ptr()),
            CC_XETEX_COUNT_FEATURES_CODE => print_esc_cstr(c"XeTeXcountfeatures".as_ptr()),
            CC_XETEX_FEATURE_CODE_CODE => print_esc_cstr(c"XeTeXfeaturecode".as_ptr()),
            CC_XETEX_FIND_FEATURE_BY_NAME_CODE => {
                print_esc_cstr(c"XeTeXfindfeaturebyname".as_ptr())
            }
            CC_XETEX_IS_EXCLUSIVE_FEATURE_CODE => {
                print_esc_cstr(c"XeTeXisexclusivefeature".as_ptr())
            }
            CC_XETEX_COUNT_SELECTORS_CODE => print_esc_cstr(c"XeTeXcountselectors".as_ptr()),
            CC_XETEX_SELECTOR_CODE_CODE => print_esc_cstr(c"XeTeXselectorcode".as_ptr()),
            CC_XETEX_FIND_SELECTOR_BY_NAME_CODE => {
                print_esc_cstr(c"XeTeXfindselectorbyname".as_ptr())
            }
            CC_XETEX_IS_DEFAULT_SELECTOR_CODE => print_esc_cstr(c"XeTeXisdefaultselector".as_ptr()),
            CC_XETEX_OT_COUNT_SCRIPTS_CODE => print_esc_cstr(c"XeTeXOTcountscripts".as_ptr()),
            CC_XETEX_OT_COUNT_LANGUAGES_CODE => print_esc_cstr(c"XeTeXOTcountlanguages".as_ptr()),
            CC_XETEX_OT_COUNT_FEATURES_CODE => print_esc_cstr(c"XeTeXOTcountfeatures".as_ptr()),
            CC_XETEX_OT_SCRIPT_CODE => print_esc_cstr(c"XeTeXOTscripttag".as_ptr()),
            CC_XETEX_OT_LANGUAGE_CODE => print_esc_cstr(c"XeTeXOTlanguagetag".as_ptr()),
            CC_XETEX_OT_FEATURE_CODE => print_esc_cstr(c"XeTeXOTfeaturetag".as_ptr()),
            CC_XETEX_MAP_CHAR_TO_GLYPH_CODE => print_esc_cstr(c"XeTeXcharglyph".as_ptr()),
            CC_XETEX_GLYPH_INDEX_CODE => print_esc_cstr(c"XeTeXglyphindex".as_ptr()),
            CC_XETEX_GLYPH_BOUNDS_CODE => print_esc_cstr(c"XeTeXglyphbounds".as_ptr()),
            CC_XETEX_FONT_TYPE_CODE => print_esc_cstr(c"XeTeXfonttype".as_ptr()),
            CC_XETEX_FIRST_CHAR_CODE => print_esc_cstr(c"XeTeXfirstfontchar".as_ptr()),
            CC_XETEX_LAST_CHAR_CODE => print_esc_cstr(c"XeTeXlastfontchar".as_ptr()),
            CC_XETEX_PDF_PAGE_COUNT_CODE => print_esc_cstr(c"XeTeXpdfpagecount".as_ptr()),
            CC_CURRENT_GROUP_LEVEL_CODE => print_esc_cstr(c"currentgrouplevel".as_ptr()),
            CC_CURRENT_GROUP_TYPE_CODE => print_esc_cstr(c"currentgrouptype".as_ptr()),
            CC_CURRENT_IF_LEVEL_CODE => print_esc_cstr(c"currentiflevel".as_ptr()),
            CC_CURRENT_IF_TYPE_CODE => print_esc_cstr(c"currentiftype".as_ptr()),
            CC_CURRENT_IF_BRANCH_CODE => print_esc_cstr(c"currentifbranch".as_ptr()),
            CC_FONT_CHAR_WD_CODE => print_esc_cstr(c"fontcharwd".as_ptr()),
            CC_FONT_CHAR_HT_CODE => print_esc_cstr(c"fontcharht".as_ptr()),
            CC_FONT_CHAR_DP_CODE => print_esc_cstr(c"fontchardp".as_ptr()),
            CC_FONT_CHAR_IC_CODE => print_esc_cstr(c"fontcharic".as_ptr()),
            CC_PAR_SHAPE_LENGTH_CODE => print_esc_cstr(c"parshapelength".as_ptr()),
            CC_PAR_SHAPE_INDENT_CODE => print_esc_cstr(c"parshapeindent".as_ptr()),
            CC_PAR_SHAPE_DIMEN_CODE => print_esc_cstr(c"parshapedimen".as_ptr()),
            x if x == CC_ETEX_EXPR - CC_INT_VAL + CC_INT_VAL => print_esc_cstr(c"numexpr".as_ptr()),
            x if x == CC_ETEX_EXPR - CC_INT_VAL + CC_DIMEN_VAL => {
                print_esc_cstr(c"dimexpr".as_ptr())
            }
            x if x == CC_ETEX_EXPR - CC_INT_VAL + CC_GLUE_VAL => {
                print_esc_cstr(c"glueexpr".as_ptr())
            }
            x if x == CC_ETEX_EXPR - CC_INT_VAL + CC_MU_VAL => print_esc_cstr(c"muexpr".as_ptr()),
            CC_GLUE_STRETCH_ORDER_CODE => print_esc_cstr(c"gluestretchorder".as_ptr()),
            CC_GLUE_SHRINK_ORDER_CODE => print_esc_cstr(c"glueshrinkorder".as_ptr()),
            CC_GLUE_STRETCH_CODE => print_esc_cstr(c"gluestretch".as_ptr()),
            CC_GLUE_SHRINK_CODE => print_esc_cstr(c"glueshrink".as_ptr()),
            CC_MU_TO_GLUE_CODE => print_esc_cstr(c"mutoglue".as_ptr()),
            CC_GLUE_TO_MU_CODE => print_esc_cstr(c"gluetomu".as_ptr()),
            CC_PDF_LAST_X_POS_CODE => print_esc_cstr(c"pdflastxpos".as_ptr()),
            CC_PDF_LAST_Y_POS_CODE => print_esc_cstr(c"pdflastypos".as_ptr()),
            CC_ELAPSED_TIME_CODE => print_esc_cstr(c"elapsedtime".as_ptr()),
            CC_PDF_SHELL_ESCAPE_CODE => print_esc_cstr(c"shellescape".as_ptr()),
            CC_RANDOM_SEED_CODE => print_esc_cstr(c"randomseed".as_ptr()),
            _ => print_esc_cstr(c"badness".as_ptr()),
        },
        CMD_CONVERT => match chr_code {
            CC_NUMBER_CODE => print_esc_cstr(c"number".as_ptr()),
            CC_ROMAN_NUMERAL_CODE => print_esc_cstr(c"romannumeral".as_ptr()),
            CC_STRING_CODE => print_esc_cstr(c"string".as_ptr()),
            CC_MEANING_CODE => print_esc_cstr(c"meaning".as_ptr()),
            CC_FONT_NAME_CODE => print_esc_cstr(c"fontname".as_ptr()),
            CC_ETEX_REVISION_CODE => print_esc_cstr(c"eTeXrevision".as_ptr()),
            CC_EXPANDED_CODE => print_esc_cstr(c"expanded".as_ptr()),
            CC_LEFT_MARGIN_KERN_CODE => print_esc_cstr(c"leftmarginkern".as_ptr()),
            CC_RIGHT_MARGIN_KERN_CODE => print_esc_cstr(c"rightmarginkern".as_ptr()),
            CC_PDF_CREATION_DATE_CODE => print_esc_cstr(c"creationdate".as_ptr()),
            CC_PDF_FILE_MOD_DATE_CODE => print_esc_cstr(c"filemoddate".as_ptr()),
            CC_PDF_FILE_SIZE_CODE => print_esc_cstr(c"filesize".as_ptr()),
            CC_PDF_MDFIVE_SUM_CODE => print_esc_cstr(c"mdfivesum".as_ptr()),
            CC_PDF_FILE_DUMP_CODE => print_esc_cstr(c"filedump".as_ptr()),
            CC_PDF_STRCMP_CODE => print_esc_cstr(c"strcmp".as_ptr()),
            CC_UNIFORM_DEVIATE_CODE => print_esc_cstr(c"uniformdeviate".as_ptr()),
            CC_NORMAL_DEVIATE_CODE => print_esc_cstr(c"normaldeviate".as_ptr()),
            CC_XETEX_REVISION_CODE => print_esc_cstr(c"XeTeXrevision".as_ptr()),
            CC_XETEX_VARIATION_NAME_CODE => print_esc_cstr(c"XeTeXvariationname".as_ptr()),
            CC_XETEX_FEATURE_NAME_CODE => print_esc_cstr(c"XeTeXfeaturename".as_ptr()),
            CC_XETEX_SELECTOR_NAME_CODE => print_esc_cstr(c"XeTeXselectorname".as_ptr()),
            CC_XETEX_GLYPH_NAME_CODE => print_esc_cstr(c"XeTeXglyphname".as_ptr()),
            CC_XETEX_UCHAR_CODE => print_esc_cstr(c"Uchar".as_ptr()),
            CC_XETEX_UCHARCAT_CODE => print_esc_cstr(c"Ucharcat".as_ptr()),
            _ => print_esc_cstr(c"jobname".as_ptr()),
        },
        CMD_IF_TEST => {
            if chr_code >= CC_UNLESS_CODE {
                print_esc_cstr(c"unless".as_ptr());
            }
            match chr_code % CC_UNLESS_CODE {
                CC_IF_CAT_CODE => print_esc_cstr(c"ifcat".as_ptr()),
                CC_IF_INT_CODE => print_esc_cstr(c"ifnum".as_ptr()),
                CC_IF_DIM_CODE => print_esc_cstr(c"ifdim".as_ptr()),
                CC_IF_ODD_CODE => print_esc_cstr(c"ifodd".as_ptr()),
                CC_IF_VMODE_CODE => print_esc_cstr(c"ifvmode".as_ptr()),
                CC_IF_HMODE_CODE => print_esc_cstr(c"ifhmode".as_ptr()),
                CC_IF_MMODE_CODE => print_esc_cstr(c"ifmmode".as_ptr()),
                CC_IF_INNER_CODE => print_esc_cstr(c"ifinner".as_ptr()),
                CC_IF_VOID_CODE => print_esc_cstr(c"ifvoid".as_ptr()),
                CC_IF_HBOX_CODE => print_esc_cstr(c"ifhbox".as_ptr()),
                CC_IF_VBOX_CODE => print_esc_cstr(c"ifvbox".as_ptr()),
                CC_IFX_CODE => print_esc_cstr(c"ifx".as_ptr()),
                CC_IF_EOF_CODE => print_esc_cstr(c"ifeof".as_ptr()),
                CC_IF_TRUE_CODE => print_esc_cstr(c"iftrue".as_ptr()),
                CC_IF_FALSE_CODE => print_esc_cstr(c"iffalse".as_ptr()),
                CC_IF_CASE_CODE => print_esc_cstr(c"ifcase".as_ptr()),
                CC_IF_PRIMITIVE_CODE => print_esc_cstr(c"ifprimitive".as_ptr()),
                CC_IF_DEF_CODE => print_esc_cstr(c"ifdefined".as_ptr()),
                CC_IF_CS_CODE => print_esc_cstr(c"ifcsname".as_ptr()),
                CC_IF_FONT_CHAR_CODE => print_esc_cstr(c"iffontchar".as_ptr()),
                CC_IF_IN_CSNAME_CODE => print_esc_cstr(c"ifincsname".as_ptr()),
                _ => print_esc_cstr(c"if".as_ptr()),
            }
        }
        CMD_FI_OR_ELSE => {
            if chr_code == CC_FI_CODE {
                print_esc_cstr(c"fi".as_ptr());
            } else if chr_code == CC_OR_CODE {
                print_esc_cstr(c"or".as_ptr());
            } else {
                print_esc_cstr(c"else".as_ptr());
            }
        }
        CMD_TAB_MARK => {
            if chr_code == CC_SPAN_CODE {
                print_esc_cstr(c"span".as_ptr());
            } else {
                print_cstr(c"alignment tab character ".as_ptr());
                print_char_or_wide(chr_code);
            }
        }
        CMD_CAR_RET => {
            if chr_code == CC_CR_CODE {
                print_esc_cstr(c"cr".as_ptr());
            } else {
                print_esc_cstr(c"crcr".as_ptr());
            }
        }
        CMD_SET_PAGE_DIMEN => match chr_code {
            0 => print_esc_cstr(c"pagegoal".as_ptr()),
            1 => print_esc_cstr(c"pagetotal".as_ptr()),
            2 => print_esc_cstr(c"pagestretch".as_ptr()),
            3 => print_esc_cstr(c"pagefilstretch".as_ptr()),
            4 => print_esc_cstr(c"pagefillstretch".as_ptr()),
            5 => print_esc_cstr(c"pagefilllstretch".as_ptr()),
            6 => print_esc_cstr(c"pageshrink".as_ptr()),
            _ => print_esc_cstr(c"pagedepth".as_ptr()),
        },
        CMD_STOP => {
            if chr_code == 1 {
                print_esc_cstr(c"dump".as_ptr());
            } else {
                print_esc_cstr(c"end".as_ptr());
            }
        }
        CMD_HSKIP => match chr_code {
            CC_SKIP_CODE => print_esc_cstr(c"hskip".as_ptr()),
            CC_FIL_CODE => print_esc_cstr(c"hfil".as_ptr()),
            CC_FILL_CODE => print_esc_cstr(c"hfill".as_ptr()),
            CC_SS_CODE => print_esc_cstr(c"hss".as_ptr()),
            _ => print_esc_cstr(c"hfilneg".as_ptr()),
        },
        CMD_VSKIP => match chr_code {
            CC_SKIP_CODE => print_esc_cstr(c"vskip".as_ptr()),
            CC_FIL_CODE => print_esc_cstr(c"vfil".as_ptr()),
            CC_FILL_CODE => print_esc_cstr(c"vfill".as_ptr()),
            CC_SS_CODE => print_esc_cstr(c"vss".as_ptr()),
            _ => print_esc_cstr(c"vfilneg".as_ptr()),
        },
        CMD_MSKIP => print_esc_cstr(c"mskip".as_ptr()),
        CMD_KERN => print_esc_cstr(c"kern".as_ptr()),
        CMD_MKERN => print_esc_cstr(c"mkern".as_ptr()),
        CMD_HMOVE => {
            if chr_code == 1 {
                print_esc_cstr(c"moveleft".as_ptr());
            } else {
                print_esc_cstr(c"moveright".as_ptr());
            }
        }
        CMD_VMOVE => {
            if chr_code == 1 {
                print_esc_cstr(c"raise".as_ptr());
            } else {
                print_esc_cstr(c"lower".as_ptr());
            }
        }
        CMD_MAKE_BOX => match chr_code {
            CC_BOX_CODE => print_esc_cstr(c"box".as_ptr()),
            CC_COPY_CODE => print_esc_cstr(c"copy".as_ptr()),
            CC_LAST_BOX_CODE => print_esc_cstr(c"lastbox".as_ptr()),
            CC_VSPLIT_CODE => print_esc_cstr(c"vsplit".as_ptr()),
            CC_VTOP_CODE => print_esc_cstr(c"vtop".as_ptr()),
            x if x == CC_VTOP_CODE + CC_VMODE => print_esc_cstr(c"vbox".as_ptr()),
            _ => print_esc_cstr(c"hbox".as_ptr()),
        },
        CMD_LEADER_SHIP => {
            if chr_code == CC_A_LEADERS {
                print_esc_cstr(c"leaders".as_ptr());
            } else if chr_code == CC_C_LEADERS {
                print_esc_cstr(c"cleaders".as_ptr());
            } else if chr_code == CC_X_LEADERS {
                print_esc_cstr(c"xleaders".as_ptr());
            } else {
                print_esc_cstr(c"shipout".as_ptr());
            }
        }
        CMD_START_PAR => {
            if chr_code == 0 {
                print_esc_cstr(c"noindent".as_ptr());
            } else {
                print_esc_cstr(c"indent".as_ptr());
            }
        }
        CMD_REMOVE_ITEM => {
            if chr_code == CC_GLUE_NODE {
                print_esc_cstr(c"unskip".as_ptr());
            } else if chr_code == CC_KERN_NODE {
                print_esc_cstr(c"unkern".as_ptr());
            } else {
                print_esc_cstr(c"unpenalty".as_ptr());
            }
        }
        CMD_UN_HBOX => {
            if chr_code == CC_COPY_CODE {
                print_esc_cstr(c"unhcopy".as_ptr());
            } else {
                print_esc_cstr(c"unhbox".as_ptr());
            }
        }
        CMD_UN_VBOX => {
            if chr_code == CC_COPY_CODE {
                print_esc_cstr(c"unvcopy".as_ptr());
            } else if chr_code == CC_LAST_BOX_CODE {
                print_esc_cstr(c"pagediscards".as_ptr());
            } else if chr_code == CC_VSPLIT_CODE2 {
                print_esc_cstr(c"splitdiscards".as_ptr());
            } else {
                print_esc_cstr(c"unvbox".as_ptr());
            }
        }
        CMD_DISCRETIONARY => {
            if chr_code == 1 {
                print_esc('-' as i32);
            } else {
                print_esc_cstr(c"discretionary".as_ptr());
            }
        }
        CMD_EQ_NO => {
            if chr_code == 1 {
                print_esc_cstr(c"leqno".as_ptr());
            } else {
                print_esc_cstr(c"eqno".as_ptr());
            }
        }
        CMD_MATH_COMP => match chr_code {
            CC_ORD_NOAD => print_esc_cstr(c"mathord".as_ptr()),
            CC_OP_NOAD => print_esc_cstr(c"mathop".as_ptr()),
            CC_BIN_NOAD => print_esc_cstr(c"mathbin".as_ptr()),
            CC_REL_NOAD => print_esc_cstr(c"mathrel".as_ptr()),
            CC_OPEN_NOAD => print_esc_cstr(c"mathopen".as_ptr()),
            CC_CLOSE_NOAD => print_esc_cstr(c"mathclose".as_ptr()),
            CC_PUNCT_NOAD => print_esc_cstr(c"mathpunct".as_ptr()),
            CC_INNER_NOAD => print_esc_cstr(c"mathinner".as_ptr()),
            CC_UNDER_NOAD => print_esc_cstr(c"underline".as_ptr()),
            _ => print_esc_cstr(c"overline".as_ptr()),
        },
        CMD_LIMIT_SWITCH => {
            if chr_code == CC_LIMITS {
                print_esc_cstr(c"limits".as_ptr());
            } else if chr_code == CC_NO_LIMITS {
                print_esc_cstr(c"nolimits".as_ptr());
            } else {
                print_esc_cstr(c"displaylimits".as_ptr());
            }
        }
        CMD_MATH_STYLE => print_style(chr_code),
        CMD_ABOVE => match chr_code {
            CC_OVER_CODE => print_esc_cstr(c"over".as_ptr()),
            CC_ATOP_CODE => print_esc_cstr(c"atop".as_ptr()),
            x if x == CC_DELIMITED_CODE + CC_ABOVE_CODE => {
                print_esc_cstr(c"abovewithdelims".as_ptr())
            }
            x if x == CC_DELIMITED_CODE + CC_OVER_CODE => {
                print_esc_cstr(c"overwithdelims".as_ptr())
            }
            x if x == CC_DELIMITED_CODE + CC_ATOP_CODE => {
                print_esc_cstr(c"atopwithdelims".as_ptr())
            }
            _ => print_esc_cstr(c"above".as_ptr()),
        },
        CMD_LEFT_RIGHT => {
            if chr_code == CC_LEFT_NOAD {
                print_esc_cstr(c"left".as_ptr());
            } else if chr_code == CC_MIDDLE_NOAD {
                print_esc_cstr(c"middle".as_ptr());
            } else {
                print_esc_cstr(c"right".as_ptr());
            }
        }
        CMD_PREFIX => {
            if chr_code == 1 {
                print_esc_cstr(c"long".as_ptr());
            } else if chr_code == 2 {
                print_esc_cstr(c"outer".as_ptr());
            } else if chr_code == 8 {
                print_esc_cstr(c"protected".as_ptr());
            } else {
                print_esc_cstr(c"global".as_ptr());
            }
        }
        CMD_DEF => {
            if chr_code == 0 {
                print_esc_cstr(c"def".as_ptr());
            } else if chr_code == 1 {
                print_esc_cstr(c"gdef".as_ptr());
            } else if chr_code == 2 {
                print_esc_cstr(c"edef".as_ptr());
            } else {
                print_esc_cstr(c"xdef".as_ptr());
            }
        }
        CMD_LET => {
            if chr_code != CC_NORMAL {
                print_esc_cstr(c"futurelet".as_ptr());
            } else {
                print_esc_cstr(c"let".as_ptr());
            }
        }
        CMD_SHORTHAND_DEF => match chr_code {
            CC_CHAR_DEF_CODE => print_esc_cstr(c"chardef".as_ptr()),
            CC_MATH_CHAR_DEF_CODE => print_esc_cstr(c"mathchardef".as_ptr()),
            CC_XETEX_MATH_CHAR_DEF_CODE => print_esc_cstr(c"Umathchardef".as_ptr()),
            CC_XETEX_MATH_CHAR_NUM_DEF_CODE => print_esc_cstr(c"Umathcharnumdef".as_ptr()),
            CC_COUNT_DEF_CODE => print_esc_cstr(c"countdef".as_ptr()),
            CC_DIMEN_DEF_CODE => print_esc_cstr(c"dimendef".as_ptr()),
            CC_SKIP_DEF_CODE => print_esc_cstr(c"skipdef".as_ptr()),
            CC_MU_SKIP_DEF_CODE => print_esc_cstr(c"muskipdef".as_ptr()),
            CC_CHAR_SUB_DEF_CODE => print_esc_cstr(c"charsubdef".as_ptr()),
            _ => print_esc_cstr(c"toksdef".as_ptr()),
        },
        CMD_CHAR_GIVEN => {
            print_esc_cstr(c"char".as_ptr());
            print_hex(chr_code);
        }
        CMD_MATH_GIVEN => {
            print_esc_cstr(c"mathchar".as_ptr());
            print_hex(chr_code);
        }
        CMD_XETEX_MATH_GIVEN => {
            print_esc_cstr(c"Umathchar".as_ptr());
            print_hex(((chr_code as u32 >> 21) & 0x07) as i32); // math_class
            print_hex(((chr_code as u32 >> 24) & 0xFF) as i32); // math_fam
            print_hex((chr_code as u32 & 0x1FFFFF) as i32); // math_char
        }
        CMD_DEF_CODE => {
            if chr_code == CC_LC_CODE_BASE {
                print_esc_cstr(c"lccode".as_ptr());
            } else if chr_code == CC_UC_CODE_BASE {
                print_esc_cstr(c"uccode".as_ptr());
            } else if chr_code == CC_SF_CODE_BASE {
                print_esc_cstr(c"sfcode".as_ptr());
            } else if chr_code == CC_MATH_CODE_BASE {
                print_esc_cstr(c"mathcode".as_ptr());
            } else if chr_code == CC_DEL_CODE_BASE {
                print_esc_cstr(c"delcode".as_ptr());
            } else {
                print_esc_cstr(c"catcode".as_ptr());
            }
        }
        CMD_XETEX_DEF_CODE => {
            if chr_code == CC_SF_CODE_BASE {
                print_esc_cstr(c"XeTeXcharclass".as_ptr());
            } else if chr_code == CC_MATH_CODE_BASE {
                print_esc_cstr(c"Umathcodenum".as_ptr());
            } else if chr_code == CC_MATH_CODE_BASE + 1 {
                print_esc_cstr(c"Umathcode".as_ptr());
            } else if chr_code == CC_DEL_CODE_BASE {
                print_esc_cstr(c"Udelcodenum".as_ptr());
            } else {
                print_esc_cstr(c"Udelcode".as_ptr());
            }
        }
        CMD_DEF_FAMILY => print_size(chr_code - CC_MATH_FONT_BASE),
        CMD_HYPH_DATA => {
            if chr_code == 1 {
                print_esc_cstr(c"patterns".as_ptr());
            } else {
                print_esc_cstr(c"hyphenation".as_ptr());
            }
        }
        CMD_ASSIGN_FONT_INT => match chr_code {
            0 => print_esc_cstr(c"hyphenchar".as_ptr()),
            1 => print_esc_cstr(c"skewchar".as_ptr()),
            CC_LP_CODE_BASE => print_esc_cstr(c"lpcode".as_ptr()),
            CC_RP_CODE_BASE => print_esc_cstr(c"rpcode".as_ptr()),
            _ => {}
        },
        CMD_SET_FONT => {
            print_cstr(c"select font ".as_ptr());
            let font_name_str = font_name_entry(chr_code);
            let fa = font_area_entry(chr_code);
            if fa == CC_AAT_FONT_FLAG || fa == CC_OTGR_FONT_FLAG {
                let len = crate::string_pool::length(font_name_str);
                let mut quote_char = b'"' as i32;
                for n in 0..len {
                    if str_pool_entry(str_start(font_name_str - 65536) + n) == b'"' as u16 {
                        quote_char = b'\'' as i32;
                    }
                }
                print_char(quote_char);
                print(font_name_str);
                print_char(quote_char);
            } else {
                print(font_name_str);
            }
            if font_size_entry(chr_code) != font_dsize_entry(chr_code) {
                print_cstr(c" at ".as_ptr());
                print_scaled(font_size_entry(chr_code));
                print_cstr(c"pt".as_ptr());
            }
        }
        CMD_SET_INTERACTION => match chr_code {
            CC_BATCH_MODE => print_esc_cstr(c"batchmode".as_ptr()),
            CC_NONSTOP_MODE => print_esc_cstr(c"nonstopmode".as_ptr()),
            CC_SCROLL_MODE => print_esc_cstr(c"scrollmode".as_ptr()),
            _ => print_esc_cstr(c"errorstopmode".as_ptr()),
        },
        CMD_IN_STREAM => {
            if chr_code == 0 {
                print_esc_cstr(c"closein".as_ptr());
            } else {
                print_esc_cstr(c"openin".as_ptr());
            }
        }
        CMD_MESSAGE => {
            if chr_code == 0 {
                print_esc_cstr(c"message".as_ptr());
            } else {
                print_esc_cstr(c"errmessage".as_ptr());
            }
        }
        CMD_CASE_SHIFT => {
            if chr_code == CC_LC_CODE_BASE {
                print_esc_cstr(c"lowercase".as_ptr());
            } else {
                print_esc_cstr(c"uppercase".as_ptr());
            }
        }
        CMD_XRAY => match chr_code {
            CC_SHOW_BOX_CODE => print_esc_cstr(c"showbox".as_ptr()),
            CC_SHOW_THE_CODE => print_esc_cstr(c"showthe".as_ptr()),
            CC_SHOW_LISTS => print_esc_cstr(c"showlists".as_ptr()),
            CC_SHOW_GROUPS => print_esc_cstr(c"showgroups".as_ptr()),
            CC_SHOW_TOKENS => print_esc_cstr(c"showtokens".as_ptr()),
            CC_SHOW_IFS => print_esc_cstr(c"showifs".as_ptr()),
            _ => print_esc_cstr(c"show".as_ptr()),
        },
        CMD_UNDEFINED_CS => print_cstr(c"undefined".as_ptr()),
        CMD_CALL | CMD_LONG_CALL | CMD_OUTER_CALL | CMD_LONG_OUTER_CALL => {
            let mut n = cmd as i32 - CMD_CALL as i32;
            if mem_b32s0(mem_b32s1(chr_code)) == CC_PROTECTED_TOKEN {
                n += 4;
            }
            if n / 4 % 2 != 0 {
                print_esc_cstr(c"protected".as_ptr());
            }
            if n % 2 != 0 {
                print_esc_cstr(c"long".as_ptr());
            }
            if n / 2 % 2 != 0 {
                print_esc_cstr(c"outer".as_ptr());
            }
            if n > 0 {
                print_char(' ' as i32);
            }
            print_cstr(c"macro".as_ptr());
        }
        CMD_END_TEMPLATE => print_esc_cstr(c"outer endtemplate".as_ptr()),
        CMD_EXTENSION => match chr_code {
            CC_OPEN_NODE => print_esc_cstr(c"openout".as_ptr()),
            CC_WRITE_NODE => print_esc_cstr(c"write".as_ptr()),
            CC_CLOSE_NODE => print_esc_cstr(c"closeout".as_ptr()),
            CC_SPECIAL_NODE => print_esc_cstr(c"special".as_ptr()),
            CC_IMMEDIATE_CODE => print_esc_cstr(c"immediate".as_ptr()),
            CC_SET_LANGUAGE_CODE => print_esc_cstr(c"setlanguage".as_ptr()),
            CC_PDF_SAVE_POS_NODE => print_esc_cstr(c"pdfsavepos".as_ptr()),
            CC_RESET_TIMER_CODE => print_esc_cstr(c"resettimer".as_ptr()),
            CC_SET_RANDOM_SEED_CODE => print_esc_cstr(c"setrandomseed".as_ptr()),
            CC_PIC_FILE_CODE => print_esc_cstr(c"XeTeXpicfile".as_ptr()),
            CC_PDF_FILE_CODE => print_esc_cstr(c"XeTeXpdffile".as_ptr()),
            CC_GLYPH_CODE => print_esc_cstr(c"XeTeXglyph".as_ptr()),
            CC_XETEX_LINEBREAK_LOCALE_EXTENSION_CODE => {
                print_esc_cstr(c"XeTeXlinebreaklocale".as_ptr())
            }
            CC_XETEX_INPUT_ENCODING_EXTENSION_CODE => {
                print_esc_cstr(c"XeTeXinputencoding".as_ptr())
            }
            CC_XETEX_DEFAULT_ENCODING_EXTENSION_CODE => {
                print_esc_cstr(c"XeTeXdefaultencoding".as_ptr())
            }
            _ => print_cstr(c"[unknown extension!]".as_ptr()),
        },
        _ => print_cstr(c"[unknown command code!]".as_ptr()),
    }
}

// ---------------------------------------------------------------------------
// print_meaning
// ---------------------------------------------------------------------------

/// Print the meaning of the current token (`cur_cmd`/`cur_chr`).
///
/// translated from xetex-xetex0.c:3383 (print_meaning)
#[no_mangle]
pub unsafe extern "C" fn print_meaning() {
    print_cmd_chr(cur_cmd() as u16, cur_chr());
    if cur_cmd() as u16 >= CALL {
        print_char(':' as i32);
        print_ln();
        token_show(cur_chr());
    } else if cur_cmd() as u16 == TOP_BOT_MARK && (cur_chr() as usize) < 5 {
        print_char(':' as i32);
        print_ln();
        token_show(cur_mark(cur_chr() as usize));
    }
}

// ---------------------------------------------------------------------------
// group_warning
// ---------------------------------------------------------------------------

/// Warn if a group spans file boundaries (\tracingcommands > 0).
///
/// translated from xetex-xetex0.c:2961 (group_warning)
#[no_mangle]
pub unsafe extern "C" fn group_warning() {
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
#[no_mangle]
pub unsafe extern "C" fn if_warning() {
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
#[no_mangle]
pub unsafe extern "C" fn file_warning() {
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
#[no_mangle]
pub unsafe extern "C" fn show_cur_cmd_chr() {
    begin_diagnostic();
    diagnostic_begin_capture_warning_here();
    print_nl(b'{' as i32);
    if cur_list_mode() != shown_mode() as i16 {
        print_mode(cur_list_mode() as i32);
        print_cstr(c": ".as_ptr());
        set_shown_mode(cur_list_mode());
    }
    print_cmd_chr(cur_cmd() as u16, cur_chr());
    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_IFS) > 0 {
        if cur_cmd() >= IF_TEST as u8 {
            if cur_cmd() <= FI_OR_ELSE {
                print_cstr(c": ".as_ptr());
                let (mut n, mut l) = if cur_cmd() == FI_OR_ELSE {
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
#[no_mangle]
pub unsafe extern "C" fn change_if_limit(l: i16, p: i32) {
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

extern "C" {
    fn print_utf8_str(s: *const libc::c_void, len: i32);
}

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
        print_utf8_str(setting_name_p, set_len);
        print_cstr(c"' for ".as_ptr());
    }
    print_cstr(c"feature `".as_ptr());
    print_utf8_str(feature_name_p, feat_len);
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
#[no_mangle]
pub unsafe extern "C" fn font_mapping_warning(
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
    print_utf8_str(mapping_name_p, mapping_name_len);
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

/// translated from xetex-xetex0.c:8764 (graphite_warning)
#[no_mangle]
pub unsafe extern "C" fn graphite_warning() {
    begin_diagnostic();
    diagnostic_begin_capture_warning_here();
    print_nl_cstr(c"Font `".as_ptr());
    let nof = crate::globals::name_of_file();
    let mut i = 0;
    while *nof.offset(i) != 0 {
        print_raw_char(*nof.offset(i) as u16, true);
        i += 1;
    }
    print_cstr(c"' does not support Graphite. Trying OpenType layout instead.".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    end_diagnostic(false);
}

// ---------------------------------------------------------------------------
// bad_utf8_warning
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:8970 (bad_utf8_warning)
#[no_mangle]
pub unsafe extern "C" fn bad_utf8_warning() {
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
const CS_TOKEN_FLAG_STL: i32 = 0x1FF_FFFF;

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
#[no_mangle]
pub unsafe extern "C" fn show_token_list(mut p: i32, q: i32, l: i32) {
    use crate::globals::{
        error_line, half_error_line, hi_mem_min, mem_b32s0, mem_b32s1, mem_end, set_first_count,
        set_tally, set_trick_count, tally,
    };
    let mut match_chr: i32 = '#' as i32;
    let mut n: i32 = '0' as i32;
    set_tally(0);
    while p != crate::mem::TEX_NULL && tally() < l {
        if p == q {
            set_first_count(tally());
            let tc = tally() + 1 + error_line() - half_error_line();
            set_trick_count(tc.max(error_line()));
        }
        if p < hi_mem_min() || p > mem_end() {
            print_esc_cstr(c"CLOBBERED.".as_ptr());
            return;
        }
        let w = mem_b32s0(p);
        if w >= CS_TOKEN_FLAG_STL {
            print_cs(w - CS_TOKEN_FLAG_STL);
        } else if w < 0 {
            print_esc_cstr(c"BAD.".as_ptr());
        } else {
            let m = w / MAX_CHAR_VAL_STL;
            let c = w % MAX_CHAR_VAL_STL;
            match m {
                LEFT_BRACE_STL | RIGHT_BRACE_STL | MATH_SHIFT_STL | TAB_MARK_STL | SUP_MARK_STL
                | SUB_MARK_STL | SPACER_STL | LETTER_STL | OTHER_CHAR_STL => {
                    print_char(c);
                }
                MAC_PARAM_STL => {
                    print_char(c);
                    print_char(c);
                }
                OUT_PARAM_STL => {
                    print_char(match_chr);
                    if c <= 9 {
                        print_char(c + 48);
                    } else {
                        print_char('!' as i32);
                        return;
                    }
                }
                MATCH_STL => {
                    match_chr = c;
                    print_char(c);
                    n += 1;
                    print_char(n);
                    if n > '9' as i32 {
                        return;
                    }
                }
                END_MATCH_STL => {
                    if c == 0 {
                        print_cstr(c"->".as_ptr());
                    }
                }
                _ => {
                    print_esc_cstr(c"BAD.".as_ptr());
                }
            }
        }
        p = mem_b32s1(p);
    }
    if p != crate::mem::TEX_NULL {
        print_esc_cstr(c"ETC.".as_ptr());
    }
}

// translated from xetex-xetex0.c:9282 (print_totals)
#[no_mangle]
pub unsafe extern "C" fn print_totals() {
    print_scaled(crate::globals::page_so_far(1));
    if crate::globals::page_so_far(2) != 0 {
        print_cstr(c" plus ".as_ptr());
        print_scaled(crate::globals::page_so_far(2));
        print_cstr(c"".as_ptr());
    }
    if crate::globals::page_so_far(3) != 0 {
        print_cstr(c" plus ".as_ptr());
        print_scaled(crate::globals::page_so_far(3));
        print_cstr(c"fil".as_ptr());
    }
    if crate::globals::page_so_far(4) != 0 {
        print_cstr(c" plus ".as_ptr());
        print_scaled(crate::globals::page_so_far(4));
        print_cstr(c"fill".as_ptr());
    }
    if crate::globals::page_so_far(5) != 0 {
        print_cstr(c" plus ".as_ptr());
        print_scaled(crate::globals::page_so_far(5));
        print_cstr(c"filll".as_ptr());
    }
    if crate::globals::page_so_far(6) != 0 {
        print_cstr(c" minus ".as_ptr());
        print_scaled(crate::globals::page_so_far(6));
    }
}
