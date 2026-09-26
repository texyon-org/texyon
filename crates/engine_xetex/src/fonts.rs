//! Font loading: the TFM reader, native (OpenType/Graphite) font loading, native
//! character nodes and the TFM `char_info` accessors.

use crate::consts::{
    EMPTY_STRING, FONT_BASE, INT_PAR_DEFAULT_HYPHEN_CHAR, INT_PAR_DEFAULT_SKEW_CHAR,
    INT_PAR_SUPPRESS_FONTNOTFOUND_ERROR, INT_PAR_TRACING_LOST_CHARS,
    INT_PAR_XETEX_GENERATE_ACTUAL_TEXT, INT_PAR_XETEX_TRACING_FONTS,
    INT_PAR_XETEX_USE_GLYPH_METRICS, INVALID_HANDLE, NATIVE_NODE_SIZE, NON_ADDRESS, TOO_BIG_CHAR,
};
use crate::ext::{
    aat_get_font_metrics, apply_tfm_font_mapping, find_native_font, load_tfm_font_mapping,
    ot_get_font_metrics, NativeFontMetrics,
};
use crate::io::{pack_file_name, tt_xetex_open_input};
use crate::mem_types::B16x4;
use crate::node::{NodePtr, WhatsitNode, WhatsitSubtype};
use crate::output::{
    begin_diagnostic, capture_to_diagnostic, char_warning, diagnostic_begin_capture_warning_here,
    end_diagnostic, error_here_with_diagnostic, print, print_char, print_file_name, print_int,
    print_nl, print_scaled, sprint_cs,
};
use crate::state::{EngineState, EqtbState, FontState, MemState};

use crate::bridge::{input_getc, TTBC_FILE_FORMAT_TFM};
use crate::errors::Abort;

extern "C" {
    fn get_loaded_font_design_size() -> i32;
    fn isOpenTypeMathFont(engine: *mut libc::c_void) -> bool;
}

const EOF: i32 = -1;
pub(crate) const AAT_FONT_FLAG: i32 = 0xFFFF;
pub(crate) const OTGR_FONT_FLAG: i32 = 0xFFFE;
pub(crate) const LIG_TAG: u16 = 1;
pub(crate) const LIST_TAG: u16 = 2;
pub(crate) const EXT_TAG: u16 = 3;
pub(crate) const SLANT_CODE: i32 = 1;
pub(crate) const SPACE_CODE: i32 = 2;
pub(crate) const SPACE_SHRINK_CODE: i32 = 4;
pub(crate) const X_HEIGHT_CODE: i32 = 5;
pub(crate) const QUAD_CODE: i32 = 6;
pub(crate) const EXTRA_SPACE_CODE: i32 = 7;

pub(crate) type CharInfo = B16x4;

impl FontState {
    pub(crate) fn is_native_font(&self, f: i32) -> bool {
        let area = self.font_area_entry(f);
        area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG
    }

    pub(crate) fn char_info(&self, f: i32, c: i32) -> CharInfo {
        self.font_info_b16(self.char_base_entry(f) + c)
    }

    pub(crate) fn char_width(&self, f: i32, i: CharInfo) -> i32 {
        self.font_info_sc(self.width_base_entry(f) + i.s3 as i32)
    }

    pub(crate) fn char_italic(&self, f: i32, i: CharInfo) -> i32 {
        self.font_info_sc(self.italic_base_entry(f) + i.s1 as i32 / 4)
    }

    pub(crate) fn char_height(&self, f: i32, i: CharInfo) -> i32 {
        self.font_info_sc(self.height_base_entry(f) + i.s2 as i32 / 16)
    }

    pub(crate) fn char_depth(&self, f: i32, i: CharInfo) -> i32 {
        self.font_info_sc(self.depth_base_entry(f) + i.s2 as i32 % 16)
    }

    /// The kern amount of lig/kern instruction `i` in font `f`.
    pub(crate) fn char_kern(&self, f: i32, i: CharInfo) -> i32 {
        self.font_info_sc(self.kern_base_entry(f) + 256 * i.s1 as i32 + i.s0 as i32)
    }

    pub fn effective_char(&mut self, f: i32, c: u16) -> i32 {
        let mut c = c;
        let mapping = self.font_mapping_ptr(f);
        if !self.xtx_ligature_present && !mapping.is_null() {
            // SAFETY: `mapping` is the font's live TECkit converter.
            c = unsafe { apply_tfm_font_mapping(mapping, c as i32) } as u16;
        }
        self.xtx_ligature_present = false;
        c as i32
    }

