//! The boundary with the JavaScript host.
//!
//! TeX is synchronous: it asks for `article.cls` partway through typesetting a
//! paragraph and cannot continue until the bytes arrive. There is no way to
//! await a `fetch()` from that position, so support-file resolution is a single
//! blocking host import. The runtime that ships with this crate satisfies it in
//! a Web Worker, where a synchronous `XMLHttpRequest` — or an `Atomics.wait`
//! handshake with a fetching thread — can block without freezing the page.

use std::alloc::{alloc, dealloc, Layout};

/// Outcome of a host file request.
mod status {
    /// The file was found and the buffer holds its contents.
    pub const OK: i32 = 0;
    /// No such file. TeX treats this as a normal "not found", not an error.
    pub const NOT_FOUND: i32 = 1;
}

// The explicit import module keeps rust-lld from treating these as undefined
// symbols: only symbols carrying import metadata may be unresolved at link
// time, and the runtime supplies them under `env`.
#[link(wasm_import_module = "env")]
extern "C" {
    /// Resolve a support file by TeX name, blocking until it is available.
    ///
    /// On success the host stores a buffer allocated with
    /// [`tectonic_wasm_alloc`] into `out_ptr`/`out_len` and gives up ownership
    /// of it. Any other return leaves the out-parameters untouched.
    fn tectonic_host_read_file(
        name_ptr: *const u8,
        name_len: usize,
        out_ptr: *mut *mut u8,
        out_len: *mut usize,
    ) -> i32;

    /// Resolve a project source file by the caller's own path spelling.
    ///
    /// Same contract as [`tectonic_host_read_file`], but answered from the
    /// host's resident copy of the project text rather than the support pack.
    fn tectonic_host_read_source(
        name_ptr: *const u8,
        name_len: usize,
        out_ptr: *mut *mut u8,
        out_len: *mut usize,
    ) -> i32;

    /// Report a diagnostic line from the engine to the host.
    fn tectonic_host_log(kind: i32, msg_ptr: *const u8, msg_len: usize);
}

/// Allocate a buffer for the host to fill.
///
/// The host passes the result back through an out-parameter, transferring
/// ownership to Rust. A buffer that never makes that trip — because the request
/// failed after allocation — must be returned with [`tectonic_wasm_free`].
///
/// Returns null if `len` is zero or the allocation fails.
#[no_mangle]
pub extern "C" fn tectonic_wasm_alloc(len: usize) -> *mut u8 {
    let Ok(layout) = Layout::from_size_align(len, 1) else {
        return std::ptr::null_mut();
    };

    if len == 0 {
        return std::ptr::null_mut();
    }

    // SAFETY: the layout is non-zero-sized, which is alloc's only requirement.
    unsafe { alloc(layout) }
}

/// Release a buffer from [`tectonic_wasm_alloc`] that Rust never took over.
///
/// # Safety
///
/// `ptr` must have come from [`tectonic_wasm_alloc`] with the same `len`, and
/// must not have been handed to Rust through a host out-parameter.
#[no_mangle]
pub unsafe extern "C" fn tectonic_wasm_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }

    if let Ok(layout) = Layout::from_size_align(len, 1) {
        dealloc(ptr, layout);
    }
}

/// Take ownership of a buffer the host filled through an out-parameter.
///
/// # Safety
///
/// `ptr`/`len` must describe a buffer from [`tectonic_wasm_alloc`] whose
/// ownership the host has released.
unsafe fn adopt(ptr: *mut u8, len: usize) -> Vec<u8> {
    if ptr.is_null() || len == 0 {
        return Vec::new();
    }

    // The buffer came from the global allocator with alignment 1, which is what
    // Vec<u8> also asks for, so it can be reconstituted directly.
    Vec::from_raw_parts(ptr, len, len)
}

/// Ask the host for a support file.
///
/// `Ok(None)` means the host is certain the file does not exist — the answer
/// TeX needs in order to move on to the next candidate name.
pub fn read_file(name: &str) -> Result<Option<Vec<u8>>, String> {
    let mut out_ptr: *mut u8 = std::ptr::null_mut();
    let mut out_len: usize = 0;

    // SAFETY: we pass a live UTF-8 slice and two writable out-parameters.
    let rc =
        unsafe { tectonic_host_read_file(name.as_ptr(), name.len(), &mut out_ptr, &mut out_len) };

    match rc {
        status::OK => {
            // SAFETY: a zero return is the host promising it filled the
            // out-parameters with a buffer it has given up.
            Ok(Some(unsafe { adopt(out_ptr, out_len) }))
        }
        status::NOT_FOUND => Ok(None),
        other => Err(format!("the host failed to read {name} (code {other})")),
    }
}

/// Ask the host for a project source file.
///
/// `Ok(None)` means the host holds no source under that path.
pub fn read_source(name: &str) -> Result<Option<Vec<u8>>, String> {
    let mut out_ptr: *mut u8 = std::ptr::null_mut();
    let mut out_len: usize = 0;

    // SAFETY: we pass a live UTF-8 slice and two writable out-parameters.
    let rc =
        unsafe { tectonic_host_read_source(name.as_ptr(), name.len(), &mut out_ptr, &mut out_len) };

    match rc {
        status::OK => {
            // SAFETY: a zero return is the host promising it filled the
            // out-parameters with a buffer it has given up.
            Ok(Some(unsafe { adopt(out_ptr, out_len) }))
        }
        status::NOT_FOUND => Ok(None),
        other => Err(format!(
            "the host failed to read the source {name} (code {other})"
        )),
    }
}

/// Severity of a message forwarded to the host.
#[derive(Clone, Copy, Debug)]
#[repr(i32)]
pub enum LogKind {
    /// Informational engine chatter.
    Note = 0,
    /// A TeX warning.
    Warning = 1,
    /// A TeX error.
    Error = 2,
}

/// Forward one diagnostic line to the host.
pub fn log(kind: LogKind, message: &str) {
    // SAFETY: we pass a live UTF-8 slice; the host only reads it.
    unsafe { tectonic_host_log(kind as i32, message.as_ptr(), message.len()) }
}
