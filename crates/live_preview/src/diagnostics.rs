//! Turning a TeX transcript into diagnostics an editor can put in a gutter.
//!
//! TeX reports trouble to a log written for a person reading a terminal in
//! 1982, and an editor needs `(file, line, severity, message)`. Nothing in the
//! transcript states the file a message belongs to: it has to be recovered from
//! the parentheses TeX prints when it opens and closes one, which is why this
//! exists at all and why it is the part most worth being careful about.
//!
//! Two things make that harder than balancing brackets. The transcript wraps at
//! [`WRAP_WIDTH`] columns, mid-token and without a continuation marker, so a
//! message can be split anywhere. And the parentheses are not only file
//! delimiters — TeX's own help prose contains them too. Both are handled below,
//! and neither can be handled perfectly; see [`parse`].

use std::fmt;

/// Column at which the engine wraps transcript lines.
///
/// This is `max_print_line`, which the engine fixes at 79. A line of exactly
/// this length is a wrap rather than a sentence that happened to end there —
/// see [`unwrap_lines`] for what that costs.
pub const WRAP_WIDTH: usize = 79;

/// How much a diagnostic matters.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Severity {
    /// TeX could not do what was asked and recovered as best it could.
    Error,
    /// Something is suspect but the output exists.
    Warning,
    /// Chatter worth surfacing only on request.
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        })
    }
}

/// One thing the engine said about one place in the source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// How much it matters.
    pub severity: Severity,
    /// The file it belongs to, as the transcript named it.
    ///
    /// Absent when the message arrived outside any file — before the document
    /// was opened, or after everything closed.
    pub path: Option<String>,
    /// One-based source line, when the engine named one.
    pub line: Option<usize>,
    /// What it said, with the transcript's wrapping undone.
    pub message: String,
}

/// Recover the diagnostics from a TeX transcript.
///
/// `root` is the file the transcript begins inside — the primary input. It is
/// needed rather than merely convenient: an incremental replay resumes from a
/// checkpoint partway through the run, so its transcript starts in the middle
/// of the primary file and the `(main.tex` that opened it was printed long
/// before. Without a floor to fall back to, the stack is empty at the point
/// most errors are reported and every one of them comes out with no file —
/// which, for an editor, means nothing appears in the gutter at all.
///
/// # What this cannot do
///
/// Two ambiguities are inherent to the format rather than to this
/// implementation, and both are resolved the way every TeX log parser resolves
/// them:
///
/// A line of exactly [`WRAP_WIDTH`] characters is assumed to be a wrap. A
/// message that genuinely ends on that column is joined to the next line.
///
/// A `(` in TeX's help prose is indistinguishable from one opening a file. Such
/// parentheses are balanced within the prose, so tracking every `(` and `)` —
/// and remembering which ones named a plausible path — keeps the file stack
/// aligned even though it briefly contains nonsense.
pub fn parse(log: &str, root: Option<&str>) -> Vec<Diagnostic> {
    let lines = unwrap_lines(log);

    // The floor. A `)` closing a file opened before the transcript began pops
    // it, so it is restored rather than merely pushed once — otherwise the
    // first stray close would lose the root for the rest of the run.
    let root = root.map(normalize);
    let mut stack: Vec<Option<String>> = Vec::new();
    let mut found = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = &lines[index];

        // The engine reports an error, then several lines of advice, then the
        // line it was on, then more advice. The advice is not a diagnostic, and
        // the `l.<n>` marker is further down than it looks: LaTeX puts its
        // "See the LaTeX manual" and "Type H <return>" between them, so a
        // lookahead of a few lines finds nothing and reports no line at all.
        if let Some(message) = line.strip_prefix("! ") {
            let message = strip_error_prefix(message.trim_end());
            let line_number = lines[index + 1..]
                .iter()
                // Bounded, and stopping at the next error so one cannot take
                // the line belonging to another.
                .take(20)
                .take_while(|candidate| !candidate.starts_with("! "))
                .find_map(|candidate| source_line_marker(candidate));

            found.push(Diagnostic {
                severity: Severity::Error,
                path: current_file(&stack).or_else(|| root.clone()),
                line: line_number,
                message: trim_sentence(message).to_owned(),
            });
        } else if let Some((severity, message)) = classify(line) {
            found.push(Diagnostic {
                severity,
                path: current_file(&stack).or_else(|| root.clone()),
                line: reported_line(line),
                message: trim_sentence(message).to_owned(),
            });
        }

        // The stack is advanced after classifying, so a message is attributed
        // to the file that was open when it was printed rather than to one the
        // same line happens to open afterwards.
        track_files(line, &mut stack);
        index += 1;
    }

    found
}

