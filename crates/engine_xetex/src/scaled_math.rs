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

use crate::globals::{set_arith_error, set_tex_remainder};

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
