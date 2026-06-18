//! Rust translation of xetex-io.c: low-level input/output helpers.

use crate::consts::{BIGGEST_CHAR, INT_BASE, SELECTOR_TERM_ONLY, TOO_BIG_CHAR};
use crate::globals::{
    buf_size, buffer_entry, cur_area, cur_chr, cur_ext, cur_name, cur_val, eqtb_b32s1, first, last,
    name_length16, name_of_file, name_of_file16_entry, read_file_entry, read_file_ptr,
    read_open_entry, set_buffer_entry, set_input_file, set_last, set_name_in_progress,
    set_name_length, set_read_open_entry, set_stop_at_space,
};
use crate::globals::{
    cur_cmd, cur_cs, cur_input, def_ref, file_offset, init_pool_ptr, max_print_line, mem_b32s1,
    old_setting, open_parens, pool_ptr, pool_size, rust_stdout, scanner_status, selector,
    set_cur_cs, set_cur_input_limit, set_cur_input_loc, set_cur_input_name, set_cur_input_state,
    set_def_ref, set_eqtb_b32s1, set_first, set_line, set_old_setting, set_open_parens,
    set_pool_ptr, set_scanner_status, set_selector, set_str_pool, set_str_ptr, set_warning_index,
    stop_at_space, str_pool_entry, str_ptr, str_start, term_offset, texmf_log_name, warning_index,
    write_open,
};
use crate::input::InputAbort;

// ---------------------------------------------------------------------------
// UFILE — Rust mirror of the C `UFILE` struct (xetex-io.h:12–18)
// ---------------------------------------------------------------------------

/// Rust mirror of `UFILE` (xetex-io.h:12–18). Must stay layout-compatible.
///
/// On Linux x86_64 the C layout is:
use crate::output::begin_diagnostic;
use crate::output::capture_to_diagnostic;
use crate::output::diagnostic_begin_capture_warning_here;
use crate::output::end_diagnostic;
use crate::output::print_cstr;
use crate::output::print_int;
use crate::output::print_nl;
use crate::output::print_nl_cstr;
use crate::scan::get_x_token;
use crate::scan::scan_four_bit_int;
use crate::scan::scan_optional_equals;
use crate::scan::scan_toks;
use crate::shipout::finalize_dvi_file;
use crate::texmfmp::gettexstring;

// Consts used by the file-naming / start_input / close_files group, moved with
// it from scan.rs (workstream 02).
const EMPTY_STRING: i32 = TOO_BIG_CHAR + 1;
const INT_PAR_END_LINE_CHAR: i32 = 48;
const INT_PAR_NEW_LINE_CHAR_CFT: i32 = 49;
const INT_PAR_TRACING_STACK_LEVELS: i32 = 55;
const LEFT_BRACE: u8 = 1;
const NEW_LINE: u16 = 33;
const OTHER_CHAR: u8 = 12;
const RELAX: u8 = 0;
const SELECTOR_LOG_ONLY: i32 = 18;
const SELECTOR_NEW_STRING: i32 = 21;
const SPACER: u8 = 10;
const TTBC_FILE_FORMAT_TECTONIC_PRIMARY_SI: i32 = 59;
const TTBC_FILE_FORMAT_TEX_SI: i32 = 26;

// UTF-8 decode tables used by start_input (moved with it from scan.rs, ws02).
mod raw_utf8 {
    extern "C" {
        pub(super) static bytesFromUTF8: [u8; 256];
        pub(super) static offsetsFromUTF8: [u32; 6];
    }
}

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
    fn ttstub_output_open(path: *const libc::c_char, gzip: libc::c_int) -> usize;
    fn ttstub_output_flush(handle: usize) -> libc::c_int;
    fn ttstub_output_close(handle: usize) -> libc::c_int;
    fn ttstub_output_putc(handle: usize, c: i32) -> i32;

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

    crate::state::with_state_mut(|state| {
        if ttstub_get_last_input_abspath(
            state.input.abspath_of_input_file.0.as_mut_ptr(),
            state.input.abspath_of_input_file.0.len(),
        ) < 1
        {
            state.input.abspath_of_input_file.0[0] = 0;
        }
    });

    let nof = name_of_file();
    set_name_length(libc::strlen(nof) as i32);
    crate::state::with_state_mut(|state| {
        libc::free(state.input.name_of_input_file as *mut libc::c_void);
        state.input.name_of_input_file = xstrdup(nof);
    });
    handle
}

// ---------------------------------------------------------------------------
// u_open_in
// ---------------------------------------------------------------------------

