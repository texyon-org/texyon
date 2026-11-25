//! The main control loop and its command dispatch.

use crate::base::cmd::Cmd;
use crate::base::codes::{LeaderCode, ScannerStatus, SkipCode, ValignCode};
use crate::base::command::Command;
use crate::base::consts::IGNORE_DEPTH;
use crate::base::mode::ListKind::{Horizontal, Math, Vertical};
use crate::base::mode::{Mode, ModeName};
use crate::base::params::{GluePar, IntPar, TokPar};
use crate::base::token::{Token, TokenKind};
use crate::base::units::{MemPtr, NodeType, Scaled};
use crate::build::extension::{do_extension, insert_src_special};
use crate::build::math::{
    after_math, append_choices, init_math, math_ac, math_fraction, math_left_right,
    math_limit_switch, math_radical, start_eq_no, sub_sup,
};
use crate::build::pagebuilder::build_page;
use crate::env::EngineEnv;
use crate::files::io::open_or_close;
use crate::memory::mem::{ZERO_GLUE, push_nest};
use crate::memory::node::{GlueKind, LeaderKind, MathSlot};
use crate::preview::session::{capture_command_fence, capture_prefetch_fence};
use crate::print::errors::{Abort, OrConfusion, cs_error, privileged, report_illegal_case};
use crate::print::{issue_message, show_cur_cmd_chr, show_whatever};
use crate::scanning::input::{TokenType, back_input, begin_token_list};
use crate::scanning::scan::{
    eTeX_enabled, get_next, get_token, get_x_token, insert_dollar_sign, math_code_from_tex,
    scan_char_num, scan_delim_num, scan_dimen, scan_math, scan_math_char_num, scan_rule_spec,
    scan_spec, scan_toks, scan_usv_num, set_math_char,
};
use crate::state::{EngineExecutionMode, EngineState};
use crate::tables::cs::{CsId, CsKind};
use crate::tables::eqtb::{CaseTable, Register};
use crate::tables::save::{GroupCode, unsave};

mod align;

pub(crate) use align::SavedAlign;
mod assign;
mod boxes;
mod lists;
mod main_loop;

use align::{align_error, do_endv, init_align, no_align_error, omit_error};
use assign::prefixed_command;
pub(crate) use assign::scan_inter_char_pair;
use boxes::{begin_box, delete_last, scan_box, unpackage};
pub(crate) use lists::{
    NewChar, new_character, new_graf, normal_paragraph, off_save, push_math,
    start_paragraph_language,
};
use lists::{
    ParTokenContext, app_space, append_discretionary, append_glue, append_italic_correction,
    append_kern, append_penalty, begin_insert_or_adjust, end_graf, font_space_glue,
    handle_right_brace, head_for_vmode, indent_in_hmode, insert_par_token_before, its_all_over,
    make_accent, make_mark,
};
pub(crate) use main_loop::InterCharState;
use main_loop::{insert_boundary_inter_char_toks, main_control_main_loop};

pub(crate) const LP_CODE_BASE: i32 = 2;
pub(crate) const RP_CODE_BASE: i32 = 3;
pub(crate) const SPAN_CODE: i32 = 1_114_113;
pub(crate) const CR_CODE: i32 = 1_114_114;
pub(crate) const CR_CR_CODE: i32 = 1_114_115;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum BoxContext {
    Shift(Scaled),
    SetBox { register: Register, global: bool },
    ShipOut,
    Leaders(LeaderKind),
}

impl BoxContext {
    pub(crate) fn leader_ship(code: LeaderCode) -> BoxContext {
        match code.glue_leaders() {
            Some(kind) => BoxContext::Leaders(kind),
            None => BoxContext::ShipOut,
        }
    }
}

fn append_list(st: &mut EngineState, head: MemPtr, tail: MemPtr) -> Result<(), Abort> {
    let first = head.or_confusion("append_list")?.link(&st.mem);
    st.input.cur_list.tail.set_link(&mut st.mem, first);
    st.input.cur_list.tail = tail.or_confusion("append_list")?;
    Ok(())
}

fn begin_every(
    st: &mut EngineState,
    env: &mut EngineEnv,
    par: TokPar,
    kind: TokenType,
) -> Result<(), Abort> {
    let list = st.eqtb.tok_par(par);
    if !list.is_null() {
        begin_token_list(st, env, list, kind)?;
    }
    Ok(())
}

