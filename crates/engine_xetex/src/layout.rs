//! Safe handles over the C APIs of the native font layer: `tectonic_xetex_layout`
//! engines and fonts, HarfBuzz tags, TECkit converters and ICU line breaking
//! and bidi. Every raw pointer stays inside this module.

use std::ffi::{c_char, c_int, c_long, c_uint, c_void, CStr};
use std::marker::PhantomData;
use std::ptr::{self, NonNull};

use tectonic_bridge_harfbuzz::sys::hb_font_t;

type RawEngine = *mut c_void;
type RawFont = *mut c_void;
type RawPlatformFont = *mut c_void;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub(crate) struct FloatPoint {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Default)]
pub(crate) struct GlyphBBox {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

#[repr(C)]
pub(crate) struct HbFeature {
    tag: u32,
    value: u32,
    start: u32,
    end: u32,
}

impl HbFeature {
    pub(crate) fn global(tag: u32, value: u32) -> HbFeature {
        HbFeature {
            tag,
            value,
            start: 0,
            end: u32::MAX,
        }
    }
}

unsafe extern "C" {
    pub(crate) safe fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;
    pub(crate) safe fn set_cp_code(font_num: i32, code: u32, side: i32, value: i32);
    pub(crate) safe fn get_loaded_font_design_size() -> i32;
    pub(crate) safe fn set_loaded_font_design_size(val: i32);
    #[link_name = "getReqEngine"]
    safe fn raw_req_engine() -> c_char;
    #[link_name = "setReqEngine"]
    safe fn raw_set_req_engine(engine: c_char);

    fn hb_tag_from_string(str_: *const c_char, len: c_int) -> u32;

    fn deleteLayoutEngine(engine: RawEngine);
    fn createLayoutEngine(
        font: RawFont,
        script: u32,
        language: *mut c_char,
        features: *mut HbFeature,
        n_features: c_int,
        shapers: *mut *const c_char,
        rgb_value: u32,
        extend: f32,
        slant: f32,
        embolden: f32,
    ) -> RawEngine;
    fn createLayoutEngineBorrowed(
        font: RawFont,
        script: u32,
        language: *mut c_char,
        features: *mut HbFeature,
        n_features: c_int,
        shapers: *const *const c_char,
        rgb_value: u32,
        extend: f32,
        slant: f32,
        embolden: f32,
    ) -> RawEngine;
    fn getFont(engine: RawEngine) -> RawFont;
    fn getExtendFactor(engine: RawEngine) -> f32;
    fn getSlantFactor(engine: RawEngine) -> f32;
    fn getEmboldenFactor(engine: RawEngine) -> f32;
    fn getPointSize(engine: RawEngine) -> f32;
    fn getRgbValue(engine: RawEngine) -> u32;
    fn getAscentAndDescent(engine: RawEngine, ascent: *mut f32, descent: *mut f32);
    fn getCapAndXHeight(engine: RawEngine, capheight: *mut f32, xheight: *mut f32);
    fn getGlyphHeightDepth(engine: RawEngine, glyph_id: u32, height: *mut f32, depth: *mut f32);
    fn getGlyphSidebearings(engine: RawEngine, glyph_id: u32, lsb: *mut f32, rsb: *mut f32);
    fn getGlyphItalCorr(engine: RawEngine, glyph_id: u32) -> f32;
    fn getGlyphWidthFromEngine(engine: RawEngine, glyph_id: u32) -> f32;
    fn getGlyphBounds(engine: RawEngine, glyph_id: u32, bbox: *mut GlyphBBox);
    fn mapCharToGlyph(engine: RawEngine, char_code: u32) -> u32;
    fn mapGlyphToIndex(engine: RawEngine, glyph_name: *const c_char) -> c_int;
    fn getFontCharRange(engine: RawEngine, req_first: c_int) -> c_int;
    fn getFontFilename(engine: RawEngine, index: *mut u32) -> *const c_char;
    fn freeFontFilename(filename: *const c_char);
    fn getDefaultDirection(engine: RawEngine) -> c_int;
    fn layoutChars(
        engine: RawEngine,
        chars: *mut u16,
        offset: i32,
        count: i32,
        max: i32,
        rtl: bool,
    ) -> c_int;
    fn getGlyphs(engine: RawEngine, glyphs: *mut u32);
    fn getGlyphAdvances(engine: RawEngine, advances: *mut f32);
    fn getGlyphPositions(engine: RawEngine, positions: *mut FloatPoint);
    fn usingGraphite(engine: RawEngine) -> bool;
    fn usingOpenType(engine: RawEngine) -> bool;
    fn isOpenTypeMathFont(engine: RawEngine) -> bool;
    fn ttxl_get_hb_font(engine: RawEngine) -> *mut hb_font_t;
    fn countGraphiteFeatures(engine: RawEngine) -> u32;
    fn getGraphiteFeatureCode(engine: RawEngine, index: u32) -> u32;
    fn countGraphiteFeatureSettings(engine: RawEngine, feature_id: u32) -> u32;
    fn getGraphiteFeatureSettingCode(engine: RawEngine, feature_id: u32, setting_index: u32)
        -> u32;
    fn getGraphiteFeatureDefaultSetting(engine: RawEngine, feature_id: u32) -> u32;
    fn getGraphiteFeatureLabel(engine: RawEngine, feature_id: u32) -> *mut c_char;
    fn getGraphiteFeatureSettingLabel(
        engine: RawEngine,
        feature_id: u32,
        setting_index: u32,
    ) -> *mut c_char;
    fn gr_label_destroy(name: *mut c_void);
    fn findGraphiteFeatureNamed(
        engine: RawEngine,
        name: *const c_char,
        namelength: c_int,
    ) -> c_long;
    fn findGraphiteFeatureSettingNamed(
        engine: RawEngine,
        feature_id: u32,
        name: *const c_char,
        namelength: c_int,
    ) -> c_long;
    fn findGraphiteFeature(
        engine: RawEngine,
        s: *const c_char,
        e: *const c_char,
        f: *mut u32,
        v: *mut c_int,
    ) -> bool;
    fn initGraphiteBreaking(engine: RawEngine, txt_ptr: *const u16, txt_len: u32) -> bool;
    fn findNextGraphiteBreak(engine: RawEngine) -> c_int;

    fn createFont(font_ref: RawPlatformFont, point_size: i32) -> RawFont;
    fn createFontFromFile(filename: *const c_char, index: c_int, point_size: i32) -> RawFont;
    fn deleteFont(font: RawFont);
    fn getDesignSize(font: RawFont) -> f64;
    fn setFontLayoutDir(font: RawFont, vertical: c_int);
    fn getSlant(font: RawFont) -> i32;
    fn getGlyphWidth(font: RawFont, gid: u32) -> f32;
    fn getGlyphName(font: RawFont, gid: u16, len: *mut c_int) -> *const c_char;
    fn freeGlyphName(name: *mut c_char);
    fn countGlyphs(font: RawFont) -> c_uint;
    fn countScripts(font: RawFont) -> c_uint;
    fn countLanguages(font: RawFont, script: u32) -> c_uint;
    fn countFeatures(font: RawFont, script: u32, language: u32) -> c_uint;
    fn getIndScript(font: RawFont, index: c_uint) -> u32;
    fn getIndLanguage(font: RawFont, script: u32, index: c_uint) -> u32;
    fn getIndFeature(font: RawFont, script: u32, language: u32, index: c_uint) -> u32;
    fn ttxl_font_units_to_points(font: RawFont, units: f32) -> f32;
    fn ttxl_font_points_to_units(font: RawFont, points: f32) -> f32;
    fn ttxl_font_get_point_size(font: RawFont) -> f32;

    fn findFontByName(name: *const c_char, var: *mut c_char, size: f64) -> RawPlatformFont;
    fn getFullName(font: RawPlatformFont) -> *const c_char;
    fn ttxl_platfont_get_desc(font: RawPlatformFont) -> *const c_char;

    fn getCachedGlyphBBox(font_id: u16, glyph_id: u16, bbox: *mut GlyphBBox) -> i32;
    fn cacheGlyphBBox(font_id: u16, glyph_id: u16, bbox: *const GlyphBBox);

    // TECkit_Status is a C `long`, which is 32-bit on wasm32 and Windows.
    fn TECkit_CreateConverter(
        mapping: *const u8,
        mapping_size: u32,
        map_forward: u8,
        source_form: u16,
        target_form: u16,
        converter: *mut *mut c_void,
    ) -> c_long;
    fn TECkit_ConvertBuffer(
        converter: *mut c_void,
        in_buffer: *const u8,
        in_length: u32,
        in_used: *mut u32,
        out_buffer: *mut u8,
        out_length: u32,
        out_used: *mut u32,
        input_is_complete: u8,
    ) -> c_long;
    fn TECkit_ResetConverter(converter: *mut c_void) -> c_long;
    fn TECkit_DisposeConverter(converter: *mut c_void) -> c_long;
}

// ICU checks the incoming UErrorCode, so each status starts at zero.
unsafe extern "C" {
    #[link_name = concat!("ubidi_open", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_open() -> *mut c_void;
    #[link_name = concat!("ubidi_close", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_close(bidi: *mut c_void);
    #[link_name = concat!("ubidi_setPara", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_setPara(
        bidi: *mut c_void,
        text: *const u16,
        length: i32,
        para_level: u8,
        embedding_levels: *mut u8,
        status: *mut i32,
    );
    #[link_name = concat!("ubidi_getDirection", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_getDirection(bidi: *mut c_void) -> c_int;
    #[link_name = concat!("ubidi_countRuns", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_countRuns(bidi: *mut c_void, status: *mut i32) -> c_int;
    #[link_name = concat!("ubidi_getVisualRun", env!("ICU_VERSION_SUFFIX"))]
    fn ubidi_getVisualRun(
        bidi: *mut c_void,
        run_index: i32,
        logical_start: *mut i32,
        length: *mut i32,
    ) -> c_int;
    #[link_name = concat!("ubrk_open", env!("ICU_VERSION_SUFFIX"))]
    fn ubrk_open(
        type_: c_int,
        locale: *const c_char,
        text: *const u16,
        text_length: i32,
        status: *mut i32,
    ) -> *mut c_void;
    #[link_name = concat!("ubrk_close", env!("ICU_VERSION_SUFFIX"))]
    fn ubrk_close(bi: *mut c_void);
    #[link_name = concat!("ubrk_setText", env!("ICU_VERSION_SUFFIX"))]
    fn ubrk_setText(bi: *mut c_void, text: *const u16, text_length: i32, status: *mut i32);
    #[link_name = concat!("ubrk_next", env!("ICU_VERSION_SUFFIX"))]
    fn ubrk_next(bi: *mut c_void) -> c_int;
}

