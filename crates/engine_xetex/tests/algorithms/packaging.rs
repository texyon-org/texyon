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