/// Undo the transcript's fixed-column wrapping.
///
/// A wrapped line is exactly [`WRAP_WIDTH`] characters and continues on the
/// next one, with no marker and no regard for word boundaries — the join is
/// therefore direct, with nothing inserted between the halves.
fn unwrap_lines(log: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;

    for raw in log.lines() {
        let mut current = pending.take().unwrap_or_default();
        current.push_str(raw);

        // Width is in characters, not bytes: a transcript carries file names
        // and package messages that are not ASCII.
        if raw.chars().count() == WRAP_WIDTH {
            pending = Some(current);
        } else {
            lines.push(current);
        }
    }

    if let Some(last) = pending {
        lines.push(last);
    }

    lines
}

/// Follow the parentheses that open and close files.
fn track_files(line: &str, stack: &mut Vec<Option<String>>) {
    let mut chars = line.char_indices().peekable();

    while let Some((at, character)) = chars.next() {
        match character {
            '(' => {
                let rest = &line[at + 1..];
                let name: String = rest
                    .chars()
                    .take_while(|c| !c.is_whitespace() && *c != '(' && *c != ')')
                    .collect();

                stack.push(plausible_path(&name).then(|| normalize(&name)));

                // Skip what was consumed as a name so a `)` inside it is not
                // read as a close.
                for _ in 0..name.chars().count() {
                    chars.next();
                }
            }
            ')' => {
                stack.pop();
            }
            _ => {}
        }
    }
}

/// Whether a token after `(` reads like a file rather than like prose.
fn plausible_path(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // TeX's advice contains parenthesised asides — "(e.g., `\hobx')" — whose
    // first token is punctuated in ways a file name is not.
    !name.starts_with('\\')
        && !name.ends_with(',')
        && !name.contains('`')
        && !name.contains('\'')
        && name.chars().all(|c| !c.is_control())
}

/// Give a bare name the extension TeX resolved it with.
///
/// The transcript prints `\input{chapters/one}` as `(chapters/one`, but the
/// file the caller knows about is `chapters/one.tex`.
fn normalize(name: &str) -> String {
    let stem = name.rsplit('/').next().unwrap_or(name);
    if stem.contains('.') {
        name.to_owned()
    } else {
        format!("{name}.tex")
    }
}

/// The innermost open file, ignoring frames that were only prose.
fn current_file(stack: &[Option<String>]) -> Option<String> {
    stack.iter().rev().find_map(|frame| frame.clone())
}

