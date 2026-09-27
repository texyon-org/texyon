//! Editor diagnostics recovered from a TeX transcript.
//!
//! The transcript never names the file a message belongs to, so it is tracked
//! from the parentheses TeX prints when opening and closing files. Lines wrap
//! at [`WRAP_WIDTH`] without a marker and help prose contains parentheses too,
//! so neither can be handled perfectly. See [`parse`].

use std::fmt;

/// Column at which the engine wraps transcript lines (`max_print_line`).
pub const WRAP_WIDTH: usize = 79;

/// Lines searched below an error for its `l.<n>` marker.
const ERROR_LINE_LOOKAHEAD: usize = 20;

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
    /// The file it belongs to as the transcript named it, if any was open.
    pub path: Option<String>,
    /// One-based source line, when the engine named one.
    pub line: Option<usize>,
    /// What it said, with the transcript's wrapping undone.
    pub message: String,
}

/// Recover the diagnostics from a TeX transcript.
///
/// `root` is the primary input. An incremental replay resumes mid-file, so its
/// transcript never shows the primary being opened and messages outside any
/// tracked file fall back to `root`.
///
/// A line of exactly [`WRAP_WIDTH`] characters is assumed to be a wrap, and a
/// `(` in help prose is tracked like a file open. Prose parentheses balance, so
/// the file stack stays aligned.
pub fn parse(log: &str, root: Option<&str>) -> Vec<Diagnostic> {
    let lines = unwrap_lines(log);
    let root = root.map(normalize);
    let mut stack: Vec<Option<String>> = Vec::new();
    let mut found = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        if let Some(message) = line.strip_prefix("! ") {
            // LaTeX prints its advice between the error and `l.<n>`, so look a
            // bounded distance ahead, stopping at the next error.
            let line_number = lines[index + 1..]
                .iter()
                .take(ERROR_LINE_LOOKAHEAD)
                .take_while(|candidate| !candidate.starts_with("! "))
                .find_map(|candidate| source_line_marker(candidate));

            found.push(Diagnostic {
                severity: Severity::Error,
                path: current_file(&stack).or_else(|| root.clone()),
                line: line_number,
                message: trim_sentence(strip_error_prefix(message.trim_end())).to_owned(),
            });
        } else if let Some((severity, message)) = classify(line) {
            found.push(Diagnostic {
                severity,
                path: current_file(&stack).or_else(|| root.clone()),
                line: reported_line(line),
                message: trim_sentence(message).to_owned(),
            });
        }

        // After classifying, so a message belongs to the file open when it was printed.
        track_files(line, &mut stack);
    }

    found
}

/// Undo the transcript's fixed-column wrapping by joining halves directly.
fn unwrap_lines(log: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;

    for raw in log.lines() {
        let mut current = pending.take().unwrap_or_default();
        current.push_str(raw);

        // Characters, not bytes: file names and package messages may be non-ASCII.
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
                let name: String = line[at + 1..]
                    .chars()
                    .take_while(|c| !c.is_whitespace() && *c != '(' && *c != ')')
                    .collect();

                stack.push(plausible_path(&name).then(|| normalize(&name)));

                // Skip the name so a `)` inside it is not read as a close.
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
    !name.is_empty()
        && !name.starts_with('\\')
        && !name.ends_with(',')
        && !name.contains('`')
        && !name.contains('\'')
        && name.chars().all(|c| !c.is_control())
}

/// Give a bare name the `.tex` extension TeX resolved it with.
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

fn leading_number(text: &str) -> Option<usize> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// `l.12 \foo`, the line an error was raised on.
fn source_line_marker(line: &str) -> Option<usize> {
    leading_number(line.strip_prefix("l.")?)
}

/// The line a warning names. For `at lines 5--6` that is the first.
fn reported_line(line: &str) -> Option<usize> {
    ["on input line ", "at lines ", "at line "]
        .into_iter()
        .find_map(|marker| line.find(marker).map(|at| &line[at + marker.len()..]))
        .and_then(leading_number)
}

/// Sort one transcript line into a severity, if it carries one at all.
fn classify(line: &str) -> Option<(Severity, &str)> {
    const WARNING_PREFIXES: [&str; 6] = [
        "Overfull ",
        "Underfull ",
        "Tight \\",
        "Loose \\",
        "No file ",
        "No pages of output",
    ];
    if WARNING_PREFIXES
        .iter()
        .any(|prefix| line.starts_with(prefix))
    {
        return Some((Severity::Warning, line));
    }

    // Matched on the label's shape, so unknown packages are classified too.
    let (severity, at, label) = [(Severity::Warning, "Warning: "), (Severity::Info, "Info: ")]
        .into_iter()
        .find_map(|(severity, label)| line.find(label).map(|at| (severity, at, label)))?;
    is_issuer(&line[..at]).then(|| (severity, line[at + label.len()..].trim()))
}

/// Whether what precedes `Warning:` is a TeX label rather than prose.
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

/// Drop the redundant label from `! LaTeX Error: Foo.`.
fn strip_error_prefix(message: &str) -> &str {
    for prefix in ["LaTeX Error: ", "Package Error: ", "Emergency stop"] {
        if let Some(rest) = message.strip_prefix(prefix) {
            return if rest.is_empty() { message } else { rest };
        }
    }
    message
}

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
