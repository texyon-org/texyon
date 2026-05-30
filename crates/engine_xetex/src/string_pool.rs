//! String pool functions translated from `xetex/xetex-stringpool.c`.
//!
//! The "string pool" is XeTeX's global heap for TeX-level strings. Strings
//! are stored as UTF-16 code units in `str_pool[0..pool_ptr]`, with
//! `str_start[k - TOO_BIG_CHAR]` giving the start offset of string `k`.
//! Single-character strings below 65536 are stored implicitly (no pool slot).

use crate::globals::{
    buffer_entry, init_pool_ptr, init_str_ptr, max_strings, pool_ptr, pool_size, set_pool_ptr,
    set_str_pool, set_str_ptr, set_str_start, str_pool_entry, str_ptr, str_start,
};

// TOO_BIG_CHAR: the threshold above which str_number values index the pool.
const TOO_BIG_CHAR: i32 = 65536;
// EMPTY_STRING = TOO_BIG_CHAR + 1 (xetex-stringpool.h:12).
use crate::errors::overflow;

const EMPTY_STRING: i32 = 65537;

// C function still needed — not yet translated.
extern "C" {}

// ---------------------------------------------------------------------------
// length
// ---------------------------------------------------------------------------

/// Return the number of UTF-16 code units in string `s`.
///
/// translated from xetex-stringpool.c:42 (length)
pub unsafe fn length(s: i32) -> i32 {
    if s >= TOO_BIG_CHAR {
        return str_start(s + 1 - TOO_BIG_CHAR) - str_start(s - TOO_BIG_CHAR);
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
    8
}

// ---------------------------------------------------------------------------
// make_string
// ---------------------------------------------------------------------------

/// Finalise a string being assembled in the pool and return its str_number.
///
/// translated from xetex-stringpool.c:57 (make_string)
pub unsafe fn make_string() -> i32 {
    if str_ptr() == max_strings() {
        overflow(
            c"number of strings".as_ptr(),
            max_strings() - init_str_ptr(),
        );
    }
    set_str_ptr(str_ptr() + 1);
    set_str_start(str_ptr() - TOO_BIG_CHAR, pool_ptr());
    str_ptr() - 1
}

// ---------------------------------------------------------------------------
// append_str
// ---------------------------------------------------------------------------

/// Append string `s` to the current pool buffer (extending the string
/// being built by a prior `make_string` caller).
///
/// translated from xetex-stringpool.c:69 (append_str)
#[no_mangle]
pub unsafe extern "C" fn append_str(s: i32) {
    let i = length(s);
    if pool_ptr() + i > pool_size() {
        overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr());
    }
    let mut j = str_start(s - TOO_BIG_CHAR);
    let mut remaining = i;
    while remaining > 0 {
        set_str_pool(pool_ptr(), str_pool_entry(j));
        set_pool_ptr(pool_ptr() + 1);
        j += 1;
        remaining -= 1;
    }
}

// ---------------------------------------------------------------------------
// str_eq_buf
// ---------------------------------------------------------------------------

/// Return true if string `s` equals the buffer run starting at position `k`.
///
/// translated from xetex-stringpool.c:91 (str_eq_buf)
pub unsafe fn str_eq_buf(s: i32, mut k: i32) -> bool {
    let mut j = str_start(s - TOO_BIG_CHAR);
    while j < str_start(s + 1 - TOO_BIG_CHAR) {
        let bk = buffer_entry(k);
        if bk >= TOO_BIG_CHAR {
            // Surrogate pair in buffer: two pool slots needed.
            let hi = 0xD800 + (bk - TOO_BIG_CHAR) / 1024;
            let lo = 0xDC00 + (bk - TOO_BIG_CHAR) % 1024;
            if str_pool_entry(j) as i32 != hi || str_pool_entry(j + 1) as i32 != lo {
                return false;
            }
            j += 1;
        } else if str_pool_entry(j) as i32 != bk {
            return false;
        }
        j += 1;
        k += 1;
    }
    true
}

