//! Mode-independent processing: `prefixed_command` and its assignments.

use super::BoxContext;
use super::boxes::scan_box;
use crate::base::cmd::Cmd;
use crate::base::codes::{
    AuxCode, BoxDimenCode, FontIntCode, PageDimenCode, PageIntCode, ShorthandDefCode, ValueLevel,
};
use crate::base::command::Command;
use crate::base::command::RelaxCode;
use crate::base::command::Usv;
use crate::base::consts::NULL_FONT;
use crate::base::consts::{ACTIVE_MATH_CHAR, BIGGEST_USV, MAX_HALFWORD, TEX_INFINITY};
use crate::base::params::{IntPar, TokPar};
use crate::base::scaled_math::{mult_and_add, x_over_n};
use crate::base::token::{Token, TokenList};
use crate::base::units::{FontId, GlueOrder, Scaled, StrNumber};
use crate::build::hyphenation::{new_hyph_exceptions, new_patterns};
use crate::fonts::FontSize;
use crate::fonts::Side;
use crate::fonts::read_font_info;
use crate::memory::glue_store::GlueSpecId;
use crate::memory::mem::ZERO_GLUE;
use crate::memory::node::{ParShapeNode, PenaltiesNode};
use crate::print::Selector;
use crate::print::errors::Abort;
use crate::print::errors::InteractionMode;
use crate::print::errors::OrConfusion;
use crate::scanning::scan::fetch_box;
use crate::state::EngineState;
use crate::tables::cs::{CsId, CsKind};
use crate::tables::eqtb::{
    self, CatLoc, CodeTable, DelLoc, DimenLoc, FontLoc, GlueLoc, IntLoc, LcLoc, MathLoc, SfLoc,
    ShapeLoc, ShapeRef, ToksLoc, UcLoc, XetexCode,
};
use crate::tables::eqtb::{EqStore, InterCharPair};

fn new_interaction(st: &mut EngineState, mode: InteractionMode) {
    crate::print::print_ln(st);
    st.errors.interaction = mode;
    st.shipout.selector = if st.errors.interaction == crate::print::errors::InteractionMode::Batch {
        Selector::NoPrint
    } else {
        Selector::TermOnly
    };
    if st.errors.log_opened {
        st.shipout.selector = st.shipout.selector.with_log();
    }
}

fn alter_aux(st: &mut EngineState, code: AuxCode) -> Result<(), Abort> {
    if st.input.cur_list.mode.kind() != Some(code.kind()) {
        return crate::print::errors::report_illegal_case(st);
    }
    crate::scanning::scan::scan_optional_equals(st)?;
    if code == AuxCode::PrevDepth {
        let d = crate::scanning::scan::scan_dimen(st, false)?;
        return st.input.cur_list.set_prev_depth(d);
    }
    let n = crate::scanning::scan::scan_int(st)?;
    if n <= 0 || n > 32767 {
        return crate::print::errors::error(
            st,
            ("Bad space factor", " (", n, ")"),
            &["I allow only values in the range 1..32767 here."],
        );
    }
    st.input.cur_list.set_space_factor(n)
}

fn alter_prev_graf(st: &mut EngineState) -> Result<(), Abort> {
    let np = st.input.nest.len();
    st.input.nest[np] = st.input.cur_list;
    let mut p = np;
    while !st.input.nest[p].mode.is_vertical() {
        p -= 1;
    }
    crate::scanning::scan::scan_optional_equals(st)?;
    let n = crate::scanning::scan::scan_int(st)?;
    if n < 0 {
        return crate::print::errors::error(
            st,
            ("Bad ", crate::print::Esc("prevgraf"), " (", n, ")"),
            &["I allow only nonnegative values here."],
        );
    }
    st.input.nest[p].prev_graf = n;
    st.input.cur_list = st.input.nest[np];
    Ok(())
}

fn alter_page_so_far(st: &mut EngineState, code: PageDimenCode) -> Result<(), Abort> {
    crate::scanning::scan::scan_optional_equals(st)?;
    let d = crate::scanning::scan::scan_dimen(st, false)?;
    st.shipout.page_so_far[code.index()] = d;
    Ok(())
}

fn alter_integer(st: &mut EngineState, code: PageIntCode) -> Result<(), Abort> {
    crate::scanning::scan::scan_optional_equals(st)?;
    let n = crate::scanning::scan::scan_int(st)?;
    match code {
        PageIntCode::DeadCycles => st.shipout.dead_cycles = n,
        PageIntCode::InteractionMode => {
            let Some(mode) = InteractionMode::from_raw(n) else {
                return crate::print::errors::error(
                    st,
                    ("Bad interaction mode", " (", n, ")"),
                    &[
                        "Modes are 0=batch, 1=nonstop, 2=scroll, and",
                        "3=errorstop. Proceed, and I'll ignore this case.",
                    ],
                );
            };
            new_interaction(st, mode);
        }
        PageIntCode::InsertPenalties => st.shipout.insert_penalties = n,
    }
    Ok(())
}

