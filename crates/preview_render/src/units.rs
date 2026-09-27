//! TeX units and float canonicalization shared by the analysis passes.

pub(crate) const SP_TO_BP: f32 = 72.0 / 72.27 / 65_536.0;

/// 1.0 in 16.16 fixed point.
pub(crate) const FIXED_ONE: u32 = 65_536;

/// Signed 16.16 fixed point stored in a `u32`.
pub(crate) fn fixed_16_16(value: u32) -> f32 {
    i32::from_be_bytes(value.to_be_bytes()) as f32 / FIXED_ONE as f32
}

/// Map `-0.0` to `0.0` so equal values hash equally.
pub(crate) fn normalize_zero(value: f32) -> f32 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}
