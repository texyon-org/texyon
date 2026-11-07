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

#[test]
fn sparse_registers_restore_at_group_end() {
    let r = run(r"\count300=1 \dimen400=1pt
                  {\count300=2 {\global\dimen400=3pt \count300=4 \showthe\count300}
                   \showthe\count300 \showthe\dimen400}
                  \showthe\count300 \showthe\dimen400");
    assert_eq!(r.shown(), ["4", "2", "3.0pt", "1", "3.0pt"]);
}

#[test]
fn internal_values_coerce_down_to_the_wanted_level() {
    let r = run(
        r"\skip1=3pt plus 1fil \dimen1=\skip1 \count1=\skip1 \count2=\dimen1
                  \dimen2=2\skip1 \count3=4 \skip2=\count3 pt plus 1pt
                  \showthe\dimen1 \showthe\count1 \showthe\count2 \showthe\dimen2
                  \showthe\skip2",
    );
    assert_eq!(
        r.shown(),
        ["3.0pt", "196608", "196608", "6.0pt", "4.0pt plus 1.0pt"]
    );
    assert_eq!(r.errors(), Vec::<&str>::new());
}

#[test]
fn negated_internal_values_leave_their_source_alone() {
    let r = run(r"\skip1=1pt plus 2fil minus 3pt \skip2=-\skip1
                  \skip3=-\glueexpr\skip1*2\relax \dimen1=-\dimexpr 1pt+1pt\relax
                  \count1=-\numexpr 3\relax \showthe\skip1 \showthe\skip2 \showthe\skip3
                  \showthe\dimen1 \showthe\count1");
    assert_eq!(
        r.shown(),
        [
            "1.0pt plus 2.0fil minus 3.0pt",
            "-1.0pt plus -2.0fil minus -3.0pt",
            "-2.0pt plus -4.0fil minus -6.0pt",
            "-2.0pt",
            "-3"
        ]
    );
}

#[test]
fn a_fetched_glue_register_keeps_its_spec_alive() {
    let r = run(r"\skip1=1pt plus 1fil \skip2=\skip1 \skip1=2pt
                  \skip3=5pt minus 1pt \skip4=6pt minus 2pt \showthe\skip2");
    assert_eq!(r.shown(), ["1.0pt plus 1.0fil"]);
}

#[test]
fn mu_glue_converts_only_through_mutoglue_and_gluetomu() {
    let r = run(r"\muskip1=3mu plus 1fil \skip1=2pt minus 1pt
                  \showthe\mutoglue\muskip1 \showthe\gluetomu\skip1 \dimen1=\muskip1
                  \showthe\dimen1");
    assert_eq!(
        r.shown(),
        ["3.0pt plus 1.0fil", "2.0mu minus 1.0mu", "3.0pt"]
    );
    assert_eq!(r.errors(), ["Incompatible glue units."]);
}

#[test]
fn last_items_read_the_tail_of_the_current_list() {
    let r = run(
        r"\setbox0\hbox{\showthe\lastnodetype \penalty7 \showthe\lastpenalty
                  \showthe\lastnodetype \kern2pt \showthe\lastkern \showthe\lastpenalty}
                  \setbox0\hbox{$\mskip 3mu plus 1fil \showthe\lastskip$}",
    );
    assert_eq!(
        r.shown(),
        ["-1", "7", "13", "2.0pt", "0", "3.0mu plus 1.0fil"]
    );
}

#[test]
fn the_rejects_ignorespaces() {
    let r = run(r"\count1=\the\ignorespaces 5 \showthe\count1");
    assert_eq!(r.errors(), ["You can't use `\\ignorespaces' after \\the."]);
    assert_eq!(r.shown(), ["5"]);
}

#[test]
fn mode_and_page_quantities_read_their_own_state() {
    let r = run(r"\showthe\spacefactor \showthe\prevdepth \showthe\pagegoal
                  \showthe\interactionmode \setbox1\hbox{\showthe\spacefactor}");
    assert_eq!(r.errors(), ["Improper \\spacefactor."]);
    assert_eq!(r.shown(), ["0", "-1000.0pt", "16383.99998pt", "2", "1000"]);
}
