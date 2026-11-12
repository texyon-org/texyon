//! TFM font metrics, decoded once at load into what they mean.

use std::rc::Rc;

use crate::bridge::input_getc;
use crate::consts::DESIGN_SIZE;
use crate::fonts::LigChar;
use crate::units::Scaled;

/// An index into a font's lig/kern program.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct LigKernIdx(u16);

impl LigKernIdx {
    pub(crate) const fn from_raw(raw: u16) -> LigKernIdx {
        LigKernIdx(raw)
    }

    pub(crate) const fn raw(self) -> u16 {
        self.0
    }

    fn index(self) -> usize {
        usize::from(self.0)
    }
}

/// An index into a font's extensible recipes.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct ExtIdx(u8);

impl ExtIdx {
    pub(crate) const fn from_raw(raw: u8) -> ExtIdx {
        ExtIdx(raw)
    }

    pub(crate) const fn raw(self) -> u8 {
        self.0
    }
}

/// What else a character has besides its dimensions.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum CharTag {
    None,
    /// The start of its lig/kern program.
    Lig(LigKernIdx),
    /// The next larger character.
    List(u16),
    /// Its extensible recipe.
    Ext(ExtIdx),
}

/// The metrics of one character of a TFM font.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct CharInfo {
    pub width: Scaled,
    pub height: Scaled,
    pub depth: Scaled,
    pub italic: Scaled,
    pub tag: CharTag,
}

impl CharInfo {
    /// TeX's `null_character`: no dimensions and no tag.
    pub(crate) const NULL: CharInfo = CharInfo {
        width: Scaled::ZERO,
        height: Scaled::ZERO,
        depth: Scaled::ZERO,
        italic: Scaled::ZERO,
        tag: CharTag::None,
    };
}

/// A ligature operation, named as in PL files.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum LigOp {
    /// `=:` replaces both characters.
    Replace,
    /// `=:|` replaces the left character.
    KeepRight,
    /// `|=:` replaces the right character.
    KeepLeft,
    /// `|=:|` inserts between them.
    KeepBoth,
    /// `=:|>`, then moves past the result.
    KeepRightSkip,
    /// `|=:>`, then moves past the left character.
    KeepLeftSkip,
    /// `|=:|>`, then moves past the left character.
    KeepBothSkip,
    /// `|=:|>>`, then moves past the inserted character.
    KeepBothSkip2,
}

impl LigOp {
    /// The operation of TFM op byte `op`, `None` for an undefined one.
    pub(crate) const fn from_byte(op: u8) -> Option<LigOp> {
        Some(match op {
            0 => LigOp::Replace,
            1 => LigOp::KeepRight,
            2 => LigOp::KeepLeft,
            3 => LigOp::KeepBoth,
            5 => LigOp::KeepRightSkip,
            6 => LigOp::KeepLeftSkip,
            7 => LigOp::KeepBothSkip,
            11 => LigOp::KeepBothSkip2,
            _ => return None,
        })
    }

    pub(crate) const fn byte(self) -> u8 {
        match self {
            LigOp::Replace => 0,
            LigOp::KeepRight => 1,
            LigOp::KeepLeft => 2,
            LigOp::KeepBoth => 3,
            LigOp::KeepRightSkip => 5,
            LigOp::KeepLeftSkip => 6,
            LigOp::KeepBothSkip => 7,
            LigOp::KeepBothSkip2 => 11,
        }
    }
}

/// What a matching lig/kern rule does.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum LigKernAction {
    Kern(Scaled),
    Lig { op: LigOp, char: u16 },
}

/// One step of a lig/kern program.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum LigKernStep {
    /// A skip byte above 128. As a character's first step it redirects to the
    /// real program, anywhere else it ends the program.
    Jump(LigKernIdx),
    Rule {
        next_char: u16,
        /// Steps to skip to the character's next rule, `None` after its last.
        skip: Option<u8>,
        action: LigKernAction,
    },
}

impl LigKernStep {
    /// The action of this step when the next character is `c`.
    #[inline]
    pub(crate) fn action_for(self, c: LigChar) -> Option<LigKernAction> {
        match self {
            LigKernStep::Rule {
                next_char, action, ..
            } if i32::from(next_char) == c.raw() => Some(action),
            _ => None,
        }
    }

