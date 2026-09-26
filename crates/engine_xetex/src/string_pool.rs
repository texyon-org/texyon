//! String pool: TeX strings stored as UTF-16 code units in `str_pool`.
//! Strings below `TOO_BIG_CHAR` are single characters stored implicitly.

use crate::consts::{EMPTY_STRING, TOO_BIG_CHAR};
use crate::scan::NodeRecycleAbort;
use crate::state::StringsState;

fn str_start(strings: &StringsState, s: i32) -> i32 {
    strings.str_start[(s - TOO_BIG_CHAR) as usize]
}

pub fn length(strings: &StringsState, s: i32) -> i32 {
    if s >= TOO_BIG_CHAR {
        return str_start(strings, s + 1) - str_start(strings, s);
    }
    match s {
        32..=126 => 1,
        ..=127 => 3,
        128..=255 => 4,
        _ => 8,
    }
}

pub fn make_string(strings: &mut StringsState) -> Result<i32, NodeRecycleAbort> {
    if strings.str_ptr == strings.max_strings {
        return Err(NodeRecycleAbort::Overflow);
    }
    strings.str_ptr += 1;
    let slot = strings.cur_start_slot();
    strings.str_start[slot] = strings.pool_ptr;
    Ok(strings.str_ptr - 1)
}

pub fn append_str(strings: &mut StringsState, s: i32) -> Result<(), NodeRecycleAbort> {
    let len = length(strings, s);
    if strings.pool_ptr + len > strings.pool_size {
        return Err(NodeRecycleAbort::Overflow);
    }
    let src = str_start(strings, s) as usize;
    let dst = strings.pool_ptr as usize;
    for i in 0..len.max(0) as usize {
        strings.str_pool[dst + i] = strings.str_pool[src + i];
    }
    strings.pool_ptr += len.max(0);
    Ok(())
}

/// Whether string `s` equals the buffer run starting at `k`.
pub fn str_eq_buf(strings: &StringsState, buffer: &[i32], s: i32, mut k: i32) -> bool {
    let mut j = str_start(strings, s);
    let end = str_start(strings, s + 1);
    while j < end {
        let c = buffer[k as usize];
        if c >= TOO_BIG_CHAR {
            let hi = 0xD800 + (c - TOO_BIG_CHAR) / 1024;
            let lo = 0xDC00 + (c - TOO_BIG_CHAR) % 1024;
            if strings.str_pool[j as usize] as i32 != hi
                || strings.str_pool[(j + 1) as usize] as i32 != lo
            {
                return false;
            }
            j += 1;
        } else if strings.str_pool[j as usize] as i32 != c {
            return false;
        }
        j += 1;
        k += 1;
    }
    true
}

pub fn str_eq_str(strings: &StringsState, s: i32, t: i32) -> bool {
    let len = length(strings, s);
    if len != length(strings, t) {
        return false;
    }
    if len == 1 {
        let first_unit = |x: i32| {
            if x < TOO_BIG_CHAR {
                x as u16
            } else {
                strings.str_pool[str_start(strings, x) as usize]
            }
        };
        return first_unit(s) == first_unit(t);
    }
    let j = str_start(strings, s) as usize;
    let k = str_start(strings, t) as usize;
    let n = (str_start(strings, s + 1) as usize).saturating_sub(j);
    strings.str_pool[j..j + n] == strings.str_pool[k..k + n]
}

/// An existing string equal to `search`, or 0 if none.
pub fn search_string(strings: &StringsState, search: i32) -> i32 {
    let len = length(strings, search);
    if len == 0 {
        return EMPTY_STRING;
    }
    (TOO_BIG_CHAR..search)
        .rev()
        .find(|&s| length(strings, s) == len && str_eq_str(strings, s, search))
        .unwrap_or(0)
}

/// `make_string` that reuses an existing equal string instead.
pub fn slow_make_string(strings: &mut StringsState) -> Result<i32, NodeRecycleAbort> {
    let t = make_string(strings)?;
    let s = search_string(strings, t);
    if s > 0 {
        flush_str(strings, t);
        return Ok(s);
    }
    Ok(t)
}

/// Returns the last loaded string number, or 0 if the pool would overflow.
pub fn load_pool_strings(
    strings: &mut StringsState,
    spare_size: i32,
) -> Result<i32, NodeRecycleAbort> {
    const STRING_CONSTANTS: &[&[u8]] = &[b"this marks the start of the stringpool", b""];

    let mut total_len = 0;
    let mut g = 0;
    for s in STRING_CONSTANTS {
        total_len += s.len();
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

/// Frees `s` if it is the most recently made string.
pub fn flush_str(strings: &mut StringsState, s: i32) {
    if s == strings.str_ptr - 1 {
        strings.str_ptr -= 1;
        strings.pool_ptr = str_start(strings, strings.str_ptr);
    }
}
