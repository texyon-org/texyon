//! Error reporting.

use crate::{run, run_with};

#[test]
fn halt_on_error_still_logs_the_help() {
    let r = run_with("\\undefined\n\\message{after}", true);
    assert_eq!(r.errors(), ["Undefined control sequence."]);
    assert_eq!(
        r.lines_starting("The control sequence"),
        ["The control sequence at the end of the top line"]
    );
    assert!(!r.log.contains("after"));
}

#[test]
fn partokenname_takes_no_prefix_and_ends_assignments() {
    let r = run(r"\count1=0 \def\p{\advance\count1 by 1 }
\global\partokenname\p

\font\x=cmr10 \x \accent65 \partokenname\p x\par

\showthe\count1");
    assert_eq!(
        r.errors(),
        ["You can't use a prefix with `\\partokenname'."]
    );
    assert_eq!(r.shown(), ["2"]);
}

#[test]
fn a_bad_prevgraf_reports_the_message_as_logged() {
    let r = run(r"\prevgraf=-1");
    assert_eq!(r.errors(), ["Bad \\prevgraf (-1)."]);
    assert_eq!(r.reports, ["texput.tex:5: Bad \\prevgraf (-1)"]);
}

#[test]
fn a_missing_delimiter_is_reported() {
    let r = run(r"$\left x\right.$");
    assert_eq!(
        r.reports,
        [
            "texput.tex:5: Missing delimiter (. inserted)",
            "texput.tex:5: Math formula deleted: Insufficient symbol fonts",
        ]
    );
}

#[test]
fn a_lost_character_is_reported_as_an_error() {
    let r = run(r"\tracinglostchars=3 \font\x=cmr10 \x \char150 \par");
    assert_eq!(
        r.errors(),
        ["Missing character: There is no ^^96 (\"96) in font cmr10."]
    );
    assert_eq!(
        r.reports,
        ["texput.tex:5: Missing character: There is no ^^96 (\"96) in font cmr10"]
    );
}

#[test]
fn dumping_inside_a_group_is_fatal() {
    let r = run(r"\begingroup\dump");
    assert_eq!(r.errors(), ["You can't dump inside a group."]);
    assert_eq!(r.reports, ["!You can't dump inside a group"]);
    assert!(!r.log.contains("Beginning to dump"));
}
