//! Programmatic interface for launching the XeTeX engine.

use crate::state::ConfigState;

/// Batch entry point. Every dpx call that can `_tt_abort` goes through a
/// guarded probe in pdf_io that catches its own `longjmp`.
pub(crate) unsafe fn tt_engine_xetex_main(
    api: &mut tectonic_bridge_core::CoreBridgeState,
    config: ConfigState,
    dump_name: &str,
    input_file_name: &str,
    build_date: u64,
) -> libc::c_int {
    let api = api as *mut tectonic_bridge_core::CoreBridgeState as *mut libc::c_void;
    crate::bridge::enter_global_engine(api);
    let rv = crate::engine::tt_run_engine(
        api,
        config,
        dump_name,
        input_file_name,
        build_date as libc::time_t,
    );
    crate::bridge::exit_global_engine();
    rv
}
