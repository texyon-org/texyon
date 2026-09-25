//! Control-sequence hash table lookups.

use crate::consts::{
    BIGGEST_CHAR, EQTB_SIZE, TOO_BIG_CHAR, UNDEFINED_CONTROL_SEQUENCE, UNDEFINED_PRIMITIVE,
};
use crate::state::{EqtbState, StringsState};
use crate::string_pool::{length, make_string, str_eq_buf, str_eq_str};

pub(crate) const HASH_BASE: i32 = 2_228_226;
const HASH_SIZE: i32 = 15000;
const HASH_PRIME: i32 = 8501;
const PRIM_SIZE: i32 = 2100;
const PRIM_PRIME: i32 = 1777;
const PRIM_BASE: i32 = 1;

#[derive(Debug, Clone, Copy)]
pub(crate) enum HashLookupError {
    HashSizeOverflow(i32),
    PoolSizeOverflow(i32),
    StringNumberOverflow(i32),
    PrimitiveSizeOverflow(i32),
}

#[inline]
pub(crate) fn cur_length(strings: &StringsState) -> i32 {
    strings.pool_ptr - strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize]
}

/// Kept out of line so LLVM cannot hoist the modulo back into `id_lookup`'s loop.
#[cold]
#[inline(never)]
fn hash_mod_rare(h: i32) -> i32 {
    h % HASH_PRIME
}

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
        // Peeled subtractions: a plain while loop becomes a slow imul modulo chain.
        if h >= HASH_PRIME {
            h -= HASH_PRIME;
            if h >= HASH_PRIME {
                h -= HASH_PRIME;
                if h >= HASH_PRIME {
                    h = hash_mod_rare(h);
                }
            }
        }
    }

    let mut p = h + HASH_BASE;

    let mut utf16_len = l;
    for k in j..j + l {
        if buffer[k as usize] >= TOO_BIG_CHAR {
            utf16_len += 1;
        }
    }

    loop {
        let text = eqtb.hash_s1(p);
        if text > 0 && length(strings, text) == utf16_len && str_eq_buf(strings, buffer, text, j) {
            return Ok(p);
        }

        if eqtb.hash_s0(p) == 0 {
            if eqtb.no_new_control_sequence {
                p = UNDEFINED_CONTROL_SEQUENCE;
            } else {
                if text > 0 {
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

                if strings.pool_ptr + utf16_len > strings.pool_size {
                    return Err(HashLookupError::PoolSizeOverflow(
                        strings.pool_size - strings.init_pool_ptr,
                    ));
                }

                let d = cur_length(strings);
                while strings.pool_ptr
                    > strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize]
                {
                    strings.pool_ptr -= 1;
                    strings.str_pool[(strings.pool_ptr + l) as usize] =
                        strings.str_pool[strings.pool_ptr as usize];
                }

                for k in j..j + l {
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

                let s_idx = make_string(strings).map_err(|_| {
                    HashLookupError::StringNumberOverflow(
                        strings.max_strings - strings.init_str_ptr,
                    )
                })?;
                eqtb.set_hash_s1(p, s_idx);
                strings.pool_ptr += d;
            }
            return Ok(p);
        }

        p = eqtb.hash_s0(p);
    }
}

pub fn prim_lookup(
    eqtb: &mut EqtbState,
    strings: &mut StringsState,
    s: i32,
) -> Result<i32, HashLookupError> {
    let mut p;
    let l;
    if s <= BIGGEST_CHAR {
        if s < 0 {
            return Ok(UNDEFINED_PRIMITIVE);
        }
        p = s % PRIM_PRIME + 1;
        l = 0;
    } else {
        let j = strings.str_start[(s - TOO_BIG_CHAR) as usize];
        l = if s == strings.str_ptr {
            cur_length(strings)
        } else {
            length(strings, s)
        };
        let mut h = strings.str_pool[j as usize] as i32;
        for k in j + 1..j + l {
            h = h + h + strings.str_pool[k as usize] as i32;
            while h >= PRIM_PRIME {
                h -= PRIM_PRIME;
            }
        }
        p = h + 1;
    }

    loop {
        let text = eqtb.prim_s1(p);
        if text > TOO_BIG_CHAR {
            if length(strings, text) - 1 == l && str_eq_str(strings, text - 1, s) {
                return Ok(p);
            }
        } else if text == 1 + s {
            return Ok(p);
        }

        if eqtb.prim_s0(p) == 0 {
            if eqtb.no_new_control_sequence {
                p = UNDEFINED_PRIMITIVE;
            } else {
                if text > 0 {
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

/// `prim_lookup` of the name of control sequence `cs`.
pub(crate) fn prim_lookup_cs(
    eqtb: &mut EqtbState,
    strings: &mut StringsState,
    cs: i32,
) -> Result<i32, HashLookupError> {
    let s = if cs < HASH_BASE {
        cs - crate::consts::SINGLE_BASE
    } else {
        eqtb.hash_s1(cs)
    };
    prim_lookup(eqtb, strings, s)
}
