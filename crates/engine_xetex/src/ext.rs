// translated from xetex-ext.c
use crate::bridge::{diag_append_str, ttstub_input_get_size, ttstub_input_open, ttstub_input_read};
use crate::mem::{xcalloc, xmalloc, xrealloc};
use crate::state::EngineState;

use crate::consts::{
    CS_TOKEN_FLAG, CUR_FONT_LOC, INSERTED, INT_BASE, INT_PAR_XETEX_TRACING_FONTS, LEFT_SIDE,
    NATIVE_GLYPH_INFO_SIZE, RIGHT_SIDE, XETEX_INT,
};
use crate::glyph_arena::{FixedPoint, GlyphBlob};
use crate::main_control::{mc_propagate_error, new_graf_in};
use crate::output::{capture_to_diagnostic_in, error_here_with_diagnostic_in};
use crate::pic::load_picture_in;
use crate::scan::{get_x_token_in, scan_int_in};

use libc::{c_char, c_double, c_float, c_int, c_long, c_uint, c_void};
use std::ptr;

type XeTeXFont = *mut c_void;
type XeTeXLayoutEngine = *mut c_void;
type RawPlatformFontRef = *mut c_void;

const TTBC_FILE_FORMAT_MISC_FONTS: c_int = 41;
const XETEX_FEATURE_NAME_CODE: i32 = 35;
const XETEX_SELECTOR_NAME_CODE: i32 = 36;
const XETEX_COUNT_GLYPHS: i32 = 28 - XETEX_INT;
const XETEX_COUNT_FEATURES: i32 = 35 - XETEX_INT;
const XETEX_FEATURE_CODE: i32 = 36 - XETEX_INT;
const XETEX_FIND_FEATURE_BY_NAME: i32 = 37 - XETEX_INT;
const XETEX_IS_EXCLUSIVE_FEATURE: i32 = 38 - XETEX_INT;
const XETEX_COUNT_SELECTORS: i32 = 39 - XETEX_INT;
const XETEX_SELECTOR_CODE: i32 = 40 - XETEX_INT;
const XETEX_FIND_SELECTOR_BY_NAME: i32 = 41 - XETEX_INT;
const XETEX_IS_DEFAULT_SELECTOR: i32 = 42 - XETEX_INT;
const XETEX_OT_COUNT_SCRIPTS: i32 = 43 - XETEX_INT;
const XETEX_OT_COUNT_LANGUAGES: i32 = 44 - XETEX_INT;
const XETEX_OT_COUNT_FEATURES: i32 = 45 - XETEX_INT;
const XETEX_OT_SCRIPT_CODE: i32 = 46 - XETEX_INT;
const XETEX_OT_LANGUAGE_CODE: i32 = 47 - XETEX_INT;
const XETEX_OT_FEATURE_CODE: i32 = 48 - XETEX_INT;
#[cfg(target_os = "macos")]
const AAT_FONT_FLAG: u32 = 0xFFFF;
const OTGR_FONT_FLAG: u32 = 0xFFFE;
const HB_TAG_NONE: u32 = 0;
const AUTO: c_int = 0;
const UTF8: c_int = 1;
const UTF16BE: c_int = 2;
const UTF16LE: c_int = 3;
const RAW: c_int = 4;
const ICUMAPPING: c_int = 5;
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

extern "C" {
    // TECkit_Status is `long` (teckit-Common.h:50), which is 64-bit on LP64
    // targets and 32-bit on wasm32 and Windows. `c_long` is the spelling that
    // is right everywhere; wasm validates call signatures, so getting this
    // wrong traps rather than quietly working.
    fn TECkit_CreateConverter(
        mapping: *const u8,
        mapping_size: u32,
        map_forward: u8,
        source_form: u16,
        target_form: u16,
        converter: *mut *mut c_void,
    ) -> libc::c_long;
    fn TECkit_ConvertBuffer(
        converter: *mut c_void,
        in_buffer: *const u8,
        in_length: u32,
        in_used: *mut u32,
        out_buffer: *mut u8,
        out_length: u32,
        out_used: *mut u32,
        input_is_complete: u8,
    ) -> libc::c_long;
    fn TECkit_ResetConverter(converter: *mut c_void) -> libc::c_long;
    fn TECkit_DisposeConverter(converter: *mut c_void) -> libc::c_long;
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
    #[cfg(target_os = "macos")]
    fn CFRelease(cf: *const c_void);
}

const FONT_FLAGS_VERTICAL: i32 = 0x02;
const UBRK_LINE: c_int = 2;
const UBIDI_RTL: c_int = 1;
const UBIDI_MIXED: c_int = 2;
const XDV_FLAG_VERTICAL: u16 = 0x0100;
const XDV_FLAG_COLORED: u16 = 0x0200;
const XDV_FLAG_EXTEND: u16 = 0x1000;
const XDV_FLAG_SLANT: u16 = 0x2000;
const XDV_FLAG_EMBOLDEN: u16 = 0x4000;

#[repr(C)]
struct FloatPoint {
    x: f32,
    y: f32,
}

#[repr(C)]
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

unsafe fn print_c_string_in(st: &mut EngineState, s: *const c_char) {
    let mut p = s;
    while *p != 0 {
        crate::output::print_raw_char_in(st, *p as u8 as u16, true);
        p = p.add(1);
    }
}

unsafe fn peek(ptr: *const c_char) -> u8 {
    *ptr as u8
}

unsafe fn str_starts_with(s: *const c_char, prefix: &[u8]) -> *const c_char {
    for (idx, byte) in prefix.iter().enumerate() {
        if peek(s.add(idx)) != *byte {
            return ptr::null();
        }
    }
    s.add(prefix.len())
}

unsafe fn push_hb_feature(
    features: &mut *mut HbFeature,
    n_features: &mut c_int,
    tag: u32,
    value: u32,
) {
    *features = xrealloc(
        *features as *mut c_void,
        ((*n_features as usize + 1) * std::mem::size_of::<HbFeature>()) as libc::size_t,
    ) as *mut HbFeature;
    let feature = (*features).add(*n_features as usize);
    (*feature).tag = tag;
    (*feature).value = value;
    (*feature).start = 0;
    (*feature).end = u32::MAX;
    *n_features += 1;
}

unsafe fn copy_c_slice(start: *const c_char, end: *const c_char) -> *mut c_char {
    let len = end.offset_from(start) as usize;
    let out = xmalloc((len + 1) as libc::size_t) as *mut c_char;
    ptr::copy_nonoverlapping(start, out, len);
    *out.add(len) = 0;
    out
}

// translated from xetex-ext.c:1882 (D2Fix)
#[no_mangle]
pub extern "C" fn D2Fix(d: c_double) -> i32 {
    (d * 65536.0 + 0.5) as i32
}

// translated from xetex-ext.c:1888 (Fix2D)
#[no_mangle]
pub extern "C" fn Fix2D(f: i32) -> c_double {
    f as c_double / 65536.0
}

// translated from xetex-ext.c:279 (read_double)
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
    let c = peek(*cp);
    let value = match c {
        b'0'..=b'9' => (c - b'0') as u32,
        b'A'..=b'F' => (c - b'A' + 10) as u32,
        b'a'..=b'f' => (c - b'a' + 10) as u32,
        _ => return None,
    };
    *cp = (*cp).add(1);
    Some(value)
}

// translated from xetex-ext.c:344 (read_rgb_a)
pub unsafe fn read_rgb_a(cp: *mut *const c_char) -> c_uint {
    let mut rgb_value = 0u32;
    let mut alpha = 0u32;

    for _ in 0..6 {
        let Some(value) = read_hex_digit(cp) else {
            return 0x000000FF;
        };
        rgb_value = (rgb_value << 4) + value;
    }

    rgb_value <<= 8;
    let mut digits = 0;
    for _ in 0..2 {
        let Some(value) = read_hex_digit(cp) else {
            break;
        };
        alpha = (alpha << 4) + value;
        digits += 1;
    }

    if digits == 2 {
        rgb_value += alpha;
    } else {
        rgb_value += 0xFF;
    }

    rgb_value
}

// translated from xetex-ext.c:127 (get_encoding_mode_and_info)
pub(crate) unsafe fn get_encoding_mode_and_info_in(
    st: &mut EngineState,
    name_of_file: *const c_char,
    info: *mut i32,
) -> Result<c_int, crate::texmfmp::MakeTexStringAbort> {
    *info = 0;

    if crate::compat::strcasecmp(name_of_file, c"auto".as_ptr()) == 0 {
        return Ok(AUTO);
    }
    if crate::compat::strcasecmp(name_of_file, c"utf8".as_ptr()) == 0 {
        return Ok(UTF8);
    }
    if crate::compat::strcasecmp(name_of_file, c"utf16".as_ptr()) == 0 {
        return Ok(US_NATIVE_UTF16);
    }
    if crate::compat::strcasecmp(name_of_file, c"utf16be".as_ptr()) == 0 {
        return Ok(UTF16BE);
    }
    if crate::compat::strcasecmp(name_of_file, c"utf16le".as_ptr()) == 0 {
        return Ok(UTF16LE);
    }
    if crate::compat::strcasecmp(name_of_file, c"bytes".as_ptr()) == 0 {
        return Ok(RAW);
    }

    let mut err = 0;
    let cnv = crate::io::xetex_ucnv_open(name_of_file, &mut err);
    if cnv.is_null() {
        crate::output::begin_diagnostic_in(st);
        crate::output::print_nl_in(st, b'U' as i32);
        crate::output::print_cstr_in(st, c"nknown encoding `".as_ptr());
        crate::output::print_cstr_in(st, name_of_file);
        crate::output::print_cstr_in(st, c"'; reading as raw bytes".as_ptr());
        crate::output::end_diagnostic_in(st, true);
        Ok(RAW)
    } else {
        crate::io::xetex_ucnv_close(cnv);
        *info = crate::texmfmp::maketexstring_in(&mut st.strings, name_of_file)?;
        Ok(ICUMAPPING)
    }
}

// translated from xetex-ext.c:173 (read_tag_with_param)
pub unsafe fn read_tag_with_param(cp: *const c_char, param: *mut c_int) -> u32 {
    let mut cp2 = cp;
    while peek(cp2) != 0
        && peek(cp2) != b':'
        && peek(cp2) != b';'
        && peek(cp2) != b','
        && peek(cp2) != b'='
    {
        cp2 = cp2.add(1);
    }

    let tag = hb_tag_from_string(cp, cp2.offset_from(cp) as c_int);

    let mut cp = cp2;
    if peek(cp) == b'=' {
        let mut neg = false;
        cp = cp.add(1);
        if peek(cp) == b'-' {
            neg = true;
            cp = cp.add(1);
        }
        while peek(cp).is_ascii_digit() {
            *param = *param * 10 + (peek(cp) - b'0') as c_int;
            cp = cp.add(1);
        }
        if neg {
            *param = -*param;
        }
    }

    tag
}

// translated from xetex-ext.c:204 (readFeatureNumber)
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

/// translated from xetex-ext.c:173 (print_utf8_str).
pub(crate) fn print_utf8_str_in(
    st: &mut crate::state::EngineState,
    mut str_: *const u8,
    mut len: c_int,
) {
    while len > 0 {
        crate::output::print_raw_char_in(st, unsafe { *str_ } as u16, true);
        str_ = unsafe { str_.add(1) };
        len -= 1;
    }
}

