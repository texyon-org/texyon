//! Rust translation of xetex-io.c: low-level input/output helpers.

use crate::consts::{
    BIGGEST_CHAR, CLOSED, INT_BASE, INT_PAR_END_LINE_CHAR, INVALID_HANDLE, JUST_OPEN, LEFT_BRACE,
    NEW_LINE, OTHER_CHAR, RELAX, SELECTOR_LOG_ONLY, SELECTOR_NEW_STRING, SELECTOR_TERM_ONLY,
    SPACER, TOO_BIG_CHAR,
};
use crate::input::InputAbort;
use crate::mem::{xcalloc, xmalloc};
use tectonic_bridge_core::EOF;

// ---------------------------------------------------------------------------
// UFILE — Rust mirror of the C `UFILE` struct (xetex-io.h:12–18)
// ---------------------------------------------------------------------------

/// Rust mirror of `UFILE` (xetex-io.h:12–18). Must stay layout-compatible.
///
/// On Linux x86_64 the C layout is:
// Consts used by the file-naming / start_input / close_files group.
use crate::consts::EMPTY_STRING;
const INT_PAR_NEW_LINE_CHAR_CFT: i32 = 49;
const INT_PAR_TRACING_STACK_LEVELS: i32 = 55;
const TTBC_FILE_FORMAT_TECTONIC_PRIMARY_SI: i32 = 59;
const TTBC_FILE_FORMAT_TEX_SI: i32 = 26;

// UTF-8 decode tables used by start_input (crate::utf8).

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

// The C `UFILE` is gone — nothing outside this crate names the type any more —
// but the layout above is still the one the C struct had, and staying
// bit-compatible with it keeps the checkpoint format readable. The documented
// offsets assume 64-bit pointers, so only assert them there; on wasm32 the same
// field order packs down to 24 bytes.
#[cfg(target_pointer_width = "64")]
const _: () = assert!(
    std::mem::size_of::<UFile>() == 32,
    "UFile size mismatch with C UFILE"
);

impl Drop for UFile {
    fn drop(&mut self) {
        if self.encoding_mode == ICUMAPPING as i16 && !self.conversion_data.is_null() {
            // SAFETY: this slot owns the converter installed by
            // `set_input_file_encoding_in`.
            unsafe { xetex_ucnv_close(self.conversion_data) };
        }
    }
}

pub(crate) fn stdin_ufile_ptr() -> *mut UFile {
    &raw mut STDIN_UFILE
}

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
const K_STATUS_NO_ERROR: libc::c_long = 0;

// File format constants (tectonic_bridge_core_generated.h).
const TTBC_FILE_FORMAT_TECTONIC_PRIMARY: libc::c_int = 59;
const TTBC_FILE_FORMAT_TEX: libc::c_int = 26;

// INT_BASE and INT_PAR offsets used by open_or_close_in (xetex_format.h:219,110–111).
const INT_PAR_XETEX_DEFAULT_INPUT_MODE: i32 = 75;
const INT_PAR_XETEX_DEFAULT_INPUT_ENCODING: i32 = 76;

// Bridge I/O: direct Rust calls into tectonic_bridge_core (crate::bridge),
// keeping the C shim names. `input_seek`/`input_close` are the non-longjmp
// seek/close replacements, re-exported here for their cross-module callers.
pub(crate) use crate::bridge::{input_close, input_seek};
use crate::bridge::{
    ttstub_get_last_input_abspath, ttstub_input_getc, ttstub_input_open, ttstub_input_open_primary,
    ttstub_input_ungetc, ttstub_output_close, ttstub_output_flush, ttstub_output_open,
    ttstub_output_putc,
};

// ---------------------------------------------------------------------------
// C externs needed by translated functions
// ---------------------------------------------------------------------------

extern "C" {
    // TECkit functions (C++ engine with C API — extern "C" ABI).
    // TECkit_Status is `long` (teckit-Common.h:50): 64-bit on LP64, 32-bit on
    // wasm32 and Windows, so it has to be spelled `c_long`. wasm validates call
    // signatures, so a mismatch here traps instead of quietly working.
    fn TECkit_CreateConverter(
        mapping: *const u8,
        mapping_size: u32,
        map_forward: u8,
        source_form: u16,
        target_form: u16,
        converter: *mut *mut libc::c_void,
    ) -> libc::c_long;
    fn TECkit_ConvertBuffer(
        converter: *mut libc::c_void,
        in_buffer: *const u8,
        in_length: u32,
        in_used: *mut u32,
        out_buffer: *mut u8,
        out_length: u32,
        out_used: *mut u32,
        input_is_complete: u8,
    ) -> libc::c_long;
    fn TECkit_ResetConverter(converter: *mut libc::c_void) -> libc::c_long;

    // bad_utf8_warning remains in C (xetex-xetexd.h:912).

    // gettexstring — translated to Rust (texmfmp.rs) but called via FFI.

    // Output helpers — all translated to Rust in output.rs.

    // Callees of open_or_close_in still in C.

}

// ---------------------------------------------------------------------------
// ICU converter FFI (direct, using the versioned symbol names)
// ---------------------------------------------------------------------------

// UCNV_UTF32_NativeEndian (xetex-core.h:30,35): the UConverterType enum value
// for host-endian UTF-32 (unicode/ucnv.h: BigEndian = 7, LittleEndian = 8).
#[cfg(target_endian = "little")]
const UCNV_UTF32_NATIVE_ENDIAN: libc::c_int = 8;
#[cfg(target_endian = "big")]
const UCNV_UTF32_NATIVE_ENDIAN: libc::c_int = 7;

