//! Scaled-point arithmetic with XeTeX's overflow checks and remainder-sign
//! conventions.

use crate::units::Scaled;

/// A 32-bit TeX value that the checked arithmetic accepts: an integer or a `Scaled`.
pub(crate) trait TexWord: Copy {
    fn raw(self) -> i32;
    fn from_raw(raw: i32) -> Self;
}

impl TexWord for i32 {
    #[inline(always)]
    fn raw(self) -> i32 {
        self
    }
    #[inline(always)]
    fn from_raw(raw: i32) -> Self {
        raw
    }
}

impl TexWord for Scaled {
    #[inline(always)]
    fn raw(self) -> i32 {
        self.raw()
    }
    #[inline(always)]
    fn from_raw(raw: i32) -> Self {
        Scaled::from_raw(raw)
    }
}

/// Round half away from zero, clipping to the `i32` range.
pub fn tex_round(r: f64) -> i32 {
    if r > 2_147_483_647.0 {
        return i32::MAX;
    }
    if r < -2_147_483_648.0 {
        return i32::MIN;
    }
    if r >= 0.0 {
        (r + 0.5) as i32
    } else {
        (r - 0.5) as i32
    }
}

/// `(x + 1) / 2` for odd `x`, else `x / 2`.
pub fn half(x: i32) -> i32 {
    if x & 1 != 0 {
        x.wrapping_add(1) / 2
    } else {
        x / 2
    }
}

/// A result with TeX's `arith_error`. On overflow `value` is what TeX leaves.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Overflowing<T> {
    pub value: T,
    pub overflowed: bool,
}

impl<T> Overflowing<T> {
    fn ok(value: T) -> Self {
        Overflowing {
            value,
            overflowed: false,
        }
    }

    fn overflow(value: T) -> Self {
        Overflowing {
            value,
            overflowed: true,
        }
    }

    fn map<U>(self, f: impl FnOnce(T) -> U) -> Overflowing<U> {
        Overflowing {
            value: f(self.value),
            overflowed: self.overflowed,
        }
    }

    /// The value, raising `overflowed` on overflow.
    pub(crate) fn track(self, overflowed: &mut bool) -> T {
        *overflowed |= self.overflowed;
        self.value
    }
}

/// A quotient with its remainder and TeX's `arith_error`.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Divided<T, R> {
    pub quotient: T,
    pub remainder: R,
    pub overflowed: bool,
}

/// `n*x + y`, or 0 with overflow beyond `max_answer`.
pub(crate) fn mult_and_add<T: TexWord>(n: i32, x: T, y: T, max_answer: T) -> Overflowing<T> {
    mult_and_add_raw(n, x.raw(), y.raw(), max_answer.raw()).map(T::from_raw)
}

fn mult_and_add_raw(mut n: i32, mut x: i32, y: i32, max_answer: i32) -> Overflowing<i32> {
    if n < 0 {
        x = x.wrapping_neg();
        n = n.wrapping_neg();
    }
    if n == 0 {
        return Overflowing::ok(y);
    }
    // The bounds may overflow i32 and must wrap.
    let upper_ok = x <= max_answer.wrapping_sub(y) / n;
    let lower_ok = x.wrapping_neg() <= max_answer.wrapping_add(y) / n;
    if upper_ok && lower_ok {
        Overflowing::ok(n.wrapping_mul(x).wrapping_add(y))
    } else {
        Overflowing::overflow(0)
    }
}

/// `x / n` toward zero, the remainder taking the sign of `x`. Dividing
/// by zero overflows with quotient 0 and remainder `x`.
pub(crate) fn x_over_n<T: TexWord>(x: T, n: i32) -> Divided<T, T> {
    let d = x_over_n_raw(x.raw(), n);
    Divided {
        quotient: T::from_raw(d.quotient),
        remainder: T::from_raw(d.remainder),
        overflowed: d.overflowed,
    }
}

fn x_over_n_raw(mut x: i32, mut n: i32) -> Divided<i32, i32> {
    if n == 0 {
        return Divided {
            quotient: 0,
            remainder: x,
            overflowed: true,
        };
    }
    let negative = n < 0;
    if negative {
        x = x.wrapping_neg();
        n = n.wrapping_neg();
    }
    let (quotient, remainder) = if x >= 0 {
        (x / n, x % n)
    } else {
        let neg_x = x.wrapping_neg();
        ((neg_x / n).wrapping_neg(), (neg_x % n).wrapping_neg())
    };
    Divided {
        quotient,
        remainder: if negative {
            remainder.wrapping_neg()
        } else {
            remainder
        },
        overflowed: false,
    }
}

/// `x * n / d` toward zero. The remainder counts `d`ths of one unit of the quotient.
pub(crate) fn xn_over_d<T: TexWord>(x: T, n: i32, d: i32) -> Divided<T, i32> {
    let r = xn_over_d_raw(x.raw(), n, d);
    Divided {
        quotient: T::from_raw(r.quotient),
        remainder: r.remainder,
        overflowed: r.overflowed,
    }
}

