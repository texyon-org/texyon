//! Centralized bindings to XeTeX's C globals.
//!
//! Translated Rust functions read and write the engine's global state
//! through this module rather than declaring `extern "C"` statics at
//! their call sites. This keeps the `unsafe` blob small and ensures we
//! audit aliasing in one place.
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
#[derive(Copy, Clone, Default)]
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

        // xetex-xetexd.h:459 — temporary pointer register.

        // Output file handles (xetex-xetexd.h:429–430, 691).
        // `rust_output_handle_t` is `uintptr_t` (tectonic_bridge_core_generated.h:9).

        // Current diagnostic capture handle (xetex-output.c).
        // Null (0) means no active capture. Type is opaque `ttbc_diagnostic_t *`.

        // Name-of-file globals (xetex-xetexd.h:388–391).

        // Font memory tables (xetex-xetexd.h:555–590).
        // Current control-sequence index (xetex-xetexd.h:501; 0 = not a cs).
        // Current token (xetex-xetexd.h:502; token encoded as int32).
        // Current character code (xetex-xetexd.h:500).
        // Current command code (`eight_bits` = u8, xetex-xetexd.h:499).
        // Current parsed integer value (xetex-xetexd.h:530).

        // Macro parameter stack (xetex-xetexd.h:517–519).

        // SA trie cursor — scratch pointer used during SA node traversal/allocation
        // (xetex-xetexd.h:710; xetex-ini.c:336).

        // Semantic nest (mode stack, xetex-xetexd.h:476–479,413).
        // Current list state (xetex-xetexd.h:479; direct struct, not a pointer).

        // Save-stack (xetex-xetexd.h:492–494).

        // Expression-depth guard (xetex-xetexd.h:415, 525).

        // Input buffer: first = next free slot; buf_size = capacity
        // (xetex-xetexd.h:393–395, 409).
        // Maximum allowed input-nesting depth (xetex-xetexd.h:411).

        // main_control scratch glue-spec and font-info index (xetex-xetexd.h:675–676).
        // main_control inner-loop state (xetex-xetexd.h:672–686).
        // Ligature/kern inner-loop state (xetex-xetexd.h:630–634, 683–686).
        // Further main_loop state (xetex-xetexd.h:679–686).
        // XeTeX inter-character class state (xetex-xetexd.h:679-681).
        // Hyphenation trie and exception tables (xetex-xetexd.h:635-645).
    }
}

// ---------------------------------------------------------------------------
// arith_error
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// tex_remainder
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// in_open / full_source_filename_stack / line_stack / file_line_error_style_p
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// hash / prim / eqtb_top
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// hash management globals (hash_used / hash_extra / hash_high /
//                          no_new_control_sequence / prim_used)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// temp_ptr
// ---------------------------------------------------------------------------

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

/// Raw borrow-free mirrors of the three `FontState` arrays the OpenType-math
/// C++ FFI (`xetex-XeTeXOTMath.cpp`) reads back via the `tt_get_font_*_entry`
/// callbacks below. Those callbacks run *inside* the C++ OT-math helpers.
/// A callback that took the `ENGINE_STATE` borrow would nest-panic
/// (`RefCell already borrowed`), so it must read through these raw pointers
/// instead.
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
        font_ffi_mirror::FONT_LAYOUT_ENGINE = fonts.font_layout_engine_mirror.as_ptr();
    }
}

/// C++ bridge for OpenType math helpers that still need font_area. Reads the
/// borrow-free mirror (see `font_ffi_mirror`).
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

// ---------------------------------------------------------------------------
// font_max
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// cur_cmd / cur_mark
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// set_in_open
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// source_filename_stack / full_source_filename_stack / line_stack
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// max_buf_stack / set_buffer_entry / cur_input_limit
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// expand_depth / expand_depth_count / cur_val_level
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// scanner_status / warning_index / def_ref
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// gave_char_warning_help / main_p / main_k
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// main_loop inner-loop globals
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// active_width / best_height_plus_depth
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// lr_ptr / lr_problems / cur_dir
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// format_ident / edit_name_start / empty / sa_null per-field setters
// ---------------------------------------------------------------------------
