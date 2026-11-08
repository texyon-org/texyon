use crate::bridge::{output_flush, set_error_message};
use crate::cmd::Cmd;
use crate::codes::ScannerStatus;
use crate::consts::{HISTORY_ERROR_ISSUED, HISTORY_FATAL_ERROR, MEM_TOP};
use crate::output::{
    append_to_diagnostic, begin_diagnostic, capture_to_diagnostic,
    diagnostic_begin_capture_warning_here, end_diagnostic, error_here_with_diagnostic, print,
    print_char, print_cmd_chr, print_err, print_esc, print_in_mode, print_int, print_ln, print_nl,
    show_box, show_context, show_token_list, token_show, TexPrint,
};
use crate::params::{IntPar, TokPar};
use crate::state::{EngineState, EqtbState, ErrorState, InteractionMode, Selector};
use crate::units::{FontId, StrNumber};

impl ErrorState {
    /// Sets the help text shown after the next error, in display order.
    pub(crate) fn help(&mut self, lines: &[&'static str]) {
        self.help_lines[..lines.len()].copy_from_slice(lines);
        self.help_len = lines.len() as u8;
    }
}

fn pre_error_message(st: &mut EngineState) {
    st.shipout.selector = if st.errors.log_opened {
        Selector::TermAndLog
    } else {
        Selector::TermOnly
    };
    if st.strings.job_name == StrNumber::NONE {
        // Already on the fatal path, so a log-open failure just leaves TERM_ONLY.
        let _ = crate::io::open_log_file(st);
    }
    if st.errors.interaction == InteractionMode::Batch {
        st.shipout.selector = st.shipout.selector.without_terminal();
    }
    error_here_with_diagnostic(st, "");
}

fn post_error_message(st: &mut EngineState, need_to_print_it: bool) {
    capture_to_diagnostic(st, None);
    if st.errors.interaction == InteractionMode::ErrorStop {
        st.errors.interaction = InteractionMode::Scroll;
    }
    if need_to_print_it && st.errors.log_opened {
        // Already committed to a fatal history, so a nested abort is dropped.
        let _ = finish_error(st);
    }
    st.errors.history = HISTORY_FATAL_ERROR;
    let _ = crate::io::close_files_and_terminate(st);
    crate::engine::cleanup(st);
    output_flush(st.shipout.rust_stdout);
}

pub(crate) fn finish_error(st: &mut EngineState) -> Result<(), Abort> {
    if st.errors.history < HISTORY_ERROR_ISSUED {
        st.errors.history = HISTORY_ERROR_ISSUED;
    }
    print_char(st, b'.');
    show_context(st);
    if st.config.halt_on_error_p != 0 {
        // A second error while halting would print the help again, so stop at once.
        if !st.errors.halting_on_error {
            st.errors.halting_on_error = true;
            put_help_on_transcript(st);
        }
        st.errors.history = HISTORY_FATAL_ERROR;
        return Err(Abort::HaltOnError);
    }

    st.errors.error_count = st.errors.error_count.wrapping_add(1);
    if st.errors.error_count == 100 {
        print_nl(st, "(That makes 100 errors; please try again.)");
        st.errors.history = HISTORY_FATAL_ERROR;
        return Err(Abort::HundredErrors);
    }

    put_help_on_transcript(st);
    Ok(())
}

/// TeX's "Put help message on the transcript file".
fn put_help_on_transcript(st: &mut EngineState) {
    if st.errors.interaction > InteractionMode::Batch {
        st.shipout.selector = st.shipout.selector.without_terminal();
    }
    if st.errors.use_err_help {
        print_ln(st);
        give_err_help(st);
    } else {
        let lines = st.errors.help_lines;
        let len = st.errors.help_len as usize;
        st.errors.help_len = 0;
        for line in &lines[..len] {
            print_nl(st, *line);
        }
    }
    print_ln(st);
    if st.errors.interaction > InteractionMode::Batch {
        st.shipout.selector = st.shipout.selector.with_terminal();
    }
    print_ln(st);
}

/// Why a run ends early. Reported once, by [`Abort::into_history`].
#[must_use]
#[derive(Debug)]
pub(crate) enum Abort {
    /// A resident operation was cancelled cooperatively.
    Cancelled,
    /// A viewport-bounded resident operation stopped at a resumable command fence.
    Horizoned,
    /// `overflow(what, n)`.
    Overflow {
        what: &'static str,
        n: i32,
    },
    PoolOverflow,
    StringOverflow,
    /// `get_avail` ran out: the runaway text, then the main memory overflow.
    AvailOverflow,
    /// `get_node` ran out: the main memory overflow alone.
    NodeOverflow,
    SaveOverflow,
    NestOverflow,
    Confusion(&'static str),
    Fatal(&'static str),
    /// `pdf_error(t, p)`.
    PdfError(&'static [&'static str; 2]),
    /// `error` with `\haltonerror` set.
    HaltOnError,
    /// The hundredth `error` of the run.
    HundredErrors,
    NoLegalEnd {
        open_log: bool,
    },
    /// A host-level failure: no TeX error is printed, only `msg` is surfaced.
    Internal(&'static str),
    InputClose,
    CannotOpenInput,
    CannotOpenOutput,
}

/// An error `\ignoreprimitiveerror` can turn into a warning, valued by its bit (one-based).
#[derive(Copy, Clone)]
pub(crate) enum IgnoredError {
    /// Infinite glue shrinkage in a box being split.
    VsplitInfiniteShrink = 1,
}

impl EqtbState {
    /// Whether `\ignoreprimitiveerror` has the bit for `e` set, as WEB's `is_bit_set` reads it.
    pub(crate) fn ignores_primitive_error(&self, e: IgnoredError) -> bool {
        let n = self.int_par(IntPar::IgnorePrimitiveError);
        let m = 1_i32.checked_shl(e as u32 - 1).unwrap_or(0);
        m != 0 && (n / m) % 2 != 0
    }
}

impl Abort {
    fn overflow_report(st: &mut EngineState, what: &str, n: i32) {
        pre_error_message(st);
        print(st, "TeX capacity exceeded, sorry [");
        print(st, what);
        print_char(st, b'=');
        print_int(st, n);
        print_char(st, b']');
        st.errors.help(&[
            "If you really absolutely need more capacity,",
            "you can ask a wizard to enlarge me.",
        ]);
        post_error_message(st, true);
        set_error_message("halted on overflow()");
    }

    fn fatal_report(st: &mut EngineState, s: &'static str) {
        pre_error_message(st);
        print(st, "Emergency stop");
        // WEB prints the reason as help, after the context, but the diagnostic needs it.
        append_to_diagnostic(st, "\n");
        append_to_diagnostic(st, s);
        st.errors.help(&[s]);
        post_error_message(st, true);
        set_error_message(s);
    }

    fn open_failure(st: &EngineState, what: &str) {
        let name = st.input.name_of_file_bytes();
        let mut msg = format!("{what} \"").into_bytes();
        msg.extend_from_slice(name);
        msg.push(b'"');
        msg.truncate(511);
        set_error_message(msg);
    }

    /// Prints the abort and cleans up, then returns the fatal history.
    pub(crate) fn into_history(self, st: &mut EngineState) -> i32 {
        match self {
            Abort::Cancelled => st.errors.history = HISTORY_FATAL_ERROR,
            Abort::Horizoned => {
                return Abort::Confusion("horizon stop escaped resident orchestration")
                    .into_history(st)
            }
            Abort::Overflow { what, n } => Self::overflow_report(st, what, n),
            Abort::PoolOverflow => {
                let n = st.strings.pool_size - st.strings.init_pool_ptr;
                Self::overflow_report(st, "pool size", n);
            }
            Abort::StringOverflow => {
                let n = st.strings.max_strings - st.strings.init_str_ptr;
                Self::overflow_report(st, "number of strings", n);
            }
            Abort::AvailOverflow => {
                runaway(st);
                Self::overflow_report(st, "main memory size", MEM_TOP + 1);
            }
            Abort::NodeOverflow => Self::overflow_report(st, "main memory size", MEM_TOP + 1),
            Abort::SaveOverflow => {
                let n = i32::try_from(st.save_stack.save_size).unwrap_or(i32::MAX);
                Self::overflow_report(st, "save size", n);
            }
            Abort::NestOverflow => {
                let n = i32::try_from(st.input.nest.size()).unwrap_or(i32::MAX);
                Self::overflow_report(st, "semantic nest size", n);
            }
            Abort::Confusion(s) => {
                pre_error_message(st);
                if st.errors.history < HISTORY_ERROR_ISSUED {
                    print(st, "This can't happen (");
                    print(st, s);
                    print_char(st, b')');
                    st.errors
                        .help(&["I'm broken. Please show this to someone who can fix can fix"]);
                } else {
                    print(st, "I can't go on meeting you like this");
                    st.errors.help(&[
                        "One of your faux pas seems to have wounded me deeply...",
                        "in fact, I'm barely conscious. Please fix it and try again.",
                    ]);
                }
                post_error_message(st, true);
                set_error_message("halted on confusion()");
            }
            Abort::Fatal(s) => Self::fatal_report(st, s),
            Abort::NoLegalEnd { open_log } => {
                if open_log {
                    let _ = crate::io::open_log_file(st);
                }
                Self::fatal_report(st, "*** (job aborted, no legal \\end found)");
            }
            Abort::PdfError([t, p]) => {
                pre_error_message(st);
                print(st, "Error (");
                print(st, *t);
                print(st, "): ");
                print(st, *p);
                post_error_message(st, true);
                set_error_message("halted on pdf_error()");
            }
            Abort::HaltOnError => {
                post_error_message(st, false);
                set_error_message("halted on potentially-recoverable error as specified");
            }
            Abort::HundredErrors => {
                post_error_message(st, false);
                set_error_message("halted after 100 potentially-recoverable errors");
            }
            Abort::Internal(msg) => set_error_message(msg),
            Abort::InputClose => set_error_message("serious internal error closing an input file"),
            Abort::CannotOpenInput => Self::open_failure(st, "failed to open input file"),
            Abort::CannotOpenOutput => Self::open_failure(st, "cannot open output file"),
        }
        HISTORY_FATAL_ERROR
    }
}

pub(crate) fn int_error(st: &mut EngineState, n: i32) -> Result<(), Abort> {
    print(st, " (");
    print_int(st, n);
    print_char(st, b')');
    finish_error(st)
}

pub(crate) fn not_font_error_common(
    st: &mut EngineState,
    cmd: Cmd,
    c: i32,
    f: FontId,
    suffix: &str,
) -> Result<(), Abort> {
    error_here_with_diagnostic(st, "Cannot use ");
    print_cmd_chr(st, cmd, c);
    print(st, " with ");
    print(st, st.fonts.font_name[f]);
    print(st, suffix);
    capture_to_diagnostic(st, None);
    finish_error(st)
}

pub(crate) fn give_err_help(st: &mut EngineState) {
    token_show(st, st.eqtb.tok_par(TokPar::ErrHelp));
}

/// Report a recoverable error and carry on (`print_err`, `help`, `error`).
pub(crate) fn error(
    st: &mut EngineState,
    message: impl TexPrint,
    help: &[&'static str],
) -> Result<(), Abort> {
    print_err(st, message);
    st.errors.help(help);
    finish_error(st)
}

/// [`error`], first putting the current token back to be read again.
pub(crate) fn back_error(
    st: &mut EngineState,
    message: impl TexPrint,
    help: &[&'static str],
) -> Result<(), Abort> {
    print_err(st, message);
    st.errors.help(help);
    crate::input::back_finish_error(st)
}

/// [`back_error`], marking the token as inserted rather than read.
pub(crate) fn ins_error(
    st: &mut EngineState,
    message: impl TexPrint,
    help: &[&'static str],
) -> Result<(), Abort> {
    print_err(st, message);
    st.errors.help(help);
    crate::input::ins_finish_error(st)
}

pub(crate) fn mu_error(st: &mut EngineState) -> Result<(), Abort> {
    error(
        st,
        "Incompatible glue units",
        &["I'm going to assume that 1mu=1pt when they're mixed."],
    )
}

pub(crate) fn extra_right_brace(st: &mut EngineState) -> Result<(), Abort> {
    let closer = st.save_stack.cur_group.closer();
    error(
        st,
        ("Extra }, or forgotten ", closer),
        &[
            "I've deleted a group-closing symbol because it seems to be",
            "spurious, as in `$x}$'. But perhaps the } is legitimate and",
            r"you forgot something else, as in `\hbox{$x}'. In such cases",
            "the way to recover is to insert both the forgotten and the",
            "deleted material, e.g., by typing `I$}'.",
        ],
    )?;
    st.scanner.align_state += 1;
    Ok(())
}

pub(crate) fn you_cant(st: &mut EngineState) {
    error_here_with_diagnostic(st, "You can't use `");
    print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
    print_in_mode(st, i32::from(st.input.cur_list.mode));
    capture_to_diagnostic(st, None);
}

pub(crate) fn report_illegal_case(st: &mut EngineState) -> Result<(), Abort> {
    you_cant(st);
    st.errors.help(&[
        "Sorry, but I'm not programmed to handle this case;",
        "I'll just pretend that you didn't ask for it.",
        "If you're in the wrong mode, you might be able to",
        r"return to the right one by typing `I}' or `I$' or `I\par'.",
    ]);
    finish_error(st)
}

pub(crate) fn privileged(st: &mut EngineState) -> Result<bool, Abort> {
    if st.input.cur_list.mode > 0 {
        return Ok(true);
    }
    report_illegal_case(st)?;
    Ok(false)
}

pub(crate) fn cs_error(st: &mut EngineState) -> Result<(), Abort> {
    error_here_with_diagnostic(st, "Extra ");
    print_esc(st, "endcsname");
    capture_to_diagnostic(st, None);
    st.errors
        .help(&[r"I'm ignoring this, since I wasn't doing a \csname."]);
    finish_error(st)
}

pub(crate) fn box_error(st: &mut EngineState, n: u8) -> Result<(), Abort> {
    let box_val = st.eqtb.box_reg(i32::from(n));
    finish_error(st)?;
    begin_diagnostic(st);
    diagnostic_begin_capture_warning_here(st);
    print_nl(st, "The following box has been deleted:");
    show_box(st, box_val);
    capture_to_diagnostic(st, None);
    end_diagnostic(st, true);
    st.mem.flush_node_list(box_val)?;
    st.eqtb
        .set_box_reg(i32::from(n), crate::units::MemPtr::NULL);
    Ok(())
}

pub(crate) fn runaway(st: &mut EngineState) {
    let (msg, p) = match st.scanner.scanner_status {
        ScannerStatus::Defining => ("Runaway definition", st.scanner.def_ref),
        ScannerStatus::Matching => ("Runaway argument", crate::mem::TEMP_HEAD_TOKS),
        ScannerStatus::Aligning => ("Runaway preamble", crate::mem::HOLD_HEAD_TOKS),
        ScannerStatus::Absorbing => ("Runaway text", st.scanner.def_ref),
        _ => return,
    };
    print_nl(st, msg);
    print_char(st, b'?');
    print_ln(st);
    let head = p.link(&st.mem);
    let l = st.errors.error_line - 10;
    show_token_list(st, head, crate::token::TokenList::NULL, l);
}
