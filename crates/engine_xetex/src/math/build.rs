//! Building math lists: the math-mode commands and `fin_mlist`.

use crate::cmd::Cmd;
use crate::codes::AboveCode;
use crate::codes::DelimNumCode;
use crate::codes::LeftRightCode;
use crate::codes::LimitCode;
use crate::codes::{MathAccentCode, RadicalCode};
use crate::command::Command;
use crate::consts::{EMPTY, MATH_CHAR, NORMAL, NUMBER_MATH_FAMILIES, SUB_MLIST};
use crate::errors::Abort;
use crate::input::TokenType;
use crate::mem::GARBAGE;
use crate::mem_types::ListStateRecord;
use crate::node::{ChoiceNode, DelimiterField, MathField, Noad, NodePtr};
use crate::params::{IntPar, TokPar};
use crate::state::{EngineState, EqtbState, GroupCode, MemState, SaveStackState};
use crate::units::{EqtbPtr, MemIdx, MemPtr, NodeType, Scaled};

use super::{BOTTOM_ACC, DEFAULT_CODE, MIDDLE_NOAD};

const FIXED_ACC: u16 = 1;

/// The family and character of a `\Udelimiter`, encoded with the extended flag.
fn scan_extended_delimiter(st: &mut EngineState) -> Result<i32, Abort> {
    let fam = crate::scan::scan_math_fam_int(st)?;
    let c = crate::scan::scan_usv_num(st)?;
    Ok(0x4000_0000 + fam * 0x20_0000 + c)
}

/// Scans a delimiter into `p`, after `\radical` or `\Uradical` when `radical` is given.
fn scan_delimiter(
    st: &mut EngineState,
    p: NodePtr<DelimiterField>,
    radical: Option<RadicalCode>,
) -> Result<(), Abort> {
    let mut v = if let Some(radical) = radical {
        if radical == RadicalCode::Uradical {
            scan_extended_delimiter(st)?
        } else {
            crate::scan::scan_delimiter_int(st)?
        }
    } else {
        loop {
            crate::scan::get_x_token(st)?;
            if st.scanner.cur_cmd != Cmd::Spacer && st.scanner.cur_cmd != Cmd::Relax {
                break;
            }
        }

        match st.scanner.try_command() {
            Some(Command::Letter(c) | Command::OtherChar(c)) => st.eqtb.del_code(c.raw()),
            Some(Command::DelimNum(DelimNumCode::Udelimiter)) => {
                crate::scan::scan_math_class_int(st)?;
                scan_extended_delimiter(st)?
            }
            Some(Command::DelimNum(DelimNumCode::Delimiter)) => {
                crate::scan::scan_delimiter_int(st)?
            }
            _ => -1,
        }
    };

    if v < 0 {
        crate::output::print_error_prefix(st);
        crate::output::print(st, "Missing delimiter (. inserted)");
        st.errors.help(&[
            r"I was expecting to see something like `(' or `\{' or",
            r"`\}' here. If you typed, e.g., `{' instead of `\{', you",
            "should probably delete the `{' by typing `1' now, so that",
            "braces don't get unbalanced. Otherwise just proceed.",
            r"Acceptable delimiters are characters whose \delcode is",
            r"nonnegative, or you can use `\delimiter <delimiter code>'.",
        ]);
        crate::input::back_error(st)?;
        v = 0;
    }

    if v >= 0x4000_0000 {
        let plane_and_fam = ((v % 0x20_0000) / 0x1_0000) * 0x100 + (v / 0x20_0000) % 0x100;
        p.set_small(&mut st.mem, plane_and_fam as u16, (v % 0x1_0000) as u16);
        p.set_large(&mut st.mem, 0, 0);
    } else {
        p.set_small(
            &mut st.mem,
            ((v / 0x10_0000) % 16) as u16,
            ((v / 0x1000) % 0x100) as u16,
        );
        p.set_large(&mut st.mem, ((v / 0x100) % 16) as u16, (v % 0x100) as u16);
    }
    Ok(())
}

