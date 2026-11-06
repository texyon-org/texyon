//! The main control loop and its command dispatch.

use crate::cmd::Cmd;
use crate::codes::ScannerStatus;
use crate::consts::{A_LEADERS, COND_MATH_GLUE, HMODE, IGNORE_DEPTH, MMODE, SINGLE_BASE, VMODE};
use crate::errors::Abort;
use crate::ext::do_extension;
use crate::input::TokenType;
use crate::math::{after_math, init_math};
use crate::mem::ZERO_GLUE;
use crate::params::{GluePar, IntPar, TokPar};
use crate::state::{EngineState, GroupCode};
use crate::token::Token;
use crate::units::{EqtbPtr, MemPtr, Scaled};

mod align;
mod assign;
mod boxes;
mod lists;
mod main_loop;

use self::align::{align_error, do_endv, init_align, no_align_error, omit_error};
use self::assign::prefixed_command;
use self::boxes::{begin_box, delete_last, scan_box, unpackage};
use self::lists::{
    app_space, append_discretionary, append_glue, append_italic_correction, append_kern,
    append_penalty, begin_insert_or_adjust, end_graf, font_space_glue, handle_right_brace,
    head_for_vmode, indent_in_hmode, its_all_over, make_accent, make_mark,
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

pub(crate) fn shift_case(st: &mut EngineState) -> Result<(), Abort> {
    let b = st.scanner.chr_loc();
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

fn valign_or_lr(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.cur_chr <= 0 {
        return init_align(st);
    }
    let enabled = crate::scan::eTeX_enabled(
        st,
        st.eqtb.int_par(IntPar::Texxet) > 0,
        st.scanner.cur_cmd,
        st.scanner.cur_chr,
    )?;
    if enabled {
        let m = st.mem.new_math(Scaled::ZERO, st.scanner.cur_chr as u16)?;
        st.input.cur_list.tail_append(&mut st.mem, m.ptr());
    }
    Ok(())
}

/// Runs `action` if the math-shift group is open, else inserts what closes the current group.
fn in_math_shift_group(
    st: &mut EngineState,
    action: fn(&mut EngineState) -> Result<(), Abort>,
) -> Result<(), Abort> {
    if st.save_stack.cur_group == GroupCode::MathShift {
        action(st)
    } else {
        off_save(st)
    }
}

/// Appends a noad of type `cur_chr` (or an ordinary one for `{`) and scans its nucleus.
fn append_noad(st: &mut EngineState, backup: bool) -> Result<(), Abort> {
    let n = st.mem.new_noad()?;
    st.input.cur_list.tail_append(&mut st.mem, n.ptr());
    if backup {
        crate::input::back_input(st)?;
    } else {
        n.set_type_code(&mut st.mem, st.scanner.cur_chr as u16);
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

pub(crate) fn main_control_ignore_spaces(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.cur_chr == 0 {
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

    match (mode, cmd) {
        (HMODE, Cmd::Spacer) => {
            if st.input.cur_list.space_factor() == 1000 {
                main_control_append_normal_space(st)?;
            } else {
                app_space(st)?;
            }
        }
        (HMODE | MMODE, Cmd::ExSpace) => main_control_append_normal_space(st)?,
        (_, Cmd::Relax) | (VMODE | MMODE, Cmd::Spacer) | (MMODE, Cmd::NoBoundary) => {}
        (_, Cmd::IgnoreSpaces) => {
            main_control_ignore_spaces(st)?;
            return Ok(Next::Reswitch);
        }
        (VMODE, Cmd::Stop) => {
            if its_all_over(st)? {
                return Ok(Next::Stop);
            }
        }

        (VMODE, Cmd::Vmove)
        | (HMODE | MMODE, Cmd::Hmove)
        | (_, Cmd::LastItem)
        | (VMODE, Cmd::Vadjust | Cmd::ItalCorr)
        | (VMODE | HMODE, Cmd::EqNo)
        | (_, Cmd::MacParam) => crate::errors::report_illegal_case(st)?,
        (
            VMODE | HMODE,
            Cmd::SupMark
            | Cmd::SubMark
            | Cmd::MathCharNum
            | Cmd::MathGiven
            | Cmd::XetexMathGiven
            | Cmd::MathComp
            | Cmd::DelimNum
            | Cmd::LeftRight
            | Cmd::Above
            | Cmd::Radical
            | Cmd::MathStyle
            | Cmd::MathChoice
            | Cmd::Vcenter
            | Cmd::NonScript
            | Cmd::Mkern
            | Cmd::LimitSwitch
            | Cmd::Mskip
            | Cmd::MathAccent,
        )
        | (
            MMODE,
            Cmd::Endv
            | Cmd::ParEnd
            | Cmd::Stop
            | Cmd::Vskip
            | Cmd::UnVbox
            | Cmd::Valign
            | Cmd::Hrule,
        ) => crate::scan::insert_dollar_sign(st)?,

        (VMODE, Cmd::Hrule) | (HMODE | MMODE, Cmd::Vrule) => append_rule(st)?,
        (VMODE, Cmd::Vskip) | (HMODE | MMODE, Cmd::Hskip) | (MMODE, Cmd::Mskip) => {
            append_glue(st)?;
        }
        (_, Cmd::Kern) | (MMODE, Cmd::Mkern) => append_kern(st)?,
        (VMODE | HMODE, Cmd::LeftBrace) => st
            .save_stack
            .new_save_level(GroupCode::Simple, st.mem.line)?,
        (_, Cmd::BeginGroup) => st
            .save_stack
            .new_save_level(GroupCode::SemiSimple, st.mem.line)?,
        (_, Cmd::EndGroup) => {
            if st.save_stack.cur_group == GroupCode::SemiSimple {
                crate::save::unsave(st)?;
            } else {
                off_save(st)?;
            }
        }

        (_, Cmd::RightBrace) => handle_right_brace(st)?,
        (VMODE, Cmd::Hmove) | (HMODE | MMODE, Cmd::Vmove) => {
            let t = st.scanner.cur_chr;
            let d = crate::scan::scan_dimen(st, false, false, false)?;
            let shift = if t == 0 { d } else { -d };
            scan_box(st, shift.raw())?;
        }
        (_, Cmd::LeaderShip) => {
            scan_box(st, LEADER_FLAG - i32::from(A_LEADERS) + st.scanner.cur_chr)?
        }
        (_, Cmd::MakeBox) => begin_box(st, 0)?,
        (VMODE, Cmd::StartPar) => new_graf(st, st.scanner.cur_chr > 0)?,
        (
            VMODE,
            Cmd::Letter
            | Cmd::OtherChar
            | Cmd::CharNum
            | Cmd::CharGiven
            | Cmd::MathShift
            | Cmd::UnHbox
            | Cmd::Vrule
            | Cmd::Accent
            | Cmd::Discretionary
            | Cmd::Hskip
            | Cmd::Valign
            | Cmd::ExSpace
            | Cmd::NoBoundary,
        ) => {
            crate::input::back_input(st)?;
            new_graf(st, true)?;
        }
        (HMODE | MMODE, Cmd::StartPar) => indent_in_hmode(st)?,
        (VMODE, Cmd::ParEnd) => {
            normal_paragraph(st)?;
            if st.input.cur_list.mode > 0 {
                crate::pagebuilder::build_page(st)?;
            }
        }
        (HMODE, Cmd::ParEnd) => {
            if st.scanner.align_state < 0 {
                off_save(st)?;
            }
            end_graf(st)?;
            if st.input.cur_list.mode == VMODE {
                crate::pagebuilder::build_page(st)?;
            }
        }
        (HMODE, Cmd::Stop | Cmd::Vskip | Cmd::Hrule | Cmd::UnVbox | Cmd::Halign) => {
            head_for_vmode(st)?
        }
        (_, Cmd::Insert) | (HMODE | MMODE, Cmd::Vadjust) => begin_insert_or_adjust(st)?,
        (_, Cmd::Mark) => make_mark(st)?,
        (_, Cmd::BreakPenalty) => append_penalty(st)?,
        (_, Cmd::RemoveItem) => delete_last(st)?,
        (VMODE, Cmd::UnVbox) | (HMODE | MMODE, Cmd::UnHbox) => unpackage(st)?,
        (HMODE, Cmd::ItalCorr) => append_italic_correction(st)?,
        (MMODE, Cmd::ItalCorr) => {
            let k = st.mem.new_kern(Scaled::ZERO)?;
            st.input.cur_list.tail_append(&mut st.mem, k.ptr());
        }
        (HMODE | MMODE, Cmd::Discretionary) => append_discretionary(st)?,
        (HMODE, Cmd::Accent) => make_accent(st)?,
        (_, Cmd::CarRet | Cmd::TabMark) => align_error(st)?,
        (_, Cmd::NoAlign) => no_align_error(st)?,
        (_, Cmd::Omit) => omit_error(st)?,
        (VMODE, Cmd::Halign) => init_align(st)?,
        (HMODE, Cmd::Valign) => valign_or_lr(st)?,
        (MMODE, Cmd::Halign) => {
            if crate::errors::privileged(st)? {
                in_math_shift_group(st, init_align)?;
            }
        }
        (VMODE | HMODE, Cmd::Endv) => do_endv(st)?,
        (_, Cmd::EndCsName) => crate::errors::cs_error(st)?,

        (HMODE, Cmd::MathShift) => init_math(st)?,
        (MMODE, Cmd::EqNo) => {
            if crate::errors::privileged(st)? {
                in_math_shift_group(st, crate::math::start_eq_no)?;
            }
        }
        (MMODE, Cmd::LeftBrace) => append_noad(st, true)?,
        (MMODE, Cmd::Letter | Cmd::OtherChar | Cmd::CharGiven) => {
            crate::scan::set_math_char(st, st.eqtb.math_code(st.scanner.cur_chr))?
        }
        (MMODE, Cmd::CharNum) => {
            st.scanner.cur_chr = crate::scan::scan_char_num(st)?;
            crate::scan::set_math_char(st, st.eqtb.math_code(st.scanner.cur_chr))?;
        }
        (MMODE, Cmd::MathCharNum) => {
            let c = crate::scan::scan_math_char_num(st)?;
            crate::scan::set_math_char(st, c)?;
        }
        (MMODE, Cmd::MathGiven) => {
            crate::scan::set_math_char(st, crate::scan::math_code_from_tex(st.scanner.cur_chr))?
        }
        (MMODE, Cmd::XetexMathGiven) => crate::scan::set_math_char(st, st.scanner.cur_chr)?,
        (MMODE, Cmd::DelimNum) => {
            let c = crate::scan::scan_delim_num(st)?;
            crate::scan::set_math_char(st, c)?;
        }
        (MMODE, Cmd::MathComp) => append_noad(st, false)?,
        (MMODE, Cmd::LimitSwitch) => crate::math::math_limit_switch(st)?,
        (MMODE, Cmd::Radical) => crate::math::math_radical(st)?,
        (MMODE, Cmd::Accent | Cmd::MathAccent) => crate::math::math_ac(st)?,
        (MMODE, Cmd::Vcenter) => math_vcenter(st)?,
        (MMODE, Cmd::MathStyle) => {
            let s = st.mem.new_style(st.scanner.cur_chr as i16)?;
            st.input.cur_list.tail_append(&mut st.mem, s.ptr());
        }
        (MMODE, Cmd::NonScript) => {
            let g = st.mem.new_glue(ZERO_GLUE)?;
            st.input.cur_list.tail_append(&mut st.mem, g.ptr());
            g.set_subtype(&mut st.mem, COND_MATH_GLUE);
        }
        (MMODE, Cmd::MathChoice) => crate::math::append_choices(st)?,
        (MMODE, Cmd::SubMark | Cmd::SupMark) => crate::math::sub_sup(st)?,
        (MMODE, Cmd::Above) => crate::math::math_fraction(st)?,
        (MMODE, Cmd::LeftRight) => crate::math::math_left_right(st)?,
        (MMODE, Cmd::MathShift) => in_math_shift_group(st, after_math)?,

        (_, cmd) if cmd.is_prefixed() => prefixed_command(st)?,
        (_, Cmd::AfterAssignment) => {
            crate::scan::get_token(st)?;
            st.scanner.after_token = st.scanner.cur_tok;
        }
        (_, Cmd::AfterGroup) => {
            crate::scan::get_token(st)?;
            st.save_stack.save_for_after(st.scanner.cur_tok)?;
        }
        (_, Cmd::InStream) => crate::io::open_or_close(st)?,
        (_, Cmd::Message) => crate::output::issue_message(st)?,
        (_, Cmd::CaseShift) => shift_case(st)?,
        (_, Cmd::Xray) => crate::output::show_whatever(st)?,
        (_, Cmd::Extension) => do_extension(st)?,
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
