//! `build_page`, insertions and `\vsplit`.

use crate::run;

const PAGE: &str = r"\def\l{\hrule height 30pt width 10pt}
\vsize=100pt \maxdepth=2pt \topskip=10pt
\output={\showthe\outputpenalty \setbox0\vbox{\unvbox255}\showthe\ht0 \deadcycles=0 }
";

#[test]
fn the_page_breaks_at_the_least_cost() {
    let r = run(&format!(r"{PAGE}\l\l\penalty-50 \l\l\l\l \penalty-10000"));
    assert_eq!(r.shown(), ["-50", "60.0pt", "-10000", "120.0pt"]);
}

#[test]
fn insertions_shrink_the_goal_and_split_when_full() {
    let r = run(&format!(
        r"{PAGE}\count100=500 \dimen100=40pt \skip100=5pt
          \insert100{{\hrule height 20pt}}\showthe\pagegoal
          \l\insert100{{\hrule height 30pt}}\l\showthe\pagegoal \showthe\insertpenalties
          \penalty-10000"
    ));
    assert_eq!(
        r.shown(),
        ["85.0055pt", "70.0061pt", "-10000", "-10000", "60.0pt"]
    );
}

#[test]
fn vsplit_takes_the_best_break_that_fits() {
    let r = run(&format!(
        r"{PAGE}\setbox1\vbox{{\l\mark{{a}}\penalty 100 \l\mark{{b}}\l\penalty 0 \l}}
          \setbox2\vsplit1 to 70pt \showthe\ht2 \showthe\ht1 \showthe\badness
          \message{{[\splitfirstmark|\splitbotmark]}}"
    ));
    assert_eq!(r.shown(), ["70.0pt", "90.0pt", "10000"]);
    assert!(r.log.contains("[a|a]"));
}

#[test]
fn mark_classes_carry_over_like_xetex() {
    let r = run(&format!(
        r"{PAGE}\output={{\edef\x{{\topmarks5,\firstmarks5,\topmarks20,\firstmarks20}}\show\x
          \setbox0\box255 \deadcycles=0 }}
          \l\marks5{{A}}\marks20{{B}}\penalty-10000 \l\penalty-10000"
    ));
    // XeTeX's do_marks visits 16 of each index node's 64 slots, so class 20 never moves on.
    assert_eq!(r.lines_starting("->"), ["->,A,,B.", "->A,A,,B."]);
}