pub(super) fn define_word(st: &mut EngineState, loc: EqtbPtr, val: i32) -> Result<(), Abort> {
    st.eqtb.eq_word_define(&mut st.save_stack, loc, val)
}

pub(crate) fn start_eq_no(st: &mut EngineState, left: bool) -> Result<(), Abort> {
    st.save_stack.set_saved(0, i32::from(left));
    st.save_stack.save_ptr += 1;

    crate::main_control::push_math(st, GroupCode::MathShift)?;
    define_word(st, IntPar::CurFam.loc(), -1)?;
    if st.shipout.insert_src_special_every_math {
        crate::ext::insert_src_special(st)?;
    }
    let every_math = st.eqtb.tok_par(TokPar::EveryMath);
    if !every_math.is_null() {
        crate::input::begin_token_list(st, every_math, TokenType::EveryMath)?;
    }
    Ok(())
}

impl ListStateRecord {
    fn apply_math_limit_switch(&mut self, mem: &mut MemState, limit: u16) -> bool {
        let tail = self.tail;
        if self.head == tail {
            return false;
        }
        match mem.classify_noad(tail) {
            Ok(Some(noad)) if noad.noad_type(mem) == NodeType::OpNoad => {
                noad.set_subtype(mem, limit);
                true
            }
            _ => false,
        }
    }
}

pub(crate) fn math_limit_switch(st: &mut EngineState, code: LimitCode) -> Result<(), Abort> {
    if st
        .input
        .cur_list
        .apply_math_limit_switch(&mut st.mem, code as u16)
    {
        return Ok(());
    }
    crate::output::print_err(st, "Limit controls must follow a math operator");
    st.errors
        .help(&[r"I'm ignoring this misplaced \limits or \nolimits command."]);
    crate::errors::error(st)
}

impl ListStateRecord {
    fn append_radical_noad(&mut self, mem: &mut MemState) -> Result<NodePtr<Noad>, Abort> {
        let node = NodePtr::<Noad>::alloc_radical(mem)?;
        self.tail_append(mem, node.ptr());
        node.clear_fields(mem);
        Ok(node)
    }
}

pub(crate) fn math_radical(st: &mut EngineState, code: RadicalCode) -> Result<(), Abort> {
    let tail = st.input.cur_list.append_radical_noad(&mut st.mem)?;
    scan_delimiter(st, tail.left_delimiter(), Some(code))?;
    crate::scan::scan_math(st, tail.nucleus())
}

impl ListStateRecord {
    fn append_choices(
        &mut self,
        mem: &mut MemState,
        save_stack: &mut SaveStackState,
    ) -> Result<(), Abort> {
        let choice = mem.new_choice()?;
        self.tail_append(mem, choice.ptr());
        save_stack.save_ptr += 1;
        save_stack.set_saved(-1, 0);
        Ok(())
    }
}

pub(crate) fn append_choices(st: &mut EngineState) -> Result<(), Abort> {
    st.input
        .cur_list
        .append_choices(&mut st.mem, &mut st.save_stack)?;
    crate::main_control::push_math(st, GroupCode::MathChoice)?;
    crate::scan::scan_left_brace(st)?;
    Ok(())
}

impl ListStateRecord {
    fn fin_mlist(&mut self, mem: &mut MemState, p: MemPtr) -> Result<MemPtr, Abort> {
        let q;
        if let Some(aux_noad) = self.incompleat_noad() {
            aux_noad.subscr().set_math_type(mem, SUB_MLIST);
            aux_noad.subscr().set_info(mem, self.head.link(mem));
            if p.is_null() {
                q = aux_noad.ptr();
            } else {
                q = aux_noad.supscr().info(mem);
                let etex_aux = self.e_tex_aux;
                if mem.node_type(q) != Ok(NodeType::LeftNoad) || etex_aux.is_null() {
                    return Err(Abort::Confusion("right"));
                }
                let etex_link = etex_aux.link(mem);
                aux_noad.supscr().set_info(mem, etex_link);
                etex_aux.set_link(mem, aux_noad.ptr());
                aux_noad.set_link(mem, p);
            }
        } else {
            self.tail.set_link(mem, p);
            q = self.head.link(mem);
        }
        Ok(q)
    }
}

