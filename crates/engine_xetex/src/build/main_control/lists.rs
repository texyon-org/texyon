//! Building lists: paragraphs, glue, kerns, inserts, marks, discretionaries, accents.

use super::align::align_peek;
use super::assign::do_assignments;
use super::begin_every;
use super::boxes::package;
use crate::base::cmd::Cmd;
use crate::base::codes::{BoxCode, SkipCode, ValueLevel};
use crate::base::command::Command;
use crate::base::consts::{
    BIGGEST_CHAR, EMPTY, MAX_DIMEN, NULL_FLAG, SUB_BOX, SUB_MLIST, TEX_NULL,
};
use crate::base::mode::{Mode, ModeName};
use crate::base::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::base::scaled_math::xn_over_d;
use crate::base::token::Token;
use crate::base::units::{FontId, MemAddr, MemIdx, MemPtr, NodeType, Scaled};
use crate::build::extension::insert_src_special;
use crate::build::hyphenation::{norm_lang, norm_min};
use crate::build::linebreak::line_break;
use crate::build::math::{build_choices, fin_mlist};
use crate::build::packaging::{PackSpec, hpack, vpackage};
use crate::build::pagebuilder::build_page;
use crate::fonts::{
    EXTRA_SPACE_CODE, SLANT_CODE, SPACE_CODE, SPACE_SHRINK_CODE, SPACE_STRETCH_CODE, X_HEIGHT_CODE,
    new_native_character,
};
use crate::memory::glue_store::GlueSpecId;
use crate::memory::mem::{
    CONTRIB_HEAD, FIL_GLUE, FIL_NEG_GLUE, FILL_GLUE, MemState, PAGE_HEAD, SS_GLUE, ZERO_GLUE,
    push_nest,
};
use crate::memory::mem_types::ListStateRecord;
use crate::memory::node::{
    AdjustNode, AdjustPlacement, AnyNode, CharNode, DiscNode, GlueKind, GlueNode, InsNode,
    KernSubtype, LanguageNode, LrNode, MarkNode, MathSlot, NativeWordNode, Noad, NoadVariant,
    NodePtr, View, WhatsitSubtype, WhatsitVariant,
};
use crate::print::errors::{
    Abort, OrConfusion, box_error, error, extra_right_brace, ins_error, privileged,
};
use crate::print::{Esc, begin_diagnostic, char_warning, end_diagnostic, print_nl, show_box};
use crate::scanning::input::{TOKEN_LIST, TokenType, back_input, begin_token_list, end_token_list};
use crate::scanning::scan::{
    get_token, scan_char_num, scan_dimen, scan_eight_bit_int, scan_glue, scan_int, scan_keyword,
    scan_left_brace, scan_register_num, scan_toks,
};
use crate::state::EngineState;
use crate::tables::cs::Frozen;
use crate::tables::eqtb::{EqtbState, Register, ShapeLoc, ShapeRef, define};
use crate::tables::save::{GroupCode, Insertion, Saved, unsave};

pub(crate) fn push_math(st: &mut EngineState, c: GroupCode) -> Result<(), Abort> {
    push_nest(st)?;
    st.input.cur_list.mode = Mode::math(true);
    st.save_stack.new_save_level(c, st.mem.line)?;
    Ok(())
}

pub(super) fn end_graf(st: &mut EngineState) -> Result<(), Abort> {
    if !st.input.cur_list.mode.is_paragraph() {
        return Ok(());
    }
    if st.input.cur_list.head == st.input.cur_list.tail {
        st.mem.pop_nest(&mut st.input)?;
    } else {
        line_break(st, false)?;
    }
    let lr_save = st.input.cur_list.e_tex_aux;
    if !lr_save.is_none() {
        st.mem.flush_records::<LrNode>(lr_save);
        st.input.cur_list.e_tex_aux = None;
    }
    normal_paragraph(st)?;
    st.errors.error_count = 0;
    Ok(())
}

pub(super) fn append_penalty(st: &mut EngineState) -> Result<(), Abort> {
    let n = scan_int(st)?;
    let p = st.mem.new_penalty(n)?;
    st.input.cur_list.tail_append(&mut st.mem, p);
    if st.input.cur_list.mode.name() == ModeName::Vertical {
        build_page(st)?;
    }
    Ok(())
}

