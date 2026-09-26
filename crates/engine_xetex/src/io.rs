//! Low-level input and output: encoded line reading, file names, and the
//! log and input-file lifecycle.

use crate::consts::{
    BIGGEST_CHAR, CLOSED, EMPTY_STRING, INT_BASE, INT_PAR_END_LINE_CHAR, INT_PAR_NEW_LINE_CHAR,
    INT_PAR_TRACING_STACK_LEVELS, INT_PAR_XETEX_DEFAULT_INPUT_ENCODING,
    INT_PAR_XETEX_DEFAULT_INPUT_MODE, INT_PAR_XETEX_INPUT_NORMALIZATION, INVALID_HANDLE, JUST_OPEN,
    SELECTOR_LOG_ONLY, SELECTOR_NEW_STRING, SELECTOR_TERM_ONLY, TOO_BIG_CHAR,
};
use crate::hash::cur_length;
use crate::input::{InputAbort, NEW_LINE};
use crate::mem::{xcalloc, xmalloc};
use crate::scan::SccAbort;
use crate::state::EngineState;
use crate::texmfmp::{next_utf8_char, push_utf16, texstring_cstring, utf16_units};
use tectonic_bridge_core::EOF;

pub(crate) use crate::bridge::{input_close, input_seek};
use crate::bridge::{
    ttstub_get_last_input_abspath, ttstub_input_getc, ttstub_input_open, ttstub_input_open_primary,
    ttstub_input_ungetc, ttstub_output_close, ttstub_output_flush, ttstub_output_open,
    ttstub_output_putc, TTBC_FILE_FORMAT_TECTONIC_PRIMARY, TTBC_FILE_FORMAT_TEX,
};
use crate::cmd::{LEFT_BRACE, OTHER_CHAR, RELAX, SPACER};

pub(crate) const AUTO: i32 = 0;
pub(crate) const UTF8: i32 = 1;
pub(crate) const UTF16BE: i32 = 2;
pub(crate) const UTF16LE: i32 = 3;
pub(crate) const RAW: i32 = 4;
pub(crate) const ICUMAPPING: i32 = 5;

#[cfg(target_endian = "little")]
const NATIVE_UTF32: u16 = 6;
#[cfg(target_endian = "big")]
const NATIVE_UTF32: u16 = 5;
const K_FORM_NFC: u16 = 0x0100;
const K_FORM_NFD: u16 = 0x0200;
pub(crate) const K_STATUS_NO_ERROR: libc::c_long = 0;

/// The ICU `UConverterType` of host-endian UTF-32.
#[cfg(target_endian = "little")]
const UCNV_UTF32_NATIVE_ENDIAN: libc::c_int = 8;
#[cfg(target_endian = "big")]
const UCNV_UTF32_NATIVE_ENDIAN: libc::c_int = 7;

/// An open input file with its encoding state. The layout is that of the
/// C `UFILE`, which keeps the checkpoint format readable.
#[repr(C)]
pub(crate) struct UFile {
    pub handle: usize,
    pub saved_char: i64,
    pub skip_next_lf: i16,
    pub encoding_mode: i16,
    pub conversion_data: *mut libc::c_void,
}

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

static mut STDIN_UFILE: UFile = UFile {
    handle: INVALID_HANDLE,
    saved_char: -1,
    skip_next_lf: 0,
    encoding_mode: UTF8 as i16,
    conversion_data: std::ptr::null_mut(),
};

pub(crate) fn stdin_ufile_ptr() -> *mut UFile {
    &raw mut STDIN_UFILE
}

extern "C" {
    // TECkit_Status is a C `long`, so its width differs between LP64, wasm32
    // and Windows.
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
}

// ICU checks the incoming UErrorCode, so it must be zeroed before each call.
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

pub(crate) unsafe fn xetex_ucnv_close(converter: *mut libc::c_void) {
    ucnv_close(converter);
}

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

/// Converts `source` to host-endian UTF-32 in `target`, returning the byte
/// count and the ICU error code.
unsafe fn xetex_ucnv_to_utf32(
    cnv: *mut libc::c_void,
    target: *mut libc::c_char,
    target_capacity: i32,
    source: *const libc::c_char,
    source_length: i32,
) -> (i32, libc::c_int) {
    let mut err: i32 = 0;
    let len = ucnv_toAlgorithmic(
        UCNV_UTF32_NATIVE_ENDIAN,
        cnv,
        target,
        target_capacity,
        source,
        source_length,
        &mut err,
    );
    (len, err)
}

pub(crate) fn name_of_file_ptr(st: &EngineState) -> *const libc::c_char {
    if st.input.name_of_file.is_empty() {
        std::ptr::null()
    } else {
        st.input.name_of_file.as_ptr()
    }
}

