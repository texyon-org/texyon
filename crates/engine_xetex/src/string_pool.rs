//! String pool: TeX strings stored as UTF-16 code units in `str_pool`.
//! Strings below `TOO_BIG_CHAR` are single characters stored implicitly.

use crate::consts::{EMPTY_STRING, TOO_BIG_CHAR};
use crate::errors::Abort;
use crate::state::StringsState;
use crate::units::StrNumber;

impl StringsState {
    fn str_start(&self, s: StrNumber) -> i32 {
        self.str_start[s.slot()]
    }

    /// The `str_pool` range of string `s`.
    pub(crate) fn str_range(&self, s: StrNumber) -> std::ops::Range<usize> {
        self.str_start(s) as usize..self.str_start(s.next()) as usize
    }

    pub fn length(&self, s: StrNumber) -> i32 {
        if s.is_pool_string() {
            return self.str_start(s.next()) - self.str_start(s);
        }
        match s.0 {
            32..=126 => 1,
            ..=127 => 3,
            128..=255 => 4,
            _ => 8,
        }
    }

    pub fn make_string(&mut self) -> Result<StrNumber, Abort> {
        if self.str_ptr == self.max_strings {
            return Err(Abort::StringOverflow);
        }
        self.str_ptr += 1;
        let slot = self.cur_start_slot();
        self.str_start[slot] = self.pool_ptr;
        Ok(StrNumber(self.str_ptr - 1))
    }

    pub fn append_str(&mut self, s: StrNumber) -> Result<(), Abort> {
        let len = self.length(s);
        if self.pool_ptr + len > self.pool_size {
            return Err(Abort::PoolOverflow);
        }
        let src = self.str_start(s) as usize;
        let dst = self.pool_ptr as usize;
        for i in 0..len.max(0) as usize {
            self.str_pool[dst + i] = self.str_pool[src + i];
        }
        self.pool_ptr += len.max(0);
        Ok(())
    }

    /// Whether string `s` equals the buffer run starting at `k`.
    pub fn str_eq_buf(&self, buffer: &[i32], s: StrNumber, mut k: i32) -> bool {
        let mut j = self.str_start(s);
        let end = self.str_start(s.next());
        while j < end {
            let c = buffer[k as usize];
            if c >= TOO_BIG_CHAR {
                let hi = 0xD800 + (c - TOO_BIG_CHAR) / 1024;
                let lo = 0xDC00 + (c - TOO_BIG_CHAR) % 1024;
                if i32::from(self.str_pool[j as usize]) != hi
                    || i32::from(self.str_pool[(j + 1) as usize]) != lo
                {
                    return false;
                }
                j += 1;
            } else if i32::from(self.str_pool[j as usize]) != c {
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
                    self.str_pool[self.str_start(x) as usize]
                } else {
                    x.0 as u16
                }
            };
            return first_unit(s) == first_unit(t);
        }
        let j = self.str_start(s) as usize;
        let k = self.str_start(t) as usize;
        let n = (self.str_start(s.next()) as usize).saturating_sub(j);
        self.str_pool[j..j + n] == self.str_pool[k..k + n]
    }

    /// An existing string equal to `search`, or 0 if none.
    pub fn search_string(&self, search: StrNumber) -> StrNumber {
        let len = self.length(search);
        if len == 0 {
            return EMPTY_STRING;
        }
        (TOO_BIG_CHAR..search.0)
            .rev()
            .map(StrNumber)
            .find(|&s| self.length(s) == len && self.str_eq_str(s, search))
            .unwrap_or_default()
    }

    /// `make_string` that reuses an existing equal string instead.
    pub fn slow_make_string(&mut self) -> Result<StrNumber, Abort> {
        let t = self.make_string()?;
        let s = self.search_string(t);
        if s.0 > 0 {
            self.flush_str(t);
            return Ok(s);
        }
        Ok(t)
    }

    /// Returns the last loaded string number, or 0 if the pool would overflow.
    pub fn load_pool_strings(&mut self, spare_size: i32) -> Result<StrNumber, Abort> {
        const STRING_CONSTANTS: &[&[u8]] = &[b"this marks the start of the stringpool", b""];

        let mut total_len = 0;
        let mut g = StrNumber(0);
        for s in STRING_CONSTANTS {
            total_len += s.len();
            if total_len >= spare_size as usize {
                return Ok(StrNumber(0));
            }
            for &byte in *s {
                self.str_pool[self.pool_ptr as usize] = u16::from(byte);
                self.pool_ptr += 1;
            }
            g = self.make_string()?;
        }
        Ok(g)
    }

    /// Frees `s` if it is the most recently made string.
    pub fn flush_str(&mut self, s: StrNumber) {
        if s.0 == self.str_ptr - 1 {
            self.str_ptr -= 1;
            self.pool_ptr = self.str_start(StrNumber(self.str_ptr));
        }
    }
}
