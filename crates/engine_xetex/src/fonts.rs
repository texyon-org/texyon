//! Font loading: the TFM reader, native (OpenType/Graphite) font loading, native
//! character nodes and the TFM `char_info` accessors.

use crate::consts::{EMPTY_STRING, INVALID_HANDLE, NULL_FONT, TOO_BIG_CHAR};
use crate::cs::CsId;
use crate::ext::{
    aat_get_font_metrics, apply_tfm_font_mapping, find_native_font, load_mapping_file,
    ot_get_font_metrics, split_tfm_font_mapping, NativeFontMetrics,
};
use crate::io::{open_input_file, FileName, ScannedName};
use crate::node::{NativeWordNode, NodePtr, WhatsitSubtype};
use crate::output::{
    begin_diagnostic, char_warning, end_diagnostic, print, print_char, print_int, print_nl,
    print_scaled, CsName, TexPrint,
};
use crate::params::IntPar;
use crate::state::{EngineState, EqtbState, FontState, MemState};
use crate::tfm::{ExtIdx, FontMetrics, FontParams, TfmError, TfmMetrics};
use crate::units::{FontId, Scaled, StrNumber};

use crate::bridge::FileFormat;
use crate::errors::Abort;
use crate::errors::OrConfusion;

use crate::ext::loaded_font_design_size;
use std::collections::BTreeMap;
use std::rc::Rc;

