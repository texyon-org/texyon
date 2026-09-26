use std::ffi::CStr;

use crate::bridge::ttstub_output_flush;
use crate::consts::{
    HISTORY_ERROR_ISSUED, HISTORY_FATAL_ERROR, LOCAL_ERR_HELP, MEM_TOP, SELECTOR_TERM_AND_LOG,
    SELECTOR_TERM_ONLY, TEX_NULL,
};
use crate::mem::{HOLD_HEAD, TEMP_HEAD};
use crate::output::{
    begin_diagnostic, capture_to_diagnostic, diagnostic_begin_capture_warning_here, end_diagnostic,
    error_here_with_diagnostic, print, print_char, print_cmd_chr, print_cstr, print_err,
    print_esc_cstr, print_in_mode, print_int, print_ln, print_nl_cstr, print_nl_str, show_box,
    show_context, show_token_list, token_show,
};
use crate::scan::{ABSORBING, ALIGNING, DEFINING, MATCHING};
use crate::state::{EngineState, ErrorState, GroupCode, InteractionMode};

extern "C" {
    fn _ttbc_set_error_message(message: *const libc::c_char);
}

impl ErrorState {
    /// Sets the help text shown after the next error, in display order.
    pub(crate) fn help(&mut self, lines: &[&'static str]) {
        self.help_lines[..lines.len()].copy_from_slice(lines);
        self.help_len = lines.len() as u8;
    }
}

unsafe fn pre_error_message(st: &mut EngineState) {
    st.shipout.selector = if st.errors.log_opened {
        SELECTOR_TERM_AND_LOG
    } else {
        SELECTOR_TERM_ONLY
    };
    if st.strings.job_name == 0 {
        // Already on the fatal path, so a log-open failure just leaves TERM_ONLY.
        let _ = crate::io::open_log_file(st);
    }
    if st.errors.interaction == InteractionMode::Batch {
        st.shipout.selector -= 1;
    }
    error_here_with_diagnostic(st, c"".as_ptr());
}

unsafe fn post_error_message(st: &mut EngineState, need_to_print_it: bool) {
    capture_to_diagnostic(st, std::ptr::null_mut());
    if st.errors.interaction == InteractionMode::ErrorStop {
        st.errors.interaction = InteractionMode::Scroll;
    }
    if need_to_print_it && st.errors.log_opened {
        // Already committed to a fatal history, so a nested abort is dropped.
        let _ = error(st);
    }
    st.errors.history = HISTORY_FATAL_ERROR;
    let _ = crate::io::close_files_and_terminate(st);
    crate::engine::tt_cleanup(st);
    ttstub_output_flush(st.shipout.rust_stdout);
}

#[must_use]
pub(crate) enum ErrorOutcome {
    Continue,
    Halt,
    Hundred,
    Confusion(&'static CStr),
    Overflow { what: &'static CStr, n: i32 },
}

impl ErrorOutcome {
    pub(crate) fn into_engine_abort(self) -> EngineAbort {
        match self {
            ErrorOutcome::Continue => {
                unreachable!("Continue is never propagated as a terminal abort")
            }
            ErrorOutcome::Halt => EngineAbort::HaltMessage(
                c"halted on potentially-recoverable error as specified".as_ptr(),
            ),
            ErrorOutcome::Hundred => EngineAbort::HaltMessage(
                c"halted after 100 potentially-recoverable errors".as_ptr(),
            ),
            ErrorOutcome::Confusion(msg) => EngineAbort::Confusion(msg.as_ptr()),
            ErrorOutcome::Overflow { what, n } => EngineAbort::Overflow {
                what: what.as_ptr(),
                n,
            },
        }
    }
}

pub(crate) fn error(st: &mut EngineState) -> ErrorOutcome {
    if st.errors.history < HISTORY_ERROR_ISSUED {
        st.errors.history = HISTORY_ERROR_ISSUED;
    }
    print_char(st, b'.' as i32);
    show_context(st);
    if st.config.halt_on_error_p != 0 {
        st.errors.history = HISTORY_FATAL_ERROR;
        return ErrorOutcome::Halt;
    }

    st.errors.error_count = st.errors.error_count.wrapping_add(1);
    if st.errors.error_count == 100 {
        print_nl_cstr(st, c"(That makes 100 errors; please try again.)".as_ptr());
        st.errors.history = HISTORY_FATAL_ERROR;
        return ErrorOutcome::Hundred;
    }

    if st.errors.interaction > InteractionMode::Batch {
        st.shipout.selector -= 1;
    }
    if st.errors.use_err_help {
        print_ln(st);
        give_err_help(st);
    } else {
        let lines = st.errors.help_lines;
        let len = st.errors.help_len as usize;
        st.errors.help_len = 0;
        for line in &lines[..len] {
            print_nl_str(st, line);
        }
    }
    print_ln(st);
    if st.errors.interaction > InteractionMode::Batch {
        st.shipout.selector += 1;
    }
    print_ln(st);
    ErrorOutcome::Continue
}

/// A terminal abort, reported by [`EngineAbort::into_history`].
pub(crate) enum EngineAbort {
    Cancelled,
    /// A viewport-bounded resident operation stopped at a resumable command fence.
    Horizoned,
    Overflow {
        what: *const libc::c_char,
        n: i32,
    },
    PoolOverflow,
    StringOverflow,
    MainMemoryOverflow,
    SaveOverflow,
    NestOverflow,
    Confusion(*const libc::c_char),
    Fatal(*const libc::c_char),
    PdfError {
        t: *const libc::c_char,
        p: *const libc::c_char,
    },
    /// Prints nothing, only surfaces `msg` as the abort message.
    TtAbort(*const libc::c_char),
    CannotOpenOutput,
    NoLegalEnd {
        open_log: bool,
    },
    /// The `Halt`/`Hundred` terminal of [`ErrorOutcome`].
    HaltMessage(*const libc::c_char),
}

impl EngineAbort {
    unsafe fn overflow_report(st: &mut EngineState, what: *const libc::c_char, n: i32) {
        pre_error_message(st);
        print_cstr(st, c"TeX capacity exceeded, sorry [".as_ptr());
        print_cstr(st, what);
        print_char(st, b'=' as i32);
        print_int(st, n);
        print_char(st, b']' as i32);
        st.errors.help(&[
            "If you really absolutely need more capacity,",
            "you can ask a wizard to enlarge me.",
        ]);
        post_error_message(st, true);
        _ttbc_set_error_message(c"halted on overflow()".as_ptr());
    }

    unsafe fn fatal_report(st: &mut EngineState, s: *const libc::c_char) {
        pre_error_message(st);
        print_cstr(st, c"Emergency stop".as_ptr());
        print_nl_cstr(st, s);
        capture_to_diagnostic(st, std::ptr::null_mut());
        let _ = crate::io::close_files_and_terminate(st);
        crate::engine::tt_cleanup(st);
        ttstub_output_flush(st.shipout.rust_stdout);
        _ttbc_set_error_message(s);
    }

    /// Prints the abort and cleans up, then returns the fatal history.
    pub(crate) unsafe fn into_history(self, st: &mut EngineState) -> i32 {
        match self {
            EngineAbort::Cancelled => st.errors.history = HISTORY_FATAL_ERROR,
            EngineAbort::Horizoned => {
                return EngineAbort::Confusion(
                    c"horizon stop escaped resident orchestration".as_ptr(),
                )
                .into_history(st)
            }
            EngineAbort::Overflow { what, n } => Self::overflow_report(st, what, n),
            EngineAbort::PoolOverflow => {
                let n = st.strings.pool_size - st.strings.init_pool_ptr;
                Self::overflow_report(st, c"pool size".as_ptr(), n);
            }
            EngineAbort::StringOverflow => {
                let n = st.strings.max_strings - st.strings.init_str_ptr;
                Self::overflow_report(st, c"number of strings".as_ptr(), n);
            }
            EngineAbort::MainMemoryOverflow => {
                runaway(st);
                Self::overflow_report(st, c"main memory size".as_ptr(), MEM_TOP + 1);
            }
            EngineAbort::SaveOverflow => {
                let n = st.save_stack.save_size;
                Self::overflow_report(st, c"save size".as_ptr(), n);
            }
            EngineAbort::NestOverflow => {
                let n = st.input.nest_size;
                Self::overflow_report(st, c"semantic nest size".as_ptr(), n);
            }
            EngineAbort::Confusion(s) => {
                pre_error_message(st);
                if st.errors.history < HISTORY_ERROR_ISSUED {
                    print_cstr(st, c"This can't happen (".as_ptr());
                    print_cstr(st, s);
                    print_char(st, b')' as i32);
                    st.errors
                        .help(&["I'm broken. Please show this to someone who can fix can fix"]);
                } else {
                    print_cstr(st, c"I can't go on meeting you like this".as_ptr());
                    st.errors.help(&[
                        "One of your faux pas seems to have wounded me deeply...",
                        "in fact, I'm barely conscious. Please fix it and try again.",
                    ]);
                }
                post_error_message(st, true);
                _ttbc_set_error_message(c"halted on confusion()".as_ptr());
            }
            EngineAbort::Fatal(s) => Self::fatal_report(st, s),
            EngineAbort::NoLegalEnd { open_log } => {
                if open_log {
                    let _ = crate::io::open_log_file(st);
                }
                Self::fatal_report(st, c"*** (job aborted, no legal \\end found)".as_ptr());
            }
            EngineAbort::PdfError { t, p } => {
                pre_error_message(st);
                print_cstr(st, c"Error".as_ptr());
                if !t.is_null() {
                    print_cstr(st, c" (".as_ptr());
                    print_cstr(st, t);
                    print_char(st, b')' as i32);
                }
                print_cstr(st, c": ".as_ptr());
                print_cstr(st, p);
                post_error_message(st, true);
                _ttbc_set_error_message(c"halted on pdf_error()".as_ptr());
            }
            EngineAbort::TtAbort(msg) => _ttbc_set_error_message(msg),
            EngineAbort::CannotOpenOutput => {
                let mut buf = vec![0u8; 512];
                let np = if st.input.name_of_file.is_empty() {
                    std::ptr::null()
                } else {
                    st.input.name_of_file.as_ptr()
                };
                crate::compat::snprintf(
                    buf.as_mut_ptr() as *mut libc::c_char,
                    buf.len(),
                    c"cannot open output file \"%s\"".as_ptr(),
                    np,
                );
                let ptr = buf.leak().as_ptr() as *const libc::c_char;
                _ttbc_set_error_message(ptr);
            }
            EngineAbort::HaltMessage(msg) => {
                post_error_message(st, false);
                _ttbc_set_error_message(msg);
            }
        }
        HISTORY_FATAL_ERROR
    }
}

pub(crate) fn int_error(st: &mut EngineState, n: i32) -> ErrorOutcome {
    print_cstr(st, c" (".as_ptr());
    print_int(st, n);
    print_char(st, b')' as i32);
    error(st)
}

pub(crate) fn not_font_error_common(
    st: &mut EngineState,
    cmd: u8,
    c: i32,
    f: i32,
    suffix: *const libc::c_char,
) -> ErrorOutcome {
    error_here_with_diagnostic(st, c"Cannot use ".as_ptr());
    print_cmd_chr(st, cmd, c);
    print_cstr(st, c" with ".as_ptr());
    print(st, st.fonts.font_name[f as usize]);
    print_cstr(st, suffix);
    capture_to_diagnostic(st, std::ptr::null_mut());
    error(st)
}

pub(crate) fn give_err_help(st: &mut EngineState) {
    token_show(st, st.eqtb.local(LOCAL_ERR_HELP));
}

pub(crate) fn mu_error(st: &mut EngineState) -> ErrorOutcome {
    print_err(st, c"Incompatible glue units");
    st.errors
        .help(&["I'm going to assume that 1mu=1pt when they're mixed."]);
    error(st)
}

pub(crate) fn extra_right_brace(st: &mut EngineState) -> ErrorOutcome {
    error_here_with_diagnostic(st, c"Extra }, or forgotten ".as_ptr());
    match st.save_stack.cur_group {
        GroupCode::SemiSimple => print_esc_cstr(st, c"endgroup".as_ptr()),
        GroupCode::MathShift => print_char(st, b'$' as i32),
        GroupCode::MathLeft => print_esc_cstr(st, c"right".as_ptr()),
        _ => {}
    }
    capture_to_diagnostic(st, std::ptr::null_mut());
    st.errors.help(&[
        "I've deleted a group-closing symbol because it seems to be",
        "spurious, as in `$x}$'. But perhaps the } is legitimate and",
        r"you forgot something else, as in `\hbox{$x}'. In such cases",
        "the way to recover is to insert both the forgotten and the",
        "deleted material, e.g., by typing `I$}'.",
    ]);
    let outcome = error(st);
    if matches!(outcome, ErrorOutcome::Continue) {
        st.scanner.align_state += 1;
    }
    outcome
}

pub(crate) fn you_cant(st: &mut EngineState) {
    error_here_with_diagnostic(st, c"You can't use `".as_ptr());
    print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
    print_in_mode(st, st.input.cur_list.mode as i32);
    capture_to_diagnostic(st, std::ptr::null_mut());
}

pub(crate) fn report_illegal_case(st: &mut EngineState) -> ErrorOutcome {
    you_cant(st);
    st.errors.help(&[
        "Sorry, but I'm not programmed to handle this case;",
        "I'll just pretend that you didn't ask for it.",
        "If you're in the wrong mode, you might be able to",
        r"return to the right one by typing `I}' or `I$' or `I\par'.",
    ]);
    error(st)
}

pub(crate) fn privileged(st: &mut EngineState) -> Result<bool, ErrorOutcome> {
    if st.input.cur_list.mode > 0 {
        return Ok(true);
    }
    match report_illegal_case(st) {
        ErrorOutcome::Continue => Ok(false),
        o => Err(o),
    }
}

pub(crate) fn cs_error(st: &mut EngineState) -> ErrorOutcome {
    error_here_with_diagnostic(st, c"Extra ".as_ptr());
    print_esc_cstr(st, c"endcsname".as_ptr());
    capture_to_diagnostic(st, std::ptr::null_mut());
    st.errors
        .help(&[r"I'm ignoring this, since I wasn't doing a \csname."]);
    error(st)
}

pub(crate) fn box_error(st: &mut EngineState, n: u8) -> ErrorOutcome {
    let box_val = st.eqtb.box_reg(n as i32);
    let outcome = error(st);
    if !matches!(outcome, ErrorOutcome::Continue) {
        return outcome;
    }
    begin_diagnostic(st);
    diagnostic_begin_capture_warning_here(st);
    print_nl_cstr(st, c"The following box has been deleted:".as_ptr());
    show_box(st, box_val);
    capture_to_diagnostic(st, std::ptr::null_mut());
    end_diagnostic(st, true);
    if let Err(msg) = st.mem.flush_node_list(box_val) {
        return ErrorOutcome::Confusion(msg);
    }
    st.eqtb.set_box_reg(n as i32, crate::units::MemPtr::NULL);
    ErrorOutcome::Continue
}

pub(crate) fn runaway(st: &mut EngineState) {
    let (msg, p) = match st.scanner.scanner_status {
        DEFINING => (c"Runaway definition", st.scanner.def_ref),
        MATCHING => (c"Runaway argument", TEMP_HEAD),
        ALIGNING => (c"Runaway preamble", HOLD_HEAD),
        ABSORBING => (c"Runaway text", st.scanner.def_ref),
        _ => return,
    };
    print_nl_cstr(st, msg.as_ptr());
    print_char(st, b'?' as i32);
    print_ln(st);
    let head = p.link(&st.mem);
    let l = st.errors.error_line - 10;
    show_token_list(st, head, TEX_NULL, l);
}