    /// The step after this one at `k`, `None` at the end of the program.
    #[inline]
    pub(crate) fn next(self, k: LigKernIdx) -> Option<LigKernIdx> {
        match self {
            LigKernStep::Rule {
                skip: Some(skip), ..
            } => Some(LigKernIdx(k.0 + u16::from(skip) + 1)),
            _ => None,
        }
    }
}

/// An extensible recipe: the pieces a delimiter is built from.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct ExtRecipe {
    pub top: Option<u16>,
    pub mid: Option<u16>,
    pub bot: Option<u16>,
    pub rep: u16,
}

/// The immutable metrics of a TFM font.
#[derive(Default, PartialEq, Eq, Hash, Debug)]
pub(crate) struct TfmMetrics {
    bc: u16,
    /// Characters `bc..`, `None` where the font has none.
    chars: Box<[Option<CharInfo>]>,
    lig_kern: Box<[LigKernStep]>,
    exten: Box<[ExtRecipe]>,
    /// The program for a left boundary.
    bchar_label: Option<LigKernIdx>,
}

impl TfmMetrics {
    /// The metrics, or `None` when an index points outside its table or a
    /// charlist cycles.
    pub(crate) fn new(
        bc: u16,
        chars: Box<[Option<CharInfo>]>,
        lig_kern: Box<[LigKernStep]>,
        exten: Box<[ExtRecipe]>,
        bchar_label: Option<LigKernIdx>,
    ) -> Option<TfmMetrics> {
        let metrics = TfmMetrics {
            bc,
            chars,
            lig_kern,
            exten,
            bchar_label,
        };
        metrics.is_consistent().then_some(metrics)
    }

    fn is_consistent(&self) -> bool {
        let lig_ok = |k: LigKernIdx| k.index() < self.lig_kern.len();
        let tags_ok = self.chars.iter().flatten().all(|ci| match ci.tag {
            CharTag::None => true,
            CharTag::Lig(k) => lig_ok(k),
            CharTag::List(c) => self.list_ends(c),
            CharTag::Ext(e) => usize::from(e.0) < self.exten.len(),
        });
        let steps_ok = self
            .lig_kern
            .iter()
            .enumerate()
            .all(|(k, step)| match *step {
                LigKernStep::Jump(target) => lig_ok(target),
                LigKernStep::Rule {
                    skip: Some(skip), ..
                } => k + usize::from(skip) + 1 < self.lig_kern.len(),
                LigKernStep::Rule { skip: None, .. } => true,
            });
        tags_ok && steps_ok && self.bchar_label.is_none_or(lig_ok)
    }

    /// The charlist from `c` stays in range and ends.
    fn list_ends(&self, mut c: u16) -> bool {
        for _ in 0..=self.chars.len() {
            let Some(slot) = c.checked_sub(self.bc).map(usize::from) else {
                return false;
            };
            match self.chars.get(slot) {
                None => return false,
                Some(Some(CharInfo {
                    tag: CharTag::List(next),
                    ..
                })) => c = *next,
                Some(_) => return true,
            }
        }
        false
    }

    /// Character `c`, `None` when the font does not have it.
    #[inline]
    pub(crate) fn char_info(&self, c: i32) -> Option<CharInfo> {
        let slot = usize::try_from(c - i32::from(self.bc)).ok()?;
        self.chars.get(slot).copied().flatten()
    }

    #[inline]
    pub(crate) fn lig_kern_step(&self, k: LigKernIdx) -> Option<LigKernStep> {
        self.lig_kern.get(k.index()).copied()
    }

    /// The index and first step of the program starting at `k`, following a
    /// redirect.
    #[inline]
    pub(crate) fn lig_kern_start(&self, k: LigKernIdx) -> Option<(LigKernIdx, LigKernStep)> {
        match self.lig_kern_step(k)? {
            LigKernStep::Jump(target) => Some((target, self.lig_kern_step(target)?)),
            step => Some((k, step)),
        }
    }

    pub(crate) fn ext_recipe(&self, e: ExtIdx) -> Option<ExtRecipe> {
        self.exten.get(usize::from(e.0)).copied()
    }

    pub(crate) fn bchar_label(&self) -> Option<LigKernIdx> {
        self.bchar_label
    }

    pub(crate) fn bc(&self) -> u16 {
        self.bc
    }

    pub(crate) fn chars(&self) -> &[Option<CharInfo>] {
        &self.chars
    }

