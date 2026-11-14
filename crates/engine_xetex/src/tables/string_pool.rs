//! String pool: TeX strings stored as UTF-16 code units in `str_pool`.
//! Strings below `TOO_BIG_CHAR` are single characters stored implicitly.

use crate::base::consts::{EMPTY_STRING, TOO_BIG_CHAR};
use crate::base::units::{PoolPos, StrNumber};
use crate::print::errors::Abort;
use crate::state::StringsState;

impl StringsState {
    /// Where pool string `s` starts in `str_pool`.
    pub(crate) fn str_start(&self, s: StrNumber) -> PoolPos {
        self.str_start[s.slot()]
    }

    /// The `str_pool` range of string `s`.
    pub(crate) fn str_range(&self, s: StrNumber) -> std::ops::Range<usize> {
        self.str_start(s).index()..self.str_start(s.next()).index()
    }

    /// The character of `s` when it is one surrogate pair, else `TOO_BIG_USV`.
    pub(crate) fn astral_char(&self, s: StrNumber) -> i32 {
        if !s.is_pool_string() || self.length(s) != 2 {
            return crate::base::consts::TOO_BIG_USV;
        }
        let start = self.str_start(s).index();
        let (hi, lo) = (self.str_pool[start], self.str_pool[start + 1]);
        if (0xD800..=0xDBFF).contains(&hi) && (0xDC00..=0xDFFF).contains(&lo) {
            0x10000 + (i32::from(hi) - 0xD800) * 0x400 + i32::from(lo) - 0xDC00
        } else {
            crate::base::consts::TOO_BIG_USV
        }
    }

    pub fn length(&self, s: StrNumber) -> i32 {
        if s.is_pool_string() {
            return self.str_start(s.next()) - self.str_start(s);
        }
        match s.raw() {
            32..=126 => 1,
            ..=127 => 3,
            128..=255 => 4,
            _ => 8,
        }
    }

    pub fn make_string(&mut self) -> Result<StrNumber, Abort> {
        if self.str_ptr == self.max_strings {
            return Err(self.string_overflow());
        }
        self.str_ptr = self.str_ptr.next();
        let slot = self.cur_start_slot();
        self.str_start[slot] = self.pool_ptr;
        Ok(self.str_ptr.prev())
    }

    pub fn append_str(&mut self, s: StrNumber) -> Result<(), Abort> {
        let len = self.length(s);
        if self.pool_ptr + len > self.pool_size {
            return Err(self.pool_overflow());
        }
        let src = self.str_start(s).index();
        let dst = self.pool_ptr.index();
        for i in 0..len.max(0) as usize {
            self.str_pool[dst + i] = self.str_pool[src + i];
        }
        self.pool_ptr += len.max(0);
        Ok(())
    }

    /// Whether pool string `s` spells the characters `name`, after UTF-16 encoding.
    pub fn str_eq_buf(&self, name: &[i32], s: StrNumber) -> bool {
        let mut j = self.str_start(s);
        let end = self.str_start(s.next());
        let mut k = 0;
        while j < end {
            let Some(&c) = name.get(k) else {
                return false;
            };
            if c >= TOO_BIG_CHAR {
                let hi = 0xD800 + (c - TOO_BIG_CHAR) / 1024;
                let lo = 0xDC00 + (c - TOO_BIG_CHAR) % 1024;
                if i32::from(self.str_pool[j.index()]) != hi
                    || i32::from(self.str_pool[(j + 1).index()]) != lo
                {
                    return false;
                }
                j += 1;
            } else if i32::from(self.str_pool[j.index()]) != c {
                return false;
            }
            j += 1;
            k += 1;
        }
        true
    }

    pub fn str_eq_str(&self, s: StrNumber, t: StrNumber) -> bool {
        let len = self.length(s);
        if len != self.length(t) {
            return false;
        }
        if len == 1 {
            let first_unit = |x: StrNumber| {
                if x.is_pool_string() {
                    self.str_pool[self.str_start(x).index()]
                } else {
                    x.raw() as u16
                }
            };
            return first_unit(s) == first_unit(t);
        }
        let n = self.str_range(s).len();
        let k = self.str_start(t).index();
        self.str_pool[self.str_range(s)] == self.str_pool[k..k + n]
    }

    /// An existing string equal to `search`, or 0 if none.
    pub fn search_string(&self, search: StrNumber) -> StrNumber {
        let len = self.length(search);
        if len == 0 {
            return EMPTY_STRING;
        }
        (TOO_BIG_CHAR..search.raw())
            .rev()
            .map(StrNumber::from_raw)
            .find(|&s| self.length(s) == len && self.str_eq_str(s, search))
            .unwrap_or_default()
    }

    /// `make_string` that reuses an existing equal string instead.
    pub fn slow_make_string(&mut self) -> Result<StrNumber, Abort> {
        let t = self.make_string()?;
        let s = self.search_string(t);
        if s.raw() > 0 {
            self.flush_str(t);
            return Ok(s);
        }
        Ok(t)
    }

    /// Returns the last loaded string number, or 0 if the pool would overflow.
    pub fn load_pool_strings(&mut self, spare_size: i32) -> Result<StrNumber, Abort> {
        const STRING_CONSTANTS: &[&[u8]] = &[b"this marks the start of the stringpool", b""];

        let mut total_len = 0;
        let mut g = StrNumber::from_raw(0);
        for s in STRING_CONSTANTS {
            total_len += s.len();
            if total_len >= spare_size as usize {
                return Ok(StrNumber::from_raw(0));
            }
            for &byte in *s {
                self.append_char(u16::from(byte));
            }
            g = self.make_string()?;
        }
        Ok(g)
    }

    /// Frees `s` if it is the most recently made string.
    pub fn flush_str(&mut self, s: StrNumber) {
        if s == self.str_ptr.prev() {
            self.flush_string();
        }
    }

    /// TeX's `flush_string`: frees the most recently made string.
    pub fn flush_string(&mut self) {
        self.str_ptr = self.str_ptr.prev();
        self.pool_ptr = self.str_start(self.str_ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool() -> StringsState {
        StringsState {
            str_ptr: StrNumber::from_raw(TOO_BIG_CHAR),
            max_strings: StrNumber::from_raw(TOO_BIG_CHAR + 8),
            pool_size: PoolPos::from_raw(32),
            str_start: vec![PoolPos::ZERO; 9],
            str_pool: vec![0; 33],
            ..StringsState::default()
        }
    }

    fn make(strings: &mut StringsState, text: &str) -> StrNumber {
        strings.append_units(&text.encode_utf16().collect::<Vec<_>>());
        strings.make_string().expect("room for the string")
    }

    #[test]
    fn str_eq_str_compares_contents() {
        let mut strings = pool();
        let ab = make(&mut strings, "ab");
        let ac = make(&mut strings, "ac");
        let ab2 = make(&mut strings, "ab");
        assert!(strings.str_eq_str(ab, ab2));
        assert!(!strings.str_eq_str(ab, ac));
        assert_eq!(strings.search_string(ab2), ab);
    }

    #[test]
    fn flush_str_frees_only_the_latest_string() {
        let mut strings = pool();
        let first = make(&mut strings, "abc");
        let second = make(&mut strings, "de");
        strings.flush_str(first);
        assert_eq!(
            (strings.str_ptr, strings.pool_ptr),
            (second.next(), PoolPos::from_raw(5))
        );
        strings.flush_str(second);
        assert_eq!(
            (strings.str_ptr, strings.pool_ptr),
            (second, PoolPos::from_raw(3))
        );
    }
}