pub(crate) fn fin_mlist(st: &mut crate::state::EngineState, p: MemPtr) -> Result<MemPtr, Abort> {
    let q = st.input.cur_list.fin_mlist(&mut st.mem, p)?;
    st.mem.pop_nest(&mut st.input)?;
    Ok(q)
}

impl ListStateRecord {
    fn build_choices_finish(
        &mut self,
        mem: &mut MemState,
        save_stack: &mut SaveStackState,
        p: MemPtr,
    ) -> bool {
        // SAFETY: `append_choices` left the choice node as the enclosing list's tail.
        let tail = NodePtr::<ChoiceNode>::from_idx_unchecked(MemIdx::new(self.tail));
        match save_stack.saved(-1) {
            0 => mem[tail].display_mlist = p,
            1 => mem[tail].text_mlist = p,
            2 => mem[tail].script_mlist = p,
            3 => {
                mem[tail].script_script_mlist = p;
                save_stack.save_ptr -= 1;
                return true;
            }
            _ => {}
        }
        let choice_state = save_stack.saved(-1);
        save_stack.set_saved(-1, choice_state + 1);
        false
    }
}

pub(crate) fn build_choices(st: &mut crate::state::EngineState) -> Result<(), Abort> {
    crate::save::unsave(st)?;
    let p = st.input.cur_list.fin_mlist(&mut st.mem, MemPtr::NULL)?;
    st.mem.pop_nest(&mut st.input)?;
    let complete = st
        .input
        .cur_list
        .build_choices_finish(&mut st.mem, &mut st.save_stack, p);
    if complete {
        return Ok(());
    }
    crate::main_control::push_math(st, GroupCode::MathChoice)?;
    crate::scan::scan_left_brace(st)?;
    Ok(())
}

enum ScriptDuplication {
    Superscript,
    Subscript,
}

impl ListStateRecord {
    fn prepare_sub_sup(
        &mut self,
        mem: &mut MemState,
        cmd: Cmd,
    ) -> Result<(NodePtr<MathField>, Option<ScriptDuplication>), Abort> {
        let script = |n: NodePtr<Noad>| {
            if cmd == Cmd::SupMark {
                n.supscr()
            } else {
                n.subscr()
            }
        };
        let mut t = EMPTY;
        let mut p = None;

        if self.tail != self.head {
            if let Ok(Some(n)) = mem.classify_noad(self.tail) {
                if n.noad_type(mem).as_raw() < NodeType::LeftNoad.as_raw() {
                    let field = script(n);
                    t = field.math_type(mem);
                    p = Some(field);
                }
            }
        }

        let mut duplicate = None;
        let p = match p {
            Some(field) if t == EMPTY => field,
            _ => {
                let noad = mem.new_noad()?;
                self.tail_append(mem, noad.ptr());
                if t != EMPTY {
                    duplicate = Some(if cmd == Cmd::SupMark {
                        ScriptDuplication::Superscript
                    } else {
                        ScriptDuplication::Subscript
                    });
                }
                script(noad)
            }
        };
        Ok((p, duplicate))
    }
}

pub(crate) fn sub_sup(st: &mut EngineState) -> Result<(), Abort> {
    let cmd = st.scanner.cur_cmd;
    let (p, duplicate) = st.input.cur_list.prepare_sub_sup(&mut st.mem, cmd)?;
    if let Some(duplicate) = duplicate {
        match duplicate {
            ScriptDuplication::Superscript => {
                crate::output::error_here_with_diagnostic(st, "Double superscript");
                st.errors
                    .help(&["I treat `x^1^2' essentially like `x^1{}^2'."]);
            }
            ScriptDuplication::Subscript => {
                crate::output::error_here_with_diagnostic(st, "Double subscript");
                st.errors
                    .help(&["I treat `x_1_2' essentially like `x_1{}_2'."]);
            }
        }
        crate::output::capture_to_diagnostic(st, None);
        crate::errors::error(st)?;
    }
    crate::scan::scan_math(st, p)
}