pub(crate) fn shift_case(
    st: &mut EngineState,
    env: &mut EngineEnv,
    b: CaseTable,
) -> Result<(), Abort> {
    let list = scan_toks(st, env, false, false)?;
    let eqtb = &st.eqtb;
    let map = |c: i32| match b {
        CaseTable::Lower => eqtb.lc_code(c),
        CaseTable::Upper => eqtb.uc_code(c),
    };
    st.mem.tokens.map_tokens(list, |t| match t.kind() {
        TokenKind::Char(cmd, c) => match map(c) {
            0 => t,
            mapped => Token::new(cmd, mapped),
        },
        TokenKind::Cs(cs) => match cs.kind() {
            CsKind::Active(c) => match map(c.raw()) {
                0 => t,
                mapped => CsId::of_active(mapped).map_or(t, Token::from_cs),
            },
            _ => t,
        },
    });
    begin_token_list(st, env, list, TokenType::BackedUp)
}

pub(crate) fn main_control_append_normal_space(
    st: &mut EngineState,
    env: &mut EngineEnv,
) -> Result<(), Abort> {
    if insert_boundary_inter_char_toks(st, env)? {
        return Ok(());
    }
    let glue = if st.eqtb.glue_par(GluePar::SpaceSkip) == ZERO_GLUE {
        let spec = font_space_glue(st)?;
        st.mem.new_glue(spec)?
    } else {
        st.mem.new_param_glue(&st.eqtb, GluePar::SpaceSkip)?
    };
    st.mem.temp_ptr = glue.ptr();
    st.input.cur_list.tail_append(&mut st.mem, glue);
    Ok(())
}

fn append_rule(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    let rule = scan_rule_spec(st, env)?;
    st.input.cur_list.tail_append(&mut st.mem, rule);
    match &mut st.input.cur_list.mode {
        Mode::Vertical { prev_depth, .. } => *prev_depth = IGNORE_DEPTH,
        Mode::Horizontal { space_factor, .. } => *space_factor = 1000,
        Mode::Math { .. } | Mode::None => {}
    }
    Ok(())
}

fn valign_or_lr(st: &mut EngineState, env: &mut EngineEnv, code: ValignCode) -> Result<(), Abort> {
    let Some(math_code) = code.math_code() else {
        return init_align(st, env);
    };
    let enabled = eTeX_enabled(
        st,
        env,
        st.eqtb.int_par(IntPar::Texxet) > 0,
        Command::Valign(code),
    )?;
    if enabled {
        let m = st.mem.new_math(Scaled::ZERO, math_code)?;
        st.input.cur_list.tail_append(&mut st.mem, m);
    }
    Ok(())
}

fn in_math_shift_group(
    st: &mut EngineState,
    env: &mut EngineEnv,
    action: impl FnOnce(&mut EngineState, &mut EngineEnv) -> Result<(), Abort>,
) -> Result<(), Abort> {
    if st.save_stack.cur_group == GroupCode::MathShift {
        action(st, env)
    } else {
        off_save(st, env)
    }
}

fn append_noad(
    st: &mut EngineState,
    env: &mut EngineEnv,
    kind: Option<NodeType>,
) -> Result<(), Abort> {
    let n = st.mem.new_noad()?;
    st.input.cur_list.tail_append(&mut st.mem, n);
    match kind {
        Some(t) => st.mem[n].noad_type = t.into(),
        None => back_input(st)?,
    }
    scan_math(st, env, MathSlot::Nucleus(n))?;
    Ok(())
}

fn math_vcenter(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    scan_spec(st, env, GroupCode::Vcenter, None)?;
    normal_paragraph(st)?;
    push_nest(st)?;
    st.input.cur_list.mode = Mode::vertical(true);
    if st.shipout.insert_src_special_every_vbox {
        insert_src_special(st, env)?;
    }
    begin_every(st, env, TokPar::EveryVbox, TokenType::EveryVbox)
}

pub(crate) fn main_control_ignore_spaces(
    st: &mut EngineState,
    env: &mut EngineEnv,
    primitive: bool,
) -> Result<(), Abort> {
    if !primitive {
        loop {
            get_x_token(st, env)?;
            if st.scanner.cur.cmd() != Cmd::Spacer {
                break;
            }
        }
    } else {
        let saved = st.scanner.scanner_status;
        st.scanner.scanner_status = ScannerStatus::Normal;
        let outcome = get_next(st, env);
        st.scanner.scanner_status = saved;
        outcome?;

        if let Some(prim) = st.eqtb.prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)? {
            let p = CsId::primitive(prim);
            st.scanner.cur = st.eqtb.meaning(p);
            st.scanner.cur_cs = Some(p);
            st.scanner.cur_tok = Token::from_cs(p);
        } else {
            st.scanner.cur_cs = None;
        }
    }
    Ok(())
}

