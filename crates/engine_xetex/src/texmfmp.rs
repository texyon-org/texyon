//! String-pool conversions, clock queries and the file-info primitives.

use std::ffi::CString;

use crate::bridge::{
    data_md5, get_file_md5, input_get_mtime, input_get_size, input_open, input_read,
    TTBC_FILE_FORMAT_TEX,
};
use crate::consts::{EMPTY_STRING, INVALID_HANDLE, TOO_BIG_CHAR};
use crate::errors::Abort;
use crate::state::{ShipoutState, StringsState, TexmfState};
use crate::utf8::{BYTES_FROM_UTF8, FIRST_BYTE_MARK, OFFSETS_FROM_UTF8};

const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

/// `gmtime` or `localtime` of `t`.
fn broken_down_time(t: libc::time_t, utc: bool) -> libc::tm {
    // SAFETY: both return null or a pointer to a static `tm` that stays valid
    // until the next call, and it is copied out right away.
    let tm = unsafe {
        if utc {
            crate::compat::gmtime(&t)
        } else {
            crate::compat::localtime(&t)
        }
        .as_ref()
        .copied()
    };
    tm.expect("time out of range")
}

/// Returns `(minutes, day, month, year)` for `source_date_epoch`.
pub fn get_date_and_time(source_date_epoch: libc::time_t, use_utc: bool) -> (i32, i32, i32, i32) {
    let tm = broken_down_time(source_date_epoch, use_utc);
    (
        tm.tm_hour * 60 + tm.tm_min,
        tm.tm_mday,
        tm.tm_mon + 1,
        tm.tm_year + 1900,
    )
}

impl ShipoutState {
    /// Time since the recorded epoch in units of 1/65536 s, or -1 past 0x7FFF s.
    pub fn get_microinterval(&self) -> i32 {
        let (s, m) = get_seconds_and_micros();
        if (s - self.epochseconds) > 0x7FFF {
            -1
        } else if self.microseconds > m {
            let t = (s - 1 - self.epochseconds) as f64 * 65536.0
                + ((m as f64 + 1_000_000.0 - self.microseconds as f64) / 100.0) * 65536.0
                    / 10_000.0;
            t as i32
        } else {
            let t = (s - self.epochseconds) as f64 * 65536.0
                + ((m - self.microseconds) as f64 / 100.0) * 65536.0 / 10_000.0;
            t as i32
        }
    }
}

pub fn get_seconds_and_micros() -> (i32, i32) {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    (elapsed.as_secs() as i32, elapsed.subsec_micros() as i32)
}

/// Decodes one UTF-8 sequence from the non-empty `s` without validation,
/// advancing it. The end of `s` cuts the sequence short.
pub(crate) fn next_utf8_char(s: &mut &[u8]) -> u32 {
    let (&first, mut rest) = s.split_first().expect("next_utf8_char on empty input");
    let mut rval = first as u32;
    let extra_bytes = BYTES_FROM_UTF8[first as usize] as usize;
    for _ in 0..extra_bytes {
        rval <<= 6;
        if let Some((&b, tail)) = rest.split_first() {
            rval += b as u32;
            rest = tail;
        }
    }
    *s = rest;
    rval - OFFSETS_FROM_UTF8[extra_bytes]
}

/// The UTF-16 encoding of `c` and its length in units.
pub(crate) fn utf16_units(c: u32) -> ([u16; 2], usize) {
    if c > 0xFFFF {
        let c = c - 0x10000;
        (
            [(0xD800 + c / 0x0400) as u16, (0xDC00 + c % 0x0400) as u16],
            2,
        )
    } else {
        ([c as u16, 0], 1)
    }
}

