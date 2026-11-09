//! Engine-side I/O and diagnostics over the run's `CoreBridgeState`.
//!
//! Handles are raw `usize` values with 0 meaning invalid. Replay sources
//! answer first, everything else goes to the state installed by [`enter`].

use std::borrow::Cow;
use std::ffi::CStr;
use std::io::SeekFrom;
use std::marker::PhantomData;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use texyon_bridge_core::{CoreBridgeState, InputId, OutputId};

pub(crate) use texyon_bridge_core::{Diagnostic, FileFormat};

/// The bridge state of the current run. Every run holds the engine lock, so
/// one slot serves all threads.
static BRIDGE: AtomicPtr<CoreBridgeState<'static>> = AtomicPtr::new(ptr::null_mut());

/// Keeps a run's bridge state installed while holding its exclusive borrow.
pub(crate) struct BridgeScope<'a> {
    previous: *mut CoreBridgeState<'static>,
    _borrow: PhantomData<&'a mut ()>,
}

/// Installs `api` as the bridge state until the returned scope drops.
pub(crate) fn enter<'a>(api: &'a mut CoreBridgeState<'_>) -> BridgeScope<'a> {
    let api = (api as *mut CoreBridgeState<'_>).cast::<CoreBridgeState<'static>>();
    BridgeScope {
        previous: BRIDGE.swap(api, Ordering::Relaxed),
        _borrow: PhantomData,
    }
}

impl Drop for BridgeScope<'_> {
    fn drop(&mut self) {
        BRIDGE.store(self.previous, Ordering::Relaxed);
    }
}

/// Runs `f` on the bridge state of the current run.
#[inline]
pub(crate) fn with_bridge<R>(f: impl FnOnce(&mut CoreBridgeState<'_>) -> R) -> R {
    // SAFETY: a live `BridgeScope` holds the exclusive borrow behind the pointer,
    // the engine is single-threaded and `f` never re-enters, so this borrow is unique.
    let es = unsafe { BRIDGE.load(Ordering::Relaxed).as_mut() };
    f(es.expect("bridge state used outside a run"))
}

/// Runs `f`, a call into C code that does its I/O through bridge-core's C
/// global state, with that state set to the current run's.
pub(crate) fn with_c_io<R>(f: impl FnOnce() -> R) -> R {
    with_bridge(|es| {
        let _global = es.enter_global_engine();
        f()
    })
}

fn input_id(handle: usize) -> InputId {
    InputId::from_raw(handle).expect("valid input handle")
}

fn output_id(handle: usize) -> OutputId {
    OutputId::from_raw(handle).expect("valid output handle")
}

/// `bytes` as bridge-core saw them through a C string: empty when they hold a NUL.
fn c_view(bytes: &[u8]) -> Cow<'_, str> {
    if bytes.contains(&0) {
        Cow::Borrowed("")
    } else {
        String::from_utf8_lossy(bytes)
    }
}

/// Opens input file `path`, or returns 0.
pub(crate) fn input_open(path: &[u8], format: FileFormat, is_gz: bool) -> usize {
    let path = String::from_utf8_lossy(path);
    if let Some(handle) = crate::replay::adapter::open_named(&path, format) {
        return handle;
    }
    crate::replay::adapter::clear_last_path();
    InputId::to_raw(with_bridge(|es| es.input_open(&path, format, is_gz)))
}

pub(crate) fn input_open_primary() -> usize {
    if let Some(handle) = crate::replay::adapter::open_primary() {
        return handle;
    }
    crate::replay::adapter::clear_last_path();
    InputId::to_raw(with_bridge(|es| es.input_open_primary()))
}

/// The extension the I/O layer appended to open the last input, if any.
pub(crate) fn last_input_extension() -> Option<&'static str> {
    if crate::replay::adapter::opened_last() {
        return None;
    }
    with_bridge(|es| es.latest_input_extension())
}

/// Longest absolute input path kept, including C's terminating NUL.
const ABSPATH_CAPACITY: usize = 1024;

/// The absolute path of the last opened input, empty when unknown.
pub(crate) fn last_input_abspath() -> Vec<u8> {
    if let Some(path) = crate::replay::adapter::last_abspath(ABSPATH_CAPACITY - 1) {
        return path;
    }
    with_bridge(|es| match es.latest_input_abspath() {
        Some(path) if path.len() < ABSPATH_CAPACITY => {
            let end = path.find('\0').unwrap_or(path.len());
            path.as_bytes()[..end].to_vec()
        }
        _ => Vec::new(),
    })
}

pub(crate) fn input_get_size(handle: usize) -> usize {
    if let Some(size) = crate::replay::adapter::get_size(handle) {
        return size;
    }
    with_bridge(|es| es.input_get_size(input_id(handle)))
}

pub(crate) fn input_get_mtime(handle: usize) -> libc::time_t {
    if crate::replay::adapter::is_mem(handle) {
        return 0;
    }
    with_bridge(|es| es.input_get_mtime(input_id(handle))) as libc::time_t
}

