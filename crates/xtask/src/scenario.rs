//! The editing session profiles and benchmarks replay: open, a cold compile
//! and its tail, a full pass, keystrokes compiled around their page, an export.

use std::{
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{Result, bail};
use texyon_preview_protocol::session::{Response, SessionRequest};

use crate::{
    project::Project,
    session::{Host, Session, describe},
};

/// Keystrokes per document, spread over its prose.
const KEYSTROKES: usize = 12;
/// Pixels per bp, about what an editor pane draws at.
pub const SCALE: f32 = 1.5;
/// What every keystroke types.
const TYPED: &str = " zq";

/// How long each step of one run took.
#[derive(Default)]
pub struct Timings {
    /// Pages in the document.
    pub pages: usize,
    /// The first compile of the opening viewport.
    pub cold: Duration,
    /// Typesetting the rest after it.
    pub tail: Duration,
    /// A full pass.
    pub full_pass: Duration,
    /// Each keystroke's compile around its page.
    pub keystrokes: Vec<Duration>,
    /// The tail after each keystroke.
    pub keystroke_tails: Vec<Duration>,
    /// Writing the PDF.
    pub export: Duration,
}

/// Replay the session on `project` in a fresh host. An instrumented host writes its profile into `profile`.
pub fn run(host: &Host, mut project: Project, profile: Option<&Path>) -> Result<Timings> {
    let mut session = Session::start(host, profile)?;
    session.open(host, &project, true)?;
    let mut timings = Timings::default();

    let started = Instant::now();
    let report = session.compile(project.compile(Some((0, 1)), Some(SCALE)))?;
    timings.cold = started.elapsed();
    if report.page_count == 0 {
        bail!("{} produced no pages:\n{}", project.primary, report.log);
    }
    timings.tail = timed(|| session.finish_tail().map(drop))?;

    let started = Instant::now();
    timings.pages = session.compile(SessionRequest::FullPass)?.page_count;
    timings.full_pass = started.elapsed();
    session.finish_tail()?;

    for (path, line) in keystroke_spots(&project) {
        type_into(&mut project, &path, line);
        let page = session.page_of(&path, line)?;
        let started = Instant::now();
        session.compile(project.compile(Some((page, page + 1)), Some(SCALE)))?;
        timings.keystrokes.push(started.elapsed());
        timings
            .keystroke_tails
            .push(timed(|| session.finish_tail().map(drop))?);
    }

    let started = Instant::now();
    match session.send(SessionRequest::Export { now: None })? {
        Response::Exported(_) => timings.export = started.elapsed(),
        other => bail!("{} did not export: {}", project.primary, describe(&other)),
    }
    session.finish()?;
    Ok(timings)
}

fn timed(step: impl FnOnce() -> Result<()>) -> Result<Duration> {
    let started = Instant::now();
    step()?;
    Ok(started.elapsed())
}

/// Lines of prose, as (path, zero-based line), spread evenly over the project.
pub fn keystroke_spots(project: &Project) -> Vec<(String, usize)> {
    let prose: Vec<(String, usize)> = project
        .texts()
        .flat_map(|(path, text)| {
            // Only the body: a keystroke in the preamble would rename a font or a package.
            let body = text
                .lines()
                .position(|line| line.contains(r"\begin{document}"))
                .map_or(0, |line| line + 1);
            text.lines()
                .enumerate()
                .skip(body)
                .filter(|(_, line)| {
                    line.len() > 40
                        && line
                            .trim_start()
                            .starts_with(|c: char| c.is_ascii_alphabetic())
                })
                .map(move |(number, _)| (path.to_owned(), number))
        })
        .collect();
    let count = KEYSTROKES.min(prose.len());
    (0..count)
        .map(|k| prose[k * prose.len() / count].clone())
        .collect()
}

/// Type [`TYPED`] into the middle of a line.
pub fn type_into(project: &mut Project, path: &str, line: usize) {
    let Some(text) = project
        .texts()
        .find(|(held, _)| *held == path)
        .map(|(_, text)| text.to_owned())
    else {
        return;
    };
    let edited: Vec<String> = text
        .split('\n')
        .enumerate()
        .map(|(number, content)| {
            if number != line {
                return content.to_owned();
            }
            let middle = content
                .char_indices()
                .nth(content.chars().count() / 2)
                .map_or(content.len(), |(at, _)| at);
            format!("{}{TYPED}{}", &content[..middle], &content[middle..])
        })
        .collect();
    project.set(path, edited.join("\n"));
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{keystroke_spots, type_into};
    use crate::project::Project;

    #[test]
    fn keystrokes_land_in_body_prose_only() {
        let mut project = Project::new(Path::new("."), "main.tex");
        project.set(
            "main.tex",
            "\\documentclass{article}\n\
             \\setmainfont{LibertinusSerif}[\n\
               BoldFont = LibertinusSerif-Bold, ItalicFont = LibertinusSerif-Italic]\n\
             \\begin{document}\n\
             \\section{A heading that is long enough to count as prose}\n\
             The only line of body prose here, comfortably over forty characters.\n\
             \\end{document}\n",
        );
        assert_eq!(keystroke_spots(&project), [("main.tex".to_owned(), 5)]);

        type_into(&mut project, "main.tex", 5);
        let (_, text) = project.texts().next().unwrap();
        assert_eq!(
            text.lines().nth(5),
            Some("The only line of body prose here,  zqcomfortably over forty characters.")
        );
    }
}
