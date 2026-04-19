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
#[derive(Copy, Clone)]
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

        // String pool (xetex-xetexd.h:424–426).
        // str_start[i] = pool offset of the i-th string (pool_pointer = i32).
        pub(super) static mut str_start: *mut i32;
        // str_ptr = number of strings in the pool (str_number = i32).
        pub(super) static mut str_ptr: i32;
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

        // Input-level depth (xetex-xetexd.h:507).
        pub(super) static mut in_open: i32;
        // Pointer to the filename-stack array (xetex-xetexd.h:513, str_number* = *mut i32).
        pub(super) static mut full_source_filename_stack: *mut i32;
        // Pointer to the line-number-stack array (xetex-xetexd.h:511, *mut i32).
        pub(super) static mut line_stack: *mut i32;
        // Non-zero when file:line error format is active (xetex-xetexd.h:416).
        pub(super) static mut file_line_error_style_p: i32;

        // Hash table: `b32x2 *hash` (xetex-xetexd.h:482).
        pub(super) static mut hash: *mut crate::mem_types::B32x2;
        // Hash table management globals (xetex-xetexd.h:483–488).
        pub(super) static mut hash_used: i32;
        pub(super) static mut hash_extra: i32;
        pub(super) static mut hash_high: i32;
        pub(super) static mut no_new_control_sequence: bool;
        // Primitive hash table: `b32x2 prim[PRIM_SIZE + 1]` (xetex-xetexd.h:490).
        pub(super) static mut prim: [crate::mem_types::B32x2; 2101];
        // prim_used: next free slot in prim (xetex-xetexd.h:491).
        pub(super) static mut prim_used: i32;
        // Top of the eqtb array (xetex-xetexd.h:486).
        pub(super) static mut eqtb_top: i32;

        // String pool size limits (xetex-xetexd.h:400,427–428).
        pub(super) static mut max_strings: i32;
        pub(super) static mut init_str_ptr: i32;
        pub(super) static mut init_pool_ptr: i32;

        // Input buffer (xetex-xetexd.h:392); UnicodeScalar = int32_t.
        pub(super) static mut buffer: *mut i32;

        // Error-help machinery (xetex-xetexd.h:449–450).
        pub(super) static mut help_line: [*const libc::c_char; 6];
        pub(super) static mut help_ptr: u8;

        // Scaled-math tables (xetex-xetexd.h:457–458).
        pub(super) static mut two_to_the: [i32; 31];
        pub(super) static mut spec_log: [i32; 29];

        // Error / interaction state (xetex-xetexd.h:444–452, 551–552).
        // `interaction` is `unsigned char`; mode values: BATCH=0,SCROLL=2,ERROR_STOP=3.
        pub(super) static mut interaction: u8;
        // `history` is `tt_history_t` (int-sized enum, values 0–3).
        pub(super) static mut history: i32;
        pub(super) static mut error_count: i8;
        pub(super) static mut use_err_help: bool;
        pub(super) static mut halt_on_error_p: i32;
        // `log_opened` and `job_name` (xetex-xetexd.h:551–552).
        pub(super) static mut log_opened: bool;
        pub(super) static mut job_name: i32;

        // Engine config flags set before launch (xetex-xetexd.h:385,396,722,724).
        pub(super) static mut in_initex_mode: bool;
        pub(super) static mut synctex_enabled: i32;
        pub(super) static mut semantic_pagination_enabled: bool;
        pub(super) static mut shell_escape_enabled: bool;

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

        // Name-of-file globals (xetex-xetexd.h:388–391).
        // name_of_file: current input filename (NUL-terminated C string).
        pub(super) static mut name_of_file: *mut libc::c_char;
        // name_length: byte length of name_of_file (excluding NUL).
        pub(super) static mut name_length: i32;
        // name_of_file16: UTF-16 encoding of name_of_file (xetex-xetexd.h:389).
        pub(super) static mut name_of_file16: *mut u16;
        // name_length16: number of UTF-16 code units in name_of_file16.
        pub(super) static mut name_length16: i32;

        // Saved selector value used by begin_diagnostic/end_diagnostic (xetex-xetexd.h:481).
        pub(super) static mut old_setting: u8;
        // SyncTeX timing base (xetex-xetexd.h:601–602).
        pub(super) static mut epochseconds: i32;
        pub(super) static mut microseconds: i32;
        // Font name table (xetex-xetexd.h:562; str_number = i32).
        pub(super) static mut font_name: *mut i32;
        // Current character code (xetex-xetexd.h:500).
        pub(super) static mut cur_chr: i32;
        // Current command code (`eight_bits` = u8, xetex-xetexd.h:499).
        pub(super) static mut cur_cmd: u8;
        // Mark registers: cur_mark[0..4] (xetex-xetexd.h:527; str_number = i32).
        pub(super) static mut cur_mark: [i32; 5];
        // Current parsed integer value (xetex-xetexd.h:530).
        pub(super) static mut cur_val: i32;
        // Parsed file-name components (xetex-xetexd.h:542–544; str_number = i32).
        pub(super) static mut cur_name: i32;
        pub(super) static mut cur_area: i32;
        pub(super) static mut cur_ext: i32;
        // True while a filename is being accumulated (xetex-xetexd.h:550).
        pub(super) static mut name_in_progress: bool;
        // Controls whether more_name stops at spaces (xetex-xetexd.h:718).
        pub(super) static mut stop_at_space: bool;
        // Magnification as actually set this job (xetex-xetexd.h:498; 0 = not set yet).
        pub(super) static mut mag_set: i32;

        // Maximum font id (xetex-xetexd.h:406).
        pub(super) static mut font_max: i32;

        // Input state stack (xetex-xetexd.h:503–504; layout mirrors InputState).
        pub(super) static mut input_stack: *mut super::InputState;
        pub(super) static mut input_ptr: i32;
        // Base pointer into input_stack (xetex-xetexd.h:521).
        pub(super) static mut base_ptr: i32;

        // Group-stack: grp_stack[i] = save-stack boundary for level i
        // (xetex-xetexd.h:705; save_pointer = i32).
        pub(super) static mut grp_stack: *mut i32;
        // If-stack: if_stack[i] = cond_ptr value at nesting level i
        // (xetex-xetexd.h:706).
        pub(super) static mut if_stack: *mut i32;

        // Conditional state (xetex-xetexd.h:537–540).
        pub(super) static mut cond_ptr: i32;
        // if_limit: unsigned char (xetex-xetexd.h:538).
        pub(super) static mut if_limit: u8;
        // cur_if: small_number = i16 (xetex-xetexd.h:539).
        pub(super) static mut cur_if: i16;
        pub(super) static mut if_line: i32;

        // Semantic nest (mode stack, xetex-xetexd.h:476–479,413).
        pub(super) static mut nest: *mut crate::mem_types::ListStateRecord;
        pub(super) static mut nest_ptr: i32;
        pub(super) static mut max_nest_stack: i32;
        pub(super) static mut nest_size: i32;
        // Current list state (xetex-xetexd.h:479; direct struct, not a pointer).
        pub(super) static mut cur_list: crate::mem_types::ListStateRecord;
        // File-open states for \read streams (xetex-xetexd.h:536; CLOSED=2, JUST_OPEN=1).
        pub(super) static mut read_open: [u8; 17];
        // Array of UFILE* for \read streams (xetex-xetexd.h:535).
        // Declared as *mut c_void to avoid a cyclic type dependency with io.rs.
        pub(super) static mut read_file: [*mut libc::c_void; 16];

        // Save-stack (xetex-xetexd.h:492–494).
        pub(super) static mut save_stack: *mut crate::mem_types::MemoryWord;
        pub(super) static mut save_ptr: i32;
        pub(super) static mut max_save_stack: i32;
        pub(super) static mut save_size: i32;
        // Grouping level / group-code (xetex-xetexd.h:495–496).
        pub(super) static mut cur_level: u16;
        // group_code is unsigned char (xetex-xetexd.h:325).
        pub(super) static mut cur_group: u8;
        // Current group boundary in the save stack (xetex-xetexd.h:497).
        pub(super) static mut cur_boundary: i32;
        // XEQ_LEVEL array — level at which each eqtb entry was defined
        // (xetex-xetexd.h:733; size = EQTB_SIZE - INT_BASE + 1 = 1114730).
        pub(super) static mut _xeq_level_array: [u16; 1_114_730];
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
    // SAFETY: see module-level doc.
    unsafe { raw::lo_mem_max }
}

