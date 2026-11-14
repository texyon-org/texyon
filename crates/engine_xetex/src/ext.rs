//! The native-font layer: OpenType/Graphite font loading and feature parsing,
//! TECkit mappings, shaping and measuring native word and glyph nodes, and the
//! extension (`\openout`, `\write`, `\special`, ...) commands.

use crate::bridge::{input_get_size, input_open, input_read, with_bridge, FileFormat};
use crate::codes::{ConvertCode, LastItemCode};
use crate::errors::Abort;
use crate::fonts::FontSize;
use crate::fonts::OTGR_FONT_FLAG;
use crate::io::{AUTO, ICUMAPPING, RAW, UTF16BE, UTF16LE, UTF8};
use crate::layout::{hb_tag, FontMappingHandle};
use crate::params::IntPar;
use crate::state::{EngineState, FontEngineHandle};
use crate::units::{FontId, Scaled};
use std::ffi::c_int;
use std::sync::Arc;
use texyon_bridge_harfbuzz::Feature;
use texyon_xetex_layout::{
    Engine, EngineOptions, Font, FontManager, LayoutEngine, PlatformFontRef,
};

mod extension;
mod native;

pub(crate) use self::extension::{
    append_src_special, do_extension, insert_special, insert_src_special,
};

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
/// Font-wide metrics of a native font, in scaled points.
#[derive(Default)]
pub(crate) struct NativeFontMetrics {
    pub ascent: Scaled,
    pub descent: Scaled,
    pub x_height: Scaled,
    pub cap_height: Scaled,
    pub slant: Scaled,
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

pub(crate) fn d2fix(d: f64) -> Scaled {
    Scaled::from_raw((d * 65536.0 + 0.5) as i32)
}

pub(crate) fn fix2d(f: Scaled) -> f64 {
    f64::from(f.raw()) / 65536.0
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
        val = val * 10.0 + f64::from(at(s, i) - b'0');
        i += 1;
    }

