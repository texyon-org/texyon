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
    extern "C" {
        // xetex-xetexd.h:506 — current input state; layout mirrors InputState above.
        pub(super) static mut cur_input: super::InputState;
        // open_parens moved to EngineState.input.open_parens.
        // xetex-xetexd.h:510 — current source line number.
        pub(super) static mut line: i32;

        // eqtb (*mut memory_word) moved to EngineState.eqtb.data (session 51).

        // xetex-xetexd.h:459 — temporary pointer register.
        pub(super) static mut temp_ptr: i32;

        // The `mem` arena (`memory_word *mem`) and its bookkeeping scalars
        // (`lo_mem_max`, `hi_mem_min`, `avail`, `mem_end`, `rover`, `var_used`,
        // `dyn_used`) are now Rust-owned on `EngineState.mem` (`MemState`);
        // accessors index `s.mem.data` / read the scalars. (session 58)

        // Output file handles (xetex-xetexd.h:429–430, 691).
        // `rust_output_handle_t` is `uintptr_t` (tectonic_bridge_core_generated.h:9).

        // Current diagnostic capture handle (xetex-output.c, un-static'd session 8).
        // Null (0) means no active capture. Type is opaque `ttbc_diagnostic_t *`.

        // pseudo_files moved to EngineState.input.pseudo_files.
        // in_open moved to EngineState.input.in_open.
        // Hash table: `b32x2 *hash` moved to EngineState.eqtb.hash (session 52).
        // Hash table management globals: `hash_extra` moved to EngineState.eqtb.hash_extra (session 52).
        // prim[PRIM_SIZE + 1] moved to EngineState.eqtb.prim (session 49).
        // Top of the eqtb array: `eqtb_top` moved to EngineState.eqtb.eqtb_top (session 52).

        // Name-of-file globals (xetex-xetexd.h:388–391).
        // name_of_file moved to EngineState.input.name_of_file.
        // name_length moved to EngineState.input.name_length (i32).
        // name_of_file16 moved to EngineState.input.name_of_file16.
        // name_length16 moved to EngineState.input.name_length16 (i32).

        // Font memory tables (xetex-xetexd.h:555–590).
        // font_info moved to EngineState.fonts.font_info (Vec<MemoryWord>).
        // xdv_buffer moved to EngineState.fonts.xdv_buffer.
        // Current control-sequence index (xetex-xetexd.h:501; 0 = not a cs).
        pub(super) static mut cur_cs: i32;
        // Current token (xetex-xetexd.h:502; token encoded as int32).
        pub(super) static mut cur_tok: i32;
        // align_state moved to EngineState.scanner (scalar).
        // Current character code (xetex-xetexd.h:500).
        pub(super) static mut cur_chr: i32;
        // Current command code (`eight_bits` = u8, xetex-xetexd.h:499).
        pub(super) static mut cur_cmd: u8;
        // cur_mark[5] moved to EngineState.scanner (inline fixed array).
        // Current parsed integer value (xetex-xetexd.h:530).
        pub(super) static mut cur_val: i32;

        // Input state stack ownership moved to EngineState.input.input_stack
        // (Rust-owned Vec<InputState>); see src/state.rs.
        // input_ptr/max_in_stack/base_ptr/par_loc/par_token moved to
        // EngineState.input (scalars). Capacity stack_size stays a C global.
        pub(super) static mut stack_size: i32;

        // Macro parameter stack (xetex-xetexd.h:517–519). The `param_stack`
        // array is Rust-owned on `EngineState.input.param_stack`; the scalars
        // param_ptr/max_param_stack are now on EngineState.input too.
        // Allocated size of param_stack stays a C global.
        pub(super) static mut param_size: i32;

        // SA trie cursor — scratch pointer used during SA node traversal/allocation
        // (xetex-xetexd.h:710; xetex-ini.c:336).
        pub(super) static mut cur_ptr: i32;

        // `var_used` / `dyn_used` memory-usage counters moved to
        // EngineState.mem (MemState) with the `mem` arena. (session 58)

        // cond_ptr / if_limit / cur_if / if_line / skip_line moved to
        // EngineState.scanner (scalars).

        // Semantic nest (mode stack, xetex-xetexd.h:476–479,413).
        // nest_ptr/max_nest_stack moved to EngineState.input (scalars).
        // Capacity nest_size stays a C global.
        pub(super) static mut nest_size: i32;
        // Current list state (xetex-xetexd.h:479; direct struct, not a pointer).
        pub(super) static mut cur_list: crate::mem_types::ListStateRecord;
        // read_open moved to EngineState.input.read_open ([u8; 17]).
        // read_file moved to EngineState.input.read_file ([*mut c_void; 16]).

        // Save-stack (xetex-xetexd.h:492–494). The `save_stack` array itself
        // is Rust-owned (`EngineState.save_stack.data`); only the capacity
        // `save_size` remains a C global here.
        pub(super) static mut save_size: i32;
        // _xeq_level_array (XEQ_LEVEL; EQTB_SIZE - INT_BASE + 1 = 1114730 entries)
        // moved to EngineState.eqtb.xeq_level (session 49).

        // Expression-depth guard (xetex-xetexd.h:415, 525).
        pub(super) static mut expand_depth: i32;
        // cur_val_level moved to EngineState.scanner.

        // scanner_status / warning_index / def_ref / long_state moved to
        // EngineState.scanner (scalars).

        // Input buffer: first = next free slot; buf_size = capacity
        // last/max_buf_stack moved to EngineState.input (scalars).
        // (xetex-xetexd.h:393–395, 409).
        pub(super) static mut first: i32;
        pub(super) static mut buf_size: i32;
        // Maximum allowed input-nesting depth (xetex-xetexd.h:411).
        pub(super) static mut max_in_open: i32;

        // is_in_csname moved to EngineState.scanner.
        // force_eof moved to EngineState.input.
        // used_tectonic_coda_tokens moved to EngineState.scanner.
        // cur_val1 moved to EngineState.scanner.

        // disc_ptr[4] moved to EngineState.scanner (inline fixed array).

        // mapped_text moved to EngineState.fonts.mapped_text.
        // main_control scratch glue-spec and font-info index (xetex-xetexd.h:675–676).
        pub(super) static mut main_p: i32;
        pub(super) static mut main_k: i32;
        // main_control inner-loop state (xetex-xetexd.h:672–686).
        pub(super) static mut main_f: i32; // internal_font_number
        pub(super) static mut main_h: i32;
        pub(super) static mut main_pp: i32;
        pub(super) static mut main_ppp: i32;
        pub(super) static mut main_s: i32;
        // Ligature/kern inner-loop state (xetex-xetexd.h:630–634, 683–686).
        pub(super) static mut cur_l: i32;
        pub(super) static mut cur_r: i32;
        pub(super) static mut cur_q: i32;
        pub(super) static mut lig_stack: i32;
        pub(super) static mut ligature_present: bool;
        pub(super) static mut lft_hit: bool;
        pub(super) static mut rt_hit: bool;
        // native_text cluster moved to EngineState.scanner.
        // Further main_loop state (xetex-xetexd.h:679–686).
        pub(super) static mut bchar: i32;
        pub(super) static mut false_bchar: i32;
        pub(super) static mut cancel_boundary: bool;
        pub(super) static mut ins_disc: bool;
        // XeTeX inter-character class state (xetex-xetexd.h:679-681).
        pub(super) static mut space_class: i32;
        pub(super) static mut prev_class: i32;
        // Hyphenation trie and exception tables (xetex-xetexd.h:635-645).

        // last_leftmost_char .. best_pl_glue (the line-breaker / LR-reverse /
        // hyphenation-attempt scratch) moved to EngineState.linebreak
        // (LinebreakState); see src/state.rs.
        // Shipout / DVI output state (total_pages, max_v, max_h, max_push,
        // last_bop, doing_leaders, rule_ht/dp/wd, cur_h, cur_v,
        // cur_page_width/height, cur_h_offset/v_offset, write_loc) moved to
        // EngineState.shipout (ShipoutState); see src/state.rs (session 47).

        // xetex-math.c module-level state (cur_mlist, cur_style, cur_size,
        // cur_mu, mlist_penalties) moved to EngineState.scanner (ScannerState);
        // see src/state.rs (session 48).
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
    crate::state::with_state(|state| state.errors.arith_error)
}

/// Sets the engine's `arith_error` flag.
#[inline]
pub(crate) fn set_arith_error(v: bool) {
    crate::state::with_state_mut(|state| {
        state.errors.arith_error = v;
    });
}

// ---------------------------------------------------------------------------
// tex_remainder
// ---------------------------------------------------------------------------

/// Returns the `tex_remainder` output register of `x_over_n` /
/// `xn_over_d`.
#[inline]
#[allow(dead_code)] // used in unit tests and by future Rust callers.
pub(crate) fn tex_remainder() -> i32 {
    crate::state::with_state(|state| state.errors.tex_remainder)
}

/// Stores a value into the `tex_remainder` register.
#[inline]
pub(crate) fn set_tex_remainder(v: i32) {
    crate::state::with_state_mut(|state| {
        state.errors.tex_remainder = v;
    });
}

// ---------------------------------------------------------------------------
// randoms / j_random
// ---------------------------------------------------------------------------

/// Returns `randoms[k]`, the k-th element of the pseudo-random table.
///
/// Panics in debug mode if `k >= 55`.
#[inline]
pub(crate) fn random_entry(k: usize) -> i32 {
    crate::state::with_state(|state| state.errors.randoms[k])
}

/// Sets `randoms[k]` to `v`.
#[inline]
pub(crate) fn set_random_entry(k: usize, v: i32) {
    crate::state::with_state_mut(|state| {
        state.errors.randoms[k] = v;
    });
}

/// Returns the current `j_random` index.
#[inline]
pub(crate) fn j_random() -> u8 {
    crate::state::with_state(|state| state.errors.j_random)
}

/// Sets `j_random`.
#[inline]
pub(crate) fn set_j_random(v: u8) {
    crate::state::with_state_mut(|state| {
        state.errors.j_random = v;
    });
}

// ---------------------------------------------------------------------------
// mem arena word accessors
// ---------------------------------------------------------------------------

/// Read `mem[p].b32.s0` (the first 32-bit word / node-size / type field).
#[inline]
pub(crate) fn mem_b32s0(p: i32) -> i32 {
    // SAFETY: union projection; the `Vec` index is bounds-checked.
    crate::state::with_state(|s| unsafe { s.mem.data[p as usize].b32.s0 })
}

/// Write `v` into `mem[p].b32.s0`.
#[inline]
pub(crate) fn set_mem_b32s0(p: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.mem.data[p as usize].b32.s0 = v;
    });
}

/// Read `mem[p].b32.s1` (the `LLIST_link` / successor-pointer field).
///
/// # Safety
/// `p` must be a valid arena index (`0 ..= MEM_TOP`). TEX_NULL is never
/// a valid index; callers must guard against it before calling this.
#[inline]
pub(crate) fn mem_b32s1(p: i32) -> i32 {
    // SAFETY: union projection; the `Vec` index is bounds-checked.
    crate::state::with_state(|s| unsafe { s.mem.data[p as usize].b32.s1 })
}

/// Write `v` into `mem[p].b32.s1`.
#[inline]
pub(crate) fn set_mem_b32s1(p: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.mem.data[p as usize].b32.s1 = v;
    });
}

