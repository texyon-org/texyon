//! Control-sequence hash table lookups.

use crate::base::consts::{BIGGEST_CHAR, PRIM_SIZE, TOO_BIG_CHAR};
use crate::base::units::{PrimNum, StrNumber};
use crate::memory::mem_types::ZeroWord;
use crate::print::errors::{Abort, Capacity};
use crate::tables::cs::{CsId, CsKind, NameSlot};
use crate::tables::eqtb::EqtbState;
use crate::tables::string_pool::StringsState;

/// A multi-letter control sequence's slot in the hash table.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[repr(C)]
pub(crate) struct HashEntry {
    /// The next slot in the chain of names with the same hash.
    pub next: Option<CsId>,
    /// The name, `StrNumber::NONE` while the slot is free.
    pub text: StrNumber,
}

// SAFETY: `None` and `StrNumber(0)` are both all-zero bits.
unsafe impl ZeroWord for HashEntry {}

/// A slot of the `\primitive` lookup table.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub(crate) struct PrimEntry {
    /// The next slot in the chain of names with the same hash.
    pub next: Option<PrimNum>,
    /// The primitive's name, `None` while the slot is free.
    pub text: Option<StrNumber>,
}

pub(crate) const HASH_SIZE: i32 = 15000;
pub(crate) const HASH_PRIME: i32 = 8501;
const PRIM_PRIME: i32 = 1777;

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
    pub fn id_lookup(&mut self, strings: &mut StringsState, name: &[i32]) -> Result<CsId, Abort> {
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

        let mut p = NameSlot::bucket(h.cast_unsigned()).cs();

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

            let Some(next) = self.hash_next(p) else {
                if self.no_new_control_sequence {
                    p = CsId::UNDEFINED;
                } else {
                    if text > StrNumber::NONE {
                        if self.hash_high < self.hash_extra {
                            self.hash_high += 1;
                            let extra = NameSlot::extra(self.hash_high).cs();
                            self.set_hash_next(p, extra);
                            p = extra;
                        } else {
                            loop {
                                let Some(prev) = self.hash_used.prev_bucket() else {
                                    return Err(Abort::Overflow {
                                        capacity: Capacity::HashSize,
                                        n: HASH_SIZE.saturating_add_unsigned(self.hash_extra),
                                    });
                                };
                                self.hash_used = prev;
                                if self.text(prev.cs()) == StrNumber::NONE {
                                    break;
                                }
                            }
                            let free = self.hash_used.cs();
                            self.set_hash_next(p, free);
                            p = free;
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
            };

            p = next;
        }
    }

    pub fn prim_lookup(
        &mut self,
        strings: &mut StringsState,
        s: StrNumber,
    ) -> Result<Option<PrimNum>, Abort> {
        let mut p;
        let l;
        if s.raw() <= BIGGEST_CHAR {
            let Ok(c) = u32::try_from(s.raw()) else {
                return Ok(None);
            };
            p = PrimNum::FIRST.plus(c % PRIM_PRIME.cast_unsigned());
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
            p = PrimNum::FIRST.plus(h.cast_unsigned());
        }

        loop {
            let text = self.prim_text(p);
            match text {
                Some(name) if name.is_pool_string() => {
                    if strings.length(name) == l && strings.str_eq_str(name, s) {
                        return Ok(Some(p));
                    }
                }
                Some(name) if name == s => return Ok(Some(p)),
                _ => {}
            }

            let Some(next) = self.prim_next(p) else {
                if self.no_new_control_sequence {
                    return Ok(None);
                }
                if text.is_some() {
                    loop {
                        let Some(prev) = self.prim_used.prev() else {
                            return Err(Abort::Overflow {
                                capacity: Capacity::PrimitiveSize,
                                n: PRIM_SIZE,
                            });
                        };
                        self.prim_used = prev;
                        if self.prim_text(self.prim_used).is_none() {
                            break;
                        }
                    }
                    self.set_prim_next(p, Some(self.prim_used));
                    p = self.prim_used;
                }
                self.set_prim_text(p, Some(s));
                return Ok(Some(p));
            };

            p = next;
        }
    }

    /// The primitive named like control sequence `cs`, if there is one.
    pub(crate) fn prim_lookup_cs(
        &mut self,
        strings: &mut StringsState,
        cs: Option<CsId>,
    ) -> Result<Option<PrimNum>, Abort> {
        let Some(cs) = cs else {
            return Ok(None);
        };
        let s = match cs.kind() {
            CsKind::Active(_) => return Ok(None),
            CsKind::Single(c) => StrNumber::single_char(i32::from(c)),
            // WEB looks up the string after the single characters, as `null_cs - single_base`.
            CsKind::Null => StrNumber::from_raw(TOO_BIG_CHAR),
            _ => self.text(cs),
        };
        self.prim_lookup(strings, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::units::PoolPos;

    fn tables() -> (EqtbState, StringsState) {
        let eqtb = EqtbState {
            prim_used: PrimNum::LAST,
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
        assert_eq!(first, Some(PrimNum::FIRST.plus(97)));
        assert_eq!(second, PrimNum::LAST.prev());
        assert_eq!(eqtb.prim_next(first.expect("slot")), second);
        assert_eq!(eqtb.prim_text(second.expect("slot")), Some(b));
        assert_eq!(eqtb.prim_lookup(&mut strings, b).expect("lookup"), second);
        assert_eq!(eqtb.prim_lookup(&mut strings, a).expect("lookup"), first);

        eqtb.no_new_control_sequence = true;
        let c = StrNumber::from_raw(97 + 2 * PRIM_PRIME);
        assert_eq!(eqtb.prim_lookup(&mut strings, c).expect("lookup"), None);
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
        assert_eq!(eqtb.prim_lookup(&mut strings, ba).expect("lookup"), None);
    }
}