    pub fn effective_char_info(&mut self, f: i32, c: u16) -> CharInfo {
        let c = self.effective_char(f, c);
        self.char_info(f, c)
    }
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

/// Where a native character's UTF-16 text comes from.
enum NativeCharSrc {
    /// `len` units already in `FontState::mapped_text`.
    Mapped(i32),
    Single(u32),
}

impl MemState {
    fn new_native_word(
        &mut self,
        f: i32,
        n: i32,
        subtype: WhatsitSubtype,
    ) -> Result<NodePtr<WhatsitNode>, Abort> {
        let size = NATIVE_NODE_SIZE + (n * 2 + 7) / 8;
        let w = NodePtr::<WhatsitNode>::alloc(self, size, subtype)?;
        w.set_native_size(self, size);
        w.set_native_font(self, crate::units::FontId(f));
        w.set_native_length(self, n as u16);
        w.set_native_glyph_count(self, 0);
        w.set_native_glyph_info_handle(self, 0);
        Ok(w)
    }

    /// A native-word node for `n` UTF-16 units of font `f`, text left unset.
    pub(crate) fn new_native_word_node(
        &mut self,
        eqtb: &EqtbState,
        f: i32,
        n: i32,
    ) -> Result<NodePtr<WhatsitNode>, Abort> {
        let subtype = if eqtb.int_par(INT_PAR_XETEX_GENERATE_ACTUAL_TEXT) > 0 {
            WhatsitSubtype::NativeWordAt
        } else {
            WhatsitSubtype::NativeWord
        };
        self.new_native_word(f, n, subtype)
    }

    fn build_native_character(
        &mut self,
        eqtb: &EqtbState,
        fonts: &FontState,
        f: i32,
        src: NativeCharSrc,
    ) -> Result<NodePtr<WhatsitNode>, Abort> {
        match src {
            NativeCharSrc::Mapped(len) => {
                let w = self.new_native_word_node(eqtb, f, len)?;
                let src = &fonts.mapped_text[..len as usize];
                let text = w.native_text_ptr_mut(self);
                // SAFETY: the node holds `len` units and the arena never reallocates.
                unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), text, src.len()) };
                Ok(w)
            }
            NativeCharSrc::Single(c) => {
                let (units, n) = utf16_units(c);
                let w = self.new_native_word(f, n as i32, WhatsitSubtype::NativeWord)?;
                let text = w.native_text_ptr_mut(self);
                // SAFETY: the node holds `n` units and the arena never reallocates.
                unsafe { std::ptr::copy_nonoverlapping(units.as_ptr(), text, n) };
                Ok(w)
            }
        }
    }
}

/// Warn about `c` when font `f` has no glyph for it.
fn check_native_char(st: &mut EngineState, f: i32, c: i32) -> Result<(), Abort> {
    // SAFETY: font-table lookup over FFI, no state re-entry.
    if unsafe { st.fonts.map_char_to_glyph(f, c) }? == 0 {
        char_warning(st, f, c)?;
    }
    Ok(())
}