// ---------------------------------------------------------------------------
// lo_mem_max / hi_mem_min / avail / mem_end / rover
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn lo_mem_max() -> i32 {
    crate::state::with_state(|s| s.mem.lo_mem_max)
}

#[inline]
#[allow(dead_code)]
pub(crate) fn set_lo_mem_max(v: i32) {
    crate::state::with_state_mut(|s| {
        s.mem.lo_mem_max = v;
    });
}

#[inline]
pub(crate) fn hi_mem_min() -> i32 {
    crate::state::with_state(|s| s.mem.hi_mem_min)
}

#[inline]
pub(crate) fn set_hi_mem_min(v: i32) {
    crate::state::with_state_mut(|s| {
        s.mem.hi_mem_min = v;
    });
}

#[inline]
pub(crate) fn avail() -> i32 {
    crate::state::with_state(|s| s.mem.avail)
}

#[inline]
pub(crate) fn set_avail(v: i32) {
    crate::state::with_state_mut(|s| {
        s.mem.avail = v;
    });
}

#[inline]
pub(crate) fn mem_end() -> i32 {
    crate::state::with_state(|s| s.mem.mem_end)
}

#[inline]
pub(crate) fn set_mem_end(v: i32) {
    crate::state::with_state_mut(|s| {
        s.mem.mem_end = v;
    });
}

#[inline]
pub(crate) fn rover() -> i32 {
    crate::state::with_state(|s| s.mem.rover)
}

#[inline]
pub(crate) fn set_rover(v: i32) {
    crate::state::with_state_mut(|s| {
        s.mem.rover = v;
    });
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
/// Writes `cur_input.loc = v`.
#[inline]
pub(crate) fn set_cur_input_loc(v: i32) {
    unsafe {
        raw::cur_input.loc = v;
    }
}
/// Writes `cur_input.start = v`.
#[inline]
pub(crate) fn set_cur_input_start(v: i32) {
    unsafe {
        raw::cur_input.start = v;
    }
}
/// Writes `cur_input.state = v`.
#[inline]
pub(crate) fn set_cur_input_state(v: u16) {
    unsafe {
        raw::cur_input.state = v;
    }
}
/// Writes `cur_input.index = v`.
#[inline]
pub(crate) fn set_cur_input_index(v: u16) {
    unsafe {
        raw::cur_input.index = v;
    }
}
/// Writes `cur_input.name = v`.
#[inline]
pub(crate) fn set_cur_input_name(v: i32) {
    unsafe {
        raw::cur_input.name = v;
    }
}
/// Writes `cur_input.synctex_tag = v`.
#[inline]
pub(crate) fn set_cur_input_synctex_tag(v: i32) {
    unsafe {
        raw::cur_input.synctex_tag = v;
    }
}
/// Replaces the entire `cur_input` struct (e.g. when restoring from input_stack).
#[inline]
pub(crate) fn set_cur_input(v: InputState) {
    unsafe {
        raw::cur_input = v;
    }
}

/// Returns the current source line number.
#[inline]
pub(crate) fn line() -> i32 {
    // SAFETY: see module-level doc.
    unsafe { raw::line }
}

/// Returns `open_parens` (count of open pseudo-file parentheses in the transcript).
#[inline]
pub(crate) fn open_parens() -> i32 {
    crate::state::with_state(|s| s.input.open_parens)
}
/// Sets `open_parens = v`.
#[inline]
pub(crate) fn set_open_parens(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.open_parens = v;
    });
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
    // SAFETY: union projection; the `Vec` index is bounds-checked.
    crate::state::with_state(|s| unsafe { s.mem.data[p as usize].b16.s0 })
}
/// Write `v` into `mem[p].b16.s0`.
#[inline]
pub(crate) fn set_mem_b16s0(p: i32, v: u16) {
    crate::state::with_state_mut(|s| {
        s.mem.data[p as usize].b16.s0 = v;
    });
}

/// Read `mem[p].b16.s1` (high 16 bits of the first 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s1(p: i32) -> u16 {
    // SAFETY: union projection; the `Vec` index is bounds-checked.
    crate::state::with_state(|s| unsafe { s.mem.data[p as usize].b16.s1 })
}
/// Write `v` into `mem[p].b16.s1`.
#[inline]
pub(crate) fn set_mem_b16s1(p: i32, v: u16) {
    crate::state::with_state_mut(|s| {
        s.mem.data[p as usize].b16.s1 = v;
    });
}
/// Read `mem[p].b16.s2` (low 16 bits of the second 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s2(p: i32) -> u16 {
    // SAFETY: union projection; the `Vec` index is bounds-checked.
    crate::state::with_state(|s| unsafe { s.mem.data[p as usize].b16.s2 })
}
/// Read `mem[p].b16.s3` (high 16 bits of the second 32-bit half-word).
#[inline]
pub(crate) fn mem_b16s3(p: i32) -> u16 {
    // SAFETY: union projection; the `Vec` index is bounds-checked.
    crate::state::with_state(|s| unsafe { s.mem.data[p as usize].b16.s3 })
}
/// Write `v` into `mem[p].b16.s2`.
#[inline]
pub(crate) fn set_mem_b16s2(p: i32, v: u16) {
    crate::state::with_state_mut(|s| {
        s.mem.data[p as usize].b16.s2 = v;
    });
}

/// Read the `gr` (f64) field of `mem[p]`.
#[inline]
#[allow(dead_code)]
pub(crate) fn mem_gr(p: i32) -> f64 {
    // SAFETY: union projection; the `Vec` index is bounds-checked.
    crate::state::with_state(|s| unsafe { s.mem.data[p as usize].gr })
}

/// Returns `&mem[p]` as a raw pointer — for C functions that expect a
/// `memory_word*` node pointer (e.g. `real_get_native_italic_correction`).
/// Equivalent to the C macro `get_native_italic_correction(p)` = `&mem[p]`.
/// Valid past the borrow because the thread-local arena owner never reallocates.
#[inline]
pub(crate) fn mem_node_addr(p: i32) -> *mut std::ffi::c_void {
    crate::state::with_state_mut(|s| unsafe {
        s.mem.data.as_mut_ptr().add(p as usize) as *mut std::ffi::c_void
    })
}
/// Return a `*mut u16` pointing to `mem[p]` — for writing NATIVE_NODE_text.
/// Valid past the borrow because the thread-local arena owner never reallocates.
#[inline]
pub(crate) fn mem_as_u16_ptr_mut(p: i32) -> *mut u16 {
    crate::state::with_state_mut(|s| s.mem.data.as_mut_ptr().wrapping_add(p as usize) as *mut u16)
}

/// Copy the full MemoryWord at `mem[p]`.
#[inline]
pub(crate) fn mem_word(p: i32) -> crate::mem_types::MemoryWord {
    crate::state::with_state(|s| s.mem.data[p as usize])
}
/// Write a full MemoryWord to `mem[p]`.
#[inline]
pub(crate) fn set_mem_word(p: i32, w: crate::mem_types::MemoryWord) {
    crate::state::with_state_mut(|s| {
        s.mem.data[p as usize] = w;
    });
}

// ---------------------------------------------------------------------------
// eqtb accessor
// ---------------------------------------------------------------------------

/// Copy the full MemoryWord at `eqtb[p]`.
#[inline]
pub(crate) fn eqtb_word(p: i32) -> crate::mem_types::MemoryWord {
    crate::state::with_state(|s| s.eqtb.data[p as usize])
}
/// Write a full MemoryWord to `eqtb[p]`.
#[inline]
pub(crate) fn set_eqtb_word(p: i32, w: crate::mem_types::MemoryWord) {
    crate::state::with_state_mut(|s| {
        s.eqtb.data[p as usize] = w;
    });
}
/// Read `eqtb[p].b32.s1` (the "value" half of an equivalents entry).
#[inline]
pub(crate) fn eqtb_b32s1(p: i32) -> i32 {
    // SAFETY: `MemoryWord` union projection; caller ensures p is a valid eqtb index.
    crate::state::with_state(|s| unsafe { s.eqtb.data[p as usize].b32.s1 })
}
/// Write `v` into `eqtb[p].b32.s1`.
#[inline]
pub(crate) fn set_eqtb_b32s1(p: i32, v: i32) {
    // Union-field write through a `Vec` index needs no `unsafe` (session 41).
    crate::state::with_state_mut(|s| {
        s.eqtb.data[p as usize].b32.s1 = v;
    });
}
/// Read `eqtb[p].b16.s1` (the "type" half of an equivalents entry).
#[inline]
pub(crate) fn eqtb_b16s1(p: i32) -> u16 {
    // SAFETY: `MemoryWord` union projection; caller ensures p is a valid eqtb index.
    crate::state::with_state(|s| unsafe { s.eqtb.data[p as usize].b16.s1 })
}
/// Write `v` into `eqtb[p].b16.s1`.
#[inline]
pub(crate) fn set_eqtb_b16s1(p: i32, v: u16) {
    crate::state::with_state_mut(|s| {
        s.eqtb.data[p as usize].b16.s1 = v;
    });
}
/// Read `eqtb[p].b16.s0` (the "level" half of an equivalents entry).
#[inline]
pub(crate) fn eqtb_b16s0(p: i32) -> u16 {
    // SAFETY: `MemoryWord` union projection; caller ensures p is a valid eqtb index.
    crate::state::with_state(|s| unsafe { s.eqtb.data[p as usize].b16.s0 })
}
/// Write `v` into `eqtb[p].b16.s0`.
#[inline]
pub(crate) fn set_eqtb_b16s0(p: i32, v: u16) {
    crate::state::with_state_mut(|s| {
        s.eqtb.data[p as usize].b16.s0 = v;
    });
}

// ---------------------------------------------------------------------------
// str_ptr / str_start / str_pool (read accessor)
// ---------------------------------------------------------------------------

/// Returns `name_of_file` as a raw pointer (NUL-terminated C string).
#[inline]
pub(crate) fn name_of_file() -> *mut libc::c_char {
    crate::state::with_state(|s| {
        if s.input.name_of_file.is_empty() {
            std::ptr::null_mut()
        } else {
            s.input.name_of_file.as_ptr() as *mut libc::c_char
        }
    })
}
/// Write `name_of_file = v`.
#[inline]
pub(crate) fn set_name_of_file(v: *mut libc::c_char) {
    crate::state::with_state_mut(|s| {
        if v.is_null() {
            s.input.name_of_file.clear();
        } else {
            unsafe {
                let len = libc::strlen(v);
                s.input.name_of_file.resize(len + 1, 0);
                libc::strcpy(s.input.name_of_file.as_mut_ptr(), v);
            }
        }
    });
}
/// Write `name_length = v`.
#[inline]
pub(crate) fn set_name_length(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.name_length = v;
    });
}
/// Returns `name_length`.
#[inline]
pub(crate) fn name_length() -> i32 {
    crate::state::with_state(|s| s.input.name_length)
}

/// Returns the current number of strings in the string pool.
#[inline]
pub(crate) fn str_ptr() -> i32 {
    crate::state::with_state(|state| state.strings.str_ptr)
}

/// Returns `str_start[k]` — the pool offset of the k-th string.
#[inline]
pub(crate) fn str_start(k: i32) -> i32 {
    crate::state::with_state(|state| state.strings.str_start[k as usize])
}

