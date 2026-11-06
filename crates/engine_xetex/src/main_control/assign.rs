//! Mode-independent processing: `prefixed_command` and its assignments.

use crate::cmd::Cmd;
use crate::codes::{BoxDimenCode, PageIntCode, ShorthandDefCode, ValueLevel};
use crate::consts::{
    ACTIVE_BASE, ACTIVE_MATH_CHAR, BIGGEST_USV, CAT_CODE_BASE, CHAR_CLASS_LIMIT, COUNT_BASE,
    CUR_FONT_LOC, DEL_CODE_BASE, DESIGN_SIZE, FONT_BASE, HASH_BASE, LEFT_SIDE, MATH_CODE_BASE,
    MAX_HALFWORD, MU_SKIP_BASE, NORMAL, NULL_CS, PAR_SHAPE_LOC, RIGHT_SIDE, SCALED_BASE,
    SF_CODE_BASE, SINGLE_BASE, SKIP_BASE, TEX_INFINITY, TOKS_BASE, TOO_BIG_USV, VMODE,
};
use crate::errors::Abort;
use crate::fonts::read_font_info;
use crate::hyphenation::{new_hyph_exceptions, new_patterns};
use crate::mem::ZERO_GLUE;
use crate::node::{GlueSpecNode, NodePtr, ParShapeNode, PenaltiesNode, SaNode};
use crate::params::{IntPar, TokPar};
use crate::scan::fetch_box;
use crate::state::{EngineState, MemState, Selector};
use crate::token::{Token, TokenList};
use crate::units::{EqtbPtr, FontId, MemPtr, Scaled, StrNumber};

use texyon_xetex_layout::set_cp_code;

use super::boxes::scan_box;
use super::{BOX_FLAG, GLOBAL_BOX_FLAG, LP_CODE_BASE, RP_CODE_BASE};

fn new_interaction(st: &mut EngineState) {
    crate::output::print_ln(st);
    st.errors.interaction = crate::state::InteractionMode::from_raw(st.scanner.cur_chr)
        .expect("interaction primitive chr is 0..=3");
    st.shipout.selector = if st.errors.interaction == crate::state::InteractionMode::Batch {
        Selector::NoPrint
    } else {
        Selector::TermOnly
    };
    if st.errors.log_opened {
        st.shipout.selector = st.shipout.selector.with_log();
    }
}

fn alter_aux(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.cur_chr != i32::from(st.input.cur_list.mode.abs()) {
        return crate::errors::report_illegal_case(st);
    }
    let c = st.scanner.cur_chr;
    crate::scan::scan_optional_equals(st)?;
    if c == i32::from(VMODE) {
        crate::scan::scan_dimen(st, false, false, false)?;
        st.input.cur_list.set_prev_depth(st.scanner.cur_dimen());
        return Ok(());
    }
    crate::scan::scan_int(st)?;
    if st.scanner.cur_val <= 0 || st.scanner.cur_val > 32767 {
        crate::output::error_here_with_diagnostic(st, "Bad space factor");
        crate::output::append_to_diagnostic(st, &format!(" ({})", st.scanner.cur_val));
        crate::output::capture_to_diagnostic(st, None);
        st.errors
            .help(&["I allow only values in the range 1..32767 here."]);
        return crate::errors::int_error(st, st.scanner.cur_val);
    }
    st.input.cur_list.set_space_factor(st.scanner.cur_val);
    Ok(())
}

fn alter_prev_graf(st: &mut EngineState) -> Result<(), Abort> {
    let np = st.input.nest_ptr as usize;
    st.input.nest[np] = st.input.cur_list;
    let mut p = np;
    while st.input.nest[p].mode.abs() != VMODE {
        p -= 1;
    }
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_int(st)?;
    if st.scanner.cur_val < 0 {
        crate::output::error_here_with_diagnostic(st, "Bad ");
        crate::output::append_to_diagnostic(st, &format!(" ({})", st.scanner.cur_val));
        crate::output::print_esc(st, "prevgraf");
        crate::output::capture_to_diagnostic(st, None);
        st.errors.help(&["I allow only nonnegative values here."]);
        return crate::errors::int_error(st, st.scanner.cur_val);
    }
    st.input.nest[p].prev_graf = st.scanner.cur_val;
    st.input.cur_list = st.input.nest[np];
    Ok(())
}

fn alter_page_so_far(st: &mut EngineState) -> Result<(), Abort> {
    let c = st.scanner.cur_chr as usize;
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_dimen(st, false, false, false)?;
    st.shipout.page_so_far[c] = st.scanner.cur_dimen();
    Ok(())
}

fn alter_integer(st: &mut EngineState) -> Result<(), Abort> {
    let code = PageIntCode::from_raw(st.scanner.cur_chr);
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_int(st)?;
    match code {
        Some(PageIntCode::DeadCycles) => st.shipout.dead_cycles = st.scanner.cur_val,
        Some(PageIntCode::InteractionMode) => {
            if crate::state::InteractionMode::from_raw(st.scanner.cur_val).is_none() {
                crate::output::error_here_with_diagnostic(st, "Bad interaction mode");
                crate::output::append_to_diagnostic(st, &format!(" ({})", st.scanner.cur_val));
                crate::output::capture_to_diagnostic(st, None);
                st.errors.help(&[
                    "Modes are 0=batch, 1=nonstop, 2=scroll, and",
                    "3=errorstop. Proceed, and I'll ignore this case.",
                ]);
                return crate::errors::int_error(st, st.scanner.cur_val);
            }
            st.scanner.cur_chr = st.scanner.cur_val;
            new_interaction(st);
        }
        _ => st.shipout.insert_penalties = st.scanner.cur_val,
    }
    Ok(())
}

