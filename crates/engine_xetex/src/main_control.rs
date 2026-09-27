//! The main control loop and its command dispatch.

use crate::cmd::{
    ABOVE, ACCENT, AFTER_ASSIGNMENT, AFTER_GROUP, BEGIN_GROUP, BREAK_PENALTY, CAR_RET, CASE_SHIFT,
    CHAR_GIVEN, CHAR_NUM, DELIM_NUM, DISCRETIONARY, ENDV, END_CS_NAME, END_GROUP, EQ_NO, EXTENSION,
    EX_SPACE, HALIGN, HMOVE, HRULE, HSKIP, IGNORE_SPACES, INSERT, IN_STREAM, ITAL_CORR, KERN,
    LAST_ITEM, LEADER_SHIP, LEFT_BRACE, LEFT_RIGHT, LETTER, LIMIT_SWITCH, MAC_PARAM, MAKE_BOX,
    MARK, MATH_ACCENT, MATH_CHAR_NUM, MATH_CHOICE, MATH_COMP, MATH_GIVEN, MATH_SHIFT, MATH_STYLE,
    MESSAGE, MKERN, MSKIP, NON_SCRIPT, NO_ALIGN, NO_BOUNDARY, OMIT, OTHER_CHAR, PAR_END, RADICAL,
    RELAX, REMOVE_ITEM, RIGHT_BRACE, SET_INTERACTION, SPACER, START_PAR, STOP, SUB_MARK, SUP_MARK,
    TAB_MARK, TOKS_REGISTER, UN_HBOX, UN_VBOX, VADJUST, VALIGN, VCENTER, VMOVE, VRULE, VSKIP,
    XETEX_MATH_GIVEN, XRAY,
};
use crate::codes::ScannerStatus;
use crate::consts::{
    A_LEADERS, COND_MATH_GLUE, HMODE, IGNORE_DEPTH, MMODE, PRIM_EQTB_BASE, SINGLE_BASE,
    UNDEFINED_PRIMITIVE, VMODE,
};
use crate::errors::Abort;
use crate::ext::do_extension;
use crate::input::TokenType;
use crate::math::{after_math, init_math};
use crate::mem::ZERO_GLUE;
use crate::params::{GluePar, IntPar, TokPar};
use crate::state::{EngineState, GroupCode};
use crate::token::Token;
use crate::units::{MemPtr, Scaled};

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
    let b = st.scanner.cur_chr;
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
    st.mem.free_avail(def_ref);
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
            if st.scanner.cur_cmd != SPACER {
                break;
            }
        }
    } else {
        let saved = st.scanner.scanner_status;
        st.scanner.scanner_status = ScannerStatus::Normal;
        let outcome = crate::scan::get_next(st);
        st.scanner.scanner_status = saved;
        outcome?;

        st.scanner.cur_cs = st.eqtb.prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)?;

        if st.scanner.cur_cs != UNDEFINED_PRIMITIVE {
            st.scanner.cur_cmd = st.eqtb.eq_type(PRIM_EQTB_BASE + st.scanner.cur_cs);
            st.scanner.cur_chr = st.eqtb.equiv(PRIM_EQTB_BASE + st.scanner.cur_cs);
            st.scanner.cur_tok = Token::from_cs(PRIM_EQTB_BASE + st.scanner.cur_cs);
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
            LETTER | OTHER_CHAR | CHAR_GIVEN | CHAR_NUM => {
                if cmd == CHAR_NUM {
                    crate::scan::scan_usv_num(st)?;
                    st.scanner.cur_chr = st.scanner.cur_val;
                }
                return Ok(if main_control_main_loop(st)? {
                    Next::BigSwitch
                } else {
                    Next::Reswitch
                });
            }
            NO_BOUNDARY => {
                crate::scan::get_x_token(st)?;
                if matches!(
                    st.scanner.cur_cmd,
                    LETTER | OTHER_CHAR | CHAR_GIVEN | CHAR_NUM
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
        (HMODE, SPACER) => {
            if st.input.cur_list.space_factor() == 1000 {
                main_control_append_normal_space(st)?;
            } else {
                app_space(st)?;
            }
        }
        (HMODE | MMODE, EX_SPACE) => main_control_append_normal_space(st)?,
        (_, RELAX) | (VMODE | MMODE, SPACER) | (MMODE, NO_BOUNDARY) => {}
        (_, IGNORE_SPACES) => {
            main_control_ignore_spaces(st)?;
            return Ok(Next::Reswitch);
        }
        (VMODE, STOP) => {
            if its_all_over(st)? {
                return Ok(Next::Stop);
            }
        }

        (VMODE, VMOVE)
        | (HMODE | MMODE, HMOVE)
        | (_, LAST_ITEM)
        | (VMODE, VADJUST | ITAL_CORR)
        | (VMODE | HMODE, EQ_NO)
        | (_, MAC_PARAM) => crate::errors::report_illegal_case(st)?,
        (
            VMODE | HMODE,
            SUP_MARK | SUB_MARK | MATH_CHAR_NUM | MATH_GIVEN | XETEX_MATH_GIVEN | MATH_COMP
            | DELIM_NUM | LEFT_RIGHT | ABOVE | RADICAL | MATH_STYLE | MATH_CHOICE | VCENTER
            | NON_SCRIPT | MKERN | LIMIT_SWITCH | MSKIP | MATH_ACCENT,
        )
        | (MMODE, ENDV | PAR_END | STOP | VSKIP | UN_VBOX | VALIGN | HRULE) => {
            crate::scan::insert_dollar_sign(st)?
        }

        (VMODE, HRULE) | (HMODE | MMODE, VRULE) => append_rule(st)?,
        (VMODE, VSKIP) | (HMODE | MMODE, HSKIP) | (MMODE, MSKIP) => {
            append_glue(st)?;
        }
        (_, KERN) | (MMODE, MKERN) => append_kern(st)?,
        (VMODE | HMODE, LEFT_BRACE) => st
            .save_stack
            .new_save_level(GroupCode::Simple, st.mem.line)?,
        (_, BEGIN_GROUP) => st
            .save_stack
            .new_save_level(GroupCode::SemiSimple, st.mem.line)?,
        (_, END_GROUP) => {
            if st.save_stack.cur_group == GroupCode::SemiSimple {
                crate::save::unsave(st)?;
            } else {
                off_save(st)?;
            }
        }

        (_, RIGHT_BRACE) => handle_right_brace(st)?,
        (VMODE, HMOVE) | (HMODE | MMODE, VMOVE) => {
            let t = st.scanner.cur_chr;
            crate::scan::scan_dimen(st, false, false, false)?;
            let shift = if t == 0 {
                st.scanner.cur_val
            } else {
                -st.scanner.cur_val
            };
            scan_box(st, shift)?;
        }
        (_, LEADER_SHIP) => scan_box(st, LEADER_FLAG - i32::from(A_LEADERS) + st.scanner.cur_chr)?,
        (_, MAKE_BOX) => begin_box(st, 0)?,
        (VMODE, START_PAR) => new_graf(st, st.scanner.cur_chr > 0)?,
        (
            VMODE,
            LETTER | OTHER_CHAR | CHAR_NUM | CHAR_GIVEN | MATH_SHIFT | UN_HBOX | VRULE | ACCENT
            | DISCRETIONARY | HSKIP | VALIGN | EX_SPACE | NO_BOUNDARY,
        ) => {
            crate::input::back_input(st)?;
            new_graf(st, true)?;
        }
        (HMODE | MMODE, START_PAR) => indent_in_hmode(st)?,
        (VMODE, PAR_END) => {
            normal_paragraph(st)?;
            if st.input.cur_list.mode > 0 {
                crate::pagebuilder::build_page(st)?;
            }
        }
        (HMODE, PAR_END) => {
            if st.scanner.align_state < 0 {
                off_save(st)?;
            }
            end_graf(st)?;
            if st.input.cur_list.mode == VMODE {
                crate::pagebuilder::build_page(st)?;
            }
        }
        (HMODE, STOP | VSKIP | HRULE | UN_VBOX | HALIGN) => head_for_vmode(st)?,
        (_, INSERT) | (HMODE | MMODE, VADJUST) => begin_insert_or_adjust(st)?,
        (_, MARK) => make_mark(st)?,
        (_, BREAK_PENALTY) => append_penalty(st)?,
        (_, REMOVE_ITEM) => delete_last(st)?,
        (VMODE, UN_VBOX) | (HMODE | MMODE, UN_HBOX) => unpackage(st)?,
        (HMODE, ITAL_CORR) => append_italic_correction(st)?,
        (MMODE, ITAL_CORR) => {
            let k = st.mem.new_kern(Scaled::ZERO)?;
            st.input.cur_list.tail_append(&mut st.mem, k.ptr());
        }
        (HMODE | MMODE, DISCRETIONARY) => append_discretionary(st)?,
        (HMODE, ACCENT) => make_accent(st)?,
        (_, CAR_RET | TAB_MARK) => align_error(st)?,
        (_, NO_ALIGN) => no_align_error(st)?,
        (_, OMIT) => omit_error(st)?,
        (VMODE, HALIGN) => init_align(st)?,
        (HMODE, VALIGN) => valign_or_lr(st)?,
        (MMODE, HALIGN) => {
            if crate::errors::privileged(st)? {
                in_math_shift_group(st, init_align)?;
            }
        }
        (VMODE | HMODE, ENDV) => do_endv(st)?,
        (_, END_CS_NAME) => crate::errors::cs_error(st)?,

        (HMODE, MATH_SHIFT) => init_math(st)?,
        (MMODE, EQ_NO) => {
            if crate::errors::privileged(st)? {
                in_math_shift_group(st, crate::math::start_eq_no)?;
            }
        }
        (MMODE, LEFT_BRACE) => append_noad(st, true)?,
        (MMODE, LETTER | OTHER_CHAR | CHAR_GIVEN) => {
            crate::scan::set_math_char(st, st.eqtb.math_code(st.scanner.cur_chr))?
        }
        (MMODE, CHAR_NUM) => {
            crate::scan::scan_char_num(st)?;
            st.scanner.cur_chr = st.scanner.cur_val;
            crate::scan::set_math_char(st, st.eqtb.math_code(st.scanner.cur_chr))?;
        }
        (MMODE, MATH_CHAR_NUM) => {
            let c = crate::scan::scan_math_char_num(st)?;
            crate::scan::set_math_char(st, c)?;
        }
        (MMODE, MATH_GIVEN) => {
            crate::scan::set_math_char(st, crate::scan::math_code_from_tex(st.scanner.cur_chr))?
        }
        (MMODE, XETEX_MATH_GIVEN) => crate::scan::set_math_char(st, st.scanner.cur_chr)?,
        (MMODE, DELIM_NUM) => {
            let c = crate::scan::scan_delim_num(st)?;
            crate::scan::set_math_char(st, c)?;
        }
        (MMODE, MATH_COMP) => append_noad(st, false)?,
        (MMODE, LIMIT_SWITCH) => crate::math::math_limit_switch(st)?,
        (MMODE, RADICAL) => crate::math::math_radical(st)?,
        (MMODE, ACCENT | MATH_ACCENT) => crate::math::math_ac(st)?,
        (MMODE, VCENTER) => math_vcenter(st)?,
        (MMODE, MATH_STYLE) => {
            let s = st.mem.new_style(st.scanner.cur_chr as i16)?;
            st.input.cur_list.tail_append(&mut st.mem, s.ptr());
        }
        (MMODE, NON_SCRIPT) => {
            let g = st.mem.new_glue(ZERO_GLUE)?;
            st.input.cur_list.tail_append(&mut st.mem, g.ptr());
            g.set_subtype(&mut st.mem, COND_MATH_GLUE);
        }
        (MMODE, MATH_CHOICE) => crate::math::append_choices(st)?,
        (MMODE, SUB_MARK | SUP_MARK) => crate::math::sub_sup(st)?,
        (MMODE, ABOVE) => crate::math::math_fraction(st)?,
        (MMODE, LEFT_RIGHT) => crate::math::math_left_right(st)?,
        (MMODE, MATH_SHIFT) => in_math_shift_group(st, after_math)?,

        (_, TOKS_REGISTER..=SET_INTERACTION) => prefixed_command(st)?,
        (_, AFTER_ASSIGNMENT) => {
            crate::scan::get_token(st)?;
            st.scanner.after_token = st.scanner.cur_tok;
        }
        (_, AFTER_GROUP) => {
            crate::scan::get_token(st)?;
            st.save_stack.save_for_after(st.scanner.cur_tok)?;
        }
        (_, IN_STREAM) => crate::io::open_or_close(st)?,
        (_, MESSAGE) => crate::output::issue_message(st)?,
        (_, CASE_SHIFT) => shift_case(st)?,
        (_, XRAY) => crate::output::show_whatever(st)?,
        (_, EXTENSION) => do_extension(st)?,
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
