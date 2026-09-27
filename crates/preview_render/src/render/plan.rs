//! Frozen per-page drawing plans produced by analysis.

use crate::backend::{Matrix, Path, StrokeStyle};
use crate::color::Color;
use crate::links::{Destination, Link};
use crate::resource::ResourceIdentity;

use super::UnsupportedSummary;

pub(crate) const SP_TO_BP: f32 = 72.0 / 72.27 / 65_536.0;

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
        glyphs: Vec<GlyphPosition>,
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
        chars: Vec<ClassicPosition>,
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

#[derive(Clone)]
pub(crate) struct GlyphPosition {
    pub(crate) glyph_id: u16,
    pub(crate) transform: Matrix,
}

#[derive(Clone)]
pub(crate) struct ClassicPosition {
    pub(crate) character: i32,
    pub(crate) transform: Matrix,
}

pub(crate) fn fixed_16_16(value: u32) -> f32 {
    i32::from_be_bytes(value.to_be_bytes()) as f32 / 65_536.0
}