// ---------------------------------------------------------------------------
// in_open / full_source_filename_stack / line_stack / file_line_error_style_p
// ---------------------------------------------------------------------------

/// Returns the current input nesting depth.
#[inline]
pub(crate) fn in_open() -> i32 {
    crate::state::with_state(|s| s.input.in_open)
}
/// Returns `pseudo_files` — head of the pseudo-file linked list (xetex-xetexd.h:704).
#[inline]
pub(crate) fn pseudo_files() -> i32 {
    crate::state::with_state(|s| s.input.pseudo_files)
}
/// Sets `pseudo_files`.
#[inline]
pub(crate) fn set_pseudo_files(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.pseudo_files = v;
    });
}

/// Returns non-zero when file:line-style error messages are active.
#[inline]
pub(crate) fn file_line_error_style_p() -> i32 {
    crate::state::with_state(|state| state.config.file_line_error_style_p)
}
/// Sets non-zero when file:line-style error messages are active.
#[inline]
pub(crate) fn set_file_line_error_style_p(v: i32) {
    crate::state::with_state_mut(|state| {
        state.config.file_line_error_style_p = v;
    });
}

// ---------------------------------------------------------------------------
// hash / prim / eqtb_top
// ---------------------------------------------------------------------------

/// Read `hash[p].s0` (the "link" half of a hash-table entry).
#[inline]
pub(crate) fn hash_s0(p: i32) -> i32 {
    crate::state::with_state(|s| s.eqtb.hash[(p as usize) - 514].s0)
}
/// Write `hash[p].s0 = v`.
#[inline]
pub(crate) fn set_hash_s0(p: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.eqtb.hash[(p as usize) - 514].s0 = v;
    });
}
/// Read `hash[p].s1` (the "text" half of a hash-table entry).
#[inline]
pub(crate) fn hash_s1(p: i32) -> i32 {
    crate::state::with_state(|s| s.eqtb.hash[(p as usize) - 514].s1)
}
/// Write `hash[p].s1 = v`.
#[inline]
pub(crate) fn set_hash_s1(p: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.eqtb.hash[(p as usize) - 514].s1 = v;
    });
}

/// Read `prim[p].s0` (the link half of a primitive-table entry).
#[inline]
pub(crate) fn prim_s0(p: i32) -> i32 {
    crate::state::with_state(|s| s.eqtb.prim[p as usize].s0)
}
/// Write `prim[p].s0 = v`.
#[inline]
pub(crate) fn set_prim_s0(p: i32, v: i32) {
    crate::state::with_state_mut(|s| s.eqtb.prim[p as usize].s0 = v);
}
/// Read `prim[p].s1` (the "text" half of a primitive-table entry).
#[inline]
pub(crate) fn prim_s1(p: i32) -> i32 {
    crate::state::with_state(|s| s.eqtb.prim[p as usize].s1)
}
/// Write `prim[p].s1 = v`.
#[inline]
pub(crate) fn set_prim_s1(p: i32, v: i32) {
    crate::state::with_state_mut(|s| s.eqtb.prim[p as usize].s1 = v);
}

/// Returns `eqtb_top`.
#[inline]
pub(crate) fn eqtb_top() -> i32 {
    crate::state::with_state(|state| state.eqtb.eqtb_top)
}

// ---------------------------------------------------------------------------
// hash management globals (hash_used / hash_extra / hash_high /
//                          no_new_control_sequence / prim_used)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn hash_used() -> i32 {
    crate::state::with_state(|state| state.eqtb.hash_used)
}
#[inline]
pub(crate) fn set_hash_used(v: i32) {
    crate::state::with_state_mut(|state| {
        state.eqtb.hash_used = v;
    });
}
#[inline]
pub(crate) fn hash_extra() -> i32 {
    crate::state::with_state(|state| state.eqtb.hash_extra)
}
#[inline]
pub(crate) fn hash_top() -> i32 {
    crate::state::with_state(|state| state.eqtb.hash_top)
}
#[inline]
pub(crate) fn hash_high() -> i32 {
    crate::state::with_state(|state| state.eqtb.hash_high)
}
#[inline]
pub(crate) fn set_hash_high(v: i32) {
    crate::state::with_state_mut(|state| {
        state.eqtb.hash_high = v;
    });
}
#[inline]
pub(crate) fn set_no_new_control_sequence(v: bool) {
    crate::state::with_state_mut(|state| {
        state.eqtb.no_new_control_sequence = v;
    });
}
#[inline]
pub(crate) fn set_prim_used(v: i32) {
    crate::state::with_state_mut(|state| {
        state.eqtb.prim_used = v;
    });
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
    unsafe {
        raw::temp_ptr = v;
    }
}

// ---------------------------------------------------------------------------
// dig
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// selector / term_offset / file_offset
// ---------------------------------------------------------------------------

/// Returns the current output selector (xetex-xetexd.h:431).
#[inline]
pub(crate) fn selector() -> i32 {
    crate::state::with_state(|state| state.shipout.selector)
}
/// Write `selector = v`.
#[inline]
pub(crate) fn set_selector(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.selector = v;
    });
}

/// Sets `term_offset`.
#[inline]
pub(crate) fn set_term_offset(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.term_offset = v;
    });
}

/// Sets `file_offset`.
#[inline]
pub(crate) fn set_file_offset(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.file_offset = v;
    });
}

// ---------------------------------------------------------------------------
// rust_stdout / log_file / write_file / current_diagnostic
// ---------------------------------------------------------------------------

/// Returns the `rust_stdout` output handle.
#[inline]
pub(crate) fn rust_stdout() -> usize {
    crate::state::with_state(|s| s.shipout.rust_stdout)
}

/// Sets the `rust_stdout` output handle.
#[inline]
pub(crate) fn set_rust_stdout(v: usize) {
    crate::state::with_state_mut(|s| {
        s.shipout.rust_stdout = v;
    });
}

/// Returns the `log_file` output handle.
#[inline]
pub(crate) fn log_file() -> usize {
    crate::state::with_state(|s| s.shipout.log_file)
}

/// Returns `write_file[i]`, the output handle for write-stream `i` (0–15).
#[inline]
pub(crate) fn write_file(i: usize) -> usize {
    crate::state::with_state(|s| s.shipout.write_file[i])
}

/// Returns `write_open[i]`, whether write-stream `i` is open (xetex-xetexd.h:692).
#[inline]
pub(crate) fn write_open(i: usize) -> bool {
    crate::state::with_state(|s| s.shipout.write_open[i])
}

/// Sets `write_open[i]` to `v`.
#[inline]
pub(crate) fn set_write_open(i: usize, v: bool) {
    crate::state::with_state_mut(|s| {
        s.shipout.write_open[i] = v;
    });
}

// ---------------------------------------------------------------------------
// print_raw_char globals
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn max_print_line() -> i32 {
    crate::state::with_state(|state| state.config.max_print_line)
}
#[inline]
pub(crate) fn set_max_print_line(v: i32) {
    crate::state::with_state_mut(|state| {
        state.config.max_print_line = v;
    });
}

#[inline]
pub(crate) fn set_tally(v: i32) {
    crate::state::with_state_mut(|state| {
        state.errors.tally = v;
    });
}

#[inline]
pub(crate) fn error_line() -> i32 {
    crate::state::with_state(|state| state.errors.error_line)
}
#[inline]
pub(crate) fn set_error_line(v: i32) {
    crate::state::with_state_mut(|state| {
        state.errors.error_line = v;
    });
}
#[inline]
pub(crate) fn half_error_line() -> i32 {
    crate::state::with_state(|state| state.errors.half_error_line)
}
#[inline]
pub(crate) fn set_half_error_line(v: i32) {
    crate::state::with_state_mut(|state| {
        state.errors.half_error_line = v;
    });
}

#[inline]
pub(crate) fn pool_size() -> i32 {
    crate::state::with_state(|state| state.strings.pool_size)
}
#[inline]
pub(crate) fn string_vacancies() -> i32 {
    crate::state::with_state(|state| state.strings.string_vacancies)
}
#[inline]
pub(crate) fn pool_ptr() -> i32 {
    crate::state::with_state(|state| state.strings.pool_ptr)
}
#[inline]
pub(crate) fn set_pool_ptr(v: i32) {
    crate::state::with_state_mut(|state| {
        state.strings.pool_ptr = v;
    });
}
// ---------------------------------------------------------------------------
// str_start write / str_ptr write / string pool limits
// ---------------------------------------------------------------------------

/// Write `str_start[k] = v`.
#[inline]
pub(crate) fn set_str_start(k: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.strings.str_start[k as usize] = v;
    });
}
/// Write `str_ptr = v`.
#[inline]
pub(crate) fn set_str_ptr(v: i32) {
    crate::state::with_state_mut(|state| {
        state.strings.str_ptr = v;
    });
}
#[inline]
pub(crate) fn max_strings() -> i32 {
    crate::state::with_state(|state| state.strings.max_strings)
}
#[inline]
pub(crate) fn init_str_ptr() -> i32 {
    crate::state::with_state(|state| state.strings.init_str_ptr)
}
#[inline]
pub(crate) fn init_pool_ptr() -> i32 {
    crate::state::with_state(|state| state.strings.init_pool_ptr)
}

// ---------------------------------------------------------------------------
// help_ptr / help_line
// ---------------------------------------------------------------------------

#[inline]
#[allow(dead_code)] // getter defined for completeness; only setter used so far
pub(crate) fn help_ptr() -> u8 {
    crate::state::with_state(|state| state.errors.help_ptr)
}
#[inline]
pub(crate) fn set_help_ptr(v: u8) {
    crate::state::with_state_mut(|state| {
        state.errors.help_ptr = v;
    });
}
/// Write `help_line[k] = v` (a C string pointer; caller owns the lifetime).
#[inline]
pub(crate) fn set_help_line(k: usize, v: *const libc::c_char) {
    crate::state::with_state_mut(|state| {
        state.errors.help_line[k] = v;
    });
}

// ---------------------------------------------------------------------------
// two_to_the / spec_log (read-only tables from xetex-ini.c)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn two_to_the(k: usize) -> i32 {
    crate::state::with_state(|state| state.errors.two_to_the[k])
}
#[inline]
pub(crate) fn set_two_to_the(k: usize, v: i32) {
    crate::state::with_state_mut(|state| {
        state.errors.two_to_the[k] = v;
    });
}
#[inline]
pub(crate) fn spec_log(k: usize) -> i32 {
    crate::state::with_state(|state| state.errors.spec_log[k])
}
#[inline]
pub(crate) fn set_spec_log(k: usize, v: i32) {
    crate::state::with_state_mut(|state| {
        state.errors.spec_log[k] = v;
    });
}

