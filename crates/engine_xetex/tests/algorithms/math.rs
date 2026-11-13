//! `mlist_to_hlist`: scripts, spacing, fractions and delimiters, set in
//! Computer Modern.

use crate::{assert_lines, run, MATH_FONTS};

fn formula(math: &str) -> String {
    let r = run(&format!(
        "{MATH_FONTS}\\medmuskip=4mu plus 2mu minus 4mu \\mathsurround=0pt
         \\setbox0\\hbox{{${math}$}}\\showbox0"
    ));
    r.boxes().remove(0)
}

#[test]
fn a_superscript_is_raised_by_sup1() {
    assert_lines(
        &formula("x^y"),
        r"
        \hbox(7.93446+0.0)x10.97687
        .\mathon
        .\mi x
        .\hbox(4.30554+1.94444)x5.2616, shifted -3.62892
        ..\mi y
        .\mathoff",
    );
}

#[test]
fn both_scripts_stack_in_one_vbox() {
    assert_lines(
        &formula("x_y^x"),
        r"
        \hbox(7.93446+4.41661)x11.43054
        .\mathon
        .\mi x
        .\vbox(10.40663+1.94444)x5.71527, shifted 2.47217
        ..\hbox(4.30554+0.0)x5.71527
        ...\mi x
        ..\kern1.79555
        ..\hbox(4.30554+1.94444)x5.2616
        ...\mi y
        .\mathoff",
    );
}

#[test]
fn a_binary_operator_gets_medium_space_and_italic_correction_follows() {
    assert_lines(
        &formula("x+y"),
        r"
        \hbox(5.83333+1.94444)x23.199
        .\mathon
        .\mi x
        .\glue(\medmuskip) 2.22217 plus 1.11108 minus 2.22217
        .\rm +
        .\glue(\medmuskip) 2.22217 plus 1.11108 minus 2.22217
        .\mi y
        .\kern0.35878
        .\mathoff",
    );
}

#[test]
fn a_display_fraction_centers_on_the_axis() {
    assert_lines(
        &formula(r"\displaystyle{x\over y}"),
        r"
        \hbox(11.07062+8.80396)x5.71527
        .\mathon
        .\hbox(11.07062+8.80396)x5.71527
        ..\hbox(11.07062+8.80396)x5.71527
        ...\hbox(0.0+0.0)x0.0, shifted -2.5
        ...\vbox(11.07062+8.80396)x5.71527
        ....\hbox(4.30554+0.0)x5.71527
        .....\mi x
        ....\kern4.06508
        ....\rule(0.39998+0.0)x*
        ....\kern4.85397
        ....\hbox(4.30554+1.94444)x5.71527, glue set 0.22684fil
        .....\glue 0.0 plus 1.0fil minus 1.0fil
        .....\mi y
        .....\kern0.35878
        .....\glue 0.0 plus 1.0fil minus 1.0fil
        ...\hbox(0.0+0.0)x0.0, shifted -2.5
        .\mathoff",
    );
}

#[test]
fn left_and_right_build_extensible_delimiters() {
    assert_lines(
        &formula(r"\left(\vrule height 20pt depth 10pt\right)"),
        r"
        \hbox(20.50017+15.50017)x17.90002
        .\mathon
        .\hbox(20.50017+15.50017)x17.90002
        ..\vbox(0.39998+35.60036)x8.75002, shifted -20.10019
        ...\hbox(0.39998+17.60019)x8.75002
        ....\ex 0
        ...\hbox(0.39998+17.60019)x8.75002
        ....\ex @
        ..\rule(20.0+10.0)x0.4
        ..\vbox(0.39998+35.60036)x8.75002, shifted -20.10019
        ...\hbox(0.39998+17.60019)x8.75002
        ....\ex 1
        ...\hbox(0.39998+17.60019)x8.75002
        ....\ex A
        .\mathoff",
    );
}

const EVERY_NOAD: &str = r#"\mathop{x}\limits^y_x \radical"270370{x^y} \mathaccent"7015 x_y
    \overline{x}\underline{y}\vcenter{}\mathpunct{x}\left(x\middle)x\over y"#;

