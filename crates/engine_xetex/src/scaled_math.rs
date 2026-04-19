//! Pure-arithmetic helpers translated from `xetex/xetex-scaledmath.c`.
//!
//! XeTeX measures lengths in "scaled points" (sp): 1/65536 of a TeX
//! point. All positions, widths, kerns, and glue amounts flow through
//! these routines, so correctness is paramount. Translations here are
//! deliberately boring: they match the C source line-for-line,
//! preserving the same overflow checks and remainder-sign conventions.
//!
//! See `plan/04-migration-order.md` Phase 1 for context on why these
//! functions are the very first ones translated.

use crate::globals::{
    j_random, random_entry, set_arith_error, set_help_line, set_help_ptr, set_j_random,
    set_random_entry, set_tex_remainder, spec_log, two_to_the,
};

extern "C" {
    fn error();
    fn capture_to_diagnostic(diagnostic: *mut std::ffi::c_void);
    fn error_here_with_diagnostic(message: *const libc::c_char) -> *mut std::ffi::c_void;
    fn print_scaled(s: i32);
    fn print_cstr(s: *const libc::c_char);
}

// ---------------------------------------------------------------------------
// tex_round
// ---------------------------------------------------------------------------

/// Round a `double` to the nearest `int32_t`, with half-way values
/// going away from zero (0.5 → 1, -0.5 → -1), clipping to the `i32`
/// range.
///
/// translated from xetex-scaledmath.c:11 (tex_round)
#[no_mangle]
pub unsafe extern "C" fn tex_round(r: f64) -> i32 {
    // We must reproduce very particular rounding semantics to pass the
    // TRIP test (see comment in the C source). Values outside the i32
    // range are clipped; within-range values round half-away-from-zero.

    if r > 2_147_483_647.0 {
        return i32::MAX;
    }
    if r < -2_147_483_648.0 {
        return i32::MIN;
    }

    // ANSI C's float-to-int cast truncates toward zero, so adding
    // ±0.5 before the cast yields round-half-away-from-zero. Rust's
    // `as i32` has the same truncation semantics for values in range.
    if r >= 0.0 {
        (r + 0.5) as i32
    } else {
        (r - 0.5) as i32
    }
}

// ---------------------------------------------------------------------------
// half
// ---------------------------------------------------------------------------

/// `half(x)` computes `(x + 1) / 2` when `x` is odd, otherwise `x / 2`.
/// Matches Pascal's `(x + 1) div 2` idiom and C integer division for
/// non-negative `x`; for negative odd `x` it rounds toward +∞ rather
/// than 0 (because `(x + 1) / 2` in C truncates toward zero).
///
/// translated from xetex-scaledmath.c:51 (half)
#[no_mangle]
pub unsafe extern "C" fn half(x: i32) -> i32 {
    // The C macro `odd(x)` is `((x) & 1)`; Rust's `x & 1 != 0` is the
    // same. We wrap the add/divide in `wrapping_*` to match the C's
    // implementation-defined-but-well-known behaviour for i32 overflow.
    if x & 1 != 0 {
        x.wrapping_add(1) / 2
    } else {
        x / 2
    }
}

// ---------------------------------------------------------------------------
// mult_and_add
// ---------------------------------------------------------------------------

