//! Font loading — TFM reader and native (OpenType/AAT/Graphite) font loading,
//! plus the `effective_char` accessors. Relocated from `scan.rs` (= xetex0.c's
//! font-loading group) by workstream 02 (module decomposition).

use crate::consts::{INT_BASE, TOO_BIG_CHAR};
use crate::errors::ErrorOutcome;
use crate::ext::{
    aat_get_font_metrics, apply_mapping, apply_tfm_font_mapping, check_for_tfm_font_mapping_in,
    find_native_font_in, load_tfm_font_mapping_in, map_char_to_glyph, measure_native_node,
    ot_get_font_metrics, release_font_engine,
};
use crate::io::{pack_file_name_in, tt_xetex_open_input_in};
use crate::output::char_warning_in;
use crate::scan::cnl_get_node;
use crate::scan::NodeRecycleAbort;
use crate::state::{EngineState, FontState};

extern "C" {
    fn _tt_abort(fmt: *const libc::c_char) -> !;
    fn ttstub_input_getc(handle: usize) -> libc::c_int;
    fn ttstub_input_close(handle: usize) -> libc::c_int;
    fn get_loaded_font_design_size() -> i32;
    fn isOpenTypeMathFont(engine: *mut libc::c_void) -> bool;
    fn get_ot_math_constant(f: i32, n: i32) -> i32;
}

// ---- moved verbatim from scan.rs (workstream 02) ----

/// translated from xetex-xetex0.c:4457 (effective_char)
///
/// Phase-C: threaded to `&mut FontState`; legacy callers use
/// [`effective_char_global`].
pub fn effective_char(fonts: &mut FontState, _err_p: bool, f: i32, c: u16) -> i32 {
    let mut c = c;
    let mapping = fonts.font_mapping_ptr(f);
    if !fonts.xtx_ligature_present() && !mapping.is_null() {
        // SAFETY: preserves the legacy call into the external TECkit mapping layer.
        c = unsafe { apply_tfm_font_mapping(mapping, c as i32) } as u16;
    }
    fonts.set_xtx_ligature_present(false);
    c as i32
}
/// translated from xetex-xetex0.c:6434 (effective_char_info)
///
/// Phase-C: threaded to `&mut FontState`; legacy callers use
/// [`effective_char_info_global`].
pub fn effective_char_info(fonts: &mut FontState, f: i32, c: u16) -> crate::mem_types::B16x4 {
    let c = effective_char(fonts, true, f, c);
    fonts.font_info_b16(fonts.char_base_entry(f) + c)
}
/// Allocate a native-word whatsit node for `n` UTF-16 code units of font `f`.
///
/// translated from xetex-xetex0.c:6250 (new_native_word_node)
///
/// Phase-C: threaded to `&mut MemState` (+ a disjoint `&EqtbState` read for the
/// `xetex_generate_actual_text` int-param). Bucket **B**: pure mem allocation
/// over the threaded [`crate::mem::get_node`], propagating the `01b` §5
/// [`NodeRecycleAbort::Overflow`] sentinel; [`new_native_word_node_global`]
/// runs the cold `overflow()` abort after the borrow drops.
pub(crate) fn new_native_word_node(
    mem: &mut crate::state::MemState,
    eqtb: &crate::state::EqtbState,
    f: i32,
    n: i32,
) -> Result<i32, NodeRecycleAbort> {
    // sizeof(UTF16_code) = 2, sizeof(memory_word) = 8
    let l = NATIVE_NODE_SIZE_NN + (n * 2 + 7) / 8;
    let q = cnl_get_node!(mem, l);
    mem.set_b16s1(q, WHATSIT_NODE_NN);
    if eqtb.b32s1(INT_BASE + INT_PAR_XETEX_GENERATE_ACTUAL_TEXT) > 0 {
        mem.set_b16s0(q, NATIVE_WORD_NODE_AT_NN);
    } else {
        mem.set_b16s0(q, NATIVE_WORD_NODE_NN);
    }
    mem.set_b16s3(q + 4, l as u16);
    mem.set_b16s2(q + 4, f as u16);
    mem.set_b16s1(q + 4, n as u16);
    mem.set_b16s0(q + 4, 0);
    mem.set_ptr(q + 5, std::ptr::null_mut());
    Ok(q)
}
/// Phase-C threaded mem core of [`new_native_character`]: allocate the native
/// node and fill its UTF-16 payload under a narrow `&mut MemState` (+ disjoint
/// `&EqtbState` for `new_native_word_node`, `&FontState` for the mapped text).
/// Bucket **B**; propagates the `01b` §5 [`NodeRecycleAbort::Overflow`] sentinel.
/// All the `with_state`-bearing work (string pool, TECkit mapping, lost-char
/// warnings, native measurement) stays in the [`new_native_character`] wrapper,
/// outside this borrow.
fn build_native_character(
    mem: &mut crate::state::MemState,
    eqtb: &crate::state::EqtbState,
    fonts: &crate::state::FontState,
    f: i32,
    src: NativeCharSrc,
) -> Result<i32, NodeRecycleAbort> {
    match src {
        NativeCharSrc::Mapped(len) => {
            let p = new_native_word_node(mem, eqtb, f, len)?;
            let text = mem.as_u16_ptr_mut(p + NATIVE_NODE_SIZE_NN);
            for i in 0..len {
                // SAFETY: node sized for `len` UTF-16 units; `i < len`. The
                // arena never reallocates, so `text` stays valid.
                unsafe {
                    *text.add(i as usize) = fonts.mapped_text[i as usize];
                }
            }
            Ok(p)
        }
        NativeCharSrc::Single(c) => {
            let p = cnl_get_node!(mem, NATIVE_NODE_SIZE_NN + 1);
            mem.set_b16s1(p, WHATSIT_NODE_NN);
            mem.set_b16s0(p, NATIVE_WORD_NODE_NN);
            mem.set_b16s3(p + 4, (NATIVE_NODE_SIZE_NN + 1) as u16);
            mem.set_b16s0(p + 4, 0);
            mem.set_ptr(p + 5, std::ptr::null_mut());
            mem.set_b16s2(p + 4, f as u16);
            if c > 65_535 {
                mem.set_b16s1(p + 4, 2);
            } else {
                mem.set_b16s1(p + 4, 1);
            }
            let text = mem.as_u16_ptr_mut(p + NATIVE_NODE_SIZE_NN);
            // SAFETY: node sized for up to 2 UTF-16 units; arena never moves.
            unsafe {
                if c > 65_535 {
                    *text.add(0) = ((c - 65_536) / 1024 + 0xD800) as u16;
                    *text.add(1) = ((c - 65_536) % 1024 + 0xDC00) as u16;
                } else {
                    *text.add(0) = c as u16;
                }
            }
            Ok(p)
        }
    }
}
/// Cold-abort sentinel (§5) for [`new_native_character_in`]: the threaded core
/// returns this and the [`new_native_character`] wrapper runs the divergent tail
/// *after* the state borrow drops.
pub(crate) enum NativeCharAbort {
    /// String-pool capacity exhausted while staging the mapped text.
    PoolOverflow,
    /// `build_native_character` ran out of `mem`.
    Recycle(NodeRecycleAbort),
    /// A lost-char `char_warning` tripped the error machinery. Every non-`Continue`
    /// [`ErrorOutcome`] is divergent in `finish()`, so the legacy global
    /// `char_warning` would have aborted at this exact point too (before building
    /// the node) — deferring it to the wrapper is behaviour-identical.
    Error(ErrorOutcome),
}

