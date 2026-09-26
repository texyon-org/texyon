//! Math mode: noad construction and `mlist_to_hlist`.

use crate::state::{EqtbState, FontState};
use crate::units::{FontId, Scaled};

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

pub(crate) const DELIMITED_CODE: i32 = 3;
pub(crate) const ABOVE_CODE: i32 = 0;
pub(crate) const OVER_CODE: i32 = 1;
pub(crate) const ATOP_CODE: i32 = 2;
pub(crate) const DEFAULT_CODE: Scaled = Scaled(0x4000_0000);

const BOTTOM_ACC: u16 = 2;
pub(crate) const MIDDLE_NOAD: u16 = 1;

pub(crate) const LIMITS: u16 = 1;
pub(crate) const NO_LIMITS: u16 = 2;
const SCRIPT_SCRIPT_SIZE: i32 = 512;
const TEXT_STYLE: i16 = 2;
const SCRIPT_STYLE: i16 = 4;

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
