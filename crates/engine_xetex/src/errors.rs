//! Error-handling functions translated from `xetex/xetex-errors.c`.

use crate::consts::{
    BATCH_MODE, BOX_BASE, ERROR_STOP_MODE, SCROLL_MODE, SELECTOR_TERM_AND_LOG, SELECTOR_TERM_ONLY,
};

// Interaction-mode constants (xetex_format.h:644–647).

// History constants (xetex-xetexd.h:355–358).
use crate::output::{
    begin_diagnostic_in, capture_to_diagnostic_in, diagnostic_begin_capture_warning_here_in,
    end_diagnostic_in, error_here_with_diagnostic_in, print_char_in, print_cmd_chr_in,
    print_cstr_in, print_esc_cstr_in, print_in, print_in_mode_in, print_int_in, print_ln_in,
    print_nl_cstr_in, show_box_in, show_context_in, token_show_in,
};
use crate::state::EngineState;

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
    /// Set the bridge's abort message without longjmp-ing, so the host's
    /// `EngineAbortedError` surfaces the same string a `_tt_abort` would have
    /// (the fatal return path bypasses `_tt_abort`). See `01d` §1.
    fn _ttbc_set_error_message(message: *const libc::c_char);
}

// ---------------------------------------------------------------------------
// pre_error_message  (was static in C)
// ---------------------------------------------------------------------------

unsafe fn pre_error_message_in(st: &mut EngineState) {
    if st.errors.log_opened {
        st.shipout.selector = SELECTOR_TERM_AND_LOG;
    } else {
        st.shipout.selector = SELECTOR_TERM_ONLY;
    }
    if st.strings.job_name == 0 {
        // Absorb an open-log failure instead of the open_log_file()/OpenLogAbort
        // longjmp: pre_error_message runs inside into_history's terminal cleanup,
        // which commits to HISTORY_FATAL_ERROR just below. A CannotOpen is dropped —
        // the selector already fell back to TERM_ONLY, and into_history still returns
        // 3 surfacing the primary abort's message (`01d` §5.6).
        let _ = crate::io::open_log_file_in(st);
    }
    if st.errors.interaction == BATCH_MODE {
        st.shipout.selector -= 1;
    }
    crate::output::error_here_with_diagnostic_in(st, c"".as_ptr());
}

// ---------------------------------------------------------------------------
// post_error_message  (was static in C)
// ---------------------------------------------------------------------------

unsafe fn post_error_message_in(st: &mut EngineState, need_to_print_it: bool) {
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    if st.errors.interaction == ERROR_STOP_MODE {
        st.errors.interaction = SCROLL_MODE;
    }
    if need_to_print_it && st.errors.log_opened {
        // Absorb the recoverable-error outcome instead of the error()/finish()
        // longjmp: post_error_message runs inside into_history's terminal cleanup,
        // which unconditionally commits to HISTORY_FATAL_ERROR just below. A nested
        // Halt/Hundred outcome is dropped — the surrounding into_history still
        // returns 3 (an unexercised nested-abort path) (`01d` §5.6).
        let _ = error_in(st);
    }
    st.errors.history = HISTORY_FATAL_ERROR;
    // Absorb a finalize_dvi_file failure during terminal cleanup: we already
    // committed to HISTORY_FATAL_ERROR (`01d` §5.6).
    let _ = crate::io::close_files_and_terminate_in(st);
    crate::engine::tt_cleanup_in(st);
    ttstub_output_flush(st.shipout.rust_stdout);
}

// ---------------------------------------------------------------------------
// error
// ---------------------------------------------------------------------------