fn alter_box_dimen(st: &mut EngineState) -> Result<(), Abort> {
    let c = st.scanner.cur_chr;
    crate::scan::scan_register_num(st)?;
    let b = fetch_box(st, st.scanner.cur_val);
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_dimen(st, false, false, false)?;
    if let Some(b) = b {
        let v = st.scanner.cur_dimen();
        match BoxDimenCode::from_raw(c) {
            Some(BoxDimenCode::Width) => b.set_width(&mut st.mem, v),
            Some(BoxDimenCode::Depth) => b.set_depth(&mut st.mem, v),
            _ => b.set_height(&mut st.mem, v),
        }
    }
    Ok(())
}

/// Replaces an all-zero scanned glue spec in `cur_val` by the shared `zero_glue`.
fn trap_zero_glue(st: &mut EngineState) {
    let spec = st.scanner.cur_glue();
    let mem = &mut st.mem;
    if spec.width(mem) == Scaled::ZERO
        && spec.stretch(mem) == Scaled::ZERO
        && spec.shrink(mem) == Scaled::ZERO
    {
        mem.add_glue_ref(ZERO_GLUE);
        mem.delete_glue_ref(spec);
        st.scanner.cur_val = ZERO_GLUE.idx().raw();
    }
}

fn flush_last_string(st: &mut EngineState) {
    st.strings.str_ptr -= 1;
    st.strings.discard_cur_string();
}

fn new_font(st: &mut EngineState, a: i16) -> Result<(), Abort> {
    if st.strings.job_name == StrNumber::NONE {
        crate::io::open_log_file(st)?;
    }
    crate::scan::get_r_token(st)?;
    let u = st.scanner.cur_cs;
    let t = if u >= HASH_BASE {
        st.eqtb.text(u)
    } else if u >= SINGLE_BASE {
        if u == NULL_CS {
            st.strings.make_tex_string("FONT")?
        } else {
            StrNumber::from_raw(u - SINGLE_BASE)
        }
    } else {
        let old_setting = st.shipout.selector;
        st.shipout.selector = Selector::NewString;
        crate::output::print(st, "FONT");
        crate::output::print(st, u - ACTIVE_BASE);
        st.shipout.selector = old_setting;
        if st.strings.pool_ptr + 1 > st.strings.pool_size {
            return Err(Abort::PoolOverflow);
        }
        st.strings.make_string()?
    };

    let global = a >= 4;
    define(st, global, u, Cmd::SetFont, FONT_BASE)?;
    crate::scan::scan_optional_equals(st)?;
    crate::io::scan_file_name(st)?;
    st.scanner.name_in_progress = true;
    let s = if crate::scan::scan_keyword(st, "at")? {
        crate::scan::scan_dimen(st, false, false, false)?;
        let size = st.scanner.cur_dimen();
        if size <= Scaled::ZERO || size >= Scaled::from_raw(0x800_0000) {
            crate::output::error_here_with_diagnostic(st, "Improper `at' size (");
            crate::output::print_scaled(st, size);
            crate::output::print(st, "pt), replaced by 10pt");
            crate::output::capture_to_diagnostic(st, None);
            st.errors.help(&[
                "I can only handle fonts at positive sizes that are",
                "less than 2048pt, so I've changed what you said to 10pt.",
            ]);
            crate::errors::error(st)?;
            Scaled::UNITY * 10
        } else {
            size
        }
    } else if crate::scan::scan_keyword(st, "scaled")? {
        crate::scan::scan_int(st)?;
        let mag = st.scanner.cur_val;
        if mag <= 0 || mag > 32768 {
            crate::output::error_here_with_diagnostic(
                st,
                "Illegal magnification has been changed to 1000",
            );
            crate::output::append_to_diagnostic(st, &format!(" ({})", mag));
            crate::output::capture_to_diagnostic(st, None);
            st.errors
                .help(&["The magnification ratio must be between 1 and 32768."]);
            crate::errors::int_error(st, mag)?;
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
            st.fonts.font_size[f] == st.errors.xn_over_d(st.fonts.font_dsize[f], -s.raw(), 1000)
        }
    };
    let f = 'search: {
        for f in st.fonts.font_ptr.loaded() {
            let area_matches = (st.strings.length(st.scanner.cur_area) == 0
                && st.fonts.is_native_font(f))
                || st
                    .strings
                    .str_eq_str(st.fonts.font_area[f], st.scanner.cur_area);
            if st
                .strings
                .str_eq_str(st.fonts.font_name[f], st.scanner.cur_name)
                && area_matches
                && size_matches(st, f)
            {
                break 'search f;
            }
            for part in [st.scanner.cur_area, st.scanner.cur_name, st.scanner.cur_ext] {
                st.strings.append_str(part)?;
            }
            let full_name = st.strings.make_string()?;
            let same_name = st.strings.str_eq_str(st.fonts.font_name[f], full_name);
            flush_last_string(st);
            if same_name && st.fonts.is_native_font(f) && size_matches(st, f) {
                break 'search f;
            }
        }
        read_font_info(st, u, st.scanner.cur_name, st.scanner.cur_area, s)?
    };

    define(st, global, u, Cmd::SetFont, f.raw())?;
    let word = st.eqtb.word(u);
    st.eqtb.set_word(f.id_loc(), word);
    st.eqtb.set_text(f.id_loc(), t);
    Ok(())
}