/// The shaper `getReqEngine` asks for: `b'O'`, `b'G'`, `b'A'` or 0.
pub(crate) fn req_engine() -> u8 {
    raw_req_engine() as u8
}

pub(crate) fn set_req_engine(engine: u8) {
    raw_set_req_engine(engine as c_char);
}

/// HarfBuzz's tag for the text `tag`.
pub(crate) fn hb_tag(tag: &[u8]) -> u32 {
    // SAFETY: `tag` is valid for the length passed.
    unsafe { hb_tag_from_string(tag.as_ptr().cast(), tag.len() as c_int) }
}

/// The glyph bounds cached for font `f`, if any.
pub(crate) fn cached_glyph_bbox(f: u16, gid: u16) -> Option<GlyphBBox> {
    let mut bbox = GlyphBBox::default();
    // SAFETY: `bbox` is a valid out-parameter.
    (unsafe { getCachedGlyphBBox(f, gid, &mut bbox) } != 0).then_some(bbox)
}

pub(crate) fn cache_glyph_bbox(f: u16, gid: u16, bbox: &GlyphBBox) {
    // SAFETY: `bbox` is valid for reads.
    unsafe { cacheGlyphBBox(f, gid, bbox) };
}

/// Copies a C string the callee owns, then frees it with `free`.
///
/// # Safety
///
/// `s` must be null or a live NUL-terminated string that `free` accepts.
unsafe fn take_c_string(
    s: *mut c_char,
    free: unsafe extern "C" fn(*mut c_char),
) -> Option<Vec<u8>> {
    if s.is_null() {
        return None;
    }
    let bytes = CStr::from_ptr(s).to_bytes().to_vec();
    free(s);
    Some(bytes)
}

