//! Showing internal state: `\show...`, `\message`, the semantic nest and the error context.

use super::cmd_chr::{print_command, print_meaning};
use super::diagnostic::{DiagnosticKind, begin_capture, end_capture};
use super::errors::{Abort, InteractionMode, errmessage, finish_error};
use super::show_box::show_box;
use super::warnings::{leave_group_boundary, print_if_line};
use super::{
    Selector, begin_diagnostic, end_diagnostic, print, print_char, print_cs, print_error_prefix,
    print_esc, print_group, print_int, print_ln, print_mode, print_nl, print_raw_char,
    print_scaled, sprint_cs, token_show,
};
use crate::base::cmd::Cmd;
use crate::base::codes::{FiOrElseCode, LeaderCode, ShowCode, TheCode};
use crate::base::command::Command;
use crate::base::consts::IGNORE_DEPTH;
use crate::base::mode::{Mode, ModeName};
use crate::base::params::{IntPar, TokPar};
use crate::base::scaled_math::x_over_n;
use crate::base::token::{TokCursor, Token, TokenKind};
use crate::base::units::{NodeType, Scaled};
use crate::build::main_control::BoxContext;
use crate::build::packaging::PackSpec;
use crate::build::pagebuilder::PageContents;
use crate::env::EngineEnv;
use crate::ffi::bridge::output_flush;
use crate::memory::mem::{CONTRIB_HEAD, PAGE_HEAD, PAGE_INS_HEAD};
use crate::memory::node::{AnyNode, NodePtr, PageInsNode};
use crate::scanning::input::{InputName, TOKEN_LIST, TokenType, level_start};
use crate::scanning::scan::{fetch_box, get_token, scan_register_num, scan_toks, the_toks};
use crate::shipout::ensure_write_stream_open;
use crate::state::EngineState;
use crate::tables::save::{GroupCode, Saved};

pub(crate) fn show_cur_cmd_chr(st: &mut EngineState, env: &mut EngineEnv) {
    begin_diagnostic(st);
    print_nl(st, env, b'{');
    let mode = st.input.cur_list.mode.name();
    if mode != st.scanner.shown_mode {
        print_mode(st, env, mode);
        print(st, env, ": ");
        st.scanner.shown_mode = mode;
    }
    print_command(st, env, st.scanner.cur);
    if st.eqtb.int_par(IntPar::TracingIfs) > 0
        && (Cmd::IfTest..=Cmd::FiOrElse).contains(&st.scanner.cur.cmd())
    {
        print(st, env, ": ");
        let (mut n, l) = if st.scanner.cur.cmd() == Cmd::FiOrElse {
            print_command(st, env, Command::IfTest(st.scanner.cur_if));
            print_char(st, env, b' ');
            (0, st.scanner.if_line)
        } else {
            (1, st.mem.line)
        };
        n += i32::try_from(st.scanner.conds.depth()).unwrap_or(i32::MAX);
        print(st, env, "(level ");
        print_int(st, env, n);
        print_char(st, env, b')');
        print_if_line(st, env, l);
    }
    print_char(st, env, b'}');
    end_diagnostic(st, env, false);
}

/// Records where the error context splits, when printing reaches the current position.
fn set_trick_count(st: &mut EngineState) {
    st.errors.first_count = st.errors.tally;
    let tc = st.errors.tally + 1 + st.errors.error_line - st.errors.half_error_line;
    st.errors.trick_count = tc.max(st.errors.error_line);
}

/// Shows the tokens from `p`, marking where `q` starts for `show_context`'s two-line display.
pub(crate) fn show_token_list(
    st: &mut EngineState,
    env: &mut EngineEnv,
    p: impl Into<TokCursor>,
    q: TokCursor,
    l: i32,
) {
    let mut p = p.into();
    let mut shown = ShownTokens::default();
    st.errors.tally = 0;
    while !p.is_end() && st.errors.tally < l {
        if p == q {
            set_trick_count(st);
        }
        if !st.mem.tokens.holds(p) {
            print_esc(st, env, "CLOBBERED.");
            return;
        }
        let Some(w) = st.mem.tokens.read(&mut p) else {
            break;
        };
        if !shown.show(st, env, w) {
            return;
        }
    }
    if !p.is_end() {
        print_esc(st, env, "ETC.");
    }
}