// translated from xetex-ext.c:187 (load_mapping_file)
unsafe fn load_mapping_file_in(
    st: &mut EngineState,
    s: *const c_char,
    e: *const c_char,
    byte_mapping: c_char,
) -> Result<Option<std::sync::Arc<crate::state::FontMappingHandle>>, NativeFontAbort> {
    let source_len = e.offset_from(s) as usize;
    let buffer = xmalloc((source_len + 5) as libc::size_t) as *mut c_char;
    ptr::copy_nonoverlapping(s, buffer, source_len);
    ptr::copy_nonoverlapping(c".tec".as_ptr(), buffer.add(source_len), 5);

    let mut cnv: *mut c_void = ptr::null_mut();
    let map = ttstub_input_open(buffer, TTBC_FILE_FORMAT_MISC_FONTS, 0);
    if map != 0 {
        let mapping_size = ttstub_input_get_size(map);
        let mapping = xmalloc(mapping_size) as *mut u8;
        let read = ttstub_input_read(map, mapping, mapping_size);

        if read < 0 || read as libc::size_t != mapping_size {
            return Err(NativeFontAbort::CouldNotReadMapping);
        }

        // Non-longjmp input-close: the C ttstub_input_close wrapper
        // longjmp'd on a serious internal close error; propagate it as an abort.
        if crate::io::input_close(map) {
            return Err(NativeFontAbort::InputClose);
        }

        if byte_mapping != 0 {
            TECkit_CreateConverter(
                mapping,
                mapping_size as u32,
                0,
                UTF16_NATIVE,
                K_FORM_BYTES,
                &mut cnv,
            );
        } else {
            TECkit_CreateConverter(
                mapping,
                mapping_size as u32,
                1,
                UTF16_NATIVE,
                UTF16_NATIVE,
                &mut cnv,
            );
        }

        if cnv.is_null() {
            crate::output::font_mapping_warning_in(
                st,
                buffer as *const c_void,
                libc::strlen(buffer) as i32,
                2,
            );
        } else if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_TRACING_FONTS) > 1 {
            crate::output::font_mapping_warning_in(
                st,
                buffer as *const c_void,
                libc::strlen(buffer) as i32,
                0,
            );
        }

        libc::free(mapping as *mut c_void);
    } else {
        crate::output::font_mapping_warning_in(
            st,
            buffer as *const c_void,
            libc::strlen(buffer) as i32,
            1,
        );
    }

    libc::free(buffer as *mut c_void);
    Ok((!cnv.is_null()).then(|| std::sync::Arc::new(crate::state::FontMappingHandle { raw: cnv })))
}

// translated from xetex-ext.c:236 (check_for_tfm_font_mapping)
pub(crate) unsafe fn check_for_tfm_font_mapping_in(
    fonts: &mut crate::state::FontState,
    name_of_file: *mut c_char,
) {
    let cp = libc::strstr(name_of_file, c":mapping=".as_ptr());
    fonts.saved_mapping_name = None;
    if !cp.is_null() {
        *cp = 0;
        let mut cp = cp.add(9);
        while *cp != 0 && *cp <= b' ' as c_char {
            cp = cp.add(1);
        }
        if *cp != 0 {
            fonts.saved_mapping_name = Some(std::ffi::CStr::from_ptr(cp).to_owned());
        }
    }
}

// translated from xetex-ext.c:251 (load_tfm_font_mapping)
pub(crate) unsafe fn load_tfm_font_mapping_in(
    st: &mut EngineState,
) -> Result<Option<std::sync::Arc<crate::state::FontMappingHandle>>, NativeFontAbort> {
    let Some(name) = st.fonts.saved_mapping_name.take() else {
        return Ok(None);
    };
    let start = name.as_ptr();
    load_mapping_file_in(st, start, start.add(name.as_bytes().len()), 1)
}

// translated from xetex-ext.c:263 (apply_tfm_font_mapping)
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

// translated from xetex-ext.c:310 (readCommonFeatures).
unsafe fn read_common_features_in(
    st: &mut EngineState,
    feat: *const c_char,
    end: *const c_char,
    extend: *mut f32,
    slant: *mut f32,
    embolden: *mut f32,
    letterspace: *mut f32,
    rgb_value: *mut u32,
) -> Result<c_int, NativeFontAbort> {
    let mut sep = str_starts_with(feat, b"mapping");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return Ok(-1);
        }
        st.fonts.loaded_font_mapping = load_mapping_file_in(st, sep.add(1), end, 0)?;
        return Ok(1);
    }

    sep = str_starts_with(feat, b"extend");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return Ok(-1);
        }
        sep = sep.add(1);
        *extend = read_double(&mut sep) as f32;
        return Ok(1);
    }

    sep = str_starts_with(feat, b"slant");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return Ok(-1);
        }
        sep = sep.add(1);
        *slant = read_double(&mut sep) as f32;
        return Ok(1);
    }

    sep = str_starts_with(feat, b"embolden");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return Ok(-1);
        }
        sep = sep.add(1);
        *embolden = read_double(&mut sep) as f32;
        return Ok(1);
    }

    sep = str_starts_with(feat, b"letterspace");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return Ok(-1);
        }
        sep = sep.add(1);
        *letterspace = read_double(&mut sep) as f32;
        return Ok(1);
    }

    sep = str_starts_with(feat, b"color");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return Ok(-1);
        }
        sep = sep.add(1);
        let start = sep;
        *rgb_value = read_rgb_a(&mut sep);
        let consumed = sep.offset_from(start);
        if consumed == 6 || consumed == 8 {
            st.fonts.loaded_font_flags =
                (st.fonts.loaded_font_flags as i32 | FONT_FLAGS_COLORED) as c_char;
        } else {
            return Ok(-1);
        }
        return Ok(1);
    }

    Ok(0)
}

// translated from xetex-ext.c:72 (loadOTfont) — private helper
unsafe fn load_ot_font(
    st: &mut EngineState,
    _font_ref: RawPlatformFontRef,
    font: XeTeXFont,
    scaled_size: i32,
    mut cp1: *mut c_char,
) -> Result<Option<crate::state::FontEngineHandle>, NativeFontAbort> {
    let mut engine: XeTeXLayoutEngine = ptr::null_mut();
    let mut script = HB_TAG_NONE;
    let mut language: *mut c_char = ptr::null_mut();
    let mut features: *mut HbFeature = ptr::null_mut();
    let mut shapers: *mut *const c_char = ptr::null_mut();
    let mut n_features: c_int = 0;
    let mut n_shapers: c_int = 0;
    let mut rgb_value: u32 = 0x000000FF;
    let mut extend: c_float = 1.0;
    let mut slant: c_float = 0.0;
    let mut embolden: c_float = 0.0;
    let mut letterspace: c_float = 0.0;
    let req_engine = getReqEngine();

    if req_engine == b'O' as c_char || req_engine == b'G' as c_char {
        shapers = xrealloc(
            shapers as *mut c_void,
            ((n_shapers as usize + 1) * std::mem::size_of::<*const c_char>()) as libc::size_t,
        ) as *mut *const c_char;
        if req_engine == b'O' as c_char {
            *shapers.add(n_shapers as usize) = c"ot".as_ptr();
        } else {
            *shapers.add(n_shapers as usize) = c"graphite2".as_ptr();
        }
        n_shapers += 1;
    }

    if req_engine == b'G' as c_char {
        let graphite_shaper = [c"graphite2".as_ptr(), ptr::null()];
        engine = createLayoutEngineBorrowed(
            font,
            script,
            ptr::null_mut(),
            features,
            n_features,
            graphite_shaper.as_ptr(),
            rgb_value,
            extend,
            slant,
            embolden,
        );
        if engine.is_null() {
            return Ok(None);
        }
    }

    if !cp1.is_null() {
        while *cp1 != 0 {
            if *cp1 == b':' as c_char || *cp1 == b';' as c_char || *cp1 == b',' as c_char {
                cp1 = cp1.add(1);
            }
            while *cp1 == b' ' as c_char || *cp1 == b'\t' as c_char {
                cp1 = cp1.add(1);
            }
            if *cp1 == 0 {
                break;
            }

            let mut cp2 = cp1;
            while *cp2 != 0
                && *cp2 != b':' as c_char
                && *cp2 != b';' as c_char
                && *cp2 != b',' as c_char
            {
                cp2 = cp2.add(1);
            }

            let mut bad_option = false;
            let mut handled = false;

            let mut cp3 = str_starts_with(cp1, b"script");
            if !cp3.is_null() {
                if *cp3 != b'=' as c_char {
                    bad_option = true;
                } else {
                    cp3 = cp3.add(1);
                    script = hb_tag_from_string(cp3, cp2.offset_from(cp3) as c_int);
                    handled = true;
                }
            }

            if !handled && !bad_option {
                cp3 = str_starts_with(cp1, b"language");
                if !cp3.is_null() {
                    if *cp3 != b'=' as c_char {
                        bad_option = true;
                    } else {
                        cp3 = cp3.add(1);
                        language = copy_c_slice(cp3, cp2);
                        handled = true;
                    }
                }
            }

            if !handled && !bad_option {
                cp3 = str_starts_with(cp1, b"shaper");
                if !cp3.is_null() {
                    if *cp3 != b'=' as c_char {
                        bad_option = true;
                    } else {
                        cp3 = cp3.add(1);
                        shapers = xrealloc(
                            shapers as *mut c_void,
                            ((n_shapers as usize + 1) * std::mem::size_of::<*const c_char>())
                                as libc::size_t,
                        ) as *mut *const c_char;
                        *shapers.add(n_shapers as usize) = copy_c_slice(cp3, cp2);
                        n_shapers += 1;
                        handled = true;
                    }
                }
            }

            if !handled && !bad_option {
                let i = read_common_features_in(
                    st,
                    cp1,
                    cp2,
                    &mut extend,
                    &mut slant,
                    &mut embolden,
                    &mut letterspace,
                    &mut rgb_value,
                )?;
                if i == 1 {
                    handled = true;
                } else if i == -1 {
                    bad_option = true;
                }
            }

            if !handled && !bad_option && req_engine == b'G' as c_char {
                let mut value = 0;
                let mut tag = 0;
                if readFeatureNumber(cp1, cp2, &mut tag, &mut value)
                    || findGraphiteFeature(engine, cp1, cp2, &mut tag, &mut value)
                {
                    push_hb_feature(&mut features, &mut n_features, tag, value as u32);
                    handled = true;
                }
            }

            if !handled && !bad_option && *cp1 == b'+' as c_char {
                let mut param = 0;
                let tag = read_tag_with_param(cp1.add(1), &mut param);
                if param >= 0 {
                    param += 1;
                }
                push_hb_feature(&mut features, &mut n_features, tag, param as u32);
                handled = true;
            }

            if !handled && !bad_option && *cp1 == b'-' as c_char {
                cp1 = cp1.add(1);
                let tag = hb_tag_from_string(cp1, cp2.offset_from(cp1) as c_int);
                push_hb_feature(&mut features, &mut n_features, tag, 0);
                handled = true;
            }

            if !handled && !bad_option && !str_starts_with(cp1, b"vertical").is_null() {
                cp3 = cp2;
                if *cp3 == b';' as c_char || *cp3 == b':' as c_char || *cp3 == b',' as c_char {
                    cp3 = cp3.sub(1);
                }
                while *cp3 == 0 || *cp3 == b' ' as c_char || *cp3 == b'\t' as c_char {
                    cp3 = cp3.sub(1);
                }
                if *cp3 != 0 {
                    cp3 = cp3.add(1);
                }
                if cp3 == cp1.add(8) {
                    st.fonts.loaded_font_flags =
                        (st.fonts.loaded_font_flags as i32 | FONT_FLAGS_VERTICAL) as c_char;
                    handled = true;
                }
            }

            if !handled || bad_option {
                crate::output::font_feature_warning_in(
                    st,
                    cp1 as *const c_void,
                    cp2.offset_from(cp1) as i32,
                    ptr::null(),
                    0,
                );
            }
            cp1 = cp2;
        }
    }

    if !shapers.is_null() {
        shapers = xrealloc(
            shapers as *mut c_void,
            ((n_shapers as usize + 1) * std::mem::size_of::<*const c_char>()) as libc::size_t,
        ) as *mut *const c_char;
        *shapers.add(n_shapers as usize) = ptr::null();
    }

    if !engine.is_null() {
        deleteLayoutEngine(engine);
    }

    if embolden != 0.0 {
        embolden = (embolden as c_double * Fix2D(scaled_size) / 100.0) as c_float;
    }

    if letterspace != 0.0 {
        st.fonts.loaded_font_letter_space =
            (letterspace as c_double / 100.0 * scaled_size as c_double) as i32;
    }

    if (st.fonts.loaded_font_flags as i32 & FONT_FLAGS_COLORED) == 0 {
        rgb_value = 0x000000FF;
    }

    if (st.fonts.loaded_font_flags as i32 & FONT_FLAGS_VERTICAL) != 0 {
        setFontLayoutDir(font, 1);
    }

    engine = createLayoutEngine(
        font, script, language, features, n_features, shapers, rgb_value, extend, slant, embolden,
    );

    if !features.is_null() {
        libc::free(features as *mut c_void);
    }
    if !shapers.is_null() {
        libc::free(shapers as *mut c_void);
    }
    if !language.is_null() {
        libc::free(language as *mut c_void);
    }
    if !engine.is_null() {
        st.fonts.native_font_type_flag = OTGR_FONT_FLAG as i32;
        Ok(Some(crate::state::FontEngineHandle {
            raw: engine,
            area: OTGR_FONT_FLAG as i32,
        }))
    } else {
        Ok(None)
    }
}

