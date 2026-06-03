//! Control-sequence hash table lookups (id_lookup, prim_lookup).
//!
//! Translated from xetex-xetex0.c.
use crate::consts::{BIGGEST_CHAR, EQTB_SIZE, TOO_BIG_CHAR};
use crate::state::{EqtbState, StringsState};

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

#[derive(Debug, Clone, Copy)]
pub(crate) enum HashLookupError {
    HashSizeOverflow(i32),
    PoolSizeOverflow(i32),
    StringNumberOverflow(i32),
    PrimitiveSizeOverflow(i32),
}

/// Inline equivalent of `cur_length()` (xetex-xetexd.h:1175).
#[inline]
pub(crate) fn cur_length(strings: &StringsState) -> i32 {
    strings.pool_ptr - strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize]
}

#[inline]
pub(crate) fn cur_length_global() -> i32 {
    crate::state::with_state(|st| cur_length(&st.strings))
}

// translated from xetex-xetex0.c:2713 (id_lookup)
pub fn id_lookup(
    eqtb: &mut EqtbState,
    strings: &mut StringsState,
    buffer: &[i32],
    j: i32,
    l: i32,
) -> Result<i32, HashLookupError> {
    let mut h: i32 = 0;
    for k in j..=(j + l - 1) {
        h = h + h + buffer[k as usize];
        while h >= HASH_PRIME {
            h -= HASH_PRIME;
        }
    }

    let mut p = h + HASH_BASE;

    // Count ll = UTF-16 code units (surrogate pairs expand buffer scalars >= 65536)
    let mut ll = l;
    for d in 0..=(l - 1) {
        if buffer[(j + d) as usize] >= TOO_BIG_CHAR {
            ll += 1;
        }
    }

    loop {
        if eqtb.hash_s1(p) > 0
            && crate::string_pool::length(strings, eqtb.hash_s1(p)) == ll
            && crate::string_pool::str_eq_buf(strings, buffer, eqtb.hash_s1(p), j)
        {
            return Ok(p); // found
        }

        if eqtb.hash_s0(p) == 0 {
            if eqtb.no_new_control_sequence {
                p = UNDEFINED_CONTROL_SEQUENCE;
            } else {
                // new entry — allocate a slot
                if eqtb.hash_s1(p) > 0 {
                    // current slot occupied: find a free slot
                    if eqtb.hash_high < eqtb.hash_extra {
                        eqtb.hash_high += 1;
                        eqtb.set_hash_s0(p, eqtb.hash_high + EQTB_SIZE);
                        p = eqtb.hash_high + EQTB_SIZE;
                    } else {
                        loop {
                            if eqtb.hash_used == HASH_BASE {
                                return Err(HashLookupError::HashSizeOverflow(
                                    HASH_SIZE + eqtb.hash_extra,
                                ));
                            }
                            eqtb.hash_used -= 1;
                            if eqtb.hash_s1(eqtb.hash_used) == 0 {
                                break;
                            }
                        }
                        eqtb.set_hash_s0(p, eqtb.hash_used);
                        p = eqtb.hash_used;
                    }
                }

                // copy buffer[j..j+l-1] into the string pool, making room
                if strings.pool_ptr + ll > strings.pool_size {
                    return Err(HashLookupError::PoolSizeOverflow(
                        strings.pool_size - strings.init_pool_ptr,
                    ));
                }

                let d = cur_length(strings);

                // shift current partial string up by l positions
                while strings.pool_ptr
                    > strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize]
                {
                    strings.pool_ptr -= 1;
                    strings.str_pool[(strings.pool_ptr + l) as usize] =
                        strings.str_pool[strings.pool_ptr as usize];
                }

                // encode buffer[j..j+l-1] as UTF-16 (surrogate pairs if needed)
                for k in j..=(j + l - 1) {
                    let bk = buffer[k as usize];
                    if bk < TOO_BIG_CHAR {
                        strings.str_pool[strings.pool_ptr as usize] = bk as u16;
                        strings.pool_ptr += 1;
                    } else {
                        let hi = 0xD800_i32 + (bk - TOO_BIG_CHAR) / 1024;
                        let lo = 0xDC00_i32 + (bk - TOO_BIG_CHAR) % 1024;
                        strings.str_pool[strings.pool_ptr as usize] = hi as u16;
                        strings.pool_ptr += 1;
                        strings.str_pool[strings.pool_ptr as usize] = lo as u16;
                        strings.pool_ptr += 1;
                    }
                }

                let s_idx = match crate::string_pool::make_string(strings) {
                    Ok(val) => val,
                    Err(_) => {
                        return Err(HashLookupError::StringNumberOverflow(
                            strings.max_strings - strings.init_str_ptr,
                        ));
                    }
                };
                eqtb.set_hash_s1(p, s_idx);
                strings.pool_ptr += d;
            }
            return Ok(p); // found (new or undefined)
        }

        p = eqtb.hash_s0(p);
    }
}

