//! End-to-end test of a built host: typesetting, incremental edits, export,
//! SyncTeX both ways, diagnostics, links, the command list and raster pages.

use std::fs;

use anyhow::{Context, Result, ensure};
use texyon_preview_protocol::session::{
    CompileReport, InlineBytes, Response, SessionRequest, Severity,
};

use crate::{
    cargo,
    project::Project,
    session::{Host, Session, describe},
};

const MAIN: &str = r"\documentclass{article}
\usepackage{amsmath}
\begin{document}
\section{Test}
\input{chapters/one}
\begin{equation}
  \int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}
\end{equation}
\end{document}
";
const CHILD: &str = "chapters/one.tex";

fn child(extra: &str) -> String {
    format!("Hello from TeXyoN.{extra}\n")
}

/// Run every check against `host`.
pub fn run(host: &Host) -> Result<()> {
    let root = cargo::target_dir().join("session-test");
    fs::create_dir_all(&root)?;
    let mut project = Project::new(&root, "main.tex");
    project.set("main.tex", MAIN);
    project.set(CHILD, child(""));

    let mut session = Session::start(host, None)?;
    session.open(host, &project, false)?;
    edits_and_export(&mut session, &mut project)?;
    synctex(&mut session)?;
    diagnostics(&mut session, &mut project)?;
    links(&mut session, &mut project)?;
    commands(&mut session, &mut project)?;
    pictures(&mut session, &mut project)?;
    session.finish()?;

    let mut project = Project::new(&root, "main.tex");
    project.set("main.tex", MAIN);
    project.set(CHILD, child(""));
    let mut session = Session::start(host, None)?;
    session.open(host, &project, true)?;
    raster(&mut session, &mut project)?;
    session.finish()?;

    println!("all session checks passed");
    Ok(())
}

fn compile(session: &mut Session, project: &mut Project) -> Result<CompileReport<InlineBytes>> {
    session.compile(project.compile(None, None))
}

fn edits_and_export(session: &mut Session, project: &mut Project) -> Result<()> {
    let cold = compile(session, project)?;
    ensure!(
        cold.page_count > 0,
        "no pages, outcome {:?}:\n{}",
        cold.outcome,
        cold.log
    );
    ensure!(
        cold.pages.len() == cold.changed.len(),
        "the drawn pages differ from the changed ones"
    );
    for page in &cold.pages {
        ensure!(
            contains(&page.svg.data, b"<svg"),
            "page {} is not SVG",
            page.index
        );
    }

    project.set(CHILD, child(" Edited."));
    let warm = compile(session, project)?;
    ensure!(
        warm.changed == [0],
        "an edit on page 1 changed {:?}",
        warm.changed
    );

    let unchanged = compile(session, project)?;
    ensure!(
        unchanged.changed.is_empty(),
        "an unchanged document redrew {:?}",
        unchanged.changed
    );

    let pdf = match session.send(SessionRequest::Export { now: None })? {
        Response::Exported(pdf) => pdf.data,
        other => anyhow::bail!("export failed: {}", describe(&other)),
    };
    ensure!(pdf.starts_with(b"%PDF-"), "the export has no PDF header");
    ensure!(
        contains(&pdf[pdf.len().saturating_sub(32)..], b"%%EOF"),
        "the export has no trailer"
    );

    // xdvipdfmx borrows pdf_io's file registry from TeX, a run after an export proves it came back.
    project.set(CHILD, child(" Edited again."));
    compile(session, project)?;
    Ok(())
}

fn synctex(session: &mut Session) -> Result<()> {
    let (rect, boxes) = match session.send(SessionRequest::ForwardSearch {
        path: CHILD.into(),
        line: 1,
    })? {
        Response::Located {
            rect: Some(rect),
            boxes,
            ..
        } => (rect, boxes),
        other => anyhow::bail!(
            "forward search found nothing for {CHILD}:1: {}",
            describe(&other)
        ),
    };
    ensure!(
        !boxes.is_empty()
            && boxes
                .iter()
                .all(|found| found.page_index == rect.page_index),
        "forward search flashed no boxes on the page it scrolled to"
    );

    // Inside the line, where a reader clicks.
    let source = match session.send(SessionRequest::BackwardSearch {
        page_index: rect.page_index,
        h_bp: rect.h_bp + rect.width_bp / 2.0,
        v_bp: rect.v_bp,
    })? {
        Response::Located {
            source: Some(source),
            ..
        } => source,
        other => anyhow::bail!("backward search found nothing: {}", describe(&other)),
    };
    ensure!(
        source.path == CHILD,
        "backward search came back as {}",
        source.path
    );
    Ok(())
}

fn diagnostics(session: &mut Session, project: &mut Project) -> Result<()> {
    project.set(CHILD, "Fine.\n\n\\undefinedcommandhere\n");
    let report = compile(session, project)?;
    let error = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == Severity::Error)
        .context("no error for an undefined command")?;
    ensure!(
        error.path.as_deref() == Some(CHILD) && error.line == Some(3),
        "the error is attributed to {}:{}, not {CHILD}:3",
        error.path.as_deref().unwrap_or("no file"),
        error
            .line
            .map_or_else(|| "no line".into(), |line| line.to_string())
    );
    Ok(())
}

