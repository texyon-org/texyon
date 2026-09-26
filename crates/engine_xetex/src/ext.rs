//! The native-font layer: OpenType/Graphite font loading and feature parsing,
//! TECkit mappings, shaping and measuring native word and glyph nodes, and the
//! extension (`\openout`, `\write`, `\special`, ...) commands.

use std::ffi::{CStr, CString};
use std::ptr;

use libc::{c_char, c_double, c_float, c_int, c_long, c_uint, c_void};

use crate::bridge::{input_get_size, input_open, input_read, TTBC_FILE_FORMAT_MISC_FONTS};
use crate::cmd::EXTENSION;
use crate::consts::{
    CS_TOKEN_FLAG, FROZEN_SPECIAL, GLYPH_NODE_SIZE, HMODE, INT_PAR_LEFT_HYPHEN_MIN,
    INT_PAR_RIGHT_HYPHEN_MIN, INT_PAR_XETEX_DEFAULT_INPUT_ENCODING,
    INT_PAR_XETEX_DEFAULT_INPUT_MODE, INT_PAR_XETEX_LINEBREAK_LOCALE, INT_PAR_XETEX_TRACING_FONTS,
    INT_PAR_XETEX_USE_GLYPH_METRICS, LEFT_SIDE, MMODE, OPEN_NODE_SIZE, RIGHT_SIDE, SMALL_NODE_SIZE,
    VMODE, WRITE_NODE_SIZE,
};
use crate::errors::Abort;
use crate::expand::{XETEX_FEATURE_NAME_CODE, XETEX_SELECTOR_NAME_CODE};
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::glyph_arena::{FixedPoint, GlyphBlob};
use crate::input::INSERTED;
use crate::io::{AUTO, ICUMAPPING, K_STATUS_NO_ERROR, RAW, UTF16BE, UTF16LE, UTF8};
use crate::main_control::new_graf;
use crate::mem::TEMP_HEAD;
use crate::node::{NodePtr, WhatsitNode, WhatsitSubtype};
use crate::output::{capture_to_diagnostic, error_here_with_diagnostic};
use crate::pic::load_picture;
use crate::scan::{
    get_x_token, scan_int, LEFT_BRACE_TOKEN, RIGHT_BRACE_TOKEN, XETEX_COUNT_FEATURES_CODE,
    XETEX_COUNT_GLYPHS_CODE, XETEX_COUNT_SELECTORS_CODE, XETEX_FEATURE_CODE_CODE,
    XETEX_FIND_FEATURE_BY_NAME_CODE, XETEX_FIND_SELECTOR_BY_NAME_CODE,
    XETEX_IS_DEFAULT_SELECTOR_CODE, XETEX_IS_EXCLUSIVE_FEATURE_CODE, XETEX_OT_COUNT_FEATURES_CODE,
    XETEX_OT_COUNT_LANGUAGES_CODE, XETEX_OT_COUNT_SCRIPTS_CODE, XETEX_OT_FEATURE_CODE,
    XETEX_OT_LANGUAGE_CODE, XETEX_OT_SCRIPT_CODE, XETEX_SELECTOR_CODE_CODE,
};
use crate::state::{EngineState, ErrorState, FontState, MemState};
use crate::units::{FontId, MemPtr, Scaled};

type XeTeXFont = *mut c_void;
type XeTeXLayoutEngine = *mut c_void;
type RawPlatformFontRef = *mut c_void;

const HB_TAG_NONE: u32 = 0;
#[cfg(target_endian = "big")]
const US_NATIVE_UTF16: c_int = UTF16BE;
#[cfg(target_endian = "little")]
const US_NATIVE_UTF16: c_int = UTF16LE;
const K_FORM_BYTES: u16 = 1;
#[cfg(target_endian = "big")]
const UTF16_NATIVE: u16 = 3;
#[cfg(target_endian = "little")]
const UTF16_NATIVE: u16 = 4;
const FONT_FLAGS_COLORED: i32 = 0x01;
const FONT_FLAGS_VERTICAL: i32 = 0x02;
const UBRK_LINE: c_int = 2;
const UBIDI_RTL: c_int = 1;
const UBIDI_MIXED: c_int = 2;
const XDV_FLAG_VERTICAL: u16 = 0x0100;
const XDV_FLAG_COLORED: u16 = 0x0200;
const XDV_FLAG_EXTEND: u16 = 0x1000;
const XDV_FLAG_SLANT: u16 = 0x2000;
const XDV_FLAG_EMBOLDEN: u16 = 0x4000;

extern "C" {
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
    fn hb_tag_from_string(str_: *const c_char, len: c_int) -> u32;

    fn deleteLayoutEngine(engine: XeTeXLayoutEngine);
    fn createLayoutEngine(
        font: XeTeXFont,
        script: u32,
        language: *mut c_char,
        features: *mut HbFeature,
        n_features: c_int,
        shapers: *mut *const c_char,
        rgb_value: u32,
        extend: c_float,
        slant: c_float,
        embolden: c_float,
    ) -> XeTeXLayoutEngine;
    fn createLayoutEngineBorrowed(
        font: XeTeXFont,
        script: u32,
        language: *mut c_char,
        features: *mut HbFeature,
        n_features: c_int,
        shapers: *const *const c_char,
        rgb_value: u32,
        extend: c_float,
        slant: c_float,
        embolden: c_float,
    ) -> XeTeXLayoutEngine;
    fn getFont(engine: XeTeXLayoutEngine) -> XeTeXFont;
    fn getExtendFactor(engine: XeTeXLayoutEngine) -> f32;
    fn getSlantFactor(engine: XeTeXLayoutEngine) -> f32;
    fn getAscentAndDescent(engine: XeTeXLayoutEngine, ascent: *mut f32, descent: *mut f32);
    fn getCapAndXHeight(engine: XeTeXLayoutEngine, capheight: *mut f32, xheight: *mut f32);
    fn getGlyphHeightDepth(
        engine: XeTeXLayoutEngine,
        glyph_id: u32,
        height: *mut f32,
        depth: *mut f32,
    );
    fn mapCharToGlyph(engine: XeTeXLayoutEngine, char_code: u32) -> u32;
    fn createFont(font_ref: RawPlatformFontRef, point_size: i32) -> XeTeXFont;
    fn createFontFromFile(filename: *const c_char, index: c_int, point_size: i32) -> XeTeXFont;
    fn deleteFont(font: XeTeXFont);
    fn findFontByName(name: *const c_char, var: *mut c_char, size: c_double) -> RawPlatformFontRef;
    fn getReqEngine() -> c_char;
    fn setReqEngine(engine: c_char);
    fn getFullName(font: RawPlatformFontRef) -> *const c_char;
    fn getDesignSize(font: XeTeXFont) -> c_double;
    fn set_loaded_font_design_size(val: i32);
    fn ttxl_platfont_get_desc(font: RawPlatformFontRef) -> *const c_char;
    fn setFontLayoutDir(font: XeTeXFont, vertical: c_int);
    fn getSlant(font: XeTeXFont) -> i32;
    fn countGlyphs(font: XeTeXFont) -> c_uint;
    fn countGraphiteFeatures(engine: XeTeXLayoutEngine) -> u32;
    fn countScripts(font: XeTeXFont) -> c_uint;
    fn countLanguages(font: XeTeXFont, script: u32) -> c_uint;
    fn getIndScript(font: XeTeXFont, index: c_uint) -> u32;
    fn getGraphiteFeatureCode(engine: XeTeXLayoutEngine, index: u32) -> u32;
    fn countGraphiteFeatureSettings(engine: XeTeXLayoutEngine, feature_id: u32) -> u32;
    fn getIndLanguage(font: XeTeXFont, script: u32, index: c_uint) -> u32;
    fn countFeatures(font: XeTeXFont, script: u32, language: u32) -> c_uint;
    fn getGraphiteFeatureSettingCode(
        engine: XeTeXLayoutEngine,
        feature_id: u32,
        setting_index: u32,
    ) -> u32;
    fn getGraphiteFeatureDefaultSetting(engine: XeTeXLayoutEngine, feature_id: u32) -> u32;
    fn getIndFeature(font: XeTeXFont, script: u32, language: u32, index: c_uint) -> u32;
    fn getGraphiteFeatureLabel(engine: XeTeXLayoutEngine, feature_id: u32) -> *mut c_char;
    fn getGraphiteFeatureSettingLabel(
        engine: XeTeXLayoutEngine,
        feature_id: u32,
        setting_index: u32,
    ) -> *mut c_char;
    fn gr_label_destroy(name: *mut c_void);
    fn findGraphiteFeatureNamed(
        engine: XeTeXLayoutEngine,
        name: *const c_char,
        namelength: c_int,
    ) -> c_long;
    fn findGraphiteFeatureSettingNamed(
        engine: XeTeXLayoutEngine,
        feature_id: u32,
        name: *const c_char,
        namelength: c_int,
    ) -> c_long;
    fn findGraphiteFeature(
        engine: XeTeXLayoutEngine,
        s: *const c_char,
        e: *const c_char,
        f: *mut u32,
        v: *mut c_int,
    ) -> bool;
    fn getGlyphItalCorr(engine: XeTeXLayoutEngine, glyph_id: u32) -> f32;
    fn getGlyphWidth(font: XeTeXFont, gid: u32) -> f32;
    fn getGlyphSidebearings(engine: XeTeXLayoutEngine, glyph_id: u32, lsb: *mut f32, rsb: *mut f32);
    fn getGlyphWidthFromEngine(engine: XeTeXLayoutEngine, glyph_id: u32) -> f32;
    fn mapGlyphToIndex(engine: XeTeXLayoutEngine, glyph_name: *const c_char) -> c_int;
    fn getFontCharRange(engine: XeTeXLayoutEngine, req_first: c_int) -> c_int;
    fn getGlyphName(font: XeTeXFont, gid: u16, len: *mut c_int) -> *const c_char;
    fn freeGlyphName(name: *mut c_char);
    fn layoutChars(
        engine: XeTeXLayoutEngine,
        chars: *mut u16,
        offset: i32,
        count: i32,
        max: i32,
        rtl: bool,
    ) -> c_int;
    fn getGlyphs(engine: XeTeXLayoutEngine, glyphs: *mut u32);
    fn getGlyphAdvances(engine: XeTeXLayoutEngine, advances: *mut f32);
    fn getGlyphPositions(engine: XeTeXLayoutEngine, positions: *mut FloatPoint);
    fn getDefaultDirection(engine: XeTeXLayoutEngine) -> c_int;
    fn getCachedGlyphBBox(font_id: u16, glyph_id: u16, bbox: *mut GlyphBBox) -> i32;
    fn cacheGlyphBBox(font_id: u16, glyph_id: u16, bbox: *const GlyphBBox);
    fn getGlyphBounds(engine: XeTeXLayoutEngine, glyph_id: u32, bbox: *mut GlyphBBox);
    fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;

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
    fn initGraphiteBreaking(engine: XeTeXLayoutEngine, txt_ptr: *const u16, txt_len: u32) -> bool;
    fn findNextGraphiteBreak(engine: XeTeXLayoutEngine) -> c_int;

    fn getFontFilename(engine: XeTeXLayoutEngine, index: *mut u32) -> *const c_char;
    fn freeFontFilename(filename: *const c_char);
    fn getRgbValue(engine: XeTeXLayoutEngine) -> u32;
    fn getEmboldenFactor(engine: XeTeXLayoutEngine) -> f32;
    fn getPointSize(engine: XeTeXLayoutEngine) -> f32;
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct FloatPoint {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Default)]
struct GlyphBBox {
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
}