fn do_register_command(st: &mut EngineState, a: i16) -> Result<(), Abort> {
    let q = st.scanner.cur_cmd;
    let (l, p) = 'found: {
        if q != Cmd::Register {
            crate::scan::get_x_token(st)?;
            let level = match st.scanner.cur_cmd {
                Cmd::AssignInt => Some(ValueLevel::Int),
                Cmd::AssignDimen => Some(ValueLevel::Dimen),
                Cmd::AssignGlue => Some(ValueLevel::Glue),
                Cmd::AssignMuGlue => Some(ValueLevel::Mu),
                _ => None,
            };
            if let Some(level) = level {
                break 'found (RegisterLoc::Eqtb(st.scanner.chr_loc()), level);
            }
            if st.scanner.cur_cmd != Cmd::Register {
                crate::output::error_here_with_diagnostic(st, "You can't use `");
                crate::output::print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
                crate::output::print(st, "' after ");
                crate::output::print_cmd_chr(st, q, 0);
                crate::output::capture_to_diagnostic(st, None);
                st.errors
                    .help(&["I'm forgetting what you said and not changing anything."]);
                return crate::errors::error(st);
            }
        }
        let Some(p) = ValueLevel::from_raw(st.scanner.cur_chr) else {
            let sa = NodePtr::<SaNode>::at(MemPtr::from_raw(st.scanner.cur_chr));
            break 'found (RegisterLoc::Sparse(sa), sa.sa_type(&st.mem));
        };
        crate::scan::scan_register_num(st)?;
        let n = st.scanner.cur_val;
        if n > 255 {
            let sa = st.mem.create_sa_element(&mut st.scanner, p, n)?;
            break 'found (RegisterLoc::Sparse(sa), p);
        }
        let base = match p {
            ValueLevel::Int => COUNT_BASE,
            ValueLevel::Dimen => SCALED_BASE,
            ValueLevel::Glue => SKIP_BASE,
            ValueLevel::Mu => MU_SKIP_BASE,
            _ => unreachable!("register value level"),
        };
        (RegisterLoc::Eqtb(base + n), p)
    };

    let (w, s) = match (p < ValueLevel::Glue, l) {
        (true, RegisterLoc::Sparse(sa)) => (sa.sa_int(&st.mem), MemPtr::NULL),
        (true, RegisterLoc::Eqtb(l)) => (st.eqtb.equiv(l), MemPtr::NULL),
        (false, RegisterLoc::Sparse(sa)) => (0, sa.sa_ptr(&st.mem)),
        (false, RegisterLoc::Eqtb(l)) => (0, st.eqtb.equiv_ptr(l)),
    };

    if q == Cmd::Register {
        crate::scan::scan_optional_equals(st)?;
    } else {
        crate::scan::scan_keyword(st, "by")?;
    }
    st.errors.arith_error = false;

    if q < Cmd::Multiply {
        if p < ValueLevel::Glue {
            if p == ValueLevel::Int {
                crate::scan::scan_int(st)?;
            } else {
                crate::scan::scan_dimen(st, false, false, false)?;
            }
            if q == Cmd::Advance {
                st.scanner.cur_val = st.scanner.cur_val.wrapping_add(w);
            }
        } else {
            crate::scan::scan_glue(st, p)?;
            if q == Cmd::Advance {
                let sum = st.mem.new_spec(st.scanner.cur_glue())?;
                st.mem.delete_glue_ref(st.scanner.cur_glue());
                let old = NodePtr::<GlueSpecNode>::from_glue_ptr(s);
                let width = sum.width(&st.mem) + old.width(&st.mem);
                sum.set_width(&mut st.mem, width);
                if sum.stretch(&st.mem) == Scaled::ZERO {
                    sum.set_stretch_order(&mut st.mem, NORMAL);
                }
                if sum.stretch_order(&st.mem) == old.stretch_order(&st.mem) {
                    let v = sum.stretch(&st.mem) + old.stretch(&st.mem);
                    sum.set_stretch(&mut st.mem, v);
                } else if sum.stretch_order(&st.mem) < old.stretch_order(&st.mem)
                    && old.stretch(&st.mem) != Scaled::ZERO
                {
                    let (v, order) = (old.stretch(&st.mem), old.stretch_order(&st.mem));
                    sum.set_stretch(&mut st.mem, v);
                    sum.set_stretch_order(&mut st.mem, order);
                }
                if sum.shrink(&st.mem) == Scaled::ZERO {
                    sum.set_shrink_order(&mut st.mem, NORMAL);
                }
                if sum.shrink_order(&st.mem) == old.shrink_order(&st.mem) {
                    let v = sum.shrink(&st.mem) + old.shrink(&st.mem);
                    sum.set_shrink(&mut st.mem, v);
                } else if sum.shrink_order(&st.mem) < old.shrink_order(&st.mem)
                    && old.shrink(&st.mem) != Scaled::ZERO
                {
                    let (v, order) = (old.shrink(&st.mem), old.shrink_order(&st.mem));
                    sum.set_shrink(&mut st.mem, v);
                    sum.set_shrink_order(&mut st.mem, order);
                }
                st.scanner.cur_val = sum.ptr().raw();
            }
        }
    } else {
        crate::scan::scan_int(st)?;
        let n = st.scanner.cur_val;
        let scale = |errors: &mut crate::state::ErrorState, x: i32, max_answer: i32| {
            if q == Cmd::Multiply {
                errors.mult_and_add(x, n, 0, max_answer)
            } else {
                errors.x_over_n(x, n)
            }
        };
        if p < ValueLevel::Glue {
            let max_answer = if p == ValueLevel::Int {
                TEX_INFINITY
            } else {
                MAX_HALFWORD
            };
            st.scanner.cur_val = scale(&mut st.errors, w, max_answer);
        } else {
            let old = NodePtr::<GlueSpecNode>::from_glue_ptr(s);
            let r = st.mem.new_spec(old)?;
            let (width, stretch, shrink) = (
                old.width(&st.mem),
                old.stretch(&st.mem),
                old.shrink(&st.mem),
            );
            let v = scale(&mut st.errors, width.raw(), MAX_HALFWORD);
            r.set_width(&mut st.mem, Scaled::from_raw(v));
            let v = scale(&mut st.errors, stretch.raw(), MAX_HALFWORD);
            r.set_stretch(&mut st.mem, Scaled::from_raw(v));
            let v = scale(&mut st.errors, shrink.raw(), MAX_HALFWORD);
            r.set_shrink(&mut st.mem, Scaled::from_raw(v));
            st.scanner.cur_val = r.ptr().raw();
        }
    }

    if st.errors.arith_error {
        crate::output::print_err(st, "Arithmetic overflow");
        st.errors.help(&[
            "I can't carry out that multiplication or division,",
            "since the result is out of range.",
        ]);
        if p >= ValueLevel::Glue {
            st.mem.delete_glue_ref(st.scanner.cur_glue());
        }
        return crate::errors::error(st);
    }

    let global = a >= 4;
    if p < ValueLevel::Glue {
        let value = st.scanner.cur_val;
        match l {
            RegisterLoc::Sparse(sa) => sa_word_define(st, global, sa, value)?,
            RegisterLoc::Eqtb(l) => word_define(st, global, l, value)?,
        }
    } else {
        trap_zero_glue(st);
        let value = st.scanner.cur_glue().ptr();
        match l {
            RegisterLoc::Sparse(sa) => sa_define(st, global, sa, value)?,
            RegisterLoc::Eqtb(l) => define(st, global, l, Cmd::GlueRef, value.raw())?,
        }
    }
    Ok(())
}

