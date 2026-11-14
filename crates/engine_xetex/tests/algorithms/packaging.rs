//! `hpack` and `vpack`: box dimensions, glue setting and badness.

use crate::{MATH_FONTS, assert_lines, run, show_the};

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
    assert!(
        r.log
            .contains("Underfull \\hbox (badness 800) detected at line 6")
    );
    assert!(
        r.log
            .contains("Overfull \\hbox (3.0pt too wide) detected at line 7")
    );
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
fn inter_char_tokens_end_the_word_before_them() {
    let r = run(
        r"\font\rm=cmr10 \rm \XeTeXinterchartokenstate=1 \XeTeXcharclass`a=1
                  \XeTeXinterchartoks 0 1={\relax}\setbox0\hbox{fia}\showbox0",
    );
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(6.94444+0.0)x10.55559
        .\rm ^^L (ligature fi)
        .\rm a",
    );
}

#[test]
fn boundary_inter_char_tokens_keep_the_token_after_them() {
    let r = run(
        r"\font\rm=cmr10 \rm \XeTeXinterchartokenstate=1 \XeTeXcharclass`a=1
                  \XeTeXinterchartoks 1 4095={\show}\setbox0\hbox{a{b}}",
    );
    assert_eq!(r.lines_starting("> "), ["> begin-group character {."]);
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

#[test]
fn io_whatsits_keep_their_fields_through_a_copy() {
    let r = run(r"\lefthyphenmin=2 \righthyphenmin=3
        \setbox0\hbox{\openout3=foo \write3{x}\closeout3 \write-1{y}\special{s}\setlanguage5 }
        \setbox1\copy0 \setbox0\hbox{}\showbox1 \immediate\write-1{hello}");
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(0.0+0.0)x0.0
        .\openout3=foo
        .\write3{x}
        .\closeout3
        .\write-{y}
        .\special{s}
        .\setlanguage5 (hyphenmin 2,3)",
    );
    assert!(r.log.lines().any(|l| l == "hello"));
}

#[test]
fn kerns_show_what_made_them() {
    let r = run(&format!(
        r#"{MATH_FONTS}\setbox0\hbox{{AV\kern1pt f\/{{\mi\accent"7F A}}$\mkern18mu\showlists$}}
          \showbox0"#
    ));
    assert_eq!(r.lines_starting("\\mkern"), [r"\mkern18.0mu"]);
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(9.47221+0.0)x36.22203
        .\rm A
        .\kern-1.11113
        .\rm V
        .\kern 1.0
        .\rm f
        .\kern 0.77779
        .\kern 2.99306 (for accent)
        .\hbox(6.94444+0.0)x2.77779, shifted -2.52777
        ..\mi ^^?
        .\kern -5.77084 (for accent)
        .\mi A
        .\mathon
        .\kern 9.99976
        .\mathoff",
    );
}

#[test]
fn vadjust_pre_lands_before_the_line() {
    let r = run(&format!(
        r"{MATH_FONTS}\hsize=100pt \parindent=0pt \parfillskip=0pt plus 1fil
          \setbox1\vbox{{x\vadjust{{\hrule height1pt}}\vadjust pre{{\hrule height2pt}}\showlists\par}}
          \showbox1"
    ));
    assert_eq!(
        r.lines_starting("\\vadjust"),
        [r"\vadjust", r"\vadjust pre "]
    );
    assert_lines(
        &r.boxes()[0],
        r"
        \vbox(7.30554+0.0)x100.0
        .\rule(2.0+0.0)x*
        .\hbox(4.30554+0.0)x100.0, glue set 94.7222fil
        ..\hbox(0.0+0.0)x0.0
        ..\rm x
        ..\penalty 10000
        ..\glue(\parfillskip) 0.0 plus 1.0fil
        ..\glue(\rightskip) 0.0
        .\rule(1.0+0.0)x*",
    );
}

#[test]
fn discretionaries_and_spans_keep_their_counts() {
    let r = run(&format!(
        r"{MATH_FONTS}\setbox0\hbox{{\discretionary{{a}}{{b}}{{cd}}}}\showbox0
          \setbox1\vbox{{\halign{{#\tabskip=0pt&#\cr\hskip10pt&\hskip20pt\cr\hskip50pt\span\cr}}}}\showbox1"
    ));
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(6.94444+0.0)x10.00002
        .\discretionary replacing 2
        ..\rm a
        .|\rm b
        .\rm c
        .\rm d",
    );
    assert_lines(
        &r.boxes()[1],
        r"
        \vbox(0.0+0.0)x50.0
        .\hbox(0.0+0.0)x50.0
        ..\glue(\tabskip) 0.0
        ..\hbox(0.0+0.0)x10.0
        ...\glue 10.0
        ..\glue(\tabskip) 0.0
        ..\hbox(0.0+0.0)x40.0
        ...\glue 20.0
        ..\glue(\tabskip) 0.0
        .\glue(\baselineskip) 0.0
        .\hbox(0.0+0.0)x50.0
        ..\glue(\tabskip) 0.0
        ..\hbox(0.0+0.0)x10.0
        ...\glue 50.0
        ..\glue(\tabskip) 0.0
        ..\hbox(0.0+0.0)x40.0
        ..\glue(\tabskip) 0.0",
    );
}

#[test]
fn glue_shows_its_kind() {
    let r = run(&format!(
        r"{MATH_FONTS}\setbox0\hbox to 30pt{{\leaders\hrule\hfil\cleaders\hrule\hfil
          \xleaders\hrule\hfil$x\nonscript\mskip 3mu x\showlists$}}\showbox0"
    ));
    assert_eq!(
        r.lines_starting("\\glue"),
        [r"\glue(\nonscript)", r"\glue(\mskip) 3.0mu"]
    );
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(4.30554+0.0)x30.0, glue set 5.63428fil
        .\leaders 0.0 plus 1.0fil
        ..\rule(0.4+0.0)x*
        .\cleaders 0.0 plus 1.0fil
        ..\rule(0.4+0.0)x*
        .\xleaders 0.0 plus 1.0fil
        ..\rule(0.4+0.0)x*
        .\mathon
        .\mi x
        .\glue(\nonscript)
        .\glue 1.66663
        .\mi x
        .\mathoff",
    );
}

#[test]
fn showgroups_names_inserts_and_vadjusts() {
    let r = run(
        r"\setbox0\hbox{\vadjust{\showgroups}\vadjust pre{\showgroups}}
\insert100{\showgroups}",
    );
    let shown: Vec<_> = r
        .lines_starting("### insert")
        .iter()
        .filter_map(|l| l.split_once(" ("))
        .map(|(_, rest)| rest.split_once(" (").map_or(rest, |(_, g)| g))
        .collect();
    assert_eq!(shown, [r"\vadjust{)", r"\vadjust pre{)", r"\insert100{)"]);
}
