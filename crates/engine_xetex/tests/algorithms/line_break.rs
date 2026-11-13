//! `line_break`: feasible breakpoints, demerits and the paragraph shape.
//!
//! Most "words" are 10pt rules, so most tests need no fonts.

use crate::{assert_lines, rows, run, MATH_FONTS};

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

#[test]
fn line_numbers_wrapped_by_prevgraf_are_past_the_parshape() {
    let r = paragraph(
        r"\setbox0\vbox{\parshape 2 5pt 30pt 0pt 50pt \interlinepenalties 2 7 8
          \w\prevgraf=2147483647 \w\w\w\w\w\w\par \showthe\prevgraf}\showbox0",
    );
    let shown = r.boxes().remove(0);
    let lines = rows(&shown);
    assert_eq!(lines.lines().count(), 7);
    assert!(lines.lines().all(|l| l.starts_with("\\hbox(5.0+0.0)x50.0")));
    assert!(shown
        .lines()
        .filter(|l| l.starts_with(".\\penalty"))
        .all(|l| l == ".\\penalty 8"));
    assert_eq!(r.shown(), ["-2147483642"]);
}

/// Typesets each word on its own and returns the hyphenated words, as `\showhyphens` reports them.
fn hyphenated(setup: &str, words: &str) -> Vec<String> {
    let r = run(&format!(
        r"\font\rm=cmr10 \rm \hyphenchar\rm=`- \lefthyphenmin=1 \righthyphenmin=1
        \lccode`a=`a \lccode`b=`b \lccode`c=`c \lccode`d=`d
        {setup}
        \hsize=1000pt \parfillskip=0pt \pretolerance=-1 \tolerance=-1 \hbadness=0
        \setbox0\vbox{{\ {words}}}"
    ));
    r.log
        .lines()
        .filter_map(|l| l.strip_prefix(r"[] \rm "))
        .map(str::to_owned)
        .collect()
}

#[test]
fn patterns_take_the_highest_value_at_each_position() {
    let words = hyphenated(r"\patterns{a1b b1c ab2c c1d}", "abcd");
    assert_eq!(words, ["a-bc-d"]);
}

#[test]
fn exceptions_override_patterns() {
    let words = hyphenated(r"\patterns{a1b} \hyphenation{abc-d}", r"abcd\ abab");
    assert_eq!(words, ["abc-d a-ba-b"]);
}

#[test]
fn saved_hyph_codes_outlive_later_lccode_changes() {
    let saved = hyphenated(r"\savinghyphcodes=1 \patterns{c1d} \lccode`d=0", "abcd");
    let unsaved = hyphenated(r"\patterns{c1d} \lccode`d=0", "abcd");
    assert_eq!(
        (saved, unsaved),
        (vec!["abc-d".to_owned()], vec!["abcd".to_owned()])
    );
}

#[test]
fn every_digit_of_a_pattern_applies() {
    let words = hyphenated(r"\patterns{a1b3c}", "abc");
    assert_eq!(words, ["a-b-c"]);
}

#[test]
fn exceptions_with_the_same_hash_share_a_chain() {
    // `aac` and `aba` land in the same exception slot.
    let words = hyphenated(r"\hyphenation{a-ac ab-a}", r"aac\ aba");
    assert_eq!(words, ["a-ac ab-a"]);
}

#[test]
fn lr_segments_reopen_on_the_next_line() {
    let r = run(&format!(
        r"{MATH_FONTS}\TeXXeTstate=1 \hsize=20pt \parindent=0pt \rightskip=0pt plus 100pt
          \setbox1\vbox{{\beginR x\hskip 30pt x\endR\par}}\showbox1
          \setbox2\hbox{{$x$\beginL x\endR}}\showbox2"
    ));
    assert_lines(
        &r.boxes()[0],
        r"
        \vbox(8.61108+0.0)x20.0
        .\hbox(4.30554+0.0)x20.0, glue set 0.14722
        ..\hbox(0.0+0.0)x0.0
        ..\beginR
        ..\rm x
        ..\endR
        ..\glue(\rightskip) 0.0 plus 100.0
        .\glue(\lineskip) 0.0
        .\hbox(4.30554+0.0)x20.0, glue set 0.14722
        ..\beginR
        ..\rm x
        ..\endR
        ..\penalty 10000
        ..\glue(\parfillskip) 0.0
        ..\glue(\rightskip) 0.0 plus 100.0",
    );
    assert_eq!(
        r.lines_starting("\\endL or"),
        [r"\endL or \endR problem (1 missing, 1 extra) detected at line 14"]
    );
    assert_lines(
        &r.boxes()[1],
        r"
        \hbox(4.30554+0.0)x10.99307
        .\mathon
        .\mi x
        .\mathoff
        .\beginL
        .\rm x
        .\kern 0.0
        .\endL",
    );
}