pub(crate) fn show_tokens(st: &mut EngineState, env: &mut EngineEnv, tokens: &[Token], l: i32) {
    let mut shown = ShownTokens::default();
    st.errors.tally = 0;
    for &w in tokens {
        if st.errors.tally >= l {
            print_esc(st, env, "ETC.");
            return;
        }
        if !shown.show(st, env, w) {
            return;
        }
    }
}

struct ShownTokens {
    match_chr: i32,
    n: i32,
}

impl Default for ShownTokens {
    fn default() -> Self {
        Self {
            match_chr: i32::from(b'#'),
            n: i32::from(b'0'),
        }
    }
}

impl ShownTokens {
    fn show(&mut self, st: &mut EngineState, env: &mut EngineEnv, w: Token) -> bool {
        match w.kind() {
            TokenKind::Cs(cs) => print_cs(st, env, Some(cs)),
            _ if w.0 < 0 => print_esc(st, env, "BAD."),
            TokenKind::Char(cmd, c) => match cmd {
                Cmd::LeftBrace
                | Cmd::RightBrace
                | Cmd::MathShift
                | Cmd::TabMark
                | Cmd::SupMark
                | Cmd::SubMark
                | Cmd::Spacer
                | Cmd::Letter
                | Cmd::OtherChar => print_char(st, env, c),
                Cmd::MacParam => {
                    print_char(st, env, c);
                    print_char(st, env, c);
                }
                Cmd::OUT_PARAM => {
                    print_char(st, env, self.match_chr);
                    if c > 9 {
                        print_char(st, env, b'!');
                        return false;
                    }
                    print_char(st, env, i32::from(b'0') + c);
                }
                Cmd::MATCH => {
                    self.match_chr = c;
                    print_char(st, env, c);
                    self.n += 1;
                    print_char(st, env, self.n);
                    if self.n > i32::from(b'9') {
                        return false;
                    }
                }
                Cmd::END_MATCH => {
                    if c == 0 {
                        print(st, env, "->");
                    }
                }
                _ => print_esc(st, env, "BAD."),
            },
        }
        true
    }
}

fn print_totals(st: &mut EngineState, env: &mut EngineEnv) {
    print_scaled(st, env, st.shipout.page_so_far[1]);
    for (i, order) in [(2, ""), (3, "fil"), (4, "fill"), (5, "filll")] {
        if st.shipout.page_so_far[i] != Scaled::ZERO {
            print(st, env, " plus ");
            print_scaled(st, env, st.shipout.page_so_far[i]);
            print(st, env, order);
        }
    }
    if st.shipout.page_so_far[6] != Scaled::ZERO {
        print(st, env, " minus ");
        print_scaled(st, env, st.shipout.page_so_far[6]);
    }
}

pub(crate) fn issue_message(
    st: &mut EngineState,
    env: &mut EngineEnv,
    err: bool,
) -> Result<(), Abort> {
    let text = scan_toks(st, env, false, true)?;
    let old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    token_show(st, env, text);
    st.shipout.selector = old_setting;
    st.mem.tokens.flush(text);
    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(st.strings.pool_overflow());
    }
    let s = st.strings.make_string()?;
    if !err {
        let len = st.strings.length(s);
        if st.shipout.term_offset + len > st.config.max_print_line - 2 {
            print_ln(st, env);
        } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
            print_char(st, env, b' ');
        }
        begin_capture(st, env, DiagnosticKind::Written);
        print(st, env, s);
        end_capture(st, env);
        output_flush(env, st.shipout.rust_stdout);
    } else {
        let help: &[&'static str] = if !st.eqtb.tok_par(TokPar::ErrHelp).is_null() {
            &[]
        } else if st.errors.long_help_seen {
            &[r"(That was another \errmessage.)"]
        } else {
            if st.errors.interaction < InteractionMode::ErrorStop {
                st.errors.long_help_seen = true;
            }
            &[
                r"This error message was generated by an \errmessage",
                "command, so I can't give any explicit help.",
                "Pretend that you're Hercule Poirot: Examine all clues,",
                "and deduce the truth by order and method.",
            ]
        };
        errmessage(st, env, s, help)?;
    }
    st.strings.flush_string();
    Ok(())
}

