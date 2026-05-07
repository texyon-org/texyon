// translated from xetex-ext.c

use libc::{c_char, c_double, c_int, c_long, c_uint, c_void};
use std::ptr;

type XeTeXFont = *mut c_void;
type XeTeXLayoutEngine = *mut c_void;

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
    fn xmalloc(size: libc::size_t) -> *mut c_void;
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
    fn maketexstring(s: *const c_char) -> c_int;
    fn print_raw_char(s: i32, set_trick_count: bool);
    fn print_char(s: i32);
    fn font_mapping_warning(mapping_name: *const c_void, mapping_name_len: i32, warning_type: i32);
    fn get_tracing_fonts_state() -> i32;
    fn deleteLayoutEngine(engine: XeTeXLayoutEngine);
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
    fn getGlyphItalCorr(engine: XeTeXLayoutEngine, glyph_id: u32) -> f32;
    fn getGlyphWidth(font: XeTeXFont, gid: u32) -> f32;
    fn getGlyphSidebearings(
        engine: XeTeXLayoutEngine,
        glyph_id: u32,
        lsb: *mut f32,
        rsb: *mut f32,
    );
    fn getGlyphWidthFromEngine(engine: XeTeXLayoutEngine, glyph_id: u32) -> f32;
    fn mapGlyphToIndex(engine: XeTeXLayoutEngine, glyph_name: *const c_char) -> c_int;
    fn getFontCharRange(engine: XeTeXLayoutEngine, req_first: c_int) -> c_int;
    fn getGlyphName(font: XeTeXFont, gid: u16, len: *mut c_int) -> *const c_char;
    fn freeGlyphName(name: *mut c_char);
    fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;
    fn gettexstring(s: i32) -> *mut c_char;
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
    fn begin_diagnostic();
    fn end_diagnostic(newline: c_int);
    fn print_nl(s: i32);
    fn print_int(n: i32);
    fn getFontFilename(engine: XeTeXLayoutEngine, index: *mut u32) -> *const c_char;
    fn freeFontFilename(filename: *const c_char);
    fn getRgbValue(engine: XeTeXLayoutEngine) -> u32;
    fn getEmboldenFactor(engine: XeTeXLayoutEngine) -> f32;
    fn getPointSize(engine: XeTeXLayoutEngine) -> f32;
    fn measure_native_node(p_node: *mut c_void, use_glyph_metrics: bool);
    static mut xdv_buffer: *mut c_char;
    static mut mapped_text: *mut u16;
    #[cfg(target_os = "macos")]
    fn CFRelease(cf: *const c_void);
}

const LEFT_SIDE: i32 = 0;
const RIGHT_SIDE: i32 = 1;
const FONT_FLAGS_VERTICAL: i32 = 0x02;
const UBRK_LINE: c_int = 2;
const XDV_FLAG_VERTICAL: u16 = 0x0100;
const XDV_FLAG_COLORED: u16 = 0x0200;
const XDV_FLAG_EXTEND: u16 = 0x1000;
const XDV_FLAG_SLANT: u16 = 0x2000;
const XDV_FLAG_EMBOLDEN: u16 = 0x4000;
const NATIVE_GLYPH_INFO_SIZE: usize = 10;

static mut BRK_ITER: *mut c_void = ptr::null_mut();
static mut BRK_LOCALE_STR_NUM: i32 = 0;
static mut XDV_BUF_SIZE: c_int = 0;
static mut APPLY_MAPPING_OUT_LENGTH: u32 = 0;