// UErrorCode is a C enum: i32 ABI, U_ZERO_ERROR = 0. ICU checks the incoming
// value (U_FAILURE short-circuits), so it must be zero-initialized before calls.
extern "C" {
    #[link_name = concat!("ucnv_open", env!("ICU_VERSION_SUFFIX"))]
    fn ucnv_open(converter_name: *const libc::c_char, err: *mut i32) -> *mut libc::c_void;
    #[link_name = concat!("ucnv_close", env!("ICU_VERSION_SUFFIX"))]
    fn ucnv_close(converter: *mut libc::c_void);
    #[link_name = concat!("ucnv_toAlgorithmic", env!("ICU_VERSION_SUFFIX"))]
    fn ucnv_toAlgorithmic(
        algorithmic_type: libc::c_int,
        cnv: *mut libc::c_void,
        target: *mut libc::c_char,
        target_capacity: i32,
        source: *const libc::c_char,
        source_length: i32,
        err: *mut i32,
    ) -> i32;
}

/// translated from xetex-io.c (xetex_ucnv_close)
pub(crate) unsafe fn xetex_ucnv_close(converter: *mut libc::c_void) {
    ucnv_close(converter);
}

/// translated from xetex-io.c (xetex_ucnv_open)
pub(crate) unsafe fn xetex_ucnv_open(
    name: *const libc::c_char,
    out_err: *mut libc::c_int,
) -> *mut libc::c_void {
    let mut err: i32 = 0;
    let cnv = ucnv_open(name, &mut err);
    if !out_err.is_null() {
        *out_err = err;
    }
    cnv
}

/// translated from xetex-io.c (xetex_ucnv_to_utf32)
pub(crate) unsafe fn xetex_ucnv_to_utf32(
    cnv: *mut libc::c_void,
    target: *mut libc::c_char,
    target_capacity: i32,
    source: *const libc::c_char,
    source_length: i32,
    out_err: *mut libc::c_int,
) -> i32 {
    let mut err: i32 = 0;
    let result = ucnv_toAlgorithmic(
        UCNV_UTF32_NATIVE_ENDIAN,
        cnv,
        target,
        target_capacity,
        source,
        source_length,
        &mut err,
    );
    if !out_err.is_null() {
        *out_err = err;
    }
    result
}

// ---------------------------------------------------------------------------
// tt_xetex_open_input
// ---------------------------------------------------------------------------

pub(crate) unsafe fn tt_xetex_open_input_in(
    st: &mut crate::state::EngineState,
    filefmt: libc::c_int,
) -> usize {
    let nof = || {
        if st.input.name_of_file.is_empty() {
            std::ptr::null_mut::<libc::c_char>()
        } else {
            st.input.name_of_file.as_ptr() as *mut libc::c_char
        }
    };
    let handle = if filefmt == TTBC_FILE_FORMAT_TECTONIC_PRIMARY {
        ttstub_input_open_primary()
    } else if *nof() == b'|' as libc::c_char {
        crate::output::print_nl_cstr_in(st, c"Warning: ".as_ptr());
        crate::output::diagnostic_begin_capture_warning_here_in(st);
        crate::output::print_cstr_in(
            st,
            c"piped inputs from external commands are not implemented in Tectonic".as_ptr(),
        );
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        return INVALID_HANDLE;
    } else {
        ttstub_input_open(nof(), filefmt, 0)
    };

    if handle == INVALID_HANDLE {
        return INVALID_HANDLE;
    }

    if ttstub_get_last_input_abspath(
        st.input.abspath_of_input_file.0.as_mut_ptr(),
        st.input.abspath_of_input_file.0.len(),
    ) < 1
    {
        st.input.abspath_of_input_file.0[0] = 0;
    }

    let nof_ptr = nof();
    st.input.name_length = libc::strlen(nof_ptr) as i32;
    st.input.name_of_input_file = Some(std::ffi::CStr::from_ptr(nof_ptr).to_owned());
    handle
}

/// Open and configure an owned input file. `None` means the bridge open failed.
pub(crate) unsafe fn u_open_in_in(
    st: &mut crate::state::EngineState,
    filefmt: i32,
    mut mode: i32,
    encoding_data: i32,
) -> Result<Option<Box<UFile>>, crate::texmfmp::MakeTexStringAbort> {
    let handle = tt_xetex_open_input_in(st, filefmt);
    if handle == INVALID_HANDLE {
        return Ok(None);
    }

    let mut f = Box::new(UFile {
        handle,
        saved_char: -1,
        skip_next_lf: 0,
        encoding_mode: 0,
        conversion_data: std::ptr::null_mut(),
    });

    if mode == AUTO {
        let b1 = ttstub_input_getc(f.handle);
        let b2 = ttstub_input_getc(f.handle);

        if b1 == 0xfe && b2 == 0xff {
            mode = UTF16BE;
        } else if b2 == 0xfe && b1 == 0xff {
            mode = UTF16LE;
        } else if b1 == 0 && b2 != 0 {
            mode = UTF16BE;
            input_seek(f.handle, 0, libc::SEEK_SET);
        } else if b2 == 0 && b1 != 0 {
            mode = UTF16LE;
            input_seek(f.handle, 0, libc::SEEK_SET);
        } else if b1 == 0xEF && b2 == 0xBB {
            let b3 = ttstub_input_getc(f.handle);
            if b3 == 0xBF {
                mode = UTF8;
            }
        }

        if mode == AUTO {
            input_seek(f.handle, 0, libc::SEEK_SET);
            mode = UTF8;
        }
    }

    set_input_file_encoding_in(st, &mut *f, mode, encoding_data)?;
    Ok(Some(f))
}

// ---------------------------------------------------------------------------
// set_input_file_encoding
// ---------------------------------------------------------------------------

