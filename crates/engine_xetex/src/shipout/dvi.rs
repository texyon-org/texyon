//! The XDV writer: TeX's half-swapped `dvi_buf` and the commands shipout emits.

use std::mem;

use crate::base::consts::TEX_INFINITY;
use crate::base::units::Scaled;
use crate::env::EngineEnv;
use crate::memory::glyph_arena::FixedPoint;
use crate::preview::page_manifest::PageManifest;
use crate::print::errors::Abort;

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
    buf: Vec<u8>,
    limit: i32,
    ptr: i32,
    /// Bytes before `buf[0]`.
    offset: i32,
    /// Bytes already written to the file.
    gone: i32,
    pub manifest: PageManifest,
    pub last_bop: i32,
    /// The push depth (`cur_s`): -1 between pages, -2 once the file has overflowed.
    pub depth: i32,
    pub max_push: i32,
}

/// What a `fnt_def` records about a TFM font besides its name.
#[derive(Clone, Copy)]
pub(crate) struct DviFontMetrics {
    pub check: [u8; 4],
    pub size: Scaled,
    pub design_size: Scaled,
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

    #[inline]
    pub(crate) fn pos(&self) -> i32 {
        self.offset + self.ptr
    }

    pub(crate) fn bytes_written(&self) -> i32 {
        self.gone
    }

    /// The half-swapped buffer, with `[ptr, offset, gone]`, for state digests.
    pub(crate) fn digest_view(&self) -> ([i32; 3], &[u8]) {
        ([self.ptr, self.offset, self.gone], &self.buf)
    }

    pub(crate) fn buffer_capacity(&self) -> usize {
        self.buf.capacity()
    }

    pub(crate) fn release_buffer(&mut self) {
        self.buf = Vec::new();
    }

    /// This writer for a checkpoint, without the page manifest of the
    /// current operation.
    pub(crate) fn into_checkpoint(mut self) -> DviWriter {
        self.manifest = PageManifest::default();
        self
    }

    /// Returns to a checkpoint, reusing this buffer's allocation and keeping
    /// the page manifest.
    pub(crate) fn restore(&mut self, saved: &DviWriter) {
        let DviWriter {
            ref buf,
            limit,
            ptr,
            offset,
            gone,
            manifest: _,
            last_bop,
            depth,
            max_push,
        } = *saved;
        self.buf.clone_from(buf);
        *self = DviWriter {
            buf: mem::take(&mut self.buf),
            limit,
            ptr,
            offset,
            gone,
            manifest: mem::take(&mut self.manifest),
            last_bop,
            depth,
            max_push,
        };
    }

