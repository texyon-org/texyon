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
fn every_mark_class_carries_over() {
    let r = run(&format!(
        r"{PAGE}\output={{\edef\x{{\topmarks5,\firstmarks5,\topmarks20,\firstmarks20}}\show\x
          \setbox0\box255 \deadcycles=0 }}
          \l\marks5{{A}}\marks20{{B}}\penalty-10000 \l\penalty-10000"
    ));
    // Upstream XeTeX leaves class 20 behind (UPSTREAM_BUGS.md 2).
    assert_eq!(r.lines_starting("->"), ["->,A,,B.", "->A,A,B,B."]);
}

#[test]
fn insertions_reach_the_box_of_their_class() {
    let r = run(&format!(
        r"{PAGE}\output={{\showthe\ht254 \showthe\ht7 \setbox0\box255 \deadcycles=0 }}
          \count254=1000 \dimen254=100pt \count7=1000 \dimen7=100pt
          \l\insert254{{\hrule height 4pt}}\insert7{{\hrule height 6pt}}\insert254{{\hrule height 3pt}}
          \showlists \penalty-10000"
    ));
    assert_eq!(r.shown(), ["7.0pt", "6.0pt"]);
    assert!(r.log.contains(r"\insert254 adds 7.0"), "{}", r.log);
}

#[test]
fn an_output_routine_left_open_is_fatal() {
    let r = run(
        r"\let\lb={ \vsize=100pt \output={\setbox0\box255 \deadcycles=0 \lb}
        \hrule height 10pt \penalty-10000",
    );
    assert_eq!(r.errors(), ["Emergency stop."]);
    assert_eq!(
        r.lines_starting("Unbalanced"),
        ["Unbalanced output routine"]
    );
    assert_eq!(
        r.reports,
        ["texput.tex:6: Emergency stop\nUnbalanced output routine"]
    );
}
