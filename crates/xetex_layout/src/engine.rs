//! Base engine for laying out fonts.

use crate::font::Font;
use crate::manager::{Engine, FontManager};
use crate::{FloatPoint, GlyphBBox, OTTag};
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr;
use texyon_bridge_graphite2 as gr;
use texyon_bridge_harfbuzz as hb;

/// A font that an engine owns or borrows.
enum FontSlot<'a> {
    Owned(Box<Font>),
    Borrowed(&'a Font),
}

impl Deref for FontSlot<'_> {
    type Target = Font;

    fn deref(&self) -> &Font {
        match self {
            FontSlot::Owned(font) => font,
            FontSlot::Borrowed(font) => font,
        }
    }
}

struct GrBreak {
    segment: gr::Segment,
    slot: gr::Slot,
    text_len: u32,
}

/// The options of a new [`LayoutEngine`].
pub struct EngineOptions<'a> {
    /// The OpenType script tag, or 0.
    pub script: OTTag,
    /// The language: a BCP 47 tag for Graphite, an OpenType tag otherwise.
    pub language: Option<&'a [u8]>,
    /// The features to apply.
    pub features: Vec<hb::Feature>,
    /// The HarfBuzz shapers to try, in order. Empty means OpenType.
    pub shapers: &'a [&'a [u8]],
    /// The text colour as RGBA.
    pub rgb_value: u32,
    /// The horizontal stretch factor.
    pub extend: f32,
    /// The slant factor.
    pub slant: f32,
    /// The emboldening amount.
    pub embolden: f32,
}

/// Base layout engine. Combines all the information needed to shape a font for TeX.
pub struct LayoutEngine<'a> {
    font: FontSlot<'a>,
    script: hb::Tag,
    language: hb::Language,
    features: Box<[hb::Feature]>,
    /// Owns the names that `shaper_list` points to.
    _shaper_names: Vec<CString>,
    /// The requested shapers as HarfBuzz takes them, null-terminated.
    shaper_list: Vec<*const libc::c_char>,
    /// The shaper that ran last.
    shaper: RefCell<Option<CString>>,
    rgb_value: u32,
    extend: f32,
    slant: f32,
    embolden: f32,
    hb_buffer: RefCell<hb::Buffer>,
    gr_breaking: RefCell<Option<GrBreak>>,
}

impl LayoutEngine<'static> {
    /// An engine that owns `font`.
    pub fn new(font: Font, options: EngineOptions<'_>) -> LayoutEngine<'static> {
        LayoutEngine::with_font(FontSlot::Owned(Box::new(font)), options)
    }
}

impl<'a> LayoutEngine<'a> {
    /// A Graphite engine over a borrowed font with default options, for
    /// resolving feature names.
    pub fn graphite_probe(font: &'a Font, script: OTTag, rgb_value: u32) -> LayoutEngine<'a> {
        let options = EngineOptions {
            script,
            language: None,
            features: Vec::new(),
            shapers: &[b"graphite2"],
            rgb_value,
            extend: 1.0,
            slant: 0.0,
            embolden: 0.0,
        };
        LayoutEngine::with_font(FontSlot::Borrowed(font), options)
    }

    fn with_font(font: FontSlot<'a>, options: EngineOptions<'_>) -> LayoutEngine<'a> {
        let req_engine = FontManager::with_font_manager(|mgr| mgr.get_req_engine());
        let language = options
            .language
            .map(|lang| CString::new(lang).unwrap_or_default());
        // For Graphite fonts treat the language as BCP 47 tag, for OpenType we
        // treat it as a OT language tag for backward compatibility with pre-0.9999
        // XeTeX.
        let language = if req_engine == Engine::Graphite {
            language
                .map(|lang| hb::Language::from_cstr(&lang))
                .unwrap_or_default()
        } else {
            language
                .map(|lang| hb::Tag::from_cstr(&lang).to_language())
                .unwrap_or_default()
        };

        let shaper_names: Vec<CString> = options
            .shapers
            .iter()
            .map(|name| CString::new(*name).unwrap_or_default())
            .collect();
        // HarfBuzz gives graphite2 shaper a priority, so that for hybrid
        // Graphite/OpenType fonts, Graphite will be used. However, pre-0.9999
        // XeTeX preferred OpenType over Graphite, so we are doing the same
        // here for sake of backward compatibility. Since "ot" shaper never
        // fails, we set the shaper list to just include it.
        let mut shaper_list: Vec<*const libc::c_char> = if shaper_names.is_empty() {
            vec![c"ot".as_ptr()]
        } else {
            shaper_names.iter().map(|name| name.as_ptr()).collect()
        };
        shaper_list.push(ptr::null());

        LayoutEngine {
            font,
            script: hb::Tag::new(options.script),
            language,
            features: options.features.into_boxed_slice(),
            _shaper_names: shaper_names,
            shaper_list,
            shaper: RefCell::new(None),
            rgb_value: options.rgb_value,
            extend: options.extend,
            slant: options.slant,
            embolden: options.embolden,
            hb_buffer: RefCell::new(hb::Buffer::new()),
            gr_breaking: RefCell::new(None),
        }
    }

