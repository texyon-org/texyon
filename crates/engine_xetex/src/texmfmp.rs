//! String-pool conversions, clock queries and the file-info primitives.

use std::ffi::CString;

use crate::bridge::{
    ttstub_get_file_md5, ttstub_input_get_mtime, ttstub_input_get_size, ttstub_input_open,
    ttstub_input_read,
};
use crate::consts::{EMPTY_STRING, INVALID_HANDLE, TOO_BIG_CHAR};
use crate::mem::xmalloc;
use crate::state::StringsState;
use crate::utf8::{BYTES_FROM_UTF8, FIRST_BYTE_MARK, OFFSETS_FROM_UTF8};

const TTBC_FILE_FORMAT_TEX: libc::c_int = 26;
const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

/// Returns `(minutes, day, month, year)` for `source_date_epoch`.
pub fn get_date_and_time(source_date_epoch: libc::time_t, use_utc: bool) -> (i32, i32, i32, i32) {
    // SAFETY: gmtime/localtime return a pointer valid until the next call.
    let tm = unsafe {
        *if use_utc {
            crate::compat::gmtime(&source_date_epoch)
        } else {
            crate::compat::localtime(&source_date_epoch)
        }
    };
    (
        tm.tm_hour * 60 + tm.tm_min,
        tm.tm_mday,
        tm.tm_mon + 1,
        tm.tm_year + 1900,
    )
}

/// Time since the recorded epoch in units of 1/65536 s, or -1 past 0x7FFF s.
pub fn get_microinterval(shipout: &crate::state::ShipoutState) -> i32 {
    let (s, m) = get_seconds_and_micros();
    if (s - shipout.epochseconds) > 0x7FFF {
        -1
    } else if shipout.microseconds > m {
        let t = (s - 1 - shipout.epochseconds) as f64 * 65536.0
            + ((m as f64 + 1_000_000.0 - shipout.microseconds as f64) / 100.0) * 65536.0 / 10_000.0;
        t as i32
    } else {
        let t = (s - shipout.epochseconds) as f64 * 65536.0
            + ((m - shipout.microseconds) as f64 / 100.0) * 65536.0 / 10_000.0;
        t as i32
    }
}

pub fn get_seconds_and_micros() -> (i32, i32) {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    (elapsed.as_secs() as i32, elapsed.subsec_micros() as i32)
}

/// Encodes pool string `s` as UTF-8. Single-character strings (`s` below
/// 65536) come out empty, and unpaired low surrogates are kept as 3-byte
/// sequences.
pub(crate) fn texstring_utf8(strings: &StringsState, s: i32) -> Vec<u8> {
    if s < TOO_BIG_CHAR {
        return Vec::new();
    }
    let base = strings.str_start[(s - TOO_BIG_CHAR) as usize];
    let len = strings.str_start[(s + 1 - TOO_BIG_CHAR) as usize] - base;
    let unit = |i: i32| strings.str_pool[(base + i) as usize] as u32;

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
pub(crate) fn texstring_cstring(strings: &StringsState, s: i32) -> CString {
    let mut bytes = texstring_utf8(strings, s);
    if let Some(nul) = bytes.iter().position(|&b| b == 0) {
        bytes.truncate(nul);
    }
    // SAFETY: the bytes were cut at the first NUL.
    unsafe { CString::from_vec_unchecked(bytes) }
}

/// Pool string `s` as a `malloc`ed NUL-terminated UTF-8 string the caller frees.
pub unsafe fn gettexstring_in(strings: &StringsState, s: i32) -> *mut libc::c_char {
    let bytes = texstring_utf8(strings, s);
    let name = xmalloc(bytes.len() + 1) as *mut u8;
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), name, bytes.len());
    *name.add(bytes.len()) = 0;
    name as *mut libc::c_char
}

