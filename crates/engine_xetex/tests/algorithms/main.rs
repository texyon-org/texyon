//! Tests of TeX's core algorithms: small documents run through INITEX, with
//! expectations read back from what they `\show` in the log.

mod arithmetic;
mod errors;
mod line_break;
mod math;
mod packaging;
mod page_builder;
mod scanning;

use std::collections::HashSet;
use std::io::{Cursor, Read, SeekFrom};
use std::path::Path;

use texyon_bridge_core::{CoreBridgeLauncher, MinimalDriver};
use texyon_engine_xetex::TexEngine;
use texyon_errors::{Error, Result};
use texyon_io_base::filesystem::FilesystemIo;
use texyon_io_base::stack::IoStack;
use texyon_io_base::{InputFeatures, InputHandle, InputOrigin, IoProvider, MemoryIo, OpenResult};
use texyon_status_base::{MessageKind, StatusBackend};

/// INITEX has no macros, so the category codes plain TeX sets come first.
const PRELUDE: &str = r"\catcode`\{=1 \catcode`\}=2 \catcode`\$=3 \catcode`\&=4
\catcode`\#=6 \catcode`\^=7 \catcode`\_=8 \catcode`\^^I=10
\scrollmode \showboxdepth=100 \showboxbreadth=100 \tracingonline=1
\hbadness=10000 \vbadness=10000 \hfuzz=16383pt \vfuzz=16383pt
";

/// Loads the Computer Modern fonts that math needs in every size.
const MATH_FONTS: &str = r#"\font\rm=cmr10 \font\mi=cmmi10 \font\sy=cmsy10 \font\ex=cmex10
\textfont0=\rm \scriptfont0=\rm \scriptscriptfont0=\rm
\textfont1=\mi \scriptfont1=\mi \scriptscriptfont1=\mi
\textfont2=\sy \scriptfont2=\sy \scriptscriptfont2=\sy
\textfont3=\ex \scriptfont3=\ex \scriptscriptfont3=\ex
\mathcode`x="7178 \mathcode`y="7179 \mathcode`+="202B
\delcode`(="028300 \delcode`)="029301 \rm
"#;

/// The primary input, served from memory.
struct Source(Vec<u8>);

struct SourceReader(Cursor<Vec<u8>>);

impl Read for SourceReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl InputFeatures for SourceReader {
    fn get_size(&mut self) -> Result<usize> {
        Ok(self.0.get_ref().len())
    }

    fn try_seek(&mut self, pos: SeekFrom) -> Result<u64> {
        Ok(std::io::Seek::seek(&mut self.0, pos)?)
    }
}

impl IoProvider for Source {
    fn input_open_primary(&mut self, _: &mut dyn StatusBackend) -> OpenResult<InputHandle> {
        OpenResult::Ok(InputHandle::new_read_only(
            "texput.tex",
            SourceReader(Cursor::new(self.0.clone())),
            InputOrigin::Other,
        ))
    }
}

/// Keeps the error reports a run sends to its status backend.
#[derive(Default)]
struct Reports(Vec<String>);

impl StatusBackend for Reports {
    fn report(&mut self, kind: MessageKind, args: std::fmt::Arguments, _: Option<&Error>) {
        if kind == MessageKind::Error {
            self.0.push(args.to_string());
        }
    }

    fn dump_error_logs(&mut self, _: &[u8]) {}
}

/// The log of one INITEX run.
struct Run {
    log: String,
    /// The error reports, such as the editor's diagnostics.
    reports: Vec<String>,
}

/// Runs `body` after the prelude, then `\end`.
fn run(body: &str) -> Run {
    run_with(body, false)
}

/// Runs `body` like [`run`], stopping at the first error when `halt_on_error` is set.
fn run_with(body: &str, halt_on_error: bool) -> Run {
    let mut outputs = MemoryIo::new(true);
    let source = format!("{PRELUDE}{body}\n\\end\n");
    let reports = run_engine(&mut outputs, source, true, halt_on_error);
    read_log(&outputs, reports)
}

