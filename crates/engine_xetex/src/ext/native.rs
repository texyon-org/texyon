//! Shaping and measuring native word and glyph nodes.

use crate::errors::Abort;
use crate::fonts::{Side, OTGR_FONT_FLAG};
use crate::glyph_arena::{FixedPoint, GlyphBlob};
use crate::layout::{
    Bidi, FontMappingHandle, K_STATUS_NO_ERROR, K_STATUS_OUTPUT_BUFFER_FULL, UBIDI_MIXED, UBIDI_RTL,
};
use crate::node::{NodePtr, WhatsitNode};
use crate::state::{FontEngineHandle, FontState, MemState};
use crate::units::{FontId, Scaled, StrNumber};
use std::ffi::c_int;
use std::rc::Rc;
use texyon_xetex_layout::{FloatPoint, GlyphBBox};

use super::{d2fix, fix2d, FONT_FLAGS_COLORED, FONT_FLAGS_VERTICAL};

const XDV_FLAG_VERTICAL: u16 = 0x0100;
const XDV_FLAG_COLORED: u16 = 0x0200;
const XDV_FLAG_EXTEND: u16 = 0x1000;
const XDV_FLAG_SLANT: u16 = 0x2000;
const XDV_FLAG_EMBOLDEN: u16 = 0x4000;

#[derive(Default)]
struct ShapedGlyphs {
    locations: Vec<FixedPoint>,
    gids: Vec<u16>,
    advances: Vec<Scaled>,
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
        let (glyphs, advances, positions) = engine.last_layout();

        self.gids.extend(glyphs.iter().map(|&g| g as u16));
        self.locations
            .extend(positions[..n].iter().map(|pos| FixedPoint {
                x: d2fix(f64::from(pos.x) + x),
                y: d2fix(f64::from(pos.y) + y),
            }));
        self.advances
            .extend(advances.iter().map(|&a| d2fix(f64::from(a))));
        positions[n]
    }
}

fn layout_visual_run(bidi: &Bidi, engine: &FontEngineHandle, text: &[u16], run_index: i32) -> i32 {
    let (logical_start, length, dir) = bidi.visual_run(run_index);
    let run = &text[logical_start as usize..(logical_start + length) as usize];
    engine.layout_chars(run, dir == UBIDI_RTL) as i32
}

impl FontState {
    /// The bounds of glyph `gid` of font `f`, measured once per font slot.
    fn glyph_bbox(&self, engine: &FontEngineHandle, f: FontId, gid: u16) -> GlyphBBox {
        *self
            .glyph_bboxes
            .borrow_mut()
            .entry((f, gid))
            .or_insert_with(|| engine.glyph_bounds(u32::from(gid)))
    }
}

impl MemState {
    pub fn get_native_glyph(&self, w: NodePtr<WhatsitNode>, index: u32) -> u16 {
        if index as usize >= w.native_glyph_count(self) as usize {
            0
        } else {
            self.glyph_blobs.get(w.native_glyph_info_handle(self)).gids[index as usize]
        }
    }

    /// The first or last glyph of native word `w`, whose protrusion code applies.
    pub(crate) fn native_word_edge_glyph(
        &self,
        w: NodePtr<WhatsitNode>,
        side: Side,
    ) -> Option<u16> {
        let glyph_count = usize::from(w.native_glyph_count(self));
        if glyph_count == 0 {
            return None;
        }
        let gids = &self.glyph_blobs.get(w.native_glyph_info_handle(self)).gids;
        let gids = gids.get(..glyph_count)?;
        match side {
            Side::Left => gids.first(),
            Side::Right => gids.last(),
        }
        .copied()
    }
}

fn snap_zone(value: &mut Scaled, snap_value: Scaled, fuzz: Scaled) {
    let difference = *value - snap_value;
    if difference <= fuzz && difference >= -fuzz {
        *value = snap_value;
    }
}

impl FontState {
    /// The layout engine of OpenType/Graphite font `f`.
    pub(crate) fn layout_engine(&self, f: FontId) -> Option<&FontEngineHandle> {
        if self.font_area[f] == OTGR_FONT_FLAG {
            self.font_layout_engine[f].as_deref()
        } else {
            None
        }
    }

    /// `layout_engine`, shared, for callers that need the state meanwhile.
    pub(crate) fn shared_layout_engine(&self, f: FontId) -> Option<Rc<FontEngineHandle>> {
        if self.font_area[f] == OTGR_FONT_FLAG {
            self.font_layout_engine[f].clone()
        } else {
            None
        }
    }

