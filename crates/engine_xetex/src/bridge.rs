//! Engine-side wrappers over the `tectonic_bridge_core` I/O functions.
//!
//! Handles are raw `usize` values with 0 meaning invalid. The state pointer is
//! installed per run in `BRIDGE_PTR`.

use std::ffi::{CStr, CString};

use tectonic_bridge_core::{
    ttbc_diag_append, ttbc_diag_begin_error, ttbc_diag_begin_warning, ttbc_diag_finish,
    ttbc_get_file_md5, ttbc_get_last_input_abspath, ttbc_input_close, ttbc_input_get_mtime,
    ttbc_input_get_size, ttbc_input_getc, ttbc_input_open_primary, ttbc_input_read,
    ttbc_input_seek, ttbc_input_ungetc, ttbc_issue_error, ttbc_issue_warning, ttbc_output_close,
    ttbc_output_flush, ttbc_output_open_stdout, ttbc_output_putc, ttbc_output_write,
    ttbc_shell_escape, CoreBridgeState, FileFormat, InputId, OutputId,
};

pub(crate) use tectonic_bridge_core::Diagnostic;

static mut BRIDGE_PTR: *mut libc::c_void = std::ptr::null_mut();

pub(crate) const TTBC_FILE_FORMAT_TFM: libc::c_int = FileFormat::Tfm as libc::c_int;
pub(crate) const TTBC_FILE_FORMAT_FORMAT: libc::c_int = FileFormat::Format as libc::c_int;
pub(crate) const TTBC_FILE_FORMAT_PICT: libc::c_int = FileFormat::Pict as libc::c_int;
pub(crate) const TTBC_FILE_FORMAT_TEX: libc::c_int = FileFormat::Tex as libc::c_int;
pub(crate) const TTBC_FILE_FORMAT_MISC_FONTS: libc::c_int = FileFormat::MiscFonts as libc::c_int;
pub(crate) const TTBC_FILE_FORMAT_TECTONIC_PRIMARY: libc::c_int =
    FileFormat::TectonicPrimary as libc::c_int;

/// Installs the `CoreBridgeState` pointer for this run, or clears it with null.
///
/// # Safety
///
/// `ptr` must be null or point to a `CoreBridgeState` that stays valid and
/// otherwise unborrowed until it is cleared.
pub(crate) unsafe fn set_bridge_state(ptr: *mut libc::c_void) {
    BRIDGE_PTR = ptr;
}

/// Clears the bridge state installed by [`set_bridge_state`].
pub(crate) fn clear_bridge_state() {
    // SAFETY: null is always a valid bridge state.
    unsafe { set_bridge_state(std::ptr::null_mut()) };
}

extern "C" {
    fn _ttbc_set_error_message(message: *const libc::c_char);
    fn ttbc_global_engine_enter(api: *mut libc::c_void) -> *mut libc::c_void;
    fn ttbc_global_engine_exit();
}

/// Registers a Rust-direct resident operation with bridge-core's C callbacks.
///
/// # Safety
///
/// `api` must point to the operation's `CoreBridgeState`.
pub(crate) unsafe fn enter_global_engine(api: *mut libc::c_void) {
    let _ = ttbc_global_engine_enter(api);
}

/// Clears the callback state installed by [`enter_global_engine`].
pub(crate) fn exit_global_engine() {
    // SAFETY: only resets bridge-core's global callback slot.
    unsafe { ttbc_global_engine_exit() };
}

