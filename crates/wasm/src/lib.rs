//! Tectonic's live-preview engine as a WebAssembly module.
//!
//! Everything goes through [`tectonic_request`]: a JSON request in, a JSON
//! response out, with bulk payloads left in linear memory. The host supplies
//! the imports declared in `host`.

// Linked in for its C allocation symbols, never called from Rust.
#[cfg(target_family = "wasm")]
mod alloc_shim;

mod host;
mod io;
mod protocol;
mod resources;
mod session;
mod sink;
mod support;

use std::cell::RefCell;

use host::HostBuffer;
use protocol::{CompileReport, RenderedPage, Request, Response};
use session::{CompileOutput, Session};

thread_local! {
    // XeTeX keeps process-global state, so one module instance hosts one session.
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

/// Handle one JSON request from the host.
///
/// Returns 0 with the reply in `out_ptr`/`out_len`, which the host releases
/// with `tectonic_wasm_free`, or -1 if no reply could be encoded.
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

/// Run `operation` on the open session, or report that there is none.
fn with_session(operation: impl FnOnce(&mut Session) -> Response) -> Response {
    SESSION.with_borrow_mut(|slot| match slot.as_mut() {
        Some(session) => operation(session),
        None => Response::error("no session is open"),
    })
}

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

        Request::Preload { name, ptr, len } => with_session(|session| {
            // SAFETY: the host allocated this with tectonic_wasm_alloc and gave
            // up ownership by sending the request.
            let bytes = unsafe { HostBuffer { ptr, len }.adopt() };
            session.preload(&name, bytes);
            Response::Ok
        }),

        Request::Compile { files, viewport } => {
            with_session(|session| match session.compile(files, viewport) {
                Ok(output) => Response::Compiled(Box::new(report(session, output))),
                Err(error) => Response::error(error),
            })
        }

        Request::ContinueTail => with_session(|session| match session.continue_tail() {
            Ok(Some(output)) => Response::Compiled(Box::new(report(session, output))),
            Ok(None) => Response::Idle,
            Err(error) => Response::error(error),
        }),

        Request::Export => with_session(|session| match session.export_pdf() {
            Ok(pdf) => Response::Exported(HostBuffer::hand_over(pdf)),
            Err(error) => Response::error(error),
        }),

        Request::ForwardSearch { path, line } => with_session(|session| {
            let found = session.forward_search(&path, line);
            Response::Located {
                rect: found.result.map(Into::into),
                boxes: found.boxes.into_iter().map(Into::into).collect(),
                source: None,
            }
        }),

        Request::BackwardSearch {
            page_index,
            h_bp,
            v_bp,
        } => with_session(|session| Response::Located {
            rect: None,
            boxes: Vec::new(),
            source: session
                .backward_search(page_index, h_bp, v_bp)
                .map(Into::into),
        }),

        Request::Commands => with_session(|session| Response::Commands {
            commands: session
                .defined_commands()
                .into_iter()
                .map(Into::into)
                .collect(),
        }),

        Request::Cancel => with_session(|session| {
            session.cancel();
            Response::Ok
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

/// Package one compile for the host, moving the SVG into linear memory.
fn report(session: &Session, output: CompileOutput) -> CompileReport {
    let (support_files, support_bytes) = session.support_stats();

    CompileReport {
        published: output.published,
        pages: output
            .pages
            .into_iter()
            .map(|page| RenderedPage {
                index: page.index,
                svg: HostBuffer::hand_over(page.svg.into_bytes()),
            })
            .collect(),
        changed: output.changed,
        removed: output.removed,
        page_sizes: output.page_sizes,
        page_overlays: output.page_overlays.into_iter().map(Into::into).collect(),
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