unsafe extern "C" fn free_gr_label(s: *mut c_char) {
    gr_label_destroy(s.cast());
}

/// A font not owned by any layout engine yet.
pub(crate) struct OwnedFont {
    raw: NonNull<c_void>,
}

impl Drop for OwnedFont {
    fn drop(&mut self) {
        // SAFETY: this font is owned here and nothing else refers to it.
        unsafe { deleteFont(self.raw.as_ptr()) };
    }
}

impl OwnedFont {
    /// The font in file `path` at face `index`, or `None`.
    pub(crate) fn from_file(path: &CStr, index: c_int, point_size: i32) -> Option<OwnedFont> {
        // SAFETY: `path` is NUL-terminated.
        let raw = unsafe { createFontFromFile(path.as_ptr(), index, point_size) };
        NonNull::new(raw).map(|raw| OwnedFont { raw })
    }

    pub(crate) fn design_size(&self) -> f64 {
        // SAFETY: `raw` is a live font.
        unsafe { getDesignSize(self.raw.as_ptr()) }
    }

    pub(crate) fn set_layout_dir(&self, vertical: bool) {
        // SAFETY: `raw` is a live font.
        unsafe { setFontLayoutDir(self.raw.as_ptr(), vertical as c_int) };
    }
}

/// A font owned by a live layout engine.
#[derive(Clone, Copy)]
pub(crate) struct FontRef<'a> {
    raw: RawFont,
    _engine: PhantomData<&'a FontEngineHandle>,
}

