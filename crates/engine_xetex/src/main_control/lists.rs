//! Building lists: paragraphs, glue, kerns, inserts, marks, discretionaries, accents.

use crate::cmd::Cmd;
use crate::codes::{BoxCode, SkipCode, ValueLevel};
use crate::command::Command;
use crate::consts::{
    BIGGEST_CHAR, BOX_NODE_SIZE, EMPTY, ETEX_PEN_BASE, FROZEN_CR, FROZEN_END_GROUP, FROZEN_RIGHT,
    HMODE, IGNORE_DEPTH, MAX_DIMEN, MMODE, MU_GLUE, NOAD_SIZE, NULL_FLAG, PAR_SHAPE_LOC, SUB_BOX,
    SUB_MLIST, TEX_NULL, VMODE,
};
use crate::errors::Abort;
use crate::fonts::{
    EXTRA_SPACE_CODE, SLANT_CODE, SPACE_CODE, SPACE_SHRINK_CODE, SPACE_STRETCH_CODE, X_HEIGHT_CODE,
};
use crate::input::TokenType;
use crate::mem::{
    ADJUST_HEAD, CONTRIB_HEAD, FILL_GLUE, FIL_GLUE, FIL_NEG_GLUE, PAGE_HEAD, PRE_ADJUST_HEAD,
    SS_GLUE, TEMP_HEAD_TOKS, ZERO_GLUE,
};
use crate::mem_types::ListStateRecord;
use crate::node::{
    AdjustNode, AdjustPlacement, AnyNode, CharNode, DiscNode, GlueNode, GlueSpecNode, InsNode,
    KernSubtype, LanguageNode, MarkNode, MathField, NativeWordNode, NoadVariant, NodePtr,
    WhatsitSubtype, WhatsitVariant,
};
use crate::packaging::PackSpec;
use crate::params::{DimenPar, GluePar, IntPar, TokPar};
use crate::state::{EngineState, GroupCode, MemState};
use crate::token::Token;
use crate::units::{FontId, MemIdx, MemPtr, NodeType, Scaled};

use super::align::align_peek;
use super::assign::{define, do_assignments, word_define};
use super::boxes::package;
use super::{begin_every, INNER_HMODE, INNER_VMODE};

pub(crate) fn push_math(st: &mut EngineState, c: GroupCode) -> Result<(), Abort> {
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = -MMODE;
    st.input.cur_list.set_incompleat_noad(None);
    st.save_stack.new_save_level(c, st.mem.line)?;
    Ok(())
}

pub(super) fn end_graf(st: &mut EngineState) -> Result<(), Abort> {
    if st.input.cur_list.mode != HMODE {
        return Ok(());
    }
    if st.input.cur_list.head == st.input.cur_list.tail {
        st.mem.pop_nest(&mut st.input)?;
    } else {
        crate::linebreak::line_break(st, false)?;
    }
    let lr_save = st.input.cur_list.e_tex_aux;
    if !lr_save.is_null() {
        st.mem.flush_list(lr_save);
        st.input.cur_list.e_tex_aux = MemPtr::NULL;
    }
    normal_paragraph(st)?;
    st.errors.error_count = 0;
    Ok(())
}

pub(super) fn append_penalty(st: &mut EngineState) -> Result<(), Abort> {
    let n = crate::scan::scan_int(st)?;
    let p = st.mem.new_penalty(n)?;
    st.input.cur_list.tail_append(&mut st.mem, p.ptr());
    if st.input.cur_list.mode == VMODE {
        crate::pagebuilder::build_page(st)?;
    }
    Ok(())
}

pub(crate) fn normal_paragraph(st: &mut EngineState) -> Result<(), Abort> {
    for (loc, value) in [
        (IntPar::Looseness.loc(), 0),
        (DimenPar::HangIndent.loc(), 0),
        (IntPar::HangAfter.loc(), 1),
    ] {
        if st.eqtb.int_at(loc) != value {
            word_define(st, false, loc, value)?;
        }
    }
    for loc in [PAR_SHAPE_LOC, ETEX_PEN_BASE] {
        if st.eqtb.equiv(loc) != TEX_NULL {
            define(st, false, loc, Cmd::ShapeRef, TEX_NULL)?;
        }
    }
    Ok(())
}

