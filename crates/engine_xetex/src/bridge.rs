//! Engine-side wrappers over the `tectonic_bridge_core` I/O functions.
//!
//! Handles are raw `usize` values with 0 meaning invalid. The state pointer is
//! installed per run in `BRIDGE_PTR`.

use tectonic_bridge_core::{
    ttbc_diag_begin_error, ttbc_diag_begin_warning, ttbc_diag_finish, ttbc_get_file_md5,
    ttbc_get_last_input_abspath, ttbc_input_close, ttbc_input_get_mtime, ttbc_input_get_size,
    ttbc_input_getc, ttbc_input_open, ttbc_input_open_primary, ttbc_input_read, ttbc_input_seek,
    ttbc_input_ungetc, ttbc_issue_error, ttbc_issue_warning, ttbc_output_close, ttbc_output_flush,
    ttbc_output_open, ttbc_output_open_stdout, ttbc_output_putc, ttbc_output_write,
    ttbc_shell_escape, CoreBridgeState, Diagnostic, FileFormat, InputId, OutputId,
};

static mut BRIDGE_PTR: *mut libc::c_void = std::ptr::null_mut();

/// Installs the `CoreBridgeState` pointer for this run, or clears it with null.
pub(crate) unsafe fn set_bridge_state(ptr: *mut libc::c_void) {
    BRIDGE_PTR = ptr;
}

extern "C" {
    fn ttbc_global_engine_enter(api: *mut libc::c_void) -> *mut libc::c_void;
    fn ttbc_global_engine_exit();
}

/// Register a Rust-direct resident operation with bridge-core's C callbacks.
pub(crate) unsafe fn enter_global_engine(api: *mut libc::c_void) {
    let _ = ttbc_global_engine_enter(api);
}

/// Clear the callback state installed by [`enter_global_engine`].
pub(crate) unsafe fn exit_global_engine() {
    ttbc_global_engine_exit();
}