fn alter_box_dimen(st: &mut EngineState, code: BoxDimenCode) -> Result<(), Abort> {
    let n = crate::scanning::scan::scan_register_num(st)?;
    let b = fetch_box(st, n);
    crate::scanning::scan::scan_optional_equals(st)?;
    let d = crate::scanning::scan::scan_dimen(st, false)?;
    if let Some(b) = b {
        let v = d;
        match code {
            BoxDimenCode::Width => st.mem[b].width = v,
            BoxDimenCode::Depth => st.mem[b].depth = v,
            BoxDimenCode::Height => st.mem[b].height = v,
        }
    }
    Ok(())
}

/// `spec`, or the shared `zero_glue` in its place when it is all zero.
fn trap_zero_glue(st: &mut EngineState, spec: GlueSpecId) -> GlueSpecId {
    let mem = &mut st.mem;
    if mem[spec].width == Scaled::ZERO
        && mem[spec].stretch == Scaled::ZERO
        && mem[spec].shrink == Scaled::ZERO
    {
        mem.add_glue_ref(ZERO_GLUE);
        mem.delete_glue_ref(spec);
        ZERO_GLUE
    } else {
        spec
    }
}

fn new_font(st: &mut EngineState, a: i16) -> Result<(), Abort> {
    if st.strings.job_name == StrNumber::NONE {
        crate::files::io::open_log_file(st)?;
    }
    let u = crate::scanning::scan::get_r_token(st)?;
    let t = match u.kind() {
        CsKind::Null => st.strings.make_tex_string("FONT")?,
        CsKind::Single(c) => StrNumber::single_char(i32::from(c)),
        CsKind::Active(c) => {
            let old_setting = st.shipout.selector;
            st.shipout.selector = Selector::NewString;
            crate::print::print(st, "FONT");
            crate::print::print(st, StrNumber::single_char(c.raw()));
            st.shipout.selector = old_setting;
            if st.strings.pool_ptr + 1 > st.strings.pool_size {
                return Err(st.strings.pool_overflow());
            }
            st.strings.make_string()?
        }
        _ => st.eqtb.text(u),
    };

    let global = a >= 4;
    eqtb::define(st, global, u, Command::SetFont(NULL_FONT))?;
    crate::scanning::scan::scan_optional_equals(st)?;
    let scanned = crate::files::io::scan_file_name(st)?;
    let file = scanned.file;
    st.scanner.name_in_progress = true;
    let s = if crate::scanning::scan::scan_keyword(st, "at")? {
        let size = crate::scanning::scan::scan_dimen(st, false)?;
        if size <= Scaled::ZERO || size >= Scaled::from_raw(0x800_0000) {
            crate::print::errors::error(
                st,
                ("Improper `at' size (", size, "pt), replaced by 10pt"),
                &[
                    "I can only handle fonts at positive sizes that are",
                    "less than 2048pt, so I've changed what you said to 10pt.",
                ],
            )?;
            FontSize::At(Scaled::UNITY * 10)
        } else {
            FontSize::At(size)
        }
    } else if crate::scanning::scan::scan_keyword(st, "scaled")? {
        let mag = crate::scanning::scan::scan_int(st)?;
        if mag <= 0 || mag > 32768 {
            crate::print::errors::error(
                st,
                (
                    "Illegal magnification has been changed to 1000",
                    " (",
                    mag,
                    ")",
                ),
                &["The magnification ratio must be between 1 and 32768."],
            )?;
            FontSize::DESIGN
        } else {
            FontSize::Scaled(mag)
        }
    } else {
        FontSize::DESIGN
    };
    st.scanner.name_in_progress = false;

    let size_matches = |st: &mut EngineState, f: FontId| -> bool {
        let scaled = s.of_design(st.fonts.font_dsize[f]);
        st.fonts.font_size[f] == scaled.quotient && !scaled.overflowed
    };
    let f = 'search: {
        for f in st.fonts.font_ptr.loaded() {
            // A native font's area holds its font type, which no area string can match.
            let area_matches = |st: &EngineState| {
                if st.fonts.is_native_font(f) {
                    st.strings.length(file.area) == 0
                } else {
                    st.strings.str_eq_str(st.fonts.font_area[f], file.area)
                }
            };
            if st.strings.str_eq_str(st.fonts.font_name[f], file.name)
                && area_matches(st)
                && size_matches(st, f)
            {
                break 'search f;
            }
            for part in [file.area, file.name, file.ext] {
                st.strings.append_str(part)?;
            }
            let full_name = st.strings.make_string()?;
            let same_name = st.strings.str_eq_str(st.fonts.font_name[f], full_name);
            st.strings.flush_string();
            if same_name && st.fonts.is_native_font(f) && size_matches(st, f) {
                break 'search f;
            }
        }
        read_font_info(st, u, &scanned, s)?
    };

    eqtb::define(st, global, u, Command::SetFont(f))?;
    let (m, level) = (st.eqtb.meaning(u), st.eqtb.cs_level(u));
    st.eqtb.set_meaning(CsId::font_id(f), m, level);
    st.eqtb.set_text(CsId::font_id(f), t);
    Ok(())
}

