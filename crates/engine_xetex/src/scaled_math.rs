//! Scaled-point arithmetic with XeTeX's overflow checks and remainder-sign
//! conventions.

use crate::state::{EngineState, ErrorState};
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

/// Side-effect-free core of `xn_over_d`: `(quotient, remainder, overflowed)`.
pub(crate) fn xn_over_d_parts<T: TexWord>(x: T, n: i32, d: i32) -> (T, i32, bool) {
    let (q, r, overflowed) = xn_over_d_parts_raw(x.raw(), n, d);
    (T::from_raw(q), r, overflowed)
}

fn xn_over_d_parts_raw(mut x: i32, n: i32, d: i32) -> (i32, i32, bool) {
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
        (u, v % d, overflowed)
    } else {
        (u.wrapping_neg(), (v % d).wrapping_neg(), overflowed)
    }
}

/// The sign of `a*b - c*d`, computed without overflow.
fn ab_vs_cd(mut a: i32, mut b: i32, mut c: i32, mut d: i32) -> i32 {
    if a < 0 {
        a = -a;
        b = -b;
    }
    if c < 0 {
        c = -c;
        d = -d;
    }

    if d <= 0 {
        if b >= 0 {
            return if (a == 0 || b == 0) && (c == 0 || d == 0) {
                0
            } else {
                1
            };
        }
        if d == 0 {
            return if a == 0 { 0 } else { -1 };
        }
        std::mem::swap(&mut a, &mut c);
        let tmp = -b;
        b = -d;
        d = tmp;
    } else if b <= 0 {
        if b < 0 && a > 0 {
            return -1;
        }
        return if c == 0 { 0 } else { -1 };
    }

    loop {
        let q = a / d;
        let r = c / b;
        if q != r {
            return if q > r { 1 } else { -1 };
        }

        let q = a % d;
        let r = c % b;
        if r == 0 {
            return if q == 0 { 0 } else { 1 };
        }
        if q == 0 {
            return -1;
        }

        a = b;
        b = q;
        c = d;
        d = r;
    }
}

impl ErrorState {
    /// `n*x + y`, or 0 with `arith_error` beyond `max_answer`.
    pub(crate) fn mult_and_add<T: TexWord>(&mut self, n: i32, x: T, y: T, max_answer: T) -> T {
        T::from_raw(self.mult_and_add_raw(n, x.raw(), y.raw(), max_answer.raw()))
    }

    pub fn x_over_n<T: TexWord>(&mut self, x: T, n: i32) -> T {
        T::from_raw(self.x_over_n_raw(x.raw(), n))
    }

    pub(crate) fn xn_over_d<T: TexWord>(&mut self, x: T, n: i32, d: i32) -> T {
        T::from_raw(self.xn_over_d_raw(x.raw(), n, d))
    }

    /// `xn_over_d` rounded half away from zero. Leaves `tex_remainder` alone.
    pub fn round_xn_over_d<T: TexWord>(&mut self, x: T, n: i32, d: i32) -> T {
        T::from_raw(self.round_xn_over_d_raw(x.raw(), n, d))
    }

    fn mult_and_add_raw(&mut self, mut n: i32, mut x: i32, y: i32, max_answer: i32) -> i32 {
        if n < 0 {
            x = x.wrapping_neg();
            n = n.wrapping_neg();
        }
        if n == 0 {
            return y;
        }
        // The bounds may overflow i32 and must wrap.
        let upper_ok = x <= max_answer.wrapping_sub(y) / n;
        let lower_ok = x.wrapping_neg() <= max_answer.wrapping_add(y) / n;
        if upper_ok && lower_ok {
            n.wrapping_mul(x).wrapping_add(y)
        } else {
            self.arith_error = true;
            0
        }
    }

    fn x_over_n_raw(&mut self, mut x: i32, mut n: i32) -> i32 {
        if n == 0 {
            self.arith_error = true;
            self.tex_remainder = x;
            return 0;
        }
        if n < 0 {
            x = x.wrapping_neg();
            n = n.wrapping_neg();
            self.tex_remainder = self.tex_remainder.wrapping_neg();
        }
        if x >= 0 {
            self.tex_remainder = x % n;
            x / n
        } else {
            let neg_x = x.wrapping_neg();
            self.tex_remainder = (neg_x % n).wrapping_neg();
            (neg_x / n).wrapping_neg()
        }
    }

