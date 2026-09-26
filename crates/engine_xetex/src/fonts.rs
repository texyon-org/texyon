//! Font loading: the TFM reader, native (OpenType/Graphite) font loading, native
//! character nodes and the TFM `char_info` accessors.

use crate::consts::{
    EMPTY_STRING, FONT_BASE, INT_PAR_DEFAULT_HYPHEN_CHAR, INT_PAR_DEFAULT_SKEW_CHAR,
    INT_PAR_SUPPRESS_FONTNOTFOUND_ERROR, INT_PAR_TRACING_LOST_CHARS,
    INT_PAR_XETEX_GENERATE_ACTUAL_TEXT, INT_PAR_XETEX_TRACING_FONTS,
    INT_PAR_XETEX_USE_GLYPH_METRICS, INVALID_HANDLE, TOO_BIG_CHAR,
};
use crate::errors::ErrorOutcome;
use crate::ext::{
    aat_get_font_metrics, apply_mapping, apply_tfm_font_mapping, check_for_tfm_font_mapping_in,
    find_native_font_in, load_tfm_font_mapping_in, map_char_to_glyph, measure_native_node,
    ot_get_font_metrics, NativeFontAbort, NativeFontMetrics,
};
use crate::io::{pack_file_name_in, tt_xetex_open_input_in};
use crate::mem_types::B16x4;
use crate::node::{NodePtr, WhatsitNode, WhatsitSubtype};
use crate::output::{
    begin_diagnostic_in, capture_to_diagnostic_in, char_warning_in,
    diagnostic_begin_capture_warning_here_in, end_diagnostic_in, error_here_with_diagnostic_in,
    print_char_in, print_cstr_in, print_file_name_in, print_int_in, print_nl_cstr_in,
    print_scaled_in, sprint_cs_in,
};
use crate::packaging::int_par;
use crate::scan::{NodeRecycleAbort, AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::state::{EngineState, EqtbState, FontState, MemState};

use crate::bridge::ttstub_input_getc;

extern "C" {
    fn get_loaded_font_design_size() -> i32;
    fn isOpenTypeMathFont(engine: *mut libc::c_void) -> bool;
}

const EOF: i32 = -1;
const LIST_TAG: u16 = 2;
const NON_ADDRESS: i32 = 0;
const TTBC_FILE_FORMAT_TFM: libc::c_int = 3;
const NATIVE_NODE_SIZE: i32 = 6;

pub(crate) type CharInfo = B16x4;

pub(crate) fn char_width(fonts: &FontState, f: i32, i: CharInfo) -> i32 {
    fonts.font_info_b32s1(fonts.width_base_entry(f) + i.s3 as i32)
}

pub(crate) fn char_italic(fonts: &FontState, f: i32, i: CharInfo) -> i32 {
    fonts.font_info_b32s1(fonts.italic_base_entry(f) + i.s1 as i32 / 4)
}

pub(crate) fn char_height(fonts: &FontState, f: i32, i: CharInfo) -> i32 {
    fonts.font_info_b32s1(fonts.height_base_entry(f) + i.s2 as i32 / 16)
}

pub(crate) fn char_depth(fonts: &FontState, f: i32, i: CharInfo) -> i32 {
    fonts.font_info_b32s1(fonts.depth_base_entry(f) + i.s2 as i32 % 16)
}

pub fn effective_char(fonts: &mut FontState, _err_p: bool, f: i32, c: u16) -> i32 {
    let mut c = c;
    let mapping = fonts.font_mapping_ptr(f);
    if !fonts.xtx_ligature_present() && !mapping.is_null() {
        // SAFETY: `mapping` is the font's live TECkit converter.
        c = unsafe { apply_tfm_font_mapping(mapping, c as i32) } as u16;
    }
    fonts.set_xtx_ligature_present(false);
    c as i32
}

pub fn effective_char_info(fonts: &mut FontState, f: i32, c: u16) -> CharInfo {
    let c = effective_char(fonts, true, f, c);
    fonts.font_info_b16(fonts.char_base_entry(f) + c)
}

/// The UTF-16 encoding of `c`: one unit, or a surrogate pair above the BMP.
fn utf16_units(c: u32) -> ([u16; 2], usize) {
    if c > 65_535 {
        let v = c - 65_536;
        ([(v / 1024 + 0xD800) as u16, (v % 1024 + 0xDC00) as u16], 2)
    } else {
        ([c as u16, 0], 1)
    }
}

fn alloc_native_word(
    mem: &mut MemState,
    f: i32,
    n: i32,
    subtype: WhatsitSubtype,
) -> Result<NodePtr<WhatsitNode>, NodeRecycleAbort> {
    let size = NATIVE_NODE_SIZE + (n * 2 + 7) / 8;
    let w = NodePtr::<WhatsitNode>::alloc(mem, size, subtype).ok_or(NodeRecycleAbort::Overflow)?;
    w.set_native_size(mem, size as u16);
    w.set_native_font(mem, f as u16);
    w.set_native_length(mem, n as u16);
    w.set_native_glyph_count(mem, 0);
    w.set_native_glyph_info_handle(mem, 0);
    Ok(w)
}

/// A native-word node for `n` UTF-16 units of font `f`, text left unset.
pub(crate) fn new_native_word_node(
    mem: &mut MemState,
    eqtb: &EqtbState,
    f: i32,
    n: i32,
) -> Result<NodePtr<WhatsitNode>, NodeRecycleAbort> {
    let subtype = if int_par(eqtb, INT_PAR_XETEX_GENERATE_ACTUAL_TEXT) > 0 {
        WhatsitSubtype::NativeWordAt
    } else {
        WhatsitSubtype::NativeWord
    };
    alloc_native_word(mem, f, n, subtype)
}

/// Where a native character's UTF-16 text comes from.
enum NativeCharSrc {
    /// `len` units already in `FontState::mapped_text`.
    Mapped(i32),
    Single(u32),
}

fn build_native_character(
    mem: &mut MemState,
    eqtb: &EqtbState,
    fonts: &FontState,
    f: i32,
    src: NativeCharSrc,
) -> Result<NodePtr<WhatsitNode>, NodeRecycleAbort> {
    match src {
        NativeCharSrc::Mapped(len) => {
            let w = new_native_word_node(mem, eqtb, f, len)?;
            let src = &fonts.mapped_text[..len as usize];
            let text = w.native_text_ptr_mut(mem);
            // SAFETY: the node holds `len` units and the arena never reallocates.
            unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), text, src.len()) };
            Ok(w)
        }
        NativeCharSrc::Single(c) => {
            let (units, n) = utf16_units(c);
            let w = alloc_native_word(mem, f, n as i32, WhatsitSubtype::NativeWord)?;
            let text = w.native_text_ptr_mut(mem);
            // SAFETY: the node holds `n` units and the arena never reallocates.
            unsafe { std::ptr::copy_nonoverlapping(units.as_ptr(), text, n) };
            Ok(w)
        }
    }
}