/// Compute `n*x + y` with overflow detection.
///
/// Sets the `arith_error` flag (and returns 0) if the result would
/// exceed `max_answer` in magnitude.
///
/// translated from xetex-scaledmath.c:60 (mult_and_add)
#[no_mangle]
pub unsafe extern "C" fn mult_and_add(
    mut n: i32,
    mut x: i32, // scaled_t
    y: i32,     // scaled_t
    max_answer: i32, // scaled_t
) -> i32 {
    if n < 0 {
        x = x.wrapping_neg();
        n = n.wrapping_neg();
    }

    if n == 0 {
        return y;
    }

    // C: `x <= (max_answer - y) / n && (-x <= (max_answer + y) / n)`
    //
    // `max_answer - y` / `max_answer + y` can overflow i32 when
    // `y` has magnitude comparable to `i32::MAX`. C signed overflow
    // is undefined but in practice wraps on two's-complement hardware;
    // Rust debug mode panics unless we use `wrapping_*`. The wrapped
    // result matches the C behaviour bit-for-bit, so the comparison
    // returns the same boolean the C engine computes.
    let upper_ok = x <= max_answer.wrapping_sub(y) / n;
    let lower_ok = x.wrapping_neg() <= max_answer.wrapping_add(y) / n;

    if upper_ok && lower_ok {
        // The checks above prove `n*x + y` fits in i32, so
        // `wrapping_*` is semantically identical to checked arithmetic
        // here. We still use wrapping forms because a bug in the
        // checks would otherwise manifest as a debug-mode panic that
        // diverges from the C engine's observed behaviour.
        n.wrapping_mul(x).wrapping_add(y)
    } else {
        set_arith_error(true);
        0
    }
}

// ---------------------------------------------------------------------------
// x_over_n
// ---------------------------------------------------------------------------

/// Integer division `x / n` with `tex_remainder` set to `x % n`,
/// following TeX's sign conventions. Sets `arith_error` (and returns
/// 0, with `tex_remainder = x`) on division by zero.
///
/// translated from xetex-scaledmath.c:79 (x_over_n)
#[no_mangle]
pub unsafe extern "C" fn x_over_n(mut x: i32, mut n: i32) -> i32 {
    // scaled_t is i32

    if n == 0 {
        set_arith_error(true);
        set_tex_remainder(x);
        return 0;
    }

    // Track the running remainder across sign flips — the C writes
    // this into the shared global between sign-flipping the inputs
    // and doing the divide. We read the current remainder via an
    // extra helper to match that behaviour.
    //
    // In the C body:
    //   if (n < 0) { x = -x; n = -n; tex_remainder = -tex_remainder; }
    // The `-tex_remainder` looks pointless (it's immediately
    // overwritten below), but we preserve it for faithfulness.
    if n < 0 {
        x = x.wrapping_neg();
        n = n.wrapping_neg();
        set_tex_remainder(crate::globals::tex_remainder().wrapping_neg());
    }

    if x >= 0 {
        set_tex_remainder(x % n);
        x / n
    } else {
        // Negating x may wrap (if x was i32::MIN), but that case is
        // still handled correctly: i32::MIN % positive_n and
        // i32::MIN / positive_n are both defined in Rust and match
        // C's behaviour after the implicit cast-through-wrap.
        let neg_x = x.wrapping_neg();
        set_tex_remainder((neg_x % n).wrapping_neg());
        (neg_x / n).wrapping_neg()
    }
}

// ---------------------------------------------------------------------------
// xn_over_d
// ---------------------------------------------------------------------------

/// Compute `x * n / d` using a 47-bit intermediate (low 15 bits + high
/// 32 bits) to avoid overflow on the multiply. `tex_remainder` is set
/// to the unsigned remainder, then sign-flipped to follow the sign of
/// `x`.
///
/// translated from xetex-scaledmath.c:105 (xn_over_d)
#[no_mangle]
pub unsafe extern "C" fn xn_over_d(mut x: i32, n: i32, d: i32) -> i32 {
    // scaled_t is i32

    let positive: bool;
    if x >= 0 {
        positive = true;
    } else {
        x = x.wrapping_neg();
        positive = false;
    }

    // 32768 == 2^15; the code splits x into high/low halves to keep
    // the n*half products' final *stored* values inside 32 bits. The
    // intermediates in C may transiently exceed i32 (since `32768L` is
    // `long`, which is 64-bit on Linux/macOS), but the result is
    // truncated into the i32 locals `t`/`u`/`v`. Rust `wrapping_*`
    // gives the identical truncated result deterministically on every
    // target.
    let t = (x % 32768).wrapping_mul(n);
    let mut u = (x / 32768).wrapping_mul(n).wrapping_add(t / 32768);
    let v = (u % d).wrapping_mul(32768).wrapping_add(t % 32768);

    if u / d >= 32768 {
        set_arith_error(true);
    } else {
        u = 32768i32.wrapping_mul(u / d).wrapping_add(v / d);
    }

    if positive {
        set_tex_remainder(v % d);
        u
    } else {
        set_tex_remainder((v % d).wrapping_neg());
        u.wrapping_neg()
    }
}