// ---------------------------------------------------------------------------
// interaction / history / error_count / use_err_help / halt_on_error_p
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn interaction() -> u8 {
    crate::state::with_state(|state| state.errors.interaction)
}
#[inline]
pub(crate) fn set_interaction(v: u8) {
    crate::state::with_state_mut(|state| {
        state.errors.interaction = v;
    });
}
#[inline]
pub(crate) fn history() -> i32 {
    crate::state::with_state(|state| state.errors.history)
}
#[inline]
pub(crate) fn set_history(v: i32) {
    crate::state::with_state_mut(|state| {
        state.errors.history = v;
    });
}
#[inline]
pub(crate) fn error_count() -> i8 {
    crate::state::with_state(|state| state.errors.error_count)
}
#[inline]
pub(crate) fn set_error_count(v: i8) {
    crate::state::with_state_mut(|state| {
        state.errors.error_count = v;
    });
}
#[inline]
pub(crate) fn set_use_err_help(v: bool) {
    crate::state::with_state_mut(|state| {
        state.errors.use_err_help = v;
    });
}
#[inline]
pub(crate) fn set_long_help_seen(v: bool) {
    crate::state::with_state_mut(|state| {
        state.errors.long_help_seen = v;
    });
}
#[inline]
pub(crate) fn log_opened() -> bool {
    crate::state::with_state(|state| state.errors.log_opened)
}
#[inline]
pub(crate) fn set_log_opened(v: bool) {
    crate::state::with_state_mut(|state| {
        state.errors.log_opened = v;
    });
}
#[inline]
pub(crate) fn job_name() -> i32 {
    crate::state::with_state(|state| state.strings.job_name)
}
#[inline]
pub(crate) fn set_job_name(v: i32) {
    crate::state::with_state_mut(|state| {
        state.strings.job_name = v;
    });
}
#[inline]
pub(crate) fn texmf_log_name() -> i32 {
    crate::state::with_state(|state| state.strings.texmf_log_name)
}
#[inline]
pub(crate) fn set_halt_on_error_p(v: i32) {
    crate::state::with_state_mut(|state| {
        state.config.halt_on_error_p = v;
    });
}

// ---------------------------------------------------------------------------
// engine launch config flags (set by tt_xetex_set_int_variable)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn in_initex_mode() -> bool {
    crate::state::with_state(|state| state.config.in_initex_mode)
}
#[inline]
pub(crate) fn set_in_initex_mode(v: bool) {
    crate::state::with_state_mut(|state| {
        state.config.in_initex_mode = v;
    });
}
#[inline]
pub(crate) fn set_synctex_enabled(v: i32) {
    crate::state::with_state_mut(|state| {
        state.config.synctex_enabled = v;
    });
}
#[inline]
pub(crate) fn semantic_pagination_enabled() -> bool {
    crate::state::with_state(|state| state.config.semantic_pagination_enabled)
}
#[inline]
pub(crate) fn set_semantic_pagination_enabled(v: bool) {
    crate::state::with_state_mut(|state| {
        state.config.semantic_pagination_enabled = v;
    });
}
#[inline]
pub(crate) fn set_shell_escape_enabled(v: bool) {
    crate::state::with_state_mut(|state| {
        state.config.shell_escape_enabled = v;
    });
}

// ---------------------------------------------------------------------------
// old_setting
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// cur_chr / cur_val / cur_name / cur_area / cur_ext
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn cur_cs() -> i32 {
    unsafe { raw::cur_cs }
}
#[inline]
pub(crate) fn set_cur_cs(v: i32) {
    unsafe {
        raw::cur_cs = v;
    }
}
#[inline]
pub(crate) fn cur_tok() -> i32 {
    unsafe { raw::cur_tok }
}
#[inline]
pub(crate) fn set_cur_tok(v: i32) {
    unsafe {
        raw::cur_tok = v;
    }
}
#[inline]
pub(crate) fn set_after_token(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.after_token = v;
    });
}
#[inline]
pub(crate) fn align_state() -> i32 {
    crate::state::with_state(|s| s.scanner.align_state)
}
#[inline]
pub(crate) fn set_align_state(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.align_state = v;
    });
}
#[inline]
pub(crate) fn cur_chr() -> i32 {
    unsafe { raw::cur_chr }
}
#[inline]
pub(crate) fn cur_val() -> i32 {
    unsafe { raw::cur_val }
}
#[inline]
pub(crate) fn set_cur_val(v: i32) {
    unsafe {
        raw::cur_val = v;
    }
}
#[inline]
pub(crate) fn cur_name() -> i32 {
    crate::state::with_state(|state| state.scanner.cur_name)
}
#[inline]
pub(crate) fn cur_area() -> i32 {
    crate::state::with_state(|state| state.scanner.cur_area)
}
#[inline]
pub(crate) fn cur_ext() -> i32 {
    crate::state::with_state(|state| state.scanner.cur_ext)
}

// ---------------------------------------------------------------------------
// name_in_progress / stop_at_space
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_name_in_progress(v: bool) {
    crate::state::with_state_mut(|state| {
        state.scanner.name_in_progress = v;
    });
}
#[inline]
pub(crate) fn set_stop_at_space(v: bool) {
    crate::state::with_state_mut(|state| {
        state.scanner.stop_at_space = v;
    });
}

// ---------------------------------------------------------------------------
// save_stack / save_ptr / cur_level / cur_group
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn save_ptr() -> i32 {
    crate::state::with_state(|state| state.save_stack.save_ptr)
}
#[inline]
pub(crate) fn set_save_ptr(v: i32) {
    crate::state::with_state_mut(|state| {
        state.save_stack.save_ptr = v;
    });
}
#[inline]
pub(crate) fn set_max_save_stack(v: i32) {
    crate::state::with_state_mut(|state| {
        state.save_stack.max_save_stack = v;
    });
}
#[inline]
pub(crate) fn save_size() -> i32 {
    unsafe { raw::save_size }
}
#[inline]
pub(crate) fn cur_level() -> u16 {
    crate::state::with_state(|state| state.save_stack.cur_level)
}
#[inline]
pub(crate) fn set_cur_level(v: u16) {
    crate::state::with_state_mut(|state| {
        state.save_stack.cur_level = v;
    });
}
#[inline]
pub(crate) fn cur_group() -> u8 {
    crate::state::with_state(|state| state.save_stack.cur_group)
}
#[inline]
pub(crate) fn set_cur_group(v: u8) {
    crate::state::with_state_mut(|state| {
        state.save_stack.cur_group = v;
    });
}
#[inline]
pub(crate) fn set_cur_boundary(v: i32) {
    crate::state::with_state_mut(|state| {
        state.save_stack.cur_boundary = v;
    });
}
/// Write `XEQ_LEVEL(p) = v`.
#[inline]
pub(crate) fn set_xeq_level(p: i32, v: u16) {
    crate::state::with_state_mut(|s| s.eqtb.xeq_level[(p - 7_826_729) as usize] = v);
}

// ---------------------------------------------------------------------------
// read_open / read_file (for \read stream management)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_read_open_entry(n: usize, v: u8) {
    crate::state::with_state_mut(|s| {
        s.input.read_open[n] = v;
    });
}

// ---------------------------------------------------------------------------
// font_name
// ---------------------------------------------------------------------------

/// Returns `font_name[f]` — the string-pool index for font f's name.
#[inline]
pub(crate) fn font_name_entry(f: i32) -> i32 {
    crate::state::with_state(|state| state.fonts.font_name[f as usize])
}

/// Writes `font_name[f] = v`.
#[inline]
pub(crate) fn set_font_name_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_name[f as usize] = v;
    });
}

/// Returns `font_info[k].b32.s1`.
#[inline]
pub(crate) fn font_info_b32s1(k: i32) -> i32 {
    crate::state::with_state(|s| unsafe { s.fonts.font_info[k as usize].b32.s1 })
}

/// Writes `font_info[k].b32.s1 = v`.
#[inline]
pub(crate) fn set_font_info_b32s1(k: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.font_info[k as usize].b32.s1 = v;
    });
}

/// Returns `fmem_ptr`.
#[inline]
pub(crate) fn fmem_ptr() -> i32 {
    crate::state::with_state(|s| s.fonts.fmem_ptr)
}

/// Sets `fmem_ptr`.
#[inline]
pub(crate) fn set_fmem_ptr(v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.fmem_ptr = v;
    });
}

/// Returns `font_ptr`.
#[inline]
pub(crate) fn font_ptr() -> i32 {
    crate::state::with_state(|s| s.fonts.font_ptr)
}

/// Sets `font_ptr`.
#[inline]
pub(crate) fn set_font_ptr(v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.font_ptr = v;
    });
}

/// Returns `font_mem_size`.
#[inline]
pub(crate) fn font_mem_size() -> i32 {
    crate::state::with_state(|state| state.fonts.font_mem_size)
}

/// Sets `font_mem_size`.
#[inline]
pub(crate) fn set_font_mem_size(v: i32) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_mem_size = v;
    });
}

/// Sets `font_params[f] = v`.
#[inline]
pub(crate) fn set_font_params(f: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_params[f as usize] = v;
    });
}

/// Returns `font_glue[f]`.
#[inline]
pub(crate) fn font_glue(f: i32) -> i32 {
    crate::state::with_state(|state| state.fonts.font_glue[f as usize])
}

/// Sets `font_glue[f] = v`.
#[inline]
pub(crate) fn set_font_glue(f: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_glue[f as usize] = v;
    });
}

/// Returns `param_base[f]`.
#[inline]
pub(crate) fn param_base(f: i32) -> i32 {
    crate::state::with_state(|s| s.fonts.param_base[f as usize])
}

/// Sets `param_base[f] = v`.
#[inline]
pub(crate) fn set_param_base(f: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.param_base[f as usize] = v;
    });
}

/// Returns `font_info[k].b16` (the B16x4 view of font_info word k).
#[inline]
pub(crate) fn font_info_b16(k: i32) -> crate::mem_types::B16x4 {
    crate::state::with_state(|s| unsafe { s.fonts.font_info[k as usize].b16 })
}

/// Raw borrow-free mirrors of the three `FontState` arrays the OpenType-math
/// C++ FFI (`xetex-XeTeXOTMath.cpp`) reads back via the `tt_get_font_*_entry`
/// callbacks below. Those callbacks run *inside* the C++ OT-math helpers, which
/// are themselves called from a threaded `_in` core holding `with_state_mut`
/// (the `mlist_to_hlist` SCC). A callback that took the `ENGINE_STATE` borrow
/// would nest-panic (`RefCell already borrowed`) — the session-206 wall — so it
/// must read through these raw pointers instead, like the `mod raw` deferred
/// set (`line`/`cur_input`).
///
/// INVARIANT: `refresh_font_ffi_mirrors` must run after **every** reallocation
/// of `fonts.{font_size,font_area,font_layout_engine}`. Those Vecs are sized to
/// `font_max + 1` and never grow mid-run (`font_ptr` is bounded by `font_max`),
/// so the data pointers stay stable except at the three reallocation sites:
/// engine init, format undump (`format_io`), and cleanup. In-place writes via
/// the `set_font_*_entry` accessors do not reallocate, so the mirrors keep
/// observing live values. A missed refresh after a new reallocation would be
/// silent UB — validated by the full corpus (`unicode_math`), not the dashboard.
mod font_ffi_mirror {
    use libc::c_void;
    pub(super) static mut FONT_SIZE: *const i32 = std::ptr::null();
    pub(super) static mut FONT_AREA: *const i32 = std::ptr::null();
    pub(super) static mut FONT_LAYOUT_ENGINE: *const *mut c_void = std::ptr::null();
}