pub(crate) enum NativeCharAbort {
    PoolOverflow,
    Recycle(NodeRecycleAbort),
    Error(ErrorOutcome),
    NativeFont(NativeFontAbort),
}

impl From<NativeFontAbort> for NativeCharAbort {
    fn from(e: NativeFontAbort) -> Self {
        NativeCharAbort::NativeFont(e)
    }
}

impl NativeCharAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            NativeCharAbort::PoolOverflow => crate::errors::EngineAbort::PoolOverflow,
            NativeCharAbort::Recycle(e) => e.into_engine_abort(),
            NativeCharAbort::Error(outcome) => outcome.into_engine_abort(),
            NativeCharAbort::NativeFont(e) => e.into_engine_abort(),
        }
    }
}

/// Warn about `c` when font `f` has no glyph for it.
fn check_native_char_in(st: &mut EngineState, f: i32, c: i32) -> Result<(), NativeCharAbort> {
    // SAFETY: font-table lookup over FFI, no state re-entry.
    if unsafe { map_char_to_glyph(&st.fonts, f, c) }? == 0 {
        let outcome = char_warning_in(st, f, c);
        if !matches!(outcome, ErrorOutcome::Continue) {
            return Err(NativeCharAbort::Error(outcome));
        }
    }
    Ok(())
}

