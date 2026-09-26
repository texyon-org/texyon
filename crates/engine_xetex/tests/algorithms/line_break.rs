//! `line_break`: feasible breakpoints, demerits and the paragraph shape.
//!
//! The "words" are 10pt rules, so the tests need no fonts.

use crate::{assert_lines, rows, run};

const WORDS: &str = r"\def\w{\vrule width 10pt height 5pt \hskip 5pt plus 5pt minus 2pt}
\def\s{\vrule width 10pt height 5pt \hskip 5pt plus 20pt}
\def\t{\vrule width 10pt height 5pt \hskip 5pt plus 1pt}
\parfillskip=0pt plus 1fil \pretolerance=-1 \tolerance=10000 \linepenalty=10
\parindent=0pt \hsize=40pt
";

fn paragraph(body: &str) -> crate::Run {
    run(&format!("{WORDS}{body}"))
}

#[test]
fn lines_fill_the_measure_without_stretching() {
    let r = paragraph(r"\setbox0\vbox{\w\w\w\w\w\w\par \showthe\prevgraf}\showbox0");
    assert_lines(
        &rows(&r.boxes()[0]),
        r"
        \hbox(5.0+0.0)x40.0
        \hbox(5.0+0.0)x40.0",
    );
    assert_eq!(r.shown(), ["2"]);
}

#[test]
fn an_eject_penalty_forces_a_break() {
    let r = paragraph(r"\setbox0\vbox{\w\w\penalty-10000 \w\w\w\w\par \showthe\prevgraf}\showbox0");
    assert_lines(
        &rows(&r.boxes()[0]),
        r"
        \hbox(5.0+0.0)x40.0, glue set 1.0
        \hbox(5.0+0.0)x40.0
        \hbox(5.0+0.0)x40.0, glue set 30.0fil",
    );
    assert_eq!(r.shown(), ["3"]);
}

#[test]
fn looseness_asks_for_one_more_line() {
    let r = paragraph(
        r"\setbox0\vbox{\s\s\s\s\s\s\par \showthe\prevgraf}
          \setbox0\vbox{\looseness=1 \s\s\s\s\s\s\par \showthe\prevgraf}",
    );
    assert_eq!(r.shown(), ["2", "3"]);
}

#[test]
fn emergency_stretch_rescues_an_infeasible_paragraph() {
    let r = paragraph(
        r"\setbox0\vbox{\hsize=44pt \tolerance=100 \t\t\t\t\t\t\par}\showbox0
          \setbox0\vbox{\hsize=44pt \tolerance=100 \emergencystretch=10pt \t\t\t\t\t\t\par}\showbox0",
    );
    let boxes = r.boxes();
    assert_lines(
        &rows(&boxes[0]),
        r"
        \hbox(5.0+0.0)x44.0
        \hbox(5.0+0.0)x44.0, glue set 19.0fil",
    );
    assert_lines(
        &rows(&boxes[1]),
        r"
        \hbox(5.0+0.0)x44.0, glue set 2.0
        \hbox(5.0+0.0)x44.0, glue set 4.0fil",
    );
}

#[test]
fn parshape_gives_each_line_its_width_and_indent() {
    let r = paragraph(r"\setbox0\vbox{\parshape 2 5pt 30pt 0pt 50pt \w\w\w\w\w\w\w\par}\showbox0");
    assert_lines(
        &rows(&r.boxes()[0]),
        r"
        \hbox(5.0+0.0)x30.0, glue set 1.0, shifted 5.0
        \hbox(5.0+0.0)x50.0, glue set - 0.83333
        \hbox(5.0+0.0)x50.0, glue set 40.0fil",
    );
}

#[test]
fn hanging_indentation_applies_to_the_first_lines() {
    let r = paragraph(r"\setbox0\vbox{\hangindent=10pt \hangafter=-1 \w\w\w\w\w\par}\showbox0");
    assert_lines(
        &rows(&r.boxes()[0]),
        r"
        \hbox(5.0+0.0)x30.0, glue set 1.0, shifted 10.0
        \hbox(5.0+0.0)x40.0",
    );
}

#[test]
fn last_line_fit_sets_the_last_line_like_the_one_before() {
    let r = paragraph(r"\setbox0\vbox{\lastlinefit=1000 \hsize=38pt \w\w\w\w\w\par}\showbox0");
    assert_lines(
        &rows(&r.boxes()[0]),
        r"
        \hbox(5.0+0.0)x38.0, glue set - 0.5
        \hbox(5.0+0.0)x38.0, glue set - 0.5",
    );
    assert!(r.boxes()[0].contains("\\glue(\\parfillskip) 14.0"));
}

#[test]
fn interline_penalties_add_up_with_32_bit_wraparound() {
    let r = paragraph(
        r"\interlinepenalty=2000000000 \clubpenalty=2000000000 \hsize=10pt
          \setbox0\vbox{\w\w\w\par}\showbox0",
    );
    let shown = r.boxes().remove(0);
    let penalties: Vec<&str> = shown
        .lines()
        .filter(|l| l.starts_with(".\\penalty"))
        .collect();
    assert_eq!(
        penalties,
        [".\\penalty -294967296", ".\\penalty 2000000000"]
    );
}