#[repr(C)]
struct HbFeature {
    tag: u32,
    value: u32,
    start: u32,
    end: u32,
}

impl HbFeature {
    fn global(tag: u32, value: u32) -> HbFeature {
        HbFeature {
            tag,
            value,
            start: 0,
            end: u32::MAX,
        }
    }
}

/// Font-wide metrics of a native font, in scaled points.
#[derive(Default)]
pub(crate) struct NativeFontMetrics {
    pub ascent: i32,
    pub descent: i32,
    pub x_height: i32,
    pub cap_height: i32,
    pub slant: i32,
}

unsafe fn peek(ptr: *const c_char) -> u8 {
    *ptr as u8
}

/// The position just past `prefix` if `s` starts with it, else null.
unsafe fn str_starts_with(s: *const c_char, prefix: &[u8]) -> *const c_char {
    for (idx, byte) in prefix.iter().enumerate() {
        if peek(s.add(idx)) != *byte {
            return ptr::null();
        }
    }
    s.add(prefix.len())
}

/// For a `key=value` option: `None` if `s` does not start with `key`, else the
/// value, or `Err` when the `=` is missing.
unsafe fn option_value(s: *const c_char, key: &[u8]) -> Option<Result<*const c_char, ()>> {
    let sep = str_starts_with(s, key);
    if sep.is_null() {
        None
    } else if peek(sep) == b'=' {
        Some(Ok(sep.add(1)))
    } else {
        Some(Err(()))
    }
}

unsafe fn copy_c_slice(start: *const c_char, end: *const c_char) -> *mut c_char {
    let len = end.offset_from(start) as usize;
    let out = crate::mem::xmalloc((len + 1) as libc::size_t) as *mut c_char;
    ptr::copy_nonoverlapping(start, out, len);
    *out.add(len) = 0;
    out
}

#[no_mangle]
pub extern "C" fn D2Fix(d: c_double) -> i32 {
    (d * 65536.0 + 0.5) as i32
}

#[no_mangle]
pub extern "C" fn Fix2D(f: i32) -> c_double {
    f as c_double / 65536.0
}

#[no_mangle]
pub unsafe extern "C" fn read_double(s: *mut *const c_char) -> c_double {
    let mut neg = false;
    let mut val = 0.0;
    let mut cp = *s;

    while peek(cp) == b' ' || peek(cp) == b'\t' {
        cp = cp.add(1);
    }

    if peek(cp) == b'-' {
        neg = true;
        cp = cp.add(1);
    } else if peek(cp) == b'+' {
        cp = cp.add(1);
    }

    while peek(cp).is_ascii_digit() {
        val = val * 10.0 + (peek(cp) - b'0') as c_double;
        cp = cp.add(1);
    }

    if peek(cp) == b'.' {
        let mut dec = 10.0;
        cp = cp.add(1);
        while peek(cp).is_ascii_digit() {
            val += (peek(cp) - b'0') as c_double / dec;
            cp = cp.add(1);
            dec *= 10.0;
        }
    }

    *s = cp;
    if neg {
        -val
    } else {
        val
    }
}

unsafe fn read_hex_digit(cp: *mut *const c_char) -> Option<u32> {
    let value = (peek(*cp) as char).to_digit(16)?;
    *cp = (*cp).add(1);
    Some(value)
}

/// Parse `RRGGBB` with an optional `AA` (default opaque).
pub unsafe fn read_rgb_a(cp: *mut *const c_char) -> c_uint {
    let mut rgb_value = 0u32;
    for _ in 0..6 {
        let Some(value) = read_hex_digit(cp) else {
            return 0x000000FF;
        };
        rgb_value = (rgb_value << 4) + value;
    }

    let mut alpha = 0u32;
    let mut digits = 0;
    for _ in 0..2 {
        let Some(value) = read_hex_digit(cp) else {
            break;
        };
        alpha = (alpha << 4) + value;
        digits += 1;
    }
    (rgb_value << 8) + if digits == 2 { alpha } else { 0xFF }
}

/// The input mode named `name_of_file`, with the ICU converter name string
/// for `ICUMAPPING`.
fn get_encoding_mode_and_info(
    st: &mut EngineState,
    name_of_file: &CStr,
) -> Result<(c_int, i32), Abort> {
    let bytes = name_of_file.to_bytes();
    for (name, mode) in [
        ("auto", AUTO),
        ("utf8", UTF8),
        ("utf16", US_NATIVE_UTF16),
        ("utf16be", UTF16BE),
        ("utf16le", UTF16LE),
        ("bytes", RAW),
    ] {
        if bytes.eq_ignore_ascii_case(name.as_bytes()) {
            return Ok((mode, 0));
        }
    }

    if crate::io::icu_converter_exists(name_of_file).is_err() {
        crate::output::begin_diagnostic(st);
        crate::output::print_nl(st, b'U' as i32);
        crate::output::print(st, "nknown encoding `");
        crate::output::print(st, bytes);
        crate::output::print(st, "'; reading as raw bytes");
        crate::output::end_diagnostic(st, true);
        Ok((RAW, 0))
    } else {
        let info = st.strings.make_tex_string(bytes)?;
        Ok((ICUMAPPING, info))
    }
}

/// Parse an OpenType feature tag with an optional `=n` parameter.
unsafe fn read_tag_with_param(cp: *const c_char) -> (u32, c_int) {
    let mut cp2 = cp;
    while !matches!(peek(cp2), 0 | b':' | b';' | b',' | b'=') {
        cp2 = cp2.add(1);
    }

    let tag = hb_tag_from_string(cp, cp2.offset_from(cp) as c_int);

    let mut param: c_int = 0;
    let mut cp = cp2;
    if peek(cp) == b'=' {
        let mut neg = false;
        cp = cp.add(1);
        if peek(cp) == b'-' {
            neg = true;
            cp = cp.add(1);
        }
        while peek(cp).is_ascii_digit() {
            param = param * 10 + (peek(cp) - b'0') as c_int;
            cp = cp.add(1);
        }
        if neg {
            param = -param;
        }
    }
    (tag, param)
}

/// Parse a Graphite `feature=setting` pair given as numbers, spanning exactly
/// `s..e`.
#[allow(non_snake_case)]
pub unsafe fn readFeatureNumber(
    mut s: *const c_char,
    e: *const c_char,
    f: *mut u32,
    v: *mut c_int,
) -> bool {
    *f = 0;
    *v = 0;
    if !peek(s).is_ascii_digit() {
        return false;
    }
    while peek(s).is_ascii_digit() {
        *f = *f * 10 + (peek(s) - b'0') as u32;
        s = s.add(1);
    }
    while peek(s) == b' ' || peek(s) == b'\t' {
        s = s.add(1);
    }
    if peek(s) != b'=' {
        return false;
    }
    s = s.add(1);

    if !peek(s).is_ascii_digit() {
        return false;
    }
    while peek(s).is_ascii_digit() {
        *v = *v * 10 + (peek(s) - b'0') as c_int;
        s = s.add(1);
    }
    while peek(s) == b' ' || peek(s) == b'\t' {
        s = s.add(1);
    }
    s == e
}

/// Load TECkit mapping `s..e` (plus `.tec`), mapping to bytes for TFM fonts or
/// to UTF-16 for native ones.
unsafe fn load_mapping_file(
    st: &mut EngineState,
    s: *const c_char,
    e: *const c_char,
    byte_mapping: bool,
) -> Result<Option<std::sync::Arc<crate::state::FontMappingHandle>>, Abort> {
    let source = std::slice::from_raw_parts(s as *const u8, e.offset_from(s) as usize);
    let file_name = [source, b".tec"].concat();
    let name_len = file_name
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(file_name.len());
    let name = &file_name[..name_len];

    let mut cnv: *mut c_void = ptr::null_mut();
    let map = input_open(name, TTBC_FILE_FORMAT_MISC_FONTS, false);
    if map == 0 {
        crate::output::font_mapping_warning(st, name, 1);
        return Ok(None);
    }

    let mapping_size = input_get_size(map);
    let mut mapping = vec![0u8; mapping_size];
    let read = input_read(map, &mut mapping);
    if read < 0 || read as libc::size_t != mapping_size {
        return Err(Abort::Internal("could not read mapping file"));
    }
    if crate::io::input_close(map) {
        return Err(Abort::InputClose);
    }

    let (map_forward, target_form) = if byte_mapping {
        (0, K_FORM_BYTES)
    } else {
        (1, UTF16_NATIVE)
    };
    TECkit_CreateConverter(
        mapping.as_ptr(),
        mapping_size as u32,
        map_forward,
        UTF16_NATIVE,
        target_form,
        &mut cnv,
    );

    if cnv.is_null() {
        crate::output::font_mapping_warning(st, name, 2);
    } else if st.eqtb.int_par(INT_PAR_XETEX_TRACING_FONTS) > 1 {
        crate::output::font_mapping_warning(st, name, 0);
    }

    Ok((!cnv.is_null()).then(|| std::sync::Arc::new(crate::state::FontMappingHandle { raw: cnv })))
}

impl FontState {
    /// Cut a `:mapping=` suffix off `name_of_file`, saving the mapping name.
    pub(crate) fn check_for_tfm_font_mapping(&mut self, name_of_file: &mut [u8]) {
        const KEY: &[u8] = b":mapping=";
        self.saved_mapping_name = None;
        let len = name_of_file.iter().position(|&b| b == 0).unwrap_or(0);
        let Some(at) = name_of_file[..len]
            .windows(KEY.len())
            .position(|w| w == KEY)
        else {
            return;
        };
        name_of_file[at] = 0;
        // C's signed `char` also skips bytes 0x80 and up.
        let value = &name_of_file[at + KEY.len()..len];
        let start = value.iter().position(|&b| b as i8 > b' ' as i8);
        if let Some(start) = start {
            self.saved_mapping_name = Some(CString::new(&value[start..]).unwrap_or_default());
        }
    }
}