enum Next {
    /// `big_switch`: fetch a new token.
    BigSwitch,
    /// `reswitch`: act on the current token again.
    Reswitch,
    /// `reswitch` on the character that `\noboundary` came before.
    ReswitchNoBoundary,
    Stop,
}

#[inline]
fn dispatch_command(
    st: &mut EngineState,
    env: &mut EngineEnv,
    cancel_boundary: bool,
) -> Result<Next, Abort> {
    if st.eqtb.int_par(IntPar::TracingCommands) > 0 {
        show_cur_cmd_chr(st, env);
    }
    let mode = st.input.cur_list.mode.kind().or_confusion("mode")?;
    let cmd = st.scanner.cur.cmd();
    if mode == Horizontal {
        match cmd {
            Cmd::Letter | Cmd::OtherChar | Cmd::CharGiven | Cmd::CharNum => {
                let chr = match st.scanner.cur.char_code() {
                    Some(c) => c,
                    None => scan_usv_num(st, env)?,
                };
                return Ok(if main_control_main_loop(st, env, chr, cancel_boundary)? {
                    Next::BigSwitch
                } else {
                    Next::Reswitch
                });
            }
            Cmd::NoBoundary => {
                get_x_token(st, env)?;
                return Ok(
                    if matches!(
                        st.scanner.cur.cmd(),
                        Cmd::Letter | Cmd::OtherChar | Cmd::CharGiven | Cmd::CharNum
                    ) {
                        Next::ReswitchNoBoundary
                    } else {
                        Next::Reswitch
                    },
                );
            }
            _ => {}
        }
        if insert_boundary_inter_char_toks(st, env)? {
            return Ok(Next::BigSwitch);
        }
    }

    match (mode, st.scanner.cur) {
        (Horizontal, Command::Spacer(_)) => {
            if st.input.cur_list.space_factor()? == 1000 {
                main_control_append_normal_space(st, env)?;
            } else {
                app_space(st)?;
            }
        }
        (Horizontal | Math, Command::ExSpace) => main_control_append_normal_space(st, env)?,
        (_, Command::Relax { .. })
        | (Vertical | Math, Command::Spacer(_))
        | (Math, Command::NoBoundary) => {}
        (_, Command::IgnoreSpaces { primitive }) => {
            main_control_ignore_spaces(st, env, primitive)?;
            return Ok(Next::Reswitch);
        }
        (Vertical, Command::Stop { .. }) => {
            if its_all_over(st, env)? {
                return Ok(Next::Stop);
            }
        }

        (Vertical, Command::Vmove { .. })
        | (Horizontal | Math, Command::Hmove { .. })
        | (_, Command::LastItem(_))
        | (Vertical, Command::Vadjust | Command::ItalCorr)
        | (Vertical | Horizontal, Command::EqNo { .. })
        | (_, Command::MacParam(_)) => report_illegal_case(st, env)?,
        (
            Vertical | Horizontal,
            Command::SupMark(_)
            | Command::SubMark(_)
            | Command::MathCharNum(_)
            | Command::MathGiven(_)
            | Command::XetexMathGiven(_)
            | Command::MathComp(_)
            | Command::DelimNum(_)
            | Command::LeftRight(_)
            | Command::Above(_)
            | Command::Radical(_)
            | Command::MathStyle(_)
            | Command::MathChoice
            | Command::Vcenter
            | Command::NonScript
            | Command::Mkern
            | Command::LimitSwitch(_)
            | Command::Mskip
            | Command::MathAccent(_),
        )
        | (
            Math,
            Command::Endv
            | Command::ParEnd
            | Command::Stop { .. }
            | Command::Vskip(_)
            | Command::UnVbox(_)
            | Command::Valign(_)
            | Command::Hrule,
        ) => insert_dollar_sign(st, env)?,

        (Vertical, Command::Hrule) | (Horizontal | Math, Command::Vrule) => append_rule(st, env)?,
        (Vertical, Command::Vskip(s)) | (Horizontal | Math, Command::Hskip(s)) => {
            append_glue(st, env, s)?;
        }
        (Math, Command::Mskip) => {
            append_glue(st, env, SkipCode::Mskip)?;
        }
        (_, Command::Kern) => append_kern(st, env, false)?,
        (Math, Command::Mkern) => append_kern(st, env, true)?,
        (Vertical | Horizontal, Command::LeftBrace(_)) => st
            .save_stack
            .new_save_level(GroupCode::Simple, st.mem.line)?,
        (_, Command::BeginGroup) => st
            .save_stack
            .new_save_level(GroupCode::SemiSimple, st.mem.line)?,
        (_, Command::EndGroup) => {
            if st.save_stack.cur_group == GroupCode::SemiSimple {
                unsave(st, env)?;
            } else {
                off_save(st, env)?;
            }
        }

        (_, Command::RightBrace(_)) => handle_right_brace(st, env)?,
        (Vertical, Command::Hmove { left: negate })
        | (Horizontal | Math, Command::Vmove { raise: negate }) => {
            let d = scan_dimen(st, env, false)?;
            let shift = if negate { -d } else { d };
            scan_box(st, env, BoxContext::Shift(shift))?;
        }
        (_, Command::LeaderShip(code)) => scan_box(st, env, BoxContext::leader_ship(code))?,
        (_, Command::MakeBox(code)) => begin_box(st, env, BoxContext::Shift(Scaled::ZERO), code)?,
        (Vertical, Command::StartPar { indent }) => new_graf(st, env, indent)?,
        (
            Vertical,
            Command::Letter(_)
            | Command::OtherChar(_)
            | Command::CharNum
            | Command::CharGiven(_)
            | Command::MathShift(_)
            | Command::UnHbox(_)
            | Command::Vrule
            | Command::Accent
            | Command::Discretionary { .. }
            | Command::Hskip(_)
            | Command::Valign(_)
            | Command::ExSpace
            | Command::NoBoundary,
        ) => {
            back_input(st)?;
            new_graf(st, env, true)?;
        }
        (Horizontal | Math, Command::StartPar { indent }) => indent_in_hmode(st, indent)?,
        (Vertical, Command::ParEnd) => {
            normal_paragraph(st)?;
            if st.input.cur_list.mode.is_outer() {
                build_page(st, env)?;
            }
        }
        (Horizontal, Command::ParEnd) => {
            if st.scanner.align_state < 0 {
                off_save(st, env)?;
            }
            end_graf(st, env)?;
            if st.input.cur_list.mode.name() == ModeName::Vertical {
                build_page(st, env)?;
            }
        }
        (
            Horizontal,
            Command::Stop { .. }
            | Command::Vskip(_)
            | Command::Hrule
            | Command::UnVbox(_)
            | Command::Halign,
        ) => head_for_vmode(st, env)?,
        (_, Command::Insert) | (Horizontal | Math, Command::Vadjust) => {
            begin_insert_or_adjust(st, env)?
        }
        (_, Command::Mark { marks }) => make_mark(st, env, marks)?,
        (_, Command::BreakPenalty) => append_penalty(st, env)?,
        (_, Command::RemoveItem(t)) => delete_last(st, env, t)?,
        (Vertical, Command::UnVbox(code)) | (Horizontal | Math, Command::UnHbox(code)) => {
            unpackage(st, env, code)?
        }
        (Horizontal, Command::ItalCorr) => append_italic_correction(st)?,
        (Math, Command::ItalCorr) => {
            let k = st.mem.new_kern(Scaled::ZERO)?;
            st.input.cur_list.tail_append(&mut st.mem, k);
        }
        (Horizontal | Math, Command::Discretionary { hyphen }) => {
            append_discretionary(st, env, hyphen)?
        }
        (Horizontal, Command::Accent) => make_accent(st, env)?,
        (_, Command::CarRet { .. } | Command::TabMark(_)) => align_error(st, env)?,
        (_, Command::NoAlign) => no_align_error(st, env)?,
        (_, Command::Omit) => omit_error(st, env)?,
        (Vertical, Command::Halign) => init_align(st, env)?,
        (Horizontal, Command::Valign(code)) => valign_or_lr(st, env, code)?,
        (Math, Command::Halign) => {
            if privileged(st, env)? {
                in_math_shift_group(st, env, init_align)?;
            }
        }
        (Vertical | Horizontal, Command::Endv) => {
            if !insert_par_token_before(st, ParTokenContext::AllGroups)? {
                do_endv(st, env)?;
            }
        }
        (_, Command::EndCsName) => cs_error(st, env)?,

        (Horizontal, Command::MathShift(_)) => init_math(st, env)?,
        (Math, Command::EqNo { left }) => {
            if privileged(st, env)? {
                in_math_shift_group(st, env, |st, env| start_eq_no(st, env, left))?;
            }
        }
        (Math, Command::LeftBrace(_)) => append_noad(st, env, None)?,
        (Math, Command::Letter(c) | Command::OtherChar(c) | Command::CharGiven(c)) => {
            set_math_char(st, env, c.raw(), st.eqtb.math_code(c.raw()))?
        }
        (Math, Command::CharNum) => {
            let c = scan_char_num(st, env)?;
            set_math_char(st, env, c, st.eqtb.math_code(c))?;
        }
        (Math, Command::MathCharNum(code)) => {
            let c = scan_math_char_num(st, env, code)?;
            set_math_char(st, env, st.scanner.cur.encode().1, c)?;
        }
        (Math, Command::MathGiven(code)) => {
            let c = math_code_from_tex(code);
            set_math_char(st, env, code, c)?
        }
        (Math, Command::XetexMathGiven(code)) => set_math_char(st, env, code, code)?,
        (Math, Command::DelimNum(code)) => {
            let c = scan_delim_num(st, env, code)?;
            set_math_char(st, env, st.scanner.cur.encode().1, c)?;
        }
        (Math, Command::MathComp(t)) => append_noad(st, env, Some(t))?,
        (Math, Command::LimitSwitch(code)) => math_limit_switch(st, env, code)?,
        (Math, Command::Radical(code)) => math_radical(st, env, code)?,
        (Math, Command::Accent) => math_ac(st, env, None)?,
        (Math, Command::MathAccent(code)) => math_ac(st, env, Some(code))?,
        (Math, Command::Vcenter) => math_vcenter(st, env)?,
        (Math, Command::MathStyle(style)) => {
            let s = st.mem.new_style(style)?;
            st.input.cur_list.tail_append(&mut st.mem, s);
        }
        (Math, Command::NonScript) => {
            let g = st.mem.new_glue(ZERO_GLUE)?;
            st.input.cur_list.tail_append(&mut st.mem, g);
            st.mem[g].subtype = GlueKind::CondMath.into();
        }
        (Math, Command::MathChoice) => append_choices(st, env)?,
        (Math, Command::SubMark(_) | Command::SupMark(_)) => sub_sup(st, env)?,
        (Math, Command::Above(code)) => math_fraction(st, env, code)?,
        (Math, Command::LeftRight(code)) => math_left_right(st, env, code)?,
        (Math, Command::MathShift(_)) => in_math_shift_group(st, env, after_math)?,

        (_, Command::PartokenName) => {
            get_token(st, env)?;
            if !st.scanner.cur_cs.is_none() {
                st.input.par_loc = st.scanner.cur_cs;
                st.input.par_token = st.scanner.cur_tok;
            }
        }
        (_, _) if cmd.is_prefixed() => prefixed_command(st, env)?,
        (_, Command::AfterAssignment) => {
            get_token(st, env)?;
            st.scanner.after_token = st.scanner.cur_tok;
        }
        (_, Command::AfterGroup) => {
            get_token(st, env)?;
            st.save_stack.save_for_after(st.scanner.cur_tok)?;
        }
        (_, Command::InStream { open }) => open_or_close(st, env, open)?,
        (_, Command::Message { err }) => issue_message(st, env, err)?,
        (_, Command::CaseShift(base)) => shift_case(st, env, base)?,
        (_, Command::Xray(code)) => show_whatever(st, env, code)?,
        (_, Command::Extension(code)) => do_extension(st, env, code)?,
        _ => {}
    }
    Ok(Next::BigSwitch)
}