/// Open a UFILE for reading with BOM-sniffing encoding detection.
///
/// translated from xetex-io.c:127 (u_open_in)
pub unsafe fn u_open_in(
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
pub unsafe fn set_input_file_encoding(f: *mut UFile, mode: i32, encoding_data: i32) {
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
// input_line private helpers: conversion_error, apply_normalization
// ---------------------------------------------------------------------------

// Persistent state for input_line, replacing C file-statics.
// `BYTE_BUFFER`: byte staging area for ICU decoding (xetex-io.c:138).
static mut BYTE_BUFFER: *mut libc::c_char = std::ptr::null_mut();
// `UTF32_BUF`: UTF-32 staging area for normalization (xetex-io.c:139).
static mut UTF32_BUF: *mut u32 = std::ptr::null_mut();
// TECkit normalizer converters; replaces `static TECkit_Converter normalizers[2]` (xetex-io.c:113).
static mut NORMALIZERS: [*mut libc::c_void; 2] = [std::ptr::null_mut(), std::ptr::null_mut()];

// translated from xetex-io.c:98 (conversion_error, was static)
// Threaded `_in` core (`01b` §3); prints through the threaded diagnostic cores
// under the live `&mut EngineState` borrow.
fn conversion_error_in(st: &mut crate::state::EngineState, errcode: i32) {
    crate::output::begin_diagnostic_in(st);
    crate::output::print_nl_in(st, b'U' as i32);
    crate::output::print_cstr_in(st, c"nicode conversion failed (ICU error code = ".as_ptr());
    crate::output::print_int_in(st, errcode);
    crate::output::print_cstr_in(st, c") discarding any remaining text".as_ptr());
    crate::output::end_diagnostic_in(st, true);
}

// translated from xetex-io.c:110 (apply_normalization, was static)
// Sets `input.last` on return; called from `input_line_in`. The cold
// converter-creation / buffer-overflow `_tt_abort`s ride the `InputAbort`
// sentinel (`01b` §5).
unsafe fn apply_normalization_in(
    st: &mut crate::state::EngineState,
    buf: *mut u32,
    len: i32,
    norm: i32,
) -> Result<(), InputAbort> {
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
            return Err(InputAbort::TtAbort(c"failed to create normalizer".as_ptr()));
        }
    }

    let mut in_used: u32 = 0;
    let mut out_used: u32 = 0;
    let out_ptr = st.input.buffer.as_mut_ptr().add(first() as usize) as *mut u8;
    let out_cap = (4 * (buf_size() - first())) as u32;
    let status = TECkit_ConvertBuffer(
        *norm_ptr,
        buf as *const u8,
        (len * 4) as u32,
        &mut in_used,
        out_ptr,
        out_cap,
        &mut out_used,
        1,
    );
    TECkit_ResetConverter(*norm_ptr);
    if status != K_STATUS_NO_ERROR {
        return Err(InputAbort::TtAbort(
            c"unable to read an entire line (buffer too small)".as_ptr(),
        ));
    }
    st.input.last = first() + (out_used / 4) as i32;
    Ok(())
}

// ---------------------------------------------------------------------------
// input_line
// ---------------------------------------------------------------------------

/// Read one line from UFILE into the TeX `buffer`, handling encodings.
///
/// translated from xetex-io.c:135 (input_line)
pub unsafe fn input_line(f: *mut UFile) -> libc::c_int {
    match crate::state::with_state_mut(|st| input_line_in(st, f)) {
        Ok(v) => v,
        Err(a) => a.fire(),
    }
}