#[repr(C)]
struct FixedPoint {
    x: i32,
    y: i32,
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

// translated from xetex-ext.c:1882 (D2Fix)
#[no_mangle]
pub unsafe extern "C" fn D2Fix(d: c_double) -> i32 {
    (d * 65536.0 + 0.5) as i32
}

// translated from xetex-ext.c:1888 (Fix2D)
#[no_mangle]
pub unsafe extern "C" fn Fix2D(f: i32) -> c_double {
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
#[no_mangle]
pub unsafe extern "C" fn read_rgb_a(cp: *mut *const c_char) -> c_uint {
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
#[no_mangle]
pub unsafe extern "C" fn get_encoding_mode_and_info(info: *mut i32) -> c_int {
    let name_of_file = crate::globals::name_of_file();
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
#[no_mangle]
pub unsafe extern "C" fn read_tag_with_param(cp: *const c_char, param: *mut c_int) -> u32 {
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
#[no_mangle]
pub unsafe extern "C" fn readFeatureNumber(
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
#[no_mangle]
pub unsafe extern "C" fn print_utf8_str(mut str_: *const u8, mut len: c_int) {
    while len > 0 {
        print_raw_char(*str_ as i32, true);
        str_ = str_.add(1);
        len -= 1;
    }
}

// translated from xetex-ext.c:180 (print_chars)
#[no_mangle]
pub unsafe extern "C" fn print_chars(mut str_: *const u16, mut len: c_int) {
    while len > 0 {
        print_char(*str_ as i32);
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
#[no_mangle]
pub unsafe extern "C" fn check_for_tfm_font_mapping() {
    let cp = libc::strstr(crate::globals::name_of_file(), c":mapping=".as_ptr());
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
#[no_mangle]
pub unsafe extern "C" fn load_tfm_font_mapping() -> *mut c_void {
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
#[no_mangle]
pub unsafe extern "C" fn apply_tfm_font_mapping(cnv: *mut c_void, c: c_int) -> c_int {
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

// translated from xetex-ext.c:440 (splitFontName)
#[no_mangle]
pub unsafe extern "C" fn splitFontName(
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
#[no_mangle]
pub unsafe extern "C" fn release_font_engine(engine: *mut c_void, type_flag: c_int) {
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
#[no_mangle]
pub unsafe extern "C" fn ot_get_font_metrics(
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
#[no_mangle]
pub unsafe extern "C" fn ot_font_get(what: i32, p_engine: *mut c_void) -> i32 {
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
#[no_mangle]
pub unsafe extern "C" fn ot_font_get_1(what: i32, p_engine: *mut c_void, param: i32) -> i32 {
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
#[no_mangle]
pub unsafe extern "C" fn ot_font_get_2(
    what: i32,
    p_engine: *mut c_void,
    param1: i32,
    param2: i32,
) -> i32 {
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
#[no_mangle]
pub unsafe extern "C" fn ot_font_get_3(
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
#[no_mangle]
pub unsafe extern "C" fn gr_print_font_name(
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
        crate::output::print_cstr(name);
        gr_label_destroy(name as *mut c_void);
    }
}

// translated from xetex-ext.c:670 (gr_font_get_named)
#[no_mangle]
pub unsafe extern "C" fn gr_font_get_named(what: i32, p_engine: *mut c_void) -> i32 {
    let mut rval: c_long = -1;
    let engine = p_engine as XeTeXLayoutEngine;
    match what {
        XETEX_FIND_FEATURE_BY_NAME => {
            rval = findGraphiteFeatureNamed(
                engine,
                crate::globals::name_of_file(),
                crate::globals::name_length(),
            );
        }
        _ => {}
    }
    rval as i32
}

// translated from xetex-ext.c:683 (gr_font_get_named_1)
#[no_mangle]
pub unsafe extern "C" fn gr_font_get_named_1(
    what: i32,
    p_engine: *mut c_void,
    param: i32,
) -> i32 {
    let mut rval: c_long = -1;
    let engine = p_engine as XeTeXLayoutEngine;
    match what {
        XETEX_FIND_SELECTOR_BY_NAME => {
            rval = findGraphiteFeatureSettingNamed(
                engine,
                param as u32,
                crate::globals::name_of_file(),
                crate::globals::name_length(),
            );
        }
        _ => {}
    }
    rval as i32
}

// translated from xetex-ext.c:921 (real_get_native_glyph)
// glyph info buffer layout: [FixedPoint(8b) * glyph_count][u16 * glyph_count]
#[no_mangle]
pub unsafe extern "C" fn real_get_native_glyph(p_node: *mut c_void, index: u32) -> u16 {
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
#[no_mangle]
pub unsafe extern "C" fn real_get_native_italic_correction(p_node: *mut c_void) -> i32 {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let f = (*node.add(4)).b16.s2 as i32; // native_font
    let n = (*node.add(4)).b16.s0 as usize; // native_glyph_count
    if n > 0 {
        let glyph_info = (*node.add(5)).ptr as *const u8; // native_glyph_info_ptr
        let glyph_ids = glyph_info.add(n * 8) as *const u16;
        let last_glyph = *glyph_ids.add(n - 1);
        if crate::globals::font_area_entry(f) as u32 == OTGR_FONT_FLAG {
            return D2Fix(
                getGlyphItalCorr(
                    crate::globals::font_layout_engine_ptr(f) as XeTeXLayoutEngine,
                    last_glyph as u32,
                ) as c_double,
            ) + crate::globals::font_letter_space_entry(f);
        }
    }
    0
}

// translated from xetex-ext.c:1207 (real_get_native_glyph_italic_correction)
#[no_mangle]
pub unsafe extern "C" fn real_get_native_glyph_italic_correction(p_node: *mut c_void) -> i32 {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let gid = (*node.add(4)).b16.s1; // native_glyph = native_length
    let f = (*node.add(4)).b16.s2 as i32; // native_font
    if crate::globals::font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        return D2Fix(
            getGlyphItalCorr(
                crate::globals::font_layout_engine_ptr(f) as XeTeXLayoutEngine,
                gid as u32,
            ) as c_double,
        );
    }
    0
}

// translated from xetex-ext.c:1224 (measure_native_glyph)
#[no_mangle]
pub unsafe extern "C" fn measure_native_glyph(p_node: *mut c_void, use_glyph_metrics: c_int) {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let gid = (*node.add(4)).b16.s1; // native_glyph = native_length
    let f = (*node.add(4)).b16.s2 as i32; // native_font
    let mut ht = 0.0f32;
    let mut dp = 0.0f32;

    if crate::globals::font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        let engine = crate::globals::font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        let font_inst = getFont(engine);
        // node_width(node) = node[1].b32.s1
        (*node.add(1)).b32.s1 = D2Fix(getGlyphWidth(font_inst, gid as u32) as c_double);
        if use_glyph_metrics != 0 {
            getGlyphHeightDepth(engine, gid as u32, &mut ht, &mut dp);
        }
    } else {
        _tt_abort(b"bad native font flag in `measure_native_glyph`\0".as_ptr() as *const c_char);
    }

    if use_glyph_metrics != 0 {
        (*node.add(3)).b32.s1 = D2Fix(ht as c_double); // node_height
        (*node.add(2)).b32.s1 = D2Fix(dp as c_double); // node_depth
    } else {
        (*node.add(3)).b32.s1 = crate::globals::height_base_entry(f); // node_height = height_base[f]
        (*node.add(2)).b32.s1 = crate::globals::depth_base_entry(f); // node_depth = depth_base[f]
    }
}

// translated from xetex-ext.c:1559 (real_get_native_word_cp)
#[no_mangle]
pub unsafe extern "C" fn real_get_native_word_cp(p_node: *mut c_void, side: c_int) -> i32 {
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
#[no_mangle]
pub unsafe extern "C" fn get_native_char_height_depth(
    font: i32,
    ch: i32,
    height: *mut i32,
    depth: *mut i32,
) {
    let mut ht = 0.0f32;
    let mut dp = 0.0f32;

    if crate::globals::font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        let engine = crate::globals::font_layout_engine_ptr(font) as XeTeXLayoutEngine;
        let gid = mapCharToGlyph(engine, ch as u32);
        getGlyphHeightDepth(engine, gid, &mut ht, &mut dp);
    } else {
        _tt_abort(
            b"bad native font flag in `get_native_char_height_depth`\0".as_ptr() as *const c_char,
        );
    }

    *height = D2Fix(ht as c_double);
    *depth = D2Fix(dp as c_double);

    // snap to known zones (baseline, x-height, cap-height) if within 4% of em-size
    let fuzz =
        crate::globals::font_info_b32s1(6 + crate::globals::param_base(font)) / 25;
    snap_zone(depth, 0, fuzz);
    snap_zone(height, 0, fuzz);
    snap_zone(
        height,
        crate::globals::font_info_b32s1(5 + crate::globals::param_base(font)),
        fuzz,
    );
    snap_zone(
        height,
        crate::globals::font_info_b32s1(8 + crate::globals::param_base(font)),
        fuzz,
    );
}

// translated from xetex-ext.c:822 (getnativecharht)
#[no_mangle]
pub unsafe extern "C" fn getnativecharht(f: i32, c: i32) -> i32 {
    let mut h = 0i32;
    let mut d = 0i32;
    get_native_char_height_depth(f, c, &mut h, &mut d);
    h
}

// translated from xetex-ext.c:830 (getnativechardp)
#[no_mangle]
pub unsafe extern "C" fn getnativechardp(f: i32, c: i32) -> i32 {
    let mut h = 0i32;
    let mut d = 0i32;
    get_native_char_height_depth(f, c, &mut h, &mut d);
    d
}

// translated from xetex-ext.c:838 (get_native_char_sidebearings)
#[no_mangle]
pub unsafe extern "C" fn get_native_char_sidebearings(
    font: i32,
    ch: i32,
    lsb: *mut i32,
    rsb: *mut i32,
) {
    let mut l = 0.0f32;
    let mut r = 0.0f32;

    if crate::globals::font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        let engine = crate::globals::font_layout_engine_ptr(font) as XeTeXLayoutEngine;
        let gid = mapCharToGlyph(engine, ch as u32);
        getGlyphSidebearings(engine, gid, &mut l, &mut r);
    } else {
        _tt_abort(
            b"bad native font flag in `get_native_char_side_bearings`\0".as_ptr()
                as *const c_char,
        );
    }

    *lsb = D2Fix(l as c_double);
    *rsb = D2Fix(r as c_double);
}

// translated from xetex-ext.c:862 (get_glyph_bounds)
#[no_mangle]
pub unsafe extern "C" fn get_glyph_bounds(font: i32, edge: i32, gid: i32) -> i32 {
    // edge codes: 1,2,3,4 => L T R B
    let mut a = 0.0f32;
    let mut b = 0.0f32;

    if crate::globals::font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        let engine = crate::globals::font_layout_engine_ptr(font) as XeTeXLayoutEngine;
        if edge & 1 != 0 {
            getGlyphSidebearings(engine, gid as u32, &mut a, &mut b);
        } else {
            getGlyphHeightDepth(engine, gid as u32, &mut a, &mut b);
        }
    } else {
        _tt_abort(
            b"bad native font flag in `get_glyph_bounds`\0".as_ptr() as *const c_char,
        );
    }
    D2Fix(if edge <= 2 { a } else { b } as c_double)
}

// translated from xetex-ext.c:889 (getnativecharic)
#[no_mangle]
pub unsafe extern "C" fn getnativecharic(f: i32, c: i32) -> i32 {
    let mut lsb = 0i32;
    let mut rsb = 0i32;
    get_native_char_sidebearings(f, c, &mut lsb, &mut rsb);
    if rsb < 0 {
        crate::globals::font_letter_space_entry(f) - rsb
    } else {
        crate::globals::font_letter_space_entry(f)
    }
}

// translated from xetex-ext.c:900 (getnativecharwd)
#[no_mangle]
pub unsafe extern "C" fn getnativecharwd(f: i32, c: i32) -> i32 {
    if crate::globals::font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        let engine = crate::globals::font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        let gid = mapCharToGlyph(engine, c as u32);
        D2Fix(getGlyphWidthFromEngine(engine, gid) as c_double)
    } else {
        _tt_abort(b"bad native font flag in `get_native_char_wd`\0".as_ptr() as *const c_char)
    }
}

// translated from xetex-ext.c:1261 (map_char_to_glyph)
#[no_mangle]
pub unsafe extern "C" fn map_char_to_glyph(font: i32, ch: i32) -> i32 {
    if ch > 0x10ffff || (ch >= 0xd800 && ch <= 0xdfff) {
        return 0;
    }
    if crate::globals::font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        mapCharToGlyph(
            crate::globals::font_layout_engine_ptr(font) as XeTeXLayoutEngine,
            ch as u32,
        ) as i32
    } else {
        _tt_abort(b"bad native font flag in `map_char_to_glyph`\0".as_ptr() as *const c_char)
    }
}

// translated from xetex-ext.c:1278 (map_glyph_to_index)
// glyph name is at name_of_file
#[no_mangle]
pub unsafe extern "C" fn map_glyph_to_index(font: i32) -> i32 {
    if crate::globals::font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        mapGlyphToIndex(
            crate::globals::font_layout_engine_ptr(font) as XeTeXLayoutEngine,
            crate::globals::name_of_file(),
        )
    } else {
        _tt_abort(b"bad native font flag in `map_glyph_to_index`\0".as_ptr() as *const c_char)
    }
}

// translated from xetex-ext.c:1293 (get_font_char_range)
#[no_mangle]
pub unsafe extern "C" fn get_font_char_range(font: i32, first: i32) -> i32 {
    if crate::globals::font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        getFontCharRange(
            crate::globals::font_layout_engine_ptr(font) as XeTeXLayoutEngine,
            first,
        )
    } else {
        _tt_abort(
            b"bad native font flag in `get_font_char_range'`\0".as_ptr() as *const c_char,
        )
    }
}

// translated from xetex-ext.c:72 (linebreak_start)
#[no_mangle]
pub unsafe extern "C" fn linebreak_start(
    f: c_int,
    locale_str_num: i32,
    text: *mut u16,
    text_length: i32,
) {
    let mut status: i32 = 0; // U_ZERO_ERROR
    let locale = gettexstring(locale_str_num);

    if crate::globals::font_area_entry(f) as u32 == OTGR_FONT_FLAG
        && libc::strcmp(locale, b"G\0".as_ptr() as *const c_char) == 0
    {
        let engine = crate::globals::font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        if initGraphiteBreaking(engine, text as *const u16, text_length as u32) {
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
            print_c_string(b"rror \0".as_ptr() as *const c_char);
            print_int(status);
            print_c_string(
                b" creating linebreak iterator for locale `\0".as_ptr() as *const c_char,
            );
            print_c_string(locale);
            print_c_string(b"'; trying default locale `en_us'.\0".as_ptr() as *const c_char);
            end_diagnostic(1);
            if !BRK_ITER.is_null() {
                ubrk_close(BRK_ITER);
            }
            status = 0;
            BRK_ITER = ubrk_open(
                UBRK_LINE,
                b"en_us\0".as_ptr() as *const c_char,
                ptr::null(),
                0,
                &mut status,
            );
        }
        libc::free(locale as *mut c_void);
        BRK_LOCALE_STR_NUM = locale_str_num;
    }

    if BRK_ITER.is_null() {
        _tt_abort(b"failed to create linebreak iterator\0".as_ptr() as *const c_char);
    }

    ubrk_setText(BRK_ITER, text as *const u16, text_length, &mut status);
}

// translated from xetex-ext.c:116 (linebreak_next)
#[no_mangle]
pub unsafe extern "C" fn linebreak_next(f: c_int) -> c_int {
    if !BRK_ITER.is_null() {
        ubrk_next(BRK_ITER)
    } else {
        let engine = crate::globals::font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        findNextGraphiteBreak(engine)
    }
}

// translated from xetex-ext.c:529 (makeXDVGlyphArrayData)
// xdvBufSize was a C file-static; promoted to module-level static XDV_BUF_SIZE
#[no_mangle]
pub unsafe extern "C" fn makeXDVGlyphArrayData(p_node: *mut c_void) -> c_int {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let glyph_count = (*node.add(4)).b16.s0 as usize;

    let needed = (glyph_count * NATIVE_GLYPH_INFO_SIZE + 8) as c_int;
    if needed > XDV_BUF_SIZE {
        libc::free(xdv_buffer as *mut c_void);
        XDV_BUF_SIZE = ((needed / 1024) + 1) * 1024;
        xdv_buffer = xmalloc(XDV_BUF_SIZE as libc::size_t) as *mut c_char;
    }

    let glyph_info = (*node.add(5)).ptr as *const u8;
    let locations = glyph_info as *const FixedPoint;
    let glyph_ids = glyph_info.add(glyph_count * 8) as *const u16;

    let mut cp = xdv_buffer as *mut u8;
    let width = (*node.add(1)).b32.s1 as u32;

    *cp = (width >> 24) as u8; cp = cp.add(1);
    *cp = (width >> 16) as u8; cp = cp.add(1);
    *cp = (width >> 8) as u8; cp = cp.add(1);
    *cp = width as u8; cp = cp.add(1);

    *cp = (glyph_count >> 8) as u8; cp = cp.add(1);
    *cp = glyph_count as u8; cp = cp.add(1);

    for i in 0..glyph_count {
        let x = (*locations.add(i)).x as u32;
        let y = (*locations.add(i)).y as u32;
        *cp = (x >> 24) as u8; cp = cp.add(1);
        *cp = (x >> 16) as u8; cp = cp.add(1);
        *cp = (x >> 8) as u8; cp = cp.add(1);
        *cp = x as u8; cp = cp.add(1);
        *cp = (y >> 24) as u8; cp = cp.add(1);
        *cp = (y >> 16) as u8; cp = cp.add(1);
        *cp = (y >> 8) as u8; cp = cp.add(1);
        *cp = y as u8; cp = cp.add(1);
    }

    for i in 0..glyph_count {
        let g = *glyph_ids.add(i);
        *cp = (g >> 8) as u8; cp = cp.add(1);
        *cp = g as u8; cp = cp.add(1);
    }

    cp.offset_from(xdv_buffer as *mut u8) as c_int
}

// translated from xetex-ext.c:584 (make_font_def)
// xdvBufSize was a C file-static; shared with makeXDVGlyphArrayData via XDV_BUF_SIZE
#[no_mangle]
pub unsafe extern "C" fn make_font_def(f: i32) -> c_int {
    let mut flags: u16 = 0;
    let rgba: u32;
    let size: i32;
    let filename: *const c_char;
    let index: u32;
    let extend: f32;
    let slant: f32;
    let embolden: f32;

    if crate::globals::font_area_entry(f) as u32 == OTGR_FONT_FLAG {
        let engine = crate::globals::font_layout_engine_ptr(f) as XeTeXLayoutEngine;
        let mut idx: u32 = 0;
        filename = getFontFilename(engine, &mut idx);
        assert!(!filename.is_null());
        index = idx;

        rgba = getRgbValue(engine);
        if (crate::globals::font_flags_entry(f) & FONT_FLAGS_VERTICAL) != 0 {
            flags |= XDV_FLAG_VERTICAL;
        }

        extend = getExtendFactor(engine);
        slant = getSlantFactor(engine);
        embolden = getEmboldenFactor(engine);
        size = D2Fix(getPointSize(engine) as c_double);
    } else {
        _tt_abort(b"bad native font flag in `make_font_def`\0".as_ptr() as *const c_char);
    }

    let filename_len = libc::strlen(filename) as u8;

    let mut font_def_length: usize = 4 + 2 + 1 + filename_len as usize + 4;

    if (crate::globals::font_flags_entry(f) & FONT_FLAGS_COLORED) != 0 {
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

    if font_def_length as c_int > XDV_BUF_SIZE {
        libc::free(xdv_buffer as *mut c_void);
        XDV_BUF_SIZE = (((font_def_length / 1024) + 1) * 1024) as c_int;
        xdv_buffer = xmalloc(XDV_BUF_SIZE as libc::size_t) as *mut c_char;
    }

    let mut cp = xdv_buffer as *mut u8;

    std::ptr::write_unaligned(cp as *mut u32, (size as u32).to_be());
    cp = cp.add(4);

    std::ptr::write_unaligned(cp as *mut u16, flags.to_be());
    cp = cp.add(2);

    *cp = filename_len;
    cp = cp.add(1);
    libc::memcpy(cp as *mut c_void, filename as *const c_void, filename_len as libc::size_t);
    cp = cp.add(filename_len as usize);

    std::ptr::write_unaligned(cp as *mut u32, index.to_be());
    cp = cp.add(4);

    if (crate::globals::font_flags_entry(f) & FONT_FLAGS_COLORED) != 0 {
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
// outLength was a C function-static; promoted to module-level APPLY_MAPPING_OUT_LENGTH
#[no_mangle]
pub unsafe extern "C" fn apply_mapping(p_cnv: *mut c_void, txt_ptr: *mut u16, txt_len: c_int) -> c_int {
    const K_STATUS_NO_ERROR: i64 = 0;
    const K_STATUS_OUTPUT_BUFFER_FULL: i64 = 1;
    let txt_bytes = txt_len as u32 * 2; // sizeof(UniChar) == 2

    if APPLY_MAPPING_OUT_LENGTH < txt_bytes + 32 {
        libc::free(mapped_text as *mut c_void);
        APPLY_MAPPING_OUT_LENGTH = txt_bytes + 32;
        mapped_text = xmalloc(APPLY_MAPPING_OUT_LENGTH as libc::size_t) as *mut u16;
    }

    loop {
        let mut in_used: u32 = 0;
        let mut out_used: u32 = 0;
        let status = TECkit_ConvertBuffer(
            p_cnv,
            txt_ptr as *const u8,
            txt_bytes,
            &mut in_used,
            mapped_text as *mut u8,
            APPLY_MAPPING_OUT_LENGTH,
            &mut out_used,
            1,
        );
        TECkit_ResetConverter(p_cnv);

        match status {
            K_STATUS_NO_ERROR => return (out_used / 2) as c_int,
            K_STATUS_OUTPUT_BUFFER_FULL => {
                APPLY_MAPPING_OUT_LENGTH += txt_bytes + 32;
                libc::free(mapped_text as *mut c_void);
                mapped_text = xmalloc(APPLY_MAPPING_OUT_LENGTH as libc::size_t) as *mut u16;
            }
            _ => return 0,
        }
    }
}

// translated from xetex-ext.c:776 (store_justified_native_glyphs)
#[no_mangle]
pub unsafe extern "C" fn store_justified_native_glyphs(p_node: *mut c_void) {
    let node = p_node as *mut crate::mem_types::MemoryWord;
    let f = (*node.add(4)).b16.s2 as i32;

    let saved_width = (*node.add(1)).b32.s1;
    measure_native_node(p_node, false);

    if (*node.add(1)).b32.s1 != saved_width {
        let just_amount = Fix2D(saved_width - (*node.add(1)).b32.s1);

        let glyph_count = (*node.add(4)).b16.s0 as usize;
        let glyph_info = (*node.add(5)).ptr as *mut u8;
        let locations = glyph_info as *mut FixedPoint;
        let glyph_ids = glyph_info.add(glyph_count * 8) as *mut u16;

        let space_glyph = map_char_to_glyph(f, b' ' as i32);
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
#[no_mangle]
pub unsafe extern "C" fn aat_get_font_metrics(
    _attributes: *mut c_void,
    _ascent: *mut i32,
    _descent: *mut i32,
    _xheight: *mut i32,
    _capheight: *mut i32,
    _slant: *mut i32,
) {
}

// translated from xetex-ext.c:1041 (aat_font_get)
#[no_mangle]
pub unsafe extern "C" fn aat_font_get(_what: c_int, _attributes: *mut c_void) -> c_int {
    -1
}

// translated from xetex-ext.c:1067 (aat_font_get_1)
#[no_mangle]
pub unsafe extern "C" fn aat_font_get_1(
    _what: c_int,
    _attributes: *mut c_void,
    _param: c_int,
) -> c_int {
    -1
}

// translated from xetex-ext.c:1125 (aat_font_get_2)
#[no_mangle]
pub unsafe extern "C" fn aat_font_get_2(
    _what: c_int,
    _attributes: *mut c_void,
    _param1: c_int,
    _param2: c_int,
) -> c_int {
    -1
}

// translated from xetex-ext.c:1168 (aat_font_get_named)
#[no_mangle]
pub unsafe extern "C" fn aat_font_get_named(_what: c_int, _attributes: *mut c_void) -> c_int {
    -1
}

// translated from xetex-ext.c:1193 (aat_font_get_named_1)
#[no_mangle]
pub unsafe extern "C" fn aat_font_get_named_1(
    _what: c_int,
    _attributes: *mut c_void,
    _param: c_int,
) -> c_int {
    -1
}

// translated from xetex-ext.c:1218 (aat_print_font_name)
#[no_mangle]
pub unsafe extern "C" fn aat_print_font_name(
    _what: c_int,
    _attributes: *mut c_void,
    _param1: c_int,
    _param2: c_int,
) {
}

// translated from xetex-ext.c:1538 (print_glyph_name)
#[no_mangle]
pub unsafe extern "C" fn print_glyph_name(font: i32, gid: i32) {
    let mut len: c_int = 0;
    let s;

    if crate::globals::font_area_entry(font) as u32 == OTGR_FONT_FLAG {
        let engine = crate::globals::font_layout_engine_ptr(font) as XeTeXLayoutEngine;
        s = getGlyphName(getFont(engine), gid as u16, &mut len);
    } else {
        _tt_abort(b"bad native font flag in `print_glyph_name`\0".as_ptr() as *const c_char);
    }

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
        let mut bracketed = CString::new("[font/name.ttf:2]:color=FF0000").unwrap().into_bytes_with_nul();
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

        let mut named = CString::new("Font/Bold:script=latn").unwrap().into_bytes_with_nul();
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