    fn xn_over_d_raw(&mut self, x: i32, n: i32, d: i32) -> i32 {
        let (quotient, remainder, overflowed) = xn_over_d_parts(x, n, d);
        if overflowed {
            self.arith_error = true;
        }
        self.tex_remainder = remainder;
        quotient
    }

    fn round_xn_over_d_raw(&mut self, x: i32, n: i32, d: i32) -> i32 {
        let (quotient, remainder, overflowed) = xn_over_d_parts(x, n, d);
        if overflowed {
            self.arith_error = true;
        }
        if x >= 0 {
            if remainder.wrapping_mul(2) >= d {
                quotient.wrapping_add(1)
            } else {
                quotient
            }
        } else if remainder.wrapping_neg().wrapping_mul(2) >= d {
            quotient.wrapping_sub(1)
        } else {
            quotient
        }
    }

    /// `p/q` as a 28-bit fraction (1.0 == 0x10000000), saturating at |p/q| >= 8.
    fn make_frac(&mut self, mut p: i32, mut q: i32) -> i32 {
        let mut negative = p < 0;
        if negative {
            p = -p;
        }
        if q <= 0 {
            q = -q;
            negative = !negative;
        }

        let n = p / q;
        p %= q;
        if n >= 8 {
            self.arith_error = true;
            return if negative { -0x7FFF_FFFF } else { 0x7FFF_FFFF };
        }

        let n = (n - 1).wrapping_mul(0x1000_0000);
        let mut f = 1_i32;
        loop {
            let be_careful = p.wrapping_sub(q);
            p = be_careful.wrapping_add(p);
            if p >= 0 {
                f = f + f + 1;
            } else {
                f = f + f;
                p += q;
            }
            if f >= 0x1000_0000 {
                break;
            }
        }

        let be_careful = p.wrapping_sub(q);
        if be_careful.wrapping_add(p) >= 0 {
            f += 1;
        }

        if negative {
            -(f + n)
        } else {
            f + n
        }
    }

    /// `q` times the 28-bit fraction `f`. A `q` of `i32::MIN` wraps as in C.
    fn take_frac(&mut self, mut q: i32, mut f: i32) -> i32 {
        let mut negative = f < 0;
        if negative {
            f = -f;
        }
        if q < 0 {
            q = q.wrapping_neg();
            negative = !negative;
        }

        let mut n = if f < 0x1000_0000 {
            0_i32
        } else {
            let nn = f / 0x1000_0000;
            f %= 0x1000_0000;
            if q <= 0x7FFF_FFFF / nn {
                nn * q
            } else {
                self.arith_error = true;
                0x7FFF_FFFF
            }
        };

        f += 0x1000_0000;
        let mut p = 0x0800_0000_i32;
        if q < 0x4000_0000 {
            loop {
                if f & 1 != 0 {
                    p = p.wrapping_add(q) / 2;
                } else {
                    p /= 2;
                }
                f /= 2;
                if f == 1 {
                    break;
                }
            }
        } else {
            loop {
                if f & 1 != 0 {
                    p += q.wrapping_sub(p) / 2;
                } else {
                    p /= 2;
                }
                f /= 2;
                if f == 1 {
                    break;
                }
            }
        }

        let be_careful = n - 0x7FFF_FFFF;
        if be_careful.wrapping_add(p) > 0 {
            self.arith_error = true;
            n = 0x7FFF_FFFF_i32.wrapping_sub(p);
        }

        if negative {
            n.wrapping_add(p).wrapping_neg()
        } else {
            n.wrapping_add(p)
        }
    }

    fn new_randoms(&mut self) {
        for k in 0_usize..24 {
            let x = self.randoms[k].wrapping_sub(self.randoms[k + 31]);
            self.randoms[k] = if x < 0 { x + 0x1000_0000 } else { x };
        }
        for k in 24_usize..55 {
            let x = self.randoms[k].wrapping_sub(self.randoms[k - 24]);
            self.randoms[k] = if x < 0 { x + 0x1000_0000 } else { x };
        }
        self.j_random = 54;
    }