/// Decodes one UTF-8 sequence from a NUL-terminated string without
/// validation, advancing `cp`. A NUL cuts the sequence short.
pub(crate) unsafe fn next_utf8_char(cp: &mut *const u8) -> u32 {
    let mut rval = **cp as u32;
    *cp = cp.add(1);
    let extra_bytes = BYTES_FROM_UTF8[rval as usize] as usize;
    for _ in 0..extra_bytes {
        rval <<= 6;
        if **cp != 0 {
            rval += **cp as u32;
            *cp = cp.add(1);
        }
    }
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

/// Appends code point `c` to the pool as one or two UTF-16 units.
pub(crate) fn push_utf16(strings: &mut StringsState, c: u32) {
    let (units, n) = utf16_units(c);
    let p = strings.pool_ptr as usize;
    strings.str_pool[p..p + n].copy_from_slice(&units[..n]);
    strings.pool_ptr += n as i32;
}

fn push_bytes(strings: &mut StringsState, bytes: &[u8]) {
    let start = strings.pool_ptr as usize;
    for (slot, &b) in strings.str_pool[start..start + bytes.len()]
        .iter_mut()
        .zip(bytes)
    {
        *slot = b as u16;
    }
    strings.pool_ptr += bytes.len() as i32;
}

/// Appends `bytes` if they fit, else marks the pool as full.
fn push_bytes_or_fill(strings: &mut StringsState, bytes: &[u8]) {
    if strings.pool_ptr as usize + bytes.len() >= strings.pool_size as usize {
        strings.pool_ptr = strings.pool_size;
    } else {
        push_bytes(strings, bytes);
    }
}

fn push_hex(strings: &mut StringsState, bytes: &[u8]) {
    for &b in bytes {
        push_bytes(
            strings,
            &[
                HEX_DIGITS[(b >> 4) as usize],
                HEX_DIGITS[(b & 0xF) as usize],
            ],
        );
    }
}

pub(crate) enum MakeTexStringAbort {
    /// The pool can't hold the string.
    PoolOverflow,
    /// `str_ptr` is exhausted in `make_string`.
    StringOverflow,
}

impl MakeTexStringAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            MakeTexStringAbort::PoolOverflow => {
                crate::errors::EngineAbort::TtAbort(c"string pool overflow".as_ptr())
            }
            MakeTexStringAbort::StringOverflow => crate::errors::EngineAbort::StringOverflow,
        }
    }
}

/// Converts a NUL-terminated UTF-8 string into a new pool string.
pub(crate) unsafe fn maketexstring_in(
    strings: &mut StringsState,
    s: *const libc::c_char,
) -> Result<i32, MakeTexStringAbort> {
    if s.is_null() || *s == 0 {
        return Ok(EMPTY_STRING);
    }

    if strings.pool_ptr as usize + libc::strlen(s) >= strings.pool_size as usize {
        return Err(MakeTexStringAbort::PoolOverflow);
    }

    let mut cp = s as *const u8;
    while *cp != 0 {
        push_utf16(strings, next_utf8_char(&mut cp));
    }

    crate::string_pool::make_string(strings).map_err(|_| MakeTexStringAbort::StringOverflow)
}

/// Appends the hex MD5 of file (or string) `s` to the pool.
pub(crate) unsafe fn getmd5sum_in(strings: &mut StringsState, s: i32, file: bool) {
    let mut digest = [0u8; 16];
    let xname = texstring_cstring(strings, s);

    let ret = if file {
        ttstub_get_file_md5(xname.as_ptr(), digest.as_mut_ptr())
    } else {
        let bytes = xname.as_bytes();
        tectonic_bridge_core::ttbc_get_data_md5(bytes.as_ptr(), bytes.len(), digest.as_mut_ptr())
    };

    if ret != 0 || strings.pool_ptr + 2 * digest.len() as i32 >= strings.pool_size {
        return;
    }
    push_hex(strings, &digest);
}

/// Writes "src:<lineno> <filename>" into the pool and returns its start, or
/// `Err` if the pool is too small.
pub(crate) fn make_src_special_in(
    st: &mut crate::state::EngineState,
    srcfilename: i32,
    lineno: i32,
) -> Result<i32, ()> {
    let old_pool_ptr = st.strings.pool_ptr;
    let filename = texstring_cstring(&st.strings, srcfilename);
    let prefix = format!("src:{} ", lineno);
    let filename = filename.as_bytes();

    if st.strings.pool_ptr as usize + prefix.len() + filename.len() >= st.strings.pool_size as usize
    {
        return Err(());
    }
    push_bytes(&mut st.strings, prefix.as_bytes());
    push_bytes(&mut st.strings, filename);
    Ok(old_pool_ptr)
}

unsafe fn open_tex_file(strings: &StringsState, s: i32) -> usize {
    let name = texstring_cstring(strings, s);
    ttstub_input_open(name.as_ptr(), TTBC_FILE_FORMAT_TEX, 0)
}