pub(super) fn its_all_over(st: &mut EngineState) -> Result<bool, Abort> {
    if !crate::errors::privileged(st)? {
        return Ok(false);
    }
    if st.shipout.page_tail == PAGE_HEAD
        && st.input.cur_list.head == st.input.cur_list.tail
        && st.shipout.dead_cycles == 0
    {
        return Ok(true);
    }
    crate::input::back_input(st)?;
    let b = st.mem.new_null_box()?;
    st.input.cur_list.tail_append(&mut st.mem, b.ptr());
    st.mem[b].width = st.eqtb.dimen_par(DimenPar::Hsize);
    let g = st.mem.new_glue(FILL_GLUE)?;
    st.input.cur_list.tail_append(&mut st.mem, g.ptr());
    let p = st.mem.new_penalty(NULL_FLAG)?;
    st.input.cur_list.tail_append(&mut st.mem, p.ptr());
    crate::pagebuilder::build_page(st)?;
    Ok(false)
}

pub(super) fn indent_in_hmode(st: &mut EngineState, indent: bool) -> Result<(), Abort> {
    if !indent {
        return Ok(());
    }
    let b = st.mem.new_null_box()?;
    st.mem[b].width = st.eqtb.dimen_par(DimenPar::ParIndent);
    let p = if st.input.cur_list.mode.abs() == HMODE {
        st.input.cur_list.set_space_factor(1000);
        b.ptr()
    } else {
        let q = st.mem.new_noad()?;
        st.mem[q.nucleus()].math_type = SUB_BOX;
        st.mem[q.nucleus()].set_info(b.ptr());
        q.ptr()
    };
    st.input.cur_list.tail_append(&mut st.mem, p);
    Ok(())
}

pub(super) fn head_for_vmode(st: &mut EngineState) -> Result<(), Abort> {
    if st.input.cur_list.mode >= 0 {
        crate::input::back_input(st)?;
        st.scanner.cur_tok = st.input.par_token;
        crate::input::back_input(st)?;
        st.input.cur_input.set_token_type(TokenType::Inserted);
        return Ok(());
    }
    if st.scanner.cur_cmd != Cmd::Hrule {
        return off_save(st);
    }
    crate::output::error_here_with_diagnostic(st, "You can't use `");
    crate::output::print_esc(st, "hrule");
    crate::output::print(st, "' here except with leaders");
    crate::output::capture_to_diagnostic(st, None);
    st.errors.help(&[
        "To put a horizontal rule in an hbox or an alignment,",
        r"you should use \leaders or \hrulefill (see The TeXbook).",
    ]);
    crate::errors::error(st)
}

pub(super) fn begin_insert_or_adjust(st: &mut EngineState) -> Result<(), Abort> {
    let n = if st.scanner.cur_cmd == Cmd::Vadjust {
        255
    } else {
        match crate::scan::scan_eight_bit_int(st)? {
            255 => {
                crate::output::error_here_with_diagnostic(st, "You can't ");
                crate::output::print_esc(st, "insert");
                crate::output::print_int(st, 255);
                crate::output::capture_to_diagnostic(st, None);
                st.errors
                    .help(&[r"I'm changing to \insert0; box 255 is special."]);
                crate::errors::error(st)?;
                0
            }
            n => n,
        }
    };
    st.save_stack.set_saved(0, n);
    let pre = st.scanner.cur_cmd == Cmd::Vadjust && crate::scan::scan_keyword(st, "pre")?;
    st.save_stack.set_saved(1, i32::from(pre));
    st.save_stack.save_ptr += 2;
    st.save_stack
        .new_save_level(GroupCode::Insert, st.mem.line)?;
    crate::scan::scan_left_brace(st)?;
    normal_paragraph(st)?;
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = INNER_VMODE;
    st.input.cur_list.set_prev_depth(IGNORE_DEPTH);
    Ok(())
}

