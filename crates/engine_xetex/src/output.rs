//! Phase 3 — printing helpers translated from `xetex/xetex-output.c`.

use crate::globals::{dig_entry, eqtb_b32s1, file_offset, selector, set_dig_entry, term_offset};

// Selector constants (xetex-core.h:42–51). Only those needed here.
const SELECTOR_LOG_ONLY: i32 = 18;

// Constants for INTPAR(escape_char) = eqtb[INT_BASE + INT_PAR__escape_char].b32.s1
// (xetex_format.h:219,80 and xetex-constants.h:83).
const INT_BASE: i32 = 7_826_729;
const INT_PAR_ESCAPE_CHAR: i32 = 45;
const BIGGEST_USV: i32 = 0x10_FFFF;

// C functions not yet translated — still live in xetex-output.c.
extern "C" {
    fn print_char(s: i32);
    fn print_ln();
    fn print(s: i32);
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
