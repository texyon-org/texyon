//! OpenType MATH table queries, translated from xetex-XeTeXOTMath.cpp.

use libc::c_void;
use tectonic_bridge_harfbuzz::sys as hb;

use crate::ext::{D2Fix, Fix2D};
use crate::state::FontState;

const OTGR_FONT_FLAG: i32 = 0xFFFE;

type XeTeXFont = *mut c_void;
type XeTeXLayoutEngine = *mut c_void;

extern "C" {
    fn getFont(engine: XeTeXLayoutEngine) -> XeTeXFont;
    fn getGlyphHeightDepth(
        engine: XeTeXLayoutEngine,
        glyph_id: u32,
        height: *mut f32,
        depth: *mut f32,
    );
    fn ttxl_get_hb_font(engine: XeTeXLayoutEngine) -> *mut hb::hb_font_t;
    fn ttxl_font_units_to_points(font: XeTeXFont, units: f32) -> f32;
    fn ttxl_font_points_to_units(font: XeTeXFont, points: f32) -> f32;
    fn ttxl_font_get_point_size(font: XeTeXFont) -> f32;
}

const HB_OT_MATH_CONSTANT_SCRIPT_PERCENT_SCALE_DOWN: i32 = 0;
const HB_OT_MATH_CONSTANT_SCRIPT_SCRIPT_PERCENT_SCALE_DOWN: i32 = 1;
const HB_OT_MATH_CONSTANT_RADICAL_DEGREE_BOTTOM_RAISE_PERCENT: i32 = 55;

const MATH_QUAD: i32 = 6;
const DELIM1: i32 = 20;
const DELIM2: i32 = 21;

/// TeX math-symbol fontdimen (family 2) to OpenType MATH constant.
const TEX_SYM_TO_OT_MAP: [Option<i32>; 23] = [
    None,
    None,
    None,
    None,
    None,
    Some(6), // x-height: accentBaseHeight
    None,    // quad
    None,
    Some(33), // num1: fractionNumeratorDisplayStyleShiftUp
    Some(32), // num2: fractionNumeratorShiftUp
    Some(22), // num3: stackTopShiftUp
    Some(35), // denom1: fractionDenominatorDisplayStyleShiftDown
    Some(34), // denom2: fractionDenominatorShiftDown
    Some(11), // sup1: superscriptShiftUp
    Some(11), // sup2: superscriptShiftUp
    Some(12), // sup3: superscriptShiftUpCramped
    Some(8),  // sub1: subscriptShiftDown
    Some(8),  // sub2: subscriptShiftDown
    Some(14), // sup_drop: superscriptBaselineDropMax
    Some(10), // sub_drop: subscriptBaselineDropMin
    Some(2),  // delim1: delimitedSubFormulaMinHeight
    None,     // delim2: computed from quad
    Some(5),  // axis_height: axisHeight
];

/// TeX math-extension fontdimen (family 3) to OpenType MATH constant.
const TEX_EXT_TO_OT_MAP: [Option<i32>; 14] = [
    None,
    None,
    None,
    None,
    None,
    Some(6), // x-height: accentBaseHeight
    None,    // quad
    None,
    Some(38), // default_rule_thickness: fractionRuleThickness
    Some(18), // big_op_spacing1: upperLimitGapMin
    Some(20), // big_op_spacing2: lowerLimitGapMin
    Some(19), // big_op_spacing3: upperLimitBaselineRiseMin
    Some(21), // big_op_spacing4: lowerLimitBaselineDropMin
    Some(26), // big_op_spacing5: stackGapMin
];

/// The layout engine of `f` when it is an OpenType (HarfBuzz) font.
fn ot_engine(fonts: &FontState, f: i32) -> Option<XeTeXLayoutEngine> {
    (fonts.font_area_entry(f) == OTGR_FONT_FLAG).then(|| fonts.font_layout_engine_ptr(f))
}

fn units_to_fix(font: XeTeXFont, units: i32) -> i32 {
    unsafe { D2Fix(ttxl_font_units_to_points(font, units as f32) as f64) }
}

fn direction(horiz: bool) -> hb::hb_direction_t {
    if horiz {
        hb::hb_direction_t::Rtl
    } else {
        hb::hb_direction_t::Ttb
    }
}

