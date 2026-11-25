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
use crate::env::EngineEnv;
use crate::ffi::bridge::issue_warning;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::scanning::input::{InputName, TOKEN_LIST};
use crate::state::EngineState;

fn level_is_file(st: &mut EngineState, level: usize) -> bool {
    while st.input.input_stack[st.input.base_ptr].state == TOKEN_LIST
        || usize::from(st.input.input_stack[st.input.base_ptr].index) > level
    {
        st.input.base_ptr -= 1;
    }
    st.input.input_stack[st.input.base_ptr].name.reads_lines()
}

pub(crate) fn group_warning(st: &mut EngineState, env: &mut EngineEnv) {
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
        print_nl(st, env, "Warning: ");
        begin_warning(st, env);
        print(st, env, "end of ");
        print_group(st, env, true);
        print(st, env, " of a different file");
        print_ln(st, env);
        if st.eqtb.int_par(IntPar::TracingNesting) > 1 {
            show_context(st, env);
        }
        note_warning_issued(st);
        end_capture(st, env);
    }
}

pub(crate) fn if_warning(st: &mut EngineState, env: &mut EngineEnv) {
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
        print_nl(st, env, "Warning: ");
        begin_warning(st, env);
        print(st, env, "end of ");
        print_command(st, env, Command::IfTest(st.scanner.cur_if));
        print_if_line(st, env, st.scanner.if_line);
        print(st, env, " of a different file");
        print_ln(st, env);
        if st.eqtb.int_par(IntPar::TracingNesting) > 1 {
            show_context(st, env);
        }
        end_capture(st, env);
        note_warning_issued(st);
    }
}

pub(super) fn print_if_line(st: &mut EngineState, env: &mut EngineEnv, line: i32) {
    if line != 0 {
        print(st, env, " entered on line ");
        print_int(st, env, line);
    }
}

pub(super) fn leave_group_boundary(st: &mut EngineState) -> bool {
    let ss = &mut st.save_stack;
    let Some((group, below, _)) = ss.boundary() else {
        return false;
    };
    (ss.cur_group, ss.cur_boundary) = (group, below);
    true
}