fn name_of_file_mut_ptr(st: &mut EngineState) -> *mut c_char {
    if st.input.name_of_file.is_empty() {
        ptr::null_mut()
    } else {
        st.input.name_of_file.as_mut_ptr()
    }
}

// translated from xetex-ext.c:283 (find_native_font)
pub(crate) unsafe fn find_native_font_in(
    st: &mut EngineState,
    uname: *mut c_char,
    mut scaled_size: i32,
) -> Result<Option<crate::state::FontEngineHandle>, NativeFontAbort> {
    let mut rval = None;
    let name = uname;
    let mut var: *mut c_char = ptr::null_mut();
    let mut feat: *mut c_char = ptr::null_mut();
    let mut end: *mut c_char = ptr::null_mut();
    let mut var_string: *mut c_char = ptr::null_mut();
    let mut feat_string: *mut c_char = ptr::null_mut();
    let mut font: XeTeXFont;
    let mut index = 0;

    st.fonts.loaded_font_mapping = None;
    st.fonts.loaded_font_flags = 0;
    st.fonts.loaded_font_letter_space = 0;

    splitFontName(name, &mut var, &mut feat, &mut end, &mut index);
    let name_string = xmalloc((var.offset_from(name) + 1) as libc::size_t) as *mut c_char;
    ptr::copy_nonoverlapping(name, name_string, var.offset_from(name) as usize);
    *name_string.add(var.offset_from(name) as usize) = 0;

    if feat > var {
        var_string = copy_c_slice(var.add(1), feat);
    }
    if end > feat {
        feat_string = copy_c_slice(feat.add(1), end);
    }

    if *name_string == b'[' as c_char {
        if scaled_size < 0 {
            font = createFontFromFile(name_string.add(1), index, 655360);
            if !font.is_null() {
                let dsize = D2Fix(getDesignSize(font));
                if scaled_size == -1000 {
                    scaled_size = dsize;
                } else {
                    scaled_size =
                        crate::scaled_math::xn_over_d_in(&mut st.errors, dsize, -scaled_size, 1000);
                }
                deleteFont(font);
            }
        }
        font = createFontFromFile(name_string.add(1), index, scaled_size);
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

            rval = load_ot_font(st, ptr::null_mut(), font, scaled_size, feat_string)?;
            if rval.is_none() {
                deleteFont(font);
            }
            if rval.is_some() && st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_TRACING_FONTS) > 0 {
                crate::output::begin_diagnostic_in(st);
                crate::output::print_nl_in(st, b' ' as i32);
                print_c_string_in(st, c"-> ".as_ptr());
                print_c_string_in(st, name_string.add(1));
                crate::output::end_diagnostic_in(st, false);
            }
        }
    } else {
        let font_ref = findFontByName(name_string, var_string, Fix2D(scaled_size));

        if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_TRACING_FONTS) > 0 {
            crate::output::begin_diagnostic_in(st);
            crate::output::print_nl_in(st, b' ' as i32);
            print_c_string_in(st, c"-> ".as_ptr());
            print_c_string_in(st, ttxl_platfont_get_desc(font_ref));
            crate::output::end_diagnostic_in(st, false);
        }

        if !font_ref.is_null() {
            let full_name = getFullName(font_ref);
            let mut name_length = libc::strlen(full_name) as i32;
            if !feat_string.is_null() {
                name_length += libc::strlen(feat_string) as i32 + 1;
            }
            if !var_string.is_null() {
                name_length += libc::strlen(var_string) as i32 + 1;
            }
            st.input.name_of_file.resize((name_length + 1) as usize, 0);
            libc::strcpy(name_of_file_mut_ptr(st), full_name);

            if scaled_size < 0 {
                font = createFont(font_ref, scaled_size);
                if !font.is_null() {
                    let dsize = D2Fix(getDesignSize(font));
                    if scaled_size == -1000 {
                        scaled_size = dsize;
                    } else {
                        scaled_size = crate::scaled_math::xn_over_d_in(
                            &mut st.errors,
                            dsize,
                            -scaled_size,
                            1000,
                        );
                    }
                    deleteFont(font);
                }
            }

            font = createFont(font_ref, scaled_size);
            if !font.is_null() {
                #[cfg(target_os = "macos")]
                {
                    compile_error!("AAT native font loading is still implemented in C on macOS");
                }
                #[cfg(not(target_os = "macos"))]
                {
                    rval = load_ot_font(st, font_ref, font, scaled_size, feat_string)?;
                    if rval.is_none() {
                        deleteFont(font);
                    }
                }
            }

            if !var_string.is_null() && *var_string != 0 {
                libc::strcat(name_of_file_mut_ptr(st), c"/".as_ptr());
                libc::strcat(name_of_file_mut_ptr(st), var_string);
            }
            if !feat_string.is_null() && *feat_string != 0 {
                libc::strcat(name_of_file_mut_ptr(st), c":".as_ptr());
                libc::strcat(name_of_file_mut_ptr(st), feat_string);
            }
            st.input.name_length = libc::strlen(name_of_file_mut_ptr(st)) as i32;
        }
    }

    libc::free(var_string as *mut c_void);
    libc::free(feat_string as *mut c_void);
    libc::free(name_string as *mut c_void);

    Ok(rval)
}

// translated from xetex-ext.c:440 (splitFontName)
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
                    while *name >= b'0' as c_char && *name <= b'9' as c_char {
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
        *end = name;
    } else {
        while *name != 0 {
            if *name == b'/' as c_char && (*var).is_null() && (*feat).is_null() {
                *var = name;
            } else if *name == b':' as c_char && (*feat).is_null() {
                *feat = name;
            }
            name = name.add(1);
        }
        *end = name;
    }

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

// translated from xetex-ext.c:503 (release_font_engine)
pub unsafe fn release_font_engine(engine: *mut c_void, type_flag: c_int) {
    #[cfg(target_os = "macos")]
    if type_flag as u32 == AAT_FONT_FLAG {
        CFRelease(engine);
        return;
    }

    if type_flag as u32 == OTGR_FONT_FLAG {
        deleteLayoutEngine(engine as XeTeXLayoutEngine);
    }
}

// translated from xetex-ext.c:517 (ot_get_font_metrics)
pub unsafe fn ot_get_font_metrics(
    p_engine: *mut c_void,
    ascent: *mut i32,
    descent: *mut i32,
    xheight: *mut i32,
    capheight: *mut i32,
    slant: *mut i32,
) {
    let engine = p_engine as XeTeXLayoutEngine;
    let mut a = 0.0f32;
    let mut d = 0.0f32;

    getAscentAndDescent(engine, &mut a, &mut d);
    *ascent = D2Fix(a as c_double);
    *descent = D2Fix(d as c_double);

    *slant = D2Fix(
        Fix2D(getSlant(getFont(engine))) * getExtendFactor(engine) as c_double
            + getSlantFactor(engine) as c_double,
    );

    getCapAndXHeight(engine, &mut a, &mut d);
    *capheight = D2Fix(a as c_double);
    *xheight = D2Fix(d as c_double);

    if *xheight == 0 {
        let glyph_id = mapCharToGlyph(engine, b'x' as u32);
        if glyph_id != 0 {
            getGlyphHeightDepth(engine, glyph_id, &mut a, &mut d);
            *xheight = D2Fix(a as c_double);
        } else {
            *xheight = *ascent / 2;
        }
    }

    if *capheight == 0 {
        let glyph_id = mapCharToGlyph(engine, b'X' as u32);
        if glyph_id != 0 {
            getGlyphHeightDepth(engine, glyph_id, &mut a, &mut d);
            *capheight = D2Fix(a as c_double);
        } else {
            *capheight = *ascent;
        }
    }
}

// translated from xetex-ext.c:560 (ot_font_get)
pub unsafe fn ot_font_get(what: i32, p_engine: *mut c_void) -> i32 {
    let engine = p_engine as XeTeXLayoutEngine;
    let font_inst = getFont(engine);
    match what {
        XETEX_COUNT_GLYPHS => countGlyphs(font_inst) as i32,
        XETEX_COUNT_FEATURES => countGraphiteFeatures(engine) as i32,
        XETEX_OT_COUNT_SCRIPTS => countScripts(font_inst) as i32,
        _ => 0,
    }
}

// translated from xetex-ext.c:580 (ot_font_get_1)
pub unsafe fn ot_font_get_1(what: i32, p_engine: *mut c_void, param: i32) -> i32 {
    let engine = p_engine as XeTeXLayoutEngine;
    let font_inst = getFont(engine);
    match what {
        XETEX_OT_COUNT_LANGUAGES => countLanguages(font_inst, param as u32) as i32,
        XETEX_OT_SCRIPT_CODE => getIndScript(font_inst, param as c_uint) as i32,
        XETEX_FEATURE_CODE => getGraphiteFeatureCode(engine, param as u32) as i32,
        XETEX_IS_EXCLUSIVE_FEATURE => 1,
        XETEX_COUNT_SELECTORS => countGraphiteFeatureSettings(engine, param as u32) as i32,
        _ => 0,
    }
}

// translated from xetex-ext.c:608 (ot_font_get_2)
pub unsafe fn ot_font_get_2(what: i32, p_engine: *mut c_void, param1: i32, param2: i32) -> i32 {
    let engine = p_engine as XeTeXLayoutEngine;
    let font_inst = getFont(engine);
    match what {
        XETEX_OT_LANGUAGE_CODE => getIndLanguage(font_inst, param1 as u32, param2 as c_uint) as i32,
        XETEX_OT_COUNT_FEATURES => countFeatures(font_inst, param1 as u32, param2 as u32) as i32,
        XETEX_SELECTOR_CODE => {
            getGraphiteFeatureSettingCode(engine, param1 as u32, param2 as u32) as i32
        }
        XETEX_IS_DEFAULT_SELECTOR => {
            (getGraphiteFeatureDefaultSetting(engine, param1 as u32) == param2 as u32) as i32
        }
        _ => 0,
    }
}