#[inline]
#[allow(dead_code)]
pub(crate) fn set_lo_mem_max(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::lo_mem_max = v; }
}

#[inline]
pub(crate) fn hi_mem_min() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::hi_mem_min }
}

#[inline]
pub(crate) fn set_hi_mem_min(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::hi_mem_min = v; }
}

#[inline]
pub(crate) fn avail() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::avail }
}

#[inline]
pub(crate) fn set_avail(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::avail = v; }
}

#[inline]
pub(crate) fn mem_end() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::mem_end }
}

#[inline]
pub(crate) fn set_mem_end(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::mem_end = v; }
}

#[inline]
pub(crate) fn rover() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::rover }
}

#[inline]
pub(crate) fn set_rover(v: i32) {
    // SAFETY: see module-level doc.
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

/// Returns a copy of the entire `cur_input` input-state struct.
/// Used to save the current state onto `input_stack`.
#[inline]
pub(crate) fn cur_input() -> InputState {
    // SAFETY: see module-level doc.  InputState is Copy.
    unsafe { raw::cur_input }
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
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).b16.s0 }
}
/// Write `v` into `mem[p].b16.s0`.
#[inline]
pub(crate) fn set_mem_b16s0(p: i32, v: u16) {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).b16.s0 = v; }
}

/// Read `mem[p].b16.s1` (high 16 bits of the first 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s1(p: i32) -> u16 {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).b16.s1 }
}
/// Write `v` into `mem[p].b16.s1`.
#[inline]
pub(crate) fn set_mem_b16s1(p: i32, v: u16) {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).b16.s1 = v; }
}
/// Read `mem[p].b16.s2` (low 16 bits of the second 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s2(p: i32) -> u16 {
    unsafe { (*raw::mem.offset(p as isize)).b16.s2 }
}
/// Read `mem[p].b16.s3` (high 16 bits of the second 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s3(p: i32) -> u16 {
    unsafe { (*raw::mem.offset(p as isize)).b16.s3 }
}

/// Read the `gr` (f64) field of `mem[p]`.
#[inline]
#[allow(dead_code)]
pub(crate) fn mem_gr(p: i32) -> f64 {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).gr }
}
/// Write the `gr` (f64) field of `mem[p]`.
#[inline]
pub(crate) fn set_mem_gr(p: i32, v: f64) {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).gr = v; }
}

