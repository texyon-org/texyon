//! Error-handling functions translated from `xetex/xetex-errors.c`.

use crate::globals::{
    error_count, halt_on_error_p, help_line, history, interaction, job_name, log_opened,
    rust_stdout, selector, set_error_count, set_help_line, set_help_ptr, set_history,
    set_interaction, set_selector, use_err_help,
};

// Interaction-mode constants (xetex_format.h:644–647).
const BATCH_MODE: u8 = 0;
const SCROLL_MODE: u8 = 2;
const ERROR_STOP_MODE: u8 = 3;

// History constants (xetex-xetexd.h:355–358).
const HISTORY_ERROR_ISSUED: i32 = 2;
const HISTORY_FATAL_ERROR: i32 = 3;

// Selector constants (xetex-core.h:46–48).
const SELECTOR_TERM_ONLY: i32 = 17;
const SELECTOR_TERM_AND_LOG: i32 = 19;

extern "C" {
    fn error_here_with_diagnostic(msg: *const libc::c_char) -> *mut std::ffi::c_void;
    fn capture_to_diagnostic(diag: *mut std::ffi::c_void);
    fn print_cmd_chr(cmd: u16, chr_code: i32);
    fn print(s: i32);
    fn print_char(c: i32);
    fn print_cstr(s: *const libc::c_char);
    fn print_nl_cstr(s: *const libc::c_char);
    fn print_int(n: i32);
    fn print_ln();
    fn show_context();
    fn open_log_file();
    fn close_files_and_terminate();
    fn tt_cleanup();
    fn ttstub_output_flush(handle: usize) -> libc::c_int;
    fn _tt_abort(fmt: *const libc::c_char) -> !;
}

// ---------------------------------------------------------------------------
// pre_error_message  (was static in C)
// ---------------------------------------------------------------------------

unsafe fn pre_error_message() {
    if log_opened() {
        set_selector(SELECTOR_TERM_AND_LOG);
    } else {
        set_selector(SELECTOR_TERM_ONLY);
    }
    if job_name() == 0 {
        open_log_file();
    }
    if interaction() == BATCH_MODE {
        set_selector(selector() - 1);
    }
    error_here_with_diagnostic(c"".as_ptr());
}

// ---------------------------------------------------------------------------
// post_error_message  (was static in C)
// ---------------------------------------------------------------------------

unsafe fn post_error_message(need_to_print_it: bool) {
    capture_to_diagnostic(std::ptr::null_mut());
    if interaction() == ERROR_STOP_MODE {
        set_interaction(SCROLL_MODE);
    }
    if need_to_print_it && log_opened() {
        error();
    }
    set_history(HISTORY_FATAL_ERROR);
    close_files_and_terminate();
    tt_cleanup();
    ttstub_output_flush(rust_stdout());
}

// ---------------------------------------------------------------------------
// error
// ---------------------------------------------------------------------------

/// Handle a recoverable TeX error.
///
/// translated from xetex-errors.c:52 (error)
#[no_mangle]
pub unsafe extern "C" fn error() {
    if history() < HISTORY_ERROR_ISSUED {
        set_history(HISTORY_ERROR_ISSUED);
    }
    print_char(b'.' as i32);
    show_context();
    if halt_on_error_p() {
        set_history(HISTORY_FATAL_ERROR);
        post_error_message(false);
        _tt_abort(c"halted on potentially-recoverable error as specified".as_ptr());
    }

    set_error_count(error_count().wrapping_add(1));
    if error_count() == 100 {
        print_nl_cstr(c"(That makes 100 errors; please try again.)".as_ptr());
        set_history(HISTORY_FATAL_ERROR);
        post_error_message(false);
        _tt_abort(c"halted after 100 potentially-recoverable errors".as_ptr());
    }

    if interaction() > BATCH_MODE {
        set_selector(selector() - 1);
    }
    if use_err_help() {
        print_ln();
        give_err_help();
    } else {
        let mut hp = crate::globals::help_ptr();
        while hp > 0 {
            hp -= 1;
            print_nl_cstr(help_line(hp as usize));
        }
        crate::globals::set_help_ptr(hp);
    }
    print_ln();
    if interaction() > BATCH_MODE {
        set_selector(selector() + 1);
    }
    print_ln();
}

// ---------------------------------------------------------------------------
// fatal_error
// ---------------------------------------------------------------------------

/// Print `s` as a fatal "Emergency stop" message and abort.
///
/// translated from xetex-errors.c:98 (fatal_error)
#[no_mangle]
pub unsafe extern "C" fn fatal_error(s: *const libc::c_char) {
    pre_error_message();
    print_cstr(c"Emergency stop".as_ptr());
    print_nl_cstr(s);
    capture_to_diagnostic(std::ptr::null_mut());
    close_files_and_terminate();
    tt_cleanup();
    ttstub_output_flush(rust_stdout());
    _tt_abort(s);
}

// ---------------------------------------------------------------------------
// overflow
// ---------------------------------------------------------------------------

