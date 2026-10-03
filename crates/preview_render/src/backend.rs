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

/// One PDF path construction command.
#[derive(Clone, Debug, PartialEq)]
pub enum PathCommand {
    /// Begin a subpath.
    MoveTo(f32, f32),
    /// Add a straight segment.
    LineTo(f32, f32),
    /// Add a cubic Bézier segment.
    CurveTo(f32, f32, f32, f32, f32, f32),
    /// Add an axis-aligned rectangle `(x0, y0, x1, y1)`.
    Rect(f32, f32, f32, f32),
    /// Close the current subpath.
    Close,
}

/// Persistent PDF content-stream path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    /// Path commands in construction order.
    pub commands: Vec<PathCommand>,
}

/// PDF line drawing parameters.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    /// Line width in content-stream units.
    pub width: f32,
    /// PDF line join code.
    pub join: i32,
    /// PDF line cap code.
    pub cap: i32,
    /// Miter limit.
    pub miter_limit: f32,
    /// Dash array, bounded to four entries.
    pub dash: [f32; 4],
    /// Active entries in `dash`.
    pub dash_len: usize,
    /// Dash phase.
    pub dash_phase: f32,
}

impl Matrix {
    pub(crate) const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

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
    /// Affine map from glyph-local, y-up big points into page space.
    pub transform: Matrix,
}

/// One classic-font character positioned in big points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedChar {
    /// TeX character code.
    pub character: u32,
    /// Affine map from glyph-local, y-up big points into page space.
    pub transform: Matrix,
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

    /// Fill a rule rectangle mapped into page space by `transform`.
    fn fill_rule(
        &mut self,
        width_bp: f32,
        height_bp: f32,
        transform: Matrix,
        color: Color,
    ) -> Result<(), RenderError>;

    /// Fill a PDF content-stream path.
    fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        transform: Matrix,
        color: Color,
    ) -> Result<(), RenderError>;

    /// Stroke a PDF content-stream path.
    fn stroke_path(
        &mut self,
        path: &Path,
        style: StrokeStyle,
        transform: Matrix,
        color: Color,
    ) -> Result<(), RenderError>;

    /// Intersect the device clip with a PDF path.
    fn clip_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        transform: Matrix,
    ) -> Result<(), RenderError>;

    /// Pop one clip installed by `clip_path`.
    fn pop_clip(&mut self) -> Result<(), RenderError>;

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

    /// Finish the page: SVG markup, or the raster layout of [`Backend::set_raster`].
    fn finish_page(&mut self) -> Result<Vec<u8>, RenderError>;

    /// Draw later pages to pixels at `scale` pixels per bp, or to SVG when `None`.
    fn set_raster(&mut self, _scale: Option<f32>) -> Result<(), RenderError> {
        Err(RenderError::Backend("this backend cannot rasterize".into()))
    }
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
        _width_bp: f32,
        _height_bp: f32,
        _transform: Matrix,
        _color: Color,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn fill_path(
        &mut self,
        _path: &Path,
        _even_odd: bool,
        _transform: Matrix,
        _color: Color,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn stroke_path(
        &mut self,
        _path: &Path,
        _style: StrokeStyle,
        _transform: Matrix,
        _color: Color,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn clip_path(
        &mut self,
        _path: &Path,
        _even_odd: bool,
        _transform: Matrix,
    ) -> Result<(), RenderError> {
        Ok(())
    }

    fn pop_clip(&mut self) -> Result<(), RenderError> {
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

    fn finish_page(&mut self) -> Result<Vec<u8>, RenderError> {
        Ok(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_vec())
    }
}