    fn write_to_file(&self, env: &mut EngineEnv, a: i32, b: i32) -> Result<(), Abort> {
        if !env.write_xdv(&self.buf[a as usize..=b as usize]) {
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

    fn swap(&mut self, env: &mut EngineEnv) -> Result<(), Abort> {
        self.check_length()?;
        if self.limit == DVI_BUF_SIZE {
            self.write_to_file(env, 0, HALF_BUF - 1)?;
            self.limit = HALF_BUF;
            self.offset += DVI_BUF_SIZE;
            self.ptr = 0;
        } else {
            self.write_to_file(env, HALF_BUF, DVI_BUF_SIZE - 1)?;
            self.limit = DVI_BUF_SIZE;
        }
        self.gone += HALF_BUF;
        Ok(())
    }

    #[inline]
    fn out(&mut self, env: &mut EngineEnv, c: u8) -> Result<(), Abort> {
        self.manifest.record_byte(c);
        self.buf[self.ptr as usize] = c;
        self.ptr += 1;
        if self.ptr == self.limit {
            self.swap(env)?;
        }
        Ok(())
    }

    #[inline]
    fn bytes(&mut self, env: &mut EngineEnv, mut bytes: &[u8]) -> Result<(), Abort> {
        while !bytes.is_empty() {
            let ptr = self.ptr as usize;
            let n = bytes.len().min(self.limit as usize - ptr);
            self.buf[ptr..ptr + n].copy_from_slice(&bytes[..n]);
            self.manifest.record_bytes(n);
            self.ptr += n as i32;
            bytes = &bytes[n..];
            if self.ptr == self.limit {
                self.swap(env)?;
            }
        }
        Ok(())
    }

    #[inline]
    fn four(&mut self, env: &mut EngineEnv, x: i32) -> Result<(), Abort> {
        self.bytes(env, &x.to_be_bytes())
    }

    #[inline]
    fn two(&mut self, env: &mut EngineEnv, x: u16) -> Result<(), Abort> {
        self.bytes(env, &x.to_be_bytes())
    }

    fn text(&mut self, env: &mut EngineEnv, s: &[u16]) -> Result<(), Abort> {
        for &c in s {
            self.out(env, c as u8)?;
        }
        Ok(())
    }

    /// `pre`, up to the magnification that `prepare_mag` may still change.
    pub(crate) fn begin_preamble(&mut self, env: &mut EngineEnv, id: u8) -> Result<(), Abort> {
        self.bytes(env, &[PRE, id])?;
        self.four(env, NUMERATOR)?;
        self.four(env, DENOMINATOR)
    }

    pub(crate) fn end_preamble(
        &mut self,
        env: &mut EngineEnv,
        mag: i32,
        comment: &[u8],
    ) -> Result<(), Abort> {
        self.four(env, mag)?;
        self.out(env, comment.len() as u8)?;
        self.bytes(env, comment)
    }

    pub(crate) fn bop(&mut self, env: &mut EngineEnv, counts: [i32; 10]) -> Result<(), Abort> {
        let page_loc = self.pos();
        self.manifest.begin_page(page_loc as u64);
        self.out(env, BOP)?;
        for count in counts {
            self.four(env, count)?;
        }
        self.four(env, self.last_bop)?;
        self.last_bop = page_loc;
        Ok(())
    }

    pub(crate) fn eop(&mut self, env: &mut EngineEnv, logical_clock: u64) -> Result<(), Abort> {
        self.out(env, EOP)?;
        self.manifest.finish_page(logical_clock);
        Ok(())
    }

    /// Enters a box: `push` unless at page level. Returns the position to
    /// hand back to `leave_level`.
    pub(crate) fn enter_level(&mut self, env: &mut EngineEnv) -> Result<i32, Abort> {
        self.depth += 1;
        if self.depth > 0 {
            self.out(env, PUSH)?;
        }
        self.max_push = self.max_push.max(self.depth);
        Ok(self.pos())
    }

    /// Leaves a box entered at `save_loc`: `pop`, or drop the `push` if the
    /// box wrote nothing.
    pub(crate) fn leave_level(&mut self, env: &mut EngineEnv, save_loc: i32) -> Result<(), Abort> {
        if self.depth > 0 {
            if save_loc == self.pos() && self.ptr > 0 {
                self.ptr -= 1;
                self.manifest.pop_byte();
            } else {
                self.out(env, POP)?;
            }
        }
        self.depth -= 1;
        Ok(())
    }

    /// Pops the levels and ends the page left open by an interrupted
    /// shipout. Returns whether there was one.
    pub(crate) fn close_open_page(
        &mut self,
        env: &mut EngineEnv,
        logical_clock: u64,
    ) -> Result<bool, Abort> {
        if self.depth < 0 {
            return Ok(false);
        }
        while self.depth > 0 {
            self.out(env, POP)?;
            self.depth -= 1;
        }
        self.eop(env, logical_clock)?;
        self.depth = -1;
        Ok(true)
    }

    #[inline]
    pub(crate) fn set_char(&mut self, env: &mut EngineEnv, c: u16) -> Result<(), Abort> {
        if c >= 128 {
            self.out(env, SET1)?;
        }
        self.out(env, c as u8)
    }

    pub(crate) fn set_rule(
        &mut self,
        env: &mut EngineEnv,
        height: Scaled,
        width: Scaled,
    ) -> Result<(), Abort> {
        self.rule(env, SET_RULE, height, width)
    }

    pub(crate) fn put_rule(
        &mut self,
        env: &mut EngineEnv,
        height: Scaled,
        width: Scaled,
    ) -> Result<(), Abort> {
        self.rule(env, PUT_RULE, height, width)
    }

    fn rule(
        &mut self,
        env: &mut EngineEnv,
        opcode: u8,
        height: Scaled,
        width: Scaled,
    ) -> Result<(), Abort> {
        self.out(env, opcode)?;
        self.four(env, height.raw())?;
        self.four(env, width.raw())
    }

    pub(crate) fn fnt(&mut self, env: &mut EngineEnv, k: i32) -> Result<(), Abort> {
        if k < 64 {
            self.out(env, FNT_NUM_0 + k as u8)
        } else if k < 256 {
            self.bytes(env, &[FNT1, k as u8])
        } else {
            self.bytes(env, &[FNT1 + 1, (k / 256) as u8, (k % 256) as u8])
        }
    }

    pub(crate) fn fnt_def(
        &mut self,
        env: &mut EngineEnv,
        k: i32,
        metrics: DviFontMetrics,
        area: &[u16],
        name: &[u16],
    ) -> Result<(), Abort> {
        if k < 256 {
            self.bytes(env, &[FNT_DEF1, k as u8])?;
        } else {
            self.bytes(env, &[FNT_DEF1 + 1, (k / 256) as u8, (k % 256) as u8])?;
        }
        self.bytes(env, &metrics.check)?;
        self.four(env, metrics.size.raw())?;
        self.four(env, metrics.design_size.raw())?;
        self.bytes(env, &[area.len() as u8, name.len() as u8])?;
        self.text(env, area)?;
        self.text(env, name)
    }

    pub(crate) fn native_font_def(
        &mut self,
        env: &mut EngineEnv,
        k: i32,
        def: &[u8],
    ) -> Result<(), Abort> {
        self.out(env, DEFINE_NATIVE_FONT)?;
        self.four(env, k)?;
        self.bytes(env, def)
    }

    pub(crate) fn set_glyph(
        &mut self,
        env: &mut EngineEnv,
        width: Scaled,
        glyph: u16,
    ) -> Result<(), Abort> {
        self.out(env, SET_GLYPHS)?;
        self.glyph_array(env, width, &[FixedPoint::default()], &[glyph])
    }

    pub(crate) fn set_glyphs(
        &mut self,
        env: &mut EngineEnv,
        width: Scaled,
        locations: &[FixedPoint],
        gids: &[u16],
    ) -> Result<(), Abort> {
        self.out(env, SET_GLYPHS)?;
        self.glyph_array(env, width, locations, gids)
    }

    pub(crate) fn set_text_and_glyphs(
        &mut self,
        env: &mut EngineEnv,
        text: &[u16],
        width: Scaled,
        locations: &[FixedPoint],
        gids: &[u16],
    ) -> Result<(), Abort> {
        self.out(env, SET_TEXT_AND_GLYPHS)?;
        self.two(env, text.len() as u16)?;
        for &unit in text {
            self.two(env, unit)?;
        }
        self.glyph_array(env, width, locations, gids)
    }

    fn glyph_array(
        &mut self,
        env: &mut EngineEnv,
        width: Scaled,
        locations: &[FixedPoint],
        gids: &[u16],
    ) -> Result<(), Abort> {
        self.four(env, width.raw())?;
        self.two(env, gids.len() as u16)?;
        for loc in locations {
            self.four(env, loc.x.raw())?;
            self.four(env, loc.y.raw())?;
        }
        for &gid in gids {
            self.two(env, gid)?;
        }
        Ok(())
    }
    pub(crate) fn special(&mut self, env: &mut EngineEnv, s: &[u16]) -> Result<(), Abort> {
        let len = s.len();
        if len < 256 {
            self.bytes(env, &[XXX1, len as u8])?;
        } else {
            self.out(env, XXX4)?;
            self.four(env, len as i32)?;
        }
        self.text(env, s)
    }

    pub(crate) fn movement(
        &mut self,
        env: &mut EngineEnv,
        axis: Axis,
        w: Scaled,
    ) -> Result<(), Abort> {
        let w = w.raw();
        let len = if w.wrapping_abs() >= 0x800000 {
            4
        } else if w.wrapping_abs() >= 0x8000 {
            3
        } else if w.wrapping_abs() >= 128 {
            2
        } else {
            1
        };
        self.out(env, axis.op1() + len as u8 - 1)?;
        self.bytes(env, &w.to_be_bytes()[4 - len..])
    }

    pub(crate) fn movement_by_reg(
        &mut self,
        env: &mut EngineEnv,
        axis: Axis,
        reg: MoveReg,
    ) -> Result<(), Abort> {
        self.out(env, axis.op1() + reg.delta() - 1)
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
    pub(crate) fn begin_postamble(&mut self, env: &mut EngineEnv) -> Result<(), Abort> {
        self.out(env, POST)?;
        self.four(env, self.last_bop)?;
        self.last_bop = self.pos() - 5;
        self.four(env, NUMERATOR)?;
        self.four(env, DENOMINATOR)
    }

    pub(crate) fn end_postamble(
        &mut self,
        env: &mut EngineEnv,
        mag: i32,
        max_v: Scaled,
        max_h: Scaled,
        total_pages: i32,
    ) -> Result<(), Abort> {
        self.four(env, mag)?;
        self.four(env, max_v.raw())?;
        self.four(env, max_h.raw())?;
        let max_push = self.max_push;
        self.bytes(
            env,
            &[
                (max_push / 256) as u8,
                (max_push % 256) as u8,
                ((total_pages / 256) % 256) as u8,
                (total_pages % 256) as u8,
            ],
        )
    }

    pub(crate) fn finish(&mut self, env: &mut EngineEnv, id: u8) -> Result<(), Abort> {
        self.out(env, POST_POST)?;
        self.four(env, self.last_bop)?;
        self.out(env, id)?;
        let padding = 4 + (DVI_BUF_SIZE - self.ptr) % 4;
        for _ in 0..padding {
            self.out(env, 223)?;
        }
        if self.limit == HALF_BUF {
            self.write_to_file(env, HALF_BUF, DVI_BUF_SIZE - 1)?;
        }
        self.check_length()?;
        if self.ptr > 0 {
            self.write_to_file(env, 0, self.ptr - 1)?;
        }
        Ok(())
    }
}