pub(crate) unsafe fn load_tfm_font_mapping(
    st: &mut EngineState,
) -> Result<Option<std::sync::Arc<crate::state::FontMappingHandle>>, Abort> {
    let Some(name) = st.fonts.saved_mapping_name.take() else {
        return Ok(None);
    };
    let start = name.as_ptr();
    load_mapping_file(st, start, start.add(name.as_bytes().len()), true)
}

pub unsafe fn apply_tfm_font_mapping(cnv: *mut c_void, c: c_int) -> c_int {
    let input = c as u16;
    let mut out = [0u8; 2];
    let mut in_used = 0u32;
    let mut out_used = 0u32;

    TECkit_ConvertBuffer(
        cnv,
        &input as *const u16 as *const u8,
        std::mem::size_of::<u16>() as u32,
        &mut in_used,
        out.as_mut_ptr(),
        out.len() as u32,
        &mut out_used,
        1,
    );
    TECkit_ResetConverter(cnv);

    if out_used < 1 {
        0
    } else {
        out[0] as c_int
    }
}

struct CommonFeatures {
    extend: c_float,
    slant: c_float,
    embolden: c_float,
    letterspace: c_float,
    rgb_value: u32,
}

/// Parse one common option from `feat..end`: `None` if it is not one, else
/// whether it was well formed.
unsafe fn read_common_feature(
    st: &mut EngineState,
    feat: *const c_char,
    end: *const c_char,
    opts: &mut CommonFeatures,
) -> Result<Option<bool>, Abort> {
    if let Some(value) = option_value(feat, b"mapping") {
        let Ok(value) = value else {
            return Ok(Some(false));
        };
        st.fonts.loaded_font_mapping = load_mapping_file(st, value, end, false)?;
        return Ok(Some(true));
    }

    for (key, slot) in [
        (&b"extend"[..], &mut opts.extend),
        (b"slant", &mut opts.slant),
        (b"embolden", &mut opts.embolden),
        (b"letterspace", &mut opts.letterspace),
    ] {
        if let Some(value) = option_value(feat, key) {
            let Ok(mut value) = value else {
                return Ok(Some(false));
            };
            *slot = read_double(&mut value) as c_float;
            return Ok(Some(true));
        }
    }

    if let Some(value) = option_value(feat, b"color") {
        let Ok(mut value) = value else {
            return Ok(Some(false));
        };
        let start = value;
        opts.rgb_value = read_rgb_a(&mut value);
        let consumed = value.offset_from(start);
        if consumed != 6 && consumed != 8 {
            return Ok(Some(false));
        }
        st.fonts.loaded_font_flags |= FONT_FLAGS_COLORED as c_char;
        return Ok(Some(true));
    }

    Ok(None)
}

/// Build the OpenType/Graphite layout engine for `font` with the options in
/// the feature string `cp1`.
unsafe fn load_ot_font(
    st: &mut EngineState,
    font: XeTeXFont,
    scaled_size: i32,
    mut cp1: *mut c_char,
) -> Result<Option<crate::state::FontEngineHandle>, Abort> {
    let mut script = HB_TAG_NONE;
    let mut language: *mut c_char = ptr::null_mut();
    let mut features: Vec<HbFeature> = Vec::new();
    let mut shapers: Vec<*const c_char> = Vec::new();
    let mut opts = CommonFeatures {
        extend: 1.0,
        slant: 0.0,
        embolden: 0.0,
        letterspace: 0.0,
        rgb_value: 0x000000FF,
    };
    let req_engine = getReqEngine() as u8;

    match req_engine {
        b'O' => shapers.push(c"ot".as_ptr()),
        b'G' => shapers.push(c"graphite2".as_ptr()),
        _ => {}
    }

    // Graphite feature names are resolved against a probe engine.
    let mut probe: XeTeXLayoutEngine = ptr::null_mut();
    if req_engine == b'G' {
        let graphite_shaper = [c"graphite2".as_ptr(), ptr::null()];
        probe = createLayoutEngineBorrowed(
            font,
            script,
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            graphite_shaper.as_ptr(),
            opts.rgb_value,
            opts.extend,
            opts.slant,
            opts.embolden,
        );
        if probe.is_null() {
            return Ok(None);
        }
    }

    if !cp1.is_null() {
        while *cp1 != 0 {
            if matches!(peek(cp1), b':' | b';' | b',') {
                cp1 = cp1.add(1);
            }
            while matches!(peek(cp1), b' ' | b'\t') {
                cp1 = cp1.add(1);
            }
            if *cp1 == 0 {
                break;
            }

            let mut cp2 = cp1;
            while !matches!(peek(cp2), 0 | b':' | b';' | b',') {
                cp2 = cp2.add(1);
            }

            let parsed = 'option: {
                if let Some(value) = option_value(cp1, b"script") {
                    let Ok(value) = value else {
                        break 'option Some(false);
                    };
                    script = hb_tag_from_string(value, cp2.offset_from(value) as c_int);
                    break 'option Some(true);
                }
                if let Some(value) = option_value(cp1, b"language") {
                    let Ok(value) = value else {
                        break 'option Some(false);
                    };
                    language = copy_c_slice(value, cp2);
                    break 'option Some(true);
                }
                if let Some(value) = option_value(cp1, b"shaper") {
                    let Ok(value) = value else {
                        break 'option Some(false);
                    };
                    // Leaked on purpose: the layout engine keeps the pointer.
                    shapers.push(copy_c_slice(value, cp2));
                    break 'option Some(true);
                }
                if let Some(valid) = read_common_feature(st, cp1, cp2, &mut opts)? {
                    break 'option Some(valid);
                }
                if req_engine == b'G' {
                    let mut value = 0;
                    let mut tag = 0;
                    if readFeatureNumber(cp1, cp2, &mut tag, &mut value)
                        || findGraphiteFeature(probe, cp1, cp2, &mut tag, &mut value)
                    {
                        features.push(HbFeature::global(tag, value as u32));
                        break 'option Some(true);
                    }
                }
                if peek(cp1) == b'+' {
                    let (tag, mut param) = read_tag_with_param(cp1.add(1));
                    if param >= 0 {
                        param += 1;
                    }
                    features.push(HbFeature::global(tag, param as u32));
                    break 'option Some(true);
                }
                if peek(cp1) == b'-' {
                    let name = cp1.add(1);
                    let tag = hb_tag_from_string(name, cp2.offset_from(name) as c_int);
                    features.push(HbFeature::global(tag, 0));
                    break 'option Some(true);
                }
                if !str_starts_with(cp1, b"vertical").is_null() {
                    let mut cp3 = cp2 as *const c_char;
                    if matches!(peek(cp3), b';' | b':' | b',') {
                        cp3 = cp3.sub(1);
                    }
                    while matches!(peek(cp3), 0 | b' ' | b'\t') {
                        cp3 = cp3.sub(1);
                    }
                    if *cp3 != 0 {
                        cp3 = cp3.add(1);
                    }
                    if cp3 == cp1.add(8) {
                        st.fonts.loaded_font_flags |= FONT_FLAGS_VERTICAL as c_char;
                        break 'option Some(true);
                    }
                }
                None
            };

            if parsed != Some(true) {
                let feature =
                    std::slice::from_raw_parts(cp1 as *const u8, cp2.offset_from(cp1) as usize);
                crate::output::font_feature_warning(st, feature, &[]);
            }
            cp1 = cp2;
        }
    }

    if !shapers.is_empty() {
        shapers.push(ptr::null());
    }
    if !probe.is_null() {
        deleteLayoutEngine(probe);
    }

    if opts.embolden != 0.0 {
        opts.embolden = (opts.embolden as c_double * Fix2D(scaled_size) / 100.0) as c_float;
    }
    if opts.letterspace != 0.0 {
        st.fonts.loaded_font_letter_space =
            (opts.letterspace as c_double / 100.0 * scaled_size as c_double) as i32;
    }
    if (st.fonts.loaded_font_flags as i32 & FONT_FLAGS_COLORED) == 0 {
        opts.rgb_value = 0x000000FF;
    }
    if (st.fonts.loaded_font_flags as i32 & FONT_FLAGS_VERTICAL) != 0 {
        setFontLayoutDir(font, 1);
    }

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
    let engine = createLayoutEngine(
        font,
        script,
        language,
        features_ptr,
        features.len() as c_int,
        shapers_ptr,
        opts.rgb_value,
        opts.extend,
        opts.slant,
        opts.embolden,
    );
    if !language.is_null() {
        libc::free(language as *mut c_void);
    }

    if engine.is_null() {
        return Ok(None);
    }
    st.fonts.native_font_type_flag = OTGR_FONT_FLAG;
    Ok(Some(crate::state::FontEngineHandle {
        raw: engine,
        area: OTGR_FONT_FLAG,
    }))
}

impl ErrorState {
    /// Resolve a negative (`scaled`) font size against `font`'s design size, then
    /// delete `font`.
    unsafe fn scale_by_design_size(&mut self, font: XeTeXFont, scaled_size: i32) -> i32 {
        if font.is_null() {
            return scaled_size;
        }
        let dsize = D2Fix(getDesignSize(font));
        let size = if scaled_size == -1000 {
            dsize
        } else {
            self.xn_over_d(dsize, -scaled_size, 1000)
        };
        deleteFont(font);
        size
    }
}