/// Read the `ptr` (void*) field of `mem[p]`.
#[inline]
pub(crate) fn mem_ptr(p: i32) -> *mut std::ffi::c_void {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).ptr }
}
/// Write the `ptr` (void*) field of `mem[p]`.
#[inline]
pub(crate) fn set_mem_ptr(p: i32, v: *mut std::ffi::c_void) {
    // SAFETY: see module-level doc.
    unsafe { (*raw::mem.offset(p as isize)).ptr = v; }
}

/// Return a `*const u16` pointing to `mem[p]` — used for NATIVE_NODE_text.
#[inline]
pub(crate) fn mem_as_u16_ptr(p: i32) -> *const u16 {
    // SAFETY: see module-level doc. Caller ensures p + NATIVE_NODE_SIZE is a valid mem index.
    unsafe { raw::mem.offset(p as isize) as *const u16 }
}

/// Copy the full MemoryWord at `mem[p]`.
#[inline]
pub(crate) fn mem_word(p: i32) -> crate::mem_types::MemoryWord {
    // SAFETY: see module-level doc.
    unsafe { *raw::mem.offset(p as isize) }
}
/// Write a full MemoryWord to `mem[p]`.
#[inline]
pub(crate) fn set_mem_word(p: i32, w: crate::mem_types::MemoryWord) {
    // SAFETY: see module-level doc.
    unsafe { *raw::mem.offset(p as isize) = w; }
}

// ---------------------------------------------------------------------------
// eqtb accessor
// ---------------------------------------------------------------------------

/// Copy the full MemoryWord at `eqtb[p]`.
#[inline]
pub(crate) fn eqtb_word(p: i32) -> crate::mem_types::MemoryWord {
    unsafe { *raw::eqtb.offset(p as isize) }
}
/// Write a full MemoryWord to `eqtb[p]`.
#[inline]
pub(crate) fn set_eqtb_word(p: i32, w: crate::mem_types::MemoryWord) {
    unsafe { *raw::eqtb.offset(p as isize) = w; }
}
/// Read `eqtb[p].b32.s1` (the "value" half of an equivalents entry).
#[inline]
pub(crate) fn eqtb_b32s1(p: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures p is a valid eqtb index.
    unsafe { (*raw::eqtb.offset(p as isize)).b32.s1 }
}
/// Write `v` into `eqtb[p].b32.s1`.
#[inline]
pub(crate) fn set_eqtb_b32s1(p: i32, v: i32) {
    // SAFETY: see module-level doc. Caller ensures p is a valid eqtb index.
    unsafe { (*raw::eqtb.offset(p as isize)).b32.s1 = v; }
}
/// Read `eqtb[p].b16.s1` (the "type" half of an equivalents entry).
#[inline]
pub(crate) fn eqtb_b16s1(p: i32) -> u16 {
    unsafe { (*raw::eqtb.offset(p as isize)).b16.s1 }
}
/// Write `v` into `eqtb[p].b16.s1`.
#[inline]
pub(crate) fn set_eqtb_b16s1(p: i32, v: u16) {
    unsafe { (*raw::eqtb.offset(p as isize)).b16.s1 = v; }
}
/// Read `eqtb[p].b16.s0` (the "level" half of an equivalents entry).
#[inline]
pub(crate) fn eqtb_b16s0(p: i32) -> u16 {
    unsafe { (*raw::eqtb.offset(p as isize)).b16.s0 }
}
/// Write `v` into `eqtb[p].b16.s0`.
#[inline]
pub(crate) fn set_eqtb_b16s0(p: i32, v: u16) {
    unsafe { (*raw::eqtb.offset(p as isize)).b16.s0 = v; }
}

// ---------------------------------------------------------------------------
// str_ptr / str_start / str_pool (read accessor)
// ---------------------------------------------------------------------------

/// Returns `name_of_file` as a raw pointer (NUL-terminated C string).
#[inline]
pub(crate) fn name_of_file() -> *mut libc::c_char {
    unsafe { raw::name_of_file }
}
/// Write `name_length = v`.
#[inline]
pub(crate) fn set_name_length(v: i32) {
    unsafe { raw::name_length = v; }
}
/// Returns `name_of_file16` (UTF-16 name buffer).
#[inline]
pub(crate) fn name_of_file16() -> *mut u16 {
    unsafe { raw::name_of_file16 }
}
/// Write `name_of_file16 = v`.
#[inline]
pub(crate) fn set_name_of_file16(v: *mut u16) {
    unsafe { raw::name_of_file16 = v; }
}
/// Returns `name_length16`.
#[inline]
pub(crate) fn name_length16() -> i32 {
    unsafe { raw::name_length16 }
}
/// Write `name_length16 = v`.
#[inline]
pub(crate) fn set_name_length16(v: i32) {
    unsafe { raw::name_length16 = v; }
}
/// Returns `name_length`.
#[inline]
pub(crate) fn name_length() -> i32 {
    unsafe { raw::name_length }
}

/// Returns the current number of strings in the string pool.
#[inline]
pub(crate) fn str_ptr() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::str_ptr }
}

/// Returns `str_start[k]` — the pool offset of the k-th string.
#[inline]
pub(crate) fn str_start(k: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures k is a valid string index.
    unsafe { *raw::str_start.offset(k as isize) }
}

/// Read `str_pool[k]` (a packed UTF-16 code unit).
#[inline]
pub(crate) fn str_pool_entry(k: i32) -> u16 {
    // SAFETY: see module-level doc. Caller ensures k is a valid pool index.
    unsafe { *raw::str_pool.offset(k as isize) }
}

