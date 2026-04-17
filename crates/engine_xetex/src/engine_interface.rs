//! Programmatic interface for configuring the XeTeX engine before launch.
//!
//! Translates `tt_xetex_set_int_variable` and `tt_xetex_set_string_variable`
//! from `xetex-engine-interface.c`.  The remaining two functions in that file
//! (`tt_engine_xetex_main` and the aarch64 linker hack) use `setjmp`/`longjmp`
//! and must stay in C.

use crate::globals::{
    set_halt_on_error_p, set_in_initex_mode, set_semantic_pagination_enabled,
    set_shell_escape_enabled, set_synctex_enabled,
};

#[no_mangle]
pub unsafe extern "C" fn tt_xetex_set_int_variable(
    var_name: *const libc::c_char,
    value: libc::c_int,
) -> libc::c_int {
    // SAFETY: var_name is a valid C string for the duration of this call.
    let name = unsafe { std::ffi::CStr::from_ptr(var_name) };
    if name == c"halt_on_error_p" {
        set_halt_on_error_p(value);
    } else if name == c"in_initex_mode" {
        set_in_initex_mode(value != 0);
    } else if name == c"synctex_enabled" {
        set_synctex_enabled(value);
    } else if name == c"semantic_pagination_enabled" {
        set_semantic_pagination_enabled(value != 0);
    } else if name == c"shell_escape_enabled" {
        set_shell_escape_enabled(value != 0);
    } else {
        return 1; // unrecognized variable
    }
    0 // success
}

#[no_mangle]
pub unsafe extern "C" fn tt_xetex_set_string_variable(
    _var_name: *const libc::c_char,
    _value: *const libc::c_char,
) -> libc::c_int {
    // Currently unused; see Git history for how we used to set output_comment.
    1
}