/// Outcome of [`error_in`]: a cold abort the wrapper must run after the state
/// borrow drops (§5 sentinel — `post_error_message`/`_tt_abort` are
/// `with_state`-bearing/FFI and would nest-panic under a live borrow). The hot
/// path returns [`ErrorOutcome::Continue`].
#[must_use]
pub(crate) enum ErrorOutcome {
    Continue,
    /// `halt_on_error_p` tripped.
    Halt,
    /// 100th recoverable error reached.
    Hundred,
    /// A threaded helper hit a `confusion(...)` cold path under the borrow.
    Confusion(&'static std::ffi::CStr),
    /// A threaded helper hit an `overflow(what, n)` cold path under the borrow.
    Overflow {
        what: &'static std::ffi::CStr,
        n: i32,
    },
}

impl ErrorOutcome {
    /// Map this cold outcome to the unified [`EngineAbort`] terminal (`01d`
    /// §5.5), mirroring the former `ErrorOutcome::finish` arm-for-arm but
    /// *without* firing — the caller propagates the value up to `tt_run_engine`,
    /// which runs the print+cleanup via [`EngineAbort::into_history`]. The
    /// `Halt`/`Hundred` `post_error_message(false)` runs inside
    /// `into_history(HaltMessage)`, not here. `Continue` never reaches a terminal.
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

/// `&mut EngineState` core of the recoverable TeX error handler (translated from
/// xetex-errors.c:52, `error`). Returns the cold-abort sentinel (§5); the hot
/// path is `Continue`. The old global `error()` wrapper (which ran this then
/// `ErrorOutcome::finish()`'s `_tt_abort` longjmp) is deleted: all callers now
/// thread the outcome to `EngineAbort` (`01d` §5.6).
pub(crate) fn error_in(st: &mut EngineState) -> ErrorOutcome {
    if st.errors.history < HISTORY_ERROR_ISSUED {
        st.errors.history = HISTORY_ERROR_ISSUED;
    }
    print_char_in(st, b'.' as i32);
    show_context_in(st);
    if st.config.halt_on_error_p != 0 {
        st.errors.history = HISTORY_FATAL_ERROR;
        return ErrorOutcome::Halt;
    }

    st.errors.error_count = st.errors.error_count.wrapping_add(1);
    if st.errors.error_count == 100 {
        print_nl_cstr_in(st, c"(That makes 100 errors; please try again.)".as_ptr());
        st.errors.history = HISTORY_FATAL_ERROR;
        return ErrorOutcome::Hundred;
    }

    if st.errors.interaction > BATCH_MODE {
        st.shipout.selector -= 1;
    }
    if st.errors.use_err_help {
        print_ln_in(st);
        give_err_help_in(st);
    } else {
        let mut hp = st.errors.help_ptr;
        while hp > 0 {
            hp -= 1;
            print_nl_cstr_in(st, st.errors.help_line[hp as usize]);
        }
        st.errors.help_ptr = hp;
    }
    print_ln_in(st);
    if st.errors.interaction > BATCH_MODE {
        st.shipout.selector += 1;
    }
    print_ln_in(st);
    ErrorOutcome::Continue
}

// ---------------------------------------------------------------------------
// EngineAbort — the unified terminal abort (plan/idiom/01d-engine-abort-and-setjmp.md)
// ---------------------------------------------------------------------------

/// The unified terminal abort cause. Every scattered `*Abort` sentinel reduces
/// to one of these (see `01d` §3). Consumed two ways:
///
/// - [`into_history`](EngineAbort::into_history) — prints + cleans **without** the
///   `_tt_abort` longjmp and returns `HISTORY_FATAL_ERROR`, for `Result` catch
///   points that propagate the abort up to `tt_run_engine`'s `i32` return.
/// - (future) `fire(self) -> !` — the single longjmp dispatcher, used by the
///   leaf wrappers not yet converted to propagate, plus the setjmp backstop.
///
/// Variants are added as each subsystem converts to propagate
/// `Result<_, EngineAbort>`; today only the `tt_run_engine` init paths use it.
pub(crate) enum EngineAbort {
    /// `overflow(what, n)` — TeX capacity exceeded.
    Overflow { what: *const libc::c_char, n: i32 },
    /// `overflow("pool size", pool_size - init_pool_ptr)` — string-pool capacity
    /// exceeded. The count is computed from `st.strings` in `into_history` (which
    /// holds `st`), so the `into_engine_abort` mapping site needs no state borrow.
    PoolOverflow,
    /// `overflow("number of strings", max_strings - init_str_ptr)` — as above but
    /// for the string count (`st.strings.max_strings - init_str_ptr`).
    StringOverflow,
    /// `runaway()` then `overflow("main memory size", MEM_TOP+1)`. `into_history`
    /// runs `runaway_in(st)` + the report (both need `st`), so the mapping site
    /// (`main_memory_overflow_abort`) needs no state borrow.
    MainMemoryOverflow,
    /// `overflow("save size", save_size)` — `into_history` reads the count from
    /// `st.save_stack.save_size`, so the `SaMutationAbort` mapping site (which has
    /// no `save_stack`) needs no state borrow (`01e`).
    SaveOverflow,
    /// `overflow("semantic nest size", nest_size)` — `into_history` reads the
    /// count from `st.input.nest_size`, so the `PushNestAbort` mapping site
    /// (which has no `st`) needs no state borrow (`01e`).
    NestOverflow,
    /// `confusion(s)` — internal consistency failure ("This can't happen").
    Confusion(*const libc::c_char),
    /// `fatal_error(s)` — "Emergency stop" + `s` (its own inline cleanup, not
    /// `post_error_message`).
    Fatal(*const libc::c_char),
    /// `pdf_error(t, p)` — "Error (t): p".
    PdfError {
        t: *const libc::c_char,
        p: *const libc::c_char,
    },
    /// `_tt_abort(msg)` — direct longjmp; `msg` is the surfaced abort message.
    TtAbort(*const libc::c_char),
    /// `_tt_abort("cannot open output file \"<name>\"")` — the message is formatted
    /// in `into_history` (which holds `st`) from `st.input.name_of_file`, so the
    /// `ShipoutAbort` mapping site needs no state borrow (`01e`).
    CannotOpenOutput,
    /// `post_error_message(false)` then `_tt_abort(msg)` — the `ErrorOutcome`
    /// `Halt`/`Hundred` terminal (halt-on-error / 100-errors).
    HaltMessage(*const libc::c_char),
}

impl EngineAbort {
    /// Print + clean the abort and return `HISTORY_FATAL_ERROR` (3) **without**
    /// longjmp-ing. Call only *after* the state borrow has dropped (it re-enters
    /// `with_state` through the print subtree). Observationally identical to the
    /// matching longjmp: same prints, same cleanup, the C wrapper returns 3, and
    /// the bridge's surfaced message is set to the same string `_tt_abort` would
    /// have written (`01d` §1).
    /// Shared print+cleanup for the three overflow terminals (`Overflow`,
    /// `PoolOverflow`, `StringOverflow`) — the former inline `Overflow` arm.
    unsafe fn overflow_report(st: &mut EngineState, what: *const libc::c_char, n: i32) {
        pre_error_message_in(st);
        print_cstr_in(st, c"TeX capacity exceeded, sorry [".as_ptr());
        print_cstr_in(st, what);
        print_char_in(st, b'=' as i32);
        print_int_in(st, n);
        print_char_in(st, b']' as i32);
        st.errors.help_ptr = 2;
        st.errors.help_line[1] = c"If you really absolutely need more capacity,".as_ptr();
        st.errors.help_line[0] = c"you can ask a wizard to enlarge me.".as_ptr();
        post_error_message_in(st, true);
        _ttbc_set_error_message(c"halted on overflow()".as_ptr());
    }