// ---------------------------------------------------------------------------
// current_diagnostic setter
// ---------------------------------------------------------------------------

/// Sets `current_diagnostic` to `v` (pass null to clear).
#[inline]
pub(crate) fn set_current_diagnostic(v: *mut std::ffi::c_void) {
    // SAFETY: see module-level doc.
    unsafe { raw::current_diagnostic = v; }
}

// ---------------------------------------------------------------------------
// in_open / full_source_filename_stack / line_stack / file_line_error_style_p
// ---------------------------------------------------------------------------

/// Returns the current input nesting depth.
#[inline]
pub(crate) fn in_open() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::in_open }
}

/// Returns `full_source_filename_stack[k]` (the str_number of the source file
/// at nesting level k).
#[inline]
pub(crate) fn full_source_filename_stack(k: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures k <= in_open.
    unsafe { *raw::full_source_filename_stack.offset(k as isize) }
}

/// Returns `line_stack[k]` (the saved line number at nesting level k).
#[inline]
pub(crate) fn line_stack(k: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures k <= in_open.
    unsafe { *raw::line_stack.offset(k as isize) }
}

/// Returns non-zero when file:line-style error messages are active.
#[inline]
pub(crate) fn file_line_error_style_p() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::file_line_error_style_p }
}

// ---------------------------------------------------------------------------
// hash / prim / eqtb_top
// ---------------------------------------------------------------------------

/// Read `hash[p].s0` (the "link" half of a hash-table entry).
#[inline]
pub(crate) fn hash_s0(p: i32) -> i32 {
    unsafe { (*raw::hash.offset(p as isize)).s0 }
}
/// Write `hash[p].s0 = v`.
#[inline]
pub(crate) fn set_hash_s0(p: i32, v: i32) {
    unsafe { (*raw::hash.offset(p as isize)).s0 = v; }
}
/// Read `hash[p].s1` (the "text" half of a hash-table entry).
#[inline]
pub(crate) fn hash_s1(p: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures p is a valid hash index.
    unsafe { (*raw::hash.offset(p as isize)).s1 }
}
/// Write `hash[p].s1 = v`.
#[inline]
pub(crate) fn set_hash_s1(p: i32, v: i32) {
    unsafe { (*raw::hash.offset(p as isize)).s1 = v; }
}

/// Read `prim[p].s0` (the link half of a primitive-table entry).
#[inline]
pub(crate) fn prim_s0(p: i32) -> i32 {
    unsafe { raw::prim[p as usize].s0 }
}
/// Write `prim[p].s0 = v`.
#[inline]
pub(crate) fn set_prim_s0(p: i32, v: i32) {
    unsafe { raw::prim[p as usize].s0 = v; }
}
/// Read `prim[p].s1` (the "text" half of a primitive-table entry).
#[inline]
pub(crate) fn prim_s1(p: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures p < prim.len().
    unsafe { raw::prim[p as usize].s1 }
}
/// Write `prim[p].s1 = v`.
#[inline]
pub(crate) fn set_prim_s1(p: i32, v: i32) {
    unsafe { raw::prim[p as usize].s1 = v; }
}

/// Returns `eqtb_top`.
#[inline]
pub(crate) fn eqtb_top() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::eqtb_top }
}

// ---------------------------------------------------------------------------
// hash management globals (hash_used / hash_extra / hash_high /
//                          no_new_control_sequence / prim_used)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn hash_used() -> i32 {
    unsafe { raw::hash_used }
}
#[inline]
pub(crate) fn set_hash_used(v: i32) {
    unsafe { raw::hash_used = v; }
}
#[inline]
pub(crate) fn hash_extra() -> i32 {
    unsafe { raw::hash_extra }
}
#[inline]
pub(crate) fn hash_high() -> i32 {
    unsafe { raw::hash_high }
}
#[inline]
pub(crate) fn set_hash_high(v: i32) {
    unsafe { raw::hash_high = v; }
}
#[inline]
pub(crate) fn no_new_control_sequence() -> bool {
    unsafe { raw::no_new_control_sequence }
}
#[inline]
pub(crate) fn prim_used() -> i32 {
    unsafe { raw::prim_used }
}
#[inline]
pub(crate) fn set_prim_used(v: i32) {
    unsafe { raw::prim_used = v; }
}

// ---------------------------------------------------------------------------
// temp_ptr
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn temp_ptr() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::temp_ptr }
}
#[inline]
pub(crate) fn set_temp_ptr(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::temp_ptr = v; }
}

// ---------------------------------------------------------------------------
// dig
// ---------------------------------------------------------------------------

/// Read `dig[k]`, the k-th digit scratch byte (xetex-xetexd.h:432).
#[inline]
pub(crate) fn dig_entry(k: usize) -> u8 {
    // SAFETY: see module-level doc.
    unsafe { raw::dig[k] }
}

/// Write `v` into `dig[k]`.
#[inline]
pub(crate) fn set_dig_entry(k: usize, v: u8) {
    // SAFETY: see module-level doc.
    unsafe { raw::dig[k] = v; }
}

// ---------------------------------------------------------------------------
// selector / term_offset / file_offset
// ---------------------------------------------------------------------------

/// Returns the current output selector (xetex-xetexd.h:431).
#[inline]
pub(crate) fn selector() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::selector }
}
/// Write `selector = v`.
#[inline]
pub(crate) fn set_selector(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::selector = v; }
}

/// Returns true while the engine is processing a `\special` (xetex-xetexd.h:439).
#[inline]
pub(crate) fn doing_special() -> bool {
    // SAFETY: see module-level doc.
    unsafe { raw::doing_special }
}