impl NativeCharAbort {
    /// Map to the unified [`EngineAbort`] terminal (`01d` §5.5), mirroring the
    /// former `finish()` cold-abort tail without longjmp-ing. Call only *after*
    /// the producing state borrow has dropped (`PoolOverflow` re-reads state).
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            NativeCharAbort::PoolOverflow => {
                let (pool_size, init_pool_ptr) =
                    crate::state::with_state(|st| (st.strings.pool_size, st.strings.init_pool_ptr));
                crate::errors::EngineAbort::Overflow {
                    what: c"pool size".as_ptr(),
                    n: pool_size - init_pool_ptr,
                }
            }
            NativeCharAbort::Recycle(e) => e.into_engine_abort(),
            NativeCharAbort::Error(outcome) => outcome.into_engine_abort(),
        }
    }
}

/// `&mut EngineState` core of [`new_native_character`]. Threaded so it can be
/// reached from a caller that already holds a state borrow (the hyphenation /
/// line-break / math measurer paths — the 135/136 FFI re-entry wall): the
/// string-pool staging, TECkit mapping, lost-char `char_warning_in`, node build,
/// and native measurement all run under one borrow, calling the threaded
/// `measure_native_node` / `map_char_to_glyph` / `char_warning_in` cores directly
/// rather than the re-entrant `_global` bridges. Cold aborts ride
/// [`NativeCharAbort`] (§5).
pub(crate) fn new_native_character_in(
    st: &mut crate::state::EngineState,
    f: i32,
    mut c: u32,
) -> Result<i32, NativeCharAbort> {
    let src = if !st.fonts.font_mapping[f as usize].is_null() {
        if c > 65_535 {
            if st.strings.pool_ptr + 2 > st.strings.pool_size {
                return Err(NativeCharAbort::PoolOverflow);
            }
            st.strings.str_pool[st.strings.pool_ptr as usize] =
                ((c - 65_536) / 1024 + 0xD800) as u16;
            st.strings.pool_ptr += 1;
            st.strings.str_pool[st.strings.pool_ptr as usize] =
                ((c - 65_536) % 1024 + 0xDC00) as u16;
            st.strings.pool_ptr += 1;
        } else {
            if st.strings.pool_ptr + 1 > st.strings.pool_size {
                return Err(NativeCharAbort::PoolOverflow);
            }
            st.strings.str_pool[st.strings.pool_ptr as usize] = c as u16;
            st.strings.pool_ptr += 1;
        }

        let start = st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
        let map = st.fonts.font_mapping[f as usize];
        let cl = crate::hash::cur_length(&st.strings);
        // SAFETY: TECkit mapping over a raw pointer into the (non-reallocating)
        // string pool; identical to the legacy `str_pool_ptr(start)` call. `dst`
        // points into `st.strings`, disjoint from the `&mut st.fonts` borrow.
        let len = unsafe {
            let dst = st.strings.str_pool.as_mut_ptr().add(start as usize);
            apply_mapping(&mut st.fonts, map, dst, cl)
        };
        st.strings.pool_ptr = start;

        let mut i = 0;
        while i < len {
            let mapped = st.fonts.mapped_text[i as usize];
            if (0xD800..0xDC00).contains(&mapped) {
                c = ((mapped as u32 - 0xD800) * 1024)
                    + st.fonts.mapped_text[(i + 1) as usize] as u32
                    + 9216;
                // SAFETY: pure font-table + FFI glyph lookup, no state re-entry.
                if unsafe { map_char_to_glyph(&st.fonts, f, c as i32) } == 0 {
                    let outcome = char_warning_in(st, f, c as i32);
                    if !matches!(outcome, ErrorOutcome::Continue) {
                        return Err(NativeCharAbort::Error(outcome));
                    }
                }
                i += 2;
            } else {
                if unsafe { map_char_to_glyph(&st.fonts, f, mapped as i32) } == 0 {
                    let outcome = char_warning_in(st, f, mapped as i32);
                    if !matches!(outcome, ErrorOutcome::Continue) {
                        return Err(NativeCharAbort::Error(outcome));
                    }
                }
                i += 1;
            }
        }

        NativeCharSrc::Mapped(len)
    } else {
        if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_LOST_CHARS_NC) > 0
            && unsafe { map_char_to_glyph(&st.fonts, f, c as i32) } == 0
        {
            let outcome = char_warning_in(st, f, c as i32);
            if !matches!(outcome, ErrorOutcome::Continue) {
                return Err(NativeCharAbort::Error(outcome));
            }
        }

        NativeCharSrc::Single(c)
    };

    let p = build_native_character(&mut st.mem, &st.eqtb, &st.fonts, f, src)
        .map_err(NativeCharAbort::Recycle)?;

    // SAFETY: the C++ measurer only touches the passed `&FontState` + pure C FFI
    // (re-entry-free); `node_addr(p)` points into the non-reallocating mem arena.
    unsafe {
        measure_native_node(
            &st.fonts,
            st.mem.node_addr(p),
            (st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS_NC) > 0) as libc::c_int,
        );
    }
    Ok(p)
}

