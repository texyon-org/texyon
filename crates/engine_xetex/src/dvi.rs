//! The XDV writer: TeX's half-swapped `dvi_buf` and the commands shipout emits.

use crate::bridge::output_write;
use crate::consts::TEX_INFINITY;
use crate::errors::Abort;
use crate::page_manifest::PageManifest;
use crate::units::Scaled;

const DVI_BUF_SIZE: i32 = 16384;
const HALF_BUF: i32 = 8192;

const SET1: u8 = 128;
const SET_RULE: u8 = 132;
const PUT_RULE: u8 = 137;
const BOP: u8 = 139;
const EOP: u8 = 140;
const PUSH: u8 = 141;
const POP: u8 = 142;
const RIGHT1: u8 = 143;
const DOWN1: u8 = 157;
const FNT_NUM_0: u8 = 171;
const FNT1: u8 = 235;
const XXX1: u8 = 239;
const XXX4: u8 = 242;
const FNT_DEF1: u8 = 243;
const PRE: u8 = 247;
const POST: u8 = 248;
const POST_POST: u8 = 249;
const DEFINE_NATIVE_FONT: u8 = 252;
const SET_GLYPHS: u8 = 253;
const SET_TEXT_AND_GLYPHS: u8 = 254;

/// The units of DVI lengths: 10^-7 m in TeX's scaled points.
const NUMERATOR: i32 = 25_400_000;
const DENOMINATOR: i32 = 473_628_672;

/// The direction of a movement command.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Axis {
    /// `right`, with registers `w` and `x`.
    Right,
    /// `down`, with registers `y` and `z`.
    Down,
}

impl Axis {
    fn op1(self) -> u8 {
        match self {
            Axis::Right => RIGHT1,
            Axis::Down => DOWN1,
        }
    }
}

/// The first (`w` or `y`) or second (`x` or `z`) register of an axis.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum MoveReg {
    Y,
    Z,
}

