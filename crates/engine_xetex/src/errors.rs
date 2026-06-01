//! Error-handling functions translated from `xetex/xetex-errors.c`.

use crate::consts::{
    BATCH_MODE, BOX_BASE, ERROR_STOP_MODE, SCROLL_MODE, SELECTOR_TERM_AND_LOG, SELECTOR_TERM_ONLY,
};
use crate::globals::{
    align_state, cur_chr, cur_cmd, cur_list_mode, eqtb_b32s1, error_count, halt_on_error_p,
    help_line, history, interaction, job_name, log_opened, rust_stdout, selector, set_align_state,
    set_error_count, set_help_line, set_help_ptr, set_history, set_interaction, set_selector,
    use_err_help,
};
use crate::output::print_cmd_chr;
use crate::output::show_box;
use crate::packaging::flush_node_list_global;

// Interaction-mode constants (xetex_format.h:644–647).

// History constants (xetex-xetexd.h:355–358).
use crate::engine::tt_cleanup;
use crate::io::close_files_and_terminate;
use crate::io::open_log_file;
use crate::output::capture_to_diagnostic;
use crate::output::error_here_with_diagnostic;
use crate::output::print;
use crate::output::print_char;
use crate::output::print_cstr;
use crate::output::print_int;
use crate::output::print_ln;
use crate::output::print_nl_cstr;
use crate::output::show_context;

const HISTORY_ERROR_ISSUED: i32 = 2;
const HISTORY_FATAL_ERROR: i32 = 3;

// Group codes matched by `extra_right_brace` (relocated from scan.rs, ws02).
const SEMI_SIMPLE_GROUP: u8 = 14;
const MATH_SHIFT_GROUP: u8 = 15;
const MATH_LEFT_GROUP: u8 = 16;

// Selector constants (xetex-core.h:46–48).

