//! Phase 3 — printing helpers translated from `xetex/xetex-output.c`.

use crate::globals::{
    current_diagnostic, dig_entry, doing_special, eqtb_b32s1, error_line, file_offset, log_file,
    max_print_line, pool_ptr, pool_size, rust_stdout, selector, set_dig_entry, set_eqtb_b32s1,
    set_file_offset, set_pool_ptr, set_str_pool, set_tally, set_term_offset, set_trick_buf,
    str_pool_entry, str_ptr, str_start, tally, term_offset, trick_count, write_file,
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
const BIGGEST_USV: i32 = 0x10_FFFF;
const BIGGEST_CHAR: i32 = 0xFFFF;

// C functions not yet translated — still live in xetex-output.c or bridge.
extern "C" {
    // tectonic_bridge_core output helpers.
    fn ttstub_output_putc(handle: usize, c: i32) -> i32;
    fn ttbc_diag_append(diag: *mut std::ffi::c_void, text: *const libc::c_char);
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
    if c >= 0 && c <= BIGGEST_USV {
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
    if c >= 0 && c <= BIGGEST_USV {
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
            u = u / (ROMAN_DATA[k - 1] as i32 - b'0' as i32);
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
        s = -(s as i32);
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
    if s == eqtb_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR) {
        if selector() < SELECTOR_PSEUDO {
            print_ln();
            return;
        }
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
        if s == eqtb_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR) {
            if selector() < SELECTOR_PSEUDO {
                print_ln();
                return;
            }
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
        if c >= 0xD800
            && c < 0xDC00
            && i + 1 < end
        {
            let c2 = str_pool_entry(i + 1);
            if c2 >= 0xDC00 && c2 < 0xE000 {
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