fn do_register_command(st: &mut EngineState, a: i16) -> Result<(), Abort> {
    let op = st.scanner.cur;
    let q = op.cmd();
    let (l, p) = 'found: {
        if q != Cmd::Register {
            crate::scanning::scan::get_x_token(st)?;
            match st.scanner.cur {
                Command::AssignInt(loc) => break 'found (RegisterLoc::Int(loc), ValueLevel::Int),
                Command::AssignDimen(loc) => {
                    break 'found (RegisterLoc::Dimen(loc), ValueLevel::Dimen);
                }
                Command::AssignGlue(loc) => {
                    break 'found (RegisterLoc::Glue(loc), ValueLevel::Glue);
                }
                Command::AssignMuGlue(loc) => {
                    break 'found (RegisterLoc::Glue(loc), ValueLevel::Mu);
                }
                _ => {}
            }
            if st.scanner.cur.cmd() != Cmd::Register {
                return crate::print::errors::error(
                    st,
                    ("You can't use `", st.scanner.cur, "' after ", op),
                    &["I'm forgetting what you said and not changing anything."],
                );
            }
        }
        let Command::Register(p) = st.scanner.cur else {
            return Err(Abort::Confusion("register"));
        };
        let r = crate::scanning::scan::scan_register_num(st)?;
        let l = match p {
            ValueLevel::Int => RegisterLoc::Int(IntLoc::count(r)),
            ValueLevel::Dimen => RegisterLoc::Dimen(DimenLoc::dimen(r)),
            ValueLevel::Glue => RegisterLoc::Glue(GlueLoc::skip(r)),
            ValueLevel::Mu => RegisterLoc::Glue(GlueLoc::mu_skip(r)),
            _ => return Err(Abort::Confusion("register")),
        };
        (l, p)
    };

    let (w, s) = match (p < ValueLevel::Glue, l) {
        (true, RegisterLoc::Int(l)) => (st.eqtb.ints.get(l), ZERO_GLUE),
        (true, RegisterLoc::Dimen(l)) => (st.eqtb.dimens.get(l).raw(), ZERO_GLUE),
        (false, RegisterLoc::Glue(l)) => (0, st.eqtb.glue.get(l)),
        _ => return Err(Abort::Confusion("register")),
    };

    if q == Cmd::Register {
        crate::scanning::scan::scan_optional_equals(st)?;
    } else {
        crate::scanning::scan::scan_keyword(st, "by")?;
    }
    let mut overflowed = false;

    let value = if q < Cmd::Multiply {
        if p < ValueLevel::Glue {
            let v = if p == ValueLevel::Int {
                crate::scanning::scan::scan_int(st)?
            } else {
                crate::scanning::scan::scan_dimen(st, false)?.raw()
            };
            RegisterValue::Word(if q == Cmd::Advance {
                v.wrapping_add(w)
            } else {
                v
            })
        } else {
            let g = crate::scanning::scan::scan_glue(st, p)?;
            if q == Cmd::Advance {
                let sum = st.mem.new_spec(g)?;
                st.mem.delete_glue_ref(g);
                let old = s;
                let width = st.mem[sum].width + st.mem[old].width;
                st.mem[sum].width = width;
                if st.mem[sum].stretch == Scaled::ZERO {
                    st.mem[sum].stretch_order = GlueOrder::Normal.into();
                }
                if st.mem[sum].stretch_order == st.mem[old].stretch_order {
                    let v = st.mem[sum].stretch + st.mem[old].stretch;
                    st.mem[sum].stretch = v;
                } else if st.mem[sum].stretch_order < st.mem[old].stretch_order
                    && st.mem[old].stretch != Scaled::ZERO
                {
                    let (v, order) = (st.mem[old].stretch, st.mem[old].stretch_order);
                    st.mem[sum].stretch = v;
                    st.mem[sum].stretch_order = order;
                }
                if st.mem[sum].shrink == Scaled::ZERO {
                    st.mem[sum].shrink_order = GlueOrder::Normal.into();
                }
                if st.mem[sum].shrink_order == st.mem[old].shrink_order {
                    let v = st.mem[sum].shrink + st.mem[old].shrink;
                    st.mem[sum].shrink = v;
                } else if st.mem[sum].shrink_order < st.mem[old].shrink_order
                    && st.mem[old].shrink != Scaled::ZERO
                {
                    let (v, order) = (st.mem[old].shrink, st.mem[old].shrink_order);
                    st.mem[sum].shrink = v;
                    st.mem[sum].shrink_order = order;
                }
                RegisterValue::Glue(sum)
            } else {
                RegisterValue::Glue(g)
            }
        }
    } else {
        let n = crate::scanning::scan::scan_int(st)?;
        let mut scale = |x: i32, max_answer: i32| {
            if q == Cmd::Multiply {
                mult_and_add(x, n, 0, max_answer).track(&mut overflowed)
            } else {
                let divided = x_over_n(x, n);
                overflowed |= divided.overflowed;
                divided.quotient
            }
        };
        if p < ValueLevel::Glue {
            let max_answer = if p == ValueLevel::Int {
                TEX_INFINITY
            } else {
                MAX_HALFWORD
            };
            RegisterValue::Word(scale(w, max_answer))
        } else {
            let old = s;
            let r = st.mem.new_spec(old)?;
            let (width, stretch, shrink) =
                (st.mem[old].width, st.mem[old].stretch, st.mem[old].shrink);
            let v = scale(width.raw(), MAX_HALFWORD);
            st.mem[r].width = Scaled::from_raw(v);
            let v = scale(stretch.raw(), MAX_HALFWORD);
            st.mem[r].stretch = Scaled::from_raw(v);
            let v = scale(shrink.raw(), MAX_HALFWORD);
            st.mem[r].shrink = Scaled::from_raw(v);
            RegisterValue::Glue(r)
        }
    };

    if overflowed {
        if let RegisterValue::Glue(g) = value {
            st.mem.delete_glue_ref(g);
        }
        return crate::print::errors::error(
            st,
            "Arithmetic overflow",
            &[
                "I can't carry out that multiplication or division,",
                "since the result is out of range.",
            ],
        );
    }

    let global = a >= 4;
    match value {
        RegisterValue::Word(v) => match l {
            RegisterLoc::Int(l) => eqtb::define(st, global, l, v)?,
            RegisterLoc::Dimen(l) => eqtb::define(st, global, l, Scaled::from_raw(v))?,
            _ => return Err(Abort::Confusion("register")),
        },
        RegisterValue::Glue(g) => {
            let g = trap_zero_glue(st, g);
            match l {
                RegisterLoc::Glue(l) => eqtb::define(st, global, l, g)?,
                _ => return Err(Abort::Confusion("register")),
            }
        }
    }
    Ok(())
}

