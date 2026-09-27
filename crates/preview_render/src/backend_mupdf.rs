//! MuPDF SVG backend.
//!
//! Enabling this module links MuPDF and therefore subjects the resulting
//! binary to MuPDF's AGPL-3.0-or-later licensing terms.

mod ffi;

use std::collections::BTreeMap;
use std::ffi::CString;

use crate::{
    Backend, Color, FontId, FontStyle, Matrix, Path, PositionedChar, PositionedGlyph, RenderError,
    ResourceIdentity, StrokeStyle,
};
use ffi::{Shim, ShimFailed};

const PLAIN: FontStyle = FontStyle {
    extend: 1.0,
    slant: 0.0,
    embolden: 0.0,
    vertical: false,
};

/// MuPDF 1.19 backend producing self-contained path SVG pages.
pub struct MupdfBackend {
    shim: Shim,
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

fn failed(message: &'static str) -> impl FnOnce(ShimFailed) -> RenderError {
    move |_| RenderError::Backend(message.into())
}

impl MupdfBackend {
    /// Create a backend with its own MuPDF context.
    pub fn new() -> Result<Self, RenderError> {
        let shim = Shim::new()
            .ok_or_else(|| RenderError::Backend("could not create MuPDF context".into()))?;
        Ok(Self {
            shim,
            fonts: BTreeMap::new(),
            classic_fonts: BTreeMap::new(),
            classic_encodings: BTreeMap::new(),
        })
    }

    /// Rasterize one SVG document at `dpi` (fidelity-oracle tests only).
    #[cfg(test)]
    pub fn rasterize_svg(&self, svg: &[u8], dpi: f32) -> Result<Pixmap, RenderError> {
        self.shim
            .rasterize(svg, c"image/svg+xml", 0, dpi)
            .map_err(failed("MuPDF rasterization failed"))
    }

    /// Rasterize one zero-based PDF page at `dpi` (fidelity-oracle tests only).
    #[cfg(test)]
    pub fn rasterize_pdf(&self, pdf: &[u8], page: u32, dpi: f32) -> Result<Pixmap, RenderError> {
        self.shim
            .rasterize(pdf, c"application/pdf", page, dpi)
            .map_err(failed("MuPDF rasterization failed"))
    }

    fn register_uncached_font(
        &mut self,
        bytes: &[u8],
        face_index: u32,
    ) -> Result<FontId, RenderError> {
        self.shim
            .register_font(bytes, face_index)
            .map_err(failed("MuPDF rejected font bytes"))
    }
}

impl Backend for MupdfBackend {
    fn begin_page(&mut self, width_bp: f32, height_bp: f32) -> Result<(), RenderError> {
        self.shim
            .begin_page(width_bp, height_bp)
            .map_err(failed("MuPDF could not start SVG page"))
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
        // MuPDF keeps the glyph mapping on the font object, so each encoding needs its own.
        let font = self.register_uncached_font(bytes, 0)?;
        let names = parse_encoding(encoding_bytes)?;
        let mut glyphs = Vec::with_capacity(names.len());
        for name in names {
            let name = CString::new(name)
                .map_err(|_| RenderError::Backend("NUL in font encoding".into()))?;
            glyphs.push(self.shim.glyph_by_name(font, &name).unwrap_or(0));
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
        self.shim
            .fill_rule(width_bp, height_bp, transform, color)
            .map_err(failed("MuPDF rule drawing failed"))
    }

    fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        transform: Matrix,
        color: Color,
    ) -> Result<(), RenderError> {
        self.shim
            .fill_path(path, even_odd, transform, color)
            .map_err(failed("MuPDF path fill failed"))
    }

    fn stroke_path(
        &mut self,
        path: &Path,
        style: StrokeStyle,
        transform: Matrix,
        color: Color,
    ) -> Result<(), RenderError> {
        self.shim
            .stroke_path(path, style, transform, color)
            .map_err(failed("MuPDF path stroke failed"))
    }

    fn clip_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        transform: Matrix,
    ) -> Result<(), RenderError> {
        self.shim
            .clip_path(path, even_odd, transform)
            .map_err(failed("MuPDF path clipping failed"))
    }

    fn pop_clip(&mut self) -> Result<(), RenderError> {
        self.shim
            .pop_clip()
            .map_err(failed("MuPDF clip pop failed"))
    }

    fn fill_glyphs(
        &mut self,
        font: FontId,
        size_bp: f32,
        glyphs: &[PositionedGlyph],
        color: Color,
        style: FontStyle,
    ) -> Result<(), RenderError> {
        self.shim
            .fill_glyphs(font, size_bp, glyphs, color, style)
            .map_err(failed("MuPDF glyph drawing failed"))
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
        self.fill_glyphs(font, size_bp, &glyphs, color, PLAIN)
    }

    fn draw_image(
        &mut self,
        bytes: &[u8],
        transform: Matrix,
        width_bp: Option<f32>,
        height_bp: Option<f32>,
    ) -> Result<(), RenderError> {
        self.shim
            .draw_image(bytes, transform, width_bp, height_bp)
            .map_err(failed("MuPDF image drawing failed"))
    }

    fn draw_pdf(&mut self, bytes: &[u8], page: u32, transform: Matrix) -> Result<(), RenderError> {
        self.shim
            .draw_pdf(bytes, page, transform)
            .map_err(failed("MuPDF embedded PDF drawing failed"))
    }

    fn finish_page(&mut self) -> Result<String, RenderError> {
        let bytes = self
            .shim
            .finish_page()
            .map_err(failed("MuPDF SVG finalization failed"))?;
        String::from_utf8(bytes)
            .map_err(|_| RenderError::Backend("MuPDF emitted non-UTF-8 SVG".into()))
    }
}

fn parse_encoding(bytes: &[u8]) -> Result<Vec<String>, RenderError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| RenderError::Backend("font encoding is not UTF-8/ASCII".into()))?;
    // Comments may hold names, as in cm-super-t1.enc's "/germandbls % or /germandbls.alt".
    let text = text
        .lines()
        .map(|line| line.split('%').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let start = text
        .find('[')
        .ok_or_else(|| RenderError::Backend("font encoding has no vector".into()))?;
    let end = text[start + 1..]
        .find(']')
        .map(|offset| start + 1 + offset)
        .ok_or_else(|| RenderError::Backend("font encoding vector is unterminated".into()))?;
    let mut names: Vec<String> = text[start + 1..end]
        .split_whitespace()
        .filter_map(|token| token.strip_prefix('/'))
        .map(str::to_owned)
        .collect();
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
    use crate::PathCommand;

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
    fn encoding_comments_hide_names_and_brackets() {
        let names = parse_encoding(
            b"% header with a stray ] bracket\n\
              /T1Encoding [\n\
              % 0x00\n\
              /grave\n\
              /germandbls % or /germandbls.alt\n\
              ] def\n",
        )
        .unwrap();
        assert_eq!(names[0], "grave");
        assert_eq!(names[1], "germandbls");
        assert_eq!(names[2], ".notdef");
        assert_eq!(names.len(), 256);
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
    fn drawing_without_an_open_page_is_an_error() {
        let mut backend = MupdfBackend::new().unwrap();
        assert!(backend
            .fill_rule(1.0, 1.0, positioned(0.0, 0.0), Color::BLACK)
            .is_err());
        assert!(backend.finish_page().is_err());
        backend.begin_page(10.0, 10.0).unwrap();
        backend.finish_page().unwrap();
        assert!(backend.pop_clip().is_err());
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
                PLAIN,
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
                    FontStyle { embolden, ..PLAIN },
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