/// Set the encoding mode (and optional ICU converter) on an open UFILE.
///
/// translated from xetex-io.c:47 (set_input_file_encoding)
pub(crate) unsafe fn set_input_file_encoding_in(
    st: &mut crate::state::EngineState,
    f: *mut UFile,
    mode: i32,
    encoding_data: i32,
) -> Result<(), crate::texmfmp::MakeTexStringAbort> {
    if (*f).encoding_mode == ICUMAPPING as i16 && !(*f).conversion_data.is_null() {
        xetex_ucnv_close((*f).conversion_data);
    }
    (*f).conversion_data = std::ptr::null_mut();

    match mode {
        UTF8 | UTF16BE | UTF16LE | RAW => {
            (*f).encoding_mode = mode as i16;
        }
        _ if mode == ICUMAPPING => {
            let name = crate::texmfmp::gettexstring_in(&st.strings, encoding_data);
            let mut err: libc::c_int = 0;
            let cnv = xetex_ucnv_open(name, &mut err);
            if cnv.is_null() {
                crate::output::begin_diagnostic_in(st);
                crate::output::print_nl_in(st, b'E' as i32);
                crate::output::print_cstr_in(st, c"rror ".as_ptr());
                crate::output::print_int_in(st, err);
                crate::output::print_cstr_in(st, c" creating Unicode converter for `".as_ptr());
                crate::output::print_cstr_in(st, name);
                crate::output::print_cstr_in(st, c"'; reading as raw bytes".as_ptr());
                crate::output::end_diagnostic_in(st, true);
                (*f).encoding_mode = RAW as i16;
            } else {
                (*f).encoding_mode = ICUMAPPING as i16;
                (*f).conversion_data = cnv;
            }
            libc::free(name as *mut libc::c_void);
        }
        _ => {}
    }
    Ok(())
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
// sentinel.
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
    let out_ptr = st.input.buffer.as_mut_ptr().add(st.input.first as usize) as *mut u8;
    let out_cap = (4 * (st.input.buf_size - st.input.first)) as u32;
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
    st.input.last = st.input.first + (out_used / 4) as i32;
    Ok(())
}

// ---------------------------------------------------------------------------
// input_line
// ---------------------------------------------------------------------------