impl StringsState {
    /// Encodes pool string `s` as UTF-8. Single-character strings (`s` below
    /// 65536) come out empty, and unpaired low surrogates are kept as 3-byte
    /// sequences.
    pub(crate) fn texstring_utf8(&self, s: i32) -> Vec<u8> {
        if s < TOO_BIG_CHAR {
            return Vec::new();
        }
        let base = self.str_start[(s - TOO_BIG_CHAR) as usize];
        let len = self.str_start[(s + 1 - TOO_BIG_CHAR) as usize] - base;
        let unit = |i: i32| self.str_pool[(base + i) as usize] as u32;

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
            out.push(((c >> (6 * (n - 1))) | FIRST_BYTE_MARK[n] as u32) as u8);
            for k in (0..n - 1).rev() {
                out.push(0x80 | ((c >> (6 * k)) & 0x3F) as u8);
            }

            i += 1;
        }
        out
    }

    /// Pool string `s` as a C string, cut at its first NUL.
    pub(crate) fn texstring_cstring(&self, s: i32) -> CString {
        let mut bytes = self.texstring_utf8(s);
        if let Some(nul) = bytes.iter().position(|&b| b == 0) {
            bytes.truncate(nul);
        }
        // SAFETY: the bytes were cut at the first NUL.
        unsafe { CString::from_vec_unchecked(bytes) }
    }

    /// Appends code point `c` to the pool as one or two UTF-16 units.
    pub(crate) fn push_utf16(&mut self, c: u32) {
        let (units, n) = utf16_units(c);
        let p = self.pool_ptr as usize;
        self.str_pool[p..p + n].copy_from_slice(&units[..n]);
        self.pool_ptr += n as i32;
    }

    fn push_bytes(&mut self, bytes: &[u8]) {
        let start = self.pool_ptr as usize;
        for (slot, &b) in self.str_pool[start..start + bytes.len()]
            .iter_mut()
            .zip(bytes)
        {
            *slot = b as u16;
        }
        self.pool_ptr += bytes.len() as i32;
    }

    /// Appends `bytes` if they fit, else marks the pool as full.
    fn push_bytes_or_fill(&mut self, bytes: &[u8]) {
        if self.pool_ptr as usize + bytes.len() >= self.pool_size as usize {
            self.pool_ptr = self.pool_size;
        } else {
            self.push_bytes(bytes);
        }
    }

    fn push_hex(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.push_bytes(&[
                HEX_DIGITS[(b >> 4) as usize],
                HEX_DIGITS[(b & 0xF) as usize],
            ]);
        }
    }

    /// Converts UTF-8 bytes into a new pool string.
    pub(crate) fn make_tex_string(&mut self, s: impl AsRef<[u8]>) -> Result<i32, Abort> {
        let mut s = s.as_ref();
        if s.is_empty() {
            return Ok(EMPTY_STRING);
        }

        if self.pool_ptr as usize + s.len() >= self.pool_size as usize {
            return Err(Abort::Internal("string pool overflow"));
        }

        while !s.is_empty() {
            self.push_utf16(next_utf8_char(&mut s));
        }

        self.make_string()
    }

    /// Appends the hex MD5 of file (or string) `s` to the pool.
    pub(crate) fn getmd5sum(&mut self, s: i32, file: bool) {
        let mut digest = [0u8; 16];
        let xname = self.texstring_cstring(s);

        let ret = if file {
            get_file_md5(&xname, &mut digest)
        } else {
            data_md5(xname.as_bytes(), &mut digest)
        };

        if ret != 0 || self.pool_ptr + 2 * digest.len() as i32 >= self.pool_size {
            return;
        }
        self.push_hex(&digest);
    }
}

/// Writes "src:<lineno> <filename>" into the pool and returns its start, or
/// `Err` if the pool is too small.
pub(crate) fn make_src_special(
    st: &mut crate::state::EngineState,
    srcfilename: i32,
    lineno: i32,
) -> Result<i32, ()> {
    let old_pool_ptr = st.strings.pool_ptr;
    let filename = st.strings.texstring_cstring(srcfilename);
    let prefix = format!("src:{} ", lineno);
    let filename = filename.as_bytes();

    if st.strings.pool_ptr as usize + prefix.len() + filename.len() >= st.strings.pool_size as usize
    {
        return Err(());
    }
    st.strings.push_bytes(prefix.as_bytes());
    st.strings.push_bytes(filename);
    Ok(old_pool_ptr)
}

impl StringsState {
    fn open_tex_file(&self, s: i32) -> usize {
        let name = self.texstring_cstring(s);
        input_open(name.as_bytes(), TTBC_FILE_FORMAT_TEX, false)
    }

    /// Appends the decimal size of file `s` to the pool.
    pub(crate) fn getfilesize(&mut self, s: i32) -> Result<(), Abort> {
        let handle = self.open_tex_file(s);
        if handle == INVALID_HANDLE {
            return Ok(());
        }

        let file_len = input_get_size(handle);
        if crate::io::input_close(handle) {
            return Err(Abort::InputClose);
        }

        self.push_bytes_or_fill((file_len as u64).to_string().as_bytes());
        Ok(())
    }