unsafe fn tfm_getc(handle: usize) -> i32 {
    ttstub_input_getc(handle) as i32
}
unsafe fn read_tfm_word(handle: usize) -> Option<(crate::mem_types::B16x4, i32, i32, i32, i32)> {
    let a = tfm_getc(handle);
    let b = tfm_getc(handle);
    let c = tfm_getc(handle);
    let d = tfm_getc(handle);
    if a == EOF_RFI || b == EOF_RFI || c == EOF_RFI || d == EOF_RFI {
        None
    } else {
        Some((
            crate::mem_types::B16x4 {
                s0: d as u16,
                s1: c as u16,
                s2: b as u16,
                s3: a as u16,
            },
            a,
            b,
            c,
            d,
        ))
    }
}
unsafe fn read_fifteen(handle: usize) -> Option<i32> {
    let mut x = tfm_getc(handle);
    if x > 127 || x == EOF_RFI {
        return None;
    }
    x *= 256;
    x += tfm_getc(handle);
    Some(x)
}
unsafe fn read_scaled_word(handle: usize, z: i32, beta: i32, alpha: i32) -> Option<i32> {
    let a = tfm_getc(handle);
    let b = tfm_getc(handle);
    let c = tfm_getc(handle);
    let d = tfm_getc(handle);
    if a == EOF_RFI || b == EOF_RFI || c == EOF_RFI || d == EOF_RFI {
        return None;
    }
    let sw = (((((d * z) / 256) + c * z) / 256) + b * z) / beta;
    if a == 0 {
        Some(sw)
    } else if a == 255 {
        Some(sw - alpha)
    } else {
        None
    }
}
pub(crate) enum ReadFontInfoAbort {
    TfmOutOfMemory,
    Native(LoadNativeFontAbort),
    Error(ErrorOutcome),
}

impl ReadFontInfoAbort {
    /// Map to the unified [`EngineAbort`] terminal (`01d` §5.5), mirroring the
    /// former `finish()` cold-abort tail without longjmp-ing.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            ReadFontInfoAbort::TfmOutOfMemory => crate::errors::EngineAbort::TtAbort(
                c"not enough memory to load another font".as_ptr(),
            ),
            ReadFontInfoAbort::Native(e) => e.into_engine_abort(),
            ReadFontInfoAbort::Error(outcome) => outcome.into_engine_abort(),
        }
    }
}