pub(crate) fn show_save_groups(st: &mut EngineState, env: &mut EngineEnv) {
    // Where the tex.web `found1`/`found2`/`found` label chain is entered.
    enum Entry {
        BoxContext,
        Found1,
        Found2,
        Found,
    }

    let mut p = st.input.nest.len();
    st.input.nest.park(st.input.cur_list);
    let saved_boundary = st.save_stack.cur_boundary;
    let saved_level = st.save_stack.cur_level;
    let saved_group = st.save_stack.cur_group;
    st.save_stack.cur_level -= 1;
    let mut a: i8 = 1;

    print_nl(st, env, "");
    print_ln(st, env);

    loop {
        print_nl(st, env, "### ");
        print_group(st, env, true);
        if st.save_stack.cur_group == GroupCode::BottomLevel {
            break;
        }

        // Find the enclosing non-paragraph mode, leaving `p` just below it.
        // A simple group steps `p` up, so it can pass the parked slot as in WEB.
        let m = loop {
            let cur_m = st.input.nest.stale_slot(p).mode.name();
            if p == 0 {
                break ModeName::Vertical;
            }
            p -= 1;
            if cur_m != ModeName::Horizontal {
                break cur_m;
            }
        };

        print(st, env, " (");

        let mut s: &str = "";
        let entry = match st.save_stack.cur_group {
            GroupCode::Simple => {
                p += 1;
                Entry::Found2
            }
            GroupCode::Hbox | GroupCode::AdjustedHbox => {
                s = "hbox";
                Entry::BoxContext
            }
            GroupCode::Vbox => {
                s = "vbox";
                Entry::BoxContext
            }
            GroupCode::Vtop => {
                s = "vtop";
                Entry::BoxContext
            }
            GroupCode::Align if a == 0 => {
                s = if m == ModeName::InternalVertical {
                    "halign"
                } else {
                    "valign"
                };
                a = 1;
                Entry::Found1
            }
            GroupCode::Align => {
                if a == 1 {
                    print(st, env, "align entry");
                } else {
                    print_esc(st, env, "cr");
                }
                match a {
                    1 if p >= 1 => p -= 1,
                    -1 => p += 1,
                    _ => {}
                }
                a = 0;
                Entry::Found
            }
            GroupCode::NoAlign => {
                p += 1;
                a = -1;
                print_esc(st, env, "noalign");
                Entry::Found2
            }
            GroupCode::Output => {
                print_esc(st, env, "output");
                Entry::Found
            }
            GroupCode::Math => Entry::Found2,
            GroupCode::Disc | GroupCode::MathChoice => {
                if st.save_stack.cur_group == GroupCode::Disc {
                    print_esc(st, env, "discretionary");
                } else {
                    print_esc(st, env, "mathchoice");
                }
                let braces = match st.save_stack.kept(1) {
                    Some(Saved::DiscPart(n) | Saved::ChoicePart(n)) => n,
                    _ => 0,
                };
                for i in 1..=3 {
                    if i <= braces {
                        print(st, env, "{}");
                    }
                }
                Entry::Found2
            }
            GroupCode::Insert => {
                let (n, pre) = match st.save_stack.kept(1) {
                    Some(Saved::Insert { n, pre }) => (n, pre),
                    _ => (0, false),
                };
                if n == 255 {
                    print_esc(st, env, "vadjust");
                    if pre {
                        print(st, env, " pre");
                    }
                } else {
                    print_esc(st, env, "insert");
                    print_int(st, env, n);
                }
                Entry::Found2
            }
            GroupCode::Vcenter => {
                s = "vcenter";
                Entry::Found1
            }
            GroupCode::SemiSimple => {
                p += 1;
                print_esc(st, env, "begingroup");
                Entry::Found
            }
            GroupCode::MathShift => {
                if m == ModeName::DisplayMath {
                    print_char(st, env, b'$');
                } else if st.input.nest.stale_slot(p).mode.name() == ModeName::DisplayMath {
                    let left = matches!(st.save_stack.kept(1), Some(Saved::EqNo { left: true }));
                    print_command(st, env, Command::EqNo { left });
                    print_char(st, env, b')');
                    st.save_stack.cur_level -= 1;
                    if !leave_group_boundary(st) {
                        break;
                    }
                    continue;
                }
                print_char(st, env, b'$');
                Entry::Found
            }
            GroupCode::MathLeft => {
                let delim = st.input.nest.stale_slot(p + 1).e_tex_aux;
                if delim.is_some_and(|d| st.mem.node_type(d) == Ok(NodeType::LeftNoad)) {
                    print_esc(st, env, "left");
                } else {
                    print_esc(st, env, "middle");
                }
                Entry::Found
            }
            GroupCode::BottomLevel => Entry::Found,
        };

        if matches!(entry, Entry::BoxContext) {
            match st.save_stack.kept(2) {
                Some(Saved::BoxContext(BoxContext::Shift(d))) if d != Scaled::ZERO => {
                    let negative = d < Scaled::ZERO;
                    let command = if st.input.nest.stale_slot(p).mode.is_vertical() {
                        Command::Hmove { left: negative }
                    } else {
                        Command::Vmove { raise: negative }
                    };
                    print_command(st, env, command);
                    print_scaled(st, env, d.abs());
                    print(st, env, "pt");
                }
                Some(Saved::BoxContext(BoxContext::SetBox { register, global })) => {
                    if global {
                        print_esc(st, env, "global");
                    }
                    print_esc(st, env, "setbox");
                    print_int(st, env, register.into());
                    print_char(st, env, b'=');
                }
                Some(Saved::BoxContext(BoxContext::ShipOut)) => {
                    print_command(st, env, Command::LeaderShip(LeaderCode::Shipout));
                }
                Some(Saved::BoxContext(BoxContext::Leaders(kind))) => {
                    print_command(st, env, Command::LeaderShip(LeaderCode::of_leaders(kind)));
                }
                _ => {}
            }
        }

        if matches!(entry, Entry::BoxContext | Entry::Found1) {
            print_esc(st, env, s);
            let (word, amount) = match st.save_stack.kept(1) {
                Some(Saved::Spec(PackSpec::Exactly(size))) => ("to", size),
                Some(Saved::Spec(PackSpec::Additional(extra))) => ("spread", extra),
                _ => ("spread", Scaled::ZERO),
            };
            if amount != Scaled::ZERO {
                print_char(st, env, b' ');
                print(st, env, word);
                print_scaled(st, env, amount);
                print(st, env, "pt");
            }
        }

        if !matches!(entry, Entry::Found) {
            print_char(st, env, b'{');
        }

        print_char(st, env, b')');
        st.save_stack.cur_level -= 1;
        if !leave_group_boundary(st) {
            break;
        }
    }

    st.save_stack.cur_boundary = saved_boundary;
    st.save_stack.cur_level = saved_level;
    st.save_stack.cur_group = saved_group;
}