    /// Appends a hex dump of `length` bytes of file `s` from `offset` to the pool.
    pub(crate) fn getfiledump(
        &mut self,
        s: i32,
        offset: libc::c_int,
        length: libc::c_int,
    ) -> Result<(), Abort> {
        if length == 0 {
            return Ok(());
        }

        if self.pool_ptr + 2 * length + 1 >= self.pool_size {
            self.pool_ptr = self.pool_size;
            return Ok(());
        }

        let mut buffer = vec![0u8; length as usize + 1];
        let handle = self.open_tex_file(s);
        if handle == INVALID_HANDLE {
            return Ok(());
        }

        crate::io::input_seek(handle, offset as libc::ssize_t, libc::SEEK_SET);
        let actual = input_read(handle, &mut buffer[..length as usize]);
        if crate::io::input_close(handle) {
            return Err(Abort::InputClose);
        }

        self.push_hex(&buffer[..actual.max(0) as usize]);
        Ok(())
    }
}

/// Whether `srcfilename:lineno` differs from the last remembered source.
pub(crate) fn is_new_source(st: &crate::state::EngineState, srcfilename: i32, lineno: i32) -> bool {
    let name = st.strings.texstring_cstring(srcfilename);
    let last_name = st
        .texmf
        .last_source_name
        .as_deref()
        .map_or(&[][..], |name| name.to_bytes());
    name.as_bytes() != last_name || lineno != st.texmf.last_lineno
}

pub(crate) fn remember_source_info(
    st: &mut crate::state::EngineState,
    srcfilename: i32,
    lineno: i32,
) {
    st.texmf.last_source_name = Some(st.strings.texstring_cstring(srcfilename));
    st.texmf.last_lineno = lineno;
}

/// Formats `t` as a NUL-terminated PDF date string into `time_str`.
fn makepdftime(t: libc::time_t, time_str: &mut [u8; 30], utc: bool) {
    let lt = broken_down_time(t, utc);
    // SAFETY: `time_str` holds the 30 bytes passed, the format is a C string
    // and `lt` is a valid `tm`.
    let size = unsafe {
        crate::compat::strftime(
            time_str.as_mut_ptr().cast(),
            30,
            c"D:%Y%m%d%H%M%S".as_ptr(),
            &lt,
        )
    };
    if size == 0 {
        time_str[0] = 0;
        return;
    }

    // %S can be 60 or 61 (leap seconds) but PDF expects 00..59.
    if time_str[14] == b'6' {
        time_str[14] = b'5';
        time_str[15] = b'9';
        time_str[16] = 0;
    }

    let gmt = broken_down_time(t, true);
    let mut off = 60 * (lt.tm_hour - gmt.tm_hour) + lt.tm_min - gmt.tm_min;
    if lt.tm_year != gmt.tm_year {
        off += if lt.tm_year > gmt.tm_year {
            1440
        } else {
            -1440
        };
    } else if lt.tm_yday != gmt.tm_yday {
        off += if lt.tm_yday > gmt.tm_yday {
            1440
        } else {
            -1440
        };
    }

    if off == 0 {
        time_str[size] = b'Z';
        time_str[size + 1] = 0;
    } else {
        let off_hours = off / 60;
        let off_mins = (off - off_hours * 60).abs();
        let tz = format!("{:+03}'{:02}'", off_hours, off_mins);
        let tz = &tz.as_bytes()[..tz.len().min(8)];
        time_str[size..size + tz.len()].copy_from_slice(tz);
        time_str[size + tz.len()] = 0;
    }
}

fn nul_terminated(buf: &[u8]) -> &[u8] {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    &buf[..end]
}

/// Records the PDF date string of `source_date_epoch` for `getcreationdate`.
pub fn init_start_time(st: &mut crate::state::EngineState, source_date_epoch: libc::time_t) {
    let mut buf = [0u8; 30];
    makepdftime(source_date_epoch, &mut buf, true);
    st.texmf.start_time_str = buf;
}

impl TexmfState {
    pub(crate) fn getcreationdate(&self, strings: &mut StringsState) {
        strings.push_bytes_or_fill(nul_terminated(&self.start_time_str));
    }
}

impl StringsState {
    /// Appends the modification date of file `s` to the pool as a PDF date.
    pub(crate) fn getfilemoddate(&mut self, s: i32) -> Result<(), Abort> {
        let handle = self.open_tex_file(s);
        if handle == INVALID_HANDLE {
            return Ok(());
        }

        let mtime = input_get_mtime(handle);
        if crate::io::input_close(handle) {
            return Err(Abort::InputClose);
        }

        let mut buf = [0u8; 30];
        makepdftime(mtime, &mut buf, true);
        self.push_bytes_or_fill(nul_terminated(&buf));
        Ok(())
    }
}