fn main_control_from(
    st: &mut EngineState,
    env: &mut EngineEnv,
    run_every_job: bool,
    mut resume_pending_command: bool,
) -> Result<(), Abort> {
    if run_every_job {
        begin_every(st, env, TokPar::EveryJob, TokenType::EveryJob)?;
    }
    // Cancellation and fence hooks are for resident sessions only, batch runs
    // must not pay for them per command.
    let resident = st.config.execution_mode == EngineExecutionMode::Resident;
    loop {
        if resident && st.cancellation.at_command_boundary() {
            return Err(Abort::Cancelled);
        }
        if resume_pending_command {
            resume_pending_command = false;
        } else {
            if resident {
                capture_prefetch_fence(st);
            }
            get_x_token(st, env)?;
        }
        if resident && capture_command_fence(st) {
            return Err(Abort::Horizoned);
        }
        let mut cancel_boundary = false;
        loop {
            match dispatch_command(st, env, cancel_boundary)? {
                Next::BigSwitch => break,
                Next::Reswitch => cancel_boundary = false,
                Next::ReswitchNoBoundary => cancel_boundary = true,
                Next::Stop => return Ok(()),
            }
        }
    }
}

pub(crate) fn main_control(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    main_control_from(st, env, true, false)
}

pub(crate) fn main_control_resume(
    st: &mut EngineState,
    env: &mut EngineEnv,
    resume_pending_command: bool,
) -> Result<(), Abort> {
    main_control_from(st, env, false, resume_pending_command)
}