/// The bridge state for the current run. The engine is single-threaded and
/// none of the wrappers re-enter, so the exclusive borrow is short-lived and
/// unique.
#[inline]
unsafe fn es() -> &'static mut CoreBridgeState<'static> {
    debug_assert!(
        !BRIDGE_PTR.is_null(),
        "bridge state used outside tt_run_engine"
    );
    &mut *(BRIDGE_PTR as *mut CoreBridgeState<'static>)
}

/// Maps a `ttbc_file_format` number to its `FileFormat`.
fn file_format_from_int(format: libc::c_int) -> FileFormat {
    match format {
        1 => FileFormat::Pk,
        3 => FileFormat::Tfm,
        4 => FileFormat::Afm,
        6 => FileFormat::Bib,
        7 => FileFormat::Bst,
        8 => FileFormat::Cnf,
        10 => FileFormat::Format,
        11 => FileFormat::FontMap,
        20 => FileFormat::Ofm,
        23 => FileFormat::Ovf,
        25 => FileFormat::Pict,
        30 => FileFormat::TexPsHeader,
        32 => FileFormat::Type1,
        33 => FileFormat::Vf,
        36 => FileFormat::TrueType,
        39 => FileFormat::ProgramData,
        41 => FileFormat::MiscFonts,
        44 => FileFormat::Enc,
        45 => FileFormat::Cmap,
        46 => FileFormat::Sfd,
        47 => FileFormat::OpenType,
        59 => FileFormat::TectonicPrimary,
        _ => FileFormat::Tex, // 26 and the C wrapper's implicit default
    }
}

pub(crate) unsafe fn ttstub_input_open(
    path: *const libc::c_char,
    format: libc::c_int,
    is_gz: libc::c_int,
) -> usize {
    if let Some(handle) = crate::replay::adapter::open_named(path, format) {
        return handle;
    }
    crate::replay::adapter::clear_last_path();
    InputId::to_raw(ttbc_input_open(
        es(),
        path,
        file_format_from_int(format),
        is_gz,
    ))
}

pub(crate) unsafe fn ttstub_input_open_primary() -> usize {
    if let Some(handle) = crate::replay::adapter::open_primary() {
        return handle;
    }
    crate::replay::adapter::clear_last_path();
    InputId::to_raw(ttbc_input_open_primary(es()))
}

pub(crate) unsafe fn ttstub_get_last_input_abspath(
    buffer: *mut libc::c_char,
    len: libc::size_t,
) -> libc::ssize_t {
    if let Some(copied) = crate::replay::adapter::copy_last_abspath(buffer, len) {
        return copied;
    }
    ttbc_get_last_input_abspath(es(), buffer as *mut u8, len)
}

pub(crate) unsafe fn ttstub_input_get_size(handle: usize) -> libc::size_t {
    if let Some(size) = crate::replay::adapter::get_size(handle) {
        return size;
    }
    ttbc_input_get_size(es(), InputId::from_raw(handle))
}

pub(crate) unsafe fn ttstub_input_get_mtime(handle: usize) -> libc::time_t {
    if crate::replay::adapter::is_mem(handle) {
        return 0;
    }
    ttbc_input_get_mtime(es(), InputId::from_raw(handle)) as libc::time_t
}

pub(crate) unsafe fn ttstub_input_getc(handle: usize) -> libc::c_int {
    if let Some(value) = crate::replay::adapter::getc(handle) {
        return value;
    }
    ttbc_input_getc(es(), InputId::from_raw(handle))
}

pub(crate) unsafe fn ttstub_input_ungetc(handle: usize, ch: libc::c_int) -> libc::c_int {
    if let Some(value) = crate::replay::adapter::ungetc(handle, ch) {
        return value;
    }
    ttbc_input_ungetc(es(), InputId::from_raw(handle), ch)
}

pub(crate) unsafe fn ttstub_input_read(
    handle: usize,
    data: *mut u8,
    len: libc::size_t,
) -> libc::ssize_t {
    if let Some(value) = crate::replay::adapter::read(handle, data, len) {
        return value;
    }
    ttbc_input_read(es(), InputId::from_raw(handle), data, len)
}

/// `ttbc_input_seek` flags an error only for an invalid `whence`, and every
/// caller seeks with `SEEK_SET`, so the error path is unreachable.
pub(crate) unsafe fn input_seek(
    handle: usize,
    offset: libc::ssize_t,
    whence: libc::c_int,
) -> libc::size_t {
    if let Some(position) = crate::replay::adapter::seek(handle, offset, whence) {
        return position;
    }
    let mut internal_error: libc::c_int = 0;
    let rv = ttbc_input_seek(
        es(),
        InputId::from_raw(handle),
        offset,
        whence,
        &mut internal_error,
    );
    debug_assert_eq!(
        internal_error, 0,
        "ttbc_input_seek flags internal_error only for an invalid whence"
    );
    rv
}

/// Returns `true` on a serious internal close error.
#[must_use]
pub(crate) unsafe fn input_close(handle: usize) -> bool {
    if let Some(error) = crate::replay::adapter::close(handle) {
        return error;
    }
    ttbc_input_close(es(), InputId::from_raw(handle)) != 0
}

pub(crate) unsafe fn ttstub_output_open(path: *const libc::c_char, is_gz: libc::c_int) -> usize {
    OutputId::to_raw(ttbc_output_open(es(), path, is_gz))
}

pub(crate) unsafe fn ttstub_output_open_stdout() -> usize {
    OutputId::to_raw(ttbc_output_open_stdout(es()))
}

pub(crate) unsafe fn ttstub_output_putc(handle: usize, c: libc::c_int) -> libc::c_int {
    ttbc_output_putc(es(), OutputId::from_raw(handle), c)
}

pub(crate) unsafe fn ttstub_output_write(
    handle: usize,
    data: *const libc::c_char,
    len: libc::size_t,
) -> libc::size_t {
    ttbc_output_write(es(), OutputId::from_raw(handle), data as *const u8, len)
}

pub(crate) unsafe fn ttstub_output_flush(handle: usize) -> libc::c_int {
    ttbc_output_flush(es(), OutputId::from_raw(handle))
}

pub(crate) unsafe fn ttstub_output_close(handle: usize) -> libc::c_int {
    ttbc_output_close(es(), OutputId::from_raw(handle))
}

pub(crate) fn ttbc_diag_begin_warning_ptr() -> *mut libc::c_void {
    ttbc_diag_begin_warning() as *mut libc::c_void
}

pub(crate) fn ttbc_diag_begin_error_ptr() -> *mut libc::c_void {
    ttbc_diag_begin_error() as *mut libc::c_void
}

/// Appends a NUL-terminated C string to a diagnostic.
pub(crate) unsafe fn diag_append_cstr(diag: *mut libc::c_void, text: *const libc::c_char) {
    tectonic_bridge_core::ttbc_diag_append(&mut *(diag as *mut Diagnostic), text);
}

/// Appends Rust-formatted text to a diagnostic.
pub(crate) unsafe fn diag_append_str(diag: *mut libc::c_void, text: &str) {
    let ctext = std::ffi::CString::new(text).unwrap_or_default();
    tectonic_bridge_core::ttbc_diag_append(&mut *(diag as *mut Diagnostic), ctext.as_ptr());
}

pub(crate) unsafe fn ttstub_diag_finish(diag: *mut libc::c_void) {
    ttbc_diag_finish(es(), diag as *mut Diagnostic);
}

pub(crate) unsafe fn issue_warning_cstr(text: *const libc::c_char) {
    ttbc_issue_warning(es(), text);
}

pub(crate) unsafe fn issue_warning_str(text: &str) {
    let ctext = std::ffi::CString::new(text).unwrap_or_default();
    ttbc_issue_warning(es(), ctext.as_ptr());
}

pub(crate) unsafe fn issue_error_cstr(text: *const libc::c_char) {
    ttbc_issue_error(es(), text);
}

pub(crate) unsafe fn ttstub_shell_escape(cmd: *const u16, len: libc::size_t) -> libc::c_int {
    ttbc_shell_escape(es(), cmd, len)
}

pub(crate) unsafe fn ttstub_get_file_md5(
    path: *const libc::c_char,
    digest: *mut u8,
) -> libc::c_int {
    ttbc_get_file_md5(es(), path, digest)
}