// translated from xetex-ext.c:636 (ot_font_get_3)
pub unsafe fn ot_font_get_3(
    what: i32,
    p_engine: *mut c_void,
    param1: i32,
    param2: i32,
    param3: i32,
) -> i32 {
    let engine = p_engine as XeTeXLayoutEngine;
    let font_inst = getFont(engine);
    match what {
        XETEX_OT_FEATURE_CODE => {
            getIndFeature(font_inst, param1 as u32, param2 as u32, param3 as c_uint) as i32
        }
        _ => 0,
    }
}

pub(crate) unsafe fn gr_print_font_name_in(
    st: &mut crate::state::EngineState,
    what: i32,
    p_engine: *mut c_void,
    param1: i32,
    param2: i32,
) {
    let engine = p_engine as XeTeXLayoutEngine;
    let name = match what {
        XETEX_FEATURE_NAME_CODE => getGraphiteFeatureLabel(engine, param1 as u32),
        XETEX_SELECTOR_NAME_CODE => {
            getGraphiteFeatureSettingLabel(engine, param1 as u32, param2 as u32)
        }
        _ => ptr::null_mut(),
    };

    if !name.is_null() {
        crate::output::print_cstr_in(st, name);
        gr_label_destroy(name as *mut c_void);
    }
}

// translated from xetex-ext.c:670 (gr_font_get_named)
pub unsafe fn gr_font_get_named(
    st: &mut crate::state::EngineState,
    what: i32,
    p_engine: *mut c_void,
) -> i32 {
    let mut rval: c_long = -1;
    let engine = p_engine as XeTeXLayoutEngine;
    if what == XETEX_FIND_FEATURE_BY_NAME {
        let np = name_of_file_mut_ptr(st);
        let nl = st.input.name_length;
        rval = findGraphiteFeatureNamed(engine, np, nl);
    }
    rval as i32
}

// translated from xetex-ext.c:683 (gr_font_get_named_1)
pub unsafe fn gr_font_get_named_1(
    st: &mut crate::state::EngineState,
    what: i32,
    p_engine: *mut c_void,
    param: i32,
) -> i32 {
    let mut rval: c_long = -1;
    let engine = p_engine as XeTeXLayoutEngine;
    if what == XETEX_FIND_SELECTOR_BY_NAME {
        let np = name_of_file_mut_ptr(st);
        let nl = st.input.name_length;
        rval = findGraphiteFeatureSettingNamed(engine, param as u32, np, nl);
    }
    rval as i32
}

/// Cold-abort sentinel for the native-font measurement layer.
///
/// Every arm mirrors the former inline `_tt_abort` exactly — no cleanup, sets the
/// surfaced message, history 3 — via [`EngineAbort::TtAbort`]. The `BadFlag` arms
/// are internal-consistency checks (`font_area` is neither AAT nor OTGR), which
/// are *unreachable* for a real native node/font; `LinebreakIterator` /
/// `CouldNotReadMapping` are genuine resource failures and `InputClose` is the
/// deferred serious-internal input-close error.
pub(crate) enum NativeFontAbort {
    /// `_tt_abort("bad native font flag in `<fn>`")`.
    BadFlag(*const c_char),
    /// `_tt_abort("failed to create linebreak iterator")`.
    LinebreakIterator,
    /// `_tt_abort("could not read mapping file")` — the font mapping `.tec` file
    /// could not be read.
    CouldNotReadMapping,
    /// The deferred `ttstub_input_close` serious-internal error.
    InputClose,
}

impl NativeFontAbort {
    /// Map to the unified [`EngineAbort`] terminal — every arm is a
    /// `TtAbort` (the former `_tt_abort`: no cleanup, sets the surfaced message,
    /// returns `HISTORY_FATAL_ERROR`).
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        crate::errors::EngineAbort::TtAbort(match self {
            NativeFontAbort::BadFlag(m) => m,
            NativeFontAbort::LinebreakIterator => c"failed to create linebreak iterator".as_ptr(),
            NativeFontAbort::CouldNotReadMapping => c"could not read mapping file".as_ptr(),
            NativeFontAbort::InputClose => c"serious internal error closing an input file".as_ptr(),
        })
    }
}

// translated from xetex-ext.c:98 (measure_native_node)
pub unsafe fn measure_native_node(
    fonts: &crate::state::FontState,
    mem: &mut crate::state::MemState,
    p: i32,
    use_glyph_metrics: c_int,
) -> Result<(), NativeFontAbort> {
    mem.journal_mark_range(p, 6);
    let node = mem.data.as_mut_ptr().add(p as usize);
    let txt_len = (*node.add(4)).b16.s1 as i32; // native_length
    let txt_ptr = node.add(6) as *mut u16; // node + NATIVE_NODE_SIZE
    let f = (*node.add(4)).b16.s2 as i32; // native_font
    let mut locations: *mut FixedPoint = ptr::null_mut();
    let mut glyph_advances: *mut i32 = ptr::null_mut();
    let mut glyph_info: *mut c_void = ptr::null_mut();
    let mut total_glyph_count = 0i32;

    if fonts.font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        let bidi = ubidi_open();
        let mut error_code = 0i32;

        ubidi_setPara(
            bidi,
            txt_ptr as *const u16,
            txt_len,
            getDefaultDirection(engine) as u8,
            ptr::null_mut(),
            &mut error_code,
        );

        let mut dir = ubidi_getDirection(bidi);
        if dir == UBIDI_MIXED {
            let n_runs = ubidi_countRuns(bidi, &mut error_code);
            let mut width = 0.0f64;
            let mut logical_start = 0i32;
            let mut length = 0i32;

            for run_index in 0..n_runs {
                dir = ubidi_getVisualRun(bidi, run_index, &mut logical_start, &mut length);
                total_glyph_count += layoutChars(
                    engine,
                    txt_ptr,
                    logical_start,
                    length,
                    txt_len,
                    dir == UBIDI_RTL,
                );
            }

            if total_glyph_count > 0 {
                glyph_info = xcalloc(
                    total_glyph_count as libc::size_t,
                    NATIVE_GLYPH_INFO_SIZE as libc::size_t,
                );
                locations = glyph_info as *mut FixedPoint;
                let glyph_ids = locations.add(total_glyph_count as usize) as *mut u16;
                glyph_advances = xcalloc(
                    total_glyph_count as libc::size_t,
                    std::mem::size_of::<i32>() as libc::size_t,
                ) as *mut i32;
                total_glyph_count = 0;

                let mut x = 0.0f64;
                let mut y = 0.0f64;
                for run_index in 0..n_runs {
                    dir = ubidi_getVisualRun(bidi, run_index, &mut logical_start, &mut length);
                    let n_glyphs = layoutChars(
                        engine,
                        txt_ptr,
                        logical_start,
                        length,
                        txt_len,
                        dir == UBIDI_RTL,
                    );

                    let glyphs = xcalloc(
                        n_glyphs as libc::size_t,
                        std::mem::size_of::<u32>() as libc::size_t,
                    ) as *mut u32;
                    let positions = xcalloc(
                        (n_glyphs + 1) as libc::size_t,
                        std::mem::size_of::<FloatPoint>() as libc::size_t,
                    ) as *mut FloatPoint;
                    let advances = xcalloc(
                        n_glyphs as libc::size_t,
                        std::mem::size_of::<f32>() as libc::size_t,
                    ) as *mut f32;

                    getGlyphs(engine, glyphs);
                    getGlyphAdvances(engine, advances);
                    getGlyphPositions(engine, positions);

                    for i in 0..n_glyphs as usize {
                        let out_index = total_glyph_count as usize;
                        *glyph_ids.add(out_index) = *glyphs.add(i) as u16;
                        (*locations.add(out_index)).x =
                            D2Fix((*positions.add(i)).x as c_double + x);
                        (*locations.add(out_index)).y =
                            D2Fix((*positions.add(i)).y as c_double + y);
                        *glyph_advances.add(out_index) = D2Fix(*advances.add(i) as c_double);
                        total_glyph_count += 1;
                    }

                    x += (*positions.add(n_glyphs as usize)).x as f64;
                    y += (*positions.add(n_glyphs as usize)).y as f64;

                    libc::free(glyphs as *mut c_void);
                    libc::free(positions as *mut c_void);
                    libc::free(advances as *mut c_void);
                }
                width = x;
            }

            (*node.add(1)).b32.s1 = D2Fix(width);
            (*node.add(4)).b16.s0 = total_glyph_count as u16;
        } else {
            let mut width = 0.0f64;
            total_glyph_count = layoutChars(engine, txt_ptr, 0, txt_len, txt_len, dir == UBIDI_RTL);

            let glyphs = xcalloc(
                total_glyph_count as libc::size_t,
                std::mem::size_of::<u32>() as libc::size_t,
            ) as *mut u32;
            let positions = xcalloc(
                (total_glyph_count + 1) as libc::size_t,
                std::mem::size_of::<FloatPoint>() as libc::size_t,
            ) as *mut FloatPoint;
            let advances = xcalloc(
                total_glyph_count as libc::size_t,
                std::mem::size_of::<f32>() as libc::size_t,
            ) as *mut f32;

            getGlyphs(engine, glyphs);
            getGlyphAdvances(engine, advances);
            getGlyphPositions(engine, positions);

            if total_glyph_count > 0 {
                glyph_info = xcalloc(
                    total_glyph_count as libc::size_t,
                    NATIVE_GLYPH_INFO_SIZE as libc::size_t,
                );
                locations = glyph_info as *mut FixedPoint;
                let glyph_ids = locations.add(total_glyph_count as usize) as *mut u16;
                glyph_advances = xcalloc(
                    total_glyph_count as libc::size_t,
                    std::mem::size_of::<i32>() as libc::size_t,
                ) as *mut i32;

                for i in 0..total_glyph_count as usize {
                    *glyph_ids.add(i) = *glyphs.add(i) as u16;
                    *glyph_advances.add(i) = D2Fix(*advances.add(i) as c_double);
                    (*locations.add(i)).x = D2Fix((*positions.add(i)).x as c_double);
                    (*locations.add(i)).y = D2Fix((*positions.add(i)).y as c_double);
                }
                width = (*positions.add(total_glyph_count as usize)).x as f64;
            }

            (*node.add(1)).b32.s1 = D2Fix(width);
            (*node.add(4)).b16.s0 = total_glyph_count as u16;

            libc::free(glyphs as *mut c_void);
            libc::free(positions as *mut c_void);
            libc::free(advances as *mut c_void);
        }

        ubidi_close(bidi);

        let letter_space = fonts.font_letter_space_entry(f);
        if letter_space != 0 {
            let mut ls_delta = 0i32;
            for i in 0..total_glyph_count as usize {
                if *glyph_advances.add(i) == 0 && ls_delta != 0 {
                    ls_delta -= letter_space;
                }
                (*locations.add(i)).x += ls_delta;
                ls_delta += letter_space;
            }
            if ls_delta != 0 {
                ls_delta -= letter_space;
                (*node.add(1)).b32.s1 += ls_delta;
            }
        }
        libc::free(glyph_advances as *mut c_void);
    } else {
        return Err(NativeFontAbort::BadFlag(
            c"bad native font flag in `measure_native_node`".as_ptr(),
        ));
    }

    if use_glyph_metrics == 0 || (*node.add(4)).b16.s0 == 0 {
        (*node.add(3)).b32.s1 = fonts.height_base_entry(f);
        (*node.add(2)).b32.s1 = fonts.depth_base_entry(f);
    } else {
        let glyph_count = (*node.add(4)).b16.s0 as usize;
        let glyph_ids = locations.add(glyph_count) as *mut u16;
        let mut y_min = 65536.0f32;
        let mut y_max = -65536.0f32;

        for i in 0..glyph_count {
            let y = Fix2D(-(*locations.add(i)).y) as f32;
            let mut bbox = GlyphBBox {
                x_min: 0.0,
                y_min: 0.0,
                x_max: 0.0,
                y_max: 0.0,
            };
            let glyph_id = *glyph_ids.add(i);

            if getCachedGlyphBBox(f as u16, glyph_id, &mut bbox) == 0 {
                getGlyphBounds(
                    fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine,
                    glyph_id as u32,
                    &mut bbox,
                );
                cacheGlyphBBox(f as u16, glyph_id, &bbox);
            }

            let ht = bbox.y_max;
            let dp = -bbox.y_min;
            if y + ht > y_max {
                y_max = y + ht;
            }
            if y - dp < y_min {
                y_min = y - dp;
            }
        }

        (*node.add(3)).b32.s1 = D2Fix(y_max as c_double);
        (*node.add(2)).b32.s1 = -D2Fix(y_min as c_double);
    }
    let old_handle = mem.b32s0(p + 5);
    if old_handle != 0 {
        mem.glyph_blobs.release(old_handle);
    }
    let glyph_count = (*node.add(4)).b16.s0 as usize;
    let handle = if glyph_count == 0 {
        0
    } else {
        let glyph_ids = locations.add(glyph_count) as *const u16;
        let blob = GlyphBlob {
            locations: std::slice::from_raw_parts(locations, glyph_count).to_vec(),
            gids: std::slice::from_raw_parts(glyph_ids, glyph_count).to_vec(),
        };
        mem.glyph_blobs.insert(blob)
    };
    libc::free(glyph_info);
    mem.set_b32s0(p + 5, handle);
    mem.set_b32s1(p + 5, 0);
    Ok(())
}