// ---------------------------------------------------------------------------
// round_xn_over_d
// ---------------------------------------------------------------------------

/// Like [`xn_over_d`] but rounds the quotient to the nearest integer
/// (ties away from zero). Does not touch `tex_remainder`.
///
/// translated from xetex-scaledmath.c:137 (round_xn_over_d)
#[no_mangle]
pub unsafe extern "C" fn round_xn_over_d(mut x: i32, n: i32, d: i32) -> i32 {
    // scaled_t is i32

    let positive: bool;
    if x >= 0 {
        positive = true;
    } else {
        x = x.wrapping_neg();
        positive = false;
    }

    // See `xn_over_d` for why we use `wrapping_*` here.
    let t = (x % 32768).wrapping_mul(n);
    let mut u = (x / 32768).wrapping_mul(n).wrapping_add(t / 32768);
    let v = (u % d).wrapping_mul(32768).wrapping_add(t % 32768);

    if u / d >= 32768 {
        set_arith_error(true);
    } else {
        u = 32768i32.wrapping_mul(u / d).wrapping_add(v / d);
    }

    let v = v % d;
    if v.wrapping_mul(2) >= d {
        u = u.wrapping_add(1);
    }

    if positive {
        u
    } else {
        u.wrapping_neg()
    }
}

// ---------------------------------------------------------------------------
// make_frac
// ---------------------------------------------------------------------------

/// Compute the fraction `p/q` in the range (-8, 8) as a 28-bit fixed-point
/// "frac" value (1.0 == 0x10000000). Sets `arith_error` and returns
/// ±0x7FFFFFFF if |p/q| >= 8.
///
/// translated from xetex-scaledmath.c:165 (make_frac)
#[no_mangle]
pub unsafe extern "C" fn make_frac(mut p: i32, mut q: i32) -> i32 {
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
        set_arith_error(true);
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

    if negative { -(f + n) } else { f + n }
}

// ---------------------------------------------------------------------------
// take_frac
// ---------------------------------------------------------------------------

/// Multiply `q` by the 28-bit frac `f` (1.0 == 0x10000000), returning
/// a scaled integer. Sets `arith_error` on overflow.
///
/// translated from xetex-scaledmath.c:220 (take_frac)
#[no_mangle]
pub unsafe extern "C" fn take_frac(mut q: i32, mut f: i32) -> i32 {
    let mut negative = f < 0;
    if negative {
        f = -f;
    }

    if q < 0 {
        q = -q;
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
            set_arith_error(true);
            0x7FFF_FFFF
        }
    };

    f += 0x1000_0000;
    let mut p = 0x0800_0000_i32;

    if q < 0x4000_0000 {
        loop {
            if f & 1 != 0 {
                p = (p + q) / 2;
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
                p = p + (q - p) / 2;
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
    if be_careful + p > 0 {
        set_arith_error(true);
        n = 0x7FFF_FFFF - p;
    }

    if negative { -(n + p) } else { n + p }
}

// ---------------------------------------------------------------------------
// ab_vs_cd
// ---------------------------------------------------------------------------

/// Compare `a*b` vs `c*d` without overflow, returning 1, 0, or -1.
///
/// translated from xetex-scaledmath.c:332 (ab_vs_cd)
#[no_mangle]
pub unsafe extern "C" fn ab_vs_cd(mut a: i32, mut b: i32, mut c: i32, mut d: i32) -> i32 {
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
            return if (a == 0 || b == 0) && (c == 0 || d == 0) { 0 } else { 1 };
        }

        if d == 0 {
            return if a == 0 { 0 } else { -1 };
        }

        // swap pairs and flip signs to bring into the main loop
        std::mem::swap(&mut a, &mut c);
        let tmp = -b; b = -d; d = tmp;
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

        a = b; b = q; c = d; d = r;
    }
}

