//! Building math lists: the math-mode commands and `fin_mlist`.

use crate::cmd::Cmd;
use crate::codes::AboveCode;
use crate::codes::DelimNumCode;
use crate::codes::LeftRightCode;
use crate::codes::LimitCode;
use crate::codes::{MathAccentCode, RadicalCode};
use crate::command::Command;
use crate::consts::{EMPTY, MATH_CHAR, NUMBER_MATH_FAMILIES, SUB_MLIST};
use crate::errors::Abort;
use crate::input::TokenType;
use crate::mem_types::ListStateRecord;
use crate::node::{
    AccentNoad, ChoiceNode, FenceNoad, FenceSubtype, FractionNoad, MathAccentKind, MathSlot, Noad,
    NoadVariant, NodePtr, RadicalNoad,
};
use crate::overlay::DelimiterWord;
use crate::params::{IntPar, TokPar};
use crate::state::{EngineState, EqtbState, GroupCode, MemState, SaveStackState};
use crate::units::{EqtbPtr, MemIdx, MemPtr, NodeType, Scaled};

use super::DEFAULT_CODE;

/// The family and character of a `\Udelimiter`, encoded with the extended flag.
fn scan_extended_delimiter(st: &mut EngineState) -> Result<i32, Abort> {
    let fam = crate::scan::scan_math_fam_int(st)?;
    let c = crate::scan::scan_usv_num(st)?;
    Ok(0x4000_0000 + fam * 0x20_0000 + c)
}

/// Scans a delimiter, after `\radical` or `\Uradical` when `radical` is given.
fn scan_delimiter(
    st: &mut EngineState,
    radical: Option<RadicalCode>,
) -> Result<DelimiterWord, Abort> {
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
        crate::errors::back_error(
            st,
            "Missing delimiter (. inserted)",
            &[
                r"I was expecting to see something like `(' or `\{' or",
                r"`\}' here. If you typed, e.g., `{' instead of `\{', you",
                "should probably delete the `{' by typing `1' now, so that",
                "braces don't get unbalanced. Otherwise just proceed.",
                r"Acceptable delimiters are characters whose \delcode is",
                r"nonnegative, or you can use `\delimiter <delimiter code>'.",
            ],
        )?;
        v = 0;
    }

    let mut d = <DelimiterWord as bytemuck::Zeroable>::zeroed();
    if v >= 0x4000_0000 {
        let plane_and_fam = ((v % 0x20_0000) / 0x1_0000) * 0x100 + (v / 0x20_0000) % 0x100;
        d.set_small(plane_and_fam as u16, (v % 0x1_0000) as u16);
        d.set_large(0, 0);
    } else {
        d.set_small(((v / 0x10_0000) % 16) as u16, ((v / 0x1000) % 0x100) as u16);
        d.set_large(((v / 0x100) % 16) as u16, (v % 0x100) as u16);
    }
    Ok(d)
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
    fn apply_math_limit_switch(&mut self, mem: &mut MemState, limit: LimitCode) -> bool {
        let tail = self.tail;
        if self.head == tail {
            return false;
        }
        let Ok(Some(any)) = mem.classify_noad(tail) else {
            return false;
        };
        match any.variant(mem) {
            NoadVariant::Noad(n) if mem[n].noad_type == NodeType::OpNoad => {
                mem[n].set_limits(limit);
                true
            }
            _ => false,
        }
    }
}

pub(crate) fn math_limit_switch(st: &mut EngineState, code: LimitCode) -> Result<(), Abort> {
    if st.input.cur_list.apply_math_limit_switch(&mut st.mem, code) {
        return Ok(());
    }
    crate::errors::error(
        st,
        "Limit controls must follow a math operator",
        &[r"I'm ignoring this misplaced \limits or \nolimits command."],
    )
}

impl ListStateRecord {
    fn append_radical_noad(&mut self, mem: &mut MemState) -> Result<NodePtr<RadicalNoad>, Abort> {
        let node = NodePtr::<RadicalNoad>::alloc(mem)?;
        self.tail_append(mem, node.ptr());
        mem[node].base.clear_fields();
        Ok(node)
    }
}