impl ListStateRecord {
    fn start_fraction_noad(&mut self, mem: &mut MemState) -> Result<NodePtr<Noad>, Abort> {
        let fraction = NodePtr::<Noad>::alloc_fraction(mem)?;
        self.set_incompleat_noad(Some(fraction));
        fraction.set_subtype(mem, NORMAL);
        fraction.supscr().set_math_type(mem, SUB_MLIST);
        fraction.supscr().set_info(mem, self.head.link(mem));
        fraction.subscr().clear(mem);
        fraction.left_delimiter().clear(mem);
        fraction.right_delimiter().clear(mem);

        self.head.set_link(mem, MemPtr::NULL);
        self.tail = self.head;
        Ok(fraction)
    }
}

pub(crate) fn math_fraction(st: &mut EngineState, code: AboveCode) -> Result<(), Abort> {
    let delimited = matches!(
        code,
        AboveCode::AboveWithDelims | AboveCode::OverWithDelims | AboveCode::AtopWithDelims
    );
    let kind = match code {
        AboveCode::Above | AboveCode::AboveWithDelims => AboveCode::Above,
        AboveCode::Over | AboveCode::OverWithDelims => AboveCode::Over,
        AboveCode::Atop | AboveCode::AtopWithDelims => AboveCode::Atop,
    };

    if st.input.cur_list.incompleat_noad().is_some() {
        if delimited {
            scan_delimiter(st, NodePtr::at(GARBAGE), None)?;
            scan_delimiter(st, NodePtr::at(GARBAGE), None)?;
        }

        if kind == AboveCode::Above {
            crate::scan::scan_dimen(st, false, false)?;
        }

        crate::output::print_error_prefix(st);
        crate::output::print(st, "Ambiguous; you need another { and }");
        st.errors.help(&[
            "I'm ignoring this fraction specification, since I don't",
            r"know whether a construction like `x \over y \over z'",
            r"means `{x \over y} \over z' or `x \over {y \over z}'.",
        ]);
        crate::errors::error(st)?;
    } else {
        let aux = st.input.cur_list.start_fraction_noad(&mut st.mem)?;

        if delimited {
            scan_delimiter(st, aux.left_delimiter(), None)?;
            scan_delimiter(st, aux.right_delimiter(), None)?;
        }

        match kind {
            AboveCode::Over => aux.set_thickness(&mut st.mem, DEFAULT_CODE),
            AboveCode::Atop => aux.set_thickness(&mut st.mem, Scaled::ZERO),
            _ => {
                let d = crate::scan::scan_dimen(st, false, false)?;
                aux.set_thickness(&mut st.mem, d);
            }
        }
    }
    Ok(())
}

impl ListStateRecord {
    fn append_accent_noad(&mut self, mem: &mut MemState) -> Result<NodePtr<Noad>, Abort> {
        let accent = NodePtr::<Noad>::alloc_accent(mem)?;
        self.tail_append(mem, accent.ptr());
        accent.clear_fields(mem);
        accent.accent_chr().set_math_type(mem, MATH_CHAR);
        Ok(accent)
    }
}

impl MemState {
    fn finish_accent_math_field(&mut self, eqtb: &EqtbState, tail: NodePtr<Noad>, value: i32) {
        tail.accent_chr()
            .set_character(self, (value % 65536) as u16);
        let intpar_cur_fam = eqtb.int_par(IntPar::CurFam);
        if crate::scan::math_class(value) == 7
            && (0..NUMBER_MATH_FAMILIES).contains(&intpar_cur_fam)
        {
            tail.accent_chr()
                .set_plane_and_fam(self, intpar_cur_fam as u16);
        } else {
            tail.accent_chr()
                .set_plane_and_fam(self, crate::scan::math_fam(value) as u16);
        }
        let current_fam = tail.accent_chr().plane_and_fam(self);
        tail.accent_chr().set_plane_and_fam(
            self,
            current_fam + (crate::scan::math_char(value) / 65536) as u16 * 256,
        );
    }
}

