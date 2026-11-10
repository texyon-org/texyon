//! Font loading: the TFM reader, native (OpenType/Graphite) font loading, native
//! character nodes and the TFM `char_info` accessors.

use crate::consts::{
    DESIGN_SIZE, EMPTY_STRING, INVALID_HANDLE, NATIVE_NODE_SIZE, NULL_FONT, TOO_BIG_CHAR,
};
use crate::ext::{
    aat_get_font_metrics, apply_tfm_font_mapping, find_native_font, load_mapping_file,
    ot_get_font_metrics, split_tfm_font_mapping, NativeFontMetrics,
};
use crate::io::{open_input_file, FileName, ScannedName};
use crate::mem_types::B16x4;
use crate::node::{NativeWordNode, NodePtr, WhatsitSubtype};
use crate::output::{
    begin_diagnostic, char_warning, end_diagnostic, print, print_char, print_int, print_nl,
    print_scaled, CsName, TexPrint,
};
use crate::params::IntPar;
use crate::state::{EngineState, EqtbState, FontState, MemState};
use crate::units::{EqtbPtr, FontId, FontInfoIdx, Scaled, StrNumber};

use crate::bridge::{input_getc, FileFormat};
use crate::errors::Abort;

use crate::ext::loaded_font_design_size;
use std::collections::BTreeMap;
use std::rc::Rc;

/// The edge of a line a character protrudes into.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub(crate) enum Side {
    Left,
    Right,
}

/// `\lpcode` and `\rpcode` values, keyed by font and character or glyph.
/// Clones share the table until one of them is written.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct ProtrusionCodes(Rc<BTreeMap<(FontId, Side, u32), i32>>);

impl ProtrusionCodes {
    /// The code of `c` in font `f`, 0 when unset.
    pub(crate) fn get(&self, f: FontId, side: Side, c: u32) -> i32 {
        self.0.get(&(f, side, c)).copied().unwrap_or(0)
    }

    pub(crate) fn set(&mut self, f: FontId, side: Side, c: u32, value: i32) {
        Rc::make_mut(&mut self.0).insert((f, side, c), value);
    }

    /// Every set code, in key order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&(FontId, Side, u32), &i32)> {
        self.0.iter()
    }
}

const EOF: i32 = -1;
/// The `font_area` of an AAT font, which no real area string can equal.
pub(crate) const AAT_FONT_FLAG: StrNumber = StrNumber::from_raw(0xFFFF);
/// The `font_area` of an OpenType or Graphite font.
pub(crate) const OTGR_FONT_FLAG: StrNumber = StrNumber::from_raw(0xFFFE);
pub(crate) const SLANT_CODE: i32 = 1;
pub(crate) const SPACE_CODE: i32 = 2;
pub(crate) const SPACE_STRETCH_CODE: i32 = 3;
pub(crate) const SPACE_SHRINK_CODE: i32 = 4;
pub(crate) const X_HEIGHT_CODE: i32 = 5;
pub(crate) const QUAD_CODE: i32 = 6;
pub(crate) const EXTRA_SPACE_CODE: i32 = 7;

/// What a character's `remainder` means.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum CharTag {
    None,
    /// The remainder starts a lig/kern program.
    Lig,
    /// The remainder is the next larger character.
    List,
    /// The remainder indexes an extensible recipe.
    Ext,
}

/// A TFM `char_info` word.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct CharInfo(B16x4);

impl CharInfo {
    /// TeX's `null_character`.
    pub(crate) const NULL: CharInfo = CharInfo(B16x4 {
        s0: 0,
        s1: 0,
        s2: 0,
        s3: 0,
    });

    #[inline]
    pub(crate) fn exists(self) -> bool {
        self.0.s3 > 0
    }

    #[inline]
    fn width_index(self) -> i32 {
        i32::from(self.0.s3)
    }

    #[inline]
    fn height_index(self) -> i32 {
        i32::from(self.0.s2) / 16
    }