pub(crate) fn normal_paragraph(st: &mut EngineState) -> Result<(), Abort> {
    if st.eqtb.int_par(IntPar::Looseness) != 0 {
        define(st, false, IntPar::Looseness.loc(), 0)?;
    }
    if st.eqtb.dimen_par(DimenPar::HangIndent) != Scaled::ZERO {
        define(st, false, DimenPar::HangIndent.loc(), Scaled::ZERO)?;
    }
    if st.eqtb.int_par(IntPar::HangAfter) != 1 {
        define(st, false, IntPar::HangAfter.loc(), 1)?;
    }
    for loc in [ShapeLoc::ParShape, ShapeLoc::InterLinePenalties] {
        if st.eqtb.shapes.get(loc) != ShapeRef::NONE {
            define(st, false, loc, ShapeRef::NONE)?;
        }
    }
    Ok(())
}

pub(super) fn its_all_over(st: &mut EngineState) -> Result<bool, Abort> {
    if !privileged(st)? {
        return Ok(false);
    }
    if st.shipout.page_tail == Some(PAGE_HEAD)
        && st.input.cur_list.head == st.input.cur_list.tail
        && st.shipout.dead_cycles == 0
    {
        return Ok(true);
    }
    back_input(st)?;
    let b = st.mem.new_null_box()?;
    st.input.cur_list.tail_append(&mut st.mem, b);
    st.mem[b].width = st.eqtb.dimen_par(DimenPar::Hsize);
    let g = st.mem.new_glue(FILL_GLUE)?;
    st.input.cur_list.tail_append(&mut st.mem, g);
    let p = st.mem.new_penalty(NULL_FLAG)?;
    st.input.cur_list.tail_append(&mut st.mem, p);
    build_page(st)?;
    Ok(false)
}

pub(super) fn indent_in_hmode(st: &mut EngineState, indent: bool) -> Result<(), Abort> {
    if !indent {
        return Ok(());
    }
    let b = st.mem.new_null_box()?;
    st.mem[b].width = st.eqtb.dimen_par(DimenPar::ParIndent);
    let p = if let Mode::Horizontal { space_factor, .. } = &mut st.input.cur_list.mode {
        *space_factor = 1000;
        b.idx()
    } else {
        let q = st.mem.new_noad()?;
        st.mem[q].nucleus.math_type = SUB_BOX;
        st.mem[q].nucleus.set_info(b.ptr());
        q.idx()
    };
    st.input.cur_list.tail_append(&mut st.mem, p);
    Ok(())
}

fn back_input_after_par_token(st: &mut EngineState) -> Result<(), Abort> {
    back_input(st)?;
    st.scanner.cur_tok = st.input.par_token;
    back_input(st)?;
    st.input.cur_input.set_token_type(TokenType::Inserted);
    Ok(())
}

/// The least `\partokencontext` at which a group end in horizontal mode inserts the par token.
#[derive(Clone, Copy)]
pub(super) enum ParTokenContext {
    Boxes = 1,
    /// Also `\insert`, `\vadjust`, `\output`, `\noalign` and `\valign` items.
    AllGroups = 2,
}

impl ParTokenContext {
    fn of_group(group: GroupCode) -> Option<ParTokenContext> {
        match group {
            GroupCode::Vbox | GroupCode::Vtop | GroupCode::Vcenter => Some(ParTokenContext::Boxes),
            GroupCode::Insert | GroupCode::Output | GroupCode::NoAlign => {
                Some(ParTokenContext::AllGroups)
            }
            _ => None,
        }
    }
}

/// partoken.ch: ends an open paragraph with the par token instead of `end_graf`.
pub(super) fn insert_par_token_before(
    st: &mut EngineState,
    context: ParTokenContext,
) -> Result<bool, Abort> {
    if st.eqtb.int_par(IntPar::PartokenContext) < context as i32
        || !st.input.cur_list.mode.is_paragraph()
    {
        return Ok(false);
    }
    back_input_after_par_token(st)?;
    Ok(true)
}

pub(super) fn head_for_vmode(st: &mut EngineState) -> Result<(), Abort> {
    if !st.input.cur_list.mode.is_inner() {
        return back_input_after_par_token(st);
    }
    if st.scanner.cur.cmd() != Cmd::Hrule {
        return off_save(st);
    }
    error(
        st,
        (
            "You can't use `",
            Esc("hrule"),
            "' here except with leaders",
        ),
        &[
            "To put a horizontal rule in an hbox or an alignment,",
            r"you should use \leaders or \hrulefill (see The TeXbook).",
        ],
    )
}

