//! TeXyoN's live-preview engine as a WebAssembly module.
//!
//! Everything goes through [`texyon_request`]: a JSON request in, a JSON
//! response out, with bulk payloads left in linear memory. The host supplies
//! the imports declared in `host`.

// Linked in for its C allocation symbols, never called from Rust.
#[cfg(target_family = "wasm")]
mod alloc_shim;

mod host;
mod io;
mod resources;
mod session;
mod support;

use std::cell::RefCell;

use host::HostBuffer;
use session::Session;
use texyon_live_preview::Payload;
use texyon_preview_protocol::session::{HostRequest, Request, Response};

thread_local! {
    // XeTeX keeps process-global state, so one module instance hosts one session.
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

impl Payload for HostBuffer {
    fn hand_over(bytes: Vec<u8>) -> Self {
        HostBuffer::hand_over(bytes)
    }
}

/// Handle one JSON request from the host.
///
/// Returns 0 with the reply in `out_ptr`/`out_len`, which the host releases
/// with `texyon_wasm_free`, or -1 if no reply could be encoded.
///
/// # Safety
///
/// `req_ptr` must point to `req_len` readable bytes, and both out-parameters
/// must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn texyon_request(
    req_ptr: *const u8,
    req_len: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    // SAFETY: the caller guarantees `req_ptr` points to `req_len` readable bytes.
    let request = unsafe { std::slice::from_raw_parts(req_ptr, req_len) };

    let response = match std::str::from_utf8(request) {
        Ok(text) => dispatch(text),
        Err(error) => Response::error(format!("request was not UTF-8: {error}")),
    };

    let Ok(encoded) = serde_json::to_vec(&response) else {
        return -1;
    };

    let (ptr, len) = host::hand_over(encoded);
    // SAFETY: the caller guarantees both out-parameters are writable.
    unsafe {
        *out_ptr = ptr;
        *out_len = len;
    }
    0
}

/// Write the PGO counters gathered so far, as a `.profraw` file.
///
/// Returns 0 with the bytes in `out_ptr`/`out_len`, which the host releases
/// with `texyon_wasm_free`, or -1 if they could not be written.
///
/// # Safety
///
/// Both out-parameters must be writable, and no other request may be running.
#[cfg(all(feature = "pgo", target_family = "wasm"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn texyon_pgo_profile(out_ptr: *mut *mut u8, out_len: *mut usize) -> i32 {
    let mut profile = Vec::new();
    // SAFETY: the caller guarantees no request runs concurrently.
    if unsafe { minicov::capture_coverage(&mut profile) }.is_err() {
        return -1;
    }
    let (ptr, len) = host::hand_over(profile);
    // SAFETY: the caller guarantees both out-parameters are writable.
    unsafe {
        *out_ptr = ptr;
        *out_len = len;
    }
    0
}

fn dispatch(text: &str) -> Response<HostBuffer> {
    let request: Request<HostBuffer> = match serde_json::from_str(text) {
        Ok(request) => request,
        Err(error) => return Response::error(format!("malformed request: {error}")),
    };

    SESSION.with_borrow_mut(|slot| match request {
        Request::Host(HostRequest::Open {
            format,
            primary,
            raster,
            build_date,
            ..
        }) => {
            if slot.is_some() {
                return Response::error("a session is already open");
            }
            match session::open(&format, &primary, raster, build_date) {
                Ok(session) => {
                    *slot = Some(session);
                    Response::Ok
                }
                Err(error) => Response::error(error),
            }
        }

        Request::Host(HostRequest::Preload { name, bytes }) => {
            // SAFETY: the host allocated this with texyon_wasm_alloc and gave
            // up ownership by sending the request.
            let bytes = unsafe { bytes.adopt() };
            match slot.as_ref() {
                Some(session) => {
                    session::preload(session, &name, bytes);
                    Response::Ok
                }
                None => Response::error("no session is open"),
            }
        }

        Request::Host(HostRequest::Shutdown) => match slot.take() {
            Some(mut session) => match session.finish() {
                Ok(()) => Response::Ok,
                Err(error) => Response::error(error),
            },
            None => Response::Ok,
        },

        Request::Session(request) => match slot.as_mut() {
            Some(session) => texyon_live_preview::handle(session, request),
            None => Response::error("no session is open"),
        },
    })
}