    #[inline]
    fn depth_index(self) -> i32 {
        i32::from(self.0.s2) % 16
    }

    #[inline]
    fn italic_index(self) -> i32 {
        i32::from(self.0.s1) / 4
    }

    #[inline]
    pub(crate) fn tag(self) -> CharTag {
        match self.0.s1 % 4 {
            0 => CharTag::None,
            1 => CharTag::Lig,
            2 => CharTag::List,
            _ => CharTag::Ext,
        }
    }

    #[inline]
    pub(crate) fn remainder(self) -> u16 {
        self.0.s0
    }
}

/// One lig/kern instruction.
#[derive(Copy, Clone, Debug)]
pub(crate) struct LigKernStep(B16x4);

impl LigKernStep {
    const STOP_FLAG: u16 = 128;
    const KERN_FLAG: u16 = 128;

    /// This is the last instruction for the character.
    #[inline]
    pub(crate) fn is_last(self) -> bool {
        self.0.s3 >= Self::STOP_FLAG
    }

    /// A first instruction that only points to the real program.
    #[inline]
    pub(crate) fn is_redirect(self) -> bool {
        self.0.s3 > Self::STOP_FLAG
    }

    #[inline]
    pub(crate) fn next_char(self) -> u16 {
        self.0.s2
    }

    /// The instruction applies to next character `c`.
    #[inline]
    pub(crate) fn matches(self, c: LigChar) -> bool {
        i32::from(self.next_char()) == c.0 && !self.is_redirect()
    }

    #[inline]
    pub(crate) fn is_kern(self) -> bool {
        self.0.s1 >= Self::KERN_FLAG
    }

    /// The ligature operation (`=:`, `|=:|>`, ...) as TeX numbers it.
    #[inline]
    pub(crate) fn lig_op(self) -> u16 {
        self.0.s1
    }

    /// The character a ligature instruction inserts.
    #[inline]
    pub(crate) fn lig_char(self) -> u16 {
        self.0.s0
    }

    /// The index of the next instruction for the character, from this one's `k`.
    #[inline]
    pub(crate) fn next_index(self, k: FontInfoIdx) -> FontInfoIdx {
        k + i32::from(self.0.s3) + 1
    }
}

/// A character in a lig/kern program, or `NONE` (WEB's `non_char`) for a boundary.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub(crate) struct LigChar(i32);

impl LigChar {
    pub(crate) const NONE: LigChar = LigChar(TOO_BIG_CHAR);

    /// The value from a character code, a sentinel such as `max_hyph_char`, or a format file.
    #[inline]
    pub(crate) const fn from_raw(raw: i32) -> LigChar {
        LigChar(raw)
    }

    /// The raw form for a format file.
    #[inline]
    pub(crate) const fn raw(self) -> i32 {
        self.0
    }

    #[inline]
    pub(crate) fn of(c: u16) -> LigChar {
        LigChar(i32::from(c))
    }

    /// Below `NONE`, so a real character rather than a boundary.
    #[inline]
    pub(crate) fn is_char(self) -> bool {
        self.0 < TOO_BIG_CHAR
    }

    /// The code as a node stores it, cut to 16 bits as WEB's `qi` does.
    #[inline]
    pub(crate) fn code(self) -> u16 {
        self.0 as u16
    }
}

/// An extensible recipe: the character of each piece, 0 for none.
#[derive(Copy, Clone, Debug)]
pub(crate) struct ExtRecipe {
    pub top: u16,
    pub mid: u16,
    pub bot: u16,
    pub rep: u16,
}

impl FontState {
    pub(crate) fn is_native_font(&self, f: FontId) -> bool {
        let area = self.font_area[f];
        area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG
    }

    /// A native font's ascent, which it keeps in `height_base`.
    pub(crate) fn native_height(&self, f: FontId) -> Scaled {
        Scaled::from_raw(self.height_base[f].raw())
    }