/// Returns `term_offset` (columns written to terminal since last newline).
#[inline]
pub(crate) fn term_offset() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::term_offset }
}
/// Sets `term_offset`.
#[inline]
pub(crate) fn set_term_offset(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::term_offset = v; }
}

/// Returns `file_offset` (columns written to log file since last newline).
#[inline]
pub(crate) fn file_offset() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::file_offset }
}
/// Sets `file_offset`.
#[inline]
pub(crate) fn set_file_offset(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::file_offset = v; }
}

// ---------------------------------------------------------------------------
// rust_stdout / log_file / write_file / current_diagnostic
// ---------------------------------------------------------------------------

/// Returns the `rust_stdout` output handle.
#[inline]
pub(crate) fn rust_stdout() -> usize {
    // SAFETY: see module-level doc.
    unsafe { raw::rust_stdout }
}

/// Returns the `log_file` output handle.
#[inline]
pub(crate) fn log_file() -> usize {
    // SAFETY: see module-level doc.
    unsafe { raw::log_file }
}

/// Returns `write_file[i]`, the output handle for write-stream `i` (0–15).
#[inline]
pub(crate) fn write_file(i: usize) -> usize {
    // SAFETY: see module-level doc.
    unsafe { raw::write_file[i] }
}

/// Returns the current diagnostic capture pointer (null = no active capture).
#[inline]
pub(crate) fn current_diagnostic() -> *mut std::ffi::c_void {
    // SAFETY: see module-level doc.
    unsafe { raw::current_diagnostic }
}

// ---------------------------------------------------------------------------
// print_raw_char globals
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn max_print_line() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::max_print_line }
}

#[inline]
pub(crate) fn tally() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::tally }
}
#[inline]
pub(crate) fn set_tally(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::tally = v; }
}

#[inline]
pub(crate) fn error_line() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::error_line }
}

/// Read `trick_buf[k]`.
#[inline]
pub(crate) fn trick_count() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::trick_count }
}
/// Write `trick_buf[k] = v`.
#[inline]
pub(crate) fn set_trick_buf(k: usize, v: u16) {
    // SAFETY: see module-level doc.
    unsafe { raw::trick_buf[k] = v; }
}

#[inline]
pub(crate) fn pool_size() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::pool_size }
}
#[inline]
pub(crate) fn pool_ptr() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::pool_ptr }
}
#[inline]
pub(crate) fn set_pool_ptr(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::pool_ptr = v; }
}
/// Write `str_pool[k] = v`.
#[inline]
pub(crate) fn set_str_pool(k: i32, v: u16) {
    // SAFETY: see module-level doc.
    unsafe { *raw::str_pool.offset(k as isize) = v; }
}

// ---------------------------------------------------------------------------
// str_start write / str_ptr write / string pool limits
// ---------------------------------------------------------------------------

/// Write `str_start[k] = v`.
#[inline]
pub(crate) fn set_str_start(k: i32, v: i32) {
    // SAFETY: see module-level doc. Caller ensures k is a valid string index.
    unsafe { *raw::str_start.offset(k as isize) = v; }
}
/// Write `str_ptr = v`.
#[inline]
pub(crate) fn set_str_ptr(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::str_ptr = v; }
}
#[inline]
pub(crate) fn max_strings() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::max_strings }
}
#[inline]
pub(crate) fn init_str_ptr() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::init_str_ptr }
}
#[inline]
pub(crate) fn init_pool_ptr() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::init_pool_ptr }
}
/// Read `buffer[k]` (UnicodeScalar = i32).
#[inline]
pub(crate) fn buffer_entry(k: i32) -> i32 {
    // SAFETY: see module-level doc. Caller ensures k is a valid buffer index.
    unsafe { *raw::buffer.offset(k as isize) }
}

// ---------------------------------------------------------------------------
// help_ptr / help_line
// ---------------------------------------------------------------------------

#[inline]
#[allow(dead_code)] // getter defined for completeness; only setter used so far
pub(crate) fn help_ptr() -> u8 {
    // SAFETY: see module-level doc.
    unsafe { raw::help_ptr }
}
#[inline]
pub(crate) fn set_help_ptr(v: u8) {
    // SAFETY: see module-level doc.
    unsafe { raw::help_ptr = v; }
}
/// Read `help_line[k]` as a C string pointer.
#[inline]
pub(crate) fn help_line(k: usize) -> *const libc::c_char {
    // SAFETY: see module-level doc. k must be < 6.
    unsafe { raw::help_line[k] }
}
/// Write `help_line[k] = v` (a C string pointer; caller owns the lifetime).
#[inline]
pub(crate) fn set_help_line(k: usize, v: *const libc::c_char) {
    // SAFETY: see module-level doc. k must be < 6.
    unsafe { raw::help_line[k] = v; }
}

// ---------------------------------------------------------------------------
// two_to_the / spec_log (read-only tables from xetex-ini.c)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn two_to_the(k: usize) -> i32 {
    // SAFETY: see module-level doc. k must be < 31.
    unsafe { raw::two_to_the[k] }
}
#[inline]
pub(crate) fn spec_log(k: usize) -> i32 {
    // SAFETY: see module-level doc. k must be < 29.
    unsafe { raw::spec_log[k] }
}