/// The next byte of `handle`, or `EOF`.
pub(crate) fn input_getc(handle: usize) -> libc::c_int {
    if let Some(value) = crate::replay::adapter::getc(handle) {
        return value;
    }
    with_bridge(|es| es.input_getc(input_id(handle))).map_or(texyon_bridge_core::EOF, i32::from)
}

pub(crate) fn input_ungetc(handle: usize, ch: libc::c_int) {
    if crate::replay::adapter::ungetc(handle, ch).is_some() {
        return;
    }
    with_bridge(|es| es.input_ungetc(input_id(handle), ch as u8));
}

/// Fills `buf` from `handle`, returning whether it was filled.
#[must_use]
pub(crate) fn input_read(handle: usize, buf: &mut [u8]) -> bool {
    if let Some(value) = crate::replay::adapter::read(handle, buf) {
        return value >= 0;
    }
    with_bridge(|es| es.input_read(input_id(handle), buf)).is_ok()
}

/// Seeks `handle` to byte `offset`, returning the new position.
pub(crate) fn input_seek(handle: usize, offset: u64) -> usize {
    if let Some(position) =
        crate::replay::adapter::seek(handle, offset as libc::ssize_t, libc::SEEK_SET)
    {
        return position;
    }
    with_bridge(|es| es.input_seek(input_id(handle), SeekFrom::Start(offset))) as usize
}

/// Returns `true` on a serious internal close error.
#[must_use]
pub(crate) fn input_close(handle: usize) -> bool {
    if let Some(error) = crate::replay::adapter::close(handle) {
        return error;
    }
    InputId::from_raw(handle).is_some_and(|id| with_bridge(|es| es.input_close(id)))
}

/// Opens output file `path`, or returns 0.
pub(crate) fn output_open(path: &[u8], is_gz: bool) -> usize {
    let path = String::from_utf8_lossy(path);
    OutputId::to_raw(with_bridge(|es| es.output_open(&path, is_gz)))
}

pub(crate) fn output_open_stdout() -> usize {
    OutputId::to_raw(with_bridge(|es| es.output_open_stdout()))
}

pub(crate) fn output_putc(handle: usize, byte: u8) {
    with_bridge(|es| es.output_write(output_id(handle), &[byte]));
}

/// Writes `data` to `handle`, returning whether it was all written.
#[must_use]
pub(crate) fn output_write(handle: usize, data: &[u8]) -> bool {
    !with_bridge(|es| es.output_write(output_id(handle), data))
}

pub(crate) fn output_flush(handle: usize) {
    with_bridge(|es| es.output_flush(output_id(handle)));
}

/// Closes `handle`, returning `true` on an error.
pub(crate) fn output_close(handle: usize) -> bool {
    OutputId::from_raw(handle).is_some_and(|id| with_bridge(|es| es.output_close(id)))
}

/// Appends `text`. A text with a NUL appends nothing.
pub(crate) fn diag_append(diag: &mut Diagnostic, text: &str) {
    if !text.contains('\0') {
        diag.append(text);
    }
}

/// Reports `diag` to the driver.
pub(crate) fn diag_finish(diag: Diagnostic) {
    with_bridge(|es| es.finish_diagnostic(diag));
}

/// Issues `text` as a warning, empty when it holds a NUL.
pub(crate) fn issue_warning(text: &str) {
    with_bridge(|es| es.issue_warning(&c_view(text.as_bytes())));
}

/// Issues `text` as an error, empty when it holds a NUL.
pub(crate) fn issue_error(text: &str) {
    with_bridge(|es| es.issue_error(&c_view(text.as_bytes())));
}

pub(crate) fn shell_escape(cmd: &[u16]) {
    with_bridge(|es| match String::from_utf16(cmd) {
        Ok(cmd) => {
            es.shell_escape(&cmd);
        }
        Err(err) => es.issue_error(&format!("command was not valid UTF-16: {err}")),
    });
}

/// The C library's message for error number `errnum`.
pub(crate) fn strerror(errnum: i32) -> Vec<u8> {
    // SAFETY: strerror returns a NUL-terminated message, copied before any other call.
    unsafe { CStr::from_ptr(libc::strerror(errnum)) }
        .to_bytes()
        .to_vec()
}

/// The MD5 digest of file `path`, or `None` on failure.
pub(crate) fn get_file_md5(path: &[u8]) -> Option<[u8; 16]> {
    let mut digest = [0; 16];
    let failed = with_bridge(|es| es.get_file_md5(&c_view(path), &mut digest));
    (!failed).then_some(digest)
}

/// Surfaces `message` as the run's abort message. A message with a NUL is dropped.
pub(crate) fn set_error_message(message: impl AsRef<[u8]>) {
    texyon_bridge_core::set_abort_message(message.as_ref());
}