impl FontRef<'_> {
    pub(crate) fn slant(self) -> i32 {
        // SAFETY: the engine keeps `raw` alive for `'a`.
        unsafe { getSlant(self.raw) }
    }

    pub(crate) fn glyph_width(self, gid: u32) -> f32 {
        // SAFETY: as in `slant`.
        unsafe { getGlyphWidth(self.raw, gid) }
    }

    /// The bytes of glyph `gid`'s name.
    pub(crate) fn glyph_name(self, gid: u16) -> Vec<u8> {
        let mut len: c_int = 0;
        // SAFETY: as in `slant`, and `len` is a valid out-parameter.
        let s = unsafe { getGlyphName(self.raw, gid, &mut len) };
        if s.is_null() {
            return Vec::new();
        }
        // SAFETY: the name holds `len` bytes and is ours to free.
        unsafe {
            let bytes = std::slice::from_raw_parts(s.cast::<u8>(), len.max(0) as usize).to_vec();
            freeGlyphName(s.cast_mut());
            bytes
        }
    }

    pub(crate) fn count_glyphs(self) -> u32 {
        // SAFETY: as in `slant`.
        unsafe { countGlyphs(self.raw) }
    }

    pub(crate) fn count_scripts(self) -> u32 {
        // SAFETY: as in `slant`.
        unsafe { countScripts(self.raw) }
    }

    pub(crate) fn count_languages(self, script: u32) -> u32 {
        // SAFETY: as in `slant`.
        unsafe { countLanguages(self.raw, script) }
    }

    pub(crate) fn count_features(self, script: u32, language: u32) -> u32 {
        // SAFETY: as in `slant`.
        unsafe { countFeatures(self.raw, script, language) }
    }

    pub(crate) fn ind_script(self, index: u32) -> u32 {
        // SAFETY: as in `slant`.
        unsafe { getIndScript(self.raw, index) }
    }

    pub(crate) fn ind_language(self, script: u32, index: u32) -> u32 {
        // SAFETY: as in `slant`.
        unsafe { getIndLanguage(self.raw, script, index) }
    }

    pub(crate) fn ind_feature(self, script: u32, language: u32, index: u32) -> u32 {
        // SAFETY: as in `slant`.
        unsafe { getIndFeature(self.raw, script, language, index) }
    }

    pub(crate) fn units_to_points(self, units: f32) -> f32 {
        // SAFETY: as in `slant`.
        unsafe { ttxl_font_units_to_points(self.raw, units) }
    }

    pub(crate) fn points_to_units(self, points: f32) -> f32 {
        // SAFETY: as in `slant`.
        unsafe { ttxl_font_points_to_units(self.raw, points) }
    }

    pub(crate) fn point_size(self) -> f32 {
        // SAFETY: as in `slant`.
        unsafe { ttxl_font_get_point_size(self.raw) }
    }
}

/// A platform font reference from the font manager, which caches it for the run.
pub(crate) struct PlatformFont {
    raw: NonNull<c_void>,
}

impl PlatformFont {
    /// The installed font `name` with variant `var`, which the manager may edit
    /// in place, or `None`.
    pub(crate) fn find(name: &CStr, var: Option<&mut Vec<u8>>, size: f64) -> Option<PlatformFont> {
        let var = var.map_or(ptr::null_mut(), |v| {
            if v.last() != Some(&0) {
                v.push(0);
            }
            v.as_mut_ptr().cast()
        });
        // SAFETY: `name` and `var` (when not null) are NUL-terminated and `var`
        // is writable up to its NUL.
        let raw = unsafe { findFontByName(name.as_ptr(), var, size) };
        NonNull::new(raw).map(|raw| PlatformFont { raw })
    }

    pub(crate) fn full_name(&self) -> Vec<u8> {
        // SAFETY: the manager keeps `raw` and the name it returns alive.
        unsafe { CStr::from_ptr(getFullName(self.raw.as_ptr())) }
            .to_bytes()
            .to_vec()
    }

    pub(crate) fn desc(&self) -> Vec<u8> {
        // SAFETY: as in `full_name`.
        unsafe { CStr::from_ptr(ttxl_platfont_get_desc(self.raw.as_ptr())) }
            .to_bytes()
            .to_vec()
    }

    pub(crate) fn create_font(&self, point_size: i32) -> Option<OwnedFont> {
        // SAFETY: `raw` is a live platform font.
        let raw = unsafe { createFont(self.raw.as_ptr(), point_size) };
        NonNull::new(raw).map(|raw| OwnedFont { raw })
    }
}

/// Options of a new layout engine. `shapers` must be null-terminated when not empty.
pub(crate) struct EngineOptions<'a> {
    pub script: u32,
    pub language: Option<&'a CStr>,
    pub features: &'a mut [HbFeature],
    pub shapers: &'a mut [*const c_char],
    pub rgb_value: u32,
    pub extend: f32,
    pub slant: f32,
    pub embolden: f32,
}

/// A layout engine that borrows its font, for probing Graphite feature names.
pub(crate) struct ProbeEngine<'f> {
    raw: NonNull<c_void>,
    _font: PhantomData<&'f OwnedFont>,
}

impl Drop for ProbeEngine<'_> {
    fn drop(&mut self) {
        // SAFETY: the probe is owned here and nothing else refers to it.
        unsafe { deleteLayoutEngine(self.raw.as_ptr()) };
    }
}

impl<'f> ProbeEngine<'f> {
    /// A Graphite engine over `font` with default options.
    pub(crate) fn graphite(font: &'f OwnedFont, script: u32, rgb_value: u32) -> Option<Self> {
        let shapers = [c"graphite2".as_ptr(), ptr::null()];
        // SAFETY: `font` stays alive for `'f`, the shaper list is
        // null-terminated and outlives the call.
        let raw = unsafe {
            createLayoutEngineBorrowed(
                font.raw.as_ptr(),
                script,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                shapers.as_ptr(),
                rgb_value,
                1.0,
                0.0,
                0.0,
            )
        };
        NonNull::new(raw).map(|raw| ProbeEngine {
            raw,
            _font: PhantomData,
        })
    }

