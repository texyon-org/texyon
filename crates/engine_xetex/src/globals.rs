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

/// Rust mirror of `input_state_t` (xetex-xetexd.h:344–352).
/// Must stay layout-compatible with the C struct (`#[repr(C)]`).
#[repr(C)]
pub(crate) struct InputState {
    pub state: u16,
    pub index: u16,
    pub start: i32,
    pub loc: i32,
    pub limit: i32,
    pub name: i32,
    pub synctex_tag: i32,
}

mod raw {
    use crate::mem_types::MemoryWord;

    extern "C" {
        // xetex-xetexd.h:452
        pub(super) static mut arith_error: bool;
        // xetex-xetexd.h:453 — `scaled_t` is `int32_t` (xetex-core.h:40).
        pub(super) static mut tex_remainder: i32;
        // xetex-xetexd.h:432 — digit scratch buffer for print_int / print_hex.
        pub(super) static mut dig: [u8; 23];

        // xetex-xetexd.h:454 — array of 55 pseudo-random fractions in
        // [0, 0x10000000).
        pub(super) static mut randoms: [i32; 55];
        // xetex-xetexd.h:455 — index into `randoms`; `unsigned char` maps to u8.
        pub(super) static mut j_random: u8;

        // xetex-xetexd.h:506 — current input state; layout mirrors InputState above.
        pub(super) static mut cur_input: super::InputState;
        // xetex-xetexd.h:510 — current source line number.
        pub(super) static mut line: i32;

        // xetex-xetexd.h:386 — equivalents table (size eqtb_top+1 words).
        // Accessed by index like `mem`; valid indices depend on context.
        pub(super) static mut eqtb: *mut crate::mem_types::MemoryWord;
        // xetex-xetexd.h:459 — temporary pointer register.
        pub(super) static mut temp_ptr: i32;

        // Memory arena globals (xetex-xetexd.h:460–466).
        // Allocated by C (`xmalloc_array`) during engine init; valid indices
        // are 0 .. =MEM_TOP (4999999). TEX_NULL (-268435455) is never used
        // as an index — it is a sentinel that callers check before indexing.
        pub(super) static mut mem: *mut MemoryWord;
        pub(super) static mut lo_mem_max: i32;
        pub(super) static mut hi_mem_min: i32;
        pub(super) static mut avail: i32;
        pub(super) static mut mem_end: i32;
        pub(super) static mut rover: i32;

        // Output selector (xetex-xetexd.h:431). C type `selector_t` is an
        // int-sized enum; values range 0–21 (see xetex-core.h).
        pub(super) static mut selector: i32;
        // True while processing a \special (xetex-xetexd.h:439).
        pub(super) static mut doing_special: bool;
        // Print position counters (xetex-xetexd.h:434–435).
        pub(super) static mut term_offset: i32;
        pub(super) static mut file_offset: i32;

        // Output file handles (xetex-xetexd.h:429–430, 691).
        // `rust_output_handle_t` is `uintptr_t` (tectonic_bridge_core_generated.h:9).
        pub(super) static mut rust_stdout: usize;
        pub(super) static mut log_file: usize;
        pub(super) static mut write_file: [usize; 16];

        // Current diagnostic capture handle (xetex-output.c, un-static'd session 8).
        // Null (0) means no active capture. Type is opaque `ttbc_diagnostic_t *`.
        pub(super) static mut current_diagnostic: *mut std::ffi::c_void;

        // Print line-length limit (xetex-xetexd.h:399).
        pub(super) static mut max_print_line: i32;
        // Tally of characters output (xetex-xetexd.h:433).
        pub(super) static mut tally: i32;
        // Error-line display helpers (xetex-xetexd.h:397, 436–437).
        pub(super) static mut error_line: i32;
        pub(super) static mut trick_buf: [u16; 256];
        pub(super) static mut trick_count: i32;
        // String pool (xetex-xetexd.h:403, 423, 425).
        // `packed_UTF16_code` = `unsigned short`; `pool_pointer` = `int32_t`.
        pub(super) static mut pool_size: i32;
        pub(super) static mut str_pool: *mut u16;
        pub(super) static mut pool_ptr: i32;
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

// ---------------------------------------------------------------------------
// mem arena word accessors
// ---------------------------------------------------------------------------

/// Read `mem[p].b32.s0` (the first 32-bit word / node-size / type field).
#[inline]
pub(crate) fn mem_b32s0(p: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures p is in-bounds.
    unsafe { (*raw::mem.offset(p as isize)).b32.s0 }
}

/// Write `v` into `mem[p].b32.s0`.
#[inline]
pub(crate) fn set_mem_b32s0(p: i32, v: i32) {
    // SAFETY: see module-level doc. Caller ensures p is in-bounds.
    unsafe {
        (*raw::mem.offset(p as isize)).b32.s0 = v;
    }
}

/// Read `mem[p].b32.s1` (the `LLIST_link` / successor-pointer field).
///
/// # Safety
/// `p` must be a valid arena index (`0 ..= MEM_TOP`). TEX_NULL is never
/// a valid index; callers must guard against it before calling this.
#[inline]
pub(crate) fn mem_b32s1(p: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures p is in-bounds.
    unsafe { (*raw::mem.offset(p as isize)).b32.s1 }
}

/// Write `v` into `mem[p].b32.s1`.
#[inline]
pub(crate) fn set_mem_b32s1(p: i32, v: i32) {
    // SAFETY: see module-level doc. Caller ensures p is in-bounds.
    unsafe {
        (*raw::mem.offset(p as isize)).b32.s1 = v;
    }
}

// ---------------------------------------------------------------------------
// lo_mem_max / hi_mem_min / avail / mem_end / rover
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn lo_mem_max() -> i32 {
    unsafe { raw::lo_mem_max }
}

#[inline]
#[allow(dead_code)]
pub(crate) fn set_lo_mem_max(v: i32) {
    unsafe { raw::lo_mem_max = v; }
}

#[inline]
pub(crate) fn hi_mem_min() -> i32 {
    unsafe { raw::hi_mem_min }
}

#[inline]
pub(crate) fn set_hi_mem_min(v: i32) {
    unsafe { raw::hi_mem_min = v; }
}

#[inline]
pub(crate) fn avail() -> i32 {
    unsafe { raw::avail }
}

#[inline]
pub(crate) fn set_avail(v: i32) {
    unsafe { raw::avail = v; }
}

#[inline]
pub(crate) fn mem_end() -> i32 {
    unsafe { raw::mem_end }
}

#[inline]
pub(crate) fn set_mem_end(v: i32) {
    unsafe { raw::mem_end = v; }
}

#[inline]
pub(crate) fn rover() -> i32 {
    unsafe { raw::rover }
}

#[inline]
pub(crate) fn set_rover(v: i32) {
    unsafe { raw::rover = v; }
}

// ---------------------------------------------------------------------------
// cur_input / line
// ---------------------------------------------------------------------------

/// Returns `cur_input.synctex_tag`.
#[inline]
pub(crate) fn cur_input_synctex_tag() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::cur_input.synctex_tag }
}

