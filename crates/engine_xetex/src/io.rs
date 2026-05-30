//! Rust translation of xetex-io.c: low-level input/output helpers.

use crate::consts::{INT_BASE};
use crate::globals::{
    buf_size, buffer_entry, buffer_ptr, cur_area, cur_chr, cur_ext, cur_name, cur_val, eqtb_b32s1,
    first, last, max_buf_stack, name_length, name_length16, name_of_file, name_of_file16,
    name_of_file16_entry, read_file_entry, read_file_ptr, read_open_entry, set_buffer_entry,
    set_last, set_max_buf_stack, set_name_in_progress, set_name_length, set_name_length16,
    set_name_of_file16, set_read_open_entry, set_stop_at_space,
};

// ---------------------------------------------------------------------------
// UFILE — Rust mirror of the C `UFILE` struct (xetex-io.h:12–18)
// ---------------------------------------------------------------------------

/// Rust mirror of `UFILE` (xetex-io.h:12–18). Must stay layout-compatible.
///
/// On Linux x86_64 the C layout is:
use crate::output::bad_utf8_warning;
use crate::output::begin_diagnostic;
use crate::output::capture_to_diagnostic;
use crate::output::diagnostic_begin_capture_warning_here;
use crate::output::end_diagnostic;
use crate::output::print_cstr;
use crate::output::print_int;
use crate::output::print_nl;
use crate::output::print_nl_cstr;
use crate::scan::begin_name;
use crate::scan::end_name;
use crate::scan::more_name;
use crate::scan::pack_file_name;
use crate::scan::scan_file_name;
use crate::scan::scan_four_bit_int;
use crate::scan::scan_optional_equals;
use crate::texmfmp::gettexstring;

///   offset 0: handle (uintptr_t, 8 B)
///   offset 8: savedChar (long, 8 B)
///   offset 16: skipNextLF (short, 2 B)
///   offset 18: encodingMode (short, 2 B)
///   offset 20: [4 B padding]
///   offset 24: conversionData (*void, 8 B)
///   total: 32 bytes
#[repr(C)]
pub(crate) struct UFile {
    pub handle: usize,
    pub saved_char: i64, // C `long` = i64 on Linux 64-bit
    pub skip_next_lf: i16,
    pub encoding_mode: i16,
    pub conversion_data: *mut libc::c_void,
}

const _: () = assert!(
    std::mem::size_of::<UFile>() == 32,
    "UFile size mismatch with C UFILE"
);

// Encoding mode constants (xetex-xetexd.h:1210–1215).
const AUTO: i32 = 0;
const UTF8: i32 = 1;
const UTF16BE: i32 = 2;
const UTF16LE: i32 = 3;
const RAW: i32 = 4;
const ICUMAPPING: i32 = 5;

// TECkit normalization form constants (teckit-Common.h:92–93, xetex-core.h:29,34).
// NATIVE_UTF32 = kForm_UTF32LE on little-endian (Linux x86_64).
#[cfg(target_endian = "little")]
const NATIVE_UTF32: u16 = 6; // kForm_UTF32LE
#[cfg(target_endian = "big")]
const NATIVE_UTF32: u16 = 5; // kForm_UTF32BE
const K_FORM_NFC: u16 = 0x0100;
const K_FORM_NFD: u16 = 0x0200;

// TECkit status constants (teckit-Common.h:55).
const K_STATUS_NO_ERROR: i64 = 0;

// File format constants (tectonic_bridge_core_generated.h).
const TTBC_FILE_FORMAT_TECTONIC_PRIMARY: libc::c_int = 59;
const TTBC_FILE_FORMAT_TEX: libc::c_int = 26;
// INVALID_HANDLE = 0
const INVALID_HANDLE: usize = 0;

// Constants for open_or_close_in (xetex-constants.h:268,277).
const JUST_OPEN: u8 = 1;
const CLOSED: u8 = 2;

// INT_BASE and INT_PAR offsets used by open_or_close_in (xetex_format.h:219,110–111).
const INT_PAR_XETEX_DEFAULT_INPUT_MODE: i32 = 75;
const INT_PAR_XETEX_DEFAULT_INPUT_ENCODING: i32 = 76;