pub(crate) unsafe fn tt_xetex_open_input_in(st: &mut EngineState, filefmt: libc::c_int) -> usize {
    let handle = if filefmt == TTBC_FILE_FORMAT_TECTONIC_PRIMARY {
        ttstub_input_open_primary()
    } else if *name_of_file_ptr(st) == b'|' as libc::c_char {
        crate::output::print_nl_cstr_in(st, c"Warning: ".as_ptr());
        crate::output::diagnostic_begin_capture_warning_here_in(st);
        crate::output::print_cstr_in(
            st,
            c"piped inputs from external commands are not implemented in Tectonic".as_ptr(),
        );
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        return INVALID_HANDLE;
    } else {
        ttstub_input_open(name_of_file_ptr(st), filefmt, 0)
    };

    if handle == INVALID_HANDLE {
        return INVALID_HANDLE;
    }

    let abspath = &mut st.input.abspath_of_input_file.0;
    if ttstub_get_last_input_abspath(abspath.as_mut_ptr(), abspath.len()) < 1 {
        abspath[0] = 0;
    }

    let name = std::ffi::CStr::from_ptr(name_of_file_ptr(st));
    st.input.name_length = name.to_bytes().len() as i32;
    st.input.name_of_input_file = Some(name.to_owned());
    handle
}

/// Opens and configures an input file. `None` means the bridge open failed.
pub(crate) unsafe fn u_open_in_in(
    st: &mut EngineState,
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
        } else if b1 == 0xEF && b2 == 0xBB && ttstub_input_getc(f.handle) == 0xBF {
            mode = UTF8;
        }

        if mode == AUTO {
            input_seek(f.handle, 0, libc::SEEK_SET);
            mode = UTF8;
        }
    }

    set_input_file_encoding_in(st, &mut *f, mode, encoding_data)?;
    Ok(Some(f))
}

/// Sets the encoding mode, and the ICU converter if any, of an open file.
pub(crate) unsafe fn set_input_file_encoding_in(
    st: &mut EngineState,
    f: *mut UFile,
    mode: i32,
    encoding_data: i32,
) -> Result<(), crate::texmfmp::MakeTexStringAbort> {
    let f = &mut *f;
    if f.encoding_mode == ICUMAPPING as i16 && !f.conversion_data.is_null() {
        xetex_ucnv_close(f.conversion_data);
    }
    f.conversion_data = std::ptr::null_mut();

    match mode {
        UTF8 | UTF16BE | UTF16LE | RAW => {
            f.encoding_mode = mode as i16;
        }
        ICUMAPPING => {
            let name = texstring_cstring(&st.strings, encoding_data);
            let mut err: libc::c_int = 0;
            let cnv = xetex_ucnv_open(name.as_ptr(), &mut err);
            if cnv.is_null() {
                crate::output::begin_diagnostic_in(st);
                crate::output::print_nl_in(st, b'E' as i32);
                crate::output::print_cstr_in(st, c"rror ".as_ptr());
                crate::output::print_int_in(st, err);
                crate::output::print_cstr_in(st, c" creating Unicode converter for `".as_ptr());
                crate::output::print_cstr_in(st, name.as_ptr());
                crate::output::print_cstr_in(st, c"'; reading as raw bytes".as_ptr());
                crate::output::end_diagnostic_in(st, true);
                f.encoding_mode = RAW as i16;
            } else {
                f.encoding_mode = ICUMAPPING as i16;
                f.conversion_data = cnv;
            }
        }
        _ => {}
    }
    Ok(())
}

// Process-lifetime staging buffers and normalizers for `input_line_in`.
static mut BYTE_BUFFER: *mut libc::c_char = std::ptr::null_mut();
static mut UTF32_BUF: *mut u32 = std::ptr::null_mut();
static mut NORMALIZERS: [*mut libc::c_void; 2] = [std::ptr::null_mut(), std::ptr::null_mut()];

unsafe fn utf32_buf(buf_size: i32) -> *mut u32 {
    if UTF32_BUF.is_null() {
        UTF32_BUF = xcalloc(buf_size as libc::size_t, std::mem::size_of::<u32>()) as *mut u32;
    }
    UTF32_BUF
}

fn is_line_end(c: i32) -> bool {
    c == EOF || c == b'\n' as i32 || c == b'\r' as i32
}

/// An EOF that is not an interrupted read.
unsafe fn is_true_eof(c: i32) -> bool {
    c == EOF && *crate::compat::errno_location() != libc::EINTR
}

fn line_too_long() -> InputAbort {
    InputAbort::TtAbort(c"unable to read an entire line (buffer too small)".as_ptr())
}

fn conversion_error_in(st: &mut EngineState, errcode: i32) {
    crate::output::begin_diagnostic_in(st);
    crate::output::print_nl_in(st, b'U' as i32);
    crate::output::print_cstr_in(st, c"nicode conversion failed (ICU error code = ".as_ptr());
    crate::output::print_int_in(st, errcode);
    crate::output::print_cstr_in(st, c") discarding any remaining text".as_ptr());
    crate::output::end_diagnostic_in(st, true);
}

