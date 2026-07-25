//! Tectonic's live-preview engine, compiled for the browser.
//!
//! This crate is the WebAssembly face of the resident XeTeX session: it holds
//! one document, applies whole-buffer edits to it, and returns SVG for the
//! pages whose appearance moved — or, on request, the whole thing as a PDF
//! through xdvipdfmx, which is the same typesetting written out differently
//! rather than a second compile.
//!
//! The point is latency. A cold LaTeX compile spends most of its time before
//! your cursor
//! — loading a 20 MB format dump, the document class, its packages — and
//! [`IncrementalXeTeXSession`] makes that cost payable once, rewinding to a
//! checkpoint just before the edit instead of starting over.
//!
//! [`IncrementalXeTeXSession`]: tectonic_engine_xetex::IncrementalXeTeXSession
//!
//! # The ABI
//!
//! Everything goes through one entry point, [`tectonic_request`]: a JSON
//! request in, a JSON response out, with bulk payloads left in linear memory
//! and reported as offset/length pairs. See [`protocol`] for the message types
//! and `js/` for the TypeScript wrapper that is the intended way to use this.
//!
//! The host must supply two imports, described in [`host`]: one to read a
//! support file by name — blocking, because TeX asks for files in the middle of
//! typesetting — and one to receive diagnostics.

// Defining the C allocation entry points is a link-time act: nothing calls into
// this module from Rust, but every `malloc` in the engine's C resolves here.
#[cfg(target_family = "wasm")]
mod alloc_shim;

pub mod host;
pub mod io;
pub mod protocol;
pub mod resources;
pub mod session;
pub mod sink;
pub mod support;

use std::cell::RefCell;

use protocol::{CompileReport, RenderedPage, Request, Response};
use session::Session;