// ---------------------------------------------------------------------------
// interaction / history / error_count / use_err_help / halt_on_error_p
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn interaction() -> u8 {
    // SAFETY: see module-level doc.
    unsafe { raw::interaction }
}
#[inline]
pub(crate) fn set_interaction(v: u8) {
    // SAFETY: see module-level doc.
    unsafe { raw::interaction = v; }
}
#[inline]
pub(crate) fn history() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::history }
}
#[inline]
pub(crate) fn set_history(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::history = v; }
}
#[inline]
pub(crate) fn error_count() -> i8 {
    // SAFETY: see module-level doc.
    unsafe { raw::error_count }
}
#[inline]
pub(crate) fn set_error_count(v: i8) {
    // SAFETY: see module-level doc.
    unsafe { raw::error_count = v; }
}
#[inline]
pub(crate) fn use_err_help() -> bool {
    // SAFETY: see module-level doc.
    unsafe { raw::use_err_help }
}
#[inline]
pub(crate) fn halt_on_error_p() -> bool {
    // SAFETY: see module-level doc.
    unsafe { raw::halt_on_error_p != 0 }
}
#[inline]
pub(crate) fn log_opened() -> bool {
    // SAFETY: see module-level doc.
    unsafe { raw::log_opened }
}
#[inline]
pub(crate) fn job_name() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::job_name }
}
#[inline]
pub(crate) fn set_halt_on_error_p(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::halt_on_error_p = v; }
}

// ---------------------------------------------------------------------------
// engine launch config flags (set by tt_xetex_set_int_variable)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_in_initex_mode(v: bool) {
    // SAFETY: see module-level doc.
    unsafe { raw::in_initex_mode = v; }
}
#[inline]
pub(crate) fn set_synctex_enabled(v: i32) {
    // SAFETY: see module-level doc.
    unsafe { raw::synctex_enabled = v; }
}
#[inline]
pub(crate) fn set_semantic_pagination_enabled(v: bool) {
    // SAFETY: see module-level doc.
    unsafe { raw::semantic_pagination_enabled = v; }
}
#[inline]
pub(crate) fn set_shell_escape_enabled(v: bool) {
    // SAFETY: see module-level doc.
    unsafe { raw::shell_escape_enabled = v; }
}

// ---------------------------------------------------------------------------
// old_setting
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn old_setting() -> u8 {
    unsafe { raw::old_setting }
}
#[inline]
pub(crate) fn set_old_setting(v: u8) {
    unsafe { raw::old_setting = v; }
}

// ---------------------------------------------------------------------------
// cur_chr / cur_val / cur_name / cur_area / cur_ext
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn cur_chr() -> i32 {
    unsafe { raw::cur_chr }
}
#[inline]
pub(crate) fn cur_val() -> i32 {
    unsafe { raw::cur_val }
}
#[inline]
pub(crate) fn cur_name() -> i32 {
    unsafe { raw::cur_name }
}
#[inline]
pub(crate) fn cur_area() -> i32 {
    unsafe { raw::cur_area }
}
#[inline]
pub(crate) fn cur_ext() -> i32 {
    unsafe { raw::cur_ext }
}

// ---------------------------------------------------------------------------
// name_in_progress / stop_at_space
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_name_in_progress(v: bool) {
    unsafe { raw::name_in_progress = v; }
}
#[inline]
pub(crate) fn set_stop_at_space(v: bool) {
    unsafe { raw::stop_at_space = v; }
}

// ---------------------------------------------------------------------------
// save_stack / save_ptr / cur_level / cur_group
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn save_ptr() -> i32 {
    unsafe { raw::save_ptr }
}
#[inline]
pub(crate) fn set_save_ptr(v: i32) {
    unsafe { raw::save_ptr = v; }
}
#[inline]
pub(crate) fn max_save_stack() -> i32 {
    unsafe { raw::max_save_stack }
}
#[inline]
pub(crate) fn set_max_save_stack(v: i32) {
    unsafe { raw::max_save_stack = v; }
}
#[inline]
pub(crate) fn save_size() -> i32 {
    unsafe { raw::save_size }
}
/// Read `save_stack[k].b32.s1`.
#[inline]
pub(crate) fn save_stack_b32s1(k: i32) -> i32 {
    unsafe { (*raw::save_stack.offset(k as isize)).b32.s1 }
}
/// Read `save_stack[k].b32.s0`.
#[inline]
pub(crate) fn save_stack_b32s0(k: i32) -> i32 {
    unsafe { (*raw::save_stack.offset(k as isize)).b32.s0 }
}
/// Write `save_stack[k].b32.s1 = v`.
#[inline]
pub(crate) fn set_save_stack_b32s1(k: i32, v: i32) {
    unsafe { (*raw::save_stack.offset(k as isize)).b32.s1 = v; }
}
/// Write `save_stack[k].b32.s0 = v`.
#[inline]
pub(crate) fn set_save_stack_b32s0(k: i32, v: i32) {
    unsafe { (*raw::save_stack.offset(k as isize)).b32.s0 = v; }
}
/// Read `save_stack[k].b16.s1`.
#[inline]
pub(crate) fn save_stack_b16s1(k: i32) -> u16 {
    unsafe { (*raw::save_stack.offset(k as isize)).b16.s1 }
}
/// Read `save_stack[k].b16.s0`.
#[inline]
pub(crate) fn save_stack_b16s0(k: i32) -> u16 {
    unsafe { (*raw::save_stack.offset(k as isize)).b16.s0 }
}
/// Write `save_stack[k].b16.s1 = v`.
#[inline]
pub(crate) fn set_save_stack_b16s1(k: i32, v: u16) {
    unsafe { (*raw::save_stack.offset(k as isize)).b16.s1 = v; }
}
/// Write `save_stack[k].b16.s0 = v`.
#[inline]
pub(crate) fn set_save_stack_b16s0(k: i32, v: u16) {
    unsafe { (*raw::save_stack.offset(k as isize)).b16.s0 = v; }
}
/// Copy a full MemoryWord to `save_stack[k]`.
#[inline]
pub(crate) fn set_save_stack_word(k: i32, w: crate::mem_types::MemoryWord) {
    unsafe { *raw::save_stack.offset(k as isize) = w; }
}
#[inline]
pub(crate) fn cur_level() -> u16 {
    unsafe { raw::cur_level }
}
#[inline]
pub(crate) fn set_cur_level(v: u16) {
    unsafe { raw::cur_level = v; }
}
#[inline]
pub(crate) fn cur_group() -> u8 {
    unsafe { raw::cur_group }
}
#[inline]
pub(crate) fn set_cur_group(v: u8) {
    unsafe { raw::cur_group = v; }
}
#[inline]
pub(crate) fn cur_boundary() -> i32 {
    unsafe { raw::cur_boundary }
}
#[inline]
pub(crate) fn set_cur_boundary(v: i32) {
    unsafe { raw::cur_boundary = v; }
}
/// Read `XEQ_LEVEL(p)` = `_xeq_level_array[p - INT_BASE]`.
#[inline]
pub(crate) fn xeq_level(p: i32) -> u16 {
    // SAFETY: caller ensures p >= INT_BASE and p - INT_BASE < 1_114_730.
    unsafe { raw::_xeq_level_array[(p - 7_826_729) as usize] }
}
/// Write `XEQ_LEVEL(p) = v`.
#[inline]
pub(crate) fn set_xeq_level(p: i32, v: u16) {
    unsafe { raw::_xeq_level_array[(p - 7_826_729) as usize] = v; }
}