/// Dumps a format from the prelude and `setup`, then runs `body` on that format.
fn run_on_format(setup: &str, body: &str) -> Run {
    let mut outputs = MemoryIo::new(true);
    let dump = format!("{PRELUDE}{setup}\n\\dump\n");
    run_engine(&mut outputs, dump, true, false);
    let reports = run_engine(&mut outputs, format!("{body}\n\\end\n"), false, false);
    read_log(&outputs, reports)
}

/// Runs `source` as `texput.tex`, with `texput.fmt` as the format, returning the error reports.
fn run_engine(
    outputs: &mut MemoryIo,
    source: String,
    initex: bool,
    halt_on_error: bool,
) -> Vec<String> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/assets");
    let mut fonts = FilesystemIo::new(&assets, false, false, HashSet::new());
    let mut primary = Source(source.into_bytes());
    let mut status = Reports::default();
    {
        let io = IoStack::new(vec![
            Box::new(outputs),
            Box::new(&mut primary),
            Box::new(&mut fonts),
        ]);
        let mut hooks = MinimalDriver::new(io);
        let mut launcher = CoreBridgeLauncher::new(&mut hooks, &mut status);
        let _ = TexEngine::default()
            .halt_on_error_mode(halt_on_error)
            .initex_mode(initex)
            .deterministic_mode(true)
            .process(&mut launcher, "texput.fmt", "texput.tex");
    }
    status.0
}

fn read_log(outputs: &MemoryIo, reports: Vec<String>) -> Run {
    let files = outputs.files.borrow();
    let log = files.get("texput.log").expect("the run wrote a log");
    Run {
        log: String::from_utf8_lossy(&log.data).into_owned(),
        reports,
    }
}

impl Run {
    /// What each `\showthe` showed, in order.
    fn shown(&self) -> Vec<&str> {
        self.log
            .lines()
            .filter_map(|l| l.strip_prefix("> "))
            .filter(|l| !l.starts_with('\\'))
            .map(|l| l.strip_suffix('.').unwrap_or(l))
            .collect()
    }

    /// The contents of each `\showbox`, one string of lines per box.
    fn boxes(&self) -> Vec<String> {
        let mut boxes = Vec::new();
        let mut lines = self.log.lines();
        while let Some(line) = lines.next() {
            if line.starts_with("> \\box") {
                let body: Vec<&str> = lines.by_ref().take_while(|l| !l.is_empty()).collect();
                boxes.push(body.join("\n"));
            }
        }
        boxes
    }

    /// Log lines that start with `prefix`.
    fn lines_starting(&self, prefix: &str) -> Vec<&str> {
        self.log.lines().filter(|l| l.starts_with(prefix)).collect()
    }

    /// The error messages, without their `! `.
    fn errors(&self) -> Vec<&str> {
        self.log
            .lines()
            .filter_map(|l| l.strip_prefix("! "))
            .filter(|l| *l != "OK.")
            .collect()
    }
}

/// The boxes directly inside a shown box, such as the lines of a paragraph.
fn rows(shown_box: &str) -> String {
    shown_box
        .lines()
        .filter_map(|l| l.strip_prefix('.'))
        .filter(|l| l.starts_with("\\hbox") || l.starts_with("\\vbox"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Shows `\the` of each item, returning the values.
fn show_the(setup: &str, items: &[&str]) -> Vec<String> {
    let shows: String = items.iter().map(|i| format!("\\showthe{i}\n")).collect();
    run(&format!("{setup}\n{shows}"))
        .shown()
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// Asserts that the lines of `actual` are `expected`, ignoring the common
/// indentation of `expected`.
#[track_caller]
fn assert_lines(actual: &str, expected: &str) {
    let expected: Vec<&str> = expected
        .lines()
        .skip_while(|l| l.trim().is_empty())
        .collect();
    let indent = expected
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let expected: Vec<&str> = expected
        .iter()
        .map(|l| l.get(indent..).unwrap_or("").trim_end())
        .collect();
    let expected = expected.join("\n");
    assert_eq!(
        actual,
        expected.trim_end(),
        "\n--- actual:\n{actual}\n--- expected:\n{expected}"
    );
}