    /// A native font's descent, which it keeps in `depth_base`.
    pub(crate) fn native_depth(&self, f: FontId) -> Scaled {
        Scaled::from_raw(self.depth_base[f].raw())
    }

    fn set_native_extent(&mut self, f: FontId, ascent: Scaled, descent: Scaled) {
        self.height_base[f] = FontInfoIdx::from_raw(ascent.raw());
        self.depth_base[f] = FontInfoIdx::from_raw(-descent.raw());
    }

    #[inline]
    pub(crate) fn char_info(&self, f: FontId, c: i32) -> CharInfo {
        CharInfo(self.font_info_b16(self.char_base[f] + c))
    }

    /// `c` is in the font's range and exists.
    #[inline]
    pub(crate) fn char_exists(&self, f: FontId, c: i32) -> bool {
        (i32::from(self.font_bc[f])..=i32::from(self.font_ec[f])).contains(&c)
            && self.char_info(f, c).exists()
    }

    #[inline]
    pub(crate) fn char_width(&self, f: FontId, i: CharInfo) -> Scaled {
        self.font_info_sc(self.width_base[f] + i.width_index())
    }

    #[inline]
    pub(crate) fn char_italic(&self, f: FontId, i: CharInfo) -> Scaled {
        self.font_info_sc(self.italic_base[f] + i.italic_index())
    }

    #[inline]
    pub(crate) fn char_height(&self, f: FontId, i: CharInfo) -> Scaled {
        self.font_info_sc(self.height_base[f] + i.height_index())
    }

    #[inline]
    pub(crate) fn char_depth(&self, f: FontId, i: CharInfo) -> Scaled {
        self.font_info_sc(self.depth_base[f] + i.depth_index())
    }

    /// The lig/kern instruction at `font_info` index `k`.
    #[inline]
    pub(crate) fn lig_kern_step(&self, k: FontInfoIdx) -> LigKernStep {
        LigKernStep(self.font_info_b16(k))
    }

    /// The index and first instruction of the lig/kern program of `i`, which
    /// is tagged `Lig`.
    #[inline]
    pub(crate) fn lig_kern_start(&self, f: FontId, i: CharInfo) -> (FontInfoIdx, LigKernStep) {
        let k = self.lig_kern_base[f] + i32::from(i.remainder());
        let step = self.lig_kern_step(k);
        if !step.is_redirect() {
            return (k, step);
        }
        let k = self.lig_kern_base[f] + 256 * i32::from(step.0.s1) + i32::from(step.0.s0);
        (k, self.lig_kern_step(k))
    }

    /// The amount of kern instruction `step`.
    #[inline]
    pub(crate) fn char_kern(&self, f: FontId, step: LigKernStep) -> Scaled {
        self.font_info_sc(self.kern_base[f] + 256 * i32::from(step.0.s1) + i32::from(step.0.s0))
    }

    /// The extensible recipe of `i`, which is tagged `Ext`.
    pub(crate) fn ext_recipe(&self, f: FontId, i: CharInfo) -> ExtRecipe {
        let w = self.font_info_b16(self.exten_base[f] + i32::from(i.remainder()));
        ExtRecipe {
            top: w.s3,
            mid: w.s2,
            bot: w.s1,
            rep: w.s0,
        }
    }

    /// A TFM font's check sum, in file byte order.
    pub(crate) fn check_bytes(&self, f: FontId) -> [u8; 4] {
        self.font_check[f]
    }

    pub fn effective_char(&mut self, f: FontId, c: u16) -> i32 {
        let mut c = c;
        if !self.xtx_ligature_present {
            if let Some(mapping) = self.font_mapping[f].as_deref() {
                c = apply_tfm_font_mapping(mapping, i32::from(c)) as u16;
            }
        }
        self.xtx_ligature_present = false;
        i32::from(c)
    }