/// `l.12 \foo` — the line an error was raised on.
fn source_line_marker(line: &str) -> Option<usize> {
    let rest = line.strip_prefix("l.")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The line a warning names, in either spelling TeX uses for it.
fn reported_line(line: &str) -> Option<usize> {
    if let Some(at) = line.find("on input line ") {
        let rest = &line[at + "on input line ".len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        return digits.parse().ok();
    }

    // "at lines 5--6" names the range a box spans; the first is where a reader
    // would want the cursor.
    for marker in ["at lines ", "at line "] {
        if let Some(at) = line.find(marker) {
            let rest = &line[at + marker.len()..];
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            return digits.parse().ok();
        }
    }

    None
}

/// Sort one transcript line into a severity, if it carries one at all.
fn classify(line: &str) -> Option<(Severity, &str)> {
    // Boxes that did not fit. These matter enough to surface and are spelled
    // without any "Warning" of their own.
    for prefix in ["Overfull ", "Underfull ", "Tight \\", "Loose \\"] {
        if line.starts_with(prefix) {
            return Some((Severity::Warning, line));
        }
    }

    if line.starts_with("No file ") || line.starts_with("No pages of output") {
        return Some((Severity::Warning, line));
    }

    // "LaTeX Warning:", "Package hyperref Warning:", "Class article Warning:",
    // and the Info spellings of each. Matched on the label's shape rather than
    // on a list of issuers, so a package this has never heard of is classified
    // like any other.
    let (severity, marker) = if let Some(at) = line.find("Warning: ") {
        (Severity::Warning, at)
    } else if let Some(at) = line.find("Info: ") {
        (Severity::Info, at)
    } else {
        return None;
    };

    if !is_issuer(&line[..marker]) {
        return None;
    }

    let body = marker
        + if severity == Severity::Warning {
            "Warning: ".len()
        } else {
            "Info: ".len()
        };
    Some((severity, line[body..].trim()))
}

/// Whether what precedes `Warning:` names something entitled to warn.
///
/// TeX's labels are at most a handful of words and always begin by saying what
/// kind of thing is speaking. Prose that merely contains the word does not, and
/// this is the whole of what separates the two.
fn is_issuer(prefix: &str) -> bool {
    let words: Vec<&str> = prefix.split_whitespace().collect();

    match words.split_first() {
        // "LaTeX Warning:" and "LaTeX Font Warning:".
        Some((&"LaTeX", rest)) => rest.len() <= 1,
        // "Package hyperref Warning:", "Class article Warning:".
        Some((&"Package" | &"Class" | &"Module", rest)) => rest.len() == 1,
        _ => false,
    }
}

/// `! LaTeX Error: Foo.` says the same thing as `! Foo.` twice.
fn strip_error_prefix(message: &str) -> &str {
    for prefix in ["LaTeX Error: ", "Package Error: ", "Emergency stop"] {
        if let Some(rest) = message.strip_prefix(prefix) {
            return if rest.is_empty() { message } else { rest };
        }
    }
    message
}

/// Drop a trailing full stop, which every message has and no gutter wants.
fn trim_sentence(message: &str) -> &str {
    message.trim().trim_end_matches('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Excerpted verbatim from a transcript this engine produced, wrapping and
    /// all — the `Trying to load font information` line is exactly 79 columns
    /// and continues with a lone `.`.
    const REAL_LOG: &str = "**\n\
(main.tex\n\
LaTeX2e <2021-11-15> patch level 1\n\
L3 programming layer <2022-02-24> (article.cls\n\
Document Class: article 2021/10/04 v1.4n Standard LaTeX document class\n\
(size10.clo\n\
File: size10.clo 2021/10/04 v1.4n Standard LaTeX file (size option)\n\
)\n\
)\n\
No file main.aux.\n\
LaTeX Font Info:    Trying to load font information for TS1+cmr on input line 2\n\
.\n\
 (chapters/one\n\
! Undefined control sequence.\n\
l.3 \\undefinedcommandhere\n\
                         \n\
The control sequence at the end of the top line\n\
of your error message was never \\def'ed. If you have\n\
misspelled it (e.g., `\\hobx'), type `I' and the correct\n\
spelling (e.g., `I\\hbox'). Otherwise just continue,\n\
and I'll forget about whatever was undefined.\n\
\n\
)\n\
\n\
LaTeX Warning: Reference `nope' on page 1 undefined on input line 5.\n\
\n\
\n\
Overfull \\hbox (261.77501pt too wide) in paragraph at lines 5--6\n\
[]\\TU/lmr/m/n/10 Another line\n\
 []\n\
\n\
[1\n\
\n\
] (main.aux)\n\
\n\
LaTeX Warning: There were undefined references.\n\
\n\
 )\n\
Output written on main.xdv (1 page, 2280 bytes).\n";

    /// A LaTeX error, spelled the way LaTeX actually spells one.
    ///
    /// The `l.15` is six lines below the `!`, with the standard advice in
    /// between. Anything looking only at the next line or two finds no line
    /// number and reports the error against the whole file.
    const LATEX_ERROR: &str = "(main.tex\n\
! LaTeX Error: \\mathbf allowed only in math mode.\n\
\n\
See the LaTeX manual or LaTeX Companion for explanation.\n\
Type  H <return>  for immediate help.\n\
 ...                                              \n\
                                                  \n\
l.15 \\vect{x}\n\
             \n\
You're in trouble here.  Try typing  <return>  to proceed.\n";

    #[test]
    fn a_latex_error_finds_the_line_below_its_advice() {
        let found = parse(LATEX_ERROR, None);
        let error = found
            .iter()
            .find(|d| d.severity == Severity::Error)
            .expect("the transcript contains one error");

        assert_eq!(error.message, "\\mathbf allowed only in math mode");
        assert_eq!(error.line, Some(15));
        assert_eq!(error.path.as_deref(), Some("main.tex"));
    }

    #[test]
    fn one_error_does_not_take_the_line_belonging_to_the_next() {
        let found = parse(
            "(main.tex\n! First problem.\n! Second problem.\nl.9 \\foo\n",
            None,
        );

        let lines: Vec<_> = found.iter().map(|d| d.line).collect();
        assert_eq!(lines, vec![None, Some(9)]);
    }

    #[test]
    fn an_error_is_attributed_to_the_child_that_raised_it() {
        let found = parse(REAL_LOG, None);
        let error = found
            .iter()
            .find(|d| d.severity == Severity::Error)
            .expect("the transcript contains one error");

        assert_eq!(error.message, "Undefined control sequence");
        // The file stack, not the message, is what knows this.
        assert_eq!(error.path.as_deref(), Some("chapters/one.tex"));
        assert_eq!(error.line, Some(3));
    }

    #[test]
    fn a_reference_warning_keeps_its_input_line() {
        let found = parse(REAL_LOG, None);
        let warning = found
            .iter()
            .find(|d| d.message.starts_with("Reference"))
            .expect("the transcript warns about an undefined reference");

        assert_eq!(warning.severity, Severity::Warning);
        assert_eq!(warning.line, Some(5));
        // Raised after the child closed, so it belongs to the primary.
        assert_eq!(warning.path.as_deref(), Some("main.tex"));
    }

    #[test]
    fn an_overfull_box_reports_the_first_line_of_its_range() {
        let found = parse(REAL_LOG, None);
        let box_warning = found
            .iter()
            .find(|d| d.message.starts_with("Overfull"))
            .expect("the transcript reports an overfull box");

        assert_eq!(box_warning.severity, Severity::Warning);
        assert_eq!(box_warning.line, Some(5));
    }

    #[test]
    fn helpful_prose_does_not_unbalance_the_file_stack() {
        // "(e.g., `\hobx')" and its twin sit between the error and the `)` that
        // closes the child. If either were taken for a file, everything after
        // would be attributed one level too deep.
        let found = parse(REAL_LOG, None);
        let last = found.last().expect("the transcript ends with a warning");
        assert_eq!(last.message, "There were undefined references");
        assert_eq!(last.path.as_deref(), Some("main.tex"));
    }

    #[test]
    fn a_wrapped_line_is_rejoined_before_it_is_read() {
        let found = parse(REAL_LOG, None);
        let info = found
            .iter()
            .find(|d| d.message.starts_with("Trying to load"))
            .expect("the wrapped font-info line survives unwrapping");

        // The line number is on the far side of the wrap, so finding it at all
        // is the assertion.
        assert_eq!(info.severity, Severity::Info);
        assert_eq!(info.line, Some(2));
    }

    #[test]
    fn a_missing_file_is_a_warning() {
        let found = parse(REAL_LOG, None);
        assert!(found
            .iter()
            .any(|d| d.severity == Severity::Warning && d.message == "No file main.aux"));
    }

    #[test]
    fn a_latex_error_is_not_labelled_twice() {
        let found = parse(
            "(main.tex\n! LaTeX Error: \\begin{document} ended by \\end{foo}.\nl.9 \\end{foo}\n",
            None,
        );
        assert_eq!(found[0].message, "\\begin{document} ended by \\end{foo}");
        assert_eq!(found[0].line, Some(9));
    }

    #[test]
    fn a_package_warning_is_classified_without_knowing_the_package() {
        let found = parse(
            "(main.tex\nPackage inventedbyme Warning: Something is off on input line 12.\n",
            None,
        );
        assert_eq!(found[0].severity, Severity::Warning);
        assert_eq!(found[0].message, "Something is off on input line 12");
        assert_eq!(found[0].line, Some(12));
    }

    #[test]
    fn quoted_prose_is_not_mistaken_for_a_label() {
        // The word appears mid-sentence rather than as a label, so this line is
        // prose about a warning, not a warning.
        let found = parse(
            "(main.tex\nThis text mentions a Warning: but is not one.\n",
            None,
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_transcript_with_nothing_to_say_yields_nothing() {
        assert!(parse("", None).is_empty());
        assert!(parse("**\n(main.tex\n)\n", None).is_empty());
    }

    #[test]
    fn a_replay_starting_mid_file_still_names_the_file() {
        // What an incremental replay actually hands over: it resumes from a
        // checkpoint, so the `(main.tex` that opened the primary was printed
        // before any of this and the stack is empty by the time it matters.
        let found = parse(
            "(l3backend-xetex.def\nFile: l3backend-xetex.def\n)\n\
             ! LaTeX Error: \\mathbf allowed only in math mode.\n\
             \n\
             See the LaTeX manual or LaTeX Companion for explanation.\n\
             Type  H <return>  for immediate help.\n\
              ...                                              \n\
                                                               \n\
             l.12 Here is \\vect{x}\n",
            Some("main.tex"),
        );

        assert_eq!(found[0].path.as_deref(), Some("main.tex"));
        assert_eq!(found[0].line, Some(12));
    }

    #[test]
    fn a_file_opened_inside_the_replay_still_wins_over_the_root() {
        let found = parse(
            "(chapters/method.tex\nOverfull \\hbox (1.0pt too wide) in paragraph at lines 9--9\n",
            Some("main.tex"),
        );

        assert_eq!(found[0].path.as_deref(), Some("chapters/method.tex"));
    }
}
