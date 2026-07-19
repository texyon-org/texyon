use crate::engine::LayoutEngine;
use crate::font::Font;
use std::collections::BTreeMap;
use std::sync::Mutex;
#[cfg(not(target_os = "macos"))]
use tectonic_bridge_fontconfig as fc;

mod engine;
mod font;
mod manager;

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(C)]
pub struct FloatPoint {
    pub x: f32,
    pub y: f32,
}

/// cbindgen:rename-all=camelCase
#[derive(Copy, Clone, Default, PartialEq, Debug)]
#[repr(C)]
pub struct GlyphBBox {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

#[cfg(not(target_os = "macos"))]
pub type Fixed = i32;
/// cbindgen:ignore
#[cfg(target_os = "macos")]
pub type Fixed = u32;
pub type OTTag = u32;
pub type GlyphID = u16;
/// cbindgen:ignore
pub type XeTeXFont = *mut Font;
/// cbindgen:ignore
pub type XeTeXLayoutEngine = *mut LayoutEngine;
#[cfg(not(target_os = "macos"))]
pub(crate) type RawPlatformFontRef = *mut fc::sys::FcPattern;
#[cfg(target_os = "macos")]
pub(crate) type RawPlatformFontRef = tectonic_mac_core::sys::CTFontDescriptorRef;
/// cbindgen:ignore
#[cfg(not(target_os = "macos"))]
pub(crate) type PlatformFontRef = fc::Pattern;
/// cbindgen:ignore
#[cfg(target_os = "macos")]
pub(crate) type PlatformFontRef = tectonic_mac_core::CTFontDescriptor;

/// key is combined value representing `(font_id, glyph)`
/// value is glyph bounding box in TeX points
static GLYPH_BOXES: Mutex<BTreeMap<(u16, u16), GlyphBBox>> = Mutex::new(BTreeMap::new());

#[no_mangle]
pub unsafe extern "C" fn getCachedGlyphBBox(
    font_id: u16,
    glyph_id: u16,
    bbox: *mut GlyphBBox,
) -> i32 {
    match GLYPH_BOXES.lock().unwrap().get(&(font_id, glyph_id)) {
        Some(val) => {
            *bbox = *val;
            1
        }
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn cacheGlyphBBox(font_id: u16, glyph_id: u16, bbox: *const GlyphBBox) {
    GLYPH_BOXES
        .lock()
        .unwrap()
        .insert((font_id, glyph_id), *bbox);
}

/* The following code used to be in a file called "hz.cpp" and there's no
 * particular reason for it to be here, but it was a tiny file with a weird
 * name so I wanted to get rid of it. The functions are invoked from the C
 * code. */

pub const LEFT_SIDE: i32 = 0;
pub const RIGHT_SIDE: i32 = 1;

static LEFT_PROT: Mutex<BTreeMap<(i32, u32), i32>> = Mutex::new(BTreeMap::new());
static RIGHT_PROT: Mutex<BTreeMap<(i32, u32), i32>> = Mutex::new(BTreeMap::new());

/// Clear process-global tables keyed by XeTeX font slot. Slot numbers are
/// meaningful only within one document operation, so these values must not
/// leak into a subsequent resident run.
#[no_mangle]
pub extern "C" fn xetex_layout_reset_font_caches() {
    GLYPH_BOXES.lock().unwrap().clear();
    LEFT_PROT.lock().unwrap().clear();
    RIGHT_PROT.lock().unwrap().clear();
}

#[no_mangle]
pub extern "C" fn set_cp_code(font_num: i32, code: u32, side: i32, value: i32) {
    match side {
        LEFT_SIDE => LEFT_PROT.lock().unwrap().insert((font_num, code), value),
        RIGHT_SIDE => RIGHT_PROT.lock().unwrap().insert((font_num, code), value),
        _ => unreachable!(),
    };
}

#[no_mangle]
pub extern "C" fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32 {
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
        // SAFETY: `bbox` is a valid readable value for the duration of the call.
        unsafe { cacheGlyphBBox(7, 11, &bbox) };
        set_cp_code(7, 11, LEFT_SIDE, 12);
        set_cp_code(7, 11, RIGHT_SIDE, 13);

        xetex_layout_reset_font_caches();

        let mut out = GlyphBBox::default();
        // SAFETY: `out` is a valid writable value for the duration of the call.
        assert_eq!(unsafe { getCachedGlyphBBox(7, 11, &mut out) }, 0);
        assert_eq!(get_cp_code(7, 11, LEFT_SIDE), 0);
        assert_eq!(get_cp_code(7, 11, RIGHT_SIDE), 0);
    }
}
