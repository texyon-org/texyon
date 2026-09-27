//! XDV event handlers that build the page plans.

use std::collections::BTreeMap;
use std::sync::Arc;

use tectonic_xdv::{FileType, XdvEvents};

use crate::color::Color;
use crate::encoding::type1_builtin;
use crate::pagesize::{is_page_size_special, DEFAULT_BP};
use crate::resource::{ResourceKind, ResourceProvider, ResourceRequest};
use crate::tfm::Tfm;
use crate::RenderError;

use super::super::plan::{ClassicPosition, GlyphPosition};
use super::super::{DrawOperation, ImageKind, PagePlan, SP_TO_BP};
use super::annotation::{ascent, descent};
use super::specials::parse_image_special;
use super::{ClassicFont, NativeFont, Walker};

impl<P: ResourceProvider> XdvEvents for Walker<'_, P> {
    type Error = RenderError;

    fn handle_header(&mut self, filetype: FileType, _comment: &[u8]) -> Result<(), Self::Error> {
        if filetype != FileType::Xdv {
            return Err(RenderError::Xdv(
                "SPX is not a paginated preview document".into(),
            ));
        }
        self.filetype = filetype;
        Ok(())
    }

    fn handle_begin_page(
        &mut self,
        _counters: &[i32],
        _previous_bop: i32,
    ) -> Result<(), Self::Error> {
        self.finish_page();
        self.capture_resume();
        self.page_size.begin_page();
        self.page = Some(PagePlan {
            width_bp: DEFAULT_BP[0],
            height_bp: DEFAULT_BP[1],
            operations: Vec::new(),
            links: Vec::new(),
            destinations: Vec::new(),
            unsupported: BTreeMap::new(),
            complete: true,
        });
        self.pdf_graphics.reset_page();
        Ok(())
    }

    fn handle_special(&mut self, x: i32, y: i32, contents: &[u8]) -> Result<(), Self::Error> {
        if is_page_size_special(contents) {
            // Matches the previous dedicated size pass, which only tracked
            // media-size specials appearing inside a page.
            if self.page.is_some() {
                self.page_size.apply(contents);
            }
            self.ledger.record_supported("page-size");
            return Ok(());
        }
        if self.apply_color_special(contents)? {
            return Ok(());
        }
        if self.apply_transform_special(x, y, contents)? {
            return Ok(());
        }
        if contents == b"pdf:code" || contents.starts_with(b"pdf:code ") {
            let payload = &contents[b"pdf:code".len()..];
            let run = self.pdf_graphics.run(
                payload.strip_prefix(b" ").unwrap_or(payload),
                x,
                y,
                SP_TO_BP,
            );
            self.color = self.pdf_graphics.fill;
            self.append_pdf_operations(run.operations)?;
            for name in run.supported {
                self.ledger.record_supported(name);
            }
            for name in run.unsupported {
                self.unsupported(&name)?;
            }
            return Ok(());
        }
        let image_origin = self.pdf_graphics.current_transform(x, y, SP_TO_BP);
        if let Some(image) = parse_image_special(contents, image_origin) {
            let request = ResourceRequest {
                kind: match image.kind {
                    ImageKind::Raster => ResourceKind::Image,
                    ImageKind::Pdf => ResourceKind::Pdf,
                },
                name: image.path,
            };
            let Some(resource) = self.provider.resolve(&request)? else {
                return self.unsupported("missing-image-resource");
            };
            let identity = resource.identity;
            self.resources.insert(identity, resource);
            self.page_mut()?.operations.push(DrawOperation::Image {
                identity,
                kind: image.kind,
                transform: image.transform,
                page: image.page,
                width: image.width,
                height: image.height,
            });
            self.ledger.record_supported(match image.kind {
                ImageKind::Raster => "raster-image",
                ImageKind::Pdf => "embedded-pdf",
            });
            return Ok(());
        }
        if self.apply_link_special(x, y, contents)? {
            return Ok(());
        }
        if self.apply_nonvisual_special(contents) {
            return Ok(());
        }
        let name = std::str::from_utf8(contents)
            .ok()
            .and_then(|text| text.split([' ', ':', '=']).next())
            .filter(|name| !name.is_empty())
            .unwrap_or("non-utf8-special");
        self.unsupported(name)
    }

    fn handle_char_run(&mut self, _font_num: i32, chars: &[i32]) -> Result<(), Self::Error> {
        for _ in chars {
            self.unsupported("classic-font-char")?;
        }
        Ok(())
    }

    fn handle_char_run_ext(
        &mut self,
        font_num: i32,
        chars: &[i32],
        x: i32,
        y: i32,
    ) -> Result<i32, Self::Error> {
        let Some(font) = self.classic_fonts.get(&font_num) else {
            self.unsupported("undefined-classic-font")?;
            return Ok(0);
        };
        let Some(tfm) = font.tfm.as_ref() else {
            self.unsupported("missing-tfm")?;
            return Ok(0);
        };
        let mut pen = x;
        let mut positions = Vec::with_capacity(chars.len());
        for &character in chars {
            positions.push(ClassicPosition {
                character,
                transform: self.pdf_graphics.current_transform(pen, y, SP_TO_BP),
            });
            pen = pen.wrapping_add(tfm.advance(character, font.scale));
        }
        let advance = pen.wrapping_sub(x);
        let scale = font.scale;
        let identities = (
            font.tfm_identity,
            font.program_identity,
            font.encoding_identity,
        );
        let (Some(tfm_identity), Some(program_identity), Some(encoding_identity)) = identities
        else {
            self.unsupported("incomplete-classic-font-resources")?;
            return Ok(advance);
        };
        let color = self.color;
        self.page_mut()?
            .operations
            .push(DrawOperation::ClassicChars {
                tfm_identity,
                program_identity,
                encoding_identity,
                scale,
                chars: positions,
                color,
            });
        self.ledger.record_supported("classic-char-run");
        Ok(advance)
    }

    fn handle_define_font(
        &mut self,
        font_num: i32,
        _checksum: u32,
        scale_factor: u32,
        _design_size: u32,
        area: &str,
        name: &str,
    ) -> Result<(), Self::Error> {
        let resource_name = format!("{area}{name}");
        let font_map = self.font_map()?;
        let mapped = font_map
            .get(&resource_name)
            .or_else(|| font_map.get(name))
            .cloned();
        let program_name = mapped
            .as_ref()
            .and_then(|entry| entry.program.as_ref())
            .cloned()
            .unwrap_or_else(|| resource_name.clone());
        let encoding_name = mapped
            .as_ref()
            .and_then(|entry| entry.encoding.as_ref())
            .cloned()
            .unwrap_or_else(|| resource_name.clone());
        let tfm_resource = self.provider.resolve(&ResourceRequest {
            kind: ResourceKind::Tfm,
            name: resource_name.clone(),
        })?;
        let program = self.provider.resolve(&ResourceRequest {
            kind: ResourceKind::FontProgram,
            name: program_name,
        })?;
        let mut encoding = self.provider.resolve(&ResourceRequest {
            kind: ResourceKind::Encoding,
            name: encoding_name,
        })?;
        if encoding.is_none() {
            encoding = program.as_ref().and_then(|program| {
                self.artifacts
                    .builtin_encoding
                    .entry(program.identity)
                    .or_insert_with(|| type1_builtin(&program.bytes))
                    .clone()
            });
        }
        let tfm = tfm_resource.as_ref().and_then(|resource| {
            self.artifacts
                .tfm
                .entry(resource.identity)
                .or_insert_with(|| Tfm::parse(&resource.bytes).map(Arc::new))
                .clone()
        });
        let tfm_identity = tfm_resource.as_ref().map(|resource| resource.identity);
        let program_identity = program.as_ref().map(|resource| resource.identity);
        let encoding_identity = encoding.as_ref().map(|resource| resource.identity);
        for resource in [tfm_resource, program, encoding].into_iter().flatten() {
            self.resources.insert(resource.identity, resource);
        }
        if tfm.is_some() && program_identity.is_some() && encoding_identity.is_some() {
            self.ledger.record_supported("classic-font-resource");
        } else {
            self.ledger.record_unsupported("classic-font-resource");
        }
        self.classic_fonts.insert(
            font_num,
            ClassicFont {
                scale: scale_factor,
                tfm,
                tfm_identity,
                program_identity,
                encoding_identity,
            },
        );
        Ok(())
    }

    fn handle_glyph_run(
        &mut self,
        font_num: i32,
        glyphs: &[u16],
        x: &[i32],
        y: &[i32],
    ) -> Result<(), Self::Error> {
        let Some(font) = self.fonts.get(&font_num) else {
            return self.unsupported("undefined-native-font");
        };
        let Some(identity) = font.resource else {
            return self.unsupported("missing-native-font");
        };
        let face_index = font.face_index;
        let size = font.size;
        let font_color = font.color;
        let extend = font.extend;
        let slant = font.slant;
        let embolden = font.embolden;
        let vertical = font.vertical;
        debug_assert_eq!(glyphs.len(), x.len());
        debug_assert_eq!(glyphs.len(), y.len());
        let positions = glyphs
            .iter()
            .zip(x)
            .zip(y)
            .map(|((&glyph_id, &x), &y)| GlyphPosition {
                glyph_id,
                transform: self.pdf_graphics.current_transform(x, y, SP_TO_BP),
            })
            .collect();
        if self.open_link.is_some() {
            if let (Some(&first), Some(&last)) = (x.first(), x.last()) {
                let baseline = y.first().copied().unwrap_or(0);
                self.extend_link(first, last, baseline, ascent(size), descent(size));
            }
        }
        let color = font_color.unwrap_or(self.color);
        self.page_mut()?
            .operations
            .push(DrawOperation::NativeGlyphs {
                identity,
                face_index,
                size,
                glyphs: positions,
                color,
                extend,
                slant,
                embolden,
                vertical,
            });
        self.ledger.record_supported("native-glyph-run");
        Ok(())
    }

    fn handle_text_and_glyphs(
        &mut self,
        font_num: i32,
        _text: &str,
        _width: i32,
        glyphs: &[u16],
        x: &[i32],
        y: &[i32],
    ) -> Result<(), Self::Error> {
        self.handle_glyph_run(font_num, glyphs, x, y)
    }

    fn handle_define_native_font_ext(
        &mut self,
        name: &str,
        font_num: i32,
        size: i32,
        face_index: u32,
        vertical: bool,
        color_rgba: Option<u32>,
        extend: Option<u32>,
        slant: Option<u32>,
        embolden: Option<u32>,
    ) -> Result<(), Self::Error> {
        let color = color_rgba.map(|rgba| {
            Color::rgba(
                ((rgba >> 24) & 0xff) as f32 / 255.0,
                ((rgba >> 16) & 0xff) as f32 / 255.0,
                ((rgba >> 8) & 0xff) as f32 / 255.0,
                (rgba & 0xff) as f32 / 255.0,
            )
        });
        let request = ResourceRequest {
            kind: ResourceKind::FontProgram,
            name: name.into(),
        };
        let resource = self.provider.resolve(&request)?;
        let identity = resource.as_ref().map(|resource| resource.identity);
        if let Some(resource) = resource {
            self.resources.insert(resource.identity, resource);
            self.ledger.record_supported("native-font-resource");
        } else {
            self.ledger.record_unsupported("native-font-resource");
        }
        self.fonts.insert(
            font_num,
            NativeFont {
                resource: identity,
                face_index,
                size,
                color,
                extend: extend.unwrap_or(65_536),
                slant: slant.unwrap_or(0),
                embolden: embolden.unwrap_or(0),
                vertical,
            },
        );
        Ok(())
    }

    fn handle_rule(&mut self, x: i32, y: i32, height: i32, width: i32) -> Result<(), Self::Error> {
        if height > 0 && width > 0 {
            // A rule states its own box, so no estimate is needed: it runs from
            // the baseline up by its height.
            self.extend_link(x, x + width, y, height, 0);
            let color = self.color;
            let transform = self.pdf_graphics.current_transform(x, y, SP_TO_BP);
            self.page_mut()?.operations.push(DrawOperation::Rule {
                height,
                width,
                transform,
                color,
            });
            self.ledger.record_supported("rule");
        }
        Ok(())
    }
}