    /// Get the font script used
    pub fn script(&self) -> hb::Tag {
        self.script
    }

    /// Get the extend
    pub fn extend(&self) -> f32 {
        self.extend
    }

    /// Get the slant
    pub fn slant(&self) -> f32 {
        self.slant
    }

    /// Get the embolden
    pub fn embolden(&self) -> f32 {
        self.embolden
    }

    /// Get the RGB
    pub fn rgb(&self) -> u32 {
        self.rgb_value
    }

    /// Get the font
    pub fn font(&self) -> &Font {
        &self.font
    }

    /// OpenType MATH queries on the HarfBuzz font.
    pub fn ot_math(&self) -> hb::ot::Math<'_> {
        self.font.hb_font().ot_math()
    }

    /// Whether the font has an OpenType MATH table.
    pub fn is_opentype_math_font(&self) -> bool {
        self.font.hb_font().face().has_ot_math_data()
    }

    /// Get the default direction
    pub fn default_dir(&self) -> u8 {
        pub const UBIDI_DEFAULT_LTR: u8 = 0xFE;
        pub const UBIDI_DEFAULT_RTL: u8 = 0xFF;

        let script = self.hb_buffer.borrow().as_ref().get_script();
        if script.get_horizontal_direction() == hb::Direction::Rtl {
            UBIDI_DEFAULT_RTL
        } else {
            UBIDI_DEFAULT_LTR
        }
    }

    /// Check whether we're using the graphite shaper
    pub fn using_graphite(&self) -> bool {
        self.shaper
            .borrow()
            .as_ref()
            .is_some_and(|s| s.to_bytes() == b"graphite2")
    }

    /// Check whether we're using the OpenType shaper
    pub fn using_opentype(&self) -> bool {
        self.shaper
            .borrow()
            .as_ref()
            .is_some_and(|s| s.to_bytes() == b"ot")
    }

    /// The font's ascent and descent.
    pub fn ascent_and_descent(&self) -> (f32, f32) {
        (self.font.ascent(), self.font.descent())
    }

    /// The font's cap height and x-height.
    pub fn cap_and_x_height(&self) -> (f32, f32) {
        (self.font.cap_height(), self.font.x_height())
    }

    /// The bounds of glyph `gid`, stretched by the extend factor.
    pub fn glyph_bounds(&self, gid: u32) -> GlyphBBox {
        let mut bbox = self.font.glyph_bounds(gid as u16);
        if self.extend != 0.0 {
            bbox.x_min *= self.extend;
            bbox.x_max *= self.extend;
        }
        bbox
    }

    /// The advance of glyph `gid`, stretched by the extend factor.
    pub fn glyph_width(&self, gid: u32) -> f32 {
        self.extend * self.font.glyph_width(gid)
    }

    /// The height and depth of glyph `gid`.
    pub fn glyph_height_depth(&self, gid: u32) -> (f32, f32) {
        self.font.glyph_height_depth(gid as u16)
    }

    /// The sidebearings of glyph `gid`, stretched by the extend factor.
    pub fn glyph_sidebearings(&self, gid: u32) -> (f32, f32) {
        let (lsb, rsb) = self.font.glyph_sidebearings(gid as u16);
        if self.extend != 0.0 {
            (lsb * self.extend, rsb * self.extend)
        } else {
            (lsb, rsb)
        }
    }

    /// The italic correction of glyph `gid`, stretched by the extend factor.
    pub fn glyph_ital_corr(&self, gid: u32) -> f32 {
        self.extend * self.font.glyph_ital_corr(gid as u16)
    }

    /// The glyph of character `c`, or 0.
    pub fn map_char_to_glyph(&self, c: u32) -> u32 {
        u32::from(self.font.map_char_to_glyph(c))
    }

    /// The glyph named `name`, or 0.
    pub fn map_glyph_to_index(&self, name: &[u8]) -> u32 {
        let name = CString::new(name).unwrap_or_default();
        u32::from(self.font.map_glyph_to_index(&name))
    }

    /// The first or last character code the font maps.
    pub fn font_char_range(&self, first: bool) -> u32 {
        if first {
            self.font.first_char_code()
        } else {
            self.font.last_char_code()
        }
    }

    /// Shapes `chars` and returns the glyph count.
    pub fn layout_chars(&self, chars: &[u16], rtl: bool) -> usize {
        let hb_font = self.font.hb_font();
        let hb_face = hb_font.face();
        let mut hb_buffer = self.hb_buffer.borrow_mut();

        let direction = if self.font.layout_dir_vertical() {
            hb::Direction::Ttb
        } else if rtl {
            hb::Direction::Rtl
        } else {
            hb::Direction::Ltr
        };

        let script = self.script.to_script();
        hb_buffer.as_mut().reset();
        hb_buffer.as_mut().add_utf16(chars);
        hb_buffer.as_mut().set_direction(direction);
        hb_buffer.as_mut().set_script(script);
        hb_buffer.as_mut().set_language(self.language);

        hb_buffer.as_mut().guess_segment_properties();
        let segment_props = hb_buffer.as_mut().get_segment_properties();

        let mut shape_plan = hb::ShapePlan::new_cached(
            hb_face,
            &segment_props,
            &self.features,
            Some(&self.shaper_list),
        );
        let res = shape_plan
            .as_mut()
            .execute(hb_font, hb_buffer.as_mut(), &self.features);

        let shaper = if res {
            shape_plan.as_ref().get_shaper().map(CStr::to_owned)
        } else {
            // all selected shapers failed, retrying with default
            // we don't use _cached here as the cached plan will always fail.
            shape_plan = hb::ShapePlan::new(hb_face, &segment_props, &self.features, None);
            let res = shape_plan
                .as_mut()
                .execute(hb_font, hb_buffer.as_mut(), &self.features);

            if !res {
                panic!("all shapers failed");
            }
            shape_plan.as_ref().get_shaper().map(CStr::to_owned)
        };
        *self.shaper.borrow_mut() = shaper;

        hb_buffer.as_ref().len()
    }

    /// The glyphs, advances and positions of the last [`Self::layout_chars`],
    /// with one more position than glyphs for the end point.
    pub fn last_layout(&self) -> (Vec<u32>, Vec<f32>, Vec<FloatPoint>) {
        let hb_buffer = self.hb_buffer.borrow();
        let glyphs = hb_buffer
            .as_ref()
            .glyph_info()
            .unwrap()
            .iter()
            .map(|glyph| glyph.codepoint)
            .collect();
        let hb_positions = hb_buffer.as_ref().glyph_positions().unwrap();
        let font = self.font();
        let vertical = font.layout_dir_vertical();

        let advances = hb_positions
            .iter()
            .map(|pos| {
                let advance = if vertical {
                    pos.y_advance
                } else {
                    pos.x_advance
                };
                font.units_to_points(advance as f64) as f32
            })
            .collect();

        let mut positions = Vec::with_capacity(hb_positions.len() + 1);
        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        if vertical {
            for pos in hb_positions {
                positions.push(FloatPoint {
                    x: -font.units_to_points((x + pos.y_offset as f32) as f64) as f32,
                    y: font.units_to_points((y - pos.x_offset as f32) as f64) as f32,
                });
                x += pos.y_advance as f32;
                y += pos.x_advance as f32;
            }
            positions.push(FloatPoint {
                x: -font.units_to_points(x as f64) as f32,
                y: font.units_to_points(y as f64) as f32,
            });
        } else {
            for pos in hb_positions {
                positions.push(FloatPoint {
                    x: font.units_to_points((x + pos.x_offset as f32) as f64) as f32,
                    // Negative is upwards.
                    y: -font.units_to_points((y + pos.y_offset as f32) as f64) as f32,
                });
                x += pos.x_advance as f32;
                y += pos.y_advance as f32;
            }
            positions.push(FloatPoint {
                x: font.units_to_points(x as f64) as f32,
                y: -font.units_to_points(y as f64) as f32,
            });
        }

        if self.extend != 1.0 || self.slant != 0.0 {
            for pos in &mut positions {
                pos.x = pos.x * self.extend - pos.y * self.slant;
            }
        }

        (glyphs, advances, positions)
    }

    fn gr_face(&self) -> Option<gr::FaceRef<'_>> {
        self.font.hb_font().face().gr_face()
    }

    fn language_tag(&self) -> u32 {
        self.language
            .to_string()
            .map(hb::Tag::from_cstr)
            .map(hb::Tag::to_raw)
            .unwrap_or(0)
    }

    /// The number of Graphite features.
    pub fn count_graphite_features(&self) -> u32 {
        self.gr_face()
            .map_or(0, |face| face.num_feature_refs() as u32)
    }

    /// The id of Graphite feature `index`, or 0.
    pub fn graphite_feature_code(&self, index: u32) -> u32 {
        self.gr_face()
            .and_then(|face| Some(face.feature_ref(index as usize)?.id()))
            .unwrap_or(0)
    }

    /// The number of settings of Graphite feature `feature_id`.
    pub fn count_graphite_feature_settings(&self, feature_id: u32) -> u32 {
        self.gr_face()
            .and_then(|face| Some(face.find_feature_ref(feature_id)?.num_values() as u32))
            .unwrap_or(0)
    }

    /// The value of setting `index` of Graphite feature `feature_id`, or 0.
    pub fn graphite_feature_setting_code(&self, feature_id: u32, index: u32) -> u32 {
        self.gr_face()
            .and_then(|face| Some(face.find_feature_ref(feature_id)?.value(index as usize) as u32))
            .unwrap_or(0)
    }

    /// The default setting of Graphite feature `feature_id` for this language, or 0.
    pub fn graphite_feature_default_setting(&self, feature_id: u32) -> u32 {
        let Some(face) = self.gr_face() else {
            return 0;
        };
        let Some(feat) = face.find_feature_ref(feature_id) else {
            return 0;
        };
        let feature_values = face.feature_val_for_lang(self.language_tag());
        feat.feat_value(feature_values.as_ref()) as u32
    }

    /// The label of Graphite feature `feature_id`.
    pub fn graphite_feature_label(&self, feature_id: u32) -> Option<Vec<u8>> {
        let label = self
            .gr_face()?
            .find_feature_ref(feature_id)?
            .label(ENGLISH_LANG_ID)?;
        Some(label_bytes(&label))
    }

    /// The label of setting `setting_id` of Graphite feature `feature_id`.
    pub fn graphite_feature_setting_label(
        &self,
        feature_id: u32,
        setting_id: u32,
    ) -> Option<Vec<u8>> {
        let feature = self.gr_face()?.find_feature_ref(feature_id)?;
        let i = (0..feature.num_values()).find(|&i| setting_id == feature.value(i) as u32)?;
        Some(label_bytes(&feature.value_label(i, ENGLISH_LANG_ID)?))
    }

    /// The Graphite feature and value named by `spec`, a `name=value` pair.
    pub fn find_graphite_feature(&self, spec: &[u8]) -> Option<(u32, i32)> {
        let mut idx = 0;
        while spec.get(idx).is_some_and(|c| *c == b' ' || *c == b'\t') {
            idx += 1;
        }
        while spec.get(idx).is_some_and(|c| *c != b'=') {
            idx += 1;
        }

        let tag = self.graphite_feature_named(&spec[..idx])?;

        idx += 1;
        while idx < spec.len() && (spec[idx] == b' ' || spec[idx] == b'\t') {
            idx += 1;
        }
        if idx >= spec.len() {
            return None;
        }

        let value = self.find_graphite_feature_setting_named(tag, &spec[idx..])?;
        (value != -1).then_some((tag, i32::from(value)))
    }

    /// The Graphite feature whose tag is `name` or whose label starts with it.
    pub fn find_graphite_feature_named(&self, name: &[u8]) -> Option<u32> {
        if name.is_empty() {
            return None;
        }
        self.graphite_feature_named(name)
    }

    /// As [`Self::find_graphite_feature_named`], but an empty name matches the
    /// first labelled feature.
    fn graphite_feature_named(&self, name: &[u8]) -> Option<u32> {
        let gr_face = self.gr_face()?;
        // Matching the tag as well as the label prefix is a Tectonic extension.
        let tag = std::str::from_utf8(name).map_or(0, |n| hb::Tag::from_str(n).to_raw());

        (0..gr_face.num_feature_refs())
            .filter_map(|i| gr_face.feature_ref(i))
            .find(|feature| {
                feature.id() == tag
                    || feature
                        .label(ENGLISH_LANG_ID)
                        .is_some_and(|l| l.as_bytes().starts_with(name))
            })
            .map(|feature| feature.id())
    }

    /// The value of the setting of feature `id` whose label starts with `name`.
    pub fn find_graphite_feature_setting_named(&self, id: u32, name: &[u8]) -> Option<i16> {
        if name.is_empty() {
            return None;
        }
        let feature = self.gr_face()?.find_feature_ref(id)?;
        (0..feature.num_values())
            .find(|&i| {
                feature
                    .value_label(i, ENGLISH_LANG_ID)
                    .is_some_and(|l| l.as_bytes().starts_with(name))
            })
            .map(|i| feature.value(i))
    }

    /// The Graphite break opportunities in `text`, if Graphite can break it.
    pub fn graphite_breaks<'t>(&'t self, text: &'t [u16]) -> Option<GraphiteBreaks<'t>> {
        *self.gr_breaking.borrow_mut() = None;

        let hb_font = self.font.hb_font();
        let gr_face = hb_font.face().gr_face()?;
        let gr_font = gr::Font::new(hb_font.ptem(), gr_face)?;

        let mut gr_feature_values = gr_face.feature_val_for_lang(self.language_tag());
        for feature in self.features.iter().rev() {
            if let Some(fref) = gr_face.find_feature_ref(feature.tag) {
                let _ = fref.set_feat_value(gr_feature_values.as_mut(), feature.value as u16);
            }
        }

        let segment = gr::Segment::new(
            gr_font.as_ref(),
            gr_face,
            self.script.to_raw(),
            gr_feature_values.as_ref(),
            text,
        )
        .unwrap();

        *self.gr_breaking.borrow_mut() = Some(GrBreak {
            slot: segment.as_ref().first_slot(),
            segment,
            text_len: text.len() as u32,
        });

        Some(GraphiteBreaks {
            engine: self,
            _text: PhantomData,
        })
    }
}

