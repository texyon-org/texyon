//! Linux MuPDF SVG backend.
//!
//! Enabling this module links MuPDF 1.19 and therefore subjects the resulting
//! binary to MuPDF's AGPL-3.0-or-later licensing terms.

use std::collections::BTreeMap;
use std::ffi::c_void;

use crate::{
    Backend, Color, FontId, FontStyle, Matrix, PositionedChar, PositionedGlyph, RenderError,
    ResourceIdentity,
};

extern "C" {
    fn pr_backend_new() -> *mut c_void;
    fn pr_backend_drop(backend: *mut c_void);
    fn pr_backend_begin(backend: *mut c_void, width: f32, height: f32) -> i32;
    fn pr_backend_font(backend: *mut c_void, data: *const u8, length: usize, face: i32) -> i32;
    fn pr_backend_glyphs(
        backend: *mut c_void,
        font: i32,
        size: f32,
        gids: *const u16,
        positions: *const f32,
        count: i32,
        rgba: *const f32,
        extend: f32,
        slant: f32,
        vertical: i32,
    ) -> i32;
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
    fonts: BTreeMap<(ResourceIdentity, u32), FontId>,
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
        Ok(Self {
            raw,
            fonts: BTreeMap::new(),
        })
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
        identity: ResourceIdentity,
        bytes: &[u8],
        face_index: u32,
    ) -> Result<FontId, RenderError> {
        let key = (identity, face_index);
        if let Some(font) = self.fonts.get(&key) {
            return Ok(*font);
        }
        // SAFETY: byte slice remains live; the shim copies it and catches exceptions.
        let raw =
            unsafe { pr_backend_font(self.raw, bytes.as_ptr(), bytes.len(), face_index as i32) };
        if raw < 0 {
            return Err(RenderError::Backend("MuPDF rejected font bytes".into()));
        }
        let font = FontId(raw as u32);
        self.fonts.insert(key, font);
        Ok(font)
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
        font: FontId,
        size_bp: f32,
        glyphs: &[PositionedGlyph],
        color: Color,
        style: FontStyle,
    ) -> Result<(), RenderError> {
        if style.embolden != 0.0 {
            return Err(RenderError::Backend(
                "MuPDF synthetic emboldening is not implemented".into(),
            ));
        }
        let gids: Vec<_> = glyphs.iter().map(|glyph| glyph.glyph_id).collect();
        let positions: Vec<_> = glyphs
            .iter()
            .flat_map(|glyph| [glyph.x_bp, glyph.y_bp])
            .collect();
        let rgba = [color.red, color.green, color.blue, color.alpha];
        // SAFETY: all slices remain live and the C shim catches MuPDF exceptions.
        let ok = unsafe {
            pr_backend_glyphs(
                self.raw,
                font.0 as i32,
                size_bp,
                gids.as_ptr(),
                positions.as_ptr(),
                gids.len() as i32,
                rgba.as_ptr(),
                style.extend,
                style.slant,
                i32::from(style.vertical),
            )
        };
        if ok == 0 {
            return Err(RenderError::Backend("MuPDF glyph drawing failed".into()));
        }
        Ok(())
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

    #[test]
    fn emits_native_glyph_outlines() {
        let bytes = std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf").unwrap();
        let resource = crate::Resource::from_bytes(bytes);
        let mut backend = MupdfBackend::new().unwrap();
        backend.begin_page(100.0, 120.0).unwrap();
        let font = backend
            .register_font(resource.identity, &resource.bytes, 0)
            .unwrap();
        backend
            .fill_glyphs(
                font,
                20.0,
                &[PositionedGlyph {
                    glyph_id: 36,
                    x_bp: 10.0,
                    y_bp: 30.0,
                }],
                Color::BLACK,
                FontStyle {
                    extend: 1.0,
                    slant: 0.0,
                    embolden: 0.0,
                    vertical: false,
                },
            )
            .unwrap();
        let svg = backend.finish_page().unwrap();
        assert!(svg.contains("path"));
    }
}