fn name_of_file_ptr(st: &mut EngineState) -> *mut libc::c_char {
    if st.input.name_of_file.is_empty() {
        std::ptr::null_mut()
    } else {
        st.input.name_of_file.as_mut_ptr()
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

    if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_TRACING_FONTS) > 0 {
        crate::output::begin_diagnostic_in(st);
        crate::output::diagnostic_begin_capture_warning_here_in(st);
        crate::output::print_nl_cstr_in(st, c"Requested font \"".as_ptr());
        let name = name_of_file_ptr(st);
        crate::output::print_cstr_in(st, name);
        crate::output::print_char_in(st, b'"' as i32);
        if s < 0 {
            crate::output::print_cstr_in(st, c" scaled ".as_ptr());
            crate::output::print_int_in(st, -s);
        } else {
            crate::output::print_cstr_in(st, c" at ".as_ptr());
            crate::output::print_scaled_in(st, s);
            crate::output::print_cstr_in(st, c"pt".as_ptr());
        }
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        crate::output::end_diagnostic_in(st, false);
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
        check_for_tfm_font_mapping_in(name_of_file_ptr(st));

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

        // Phase-C (session 129): the pure TFM parse + font-table fill runs
        // under one `&mut FontState` borrow (plus a disjoint `&EqtbState` read
        // for the two default-char int-params). The cold `_tt_abort` overflow
        // path and the with_state-bearing `load_tfm_font_mapping` are kept
        // OUTSIDE the borrow (01b §4a/§5 borrow-dropping).
        let (parse, scale_side_effect) =
            read_font_info_tfm(&mut st.fonts, &st.eqtb, tfm_file, nom, aire, s);
        // Apply the borrow-free `xn_over_d` side effects (01b §4a). Timing is
        // observably identical to the original: nothing between the scaling
        // point and here reads `tex_remainder`/`arith_error`.
        if let Some((remainder, overflowed)) = scale_side_effect {
            st.errors.tex_remainder = remainder;
            if overflowed {
                st.errors.arith_error = true;
            }
        }
        match parse {
            TfmParse::OutOfMemory => {
                return Err(ReadFontInfoAbort::TfmOutOfMemory);
            }
            TfmParse::Bad => bad!(),
            TfmParse::Loaded(f) => {
                g = f;
                let mapping = load_tfm_font_mapping_in(st);
                st.fonts.set_font_mapping_ptr(f, mapping);
            }
        }
    }

    if bad_tfm
        && st
            .eqtb
            .b32s1(INT_BASE + INT_PAR_SUPPRESS_FONTNOTFOUND_ERROR_RFI)
            == 0
    {
        crate::output::error_here_with_diagnostic_in(st, c"Font ".as_ptr());
        crate::output::sprint_cs_in(st, u);
        crate::output::print_char_in(st, b'=' as i32);
        if st.scanner.file_name_quote_char != 0 {
            crate::output::print_char_in(st, st.scanner.file_name_quote_char as i32);
        }
        crate::output::print_file_name_in(st, nom, aire, st.scanner.cur_ext);
        if st.scanner.file_name_quote_char != 0 {
            crate::output::print_char_in(st, st.scanner.file_name_quote_char as i32);
        }
        if s >= 0 {
            crate::output::print_cstr_in(st, c" at ".as_ptr());
            crate::output::print_scaled_in(st, s);
            crate::output::print_cstr_in(st, c"pt".as_ptr());
        } else if s != -1000 {
            crate::output::print_cstr_in(st, c" scaled ".as_ptr());
            crate::output::print_int_in(st, -s);
        }

        if file_opened {
            crate::output::print_cstr_in(st, c" not loadable: Bad metric (TFM) file".as_ptr());
        } else if name_too_long {
            crate::output::print_cstr_in(
                st,
                c" not loadable: Metric (TFM) file name too long".as_ptr(),
            );
        } else {
            crate::output::print_cstr_in(
                st,
                c" not loadable: Metric (TFM) file or installed font not found".as_ptr(),
            );
        }
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());

        st.errors.help_ptr = 5;
        st.errors.help_line[4] = c"I wasn't able to read the size data for this font,".as_ptr();
        st.errors.help_line[3] = c"so I will ignore the font specification.".as_ptr();
        st.errors.help_line[2] = c"[Wizards can fix TFM files using TFtoPL/PLtoTF.]".as_ptr();
        st.errors.help_line[1] = c"You might try inserting a different font spec;".as_ptr();
        st.errors.help_line[0] =
            c"e.g., type `I\\font<same font id>=<substitute font name>'.".as_ptr();

        let outcome = crate::errors::error_in(st);
        if !matches!(outcome, ErrorOutcome::Continue) {
            return Err(ReadFontInfoAbort::Error(outcome));
        }
    }

    if file_opened {
        ttstub_input_close(tfm_file);
    }

    if st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_TRACING_FONTS) > 0 && (g == FONT_BASE || file_opened)
    {
        crate::output::begin_diagnostic_in(st);
        crate::output::diagnostic_begin_capture_warning_here_in(st);

        crate::output::print_nl_cstr_in(st, c" -> ".as_ptr());
        if g == FONT_BASE {
            crate::output::print_cstr_in(st, c"font not found, using \"nullfont\"".as_ptr());
        } else {
            let name = name_of_file_ptr(st);
            crate::output::print_cstr_in(st, name);
        }

        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        crate::output::end_diagnostic_in(st, false);
    }

    Ok(g)
}

/// Outcome of the threaded TFM-parse core ([`read_font_info_tfm`]).
enum TfmParse {
    /// Metrics parsed and font tables filled; the new font is `f`.
    Loaded(i32),
    /// The TFM file is malformed; the caller takes the `bad_tfm` error path.
    Bad,
    /// The font tables are full (the `01b` §5 cold abort sentinel): the caller
    /// runs `_tt_abort` after the `&mut FontState` borrow has dropped.
    OutOfMemory,
}