fn links(session: &mut Session, project: &mut Project) -> Result<()> {
    project.retain(&["main.tex"]);
    project.set(
        "main.tex",
        r"\documentclass{article}
\usepackage{hyperref}
\begin{document}
\hypertarget{spot}{Here}. Jump \hyperlink{spot}{back} or visit
\url{https://example.com/a(b)}.
\end{document}
",
    );
    let report = compile(session, project)?;
    let overlay = report
        .page_overlays
        .first()
        .context("no overlay for page 1")?;

    let external = overlay
        .links
        .iter()
        .find(|link| link.uri.is_some())
        .context("no external link")?;
    // A reader that stopped at the first `)` would link somewhere else.
    ensure!(
        external.uri.as_deref() == Some("https://example.com/a(b)"),
        "the external link is {:?}",
        external.uri
    );
    ensure!(
        external.right > external.left && external.bottom > external.top,
        "the external link has no area"
    );

    let internal = overlay
        .links
        .iter()
        .find(|link| link.destination.is_some())
        .context("no internal link")?;
    ensure!(
        internal.destination.as_deref() == Some("spot"),
        "the internal link points at {:?}",
        internal.destination
    );
    ensure!(
        overlay
            .destinations
            .iter()
            .any(|destination| destination.name == "spot"),
        "the link's destination was not recorded"
    );
    Ok(())
}

fn commands(session: &mut Session, project: &mut Project) -> Result<()> {
    project.set(
        "main.tex",
        r"\documentclass{article}
\usepackage{amsmath}
\newcommand{\vect}[1]{\mathbf{#1}}
\begin{document}
$\vect{x}$
\end{document}
",
    );
    compile(session, project)?;
    let commands = match session.send(SessionRequest::Commands)? {
        Response::Commands { commands } => commands,
        other => anyhow::bail!("no command list: {}", describe(&other)),
    };
    let arity = |name: &str| {
        commands
            .iter()
            .find(|command| command.name == name)
            .map(|command| command.arity)
    };
    ensure!(
        arity("vect") == Some(1),
        "the document's \\vect has arity {:?}",
        arity("vect")
    );
    // \frac is robust, its parameters live behind a wrapper the list must follow.
    ensure!(
        arity("frac") == Some(2),
        "\\frac has arity {:?}",
        arity("frac")
    );
    ensure!(
        !commands
            .iter()
            .any(|command| command.name.ends_with(' ') || command.name.starts_with('\\')),
        "wrapper names leaked into the command list"
    );
    Ok(())
}

fn raster(session: &mut Session, project: &mut Project) -> Result<()> {
    let report = compile(session, project)?;
    ensure!(
        report.pages.is_empty() && report.rasters.is_empty(),
        "a raster session drew pages nobody asked for"
    );
    let [width_bp, height_bp] = *report.page_sizes.first().context("no page size")?;

    for scale in [2.0_f32, 1.0] {
        let pages = match session.send(SessionRequest::Raster {
            pages: vec![0],
            scale,
        })? {
            Response::Rastered { pages } => pages,
            other => anyhow::bail!("raster at {scale} failed: {}", describe(&other)),
        };
        let page = pages.first().context("no raster page")?;
        let pixels = &page.pixels.data;
        let width = u32::from_le_bytes(pixels[0..4].try_into()?);
        let height = u32::from_le_bytes(pixels[4..8].try_into()?);
        ensure!(
            f64::from(width) == (f64::from(width_bp) * f64::from(scale)).ceil()
                && f64::from(height) == (f64::from(height_bp) * f64::from(scale)).ceil(),
            "raster at {scale} is {width}x{height} for a {width_bp}x{height_bp} bp page"
        );
        let rgba = pixels[8..].chunks_exact(4);
        ensure!(
            rgba.clone().all(|pixel| pixel[3] == 255),
            "a raster page is not opaque"
        );
        ensure!(
            rgba.clone().any(|pixel| pixel[0] < 128),
            "the raster at {scale} is blank"
        );
    }

    project.set(CHILD, child(" Rastered."));
    let edited = session.compile(project.compile(Some((0, 0)), Some(2.0)))?;
    let drawn: Vec<usize> = edited.rasters.iter().map(|page| page.index).collect();
    ensure!(drawn == [0], "an edit in view drew pages {drawn:?}");
    Ok(())
}

/// A one-page PDF of `width` by `height` bp, binary like a real one, so the host serves it from disk.
fn pdf_page(width: u32, height: u32) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] >>"),
    ];
    let mut pdf = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in (1..).zip(&objects) {
        offsets.push(pdf.len());
        pdf.extend(format!("{number} 0 obj\n{body}\nendobj\n").bytes());
    }
    let xref = pdf.len();
    let size = objects.len() + 1;
    pdf.extend(format!("xref\n0 {size}\n0000000000 65535 f \n").bytes());
    for offset in offsets {
        pdf.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    pdf.extend(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").bytes(),
    );
    pdf
}

fn pictures(session: &mut Session, project: &mut Project) -> Result<()> {
    project.retain(&["main.tex"]);
    // A PDF parsed in one operation must not stand in for its edited file in the next.
    for (width, label, expected) in [
        (100, "first", "[first 100.375pt]"),
        (200, "second", "[second 200.75pt]"),
    ] {
        fs::write(project.root.join("pic.pdf"), pdf_page(width, 50))?;
        project.set(
            "main.tex",
            format!(
                "\\documentclass{{article}}\n\\begin{{document}}\n\\setbox0\\hbox{{\\XeTeXpdffile \"pic.pdf\" }}\\typeout{{[{label} \\the\\wd0]}}\nText.\n\\end{{document}}\n"
            ),
        );
        let report = compile(session, project)?;
        ensure!(
            report.log.contains(expected),
            "a {width} bp picture did not report {expected}:\n{}",
            report.log
        );
    }
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