// ---------------------------------------------------------------------------
// C-side lookup tables (defined in xetex-io.c, declared extern in xetex-io.h)
// ---------------------------------------------------------------------------

mod raw {
    extern "C" {
        pub(super) static bytesFromUTF8: [u8; 256];
        pub(super) static offsetsFromUTF8: [u32; 6];
    }
}

// ---------------------------------------------------------------------------
// C externs needed by translated functions
// ---------------------------------------------------------------------------

extern "C" {
    fn xmalloc(size: libc::size_t) -> *mut libc::c_void;
    fn xcalloc(nelem: libc::size_t, elsize: libc::size_t) -> *mut libc::c_void;
    fn xstrdup(s: *const libc::c_char) -> *mut libc::c_char;
    fn _tt_abort(fmt: *const libc::c_char) -> !;

    fn ttstub_input_open(
        path: *const libc::c_char,
        format: libc::c_int,
        is_gz: libc::c_int,
    ) -> usize;
    fn ttstub_input_open_primary() -> usize;
    fn ttstub_get_last_input_abspath(buffer: *mut libc::c_char, len: libc::size_t)
        -> libc::ssize_t;
    fn ttstub_input_close(handle: usize) -> libc::c_int;
    fn ttstub_input_seek(handle: usize, offset: libc::ssize_t, whence: libc::c_int)
        -> libc::size_t;
    fn ttstub_input_getc(handle: usize) -> libc::c_int;
    fn ttstub_input_ungetc(handle: usize, ch: libc::c_int) -> libc::c_int;

    // Thin C wrappers around ICU functions (ICU linked via C objects, not directly from Rust).
    fn xetex_ucnv_close(converter: *mut libc::c_void);
    fn xetex_ucnv_open(name: *const libc::c_char, out_err: *mut libc::c_int) -> *mut libc::c_void;
    fn xetex_ucnv_to_utf32(
        cnv: *mut libc::c_void,
        target: *mut libc::c_char,
        target_capacity: i32,
        source: *const libc::c_char,
        source_length: i32,
        out_err: *mut libc::c_int,
    ) -> i32;

    // TECkit functions (C++ engine with C API — extern "C" ABI).
    // TECkit_Status is `long` = i64 on Linux 64-bit; TECkit_Converter is opaque ptr (*mut c_void).
    fn TECkit_CreateConverter(
        mapping: *const u8,
        mapping_size: u32,
        map_forward: u8,
        source_form: u16,
        target_form: u16,
        converter: *mut *mut libc::c_void,
    ) -> i64;
    fn TECkit_ConvertBuffer(
        converter: *mut libc::c_void,
        in_buffer: *const u8,
        in_length: u32,
        in_used: *mut u32,
        out_buffer: *mut u8,
        out_length: u32,
        out_used: *mut u32,
        input_is_complete: u8,
    ) -> i64;
    fn TECkit_ResetConverter(converter: *mut libc::c_void) -> i64;

    // bad_utf8_warning remains in C (xetex-xetexd.h:912).

    // gettexstring — translated to Rust (texmfmp.rs) but called via FFI.

    // Output helpers — all translated to Rust in output.rs.

    // Callees of open_or_close_in still in C.

}

// ---------------------------------------------------------------------------
// Rust statics replacing C file-level variables (xetex-io.c:14,20)
// ---------------------------------------------------------------------------

/// Replaces `char *name_of_input_file = NULL` in xetex-io.c.
#[no_mangle]
pub static mut name_of_input_file: *mut libc::c_char = std::ptr::null_mut();

/// Replaces `char abspath_of_input_file[1024] = ""` in xetex-io.c.
#[no_mangle]
pub static mut abspath_of_input_file: [libc::c_char; 1024] = [0; 1024];



// ---------------------------------------------------------------------------
// tt_xetex_open_input
// ---------------------------------------------------------------------------