    if at(s, i) == b'.' {
        let mut dec = 10.0;
        i += 1;
        while at(s, i).is_ascii_digit() {
            val += f64::from(at(s, i) - b'0') / dec;
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

/// The input mode named `bytes`, with the ICU converter name string
/// for `ICUMAPPING`.
fn get_encoding_mode_and_info(st: &mut EngineState, bytes: &[u8]) -> Result<(c_int, i32), Abort> {
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

    if crate::io::icu_converter_exists(bytes).is_err() {
        crate::output::begin_diagnostic(st);
        crate::output::print_nl(st, b'U');
        crate::output::print(st, "nknown encoding `");
        crate::output::print(st, bytes);
        crate::output::print(st, "'; reading as raw bytes");
        crate::output::end_diagnostic(st, true);
        Ok((RAW, 0))
    } else {
        let info = st.strings.make_tex_string(bytes)?;
        Ok((ICUMAPPING, info.raw()))
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
            param = param * 10 + c_int::from(at(s, i) - b'0');
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
        f = f * 10 + u32::from(at(spec, i) - b'0');
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
        v = v * 10 + c_int::from(at(spec, i) - b'0');
        i += 1;
    }
    while at(spec, i) == b' ' || at(spec, i) == b'\t' {
        i += 1;
    }
    (i == spec.len()).then_some((f, v))
}

/// Load TECkit mapping `name` (plus `.tec`) for font `font_name`, mapping to
/// bytes for TFM fonts or to UTF-16 for native ones.
pub(crate) fn load_mapping_file(
    st: &mut EngineState,
    name: &[u8],
    byte_mapping: bool,
    font_name: &[u8],
) -> Result<Option<Arc<FontMappingHandle>>, Abort> {
    let file_name = [name, b".tec"].concat();
    let name_len = file_name
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(file_name.len());
    let name = &file_name[..name_len];

    let map = input_open(name, FileFormat::MiscFonts, false);
    if map == 0 {
        crate::output::font_mapping_warning(st, name, font_name, 1);
        return Ok(None);
    }

    let mapping_size = input_get_size(map);
    let mut mapping = vec![0u8; mapping_size];
    if !input_read(map, &mut mapping) {
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
        None => crate::output::font_mapping_warning(st, name, font_name, 2),
        Some(_) if st.eqtb.int_par(IntPar::XetexTracingFonts) > 1 => {
            crate::output::font_mapping_warning(st, name, font_name, 0)
        }
        Some(_) => {}
    }
    Ok(cnv.map(Arc::new))
}

/// Split a TFM font name at a `:mapping=` suffix into the name to look up and
/// the mapping, if one is named.
pub(crate) fn split_tfm_font_mapping(path: &[u8]) -> (&[u8], Option<&[u8]>) {
    const KEY: &[u8] = b":mapping=";
    let Some(at) = path.windows(KEY.len()).position(|w| w == KEY) else {
        return (path, None);
    };
    let value = &path[at + KEY.len()..];
    // C's signed `char` also skips bytes 0x80 and up.
    let start = value.iter().position(|&b| b as i8 > b' ' as i8);
    (&path[..at], start.map(|start| &value[start..]))
}

pub(crate) fn apply_tfm_font_mapping(cnv: &FontMappingHandle, c: c_int) -> c_int {
    let input = (c as u16).to_ne_bytes();
    let mut out = [0u8; 2];
    let (_, out_used) = cnv.convert(&input, &mut out);
    if out_used < 1 {
        0
    } else {
        c_int::from(out[0])
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
    font_name: &[u8],
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
        st.fonts.loaded_font_mapping =
            load_mapping_file(st, &feat[start + value..end], false, font_name)?;
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
/// the feature string `feat`. Warnings name the font `font_name`.
fn load_ot_font(
    st: &mut EngineState,
    mut font: Font,
    scaled_size: Scaled,
    feat: Option<&[u8]>,
    font_name: &[u8],
) -> Result<Option<FontEngineHandle>, Abort> {
    let mut script = HB_TAG_NONE;
    let mut language: Option<&[u8]> = None;
    let mut features: Vec<Feature> = Vec::new();
    let mut shapers: Vec<&[u8]> = Vec::new();
    let mut opts = CommonFeatures {
        extend: 1.0,
        slant: 0.0,
        embolden: 0.0,
        letterspace: 0.0,
        rgb_value: 0x000000FF,
    };
    let req_engine = FontManager::with_font_manager(|mgr| mgr.get_req_engine());

    match req_engine {
        Engine::OpenType => shapers.push(b"ot"),
        Engine::Graphite => shapers.push(b"graphite2"),
        _ => {}
    }

    // Graphite feature names are resolved against a probe engine.
    let probe = (req_engine == Engine::Graphite)
        .then(|| LayoutEngine::graphite_probe(&font, script, opts.rgb_value));

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
                    language = Some(&feat[cp1 + value..cp2]);
                    break 'option Some(true);
                }
                if let Some(value) = option_value(option, b"shaper") {
                    let Ok(value) = value else {
                        break 'option Some(false);
                    };
                    shapers.push(&feat[cp1 + value..cp2]);
                    break 'option Some(true);
                }
                if let Some(valid) = read_common_feature(st, font_name, feat, cp1, cp2, &mut opts)?
                {
                    break 'option Some(valid);
                }
                if let Some(probe) = &probe
                    && let Some((tag, value)) =
                        read_feature_number(spec).or_else(|| probe.find_graphite_feature(spec))
                {
                    features.push(Feature::global(tag, value as u32));
                    break 'option Some(true);
                }
                if at(feat, cp1) == b'+' {
                    let (tag, mut param) = read_tag_with_param(&feat[cp1 + 1..]);
                    if param >= 0 {
                        param += 1;
                    }
                    features.push(Feature::global(tag, param as u32));
                    break 'option Some(true);
                }
                if at(feat, cp1) == b'-' {
                    let tag = hb_tag(&feat[cp1 + 1..cp2]);
                    features.push(Feature::global(tag, 0));
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
                crate::output::font_feature_warning(st, spec, &[], font_name);
            }
            cp1 = cp2;
        }
    }

    drop(probe);

    if opts.embolden != 0.0 {
        opts.embolden = (f64::from(opts.embolden) * fix2d(scaled_size) / 100.0) as f32;
    }
    if opts.letterspace != 0.0 {
        st.fonts.loaded_font_letter_space = Scaled::from_raw(
            (f64::from(opts.letterspace) / 100.0 * f64::from(scaled_size.raw())) as i32,
        );
    }
    if st.fonts.loaded_font_flags & FONT_FLAGS_COLORED == 0 {
        opts.rgb_value = 0x000000FF;
    }
    if st.fonts.loaded_font_flags & FONT_FLAGS_VERTICAL != 0 {
        font.set_layout_dir_vertical(true);
    }

    let engine = LayoutEngine::new(
        font,
        EngineOptions {
            script,
            language,
            features,
            shapers: &shapers,
            rgb_value: opts.rgb_value,
            extend: opts.extend,
            slant: opts.slant,
            embolden: opts.embolden,
        },
    );
    st.fonts.native_font_type_flag = OTGR_FONT_FLAG;
    Ok(Some(engine))
}

/// The design size of the font the font manager loaded last.
pub(crate) fn loaded_font_design_size() -> Scaled {
    Scaled::from_raw(FontManager::with_font_manager(|mgr| mgr.font_design_size()))
}

fn set_loaded_font_design_size(size: Scaled) {
    FontManager::with_font_manager(|mgr| mgr.set_font_design_size(size.raw()));
}

fn set_req_engine(engine: Engine) {
    FontManager::with_font_manager(|mgr| mgr.set_req_engine(engine));
}

/// A path as the layout library saw it through a C string.
fn c_path(path: &[u8]) -> &str {
    if path.contains(&0) {
        ""
    } else {
        std::str::from_utf8(path).expect("native font paths are UTF-8")
    }
}

/// The font in file `path` at face `index`.
fn font_from_file(path: &[u8], index: c_int, point_size: Scaled) -> Option<Font> {
    let path = c_path(path);
    with_bridge(|io| Font::from_file(path, index as usize, point_size.raw(), io))
}

fn font_from_platform(font_ref: &PlatformFontRef, point_size: Scaled) -> Option<Font> {
    with_bridge(|io| Font::from_platform(font_ref.clone(), point_size.raw(), io))
}

/// The point size to load at, `size` resolved against `font`'s design size.
/// Without a font it stays in TeX's raw form, which the loaders also read.
fn point_size(font: Option<Font>, size: FontSize) -> Scaled {
    let Some(font) = font else {
        return size.raw();
    };
    let dsize = d2fix(font.design_size());
    match size {
        FontSize::DESIGN => dsize,
        _ => size.of_design(dsize).quotient,
    }
}

/// Look up native font `name` (a `[file]` path or an installed font name with
/// `/variant` and `:features`) and build its layout engine. Also returns the
/// font's full name: `name` for a file, the installed font's name otherwise.
pub(crate) fn find_native_font(
    st: &mut EngineState,
    name: &[u8],
    size: FontSize,
) -> Result<Option<(FontEngineHandle, Vec<u8>)>, Abort> {
    let mut rval = None;

    st.fonts.loaded_font_mapping = None;
    st.fonts.loaded_font_flags = 0;
    st.fonts.loaded_font_letter_space = Scaled::ZERO;

    let split = split_font_name(name);
    let name_string = &name[..split.var];
    let mut var_string = (split.feat > split.var).then(|| name[split.var + 1..split.feat].to_vec());
    let feat_string = (split.end > split.feat).then(|| name[split.feat + 1..split.end].to_vec());

    if at(name_string, 0) == b'[' {
        let path = &name_string[1..];
        let scaled_size = match size {
            FontSize::At(s) => s,
            FontSize::Scaled(_) => {
                point_size(font_from_file(path, split.index, Scaled::UNITY * 10), size)
            }
        };
        if let Some(font) = font_from_file(path, split.index, scaled_size) {
            set_loaded_font_design_size(d2fix(font.design_size()));
            let found = font.found_name().to_vec();

            set_req_engine(Engine::Default);
            if let Some(var) = &var_string {
                if var.starts_with(b"/AAT") {
                    set_req_engine(Engine::Apple);
                } else if var.starts_with(b"/OT") || var.starts_with(b"/ICU") {
                    set_req_engine(Engine::OpenType);
                } else if var.starts_with(b"/GR") {
                    set_req_engine(Engine::Graphite);
                }
            }

            rval = load_ot_font(st, font, scaled_size, feat_string.as_deref(), name)?
                .map(|engine| (engine, name.to_vec()));
            if rval.is_some() && st.eqtb.int_par(IntPar::XetexTracingFonts) > 0 {
                crate::output::begin_diagnostic(st);
                crate::output::print_nl(st, b' ');
                crate::output::print_raw_bytes(st, b"-> ");
                crate::output::print_raw_bytes(st, &found);
                crate::output::end_diagnostic(st, false);
            }
        }
    } else {
        let found = with_bridge(|io| {
            FontManager::with_font_manager(|mgr| {
                mgr.find_font(name_string, var_string.as_mut(), fix2d(size.raw()), io)
            })
        });
        if let Some(font_ref) = found {
            if st.eqtb.int_par(IntPar::XetexTracingFonts) > 0 {
                crate::output::begin_diagnostic(st);
                crate::output::print_nl(st, b' ');
                crate::output::print_raw_bytes(st, b"-> ");
                let desc = FontManager::with_font_manager(|mgr| {
                    mgr.get_platform_font_desc(&font_ref).to_bytes().to_vec()
                });
                crate::output::print_raw_bytes(st, &desc);
                crate::output::end_diagnostic(st, false);
            }

            let mut full_name = FontManager::with_font_manager(|mgr| {
                mgr.get_full_name(&font_ref).to_bytes().to_vec()
            });

            let scaled_size = match size {
                FontSize::At(s) => s,
                FontSize::Scaled(_) => point_size(font_from_platform(&font_ref, size.raw()), size),
            };

            let mut engine = None;
            if let Some(font) = font_from_platform(&font_ref, scaled_size) {
                engine = load_ot_font(st, font, scaled_size, feat_string.as_deref(), &full_name)?;
            }

            if let Some(var) = var_string.as_deref().filter(|v| !v.is_empty()) {
                full_name.push(b'/');
                full_name.extend_from_slice(var);
            }
            if let Some(feat) = feat_string.as_deref().filter(|f| !f.is_empty()) {
                full_name.push(b':');
                full_name.extend_from_slice(feat);
            }
            rval = engine.map(|engine| (engine, full_name));
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
                        index = index * 10 + c_int::from(name[i] - b'0');
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
    let ascent = d2fix(f64::from(a));
    let descent = d2fix(f64::from(d));

    let slant = d2fix(
        fix2d(Scaled::from_raw(engine.font().slant())) * f64::from(engine.extend())
            + f64::from(engine.slant()),
    );

    let (cap, x) = engine.cap_and_x_height();
    let mut cap_height = d2fix(f64::from(cap));
    let mut x_height = d2fix(f64::from(x));

    // Fall back to the height of `x` and `X`, else to fractions of the ascent.
    if x_height == Scaled::ZERO {
        let glyph_id = engine.map_char_to_glyph(u32::from(b'x'));
        x_height = if glyph_id != 0 {
            d2fix(f64::from(engine.glyph_height_depth(glyph_id).0))
        } else {
            ascent / 2
        };
    }
    if cap_height == Scaled::ZERO {
        let glyph_id = engine.map_char_to_glyph(u32::from(b'X'));
        cap_height = if glyph_id != 0 {
            d2fix(f64::from(engine.glyph_height_depth(glyph_id).0))
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

pub(crate) fn ot_font_get(what: LastItemCode, engine: &FontEngineHandle) -> i32 {
    let font = engine.font();
    match what {
        LastItemCode::XetexCountGlyphs => font.count_glyphs() as i32,
        LastItemCode::XetexCountFeatures => engine.count_graphite_features() as i32,
        LastItemCode::XetexOtCountScripts => font.count_scripts() as i32,
        _ => 0,
    }
}

pub(crate) fn ot_font_get_1(what: LastItemCode, engine: &FontEngineHandle, param: i32) -> i32 {
    let font = engine.font();
    match what {
        LastItemCode::XetexOtCountLanguages => font.count_languages(param as u32) as i32,
        LastItemCode::XetexOtScript => font.ind_script(param as u32) as i32,
        LastItemCode::XetexFeatureCode => engine.graphite_feature_code(param as u32) as i32,
        LastItemCode::XetexIsExclusiveFeature => 1,
        LastItemCode::XetexCountSelectors => {
            engine.count_graphite_feature_settings(param as u32) as i32
        }
        _ => 0,
    }
}

pub(crate) fn ot_font_get_2(
    what: LastItemCode,
    engine: &FontEngineHandle,
    param1: i32,
    param2: i32,
) -> i32 {
    let font = engine.font();
    match what {
        LastItemCode::XetexOtLanguage => font.ind_language(param1 as u32, param2 as u32) as i32,
        LastItemCode::XetexOtCountFeatures => {
            font.count_features(param1 as u32, param2 as u32) as i32
        }
        LastItemCode::XetexSelectorCode => {
            engine.graphite_feature_setting_code(param1 as u32, param2 as u32) as i32
        }
        LastItemCode::XetexIsDefaultSelector => {
            i32::from(engine.graphite_feature_default_setting(param1 as u32) == param2 as u32)
        }
        _ => 0,
    }
}

pub(crate) fn ot_font_get_3(
    what: LastItemCode,
    engine: &FontEngineHandle,
    param1: i32,
    param2: i32,
    param3: i32,
) -> i32 {
    match what {
        LastItemCode::XetexOtFeature => {
            engine
                .font()
                .ind_feature(param1 as u32, param2 as u32, param3 as u32) as i32
        }
        _ => 0,
    }
}

pub(crate) fn gr_print_font_name(
    st: &mut EngineState,
    what: ConvertCode,
    engine: &FontEngineHandle,
    param1: i32,
    param2: i32,
) {
    let name = match what {
        ConvertCode::XetexFeatureName => engine.graphite_feature_label(param1 as u32),
        ConvertCode::XetexSelectorName => {
            engine.graphite_feature_setting_label(param1 as u32, param2 as u32)
        }
        _ => None,
    };
    if let Some(name) = name {
        crate::output::print(st, name.as_slice());
    }
}

pub(crate) fn gr_font_get_named(what: LastItemCode, engine: &FontEngineHandle, name: &[u8]) -> i32 {
    if what != LastItemCode::XetexFindFeatureByName {
        return -1;
    }
    engine
        .find_graphite_feature_named(name)
        .map_or(-1, |id| id as i32)
}

pub(crate) fn gr_font_get_named_1(
    what: LastItemCode,
    engine: &FontEngineHandle,
    param: i32,
    name: &[u8],
) -> i32 {
    if what != LastItemCode::XetexFindSelectorByName {
        return -1;
    }
    engine
        .find_graphite_feature_setting_named(param as u32, name)
        .map_or(-1, i32::from)
}

pub fn aat_get_font_metrics() -> NativeFontMetrics {
    NativeFontMetrics::default()
}

pub fn aat_font_get(_what: LastItemCode) -> c_int {
    -1
}

pub fn aat_font_get_1(_what: LastItemCode, _param: c_int) -> c_int {
    -1
}

pub fn aat_font_get_2(_what: LastItemCode, _param1: c_int, _param2: c_int) -> c_int {
    -1
}

pub fn aat_font_get_named(_what: LastItemCode) -> c_int {
    -1
}

pub fn aat_font_get_named_1(_what: LastItemCode, _param: c_int) -> c_int {
    -1
}

pub(crate) fn print_glyph_name(st: &mut EngineState, font: FontId, gid: i32) -> Result<(), Abort> {
    let engine = st
        .fonts
        .require_ot_engine(font, "bad native font flag in `print_glyph_name`")?;
    let name = engine.font().glyph_name(gid as u16).unwrap_or_default();
    crate::output::print(st, name.as_bytes());
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
