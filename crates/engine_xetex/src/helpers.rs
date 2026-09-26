use crate::consts::{BIGGEST_LANG, INF_BAD};
use crate::units::Scaled;

/// Badness of stretching or shrinking natural size `t` by available amount `s`.
pub fn badness(t: Scaled, s: Scaled) -> i32 {
    let (t, s) = (t.0, s.0);
    if t == 0 {
        return 0;
    }
    if s <= 0 {
        return INF_BAD;
    }
    // Approximates 100(t/s)^3 without overflow for t >= 0, as in tex.web.
    let r = if t <= 7_230_584 {
        t.wrapping_mul(297) / s
    } else if s >= 1_663_497 {
        t / (s / 297)
    } else {
        t
    };
    if r > 1290 {
        return INF_BAD;
    }
    r.wrapping_mul(r).wrapping_mul(r).wrapping_add(0x2_0000) / 0x4_0000
}

/// Runs its closure when dropped, including during unwinding.
pub(crate) struct OnDrop<F: FnMut()>(pub F);

impl<F: FnMut()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

pub fn norm_min(h: i32) -> i16 {
    h.clamp(1, 63) as i16
}

/// `\language` as a valid language number, 0 when out of range.
pub fn norm_lang(lang: i32) -> u8 {
    if lang <= 0 || lang > BIGGEST_LANG {
        0
    } else {
        lang as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badness_zero_t() {
        assert_eq!(badness(Scaled::ZERO, Scaled(1000)), 0);
    }

    #[test]
    fn badness_zero_or_neg_s() {
        assert_eq!(badness(Scaled(1), Scaled::ZERO), INF_BAD);
        assert_eq!(badness(Scaled(1), Scaled(-5)), INF_BAD);
    }

    #[test]
    fn badness_perfect_fit() {
        assert_eq!(badness(Scaled(1000), Scaled(1000)), 100);
    }

    #[test]
    fn badness_small_ratio_gives_zero() {
        assert_eq!(badness(Scaled(1), Scaled(1_000_000)), 0);
    }

    #[test]
    fn badness_large_t_s_path() {
        assert_eq!(badness(Scaled(8_000_000), Scaled(1_000_000)), INF_BAD);
    }

    #[test]
    fn badness_large_s_path() {
        let b = badness(Scaled(8_000_000), Scaled(2_000_000));
        assert!(b < INF_BAD);
        assert!(b > 0);
    }
}