// translated from xetex-ext.c:921 (real_get_native_glyph)
// glyph info buffer layout: [FixedPoint(8b) * glyph_count][u16 * glyph_count]
pub fn real_get_native_glyph(mem: &crate::state::MemState, p: i32, index: u32) -> u16 {
    let glyph_count = mem.b16s0(p + 4) as usize;
    if index as usize >= glyph_count {
        0
    } else {
        mem.glyph_blobs.get(mem.b32s0(p + 5)).gids[index as usize]
    }
}

// translated from xetex-ext.c:1183 (real_get_native_italic_correction)
pub unsafe fn real_get_native_italic_correction(
    fonts: &crate::state::FontState,
    mem: &crate::state::MemState,
    p: i32,
) -> i32 {
    let f = mem.b16s2(p + 4) as i32; // native_font
    let n = mem.b16s0(p + 4) as usize; // native_glyph_count
    if n > 0 {
        let last_glyph = mem.glyph_blobs.get(mem.b32s0(p + 5)).gids[n - 1];
        if fonts.font_area_entry(f) as u32 == OTGR_FONT_FLAG {
            return D2Fix(getGlyphItalCorr(
                fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine,
                last_glyph as u32,
            ) as c_double)
                + fonts.font_letter_space_entry(f);
        }
    }
    0
}

// translated from xetex-ext.c:1207 (real_get_native_glyph_italic_correction)
pub unsafe fn real_get_native_glyph_italic_correction(
    fonts: &crate::state::FontState,
    mem: &crate::state::MemState,
    p: i32,
) -> i32 {
    let gid = mem.b16s1(p + 4); // native_glyph = native_length
    let f = mem.b16s2(p + 4) as i32; // native_font
    if fonts.font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        return D2Fix(getGlyphItalCorr(
            fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine,
            gid as u32,
        ) as c_double);
    }
    0
}

// translated from xetex-ext.c:1224 (measure_native_glyph)
pub unsafe fn measure_native_glyph(
    fonts: &crate::state::FontState,
    mem: &mut crate::state::MemState,
    p: i32,
    use_glyph_metrics: c_int,
) -> Result<(), NativeFontAbort> {
    let gid = mem.b16s1(p + 4); // native_glyph = native_length
    let f = mem.b16s2(p + 4) as i32; // native_font
    let mut ht = 0.0f32;
    let mut dp = 0.0f32;

    if fonts.font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        let font_inst = getFont(engine);
        // node_width(node) = node[1].b32.s1
        mem.set_b32s1(
            p + 1,
            D2Fix(getGlyphWidth(font_inst, gid as u32) as c_double),
        );
        if use_glyph_metrics != 0 {
            getGlyphHeightDepth(engine, gid as u32, &mut ht, &mut dp);
        }
    } else {
        return Err(NativeFontAbort::BadFlag(
            c"bad native font flag in `measure_native_glyph`".as_ptr(),
        ));
    }

    if use_glyph_metrics != 0 {
        mem.set_b32s1(p + 3, D2Fix(ht as c_double)); // node_height
        mem.set_b32s1(p + 2, D2Fix(dp as c_double)); // node_depth
    } else {
        mem.set_b32s1(p + 3, fonts.height_base_entry(f)); // node_height = height_base[f]
        mem.set_b32s1(p + 2, fonts.depth_base_entry(f)); // node_depth = depth_base[f]
    }
    Ok(())
}

// translated from xetex-ext.c:1559 (real_get_native_word_cp)
pub unsafe fn real_get_native_word_cp(mem: &crate::state::MemState, p: i32, side: c_int) -> i32 {
    let glyph_count = mem.b16s0(p + 4) as usize;
    let f = mem.b16s2(p + 4) as i32; // native_font

    if glyph_count == 0 {
        return 0;
    }

    let actual_glyph = match side {
        LEFT_SIDE => mem.glyph_blobs.get(mem.b32s0(p + 5)).gids[0],
        RIGHT_SIDE => mem.glyph_blobs.get(mem.b32s0(p + 5)).gids[glyph_count - 1],
        _ => unreachable!(), // C: assert(0) — should not reach this point
    };
    get_cp_code(f, actual_glyph as u32, side)
}

// translated from xetex-ext.c:777 (snap_zone) — private helper
unsafe fn snap_zone(value: *mut i32, snap_value: i32, fuzz: i32) {
    let difference = *value - snap_value;
    if difference <= fuzz && difference >= -fuzz {
        *value = snap_value;
    }
}

// translated from xetex-ext.c:785 (get_native_char_height_depth)
pub unsafe fn get_native_char_height_depth(
    fonts: &crate::state::FontState,
    font: i32,
    ch: i32,
    height: *mut i32,
    depth: *mut i32,
) -> Result<(), NativeFontAbort> {
    let mut ht = 0.0f32;
    let mut dp = 0.0f32;

    if fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine;
        let gid = mapCharToGlyph(engine, ch as u32);
        getGlyphHeightDepth(engine, gid, &mut ht, &mut dp);
    } else {
        return Err(NativeFontAbort::BadFlag(
            c"bad native font flag in `get_native_char_height_depth`".as_ptr(),
        ));
    }

    *height = D2Fix(ht as c_double);
    *depth = D2Fix(dp as c_double);

    // snap to known zones (baseline, x-height, cap-height) if within 4% of em-size
    let fuzz = fonts.font_info_b32s1(6 + fonts.param_base(font)) / 25;
    snap_zone(depth, 0, fuzz);
    snap_zone(height, 0, fuzz);
    snap_zone(
        height,
        fonts.font_info_b32s1(5 + fonts.param_base(font)),
        fuzz,
    );
    snap_zone(
        height,
        fonts.font_info_b32s1(8 + fonts.param_base(font)),
        fuzz,
    );
    Ok(())
}

// translated from xetex-ext.c:822 (getnativecharht)
pub unsafe fn getnativecharht(
    fonts: &crate::state::FontState,
    f: i32,
    c: i32,
) -> Result<i32, NativeFontAbort> {
    let mut h = 0i32;
    let mut d = 0i32;
    get_native_char_height_depth(fonts, f, c, &mut h, &mut d)?;
    Ok(h)
}

// translated from xetex-ext.c:830 (getnativechardp)
pub unsafe fn getnativechardp(
    fonts: &crate::state::FontState,
    f: i32,
    c: i32,
) -> Result<i32, NativeFontAbort> {
    let mut h = 0i32;
    let mut d = 0i32;
    get_native_char_height_depth(fonts, f, c, &mut h, &mut d)?;
    Ok(d)
}

// translated from xetex-ext.c:838 (get_native_char_sidebearings)
pub unsafe fn get_native_char_sidebearings(
    fonts: &crate::state::FontState,
    font: i32,
    ch: i32,
    lsb: *mut i32,
    rsb: *mut i32,
) -> Result<(), NativeFontAbort> {
    let mut l = 0.0f32;
    let mut r = 0.0f32;

    if fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine;
        let gid = mapCharToGlyph(engine, ch as u32);
        getGlyphSidebearings(engine, gid, &mut l, &mut r);
    } else {
        return Err(NativeFontAbort::BadFlag(
            c"bad native font flag in `get_native_char_side_bearings`".as_ptr(),
        ));
    }

    *lsb = D2Fix(l as c_double);
    *rsb = D2Fix(r as c_double);
    Ok(())
}

// translated from xetex-ext.c:862 (get_glyph_bounds)
pub unsafe fn get_glyph_bounds(
    fonts: &crate::state::FontState,
    font: i32,
    edge: i32,
    gid: i32,
) -> Result<i32, NativeFontAbort> {
    // edge codes: 1,2,3,4 => L T R B
    let mut a = 0.0f32;
    let mut b = 0.0f32;

    if fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine;
        if edge & 1 != 0 {
            getGlyphSidebearings(engine, gid as u32, &mut a, &mut b);
        } else {
            getGlyphHeightDepth(engine, gid as u32, &mut a, &mut b);
        }
    } else {
        return Err(NativeFontAbort::BadFlag(
            c"bad native font flag in `get_glyph_bounds`".as_ptr(),
        ));
    }
    Ok(D2Fix(if edge <= 2 { a } else { b } as c_double))
}

// translated from xetex-ext.c:889 (getnativecharic)
pub unsafe fn getnativecharic(
    fonts: &crate::state::FontState,
    f: i32,
    c: i32,
) -> Result<i32, NativeFontAbort> {
    let mut lsb = 0i32;
    let mut rsb = 0i32;
    get_native_char_sidebearings(fonts, f, c, &mut lsb, &mut rsb)?;
    Ok(if rsb < 0 {
        fonts.font_letter_space_entry(f) - rsb
    } else {
        fonts.font_letter_space_entry(f)
    })
}

// translated from xetex-ext.c:900 (getnativecharwd)
pub unsafe fn getnativecharwd(
    fonts: &crate::state::FontState,
    f: i32,
    c: i32,
) -> Result<i32, NativeFontAbort> {
    if fonts.font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        let gid = mapCharToGlyph(engine, c as u32);
        Ok(D2Fix(getGlyphWidthFromEngine(engine, gid) as c_double))
    } else {
        Err(NativeFontAbort::BadFlag(
            c"bad native font flag in `get_native_char_wd`".as_ptr(),
        ))
    }
}

// translated from xetex-ext.c:1261 (map_char_to_glyph)
pub unsafe fn map_char_to_glyph(
    fonts: &crate::state::FontState,
    font: i32,
    ch: i32,
) -> Result<i32, NativeFontAbort> {
    if ch > 0x10ffff || (0xd800..=0xdfff).contains(&ch) {
        return Ok(0);
    }
    if fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        Ok(mapCharToGlyph(
            fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine,
            ch as u32,
        ) as i32)
    } else {
        Err(NativeFontAbort::BadFlag(
            c"bad native font flag in `map_char_to_glyph`".as_ptr(),
        ))
    }
}