/// Threaded `_in` core of [`input_line`] (`01b` §3). Holds one `&mut
/// EngineState` borrow across the read, touching `st.input` (buffer/`last`/
/// `max_buf_stack`) and `st.eqtb` (input normalization) directly and calling
/// the threaded `get_uni_c_in`/`apply_normalization_in`/`conversion_error_in`
/// cores under it. Cold `_tt_abort`s ride the [`InputAbort`] sentinel.
pub(crate) unsafe fn input_line_in(
    st: &mut crate::state::EngineState,
    f: *mut UFile,
) -> Result<libc::c_int, InputAbort> {
    // INT_PAR_XETEX_INPUT_NORMALIZATION (xetex_format.h:109), cf. scan.rs.
    const INT_PAR_XETEX_INPUT_NORMALIZATION: i32 = 74;
    let norm = if st.eqtb.data.is_empty() {
        0
    } else {
        st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_INPUT_NORMALIZATION)
    };

    if (*f).handle == INVALID_HANDLE {
        return Err(InputAbort::TtAbort(
            c"reads from synthetic \"terminal\" file #0 should never happen".as_ptr(),
        ));
    }

    st.input.last = first();

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
            return Ok(0); // false
        }

        if i != libc::EOF && i != b'\n' as i32 && i != b'\r' as i32 {
            return Err(InputAbort::TtAbort(
                c"unable to read an entire line (buffer too small)".as_ptr(),
            ));
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
                    conversion_error_in(st, error_code);
                    return Ok(0);
                }
                // tmp_len is bytes; divide by 4 to get u32 count.
                apply_normalization_in(st, UTF32_BUF, tmp_len / 4, norm)?;
            }
            _ => {
                // No normalization: convert directly into buffer[first..].
                let out_ptr = st.input.buffer.as_mut_ptr().add(first() as usize);
                let out_len = xetex_ucnv_to_utf32(
                    cnv,
                    out_ptr as *mut libc::c_char,
                    4 * (buf_size() - first()),
                    BYTE_BUFFER as *const libc::c_char,
                    bytes_read as i32,
                    &mut error_code,
                );
                if error_code != 0 {
                    conversion_error_in(st, error_code);
                    return Ok(0);
                }
                st.input.last = first() + out_len / 4;
            }
        }

        // Reuse `i` for the CR-tracking check at the end.
        if i == b'\r' as i32 {
            (*f).skip_next_lf = 1;
        }
    } else {
        // Non-ICU path: get_uni_c reads UTF-8/16/RAW.
        let mut i = get_uni_c_in(st, f)?;
        if (*f).skip_next_lf != 0 {
            (*f).skip_next_lf = 0;
            if i == b'\n' as i32 {
                i = get_uni_c_in(st, f)?;
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
                        i = get_uni_c_in(st, f)?;
                        if i == libc::EOF || i == b'\n' as i32 || i == b'\r' as i32 {
                            break;
                        }
                        *UTF32_BUF.add(tmp_len as usize) = i as u32;
                        tmp_len += 1;
                    }
                }

                if i == libc::EOF && *libc::__errno_location() != libc::EINTR && tmp_len == 0 {
                    return Ok(0);
                }
                if i != libc::EOF && i != b'\n' as i32 && i != b'\r' as i32 {
                    return Err(InputAbort::TtAbort(
                        c"unable to read an entire line (buffer too small)".as_ptr(),
                    ));
                }
                apply_normalization_in(st, UTF32_BUF, tmp_len, norm)?;
            }
            _ => {
                // No normalization: store directly into buffer[last..].
                if st.input.last < buf_size()
                    && i != libc::EOF
                    && i != b'\n' as i32
                    && i != b'\r' as i32
                {
                    st.input.buffer[st.input.last as usize] = i;
                    st.input.last += 1;
                    while st.input.last < buf_size() {
                        i = get_uni_c_in(st, f)?;
                        if i == libc::EOF || i == b'\n' as i32 || i == b'\r' as i32 {
                            break;
                        }
                        st.input.buffer[st.input.last as usize] = i;
                        st.input.last += 1;
                    }
                }

                if i == libc::EOF
                    && *libc::__errno_location() != libc::EINTR
                    && st.input.last == first()
                {
                    return Ok(0);
                }
                if i != libc::EOF && i != b'\n' as i32 && i != b'\r' as i32 {
                    return Err(InputAbort::TtAbort(
                        c"unable to read an entire line (buffer too small)".as_ptr(),
                    ));
                }
            }
        }

        if i == b'\r' as i32 {
            (*f).skip_next_lf = 1;
        }
    }

    st.input.buffer[st.input.last as usize] = b' ' as i32;
    if st.input.last >= st.input.max_buf_stack {
        st.input.max_buf_stack = st.input.last;
    }

    // Trim trailing spaces and EOL characters.
    loop {
        if st.input.last <= first() {
            break;
        }
        let c = st.input.buffer[(st.input.last - 1) as usize];
        if c != b' ' as i32 && c != b'\r' as i32 && c != b'\n' as i32 {
            break;
        }
        st.input.last -= 1;
    }

    Ok(1) // true
}

// ---------------------------------------------------------------------------
// make_utf16_name
// ---------------------------------------------------------------------------

/// Convert `name_of_file` (UTF-8) to UTF-16 in `name_of_file16`.
///
/// translated from xetex-io.c:481 (make_utf16_name)
/// `&mut EngineState` core of [`make_utf16_name`] (`01b` §3). Touches only the
/// `input` cluster (name buffers); no cold path. Safe to call under a live
/// borrow.
pub(crate) unsafe fn make_utf16_name_in(st: &mut crate::state::EngineState) {
    let name_len = st.input.name_length as usize;
    let needed = name_len + 10;
    if st.input.name_of_file16.len() < needed {
        st.input.name_of_file16.resize(needed, 0);
    }
    let base = st.input.name_of_file.as_ptr();
    let mut t = st.input.name_of_file16.as_mut_ptr();
    let t0 = t as *const u16;
    let end = (base as *const u8).add(name_len);
    let mut s = base as *const u8;

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
    st.input.name_length16 = t.offset_from(t0) as i32;
}

/// Bridge-down wrapper for legacy `make_utf16_name` callers (`01b` §3).
pub unsafe fn make_utf16_name() {
    crate::state::with_state_mut(|st| make_utf16_name_in(st));
}

// ---------------------------------------------------------------------------
// get_uni_c
// ---------------------------------------------------------------------------