pub(super) fn begin_insert_or_adjust(st: &mut EngineState) -> Result<(), Abort> {
    let n = if st.scanner.cur.cmd() == Cmd::Vadjust {
        255
    } else {
        match scan_eight_bit_int(st)? {
            255 => {
                error(
                    st,
                    ("You can't ", Esc("insert"), 255),
                    &[r"I'm changing to \insert0; box 255 is special."],
                )?;
                0
            }
            n => n,
        }
    };
    let pre = st.scanner.cur.cmd() == Cmd::Vadjust && scan_keyword(st, "pre")?;
    st.save_stack.push_saved(Saved::Insert { n, pre });
    st.save_stack
        .new_save_level(GroupCode::Insert, st.mem.line)?;
    scan_left_brace(st)?;
    normal_paragraph(st)?;
    push_nest(st)?;
    st.input.cur_list.mode = Mode::vertical(true);
    Ok(())
}

pub(super) fn app_space(st: &mut EngineState) -> Result<(), Abort> {
    let space_factor = st.input.cur_list.space_factor()?;
    let q;
    if space_factor >= 2000 && st.eqtb.glue_par(GluePar::XspaceSkip) != ZERO_GLUE {
        q = st.mem.new_param_glue(&st.eqtb, GluePar::XspaceSkip)?.idx();
    } else {
        let space_skip = st.eqtb.glue_par(GluePar::SpaceSkip);
        let base = if space_skip != ZERO_GLUE {
            space_skip
        } else {
            font_space_glue(st)?
        };
        let spec = st.mem.new_spec(base)?;
        if space_factor >= 2000 {
            let cf = st.eqtb.cur_font();
            let extra = st.fonts.param(cf, EXTRA_SPACE_CODE);
            let width = st.mem[spec].width + extra;
            st.mem[spec].width = width;
        }
        let stretch = xn_over_d(st.mem[spec].stretch, space_factor, 1000).quotient;
        st.mem[spec].stretch = stretch;
        let shrink = xn_over_d(st.mem[spec].shrink, 1000, space_factor).quotient;
        st.mem[spec].shrink = shrink;
        q = st.mem.new_glue(spec)?.idx();
        st.mem[spec].ref_count = TEX_NULL;
    }
    st.input.cur_list.tail_append(&mut st.mem, q);
    Ok(())
}

pub(super) fn font_space_glue(st: &mut EngineState) -> Result<GlueSpecId, Abort> {
    let cf = st.eqtb.cur_font();
    if let Some(glue) = st.fonts.font_glue[cf] {
        return Ok(glue);
    }
    let spec = st.mem.new_spec(ZERO_GLUE)?;
    let node = &mut st.mem[spec];
    node.width = st.fonts.param(cf, SPACE_CODE);
    node.stretch = st.fonts.param(cf, SPACE_STRETCH_CODE);
    node.shrink = st.fonts.param(cf, SPACE_SHRINK_CODE);
    st.fonts.font_glue[cf] = Some(spec);
    Ok(spec)
}

pub(crate) fn start_paragraph_language(st: &mut EngineState) -> Result<(), Abort> {
    let l = norm_lang(st.eqtb.int_par(IntPar::Language));
    st.hyph.cur_lang = l;
    st.input.cur_list.set_clang(i32::from(l))?;
    let lhm = i32::from(norm_min(st.eqtb.int_par(IntPar::LeftHyphenMin)));
    let rhm = i32::from(norm_min(st.eqtb.int_par(IntPar::RightHyphenMin)));
    st.input.cur_list.prev_graf = (lhm * 64 + rhm) * 65536 + i32::from(l);
    Ok(())
}

pub(crate) fn new_graf(st: &mut EngineState, indented: bool) -> Result<(), Abort> {
    st.input.cur_list.prev_graf = 0;
    if st.input.cur_list.mode.name() == ModeName::Vertical
        || st.input.cur_list.head != st.input.cur_list.tail
    {
        let g = st.mem.new_param_glue(&st.eqtb, GluePar::ParSkip)?;
        st.input.cur_list.tail_append(&mut st.mem, g);
    }
    push_nest(st)?;
    st.input.cur_list.mode = Mode::horizontal(false);

    start_paragraph_language(st)?;

    if indented {
        let b = st.mem.new_null_box()?;
        st.input.cur_list.tail = b.idx();
        st.input.cur_list.head.set_link(&mut st.mem, b.ptr());
        st.mem[b].width = st.eqtb.dimen_par(DimenPar::ParIndent);
        if st.shipout.insert_src_special_every_par {
            insert_src_special(st)?;
        }
    }
    begin_every(st, TokPar::EveryPar, TokenType::EveryPar)?;
    if st.input.nest.len() == 1 {
        build_page(st)?;
    }
    Ok(())
}