pub(crate) fn math_radical(st: &mut EngineState, code: RadicalCode) -> Result<(), Abort> {
    let tail = st.input.cur_list.append_radical_noad(&mut st.mem)?;
    st.mem[tail].left_delimiter = scan_delimiter(st, Some(code))?;
    crate::scan::scan_math(st, MathSlot::Nucleus(tail.as_noad()))
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
            let list = self.head.link(mem);
            let d = &mut mem[aux_noad].denominator;
            d.math_type = SUB_MLIST;
            d.set_info(list);
            if p.is_null() {
                q = aux_noad.ptr();
            } else {
                q = mem[aux_noad].numerator.info();
                let etex_aux = self.e_tex_aux;
                if mem.node_type(q) != Ok(NodeType::LeftNoad) || etex_aux.is_null() {
                    return Err(Abort::Confusion("right"));
                }
                let etex_link = etex_aux.link(mem);
                mem[aux_noad].numerator.set_info(etex_link);
                etex_aux.set_link(mem, aux_noad.ptr());
                mem[aux_noad].link = p;
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
    ) -> Result<(MathSlot, Option<ScriptDuplication>), Abort> {
        let sup = cmd == Cmd::SupMark;
        let script = |n: NodePtr<Noad>| {
            if sup {
                MathSlot::Supscr(n)
            } else {
                MathSlot::Subscr(n)
            }
        };
        let mut t = EMPTY;
        let mut p = None;

        if self.tail != self.head {
            if let Ok(Some(any)) = mem.classify_noad(self.tail) {
                // WEB's `scripts_allowed` admits fractions, whose script words are the numerator and denominator.
                let field = match any.variant(mem) {
                    NoadVariant::Noad(n) => Some(script(n)),
                    NoadVariant::Radical(r) => Some(script(r.as_noad())),
                    NoadVariant::Accent(a) => Some(script(a.as_noad())),
                    NoadVariant::Fraction(f) => Some(if sup {
                        MathSlot::Numerator(f)
                    } else {
                        MathSlot::Denominator(f)
                    }),
                    NoadVariant::Fence(_) => None,
                };
                if let Some(field) = field {
                    t = mem[field].math_type;
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
                    duplicate = Some(if sup {
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
        let (message, help) = match duplicate {
            ScriptDuplication::Superscript => (
                "Double superscript",
                "I treat `x^1^2' essentially like `x^1{}^2'.",
            ),
            ScriptDuplication::Subscript => (
                "Double subscript",
                "I treat `x_1_2' essentially like `x_1{}_2'.",
            ),
        };
        crate::errors::error(st, message, &[help])?;
    }
    crate::scan::scan_math(st, p)
}

impl ListStateRecord {
    fn start_fraction_noad(&mut self, mem: &mut MemState) -> Result<NodePtr<FractionNoad>, Abort> {
        let fraction = NodePtr::<FractionNoad>::alloc(mem)?;
        self.set_incompleat_noad(Some(fraction));
        let list = self.head.link(mem);
        let f = &mut mem[fraction];
        f.new_hlist = MemPtr::NULL;
        f.numerator.math_type = SUB_MLIST;
        f.numerator.set_info(list);
        f.denominator.clear();
        f.left_delimiter.clear();
        f.right_delimiter.clear();

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
            scan_delimiter(st, None)?;
            scan_delimiter(st, None)?;
        }

        if kind == AboveCode::Above {
            crate::scan::scan_dimen(st, false)?;
        }

        crate::errors::error(
            st,
            "Ambiguous; you need another { and }",
            &[
                "I'm ignoring this fraction specification, since I don't",
                r"know whether a construction like `x \over y \over z'",
                r"means `{x \over y} \over z' or `x \over {y \over z}'.",
            ],
        )?;
    } else {
        let aux = st.input.cur_list.start_fraction_noad(&mut st.mem)?;

        if delimited {
            st.mem[aux].left_delimiter = scan_delimiter(st, None)?;
            st.mem[aux].right_delimiter = scan_delimiter(st, None)?;
        }

        match kind {
            AboveCode::Over => st.mem[aux].thickness = DEFAULT_CODE,
            AboveCode::Atop => st.mem[aux].thickness = Scaled::ZERO,
            _ => {
                let d = crate::scan::scan_dimen(st, false)?;
                st.mem[aux].thickness = d;
            }
        }
    }
    Ok(())
}

impl ListStateRecord {
    fn append_accent_noad(&mut self, mem: &mut MemState) -> Result<NodePtr<AccentNoad>, Abort> {
        let accent = NodePtr::<AccentNoad>::alloc(mem)?;
        self.tail_append(mem, accent.ptr());
        let a = &mut mem[accent];
        a.base.clear_fields();
        a.accent_chr.math_type = MATH_CHAR;
        Ok(accent)
    }
}

impl MemState {
    fn finish_accent_math_field(
        &mut self,
        eqtb: &EqtbState,
        tail: NodePtr<AccentNoad>,
        value: i32,
    ) {
        self[tail].accent_chr.set_character((value % 65536) as u16);
        let intpar_cur_fam = eqtb.int_par(IntPar::CurFam);
        if crate::scan::math_class(value) == 7
            && (0..NUMBER_MATH_FAMILIES).contains(&intpar_cur_fam)
        {
            self[tail]
                .accent_chr
                .set_plane_and_fam(intpar_cur_fam as u16);
        } else {
            self[tail]
                .accent_chr
                .set_plane_and_fam(crate::scan::math_fam(value) as u16);
        }
        let current_fam = self[tail].accent_chr.plane_and_fam();
        self[tail]
            .accent_chr
            .set_plane_and_fam(current_fam + (crate::scan::math_char(value) / 65536) as u16 * 256);
    }
}

impl MemState {
    /// A fence noad for `code`. A middle one is a right noad with a middle subtype.
    fn new_fence(&mut self, code: LeftRightCode) -> Result<NodePtr<FenceNoad>, Abort> {
        let p = NodePtr::<FenceNoad>::alloc(self)?;
        let n = &mut self[p];
        n.subtype = FenceSubtype::Normal.into();
        n.new_hlist = MemPtr::NULL;
        n.delimiter.clear();
        n.supscr.clear();
        n.subscr.clear();
        match code {
            LeftRightCode::Left => {}
            LeftRightCode::Right => n.noad_type = NodeType::RightNoad.into(),
            LeftRightCode::Middle => {
                n.noad_type = NodeType::RightNoad.into();
                n.subtype = FenceSubtype::Middle.into();
            }
        }
        Ok(p)
    }
}

pub(crate) fn math_left_right(st: &mut EngineState, code: LeftRightCode) -> Result<(), Abort> {
    if code != LeftRightCode::Left && st.save_stack.cur_group != GroupCode::MathLeft {
        if st.save_stack.cur_group == GroupCode::MathShift {
            scan_delimiter(st, None)?;
            let (name, help) = if code == LeftRightCode::Middle {
                (
                    "middle",
                    r"I'm ignoring a \middle that had no matching \left.",
                )
            } else {
                (
                    "right",
                    r"I'm ignoring a \right that had no matching \left.",
                )
            };
            crate::errors::error(st, ("Extra ", crate::output::Esc(name)), &[help])?;
        } else {
            crate::main_control::off_save(st)?;
        }
    } else {
        let p = st.mem.new_fence(code)?;
        st.mem[p].delimiter = scan_delimiter(st, None)?;
        let q = if code == LeftRightCode::Left {
            p.ptr()
        } else {
            let q = fin_mlist(st, p.ptr())?;
            crate::save::unsave(st)?;
            q
        };
        if code != LeftRightCode::Right {
            crate::main_control::push_math(st, GroupCode::MathLeft)?;
            let cl = &mut st.input.cur_list;
            cl.head.set_link(&mut st.mem, q);
            cl.tail = p.ptr();
            cl.e_tex_aux = p.ptr();
        } else {
            let inner = st.mem.new_noad()?;
            let cl = &mut st.input.cur_list;
            cl.tail_append(&mut st.mem, inner.ptr());
            let n = &mut st.mem[inner];
            n.noad_type = NodeType::InnerNoad.into();
            n.nucleus.math_type = SUB_MLIST;
            n.nucleus.set_info(q);
        }
    }
    Ok(())
}

/// `\mathaccent`, `\Umathaccent`, or `\accent` in math mode when `code` is `None`.
pub(crate) fn math_ac(st: &mut EngineState, code: Option<MathAccentCode>) -> Result<(), Abort> {
    if code.is_none() {
        crate::errors::error(
            st,
            (
                "Please use ",
                crate::output::Esc("mathaccent"),
                " for accents in math mode",
            ),
            &[
                r"I'm changing \accent to \mathaccent here; wish me luck.",
                "(Accents are not the same in formulas as they are in text.)",
            ],
        )?;
    }

    let tail = st.input.cur_list.append_accent_noad(&mut st.mem)?;

    let value = if code == Some(MathAccentCode::UmathAccent) {
        if crate::scan::scan_keyword(st, "fixed")? {
            st.mem[tail].set_kind(MathAccentKind::TopFixed);
        } else if crate::scan::scan_keyword(st, "bottom")? {
            if crate::scan::scan_keyword(st, "fixed")? {
                st.mem[tail].set_kind(MathAccentKind::BottomFixed);
            } else {
                st.mem[tail].set_kind(MathAccentKind::Bottom);
            }
        }
        crate::scan::scan_umath_code(st)?
    } else {
        crate::scan::math_code_from_tex(crate::scan::scan_fifteen_bit_int(st)?)
    };
    st.mem.finish_accent_math_field(&st.eqtb, tail, value);
    crate::scan::scan_math(st, MathSlot::Nucleus(tail.as_noad()))
}
