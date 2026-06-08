// translated from xetex-ext.c
use crate::output::begin_diagnostic;
use crate::output::end_diagnostic;
use crate::output::font_mapping_warning;
use crate::output::print_char;
use crate::output::print_int;
use crate::output::print_nl;
use crate::output::print_raw_char;
use crate::scan::get_tracing_fonts_state;
use crate::texmfmp::gettexstring;
use crate::texmfmp::maketexstring;

use crate::consts::{CS_TOKEN_FLAG, CUR_FONT_LOC, INT_BASE};
use crate::errors::{error, int_error, not_native_font_error, report_illegal_case};
use crate::globals::{
    cur_chr, cur_cmd, cur_val, def_ref, eqtb_b32s1, mem_b32s1, pool_ptr, set_cur_val, set_def_ref,
    set_eqtb_b32s1, set_help_line, set_help_ptr, set_mem_b16s0, set_mem_b16s1, set_mem_b16s2,
    set_mem_b32s0, set_mem_b32s1, set_pool_ptr, set_str_pool,
};
use crate::input::begin_token_list;
use crate::io::scan_and_pack_name;
use crate::main_control::new_graf;
use crate::output::{capture_to_diagnostic, error_here_with_diagnostic};
use crate::packaging::flush_node_list_global;
use crate::pic::load_picture;
use crate::scan::{
    get_x_token, scan_four_bit_int, scan_int, scan_optional_equals, scan_toks, str_toks,
};
use crate::shipout::out_what;

use libc::{c_char, c_double, c_float, c_int, c_long, c_uint, c_void};
use std::ptr;

type XeTeXFont = *mut c_void;
type XeTeXLayoutEngine = *mut c_void;
type RawPlatformFontRef = *mut c_void;

const TTBC_FILE_FORMAT_MISC_FONTS: c_int = 41;
const XETEX_INT: i32 = 27;
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

static mut SAVED_MAPPING_NAME: *mut c_char = ptr::null_mut();

extern "C" {
    fn _tt_abort(fmt: *const c_char) -> !;
    fn ttstub_diag_printf(d: *mut std::ffi::c_void, fmt: *const libc::c_char, ...) -> libc::c_int;
    fn xmalloc(size: libc::size_t) -> *mut c_void;
    fn xcalloc(nelem: libc::size_t, elsize: libc::size_t) -> *mut c_void;
    fn xrealloc(ptr: *mut c_void, size: libc::size_t) -> *mut c_void;
    fn xstrdup(s: *const c_char) -> *mut c_char;
    fn ttstub_input_open(path: *const c_char, format: c_int, mag: c_int) -> usize;
    fn ttstub_input_get_size(handle: usize) -> libc::size_t;
    fn ttstub_input_read(handle: usize, data: *mut u8, len: libc::size_t) -> libc::ssize_t;
    fn ttstub_input_close(handle: usize) -> c_int;
    fn xetex_ucnv_open(name: *const c_char, out_err: *mut c_int) -> *mut c_void;
    fn xetex_ucnv_close(cnv: *mut c_void);
    fn TECkit_CreateConverter(
        mapping: *const u8,
        mapping_size: u32,
        map_forward: u8,
        source_form: u16,
        target_form: u16,
        converter: *mut *mut c_void,
    ) -> i64;
    fn TECkit_ConvertBuffer(
        converter: *mut c_void,
        in_buffer: *const u8,
        in_length: u32,
        in_used: *mut u32,
        out_buffer: *mut u8,
        out_length: u32,
        out_used: *mut u32,
        input_is_complete: u8,
    ) -> i64;
    fn TECkit_ResetConverter(converter: *mut c_void) -> i64;
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

const LEFT_SIDE: i32 = 0;
const RIGHT_SIDE: i32 = 1;
const FONT_FLAGS_VERTICAL: i32 = 0x02;
const UBRK_LINE: c_int = 2;
const UBIDI_RTL: c_int = 1;
const UBIDI_MIXED: c_int = 2;
const XDV_FLAG_VERTICAL: u16 = 0x0100;
const XDV_FLAG_COLORED: u16 = 0x0200;
const XDV_FLAG_EXTEND: u16 = 0x1000;
const XDV_FLAG_SLANT: u16 = 0x2000;
const XDV_FLAG_EMBOLDEN: u16 = 0x4000;
const NATIVE_GLYPH_INFO_SIZE: usize = 10;

static mut BRK_ITER: *mut c_void = ptr::null_mut();
static mut BRK_LOCALE_STR_NUM: i32 = 0;

#[repr(C)]
struct FixedPoint {
    x: i32,
    y: i32,
}

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

unsafe fn print_c_string(s: *const c_char) {
    let mut p = s;
    while *p != 0 {
        print_char(*p as i32);
        p = p.add(1);
    }
}

unsafe fn mfree_ptr<T>(ptr: *mut T) -> *mut T {
    if !ptr.is_null() {
        libc::free(ptr as *mut c_void);
    }
    ptr::null_mut()
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
// `name_of_file` (the encoding name to parse) is read by the `_global` wrapper
// from `name_of_file()` before the call (it is with_state-bearing); the core
// only reads the passed pointer, leaving its diagnostic path borrow-free (§4a).
pub unsafe fn get_encoding_mode_and_info(name_of_file: *const c_char, info: *mut i32) -> c_int {
    *info = 0;

    if libc::strcasecmp(name_of_file, c"auto".as_ptr()) == 0 {
        return AUTO;
    }
    if libc::strcasecmp(name_of_file, c"utf8".as_ptr()) == 0 {
        return UTF8;
    }
    if libc::strcasecmp(name_of_file, c"utf16".as_ptr()) == 0 {
        return US_NATIVE_UTF16;
    }
    if libc::strcasecmp(name_of_file, c"utf16be".as_ptr()) == 0 {
        return UTF16BE;
    }
    if libc::strcasecmp(name_of_file, c"utf16le".as_ptr()) == 0 {
        return UTF16LE;
    }
    if libc::strcasecmp(name_of_file, c"bytes".as_ptr()) == 0 {
        return RAW;
    }

    let mut err = 0;
    let cnv = xetex_ucnv_open(name_of_file, &mut err);
    if cnv.is_null() {
        crate::output::begin_diagnostic();
        crate::output::print_nl(b'U' as i32);
        crate::output::print_cstr(c"nknown encoding `".as_ptr());
        crate::output::print_cstr(name_of_file);
        crate::output::print_cstr(c"'; reading as raw bytes".as_ptr());
        crate::output::end_diagnostic(true);
        RAW
    } else {
        xetex_ucnv_close(cnv);
        *info = maketexstring(name_of_file);
        ICUMAPPING
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

// translated from xetex-ext.c:173 (print_utf8_str)
pub unsafe fn print_utf8_str(mut str_: *const u8, mut len: c_int) {
    while len > 0 {
        print_raw_char(*str_ as u16, true);
        str_ = str_.add(1);
        len -= 1;
    }
}

// translated from xetex-ext.c:187 (load_mapping_file)
unsafe fn load_mapping_file(
    s: *const c_char,
    e: *const c_char,
    byte_mapping: c_char,
) -> *mut c_void {
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
            _tt_abort(c"could not read mapping file".as_ptr());
        }

        ttstub_input_close(map);

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
            font_mapping_warning(buffer as *const c_void, libc::strlen(buffer) as i32, 2);
        } else if get_tracing_fonts_state() > 1 {
            font_mapping_warning(buffer as *const c_void, libc::strlen(buffer) as i32, 0);
        }

        libc::free(mapping as *mut c_void);
    } else {
        font_mapping_warning(buffer as *const c_void, libc::strlen(buffer) as i32, 1);
    }

    libc::free(buffer as *mut c_void);
    cnv
}

// translated from xetex-ext.c:236 (check_for_tfm_font_mapping)
// `name_of_file` is read (and the `:mapping=` separator nulled out) through the
// pointer passed by the `_global` wrapper, which reads `name_of_file()` (itself
// with_state-bearing) before the call.
pub unsafe fn check_for_tfm_font_mapping(name_of_file: *mut c_char) {
    let cp = libc::strstr(name_of_file, c":mapping=".as_ptr());
    SAVED_MAPPING_NAME = mfree_ptr(SAVED_MAPPING_NAME);
    if !cp.is_null() {
        *cp = 0;
        let mut cp = cp.add(9);
        while *cp != 0 && *cp <= b' ' as c_char {
            cp = cp.add(1);
        }
        if *cp != 0 {
            SAVED_MAPPING_NAME = xstrdup(cp);
        }
    }
}

// translated from xetex-ext.c:251 (load_tfm_font_mapping)
pub unsafe fn load_tfm_font_mapping() -> *mut c_void {
    let mut rval = ptr::null_mut();
    if !SAVED_MAPPING_NAME.is_null() {
        rval = load_mapping_file(
            SAVED_MAPPING_NAME,
            SAVED_MAPPING_NAME.add(libc::strlen(SAVED_MAPPING_NAME)),
            1,
        );
        SAVED_MAPPING_NAME = mfree_ptr(SAVED_MAPPING_NAME);
    }
    rval
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

// translated from xetex-ext.c:310 (readCommonFeatures)
#[no_mangle]
pub unsafe extern "C" fn readCommonFeatures(
    feat: *const c_char,
    end: *const c_char,
    extend: *mut f32,
    slant: *mut f32,
    embolden: *mut f32,
    letterspace: *mut f32,
    rgb_value: *mut u32,
) -> c_int {
    let mut sep = str_starts_with(feat, b"mapping");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return -1;
        }
        crate::globals::set_loaded_font_mapping(load_mapping_file(sep.add(1), end, 0));
        return 1;
    }

    sep = str_starts_with(feat, b"extend");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return -1;
        }
        sep = sep.add(1);
        *extend = read_double(&mut sep) as f32;
        return 1;
    }

    sep = str_starts_with(feat, b"slant");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return -1;
        }
        sep = sep.add(1);
        *slant = read_double(&mut sep) as f32;
        return 1;
    }

    sep = str_starts_with(feat, b"embolden");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return -1;
        }
        sep = sep.add(1);
        *embolden = read_double(&mut sep) as f32;
        return 1;
    }

    sep = str_starts_with(feat, b"letterspace");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return -1;
        }
        sep = sep.add(1);
        *letterspace = read_double(&mut sep) as f32;
        return 1;
    }

    sep = str_starts_with(feat, b"color");
    if !sep.is_null() {
        if peek(sep) != b'=' {
            return -1;
        }
        sep = sep.add(1);
        let start = sep;
        *rgb_value = read_rgb_a(&mut sep);
        let consumed = sep.offset_from(start);
        if consumed == 6 || consumed == 8 {
            crate::globals::set_loaded_font_flags(
                crate::globals::loaded_font_flags() | FONT_FLAGS_COLORED,
            );
        } else {
            return -1;
        }
        return 1;
    }

    0
}

