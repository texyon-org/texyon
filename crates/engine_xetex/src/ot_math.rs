//! OpenType MATH table queries.

use crate::units::{FontId, Scaled};
use tectonic_bridge_harfbuzz::sys as hb;

use crate::ext::{d2fix, fix2d};
use crate::state::FontEngineHandle;
use crate::state::FontState;
use tectonic_xetex_layout::Font;

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

fn units_to_fix(font: &Font, units: i32) -> Scaled {
    d2fix(f64::from(
        font.units_to_points(f64::from(units as f32)) as f32
    ))
}

fn points_to_units(font: &Font, points: f32) -> f32 {
    font.points_to_units(f64::from(points)) as f32
}

fn direction(horiz: bool) -> hb::hb_direction_t {
    if horiz {
        hb::hb_direction_t::Rtl
    } else {
        hb::hb_direction_t::Ttb
    }
}

/// The parts HarfBuzz reports for building a stretched glyph.
pub(crate) struct GlyphAssembly {
    parts: Vec<hb::hb_ot_math_glyph_part_t>,
}

impl GlyphAssembly {
    pub(crate) fn count(&self) -> i32 {
        self.parts.len() as i32
    }

    pub(crate) fn glyph(&self, i: i32) -> i32 {
        self.parts[i as usize].glyph as i32
    }

    pub(crate) fn is_extender(&self, i: i32) -> bool {
        self.parts[i as usize].flags & hb::HB_OT_MATH_GLYPH_PART_FLAG_EXTENDER != 0
    }

    fn part_length(&self, fonts: &FontState, f: FontId, units: i32) -> Scaled {
        match fonts.ot_engine(f) {
            Some(engine) => units_to_fix(engine.font(), units),
            None => Scaled::ZERO,
        }
    }

    pub(crate) fn start_connector(&self, fonts: &FontState, f: FontId, i: i32) -> Scaled {
        self.part_length(fonts, f, self.parts[i as usize].start_connector_length)
    }

    pub(crate) fn end_connector(&self, fonts: &FontState, f: FontId, i: i32) -> Scaled {
        self.part_length(fonts, f, self.parts[i as usize].end_connector_length)
    }

