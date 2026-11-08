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