    pub(crate) fn lig_kern(&self) -> &[LigKernStep] {
        &self.lig_kern
    }

    pub(crate) fn exten(&self) -> &[ExtRecipe] {
        &self.exten
    }
}

/// How a font measures its characters.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum FontMetrics {
    Tfm(Rc<TfmMetrics>),
    /// An AAT, OpenType or Graphite font, measured by its layout engine.
    Native {
        ascent: Scaled,
        descent: Scaled,
    },
}

impl Default for FontMetrics {
    fn default() -> Self {
        FontMetrics::Tfm(Rc::default())
    }
}

/// A font's `\fontdimen` parameters, shared between clones until one is written.
#[derive(Clone, Default, PartialEq, Eq, Hash, Debug)]
pub(crate) struct FontParams(Rc<[Scaled]>);

impl FontParams {
    pub(crate) fn new(values: Vec<Scaled>) -> FontParams {
        FontParams(values.into())
    }

    /// TeX's `font_params`.
    pub(crate) fn count(&self) -> i32 {
        i32::try_from(self.0.len()).unwrap_or(i32::MAX)
    }

    /// Parameter `n`, counted from 1.
    #[inline]
    pub(crate) fn get(&self, n: i32) -> Option<Scaled> {
        let slot = usize::try_from(n).ok()?.checked_sub(1)?;
        self.0.get(slot).copied()
    }

    /// Sets parameter `n`. Returns false when the font has no such parameter.
    pub(crate) fn set(&mut self, n: i32, v: Scaled) -> bool {
        let Some(slot) = usize::try_from(n).ok().and_then(|n| n.checked_sub(1)) else {
            return false;
        };
        match Rc::make_mut(&mut self.0).get_mut(slot) {
            Some(p) => {
                *p = v;
                true
            }
            None => false,
        }
    }

    /// Adds a zero parameter after the last.
    pub(crate) fn grow(&mut self) {
        let mut values = self.0.to_vec();
        values.push(Scaled::ZERO);
        self.0 = values.into();
    }

    pub(crate) fn as_slice(&self) -> &[Scaled] {
        &self.0
    }
}

/// A TFM file that failed to load.
pub(crate) enum TfmError {
    /// The file is malformed.
    Bad,
    /// No font slot or font memory is left for it.
    OutOfMemory,
}

/// A parsed TFM font, ready to fill a font slot.
pub(crate) struct Tfm {
    pub metrics: TfmMetrics,
    pub params: Vec<Scaled>,
    pub check: [u8; 4],
    pub design_size: Scaled,
    pub size: Scaled,
    pub bc: u16,
    pub ec: u16,
    pub bchar: LigChar,
    pub false_bchar: LigChar,
    /// TeX's count of `font_info` words for the font.
    pub words: i32,
}

const EOF: i32 = -1;

fn getc(handle: usize) -> i32 {
    input_getc(handle) as i32
}

fn read_fifteen(handle: usize) -> Result<i32, TfmError> {
    let x = getc(handle);
    if x > 127 || x == EOF {
        return Err(TfmError::Bad);
    }
    Ok(x * 256 + getc(handle))
}

fn read_bytes(handle: usize) -> Result<[i32; 4], TfmError> {
    let bytes = [getc(handle), getc(handle), getc(handle), getc(handle)];
    if bytes.contains(&EOF) {
        return Err(TfmError::Bad);
    }
    Ok(bytes)
}

/// The fix_word scaler of TeX's `store_scaled`.
struct FixScale {
    z: i32,
    alpha: i32,
    beta: i32,
}

impl FixScale {
    fn new(mut z: i32) -> FixScale {
        let mut alpha = 16;
        while z >= 0x80_0000 {
            z /= 2;
            alpha += alpha;
        }
        FixScale {
            z,
            beta: 256 / alpha,
            alpha: alpha * z,
        }
    }

    fn read(&self, handle: usize) -> Result<Scaled, TfmError> {
        let [a, b, c, d] = read_bytes(handle)?;
        let z = self.z;
        let sw = (((((d * z) / 256) + c * z) / 256) + b * z) / self.beta;
        match a {
            0 => Ok(Scaled::from_raw(sw)),
            255 => Ok(Scaled::from_raw(sw - self.alpha)),
            _ => Err(TfmError::Bad),
        }
    }

