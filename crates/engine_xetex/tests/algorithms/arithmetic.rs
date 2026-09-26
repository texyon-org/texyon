//! Register arithmetic: TeX's explicit overflow checks, and the 32-bit
//! wraparound everywhere else.

use crate::{run, show_the};

#[test]
fn advance_wraps_around() {
    let shown = show_the(
        r"\count1=2147483647 \advance\count1 by 1 \count2=-\count1
          \dimen1=16383pt \advance\dimen1 by 16383pt",
        &[r"\count1", r"\count2", r"\dimen1"],
    );
    assert_eq!(shown, ["-2147483648", "-2147483648", "32766.0pt"]);
}

#[test]
fn multiply_and_divide_check_for_overflow() {
    let r = run(
        r"\count1=1073741824 \multiply\count1 by 2 \count2=7 \divide\count2 by 0
          \count3=-7 \divide\count3 by 2
          \showthe\count1 \showthe\count2 \showthe\count3",
    );
    assert_eq!(r.errors(), ["Arithmetic overflow.", "Arithmetic overflow."]);
    assert_eq!(r.shown(), ["1073741824", "7", "-3"]);
}

#[test]
fn box_dimensions_wrap_around() {
    let shown = show_the(
        r"\setbox1\hbox{\hskip 16383pt \hskip 16383pt \hskip 16383pt \hskip 16383pt}
          \setbox2\vbox{\kern 16383pt \kern 16383pt}",
        &[r"\wd1", r"\ht2"],
    );
    assert_eq!(shown, ["-4.0pt", "32766.0pt"]);
}

#[test]
fn random_numbers_accept_the_most_negative_integer() {
    let shown = show_the(
        r"\count1=2147483647 \advance\count1 by 1 \setrandomseed\count1",
        &[r"\randomseed", r"\numexpr\uniformdeviate\count1\relax"],
    );
    assert_eq!(shown, ["-2147483648", "-2147483647"]);
}
