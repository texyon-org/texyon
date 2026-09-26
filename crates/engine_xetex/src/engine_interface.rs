//! Programmatic interface for configuring and launching the XeTeX engine.

use crate::state::ConfigState;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

// The host sets these before an `EngineState` exists, so they live here until
// `apply_config` copies them in.
static CFG_HALT_ON_ERROR_P: AtomicI32 = AtomicI32::new(0);
static CFG_IN_INITEX_MODE: AtomicBool = AtomicBool::new(false);
static CFG_SYNCTEX_ENABLED: AtomicI32 = AtomicI32::new(0);
static CFG_SEMANTIC_PAGINATION: AtomicBool = AtomicBool::new(false);
static CFG_SHELL_ESCAPE: AtomicBool = AtomicBool::new(false);
static CFG_DETERMINISTIC_MODE: AtomicBool = AtomicBool::new(false);

impl ConfigState {
    pub(crate) fn apply_config(&mut self) {
        self.halt_on_error_p = CFG_HALT_ON_ERROR_P.load(Ordering::Relaxed);
        self.in_initex_mode = CFG_IN_INITEX_MODE.load(Ordering::Relaxed);
        self.synctex_enabled = CFG_SYNCTEX_ENABLED.load(Ordering::Relaxed);
        self.semantic_pagination_enabled = CFG_SEMANTIC_PAGINATION.load(Ordering::Relaxed);
        self.shell_escape_enabled = CFG_SHELL_ESCAPE.load(Ordering::Relaxed);
        self.deterministic_mode = CFG_DETERMINISTIC_MODE.load(Ordering::Relaxed);
    }
}

#[no_mangle]
pub extern "C" fn tt_xetex_set_int_variable(
    var_name: *const libc::c_char,
    value: libc::c_int,
) -> libc::c_int {
    // SAFETY: var_name is a valid C string for the duration of this call.
    let name = unsafe { std::ffi::CStr::from_ptr(var_name) };
    match name.to_bytes() {
        b"halt_on_error_p" => CFG_HALT_ON_ERROR_P.store(value, Ordering::Relaxed),
        b"in_initex_mode" => CFG_IN_INITEX_MODE.store(value != 0, Ordering::Relaxed),
        b"synctex_enabled" => CFG_SYNCTEX_ENABLED.store(value, Ordering::Relaxed),
        b"semantic_pagination_enabled" => {
            CFG_SEMANTIC_PAGINATION.store(value != 0, Ordering::Relaxed)
        }
        b"shell_escape_enabled" => CFG_SHELL_ESCAPE.store(value != 0, Ordering::Relaxed),
        b"deterministic_mode" => CFG_DETERMINISTIC_MODE.store(value != 0, Ordering::Relaxed),
        _ => return 1,
    }
    0
}

/// Batch entry point. Every dpx call that can `_tt_abort` goes through a
/// guarded probe in pdf_io that catches its own `longjmp`.
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
