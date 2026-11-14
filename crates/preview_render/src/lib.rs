//! Portable XDV analysis and rendering for realtime preview.
//!
//! The default build has no filesystem or native renderer: callers supply
//! resource bytes and a drawing backend.

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
#[cfg(test)]
mod testxdv;
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
pub use fidelity::{RasterDiff, compare_rasters};
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
