//! The main control loop and its command dispatch.

use crate::cmd::Cmd;
use crate::codes::SkipCode;
use crate::codes::ValignCode;
use crate::codes::{LeaderCode, ScannerStatus};
use crate::command::Command;
use crate::consts::{HMODE, IGNORE_DEPTH, MMODE, SINGLE_BASE, VMODE};
use crate::errors::Abort;
use crate::ext::do_extension;
use crate::input::TokenType;
use crate::math::{after_math, init_math};
use crate::mem::ZERO_GLUE;
use crate::node::GlueKind;
use crate::params::{GluePar, IntPar, TokPar};
use crate::state::{EngineState, GroupCode};
use crate::token::Token;
use crate::units::NodeType;
use crate::units::{EqtbPtr, MemPtr, Scaled};

mod align;
mod assign;
mod boxes;
mod lists;
mod main_loop;
pub(crate) use main_loop::LigStack;

use self::align::{align_error, do_endv, init_align, no_align_error, omit_error};
use self::assign::prefixed_command;
use self::boxes::{begin_box, delete_last, scan_box, unpackage};
use self::lists::{
    app_space, append_discretionary, append_glue, append_italic_correction, append_kern,
    append_penalty, begin_insert_or_adjust, end_graf, font_space_glue, handle_right_brace,
    head_for_vmode, indent_in_hmode, insert_par_token_before, its_all_over, make_accent, make_mark,
    ParTokenContext,
};
pub(crate) use self::lists::{
    new_character, new_graf, normal_paragraph, off_save, push_math, start_paragraph_language,
    NewChar,
};
use self::main_loop::{insert_boundary_inter_char_toks, main_control_main_loop};

const INNER_VMODE: i16 = -VMODE;
const INNER_HMODE: i16 = -HMODE;

// `cur_chr` modifiers.
pub(crate) const LP_CODE_BASE: i32 = 2;
pub(crate) const RP_CODE_BASE: i32 = 3;
pub(crate) const SPAN_CODE: i32 = 1_114_113;
pub(crate) const CR_CODE: i32 = 1_114_114;
pub(crate) const CR_CR_CODE: i32 = 1_114_115;

pub(crate) const BOX_FLAG: i32 = 0x4000_0000;
pub(crate) const GLOBAL_BOX_FLAG: i32 = 0x4000_8000;
pub(crate) const SHIP_OUT_FLAG: i32 = 0x4001_0000;
pub(crate) const LEADER_FLAG: i32 = 0x4001_0001;

fn append_list(st: &mut EngineState, head: MemPtr, tail: MemPtr) {
    let first = head.link(&st.mem);
    st.input.cur_list.tail.set_link(&mut st.mem, first);
    st.input.cur_list.tail = tail;
}

fn begin_every(st: &mut EngineState, par: TokPar, kind: TokenType) -> Result<(), Abort> {
    let list = st.eqtb.tok_par(par);
    if !list.is_null() {
        crate::input::begin_token_list(st, list, kind)?;
    }
    Ok(())
}

pub(crate) fn shift_case(st: &mut EngineState, b: EqtbPtr) -> Result<(), Abort> {
    crate::scan::scan_toks(st, false, false)?;
    let mut p = st.scanner.def_ref.link(&st.mem);
    while !p.is_null() {
        let t = p.token(&st.mem);
        if t < Token::from_cs(SINGLE_BASE) {
            let c = t.chr();
            let mapped = st.eqtb.equiv(b + c);
            if mapped != 0 {
                p.set_token(&mut st.mem, Token(t.0 - c + mapped));
            }
        }
        p = p.link(&st.mem);
    }
    let def_ref = st.scanner.def_ref;
    crate::input::begin_token_list(st, def_ref.link(&st.mem), TokenType::BackedUp)?;
    st.mem.free_avail(def_ref.ptr());
    Ok(())
}