#[test]
fn every_noad_kind_shows_its_fields() {
    let r = run(&format!(
        "{MATH_FONTS}\\setbox0\\hbox{{${EVERY_NOAD}\\showlists\\right)$}}"
    ));
    let start = r.log.find("### math mode").expect("the lists were shown");
    let end = r.log[start..].find("! OK").expect("the show ended") + start;
    assert_lines(
        r.log[start..end].trim_end(),
        r#"
        ### math mode entered at line 13
        \mathord
        .\fam1 y
        this will begin denominator of:
        \fraction, thickness = default
        \\left"28300
        \\mathord
        \.\fam1 x
        \\middle"29301
        \\mathord
        \.\fam1 x
        ### math mode entered at line 12
        \mathop\limits
        .\fam1 x
        ^\fam1 y
        _\fam1 x
        \radical"270370
        .\mathord
        ..\fam1 x
        .^\fam1 y
        \accent\fam0 ^^U
        .\fam1 x
        _\fam1 y
        \overline
        .\fam1 x
        \underline
        .\fam1 y
        \vcenter
        .\vbox(0.0+0.0)x0.0
        \mathpunct
        .\fam1 x
        ### restricted horizontal mode entered at line 12
        spacefactor 1000
        ### vertical mode entered at line 0
        prevdepth ignored"#,
    );
}

#[test]
fn every_noad_kind_typesets() {
    assert_lines(
        &formula(&format!(r"{EVERY_NOAD}\right)")),
        r"
        \hbox(13.01387+7.0)x82.04173
        .\mathon
        .\vbox(13.01387+7.0)x5.71527
        ..\kern1.0
        ..\hbox(4.30554+1.94444)x5.71527, glue set 0.22684fil
        ...\glue 0.0 plus 1.0fil minus 1.0fil
        ...\mi y
        ...\kern0.35878
        ...\glue 0.0 plus 1.0fil minus 1.0fil
        ..\kern1.11111
        ..\hbox(4.65277+0.0)x5.71527
        ...\hbox(4.30554+0.0)x5.71527, shifted -0.34723
        ....\mi x
        ..\kern1.69446
        ..\hbox(4.30554+0.0)x5.71527
        ...\mi x
        ..\kern1.0
        .\glue(\thinmuskip) 0.0
        .\hbox(9.51314+0.88683)x19.31023
        ..\hbox(0.39998+9.6)x8.33336, shifted -8.71318
        ...\sy p
        ..\vbox(9.51314+0.0)x10.97687
        ...\kern0.39998
        ...\rule(0.39998+0.0)x*
        ...\kern1.38681
        ...\hbox(7.32637+0.0)x10.97687
        ....\mi x
        ....\hbox(4.30554+1.94444)x5.2616, shifted -3.02083
        .....\mi y
        .\vbox(6.94444+3.44443)x10.97687
        ..\hbox(6.94444+0.0)x0.0, shifted 0.35764
        ...\rm ^^U
        ..\kern-4.30554
        ..\hbox(4.30554+3.44443)x10.97687
        ...\mi x
        ...\hbox(4.30554+1.94444)x5.2616, shifted 1.49998
        ....\mi y
        .\vbox(6.30544+0.0)x5.71527
        ..\kern0.39998
        ..\rule(0.39998+0.0)x*
        ..\kern1.19994
        ..\hbox(4.30554+0.0)x5.71527
        ...\mi x
        .\vbox(4.30554+3.94434)x5.2616
        ..\hbox(4.30554+1.94444)x5.2616
        ...\mi y
        ..\kern1.19994
        ..\rule(0.39998+0.0)x*
        .\vbox(2.5+-2.5)x0.0
        .\mi x
        .\glue(\thinmuskip) 0.0
        .\hbox(11.50008+6.50009)x29.34723
        ..\hbox(0.39998+17.60019)x5.97223, shifted -11.1001
        ...\ex ^^P
        ..\mi x
        ..\hbox(0.39998+17.60019)x5.97223, shifted -11.1001
        ...\ex ^^Q
        ..\hbox(8.24286+5.39285)x5.71527
        ...\hbox(0.0+0.0)x0.0, shifted -2.5
        ...\vbox(8.24286+5.39285)x5.71527
        ....\hbox(4.30554+0.0)x5.71527
        .....\mi x
        ....\kern1.23732
        ....\rule(0.39998+0.0)x*
        ....\kern1.44287
        ....\hbox(4.30554+1.94444)x5.71527, glue set 0.22684fil
        .....\glue 0.0 plus 1.0fil minus 1.0fil
        .....\mi y
        .....\kern0.35878
        .....\glue 0.0 plus 1.0fil minus 1.0fil
        ...\hbox(0.0+0.0)x0.0, shifted -2.5
        ..\hbox(0.39998+17.60019)x5.97223, shifted -11.1001
        ...\ex ^^Q
        .\mathoff",
    );
}