/// Graphite's language id for English labels.
const ENGLISH_LANG_ID: u16 = 0x409;

/// A label's bytes up to any NUL.
fn label_bytes(label: &gr::Label) -> Vec<u8> {
    let bytes = label.as_bytes();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    bytes[..end].to_vec()
}

/// The break positions of the text given to [`LayoutEngine::graphite_breaks`].
pub struct GraphiteBreaks<'t> {
    engine: &'t LayoutEngine<'t>,
    _text: PhantomData<&'t [u16]>,
}

impl GraphiteBreaks<'_> {
    /// The next break offset, or -1 at the end.
    pub fn next_break(&mut self) -> i32 {
        let mut gr_breaking = self.engine.gr_breaking.borrow_mut();
        let Some(breaking) = gr_breaking.as_mut() else {
            return -1;
        };

        let segment = breaking.segment.as_ref();
        if breaking.slot == segment.last_slot() {
            return -1;
        }

        let mut s = segment.next(&breaking.slot);
        let mut ret = -1;
        while let Some(slot) = s {
            let ci = segment.cinfo(segment.index(&slot));
            let bw = ci.break_weight();
            if (gr::BREAK_BEFORE_WORD..gr::BREAK_NONE).contains(&bw) {
                breaking.slot = slot.clone();
                ret = ci.base() as i32;
            } else if (gr::BREAK_NONE + 1..=gr::BREAK_WORD).contains(&bw) {
                breaking.slot = segment.next(&slot).unwrap();
                ret = (ci.base() + 1) as i32;
            }

            if ret != -1 {
                break;
            }

            s = segment.next(&slot);
        }

        if ret == -1 {
            breaking.slot = segment.last_slot();
            breaking.text_len as i32
        } else {
            ret
        }
    }
}