/// Normalizes `len` UTF-32 units from `buf` into `buffer[first..]` (NFC for
/// `norm` 1, NFD for 2) and sets `last`.
unsafe fn apply_normalization_in(
    st: &mut EngineState,
    buf: *mut u32,
    len: i32,
    norm: i32,
) -> Result<(), InputAbort> {
    let normalizer = &mut NORMALIZERS[(norm - 1) as usize];
    if normalizer.is_null() {
        let target_form = NATIVE_UTF32 | if norm == 1 { K_FORM_NFC } else { K_FORM_NFD };
        let status = TECkit_CreateConverter(
            std::ptr::null(),
            0,
            1,
            NATIVE_UTF32,
            target_form,
            normalizer as *mut *mut libc::c_void,
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
        *normalizer,
        buf as *const u8,
        (len * 4) as u32,
        &mut in_used,
        out_ptr,
        out_cap,
        &mut out_used,
        1,
    );
    TECkit_ResetConverter(*normalizer);
    if status != K_STATUS_NO_ERROR {
        return Err(line_too_long());
    }
    st.input.last = st.input.first + (out_used / 4) as i32;
    Ok(())
}

/// Reads one line of `f` into `buffer[first..last]`, decoding and optionally
/// normalizing it. Returns 0 at end of file.
pub(crate) unsafe fn input_line_in(
    st: &mut EngineState,
    f: *mut UFile,
) -> Result<libc::c_int, InputAbort> {
    let norm = if st.eqtb.data.is_empty() {
        0
    } else {
        st.eqtb.int_par(INT_PAR_XETEX_INPUT_NORMALIZATION)
    };

    if (*f).handle == INVALID_HANDLE {
        return Err(InputAbort::TtAbort(
            c"reads from synthetic \"terminal\" file #0 should never happen".as_ptr(),
        ));
    }

    st.input.last = st.input.first;

    let mut i;
    if (*f).encoding_mode as i32 == ICUMAPPING {
        let mut bytes_read: u32 = 0;

        if BYTE_BUFFER.is_null() {
            BYTE_BUFFER = xmalloc((st.input.buf_size + 1) as libc::size_t) as *mut libc::c_char;
        }

        i = ttstub_input_getc((*f).handle);
        if (*f).skip_next_lf != 0 {
            (*f).skip_next_lf = 0;
            if i == b'\n' as libc::c_int {
                i = ttstub_input_getc((*f).handle);
            }
        }

        if !is_line_end(i) {
            *BYTE_BUFFER.add(bytes_read as usize) = i as libc::c_char;
            bytes_read += 1;
            while bytes_read < st.input.buf_size as u32 {
                i = ttstub_input_getc((*f).handle);
                if is_line_end(i) {
                    break;
                }
                *BYTE_BUFFER.add(bytes_read as usize) = i as libc::c_char;
                bytes_read += 1;
            }
        }

        if is_true_eof(i) && bytes_read == 0 {
            return Ok(0);
        }
        if !is_line_end(i) {
            return Err(line_too_long());
        }

        let cnv = (*f).conversion_data;
        if norm == 1 || norm == 2 {
            let buf = utf32_buf(st.input.buf_size);
            let (len, error_code) = xetex_ucnv_to_utf32(
                cnv,
                buf as *mut libc::c_char,
                st.input.buf_size * 4,
                BYTE_BUFFER,
                bytes_read as i32,
            );
            if error_code != 0 {
                conversion_error_in(st, error_code);
                return Ok(0);
            }
            apply_normalization_in(st, buf, len / 4, norm)?;
        } else {
            let out_ptr = st.input.buffer.as_mut_ptr().add(st.input.first as usize);
            let (len, error_code) = xetex_ucnv_to_utf32(
                cnv,
                out_ptr as *mut libc::c_char,
                4 * (st.input.buf_size - st.input.first),
                BYTE_BUFFER,
                bytes_read as i32,
            );
            if error_code != 0 {
                conversion_error_in(st, error_code);
                return Ok(0);
            }
            st.input.last = st.input.first + len / 4;
        }
    } else {
        i = get_uni_c_in(st, f)?;
        if (*f).skip_next_lf != 0 {
            (*f).skip_next_lf = 0;
            if i == b'\n' as i32 {
                i = get_uni_c_in(st, f)?;
            }
        }

        if norm == 1 || norm == 2 {
            let buf = utf32_buf(st.input.buf_size);
            let mut len: i32 = 0;
            if !is_line_end(i) {
                *buf.add(len as usize) = i as u32;
                len += 1;
                while len < st.input.buf_size {
                    i = get_uni_c_in(st, f)?;
                    if is_line_end(i) {
                        break;
                    }
                    *buf.add(len as usize) = i as u32;
                    len += 1;
                }
            }

            if is_true_eof(i) && len == 0 {
                return Ok(0);
            }
            if !is_line_end(i) {
                return Err(line_too_long());
            }
            apply_normalization_in(st, buf, len, norm)?;
        } else {
            if st.input.last < st.input.buf_size && !is_line_end(i) {
                st.input.buffer[st.input.last as usize] = i;
                st.input.last += 1;
                while st.input.last < st.input.buf_size {
                    i = get_uni_c_in(st, f)?;
                    if is_line_end(i) {
                        break;
                    }
                    st.input.buffer[st.input.last as usize] = i;
                    st.input.last += 1;
                }
            }

            if is_true_eof(i) && st.input.last == st.input.first {
                return Ok(0);
            }
            if !is_line_end(i) {
                return Err(line_too_long());
            }
        }
    }

    if i == b'\r' as i32 {
        (*f).skip_next_lf = 1;
    }

    st.input.buffer[st.input.last as usize] = b' ' as i32;
    if st.input.last >= st.input.max_buf_stack {
        st.input.max_buf_stack = st.input.last;
    }

    while st.input.last > st.input.first {
        let c = st.input.buffer[(st.input.last - 1) as usize];
        if c != b' ' as i32 && c != b'\r' as i32 && c != b'\n' as i32 {
            break;
        }
        st.input.last -= 1;
    }

    Ok(1)
}

/// Converts `name_of_file` (UTF-8) to UTF-16 in `name_of_file16`.
pub(crate) unsafe fn make_utf16_name_in(st: &mut EngineState) {
    let name_len = st.input.name_length as usize;
    let needed = name_len + 10;
    if st.input.name_of_file16.len() < needed {
        st.input.name_of_file16.resize(needed, 0);
    }
    let mut s = st.input.name_of_file.as_ptr() as *const u8;
    let end = s.add(name_len);
    let mut t = 0;
    while s < end {
        let (units, n) = utf16_units(next_utf8_char(&mut s));
        st.input.name_of_file16[t..t + n].copy_from_slice(&units[..n]);
        t += n;
    }
    st.input.name_length16 = t as i32;
}

/// The only abort of [`get_uni_c_in`]. Being fieldless keeps the per-character
/// `Result` in registers on this hot path.
pub(crate) struct InvalidInputMode;

impl From<InvalidInputMode> for InputAbort {
    fn from(_: InvalidInputMode) -> Self {
        InputAbort::TtAbort(c"internal error; file input mode is invalid".as_ptr())
    }
}

/// Reads one code point from `f` in its encoding mode.
pub(crate) unsafe fn get_uni_c_in(
    st: &mut EngineState,
    f: *mut UFile,
) -> Result<libc::c_int, InvalidInputMode> {
    let f = &mut *f;
    if f.saved_char != -1 {
        let rval = f.saved_char as i32;
        f.saved_char = -1;
        return Ok(rval);
    }

    let handle = f.handle;
    let rval = match f.encoding_mode as i32 {
        UTF8 => {
            let mut rval = ttstub_input_getc(handle);
            if rval != EOF {
                let extra_bytes = crate::utf8::BYTES_FROM_UTF8[rval as usize] as usize;
                if extra_bytes >= 4 {
                    crate::output::bad_utf8_warning_in(st);
                    return Ok(0xFFFD);
                }
                for _ in 0..extra_bytes {
                    let c = ttstub_input_getc(handle);
                    if !(0x80..0xC0).contains(&c) {
                        if c != EOF {
                            ttstub_input_ungetc(handle, c);
                        }
                        crate::output::bad_utf8_warning_in(st);
                        return Ok(0xFFFD);
                    }
                    rval = (rval << 6) + c;
                }
                let offset = crate::utf8::OFFSETS_FROM_UTF8[extra_bytes];
                rval = (rval as u32).wrapping_sub(offset) as i32;
                if !(0..=0x10ffff).contains(&rval) {
                    crate::output::bad_utf8_warning_in(st);
                    return Ok(0xFFFD);
                }
            }
            rval
        }

        mode @ (UTF16BE | UTF16LE) => {
            let unit = |first: i32| {
                let second = ttstub_input_getc(handle);
                if mode == UTF16BE {
                    (first << 8) + second
                } else {
                    first + (second << 8)
                }
            };
            let mut rval = ttstub_input_getc(handle);
            if rval != EOF {
                rval = unit(rval);
                if (0xd800..=0xdbff).contains(&rval) {
                    let lo = unit(ttstub_input_getc(handle));
                    if (0xdc00..=0xdfff).contains(&lo) {
                        rval = 0x10000 + (rval - 0xd800) * 0x400 + (lo - 0xdc00);
                    } else {
                        rval = 0xfffd;
                        f.saved_char = lo as i64;
                    }
                } else if (0xdc00..=0xdfff).contains(&rval) {
                    rval = 0xfffd;
                }
            }
            rval
        }

        RAW => ttstub_input_getc(handle),

        _ => return Err(InvalidInputMode),
    };

    Ok(rval)
}

/// Closes an input slot. Returns `true` on a serious internal close error, in
/// which case the converter and the file are leaked like the old abort did.
#[must_use]
pub unsafe fn u_close(slot: crate::state::InputSlot) -> bool {
    let crate::state::InputSlot::Open(f) = slot else {
        return false;
    };

    if f.handle == INVALID_HANDLE {
        std::mem::forget(f);
        return false;
    }

    if input_close(f.handle) {
        std::mem::forget(f);
        return true;
    }

    drop(f);
    false
}

pub(crate) enum OpenOrCloseAbort {
    Scanner(SccAbort),
    Name(NameAbort),
    MakeString(crate::texmfmp::MakeTexStringAbort),
    /// A serious internal error closing a `\read` stream.
    InputClose,
}

impl OpenOrCloseAbort {
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

/// Runs `name_of_file` back through the name scanner so that `cur_name`,
/// `cur_area` and `cur_ext` describe the file actually opened.
unsafe fn rescan_opened_name_in(st: &mut EngineState) -> Result<(), NameAbort> {
    make_utf16_name_in(st);
    st.scanner.name_in_progress = true;
    begin_name_in(st);
    st.scanner.stop_at_space = false;
    for k in 0..st.input.name_length16 as usize {
        if !more_name_in(st, st.input.name_of_file16[k])? {
            break;
        }
    }
    st.scanner.stop_at_space = true;
    end_name_in(st)?;
    st.scanner.name_in_progress = false;
    Ok(())
}

/// `\openin` and `\closein`.
pub(crate) unsafe fn open_or_close_in(st: &mut EngineState) -> Result<(), OpenOrCloseAbort> {
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

    if c == 0 {
        return Ok(());
    }

    crate::scan::scan_optional_equals_in(st).map_err(OpenOrCloseAbort::Scanner)?;
    scan_and_pack_name_in(st).map_err(OpenOrCloseAbort::Scanner)?;

    let mode = st.eqtb.int_par(INT_PAR_XETEX_DEFAULT_INPUT_MODE);
    let encoding = st
        .eqtb
        .b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_ENCODING);
    if let Some(file) = u_open_in_in(st, TTBC_FILE_FORMAT_TEX, mode, encoding)
        .map_err(OpenOrCloseAbort::MakeString)?
    {
        st.input.read_file[n] = crate::state::InputSlot::Open(file);
        rescan_opened_name_in(st).map_err(OpenOrCloseAbort::Name)?;
        st.input.read_open[n] = JUST_OPEN;
    }

    Ok(())
}

pub(crate) fn begin_name_in(st: &mut EngineState) {
    st.scanner.area_delimiter = 0;
    st.scanner.ext_delimiter = 0;
    st.scanner.quoted_filename = false;
    st.scanner.file_name_quote_char = 0;
}

/// Overflows of the file-name machinery.
pub(crate) enum NameAbort {
    PoolSize,
    NumberOfStrings,
}

impl NameAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            NameAbort::PoolSize => crate::errors::EngineAbort::PoolOverflow,
            NameAbort::NumberOfStrings => crate::errors::EngineAbort::StringOverflow,
        }
    }
}

