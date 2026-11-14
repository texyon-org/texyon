//! Timings of the editing session for each document.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Result, bail};

use crate::{corpus, scenario, session::Host};

/// Run the session `rounds` times on each document and print the fastest of each step.
pub fn run(host: &Host, documents: &[PathBuf], rounds: usize) -> Result<()> {
    println!(
        "{:<32} {:>6} {:>8} {:>8} {:>8} {:>9} {:>9} {:>8}",
        "document", "pages", "cold", "tail", "full", "keystroke", "key tail", "export"
    );
    let mut failed = 0;
    for path in documents {
        match fastest(host, path, rounds) {
            Ok(row) => println!(
                "{:<32} {:>6} {:>8} {:>8} {:>8} {:>9} {:>9} {:>8}",
                label(path),
                row.pages,
                ms(row.cold),
                ms(row.tail),
                ms(row.full_pass),
                ms(row.keystroke),
                ms(row.keystroke_tail),
                ms(row.export)
            ),
            Err(error) => {
                failed += 1;
                println!("{:<32} failed: {error:#}", label(path));
            }
        }
    }
    println!("milliseconds, fastest of {rounds} rounds, keystrokes as the median of each round");
    if failed > 0 {
        bail!("{failed} of {} documents failed", documents.len());
    }
    Ok(())
}

fn fastest(host: &Host, path: &Path, rounds: usize) -> Result<Row> {
    let mut best = Row::from(scenario::run(host, corpus::open(path)?, None)?);
    for _ in 1..rounds {
        best = best.fastest(&Row::from(scenario::run(host, corpus::open(path)?, None)?));
    }
    Ok(best)
}

/// The last two parts of a path, enough to tell `paper/main.tex` from `thesis/main.tex`.
fn label(path: &Path) -> String {
    let parts: Vec<_> = path.components().rev().take(2).collect();
    parts
        .iter()
        .rev()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

struct Row {
    pages: usize,
    cold: Duration,
    tail: Duration,
    full_pass: Duration,
    keystroke: Duration,
    keystroke_tail: Duration,
    export: Duration,
}

impl From<scenario::Timings> for Row {
    fn from(timings: scenario::Timings) -> Self {
        Self {
            pages: timings.pages,
            cold: timings.cold,
            tail: timings.tail,
            full_pass: timings.full_pass,
            keystroke: median(timings.keystrokes),
            keystroke_tail: median(timings.keystroke_tails),
            export: timings.export,
        }
    }
}

impl Row {
    fn fastest(&self, other: &Self) -> Self {
        Self {
            pages: self.pages,
            cold: self.cold.min(other.cold),
            tail: self.tail.min(other.tail),
            full_pass: self.full_pass.min(other.full_pass),
            keystroke: self.keystroke.min(other.keystroke),
            keystroke_tail: self.keystroke_tail.min(other.keystroke_tail),
            export: self.export.min(other.export),
        }
    }
}

fn median(mut durations: Vec<Duration>) -> Duration {
    durations.sort();
    durations
        .get(durations.len() / 2)
        .copied()
        .unwrap_or_default()
}

fn ms(duration: Duration) -> String {
    format!("{:.1}", duration.as_secs_f64() * 1000.0)
}