// translated from xetex-ext.c:72 (loadOTfont) — private helper
unsafe fn load_ot_font(
    _font_ref: RawPlatformFontRef,
    font: XeTeXFont,
    scaled_size: i32,
    mut cp1: *mut c_char,
) -> *mut c_void {
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
            return ptr::null_mut();
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
                let i = readCommonFeatures(
                    cp1,
                    cp2,
                    &mut extend,
                    &mut slant,
                    &mut embolden,
                    &mut letterspace,
                    &mut rgb_value,
                );
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
                    crate::globals::set_loaded_font_flags(
                        crate::globals::loaded_font_flags() | FONT_FLAGS_VERTICAL,
                    );
                    handled = true;
                }
            }

            if !handled || bad_option {
                crate::output::font_feature_warning(
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
        crate::globals::set_loaded_font_letter_space(
            (letterspace as c_double / 100.0 * scaled_size as c_double) as i32,
        );
    }

    if (crate::globals::loaded_font_flags() & FONT_FLAGS_COLORED) == 0 {
        rgb_value = 0x000000FF;
    }

    if (crate::globals::loaded_font_flags() & FONT_FLAGS_VERTICAL) != 0 {
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
        crate::globals::set_native_font_type_flag(OTGR_FONT_FLAG as i32);
    }

    engine
}