    /// The Graphite feature and value named by `spec`.
    pub(crate) fn find_graphite_feature(&self, spec: &[u8]) -> Option<(u32, c_int)> {
        let (mut tag, mut value) = (0, 0);
        let range = spec.as_ptr_range();
        // SAFETY: `spec` is valid between the two pointers and the outputs are
        // valid out-parameters.
        let found = unsafe {
            findGraphiteFeature(
                self.raw.as_ptr(),
                range.start.cast(),
                range.end.cast(),
                &mut tag,
                &mut value,
            )
        };
        found.then_some((tag, value))
    }
}

/// An owned OpenType/Graphite layout engine with exactly-once disposal. Checkpoints
/// may share it through an `Arc`, and it is released with its last owner.
pub(crate) struct FontEngineHandle {
    raw: NonNull<c_void>,
}

impl Drop for FontEngineHandle {
    fn drop(&mut self) {
        // SAFETY: the engine is owned here and the last owner is dropping it.
        unsafe { deleteLayoutEngine(self.raw.as_ptr()) };
    }
}

impl FontEngineHandle {
    /// An engine for `font` with `options`, which takes the font over.
    pub(crate) fn new(font: OwnedFont, options: EngineOptions) -> Option<FontEngineHandle> {
        let EngineOptions {
            script,
            language,
            features,
            shapers,
            rgb_value,
            extend,
            slant,
            embolden,
        } = options;
        let features_ptr = if features.is_empty() {
            ptr::null_mut()
        } else {
            features.as_mut_ptr()
        };
        let shapers_ptr = if shapers.is_empty() {
            ptr::null_mut()
        } else {
            shapers.as_mut_ptr()
        };
        // SAFETY: `font` is live, `language` is null or NUL-terminated, the
        // feature and shaper arrays are valid for the call and the shaper
        // names are 'static.
        let raw = unsafe {
            createLayoutEngine(
                font.raw.as_ptr(),
                script,
                language.map_or(ptr::null_mut(), |l| l.as_ptr().cast_mut()),
                features_ptr,
                features.len() as c_int,
                shapers_ptr,
                rgb_value,
                extend,
                slant,
                embolden,
            )
        };
        let raw = NonNull::new(raw)?;
        std::mem::forget(font);
        Some(FontEngineHandle { raw })
    }

    fn raw(&self) -> RawEngine {
        self.raw.as_ptr()
    }

