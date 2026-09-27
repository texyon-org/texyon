//! Portable XDV analysis and rendering for realtime preview.
//!
//! This crate's default build contains no filesystem, native renderer, or
//! platform integration. Callers provide immutable resource bytes and a drawing
//! backend. A native MuPDF adapter can therefore remain an adapter rather than
//! becoming part of page semantics.

mod backend;
mod color;
mod encoding;
mod error;
#[cfg(test)]
mod fidelity;
mod fingerprint;
mod fontmap;
mod links;
mod pagesize;
mod pdfcode;
mod render;
mod resource;
mod tfm;
mod units;

#[cfg(all(
    feature = "backend-mupdf",
    any(
        target_os = "linux",
        all(target_os = "windows", target_env = "msvc"),
        target_family = "wasm"
    )
))]
mod backend_mupdf;

pub use backend::{
    Backend, FontId, FontStyle, Matrix, NullBackend, Path, PathCommand, PositionedChar,
    PositionedGlyph, StrokeStyle,
};
pub use color::Color;
pub use error::RenderError;
#[cfg(test)]
pub use fidelity::{compare_rasters, RasterDiff};
pub use links::{Destination, Link, LinkAction, RectBp};
pub use render::{
    Analysis, CoverageLedger, HotRenderer, PageFingerprint, PageSvg, PageVisual, UnsupportedSummary,
};
pub use resource::{Resource, ResourceIdentity, ResourceKind, ResourceProvider, ResourceRequest};

#[cfg(all(
    feature = "backend-mupdf",
    any(
        target_os = "linux",
        all(target_os = "windows", target_env = "msvc"),
        target_family = "wasm"
    )
))]
pub use backend_mupdf::MupdfBackend;
#[cfg(all(
    test,
    feature = "backend-mupdf",
    any(target_os = "linux", all(target_os = "windows", target_env = "msvc"))
))]
pub use backend_mupdf::Pixmap;