// translated from xetex-ext.c:283 (find_native_font)
pub unsafe fn find_native_font(uname: *mut c_char, mut scaled_size: i32) -> *mut c_void {
    let mut rval: *mut c_void = ptr::null_mut();
    let name = uname;
    let mut var: *mut c_char = ptr::null_mut();
    let mut feat: *mut c_char = ptr::null_mut();
    let mut end: *mut c_char = ptr::null_mut();
    let mut var_string: *mut c_char = ptr::null_mut();
    let mut feat_string: *mut c_char = ptr::null_mut();
    let mut font: XeTeXFont = ptr::null_mut();
    let mut index = 0;

    crate::globals::set_loaded_font_mapping(ptr::null_mut());
    crate::globals::set_loaded_font_flags(0);
    crate::globals::set_loaded_font_letter_space(0);

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
                    scaled_size = crate::scaled_math::xn_over_d(dsize, -scaled_size, 1000);
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

            rval = load_ot_font(ptr::null_mut(), font, scaled_size, feat_string);
            if rval.is_null() {
                deleteFont(font);
            }
            if !rval.is_null() && get_tracing_fonts_state() > 0 {
                begin_diagnostic();
                print_nl(b' ' as i32);
                print_c_string(c"-> ".as_ptr());
                print_c_string(name_string.add(1));
                end_diagnostic(false);
            }
        }
    } else {
        let font_ref = findFontByName(name_string, var_string, Fix2D(scaled_size));

        if get_tracing_fonts_state() > 0 {
            begin_diagnostic();
            print_nl(b' ' as i32);
            print_c_string(c"-> ".as_ptr());
            print_c_string(ttxl_platfont_get_desc(font_ref));
            end_diagnostic(false);
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
            libc::free(crate::globals::name_of_file() as *mut c_void);
            crate::globals::set_name_of_file(
                xmalloc((name_length + 1) as libc::size_t) as *mut c_char
            );
            libc::strcpy(crate::globals::name_of_file(), full_name);

            if scaled_size < 0 {
                font = createFont(font_ref, scaled_size);
                if !font.is_null() {
                    let dsize = D2Fix(getDesignSize(font));
                    if scaled_size == -1000 {
                        scaled_size = dsize;
                    } else {
                        scaled_size = crate::scaled_math::xn_over_d(dsize, -scaled_size, 1000);
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
                    rval = load_ot_font(font_ref, font, scaled_size, feat_string);
                    if rval.is_null() {
                        deleteFont(font);
                    }
                }
            }

            if !var_string.is_null() && *var_string != 0 {
                libc::strcat(crate::globals::name_of_file(), c"/".as_ptr());
                libc::strcat(crate::globals::name_of_file(), var_string);
            }
            if !feat_string.is_null() && *feat_string != 0 {
                libc::strcat(crate::globals::name_of_file(), c":".as_ptr());
                libc::strcat(crate::globals::name_of_file(), feat_string);
            }
            crate::globals::set_name_length(libc::strlen(crate::globals::name_of_file()) as i32);
        }
    }

    libc::free(var_string as *mut c_void);
    libc::free(feat_string as *mut c_void);
    libc::free(name_string as *mut c_void);

    rval
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

// translated from xetex-ext.c:651 (gr_print_font_name)
pub unsafe fn gr_print_font_name(what: i32, p_engine: *mut c_void, param1: i32, param2: i32) {
    let engine = p_engine as XeTeXLayoutEngine;
    let name = match what {
        XETEX_FEATURE_NAME_CODE => getGraphiteFeatureLabel(engine, param1 as u32),
        XETEX_SELECTOR_NAME_CODE => {
            getGraphiteFeatureSettingLabel(engine, param1 as u32, param2 as u32)
        }
        _ => ptr::null_mut(),
    };

    if !name.is_null() {
        crate::output::print_cstr(name);
        gr_label_destroy(name as *mut c_void);
    }
}

// translated from xetex-ext.c:670 (gr_font_get_named)
pub unsafe fn gr_font_get_named(what: i32, p_engine: *mut c_void) -> i32 {
    let mut rval: c_long = -1;
    let engine = p_engine as XeTeXLayoutEngine;
    if what == XETEX_FIND_FEATURE_BY_NAME {
        rval = findGraphiteFeatureNamed(
            engine,
            crate::globals::name_of_file(),
            crate::globals::name_length(),
        );
    }
    rval as i32
}

// translated from xetex-ext.c:683 (gr_font_get_named_1)
pub unsafe fn gr_font_get_named_1(what: i32, p_engine: *mut c_void, param: i32) -> i32 {
    let mut rval: c_long = -1;
    let engine = p_engine as XeTeXLayoutEngine;
    if what == XETEX_FIND_SELECTOR_BY_NAME {
        rval = findGraphiteFeatureSettingNamed(
            engine,
            param as u32,
            crate::globals::name_of_file(),
            crate::globals::name_length(),
        );
    }
    rval as i32
}

// translated from xetex-ext.c:98 (measure_native_node)
pub unsafe fn measure_native_node(
    fonts: &crate::state::FontState,
    p_node: *mut c_void,
    use_glyph_metrics: c_int,
) {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let txt_len = (*node.add(4)).b16.s1 as i32; // native_length
    let txt_ptr = node.add(6) as *mut u16; // node + NATIVE_NODE_SIZE
    let f = (*node.add(4)).b16.s2 as i32; // native_font
    let mut locations: *mut FixedPoint = ptr::null_mut();
    let mut glyph_advances: *mut i32 = ptr::null_mut();
    let mut total_glyph_count = 0i32;

    if fonts.font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        let mut glyph_info: *mut c_void = ptr::null_mut();
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
            (*node.add(5)).ptr = glyph_info;
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
            (*node.add(5)).ptr = glyph_info;

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
        _tt_abort(c"bad native font flag in `measure_native_node`".as_ptr());
    }

    if use_glyph_metrics == 0 || (*node.add(4)).b16.s0 == 0 {
        (*node.add(3)).b32.s1 = fonts.height_base_entry(f);
        (*node.add(2)).b32.s1 = fonts.depth_base_entry(f);
    } else {
        let glyph_count = (*node.add(4)).b16.s0 as usize;
        let locations = (*node.add(5)).ptr as *mut FixedPoint;
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
}

// translated from xetex-ext.c:921 (real_get_native_glyph)
// glyph info buffer layout: [FixedPoint(8b) * glyph_count][u16 * glyph_count]
pub unsafe fn real_get_native_glyph(p_node: *mut c_void, index: u32) -> u16 {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let glyph_count = (*node.add(4)).b16.s0 as usize;
    let glyph_info = (*node.add(5)).ptr as *const u8;
    let glyph_ids = glyph_info.add(glyph_count * 8) as *const u16;
    if index as usize >= glyph_count {
        0
    } else {
        *glyph_ids.add(index as usize)
    }
}

// translated from xetex-ext.c:1183 (real_get_native_italic_correction)
pub unsafe fn real_get_native_italic_correction(
    fonts: &crate::state::FontState,
    p_node: *mut c_void,
) -> i32 {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let f = (*node.add(4)).b16.s2 as i32; // native_font
    let n = (*node.add(4)).b16.s0 as usize; // native_glyph_count
    if n > 0 {
        let glyph_info = (*node.add(5)).ptr as *const u8; // native_glyph_info_ptr
        let glyph_ids = glyph_info.add(n * 8) as *const u16;
        let last_glyph = *glyph_ids.add(n - 1);
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
    p_node: *mut c_void,
) -> i32 {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let gid = (*node.add(4)).b16.s1; // native_glyph = native_length
    let f = (*node.add(4)).b16.s2 as i32; // native_font
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
    p_node: *mut c_void,
    use_glyph_metrics: c_int,
) {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let gid = (*node.add(4)).b16.s1; // native_glyph = native_length
    let f = (*node.add(4)).b16.s2 as i32; // native_font
    let mut ht = 0.0f32;
    let mut dp = 0.0f32;

    if fonts.font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        let font_inst = getFont(engine);
        // node_width(node) = node[1].b32.s1
        (*node.add(1)).b32.s1 = D2Fix(getGlyphWidth(font_inst, gid as u32) as c_double);
        if use_glyph_metrics != 0 {
            getGlyphHeightDepth(engine, gid as u32, &mut ht, &mut dp);
        }
    } else {
        _tt_abort(c"bad native font flag in `measure_native_glyph`".as_ptr());
    }

    if use_glyph_metrics != 0 {
        (*node.add(3)).b32.s1 = D2Fix(ht as c_double); // node_height
        (*node.add(2)).b32.s1 = D2Fix(dp as c_double); // node_depth
    } else {
        (*node.add(3)).b32.s1 = fonts.height_base_entry(f); // node_height = height_base[f]
        (*node.add(2)).b32.s1 = fonts.depth_base_entry(f); // node_depth = depth_base[f]
    }
}

// translated from xetex-ext.c:1559 (real_get_native_word_cp)
pub unsafe fn real_get_native_word_cp(p_node: *mut c_void, side: c_int) -> i32 {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let glyph_count = (*node.add(4)).b16.s0;
    let f = (*node.add(4)).b16.s2 as i32; // native_font
    let glyph_info = (*node.add(5)).ptr as *const u8; // native_glyph_info_ptr
    let glyph_ids = glyph_info.add(glyph_count as usize * 8) as *const u16;

    if glyph_count == 0 {
        return 0;
    }

    let actual_glyph = match side {
        LEFT_SIDE => *glyph_ids,
        RIGHT_SIDE => *glyph_ids.add(glyph_count as usize - 1),
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
) {
    let mut ht = 0.0f32;
    let mut dp = 0.0f32;

    if fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine;
        let gid = mapCharToGlyph(engine, ch as u32);
        getGlyphHeightDepth(engine, gid, &mut ht, &mut dp);
    } else {
        _tt_abort(c"bad native font flag in `get_native_char_height_depth`".as_ptr());
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
}

// translated from xetex-ext.c:822 (getnativecharht)
pub unsafe fn getnativecharht(fonts: &crate::state::FontState, f: i32, c: i32) -> i32 {
    let mut h = 0i32;
    let mut d = 0i32;
    get_native_char_height_depth(fonts, f, c, &mut h, &mut d);
    h
}

// translated from xetex-ext.c:830 (getnativechardp)
pub unsafe fn getnativechardp(fonts: &crate::state::FontState, f: i32, c: i32) -> i32 {
    let mut h = 0i32;
    let mut d = 0i32;
    get_native_char_height_depth(fonts, f, c, &mut h, &mut d);
    d
}

// translated from xetex-ext.c:838 (get_native_char_sidebearings)
pub unsafe fn get_native_char_sidebearings(
    fonts: &crate::state::FontState,
    font: i32,
    ch: i32,
    lsb: *mut i32,
    rsb: *mut i32,
) {
    let mut l = 0.0f32;
    let mut r = 0.0f32;

    if fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine;
        let gid = mapCharToGlyph(engine, ch as u32);
        getGlyphSidebearings(engine, gid, &mut l, &mut r);
    } else {
        _tt_abort(c"bad native font flag in `get_native_char_side_bearings`".as_ptr());
    }

    *lsb = D2Fix(l as c_double);
    *rsb = D2Fix(r as c_double);
}

// translated from xetex-ext.c:862 (get_glyph_bounds)
pub unsafe fn get_glyph_bounds(
    fonts: &crate::state::FontState,
    font: i32,
    edge: i32,
    gid: i32,
) -> i32 {
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
        _tt_abort(c"bad native font flag in `get_glyph_bounds`".as_ptr());
    }
    D2Fix(if edge <= 2 { a } else { b } as c_double)
}