    pub(crate) fn font(&self) -> FontRef<'_> {
        FontRef {
            // SAFETY: `raw` is a live engine.
            raw: unsafe { getFont(self.raw()) },
            _engine: PhantomData,
        }
    }

    /// The HarfBuzz font, alive as long as this engine.
    pub(crate) fn hb_font(&self) -> *mut hb_font_t {
        // SAFETY: `raw` is a live engine.
        unsafe { ttxl_get_hb_font(self.raw()) }
    }

    pub(crate) fn extend_factor(&self) -> f32 {
        // SAFETY: `raw` is a live engine.
        unsafe { getExtendFactor(self.raw()) }
    }

    pub(crate) fn slant_factor(&self) -> f32 {
        // SAFETY: `raw` is a live engine.
        unsafe { getSlantFactor(self.raw()) }
    }

    pub(crate) fn embolden_factor(&self) -> f32 {
        // SAFETY: `raw` is a live engine.
        unsafe { getEmboldenFactor(self.raw()) }
    }

    pub(crate) fn point_size(&self) -> f32 {
        // SAFETY: `raw` is a live engine.
        unsafe { getPointSize(self.raw()) }
    }

    pub(crate) fn rgb_value(&self) -> u32 {
        // SAFETY: `raw` is a live engine.
        unsafe { getRgbValue(self.raw()) }
    }

    pub(crate) fn ascent_and_descent(&self) -> (f32, f32) {
        let (mut a, mut d) = (0.0, 0.0);
        // SAFETY: `raw` is a live engine and the outputs are valid.
        unsafe { getAscentAndDescent(self.raw(), &mut a, &mut d) };
        (a, d)
    }

    pub(crate) fn cap_and_x_height(&self) -> (f32, f32) {
        let (mut cap, mut x) = (0.0, 0.0);
        // SAFETY: as in `ascent_and_descent`.
        unsafe { getCapAndXHeight(self.raw(), &mut cap, &mut x) };
        (cap, x)
    }

    pub(crate) fn glyph_height_depth(&self, gid: u32) -> (f32, f32) {
        let (mut ht, mut dp) = (0.0, 0.0);
        // SAFETY: as in `ascent_and_descent`.
        unsafe { getGlyphHeightDepth(self.raw(), gid, &mut ht, &mut dp) };
        (ht, dp)
    }

    pub(crate) fn glyph_sidebearings(&self, gid: u32) -> (f32, f32) {
        let (mut lsb, mut rsb) = (0.0, 0.0);
        // SAFETY: as in `ascent_and_descent`.
        unsafe { getGlyphSidebearings(self.raw(), gid, &mut lsb, &mut rsb) };
        (lsb, rsb)
    }

    pub(crate) fn glyph_ital_corr(&self, gid: u32) -> f32 {
        // SAFETY: `raw` is a live engine.
        unsafe { getGlyphItalCorr(self.raw(), gid) }
    }

    pub(crate) fn glyph_width(&self, gid: u32) -> f32 {
        // SAFETY: `raw` is a live engine.
        unsafe { getGlyphWidthFromEngine(self.raw(), gid) }
    }

    pub(crate) fn glyph_bounds(&self, gid: u32) -> GlyphBBox {
        let mut bbox = GlyphBBox::default();
        // SAFETY: as in `ascent_and_descent`.
        unsafe { getGlyphBounds(self.raw(), gid, &mut bbox) };
        bbox
    }

    pub(crate) fn map_char_to_glyph(&self, c: u32) -> u32 {
        // SAFETY: `raw` is a live engine.
        unsafe { mapCharToGlyph(self.raw(), c) }
    }

    pub(crate) fn map_glyph_to_index(&self, name: &CStr) -> c_int {
        // SAFETY: `raw` is a live engine and `name` is NUL-terminated.
        unsafe { mapGlyphToIndex(self.raw(), name.as_ptr()) }
    }

    pub(crate) fn font_char_range(&self, first: c_int) -> c_int {
        // SAFETY: `raw` is a live engine.
        unsafe { getFontCharRange(self.raw(), first) }
    }

    /// The font file name and face index.
    pub(crate) fn font_filename(&self) -> (Vec<u8>, u32) {
        let mut index = 0;
        // SAFETY: `raw` is a live engine and `index` is a valid out-parameter.
        let name = unsafe { getFontFilename(self.raw(), &mut index) };
        assert!(!name.is_null());
        // SAFETY: the name is NUL-terminated and ours to free.
        let bytes = unsafe {
            let bytes = CStr::from_ptr(name).to_bytes().to_vec();
            freeFontFilename(name);
            bytes
        };
        (bytes, index)
    }

    pub(crate) fn default_direction(&self) -> c_int {
        // SAFETY: `raw` is a live engine.
        unsafe { getDefaultDirection(self.raw()) }
    }

    /// Shapes `text[offset..offset + count]` and returns the glyph count.
    pub(crate) fn layout_chars(&self, text: &[u16], offset: i32, count: i32, rtl: bool) -> c_int {
        assert!(offset >= 0 && count >= 0 && (offset + count) as usize <= text.len());
        // SAFETY: `raw` is a live engine and the shaper only reads `text`
        // within its length.
        unsafe {
            layoutChars(
                self.raw(),
                text.as_ptr().cast_mut(),
                offset,
                count,
                text.len() as i32,
                rtl,
            )
        }
    }

    /// The glyphs, advances and `n + 1` positions of the last `layout_chars`.
    pub(crate) fn last_layout(&self, n: usize) -> (Vec<u32>, Vec<f32>, Vec<FloatPoint>) {
        let mut glyphs = vec![0u32; n];
        let mut advances = vec![0f32; n];
        let mut positions = vec![FloatPoint::default(); n + 1];
        // SAFETY: `raw` is a live engine that just laid out `n` glyphs, and each
        // buffer is sized for what it writes.
        unsafe {
            getGlyphs(self.raw(), glyphs.as_mut_ptr());
            getGlyphAdvances(self.raw(), advances.as_mut_ptr());
            getGlyphPositions(self.raw(), positions.as_mut_ptr());
        }
        (glyphs, advances, positions)
    }

    pub(crate) fn using_graphite(&self) -> bool {
        // SAFETY: `raw` is a live engine.
        unsafe { usingGraphite(self.raw()) }
    }

    pub(crate) fn using_opentype(&self) -> bool {
        // SAFETY: `raw` is a live engine.
        unsafe { usingOpenType(self.raw()) }
    }

    pub(crate) fn is_opentype_math_font(&self) -> bool {
        // SAFETY: `raw` is a live engine.
        unsafe { isOpenTypeMathFont(self.raw()) }
    }

    pub(crate) fn count_graphite_features(&self) -> u32 {
        // SAFETY: `raw` is a live engine.
        unsafe { countGraphiteFeatures(self.raw()) }
    }

    pub(crate) fn graphite_feature_code(&self, index: u32) -> u32 {
        // SAFETY: `raw` is a live engine.
        unsafe { getGraphiteFeatureCode(self.raw(), index) }
    }

    pub(crate) fn count_graphite_feature_settings(&self, feature: u32) -> u32 {
        // SAFETY: `raw` is a live engine.
        unsafe { countGraphiteFeatureSettings(self.raw(), feature) }
    }

    pub(crate) fn graphite_feature_setting_code(&self, feature: u32, index: u32) -> u32 {
        // SAFETY: `raw` is a live engine.
        unsafe { getGraphiteFeatureSettingCode(self.raw(), feature, index) }
    }

    pub(crate) fn graphite_feature_default_setting(&self, feature: u32) -> u32 {
        // SAFETY: `raw` is a live engine.
        unsafe { getGraphiteFeatureDefaultSetting(self.raw(), feature) }
    }

    pub(crate) fn graphite_feature_label(&self, feature: u32) -> Option<Vec<u8>> {
        // SAFETY: `raw` is a live engine, and the label is ours to destroy.
        unsafe { take_c_string(getGraphiteFeatureLabel(self.raw(), feature), free_gr_label) }
    }

    pub(crate) fn graphite_feature_setting_label(
        &self,
        feature: u32,
        index: u32,
    ) -> Option<Vec<u8>> {
        // SAFETY: as in `graphite_feature_label`.
        unsafe {
            take_c_string(
                getGraphiteFeatureSettingLabel(self.raw(), feature, index),
                free_gr_label,
            )
        }
    }

    pub(crate) fn find_graphite_feature_named(&self, name: &[u8]) -> c_long {
        // SAFETY: `raw` is a live engine and `name` is valid for its length.
        unsafe { findGraphiteFeatureNamed(self.raw(), name.as_ptr().cast(), name.len() as c_int) }
    }

    pub(crate) fn find_graphite_feature_setting_named(&self, feature: u32, name: &[u8]) -> c_long {
        // SAFETY: as in `find_graphite_feature_named`.
        unsafe {
            findGraphiteFeatureSettingNamed(
                self.raw(),
                feature,
                name.as_ptr().cast(),
                name.len() as c_int,
            )
        }
    }

    /// Graphite break opportunities in `text`, if Graphite can break it.
    pub(crate) fn graphite_breaks<'t>(&'t self, text: &'t [u16]) -> Option<GraphiteBreaks<'t>> {
        // SAFETY: `raw` is a live engine, and the returned iterator keeps
        // `text` borrowed for as long as the engine may read it.
        let ok = unsafe { initGraphiteBreaking(self.raw(), text.as_ptr(), text.len() as u32) };
        ok.then_some(GraphiteBreaks {
            engine: self,
            _text: PhantomData,
        })
    }
}

