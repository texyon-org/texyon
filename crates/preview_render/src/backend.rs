//! Backend seam for platform-specific page drawing.

use crate::{RenderError, ResourceIdentity};

/// Backend-owned font handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontId(pub u32);

/// One glyph positioned in PostScript big points, with a y-down origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedGlyph {
    /// Font glyph identifier.
    pub glyph_id: u16,
    /// Horizontal origin in big points.
    pub x_bp: f32,
    /// Vertical origin in big points.
    pub y_bp: f32,
}

/// Drawing adapter used by the pure XDV core.
pub trait Backend {
    /// Begin a new page.
    fn begin_page(&mut self, width_bp: f32, height_bp: f32) -> Result<(), RenderError>;

    /// Register immutable font bytes, returning a backend-local handle.
    fn register_font(
        &mut self,
        identity: ResourceIdentity,
        bytes: &[u8],
        face_index: u32,
    ) -> Result<FontId, RenderError>;

    /// Fill an axis-aligned rule in black.
    fn fill_rule(
        &mut self,
        x_bp: f32,
        y_bp: f32,
        width_bp: f32,
        height_bp: f32,
    ) -> Result<(), RenderError>;

    /// Fill positioned native-font glyph outlines in black.
    fn fill_glyphs(
        &mut self,
        font: FontId,
        size_bp: f32,
        glyphs: &[PositionedGlyph],
    ) -> Result<(), RenderError>;

    /// Finish the page and return one self-contained SVG document.
    fn finish_page(&mut self) -> Result<String, RenderError>;
}

/// Minimal backend useful for analysis tests and unsupported-page telemetry.
#[derive(Default)]
pub struct NullBackend;

impl Backend for NullBackend {
    fn begin_page(&mut self, _width_bp: f32, _height_bp: f32) -> Result<(), RenderError> {
        Ok(())
    }

    fn register_font(
        &mut self,
        _identity: ResourceIdentity,
        _bytes: &[u8],
        _face_index: u32,
    ) -> Result<FontId, RenderError> {
        Ok(FontId(0))
    }

    fn fill_rule(
        &mut self,
        _x_bp: f32,
        _y_bp: f32,
        _width_bp: f32,
        _height_bp: f32,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn fill_glyphs(
        &mut self,
        _font: FontId,
        _size_bp: f32,
        _glyphs: &[PositionedGlyph],
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn finish_page(&mut self) -> Result<String, RenderError> {
        Ok("<svg xmlns=\"http://www.w3.org/2000/svg\"/>".into())
    }
}