// translated from xetex-ext.c:1278 (map_glyph_to_index)
pub unsafe fn map_glyph_to_index(
    fonts: &crate::state::FontState,
    font: i32,
    name: *const c_char,
) -> Result<i32, NativeFontAbort> {
    if fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        Ok(mapGlyphToIndex(
            fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine,
            name,
        ))
    } else {
        Err(NativeFontAbort::BadFlag(
            c"bad native font flag in `map_glyph_to_index`".as_ptr(),
        ))
    }
}

// translated from xetex-ext.c:1293 (get_font_char_range)
pub unsafe fn get_font_char_range(
    fonts: &crate::state::FontState,
    font: i32,
    first: i32,
) -> Result<i32, NativeFontAbort> {
    if fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        Ok(getFontCharRange(
            fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine,
            first,
        ))
    } else {
        Err(NativeFontAbort::BadFlag(
            c"bad native font flag in `get_font_char_range'`".as_ptr(),
        ))
    }
}

pub(crate) unsafe fn linebreak_start_in(
    st: &mut EngineState,
    f: c_int,
    locale_str_num: i32,
    text: *mut u16,
    text_length: i32,
) -> Result<(), NativeFontAbort> {
    let mut status: i32 = 0; // U_ZERO_ERROR
    let locale = crate::texmfmp::gettexstring_in(&st.strings, locale_str_num);

    let graphite_engine = if st.fonts.font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        Some(st.fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine)
    } else {
        None
    };
    if let Some(engine) = graphite_engine {
        if libc::strcmp(locale, c"G".as_ptr()) == 0
            && initGraphiteBreaking(engine, text as *const u16, text_length as u32)
        {
            return Ok(()); // locale intentionally not freed (matches C behavior)
        }
    }

    if locale_str_num != st.fonts.brk_locale_str_num {
        st.fonts.brk_iter = None;
    }

    if st.fonts.brk_iter.is_none() {
        let mut raw = ubrk_open(UBRK_LINE, locale, ptr::null(), 0, &mut status);
        if status > 0 {
            crate::output::begin_diagnostic_in(st);
            crate::output::print_nl_in(st, b'E' as i32);
            crate::output::print_cstr_in(st, c"rror ".as_ptr());
            crate::output::print_int_in(st, status);
            crate::output::print_cstr_in(st, c" creating linebreak iterator for locale `".as_ptr());
            crate::output::print_cstr_in(st, locale);
            crate::output::print_cstr_in(st, c"'; trying default locale `en_us'.".as_ptr());
            crate::output::end_diagnostic_in(st, true);
            if !raw.is_null() {
                ubrk_close(raw);
            }
            status = 0;
            raw = ubrk_open(UBRK_LINE, c"en_us".as_ptr(), ptr::null(), 0, &mut status);
        }
        libc::free(locale as *mut c_void);
        st.fonts.brk_locale_str_num = locale_str_num;
        if !raw.is_null() {
            st.fonts.brk_iter = Some(crate::state::LinebreakHandle { raw });
        }
    }

    let iterator = st.fonts.linebreak_iterator_ptr();
    if iterator.is_null() {
        return Err(NativeFontAbort::LinebreakIterator);
    }

    ubrk_setText(iterator, text as *const u16, text_length, &mut status);
    Ok(())
}

pub(crate) fn release_linebreak_state(fonts: &mut crate::state::FontState) {
    fonts.brk_iter = None;
    fonts.brk_locale_str_num = 0;
    fonts.saved_mapping_name = None;
}

pub(crate) unsafe fn close_linebreak_iterator(iterator: *mut c_void) {
    ubrk_close(iterator);
}

// translated from xetex-ext.c:116 (linebreak_next)
pub unsafe fn linebreak_next(fonts: &crate::state::FontState, f: c_int) -> c_int {
    let iterator = fonts.linebreak_iterator_ptr();
    if !iterator.is_null() {
        ubrk_next(iterator)
    } else {
        let engine = fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        findNextGraphiteBreak(engine)
    }
}

// translated from xetex-ext.c:584 (make_font_def)
pub unsafe fn make_font_def(
    fonts: &mut crate::state::FontState,
    f: i32,
) -> Result<c_int, NativeFontAbort> {
    let mut flags: u16 = 0;
    let rgba: u32;
    let size: i32;
    let filename: *const c_char;
    let index: u32;
    let extend: f32;
    let slant: f32;
    let embolden: f32;

    if fonts.font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        let mut idx: u32 = 0;
        filename = getFontFilename(engine, &mut idx);
        assert!(!filename.is_null());
        index = idx;

        rgba = getRgbValue(engine);
        if (fonts.font_flags_entry(f) & FONT_FLAGS_VERTICAL) != 0 {
            flags |= XDV_FLAG_VERTICAL;
        }

        extend = getExtendFactor(engine);
        slant = getSlantFactor(engine);
        embolden = getEmboldenFactor(engine);
        size = D2Fix(getPointSize(engine) as c_double);
    } else {
        return Err(NativeFontAbort::BadFlag(
            c"bad native font flag in `make_font_def`".as_ptr(),
        ));
    }

    let filename_len = libc::strlen(filename) as u8;

    let mut font_def_length: usize = 4 + 2 + 1 + filename_len as usize + 4;

    if (fonts.font_flags_entry(f) & FONT_FLAGS_COLORED) != 0 {
        font_def_length += 4;
        flags |= XDV_FLAG_COLORED;
    }
    if extend != 1.0 {
        font_def_length += 4;
        flags |= XDV_FLAG_EXTEND;
    }
    if slant != 0.0 {
        font_def_length += 4;
        flags |= XDV_FLAG_SLANT;
    }
    if embolden != 0.0 {
        font_def_length += 4;
        flags |= XDV_FLAG_EMBOLDEN;
    }

    let needed = font_def_length;
    if fonts.xdv_buffer.len() < needed {
        fonts.xdv_buffer.resize(needed, 0);
    }

    let xdv_ptr = fonts.xdv_buffer.as_ptr() as *mut u8;
    let mut cp = xdv_ptr;

    std::ptr::write_unaligned(cp as *mut u32, (size as u32).to_be());
    cp = cp.add(4);

    std::ptr::write_unaligned(cp as *mut u16, flags.to_be());
    cp = cp.add(2);

    *cp = filename_len;
    cp = cp.add(1);
    libc::memcpy(
        cp as *mut c_void,
        filename as *const c_void,
        filename_len as libc::size_t,
    );
    cp = cp.add(filename_len as usize);

    std::ptr::write_unaligned(cp as *mut u32, index.to_be());
    cp = cp.add(4);

    if (fonts.font_flags_entry(f) & FONT_FLAGS_COLORED) != 0 {
        std::ptr::write_unaligned(cp as *mut u32, rgba.to_be());
        cp = cp.add(4);
    }
    if flags & XDV_FLAG_EXTEND != 0 {
        std::ptr::write_unaligned(cp as *mut u32, (D2Fix(extend as c_double) as u32).to_be());
        cp = cp.add(4);
    }
    if flags & XDV_FLAG_SLANT != 0 {
        std::ptr::write_unaligned(cp as *mut u32, (D2Fix(slant as c_double) as u32).to_be());
        cp = cp.add(4);
    }
    if flags & XDV_FLAG_EMBOLDEN != 0 {
        std::ptr::write_unaligned(cp as *mut u32, (D2Fix(embolden as c_double) as u32).to_be());
    }

    freeFontFilename(filename);

    Ok(font_def_length as c_int)
}

// translated from xetex-ext.c:738 (apply_mapping)
pub unsafe fn apply_mapping(
    fonts: &mut crate::state::FontState,
    p_cnv: *mut c_void,
    txt_ptr: *mut u16,
    txt_len: c_int,
) -> c_int {
    const K_STATUS_NO_ERROR: libc::c_long = 0;
    const K_STATUS_OUTPUT_BUFFER_FULL: libc::c_long = 1;
    let txt_bytes = txt_len as u32 * 2; // sizeof(UniChar) == 2

    let mut needed = (txt_bytes + 32) as usize;
    let needed_u16 = needed / 2;
    if fonts.mapped_text.len() < needed_u16 {
        fonts.mapped_text.resize(needed_u16, 0);
    }

    loop {
        let mut in_used: u32 = 0;
        let mut out_used: u32 = 0;
        let mapped_ptr = fonts.mapped_text.as_mut_ptr() as *mut u8;
        let mapped_len_bytes = (fonts.mapped_text.len() * 2) as u32;
        let status = TECkit_ConvertBuffer(
            p_cnv,
            txt_ptr as *const u8,
            txt_bytes,
            &mut in_used,
            mapped_ptr,
            mapped_len_bytes,
            &mut out_used,
            1,
        );
        TECkit_ResetConverter(p_cnv);

        match status {
            K_STATUS_NO_ERROR => return (out_used / 2) as c_int,
            K_STATUS_OUTPUT_BUFFER_FULL => {
                needed += (txt_bytes + 32) as usize;
                let needed_u16 = needed / 2;
                fonts.mapped_text.resize(needed_u16, 0);
            }
            _ => return 0,
        }
    }
}

