//! Diagnostic warnings: e-TeX group/if/file nesting, fonts, UTF-8 and missing characters.

use super::cmd_chr::print_command;
use super::diagnostic::{begin_warning, end_capture};
use super::errors::{Abort, error};
use super::show::show_context;
use super::{
    Selector, TexPrint, begin_diagnostic, end_diagnostic, note_warning_issued, print, print_code,
    print_esc, print_group, print_hex, print_int, print_ln, print_nl, print_raw_bytes,
    print_ucs_code,
};
use crate::base::codes::FiOrElseCode;
use crate::base::command::Command;
use crate::base::params::IntPar;
use crate::base::units::FontId;
use crate::ffi::bridge::issue_warning;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::scanning::input::{InputName, TOKEN_LIST};
use crate::state::EngineState;

/// Walks `base_ptr` down to the file at input level `level`, reporting whether it is a real file.
fn level_is_file(st: &mut EngineState, level: usize) -> bool {
    while st.input.input_stack[st.input.base_ptr].state == TOKEN_LIST
        || usize::from(st.input.input_stack[st.input.base_ptr].index) > level
    {
        st.input.base_ptr -= 1;
    }
    st.input.input_stack[st.input.base_ptr].name.reads_lines()
}

pub(crate) fn group_warning(st: &mut EngineState) {
    st.input.base_ptr = st.input.input_stack.len();
    st.input.input_stack.park(st.input.cur_input);
    let mut i = st.input.in_open;
    let mut w = false;
    while st.input.open_levels[i].grp == st.save_stack.cur_boundary && i > 0 {
        if st.eqtb.int_par(IntPar::TracingNesting) > 0 && level_is_file(st, i) {
            w = true;
        }
        if let Some((_, below, _)) = st.save_stack.boundary() {
            st.input.open_levels[i].grp = below;
        }
        i -= 1;
    }
    if w {
        print_nl(st, "Warning: ");
        begin_warning(st);
        print(st, "end of ");
        print_group(st, true);
        print(st, " of a different file");
        print_ln(st);
        if st.eqtb.int_par(IntPar::TracingNesting) > 1 {
            show_context(st);
        }
        note_warning_issued(st);
        end_capture(st);
    }
}

pub(crate) fn if_warning(st: &mut EngineState) {
    st.input.base_ptr = st.input.input_stack.len();
    st.input.input_stack.park(st.input.cur_input);
    let mut i = st.input.in_open;
    let mut w = false;
    while st.input.open_levels[i].cond == st.scanner.conds.top() {
        if st.eqtb.int_par(IntPar::TracingNesting) > 0 && level_is_file(st, i) {
            w = true;
        }
        st.input.open_levels[i].cond = st.scanner.conds.below_top();
        i -= 1;
    }
    if w {
        print_nl(st, "Warning: ");
        begin_warning(st);
        print(st, "end of ");
        print_command(st, Command::IfTest(st.scanner.cur_if));
        print_if_line(st, st.scanner.if_line);
        print(st, " of a different file");
        print_ln(st);
        if st.eqtb.int_par(IntPar::TracingNesting) > 1 {
            show_context(st);
        }
        end_capture(st);
        note_warning_issued(st);
    }
}

pub(super) fn print_if_line(st: &mut EngineState, line: i32) {
    if line != 0 {
        print(st, " entered on line ");
        print_int(st, line);
    }
}

/// Steps from the current group boundary to the enclosing one, false if none.
pub(super) fn leave_group_boundary(st: &mut EngineState) -> bool {
    let ss = &mut st.save_stack;
    let Some((group, below, _)) = ss.boundary() else {
        return false;
    };
    (ss.cur_group, ss.cur_boundary) = (group, below);
    true
}

pub(crate) fn file_warning(st: &mut EngineState) {
    let saved_boundary = st.save_stack.cur_boundary;
    let saved_level = st.save_stack.cur_level;
    let saved_group = st.save_stack.cur_group;
    while st.input.open_levels[st.input.in_open].grp != st.save_stack.cur_boundary {
        st.save_stack.cur_level -= 1;
        print_nl(st, "Warning: ");
        begin_warning(st);
        print(st, "end of file when ");
        print_group(st, true);
        print(st, " is incomplete");
        end_capture(st);
        if !leave_group_boundary(st) {
            break;
        }
    }
    st.save_stack.cur_boundary = saved_boundary;
    st.save_stack.cur_level = saved_level;
    st.save_stack.cur_group = saved_group;

    let saved_limit = st.scanner.if_limit;
    let saved_if = st.scanner.cur_if;
    let saved_line = st.scanner.if_line;
    let opened_in = st.input.open_levels[st.input.in_open].cond;
    let saved: Vec<_> = st.scanner.conds.iter_from_top().copied().collect();
    let mut outer = saved.iter();
    let mut cur = st.scanner.conds.top();
    while opened_in != cur {
        print_nl(st, "Warning: ");
        begin_warning(st);
        print(st, "end of file when ");
        print_command(st, Command::IfTest(st.scanner.cur_if));
        if st.scanner.if_limit == FiOrElseCode::Fi {
            print_esc(st, "else");
        }
        print_if_line(st, st.scanner.if_line);
        print(st, " is incomplete");
        end_capture(st);
        let Some(cond) = outer.next() else {
            break;
        };
        (st.scanner.cur_if, st.scanner.if_line, st.scanner.if_limit) =
            (cond.cur_if, cond.if_line, cond.if_limit);
        cur = outer.as_slice().first().map(|c| c.id);
    }
    st.scanner.if_limit = saved_limit;
    st.scanner.cur_if = saved_if;
    st.scanner.if_line = saved_line;

    print_ln(st);
    if st.eqtb.int_par(IntPar::TracingNesting) > 1 {
        show_context(st);
    }
    note_warning_issued(st);
}

