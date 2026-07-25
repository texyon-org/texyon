//! MuPDF SVG backend.
//!
//! Enabling this module links MuPDF and therefore subjects the resulting
//! binary to MuPDF's AGPL-3.0-or-later licensing terms.

use std::collections::BTreeMap;
use std::ffi::{c_void, CString};

use crate::{
    Backend, Color, FontId, FontStyle, Matrix, Path, PathCommand, PositionedChar, PositionedGlyph,
    RenderError, ResourceIdentity, StrokeStyle,
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
        transforms: *const Matrix,
        count: i32,
        rgba: *const f32,
        extend: f32,
        slant: f32,
        embolden: f32,
        vertical: i32,
    ) -> i32;
    fn pr_backend_rule(
        backend: *mut c_void,
        width: f32,
        height: f32,
        transform: Matrix,
        rgba: *const f32,
    ) -> i32;
    fn pr_backend_fill_path(
        backend: *mut c_void,
        kinds: *const u8,
        coordinates: *const f32,
        count: i32,
        even_odd: i32,
        transform: Matrix,
        rgba: *const f32,
    ) -> i32;
    fn pr_backend_stroke_path(
        backend: *mut c_void,
        kinds: *const u8,
        coordinates: *const f32,
        count: i32,
        style: StrokeStyle,
        transform: Matrix,
        rgba: *const f32,
    ) -> i32;
    fn pr_backend_clip_path(
        backend: *mut c_void,
        kinds: *const u8,
        coordinates: *const f32,
        count: i32,
        even_odd: i32,
        transform: Matrix,
    ) -> i32;
    fn pr_backend_pop_clip(backend: *mut c_void) -> i32;
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
    #[cfg(test)]
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
    classic_fonts: BTreeMap<(ResourceIdentity, ResourceIdentity), FontId>,
    classic_encodings: BTreeMap<FontId, Vec<u16>>,
}

/// Owned RGB/RGBA raster produced by the fidelity oracle (test-only).
#[cfg(test)]
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
            classic_fonts: BTreeMap::new(),
            classic_encodings: BTreeMap::new(),
        })
    }

    /// Rasterize one SVG document at `dpi` (fidelity-oracle tests only).
    #[cfg(test)]
    pub fn rasterize_svg(&self, svg: &[u8], dpi: f32) -> Result<Pixmap, RenderError> {
        self.rasterize(svg, c"image/svg+xml", 0, dpi)
    }

    /// Rasterize one zero-based PDF page at `dpi` (fidelity-oracle tests only).
    #[cfg(test)]
    pub fn rasterize_pdf(&self, pdf: &[u8], page: u32, dpi: f32) -> Result<Pixmap, RenderError> {
        self.rasterize(pdf, c"application/pdf", page, dpi)
    }

    #[cfg(test)]
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

    fn register_uncached_font(
        &mut self,
        bytes: &[u8],
        face_index: u32,
    ) -> Result<FontId, RenderError> {
        // SAFETY: byte slice remains live; the shim copies it and catches exceptions.
        let raw =
            unsafe { pr_backend_font(self.raw, bytes.as_ptr(), bytes.len(), face_index as i32) };
        if raw < 0 {
            return Err(RenderError::Backend("MuPDF rejected font bytes".into()));
        }
        Ok(FontId(raw as u32))
    }
}