pub(crate) fn more_name_in(st: &mut EngineState, c: u16) -> Result<bool, NameAbort> {
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
    if c == b'/' as u16 {
        st.scanner.area_delimiter = cur_length(&st.strings);
        st.scanner.ext_delimiter = 0;
    } else if c == b'.' as u16 {
        st.scanner.ext_delimiter = cur_length(&st.strings);
    }
    Ok(true)
}

pub(crate) fn end_name_in(st: &mut EngineState) -> Result<(), NameAbort> {
    use crate::string_pool::{make_string, search_string, slow_make_string};

    if st.strings.str_ptr + 3 > st.strings.max_strings {
        return Err(NameAbort::NumberOfStrings);
    }

    let area_delimiter = st.scanner.area_delimiter;
    if area_delimiter == 0 {
        st.scanner.cur_area = EMPTY_STRING;
    } else {
        let strings = &mut st.strings;
        st.scanner.cur_area = strings.str_ptr;
        let start = strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize];
        strings.str_start[(strings.str_ptr + 1 - TOO_BIG_CHAR) as usize] = start + area_delimiter;
        strings.str_ptr += 1;
        let temp_str = search_string(strings, st.scanner.cur_area);
        if temp_str > 0 {
            st.scanner.cur_area = temp_str;
            strings.str_ptr -= 1;
            let base = strings.str_start[(strings.str_ptr + 1 - TOO_BIG_CHAR) as usize];
            for j in base..strings.pool_ptr {
                strings.str_pool[(j - area_delimiter) as usize] = strings.str_pool[j as usize];
            }
            strings.pool_ptr -= area_delimiter;
        }
    }

    if st.scanner.ext_delimiter == 0 {
        st.scanner.cur_ext = EMPTY_STRING;
        st.scanner.cur_name =
            slow_make_string(&mut st.strings).map_err(|_| NameAbort::NumberOfStrings)?;
    } else {
        let strings = &mut st.strings;
        let shift = st.scanner.ext_delimiter - area_delimiter - 1;
        st.scanner.cur_name = strings.str_ptr;
        let start = strings.str_start[(strings.str_ptr - TOO_BIG_CHAR) as usize];
        strings.str_start[(strings.str_ptr + 1 - TOO_BIG_CHAR) as usize] = start + shift;
        strings.str_ptr += 1;

        st.scanner.cur_ext = make_string(strings).map_err(|_| NameAbort::NumberOfStrings)?;
        strings.str_ptr -= 1;
        let temp_str = search_string(strings, st.scanner.cur_name);
        if temp_str > 0 {
            st.scanner.cur_name = temp_str;
            strings.str_ptr -= 1;
            let base = strings.str_start[(strings.str_ptr + 1 - TOO_BIG_CHAR) as usize];
            for j in base..strings.pool_ptr {
                strings.str_pool[(j - shift) as usize] = strings.str_pool[j as usize];
            }
            strings.pool_ptr -= shift;
        }
        st.scanner.cur_ext = slow_make_string(strings).map_err(|_| NameAbort::NumberOfStrings)?;
    }
    Ok(())
}

