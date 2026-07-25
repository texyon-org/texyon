//! The JSON messages exchanged with the host.
//!
//! Control messages go through JSON because it keeps the C ABI to a single
//! entry point and the TypeScript types honest. Bulk payloads do not: the PDF
//! is left in linear memory and reported as an offset and a length, so a
//! multi-megabyte document never becomes a base64 string.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tectonic_preview_protocol::{SourceLocation, SyncRectPayload};

/// Where a source line landed on the page.
///
/// A camelCase mirror of the JSON Lines protocol's own rectangle. The two carry
/// the same numbers; only the spelling differs, because that protocol is
/// snake_case on the wire and this one is not.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRect {
    /// Zero-based output page.
    pub page_index: usize,
    /// Left edge in PostScript big points.
    pub h_bp: f64,
    /// Baseline in PostScript big points.
    pub v_bp: f64,
    /// Width in PostScript big points.
    pub width_bp: f64,
    /// Height above the baseline in PostScript big points.
    pub height_bp: f64,
    /// Depth below the baseline in PostScript big points.
    pub depth_bp: f64,
}

impl From<SyncRectPayload> for SyncRect {
    fn from(rect: SyncRectPayload) -> Self {
        Self {
            page_index: rect.page_index,
            h_bp: rect.h_bp,
            v_bp: rect.v_bp,
            width_bp: rect.width_bp,
            height_bp: rect.height_bp,
            depth_bp: rect.depth_bp,
        }
    }
}

/// One command the engine has defined.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    /// The name, without its leading backslash.
    pub name: String,
    /// How many arguments it takes, including an optional one if it has it.
    pub arity: u8,
    /// Whether the first argument is optional.
    pub optional: bool,
    /// `macro`, `protected` or `builtin`.
    pub kind: &'static str,
}

impl From<tectonic_engine_xetex::introspect::DefinedCommand> for Command {
    fn from(found: tectonic_engine_xetex::introspect::DefinedCommand) -> Self {
        use tectonic_engine_xetex::introspect::CommandKind;
        Self {
            name: found.name,
            arity: found.arity,
            optional: found.optional,
            kind: match found.kind {
                CommandKind::Macro => "macro",
                CommandKind::ProtectedMacro => "protected",
                CommandKind::Builtin => "builtin",
            },
        }
    }
}

/// A clickable region of a page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// Left edge in big points from the page's left.
    pub left: f32,
    /// Top edge in big points from the page's top.
    pub top: f32,
    /// Right edge.
    pub right: f32,
    /// Bottom edge.
    pub bottom: f32,
    /// An absolute URL, for a link that leaves the document.
    pub uri: Option<String>,
    /// A named destination in this document, for one that does not.
    pub destination: Option<String>,
}

impl From<tectonic_preview_render::Link> for Link {
    fn from(link: tectonic_preview_render::Link) -> Self {
        let (uri, destination) = match link.action {
            tectonic_preview_render::LinkAction::Uri(uri) => (Some(uri), None),
            tectonic_preview_render::LinkAction::Destination(name) => (None, Some(name)),
        };
        Self {
            left: link.rect.left,
            top: link.rect.top,
            right: link.rect.right,
            bottom: link.rect.bottom,
            uri,
            destination,
        }
    }
}

/// A named spot on a page that a link can reach.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    /// The name `\label` and its kin produced.
    pub name: String,
    /// Horizontal position in big points from the page's left edge.
    pub x_bp: f32,
    /// Vertical position in big points from the page's top edge.
    pub y_bp: f32,
}

impl From<tectonic_preview_render::Destination> for Destination {
    fn from(destination: tectonic_preview_render::Destination) -> Self {
        Self {
            name: destination.name,
            x_bp: destination.x_bp,
            y_bp: destination.y_bp,
        }
    }
}