pub(crate) fn math_left_right(st: &mut EngineState, code: LeftRightCode) -> Result<(), Abort> {
    let t = code as u16;
    if t != NodeType::LeftNoad.as_raw() && st.save_stack.cur_group != GroupCode::MathLeft {
        if st.save_stack.cur_group == GroupCode::MathShift {
            scan_delimiter(st, NodePtr::at(GARBAGE), None)?;
            crate::output::error_here_with_diagnostic(st, "Extra ");
            if t == MIDDLE_NOAD {
                crate::output::print_esc(st, "middle");
                st.errors
                    .help(&[r"I'm ignoring a \middle that had no matching \left."]);
            } else {
                crate::output::print_esc(st, "right");
                st.errors
                    .help(&[r"I'm ignoring a \right that had no matching \left."]);
            }
            crate::output::capture_to_diagnostic(st, None);
            crate::errors::error(st)?;
        } else {
            crate::main_control::off_save(st)?;
        }
    } else {
        let p = st.mem.new_noad()?;
        p.set_type_code(&mut st.mem, t);
        scan_delimiter(st, p.delimiter(), None)?;
        if t == MIDDLE_NOAD {
            p.set_noad_type(&mut st.mem, NodeType::RightNoad);
            p.set_subtype(&mut st.mem, MIDDLE_NOAD);
        }
        let q = if t == NodeType::LeftNoad.as_raw() {
            p.ptr()
        } else {
            let q = fin_mlist(st, p.ptr())?;
            crate::save::unsave(st)?;
            q
        };
        if t != NodeType::RightNoad.as_raw() {
            crate::main_control::push_math(st, GroupCode::MathLeft)?;
            let cl = &mut st.input.cur_list;
            cl.head.set_link(&mut st.mem, q);
            cl.tail = p.ptr();
            cl.e_tex_aux = p.ptr();
        } else {
            let inner = st.mem.new_noad()?;
            let cl = &mut st.input.cur_list;
            cl.tail_append(&mut st.mem, inner.ptr());
            inner.set_noad_type(&mut st.mem, NodeType::InnerNoad);
            inner.nucleus().set_math_type(&mut st.mem, SUB_MLIST);
            inner.nucleus().set_info(&mut st.mem, q);
        }
    }
    Ok(())
}

/// `\mathaccent`, `\Umathaccent`, or `\accent` in math mode when `code` is `None`.
pub(crate) fn math_ac(st: &mut EngineState, code: Option<MathAccentCode>) -> Result<(), Abort> {
    if code.is_none() {
        crate::output::error_here_with_diagnostic(st, "Please use ");
        crate::output::print_esc(st, "mathaccent");
        crate::output::print(st, " for accents in math mode");
        crate::output::capture_to_diagnostic(st, None);
        st.errors.help(&[
            r"I'm changing \accent to \mathaccent here; wish me luck.",
            "(Accents are not the same in formulas as they are in text.)",
        ]);
        crate::errors::error(st)?;
    }

    let tail = st.input.cur_list.append_accent_noad(&mut st.mem)?;

    let value = if code == Some(MathAccentCode::UmathAccent) {
        if crate::scan::scan_keyword(st, "fixed")? {
            tail.set_subtype(&mut st.mem, FIXED_ACC);
        } else if crate::scan::scan_keyword(st, "bottom")? {
            if crate::scan::scan_keyword(st, "fixed")? {
                tail.set_subtype(&mut st.mem, BOTTOM_ACC + 1);
            } else {
                tail.set_subtype(&mut st.mem, BOTTOM_ACC);
            }
        }
        crate::scan::scan_umath_code(st)?
    } else {
        crate::scan::math_code_from_tex(crate::scan::scan_fifteen_bit_int(st)?)
    };
    st.mem.finish_accent_math_field(&st.eqtb, tail, value);
    crate::scan::scan_math(st, tail.nucleus())
}