pub(crate) unsafe fn make_name_string_in(st: &mut EngineState) -> Result<i32, NameAbort> {
    if st.strings.pool_ptr + st.input.name_length > st.strings.pool_size
        || st.strings.str_ptr == st.strings.max_strings
        || cur_length(&st.strings) > 0
    {
        return Ok(b'?' as i32);
    }

    make_utf16_name_in(st);

    let name_len16 = st.input.name_length16 as usize;
    let pool_ptr = st.strings.pool_ptr as usize;
    st.strings.str_pool[pool_ptr..pool_ptr + name_len16]
        .copy_from_slice(&st.input.name_of_file16[..name_len16]);
    st.strings.pool_ptr += name_len16 as i32;

    let result =
        crate::string_pool::make_string(&mut st.strings).map_err(|_| NameAbort::NumberOfStrings)?;
    let save_area_delimiter = st.scanner.area_delimiter;
    let save_ext_delimiter = st.scanner.ext_delimiter;
    let save_name_in_progress = st.scanner.name_in_progress;
    let save_stop_at_space = st.scanner.stop_at_space;

    st.scanner.name_in_progress = true;
    begin_name_in(st);
    st.scanner.stop_at_space = false;

    for k in 0..name_len16 {
        if !more_name_in(st, st.input.name_of_file16[k])? {
            break;
        }
    }

    st.scanner.stop_at_space = save_stop_at_space;
    end_name_in(st)?;
    st.scanner.name_in_progress = save_name_in_progress;
    st.scanner.area_delimiter = save_area_delimiter;
    st.scanner.ext_delimiter = save_ext_delimiter;
    Ok(result)
}