/// A measured native-word node for character `c` of font `f`, run through the
/// font's TECkit mapping if it has one.
pub(crate) fn new_native_character_in(
    st: &mut EngineState,
    f: i32,
    c: u32,
) -> Result<NodePtr<WhatsitNode>, NativeCharAbort> {
    let src = if st.fonts.font_mapping[f as usize].is_some() {
        let (units, n) = utf16_units(c);
        if st.strings.pool_ptr + n as i32 > st.strings.pool_size {
            return Err(NativeCharAbort::PoolOverflow);
        }
        for unit in &units[..n] {
            st.strings.str_pool[st.strings.pool_ptr as usize] = *unit;
            st.strings.pool_ptr += 1;
        }

        let start = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
        let map = st.fonts.font_mapping_ptr(f);
        let cur_len = crate::hash::cur_length(&st.strings);
        // SAFETY: `dst` points into the non-reallocating string pool, disjoint
        // from the `&mut st.fonts` borrow.
        let len = unsafe {
            let dst = st.strings.str_pool.as_mut_ptr().add(start as usize);
            apply_mapping(&mut st.fonts, map, dst, cur_len)
        };
        st.strings.pool_ptr = start;

        let mut i = 0;
        while i < len {
            let mapped = st.fonts.mapped_text[i as usize];
            if (0xD800..0xDC00).contains(&mapped) {
                let low = st.fonts.mapped_text[(i + 1) as usize] as u32;
                let c = (mapped as u32 - 0xD800) * 1024 + low + 9216;
                check_native_char_in(st, f, c as i32)?;
                i += 2;
            } else {
                check_native_char_in(st, f, mapped as i32)?;
                i += 1;
            }
        }
        NativeCharSrc::Mapped(len)
    } else {
        if int_par(&st.eqtb, INT_PAR_TRACING_LOST_CHARS) > 0 {
            check_native_char_in(st, f, c as i32)?;
        }
        NativeCharSrc::Single(c)
    };

    let w = build_native_character(&mut st.mem, &st.eqtb, &st.fonts, f, src)
        .map_err(NativeCharAbort::Recycle)?;
    let use_glyph_metrics = int_par(&st.eqtb, INT_PAR_XETEX_USE_GLYPH_METRICS) > 0;
    // SAFETY: the measurer only reads font tables over re-entry-free FFI.
    unsafe { measure_native_node(&st.fonts, &mut st.mem, w, use_glyph_metrics)? };
    Ok(w)
}

unsafe fn tfm_getc(handle: usize) -> i32 {
    ttstub_input_getc(handle) as i32
}

/// The next four TFM bytes, or `None` at end of file.
unsafe fn read_tfm_bytes(handle: usize) -> Option<[i32; 4]> {
    let bytes = [
        tfm_getc(handle),
        tfm_getc(handle),
        tfm_getc(handle),
        tfm_getc(handle),
    ];
    (!bytes.contains(&EOF)).then_some(bytes)
}

fn four_quarters([a, b, c, d]: [i32; 4]) -> B16x4 {
    B16x4 {
        s0: d as u16,
        s1: c as u16,
        s2: b as u16,
        s3: a as u16,
    }
}

unsafe fn read_fifteen(handle: usize) -> Option<i32> {
    let x = tfm_getc(handle);
    if x > 127 || x == EOF {
        return None;
    }
    Some(x * 256 + tfm_getc(handle))
}

unsafe fn read_scaled_word(handle: usize, z: i32, beta: i32, alpha: i32) -> Option<i32> {
    let [a, b, c, d] = read_tfm_bytes(handle)?;
    let sw = (((((d * z) / 256) + c * z) / 256) + b * z) / beta;
    match a {
        0 => Some(sw),
        255 => Some(sw - alpha),
        _ => None,
    }
}

pub(crate) enum ReadFontInfoAbort {
    TfmOutOfMemory,
    Native(LoadNativeFontAbort),
    Error(ErrorOutcome),
    InputClose,
}

impl From<NativeFontAbort> for ReadFontInfoAbort {
    fn from(e: NativeFontAbort) -> Self {
        ReadFontInfoAbort::Native(LoadNativeFontAbort::NativeFont(e))
    }
}

impl ReadFontInfoAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            ReadFontInfoAbort::TfmOutOfMemory => crate::errors::EngineAbort::TtAbort(
                c"not enough memory to load another font".as_ptr(),
            ),
            ReadFontInfoAbort::Native(e) => e.into_engine_abort(),
            ReadFontInfoAbort::Error(outcome) => outcome.into_engine_abort(),
            ReadFontInfoAbort::InputClose => crate::errors::EngineAbort::TtAbort(
                c"serious internal error closing the font metric file".as_ptr(),
            ),
        }
    }
}

pub(crate) fn name_of_file_ptr(st: &mut EngineState) -> *mut libc::c_char {
    if st.input.name_of_file.is_empty() {
        std::ptr::null_mut()
    } else {
        st.input.name_of_file.as_mut_ptr()
    }
}