// translated from xetex-ext.c:889 (getnativecharic)
pub unsafe fn getnativecharic(fonts: &crate::state::FontState, f: i32, c: i32) -> i32 {
    let mut lsb = 0i32;
    let mut rsb = 0i32;
    get_native_char_sidebearings(fonts, f, c, &mut lsb, &mut rsb);
    if rsb < 0 {
        fonts.font_letter_space_entry(f) - rsb
    } else {
        fonts.font_letter_space_entry(f)
    }
}

// translated from xetex-ext.c:900 (getnativecharwd)
pub unsafe fn getnativecharwd(fonts: &crate::state::FontState, f: i32, c: i32) -> i32 {
    if fonts.font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        let engine = fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        let gid = mapCharToGlyph(engine, c as u32);
        D2Fix(getGlyphWidthFromEngine(engine, gid) as c_double)
    } else {
        _tt_abort(c"bad native font flag in `get_native_char_wd`".as_ptr())
    }
}

// translated from xetex-ext.c:1261 (map_char_to_glyph)
pub unsafe fn map_char_to_glyph(fonts: &crate::state::FontState, font: i32, ch: i32) -> i32 {
    if ch > 0x10ffff || (0xd800..=0xdfff).contains(&ch) {
        return 0;
    }
    if fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        mapCharToGlyph(
            fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine,
            ch as u32,
        ) as i32
    } else {
        _tt_abort(c"bad native font flag in `map_char_to_glyph`".as_ptr())
    }
}

// translated from xetex-ext.c:1278 (map_glyph_to_index)
// `name` (the glyph name to look up, originally `name_of_file`) is read by the
// `_global` wrapper *before* the state borrow, since `name_of_file()` is itself
// `with_state`-bearing and would nest-panic under a live borrow (§4a).
pub unsafe fn map_glyph_to_index(
    fonts: &crate::state::FontState,
    font: i32,
    name: *const c_char,
) -> i32 {
    if fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        mapGlyphToIndex(
            fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine,
            name,
        )
    } else {
        _tt_abort(c"bad native font flag in `map_glyph_to_index`".as_ptr())
    }
}

// translated from xetex-ext.c:1293 (get_font_char_range)
pub unsafe fn get_font_char_range(fonts: &crate::state::FontState, font: i32, first: i32) -> i32 {
    if fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        getFontCharRange(
            fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine,
            first,
        )
    } else {
        _tt_abort(c"bad native font flag in `get_font_char_range'`".as_ptr())
    }
}