// translated from xetex-ext.c:776 (store_justified_native_glyphs)
pub unsafe fn store_justified_native_glyphs(
    fonts: &crate::state::FontState,
    mem: &mut crate::state::MemState,
    p: i32,
) -> Result<(), NativeFontAbort> {
    let f = mem.b16s2(p + 4) as i32;

    let saved_width = mem.b32s1(p + 1);
    measure_native_node(fonts, mem, p, 0)?;

    if mem.b32s1(p + 1) != saved_width {
        let just_amount = Fix2D(saved_width - mem.b32s1(p + 1));

        let glyph_count = mem.b16s0(p + 4) as usize;
        let handle = mem.b32s0(p + 5);
        let glyphs = mem.glyph_blobs.get_mut(handle);

        let space_glyph = map_char_to_glyph(fonts, f, b' ' as i32)?;
        let mut space_count = 0i32;
        for i in 0..glyph_count {
            if glyphs.gids[i] as i32 == space_glyph {
                space_count += 1;
            }
        }

        if space_count > 0 {
            let mut adjustment = 0.0f64;
            let mut space_index = 0i32;
            for i in 0..glyph_count {
                glyphs.locations[i].x = D2Fix(Fix2D(glyphs.locations[i].x) + adjustment);
                if glyphs.gids[i] as i32 == space_glyph {
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

        mem.set_b32s1(p + 1, saved_width);
    }
    Ok(())
}

// translated from xetex-ext.c:1027 (aat_get_font_metrics)
pub fn aat_get_font_metrics(
    _attributes: *mut c_void,
    _ascent: *mut i32,
    _descent: *mut i32,
    _xheight: *mut i32,
    _capheight: *mut i32,
    _slant: *mut i32,
) {
}

// translated from xetex-ext.c:1041 (aat_font_get)
pub fn aat_font_get(_what: c_int, _attributes: *mut c_void) -> c_int {
    -1
}

// translated from xetex-ext.c:1067 (aat_font_get_1)
pub fn aat_font_get_1(_what: c_int, _attributes: *mut c_void, _param: c_int) -> c_int {
    -1
}

// translated from xetex-ext.c:1125 (aat_font_get_2)
pub fn aat_font_get_2(
    _what: c_int,
    _attributes: *mut c_void,
    _param1: c_int,
    _param2: c_int,
) -> c_int {
    -1
}

// translated from xetex-ext.c:1168 (aat_font_get_named)
pub fn aat_font_get_named(_what: c_int, _attributes: *mut c_void) -> c_int {
    -1
}

// translated from xetex-ext.c:1193 (aat_font_get_named_1)
pub fn aat_font_get_named_1(_what: c_int, _attributes: *mut c_void, _param: c_int) -> c_int {
    -1
}

// translated from xetex-ext.c:1218 (aat_print_font_name)
pub fn aat_print_font_name(_what: c_int, _attributes: *mut c_void, _param1: c_int, _param2: c_int) {
}

pub(crate) unsafe fn print_glyph_name_in(
    st: &mut crate::state::EngineState,
    font: i32,
    gid: i32,
) -> Result<(), NativeFontAbort> {
    let mut len: c_int = 0;
    let engine = if st.fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        st.fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine
    } else {
        return Err(NativeFontAbort::BadFlag(
            c"bad native font flag in `print_glyph_name`".as_ptr(),
        ));
    };
    let s = getGlyphName(getFont(engine), gid as u16, &mut len);

    let mut p = s;
    while len > 0 {
        crate::output::print_char_in(st, *p as i32);
        p = p.add(1);
        len -= 1;
    }
    if !s.is_null() {
        freeGlyphName(s as *mut c_char);
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

// translated from xetex-xetex0.c:11376 (append_src_special)
pub(crate) fn append_src_special_in(
    st: &mut crate::state::EngineState,
) -> Result<(), InsertSrcAbort> {
    let fname = st.input.source_filename_stack[st.input.in_open as usize];
    if fname > 0 && crate::texmfmp::is_new_source_in(st, fname, st.mem.line) {
        crate::mem::new_whatsit(
            &mut st.input.cur_list,
            &mut st.mem,
            crate::node::WhatsitSubtype::Special as i16,
            WRITE_NODE_SIZE,
        )
        .ok_or(InsertSrcAbort::MainMemory)?;
        st.mem.set_b32s0(st.input.cur_list.tail + 1, 0);
        let d = crate::mem::get_avail(&mut st.mem).ok_or(InsertSrcAbort::MainMemory)?;
        st.scanner.def_ref = d;
        d.set_info(&mut st.mem, crate::mem::TEX_NULL);
        let b = crate::texmfmp::make_src_special_in(st, fname, st.mem.line)
            .map_err(|()| InsertSrcAbort::MakeSrcPool)?;
        match crate::scan::str_toks_cat_in(&mut st.mem, &mut st.strings, b, 0) {
            Ok(_) => {}
            Err(crate::scan::StrToksAbort::PoolOverflow) => {
                return Err(InsertSrcAbort::StrToksPool)
            }
            Err(crate::scan::StrToksAbort::MainMemoryOverflow) => {
                return Err(InsertSrcAbort::MainMemory)
            }
        };
        let th = crate::units::MemPtr::from_raw(TEMP_HEAD).link(&st.mem);
        d.set_link(&mut st.mem, th);
        st.mem.set_b32s1(st.input.cur_list.tail + 1, d.0);
        crate::texmfmp::remember_source_info_in(st, fname, st.mem.line);
    }
    Ok(())
}
// translated from xetex-xetex0.c:11045 (new_write_whatsit)
pub(crate) enum NewWriteWhatsitAbort {
    Scanner(crate::scan::SccAbort),
    MainMemory,
}

impl From<crate::scan::SccAbort> for NewWriteWhatsitAbort {
    fn from(e: crate::scan::SccAbort) -> Self {
        NewWriteWhatsitAbort::Scanner(e)
    }
}

pub(crate) fn new_write_whatsit_in(
    st: &mut EngineState,
    w: i16,
) -> Result<(), NewWriteWhatsitAbort> {
    crate::mem::new_whatsit(
        &mut st.input.cur_list,
        &mut st.mem,
        st.scanner.cur_chr as i16,
        w,
    )
    .ok_or(NewWriteWhatsitAbort::MainMemory)?;
    if w != WRITE_NODE_SIZE {
        crate::scan::scan_four_bit_int_in(st)?;
    } else {
        crate::scan::scan_int_in(st)?;
        if st.scanner.cur_val < 0 {
            st.scanner.cur_val = 17;
        } else if st.scanner.cur_val > 15 && st.scanner.cur_val != 18 {
            st.scanner.cur_val = 16;
        }
    }
    st.mem
        .set_b32s0(st.input.cur_list.tail.0 + 1, st.scanner.cur_val);
    Ok(())
}
// translated from xetex-xetex0.c:9978 (insert_src_special)
/// Cold-path abort raised by [`insert_src_special_in`]. Surfaces the
/// `make_src_special`/`str_toks` pool overflow, the `get_avail` main-memory
/// overflow, and the `begin_token_list` input abort.
pub(crate) enum InsertSrcAbort {
    MakeSrcPool,
    StrToksPool,
    MainMemory,
    Input(crate::input::InputAbort),
}

impl InsertSrcAbort {
    /// Map to the unified [`EngineAbort`] terminal. `MakeSrcPool` is the
    /// `fire_make_src_pool_overflow` `_tt_abort`; `MainMemory` runs `runaway`.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            InsertSrcAbort::MakeSrcPool => {
                crate::errors::EngineAbort::TtAbort(c"string pool overflow".as_ptr())
            }
            InsertSrcAbort::StrToksPool => crate::errors::EngineAbort::PoolOverflow,
            InsertSrcAbort::MainMemory => crate::mem::main_memory_overflow_abort(),
            InsertSrcAbort::Input(e) => e.into_engine_abort(),
        }
    }
}

impl From<crate::input::InputAbort> for InsertSrcAbort {
    fn from(e: crate::input::InputAbort) -> Self {
        InsertSrcAbort::Input(e)
    }
}

/// Builds the `\special{src:<line> <file>}` token list and inserts it. Cold
/// aborts ride [`InsertSrcAbort`].
///
/// translated from xetex-xetex0.c:9966 (insert_src_special)
pub(crate) fn insert_src_special_in(
    st: &mut crate::state::EngineState,
) -> Result<(), InsertSrcAbort> {
    const FROZEN_SPECIAL: i32 = 2_243_226 + 10; // FROZEN_CONTROL_SEQUENCE + 10
    const RIGHT_BRACE_TOKEN: i32 = 0x400000; // xetex-constants.h:351

    let sfn = st.input.source_filename_stack[st.input.in_open as usize];
    if sfn > 0 && crate::texmfmp::is_new_source_in(st, sfn, st.mem.line) {
        let toklist = crate::mem::get_avail(&mut st.mem).ok_or(InsertSrcAbort::MainMemory)?;
        let mut p = toklist;
        p.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_SPECIAL);
        let nxt = crate::mem::get_avail(&mut st.mem).ok_or(InsertSrcAbort::MainMemory)?;
        p.set_link(&mut st.mem, nxt);
        p = nxt;
        p.set_info(&mut st.mem, LEFT_BRACE_TOKEN + b'{' as i32);
        let b = crate::texmfmp::make_src_special_in(st, sfn, st.mem.line)
            .map_err(|()| InsertSrcAbort::MakeSrcPool)?;
        let q = match crate::scan::str_toks_cat_in(&mut st.mem, &mut st.strings, b, 0) {
            Ok(q) => q,
            Err(crate::scan::StrToksAbort::PoolOverflow) => {
                return Err(InsertSrcAbort::StrToksPool)
            }
            Err(crate::scan::StrToksAbort::MainMemoryOverflow) => {
                return Err(InsertSrcAbort::MainMemory)
            }
        };
        let th = crate::units::MemPtr::from_raw(TEMP_HEAD).link(&st.mem);
        p.set_link(&mut st.mem, th);
        p = q;
        let nxt = crate::mem::get_avail(&mut st.mem).ok_or(InsertSrcAbort::MainMemory)?;
        p.set_link(&mut st.mem, nxt);
        p = nxt;
        p.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
        crate::input::begin_token_list_in(st, toklist.0, INSERTED)?;
        crate::texmfmp::remember_source_info_in(st, sfn, st.mem.line);
    }
    Ok(())
}
pub(crate) unsafe fn tt_insert_special_in(
    st: &mut crate::state::EngineState,
    ascii_text: *const libc::c_char,
) -> Result<(), crate::main_control::MainControlAbort> {
    const FROZEN_SPECIAL: i32 = 2_243_226 + 10; // FROZEN_CONTROL_SEQUENCE + 10
    const RIGHT_BRACE_TOKEN: i32 = 0x400000; // xetex-constants.h:351

    let start_pool = st.strings.pool_ptr;
    let text = std::ffi::CStr::from_ptr(ascii_text).to_bytes();
    if st.strings.pool_ptr + text.len() as i32 >= st.strings.pool_size {
        return Err(crate::input::InputAbort::Overflow {
            what: c"string pool overflow".as_ptr(),
            n: 0,
        }
        .into());
    }
    for &b in text {
        st.strings.str_pool[st.strings.pool_ptr as usize] = b as u16;
        st.strings.pool_ptr += 1;
    }

    let toklist_start =
        crate::mem::get_avail(&mut st.mem).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    toklist_start.set_info(&mut st.mem, CS_TOKEN_FLAG + FROZEN_SPECIAL);
    let q = crate::mem::get_avail(&mut st.mem).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    q.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'{' as i32);
    toklist_start.set_link(&mut st.mem, q);
    let p = crate::scan::str_toks_scc(st, start_pool, 0)?;
    let th = crate::units::MemPtr::from_raw(TEMP_HEAD).link(&st.mem);
    q.set_link(&mut st.mem, th);
    let q2 = crate::mem::get_avail(&mut st.mem).ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    q2.set_info(&mut st.mem, RIGHT_BRACE_TOKEN + b'}' as i32);
    p.set_link(&mut st.mem, q2);
    crate::input::begin_token_list_in(st, toklist_start.0, INSERTED)?;
    Ok(())
}
/// translated from xetex-xetex0.c:8024 (do_extension)
pub(crate) fn do_extension_in(
    st: &mut EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    use crate::mem::TEX_NULL;

    match st.scanner.cur_chr {
        OPEN_NODE_DE => {
            do_new_write_whatsit_in(st, OPEN_NODE_SIZE_DE)?;
            crate::scan::scan_optional_equals_in(st)?;
            crate::io::scan_file_name_in(st)?;
            st.mem
                .set_b32s1(st.input.cur_list.tail.0 + 1, st.scanner.cur_name);
            st.mem
                .set_b32s0(st.input.cur_list.tail.0 + 2, st.scanner.cur_area);
            st.mem
                .set_b32s1(st.input.cur_list.tail.0 + 2, st.scanner.cur_ext);
        }

        WRITE_NODE_DE => {
            let k = st.scanner.cur_cs;
            do_new_write_whatsit_in(st, WRITE_NODE_SIZE_DE)?;
            st.scanner.cur_cs = k;
            crate::scan::scan_toks_in(st, false, false)?;
            st.mem
                .set_b32s1(st.input.cur_list.tail.0 + 1, st.scanner.def_ref.0);
        }

        CLOSE_NODE_DE => {
            do_new_write_whatsit_in(st, WRITE_NODE_SIZE_DE)?;
            st.mem.set_b32s1(st.input.cur_list.tail.0 + 1, TEX_NULL);
        }

        SPECIAL_NODE_DE => {
            do_new_whatsit_in(
                st,
                crate::node::WhatsitSubtype::Special as i16,
                WRITE_NODE_SIZE_DE,
            )?;
            st.mem.set_b32s0(st.input.cur_list.tail.0 + 1, TEX_NULL);
            crate::scan::scan_toks_in(st, false, true)?;
            st.mem
                .set_b32s1(st.input.cur_list.tail.0 + 1, st.scanner.def_ref.0);
        }

        IMMEDIATE_CODE_DE => {
            get_x_token_in(st)?;
            if st.scanner.cur_cmd == EXTENSION_DE && st.scanner.cur_chr <= CLOSE_NODE_DE {
                let p = st.input.cur_list.tail.0;
                do_extension_in(st)?;
                crate::shipout::out_what_in(st, st.input.cur_list.tail.0)?;
                crate::packaging::flush_node_list(&mut st.mem, st.input.cur_list.tail.0)
                    .map_err(crate::main_control::MainControlAbort::Confusion)?;
                st.input.cur_list.tail.0 = p;
                st.mem.set_b32s1(p, TEX_NULL);
            } else {
                crate::input::back_input_in(st)?;
            }
        }

        SET_LANGUAGE_CODE_DE => {
            if (st.input.cur_list.mode as i32).abs() != HMODE_DE as i32 {
                mc_propagate_error(crate::errors::report_illegal_case_in(st))?;
            } else {
                do_new_whatsit_in(
                    st,
                    crate::node::WhatsitSubtype::Language as i16,
                    SMALL_NODE_SIZE_DE,
                )?;
                scan_int_in(st)?;
                let lang = if st.scanner.cur_val <= 0 || st.scanner.cur_val > 255 {
                    0
                } else {
                    st.scanner.cur_val
                };
                st.input.cur_list.set_aux_s1(lang);
                st.mem.set_b32s1(st.input.cur_list.tail + 1, lang);
                st.mem.set_b16s1(
                    st.input.cur_list.tail.0 + 1,
                    crate::helpers::norm_min(st.eqtb.b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN_DE))
                        as u16,
                );
                st.mem.set_b16s0(
                    st.input.cur_list.tail.0 + 1,
                    crate::helpers::norm_min(st.eqtb.b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN_DE))
                        as u16,
                );
            }
        }

        PDF_SAVE_POS_NODE_DE => {
            do_new_whatsit_in(
                st,
                crate::node::WhatsitSubtype::PdfSavePos as i16,
                SMALL_NODE_SIZE_DE,
            )?;
        }

        RESET_TIMER_CODE_DE => {
            if !st.config.deterministic_mode {
                let mut s = st.shipout.epochseconds;
                let mut m = st.shipout.microseconds;
                unsafe { crate::texmfmp::get_seconds_and_micros(&mut s, &mut m) };
                st.shipout.epochseconds = s;
                st.shipout.microseconds = m;
            }
        }

        SET_RANDOM_SEED_CODE_DE => {
            scan_int_in(st)?;
            if st.scanner.cur_val < 0 {
                st.scanner.cur_val = -st.scanner.cur_val;
            }
            st.errors.random_seed = st.scanner.cur_val;
            let random_seed = st.errors.random_seed;
            crate::scaled_math::init_randoms(&mut st.errors, random_seed);
        }

        PIC_FILE_CODE_DE => {
            if (st.input.cur_list.mode as i32).abs() == MMODE_DE as i32 {
                mc_propagate_error(crate::errors::report_illegal_case_in(st))?;
            } else {
                load_picture_in(st, false)?;
            }
        }

        PDF_FILE_CODE_DE => {
            if (st.input.cur_list.mode as i32).abs() == MMODE_DE as i32 {
                mc_propagate_error(crate::errors::report_illegal_case_in(st))?;
            } else {
                load_picture_in(st, true)?;
            }
        }

        GLYPH_CODE_DE => {
            if (st.input.cur_list.mode as i32).abs() == VMODE_DE as i32 {
                crate::input::back_input_in(st)?;
                new_graf_in(st, true)?;
            } else if (st.input.cur_list.mode as i32).abs() == MMODE_DE as i32 {
                mc_propagate_error(crate::errors::report_illegal_case_in(st))?;
            } else {
                let cur_font = st.eqtb.b32s1(CUR_FONT_LOC);
                let fa = st.fonts.font_area_entry(cur_font);
                if fa == AAT_FONT_FLAG_DE || fa == OTGR_FONT_FLAG_DE {
                    do_new_whatsit_in(
                        st,
                        crate::node::WhatsitSubtype::Glyph as i16,
                        GLYPH_NODE_SIZE_DE,
                    )?;
                    scan_int_in(st)?;
                    if st.scanner.cur_val < 0 || st.scanner.cur_val > 65535 {
                        let errmsg =
                            error_here_with_diagnostic_in(st, c"Bad glyph number".as_ptr());
                        unsafe { diag_append_str(errmsg, &format!(" ({})", st.scanner.cur_val)) };
                        capture_to_diagnostic_in(st, std::ptr::null_mut());
                        st.errors.help_ptr = 2;
                        st.errors.help_line[1] =
                            c"A glyph number must be between 0 and 65535.".as_ptr();
                        st.errors.help_line[0] = c"I changed this one to zero.".as_ptr();
                        mc_propagate_error(crate::errors::int_error_in(st, st.scanner.cur_val))?;
                        st.scanner.cur_val = 0;
                    }
                    st.mem
                        .set_b16s2(st.input.cur_list.tail.0 + 4, cur_font as u16);
                    st.mem
                        .set_b16s1(st.input.cur_list.tail.0 + 4, st.scanner.cur_val as u16);
                    // set_native_glyph_metrics(cur_list.tail.0, ...) expands to
                    // measure_native_glyph(&mem[cur_list.tail.0], use_glyph_metrics)
                    let use_glyph_metrics =
                        st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS_DE) > 0;
                    let tail = st.input.cur_list.tail.0;
                    unsafe {
                        measure_native_glyph(
                            &st.fonts,
                            &mut st.mem,
                            tail,
                            use_glyph_metrics as i32,
                        )?;
                    }
                } else {
                    mc_propagate_error(crate::errors::not_font_error_common_in(
                        st,
                        EXTENSION_DE as u16,
                        GLYPH_CODE_DE,
                        cur_font,
                        c"; not a native platform font".as_ptr(),
                    ))?;
                }
            }
        }

        XETEX_INPUT_ENCODING_EXTENSION_CODE_DE => {
            crate::io::scan_and_pack_name_in(st)?;
            let mut j: i32 = 0;
            let name = st.input.name_of_file.as_mut_ptr();
            let i = unsafe { get_encoding_mode_and_info_in(st, name, &mut j)? };
            if i == XETEX_INPUT_MODE_AUTO_DE {
                error_here_with_diagnostic_in(
                    st,
                    c"Encoding mode `auto' is not valid for \\XeTeXinputencoding".as_ptr(),
                );
                capture_to_diagnostic_in(st, std::ptr::null_mut());
                st.errors.help_ptr = 2;
                st.errors.help_line[1] =
                    c"You can't use `auto' encoding here, only for \\XeTeXdefaultencoding."
                        .as_ptr();
                st.errors.help_line[0] =
                    c"I'll ignore this and leave the current encoding unchanged.".as_ptr();
                mc_propagate_error(crate::errors::error_in(st))?;
            } else {
                unsafe {
                    let f = st.input.input_file_ptr(st.input.in_open);
                    // `\scantokens` occupies an input level without a UFile.
                    if !f.is_null() {
                        crate::io::set_input_file_encoding_in(st, f, i, j)?;
                    }
                }
            }
        }

        XETEX_DEFAULT_ENCODING_EXTENSION_CODE_DE => {
            crate::io::scan_and_pack_name_in(st)?;
            let mut j: i32 = 0;
            let name = st.input.name_of_file.as_mut_ptr();
            let i = unsafe { get_encoding_mode_and_info_in(st, name, &mut j)? };
            st.eqtb
                .set_b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_MODE_DE, i);
            st.eqtb
                .set_b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_ENCODING_DE, j);
        }

        XETEX_LINEBREAK_LOCALE_EXTENSION_CODE_DE => {
            crate::io::scan_file_name_in(st)?;
            if crate::string_pool::length(&st.strings, st.scanner.cur_name) == 0 {
                st.eqtb
                    .set_b32s1(INT_BASE + INT_PAR_XETEX_LINEBREAK_LOCALE_DE, 0);
            } else {
                st.eqtb.set_b32s1(
                    INT_BASE + INT_PAR_XETEX_LINEBREAK_LOCALE_DE,
                    st.scanner.cur_name,
                );
            }
        }

        _ => {
            return Err(crate::main_control::MainControlAbort::Confusion(c"ext1"));
        }
    }
    Ok(())
}

