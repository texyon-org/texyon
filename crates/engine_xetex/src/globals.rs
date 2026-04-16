//! Centralized bindings to XeTeX's C globals.
//!
//! During Phase A of the C→Rust migration (see `plan/02-state.md`),
//! translated Rust functions read and write the engine's global state
//! through this module rather than declaring `extern "C"` statics at
//! their call sites. This keeps the `unsafe` blob small, ensures we
//! audit aliasing in one place, and gives us a trivial rename target
//! once state is threaded through `EngineState` in Phase B.
//!
//! ## Safety invariant for this entire module
//!
//! The tectonic engine holds a process-wide mutex (`bridge_core`'s
//! global engine lock) while a (Xe)TeX invocation is in flight, so
//! there are no concurrent readers or writers of any global declared
//! here. Every accessor therefore performs a single read-and-return
//! or write-and-commit: we **never** hand out a live `&` / `&mut`
//! reference that could overlap another call. In particular, do not
//! introduce a helper that returns `&'static mut` — two calls would
//! alias and that's insta-UB.
//!
//! ## Scope
//!
//! Only the globals that have actual Rust callers live here. Growing
//! this file on spec is busywork (there are ~150 globals in
//! `xetex-xetexd.h`). Add a declaration the first time a Rust function
//! needs it.

// ---------------------------------------------------------------------------
// extern bindings
// ---------------------------------------------------------------------------
//
// Raw extern statics are hidden in this private submodule so their names
// don't collide with the safe wrappers of the same name exported below.

mod raw {
    extern "C" {
        // xetex-xetexd.h:452
        pub(super) static mut arith_error: bool;
        // xetex-xetexd.h:453 — `scaled_t` is `int32_t` (xetex-core.h:40).
        pub(super) static mut tex_remainder: i32;
        // xetex-xetexd.h:454 — array of 55 pseudo-random fractions in
        // [0, 0x10000000).
        pub(super) static mut randoms: [i32; 55];
        // xetex-xetexd.h:455 — index into `randoms`; `unsigned char` maps to u8.
        pub(super) static mut j_random: u8;
    }
}

// ---------------------------------------------------------------------------
// arith_error
// ---------------------------------------------------------------------------

/// Returns the current value of the engine's `arith_error` flag.
#[inline]
#[allow(dead_code)] // used in unit tests and by future Rust callers of
                    // `mult_and_add` / `xn_over_d`.
pub(crate) fn arith_error() -> bool {
    // SAFETY: see module-level doc.
    unsafe { raw::arith_error }
}

/// Sets the engine's `arith_error` flag.
#[inline]
pub(crate) fn set_arith_error(v: bool) {
    // SAFETY: see module-level doc.
    unsafe {
        raw::arith_error = v;
    }
}

// ---------------------------------------------------------------------------
// tex_remainder
// ---------------------------------------------------------------------------

/// Returns the `tex_remainder` output register of `x_over_n` /
/// `xn_over_d`.
#[inline]
#[allow(dead_code)] // used in unit tests and by future Rust callers.
pub(crate) fn tex_remainder() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::tex_remainder }
}

/// Stores a value into the `tex_remainder` register.
#[inline]
pub(crate) fn set_tex_remainder(v: i32) {
    // SAFETY: see module-level doc.
    unsafe {
        raw::tex_remainder = v;
    }
}

// ---------------------------------------------------------------------------
// randoms / j_random
// ---------------------------------------------------------------------------

/// Returns `randoms[k]`, the k-th element of the pseudo-random table.
///
/// Panics in debug mode if `k >= 55`.
#[inline]
pub(crate) fn random_entry(k: usize) -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::randoms[k] }
}

/// Sets `randoms[k]` to `v`.
#[inline]
pub(crate) fn set_random_entry(k: usize, v: i32) {
    // SAFETY: see module-level doc.
    unsafe {
        raw::randoms[k] = v;
    }
}

/// Returns the current `j_random` index.
#[inline]
pub(crate) fn j_random() -> u8 {
    // SAFETY: see module-level doc.
    unsafe { raw::j_random }
}

/// Sets `j_random`.
#[inline]
pub(crate) fn set_j_random(v: u8) {
    // SAFETY: see module-level doc.
    unsafe {
        raw::j_random = v;
    }
}