/// Read one line from UFILE into the TeX `buffer`, handling encodings.
///
/// translated from xetex-io.c:135 (input_line).
///
/// Cold `_tt_abort`s ride the [`InputAbort`] sentinel.
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

    st.input.last = st.input.first;

    if (*f).encoding_mode as i32 == ICUMAPPING {
        // ICU-converter path.
        let mut bytes_read: u32 = 0;
        let mut error_code: libc::c_int = 0;

        if BYTE_BUFFER.is_null() {
            BYTE_BUFFER = xmalloc((st.input.buf_size + 1) as libc::size_t) as *mut libc::c_char;
        }

        let mut i = ttstub_input_getc((*f).handle);
        if (*f).skip_next_lf != 0 {
            (*f).skip_next_lf = 0;
            if i == b'\n' as libc::c_int {
                i = ttstub_input_getc((*f).handle);
            }
        }

        if i != EOF && i != b'\n' as i32 && i != b'\r' as i32 {
            *BYTE_BUFFER.add(bytes_read as usize) = i as libc::c_char;
            bytes_read += 1;
            while bytes_read < st.input.buf_size as u32 {
                i = ttstub_input_getc((*f).handle);
                if i == EOF || i == b'\n' as i32 || i == b'\r' as i32 {
                    break;
                }
                *BYTE_BUFFER.add(bytes_read as usize) = i as libc::c_char;
                bytes_read += 1;
            }
        }

        if i == EOF && *crate::compat::errno_location() != libc::EINTR && bytes_read == 0 {
            return Ok(0); // false
        }

        if i != EOF && i != b'\n' as i32 && i != b'\r' as i32 {
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
                    UTF32_BUF = xcalloc(
                        st.input.buf_size as libc::size_t,
                        std::mem::size_of::<u32>(),
                    ) as *mut u32;
                }
                let tmp_len = xetex_ucnv_to_utf32(
                    cnv,
                    UTF32_BUF as *mut libc::c_char,
                    st.input.buf_size * 4,
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
                let out_ptr = st.input.buffer.as_mut_ptr().add(st.input.first as usize);
                let out_len = xetex_ucnv_to_utf32(
                    cnv,
                    out_ptr as *mut libc::c_char,
                    4 * (st.input.buf_size - st.input.first),
                    BYTE_BUFFER as *const libc::c_char,
                    bytes_read as i32,
                    &mut error_code,
                );
                if error_code != 0 {
                    conversion_error_in(st, error_code);
                    return Ok(0);
                }
                st.input.last = st.input.first + out_len / 4;
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
                    UTF32_BUF = xcalloc(
                        st.input.buf_size as libc::size_t,
                        std::mem::size_of::<u32>(),
                    ) as *mut u32;
                }
                let mut tmp_len: i32 = 0;
                if i != EOF && i != b'\n' as i32 && i != b'\r' as i32 {
                    *UTF32_BUF.add(tmp_len as usize) = i as u32;
                    tmp_len += 1;
                    while tmp_len < st.input.buf_size {
                        i = get_uni_c_in(st, f)?;
                        if i == EOF || i == b'\n' as i32 || i == b'\r' as i32 {
                            break;
                        }
                        *UTF32_BUF.add(tmp_len as usize) = i as u32;
                        tmp_len += 1;
                    }
                }

                if i == EOF && *crate::compat::errno_location() != libc::EINTR && tmp_len == 0 {
                    return Ok(0);
                }
                if i != EOF && i != b'\n' as i32 && i != b'\r' as i32 {
                    return Err(InputAbort::TtAbort(
                        c"unable to read an entire line (buffer too small)".as_ptr(),
                    ));
                }
                apply_normalization_in(st, UTF32_BUF, tmp_len, norm)?;
            }
            _ => {
                // No normalization: store directly into buffer[last..].
                if st.input.last < st.input.buf_size
                    && i != EOF
                    && i != b'\n' as i32
                    && i != b'\r' as i32
                {
                    st.input.buffer[st.input.last as usize] = i;
                    st.input.last += 1;
                    while st.input.last < st.input.buf_size {
                        i = get_uni_c_in(st, f)?;
                        if i == EOF || i == b'\n' as i32 || i == b'\r' as i32 {
                            break;
                        }
                        st.input.buffer[st.input.last as usize] = i;
                        st.input.last += 1;
                    }
                }

                if i == EOF
                    && *crate::compat::errno_location() != libc::EINTR
                    && st.input.last == st.input.first
                {
                    return Ok(0);
                }
                if i != EOF && i != b'\n' as i32 && i != b'\r' as i32 {
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
        if st.input.last <= st.input.first {
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
        let extra_bytes = crate::utf8::BYTES_FROM_UTF8[rval as usize] as usize;
        // Translate C's falling-through switch (same pattern as maketexstring).
        for _ in 0..extra_bytes {
            if *s != 0 {
                rval = (rval << 6) + *s as u32;
                s = s.add(1);
            } else {
                rval <<= 6;
            }
        }
        rval -= crate::utf8::OFFSETS_FROM_UTF8[extra_bytes] as u32;
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

// ---------------------------------------------------------------------------
// get_uni_c
// ---------------------------------------------------------------------------

/// Sole abort from [`get_uni_c_in`]: the C `_tt_abort("internal error; file
/// input mode is invalid")`. A dedicated fieldless error keeps the
/// per-character `Result` a scalar pair returned in registers — carrying
/// `InputAbort` (16-byte aggregate) forced an sret memory return on every
/// character read, ~2x the C `get_uni_c` cost on this hot path.
pub(crate) struct InvalidInputMode;

impl From<InvalidInputMode> for InputAbort {
    fn from(_: InvalidInputMode) -> Self {
        InputAbort::TtAbort(c"internal error; file input mode is invalid".as_ptr())
    }
}

/// Read one Unicode codepoint from a UFILE (handles UTF-8, UTF-16BE/LE, RAW).
///
/// translated from xetex-io.c:373 (get_uni_c)
// The impossible invalid-mode `_tt_abort` rides the `InvalidInputMode`
// sentinel; the `?` at the `input_line_in` call sites converts it to
// `InputAbort::TtAbort` with the C abort message.
// Called only from `input_line_in`.
pub(crate) unsafe fn get_uni_c_in(
    st: &mut crate::state::EngineState,
    f: *mut UFile,
) -> Result<libc::c_int, InvalidInputMode> {
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
            if rval != EOF {
                let extra_bytes = crate::utf8::BYTES_FROM_UTF8[rval as usize] as usize;
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
                        if c != EOF {
                            ttstub_input_ungetc((*f).handle, c);
                        }
                        crate::output::bad_utf8_warning_in(st);
                        return Ok(0xFFFD);
                    }
                    rval = (rval << 6) + c;
                }
                // Subtract offset and validate range.
                let offset = crate::utf8::OFFSETS_FROM_UTF8[extra_bytes];
                rval = (rval as u32).wrapping_sub(offset) as i32;
                if !(0..=0x10ffff).contains(&rval) {
                    crate::output::bad_utf8_warning_in(st);
                    return Ok(0xfffd);
                }
            }
        }

        UTF16BE => {
            rval = ttstub_input_getc((*f).handle);
            if rval != EOF {
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
            if rval != EOF {
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
            return Err(InvalidInputMode);
        }
    }

    Ok(rval)
}

// ---------------------------------------------------------------------------
// u_close
// ---------------------------------------------------------------------------

// translated from xetex-io.c:273 (u_close)
///
/// Returns `true` on the serious internal close error the C `ttstub_input_close`
/// wrapper `longjmp`'d on: callers on a `Result` path propagate it
/// as their `InputClose` abort, the terminal-cleanup caller (`tt_cleanup`)
/// absorbs it. On that error the ICU-converter close + `free(f)` are skipped —
/// exactly as the old longjmp skipped them.
#[must_use]
pub unsafe fn u_close(slot: crate::state::InputSlot) -> bool {
    let crate::state::InputSlot::Open(f) = slot else {
        return false;
    };

    if f.handle == INVALID_HANDLE {
        std::mem::forget(f);
        return false;
    }

    if crate::io::input_close(f.handle) {
        // Preserve the legacy longjmp path, which skipped converter cleanup.
        std::mem::forget(f);
        return true;
    }

    drop(f);
    false
}

// ---------------------------------------------------------------------------
// open_or_close_in
// ---------------------------------------------------------------------------

/// Open or close a \read stream.
///
/// translated from xetex-io.c:294 (open_or_close_in)
pub(crate) enum OpenOrCloseAbort {
    Scanner(SccAbort),
    Name(NameAbort),
    MakeString(crate::texmfmp::MakeTexStringAbort),
    /// Serious internal error closing a `\read` stream (the C `ttstub_input_close`
    /// wrapper `longjmp`'d here) — from `u_close`.
    InputClose,
}

impl OpenOrCloseAbort {
    /// Map to the unified [`EngineAbort`] terminal.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            OpenOrCloseAbort::Scanner(e) => e.into_engine_abort(),
            OpenOrCloseAbort::Name(e) => e.into_engine_abort(),
            OpenOrCloseAbort::MakeString(e) => e.into_engine_abort(),
            OpenOrCloseAbort::InputClose => crate::errors::EngineAbort::TtAbort(
                c"serious internal error closing an input file".as_ptr(),
            ),
        }
    }
}

