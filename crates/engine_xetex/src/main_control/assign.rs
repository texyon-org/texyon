//! Mode-independent processing: `prefixed_command` and its assignments.

use crate::cmd::Cmd;
use crate::codes::{
    AuxCode, BoxDimenCode, FontIntCode, PageDimenCode, PageIntCode, ShorthandDefCode, ValueLevel,
};
use crate::command::RegisterCode;
use crate::command::RelaxCode;
use crate::command::ToksRef;
use crate::command::Usv;
use crate::command::{Command, RegisterRef};
use crate::consts::NULL_FONT;
use crate::consts::{
    ACTIVE_MATH_CHAR, BIGGEST_USV, CHAR_CLASS_LIMIT, DESIGN_SIZE, MAX_HALFWORD, TEX_INFINITY,
};
use crate::cs::{CsId, CsKind};
use crate::eqtb::{
    self, CatLoc, CodeTable, DelLoc, DimenLoc, FontLoc, GlueLoc, IntLoc, LcLoc, MathLoc, SfLoc,
    ShapeLoc, ShapeRef, ToksLoc, UcLoc, XetexCode,
};
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::fonts::read_font_info;
use crate::glue_store::GlueSpecId;
use crate::hyphenation::{new_hyph_exceptions, new_patterns};
use crate::mem::ZERO_GLUE;
use crate::node::{
    NodePtr, ParShapeNode, PenaltiesNode, SaElement, SaLeaf, SaPointerNode, SaWordNode,
};
use crate::params::{IntPar, TokPar};
use crate::scaled_math::{mult_and_add, x_over_n, xn_over_d};
use crate::scan::fetch_box;
use crate::state::{EngineState, InteractionMode, MemState, Selector};
use crate::token::{Token, TokenList};
use crate::units::MemIdx;
use crate::units::RawPtr;
use crate::units::{FontId, GlueOrder, MemPtr, Scaled, StrNumber};

use crate::fonts::Side;

use super::boxes::scan_box;
use super::{BOX_FLAG, GLOBAL_BOX_FLAG};