/// Read one Unicode codepoint from a UFILE (handles UTF-8, UTF-16BE/LE, RAW).
///
/// translated from xetex-io.c:373 (get_uni_c)
// Threaded `_in` core (`01b` §3); only re-enters state through the threaded
// `bad_utf8_warning_in`. The impossible invalid-mode `_tt_abort` rides the
// `InputAbort` sentinel. Called only from `input_line_in`.
pub(crate) unsafe fn get_uni_c_in(
    st: &mut crate::state::EngineState,
    f: *mut UFile,
) -> Result<libc::c_int, InputAbort> {
    if (*f).saved_char != -1 {
        let rval = (*f).saved_char as i32;
        (*f).saved_char = -1;
        return Ok(rval);
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
                    crate::output::bad_utf8_warning_in(st);
                    return Ok(0xFFFD);
                }
                // cases 3, 2, 1, 0: read `extra_bytes` continuation bytes.
                // Translates C's falling-through switch (goto bad_utf8 on error).
                for _ in 0..extra_bytes {
                    c = ttstub_input_getc((*f).handle);
                    if !(0x80..0xC0).contains(&c) {
                        if c != libc::EOF {
                            ttstub_input_ungetc((*f).handle, c);
                        }
                        crate::output::bad_utf8_warning_in(st);
                        return Ok(0xFFFD);
                    }
                    rval = (rval << 6) + c;
                }
                // Subtract offset and validate range.
                let offset = raw::offsetsFromUTF8[extra_bytes];
                rval = (rval as u32).wrapping_sub(offset) as i32;
                if !(0..=0x10ffff).contains(&rval) {
                    crate::output::bad_utf8_warning_in(st);
                    return Ok(0xfffd);
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
            return Err(InputAbort::TtAbort(
                c"internal error; file input mode is invalid".as_ptr(),
            ));
        }
    }

    Ok(rval)
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
pub unsafe fn open_or_close_in() {
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

// ---- moved verbatim from scan.rs (workstream 02) ----

/// translated from xetex-xetex0.c:7827 (begin_name)
/// `&mut EngineState` core of [`begin_name`] (`01b` §3). Touches `scanner` only.
pub(crate) fn begin_name_in(st: &mut crate::state::EngineState) {
    st.scanner.area_delimiter = 0;
    st.scanner.ext_delimiter = 0;
    st.scanner.quoted_filename = false;
    st.scanner.file_name_quote_char = 0;
}

/// Bridge-down wrapper for legacy `begin_name` callers (`01b` §3).
pub fn begin_name() {
    crate::state::with_state_mut(|st| begin_name_in(st));
}
/// Cold abort reasons threaded out of the file-name machinery
/// ([`more_name_in`]/[`end_name_in`]/[`make_name_string_in`]) so the bridge
/// wrappers run the `overflow` longjmp after the state borrow drops (`01b` §5).
pub(crate) enum NameAbort {
    /// `pool_ptr + 1 > pool_size`: `overflow("pool size", …)`.
    PoolSize,
    /// `str_ptr + 3 > max_strings` (or `make_string`): `overflow("number of strings", …)`.
    NumberOfStrings,
}

/// `&mut EngineState` core of [`more_name`] (`01b` §3). Touches `scanner` +
/// `strings`; the cold pool-size `overflow` defers via [`NameAbort`].
pub(crate) fn more_name_in(st: &mut crate::state::EngineState, c: u16) -> Result<bool, NameAbort> {
    let stop = st.scanner.stop_at_space;
    let q = st.scanner.file_name_quote_char;
    if stop && q == 0 && c == b' ' as u16 {
        return Ok(false);
    }
    if stop && q != 0 && c == q {
        st.scanner.file_name_quote_char = 0;
        return Ok(true);
    }
    if stop && q == 0 && (c == b'"' as u16 || c == b'\'' as u16) {
        st.scanner.file_name_quote_char = c;
        st.scanner.quoted_filename = true;
        return Ok(true);
    }
    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(NameAbort::PoolSize);
    }
    st.strings.str_pool[st.strings.pool_ptr as usize] = c;
    st.strings.pool_ptr += 1;
    // cur_length = pool_ptr - str_start[str_ptr - TOO_BIG_CHAR]
    let cur_len =
        st.strings.pool_ptr - st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
    if c == b'/' as u16 {
        // IS_DIR_SEP
        st.scanner.area_delimiter = cur_len;
        st.scanner.ext_delimiter = 0;
    } else if c == b'.' as u16 {
        st.scanner.ext_delimiter = cur_len;
    }
    Ok(true)
}

/// Bridge-down wrapper for legacy `more_name` callers (`01b` §3, §5).
/// translated from xetex-xetex0.c:7837 (more_name)
pub unsafe fn more_name(c: u16) -> bool {
    match crate::state::with_state_mut(|st| more_name_in(st, c)) {
        Ok(v) => v,
        Err(e) => fire_name_overflow(e),
    }
}

