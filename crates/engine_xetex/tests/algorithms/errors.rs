//! Error reporting.

use crate::run_with;

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