// translated from xetex-XeTeXOTMath.cpp:47 (get_ot_math_constant)
pub(crate) fn get_ot_math_constant(fonts: &FontState, f: i32, n: i32) -> i32 {
    let Some(engine) = ot_engine(fonts, f) else {
        return 0;
    };
    unsafe {
        let font = getFont(engine);
        let rval = hb::hb_ot_math_get_constant(ttxl_get_hb_font(engine), n);
        match n {
            HB_OT_MATH_CONSTANT_SCRIPT_PERCENT_SCALE_DOWN
            | HB_OT_MATH_CONSTANT_SCRIPT_SCRIPT_PERCENT_SCALE_DOWN
            | HB_OT_MATH_CONSTANT_RADICAL_DEGREE_BOTTOM_RAISE_PERCENT => rval,
            _ => units_to_fix(font, rval),
        }
    }
}

fn mapped_constant(fonts: &FontState, f: i32, map: &[Option<i32>], n: i32) -> i32 {
    match usize::try_from(n).ok().and_then(|i| map.get(i)) {
        Some(Some(ot_index)) => get_ot_math_constant(fonts, f, *ot_index),
        _ => 0,
    }
}

// translated from xetex-XeTeXOTMath.cpp:120 (get_native_mathsy_param)
pub(crate) fn get_native_mathsy_param(fonts: &FontState, f: i32, n: i32) -> i32 {
    if n == MATH_QUAD {
        fonts.font_size[f as usize]
    } else if n == DELIM2 {
        // No OT parameter fits, so XeTeX uses 1.5em clamped to the delim1 height.
        ((1.5 * fonts.font_size[f as usize] as f64) as i32)
            .min(get_native_mathsy_param(fonts, f, DELIM1))
    } else {
        mapped_constant(fonts, f, &TEX_SYM_TO_OT_MAP, n)
    }
}

// translated from xetex-XeTeXOTMath.cpp:169 (get_native_mathex_param)
pub(crate) fn get_native_mathex_param(fonts: &FontState, f: i32, n: i32) -> i32 {
    if n == MATH_QUAD {
        fonts.font_size[f as usize]
    } else {
        mapped_constant(fonts, f, &TEX_EXT_TO_OT_MAP, n)
    }
}

// translated from xetex-XeTeXOTMath.cpp:188 (get_ot_math_variant)
pub(crate) fn get_ot_math_variant(
    fonts: &FontState,
    f: i32,
    g: i32,
    v: i32,
    adv: &mut i32,
    horiz: bool,
) -> i32 {
    *adv = -1;
    let Some(engine) = ot_engine(fonts, f) else {
        return g;
    };
    unsafe {
        let mut variant = hb::hb_ot_math_glyph_variant_t {
            glyph: 0,
            advance: 0,
        };
        let mut count = 1;
        hb::hb_ot_math_get_glyph_variants(
            ttxl_get_hb_font(engine),
            g as u32,
            direction(horiz),
            v as u32,
            &mut count,
            &mut variant,
        );
        if count == 0 {
            return g;
        }
        *adv = units_to_fix(getFont(engine), variant.advance);
        variant.glyph as i32
    }
}

/// The parts HarfBuzz reports for building a stretched glyph.
pub(crate) struct GlyphAssembly {
    parts: Vec<hb::hb_ot_math_glyph_part_t>,
}

