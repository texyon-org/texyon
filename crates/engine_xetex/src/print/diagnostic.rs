//! What the engine tells its host about the document: errors, warnings, and
//! the text the document writes to its transcript.

use std::mem;

use crate::base::units::StrNumber;
use crate::ffi::bridge;
use crate::ffi::bridge::{diag_append, diag_finish};
use crate::state::{EngineExecutionMode, EngineState};

/// What a diagnostic reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    /// The run stopped. The help says why.
    Fatal,
    /// TeX could not do what was asked and recovered as best it could.
    Error,
    /// Something TeX itself found suspect, such as an overfull box.
    Warning,
    /// Text the document wrote to the transcript, for the host to classify.
    Written,
}

/// One thing the engine reported about one place in the source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// What it reports.
    pub kind: DiagnosticKind,
    /// The message as the transcript prints it, without TeX's line wrapping.
    pub message: String,
    /// TeX's help text, one entry per line.
    pub help: Vec<String>,
    /// The innermost open input file, as TeX names it.
    pub file: Option<String>,
    /// One-based line in `file`.
    pub line: Option<u32>,
}

/// A diagnostic whose message is being printed.
#[derive(Clone, Debug)]
pub(crate) struct Capture {
    kind: DiagnosticKind,
    file: Option<String>,
    line: Option<u32>,
    text: Vec<u8>,
}

impl Capture {
    fn into_diagnostic(self, help: Vec<String>) -> Diagnostic {
        Diagnostic {
            kind: self.kind,
            message: String::from_utf8_lossy(&self.text).into_owned(),
            help,
            file: self.file,
            line: self.line,
        }
    }
}

/// The diagnostics of the running operation. Restoring a checkpoint keeps
/// them, so a resumed operation reports only what it ran.
#[derive(Clone, Debug, Default)]
pub(crate) struct DiagnosticsState {
    capture: Option<Capture>,
    issued: Vec<Diagnostic>,
    /// Diagnostics of earlier operations this one continues after.
    pub resumed_at: Option<usize>,
}

impl DiagnosticsState {
    /// Starts a new operation's list.
    pub(crate) fn begin_operation(&mut self) {
        self.capture = None;
        self.issued.clear();
        self.resumed_at = None;
    }

    /// The operation resumed after `issued` diagnostics of earlier ones.
    pub(crate) fn resume_after(&mut self, issued: usize) {
        self.capture = None;
        self.issued.clear();
        self.resumed_at = Some(issued);
    }

    /// The diagnostics this operation reported, in order.
    pub(crate) fn take(&mut self) -> Vec<Diagnostic> {
        mem::take(&mut self.issued)
    }

    /// Adds a printed byte to the message being captured.
    #[inline]
    pub(crate) fn mirror(&mut self, byte: u8) {
        if let Some(capture) = &mut self.capture {
            capture.text.push(byte);
        }
    }
}

/// The innermost input level that is a real file, with its current line.
pub(crate) fn current_file_line(st: &EngineState) -> Option<(StrNumber, i32)> {
    let mut level = st.input.in_open;
    while level > 0 && st.input.open_levels[level].full_source_name == StrNumber::NONE {
        level -= 1;
    }
    if level == 0 {
        return None;
    }
    let line = if level == st.input.in_open {
        st.mem.line
    } else {
        st.input.open_levels[level + 1].outer_line
    };
    Some((st.input.open_levels[level].full_source_name, line))
}

/// Starts capturing a `kind` diagnostic at the current position, reporting
/// any capture still open.
pub(crate) fn begin_capture(st: &mut EngineState, kind: DiagnosticKind) {
    end_capture(st);
    let (file, line) = match current_file_line(st) {
        Some((name, line)) => {
            let name = st.strings.texstring_bytes(name);
            (
                Some(String::from_utf8_lossy(&name).into_owned()),
                u32::try_from(line).ok(),
            )
        }
        None => (None, None),
    };
    st.diagnostics.capture = Some(Capture {
        kind,
        file,
        line,
        text: Vec::new(),
    });
}

/// Starts capturing a warning at the current position.
pub(crate) fn begin_warning(st: &mut EngineState) {
    begin_capture(st, DiagnosticKind::Warning);
}

/// Reports a warning to the host without printing it to the transcript.
pub(crate) fn warn_host(st: &mut EngineState, message: &str) {
    begin_warning(st);
    if let Some(capture) = &mut st.diagnostics.capture {
        capture.text.extend_from_slice(message.as_bytes());
    }
    end_capture(st);
}

/// Stops capturing and returns the diagnostic so far, without help.
pub(crate) fn take_capture(st: &mut EngineState, help: Vec<String>) -> Option<Diagnostic> {
    st.diagnostics
        .capture
        .take()
        .map(|capture| capture.into_diagnostic(help))
}

/// Stops capturing and reports what was captured.
pub(crate) fn end_capture(st: &mut EngineState) {
    if let Some(diagnostic) = take_capture(st, Vec::new()) {
        emit(st, diagnostic);
    }
}

/// Reports `diagnostic` to the host.
pub(crate) fn emit(st: &mut EngineState, diagnostic: Diagnostic) {
    let cli = cli_text(&diagnostic, false);
    emit_with_cli_text(st, diagnostic, cli);
}

/// Reports a fatal stop, whose reason the command line shows below it.
pub(crate) fn emit_fatal(st: &mut EngineState, diagnostic: Diagnostic) {
    let cli = cli_text(&diagnostic, true);
    emit_with_cli_text(st, diagnostic, cli);
}

fn emit_with_cli_text(st: &mut EngineState, diagnostic: Diagnostic, cli: String) {
    let bridged = match diagnostic.kind {
        DiagnosticKind::Fatal | DiagnosticKind::Error => Some(bridge::Diagnostic::error()),
        DiagnosticKind::Warning => Some(bridge::Diagnostic::warning()),
        DiagnosticKind::Written => None,
    };
    if let Some(mut bridged) = bridged {
        diag_append(&mut bridged, &cli);
        diag_finish(bridged);
    }
    st.shipout.diagnostics_issued += 1;
    if st.config.execution_mode == EngineExecutionMode::Resident {
        st.diagnostics.issued.push(diagnostic);
    }
}

/// `file:line: message` as the command line shows it, `!message` outside any file.
fn cli_text(diagnostic: &Diagnostic, with_help: bool) -> String {
    let mut text = match (&diagnostic.file, diagnostic.line) {
        (Some(file), Some(line)) => format!("{file}:{line}: "),
        _ => String::from("!"),
    };
    text.push_str(&diagnostic.message);
    if with_help {
        for line in &diagnostic.help {
            text.push('\n');
            text.push_str(line);
        }
    }
    text
}