pub(crate) unsafe fn open_or_close_in(
    st: &mut crate::state::EngineState,
) -> Result<(), OpenOrCloseAbort> {
    let c = st.scanner.cur_chr as u8;
    crate::scan::scan_four_bit_int_in(st).map_err(OpenOrCloseAbort::Scanner)?;
    let n = st.scanner.cur_val as u8 as usize;

    if st.input.read_open[n] != CLOSED {
        let slot = std::mem::take(&mut st.input.read_file[n]);
        if u_close(slot) {
            return Err(OpenOrCloseAbort::InputClose);
        }
        st.input.read_open[n] = CLOSED;
    }

    if c != 0 {
        crate::scan::scan_optional_equals_in(st).map_err(OpenOrCloseAbort::Scanner)?;
        scan_file_name_in(st).map_err(OpenOrCloseAbort::Scanner)?;
        let (name, area, ext) = (st.scanner.cur_name, st.scanner.cur_area, st.scanner.cur_ext);
        pack_file_name_in(st, name, area, ext);

        if let Some(file) = u_open_in_in(
            st,
            TTBC_FILE_FORMAT_TEX,
            st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_MODE),
            st.eqtb
                .b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_ENCODING),
        )
        .map_err(OpenOrCloseAbort::MakeString)?
        {
            st.input.read_file[n] = crate::state::InputSlot::Open(file);
            make_utf16_name_in(st);
            st.scanner.name_in_progress = true;
            begin_name_in(st);
            st.scanner.stop_at_space = false;
            let mut k = 0i32;
            while k < st.input.name_length16
                && more_name_in(st, st.input.name_of_file16[k as usize])
                    .map_err(OpenOrCloseAbort::Name)?
            {
                k += 1;
            }
            st.scanner.stop_at_space = true;
            end_name_in(st).map_err(OpenOrCloseAbort::Name)?;
            st.scanner.name_in_progress = false;
            st.input.read_open[n] = JUST_OPEN;
        }
    }

    Ok(())
}

/// translated from xetex-xetex0.c:7827 (begin_name)
pub(crate) fn begin_name_in(st: &mut crate::state::EngineState) {
    st.scanner.area_delimiter = 0;
    st.scanner.ext_delimiter = 0;
    st.scanner.quoted_filename = false;
    st.scanner.file_name_quote_char = 0;
}

/// Cold abort reasons threaded out of the file-name machinery
/// ([`more_name_in`]/[`end_name_in`]/[`make_name_string_in`]).
pub(crate) enum NameAbort {
    /// `pool_ptr + 1 > pool_size`: `overflow("pool size", …)`.
    PoolSize,
    /// `str_ptr + 3 > max_strings` (or `make_string`): `overflow("number of strings", …)`.
    NumberOfStrings,
}

/// The cold pool-size `overflow` defers via [`NameAbort`].
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

impl NameAbort {
    /// Map to the unified [`EngineAbort`] terminal.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            NameAbort::PoolSize => crate::errors::EngineAbort::PoolOverflow,
            NameAbort::NumberOfStrings => crate::errors::EngineAbort::StringOverflow,
        }
    }
}
/// The cold `overflow("number of strings")` (direct or via
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

/// The cold `overflow` defers via [`NameAbort`].
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

/// The string build's overflow rides [`crate::texmfmp::MakeTexStringAbort`].
pub(crate) unsafe fn pack_job_name_in(
    st: &mut crate::state::EngineState,
    s: *const libc::c_char,
) -> Result<(), crate::texmfmp::MakeTexStringAbort> {
    st.scanner.cur_area = EMPTY_STRING;
    st.scanner.cur_ext = crate::texmfmp::maketexstring_in(&mut st.strings, s)?;
    st.scanner.cur_name = st.strings.job_name;
    let (n, a, e) = (st.scanner.cur_name, st.scanner.cur_area, st.scanner.cur_ext);
    pack_file_name_in(st, n, a, e);
    Ok(())
}
/// Cold-abort sentinel for [`open_log_file_in`].
pub(crate) enum OpenLogAbort {
    /// A `maketexstring`/`pack_job_name` overflow.
    MakeString(crate::texmfmp::MakeTexStringAbort),
    /// A `make_name_string` overflow.
    Name(NameAbort),
    /// `_tt_abort("cannot open log file output")` — the host refused the file.
    CannotOpen,
}

impl OpenLogAbort {
    /// Map to the unified [`EngineAbort`] terminal.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            OpenLogAbort::MakeString(e) => e.into_engine_abort(),
            OpenLogAbort::Name(e) => e.into_engine_abort(),
            OpenLogAbort::CannotOpen => {
                crate::errors::EngineAbort::TtAbort(c"cannot open log file output".as_ptr())
            }
        }
    }
}

/// translated from xetex-xetex0.c:7937 (open_log_file).
///
/// Opens and writes the transcript banner; the three cold paths ride
/// [`OpenLogAbort`].
pub(crate) unsafe fn open_log_file_in(
    st: &mut crate::state::EngineState,
) -> Result<(), OpenLogAbort> {
    let old_setting = st.shipout.selector;
    if st.strings.job_name == 0 {
        st.strings.job_name = crate::texmfmp::maketexstring_in(&mut st.strings, c"texput".as_ptr())
            .map_err(OpenLogAbort::MakeString)?;
    }

    pack_job_name_in(st, c".log".as_ptr()).map_err(OpenLogAbort::MakeString)?;

    let name = if st.input.name_of_file.is_empty() {
        std::ptr::null_mut()
    } else {
        st.input.name_of_file.as_ptr() as *mut libc::c_char
    };
    let handle = ttstub_output_open(name, 0);
    if handle == INVALID_HANDLE {
        return Err(OpenLogAbort::CannotOpen);
    }
    st.shipout.log_file = handle;

    st.strings.texmf_log_name = make_name_string_in(st).map_err(OpenLogAbort::Name)?;
    st.shipout.selector = SELECTOR_LOG_ONLY;
    st.errors.log_opened = true;

    // `st.input.cur_input` reads the raw `cur_input` global.
    let ptr = st.input.input_ptr;
    st.input.input_stack[ptr as usize] = st.input.cur_input;

    crate::output::print_nl_cstr_in(st, c"**".as_ptr());
    let mut l = st.input.input_stack[0].limit;
    if st.input.buffer[l as usize] == st.eqtb.b32s1(INT_BASE + INT_PAR_END_LINE_CHAR) {
        l -= 1;
    }

    let mut k = 1;
    while k <= l {
        let ch = st.input.buffer[k as usize];
        crate::output::print_in(st, ch);
        k += 1;
    }

    crate::output::print_ln_in(st);
    st.shipout.selector = old_setting + 2;
    Ok(())
}