// translated from xetex-XeTeXOTMath.cpp:211 (get_ot_assembly_ptr)
pub(crate) fn get_ot_assembly(
    fonts: &FontState,
    f: i32,
    g: i32,
    horiz: bool,
) -> Option<GlyphAssembly> {
    let engine = ot_engine(fonts, f)?;
    unsafe {
        let hb_font = ttxl_get_hb_font(engine);
        let count = hb::hb_ot_math_get_glyph_assembly(
            hb_font,
            g as u32,
            direction(horiz),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if count == 0 {
            return None;
        }
        let mut parts = vec![hb::hb_ot_math_glyph_part_t::default(); count as usize];
        let mut filled = count;
        hb::hb_ot_math_get_glyph_assembly(
            hb_font,
            g as u32,
            direction(horiz),
            0,
            &mut filled,
            parts.as_mut_ptr(),
            std::ptr::null_mut(),
        );
        parts.truncate(filled as usize);
        Some(GlyphAssembly { parts })
    }
}

impl GlyphAssembly {
    // translated from xetex-XeTeXOTMath.cpp:472 (ot_part_count)
    pub(crate) fn count(&self) -> i32 {
        self.parts.len() as i32
    }

    // translated from xetex-XeTeXOTMath.cpp:478 (ot_part_glyph)
    pub(crate) fn glyph(&self, i: i32) -> i32 {
        self.parts[i as usize].glyph as i32
    }

    // translated from xetex-XeTeXOTMath.cpp:484 (ot_part_is_extender)
    pub(crate) fn is_extender(&self, i: i32) -> bool {
        self.parts[i as usize].flags & hb::HB_OT_MATH_GLYPH_PART_FLAG_EXTENDER != 0
    }

    fn part_length(&self, fonts: &FontState, f: i32, units: i32) -> i32 {
        match ot_engine(fonts, f) {
            Some(engine) => units_to_fix(unsafe { getFont(engine) }, units),
            None => 0,
        }
    }

    // translated from xetex-XeTeXOTMath.cpp:490 (ot_part_start_connector)
    pub(crate) fn start_connector(&self, fonts: &FontState, f: i32, i: i32) -> i32 {
        self.part_length(fonts, f, self.parts[i as usize].start_connector_length)
    }

    // translated from xetex-XeTeXOTMath.cpp:503 (ot_part_end_connector)
    pub(crate) fn end_connector(&self, fonts: &FontState, f: i32, i: i32) -> i32 {
        self.part_length(fonts, f, self.parts[i as usize].end_connector_length)
    }

    // translated from xetex-XeTeXOTMath.cpp:516 (ot_part_full_advance)
    pub(crate) fn full_advance(&self, fonts: &FontState, f: i32, i: i32) -> i32 {
        self.part_length(fonts, f, self.parts[i as usize].full_advance)
    }
}

// translated from xetex-XeTeXOTMath.cpp:248 (get_ot_math_ital_corr)
pub(crate) fn get_ot_math_ital_corr(fonts: &FontState, f: i32, g: i32) -> i32 {
    let Some(engine) = ot_engine(fonts, f) else {
        return 0;
    };
    unsafe {
        let rval = hb::hb_ot_math_get_glyph_italics_correction(ttxl_get_hb_font(engine), g as u32);
        units_to_fix(getFont(engine), rval)
    }
}

// translated from xetex-XeTeXOTMath.cpp:263 (get_ot_math_accent_pos)
pub(crate) fn get_ot_math_accent_pos(fonts: &FontState, f: i32, g: i32) -> i32 {
    let Some(engine) = ot_engine(fonts, f) else {
        return 0x7fff_ffff;
    };
    unsafe {
        let rval =
            hb::hb_ot_math_get_glyph_top_accent_attachment(ttxl_get_hb_font(engine), g as u32);
        units_to_fix(getFont(engine), rval)
    }
}

// translated from xetex-XeTeXOTMath.cpp:278 (ot_min_connector_overlap)
pub(crate) fn ot_min_connector_overlap(fonts: &FontState, f: i32) -> i32 {
    let Some(engine) = ot_engine(fonts, f) else {
        return 0;
    };
    unsafe {
        let rval = hb::hb_ot_math_get_min_connector_overlap(
            ttxl_get_hb_font(engine),
            hb::hb_direction_t::Rtl,
        );
        units_to_fix(getFont(engine), rval)
    }
}

// translated from xetex-XeTeXOTMath.cpp:293 (getMathKernAt)
fn math_kern_at(
    fonts: &FontState,
    f: i32,
    g: i32,
    height: i32,
    side: hb::hb_ot_math_kern_t,
) -> i32 {
    let Some(engine) = ot_engine(fonts, f) else {
        return 0;
    };
    unsafe { hb::hb_ot_math_get_glyph_kerning(ttxl_get_hb_font(engine), g as u32, side, height) }
}

// translated from xetex-XeTeXOTMath.cpp:306 (glyph_height) and :319 (glyph_depth)
fn glyph_height_depth(fonts: &FontState, f: i32, g: i32) -> (f32, f32) {
    let (mut height, mut depth) = (0.0, 0.0);
    if let Some(engine) = ot_engine(fonts, f) {
        unsafe { getGlyphHeightDepth(engine, g as u32, &mut height, &mut depth) };
    }
    (height, depth)
}

pub(crate) const SUP_CMD: i32 = 0;
pub(crate) const SUB_CMD: i32 = 1;

// translated from xetex-XeTeXOTMath.cpp:336 (get_ot_math_kern)
pub(crate) fn get_ot_math_kern(
    fonts: &FontState,
    f: i32,
    g: i32,
    sf: i32,
    sg: i32,
    cmd: i32,
    shift_scaled: i32,
) -> i32 {
    let (Some(engine), Some(sengine)) = (ot_engine(fonts, f), ot_engine(fonts, sf)) else {
        return 0;
    };
    // Kerns are computed in base-glyph units, with the float/int conversions of the C++.
    unsafe {
        let font = getFont(engine);
        let sfont = getFont(sengine);

        let (g_h, g_d) = glyph_height_depth(fonts, f, g);
        let (sg_h, sg_d) = glyph_height_depth(fonts, sf, sg);
        let g_height = ttxl_font_points_to_units(font, g_h) as i32;
        let g_depth = ttxl_font_points_to_units(font, g_d) as i32;
        let sg_height = ttxl_font_points_to_units(sfont, sg_h) as i32;
        let sg_depth = ttxl_font_points_to_units(sfont, sg_d) as i32;

        let shift = ttxl_font_points_to_units(font, Fix2D(shift_scaled) as f32) as i32;

        let f_size = ttxl_font_get_point_size(font);
        let sf_size = ttxl_font_get_point_size(sfont);
        assert!(f_size != 0.0);
        let scale = sf_size / f_size;

        let scaled_sum = |kern: i32, skern: i32| (kern as f32 + scale * skern as f32) as i32;

        let rval = match cmd {
            SUP_CMD => {
                let kern = math_kern_at(
                    fonts,
                    f,
                    g,
                    (shift as f32 - scale * sg_depth as f32) as i32,
                    hb::HB_OT_MATH_KERN_TOP_RIGHT,
                );
                let skern = math_kern_at(fonts, sf, sg, -sg_depth, hb::HB_OT_MATH_KERN_BOTTOM_LEFT);
                let top_kern = scaled_sum(kern, skern);

                let kern = math_kern_at(fonts, f, g, g_height, hb::HB_OT_MATH_KERN_TOP_RIGHT);
                let skern = math_kern_at(
                    fonts,
                    sf,
                    sg,
                    ((g_height - shift) as f32 / scale) as i32,
                    hb::HB_OT_MATH_KERN_BOTTOM_LEFT,
                );
                let bot_kern = scaled_sum(kern, skern);

                // The spec says minimum, but the maximum gives the greater separation.
                top_kern.max(bot_kern)
            }
            SUB_CMD => {
                let kern = math_kern_at(
                    fonts,
                    f,
                    g,
                    (scale * sg_height as f32 - shift as f32) as i32,
                    hb::HB_OT_MATH_KERN_BOTTOM_RIGHT,
                );
                let skern = math_kern_at(fonts, sf, sg, sg_height, hb::HB_OT_MATH_KERN_TOP_LEFT);
                let top_kern = scaled_sum(kern, skern);

                let kern = math_kern_at(fonts, f, g, -g_depth, hb::HB_OT_MATH_KERN_BOTTOM_RIGHT);
                let skern = math_kern_at(
                    fonts,
                    sf,
                    sg,
                    ((shift - g_depth) as f32 / scale) as i32,
                    hb::HB_OT_MATH_KERN_TOP_LEFT,
                );
                let bot_kern = scaled_sum(kern, skern);

                top_kern.max(bot_kern)
            }
            _ => unreachable!("get_ot_math_kern: cmd is neither sup nor sub"),
        };

        units_to_fix(font, rval)
    }
}
