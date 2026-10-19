//! The JSON messages exchanged with the host.
//!
//! Bulk payloads (SVG, PDF) stay in linear memory as a [`HostBuffer`] rather
//! than being escaped into the JSON. Lengths and positions are in PostScript
//! big points.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use texyon_engine_xetex::introspect::{CommandKind, DefinedCommand};
use texyon_preview_protocol::{SourceLocation, SyncRectPayload};
use texyon_preview_render::LinkAction;

use crate::host::HostBuffer;

/// A camelCase mirror of the JSON Lines protocol's snake_case rectangle.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRect {
    pub page_index: usize,
    pub h_bp: f64,
    /// Baseline.
    pub v_bp: f64,
    pub width_bp: f64,
    pub height_bp: f64,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    /// Without the leading backslash.
    pub name: String,
    /// Includes the optional argument if there is one.
    pub arity: u8,
    /// Whether the first argument is optional.
    pub optional: bool,
    /// `macro`, `protected` or `builtin`.
    pub kind: &'static str,
}

impl From<DefinedCommand> for Command {
    fn from(found: DefinedCommand) -> Self {
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

/// A clickable region, measured from the page's top left.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub uri: Option<String>,
    /// A named destination in this document.
    pub destination: Option<String>,
}

impl From<texyon_preview_render::Link> for Link {
    fn from(link: texyon_preview_render::Link) -> Self {
        let (uri, destination) = match link.action {
            LinkAction::Uri(uri) => (Some(uri), None),
            LinkAction::Destination(name) => (None, Some(name)),
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

/// A named link target, measured from the page's top left.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    pub name: String,
    pub x_bp: f32,
    pub y_bp: f32,
}

impl From<texyon_preview_render::Destination> for Destination {
    fn from(destination: texyon_preview_render::Destination) -> Self {
        Self {
            name: destination.name,
            x_bp: destination.x_bp,
            y_bp: destination.y_bp,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageOverlay {
    pub links: Vec<Link>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    /// `error`, `warning` or `info`.
    pub severity: String,
    /// As the transcript named it.
    pub path: Option<String>,
    /// One-based.
    pub line: Option<usize>,
    pub message: String,
}

impl From<texyon_live_preview::Diagnostic> for Diagnostic {
    fn from(found: texyon_live_preview::Diagnostic) -> Self {
        Self {
            severity: found.severity.to_string(),
            path: found.path,
            line: found.line,
            message: found.message,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSpot {
    pub path: String,
    /// One-based.
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

/// One splice into a file's text, in the host's UTF-16 code units.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceEdit {
    /// The revision the offsets are into.
    pub base: u64,
    pub from: usize,
    pub to: usize,
    pub insert: String,
}

/// A message from the host.
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Request {
    /// Start a session. The format dump must already be preloaded.
    #[serde(rename_all = "camelCase")]
    Open {
        format: String,
        primary: String,
        /// Leave pages out of compile reports and draw them to pixels on request.
        #[serde(default)]
        raster: bool,
    },

    /// Install a support file from a `texyon_wasm_alloc` buffer, taking
    /// ownership of it.
    #[serde(rename_all = "camelCase")]
    Preload {
        name: String,
        ptr: u32,
        len: u32,
    },

    /// Typeset the complete project.
    ///
    /// `files` maps each path to the host's revision counter. The session
    /// pulls the text of any revision it does not hold through
    /// `texyon_host_read_source`, unless `edits` says how to get there from
    /// the one it does.
    #[serde(rename_all = "camelCase")]
    Compile {
        files: BTreeMap<String, u64>,
        #[serde(default)]
        edits: BTreeMap<String, SourceEdit>,
        /// Zero-based inclusive first and last visible page.
        viewport: Option<(usize, usize)>,
        /// Pixels per bp for the changed pages around `viewport`, in a raster session.
        #[serde(default)]
        scale: Option<f32>,
    },

    ContinueTail,

    /// Draw published pages to pixels, in a raster session.
    #[serde(rename_all = "camelCase")]
    Raster {
        pages: Vec<usize>,
        scale: f32,
    },

    Export,

    #[serde(rename_all = "camelCase")]
    ForwardSearch {
        path: String,
        /// One-based.
        line: usize,
    },

    #[serde(rename_all = "camelCase")]
    BackwardSearch {
        page_index: usize,
        h_bp: f64,
        v_bp: f64,
    },

    Commands,

    Cancel,

    Shutdown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPage {
    pub index: usize,
    /// UTF-8 SVG markup.
    #[serde(flatten)]
    pub svg: HostBuffer,
}

/// One page drawn to pixels: width and height as little-endian `u32`, then RGBA rows.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RasterPage {
    pub index: usize,
    #[serde(flatten)]
    pub pixels: HostBuffer,
}

/// The result of one compile, as a patch: `pages` holds only changed pages.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileReport {
    /// False for a cancelled or aborted run, whose page fields mean "keep
    /// what you have".
    pub published: bool,
    pub pages: Vec<RenderedPage>,
    pub rasters: Vec<RasterPage>,
    pub changed: Vec<usize>,
    pub removed: Vec<usize>,
    pub page_sizes: Vec<[f32; 2]>,
    pub page_overlays: Vec<PageOverlay>,
    pub page_count: usize,
    pub log: String,
    /// `spotless`, `warnings` or `errors`.
    pub outcome: String,
    pub aborted: Option<String>,
    pub tail_pending: bool,
    pub messages: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub checkpoint_rungs: usize,
    pub checkpoint_bytes: usize,
    pub reused_prefix: bool,
    pub replay_rung: usize,
    pub replay_commands: u64,
    pub tex_ms: f64,
    pub analyze_ms: f64,
    pub render_ms: f64,
    pub shipouts: usize,
    pub pages_fingerprinted: usize,
    pub pages_rendered: usize,
    pub unsupported_pages: usize,
    pub support_files: usize,
    pub support_bytes: usize,
}

/// A reply to the host.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Response {
    Ok,
    Compiled(Box<CompileReport>),
    /// A PDF in linear memory.
    Exported(HostBuffer),
    #[serde(rename_all = "camelCase")]
    Located {
        rect: Option<SyncRect>,
        /// The boxes to flash, on the page of `rect`, best first.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        boxes: Vec<SyncRect>,
        source: Option<SourceSpot>,
    },
    #[serde(rename_all = "camelCase")]
    Commands {
        commands: Vec<Command>,
    },
    #[serde(rename_all = "camelCase")]
    Rastered {
        pages: Vec<RasterPage>,
    },
    /// Nothing was pending.
    Idle,
    #[serde(rename_all = "camelCase")]
    Error {
        message: String,
    },
}

impl Response {
    pub fn error(message: impl std::fmt::Display) -> Response {
        Response::Error {
            message: message.to_string(),
        }
    }
}