fn flatten_path(path: &Path) -> (Vec<u8>, Vec<f32>) {
    let mut kinds = Vec::with_capacity(path.commands.len());
    let mut coordinates = Vec::new();
    for command in &path.commands {
        match command {
            PathCommand::MoveTo(x, y) => {
                kinds.push(0);
                coordinates.extend([x, y]);
            }
            PathCommand::LineTo(x, y) => {
                kinds.push(1);
                coordinates.extend([x, y]);
            }
            PathCommand::CurveTo(x1, y1, x2, y2, x3, y3) => {
                kinds.push(2);
                coordinates.extend([x1, y1, x2, y2, x3, y3]);
            }
            PathCommand::Rect(x0, y0, x1, y1) => {
                kinds.push(3);
                coordinates.extend([x0, y0, x1, y1]);
            }
            PathCommand::Close => kinds.push(4),
        }
    }
    (kinds, coordinates)
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
        let font = self.register_uncached_font(bytes, face_index)?;
        self.fonts.insert(key, font);
        Ok(font)
    }

    fn register_classic_font(
        &mut self,
        identity: ResourceIdentity,
        bytes: &[u8],
        encoding_identity: ResourceIdentity,
        encoding_bytes: &[u8],
    ) -> Result<FontId, RenderError> {
        let key = (identity, encoding_identity);
        if let Some(font) = self.classic_fonts.get(&key) {
            return Ok(*font);
        }
        // MuPDF stores a font's glyph mapping on its font object. The same
        // Type1 program can therefore only carry one TeX encoding per MuPDF
        // font instance.
        let font = self.register_uncached_font(bytes, 0)?;
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
        self.classic_fonts.insert(key, font);
        Ok(font)
    }

    fn fill_rule(
        &mut self,
        width_bp: f32,
        height_bp: f32,
        transform: Matrix,
        color: Color,
    ) -> Result<(), RenderError> {
        let rgba = [color.red, color.green, color.blue, color.alpha];
        // SAFETY: fixed-size transform and color values remain live for the guarded C call.
        if unsafe { pr_backend_rule(self.raw, width_bp, height_bp, transform, rgba.as_ptr()) } == 0
        {
            return Err(RenderError::Backend("MuPDF rule drawing failed".into()));
        }
        Ok(())
    }

    fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        transform: Matrix,
        color: Color,
    ) -> Result<(), RenderError> {
        let (kinds, coordinates) = flatten_path(path);
        let rgba = [color.red, color.green, color.blue, color.alpha];
        // SAFETY: the flattened slices and color remain live during the guarded C call.
        let ok = unsafe {
            pr_backend_fill_path(
                self.raw,
                kinds.as_ptr(),
                coordinates.as_ptr(),
                kinds.len() as i32,
                i32::from(even_odd),
                transform,
                rgba.as_ptr(),
            )
        };
        if ok == 0 {
            return Err(RenderError::Backend("MuPDF path fill failed".into()));
        }
        Ok(())
    }

    fn stroke_path(
        &mut self,
        path: &Path,
        style: StrokeStyle,
        transform: Matrix,
        color: Color,
    ) -> Result<(), RenderError> {
        let (kinds, coordinates) = flatten_path(path);
        let rgba = [color.red, color.green, color.blue, color.alpha];
        // SAFETY: the flattened slices and color remain live during the guarded C call.
        let ok = unsafe {
            pr_backend_stroke_path(
                self.raw,
                kinds.as_ptr(),
                coordinates.as_ptr(),
                kinds.len() as i32,
                style,
                transform,
                rgba.as_ptr(),
            )
        };
        if ok == 0 {
            return Err(RenderError::Backend("MuPDF path stroke failed".into()));
        }
        Ok(())
    }

    fn clip_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        transform: Matrix,
    ) -> Result<(), RenderError> {
        let (kinds, coordinates) = flatten_path(path);
        // SAFETY: the flattened slices remain live during the guarded C call.
        let ok = unsafe {
            pr_backend_clip_path(
                self.raw,
                kinds.as_ptr(),
                coordinates.as_ptr(),
                kinds.len() as i32,
                i32::from(even_odd),
                transform,
            )
        };
        if ok == 0 {
            return Err(RenderError::Backend("MuPDF path clipping failed".into()));
        }
        Ok(())
    }

    fn pop_clip(&mut self) -> Result<(), RenderError> {
        // SAFETY: shim call on a live context; exceptions are caught in C.
        if unsafe { pr_backend_pop_clip(self.raw) } == 0 {
            return Err(RenderError::Backend("MuPDF clip pop failed".into()));
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
        let gids: Vec<_> = glyphs.iter().map(|glyph| glyph.glyph_id).collect();
        let transforms: Vec<_> = glyphs.iter().map(|glyph| glyph.transform).collect();
        let rgba = [color.red, color.green, color.blue, color.alpha];
        // SAFETY: all slices remain live and the C shim catches MuPDF exceptions.
        let ok = unsafe {
            pr_backend_glyphs(
                self.raw,
                font.0 as i32,
                size_bp,
                gids.as_ptr(),
                transforms.as_ptr(),
                gids.len() as i32,
                rgba.as_ptr(),
                style.extend,
                style.slant,
                style.embolden,
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
                transform: character.transform,
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

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    fn positioned(x: f32, y: f32) -> Matrix {
        Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: -1.0,
            e: x,
            f: y,
        }
    }

    #[test]
    fn emits_a_path_svg_for_a_rule() {
        let mut backend = MupdfBackend::new().unwrap();
        backend.begin_page(100.0, 120.0).unwrap();
        backend
            .fill_rule(40.0, 20.0, positioned(10.0, 30.0), Color::BLACK)
            .unwrap();
        let svg = backend.finish_page().unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("path"));
    }

    #[test]
    fn emits_balanced_vector_clip_groups() {
        let identity = Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        };
        let rectangle = |x0, y0, x1, y1| Path {
            commands: vec![PathCommand::Rect(x0, y0, x1, y1)],
        };
        let mut backend = MupdfBackend::new().unwrap();
        backend.begin_page(60.0, 60.0).unwrap();
        backend
            .clip_path(&rectangle(10.0, 10.0, 30.0, 30.0), false, identity)
            .unwrap();
        backend
            .fill_path(
                &rectangle(0.0, 0.0, 50.0, 50.0),
                false,
                identity,
                Color::rgb(1.0, 0.0, 0.0),
            )
            .unwrap();
        backend.pop_clip().unwrap();
        backend
            .stroke_path(
                &Path {
                    commands: vec![
                        PathCommand::MoveTo(5.0, 50.0),
                        PathCommand::LineTo(55.0, 50.0),
                    ],
                },
                StrokeStyle {
                    width: 2.0,
                    join: 1,
                    cap: 1,
                    miter_limit: 10.0,
                    dash: [3.0, 2.0, 0.0, 0.0],
                    dash_len: 2,
                    dash_phase: 0.0,
                },
                identity,
                Color::BLACK,
            )
            .unwrap();
        let svg = backend.finish_page().unwrap();
        assert!(svg.contains("<clipPath"));
        assert!(svg.contains("<g clip-path="));
        assert!(svg.contains("</g>"));
        assert!(svg.contains("stroke-dasharray="));
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
                    transform: positioned(10.0, 30.0),
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
    fn emits_synthetic_bold_glyph_outlines() {
        let bytes = std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf").unwrap();
        let resource = crate::Resource::from_bytes(bytes);
        let mut backend = MupdfBackend::new().unwrap();
        let font = backend
            .register_font(resource.identity, &resource.bytes, 0)
            .unwrap();
        let render = |backend: &mut MupdfBackend, embolden| {
            backend.begin_page(100.0, 120.0).unwrap();
            backend
                .fill_glyphs(
                    font,
                    20.0,
                    &[PositionedGlyph {
                        glyph_id: 36,
                        transform: positioned(10.0, 30.0),
                    }],
                    Color::BLACK,
                    FontStyle {
                        extend: 1.0,
                        slant: 0.0,
                        embolden,
                        vertical: false,
                    },
                )
                .unwrap();
            backend.finish_page().unwrap()
        };
        let regular_svg = render(&mut backend, 0.0);
        let bold_svg = render(&mut backend, 0.1);
        assert!(bold_svg.contains("path"));
        let regular = backend
            .rasterize_svg(regular_svg.as_bytes(), 144.0)
            .unwrap();
        let bold = backend.rasterize_svg(bold_svg.as_bytes(), 144.0).unwrap();
        let ink = |pixmap: &Pixmap| {
            pixmap
                .samples
                .chunks_exact(pixmap.components)
                .filter(|pixel| pixel[..3] != [255, 255, 255])
                .count()
        };
        assert!(ink(&bold) > ink(&regular));
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
                    transform: positioned(10.0, 30.0),
                }],
                Color::BLACK,
            )
            .unwrap();
        assert!(backend.finish_page().unwrap().contains("path"));
    }

    #[test]
    fn same_classic_program_can_use_distinct_encodings() {
        let bytes = std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf").unwrap();
        let program = crate::Resource::from_bytes(bytes);
        let encoding_a = crate::Resource::from_bytes(b"[ /A ]".to_vec());
        let encoding_b = crate::Resource::from_bytes(b"[ /bullet ]".to_vec());
        let mut backend = MupdfBackend::new().unwrap();

        let font_a = backend
            .register_classic_font(
                program.identity,
                &program.bytes,
                encoding_a.identity,
                &encoding_a.bytes,
            )
            .unwrap();
        let font_b = backend
            .register_classic_font(
                program.identity,
                &program.bytes,
                encoding_b.identity,
                &encoding_b.bytes,
            )
            .unwrap();

        assert_ne!(font_a, font_b);
    }
}