/// Look up native font `name` (a `[file]` path or an installed font name with
/// `/variant` and `:features`) and build its layout engine.
pub(crate) unsafe fn find_native_font(
    st: &mut EngineState,
    name: *mut c_char,
    mut scaled_size: i32,
) -> Result<Option<crate::state::FontEngineHandle>, Abort> {
    let mut rval = None;
    let mut var: *mut c_char = ptr::null_mut();
    let mut feat: *mut c_char = ptr::null_mut();
    let mut end: *mut c_char = ptr::null_mut();
    let mut var_string: *mut c_char = ptr::null_mut();
    let mut feat_string: *mut c_char = ptr::null_mut();
    let mut index = 0;

    st.fonts.loaded_font_mapping = None;
    st.fonts.loaded_font_flags = 0;
    st.fonts.loaded_font_letter_space = 0;

    splitFontName(name, &mut var, &mut feat, &mut end, &mut index);
    let name_string = copy_c_slice(name, var);
    if feat > var {
        var_string = copy_c_slice(var.add(1), feat);
    }
    if end > feat {
        feat_string = copy_c_slice(feat.add(1), end);
    }

    if *name_string == b'[' as c_char {
        let path = name_string.add(1);
        if scaled_size < 0 {
            let font = createFontFromFile(path, index, 655360);
            scaled_size = st.errors.scale_by_design_size(font, scaled_size);
        }
        let font = createFontFromFile(path, index, scaled_size);
        if !font.is_null() {
            set_loaded_font_design_size(D2Fix(getDesignSize(font)));

            setReqEngine(0);
            if !var_string.is_null() {
                if !str_starts_with(var_string, b"/AAT").is_null() {
                    setReqEngine(b'A' as c_char);
                } else if !str_starts_with(var_string, b"/OT").is_null()
                    || !str_starts_with(var_string, b"/ICU").is_null()
                {
                    setReqEngine(b'O' as c_char);
                } else if !str_starts_with(var_string, b"/GR").is_null() {
                    setReqEngine(b'G' as c_char);
                }
            }

            rval = load_ot_font(st, font, scaled_size, feat_string)?;
            if rval.is_none() {
                deleteFont(font);
            }
            if rval.is_some() && st.eqtb.int_par(INT_PAR_XETEX_TRACING_FONTS) > 0 {
                crate::output::begin_diagnostic(st);
                crate::output::print_nl(st, b' ' as i32);
                crate::output::print_raw_bytes(st, b"-> ");
                crate::output::print_raw_bytes(st, CStr::from_ptr(path).to_bytes());
                crate::output::end_diagnostic(st, false);
            }
        }
    } else {
        let font_ref = findFontByName(name_string, var_string, Fix2D(scaled_size));
        if !font_ref.is_null() {
            if st.eqtb.int_par(INT_PAR_XETEX_TRACING_FONTS) > 0 {
                crate::output::begin_diagnostic(st);
                crate::output::print_nl(st, b' ' as i32);
                crate::output::print_raw_bytes(st, b"-> ");
                crate::output::print_raw_bytes(
                    st,
                    CStr::from_ptr(ttxl_platfont_get_desc(font_ref)).to_bytes(),
                );
                crate::output::end_diagnostic(st, false);
            }

            let mut name = CStr::from_ptr(getFullName(font_ref)).to_bytes().to_vec();
            st.input.name_of_file.clear();
            st.input.name_of_file.extend_from_slice(&name);
            st.input.name_of_file.push(0);

            if scaled_size < 0 {
                let font = createFont(font_ref, scaled_size);
                scaled_size = st.errors.scale_by_design_size(font, scaled_size);
            }

            let font = createFont(font_ref, scaled_size);
            if !font.is_null() {
                rval = load_ot_font(st, font, scaled_size, feat_string)?;
                if rval.is_none() {
                    deleteFont(font);
                }
            }

            if !var_string.is_null() && *var_string != 0 {
                name.push(b'/');
                name.extend_from_slice(CStr::from_ptr(var_string).to_bytes());
            }
            if !feat_string.is_null() && *feat_string != 0 {
                name.push(b':');
                name.extend_from_slice(CStr::from_ptr(feat_string).to_bytes());
            }
            st.input.name_length = name.len() as i32;
            name.push(0);
            st.input.name_of_file = name;
        }
    }

    libc::free(var_string as *mut c_void);
    libc::free(feat_string as *mut c_void);
    libc::free(name_string as *mut c_void);

    Ok(rval)
}

/// Split a native font spec into its name, `/variant` and `:features` parts.
/// `[file:index]` names carry a face index.
#[allow(non_snake_case)]
pub unsafe fn splitFontName(
    mut name: *mut c_char,
    var: *mut *mut c_char,
    feat: *mut *mut c_char,
    end: *mut *mut c_char,
    index: *mut c_int,
) {
    *var = ptr::null_mut();
    *feat = ptr::null_mut();
    *index = 0;

    if *name == b'[' as c_char {
        let mut within_file_name = true;
        name = name.add(1);
        while *name != 0 {
            if within_file_name && *name == b']' as c_char {
                within_file_name = false;
                if (*var).is_null() {
                    *var = name;
                }
            } else if *name == b':' as c_char {
                if within_file_name && (*var).is_null() {
                    *var = name;
                    name = name.add(1);
                    while (*name as u8).is_ascii_digit() {
                        *index = *index * 10 + (*name - b'0' as c_char) as c_int;
                        name = name.add(1);
                    }
                    name = name.sub(1);
                } else if !within_file_name && (*feat).is_null() {
                    *feat = name;
                }
            }
            name = name.add(1);
        }
    } else {
        while *name != 0 {
            if *name == b'/' as c_char && (*var).is_null() && (*feat).is_null() {
                *var = name;
            } else if *name == b':' as c_char && (*feat).is_null() {
                *feat = name;
            }
            name = name.add(1);
        }
    }
    *end = name;

    if (*feat).is_null() {
        *feat = name;
    }
    if (*var).is_null() {
        *var = *feat;
    }
}

pub(crate) unsafe fn dispose_font_mapping(cnv: *mut c_void) {
    TECkit_DisposeConverter(cnv);
}

pub unsafe fn release_font_engine(engine: *mut c_void, type_flag: c_int) {
    if type_flag == OTGR_FONT_FLAG {
        deleteLayoutEngine(engine as XeTeXLayoutEngine);
    }
}

pub unsafe fn ot_get_font_metrics(engine: *mut c_void) -> NativeFontMetrics {
    let engine = engine as XeTeXLayoutEngine;
    let mut a = 0.0f32;
    let mut d = 0.0f32;

    getAscentAndDescent(engine, &mut a, &mut d);
    let ascent = D2Fix(a as c_double);
    let descent = D2Fix(d as c_double);

    let slant = D2Fix(
        Fix2D(getSlant(getFont(engine))) * getExtendFactor(engine) as c_double
            + getSlantFactor(engine) as c_double,
    );

    getCapAndXHeight(engine, &mut a, &mut d);
    let mut cap_height = D2Fix(a as c_double);
    let mut x_height = D2Fix(d as c_double);

    // Fall back to the height of `x` and `X`, else to fractions of the ascent.
    if x_height == 0 {
        let glyph_id = mapCharToGlyph(engine, b'x' as u32);
        x_height = if glyph_id != 0 {
            getGlyphHeightDepth(engine, glyph_id, &mut a, &mut d);
            D2Fix(a as c_double)
        } else {
            ascent / 2
        };
    }
    if cap_height == 0 {
        let glyph_id = mapCharToGlyph(engine, b'X' as u32);
        cap_height = if glyph_id != 0 {
            getGlyphHeightDepth(engine, glyph_id, &mut a, &mut d);
            D2Fix(a as c_double)
        } else {
            ascent
        };
    }

    NativeFontMetrics {
        ascent,
        descent,
        x_height,
        cap_height,
        slant,
    }
}

pub unsafe fn ot_font_get(what: i32, engine: *mut c_void) -> i32 {
    let font = getFont(engine);
    match what {
        XETEX_COUNT_GLYPHS_CODE => countGlyphs(font) as i32,
        XETEX_COUNT_FEATURES_CODE => countGraphiteFeatures(engine) as i32,
        XETEX_OT_COUNT_SCRIPTS_CODE => countScripts(font) as i32,
        _ => 0,
    }
}

pub unsafe fn ot_font_get_1(what: i32, engine: *mut c_void, param: i32) -> i32 {
    let font = getFont(engine);
    match what {
        XETEX_OT_COUNT_LANGUAGES_CODE => countLanguages(font, param as u32) as i32,
        XETEX_OT_SCRIPT_CODE => getIndScript(font, param as c_uint) as i32,
        XETEX_FEATURE_CODE_CODE => getGraphiteFeatureCode(engine, param as u32) as i32,
        XETEX_IS_EXCLUSIVE_FEATURE_CODE => 1,
        XETEX_COUNT_SELECTORS_CODE => countGraphiteFeatureSettings(engine, param as u32) as i32,
        _ => 0,
    }
}

pub unsafe fn ot_font_get_2(what: i32, engine: *mut c_void, param1: i32, param2: i32) -> i32 {
    let font = getFont(engine);
    match what {
        XETEX_OT_LANGUAGE_CODE => getIndLanguage(font, param1 as u32, param2 as c_uint) as i32,
        XETEX_OT_COUNT_FEATURES_CODE => countFeatures(font, param1 as u32, param2 as u32) as i32,
        XETEX_SELECTOR_CODE_CODE => {
            getGraphiteFeatureSettingCode(engine, param1 as u32, param2 as u32) as i32
        }
        XETEX_IS_DEFAULT_SELECTOR_CODE => {
            (getGraphiteFeatureDefaultSetting(engine, param1 as u32) == param2 as u32) as i32
        }
        _ => 0,
    }
}

pub unsafe fn ot_font_get_3(
    what: i32,
    engine: *mut c_void,
    param1: i32,
    param2: i32,
    param3: i32,
) -> i32 {
    let font = getFont(engine);
    match what {
        XETEX_OT_FEATURE_CODE => {
            getIndFeature(font, param1 as u32, param2 as u32, param3 as c_uint) as i32
        }
        _ => 0,
    }
}

pub(crate) unsafe fn gr_print_font_name(
    st: &mut EngineState,
    what: i32,
    engine: *mut c_void,
    param1: i32,
    param2: i32,
) {
    let name = match what {
        XETEX_FEATURE_NAME_CODE => getGraphiteFeatureLabel(engine, param1 as u32),
        XETEX_SELECTOR_NAME_CODE => {
            getGraphiteFeatureSettingLabel(engine, param1 as u32, param2 as u32)
        }
        _ => ptr::null_mut(),
    };
    if !name.is_null() {
        crate::output::print(st, CStr::from_ptr(name).to_bytes());
        gr_label_destroy(name as *mut c_void);
    }
}

pub unsafe fn gr_font_get_named(st: &mut EngineState, what: i32, engine: *mut c_void) -> i32 {
    if what != XETEX_FIND_FEATURE_BY_NAME_CODE {
        return -1;
    }
    let name = crate::fonts::name_of_file_ptr(st);
    findGraphiteFeatureNamed(engine, name, st.input.name_length) as i32
}

pub unsafe fn gr_font_get_named_1(
    st: &mut EngineState,
    what: i32,
    engine: *mut c_void,
    param: i32,
) -> i32 {
    if what != XETEX_FIND_SELECTOR_BY_NAME_CODE {
        return -1;
    }
    let name = crate::fonts::name_of_file_ptr(st);
    findGraphiteFeatureSettingNamed(engine, param as u32, name, st.input.name_length) as i32
}