/// Start a "Font \cs=name at size" error.
fn font_error_start_in(st: &mut EngineState, u: i32, nom: i32, aire: i32, s: i32) {
    error_here_with_diagnostic_in(st, c"Font ".as_ptr());
    sprint_cs_in(st, u);
    print_char_in(st, b'=' as i32);
    let quote = st.scanner.file_name_quote_char;
    if quote != 0 {
        print_char_in(st, quote as i32);
    }
    print_file_name_in(st, nom, aire, st.scanner.cur_ext);
    if quote != 0 {
        print_char_in(st, quote as i32);
    }
    if s >= 0 {
        print_cstr_in(st, c" at ".as_ptr());
        print_scaled_in(st, s);
        print_cstr_in(st, c"pt".as_ptr());
    } else if s != -1000 {
        print_cstr_in(st, c" scaled ".as_ptr());
        print_int_in(st, -s);
    }
}

pub(crate) unsafe fn read_font_info_in(
    st: &mut EngineState,
    u: i32,
    nom: i32,
    aire: i32,
    s: i32,
) -> Result<i32, ReadFontInfoAbort> {
    let mut name_too_long = false;
    let mut file_opened = false;
    let mut g = FONT_BASE;
    let mut tfm_file = INVALID_HANDLE;

    pack_file_name_in(st, nom, aire, st.scanner.cur_ext);

    if int_par(&st.eqtb, INT_PAR_XETEX_TRACING_FONTS) > 0 {
        begin_diagnostic_in(st);
        diagnostic_begin_capture_warning_here_in(st);
        print_nl_cstr_in(st, c"Requested font \"".as_ptr());
        let name = name_of_file_ptr(st);
        print_cstr_in(st, name);
        print_char_in(st, b'"' as i32);
        if s < 0 {
            print_cstr_in(st, c" scaled ".as_ptr());
            print_int_in(st, -s);
        } else {
            print_cstr_in(st, c" at ".as_ptr());
            print_scaled_in(st, s);
            print_cstr_in(st, c"pt".as_ptr());
        }
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        end_diagnostic_in(st, false);
    }

    let mut bad_tfm = false;
    'read: {
        macro_rules! bad {
            () => {{
                bad_tfm = true;
                break 'read;
            }};
        }

        if st.scanner.quoted_filename {
            g = load_native_font_in(st, u, nom, aire, s).map_err(ReadFontInfoAbort::Native)?;
            if g != FONT_BASE {
                break 'read;
            }
        }

        name_too_long = crate::string_pool::length(&st.strings, nom) > 255
            || crate::string_pool::length(&st.strings, aire) > 255;
        if name_too_long {
            bad!();
        }

        pack_file_name_in(st, nom, aire, EMPTY_STRING);
        let name_of_file = name_of_file_ptr(st);
        check_for_tfm_font_mapping_in(&mut st.fonts, name_of_file);

        tfm_file = tt_xetex_open_input_in(st, TTBC_FILE_FORMAT_TFM);
        if tfm_file == INVALID_HANDLE {
            if !st.scanner.quoted_filename {
                g = load_native_font_in(st, u, nom, aire, s).map_err(ReadFontInfoAbort::Native)?;
                if g != FONT_BASE {
                    break 'read;
                }
            }
            bad!();
        }
        file_opened = true;

        let (parse, scale_side_effect) =
            read_font_info_tfm(&mut st.fonts, &st.eqtb, tfm_file, nom, aire, s);
        if let Some((remainder, overflowed)) = scale_side_effect {
            st.errors.tex_remainder = remainder;
            if overflowed {
                st.errors.arith_error = true;
            }
        }
        match parse {
            TfmParse::OutOfMemory => return Err(ReadFontInfoAbort::TfmOutOfMemory),
            TfmParse::Bad => bad!(),
            TfmParse::Loaded(f) => {
                g = f;
                let mapping = load_tfm_font_mapping_in(st)?;
                st.fonts.set_font_mapping(f, mapping);
            }
        }
    }

    if bad_tfm && int_par(&st.eqtb, INT_PAR_SUPPRESS_FONTNOTFOUND_ERROR) == 0 {
        font_error_start_in(st, u, nom, aire, s);
        let reason = if file_opened {
            c" not loadable: Bad metric (TFM) file"
        } else if name_too_long {
            c" not loadable: Metric (TFM) file name too long"
        } else {
            c" not loadable: Metric (TFM) file or installed font not found"
        };
        print_cstr_in(st, reason.as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());

        st.errors.set_help(&[
            c"I wasn't able to read the size data for this font,",
            c"so I will ignore the font specification.",
            c"[Wizards can fix TFM files using TFtoPL/PLtoTF.]",
            c"You might try inserting a different font spec;",
            c"e.g., type `I\\font<same font id>=<substitute font name>'.",
        ]);

        let outcome = crate::errors::error_in(st);
        if !matches!(outcome, ErrorOutcome::Continue) {
            return Err(ReadFontInfoAbort::Error(outcome));
        }
    }

    if file_opened && crate::io::input_close(tfm_file) {
        return Err(ReadFontInfoAbort::InputClose);
    }

    if int_par(&st.eqtb, INT_PAR_XETEX_TRACING_FONTS) > 0 && (g == FONT_BASE || file_opened) {
        begin_diagnostic_in(st);
        diagnostic_begin_capture_warning_here_in(st);
        print_nl_cstr_in(st, c" -> ".as_ptr());
        if g == FONT_BASE {
            print_cstr_in(st, c"font not found, using \"nullfont\"".as_ptr());
        } else {
            let name = name_of_file_ptr(st);
            print_cstr_in(st, name);
        }
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        end_diagnostic_in(st, false);
    }

    Ok(g)
}

