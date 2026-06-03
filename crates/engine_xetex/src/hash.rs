//! Control-sequence hash table lookups (id_lookup, prim_lookup).
//!
//! Translated from xetex-xetex0.c.
use crate::consts::{BIGGEST_CHAR, EQTB_SIZE, TOO_BIG_CHAR};
use crate::errors::overflow;
use crate::string_pool::length_global as length;
use crate::string_pool::make_string_global as make_string;
use crate::string_pool::str_eq_buf_global as str_eq_buf;
use crate::string_pool::str_eq_str_global as str_eq_str;

use crate::globals::{
    buffer_entry, hash_extra, hash_high, hash_s0, hash_s1, hash_used, init_pool_ptr,
    no_new_control_sequence, pool_ptr, pool_size, prim_s0, prim_s1, prim_used, set_hash_high,
    set_hash_s0, set_hash_s1, set_hash_used, set_pool_ptr, set_prim_s0, set_prim_s1, set_prim_used,
    set_str_pool, str_pool_entry, str_ptr, str_start,
};

extern "C" {}

// xetex-constants.h:25
// xetex-constants.h:340
// xetex_format.h:15
const HASH_SIZE: i32 = 15000;
// xetex_format.h:18
const HASH_PRIME: i32 = 8501;
// xetex_format.h:27
const PRIM_SIZE: i32 = 2100;
// xetex_format.h:28
const PRIM_PRIME: i32 = 1777;
// xetex-constants.h:270
const PRIM_BASE: i32 = 1;
// xetex_format.h:200
const HASH_BASE: i32 = 2_228_226;
// xetex_format.h:204
const UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
// xetex_format.h:224
// xetex-constants.h:262
const UNDEFINED_PRIMITIVE: i32 = 0;

/// Inline equivalent of `cur_length()` (xetex-xetexd.h:1175).
#[inline]
pub(crate) fn cur_length() -> i32 {
    pool_ptr() - str_start(str_ptr() - TOO_BIG_CHAR)
}

// translated from xetex-xetex0.c:2713 (id_lookup)
#[no_mangle]
pub unsafe extern "C" fn id_lookup(j: i32, l: i32) -> i32 {
    let mut h: i32 = 0;
    for k in j..=(j + l - 1) {
        h = h + h + buffer_entry(k);
        while h >= HASH_PRIME {
            h -= HASH_PRIME;
        }
    }

    let mut p = h + HASH_BASE;

    // Count ll = UTF-16 code units (surrogate pairs expand buffer scalars >= 65536)
    let mut ll = l;
    for d in 0..=(l - 1) {
        if buffer_entry(j + d) >= TOO_BIG_CHAR {
            ll += 1;
        }
    }

    loop {
        if hash_s1(p) > 0 && length(hash_s1(p)) == ll && str_eq_buf(hash_s1(p), j) {
            return p; // found
        }

        if hash_s0(p) == 0 {
            if no_new_control_sequence() {
                p = UNDEFINED_CONTROL_SEQUENCE;
            } else {
                // new entry — allocate a slot
                if hash_s1(p) > 0 {
                    // current slot occupied: find a free slot
                    if hash_high() < hash_extra() {
                        set_hash_high(hash_high() + 1);
                        set_hash_s0(p, hash_high() + EQTB_SIZE);
                        p = hash_high() + EQTB_SIZE;
                    } else {
                        loop {
                            if hash_used() == HASH_BASE {
                                overflow(c"hash size".as_ptr(), HASH_SIZE + hash_extra());
                            }
                            set_hash_used(hash_used() - 1);
                            if hash_s1(hash_used()) == 0 {
                                break;
                            }
                        }
                        set_hash_s0(p, hash_used());
                        p = hash_used();
                    }
                }

                // copy buffer[j..j+l-1] into the string pool, making room
                if pool_ptr() + ll > pool_size() {
                    overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr());
                }

                let d = cur_length();

                // shift current partial string up by l positions
                while pool_ptr() > str_start(str_ptr() - TOO_BIG_CHAR) {
                    set_pool_ptr(pool_ptr() - 1);
                    set_str_pool(pool_ptr() + l, str_pool_entry(pool_ptr()));
                }

                // encode buffer[j..j+l-1] as UTF-16 (surrogate pairs if needed)
                for k in j..=(j + l - 1) {
                    let bk = buffer_entry(k);
                    if bk < TOO_BIG_CHAR {
                        set_str_pool(pool_ptr(), bk as u16);
                        set_pool_ptr(pool_ptr() + 1);
                    } else {
                        let hi = 0xD800_i32 + (bk - TOO_BIG_CHAR) / 1024;
                        let lo = 0xDC00_i32 + (bk - TOO_BIG_CHAR) % 1024;
                        set_str_pool(pool_ptr(), hi as u16);
                        set_pool_ptr(pool_ptr() + 1);
                        set_str_pool(pool_ptr(), lo as u16);
                        set_pool_ptr(pool_ptr() + 1);
                    }
                }

                set_hash_s1(p, make_string());
                set_pool_ptr(pool_ptr() + d);
            }
            return p; // found (new or undefined)
        }

        p = hash_s0(p);
    }
}

// translated from xetex-xetex0.c:2803 (prim_lookup)
#[no_mangle]
pub unsafe extern "C" fn prim_lookup(s: i32) -> i32 {
    let mut p: i32;
    // l mirrors the C local: 0 for single-char s, pool-length for pooled s.
    let l: i32;

    if s <= BIGGEST_CHAR {
        if s < 0 {
            return UNDEFINED_PRIMITIVE;
        }
        p = (s % PRIM_PRIME) + 1;
        l = 0; // not used in the BIGGEST_CHAR path
    } else {
        let j = str_start(s - TOO_BIG_CHAR);
        l = if s == str_ptr() {
            cur_length()
        } else {
            length(s)
        };
        let mut h = str_pool_entry(j) as i32;
        let for_end = j + l - 1;
        let mut k = j + 1;
        while k <= for_end {
            h = h + h + str_pool_entry(k) as i32;
            while h >= PRIM_PRIME {
                h -= PRIM_PRIME;
            }
            k += 1;
        }
        p = h + 1;
    }

    loop {
        if prim_s1(p) > TOO_BIG_CHAR {
            if length(prim_s1(p)) - 1 == l && str_eq_str(prim_s1(p) - 1, s) {
                return p;
            }
        } else if prim_s1(p) == 1 + s {
            return p;
        }

        if prim_s0(p) == 0 {
            if no_new_control_sequence() {
                p = UNDEFINED_PRIMITIVE;
            } else {
                if prim_s1(p) > 0 {
                    loop {
                        if prim_used() == PRIM_BASE {
                            overflow(c"primitive size".as_ptr(), PRIM_SIZE);
                        }
                        set_prim_used(prim_used() - 1);
                        if prim_s1(prim_used()) == 0 {
                            break;
                        }
                    }
                    set_prim_s0(p, prim_used());
                    p = prim_used();
                }
                set_prim_s1(p, s + 1);
            }
            return p;
        }

        p = prim_s0(p);
    }
}