/// Report a TeX capacity overflow for resource `s` of size `n` and abort.
///
/// translated from xetex-errors.c:113 (overflow)
#[no_mangle]
pub unsafe extern "C" fn overflow(s: *const libc::c_char, n: i32) -> ! {
    pre_error_message();
    print_cstr(c"TeX capacity exceeded, sorry [".as_ptr());
    print_cstr(s);
    print_char(b'=' as i32);
    print_int(n);
    print_char(b']' as i32);
    set_help_ptr(2);
    set_help_line(1, c"If you really absolutely need more capacity,".as_ptr());
    set_help_line(0, c"you can ask a wizard to enlarge me.".as_ptr());
    post_error_message(true);
    _tt_abort(c"halted on overflow()".as_ptr())
}

// ---------------------------------------------------------------------------
// confusion
// ---------------------------------------------------------------------------

/// Report an internal consistency failure and abort.
///
/// translated from xetex-errors.c:132 (confusion)
#[no_mangle]
pub unsafe extern "C" fn confusion(s: *const libc::c_char) -> ! {
    pre_error_message();
    if history() < HISTORY_ERROR_ISSUED {
        print_cstr(c"This can't happen (".as_ptr());
        print_cstr(s);
        print_char(b')' as i32);
        set_help_ptr(1);
        set_help_line(0, c"I'm broken. Please show this to someone who can fix can fix".as_ptr());
    } else {
        print_cstr(c"I can't go on meeting you like this".as_ptr());
        set_help_ptr(2);
        set_help_line(1, c"One of your faux pas seems to have wounded me deeply...".as_ptr());
        set_help_line(0, c"in fact, I'm barely conscious. Please fix it and try again.".as_ptr());
    }
    post_error_message(true);
    _tt_abort(c"halted on confusion()".as_ptr())
}

// ---------------------------------------------------------------------------
// pdf_error
// ---------------------------------------------------------------------------

/// Report an XDV/PDF-level error and abort.
// ---------------------------------------------------------------------------
// int_error
// ---------------------------------------------------------------------------

/// Append "(N)" to the current error message and call error().
///
/// translated from xetex-xetex0.c:15 (int_error, was static)
#[no_mangle]
pub unsafe extern "C" fn int_error(n: i32) {
    print_cstr(c" (".as_ptr());
    print_int(n);
    print_char(b')' as i32);
    error();
}

// ---------------------------------------------------------------------------
// not_*_font_error cluster
// ---------------------------------------------------------------------------

unsafe fn not_font_error_common(cmd: u16, c: i32, f: i32, suffix: *const libc::c_char) {
    error_here_with_diagnostic(c"Cannot use ".as_ptr());
    print_cmd_chr(cmd, c);
    print_cstr(c" with ".as_ptr());
    print(crate::globals::font_name_entry(f));
    print_cstr(suffix);
    capture_to_diagnostic(std::ptr::null_mut());
    error();
}

/// translated from xetex-xetex0.c:2727 (not_aat_font_error)
#[no_mangle]
pub unsafe extern "C" fn not_aat_font_error(cmd: i32, c: i32, f: i32) {
    not_font_error_common(cmd as u16, c, f, c"; not an AAT font".as_ptr());
}

/// translated from xetex-xetex0.c:2739 (not_aat_gr_font_error)
#[no_mangle]
pub unsafe extern "C" fn not_aat_gr_font_error(cmd: i32, c: i32, f: i32) {
    not_font_error_common(cmd as u16, c, f, c"; not an AAT or Graphite font".as_ptr());
}

/// translated from xetex-xetex0.c:2751 (not_ot_font_error)
#[no_mangle]
pub unsafe extern "C" fn not_ot_font_error(cmd: i32, c: i32, f: i32) {
    not_font_error_common(cmd as u16, c, f, c"; not an OpenType Layout font".as_ptr());
}

/// translated from xetex-xetex0.c:2763 (not_native_font_error)
#[no_mangle]
pub unsafe extern "C" fn not_native_font_error(cmd: i32, c: i32, f: i32) {
    not_font_error_common(cmd as u16, c, f, c"; not a native platform font".as_ptr());
}

///
/// translated from xetex-errors.c:157 (pdf_error)
#[no_mangle]
pub unsafe extern "C" fn pdf_error(t: *const libc::c_char, p: *const libc::c_char) -> ! {
    pre_error_message();
    print_cstr(c"Error".as_ptr());
    if !t.is_null() {
        print_cstr(c" (".as_ptr());
        print_cstr(t);
        print_char(b')' as i32);
    }
    print_cstr(c": ".as_ptr());
    print_cstr(p);
    post_error_message(true);
    _tt_abort(c"halted on pdf_error()".as_ptr())
}

// translated from xetex-xetex0.c:13499 (give_err_help)
#[no_mangle]
pub unsafe extern "C" fn give_err_help() {
    // LOCAL(err_help) = eqtb[LOCAL_BASE + LOCAL__err_help].b32.s1
    // LOCAL_BASE = 2_254_871, LOCAL__err_help = 9
    crate::output::token_show(crate::globals::eqtb_b32s1(2_254_871 + 9));
}