crate::codes::code_enum! {
    /// The edge of a line a character protrudes into.
    #[derive(PartialOrd, Ord)]
    pub(crate) enum Side: u16 {
        Left = 0,
        Right = 1,
    }
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

pub(crate) use crate::tfm::{
    CharInfo, CharTag, ExtRecipe, LigKernAction, LigKernIdx, LigKernStep, LigOp,
};

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

impl FontState {
    pub(crate) fn is_native_font(&self, f: FontId) -> bool {
        let area = self.font_area[f];
        area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG
    }

    /// A native font's ascent.
    pub(crate) fn native_height(&self, f: FontId) -> Scaled {
        match self.metrics[f] {
            FontMetrics::Native { ascent, .. } => ascent,
            FontMetrics::Tfm(_) => Scaled::ZERO,
        }
    }

    /// A native font's descent, as a positive depth.
    pub(crate) fn native_depth(&self, f: FontId) -> Scaled {
        match self.metrics[f] {
            FontMetrics::Native { descent, .. } => -descent,
            FontMetrics::Tfm(_) => Scaled::ZERO,
        }
    }

    /// The metrics of TFM font `f`, `None` for a native font.
    #[inline]
    fn tfm(&self, f: FontId) -> Option<&TfmMetrics> {
        match &self.metrics[f] {
            FontMetrics::Tfm(m) => Some(m),
            FontMetrics::Native { .. } => None,
        }
    }

    /// Character `c` of TFM font `f`, `None` when the font does not have it.
    #[inline]
    pub(crate) fn char_info(&self, f: FontId, c: i32) -> Option<CharInfo> {
        self.tfm(f)?.char_info(c)
    }

    /// Character `c`, with no dimensions when the font does not have it.
    #[inline]
    pub(crate) fn char_metrics(&self, f: FontId, c: i32) -> CharInfo {
        self.char_info(f, c).unwrap_or(CharInfo::NULL)
    }

    /// `c` is in the font's range and exists.
    #[inline]
    pub(crate) fn char_exists(&self, f: FontId, c: i32) -> bool {
        self.char_info(f, c).is_some()
    }

    #[inline]
    pub(crate) fn lig_kern_step(&self, f: FontId, k: LigKernIdx) -> Result<LigKernStep, Abort> {
        self.tfm(f)
            .and_then(|m| m.lig_kern_step(k))
            .or_confusion("lig/kern step")
    }

    /// The index and first step of the lig/kern program at `k`, following a redirect.
    #[inline]
    pub(crate) fn lig_kern_start(
        &self,
        f: FontId,
        k: LigKernIdx,
    ) -> Result<(LigKernIdx, LigKernStep), Abort> {
        self.tfm(f)
            .and_then(|m| m.lig_kern_start(k))
            .or_confusion("lig/kern start")
    }

    /// The program of a left boundary, `None` when the font has none.
    #[inline]
    pub(crate) fn bchar_label(&self, f: FontId) -> Option<LigKernIdx> {
        self.tfm(f)?.bchar_label()
    }

    pub(crate) fn ext_recipe(&self, f: FontId, e: ExtIdx) -> Result<ExtRecipe, Abort> {
        self.tfm(f)
            .and_then(|m| m.ext_recipe(e))
            .or_confusion("extensible recipe")
    }

    /// A TFM font's check sum, in file byte order.
    pub(crate) fn check_bytes(&self, f: FontId) -> [u8; 4] {
        self.font_check[f]
    }

    /// The width, height and depth of character `c` in font `f`.
    pub(crate) fn char_box(&mut self, f: FontId, c: u16) -> (Scaled, Scaled, Scaled) {
        let c = self.effective_char(f, c);
        let info = self.char_info(f, c).unwrap_or(CharInfo::NULL);
        (info.width, info.height, info.depth)
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

    pub(crate) fn effective_char_info(&mut self, f: FontId, c: u16) -> Option<CharInfo> {
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
        let length = usize::try_from(n).or_confusion("native word length")?;
        let w = self.alloc_array::<NativeWordNode>(length)?;
        let word = &mut self[w];
        word.subtype = subtype.into();
        word.font = f;
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

/// The size `\font` asks for.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum FontSize {
    /// `at` this size.
    At(Scaled),
    /// `scaled` by this many thousandths of the design size.
    Scaled(i32),
}

impl FontSize {
    /// The design size, which is also `scaled 1000`.
    pub(crate) const DESIGN: FontSize = FontSize::Scaled(1000);

    /// The size for a font designed at `design`.
    pub(crate) fn of_design(self, design: Scaled) -> crate::scaled_math::Divided<Scaled, i32> {
        match self {
            FontSize::At(s) => crate::scaled_math::Divided {
                quotient: s,
                remainder: 0,
                overflowed: false,
            },
            FontSize::Scaled(m) => crate::scaled_math::xn_over_d(design, m, 1000),
        }
    }

    /// TeX's encoding, a negative ratio for `scaled`, which the font manager reads.
    pub(crate) fn raw(self) -> Scaled {
        match self {
            FontSize::At(s) => s,
            FontSize::Scaled(m) => Scaled::from_raw(-m),
        }
    }
}

/// "Font \cs=name at size", how a font loading error names the font.
#[derive(Copy, Clone)]
struct FontSpec {
    cs: CsId,
    file: FileName,
    quote: Option<u8>,
    size: FontSize,
}

impl FontSpec {
    fn new(cs: CsId, name: &ScannedName, size: FontSize) -> Self {
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
        print(st, ("Font ", CsName(Some(self.cs)), '='));
        if let Some(q) = self.quote {
            print_char(st, q);
        }
        print(st, self.file);
        if let Some(q) = self.quote {
            print_char(st, q);
        }
        match self.size {
            FontSize::At(s) => print(st, (" at ", s, "pt")),
            FontSize::DESIGN => {}
            FontSize::Scaled(m) => print(st, (" scaled ", m)),
        }
    }
}

pub(crate) fn read_font_info(
    st: &mut EngineState,
    u: CsId,
    name: &ScannedName,
    s: FontSize,
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
        match s {
            FontSize::Scaled(m) => {
                print(st, " scaled ");
                print_int(st, m);
            }
            FontSize::At(s) => {
                print(st, " at ");
                print_scaled(st, s);
                print(st, "pt");
            }
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

        let mut too_big = false;
        let fonts = &st.fonts;
        let room = |words: i32| {
            fonts.font_ptr != fonts.font_max && fonts.fmem_ptr + words <= fonts.font_mem_size
        };
        let parse = crate::tfm::read_tfm(tfm_file, s, room, &mut too_big);
        if too_big {
            crate::errors::error(
                st,
                (font, " scaled to 2048pt or higher"),
                &["I will ignore the scaling factor."],
            )?;
        }
        match parse {
            Err(TfmError::OutOfMemory) => {
                return Err(Abort::Internal("not enough memory to load another font"))
            }
            Err(TfmError::Bad) => bad!(),
            Ok(tfm) => {
                let f = st.fonts.install_tfm(&st.eqtb, tfm, nom, aire);
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

impl FontState {
    /// Fills the next font slot with `tfm`.
    fn install_tfm(
        &mut self,
        eqtb: &EqtbState,
        tfm: crate::tfm::Tfm,
        nom: StrNumber,
        aire: StrNumber,
    ) -> FontId {
        let f = self.font_ptr.next();
        self.metrics[f] = FontMetrics::Tfm(Rc::new(tfm.metrics));
        self.params[f] = FontParams::new(tfm.params);
        self.font_check[f] = tfm.check;
        self.font_dsize[f] = tfm.design_size;
        self.font_size[f] = tfm.size;
        self.hyphen_char[f] = eqtb.int_par(IntPar::DefaultHyphenChar);
        self.skew_char[f] = eqtb.int_par(IntPar::DefaultSkewChar);
        self.font_bchar[f] = tfm.bchar;
        self.font_false_bchar[f] = tfm.false_bchar;
        self.font_name[f] = nom;
        self.font_area[f] = aire;
        self.font_bc[f] = tfm.bc;
        self.font_ec[f] = tfm.ec;
        self.font_glue[f] = None;
        self.fmem_ptr += tfm.words;
        self.font_ptr = f;
        f
    }

    /// Set up the tables of a new native font slot. Returns the slot and the
    /// font's metrics.
    fn init_native_font_tables(
        &mut self,
        eqtb: &EqtbState,
        font_engine: crate::state::FontEngineHandle,
        full_name: StrNumber,
        actual_size: Scaled,
        num_font_dimens: usize,
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

        self.metrics[font_ptr] = FontMetrics::Native {
            ascent: metrics.ascent,
            descent: metrics.descent,
        };
        self.params[font_ptr] = FontParams::new(vec![Scaled::ZERO; num_font_dimens]);
        self.font_bc[font_ptr] = 0;
        self.font_ec[font_ptr] = 65_535;
        self.font_used[font_ptr] = false;
        self.hyphen_char[font_ptr] = eqtb.int_par(IntPar::DefaultHyphenChar);
        self.skew_char[font_ptr] = eqtb.int_par(IntPar::DefaultSkewChar);
        self.font_layout_engine[font_ptr] = Some(std::rc::Rc::new(font_engine));
        self.font_mapping[font_ptr] = None;
        self.font_letter_space[font_ptr] = self.loaded_font_letter_space;

        (font_ptr, metrics)
    }
}

/// Load native OpenType/Graphite font `name` and set up its TeX font tables.
fn load_native_font(st: &mut EngineState, font: FontSpec, name: &[u8]) -> Result<FontId, Abort> {
    let s = font.size;
    let Some((font_engine, full_name)) = find_native_font(st, name, s)? else {
        return Ok(NULL_FONT);
    };

    let actual_size = match s {
        FontSize::DESIGN => loaded_font_design_size(),
        _ => s.of_design(loaded_font_design_size()).quotient,
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

    let num_font_dimens: usize = if st.fonts.native_font_type_flag == OTGR_FONT_FLAG
        && font_engine.is_opentype_math_font()
    {
        65
    } else {
        8
    };
    let words = i32::try_from(num_font_dimens).or_confusion("font dimens")?;

    if st.fonts.font_ptr == st.fonts.font_max || st.fonts.fmem_ptr + words > st.fonts.font_mem_size
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
    let mut params = vec![
        metrics.slant,
        space_width,
        space_width / 2,
        space_width / 3,
        metrics.x_height,
        size,
        space_width / 3,
        metrics.cap_height,
    ];
    if num_font_dimens == 65 {
        params.push(Scaled::from_raw(65));
        for k in 0..=55 {
            params.push(st.fonts.get_ot_math_constant(font_ptr, k));
        }
    }
    st.fonts.params[font_ptr] = FontParams::new(params);
    st.fonts.fmem_ptr += words;

    let staged_mapping = st.fonts.loaded_font_mapping.take();
    st.fonts.font_mapping[font_ptr] = staged_mapping;
    st.fonts.font_flags[font_ptr] = st.fonts.loaded_font_flags;
    Ok(font_ptr)
}