pub(super) fn do_assignments(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        crate::scan::get_x_non_blank_non_relax(st)?;
        if st.scanner.cur_cmd <= Cmd::MAX_NON_PREFIXED_COMMAND {
            return Ok(());
        }
        st.scanner.set_box_allowed = false;
        prefixed_command(st)?;
        st.scanner.set_box_allowed = true;
    }
}

pub(super) fn define(
    st: &mut EngineState,
    global: bool,
    p: EqtbPtr,
    t: Cmd,
    e: i32,
) -> Result<(), Abort> {
    if global {
        st.eqtb.geq_define(&mut st.mem, &mut st.scanner, p, t, e)
    } else {
        st.eqtb
            .eq_define(&mut st.save_stack, &mut st.mem, &mut st.scanner, p, t, e)
    }
}

pub(super) fn word_define(
    st: &mut EngineState,
    global: bool,
    p: EqtbPtr,
    w: i32,
) -> Result<(), Abort> {
    if global {
        st.eqtb.geq_word_define(p, w);
        Ok(())
    } else {
        st.eqtb.eq_word_define(&mut st.save_stack, p, w)
    }
}

pub(super) fn sa_define(
    st: &mut EngineState,
    global: bool,
    p: NodePtr<SaNode>,
    e: MemPtr,
) -> Result<(), Abort> {
    let result = if global {
        st.mem.gsa_def(p, e)
    } else {
        st.mem.sa_def(&mut st.scanner, &mut st.save_stack, p, e)?
    };
    crate::save::finish_sa_mutation(st, p, result)?;
    Ok(())
}

fn sa_word_define(
    st: &mut EngineState,
    global: bool,
    p: NodePtr<SaNode>,
    w: i32,
) -> Result<(), Abort> {
    let result = if global {
        st.mem.gsa_w_def(p, w)
    } else {
        st.mem.sa_w_def(&mut st.scanner, &mut st.save_stack, p, w)?
    };
    crate::save::finish_sa_mutation(st, p, result)?;
    Ok(())
}

impl MemState {
    fn add_sa_ref(&mut self, p: MemPtr) {
        if let Some(sa) = NodePtr::<SaNode>::from_ptr(p) {
            let refs = sa.sa_ref(self);
            sa.set_sa_ref(self, refs + 1);
        }
    }
}

/// The `sa_ptr` of a sparse-array record, or null without one.
/// Scans a pair of character classes into its `\XeTeXinterchartoks` index.
fn scan_inter_char_pair(st: &mut EngineState) -> Result<i32, Abort> {
    crate::scan::scan_char_class_not_ignored(st)?;
    // TeX parks the first class in `cur_ptr` while scanning the second.
    st.scanner.cur_ptr = MemPtr::from_raw(st.scanner.cur_val);
    crate::scan::scan_char_class_not_ignored(st)?;
    Ok(st.scanner.cur_ptr.raw() * CHAR_CLASS_LIMIT + st.scanner.cur_val)
}

/// Defines token list register `p`, a sparse-array entry if `e`, as `list`.
/// Where a register assignment stores its value.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RegisterLoc {
    Eqtb(EqtbPtr),
    Sparse(NodePtr<SaNode>),
}

fn define_toks(
    st: &mut EngineState,
    global: bool,
    target: RegisterLoc,
    list: TokenList,
) -> Result<(), Abort> {
    match target {
        RegisterLoc::Sparse(sa) => sa_define(st, global, sa, list.ptr()),
        RegisterLoc::Eqtb(p) => {
            let t = if list.is_null() {
                Cmd::UndefinedCs
            } else {
                Cmd::Call
            };
            define(st, global, p, t, list.ptr().raw())
        }
    }
}

