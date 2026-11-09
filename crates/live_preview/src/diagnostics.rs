//! Editor diagnostics from what the engine reported.
//!
//! The engine reports its own errors and warnings with their file and line.
//! Text the document wrote to the transcript is classified here, since only a
//! LaTeX-style label (`Package hyperref Warning:`) marks it as a warning.

pub use texyon_engine_xetex::{Diagnostic as EngineDiagnostic, DiagnosticKind};
pub use texyon_preview_protocol::session::{Diagnostic, Severity};

/// The editor's diagnostics for what the engine reported, in order.
pub fn from_engine(reported: &[EngineDiagnostic]) -> Vec<Diagnostic> {
    reported.iter().filter_map(convert).collect()
}

fn convert(reported: &EngineDiagnostic) -> Option<Diagnostic> {
    let (severity, source, message) = match reported.kind {
        DiagnosticKind::Fatal | DiagnosticKind::Error => match Label::parse(&reported.message) {
            Some(label) if label.kind == LabelKind::Error => {
                (Severity::Error, Some(label.issuer), label.message)
            }
            _ => (Severity::Error, None, reported.message.clone()),
        },
        DiagnosticKind::Warning => (Severity::Warning, None, reported.message.clone()),
        DiagnosticKind::Written if reported.message.starts_with("No file ") => (
            Severity::Warning,
            Some("LaTeX".to_owned()),
            reported.message.clone(),
        ),
        DiagnosticKind::Written => {
            let label = Label::parse(&reported.message)?;
            let severity = match label.kind {
                LabelKind::Warning => Severity::Warning,
                LabelKind::Info => Severity::Info,
                LabelKind::Error => return None,
            };
            (severity, Some(label.issuer), label.message)
        }
    };
    Some(Diagnostic {
        severity,
        path: reported.file.as_deref().map(normalize),
        line: reported.line.and_then(|line| usize::try_from(line).ok()),
        message: trim_sentence(&message).to_owned(),
        help: reported.help.clone(),
        source,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LabelKind {
    Error,
    Warning,
    Info,
}

/// A message that names its issuer: `Package hyperref Warning: ...`.
#[derive(Debug, PartialEq, Eq)]
struct Label {
    kind: LabelKind,
    /// `hyperref`, `LaTeX`, `LaTeX Font`.
    issuer: String,
    /// The text after the label, continuation lines joined.
    message: String,
}

impl Label {
    /// The label of `text`, if its first line starts with one.
    fn parse(text: &str) -> Option<Self> {
        let text = text.replace("^^J", "\n");
        let mut lines = text.lines().skip_while(|line| line.trim().is_empty());
        let first = lines.next()?;

        let (kind, at, label) = [
            (LabelKind::Error, " Error: "),
            (LabelKind::Warning, " Warning: "),
            (LabelKind::Info, " Info: "),
        ]
        .into_iter()
        .find_map(|(kind, label)| first.find(label).map(|at| (kind, at, label)))?;
        let issuer = issuer(&first[..at])?;

        let mut message = first[at + label.len()..].trim().to_owned();
        for line in lines {
            let line = continuation(line);
            if !line.is_empty() {
                message.push(' ');
                message.push_str(line);
            }
        }
        Some(Self {
            kind,
            issuer,
            message: strip_input_line(&message).to_owned(),
        })
    }
}

/// Who a label names, if the words before `Warning:` form one.
fn issuer(prefix: &str) -> Option<String> {
    let words: Vec<&str> = prefix.split_whitespace().collect();
    match words.as_slice() {
        // "LaTeX Warning:" and "LaTeX Font Warning:".
        ["LaTeX"] | ["LaTeX", _] => Some(words.join(" ")),
        // "Package hyperref Warning:", "Class article Warning:".
        ["Package" | "Class" | "Module", name] => Some((*name).to_owned()),
        _ => None,
    }
}

/// A continuation line without its `(issuer)` prefix and alignment spaces.
fn continuation(line: &str) -> &str {
    let line = line.trim_start();
    match line.strip_prefix('(').and_then(|rest| rest.split_once(')')) {
        Some((name, rest)) if !name.contains(char::is_whitespace) => rest.trim(),
        _ => line.trim(),
    }
}

/// Drop LaTeX's ` on input line N.`, since the engine reports the line itself.
fn strip_input_line(message: &str) -> &str {
    let marker = " on input line ";
    match message.rfind(marker) {
        Some(at)
            if message[at + marker.len()..]
                .trim_end_matches('.')
                .chars()
                .all(|c| c.is_ascii_digit()) =>
        {
            &message[..at]
        }
        _ => message,
    }
}

/// The file as the editor names it.
fn normalize(name: &str) -> String {
    let name = name.strip_prefix("./").unwrap_or(name);
    let stem = name.rsplit('/').next().unwrap_or(name);
    if stem.contains('.') {
        name.to_owned()
    } else {
        format!("{name}.tex")
    }
}

fn trim_sentence(message: &str) -> &str {
    message.trim().trim_end_matches('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine(kind: DiagnosticKind, message: &str) -> EngineDiagnostic {
        EngineDiagnostic {
            kind,
            message: message.to_owned(),
            help: Vec::new(),
            file: Some("./chapters/one.tex".to_owned()),
            line: Some(12),
        }
    }

    fn written(message: &str) -> Option<Diagnostic> {
        convert(&engine(DiagnosticKind::Written, message))
    }

    #[test]
    fn a_package_warning_is_joined_and_named() {
        let found = written(
            "\nPackage hyperref Warning: Token not allowed in a PDF string (Unicode):\n\
             (hyperref)                removing `\\@ifnextchar' on input line 12.\n",
        );
        let found = found.expect("a labelled warning");
        assert_eq!(found.severity, Severity::Warning);
        assert_eq!(found.source.as_deref(), Some("hyperref"));
        assert_eq!(
            found.message,
            "Token not allowed in a PDF string (Unicode): removing `\\@ifnextchar'"
        );
        assert_eq!(found.path.as_deref(), Some("chapters/one.tex"));
        assert_eq!(found.line, Some(12));
    }

    #[test]
    fn a_latex_warning_keeps_its_page_but_not_its_input_line() {
        let found = written("LaTeX Warning: Reference `nope' on page 1 undefined on input line 5.")
            .expect("a labelled warning");
        assert_eq!(found.source.as_deref(), Some("LaTeX"));
        assert_eq!(found.message, "Reference `nope' on page 1 undefined");
    }

    #[test]
    fn a_font_warning_drops_its_continuation_label() {
        let found = written(
            "LaTeX Font Warning: Font shape `TU/lmr/m/sl' undefined\n\
             (Font)              using `TU/lmr/m/n' instead on input line 3.",
        )
        .expect("a labelled warning");
        assert_eq!(found.source.as_deref(), Some("LaTeX Font"));
        assert_eq!(
            found.message,
            "Font shape `TU/lmr/m/sl' undefined using `TU/lmr/m/n' instead"
        );
    }

    #[test]
    fn an_expl3_message_rejoins_its_own_wrapping() {
        let found = written(
            "Package fontspec Warning: Font \"X\" does not contain requested^^J\
             (fontspec)                Script \"Latin\".",
        )
        .expect("a labelled warning");
        assert_eq!(
            found.message,
            "Font \"X\" does not contain requested Script \"Latin\""
        );
    }

    #[test]
    fn a_missing_file_is_a_warning() {
        let found = written("No file main.aux.").expect("a warning");
        assert_eq!(found.severity, Severity::Warning);
        assert_eq!(found.message, "No file main.aux");
    }

    #[test]
    fn package_info_is_info() {
        let found = written("Package xcolor Info: Driver file: xetex.def on input line 2.")
            .expect("a labelled info");
        assert_eq!(found.severity, Severity::Info);
        assert_eq!(found.message, "Driver file: xetex.def");
    }

    #[test]
    fn unlabelled_or_prose_text_is_not_a_diagnostic() {
        assert_eq!(written("Document Class: article 2021/10/04 v1.4n"), None);
        assert_eq!(written("the Package docs say Warning: careful"), None);
        assert_eq!(written("(see the Warning: above)"), None);
    }

    #[test]
    fn a_package_error_is_named_without_its_label() {
        let found = convert(&engine(
            DiagnosticKind::Error,
            "Package hyperref Error: Wrong DVI mode driver option `dvipdfm'",
        ))
        .expect("an error");
        assert_eq!(found.severity, Severity::Error);
        assert_eq!(found.source.as_deref(), Some("hyperref"));
        assert_eq!(found.message, "Wrong DVI mode driver option `dvipdfm'");
    }

    #[test]
    fn an_engine_error_keeps_its_message_and_help() {
        let mut reported = engine(DiagnosticKind::Error, "Undefined control sequence");
        reported.help = vec!["The control sequence at the end of the top line".to_owned()];
        let found = convert(&reported).expect("an error");
        assert_eq!(found.message, "Undefined control sequence");
        assert_eq!(found.help, reported.help);
        assert_eq!(found.source, None);
    }

    #[test]
    fn a_name_without_an_extension_gets_tex() {
        assert_eq!(normalize("./chapters/one"), "chapters/one.tex");
        assert_eq!(normalize("main.tex"), "main.tex");
    }
}