// ---------------------------------------------------------------------------
// new_randoms
// ---------------------------------------------------------------------------

/// Advance the pseudo-random table by one generation (Fibonacci-lagged
/// subtraction generator).
///
/// translated from xetex-scaledmath.c:418 (new_randoms)
#[no_mangle]
pub unsafe extern "C" fn new_randoms() {
    for k in 0_usize..24 {
        let x = random_entry(k).wrapping_sub(random_entry(k + 31));
        set_random_entry(k, if x < 0 { x + 0x1000_0000 } else { x });
    }

    for k in 24_usize..55 {
        let x = random_entry(k).wrapping_sub(random_entry(k - 24));
        set_random_entry(k, if x < 0 { x + 0x1000_0000 } else { x });
    }

    set_j_random(54);
}

// ---------------------------------------------------------------------------
// init_randoms
// ---------------------------------------------------------------------------

/// Seed the pseudo-random table and run three warmup cycles.
///
/// translated from xetex-scaledmath.c:441 (init_randoms)
#[no_mangle]
pub unsafe extern "C" fn init_randoms(seed: i32) {
    let mut j = seed.abs();

    while j >= 0x1000_0000 {
        j /= 2;
    }

    let mut k = 1_i32;

    for i in 0_u8..55 {
        let jj = k;
        k = j - k;
        j = jj;
        if k < 0 {
            k += 0x1000_0000;
        }
        set_random_entry(((i as usize) * 21) % 55, j);
    }

    new_randoms();
    new_randoms();
    new_randoms();
}

// ---------------------------------------------------------------------------
// unif_rand
// ---------------------------------------------------------------------------

/// Return a pseudo-random integer uniformly distributed in
/// `[0, abs(x))` (or `(-abs(x), 0]` when `x < 0`).
///
/// translated from xetex-scaledmath.c:468 (unif_rand)
#[no_mangle]
pub unsafe extern "C" fn unif_rand(x: i32) -> i32 {
    if j_random() == 0 {
        new_randoms();
    } else {
        set_j_random(j_random() - 1);
    }

    let y = take_frac(x.abs(), random_entry(j_random() as usize));
    if y == x.abs() {
        0
    } else if x > 0 {
        y
    } else {
        -y
    }
}

// ---------------------------------------------------------------------------
// m_log
// ---------------------------------------------------------------------------

/// Natural logarithm in TeX's scaled-point units (result = ln(x/2^30) * 2^24).
///
/// translated from xetex-scaledmath.c:22 (m_log)
#[no_mangle]
pub unsafe extern "C" fn m_log(mut x: i32) -> i32 {
    if x <= 0 {
        error_here_with_diagnostic(c"Logarithm of ".as_ptr());
        print_scaled(x);
        print_cstr(c" has been replaced by 0".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, c"Since I don't take logs of non-positive numbers,".as_ptr());
        set_help_line(0, c"I'm zeroing this one. Proceed, with fingers crossed.".as_ptr());
        error();
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
        let mut z = ((x - 1) / two_to_the(k)) + 1;
        while x < 0x4000_0000 + z {
            z = (z + 1) / 2;
            k += 1;
        }
        y += spec_log(k);
        x -= z;
    }

    y / 8
}

// ---------------------------------------------------------------------------
// norm_rand
// ---------------------------------------------------------------------------

