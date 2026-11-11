use crate::bridge::{output_flush, set_error_message};
use crate::cmd::Cmd;
use crate::codes::ScannerStatus;
use crate::consts::{HISTORY_ERROR_ISSUED, HISTORY_FATAL_ERROR, MAIN_MEMORY_WORDS};
use crate::diagnostic::{begin_capture, emit, emit_fatal, take_capture, DiagnosticKind};
use crate::output::{
    begin_diagnostic, end_diagnostic, print, print_char, print_err, print_error_prefix, print_int,
    print_ln, print_nl, show_box, show_context, show_token_list, token_show, CmdChr, InMode,
    TexPrint,
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

    /// The help text set for the next error, one entry per line.
    fn help_text(&self) -> Vec<String> {
        self.help_lines[..usize::from(self.help_len)]
            .iter()
            .map(|line| (*line).to_owned())
            .collect()
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
    print_error_prefix(st);
    begin_capture(st, DiagnosticKind::Fatal);
}

fn post_error_message(st: &mut EngineState, need_to_print_it: bool) {
    let help = st.errors.help_text();
    if let Some(diagnostic) = take_capture(st, help) {
        emit(st, diagnostic);
    }
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

/// TeX's `error`, after the message is printed: the context, the help, and
/// the stop on `--halt-on-error` or the hundredth error.
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
    /// `overflow(capacity, n)`: a fixed capacity of size `n` ran out.
    Overflow {
        capacity: Capacity,
        n: i32,
    },
    /// `get_avail` ran out: the runaway text, then the main memory overflow.
    AvailOverflow,
    Confusion(&'static str),
    Fatal(&'static str),
    /// `\dump` inside a group, which TeX reports and then succumbs to.
    DumpInGroup,
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
    /// No input file could be opened under the name.
    CannotOpenInput {
        name: Box<[u8]>,
    },
    /// The output file of that name could not be created.
    CannotOpenOutput {
        name: Box<[u8]>,
    },
}

/// Turns a broken internal invariant into TeX's "This can't happen" stop.
pub(crate) trait OrConfusion<T> {
    fn or_confusion(self, what: &'static str) -> Result<T, Abort>;
}

impl<T> OrConfusion<T> for Option<T> {
    #[inline]
    fn or_confusion(self, what: &'static str) -> Result<T, Abort> {
        self.ok_or(Abort::Confusion(what))
    }
}

impl<T, E> OrConfusion<T> for Result<T, E> {
    #[inline]
    fn or_confusion(self, what: &'static str) -> Result<T, Abort> {
        self.map_err(|_| Abort::Confusion(what))
    }
}

/// A fixed capacity of the engine that a run can exceed.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Capacity {
    MainMemory,
    PoolSize,
    NumberOfStrings,
    SaveSize,
    SemanticNest,
    InputStack,
    ParameterStack,
    TextInputLevels,
    GroupingLevels,
    BufferSize,
    NativeTextSize,
    FontMemory,
    HashSize,
    PrimitiveSize,
    PatternMemory,
    PatternMemoryOps,
    PatternMemoryOpsPerLanguage,
    ExceptionDictionary,
    ExpansionDepth,
}

impl Capacity {
    /// The name `overflow` prints.
    fn name(self) -> &'static str {
        match self {
            Capacity::MainMemory => "main memory size",
            Capacity::PoolSize => "pool size",
            Capacity::NumberOfStrings => "number of strings",
            Capacity::SaveSize => "save size",
            Capacity::SemanticNest => "semantic nest size",
            Capacity::InputStack => "input stack size",
            Capacity::ParameterStack => "parameter stack size",
            Capacity::TextInputLevels => "text input levels",
            Capacity::GroupingLevels => "grouping levels",
            Capacity::BufferSize => "buffer size",
            Capacity::NativeTextSize => "native text size",
            Capacity::FontMemory => "font memory",
            Capacity::HashSize => "hash size",
            Capacity::PrimitiveSize => "primitive size",
            Capacity::PatternMemory => "pattern memory",
            Capacity::PatternMemoryOps => "pattern memory ops",
            Capacity::PatternMemoryOpsPerLanguage => "pattern memory ops per language",
            Capacity::ExceptionDictionary => "exception dictionary",
            Capacity::ExpansionDepth => "expansion depth",
        }
    }
}

impl Abort {
    /// `get_node` ran out of main memory.
    pub(crate) const NODE_OVERFLOW: Abort = Abort::Overflow {
        capacity: Capacity::MainMemory,
        n: MAIN_MEMORY_WORDS,
    };

    /// `capacity`, whose size is a `usize`, ran out.
    pub(crate) fn overflow(capacity: Capacity, n: usize) -> Abort {
        Abort::Overflow {
            capacity,
            n: i32::try_from(n).unwrap_or(i32::MAX),
        }
    }
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
        st.errors.help(&[s]);
        if let Some(diagnostic) = take_capture(st, st.errors.help_text()) {
            emit_fatal(st, diagnostic);
        }
        post_error_message(st, true);
        set_error_message(s);
    }

    fn open_failure(what: &str, name: &[u8]) {
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
            Abort::Overflow { capacity, n } => Self::overflow_report(st, capacity.name(), n),
            Abort::AvailOverflow => {
                runaway(st);
                Self::overflow_report(st, Capacity::MainMemory.name(), MAIN_MEMORY_WORDS);
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
            Abort::DumpInGroup => {
                print_err(st, DiagnosticKind::Fatal, "You can't dump inside a group");
                st.errors.help(&[r"`{...\dump}' is a no-no."]);
                post_error_message(st, true);
                set_error_message("\\dump inside a group");
            }
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
            Abort::CannotOpenInput { name } => {
                Self::open_failure("failed to open input file", &name)
            }
            Abort::CannotOpenOutput { name } => {
                Self::open_failure("cannot open output file", &name)
            }
        }
        HISTORY_FATAL_ERROR
    }
}

pub(crate) fn not_font_error_common(
    st: &mut EngineState,
    cmd: Cmd,
    c: i32,
    f: FontId,
    suffix: &str,
) -> Result<(), Abort> {
    let font = st.fonts.font_name[f];
    error(
        st,
        ("Cannot use ", CmdChr(cmd, c), " with ", font, suffix),
        &[],
    )
}

pub(crate) fn give_err_help(st: &mut EngineState) {
    token_show(st, st.eqtb.tok_par(TokPar::ErrHelp));
}

/// Print `message` as an error and report it with `help`.
fn report_error(st: &mut EngineState, message: impl TexPrint, help: &[&'static str]) {
    print_err(st, DiagnosticKind::Error, message);
    st.errors.help(help);
    if let Some(diagnostic) = take_capture(st, st.errors.help_text()) {
        emit(st, diagnostic);
    }
}

/// Report a recoverable error and carry on (`print_err`, `help`, `error`).
pub(crate) fn error(
    st: &mut EngineState,
    message: impl TexPrint,
    help: &[&'static str],
) -> Result<(), Abort> {
    report_error(st, message, help);
    finish_error(st)
}

/// `\errmessage`: an error whose help is `\errhelp` when that is set, else `fallback`.
/// `fallback` is unused when `\errhelp` is set.
pub(crate) fn errmessage(
    st: &mut EngineState,
    message: StrNumber,
    fallback: &[&'static str],
) -> Result<(), Abort> {
    let err_help = st.eqtb.tok_par(TokPar::ErrHelp);
    if err_help.is_null() {
        return error(st, message, fallback);
    }
    let help = err_help_text(st, err_help);
    print_err(st, DiagnosticKind::Error, message);
    if let Some(diagnostic) = take_capture(st, help) {
        emit(st, diagnostic);
    }
    st.errors.use_err_help = true;
    let outcome = finish_error(st);
    st.errors.use_err_help = false;
    outcome
}

/// The lines `\errhelp` shows, as the transcript prints them.
fn err_help_text(st: &mut EngineState, list: crate::token::TokenList) -> Vec<String> {
    let old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    let start = st.strings.cur_string_start();
    token_show(st, list);
    st.shipout.selector = old_setting;
    let units: Vec<u16> = start
        .until(st.strings.pool_ptr)
        .map(|k| st.strings.str_pool[k.index()])
        .collect();
    st.strings.discard_cur_string();
    String::from_utf16_lossy(&units)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// [`error`], first putting the current token back to be read again.
pub(crate) fn back_error(
    st: &mut EngineState,
    message: impl TexPrint,
    help: &[&'static str],
) -> Result<(), Abort> {
    report_error(st, message, help);
    crate::input::back_input(st)?;
    finish_error(st)
}

/// [`back_error`], marking the token as inserted rather than read.
pub(crate) fn ins_error(
    st: &mut EngineState,
    message: impl TexPrint,
    help: &[&'static str],
) -> Result<(), Abort> {
    report_error(st, message, help);
    crate::input::back_input(st)?;
    st.input
        .cur_input
        .set_token_type(crate::input::TokenType::Inserted);
    finish_error(st)
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

/// "You can't use `<current command>' in <mode>".
pub(crate) fn you_cant(st: &EngineState) -> impl TexPrint {
    (
        "You can't use `",
        CmdChr(st.scanner.cur_cmd, st.scanner.cur_chr),
        InMode(i32::from(st.input.cur_list.mode)),
    )
}

pub(crate) fn report_illegal_case(st: &mut EngineState) -> Result<(), Abort> {
    error(
        st,
        you_cant(st),
        &[
            "Sorry, but I'm not programmed to handle this case;",
            "I'll just pretend that you didn't ask for it.",
            "If you're in the wrong mode, you might be able to",
            r"return to the right one by typing `I}' or `I$' or `I\par'.",
        ],
    )
}

pub(crate) fn privileged(st: &mut EngineState) -> Result<bool, Abort> {
    if st.input.cur_list.mode > 0 {
        return Ok(true);
    }
    report_illegal_case(st)?;
    Ok(false)
}

pub(crate) fn cs_error(st: &mut EngineState) -> Result<(), Abort> {
    crate::errors::error(
        st,
        ("Extra ", crate::output::Esc("endcsname")),
        &[r"I'm ignoring this, since I wasn't doing a \csname."],
    )
}

/// [`error`], then show and discard box register `n`.
pub(crate) fn box_error(
    st: &mut EngineState,
    n: u8,
    message: impl TexPrint,
    help: &[&'static str],
) -> Result<(), Abort> {
    let box_val = st.eqtb.box_reg(i32::from(n));
    error(st, message, help)?;
    begin_diagnostic(st);
    print_nl(st, "The following box has been deleted:");
    show_box(st, box_val);
    end_diagnostic(st, true);
    st.mem.flush_node_list(box_val)?;
    st.eqtb
        .set_box_reg(i32::from(n), crate::units::MemPtr::NULL);
    Ok(())
}

/// Shows the list being built when scanning ran away: the innermost open build.
pub(crate) fn runaway(st: &mut EngineState) {
    let msg = match st.scanner.scanner_status {
        ScannerStatus::Defining => "Runaway definition",
        ScannerStatus::Matching => "Runaway argument",
        ScannerStatus::Aligning => "Runaway preamble",
        ScannerStatus::Absorbing => "Runaway text",
        _ => return,
    };
    print_nl(st, msg);
    print_char(st, b'?');
    print_ln(st);
    let built = st.mem.tokens.innermost_built();
    let l = st.errors.error_line - 10;
    show_token_list(st, built, crate::token::TokCursor::END, l);
}