/// Open the current input file (named by `name_of_file`) via the bridge.
///
/// translated from xetex-io.c:22 (tt_xetex_open_input)
pub unsafe fn tt_xetex_open_input(filefmt: libc::c_int) -> usize {
    let handle = if filefmt == TTBC_FILE_FORMAT_TECTONIC_PRIMARY {
        ttstub_input_open_primary()
    } else if *name_of_file() == b'|' as libc::c_char {
        print_nl_cstr(c"Warning: ".as_ptr());
        diagnostic_begin_capture_warning_here();
        print_cstr(c"piped inputs from external commands are not implemented in Tectonic".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        return INVALID_HANDLE;
    } else {
        ttstub_input_open(name_of_file(), filefmt, 0)
    };

    if handle == INVALID_HANDLE {
        return INVALID_HANDLE;
    }

    if ttstub_get_last_input_abspath(
        abspath_of_input_file.as_mut_ptr(),
        abspath_of_input_file.len(),
    ) < 1
    {
        abspath_of_input_file[0] = 0;
    }

    set_name_length(libc::strlen(name_of_file()) as i32);
    libc::free(name_of_input_file as *mut libc::c_void);
    name_of_input_file = xstrdup(name_of_file());
    handle
}

// ---------------------------------------------------------------------------
// u_open_in
// ---------------------------------------------------------------------------

/// Open a UFILE for reading with BOM-sniffing encoding detection.
///
/// translated from xetex-io.c:127 (u_open_in)
#[no_mangle]
pub unsafe extern "C" fn u_open_in(
    f: *mut *mut UFile,
    filefmt: i32,
    _fopen_mode: *const libc::c_char,
    mut mode: i32,
    encoding_data: i32,
) -> libc::c_int {
    let handle = tt_xetex_open_input(filefmt);
    if handle == INVALID_HANDLE {
        return 0;
    }

    *f = xmalloc(std::mem::size_of::<UFile>()) as *mut UFile;
    (**f).encoding_mode = 0;
    (**f).conversion_data = std::ptr::null_mut();
    (**f).saved_char = -1;
    (**f).skip_next_lf = 0;
    (**f).handle = handle;

    if mode == AUTO {
        let b1 = ttstub_input_getc((**f).handle);
        let b2 = ttstub_input_getc((**f).handle);

        if b1 == 0xfe && b2 == 0xff {
            mode = UTF16BE;
        } else if b2 == 0xfe && b1 == 0xff {
            mode = UTF16LE;
        } else if b1 == 0 && b2 != 0 {
            mode = UTF16BE;
            ttstub_input_seek((**f).handle, 0, libc::SEEK_SET);
        } else if b2 == 0 && b1 != 0 {
            mode = UTF16LE;
            ttstub_input_seek((**f).handle, 0, libc::SEEK_SET);
        } else if b1 == 0xEF && b2 == 0xBB {
            let b3 = ttstub_input_getc((**f).handle);
            if b3 == 0xBF {
                mode = UTF8;
            }
        }

        if mode == AUTO {
            ttstub_input_seek((**f).handle, 0, libc::SEEK_SET);
            mode = UTF8;
        }
    }

    set_input_file_encoding(*f, mode, encoding_data);
    1
}

/// Wrapper so scan.rs can call `set_input_file_encoding` without re-declaring it.
pub(crate) unsafe fn call_set_input_file_encoding(f: *mut UFile, mode: i32, encoding_data: i32) {
    set_input_file_encoding(f, mode, encoding_data);
}

// ---------------------------------------------------------------------------
// set_input_file_encoding
// ---------------------------------------------------------------------------

/// Set the encoding mode (and optional ICU converter) on an open UFILE.
///
/// translated from xetex-io.c:47 (set_input_file_encoding)
#[no_mangle]
pub unsafe extern "C" fn set_input_file_encoding(f: *mut UFile, mode: i32, encoding_data: i32) {
    if (*f).encoding_mode == ICUMAPPING as i16 && !(*f).conversion_data.is_null() {
        xetex_ucnv_close((*f).conversion_data);
    }
    (*f).conversion_data = std::ptr::null_mut();

    match mode {
        UTF8 | UTF16BE | UTF16LE | RAW => {
            (*f).encoding_mode = mode as i16;
        }
        _ if mode == ICUMAPPING => {
            let name = gettexstring(encoding_data);
            let mut err: libc::c_int = 0;
            let cnv = xetex_ucnv_open(name, &mut err);
            if cnv.is_null() {
                begin_diagnostic();
                print_nl(b'E' as i32);
                print_cstr(c"rror ".as_ptr());
                print_int(err);
                print_cstr(c" creating Unicode converter for `".as_ptr());
                print_cstr(name);
                print_cstr(c"'; reading as raw bytes".as_ptr());
                end_diagnostic(true);
                (*f).encoding_mode = RAW as i16;
            } else {
                (*f).encoding_mode = ICUMAPPING as i16;
                (*f).conversion_data = cnv;
            }
            libc::free(name as *mut libc::c_void);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// input_line private helpers: buffer_overflow, conversion_error, apply_normalization
// ---------------------------------------------------------------------------

// Persistent state for input_line, replacing C file-statics.
// `BYTE_BUFFER`: byte staging area for ICU decoding (xetex-io.c:138).
static mut BYTE_BUFFER: *mut libc::c_char = std::ptr::null_mut();
// `UTF32_BUF`: UTF-32 staging area for normalization (xetex-io.c:139).
static mut UTF32_BUF: *mut u32 = std::ptr::null_mut();
// TECkit normalizer converters; replaces `static TECkit_Converter normalizers[2]` (xetex-io.c:113).
static mut NORMALIZERS: [*mut libc::c_void; 2] = [std::ptr::null_mut(), std::ptr::null_mut()];

// translated from xetex-io.c:92 (buffer_overflow, was static)
unsafe fn buffer_overflow() -> ! {
    _tt_abort(c"unable to read an entire line (buffer too small)".as_ptr())
}

// translated from xetex-io.c:98 (conversion_error, was static)
unsafe fn conversion_error(errcode: i32) {
    begin_diagnostic();
    print_nl(b'U' as i32);
    print_cstr(c"nicode conversion failed (ICU error code = ".as_ptr());
    print_int(errcode);
    print_cstr(c") discarding any remaining text".as_ptr());
    end_diagnostic(true);
}

// translated from xetex-io.c:110 (apply_normalization, was static)
// Sets `last` on return; called from `input_line`.
unsafe fn apply_normalization(buf: *mut u32, len: i32, norm: i32) {
    let norm_idx = (norm - 1) as usize; // 0 for NFC, 1 for NFD
    let norm_ptr = &mut NORMALIZERS[norm_idx];
    if norm_ptr.is_null() {
        let target_form = NATIVE_UTF32 | if norm == 1 { K_FORM_NFC } else { K_FORM_NFD };
        let status = TECkit_CreateConverter(
            std::ptr::null(),
            0,
            1,
            NATIVE_UTF32,
            target_form,
            norm_ptr as *mut *mut libc::c_void,
        );
        if status != K_STATUS_NO_ERROR {
            _tt_abort(c"failed to create normalizer".as_ptr())
        }
    }

    let mut in_used: u32 = 0;
    let mut out_used: u32 = 0;
    let status = TECkit_ConvertBuffer(
        *norm_ptr,
        buf as *const u8,
        (len * 4) as u32,
        &mut in_used,
        buffer_ptr(first()) as *mut u8,
        (4 * (buf_size() - first())) as u32,
        &mut out_used,
        1,
    );
    TECkit_ResetConverter(*norm_ptr);
    if status != K_STATUS_NO_ERROR {
        buffer_overflow();
    }
    set_last(first() + (out_used / 4) as i32);
}

// ---------------------------------------------------------------------------
// input_line
// ---------------------------------------------------------------------------

/// Read one line from UFILE into the TeX `buffer`, handling encodings.
///
/// translated from xetex-io.c:135 (input_line)
pub unsafe fn input_line(f: *mut UFile) -> libc::c_int {
    let norm = crate::scan::get_input_normalization_state();

    if (*f).handle == INVALID_HANDLE {
        _tt_abort(c"reads from synthetic \"terminal\" file #0 should never happen".as_ptr())
    }

    set_last(first());

    if (*f).encoding_mode as i32 == ICUMAPPING {
        // ICU-converter path.
        let mut bytes_read: u32 = 0;
        let mut error_code: libc::c_int = 0;

        if BYTE_BUFFER.is_null() {
            BYTE_BUFFER = xmalloc((buf_size() + 1) as libc::size_t) as *mut libc::c_char;
        }

        let mut i = ttstub_input_getc((*f).handle);
        if (*f).skip_next_lf != 0 {
            (*f).skip_next_lf = 0;
            if i == b'\n' as libc::c_int {
                i = ttstub_input_getc((*f).handle);
            }
        }

        if i != libc::EOF && i != b'\n' as i32 && i != b'\r' as i32 {
            *BYTE_BUFFER.add(bytes_read as usize) = i as libc::c_char;
            bytes_read += 1;
            while bytes_read < buf_size() as u32 {
                i = ttstub_input_getc((*f).handle);
                if i == libc::EOF || i == b'\n' as i32 || i == b'\r' as i32 {
                    break;
                }
                *BYTE_BUFFER.add(bytes_read as usize) = i as libc::c_char;
                bytes_read += 1;
            }
        }

        if i == libc::EOF && *libc::__errno_location() != libc::EINTR && bytes_read == 0 {
            return 0; // false
        }

        if i != libc::EOF && i != b'\n' as i32 && i != b'\r' as i32 {
            buffer_overflow();
        }

        // Apply ICU mapping to produce UTF-32 in buffer.
        let cnv = (*f).conversion_data;
        match norm {
            1 | 2 => {
                // NFC or NFD: convert to utf32Buf first, then normalize into buffer.
                if UTF32_BUF.is_null() {
                    UTF32_BUF =
                        xcalloc(buf_size() as libc::size_t, std::mem::size_of::<u32>()) as *mut u32;
                }
                let tmp_len = xetex_ucnv_to_utf32(
                    cnv,
                    UTF32_BUF as *mut libc::c_char,
                    buf_size() * 4,
                    BYTE_BUFFER as *const libc::c_char,
                    bytes_read as i32,
                    &mut error_code,
                );
                if error_code != 0 {
                    conversion_error(error_code);
                    return 0;
                }
                // tmp_len is bytes; divide by 4 to get u32 count.
                apply_normalization(UTF32_BUF, tmp_len / 4, norm);
            }
            _ => {
                // No normalization: convert directly into buffer[first..].
                let out_len = xetex_ucnv_to_utf32(
                    cnv,
                    buffer_ptr(first()) as *mut libc::c_char,
                    4 * (buf_size() - first()),
                    BYTE_BUFFER as *const libc::c_char,
                    bytes_read as i32,
                    &mut error_code,
                );
                if error_code != 0 {
                    conversion_error(error_code);
                    return 0;
                }
                set_last(first() + out_len / 4);
            }
        }

        // Reuse `i` for the CR-tracking check at the end.
        if i == b'\r' as i32 {
            (*f).skip_next_lf = 1;
        }
    } else {
        // Non-ICU path: get_uni_c reads UTF-8/16/RAW.
        let mut i = get_uni_c(f);
        if (*f).skip_next_lf != 0 {
            (*f).skip_next_lf = 0;
            if i == b'\n' as i32 {
                i = get_uni_c(f);
            }
        }

        match norm {
            1 | 2 => {
                // NFC or NFD: read into utf32Buf then normalize.
                if UTF32_BUF.is_null() {
                    UTF32_BUF =
                        xcalloc(buf_size() as libc::size_t, std::mem::size_of::<u32>()) as *mut u32;
                }
                let mut tmp_len: i32 = 0;
                if i != libc::EOF && i != b'\n' as i32 && i != b'\r' as i32 {
                    *UTF32_BUF.add(tmp_len as usize) = i as u32;
                    tmp_len += 1;
                    while tmp_len < buf_size() {
                        i = get_uni_c(f);
                        if i == libc::EOF || i == b'\n' as i32 || i == b'\r' as i32 {
                            break;
                        }
                        *UTF32_BUF.add(tmp_len as usize) = i as u32;
                        tmp_len += 1;
                    }
                }

                if i == libc::EOF && *libc::__errno_location() != libc::EINTR && tmp_len == 0 {
                    return 0;
                }
                if i != libc::EOF && i != b'\n' as i32 && i != b'\r' as i32 {
                    buffer_overflow();
                }
                apply_normalization(UTF32_BUF, tmp_len, norm);
            }
            _ => {
                // No normalization: store directly into buffer[last..].
                if last() < buf_size() && i != libc::EOF && i != b'\n' as i32 && i != b'\r' as i32 {
                    set_buffer_entry(last(), i);
                    set_last(last() + 1);
                    while last() < buf_size() {
                        i = get_uni_c(f);
                        if i == libc::EOF || i == b'\n' as i32 || i == b'\r' as i32 {
                            break;
                        }
                        set_buffer_entry(last(), i);
                        set_last(last() + 1);
                    }
                }

                if i == libc::EOF && *libc::__errno_location() != libc::EINTR && last() == first() {
                    return 0;
                }
                if i != libc::EOF && i != b'\n' as i32 && i != b'\r' as i32 {
                    buffer_overflow();
                }
            }
        }

        if i == b'\r' as i32 {
            (*f).skip_next_lf = 1;
        }
    }

    set_buffer_entry(last(), b' ' as i32);
    if last() >= max_buf_stack() {
        set_max_buf_stack(last());
    }

    // Trim trailing spaces and EOL characters.
    loop {
        if last() <= first() {
            break;
        }
        let c = buffer_entry(last() - 1);
        if c != b' ' as i32 && c != b'\r' as i32 && c != b'\n' as i32 {
            break;
        }
        set_last(last() - 1);
    }

    1 // true
}

// ---------------------------------------------------------------------------
// make_utf16_name
// ---------------------------------------------------------------------------

/// Convert `name_of_file` (UTF-8) to UTF-16 in `name_of_file16`.
///
/// translated from xetex-io.c:481 (make_utf16_name)
#[no_mangle]
pub unsafe extern "C" fn make_utf16_name() {
    crate::state::with_state_mut(|state| {
        let name_len = state.input.name_length as usize;
        let needed = name_len + 10;
        if state.input.name_of_file16.len() < needed {
            state.input.name_of_file16.resize(needed, 0);
        }
    });
    let mut t = name_of_file16();
    let end = (name_of_file() as *const u8).add(name_length() as usize);
    let mut s = name_of_file() as *const u8;

    while s < end {
        let mut rval = *s as u32;
        s = s.add(1);
        let extra_bytes = raw::bytesFromUTF8[rval as usize] as usize;
        // Translate C's falling-through switch (same pattern as maketexstring).
        for _ in 0..extra_bytes {
            if *s != 0 {
                rval = (rval << 6) + *s as u32;
                s = s.add(1);
            } else {
                rval <<= 6;
            }
        }
        rval -= raw::offsetsFromUTF8[extra_bytes] as u32;
        if rval > 0xffff {
            rval -= 0x10000;
            *t = (0xd800 + rval / 0x0400) as u16;
            t = t.add(1);
            *t = (0xdc00 + rval % 0x0400) as u16;
            t = t.add(1);
        } else {
            *t = rval as u16;
            t = t.add(1);
        }
    }
    set_name_length16(t.offset_from(name_of_file16()) as i32);
}

// ---------------------------------------------------------------------------
// get_uni_c
// ---------------------------------------------------------------------------

/// Read one Unicode codepoint from a UFILE (handles UTF-8, UTF-16BE/LE, RAW).
///
/// translated from xetex-io.c:373 (get_uni_c)
#[no_mangle]
pub unsafe extern "C" fn get_uni_c(f: *mut UFile) -> libc::c_int {
    if (*f).saved_char != -1 {
        let rval = (*f).saved_char as i32;
        (*f).saved_char = -1;
        return rval;
    }

    let mut rval: i32;

    match (*f).encoding_mode as i32 {
        UTF8 => {
            let mut c = ttstub_input_getc((*f).handle);
            rval = c;
            if rval != libc::EOF {
                let extra_bytes = raw::bytesFromUTF8[rval as usize] as usize;
                if extra_bytes >= 4 {
                    // case 5 and 4 in C: invalid leading bytes
                    bad_utf8_warning();
                    return 0xFFFD;
                }
                // cases 3, 2, 1, 0: read `extra_bytes` continuation bytes.
                // Translates C's falling-through switch (goto bad_utf8 on error).
                for _ in 0..extra_bytes {
                    c = ttstub_input_getc((*f).handle);
                    if !(0x80..0xC0).contains(&c) {
                        if c != libc::EOF {
                            ttstub_input_ungetc((*f).handle, c);
                        }
                        bad_utf8_warning();
                        return 0xFFFD;
                    }
                    rval = (rval << 6) + c;
                }
                // Subtract offset and validate range.
                let offset = raw::offsetsFromUTF8[extra_bytes];
                rval = (rval as u32).wrapping_sub(offset) as i32;
                if !(0..=0x10ffff).contains(&rval) {
                    bad_utf8_warning();
                    return 0xfffd;
                }
            }
        }

        UTF16BE => {
            rval = ttstub_input_getc((*f).handle);
            if rval != libc::EOF {
                rval <<= 8;
                rval += ttstub_input_getc((*f).handle);
                if (0xd800..=0xdbff).contains(&rval) {
                    let mut lo = ttstub_input_getc((*f).handle);
                    lo <<= 8;
                    lo += ttstub_input_getc((*f).handle);
                    if (0xdc00..=0xdfff).contains(&lo) {
                        rval = 0x10000 + (rval - 0xd800) * 0x400 + (lo - 0xdc00);
                    } else {
                        rval = 0xfffd;
                        (*f).saved_char = lo as i64;
                    }
                } else if (0xdc00..=0xdfff).contains(&rval) {
                    rval = 0xfffd;
                }
            }
        }

        UTF16LE => {
            rval = ttstub_input_getc((*f).handle);
            if rval != libc::EOF {
                rval += ttstub_input_getc((*f).handle) << 8;
                if (0xd800..=0xdbff).contains(&rval) {
                    let mut lo = ttstub_input_getc((*f).handle);
                    lo += ttstub_input_getc((*f).handle) << 8;
                    if (0xdc00..=0xdfff).contains(&lo) {
                        rval = 0x10000 + (rval - 0xd800) * 0x400 + (lo - 0xdc00);
                    } else {
                        rval = 0xfffd;
                        (*f).saved_char = lo as i64;
                    }
                } else if (0xdc00..=0xdfff).contains(&rval) {
                    rval = 0xfffd;
                }
            }
        }

        RAW => {
            rval = ttstub_input_getc((*f).handle);
        }

        _ => {
            _tt_abort(c"internal error; file input mode is invalid".as_ptr());
        }
    }

    rval
}

// ---------------------------------------------------------------------------
// u_close
// ---------------------------------------------------------------------------

// translated from xetex-io.c:273 (u_close)
pub unsafe fn u_close(f: *mut UFile) {
    const ICUMAPPING: i16 = 5; // xetex-xetexd.h:1215

    if f.is_null() || (*f).handle == INVALID_HANDLE {
        return;
    }

    ttstub_input_close((*f).handle);

    if (*f).encoding_mode == ICUMAPPING && !(*f).conversion_data.is_null() {
        xetex_ucnv_close((*f).conversion_data);
    }

    libc::free(f as *mut libc::c_void);
}

// ---------------------------------------------------------------------------
// open_or_close_in
// ---------------------------------------------------------------------------

/// Open or close a \read stream.
///
/// translated from xetex-io.c:294 (open_or_close_in)
#[no_mangle]
pub unsafe extern "C" fn open_or_close_in() {
    let c = cur_chr() as u8;
    scan_four_bit_int();
    let n = cur_val() as u8 as usize;

    if read_open_entry(n) != CLOSED {
        u_close(read_file_entry(n));
        set_read_open_entry(n, CLOSED);
    }

    if c != 0 {
        scan_optional_equals();
        scan_file_name();
        pack_file_name(cur_name(), cur_area(), cur_ext());

        if u_open_in(
            read_file_ptr(n),
            TTBC_FILE_FORMAT_TEX,
            c"rb".as_ptr(),
            eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_MODE),
            eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_ENCODING),
        ) != 0
        {
            make_utf16_name();
            set_name_in_progress(true);
            begin_name();
            set_stop_at_space(false);
            let mut k = 0i32;
            while k < name_length16() && more_name(name_of_file16_entry(k)) {
                k += 1;
            }
            set_stop_at_space(true);
            end_name();
            set_name_in_progress(false);
            set_read_open_entry(n, JUST_OPEN);
        }
    }
}
