use crate::Fixed;

pub fn fix_to_d(f: Fixed) -> f64 {
    f as f64 / 65536.0
}

pub fn d_to_fix(d: f64) -> Fixed {
    (d * 65536.0 + 0.5) as Fixed
}
