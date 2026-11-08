//! Control-sequence hash table lookups.

use crate::consts::{
    BIGGEST_CHAR, EQTB_SIZE, HASH_BASE, PRIM_SIZE, TOO_BIG_CHAR, UNDEFINED_CONTROL_SEQUENCE,
};
use crate::errors::Abort;
use crate::state::{EqtbState, StringsState};
use crate::units::{EqtbPtr, PrimNum, StrNumber};

pub(crate) const HASH_OFFSET: i32 = 514;
pub(crate) const HASH_SIZE: i32 = 15000;
pub(crate) const HASH_PRIME: i32 = 8501;
const PRIM_PRIME: i32 = 1777;

/// The index of control sequence `p` in the `hash` array.
#[inline(always)]
pub(crate) fn hash_slot(p: EqtbPtr) -> i32 {
    p.raw() - HASH_OFFSET
}
const PRIM_BASE: PrimNum = PrimNum::from_raw(1);

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
    /// The control sequence named by the characters `name`, entered in the hash
    /// unless `no_new_control_sequence`.
    pub fn id_lookup(
        &mut self,
        strings: &mut StringsState,
        name: &[i32],
    ) -> Result<EqtbPtr, Abort> {
        let l = i32::try_from(name.len()).map_err(|_| strings.pool_overflow())?;
        let mut h: i32 = 0;
        for &c in name {
            h = h + h + c;
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

        let mut p = HASH_BASE + h;

        let utf16_len = name
            .iter()
            .fold(l, |n, &c| if c >= TOO_BIG_CHAR { n + 1 } else { n });

        loop {
            let text = self.text(p);
            if text > StrNumber::NONE
                && strings.length(text) == utf16_len
                && strings.str_eq_buf(name, text)
            {
                return Ok(p);
            }

            if self.hash_s0(p).is_none() {
                if self.no_new_control_sequence {
                    p = UNDEFINED_CONTROL_SEQUENCE;
                } else {
                    if text > StrNumber::NONE {
                        if self.hash_high < self.hash_extra {
                            self.hash_high += 1;
                            self.set_hash_s0(p, EQTB_SIZE + self.hash_high);
                            p = EQTB_SIZE + self.hash_high;
                        } else {
                            loop {
                                if self.hash_used == HASH_BASE {
                                    return Err(Abort::Overflow {
                                        capacity: crate::errors::Capacity::HashSize,
                                        n: HASH_SIZE + self.hash_extra,
                                    });
                                }
                                self.hash_used -= 1;
                                if self.text(self.hash_used) == StrNumber::NONE {
                                    break;
                                }
                            }
                            self.set_hash_s0(p, self.hash_used);
                            p = self.hash_used;
                        }
                    }

                    if strings.pool_ptr + utf16_len > strings.pool_size {
                        return Err(strings.pool_overflow());
                    }

                    let d = strings.cur_length();
                    while strings.pool_ptr > strings.cur_string_start() {
                        strings.pool_ptr -= 1;
                        strings.str_pool[(strings.pool_ptr + l).index()] =
                            strings.str_pool[strings.pool_ptr.index()];
                    }

                    for &bk in name {
                        if bk < TOO_BIG_CHAR {
                            strings.append_char(bk as u16);
                        } else {
                            let hi = 0xD800_i32 + (bk - TOO_BIG_CHAR) / 1024;
                            let lo = 0xDC00_i32 + (bk - TOO_BIG_CHAR) % 1024;
                            strings.append_char(hi as u16);
                            strings.append_char(lo as u16);
                        }
                    }

                    let s_idx = strings.make_string()?;
                    self.set_text(p, s_idx);
                    self.pgfmath.note(name, p);
                    strings.pool_ptr += d;
                }
                return Ok(p);
            }

            p = self.hash_s0(p);
        }
    }

    pub fn prim_lookup(
        &mut self,
        strings: &mut StringsState,
        s: StrNumber,
    ) -> Result<PrimNum, Abort> {
        let mut p;
        let l;
        if s.raw() <= BIGGEST_CHAR {
            if s.raw() < 0 {
                return Ok(PrimNum::UNDEFINED);
            }
            p = PrimNum::from_raw(s.raw() % PRIM_PRIME + 1);
            l = 0;
        } else {
            let j = strings.str_start(s);
            l = if s == strings.str_ptr {
                strings.cur_length()
            } else {
                strings.length(s)
            };
            let mut h = i32::from(strings.str_pool[j.index()]);
            for k in (j + 1).until(j + l) {
                h = h + h + i32::from(strings.str_pool[k.index()]);
                while h >= PRIM_PRIME {
                    h -= PRIM_PRIME;
                }
            }
            p = PrimNum::from_raw(h + 1);
        }

        loop {
            let text = self.prim_text(p);
            match text {
                Some(name) if name.is_pool_string() => {
                    if strings.length(name) == l && strings.str_eq_str(name, s) {
                        return Ok(p);
                    }
                }
                Some(name) if name == s => return Ok(p),
                _ => {}
            }

            if self.prim_next(p) == PrimNum::UNDEFINED {
                if self.no_new_control_sequence {
                    p = PrimNum::UNDEFINED;
                } else {
                    if text.is_some() {
                        loop {
                            if self.prim_used == PRIM_BASE {
                                return Err(Abort::Overflow {
                                    capacity: crate::errors::Capacity::PrimitiveSize,
                                    n: PRIM_SIZE,
                                });
                            }
                            self.prim_used = self.prim_used.prev();
                            if self.prim_text(self.prim_used).is_none() {
                                break;
                            }
                        }
                        self.set_prim_next(p, self.prim_used);
                        p = self.prim_used;
                    }
                    self.set_prim_text(p, Some(s));
                }
                return Ok(p);
            }

            p = self.prim_next(p);
        }
    }

    /// The primitive named like control sequence `cs`, if there is one.
    pub(crate) fn prim_lookup_cs(
        &mut self,
        strings: &mut StringsState,
        cs: EqtbPtr,
    ) -> Result<Option<PrimNum>, Abort> {
        let s = if cs < HASH_BASE {
            StrNumber::from_raw(cs - crate::consts::SINGLE_BASE)
        } else {
            self.text(cs)
        };
        let p = self.prim_lookup(strings, s)?;
        Ok((p != PrimNum::UNDEFINED).then_some(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::PoolPos;

    fn tables() -> (EqtbState, StringsState) {
        let eqtb = EqtbState {
            prim_used: PrimNum::from_raw(PRIM_SIZE),
            ..EqtbState::default()
        };
        let strings = StringsState {
            str_ptr: StrNumber::from_raw(TOO_BIG_CHAR),
            max_strings: StrNumber::from_raw(TOO_BIG_CHAR + 8),
            pool_size: PoolPos::from_raw(32),
            str_start: vec![PoolPos::ZERO; 9],
            str_pool: vec![0; 33],
            ..StringsState::default()
        };
        (eqtb, strings)
    }

    fn make(strings: &mut StringsState, text: &str) -> StrNumber {
        strings.append_units(&text.encode_utf16().collect::<Vec<_>>());
        strings.make_string().expect("room for the string")
    }

    #[test]
    fn prim_lookup_chains_colliding_names() {
        let (mut eqtb, mut strings) = tables();
        let a = StrNumber::from_raw(97);
        let b = StrNumber::from_raw(97 + PRIM_PRIME);
        let first = eqtb.prim_lookup(&mut strings, a).expect("lookup");
        let second = eqtb.prim_lookup(&mut strings, b).expect("lookup");
        assert_eq!(first, PrimNum::from_raw(98));
        assert_eq!(second, PrimNum::from_raw(PRIM_SIZE - 1));
        assert_eq!(eqtb.prim_next(first), second);
        assert_eq!(eqtb.prim_text(second), Some(b));
        assert_eq!(eqtb.prim_lookup(&mut strings, b).expect("lookup"), second);
        assert_eq!(eqtb.prim_lookup(&mut strings, a).expect("lookup"), first);

        eqtb.no_new_control_sequence = true;
        let c = StrNumber::from_raw(97 + 2 * PRIM_PRIME);
        assert_eq!(
            eqtb.prim_lookup(&mut strings, c).expect("lookup"),
            PrimNum::UNDEFINED
        );
    }

    #[test]
    fn prim_lookup_matches_pool_names_by_content() {
        let (mut eqtb, mut strings) = tables();
        // Same length and same hash bucket.
        let ac = make(&mut strings, "ac");
        let ba = make(&mut strings, "ba");
        let p = eqtb.prim_lookup(&mut strings, ac).expect("lookup");
        let again = make(&mut strings, "ac");
        assert_eq!(eqtb.prim_lookup(&mut strings, again).expect("lookup"), p);

        eqtb.no_new_control_sequence = true;
        assert_eq!(
            eqtb.prim_lookup(&mut strings, ba).expect("lookup"),
            PrimNum::UNDEFINED
        );
    }
}