/// Appends interword glue for the current space factor.
pub(super) fn app_space(st: &mut EngineState) -> Result<(), Abort> {
    let space_factor = st.input.cur_list.space_factor();
    let q;
    let mut side_effects = None;
    if space_factor >= 2000 && st.eqtb.glue_par(GluePar::XspaceSkip) != ZERO_GLUE {
        q = st.mem.new_param_glue(&st.eqtb, GluePar::XspaceSkip)?.ptr();
    } else {
        let space_skip = st.eqtb.glue_par(GluePar::SpaceSkip);
        let base = if space_skip != ZERO_GLUE {
            space_skip
        } else {
            font_space_glue(st)?
        };
        st.mainloop.main_p = base.ptr();
        let spec = st.mem.new_spec(base)?;
        st.mainloop.main_p = spec.ptr();
        if space_factor >= 2000 {
            let cf = st.eqtb.cur_font();
            let extra = st.fonts.param(cf, EXTRA_SPACE_CODE);
            let width = st.mem[spec].width + extra;
            st.mem[spec].width = width;
        }
        let (stretch, _, stretch_overflow) =
            crate::scaled_math::xn_over_d_parts(st.mem[spec].stretch, space_factor, 1000);
        st.mem[spec].stretch = stretch;
        let (shrink, remainder, shrink_overflow) =
            crate::scaled_math::xn_over_d_parts(st.mem[spec].shrink, 1000, space_factor);
        st.mem[spec].shrink = shrink;
        side_effects = Some((remainder, stretch_overflow || shrink_overflow));
        q = st.mem.new_glue(spec)?.ptr();
        st.mem[spec].ref_count = TEX_NULL;
    }
    st.input.cur_list.tail_append(&mut st.mem, q);
    if let Some((remainder, overflowed)) = side_effects {
        if overflowed {
            st.errors.arith_error = true;
        }
        st.errors.tex_remainder = remainder;
    }
    Ok(())
}

/// The current font's interword glue spec, built from its parameters on first use.
pub(super) fn font_space_glue(st: &mut EngineState) -> Result<NodePtr<GlueSpecNode>, Abort> {
    let cf = st.eqtb.cur_font();
    if let Some(glue) = st.fonts.font_glue[cf] {
        return Ok(glue);
    }
    let spec = st.mem.new_spec(ZERO_GLUE)?;
    st.mem[spec].width = st.fonts.param(cf, SPACE_CODE);
    st.mem[spec].stretch = st.fonts.param(cf, SPACE_STRETCH_CODE);
    st.mem[spec].shrink = st.fonts.param(cf, SPACE_SHRINK_CODE);
    st.fonts.font_glue[cf] = Some(spec);
    Ok(spec)
}

/// Sets `cur_lang` and the list's language and hyphen minima for a new paragraph.
pub(crate) fn start_paragraph_language(st: &mut EngineState) {
    let l = crate::helpers::norm_lang(st.eqtb.int_par(IntPar::Language));
    st.hyph.cur_lang = l;
    st.input.cur_list.set_clang(i32::from(l));
    let lhm = i32::from(crate::helpers::norm_min(
        st.eqtb.int_par(IntPar::LeftHyphenMin),
    ));
    let rhm = i32::from(crate::helpers::norm_min(
        st.eqtb.int_par(IntPar::RightHyphenMin),
    ));
    st.input.cur_list.prev_graf = (lhm * 64 + rhm) * 65536 + i32::from(l);
}

pub(crate) fn new_graf(st: &mut EngineState, indented: bool) -> Result<(), Abort> {
    st.input.cur_list.prev_graf = 0;
    if st.input.cur_list.mode == VMODE || st.input.cur_list.head != st.input.cur_list.tail {
        let g = st.mem.new_param_glue(&st.eqtb, GluePar::ParSkip)?;
        st.input.cur_list.tail_append(&mut st.mem, g.ptr());
    }
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = HMODE;
    st.input.cur_list.set_space_factor(1000);

    start_paragraph_language(st);

    if indented {
        let b = st.mem.new_null_box()?;
        st.input.cur_list.tail = b.ptr();
        st.input.cur_list.head.set_link(&mut st.mem, b.ptr());
        st.mem[b].width = st.eqtb.dimen_par(DimenPar::ParIndent);
        if st.shipout.insert_src_special_every_par {
            crate::ext::insert_src_special(st)?;
        }
    }
    begin_every(st, TokPar::EveryPar, TokenType::EveryPar)?;
    if st.input.nest.len() == 1 {
        crate::pagebuilder::build_page(st)?;
    }
    Ok(())
}

