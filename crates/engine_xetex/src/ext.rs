// translated from xetex-ext.c

use libc::{c_char, c_double, c_int, c_uint, c_void};
use std::ptr;

const TTBC_FILE_FORMAT_MISC_FONTS: c_int = 41;
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
    fn print_raw_char(s: i32, set_trick_count: bool);
    fn print_char(s: i32);
    fn font_mapping_warning(mapping_name: *const c_void, mapping_name_len: i32, warning_type: i32);
    fn get_tracing_fonts_state() -> i32;
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
}