/// A measured native-word node for character `c` of font `f`, run through the
/// font's TECkit mapping if it has one.
pub(crate) fn new_native_character(
    st: &mut EngineState,
    f: i32,
    c: u32,
) -> Result<NodePtr<WhatsitNode>, Abort> {
    let src = if st.fonts.font_mapping[f as usize].is_some() {
        let (units, n) = utf16_units(c);
        if st.strings.pool_ptr + n as i32 > st.strings.pool_size {
            return Err(Abort::PoolOverflow);
        }
        for unit in &units[..n] {
            st.strings.str_pool[st.strings.pool_ptr as usize] = *unit;
            st.strings.pool_ptr += 1;
        }

        let start = st.strings.cur_string_start();
        let map = st.fonts.font_mapping_ptr(f);
        let cur_len = st.strings.cur_length();
        // SAFETY: `dst` points into the non-reallocating string pool, disjoint
        // from the `&mut st.fonts` borrow.
        let len = unsafe {
            let dst = st.strings.str_pool.as_mut_ptr().add(start as usize);
            st.fonts.apply_mapping(map, dst, cur_len)
        };
        st.strings.pool_ptr = start;

        let mut i = 0;
        while i < len {
            let mapped = st.fonts.mapped_text[i as usize];
            if (0xD800..0xDC00).contains(&mapped) {
                let low = st.fonts.mapped_text[(i + 1) as usize] as u32;
                let c = (mapped as u32 - 0xD800) * 1024 + low + 9216;
                check_native_char(st, f, c as i32)?;
                i += 2;
            } else {
                check_native_char(st, f, mapped as i32)?;
                i += 1;
            }
        }
        NativeCharSrc::Mapped(len)
    } else {
        if st.eqtb.int_par(INT_PAR_TRACING_LOST_CHARS) > 0 {
            check_native_char(st, f, c as i32)?;
        }
        NativeCharSrc::Single(c)
    };

    let w = st.mem.build_native_character(&st.eqtb, &st.fonts, f, src)?;
    let use_glyph_metrics = st.eqtb.int_par(INT_PAR_XETEX_USE_GLYPH_METRICS) > 0;
    // SAFETY: the measurer only reads font tables over re-entry-free FFI.
    unsafe {
        st.fonts
            .measure_native_node(&mut st.mem, w, use_glyph_metrics)?
    };
    Ok(w)
}

unsafe fn tfm_getc(handle: usize) -> i32 {
    input_getc(handle) as i32
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

pub(crate) fn name_of_file_ptr(st: &mut EngineState) -> *mut libc::c_char {
    if st.input.name_of_file.is_empty() {
        std::ptr::null_mut()
    } else {
        st.input.name_of_file.as_mut_ptr().cast()
    }
}

/// Start a "Font \cs=name at size" error.
fn font_error_start(st: &mut EngineState, u: i32, nom: i32, aire: i32, s: i32) {
    error_here_with_diagnostic(st, "Font ");
    sprint_cs(st, u);
    print_char(st, b'=' as i32);
    let quote = st.scanner.file_name_quote_char;
    if quote != 0 {
        print_char(st, quote as i32);
    }
    print_file_name(st, nom, aire, st.scanner.cur_ext);
    if quote != 0 {
        print_char(st, quote as i32);
    }
    if s >= 0 {
        print(st, " at ");
        print_scaled(st, s);
        print(st, "pt");
    } else if s != -1000 {
        print(st, " scaled ");
        print_int(st, -s);
    }
}

pub(crate) unsafe fn read_font_info(
    st: &mut EngineState,
    u: i32,
    nom: i32,
    aire: i32,
    s: i32,
) -> Result<i32, Abort> {
    let mut name_too_long = false;
    let mut file_opened = false;
    let mut g = FONT_BASE;
    let mut tfm_file = INVALID_HANDLE;

    pack_file_name(st, nom, aire, st.scanner.cur_ext);

    if st.eqtb.int_par(INT_PAR_XETEX_TRACING_FONTS) > 0 {
        begin_diagnostic(st);
        diagnostic_begin_capture_warning_here(st);
        print_nl(st, "Requested font \"");
        let name = st.input.name_of_file_bytes().to_vec();
        print(st, name.as_slice());
        print_char(st, b'"' as i32);
        if s < 0 {
            print(st, " scaled ");
            print_int(st, -s);
        } else {
            print(st, " at ");
            print_scaled(st, s);
            print(st, "pt");
        }
        capture_to_diagnostic(st, None);
        end_diagnostic(st, false);
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
            g = load_native_font(st, u, nom, aire, s)?;
            if g != FONT_BASE {
                break 'read;
            }
        }

        name_too_long = st.strings.length(nom) > 255 || st.strings.length(aire) > 255;
        if name_too_long {
            bad!();
        }

        pack_file_name(st, nom, aire, EMPTY_STRING);
        st.fonts
            .check_for_tfm_font_mapping(&mut st.input.name_of_file);

        tfm_file = tt_xetex_open_input(st, TTBC_FILE_FORMAT_TFM);
        if tfm_file == INVALID_HANDLE {
            if !st.scanner.quoted_filename {
                g = load_native_font(st, u, nom, aire, s)?;
                if g != FONT_BASE {
                    break 'read;
                }
            }
            bad!();
        }
        file_opened = true;

        let (parse, scale_side_effect) = st
            .fonts
            .read_font_info_tfm(&st.eqtb, tfm_file, nom, aire, s);
        if let Some((remainder, overflowed)) = scale_side_effect {
            st.errors.tex_remainder = remainder;
            if overflowed {
                st.errors.arith_error = true;
            }
        }
        match parse {
            TfmParse::OutOfMemory => {
                return Err(Abort::Internal("not enough memory to load another font"))
            }
            TfmParse::Bad => bad!(),
            TfmParse::Loaded(f) => {
                g = f;
                let mapping = load_tfm_font_mapping(st)?;
                st.fonts.set_font_mapping(f, mapping);
            }
        }
    }

    if bad_tfm && st.eqtb.int_par(INT_PAR_SUPPRESS_FONTNOTFOUND_ERROR) == 0 {
        font_error_start(st, u, nom, aire, s);
        let reason = if file_opened {
            " not loadable: Bad metric (TFM) file"
        } else if name_too_long {
            " not loadable: Metric (TFM) file name too long"
        } else {
            " not loadable: Metric (TFM) file or installed font not found"
        };
        print(st, reason);
        capture_to_diagnostic(st, None);

        st.errors.help(&[
            "I wasn't able to read the size data for this font,",
            "so I will ignore the font specification.",
            "[Wizards can fix TFM files using TFtoPL/PLtoTF.]",
            "You might try inserting a different font spec;",
            r"e.g., type `I\font<same font id>=<substitute font name>'.",
        ]);

        crate::errors::error(st)?;
    }

    if file_opened && crate::io::input_close(tfm_file) {
        return Err(Abort::Internal(
            "serious internal error closing the font metric file",
        ));
    }

    if st.eqtb.int_par(INT_PAR_XETEX_TRACING_FONTS) > 0 && (g == FONT_BASE || file_opened) {
        begin_diagnostic(st);
        diagnostic_begin_capture_warning_here(st);
        print_nl(st, " -> ");
        if g == FONT_BASE {
            print(st, "font not found, using \"nullfont\"");
        } else {
            let name = st.input.name_of_file_bytes().to_vec();
            print(st, name.as_slice());
        }
        capture_to_diagnostic(st, None);
        end_diagnostic(st, false);
    }

    Ok(g)
}