/// Return a pseudo-random integer drawn from a normal distribution.
///
/// translated from xetex-scaledmath.c:487 (norm_rand). Calls `m_log`.
#[no_mangle]
pub unsafe extern "C" fn norm_rand() -> i32 {
    loop {
        let (x, u);

        // inner do-while
        loop {
            if j_random() == 0 {
                new_randoms();
            } else {
                set_j_random(j_random() - 1);
            }
            let xi = take_frac(112429, random_entry(j_random() as usize) - 0x0800_0000);

            if j_random() == 0 {
                new_randoms();
            } else {
                set_j_random(j_random() - 1);
            }
            let ui = random_entry(j_random() as usize);

            if xi.abs() < ui {
                x = xi;
                u = ui;
                break;
            }
        }

        let xi = make_frac(x, u);
        let l = 139_548_960_i32 - m_log(u);
        if ab_vs_cd(1024, l, xi, xi) >= 0 {
            return xi;
        }
    }
}

// ===========================================================================
// tests
// ===========================================================================

#[cfg(test)]
mod tests {
    //! Unit tests for the pure arithmetic helpers.
    //!
    //! These tests touch the shared `arith_error` / `tex_remainder`
    //! globals, which is fine in test builds (the crate is not linked
    //! against the live engine in `cargo test -p tectonic_engine_xetex`
    //! alone — dependency graph means we bring in the C statics via
    //! `build.rs`, but the engine isn't *running*, so these globals
    //! are ours to trample for the duration of a test).
    //!
    //! We serialise on a mutex because every test in this module
    //! reads/writes the same globals.
    //!
    //! NOTE: we avoid `Mutex::lock().unwrap()` because a panicking
    //! assertion would poison the mutex for the rest of the run.

    use super::*;
    use crate::globals::{arith_error, set_arith_error, tex_remainder};
    use std::sync::Mutex;

    static GLOBALS_LOCK: Mutex<()> = Mutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        GLOBALS_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    // -----------------------------------------------------------------
    // tex_round
    // -----------------------------------------------------------------