#[derive(Default)]
struct ShapedGlyphs {
    locations: Vec<FixedPoint>,
    gids: Vec<u16>,
    advances: Vec<i32>,
}

impl ShapedGlyphs {
    /// Append the `n_glyphs` glyphs of the engine's last layout, offset by
    /// `(x, y)`. Returns the pen position after them.
    unsafe fn append_run(
        &mut self,
        engine: XeTeXLayoutEngine,
        n_glyphs: i32,
        x: f64,
        y: f64,
    ) -> FloatPoint {
        let n = n_glyphs as usize;
        let mut glyphs = vec![0u32; n];
        let mut advances = vec![0f32; n];
        let mut positions = vec![FloatPoint::default(); n + 1];
        getGlyphs(engine, glyphs.as_mut_ptr());
        getGlyphAdvances(engine, advances.as_mut_ptr());
        getGlyphPositions(engine, positions.as_mut_ptr());

        self.gids.extend(glyphs.iter().map(|&g| g as u16));
        self.locations
            .extend(positions[..n].iter().map(|pos| FixedPoint {
                x: D2Fix(pos.x as c_double + x),
                y: D2Fix(pos.y as c_double + y),
            }));
        self.advances
            .extend(advances.iter().map(|&a| D2Fix(a as c_double)));
        positions[n]
    }
}

unsafe fn layout_visual_run(
    bidi: *mut c_void,
    engine: XeTeXLayoutEngine,
    text: *mut u16,
    text_len: i32,
    run_index: i32,
) -> i32 {
    let mut logical_start = 0i32;
    let mut length = 0i32;
    let dir = ubidi_getVisualRun(bidi, run_index, &mut logical_start, &mut length);
    layoutChars(
        engine,
        text,
        logical_start,
        length,
        text_len,
        dir == UBIDI_RTL,
    )
}

unsafe fn cached_glyph_bbox(engine: XeTeXLayoutEngine, f: i32, gid: u16) -> GlyphBBox {
    let mut bbox = GlyphBBox::default();
    if getCachedGlyphBBox(f as u16, gid, &mut bbox) == 0 {
        getGlyphBounds(engine, gid as u32, &mut bbox);
        cacheGlyphBBox(f as u16, gid, &bbox);
    }
    bbox
}

impl MemState {
    pub fn real_get_native_glyph(&self, w: NodePtr<WhatsitNode>, index: u32) -> u16 {
        if index as usize >= w.native_glyph_count(self) as usize {
            0
        } else {
            self.glyph_blobs.get(w.native_glyph_info_handle(self)).gids[index as usize]
        }
    }

    /// The protrusion code of the first or last glyph of native word `w`.
    pub unsafe fn real_get_native_word_cp(&self, w: NodePtr<WhatsitNode>, side: c_int) -> i32 {
        let glyph_count = w.native_glyph_count(self) as usize;
        let f = w.native_font(self).0;
        if glyph_count == 0 {
            return 0;
        }
        let gids = &self.glyph_blobs.get(w.native_glyph_info_handle(self)).gids;
        let actual_glyph = match side {
            LEFT_SIDE => gids[0],
            RIGHT_SIDE => gids[glyph_count - 1],
            _ => unreachable!(),
        };
        get_cp_code(f, actual_glyph as u32, side)
    }
}

fn snap_zone(value: &mut i32, snap_value: i32, fuzz: i32) {
    let difference = *value - snap_value;
    if difference <= fuzz && difference >= -fuzz {
        *value = snap_value;
    }
}

impl FontState {
    /// The layout engine of OpenType/Graphite font `f`, or `BadFlag(msg)` for any
    /// other font.
    fn require_ot_engine(&self, f: i32, msg: &'static str) -> Result<XeTeXLayoutEngine, Abort> {
        if self.font_area_entry(f) == OTGR_FONT_FLAG {
            Ok(self.font_layout_engine_ptr(f) as XeTeXLayoutEngine)
        } else {
            Err(Abort::Internal(msg))
        }
    }

    /// Shape native word `w` and store its width, height, depth and glyphs.
    pub unsafe fn measure_native_node(
        &self,
        mem: &mut MemState,
        w: NodePtr<WhatsitNode>,
        use_glyph_metrics: bool,
    ) -> Result<(), Abort> {
        let text_len = w.native_length(mem) as i32;
        // The shaper only reads the text.
        let text = w.native_text_ptr(mem).cast_mut();
        let f = w.native_font(mem).0;
        let engine = self.require_ot_engine(f, "bad native font flag in `measure_native_node`")?;

        let bidi = ubidi_open();
        let mut error_code = 0i32;
        ubidi_setPara(
            bidi,
            text,
            text_len,
            getDefaultDirection(engine) as u8,
            ptr::null_mut(),
            &mut error_code,
        );

        let mut shaped = ShapedGlyphs::default();
        let dir = ubidi_getDirection(bidi);
        let width = if dir == UBIDI_MIXED {
            let n_runs = ubidi_countRuns(bidi, &mut error_code);
            let mut total = 0;
            for run in 0..n_runs {
                total += layout_visual_run(bidi, engine, text, text_len, run);
            }
            let (mut x, mut y) = (0.0f64, 0.0f64);
            if total > 0 {
                for run in 0..n_runs {
                    let n_glyphs = layout_visual_run(bidi, engine, text, text_len, run);
                    let end = shaped.append_run(engine, n_glyphs, x, y);
                    x += end.x as f64;
                    y += end.y as f64;
                }
            }
            x
        } else {
            let n_glyphs = layoutChars(engine, text, 0, text_len, text_len, dir == UBIDI_RTL);
            let end = shaped.append_run(engine, n_glyphs, 0.0, 0.0);
            if n_glyphs > 0 {
                end.x as f64
            } else {
                0.0
            }
        };
        w.set_width(mem, Scaled(D2Fix(width)));
        w.set_native_glyph_count(mem, shaped.gids.len() as u16);
        ubidi_close(bidi);

        let letter_space = self.font_letter_space_entry(f);
        if letter_space != 0 {
            let mut ls_delta = 0i32;
            for (loc, &advance) in shaped.locations.iter_mut().zip(&shaped.advances) {
                if advance == 0 && ls_delta != 0 {
                    ls_delta -= letter_space;
                }
                loc.x += ls_delta;
                ls_delta += letter_space;
            }
            if ls_delta != 0 {
                ls_delta -= letter_space;
                w.set_width(mem, Scaled(w.width(mem).0 + ls_delta));
            }
        }

        let glyph_count = w.native_glyph_count(mem) as usize;
        if !use_glyph_metrics || glyph_count == 0 {
            w.set_height(mem, Scaled(self.height_base_entry(f)));
            w.set_depth(mem, Scaled(self.depth_base_entry(f)));
        } else {
            let mut y_min = 65536.0f32;
            let mut y_max = -65536.0f32;
            for (loc, &gid) in shaped.locations[..glyph_count].iter().zip(&shaped.gids) {
                let y = Fix2D(-loc.y) as f32;
                let bbox = cached_glyph_bbox(engine, f, gid);
                let ht = bbox.y_max;
                let dp = -bbox.y_min;
                if y + ht > y_max {
                    y_max = y + ht;
                }
                if y - dp < y_min {
                    y_min = y - dp;
                }
            }
            w.set_height(mem, Scaled(D2Fix(y_max as c_double)));
            w.set_depth(mem, Scaled(-D2Fix(y_min as c_double)));
        }

        let old_handle = w.native_glyph_info_handle(mem);
        if old_handle != 0 {
            mem.glyph_blobs.release(old_handle);
        }
        let handle = if glyph_count == 0 {
            0
        } else {
            shaped.locations.truncate(glyph_count);
            shaped.gids.truncate(glyph_count);
            mem.glyph_blobs.insert(GlyphBlob {
                locations: shaped.locations,
                gids: shaped.gids,
            })
        };
        w.set_native_glyph_info_handle(mem, handle);
        Ok(())
    }

    pub unsafe fn real_get_native_italic_correction(
        &self,
        mem: &MemState,
        w: NodePtr<WhatsitNode>,
    ) -> i32 {
        let f = w.native_font(mem).0;
        let n = w.native_glyph_count(mem) as usize;
        if n > 0 {
            let last_glyph = mem.glyph_blobs.get(w.native_glyph_info_handle(mem)).gids[n - 1];
            if self.font_area_entry(f) == OTGR_FONT_FLAG {
                let engine = self.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
                return D2Fix(getGlyphItalCorr(engine, last_glyph as u32) as c_double)
                    + self.font_letter_space_entry(f);
            }
        }
        0
    }

    pub unsafe fn real_get_native_glyph_italic_correction(
        &self,
        mem: &MemState,
        w: NodePtr<WhatsitNode>,
    ) -> i32 {
        let gid = w.native_glyph(mem);
        let f = w.native_font(mem).0;
        if self.font_area_entry(f) == OTGR_FONT_FLAG {
            let engine = self.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
            return D2Fix(getGlyphItalCorr(engine, gid as u32) as c_double);
        }
        0
    }

    pub unsafe fn measure_native_glyph(
        &self,
        mem: &mut MemState,
        w: NodePtr<WhatsitNode>,
        use_glyph_metrics: bool,
    ) -> Result<(), Abort> {
        let gid = w.native_glyph(mem) as u32;
        let f = w.native_font(mem).0;
        let engine = self.require_ot_engine(f, "bad native font flag in `measure_native_glyph`")?;
        w.set_width(
            mem,
            Scaled(D2Fix(getGlyphWidth(getFont(engine), gid) as c_double)),
        );
        if use_glyph_metrics {
            let (mut ht, mut dp) = (0.0f32, 0.0f32);
            getGlyphHeightDepth(engine, gid, &mut ht, &mut dp);
            w.set_height(mem, Scaled(D2Fix(ht as c_double)));
            w.set_depth(mem, Scaled(D2Fix(dp as c_double)));
        } else {
            w.set_height(mem, Scaled(self.height_base_entry(f)));
            w.set_depth(mem, Scaled(self.depth_base_entry(f)));
        }
        Ok(())
    }

