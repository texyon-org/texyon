//! Portable XDV analysis and rendering for realtime preview.
//!
//! This crate's default build contains no filesystem, native renderer, or
//! platform integration. Callers provide immutable resource bytes and a drawing
//! backend. A Linux MuPDF adapter can therefore remain an adapter rather than
//! becoming part of page semantics.

mod backend;
mod color;
mod fingerprint;
mod pagesize;
mod render;
mod resource;
mod tfm;

pub use backend::{
    Backend, FontId, FontStyle, Matrix, NullBackend, PositionedChar, PositionedGlyph,
};
pub use color::Color;
pub use render::{
    Analysis, CoverageLedger, HotRenderer, PageFingerprint, PageSvg, PageVisual, RenderError,
    UnsupportedSummary,
};
pub use resource::{Resource, ResourceIdentity, ResourceKind, ResourceProvider, ResourceRequest};