pub(crate) unsafe fn pack_job_name_in(
    st: &mut EngineState,
    s: *const libc::c_char,
) -> Result<(), crate::texmfmp::MakeTexStringAbort> {
    st.scanner.cur_area = EMPTY_STRING;
    st.scanner.cur_ext = crate::texmfmp::maketexstring_in(&mut st.strings, s)?;
    st.scanner.cur_name = st.strings.job_name;
    pack_file_name_in(
        st,
        st.scanner.cur_name,
        st.scanner.cur_area,
        st.scanner.cur_ext,
    );
    Ok(())
}

pub(crate) enum OpenLogAbort {
    MakeString(crate::texmfmp::MakeTexStringAbort),
    Name(NameAbort),
    /// The host refused to open the log file.
    CannotOpen,
}

impl OpenLogAbort {
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

/// Opens the transcript and writes its banner.
pub(crate) unsafe fn open_log_file_in(st: &mut EngineState) -> Result<(), OpenLogAbort> {
    let old_setting = st.shipout.selector;
    if st.strings.job_name == 0 {
        st.strings.job_name = crate::texmfmp::maketexstring_in(&mut st.strings, c"texput".as_ptr())
            .map_err(OpenLogAbort::MakeString)?;
    }

    pack_job_name_in(st, c".log".as_ptr()).map_err(OpenLogAbort::MakeString)?;

    let handle = ttstub_output_open(name_of_file_ptr(st), 0);
    if handle == INVALID_HANDLE {
        return Err(OpenLogAbort::CannotOpen);
    }
    st.shipout.log_file = handle;

    st.strings.texmf_log_name = make_name_string_in(st).map_err(OpenLogAbort::Name)?;
    st.shipout.selector = SELECTOR_LOG_ONLY;
    st.errors.log_opened = true;

    st.input.input_stack[st.input.input_ptr as usize] = st.input.cur_input;

    crate::output::print_nl_cstr_in(st, c"**".as_ptr());
    let mut l = st.input.input_stack[0].limit;
    if st.input.buffer[l as usize] == st.eqtb.int_par(INT_PAR_END_LINE_CHAR) {
        l -= 1;
    }
    for k in 1..=l {
        crate::output::print_in(st, st.input.buffer[k as usize]);
    }

    crate::output::print_ln_in(st);
    st.shipout.selector = old_setting + 2;
    Ok(())
}

/// Recreates bridge-owned output handles after restoring a resident checkpoint.
///
/// Checkpoints keep the logical output state, but bridge handles and
/// diagnostic pointers are valid only for the operation that created them.
pub(crate) unsafe fn reopen_resident_outputs_in(
    st: &mut EngineState,
) -> Result<(), crate::errors::EngineAbort> {
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

/// Builds `name_of_file` and `name_length` from area `a`, name `n` and
/// extension `e`.
pub(crate) fn pack_file_name_in(st: &mut EngineState, n: i32, a: i32, e: i32) {
    let name = &mut st.input.name_of_file;
    name.clear();
    for part in [a, n, e] {
        let part = texstring_cstring(&st.strings, part);
        name.extend(part.as_bytes().iter().map(|&b| b as libc::c_char));
    }
    st.input.name_length = name.len() as i32;
    name.push(0);
}

pub(crate) unsafe fn close_files_and_terminate_in(
    st: &mut EngineState,
) -> Result<(), crate::errors::EngineAbort> {
    for k in 0usize..=15 {
        if st.shipout.write_open[k] {
            crate::shipout::ensure_write_stream_open(st, k)
                .map_err(|error| error.into_engine_abort())?;
            ttstub_output_close(st.shipout.write_file[k]);
        }
    }

    // Routed through the journaling setter so a rewind restores `\newlinechar`.
    st.eqtb.set_int_par(INT_PAR_NEW_LINE_CHAR, -1);

    crate::shipout::finalize_dvi_file_in(st).map_err(|e| e.into_engine_abort())?;
    crate::synctex::synctex_terminate(st);

    if st.errors.log_opened {
        let log_file = st.shipout.log_file;
        ttstub_output_putc(log_file, b'\n' as i32);
        ttstub_output_close(log_file);
        st.shipout.selector -= 2;
        if st.shipout.selector == SELECTOR_TERM_ONLY {
            crate::output::print_nl_cstr_in(st, c"Transcript written on ".as_ptr());
            crate::output::print_in(st, st.strings.texmf_log_name);
            crate::output::print_char_in(st, b'.' as i32);
        }
    }

    crate::output::print_ln_in(st);
    Ok(())
}

pub unsafe fn init_io(st: &mut EngineState) {
    STDIN_UFILE.handle = INVALID_HANDLE;
    STDIN_UFILE.saved_char = -1;
    STDIN_UFILE.skip_next_lf = 0;
    STDIN_UFILE.encoding_mode = UTF8 as i16;
    STDIN_UFILE.conversion_data = std::ptr::null_mut();

    let first = st.input.first;
    st.input.input_file[0] = crate::state::InputSlot::Terminal;
    st.input.buffer[first as usize] = 0;
    st.input.last = first;
    st.input.cur_input.loc = first;
    st.input.cur_input.limit = first;
    st.input.first = first + 1;
}

/// Lifts a file-name overflow into [`SccAbort`], sized from the live pool.
fn name_abort_scc(st: &EngineState, e: NameAbort) -> SccAbort {
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
    SccAbort::Input(InputAbort::Overflow { what, n })
}

pub(crate) fn scan_and_pack_name_in(st: &mut EngineState) -> Result<(), SccAbort> {
    scan_file_name_in(st)?;
    pack_file_name_in(
        st,
        st.scanner.cur_name,
        st.scanner.cur_area,
        st.scanner.cur_ext,
    );
    Ok(())
}

pub(crate) fn scan_file_name_braced_in(st: &mut EngineState) -> Result<(), SccAbort> {
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
    crate::mem::delete_token_ref(&mut st.mem, st.scanner.def_ref);
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

pub(crate) fn scan_file_name_in(st: &mut EngineState) -> Result<(), SccAbort> {
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

/// `\input`, or the primary input when `primary_input_name` is non-null.
pub(crate) unsafe fn start_input_in(
    st: &mut EngineState,
    primary_input_name: *const libc::c_char,
) -> Result<(), SccAbort> {
    let format = if primary_input_name.is_null() {
        TTBC_FILE_FORMAT_TEX
    } else {
        TTBC_FILE_FORMAT_TECTONIC_PRIMARY
    };

    if primary_input_name.is_null() {
        scan_file_name_in(st)?;
    } else {
        st.scanner.name_in_progress = true;
        begin_name_in(st);
        st.scanner.stop_at_space = false;

        if st.strings.pool_ptr + libc::strlen(primary_input_name) as i32 * 2 >= st.strings.pool_size
        {
            return Err(SccAbort::Input(InputAbort::TtAbort(
                c"string pool overflow".as_ptr(),
            )));
        }

        let mut cp = primary_input_name as *const u8;
        while *cp != 0 {
            let rval = next_utf8_char(&mut cp);
            push_utf16(&mut st.strings, rval);
            if rval == b'/' as u32 {
                st.scanner.area_delimiter = cur_length(&st.strings);
                st.scanner.ext_delimiter = 0;
            } else if rval == b'.' as u32 {
                st.scanner.ext_delimiter = cur_length(&st.strings);
            }
        }

        st.scanner.stop_at_space = true;
        end_name_in(st).map_err(|e| name_abort_scc(st, e))?;
        st.scanner.name_in_progress = false;
    }

    pack_file_name_in(
        st,
        st.scanner.cur_name,
        st.scanner.cur_area,
        st.scanner.cur_ext,
    );
    crate::input::begin_file_reading_in(st)?;

    let idx = st.input.cur_input.index as i32;
    let mode = st.eqtb.int_par(INT_PAR_XETEX_DEFAULT_INPUT_MODE);
    let encoding = st
        .eqtb
        .b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_ENCODING);
    let Some(file) = u_open_in_in(st, format, mode, encoding).map_err(SccAbort::MakeTexString)?
    else {
        // The abort keeps a pointer to the message, so the buffer is leaked.
        let buf = libc::malloc(512) as *mut libc::c_char;
        crate::compat::snprintf(
            buf,
            512,
            c"failed to open input file \"%s\"".as_ptr(),
            name_of_file_ptr(st),
        );
        return Err(SccAbort::Input(InputAbort::TtAbort(buf)));
    };
    st.input.input_file[idx as usize] = crate::state::InputSlot::Open(file);

    rescan_opened_name_in(st).map_err(|e| name_abort_scc(st, e))?;

    let name = make_name_string_in(st).map_err(|e| name_abort_scc(st, e))?;
    st.input.cur_input.name = name;
    let in_open = st.input.in_open as usize;
    st.input.source_filename_stack[in_open] = name;
    let full_name = st
        .input
        .name_of_input_file
        .as_deref()
        .map_or(std::ptr::null(), |name| name.as_ptr());
    let full_name = crate::texmfmp::maketexstring_in(&mut st.strings, full_name)
        .map_err(SccAbort::MakeTexString)?;
    st.input.full_source_filename_stack[in_open] = full_name;

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
        open_log_file_in(st).map_err(SccAbort::OpenLog)?;
    }

    if st.shipout.term_offset + crate::string_pool::length(&st.strings, full_name)
        > st.config.max_print_line - 2
    {
        crate::output::print_ln_in(st);
    } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
        crate::output::print_char_in(st, b' ' as i32);
    }
    crate::output::print_char_in(st, b'(' as i32);
    st.input.open_parens += 1;
    crate::output::print_in(st, full_name);
    ttstub_output_flush(st.shipout.rust_stdout);

    let tracing_stack_levels = st.eqtb.int_par(INT_PAR_TRACING_STACK_LEVELS);
    if tracing_stack_levels > 0 {
        crate::output::begin_diagnostic_in(st);
        crate::output::diagnostic_begin_capture_warning_here_in(st);
        crate::output::print_ln_in(st);
        crate::output::print_char_in(st, b'~' as i32);
        let v = st.input.input_ptr - 1;
        if v < tracing_stack_levels {
            for _ in 0..v {
                crate::output::print_char_in(st, b'.' as i32);
            }
        } else {
            crate::output::print_char_in(st, b'~' as i32);
        }
        crate::output::print_cstr_in(st, c"INPUT ".as_ptr());
        crate::output::print_in(st, st.scanner.cur_name);
        crate::output::print_in(st, st.scanner.cur_ext);
        crate::output::print_ln_in(st);
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        crate::output::end_diagnostic_in(st, false);
    }

    st.input.cur_input.state = NEW_LINE;
    let synctex_job_name = crate::texmfmp::gettexstring_in(&st.strings, st.strings.job_name);
    crate::synctex::synctex_start_input(st, synctex_job_name);
    st.mem.line = 1;
    let f = st.input.input_file_ptr(idx);
    input_line_in(st, f)?;
    st.input.cur_input.limit = st.input.last;
    crate::scan::finish_line_in(st);
    Ok(())
}