/// Refresh the OT-math FFI mirror pointers from the current `FontState` Vec
/// buffers. Call this at every reallocation of the three mirrored arrays
/// (see `font_ffi_mirror`).
pub(crate) fn refresh_font_ffi_mirrors(fonts: &crate::state::FontState) {
    // SAFETY: single-threaded engine; the mirrors are only read by the
    // `tt_get_font_*_entry` callbacks, never concurrently with this write.
    unsafe {
        font_ffi_mirror::FONT_SIZE = fonts.font_size.as_ptr();
        font_ffi_mirror::FONT_AREA = fonts.font_area.as_ptr();
        font_ffi_mirror::FONT_LAYOUT_ENGINE = fonts.font_layout_engine.as_ptr();
    }
}

/// Returns `font_area[f]` — the str_number encoding the font's area.
#[inline]
pub(crate) fn font_area_entry(f: i32) -> i32 {
    crate::state::with_state(|state| state.fonts.font_area[f as usize])
}
/// Writes `font_area[f] = v`.
#[inline]
pub(crate) fn set_font_area_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_area[f as usize] = v;
    });
}
/// C++ bridge for OpenType math helpers that still need font_area. Reads the
/// borrow-free mirror so it is safe to call under a held `EngineState` borrow
/// (see `font_ffi_mirror`).
#[no_mangle]
pub extern "C" fn tt_get_font_area_entry(f: i32) -> i32 {
    // SAFETY: mirror kept in sync with fonts.font_area by refresh_font_ffi_mirrors.
    unsafe { *font_ffi_mirror::FONT_AREA.add(f as usize) }
}
/// Returns `font_size[f]` — the current at-size (scaled_t) for font f.
#[inline]
pub(crate) fn font_size_entry(f: i32) -> i32 {
    crate::state::with_state(|state| state.fonts.font_size[f as usize])
}
/// C++ bridge for the remaining OpenType math helper. Reads the borrow-free
/// mirror (see `font_ffi_mirror`).
#[no_mangle]
pub extern "C" fn tt_get_font_size_entry(f: i32) -> i32 {
    // SAFETY: mirror kept in sync with fonts.font_size by refresh_font_ffi_mirrors.
    unsafe { *font_ffi_mirror::FONT_SIZE.add(f as usize) }
}
/// Writes `font_size[f] = v`.
#[inline]
pub(crate) fn set_font_size_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_size[f as usize] = v;
    });
}
/// Returns `font_dsize[f]` — the design-size (scaled_t) for font f.
#[inline]
pub(crate) fn font_dsize_entry(f: i32) -> i32 {
    crate::state::with_state(|state| state.fonts.font_dsize[f as usize])
}
/// Writes `font_dsize[f] = v`.
#[inline]
pub(crate) fn set_font_dsize_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_dsize[f as usize] = v;
    });
}
/// Returns `font_bc[f]` — first valid char code.
#[inline]
pub(crate) fn font_bc_entry(f: i32) -> u16 {
    crate::state::with_state(|state| state.fonts.font_bc[f as usize])
}
/// Writes `font_bc[f] = v`.
#[inline]
pub(crate) fn set_font_bc_entry(f: i32, v: u16) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_bc[f as usize] = v;
    });
}
/// Returns `font_ec[f]` — last valid char code.
#[inline]
pub(crate) fn font_ec_entry(f: i32) -> u16 {
    crate::state::with_state(|state| state.fonts.font_ec[f as usize])
}
/// Writes `font_ec[f] = v`.
#[inline]
pub(crate) fn set_font_ec_entry(f: i32, v: u16) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_ec[f as usize] = v;
    });
}
/// Writes `bchar_label[f] = v`.
#[inline]
pub(crate) fn set_bchar_label_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.fonts.bchar_label[f as usize] = v;
    });
}
/// Writes `font_bchar[f] = v`.
#[inline]
pub(crate) fn set_font_bchar_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_bchar[f as usize] = v;
    });
}
/// Writes `font_false_bchar[f] = v`.
#[inline]
pub(crate) fn set_font_false_bchar_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_false_bchar[f as usize] = v;
    });
}
/// Returns `bchar_label[f]`.
#[inline]
pub(crate) fn bchar_label_entry(f: i32) -> i32 {
    crate::state::with_state(|state| state.fonts.bchar_label[f as usize])
}
/// Returns `font_bchar[f]`.
#[inline]
pub(crate) fn font_bchar_entry(f: i32) -> i32 {
    crate::state::with_state(|state| state.fonts.font_bchar[f as usize])
}
/// Returns `font_false_bchar[f]`.
#[inline]
pub(crate) fn font_false_bchar_entry(f: i32) -> i32 {
    crate::state::with_state(|state| state.fonts.font_false_bchar[f as usize])
}
/// Returns `font_layout_engine[f]`.
#[inline]
pub(crate) fn font_layout_engine_ptr(f: i32) -> *mut libc::c_void {
    crate::state::with_state(|s| s.fonts.font_layout_engine[f as usize])
}
/// Writes `font_layout_engine[f] = v`.
#[inline]
pub(crate) fn set_font_layout_engine_ptr(f: i32, v: *mut libc::c_void) {
    crate::state::with_state_mut(|s| {
        s.fonts.font_layout_engine[f as usize] = v;
    });
}
/// C++ bridge: returns `font_layout_engine[f]` for use in OpenType math helpers.
/// Reads the borrow-free mirror (see `font_ffi_mirror`).
#[no_mangle]
pub extern "C" fn tt_get_font_layout_engine_entry(f: i32) -> *mut libc::c_void {
    // SAFETY: mirror kept in sync with fonts.font_layout_engine by refresh_font_ffi_mirrors.
    unsafe { *font_ffi_mirror::FONT_LAYOUT_ENGINE.add(f as usize) }
}
/// Returns `font_mapping[f]`.
#[inline]
pub(crate) fn font_mapping_ptr(f: i32) -> *mut libc::c_void {
    crate::state::with_state(|s| s.fonts.font_mapping[f as usize])
}
/// Writes `font_mapping[f] = v`.
#[inline]
pub(crate) fn set_font_mapping_ptr(f: i32, v: *mut libc::c_void) {
    crate::state::with_state_mut(|s| {
        s.fonts.font_mapping[f as usize] = v;
    });
}
/// Returns `mapped_text[i]`, the UTF-16 output buffer from `apply_mapping`.
#[inline]
pub(crate) fn mapped_text_entry(i: i32) -> u16 {
    crate::state::with_state(|state| state.fonts.mapped_text[i as usize])
}

/// Returns `hyphen_char[f]`.
#[inline]
pub(crate) fn hyphen_char_entry(f: i32) -> i32 {
    crate::state::with_state(|s| s.fonts.hyphen_char[f as usize])
}
/// Writes `hyphen_char[f] = v`.
#[inline]
pub(crate) fn set_hyphen_char_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.hyphen_char[f as usize] = v;
    })
}
/// Writes `skew_char[f] = v`.
#[inline]
pub(crate) fn set_skew_char_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.skew_char[f as usize] = v;
    })
}
/// Returns `char_base[f]`.
#[inline]
pub(crate) fn char_base_entry(f: i32) -> i32 {
    crate::state::with_state(|s| s.fonts.char_base[f as usize])
}
/// Writes `char_base[f] = v`.
#[inline]
pub(crate) fn set_char_base_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.char_base[f as usize] = v;
    });
}
/// Returns `width_base[f]`.
#[inline]
pub(crate) fn width_base_entry(f: i32) -> i32 {
    crate::state::with_state(|s| s.fonts.width_base[f as usize])
}
/// Writes `width_base[f] = v`.
#[inline]
pub(crate) fn set_width_base_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.width_base[f as usize] = v;
    });
}
/// Returns `height_base[f]`.
#[inline]
pub(crate) fn height_base_entry(f: i32) -> i32 {
    crate::state::with_state(|s| s.fonts.height_base[f as usize])
}
/// Writes `height_base[f] = v`.
#[inline]
pub(crate) fn set_height_base_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.height_base[f as usize] = v;
    });
}
/// Writes `depth_base[f] = v`.
#[inline]
pub(crate) fn set_depth_base_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.depth_base[f as usize] = v;
    });
}
/// Writes `italic_base[f] = v`.
#[inline]
pub(crate) fn set_italic_base_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.italic_base[f as usize] = v;
    });
}
/// Returns `lig_kern_base[f]`.
#[inline]
pub(crate) fn lig_kern_base_entry(f: i32) -> i32 {
    crate::state::with_state(|s| s.fonts.lig_kern_base[f as usize])
}
/// Writes `lig_kern_base[f] = v`.
#[inline]
pub(crate) fn set_lig_kern_base_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.lig_kern_base[f as usize] = v;
    });
}
/// Returns `kern_base[f]`.
#[inline]
pub(crate) fn kern_base_entry(f: i32) -> i32 {
    crate::state::with_state(|s| s.fonts.kern_base[f as usize])
}
/// Writes `kern_base[f] = v`.
#[inline]
pub(crate) fn set_kern_base_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.kern_base[f as usize] = v;
    });
}
/// Writes `exten_base[f] = v`.
#[inline]
pub(crate) fn set_exten_base_entry(f: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.fonts.exten_base[f as usize] = v;
    });
}

#[inline]
pub(crate) fn set_is_in_csname(v: bool) {
    crate::state::with_state_mut(|s| {
        s.scanner.is_in_csname = v;
    });
}

// ---------------------------------------------------------------------------
// cur_val1 / radix
// ---------------------------------------------------------------------------
#[inline]
pub(crate) fn set_radix(v: u8) {
    crate::state::with_state_mut(|s| {
        s.scanner.radix = v;
    });
}

// ---------------------------------------------------------------------------
// page builder / last-node / output_active / random_seed / pdf positions
// ---------------------------------------------------------------------------
#[inline]
pub(crate) fn set_dead_cycles(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.dead_cycles = v;
    });
}
#[inline]
pub(crate) fn set_last_badness(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.last_badness = v;
    });
}
#[inline]
pub(crate) fn set_page_tail(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.page_tail = v;
    });
}
pub(crate) fn set_page_contents(v: u8) {
    crate::state::with_state_mut(|state| {
        state.shipout.page_contents = v;
    });
}
#[inline]
pub(crate) fn set_page_so_far(k: usize, v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.page_so_far[k] = v;
    });
}
#[inline]
pub(crate) fn last_glue() -> i32 {
    crate::state::with_state(|state| state.shipout.last_glue)
}
#[inline]
pub(crate) fn set_last_glue(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.last_glue = v;
    });
}
#[inline]
pub(crate) fn disc_ptr(k: usize) -> i32 {
    crate::state::with_state(|s| s.scanner.disc_ptr[k])
}
#[inline]
pub(crate) fn set_disc_ptr(k: usize, v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.disc_ptr[k] = v;
    });
}
#[inline]
pub(crate) fn set_last_penalty(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.last_penalty = v;
    });
}
#[inline]
pub(crate) fn set_last_kern(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.last_kern = v;
    });
}
#[inline]
pub(crate) fn set_last_node_type(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.last_node_type = v;
    });
}
pub(crate) fn set_insert_penalties(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.insert_penalties = v;
    });
}
#[inline]
pub(crate) fn set_output_active(v: bool) {
    crate::state::with_state_mut(|state| {
        state.shipout.output_active = v;
    });
}
#[inline]
pub(crate) fn random_seed() -> i32 {
    crate::state::with_state(|state| state.errors.random_seed)
}
#[inline]
pub(crate) fn set_random_seed(v: i32) {
    crate::state::with_state_mut(|state| {
        state.errors.random_seed = v;
    });
}

