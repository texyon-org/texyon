//! The JSON messages exchanged with the host.
//!
//! Control messages go through JSON because it keeps the C ABI to a single
//! entry point and the TypeScript types honest. Bulk payloads do not: the PDF
//! is left in linear memory and reported as an offset and a length, so a
//! multi-megabyte document never becomes a base64 string.

use serde::{Deserialize, Serialize};

/// A message from the host.
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Request {
    /// Start a session for one document.
    ///
    /// The format dump must already have been handed over with `preload`.
    #[serde(rename_all = "camelCase")]
    Open {
        /// Name the engine should load its format dump under.
        format: String,
        /// The document's main file name, e.g. `main.tex`.
        primary: String,
    },

    /// Install a support file the host already has, skipping a round trip.
    ///
    /// The bytes must have been written into linear memory at `ptr` by way of
    /// `tectonic_wasm_alloc`; this takes ownership of them.
    #[serde(rename_all = "camelCase")]
    Preload {
        /// TeX name to file the bytes under.
        name: String,
        /// Offset in linear memory.
        ptr: u32,
        /// Length in bytes.
        len: u32,
    },

    /// Typeset the document at this text.
    ///
    /// `viewport` bounds the foreground work to a visible page range; the rest
    /// is left for `ContinueTail`.
    #[serde(rename_all = "camelCase")]
    Compile {
        /// The complete current text of the primary file.
        text: String,
        /// Zero-based inclusive first and last visible page.
        viewport: Option<(usize, usize)>,
    },

    /// Finish a viewport-bounded compile through the end of the document.
    ContinueTail,

    /// Convert the current document to PDF.
    ///
    /// The bytes are left in linear memory and reported by
    /// [`Response::Exported`]; the host must release them with
    /// `tectonic_wasm_free`.
    Export,

    /// Ask the running operation to stop at its next cancellation point.
    Cancel,

    /// Release the engine.
    Shutdown,
}

/// One page of the rendered document.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPage {
    /// Zero-based page index.
    pub index: usize,
    /// Self-contained SVG markup.
    pub svg: String,
}

/// The result of one compile.
///
/// This is a *patch*, not a snapshot: `pages` carries only the pages whose
/// appearance changed, and the viewer keeps the rest of what it already has.
/// That is the point of the renderer's fingerprinting — an edit in the last
/// paragraph of a long document redraws one page.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileReport {
    /// SVG for the changed pages. Its indices are exactly `changed`.
    pub pages: Vec<RenderedPage>,
    /// Indices of the pages that changed.
    pub changed: Vec<usize>,
    /// Indices the document no longer has, which the viewer should drop.
    pub removed: Vec<usize>,
    /// Every page's size in PostScript big points, so the viewer can lay out
    /// pages it has not been sent.
    pub page_sizes: Vec<[f32; 2]>,
    /// Pages in the document.
    pub page_count: usize,
    /// TeX's `.log` transcript.
    pub log: String,
    /// `spotless`, `warnings` or `errors`.
    pub outcome: String,
    /// A recoverable fatal-class abort, if one happened.
    pub aborted: Option<String>,
    /// Whether a tail continuation is pending.
    pub tail_pending: bool,
    /// Diagnostics gathered during the run.
    pub messages: Vec<String>,
    /// Checkpoint rungs retained.
    pub checkpoint_rungs: usize,
    /// Bytes held in the checkpoint ladder.
    pub checkpoint_bytes: usize,
    /// XDV byte prefix the incremental replay reused, when it took that path.
    pub splice_at: Option<usize>,
    /// Checkpoint rung the run resumed from; above zero means the preamble was
    /// skipped.
    pub replay_rung: usize,
    /// TeX commands executed after the replay origin.
    pub replay_commands: u64,
    /// Milliseconds spent in XeTeX.
    pub tex_ms: f64,
    /// Milliseconds spent analyzing the XDV.
    pub analyze_ms: f64,
    /// Milliseconds spent drawing the changed pages.
    pub render_ms: f64,
    /// Pages the analysis reused without re-walking them.
    pub reused_pages: usize,
    /// Support files resident in the module.
    pub support_files: usize,
    /// Bytes of support files resident in the module.
    pub support_bytes: usize,
}

/// A reply to the host.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Response {
    /// The request succeeded and produced no payload.
    Ok,
    /// A compile finished.
    Compiled(Box<CompileReport>),
    /// A PDF was produced and left in linear memory.
    ///
    /// Deliberately not carried in the JSON: a fifty-page document is megabytes
    /// of PDF, and base64 through a JSON string would cost a copy, a third again
    /// in size, and a parse. The host copies the bytes out and frees them with
    /// `tectonic_wasm_free`.
    #[serde(rename_all = "camelCase")]
    Exported {
        /// Offset of the PDF in linear memory.
        ptr: u32,
        /// Its length in bytes.
        len: u32,
    },
    /// Nothing was pending.
    Idle,
    /// The request failed.
    #[serde(rename_all = "camelCase")]
    Error {
        /// What went wrong, for display.
        message: String,
    },
}

impl Response {
    /// Build an error reply from anything displayable.
    pub fn error(message: impl std::fmt::Display) -> Response {
        Response::Error {
            message: message.to_string(),
        }
    }
}
