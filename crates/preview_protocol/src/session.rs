//! The request and response messages of a preview session, shared by every
//! host.
//!
//! Bulk payloads (SVG, pixels, PDF) are a host-chosen type `B`, flattened
//! into the message that carries them: the wasm host leaves them in linear
//! memory and sends their address, the native host sends the bytes inline.
//! Lengths and positions are in PostScript big points.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{SourceLocation, SyncRectPayload};

/// Version of these messages. A native client passes it to the server, which refuses a mismatch.
pub const SESSION_PROTOCOL_VERSION: u32 = 2;

/// Bytes carried inline, for a host on the other side of a byte stream.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct InlineBytes {
    /// The payload.
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

/// A preview rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRect {
    /// Zero-based page.
    pub page_index: usize,
    /// Left edge.
    pub h_bp: f64,
    /// Baseline.
    pub v_bp: f64,
    /// Width.
    pub width_bp: f64,
    /// Height above the baseline.
    pub height_bp: f64,
    /// Depth below the baseline.
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

/// How a defined control sequence was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandKind {
    /// `\def` and friends.
    Macro,
    /// `\protected\def`.
    Protected,
    /// A primitive.
    Builtin,
}

/// A control sequence the engine has defined.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    /// Without the leading backslash.
    pub name: String,
    /// Includes the optional argument if there is one.
    pub arity: u8,
    /// Whether the first argument is optional.
    pub optional: bool,
    /// How it was made.
    pub kind: CommandKind,
}

/// A clickable region, measured from the page's top left.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// Left edge.
    pub left: f32,
    /// Top edge.
    pub top: f32,
    /// Right edge.
    pub right: f32,
    /// Bottom edge.
    pub bottom: f32,
    /// An external target.
    pub uri: Option<String>,
    /// A named destination in this document.
    pub destination: Option<String>,
}

/// A named link target, measured from the page's top left.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    /// The name links refer to.
    pub name: String,
    /// Horizontal position.
    pub x_bp: f32,
    /// Vertical position.
    pub y_bp: f32,
}

/// What a viewer draws over one page.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageOverlay {
    /// Clickable regions.
    pub links: Vec<Link>,
    /// Named link targets.
    pub destinations: Vec<Destination>,
}

/// How much a diagnostic matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// TeX could not do what was asked and recovered as best it could.
    Error,
    /// Something is suspect but the output exists.
    Warning,
    /// Chatter worth surfacing only on request.
    Info,
}

/// One thing the engine said about one place in the source.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    /// How much it matters.
    pub severity: Severity,
    /// The file it belongs to as the transcript named it, if any was open.
    pub path: Option<String>,
    /// One-based source line, when the engine named one.
    pub line: Option<usize>,
    /// What it said, with the transcript's wrapping undone.
    pub message: String,
}

/// A source line.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSpot {
    /// Relative to the document root.
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
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceEdit {
    /// The revision the offsets are into.
    pub base: u64,
    /// Start of the replaced range.
    pub from: usize,
    /// End of the replaced range, exclusive.
    pub to: usize,
    /// The replacement text.
    pub insert: String,
}

/// A message from the host.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Request<B> {
    /// One the host answers itself.
    Host(HostRequest<B>),
    /// One the open session answers.
    Session(SessionRequest),
}

/// A request only the host can answer, since it creates or ends the session
/// or hands over host memory.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum HostRequest<B> {
    /// Start a session.
    #[serde(rename_all = "camelCase")]
    Open {
        /// Format name, as the host's support files spell it.
        format: String,
        /// The primary source, relative to `root`.
        primary: String,
        /// The project directory, for a host with a file system.
        #[serde(default)]
        root: Option<String>,
        /// Leave pages out of compile reports and draw them to pixels instead.
        #[serde(default)]
        raster: bool,
    },

    /// Install a support file the host already holds.
    #[serde(rename_all = "camelCase")]
    Preload {
        /// The name the engine opens it by.
        name: String,
        /// Its bytes.
        #[serde(flatten)]
        bytes: B,
    },

    /// End the session.
    Shutdown,
}

/// A request the open session answers.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum SessionRequest {
    /// Typeset the complete project.
    ///
    /// `files` maps each path the host holds to its revision counter. The
    /// session asks the host for the text of any revision it does not hold,
    /// unless `texts` carries it or `edits` says how to get there from the
    /// one it does.
    #[serde(rename_all = "camelCase")]
    Compile {
        /// Every source path the host holds, with its revision.
        files: BTreeMap<String, u64>,
        /// Splices from a held revision.
        #[serde(default)]
        edits: BTreeMap<String, SourceEdit>,
        /// Complete texts, for files the session may not hold.
        #[serde(default)]
        texts: BTreeMap<String, String>,
        /// Zero-based inclusive first and last visible page.
        viewport: Option<(usize, usize)>,
        /// Pixels per bp for the changed pages around `viewport`, in a raster session.
        #[serde(default)]
        scale: Option<f32>,
    },

    /// Typeset what a viewport-bounded compile left.
    ContinueTail,

    /// Draw published pages to pixels, in a raster session.
    #[serde(rename_all = "camelCase")]
    Raster {
        /// Zero-based page indices.
        pages: Vec<usize>,
        /// Pixels per bp.
        scale: f32,
    },

    /// Write the current document as PDF.
    Export,

    /// Find the boxes a source line produced.
    #[serde(rename_all = "camelCase")]
    ForwardSearch {
        /// Relative to the document root.
        path: String,
        /// One-based.
        line: usize,
    },

    /// Find the source line that produced a point.
    #[serde(rename_all = "camelCase")]
    BackwardSearch {
        /// Zero-based page.
        page_index: usize,
        /// Horizontal position.
        h_bp: f64,
        /// Vertical position, from the top.
        v_bp: f64,
    },

    /// List the control sequences the engine has defined.
    Commands,

    /// Stop the running operation.
    Cancel,
}