    fn next_random(&mut self) -> i32 {
        if self.j_random == 0 {
            self.new_randoms();
        } else {
            self.j_random -= 1;
        }
        self.randoms[self.j_random as usize]
    }

    pub fn init_randoms(&mut self, seed: i32) {
        let mut j = seed.wrapping_abs();
        while j >= 0x1000_0000 {
            j /= 2;
        }

        let mut k = 1_i32;
        for i in 0_usize..55 {
            let jj = k;
            k = j.wrapping_sub(k);
            j = jj;
            if k < 0 {
                k += 0x1000_0000;
            }
            self.randoms[(i * 21) % 55] = j;
        }

        self.new_randoms();
        self.new_randoms();
        self.new_randoms();
    }

    /// A uniform random integer between 0 and `x` (exclusive, sign of `x`).
    pub fn unif_rand(&mut self, x: i32) -> i32 {
        let r = self.next_random();
        let y = self.take_frac(x.wrapping_abs(), r);
        if y == x.wrapping_abs() {
            0
        } else if x > 0 {
            y
        } else {
            -y
        }
    }
}

/// `ln(x / 2^30) * 2^24`.
fn m_log(st: &mut EngineState, mut x: i32) -> i32 {
    if x <= 0 {
        crate::output::error_here_with_diagnostic(st, "Logarithm of ");
        crate::output::print_scaled(st, Scaled::from_raw(x));
        crate::output::print(st, " has been replaced by 0");
        crate::output::capture_to_diagnostic(st, None);
        st.errors.help(&[
            "Since I don't take logs of non-positive numbers,",
            "I'm zeroing this one. Proceed, with fingers crossed.",
        ]);
        // Unreachable: norm_rand always passes x >= 1.
        let _ = crate::errors::finish_error(st);
        return 0;
    }

    let mut y = 1_302_456_860_i32;
    let mut z = 6_581_195_i32;
    while x < 0x4000_0000 {
        x += x;
        y -= 93_032_639;
        z -= 48_782;
    }

    y += z / 65536;
    let mut k: usize = 2;
    while x > 0x4000_0004 {
        let mut z = ((x - 1) / st.errors.two_to_the[k]) + 1;
        while x < 0x4000_0000 + z {
            z = (z + 1) / 2;
            k += 1;
        }
        y += st.errors.spec_log[k];
        x -= z;
    }

    y / 8
}

/// A normally distributed random integer.
pub fn norm_rand(st: &mut EngineState) -> i32 {
    loop {
        let (x, u) = loop {
            let r = st.errors.next_random() - 0x0800_0000;
            let x = st.errors.take_frac(112429, r);
            let u = st.errors.next_random();
            if x.abs() < u {
                break (x, u);
            }
        };

        let xi = st.errors.make_frac(x, u);
        let l = 139_548_960_i32 - m_log(st, u);
        if ab_vs_cd(1024, l, xi, xi) >= 0 {
            return xi;
        }
    }
}

impl ErrorState {
    pub(crate) fn add_or_sub<T: TexWord>(
        &mut self,
        x: T,
        y: T,
        max_answer: T,
        negative: bool,
    ) -> T {
        T::from_raw(self.add_or_sub_raw(x.raw(), y.raw(), max_answer.raw(), negative))
    }

    pub(crate) fn fract<T: TexWord>(&mut self, x: T, n: i32, d: i32, max_answer: T) -> T {
        T::from_raw(self.fract_raw(x.raw(), n, d, max_answer.raw()))
    }

    fn add_or_sub_raw(&mut self, x: i32, y: i32, max_answer: i32, negative: bool) -> i32 {
        let y = if negative { -y } else { y };
        if x >= 0 {
            if y <= max_answer - x {
                x + y
            } else {
                self.arith_error = true;
                0
            }
        } else if y >= -max_answer - x {
            x + y
        } else {
            self.arith_error = true;
            0
        }
    }

