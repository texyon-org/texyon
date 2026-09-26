const INF_BAD: i32 = 10000;

/// Badness of stretching or shrinking natural size `t` by available amount `s`.
pub fn badness(t: i32, s: i32) -> i32 {
    if t == 0 {
        return 0;
    }
    if s <= 0 {
        return INF_BAD;
    }
    // Approximates 100(t/s)^3 without overflow, as in tex.web.
    let r = if t <= 7_230_584 {
        (t * 297) / s
    } else if s >= 1_663_497 {
        t / (s / 297)
    } else {
        t
    };
    if r > 1290 {
        return INF_BAD;
    }
    (r * r * r + 0x2_0000) / 0x4_0000
}

pub fn norm_min(h: i32) -> i16 {
    h.clamp(1, 63) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badness_zero_t() {
        assert_eq!(badness(0, 1000), 0);
    }

    #[test]
    fn badness_zero_or_neg_s() {
        assert_eq!(badness(1, 0), INF_BAD);
        assert_eq!(badness(1, -5), INF_BAD);
    }

    #[test]
    fn badness_perfect_fit() {
        assert_eq!(badness(1000, 1000), 100);
    }

    #[test]
    fn badness_small_ratio_gives_zero() {
        assert_eq!(badness(1, 1_000_000), 0);
    }

    #[test]
    fn badness_large_t_s_path() {
        assert_eq!(badness(8_000_000, 1_000_000), INF_BAD);
    }

    #[test]
    fn badness_large_s_path() {
        let b = badness(8_000_000, 2_000_000);
        assert!(b < INF_BAD);
        assert!(b > 0);
    }
}
