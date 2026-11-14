//! The main control loop and its command dispatch.

use crate::cmd::Cmd;
use crate::codes::SkipCode;
use crate::codes::ValignCode;
use crate::codes::{LeaderCode, ScannerStatus};
use crate::command::Command;
use crate::consts::IGNORE_DEPTH;
use crate::cs::{CsId, CsKind};
use crate::eqtb::CaseTable;
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::ext::do_extension;
use crate::input::TokenType;
use crate::math::{after_math, init_math};
use crate::mem::ZERO_GLUE;
use crate::mode::ListKind::{Horizontal, Math, Vertical};
use crate::mode::{Mode, ModeName};
use crate::node::{GlueKind, LeaderKind};
use crate::params::{GluePar, IntPar, TokPar};
use crate::state::{EngineState, GroupCode};
use crate::token::{Token, TokenKind};
use crate::units::NodeType;
use crate::units::{MemPtr, Scaled};

mod align;
pub(crate) use align::SavedAlign;
mod assign;
mod boxes;
mod lists;
mod main_loop;

use self::align::{align_error, do_endv, init_align, no_align_error, omit_error};
use self::assign::prefixed_command;
pub(crate) use self::assign::scan_inter_char_pair;
use self::boxes::{begin_box, delete_last, scan_box, unpackage};
pub(crate) use self::lists::{
    NewChar, new_character, new_graf, normal_paragraph, off_save, push_math,
    start_paragraph_language,
};
use self::lists::{
    ParTokenContext, app_space, append_discretionary, append_glue, append_italic_correction,
    append_kern, append_penalty, begin_insert_or_adjust, end_graf, font_space_glue,
    handle_right_brace, head_for_vmode, indent_in_hmode, insert_par_token_before, its_all_over,
    make_accent, make_mark,
};
use self::main_loop::{insert_boundary_inter_char_toks, main_control_main_loop};

// `cur_chr` modifiers.
pub(crate) const LP_CODE_BASE: i32 = 2;
pub(crate) const RP_CODE_BASE: i32 = 3;
pub(crate) const SPAN_CODE: i32 = 1_114_113;
pub(crate) const CR_CODE: i32 = 1_114_114;
pub(crate) const CR_CR_CODE: i32 = 1_114_115;

/// Where a box goes once it is made, TeX's `box_context`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum BoxContext {
    /// Onto the current list, shifted by `\raise` and friends or by zero.
    Shift(Scaled),
    /// Into a box register, by `\setbox`.
    SetBox {
        register: crate::eqtb::Register,
        global: bool,
    },
    /// Out to the page, by `\shipout`.
    ShipOut,
    /// Into the leaders of the glue that follows.
    Leaders(LeaderKind),
}

