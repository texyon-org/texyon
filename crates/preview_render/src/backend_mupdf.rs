//! Linux MuPDF SVG backend.
//!
//! Enabling this module links MuPDF 1.19 and therefore subjects the resulting
//! binary to MuPDF's AGPL-3.0-or-later licensing terms.

use std::collections::BTreeMap;
use std::ffi::{c_void, CString};

use crate::{
    Backend, Color, FontId, FontStyle, Matrix, PositionedChar, PositionedGlyph, RenderError,
    ResourceIdentity,
};

extern "C" {
    fn pr_backend_new() -> *mut c_void;
    fn pr_backend_drop(backend: *mut c_void);
    fn pr_backend_begin(backend: *mut c_void, width: f32, height: f32) -> i32;
    fn pr_backend_font(backend: *mut c_void, data: *const u8, length: usize, face: i32) -> i32;
    fn pr_backend_gid_by_name(backend: *mut c_void, font: i32, name: *const i8) -> i32;
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
    fn pr_backend_image(
        backend: *mut c_void,
        data: *const u8,
        length: usize,
        transform: Matrix,
        width: f32,
        height: f32,
    ) -> i32;
    fn pr_backend_pdf(
        backend: *mut c_void,
        data: *const u8,
        length: usize,
        page: i32,
        transform: Matrix,
    ) -> i32;
    fn pr_backend_finish(backend: *mut c_void, length: *mut usize) -> *mut u8;
    fn pr_backend_raster(
        backend: *mut c_void,
        data: *const u8,
        length: usize,
        mime: *const i8,
        page: i32,
        dpi: f32,
        width: *mut i32,
        height: *mut i32,
        components: *mut i32,
        sample_length: *mut usize,
    ) -> *mut u8;
    fn pr_backend_free(data: *mut u8);
}

/// MuPDF 1.19 backend producing self-contained path SVG pages.
pub struct MupdfBackend {
    raw: *mut c_void,
    fonts: BTreeMap<(ResourceIdentity, u32), FontId>,
    classic_encodings: BTreeMap<FontId, Vec<u16>>,
}