fn xn_over_d_raw(mut x: i32, n: i32, d: i32) -> Divided<i32, i32> {
    let positive = x >= 0;
    if !positive {
        x = x.wrapping_neg();
    }

    // The halves are truncated to i32, so the products wrap.
    let t = (x % 32768).wrapping_mul(n);
    let mut u = (x / 32768).wrapping_mul(n).wrapping_add(t / 32768);
    let v = (u % d).wrapping_mul(32768).wrapping_add(t % 32768);

    let overflowed = u / d >= 32768;
    if !overflowed {
        u = 32768i32.wrapping_mul(u / d).wrapping_add(v / d);
    }

    if positive {
        Divided {
            quotient: u,
            remainder: v % d,
            overflowed,
        }
    } else {
        Divided {
            quotient: u.wrapping_neg(),
            remainder: (v % d).wrapping_neg(),
            overflowed,
        }
    }
}

/// `xn_over_d` rounded half away from zero.
pub(crate) fn round_xn_over_d<T: TexWord>(x: T, n: i32, d: i32) -> T {
    let r = xn_over_d_raw(x.raw(), n, d);
    T::from_raw(if x.raw() >= 0 {
        if r.remainder.wrapping_mul(2) >= d {
            r.quotient.wrapping_add(1)
        } else {
            r.quotient
        }
    } else if r.remainder.wrapping_neg().wrapping_mul(2) >= d {
        r.quotient.wrapping_sub(1)
    } else {
        r.quotient
    })
}

/// e-TeX's `add_or_sub`: `x + y` or `x - y`, or 0 with overflow beyond `max_answer`.
pub(crate) fn add_or_sub<T: TexWord>(x: T, y: T, max_answer: T, negative: bool) -> Overflowing<T> {
    let (x, max_answer) = (x.raw(), max_answer.raw());
    let y = if negative { -y.raw() } else { y.raw() };
    let fits = if x >= 0 {
        y <= max_answer - x
    } else {
        y >= -max_answer - x
    };
    if fits {
        Overflowing::ok(T::from_raw(x + y))
    } else {
        Overflowing::overflow(T::from_raw(0))
    }
}

/// e-TeX's `fract`: `x * n / d` rounded, or 0 with overflow beyond `max_answer`.
pub(crate) fn fract<T: TexWord>(x: T, n: i32, d: i32, max_answer: T) -> Overflowing<T> {
    fract_raw(x.raw(), n, d, max_answer.raw()).map(T::from_raw)
}

/// e-TeX's `quotient`: `n / d` rounded half away from zero.
pub(crate) fn quotient(mut n: i32, mut d: i32) -> Overflowing<i32> {
    if d == 0 {
        return Overflowing::overflow(0);
    }
    let mut negative = d < 0;
    if negative {
        d = -d;
    }
    if n < 0 {
        n = -n;
        negative = !negative;
    }
    let mut a = n / d;
    n -= a * d;
    d = n - d;
    if d + n >= 0 {
        a += 1;
    }
    Overflowing::ok(if negative { -a } else { a })
}