/// What one page offers beyond its pixels.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageOverlay {
    /// Clickable regions.
    pub links: Vec<Link>,
    /// Named spots a link can reach.
    pub destinations: Vec<Destination>,
}

impl From<crate::session::PageOverlay> for PageOverlay {
    fn from(overlay: crate::session::PageOverlay) -> Self {
        Self {
            links: overlay.links.into_iter().map(Into::into).collect(),
            destinations: overlay.destinations.into_iter().map(Into::into).collect(),
        }
    }
}

/// One thing the engine said about one place in the source.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    /// `error`, `warning` or `info`.
    pub severity: String,
    /// The file it belongs to, as the transcript named it.
    pub path: Option<String>,
    /// One-based source line, when the engine named one.
    pub line: Option<usize>,
    /// What it said.
    pub message: String,
}

impl From<tectonic_live_preview::Diagnostic> for Diagnostic {
    fn from(found: tectonic_live_preview::Diagnostic) -> Self {
        Self {
            severity: found.severity.to_string(),
            path: found.path,
            line: found.line,
            message: found.message,
        }
    }
}

/// Which source line produced a point on a page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSpot {
    /// Source path as the engine recorded it.
    pub path: String,
    /// One-based source line.
    pub line: usize,
}

impl From<SourceLocation> for SourceSpot {
    fn from(location: SourceLocation) -> Self {
        Self {
            path: location.path,
            line: location.line,
        }
    }
}

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

    /// Typeset the project at these file contents.
    ///
    /// `files` is the project's complete current state, keyed by path relative
    /// to the document root, and must contain the primary named at `open`.
    /// Files whose text is unchanged cost nothing, so sending the whole project
    /// on every keystroke is both the intended usage and the cheap one.
    ///
    /// `viewport` bounds the foreground work to a visible page range; the rest
    /// is left for `ContinueTail`.
    #[serde(rename_all = "camelCase")]
    Compile {
        /// Every source file's complete current text, by logical path.
        files: BTreeMap<String, String>,
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

    /// Find where a source line landed on the page.
    #[serde(rename_all = "camelCase")]
    ForwardSearch {
        /// Source path, as the caller spells it.
        path: String,
        /// One-based source line.
        line: usize,
    },

    /// Find which source line produced a point on a page.
    #[serde(rename_all = "camelCase")]
    BackwardSearch {
        /// Zero-based page index.
        page_index: usize,
        /// Horizontal position in PostScript big points.
        h_bp: f64,
        /// Vertical position in PostScript big points.
        v_bp: f64,
    },

    /// List every command the document currently has defined.
    Commands,

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
    /// Every page's clickable regions and cross-reference targets.
    pub page_overlays: Vec<PageOverlay>,
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
    /// What the engine said, and where, recovered from the transcript.
    pub diagnostics: Vec<Diagnostic>,
    /// Checkpoint rungs retained.
    pub checkpoint_rungs: usize,
    /// Bytes held in the checkpoint ladder.
    pub checkpoint_bytes: usize,
    /// Whether the incremental replay reused a prefix of the previous XDV.
    pub reused_prefix: bool,
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
    /// Fresh pages the engine shipped out.
    pub shipouts: usize,
    /// Pages the analysis fingerprinted.
    pub pages_fingerprinted: usize,
    /// Pages drawn again, of those fingerprinted.
    pub pages_rendered: usize,
    /// Pages the renderer had to approximate.
    pub unsupported_pages: usize,
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
    /// A SyncTeX query was answered. `result` is null when nothing matched,
    /// which is normal rather than an error.
    #[serde(rename_all = "camelCase")]
    Located {
        /// Where the source line landed, for a forward search.
        rect: Option<SyncRect>,
        /// Which source line produced the point, for a backward search.
        source: Option<SourceSpot>,
    },
    /// The commands the document has defined.
    #[serde(rename_all = "camelCase")]
    Commands {
        /// Every command, in the order the engine's table holds them.
        commands: Vec<Command>,
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