/// The break positions of the text given to [`FontEngineHandle::graphite_breaks`].
pub(crate) struct GraphiteBreaks<'t> {
    engine: &'t FontEngineHandle,
    _text: PhantomData<&'t [u16]>,
}

impl GraphiteBreaks<'_> {
    /// The next break offset, or a negative value at the end.
    pub(crate) fn next_break(&mut self) -> c_int {
        // SAFETY: the engine is live and its text is still borrowed.
        unsafe { findNextGraphiteBreak(self.engine.raw()) }
    }
}

/// An owned TECkit converter with exactly-once disposal.
pub(crate) struct FontMappingHandle {
    raw: NonNull<c_void>,
}

impl Drop for FontMappingHandle {
    fn drop(&mut self) {
        // SAFETY: the converter is owned here and the last owner is dropping it.
        unsafe { TECkit_DisposeConverter(self.raw.as_ptr()) };
    }
}

pub(crate) const K_STATUS_NO_ERROR: c_long = 0;
pub(crate) const K_STATUS_OUTPUT_BUFFER_FULL: c_long = 1;

impl FontMappingHandle {
    /// A converter for the compiled `mapping`, or a pure normalizer for `None`,
    /// with its creation status.
    pub(crate) fn new(
        mapping: Option<&[u8]>,
        map_forward: bool,
        source_form: u16,
        target_form: u16,
    ) -> (Option<FontMappingHandle>, c_long) {
        let mut raw: *mut c_void = ptr::null_mut();
        let (mapping_ptr, mapping_len) =
            mapping.map_or((ptr::null(), 0), |m| (m.as_ptr(), m.len()));
        // SAFETY: the mapping is valid for its length and `raw` is a valid out-parameter.
        let status = unsafe {
            TECkit_CreateConverter(
                mapping_ptr,
                mapping_len as u32,
                map_forward as u8,
                source_form,
                target_form,
                &mut raw,
            )
        };
        (
            NonNull::new(raw).map(|raw| FontMappingHandle { raw }),
            status,
        )
    }

    /// Converts all of `input` into `output`, then resets the converter.
    /// Returns the status and the bytes written.
    pub(crate) fn convert(&self, input: &[u8], output: &mut [u8]) -> (c_long, u32) {
        let (mut in_used, mut out_used) = (0u32, 0u32);
        // SAFETY: the converter is live and the buffers and counts are valid
        // for the lengths passed.
        unsafe {
            let status = TECkit_ConvertBuffer(
                self.raw.as_ptr(),
                input.as_ptr(),
                input.len() as u32,
                &mut in_used,
                output.as_mut_ptr(),
                output.len() as u32,
                &mut out_used,
                1,
            );
            TECkit_ResetConverter(self.raw.as_ptr());
            (status, out_used)
        }
    }
}

/// The bytes of UTF-16 `units`.
fn utf16_bytes(units: &[u16]) -> &[u8] {
    // SAFETY: `u16` has no padding, so the view covers initialised bytes.
    unsafe { std::slice::from_raw_parts(units.as_ptr().cast(), std::mem::size_of_val(units)) }
}

fn utf16_bytes_mut(units: &mut [u16]) -> &mut [u8] {
    // SAFETY: as in `utf16_bytes`, and any bytes written form valid `u16`s.
    unsafe {
        std::slice::from_raw_parts_mut(units.as_mut_ptr().cast(), std::mem::size_of_val(units))
    }
}