/// What `new_character` makes: a character node, or a glyph for a native font.
#[derive(Copy, Clone)]
pub(crate) enum NewChar {
    Char(NodePtr<CharNode>),
    Native(NodePtr<NativeWordNode>),
}

impl NewChar {
    pub(crate) fn ptr(self) -> MemPtr {
        match self {
            NewChar::Char(c) => c.ptr(),
            NewChar::Native(w) => w.ptr(),
        }
    }
}

/// TeX's `new_character`, `None` for a character the font lacks.
pub(crate) fn new_character(
    st: &mut EngineState,
    f: FontId,
    c: u16,
) -> Result<Option<NewChar>, Abort> {
    if st.fonts.is_native_font(f) {
        return crate::fonts::new_native_character(st, f, u32::from(c))
            .map(|w| Some(NewChar::Native(w)));
    }
    let ec = st.fonts.effective_char(f, c) as u16;
    if st.fonts.char_exists(f, i32::from(ec)) {
        let p = st.mem.new_char(f, c)?;
        return Ok(Some(NewChar::Char(p)));
    }
    crate::output::char_warning(st, f, i32::from(c))?;
    Ok(None)
}

pub(super) fn handle_right_brace(st: &mut EngineState) -> Result<(), Abort> {
    match st.save_stack.cur_group {
        GroupCode::Simple => crate::save::unsave(st)?,
        GroupCode::BottomLevel => {
            crate::output::print_err(st, "Too many }'s");
            st.errors.help(&[
                "You've closed more groups than you opened.",
                "Such booboos are generally harmless, so keep going.",
            ]);
            crate::errors::error(st)?;
        }
        GroupCode::SemiSimple | GroupCode::MathShift | GroupCode::MathLeft => {
            crate::errors::extra_right_brace(st)?;
        }
        GroupCode::Hbox => package(st, false)?,
        GroupCode::AdjustedHbox => {
            st.scanner.adjust_tail = ADJUST_HEAD;
            st.scanner.pre_adjust_tail = PRE_ADJUST_HEAD;
            package(st, false)?;
        }
        GroupCode::Vbox => {
            end_graf(st)?;
            package(st, false)?;
        }
        GroupCode::Vtop => {
            end_graf(st)?;
            package(st, true)?;
        }
        GroupCode::Insert => {
            end_graf(st)?;
            let split_top_skip = st.eqtb.glue_par(GluePar::SplitTopSkip);
            st.mem.add_glue_ref(split_top_skip);
            let d = st.eqtb.dimen_par(DimenPar::SplitMaxDepth);
            let f = st.eqtb.int_par(IntPar::FloatingPenalty);
            crate::save::unsave(st)?;
            st.save_stack.save_ptr -= 2;
            let head = st.input.cur_list.head.link(&st.mem);
            let p = crate::packaging::vpackage(st, head, PackSpec::NATURAL, MAX_DIMEN)?;
            st.mem.pop_nest(&mut st.input)?;
            let n = st.save_stack.saved(0);
            if n < 255 {
                let ins = NodePtr::<InsNode>::alloc(&mut st.mem)?;
                st.input.cur_list.tail_append(&mut st.mem, ins.ptr());
                st.mem[ins].box_number =
                    u16::try_from(n).map_err(|_| Abort::Confusion("insert"))?;
                let height = st.mem[p].height + st.mem[p].depth;
                st.mem[ins].height = height;
                let list = st.mem[p].list_ptr;
                st.mem[ins].ins_ptr = list;
                st.mem[ins].split_top_ptr = split_top_skip;
                st.mem[ins].depth = d;
                st.mem[ins].float_cost = f;
            } else {
                let adj = NodePtr::<AdjustNode>::alloc(&mut st.mem)?;
                st.input.cur_list.tail_append(&mut st.mem, adj.ptr());
                st.mem[adj].placement = if st.save_stack.saved(1) != 0 {
                    AdjustPlacement::Pre
                } else {
                    AdjustPlacement::Post
                }
                .into();
                let list = st.mem[p].list_ptr;
                st.mem[adj].adjust_ptr = list;
                st.mem.delete_glue_ref(split_top_skip);
            }
            st.mem.free_node(p.ptr(), BOX_NODE_SIZE);
            if st.input.nest.is_empty() {
                crate::pagebuilder::build_page(st)?;
            }
        }
        GroupCode::Output => {
            if !st.input.cur_input.token_loc().is_null()
                || (st.input.cur_input.token_type() != TokenType::Output
                    && st.input.cur_input.token_type() != TokenType::BackedUp)
            {
                crate::output::print_err(st, "Unbalanced output routine");
                st.errors.help(&[
                    "Your sneaky output routine has problematic {'s and/or }'s.",
                    "I can't handle that very well; good luck.",
                ]);
                crate::errors::error(st)?;
                loop {
                    crate::scan::get_token(st)?;
                    if st.input.cur_input.token_loc().is_null() {
                        break;
                    }
                }
            }
            crate::input::end_token_list(st)?;
            end_graf(st)?;
            crate::save::unsave(st)?;
            st.shipout.output_active = false;
            st.shipout.insert_penalties = 0;

            if !st.eqtb.box_reg(255).is_null() {
                crate::output::error_here_with_diagnostic(st, "Output routine didn't use all of ");
                crate::output::print_esc(st, "box");
                crate::output::print_int(st, 255);
                crate::output::capture_to_diagnostic(st, None);
                st.errors.help(&[
                    r"Your \output commands should empty \box255,",
                    r"e.g., by saying `\shipout\box255'.",
                    "Proceed; I'll discard its present contents.",
                ]);
                crate::errors::box_error(st, 255)?;
            }

            if st.input.cur_list.tail != st.input.cur_list.head {
                let first = st.input.cur_list.head.link(&st.mem);
                st.shipout.page_tail.set_link(&mut st.mem, first);
                st.shipout.page_tail = st.input.cur_list.tail;
            }
            if !PAGE_HEAD.link(&st.mem).is_null() {
                if CONTRIB_HEAD.link(&st.mem).is_null() {
                    st.input.nest[0].tail = st.shipout.page_tail;
                }
                let contrib = CONTRIB_HEAD.link(&st.mem);
                st.shipout.page_tail.set_link(&mut st.mem, contrib);
                let page = PAGE_HEAD.link(&st.mem);
                CONTRIB_HEAD.set_link(&mut st.mem, page);
                PAGE_HEAD.set_link(&mut st.mem, MemPtr::NULL);
                st.shipout.page_tail = PAGE_HEAD;
            }

            let last_box = st.scanner.disc_ptr[BoxCode::LastBox as usize];
            st.mem.flush_node_list(last_box)?;
            st.scanner.disc_ptr[BoxCode::LastBox as usize] = MemPtr::NULL;
            st.mem.pop_nest(&mut st.input)?;
            crate::pagebuilder::build_page(st)?;
        }
        GroupCode::Disc => build_discretionary(st)?,
        GroupCode::Align => {
            crate::input::back_input(st)?;
            st.scanner.cur_tok = Token::from_cs(FROZEN_CR);
            crate::output::error_here_with_diagnostic(st, "Missing ");
            crate::output::print_esc(st, "cr");
            crate::output::print(st, " inserted");
            crate::output::capture_to_diagnostic(st, None);
            st.errors
                .help(&["I'm guessing that you meant to end an alignment here."]);
            crate::input::ins_error(st)?;
        }
        GroupCode::NoAlign => {
            end_graf(st)?;
            crate::save::unsave(st)?;
            align_peek(st)?;
        }
        GroupCode::Vcenter => {
            end_graf(st)?;
            crate::save::unsave(st)?;
            let spec = st.save_stack.pop_spec()?;
            let head = st.input.cur_list.head.link(&st.mem);
            let p = crate::packaging::vpackage(st, head, spec, MAX_DIMEN)?;
            st.mem.pop_nest(&mut st.input)?;
            let noad = st.mem.new_noad()?;
            st.input.cur_list.tail_append(&mut st.mem, noad.ptr());
            st.mem[noad].noad_type = NodeType::VcenterNoad.into();
            st.mem[noad.nucleus()].math_type = SUB_BOX;
            st.mem[noad.nucleus()].set_info(p.ptr());
        }
        GroupCode::MathChoice => crate::math::build_choices(st)?,
        GroupCode::Math => {
            crate::save::unsave(st)?;
            st.save_stack.save_ptr -= 1;
            let save_loc = NodePtr::<MathField>::at(MemPtr::from_raw(st.save_stack.saved(0)));
            st.mem[save_loc].math_type = SUB_MLIST;
            let p = crate::math::fin_mlist(st, MemPtr::NULL)?;
            st.mem[save_loc].set_info(p);
            if p.is_null() || !p.link(&st.mem).is_null() {
                return Ok(());
            }
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
                    st.mem.free_node(n.ptr(), NOAD_SIZE);
                }
                NoadVariant::Accent(a)
                    if save_loc.idx().raw() == st.input.cur_list.tail.raw() + 1
                        && matches!(
                            st.mem.classify(st.input.cur_list.tail),
                            Ok(AnyNode::Noad(t)) if st.mem[t].noad_type == NodeType::OrdNoad
                        ) =>
                {
                    let mut q = st.input.cur_list.head;
                    while q.link(&st.mem) != st.input.cur_list.tail {
                        q = q.link(&st.mem);
                    }
                    q.set_link(&mut st.mem, a.ptr());
                    st.mem.free_node(st.input.cur_list.tail, NOAD_SIZE);
                    st.input.cur_list.tail = a.ptr();
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
        SkipCode::Skip => crate::scan::scan_glue(st, ValueLevel::Glue)?,
        SkipCode::Mskip => crate::scan::scan_glue(st, ValueLevel::Mu)?,
    };
    let g = st.mem.new_glue(spec)?;
    st.input.cur_list.tail_append(&mut st.mem, g.ptr());
    if matches!(s, SkipCode::Skip | SkipCode::Mskip) {
        let spec = st.mem[g].glue_ptr;
        let refs = st.mem[spec].ref_count;
        st.mem[spec].ref_count = refs - 1;
        if s == SkipCode::Mskip {
            st.mem[g].subtype = MU_GLUE;
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
    let d = crate::scan::scan_dimen(st, mu, false)?;
    let p = st.mem.new_kern(d)?;
    st.mem[p].subtype = s.into();
    st.input.cur_list.tail_append(&mut st.mem, p.ptr());
    Ok(())
}

pub(super) fn make_mark(st: &mut EngineState, marks: bool) -> Result<(), Abort> {
    let c = if marks {
        crate::scan::scan_register_num(st)?
    } else {
        0
    };
    crate::scan::scan_toks(st, false, true)?;
    let mark = NodePtr::<MarkNode>::alloc(&mut st.mem)?;
    st.mem[mark].mark_class = c;
    st.mem[mark].subtype = 0;
    st.mem[mark].mark_ptr = st.scanner.def_ref;
    st.input.cur_list.tail_append(&mut st.mem, mark.ptr());
    Ok(())
}

impl ListStateRecord {
    pub(crate) fn fix_language(
        &mut self,
        mem: &mut MemState,
        eqtb: &crate::state::EqtbState,
    ) -> Result<(), Abort> {
        let l = i32::from(crate::helpers::norm_lang(eqtb.int_par(IntPar::Language)));
        if l == self.clang() {
            return Ok(());
        }
        let w = self.append_whatsit::<LanguageNode>(mem, WhatsitSubtype::Language)?;
        self.set_clang(l);
        let language = &mut mem[w];
        language.lang = l;
        language.lhm = crate::helpers::norm_min(eqtb.int_par(IntPar::LeftHyphenMin)) as u16;
        language.rhm = crate::helpers::norm_min(eqtb.int_par(IntPar::RightHyphenMin)) as u16;
        Ok(())
    }
}

pub(crate) fn off_save(st: &mut EngineState) -> Result<(), Abort> {
    if st.save_stack.cur_group == GroupCode::BottomLevel {
        crate::output::error_here_with_diagnostic(st, "Extra ");
        crate::output::print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
        crate::output::capture_to_diagnostic(st, None);
        st.errors
            .help(&["Things are pretty mixed up, but I think the worst is over."]);
        return crate::errors::error(st);
    }
    crate::input::back_input(st)?;
    let p = st.mem.get_avail_token()?;
    TEMP_HEAD_TOKS.set_link(&mut st.mem, p);
    crate::output::error_here_with_diagnostic(st, "Missing ");
    match st.save_stack.cur_group {
        GroupCode::SemiSimple => {
            p.set_token(&mut st.mem, Token::from_cs(FROZEN_END_GROUP));
            crate::output::print_esc(st, "endgroup");
        }
        GroupCode::MathShift => {
            p.set_token(&mut st.mem, Token::math_shift(b'$'));
            crate::output::print_char(st, b'$');
        }
        GroupCode::MathLeft => {
            p.set_token(&mut st.mem, Token::from_cs(FROZEN_RIGHT));
            let q = st.mem.get_avail_token()?;
            p.set_link(&mut st.mem, q);
            q.set_token(&mut st.mem, Token::other(b'.'));
            crate::output::print_esc(st, "right.");
        }
        _ => {
            p.set_token(&mut st.mem, Token::right_brace(b'}'));
            crate::output::print_char(st, b'}');
        }
    }
    crate::output::print(st, " inserted");
    crate::input::begin_token_list(st, TEMP_HEAD_TOKS.link(&st.mem), TokenType::Inserted)?;
    crate::output::capture_to_diagnostic(st, None);
    st.errors.help(&[
        "I've inserted something that you may have forgotten.",
        "(See the <inserted text> above.)",
        "With luck, this will get me unwedged. But if you",
        "really didn't forget anything, try typing `2' now; then",
        "my insertion and my current dilemma will both disappear.",
    ]);
    crate::errors::error(st)
}

fn begin_disc_group(st: &mut EngineState) -> Result<(), Abort> {
    st.save_stack.new_save_level(GroupCode::Disc, st.mem.line)?;
    crate::scan::scan_left_brace(st)?;
    crate::mem::push_nest(st)?;
    st.input.cur_list.mode = INNER_HMODE;
    st.input.cur_list.set_space_factor(1000);
    Ok(())
}

pub(super) fn append_discretionary(st: &mut EngineState, hyphen: bool) -> Result<(), Abort> {
    let d = st.mem.new_disc()?;
    st.input.cur_list.tail_append(&mut st.mem, d.ptr());
    if hyphen {
        let f = st.eqtb.cur_font();
        let c = st.fonts.hyphen_char[f];
        if (0..=BIGGEST_CHAR).contains(&c) {
            let ch = new_character(st, f, c as u16)?;
            st.mem[d].pre_break = ch.map_or(MemPtr::NULL, NewChar::ptr);
        }
        return Ok(());
    }
    st.save_stack.set_saved(0, 0);
    st.save_stack.save_ptr += 1;
    begin_disc_group(st)
}

fn build_discretionary(st: &mut EngineState) -> Result<(), Abort> {
    crate::save::unsave(st)?;
    let head = st.input.cur_list.head;
    let mut q = head;
    let mut p = q.link(&st.mem);
    let mut n: i32 = 0;
    while !p.is_null() {
        if !st.mem.is_char_node(p) {
            let allowed = match st.mem.classify(p) {
                Ok(
                    AnyNode::Box(_) | AnyNode::Rule(_) | AnyNode::Kern(_) | AnyNode::Ligature(_),
                ) => true,
                Ok(AnyNode::Whatsit(w)) => w
                    .whatsit_subtype(&st.mem)
                    .is_some_and(|s| s.is_native_word() || s == WhatsitSubtype::Glyph),
                _ => false,
            };
            if !allowed {
                crate::output::print_err(st, "Improper discretionary list");
                st.errors
                    .help(&["Discretionary lists must contain only boxes and kerns."]);
                crate::errors::error(st)?;
                crate::output::begin_diagnostic(st);
                crate::output::diagnostic_begin_capture_warning_here(st);
                crate::output::print_nl(
                    st,
                    "The following discretionary sublist has been deleted:",
                );
                crate::output::show_box(st, p);
                crate::output::capture_to_diagnostic(st, None);
                crate::output::end_diagnostic(st, true);
                st.mem.flush_node_list(p)?;
                q.set_link(&mut st.mem, MemPtr::NULL);
                break;
            }
        }
        q = p;
        p = q.link(&st.mem);
        n += 1;
    }

    p = head.link(&st.mem);
    st.mem.pop_nest(&mut st.input)?;
    // SAFETY: `append_discretionary` left the disc node as the enclosing list's tail.
    let disc = NodePtr::<DiscNode>::from_idx_unchecked(MemIdx::new(st.input.cur_list.tail));
    let part = st.save_stack.saved(-1);
    match part {
        0 => st.mem[disc].pre_break = p,
        1 => st.mem[disc].post_break = p,
        2 => {
            if n > 0 && st.input.cur_list.mode.abs() == MMODE {
                crate::output::error_here_with_diagnostic(st, "Illegal math ");
                crate::output::print_esc(st, "discretionary");
                crate::output::capture_to_diagnostic(st, None);
                st.errors.help(&[
                    "Sorry: The third part of a discretionary break must be",
                    "empty, in math formulas. I had to delete your third part.",
                ]);
                st.mem.flush_node_list(p)?;
                n = 0;
                crate::errors::error(st)?;
            } else {
                st.mem[disc].link = p;
            }
            if let Ok(count) = u16::try_from(n) {
                st.mem[disc].replace_count = count;
            } else {
                crate::output::print_err(st, "Discretionary list is too long");
                st.errors.help(&[
                    "Wow---I never thought anybody would tweak me here.",
                    "You can't seriously need such a huge discretionary list?",
                ]);
                crate::errors::error(st)?;
            }
            if n > 0 {
                st.input.cur_list.tail = q;
            }
            st.save_stack.save_ptr -= 1;
            return Ok(());
        }
        _ => {}
    }
    st.save_stack.set_saved(-1, part + 1);
    begin_disc_group(st)
}

/// The italic correction of character `ch`.
fn char_italic(st: &mut EngineState, ch: NodePtr<CharNode>) -> Scaled {
    let f = st.mem[ch].font;
    let c = st.fonts.effective_char(f, st.mem[ch].character);
    let info = st.fonts.char_info(f, c);
    st.fonts.char_italic(f, info)
}

pub(super) fn append_italic_correction(st: &mut EngineState) -> Result<(), Abort> {
    let tail = st.input.cur_list.tail;
    if tail == st.input.cur_list.head {
        return Ok(());
    }
    let italic = if let Some(ch) = st.mem.as_char(tail) {
        char_italic(st, ch)
    } else {
        match st.mem.classify(tail) {
            Ok(AnyNode::Ligature(lig)) => char_italic(st, lig.char_word()),
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
    st.input.cur_list.tail_append(&mut st.mem, kern.ptr());
    st.mem[kern].subtype = KernSubtype::Explicit.into();
    Ok(())
}

pub(super) fn make_accent(st: &mut EngineState) -> Result<(), Abort> {
    let c = crate::scan::scan_char_num(st)?;
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
            let info = st.fonts.char_info(f, c);
            st.fonts.char_width(f, info)
        }
    };

    do_assignments(st)?;
    let f = st.eqtb.cur_font();
    let (q, qc) = match st.scanner.try_command() {
        Some(Command::Letter(c) | Command::OtherChar(c) | Command::CharGiven(c)) => {
            let qc = c.raw();
            (new_character(st, f, qc as u16)?, qc)
        }
        Some(Command::CharNum) => {
            let qc = crate::scan::scan_char_num(st)?;
            (new_character(st, f, qc as u16)?, qc)
        }
        _ => {
            crate::input::back_input(st)?;
            (None, 0)
        }
    };
    let Some(q) = q else {
        st.input.cur_list.tail_append(&mut st.mem, p.ptr());
        st.input.cur_list.set_space_factor(1000);
        return Ok(());
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
            let info = st.fonts.char_info(f, c);
            let w = st.fonts.char_width(f, info);
            let h = st.fonts.char_height(f, info);
            (w, h)
        }
    };
    let accent = if h == x {
        p.ptr()
    } else {
        let b = crate::packaging::hpack(st, p.ptr(), PackSpec::NATURAL)?;
        st.mem[b].shift_amount = x - h;
        b.ptr()
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
    st.mem[r].link = accent;
    let back = st.mem.new_kern(-a - delta)?;
    st.input.cur_list.tail = back.ptr();
    st.mem[back].subtype = KernSubtype::AccKern.into();
    accent.set_link(&mut st.mem, back.ptr());
    st.input.cur_list.tail_append(&mut st.mem, q.ptr());
    st.input.cur_list.set_space_factor(1000);
    Ok(())
}
