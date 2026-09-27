//! Frozen per-page drawing plans produced by analysis.

use crate::backend::{Matrix, Path, PositionedChar, PositionedGlyph, StrokeStyle};
use crate::color::Color;
use crate::links::{Destination, Link};
use crate::resource::{ResourceIdentity, ResourceKind};

use super::UnsupportedSummary;

#[derive(Clone)]
pub(crate) struct PagePlan {
    pub(crate) width_bp: f32,
    pub(crate) height_bp: f32,
    pub(crate) operations: Vec<DrawOperation>,
    pub(crate) links: Vec<Link>,
    pub(crate) destinations: Vec<Destination>,
    pub(crate) unsupported: UnsupportedSummary,
    pub(crate) complete: bool,
}

#[derive(Clone)]
pub(crate) enum DrawOperation {
    Rule {
        height: i32,
        width: i32,
        transform: Matrix,
        color: Color,
    },
    NativeGlyphs {
        identity: ResourceIdentity,
        face_index: u32,
        size: i32,
        glyphs: Vec<PositionedGlyph>,
        color: Color,
        extend: u32,
        slant: u32,
        embolden: u32,
        vertical: bool,
    },
    Image {
        identity: ResourceIdentity,
        kind: ImageKind,
        transform: Matrix,
        page: u32,
        width: Option<f32>,
        height: Option<f32>,
    },
    ClassicChars {
        tfm_identity: ResourceIdentity,
        program_identity: ResourceIdentity,
        encoding_identity: ResourceIdentity,
        scale: u32,
        chars: Vec<PositionedChar>,
        color: Color,
    },
    PathFill {
        path: Path,
        even_odd: bool,
        transform: Matrix,
        color: Color,
    },
    PathStroke {
        path: Path,
        style: StrokeStyle,
        transform: Matrix,
        color: Color,
    },
    ClipPath {
        path: Path,
        even_odd: bool,
        transform: Matrix,
    },
    PopClip,
}

#[derive(Clone, Copy)]
pub(crate) enum ImageKind {
    Raster,
    Pdf,
}

impl ImageKind {
    pub(crate) fn resource_kind(self) -> ResourceKind {
        match self {
            Self::Raster => ResourceKind::Image,
            Self::Pdf => ResourceKind::Pdf,
        }
    }

    pub(crate) fn ledger_name(self) -> &'static str {
        match self {
            Self::Raster => "raster-image",
            Self::Pdf => "embedded-pdf",
        }
    }
}