    /// Height and depth of character `ch` of native font `font`.
    pub unsafe fn get_native_char_height_depth(
        &self,
        font: i32,
        ch: i32,
    ) -> Result<(i32, i32), Abort> {
        let engine = self.require_ot_engine(
            font,
            "bad native font flag in `get_native_char_height_depth`",
        )?;
        let (mut ht, mut dp) = (0.0f32, 0.0f32);
        let gid = mapCharToGlyph(engine, ch as u32);
        getGlyphHeightDepth(engine, gid, &mut ht, &mut dp);
        let mut height = D2Fix(ht as c_double);
        let mut depth = D2Fix(dp as c_double);

        // Snap to the baseline, x-height (param 5) and cap height (param 8) when
        // within 4% of the quad (param 6).
        let param = |k: i32| self.param(font, k);
        let fuzz = param(6) / 25;
        snap_zone(&mut depth, 0, fuzz);
        snap_zone(&mut height, 0, fuzz);
        snap_zone(&mut height, param(5), fuzz);
        snap_zone(&mut height, param(8), fuzz);
        Ok((height, depth))
    }

    pub unsafe fn getnativecharht(&self, f: i32, c: i32) -> Result<i32, Abort> {
        Ok(self.get_native_char_height_depth(f, c)?.0)
    }

    pub unsafe fn getnativechardp(&self, f: i32, c: i32) -> Result<i32, Abort> {
        Ok(self.get_native_char_height_depth(f, c)?.1)
    }

    /// Left and right side bearings of character `ch` of native font `font`.
    pub unsafe fn get_native_char_sidebearings(
        &self,
        font: i32,
        ch: i32,
    ) -> Result<(i32, i32), Abort> {
        let engine = self.require_ot_engine(
            font,
            "bad native font flag in `get_native_char_side_bearings`",
        )?;
        let (mut l, mut r) = (0.0f32, 0.0f32);
        let gid = mapCharToGlyph(engine, ch as u32);
        getGlyphSidebearings(engine, gid, &mut l, &mut r);
        Ok((D2Fix(l as c_double), D2Fix(r as c_double)))
    }

    /// Glyph bound `edge` of glyph `gid`: 1 left, 2 top, 3 right, 4 bottom.
    pub unsafe fn get_glyph_bounds(&self, font: i32, edge: i32, gid: i32) -> Result<i32, Abort> {
        let engine = self.require_ot_engine(font, "bad native font flag in `get_glyph_bounds`")?;
        let (mut a, mut b) = (0.0f32, 0.0f32);
        if edge & 1 != 0 {
            getGlyphSidebearings(engine, gid as u32, &mut a, &mut b);
        } else {
            getGlyphHeightDepth(engine, gid as u32, &mut a, &mut b);
        }
        Ok(D2Fix(if edge <= 2 { a } else { b } as c_double))
    }

    pub unsafe fn getnativecharic(&self, f: i32, c: i32) -> Result<i32, Abort> {
        let (_, rsb) = self.get_native_char_sidebearings(f, c)?;
        let letter_space = self.font_letter_space_entry(f);
        Ok(if rsb < 0 {
            letter_space - rsb
        } else {
            letter_space
        })
    }

    pub unsafe fn getnativecharwd(&self, f: i32, c: i32) -> Result<i32, Abort> {
        let engine = self.require_ot_engine(f, "bad native font flag in `get_native_char_wd`")?;
        let gid = mapCharToGlyph(engine, c as u32);
        Ok(D2Fix(getGlyphWidthFromEngine(engine, gid) as c_double))
    }

    pub unsafe fn map_char_to_glyph(&self, font: i32, ch: i32) -> Result<i32, Abort> {
        if ch > 0x10ffff || (0xd800..=0xdfff).contains(&ch) {
            return Ok(0);
        }
        let engine = self.require_ot_engine(font, "bad native font flag in `map_char_to_glyph`")?;
        Ok(mapCharToGlyph(engine, ch as u32) as i32)
    }

    pub unsafe fn map_glyph_to_index(&self, font: i32, name: *const c_char) -> Result<i32, Abort> {
        let engine =
            self.require_ot_engine(font, "bad native font flag in `map_glyph_to_index`")?;
        Ok(mapGlyphToIndex(engine, name))
    }

    pub unsafe fn get_font_char_range(&self, font: i32, first: i32) -> Result<i32, Abort> {
        let engine =
            self.require_ot_engine(font, "bad native font flag in `get_font_char_range'`")?;
        Ok(getFontCharRange(engine, first))
    }
}

pub(crate) unsafe fn linebreak_start(
    st: &mut EngineState,
    f: c_int,
    locale_str_num: i32,
    text: *mut u16,
    text_length: i32,
) -> Result<(), Abort> {
    let mut status: i32 = 0;
    let locale = st.strings.texstring_cstring(locale_str_num);

    if st.fonts.font_area_entry(f) == OTGR_FONT_FLAG {
        let engine = st.fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        if locale.as_bytes() == b"G" && initGraphiteBreaking(engine, text, text_length as u32) {
            return Ok(());
        }
    }

    if locale_str_num != st.fonts.brk_locale_str_num {
        st.fonts.brk_iter = None;
    }

    if st.fonts.brk_iter.is_none() {
        let mut raw = ubrk_open(UBRK_LINE, locale.as_ptr(), ptr::null(), 0, &mut status);
        if status > 0 {
            crate::output::begin_diagnostic(st);
            crate::output::print_nl(st, b'E' as i32);
            crate::output::print(st, "rror ");
            crate::output::print_int(st, status);
            crate::output::print(st, " creating linebreak iterator for locale `");
            crate::output::print(st, locale.as_bytes());
            crate::output::print(st, "'; trying default locale `en_us'.");
            crate::output::end_diagnostic(st, true);
            if !raw.is_null() {
                ubrk_close(raw);
            }
            status = 0;
            raw = ubrk_open(UBRK_LINE, c"en_us".as_ptr(), ptr::null(), 0, &mut status);
        }
        st.fonts.brk_locale_str_num = locale_str_num;
        if !raw.is_null() {
            st.fonts.brk_iter = Some(crate::state::LinebreakHandle { raw });
        }
    }

    let iterator = st.fonts.linebreak_iterator_ptr();
    if iterator.is_null() {
        return Err(Abort::Internal("failed to create linebreak iterator"));
    }
    ubrk_setText(iterator, text, text_length, &mut status);
    Ok(())
}

pub(crate) unsafe fn close_linebreak_iterator(iterator: *mut c_void) {
    ubrk_close(iterator);
}

impl FontState {
    pub(crate) fn release_linebreak_state(&mut self) {
        self.brk_iter = None;
        self.brk_locale_str_num = 0;
        self.saved_mapping_name = None;
    }

    pub unsafe fn linebreak_next(&self, f: c_int) -> c_int {
        let iterator = self.linebreak_iterator_ptr();
        if !iterator.is_null() {
            ubrk_next(iterator)
        } else {
            findNextGraphiteBreak(self.font_layout_engine_ptr(f) as XeTeXLayoutEngine)
        }
    }

    /// Write the XDV native font definition of `f` into `fonts.xdv_buffer` and
    /// return its length.
    pub unsafe fn make_font_def(&mut self, f: i32) -> Result<c_int, Abort> {
        let engine = self.require_ot_engine(f, "bad native font flag in `make_font_def`")?;
        let mut index: u32 = 0;
        let filename = getFontFilename(engine, &mut index);
        assert!(!filename.is_null());

        let rgba = getRgbValue(engine);
        let font_flags = self.font_flags_entry(f);
        let mut flags: u16 = 0;
        if font_flags & FONT_FLAGS_VERTICAL != 0 {
            flags |= XDV_FLAG_VERTICAL;
        }
        let extend = getExtendFactor(engine);
        let slant = getSlantFactor(engine);
        let embolden = getEmboldenFactor(engine);
        let size = D2Fix(getPointSize(engine) as c_double);

        let colored = font_flags & FONT_FLAGS_COLORED != 0;
        if colored {
            flags |= XDV_FLAG_COLORED;
        }
        if extend != 1.0 {
            flags |= XDV_FLAG_EXTEND;
        }
        if slant != 0.0 {
            flags |= XDV_FLAG_SLANT;
        }
        if embolden != 0.0 {
            flags |= XDV_FLAG_EMBOLDEN;
        }

        let name = CStr::from_ptr(filename).to_bytes();
        let name_len = name.len() as u8;
        let mut def = Vec::with_capacity(4 + 2 + 1 + name_len as usize + 4 + 16);
        def.extend_from_slice(&(size as u32).to_be_bytes());
        def.extend_from_slice(&flags.to_be_bytes());
        def.push(name_len);
        def.extend_from_slice(&name[..name_len as usize]);
        def.extend_from_slice(&index.to_be_bytes());
        if colored {
            def.extend_from_slice(&rgba.to_be_bytes());
        }
        for (flag, v) in [
            (XDV_FLAG_EXTEND, extend),
            (XDV_FLAG_SLANT, slant),
            (XDV_FLAG_EMBOLDEN, embolden),
        ] {
            if flags & flag != 0 {
                def.extend_from_slice(&(D2Fix(v as c_double) as u32).to_be_bytes());
            }
        }
        freeFontFilename(filename);

        if self.xdv_buffer.len() < def.len() {
            self.xdv_buffer.resize(def.len(), 0);
        }
        self.xdv_buffer[..def.len()].copy_from_slice(&def);
        Ok(def.len() as c_int)
    }

    /// Run `txt_len` UTF-16 units through TECkit converter `p_cnv` into
    /// `fonts.mapped_text`, returning the mapped length.
    pub unsafe fn apply_mapping(
        &mut self,
        p_cnv: *mut c_void,
        txt_ptr: *mut u16,
        txt_len: c_int,
    ) -> c_int {
        const K_STATUS_OUTPUT_BUFFER_FULL: c_long = 1;
        let txt_bytes = txt_len as u32 * 2;

        let mut needed = (txt_bytes + 32) as usize;
        if self.mapped_text.len() < needed / 2 {
            self.mapped_text.resize(needed / 2, 0);
        }

        loop {
            let mut in_used: u32 = 0;
            let mut out_used: u32 = 0;
            let status = TECkit_ConvertBuffer(
                p_cnv,
                txt_ptr as *const u8,
                txt_bytes,
                &mut in_used,
                self.mapped_text.as_mut_ptr() as *mut u8,
                (self.mapped_text.len() * 2) as u32,
                &mut out_used,
                1,
            );
            TECkit_ResetConverter(p_cnv);

            match status {
                K_STATUS_NO_ERROR => return (out_used / 2) as c_int,
                K_STATUS_OUTPUT_BUFFER_FULL => {
                    needed += (txt_bytes + 32) as usize;
                    self.mapped_text.resize(needed / 2, 0);
                }
                _ => return 0,
            }
        }
    }