    #[test]
    fn tex_round_basics() {
        unsafe {
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
    }

    #[test]
    fn tex_round_clips_to_i32_range() {
        unsafe {
            assert_eq!(tex_round(1e20), i32::MAX);
            assert_eq!(tex_round(-1e20), i32::MIN);
            assert_eq!(tex_round(2_147_483_648.0), i32::MAX);
            assert_eq!(tex_round(-2_147_483_649.0), i32::MIN);
        }
    }

    // -----------------------------------------------------------------
    // half
    // -----------------------------------------------------------------

    #[test]
    fn half_basics() {
        unsafe {
            assert_eq!(half(0), 0);
            assert_eq!(half(1), 1);
            assert_eq!(half(2), 1);
            assert_eq!(half(3), 2);
            assert_eq!(half(4), 2);
            assert_eq!(half(-1), 0); // (-1 + 1) / 2 == 0
            assert_eq!(half(-2), -1);
            assert_eq!(half(-3), -1); // (-3 + 1) / 2 == -1
            assert_eq!(half(-4), -2);
        }
    }

    // -----------------------------------------------------------------
    // mult_and_add
    // -----------------------------------------------------------------

    #[test]
    fn mult_and_add_basic() {
        let _g = lock();
        unsafe {
            // TeX calls `mult_and_add` with `max_answer = MAX_HALFWORD`
            // (0x3FFFFFFF) in practice — the i32::MAX value exceeds the
            // algorithm's internal precondition that `max_answer ± y`
            // still fits in i32, so we pick a realistic cap here.
            const MAX: i32 = 0x3FFF_FFFF;

            set_arith_error(false);
            assert_eq!(mult_and_add(3, 7, 1, MAX), 22);
            assert!(!arith_error());

            assert_eq!(mult_and_add(0, 7, 42, MAX), 42);
            assert!(!arith_error());

            // Negative n flips the sign of x and continues.
            assert_eq!(mult_and_add(-2, 3, 1, MAX), -5);
            assert!(!arith_error());
        }
    }

    #[test]
    fn mult_and_add_detects_overflow() {
        let _g = lock();
        unsafe {
            set_arith_error(false);
            // Chosen so that n*x + y > max_answer => arith_error.
            let r = mult_and_add(2, 2_000_000_000, 1_000_000_000, i32::MAX);
            assert_eq!(r, 0);
            assert!(arith_error());
        }
    }

    // -----------------------------------------------------------------
    // x_over_n
    // -----------------------------------------------------------------

    #[test]
    fn x_over_n_positive() {
        let _g = lock();
        unsafe {
            set_arith_error(false);
            assert_eq!(x_over_n(17, 5), 3);
            assert_eq!(tex_remainder(), 2);
            assert!(!arith_error());
        }
    }

    #[test]
    fn x_over_n_negative_x_gives_negative_remainder() {
        let _g = lock();
        unsafe {
            set_arith_error(false);
            assert_eq!(x_over_n(-17, 5), -3);
            assert_eq!(tex_remainder(), -2);
            assert!(!arith_error());
        }
    }

    #[test]
    fn x_over_n_zero_divisor_sets_flag() {
        let _g = lock();
        unsafe {
            set_arith_error(false);
            assert_eq!(x_over_n(42, 0), 0);
            assert_eq!(tex_remainder(), 42);
            assert!(arith_error());
        }
    }

    // -----------------------------------------------------------------
    // xn_over_d
    // -----------------------------------------------------------------

    #[test]
    fn xn_over_d_small_values() {
        let _g = lock();
        unsafe {
            set_arith_error(false);
            // 10 * 3 / 4 = 7.5 → truncated to 7, remainder 2
            assert_eq!(xn_over_d(10, 3, 4), 7);
            assert_eq!(tex_remainder(), 2);
            assert!(!arith_error());
        }
    }

    #[test]
    fn xn_over_d_large_x_and_divisor() {
        let _g = lock();
        unsafe {
            // The C algorithm is only well-defined when
            // `(x / 32768) / d < 32768`, i.e., `x < d * 2^30`. With a
            // moderately-large `d`, this leaves plenty of headroom.
            set_arith_error(false);
            // Exact: 1_000_000_000 * 3 / 1000 = 3_000_000
            assert_eq!(xn_over_d(1_000_000_000, 3, 1000), 3_000_000);
            assert!(!arith_error());
            // Remainder: 1_000_000_000 % 1000 == 0, so no remainder.
            assert_eq!(tex_remainder(), 0);
        }
    }

    #[test]
    fn xn_over_d_detects_overflow() {
        let _g = lock();
        unsafe {
            // With d=1, `u/d == u == x/32768` must stay below 32768,
            // i.e. x < 2^30. i32::MAX exceeds this bound and triggers
            // the arith_error path; the returned `u` is the pre-final
            // value (still 65535 after the high-half split). We don't
            // check the numeric return — only that the flag got set,
            // matching the C behaviour.
            set_arith_error(false);
            let _ = xn_over_d(i32::MAX, 1, 1);
            assert!(arith_error());
        }
    }

    #[test]
    fn xn_over_d_negative_x_negates_result_and_remainder() {
        let _g = lock();
        unsafe {
            set_arith_error(false);
            assert_eq!(xn_over_d(-10, 3, 4), -7);
            assert_eq!(tex_remainder(), -2);
        }
    }

    // -----------------------------------------------------------------
    // round_xn_over_d
    // -----------------------------------------------------------------

    #[test]
    fn round_xn_over_d_rounds_half_up() {
        let _g = lock();
        unsafe {
            set_arith_error(false);
            // 10 * 3 / 4 = 7.5 → rounds to 8
            assert_eq!(round_xn_over_d(10, 3, 4), 8);
            // 10 * 3 / 5 = 6.0
            assert_eq!(round_xn_over_d(10, 3, 5), 6);
            // 10 * 2 / 4 = 5.0 (exact, no rounding)
            assert_eq!(round_xn_over_d(10, 2, 4), 5);
        }
    }

    #[test]
    fn round_xn_over_d_negative_x() {
        let _g = lock();
        unsafe {
            set_arith_error(false);
            // -10 * 3 / 4 = -7.5 → rounds away from zero to -8
            assert_eq!(round_xn_over_d(-10, 3, 4), -8);
        }
    }
}

// ---------------------------------------------------------------------------
// add_or_sub / quotient / fract
// ---------------------------------------------------------------------------

/// Add or subtract two scaled values, setting arith_error on overflow.
///
/// translated from xetex-xetex0.c:6281 (add_or_sub)
#[no_mangle]
pub unsafe extern "C" fn add_or_sub(x: i32, y: i32, max_answer: i32, negative: bool) -> i32 {
    let y = if negative { -y } else { y };
    if x >= 0 {
        if y <= max_answer - x {
            x + y
        } else {
            set_arith_error(true);
            0
        }
    } else if y >= -max_answer - x {
        x + y
    } else {
        set_arith_error(true);
        0
    }
}

/// Rounded integer quotient n/d, setting arith_error on divide-by-zero.
///
/// translated from xetex-xetex0.c:6305 (quotient)
#[no_mangle]
pub unsafe extern "C" fn quotient(mut n: i32, mut d: i32) -> i32 {
    if d == 0 {
        set_arith_error(true);
        return 0;
    }
    let mut negative = if d > 0 { false } else { d = -d; true };
    if n < 0 { n = -n; negative = !negative; }
    let mut a = n / d;
    n -= a * d;
    d = n - d;
    if d + n >= 0 { a += 1; }
    if negative { -a } else { a }
}

/// Compute x * n / d clamped to max_answer, setting arith_error on overflow.
///
/// translated from xetex-xetex0.c:6336 (fract)
#[no_mangle]
pub unsafe extern "C" fn fract(mut x: i32, mut n: i32, mut d: i32, max_answer: i32) -> i32 {
    if d == 0 { set_arith_error(true); return 0; }
    let mut negative = if d > 0 { false } else { d = -d; true };
    if x < 0 { x = -x; negative = !negative; } else if x == 0 { return 0; }
    if n < 0 { n = -n; negative = !negative; }
    let t = n / d;
    if t > max_answer / x { set_arith_error(true); return 0; }
    let mut a = t * x;
    n -= t * d;
    // If n == 0, skip straight to `found` (return with sign).
    let a = if n != 0 {
        let t2 = x / d;
        if t2 > (max_answer - a) / n { set_arith_error(true); return 0; }
        a += t2 * n;
        x -= t2 * d;
        // If x == 0, skip to `found`.
        if x != 0 {
            if x < n { let tmp = x; x = n; n = tmp; }
            let mut f = 0i32;
            let mut r = (d / 2) - d;
            let h = -r;
            // The while(true) loop; all `goto found1` become `break`.
            loop {
                if n & 1 != 0 {
                    r += x;
                    if r >= 0 { r -= d; f += 1; }
                }
                n /= 2;
                if n == 0 { break; }
                if x < h {
                    x += x;
                } else {
                    let tmp = x - d;
                    x = tmp + x;
                    f += n;
                    if x < n {
                        if x == 0 { break; }
                        let tmp = x; x = n; n = tmp;
                    }
                }
            }
            // found1:
            if f > max_answer - a { set_arith_error(true); return 0; }
            a + f
        } else {
            a // x == 0: goto found
        }
    } else {
        a // n == 0: goto found
    };
    // found:
    if negative { -a } else { a }
}