enum TfmParse {
    Loaded(i32),
    Bad,
    OutOfMemory,
}

/// Read the opened TFM file into the tables of the next font slot. Also returns
/// the (remainder, overflow) side effect of size scaling for the caller to apply.
unsafe fn read_font_info_tfm(
    fonts: &mut FontState,
    eqtb: &EqtbState,
    tfm_file: usize,
    nom: i32,
    aire: i32,
    s: i32,
) -> (TfmParse, Option<(i32, bool)>) {
    let mut scale_side_effect: Option<(i32, bool)> = None;
    macro_rules! bad {
        () => {
            return (TfmParse::Bad, scale_side_effect)
        };
    }
    macro_rules! read {
        ($e:expr) => {
            match $e {
                Some(v) => v,
                None => bad!(),
            }
        };
    }

    let mut lf = read!(read_fifteen(tfm_file));
    let mut lh = read!(read_fifteen(tfm_file));
    let mut bc = read!(read_fifteen(tfm_file));
    let mut ec = read!(read_fifteen(tfm_file));
    if bc > ec + 1 || ec > 255 {
        bad!();
    }
    if bc > 255 {
        bc = 1;
        ec = 0;
    }
    let nw = read!(read_fifteen(tfm_file));
    let nh = read!(read_fifteen(tfm_file));
    let nd = read!(read_fifteen(tfm_file));
    let ni = read!(read_fifteen(tfm_file));
    let nl = read!(read_fifteen(tfm_file));
    let nk = read!(read_fifteen(tfm_file));
    let ne = read!(read_fifteen(tfm_file));
    let np = read!(read_fifteen(tfm_file));

    if lf != 6 + lh + (ec - bc + 1) + nw + nh + nd + ni + nl + nk + ne + np {
        bad!();
    }
    if nw == 0 || nh == 0 || nd == 0 || ni == 0 {
        bad!();
    }

    lf = lf - 6 - lh;
    if np < 7 {
        lf = lf + 7 - np;
    }

    if fonts.font_ptr == fonts.font_max || fonts.fmem_ptr + lf > fonts.font_mem_size {
        return (TfmParse::OutOfMemory, None);
    }

    let f = fonts.font_ptr + 1;
    fonts.set_char_base_entry(f, fonts.fmem_ptr - bc);
    fonts.set_width_base_entry(f, fonts.char_base_entry(f) + ec + 1);
    fonts.set_height_base_entry(f, fonts.width_base_entry(f) + nw);
    fonts.set_depth_base_entry(f, fonts.height_base_entry(f) + nh);
    fonts.set_italic_base_entry(f, fonts.depth_base_entry(f) + nd);
    fonts.set_lig_kern_base_entry(f, fonts.italic_base_entry(f) + ni);
    fonts.set_kern_base_entry(f, fonts.lig_kern_base_entry(f) + nl - 256 * 128);
    fonts.set_exten_base_entry(f, fonts.kern_base_entry(f) + 256 * 128 + nk);
    fonts.set_param_base(f, fonts.exten_base_entry(f) + ne);

    if lh < 2 {
        bad!();
    }

    fonts.set_font_check(f, four_quarters(read!(read_tfm_bytes(tfm_file))));

    let mut z = read!(read_fifteen(tfm_file));
    z = z * 256 + tfm_getc(tfm_file);
    z = (z * 16) + (tfm_getc(tfm_file) / 16);
    if z < 65_536 {
        bad!();
    }

    while lh > 2 {
        for _ in 0..4 {
            tfm_getc(tfm_file);
        }
        lh -= 1;
    }

    fonts.set_font_dsize_entry(f, z);
    if s != -1000 {
        if s >= 0 {
            z = s;
        } else {
            let (q, rem, overflowed) = crate::scaled_math::xn_over_d_parts(z, -s, 1000);
            z = q;
            scale_side_effect = Some((rem, overflowed));
        }
    }
    fonts.set_font_size_entry(f, z);

    let char_exists = |fonts: &FontState, ch: i32| {
        (bc..=ec).contains(&ch) && fonts.font_info_b16(fonts.char_base_entry(f) + ch).s3 != 0
    };

    for k in fonts.fmem_ptr..fonts.width_base_entry(f) {
        let bytes = read!(read_tfm_bytes(tfm_file));
        let [a, b, c, d] = bytes;
        fonts.set_font_info_b16(k, four_quarters(bytes));

        if a >= nw || b / 16 >= nh || b % 16 >= nd || c / 4 >= ni {
            bad!();
        }
        match c % 4 {
            1 if d >= nl => bad!(),
            3 if d >= ne => bad!(),
            2 => {
                if d < bc || d > ec {
                    bad!();
                }
                // A charlist must not cycle back to the current character.
                let current = k + bc - fonts.fmem_ptr;
                let mut dd = d;
                while dd < current {
                    let qw = fonts.font_info_b16(fonts.char_base_entry(f) + dd);
                    if qw.s1 % 4 != LIST_TAG {
                        break;
                    }
                    dd = qw.s0 as i32;
                }
                if dd == current {
                    bad!();
                }
            }
            _ => {}
        }
    }

    let mut alpha = 16;
    while z >= 0x800000 {
        z /= 2;
        alpha += alpha;
    }
    let beta = 256 / alpha;
    alpha *= z;

    for k in fonts.width_base_entry(f)..fonts.lig_kern_base_entry(f) {
        let sw = read!(read_scaled_word(tfm_file, z, beta, alpha));
        fonts.set_font_info_b32s1(k, sw);
    }

    if fonts.font_info_b32s1(fonts.width_base_entry(f)) != 0
        || fonts.font_info_b32s1(fonts.height_base_entry(f)) != 0
        || fonts.font_info_b32s1(fonts.depth_base_entry(f)) != 0
        || fonts.font_info_b32s1(fonts.italic_base_entry(f)) != 0
    {
        bad!();
    }

    let mut bch_label = 32_767;
    let mut bchar = 256;

    if nl > 0 {
        let lig_kern_base = fonts.lig_kern_base_entry(f);
        let lig_kern_end = fonts.kern_base_entry(f) + 256 * 128;
        for k in lig_kern_base..lig_kern_end {
            let bytes = read!(read_tfm_bytes(tfm_file));
            let [a, b, c, d] = bytes;
            fonts.set_font_info_b16(k, four_quarters(bytes));

            if a > 128 {
                if 256 * c + d >= nl {
                    bad!();
                }
                if a == 255 && k == lig_kern_base {
                    bchar = b;
                }
            } else {
                if b != bchar && !char_exists(fonts, b) {
                    bad!();
                }
                if c < 128 {
                    if !char_exists(fonts, d) {
                        bad!();
                    }
                } else if 256 * (c - 128) + d >= nk {
                    bad!();
                }
                if a < 128 && k - lig_kern_base + a + 1 >= nl {
                    bad!();
                }
            }

            if k == lig_kern_end - 1 && a == 255 {
                bch_label = 256 * c + d;
            }
        }
    }

    for k in fonts.kern_base_entry(f) + 256 * 128..fonts.exten_base_entry(f) {
        let sw = read!(read_scaled_word(tfm_file, z, beta, alpha));
        fonts.set_font_info_b32s1(k, sw);
    }

    for k in fonts.exten_base_entry(f)..fonts.param_base(f) {
        let bytes = read!(read_tfm_bytes(tfm_file));
        let [a, b, c, d] = bytes;
        fonts.set_font_info_b16(k, four_quarters(bytes));

        for ch in [a, b, c] {
            if ch != 0 && !char_exists(fonts, ch) {
                bad!();
            }
        }
        if !char_exists(fonts, d) {
            bad!();
        }
    }

    for k in 1..=np {
        let sw = if k == 1 {
            // The slant is an unscaled fix_word.
            let mut sw = tfm_getc(tfm_file);
            if sw == EOF {
                bad!();
            }
            if sw > 127 {
                sw -= 256;
            }
            sw = sw * 256 + tfm_getc(tfm_file);
            sw = sw * 256 + tfm_getc(tfm_file);
            (sw * 16) + (tfm_getc(tfm_file) / 16)
        } else {
            read!(read_scaled_word(tfm_file, z, beta, alpha))
        };
        fonts.set_font_info_b32s1(fonts.param_base(f) + k - 1, sw);
    }
    for k in np + 1..=7 {
        fonts.set_font_info_b32s1(fonts.param_base(f) + k - 1, 0);
    }

    fonts.set_font_params(f, np.max(7));
    fonts.set_hyphen_char_entry(f, int_par(eqtb, INT_PAR_DEFAULT_HYPHEN_CHAR));
    fonts.set_skew_char_entry(f, int_par(eqtb, INT_PAR_DEFAULT_SKEW_CHAR));
    if bch_label < nl {
        fonts.set_bchar_label_entry(f, bch_label + fonts.lig_kern_base_entry(f));
    } else {
        fonts.set_bchar_label_entry(f, NON_ADDRESS);
    }
    fonts.set_font_bchar_entry(f, bchar);
    fonts.set_font_false_bchar_entry(f, bchar);
    if bchar <= ec && bchar >= bc && fonts.font_info_b16(fonts.char_base_entry(f) + bchar).s3 > 0 {
        fonts.set_font_false_bchar_entry(f, TOO_BIG_CHAR);
    }

    fonts.set_font_name_entry(f, nom);
    fonts.set_font_area_entry(f, aire);
    fonts.set_font_bc_entry(f, bc as u16);
    fonts.set_font_ec_entry(f, ec as u16);
    fonts.set_font_glue(f, crate::mem::TEX_NULL);
    fonts.set_param_base(f, fonts.param_base(f) - 1);
    fonts.fmem_ptr += lf;
    fonts.font_ptr = f;
    (TfmParse::Loaded(f), scale_side_effect)
}