pub(crate) fn show_whatever(
    st: &mut EngineState,
    env: &mut EngineEnv,
    code: ShowCode,
) -> Result<(), Abort> {
    match code {
        ShowCode::Lists => {
            show_to_stream(st, env)?;
            begin_diagnostic(st);
            show_activities(st, env);
        }
        ShowCode::Box => {
            let n = scan_register_num(st, env)?;
            let contents = fetch_box(st, n);
            show_to_stream(st, env)?;
            begin_diagnostic(st);
            print_nl(st, env, "> \\box");
            print_int(st, env, n.into());
            print_char(st, env, b'=');
            match contents {
                Some(b) => show_box(st, env, b.ptr()),
                None => print(st, env, "void"),
            }
        }
        ShowCode::Show => {
            get_token(st, env)?;
            show_to_stream(st, env)?;
            print_nl(st, env, "> ");
            if !st.scanner.cur_cs.is_none() {
                sprint_cs(st, env, st.scanner.cur_cs);
                print_char(st, env, b'=');
            }
            print_meaning(st, env);
        }
        ShowCode::Groups => {
            show_to_stream(st, env)?;
            begin_diagnostic(st);
            show_save_groups(st, env);
        }
        ShowCode::Ifs => {
            show_to_stream(st, env)?;
            begin_diagnostic(st);
            print_nl(st, env, "");
            print_ln(st, env);
            if st.scanner.conds.top().is_none() {
                print_nl(st, env, "### ");
                print(st, env, "no active conditionals");
            } else {
                let saved: Vec<_> = st.scanner.conds.iter_from_top().copied().collect();
                let mut n = i32::try_from(saved.len()).unwrap_or(i32::MAX);
                let mut t = st.scanner.cur_if;
                let mut l = st.scanner.if_line;
                let mut m = st.scanner.if_limit;
                for cond in &saved {
                    print_nl(st, env, "### level ");
                    print_int(st, env, n);
                    print(st, env, ": ");
                    print_command(st, env, Command::IfTest(t));
                    if m == FiOrElseCode::Fi {
                        print_esc(st, env, "else");
                    }
                    print_if_line(st, env, l);
                    n -= 1;
                    (t, l, m) = (cond.cur_if, cond.if_line, cond.if_limit);
                }
            }
        }
        ShowCode::The | ShowCode::Tokens => {
            let the = if code == ShowCode::Tokens {
                TheCode::Detokenize
            } else {
                TheCode::The
            };
            let list = st.mem.tokens.begin()?;
            the_toks(st, env, the, &list)?;
            let list = st.mem.tokens.finish(list)?;
            show_to_stream(st, env)?;
            print_nl(st, env, "> ");
            token_show(st, env, list);
            st.mem.tokens.flush(list);
        }
    }

    if matches!(
        code,
        ShowCode::Lists | ShowCode::Box | ShowCode::Groups | ShowCode::Ifs
    ) {
        end_diagnostic(st, env, true);
        print_error_prefix(st, env);
        print(st, env, "OK");
        if st.shipout.selector == Selector::TermAndLog
            && st.eqtb.int_par(IntPar::TracingOnline) <= 0
        {
            st.shipout.selector = Selector::TermOnly;
            print(st, env, " (see the transcript file)");
            st.shipout.selector = Selector::TermAndLog;
        }
    }

    if st.shipout.selector < Selector::NoPrint {
        print_ln(st, env);
        st.shipout.selector = if st.errors.interaction == InteractionMode::Batch {
            Selector::NoPrint
        } else {
            Selector::TermOnly
        };
        if st.errors.log_opened {
            st.shipout.selector = st.shipout.selector.with_log();
        }
        return Ok(());
    }
    if st.errors.interaction < InteractionMode::ErrorStop {
        st.errors.help(&[]);
        st.errors.error_count -= 1;
    } else if st.eqtb.int_par(IntPar::TracingOnline) > 0 {
        st.errors.help(&[
            r"This isn't an error message; I'm just \showing something.",
            r"Type `I\show...' to show more (e.g., \show\cs,",
            r"\showthe\count10, \showbox255, \showlists).",
        ]);
    } else {
        st.errors.help(&[
            r"This isn't an error message; I'm just \showing something.",
            r"Type `I\show...' to show more (e.g., \show\cs,",
            r"\showthe\count10, \showbox255, \showlists).",
            r"And type `I\tracingonline=1\show...' to show boxes and",
            "lists on your terminal as well as in the transcript file.",
        ]);
    }
    finish_error(st, env)
}