    fn read_n(&self, handle: usize, n: i32) -> Result<Vec<Scaled>, TfmError> {
        (0..n).map(|_| self.read(handle)).collect()
    }
}

fn byte(v: i32) -> Result<u8, TfmError> {
    u8::try_from(v).map_err(|_| TfmError::Bad)
}

fn char16(v: i32) -> Result<u16, TfmError> {
    u16::try_from(v).map_err(|_| TfmError::Bad)
}

/// Reads the TFM file open at `handle` at size `s`. `room` says whether a
/// font of that many `font_info` words still fits. Sets `too_big` when a
/// `scaled` size reached 2048pt, so the design size was kept.
pub(crate) fn read_tfm(
    handle: usize,
    s: Scaled,
    room: impl FnOnce(i32) -> bool,
    too_big: &mut bool,
) -> Result<Tfm, TfmError> {
    use TfmError::Bad;

    let lf = read_fifteen(handle)?;
    let mut lh = read_fifteen(handle)?;
    let mut bc = read_fifteen(handle)?;
    let mut ec = read_fifteen(handle)?;
    if bc > ec + 1 || ec > 255 {
        return Err(Bad);
    }
    if bc > 255 {
        bc = 1;
        ec = 0;
    }
    let nw = read_fifteen(handle)?;
    let nh = read_fifteen(handle)?;
    let nd = read_fifteen(handle)?;
    let ni = read_fifteen(handle)?;
    let nl = read_fifteen(handle)?;
    let nk = read_fifteen(handle)?;
    let ne = read_fifteen(handle)?;
    let np = read_fifteen(handle)?;
    if lf != 6 + lh + (ec - bc + 1) + nw + nh + nd + ni + nl + nk + ne + np {
        return Err(Bad);
    }
    if nw == 0 || nh == 0 || nd == 0 || ni == 0 {
        return Err(Bad);
    }

    let words = lf - 6 - lh + (7 - np).max(0);
    if !room(words) {
        return Err(TfmError::OutOfMemory);
    }
    if lh < 2 {
        return Err(Bad);
    }

    let check = read_bytes(handle)?.map(|b| b as u8);
    let mut z = read_fifteen(handle)?;
    z = z * 256 + getc(handle);
    z = (z * 16) + (getc(handle) / 16);
    if z < 65_536 {
        return Err(Bad);
    }
    while lh > 2 {
        for _ in 0..4 {
            getc(handle);
        }
        lh -= 1;
    }

    let design_size = Scaled::from_raw(z);
    if s != DESIGN_SIZE {
        if s >= Scaled::ZERO {
            z = s.raw();
        } else {
            let scaled = crate::scaled_math::xn_over_d(z, -s.raw(), 1000);
            // tex.ch keeps the design size rather than overflow at 2048pt (2^27 sp).
            *too_big = scaled.overflowed || scaled.quotient >= 0o1_000_000_000;
            if !*too_big {
                z = scaled.quotient;
            }
        }
    }
    let size = Scaled::from_raw(z);

    let mut char_words: Vec<[i32; 4]> = Vec::new();
    for current in bc..=ec {
        let [a, b, c, d] = read_bytes(handle)?;
        if a >= nw || b / 16 >= nh || b % 16 >= nd || c / 4 >= ni {
            return Err(Bad);
        }
        match c % 4 {
            1 if d >= nl => return Err(Bad),
            3 if d >= ne => return Err(Bad),
            2 => {
                if d < bc || d > ec {
                    return Err(Bad);
                }
                // A charlist must not cycle back to the current character.
                let mut dd = d;
                while dd < current {
                    let [_, _, qc, qd] = char_words[(dd - bc) as usize];
                    if qc % 4 != 2 {
                        break;
                    }
                    dd = qd;
                }
                if dd == current {
                    return Err(Bad);
                }
            }
            _ => {}
        }
        char_words.push([a, b, c, d]);
    }
    let char_exists = |ch: i32| (bc..=ec).contains(&ch) && char_words[(ch - bc) as usize][0] > 0;

    let scale = FixScale::new(z);
    let widths = scale.read_n(handle, nw)?;
    let heights = scale.read_n(handle, nh)?;
    let depths = scale.read_n(handle, nd)?;
    let italics = scale.read_n(handle, ni)?;
    if [&widths, &heights, &depths, &italics]
        .iter()
        .any(|t| t[0] != Scaled::ZERO)
    {
        return Err(Bad);
    }

    let mut bch_label = 32_767;
    let mut bchar = 256;
    let mut lig_words: Vec<[i32; 4]> = Vec::new();
    for k in 0..nl {
        let [a, b, c, d] = read_bytes(handle)?;
        if a > 128 {
            if 256 * c + d >= nl {
                return Err(Bad);
            }
            if a == 255 && k == 0 {
                bchar = b;
            }
        } else {
            if b != bchar && !char_exists(b) {
                return Err(Bad);
            }
            if c < 128 {
                if !char_exists(d) || LigOp::from_byte(byte(c)?).is_none() {
                    return Err(Bad);
                }
            } else if 256 * (c - 128) + d >= nk {
                return Err(Bad);
            }
            if a < 128 && k + a + 1 >= nl {
                return Err(Bad);
            }
        }
        if k == nl - 1 && a == 255 {
            bch_label = 256 * c + d;
        }
        lig_words.push([a, b, c, d]);
    }

    let kerns = scale.read_n(handle, nk)?;

    let mut exten = Vec::new();
    for _ in 0..ne {
        let [a, b, c, d] = read_bytes(handle)?;
        for ch in [a, b, c] {
            if ch != 0 && !char_exists(ch) {
                return Err(Bad);
            }
        }
        if !char_exists(d) {
            return Err(Bad);
        }
        let piece =
            |ch: i32| -> Result<Option<u16>, TfmError> { Ok((ch != 0).then_some(char16(ch)?)) };
        exten.push(ExtRecipe {
            top: piece(a)?,
            mid: piece(b)?,
            bot: piece(c)?,
            rep: char16(d)?,
        });
    }

    let mut params = Vec::new();
    for k in 1..=np {
        params.push(if k == 1 {
            // The slant is an unscaled fix_word.
            let mut sw = getc(handle);
            if sw == EOF {
                return Err(Bad);
            }
            if sw > 127 {
                sw -= 256;
            }
            sw = sw * 256 + getc(handle);
            sw = sw * 256 + getc(handle);
            Scaled::from_raw((sw * 16) + (getc(handle) / 16))
        } else {
            scale.read(handle)?
        });
    }
    params.resize(params.len().max(7), Scaled::ZERO);

    let chars = char_words
        .iter()
        .map(|&[a, b, c, d]| -> Result<Option<CharInfo>, TfmError> {
            if a == 0 {
                return Ok(None);
            }
            let tag = match c % 4 {
                0 => CharTag::None,
                1 => CharTag::Lig(LigKernIdx(char16(d)?)),
                2 => CharTag::List(char16(d)?),
                _ => CharTag::Ext(ExtIdx(byte(d)?)),
            };
            Ok(Some(CharInfo {
                width: widths[a as usize],
                height: heights[(b / 16) as usize],
                depth: depths[(b % 16) as usize],
                italic: italics[(c / 4) as usize],
                tag,
            }))
        })
        .collect::<Result<Box<[_]>, _>>()?;

    let lig_kern = lig_words
        .iter()
        .map(|&[a, b, c, d]| -> Result<LigKernStep, TfmError> {
            if a > 128 {
                return Ok(LigKernStep::Jump(LigKernIdx(char16(256 * c + d)?)));
            }
            let action = if c >= 128 {
                LigKernAction::Kern(kerns[(256 * (c - 128) + d) as usize])
            } else {
                LigKernAction::Lig {
                    op: LigOp::from_byte(byte(c)?).ok_or(Bad)?,
                    char: char16(d)?,
                }
            };
            Ok(LigKernStep::Rule {
                next_char: char16(b)?,
                skip: (a < 128).then_some(byte(a)?),
                action,
            })
        })
        .collect::<Result<Box<[_]>, _>>()?;

    let bchar_label = (bch_label < nl).then_some(LigKernIdx(char16(bch_label)?));
    let bc16 = char16(bc)?;
    let metrics = TfmMetrics::new(bc16, chars, lig_kern, exten.into(), bchar_label).ok_or(Bad)?;
    let false_bchar = if char_exists(bchar) {
        LigChar::NONE
    } else {
        LigChar::from_raw(bchar)
    };
    Ok(Tfm {
        metrics,
        params,
        check,
        design_size,
        size,
        bc: bc16,
        ec: char16(ec)?,
        bchar: LigChar::from_raw(bchar),
        false_bchar,
        words,
    })
}