    /// Re-shape native word `w` at its natural width, then spread the difference
    /// to its justified width over the interword spaces (or all glyphs if none).
    pub unsafe fn store_justified_native_glyphs(
        &self,
        mem: &mut MemState,
        w: NodePtr<WhatsitNode>,
    ) -> Result<(), Abort> {
        let f = w.native_font(mem).0;
        let saved_width = w.width(mem).0;
        self.measure_native_node(mem, w, false)?;

        let width = w.width(mem).0;
        if width == saved_width {
            return Ok(());
        }
        let just_amount = Fix2D(saved_width - width);
        let glyph_count = w.native_glyph_count(mem) as usize;
        let handle = w.native_glyph_info_handle(mem);
        let glyphs = mem.glyph_blobs.get_mut(handle);

        let space_glyph = self.map_char_to_glyph(f, b' ' as i32)?;
        let is_space = |gid: u16| gid as i32 == space_glyph;
        let space_count = glyphs.gids[..glyph_count]
            .iter()
            .filter(|&&g| is_space(g))
            .count() as i32;

        if space_count > 0 {
            let mut adjustment = 0.0f64;
            let mut space_index = 0i32;
            for i in 0..glyph_count {
                glyphs.locations[i].x = D2Fix(Fix2D(glyphs.locations[i].x) + adjustment);
                if is_space(glyphs.gids[i]) {
                    space_index += 1;
                    adjustment = just_amount * space_index as f64 / space_count as f64;
                }
            }
        } else {
            for i in 1..glyph_count {
                glyphs.locations[i].x = D2Fix(
                    Fix2D(glyphs.locations[i].x)
                        + just_amount * i as f64 / (glyph_count - 1) as f64,
                );
            }
        }

        w.set_width(mem, Scaled(saved_width));
        Ok(())
    }
}

pub fn aat_get_font_metrics(_attributes: *mut c_void) -> NativeFontMetrics {
    NativeFontMetrics::default()
}

pub fn aat_font_get(_what: c_int, _attributes: *mut c_void) -> c_int {
    -1
}

pub fn aat_font_get_1(_what: c_int, _attributes: *mut c_void, _param: c_int) -> c_int {
    -1
}

pub fn aat_font_get_2(
    _what: c_int,
    _attributes: *mut c_void,
    _param1: c_int,
    _param2: c_int,
) -> c_int {
    -1
}

pub fn aat_font_get_named(_what: c_int, _attributes: *mut c_void) -> c_int {
    -1
}

pub fn aat_font_get_named_1(_what: c_int, _attributes: *mut c_void, _param: c_int) -> c_int {
    -1
}

pub fn aat_print_font_name(_what: c_int, _attributes: *mut c_void, _param1: c_int, _param2: c_int) {
}

pub(crate) unsafe fn print_glyph_name(
    st: &mut EngineState,
    font: i32,
    gid: i32,
) -> Result<(), Abort> {
    let engine = st
        .fonts
        .require_ot_engine(font, "bad native font flag in `print_glyph_name`")?;
    let mut len: c_int = 0;
    let s = getGlyphName(getFont(engine), gid as u16, &mut len);
    for i in 0..len.max(0) as usize {
        crate::output::print_char(st, *s.add(i) as i32);
    }
    if !s.is_null() {
        freeGlyphName(s as *mut c_char);
    }
    Ok(())
}

// `\openout` .. `\special` and `\pdfsavepos` carry their whatsit subtype as the
// command modifier.
pub(crate) const OPEN_NODE: i32 = WhatsitSubtype::Open as i32;
pub(crate) const WRITE_NODE: i32 = WhatsitSubtype::Write as i32;
pub(crate) const CLOSE_NODE: i32 = WhatsitSubtype::Close as i32;
pub(crate) const SPECIAL_NODE: i32 = WhatsitSubtype::Special as i32;
pub(crate) const PDF_SAVE_POS_NODE: i32 = WhatsitSubtype::PdfSavePos as i32;
pub(crate) const IMMEDIATE_CODE: i32 = 4;
pub(crate) const SET_LANGUAGE_CODE: i32 = 5;
pub(crate) const RESET_TIMER_CODE: i32 = 31;
pub(crate) const SET_RANDOM_SEED_CODE: i32 = 33;
pub(crate) const PIC_FILE_CODE: i32 = 41;
pub(crate) const PDF_FILE_CODE: i32 = 42;
pub(crate) const GLYPH_CODE: i32 = 43;
pub(crate) const XETEX_INPUT_ENCODING_EXTENSION_CODE: i32 = 44;
pub(crate) const XETEX_DEFAULT_ENCODING_EXTENSION_CODE: i32 = 45;
pub(crate) const XETEX_LINEBREAK_LOCALE_EXTENSION_CODE: i32 = 46;

/// The current source file if the input position moved since the last source
/// special.
fn new_source_position(st: &mut EngineState) -> Option<i32> {
    let fname = st.input.source_filename_stack[st.input.in_open as usize];
    (fname > 0 && crate::texmfmp::is_new_source(st, fname, st.mem.line)).then_some(fname)
}

/// Append a `\special{src:...}` whatsit for the current source line.
pub(crate) fn append_src_special(st: &mut EngineState) -> Result<(), Abort> {
    let Some(fname) = new_source_position(st) else {
        return Ok(());
    };
    let w = st
        .input
        .cur_list
        .new_whatsit(&mut st.mem, WhatsitSubtype::Special, WRITE_NODE_SIZE)?;
    w.set_write_stream(&mut st.mem, 0);
    let d = st.mem.get_avail()?;
    st.scanner.def_ref = d;
    d.set_info(&mut st.mem, crate::mem::TEX_NULL);
    let b = crate::texmfmp::make_src_special(st, fname, st.mem.line)
        .map_err(|()| Abort::Internal("string pool overflow"))?;
    st.mem.str_toks_cat(&mut st.strings, b, 0)?;
    let text = TEMP_HEAD.link(&st.mem);
    d.set_link(&mut st.mem, text);
    w.set_write_tokens(&mut st.mem, d);
    crate::texmfmp::remember_source_info(st, fname, st.mem.line);
    Ok(())
}

/// Insert `\special{src:...}` for the current source line into the input.
pub(crate) fn insert_src_special(st: &mut EngineState) -> Result<(), Abort> {
    let Some(fname) = new_source_position(st) else {
        return Ok(());
    };
    let toklist = st.mem.get_avail()?;
    toklist.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_SPECIAL);
    let open = st.mem.get_avail()?;
    toklist.set_link(&mut st.mem, open);
    open.set_info(&mut st.mem, LEFT_BRACE_TOKEN + b'{' as i32);
    let b = crate::texmfmp::make_src_special(st, fname, st.mem.line)
        .map_err(|()| Abort::Internal("string pool overflow"))?;
    let text_tail = st.mem.str_toks_cat(&mut st.strings, b, 0)?;
    let text = TEMP_HEAD.link(&st.mem);
    open.set_link(&mut st.mem, text);
    let close = st.mem.get_avail()?;
    text_tail.set_link(&mut st.mem, close);
    close.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
    crate::input::begin_token_list(st, toklist, INSERTED)?;
    crate::texmfmp::remember_source_info(st, fname, st.mem.line);
    Ok(())
}

/// Insert `\special{<ascii_text>}` into the input.
pub(crate) fn tt_insert_special(st: &mut EngineState, text: &str) -> Result<(), Abort> {
    let start_pool = st.strings.pool_ptr;
    let text = text.as_bytes();
    if st.strings.pool_ptr + text.len() as i32 >= st.strings.pool_size {
        return Err(Abort::Overflow {
            what: "string pool overflow",
            n: 0,
        });
    }
    for &b in text {
        st.strings.str_pool[st.strings.pool_ptr as usize] = b as u16;
        st.strings.pool_ptr += 1;
    }

    let toklist = st.mem.get_avail()?;
    toklist.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_SPECIAL);
    let open = st.mem.get_avail()?;
    open.set_info(&mut st.mem, LEFT_BRACE_TOKEN + b'{' as i32);
    toklist.set_link(&mut st.mem, open);
    let text_tail = st.mem.str_toks_cat(&mut st.strings, start_pool, 0)?;
    let text = TEMP_HEAD.link(&st.mem);
    open.set_link(&mut st.mem, text);
    let close = st.mem.get_avail()?;
    close.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
    text_tail.set_link(&mut st.mem, close);
    crate::input::begin_token_list(st, toklist, INSERTED)?;
    Ok(())
}

fn new_whatsit(
    st: &mut EngineState,
    subtype: WhatsitSubtype,
    size: i32,
) -> Result<NodePtr<WhatsitNode>, Abort> {
    Ok(st.input.cur_list.new_whatsit(&mut st.mem, subtype, size)?)
}

/// Append an `\openout`/`\write`/`\closeout` whatsit and scan its stream number.
fn new_write_whatsit(
    st: &mut EngineState,
    subtype: WhatsitSubtype,
    size: i32,
) -> Result<NodePtr<WhatsitNode>, Abort> {
    let w = new_whatsit(st, subtype, size)?;
    if size != WRITE_NODE_SIZE {
        crate::scan::scan_four_bit_int(st)?;
    } else {
        scan_int(st)?;
        if st.scanner.cur_val < 0 {
            st.scanner.cur_val = 17;
        } else if st.scanner.cur_val > 15 && st.scanner.cur_val != 18 {
            st.scanner.cur_val = 16;
        }
    }
    w.set_write_stream(&mut st.mem, st.scanner.cur_val);
    Ok(w)
}

/// Appends the `\openout`, `\write` or `\closeout` whatsit for `cur_chr`.
fn io_whatsit(st: &mut EngineState) -> Result<NodePtr<WhatsitNode>, Abort> {
    Ok(match st.scanner.cur_chr {
        OPEN_NODE => {
            let w = new_write_whatsit(st, WhatsitSubtype::Open, OPEN_NODE_SIZE)?;
            crate::scan::scan_optional_equals(st)?;
            crate::io::scan_file_name(st)?;
            w.set_open_name(&mut st.mem, st.scanner.cur_name);
            w.set_open_area(&mut st.mem, st.scanner.cur_area);
            w.set_open_ext(&mut st.mem, st.scanner.cur_ext);
            w
        }
        WRITE_NODE => {
            let k = st.scanner.cur_cs;
            let w = new_write_whatsit(st, WhatsitSubtype::Write, WRITE_NODE_SIZE)?;
            st.scanner.cur_cs = k;
            crate::scan::scan_toks(st, false, false)?;
            w.set_write_tokens(&mut st.mem, st.scanner.def_ref);
            w
        }
        _ => {
            let w = new_write_whatsit(st, WhatsitSubtype::Close, WRITE_NODE_SIZE)?;
            w.set_write_tokens(&mut st.mem, MemPtr::NULL);
            w
        }
    })
}

