//! String pool functions translated from `xetex/xetex-stringpool.c`.
//!
//! The "string pool" is XeTeX's global heap for TeX-level strings. Strings
//! are stored as UTF-16 code units in `str_pool[0..pool_ptr]`, with
//! `str_start[k - TOO_BIG_CHAR]` giving the start offset of string `k`.
//! Single-character strings below 65536 are stored implicitly (no pool slot).

use crate::consts::TOO_BIG_CHAR;
use crate::scan::NodeRecycleAbort;
use crate::state::StringsState;

const EMPTY_STRING: i32 = 65537;

// ---------------------------------------------------------------------------
// length
// ---------------------------------------------------------------------------

/// Return the number of UTF-16 code units in string `s`.
///
/// translated from xetex-stringpool.c:42 (length)
pub fn length(strings: &StringsState, s: i32) -> i32 {
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
    8
}

// ---------------------------------------------------------------------------
// make_string
// ---------------------------------------------------------------------------

/// Finalise a string being assembled in the pool and return its str_number.
///
/// translated from xetex-stringpool.c:57 (make_string)
pub fn make_string(strings: &mut StringsState) -> Result<i32, NodeRecycleAbort> {
    if strings.str_ptr == strings.max_strings {
        return Err(NodeRecycleAbort::Overflow);
    }
    strings.str_ptr += 1;
    strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize] = strings.pool_ptr;
    Ok(strings.str_ptr - 1)
}

pub unsafe fn make_string_global() -> i32 {
    let res = crate::state::with_state_mut(|st| make_string(&mut st.strings));
    match res {
        Ok(val) => val,
        Err(NodeRecycleAbort::Overflow) => crate::errors::overflow(
            c"number of strings".as_ptr(),
            crate::globals::max_strings() - crate::globals::init_str_ptr(),
        ),
        Err(e) => {
            crate::scan::node_recycle_abort(e);
        }
    }
}

// ---------------------------------------------------------------------------
// append_str
// ---------------------------------------------------------------------------

/// Append string `s` to the current pool buffer (extending the string
/// being built by a prior `make_string` caller).
///
/// translated from xetex-stringpool.c:69 (append_str)
pub fn append_str(strings: &mut StringsState, s: i32) -> Result<(), NodeRecycleAbort> {
    let i = length(strings, s);
    if strings.pool_ptr + i > strings.pool_size {
        return Err(NodeRecycleAbort::Overflow);
    }
    let mut j = strings.str_start[(s - TOO_BIG_CHAR) as usize];
    let mut remaining = i;
    while remaining > 0 {
        strings.str_pool[strings.pool_ptr as usize] = strings.str_pool[j as usize];
        strings.pool_ptr += 1;
        j += 1;
        remaining -= 1;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// str_eq_buf
// ---------------------------------------------------------------------------

/// Return true if string `s` equals the buffer run starting at position `k`.
///
/// translated from xetex-stringpool.c:91 (str_eq_buf)
pub fn str_eq_buf(strings: &StringsState, buffer: &[i32], s: i32, mut k: i32) -> bool {
    let mut j = strings.str_start[(s - TOO_BIG_CHAR) as usize];
    while j < strings.str_start[(s + 1 - TOO_BIG_CHAR) as usize] {
        let bk = buffer[k as usize];
        if bk >= TOO_BIG_CHAR {
            // Surrogate pair in buffer: two pool slots needed.
            let hi = 0xD800 + (bk - TOO_BIG_CHAR) / 1024;
            let lo = 0xDC00 + (bk - TOO_BIG_CHAR) % 1024;
            if strings.str_pool[j as usize] as i32 != hi
                || strings.str_pool[(j + 1) as usize] as i32 != lo
            {
                return false;
            }
            j += 1;
        } else if strings.str_pool[j as usize] as i32 != bk {
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
pub fn str_eq_str(strings: &StringsState, s: i32, t: i32) -> bool {
    if length(strings, s) != length(strings, t) {
        return false;
    }
    if length(strings, s) == 1 {
        // Single-code-unit strings: may be implicit (< TOO_BIG_CHAR) or pooled.
        let cs = if s < TOO_BIG_CHAR {
            s as u16
        } else {
            strings.str_pool[strings.str_start[(s - TOO_BIG_CHAR) as usize] as usize]
        };
        let ct = if t < TOO_BIG_CHAR {
            t as u16
        } else {
            strings.str_pool[strings.str_start[(t - TOO_BIG_CHAR) as usize] as usize]
        };
        return cs == ct;
    }
    // General case: both are pooled strings.
    let mut j = strings.str_start[(s - TOO_BIG_CHAR) as usize];
    let mut k = strings.str_start[(t - TOO_BIG_CHAR) as usize];
    while j < strings.str_start[(s + 1 - TOO_BIG_CHAR) as usize] {
        if strings.str_pool[j as usize] != strings.str_pool[k as usize] {
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
pub fn search_string(strings: &StringsState, search: i32) -> i32 {
    let len = length(strings, search);
    if len == 0 {
        return EMPTY_STRING;
    }
    let mut s = search - 1;
    while s > TOO_BIG_CHAR - 1 {
        if length(strings, s) == len && str_eq_str(strings, s, search) {
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
pub fn slow_make_string(strings: &mut StringsState) -> Result<i32, NodeRecycleAbort> {
    let t = make_string(strings)?;
    let s = search_string(strings, t);
    if s > 0 {
        strings.str_ptr -= 1;
        strings.pool_ptr = strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize];
        return Ok(s);
    }
    Ok(t)
}

// ---------------------------------------------------------------------------
// load_pool_strings
// ---------------------------------------------------------------------------

/// Load compile-time string constants into the pool; return the last
/// str_number on success, 0 if the pool would overflow.
///
/// translated from xetex-stringpool.c:17 (load_pool_strings)
pub fn load_pool_strings(
    strings: &mut StringsState,
    spare_size: i32,
) -> Result<i32, NodeRecycleAbort> {
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
            return Ok(0);
        }
        for &byte in *s {
            strings.str_pool[strings.pool_ptr as usize] = byte as u16;
            strings.pool_ptr += 1;
        }
        g = make_string(strings)?;
    }

    Ok(g)
}

pub unsafe fn load_pool_strings_global(spare_size: i32) -> i32 {
    let res = crate::state::with_state_mut(|st| load_pool_strings(&mut st.strings, spare_size));
    match res {
        Ok(val) => val,
        Err(e) => {
            crate::scan::node_recycle_abort(e);
        }
    }
}

// ---------------------------------------------------------------------------
// flush_str
// ---------------------------------------------------------------------------

/// Free the top-of-pool string if it equals s.
///
/// translated from xetex-xetex0.c:15948 (flush_str)
pub fn flush_str(strings: &mut StringsState, s: i32) {
    if s == strings.str_ptr - 1 {
        strings.str_ptr -= 1;
        strings.pool_ptr = strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize];
    }
}