/// A register's new value in `\advance`, `\multiply` or `\divide`.
#[derive(Clone, Copy)]
enum RegisterValue {
    Word(i32),
    Glue(GlueSpecId),
}

pub(super) fn do_assignments(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        crate::scanning::scan::get_x_non_blank_non_relax(st)?;
        if !st.scanner.cur.cmd().is_prefixed() {
            return Ok(());
        }
        st.scanner.set_box_allowed = false;
        prefixed_command(st)?;
        st.scanner.set_box_allowed = true;
    }
}

/// Scans a pair of character classes, a `\XeTeXinterchartoks` location.
pub(crate) fn scan_inter_char_pair(st: &mut EngineState) -> Result<InterCharPair, Abort> {
    let first = crate::scanning::scan::scan_char_class_not_ignored(st)?;
    let second = crate::scanning::scan::scan_char_class_not_ignored(st)?;
    InterCharPair::new(first, second).or_confusion("char class")
}

/// Where a register assignment stores its value.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RegisterLoc {
    Int(IntLoc),
    Dimen(DimenLoc),
    Glue(GlueLoc),
    Toks(ToksLoc),
    InterChar(InterCharPair),
}

fn define_toks(
    st: &mut EngineState,
    global: bool,
    target: RegisterLoc,
    list: TokenList,
) -> Result<(), Abort> {
    match target {
        RegisterLoc::Toks(p) => eqtb::define(st, global, p, list),
        RegisterLoc::InterChar(p) => eqtb::define(st, global, p, list),
        _ => Err(Abort::Confusion("toks")),
    }
}