/// One page as SVG.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPage<B> {
    /// Zero-based page index.
    pub index: usize,
    /// UTF-8 SVG markup.
    #[serde(flatten)]
    pub svg: B,
}

/// One page drawn to pixels: width and height as little-endian `u32`, then RGBA rows.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RasterPage<B> {
    /// Zero-based page index.
    pub index: usize,
    /// The pixels.
    #[serde(flatten)]
    pub pixels: B,
}

/// How TeX graded a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    /// No warnings.
    Spotless,
    /// Warnings, or nothing new was typeset.
    #[default]
    Warnings,
    /// Errors.
    Errors,
}

/// The result of one compile, as a patch: `pages` and `rasters` hold only changed pages.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileReport<B> {
    /// False for a cancelled or aborted run, whose page fields mean "keep
    /// what you have".
    pub published: bool,
    /// The changed pages as SVG, in a session that does not draw pixels.
    pub pages: Vec<RenderedPage<B>>,
    /// The changed pages around the viewport, in a raster session.
    pub rasters: Vec<RasterPage<B>>,
    /// Added or changed page indices.
    pub changed: Vec<usize>,
    /// Removed page indices.
    pub removed: Vec<usize>,
    /// Width and height of every page.
    pub page_sizes: Vec<[f32; 2]>,
    /// Links and destinations of every page.
    pub page_overlays: Vec<PageOverlay>,
    /// Pages after this patch.
    pub page_count: usize,
    /// The TeX transcript.
    pub log: String,
    /// How TeX graded the run.
    pub outcome: Outcome,
    /// Why the run was abandoned, keeping the last frame.
    pub aborted: Option<String>,
    /// Whether pages past the viewport are left to typeset.
    pub tail_pending: bool,
    /// Status lines from the host and the renderer.
    pub messages: Vec<String>,
    /// Errors and warnings by source line.
    pub diagnostics: Vec<Diagnostic>,
    /// Checkpoints held for rewinding.
    pub checkpoint_rungs: usize,
    /// Bytes those checkpoints hold.
    pub checkpoint_bytes: usize,
    /// Whether the run resumed from a checkpoint.
    pub reused_prefix: bool,
    /// The checkpoint it resumed from.
    pub replay_rung: usize,
    /// Commands replayed to get there.
    pub replay_commands: u64,
    /// Engine time.
    pub tex_ms: f64,
    /// Page analysis time.
    pub analyze_ms: f64,
    /// Drawing time.
    pub render_ms: f64,
    /// Pages the engine shipped.
    pub shipouts: usize,
    /// Pages fingerprinted to find changes.
    pub pages_fingerprinted: usize,
    /// Pages drawn.
    pub pages_rendered: usize,
    /// Pages the renderer had to approximate.
    pub unsupported_pages: usize,
    /// Support files resident, for a host that sends them in.
    pub support_files: usize,
    /// Bytes of those support files.
    pub support_bytes: usize,
}

/// A reply to the host.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Response<B> {
    /// Done, with nothing to report.
    Ok,
    /// A compile or tail run finished.
    Compiled(Box<CompileReport<B>>),
    /// A PDF.
    Exported(B),
    /// The answer to a search.
    #[serde(rename_all = "camelCase")]
    Located {
        /// Where a viewer scrolls to.
        rect: Option<SyncRect>,
        /// The boxes to flash, on the page of `rect`, best first.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        boxes: Vec<SyncRect>,
        /// The source line, for a backward search.
        source: Option<SourceSpot>,
    },
    /// The defined control sequences.
    #[serde(rename_all = "camelCase")]
    Commands {
        /// Every one of them.
        commands: Vec<Command>,
    },
    /// Pages drawn to pixels.
    #[serde(rename_all = "camelCase")]
    Rastered {
        /// The drawn pages.
        pages: Vec<RasterPage<B>>,
    },
    /// Nothing was pending.
    Idle,
    /// The request failed.
    #[serde(rename_all = "camelCase")]
    Error {
        /// What went wrong.
        message: String,
    },
}

impl<B> Response<B> {
    /// A failure reply.
    pub fn error(message: impl std::fmt::Display) -> Self {
        Self::Error {
            message: message.to_string(),
        }
    }
}