pub(crate) fn main_control_append_normal_space(st: &mut EngineState) -> Result<(), Abort> {
    if insert_boundary_inter_char_toks(st)? {
        return Ok(());
    }
    let glue = if st.eqtb.glue_par(GluePar::SpaceSkip) == ZERO_GLUE {
        let spec = font_space_glue(st)?;
        st.mainloop.main_p = spec.ptr();
        st.mem.new_glue(spec)?
    } else {
        st.mem.new_param_glue(&st.eqtb, GluePar::SpaceSkip)?
    };
    st.mem.temp_ptr = glue.ptr();
    st.input.cur_list.tail_append(&mut st.mem, glue.ptr());
    Ok(())
}

fn append_rule(st: &mut EngineState) -> Result<(), Abort> {
    let mode = st.input.cur_list.mode.abs();
    let rule = crate::scan::scan_rule_spec(st)?;
    st.input.cur_list.tail_append(&mut st.mem, rule.ptr());
    if mode == VMODE {
        st.input.cur_list.set_prev_depth(IGNORE_DEPTH);
    } else if mode == HMODE {
        st.input.cur_list.set_space_factor(1000);
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
        Cmd::Valign,
        code.raw(),
    )?;
    if enabled {
        let m = st.mem.new_math(Scaled::ZERO, math_code)?;
        st.input.cur_list.tail_append(&mut st.mem, m.ptr());
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
    st.input.cur_list.tail_append(&mut st.mem, n.ptr());
    match kind {
        Some(t) => st.mem[n].noad_type = t.into(),
        None => crate::input::back_input(st)?,
    }
    crate::scan::scan_math(st, n.nucleus())?;
    Ok(())
}

fn math_vcenter(st: &mut EngineState) -> Result<(), Abort> {
    crate::scan::scan_spec(st, GroupCode::Vcenter, false)?;
    normal_paragraph(st)?;
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = INNER_VMODE;
    st.input.cur_list.set_prev_depth(IGNORE_DEPTH);
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
            if st.scanner.cur_cmd != Cmd::Spacer {
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
            let p = prim.eqtb_loc();
            st.scanner.cur_cmd = st.eqtb.eq_type(p);
            st.scanner.cur_chr = st.eqtb.equiv(p);
            st.scanner.cur_cs = p;
            st.scanner.cur_tok = Token::from_cs(p);
        } else {
            st.scanner.cur_cs = EqtbPtr::NONE;
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
    /// `\end` or `\dump` finished the job.
    Stop,
}

/// Acts on the current command in the current mode, TeX's `abs(mode)+cur_cmd` case.
#[inline]
fn dispatch_command(st: &mut EngineState) -> Result<Next, Abort> {
    if st.eqtb.int_par(IntPar::TracingCommands) > 0 {
        crate::output::show_cur_cmd_chr(st);
    }
    let mode = st.input.cur_list.mode.abs();
    let cmd = st.scanner.cur_cmd;
    if mode == HMODE {
        match cmd {
            Cmd::Letter | Cmd::OtherChar | Cmd::CharGiven | Cmd::CharNum => {
                if cmd == Cmd::CharNum {
                    st.scanner.cur_chr = crate::scan::scan_usv_num(st)?;
                }
                return Ok(if main_control_main_loop(st)? {
                    Next::BigSwitch
                } else {
                    Next::Reswitch
                });
            }
            Cmd::NoBoundary => {
                crate::scan::get_x_token(st)?;
                if matches!(
                    st.scanner.cur_cmd,
                    Cmd::Letter | Cmd::OtherChar | Cmd::CharGiven | Cmd::CharNum
                ) {
                    st.mainloop.cancel_boundary = true;
                }
                return Ok(Next::Reswitch);
            }
            _ => {}
        }
        if insert_boundary_inter_char_toks(st)? {
            return Ok(Next::BigSwitch);
        }
    }

    match (mode, st.scanner.command()?) {
        (HMODE, Command::Spacer(_)) => {
            if st.input.cur_list.space_factor() == 1000 {
                main_control_append_normal_space(st)?;
            } else {
                app_space(st)?;
            }
        }
        (HMODE | MMODE, Command::ExSpace) => main_control_append_normal_space(st)?,
        (_, Command::Relax { .. })
        | (VMODE | MMODE, Command::Spacer(_))
        | (MMODE, Command::NoBoundary) => {}
        (_, Command::IgnoreSpaces { primitive }) => {
            main_control_ignore_spaces(st, primitive)?;
            return Ok(Next::Reswitch);
        }
        (VMODE, Command::Stop { .. }) => {
            if its_all_over(st)? {
                return Ok(Next::Stop);
            }
        }

        (VMODE, Command::Vmove { .. })
        | (HMODE | MMODE, Command::Hmove { .. })
        | (_, Command::LastItem(_))
        | (VMODE, Command::Vadjust | Command::ItalCorr)
        | (VMODE | HMODE, Command::EqNo { .. })
        | (_, Command::MacParam(_)) => crate::errors::report_illegal_case(st)?,
        (
            VMODE | HMODE,
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
            MMODE,
            Command::Endv
            | Command::ParEnd
            | Command::Stop { .. }
            | Command::Vskip(_)
            | Command::UnVbox(_)
            | Command::Valign(_)
            | Command::Hrule,
        ) => crate::scan::insert_dollar_sign(st)?,

        (VMODE, Command::Hrule) | (HMODE | MMODE, Command::Vrule) => append_rule(st)?,
        (VMODE, Command::Vskip(s)) | (HMODE | MMODE, Command::Hskip(s)) => {
            append_glue(st, s)?;
        }
        (MMODE, Command::Mskip) => {
            append_glue(st, SkipCode::Mskip)?;
        }
        (_, Command::Kern) => append_kern(st, false)?,
        (MMODE, Command::Mkern) => append_kern(st, true)?,
        (VMODE | HMODE, Command::LeftBrace(_)) => st
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
        (VMODE, Command::Hmove { left: negate })
        | (HMODE | MMODE, Command::Vmove { raise: negate }) => {
            let d = crate::scan::scan_dimen(st, false)?;
            let shift = if negate { -d } else { d };
            scan_box(st, shift.raw())?;
        }
        (_, Command::LeaderShip(code)) => {
            scan_box(st, LEADER_FLAG - LeaderCode::ALeaders.raw() + code.raw())?
        }
        (_, Command::MakeBox(code)) => begin_box(st, 0, code)?,
        (VMODE, Command::StartPar { indent }) => new_graf(st, indent)?,
        (
            VMODE,
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
        (HMODE | MMODE, Command::StartPar { indent }) => indent_in_hmode(st, indent)?,
        (VMODE, Command::ParEnd) => {
            normal_paragraph(st)?;
            if st.input.cur_list.mode > 0 {
                crate::pagebuilder::build_page(st)?;
            }
        }
        (HMODE, Command::ParEnd) => {
            if st.scanner.align_state < 0 {
                off_save(st)?;
            }
            end_graf(st)?;
            if st.input.cur_list.mode == VMODE {
                crate::pagebuilder::build_page(st)?;
            }
        }
        (
            HMODE,
            Command::Stop { .. }
            | Command::Vskip(_)
            | Command::Hrule
            | Command::UnVbox(_)
            | Command::Halign,
        ) => head_for_vmode(st)?,
        (_, Command::Insert) | (HMODE | MMODE, Command::Vadjust) => begin_insert_or_adjust(st)?,
        (_, Command::Mark { marks }) => make_mark(st, marks)?,
        (_, Command::BreakPenalty) => append_penalty(st)?,
        (_, Command::RemoveItem(t)) => delete_last(st, t)?,
        (VMODE, Command::UnVbox(code)) | (HMODE | MMODE, Command::UnHbox(code)) => {
            unpackage(st, code)?
        }
        (HMODE, Command::ItalCorr) => append_italic_correction(st)?,
        (MMODE, Command::ItalCorr) => {
            let k = st.mem.new_kern(Scaled::ZERO)?;
            st.input.cur_list.tail_append(&mut st.mem, k.ptr());
        }
        (HMODE | MMODE, Command::Discretionary { hyphen }) => append_discretionary(st, hyphen)?,
        (HMODE, Command::Accent) => make_accent(st)?,
        (_, Command::CarRet { .. } | Command::TabMark(_)) => align_error(st)?,
        (_, Command::NoAlign) => no_align_error(st)?,
        (_, Command::Omit) => omit_error(st)?,
        (VMODE, Command::Halign) => init_align(st)?,
        (HMODE, Command::Valign(code)) => valign_or_lr(st, code)?,
        (MMODE, Command::Halign) => {
            if crate::errors::privileged(st)? {
                in_math_shift_group(st, init_align)?;
            }
        }
        (VMODE | HMODE, Command::Endv) => {
            if !insert_par_token_before(st, ParTokenContext::AllGroups)? {
                do_endv(st)?;
            }
        }
        (_, Command::EndCsName) => crate::errors::cs_error(st)?,

        (HMODE, Command::MathShift(_)) => init_math(st)?,
        (MMODE, Command::EqNo { left }) => {
            if crate::errors::privileged(st)? {
                in_math_shift_group(st, |st| crate::math::start_eq_no(st, left))?;
            }
        }
        (MMODE, Command::LeftBrace(_)) => append_noad(st, None)?,
        (MMODE, Command::Letter(c) | Command::OtherChar(c) | Command::CharGiven(c)) => {
            crate::scan::set_math_char(st, st.eqtb.math_code(c.raw()))?
        }
        (MMODE, Command::CharNum) => {
            st.scanner.cur_chr = crate::scan::scan_char_num(st)?;
            crate::scan::set_math_char(st, st.eqtb.math_code(st.scanner.cur_chr))?;
        }
        (MMODE, Command::MathCharNum(code)) => {
            let c = crate::scan::scan_math_char_num(st, code)?;
            crate::scan::set_math_char(st, c)?;
        }
        (MMODE, Command::MathGiven(code)) => {
            crate::scan::set_math_char(st, crate::scan::math_code_from_tex(code))?
        }
        (MMODE, Command::XetexMathGiven(code)) => crate::scan::set_math_char(st, code)?,
        (MMODE, Command::DelimNum(code)) => {
            let c = crate::scan::scan_delim_num(st, code)?;
            crate::scan::set_math_char(st, c)?;
        }
        (MMODE, Command::MathComp(t)) => append_noad(st, Some(t))?,
        (MMODE, Command::LimitSwitch(code)) => crate::math::math_limit_switch(st, code)?,
        (MMODE, Command::Radical(code)) => crate::math::math_radical(st, code)?,
        (MMODE, Command::Accent) => crate::math::math_ac(st, None)?,
        (MMODE, Command::MathAccent(code)) => crate::math::math_ac(st, Some(code))?,
        (MMODE, Command::Vcenter) => math_vcenter(st)?,
        (MMODE, Command::MathStyle(style)) => {
            let s = st.mem.new_style(style)?;
            st.input.cur_list.tail_append(&mut st.mem, s.ptr());
        }
        (MMODE, Command::NonScript) => {
            let g = st.mem.new_glue(ZERO_GLUE)?;
            st.input.cur_list.tail_append(&mut st.mem, g.ptr());
            st.mem[g].subtype = GlueKind::CondMath.into();
        }
        (MMODE, Command::MathChoice) => crate::math::append_choices(st)?,
        (MMODE, Command::SubMark(_) | Command::SupMark(_)) => crate::math::sub_sup(st)?,
        (MMODE, Command::Above(code)) => crate::math::math_fraction(st, code)?,
        (MMODE, Command::LeftRight(code)) => crate::math::math_left_right(st, code)?,
        (MMODE, Command::MathShift(_)) => in_math_shift_group(st, after_math)?,

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
        loop {
            match dispatch_command(st)? {
                Next::BigSwitch => break,
                Next::Reswitch => {}
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