pub(super) fn prefixed_command(st: &mut EngineState) -> Result<(), Abort> {
    let mut a: i32 = 0;
    while let Command::Prefix(code) = st.scanner.cur {
        if (a / code.raw()) & 1 == 0 {
            a += code.raw();
        }
        crate::scanning::scan::get_x_non_blank_non_relax(st)?;
        if !st.scanner.cur.cmd().is_prefixed() {
            return crate::print::errors::back_error(
                st,
                ("You can't use a prefix with `", st.scanner.cur, '\''),
                &[r"I'll pretend you didn't say \long or \outer or \global or \protected."],
            );
        }
        if st.eqtb.int_par(IntPar::TracingCommands) > 2 {
            crate::print::show_cur_cmd_chr(st);
        }
    }

    let protected = a >= 8;
    if protected {
        a -= 8;
    }
    if st.scanner.cur.cmd() != Cmd::Def && (a % 4 != 0 || protected) {
        crate::print::errors::error(
            st,
            (
                "You can't use `",
                crate::print::Esc("long"),
                "' or `",
                crate::print::Esc("outer"),
                "' or `",
                crate::print::Esc("protected"),
                "' with `",
                st.scanner.cur,
                '\'',
            ),
            &[r"I'll pretend you didn't say \long or \outer or \protected here."],
        )?;
    }
    let global_defs = st.eqtb.int_par(IntPar::GlobalDefs);
    if global_defs < 0 && a >= 4 {
        a -= 4;
    } else if global_defs > 0 && a < 4 {
        a += 4;
    }

    match st.scanner.cur {
        Command::SetFont(f) => eqtb::define(st, a >= 4, FontLoc::CUR, f)?,
        Command::Def { global, expand } => {
            if global && a < 4 && st.eqtb.int_par(IntPar::GlobalDefs) >= 0 {
                a += 4;
            }
            let p = crate::scanning::scan::get_r_token(st)?;
            crate::scanning::scan::scan_toks(st, true, expand)?;
            if protected {
                st.mem
                    .tokens
                    .prepend(st.scanner.def_ref, Token::PROTECTED)?;
            }
            let m = Command::call(st.scanner.def_ref, a % 2 == 1, a % 4 >= 2);
            eqtb::define(st, a >= 4, p, m)?;
        }
        Command::Let { future } => {
            let p = crate::scanning::scan::get_r_token(st)?;
            if !future {
                loop {
                    crate::scanning::scan::get_token(st)?;
                    if st.scanner.cur.cmd() != Cmd::Spacer {
                        break;
                    }
                }
                if st.scanner.cur_tok == Token::other(b'=') {
                    crate::scanning::scan::get_token(st)?;
                    if st.scanner.cur.cmd() == Cmd::Spacer {
                        crate::scanning::scan::get_token(st)?;
                    }
                }
            } else {
                crate::scanning::scan::get_token(st)?;
                let q = st.scanner.cur_tok;
                crate::scanning::scan::get_token(st)?;
                crate::scanning::input::back_input(st)?;
                st.scanner.cur_tok = q;
                crate::scanning::input::back_input(st)?;
            }
            let m = st.scanner.cur;
            if let Some(body) = m.macro_body() {
                st.mem.tokens.add_ref(body);
            }
            eqtb::define(st, a >= 4, p, m)?;
        }
        Command::ShorthandDef(code) => {
            if code == ShorthandDefCode::CharSub {
                crate::print::errors::error(
                    st,
                    "vestigial MLTeX shorthand encountered??",
                    &["This should never happen in Tectonic, where MLTeX has been excised."],
                )?;
            } else {
                shorthand_def(st, a >= 4, code)?;
            }
        }
        Command::ReadToCs { line } => {
            let n = crate::scanning::scan::scan_int(st)?;
            if !crate::scanning::scan::scan_keyword(st, "to")? {
                crate::print::errors::error(
                    st,
                    "Missing `to' inserted",
                    &[
                        r"You should have said `\read<number> to \cs'.",
                        r"I'm going to look for the \cs now.",
                    ],
                )?;
            }
            let p = crate::scanning::scan::get_r_token(st)?;
            let list = crate::scanning::scan::read_toks(st, n, p, line)?;
            eqtb::define(st, a >= 4, p, Command::Call(list))?;
        }
        cmd @ (Command::ToksRegister | Command::AssignToks(_)) => assign_toks(st, a >= 4, cmd)?,
        Command::AssignInt(p) => {
            crate::scanning::scan::scan_optional_equals(st)?;
            let n = crate::scanning::scan::scan_int(st)?;
            eqtb::define(st, a >= 4, p, n)?;
        }
        Command::AssignDimen(p) => {
            crate::scanning::scan::scan_optional_equals(st)?;
            let d = crate::scanning::scan::scan_dimen(st, false)?;
            eqtb::define(st, a >= 4, p, d)?;
        }
        cmd @ (Command::AssignGlue(p) | Command::AssignMuGlue(p)) => {
            let level = if let Command::AssignMuGlue(_) = cmd {
                ValueLevel::Mu
            } else {
                ValueLevel::Glue
            };
            crate::scanning::scan::scan_optional_equals(st)?;
            let g = crate::scanning::scan::scan_glue(st, level)?;
            let g = trap_zero_glue(st, g);
            eqtb::define(st, a >= 4, p, g)?;
        }
        Command::XetexDefCode(base) => xetex_def_code(st, a >= 4, base)?,
        Command::DefCode(base) => def_code(st, a >= 4, base)?,
        Command::DefFamily(base) => {
            let fam = crate::scanning::scan::scan_math_fam_int(st)?;
            let p = base.family(u8::try_from(fam).or_confusion("family")?);
            crate::scanning::scan::scan_optional_equals(st)?;
            let f = crate::scanning::scan::scan_font_ident(st)?;
            eqtb::define(st, a >= 4, p, f)?;
        }
        Command::Register(_) | Command::Advance | Command::Multiply | Command::Divide => {
            do_register_command(st, a as i16)?
        }
        Command::SetBox => {
            let n = crate::scanning::scan::scan_register_num(st)?;
            let context = BoxContext::SetBox {
                register: n,
                global: a >= 4,
            };
            crate::scanning::scan::scan_optional_equals(st)?;
            if st.scanner.set_box_allowed {
                scan_box(st, context)?;
            } else {
                crate::print::errors::error(
                    st,
                    ("Improper ", crate::print::Esc("setbox")),
                    &[
                        r"Sorry, \setbox is not allowed after \halign in a display,",
                        r"or between \accent and an accented character.",
                    ],
                )?;
            }
        }
        Command::SetAux(code) => alter_aux(st, code)?,
        Command::SetPrevGraf => alter_prev_graf(st)?,
        Command::SetPageDimen(code) => alter_page_so_far(st, code)?,
        Command::SetPageInt(code) => alter_integer(st, code)?,
        Command::SetBoxDimen(code) => alter_box_dimen(st, code)?,
        Command::SetShape(loc) => set_shape(st, a >= 4, loc)?,
        Command::HyphData { patterns } => {
            if !patterns {
                new_hyph_exceptions(st)?;
            } else if st.config.in_initex_mode {
                new_patterns(st)?;
            } else {
                crate::print::errors::error(st, "Patterns can be loaded only by INITEX", &[])?;
                loop {
                    crate::scanning::scan::get_token(st)?;
                    if st.scanner.cur.cmd() == Cmd::RightBrace {
                        return Ok(());
                    }
                }
            }
        }
        Command::AssignFontDimen => {
            let dimen = crate::scanning::scan::find_font_dimen(st, true)?;
            crate::scanning::scan::scan_optional_equals(st)?;
            let d = crate::scanning::scan::scan_dimen(st, false)?;
            if let Some((f, n)) = dimen
                && !st.fonts.params[f].set(n, d)
            {
                return Err(Abort::Confusion("font dimen"));
            }
        }
        Command::AssignFontInt(code) => {
            let f = crate::scanning::scan::scan_font_ident(st)?;
            let side = match code {
                FontIntCode::HyphenChar | FontIntCode::SkewChar => {
                    crate::scanning::scan::scan_optional_equals(st)?;
                    let v = crate::scanning::scan::scan_int(st)?;
                    if code == FontIntCode::HyphenChar {
                        st.fonts.hyphen_char[f] = v;
                    } else {
                        st.fonts.skew_char[f] = v;
                    }
                    None
                }
                FontIntCode::LpCode => Some(Side::Left),
                FontIntCode::RpCode => Some(Side::Right),
            };
            if let Some(side) = side {
                let p = if st.fonts.is_native_font(f) {
                    crate::scanning::scan::scan_glyph_number(st, f)?
                } else {
                    crate::scanning::scan::scan_char_num(st)?
                };
                crate::scanning::scan::scan_optional_equals(st)?;
                let v = crate::scanning::scan::scan_int(st)?;
                st.fonts.cp_codes.set(f, side, p as u32, v);
            }
        }
        Command::DefFont => new_font(st, a as i16)?,
        Command::SetInteraction(mode) => new_interaction(st, mode),
        _ => return Err(Abort::Confusion("prefix")),
    }

    if st.scanner.after_token != Token::ZERO {
        st.scanner.cur_tok = st.scanner.after_token;
        crate::scanning::input::back_input(st)?;
        st.scanner.after_token = Token::ZERO;
    }
    Ok(())
}

