//! Phase 3 — printing helpers translated from `xetex/xetex-output.c`.

use crate::globals::{
    cur_group, cur_level, current_diagnostic, dig_entry, doing_special, eqtb_b32s1, eqtb_top,
    error_line, file_line_error_style_p, file_offset, full_source_filename_stack, hash_s1,
    history, in_open, line, line_stack, log_file, max_print_line, mem_as_u16_ptr, mem_b16s1,
    mem_b32s0, mem_b32s1, old_setting, pool_ptr, pool_size, prim_s1, rust_stdout, save_ptr,
    save_stack_b32s1, selector, set_current_diagnostic, set_dig_entry, set_eqtb_b32s1,
    set_file_offset, set_history, set_old_setting, set_pool_ptr, set_selector, set_str_pool,
    set_tally, set_term_offset, set_trick_buf, str_pool_entry, str_ptr, str_start, tally,
    term_offset, trick_count, write_file,
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
    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) <= 0
        && selector() == SELECTOR_TERM_AND_LOG
    {
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
