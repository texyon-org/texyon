// Copyright 2020-2021 the Tectonic Project
// Licensed under the MIT License.

//! XeTeX's font loading and layout code.
//!
//! [Tectonic]: https://tectonic-typesetting.github.io/

use std::collections::BTreeMap;
use std::sync::Mutex;
#[cfg(not(target_os = "macos"))]
use tectonic_bridge_fontconfig as fc;

pub mod engine;
pub mod font;
pub mod manager;
mod utils;

pub use engine::{EngineOptions, GraphiteBreaks, LayoutEngine};
pub use font::Font;
pub use manager::{Engine, FontManager};

/// A point in TeX points.
#[derive(Copy, Clone, Default, PartialEq, Debug)]
pub struct FloatPoint {
    /// Horizontal coordinate.
    pub x: f32,
    /// Vertical coordinate.
    pub y: f32,
}

/// A glyph bounding box in TeX points.
#[derive(Copy, Clone, Default, PartialEq, Debug)]
pub struct GlyphBBox {
    /// Left edge.
    pub x_min: f32,
    /// Bottom edge.
    pub y_min: f32,
    /// Right edge.
    pub x_max: f32,
    /// Top edge.
    pub y_max: f32,
}

/// A 16.16 fixed-point number.
#[cfg(not(target_os = "macos"))]
pub type Fixed = i32;
/// A 16.16 fixed-point number.
#[cfg(target_os = "macos")]
pub type Fixed = u32;
/// An OpenType tag.
pub type OTTag = u32;
/// A glyph index.
pub type GlyphID = u16;

/// A font reference from the platform's font manager.
#[cfg(not(target_os = "macos"))]
pub type PlatformFontRef = fc::Pattern;
/// A font reference from the platform's font manager.
#[cfg(target_os = "macos")]
pub type PlatformFontRef = tectonic_mac_core::CTFontDescriptor;

/// Glyph bounding boxes in TeX points, keyed by `(font_id, glyph)`.
static GLYPH_BOXES: Mutex<BTreeMap<(u16, u16), GlyphBBox>> = Mutex::new(BTreeMap::new());

/// The bounds cached for `glyph_id` of font `font_id`.
pub fn cached_glyph_bbox(font_id: u16, glyph_id: u16) -> Option<GlyphBBox> {
    GLYPH_BOXES
        .lock()
        .unwrap()
        .get(&(font_id, glyph_id))
        .copied()
}

/// Caches the bounds of `glyph_id` of font `font_id`.
pub fn cache_glyph_bbox(font_id: u16, glyph_id: u16, bbox: GlyphBBox) {
    GLYPH_BOXES
        .lock()
        .unwrap()
        .insert((font_id, glyph_id), bbox);
}

/// The left side of a glyph, for protrusion codes.
pub const LEFT_SIDE: i32 = 0;
/// The right side of a glyph, for protrusion codes.
pub const RIGHT_SIDE: i32 = 1;

static LEFT_PROT: Mutex<BTreeMap<(i32, u32), i32>> = Mutex::new(BTreeMap::new());
static RIGHT_PROT: Mutex<BTreeMap<(i32, u32), i32>> = Mutex::new(BTreeMap::new());

/// Clears the tables keyed by XeTeX font slot. Slot numbers mean something
/// only within one document operation, so values must not leak into the next.
pub fn reset_font_caches() {
    GLYPH_BOXES.lock().unwrap().clear();
    LEFT_PROT.lock().unwrap().clear();
    RIGHT_PROT.lock().unwrap().clear();
}

/// Sets the character protrusion code of `code` in font `font_num`.
pub fn set_cp_code(font_num: i32, code: u32, side: i32, value: i32) {
    match side {
        LEFT_SIDE => LEFT_PROT.lock().unwrap().insert((font_num, code), value),
        RIGHT_SIDE => RIGHT_PROT.lock().unwrap().insert((font_num, code), value),
        _ => unreachable!(),
    };
}

/// The character protrusion code of `code` in font `font_num`, 0 when unset.
pub fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32 {
    match side {
        LEFT_SIDE => LEFT_PROT.lock().unwrap().get(&(font_num, code)).copied(),
        RIGHT_SIDE => RIGHT_PROT.lock().unwrap().get(&(font_num, code)).copied(),
        _ => unreachable!(),
    }
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_clears_every_font_slot_table() {
        let bbox = GlyphBBox {
            x_min: 1.0,
            y_min: 2.0,
            x_max: 3.0,
            y_max: 4.0,
        };
        cache_glyph_bbox(7, 11, bbox);
        set_cp_code(7, 11, LEFT_SIDE, 12);
        set_cp_code(7, 11, RIGHT_SIDE, 13);

        reset_font_caches();

        assert_eq!(cached_glyph_bbox(7, 11), None);
        assert_eq!(get_cp_code(7, 11, LEFT_SIDE), 0);
        assert_eq!(get_cp_code(7, 11, RIGHT_SIDE), 0);
    }
}

/// Does our resulting executable link correctly?
#[test]
fn linkage() {}