// ---------------------------------------------------------------------------
// epochseconds / microseconds
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn epochseconds() -> i32 {
    crate::state::with_state(|state| state.shipout.epochseconds)
}
#[inline]
pub(crate) fn set_epochseconds(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.epochseconds = v;
    });
}
#[inline]
pub(crate) fn microseconds() -> i32 {
    crate::state::with_state(|state| state.shipout.microseconds)
}
#[inline]
pub(crate) fn set_microseconds(v: i32) {
    crate::state::with_state_mut(|state| {
        state.shipout.microseconds = v;
    });
}

// ---------------------------------------------------------------------------
// mag_set
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_mag_set(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.mag_set = v;
    });
}

// ---------------------------------------------------------------------------
// semantic nest (nest / nest_ptr / max_nest_stack / nest_size / cur_list)
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_nest_ptr(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.nest_ptr = v;
    });
}
#[inline]
pub(crate) fn set_max_nest_stack(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.max_nest_stack = v;
    });
}
#[inline]
pub(crate) fn nest_size() -> i32 {
    unsafe { raw::nest_size }
}
/// Read `cur_list` (the raw current-list record) by value.
#[inline]
pub(crate) fn cur_list() -> crate::mem_types::ListStateRecord {
    unsafe { raw::cur_list }
}

/// Overwrite `cur_list` with a record by value. A raw `mod raw` write (no
/// `with_state`), so it is safe under a live `&mut EngineState` borrow.
#[inline]
pub(crate) fn set_cur_list_record(v: crate::mem_types::ListStateRecord) {
    unsafe {
        raw::cur_list = v;
    }
}

// cur_list field accessors
#[inline]
pub(crate) fn cur_list_mode() -> i16 {
    unsafe { raw::cur_list.mode }
}
#[inline]
pub(crate) fn set_cur_list_mode(v: i16) {
    unsafe {
        raw::cur_list.mode = v;
    }
}
#[inline]
pub(crate) fn cur_list_head() -> i32 {
    unsafe { raw::cur_list.head }
}
#[inline]
pub(crate) fn set_cur_list_head(v: i32) {
    unsafe {
        raw::cur_list.head = v;
    }
}
#[inline]
pub(crate) fn cur_list_tail() -> i32 {
    unsafe { raw::cur_list.tail }
}
#[inline]
pub(crate) fn set_cur_list_tail(v: i32) {
    unsafe {
        raw::cur_list.tail = v;
    }
}
#[inline]
pub(crate) fn cur_list_prev_graf() -> i32 {
    unsafe { raw::cur_list.prev_graf }
}
/// Returns `cur_list.aux.b32.s0` (the integer/spacefactor half of aux).
#[inline]
pub(crate) fn cur_list_aux_b32s0() -> i32 {
    unsafe { raw::cur_list.aux.b32.s0 }
}
/// Sets `cur_list.aux.b32.s0 = v`.
#[inline]
pub(crate) fn set_cur_list_aux_b32s0(v: i32) {
    unsafe {
        raw::cur_list.aux.b32.s0 = v;
    }
}
/// Returns `cur_list.aux.b32.s1` (the prevdepth half of aux).
#[inline]
pub(crate) fn cur_list_aux_b32s1() -> i32 {
    unsafe { raw::cur_list.aux.b32.s1 }
}
/// Sets `cur_list.aux.b32.s1 = v`.
#[inline]
pub(crate) fn set_cur_list_aux_b32s1(v: i32) {
    unsafe {
        raw::cur_list.aux.b32.s1 = v;
    }
}
#[inline]
pub(crate) fn set_cur_list_prev_graf(v: i32) {
    unsafe {
        raw::cur_list.prev_graf = v;
    }
}
#[inline]
pub(crate) fn set_cur_list_mode_line(v: i32) {
    unsafe {
        raw::cur_list.mode_line = v;
    }
}
#[inline]
pub(crate) fn cur_list_e_tex_aux() -> i32 {
    unsafe { raw::cur_list.e_tex_aux }
}
#[inline]
pub(crate) fn set_cur_list_e_tex_aux(v: i32) {
    unsafe {
        raw::cur_list.e_tex_aux = v;
    }
}

// ---------------------------------------------------------------------------
// font_max
// ---------------------------------------------------------------------------

/// Returns `font_max` — the largest valid internal font number
/// (xetex-xetexd.h:406).
#[inline]
pub(crate) fn font_max() -> i32 {
    crate::state::with_state(|state| state.fonts.font_max)
}

/// Sets `font_max`.
#[inline]
pub(crate) fn set_font_max(v: i32) {
    crate::state::with_state_mut(|state| {
        state.fonts.font_max = v;
    });
}

// ---------------------------------------------------------------------------
// cur_cmd / cur_mark
// ---------------------------------------------------------------------------

/// Returns the current command code (`cur_cmd`, xetex-xetexd.h:499).
#[inline]
pub(crate) fn cur_cmd() -> u8 {
    unsafe { raw::cur_cmd }
}
/// Sets `cur_cmd`.
#[inline]
pub(crate) fn set_cur_cmd(v: u8) {
    unsafe {
        raw::cur_cmd = v;
    }
}
/// Sets `cur_chr`.
#[inline]
pub(crate) fn set_cur_chr(v: i32) {
    unsafe {
        raw::cur_chr = v;
    }
}

/// Returns `cur_mark[k]` — the k-th mark register token list
/// (xetex-xetexd.h:527; k must be 0..=4).
#[inline]
pub(crate) fn cur_mark(k: usize) -> i32 {
    crate::state::with_state(|s| s.scanner.cur_mark[k])
}

/// Sets `cur_mark[k]`.
#[inline]
pub(crate) fn set_cur_mark(k: usize, v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_mark[k] = v;
    });
}

// ---------------------------------------------------------------------------
// input_stack / input_ptr / base_ptr
// ---------------------------------------------------------------------------

/// Returns `input_ptr` — current top of the input-stack (xetex-xetexd.h:504).
#[inline]
pub(crate) fn input_ptr() -> i32 {
    crate::state::with_state(|s| s.input.input_ptr)
}
/// Sets `input_ptr`.
#[inline]
pub(crate) fn set_input_ptr(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.input_ptr = v;
    });
}
/// Sets `max_in_stack`.
#[inline]
pub(crate) fn set_max_in_stack(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.max_in_stack = v;
    });
}
/// Returns `stack_size` — capacity of the input stack (xetex-xetexd.h:410).
#[inline]
pub(crate) fn stack_size() -> i32 {
    unsafe { raw::stack_size }
}

/// Returns `par_loc` — hash-table location of \par (xetex-xetexd.h:522).
#[inline]
pub(crate) fn par_loc() -> i32 {
    crate::state::with_state(|s| s.input.par_loc)
}
/// Sets `par_loc`.
#[inline]
pub(crate) fn set_par_loc(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.par_loc = v;
    });
}
/// Sets `par_token`.
#[inline]
pub(crate) fn set_par_token(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.par_token = v;
    });
}

// ---------------------------------------------------------------------------
// grp_stack / if_stack
// ---------------------------------------------------------------------------

/// Sets `grp_stack[k]`.
#[inline]
pub(crate) fn set_grp_stack(k: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.grp_stack[k as usize] = v;
    });
}

/// Sets `if_stack[k]`.
#[inline]
pub(crate) fn set_if_stack(k: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.if_stack[k as usize] = v;
    });
}

// ---------------------------------------------------------------------------
// cond_ptr / if_limit / cur_if / if_line
// ---------------------------------------------------------------------------

/// Returns `cond_ptr` — head of the conditional-stack list
/// (xetex-xetexd.h:537).
#[inline]
pub(crate) fn cond_ptr() -> i32 {
    crate::state::with_state(|s| s.scanner.cond_ptr)
}
/// Sets `cond_ptr`.
#[inline]
pub(crate) fn set_cond_ptr(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.cond_ptr = v;
    });
}

/// Returns `if_limit` — current \if nesting limit code
/// (xetex-xetexd.h:538; unsigned char).
#[inline]
pub(crate) fn if_limit() -> u8 {
    crate::state::with_state(|s| s.scanner.if_limit)
}
/// Sets `if_limit`.
#[inline]
pub(crate) fn set_if_limit(v: u8) {
    crate::state::with_state_mut(|s| {
        s.scanner.if_limit = v;
    });
}

/// Returns `cur_if` — current \if type (xetex-xetexd.h:539; small_number = i16).
#[inline]
pub(crate) fn cur_if() -> i16 {
    crate::state::with_state(|s| s.scanner.cur_if)
}
/// Sets `cur_if`.
#[inline]
pub(crate) fn set_cur_if(v: i16) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_if = v;
    });
}

/// Returns `if_line` — line number where the current \if was entered
/// (xetex-xetexd.h:540).
#[inline]
pub(crate) fn if_line() -> i32 {
    crate::state::with_state(|s| s.scanner.if_line)
}
/// Sets `if_line`.
#[inline]
pub(crate) fn set_if_line(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.if_line = v;
    });
}

// ---------------------------------------------------------------------------
// sa_root / sa_chain / sa_level
// ---------------------------------------------------------------------------

/// Read `sa_root[i]` (the root of sparse-array trie for class i).
#[inline]
pub(crate) fn sa_root(i: usize) -> i32 {
    crate::state::with_state(|s| s.scanner.sa_root[i])
}
/// Write `sa_root[i] = v`.
#[inline]
pub(crate) fn set_sa_root(i: usize, v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.sa_root[i] = v;
    });
}

#[inline]
pub(crate) fn set_sa_chain(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.sa_chain = v;
    });
}
#[inline]
pub(crate) fn set_sa_level(v: u16) {
    crate::state::with_state_mut(|s| {
        s.scanner.sa_level = v;
    });
}

// ---------------------------------------------------------------------------
// cur_ptr / sa_null
// ---------------------------------------------------------------------------

/// Returns `cur_ptr` — SA trie traversal cursor (xetex-xetexd.h:710).
#[inline]
pub(crate) fn cur_ptr() -> i32 {
    unsafe { raw::cur_ptr }
}
/// Sets `cur_ptr`.
#[inline]
pub(crate) fn set_cur_ptr(v: i32) {
    unsafe {
        raw::cur_ptr = v;
    }
}
// ---------------------------------------------------------------------------
// shown_mode
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_shown_mode(v: i16) {
    crate::state::with_state_mut(|s| {
        s.scanner.shown_mode = v;
    });
}

// ---------------------------------------------------------------------------
// max_reg_num / max_reg_help_line
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_max_reg_num(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.max_reg_num = v;
    });
}
#[inline]
pub(crate) fn set_max_reg_help_line(v: *const libc::c_char) {
    crate::state::with_state_mut(|s| {
        s.scanner.max_reg_help_line = v;
    });
}
#[inline]
pub(crate) fn var_used() -> i32 {
    crate::state::with_state(|s| s.mem.var_used)
}
#[inline]
pub(crate) fn set_var_used(v: i32) {
    crate::state::with_state_mut(|s| {
        s.mem.var_used = v;
    });
}
#[inline]
pub(crate) fn dyn_used() -> i32 {
    crate::state::with_state(|s| s.mem.dyn_used)
}
#[inline]
pub(crate) fn set_dyn_used(v: i32) {
    crate::state::with_state_mut(|s| {
        s.mem.dyn_used = v;
    });
}
#[inline]
pub(crate) fn cs_count() -> i32 {
    crate::state::with_state(|state| state.eqtb.cs_count)
}
#[inline]
pub(crate) fn set_cs_count(v: i32) {
    crate::state::with_state_mut(|state| {
        state.eqtb.cs_count = v;
    });
}