pub(super) fn prefixed_command(st: &mut EngineState) -> Result<(), Abort> {
    let mut a: i32 = 0;
    while st.scanner.cur_cmd == Cmd::Prefix {
        if (a / st.scanner.cur_chr) & 1 == 0 {
            a += st.scanner.cur_chr;
        }
        crate::scan::get_x_non_blank_non_relax(st)?;
        if st.scanner.cur_cmd <= Cmd::MAX_NON_PREFIXED_COMMAND {
            crate::output::error_here_with_diagnostic(st, "You can't use a prefix with `");
            crate::output::print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
            crate::output::print_char(st, i32::from(b'\''));
            crate::output::capture_to_diagnostic(st, None);
            st.errors
                .help(&[r"I'll pretend you didn't say \long or \outer or \global or \protected."]);
            return crate::input::back_error(st);
        }
        if st.eqtb.int_par(IntPar::TracingCommands) > 2 {
            crate::output::show_cur_cmd_chr(st);
        }
    }

    let protected = a >= 8;
    if protected {
        a -= 8;
    }
    if st.scanner.cur_cmd != Cmd::Def && (a % 4 != 0 || protected) {
        crate::output::error_here_with_diagnostic(st, "You can't use `");
        crate::output::print_esc(st, "long");
        crate::output::print(st, "' or `");
        crate::output::print_esc(st, "outer");
        crate::output::print(st, "' or `");
        crate::output::print_esc(st, "protected");
        crate::output::print(st, "' with `");
        crate::output::print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
        crate::output::print_char(st, i32::from(b'\''));
        crate::output::capture_to_diagnostic(st, None);
        st.errors
            .help(&[r"I'll pretend you didn't say \long or \outer or \protected here."]);
        crate::errors::error(st)?;
    }
    let global_defs = st.eqtb.int_par(IntPar::GlobalDefs);
    if global_defs < 0 && a >= 4 {
        a -= 4;
    } else if global_defs > 0 && a < 4 {
        a += 4;
    }

    match st.scanner.cur_cmd {
        Cmd::SetFont => define(st, a >= 4, CUR_FONT_LOC, Cmd::Data, st.scanner.cur_chr)?,
        Cmd::Def => {
            if st.scanner.cur_chr & 1 != 0 && a < 4 && st.eqtb.int_par(IntPar::GlobalDefs) >= 0 {
                a += 4;
            }
            let expand = st.scanner.cur_chr >= 2;
            crate::scan::get_r_token(st)?;
            let p = st.scanner.cur_cs;
            crate::scan::scan_toks(st, true, expand)?;
            if protected {
                let q = st.mem.get_avail_token()?;
                q.set_token(&mut st.mem, Token::PROTECTED);
                let body = st.scanner.def_ref.link(&st.mem);
                q.set_link(&mut st.mem, body);
                st.scanner.def_ref.set_link(&mut st.mem, q);
            }
            define(
                st,
                a >= 4,
                p,
                Cmd::call(a % 2 == 1, a % 4 >= 2),
                st.scanner.def_ref.ptr().raw(),
            )?;
        }
        Cmd::Let => {
            let n = st.scanner.cur_chr;
            crate::scan::get_r_token(st)?;
            let p = st.scanner.cur_cs;
            if n == i32::from(NORMAL) {
                loop {
                    crate::scan::get_token(st)?;
                    if st.scanner.cur_cmd != Cmd::Spacer {
                        break;
                    }
                }
                if st.scanner.cur_tok == Token::other(b'=') {
                    crate::scan::get_token(st)?;
                    if st.scanner.cur_cmd == Cmd::Spacer {
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
            if st.scanner.cur_cmd >= Cmd::Call {
                st.mem.add_token_ref(st.scanner.cur_chr_tokens());
            } else if (st.scanner.cur_cmd == Cmd::Register
                || st.scanner.cur_cmd == Cmd::ToksRegister)
                && !(0..=19).contains(&st.scanner.cur_chr)
            {
                st.mem.add_sa_ref(st.scanner.cur_chr_ptr());
            }
            define(st, a >= 4, p, st.scanner.cur_cmd, st.scanner.cur_chr)?;
        }
        Cmd::ShorthandDef => {
            if st.scanner.cur_chr == ShorthandDefCode::CharSub.raw() {
                crate::output::print_err(st, "vestigial MLTeX shorthand encountered??");
                st.errors
                    .help(&["This should never happen in Tectonic, where MLTeX has been excised."]);
                crate::errors::error(st)?;
            } else {
                shorthand_def(st, a >= 4)?;
            }
        }
        Cmd::ReadToCs => {
            let j = st.scanner.cur_chr;
            crate::scan::scan_int(st)?;
            let n = st.scanner.cur_val;
            if !crate::scan::scan_keyword(st, "to")? {
                crate::output::print_err(st, "Missing `to' inserted");
                st.errors.help(&[
                    r"You should have said `\read<number> to \cs'.",
                    r"I'm going to look for the \cs now.",
                ]);
                crate::errors::error(st)?;
            }
            crate::scan::get_r_token(st)?;
            let p = st.scanner.cur_cs;
            crate::scan::read_toks(st, n, p, j)?;
            define(st, a >= 4, p, Cmd::Call, st.scanner.cur_val)?;
        }
        Cmd::ToksRegister | Cmd::AssignToks => assign_toks(st, a >= 4)?,
        Cmd::AssignInt => {
            let p = st.scanner.chr_loc();
            crate::scan::scan_optional_equals(st)?;
            crate::scan::scan_int(st)?;
            word_define(st, a >= 4, p, st.scanner.cur_val)?;
        }
        Cmd::AssignDimen => {
            let p = st.scanner.chr_loc();
            crate::scan::scan_optional_equals(st)?;
            crate::scan::scan_dimen(st, false, false, false)?;
            word_define(st, a >= 4, p, st.scanner.cur_val)?;
        }
        Cmd::AssignGlue | Cmd::AssignMuGlue => {
            let p = st.scanner.chr_loc();
            let level = if st.scanner.cur_cmd == Cmd::AssignMuGlue {
                ValueLevel::Mu
            } else {
                ValueLevel::Glue
            };
            crate::scan::scan_optional_equals(st)?;
            crate::scan::scan_glue(st, level)?;
            trap_zero_glue(st);
            define(st, a >= 4, p, Cmd::GlueRef, st.scanner.cur_val)?;
        }
        Cmd::XetexDefCode => xetex_def_code(st, a >= 4)?,
        Cmd::DefCode => def_code(st, a >= 4)?,
        Cmd::DefFamily => {
            let base = st.scanner.chr_loc();
            crate::scan::scan_math_fam_int(st)?;
            let p = base + st.scanner.cur_val;
            crate::scan::scan_optional_equals(st)?;
            crate::scan::scan_font_ident(st)?;
            define(st, a >= 4, p, Cmd::Data, st.scanner.cur_val)?;
        }
        Cmd::Register | Cmd::Advance | Cmd::Multiply | Cmd::Divide => {
            do_register_command(st, a as i16)?
        }
        Cmd::SetBox => {
            crate::scan::scan_register_num(st)?;
            let n = if a >= 4 {
                GLOBAL_BOX_FLAG + st.scanner.cur_val
            } else {
                BOX_FLAG + st.scanner.cur_val
            };
            crate::scan::scan_optional_equals(st)?;
            if st.scanner.set_box_allowed {
                scan_box(st, n)?;
            } else {
                crate::output::error_here_with_diagnostic(st, "Improper ");
                crate::output::print_esc(st, "setbox");
                crate::output::capture_to_diagnostic(st, None);
                st.errors.help(&[
                    r"Sorry, \setbox is not allowed after \halign in a display,",
                    r"or between \accent and an accented character.",
                ]);
                crate::errors::error(st)?;
            }
        }
        Cmd::SetAux => alter_aux(st)?,
        Cmd::SetPrevGraf => alter_prev_graf(st)?,
        Cmd::SetPageDimen => alter_page_so_far(st)?,
        Cmd::SetPageInt => alter_integer(st)?,
        Cmd::SetBoxDimen => alter_box_dimen(st)?,
        Cmd::SetShape => set_shape(st, a >= 4)?,
        Cmd::HyphData => {
            if st.scanner.cur_chr != 1 {
                new_hyph_exceptions(st)?;
            } else if st.config.in_initex_mode {
                new_patterns(st)?;
            } else {
                crate::output::print_err(st, "Patterns can be loaded only by INITEX");
                st.errors.help(&[]);
                crate::errors::error(st)?;
                loop {
                    crate::scan::get_token(st)?;
                    if st.scanner.cur_cmd == Cmd::RightBrace {
                        return Ok(());
                    }
                }
            }
        }
        Cmd::AssignFontDimen => {
            crate::scan::find_font_dimen(st, true)?;
            let k = st.scanner.cur_val;
            crate::scan::scan_optional_equals(st)?;
            crate::scan::scan_dimen(st, false, false, false)?;
            st.fonts.set_font_info_sc(k, st.scanner.cur_dimen());
        }
        Cmd::AssignFontInt => {
            let n = st.scanner.cur_chr;
            let f = crate::scan::scan_font_ident(st)?;
            if n < 2 {
                crate::scan::scan_optional_equals(st)?;
                crate::scan::scan_int(st)?;
                if n == 0 {
                    st.fonts.hyphen_char[f] = st.scanner.cur_val;
                } else {
                    st.fonts.skew_char[f] = st.scanner.cur_val;
                }
            } else {
                if st.fonts.is_native_font(f) {
                    crate::scan::scan_glyph_number(st, f)?;
                } else {
                    crate::scan::scan_char_num(st)?;
                }
                let p = st.scanner.cur_val;
                crate::scan::scan_optional_equals(st)?;
                crate::scan::scan_int(st)?;
                match n {
                    LP_CODE_BASE => set_cp_code(f.raw(), p as u32, LEFT_SIDE, st.scanner.cur_val),
                    RP_CODE_BASE => set_cp_code(f.raw(), p as u32, RIGHT_SIDE, st.scanner.cur_val),
                    _ => {}
                }
            }
        }
        Cmd::DefFont => new_font(st, a as i16)?,
        Cmd::SetInteraction => new_interaction(st),
        _ => return Err(Abort::Confusion("prefix")),
    }

    if st.scanner.after_token != Token::ZERO {
        st.scanner.cur_tok = st.scanner.after_token;
        crate::input::back_input(st)?;
        st.scanner.after_token = Token::ZERO;
    }
    Ok(())
}

fn shorthand_def(st: &mut EngineState, global: bool) -> Result<(), Abort> {
    let n =
        ShorthandDefCode::from_raw(st.scanner.cur_chr).ok_or(Abort::Confusion("shorthand_def"))?;
    crate::scan::get_r_token(st)?;
    let p = st.scanner.cur_cs;
    define(st, global, p, Cmd::Relax, TOO_BIG_USV)?;
    crate::scan::scan_optional_equals(st)?;
    let (t, e) = match n {
        ShorthandDefCode::Char => {
            crate::scan::scan_usv_num(st)?;
            (Cmd::CharGiven, st.scanner.cur_val)
        }
        ShorthandDefCode::MathChar => {
            crate::scan::scan_fifteen_bit_int(st)?;
            (Cmd::MathGiven, st.scanner.cur_val)
        }
        ShorthandDefCode::XetexMathCharNum => {
            crate::scan::scan_xetex_math_char_int(st)?;
            (Cmd::XetexMathGiven, st.scanner.cur_val)
        }
        ShorthandDefCode::XetexMathChar => (Cmd::XetexMathGiven, crate::scan::scan_umath_code(st)?),
        _ => {
            crate::scan::scan_register_num(st)?;
            if st.scanner.cur_val > 255 {
                let level = match n {
                    ShorthandDefCode::Count => ValueLevel::Int,
                    ShorthandDefCode::Dimen => ValueLevel::Dimen,
                    ShorthandDefCode::Skip => ValueLevel::Glue,
                    ShorthandDefCode::MuSkip => ValueLevel::Mu,
                    _ => ValueLevel::Tok,
                };
                let reg = st.scanner.cur_val;
                let sa = st.mem.create_sa_element(&mut st.scanner, level, reg)?;
                st.mem.add_sa_ref(sa.ptr());
                let t = if level == ValueLevel::Tok {
                    Cmd::ToksRegister
                } else {
                    Cmd::Register
                };
                (t, st.scanner.cur_ptr.raw())
            } else {
                let (t, base) = match n {
                    ShorthandDefCode::Count => (Cmd::AssignInt, COUNT_BASE),
                    ShorthandDefCode::Dimen => (Cmd::AssignDimen, SCALED_BASE),
                    ShorthandDefCode::Skip => (Cmd::AssignGlue, SKIP_BASE),
                    ShorthandDefCode::MuSkip => (Cmd::AssignMuGlue, MU_SKIP_BASE),
                    ShorthandDefCode::Toks => (Cmd::AssignToks, TOKS_BASE),
                    _ => return Ok(()),
                };
                (t, (base + st.scanner.cur_val).raw())
            }
        }
    };
    define(st, global, p, t, e)?;
    Ok(())
}

fn assign_toks(st: &mut EngineState, global: bool) -> Result<(), Abort> {
    let cs = st.scanner.cur_cs;
    let target = if st.scanner.cur_cmd == Cmd::ToksRegister {
        if st.scanner.cur_chr == 0 {
            crate::scan::scan_register_num(st)?;
            if st.scanner.cur_val > 255 {
                let reg = st.scanner.cur_val;
                let sa = st
                    .mem
                    .create_sa_element(&mut st.scanner, ValueLevel::Tok, reg)?;
                RegisterLoc::Sparse(sa)
            } else {
                RegisterLoc::Eqtb(TOKS_BASE + st.scanner.cur_val)
            }
        } else {
            RegisterLoc::Sparse(NodePtr::at(MemPtr::from_raw(st.scanner.cur_chr)))
        }
    } else if st.scanner.chr_loc() == TokPar::XetexInterCharToks.loc() {
        let n = scan_inter_char_pair(st)?;
        let sa = st
            .mem
            .create_sa_element(&mut st.scanner, ValueLevel::InterChar, n)?;
        RegisterLoc::Sparse(sa)
    } else {
        RegisterLoc::Eqtb(st.scanner.chr_loc())
    };
    crate::scan::scan_optional_equals(st)?;
    crate::scan::get_x_non_blank_non_relax(st)?;

    if st.scanner.cur_cmd != Cmd::LeftBrace
        && (st.scanner.cur_cmd == Cmd::ToksRegister || st.scanner.cur_cmd == Cmd::AssignToks)
    {
        let list = if st.scanner.cur_cmd == Cmd::ToksRegister {
            if st.scanner.cur_chr == 0 {
                crate::scan::scan_register_num(st)?;
                if st.scanner.cur_val < 256 {
                    st.eqtb.toks(st.scanner.cur_val)
                } else {
                    let reg = st.scanner.cur_val;
                    let sa = st
                        .mem
                        .find_sa_element(&mut st.scanner, ValueLevel::Tok, reg);
                    crate::scan::sa_token_list(st, sa)
                }
            } else {
                NodePtr::<SaNode>::at(MemPtr::from_raw(st.scanner.cur_chr)).sa_toks(&st.mem)
            }
        } else if st.scanner.chr_loc() == TokPar::XetexInterCharToks.loc() {
            let n = scan_inter_char_pair(st)?;
            let sa = st
                .mem
                .find_sa_element(&mut st.scanner, ValueLevel::InterChar, n);
            crate::scan::sa_token_list(st, sa)
        } else {
            st.eqtb.equiv_tokens(st.scanner.chr_loc())
        };
        if !list.is_null() {
            st.mem.add_token_ref(list);
        }
        return define_toks(st, global, target, list);
    }

    crate::input::back_input(st)?;
    st.scanner.cur_cs = cs;
    let tail = crate::scan::scan_toks(st, false, false)?;
    let def_ref = st.scanner.def_ref;
    if def_ref.link(&st.mem).is_null() {
        define_toks(st, global, target, TokenList::NULL)?;
        st.mem.free_avail(def_ref.ptr());
        return Ok(());
    }
    if target == RegisterLoc::Eqtb(TokPar::OutputRoutine.loc()) {
        let mut tail = tail;
        st.mem
            .store_new_token(&mut tail, Token::right_brace(b'}'))?;
        let q = st.mem.get_avail_token()?;
        q.set_token(&mut st.mem, Token::left_brace(b'{'));
        let body = def_ref.link(&st.mem);
        q.set_link(&mut st.mem, body);
        def_ref.set_link(&mut st.mem, q);
    }
    define_toks(st, global, target, def_ref)
}

fn xetex_def_code(st: &mut EngineState, global: bool) -> Result<(), Abort> {
    let chr = st.scanner.chr_loc();
    let base = if chr == SF_CODE_BASE || chr == MATH_CODE_BASE || chr == DEL_CODE_BASE {
        chr
    } else {
        chr - 1
    };
    crate::scan::scan_usv_num(st)?;
    let p = base + st.scanner.cur_val;
    if chr == SF_CODE_BASE {
        let sf = st.eqtb.sf_code(st.scanner.cur_val) % 65536;
        crate::scan::scan_optional_equals(st)?;
        crate::scan::scan_char_class(st)?;
        let v = st.scanner.cur_val.wrapping_mul(65536).wrapping_add(sf);
        define(st, global, p, Cmd::Data, v)?;
    } else if chr == MATH_CODE_BASE {
        crate::scan::scan_optional_equals(st)?;
        crate::scan::scan_xetex_math_char_int(st)?;
        define(st, global, p, Cmd::Data, st.scanner.cur_val)?;
    } else if chr == MATH_CODE_BASE + 1 {
        crate::scan::scan_optional_equals(st)?;
        let v = crate::scan::scan_umath_code(st)?;
        define(st, global, p, Cmd::Data, v)?;
    } else if chr == DEL_CODE_BASE {
        crate::scan::scan_optional_equals(st)?;
        crate::scan::scan_int(st)?;
        word_define(st, global, p, st.scanner.cur_val)?;
    } else {
        crate::scan::scan_optional_equals(st)?;
        // Extended delimiter code flag.
        let mut n: i32 = 0x4000_0000;
        crate::scan::scan_math_fam_int(st)?;
        n = n.wrapping_add(st.scanner.cur_val.wrapping_mul(0x20_0000));
        crate::scan::scan_usv_num(st)?;
        n = n.wrapping_add(st.scanner.cur_val);
        word_define(st, global, p, n)?;
    }
    Ok(())
}

fn def_code(st: &mut EngineState, global: bool) -> Result<(), Abort> {
    let chr = st.scanner.chr_loc();
    let max = if chr == CAT_CODE_BASE {
        Cmd::MAX_CHAR_CODE.into()
    } else if chr == MATH_CODE_BASE {
        0x8000
    } else if chr == SF_CODE_BASE {
        0x7FFF
    } else if chr == DEL_CODE_BASE {
        0xFF_FFFF
    } else {
        BIGGEST_USV
    };
    crate::scan::scan_usv_num(st)?;
    let p = chr + st.scanner.cur_val;
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_int(st)?;
    if (st.scanner.cur_val < 0 && p < DEL_CODE_BASE) || st.scanner.cur_val > max {
        crate::output::error_here_with_diagnostic(st, "Invalid code (");
        crate::output::print_int(st, st.scanner.cur_val);
        if p < DEL_CODE_BASE {
            crate::output::print(st, "), should be in the range 0..");
        } else {
            crate::output::print(st, "), should be at most ");
        }
        crate::output::print_int(st, max);
        crate::output::capture_to_diagnostic(st, None);
        st.errors
            .help(&["I'm going to use 0 instead of that illegal code value."]);
        crate::errors::error(st)?;
        st.scanner.cur_val = 0;
    }
    if p < MATH_CODE_BASE {
        let v = if p >= SF_CODE_BASE {
            (st.eqtb.int_at(p) / 65536)
                .wrapping_mul(65536)
                .wrapping_add(st.scanner.cur_val)
        } else {
            st.scanner.cur_val
        };
        define(st, global, p, Cmd::Data, v)?;
    } else if p < DEL_CODE_BASE {
        st.scanner.cur_val = if st.scanner.cur_val == 32768 {
            ACTIVE_MATH_CHAR
        } else {
            crate::scan::math_code_from_tex(st.scanner.cur_val)
        };
        define(st, global, p, Cmd::Data, st.scanner.cur_val)?;
    } else {
        word_define(st, global, p, st.scanner.cur_val)?;
    }
    Ok(())
}

fn set_shape(st: &mut EngineState, global: bool) -> Result<(), Abort> {
    let q = st.scanner.chr_loc();
    crate::scan::scan_optional_equals(st)?;
    crate::scan::scan_int(st)?;
    let n = st.scanner.cur_val;
    let p = if n <= 0 {
        MemPtr::NULL
    } else if q > PAR_SHAPE_LOC {
        let words = n / 2 + 1;
        let p = st.mem.get_node(2 * words + 1)?;
        let penalties = NodePtr::<PenaltiesNode>::at(p);
        penalties.set_words(&mut st.mem, words);
        penalties.set_count(&mut st.mem, n);
        for k in 1..=n {
            crate::scan::scan_int(st)?;
            penalties.set_penalty(&mut st.mem, k, st.scanner.cur_val);
        }
        if n & 1 == 0 {
            penalties.set_penalty(&mut st.mem, n + 1, 0);
        }
        p
    } else {
        let p = st.mem.get_node(2 * n + 1)?;
        let shape = NodePtr::<ParShapeNode>::at(p);
        shape.set_lines(&mut st.mem, n);
        for line in 1..=n {
            crate::scan::scan_dimen(st, false, false, false)?;
            shape.set_indent(&mut st.mem, line, st.scanner.cur_dimen());
            crate::scan::scan_dimen(st, false, false, false)?;
            shape.set_line_width(&mut st.mem, line, st.scanner.cur_dimen());
        }
        p
    };
    define(st, global, q, Cmd::ShapeRef, p.raw())?;
    Ok(())
}