fn shorthand_def(st: &mut EngineState, global: bool, n: ShorthandDefCode) -> Result<(), Abort> {
    let p = crate::scanning::scan::get_r_token(st)?;
    eqtb::define(st, global, p, Command::Relax(RelaxCode::Primitive))?;
    crate::scanning::scan::scan_optional_equals(st)?;
    let m = match n {
        ShorthandDefCode::Char => {
            let c = crate::scanning::scan::scan_usv_num(st)?;
            Command::CharGiven(Usv::new(c).or_confusion("chardef")?)
        }
        ShorthandDefCode::MathChar => {
            Command::MathGiven(crate::scanning::scan::scan_fifteen_bit_int(st)?)
        }
        ShorthandDefCode::XetexMathCharNum => {
            Command::XetexMathGiven(crate::scanning::scan::scan_xetex_math_char_int(st)?)
        }
        ShorthandDefCode::XetexMathChar => {
            Command::XetexMathGiven(crate::scanning::scan::scan_umath_code(st)?)
        }
        _ => {
            let r = crate::scanning::scan::scan_register_num(st)?;
            match n {
                ShorthandDefCode::Count => Command::AssignInt(IntLoc::count(r)),
                ShorthandDefCode::Dimen => Command::AssignDimen(DimenLoc::dimen(r)),
                ShorthandDefCode::Skip => Command::AssignGlue(GlueLoc::skip(r)),
                ShorthandDefCode::MuSkip => Command::AssignMuGlue(GlueLoc::mu_skip(r)),
                ShorthandDefCode::Toks => Command::AssignToks(ToksLoc::register(r)),
                _ => return Ok(()),
            }
        }
    };
    eqtb::define(st, global, p, m)?;
    Ok(())
}