    pub(crate) unsafe fn into_history(self, st: &mut EngineState) -> i32 {
        match self {
            // The print + cleanup half of `overflow` (everything bar `_tt_abort`).
            EngineAbort::Overflow { what, n } => Self::overflow_report(st, what, n),
            // Pool/string capacity overflows compute their count from `st` here
            // (in into_history) so the mapping sites need no state borrow.
            EngineAbort::PoolOverflow => {
                let n = st.strings.pool_size - st.strings.init_pool_ptr;
                Self::overflow_report(st, c"pool size".as_ptr(), n);
            }
            EngineAbort::StringOverflow => {
                let n = st.strings.max_strings - st.strings.init_str_ptr;
                Self::overflow_report(st, c"number of strings".as_ptr(), n);
            }
            EngineAbort::MainMemoryOverflow => {
                runaway_in(st);
                // MEM_TOP + 1 (MEM_TOP = 4_999_999); constant, unchanged.
                Self::overflow_report(st, c"main memory size".as_ptr(), 5_000_000);
            }
            EngineAbort::SaveOverflow => {
                let n = st.save_stack.save_size;
                Self::overflow_report(st, c"save size".as_ptr(), n);
            }
            EngineAbort::NestOverflow => {
                let n = st.input.nest_size;
                Self::overflow_report(st, c"semantic nest size".as_ptr(), n);
            }
            // The print + cleanup half of `confusion`.
            EngineAbort::Confusion(s) => {
                pre_error_message_in(st);
                if st.errors.history < HISTORY_ERROR_ISSUED {
                    print_cstr_in(st, c"This can't happen (".as_ptr());
                    print_cstr_in(st, s);
                    print_char_in(st, b')' as i32);
                    st.errors.help_ptr = 1;
                    st.errors.help_line[0] =
                        c"I'm broken. Please show this to someone who can fix can fix".as_ptr();
                } else {
                    print_cstr_in(st, c"I can't go on meeting you like this".as_ptr());
                    st.errors.help_ptr = 2;
                    st.errors.help_line[1] =
                        c"One of your faux pas seems to have wounded me deeply...".as_ptr();
                    st.errors.help_line[0] =
                        c"in fact, I'm barely conscious. Please fix it and try again.".as_ptr();
                }
                post_error_message_in(st, true);
                _ttbc_set_error_message(c"halted on confusion()".as_ptr());
            }
            // The print + cleanup half of `fatal_error` (its own inline cleanup,
            // not `post_error_message`); `_tt_abort(s)` surfaced `s`.
            EngineAbort::Fatal(s) => {
                pre_error_message_in(st);
                print_cstr_in(st, c"Emergency stop".as_ptr());
                print_nl_cstr_in(st, s);
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                // Absorb a finalize_dvi_file failure during terminal cleanup
                // (`01d` §5.6).
                let _ = crate::io::close_files_and_terminate_in(st);
                crate::engine::tt_cleanup_in(st);
                ttstub_output_flush(st.shipout.rust_stdout);
                _ttbc_set_error_message(s);
            }
            // The print + cleanup half of `pdf_error`.
            EngineAbort::PdfError { t, p } => {
                pre_error_message_in(st);
                print_cstr_in(st, c"Error".as_ptr());
                if !t.is_null() {
                    print_cstr_in(st, c" (".as_ptr());
                    print_cstr_in(st, t);
                    print_char_in(st, b')' as i32);
                }
                print_cstr_in(st, c": ".as_ptr());
                print_cstr_in(st, p);
                post_error_message_in(st, true);
                _ttbc_set_error_message(c"halted on pdf_error()".as_ptr());
            }
            // `_tt_abort(msg)` prints nothing and runs no cleanup; it only writes
            // `msg` to the bridge's surfaced message — mirror exactly that.
            EngineAbort::TtAbort(msg) => _ttbc_set_error_message(msg),
            // Format the "cannot open output file" message here (holds `st`) from
            // the folded `st.input.name_of_file`, mirroring the former shipout site.
            EngineAbort::CannotOpenOutput => {
                let mut buf = vec![0u8; 512];
                let np = if st.input.name_of_file.is_empty() {
                    std::ptr::null()
                } else {
                    st.input.name_of_file.as_ptr()
                };
                libc::snprintf(
                    buf.as_mut_ptr() as *mut libc::c_char,
                    buf.len(),
                    c"cannot open output file \"%s\"".as_ptr(),
                    np,
                );
                let ptr = buf.leak().as_ptr() as *const libc::c_char;
                _ttbc_set_error_message(ptr);
            }
            // ErrorOutcome Halt/Hundred: post_error_message(false) then the
            // `msg` the `_tt_abort` surfaced.
            EngineAbort::HaltMessage(msg) => {
                post_error_message_in(st, false);
                _ttbc_set_error_message(msg);
            }
        }
        HISTORY_FATAL_ERROR
    }
}

// ---------------------------------------------------------------------------
// int_error
// ---------------------------------------------------------------------------

/// Append "(N)" to the current error message and call error().
///
/// translated from xetex-xetex0.c:15 (int_error, was static)
pub(crate) fn int_error_in(st: &mut EngineState, n: i32) -> ErrorOutcome {
    print_cstr_in(st, c" (".as_ptr());
    print_int_in(st, n);
    print_char_in(st, b')' as i32);
    error_in(st)
}

// ---------------------------------------------------------------------------
// not_*_font_error cluster
// ---------------------------------------------------------------------------

pub(crate) fn not_font_error_common_in(
    st: &mut EngineState,
    cmd: u16,
    c: i32,
    f: i32,
    suffix: *const libc::c_char,
) -> ErrorOutcome {
    error_here_with_diagnostic_in(st, c"Cannot use ".as_ptr());
    print_cmd_chr_in(st, cmd, c);
    print_cstr_in(st, c" with ".as_ptr());
    print_in(st, st.fonts.font_name[f as usize]);
    print_cstr_in(st, suffix);
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    error_in(st)
}

// translated from xetex-xetex0.c:13499 (give_err_help)
/// `&mut EngineState` core of `give_err_help`.
pub(crate) fn give_err_help_in(st: &mut EngineState) {
    // LOCAL(err_help) = eqtb[LOCAL_BASE + LOCAL__err_help].b32.s1
    // LOCAL_BASE = 2_254_871, LOCAL__err_help = 9
    token_show_in(st, st.eqtb.b32s1(2_254_871 + 9));
}

// ---- moved verbatim from scan.rs (workstream 02) ----

/// `&mut EngineState` core of [`mu_error`].
pub(crate) fn mu_error_in(st: &mut EngineState) -> ErrorOutcome {
    error_here_with_diagnostic_in(st, c"Incompatible glue units".as_ptr());
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 1;
    st.errors.help_line[0] = c"I'm going to assume that 1mu=1pt when they're mixed.".as_ptr();
    error_in(st)
}
/// `&mut EngineState` core of [`extra_right_brace`].
pub(crate) fn extra_right_brace_in(st: &mut EngineState) -> ErrorOutcome {
    error_here_with_diagnostic_in(st, c"Extra }, or forgotten ".as_ptr());
    match st.save_stack.cur_group {
        SEMI_SIMPLE_GROUP => print_esc_cstr_in(st, c"endgroup".as_ptr()),
        MATH_SHIFT_GROUP => print_char_in(st, b'$' as i32),
        MATH_LEFT_GROUP => print_esc_cstr_in(st, c"right".as_ptr()),
        _ => {}
    }
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 5;
    st.errors.help_line[4] = c"I've deleted a group-closing symbol because it seems to be".as_ptr();
    st.errors.help_line[3] =
        c"spurious, as in `$x}$'. But perhaps the } is legitimate and".as_ptr();
    st.errors.help_line[2] =
        c"you forgot something else, as in `\\hbox{$x}'. In such cases".as_ptr();
    st.errors.help_line[1] = c"the way to recover is to insert both the forgotten and the".as_ptr();
    st.errors.help_line[0] = c"deleted material, e.g., by typing `I$}'.".as_ptr();
    let outcome = error_in(st);
    if matches!(outcome, ErrorOutcome::Continue) {
        st.scanner.align_state += 1;
    }
    outcome
}
/// `&mut EngineState` core of the `you_cant` diagnostic.
pub(crate) fn you_cant_in(st: &mut EngineState) {
    error_here_with_diagnostic_in(st, c"You can't use `".as_ptr());
    print_cmd_chr_in(st, st.scanner.cur_cmd as u16, st.scanner.cur_chr);
    print_in_mode_in(st, st.input.cur_list.mode as i32);
    capture_to_diagnostic_in(st, std::ptr::null_mut());
}
/// `&mut EngineState` core of [`report_illegal_case`].
pub(crate) fn report_illegal_case_in(st: &mut EngineState) -> ErrorOutcome {
    you_cant_in(st);
    st.errors.help_ptr = 4;
    st.errors.help_line[3] = c"Sorry, but I'm not programmed to handle this case;".as_ptr();
    st.errors.help_line[2] = c"I'll just pretend that you didn't ask for it.".as_ptr();
    st.errors.help_line[1] = c"If you're in the wrong mode, you might be able to".as_ptr();
    st.errors.help_line[0] =
        c"return to the right one by typing `I}' or `I$' or `I\\par'.".as_ptr();
    error_in(st)
}
pub(crate) fn privileged_in(st: &mut EngineState) -> Result<bool, ErrorOutcome> {
    if st.input.cur_list.mode > 0 {
        Ok(true)
    } else {
        let o = report_illegal_case_in(st);
        if matches!(o, ErrorOutcome::Continue) {
            Ok(false)
        } else {
            Err(o)
        }
    }
}
// translated from xetex-xetex0.c:10489 (cs_error)
/// `&mut EngineState` core of [`cs_error`].
pub(crate) fn cs_error_in(st: &mut EngineState) -> ErrorOutcome {
    error_here_with_diagnostic_in(st, c"Extra ".as_ptr());
    print_esc_cstr_in(st, c"endcsname".as_ptr());
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 1;
    st.errors.help_line[0] = c"I'm ignoring this, since I wasn't doing a \\csname.".as_ptr();
    error_in(st)
}
// translated from xetex-xetex0.c:9270 (box_error)
/// `&mut EngineState` core of [`box_error`].
pub(crate) fn box_error_in(st: &mut EngineState, n: u8) -> ErrorOutcome {
    let box_val = st.eqtb.b32s1(BOX_BASE + n as i32);
    let outcome = error_in(st);
    if !matches!(outcome, ErrorOutcome::Continue) {
        return outcome;
    }
    begin_diagnostic_in(st);
    diagnostic_begin_capture_warning_here_in(st);
    print_nl_cstr_in(st, c"The following box has been deleted:".as_ptr());
    show_box_in(st, box_val);
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    end_diagnostic_in(st, true);
    // flush_node_list aborts via confusion before returning on a corrupt list;
    // defer that cold path so it runs after the borrow drops, before the write.
    if let Err(msg) = crate::packaging::flush_node_list(&mut st.mem, box_val) {
        return ErrorOutcome::Confusion(msg);
    }
    // BOX_REG(n) = TEX_NULL — direct eqtb write, no save-stack involvement
    st.eqtb.set_b32s1(BOX_BASE + n as i32, crate::mem::TEX_NULL);
    ErrorOutcome::Continue
}
/// `&mut EngineState` core of `runaway` (`01b` §3). Self-contained cold
/// diagnostic: reads `scanner_status`/`def_ref` (`st.scanner`), `error_line`
/// (`st.errors`), and the token-list head (`st.mem`), printing in-order through
/// the threaded `print_*_in`/`show_token_list_in` cores (sessions 182–183). No
/// allocation, no cold-abort path.
pub(crate) fn runaway_in(st: &mut crate::state::EngineState) {
    use crate::mem::TEX_NULL;

    const SKIPPING: u8 = 1;
    const DEFINING: u8 = 2;
    const MATCHING: u8 = 3;
    const ALIGNING: u8 = 4;
    const ABSORBING: u8 = 5;
    const TEMP_HEAD: i32 = 4_999_996; // MEM_TOP - 3
    const HOLD_HEAD: i32 = 4_999_995; // MEM_TOP - 4

    if st.scanner.scanner_status > SKIPPING {
        let p = match st.scanner.scanner_status {
            DEFINING => {
                crate::output::print_nl_cstr_in(st, c"Runaway definition".as_ptr());
                st.scanner.def_ref
            }
            MATCHING => {
                crate::output::print_nl_cstr_in(st, c"Runaway argument".as_ptr());
                TEMP_HEAD
            }
            ALIGNING => {
                crate::output::print_nl_cstr_in(st, c"Runaway preamble".as_ptr());
                HOLD_HEAD
            }
            ABSORBING => {
                crate::output::print_nl_cstr_in(st, c"Runaway text".as_ptr());
                st.scanner.def_ref
            }
            _ => return,
        };
        crate::output::print_char_in(st, '?' as i32);
        crate::output::print_ln_in(st);
        let head = st.mem.b32s1(p);
        let l = st.errors.error_line - 10;
        crate::output::show_token_list_in(st, head, TEX_NULL, l);
    }
}
