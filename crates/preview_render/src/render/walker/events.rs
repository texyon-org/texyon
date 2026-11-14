//! XDV event handlers that build the page plans.

use std::collections::BTreeMap;
use std::sync::Arc;

use texyon_xdv::{FileType, XdvEvents};

use crate::RenderError;
use crate::color::Color;
use crate::encoding::type1_builtin;
use crate::pagesize::{DEFAULT_BP, is_page_size_special};
use crate::resource::{ResourceKind, ResourceProvider, ResourceRequest};
use crate::tfm::Tfm;

use crate::backend::{PositionedChar, PositionedGlyph};
use crate::units::FIXED_ONE;

use super::super::{DrawOperation, PagePlan};
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
        self.state.page_size.begin_page();
        self.page = Some(PagePlan {
            width_bp: DEFAULT_BP[0],
            height_bp: DEFAULT_BP[1],
            operations: Vec::new(),
            links: Vec::new(),
            destinations: Vec::new(),
            unsupported: BTreeMap::new(),
            complete: true,
        });
        self.state.pdf_graphics.reset_page();
        Ok(())
    }

    fn handle_special(&mut self, x: i32, y: i32, contents: &[u8]) -> Result<(), Self::Error> {
        if is_page_size_special(contents) {
            if self.page.is_some() {
                self.state.page_size.apply(contents);
            }
            self.state.ledger.record_supported("page-size");
            return Ok(());
        }
        let text = std::str::from_utf8(contents).ok();
        let trimmed = text.map(str::trim);
        if let Some(text) = trimmed
            && (self.apply_color_special(text)? || self.apply_transform_special(x, y, text)?)
        {
            return Ok(());
        }
        if contents == b"pdf:code" || contents.starts_with(b"pdf:code ") {
            let payload = &contents[b"pdf:code".len()..];
            let run =
                self.state
                    .pdf_graphics
                    .run(payload.strip_prefix(b" ").unwrap_or(payload), x, y);
            self.state.color = self.state.pdf_graphics.fill;
            self.append_pdf_operations(run.operations)?;
            for name in run.supported {
                self.state.ledger.record_supported(name);
            }
            for name in run.unsupported {
                self.unsupported(&name)?;
            }
            return Ok(());
        }
        if let Some(image) = trimmed.and_then(parse_image_special) {
            let transform = image.transform(self.state.pdf_graphics.current_transform(x, y));
            let request = ResourceRequest {
                kind: image.kind.resource_kind(),
                name: image.path,
            };
            let Some(resource) = self.provider.resolve(&request)? else {
                return self.unsupported("missing-image-resource");
            };
            let identity = resource.identity;
            self.state.resources.insert(identity, resource);
            self.page_mut()?.operations.push(DrawOperation::Image {
                identity,
                kind: image.kind,
                transform,
                page: image.page,
                width: image.width,
                height: image.height,
            });
            self.state.ledger.record_supported(image.kind.ledger_name());
            return Ok(());
        }
        if let Some(text) = trimmed
            && (self.apply_link_special(x, y, text)? || self.apply_nonvisual_special(text))
        {
            return Ok(());
        }
        let name = text
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
        let Some(font) = self.state.classic_fonts.get(&font_num) else {
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
            positions.push(PositionedChar {
                character: character as u32,
                transform: self.state.pdf_graphics.current_transform(pen, y),
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
        let color = self.state.color;
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
        self.state.ledger.record_supported("classic-char-run");
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
        let mapped = font_map.get(&resource_name).or_else(|| font_map.get(name));
        let program_name = mapped
            .and_then(|entry| entry.program.clone())
            .unwrap_or_else(|| resource_name.clone());
        let encoding_name = mapped
            .and_then(|entry| entry.encoding.clone())
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
            self.state.resources.insert(resource.identity, resource);
        }
        if tfm.is_some() && program_identity.is_some() && encoding_identity.is_some() {
            self.state.ledger.record_supported("classic-font-resource");
        } else {
            self.state
                .ledger
                .record_unsupported("classic-font-resource");
        }
        self.state.classic_fonts.insert(
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
        let Some(&font) = self.state.fonts.get(&font_num) else {
            return self.unsupported("undefined-native-font");
        };
        let Some(identity) = font.resource else {
            return self.unsupported("missing-native-font");
        };
        debug_assert_eq!(glyphs.len(), x.len());
        debug_assert_eq!(glyphs.len(), y.len());
        let positions = glyphs
            .iter()
            .zip(x)
            .zip(y)
            .map(|((&glyph_id, &x), &y)| PositionedGlyph {
                glyph_id,
                transform: self.state.pdf_graphics.current_transform(x, y),
            })
            .collect();
        if self.open_link.is_some()
            && let (Some(&first), Some(&last)) = (x.first(), x.last())
        {
            let baseline = y.first().copied().unwrap_or(0);
            self.extend_link(first, last, baseline, ascent(font.size), descent(font.size));
        }
        let color = font.color.unwrap_or(self.state.color);
        self.page_mut()?
            .operations
            .push(DrawOperation::NativeGlyphs {
                identity,
                face_index: font.face_index,
                size: font.size,
                glyphs: positions,
                color,
                extend: font.extend,
                slant: font.slant,
                embolden: font.embolden,
                vertical: font.vertical,
            });
        self.state.ledger.record_supported("native-glyph-run");
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
            let [red, green, blue, alpha] = rgba.to_be_bytes().map(|byte| f32::from(byte) / 255.0);
            Color::rgba(red, green, blue, alpha)
        });
        let request = ResourceRequest {
            kind: ResourceKind::FontProgram,
            name: name.into(),
        };
        let resource = self.provider.resolve(&request)?;
        let identity = resource.as_ref().map(|resource| resource.identity);
        if let Some(resource) = resource {
            self.state.resources.insert(resource.identity, resource);
            self.state.ledger.record_supported("native-font-resource");
        } else {
            self.state.ledger.record_unsupported("native-font-resource");
        }
        self.state.fonts.insert(
            font_num,
            NativeFont {
                resource: identity,
                face_index,
                size,
                color,
                extend: extend.unwrap_or(FIXED_ONE),
                slant: slant.unwrap_or(0),
                embolden: embolden.unwrap_or(0),
                vertical,
            },
        );
        Ok(())
    }

    fn handle_rule(&mut self, x: i32, y: i32, height: i32, width: i32) -> Result<(), Self::Error> {
        if height > 0 && width > 0 {
            self.extend_link(x, x.wrapping_add(width), y, height, 0);
            let color = self.state.color;
            let transform = self.state.pdf_graphics.current_transform(x, y);
            self.page_mut()?.operations.push(DrawOperation::Rule {
                height,
                width,
                transform,
                color,
            });
            self.state.ledger.record_supported("rule");
        }
        Ok(())
    }
}