fn assign_toks(st: &mut EngineState, global: bool, cmd: Command) -> Result<(), Abort> {
    let cs = st.scanner.cur_cs;
    let target = match cmd {
        Command::ToksRegister => RegisterLoc::Toks(ToksLoc::register(
            crate::scanning::scan::scan_register_num(st)?,
        )),
        Command::AssignToks(loc) if loc == TokPar::XetexInterCharToks.loc() => {
            RegisterLoc::InterChar(scan_inter_char_pair(st)?)
        }
        Command::AssignToks(loc) => RegisterLoc::Toks(loc),
        _ => return Err(Abort::Confusion("assign_toks")),
    };
    crate::scanning::scan::scan_optional_equals(st)?;
    crate::scanning::scan::get_x_non_blank_non_relax(st)?;

    let source = match st.scanner.cur {
        Command::ToksRegister => {
            let n = crate::scanning::scan::scan_register_num(st)?;
            Some(st.eqtb.toks(n))
        }
        Command::AssignToks(loc) if loc == TokPar::XetexInterCharToks.loc() => {
            let pair = scan_inter_char_pair(st)?;
            Some(st.eqtb.inter_char.get(pair))
        }
        Command::AssignToks(loc) => Some(st.eqtb.toks.get(loc)),
        _ => None,
    };
    if let Some(list) = source {
        if !list.is_null() {
            st.mem.tokens.add_ref(list);
        }
        return define_toks(st, global, target, list);
    }

    crate::scanning::input::back_input(st)?;
    st.scanner.cur_cs = cs;
    let def_ref = crate::scanning::scan::scan_toks(st, false, false)?;
    if st.mem.tokens.is_empty(def_ref) {
        define_toks(st, global, target, TokenList::NULL)?;
        st.mem.tokens.flush(def_ref);
        return Ok(());
    }
    if target == RegisterLoc::Toks(TokPar::OutputRoutine.loc()) {
        st.mem.tokens.append(def_ref, Token::right_brace(b'}'))?;
        st.mem.tokens.prepend(def_ref, Token::left_brace(b'{'))?;
    }
    define_toks(st, global, target, def_ref)
}

