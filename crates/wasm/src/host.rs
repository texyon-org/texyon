//! The boundary with the JavaScript host.
//!
//! TeX cannot await a `fetch()` mid-paragraph, so file resolution is a
//! blocking import that the host satisfies from a Web Worker.

use std::{
    alloc::{alloc, dealloc, Layout},
    ptr,
};

use serde::Serialize;

mod status {
    pub const OK: i32 = 0;
    pub const NOT_FOUND: i32 = 1;
}

/// Signature shared by the host's file-reading imports.
type ReadImport = unsafe extern "C" fn(*const u8, usize, *mut *mut u8, *mut usize) -> i32;

// Only symbols with import metadata may stay unresolved when rust-lld links.
#[link(wasm_import_module = "env")]
extern "C" {
    /// Resolve a support file by TeX name, blocking until it is available.
    ///
    /// On `status::OK` the host stores a [`tectonic_wasm_alloc`] buffer it
    /// gives up into the out-parameters. Any other return leaves them untouched.
    fn tectonic_host_read_file(
        name_ptr: *const u8,
        name_len: usize,
        out_ptr: *mut *mut u8,
        out_len: *mut usize,
    ) -> i32;

    /// Resolve a project source by the caller's path spelling, with the same
    /// contract as [`tectonic_host_read_file`].
    fn tectonic_host_read_source(
        name_ptr: *const u8,
        name_len: usize,
        out_ptr: *mut *mut u8,
        out_len: *mut usize,
    ) -> i32;

    fn tectonic_host_log(kind: i32, msg_ptr: *const u8, msg_len: usize);
}

/// Allocate a buffer for the host to fill.
///
/// A buffer that never makes it back into Rust must be released with
/// [`tectonic_wasm_free`]. Returns null if `len` is zero or allocation fails.
#[no_mangle]
pub extern "C" fn tectonic_wasm_alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return ptr::null_mut();
    }
    let Ok(layout) = Layout::from_size_align(len, 1) else {
        return ptr::null_mut();
    };

    // SAFETY: the layout is non-zero-sized.
    unsafe { alloc(layout) }
}

/// Release a buffer from [`tectonic_wasm_alloc`] or [`hand_over`].
///
/// # Safety
///
/// `ptr` must have come from one of those with the same `len`, and Rust must
/// not have adopted it.
#[no_mangle]
pub unsafe extern "C" fn tectonic_wasm_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }

    if let Ok(layout) = Layout::from_size_align(len, 1) {
        // SAFETY: the caller guarantees `ptr` was allocated with this layout.
        unsafe { dealloc(ptr, layout) };
    }
}

/// Give bytes to the host, which releases them with [`tectonic_wasm_free`].
pub fn hand_over(bytes: Vec<u8>) -> (*mut u8, usize) {
    let len = bytes.len();
    (Box::into_raw(bytes.into_boxed_slice()).cast(), len)
}

/// Take ownership of a buffer the host filled.
///
/// # Safety
///
/// `ptr`/`len` must describe a [`tectonic_wasm_alloc`] buffer whose ownership
/// the host has released.
pub unsafe fn adopt(ptr: *mut u8, len: usize) -> Vec<u8> {
    if ptr.is_null() || len == 0 {
        return Vec::new();
    }

    // SAFETY: an alignment-1 allocation of exactly `len` bytes is what Vec<u8>
    // expects, and the caller guarantees ownership.
    unsafe { Vec::from_raw_parts(ptr, len, len) }
}

/// A buffer in linear memory, as the JSON protocol spells it.
#[derive(Debug, Serialize)]
pub struct HostBuffer {
    pub ptr: u32,
    pub len: u32,
}

impl HostBuffer {
    /// Give bytes to the host, see [`hand_over`].
    pub fn hand_over(bytes: Vec<u8>) -> Self {
        let (ptr, len) = hand_over(bytes);
        // wasm32 addresses and lengths are 32 bits wide.
        Self {
            ptr: ptr as u32,
            len: len as u32,
        }
    }

    /// Take ownership of a buffer the host filled, see [`adopt`].
    ///
    /// # Safety
    ///
    /// As for [`adopt`].
    pub unsafe fn adopt(self) -> Vec<u8> {
        // SAFETY: forwarded to the caller.
        unsafe { adopt(self.ptr as *mut u8, self.len as usize) }
    }
}

fn read_via(import: ReadImport, name: &str) -> Result<Option<Vec<u8>>, i32> {
    let mut out_ptr = ptr::null_mut();
    let mut out_len = 0;

    // SAFETY: a live UTF-8 slice and two writable out-parameters.
    let rc = unsafe { import(name.as_ptr(), name.len(), &mut out_ptr, &mut out_len) };

    match rc {
        // SAFETY: on OK the host filled the out-parameters with a buffer it gave up.
        status::OK => Ok(Some(unsafe { adopt(out_ptr, out_len) })),
        status::NOT_FOUND => Ok(None),
        code => Err(code),
    }
}

/// Ask the host for a support file. `Ok(None)` means it does not exist.
pub fn read_file(name: &str) -> Result<Option<Vec<u8>>, String> {
    read_via(tectonic_host_read_file, name)
        .map_err(|code| format!("the host failed to read {name} (code {code})"))
}

/// Ask the host for a project source. `Ok(None)` means it holds none.
pub fn read_source(name: &str) -> Result<Option<Vec<u8>>, String> {
    read_via(tectonic_host_read_source, name)
        .map_err(|code| format!("the host failed to read the source {name} (code {code})"))
}

/// Severity of a message forwarded to the host.
#[derive(Clone, Copy, Debug)]
#[repr(i32)]
pub enum LogKind {
    Note = 0,
    Warning = 1,
    Error = 2,
}

/// Forward one diagnostic line to the host.
pub fn log(kind: LogKind, message: &str) {
    // SAFETY: a live UTF-8 slice that the host only reads.
    unsafe { tectonic_host_log(kind as i32, message.as_ptr(), message.len()) }
}