/// Returns the current source line number.
#[inline]
pub(crate) fn line() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::line }
}

// ---------------------------------------------------------------------------
// mem word-level accessors (b16 and gr fields)
// ---------------------------------------------------------------------------

// Layout (little-endian): one MemoryWord = 8 bytes.
//   b32.s0 == bytes [0,4): b16.s0 (bytes 0-1) | b16.s1 (bytes 2-3)
//   b32.s1 == bytes [4,8): b16.s2 (bytes 4-5) | b16.s3 (bytes 6-7)
//   gr           == all 8 bytes as f64

/// Read `mem[p].b16.s0` (low 16 bits of the first 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s0(p: i32) -> u16 {
    unsafe { (*raw::mem.offset(p as isize)).b16.s0 }
}
/// Write `v` into `mem[p].b16.s0`.
#[inline]
pub(crate) fn set_mem_b16s0(p: i32, v: u16) {
    unsafe { (*raw::mem.offset(p as isize)).b16.s0 = v; }
}

/// Read `mem[p].b16.s1` (high 16 bits of the first 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s1(p: i32) -> u16 {
    unsafe { (*raw::mem.offset(p as isize)).b16.s1 }
}
/// Write `v` into `mem[p].b16.s1`.
#[inline]
pub(crate) fn set_mem_b16s1(p: i32, v: u16) {
    unsafe { (*raw::mem.offset(p as isize)).b16.s1 = v; }
}

/// Read the `gr` (f64) field of `mem[p]`.
#[inline]
#[allow(dead_code)]
pub(crate) fn mem_gr(p: i32) -> f64 {
    unsafe { (*raw::mem.offset(p as isize)).gr }
}
/// Write the `gr` (f64) field of `mem[p]`.
#[inline]
pub(crate) fn set_mem_gr(p: i32, v: f64) {
    unsafe { (*raw::mem.offset(p as isize)).gr = v; }
}

/// Read the `ptr` (void*) field of `mem[p]`.
#[inline]
pub(crate) fn mem_ptr(p: i32) -> *mut std::ffi::c_void {
    unsafe { (*raw::mem.offset(p as isize)).ptr }
}
/// Write the `ptr` (void*) field of `mem[p]`.
#[inline]
pub(crate) fn set_mem_ptr(p: i32, v: *mut std::ffi::c_void) {
    unsafe { (*raw::mem.offset(p as isize)).ptr = v; }
}