    pub(crate) fn quotient(&mut self, mut n: i32, mut d: i32) -> i32 {
        if d == 0 {
            self.arith_error = true;
            return 0;
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
        if negative {
            -a
        } else {
            a
        }
    }

    fn fract_raw(&mut self, mut x: i32, mut n: i32, mut d: i32, max_answer: i32) -> i32 {
        if d == 0 {
            self.arith_error = true;
            return 0;
        }
        let mut negative = d < 0;
        if negative {
            d = -d;
        }
        if x < 0 {
            x = -x;
            negative = !negative;
        } else if x == 0 {
            return 0;
        }
        if n < 0 {
            n = -n;
            negative = !negative;
        }

        let t = n / d;
        if t > max_answer / x {
            self.arith_error = true;
            return 0;
        }
        let mut a = t * x;
        n -= t * d;
        if n != 0 {
            let t = x / d;
            if t > (max_answer - a) / n {
                self.arith_error = true;
                return 0;
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
                    self.arith_error = true;
                    return 0;
                }
                a += f;
            }
        }

        if negative {
            -a
        } else {
            a
        }
    }
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

        let mut e = ErrorState::default();
        assert_eq!(e.mult_and_add(3, 7, 1, MAX), 22);
        assert!(!e.arith_error);

        assert_eq!(e.mult_and_add(0, 7, 42, MAX), 42);
        assert!(!e.arith_error);

        assert_eq!(e.mult_and_add(-2, 3, 1, MAX), -5);
        assert!(!e.arith_error);
    }

    #[test]
    fn mult_and_add_detects_overflow() {
        let mut e = ErrorState::default();
        let r = e.mult_and_add(2, 2_000_000_000, 1_000_000_000, i32::MAX);
        assert_eq!(r, 0);
        assert!(e.arith_error);
    }

    #[test]
    fn x_over_n_positive() {
        let mut e = ErrorState::default();
        assert_eq!(e.x_over_n(17, 5), 3);
        assert_eq!(e.tex_remainder, 2);
        assert!(!e.arith_error);
    }

    #[test]
    fn x_over_n_negative_x_gives_negative_remainder() {
        let mut e = ErrorState::default();
        assert_eq!(e.x_over_n(-17, 5), -3);
        assert_eq!(e.tex_remainder, -2);
        assert!(!e.arith_error);
    }

    #[test]
    fn x_over_n_zero_divisor_sets_flag() {
        let mut e = ErrorState::default();
        assert_eq!(e.x_over_n(42, 0), 0);
        assert_eq!(e.tex_remainder, 42);
        assert!(e.arith_error);
    }

    #[test]
    fn xn_over_d_small_values() {
        let mut e = ErrorState::default();
        assert_eq!(e.xn_over_d(10, 3, 4), 7);
        assert_eq!(e.tex_remainder, 2);
        assert!(!e.arith_error);
    }

    #[test]
    fn xn_over_d_large_x_and_divisor() {
        let mut e = ErrorState::default();
        assert_eq!(e.xn_over_d(1_000_000_000, 3, 1000), 3_000_000);
        assert!(!e.arith_error);
        assert_eq!(e.tex_remainder, 0);
    }

    #[test]
    fn xn_over_d_detects_overflow() {
        let mut e = ErrorState::default();
        let _ = e.xn_over_d(i32::MAX, 1, 1);
        assert!(e.arith_error);
    }

    #[test]
    fn xn_over_d_negative_x_negates_result_and_remainder() {
        let mut e = ErrorState::default();
        assert_eq!(e.xn_over_d(-10, 3, 4), -7);
        assert_eq!(e.tex_remainder, -2);
    }

    #[test]
    fn round_xn_over_d_rounds_half_up() {
        let mut e = ErrorState::default();
        assert_eq!(e.round_xn_over_d(10, 3, 4), 8);
        assert_eq!(e.round_xn_over_d(10, 3, 5), 6);
        assert_eq!(e.round_xn_over_d(10, 2, 4), 5);
    }

    #[test]
    fn round_xn_over_d_negative_x() {
        let mut e = ErrorState::default();
        assert_eq!(e.round_xn_over_d(-10, 3, 4), -8);
    }
}
