//! Miscellaneous C-callable helpers from xetex-texmfmp.c.
//!
//! Functions involving `makepdftime` / the C-static state (`last_source_name`,
//! `start_time_str`) or complex bridge calls remain in C for now.

use crate::globals::{pool_ptr, pool_size, set_pool_ptr, set_str_pool, str_pool_entry, str_start};

// ---------------------------------------------------------------------------
// C externs needed by this module
// ---------------------------------------------------------------------------

mod raw {
    extern "C" {
        // Lookup tables defined in xetex-io.c (declared extern in xetex-io.h).
        pub(super) static firstByteMark: [u8; 7];
        pub(super) static bytesFromUTF8: [u8; 256];
        pub(super) static offsetsFromUTF8: [u32; 6];
    }
}

extern "C" {
    fn xmalloc(size: libc::size_t) -> *mut libc::c_void;
    fn ttstub_get_file_md5(path: *const libc::c_char, digest: *mut u8) -> libc::c_int;
    fn ttbc_get_data_md5(data: *const u8, len: libc::size_t, digest: *mut u8) -> libc::c_int;
    fn _tt_abort(fmt: *const libc::c_char) -> !;
    // Bridge I/O functions (tectonic_bridge_core.h).
    // rust_input_handle_t = uintptr_t = usize; INVALID_HANDLE = 0.
    fn ttstub_input_open(
        path: *const libc::c_char,
        format: libc::c_int,
        is_gz: libc::c_int,
    ) -> usize;
    fn ttstub_input_get_size(handle: usize) -> libc::size_t;
    fn ttstub_input_close(handle: usize) -> libc::c_int;
    fn ttstub_input_seek(handle: usize, offset: libc::ssize_t, whence: libc::c_int) -> libc::size_t;
    fn ttstub_input_read(handle: usize, data: *mut u8, len: libc::size_t) -> libc::ssize_t;
}

// TTBC_FILE_FORMAT_TEX = 26 (tectonic_bridge_core_generated.h:94).
const TTBC_FILE_FORMAT_TEX: libc::c_int = 26;
// INVALID_HANDLE = 0 (tectonic_bridge_core_generated.h:12).
const INVALID_HANDLE: usize = 0;

// Rust-side static state replacing C file-statics `last_source_name` and
// `last_lineno` (xetex-texmfmp.c:19–20). The engine holds a process-wide
// mutex during execution so these have the same concurrency properties as
// the C originals.
static mut LAST_SOURCE_NAME: *mut libc::c_char = std::ptr::null_mut();
static mut LAST_LINENO: i32 = 0;

// ---------------------------------------------------------------------------
// get_date_and_time / get_seconds_and_micros (translated session 13)
// ---------------------------------------------------------------------------

/// translated from xetex-texmfmp.c:116 (get_date_and_time)
#[no_mangle]
pub unsafe extern "C" fn get_date_and_time(
    source_date_epoch: libc::time_t,
    minutes: *mut i32,
    day: *mut i32,
    month: *mut i32,
    year: *mut i32,
) {
    // SAFETY: source_date_epoch is a valid time value; localtime returns a
    // thread-local pointer that is valid until the next call.
    let tmptr = libc::localtime(&source_date_epoch);
    *minutes = (*tmptr).tm_hour * 60 + (*tmptr).tm_min;
    *day = (*tmptr).tm_mday;
    *month = (*tmptr).tm_mon + 1;
    *year = (*tmptr).tm_year + 1900;
}

/// translated from xetex-texmfmp.c:128 (get_seconds_and_micros)
#[no_mangle]
pub unsafe extern "C" fn get_seconds_and_micros(seconds: *mut i32, micros: *mut i32) {
    // SAFETY: tv is properly initialised; null timezone arg is allowed by POSIX.
    let mut tv = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    libc::gettimeofday(&mut tv, std::ptr::null_mut());
    *seconds = tv.tv_sec as i32;
    *micros = tv.tv_usec as i32;
}

// ---------------------------------------------------------------------------
// gettexstring — pool UTF-16 → heap UTF-8
// ---------------------------------------------------------------------------