// ---------------------------------------------------------------------------
// param_stack / param_ptr / max_param_stack
// ---------------------------------------------------------------------------

/// Sets `param_ptr`.
#[inline]
pub(crate) fn set_param_ptr(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.param_ptr = v;
    });
}
/// Sets `max_param_stack`.
#[inline]
pub(crate) fn set_max_param_stack(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.max_param_stack = v;
    });
}
/// Returns `param_size` — allocated capacity of param_stack (xetex-ini.c:3596).
#[inline]
pub(crate) fn param_size() -> i32 {
    unsafe { raw::param_size }
}

// ---------------------------------------------------------------------------
// first / buf_size / max_in_open
// ---------------------------------------------------------------------------

/// Returns `first` — next free index in the input buffer (xetex-xetexd.h:393).
#[inline]
pub(crate) fn first() -> i32 {
    unsafe { raw::first }
}
/// Sets `first`.
#[inline]
pub(crate) fn set_first(v: i32) {
    unsafe {
        raw::first = v;
    }
}
/// Returns `buf_size` — capacity of the input buffer (xetex-xetexd.h:409).
#[inline]
pub(crate) fn buf_size() -> i32 {
    unsafe { raw::buf_size }
}
/// Returns `max_in_open` — maximum allowed input-nesting depth (xetex-xetexd.h:411).
#[inline]
pub(crate) fn max_in_open() -> i32 {
    unsafe { raw::max_in_open }
}

// ---------------------------------------------------------------------------
// set_in_open
// ---------------------------------------------------------------------------

/// Sets `in_open`.
#[inline]
pub(crate) fn set_in_open(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.in_open = v;
    });
}

// ---------------------------------------------------------------------------
// source_filename_stack / full_source_filename_stack / line_stack
// ---------------------------------------------------------------------------

/// Returns `source_filename_stack[k]` (str_number; xetex-xetexd.h:512).
#[inline]
pub(crate) fn source_filename_stack(k: i32) -> i32 {
    crate::state::with_state(|s| s.input.source_filename_stack[k as usize])
}
/// Sets `source_filename_stack[k]`.
#[inline]
pub(crate) fn set_source_filename_stack(k: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.source_filename_stack[k as usize] = v;
    });
}
/// Sets `full_source_filename_stack[k]`.
#[inline]
pub(crate) fn set_full_source_filename_stack(k: i32, v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.full_source_filename_stack[k as usize] = v;
    });
}
/// Sets `line`.
#[inline]
pub(crate) fn set_line(v: i32) {
    unsafe {
        raw::line = v;
    }
}

// ---------------------------------------------------------------------------
// eof_seen
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// input_file
// ---------------------------------------------------------------------------

/// Returns `input_file[k]` as a raw pointer (UFILE*; xetex-xetexd.h:509).
#[inline]
pub(crate) fn input_file(k: i32) -> *mut crate::io::UFile {
    crate::state::with_state(|s| s.input.input_file[k as usize])
}
/// Sets `input_file[k]`.
#[inline]
pub(crate) fn set_input_file(k: i32, v: *mut crate::io::UFile) {
    crate::state::with_state_mut(|s| s.input.input_file[k as usize] = v);
}

// ---------------------------------------------------------------------------
// last / max_buf_stack / set_buffer_entry / cur_input_limit
// ---------------------------------------------------------------------------

/// Returns `last` — index one past the last character on the current input line.
#[inline]
pub(crate) fn last() -> i32 {
    crate::state::with_state(|s| s.input.last)
}
/// Sets `last`.
#[inline]
pub(crate) fn set_last(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.last = v;
    });
}
/// Sets `max_buf_stack`.
#[inline]
pub(crate) fn set_max_buf_stack(v: i32) {
    crate::state::with_state_mut(|s| {
        s.input.max_buf_stack = v;
    });
}
/// Writes `buffer[k] = v`.
#[inline]
pub(crate) fn set_buffer_entry(k: i32, v: i32) {
    crate::state::with_state_mut(|s| s.input.buffer[k as usize] = v);
}
/// Writes `cur_input.limit = v`.
#[inline]
pub(crate) fn set_cur_input_limit(v: i32) {
    unsafe {
        raw::cur_input.limit = v;
    }
}

// ---------------------------------------------------------------------------
// expand_depth / expand_depth_count / cur_val_level
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn expand_depth() -> i32 {
    unsafe { raw::expand_depth }
}
#[inline]
pub(crate) fn set_expand_depth_count(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.expand_depth_count = v;
    });
}
#[inline]
pub(crate) fn set_cur_val_level(v: u8) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_val_level = v;
    });
}

// ---------------------------------------------------------------------------
// scanner_status / warning_index / def_ref
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn scanner_status() -> u8 {
    crate::state::with_state(|s| s.scanner.scanner_status)
}
#[inline]
pub(crate) fn set_scanner_status(v: u8) {
    crate::state::with_state_mut(|s| {
        s.scanner.scanner_status = v;
    });
}
#[inline]
pub(crate) fn set_warning_index(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.warning_index = v;
    });
}
#[inline]
pub(crate) fn def_ref() -> i32 {
    crate::state::with_state(|s| s.scanner.def_ref)
}
#[inline]
pub(crate) fn set_def_ref(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.def_ref = v;
    });
}

#[inline]
pub(crate) fn set_deletions_allowed(v: bool) {
    crate::state::with_state_mut(|state| {
        state.errors.deletions_allowed = v;
    });
}

// ---------------------------------------------------------------------------
// gave_char_warning_help / main_p / main_k
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_gave_char_warning_help(v: bool) {
    crate::state::with_state_mut(|state| {
        state.fonts.gave_char_warning_help = v;
    });
}
#[inline]
pub(crate) fn main_p() -> i32 {
    unsafe { raw::main_p }
}
#[inline]
pub(crate) fn set_main_p(v: i32) {
    unsafe {
        raw::main_p = v;
    }
}
#[inline]
pub(crate) fn main_k() -> i32 {
    unsafe { raw::main_k }
}
#[inline]
pub(crate) fn set_main_k(v: i32) {
    unsafe {
        raw::main_k = v;
    }
}

// ---------------------------------------------------------------------------
// main_loop inner-loop globals
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn main_f() -> i32 {
    unsafe { raw::main_f }
}
#[inline]
pub(crate) fn set_main_f(v: i32) {
    unsafe {
        raw::main_f = v;
    }
}
#[inline]
pub(crate) fn main_h() -> i32 {
    unsafe { raw::main_h }
}
#[inline]
pub(crate) fn set_main_h(v: i32) {
    unsafe {
        raw::main_h = v;
    }
}
#[inline]
pub(crate) fn main_pp() -> i32 {
    unsafe { raw::main_pp }
}
#[inline]
pub(crate) fn set_main_pp(v: i32) {
    unsafe {
        raw::main_pp = v;
    }
}
#[inline]
pub(crate) fn main_ppp() -> i32 {
    unsafe { raw::main_ppp }
}
#[inline]
pub(crate) fn set_main_ppp(v: i32) {
    unsafe {
        raw::main_ppp = v;
    }
}

#[inline]
pub(crate) fn set_main_s(v: i32) {
    unsafe {
        raw::main_s = v;
    }
}
#[inline]
pub(crate) fn cur_l() -> i32 {
    unsafe { raw::cur_l }
}
#[inline]
pub(crate) fn set_cur_l(v: i32) {
    unsafe {
        raw::cur_l = v;
    }
}
#[inline]
pub(crate) fn cur_r() -> i32 {
    unsafe { raw::cur_r }
}
#[inline]
pub(crate) fn set_cur_r(v: i32) {
    unsafe {
        raw::cur_r = v;
    }
}
#[inline]
pub(crate) fn cur_q() -> i32 {
    unsafe { raw::cur_q }
}
#[inline]
pub(crate) fn set_cur_q(v: i32) {
    unsafe {
        raw::cur_q = v;
    }
}
#[inline]
pub(crate) fn lig_stack() -> i32 {
    unsafe { raw::lig_stack }
}
#[inline]
pub(crate) fn set_lig_stack(v: i32) {
    unsafe {
        raw::lig_stack = v;
    }
}
#[inline]
pub(crate) fn ligature_present() -> bool {
    unsafe { raw::ligature_present }
}
#[inline]
pub(crate) fn set_ligature_present(v: bool) {
    unsafe {
        raw::ligature_present = v;
    }
}
#[inline]
pub(crate) fn lft_hit() -> bool {
    unsafe { raw::lft_hit }
}
#[inline]
pub(crate) fn set_lft_hit(v: bool) {
    unsafe {
        raw::lft_hit = v;
    }
}
#[inline]
pub(crate) fn rt_hit() -> bool {
    unsafe { raw::rt_hit }
}
#[inline]
pub(crate) fn set_rt_hit(v: bool) {
    unsafe {
        raw::rt_hit = v;
    }
}
#[inline]
pub(crate) fn native_text() -> *mut u16 {
    crate::state::with_state(|s| {
        if s.scanner.native_text.is_empty() {
            std::ptr::null_mut()
        } else {
            s.scanner.native_text.as_ptr() as *mut u16
        }
    })
}
#[inline]
pub(crate) fn set_native_text(v: *mut u16) {
    crate::state::with_state_mut(|s| {
        if v.is_null() {
            s.scanner.native_text.clear();
        } else {
            let len = s.scanner.native_text_size as usize;
            s.scanner.native_text.resize(len, 0);
            unsafe {
                std::ptr::copy_nonoverlapping(v, s.scanner.native_text.as_mut_ptr(), len);
            }
        }
    });
}
#[inline]
pub(crate) fn native_text_size() -> i32 {
    crate::state::with_state(|s| s.scanner.native_text_size)
}
#[inline]
pub(crate) fn set_native_text_size(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.native_text_size = v;
    });
}
#[inline]
pub(crate) fn native_len() -> i32 {
    crate::state::with_state(|s| s.scanner.native_len)
}
#[inline]
pub(crate) fn set_native_len(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.native_len = v;
    });
}
#[inline]
pub(crate) fn save_native_len() -> i32 {
    crate::state::with_state(|s| s.scanner.save_native_len)
}
#[inline]
pub(crate) fn set_save_native_len(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.save_native_len = v;
    });
}
#[inline]
pub(crate) fn is_hyph() -> bool {
    crate::state::with_state(|state| state.hyph.is_hyph)
}
#[inline]
pub(crate) fn set_is_hyph(v: bool) {
    crate::state::with_state_mut(|state| {
        state.hyph.is_hyph = v;
    });
}
#[inline]
pub(crate) fn bchar() -> i32 {
    unsafe { raw::bchar }
}
#[inline]
pub(crate) fn set_bchar(v: i32) {
    unsafe {
        raw::bchar = v;
    }
}
#[inline]
pub(crate) fn false_bchar() -> i32 {
    unsafe { raw::false_bchar }
}
#[inline]
pub(crate) fn set_false_bchar(v: i32) {
    unsafe {
        raw::false_bchar = v;
    }
}
#[inline]
pub(crate) fn cancel_boundary() -> bool {
    unsafe { raw::cancel_boundary }
}
#[inline]
pub(crate) fn set_cancel_boundary(v: bool) {
    unsafe {
        raw::cancel_boundary = v;
    }
}
#[inline]
pub(crate) fn ins_disc() -> bool {
    unsafe { raw::ins_disc }
}
#[inline]
pub(crate) fn set_ins_disc(v: bool) {
    unsafe {
        raw::ins_disc = v;
    }
}
#[inline]
pub(crate) fn insert_src_special_auto() -> bool {
    crate::state::with_state(|state| state.shipout.insert_src_special_auto)
}

