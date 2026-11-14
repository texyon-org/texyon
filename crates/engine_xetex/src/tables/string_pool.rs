//! String pool: TeX strings stored as UTF-16 code units in `str_pool`.
//! Strings below `TOO_BIG_CHAR` are single characters stored implicitly.

use std::ops::Range;

use crate::base::consts::{EMPTY_STRING, TOO_BIG_CHAR, TOO_BIG_USV};
use crate::base::units::{PoolPos, StrNumber};
use crate::base::utf8::{FIRST_BYTE_MARK, next_utf8_char, utf16_units};
use crate::print::errors::{Abort, Capacity};

impl StringsState {
    pub(crate) fn str_start(&self, s: StrNumber) -> PoolPos {
        self.str_start[s.slot()]
    }

    pub(crate) fn str_range(&self, s: StrNumber) -> Range<usize> {
        self.str_start(s).index()..self.str_start(s.next()).index()
    }

    /// The character of `s` when it is one surrogate pair, else `TOO_BIG_USV`.
    pub(crate) fn astral_char(&self, s: StrNumber) -> i32 {
        if !s.is_pool_string() || self.length(s) != 2 {
            return TOO_BIG_USV;
        }
        let start = self.str_start(s).index();
        let (hi, lo) = (self.str_pool[start], self.str_pool[start + 1]);
        if (0xD800..=0xDBFF).contains(&hi) && (0xDC00..=0xDFFF).contains(&lo) {
            0x10000 + (i32::from(hi) - 0xD800) * 0x400 + i32::from(lo) - 0xDC00
        } else {
            TOO_BIG_USV
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

    pub fn flush_string(&mut self) {
        self.str_ptr = self.str_ptr.prev();
        self.pool_ptr = self.str_start(self.str_ptr);
    }
}

#[derive(Default)]
pub(crate) struct StringsState {
    /// The string under construction, one past the last made string.
    pub str_ptr: StrNumber,
    pub max_strings: StrNumber,
    pub init_str_ptr: StrNumber,
    pub init_pool_ptr: PoolPos,
    pub job_name: StrNumber,
    pub texmf_log_name: StrNumber,
    pub string_vacancies: i32,
    pub pool_size: PoolPos,
    pub pool_ptr: PoolPos,
    pub format_ident: StrNumber,
    pub str_start: Vec<PoolPos>,
    pub str_pool: Vec<u16>,
}

impl StringsState {
    pub(crate) fn pool_overflow(&self) -> Abort {
        Abort::Overflow {
            capacity: Capacity::PoolSize,
            n: self.pool_size - self.init_pool_ptr,
        }
    }

    pub(crate) fn string_overflow(&self) -> Abort {
        Abort::Overflow {
            capacity: Capacity::NumberOfStrings,
            n: self.max_strings - self.init_str_ptr,
        }
    }

    #[inline]
    pub(crate) fn cur_start_slot(&self) -> usize {
        if self.str_ptr.is_pool_string() {
            self.str_ptr.slot()
        } else {
            0
        }
    }

    #[inline]
    pub(crate) fn cur_string_start(&self) -> PoolPos {
        self.str_start[self.str_ptr.slot()]
    }

    #[inline]
    pub(crate) fn append_char(&mut self, c: u16) {
        self.str_pool[self.pool_ptr.index()] = c;
        self.pool_ptr += 1;
    }

    #[inline]
    pub(crate) fn append_units(&mut self, units: &[u16]) {
        let start = self.pool_ptr.index();
        self.str_pool[start..start + units.len()].copy_from_slice(units);
        self.pool_ptr += units.len() as i32;
    }

    #[inline]
    pub(crate) fn cur_string(&self) -> &[u16] {
        &self.str_pool[self.cur_string_start().index()..self.pool_ptr.index()]
    }

    #[inline]
    pub(crate) fn discard_cur_string(&mut self) {
        self.pool_ptr = self.cur_string_start();
    }
}

impl StringsState {
    /// Encodes pool string `s` as UTF-8. Single-character strings (`s` below
    /// 65536) come out empty, and unpaired low surrogates are kept as 3-byte
    /// sequences.
    pub(crate) fn texstring_utf8(&self, s: StrNumber) -> Vec<u8> {
        if !s.is_pool_string() {
            return Vec::new();
        }
        let base = self.str_start[s.slot()];
        let len = self.str_start[s.next().slot()] - base;
        let unit = |i: i32| u32::from(self.str_pool[(base + i).index()]);

        let mut out = Vec::with_capacity(len as usize * 3);
        let mut i = 0;
        while i < len {
            let mut c = unit(i);
            if (0xD800..=0xDBFF).contains(&c) {
                i += 1;
                let lo = unit(i);
                c = if (0xDC00..=0xDFFF).contains(&lo) {
                    (c - 0xD800) * 0x0400 + lo - 0xDC00 + 0x10000
                } else {
                    0xFFFD
                };
            }

            let n = match c {
                0..=0x7F => 1,
                0x80..=0x7FF => 2,
                0x800..=0xFFFF => 3,
                0x10000..=0x10FFFF => 4,
                _ => {
                    c = 0xFFFD;
                    3
                }
            };
            out.push(((c >> (6 * (n - 1))) | u32::from(FIRST_BYTE_MARK[n])) as u8);
            for k in (0..n - 1).rev() {
                out.push(0x80 | ((c >> (6 * k)) & 0x3F) as u8);
            }

            i += 1;
        }
        out
    }

    /// Pool string `s` in UTF-8, cut at its first NUL as C saw it.
    pub(crate) fn texstring_bytes(&self, s: StrNumber) -> Vec<u8> {
        let mut bytes = self.texstring_utf8(s);
        if let Some(nul) = bytes.iter().position(|&b| b == 0) {
            bytes.truncate(nul);
        }
        bytes
    }

    pub(crate) fn push_utf16(&mut self, c: u32) {
        let (units, n) = utf16_units(c);
        self.append_units(&units[..n]);
    }

    pub(crate) fn push_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.append_char(u16::from(b));
        }
    }

    /// Appends `bytes` if they fit, else marks the pool as full.
    pub(crate) fn push_bytes_or_fill(&mut self, bytes: &[u8]) {
        if self.pool_ptr.index() + bytes.len() >= self.pool_size.index() {
            self.pool_ptr = self.pool_size;
        } else {
            self.push_bytes(bytes);
        }
    }

    pub(crate) fn make_tex_string(&mut self, s: impl AsRef<[u8]>) -> Result<StrNumber, Abort> {
        let mut s = s.as_ref();
        if s.is_empty() {
            return Ok(EMPTY_STRING);
        }

        if self.pool_ptr.index() + s.len() >= self.pool_size.index() {
            return Err(Abort::Internal("string pool overflow"));
        }

        while !s.is_empty() {
            self.push_utf16(next_utf8_char(&mut s));
        }

        self.make_string()
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