/// Owned RGB/RGBA raster produced by the fidelity oracle.
pub struct Pixmap {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Components per pixel.
    pub components: usize,
    /// Tightly packed row-major samples.
    pub samples: Vec<u8>,
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
            classic_encodings: BTreeMap::new(),
        })
    }

    /// Rasterize one SVG document at `dpi`.
    pub fn rasterize_svg(&self, svg: &[u8], dpi: f32) -> Result<Pixmap, RenderError> {
        self.rasterize(svg, c"image/svg+xml", 0, dpi)
    }

    /// Rasterize one zero-based PDF page at `dpi`.
    pub fn rasterize_pdf(&self, pdf: &[u8], page: u32, dpi: f32) -> Result<Pixmap, RenderError> {
        self.rasterize(pdf, c"application/pdf", page, dpi)
    }

    fn rasterize(
        &self,
        bytes: &[u8],
        mime: &std::ffi::CStr,
        page: u32,
        dpi: f32,
    ) -> Result<Pixmap, RenderError> {
        let (mut width, mut height, mut components, mut length) = (0, 0, 0, 0);
        // SAFETY: all inputs remain live; returned allocation is copied and freed.
        let data = unsafe {
            pr_backend_raster(
                self.raw,
                bytes.as_ptr(),
                bytes.len(),
                mime.as_ptr(),
                page as i32,
                dpi,
                &mut width,
                &mut height,
                &mut components,
                &mut length,
            )
        };
        if data.is_null() {
            return Err(RenderError::Backend("MuPDF rasterization failed".into()));
        }
        // SAFETY: shim returned `length` initialized bytes.
        let samples = unsafe { std::slice::from_raw_parts(data, length) }.to_vec();
        // SAFETY: buffer is malloc-owned and freed exactly once.
        unsafe { pr_backend_free(data) };
        Ok(Pixmap {
            width: width as usize,
            height: height as usize,
            components: components as usize,
            samples,
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
        identity: ResourceIdentity,
        bytes: &[u8],
        _encoding_identity: ResourceIdentity,
        encoding_bytes: &[u8],
    ) -> Result<FontId, RenderError> {
        let font = self.register_font(identity, bytes, 0)?;
        if self.classic_encodings.contains_key(&font) {
            return Ok(font);
        }
        let names = parse_encoding(encoding_bytes)?;
        let mut glyphs = Vec::with_capacity(names.len());
        for name in names {
            let name = CString::new(name)
                .map_err(|_| RenderError::Backend("NUL in font encoding".into()))?;
            // SAFETY: CString remains live and the shim catches MuPDF exceptions.
            let glyph = unsafe { pr_backend_gid_by_name(self.raw, font.0 as i32, name.as_ptr()) };
            glyphs.push(glyph.max(0) as u16);
        }
        self.classic_encodings.insert(font, glyphs);
        Ok(font)
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
        font: FontId,
        size_bp: f32,
        chars: &[PositionedChar],
        color: Color,
    ) -> Result<(), RenderError> {
        let encoding = self
            .classic_encodings
            .get(&font)
            .ok_or_else(|| RenderError::Backend("classic encoding was not registered".into()))?;
        let glyphs: Vec<_> = chars
            .iter()
            .map(|character| PositionedGlyph {
                glyph_id: encoding
                    .get(character.character as usize)
                    .copied()
                    .unwrap_or(0),
                x_bp: character.x_bp,
                y_bp: character.y_bp,
            })
            .collect();
        self.fill_glyphs(
            font,
            size_bp,
            &glyphs,
            color,
            FontStyle {
                extend: 1.0,
                slant: 0.0,
                embolden: 0.0,
                vertical: false,
            },
        )
    }

    fn draw_image(
        &mut self,
        bytes: &[u8],
        transform: Matrix,
        width_bp: Option<f32>,
        height_bp: Option<f32>,
    ) -> Result<(), RenderError> {
        // SAFETY: bytes remain live and the C shim catches MuPDF exceptions.
        let ok = unsafe {
            pr_backend_image(
                self.raw,
                bytes.as_ptr(),
                bytes.len(),
                transform,
                width_bp.unwrap_or(f32::NAN),
                height_bp.unwrap_or(f32::NAN),
            )
        };
        if ok == 0 {
            return Err(RenderError::Backend("MuPDF image drawing failed".into()));
        }
        Ok(())
    }

    fn draw_pdf(&mut self, bytes: &[u8], page: u32, transform: Matrix) -> Result<(), RenderError> {
        // SAFETY: bytes remain live and the C shim catches MuPDF exceptions.
        let ok = unsafe {
            pr_backend_pdf(
                self.raw,
                bytes.as_ptr(),
                bytes.len(),
                page as i32,
                transform,
            )
        };
        if ok == 0 {
            return Err(RenderError::Backend(
                "MuPDF embedded PDF drawing failed".into(),
            ));
        }
        Ok(())
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

fn parse_encoding(bytes: &[u8]) -> Result<Vec<String>, RenderError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| RenderError::Backend("font encoding is not UTF-8/ASCII".into()))?;
    let start = text
        .find('[')
        .ok_or_else(|| RenderError::Backend("font encoding has no vector".into()))?;
    let end = text[start + 1..]
        .find(']')
        .map(|offset| start + 1 + offset)
        .ok_or_else(|| RenderError::Backend("font encoding vector is unterminated".into()))?;
    let mut names = Vec::new();
    for token in text[start + 1..end].split_whitespace() {
        if let Some(name) = token.strip_prefix('/') {
            names.push(name.to_owned());
        }
    }
    if names.is_empty() || names.len() > 256 {
        return Err(RenderError::Backend(
            "font encoding vector has invalid length".into(),
        ));
    }
    names.resize(256, ".notdef".into());
    Ok(names)
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

    #[test]
    fn embeds_raster_and_pdf_resources() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let png = std::fs::read(root.join("tests/assets/redbox.png")).unwrap();
        let pdf = std::fs::read(root.join("tests/tex-outputs/the_letter_a.pdf")).unwrap();
        let identity = Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 10.0,
            f: 20.0,
        };
        let mut backend = MupdfBackend::new().unwrap();
        backend.begin_page(200.0, 240.0).unwrap();
        backend
            .draw_image(&png, identity, Some(40.0), Some(30.0))
            .unwrap();
        backend.draw_pdf(&pdf, 0, identity).unwrap();
        let svg = backend.finish_page().unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("image") || svg.contains("path"));
    }

    #[test]
    fn maps_classic_encoding_names_to_outlines() {
        let bytes = std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf").unwrap();
        let resource = crate::Resource::from_bytes(bytes);
        let mut names = vec!["/.notdef"; 65];
        names.push("/A");
        let encoding = crate::Resource::from_bytes(format!("[{}]", names.join(" ")).into_bytes());
        let mut backend = MupdfBackend::new().unwrap();
        backend.begin_page(100.0, 120.0).unwrap();
        let font = backend
            .register_classic_font(
                resource.identity,
                &resource.bytes,
                encoding.identity,
                &encoding.bytes,
            )
            .unwrap();
        backend
            .fill_chars(
                font,
                20.0,
                &[PositionedChar {
                    character: 65,
                    x_bp: 10.0,
                    y_bp: 30.0,
                }],
                Color::BLACK,
            )
            .unwrap();
        assert!(backend.finish_page().unwrap().contains("path"));
    }
}