impl BoxContext {
    /// The context of `\shipout` or a leaders primitive.
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

fn begin_every(st: &mut EngineState, par: TokPar, kind: TokenType) -> Result<(), Abort> {
    let list = st.eqtb.tok_par(par);
    if !list.is_null() {
        crate::input::begin_token_list(st, list, kind)?;
    }
    Ok(())
}

pub(crate) fn shift_case(st: &mut EngineState, b: CaseTable) -> Result<(), Abort> {
    let list = crate::scan::scan_toks(st, false, false)?;
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
    crate::input::begin_token_list(st, list, TokenType::BackedUp)
}

pub(crate) fn main_control_append_normal_space(st: &mut EngineState) -> Result<(), Abort> {
    if insert_boundary_inter_char_toks(st)? {
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

fn append_rule(st: &mut EngineState) -> Result<(), Abort> {
    let rule = crate::scan::scan_rule_spec(st)?;
    st.input.cur_list.tail_append(&mut st.mem, rule);
    match &mut st.input.cur_list.mode {
        Mode::Vertical { prev_depth, .. } => *prev_depth = IGNORE_DEPTH,
        Mode::Horizontal { space_factor, .. } => *space_factor = 1000,
        Mode::Math { .. } | Mode::None => {}
    }
    Ok(())
}

fn valign_or_lr(st: &mut EngineState, code: ValignCode) -> Result<(), Abort> {
    let Some(math_code) = code.math_code() else {
        return init_align(st);
    };
    let enabled = crate::scan::eTeX_enabled(
        st,
        st.eqtb.int_par(IntPar::Texxet) > 0,
        Command::Valign(code),
    )?;
    if enabled {
        let m = st.mem.new_math(Scaled::ZERO, math_code)?;
        st.input.cur_list.tail_append(&mut st.mem, m);
    }
    Ok(())
}

/// Runs `action` if the math-shift group is open, else inserts what closes the current group.
fn in_math_shift_group(
    st: &mut EngineState,
    action: impl FnOnce(&mut EngineState) -> Result<(), Abort>,
) -> Result<(), Abort> {
    if st.save_stack.cur_group == GroupCode::MathShift {
        action(st)
    } else {
        off_save(st)
    }
}

/// Appends a noad of type `kind` (an ordinary one for `{`, put back) and scans its nucleus.
fn append_noad(st: &mut EngineState, kind: Option<NodeType>) -> Result<(), Abort> {
    let n = st.mem.new_noad()?;
    st.input.cur_list.tail_append(&mut st.mem, n);
    match kind {
        Some(t) => st.mem[n].noad_type = t.into(),
        None => crate::input::back_input(st)?,
    }
    crate::scan::scan_math(st, crate::node::MathSlot::Nucleus(n))?;
    Ok(())
}

fn math_vcenter(st: &mut EngineState) -> Result<(), Abort> {
    crate::scan::scan_spec(st, GroupCode::Vcenter, None)?;
    normal_paragraph(st)?;
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = Mode::vertical(true);
    if st.shipout.insert_src_special_every_vbox {
        crate::ext::insert_src_special(st)?;
    }
    begin_every(st, TokPar::EveryVbox, TokenType::EveryVbox)
}

pub(crate) fn main_control_ignore_spaces(
    st: &mut EngineState,
    primitive: bool,
) -> Result<(), Abort> {
    if !primitive {
        loop {
            crate::scan::get_x_token(st)?;
            if st.scanner.cur.cmd() != Cmd::Spacer {
                break;
            }
        }
    } else {
        let saved = st.scanner.scanner_status;
        st.scanner.scanner_status = ScannerStatus::Normal;
        let outcome = crate::scan::get_next(st);
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

/// Where `main_control` continues after a command.
enum Next {
    /// `big_switch`: fetch a new token.
    BigSwitch,
    /// `reswitch`: act on the current token again.
    Reswitch,
    /// `reswitch` on the character that `\noboundary` came before.
    ReswitchNoBoundary,
    /// `\end` or `\dump` finished the job.
    Stop,
}

/// Acts on the current command in the current mode, TeX's `abs(mode)+cur_cmd` case.
#[inline]
fn dispatch_command(st: &mut EngineState, cancel_boundary: bool) -> Result<Next, Abort> {
    if st.eqtb.int_par(IntPar::TracingCommands) > 0 {
        crate::output::show_cur_cmd_chr(st);
    }
    let mode = st.input.cur_list.mode.kind().or_confusion("mode")?;
    let cmd = st.scanner.cur.cmd();
    if mode == Horizontal {
        match cmd {
            Cmd::Letter | Cmd::OtherChar | Cmd::CharGiven | Cmd::CharNum => {
                let chr = match st.scanner.cur.char_code() {
                    Some(c) => c,
                    None => crate::scan::scan_usv_num(st)?,
                };
                return Ok(if main_control_main_loop(st, chr, cancel_boundary)? {
                    Next::BigSwitch
                } else {
                    Next::Reswitch
                });
            }
            Cmd::NoBoundary => {
                crate::scan::get_x_token(st)?;
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
        if insert_boundary_inter_char_toks(st)? {
            return Ok(Next::BigSwitch);
        }
    }

    match (mode, st.scanner.cur) {
        (Horizontal, Command::Spacer(_)) => {
            if st.input.cur_list.space_factor()? == 1000 {
                main_control_append_normal_space(st)?;
            } else {
                app_space(st)?;
            }
        }
        (Horizontal | Math, Command::ExSpace) => main_control_append_normal_space(st)?,
        (_, Command::Relax { .. })
        | (Vertical | Math, Command::Spacer(_))
        | (Math, Command::NoBoundary) => {}
        (_, Command::IgnoreSpaces { primitive }) => {
            main_control_ignore_spaces(st, primitive)?;
            return Ok(Next::Reswitch);
        }
        (Vertical, Command::Stop { .. }) => {
            if its_all_over(st)? {
                return Ok(Next::Stop);
            }
        }

        (Vertical, Command::Vmove { .. })
        | (Horizontal | Math, Command::Hmove { .. })
        | (_, Command::LastItem(_))
        | (Vertical, Command::Vadjust | Command::ItalCorr)
        | (Vertical | Horizontal, Command::EqNo { .. })
        | (_, Command::MacParam(_)) => crate::errors::report_illegal_case(st)?,
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
        ) => crate::scan::insert_dollar_sign(st)?,

        (Vertical, Command::Hrule) | (Horizontal | Math, Command::Vrule) => append_rule(st)?,
        (Vertical, Command::Vskip(s)) | (Horizontal | Math, Command::Hskip(s)) => {
            append_glue(st, s)?;
        }
        (Math, Command::Mskip) => {
            append_glue(st, SkipCode::Mskip)?;
        }
        (_, Command::Kern) => append_kern(st, false)?,
        (Math, Command::Mkern) => append_kern(st, true)?,
        (Vertical | Horizontal, Command::LeftBrace(_)) => st
            .save_stack
            .new_save_level(GroupCode::Simple, st.mem.line)?,
        (_, Command::BeginGroup) => st
            .save_stack
            .new_save_level(GroupCode::SemiSimple, st.mem.line)?,
        (_, Command::EndGroup) => {
            if st.save_stack.cur_group == GroupCode::SemiSimple {
                crate::save::unsave(st)?;
            } else {
                off_save(st)?;
            }
        }

        (_, Command::RightBrace(_)) => handle_right_brace(st)?,
        (Vertical, Command::Hmove { left: negate })
        | (Horizontal | Math, Command::Vmove { raise: negate }) => {
            let d = crate::scan::scan_dimen(st, false)?;
            let shift = if negate { -d } else { d };
            scan_box(st, BoxContext::Shift(shift))?;
        }
        (_, Command::LeaderShip(code)) => scan_box(st, BoxContext::leader_ship(code))?,
        (_, Command::MakeBox(code)) => begin_box(st, BoxContext::Shift(Scaled::ZERO), code)?,
        (Vertical, Command::StartPar { indent }) => new_graf(st, indent)?,
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
            crate::input::back_input(st)?;
            new_graf(st, true)?;
        }
        (Horizontal | Math, Command::StartPar { indent }) => indent_in_hmode(st, indent)?,
        (Vertical, Command::ParEnd) => {
            normal_paragraph(st)?;
            if st.input.cur_list.mode.is_outer() {
                crate::pagebuilder::build_page(st)?;
            }
        }
        (Horizontal, Command::ParEnd) => {
            if st.scanner.align_state < 0 {
                off_save(st)?;
            }
            end_graf(st)?;
            if st.input.cur_list.mode.name() == ModeName::Vertical {
                crate::pagebuilder::build_page(st)?;
            }
        }
        (
            Horizontal,
            Command::Stop { .. }
            | Command::Vskip(_)
            | Command::Hrule
            | Command::UnVbox(_)
            | Command::Halign,
        ) => head_for_vmode(st)?,
        (_, Command::Insert) | (Horizontal | Math, Command::Vadjust) => begin_insert_or_adjust(st)?,
        (_, Command::Mark { marks }) => make_mark(st, marks)?,
        (_, Command::BreakPenalty) => append_penalty(st)?,
        (_, Command::RemoveItem(t)) => delete_last(st, t)?,
        (Vertical, Command::UnVbox(code)) | (Horizontal | Math, Command::UnHbox(code)) => {
            unpackage(st, code)?
        }
        (Horizontal, Command::ItalCorr) => append_italic_correction(st)?,
        (Math, Command::ItalCorr) => {
            let k = st.mem.new_kern(Scaled::ZERO)?;
            st.input.cur_list.tail_append(&mut st.mem, k);
        }
        (Horizontal | Math, Command::Discretionary { hyphen }) => append_discretionary(st, hyphen)?,
        (Horizontal, Command::Accent) => make_accent(st)?,
        (_, Command::CarRet { .. } | Command::TabMark(_)) => align_error(st)?,
        (_, Command::NoAlign) => no_align_error(st)?,
        (_, Command::Omit) => omit_error(st)?,
        (Vertical, Command::Halign) => init_align(st)?,
        (Horizontal, Command::Valign(code)) => valign_or_lr(st, code)?,
        (Math, Command::Halign) => {
            if crate::errors::privileged(st)? {
                in_math_shift_group(st, init_align)?;
            }
        }
        (Vertical | Horizontal, Command::Endv) => {
            if !insert_par_token_before(st, ParTokenContext::AllGroups)? {
                do_endv(st)?;
            }
        }
        (_, Command::EndCsName) => crate::errors::cs_error(st)?,

        (Horizontal, Command::MathShift(_)) => init_math(st)?,
        (Math, Command::EqNo { left }) => {
            if crate::errors::privileged(st)? {
                in_math_shift_group(st, |st| crate::math::start_eq_no(st, left))?;
            }
        }
        (Math, Command::LeftBrace(_)) => append_noad(st, None)?,
        (Math, Command::Letter(c) | Command::OtherChar(c) | Command::CharGiven(c)) => {
            crate::scan::set_math_char(st, c.raw(), st.eqtb.math_code(c.raw()))?
        }
        (Math, Command::CharNum) => {
            let c = crate::scan::scan_char_num(st)?;
            crate::scan::set_math_char(st, c, st.eqtb.math_code(c))?;
        }
        (Math, Command::MathCharNum(code)) => {
            let c = crate::scan::scan_math_char_num(st, code)?;
            crate::scan::set_math_char(st, st.scanner.cur.encode().1, c)?;
        }
        (Math, Command::MathGiven(code)) => {
            let c = crate::scan::math_code_from_tex(code);
            crate::scan::set_math_char(st, code, c)?
        }
        (Math, Command::XetexMathGiven(code)) => crate::scan::set_math_char(st, code, code)?,
        (Math, Command::DelimNum(code)) => {
            let c = crate::scan::scan_delim_num(st, code)?;
            crate::scan::set_math_char(st, st.scanner.cur.encode().1, c)?;
        }
        (Math, Command::MathComp(t)) => append_noad(st, Some(t))?,
        (Math, Command::LimitSwitch(code)) => crate::math::math_limit_switch(st, code)?,
        (Math, Command::Radical(code)) => crate::math::math_radical(st, code)?,
        (Math, Command::Accent) => crate::math::math_ac(st, None)?,
        (Math, Command::MathAccent(code)) => crate::math::math_ac(st, Some(code))?,
        (Math, Command::Vcenter) => math_vcenter(st)?,
        (Math, Command::MathStyle(style)) => {
            let s = st.mem.new_style(style)?;
            st.input.cur_list.tail_append(&mut st.mem, s);
        }
        (Math, Command::NonScript) => {
            let g = st.mem.new_glue(ZERO_GLUE)?;
            st.input.cur_list.tail_append(&mut st.mem, g);
            st.mem[g].subtype = GlueKind::CondMath.into();
        }
        (Math, Command::MathChoice) => crate::math::append_choices(st)?,
        (Math, Command::SubMark(_) | Command::SupMark(_)) => crate::math::sub_sup(st)?,
        (Math, Command::Above(code)) => crate::math::math_fraction(st, code)?,
        (Math, Command::LeftRight(code)) => crate::math::math_left_right(st, code)?,
        (Math, Command::MathShift(_)) => in_math_shift_group(st, after_math)?,

        (_, Command::PartokenName) => {
            crate::scan::get_token(st)?;
            if !st.scanner.cur_cs.is_none() {
                st.input.par_loc = st.scanner.cur_cs;
                st.input.par_token = st.scanner.cur_tok;
            }
        }
        (_, _) if cmd.is_prefixed() => prefixed_command(st)?,
        (_, Command::AfterAssignment) => {
            crate::scan::get_token(st)?;
            st.scanner.after_token = st.scanner.cur_tok;
        }
        (_, Command::AfterGroup) => {
            crate::scan::get_token(st)?;
            st.save_stack.save_for_after(st.scanner.cur_tok)?;
        }
        (_, Command::InStream { open }) => crate::io::open_or_close(st, open)?,
        (_, Command::Message { err }) => crate::output::issue_message(st, err)?,
        (_, Command::CaseShift(base)) => shift_case(st, base)?,
        (_, Command::Xray(code)) => crate::output::show_whatever(st, code)?,
        (_, Command::Extension(code)) => do_extension(st, code)?,
        _ => {}
    }
    Ok(Next::BigSwitch)
}

fn main_control_from(
    st: &mut EngineState,
    run_every_job: bool,
    mut resume_pending_command: bool,
) -> Result<(), Abort> {
    if run_every_job {
        begin_every(st, TokPar::EveryJob, TokenType::EveryJob)?;
    }
    // Cancellation and fence hooks are for resident sessions only, batch runs
    // must not pay for them per command.
    let resident = st.config.execution_mode == crate::state::EngineExecutionMode::Resident;
    loop {
        if resident && st.cancellation.at_command_boundary() {
            return Err(Abort::Cancelled);
        }
        if resume_pending_command {
            resume_pending_command = false;
        } else {
            if resident {
                crate::session::capture_prefetch_fence(st);
            }
            crate::scan::get_x_token(st)?;
        }
        if resident && crate::session::capture_command_fence(st) {
            return Err(Abort::Horizoned);
        }
        let mut cancel_boundary = false;
        loop {
            match dispatch_command(st, cancel_boundary)? {
                Next::BigSwitch => break,
                Next::Reswitch => cancel_boundary = false,
                Next::ReswitchNoBoundary => cancel_boundary = true,
                Next::Stop => return Ok(()),
            }
        }
    }
}

/// Dispatch a fresh document, including `\everyjob` and the first token fetch.
pub(crate) fn main_control(st: &mut EngineState) -> Result<(), Abort> {
    main_control_from(st, true, false)
}

/// Resume at a restored command boundary, optionally with a command already fetched.
pub(crate) fn main_control_resume(
    st: &mut EngineState,
    resume_pending_command: bool,
) -> Result<(), Abort> {
    main_control_from(st, false, resume_pending_command)
}
