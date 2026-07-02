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
        // input_ptr/max_in_stack/base_ptr/par_loc/par_token + capacity
        // stack_size moved to EngineState.input (scalars) — see `01e`.

        // Macro parameter stack (xetex-xetexd.h:517–519). The `param_stack`
        // array is Rust-owned on `EngineState.input.param_stack`; the scalars
        // param_ptr/max_param_stack are now on EngineState.input too.
        // Allocated size of param_stack stays a C global.

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
        // _xeq_level_array (XEQ_LEVEL; EQTB_SIZE - INT_BASE + 1 = 1114730 entries)
        // moved to EngineState.eqtb.xeq_level (session 49).

        // Expression-depth guard (xetex-xetexd.h:415, 525).
        // cur_val_level moved to EngineState.scanner.

        // scanner_status / warning_index / def_ref / long_state moved to
        // EngineState.scanner (scalars).

        // Input buffer: first = next free slot; buf_size = capacity
        // last/max_buf_stack moved to EngineState.input (scalars).
        // (xetex-xetexd.h:393–395, 409).
        pub(super) static mut first: i32;
        // Maximum allowed input-nesting depth (xetex-xetexd.h:411).

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

// ---------------------------------------------------------------------------
// randoms / j_random
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// mem arena word accessors
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// lo_mem_max / hi_mem_min / avail / mem_end / rover
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// mem word-level accessors (b16 and gr fields)
// ---------------------------------------------------------------------------

// Layout (little-endian): one MemoryWord = 8 bytes.
//   b32.s0 == bytes [0,4): b16.s0 (bytes 0-1) | b16.s1 (bytes 2-3)
//   b32.s1 == bytes [4,8): b16.s2 (bytes 4-5) | b16.s3 (bytes 6-7)
//   gr           == all 8 bytes as f64

// ---------------------------------------------------------------------------
// eqtb accessor
// ---------------------------------------------------------------------------

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
/// Returns `name_length`.
#[inline]
pub(crate) fn name_length() -> i32 {
    crate::state::with_state(|s| s.input.name_length)
}

// ---------------------------------------------------------------------------
// in_open / full_source_filename_stack / line_stack / file_line_error_style_p
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// hash / prim / eqtb_top
// ---------------------------------------------------------------------------

/// Read `hash[p].s1` (the "text" half of a hash-table entry).
#[inline]
pub(crate) fn hash_s1(p: i32) -> i32 {
    crate::state::with_state(|s| s.eqtb.hash[(p as usize) - 514].s1)
}

// ---------------------------------------------------------------------------
// hash management globals (hash_used / hash_extra / hash_high /
//                          no_new_control_sequence / prim_used)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// rust_stdout / log_file / write_file / current_diagnostic
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// print_raw_char globals
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// str_start write / str_ptr write / string pool limits
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// help_ptr / help_line
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// two_to_the / spec_log (read-only tables from xetex-ini.c)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// interaction / history / error_count / use_err_help / halt_on_error_p
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// engine launch config flags (set by tt_xetex_set_int_variable)
// ---------------------------------------------------------------------------

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
// ---------------------------------------------------------------------------
// name_in_progress / stop_at_space
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// save_stack / save_ptr / cur_level / cur_group
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// read_open / read_file (for \read stream management)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// font_name
// ---------------------------------------------------------------------------

/// Returns `param_base[f]`.

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

/// C++ bridge for OpenType math helpers that still need font_area. Reads the
/// borrow-free mirror so it is safe to call under a held `EngineState` borrow
/// (see `font_ffi_mirror`).
#[no_mangle]
pub extern "C" fn tt_get_font_area_entry(f: i32) -> i32 {
    // SAFETY: mirror kept in sync with fonts.font_area by refresh_font_ffi_mirrors.
    unsafe { *font_ffi_mirror::FONT_AREA.add(f as usize) }
}
/// C++ bridge for the remaining OpenType math helper. Reads the borrow-free
/// mirror (see `font_ffi_mirror`).
#[no_mangle]
pub extern "C" fn tt_get_font_size_entry(f: i32) -> i32 {
    // SAFETY: mirror kept in sync with fonts.font_size by refresh_font_ffi_mirrors.
    unsafe { *font_ffi_mirror::FONT_SIZE.add(f as usize) }
}
/// C++ bridge: returns `font_layout_engine[f]` for use in OpenType math helpers.
/// Reads the borrow-free mirror (see `font_ffi_mirror`).
#[no_mangle]
pub extern "C" fn tt_get_font_layout_engine_entry(f: i32) -> *mut libc::c_void {
    // SAFETY: mirror kept in sync with fonts.font_layout_engine by refresh_font_ffi_mirrors.
    unsafe { *font_ffi_mirror::FONT_LAYOUT_ENGINE.add(f as usize) }
}

// ---------------------------------------------------------------------------
// epochseconds / microseconds
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// mag_set
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// semantic nest (nest / nest_ptr / max_nest_stack / nest_size / cur_list)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// input_stack / input_ptr / base_ptr
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// grp_stack / if_stack
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// cond_ptr / if_limit / cur_if / if_line
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// sa_root / sa_chain / sa_level
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// max_reg_num / max_reg_help_line
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// param_stack / param_ptr / max_param_stack
// ---------------------------------------------------------------------------


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

// ---------------------------------------------------------------------------
// set_in_open
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// source_filename_stack / full_source_filename_stack / line_stack
// ---------------------------------------------------------------------------

/// Sets `line`.
#[inline]
pub(crate) fn set_line(v: i32) {
    unsafe {
        raw::line = v;
    }
}

// ---------------------------------------------------------------------------
// max_buf_stack / set_buffer_entry / cur_input_limit
// ---------------------------------------------------------------------------

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


// ---------------------------------------------------------------------------
// scanner_status / warning_index / def_ref
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// gave_char_warning_help / main_p / main_k
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// cur_order
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// alignment state
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// hpack adjust-tail / total_stretch / total_shrink
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// format_ident / edit_name_start / empty / sa_null per-field setters
// ---------------------------------------------------------------------------