/// Run a deferred file-name [`NameAbort`] overflow after the borrow drops.
/// Never returns.
pub(crate) unsafe fn fire_name_overflow(e: NameAbort) -> ! {
    match e {
        NameAbort::PoolSize => {
            crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr())
        }
        NameAbort::NumberOfStrings => crate::errors::overflow(
            c"number of strings".as_ptr(),
            crate::globals::max_strings() - crate::globals::init_str_ptr(),
        ),
    }
}
/// `&mut EngineState` core of [`end_name`] (`01b` §3). Touches `strings` +
/// `scanner`; the cold `overflow("number of strings")` (direct or via
/// `make_string`/`slow_make_string`) defers via [`NameAbort`].
pub(crate) fn end_name_in(st: &mut crate::state::EngineState) -> Result<(), NameAbort> {
    use crate::string_pool::{make_string, search_string, slow_make_string};

    if st.strings.str_ptr + 3 > st.strings.max_strings {
        return Err(NameAbort::NumberOfStrings);
    }

    if st.scanner.area_delimiter == 0 {
        st.scanner.cur_area = EMPTY_STRING;
    } else {
        st.scanner.cur_area = st.strings.str_ptr;
        // str_start[str_ptr + 1 - TOO_BIG_CHAR] = str_start[str_ptr - TOO_BIG_CHAR] + area_delimiter
        let new_start = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize]
            + st.scanner.area_delimiter;
        st.strings.str_start[(st.strings.str_ptr + 1 - TOO_BIG_CHAR) as usize] = new_start;
        st.strings.str_ptr += 1;
        let temp_str = search_string(&st.strings, st.scanner.cur_area);
        if temp_str > 0 {
            st.scanner.cur_area = temp_str;
            st.strings.str_ptr -= 1;
            // shift pool: str_pool[j - area_delimiter] = str_pool[j] for j in start..pool_ptr
            let base = st.strings.str_start[(st.strings.str_ptr + 1 - TOO_BIG_CHAR) as usize];
            let ad = st.scanner.area_delimiter;
            let pp = st.strings.pool_ptr;
            for j in base..pp {
                let v = st.strings.str_pool[j as usize];
                st.strings.str_pool[(j - ad) as usize] = v;
            }
            st.strings.pool_ptr = pp - ad;
        }
    }

    if st.scanner.ext_delimiter == 0 {
        st.scanner.cur_ext = EMPTY_STRING;
        st.scanner.cur_name =
            slow_make_string(&mut st.strings).map_err(|_| NameAbort::NumberOfStrings)?;
    } else {
        st.scanner.cur_name = st.strings.str_ptr;
        // str_start[str_ptr+1 - TOO_BIG_CHAR] = str_start[str_ptr - TOO_BIG_CHAR]
        //   + ext_delimiter - area_delimiter - 1
        let new_start = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize]
            + st.scanner.ext_delimiter
            - st.scanner.area_delimiter
            - 1;
        st.strings.str_start[(st.strings.str_ptr + 1 - TOO_BIG_CHAR) as usize] = new_start;
        st.strings.str_ptr += 1;

        st.scanner.cur_ext =
            make_string(&mut st.strings).map_err(|_| NameAbort::NumberOfStrings)?;
        st.strings.str_ptr -= 1;
        let temp_str = search_string(&st.strings, st.scanner.cur_name);
        if temp_str > 0 {
            st.scanner.cur_name = temp_str;
            st.strings.str_ptr -= 1;
            // shift pool: str_pool[j - ext_delimiter + area_delimiter + 1] = str_pool[j]
            let base = st.strings.str_start[(st.strings.str_ptr + 1 - TOO_BIG_CHAR) as usize];
            let ed = st.scanner.ext_delimiter;
            let ad = st.scanner.area_delimiter;
            let pp = st.strings.pool_ptr;
            let shift = ed - ad - 1;
            for j in base..pp {
                let v = st.strings.str_pool[j as usize];
                st.strings.str_pool[(j - shift) as usize] = v;
            }
            st.strings.pool_ptr = pp - shift;
        }
        st.scanner.cur_ext =
            slow_make_string(&mut st.strings).map_err(|_| NameAbort::NumberOfStrings)?;
    }
    Ok(())
}