#[inline]
pub(crate) fn space_class() -> i32 {
    unsafe { raw::space_class }
}
#[inline]
pub(crate) fn set_space_class(v: i32) {
    unsafe {
        raw::space_class = v;
    }
}
#[inline]
pub(crate) fn prev_class() -> i32 {
    unsafe { raw::prev_class }
}
#[inline]
pub(crate) fn set_prev_class(v: i32) {
    unsafe {
        raw::prev_class = v;
    }
}

// ---------------------------------------------------------------------------
// hyphenation globals / cur_lang / insert_src_special_every_par
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn max_hyph_char() -> i32 {
    crate::state::with_state(|state| state.hyph.max_hyph_char)
}
#[inline]
pub(crate) fn set_max_hyph_char(v: i32) {
    crate::state::with_state_mut(|state| {
        state.hyph.max_hyph_char = v;
    });
}
#[inline]
pub(crate) fn set_trie_c_entry(k: i32, v: u16) {
    crate::state::with_state_mut(|state| {
        state.hyph.trie_c[k as usize] = v;
    });
}
#[inline]
pub(crate) fn set_trie_l_entry(k: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.hyph.trie_l[k as usize] = v;
    });
}
#[inline]
pub(crate) fn set_trie_r_entry(k: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.hyph.trie_r[k as usize] = v;
    });
}
#[inline]
pub(crate) fn trie_max() -> i32 {
    crate::state::with_state(|state| state.hyph.trie_max)
}
#[inline]
pub(crate) fn set_trie_ptr(v: i32) {
    crate::state::with_state_mut(|state| {
        state.hyph.trie_ptr = v;
    });
}
#[inline]
pub(crate) fn trie_not_ready() -> bool {
    crate::state::with_state(|state| state.hyph.trie_not_ready)
}
#[inline]
pub(crate) fn set_trie_not_ready(v: bool) {
    crate::state::with_state_mut(|state| {
        state.hyph.trie_not_ready = v;
    });
}
#[inline]
pub(crate) fn hyph_start() -> i32 {
    crate::state::with_state(|state| state.hyph.hyph_start)
}
#[inline]
pub(crate) fn set_hyph_start(v: i32) {
    crate::state::with_state_mut(|state| {
        state.hyph.hyph_start = v;
    });
}
#[inline]
pub(crate) fn trie_used_entry(k: usize) -> u16 {
    crate::state::with_state(|state| state.hyph.trie_used[k])
}
#[inline]
pub(crate) fn set_trie_used_entry(k: usize, v: u16) {
    crate::state::with_state_mut(|state| {
        state.hyph.trie_used[k] = v;
    });
}
#[inline]
pub(crate) fn trie_op_ptr() -> i32 {
    crate::state::with_state(|state| state.hyph.trie_op_ptr)
}
#[inline]
pub(crate) fn set_trie_op_ptr(v: i32) {
    crate::state::with_state_mut(|state| {
        state.hyph.trie_op_ptr = v;
    });
}
pub(crate) fn set_max_op_used(v: u16) {
    crate::state::with_state_mut(|state| {
        state.hyph.max_op_used = v;
    });
}
#[inline]
pub(crate) fn hyph_word_entry(k: i32) -> i32 {
    crate::state::with_state(|state| state.hyph.hyph_word[k as usize])
}
#[inline]
pub(crate) fn hyph_list_entry(k: i32) -> i32 {
    crate::state::with_state(|state| state.hyph.hyph_list[k as usize])
}
#[inline]
pub(crate) fn hyph_link_entry(k: i32) -> u16 {
    crate::state::with_state(|state| state.hyph.hyph_link[k as usize])
}
#[inline]
pub(crate) fn set_hyph_word_entry(k: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.hyph.hyph_word[k as usize] = v;
    });
}
#[inline]
pub(crate) fn set_hyph_list_entry(k: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.hyph.hyph_list[k as usize] = v;
    });
}
#[inline]
pub(crate) fn set_hyph_link_entry(k: i32, v: u16) {
    crate::state::with_state_mut(|state| {
        state.hyph.hyph_link[k as usize] = v;
    });
}
#[inline]
pub(crate) fn hyph_count() -> i32 {
    crate::state::with_state(|state| state.hyph.hyph_count)
}
#[inline]
pub(crate) fn set_hyph_count(v: i32) {
    crate::state::with_state_mut(|state| {
        state.hyph.hyph_count = v;
    });
}
#[inline]
pub(crate) fn hyph_size() -> i32 {
    crate::state::with_state(|state| state.hyph.hyph_size)
}
#[inline]
pub(crate) fn hyph_next() -> i32 {
    crate::state::with_state(|state| state.hyph.hyph_next)
}
#[inline]
pub(crate) fn set_hyph_next(v: i32) {
    crate::state::with_state_mut(|state| {
        state.hyph.hyph_next = v;
    });
}

// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// cur_order
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_cur_order(v: u8) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_order = v;
    });
}

// ---------------------------------------------------------------------------
// alignment state
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_cur_align(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_align = v;
    });
}
#[inline]
pub(crate) fn set_cur_span(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_span = v;
    });
}
#[inline]
pub(crate) fn set_cur_loop(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_loop = v;
    });
}
#[inline]
pub(crate) fn set_align_ptr(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.align_ptr = v;
    });
}
#[inline]
pub(crate) fn set_cur_head(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_head = v;
    });
}
#[inline]
pub(crate) fn set_cur_tail_a(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_tail = v;
    });
}
#[inline]
pub(crate) fn set_cur_pre_head(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_pre_head = v;
    });
}
#[inline]
pub(crate) fn set_cur_pre_tail(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_pre_tail = v;
    });
}

// ---------------------------------------------------------------------------
// hpack adjust-tail / total_stretch / total_shrink
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_adjust_tail(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.adjust_tail = v;
    });
}
#[inline]
pub(crate) fn set_pre_adjust_tail(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.pre_adjust_tail = v;
    });
}
#[inline]
pub(crate) fn set_cur_f(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_f = v;
    });
}
#[inline]
pub(crate) fn set_pack_begin_line(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.pack_begin_line = v;
    });
}
#[inline]
pub(crate) fn set_set_box_allowed(v: bool) {
    crate::state::with_state_mut(|s| {
        s.scanner.set_box_allowed = v;
    });
}

/// Returns `cur_list.mode_line` (the source line where the current mode started).
#[inline]
pub(crate) fn cur_list_mode_line() -> i32 {
    unsafe { raw::cur_list.mode_line }
}

// ---------------------------------------------------------------------------
// active_width / best_height_plus_depth
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// LR_ptr / LR_problems / cur_dir
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn set_lr_ptr(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.LR_ptr = v;
    });
}
#[inline]
pub(crate) fn set_lr_problems(v: i32) {
    crate::state::with_state_mut(|s| {
        s.scanner.LR_problems = v;
    });
}
#[inline]
pub(crate) fn set_cur_dir(v: i16) {
    crate::state::with_state_mut(|s| {
        s.scanner.cur_dir = v;
    });
}
// ---------------------------------------------------------------------------
// force_eof (xetex-ini.c:159 / xetex-xetexd.h:524)
// ---------------------------------------------------------------------------
#[inline]
pub(crate) fn set_force_eof(v: bool) {
    crate::state::with_state_mut(|s| {
        s.input.force_eof = v;
    });
}

// ---------------------------------------------------------------------------
// used_tectonic_coda_tokens (xetex-xetexd.h:723)
// ---------------------------------------------------------------------------
#[inline]
pub(crate) fn set_used_tectonic_coda_tokens(v: bool) {
    crate::state::with_state_mut(|s| {
        s.scanner.used_tectonic_coda_tokens = v;
    });
}

// ---------------------------------------------------------------------------
// Shipout / DVI output globals (xetex-xetexd.h:592–600, 694–697, 693)
// ---------------------------------------------------------------------------
#[inline]
pub(crate) fn set_null_character(v: crate::mem_types::B16x4) {
    crate::state::with_state_mut(|state| {
        state.fonts.null_character = v;
    });
}
#[inline]
pub(crate) fn set_total_pages(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.total_pages = v);
}
#[inline]
pub(crate) fn set_max_v(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.max_v = v);
}
#[inline]
pub(crate) fn set_max_h(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.max_h = v);
}
#[inline]
pub(crate) fn set_max_push(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.max_push = v);
}
#[inline]
pub(crate) fn set_last_bop(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.last_bop = v);
}
#[inline]
pub(crate) fn set_doing_leaders(v: bool) {
    crate::state::with_state_mut(|s| s.shipout.doing_leaders = v);
}
#[inline]
#[allow(dead_code)]
pub(crate) fn rule_dp() -> i32 {
    crate::state::with_state(|s| s.shipout.rule_dp)
}
#[inline]
pub(crate) fn write_loc() -> i32 {
    crate::state::with_state(|s| s.shipout.write_loc)
}
#[inline]
pub(crate) fn set_write_loc(v: i32) {
    crate::state::with_state_mut(|s| s.shipout.write_loc = v);
}

// ---------------------------------------------------------------------------
// Missing setter for doing_special (raw already declared)
// ---------------------------------------------------------------------------
#[inline]
pub(crate) fn set_doing_special(v: bool) {
    crate::state::with_state_mut(|state| {
        state.shipout.doing_special = v;
    });
}

// ---------------------------------------------------------------------------
// format_ident / edit_name_start / empty / sa_null per-field setters
// ---------------------------------------------------------------------------

#[inline]
pub(crate) fn format_ident() -> i32 {
    crate::state::with_state(|state| state.strings.format_ident)
}
#[inline]
pub(crate) fn set_format_ident(v: i32) {
    crate::state::with_state_mut(|state| {
        state.strings.format_ident = v;
    });
}

#[inline]
pub(crate) fn set_edit_name_start(v: i32) {
    crate::state::with_state_mut(|state| {
        state.strings.edit_name_start = v;
    });
}
#[inline]
pub(crate) fn set_empty_b32s0(v: i32) {
    crate::state::with_state_mut(|state| {
        state.scanner.empty.s0 = v;
    });
}
#[inline]
pub(crate) fn set_empty_b32s1(v: i32) {
    crate::state::with_state_mut(|state| {
        state.scanner.empty.s1 = v;
    });
}
#[inline]
pub(crate) fn set_sa_null_b32s0(v: i32) {
    crate::state::with_state_mut(|state| {
        state.scanner.sa_null.b32.s0 = v;
    });
}
#[inline]
pub(crate) fn set_sa_null_b32s1(v: i32) {
    crate::state::with_state_mut(|state| {
        state.scanner.sa_null.b32.s1 = v;
    });
}
