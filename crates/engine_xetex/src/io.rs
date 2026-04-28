//! Rust translation of xetex-io.c: low-level input/output helpers.
//!
//! Complex functions that depend on ICU (input_line, set_input_file_encoding,
//! u_open_in) remain in C for now.

use crate::globals::{
    cur_area, cur_chr, cur_ext, cur_name, cur_val, eqtb_b32s1, name_length, name_of_file,
    name_of_file16, name_of_file16_entry, name_length16, read_file_entry, read_file_ptr,
    read_open_entry, set_name_in_progress, set_name_length, set_name_length16,
    set_name_of_file16, set_read_open_entry, set_stop_at_space,
};

// ---------------------------------------------------------------------------
// UFILE — Rust mirror of the C `UFILE` struct (xetex-io.h:12–18)
// ---------------------------------------------------------------------------

/// Rust mirror of `UFILE` (xetex-io.h:12–18). Must stay layout-compatible.
///
/// On Linux x86_64 the C layout is:
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
    pub saved_char: i64,  // C `long` = i64 on Linux 64-bit
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
// const ICUMAPPING: i32 = 5;  // handled in C (set_input_file_encoding)

// File format constants (tectonic_bridge_core_generated.h).
const TTBC_FILE_FORMAT_TECTONIC_PRIMARY: libc::c_int = 59;
const TTBC_FILE_FORMAT_TEX: libc::c_int = 26;
// INVALID_HANDLE = 0
const INVALID_HANDLE: usize = 0;

// Constants for open_or_close_in (xetex-constants.h:268,277).
const JUST_OPEN: u8 = 1;
const CLOSED: u8 = 2;

// INT_BASE and INT_PAR offsets used by open_or_close_in (xetex_format.h:219,110–111).
const INT_BASE: i32 = 7_826_729;
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
    fn ttstub_get_last_input_abspath(buffer: *mut libc::c_char, len: libc::size_t) -> libc::ssize_t;
    fn ttstub_input_close(handle: usize) -> libc::c_int;
    fn ttstub_input_seek(handle: usize, offset: libc::ssize_t, whence: libc::c_int) -> libc::size_t;
    fn ttstub_input_getc(handle: usize) -> libc::c_int;
    fn ttstub_input_ungetc(handle: usize, ch: libc::c_int) -> libc::c_int;

    // set_input_file_encoding remains in C; called from u_open_in via FFI.
    // u_close also remains in C (ucnv_close from ICU is not directly linkable
    // from Rust without adding cargo:rustc-link-lib=icuuc to the build).
    fn set_input_file_encoding(f: *mut UFile, mode: i32, encoding_data: i32);
    fn u_close(f: *mut UFile);

    // bad_utf8_warning remains in C (xetex-xetexd.h:912).
    fn bad_utf8_warning();

    // Callees of open_or_close_in still in C.
    fn scan_four_bit_int();
    fn scan_optional_equals();
    fn scan_file_name();
    fn pack_file_name(n: i32, a: i32, e: i32);
    fn begin_name();
    fn more_name(c: u16) -> bool;
    fn end_name();

    // Output helpers — all translated to Rust in output.rs.
    fn print_nl_cstr(s: *const libc::c_char);
    fn diagnostic_begin_capture_warning_here();
    fn print_cstr(s: *const libc::c_char);
    fn capture_to_diagnostic(diagnostic: *mut libc::c_void);
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

/// Static local from make_utf16_name: tracks the allocated size of
/// `name_of_file16`.
static mut NAME16LEN: i32 = 0;

// ---------------------------------------------------------------------------
// tt_xetex_open_input
// ---------------------------------------------------------------------------

/// Open the current input file (named by `name_of_file`) via the bridge.
///
/// translated from xetex-io.c:22 (tt_xetex_open_input)
#[no_mangle]
pub unsafe extern "C" fn tt_xetex_open_input(filefmt: libc::c_int) -> usize {
    let handle = if filefmt == TTBC_FILE_FORMAT_TECTONIC_PRIMARY {
        ttstub_input_open_primary()
    } else if *name_of_file() == b'|' as libc::c_char {
        print_nl_cstr(c"Warning: ".as_ptr());
        diagnostic_begin_capture_warning_here();
        print_cstr(
            c"piped inputs from external commands are not implemented in Tectonic".as_ptr(),
        );
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
pub(crate) unsafe fn call_set_input_file_encoding(f: *mut libc::c_void, mode: i32, encoding_data: i32) {
    set_input_file_encoding(f as *mut UFile, mode, encoding_data);
}

// ---------------------------------------------------------------------------
// make_utf16_name
// ---------------------------------------------------------------------------

/// Convert `name_of_file` (UTF-8) to UTF-16 in `name_of_file16`.
///
/// translated from xetex-io.c:481 (make_utf16_name)
#[no_mangle]
pub unsafe extern "C" fn make_utf16_name() {
    if NAME16LEN <= name_length() {
        libc::free(name_of_file16() as *mut libc::c_void);
        NAME16LEN = name_length() + 10;
        set_name_of_file16(
            xcalloc(NAME16LEN as libc::size_t, std::mem::size_of::<u16>()) as *mut u16,
        );
    }
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
                    if c < 0x80 || c >= 0xC0 {
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
                if rval < 0 || rval > 0x10ffff {
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
                if rval >= 0xd800 && rval <= 0xdbff {
                    let mut lo = ttstub_input_getc((*f).handle);
                    lo <<= 8;
                    lo += ttstub_input_getc((*f).handle);
                    if lo >= 0xdc00 && lo <= 0xdfff {
                        rval = 0x10000 + (rval - 0xd800) * 0x400 + (lo - 0xdc00);
                    } else {
                        rval = 0xfffd;
                        (*f).saved_char = lo as i64;
                    }
                } else if rval >= 0xdc00 && rval <= 0xdfff {
                    rval = 0xfffd;
                }
            }
        }

        UTF16LE => {
            rval = ttstub_input_getc((*f).handle);
            if rval != libc::EOF {
                rval += ttstub_input_getc((*f).handle) << 8;
                if rval >= 0xd800 && rval <= 0xdbff {
                    let mut lo = ttstub_input_getc((*f).handle);
                    lo += ttstub_input_getc((*f).handle) << 8;
                    if lo >= 0xdc00 && lo <= 0xdfff {
                        rval = 0x10000 + (rval - 0xd800) * 0x400 + (lo - 0xdc00);
                    } else {
                        rval = 0xfffd;
                        (*f).saved_char = lo as i64;
                    }
                } else if rval >= 0xdc00 && rval <= 0xdfff {
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