enum TfmParse {
    Loaded(i32),
    Bad,
    OutOfMemory,
}

impl FontState {
    /// Read the opened TFM file into the tables of the next font slot. Also returns
    /// the (remainder, overflow) side effect of size scaling for the caller to apply.
    unsafe fn read_font_info_tfm(
        &mut self,
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

        if self.font_ptr == self.font_max || self.fmem_ptr + lf > self.font_mem_size {
            return (TfmParse::OutOfMemory, None);
        }

        let f = self.font_ptr + 1;
        self.set_char_base_entry(f, self.fmem_ptr - bc);
        self.set_width_base_entry(f, self.char_base_entry(f) + ec + 1);
        self.set_height_base_entry(f, self.width_base_entry(f) + nw);
        self.set_depth_base_entry(f, self.height_base_entry(f) + nh);
        self.set_italic_base_entry(f, self.depth_base_entry(f) + nd);
        self.set_lig_kern_base_entry(f, self.italic_base_entry(f) + ni);
        self.set_kern_base_entry(f, self.lig_kern_base_entry(f) + nl - 256 * 128);
        self.set_exten_base_entry(f, self.kern_base_entry(f) + 256 * 128 + nk);
        self.set_param_base(f, self.exten_base_entry(f) + ne);

        if lh < 2 {
            bad!();
        }

        self.set_font_check(f, four_quarters(read!(read_tfm_bytes(tfm_file))));

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

        self.set_font_dsize_entry(f, z);
        if s != -1000 {
            if s >= 0 {
                z = s;
            } else {
                let (q, rem, overflowed) = crate::scaled_math::xn_over_d_parts(z, -s, 1000);
                z = q;
                scale_side_effect = Some((rem, overflowed));
            }
        }
        self.set_font_size_entry(f, z);

        let char_exists =
            |fonts: &FontState, ch: i32| (bc..=ec).contains(&ch) && fonts.char_info(f, ch).s3 != 0;

        for k in self.fmem_ptr..self.width_base_entry(f) {
            let bytes = read!(read_tfm_bytes(tfm_file));
            let [a, b, c, d] = bytes;
            self.set_font_info_b16(k, four_quarters(bytes));

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
                    let current = k + bc - self.fmem_ptr;
                    let mut dd = d;
                    while dd < current {
                        let qw = self.char_info(f, dd);
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

        for k in self.width_base_entry(f)..self.lig_kern_base_entry(f) {
            let sw = read!(read_scaled_word(tfm_file, z, beta, alpha));
            self.set_font_info_sc(k, sw);
        }

        if self.font_info_sc(self.width_base_entry(f)) != 0
            || self.font_info_sc(self.height_base_entry(f)) != 0
            || self.font_info_sc(self.depth_base_entry(f)) != 0
            || self.font_info_sc(self.italic_base_entry(f)) != 0
        {
            bad!();
        }

        let mut bch_label = 32_767;
        let mut bchar = 256;

        if nl > 0 {
            let lig_kern_base = self.lig_kern_base_entry(f);
            let lig_kern_end = self.kern_base_entry(f) + 256 * 128;
            for k in lig_kern_base..lig_kern_end {
                let bytes = read!(read_tfm_bytes(tfm_file));
                let [a, b, c, d] = bytes;
                self.set_font_info_b16(k, four_quarters(bytes));

                if a > 128 {
                    if 256 * c + d >= nl {
                        bad!();
                    }
                    if a == 255 && k == lig_kern_base {
                        bchar = b;
                    }
                } else {
                    if b != bchar && !char_exists(self, b) {
                        bad!();
                    }
                    if c < 128 {
                        if !char_exists(self, d) {
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

        for k in self.kern_base_entry(f) + 256 * 128..self.exten_base_entry(f) {
            let sw = read!(read_scaled_word(tfm_file, z, beta, alpha));
            self.set_font_info_sc(k, sw);
        }

        for k in self.exten_base_entry(f)..self.param_base(f) {
            let bytes = read!(read_tfm_bytes(tfm_file));
            let [a, b, c, d] = bytes;
            self.set_font_info_b16(k, four_quarters(bytes));

            for ch in [a, b, c] {
                if ch != 0 && !char_exists(self, ch) {
                    bad!();
                }
            }
            if !char_exists(self, d) {
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
            self.set_font_info_sc(self.param_base(f) + k - 1, sw);
        }
        for k in np + 1..=7 {
            self.set_font_info_sc(self.param_base(f) + k - 1, 0);
        }

        self.set_font_params(f, np.max(7));
        self.set_hyphen_char_entry(f, eqtb.int_par(INT_PAR_DEFAULT_HYPHEN_CHAR));
        self.set_skew_char_entry(f, eqtb.int_par(INT_PAR_DEFAULT_SKEW_CHAR));
        if bch_label < nl {
            self.set_bchar_label_entry(f, bch_label + self.lig_kern_base_entry(f));
        } else {
            self.set_bchar_label_entry(f, NON_ADDRESS);
        }
        self.set_font_bchar_entry(f, bchar);
        self.set_font_false_bchar_entry(f, bchar);
        if bchar <= ec && bchar >= bc && self.char_info(f, bchar).s3 > 0 {
            self.set_font_false_bchar_entry(f, TOO_BIG_CHAR);
        }

        self.set_font_name_entry(f, nom);
        self.set_font_area_entry(f, aire);
        self.set_font_bc_entry(f, bc as u16);
        self.set_font_ec_entry(f, ec as u16);
        self.set_font_glue(f, crate::mem::TEX_NULL);
        self.set_param_base(f, self.param_base(f) - 1);
        self.fmem_ptr += lf;
        self.font_ptr = f;
        (TfmParse::Loaded(f), scale_side_effect)
    }

    /// Set up the tables of a new native font slot. Returns the slot and the
    /// font's metrics.
    unsafe fn init_native_font_tables(
        &mut self,
        eqtb: &EqtbState,
        font_engine: crate::state::FontEngineHandle,
        full_name: i32,
        actual_size: i32,
        num_font_dimens: i32,
    ) -> (i32, NativeFontMetrics) {
        self.font_ptr += 1;
        let font_ptr = self.font_ptr;
        self.set_font_area_entry(font_ptr, self.native_font_type_flag);
        self.set_font_name_entry(font_ptr, full_name);
        self.set_font_check(font_ptr, B16x4::default());
        self.set_font_glue(font_ptr, crate::mem::TEX_NULL);
        self.set_font_dsize_entry(font_ptr, get_loaded_font_design_size());
        self.set_font_size_entry(font_ptr, actual_size);

        let metrics = if self.native_font_type_flag == AAT_FONT_FLAG {
            aat_get_font_metrics(font_engine.raw)
        } else {
            ot_get_font_metrics(font_engine.raw)
        };

        self.set_height_base_entry(font_ptr, metrics.ascent);
        self.set_depth_base_entry(font_ptr, -metrics.descent);
        self.set_font_params(font_ptr, num_font_dimens);
        self.set_font_bc_entry(font_ptr, 0);
        self.set_font_ec_entry(font_ptr, 65_535);
        self.set_font_used_entry(font_ptr, false);
        self.set_hyphen_char_entry(font_ptr, eqtb.int_par(INT_PAR_DEFAULT_HYPHEN_CHAR));
        self.set_skew_char_entry(font_ptr, eqtb.int_par(INT_PAR_DEFAULT_SKEW_CHAR));
        self.set_param_base(font_ptr, self.fmem_ptr - 1);
        self.set_font_layout_engine(font_ptr, Some(std::sync::Arc::new(font_engine)));
        self.set_font_mapping(font_ptr, None);
        self.set_font_letter_space_entry(font_ptr, self.loaded_font_letter_space);

        (font_ptr, metrics)
    }

    fn push_font_info(&mut self, v: i32) {
        self.set_font_info_sc(self.fmem_ptr, v);
        self.fmem_ptr += 1;
    }
}

/// Load a native OpenType/Graphite font and set up its TeX font tables.
pub(crate) unsafe fn load_native_font(
    st: &mut EngineState,
    u: i32,
    nom: i32,
    aire: i32,
    s: i32,
) -> Result<i32, Abort> {
    let name_of_file = name_of_file_ptr(st);
    let Some(font_engine) = find_native_font(st, name_of_file, s)? else {
        return Ok(FONT_BASE);
    };

    let actual_size = if s >= 0 {
        s
    } else if s != -1000 {
        st.errors.xn_over_d(get_loaded_font_design_size(), -s, 1000)
    } else {
        get_loaded_font_design_size()
    };

    if st.strings.pool_ptr + st.input.name_length > st.strings.pool_size {
        return Err(Abort::PoolOverflow);
    }
    for k in 0..st.input.name_length {
        st.strings.str_pool[st.strings.pool_ptr as usize] =
            st.input.name_of_file[k as usize] as u16;
        st.strings.pool_ptr += 1;
    }
    let full_name = st.strings.make_string()?;

    for f in FONT_BASE + 1..=st.fonts.font_ptr {
        if st.fonts.font_area[f as usize] == st.fonts.native_font_type_flag
            && st
                .strings
                .str_eq_str(st.fonts.font_name[f as usize], full_name)
            && st.fonts.font_size[f as usize] == actual_size
        {
            drop(font_engine);
            st.strings.str_ptr -= 1;
            st.strings.pool_ptr = st.strings.cur_string_start();
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
        font_error_start(st, u, nom, aire, s);
        print(st, " not loaded: Not enough room left");
        capture_to_diagnostic(st, None);

        st.errors.help(&[
            "I'm afraid I won't be able to make use of this font,",
            "because my memory for character-size data is too small.",
            "If you're really stuck, ask a wizard to enlarge me.",
            r"Or maybe try `I\font<same font id>=<name of loaded font>'.",
        ]);

        crate::errors::error(st)?;
        return Ok(FONT_BASE);
    }

    let (font_ptr, metrics) = st.fonts.init_native_font_tables(
        &st.eqtb,
        font_engine,
        full_name,
        actual_size,
        num_font_dimens,
    );

    let space = new_native_character(st, font_ptr, b' ' as u32)?;
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
        st.fonts.push_font_info(v);
    }

    if num_font_dimens == 65 {
        st.fonts.push_font_info(num_font_dimens);
        for k in 0..=55 {
            let v = st.fonts.get_ot_math_constant(font_ptr, k);
            st.fonts.push_font_info(v);
        }
    }

    let staged_mapping = st.fonts.loaded_font_mapping.take();
    st.fonts.set_font_mapping(font_ptr, staged_mapping);
    st.fonts.font_flags[font_ptr as usize] = st.fonts.loaded_font_flags as i8;
    Ok(font_ptr)
}