    pub fn effective_char_info(&mut self, f: FontId, c: u16) -> CharInfo {
        let c = self.effective_char(f, c);
        self.char_info(f, c)
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
        f: FontId,
        n: i32,
        subtype: WhatsitSubtype,
    ) -> Result<NodePtr<NativeWordNode>, Abort> {
        let size = NATIVE_NODE_SIZE + (n * 2 + 7) / 8;
        let w = NodePtr::<NativeWordNode>::alloc_sized(self, size, subtype)?;
        let word = &mut self[w];
        word.size = size as u16;
        word.font = f;
        word.length = n as u16;
        word.clear_glyphs();
        Ok(w)
    }

    /// A native-word node for `n` UTF-16 units of font `f`, text left unset.
    pub(crate) fn new_native_word_node(
        &mut self,
        eqtb: &EqtbState,
        f: FontId,
        n: i32,
    ) -> Result<NodePtr<NativeWordNode>, Abort> {
        let subtype = if eqtb.int_par(IntPar::XetexGenerateActualText) > 0 {
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
        f: FontId,
        src: NativeCharSrc,
    ) -> Result<NodePtr<NativeWordNode>, Abort> {
        match src {
            NativeCharSrc::Mapped(len) => {
                let w = self.new_native_word_node(eqtb, f, len)?;
                self.entries_mut(w)
                    .copy_from_slice(&fonts.mapped_text[..len as usize]);
                Ok(w)
            }
            NativeCharSrc::Single(c) => {
                let (units, n) = crate::utf8::utf16_units(c);
                let w = self.new_native_word(f, n as i32, WhatsitSubtype::NativeWord)?;
                self.entries_mut(w).copy_from_slice(&units[..n]);
                Ok(w)
            }
        }
    }
}

/// Warn about `c` when font `f` has no glyph for it.
fn check_native_char(st: &mut EngineState, f: FontId, c: i32) -> Result<(), Abort> {
    if st.fonts.map_char_to_glyph(f, c)? == 0 {
        char_warning(st, f, c)?;
    }
    Ok(())
}

/// A measured native-word node for character `c` of font `f`, run through the
/// font's TECkit mapping if it has one.
pub(crate) fn new_native_character(
    st: &mut EngineState,
    f: FontId,
    c: u32,
) -> Result<NodePtr<NativeWordNode>, Abort> {
    let src = if let Some(map) = st.fonts.font_mapping[f].clone() {
        let (units, n) = crate::utf8::utf16_units(c);
        if st.strings.pool_ptr + n as i32 > st.strings.pool_size {
            return Err(st.strings.pool_overflow());
        }
        st.strings.append_units(&units[..n]);
        let len = st.fonts.apply_mapping(&map, st.strings.cur_string());
        st.strings.discard_cur_string();

        let mut i = 0;
        while i < len {
            let mapped = st.fonts.mapped_text[i as usize];
            if (0xD800..0xDC00).contains(&mapped) {
                let low = u32::from(st.fonts.mapped_text[(i + 1) as usize]);
                let c = (u32::from(mapped) - 0xD800) * 1024 + low + 9216;
                check_native_char(st, f, c as i32)?;
                i += 2;
            } else {
                check_native_char(st, f, i32::from(mapped))?;
                i += 1;
            }
        }
        NativeCharSrc::Mapped(len)
    } else {
        if st.eqtb.int_par(IntPar::TracingLostChars) > 0 {
            check_native_char(st, f, c as i32)?;
        }
        NativeCharSrc::Single(c)
    };

    let w = st.mem.build_native_character(&st.eqtb, &st.fonts, f, src)?;
    let use_glyph_metrics = st.eqtb.int_par(IntPar::XetexUseGlyphMetrics) > 0;
    st.fonts
        .measure_native_node(&mut st.mem, w, use_glyph_metrics)?;
    Ok(w)
}

fn tfm_getc(handle: usize) -> i32 {
    input_getc(handle) as i32
}

/// The next four TFM bytes, or `None` at end of file.
fn read_tfm_bytes(handle: usize) -> Option<[i32; 4]> {
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

fn read_fifteen(handle: usize) -> Option<i32> {
    let x = tfm_getc(handle);
    if x > 127 || x == EOF {
        return None;
    }
    Some(x * 256 + tfm_getc(handle))
}

fn read_scaled_word(handle: usize, z: i32, beta: i32, alpha: i32) -> Option<Scaled> {
    let [a, b, c, d] = read_tfm_bytes(handle)?;
    let sw = (((((d * z) / 256) + c * z) / 256) + b * z) / beta;
    match a {
        0 => Some(Scaled::from_raw(sw)),
        255 => Some(Scaled::from_raw(sw - alpha)),
        _ => None,
    }
}

/// "Font \cs=name at size", how a font loading error names the font.
#[derive(Copy, Clone)]
struct FontSpec {
    cs: EqtbPtr,
    file: FileName,
    quote: Option<u8>,
    size: Scaled,
}

impl FontSpec {
    fn new(cs: EqtbPtr, name: &ScannedName, size: Scaled) -> Self {
        Self {
            cs,
            file: name.file,
            quote: name.open_quote,
            size,
        }
    }
}

impl TexPrint for FontSpec {
    fn print_to(&self, st: &mut EngineState) {
        print(st, ("Font ", CsName(self.cs), '='));
        if let Some(q) = self.quote {
            print_char(st, q);
        }
        print(st, self.file);
        if let Some(q) = self.quote {
            print_char(st, q);
        }
        if self.size >= Scaled::ZERO {
            print(st, (" at ", self.size, "pt"));
        } else if self.size != DESIGN_SIZE {
            print(st, (" scaled ", -self.size.raw()));
        }
    }
}

pub(crate) fn read_font_info(
    st: &mut EngineState,
    u: EqtbPtr,
    name: &ScannedName,
    s: Scaled,
) -> Result<FontId, Abort> {
    let FileName {
        name: nom,
        area: aire,
        ..
    } = name.file;
    let font = FontSpec::new(u, name, s);
    let mut name_too_long = false;
    let mut file_opened = false;
    let mut g = NULL_FONT;
    let mut tfm_file = INVALID_HANDLE;

    // Native fonts get the name as written, TFM lookups a form without extension and mapping.
    let path = name.file.path(&st.strings);
    let tfm_path = FileName {
        ext: EMPTY_STRING,
        ..name.file
    }
    .path(&st.strings);
    let (tfm_name, tfm_mapping) = split_tfm_font_mapping(&tfm_path);

    if st.eqtb.int_par(IntPar::XetexTracingFonts) > 0 {
        begin_diagnostic(st);
        print_nl(st, "Requested font \"");
        print(st, path.as_slice());
        print_char(st, b'"');
        if s < Scaled::ZERO {
            print(st, " scaled ");
            print_int(st, -s.raw());
        } else {
            print(st, " at ");
            print_scaled(st, s);
            print(st, "pt");
        }
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

        if name.quoted {
            g = load_native_font(st, font, &path)?;
            if g != NULL_FONT {
                break 'read;
            }
        }

        name_too_long = st.strings.length(nom) > 255 || st.strings.length(aire) > 255;
        if name_too_long {
            bad!();
        }

        let Some((file, _)) = open_input_file(st, tfm_name, FileFormat::Tfm) else {
            if !name.quoted {
                g = load_native_font(st, font, &path)?;
                if g != NULL_FONT {
                    break 'read;
                }
            }
            bad!();
        };
        tfm_file = file;
        file_opened = true;

        let (parse, too_big) = st
            .fonts
            .read_font_info_tfm(&st.eqtb, tfm_file, nom, aire, s);
        if too_big {
            crate::errors::error(
                st,
                (font, " scaled to 2048pt or higher"),
                &["I will ignore the scaling factor."],
            )?;
        }
        match parse {
            TfmParse::OutOfMemory => {
                return Err(Abort::Internal("not enough memory to load another font"))
            }
            TfmParse::Bad => bad!(),
            TfmParse::Loaded(f) => {
                g = f;
                st.fonts.font_mapping[f] = match tfm_mapping {
                    Some(mapping) => load_mapping_file(st, mapping, true, tfm_name)?,
                    None => None,
                };
            }
        }
    }

    if bad_tfm && st.eqtb.int_par(IntPar::SuppressFontnotfoundError) == 0 {
        let reason = if file_opened {
            " not loadable: Bad metric (TFM) file"
        } else if name_too_long {
            " not loadable: Metric (TFM) file name too long"
        } else {
            " not loadable: Metric (TFM) file or installed font not found"
        };
        crate::errors::error(
            st,
            (font, reason),
            &[
                "I wasn't able to read the size data for this font,",
                "so I will ignore the font specification.",
                "[Wizards can fix TFM files using TFtoPL/PLtoTF.]",
                "You might try inserting a different font spec;",
                r"e.g., type `I\font<same font id>=<substitute font name>'.",
            ],
        )?;
    }

    if file_opened && crate::io::input_close(tfm_file) {
        return Err(Abort::Internal(
            "serious internal error closing the font metric file",
        ));
    }

    if st.eqtb.int_par(IntPar::XetexTracingFonts) > 0 && (g == NULL_FONT || file_opened) {
        begin_diagnostic(st);
        print_nl(st, " -> ");
        if g == NULL_FONT {
            print(st, "font not found, using \"nullfont\"");
        } else {
            print(st, tfm_name);
        }
        end_diagnostic(st, false);
    }

    Ok(g)
}

enum TfmParse {
    Loaded(FontId),
    Bad,
    OutOfMemory,
}

impl FontState {
    /// Read the opened TFM file into the tables of the next font slot. Also returns
    /// whether a `scaled` size reached 2048pt, so the design size was kept.
    fn read_font_info_tfm(
        &mut self,
        eqtb: &EqtbState,
        tfm_file: usize,
        nom: StrNumber,
        aire: StrNumber,
        s: Scaled,
    ) -> (TfmParse, bool) {
        let mut too_big = false;
        macro_rules! bad {
            () => {
                return (TfmParse::Bad, too_big)
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
            return (TfmParse::OutOfMemory, false);
        }

        let f = self.font_ptr.next();
        self.char_base[f] = self.fmem_ptr - bc;
        self.width_base[f] = self.char_base[f] + ec + 1;
        self.height_base[f] = self.width_base[f] + nw;
        self.depth_base[f] = self.height_base[f] + nh;
        self.italic_base[f] = self.depth_base[f] + nd;
        self.lig_kern_base[f] = self.italic_base[f] + ni;
        self.kern_base[f] = self.lig_kern_base[f] + nl - 256 * 128;
        self.exten_base[f] = self.kern_base[f] + 256 * 128 + nk;
        self.param_base[f] = self.exten_base[f] + ne;

        if lh < 2 {
            bad!();
        }

        self.font_check[f] = read!(read_tfm_bytes(tfm_file)).map(|b| b as u8);

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

        self.font_dsize[f] = Scaled::from_raw(z);
        if s != DESIGN_SIZE {
            if s >= Scaled::ZERO {
                z = s.raw();
            } else {
                let scaled = crate::scaled_math::xn_over_d(z, -s.raw(), 1000);
                // tex.ch keeps the design size rather than overflow at 2048pt (2^27 sp).
                too_big = scaled.overflowed || scaled.quotient >= 0o1_000_000_000;
                if !too_big {
                    z = scaled.quotient;
                }
            }
        }
        self.font_size[f] = Scaled::from_raw(z);

        let char_exists =
            |fonts: &FontState, ch: i32| (bc..=ec).contains(&ch) && fonts.char_info(f, ch).exists();

        for k in self.fmem_ptr.until(self.width_base[f]) {
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
                    let current = (k - self.fmem_ptr) + bc;
                    let mut dd = d;
                    while dd < current {
                        let qw = self.char_info(f, dd);
                        if qw.tag() != CharTag::List {
                            break;
                        }
                        dd = i32::from(qw.remainder());
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

        for k in self.width_base[f].until(self.lig_kern_base[f]) {
            let sw = read!(read_scaled_word(tfm_file, z, beta, alpha));
            self.set_font_info_sc(k, sw);
        }

        if self.font_info_sc(self.width_base[f]) != Scaled::ZERO
            || self.font_info_sc(self.height_base[f]) != Scaled::ZERO
            || self.font_info_sc(self.depth_base[f]) != Scaled::ZERO
            || self.font_info_sc(self.italic_base[f]) != Scaled::ZERO
        {
            bad!();
        }

        let mut bch_label = 32_767;
        let mut bchar = 256;

        if nl > 0 {
            let lig_kern_base = self.lig_kern_base[f];
            let lig_kern_end = self.kern_base[f] + 256 * 128;
            for k in lig_kern_base.until(lig_kern_end) {
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

        for k in (self.kern_base[f] + 256 * 128).until(self.exten_base[f]) {
            let sw = read!(read_scaled_word(tfm_file, z, beta, alpha));
            self.set_font_info_sc(k, sw);
        }

        for k in self.exten_base[f].until(self.param_base[f]) {
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
                Scaled::from_raw((sw * 16) + (tfm_getc(tfm_file) / 16))
            } else {
                read!(read_scaled_word(tfm_file, z, beta, alpha))
            };
            self.set_font_info_sc(self.param_base[f] + k - 1, sw);
        }
        for k in np + 1..=7 {
            self.set_font_info_sc(self.param_base[f] + k - 1, Scaled::ZERO);
        }

        self.font_params[f] = np.max(7);
        self.hyphen_char[f] = eqtb.int_par(IntPar::DefaultHyphenChar);
        self.skew_char[f] = eqtb.int_par(IntPar::DefaultSkewChar);
        if bch_label < nl {
            self.bchar_label[f] = self.lig_kern_base[f] + bch_label;
        } else {
            self.bchar_label[f] = FontInfoIdx::NON_ADDRESS;
        }
        self.font_bchar[f] = LigChar(bchar);
        self.font_false_bchar[f] = LigChar(bchar);
        if bchar <= ec && bchar >= bc && self.char_info(f, bchar).exists() {
            self.font_false_bchar[f] = LigChar::NONE;
        }

        self.font_name[f] = nom;
        self.font_area[f] = aire;
        self.font_bc[f] = bc as u16;
        self.font_ec[f] = ec as u16;
        self.font_glue[f] = None;
        self.param_base[f] -= 1;
        self.fmem_ptr += lf;
        self.font_ptr = f;
        (TfmParse::Loaded(f), too_big)
    }

    /// Set up the tables of a new native font slot. Returns the slot and the
    /// font's metrics.
    fn init_native_font_tables(
        &mut self,
        eqtb: &EqtbState,
        font_engine: crate::state::FontEngineHandle,
        full_name: StrNumber,
        actual_size: Scaled,
        num_font_dimens: i32,
    ) -> (FontId, NativeFontMetrics) {
        self.font_ptr = self.font_ptr.next();
        let font_ptr = self.font_ptr;
        self.font_area[font_ptr] = self.native_font_type_flag;
        self.font_name[font_ptr] = full_name;
        self.font_check[font_ptr] = [0; 4];
        self.font_glue[font_ptr] = None;
        self.font_dsize[font_ptr] = loaded_font_design_size();
        self.font_size[font_ptr] = actual_size;

        let metrics = if self.native_font_type_flag == AAT_FONT_FLAG {
            aat_get_font_metrics()
        } else {
            ot_get_font_metrics(&font_engine)
        };

        self.set_native_extent(font_ptr, metrics.ascent, metrics.descent);
        self.font_params[font_ptr] = num_font_dimens;
        self.font_bc[font_ptr] = 0;
        self.font_ec[font_ptr] = 65_535;
        self.font_used[font_ptr] = false;
        self.hyphen_char[font_ptr] = eqtb.int_par(IntPar::DefaultHyphenChar);
        self.skew_char[font_ptr] = eqtb.int_par(IntPar::DefaultSkewChar);
        self.param_base[font_ptr] = self.fmem_ptr - 1;
        self.font_layout_engine[font_ptr] = Some(std::rc::Rc::new(font_engine));
        self.font_mapping[font_ptr] = None;
        self.font_letter_space[font_ptr] = self.loaded_font_letter_space;

        (font_ptr, metrics)
    }

    fn push_font_info(&mut self, v: Scaled) {
        self.set_font_info_sc(self.fmem_ptr, v);
        self.fmem_ptr += 1;
    }
}

/// Load native OpenType/Graphite font `name` and set up its TeX font tables.
fn load_native_font(st: &mut EngineState, font: FontSpec, name: &[u8]) -> Result<FontId, Abort> {
    let s = font.size;
    let Some((font_engine, full_name)) = find_native_font(st, name, s)? else {
        return Ok(NULL_FONT);
    };

    let actual_size = if s >= Scaled::ZERO {
        s
    } else if s != DESIGN_SIZE {
        crate::scaled_math::xn_over_d(loaded_font_design_size(), -s.raw(), 1000).quotient
    } else {
        loaded_font_design_size()
    };

    if st.strings.pool_ptr + full_name.len() as i32 > st.strings.pool_size {
        return Err(st.strings.pool_overflow());
    }
    for &b in &full_name {
        st.strings.append_char(u16::from(b));
    }
    let full_name = st.strings.make_string()?;

    for f in st.fonts.font_ptr.loaded() {
        if st.fonts.font_area[f] == st.fonts.native_font_type_flag
            && st.strings.str_eq_str(st.fonts.font_name[f], full_name)
            && st.fonts.font_size[f] == actual_size
        {
            drop(font_engine);
            st.strings.flush_string();
            return Ok(f);
        }
    }

    let num_font_dimens = if st.fonts.native_font_type_flag == OTGR_FONT_FLAG
        && font_engine.is_opentype_math_font()
    {
        65
    } else {
        8
    };

    if st.fonts.font_ptr == st.fonts.font_max
        || st.fonts.fmem_ptr + num_font_dimens > st.fonts.font_mem_size
    {
        crate::errors::error(
            st,
            (font, " not loaded: Not enough room left"),
            &[
                "I'm afraid I won't be able to make use of this font,",
                "because my memory for character-size data is too small.",
                "If you're really stuck, ask a wizard to enlarge me.",
                r"Or maybe try `I\font<same font id>=<name of loaded font>'.",
            ],
        )?;
        return Ok(NULL_FONT);
    }

    let (font_ptr, metrics) = st.fonts.init_native_font_tables(
        &st.eqtb,
        font_engine,
        full_name,
        actual_size,
        num_font_dimens,
    );

    let space = new_native_character(st, font_ptr, u32::from(b' '))?;
    let space_width = st.mem[space].width + st.fonts.loaded_font_letter_space;
    space.free_native(&mut st.mem);

    let size = st.fonts.font_size[font_ptr];
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
        st.fonts.push_font_info(Scaled::from_raw(num_font_dimens));
        for k in 0..=55 {
            let v = st.fonts.get_ot_math_constant(font_ptr, k);
            st.fonts.push_font_info(v);
        }
    }

    let staged_mapping = st.fonts.loaded_font_mapping.take();
    st.fonts.font_mapping[font_ptr] = staged_mapping;
    st.fonts.font_flags[font_ptr] = st.fonts.loaded_font_flags;
    Ok(font_ptr)
}