impl MoveReg {
    /// The opcode distance from `right1`/`down1` to the register's `w1`..`z1`.
    fn delta(self) -> u8 {
        match self {
            MoveReg::Y => 5,
            MoveReg::Z => 10,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct DviWriter {
    pub file: usize,
    pub buf: Vec<u8>,
    limit: i32,
    pub ptr: i32,
    /// Bytes before `buf[0]`.
    pub offset: i32,
    /// Bytes already written to the file.
    pub gone: i32,
    pub manifest: PageManifest,
    pub last_bop: i32,
    /// The push depth (`cur_s`): -1 between pages, -2 once the file has overflowed.
    pub depth: i32,
    pub max_push: i32,
}

impl DviWriter {
    pub(crate) fn init(&mut self) {
        self.buf = vec![0u8; (DVI_BUF_SIZE + 1) as usize];
        self.limit = DVI_BUF_SIZE;
        self.ptr = 0;
        self.offset = 0;
        self.gone = 0;
        self.depth = -1;
    }

    /// The file position of the next byte.
    #[inline]
    pub(crate) fn pos(&self) -> i32 {
        self.offset + self.ptr
    }

    fn write_to_file(&self, a: i32, b: i32) -> Result<(), Abort> {
        if !output_write(self.file, &self.buf[a as usize..=b as usize]) {
            return Err(Abort::Internal("failed to write data to XDV file"));
        }
        Ok(())
    }

    fn check_length(&mut self) -> Result<(), Abort> {
        if self.ptr > TEX_INFINITY - self.offset {
            self.depth = -2;
            return Err(Abort::Fatal("dvi length exceeds 0x7FFFFFFF"));
        }
        Ok(())
    }

    fn swap(&mut self) -> Result<(), Abort> {
        self.check_length()?;
        if self.limit == DVI_BUF_SIZE {
            self.write_to_file(0, HALF_BUF - 1)?;
            self.limit = HALF_BUF;
            self.offset += DVI_BUF_SIZE;
            self.ptr = 0;
        } else {
            self.write_to_file(HALF_BUF, DVI_BUF_SIZE - 1)?;
            self.limit = DVI_BUF_SIZE;
        }
        self.gone += HALF_BUF;
        Ok(())
    }

    #[inline]
    fn out(&mut self, c: u8) -> Result<(), Abort> {
        self.manifest.record_byte(c);
        self.buf[self.ptr as usize] = c;
        self.ptr += 1;
        if self.ptr == self.limit {
            self.swap()?;
        }
        Ok(())
    }

    fn bytes(&mut self, mut bytes: &[u8]) -> Result<(), Abort> {
        while !bytes.is_empty() {
            let ptr = self.ptr as usize;
            let n = bytes.len().min(self.limit as usize - ptr);
            self.buf[ptr..ptr + n].copy_from_slice(&bytes[..n]);
            self.manifest.record_bytes(n);
            self.ptr += n as i32;
            bytes = &bytes[n..];
            if self.ptr == self.limit {
                self.swap()?;
            }
        }
        Ok(())
    }

    fn four(&mut self, x: i32) -> Result<(), Abort> {
        self.bytes(&x.to_be_bytes())
    }

    fn two(&mut self, x: u16) -> Result<(), Abort> {
        self.bytes(&x.to_be_bytes())
    }

    /// Low bytes of a string pool slice.
    fn text(&mut self, s: &[u16]) -> Result<(), Abort> {
        for &c in s {
            self.out(c as u8)?;
        }
        Ok(())
    }

    /// `pre`, up to the magnification that `prepare_mag` may still change.
    pub(crate) fn begin_preamble(&mut self, id: u8) -> Result<(), Abort> {
        self.bytes(&[PRE, id])?;
        self.four(NUMERATOR)?;
        self.four(DENOMINATOR)
    }

    pub(crate) fn end_preamble(&mut self, mag: i32, comment: &[u8]) -> Result<(), Abort> {
        self.four(mag)?;
        self.out(comment.len() as u8)?;
        self.bytes(comment)
    }

    pub(crate) fn bop(&mut self, counts: [i32; 10]) -> Result<(), Abort> {
        let page_loc = self.pos();
        self.manifest.begin_page(page_loc as u64);
        self.out(BOP)?;
        for count in counts {
            self.four(count)?;
        }
        self.four(self.last_bop)?;
        self.last_bop = page_loc;
        Ok(())
    }

    /// `eop`, closing the page for the manifest at `logical_clock`.
    pub(crate) fn eop(&mut self, logical_clock: u64) -> Result<(), Abort> {
        self.out(EOP)?;
        self.manifest.finish_page(logical_clock);
        Ok(())
    }

    /// Enters a box: `push` unless at page level. Returns the position to
    /// hand back to `leave_level`.
    pub(crate) fn enter_level(&mut self) -> Result<i32, Abort> {
        self.depth += 1;
        if self.depth > 0 {
            self.out(PUSH)?;
        }
        self.max_push = self.max_push.max(self.depth);
        Ok(self.pos())
    }

    /// Leaves a box entered at `save_loc`: `pop`, or drop the `push` if the
    /// box wrote nothing.
    pub(crate) fn leave_level(&mut self, save_loc: i32) -> Result<(), Abort> {
        if self.depth > 0 {
            if save_loc == self.pos() && self.ptr > 0 {
                self.ptr -= 1;
                self.manifest.pop_byte();
            } else {
                self.out(POP)?;
            }
        }
        self.depth -= 1;
        Ok(())
    }

    /// Pops the levels and ends the page left open by an interrupted
    /// shipout. Returns whether there was one.
    pub(crate) fn close_open_page(&mut self, logical_clock: u64) -> Result<bool, Abort> {
        if self.depth < 0 {
            return Ok(false);
        }
        while self.depth > 0 {
            self.out(POP)?;
            self.depth -= 1;
        }
        self.eop(logical_clock)?;
        self.depth = -1;
        Ok(true)
    }

    /// Typesets TFM character `c` and moves right.
    #[inline]
    pub(crate) fn set_char(&mut self, c: u16) -> Result<(), Abort> {
        if c >= 128 {
            self.out(SET1)?;
        }
        self.out(c as u8)
    }

    /// A rule with its bottom left at the current point, moving right.
    pub(crate) fn set_rule(&mut self, height: Scaled, width: Scaled) -> Result<(), Abort> {
        self.rule(SET_RULE, height, width)
    }

    /// A rule with its bottom left at the current point, staying put.
    pub(crate) fn put_rule(&mut self, height: Scaled, width: Scaled) -> Result<(), Abort> {
        self.rule(PUT_RULE, height, width)
    }

    fn rule(&mut self, opcode: u8, height: Scaled, width: Scaled) -> Result<(), Abort> {
        self.out(opcode)?;
        self.four(height.0)?;
        self.four(width.0)
    }

    /// Selects font number `k`.
    pub(crate) fn fnt(&mut self, k: i32) -> Result<(), Abort> {
        if k < 64 {
            self.out(FNT_NUM_0 + k as u8)
        } else if k < 256 {
            self.bytes(&[FNT1, k as u8])
        } else {
            self.bytes(&[FNT1 + 1, (k / 256) as u8, (k % 256) as u8])
        }
    }

    pub(crate) fn fnt_def(
        &mut self,
        k: i32,
        check: [u8; 4],
        size: Scaled,
        design_size: Scaled,
        area: &[u16],
        name: &[u16],
    ) -> Result<(), Abort> {
        if k < 256 {
            self.bytes(&[FNT_DEF1, k as u8])?;
        } else {
            self.bytes(&[FNT_DEF1 + 1, (k / 256) as u8, (k % 256) as u8])?;
        }
        self.bytes(&check)?;
        self.four(size.0)?;
        self.four(design_size.0)?;
        self.bytes(&[area.len() as u8, name.len() as u8])?;
        self.text(area)?;
        self.text(name)
    }

    /// XDV's `define_native_font` with its encoded definition `def`.
    pub(crate) fn native_font_def(&mut self, k: i32, def: &[u8]) -> Result<(), Abort> {
        self.out(DEFINE_NATIVE_FONT)?;
        self.four(k)?;
        self.bytes(def)
    }

    /// One glyph at the current point, advancing by `width`.
    pub(crate) fn set_glyph(&mut self, width: Scaled, glyph: u16) -> Result<(), Abort> {
        self.out(SET_GLYPHS)?;
        self.four(width.0)?;
        self.two(1)?;
        self.four(0)?;
        self.four(0)?;
        self.two(glyph)
    }

    /// `set_glyphs` with an encoded glyph array.
    pub(crate) fn set_glyphs(&mut self, glyphs: &[u8]) -> Result<(), Abort> {
        self.out(SET_GLYPHS)?;
        self.bytes(glyphs)
    }

    /// `set_text_and_glyphs`: the UTF-16 source text, then the glyph array.
    pub(crate) fn set_text_and_glyphs(&mut self, text: &[u16], glyphs: &[u8]) -> Result<(), Abort> {
        self.out(SET_TEXT_AND_GLYPHS)?;
        self.two(text.len() as u16)?;
        for &unit in text {
            self.two(unit)?;
        }
        self.bytes(glyphs)
    }

    /// `xxx` with the low bytes of `s`.
    pub(crate) fn special(&mut self, s: &[u16]) -> Result<(), Abort> {
        let len = s.len();
        if len < 256 {
            self.bytes(&[XXX1, len as u8])?;
        } else {
            self.out(XXX4)?;
            self.four(len as i32)?;
        }
        self.text(s)
    }

    /// `right` or `down` by `w`, in as few bytes as it fits.
    pub(crate) fn movement(&mut self, axis: Axis, w: Scaled) -> Result<(), Abort> {
        let w = w.0;
        let len = if w.abs() >= 0x800000 {
            4
        } else if w.abs() >= 0x8000 {
            3
        } else if w.abs() >= 128 {
            2
        } else {
            1
        };
        self.out(axis.op1() + len as u8 - 1)?;
        self.bytes(&w.to_be_bytes()[4 - len..])
    }

    /// Moves by the amount already in register `reg` (`w0`, `x0`, `y0`, `z0`).
    pub(crate) fn movement_by_reg(&mut self, axis: Axis, reg: MoveReg) -> Result<(), Abort> {
        self.out(axis.op1() + reg.delta() - 1)
    }

    /// Turns the movement at file position `location` into one that also sets
    /// `reg`. Fails once that byte has left the buffer.
    pub(crate) fn set_reg_at(&mut self, location: i32, reg: MoveReg) -> bool {
        if location < self.gone {
            return false;
        }
        let mut k = location - self.offset;
        if k < 0 {
            k += DVI_BUF_SIZE;
        }
        self.buf[k as usize] += reg.delta();
        self.manifest.patch_byte(location as u64, reg.delta());
        true
    }

    /// `post`, up to the magnification that `prepare_mag` may still change.
    pub(crate) fn begin_postamble(&mut self) -> Result<(), Abort> {
        self.out(POST)?;
        self.four(self.last_bop)?;
        self.last_bop = self.pos() - 5;
        self.four(NUMERATOR)?;
        self.four(DENOMINATOR)
    }

    pub(crate) fn end_postamble(
        &mut self,
        mag: i32,
        max_v: Scaled,
        max_h: Scaled,
        total_pages: i32,
    ) -> Result<(), Abort> {
        self.four(mag)?;
        self.four(max_v.0)?;
        self.four(max_h.0)?;
        let max_push = self.max_push;
        self.bytes(&[
            (max_push / 256) as u8,
            (max_push % 256) as u8,
            ((total_pages / 256) % 256) as u8,
            (total_pages % 256) as u8,
        ])
    }

    /// `post_post` and the padding, then flushes the buffer.
    pub(crate) fn finish(&mut self, id: u8) -> Result<(), Abort> {
        self.out(POST_POST)?;
        self.four(self.last_bop)?;
        self.out(id)?;
        let padding = 4 + (DVI_BUF_SIZE - self.ptr) % 4;
        for _ in 0..padding {
            self.out(223)?;
        }
        if self.limit == HALF_BUF {
            self.write_to_file(HALF_BUF, DVI_BUF_SIZE - 1)?;
        }
        self.check_length()?;
        if self.ptr > 0 {
            self.write_to_file(0, self.ptr - 1)?;
        }
        Ok(())
    }
}