/// Phase-C threaded core of [`read_font_info`]: read the opened TFM file
/// (`tfm_file`) and fill the font-metric tables for the next font slot under a
/// single `&mut FontState` borrow (plus a disjoint `&EqtbState` for the two
/// default hyphen/skew char int-params). Touches **no** with_state-bearing
/// helper — the cold `_tt_abort` overflow path returns [`TfmParse::OutOfMemory`]
/// and the trailing `load_tfm_font_mapping` write stays in the wrapper (01b §4a).
///
/// translated from the metric-reading body of xetex-xetex0.c:6350 (read_font_info)
unsafe fn read_font_info_tfm(
    fonts: &mut FontState,
    eqtb: &crate::state::EqtbState,
    tfm_file: usize,
    nom: i32,
    aire: i32,
    s: i32,
) -> (TfmParse, Option<(i32, bool)>) {
    // Captured side effect of the size-scaling `xn_over_d` (remainder, overflow);
    // applied by the wrapper after the borrow drops (01b §4a). `None` until the
    // `s < 0` scaling branch runs; returned on every exit so the `bad_tfm` path
    // preserves the original `tex_remainder`/`arith_error` writes.
    let mut scale_side_effect: Option<(i32, bool)> = None;
    macro_rules! bad {
        () => {
            return (TfmParse::Bad, scale_side_effect)
        };
    }

    let mut lf = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    let mut lh = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    let mut bc = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    let mut ec = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };

    if bc > ec + 1 || ec > 255 {
        bad!();
    }

    if bc > 255 {
        bc = 1;
        ec = 0;
    }

    let nw = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    let nh = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    let nd = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    let ni = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    let nl = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    let nk = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    let ne = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    let np = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };

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

    let (mut qw, a, b, c, d) = match read_tfm_word(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    let _ = (a, b, c, d);
    fonts.set_font_check(f, qw);

    let mut z = match read_fifteen(tfm_file) {
        Some(v) => v,
        None => bad!(),
    };
    z = z * 256 + tfm_getc(tfm_file);
    z = (z * 16) + (tfm_getc(tfm_file) / 16);
    if z < 65_536 {
        bad!();
    }

    while lh > 2 {
        tfm_getc(tfm_file);
        tfm_getc(tfm_file);
        tfm_getc(tfm_file);
        tfm_getc(tfm_file);
        lh -= 1;
    }

    fonts.set_font_dsize_entry(f, z);
    if s != -1000 {
        if s >= 0 {
            z = s;
        } else {
            // Borrow-free `xn_over_d`; the wrapper applies the
            // `tex_remainder`/`arith_error` side effects after the borrow drops.
            let (q, rem, overflowed) = crate::scaled_math::xn_over_d_parts(z, -s, 1000);
            z = q;
            scale_side_effect = Some((rem, overflowed));
        }
    }

    fonts.set_font_size_entry(f, z);

    let mut k = fonts.fmem_ptr;
    while k < fonts.width_base_entry(f) {
        let (word, a, b, c, d) = match read_tfm_word(tfm_file) {
            Some(v) => v,
            None => bad!(),
        };
        qw = word;
        fonts.set_font_info_b16(k, qw);

        if a >= nw || b / 16 >= nh || b % 16 >= nd || c / 4 >= ni {
            bad!();
        }

        match c % 4 {
            1 => {
                if d >= nl {
                    bad!();
                }
            }
            3 => {
                if d >= ne {
                    bad!();
                }
            }
            2 => {
                if d < bc || d > ec {
                    bad!();
                }

                let mut dd = d;
                while dd < k + bc - fonts.fmem_ptr {
                    qw = fonts.font_info_b16(fonts.char_base_entry(f) + dd);
                    if qw.s1 % 4 != LIST_TAG_RFI {
                        break;
                    }
                    dd = qw.s0 as i32;
                }

                if dd == k + bc - fonts.fmem_ptr {
                    bad!();
                }
            }
            _ => {}
        }
        k += 1;
    }

    let mut alpha = 16;
    while z >= 0x800000 {
        z /= 2;
        alpha += alpha;
    }
    let beta = 256 / alpha;
    alpha *= z;

    k = fonts.width_base_entry(f);
    while k < fonts.lig_kern_base_entry(f) {
        let sw = match read_scaled_word(tfm_file, z, beta, alpha) {
            Some(v) => v,
            None => bad!(),
        };
        fonts.set_font_info_b32s1(k, sw);
        k += 1;
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
        k = fonts.lig_kern_base_entry(f);
        while k < fonts.kern_base_entry(f) + 256 * 128 {
            let (word, a, b, c, d) = match read_tfm_word(tfm_file) {
                Some(v) => v,
                None => bad!(),
            };
            qw = word;
            fonts.set_font_info_b16(k, qw);

            if a > 128 {
                if 256 * c + d >= nl {
                    bad!();
                }

                if a == 255 && k == fonts.lig_kern_base_entry(f) {
                    bchar = b;
                }
            } else {
                if b != bchar {
                    if b < bc || b > ec {
                        bad!();
                    }

                    qw = fonts.font_info_b16(fonts.char_base_entry(f) + b);
                    if qw.s3 == 0 {
                        bad!();
                    }
                }

                if c < 128 {
                    if d < bc || d > ec {
                        bad!();
                    }
                    qw = fonts.font_info_b16(fonts.char_base_entry(f) + d);
                    if qw.s3 == 0 {
                        bad!();
                    }
                } else if 256 * (c - 128) + d >= nk {
                    bad!();
                }

                if a < 128 && k - fonts.lig_kern_base_entry(f) + a + 1 >= nl {
                    bad!();
                }
            }

            if k == fonts.kern_base_entry(f) + 256 * 128 - 1 && a == 255 {
                bch_label = 256 * c + d;
            }
            k += 1;
        }
    }

    k = fonts.kern_base_entry(f) + 256 * 128;
    while k < fonts.exten_base_entry(f) {
        let sw = match read_scaled_word(tfm_file, z, beta, alpha) {
            Some(v) => v,
            None => bad!(),
        };
        fonts.set_font_info_b32s1(k, sw);
        k += 1;
    }

    k = fonts.exten_base_entry(f);
    while k < fonts.param_base(f) {
        let (word, a, b, c, d) = match read_tfm_word(tfm_file) {
            Some(v) => v,
            None => bad!(),
        };
        fonts.set_font_info_b16(k, word);

        for ch in [a, b, c] {
            if ch != 0 {
                if ch < bc || ch > ec {
                    bad!();
                }
                qw = fonts.font_info_b16(fonts.char_base_entry(f) + ch);
                if qw.s3 == 0 {
                    bad!();
                }
            }
        }

        if d < bc || d > ec {
            bad!();
        }
        qw = fonts.font_info_b16(fonts.char_base_entry(f) + d);
        if qw.s3 == 0 {
            bad!();
        }
        k += 1;
    }

    k = 1;
    while k <= np {
        if k == 1 {
            let mut sw = tfm_getc(tfm_file);
            if sw == EOF_RFI {
                bad!();
            }
            if sw > 127 {
                sw -= 256;
            }

            sw = sw * 256 + tfm_getc(tfm_file);
            sw = sw * 256 + tfm_getc(tfm_file);
            fonts.set_font_info_b32s1(fonts.param_base(f), (sw * 16) + (tfm_getc(tfm_file) / 16));
        } else {
            let sw = match read_scaled_word(tfm_file, z, beta, alpha) {
                Some(v) => v,
                None => bad!(),
            };
            fonts.set_font_info_b32s1(fonts.param_base(f) + k - 1, sw);
        }
        k += 1;
    }

    k = np + 1;
    while k <= 7 {
        fonts.set_font_info_b32s1(fonts.param_base(f) + k - 1, 0);
        k += 1;
    }

    if np >= 7 {
        fonts.set_font_params(f, np);
    } else {
        fonts.set_font_params(f, 7);
    }

    fonts.set_hyphen_char_entry(f, eqtb.b32s1(INT_BASE + INT_PAR_DEFAULT_HYPHEN_CHAR_LNF));
    fonts.set_skew_char_entry(f, eqtb.b32s1(INT_BASE + INT_PAR_DEFAULT_SKEW_CHAR_LNF));
    if bch_label < nl {
        fonts.set_bchar_label_entry(f, bch_label + fonts.lig_kern_base_entry(f));
    } else {
        fonts.set_bchar_label_entry(f, NON_ADDRESS_RFI);
    }
    fonts.set_font_bchar_entry(f, bchar);
    fonts.set_font_false_bchar_entry(f, bchar);

    if bchar <= ec && bchar >= bc {
        qw = fonts.font_info_b16(fonts.char_base_entry(f) + bchar);
        if qw.s3 > 0 {
            fonts.set_font_false_bchar_entry(f, TOO_BIG_CHAR);
        }
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

/// Phase-C threaded core of [`load_native_font`]: initialize the per-font
/// metric tables for the newly-allocated native font slot under one
/// `&mut FontState` borrow (+ disjoint `&EqtbState` for the two default
/// hyphen/skew char int-params). Calls only the extern-C font-metric
/// extractors (`aat_get_font_metrics`/`ot_get_font_metrics`,
/// `get_loaded_font_design_size`), which hold no thread-local borrow. Returns
/// `(font_ptr, x_ht, cap_ht, font_slant)`; the caller uses the latter three for
/// the `font_info` parameter pushes after the borrow drops.
///
/// translated from the font-table-init body of xetex-xetex0.c:6284 (load_native_font)
unsafe fn init_native_font_tables(
    fonts: &mut FontState,
    eqtb: &crate::state::EqtbState,
    font_engine: *mut libc::c_void,
    full_name: i32,
    actual_size: i32,
    num_font_dimens: i32,
) -> (i32, i32, i32, i32) {
    fonts.font_ptr += 1;
    let font_ptr = fonts.font_ptr;
    fonts.set_font_area_entry(font_ptr, fonts.native_font_type_flag);
    fonts.set_font_name_entry(font_ptr, full_name);
    fonts.set_font_check(
        font_ptr,
        crate::mem_types::B16x4 {
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
        },
    );
    fonts.set_font_glue(font_ptr, crate::mem::TEX_NULL);
    fonts.set_font_dsize_entry(font_ptr, get_loaded_font_design_size());
    fonts.set_font_size_entry(font_ptr, actual_size);

    let mut ascent = 0;
    let mut descent = 0;
    let mut x_ht = 0;
    let mut cap_ht = 0;
    let mut font_slant = 0;
    if fonts.native_font_type_flag == AAT_FONT_FLAG {
        aat_get_font_metrics(
            font_engine,
            &mut ascent,
            &mut descent,
            &mut x_ht,
            &mut cap_ht,
            &mut font_slant,
        );
    } else {
        ot_get_font_metrics(
            font_engine,
            &mut ascent,
            &mut descent,
            &mut x_ht,
            &mut cap_ht,
            &mut font_slant,
        );
    }

    fonts.set_height_base_entry(font_ptr, ascent);
    fonts.set_depth_base_entry(font_ptr, -descent);
    fonts.set_font_params(font_ptr, num_font_dimens);
    fonts.set_font_bc_entry(font_ptr, 0);
    fonts.set_font_ec_entry(font_ptr, 65_535);
    fonts.set_font_used_entry(font_ptr, false);
    fonts.set_hyphen_char_entry(
        font_ptr,
        eqtb.b32s1(INT_BASE + INT_PAR_DEFAULT_HYPHEN_CHAR_LNF),
    );
    fonts.set_skew_char_entry(
        font_ptr,
        eqtb.b32s1(INT_BASE + INT_PAR_DEFAULT_SKEW_CHAR_LNF),
    );
    fonts.set_param_base(font_ptr, fonts.fmem_ptr - 1);
    fonts.set_font_layout_engine_ptr(font_ptr, font_engine);
    fonts.set_font_mapping_ptr(font_ptr, std::ptr::null_mut());
    fonts.set_font_letter_space_entry(font_ptr, fonts.loaded_font_letter_space);

    (font_ptr, x_ht, cap_ht, font_slant)
}

pub(crate) enum LoadNativeFontAbort {
    PoolOverflow,
    StringOverflow(NodeRecycleAbort),
    NativeChar(NativeCharAbort),
    Error(ErrorOutcome),
}

impl LoadNativeFontAbort {
    /// Map to the unified [`EngineAbort`] terminal (`01d` §5.5), mirroring the
    /// former `finish()` cold-abort tail without longjmp-ing.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            LoadNativeFontAbort::PoolOverflow => {
                let (pool_size, init_pool_ptr) =
                    crate::state::with_state(|st| (st.strings.pool_size, st.strings.init_pool_ptr));
                crate::errors::EngineAbort::Overflow {
                    what: c"pool size".as_ptr(),
                    n: pool_size - init_pool_ptr,
                }
            }
            LoadNativeFontAbort::StringOverflow(e) => e.into_engine_abort(),
            LoadNativeFontAbort::NativeChar(e) => e.into_engine_abort(),
            LoadNativeFontAbort::Error(outcome) => outcome.into_engine_abort(),
        }
    }
}