// translated from xetex-ext.c:72 (linebreak_start)
pub unsafe fn linebreak_start(f: c_int, locale_str_num: i32, text: *mut u16, text_length: i32) {
    let mut status: i32 = 0; // U_ZERO_ERROR
    let locale = gettexstring(locale_str_num);

    // Read the (Graphite) font engine under a short borrow and drop it before the
    // with_state-bearing diagnostic path below (§4a borrow-dropping shape).
    let graphite_engine = crate::state::with_state(|s| {
        if s.fonts.font_area_entry(f) as u32 == OTGR_FONT_FLAG {
            Some(s.fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine)
        } else {
            None
        }
    });
    if let Some(engine) = graphite_engine {
        if libc::strcmp(locale, c"G".as_ptr()) == 0
            && initGraphiteBreaking(engine, text as *const u16, text_length as u32)
        {
            return; // locale intentionally not freed (matches C behavior)
        }
    }

    if locale_str_num != BRK_LOCALE_STR_NUM && !BRK_ITER.is_null() {
        ubrk_close(BRK_ITER);
        BRK_ITER = ptr::null_mut();
    }

    if BRK_ITER.is_null() {
        BRK_ITER = ubrk_open(UBRK_LINE, locale, ptr::null(), 0, &mut status);
        if status > 0 {
            begin_diagnostic();
            print_nl(b'E' as i32);
            print_c_string(c"rror ".as_ptr());
            print_int(status);
            print_c_string(c" creating linebreak iterator for locale `".as_ptr());
            print_c_string(locale);
            print_c_string(c"'; trying default locale `en_us'.".as_ptr());
            end_diagnostic(true);
            if !BRK_ITER.is_null() {
                ubrk_close(BRK_ITER);
            }
            status = 0;
            BRK_ITER = ubrk_open(UBRK_LINE, c"en_us".as_ptr(), ptr::null(), 0, &mut status);
        }
        libc::free(locale as *mut c_void);
        BRK_LOCALE_STR_NUM = locale_str_num;
    }

    if BRK_ITER.is_null() {
        _tt_abort(c"failed to create linebreak iterator".as_ptr());
    }

    ubrk_setText(BRK_ITER, text as *const u16, text_length, &mut status);
}

// translated from xetex-ext.c:116 (linebreak_next)
pub unsafe fn linebreak_next(fonts: &crate::state::FontState, f: c_int) -> c_int {
    if !BRK_ITER.is_null() {
        ubrk_next(BRK_ITER)
    } else {
        let engine = fonts.font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        findNextGraphiteBreak(engine)
    }
}

// translated from xetex-ext.c:529 (makeXDVGlyphArrayData)
// xdvBufSize was a C file-static; promoted to module-level static XDV_BUF_SIZE
pub unsafe fn makeXDVGlyphArrayData(p_node: *mut c_void) -> c_int {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let glyph_count = (*node.add(4)).b16.s0 as usize;

    let needed = glyph_count * NATIVE_GLYPH_INFO_SIZE + 8;
    crate::state::with_state_mut(|state| {
        if state.fonts.xdv_buffer.len() < needed {
            state.fonts.xdv_buffer.resize(needed, 0);
        }
    });

    let glyph_info = (*node.add(5)).ptr as *const u8;
    let locations = glyph_info as *const FixedPoint;
    let glyph_ids = glyph_info.add(glyph_count * 8) as *const u16;

    let xdv_ptr = crate::state::with_state(|state| state.fonts.xdv_buffer.as_ptr() as *mut u8);
    let mut cp = xdv_ptr;
    let width = (*node.add(1)).b32.s1 as u32;

    *cp = (width >> 24) as u8;
    cp = cp.add(1);
    *cp = (width >> 16) as u8;
    cp = cp.add(1);
    *cp = (width >> 8) as u8;
    cp = cp.add(1);
    *cp = width as u8;
    cp = cp.add(1);

    *cp = (glyph_count >> 8) as u8;
    cp = cp.add(1);
    *cp = glyph_count as u8;
    cp = cp.add(1);

    for i in 0..glyph_count {
        let x = (*locations.add(i)).x as u32;
        let y = (*locations.add(i)).y as u32;
        *cp = (x >> 24) as u8;
        cp = cp.add(1);
        *cp = (x >> 16) as u8;
        cp = cp.add(1);
        *cp = (x >> 8) as u8;
        cp = cp.add(1);
        *cp = x as u8;
        cp = cp.add(1);
        *cp = (y >> 24) as u8;
        cp = cp.add(1);
        *cp = (y >> 16) as u8;
        cp = cp.add(1);
        *cp = (y >> 8) as u8;
        cp = cp.add(1);
        *cp = y as u8;
        cp = cp.add(1);
    }

    for i in 0..glyph_count {
        let g = *glyph_ids.add(i);
        *cp = (g >> 8) as u8;
        cp = cp.add(1);
        *cp = g as u8;
        cp = cp.add(1);
    }

    cp.offset_from(xdv_ptr) as c_int
}

// translated from xetex-ext.c:584 (make_font_def)
// xdvBufSize was a C file-static; shared with makeXDVGlyphArrayData via XDV_BUF_SIZE
pub unsafe fn make_font_def(fonts: &mut crate::state::FontState, f: i32) -> c_int {
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
        _tt_abort(c"bad native font flag in `make_font_def`".as_ptr());
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
        cp = cp.add(4);
    }

    freeFontFilename(filename);

    font_def_length as c_int
}

// translated from xetex-ext.c:738 (apply_mapping)
pub unsafe fn apply_mapping(p_cnv: *mut c_void, txt_ptr: *mut u16, txt_len: c_int) -> c_int {
    const K_STATUS_NO_ERROR: i64 = 0;
    const K_STATUS_OUTPUT_BUFFER_FULL: i64 = 1;
    let txt_bytes = txt_len as u32 * 2; // sizeof(UniChar) == 2

    let mut needed = (txt_bytes + 32) as usize;
    crate::state::with_state_mut(|state| {
        let needed_u16 = needed / 2;
        if state.fonts.mapped_text.len() < needed_u16 {
            state.fonts.mapped_text.resize(needed_u16, 0);
        }
    });

    loop {
        let mut in_used: u32 = 0;
        let mut out_used: u32 = 0;
        let (mapped_ptr, mapped_len_bytes) = crate::state::with_state(|state| {
            (
                state.fonts.mapped_text.as_ptr() as *mut u8,
                (state.fonts.mapped_text.len() * 2) as u32,
            )
        });
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
                crate::state::with_state_mut(|state| {
                    let needed_u16 = needed / 2;
                    state.fonts.mapped_text.resize(needed_u16, 0);
                });
            }
            _ => return 0,
        }
    }
}

