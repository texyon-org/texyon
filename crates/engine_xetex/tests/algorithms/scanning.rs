//! The scanners: numbers, units, glue and e-TeX expressions.

use crate::{run, show_the};

#[test]
fn units_convert_to_points_as_tex_rounds_them() {
    let shown = show_the(
        r"\dimen1=1in \dimen2=1cm \dimen3=1bp \dimen4=3pc \dimen5=1dd \dimen6=12sp",
        &[
            r"\dimen1", r"\dimen2", r"\dimen3", r"\dimen4", r"\dimen5", r"\dimen6",
        ],
    );
    assert_eq!(
        shown,
        [
            "72.26999pt",
            "28.45274pt",
            "1.00374pt",
            "36.0pt",
            "1.07pt",
            "0.00018pt"
        ]
    );
}

#[test]
fn decimals_round_to_the_nearest_scaled_point() {
    let shown = show_the(
        r"\dimen1=1.23456789pt \dimen2=-.5\dimen1",
        &[r"\dimen1", r"\dimen2"],
    );
    assert_eq!(shown, ["1.23457pt", "-0.61728pt"]);
}

#[test]
fn integers_in_every_radix_and_with_signs() {
    let shown = show_the(
        r#"\count1='777 \count2="FF \count3=`a \count4=-+-3"#,
        &[r"\count1", r"\count2", r"\count3", r"\count4"],
    );
    assert_eq!(shown, ["511", "255", "97", "3"]);
}

#[test]
fn glue_keeps_the_order_of_each_component() {
    let shown = show_the(
        r"\skip1=1pt plus 2fil minus 3fill \skip2=1pt plus -1fil",
        &[r"\skip1", r"\skip2"],
    );
    assert_eq!(
        shown,
        ["1.0pt plus 2.0fil minus 3.0fill", "1.0pt plus -1.0fil"]
    );
}

#[test]
fn out_of_range_values_are_reported_and_clamped() {
    let r = run(r"\count1=2147483648 \dimen1=16384pt \showthe\count1 \showthe\dimen1");
    assert_eq!(r.errors(), ["Number too big.", "Dimension too large."]);
    assert_eq!(r.shown(), ["2147483647", "16383.99998pt"]);
}

#[test]
fn expressions_round_and_check_overflow() {
    let r = run(r"\showthe\numexpr 7*11/3\relax \showthe\numexpr -7/2\relax
                  \showthe\numexpr (1+2)*(3+4)\relax \showthe\dimexpr 1pt*3/4\relax
                  \showthe\glueexpr 1pt plus 2fil*2\relax \showthe\numexpr 2147483647+1\relax");
    assert_eq!(
        r.shown(),
        ["26", "-4", "21", "0.75pt", "2.0pt plus 4.0fil", "0"]
    );
    assert_eq!(r.errors(), ["Arithmetic overflow."]);
}

#[test]
fn parshapedimen_clamps_to_the_last_line() {
    let r = run(r"\parshape 2 1pt 2pt 3pt 4pt
                  \showthe\parshapedimen 3 \showthe\parshapedimen 2147483647
                  \showthe\parshapedimen 2147483646
                  \showthe\parshapeindent 2147483647 \showthe\parshapelength 2147483647");
    assert_eq!(r.shown(), ["3.0pt", "3.0pt", "4.0pt", "3.0pt", "4.0pt"]);
}

#[test]
fn showtokens_detokenizes_its_argument() {
    let r = run(r"\showtokens{a#b}");
    assert_eq!(r.shown(), ["a##b"]);
}

#[test]
fn fontdimen_grows_only_the_newest_font() {
    let r = run(r"\font\rm=cmr10 \font\mi=cmmi10
                  \showthe\fontdimen6\rm
                  \fontdimen9\mi=3pt \showthe\fontdimen9\mi \showthe\fontdimen8\mi
                  \fontdimen8\rm=5pt \showthe\fontdimen8\rm \showthe\fontdimen0\mi");
    assert_eq!(
        r.shown(),
        ["10.00002pt", "3.0pt", "0.0pt", "0.0pt", "0.0pt"]
    );
    assert_eq!(
        r.errors(),
        [
            r"Font \rm has only 7 fontdimen parameters.",
            r"Font \rm has only 7 fontdimen parameters.",
            r"Font \mi has only 9 fontdimen parameters.",
        ]
    );
}

#[test]
fn hyphenchar_and_skewchar_are_assigned_separately() {
    let r = run(r"\font\rm=cmr10 \hyphenchar\rm=-1 \skewchar\rm=45
                  \showthe\hyphenchar\rm \showthe\skewchar\rm");
    assert_eq!(r.shown(), ["-1", "45"]);
}

#[test]
fn showgroups_walks_nested_simple_groups_at_the_outer_level() {
    let r = run(r"{{\showgroups}}");
    assert_eq!(
        r.lines_starting("### "),
        [
            "### simple group (level 2) entered at line 5 ({)",
            "### simple group (level 1) entered at line 5 ({)",
            "### bottom level",
        ]
    );
}

#[test]
fn the_conditional_stack_tracks_nesting() {
    let r = run(r"\showthe\currentiflevel
                  \iftrue \iftrue \showthe\currentiflevel \fi \showthe\currentiflevel \fi
                  \ifcase 1 \or \showthe\currentiflevel \else \fi \showthe\currentiflevel");
    assert_eq!(r.shown(), ["0", "2", "1", "1", "0"]);
}

#[test]
fn changing_a_space_fontdimen_rebuilds_the_space_glue() {
    let r = run(r"\font\rm=cmr10 \rm \setbox0\hbox{a b}
                  \fontdimen2\rm=10pt \setbox2\hbox{a b} \showthe\wd0 \showthe\wd2");
    assert_eq!(r.shown(), ["13.88892pt", "20.55559pt"]);
}