    pub(crate) fn full_advance(&self, fonts: &FontState, f: FontId, i: i32) -> Scaled {
        self.part_length(fonts, f, self.parts[i as usize].full_advance)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ScriptKind {
    Sup,
    Sub,
}

impl FontState {
    /// The layout engine of `f` when it is an OpenType (HarfBuzz) font.
    fn ot_engine(&self, f: FontId) -> Option<&FontEngineHandle> {
        self.layout_engine(f)
    }

    /// OpenType MATH constant `n`, scaled except for the percentages.
    pub(crate) fn get_ot_math_constant(&self, f: FontId, n: i32) -> Scaled {
        let Some(engine) = self.ot_engine(f) else {
            return Scaled::ZERO;
        };
        // SAFETY: the engine keeps its HarfBuzz font alive.
        let rval = unsafe { hb::hb_ot_math_get_constant(engine.raw_hb_font(), n) };
        match n {
            HB_OT_MATH_CONSTANT_SCRIPT_PERCENT_SCALE_DOWN
            | HB_OT_MATH_CONSTANT_SCRIPT_SCRIPT_PERCENT_SCALE_DOWN
            | HB_OT_MATH_CONSTANT_RADICAL_DEGREE_BOTTOM_RAISE_PERCENT => Scaled(rval),
            _ => units_to_fix(engine.font(), rval),
        }
    }

    fn mapped_constant(&self, f: FontId, map: &[Option<i32>], n: i32) -> Scaled {
        match usize::try_from(n).ok().and_then(|i| map.get(i)) {
            Some(Some(ot_index)) => self.get_ot_math_constant(f, *ot_index),
            _ => Scaled::ZERO,
        }
    }

    pub(crate) fn get_native_mathsy_param(&self, f: FontId, n: i32) -> Scaled {
        if n == MATH_QUAD {
            self.font_size[f]
        } else if n == DELIM2 {
            // No OT parameter fits, so XeTeX uses 1.5em clamped to the delim1 height.
            Scaled((1.5 * f64::from(self.font_size[f].0)) as i32)
                .min(self.get_native_mathsy_param(f, DELIM1))
        } else {
            self.mapped_constant(f, &TEX_SYM_TO_OT_MAP, n)
        }
    }

    pub(crate) fn get_native_mathex_param(&self, f: FontId, n: i32) -> Scaled {
        if n == MATH_QUAD {
            self.font_size[f]
        } else {
            self.mapped_constant(f, &TEX_EXT_TO_OT_MAP, n)
        }
    }

    pub(crate) fn get_ot_math_variant(
        &self,
        f: FontId,
        g: i32,
        v: i32,
        adv: &mut Scaled,
        horiz: bool,
    ) -> i32 {
        *adv = Scaled(-1);
        let Some(engine) = self.ot_engine(f) else {
            return g;
        };
        let mut variant = hb::hb_ot_math_glyph_variant_t {
            glyph: 0,
            advance: 0,
        };
        let mut count = 1;
        // SAFETY: the engine keeps its HarfBuzz font alive, and `count` and
        // `variant` hold the one variant asked for.
        unsafe {
            hb::hb_ot_math_get_glyph_variants(
                engine.raw_hb_font(),
                g as u32,
                direction(horiz),
                v as u32,
                &mut count,
                &mut variant,
            )
        };
        if count == 0 {
            return g;
        }
        *adv = units_to_fix(engine.font(), variant.advance);
        variant.glyph as i32
    }

    pub(crate) fn get_ot_assembly(&self, f: FontId, g: i32, horiz: bool) -> Option<GlyphAssembly> {
        let hb_font = self.ot_engine(f)?.raw_hb_font();
        // SAFETY: the engine keeps its HarfBuzz font alive, and null outputs
        // only ask for the part count.
        let count = unsafe {
            hb::hb_ot_math_get_glyph_assembly(
                hb_font,
                g as u32,
                direction(horiz),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if count == 0 {
            return None;
        }
        let mut parts = vec![hb::hb_ot_math_glyph_part_t::default(); count as usize];
        let mut filled = count;
        // SAFETY: as above, and `parts` holds the `filled` parts asked for.
        unsafe {
            hb::hb_ot_math_get_glyph_assembly(
                hb_font,
                g as u32,
                direction(horiz),
                0,
                &mut filled,
                parts.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };
        parts.truncate(filled as usize);
        Some(GlyphAssembly { parts })
    }

    pub(crate) fn get_ot_math_ital_corr(&self, f: FontId, g: i32) -> Scaled {
        let Some(engine) = self.ot_engine(f) else {
            return Scaled::ZERO;
        };
        // SAFETY: the engine keeps its HarfBuzz font alive.
        let rval =
            unsafe { hb::hb_ot_math_get_glyph_italics_correction(engine.raw_hb_font(), g as u32) };
        units_to_fix(engine.font(), rval)
    }

    pub(crate) fn get_ot_math_accent_pos(&self, f: FontId, g: i32) -> Scaled {
        let Some(engine) = self.ot_engine(f) else {
            return crate::consts::INFINITE_DIMEN;
        };
        // SAFETY: the engine keeps its HarfBuzz font alive.
        let rval = unsafe {
            hb::hb_ot_math_get_glyph_top_accent_attachment(engine.raw_hb_font(), g as u32)
        };
        units_to_fix(engine.font(), rval)
    }

    pub(crate) fn ot_min_connector_overlap(&self, f: FontId) -> Scaled {
        let Some(engine) = self.ot_engine(f) else {
            return Scaled::ZERO;
        };
        // SAFETY: the engine keeps its HarfBuzz font alive.
        let rval = unsafe {
            hb::hb_ot_math_get_min_connector_overlap(engine.raw_hb_font(), hb::hb_direction_t::Rtl)
        };
        units_to_fix(engine.font(), rval)
    }

    fn math_kern_at(&self, f: FontId, g: i32, height: i32, side: hb::hb_ot_math_kern_t) -> i32 {
        let Some(engine) = self.ot_engine(f) else {
            return 0;
        };
        // SAFETY: the engine keeps its HarfBuzz font alive.
        unsafe { hb::hb_ot_math_get_glyph_kerning(engine.raw_hb_font(), g as u32, side, height) }
    }

    fn glyph_height_depth(&self, f: FontId, g: i32) -> (f32, f32) {
        self.ot_engine(f)
            .map_or((0.0, 0.0), |engine| engine.glyph_height_depth(g as u32))
    }

    pub(crate) fn get_ot_math_kern(
        &self,
        f: FontId,
        g: i32,
        sf: FontId,
        sg: i32,
        kind: ScriptKind,
        shift_scaled: Scaled,
    ) -> Scaled {
        let (Some(engine), Some(sengine)) = (self.ot_engine(f), self.ot_engine(sf)) else {
            return Scaled::ZERO;
        };
        // Kerns are computed in base-glyph units, with XeTeX's float/int conversions.
        {
            let font = engine.font();
            let sfont = sengine.font();

            let (g_h, g_d) = self.glyph_height_depth(f, g);
            let (sg_h, sg_d) = self.glyph_height_depth(sf, sg);
            let g_height = points_to_units(font, g_h) as i32;
            let g_depth = points_to_units(font, g_d) as i32;
            let sg_height = points_to_units(sfont, sg_h) as i32;
            let sg_depth = points_to_units(sfont, sg_d) as i32;

            let shift = points_to_units(font, fix2d(shift_scaled) as f32) as i32;

            let f_size = font.point_size();
            let sf_size = sfont.point_size();
            assert!(f_size != 0.0);
            let scale = sf_size / f_size;

            let scaled_sum = |kern: i32, skern: i32| (kern as f32 + scale * skern as f32) as i32;

            let rval = match kind {
                ScriptKind::Sup => {
                    let kern = self.math_kern_at(
                        f,
                        g,
                        (shift as f32 - scale * sg_depth as f32) as i32,
                        hb::HB_OT_MATH_KERN_TOP_RIGHT,
                    );
                    let skern =
                        self.math_kern_at(sf, sg, -sg_depth, hb::HB_OT_MATH_KERN_BOTTOM_LEFT);
                    let top_kern = scaled_sum(kern, skern);

                    let kern = self.math_kern_at(f, g, g_height, hb::HB_OT_MATH_KERN_TOP_RIGHT);
                    let skern = self.math_kern_at(
                        sf,
                        sg,
                        ((g_height - shift) as f32 / scale) as i32,
                        hb::HB_OT_MATH_KERN_BOTTOM_LEFT,
                    );
                    let bot_kern = scaled_sum(kern, skern);

                    // The spec says minimum, but the maximum gives the greater separation.
                    top_kern.max(bot_kern)
                }
                ScriptKind::Sub => {
                    let kern = self.math_kern_at(
                        f,
                        g,
                        (scale * sg_height as f32 - shift as f32) as i32,
                        hb::HB_OT_MATH_KERN_BOTTOM_RIGHT,
                    );
                    let skern = self.math_kern_at(sf, sg, sg_height, hb::HB_OT_MATH_KERN_TOP_LEFT);
                    let top_kern = scaled_sum(kern, skern);

                    let kern = self.math_kern_at(f, g, -g_depth, hb::HB_OT_MATH_KERN_BOTTOM_RIGHT);
                    let skern = self.math_kern_at(
                        sf,
                        sg,
                        ((shift - g_depth) as f32 / scale) as i32,
                        hb::HB_OT_MATH_KERN_TOP_LEFT,
                    );
                    let bot_kern = scaled_sum(kern, skern);

                    top_kern.max(bot_kern)
                }
            };

            units_to_fix(font, rval)
        }
    }
}