    /// The layout engine of OpenType/Graphite font `f`, or `Internal(msg)` for
    /// any other font.
    pub(super) fn require_ot_engine(
        &self,
        f: FontId,
        msg: &'static str,
    ) -> Result<&FontEngineHandle, Abort> {
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
        let f = w.native_font(mem);
        let engine = self.require_ot_engine(f, "bad native font flag in `measure_native_node`")?;

        let bidi = Bidi::new(&text, engine.default_dir());
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
                    x += f64::from(end.x);
                    y += f64::from(end.y);
                }
            }
            x
        } else {
            let n_glyphs = engine.layout_chars(&text, dir == UBIDI_RTL) as i32;
            let end = shaped.append_run(engine, n_glyphs, 0.0, 0.0);
            if n_glyphs > 0 {
                f64::from(end.x)
            } else {
                0.0
            }
        };
        w.set_width(mem, d2fix(width));
        w.set_native_glyph_count(mem, shaped.gids.len() as u16);
        drop(bidi);

        let letter_space = self.font_letter_space[f];
        if letter_space != Scaled::ZERO {
            let mut ls_delta = Scaled::ZERO;
            for (loc, &advance) in shaped.locations.iter_mut().zip(&shaped.advances) {
                if advance == Scaled::ZERO && ls_delta != Scaled::ZERO {
                    ls_delta -= letter_space;
                }
                loc.x += ls_delta;
                ls_delta += letter_space;
            }
            if ls_delta != Scaled::ZERO {
                ls_delta -= letter_space;
                w.set_width(mem, w.width(mem) + ls_delta);
            }
        }

        let glyph_count = w.native_glyph_count(mem) as usize;
        if !use_glyph_metrics || glyph_count == 0 {
            w.set_height(mem, self.native_height(f));
            w.set_depth(mem, self.native_depth(f));
        } else {
            let mut y_min = 65536.0f32;
            let mut y_max = -65536.0f32;
            for (loc, &gid) in shaped.locations[..glyph_count].iter().zip(&shaped.gids) {
                let y = fix2d(-loc.y) as f32;
                let bbox = self.glyph_bbox(engine, f, gid);
                let ht = bbox.y_max;
                let dp = -bbox.y_min;
                if y + ht > y_max {
                    y_max = y + ht;
                }
                if y - dp < y_min {
                    y_min = y - dp;
                }
            }
            w.set_height(mem, d2fix(f64::from(y_max)));
            w.set_depth(mem, -d2fix(f64::from(y_min)));
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

    pub fn get_native_italic_correction(&self, mem: &MemState, w: NodePtr<WhatsitNode>) -> Scaled {
        let f = w.native_font(mem);
        let n = w.native_glyph_count(mem) as usize;
        if n > 0 {
            let last_glyph = mem.glyph_blobs.get(w.native_glyph_info_handle(mem)).gids[n - 1];
            if let Some(engine) = self.layout_engine(f) {
                return d2fix(f64::from(engine.glyph_ital_corr(u32::from(last_glyph))))
                    + self.font_letter_space[f];
            }
        }
        Scaled::ZERO
    }

    pub fn get_native_glyph_italic_correction(
        &self,
        mem: &MemState,
        w: NodePtr<WhatsitNode>,
    ) -> Scaled {
        let gid = w.native_glyph(mem);
        match self.layout_engine(w.native_font(mem)) {
            Some(engine) => d2fix(f64::from(engine.glyph_ital_corr(u32::from(gid)))),
            None => Scaled::ZERO,
        }
    }

    pub fn measure_native_glyph(
        &self,
        mem: &mut MemState,
        w: NodePtr<WhatsitNode>,
        use_glyph_metrics: bool,
    ) -> Result<(), Abort> {
        let gid = u32::from(w.native_glyph(mem));
        let f = w.native_font(mem);
        let engine = self.require_ot_engine(f, "bad native font flag in `measure_native_glyph`")?;
        w.set_width(mem, d2fix(f64::from(engine.font().glyph_width(gid))));
        if use_glyph_metrics {
            let (ht, dp) = engine.glyph_height_depth(gid);
            w.set_height(mem, d2fix(f64::from(ht)));
            w.set_depth(mem, d2fix(f64::from(dp)));
        } else {
            w.set_height(mem, self.native_height(f));
            w.set_depth(mem, self.native_depth(f));
        }
        Ok(())
    }

    /// Height and depth of character `ch` of native font `font`.
    pub fn get_native_char_height_depth(
        &self,
        font: FontId,
        ch: i32,
    ) -> Result<(Scaled, Scaled), Abort> {
        let engine = self.require_ot_engine(
            font,
            "bad native font flag in `get_native_char_height_depth`",
        )?;
        let gid = engine.map_char_to_glyph(ch as u32);
        let (ht, dp) = engine.glyph_height_depth(gid);
        let mut height = d2fix(f64::from(ht));
        let mut depth = d2fix(f64::from(dp));

        // Snap to the baseline, x-height (param 5) and cap height (param 8) when
        // within 4% of the quad (param 6).
        let param = |k: i32| self.param(font, k);
        let fuzz = param(6) / 25;
        snap_zone(&mut depth, Scaled::ZERO, fuzz);
        snap_zone(&mut height, Scaled::ZERO, fuzz);
        snap_zone(&mut height, param(5), fuzz);
        snap_zone(&mut height, param(8), fuzz);
        Ok((height, depth))
    }

    pub fn native_char_height(&self, f: FontId, c: i32) -> Result<Scaled, Abort> {
        Ok(self.get_native_char_height_depth(f, c)?.0)
    }

    pub fn native_char_depth(&self, f: FontId, c: i32) -> Result<Scaled, Abort> {
        Ok(self.get_native_char_height_depth(f, c)?.1)
    }

    /// Left and right side bearings of character `ch` of native font `font`.
    pub fn get_native_char_sidebearings(
        &self,
        font: FontId,
        ch: i32,
    ) -> Result<(Scaled, Scaled), Abort> {
        let engine = self.require_ot_engine(
            font,
            "bad native font flag in `get_native_char_side_bearings`",
        )?;
        let gid = engine.map_char_to_glyph(ch as u32);
        let (l, r) = engine.glyph_sidebearings(gid);
        Ok((d2fix(f64::from(l)), d2fix(f64::from(r))))
    }

    /// Glyph bound `edge` of glyph `gid`: 1 left, 2 top, 3 right, 4 bottom.
    pub fn get_glyph_bounds(&self, font: FontId, edge: i32, gid: i32) -> Result<Scaled, Abort> {
        let engine = self.require_ot_engine(font, "bad native font flag in `get_glyph_bounds`")?;
        let (a, b) = if edge & 1 != 0 {
            engine.glyph_sidebearings(gid as u32)
        } else {
            engine.glyph_height_depth(gid as u32)
        };
        Ok(d2fix(f64::from(if edge <= 2 { a } else { b })))
    }

    pub fn native_char_italic_correction(&self, f: FontId, c: i32) -> Result<Scaled, Abort> {
        let (_, rsb) = self.get_native_char_sidebearings(f, c)?;
        let letter_space = self.font_letter_space[f];
        Ok(if rsb < Scaled::ZERO {
            letter_space - rsb
        } else {
            letter_space
        })
    }

    pub fn native_char_width(&self, f: FontId, c: i32) -> Result<Scaled, Abort> {
        let engine = self.require_ot_engine(f, "bad native font flag in `get_native_char_wd`")?;
        let gid = engine.map_char_to_glyph(c as u32);
        Ok(d2fix(f64::from(engine.glyph_width(gid))))
    }

    pub fn map_char_to_glyph(&self, font: FontId, ch: i32) -> Result<i32, Abort> {
        if ch > 0x10ffff || (0xd800..=0xdfff).contains(&ch) {
            return Ok(0);
        }
        let engine = self.require_ot_engine(font, "bad native font flag in `map_char_to_glyph`")?;
        Ok(engine.map_char_to_glyph(ch as u32) as i32)
    }

    pub fn map_glyph_to_index(&self, font: FontId, name: &[u8]) -> Result<i32, Abort> {
        let engine =
            self.require_ot_engine(font, "bad native font flag in `map_glyph_to_index`")?;
        Ok(engine.map_glyph_to_index(name) as i32)
    }

    pub fn get_font_char_range(&self, font: FontId, first: i32) -> Result<i32, Abort> {
        let engine =
            self.require_ot_engine(font, "bad native font flag in `get_font_char_range'`")?;
        Ok(engine.font_char_range(first != 0) as i32)
    }

    pub(crate) fn release_linebreak_state(&mut self) {
        self.brk_iter = None;
        self.brk_locale_str_num = StrNumber::NONE;
        self.saved_mapping_name = None;
    }

    /// The body of the XDV native font definition of `f`.
    pub fn make_font_def(&self, f: FontId) -> Result<Vec<u8>, Abort> {
        let engine = self.require_ot_engine(f, "bad native font flag in `make_font_def`")?;
        let (name, index) = (engine.font().filename(), engine.font().index());

        let rgba = engine.rgb();
        let font_flags = self.font_flags[f];
        let mut flags: u16 = 0;
        if font_flags & FONT_FLAGS_VERTICAL != 0 {
            flags |= XDV_FLAG_VERTICAL;
        }
        let extend = engine.extend();
        let slant = engine.slant();
        let embolden = engine.embolden();
        let size = d2fix(f64::from(engine.font().point_size()));

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
        def.extend_from_slice(&(size.raw() as u32).to_be_bytes());
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
                def.extend_from_slice(&(d2fix(f64::from(v)).raw() as u32).to_be_bytes());
            }
        }

        Ok(def)
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
        let f = w.native_font(mem);
        let saved_width = w.width(mem);
        self.measure_native_node(mem, w, false)?;

        let width = w.width(mem);
        if width == saved_width {
            return Ok(());
        }
        let just_amount = fix2d(saved_width - width);
        let glyph_count = w.native_glyph_count(mem) as usize;
        let handle = w.native_glyph_info_handle(mem);
        let glyphs = mem.glyph_blobs.get_mut(handle);

        let space_glyph = self.map_char_to_glyph(f, i32::from(b' '))?;
        let is_space = |gid: u16| i32::from(gid) == space_glyph;
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
                    adjustment = just_amount * f64::from(space_index) / f64::from(space_count);
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

        w.set_width(mem, saved_width);
        Ok(())
    }
}