/// Convert a TeX string-pool string to a heap-allocated NUL-terminated UTF-8
/// string. The caller must free the returned pointer.
///
/// translated from xetex-texmfmp.c:282 (gettexstring)
#[no_mangle]
pub unsafe extern "C" fn gettexstring(s: i32) -> *mut libc::c_char {
    let len: i32 = if s >= 65536 {
        str_start(s + 1 - 65536) - str_start(s - 65536)
    } else {
        0
    };

    // max UTF-16 → UTF-8 expansion: 3 output bytes per input code unit
    let name = xmalloc((len * 3 + 1) as libc::size_t) as *mut u8;
    let mut i: i32 = 0;
    let mut j: usize = 0;

    while i < len {
        let base = str_start(s - 65536);
        let mut c = str_pool_entry(i + base) as u32;
        if c >= 0xD800 && c <= 0xDBFF {
            i += 1;
            let lo = str_pool_entry(i + base) as u32;
            c = if lo >= 0xDC00 && lo <= 0xDFFF {
                (c - 0xD800) * 0x0400 + lo - 0xDC00 + 0x10000
            } else {
                0xFFFD
            };
        }

        let bytes_to_write: usize = if c < 0x80 {
            1
        } else if c < 0x800 {
            2
        } else if c < 0x10000 {
            3
        } else if c < 0x110000 {
            4
        } else {
            c = 0xFFFD;
            3
        };

        // Translate C's falling-through switch that writes the multi-byte sequence
        // backward using pre-decrement j. We instead save `start = j` and write
        // forward; the result is identical.
        let start = j;
        j += bytes_to_write;
        match bytes_to_write {
            4 => {
                *name.add(start + 3) = ((c | 0x80) & 0xBF) as u8;
                c >>= 6;
                *name.add(start + 2) = ((c | 0x80) & 0xBF) as u8;
                c >>= 6;
                *name.add(start + 1) = ((c | 0x80) & 0xBF) as u8;
                c >>= 6;
                *name.add(start) = (c | raw::firstByteMark[4] as u32) as u8;
            }
            3 => {
                *name.add(start + 2) = ((c | 0x80) & 0xBF) as u8;
                c >>= 6;
                *name.add(start + 1) = ((c | 0x80) & 0xBF) as u8;
                c >>= 6;
                *name.add(start) = (c | raw::firstByteMark[3] as u32) as u8;
            }
            2 => {
                *name.add(start + 1) = ((c | 0x80) & 0xBF) as u8;
                c >>= 6;
                *name.add(start) = (c | raw::firstByteMark[2] as u32) as u8;
            }
            _ => {
                *name.add(start) = (c | raw::firstByteMark[1] as u32) as u8;
            }
        }

        i += 1;
    }
    *name.add(j) = 0;
    name as *mut libc::c_char
}

// ---------------------------------------------------------------------------
// checkpool_pointer (private) + maketexstring — heap UTF-8 → pool UTF-16
// ---------------------------------------------------------------------------

/// translated from xetex-texmfmp.c:237 (checkpool_pointer, was static)
unsafe fn check_pool_pointer(pp: i32, len: usize) {
    if (pp as usize) + len >= pool_size() as usize {
        _tt_abort(c"string pool overflow".as_ptr());
    }
}

/// Convert a NUL-terminated C UTF-8 string to a TeX string-pool entry and
/// return its str_number.
///
/// translated from xetex-texmfmp.c:245 (maketexstring)
#[no_mangle]
pub unsafe extern "C" fn maketexstring(s: *const libc::c_char) -> i32 {
    // EMPTY_STRING = TOO_BIG_CHAR + 1 (xetex-stringpool.h:12)
    const EMPTY_STRING: i32 = 65537;

    if s.is_null() || *s == 0 {
        return EMPTY_STRING;
    }

    let len = libc::strlen(s);
    // comment from C: "in the XeTeX case, this may be more than enough"
    check_pool_pointer(pool_ptr(), len);

    let mut cp = s as *const u8;
    loop {
        let mut rval = *cp as u32;
        if rval == 0 {
            break;
        }
        cp = cp.add(1);

        let extra_bytes = raw::bytesFromUTF8[rval as usize] as usize;
        // Translate C's falling-through switch (case 5 … case 0) that shifts in
        // continuation bytes. Each iteration corresponds to one case, running from
        // `extra_bytes` down to 1 (case 0 is a no-op).
        for _ in 0..extra_bytes {
            rval <<= 6;
            if *cp != 0 {
                rval += *cp as u32;
                cp = cp.add(1);
            }
        }
        rval -= raw::offsetsFromUTF8[extra_bytes] as u32;

        if rval > 0xffff {
            rval -= 0x10000;
            set_str_pool(pool_ptr(), (0xd800 + rval / 0x0400) as u16);
            set_pool_ptr(pool_ptr() + 1);
            set_str_pool(pool_ptr(), (0xdc00 + rval % 0x0400) as u16);
            set_pool_ptr(pool_ptr() + 1);
        } else {
            set_str_pool(pool_ptr(), rval as u16);
            set_pool_ptr(pool_ptr() + 1);
        }
    }

    crate::string_pool::make_string()
}