// ---------------------------------------------------------------------------
// read_open / read_file (for \read stream management)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn read_open_entry(n: usize) -> u8 {
    unsafe { raw::read_open[n] }
}
#[inline]
pub(crate) fn set_read_open_entry(n: usize, v: u8) {
    unsafe { raw::read_open[n] = v; }
}
/// Returns `read_file[n]` (the UFILE* value at index n).
#[inline]
pub(crate) fn read_file_entry(n: usize) -> *mut crate::io::UFile {
    unsafe { raw::read_file[n] as *mut crate::io::UFile }
}
/// Returns a pointer to `read_file[n]`, for passing to `u_open_in`.
#[inline]
pub(crate) fn read_file_ptr(n: usize) -> *mut *mut crate::io::UFile {
    unsafe { raw::read_file.as_mut_ptr().add(n) as *mut *mut crate::io::UFile }
}

// ---------------------------------------------------------------------------
// name_of_file16_entry — indexed read of the UTF-16 name buffer
// ---------------------------------------------------------------------------

/// Returns `name_of_file16[k]`.
#[inline]
pub(crate) fn name_of_file16_entry(k: i32) -> u16 {
    unsafe { *raw::name_of_file16.add(k as usize) }
}

// ---------------------------------------------------------------------------
// font_name
// ---------------------------------------------------------------------------

/// Returns `font_name[f]` — the string-pool index for font f's name.
#[inline]
pub(crate) fn font_name_entry(f: i32) -> i32 {
    unsafe { *raw::font_name.offset(f as isize) }
}

// ---------------------------------------------------------------------------
// epochseconds / microseconds
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn epochseconds() -> i32 {
    unsafe { raw::epochseconds }
}
#[inline]
pub(crate) fn microseconds() -> i32 {
    unsafe { raw::microseconds }
}

// ---------------------------------------------------------------------------
// mag_set
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn mag_set() -> i32 {
    unsafe { raw::mag_set }
}
#[inline]
pub(crate) fn set_mag_set(v: i32) {
    unsafe { raw::mag_set = v; }
}

// ---------------------------------------------------------------------------
// semantic nest (nest / nest_ptr / max_nest_stack / nest_size / cur_list)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn nest_ptr() -> i32 {
    unsafe { raw::nest_ptr }
}
#[inline]
pub(crate) fn set_nest_ptr(v: i32) {
    unsafe { raw::nest_ptr = v; }
}
#[inline]
pub(crate) fn max_nest_stack() -> i32 {
    unsafe { raw::max_nest_stack }
}
#[inline]
pub(crate) fn set_max_nest_stack(v: i32) {
    unsafe { raw::max_nest_stack = v; }
}
#[inline]
pub(crate) fn nest_size() -> i32 {
    unsafe { raw::nest_size }
}
/// Copy `nest[k]` to `cur_list` (full struct assignment).
#[inline]
pub(crate) fn restore_cur_list_from_nest(k: i32) {
    unsafe { raw::cur_list = *raw::nest.offset(k as isize); }
}
/// Copy `cur_list` to `nest[k]` (full struct assignment).
#[inline]
pub(crate) fn save_cur_list_to_nest(k: i32) {
    unsafe { *raw::nest.offset(k as isize) = raw::cur_list; }
}
// cur_list field accessors
#[inline]
pub(crate) fn cur_list_head() -> i32 {
    unsafe { raw::cur_list.head }
}
#[inline]
pub(crate) fn set_cur_list_head(v: i32) {
    unsafe { raw::cur_list.head = v; }
}
#[inline]
pub(crate) fn set_cur_list_tail(v: i32) {
    unsafe { raw::cur_list.tail = v; }
}
#[inline]
pub(crate) fn set_cur_list_prev_graf(v: i32) {
    unsafe { raw::cur_list.prev_graf = v; }
}
#[inline]
pub(crate) fn set_cur_list_mode_line(v: i32) {
    unsafe { raw::cur_list.mode_line = v; }
}
#[inline]
pub(crate) fn set_cur_list_e_tex_aux(v: i32) {
    unsafe { raw::cur_list.e_tex_aux = v; }
}