// ---------------------------------------------------------------------------
// str_eq_str
// ---------------------------------------------------------------------------

/// Return true if strings `s` and `t` have the same content.
///
/// translated from xetex-stringpool.c:119 (str_eq_str)
pub unsafe fn str_eq_str(s: i32, t: i32) -> bool {
    if length(s) != length(t) {
        return false;
    }
    if length(s) == 1 {
        // Single-code-unit strings: may be implicit (< TOO_BIG_CHAR) or pooled.
        let cs = if s < TOO_BIG_CHAR {
            s as u16
        } else {
            str_pool_entry(str_start(s - TOO_BIG_CHAR))
        };
        let ct = if t < TOO_BIG_CHAR {
            t as u16
        } else {
            str_pool_entry(str_start(t - TOO_BIG_CHAR))
        };
        return cs == ct;
    }
    // General case: both are pooled strings.
    let mut j = str_start(s - TOO_BIG_CHAR);
    let mut k = str_start(t - TOO_BIG_CHAR);
    while j < str_start(s + 1 - TOO_BIG_CHAR) {
        if str_pool_entry(j) != str_pool_entry(k) {
            return false;
        }
        j += 1;
        k += 1;
    }
    true
}

// ---------------------------------------------------------------------------
// search_string
// ---------------------------------------------------------------------------

/// Find an existing string equal to `search` and return it; return 0 if none.
///
/// translated from xetex-stringpool.c:162 (search_string)
#[no_mangle]
pub unsafe extern "C" fn search_string(search: i32) -> i32 {
    let len = length(search);
    if len == 0 {
        return EMPTY_STRING;
    }
    let mut s = search - 1;
    while s > TOO_BIG_CHAR - 1 {
        if length(s) == len && str_eq_str(s, search) {
            return s;
        }
        s -= 1;
    }
    0
}

// ---------------------------------------------------------------------------
// slow_make_string
// ---------------------------------------------------------------------------

/// Like `make_string`, but deduplicate: if an equal string already exists,
/// discard the new pool entry and return the existing str_number.
///
/// translated from xetex-stringpool.c:190 (slow_make_string)
#[no_mangle]
pub unsafe extern "C" fn slow_make_string() -> i32 {
    let t = make_string();
    let s = search_string(t);
    if s > 0 {
        set_str_ptr(str_ptr() - 1);
        set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
        return s;
    }
    t
}

// ---------------------------------------------------------------------------
// load_pool_strings
// ---------------------------------------------------------------------------

/// Load compile-time string constants into the pool; return the last
/// str_number on success, 0 if the pool would overflow.
///
/// translated from xetex-stringpool.c:17 (load_pool_strings)
#[no_mangle]
pub unsafe extern "C" fn load_pool_strings(spare_size: i32) -> i32 {
    // The C version iterates a NULL-terminated `string_constants[]` array.
    // Tectonic's array has exactly two entries: a sentinel string and "",
    // so the loop body runs once (for the sentinel) and then once more for "".
    // We replicate it by iterating the same constants here.
    static STRING_CONSTANTS: &[&[u8]] = &[b"this marks the start of the stringpool", b""];

    let mut total_len: usize = 0;
    let mut g: i32 = 0;

    for s in STRING_CONSTANTS {
        let len = s.len();
        total_len += len;
        if total_len >= spare_size as usize {
            return 0;
        }
        for &byte in *s {
            set_str_pool(pool_ptr(), byte as u16);
            set_pool_ptr(pool_ptr() + 1);
        }
        g = make_string();
    }

    g
}

// ---------------------------------------------------------------------------
// flush_str
// ---------------------------------------------------------------------------

/// Free the top-of-pool string if it equals s.
///
/// translated from xetex-xetex0.c:15948 (flush_str)
#[no_mangle]
pub unsafe extern "C" fn flush_str(s: i32) {
    if s == str_ptr() - 1 {
        set_str_ptr(str_ptr() - 1);
        set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
    }
}