// ---------------------------------------------------------------------------
// convertStringToHexString (private) + getmd5sum
// ---------------------------------------------------------------------------

const DIGEST_SIZE: usize = 16;
const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";

/// translated from xetex-texmfmp.c:396 (convertStringToHexString, was static)
fn convert_string_to_hex(input: &[u8; DIGEST_SIZE], out: &mut [u8; 2 * DIGEST_SIZE + 1]) {
    let mut j = 0usize;
    for &byte in input.iter() {
        out[j] = HEX_CHARS[((byte >> 4) & 0xF) as usize];
        j += 1;
        out[j] = HEX_CHARS[(byte & 0xF) as usize];
        j += 1;
    }
    out[j] = 0;
}

/// Compute MD5 of a file or a data string and push the hex digest into the
/// string pool.
///
/// translated from xetex-texmfmp.c:413 (getmd5sum)
#[no_mangle]
pub unsafe extern "C" fn getmd5sum(s: i32, file: bool) {
    let mut digest = [0u8; DIGEST_SIZE];
    let mut outbuf = [0u8; 2 * DIGEST_SIZE + 1];

    let xname = gettexstring(s);

    let ret = if file {
        ttstub_get_file_md5(xname, digest.as_mut_ptr())
    } else {
        let len = libc::strlen(xname);
        ttbc_get_data_md5(xname as *const u8, len, digest.as_mut_ptr())
    };

    libc::free(xname as *mut libc::c_void);

    if ret != 0 {
        return;
    }

    if pool_ptr() + 2 * DIGEST_SIZE as i32 >= pool_size() {
        return;
    }

    convert_string_to_hex(&digest, &mut outbuf);
    for i in 0..(2 * DIGEST_SIZE) {
        set_str_pool(pool_ptr(), outbuf[i] as u16);
        set_pool_ptr(pool_ptr() + 1);
    }
}

// ---------------------------------------------------------------------------
// make_src_special — write "src:{lineno} {filename}" into the pool
// ---------------------------------------------------------------------------

/// translated from xetex-texmfmp.c:274 (make_src_special)
#[no_mangle]
pub unsafe extern "C" fn make_src_special(srcfilename: i32, lineno: i32) -> i32 {
    let old_pool_ptr = pool_ptr();
    let filename = gettexstring(srcfilename);

    // C used `sprintf(buf, "src:%d ", lineno)` into a 40-byte local buffer.
    let prefix = format!("src:{} ", lineno);
    let fname_len = libc::strlen(filename);

    if (pool_ptr() as usize) + prefix.len() + fname_len >= pool_size() as usize {
        _tt_abort(c"string pool overflow".as_ptr());
    }

    for &b in prefix.as_bytes() {
        set_str_pool(pool_ptr(), b as u16);
        set_pool_ptr(pool_ptr() + 1);
    }

    let mut s = filename as *const u8;
    while *s != 0 {
        set_str_pool(pool_ptr(), *s as u16);
        set_pool_ptr(pool_ptr() + 1);
        s = s.add(1);
    }

    libc::free(filename as *mut libc::c_void);
    old_pool_ptr
}

// ---------------------------------------------------------------------------
// getfilesize — push decimal file size into the pool
// ---------------------------------------------------------------------------