// translated from xetex-xetex0.c:2803 (prim_lookup)
pub fn prim_lookup(
    eqtb: &mut EqtbState,
    strings: &mut StringsState,
    s: i32,
) -> Result<i32, HashLookupError> {
    let mut p: i32;
    // l mirrors the C local: 0 for single-char s, pool-length for pooled s.
    let l: i32;

    if s <= BIGGEST_CHAR {
        if s < 0 {
            return Ok(UNDEFINED_PRIMITIVE);
        }
        p = (s % PRIM_PRIME) + 1;
        l = 0; // not used in the BIGGEST_CHAR path
    } else {
        let j = strings.str_start[(s - TOO_BIG_CHAR) as usize];
        l = if s == strings.str_ptr {
            cur_length(strings)
        } else {
            crate::string_pool::length(strings, s)
        };
        let mut h = strings.str_pool[j as usize] as i32;
        let for_end = j + l - 1;
        let mut k = j + 1;
        while k <= for_end {
            h = h + h + strings.str_pool[k as usize] as i32;
            while h >= PRIM_PRIME {
                h -= PRIM_PRIME;
            }
            k += 1;
        }
        p = h + 1;
    }

    loop {
        if eqtb.prim_s1(p) > TOO_BIG_CHAR {
            if crate::string_pool::length(strings, eqtb.prim_s1(p)) - 1 == l
                && crate::string_pool::str_eq_str(strings, eqtb.prim_s1(p) - 1, s)
            {
                return Ok(p);
            }
        } else if eqtb.prim_s1(p) == 1 + s {
            return Ok(p);
        }

        if eqtb.prim_s0(p) == 0 {
            if eqtb.no_new_control_sequence {
                p = UNDEFINED_PRIMITIVE;
            } else {
                if eqtb.prim_s1(p) > 0 {
                    loop {
                        if eqtb.prim_used == PRIM_BASE {
                            return Err(HashLookupError::PrimitiveSizeOverflow(PRIM_SIZE));
                        }
                        eqtb.prim_used -= 1;
                        if eqtb.prim_s1(eqtb.prim_used) == 0 {
                            break;
                        }
                    }
                    eqtb.set_prim_s0(p, eqtb.prim_used);
                    p = eqtb.prim_used;
                }
                eqtb.set_prim_s1(p, s + 1);
            }
            return Ok(p);
        }

        p = eqtb.prim_s0(p);
    }
}

pub unsafe fn id_lookup_global(j: i32, l: i32) -> i32 {
    let res = crate::state::with_state_mut(|st| {
        id_lookup(&mut st.eqtb, &mut st.strings, &st.input.buffer, j, l)
    });
    match res {
        Ok(p) => p,
        Err(HashLookupError::HashSizeOverflow(limit)) => {
            crate::errors::overflow(c"hash size".as_ptr(), limit);
        }
        Err(HashLookupError::PoolSizeOverflow(limit)) => {
            crate::errors::overflow(c"pool size".as_ptr(), limit);
        }
        Err(HashLookupError::StringNumberOverflow(limit)) => {
            crate::errors::overflow(c"number of strings".as_ptr(), limit);
        }
        Err(HashLookupError::PrimitiveSizeOverflow(limit)) => {
            crate::errors::overflow(c"primitive size".as_ptr(), limit);
        }
    }
}

pub unsafe fn prim_lookup_global(s: i32) -> i32 {
    let res = crate::state::with_state_mut(|st| {
        prim_lookup(&mut st.eqtb, &mut st.strings, s)
    });
    match res {
        Ok(p) => p,
        Err(HashLookupError::HashSizeOverflow(limit)) => {
            crate::errors::overflow(c"hash size".as_ptr(), limit);
        }
        Err(HashLookupError::PoolSizeOverflow(limit)) => {
            crate::errors::overflow(c"pool size".as_ptr(), limit);
        }
        Err(HashLookupError::StringNumberOverflow(limit)) => {
            crate::errors::overflow(c"number of strings".as_ptr(), limit);
        }
        Err(HashLookupError::PrimitiveSizeOverflow(limit)) => {
            crate::errors::overflow(c"primitive size".as_ptr(), limit);
        }
    }
}

#[export_name = "id_lookup"]
pub unsafe extern "C" fn id_lookup_c(j: i32, l: i32) -> i32 {
    id_lookup_global(j, l)
}

#[export_name = "prim_lookup"]
pub unsafe extern "C" fn prim_lookup_c(s: i32) -> i32 {
    prim_lookup_global(s)
}