fn show_to_stream(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    let Ok(stream) = u8::try_from(st.eqtb.int_par(IntPar::ShowStream)) else {
        return Ok(());
    };
    let Some(&open) = st.shipout.write_open.get(usize::from(stream)) else {
        return Ok(());
    };
    if open && stream < Selector::NoPrint.raw() {
        ensure_write_stream_open(st, env, usize::from(stream))?;
        st.shipout.selector = Selector::File(stream);
    }
    Ok(())
}

fn show_page_so_far(st: &mut EngineState, env: &mut EngineEnv) {
    if st.shipout.page_tail != Some(PAGE_HEAD) {
        print_nl(st, env, "### current page:");
        if st.shipout.output_active {
            print(st, env, " (held over for next output)");
        }
        show_box(st, env, PAGE_HEAD.link(&st.mem));
        if st.shipout.page_contents != PageContents::Empty {
            print_nl(st, env, "total height ");
            print_totals(st, env);
            print_nl(st, env, " goal height ");
            print_scaled(st, env, st.shipout.page_so_far[0]);
            let mut r = PAGE_INS_HEAD.link(&st.mem);
            while let Some(here) = r.filter(|&x| x != PAGE_INS_HEAD) {
                let ins = NodePtr::<PageInsNode>::at(here);
                print_ln(st, env);
                print_esc(st, env, "insert");
                let box_num = st.mem[ins].box_number;
                print_int(st, env, i32::from(box_num));
                print(st, env, " adds ");
                let count = st.eqtb.count(box_num);
                let height = st.mem[ins].height;
                let adds = if count == 1000 {
                    height
                } else {
                    x_over_n(height, 1000).quotient * count
                };
                print_scaled(st, env, adds);
                if st.mem[ins].is_split_up() {
                    let broken_ins = st.mem[ins].broken_ins;
                    let mut q = PAGE_HEAD;
                    let mut t = 0;
                    while let Some(next) = q.link(&st.mem) {
                        q = next;
                        if let Ok(AnyNode::Ins(node)) = st.mem.classify(q)
                            && st.mem[node].box_number == box_num
                        {
                            t += 1;
                        }
                        if Some(q) == broken_ins {
                            break;
                        }
                    }
                    print(st, env, ", #");
                    print_int(st, env, t);
                    print(st, env, " might split");
                }
                r = here.link(&st.mem);
            }
        }
    }
    if !CONTRIB_HEAD.link(&st.mem).is_none() {
        print_nl(st, env, "### recent contributions:");
    }
}

