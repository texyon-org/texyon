//! Backend seam for platform-specific page drawing.

use crate::{Color, RenderError, ResourceIdentity};

/// Affine transform in backend point space.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix {
    /// Horizontal scale/rotation component.
    pub a: f32,
    /// Vertical rotation/shear component.
    pub b: f32,
    /// Horizontal rotation/shear component.
    pub c: f32,
    /// Vertical scale/rotation component.
    pub d: f32,
    /// Horizontal translation.
    pub e: f32,
    /// Vertical translation.
    pub f: f32,
}

impl Matrix {
    pub(crate) fn dvi_origin(x: i32, y: i32, scale: f32) -> Self {
        Self {
            a: scale,
            b: 0.0,
            c: 0.0,
            d: -scale,
            e: x as f32 * scale,
            f: y as f32 * scale,
        }
    }

    pub(crate) fn concat(self, other: Self) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }
}

/// Backend-owned font handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// One classic-font character positioned in big points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedChar {
    /// TeX character code.
    pub character: u32,
    /// Horizontal origin in big points.
    pub x_bp: f32,
    /// Vertical origin in big points.
    pub y_bp: f32,
}

/// XeTeX native-font transforms and synthetic weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontStyle {
    /// Horizontal scale multiplier.
    pub extend: f32,
    /// Horizontal slant/shear.
    pub slant: f32,
    /// Synthetic emboldening in font-space units.
    pub embolden: f32,
    /// Use vertical native-font layout.
    pub vertical: bool,
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

    /// Register a classic font program and its frozen encoding vector.
    fn register_classic_font(
        &mut self,
        identity: ResourceIdentity,
        bytes: &[u8],
        encoding_identity: ResourceIdentity,
        encoding_bytes: &[u8],
    ) -> Result<FontId, RenderError>;

    /// Fill an axis-aligned rule in `color`.
    fn fill_rule(
        &mut self,
        x_bp: f32,
        y_bp: f32,
        width_bp: f32,
        height_bp: f32,
        color: Color,
    ) -> Result<(), RenderError>;

    /// Fill positioned native-font glyph outlines in `color`.
    fn fill_glyphs(
        &mut self,
        font: FontId,
        size_bp: f32,
        glyphs: &[PositionedGlyph],
        color: Color,
        style: FontStyle,
    ) -> Result<(), RenderError>;

    /// Fill positioned classic-font characters.
    fn fill_chars(
        &mut self,
        font: FontId,
        size_bp: f32,
        chars: &[PositionedChar],
        color: Color,
    ) -> Result<(), RenderError>;

    /// Draw an encoded raster image using its analyzed transform.
    fn draw_image(
        &mut self,
        bytes: &[u8],
        transform: Matrix,
        width_bp: Option<f32>,
        height_bp: Option<f32>,
    ) -> Result<(), RenderError>;

    /// Draw one zero-based page from an embedded PDF.
    fn draw_pdf(&mut self, bytes: &[u8], page: u32, transform: Matrix) -> Result<(), RenderError>;

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

    fn register_classic_font(
        &mut self,
        _identity: ResourceIdentity,
        _bytes: &[u8],
        _encoding_identity: ResourceIdentity,
        _encoding_bytes: &[u8],
    ) -> Result<FontId, RenderError> {
        Ok(FontId(0))
    }

    fn fill_rule(
        &mut self,
        _x_bp: f32,
        _y_bp: f32,
        _width_bp: f32,
        _height_bp: f32,
        _color: Color,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn fill_glyphs(
        &mut self,
        _font: FontId,
        _size_bp: f32,
        _glyphs: &[PositionedGlyph],
        _color: Color,
        _style: FontStyle,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn fill_chars(
        &mut self,
        _font: FontId,
        _size_bp: f32,
        _chars: &[PositionedChar],
        _color: Color,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn draw_image(
        &mut self,
        _bytes: &[u8],
        _transform: Matrix,
        _width_bp: Option<f32>,
        _height_bp: Option<f32>,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn draw_pdf(
        &mut self,
        _bytes: &[u8],
        _page: u32,
        _transform: Matrix,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn finish_page(&mut self) -> Result<String, RenderError> {
        Ok("<svg xmlns=\"http://www.w3.org/2000/svg\"/>".into())
    }
}
