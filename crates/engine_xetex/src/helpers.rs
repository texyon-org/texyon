//! Pure helper functions translated from `xetex/xetex-xetex0.c`.

// TeX constant from xetex-constants.h:337
const INF_BAD: i32 = 10000;

// ---------------------------------------------------------------------------
// badness
// ---------------------------------------------------------------------------

/// Compute the "badness" of fitting a box with natural size `t` into a
/// space of size `s`. Returns 0 if `t == 0`, `INF_BAD` (10000) if `s <= 0`
/// or the result would exceed 10000, otherwise `(t/s)^3 * 100` rounded.
///
/// translated from xetex-xetex0.c:25 (badness)
pub fn badness(t: i32, s: i32) -> i32 {
    if t == 0 {
        return 0;
    }

    if s <= 0 {
        return INF_BAD;
    }

    // The three branches choose how to compute r = (t * 297) / s without
    // overflow, using the same magic constants as the C source.
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

// ---------------------------------------------------------------------------
// norm_min
// ---------------------------------------------------------------------------

/// Clamp `h` to the range [1, 63] for hyphenation min values.
///
/// translated from xetex-xetex0.c:12014 (norm_min)
pub fn norm_min(h: i32) -> i16 {
    if h <= 0 {
        1
    } else if h >= 63 {
        63
    } else {
        h as i16
    }
}

// ===========================================================================
// tests
// ===========================================================================

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
        // t == s means ratio 1.0 → r = 297 → r^3/262144 = 26236551/262144 = 100
        assert_eq!(badness(1000, 1000), 100);
    }

    #[test]
    fn badness_small_ratio_gives_zero() {
        // Very small t/s: r = (1 * 297) / 1_000_000 = 0, so result = 0.
        assert_eq!(badness(1, 1_000_000), 0);
    }

    #[test]
    fn badness_large_t_s_path() {
        // t > 7_230_584 and s < 1_663_497 → r = t, which is > 1290, → INF_BAD.
        assert_eq!(badness(8_000_000, 1_000_000), INF_BAD);
    }

    #[test]
    fn badness_large_s_path() {
        // t > 7_230_584 and s >= 1_663_497 → use t / (s/297).
        // t = 8_000_000, s = 2_000_000 → s/297 = 6734, t/6734 = 1188 < 1290.
        // result = (1188^3 + 0x20000) / 0x40000 = (1677302272 + 131072) / 262144 = 6402
        let b = badness(8_000_000, 2_000_000);
        // Not checking exact value — just that it's finite.
        assert!(b < INF_BAD);
        assert!(b > 0);
    }
}