// ---------------------------------------------------------------------------
// font_max
// ---------------------------------------------------------------------------

/// Returns `font_max` — the largest valid internal font number
/// (xetex-xetexd.h:406).
#[inline]
pub(crate) fn font_max() -> i32 {
    unsafe { raw::font_max }
}

// ---------------------------------------------------------------------------
// cur_cmd / cur_mark
// ---------------------------------------------------------------------------

/// Returns the current command code (`cur_cmd`, xetex-xetexd.h:499).
#[inline]
pub(crate) fn cur_cmd() -> u8 {
    unsafe { raw::cur_cmd }
}

/// Returns `cur_mark[k]` — the k-th mark register token list
/// (xetex-xetexd.h:527; k must be 0..=4).
#[inline]
pub(crate) fn cur_mark(k: usize) -> i32 {
    unsafe { raw::cur_mark[k] }
}

// ---------------------------------------------------------------------------
// input_stack / input_ptr / base_ptr
// ---------------------------------------------------------------------------

/// Copy `input_stack[k]` (full struct copy; xetex-xetexd.h:503).
#[inline]
pub(crate) fn input_stack(k: i32) -> InputState {
    // SAFETY: see module-level doc. Caller ensures k <= input_ptr.
    unsafe { *raw::input_stack.offset(k as isize) }
}
/// Write `input_stack[k] = v` (full struct assign).
#[inline]
pub(crate) fn set_input_stack(k: i32, v: InputState) {
    // SAFETY: see module-level doc.
    unsafe { *raw::input_stack.offset(k as isize) = v; }
}

/// Returns `input_ptr` — current top of the input-stack (xetex-xetexd.h:504).
#[inline]
pub(crate) fn input_ptr() -> i32 {
    unsafe { raw::input_ptr }
}
/// Sets `input_ptr`.
#[inline]
#[allow(dead_code)]
pub(crate) fn set_input_ptr(v: i32) {
    unsafe { raw::input_ptr = v; }
}

/// Returns `base_ptr` — search base in the input stack
/// (xetex-xetexd.h:521).
#[inline]
pub(crate) fn base_ptr() -> i32 {
    unsafe { raw::base_ptr }
}
/// Sets `base_ptr`.
#[inline]
pub(crate) fn set_base_ptr(v: i32) {
    unsafe { raw::base_ptr = v; }
}

// ---------------------------------------------------------------------------
// grp_stack / if_stack
// ---------------------------------------------------------------------------

/// Returns `grp_stack[k]` — the save-stack boundary saved at input level k
/// (xetex-xetexd.h:705; save_pointer = i32).
#[inline]
pub(crate) fn grp_stack(k: i32) -> i32 {
    unsafe { *raw::grp_stack.offset(k as isize) }
}
/// Sets `grp_stack[k]`.
#[inline]
pub(crate) fn set_grp_stack(k: i32, v: i32) {
    unsafe { *raw::grp_stack.offset(k as isize) = v; }
}

/// Returns `if_stack[k]` — the `cond_ptr` saved at input level k
/// (xetex-xetexd.h:706).
#[inline]
pub(crate) fn if_stack(k: i32) -> i32 {
    unsafe { *raw::if_stack.offset(k as isize) }
}
/// Sets `if_stack[k]`.
#[inline]
pub(crate) fn set_if_stack(k: i32, v: i32) {
    unsafe { *raw::if_stack.offset(k as isize) = v; }
}

// ---------------------------------------------------------------------------
// cond_ptr / if_limit / cur_if / if_line
// ---------------------------------------------------------------------------

/// Returns `cond_ptr` — head of the conditional-stack list
/// (xetex-xetexd.h:537).
#[inline]
pub(crate) fn cond_ptr() -> i32 {
    unsafe { raw::cond_ptr }
}
/// Sets `cond_ptr`.
#[inline]
pub(crate) fn set_cond_ptr(v: i32) {
    unsafe { raw::cond_ptr = v; }
}

/// Returns `if_limit` — current \if nesting limit code
/// (xetex-xetexd.h:538; unsigned char).
#[inline]
pub(crate) fn if_limit() -> u8 {
    unsafe { raw::if_limit }
}
/// Sets `if_limit`.
#[inline]
pub(crate) fn set_if_limit(v: u8) {
    unsafe { raw::if_limit = v; }
}

/// Returns `cur_if` — current \if type (xetex-xetexd.h:539; small_number = i16).
#[inline]
pub(crate) fn cur_if() -> i16 {
    unsafe { raw::cur_if }
}
/// Sets `cur_if`.
#[inline]
pub(crate) fn set_cur_if(v: i16) {
    unsafe { raw::cur_if = v; }
}

/// Returns `if_line` — line number where the current \if was entered
/// (xetex-xetexd.h:540).
#[inline]
pub(crate) fn if_line() -> i32 {
    unsafe { raw::if_line }
}
/// Sets `if_line`.
#[inline]
pub(crate) fn set_if_line(v: i32) {
    unsafe { raw::if_line = v; }
}
