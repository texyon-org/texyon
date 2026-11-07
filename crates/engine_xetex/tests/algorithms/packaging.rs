//! `hpack` and `vpack`: box dimensions, glue setting and badness.

use crate::{assert_lines, run, show_the};

#[test]
fn hpack_sums_widths_and_takes_extreme_heights() {
    let shown = show_the(
        r"\setbox0\hbox{\vrule width 3pt height 2pt depth 1pt \hskip 4pt \kern 5pt
          \raise 3pt\hbox{\vrule width 1pt height 1pt}}",
        &[r"\wd0", r"\ht0", r"\dp0"],
    );
    assert_eq!(shown, ["13.0pt", "4.0pt", "1.0pt"]);
}

#[test]
fn hpack_stretches_the_highest_order_only() {
    let r = run(
        r"\setbox0\hbox to 100pt{\hskip 0pt plus 10pt \hskip 0pt plus 2fil \vrule width 10pt}
          \showbox0 \showthe\badness",
    );
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(0.0+0.0)x100.0, glue set 45.0fil
        .\glue 0.0 plus 10.0
        .\glue 0.0 plus 2.0fil
        .\rule(*+*)x10.0",
    );
    assert_eq!(r.shown(), ["0"]);
}

#[test]
fn hpack_badness_is_cubic_in_the_stretch_ratio() {
    let shown = show_the(
        r"\setbox0\hbox to 30pt{\vrule width 10pt \hskip 0pt plus 10pt}\count1=\badness
          \setbox0\hbox to 15pt{\vrule width 10pt \hskip 0pt plus 10pt}\count2=\badness
          \setbox0\hbox to 8pt{\vrule width 10pt \hskip 0pt minus 4pt}\count3=\badness",
        &[r"\count1", r"\count2", r"\count3"],
    );
    assert_eq!(shown, ["800", "12", "12"]);
}

#[test]
fn hpack_never_shrinks_past_the_available_shrink() {
    let r = run(
        r"\setbox0\hbox to 5pt{\vrule width 10pt \hskip 0pt minus 2pt}
          \showbox0 \showthe\badness",
    );
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(0.0+0.0)x5.0, glue set - 1.0
        .\rule(*+*)x10.0
        .\glue 0.0 minus 2.0",
    );
    assert_eq!(r.shown(), ["1000000"]);
}

#[test]
fn hpack_reports_boxes_beyond_the_tolerances() {
    let r = run(r"\hbadness=100 \hfuzz=1pt \overfullrule=5pt
                  \setbox0\hbox to 30pt{\vrule width 10pt \hskip 0pt plus 10pt}
                  \setbox0\hbox to 5pt{\vrule width 10pt \hskip 0pt minus 2pt}\showbox0");
    assert_eq!(
        r.lines_starting("Underfull").len() + r.lines_starting("Overfull").len(),
        2
    );
    assert!(r
        .log
        .contains("Underfull \\hbox (badness 800) detected at line 6"));
    assert!(r
        .log
        .contains("Overfull \\hbox (3.0pt too wide) detected at line 7"));
    assert!(r.boxes()[0].ends_with(".\\rule(*+*)x5.0"));
}

#[test]
fn vpack_limits_the_depth_to_boxmaxdepth() {
    let shown = show_the(
        r"\boxmaxdepth=2pt \setbox0\vbox{\hrule height 1pt depth 5pt}",
        &[r"\ht0", r"\dp0"],
    );
    assert_eq!(shown, ["4.0pt", "2.0pt"]);
}

#[test]
fn vtop_takes_its_height_from_the_first_item() {
    let shown = show_the(
        r"\setbox0\vtop{\hrule height 3pt depth 1pt \kern 2pt \hrule height 4pt}",
        &[r"\ht0", r"\dp0"],
    );
    assert_eq!(shown, ["3.0pt", "7.0pt"]);
}

#[test]
fn vpack_sets_glue_like_hpack() {
    let r = run(
        r"\setbox0\vbox to 10pt{\hrule height 4pt \vskip 0pt plus 1fil \vskip 0pt plus 2pt}
          \showbox0",
    );
    assert_lines(
        &r.boxes()[0],
        r"
        \vbox(10.0+0.0)x0.0, glue set 6.0fil
        .\rule(4.0+0.0)x*
        .\glue 0.0 plus 1.0fil
        .\glue 0.0 plus 2.0",
    );
}

#[test]
fn the_main_loop_builds_ligatures_and_kerns() {
    let r = run(r"\font\rm=cmr10 \rm \setbox0\hbox{ffi AV}\showbox0");
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(6.94444+0.0)x25.55559
        .\rm ^^N (ligature ffi)
        .\glue 3.33333 plus 1.66666 minus 1.11111
        .\rm A
        .\kern-1.11113
        .\rm V",
    );
}

#[test]
fn box_shifts_follow_the_direction_of_each_command() {
    let r = run(r"\setbox1\hbox{}
                  \setbox0\vbox{\moveleft 2pt\copy1 \moveright 3pt\copy1}
                  \setbox2\hbox{\raise1pt\copy1\lower2pt\copy1}
                  \showbox0 \showbox2");
    assert_lines(
        &r.boxes()[0],
        r"
        \vbox(0.0+0.0)x3.0
        .\hbox(0.0+0.0)x0.0, shifted -2.0
        .\glue(\baselineskip) 0.0
        .\hbox(0.0+0.0)x0.0, shifted 3.0",
    );
    assert_lines(
        &r.boxes()[1],
        r"
        \hbox(1.0+2.0)x0.0
        .\hbox(0.0+0.0)x0.0, shifted -1.0
        .\hbox(0.0+0.0)x0.0, shifted 2.0",
    );
}

#[test]
fn removals_take_only_their_own_node_type() {
    let r = run(r"\parindent=5pt
                  \setbox0\hbox{\kern1pt\kern2pt\unkern\hskip3pt\unkern\unskip
                                \penalty5\unskip\unpenalty\indent\noindent}
                  \showbox0");
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(0.0+0.0)x6.0
        .\kern 1.0
        .\hbox(0.0+0.0)x5.0",
    );
}

#[test]
fn case_shifts_use_their_own_code_table() {
    let r = run(r"\lowercase{\count1=`A} \uppercase{\count2=`a} \showthe\count1 \showthe\count2");
    assert_eq!(r.shown(), ["97", "65"]);
}

#[test]
fn alignment_preambles_expand_span_and_set_tabskip() {
    let r = run(r"\def\x{\hskip5pt##}\tabskip=1pt
                  \setbox0\vbox{\halign{#\tabskip=2pt&\span\x\cr\hskip1pt&\hskip3pt\cr\crcr}}
                  \showbox0");
    assert_eq!(r.errors(), Vec::<&str>::new());
    assert_lines(
        &r.boxes()[0],
        r"
        \vbox(0.0+0.0)x14.0
        .\hbox(0.0+0.0)x14.0
        ..\glue(\tabskip) 1.0
        ..\hbox(0.0+0.0)x1.0
        ...\glue 1.0
        ..\glue(\tabskip) 2.0
        ..\hbox(0.0+0.0)x8.0
        ...\glue 5.0
        ...\glue 3.0
        ..\glue(\tabskip) 2.0",
    );
}
