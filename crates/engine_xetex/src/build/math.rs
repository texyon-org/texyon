//! Math mode: noad construction and `mlist_to_hlist`.

use bytemuck::{Pod, Zeroable};

use crate::base::units::{FontId, Scaled};
use crate::state::{EqtbState, FontState};

mod build;
mod delimiters;
mod display;
mod mlist;
mod scripts;

pub(crate) use self::build::{
    append_choices, build_choices, fin_mlist, math_ac, math_fraction, math_left_right,
    math_limit_switch, math_radical, start_eq_no, sub_sup,
};
pub(crate) use self::display::{after_math, check_display_end, init_math, resume_after_display};

pub(crate) const DEFAULT_CODE: Scaled = Scaled::from_raw(0x4000_0000);

const SCRIPT_SCRIPT_SIZE: i32 = 512;
/// A math style: display 0, text 2, script 4, scriptscript 6, one more when cramped.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Pod, Zeroable)]
#[repr(transparent)]
pub(crate) struct MathStyle(u16);

impl MathStyle {
    /// The font size of this style: `TEXT_SIZE`, `SCRIPT_SIZE` or `SCRIPT_SCRIPT_SIZE`.
    pub(crate) fn size(self) -> i32 {
        if self < Self::SCRIPT {
            crate::base::consts::TEXT_SIZE
        } else {
            crate::base::consts::SCRIPT_SIZE * ((i32::from(self.raw()) - 2) / 2)
        }
    }

    pub(crate) const DISPLAY: MathStyle = MathStyle(0);
    pub(crate) const TEXT: MathStyle = MathStyle(2);
    pub(crate) const SCRIPT: MathStyle = MathStyle(4);

    /// The style a style node or `\mathchoice` stores as `raw`.
    #[inline]
    pub(crate) const fn from_raw(raw: u16) -> MathStyle {
        MathStyle(raw)
    }

    #[inline]
    pub(crate) const fn raw(self) -> u16 {
        self.0
    }

    #[inline]
    pub(crate) fn is_cramped(self) -> bool {
        !self.0.is_multiple_of(2)
    }

    /// 0 for display, 1 text, 2 script, 3 scriptscript.
    #[inline]
    pub(crate) fn level(self) -> u16 {
        self.0 / 2
    }

    /// WEB's `cramped_style`.
    #[inline]
    pub(crate) fn cramped(self) -> MathStyle {
        MathStyle(2 * (self.0 / 2) + 1)
    }

    /// WEB's `sub_style`.
    #[inline]
    pub(crate) fn sub(self) -> MathStyle {
        MathStyle(2 * (self.0 / 4) + 5)
    }

    /// WEB's `sup_style`.
    #[inline]
    pub(crate) fn sup(self) -> MathStyle {
        MathStyle(2 * (self.0 / 4) + 4 + self.0 % 2)
    }

    /// WEB's `num_style`.
    #[inline]
    pub(crate) fn num(self) -> MathStyle {
        MathStyle(self.0 + 2 - 2 * (self.0 / 6))
    }

    /// WEB's `denom_style`.
    #[inline]
    pub(crate) fn denom(self) -> MathStyle {
        MathStyle(2 * (self.0 / 2) + 3 - 2 * (self.0 / 6))
    }
}

impl FontState {
    #[inline]
    fn font_uses_opentype(&self, f: FontId) -> bool {
        self.layout_engine(f).is_some_and(|e| e.using_opentype())
    }

    #[inline]
    fn font_is_ot_math(&self, f: FontId) -> bool {
        self.layout_engine(f)
            .is_some_and(|e| e.is_opentype_math_font())
    }

    #[inline]
    fn math_sy_param(&self, eqtb: &EqtbState, size_code: i32, param: i32) -> Scaled {
        let f = eqtb.math_font(2 + size_code);
        if self.font_is_ot_math(f) {
            self.get_native_mathsy_param(f, param)
        } else {
            self.param(f, param)
        }
    }

    #[inline]
    fn math_ext_param(&self, eqtb: &EqtbState, cur_size: i32, param: i32) -> Scaled {
        let f = eqtb.math_font(3 + cur_size);
        if self.font_is_ot_math(f) {
            self.get_native_mathex_param(f, param)
        } else {
            self.param(f, param)
        }
    }

    pub fn math_x_height(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 5)
    }

    pub fn math_quad(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 6)
    }

    pub fn num1(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 8)
    }

    pub fn num2(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 9)
    }

    pub fn num3(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 10)
    }

    pub fn denom1(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 11)
    }

    pub fn denom2(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 12)
    }

    pub fn sup1(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 13)
    }

    pub fn sup2(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 14)
    }

    pub fn sup3(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 15)
    }

    pub fn sub1(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 16)
    }

    pub fn sub2(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 17)
    }

    pub fn sup_drop(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 18)
    }

    pub fn sub_drop(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 19)
    }

    pub fn delim1(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 20)
    }

    pub fn delim2(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 21)
    }

    pub fn axis_height(&self, eqtb: &EqtbState, size_code: i32) -> Scaled {
        self.math_sy_param(eqtb, size_code, 22)
    }

    pub fn default_rule_thickness(&self, eqtb: &EqtbState, cur_size: i32) -> Scaled {
        self.math_ext_param(eqtb, cur_size, 8)
    }

    pub fn big_op_spacing1(&self, eqtb: &EqtbState, cur_size: i32) -> Scaled {
        self.math_ext_param(eqtb, cur_size, 9)
    }

    pub fn big_op_spacing2(&self, eqtb: &EqtbState, cur_size: i32) -> Scaled {
        self.math_ext_param(eqtb, cur_size, 10)
    }

    pub fn big_op_spacing3(&self, eqtb: &EqtbState, cur_size: i32) -> Scaled {
        self.math_ext_param(eqtb, cur_size, 11)
    }

    pub fn big_op_spacing4(&self, eqtb: &EqtbState, cur_size: i32) -> Scaled {
        self.math_ext_param(eqtb, cur_size, 12)
    }

    pub fn big_op_spacing5(&self, eqtb: &EqtbState, cur_size: i32) -> Scaled {
        self.math_ext_param(eqtb, cur_size, 13)
    }
}

#[cfg(test)]
mod tests {
    use super::MathStyle;

    #[test]
    fn style_transitions_follow_the_tex_table() {
        // D D' T T' S S' SS SS', as in The TeXbook, Appendix G.
        let all = (0..8).map(MathStyle::from_raw);
        let row =
            |f: fn(MathStyle) -> MathStyle| all.clone().map(|s| f(s).raw()).collect::<Vec<_>>();
        assert_eq!(row(MathStyle::sup), [4, 5, 4, 5, 6, 7, 6, 7]);
        assert_eq!(row(MathStyle::sub), [5, 5, 5, 5, 7, 7, 7, 7]);
        assert_eq!(row(MathStyle::num), [2, 3, 4, 5, 6, 7, 6, 7]);
        assert_eq!(row(MathStyle::denom), [3, 3, 5, 5, 7, 7, 7, 7]);
        assert_eq!(row(MathStyle::cramped), [1, 1, 3, 3, 5, 5, 7, 7]);
    }
}