fn push_font_info_b32s1_in(st: &mut EngineState, v: i32) {
    let k = st.fonts.fmem_ptr;
    st.fonts.set_font_info_b32s1(k, v);
    st.fonts.fmem_ptr = k + 1;
}

/// Load a native AAT/OpenType/Graphite font and initialize TeX font tables.
///
/// translated from xetex-xetex0.c:6284 (load_native_font)
pub(crate) unsafe fn load_native_font_in(
    st: &mut EngineState,
    u: i32,
    nom: i32,
    aire: i32,
    mut s: i32,
) -> Result<i32, LoadNativeFontAbort> {
    let name_of_file = if st.input.name_of_file.is_empty() {
        std::ptr::null_mut()
    } else {
        st.input.name_of_file.as_mut_ptr()
    };
    let font_engine = find_native_font_in(st, name_of_file, s);
    if font_engine.is_null() {
        return Ok(FONT_BASE);
    }

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

    let mut f = FONT_BASE + 1;
    while f <= st.fonts.font_ptr {
        if st.fonts.font_area[f as usize] == st.fonts.native_font_type_flag
            && crate::string_pool::str_eq_str(
                &st.strings,
                st.fonts.font_name[f as usize],
                full_name,
            )
            && st.fonts.font_size[f as usize] == actual_size
        {
            release_font_engine(font_engine, st.fonts.native_font_type_flag);
            st.strings.str_ptr -= 1;
            st.strings.pool_ptr =
                st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
            return Ok(f);
        }
        f += 1;
    }

    let num_font_dimens =
        if st.fonts.native_font_type_flag == OTGR_FONT_FLAG && isOpenTypeMathFont(font_engine) {
            65
        } else {
            8
        };

    if st.fonts.font_ptr == st.fonts.font_max
        || st.fonts.fmem_ptr + num_font_dimens > st.fonts.font_mem_size
    {
        crate::output::error_here_with_diagnostic_in(st, c"Font ".as_ptr());
        crate::output::sprint_cs_in(st, u);
        crate::output::print_char_in(st, b'=' as i32);
        if st.scanner.file_name_quote_char != 0 {
            crate::output::print_char_in(st, st.scanner.file_name_quote_char as i32);
        }
        crate::output::print_file_name_in(st, nom, aire, st.scanner.cur_ext);
        if st.scanner.file_name_quote_char != 0 {
            crate::output::print_char_in(st, st.scanner.file_name_quote_char as i32);
        }
        if s >= 0 {
            crate::output::print_cstr_in(st, c" at ".as_ptr());
            crate::output::print_scaled_in(st, s);
            crate::output::print_cstr_in(st, c"pt".as_ptr());
        } else if s != -1000 {
            crate::output::print_cstr_in(st, c" scaled ".as_ptr());
            crate::output::print_int_in(st, -s);
        }
        crate::output::print_cstr_in(st, c" not loaded: Not enough room left".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());

        st.errors.help_ptr = 4;
        st.errors.help_line[3] = c"I'm afraid I won't be able to make use of this font,".as_ptr();
        st.errors.help_line[2] =
            c"because my memory for character-size data is too small.".as_ptr();
        st.errors.help_line[1] = c"If you're really stuck, ask a wizard to enlarge me.".as_ptr();
        st.errors.help_line[0] =
            c"Or maybe try `I\\font<same font id>=<name of loaded font>'.".as_ptr();

        let outcome = crate::errors::error_in(st);
        if !matches!(outcome, ErrorOutcome::Continue) {
            return Err(LoadNativeFontAbort::Error(outcome));
        }
        return Ok(FONT_BASE);
    }

    let (font_ptr, x_ht, cap_ht, font_slant) = init_native_font_tables(
        &mut st.fonts,
        &st.eqtb,
        font_engine,
        full_name,
        actual_size,
        num_font_dimens,
    );

    let p = new_native_character_in(st, font_ptr, b' ' as u32)
        .map_err(LoadNativeFontAbort::NativeChar)?;
    s = st.mem.b32s1(p + 1) + st.fonts.loaded_font_letter_space;
    let glyph_info = st.mem.ptr(p + 5);
    if !glyph_info.is_null() {
        libc::free(glyph_info);
        st.mem.set_ptr(p + 5, std::ptr::null_mut());
    }
    let node_size = st.mem.b16s3(p + 4) as i32;
    crate::mem::free_node(&mut st.mem, p, node_size);

    push_font_info_b32s1_in(st, font_slant);
    push_font_info_b32s1_in(st, s);
    push_font_info_b32s1_in(st, s / 2);
    push_font_info_b32s1_in(st, s / 3);
    push_font_info_b32s1_in(st, x_ht);
    push_font_info_b32s1_in(st, st.fonts.font_size[font_ptr as usize]);
    push_font_info_b32s1_in(st, s / 3);
    push_font_info_b32s1_in(st, cap_ht);

    if num_font_dimens == 65 {
        push_font_info_b32s1_in(st, num_font_dimens);
        for k in 0..=55 {
            push_font_info_b32s1_in(st, get_ot_math_constant(font_ptr, k));
        }
    }

    st.fonts
        .set_font_mapping_ptr(font_ptr, st.fonts.loaded_font_mapping);
    st.fonts.font_flags[font_ptr as usize] = st.fonts.loaded_font_flags as i8;
    Ok(font_ptr)
}