#[test]
fn margin_kerns_keep_their_side() {
    let r = run(&format!(
        r"{MATH_FONTS}\XeTeXprotrudechars=2 \lpcode\rm`A=500 \rpcode\rm`B=500
          \hsize=100pt \parindent=0pt \parfillskip=0pt
          \setbox1\vbox{{AB\par\global\setbox4\lastbox}}\showbox4
          \message{{[\leftmarginkern4|\rightmarginkern4]}}"
    ));
    assert!(r.log.contains("[-5.00002pt|-5.00002pt]"), "{}", r.log);
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(6.83331+0.0)x100.0
        .\kern-5.00002 (left margin)
        .\hbox(0.0+0.0)x0.0
        .\rm A
        .\rm B
        .\penalty 10000
        .\kern-5.00002 (right margin)
        .\glue(\parfillskip) 0.0
        .\glue(\rightskip) 0.0",
    );
}

#[test]
fn a_right_to_left_display_is_a_dlist() {
    let r = run(&format!(
        r"{MATH_FONTS}\TeXXeTstate=1 \hsize=100pt \parindent=0pt
          \setbox1\vbox{{\beginR x$$\showlists x\eqno y$$\par}}\showbox1"
    ));
    assert_lines(
        &r.boxes()[0],
        r"
        \vbox(10.55553+0.0)x100.0
        .\hbox(4.30554+0.0)x100.0
        ..\hbox(0.0+0.0)x0.0
        ..\beginR
        ..\rm x
        ..\penalty 10000
        ..\glue(\parfillskip) 0.0
        ..\endR
        ..\glue(\rightskip) 0.0
        .\penalty 0
        .\glue(\abovedisplayshortskip) 0.0
        .\glue(\lineskip) 0.0
        .\hbox(4.30554+1.94444)x100.0
        ..\beginM
        ..\kern0.0
        ..\hbox(4.30554+1.94444)x5.2616, display
        ...\mi y
        ...\kern0.35878
        ..\kern41.88077
        ..\hbox(4.30554+0.0)x5.71527, display
        ...\mi x
        ..\kern47.14236
        ..\endM
        .\penalty 0
        .\glue(\belowdisplayshortskip) 0.0",
    );
}

#[test]
fn a_wrapped_prevgraf_without_hanging_indent_keeps_the_full_measure() {
    let r = paragraph(
        r"\setbox0\vbox{\hangindent=20pt \hangafter=-5 \w\par
          \noindent\prevgraf=2147483647 \w\w\w\par}\showbox0",
    );
    assert_lines(
        &rows(&r.boxes()[0]),
        r"
        \hbox(5.0+0.0)x20.0, glue set 10.0fil, shifted 20.0
        \hbox(5.0+0.0)x40.0
        \hbox(5.0+0.0)x40.0
        \hbox(5.0+0.0)x40.0, glue set 30.0fil",
    );
}

#[test]
fn left_boundary_after_a_hyphen_is_no_character() {
    let r = run(
        r"\font\xb=xbound \xb \hyphenchar\xb=`- \lefthyphenmin1 \righthyphenmin1
          \hyphenation{a-a} \hsize1pt \tolerance10000 \parindent0pt \hfuzz\maxdimen
          \hbadness10000 \setbox0\vbox{\noindent a aa\par}\showbox0",
    );
    assert_lines(
        &r.boxes()[0],
        r"
        \vbox(20.99995+0.0)x1.0
        .\hbox(6.99998+0.0)x1.0
        ..\kern1.99998
        ..\xb a
        ..\glue(\rightskip) 0.0
        .\glue(\lineskip) 0.0
        .\hbox(6.99998+0.0)x1.0
        ..\kern1.99998
        ..\xb a
        ..\discretionary
        ..\xb -
        ..\glue(\rightskip) 0.0
        .\glue(\lineskip) 0.0
        .\hbox(6.99998+0.0)x1.0
        ..\kern1.99998
        ..\xb a
        ..\penalty 10000
        ..\glue(\parfillskip) 0.0
        ..\glue(\rightskip) 0.0",
    );
}

#[test]
fn a_word_at_the_start_of_a_list_merges_its_interrupted_pieces() {
    // XeTeX's list head is a char node by address, so it keeps the first piece apart and the
    // locale breaks `/05` after the slash (upstream bug 25).
    let r = run(
        r#"\font\lm="[lmroman12-regular.otf]" at 10pt \lm \XeTeXlinebreaklocale "ko"
          \XeTeXinterchartokenstate=1 \XeTeXcharclass`\/=1
          \XeTeXinterchartoks 0 1 {\relax}\XeTeXinterchartoks 1 0 {\relax}
          \hsize200pt \setbox0\vbox{\noindent 2014/05 x\par}\showbox0"#,
    );
    assert_lines(
        &r.boxes()[0],
        r"
        \vbox(14.06999+0.0)x200.0
        .\hbox(11.27+2.79999)x200.0, glue set 96.48528
        ..\lm 2014/05
        ..\glue 3.26 plus 1.62999 minus 1.08665
        ..\lm x
        ..\penalty 10000
        ..\glue(\parfillskip) 0.0
        ..\glue(\rightskip) 0.0",
    );
}