fn show_activities(st: &mut EngineState, env: &mut EngineEnv) {
    st.input.nest.park(st.input.cur_list);
    print_nl(st, env, "");
    print_ln(st, env);
    for p in (0..=st.input.nest.len()).rev() {
        let nest = st.input.nest[p];
        print_nl(st, env, "### ");
        print_mode(st, env, nest.mode.name());
        print(st, env, " entered at line ");
        print_int(st, env, nest.mode_line.abs());
        if nest.mode.is_paragraph() && nest.prev_graf != 0x83_0000 {
            let pg = nest.prev_graf;
            print(st, env, " (language");
            print_int(st, env, pg % 65536);
            print(st, env, ":hyphenmin");
            print_int(st, env, pg / 0x40_0000);
            print_char(st, env, b',');
            print_int(st, env, (pg / 65536) % 64);
            print_char(st, env, b')');
        }
        if nest.mode_line < 0 {
            print(st, env, " (\\output routine)");
        }
        if p == 0 {
            show_page_so_far(st, env);
        }
        show_box(st, env, nest.head.link(&st.mem));

        match nest.mode {
            Mode::Vertical { prev_depth, .. } => {
                print_nl(st, env, "prevdepth ");
                if prev_depth <= IGNORE_DEPTH {
                    print(st, env, "ignored");
                } else {
                    print_scaled(st, env, prev_depth);
                }
                if nest.prev_graf != 0 {
                    print(st, env, ", prevgraf ");
                    print_int(st, env, nest.prev_graf);
                    if nest.prev_graf != 1 {
                        print(st, env, " lines");
                    } else {
                        print(st, env, " line");
                    }
                }
            }
            Mode::Horizontal {
                inner,
                space_factor,
                clang,
            } => {
                print_nl(st, env, "spacefactor ");
                print_int(st, env, space_factor);
                if !inner && clang > 0 {
                    print(st, env, ", current language ");
                    print_int(st, env, clang);
                }
            }
            Mode::Math {
                incompleat_noad, ..
            } => {
                if let Some(n) = incompleat_noad {
                    print(st, env, "this will begin denominator of:");
                    show_box(st, env, n.ptr());
                }
            }
            Mode::None => {}
        }
    }
}

fn begin_pseudoprint(st: &mut EngineState) -> i32 {
    let l = st.errors.tally;
    st.errors.tally = 0;
    st.shipout.selector = Selector::Pseudo;
    st.errors.trick_count = 1_000_000;
    l
}

