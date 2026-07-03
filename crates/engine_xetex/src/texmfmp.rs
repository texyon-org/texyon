//! Miscellaneous C-callable helpers from xetex-texmfmp.c.

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
    // Bridge I/O functions (tectonic_bridge_core.h).
    // rust_input_handle_t = uintptr_t = usize; INVALID_HANDLE = 0.
    fn ttstub_input_open(
        path: *const libc::c_char,
        format: libc::c_int,
        is_gz: libc::c_int,
    ) -> usize;
    fn ttstub_input_get_size(handle: usize) -> libc::size_t;
    fn ttstub_input_get_mtime(handle: usize) -> libc::time_t;
    fn ttstub_input_read(handle: usize, data: *mut u8, len: libc::size_t) -> libc::ssize_t;
}

use crate::consts::INVALID_HANDLE;

// TTBC_FILE_FORMAT_TEX = 26 (tectonic_bridge_core_generated.h:94).
const TTBC_FILE_FORMAT_TEX: libc::c_int = 26;

// `last_source_name` is an owned C string; its free/reassign lifecycle stays
// in `remember_source_info` (no `tt_cleanup` edge — see state.rs::TexmfState).

// ---------------------------------------------------------------------------
// get_date_and_time / get_seconds_and_micros
// ---------------------------------------------------------------------------