fn do_new_whatsit_in(
    st: &mut EngineState,
    s: i16,
    w: i16,
) -> Result<(), crate::main_control::MainControlAbort> {
    crate::mem::new_whatsit(&mut st.input.cur_list, &mut st.mem, s, w)
        .ok_or(crate::scan::NodeRecycleAbort::Overflow)?;
    Ok(())
}

fn do_new_write_whatsit_in(
    st: &mut EngineState,
    w: i16,
) -> Result<(), crate::main_control::MainControlAbort> {
    new_write_whatsit_in(st, w).map_err(|e| match e {
        NewWriteWhatsitAbort::Scanner(e) => e.into(),
        NewWriteWhatsitAbort::MainMemory => crate::scan::NodeRecycleAbort::Overflow.into(),
    })
}

// ---- do_extension consts ----

// xetex_format.h:457–461, 462, 465, 524–533
const OPEN_NODE_DE: i32 = 0;
const WRITE_NODE_DE: i32 = 1;
const CLOSE_NODE_DE: i32 = 2;
const SPECIAL_NODE_DE: i32 = 3;
const PDF_SAVE_POS_NODE_DE: i32 = 21;
const IMMEDIATE_CODE_DE: i32 = 4;
const SET_LANGUAGE_CODE_DE: i32 = 5;
const RESET_TIMER_CODE_DE: i32 = 31;
const SET_RANDOM_SEED_CODE_DE: i32 = 33;
const PIC_FILE_CODE_DE: i32 = 41;
const PDF_FILE_CODE_DE: i32 = 42;
const GLYPH_CODE_DE: i32 = 43;
const XETEX_INPUT_ENCODING_EXTENSION_CODE_DE: i32 = 44;
const XETEX_DEFAULT_ENCODING_EXTENSION_CODE_DE: i32 = 45;
const XETEX_LINEBREAK_LOCALE_EXTENSION_CODE_DE: i32 = 46;
// xetex-constants.h:110, 112, 118, 126
const WRITE_NODE_SIZE_DE: i16 = 2;
const OPEN_NODE_SIZE_DE: i16 = 3;
const GLYPH_NODE_SIZE_DE: i16 = 5;
const SMALL_NODE_SIZE_DE: i16 = crate::consts::SMALL_NODE_SIZE as i16;
// xetex_format.h:297, 368–370
const EXTENSION_DE: u8 = 59;
const VMODE_DE: i16 = 1;
const HMODE_DE: i16 = 104;
const MMODE_DE: i16 = 207;
// xetex_format.h:212
// xetex-ext.h:47–48
const AAT_FONT_FLAG_DE: i32 = 0xFFFF;
const OTGR_FONT_FLAG_DE: i32 = 0xFFFE;
// xetex-constants.h:254
const XETEX_INPUT_MODE_AUTO_DE: i32 = 0;
// xetex_format.h:86–87, 66, 72, 75–76
const INT_PAR_LEFT_HYPHEN_MIN_DE: i32 = 51;
const INT_PAR_RIGHT_HYPHEN_MIN_DE: i32 = 52;
const INT_PAR_XETEX_LINEBREAK_LOCALE_DE: i32 = 66;
const INT_PAR_XETEX_USE_GLYPH_METRICS_DE: i32 = 72;
const INT_PAR_XETEX_DEFAULT_INPUT_MODE_DE: i32 = 75;
const INT_PAR_XETEX_DEFAULT_INPUT_ENCODING_DE: i32 = 76;

// Shared consts duplicated from scan.rs with the ext group (still used there too).
const LEFT_BRACE_TOKEN: i32 = 0x200000;
const WRITE_NODE_SIZE: i16 = 2; // xetex-constants.h:112
const TEMP_HEAD: i32 = 4_999_999 - 3;
