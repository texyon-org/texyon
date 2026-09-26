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
