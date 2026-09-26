//! The native-font layer: OpenType/Graphite font loading and feature parsing,
//! TECkit mappings, shaping and measuring native word and glyph nodes, and the
//! extension (`\openout`, `\write`, `\special`, ...) commands.

use std::ffi::{c_char, c_int, CStr, CString};
use std::sync::Arc;

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
use crate::io::{AUTO, ICUMAPPING, RAW, UTF16BE, UTF16LE, UTF8};
use crate::layout::{
    cache_glyph_bbox, cached_glyph_bbox, get_cp_code, hb_tag, req_engine,
    set_loaded_font_design_size, set_req_engine, Bidi, EngineOptions, FloatPoint, FontEngineHandle,
    FontMappingHandle, GlyphBBox, HbFeature, OwnedFont, PlatformFont, ProbeEngine,
    K_STATUS_NO_ERROR, K_STATUS_OUTPUT_BUFFER_FULL, UBIDI_MIXED, UBIDI_RTL,
};
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
const FONT_FLAGS_COLORED: i8 = 0x01;
const FONT_FLAGS_VERTICAL: i8 = 0x02;
const XDV_FLAG_VERTICAL: u16 = 0x0100;
const XDV_FLAG_COLORED: u16 = 0x0200;
const XDV_FLAG_EXTEND: u16 = 0x1000;
const XDV_FLAG_SLANT: u16 = 0x2000;
const XDV_FLAG_EMBOLDEN: u16 = 0x4000;

/// Font-wide metrics of a native font, in scaled points.
#[derive(Default)]
pub(crate) struct NativeFontMetrics {
    pub ascent: i32,
    pub descent: i32,
    pub x_height: i32,
    pub cap_height: i32,
    pub slant: i32,
}

/// The byte at `i`, or 0 past the end, as a C string reads.
fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

/// For a `key=value` option: `None` if `s` does not start with `key`, else the
/// offset of the value, or `Err` when the `=` is missing.
fn option_value(s: &[u8], key: &[u8]) -> Option<Result<usize, ()>> {
    if !s.starts_with(key) {
        None
    } else if at(s, key.len()) == b'=' {
        Some(Ok(key.len() + 1))
    } else {
        Some(Err(()))
    }
}

pub(crate) fn d2fix(d: f64) -> i32 {
    (d * 65536.0 + 0.5) as i32
}

pub(crate) fn fix2d(f: i32) -> f64 {
    f as f64 / 65536.0
}

/// Parses a decimal like `-12.5` from the front of `s`, advancing it.
fn read_double(s: &mut &[u8]) -> f64 {
    let mut neg = false;
    let mut val = 0.0;
    let mut i = 0;

    while at(s, i) == b' ' || at(s, i) == b'\t' {
        i += 1;
    }

    if at(s, i) == b'-' {
        neg = true;
        i += 1;
    } else if at(s, i) == b'+' {
        i += 1;
    }

    while at(s, i).is_ascii_digit() {
        val = val * 10.0 + (at(s, i) - b'0') as f64;
        i += 1;
    }

    if at(s, i) == b'.' {
        let mut dec = 10.0;
        i += 1;
        while at(s, i).is_ascii_digit() {
            val += (at(s, i) - b'0') as f64 / dec;
            i += 1;
            dec *= 10.0;
        }
    }

    *s = &s[i.min(s.len())..];
    if neg {
        -val
    } else {
        val
    }
}

fn read_hex_digit(s: &mut &[u8]) -> Option<u32> {
    let value = (at(s, 0) as char).to_digit(16)?;
    *s = &s[1..];
    Some(value)
}