/// Bridge-down wrapper for legacy `end_name` callers (`01b` §3, §5).
/// translated from xetex-xetex0.c:7870 (end_name)
pub unsafe fn end_name() {
    if let Err(e) = crate::state::with_state_mut(|st| end_name_in(st)) {
        fire_name_overflow(e);
    }
}
/// `&mut EngineState` core of [`make_name_string`] (`01b` §3). Touches
/// `strings`/`scanner`/`input`; cold `overflow` defers via [`NameAbort`].
pub(crate) unsafe fn make_name_string_in(
    st: &mut crate::state::EngineState,
) -> Result<i32, NameAbort> {
    let cur_length =
        st.strings.pool_ptr - st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
    if st.strings.pool_ptr + st.input.name_length > st.strings.pool_size
        || st.strings.str_ptr == st.strings.max_strings
        || cur_length > 0
    {
        return Ok(b'?' as i32);
    }

    make_utf16_name_in(st);

    for k in 0..st.input.name_length16 {
        st.strings.str_pool[st.strings.pool_ptr as usize] = st.input.name_of_file16[k as usize];
        st.strings.pool_ptr += 1;
    }

    let result =
        crate::string_pool::make_string(&mut st.strings).map_err(|_| NameAbort::NumberOfStrings)?;
    let save_area_delimiter = st.scanner.area_delimiter;
    let save_ext_delimiter = st.scanner.ext_delimiter;
    let save_name_in_progress = st.scanner.name_in_progress;
    let save_stop_at_space = st.scanner.stop_at_space;

    st.scanner.name_in_progress = true;
    begin_name_in(st);
    st.scanner.stop_at_space = false;

    let mut k = 0;
    while k < st.input.name_length16 {
        let c = st.input.name_of_file16[k as usize];
        if !more_name_in(st, c)? {
            break;
        }
        k += 1;
    }

    st.scanner.stop_at_space = save_stop_at_space;
    end_name_in(st)?;
    st.scanner.name_in_progress = save_name_in_progress;
    st.scanner.area_delimiter = save_area_delimiter;
    st.scanner.ext_delimiter = save_ext_delimiter;
    Ok(result)
}