/// Set up the tables of a new native font slot. Returns the slot and the
/// font's metrics.
unsafe fn init_native_font_tables(
    fonts: &mut FontState,
    eqtb: &EqtbState,
    font_engine: crate::state::FontEngineHandle,
    full_name: i32,
    actual_size: i32,
    num_font_dimens: i32,
) -> (i32, NativeFontMetrics) {
    fonts.font_ptr += 1;
    let font_ptr = fonts.font_ptr;
    fonts.set_font_area_entry(font_ptr, fonts.native_font_type_flag);
    fonts.set_font_name_entry(font_ptr, full_name);
    fonts.set_font_check(font_ptr, B16x4::default());
    fonts.set_font_glue(font_ptr, crate::mem::TEX_NULL);
    fonts.set_font_dsize_entry(font_ptr, get_loaded_font_design_size());
    fonts.set_font_size_entry(font_ptr, actual_size);

    let metrics = if fonts.native_font_type_flag == AAT_FONT_FLAG {
        aat_get_font_metrics(font_engine.raw)
    } else {
        ot_get_font_metrics(font_engine.raw)
    };

    fonts.set_height_base_entry(font_ptr, metrics.ascent);
    fonts.set_depth_base_entry(font_ptr, -metrics.descent);
    fonts.set_font_params(font_ptr, num_font_dimens);
    fonts.set_font_bc_entry(font_ptr, 0);
    fonts.set_font_ec_entry(font_ptr, 65_535);
    fonts.set_font_used_entry(font_ptr, false);
    fonts.set_hyphen_char_entry(font_ptr, int_par(eqtb, INT_PAR_DEFAULT_HYPHEN_CHAR));
    fonts.set_skew_char_entry(font_ptr, int_par(eqtb, INT_PAR_DEFAULT_SKEW_CHAR));
    fonts.set_param_base(font_ptr, fonts.fmem_ptr - 1);
    fonts.set_font_layout_engine(font_ptr, Some(std::sync::Arc::new(font_engine)));
    fonts.set_font_mapping(font_ptr, None);
    fonts.set_font_letter_space_entry(font_ptr, fonts.loaded_font_letter_space);

    (font_ptr, metrics)
}