/// Recreate bridge-owned output handles after restoring a resident checkpoint.
///
/// Checkpoints retain the logical output state (DVI offsets, selector, and
/// whether the transcript was opened), but bridge handles and diagnostic
/// pointers are valid only for the operation that created them.
pub(crate) unsafe fn reopen_resident_outputs_in(
    st: &mut crate::state::EngineState,
) -> Result<(), crate::errors::EngineAbort> {
    use crate::bridge::ttstub_output_open;
    use crate::consts::INVALID_HANDLE;

    st.shipout.current_diagnostic = core::ptr::null_mut();
    st.shipout.write_file = [INVALID_HANDLE; 16];
    st.shipout.log_file = INVALID_HANDLE;
    st.shipout.dvi_file = INVALID_HANDLE;

    if st.errors.log_opened {
        pack_job_name_in(st, c".log".as_ptr()).map_err(|e| e.into_engine_abort())?;
        let handle = ttstub_output_open(st.input.name_of_file.as_ptr(), 0);
        if handle == INVALID_HANDLE {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"cannot reopen log file output".as_ptr(),
            ));
        }
        st.shipout.log_file = handle;
    }

    if st.shipout.output_file_name != 0 {
        pack_job_name_in(st, crate::engine::output_file_extension(st))
            .map_err(|e| e.into_engine_abort())?;
        let handle = ttstub_output_open(st.input.name_of_file.as_ptr(), 0);
        if handle == INVALID_HANDLE {
            return Err(crate::errors::EngineAbort::TtAbort(
                c"cannot reopen XDV file output".as_ptr(),
            ));
        }
        st.shipout.dvi_file = handle;
    }

    Ok(())
}

/// Builds `name_of_file`/`name_length` from the three string-pool components.
pub(crate) unsafe fn pack_file_name_in(st: &mut crate::state::EngineState, n: i32, a: i32, e: i32) {
    extern "C" {}

    let len_a = crate::string_pool::length(&st.strings, a) as usize;
    let len_n = crate::string_pool::length(&st.strings, n) as usize;
    let len_e = crate::string_pool::length(&st.strings, e) as usize;
    // Each TeX character is at most 3 UTF-8 bytes; +2 for NUL terminators.
    let work_size = (len_a + len_n + len_e) * 3 + 2;
    let work_buffer = xmalloc(work_size as libc::size_t) as *mut libc::c_char;
    *work_buffer = 0;

    let a_utf8 = crate::texmfmp::gettexstring_in(&st.strings, a);
    libc::strcat(work_buffer, a_utf8);
    libc::free(a_utf8 as *mut libc::c_void);

    let n_utf8 = crate::texmfmp::gettexstring_in(&st.strings, n);
    libc::strcat(work_buffer, n_utf8);
    libc::free(n_utf8 as *mut libc::c_void);

    let e_utf8 = crate::texmfmp::gettexstring_in(&st.strings, e);
    libc::strcat(work_buffer, e_utf8);
    libc::free(e_utf8 as *mut libc::c_void);

    let nl = libc::strlen(work_buffer) as i32;
    st.input.name_length = nl;
    // `work_buffer` is never null here; mirror `set_name_of_file`'s non-null arm.
    let len = libc::strlen(work_buffer);
    st.input.name_of_file.resize(len + 1, 0);
    libc::strcpy(st.input.name_of_file.as_mut_ptr(), work_buffer);
    libc::free(work_buffer as *mut libc::c_void);
}

pub(crate) unsafe fn close_files_and_terminate_in(
    st: &mut crate::state::EngineState,
) -> Result<(), crate::errors::EngineAbort> {
    for k in 0usize..=15 {
        if st.shipout.write_open[k] {
            crate::shipout::ensure_write_stream_open(st, k)
                .map_err(|error| error.into_engine_abort())?;
            ttstub_output_close(st.shipout.write_file[k]);
        }
    }

    // Finalization runs while a live-preview journal may be armed. Route this
    // mutation through the accessor so rewinding can restore `\newlinechar`.
    st.eqtb.set_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR_CFT, -1);

    crate::shipout::finalize_dvi_file_in(st).map_err(|e| e.into_engine_abort())?;
    let log_was_opened = st.errors.log_opened;
    crate::synctex::synctex_terminate(st, log_was_opened);

    if st.errors.log_opened {
        let log_file = st.shipout.log_file;
        ttstub_output_putc(log_file, b'\n' as i32);
        ttstub_output_close(log_file);
        st.shipout.selector -= 2;
        if st.shipout.selector == SELECTOR_TERM_ONLY {
            let texmf_log_name = st.strings.texmf_log_name;
            crate::output::print_nl_cstr_in(st, c"Transcript written on ".as_ptr());
            crate::output::print_in(st, texmf_log_name);
            crate::output::print_char_in(st, b'.' as i32);
        }
    }

    crate::output::print_ln_in(st);
    Ok(())
}

static mut STDIN_UFILE: UFile = UFile {
    handle: INVALID_HANDLE,
    saved_char: -1,
    skip_next_lf: 0,
    encoding_mode: UTF8 as i16,
    conversion_data: std::ptr::null_mut(),
};

// translated from xetex-ini.c:3035 (init_io)
pub unsafe fn init_io(st: &mut crate::state::EngineState) {
    // This is largely vestigial at this point.
    STDIN_UFILE.handle = INVALID_HANDLE;
    STDIN_UFILE.saved_char = -1;
    STDIN_UFILE.skip_next_lf = 0;
    STDIN_UFILE.encoding_mode = UTF8 as i16;
    STDIN_UFILE.conversion_data = std::ptr::null_mut();

    let first = st.input.first;
    st.input.input_file[0] = crate::state::InputSlot::Terminal;
    st.input.buffer[first as usize] = 0;
    st.input.last = first;
    let last = st.input.last;
    st.input.cur_input.loc = first;
    st.input.cur_input.limit = last;
    st.input.first = last + 1;
}