/// translated from xetex-texmfmp.c:116 (get_date_and_time)
pub unsafe fn get_date_and_time(
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

// ---------------------------------------------------------------------------
// get_microinterval (translated from xetex-xetex0.c)
// ---------------------------------------------------------------------------

/// Return a fixed-point scaled representation of (now - epoch) in 1/65536 sec.
///
/// translated from xetex-xetex0.c:155 (get_microinterval)
pub unsafe fn get_microinterval(shipout: &crate::state::ShipoutState) -> i32 {
    let mut s = 0i32;
    let mut m = 0i32;
    get_seconds_and_micros(&mut s, &mut m);
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

/// translated from xetex-texmfmp.c:128 (get_seconds_and_micros)
pub unsafe fn get_seconds_and_micros(seconds: *mut i32, micros: *mut i32) {
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

/// `&StringsState` core of [`gettexstring`].
pub unsafe fn gettexstring_in(strings: &crate::state::StringsState, s: i32) -> *mut libc::c_char {
    let str_start = |k: i32| strings.str_start[k as usize];
    let str_pool_entry = |k: i32| strings.str_pool[k as usize];
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
        if (0xD800..=0xDBFF).contains(&c) {
            i += 1;
            let lo = str_pool_entry(i + base) as u32;
            c = if (0xDC00..=0xDFFF).contains(&lo) {
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

/// Convert a NUL-terminated C UTF-8 string to a TeX string-pool entry and
/// return its str_number.
///
/// Cold-abort sentinel for [`maketexstring_in`].
pub(crate) enum MakeTexStringAbort {
    /// `_tt_abort("string pool overflow")` — the pool can't hold the string.
    PoolOverflow,
    /// `overflow("number of strings", …)` — `str_ptr` exhausted in `make_string`.
    StringOverflow,
}

impl MakeTexStringAbort {
    /// Map to the unified [`EngineAbort`] terminal.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            MakeTexStringAbort::PoolOverflow => {
                crate::errors::EngineAbort::TtAbort(c"string pool overflow".as_ptr())
            }
            MakeTexStringAbort::StringOverflow => crate::errors::EngineAbort::StringOverflow,
        }
    }
}

/// `&mut StringsState` core of [`maketexstring`].
///
/// Writes the UTF-8 → pool UTF-16 conversion directly into the string pool;
/// the pool-overflow and string-overflow cold paths ride
/// [`MakeTexStringAbort`].
pub(crate) unsafe fn maketexstring_in(
    strings: &mut crate::state::StringsState,
    s: *const libc::c_char,
) -> Result<i32, MakeTexStringAbort> {
    // EMPTY_STRING = TOO_BIG_CHAR + 1 (xetex-stringpool.h:12)
    const EMPTY_STRING: i32 = 65537;

    if s.is_null() || *s == 0 {
        return Ok(EMPTY_STRING);
    }

    let len = libc::strlen(s);
    // comment from C: "in the XeTeX case, this may be more than enough"
    if (strings.pool_ptr as usize) + len >= strings.pool_size as usize {
        return Err(MakeTexStringAbort::PoolOverflow);
    }

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
        rval -= raw::offsetsFromUTF8[extra_bytes];

        if rval > 0xffff {
            rval -= 0x10000;
            strings.str_pool[strings.pool_ptr as usize] = (0xd800 + rval / 0x0400) as u16;
            strings.pool_ptr += 1;
            strings.str_pool[strings.pool_ptr as usize] = (0xdc00 + rval % 0x0400) as u16;
            strings.pool_ptr += 1;
        } else {
            strings.str_pool[strings.pool_ptr as usize] = rval as u16;
            strings.pool_ptr += 1;
        }
    }

    crate::string_pool::make_string(strings).map_err(|_| MakeTexStringAbort::StringOverflow)
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

/// `&mut StringsState` core of [`getmd5sum`].
pub(crate) unsafe fn getmd5sum_in(strings: &mut crate::state::StringsState, s: i32, file: bool) {
    let mut digest = [0u8; DIGEST_SIZE];
    let mut outbuf = [0u8; 2 * DIGEST_SIZE + 1];

    let xname = gettexstring_in(strings, s);

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

    if strings.pool_ptr + 2 * DIGEST_SIZE as i32 >= strings.pool_size {
        return;
    }

    convert_string_to_hex(&digest, &mut outbuf);
    for i in 0..(2 * DIGEST_SIZE) {
        strings.str_pool[strings.pool_ptr as usize] = outbuf[i] as u16;
        strings.pool_ptr += 1;
    }
}

// ---------------------------------------------------------------------------
// make_src_special — write "src:{lineno} {filename}" into the pool
// ---------------------------------------------------------------------------

/// `&mut EngineState` core of the `make_src_special` op. Returns the cold pool
/// overflow as `Err(())` for the caller to defer.
pub(crate) fn make_src_special_in(
    st: &mut crate::state::EngineState,
    srcfilename: i32,
    lineno: i32,
) -> Result<i32, ()> {
    let old_pool_ptr = st.strings.pool_ptr;
    // SAFETY: `gettexstring_in` returns a freshly-allocated NUL-terminated C
    // string; we walk it to NUL and `free` it. Pool writes are safe Vec indexing.
    unsafe {
        let filename = gettexstring_in(&st.strings, srcfilename);

        // C used `sprintf(buf, "src:%d ", lineno)` into a 40-byte local buffer.
        let prefix = format!("src:{} ", lineno);
        let fname_len = libc::strlen(filename);

        if (st.strings.pool_ptr as usize) + prefix.len() + fname_len
            >= st.strings.pool_size as usize
        {
            return Err(());
        }

        for &b in prefix.as_bytes() {
            st.strings.str_pool[st.strings.pool_ptr as usize] = b as u16;
            st.strings.pool_ptr += 1;
        }

        let mut s = filename as *const u8;
        while *s != 0 {
            st.strings.str_pool[st.strings.pool_ptr as usize] = *s as u16;
            st.strings.pool_ptr += 1;
            s = s.add(1);
        }

        libc::free(filename as *mut libc::c_void);
    }
    Ok(old_pool_ptr)
}

// ---------------------------------------------------------------------------
// getfilesize — push decimal file size into the pool
// ---------------------------------------------------------------------------

/// translated from xetex-texmfmp.c:155 (getfilesize)
pub(crate) unsafe fn getfilesize_in(
    strings: &mut crate::state::StringsState,
    s: i32,
) -> Result<(), crate::scan::SccAbort> {
    let name = gettexstring_in(strings, s);
    let handle = ttstub_input_open(name, TTBC_FILE_FORMAT_TEX, 0);
    libc::free(name as *mut libc::c_void);

    if handle == INVALID_HANDLE {
        return Ok(());
    }

    let file_len = ttstub_input_get_size(handle);
    if crate::io::input_close(handle) {
        return Err(crate::scan::SccAbort::InputClose);
    }

    // C used `snprintf(buf, 20, "%lu", file_len)` then wrote bytes to pool.
    let text = format!("{}", file_len as u64);
    let text_bytes = text.as_bytes();
    let text_len = text_bytes.len();

    if (strings.pool_ptr as usize + text_len) >= strings.pool_size as usize {
        strings.pool_ptr = strings.pool_size;
    } else {
        for &b in text_bytes {
            strings.str_pool[strings.pool_ptr as usize] = b as u16;
            strings.pool_ptr += 1;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// getfiledump — push hex dump of file bytes into the pool
// ---------------------------------------------------------------------------

/// translated from xetex-texmfmp.c:189 (getfiledump)
pub(crate) unsafe fn getfiledump_in(
    strings: &mut crate::state::StringsState,
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

    let buffer = xmalloc((length + 1) as libc::size_t) as *mut u8;
    if buffer.is_null() {
        strings.pool_ptr = strings.pool_size;
        return Ok(());
    }

    let name = gettexstring_in(strings, s);
    let handle = ttstub_input_open(name, TTBC_FILE_FORMAT_TEX, 0);
    libc::free(name as *mut libc::c_void);

    if handle == INVALID_HANDLE {
        libc::free(buffer as *mut libc::c_void);
        return Ok(());
    }

    crate::io::input_seek(handle, offset as libc::ssize_t, libc::SEEK_SET);
    let actual = ttstub_input_read(handle, buffer, length as libc::size_t);
    if crate::io::input_close(handle) {
        // Serious internal close error the C wrapper longjmp'd on; the old bare
        // longjmp skipped the buffer free too (immaterial on a terminal abort).
        return Err(crate::scan::SccAbort::InputClose);
    }

    // C used `snprintf(strbuf, 3, "%.2X", buffer[j])` — 2 uppercase hex digits.
    for j in 0..actual as usize {
        let byte = *buffer.add(j);
        let hex = format!("{:02X}", byte);
        for &c in hex.as_bytes() {
            strings.str_pool[strings.pool_ptr as usize] = c as u16;
            strings.pool_ptr += 1;
        }
    }

    libc::free(buffer as *mut libc::c_void);
    Ok(())
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
    if ret < 0 {
        -1
    } else if ret > 0 {
        1
    } else {
        0
    }
}

/// `&EngineState` core of [`is_new_source`].
pub(crate) fn is_new_source_in(
    st: &crate::state::EngineState,
    srcfilename: i32,
    lineno: i32,
) -> bool {
    // SAFETY: `gettexstring_in` returns an owned C string compared by FFI then freed.
    unsafe {
        let name = gettexstring_in(&st.strings, srcfilename);
        let changed =
            compare_paths(name, st.texmf.last_source_name) != 0 || lineno != st.texmf.last_lineno;
        libc::free(name as *mut libc::c_void);
        changed
    }
}

/// `&mut EngineState` core of [`remember_source_info`].
pub(crate) fn remember_source_info_in(
    st: &mut crate::state::EngineState,
    srcfilename: i32,
    lineno: i32,
) {
    // SAFETY: frees the previously-owned name and stores a fresh owned C string.
    unsafe {
        let old = st.texmf.last_source_name;
        libc::free(old as *mut libc::c_void);
        let new_name = gettexstring_in(&st.strings, srcfilename);
        st.texmf.last_source_name = new_name;
    }
    st.texmf.last_lineno = lineno;
}

// ---------------------------------------------------------------------------
// makepdftime (private) + init_start_time + getcreationdate + getfilemoddate
// ---------------------------------------------------------------------------

/// Format a Unix timestamp as a PDF date string into `time_str`.
///
/// translated from xetex-texmfmp.c:30 (makepdftime, was static)
unsafe fn makepdftime(t: libc::time_t, time_str: &mut [u8; 30], utc: bool) {
    let lt = if utc {
        *libc::gmtime(&t)
    } else {
        *libc::localtime(&t)
    };
    let size = libc::strftime(
        time_str.as_mut_ptr() as *mut libc::c_char,
        30,
        c"D:%Y%m%d%H%M%S".as_ptr(),
        &lt as *const libc::tm,
    );
    if size == 0 {
        time_str[0] = 0;
        return;
    }

    // %S can be 60 or 61 (leap seconds); PDF expects 00..59.
    if time_str[14] == b'6' {
        time_str[14] = b'5';
        time_str[15] = b'9';
        time_str[16] = 0;
    }

    // Compute UTC offset in minutes (method from exim's tod.c).
    let gmt = *libc::gmtime(&t);
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
        // C: snprintf(&time_str[size], 9, "%+03d'%02d'", off_hours, off_mins)
        let tz = format!("{:+03}'{:02}'", off_hours, off_mins);
        let tz = tz.as_bytes();
        let copy_len = tz.len().min(8);
        time_str[size..size + copy_len].copy_from_slice(&tz[..copy_len]);
        time_str[size + copy_len] = 0;
    }
}

/// Store the PDF date string for `source_date_epoch` in the module-level
/// static buffer for later use by `getcreationdate`.
///
/// translated from xetex-texmfmp.c:84 (init_start_time)
pub unsafe fn init_start_time(st: &mut crate::state::EngineState, source_date_epoch: libc::time_t) {
    let mut buf = [0u8; 30];
    makepdftime(source_date_epoch, &mut buf, true);
    st.texmf.start_time_str = buf;
}

/// `&mut StringsState` core of [`getcreationdate`].
pub(crate) unsafe fn getcreationdate_in(
    texmf: &crate::state::TexmfState,
    strings: &mut crate::state::StringsState,
) {
    let start = texmf.start_time_str;
    let len = libc::strlen(start.as_ptr() as *const libc::c_char);
    if (strings.pool_ptr as usize) + len >= strings.pool_size as usize {
        strings.pool_ptr = strings.pool_size;
        return;
    }
    for i in 0..len {
        strings.str_pool[strings.pool_ptr as usize] = start[i] as u16;
        strings.pool_ptr += 1;
    }
}

/// Push the modification date of a file (named by pool string `s`) into the
/// string pool as a PDF date string.
///
/// translated from xetex-texmfmp.c:120 (getfilemoddate)
pub(crate) unsafe fn getfilemoddate_in(
    strings: &mut crate::state::StringsState,
    s: i32,
) -> Result<(), crate::scan::SccAbort> {
    let name = gettexstring_in(strings, s);
    let handle = ttstub_input_open(name, TTBC_FILE_FORMAT_TEX, 0);
    libc::free(name as *mut libc::c_void);

    if handle == INVALID_HANDLE {
        return Ok(());
    }

    let mtime = ttstub_input_get_mtime(handle);
    if crate::io::input_close(handle) {
        return Err(crate::scan::SccAbort::InputClose);
    }

    // Use a 30-byte buffer (= TIME_STR_SIZE) not the C's 20-byte buf which
    // was too small for non-UTC timestamps.
    let mut buf = [0u8; 30];
    makepdftime(mtime, &mut buf, true);
    let text_len = libc::strlen(buf.as_ptr() as *const libc::c_char);

    if (strings.pool_ptr as usize) + text_len >= strings.pool_size as usize {
        strings.pool_ptr = strings.pool_size;
    } else {
        for i in 0..text_len {
            strings.str_pool[strings.pool_ptr as usize] = buf[i] as u16;
            strings.pool_ptr += 1;
        }
    }
    Ok(())
}