pub(crate) fn show_context(st: &mut EngineState, env: &mut EngineEnv) {
    st.input.base_ptr = st.input.input_stack.len();
    st.input.input_stack.park(st.input.cur_input);
    let mut nn = -1;
    let mut bottom_line = false;

    loop {
        st.input.cur_input = st.input.input_stack[st.input.base_ptr];
        st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
        let ci = st.input.cur_input;
        if ci.state != TOKEN_LIST
            && (matches!(ci.name, InputName::File(_)) || st.input.base_ptr == 0)
        {
            bottom_line = true;
        }
        let error_context_lines = st.eqtb.int_par(IntPar::ErrorContextLines);
        if st.input.base_ptr == st.input.input_stack.len()
            || bottom_line
            || nn < error_context_lines
        {
            if st.input.base_ptr == st.input.input_stack.len()
                || ci.state != TOKEN_LIST
                || ci.token_type() != TokenType::BackedUp
                || !ci.token_loc().is_end()
            {
                st.errors.tally = 0;
                let old_setting = st.shipout.selector;
                let l;
                if ci.state != TOKEN_LIST {
                    match ci.name {
                        InputName::Terminal if st.input.base_ptr == 0 => print_nl(st, env, "<*>"),
                        InputName::Terminal => print_nl(st, env, "<insert> "),
                        InputName::Read(n) => {
                            print(st, env, (super::Nl("<read "), i32::from(n), '>'))
                        }
                        InputName::ReadTerminal => print_nl(st, env, "<read *>"),
                        _ => {
                            print_nl(st, env, "l.");
                            if usize::from(ci.index) == st.input.in_open {
                                print_int(st, env, st.mem.line);
                            } else {
                                print_int(
                                    st,
                                    env,
                                    st.input.open_levels[usize::from(ci.index) + 1].outer_line,
                                );
                            }
                        }
                    }
                    print_char(st, env, b' ');
                    l = begin_pseudoprint(st);
                    let end_line_char = st.eqtb.int_par(IntPar::EndLineChar);
                    let j = if st.input.buffer[ci.limit] == end_line_char {
                        ci.limit
                    } else {
                        ci.limit + 1
                    };
                    for i in ci.start.until(j) {
                        if i == ci.loc {
                            set_trick_count(st);
                        }
                        print_char(st, env, st.input.buffer[i]);
                    }
                } else {
                    match ci.token_type() {
                        TokenType::Macro => {
                            print_ln(st, env);
                            if let InputName::Macro(cs) = ci.name {
                                print_cs(st, env, Some(cs));
                            }
                        }
                        t => print_nl(st, env, t.label(ci.token_loc())),
                    }
                    l = begin_pseudoprint(st);
                    let from = level_start(st, &ci);
                    show_token_list(st, env, from, ci.token_loc(), 100_000);
                }
                st.shipout.selector = old_setting;
                if st.errors.trick_count == 1_000_000 {
                    set_trick_count(st);
                }
                let m = if st.errors.tally < st.errors.trick_count {
                    st.errors.tally - st.errors.first_count
                } else {
                    st.errors.trick_count - st.errors.first_count
                };
                let first_count = st.errors.first_count;
                let error_line = st.errors.error_line;
                let (start, n) = if l + first_count <= st.errors.half_error_line {
                    (0, l + first_count)
                } else {
                    print(st, env, "...");
                    (
                        l + first_count - st.errors.half_error_line + 3,
                        st.errors.half_error_line,
                    )
                };
                for q in start..first_count {
                    print_char(st, env, st.errors.trick_buf[(q % error_line) as usize]);
                }
                print_ln(st, env);
                for _ in 0..n {
                    print_raw_char(st, env, b' ', true);
                }
                let end = if m + n <= error_line {
                    first_count + m
                } else {
                    first_count + (error_line - n - 3)
                };
                for q in first_count..end {
                    print_char(st, env, st.errors.trick_buf[(q % error_line) as usize]);
                }
                if m + n > error_line {
                    print(st, env, "...");
                }
                nn += 1;
            }
        } else if nn == error_context_lines {
            print_nl(st, env, "...");
            nn += 1;
        }
        if bottom_line {
            break;
        }
        st.input.base_ptr -= 1;
    }
    st.input.cur_input = st.input.input_stack[st.input.input_stack.len()];
    st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
}