pub(crate) fn do_extension(st: &mut EngineState) -> Result<(), Abort> {
    let mode = st.input.cur_list.mode.abs();
    match st.scanner.cur_chr {
        OPEN_NODE | WRITE_NODE | CLOSE_NODE => {
            io_whatsit(st)?;
        }

        SPECIAL_NODE => {
            let w = new_whatsit(st, WhatsitSubtype::Special, WRITE_NODE_SIZE)?;
            w.set_write_stream(&mut st.mem, crate::mem::TEX_NULL);
            crate::scan::scan_toks(st, false, true)?;
            w.set_write_tokens(&mut st.mem, st.scanner.def_ref);
        }

        IMMEDIATE_CODE => {
            get_x_token(st)?;
            if st.scanner.cur_cmd == EXTENSION && st.scanner.cur_chr <= CLOSE_NODE {
                let p = st.input.cur_list.tail;
                let w = io_whatsit(st)?;
                crate::shipout::out_what(st, w)?;
                st.mem.flush_node_list(st.input.cur_list.tail)?;
                st.input.cur_list.tail = p;
                p.set_link(&mut st.mem, MemPtr::NULL);
            } else {
                crate::input::back_input(st)?;
            }
        }

        SET_LANGUAGE_CODE => {
            if mode != HMODE {
                crate::errors::report_illegal_case(st)?;
            } else {
                let w = new_whatsit(st, WhatsitSubtype::Language, SMALL_NODE_SIZE)?;
                scan_int(st)?;
                let lang = if st.scanner.cur_val <= 0 || st.scanner.cur_val > 255 {
                    0
                } else {
                    st.scanner.cur_val
                };
                st.input.cur_list.set_aux_s1(lang);
                w.set_what_lang(&mut st.mem, lang);
                let lhm = crate::helpers::norm_min(st.eqtb.int_par(INT_PAR_LEFT_HYPHEN_MIN));
                w.set_what_lhm(&mut st.mem, lhm as u16);
                let rhm = crate::helpers::norm_min(st.eqtb.int_par(INT_PAR_RIGHT_HYPHEN_MIN));
                w.set_what_rhm(&mut st.mem, rhm as u16);
            }
        }

        PDF_SAVE_POS_NODE => {
            new_whatsit(st, WhatsitSubtype::PdfSavePos, SMALL_NODE_SIZE)?;
        }

        RESET_TIMER_CODE => {
            if !st.config.deterministic_mode {
                (st.shipout.epochseconds, st.shipout.microseconds) =
                    crate::texmfmp::get_seconds_and_micros();
            }
        }

        SET_RANDOM_SEED_CODE => {
            scan_int(st)?;
            if st.scanner.cur_val < 0 {
                st.scanner.cur_val = -st.scanner.cur_val;
            }
            st.errors.random_seed = st.scanner.cur_val;
            let random_seed = st.errors.random_seed;
            st.errors.init_randoms(random_seed);
        }

        PIC_FILE_CODE | PDF_FILE_CODE => {
            if mode == MMODE {
                crate::errors::report_illegal_case(st)?;
            } else {
                load_picture(st, st.scanner.cur_chr == PDF_FILE_CODE)?;
            }
        }

        GLYPH_CODE => {
            if mode == VMODE {
                crate::input::back_input(st)?;
                new_graf(st, true)?;
            } else if mode == MMODE {
                crate::errors::report_illegal_case(st)?;
            } else {
                let cur_font = st.eqtb.cur_font();
                let area = st.fonts.font_area_entry(cur_font);
                if area != AAT_FONT_FLAG && area != OTGR_FONT_FLAG {
                    return crate::errors::not_font_error_common(
                        st,
                        EXTENSION,
                        GLYPH_CODE,
                        cur_font,
                        "; not a native platform font",
                    );
                }
                let w = new_whatsit(st, WhatsitSubtype::Glyph, GLYPH_NODE_SIZE)?;
                scan_int(st)?;
                if st.scanner.cur_val < 0 || st.scanner.cur_val > 65535 {
                    error_here_with_diagnostic(st, "Bad glyph number");
                    crate::output::append_to_diagnostic(st, &format!(" ({})", st.scanner.cur_val));
                    capture_to_diagnostic(st, None);
                    st.errors.help(&[
                        "A glyph number must be between 0 and 65535.",
                        "I changed this one to zero.",
                    ]);
                    crate::errors::int_error(st, st.scanner.cur_val)?;
                    st.scanner.cur_val = 0;
                }
                w.set_native_font(&mut st.mem, FontId(cur_font));
                w.set_native_glyph(&mut st.mem, st.scanner.cur_val as u16);
                let use_glyph_metrics = st.eqtb.int_par(INT_PAR_XETEX_USE_GLYPH_METRICS) > 0;
                // SAFETY: the font is native, the measurer only reads its tables.
                unsafe {
                    st.fonts
                        .measure_native_glyph(&mut st.mem, w, use_glyph_metrics)?
                };
            }
        }

        XETEX_INPUT_ENCODING_EXTENSION_CODE => {
            crate::io::scan_and_pack_name(st)?;
            let name = st.input.name_of_file_cstr().unwrap_or_default().to_owned();
            let (enc_mode, info) = get_encoding_mode_and_info(st, &name)?;
            if enc_mode == AUTO {
                error_here_with_diagnostic(
                    st,
                    "Encoding mode `auto' is not valid for \\XeTeXinputencoding",
                );
                capture_to_diagnostic(st, None);
                st.errors.help(&[
                    r"You can't use `auto' encoding here, only for \XeTeXdefaultencoding.",
                    "I'll ignore this and leave the current encoding unchanged.",
                ]);
                crate::errors::error(st)?;
            } else {
                // `\scantokens` occupies an input level without a UFile.
                let slot = crate::io::FileSlot::Input(st.input.in_open as usize);
                crate::io::with_slot_file(st, slot, |st, f| {
                    crate::io::set_input_file_encoding(st, f, enc_mode, info)
                })
                .transpose()?;
            }
        }

        XETEX_DEFAULT_ENCODING_EXTENSION_CODE => {
            crate::io::scan_and_pack_name(st)?;
            let name = st.input.name_of_file_cstr().unwrap_or_default().to_owned();
            let (enc_mode, info) = get_encoding_mode_and_info(st, &name)?;
            st.eqtb
                .set_int_par(INT_PAR_XETEX_DEFAULT_INPUT_MODE, enc_mode);
            st.eqtb
                .set_int_par(INT_PAR_XETEX_DEFAULT_INPUT_ENCODING, info);
        }

        XETEX_LINEBREAK_LOCALE_EXTENSION_CODE => {
            crate::io::scan_file_name(st)?;
            let locale = if st.strings.length(st.scanner.cur_name) == 0 {
                0
            } else {
                st.scanner.cur_name
            };
            st.eqtb.set_int_par(INT_PAR_XETEX_LINEBREAK_LOCALE, locale);
        }

        _ => return Err(Abort::Confusion("ext1")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn read_double_parses_sign_whitespace_and_fraction() {
        let input = CString::new(" \t-12.375pt").unwrap();
        let mut ptr = input.as_ptr();

        let value = unsafe { read_double(&mut ptr) };

        assert!((value + 12.375).abs() < 1e-12);
        assert_eq!(unsafe { *ptr }, b'p' as c_char);
    }

    #[test]
    fn read_rgb_a_accepts_rgb_and_optional_alpha() {
        let rgb = CString::new("a1B2c3!").unwrap();
        let mut rgb_ptr = rgb.as_ptr();
        assert_eq!(unsafe { read_rgb_a(&mut rgb_ptr) }, 0xA1B2C3FF);
        assert_eq!(unsafe { *rgb_ptr }, b'!' as c_char);

        let rgba = CString::new("0102037f!").unwrap();
        let mut rgba_ptr = rgba.as_ptr();
        assert_eq!(unsafe { read_rgb_a(&mut rgba_ptr) }, 0x0102037F);
        assert_eq!(unsafe { *rgba_ptr }, b'!' as c_char);
    }

    #[test]
    fn read_rgb_a_invalid_base_color_returns_black_and_stops() {
        let input = CString::new("12xx").unwrap();
        let mut ptr = input.as_ptr();

        assert_eq!(unsafe { read_rgb_a(&mut ptr) }, 0x000000FF);
        assert_eq!(unsafe { *ptr }, b'x' as c_char);
    }

    #[test]
    fn read_feature_number_requires_number_setting_and_full_consumption() {
        let input = CString::new("123 =45").unwrap();
        let mut feature = 999;
        let mut value = 999;

        assert!(unsafe {
            readFeatureNumber(
                input.as_ptr(),
                input.as_ptr().add(7),
                &mut feature,
                &mut value,
            )
        });
        assert_eq!(feature, 123);
        assert_eq!(value, 45);

        let trailing = CString::new("123 =45x").unwrap();
        assert!(!unsafe {
            readFeatureNumber(
                trailing.as_ptr(),
                trailing.as_ptr().add(8),
                &mut feature,
                &mut value,
            )
        });
    }

    #[test]
    fn split_font_name_matches_bracket_and_unbracketed_forms() {
        let mut bracketed = CString::new("[font/name.ttf:2]:color=FF0000")
            .unwrap()
            .into_bytes_with_nul();
        let mut var = ptr::null_mut();
        let mut feat = ptr::null_mut();
        let mut end = ptr::null_mut();
        let mut index = -1;

        unsafe {
            splitFontName(
                bracketed.as_mut_ptr() as *mut c_char,
                &mut var,
                &mut feat,
                &mut end,
                &mut index,
            );
            let base = bracketed.as_mut_ptr() as *mut c_char;
            assert_eq!(var.offset_from(base), 14);
            assert_eq!(feat.offset_from(base), 17);
            assert_eq!(end.offset_from(base), 30);
        }
        assert_eq!(index, 2);

        let mut named = CString::new("Font/Bold:script=latn")
            .unwrap()
            .into_bytes_with_nul();
        unsafe {
            splitFontName(
                named.as_mut_ptr() as *mut c_char,
                &mut var,
                &mut feat,
                &mut end,
                &mut index,
            );
            let base = named.as_mut_ptr() as *mut c_char;
            assert_eq!(var.offset_from(base), 4);
            assert_eq!(feat.offset_from(base), 9);
            assert_eq!(end.offset_from(base), 21);
        }
        assert_eq!(index, 0);
    }
}