#[test]
fn a_display_after_a_wrapped_prevgraf_takes_the_last_parshape_line() {
    let r = run(&format!(
        "{MATH_FONTS}\\setbox0\\vbox{{\\parshape 2 5pt 30pt 7pt 50pt \\noindent
         \\prevgraf=2147483647 $$\\showthe\\displayindent \\showthe\\displaywidth$$}}"
    ));
    assert_eq!(r.shown(), ["7.0pt", "50.0pt"]);
}

#[test]
fn a_style_change_picks_the_mathchoice_branch() {
    let r = run(&format!(
        "{MATH_FONTS}\\medmuskip=4mu \\setbox0\\hbox{{$\\scriptstyle\\mathchoice{{x}}{{x}}{{y}}{{x}}+x\\showlists$}}\\showbox0"
    ));
    assert_eq!(r.lines_starting("\\scriptstyle"), [r"\scriptstyle"]);
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(5.83333+1.94444)x18.75467
        .\mathon
        .\mi y
        .\kern0.35878
        .\rm +
        .\mi x
        .\mathoff",
    );
}

#[test]
fn umathaccent_keywords_place_the_accent() {
    let r = run(&format!(
        "{MATH_FONTS}\\setbox0\\hbox{{$\\Umathaccent\"0\"1\"7F x\\Umathaccent bottom\"0\"1\"7F x\\showlists$}}\\showbox0"
    ));
    // WEB shows any noad subtype as limits, so the bottom accent reads `\nolimits`.
    assert_eq!(
        r.lines_starting("\\accent"),
        [r"\accent\fam1 ^^?", r"\accent\fam1 ^^?\nolimits"]
    );
    assert_lines(
        &r.boxes()[0],
        r"
        \hbox(6.94444+6.94444)x11.43054
        .\mathon
        .\vbox(6.94444+0.0)x5.71527
        ..\hbox(6.94444+0.0)x0.0, shifted -0.52856
        ...\mi ^^?
        ..\kern-4.30554
        ..\hbox(4.30554+0.0)x5.71527
        ...\mi x
        .\vbox(11.24998+0.0)x5.71527, shifted 6.94444
        ..\hbox(4.30554+0.0)x5.71527
        ...\mi x
        ..\hbox(6.94444+0.0)x0.0, shifted -0.52856
        ...\mi ^^?
        .\mathoff",
    );
}

#[test]
fn an_opentype_accent_over_a_tfm_character_takes_its_skew() {
    // XeTeX aborts here with "bad native font flag" (upstream bug 24).
    let r = run(&format!(
        "{MATH_FONTS}\\font\\lm=\"[lmroman12-regular.otf]\" \\textfont4=\\lm \\scriptfont4=\\lm
         \\scriptscriptfont4=\\lm \\def\\a{{$\\Umathaccent0 4 \"7E x$}}
         \\setbox0\\hbox{{\\a}}\\skewchar\\mi=127 \\setbox1\\hbox{{\\a}}\\showbox0 \\showbox1"
    ));
    let accent = |shift: &str| {
        format!(
            r"
        \hbox(4.30554+0.0)x5.71527
        .\mathon
        .\vbox(4.30554+0.0)x5.71527
        ..\kern0.60773
        ..\hbox(3.69781+0.0)x0.0, shifted {shift}
        ...\lm glyph#31
        ..\kern-4.30554
        ..\hbox(4.30554+0.0)x5.71527
        ...\mi x
        .\mathoff"
        )
    };
    // cmmi10 kerns x against its skew character 127 by 0.27779pt.
    assert_lines(&r.boxes()[0], &accent("-0.49088"));
    assert_lines(&r.boxes()[1], &accent("-0.21309"));
}