thread_local! {
    /// The one resident document.
    ///
    /// XeTeX keeps process-global state, so a module instance hosts a single
    /// session; a second document means a second Worker.
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

/// Handle one request from the host.
///
/// `req_ptr`/`req_len` describe a UTF-8 JSON request in linear memory, which
/// the host still owns. On return `out_ptr`/`out_len` describe a UTF-8 JSON
/// response that the host must release with
/// [`tectonic_wasm_free`](host::tectonic_wasm_free).
///
/// Returns 0 when a response was produced, and -1 when the response could not
/// be built at all — the request was not valid UTF-8, or memory ran out.
///
/// # Safety
///
/// `req_ptr` must point to `req_len` readable bytes, and both out-parameters
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn tectonic_request(
    req_ptr: *const u8,
    req_len: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    let request = std::slice::from_raw_parts(req_ptr, req_len);

    let response = match std::str::from_utf8(request) {
        Ok(text) => dispatch(text),
        Err(error) => Response::error(format!("request was not UTF-8: {error}")),
    };

    let Ok(encoded) = serde_json::to_vec(&response) else {
        return -1;
    };

    // Hand the encoded reply to the host, which frees it with
    // tectonic_wasm_free once it has copied the bytes out.
    let len = encoded.len();
    *out_ptr = Box::into_raw(encoded.into_boxed_slice()) as *mut u8;
    *out_len = len;
    0
}

/// Route one decoded request, turning any failure into an error response.
fn dispatch(text: &str) -> Response {
    let request: Request = match serde_json::from_str(text) {
        Ok(request) => request,
        Err(error) => return Response::error(format!("malformed request: {error}")),
    };

    match request {
        Request::Open { format, primary } => SESSION.with_borrow_mut(|slot| {
            if slot.is_some() {
                return Response::error("a session is already open");
            }
            match Session::new(&format, &primary) {
                Ok(session) => {
                    *slot = Some(session);
                    Response::Ok
                }
                Err(error) => Response::error(error),
            }
        }),

        Request::Preload { name, ptr, len } => SESSION.with_borrow_mut(|slot| {
            let Some(session) = slot.as_mut() else {
                return Response::error("no session is open");
            };
            // SAFETY: the host allocated this with tectonic_wasm_alloc and, by
            // sending the request, has given up ownership of it.
            let bytes = unsafe { take_host_bytes(ptr, len) };
            session.preload(&name, bytes);
            Response::Ok
        }),

        Request::Compile { files, viewport } => SESSION.with_borrow_mut(|slot| {
            let Some(session) = slot.as_mut() else {
                return Response::error("no session is open");
            };
            match session.compile(files, viewport) {
                Ok(output) => Response::Compiled(Box::new(report(session, output))),
                Err(error) => Response::error(error),
            }
        }),

        Request::ContinueTail => SESSION.with_borrow_mut(|slot| {
            let Some(session) = slot.as_mut() else {
                return Response::error("no session is open");
            };
            match session.continue_tail() {
                Ok(Some(output)) => Response::Compiled(Box::new(report(session, output))),
                Ok(None) => Response::Idle,
                Err(error) => Response::error(error),
            }
        }),

        Request::Export => SESSION.with_borrow_mut(|slot| {
            let Some(session) = slot.as_mut() else {
                return Response::error("no session is open");
            };
            match session.export_pdf() {
                Ok(pdf) => {
                    let len = pdf.len();
                    // Hand the bytes to the host rather than serializing them.
                    // Freed with tectonic_wasm_free once they have been copied
                    // out, exactly like the response buffer itself.
                    let ptr = Box::into_raw(pdf.into_boxed_slice()) as *mut u8;
                    Response::Exported {
                        ptr: ptr as u32,
                        len: len as u32,
                    }
                }
                Err(error) => Response::error(error),
            }
        }),

        Request::ForwardSearch { path, line } => SESSION.with_borrow(|slot| {
            let Some(session) = slot.as_ref() else {
                return Response::error("no session is open");
            };
            Response::Located {
                rect: session.forward_search(&path, line).map(Into::into),
                source: None,
            }
        }),

        Request::BackwardSearch {
            page_index,
            h_bp,
            v_bp,
        } => SESSION.with_borrow(|slot| {
            let Some(session) = slot.as_ref() else {
                return Response::error("no session is open");
            };
            Response::Located {
                rect: None,
                source: session
                    .backward_search(page_index, h_bp, v_bp)
                    .map(Into::into),
            }
        }),

        Request::Cancel => SESSION.with_borrow(|slot| match slot.as_ref() {
            Some(session) => {
                session.cancel();
                Response::Ok
            }
            None => Response::error("no session is open"),
        }),

        Request::Shutdown => SESSION.with_borrow_mut(|slot| match slot.take() {
            Some(mut session) => match session.finish() {
                Ok(()) => Response::Ok,
                Err(error) => Response::error(error),
            },
            None => Response::Ok,
        }),
    }
}

/// Take ownership of bytes the host placed in linear memory.
///
/// # Safety
///
/// `ptr`/`len` must describe a buffer from
/// [`tectonic_wasm_alloc`](host::tectonic_wasm_alloc) that the host has
/// released.
unsafe fn take_host_bytes(ptr: u32, len: u32) -> Vec<u8> {
    if ptr == 0 || len == 0 {
        return Vec::new();
    }

    std::slice::from_raw_parts(ptr as *const u8, len as usize).to_vec()
}

/// Package one compile for the host, moving the PDF into linear memory.
fn report(session: &Session, output: session::CompileOutput) -> CompileReport {
    let (support_files, support_bytes) = session.support_stats();

    CompileReport {
        pages: output
            .pages
            .into_iter()
            .map(|page| RenderedPage {
                index: page.index,
                svg: page.svg,
            })
            .collect(),
        changed: output.changed,
        removed: output.removed,
        page_sizes: output.page_sizes,
        page_count: output.page_count,
        log: String::from_utf8_lossy(&output.log).into_owned(),
        outcome: output.outcome.to_owned(),
        aborted: output.aborted,
        tail_pending: output.tail_pending,
        messages: output.messages,
        diagnostics: output.diagnostics.into_iter().map(Into::into).collect(),
        checkpoint_rungs: output.checkpoint_rungs,
        checkpoint_bytes: output.checkpoint_bytes,
        reused_prefix: output.reused_prefix,
        replay_rung: output.replay_rung,
        replay_commands: output.replay_commands,
        tex_ms: output.tex_ms,
        analyze_ms: output.analyze_ms,
        render_ms: output.render_ms,
        shipouts: output.shipouts,
        pages_fingerprinted: output.pages_fingerprinted,
        pages_rendered: output.pages_rendered,
        unsupported_pages: output.unsupported_pages,
        support_files,
        support_bytes,
    }
}
