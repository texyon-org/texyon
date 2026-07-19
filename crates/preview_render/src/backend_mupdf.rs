//! Linux MuPDF SVG backend.
//!
//! Enabling this module links MuPDF 1.19 and therefore subjects the resulting
//! binary to MuPDF's AGPL-3.0-or-later licensing terms.

use std::ffi::c_void;

use crate::{
    Backend, Color, FontId, FontStyle, Matrix, PositionedChar, PositionedGlyph, RenderError,
    ResourceIdentity,
};

extern "C" {
    fn pr_backend_new() -> *mut c_void;
    fn pr_backend_drop(backend: *mut c_void);
    fn pr_backend_begin(backend: *mut c_void, width: f32, height: f32) -> i32;
    fn pr_backend_rule(
        backend: *mut c_void,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        rgba: *const f32,
    ) -> i32;
    fn pr_backend_finish(backend: *mut c_void, length: *mut usize) -> *mut u8;
    fn pr_backend_free(data: *mut u8);
}

/// MuPDF 1.19 backend producing self-contained path SVG pages.
pub struct MupdfBackend {
    raw: *mut c_void,
}

impl MupdfBackend {
    /// Create a backend with its own MuPDF context.
    pub fn new() -> Result<Self, RenderError> {
        // SAFETY: the shim returns a live owned context or null.
        let raw = unsafe { pr_backend_new() };
        if raw.is_null() {
            return Err(RenderError::Backend(
                "could not create MuPDF context".into(),
            ));
        }
        Ok(Self { raw })
    }
}

impl Drop for MupdfBackend {
    fn drop(&mut self) {
        // SAFETY: `raw` is uniquely owned for the lifetime of this backend.
        unsafe { pr_backend_drop(self.raw) };
    }
}

impl Backend for MupdfBackend {
    fn begin_page(&mut self, width_bp: f32, height_bp: f32) -> Result<(), RenderError> {
        // SAFETY: shim call on a live context; exceptions are caught in C.
        if unsafe { pr_backend_begin(self.raw, width_bp, height_bp) } == 0 {
            return Err(RenderError::Backend(
                "MuPDF could not start SVG page".into(),
            ));
        }
        Ok(())
    }

    fn register_font(
        &mut self,
        _identity: ResourceIdentity,
        _bytes: &[u8],
        _face_index: u32,
    ) -> Result<FontId, RenderError> {
        Err(RenderError::Backend(
            "MuPDF font registration is not implemented".into(),
        ))
    }

    fn register_classic_font(
        &mut self,
        _identity: ResourceIdentity,
        _bytes: &[u8],
        _encoding_identity: ResourceIdentity,
        _encoding_bytes: &[u8],
    ) -> Result<FontId, RenderError> {
        Err(RenderError::Backend(
            "MuPDF classic fonts are not implemented".into(),
        ))
    }

    fn fill_rule(
        &mut self,
        x_bp: f32,
        y_bp: f32,
        width_bp: f32,
        height_bp: f32,
        color: Color,
    ) -> Result<(), RenderError> {
        let rgba = [color.red, color.green, color.blue, color.alpha];
        // SAFETY: fixed-size color array remains live for the guarded C call.
        if unsafe { pr_backend_rule(self.raw, x_bp, y_bp, width_bp, height_bp, rgba.as_ptr()) } == 0
        {
            return Err(RenderError::Backend("MuPDF rule drawing failed".into()));
        }
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
        Err(RenderError::Backend(
            "MuPDF glyph drawing is not implemented".into(),
        ))
    }

    fn fill_chars(
        &mut self,
        _font: FontId,
        _size_bp: f32,
        _chars: &[PositionedChar],
        _color: Color,
    ) -> Result<(), RenderError> {
        Err(RenderError::Backend(
            "MuPDF classic text is not implemented".into(),
        ))
    }

    fn draw_image(
        &mut self,
        _bytes: &[u8],
        _transform: Matrix,
        _width_bp: Option<f32>,
        _height_bp: Option<f32>,
    ) -> Result<(), RenderError> {
        Err(RenderError::Backend(
            "MuPDF raster images are not implemented".into(),
        ))
    }

    fn draw_pdf(
        &mut self,
        _bytes: &[u8],
        _page: u32,
        _transform: Matrix,
    ) -> Result<(), RenderError> {
        Err(RenderError::Backend(
            "MuPDF embedded PDFs are not implemented".into(),
        ))
    }

    fn finish_page(&mut self) -> Result<String, RenderError> {
        let mut length = 0;
        // SAFETY: shim returns a malloc-owned buffer copied before it is freed.
        let data = unsafe { pr_backend_finish(self.raw, &mut length) };
        if data.is_null() {
            return Err(RenderError::Backend("MuPDF SVG finalization failed".into()));
        }
        // SAFETY: shim returned `length` initialized bytes.
        let bytes = unsafe { std::slice::from_raw_parts(data, length) };
        let svg = String::from_utf8(bytes.to_vec())
            .map_err(|_| RenderError::Backend("MuPDF emitted non-UTF-8 SVG".into()));
        // SAFETY: buffer is malloc-owned and freed exactly once.
        unsafe { pr_backend_free(data) };
        svg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_a_path_svg_for_a_rule() {
        let mut backend = MupdfBackend::new().unwrap();
        backend.begin_page(100.0, 120.0).unwrap();
        backend
            .fill_rule(10.0, 30.0, 40.0, 20.0, Color::BLACK)
            .unwrap();
        let svg = backend.finish_page().unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("path"));
    }
}
