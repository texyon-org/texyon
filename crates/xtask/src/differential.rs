//! Incremental typesetting against fresh sessions: after every keystroke, the
//! resident session's pages must match a session that typeset the edited text
//! from scratch, and undoing every keystroke must bring back the first pages.

use std::path::PathBuf;

use anyhow::{Context, Result, bail, ensure};
use texyon_preview_protocol::session::{Response, SessionRequest};

use crate::{
    corpus,
    project::Project,
    scenario::{self, SCALE},
    session::{Host, Session, describe},
};

/// Pixels per bp for comparing pages, enough to show any moved glyph.
const COMPARE_SCALE: f32 = 0.5;

/// Check every document and fail if any differs.
pub fn run(host: &Host, documents: &[PathBuf]) -> Result<()> {
    let mut failures = 0;
    for path in documents {
        match check(host, corpus::open(path)?) {
            Ok(keystrokes) => println!("ok {} ({keystrokes} keystrokes)", path.display()),
            Err(error) => {
                failures += 1;
                println!("FAILED {}: {error:#}", path.display());
            }
        }
    }
    if failures > 0 {
        bail!(
            "{failures} of {} documents typeset differently incrementally",
            documents.len()
        );
    }
    Ok(())
}

fn check(host: &Host, mut project: Project) -> Result<usize> {
    let original: Vec<(String, String)> = project
        .texts()
        .map(|(path, text)| (path.to_owned(), text.to_owned()))
        .collect();
    let mut resident = Session::start(host, None)?;
    resident.open(host, &project, true)?;
    let first = typeset(&mut resident, &mut project, 0)?;

    let spots = scenario::keystroke_spots(&project);
    for (number, (path, line)) in spots.iter().enumerate() {
        scenario::type_into(&mut project, path, *line);
        let page = resident.page_of(path, *line)?;
        let incremental = typeset(&mut resident, &mut project, page)?;
        let fresh = typeset_fresh(host, &project)?;
        same(&incremental, &fresh)
            .with_context(|| format!("after keystroke {} in {path}:{}", number + 1, line + 1))?;
    }

    for (path, text) in original {
        project.set(&path, text);
    }
    let undone = typeset(&mut resident, &mut project, 0)?;
    same(&undone, &first).context("after undoing every keystroke")?;
    resident.finish()?;
    Ok(spots.len())
}

/// Compile around `page`, typeset the tail, and draw every page.
fn typeset(session: &mut Session, project: &mut Project, page: usize) -> Result<Vec<Vec<u8>>> {
    let report = session.compile(project.compile(Some((page, page + 1)), Some(SCALE)))?;
    let pages = session.finish_tail()?.unwrap_or(report.page_count);
    match session.send(SessionRequest::Raster {
        pages: (0..pages).collect(),
        scale: COMPARE_SCALE,
    })? {
        Response::Rastered { pages } => {
            Ok(pages.into_iter().map(|page| page.pixels.data).collect())
        }
        other => bail!("could not draw the pages: {}", describe(&other)),
    }
}

fn typeset_fresh(host: &Host, project: &Project) -> Result<Vec<Vec<u8>>> {
    let mut fresh = project.for_new_session();
    let mut session = Session::start(host, None)?;
    session.open(host, &fresh, true)?;
    let pages = typeset(&mut session, &mut fresh, 0)?;
    session.finish()?;
    Ok(pages)
}

fn same(actual: &[Vec<u8>], expected: &[Vec<u8>]) -> Result<()> {
    ensure!(
        actual.len() == expected.len(),
        "{} pages where a fresh session has {}",
        actual.len(),
        expected.len()
    );
    if let Some(page) = actual
        .iter()
        .zip(expected)
        .position(|(actual, expected)| actual != expected)
    {
        bail!("page {} differs from a fresh session's", page + 1);
    }
    Ok(())
}