// ---- consts + NativeCharSrc moved/duplicated from scan.rs (workstream 02) ----

/// Source of the UTF-16 text payload for [`build_native_character`].
enum NativeCharSrc {
    /// A TECkit-mapped run: `len` UTF-16 units already sitting in
    /// `FontState::mapped_text`.
    Mapped(i32),
    /// A single Unicode scalar, encoded inline (1 or 2 UTF-16 units).
    Single(u32),
}

// Single-function-private consts relocated with the font-loading group.
const EOF_RFI: i32 = -1;
const INT_PAR_DEFAULT_HYPHEN_CHAR_LNF: i32 = 46;
const INT_PAR_DEFAULT_SKEW_CHAR_LNF: i32 = 47;
const INT_PAR_SUPPRESS_FONTNOTFOUND_ERROR_RFI: i32 = 65;
const INT_PAR_TRACING_LOST_CHARS_NC: i32 = 35;
const INT_PAR_XETEX_GENERATE_ACTUAL_TEXT: i32 = 79;
const INT_PAR_XETEX_USE_GLYPH_METRICS_NC: i32 = 72;
const INVALID_HANDLE: usize = 0;
const LIST_TAG_RFI: u16 = 2;
const NATIVE_WORD_NODE_AT_NN: u16 = 41;
const NATIVE_WORD_NODE_NN: u16 = 40;
const NON_ADDRESS_RFI: i32 = 0;
const TTBC_FILE_FORMAT_TFM: libc::c_int = 3;
const WHATSIT_NODE_NN: u16 = 8;
const EMPTY_STRING: i32 = TOO_BIG_CHAR + 1;

// Shared consts duplicated from scan.rs (still used there too).
const INT_PAR_XETEX_TRACING_FONTS: i32 = 77; // xetex_format.h:112
const AAT_FONT_FLAG: i32 = 0xFFFF;
const NATIVE_NODE_SIZE_NN: i32 = 6;
const OTGR_FONT_FLAG: i32 = 0xFFFE;
const FONT_BASE: i32 = 0;