/// Parses `RRGGBB` with an optional `AA` (default opaque) from the front of `s`.
fn read_rgb_a(s: &mut &[u8]) -> u32 {
    let mut rgb_value = 0u32;
    for _ in 0..6 {
        let Some(value) = read_hex_digit(s) else {
            return 0x000000FF;
        };
        rgb_value = (rgb_value << 4) + value;
    }

    let mut alpha = 0u32;
    let mut digits = 0;
    for _ in 0..2 {
        let Some(value) = read_hex_digit(s) else {
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
fn read_tag_with_param(s: &[u8]) -> (u32, c_int) {
    let mut end = 0;
    while !matches!(at(s, end), 0 | b':' | b';' | b',' | b'=') {
        end += 1;
    }

    let tag = hb_tag(&s[..end]);

    let mut param: c_int = 0;
    let mut i = end;
    if at(s, i) == b'=' {
        let mut neg = false;
        i += 1;
        if at(s, i) == b'-' {
            neg = true;
            i += 1;
        }
        while at(s, i).is_ascii_digit() {
            param = param * 10 + (at(s, i) - b'0') as c_int;
            i += 1;
        }
        if neg {
            param = -param;
        }
    }
    (tag, param)
}

/// Parse a Graphite `feature=setting` pair given as numbers, spanning all of `spec`.
fn read_feature_number(spec: &[u8]) -> Option<(u32, c_int)> {
    let mut f = 0u32;
    let mut v: c_int = 0;
    let mut i = 0;
    if !at(spec, i).is_ascii_digit() {
        return None;
    }
    while at(spec, i).is_ascii_digit() {
        f = f * 10 + (at(spec, i) - b'0') as u32;
        i += 1;
    }
    while at(spec, i) == b' ' || at(spec, i) == b'\t' {
        i += 1;
    }
    if at(spec, i) != b'=' {
        return None;
    }
    i += 1;

    if !at(spec, i).is_ascii_digit() {
        return None;
    }
    while at(spec, i).is_ascii_digit() {
        v = v * 10 + (at(spec, i) - b'0') as c_int;
        i += 1;
    }
    while at(spec, i) == b' ' || at(spec, i) == b'\t' {
        i += 1;
    }
    (i == spec.len()).then_some((f, v))
}

/// Load TECkit mapping `name` (plus `.tec`), mapping to bytes for TFM fonts or
/// to UTF-16 for native ones.
fn load_mapping_file(
    st: &mut EngineState,
    name: &[u8],
    byte_mapping: bool,
) -> Result<Option<Arc<FontMappingHandle>>, Abort> {
    let file_name = [name, b".tec"].concat();
    let name_len = file_name
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(file_name.len());
    let name = &file_name[..name_len];

    let map = input_open(name, TTBC_FILE_FORMAT_MISC_FONTS, false);
    if map == 0 {
        crate::output::font_mapping_warning(st, name, 1);
        return Ok(None);
    }

    let mapping_size = input_get_size(map);
    let mut mapping = vec![0u8; mapping_size];
    let read = input_read(map, &mut mapping);
    if read < 0 || read as usize != mapping_size {
        return Err(Abort::Internal("could not read mapping file"));
    }
    if crate::io::input_close(map) {
        return Err(Abort::InputClose);
    }

    let (map_forward, target_form) = if byte_mapping {
        (false, K_FORM_BYTES)
    } else {
        (true, UTF16_NATIVE)
    };
    let (cnv, _) = FontMappingHandle::new(Some(&mapping), map_forward, UTF16_NATIVE, target_form);

    match &cnv {
        None => crate::output::font_mapping_warning(st, name, 2),
        Some(_) if st.eqtb.int_par(INT_PAR_XETEX_TRACING_FONTS) > 1 => {
            crate::output::font_mapping_warning(st, name, 0)
        }
        Some(_) => {}
    }
    Ok(cnv.map(Arc::new))
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

pub(crate) fn load_tfm_font_mapping(
    st: &mut EngineState,
) -> Result<Option<Arc<FontMappingHandle>>, Abort> {
    let Some(name) = st.fonts.saved_mapping_name.take() else {
        return Ok(None);
    };
    load_mapping_file(st, name.as_bytes(), true)
}

pub(crate) fn apply_tfm_font_mapping(cnv: &FontMappingHandle, c: c_int) -> c_int {
    let input = (c as u16).to_ne_bytes();
    let mut out = [0u8; 2];
    let (_, out_used) = cnv.convert(&input, &mut out);
    if out_used < 1 {
        0
    } else {
        out[0] as c_int
    }
}

struct CommonFeatures {
    extend: f32,
    slant: f32,
    embolden: f32,
    letterspace: f32,
    rgb_value: u32,
}

/// Parse one common option `feat[start..end]`: `None` if it is not one, else
/// whether it was well formed.
fn read_common_feature(
    st: &mut EngineState,
    feat: &[u8],
    start: usize,
    end: usize,
    opts: &mut CommonFeatures,
) -> Result<Option<bool>, Abort> {
    let option = &feat[start..];
    if let Some(value) = option_value(option, b"mapping") {
        let Ok(value) = value else {
            return Ok(Some(false));
        };
        st.fonts.loaded_font_mapping = load_mapping_file(st, &feat[start + value..end], false)?;
        return Ok(Some(true));
    }

    for (key, slot) in [
        (&b"extend"[..], &mut opts.extend),
        (b"slant", &mut opts.slant),
        (b"embolden", &mut opts.embolden),
        (b"letterspace", &mut opts.letterspace),
    ] {
        if let Some(value) = option_value(option, key) {
            let Ok(value) = value else {
                return Ok(Some(false));
            };
            *slot = read_double(&mut &option[value..]) as f32;
            return Ok(Some(true));
        }
    }

    if let Some(value) = option_value(option, b"color") {
        let Ok(value) = value else {
            return Ok(Some(false));
        };
        let mut rest = &option[value..];
        let before = rest.len();
        opts.rgb_value = read_rgb_a(&mut rest);
        let consumed = before - rest.len();
        if consumed != 6 && consumed != 8 {
            return Ok(Some(false));
        }
        st.fonts.loaded_font_flags |= FONT_FLAGS_COLORED;
        return Ok(Some(true));
    }

    Ok(None)
}

/// Build the OpenType/Graphite layout engine for `font` with the options in
/// the feature string `feat`.
fn load_ot_font(
    st: &mut EngineState,
    font: OwnedFont,
    scaled_size: i32,
    feat: Option<&[u8]>,
) -> Result<Option<FontEngineHandle>, Abort> {
    let mut script = HB_TAG_NONE;
    let mut language: Option<CString> = None;
    let mut features: Vec<HbFeature> = Vec::new();
    let mut shapers: Vec<&'static CStr> = Vec::new();
    let mut opts = CommonFeatures {
        extend: 1.0,
        slant: 0.0,
        embolden: 0.0,
        letterspace: 0.0,
        rgb_value: 0x000000FF,
    };
    let req_engine = req_engine();

    match req_engine {
        b'O' => shapers.push(c"ot"),
        b'G' => shapers.push(c"graphite2"),
        _ => {}
    }

    // Graphite feature names are resolved against a probe engine.
    let probe = if req_engine == b'G' {
        let Some(probe) = ProbeEngine::graphite(&font, script, opts.rgb_value) else {
            return Ok(None);
        };
        Some(probe)
    } else {
        None
    };

    if let Some(feat) = feat {
        let mut cp1 = 0;
        while at(feat, cp1) != 0 {
            if matches!(at(feat, cp1), b':' | b';' | b',') {
                cp1 += 1;
            }
            while matches!(at(feat, cp1), b' ' | b'\t') {
                cp1 += 1;
            }
            if at(feat, cp1) == 0 {
                break;
            }

            let mut cp2 = cp1;
            while !matches!(at(feat, cp2), 0 | b':' | b';' | b',') {
                cp2 += 1;
            }
            let option = &feat[cp1..];
            let spec = &feat[cp1..cp2];

            let parsed = 'option: {
                if let Some(value) = option_value(option, b"script") {
                    let Ok(value) = value else {
                        break 'option Some(false);
                    };
                    script = hb_tag(&feat[cp1 + value..cp2]);
                    break 'option Some(true);
                }
                if let Some(value) = option_value(option, b"language") {
                    let Ok(value) = value else {
                        break 'option Some(false);
                    };
                    language = Some(CString::new(&feat[cp1 + value..cp2]).unwrap_or_default());
                    break 'option Some(true);
                }
                if let Some(value) = option_value(option, b"shaper") {
                    let Ok(value) = value else {
                        break 'option Some(false);
                    };
                    // Leaked on purpose: the layout engine keeps the pointer.
                    let shaper = CString::new(&feat[cp1 + value..cp2]).unwrap_or_default();
                    shapers.push(Box::leak(shaper.into_boxed_c_str()));
                    break 'option Some(true);
                }
                if let Some(valid) = read_common_feature(st, feat, cp1, cp2, &mut opts)? {
                    break 'option Some(valid);
                }
                if let Some(probe) = &probe {
                    if let Some((tag, value)) =
                        read_feature_number(spec).or_else(|| probe.find_graphite_feature(spec))
                    {
                        features.push(HbFeature::global(tag, value as u32));
                        break 'option Some(true);
                    }
                }
                if at(feat, cp1) == b'+' {
                    let (tag, mut param) = read_tag_with_param(&feat[cp1 + 1..]);
                    if param >= 0 {
                        param += 1;
                    }
                    features.push(HbFeature::global(tag, param as u32));
                    break 'option Some(true);
                }
                if at(feat, cp1) == b'-' {
                    let tag = hb_tag(&feat[cp1 + 1..cp2]);
                    features.push(HbFeature::global(tag, 0));
                    break 'option Some(true);
                }
                if option.starts_with(b"vertical") {
                    let mut cp3 = cp2;
                    if matches!(at(feat, cp3), b';' | b':' | b',') {
                        cp3 -= 1;
                    }
                    while matches!(at(feat, cp3), 0 | b' ' | b'\t') {
                        cp3 -= 1;
                    }
                    if at(feat, cp3) != 0 {
                        cp3 += 1;
                    }
                    if cp3 == cp1 + 8 {
                        st.fonts.loaded_font_flags |= FONT_FLAGS_VERTICAL;
                        break 'option Some(true);
                    }
                }
                None
            };

            if parsed != Some(true) {
                crate::output::font_feature_warning(st, spec, &[]);
            }
            cp1 = cp2;
        }
    }

    drop(probe);

    if opts.embolden != 0.0 {
        opts.embolden = (opts.embolden as f64 * fix2d(scaled_size) / 100.0) as f32;
    }
    if opts.letterspace != 0.0 {
        st.fonts.loaded_font_letter_space =
            (opts.letterspace as f64 / 100.0 * scaled_size as f64) as i32;
    }
    if st.fonts.loaded_font_flags & FONT_FLAGS_COLORED == 0 {
        opts.rgb_value = 0x000000FF;
    }
    if st.fonts.loaded_font_flags & FONT_FLAGS_VERTICAL != 0 {
        font.set_layout_dir(true);
    }

    let engine = FontEngineHandle::new(
        font,
        EngineOptions {
            script,
            language: language.as_deref(),
            features: &mut features,
            shapers: &shapers,
            rgb_value: opts.rgb_value,
            extend: opts.extend,
            slant: opts.slant,
            embolden: opts.embolden,
        },
    );
    if engine.is_some() {
        st.fonts.native_font_type_flag = OTGR_FONT_FLAG;
    }
    Ok(engine)
}

impl ErrorState {
    /// Resolve a negative (`scaled`) font size against `font`'s design size.
    fn scale_by_design_size(&mut self, font: Option<OwnedFont>, scaled_size: i32) -> i32 {
        let Some(font) = font else {
            return scaled_size;
        };
        let dsize = d2fix(font.design_size());
        if scaled_size == -1000 {
            dsize
        } else {
            self.xn_over_d(dsize, -scaled_size, 1000)
        }
    }
}

/// The bytes of a C string buffer up to its NUL.
fn until_nul(buf: &[u8]) -> &[u8] {
    &buf[..buf.iter().position(|&b| b == 0).unwrap_or(buf.len())]
}

/// Look up native font `name` (a `[file]` path or an installed font name with
/// `/variant` and `:features`) and build its layout engine.
pub(crate) fn find_native_font(
    st: &mut EngineState,
    name: &[u8],
    mut scaled_size: i32,
) -> Result<Option<FontEngineHandle>, Abort> {
    let mut rval = None;

    st.fonts.loaded_font_mapping = None;
    st.fonts.loaded_font_flags = 0;
    st.fonts.loaded_font_letter_space = 0;

    let split = split_font_name(name);
    let name_string = &name[..split.var];
    let mut var_string = (split.feat > split.var).then(|| name[split.var + 1..split.feat].to_vec());
    let feat_string = (split.end > split.feat).then(|| name[split.feat + 1..split.end].to_vec());

    if at(name_string, 0) == b'[' {
        let path = CString::new(&name_string[1..]).unwrap_or_default();
        if scaled_size < 0 {
            let font = OwnedFont::from_file(&path, split.index, 655360);
            scaled_size = st.errors.scale_by_design_size(font, scaled_size);
        }
        if let Some(font) = OwnedFont::from_file(&path, split.index, scaled_size) {
            set_loaded_font_design_size(d2fix(font.design_size()));

            set_req_engine(0);
            if let Some(var) = &var_string {
                if var.starts_with(b"/AAT") {
                    set_req_engine(b'A');
                } else if var.starts_with(b"/OT") || var.starts_with(b"/ICU") {
                    set_req_engine(b'O');
                } else if var.starts_with(b"/GR") {
                    set_req_engine(b'G');
                }
            }

            rval = load_ot_font(st, font, scaled_size, feat_string.as_deref())?;
            if rval.is_some() && st.eqtb.int_par(INT_PAR_XETEX_TRACING_FONTS) > 0 {
                crate::output::begin_diagnostic(st);
                crate::output::print_nl(st, b' ' as i32);
                crate::output::print_raw_bytes(st, b"-> ");
                crate::output::print_raw_bytes(st, path.as_bytes());
                crate::output::end_diagnostic(st, false);
            }
        }
    } else {
        let name_c = CString::new(name_string).unwrap_or_default();
        if let Some(font_ref) = PlatformFont::find(&name_c, var_string.as_mut(), fix2d(scaled_size))
        {
            if st.eqtb.int_par(INT_PAR_XETEX_TRACING_FONTS) > 0 {
                crate::output::begin_diagnostic(st);
                crate::output::print_nl(st, b' ' as i32);
                crate::output::print_raw_bytes(st, b"-> ");
                crate::output::print_raw_bytes(st, &font_ref.desc());
                crate::output::end_diagnostic(st, false);
            }

            let mut full_name = font_ref.full_name();
            st.input.name_of_file.clear();
            st.input.name_of_file.extend_from_slice(&full_name);
            st.input.name_of_file.push(0);

            if scaled_size < 0 {
                let font = font_ref.create_font(scaled_size);
                scaled_size = st.errors.scale_by_design_size(font, scaled_size);
            }

            if let Some(font) = font_ref.create_font(scaled_size) {
                rval = load_ot_font(st, font, scaled_size, feat_string.as_deref())?;
            }

            if let Some(var) = var_string
                .as_deref()
                .map(until_nul)
                .filter(|v| !v.is_empty())
            {
                full_name.push(b'/');
                full_name.extend_from_slice(var);
            }
            if let Some(feat) = feat_string.as_deref().filter(|f| !f.is_empty()) {
                full_name.push(b':');
                full_name.extend_from_slice(feat);
            }
            st.input.name_length = full_name.len() as i32;
            full_name.push(0);
            st.input.name_of_file = full_name;
        }
    }

    Ok(rval)
}

/// Where a native font spec's `/variant` and `:features` parts start, where it
/// ends, and the face index of a `[file:index]` name.
#[derive(Debug, PartialEq)]
struct SplitName {
    var: usize,
    feat: usize,
    end: usize,
    index: c_int,
}

fn split_font_name(name: &[u8]) -> SplitName {
    let mut var = None;
    let mut feat = None;
    let mut index: c_int = 0;
    let mut i = 0;

    if at(name, 0) == b'[' {
        let mut within_file_name = true;
        i = 1;
        while i < name.len() {
            let c = name[i];
            if within_file_name && c == b']' {
                within_file_name = false;
                var.get_or_insert(i);
            } else if c == b':' {
                if within_file_name && var.is_none() {
                    var = Some(i);
                    i += 1;
                    while at(name, i).is_ascii_digit() {
                        index = index * 10 + (name[i] - b'0') as c_int;
                        i += 1;
                    }
                    i -= 1;
                } else if !within_file_name && feat.is_none() {
                    feat = Some(i);
                }
            }
            i += 1;
        }
    } else {
        while i < name.len() {
            let c = name[i];
            if c == b'/' && var.is_none() && feat.is_none() {
                var = Some(i);
            } else if c == b':' && feat.is_none() {
                feat = Some(i);
            }
            i += 1;
        }
    }
    let end = i;
    let feat = feat.unwrap_or(end);
    let var = var.unwrap_or(feat);
    SplitName {
        var,
        feat,
        end,
        index,
    }
}

pub(crate) fn ot_get_font_metrics(engine: &FontEngineHandle) -> NativeFontMetrics {
    let (a, d) = engine.ascent_and_descent();
    let ascent = d2fix(a as f64);
    let descent = d2fix(d as f64);

    let slant = d2fix(
        fix2d(engine.font().slant()) * engine.extend_factor() as f64 + engine.slant_factor() as f64,
    );

    let (cap, x) = engine.cap_and_x_height();
    let mut cap_height = d2fix(cap as f64);
    let mut x_height = d2fix(x as f64);

    // Fall back to the height of `x` and `X`, else to fractions of the ascent.
    if x_height == 0 {
        let glyph_id = engine.map_char_to_glyph(b'x' as u32);
        x_height = if glyph_id != 0 {
            d2fix(engine.glyph_height_depth(glyph_id).0 as f64)
        } else {
            ascent / 2
        };
    }
    if cap_height == 0 {
        let glyph_id = engine.map_char_to_glyph(b'X' as u32);
        cap_height = if glyph_id != 0 {
            d2fix(engine.glyph_height_depth(glyph_id).0 as f64)
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

pub(crate) fn ot_font_get(what: i32, engine: &FontEngineHandle) -> i32 {
    let font = engine.font();
    match what {
        XETEX_COUNT_GLYPHS_CODE => font.count_glyphs() as i32,
        XETEX_COUNT_FEATURES_CODE => engine.count_graphite_features() as i32,
        XETEX_OT_COUNT_SCRIPTS_CODE => font.count_scripts() as i32,
        _ => 0,
    }
}

pub(crate) fn ot_font_get_1(what: i32, engine: &FontEngineHandle, param: i32) -> i32 {
    let font = engine.font();
    match what {
        XETEX_OT_COUNT_LANGUAGES_CODE => font.count_languages(param as u32) as i32,
        XETEX_OT_SCRIPT_CODE => font.ind_script(param as u32) as i32,
        XETEX_FEATURE_CODE_CODE => engine.graphite_feature_code(param as u32) as i32,
        XETEX_IS_EXCLUSIVE_FEATURE_CODE => 1,
        XETEX_COUNT_SELECTORS_CODE => engine.count_graphite_feature_settings(param as u32) as i32,
        _ => 0,
    }
}

pub(crate) fn ot_font_get_2(what: i32, engine: &FontEngineHandle, param1: i32, param2: i32) -> i32 {
    let font = engine.font();
    match what {
        XETEX_OT_LANGUAGE_CODE => font.ind_language(param1 as u32, param2 as u32) as i32,
        XETEX_OT_COUNT_FEATURES_CODE => font.count_features(param1 as u32, param2 as u32) as i32,
        XETEX_SELECTOR_CODE_CODE => {
            engine.graphite_feature_setting_code(param1 as u32, param2 as u32) as i32
        }
        XETEX_IS_DEFAULT_SELECTOR_CODE => {
            (engine.graphite_feature_default_setting(param1 as u32) == param2 as u32) as i32
        }
        _ => 0,
    }
}

pub(crate) fn ot_font_get_3(
    what: i32,
    engine: &FontEngineHandle,
    param1: i32,
    param2: i32,
    param3: i32,
) -> i32 {
    match what {
        XETEX_OT_FEATURE_CODE => {
            engine
                .font()
                .ind_feature(param1 as u32, param2 as u32, param3 as u32) as i32
        }
        _ => 0,
    }
}

pub(crate) fn gr_print_font_name(
    st: &mut EngineState,
    what: i32,
    engine: &FontEngineHandle,
    param1: i32,
    param2: i32,
) {
    let name = match what {
        XETEX_FEATURE_NAME_CODE => engine.graphite_feature_label(param1 as u32),
        XETEX_SELECTOR_NAME_CODE => {
            engine.graphite_feature_setting_label(param1 as u32, param2 as u32)
        }
        _ => None,
    };
    if let Some(name) = name {
        crate::output::print(st, name.as_slice());
    }
}

pub(crate) fn gr_font_get_named(st: &EngineState, what: i32, engine: &FontEngineHandle) -> i32 {
    if what != XETEX_FIND_FEATURE_BY_NAME_CODE {
        return -1;
    }
    engine.find_graphite_feature_named(name_of_file_prefix(st)) as i32
}

pub(crate) fn gr_font_get_named_1(
    st: &EngineState,
    what: i32,
    engine: &FontEngineHandle,
    param: i32,
) -> i32 {
    if what != XETEX_FIND_SELECTOR_BY_NAME_CODE {
        return -1;
    }
    engine.find_graphite_feature_setting_named(param as u32, name_of_file_prefix(st)) as i32
}

/// The first `name_length` bytes of `name_of_file`.
fn name_of_file_prefix(st: &EngineState) -> &[u8] {
    let len = (st.input.name_length.max(0) as usize).min(st.input.name_of_file.len());
    &st.input.name_of_file[..len]
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
    fn append_run(
        &mut self,
        engine: &FontEngineHandle,
        n_glyphs: i32,
        x: f64,
        y: f64,
    ) -> FloatPoint {
        let n = n_glyphs as usize;
        let (glyphs, advances, positions) = engine.last_layout(n);

        self.gids.extend(glyphs.iter().map(|&g| g as u16));
        self.locations
            .extend(positions[..n].iter().map(|pos| FixedPoint {
                x: d2fix(pos.x as f64 + x),
                y: d2fix(pos.y as f64 + y),
            }));
        self.advances
            .extend(advances.iter().map(|&a| d2fix(a as f64)));
        positions[n]
    }
}

fn layout_visual_run(bidi: &Bidi, engine: &FontEngineHandle, text: &[u16], run_index: i32) -> i32 {
    let (logical_start, length, dir) = bidi.visual_run(run_index);
    engine.layout_chars(text, logical_start, length, dir == UBIDI_RTL)
}

fn cached_or_measured_bbox(engine: &FontEngineHandle, f: i32, gid: u16) -> GlyphBBox {
    cached_glyph_bbox(f as u16, gid).unwrap_or_else(|| {
        let bbox = engine.glyph_bounds(gid as u32);
        cache_glyph_bbox(f as u16, gid, &bbox);
        bbox
    })
}

impl MemState {
    pub fn get_native_glyph(&self, w: NodePtr<WhatsitNode>, index: u32) -> u16 {
        if index as usize >= w.native_glyph_count(self) as usize {
            0
        } else {
            self.glyph_blobs.get(w.native_glyph_info_handle(self)).gids[index as usize]
        }
    }

    /// The protrusion code of the first or last glyph of native word `w`.
    pub fn get_native_word_cp(&self, w: NodePtr<WhatsitNode>, side: c_int) -> i32 {
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
    /// The layout engine of OpenType/Graphite font `f`.
    pub(crate) fn layout_engine(&self, f: i32) -> Option<&FontEngineHandle> {
        if self.font_area_entry(f) == OTGR_FONT_FLAG {
            self.font_layout_engine[f as usize].as_deref()
        } else {
            None
        }
    }

    /// `layout_engine`, shared, for callers that need the state meanwhile.
    pub(crate) fn layout_engine_arc(&self, f: i32) -> Option<Arc<FontEngineHandle>> {
        if self.font_area_entry(f) == OTGR_FONT_FLAG {
            self.font_layout_engine[f as usize].clone()
        } else {
            None
        }
    }

    /// The layout engine of OpenType/Graphite font `f`, or `Internal(msg)` for
    /// any other font.
    fn require_ot_engine(&self, f: i32, msg: &'static str) -> Result<&FontEngineHandle, Abort> {
        self.layout_engine(f).ok_or(Abort::Internal(msg))
    }

    /// Shape native word `w` and store its width, height, depth and glyphs.
    pub fn measure_native_node(
        &self,
        mem: &mut MemState,
        w: NodePtr<WhatsitNode>,
        use_glyph_metrics: bool,
    ) -> Result<(), Abort> {
        let text = w.native_text(mem).to_vec();
        let text_len = text.len() as i32;
        let f = w.native_font(mem).0;
        let engine = self.require_ot_engine(f, "bad native font flag in `measure_native_node`")?;

        let bidi = Bidi::new(&text, engine.default_direction() as u8);
        let mut shaped = ShapedGlyphs::default();
        let dir = bidi.direction();
        let width = if dir == UBIDI_MIXED {
            let n_runs = bidi.count_runs();
            let mut total = 0;
            for run in 0..n_runs {
                total += layout_visual_run(&bidi, engine, &text, run);
            }
            let (mut x, mut y) = (0.0f64, 0.0f64);
            if total > 0 {
                for run in 0..n_runs {
                    let n_glyphs = layout_visual_run(&bidi, engine, &text, run);
                    let end = shaped.append_run(engine, n_glyphs, x, y);
                    x += end.x as f64;
                    y += end.y as f64;
                }
            }
            x
        } else {
            let n_glyphs = engine.layout_chars(&text, 0, text_len, dir == UBIDI_RTL);
            let end = shaped.append_run(engine, n_glyphs, 0.0, 0.0);
            if n_glyphs > 0 {
                end.x as f64
            } else {
                0.0
            }
        };
        w.set_width(mem, Scaled(d2fix(width)));
        w.set_native_glyph_count(mem, shaped.gids.len() as u16);
        drop(bidi);

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
                let y = fix2d(-loc.y) as f32;
                let bbox = cached_or_measured_bbox(engine, f, gid);
                let ht = bbox.y_max;
                let dp = -bbox.y_min;
                if y + ht > y_max {
                    y_max = y + ht;
                }
                if y - dp < y_min {
                    y_min = y - dp;
                }
            }
            w.set_height(mem, Scaled(d2fix(y_max as f64)));
            w.set_depth(mem, Scaled(-d2fix(y_min as f64)));
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

    pub fn get_native_italic_correction(&self, mem: &MemState, w: NodePtr<WhatsitNode>) -> i32 {
        let f = w.native_font(mem).0;
        let n = w.native_glyph_count(mem) as usize;
        if n > 0 {
            let last_glyph = mem.glyph_blobs.get(w.native_glyph_info_handle(mem)).gids[n - 1];
            if let Some(engine) = self.layout_engine(f) {
                return d2fix(engine.glyph_ital_corr(last_glyph as u32) as f64)
                    + self.font_letter_space_entry(f);
            }
        }
        0
    }

    pub fn get_native_glyph_italic_correction(
        &self,
        mem: &MemState,
        w: NodePtr<WhatsitNode>,
    ) -> i32 {
        let gid = w.native_glyph(mem);
        let f = w.native_font(mem).0;
        match self.layout_engine(f) {
            Some(engine) => d2fix(engine.glyph_ital_corr(gid as u32) as f64),
            None => 0,
        }
    }

    pub fn measure_native_glyph(
        &self,
        mem: &mut MemState,
        w: NodePtr<WhatsitNode>,
        use_glyph_metrics: bool,
    ) -> Result<(), Abort> {
        let gid = w.native_glyph(mem) as u32;
        let f = w.native_font(mem).0;
        let engine = self.require_ot_engine(f, "bad native font flag in `measure_native_glyph`")?;
        w.set_width(mem, Scaled(d2fix(engine.font().glyph_width(gid) as f64)));
        if use_glyph_metrics {
            let (ht, dp) = engine.glyph_height_depth(gid);
            w.set_height(mem, Scaled(d2fix(ht as f64)));
            w.set_depth(mem, Scaled(d2fix(dp as f64)));
        } else {
            w.set_height(mem, Scaled(self.height_base_entry(f)));
            w.set_depth(mem, Scaled(self.depth_base_entry(f)));
        }
        Ok(())
    }

    /// Height and depth of character `ch` of native font `font`.
    pub fn get_native_char_height_depth(&self, font: i32, ch: i32) -> Result<(i32, i32), Abort> {
        let engine = self.require_ot_engine(
            font,
            "bad native font flag in `get_native_char_height_depth`",
        )?;
        let gid = engine.map_char_to_glyph(ch as u32);
        let (ht, dp) = engine.glyph_height_depth(gid);
        let mut height = d2fix(ht as f64);
        let mut depth = d2fix(dp as f64);

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

    pub fn native_char_height(&self, f: i32, c: i32) -> Result<i32, Abort> {
        Ok(self.get_native_char_height_depth(f, c)?.0)
    }

    pub fn native_char_depth(&self, f: i32, c: i32) -> Result<i32, Abort> {
        Ok(self.get_native_char_height_depth(f, c)?.1)
    }

    /// Left and right side bearings of character `ch` of native font `font`.
    pub fn get_native_char_sidebearings(&self, font: i32, ch: i32) -> Result<(i32, i32), Abort> {
        let engine = self.require_ot_engine(
            font,
            "bad native font flag in `get_native_char_side_bearings`",
        )?;
        let gid = engine.map_char_to_glyph(ch as u32);
        let (l, r) = engine.glyph_sidebearings(gid);
        Ok((d2fix(l as f64), d2fix(r as f64)))
    }

    /// Glyph bound `edge` of glyph `gid`: 1 left, 2 top, 3 right, 4 bottom.
    pub fn get_glyph_bounds(&self, font: i32, edge: i32, gid: i32) -> Result<i32, Abort> {
        let engine = self.require_ot_engine(font, "bad native font flag in `get_glyph_bounds`")?;
        let (a, b) = if edge & 1 != 0 {
            engine.glyph_sidebearings(gid as u32)
        } else {
            engine.glyph_height_depth(gid as u32)
        };
        Ok(d2fix(if edge <= 2 { a } else { b } as f64))
    }

    pub fn native_char_italic_correction(&self, f: i32, c: i32) -> Result<i32, Abort> {
        let (_, rsb) = self.get_native_char_sidebearings(f, c)?;
        let letter_space = self.font_letter_space_entry(f);
        Ok(if rsb < 0 {
            letter_space - rsb
        } else {
            letter_space
        })
    }

    pub fn native_char_width(&self, f: i32, c: i32) -> Result<i32, Abort> {
        let engine = self.require_ot_engine(f, "bad native font flag in `get_native_char_wd`")?;
        let gid = engine.map_char_to_glyph(c as u32);
        Ok(d2fix(engine.glyph_width(gid) as f64))
    }

    pub fn map_char_to_glyph(&self, font: i32, ch: i32) -> Result<i32, Abort> {
        if ch > 0x10ffff || (0xd800..=0xdfff).contains(&ch) {
            return Ok(0);
        }
        let engine = self.require_ot_engine(font, "bad native font flag in `map_char_to_glyph`")?;
        Ok(engine.map_char_to_glyph(ch as u32) as i32)
    }

    pub fn map_glyph_to_index(&self, font: i32, name: &CStr) -> Result<i32, Abort> {
        let engine =
            self.require_ot_engine(font, "bad native font flag in `map_glyph_to_index`")?;
        Ok(engine.map_glyph_to_index(name))
    }

    pub fn get_font_char_range(&self, font: i32, first: i32) -> Result<i32, Abort> {
        let engine =
            self.require_ot_engine(font, "bad native font flag in `get_font_char_range'`")?;
        Ok(engine.font_char_range(first))
    }

    pub(crate) fn release_linebreak_state(&mut self) {
        self.brk_iter = None;
        self.brk_locale_str_num = 0;
        self.saved_mapping_name = None;
    }

    /// Write the XDV native font definition of `f` into `fonts.xdv_buffer` and
    /// return its length.
    pub fn make_font_def(&mut self, f: i32) -> Result<c_int, Abort> {
        let engine = self.require_ot_engine(f, "bad native font flag in `make_font_def`")?;
        let (name, index) = engine.font_filename();

        let rgba = engine.rgb_value();
        let font_flags = self.font_flags_entry(f) as i8;
        let mut flags: u16 = 0;
        if font_flags & FONT_FLAGS_VERTICAL != 0 {
            flags |= XDV_FLAG_VERTICAL;
        }
        let extend = engine.extend_factor();
        let slant = engine.slant_factor();
        let embolden = engine.embolden_factor();
        let size = d2fix(engine.point_size() as f64);

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
                def.extend_from_slice(&(d2fix(v as f64) as u32).to_be_bytes());
            }
        }

        if self.xdv_buffer.len() < def.len() {
            self.xdv_buffer.resize(def.len(), 0);
        }
        self.xdv_buffer[..def.len()].copy_from_slice(&def);
        Ok(def.len() as c_int)
    }

    /// Run `text` through TECkit converter `cnv` into `mapped_text`, returning
    /// the mapped length.
    pub fn apply_mapping(&mut self, cnv: &FontMappingHandle, text: &[u16]) -> c_int {
        let txt_bytes = text.len() as u32 * 2;

        let mut needed = (txt_bytes + 32) as usize;
        if self.mapped_text.len() < needed / 2 {
            self.mapped_text.resize(needed / 2, 0);
        }

        loop {
            let (status, out_used) = cnv.convert_units(text, &mut self.mapped_text);
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
    pub fn store_justified_native_glyphs(
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
        let just_amount = fix2d(saved_width - width);
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
                glyphs.locations[i].x = d2fix(fix2d(glyphs.locations[i].x) + adjustment);
                if is_space(glyphs.gids[i]) {
                    space_index += 1;
                    adjustment = just_amount * space_index as f64 / space_count as f64;
                }
            }
        } else {
            for i in 1..glyph_count {
                glyphs.locations[i].x = d2fix(
                    fix2d(glyphs.locations[i].x)
                        + just_amount * i as f64 / (glyph_count - 1) as f64,
                );
            }
        }

        w.set_width(mem, Scaled(saved_width));
        Ok(())
    }
}

pub fn aat_get_font_metrics() -> NativeFontMetrics {
    NativeFontMetrics::default()
}

pub fn aat_font_get(_what: c_int) -> c_int {
    -1
}

pub fn aat_font_get_1(_what: c_int, _param: c_int) -> c_int {
    -1
}

pub fn aat_font_get_2(_what: c_int, _param1: c_int, _param2: c_int) -> c_int {
    -1
}

pub fn aat_font_get_named(_what: c_int) -> c_int {
    -1
}

pub fn aat_font_get_named_1(_what: c_int, _param: c_int) -> c_int {
    -1
}

pub(crate) fn print_glyph_name(st: &mut EngineState, font: i32, gid: i32) -> Result<(), Abort> {
    let engine = st
        .fonts
        .require_ot_engine(font, "bad native font flag in `print_glyph_name`")?;
    let name = engine.font().glyph_name(gid as u16);
    for &b in &name {
        crate::output::print_char(st, b as c_char as i32);
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
pub(crate) fn insert_special(st: &mut EngineState, text: &str) -> Result<(), Abort> {
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
                st.fonts
                    .measure_native_glyph(&mut st.mem, w, use_glyph_metrics)?;
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

    #[test]
    fn read_double_parses_sign_whitespace_and_fraction() {
        let mut input = &b" \t-12.375pt"[..];
        let value = read_double(&mut input);
        assert!((value + 12.375).abs() < 1e-12);
        assert_eq!(input, b"pt");
    }

    #[test]
    fn read_rgb_a_accepts_rgb_and_optional_alpha() {
        let mut rgb = &b"a1B2c3!"[..];
        assert_eq!(read_rgb_a(&mut rgb), 0xA1B2C3FF);
        assert_eq!(rgb, b"!");

        let mut rgba = &b"0102037f!"[..];
        assert_eq!(read_rgb_a(&mut rgba), 0x0102037F);
        assert_eq!(rgba, b"!");
    }

    #[test]
    fn read_rgb_a_invalid_base_color_returns_black_and_stops() {
        let mut input = &b"12xx"[..];
        assert_eq!(read_rgb_a(&mut input), 0x000000FF);
        assert_eq!(input, b"xx");
    }

    #[test]
    fn read_feature_number_requires_number_setting_and_full_consumption() {
        assert_eq!(read_feature_number(b"123 =45"), Some((123, 45)));
        assert_eq!(read_feature_number(b"123 =45x"), None);
    }

    #[test]
    fn split_font_name_matches_bracket_and_unbracketed_forms() {
        assert_eq!(
            split_font_name(b"[font/name.ttf:2]:color=FF0000"),
            SplitName {
                var: 14,
                feat: 17,
                end: 30,
                index: 2
            }
        );
        assert_eq!(
            split_font_name(b"Font/Bold:script=latn"),
            SplitName {
                var: 4,
                feat: 9,
                end: 21,
                index: 0
            }
        );
    }
}