pub(crate) fn file_warning(st: &mut EngineState, env: &mut EngineEnv) {
    let saved_boundary = st.save_stack.cur_boundary;
    let saved_level = st.save_stack.cur_level;
    let saved_group = st.save_stack.cur_group;
    while st.input.open_levels[st.input.in_open].grp != st.save_stack.cur_boundary {
        st.save_stack.cur_level -= 1;
        print_nl(st, env, "Warning: ");
        begin_warning(st, env);
        print(st, env, "end of file when ");
        print_group(st, env, true);
        print(st, env, " is incomplete");
        end_capture(st, env);
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
        print_nl(st, env, "Warning: ");
        begin_warning(st, env);
        print(st, env, "end of file when ");
        print_command(st, env, Command::IfTest(st.scanner.cur_if));
        if st.scanner.if_limit == FiOrElseCode::Fi {
            print_esc(st, env, "else");
        }
        print_if_line(st, env, st.scanner.if_line);
        print(st, env, " is incomplete");
        end_capture(st, env);
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

    print_ln(st, env);
    if st.eqtb.int_par(IntPar::TracingNesting) > 1 {
        show_context(st, env);
    }
    note_warning_issued(st);
}

pub(crate) fn font_feature_warning(
    st: &mut EngineState,
    env: &mut EngineEnv,
    feature: &[u8],
    setting: &[u8],
    font_name: &[u8],
) {
    begin_diagnostic(st);
    begin_warning(st, env);
    print_nl(st, env, "Unknown ");
    if !setting.is_empty() {
        print(st, env, "selector `");
        print_raw_bytes(st, env, setting);
        print(st, env, "' for ");
    }
    print(st, env, "feature `");
    print_raw_bytes(st, env, feature);
    print(st, env, "' in font `");
    print_raw_bytes(st, env, font_name);
    print(st, env, "'.");
    end_capture(st, env);
    end_diagnostic(st, env, false);
}

pub(crate) fn font_mapping_warning(
    st: &mut EngineState,
    env: &mut EngineEnv,
    mapping_name: &[u8],
    font_name: &[u8],
    warning_type: i32,
) {
    begin_diagnostic(st);
    // A loaded mapping is font tracing, not a problem.
    if warning_type == 0 {
        print_nl(st, env, "Loaded mapping `");
    } else {
        begin_warning(st, env);
        print_nl(st, env, "Font mapping `");
    }
    print_raw_bytes(st, env, mapping_name);
    print(st, env, "' for font `");
    print_raw_bytes(st, env, font_name);
    match warning_type {
        1 => print(st, env, "' not found."),
        2 => {
            print(st, env, "' not usable;");
            print_nl(st, env, "bad mapping file or incorrect mapping type.");
        }
        _ => print(st, env, "'."),
    }
    end_capture(st, env);
    end_diagnostic(st, env, false);
}

pub(crate) fn bad_utf8_warning(st: &mut EngineState, env: &mut EngineEnv) {
    begin_diagnostic(st);
    begin_warning(st, env);
    print_nl(st, env, "Invalid UTF-8 byte or sequence");
    if st.input.cur_input.name == InputName::Terminal {
        print(st, env, " in terminal input");
    } else {
        print(st, env, " at line ");
        print_int(st, env, st.mem.line);
    }
    print(st, env, " replaced by U+FFFD.");
    end_capture(st, env);
    end_diagnostic(st, env, false);
}

#[derive(Copy, Clone)]
struct LostChar {
    font: FontId,
    c: i32,
}

impl TexPrint for LostChar {
    fn print_to(&self, st: &mut EngineState, env: &mut EngineEnv) {
        print_code(st, env, self.c);
        print(st, env, " (");
        let area = st.fonts.font_area[self.font];
        if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
            print_ucs_code(st, env, self.c);
        } else {
            print_hex(st, env, self.c);
        }
        print(st, env, (')', " in font ", st.fonts.font_name[self.font]));
    }
}

pub(crate) fn char_warning(
    st: &mut EngineState,
    env: &mut EngineEnv,
    f: FontId,
    c: i32,
) -> Result<(), Abort> {
    let tracing_lost = st.eqtb.int_par(IntPar::TracingLostChars);
    if tracing_lost > 0 {
        let old_setting = st.eqtb.int_par(IntPar::TracingOnline);
        if tracing_lost > 1 {
            st.eqtb.set_int_par(IntPar::TracingOnline, 1);
        }
        let lost = LostChar { font: f, c };
        if tracing_lost > 2 {
            st.eqtb.set_int_par(IntPar::TracingOnline, old_setting);
            error(st, env, ("Missing character: There is no ", lost), &[])?;
        } else {
            begin_diagnostic(st);
            begin_warning(st, env);
            print_nl(st, env, "Missing character: There is no ");
            print(st, env, (lost, '!'));
            st.eqtb.set_int_par(IntPar::TracingOnline, old_setting);
            end_capture(st, env);
            end_diagnostic(st, env, false);
        }
    }

    let font_name = st.strings.texstring_bytes(st.fonts.font_name[f]);
    let prev_sel = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    print_code(st, env, c);
    st.shipout.selector = prev_sel;
    let s = st.strings.make_string()?;
    let chr = st.strings.texstring_bytes(s);
    st.strings.flush_string();
    issue_warning(
        env,
        &format!(
            "could not represent character \"{}\" (0x{:x}) in font \"{}\"",
            String::from_utf8_lossy(&chr),
            c as u32,
            String::from_utf8_lossy(&font_name),
        ),
    );
    if !st.fonts.gave_char_warning_help {
        issue_warning(
            env,
            "  you may need to load the `fontspec` package and use (e.g.) \\setmainfont to",
        );
        issue_warning(
            env,
            "  choose a different font that covers the unrepresentable character(s)",
        );
        st.fonts.gave_char_warning_help = true;
    }
    Ok(())
}