pub(crate) enum LoadNativeFontAbort {
    PoolOverflow,
    StringOverflow(NodeRecycleAbort),
    NativeChar(NativeCharAbort),
    Error(ErrorOutcome),
    NativeFont(NativeFontAbort),
}

impl From<NativeFontAbort> for LoadNativeFontAbort {
    fn from(e: NativeFontAbort) -> Self {
        LoadNativeFontAbort::NativeFont(e)
    }
}

impl LoadNativeFontAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            LoadNativeFontAbort::PoolOverflow => crate::errors::EngineAbort::PoolOverflow,
            LoadNativeFontAbort::StringOverflow(e) => e.into_engine_abort(),
            LoadNativeFontAbort::NativeChar(e) => e.into_engine_abort(),
            LoadNativeFontAbort::Error(outcome) => outcome.into_engine_abort(),
            LoadNativeFontAbort::NativeFont(e) => e.into_engine_abort(),
        }
    }
}

fn push_font_info(fonts: &mut FontState, v: i32) {
    fonts.set_font_info_b32s1(fonts.fmem_ptr, v);
    fonts.fmem_ptr += 1;
}

/// Load a native OpenType/Graphite font and set up its TeX font tables.
pub(crate) unsafe fn load_native_font_in(
    st: &mut EngineState,
    u: i32,
    nom: i32,
    aire: i32,
    s: i32,
) -> Result<i32, LoadNativeFontAbort> {
    let name_of_file = name_of_file_ptr(st);
    let Some(font_engine) = find_native_font_in(st, name_of_file, s)? else {
        return Ok(FONT_BASE);
    };

    let actual_size = if s >= 0 {
        s
    } else if s != -1000 {
        crate::scaled_math::xn_over_d_in(&mut st.errors, get_loaded_font_design_size(), -s, 1000)
    } else {
        get_loaded_font_design_size()
    };

    if st.strings.pool_ptr + st.input.name_length > st.strings.pool_size {
        return Err(LoadNativeFontAbort::PoolOverflow);
    }
    for k in 0..st.input.name_length {
        st.strings.str_pool[st.strings.pool_ptr as usize] =
            st.input.name_of_file[k as usize] as u8 as u16;
        st.strings.pool_ptr += 1;
    }
    let full_name = crate::string_pool::make_string(&mut st.strings)
        .map_err(LoadNativeFontAbort::StringOverflow)?;

    for f in FONT_BASE + 1..=st.fonts.font_ptr {
        if st.fonts.font_area[f as usize] == st.fonts.native_font_type_flag
            && crate::string_pool::str_eq_str(
                &st.strings,
                st.fonts.font_name[f as usize],
                full_name,
            )
            && st.fonts.font_size[f as usize] == actual_size
        {
            drop(font_engine);
            st.strings.str_ptr -= 1;
            st.strings.pool_ptr =
                st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            return Ok(f);
        }
    }

    let num_font_dimens = if st.fonts.native_font_type_flag == OTGR_FONT_FLAG
        && isOpenTypeMathFont(font_engine.raw)
    {
        65
    } else {
        8
    };

    if st.fonts.font_ptr == st.fonts.font_max
        || st.fonts.fmem_ptr + num_font_dimens > st.fonts.font_mem_size
    {
        font_error_start_in(st, u, nom, aire, s);
        print_cstr_in(st, c" not loaded: Not enough room left".as_ptr());
        capture_to_diagnostic_in(st, std::ptr::null_mut());

        st.errors.set_help(&[
            c"I'm afraid I won't be able to make use of this font,",
            c"because my memory for character-size data is too small.",
            c"If you're really stuck, ask a wizard to enlarge me.",
            c"Or maybe try `I\\font<same font id>=<name of loaded font>'.",
        ]);

        let outcome = crate::errors::error_in(st);
        if !matches!(outcome, ErrorOutcome::Continue) {
            return Err(LoadNativeFontAbort::Error(outcome));
        }
        return Ok(FONT_BASE);
    }

    let (font_ptr, metrics) = init_native_font_tables(
        &mut st.fonts,
        &st.eqtb,
        font_engine,
        full_name,
        actual_size,
        num_font_dimens,
    );

    let space = new_native_character_in(st, font_ptr, b' ' as u32)
        .map_err(LoadNativeFontAbort::NativeChar)?;
    let space_width = space.width(&st.mem).0 + st.fonts.loaded_font_letter_space;
    space.free_native(&mut st.mem);

    let size = st.fonts.font_size[font_ptr as usize];
    for v in [
        metrics.slant,
        space_width,
        space_width / 2,
        space_width / 3,
        metrics.x_height,
        size,
        space_width / 3,
        metrics.cap_height,
    ] {
        push_font_info(&mut st.fonts, v);
    }

    if num_font_dimens == 65 {
        push_font_info(&mut st.fonts, num_font_dimens);
        for k in 0..=55 {
            let v = crate::ot_math::get_ot_math_constant(&st.fonts, font_ptr, k);
            push_font_info(&mut st.fonts, v);
        }
    }

    let staged_mapping = st.fonts.loaded_font_mapping.take();
    st.fonts.set_font_mapping(font_ptr, staged_mapping);
    st.fonts.font_flags[font_ptr as usize] = st.fonts.loaded_font_flags as i8;
    Ok(font_ptr)
}