fn xetex_def_code(st: &mut EngineState, global: bool, code: XetexCode) -> Result<(), Abort> {
    let c = crate::scanning::scan::scan_usv_num(st)?;
    match code {
        XetexCode::CharClass => {
            let sf = st.eqtb.sf_code(c) % 65536;
            crate::scanning::scan::scan_optional_equals(st)?;
            let class = crate::scanning::scan::scan_char_class(st)?;
            let v = class.wrapping_mul(65536).wrapping_add(sf);
            eqtb::define(st, global, SfLoc::of(c), v)?;
        }
        XetexCode::MathCodeNum => {
            crate::scanning::scan::scan_optional_equals(st)?;
            let v = crate::scanning::scan::scan_xetex_math_char_int(st)?;
            eqtb::define(st, global, MathLoc::of(c), v)?;
        }
        XetexCode::MathCode => {
            crate::scanning::scan::scan_optional_equals(st)?;
            let v = crate::scanning::scan::scan_umath_code(st)?;
            eqtb::define(st, global, MathLoc::of(c), v)?;
        }
        XetexCode::DelCodeNum => {
            crate::scanning::scan::scan_optional_equals(st)?;
            let v = crate::scanning::scan::scan_int(st)?;
            eqtb::define(st, global, DelLoc::of(c), v)?;
        }
        XetexCode::DelCode => {
            crate::scanning::scan::scan_optional_equals(st)?;
            // Extended delimiter code flag.
            let mut n: i32 = 0x4000_0000;
            let fam = crate::scanning::scan::scan_math_fam_int(st)?;
            n = n.wrapping_add(fam.wrapping_mul(0x20_0000));
            let d = crate::scanning::scan::scan_usv_num(st)?;
            n = n.wrapping_add(d);
            eqtb::define(st, global, DelLoc::of(c), n)?;
        }
    }
    Ok(())
}

fn def_code(st: &mut EngineState, global: bool, table: CodeTable) -> Result<(), Abort> {
    let max = match table {
        CodeTable::Cat => Cmd::MAX_CHAR_CODE.into(),
        CodeTable::Math => 0x8000,
        CodeTable::Sf => 0x7FFF,
        CodeTable::Del => 0xFF_FFFF,
        CodeTable::Lc | CodeTable::Uc => BIGGEST_USV,
    };
    let c = crate::scanning::scan::scan_usv_num(st)?;
    crate::scanning::scan::scan_optional_equals(st)?;
    let mut v = crate::scanning::scan::scan_int(st)?;
    let del = table == CodeTable::Del;
    if (v < 0 && !del) || v > max {
        let bound = if del {
            "), should be at most "
        } else {
            "), should be in the range 0.."
        };
        crate::print::errors::error(
            st,
            ("Invalid code (", v, bound, max),
            &["I'm going to use 0 instead of that illegal code value."],
        )?;
        v = 0;
    }
    match table {
        CodeTable::Cat => eqtb::define(st, global, CatLoc::of(c), Cmd::from_catcode_bits(v)),
        CodeTable::Lc => eqtb::define(st, global, LcLoc::of(c), v),
        CodeTable::Uc => eqtb::define(st, global, UcLoc::of(c), v),
        CodeTable::Sf => {
            let v = (st.eqtb.sf_code(c) / 65536)
                .wrapping_mul(65536)
                .wrapping_add(v);
            eqtb::define(st, global, SfLoc::of(c), v)
        }
        CodeTable::Math => {
            let v = if v == 32768 {
                ACTIVE_MATH_CHAR
            } else {
                crate::scanning::scan::math_code_from_tex(v)
            };
            eqtb::define(st, global, MathLoc::of(c), v)
        }
        CodeTable::Del => eqtb::define(st, global, DelLoc::of(c), v),
    }
}

fn set_shape(st: &mut EngineState, global: bool, q: ShapeLoc) -> Result<(), Abort> {
    crate::scanning::scan::scan_optional_equals(st)?;
    let n = crate::scanning::scan::scan_int(st)?;
    let p = if n <= 0 {
        ShapeRef::NONE
    } else if q != ShapeLoc::ParShape {
        let words = n / 2 + 1;
        let entries = usize::try_from(2 * words - 1).or_confusion("penalties")?;
        let penalties = st.mem.alloc_array::<PenaltiesNode>(entries)?;
        st.mem[penalties].count = n;
        let slots = st.mem.entries(penalties).len();
        for (k, slot) in (1..).zip(0..slots) {
            let v = if k <= n {
                crate::scanning::scan::scan_int(st)?
            } else {
                0
            };
            st.mem.entries_mut(penalties)[slot].value = v;
        }
        ShapeRef::penalties(penalties)
    } else {
        let lines = usize::try_from(n).or_confusion("parshape")?;
        let shape = st.mem.alloc_array::<ParShapeNode>(lines)?;
        for line in 0..st.mem.entries(shape).len() {
            let indent = crate::scanning::scan::scan_dimen(st, false)?;
            st.mem.entries_mut(shape)[line].indent = indent;
            let width = crate::scanning::scan::scan_dimen(st, false)?;
            st.mem.entries_mut(shape)[line].width = width;
        }
        ShapeRef::par_shape(shape)
    };
    eqtb::define(st, global, q, p)?;
    Ok(())
}