pub(crate) fn font_feature_warning(
    st: &mut EngineState,
    feature: &[u8],
    setting: &[u8],
    font_name: &[u8],
) {
    begin_diagnostic(st);
    begin_warning(st);
    print_nl(st, "Unknown ");
    if !setting.is_empty() {
        print(st, "selector `");
        print_raw_bytes(st, setting);
        print(st, "' for ");
    }
    print(st, "feature `");
    print_raw_bytes(st, feature);
    print(st, "' in font `");
    print_raw_bytes(st, font_name);
    print(st, "'.");
    end_capture(st);
    end_diagnostic(st, false);
}

pub(crate) fn font_mapping_warning(
    st: &mut EngineState,
    mapping_name: &[u8],
    font_name: &[u8],
    warning_type: i32,
) {
    begin_diagnostic(st);
    // A loaded mapping is font tracing, not a problem.
    if warning_type == 0 {
        print_nl(st, "Loaded mapping `");
    } else {
        begin_warning(st);
        print_nl(st, "Font mapping `");
    }
    print_raw_bytes(st, mapping_name);
    print(st, "' for font `");
    print_raw_bytes(st, font_name);
    match warning_type {
        1 => print(st, "' not found."),
        2 => {
            print(st, "' not usable;");
            print_nl(st, "bad mapping file or incorrect mapping type.");
        }
        _ => print(st, "'."),
    }
    end_capture(st);
    end_diagnostic(st, false);
}

pub(crate) fn bad_utf8_warning(st: &mut EngineState) {
    begin_diagnostic(st);
    begin_warning(st);
    print_nl(st, "Invalid UTF-8 byte or sequence");
    if st.input.cur_input.name == InputName::Terminal {
        print(st, " in terminal input");
    } else {
        print(st, " at line ");
        print_int(st, st.mem.line);
    }
    print(st, " replaced by U+FFFD.");
    end_capture(st);
    end_diagnostic(st, false);
}

/// "<char> (<code>) in font <name>", the character a font lacks.
#[derive(Copy, Clone)]
struct LostChar {
    font: FontId,
    c: i32,
}

impl TexPrint for LostChar {
    fn print_to(&self, st: &mut EngineState) {
        print_code(st, self.c);
        print(st, " (");
        let area = st.fonts.font_area[self.font];
        if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
            print_ucs_code(st, self.c);
        } else {
            print_hex(st, self.c);
        }
        print(st, (')', " in font ", st.fonts.font_name[self.font]));
    }
}

pub(crate) fn char_warning(st: &mut EngineState, f: FontId, c: i32) -> Result<(), Abort> {
    let tracing_lost = st.eqtb.int_par(IntPar::TracingLostChars);
    if tracing_lost > 0 {
        let old_setting = st.eqtb.int_par(IntPar::TracingOnline);
        if tracing_lost > 1 {
            st.eqtb.set_int_par(IntPar::TracingOnline, 1);
        }
        let lost = LostChar { font: f, c };
        if tracing_lost > 2 {
            st.eqtb.set_int_par(IntPar::TracingOnline, old_setting);
            error(st, ("Missing character: There is no ", lost), &[])?;
        } else {
            begin_diagnostic(st);
            begin_warning(st);
            print_nl(st, "Missing character: There is no ");
            print(st, (lost, '!'));
            st.eqtb.set_int_par(IntPar::TracingOnline, old_setting);
            end_capture(st);
            end_diagnostic(st, false);
        }
    }

    let font_name = st.strings.texstring_bytes(st.fonts.font_name[f]);
    let prev_sel = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    print_code(st, c);
    st.shipout.selector = prev_sel;
    let s = st.strings.make_string()?;
    let chr = st.strings.texstring_bytes(s);
    st.strings.flush_string();
    issue_warning(&format!(
        "could not represent character \"{}\" (0x{:x}) in font \"{}\"",
        String::from_utf8_lossy(&chr),
        c as u32,
        String::from_utf8_lossy(&font_name),
    ));
    if !st.fonts.gave_char_warning_help {
        issue_warning(
            "  you may need to load the `fontspec` package and use (e.g.) \\setmainfont to",
        );
        issue_warning("  choose a different font that covers the unrepresentable character(s)");
        st.fonts.gave_char_warning_help = true;
    }
    Ok(())
}