extern "C" {

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
pub unsafe fn error() {
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
pub unsafe fn overflow(s: *const libc::c_char, n: i32) -> ! {
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
pub unsafe fn confusion(s: *const libc::c_char) -> ! {
    pre_error_message();
    if history() < HISTORY_ERROR_ISSUED {
        print_cstr(c"This can't happen (".as_ptr());
        print_cstr(s);
        print_char(b')' as i32);
        set_help_ptr(1);
        set_help_line(
            0,
            c"I'm broken. Please show this to someone who can fix can fix".as_ptr(),
        );
    } else {
        print_cstr(c"I can't go on meeting you like this".as_ptr());
        set_help_ptr(2);
        set_help_line(
            1,
            c"One of your faux pas seems to have wounded me deeply...".as_ptr(),
        );
        set_help_line(
            0,
            c"in fact, I'm barely conscious. Please fix it and try again.".as_ptr(),
        );
    }
    post_error_message(true);
    _tt_abort(c"halted on confusion()".as_ptr())
}

// ---------------------------------------------------------------------------
// int_error
// ---------------------------------------------------------------------------

/// Append "(N)" to the current error message and call error().
///
/// translated from xetex-xetex0.c:15 (int_error, was static)
pub unsafe fn int_error(n: i32) {
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
pub unsafe fn not_aat_font_error(cmd: i32, c: i32, f: i32) {
    not_font_error_common(cmd as u16, c, f, c"; not an AAT font".as_ptr());
}

/// translated from xetex-xetex0.c:2739 (not_aat_gr_font_error)
pub unsafe fn not_aat_gr_font_error(cmd: i32, c: i32, f: i32) {
    not_font_error_common(cmd as u16, c, f, c"; not an AAT or Graphite font".as_ptr());
}

/// translated from xetex-xetex0.c:2751 (not_ot_font_error)
pub unsafe fn not_ot_font_error(cmd: i32, c: i32, f: i32) {
    not_font_error_common(cmd as u16, c, f, c"; not an OpenType Layout font".as_ptr());
}

/// translated from xetex-xetex0.c:2763 (not_native_font_error)
pub unsafe fn not_native_font_error(cmd: i32, c: i32, f: i32) {
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

// ---- moved verbatim from scan.rs (workstream 02) ----

// translated from xetex-xetex0.c:5050 (mu_error)
#[no_mangle]
pub unsafe extern "C" fn mu_error() {
    error_here_with_diagnostic(c"Incompatible glue units".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(1);
    set_help_line(
        0,
        c"I'm going to assume that 1mu=1pt when they're mixed.".as_ptr(),
    );
    error();
}
/// translated from xetex-xetex0.c:7411 (extra_right_brace)
#[no_mangle]
pub unsafe extern "C" fn extra_right_brace() {
    use crate::globals::cur_group;

    error_here_with_diagnostic(c"Extra }, or forgotten ".as_ptr());
    match cur_group() {
        SEMI_SIMPLE_GROUP => crate::output::print_esc_cstr(c"endgroup".as_ptr()),
        MATH_SHIFT_GROUP => crate::output::print_char(b'$' as i32),
        MATH_LEFT_GROUP => crate::output::print_esc_cstr(c"right".as_ptr()),
        _ => {}
    }
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(5);
    set_help_line(
        4,
        c"I've deleted a group-closing symbol because it seems to be".as_ptr(),
    );
    set_help_line(
        3,
        c"spurious, as in `$x}$'. But perhaps the } is legitimate and".as_ptr(),
    );
    set_help_line(
        2,
        c"you forgot something else, as in `\\hbox{$x}'. In such cases".as_ptr(),
    );
    set_help_line(
        1,
        c"the way to recover is to insert both the forgotten and the".as_ptr(),
    );
    set_help_line(0, c"deleted material, e.g., by typing `I$}'.".as_ptr());
    error();
    set_align_state(align_state() + 1);
}
// translated from xetex-xetex0.c:9326 (you_cant)
#[no_mangle]
pub unsafe extern "C" fn you_cant() {
    error_here_with_diagnostic(c"You can't use `".as_ptr());
    print_cmd_chr(cur_cmd() as u16, cur_chr());
    crate::output::print_in_mode(cur_list_mode() as i32);
    capture_to_diagnostic(std::ptr::null_mut());
}
// translated from xetex-xetex0.c:9334 (report_illegal_case)
#[no_mangle]
pub unsafe extern "C" fn report_illegal_case() {
    you_cant();
    set_help_ptr(4);
    set_help_line(
        3,
        c"Sorry, but I'm not programmed to handle this case;".as_ptr(),
    );
    set_help_line(2, c"I'll just pretend that you didn't ask for it.".as_ptr());
    set_help_line(
        1,
        c"If you're in the wrong mode, you might be able to".as_ptr(),
    );
    set_help_line(
        0,
        c"return to the right one by typing `I}' or `I$' or `I\\par'.".as_ptr(),
    );
    error();
}
// translated from xetex-xetex0.c:9347 (privileged)
#[no_mangle]
pub unsafe extern "C" fn privileged() -> bool {
    if cur_list_mode() > 0 {
        true
    } else {
        report_illegal_case();
        false
    }
}
// translated from xetex-xetex0.c:10489 (cs_error)
#[no_mangle]
pub unsafe extern "C" fn cs_error() {
    error_here_with_diagnostic(c"Extra ".as_ptr());
    crate::output::print_esc_cstr(c"endcsname".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(1);
    set_help_line(
        0,
        c"I'm ignoring this, since I wasn't doing a \\csname.".as_ptr(),
    );
    error();
}
// translated from xetex-xetex0.c:9270 (box_error)
pub unsafe fn box_error(n: u8) {
    let box_val = eqtb_b32s1(BOX_BASE + n as i32);
    error();
    crate::output::begin_diagnostic();
    crate::output::diagnostic_begin_capture_warning_here();
    crate::output::print_nl_cstr(c"The following box has been deleted:".as_ptr());
    show_box(box_val);
    capture_to_diagnostic(std::ptr::null_mut());
    crate::output::end_diagnostic(true);
    flush_node_list_global(box_val);
    // BOX_REG(n) = TEX_NULL — direct eqtb write, no save-stack involvement
    crate::globals::set_eqtb_b32s1(BOX_BASE + n as i32, crate::mem::TEX_NULL);
}
// translated from xetex-xetex0.c:107 (runaway)
pub unsafe fn runaway() {
    use crate::globals::{def_ref, error_line, mem_b32s1, scanner_status};
    use crate::mem::TEX_NULL;

    const SKIPPING: u8 = 1;
    const DEFINING: u8 = 2;
    const MATCHING: u8 = 3;
    const ALIGNING: u8 = 4;
    const ABSORBING: u8 = 5;
    const TEMP_HEAD: i32 = 4_999_996; // MEM_TOP - 3
    const HOLD_HEAD: i32 = 4_999_995; // MEM_TOP - 4

    if scanner_status() > SKIPPING {
        let p = match scanner_status() {
            DEFINING => {
                crate::output::print_nl_cstr(c"Runaway definition".as_ptr());
                def_ref()
            }
            MATCHING => {
                crate::output::print_nl_cstr(c"Runaway argument".as_ptr());
                TEMP_HEAD
            }
            ALIGNING => {
                crate::output::print_nl_cstr(c"Runaway preamble".as_ptr());
                HOLD_HEAD
            }
            ABSORBING => {
                crate::output::print_nl_cstr(c"Runaway text".as_ptr());
                def_ref()
            }
            _ => return,
        };
        crate::output::print_char('?' as i32);
        crate::output::print_ln();
        crate::output::show_token_list(mem_b32s1(p), TEX_NULL, error_line() - 10);
    }
}