fn fract_raw(mut x: i32, mut n: i32, mut d: i32, max_answer: i32) -> Overflowing<i32> {
    if d == 0 {
        return Overflowing::overflow(0);
    }
    let mut negative = d < 0;
    if negative {
        d = -d;
    }
    if x < 0 {
        x = -x;
        negative = !negative;
    } else if x == 0 {
        return Overflowing::ok(0);
    }
    if n < 0 {
        n = -n;
        negative = !negative;
    }

    let t = n / d;
    if t > max_answer / x {
        return Overflowing::overflow(0);
    }
    let mut a = t * x;
    n -= t * d;
    if n != 0 {
        let t = x / d;
        if t > (max_answer - a) / n {
            return Overflowing::overflow(0);
        }
        a += t * n;
        x -= t * d;
        if x != 0 {
            if x < n {
                std::mem::swap(&mut x, &mut n);
            }
            let mut f = 0i32;
            let mut r = (d / 2) - d;
            let h = -r;
            loop {
                if n & 1 != 0 {
                    r += x;
                    if r >= 0 {
                        r -= d;
                        f += 1;
                    }
                }
                n /= 2;
                if n == 0 {
                    break;
                }
                if x < h {
                    x += x;
                } else {
                    let tmp = x - d;
                    x += tmp;
                    f += n;
                    if x < n {
                        if x == 0 {
                            break;
                        }
                        std::mem::swap(&mut x, &mut n);
                    }
                }
            }
            if f > max_answer - a {
                return Overflowing::overflow(0);
            }
            a += f;
        }
    }

    Overflowing::ok(if negative { -a } else { a })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tex_round_basics() {
        assert_eq!(tex_round(0.0), 0);
        assert_eq!(tex_round(0.5), 1);
        assert_eq!(tex_round(-0.5), -1);
        assert_eq!(tex_round(0.49), 0);
        assert_eq!(tex_round(-0.49), 0);
        assert_eq!(tex_round(0.51), 1);
        assert_eq!(tex_round(-0.51), -1);
        assert_eq!(tex_round(1.5), 2);
        assert_eq!(tex_round(-1.5), -2);
    }

    #[test]
    fn tex_round_clips_to_i32_range() {
        assert_eq!(tex_round(1e20), i32::MAX);
        assert_eq!(tex_round(-1e20), i32::MIN);
        assert_eq!(tex_round(2_147_483_648.0), i32::MAX);
        assert_eq!(tex_round(-2_147_483_649.0), i32::MIN);
    }

    #[test]
    fn half_basics() {
        assert_eq!(half(0), 0);
        assert_eq!(half(1), 1);
        assert_eq!(half(2), 1);
        assert_eq!(half(3), 2);
        assert_eq!(half(4), 2);
        assert_eq!(half(-1), 0);
        assert_eq!(half(-2), -1);
        assert_eq!(half(-3), -1);
        assert_eq!(half(-4), -2);
    }

    #[test]
    fn mult_and_add_basic() {
        // TeX always passes MAX_HALFWORD as max_answer.
        const MAX: i32 = 0x3FFF_FFFF;
        assert_eq!(mult_and_add(3, 7, 1, MAX), Overflowing::ok(22));
        assert_eq!(mult_and_add(0, 7, 42, MAX), Overflowing::ok(42));
        assert_eq!(mult_and_add(-2, 3, 1, MAX), Overflowing::ok(-5));
    }

    #[test]
    fn mult_and_add_detects_overflow() {
        assert_eq!(
            mult_and_add(2, 2_000_000_000, 1_000_000_000, i32::MAX),
            Overflowing::overflow(0)
        );
    }

    #[test]
    fn track_keeps_an_earlier_overflow() {
        let mut overflowed = false;
        assert_eq!(
            mult_and_add(2, 2_000_000_000, 0, i32::MAX).track(&mut overflowed),
            0
        );
        assert_eq!(mult_and_add(2, 3, 0, i32::MAX).track(&mut overflowed), 6);
        assert!(overflowed);
    }

    fn divided<R>(quotient: i32, remainder: R, overflowed: bool) -> Divided<i32, R> {
        Divided {
            quotient,
            remainder,
            overflowed,
        }
    }

    #[test]
    fn x_over_n_remainder_takes_the_sign_of_x() {
        assert_eq!(x_over_n(17, 5), divided(3, 2, false));
        assert_eq!(x_over_n(-17, 5), divided(-3, -2, false));
        assert_eq!(x_over_n(17, -5), divided(-3, 2, false));
        assert_eq!(x_over_n(-17, -5), divided(3, -2, false));
    }

    #[test]
    fn x_over_n_zero_divisor_overflows() {
        assert_eq!(x_over_n(42, 0), divided(0, 42, true));
    }

    #[test]
    fn xn_over_d_small_values() {
        assert_eq!(xn_over_d(10, 3, 4), divided(7, 2, false));
        assert_eq!(xn_over_d(-10, 3, 4), divided(-7, -2, false));
    }

    #[test]
    fn xn_over_d_large_x_and_divisor() {
        assert_eq!(
            xn_over_d(1_000_000_000, 3, 1000),
            divided(3_000_000, 0, false)
        );
    }

    #[test]
    fn xn_over_d_detects_overflow() {
        assert_eq!(xn_over_d(i32::MAX, 1, 1), divided(65535, 0, true));
    }

    #[test]
    fn round_xn_over_d_rounds_half_up() {
        assert_eq!(round_xn_over_d(10, 3, 4), 8);
        assert_eq!(round_xn_over_d(10, 3, 5), 6);
        assert_eq!(round_xn_over_d(10, 2, 4), 5);
        assert_eq!(round_xn_over_d(-10, 3, 4), -8);
    }

    #[test]
    fn e_tex_operations_report_overflow() {
        assert_eq!(add_or_sub(5, 3, 10, true), Overflowing::ok(2));
        assert_eq!(add_or_sub(8, 3, 10, false), Overflowing::overflow(0));
        assert_eq!(quotient(7, 2), Overflowing::ok(4));
        assert_eq!(quotient(-7, 2), Overflowing::ok(-4));
        assert_eq!(quotient(7, 0), Overflowing::overflow(0));
        assert_eq!(fract(10, 2, 3, 100), Overflowing::ok(7));
        assert_eq!(fract(10, 2, 0, 100), Overflowing::overflow(0));
        assert_eq!(fract(60, 2, 1, 100), Overflowing::overflow(0));
    }
}