/// Bridge-down wrapper for legacy `make_name_string` callers (`01b` §3, §5).
/// translated from xetex-xetex0.c:7928 (make_name_string)
pub unsafe fn make_name_string() -> i32 {
    match crate::state::with_state_mut(|st| make_name_string_in(st)) {
        Ok(v) => v,
        Err(e) => fire_name_overflow(e),
    }
}
/// translated from xetex-xetex0.c:13424 (scan_and_pack_name)
pub unsafe fn scan_and_pack_name() {
    scan_file_name();
    pack_file_name(
        crate::globals::cur_name(),
        crate::globals::cur_area(),
        crate::globals::cur_ext(),
    );
}
unsafe fn scan_file_name_braced() {
    let save_scanner_status = scanner_status();
    let save_def_ref = def_ref();
    let save_cur_cs = cur_cs();
    let save_stop_at_space = stop_at_space();

    set_cur_cs(warning_index());
    scan_toks(false, true);

    set_old_setting(selector() as u8);
    set_selector(SELECTOR_NEW_STRING);
    crate::output::show_token_list(
        mem_b32s1(def_ref()),
        crate::mem::TEX_NULL,
        pool_size() - pool_ptr(),
    );
    set_selector(old_setting() as i32);

    let s = crate::string_pool::make_string_global();
    crate::mem::delete_token_ref_global(def_ref());
    set_def_ref(save_def_ref);
    set_cur_cs(save_cur_cs);
    set_scanner_status(save_scanner_status);

    begin_name();
    let start = str_start(s - TOO_BIG_CHAR);
    let end = str_start(s + 1 - TOO_BIG_CHAR);
    for i in start..end {
        more_name(str_pool_entry(i));
    }
    set_stop_at_space(save_stop_at_space);
}
/// translated from xetex-xetex0.c:8003 (scan_file_name)
pub unsafe fn scan_file_name() {
    let save_warning_index = warning_index();
    set_warning_index(cur_cs());

    loop {
        get_x_token();
        if cur_cmd() != SPACER && cur_cmd() != RELAX {
            break;
        }
    }

    crate::input::back_input();

    if cur_cmd() == LEFT_BRACE {
        scan_file_name_braced();
    } else {
        set_name_in_progress(true);
        begin_name();

        loop {
            get_x_token();
            if cur_cmd() != SPACER {
                break;
            }
        }

        loop {
            if cur_cmd() > OTHER_CHAR || cur_chr() > BIGGEST_CHAR {
                crate::input::back_input();
                break;
            }

            if !more_name(cur_chr() as u16) {
                break;
            }

            get_x_token();
        }
    }

    end_name();
    set_name_in_progress(false);
    set_warning_index(save_warning_index);
}
/// translated from xetex-xetex0.c:7970 (pack_job_name)
pub unsafe fn pack_job_name(s: *const libc::c_char) {
    use crate::globals::{job_name, set_cur_area, set_cur_ext, set_cur_name};

    set_cur_area(EMPTY_STRING);
    set_cur_ext(crate::texmfmp::maketexstring(s));
    set_cur_name(job_name());
    pack_file_name(
        crate::globals::cur_name(),
        crate::globals::cur_area(),
        crate::globals::cur_ext(),
    );
}
/// translated from xetex-xetex0.c:7937 (open_log_file)
pub unsafe fn open_log_file() {
    use crate::globals::{
        input_ptr, input_stack, job_name, name_of_file, set_input_stack, set_job_name,
        set_log_file, set_log_opened, set_texmf_log_name,
    };

    let old_setting = selector();
    if job_name() == 0 {
        set_job_name(crate::texmfmp::maketexstring(c"texput".as_ptr()));
    }

    pack_job_name(c".log".as_ptr());

    let handle = ttstub_output_open(name_of_file(), 0);
    if handle == INVALID_HANDLE {
        _tt_abort(c"cannot open log file output".as_ptr());
    }
    set_log_file(handle);

    set_texmf_log_name(make_name_string());
    set_selector(SELECTOR_LOG_ONLY);
    set_log_opened(true);

    set_input_stack(input_ptr(), cur_input());

    crate::output::print_nl_cstr(c"**".as_ptr());
    let mut l = input_stack(0).limit;
    if buffer_entry(l) == eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR) {
        l -= 1;
    }

    let mut k = 1;
    while k <= l {
        crate::output::print(buffer_entry(k));
        k += 1;
    }

    crate::output::print_ln();
    set_selector(old_setting + 2);
}
/// translated from xetex-xetex0.c:6431 (start_input)
pub unsafe fn start_input(primary_input_name: *const libc::c_char) {
    use crate::globals::{
        cur_area, cur_ext, cur_name, full_source_filename_stack, in_open, input_file,
        input_file_ptr, input_ptr, job_name, name_length16, name_of_file16_entry,
        set_area_delimiter, set_ext_delimiter, set_full_source_filename_stack, set_job_name,
        set_source_filename_stack,
    };

    let format = if !primary_input_name.is_null() {
        TTBC_FILE_FORMAT_TECTONIC_PRIMARY_SI
    } else {
        TTBC_FILE_FORMAT_TEX_SI
    };

    if !primary_input_name.is_null() {
        set_name_in_progress(true);
        begin_name();
        set_stop_at_space(false);

        let mut cp = primary_input_name as *const u8;
        if pool_ptr() + libc::strlen(primary_input_name) as i32 * 2 >= pool_size() {
            _tt_abort(c"string pool overflow".as_ptr());
        }

        loop {
            let first_byte = *cp;
            cp = cp.add(1);
            if first_byte == 0 {
                break;
            }
            let mut rval = first_byte as u32;
            let extra_bytes = raw_utf8::bytesFromUTF8[rval as usize] as usize;
            for _ in 0..extra_bytes {
                if *cp != 0 {
                    rval = (rval << 6) + *cp as u32;
                    cp = cp.add(1);
                } else {
                    rval <<= 6;
                }
            }
            rval -= raw_utf8::offsetsFromUTF8[extra_bytes];

            if rval > 0xffff {
                rval -= 0x10000;
                let pp = pool_ptr();
                set_str_pool(pp, (0xd800 + rval / 0x0400) as u16);
                set_pool_ptr(pp + 1);
                let pp = pool_ptr();
                set_str_pool(pp, (0xdc00 + rval % 0x0400) as u16);
                set_pool_ptr(pp + 1);
            } else {
                let pp = pool_ptr();
                set_str_pool(pp, rval as u16);
                set_pool_ptr(pp + 1);
            }

            let cur_len = pool_ptr() - str_start(str_ptr() - TOO_BIG_CHAR);
            if rval == b'/' as u32 {
                // IS_DIR_SEP
                set_area_delimiter(cur_len);
                set_ext_delimiter(0);
            } else if rval == b'.' as u32 {
                set_ext_delimiter(cur_len);
            }
        }

        set_stop_at_space(true);
        end_name();
        set_name_in_progress(false);
    } else {
        scan_file_name();
    }

    pack_file_name(cur_name(), cur_area(), cur_ext());
    crate::input::begin_file_reading();

    let idx = cur_input().index as i32;
    if crate::io::u_open_in(
        input_file_ptr(idx),
        format,
        c"rb".as_ptr(),
        eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_MODE),
        eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_ENCODING),
    ) == 0
    {
        // Format "failed to open input file "<name>" into a stack buffer;
        // matches the C: _tt_abort("failed to open input file \"%s\"", name_of_file)
        let mut buf = [0u8; 512];
        libc::snprintf(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            c"failed to open input file \"%s\"".as_ptr(),
            crate::globals::name_of_file(),
        );
        _tt_abort(buf.as_ptr() as *const libc::c_char);
    }

    crate::io::make_utf16_name();
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

    let name = make_name_string();
    set_cur_input_name(name);
    set_source_filename_stack(in_open(), cur_input().name);
    set_full_source_filename_stack(
        in_open(),
        crate::texmfmp::maketexstring(crate::state::with_state(|state| {
            state.input.name_of_input_file
        })),
    );

    if cur_input().name == str_ptr() - 1 {
        let temp_str = crate::string_pool::search_string_global(cur_input().name);
        if temp_str > 0 {
            set_cur_input_name(temp_str);
            set_str_ptr(str_ptr() - 1);
            set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
        }
    }

    if job_name() == 0 {
        set_job_name(cur_name());
        open_log_file();
    }

    let fss = full_source_filename_stack(in_open());
    if term_offset() + crate::string_pool::length_global(fss) > max_print_line() - 2 {
        crate::output::print_ln();
    } else if term_offset() > 0 || file_offset() > 0 {
        crate::output::print_char(b' ' as i32);
    }
    crate::output::print_char(b'(' as i32);
    set_open_parens(open_parens() + 1);
    crate::output::print(fss);
    ttstub_output_flush(rust_stdout());

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS) > 0 {
        crate::output::begin_diagnostic();
        crate::output::diagnostic_begin_capture_warning_here();
        crate::output::print_ln();
        crate::output::print_char(b'~' as i32);
        let mut v = input_ptr() - 1;
        if v < eqtb_b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS) {
            while v > 0 {
                crate::output::print_char(b'.' as i32);
                v -= 1;
            }
        } else {
            crate::output::print_char(b'~' as i32);
        }
        crate::output::print_cstr(c"INPUT ".as_ptr());
        crate::output::print(cur_name());
        crate::output::print(cur_ext());
        crate::output::print_ln();
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::end_diagnostic(false);
    }

    set_cur_input_state(NEW_LINE);
    let synctex_job_name = job_name();
    let synctex_tmp = crate::texmfmp::gettexstring(synctex_job_name);
    crate::state::with_state_mut(|st| crate::synctex::synctex_start_input(st, synctex_tmp));
    set_line(1);
    input_line(input_file(idx));
    set_cur_input_limit(last());

    if eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR) < 0
        || eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR) > 255
    {
        set_cur_input_limit(cur_input().limit - 1);
    } else {
        set_buffer_entry(
            cur_input().limit,
            eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR),
        );
    }
    set_first(cur_input().limit + 1);
    set_cur_input_loc(cur_input().start);
}
// translated from xetex-xetex0.c:6381 (pack_file_name)
pub unsafe fn pack_file_name(n: i32, a: i32, e: i32) {
    extern "C" {
        fn xmalloc(size: libc::size_t) -> *mut libc::c_void;
    }

    let len_a = crate::string_pool::length_global(a) as usize;
    let len_n = crate::string_pool::length_global(n) as usize;
    let len_e = crate::string_pool::length_global(e) as usize;
    // Each TeX character is at most 3 UTF-8 bytes; +2 for NUL terminators.
    let work_size = (len_a + len_n + len_e) * 3 + 2;
    let work_buffer = xmalloc(work_size as libc::size_t) as *mut libc::c_char;
    *work_buffer = 0;

    let a_utf8 = crate::texmfmp::gettexstring(a);
    libc::strcat(work_buffer, a_utf8);
    libc::free(a_utf8 as *mut libc::c_void);

    let n_utf8 = crate::texmfmp::gettexstring(n);
    libc::strcat(work_buffer, n_utf8);
    libc::free(n_utf8 as *mut libc::c_void);

    let e_utf8 = crate::texmfmp::gettexstring(e);
    libc::strcat(work_buffer, e_utf8);
    libc::free(e_utf8 as *mut libc::c_void);

    let nl = libc::strlen(work_buffer) as i32;
    crate::globals::set_name_length(nl);
    crate::globals::set_name_of_file(work_buffer);
    libc::free(work_buffer as *mut libc::c_void);
}
/// translated from xetex-xetex0.c:9722 (close_files_and_terminate)
pub unsafe fn close_files_and_terminate() {
    use crate::globals::{log_file, log_opened, write_file};

    for k in 0usize..=15 {
        if write_open(k) {
            ttstub_output_close(write_file(k));
        }
    }

    set_eqtb_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR_CFT, -1);

    finalize_dvi_file();
    let log_was_opened = log_opened();
    crate::state::with_state_mut(|st| crate::synctex::synctex_terminate(st, log_was_opened));

    if log_opened() {
        ttstub_output_putc(log_file(), b'\n' as i32);
        ttstub_output_close(log_file());
        set_selector(selector() - 2);
        if selector() == SELECTOR_TERM_ONLY {
            crate::output::print_nl_cstr(c"Transcript written on ".as_ptr());
            crate::output::print(texmf_log_name());
            crate::output::print_char(b'.' as i32);
        }
    }

    crate::output::print_ln();
}

static mut STDIN_UFILE: UFile = UFile {
    handle: INVALID_HANDLE,
    saved_char: -1,
    skip_next_lf: 0,
    encoding_mode: UTF8 as i16,
    conversion_data: std::ptr::null_mut(),
};

// translated from xetex-ini.c:3035 (init_io)
pub unsafe fn init_io() {
    // This is largely vestigial at this point.
    STDIN_UFILE.handle = INVALID_HANDLE;
    STDIN_UFILE.saved_char = -1;
    STDIN_UFILE.skip_next_lf = 0;
    STDIN_UFILE.encoding_mode = UTF8 as i16;
    STDIN_UFILE.conversion_data = std::ptr::null_mut();
    set_input_file(0, (&raw mut STDIN_UFILE).cast());

    set_buffer_entry(first(), 0);
    set_last(first());
    set_cur_input_loc(first());
    set_cur_input_limit(last());
    set_first(last() + 1);
}