/// Appends the decimal size of file `s` to the pool.
pub(crate) unsafe fn getfilesize_in(
    strings: &mut StringsState,
    s: i32,
) -> Result<(), crate::scan::SccAbort> {
    let handle = open_tex_file(strings, s);
    if handle == INVALID_HANDLE {
        return Ok(());
    }

    let file_len = ttstub_input_get_size(handle);
    if crate::io::input_close(handle) {
        return Err(crate::scan::SccAbort::InputClose);
    }

    push_bytes_or_fill(strings, (file_len as u64).to_string().as_bytes());
    Ok(())
}

/// Appends a hex dump of `length` bytes of file `s` from `offset` to the pool.
pub(crate) unsafe fn getfiledump_in(
    strings: &mut StringsState,
    s: i32,
    offset: libc::c_int,
    length: libc::c_int,
) -> Result<(), crate::scan::SccAbort> {
    if length == 0 {
        return Ok(());
    }

    if strings.pool_ptr + 2 * length + 1 >= strings.pool_size {
        strings.pool_ptr = strings.pool_size;
        return Ok(());
    }

    let mut buffer = vec![0u8; length as usize + 1];
    let handle = open_tex_file(strings, s);
    if handle == INVALID_HANDLE {
        return Ok(());
    }

    crate::io::input_seek(handle, offset as libc::ssize_t, libc::SEEK_SET);
    let actual = ttstub_input_read(handle, buffer.as_mut_ptr(), length as libc::size_t);
    if crate::io::input_close(handle) {
        return Err(crate::scan::SccAbort::InputClose);
    }

    push_hex(strings, &buffer[..actual.max(0) as usize]);
    Ok(())
}

/// Whether `srcfilename:lineno` differs from the last remembered source.
pub(crate) fn is_new_source_in(
    st: &crate::state::EngineState,
    srcfilename: i32,
    lineno: i32,
) -> bool {
    let name = texstring_cstring(&st.strings, srcfilename);
    let last_name = st
        .texmf
        .last_source_name
        .as_deref()
        .map_or(&[][..], |name| name.to_bytes());
    name.as_bytes() != last_name || lineno != st.texmf.last_lineno
}

pub(crate) fn remember_source_info_in(
    st: &mut crate::state::EngineState,
    srcfilename: i32,
    lineno: i32,
) {
    st.texmf.last_source_name = Some(texstring_cstring(&st.strings, srcfilename));
    st.texmf.last_lineno = lineno;
}

/// Formats `t` as a NUL-terminated PDF date string into `time_str`.
unsafe fn makepdftime(t: libc::time_t, time_str: &mut [u8; 30], utc: bool) {
    let lt = if utc {
        *crate::compat::gmtime(&t)
    } else {
        *crate::compat::localtime(&t)
    };
    let size = crate::compat::strftime(
        time_str.as_mut_ptr() as *mut libc::c_char,
        30,
        c"D:%Y%m%d%H%M%S".as_ptr(),
        &lt as *const libc::tm,
    );
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

    let gmt = *crate::compat::gmtime(&t);
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
pub unsafe fn init_start_time(st: &mut crate::state::EngineState, source_date_epoch: libc::time_t) {
    let mut buf = [0u8; 30];
    makepdftime(source_date_epoch, &mut buf, true);
    st.texmf.start_time_str = buf;
}

pub(crate) fn getcreationdate_in(texmf: &crate::state::TexmfState, strings: &mut StringsState) {
    push_bytes_or_fill(strings, nul_terminated(&texmf.start_time_str));
}

/// Appends the modification date of file `s` to the pool as a PDF date.
pub(crate) unsafe fn getfilemoddate_in(
    strings: &mut StringsState,
    s: i32,
) -> Result<(), crate::scan::SccAbort> {
    let handle = open_tex_file(strings, s);
    if handle == INVALID_HANDLE {
        return Ok(());
    }

    let mtime = ttstub_input_get_mtime(handle);
    if crate::io::input_close(handle) {
        return Err(crate::scan::SccAbort::InputClose);
    }

    let mut buf = [0u8; 30];
    makepdftime(mtime, &mut buf, true);
    push_bytes_or_fill(strings, nul_terminated(&buf));
    Ok(())
}