/// Runs `f` on the bridge state of the current run.
#[inline]
fn with_bridge<R>(f: impl FnOnce(&mut CoreBridgeState<'static>) -> R) -> R {
    // SAFETY: `set_bridge_state` keeps the pointer valid for the whole run, the
    // engine is single-threaded and `f` never re-enters, so this borrow is unique.
    let es = unsafe { BRIDGE_PTR.cast::<CoreBridgeState<'static>>().as_mut() };
    f(es.expect("bridge state used outside run_engine"))
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
        _ => FileFormat::Tex, // 26 and any unknown code
    }
}

/// Opens input file `path`, or returns 0.
pub(crate) fn input_open(path: &[u8], format: libc::c_int, is_gz: bool) -> usize {
    let path = String::from_utf8_lossy(path);
    if let Some(handle) = crate::replay::adapter::open_named(&path, format) {
        return handle;
    }
    crate::replay::adapter::clear_last_path();
    InputId::to_raw(with_bridge(|es| {
        es.input_open(&path, file_format_from_int(format), is_gz)
    }))
}

pub(crate) fn input_open_primary() -> usize {
    if let Some(handle) = crate::replay::adapter::open_primary() {
        return handle;
    }
    crate::replay::adapter::clear_last_path();
    InputId::to_raw(with_bridge(|es| ttbc_input_open_primary(es)))
}

/// Copies the absolute path of the last opened input into `buffer` as a C string.
pub(crate) fn last_input_abspath(buffer: &mut [u8]) -> libc::ssize_t {
    if let Some(copied) = crate::replay::adapter::copy_last_abspath(buffer) {
        return copied;
    }
    // SAFETY: `buffer` is valid for `buffer.len()` bytes.
    with_bridge(|es| unsafe { ttbc_get_last_input_abspath(es, buffer.as_mut_ptr(), buffer.len()) })
}

pub(crate) fn input_get_size(handle: usize) -> libc::size_t {
    if let Some(size) = crate::replay::adapter::get_size(handle) {
        return size;
    }
    with_bridge(|es| ttbc_input_get_size(es, InputId::from_raw(handle)))
}

pub(crate) fn input_get_mtime(handle: usize) -> libc::time_t {
    if crate::replay::adapter::is_mem(handle) {
        return 0;
    }
    with_bridge(|es| ttbc_input_get_mtime(es, InputId::from_raw(handle))) as libc::time_t
}

pub(crate) fn input_getc(handle: usize) -> libc::c_int {
    if let Some(value) = crate::replay::adapter::getc(handle) {
        return value;
    }
    with_bridge(|es| ttbc_input_getc(es, InputId::from_raw(handle)))
}

pub(crate) fn input_ungetc(handle: usize, ch: libc::c_int) -> libc::c_int {
    if let Some(value) = crate::replay::adapter::ungetc(handle, ch) {
        return value;
    }
    with_bridge(|es| ttbc_input_ungetc(es, InputId::from_raw(handle), ch))
}

/// Fills `buf` from `handle`, returning its length or -1 on a short read.
pub(crate) fn input_read(handle: usize, buf: &mut [u8]) -> libc::ssize_t {
    if let Some(value) = crate::replay::adapter::read(handle, buf) {
        return value;
    }
    // SAFETY: `buf` is valid for `buf.len()` bytes.
    with_bridge(|es| unsafe {
        ttbc_input_read(es, InputId::from_raw(handle), buf.as_mut_ptr(), buf.len())
    })
}

/// `ttbc_input_seek` flags an error only for an invalid `whence`, and every
/// caller seeks with `SEEK_SET`, so the error path is unreachable.
pub(crate) fn input_seek(
    handle: usize,
    offset: libc::ssize_t,
    whence: libc::c_int,
) -> libc::size_t {
    if let Some(position) = crate::replay::adapter::seek(handle, offset, whence) {
        return position;
    }
    let mut internal_error: libc::c_int = 0;
    // SAFETY: `internal_error` is a valid out-parameter.
    let rv = with_bridge(|es| unsafe {
        ttbc_input_seek(
            es,
            InputId::from_raw(handle),
            offset,
            whence,
            &mut internal_error,
        )
    });
    debug_assert_eq!(
        internal_error, 0,
        "ttbc_input_seek flags internal_error only for an invalid whence"
    );
    rv
}

/// Returns `true` on a serious internal close error.
#[must_use]
pub(crate) fn input_close(handle: usize) -> bool {
    if let Some(error) = crate::replay::adapter::close(handle) {
        return error;
    }
    with_bridge(|es| ttbc_input_close(es, InputId::from_raw(handle))) != 0
}

/// Opens output file `path`, or returns 0.
pub(crate) fn output_open(path: &[u8], is_gz: bool) -> usize {
    let path = String::from_utf8_lossy(path);
    OutputId::to_raw(with_bridge(|es| es.output_open(&path, is_gz)))
}

pub(crate) fn output_open_stdout() -> usize {
    OutputId::to_raw(with_bridge(|es| ttbc_output_open_stdout(es)))
}

pub(crate) fn output_putc(handle: usize, c: libc::c_int) -> libc::c_int {
    with_bridge(|es| ttbc_output_putc(es, OutputId::from_raw(handle), c))
}

pub(crate) fn output_write(handle: usize, data: &[u8]) -> libc::size_t {
    // SAFETY: `data` is valid for `data.len()` bytes.
    with_bridge(|es| unsafe {
        ttbc_output_write(es, OutputId::from_raw(handle), data.as_ptr(), data.len())
    })
}

pub(crate) fn output_flush(handle: usize) -> libc::c_int {
    with_bridge(|es| ttbc_output_flush(es, OutputId::from_raw(handle)))
}

pub(crate) fn output_close(handle: usize) -> libc::c_int {
    with_bridge(|es| ttbc_output_close(es, OutputId::from_raw(handle)))
}

fn diag_from_raw(raw: *mut Diagnostic) -> Box<Diagnostic> {
    // SAFETY: bridge-core allocates diagnostics with `Box::into_raw`.
    unsafe { Box::from_raw(raw) }
}

pub(crate) fn diag_begin_warning() -> Box<Diagnostic> {
    diag_from_raw(ttbc_diag_begin_warning())
}

pub(crate) fn diag_begin_error() -> Box<Diagnostic> {
    diag_from_raw(ttbc_diag_begin_error())
}

/// Appends `text`. A text with a NUL appends nothing.
pub(crate) fn diag_append(diag: &mut Diagnostic, text: &str) {
    let text = CString::new(text).unwrap_or_default();
    // SAFETY: `text` is NUL-terminated.
    unsafe { ttbc_diag_append(diag, text.as_ptr()) };
}

/// Appends one output byte as C did: alone, lossily decoded, nothing for NUL.
pub(crate) fn diag_append_byte(diag: &mut Diagnostic, byte: u8) {
    let text = [byte, 0];
    // SAFETY: `text` is NUL-terminated.
    unsafe { ttbc_diag_append(diag, text.as_ptr().cast()) };
}

/// Reports `diag` to the driver.
pub(crate) fn diag_finish(diag: Box<Diagnostic>) {
    // SAFETY: the pointer comes from `Box::into_raw` and the callee takes it back.
    with_bridge(|es| unsafe { ttbc_diag_finish(es, Box::into_raw(diag)) });
}

pub(crate) fn issue_warning(text: &str) {
    let text = CString::new(text).unwrap_or_default();
    // SAFETY: `text` is NUL-terminated.
    with_bridge(|es| unsafe { ttbc_issue_warning(es, text.as_ptr()) });
}

pub(crate) fn issue_error(text: &str) {
    let text = CString::new(text).unwrap_or_default();
    // SAFETY: `text` is NUL-terminated.
    with_bridge(|es| unsafe { ttbc_issue_error(es, text.as_ptr()) });
}

pub(crate) fn shell_escape(cmd: &[u16]) -> libc::c_int {
    // SAFETY: `cmd` is valid for `cmd.len()` units.
    with_bridge(|es| unsafe { ttbc_shell_escape(es, cmd.as_ptr(), cmd.len()) })
}

/// Writes the MD5 digest of file `path` to `digest`, returning nonzero on failure.
pub(crate) fn get_file_md5(path: &CStr, digest: &mut [u8; 16]) -> libc::c_int {
    // SAFETY: `path` is NUL-terminated and `digest` holds the 16 bytes written.
    with_bridge(|es| unsafe { ttbc_get_file_md5(es, path.as_ptr(), digest.as_mut_ptr()) })
}

/// Writes the MD5 digest of `data` to `digest`, returning nonzero on failure.
pub(crate) fn data_md5(data: &[u8], digest: &mut [u8; 16]) -> libc::c_int {
    // SAFETY: `data` is valid for its length and `digest` holds the 16 bytes written.
    unsafe {
        tectonic_bridge_core::ttbc_get_data_md5(data.as_ptr(), data.len(), digest.as_mut_ptr())
    }
}

/// Surfaces `message` as the run's abort message. A message with a NUL is dropped.
pub(crate) fn set_error_message(message: impl Into<Vec<u8>>) {
    if let Ok(msg) = CString::new(message) {
        // SAFETY: `msg` is NUL-terminated and the callee copies it.
        unsafe { _ttbc_set_error_message(msg.as_ptr()) };
    }
}