#[derive(Copy, Clone)]
pub(crate) enum NewChar {
    Char(NodePtr<CharNode>),
    Native(NodePtr<NativeWordNode>),
}

impl NewChar {
    pub(crate) fn ptr(self) -> MemPtr {
        Some(self.addr())
    }
}

impl MemAddr for NewChar {
    fn addr(self) -> MemIdx {
        match self {
            NewChar::Char(c) => c.idx(),
            NewChar::Native(w) => w.idx(),
        }
    }
}

pub(crate) fn new_character(
    st: &mut EngineState,
    f: FontId,
    c: u16,
) -> Result<Option<NewChar>, Abort> {
    if st.fonts.is_native_font(f) {
        return new_native_character(st, f, u32::from(c)).map(|w| Some(NewChar::Native(w)));
    }
    let ec = st.fonts.effective_char(f, c) as u16;
    if st.fonts.char_exists(f, i32::from(ec)) {
        let p = st.mem.new_char(f, c)?;
        return Ok(Some(NewChar::Char(p)));
    }
    char_warning(st, f, i32::from(c))?;
    Ok(None)
}

pub(super) fn handle_right_brace(st: &mut EngineState) -> Result<(), Abort> {
    if let Some(context) = ParTokenContext::of_group(st.save_stack.cur_group)
        && insert_par_token_before(st, context)?
    {
        return Ok(());
    }
    let group = st.save_stack.cur_group;
    match group {
        GroupCode::Simple => unsave(st)?,
        GroupCode::BottomLevel => {
            error(
                st,
                "Too many }'s",
                &[
                    "You've closed more groups than you opened.",
                    "Such booboos are generally harmless, so keep going.",
                ],
            )?;
        }
        GroupCode::SemiSimple | GroupCode::MathShift | GroupCode::MathLeft => {
            extra_right_brace(st)?;
        }
        GroupCode::Hbox | GroupCode::AdjustedHbox => package(st, group)?,
        GroupCode::Vbox => {
            end_graf(st)?;
            package(st, group)?;
        }
        GroupCode::Vtop => {
            end_graf(st)?;
            package(st, group)?;
        }
        GroupCode::Insert => {
            end_graf(st)?;
            let split_top_skip = st.eqtb.glue_par(GluePar::SplitTopSkip);
            st.mem.add_glue_ref(split_top_skip);
            let d = st.eqtb.dimen_par(DimenPar::SplitMaxDepth);
            let f = st.eqtb.int_par(IntPar::FloatingPenalty);
            unsave(st)?;
            let Some(Saved::Insert { n, pre }) = st.save_stack.pop_saved() else {
                return Err(Abort::Confusion("insert"));
            };
            let head = st.input.cur_list.head.link(&st.mem);
            let p = vpackage(st, head, PackSpec::NATURAL, MAX_DIMEN)?;
            st.mem.pop_nest(&mut st.input)?;
            if n < 255 {
                let ins = NodePtr::<InsNode>::alloc(&mut st.mem)?;
                st.input.cur_list.tail_append(&mut st.mem, ins);
                st.mem[ins].box_number = u8::try_from(n).or_confusion("insert")?;
                let height = st.mem[p].height + st.mem[p].depth;
                st.mem[ins].height = height;
                let list = st.mem[p].list_ptr;
                let node = &mut st.mem[ins];
                node.ins_ptr = list;
                node.split_top_ptr = split_top_skip;
                node.depth = d;
                node.float_cost = f;
            } else {
                let adj = NodePtr::<AdjustNode>::alloc(&mut st.mem)?;
                st.input.cur_list.tail_append(&mut st.mem, adj);
                st.mem[adj].placement = if pre {
                    AdjustPlacement::Pre
                } else {
                    AdjustPlacement::Post
                }
                .into();
                let list = st.mem[p].list_ptr;
                st.mem[adj].adjust_ptr = list;
                st.mem.delete_glue_ref(split_top_skip);
            }
            p.free(&mut st.mem);
            if st.input.nest.is_empty() {
                build_page(st)?;
            }
        }
        GroupCode::Output => {
            while st.input.cur_input.state == TOKEN_LIST
                && st.input.cur_input.token_loc().is_end()
                && st.input.cur_input.token_type() == TokenType::BackedUp
            {
                end_token_list(st)?;
            }
            if st.input.cur_input.state != TOKEN_LIST
                || !st.input.cur_input.token_loc().is_end()
                || st.input.cur_input.token_type() != TokenType::Output
            {
                error(
                    st,
                    "Unbalanced output routine",
                    &[
                        "Your sneaky output routine has problematic {'s and/or }'s.",
                        "I can't handle that very well; good luck.",
                    ],
                )?;
                loop {
                    get_token(st)?;
                    if st.input.cur_input.token_loc().is_end() {
                        break;
                    }
                }
            }
            st.shipout.output_can_end = true;
            end_token_list(st)?;
            st.shipout.output_can_end = false;
            end_graf(st)?;
            unsave(st)?;
            st.shipout.output_active = false;
            st.shipout.insert_penalties = 0;

            if !st.eqtb.box_reg(Register::of(255)).is_none() {
                box_error(
                    st,
                    255,
                    ("Output routine didn't use all of ", Esc("box"), 255),
                    &[
                        r"Your \output commands should empty \box255,",
                        r"e.g., by saying `\shipout\box255'.",
                        "Proceed; I'll discard its present contents.",
                    ],
                )?;
            }

            if st.input.cur_list.tail != st.input.cur_list.head {
                let first = st.input.cur_list.head.link(&st.mem);
                let page_tail = st.shipout.page_tail.or_confusion("page_tail")?;
                page_tail.set_link(&mut st.mem, first);
                st.shipout.page_tail = Some(st.input.cur_list.tail);
            }
            if !PAGE_HEAD.link(&st.mem).is_none() {
                let page_tail = st.shipout.page_tail.or_confusion("page_tail")?;
                if CONTRIB_HEAD.link(&st.mem).is_none() {
                    st.input.nest[0].tail = page_tail;
                }
                let contrib = CONTRIB_HEAD.link(&st.mem);
                page_tail.set_link(&mut st.mem, contrib);
                let page = PAGE_HEAD.link(&st.mem);
                CONTRIB_HEAD.set_link(&mut st.mem, page);
                PAGE_HEAD.set_link(&mut st.mem, None);
                st.shipout.page_tail = Some(PAGE_HEAD);
            }

            let last_box = st.scanner.disc_ptr[BoxCode::LastBox as usize];
            st.mem.flush_node_list(last_box)?;
            st.scanner.disc_ptr[BoxCode::LastBox as usize] = None;
            st.mem.pop_nest(&mut st.input)?;
            build_page(st)?;
        }
        GroupCode::Disc => build_discretionary(st)?,
        GroupCode::Align => {
            back_input(st)?;
            st.scanner.cur_tok = Token::from_cs(Frozen::Cr.cs());
            ins_error(
                st,
                ("Missing ", Esc("cr"), " inserted"),
                &["I'm guessing that you meant to end an alignment here."],
            )?;
        }
        GroupCode::NoAlign => {
            end_graf(st)?;
            unsave(st)?;
            align_peek(st)?;
        }
        GroupCode::Vcenter => {
            end_graf(st)?;
            unsave(st)?;
            let spec = st.save_stack.pop_spec()?;
            let head = st.input.cur_list.head.link(&st.mem);
            let p = vpackage(st, head, spec, MAX_DIMEN)?;
            st.mem.pop_nest(&mut st.input)?;
            let noad = st.mem.new_noad()?;
            st.input.cur_list.tail_append(&mut st.mem, noad);
            let node = &mut st.mem[noad];
            node.noad_type = NodeType::VcenterNoad.into();
            node.nucleus.math_type = SUB_BOX;
            st.mem[noad].nucleus.set_info(p.ptr());
        }
        GroupCode::MathChoice => build_choices(st)?,
        GroupCode::Math => {
            unsave(st)?;
            let Some(Saved::MathGroup(save_loc)) = st.save_stack.pop_saved() else {
                return Err(Abort::Confusion("math group"));
            };
            st.mem[save_loc].math_type = SUB_MLIST;
            let p = fin_mlist(st, None)?;
            st.mem[save_loc].set_info(p);
            let Some(p) = p.filter(|p| p.link(&st.mem).is_none()) else {
                return Ok(());
            };
            let Ok(AnyNode::Noad(noad)) = st.mem.classify(p) else {
                return Ok(());
            };
            match noad.variant(&st.mem) {
                NoadVariant::Noad(n)
                    if st.mem[n].noad_type == NodeType::OrdNoad
                        && st.mem[n].subscr.math_type == EMPTY
                        && st.mem[n].supscr.math_type == EMPTY =>
                {
                    st.mem[save_loc] = st.mem[n].nucleus;
                    n.free(&mut st.mem);
                }
                NoadVariant::Accent(a)
                    if matches!(save_loc, MathSlot::Nucleus(n) if n.idx() == st.input.cur_list.tail)
                        && matches!(
                            st.mem.classify(st.input.cur_list.tail),
                            Ok(AnyNode::Noad(t)) if st.mem[t].noad_type == NodeType::OrdNoad
                        ) =>
                {
                    let tail = st.input.cur_list.tail;
                    let q = st.mem.predecessor(st.input.cur_list.head, Some(tail))?;
                    q.set_link(&mut st.mem, a.ptr());
                    st.mem.free_node(tail, Noad::SIZE);
                    st.input.cur_list.tail = a.idx();
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub(super) fn append_glue(st: &mut EngineState, s: SkipCode) -> Result<NodePtr<GlueNode>, Abort> {
    let spec = match s {
        SkipCode::Fil => FIL_GLUE,
        SkipCode::Fill => FILL_GLUE,
        SkipCode::Ss => SS_GLUE,
        SkipCode::FilNeg => FIL_NEG_GLUE,
        SkipCode::Skip => scan_glue(st, ValueLevel::Glue)?,
        SkipCode::Mskip => scan_glue(st, ValueLevel::Mu)?,
    };
    let g = st.mem.new_glue(spec)?;
    st.input.cur_list.tail_append(&mut st.mem, g);
    if matches!(s, SkipCode::Skip | SkipCode::Mskip) {
        let spec = st.mem[g].glue_ptr;
        let refs = st.mem[spec].ref_count;
        st.mem[spec].ref_count = refs - 1;
        if s == SkipCode::Mskip {
            st.mem[g].subtype = GlueKind::Mu.into();
        }
    }
    Ok(g)
}

pub(super) fn append_kern(st: &mut EngineState, mu: bool) -> Result<(), Abort> {
    let s = if mu {
        KernSubtype::MuGlue
    } else {
        KernSubtype::Explicit
    };
    let d = scan_dimen(st, mu)?;
    let p = st.mem.new_kern(d)?;
    st.mem[p].subtype = s.into();
    st.input.cur_list.tail_append(&mut st.mem, p);
    Ok(())
}

pub(super) fn make_mark(st: &mut EngineState, marks: bool) -> Result<(), Abort> {
    let c = if marks {
        i32::from(scan_register_num(st)?)
    } else {
        0
    };
    scan_toks(st, false, true)?;
    let mark = NodePtr::<MarkNode>::alloc(&mut st.mem)?;
    let node = &mut st.mem[mark];
    node.mark_class = c;
    node.mark_ptr = st.scanner.def_ref;
    st.input.cur_list.tail_append(&mut st.mem, mark);
    Ok(())
}

impl ListStateRecord {
    pub(crate) fn fix_language(
        &mut self,
        mem: &mut MemState,
        eqtb: &EqtbState,
    ) -> Result<(), Abort> {
        let l = i32::from(norm_lang(eqtb.int_par(IntPar::Language)));
        if l == self.clang()? {
            return Ok(());
        }
        let w = self.append_whatsit::<LanguageNode>(mem, WhatsitSubtype::Language)?;
        self.set_clang(l)?;
        let language = &mut mem[w];
        language.lang = l;
        language.lhm = norm_min(eqtb.int_par(IntPar::LeftHyphenMin)) as u16;
        language.rhm = norm_min(eqtb.int_par(IntPar::RightHyphenMin)) as u16;
        Ok(())
    }
}

pub(crate) fn off_save(st: &mut EngineState) -> Result<(), Abort> {
    if st.save_stack.cur_group == GroupCode::BottomLevel {
        return error(
            st,
            ("Extra ", st.scanner.cur),
            &["Things are pretty mixed up, but I think the worst is over."],
        );
    }
    back_input(st)?;
    let insertion = Insertion(st.save_stack.cur_group.closer());
    let list = st.mem.tokens.begin()?;
    for token in insertion.tokens() {
        st.mem.tokens.push(&list, token)?;
    }
    let list = st.mem.tokens.finish(list)?;
    begin_token_list(st, list, TokenType::Inserted)?;
    error(
        st,
        ("Missing ", insertion, " inserted"),
        &[
            "I've inserted something that you may have forgotten.",
            "(See the <inserted text> above.)",
            "With luck, this will get me unwedged. But if you",
            "really didn't forget anything, try typing `2' now; then",
            "my insertion and my current dilemma will both disappear.",
        ],
    )
}

fn begin_disc_group(st: &mut EngineState) -> Result<(), Abort> {
    st.save_stack.new_save_level(GroupCode::Disc, st.mem.line)?;
    scan_left_brace(st)?;
    push_nest(st)?;
    st.input.cur_list.mode = Mode::horizontal(true);
    Ok(())
}

pub(super) fn append_discretionary(st: &mut EngineState, hyphen: bool) -> Result<(), Abort> {
    let d = st.mem.new_disc()?;
    st.input.cur_list.tail_append(&mut st.mem, d);
    if hyphen {
        let f = st.eqtb.cur_font();
        let c = st.fonts.hyphen_char[f];
        if (0..=BIGGEST_CHAR).contains(&c) {
            let ch = new_character(st, f, c as u16)?;
            st.mem[d].pre_break = ch.map(|c| c.addr());
        }
        return Ok(());
    }
    st.save_stack.push_saved(Saved::DiscPart(0));
    begin_disc_group(st)
}

fn build_discretionary(st: &mut EngineState) -> Result<(), Abort> {
    unsave(st)?;
    let head = st.input.cur_list.head;
    let mut q = head;
    let mut p = q.link(&st.mem);
    let mut n: i32 = 0;
    while let Some(here) = p {
        if !st.mem.is_char_node(here) {
            let allowed = match st.mem.classify(here) {
                Ok(
                    AnyNode::Box(_) | AnyNode::Rule(_) | AnyNode::Kern(_) | AnyNode::Ligature(_),
                ) => true,
                Ok(AnyNode::Whatsit(w)) => w
                    .whatsit_subtype(&st.mem)
                    .is_some_and(|s| s.is_native_word() || s == WhatsitSubtype::Glyph),
                _ => false,
            };
            if !allowed {
                error(
                    st,
                    "Improper discretionary list",
                    &["Discretionary lists must contain only boxes and kerns."],
                )?;
                begin_diagnostic(st);
                print_nl(st, "The following discretionary sublist has been deleted:");
                show_box(st, p);
                end_diagnostic(st, true);
                st.mem.flush_node_list(p)?;
                q.set_link(&mut st.mem, None);
                break;
            }
        }
        q = here;
        p = q.link(&st.mem);
        n += 1;
    }

    p = head.link(&st.mem);
    st.mem.pop_nest(&mut st.input)?;
    // SAFETY: `append_discretionary` left the disc node as the enclosing list's tail.
    let disc = NodePtr::<DiscNode>::from_idx_unchecked(st.input.cur_list.tail);
    let part = match st.save_stack.saved(-1) {
        Some(Saved::DiscPart(part)) => part,
        _ => return Err(Abort::Confusion("discretionary")),
    };
    match part {
        0 => st.mem[disc].pre_break = p,
        1 => st.mem[disc].post_break = p,
        2 => {
            if n > 0 && st.input.cur_list.mode.is_math() {
                st.mem.flush_node_list(p)?;
                n = 0;
                error(
                    st,
                    ("Illegal math ", Esc("discretionary")),
                    &[
                        "Sorry: The third part of a discretionary break must be",
                        "empty, in math formulas. I had to delete your third part.",
                    ],
                )?;
            } else {
                st.mem[disc].link = p;
            }
            if let Ok(count) = u16::try_from(n) {
                st.mem[disc].replace_count = count;
            } else {
                error(
                    st,
                    "Discretionary list is too long",
                    &[
                        "Wow---I never thought anybody would tweak me here.",
                        "You can't seriously need such a huge discretionary list?",
                    ],
                )?;
            }
            if n > 0 {
                st.input.cur_list.tail = q;
            }
            st.save_stack.save_ptr -= 1;
            return Ok(());
        }
        _ => {}
    }
    if let Some(Saved::DiscPart(part)) = st.save_stack.top_saved_mut() {
        *part += 1;
    }
    begin_disc_group(st)
}

fn char_italic(st: &mut EngineState, f: FontId, character: u16) -> Scaled {
    let c = st.fonts.effective_char(f, character);
    st.fonts.char_metrics(f, c).italic
}

pub(super) fn append_italic_correction(st: &mut EngineState) -> Result<(), Abort> {
    let tail = st.input.cur_list.tail;
    if tail == st.input.cur_list.head {
        return Ok(());
    }
    let italic = if let Some(ch) = st.mem.as_char(Some(tail)) {
        char_italic(st, st.mem[ch].font, st.mem[ch].character)
    } else {
        match st.mem.classify(tail) {
            Ok(AnyNode::Ligature(lig)) => char_italic(st, st.mem[lig].font, st.mem[lig].character),
            Ok(AnyNode::Whatsit(w)) => match w.variant(&st.mem) {
                Some(WhatsitVariant::NativeWord(word)) => {
                    st.fonts.get_native_italic_correction(&st.mem, word)
                }
                Some(WhatsitVariant::Glyph(glyph)) => {
                    st.fonts.get_native_glyph_italic_correction(&st.mem, glyph)
                }
                _ => return Ok(()),
            },
            _ => return Ok(()),
        }
    };
    let kern = st.mem.new_kern(italic)?;
    st.input.cur_list.tail_append(&mut st.mem, kern);
    st.mem[kern].subtype = KernSubtype::Explicit.into();
    Ok(())
}

pub(super) fn make_accent(st: &mut EngineState) -> Result<(), Abort> {
    let c = scan_char_num(st)?;
    let f = st.eqtb.cur_font();
    let Some(p) = new_character(st, f, c as u16)? else {
        return Ok(());
    };
    let x = st.fonts.param(f, X_HEIGHT_CODE);
    let s = f64::from(st.fonts.param(f, SLANT_CODE)) / 65536.0;
    let mut lsb = Scaled::ZERO;
    let mut rsb = Scaled::ZERO;
    let a = match p {
        NewChar::Native(w) => {
            let a = st.mem[w].width;
            if a == Scaled::ZERO {
                (lsb, rsb) = st.fonts.get_native_char_sidebearings(f, c)?;
            }
            a
        }
        NewChar::Char(ch) => {
            let c = st.fonts.effective_char(f, st.mem[ch].character);
            st.fonts.char_metrics(f, c).width
        }
    };

    do_assignments(st)?;
    let f = st.eqtb.cur_font();
    let (q, qc) = match st.scanner.cur {
        Command::Letter(c) | Command::OtherChar(c) | Command::CharGiven(c) => {
            let qc = c.raw();
            (new_character(st, f, qc as u16)?, qc)
        }
        Command::CharNum => {
            let qc = scan_char_num(st)?;
            (new_character(st, f, qc as u16)?, qc)
        }
        _ => {
            back_input(st)?;
            (None, 0)
        }
    };
    let Some(q) = q else {
        st.input.cur_list.tail_append(&mut st.mem, p);
        return st.input.cur_list.set_space_factor(1000);
    };

    let t = f64::from(st.fonts.param(f, SLANT_CODE)) / 65536.0;
    let (w, h) = match q {
        NewChar::Native(w) => {
            let w = st.mem[w].width;
            let (h, _) = st.fonts.get_native_char_height_depth(f, qc)?;
            (w, h)
        }
        NewChar::Char(ch) => {
            let c = st.fonts.effective_char(f, st.mem[ch].character);
            let info = st.fonts.char_metrics(f, c);
            (info.width, info.height)
        }
    };
    let accent = if h == x {
        p.addr()
    } else {
        let b = hpack(st, p.ptr(), PackSpec::NATURAL)?;
        st.mem[b].shift_amount = x - h;
        b.idx()
    };
    let delta = if st.fonts.is_native_font(f) && a == Scaled::ZERO {
        f64::from(w - lsb + rsb) / 2.0
    } else {
        f64::from(w - a) / 2.0
    };
    let delta = Scaled::round(delta + f64::from(h) * t - f64::from(x) * s);
    let r = st.mem.new_kern(delta)?;
    st.mem[r].subtype = KernSubtype::AccKern.into();
    st.input.cur_list.tail.set_link(&mut st.mem, r.ptr());
    st.mem[r].link = Some(accent);
    let back = st.mem.new_kern(-a - delta)?;
    st.input.cur_list.tail = back.idx();
    st.mem[back].subtype = KernSubtype::AccKern.into();
    accent.set_link(&mut st.mem, back.ptr());
    st.input.cur_list.tail_append(&mut st.mem, q);
    st.input.cur_list.set_space_factor(1000)
}