impl FontMappingHandle {
    /// `convert` over UTF-16 units. The byte count written comes back unchanged.
    pub(crate) fn convert_units(&self, input: &[u16], output: &mut [u16]) -> (c_long, u32) {
        self.convert(utf16_bytes(input), utf16_bytes_mut(output))
    }
}

/// An owned ICU line-break iterator. Its locale key is a TeX string-pool
/// index, so the cache belongs to one engine state and must not outlive it.
pub(crate) struct LinebreakHandle {
    raw: NonNull<c_void>,
    exhausted: bool,
}

impl Drop for LinebreakHandle {
    fn drop(&mut self) {
        // SAFETY: the iterator is owned here.
        unsafe { ubrk_close(self.raw.as_ptr()) };
    }
}

const UBRK_LINE: c_int = 2;

impl LinebreakHandle {
    /// A line-break iterator for `locale`, and ICU's status.
    pub(crate) fn open(locale: &CStr) -> (Option<LinebreakHandle>, i32) {
        let mut status = 0;
        // SAFETY: `locale` is NUL-terminated, no text is attached yet and
        // `status` is a valid out-parameter.
        let raw = unsafe { ubrk_open(UBRK_LINE, locale.as_ptr(), ptr::null(), 0, &mut status) };
        (
            NonNull::new(raw).map(|raw| LinebreakHandle {
                raw,
                exhausted: false,
            }),
            status,
        )
    }

    /// The break positions of `text`.
    pub(crate) fn breaks<'t>(&'t mut self, text: &'t [u16]) -> IcuBreaks<'t> {
        let mut status = 0;
        // SAFETY: the iterator is live, and the returned iterator keeps `text`
        // borrowed for as long as ICU may read it.
        unsafe {
            ubrk_setText(
                self.raw.as_ptr(),
                text.as_ptr(),
                text.len() as i32,
                &mut status,
            )
        };
        self.exhausted = false;
        IcuBreaks {
            iter: self,
            _text: PhantomData,
        }
    }

    /// The next break of text that ran out earlier: always the end.
    pub(crate) fn next_after_end(&self) -> c_int {
        debug_assert!(self.exhausted, "an ICU iterator reused mid-text");
        -1
    }
}

/// The break positions of the text given to [`LinebreakHandle::breaks`].
pub(crate) struct IcuBreaks<'t> {
    iter: &'t mut LinebreakHandle,
    _text: PhantomData<&'t [u16]>,
}

impl IcuBreaks<'_> {
    /// The next break offset, or -1 at the end.
    pub(crate) fn next_break(&mut self) -> c_int {
        // SAFETY: the iterator is live and its text is still borrowed.
        let offs = unsafe { ubrk_next(self.iter.raw.as_ptr()) };
        if offs < 0 {
            self.iter.exhausted = true;
        }
        offs
    }
}

/// ICU's bidi analysis of one paragraph of text.
pub(crate) struct Bidi<'t> {
    raw: NonNull<c_void>,
    /// The status of `ubidi_setPara`, which later calls see as their input.
    status: std::cell::Cell<i32>,
    _text: PhantomData<&'t [u16]>,
}

impl Drop for Bidi<'_> {
    fn drop(&mut self) {
        // SAFETY: the bidi object is owned here.
        unsafe { ubidi_close(self.raw.as_ptr()) };
    }
}

pub(crate) const UBIDI_RTL: c_int = 1;
pub(crate) const UBIDI_MIXED: c_int = 2;

impl<'t> Bidi<'t> {
    /// Analyses `text` at paragraph level `level`.
    pub(crate) fn new(text: &'t [u16], level: u8) -> Bidi<'t> {
        // SAFETY: plain allocation.
        let raw = NonNull::new(unsafe { ubidi_open() }).expect("ubidi_open failed");
        let mut status = 0;
        // SAFETY: `raw` is live, and the object keeps `text` borrowed for as long
        // as ICU may read it.
        unsafe {
            ubidi_setPara(
                raw.as_ptr(),
                text.as_ptr(),
                text.len() as i32,
                level,
                ptr::null_mut(),
                &mut status,
            )
        };
        Bidi {
            raw,
            status: std::cell::Cell::new(status),
            _text: PhantomData,
        }
    }

    pub(crate) fn direction(&self) -> c_int {
        // SAFETY: `raw` is a live bidi object.
        unsafe { ubidi_getDirection(self.raw.as_ptr()) }
    }

    pub(crate) fn count_runs(&self) -> c_int {
        let mut status = self.status.get();
        // SAFETY: `raw` is a live bidi object and `status` a valid out-parameter.
        let runs = unsafe { ubidi_countRuns(self.raw.as_ptr(), &mut status) };
        self.status.set(status);
        runs
    }

    /// The logical start, length and direction of visual run `index`.
    pub(crate) fn visual_run(&self, index: c_int) -> (i32, i32, c_int) {
        let (mut start, mut length) = (0, 0);
        // SAFETY: `raw` is a live bidi object and the outputs are valid.
        let dir = unsafe { ubidi_getVisualRun(self.raw.as_ptr(), index, &mut start, &mut length) };
        (start, length, dir)
    }
}
