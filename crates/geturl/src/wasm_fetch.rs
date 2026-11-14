//! A URL-get backend that hands each request to the JavaScript host.
//!
//! Tectonic's bundle code is synchronous top to bottom: the TeX engine asks for
//! `article.cls` in the middle of typesetting and expects the bytes back before
//! it continues. There is no way to await a `fetch()` in that position, so this
//! backend calls out to a single host import that must block until the response
//! is complete. In a Web Worker that is a synchronous `XMLHttpRequest`, or an
//! `Atomics.wait` handshake with a fetching thread — see `tectonic-wasm`'s
//! runtime for the implementation the demo ships.
//!
//! # The host interface
//!
//! The embedder must supply one import in the `tectonic` module:
//!
//! ```text
//! texyon_http_get(
//!     url_ptr: *const u8, url_len: usize,   // request URL, UTF-8
//!     offset: u64, length: u64,             // byte range; length 0 means "whole resource"
//!     out_ptr: *mut *mut u8,                // receives a buffer allocated with texyon_wasm_alloc
//!     out_len: *mut usize,                  // receives that buffer's length
//! ) -> i32                                  // 0 on success, nonzero on failure
//! ```
//!
//! On success the host owns nothing: the buffer it reports is handed to Rust,
//! which frees it. `texyon_wasm_alloc` and `texyon_wasm_free` are exported
//! from this crate for exactly that purpose.

use crate::{GetUrlBackend, RangeReader};
use std::io::Cursor;
use texyon_errors::{Result, anyhow::bail};

unsafe extern "C" {
    /// Fetch a URL, or a byte range of one, blocking until it completes.
    ///
    /// See the module documentation for the contract.
    fn texyon_http_get(
        url_ptr: *const u8,
        url_len: usize,
        offset: u64,
        length: u64,
        out_ptr: *mut *mut u8,
        out_len: *mut usize,
    ) -> i32;
}

/// Allocate a buffer for the host to fill with response bytes.
///
/// The host must pass the result back through `texyon_http_get`'s
/// out-parameters, after which Rust owns it. A host that allocates a buffer and
/// then fails the request must release it with [`texyon_wasm_free`].
///
/// Returns null if the allocation fails.
#[unsafe(no_mangle)]
pub extern "C" fn texyon_wasm_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Release a buffer from [`texyon_wasm_alloc`] that never reached Rust.
///
/// # Safety
///
/// `ptr` must have come from [`texyon_wasm_alloc`] with the same `len`, and
/// must not have been handed to Rust through `texyon_http_get`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn texyon_wasm_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, 0, len));
    }
}

/// Issue one blocking request and take ownership of the response bytes.
fn http_get(url: &str, offset: u64, length: u64) -> Result<Vec<u8>> {
    let mut out_ptr: *mut u8 = std::ptr::null_mut();
    let mut out_len: usize = 0;

    // SAFETY: we pass a valid UTF-8 slice and two writable out-parameters. The
    // host contract is that a zero return means both were set to a buffer it
    // allocated with texyon_wasm_alloc and has given up.
    let rc = unsafe {
        texyon_http_get(
            url.as_ptr(),
            url.len(),
            offset,
            length,
            &mut out_ptr,
            &mut out_len,
        )
    };

    if rc != 0 {
        bail!("HTTP request for {url} failed in the host (code {rc})");
    }

    if out_ptr.is_null() {
        bail!("HTTP request for {url} reported success without a response body");
    }

    // SAFETY: the host allocated this with texyon_wasm_alloc, i.e. as a Vec
    // with this capacity, and has transferred ownership to us.
    Ok(unsafe { Vec::from_raw_parts(out_ptr, out_len, out_len) })
}

/// A URL-get backend that delegates to the JavaScript host.
#[derive(Default, Debug)]
pub struct WasmFetchBackend {}

/// A range reader that delegates to the JavaScript host.
#[derive(Debug)]
pub struct WasmFetchRangeReader {
    url: String,
}

impl RangeReader for WasmFetchRangeReader {
    type Response = Cursor<Vec<u8>>;

    fn read_range(&mut self, offset: u64, length: usize) -> Result<Self::Response> {
        Ok(Cursor::new(http_get(&self.url, offset, length as u64)?))
    }
}

impl GetUrlBackend for WasmFetchBackend {
    type Response = Cursor<Vec<u8>>;
    type RangeReader = WasmFetchRangeReader;

    fn get_url(&mut self, url: &str) -> Result<Self::Response> {
        Ok(Cursor::new(http_get(url, 0, 0)?))
    }

    /// Redirects are the host's business: `fetch` and `XMLHttpRequest` both
    /// follow them transparently, and neither exposes the final URL in a way
    /// worth plumbing through. Report the URL unchanged, as the curl backend
    /// does.
    fn resolve_url(&mut self, url: &str) -> Result<String> {
        Ok(url.to_owned())
    }

    fn open_range_reader(&self, url: &str) -> Self::RangeReader {
        WasmFetchRangeReader {
            url: url.to_owned(),
        }
    }
}