/// Copy the full MemoryWord at `mem[p]`.
#[inline]
pub(crate) fn mem_word(p: i32) -> crate::mem_types::MemoryWord {
    unsafe { *raw::mem.offset(p as isize) }
}
/// Write a full MemoryWord to `mem[p]`.
#[inline]
pub(crate) fn set_mem_word(p: i32, w: crate::mem_types::MemoryWord) {
    unsafe { *raw::mem.offset(p as isize) = w; }
}

// ---------------------------------------------------------------------------
// eqtb accessor
// ---------------------------------------------------------------------------

/// Read `eqtb[p].b32.s1` (the "value" half of an equivalents entry).
#[inline]
pub(crate) fn eqtb_b32s1(p: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures p is a valid eqtb index.
    unsafe { (*raw::eqtb.offset(p as isize)).b32.s1 }
}

// ---------------------------------------------------------------------------
// temp_ptr
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn temp_ptr() -> i32 {
    unsafe { raw::temp_ptr }
}
#[inline]
pub(crate) fn set_temp_ptr(v: i32) {
    unsafe { raw::temp_ptr = v; }
}

// ---------------------------------------------------------------------------
// dig
// ---------------------------------------------------------------------------

/// Read `dig[k]`, the k-th digit scratch byte (xetex-xetexd.h:432).
#[inline]
pub(crate) fn dig_entry(k: usize) -> u8 {
    unsafe { raw::dig[k] }
}

/// Write `v` into `dig[k]`.
#[inline]
pub(crate) fn set_dig_entry(k: usize, v: u8) {
    unsafe { raw::dig[k] = v; }
}

// ---------------------------------------------------------------------------
// selector / term_offset / file_offset
// ---------------------------------------------------------------------------

/// Returns the current output selector (xetex-xetexd.h:431).
#[inline]
pub(crate) fn selector() -> i32 {
    unsafe { raw::selector }
}

/// Returns true while the engine is processing a `\special` (xetex-xetexd.h:439).
#[inline]
pub(crate) fn doing_special() -> bool {
    unsafe { raw::doing_special }
}

/// Returns `term_offset` (columns written to terminal since last newline).
#[inline]
pub(crate) fn term_offset() -> i32 {
    unsafe { raw::term_offset }
}
/// Sets `term_offset`.
#[inline]
pub(crate) fn set_term_offset(v: i32) {
    unsafe { raw::term_offset = v; }
}

/// Returns `file_offset` (columns written to log file since last newline).
#[inline]
pub(crate) fn file_offset() -> i32 {
    unsafe { raw::file_offset }
}
/// Sets `file_offset`.
#[inline]
pub(crate) fn set_file_offset(v: i32) {
    unsafe { raw::file_offset = v; }
}

// ---------------------------------------------------------------------------
// rust_stdout / log_file / write_file / current_diagnostic
// ---------------------------------------------------------------------------

/// Returns the `rust_stdout` output handle.
#[inline]
pub(crate) fn rust_stdout() -> usize {
    unsafe { raw::rust_stdout }
}

/// Returns the `log_file` output handle.
#[inline]
pub(crate) fn log_file() -> usize {
    unsafe { raw::log_file }
}

/// Returns `write_file[i]`, the output handle for write-stream `i` (0–15).
#[inline]
pub(crate) fn write_file(i: usize) -> usize {
    unsafe { raw::write_file[i] }
}

/// Returns the current diagnostic capture pointer (null = no active capture).
#[inline]
pub(crate) fn current_diagnostic() -> *mut std::ffi::c_void {
    unsafe { raw::current_diagnostic }
}

// ---------------------------------------------------------------------------
// print_raw_char globals
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn max_print_line() -> i32 {
    unsafe { raw::max_print_line }
}

#[inline]
pub(crate) fn tally() -> i32 {
    unsafe { raw::tally }
}
#[inline]
pub(crate) fn set_tally(v: i32) {
    unsafe { raw::tally = v; }
}

#[inline]
pub(crate) fn error_line() -> i32 {
    unsafe { raw::error_line }
}

/// Read `trick_buf[k]`.
#[inline]
pub(crate) fn trick_count() -> i32 {
    unsafe { raw::trick_count }
}
/// Write `trick_buf[k] = v`.
#[inline]
pub(crate) fn set_trick_buf(k: usize, v: u16) {
    unsafe { raw::trick_buf[k] = v; }
}

#[inline]
pub(crate) fn pool_size() -> i32 {
    unsafe { raw::pool_size }
}
#[inline]
pub(crate) fn pool_ptr() -> i32 {
    unsafe { raw::pool_ptr }
}
#[inline]
pub(crate) fn set_pool_ptr(v: i32) {
    unsafe { raw::pool_ptr = v; }
}
/// Write `str_pool[k] = v`.
#[inline]
pub(crate) fn set_str_pool(k: i32, v: u16) {
    unsafe { *raw::str_pool.offset(k as isize) = v; }
}