use crate::scan::SccAbort;

/// Lift a file-name [`NameAbort`] overflow into [`SccAbort`] (via the
/// input-stack overflow sentinel), computing `n` from the live pool/string
/// scalars.
fn name_abort_scc(st: &crate::state::EngineState, e: NameAbort) -> SccAbort {
    let (what, n) = match e {
        NameAbort::PoolSize => (
            c"pool size".as_ptr(),
            st.strings.pool_size - st.strings.init_pool_ptr,
        ),
        NameAbort::NumberOfStrings => (
            c"number of strings".as_ptr(),
            st.strings.max_strings - st.strings.init_str_ptr,
        ),
    };
    SccAbort::Input(crate::input::InputAbort::Overflow { what, n })
}

pub(crate) fn scan_and_pack_name_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    scan_file_name_in(st)?;
    let (n, a, e) = (st.scanner.cur_name, st.scanner.cur_area, st.scanner.cur_ext);
    unsafe { pack_file_name_in(st, n, a, e) };
    Ok(())
}
pub(crate) fn scan_file_name_braced_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let save_scanner_status = st.scanner.scanner_status;
    let save_def_ref = st.scanner.def_ref.0;
    let save_cur_cs = st.scanner.cur_cs;
    let save_stop_at_space = st.scanner.stop_at_space;

    st.scanner.cur_cs = st.scanner.warning_index;
    crate::scan::scan_toks_in(st, false, true)?;

    st.shipout.old_setting = st.shipout.selector as u8;
    st.shipout.selector = SELECTOR_NEW_STRING;
    let arg = st.mem.b32s1(st.scanner.def_ref);
    let l = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list_in(st, arg, crate::mem::TEX_NULL, l);
    st.shipout.selector = st.shipout.old_setting as i32;

    let s = crate::string_pool::make_string(&mut st.strings)
        .map_err(|_| name_abort_scc(st, NameAbort::NumberOfStrings))?;
    let dr = st.scanner.def_ref.0;
    crate::mem::delete_token_ref(&mut st.mem, dr);
    st.scanner.def_ref.0 = save_def_ref;
    st.scanner.cur_cs = save_cur_cs;
    st.scanner.scanner_status = save_scanner_status;

    begin_name_in(st);
    let start = st.strings.str_start[(s - TOO_BIG_CHAR) as usize];
    let end = st.strings.str_start[(s + 1 - TOO_BIG_CHAR) as usize];
    for i in start..end {
        let c = st.strings.str_pool[i as usize];
        more_name_in(st, c).map_err(|e| name_abort_scc(st, e))?;
    }
    st.scanner.stop_at_space = save_stop_at_space;
    Ok(())
}
pub(crate) fn scan_file_name_in(st: &mut crate::state::EngineState) -> Result<(), SccAbort> {
    let save_warning_index = st.scanner.warning_index;
    st.scanner.warning_index = st.scanner.cur_cs;

    loop {
        crate::scan::get_x_token_in(st)?;
        if st.scanner.cur_cmd != SPACER && st.scanner.cur_cmd != RELAX {
            break;
        }
    }

    crate::input::back_input_in(st)?;

    if st.scanner.cur_cmd == LEFT_BRACE {
        scan_file_name_braced_in(st)?;
    } else {
        st.scanner.name_in_progress = true;
        begin_name_in(st);

        loop {
            crate::scan::get_x_token_in(st)?;
            if st.scanner.cur_cmd != SPACER {
                break;
            }
        }

        loop {
            if st.scanner.cur_cmd > OTHER_CHAR || st.scanner.cur_chr > BIGGEST_CHAR {
                crate::input::back_input_in(st)?;
                break;
            }

            if !more_name_in(st, st.scanner.cur_chr as u16).map_err(|e| name_abort_scc(st, e))? {
                break;
            }

            crate::scan::get_x_token_in(st)?;
        }
    }

    end_name_in(st).map_err(|e| name_abort_scc(st, e))?;
    st.scanner.name_in_progress = false;
    st.scanner.warning_index = save_warning_index;
    Ok(())
}
pub(crate) unsafe fn start_input_in(
    st: &mut crate::state::EngineState,
    primary_input_name: *const libc::c_char,
) -> Result<(), SccAbort> {
    use crate::input::InputAbort;

    let format = if !primary_input_name.is_null() {
        TTBC_FILE_FORMAT_TECTONIC_PRIMARY_SI
    } else {
        TTBC_FILE_FORMAT_TEX_SI
    };

    if !primary_input_name.is_null() {
        st.scanner.name_in_progress = true;
        begin_name_in(st);
        st.scanner.stop_at_space = false;

        let mut cp = primary_input_name as *const u8;
        if st.strings.pool_ptr + libc::strlen(primary_input_name) as i32 * 2 >= st.strings.pool_size
        {
            return Err(SccAbort::Input(InputAbort::TtAbort(
                c"string pool overflow".as_ptr(),
            )));
        }

        loop {
            let first_byte = *cp;
            cp = cp.add(1);
            if first_byte == 0 {
                break;
            }
            let mut rval = first_byte as u32;
            let extra_bytes = crate::utf8::BYTES_FROM_UTF8[rval as usize] as usize;
            for _ in 0..extra_bytes {
                if *cp != 0 {
                    rval = (rval << 6) + *cp as u32;
                    cp = cp.add(1);
                } else {
                    rval <<= 6;
                }
            }
            rval -= crate::utf8::OFFSETS_FROM_UTF8[extra_bytes];

            if rval > 0xffff {
                rval -= 0x10000;
                let pp = st.strings.pool_ptr;
                st.strings.str_pool[pp as usize] = (0xd800 + rval / 0x0400) as u16;
                st.strings.pool_ptr = pp + 1;
                let pp = st.strings.pool_ptr;
                st.strings.str_pool[pp as usize] = (0xdc00 + rval % 0x0400) as u16;
                st.strings.pool_ptr = pp + 1;
            } else {
                let pp = st.strings.pool_ptr;
                st.strings.str_pool[pp as usize] = rval as u16;
                st.strings.pool_ptr = pp + 1;
            }

            let cur_len = st.strings.pool_ptr
                - st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            if rval == b'/' as u32 {
                // IS_DIR_SEP
                st.scanner.area_delimiter = cur_len;
                st.scanner.ext_delimiter = 0;
            } else if rval == b'.' as u32 {
                st.scanner.ext_delimiter = cur_len;
            }
        }

        st.scanner.stop_at_space = true;
        end_name_in(st).map_err(|e| name_abort_scc(st, e))?;
        st.scanner.name_in_progress = false;
    } else {
        scan_file_name_in(st)?;
    }

    let (n, a, e) = (st.scanner.cur_name, st.scanner.cur_area, st.scanner.cur_ext);
    pack_file_name_in(st, n, a, e);
    crate::input::begin_file_reading_in(st)?;

    let idx = st.input.cur_input.index as i32;
    let mode = st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_MODE);
    let enc = st
        .eqtb
        .b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_ENCODING);
    let Some(file) = u_open_in_in(st, format, mode, enc).map_err(SccAbort::MakeTexString)? else {
        // Build "failed to open input file "<name>"" into a leaked buffer for
        // the deferred _tt_abort (the C reads name_of_file via snprintf).
        let name = if st.input.name_of_file.is_empty() {
            std::ptr::null()
        } else {
            st.input.name_of_file.as_ptr()
        };
        let buf = libc::malloc(512) as *mut libc::c_char;
        crate::compat::snprintf(buf, 512, c"failed to open input file \"%s\"".as_ptr(), name);
        return Err(SccAbort::Input(InputAbort::TtAbort(buf)));
    };
    st.input.input_file[idx as usize] = crate::state::InputSlot::Open(file);

    make_utf16_name_in(st);
    st.scanner.name_in_progress = true;
    begin_name_in(st);
    st.scanner.stop_at_space = false;
    let mut k = 0i32;
    while k < st.input.name_length16 {
        let ch = st.input.name_of_file16[k as usize];
        if !more_name_in(st, ch).map_err(|e| name_abort_scc(st, e))? {
            break;
        }
        k += 1;
    }
    st.scanner.stop_at_space = true;
    end_name_in(st).map_err(|e| name_abort_scc(st, e))?;
    st.scanner.name_in_progress = false;

    let name = make_name_string_in(st).map_err(|e| name_abort_scc(st, e))?;
    st.input.cur_input.name = name;
    let io = st.input.in_open;
    st.input.source_filename_stack[io as usize] = st.input.cur_input.name;
    let nif = st
        .input
        .name_of_input_file
        .as_deref()
        .map_or(std::ptr::null(), |name| name.as_ptr());
    let fss_str =
        crate::texmfmp::maketexstring_in(&mut st.strings, nif).map_err(SccAbort::MakeTexString)?;
    st.input.full_source_filename_stack[io as usize] = fss_str;

    if st.input.cur_input.name == st.strings.str_ptr - 1 {
        let temp_str = crate::string_pool::search_string(&st.strings, st.input.cur_input.name);
        if temp_str > 0 {
            st.input.cur_input.name = temp_str;
            st.strings.str_ptr -= 1;
            st.strings.pool_ptr =
                st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
        }
    }

    if st.strings.job_name == 0 {
        st.strings.job_name = st.scanner.cur_name;
        crate::io::open_log_file_in(st).map_err(SccAbort::OpenLog)?;
    }

    let fss = st.input.full_source_filename_stack[st.input.in_open as usize];
    if st.shipout.term_offset + crate::string_pool::length(&st.strings, fss)
        > st.config.max_print_line - 2
    {
        crate::output::print_ln_in(st);
    } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
        crate::output::print_char_in(st, b' ' as i32);
    }
    crate::output::print_char_in(st, b'(' as i32);
    st.input.open_parens += 1;
    crate::output::print_in(st, fss);
    ttstub_output_flush(st.shipout.rust_stdout);

    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS) > 0 {
        crate::output::begin_diagnostic_in(st);
        crate::output::diagnostic_begin_capture_warning_here_in(st);
        crate::output::print_ln_in(st);
        crate::output::print_char_in(st, b'~' as i32);
        let mut v = st.input.input_ptr - 1;
        if v < st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_STACK_LEVELS) {
            while v > 0 {
                crate::output::print_char_in(st, b'.' as i32);
                v -= 1;
            }
        } else {
            crate::output::print_char_in(st, b'~' as i32);
        }
        crate::output::print_cstr_in(st, c"INPUT ".as_ptr());
        let cn = st.scanner.cur_name;
        crate::output::print_in(st, cn);
        let ce = st.scanner.cur_ext;
        crate::output::print_in(st, ce);
        crate::output::print_ln_in(st);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        crate::output::end_diagnostic_in(st, false);
    }

    st.input.cur_input.state = NEW_LINE;
    let synctex_job_name = st.strings.job_name;
    let synctex_tmp = crate::texmfmp::gettexstring_in(&st.strings, synctex_job_name);
    crate::synctex::synctex_start_input(st, synctex_tmp);
    st.mem.line = 1;
    let f = st.input.input_file_ptr(idx);
    input_line_in(st, f)?;
    st.input.cur_input.limit = st.input.last;

    let elc = st.eqtb.b32s1(INT_BASE + INT_PAR_END_LINE_CHAR);
    if elc < 0 || elc > 255 {
        st.input.cur_input.limit = st.input.cur_input.limit - 1;
    } else {
        st.input.buffer[st.input.cur_input.limit as usize] = elc;
    }
    st.input.first = st.input.cur_input.limit + 1;
    st.input.cur_input.loc = st.input.cur_input.start;
    Ok(())
}