/// translated from xetex-texmfmp.c:155 (getfilesize)
#[no_mangle]
pub unsafe extern "C" fn getfilesize(s: i32) {
    let name = gettexstring(s);
    let handle = ttstub_input_open(name, TTBC_FILE_FORMAT_TEX, 0);
    libc::free(name as *mut libc::c_void);

    if handle == INVALID_HANDLE {
        return;
    }

    let file_len = ttstub_input_get_size(handle);
    ttstub_input_close(handle);

    // C used `snprintf(buf, 20, "%lu", file_len)` then wrote bytes to pool.
    let text = format!("{}", file_len as u64);
    let text_bytes = text.as_bytes();
    let text_len = text_bytes.len();

    if (pool_ptr() as usize + text_len) >= pool_size() as usize {
        set_pool_ptr(pool_size());
    } else {
        for &b in text_bytes {
            set_str_pool(pool_ptr(), b as u16);
            set_pool_ptr(pool_ptr() + 1);
        }
    }
}

// ---------------------------------------------------------------------------
// getfiledump — push hex dump of file bytes into the pool
// ---------------------------------------------------------------------------

/// translated from xetex-texmfmp.c:189 (getfiledump)
#[no_mangle]
pub unsafe extern "C" fn getfiledump(s: i32, offset: libc::c_int, length: libc::c_int) {
    if length == 0 {
        return;
    }

    if pool_ptr() + 2 * length + 1 >= pool_size() {
        set_pool_ptr(pool_size());
        return;
    }

    let buffer = xmalloc((length + 1) as libc::size_t) as *mut u8;
    if buffer.is_null() {
        set_pool_ptr(pool_size());
        return;
    }

    let name = gettexstring(s);
    let handle = ttstub_input_open(name, TTBC_FILE_FORMAT_TEX, 0);
    libc::free(name as *mut libc::c_void);

    if handle == INVALID_HANDLE {
        libc::free(buffer as *mut libc::c_void);
        return;
    }

    ttstub_input_seek(handle, offset as libc::ssize_t, libc::SEEK_SET);
    let actual = ttstub_input_read(handle, buffer, length as libc::size_t);
    ttstub_input_close(handle);

    // C used `snprintf(strbuf, 3, "%.2X", buffer[j])` — 2 uppercase hex digits.
    for j in 0..actual as usize {
        let byte = *buffer.add(j);
        let hex = format!("{:02X}", byte);
        for &c in hex.as_bytes() {
            set_str_pool(pool_ptr(), c as u16);
            set_pool_ptr(pool_ptr() + 1);
        }
    }

    libc::free(buffer as *mut libc::c_void);
}

// ---------------------------------------------------------------------------
// compare_paths (private) + is_new_source + remember_source_info
// ---------------------------------------------------------------------------

/// Compare two NUL-terminated byte strings as file paths, treating any two
/// directory separators as equal. Returns -1, 0, or 1.
///
/// translated from xetex-texmfmp.c:243 (compare_paths, was static)
unsafe fn compare_paths(p1: *const libc::c_char, p2: *const libc::c_char) -> i32 {
    // Handle null pointers: treat null same as empty string.
    let p1 = if p1.is_null() { c"".as_ptr() } else { p1 };
    let p2 = if p2.is_null() { c"".as_ptr() } else { p2 };

    let mut p1 = p1 as *const u8;
    let mut p2 = p2 as *const u8;
    let mut ret: i32;
    loop {
        ret = (*p1 as i32) - (*p2 as i32);
        let equal_and_not_null = ret == 0 && *p2 != 0;
        let both_dir_sep = *p1 == b'/' && *p2 == b'/';
        if !(equal_and_not_null || both_dir_sep) {
            break;
        }
        p1 = p1.add(1);
        p2 = p2.add(1);
    }
    if ret < 0 { -1 } else if ret > 0 { 1 } else { 0 }
}

/// Return true if the source file or line number changed since the last call to
/// `remember_source_info`.
///
/// translated from xetex-texmfmp.c:257 (is_new_source)
#[no_mangle]
pub unsafe extern "C" fn is_new_source(srcfilename: i32, lineno: i32) -> bool {
    let name = gettexstring(srcfilename);
    let changed = compare_paths(name, LAST_SOURCE_NAME) != 0 || lineno != LAST_LINENO;
    libc::free(name as *mut libc::c_void);
    changed
}

/// Record the current source file and line number for later comparison by
/// `is_new_source`.
///
/// translated from xetex-texmfmp.c:264 (remember_source_info)
#[no_mangle]
pub unsafe extern "C" fn remember_source_info(srcfilename: i32, lineno: i32) {
    libc::free(LAST_SOURCE_NAME as *mut libc::c_void);
    LAST_SOURCE_NAME = gettexstring(srcfilename);
    LAST_LINENO = lineno;
}
