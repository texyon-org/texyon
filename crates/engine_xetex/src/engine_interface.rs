//! Programmatic interface for configuring and launching the XeTeX engine.
//!
//! Translated from `xetex-engine-interface.c`.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

// The host sets these BEFORE `tt_run_engine` runs (i.e. before an owned
// `EngineState` exists), so they cannot live in the state. They are retained in
// process-global atomics; `tt_run_engine` copies them into its owned
// `st.config` at entry via `apply_config`.
static CFG_HALT_ON_ERROR_P: AtomicI32 = AtomicI32::new(0);
static CFG_IN_INITEX_MODE: AtomicBool = AtomicBool::new(false);
static CFG_SYNCTEX_ENABLED: AtomicI32 = AtomicI32::new(0);
static CFG_SEMANTIC_PAGINATION: AtomicBool = AtomicBool::new(false);
static CFG_SHELL_ESCAPE: AtomicBool = AtomicBool::new(false);
static CFG_DETERMINISTIC_MODE: AtomicBool = AtomicBool::new(false);

/// Copy the host-set config atomics into the owned engine `ConfigState`. Called
/// once by `tt_run_engine` at entry. The other two `ConfigState` fields
/// (`file_line_error_style_p`, `max_print_line`) are engine-internal defaults.
pub(crate) fn apply_config(cfg: &mut crate::state::ConfigState) {
    cfg.halt_on_error_p = CFG_HALT_ON_ERROR_P.load(Ordering::Relaxed);
    cfg.in_initex_mode = CFG_IN_INITEX_MODE.load(Ordering::Relaxed);
    cfg.synctex_enabled = CFG_SYNCTEX_ENABLED.load(Ordering::Relaxed);
    cfg.semantic_pagination_enabled = CFG_SEMANTIC_PAGINATION.load(Ordering::Relaxed);
    cfg.shell_escape_enabled = CFG_SHELL_ESCAPE.load(Ordering::Relaxed);
    cfg.deterministic_mode = CFG_DETERMINISTIC_MODE.load(Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn tt_xetex_set_int_variable(
    var_name: *const libc::c_char,
    value: libc::c_int,
) -> libc::c_int {
    // SAFETY: var_name is a valid C string for the duration of this call.
    let name = unsafe { std::ffi::CStr::from_ptr(var_name) };
    if name == c"halt_on_error_p" {
        CFG_HALT_ON_ERROR_P.store(value, Ordering::Relaxed);
    } else if name == c"in_initex_mode" {
        CFG_IN_INITEX_MODE.store(value != 0, Ordering::Relaxed);
    } else if name == c"synctex_enabled" {
        CFG_SYNCTEX_ENABLED.store(value, Ordering::Relaxed);
    } else if name == c"semantic_pagination_enabled" {
        CFG_SEMANTIC_PAGINATION.store(value != 0, Ordering::Relaxed);
    } else if name == c"shell_escape_enabled" {
        CFG_SHELL_ESCAPE.store(value != 0, Ordering::Relaxed);
    } else if name == c"deterministic_mode" {
        CFG_DETERMINISTIC_MODE.store(value != 0, Ordering::Relaxed);
    } else {
        return 1; // unrecognized variable
    }
    0 // success
}

#[no_mangle]
pub extern "C" fn tt_xetex_set_string_variable(
    _var_name: *const libc::c_char,
    _value: *const libc::c_char,
) -> libc::c_int {
    // Currently unused; see Git history for how we used to set output_comment.
    1
}

// translated from xetex-engine-interface.c:26 (tt_engine_xetex_main)
/// Batch entry point. No `setjmp` is armed: every dpx call that can `_tt_abort`
/// goes through a guarded probe in pdf_io that catches its own `longjmp`.
pub(crate) unsafe fn tt_engine_xetex_main(
    api: &mut tectonic_bridge_core::CoreBridgeState,
    dump_name: *const libc::c_char,
    input_file_name: *const libc::c_char,
    build_date: u64,
) -> libc::c_int {
    let api = api as *mut tectonic_bridge_core::CoreBridgeState as *mut libc::c_void;
    crate::bridge::enter_global_engine(api);
    let rv =
        crate::engine::tt_run_engine(api, dump_name, input_file_name, build_date as libc::time_t);
    crate::bridge::exit_global_engine();
    rv
}
