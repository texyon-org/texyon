//! Control-sequence hash table lookups.

use crate::consts::{
    BIGGEST_CHAR, EQTB_SIZE, HASH_BASE, PRIM_SIZE, TOO_BIG_CHAR, UNDEFINED_CONTROL_SEQUENCE,
    UNDEFINED_PRIMITIVE,
};
use crate::errors::Abort;
use crate::state::{EqtbState, StringsState};

pub(crate) const HASH_OFFSET: i32 = 514;
pub(crate) const HASH_SIZE: i32 = 15000;
pub(crate) const HASH_PRIME: i32 = 8501;
const PRIM_PRIME: i32 = 1777;
const PRIM_BASE: i32 = 1;

impl StringsState {
    #[inline]
    pub(crate) fn cur_length(&self) -> i32 {
        self.pool_ptr - self.cur_string_start()
    }
}

/// Kept out of line so LLVM cannot hoist the modulo back into `id_lookup`'s loop.
#[cold]
#[inline(never)]
fn hash_mod_rare(h: i32) -> i32 {
    h % HASH_PRIME
}

impl EqtbState {
    pub fn id_lookup(
        &mut self,
        strings: &mut StringsState,
        buffer: &[i32],
        j: i32,
        l: i32,
    ) -> Result<i32, Abort> {
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
            let text = self.hash_s1(p);
            if text > 0 && strings.length(text) == utf16_len && strings.str_eq_buf(buffer, text, j)
            {
                return Ok(p);
            }

            if self.hash_s0(p) == 0 {
                if self.no_new_control_sequence {
                    p = UNDEFINED_CONTROL_SEQUENCE;
                } else {
                    if text > 0 {
                        if self.hash_high < self.hash_extra {
                            self.hash_high += 1;
                            self.set_hash_s0(p, self.hash_high + EQTB_SIZE);
                            p = self.hash_high + EQTB_SIZE;
                        } else {
                            loop {
                                if self.hash_used == HASH_BASE {
                                    return Err(Abort::Overflow {
                                        what: "hash size",
                                        n: HASH_SIZE + self.hash_extra,
                                    });
                                }
                                self.hash_used -= 1;
                                if self.hash_s1(self.hash_used) == 0 {
                                    break;
                                }
                            }
                            self.set_hash_s0(p, self.hash_used);
                            p = self.hash_used;
                        }
                    }

                    if strings.pool_ptr + utf16_len > strings.pool_size {
                        return Err(Abort::PoolOverflow);
                    }

                    let d = strings.cur_length();
                    while strings.pool_ptr > strings.cur_string_start() {
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

                    let s_idx = strings.make_string()?;
                    self.set_hash_s1(p, s_idx);
                    strings.pool_ptr += d;
                }
                return Ok(p);
            }

            p = self.hash_s0(p);
        }
    }

    pub fn prim_lookup(&mut self, strings: &mut StringsState, s: i32) -> Result<i32, Abort> {
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
                strings.cur_length()
            } else {
                strings.length(s)
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
            let text = self.prim_s1(p);
            if text > TOO_BIG_CHAR {
                if strings.length(text - 1) == l && strings.str_eq_str(text - 1, s) {
                    return Ok(p);
                }
            } else if text == 1 + s {
                return Ok(p);
            }

            if self.prim_s0(p) == 0 {
                if self.no_new_control_sequence {
                    p = UNDEFINED_PRIMITIVE;
                } else {
                    if text > 0 {
                        loop {
                            if self.prim_used == PRIM_BASE {
                                return Err(Abort::Overflow {
                                    what: "primitive size",
                                    n: PRIM_SIZE,
                                });
                            }
                            self.prim_used -= 1;
                            if self.prim_s1(self.prim_used) == 0 {
                                break;
                            }
                        }
                        self.set_prim_s0(p, self.prim_used);
                        p = self.prim_used;
                    }
                    self.set_prim_s1(p, s + 1);
                }
                return Ok(p);
            }

            p = self.prim_s0(p);
        }
    }

    /// `prim_lookup` of the name of control sequence `cs`.
    pub(crate) fn prim_lookup_cs(
        &mut self,
        strings: &mut StringsState,
        cs: i32,
    ) -> Result<i32, Abort> {
        let s = if cs < HASH_BASE {
            cs - crate::consts::SINGLE_BASE
        } else {
            self.hash_s1(cs)
        };
        self.prim_lookup(strings, s)
    }
}