// translated from xetex-ext.c:776 (store_justified_native_glyphs)
pub unsafe fn store_justified_native_glyphs(fonts: &crate::state::FontState, p_node: *mut c_void) {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let f = (*node.add(4)).b16.s2 as i32;

    let saved_width = (*node.add(1)).b32.s1;
    measure_native_node(fonts, p_node, 0);

    if (*node.add(1)).b32.s1 != saved_width {
        let just_amount = Fix2D(saved_width - (*node.add(1)).b32.s1);

        let glyph_count = (*node.add(4)).b16.s0 as usize;
        let glyph_info = (*node.add(5)).ptr as *mut u8;
        let locations = glyph_info as *mut FixedPoint;
        let glyph_ids = glyph_info.add(glyph_count * 8) as *mut u16;

        let space_glyph = map_char_to_glyph(fonts, f, b' ' as i32);
        let mut space_count = 0i32;
        for i in 0..glyph_count {
            if *glyph_ids.add(i) as i32 == space_glyph {
                space_count += 1;
            }
        }

        if space_count > 0 {
            let mut adjustment = 0.0f64;
            let mut space_index = 0i32;
            for i in 0..glyph_count {
                (*locations.add(i)).x = D2Fix(Fix2D((*locations.add(i)).x) + adjustment);
                if *glyph_ids.add(i) as i32 == space_glyph {
                    space_index += 1;
                    adjustment = just_amount * space_index as f64 / space_count as f64;
                }
            }
        } else {
            for i in 1..glyph_count {
                (*locations.add(i)).x = D2Fix(
                    Fix2D((*locations.add(i)).x)
                        + just_amount * i as f64 / (glyph_count - 1) as f64,
                );
            }
        }

        (*node.add(1)).b32.s1 = saved_width;
    }
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

// translated from xetex-ext.c:1538 (print_glyph_name)
pub unsafe fn print_glyph_name(font: i32, gid: i32) {
    let mut len: c_int = 0;

    // Read the font engine under a short borrow and drop it before the
    // with_state-bearing `print_char` loop below (§4a borrow-dropping shape).
    let engine = crate::state::with_state(|s| {
        if s.fonts.font_area_entry(font) as u32 == OTGR_FONT_FLAG {
            Some(s.fonts.font_layout_engine_ptr(font) as XeTeXLayoutEngine)
        } else {
            None
        }
    });
    let s = match engine {
        Some(engine) => getGlyphName(getFont(engine), gid as u16, &mut len),
        None => _tt_abort(c"bad native font flag in `print_glyph_name`".as_ptr()),
    };

    let mut p = s;
    while len > 0 {
        print_char(*p as i32);
        p = p.add(1);
        len -= 1;
    }
    if !s.is_null() {
        freeGlyphName(s as *mut c_char);
    }
}

// ---- Phase-C bridge-down wrappers (native font metric readers) ----
pub unsafe fn measure_native_node_global(p_node: *mut c_void, use_glyph_metrics: c_int) {
    crate::state::with_state(|s| measure_native_node(&s.fonts, p_node, use_glyph_metrics))
}
pub unsafe fn measure_native_glyph_global(p_node: *mut c_void, use_glyph_metrics: c_int) {
    crate::state::with_state(|s| measure_native_glyph(&s.fonts, p_node, use_glyph_metrics))
}
pub unsafe fn store_justified_native_glyphs_global(p_node: *mut c_void) {
    crate::state::with_state(|s| store_justified_native_glyphs(&s.fonts, p_node))
}
pub unsafe fn linebreak_next_global(f: c_int) -> c_int {
    crate::state::with_state(|s| linebreak_next(&s.fonts, f))
}
pub unsafe fn map_glyph_to_index_global(font: i32) -> i32 {
    // `name_of_file()` is with_state-bearing, so read it before the borrow (§4a).
    let name = crate::globals::name_of_file();
    crate::state::with_state(|s| map_glyph_to_index(&s.fonts, font, name))
}
pub unsafe fn make_font_def_global(f: i32) -> c_int {
    crate::state::with_state_mut(|s| make_font_def(&mut s.fonts, f))
}
pub unsafe fn get_encoding_mode_and_info_global(info: *mut i32) -> c_int {
    // `name_of_file()` is with_state-bearing, so read it before the call (§4a).
    let name = crate::globals::name_of_file();
    get_encoding_mode_and_info(name, info)
}
pub unsafe fn check_for_tfm_font_mapping_global() {
    // `name_of_file()` is with_state-bearing, so read it before the call (§4a).
    let name = crate::globals::name_of_file();
    check_for_tfm_font_mapping(name)
}
pub unsafe fn real_get_native_italic_correction_global(p_node: *mut c_void) -> i32 {
    crate::state::with_state(|s| real_get_native_italic_correction(&s.fonts, p_node))
}
pub unsafe fn real_get_native_glyph_italic_correction_global(p_node: *mut c_void) -> i32 {
    crate::state::with_state(|s| real_get_native_glyph_italic_correction(&s.fonts, p_node))
}
pub unsafe fn get_native_char_height_depth_global(
    font: i32,
    ch: i32,
    height: *mut i32,
    depth: *mut i32,
) {
    crate::state::with_state(|s| get_native_char_height_depth(&s.fonts, font, ch, height, depth))
}
pub unsafe fn getnativecharht_global(f: i32, c: i32) -> i32 {
    crate::state::with_state(|s| getnativecharht(&s.fonts, f, c))
}
pub unsafe fn getnativechardp_global(f: i32, c: i32) -> i32 {
    crate::state::with_state(|s| getnativechardp(&s.fonts, f, c))
}
pub unsafe fn get_native_char_sidebearings_global(
    font: i32,
    ch: i32,
    lsb: *mut i32,
    rsb: *mut i32,
) {
    crate::state::with_state(|s| get_native_char_sidebearings(&s.fonts, font, ch, lsb, rsb))
}
pub unsafe fn get_glyph_bounds_global(font: i32, edge: i32, gid: i32) -> i32 {
    crate::state::with_state(|s| get_glyph_bounds(&s.fonts, font, edge, gid))
}
pub unsafe fn getnativecharic_global(f: i32, c: i32) -> i32 {
    crate::state::with_state(|s| getnativecharic(&s.fonts, f, c))
}
pub unsafe fn getnativecharwd_global(f: i32, c: i32) -> i32 {
    crate::state::with_state(|s| getnativecharwd(&s.fonts, f, c))
}
pub unsafe fn map_char_to_glyph_global(font: i32, ch: i32) -> i32 {
    crate::state::with_state(|s| map_char_to_glyph(&s.fonts, font, ch))
}
pub unsafe fn get_font_char_range_global(font: i32, first: i32) -> i32 {
    crate::state::with_state(|s| get_font_char_range(&s.fonts, font, first))
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

// ---- moved verbatim from scan.rs (workstream 02) ----

// translated from xetex-xetex0.c:11376 (append_src_special)
pub unsafe fn append_src_special() {
    let fname = crate::globals::source_filename_stack(crate::globals::in_open());
    if fname > 0 && crate::texmfmp::is_new_source(fname, crate::globals::line()) {
        crate::mem::new_whatsit_global(SPECIAL_NODE, WRITE_NODE_SIZE);
        set_mem_b32s0(crate::globals::cur_list_tail() + 1, 0);
        let d = crate::mem::get_avail_global();
        set_def_ref(d);
        set_mem_b32s0(d, crate::mem::TEX_NULL);
        str_toks(crate::texmfmp::make_src_special(
            fname,
            crate::globals::line(),
        ));
        set_mem_b32s1(d, mem_b32s1(TEMP_HEAD));
        set_mem_b32s1(crate::globals::cur_list_tail() + 1, d);
        crate::texmfmp::remember_source_info(fname, crate::globals::line());
    }
}
// translated from xetex-xetex0.c:11045 (new_write_whatsit)
pub unsafe fn new_write_whatsit(w: i16) {
    crate::mem::new_whatsit_global(cur_chr() as i16, w);
    if w != WRITE_NODE_SIZE {
        scan_four_bit_int();
    } else {
        scan_int();
        if cur_val() < 0 {
            set_cur_val(17);
        } else if cur_val() > 15 && cur_val() != 18 {
            set_cur_val(16);
        }
    }
    set_mem_b32s0(crate::globals::cur_list_tail() + 1, cur_val());
}
// translated from xetex-xetex0.c:9978 (insert_src_special)
pub unsafe fn insert_src_special() {
    use crate::globals::{
        in_open, line, mem_b32s1, set_mem_b32s0, set_mem_b32s1, source_filename_stack,
    };
    const FROZEN_SPECIAL: i32 = 2_243_226 + 10; // FROZEN_CONTROL_SEQUENCE + 10
    const RIGHT_BRACE_TOKEN: i32 = 0x400000; // xetex-constants.h:351

    let sfn = source_filename_stack(in_open());
    if sfn > 0 && crate::texmfmp::is_new_source(sfn, line()) {
        let toklist = crate::mem::get_avail_global();
        let mut p = toklist;
        set_mem_b32s0(p, CS_TOKEN_FLAG + FROZEN_SPECIAL);
        set_mem_b32s1(p, crate::mem::get_avail_global());
        p = mem_b32s1(p);
        set_mem_b32s0(p, LEFT_BRACE_TOKEN + b'{' as i32);
        let q = str_toks(crate::texmfmp::make_src_special(sfn, line()));
        set_mem_b32s1(p, mem_b32s1(TEMP_HEAD));
        p = q;
        set_mem_b32s1(p, crate::mem::get_avail_global());
        p = mem_b32s1(p);
        set_mem_b32s0(p, RIGHT_BRACE_TOKEN + b'}' as i32);
        begin_token_list(toklist, INSERTED);
        crate::texmfmp::remember_source_info(sfn, line());
    }
}
// translated from xetex-xetex0.c:9984 (tt_insert_special)
pub unsafe fn tt_insert_special(ascii_text: *const libc::c_char) {
    const FROZEN_SPECIAL: i32 = 2_243_226 + 10; // FROZEN_CONTROL_SEQUENCE + 10
    const RIGHT_BRACE_TOKEN: i32 = 0x400000; // xetex-constants.h:351

    let start_pool = pool_ptr();
    // Copy ASCII text into str_pool
    let text = std::ffi::CStr::from_ptr(ascii_text).to_bytes();
    if pool_ptr() + text.len() as i32 >= crate::globals::pool_size() {
        crate::errors::overflow(c"string pool overflow".as_ptr(), 0);
    }
    for &b in text {
        set_str_pool(pool_ptr(), b as u16);
        set_pool_ptr(pool_ptr() + 1);
    }
    // Build token list: \special { <tokens> }
    let toklist_start = crate::mem::get_avail_global();
    set_mem_b32s0(toklist_start, CS_TOKEN_FLAG + FROZEN_SPECIAL);
    let q = crate::mem::get_avail_global();
    set_mem_b32s0(q, LEFT_BRACE_TOKEN + b'{' as i32);
    set_mem_b32s1(toklist_start, q);
    let p = str_toks(start_pool);
    set_mem_b32s1(q, mem_b32s1(TEMP_HEAD));
    let q2 = crate::mem::get_avail_global();
    set_mem_b32s0(q2, RIGHT_BRACE_TOKEN + b'}' as i32);
    set_mem_b32s1(p, q2);
    begin_token_list(toklist_start, INSERTED);
}
/// translated from xetex-xetex0.c:8024 (do_extension)
pub unsafe fn do_extension() {
    use crate::globals::{
        cur_area, cur_cs, cur_ext, cur_list_aux_b32s1, cur_list_mode, cur_list_tail, cur_name,
        epochseconds, font_area_entry, in_open, input_file, microseconds, random_seed, set_cur_cs,
        set_cur_list_aux_b32s1, set_cur_list_tail, set_epochseconds, set_microseconds,
        set_random_seed,
    };
    use crate::mem::TEX_NULL;

    match cur_chr() {
        OPEN_NODE_DE => {
            crate::ext::new_write_whatsit(OPEN_NODE_SIZE_DE);
            scan_optional_equals();
            crate::io::scan_file_name();
            set_mem_b32s1(cur_list_tail() + 1, cur_name());
            set_mem_b32s0(cur_list_tail() + 2, cur_area());
            set_mem_b32s1(cur_list_tail() + 2, cur_ext());
        }

        WRITE_NODE_DE => {
            let k = cur_cs();
            crate::ext::new_write_whatsit(WRITE_NODE_SIZE_DE);
            set_cur_cs(k);
            scan_toks(false, false);
            set_mem_b32s1(cur_list_tail() + 1, def_ref());
        }

        CLOSE_NODE_DE => {
            crate::ext::new_write_whatsit(WRITE_NODE_SIZE_DE);
            set_mem_b32s1(cur_list_tail() + 1, TEX_NULL);
        }

        SPECIAL_NODE_DE => {
            crate::mem::new_whatsit_global(SPECIAL_NODE_DE as i16, WRITE_NODE_SIZE_DE);
            set_mem_b32s0(cur_list_tail() + 1, TEX_NULL);
            scan_toks(false, true);
            set_mem_b32s1(cur_list_tail() + 1, def_ref());
        }

        IMMEDIATE_CODE_DE => {
            get_x_token();
            if cur_cmd() == EXTENSION_DE && cur_chr() <= CLOSE_NODE_DE {
                let p = cur_list_tail();
                do_extension();
                out_what(cur_list_tail());
                flush_node_list_global(cur_list_tail());
                set_cur_list_tail(p);
                set_mem_b32s1(p, TEX_NULL);
            } else {
                crate::input::back_input();
            }
        }

        SET_LANGUAGE_CODE_DE => {
            if (cur_list_mode() as i32).abs() != HMODE_DE as i32 {
                report_illegal_case();
            } else {
                crate::mem::new_whatsit_global(LANGUAGE_NODE_DE_W, SMALL_NODE_SIZE_DE);
                scan_int();
                let lang = if cur_val() <= 0 || cur_val() > 255 {
                    0
                } else {
                    cur_val()
                };
                set_cur_list_aux_b32s1(lang);
                set_mem_b32s1(cur_list_tail() + 1, cur_list_aux_b32s1());
                set_mem_b16s1(
                    cur_list_tail() + 1,
                    crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_LEFT_HYPHEN_MIN_DE))
                        as u16,
                );
                set_mem_b16s0(
                    cur_list_tail() + 1,
                    crate::helpers::norm_min(eqtb_b32s1(INT_BASE + INT_PAR_RIGHT_HYPHEN_MIN_DE))
                        as u16,
                );
            }
        }

        PDF_SAVE_POS_NODE_DE => {
            crate::mem::new_whatsit_global(PDF_SAVE_POS_NODE_DE_W, SMALL_NODE_SIZE_DE);
        }

        RESET_TIMER_CODE_DE => {
            let mut s = epochseconds();
            let mut m = microseconds();
            crate::texmfmp::get_seconds_and_micros(&mut s, &mut m);
            set_epochseconds(s);
            set_microseconds(m);
        }

        SET_RANDOM_SEED_CODE_DE => {
            scan_int();
            if cur_val() < 0 {
                set_cur_val(-cur_val());
            }
            set_random_seed(cur_val());
            crate::scaled_math::init_randoms(random_seed());
        }

        PIC_FILE_CODE_DE => {
            if (cur_list_mode() as i32).abs() == MMODE_DE as i32 {
                report_illegal_case();
            } else {
                load_picture(false);
            }
        }

        PDF_FILE_CODE_DE => {
            if (cur_list_mode() as i32).abs() == MMODE_DE as i32 {
                report_illegal_case();
            } else {
                load_picture(true);
            }
        }

        GLYPH_CODE_DE => {
            if (cur_list_mode() as i32).abs() == VMODE_DE as i32 {
                crate::input::back_input();
                new_graf(true);
            } else if (cur_list_mode() as i32).abs() == MMODE_DE as i32 {
                report_illegal_case();
            } else {
                let cur_font = eqtb_b32s1(CUR_FONT_LOC);
                let fa = font_area_entry(cur_font);
                if fa == AAT_FONT_FLAG_DE || fa == OTGR_FONT_FLAG_DE {
                    crate::mem::new_whatsit_global(GLYPH_NODE_TYPE_DE_W, GLYPH_NODE_SIZE_DE);
                    scan_int();
                    if cur_val() < 0 || cur_val() > 65535 {
                        let errmsg = error_here_with_diagnostic(c"Bad glyph number".as_ptr());
                        ttstub_diag_printf(errmsg, c" (%d)".as_ptr(), cur_val());
                        capture_to_diagnostic(std::ptr::null_mut());
                        set_help_ptr(2);
                        set_help_line(1, c"A glyph number must be between 0 and 65535.".as_ptr());
                        set_help_line(0, c"I changed this one to zero.".as_ptr());
                        int_error(cur_val());
                        set_cur_val(0);
                    }
                    set_mem_b16s2(cur_list_tail() + 4, cur_font as u16);
                    set_mem_b16s1(cur_list_tail() + 4, cur_val() as u16);
                    // set_native_glyph_metrics(cur_list.tail, ...) expands to
                    // measure_native_glyph(&mem[cur_list.tail], use_glyph_metrics)
                    let use_glyph_metrics =
                        eqtb_b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS_DE) > 0;
                    let node_ptr = crate::globals::mem_node_addr(cur_list_tail());
                    measure_native_glyph_global(node_ptr, use_glyph_metrics as i32);
                } else {
                    not_native_font_error(EXTENSION_DE as i32, GLYPH_CODE_DE, cur_font);
                }
            }
        }

        XETEX_INPUT_ENCODING_EXTENSION_CODE_DE => {
            scan_and_pack_name();
            let mut j: i32 = 0;
            let i = get_encoding_mode_and_info_global(&mut j);
            if i == XETEX_INPUT_MODE_AUTO_DE {
                error_here_with_diagnostic(
                    c"Encoding mode `auto' is not valid for \\XeTeXinputencoding".as_ptr(),
                );
                capture_to_diagnostic(std::ptr::null_mut());
                set_help_ptr(2);
                set_help_line(
                    1,
                    c"You can't use `auto' encoding here, only for \\XeTeXdefaultencoding."
                        .as_ptr(),
                );
                set_help_line(
                    0,
                    c"I'll ignore this and leave the current encoding unchanged.".as_ptr(),
                );
                error();
            } else {
                crate::io::call_set_input_file_encoding(input_file(in_open()), i, j);
            }
        }

        XETEX_DEFAULT_ENCODING_EXTENSION_CODE_DE => {
            scan_and_pack_name();
            let mut j: i32 = 0;
            let i = get_encoding_mode_and_info_global(&mut j);
            set_eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_MODE_DE, i);
            set_eqtb_b32s1(INT_BASE + INT_PAR_XETEX_DEFAULT_INPUT_ENCODING_DE, j);
        }

        XETEX_LINEBREAK_LOCALE_EXTENSION_CODE_DE => {
            crate::io::scan_file_name();
            if crate::string_pool::length_global(cur_name()) == 0 {
                set_eqtb_b32s1(INT_BASE + INT_PAR_XETEX_LINEBREAK_LOCALE_DE, 0);
            } else {
                set_eqtb_b32s1(INT_BASE + INT_PAR_XETEX_LINEBREAK_LOCALE_DE, cur_name());
            }
        }

        _ => {
            crate::errors::confusion(c"ext1".as_ptr());
        }
    }
}

// ---- do_extension consts moved verbatim from scan.rs (workstream 02) ----

// xetex_format.h:457–461, 462, 465, 524–533
const OPEN_NODE_DE: i32 = 0;
const WRITE_NODE_DE: i32 = 1;
const CLOSE_NODE_DE: i32 = 2;
const SPECIAL_NODE_DE: i32 = 3;
const LANGUAGE_NODE_DE_W: i16 = 4; // as i16 for new_whatsit_global
const PDF_SAVE_POS_NODE_DE: i32 = 21;
const PDF_SAVE_POS_NODE_DE_W: i16 = 21; // as i16 for new_whatsit_global
const GLYPH_NODE_TYPE_DE_W: i16 = 42; // node type 42 for new_whatsit_global
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
const SPECIAL_NODE: i16 = 3; // xetex_format.h:460
const WRITE_NODE_SIZE: i16 = 2; // xetex-constants.h:112
const INSERTED: u16 = 5;
const TEMP_HEAD: i32 = 4_999_999 - 3;