fn new_interaction(st: &mut EngineState, mode: InteractionMode) {
    crate::output::print_ln(st);
    st.errors.interaction = mode;
    st.shipout.selector = if st.errors.interaction == crate::state::InteractionMode::Batch {
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
        return crate::errors::report_illegal_case(st);
    }
    crate::scan::scan_optional_equals(st)?;
    if code == AuxCode::PrevDepth {
        let d = crate::scan::scan_dimen(st, false)?;
        return st.input.cur_list.set_prev_depth(d);
    }
    let n = crate::scan::scan_int(st)?;
    if n <= 0 || n > 32767 {
        return crate::errors::error(
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
    crate::scan::scan_optional_equals(st)?;
    let n = crate::scan::scan_int(st)?;
    if n < 0 {
        return crate::errors::error(
            st,
            ("Bad ", crate::output::Esc("prevgraf"), " (", n, ")"),
            &["I allow only nonnegative values here."],
        );
    }
    st.input.nest[p].prev_graf = n;
    st.input.cur_list = st.input.nest[np];
    Ok(())
}

fn alter_page_so_far(st: &mut EngineState, code: PageDimenCode) -> Result<(), Abort> {
    crate::scan::scan_optional_equals(st)?;
    let d = crate::scan::scan_dimen(st, false)?;
    st.shipout.page_so_far[code.index()] = d;
    Ok(())
}

fn alter_integer(st: &mut EngineState, code: PageIntCode) -> Result<(), Abort> {
    crate::scan::scan_optional_equals(st)?;
    let n = crate::scan::scan_int(st)?;
    match code {
        PageIntCode::DeadCycles => st.shipout.dead_cycles = n,
        PageIntCode::InteractionMode => {
            let Some(mode) = InteractionMode::from_raw(n) else {
                return crate::errors::error(
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
    let n = crate::scan::scan_register_num(st)?;
    let b = fetch_box(st, n);
    crate::scan::scan_optional_equals(st)?;
    let d = crate::scan::scan_dimen(st, false)?;
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
        crate::io::open_log_file(st)?;
    }
    let u = crate::scan::get_r_token(st)?;
    let t = match u.kind() {
        CsKind::Null => st.strings.make_tex_string("FONT")?,
        CsKind::Single(c) => StrNumber::from_raw(i32::from(c)),
        CsKind::Active(c) => {
            let old_setting = st.shipout.selector;
            st.shipout.selector = Selector::NewString;
            crate::output::print(st, "FONT");
            crate::output::print(st, StrNumber::single_char(c.raw()));
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
    crate::scan::scan_optional_equals(st)?;
    let scanned = crate::io::scan_file_name(st)?;
    let file = scanned.file;
    st.scanner.name_in_progress = true;
    let s = if crate::scan::scan_keyword(st, "at")? {
        let size = crate::scan::scan_dimen(st, false)?;
        if size <= Scaled::ZERO || size >= Scaled::from_raw(0x800_0000) {
            crate::errors::error(
                st,
                ("Improper `at' size (", size, "pt), replaced by 10pt"),
                &[
                    "I can only handle fonts at positive sizes that are",
                    "less than 2048pt, so I've changed what you said to 10pt.",
                ],
            )?;
            Scaled::UNITY * 10
        } else {
            size
        }
    } else if crate::scan::scan_keyword(st, "scaled")? {
        let mag = crate::scan::scan_int(st)?;
        if mag <= 0 || mag > 32768 {
            crate::errors::error(
                st,
                (
                    "Illegal magnification has been changed to 1000",
                    " (",
                    mag,
                    ")",
                ),
                &["The magnification ratio must be between 1 and 32768."],
            )?;
            DESIGN_SIZE
        } else {
            Scaled::from_raw(-mag)
        }
    } else {
        DESIGN_SIZE
    };
    st.scanner.name_in_progress = false;

    let size_matches = |st: &mut EngineState, f: FontId| -> bool {
        if s > Scaled::ZERO {
            s == st.fonts.font_size[f]
        } else {
            let scaled = xn_over_d(st.fonts.font_dsize[f], -s.raw(), 1000);
            st.fonts.font_size[f] == scaled.quotient && !scaled.overflowed
        }
    };
    let f = 'search: {
        for f in st.fonts.font_ptr.loaded() {
            let area_matches = (st.strings.length(file.area) == 0 && st.fonts.is_native_font(f))
                || st.strings.str_eq_str(st.fonts.font_area[f], file.area);
            if st.strings.str_eq_str(st.fonts.font_name[f], file.name)
                && area_matches
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
    let q = st.scanner.cur.cmd();
    let (l, p) = 'found: {
        if q != Cmd::Register {
            crate::scan::get_x_token(st)?;
            match st.scanner.cur {
                Command::AssignInt(loc) => break 'found (RegisterLoc::Int(loc), ValueLevel::Int),
                Command::AssignDimen(loc) => {
                    break 'found (RegisterLoc::Dimen(loc), ValueLevel::Dimen)
                }
                Command::AssignGlue(loc) => {
                    break 'found (RegisterLoc::Glue(loc), ValueLevel::Glue)
                }
                Command::AssignMuGlue(loc) => {
                    break 'found (RegisterLoc::Glue(loc), ValueLevel::Mu)
                }
                _ => {}
            }
            if st.scanner.cur.cmd() != Cmd::Register {
                return crate::errors::error(
                    st,
                    (
                        "You can't use `",
                        crate::output::CmdChr::of(st.scanner.cur),
                        "' after ",
                        crate::output::CmdChr(q, 0),
                    ),
                    &["I'm forgetting what you said and not changing anything."],
                );
            }
        }
        let Command::Register(r) = st.scanner.cur else {
            return Err(Abort::Confusion("register"));
        };
        let p = match r.get()? {
            RegisterRef::Scanned(level) => level,
            RegisterRef::Sparse(sa) => {
                let level = st.mem[sa].sa_type().get().or_confusion("sa_type")?;
                let leaf = sa.leaf(&st.mem).or_confusion("sa_type")?;
                break 'found (RegisterLoc::Sparse(leaf), level);
            }
        };
        let n = crate::scan::scan_register_num(st)?;
        let Ok(r) = u8::try_from(n) else {
            let sa = st.mem.create_sa_element(&mut st.scanner, p, n)?;
            break 'found (RegisterLoc::Sparse(sa), p);
        };
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
        (true, RegisterLoc::Sparse(SaLeaf::Word(w))) => (st.mem[w].sa_int, ZERO_GLUE),
        (true, RegisterLoc::Int(l)) => (st.eqtb.ints.get(l), ZERO_GLUE),
        (true, RegisterLoc::Dimen(l)) => (st.eqtb.dimens.get(l).raw(), ZERO_GLUE),
        (false, RegisterLoc::Sparse(SaLeaf::Pointer(p))) => {
            (0, GlueSpecId::from_raw(st.mem[p].sa_ptr.raw()))
        }
        (false, RegisterLoc::Glue(l)) => (0, st.eqtb.glue.get(l)),
        _ => return Err(Abort::Confusion("register")),
    };

    if q == Cmd::Register {
        crate::scan::scan_optional_equals(st)?;
    } else {
        crate::scan::scan_keyword(st, "by")?;
    }
    let mut overflowed = false;

    let value = if q < Cmd::Multiply {
        if p < ValueLevel::Glue {
            let v = if p == ValueLevel::Int {
                crate::scan::scan_int(st)?
            } else {
                crate::scan::scan_dimen(st, false)?.raw()
            };
            RegisterValue::Word(if q == Cmd::Advance {
                v.wrapping_add(w)
            } else {
                v
            })
        } else {
            let g = crate::scan::scan_glue(st, p)?;
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
        let n = crate::scan::scan_int(st)?;
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
        return crate::errors::error(
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
            RegisterLoc::Sparse(SaLeaf::Word(w)) => sa_word_define(st, global, w, v)?,
            RegisterLoc::Int(l) => eqtb::define(st, global, l, v)?,
            RegisterLoc::Dimen(l) => eqtb::define(st, global, l, Scaled::from_raw(v))?,
            _ => return Err(Abort::Confusion("register")),
        },
        RegisterValue::Glue(g) => {
            let g = MemIdx::from_raw(trap_zero_glue(st, g).raw());
            match l {
                RegisterLoc::Sparse(SaLeaf::Pointer(p)) => sa_define(st, global, p, g)?,
                RegisterLoc::Glue(l) => eqtb::define(st, global, l, GlueSpecId::from_raw(g.raw()))?,
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
        crate::scan::get_x_non_blank_non_relax(st)?;
        if !st.scanner.cur.cmd().is_prefixed() {
            return Ok(());
        }
        st.scanner.set_box_allowed = false;
        prefixed_command(st)?;
        st.scanner.set_box_allowed = true;
    }
}

pub(super) fn sa_define(
    st: &mut EngineState,
    global: bool,
    p: NodePtr<SaPointerNode>,
    e: MemPtr,
) -> Result<(), Abort> {
    let result = if global {
        st.mem.gsa_def(p, e)
    } else {
        st.mem.sa_def(&mut st.scanner, &mut st.save_stack, p, e)?
    };
    crate::save::finish_sa_mutation(st, p.element(), result)?;
    Ok(())
}

fn sa_word_define(
    st: &mut EngineState,
    global: bool,
    p: NodePtr<SaWordNode>,
    w: i32,
) -> Result<(), Abort> {
    let result = if global {
        st.mem.gsa_w_def(p, w)
    } else {
        st.mem.sa_w_def(&mut st.scanner, &mut st.save_stack, p, w)?
    };
    crate::save::finish_sa_mutation(st, p.element(), result)?;
    Ok(())
}

impl MemState {
    fn add_sa_ref(&mut self, e: NodePtr<SaElement>) -> Result<(), Abort> {
        match e.leaf(self) {
            Some(SaLeaf::Word(w)) => self[w].sa_ref += 1,
            Some(SaLeaf::Pointer(p)) => self[p].sa_ref += 1,
            Some(SaLeaf::MarkClass(_)) | None => return Err(Abort::Confusion("sa_ref")),
        }
        Ok(())
    }
}

/// The `sa_ptr` of a sparse-array record, or null without one.
/// Scans a pair of character classes into its `\XeTeXinterchartoks` index.
fn scan_inter_char_pair(st: &mut EngineState) -> Result<i32, Abort> {
    let first = crate::scan::scan_char_class_not_ignored(st)?;
    let second = crate::scan::scan_char_class_not_ignored(st)?;
    Ok(first * CHAR_CLASS_LIMIT + second)
}

/// Defines token list register `p`, a sparse-array entry if `e`, as `list`.
/// Where a register assignment stores its value.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RegisterLoc {
    Int(IntLoc),
    Dimen(DimenLoc),
    Glue(GlueLoc),
    Toks(ToksLoc),
    Sparse(SaLeaf),
}

fn define_toks(
    st: &mut EngineState,
    global: bool,
    target: RegisterLoc,
    list: TokenList,
) -> Result<(), Abort> {
    match target {
        RegisterLoc::Sparse(SaLeaf::Pointer(p)) => {
            sa_define(st, global, p, MemIdx::from_raw(list.raw()))
        }
        RegisterLoc::Toks(p) => eqtb::define(st, global, p, list),
        _ => Err(Abort::Confusion("toks")),
    }
}

pub(super) fn prefixed_command(st: &mut EngineState) -> Result<(), Abort> {
    let mut a: i32 = 0;
    while let Command::Prefix(code) = st.scanner.cur {
        if (a / code.raw()) & 1 == 0 {
            a += code.raw();
        }
        crate::scan::get_x_non_blank_non_relax(st)?;
        if !st.scanner.cur.cmd().is_prefixed() {
            return crate::errors::back_error(
                st,
                (
                    "You can't use a prefix with `",
                    crate::output::CmdChr::of(st.scanner.cur),
                    '\'',
                ),
                &[r"I'll pretend you didn't say \long or \outer or \global or \protected."],
            );
        }
        if st.eqtb.int_par(IntPar::TracingCommands) > 2 {
            crate::output::show_cur_cmd_chr(st);
        }
    }

    let protected = a >= 8;
    if protected {
        a -= 8;
    }
    if st.scanner.cur.cmd() != Cmd::Def && (a % 4 != 0 || protected) {
        crate::errors::error(
            st,
            (
                "You can't use `",
                crate::output::Esc("long"),
                "' or `",
                crate::output::Esc("outer"),
                "' or `",
                crate::output::Esc("protected"),
                "' with `",
                crate::output::CmdChr::of(st.scanner.cur),
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
            let p = crate::scan::get_r_token(st)?;
            crate::scan::scan_toks(st, true, expand)?;
            if protected {
                st.mem
                    .tokens
                    .prepend(st.scanner.def_ref, Token::PROTECTED)?;
            }
            let m = Command::call(st.scanner.def_ref, a % 2 == 1, a % 4 >= 2);
            eqtb::define(st, a >= 4, p, m)?;
        }
        Command::Let { future } => {
            let p = crate::scan::get_r_token(st)?;
            if !future {
                loop {
                    crate::scan::get_token(st)?;
                    if st.scanner.cur.cmd() != Cmd::Spacer {
                        break;
                    }
                }
                if st.scanner.cur_tok == Token::other(b'=') {
                    crate::scan::get_token(st)?;
                    if st.scanner.cur.cmd() == Cmd::Spacer {
                        crate::scan::get_token(st)?;
                    }
                }
            } else {
                crate::scan::get_token(st)?;
                let q = st.scanner.cur_tok;
                crate::scan::get_token(st)?;
                crate::input::back_input(st)?;
                st.scanner.cur_tok = q;
                crate::input::back_input(st)?;
            }
            let m = st.scanner.cur;
            let sparse = match m {
                Command::ToksRegister(r) => r.get().map(NodePtr::at),
                Command::Register(r) => match r.get()? {
                    RegisterRef::Sparse(sa) => Some(sa),
                    RegisterRef::Scanned(_) => None,
                },
                _ => None,
            };
            if let Some(body) = m.macro_body() {
                st.mem.tokens.add_ref(body);
            } else if let Some(sa) = sparse {
                st.mem.add_sa_ref(sa)?;
            }
            eqtb::define(st, a >= 4, p, m)?;
        }
        Command::ShorthandDef(code) => {
            if code == ShorthandDefCode::CharSub {
                crate::errors::error(
                    st,
                    "vestigial MLTeX shorthand encountered??",
                    &["This should never happen in Tectonic, where MLTeX has been excised."],
                )?;
            } else {
                shorthand_def(st, a >= 4, code)?;
            }
        }
        Command::ReadToCs { line } => {
            let n = crate::scan::scan_int(st)?;
            if !crate::scan::scan_keyword(st, "to")? {
                crate::errors::error(
                    st,
                    "Missing `to' inserted",
                    &[
                        r"You should have said `\read<number> to \cs'.",
                        r"I'm going to look for the \cs now.",
                    ],
                )?;
            }
            let p = crate::scan::get_r_token(st)?;
            let list = crate::scan::read_toks(st, n, p, line)?;
            eqtb::define(st, a >= 4, p, Command::Call(list))?;
        }
        cmd @ (Command::ToksRegister(_) | Command::AssignToks(_)) => assign_toks(st, a >= 4, cmd)?,
        Command::AssignInt(p) => {
            crate::scan::scan_optional_equals(st)?;
            let n = crate::scan::scan_int(st)?;
            eqtb::define(st, a >= 4, p, n)?;
        }
        Command::AssignDimen(p) => {
            crate::scan::scan_optional_equals(st)?;
            let d = crate::scan::scan_dimen(st, false)?;
            eqtb::define(st, a >= 4, p, d)?;
        }
        cmd @ (Command::AssignGlue(p) | Command::AssignMuGlue(p)) => {
            let level = if let Command::AssignMuGlue(_) = cmd {
                ValueLevel::Mu
            } else {
                ValueLevel::Glue
            };
            crate::scan::scan_optional_equals(st)?;
            let g = crate::scan::scan_glue(st, level)?;
            let g = trap_zero_glue(st, g);
            eqtb::define(st, a >= 4, p, g)?;
        }
        Command::XetexDefCode(base) => xetex_def_code(st, a >= 4, base)?,
        Command::DefCode(base) => def_code(st, a >= 4, base)?,
        Command::DefFamily(base) => {
            let fam = crate::scan::scan_math_fam_int(st)?;
            let p = base.family(u8::try_from(fam).or_confusion("family")?);
            crate::scan::scan_optional_equals(st)?;
            let f = crate::scan::scan_font_ident(st)?;
            eqtb::define(st, a >= 4, p, f)?;
        }
        Command::Register(_) | Command::Advance | Command::Multiply | Command::Divide => {
            do_register_command(st, a as i16)?
        }
        Command::SetBox => {
            let n = crate::scan::scan_register_num(st)?;
            let n = if a >= 4 {
                GLOBAL_BOX_FLAG + n
            } else {
                BOX_FLAG + n
            };
            crate::scan::scan_optional_equals(st)?;
            if st.scanner.set_box_allowed {
                scan_box(st, n)?;
            } else {
                crate::errors::error(
                    st,
                    ("Improper ", crate::output::Esc("setbox")),
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
                crate::errors::error(st, "Patterns can be loaded only by INITEX", &[])?;
                loop {
                    crate::scan::get_token(st)?;
                    if st.scanner.cur.cmd() == Cmd::RightBrace {
                        return Ok(());
                    }
                }
            }
        }
        Command::AssignFontDimen => {
            let dimen = crate::scan::find_font_dimen(st, true)?;
            crate::scan::scan_optional_equals(st)?;
            let d = crate::scan::scan_dimen(st, false)?;
            if let Some((f, n)) = dimen {
                if !st.fonts.params[f].set(n, d) {
                    return Err(Abort::Confusion("font dimen"));
                }
            }
        }
        Command::AssignFontInt(code) => {
            let f = crate::scan::scan_font_ident(st)?;
            let side = match code {
                FontIntCode::HyphenChar | FontIntCode::SkewChar => {
                    crate::scan::scan_optional_equals(st)?;
                    let v = crate::scan::scan_int(st)?;
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
                    crate::scan::scan_glyph_number(st, f)?
                } else {
                    crate::scan::scan_char_num(st)?
                };
                crate::scan::scan_optional_equals(st)?;
                let v = crate::scan::scan_int(st)?;
                st.fonts.cp_codes.set(f, side, p as u32, v);
            }
        }
        Command::DefFont => new_font(st, a as i16)?,
        Command::SetInteraction(mode) => new_interaction(st, mode),
        _ => return Err(Abort::Confusion("prefix")),
    }

    if st.scanner.after_token != Token::ZERO {
        st.scanner.cur_tok = st.scanner.after_token;
        crate::input::back_input(st)?;
        st.scanner.after_token = Token::ZERO;
    }
    Ok(())
}

fn shorthand_def(st: &mut EngineState, global: bool, n: ShorthandDefCode) -> Result<(), Abort> {
    let p = crate::scan::get_r_token(st)?;
    eqtb::define(st, global, p, Command::Relax(RelaxCode::Primitive))?;
    crate::scan::scan_optional_equals(st)?;
    let m = match n {
        ShorthandDefCode::Char => {
            let c = crate::scan::scan_usv_num(st)?;
            Command::CharGiven(Usv::new(c).or_confusion("chardef")?)
        }
        ShorthandDefCode::MathChar => Command::MathGiven(crate::scan::scan_fifteen_bit_int(st)?),
        ShorthandDefCode::XetexMathCharNum => {
            Command::XetexMathGiven(crate::scan::scan_xetex_math_char_int(st)?)
        }
        ShorthandDefCode::XetexMathChar => {
            Command::XetexMathGiven(crate::scan::scan_umath_code(st)?)
        }
        _ => {
            let reg = crate::scan::scan_register_num(st)?;
            if reg > 255 {
                let level = match n {
                    ShorthandDefCode::Count => ValueLevel::Int,
                    ShorthandDefCode::Dimen => ValueLevel::Dimen,
                    ShorthandDefCode::Skip => ValueLevel::Glue,
                    ShorthandDefCode::MuSkip => ValueLevel::Mu,
                    _ => ValueLevel::Tok,
                };
                let sa = st.mem.create_sa_element(&mut st.scanner, level, reg)?;
                st.mem.add_sa_ref(sa.element())?;
                if level == ValueLevel::Tok {
                    Command::ToksRegister(ToksRef::sparse(sa.element()))
                } else {
                    Command::Register(RegisterCode::new(RegisterRef::Sparse(sa.element())))
                }
            } else {
                let r = u8::try_from(reg).or_confusion("register")?;
                match n {
                    ShorthandDefCode::Count => Command::AssignInt(IntLoc::count(r)),
                    ShorthandDefCode::Dimen => Command::AssignDimen(DimenLoc::dimen(r)),
                    ShorthandDefCode::Skip => Command::AssignGlue(GlueLoc::skip(r)),
                    ShorthandDefCode::MuSkip => Command::AssignMuGlue(GlueLoc::mu_skip(r)),
                    ShorthandDefCode::Toks => Command::AssignToks(ToksLoc::register(r)),
                    _ => return Ok(()),
                }
            }
        }
    };
    eqtb::define(st, global, p, m)?;
    Ok(())
}

fn assign_toks(st: &mut EngineState, global: bool, cmd: Command) -> Result<(), Abort> {
    let cs = st.scanner.cur_cs;
    let target = match cmd {
        Command::ToksRegister(r) if r.get().is_none() => {
            let n = crate::scan::scan_register_num(st)?;
            if let Ok(r) = u8::try_from(n) {
                RegisterLoc::Toks(ToksLoc::register(r))
            } else {
                let sa = st
                    .mem
                    .create_sa_element(&mut st.scanner, ValueLevel::Tok, n)?;
                RegisterLoc::Sparse(sa)
            }
        }
        Command::ToksRegister(r) => match r.get() {
            Some(sa) => RegisterLoc::Sparse(SaLeaf::Pointer(sa)),
            None => return Err(Abort::Confusion("assign_toks")),
        },
        Command::AssignToks(loc) if loc == TokPar::XetexInterCharToks.loc() => {
            let n = scan_inter_char_pair(st)?;
            let sa = st
                .mem
                .create_sa_element(&mut st.scanner, ValueLevel::InterChar, n)?;
            RegisterLoc::Sparse(sa)
        }
        Command::AssignToks(loc) => RegisterLoc::Toks(loc),
        _ => return Err(Abort::Confusion("assign_toks")),
    };
    crate::scan::scan_optional_equals(st)?;
    crate::scan::get_x_non_blank_non_relax(st)?;

    let source = match st.scanner.cur {
        Command::ToksRegister(r) if r.get().is_none() => {
            let n = crate::scan::scan_register_num(st)?;
            Some(if let Ok(r) = u8::try_from(n) {
                st.eqtb.toks(r)
            } else {
                let sa = st.mem.find_sa_element(&st.scanner, ValueLevel::Tok, n);
                crate::scan::sa_token_list(st, sa)
            })
        }
        Command::ToksRegister(r) => r.get().map(|sa| st.mem[sa].toks()),
        Command::AssignToks(loc) if loc == TokPar::XetexInterCharToks.loc() => {
            let n = scan_inter_char_pair(st)?;
            let sa = st
                .mem
                .find_sa_element(&st.scanner, ValueLevel::InterChar, n);
            Some(crate::scan::sa_token_list(st, sa))
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

    crate::input::back_input(st)?;
    st.scanner.cur_cs = cs;
    let def_ref = crate::scan::scan_toks(st, false, false)?;
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
    let c = crate::scan::scan_usv_num(st)?;
    match code {
        XetexCode::CharClass => {
            let sf = st.eqtb.sf_code(c) % 65536;
            crate::scan::scan_optional_equals(st)?;
            let class = crate::scan::scan_char_class(st)?;
            let v = class.wrapping_mul(65536).wrapping_add(sf);
            eqtb::define(st, global, SfLoc::of(c), v)?;
        }
        XetexCode::MathCodeNum => {
            crate::scan::scan_optional_equals(st)?;
            let v = crate::scan::scan_xetex_math_char_int(st)?;
            eqtb::define(st, global, MathLoc::of(c), v)?;
        }
        XetexCode::MathCode => {
            crate::scan::scan_optional_equals(st)?;
            let v = crate::scan::scan_umath_code(st)?;
            eqtb::define(st, global, MathLoc::of(c), v)?;
        }
        XetexCode::DelCodeNum => {
            crate::scan::scan_optional_equals(st)?;
            let v = crate::scan::scan_int(st)?;
            eqtb::define(st, global, DelLoc::of(c), v)?;
        }
        XetexCode::DelCode => {
            crate::scan::scan_optional_equals(st)?;
            // Extended delimiter code flag.
            let mut n: i32 = 0x4000_0000;
            let fam = crate::scan::scan_math_fam_int(st)?;
            n = n.wrapping_add(fam.wrapping_mul(0x20_0000));
            let d = crate::scan::scan_usv_num(st)?;
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
    let c = crate::scan::scan_usv_num(st)?;
    crate::scan::scan_optional_equals(st)?;
    let mut v = crate::scan::scan_int(st)?;
    let del = table == CodeTable::Del;
    if (v < 0 && !del) || v > max {
        let bound = if del {
            "), should be at most "
        } else {
            "), should be in the range 0.."
        };
        crate::errors::error(
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
                crate::scan::math_code_from_tex(v)
            };
            eqtb::define(st, global, MathLoc::of(c), v)
        }
        CodeTable::Del => eqtb::define(st, global, DelLoc::of(c), v),
    }
}

fn set_shape(st: &mut EngineState, global: bool, q: ShapeLoc) -> Result<(), Abort> {
    crate::scan::scan_optional_equals(st)?;
    let n = crate::scan::scan_int(st)?;
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
                crate::scan::scan_int(st)?
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
            let indent = crate::scan::scan_dimen(st, false)?;
            st.mem.entries_mut(shape)[line].indent = indent;
            let width = crate::scan::scan_dimen(st, false)?;
            st.mem.entries_mut(shape)[line].width = width;
        }
        ShapeRef::par_shape(shape)
    };
    eqtb::define(st, global, q, p)?;
    Ok(())
}
