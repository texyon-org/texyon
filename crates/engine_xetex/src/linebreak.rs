use crate::consts::{
    DIMEN_BASE, ETEX_PEN_BASE, GLUE_BASE, INT_BASE, LC_CODE_BASE, LOCAL_BASE, MAX_HALFWORD,
    SMALL_NODE_SIZE, TEX_NULL, TOO_BIG_CHAR,
};
use crate::ext::{
    linebreak_next_global, linebreak_start, measure_native_node, measure_native_node_global,
    real_get_native_word_cp,
};
use crate::globals::eqtb_b32s1;
use crate::hyphenation::init_trie;
use crate::node::{
    ActiveNode, AnyBox, DeltaNode, DiscNode, GlueNode, GlueSpecNode, KernNode, LigatureNode,
    MathNode, NodePtr, PassiveNode, PenaltyNode, RuleNode,
};
use crate::scan::{new_native_word_node_global, node_recycle_abort, NodeRecycleAbort};
use crate::state::{
    ConfigState, EngineState, EqtbState, ErrorState, FontState, HyphState, LinebreakState,
    MemState, ScannerState,
};
use crate::units::{MemIdx, MemPtr, Scaled};

// Consts relocated from scan.rs with the locale-linebreak group (workstream 02).
const GLUE_PAR_XETEX_LINEBREAK_SKIP: i32 = 15;
const INT_PAR_XETEX_LINEBREAK_LOCALE: i32 = 66;
const INT_PAR_XETEX_LINEBREAK_PENALTY: i32 = 67;
const NATIVE_NODE_SIZE_NN: i32 = 6;

const MAX_HLIST_STACK: i16 = 512;
const HLIST_NODE: i32 = crate::consts::HLIST_NODE as i32;
const VLIST_NODE: i32 = 1;
const RULE_NODE: i32 = crate::consts::RULE_NODE as i32;
const DELTA_NODE: i32 = RULE_NODE;
const LIGATURE_NODE: i32 = crate::consts::LIGATURE_NODE as i32;
const INS_NODE: i32 = 3;
const MARK_NODE: i32 = 4;
const ADJUST_NODE: i32 = 5;
const DISC_NODE: i32 = crate::consts::DISC_NODE as i32;
const WHATSIT_NODE: i32 = 8;
const MATH_NODE: i32 = crate::consts::MATH_NODE as i32;
const GLUE_NODE: i32 = crate::consts::GLUE_NODE as i32;
const KERN_NODE: i32 = crate::consts::KERN_NODE as i32;
const PENALTY_NODE: i32 = crate::consts::PENALTY_NODE as i32;
const LANGUAGE_NODE: u16 = 4;
const GLYPH_NODE: u16 = 42;
const PIC_NODE: u16 = 43;
const PDF_NODE: u16 = 44;
const NORMAL: u16 = 0;
const HOLD_HEAD: i32 = 4_999_995;
const ACTIVE_LIST: i32 = 4_999_992;
const LAST_ACTIVE: i32 = ACTIVE_LIST;
const NATIVE_NODE_SIZE: i32 = 6;
const PASSIVE_NODE_SIZE: i32 = 2;
const ACTIVE_NODE_SIZE_NORMAL: i16 = 3;
const ACTIVE_NODE_SIZE_EXTENDED: i16 = 5;
const DELTA_NODE_SIZE: i32 = 7;
const LIG_TAG: u16 = 1;
const NON_ADDRESS: i32 = 0;
const NATIVE_WORD_NODE: u16 = 40;
const NATIVE_WORD_NODE_AT: u16 = 41;
const MIN_TRIE_OP: i32 = 0;
const HYPH_PRIME: i32 = 607;
const INT_PAR_PRETOLERANCE: i32 = 0;
const INT_PAR_TOLERANCE: i32 = 1;
const INT_PAR_LINE_PENALTY: i32 = 2;
const INT_PAR_HYPHEN_PENALTY: i32 = 3;
const INT_PAR_EX_HYPHEN_PENALTY: i32 = 4;
const INT_PAR_DOUBLE_HYPHEN_DEMERITS: i32 = 14;
const INT_PAR_FINAL_HYPHEN_DEMERITS: i32 = 15;
const INT_PAR_ADJ_DEMERITS: i32 = 16;
const INT_PAR_LOOSENESS: i32 = 19;
const INT_PAR_UC_HYPH: i32 = 38;
const INT_PAR_LAST_LINE_FIT: i32 = 62;
const INT_PAR_XETEX_USE_GLYPH_METRICS: i32 = 72;
const TEMP_HEAD: i32 = 4_999_996;
const ADJUST_HEAD: i32 = 4_999_994;
const PRE_ADJUST_HEAD: i32 = 4_999_985;
const GLUE_PAR_LEFT_SKIP: i16 = 7;
const GLUE_PAR_RIGHT_SKIP: i16 = 8;
const GLUE_PAR_PAR_FILL_SKIP: i16 = 14;
const INT_PAR_CLUB_PENALTY: i32 = 5;
const INT_PAR_WIDOW_PENALTY: i32 = 6;
const INT_PAR_DISPLAY_WIDOW_PENALTY: i32 = 7;
const INT_PAR_HANG_AFTER: i32 = 41;
const INT_PAR_BROKEN_PENALTY: i32 = 8;
const INT_PAR_INTER_LINE_PENALTY: i32 = 13;
const INT_PAR_XETEX_PROTRUDE_CHARS: i32 = 68;
const INT_PAR_TEXXET: i32 = 69;
const LOCAL_PAR_SHAPE: i32 = 0;
const DIMEN_PAR_HSIZE: i32 = 3;
const DIMEN_PAR_HANG_INDENT: i32 = 17;
const DIMEN_PAR_EMERGENCY_STRETCH: i32 = 20;
const ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES: i32 = 0;
const ETEX_PENALTIES_PAR_CLUB_PENALTIES: i32 = 1;
const ETEX_PENALTIES_PAR_WIDOW_PENALTIES: i32 = 2;
const ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES: i32 = 3;
const L_CODE: u16 = 4;
const END_M_CODE: u16 = 3;
const EXPLICIT: u16 = 1;
const SPACE_ADJUSTMENT: u16 = 3;
const EXACTLY: i16 = 0;
const ADDITIONAL: i16 = 1;
const UNHYPHENATED: i16 = 0;
const HYPHENATED: i16 = 1;
const VERY_LOOSE_FIT: usize = 0;
const TIGHT_FIT: usize = 3;
const DECENT_FIT: usize = 2;
const LOOSE_FIT: usize = 1;
const AWFUL_BAD: i32 = MAX_HALFWORD;
const INF_BAD: i32 = 10_000;
const INF_PENALTY: i32 = 10_000;
const EJECT_PENALTY: i32 = -10_000;

extern "C" {
    fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;
    // Hot tokenizer scalar — stays raw per ws02 Decision 4 (read under `unsafe`).
    static mut main_f: i32;
}

// translated from xetex-linebreak.c:77 (get_native_usv)
pub fn get_native_usv(mem: &MemState, p: i32, i: i32) -> i32 {
    let text = mem.as_u16_ptr(p + NATIVE_NODE_SIZE);
    let c = unsafe { *text.add(i as usize) };

    if (0xD800..0xDC00).contains(&c) {
        return 0x10000 + (c as i32 - 0xD800) * 0x400 + unsafe { *text.add(i as usize + 1) as i32 }
            - 0xDC00;
    }

    c as i32
}

pub fn get_native_usv_global(p: i32, i: i32) -> i32 {
    crate::state::with_state(|st| get_native_usv(&st.mem, p, i))
}

#[inline]
fn is_char_node_global(p: i32) -> bool {
    crate::state::with_state(|st| crate::mem::is_char_node(&st.mem, p))
}

#[inline]
fn is_non_discardable_node_global(p: i32) -> bool {
    crate::state::with_state(|st| is_non_discardable_node(&st.mem, p))
}

#[inline]
fn font_character_width(fonts: &mut FontState, f: i32, c: u16) -> i32 {
    let ec = crate::fonts::effective_char(fonts, true, f, c);
    let info = fonts.font_info_b16(fonts.char_base_entry(f) + ec);
    fonts.font_info_b32s1(fonts.width_base_entry(f) + info.s3 as i32)
}

#[inline]
fn font_character_width_global(f: i32, c: u16) -> i32 {
    crate::state::with_state_mut(|st| font_character_width(&mut st.fonts, f, c))
}

/// Wrap an active-list pointer as a typed [`NodePtr<ActiveNode>`]. This is
/// the active list's single **context-classification** boundary (`01c` §3):
/// every caller below is walking the line-breaker active list, which
/// establishes the kind (active vs. delta is by list position, not by the
/// type word — see the `node.rs` family note), and `p` is always a real
/// active node, never `TEX_NULL`.
#[inline]
fn active(p: i32) -> NodePtr<ActiveNode> {
    // SAFETY: the caller is walking the active list; `p` is a real active
    // node (the context boundary that justifies `from_idx_unchecked`).
    unsafe { NodePtr::<ActiveNode>::from_idx_unchecked(MemIdx::new(p)) }
}

#[inline]
fn delta_node(p: i32) -> NodePtr<DeltaNode> {
    // SAFETY: the caller is walking the active list and has already established
    // this node as `DELTA_NODE` in that list context.
    unsafe { NodePtr::<DeltaNode>::from_idx_unchecked(MemIdx::new(p)) }
}

#[inline]
fn passive_node(p: i32) -> NodePtr<PassiveNode> {
    // SAFETY: the caller is walking or constructing the passive list; passive
    // nodes are context-classified records with no NODE_type header.
    unsafe { NodePtr::<PassiveNode>::from_idx_unchecked(MemIdx::new(p)) }
}

#[inline]
fn glue_node(node: i32) -> NodePtr<GlueNode> {
    // SAFETY: callers enter through a `GLUE_NODE` branch.
    unsafe { NodePtr::<GlueNode>::from_idx_unchecked(MemIdx::new(node)) }
}

#[inline]
fn glue_spec(spec: i32) -> NodePtr<GlueSpecNode> {
    NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(spec)).expect("non-null glue spec")
}

#[inline]
fn kern_node(node: i32) -> NodePtr<KernNode> {
    // SAFETY: callers enter through a `KERN_NODE` branch.
    unsafe { NodePtr::<KernNode>::from_idx_unchecked(MemIdx::new(node)) }
}

#[inline]
fn ligature_node(node: i32) -> NodePtr<LigatureNode> {
    // SAFETY: callers enter through a `LIGATURE_NODE` branch.
    unsafe { NodePtr::<LigatureNode>::from_idx_unchecked(MemIdx::new(node)) }
}

#[inline]
fn disc_node(node: i32) -> NodePtr<DiscNode> {
    // SAFETY: callers enter through a `DISC_NODE` branch.
    unsafe { NodePtr::<DiscNode>::from_idx_unchecked(MemIdx::new(node)) }
}

#[inline]
fn math_node(node: i32) -> NodePtr<MathNode> {
    // SAFETY: callers enter through a `MATH_NODE` branch.
    unsafe { NodePtr::<MathNode>::from_idx_unchecked(MemIdx::new(node)) }
}

#[inline]
fn penalty_node(node: i32) -> NodePtr<PenaltyNode> {
    // SAFETY: callers enter through a `PENALTY_NODE` branch.
    unsafe { NodePtr::<PenaltyNode>::from_idx_unchecked(MemIdx::new(node)) }
}

#[inline]
fn rule_node(node: i32) -> NodePtr<RuleNode> {
    // SAFETY: callers enter through a `RULE_NODE` branch.
    unsafe { NodePtr::<RuleNode>::from_idx_unchecked(MemIdx::new(node)) }
}

#[inline]
fn box_node(mem: &MemState, node: i32) -> AnyBox {
    mem.classify_box(MemIdx::new(node))
        .expect("valid box node type")
        .expect("box node")
}

fn alloc_active_node(
    mem: &mut MemState,
    active_node_size: i32,
    link: MemPtr,
) -> Result<NodePtr<ActiveNode>, NodeRecycleAbort> {
    let p = crate::mem::get_node(mem, active_node_size);
    if p == crate::mem::ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    let p = active(p);
    p.set_link(mem, link);
    Ok(p)
}

fn alloc_delta_node(
    mem: &mut MemState,
    link: MemPtr,
) -> Result<NodePtr<DeltaNode>, NodeRecycleAbort> {
    let p = crate::mem::get_node(mem, DELTA_NODE_SIZE);
    if p == crate::mem::ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    let p = delta_node(p);
    p.set_link(mem, link);
    mem.set_b16s1(p.idx().0, DELTA_NODE as u16);
    mem.set_b16s0(p.idx().0, 0);
    Ok(p)
}

fn alloc_passive_node(mem: &mut MemState) -> Result<NodePtr<PassiveNode>, NodeRecycleAbort> {
    let p = crate::mem::get_node(mem, PASSIVE_NODE_SIZE);
    if p == crate::mem::ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    Ok(passive_node(p))
}

#[inline]
fn node_type_in(mem: &MemState, p: i32) -> i32 {
    mem.b16s1(p) as i32
}

#[inline]
fn node_type(p: i32) -> i32 {
    crate::state::with_state(|st| node_type_in(&st.mem, p))
}

#[inline]
fn node_subtype_in(mem: &MemState, p: i32) -> u16 {
    mem.b16s0(p)
}

#[inline]
fn node_subtype(p: i32) -> u16 {
    crate::state::with_state(|st| node_subtype_in(&st.mem, p))
}

#[inline]
fn box_width_in(mem: &MemState, p: i32) -> i32 {
    mem.b32s1(p + 1)
}

#[inline]
fn box_width(p: i32) -> i32 {
    crate::state::with_state(|st| box_width_in(&st.mem, p))
}

#[inline]
fn typed_list_width_in(mem: &MemState, node: i32) -> i32 {
    match node_type_in(mem, node) {
        HLIST_NODE | VLIST_NODE => box_node(mem, node).width(mem).0,
        RULE_NODE => rule_node(node).width(mem).raw(),
        KERN_NODE => kern_node(node).width(mem).0,
        MATH_NODE => math_node(node).width(mem).0,
        _ => unreachable!("typed width for non-width node"),
    }
}

#[inline]
fn glue_ptr_in(mem: &MemState, p: i32) -> i32 {
    glue_node(p).glue_ptr(mem).0
}

#[inline]
fn glue_ptr(p: i32) -> i32 {
    crate::state::with_state(|st| glue_ptr_in(&st.mem, p))
}

#[inline]
fn set_glue_ptr_in(mem: &mut MemState, p: i32, v: i32) {
    glue_node(p).set_glue_ptr(mem, MemPtr::from_raw(v));
}

#[inline]
fn set_glue_ptr(p: i32, v: i32) {
    crate::state::with_state_mut(|st| set_glue_ptr_in(&mut st.mem, p, v));
}

#[inline]
fn glue_leader_ptr_in(mem: &MemState, p: i32) -> i32 {
    glue_node(p).leader_ptr(mem).0
}

#[inline]
fn glue_leader_ptr(p: i32) -> i32 {
    crate::state::with_state(|st| glue_leader_ptr_in(&st.mem, p))
}

#[inline]
fn glue_spec_stretch_order_in(mem: &MemState, p: i32) -> u16 {
    glue_spec(p).stretch_order(mem)
}

#[inline]
fn glue_spec_stretch_order(p: i32) -> u16 {
    crate::state::with_state(|st| glue_spec_stretch_order_in(&st.mem, p))
}

#[inline]
fn glue_spec_shrink_order_in(mem: &MemState, p: i32) -> u16 {
    glue_spec(p).shrink_order(mem)
}

#[inline]
fn glue_spec_shrink_order(p: i32) -> u16 {
    crate::state::with_state(|st| glue_spec_shrink_order_in(&st.mem, p))
}

#[inline]
fn glue_spec_stretch_in(mem: &MemState, p: i32) -> i32 {
    glue_spec(p).stretch(mem).0
}

#[inline]
fn glue_spec_stretch(p: i32) -> i32 {
    crate::state::with_state(|st| glue_spec_stretch_in(&st.mem, p))
}

#[inline]
fn glue_spec_shrink_in(mem: &MemState, p: i32) -> i32 {
    glue_spec(p).shrink(mem).0
}

#[inline]
fn glue_spec_shrink(p: i32) -> i32 {
    crate::state::with_state(|st| glue_spec_shrink_in(&st.mem, p))
}

#[inline]
fn penalty_node_penalty_in(mem: &MemState, p: i32) -> i32 {
    penalty_node(p).penalty(mem)
}

#[inline]
fn penalty_node_penalty(p: i32) -> i32 {
    crate::state::with_state(|st| penalty_node_penalty_in(&st.mem, p))
}

#[inline]
fn set_penalty_node_penalty_in(mem: &mut MemState, p: i32, v: i32) {
    penalty_node(p).set_penalty(mem, v);
}

#[inline]
fn set_penalty_node_penalty(p: i32, v: i32) {
    crate::state::with_state_mut(|st| set_penalty_node_penalty_in(&mut st.mem, p, v));
}

#[inline]
fn language_node_what_lang_in(mem: &MemState, p: i32) -> u8 {
    mem.b32s1(p + 1) as u8
}

#[inline]
fn language_node_what_lang(p: i32) -> u8 {
    crate::state::with_state(|st| language_node_what_lang_in(&st.mem, p))
}

#[inline]
fn language_node_what_lhm_in(mem: &MemState, p: i32) -> i32 {
    mem.b16s1(p + 1) as i32
}

#[inline]
fn language_node_what_lhm(p: i32) -> i32 {
    crate::state::with_state(|st| language_node_what_lhm_in(&st.mem, p))
}

#[inline]
fn language_node_what_rhm_in(mem: &MemState, p: i32) -> i32 {
    mem.b16s0(p + 1) as i32
}

#[inline]
fn language_node_what_rhm(p: i32) -> i32 {
    crate::state::with_state(|st| language_node_what_rhm_in(&st.mem, p))
}

#[inline]
fn ligature_node_lig_font_in(mem: &MemState, p: i32) -> i32 {
    ligature_node(p).font(mem).0
}

#[inline]
fn ligature_node_lig_font(p: i32) -> i32 {
    crate::state::with_state(|st| ligature_node_lig_font_in(&st.mem, p))
}

#[inline]
fn ligature_node_lig_char_in(mem: &MemState, p: i32) -> u16 {
    ligature_node(p).character(mem)
}

#[inline]
fn ligature_node_lig_char(p: i32) -> u16 {
    crate::state::with_state(|st| ligature_node_lig_char_in(&st.mem, p))
}

#[inline]
fn ligature_node_lig_ptr_in(mem: &MemState, p: i32) -> i32 {
    ligature_node(p).lig_ptr(mem).0
}

#[inline]
fn ligature_node_lig_ptr(p: i32) -> i32 {
    crate::state::with_state(|st| ligature_node_lig_ptr_in(&st.mem, p))
}

#[inline]
fn disc_pre_break_in(mem: &MemState, p: i32) -> i32 {
    disc_node(p).pre_break(mem).0
}

#[inline]
fn disc_pre_break(p: i32) -> i32 {
    crate::state::with_state(|st| disc_pre_break_in(&st.mem, p))
}

#[inline]
fn disc_replace_count_in(mem: &MemState, p: i32) -> i32 {
    disc_node(p).subtype(mem) as i32
}

#[inline]
fn disc_replace_count(p: i32) -> i32 {
    crate::state::with_state(|st| disc_replace_count_in(&st.mem, p))
}

#[inline]
fn native_node_len_in(mem: &MemState, p: i32) -> i32 {
    mem.b16s1(p + 4) as i32
}

#[inline]
fn native_node_len(p: i32) -> i32 {
    crate::state::with_state(|st| native_node_len_in(&st.mem, p))
}

#[inline]
fn set_native_node_len_in(mem: &mut MemState, p: i32, v: i32) {
    mem.set_b16s1(p + 4, v as u16);
}

#[inline]
fn native_node_font_in(mem: &MemState, p: i32) -> i32 {
    mem.b16s2(p + 4) as i32
}

#[inline]
fn native_node_font(p: i32) -> i32 {
    crate::state::with_state(|st| native_node_font_in(&st.mem, p))
}

#[inline]
pub unsafe fn copy_native_tail(mem: &mut MemState, q: i32, ha: i32, start: i32) {
    let dst = mem.as_u16_ptr_mut(q + NATIVE_NODE_SIZE);
    let src = mem.as_u16_ptr(ha + NATIVE_NODE_SIZE);
    for i in start..native_node_len_in(mem, ha) {
        *dst.add((i - start) as usize) = *src.add(i as usize);
    }
}

fn split_native_tail_for_linebreak(
    st: &mut EngineState,
    start: i32,
    advance_ha: bool,
) -> Result<(), NodeRecycleAbort> {
    let ha = st.linebreak.ha;
    let len = native_node_len_in(&st.mem, ha);
    let q = crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, st.hyph.hf, len - start)?;
    st.mem.set_b16s0(q, st.mem.b16s0(ha));
    unsafe {
        copy_native_tail(&mut st.mem, q, ha, start);
        set_native_metrics(&st.mem, &st.eqtb, &st.fonts, q);
    }
    st.mem.set_b32s1(q, st.mem.b32s1(ha));
    st.mem.set_b32s1(ha, q);
    set_native_node_len_in(&mut st.mem, ha, start);
    unsafe {
        set_native_metrics(&st.mem, &st.eqtb, &st.fonts, ha);
    }
    if advance_ha {
        st.linebreak.ha = st.mem.b32s1(ha);
    }
    Ok(())
}

unsafe fn split_native_tail_for_linebreak_global(start: i32, advance_ha: bool) {
    match crate::state::with_state_mut(|st| split_native_tail_for_linebreak(st, start, advance_ha))
    {
        Ok(()) => {}
        Err(e) => node_recycle_abort(e),
    }
}

#[inline]
fn is_non_discardable_node(mem: &MemState, p: i32) -> bool {
    (mem.b16s1(p) as i32) < MATH_NODE
}

#[inline]
fn is_empty_box_in(mem: &MemState, p: i32) -> bool {
    if mem.b16s1(p) as i32 != HLIST_NODE {
        return false;
    }
    let p = box_node(mem, p);
    p.width(mem).0 == 0
        && p.height(mem).0 == 0
        && p.depth(mem).0 == 0
        && p.list_ptr(mem).0 == TEX_NULL
}

#[inline]
fn ignorable_protrusion_node_in(mem: &MemState, p: i32) -> bool {
    !crate::mem::is_char_node(mem, p)
        && match mem.b16s1(p) as i32 {
            INS_NODE | MARK_NODE | ADJUST_NODE | PENALTY_NODE => true,
            DISC_NODE => {
                let p = disc_node(p);
                p.pre_break(mem).0 == TEX_NULL
                    && p.post_break(mem).0 == TEX_NULL
                    && p.subtype(mem) == 0
            }
            MATH_NODE => math_node(p).width(mem).0 == 0,
            KERN_NODE => {
                let p = kern_node(p);
                p.width(mem).0 == 0 || p.subtype(mem) == NORMAL
            }
            GLUE_NODE => glue_node(p).glue_ptr(mem).0 == 0,
            HLIST_NODE => {
                let p = box_node(mem, p);
                p.width(mem).0 == 0
                    && p.height(mem).0 == 0
                    && p.depth(mem).0 == 0
                    && p.list_ptr(mem).0 == TEX_NULL
            }
            _ => false,
        }
}

// translated from xetex-linebreak.c:2827 (push_node)
pub fn push_node(linebreak: &mut LinebreakState, p: i32) -> bool {
    let level = linebreak.hlist_stack_level;
    if level > MAX_HLIST_STACK {
        return false;
    }

    linebreak.hlist_stack[level as usize] = p;
    linebreak.hlist_stack_level = level + 1;
    true
}

// translated from xetex-linebreak.c:2837 (pop_node)
pub fn pop_node(linebreak: &mut LinebreakState) -> Option<i32> {
    let level = linebreak.hlist_stack_level - 1;
    linebreak.hlist_stack_level = level;
    if level < 0 {
        return None;
    }

    Some(linebreak.hlist_stack[level as usize])
}

// translated from xetex-linebreak.c:2692 (total_pw)
pub fn total_pw(
    mem: &mut MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    q: i32,
    p: i32,
) -> Option<i32> {
    let break_node = active(q).break_node(mem);
    let mut l = if break_node.0 == TEX_NULL {
        linebreak.first_p
    } else {
        passive_node(break_node.0).cur_break(mem).0
    };

    let mut r = crate::mem::prev_rightmost(mem, linebreak.global_prev_p, p);
    if p != TEX_NULL && mem.b16s1(p) as i32 == DISC_NODE {
        let p_disc = disc_node(p);
        if p_disc.pre_break(mem).0 != TEX_NULL {
            r = p_disc.pre_break(mem).0;
            while mem.b32s1(r) != TEX_NULL {
                r = mem.b32s1(r);
            }
        } else {
            r = find_protchar_right(mem, linebreak, l, r)?;
        }
    } else {
        r = find_protchar_right(mem, linebreak, l, r)?;
    }

    if l != TEX_NULL && mem.b16s1(l) as i32 == DISC_NODE {
        let l_disc = disc_node(l);
        if l_disc.post_break(mem).0 != TEX_NULL {
            l = l_disc.post_break(mem).0;
        } else {
            let mut n = l_disc.subtype(mem);
            l = l_disc.link(mem).0;
            while n > 0 {
                if mem.b32s1(l) != TEX_NULL {
                    l = mem.b32s1(l);
                }
                n -= 1;
            }
            l = find_protchar_left(mem, linebreak, l, true)?;
        }
    } else {
        l = find_protchar_left(mem, linebreak, l, true)?;
    }

    // `char_pw_in` is fully threaded (mem/font reads only, no `with_state`
    // bridge), so the protrusion widths fold directly into this borrow rather
    // than running after it drops (the session-76 split is no longer needed).
    Some(char_pw_in(mem, fonts, linebreak, l, 0) + char_pw_in(mem, fonts, linebreak, r, 1))
}

// translated from xetex-linebreak.c:2731 (find_protchar_left)
pub fn find_protchar_left(
    mem: &mut MemState,
    linebreak: &mut LinebreakState,
    mut l: i32,
    d: bool,
) -> Option<i32> {
    if is_empty_box_in(mem, l) {
        l = mem.b32s1(l);
    } else if d {
        while mem.b32s1(l) != TEX_NULL
            && !(crate::mem::is_char_node(mem, l) || is_non_discardable_node(mem, l))
        {
            l = mem.b32s1(l);
        }
    }

    linebreak.hlist_stack_level = 0;
    let mut run = true;
    loop {
        let t = l;
        while run
            && mem.b16s1(l) as i32 == HLIST_NODE
            && box_node(mem, l).list_ptr(mem).0 != TEX_NULL
        {
            if !push_node(linebreak, l) {
                return None;
            }
            l = box_node(mem, l).list_ptr(mem).0;
        }

        while run && ignorable_protrusion_node_in(mem, l) {
            while mem.b32s1(l) == TEX_NULL && linebreak.hlist_stack_level > 0 {
                l = pop_node(linebreak)?;
            }

            if mem.b32s1(l) != TEX_NULL {
                l = mem.b32s1(l);
            } else if linebreak.hlist_stack_level == 0 {
                run = false;
            }
        }

        if t == l {
            break;
        }
    }

    Some(l)
}

// translated from xetex-linebreak.c:2778 (find_protchar_right)
pub fn find_protchar_right(
    mem: &mut MemState,
    linebreak: &mut LinebreakState,
    mut l: i32,
    mut r: i32,
) -> Option<i32> {
    if r == TEX_NULL {
        return Some(TEX_NULL);
    }

    linebreak.hlist_stack_level = 0;
    let mut run = true;
    loop {
        let t = r;
        while run
            && mem.b16s1(r) as i32 == HLIST_NODE
            && box_node(mem, r).list_ptr(mem).0 != TEX_NULL
        {
            if !push_node(linebreak, l) || !push_node(linebreak, r) {
                return None;
            }
            l = box_node(mem, r).list_ptr(mem).0;
            r = l;
            while mem.b32s1(r) != TEX_NULL {
                r = mem.b32s1(r);
            }
        }

        while run && ignorable_protrusion_node_in(mem, r) {
            while r == l && linebreak.hlist_stack_level > 0 {
                r = pop_node(linebreak)?;
                l = pop_node(linebreak)?;
            }

            if r != l && r != TEX_NULL {
                r = crate::mem::prev_rightmost(mem, l, r);
            } else if r == l && linebreak.hlist_stack_level == 0 {
                run = false;
            }
        }

        if t == r {
            break;
        }
    }

    Some(r)
}

fn right_margin_kern_target(
    mem: &mut MemState,
    linebreak: &mut LinebreakState,
    q: i32,
    disc_break: bool,
) -> Option<(i32, i32)> {
    if disc_break && (crate::mem::is_char_node(mem, q) || mem.b16s1(q) as i32 != DISC_NODE) {
        Some((q, q))
    } else {
        let ptmp = crate::mem::prev_rightmost(mem, mem.b32s1(TEMP_HEAD), q);
        let p = find_protchar_right(mem, linebreak, mem.b32s1(TEMP_HEAD), ptmp)?;
        Some((p, ptmp))
    }
}

pub(crate) enum MarginKernAbort {
    Overflow,
    PdfError,
}

pub fn insert_right_margin_kern(
    mem: &mut MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    q: i32,
    disc_break: bool,
    last_rightmost: i32,
) -> Result<i32, MarginKernAbort> {
    let (p, ptmp) =
        right_margin_kern_target(mem, linebreak, q, disc_break).ok_or(MarginKernAbort::PdfError)?;

    let w = char_pw_in(mem, fonts, linebreak, p, 1);
    if w == 0 {
        return Ok(q);
    }

    let k = crate::mem::new_margin_kern(mem, -w, last_rightmost, 1);
    if k == crate::mem::ALLOC_FAILED {
        return Err(MarginKernAbort::Overflow);
    }
    let k = mem
        .classify_margin_kern(MemIdx::new(k))
        .expect("new_margin_kern stamps a valid node type")
        .expect("new_margin_kern returns a margin kern node");
    k.set_link(mem, MemPtr::from_raw(mem.b32s1(ptmp)));
    mem.set_b32s1(ptmp, k.idx().0);
    let new_q = if ptmp == q { mem.b32s1(q) } else { q };
    Ok(new_q)
}

fn left_margin_kern_target(
    mem: &mut MemState,
    linebreak: &mut LinebreakState,
    q: i32,
) -> Option<i32> {
    find_protchar_left(mem, linebreak, q, false)
}

pub fn insert_left_margin_kern(
    mem: &mut MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    q: i32,
    last_leftmost: i32,
) -> Result<i32, MarginKernAbort> {
    let p = left_margin_kern_target(mem, linebreak, q).ok_or(MarginKernAbort::PdfError)?;

    let w = char_pw_in(mem, fonts, linebreak, p, 0);
    if w == 0 {
        return Ok(q);
    }

    let k = crate::mem::new_margin_kern(mem, -w, last_leftmost, 0);
    if k == crate::mem::ALLOC_FAILED {
        return Err(MarginKernAbort::Overflow);
    }
    let k = mem
        .classify_margin_kern(MemIdx::new(k))
        .expect("new_margin_kern stamps a valid node type")
        .expect("new_margin_kern returns a margin kern node");
    k.set_link(mem, MemPtr::from_raw(q));
    Ok(k.idx().0)
}

/// translated from xetex-linebreak.c:2377 (finite_shrink)
///
/// Phase-C: threaded to `&mut MemState`; legacy callers use
/// [`finite_shrink_global`].
pub fn finite_shrink(mem: &mut MemState, p: i32) -> Result<i32, NodeRecycleAbort> {
    let q = crate::mem::new_spec(mem, p);
    if q == crate::mem::ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    let spec = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(q))
        .expect("new_spec returns a glue spec");
    spec.set_shrink_order(mem, NORMAL);
    crate::mem::delete_glue_ref(mem, p);
    Ok(q)
}

/// Transitional bridge-down wrapper for legacy `finite_shrink` callers.
pub unsafe fn finite_shrink_global(p: i32) -> i32 {
    use crate::globals::{
        no_shrink_error_yet, set_help_line, set_help_ptr, set_no_shrink_error_yet,
    };

    if no_shrink_error_yet() {
        set_no_shrink_error_yet(false);
        crate::output::error_here_with_diagnostic(
            c"Infinite glue shrinkage found in a paragraph".as_ptr(),
        );
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(5);
        set_help_line(
            4,
            c"The paragraph just ended includes some glue that has".as_ptr(),
        );
        set_help_line(
            3,
            c"infinite shrinkability, e.g., `\\hskip 0pt minus 1fil'.".as_ptr(),
        );
        set_help_line(
            2,
            c"Such glue doesn't belong there---it allows a paragraph".as_ptr(),
        );
        set_help_line(
            1,
            c"of any length to fit on one line. But it's safe to proceed,".as_ptr(),
        );
        set_help_line(
            0,
            c"since the offensive shrinkability has been made finite.".as_ptr(),
        );
        crate::errors::error();
    }

    match crate::state::with_state_mut(|st| finite_shrink(&mut st.mem, p)) {
        Ok(q) => q,
        Err(e) => node_recycle_abort(e),
    }
}

/// translated from xetex-linebreak.c:2402 (reconstitute)
///
/// Phase-C: threaded to `&mut MemState`, `&mut HyphState`, and
/// `&mut FontState`; legacy callers use [`reconstitute_global`].
pub fn reconstitute(
    mem: &mut MemState,
    hyph: &mut HyphState,
    fonts: &mut FontState,
    mut j: i16,
    n: i16,
    mut bchar: i32,
    mut hchar: i32,
) -> Result<i16, NodeRecycleAbort> {
    use crate::globals::{
        cur_l, cur_q, cur_r, lft_hit, lig_stack, ligature_present, rt_hit, set_cur_l, set_cur_q,
        set_cur_r, set_lft_hit, set_lig_stack, set_ligature_present, set_rt_hit,
    };

    #[inline]
    fn odd(v: u8) -> bool {
        v % 2 == 1
    }

    #[inline]
    fn update_right(hyph: &HyphState, j: i16, n: i16, bchar: i32, hchar: i32) -> i32 {
        if j < n {
            set_cur_r(hyph.hu[(j + 1) as usize]);
        } else {
            set_cur_r(bchar);
        }
        if odd(hyph.hyf[j as usize]) {
            hchar
        } else {
            TOO_BIG_CHAR
        }
    }

    hyph.hyphen_passed = 0;
    let mut t = HOLD_HEAD;
    let mut w = 0;
    mem.set_b32s1(HOLD_HEAD, TEX_NULL);
    set_cur_l(hyph.hu[j as usize]);
    set_cur_q(t);

    if j == 0 {
        set_ligature_present(hyph.init_lig);
        let mut p = hyph.init_list;
        if ligature_present() {
            set_lft_hit(hyph.init_lft);
        }
        while p > TEX_NULL {
            let q = crate::mem::get_avail(mem);
            if q == crate::mem::ALLOC_FAILED {
                return Err(NodeRecycleAbort::OverflowRunaway);
            }
            mem.set_b32s1(t, q);
            t = mem.b32s1(t);
            mem.set_b16s1(t, hyph.hf as u16);
            mem.set_b16s0(t, mem.b16s0(p));
            p = mem.b32s1(p);
        }
    } else if cur_l() < TOO_BIG_CHAR {
        let q = crate::mem::get_avail(mem);
        if q == crate::mem::ALLOC_FAILED {
            return Err(NodeRecycleAbort::OverflowRunaway);
        }
        mem.set_b32s1(t, q);
        t = mem.b32s1(t);
        mem.set_b16s1(t, hyph.hf as u16);
        mem.set_b16s0(t, cur_l() as u16);
    }

    set_lig_stack(TEX_NULL);
    let mut cur_rh = update_right(hyph, j, n, bchar, hchar);

    loop {
        'continue_: loop {
            let mut k;
            let mut q;

            if cur_l() == TOO_BIG_CHAR {
                k = fonts.bchar_label_entry(hyph.hf);
                if k == NON_ADDRESS {
                    break 'continue_;
                }
                q = fonts.font_info_b16(k);
            } else {
                let char_base = fonts.char_base_entry(hyph.hf);
                let effective = crate::fonts::effective_char(fonts, true, hyph.hf, cur_l() as u16);
                q = fonts.font_info_b16(char_base + effective);
                if q.s1 % 4 != LIG_TAG {
                    break 'continue_;
                }
                k = fonts.lig_kern_base_entry(hyph.hf) + q.s0 as i32;
                q = fonts.font_info_b16(k);
                if q.s3 > 128 {
                    k = fonts.lig_kern_base_entry(hyph.hf)
                        + 256 * q.s1 as i32
                        + q.s0 as i32
                        + 32768
                        - 256 * 128;
                    q = fonts.font_info_b16(k);
                }
            }

            let mut test_char = if cur_rh < TOO_BIG_CHAR {
                cur_rh
            } else {
                cur_r()
            };

            loop {
                if q.s2 as i32 == test_char && q.s3 <= 128 {
                    if cur_rh < TOO_BIG_CHAR {
                        hyph.hyphen_passed = j;
                        hchar = TOO_BIG_CHAR;
                        cur_rh = TOO_BIG_CHAR;
                        continue 'continue_;
                    }

                    if hchar < TOO_BIG_CHAR && odd(hyph.hyf[j as usize]) {
                        hyph.hyphen_passed = j;
                        hchar = TOO_BIG_CHAR;
                    }

                    if q.s1 < 128 {
                        if cur_l() == TOO_BIG_CHAR {
                            set_lft_hit(true);
                        }
                        if j == n && lig_stack() == TEX_NULL {
                            set_rt_hit(true);
                        }

                        match q.s1 {
                            1 | 5 => {
                                set_cur_l(q.s0 as i32);
                                set_ligature_present(true);
                            }
                            2 | 6 => {
                                set_cur_r(q.s0 as i32);
                                if lig_stack() > TEX_NULL {
                                    mem.set_b16s0(lig_stack(), cur_r() as u16);
                                } else {
                                    let lig_item = crate::mem::new_lig_item(mem, cur_r() as u16);
                                    if lig_item == crate::mem::ALLOC_FAILED {
                                        return Err(NodeRecycleAbort::Overflow);
                                    }
                                    set_lig_stack(lig_item);
                                    if j == n {
                                        bchar = TOO_BIG_CHAR;
                                    } else {
                                        let p = crate::mem::get_avail(mem);
                                        if p == crate::mem::ALLOC_FAILED {
                                            return Err(NodeRecycleAbort::OverflowRunaway);
                                        }
                                        mem.set_b32s1(lig_stack() + 1, p);
                                        mem.set_b16s0(p, hyph.hu[(j + 1) as usize] as u16);
                                        mem.set_b16s1(p, hyph.hf as u16);
                                    }
                                }
                            }
                            3 => {
                                set_cur_r(q.s0 as i32);
                                let p = lig_stack();
                                let lig_item = crate::mem::new_lig_item(mem, cur_r() as u16);
                                if lig_item == crate::mem::ALLOC_FAILED {
                                    return Err(NodeRecycleAbort::Overflow);
                                }
                                set_lig_stack(lig_item);
                                mem.set_b32s1(lig_stack(), p);
                            }
                            7 | 11 => {
                                if ligature_present() {
                                    let p = crate::mem::new_ligature(
                                        mem,
                                        hyph.hf,
                                        cur_l() as u16,
                                        mem.b32s1(cur_q()),
                                    );
                                    if p == crate::mem::ALLOC_FAILED {
                                        return Err(NodeRecycleAbort::Overflow);
                                    }
                                    if lft_hit() {
                                        mem.set_b16s0(p, 2);
                                        set_lft_hit(false);
                                    }
                                    mem.set_b32s1(cur_q(), p);
                                    t = p;
                                    set_ligature_present(false);
                                }
                                set_cur_q(t);
                                set_cur_l(q.s0 as i32);
                                set_ligature_present(true);
                            }
                            _ => {
                                set_cur_l(q.s0 as i32);
                                set_ligature_present(true);
                                if lig_stack() > TEX_NULL {
                                    if mem.b32s1(lig_stack() + 1) > TEX_NULL {
                                        mem.set_b32s1(t, mem.b32s1(lig_stack() + 1));
                                        t = mem.b32s1(t);
                                        j += 1;
                                    }
                                    let p = lig_stack();
                                    set_lig_stack(mem.b32s1(p));
                                    crate::mem::free_node(mem, p, SMALL_NODE_SIZE);
                                    if lig_stack() == TEX_NULL {
                                        cur_rh = update_right(hyph, j, n, bchar, hchar);
                                    } else {
                                        set_cur_r(mem.b16s0(lig_stack()) as i32);
                                    }
                                } else if j == n {
                                    break 'continue_;
                                } else {
                                    let p = crate::mem::get_avail(mem);
                                    if p == crate::mem::ALLOC_FAILED {
                                        return Err(NodeRecycleAbort::OverflowRunaway);
                                    }
                                    mem.set_b32s1(t, p);
                                    t = mem.b32s1(t);
                                    mem.set_b16s1(t, hyph.hf as u16);
                                    mem.set_b16s0(t, cur_r() as u16);
                                    j += 1;
                                    cur_rh = update_right(hyph, j, n, bchar, hchar);
                                }
                            }
                        }

                        if q.s1 > 4 && q.s1 != 7 {
                            break 'continue_;
                        }
                        continue 'continue_;
                    }

                    w = fonts.font_info_b32s1(
                        fonts.kern_base_entry(hyph.hf) + 256 * q.s1 as i32 + q.s0 as i32,
                    );
                    break 'continue_;
                }

                if q.s3 >= 128 {
                    if cur_rh == TOO_BIG_CHAR {
                        break 'continue_;
                    }
                    cur_rh = TOO_BIG_CHAR;
                    continue 'continue_;
                }

                k += q.s3 as i32 + 1;
                q = fonts.font_info_b16(k);
                test_char = if cur_rh < TOO_BIG_CHAR {
                    cur_rh
                } else {
                    cur_r()
                };
            }
        }

        if ligature_present() {
            let p = crate::mem::new_ligature(mem, hyph.hf, cur_l() as u16, mem.b32s1(cur_q()));
            if p == crate::mem::ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow);
            }
            if lft_hit() {
                mem.set_b16s0(p, 2);
                set_lft_hit(false);
            }
            if rt_hit() && lig_stack() == TEX_NULL {
                mem.set_b16s0(p, mem.b16s0(p) + 1);
                set_rt_hit(false);
            }
            mem.set_b32s1(cur_q(), p);
            t = p;
            set_ligature_present(false);
        }

        if w != 0 {
            let p = crate::mem::new_kern(mem, w);
            if p == crate::mem::ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow);
            }
            mem.set_b32s1(t, p);
            t = mem.b32s1(t);
            w = 0;
            mem.set_b32s0(t + 2, 0);
        }

        if lig_stack() > TEX_NULL {
            set_cur_q(t);
            set_cur_l(mem.b16s0(lig_stack()) as i32);
            set_ligature_present(true);
            if mem.b32s1(lig_stack() + 1) > TEX_NULL {
                mem.set_b32s1(t, mem.b32s1(lig_stack() + 1));
                t = mem.b32s1(t);
                j += 1;
            }
            let p = lig_stack();
            set_lig_stack(mem.b32s1(p));
            crate::mem::free_node(mem, p, SMALL_NODE_SIZE);
            if lig_stack() == TEX_NULL {
                cur_rh = update_right(hyph, j, n, bchar, hchar);
            } else {
                set_cur_r(mem.b16s0(lig_stack()) as i32);
            }
            continue;
        }

        return Ok(j);
    }
}

/// Transitional bridge-down wrapper for legacy `reconstitute` callers.
pub unsafe fn reconstitute_global(j: i16, n: i16, bchar: i32, hchar: i32) -> i16 {
    match crate::state::with_state_mut(|st| {
        reconstitute(&mut st.mem, &mut st.hyph, &mut st.fonts, j, n, bchar, hchar)
    }) {
        Ok(j) => j,
        Err(e) => node_recycle_abort(e),
    }
}

#[inline]
pub unsafe fn set_native_metrics(mem: &MemState, eqtb: &EqtbState, fonts: &FontState, p: i32) {
    // Call the threaded `measure_native_node(&FontState, …)` directly rather than
    // the `measure_native_node_global` bridge: the bridge re-borrows the whole
    // `ENGINE_STATE` `RefCell`, so it cannot be reached from a threaded caller
    // that already holds a borrow (the 135/136 FFI re-entry wall). The C++
    // measurer itself only touches the passed `&FontState` + pure C FFI, so it is
    // re-entry-free under any borrow.
    measure_native_node(
        fonts,
        mem.node_addr(p),
        (eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0) as libc::c_int,
    );
}

#[inline]
fn int_par(eqtb: &EqtbState, n: i32) -> i32 {
    eqtb.b32s1(INT_BASE + n)
}

#[inline]
fn int_par_global(n: i32) -> i32 {
    crate::state::with_state(|st| int_par(&st.eqtb, n))
}

#[inline]
fn dimen_par(eqtb: &EqtbState, n: i32) -> i32 {
    eqtb.b32s1(DIMEN_BASE + n)
}

#[inline]
fn dimen_par_global(n: i32) -> i32 {
    crate::state::with_state(|st| dimen_par(&st.eqtb, n))
}

#[inline]
fn glue_par(eqtb: &EqtbState, n: i16) -> i32 {
    eqtb.b32s1(GLUE_BASE + n as i32)
}

#[inline]
fn glue_par_global(n: i16) -> i32 {
    crate::state::with_state(|st| glue_par(&st.eqtb, n))
}

#[inline]
fn lc_code(eqtb: &EqtbState, c: i32) -> i32 {
    eqtb.b32s1(LC_CODE_BASE + c)
}

#[inline]
fn lc_code_global(c: i32) -> i32 {
    crate::state::with_state(|st| lc_code(&st.eqtb, c))
}

fn set_current_hyph_index(hyph: &mut HyphState) {
    let lang = hyph.cur_lang as i32;
    let start = hyph.hyph_start;
    if hyph.trie_trc[(start + lang) as usize] != hyph.cur_lang as u16 {
        hyph.hyph_index = 0;
    } else {
        hyph.hyph_index = hyph.trie_trl[(start + lang) as usize];
    }
}

fn set_current_hyph_index_global() {
    crate::state::with_state_mut(|st| set_current_hyph_index(&mut st.hyph));
}

fn start_linebreak_pass(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    linebreak: &mut LinebreakState,
    errors: &mut ErrorState,
    prev_graf: i32,
) -> Result<i32, NodeRecycleAbort> {
    let q_active = alloc_active_node(
        mem,
        linebreak.active_node_size as i32,
        MemPtr::from_raw(LAST_ACTIVE),
    )?;
    q_active.set_break_type(mem, UNHYPHENATED as u16);
    q_active.set_fitness(mem, DECENT_FIT as u16);
    q_active.set_break_node(mem, MemPtr::NULL);
    q_active.set_line_number(mem, prev_graf + 1);
    q_active.set_total_demerits(mem, 0);
    mem.set_b32s1(ACTIVE_LIST, q_active.idx().0);
    if linebreak.do_last_line_fit {
        q_active.set_shortfall(mem, Scaled(0));
        q_active.set_glue(mem, Scaled(0));
    }

    for k in 1..=6 {
        scanner.active_width[k] = linebreak.background[k];
    }
    linebreak.passive = TEX_NULL;
    errors.font_in_short_display = 0;
    linebreak.cur_p = mem.b32s1(TEMP_HEAD);
    linebreak.global_prev_p = linebreak.cur_p;
    linebreak.first_p = linebreak.cur_p;

    Ok(linebreak.cur_p)
}

fn start_linebreak_pass_global(prev_graf: i32) -> i32 {
    match crate::state::with_state_mut(|st| {
        let st = &mut *st;
        start_linebreak_pass(
            &mut st.mem,
            &mut st.scanner,
            &mut st.linebreak,
            &mut st.errors,
            prev_graf,
        )
    }) {
        Ok(p) => p,
        Err(e) => unsafe { node_recycle_abort(e) },
    }
}

fn free_active_and_passive_lists(mem: &mut MemState, linebreak: &mut LinebreakState) {
    let mut q = mem.b32s1(ACTIVE_LIST);
    while q != LAST_ACTIVE {
        if mem.b16s1(q) as i32 == DELTA_NODE {
            linebreak.cur_p = delta_node(q).link(mem).0;
            crate::mem::free_node(mem, q, DELTA_NODE_SIZE);
        } else {
            linebreak.cur_p = active(q).link(mem).0;
            crate::mem::free_node(mem, q, linebreak.active_node_size as i32);
        }
        q = linebreak.cur_p;
    }

    q = linebreak.passive;
    while q != TEX_NULL {
        linebreak.cur_p = passive_node(q).link(mem).0;
        crate::mem::free_node(mem, q, PASSIVE_NODE_SIZE);
        q = linebreak.cur_p;
    }
}

fn free_final_linebreak_lists(mem: &mut MemState, linebreak: &mut LinebreakState) {
    let mut q = mem.b32s1(ACTIVE_LIST);
    while q != ACTIVE_LIST {
        let next = if mem.b16s1(q) as i32 == DELTA_NODE {
            let next = delta_node(q).link(mem).0;
            crate::mem::free_node(mem, q, DELTA_NODE_SIZE);
            next
        } else {
            let next = active(q).link(mem).0;
            crate::mem::free_node(mem, q, linebreak.active_node_size as i32);
            next
        };
        q = next;
    }

    q = linebreak.passive;
    while q != TEX_NULL {
        let next = passive_node(q).link(mem).0;
        crate::mem::free_node(mem, q, PASSIVE_NODE_SIZE);
        q = next;
    }
}

fn free_active_and_passive_lists_global() {
    crate::state::with_state_mut(|st| {
        free_active_and_passive_lists(&mut st.mem, &mut st.linebreak)
    });
}

fn free_final_linebreak_lists_global() {
    crate::state::with_state_mut(|st| free_final_linebreak_lists(&mut st.mem, &mut st.linebreak));
}

fn choose_best_break(mem: &MemState, linebreak: &mut LinebreakState, eqtb: &EqtbState) -> bool {
    if mem.b32s1(ACTIVE_LIST) == LAST_ACTIVE {
        return false;
    }

    let mut r = mem.b32s1(ACTIVE_LIST);
    let mut fewest_demerits = MAX_HALFWORD;
    loop {
        if mem.b16s1(r) as i32 != DELTA_NODE && active(r).total_demerits(mem) < fewest_demerits {
            fewest_demerits = active(r).total_demerits(mem);
            linebreak.best_bet = r;
        }
        r = if mem.b16s1(r) as i32 == DELTA_NODE {
            delta_node(r).link(mem).0
        } else {
            active(r).link(mem).0
        };
        if r == LAST_ACTIVE {
            break;
        }
    }
    linebreak.best_line = active(linebreak.best_bet).line_number(mem);

    let looseness = eqtb.b32s1(INT_BASE + INT_PAR_LOOSENESS);
    if looseness == 0 {
        return true;
    }

    let mut r = mem.b32s1(ACTIVE_LIST);
    let mut actual_looseness = 0;
    loop {
        if mem.b16s1(r) as i32 != DELTA_NODE {
            let line_diff = active(r).line_number(mem) - linebreak.best_line;
            if (line_diff < actual_looseness && looseness <= line_diff)
                || (line_diff > actual_looseness && looseness >= line_diff)
            {
                linebreak.best_bet = r;
                actual_looseness = line_diff;
                fewest_demerits = active(r).total_demerits(mem);
            } else if line_diff == actual_looseness
                && active(r).total_demerits(mem) < fewest_demerits
            {
                linebreak.best_bet = r;
                fewest_demerits = active(r).total_demerits(mem);
            }
        }
        r = if mem.b16s1(r) as i32 == DELTA_NODE {
            delta_node(r).link(mem).0
        } else {
            active(r).link(mem).0
        };
        if r == LAST_ACTIVE {
            break;
        }
    }

    linebreak.best_line = active(linebreak.best_bet).line_number(mem);
    actual_looseness == looseness || linebreak.final_pass
}

fn choose_best_break_global() -> bool {
    crate::state::with_state_mut(|st| choose_best_break(&st.mem, &mut st.linebreak, &st.eqtb))
}

fn finish_last_line_fit(
    mem: &mut MemState,
    linebreak: &mut LinebreakState,
    last_line_fill: i32,
) -> Result<(), NodeRecycleAbort> {
    if !linebreak.do_last_line_fit {
        return Ok(());
    }

    let best = active(linebreak.best_bet);
    if best.shortfall(mem).0 == 0 {
        linebreak.do_last_line_fit = false;
        return Ok(());
    }

    // SAFETY: `last_line_fill` is the glue node returned by new_param_glue.
    let fill = unsafe { NodePtr::<GlueNode>::from_idx_unchecked(MemIdx::new(last_line_fill)) };
    let old_spec = fill.glue_ptr(mem);
    let q = crate::mem::new_spec(mem, old_spec.0);
    if q == crate::mem::ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    crate::mem::delete_glue_ref(mem, old_spec.0);

    let spec = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(q))
        .expect("new_spec returns a glue spec");
    spec.set_width(
        mem,
        Scaled(spec.width(mem).0 + best.shortfall(mem).0 - best.glue(mem).0),
    );
    spec.set_stretch(mem, Scaled(0));
    fill.set_glue_ptr(mem, MemPtr::from_raw(q));
    Ok(())
}

fn finish_last_line_fit_global(last_line_fill: i32) {
    match crate::state::with_state_mut(|st| {
        finish_last_line_fit(&mut st.mem, &mut st.linebreak, last_line_fill)
    }) {
        Ok(()) => {}
        Err(e) => unsafe { node_recycle_abort(e) },
    }
}

// translated from xetex-linebreak.c:90 (line_break)
pub unsafe fn line_break(d: bool) {
    use crate::globals::{
        active_width, background, cur_list_head, cur_list_mode_line, cur_list_prev_graf,
        cur_list_tail, eqtb_b32s1, font_bchar_entry, hc_entry, hi_mem_min, hn, hyph_index,
        hyphen_char_entry, l_hyf, last_special_line, mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1,
        r_hyf, set_active_node_size, set_active_width, set_background, set_cur_lang,
        set_cur_list_tail, set_cur_p, set_disc_width, set_do_last_line_fit, set_easy_line,
        set_fill_width, set_final_pass, set_first_indent, set_first_width, set_global_prev_p,
        set_ha, set_hb, set_hc_entry, set_hf, set_hn, set_hu_entry, set_hyf_bchar, set_hyf_char,
        set_init_l_hyf, set_init_r_hyf, set_l_hyf, set_last_special_line, set_mem_b16s1,
        set_mem_b32s1, set_minimal_demerits, set_minimum_demerits, set_no_shrink_error_yet,
        set_pack_begin_line, set_r_hyf, set_second_indent, set_second_width, set_threshold,
        set_xtx_ligature_present, trie_not_ready, trie_trc_entry, trie_tro_entry,
    };

    fn add_active_width(k: usize, v: i32) {
        crate::globals::set_active_width(k, crate::globals::active_width(k) + v);
    }

    unsafe fn add_node_width_to_active(p: i32, confusion_name: *const libc::c_char) {
        if is_char_node_global(p) {
            let f = mem_b16s1(p) as i32;
            add_active_width(1, font_character_width_global(f, mem_b16s0(p)));
            return;
        }

        match node_type(p) {
            LIGATURE_NODE => {
                let f = ligature_node_lig_font(p);
                set_xtx_ligature_present(true);
                add_active_width(1, font_character_width_global(f, ligature_node_lig_char(p)));
            }
            HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                add_active_width(1, box_width(p));
            }
            WHATSIT_NODE => {
                let subtype = node_subtype(p);
                if subtype == NATIVE_WORD_NODE
                    || subtype == NATIVE_WORD_NODE_AT
                    || subtype == GLYPH_NODE
                    || subtype == PIC_NODE
                    || subtype == PDF_NODE
                {
                    add_active_width(1, box_width(p));
                } else {
                    crate::errors::confusion(confusion_name);
                }
            }
            _ => crate::errors::confusion(confusion_name),
        }
    }

    set_pack_begin_line(cur_list_mode_line());
    set_mem_b32s1(TEMP_HEAD, mem_b32s1(cur_list_head()));

    let tail = cur_list_tail();
    if is_char_node_global(tail) || node_type(tail) != GLUE_NODE {
        let p = crate::mem::new_penalty_global(INF_PENALTY);
        set_mem_b32s1(tail, p);
        set_cur_list_tail(p);
    } else {
        set_mem_b16s1(tail, PENALTY_NODE as u16);
        crate::mem::delete_glue_ref_global(glue_ptr(tail));
        crate::packaging::flush_node_list_global(glue_leader_ptr(tail));
        set_penalty_node_penalty(tail, INF_PENALTY);
    }

    let last_line_fill = crate::mem::new_param_glue_global(GLUE_PAR_PAR_FILL_SKIP);
    set_mem_b32s1(cur_list_tail(), last_line_fill);
    set_init_l_hyf(cur_list_prev_graf() / 0x0400000);
    set_init_r_hyf((cur_list_prev_graf() / 65_536) % 64);
    let init_cur_lang = (cur_list_prev_graf() % 65_536) as u8;

    crate::mem::pop_nest();
    set_no_shrink_error_yet(true);

    if glue_spec_shrink_order(glue_par_global(GLUE_PAR_LEFT_SKIP)) != NORMAL
        && glue_spec_shrink(glue_par_global(GLUE_PAR_LEFT_SKIP)) != 0
    {
        let p = finite_shrink_global(glue_par_global(GLUE_PAR_LEFT_SKIP));
        crate::globals::set_eqtb_b32s1(GLUE_BASE + GLUE_PAR_LEFT_SKIP as i32, p);
    }
    if glue_spec_shrink_order(glue_par_global(GLUE_PAR_RIGHT_SKIP)) != NORMAL
        && glue_spec_shrink(glue_par_global(GLUE_PAR_RIGHT_SKIP)) != 0
    {
        let p = finite_shrink_global(glue_par_global(GLUE_PAR_RIGHT_SKIP));
        crate::globals::set_eqtb_b32s1(GLUE_BASE + GLUE_PAR_RIGHT_SKIP as i32, p);
    }

    let q = glue_par_global(GLUE_PAR_LEFT_SKIP);
    let r = glue_par_global(GLUE_PAR_RIGHT_SKIP);
    set_background(1, box_width(q) + box_width(r));
    for k in 2..=5 {
        set_background(k, 0);
    }
    set_background(
        2 + glue_spec_stretch_order(q) as usize,
        glue_spec_stretch(q),
    );
    set_background(
        2 + glue_spec_stretch_order(r) as usize,
        background(2 + glue_spec_stretch_order(r) as usize) + glue_spec_stretch(r),
    );
    set_background(6, glue_spec_shrink(q) + glue_spec_shrink(r));

    set_do_last_line_fit(false);
    set_active_node_size(ACTIVE_NODE_SIZE_NORMAL);
    if int_par_global(INT_PAR_LAST_LINE_FIT) > 0 {
        let q = glue_ptr(last_line_fill);
        if glue_spec_stretch(q) > 0
            && glue_spec_stretch_order(q) > NORMAL
            && background(3) == 0
            && background(4) == 0
            && background(5) == 0
        {
            set_do_last_line_fit(true);
            set_active_node_size(ACTIVE_NODE_SIZE_EXTENDED);
            set_fill_width(0, 0);
            set_fill_width(1, 0);
            set_fill_width(2, 0);
            set_fill_width(
                (glue_spec_stretch_order(q) - 1) as usize,
                glue_spec_stretch(q),
            );
        }
    }

    set_minimum_demerits(AWFUL_BAD);
    for fit in [TIGHT_FIT, DECENT_FIT, LOOSE_FIT, VERY_LOOSE_FIT] {
        set_minimal_demerits(fit, AWFUL_BAD);
    }

    if eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE) == TEX_NULL {
        if dimen_par_global(DIMEN_PAR_HANG_INDENT) == 0 {
            set_last_special_line(0);
            set_second_width(dimen_par_global(DIMEN_PAR_HSIZE));
            set_second_indent(0);
        } else {
            set_last_special_line(int_par_global(INT_PAR_HANG_AFTER).abs());
            if int_par_global(INT_PAR_HANG_AFTER) < 0 {
                set_first_width(
                    dimen_par_global(DIMEN_PAR_HSIZE)
                        - dimen_par_global(DIMEN_PAR_HANG_INDENT).abs(),
                );
                set_first_indent(if dimen_par_global(DIMEN_PAR_HANG_INDENT) >= 0 {
                    dimen_par_global(DIMEN_PAR_HANG_INDENT)
                } else {
                    0
                });
                set_second_width(dimen_par_global(DIMEN_PAR_HSIZE));
                set_second_indent(0);
            } else {
                set_first_width(dimen_par_global(DIMEN_PAR_HSIZE));
                set_first_indent(0);
                set_second_width(
                    dimen_par_global(DIMEN_PAR_HSIZE)
                        - dimen_par_global(DIMEN_PAR_HANG_INDENT).abs(),
                );
                set_second_indent(if dimen_par_global(DIMEN_PAR_HANG_INDENT) >= 0 {
                    dimen_par_global(DIMEN_PAR_HANG_INDENT)
                } else {
                    0
                });
            }
        }
    } else {
        let par_shape = eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
        set_last_special_line(mem_b32s0(par_shape) - 1);
        set_second_width(mem_b32s1(par_shape + 2 * (last_special_line() + 1)));
        set_second_indent(mem_b32s1(par_shape + 2 * last_special_line() + 1));
    }

    if int_par_global(INT_PAR_LOOSENESS) == 0 {
        set_easy_line(last_special_line());
    } else {
        set_easy_line(MAX_HALFWORD);
    }

    set_threshold(int_par_global(INT_PAR_PRETOLERANCE));
    let mut second_pass;
    if crate::globals::threshold() >= 0 {
        second_pass = false;
        set_final_pass(false);
    } else {
        set_threshold(int_par_global(INT_PAR_TOLERANCE));
        second_pass = true;
        set_final_pass(dimen_par_global(DIMEN_PAR_EMERGENCY_STRETCH) <= 0);
    }

    'passes: loop {
        if crate::globals::threshold() > INF_BAD {
            set_threshold(INF_BAD);
        }

        if second_pass {
            if trie_not_ready() {
                init_trie();
            }
            set_cur_lang(init_cur_lang);
            set_l_hyf(crate::globals::init_l_hyf());
            set_r_hyf(crate::globals::init_r_hyf());
            set_current_hyph_index_global();
        }

        let mut prev_p = start_linebreak_pass_global(cur_list_prev_graf());
        let mut auto_breaking = true;

        while crate::globals::cur_p() != TEX_NULL && mem_b32s1(ACTIVE_LIST) != LAST_ACTIVE {
            let mut cur = crate::globals::cur_p();

            if is_char_node_global(cur) {
                prev_p = cur;
                set_global_prev_p(cur);
                loop {
                    let f = mem_b16s1(cur) as i32;
                    add_active_width(1, font_character_width_global(f, mem_b16s0(cur)));
                    cur = mem_b32s1(cur);
                    set_cur_p(cur);
                    if !is_char_node_global(cur) {
                        break;
                    }
                }
            }

            cur = crate::globals::cur_p();
            match node_type(cur) {
                HLIST_NODE | VLIST_NODE | RULE_NODE => {
                    add_active_width(1, box_width(cur));
                }
                WHATSIT_NODE => {
                    let subtype = node_subtype(cur);
                    if subtype == LANGUAGE_NODE {
                        set_cur_lang(language_node_what_lang(cur));
                        set_l_hyf(language_node_what_lhm(cur));
                        set_r_hyf(language_node_what_rhm(cur));
                        set_current_hyph_index_global();
                    } else if subtype == NATIVE_WORD_NODE
                        || subtype == NATIVE_WORD_NODE_AT
                        || subtype == GLYPH_NODE
                        || subtype == PIC_NODE
                        || subtype == PDF_NODE
                    {
                        add_active_width(1, box_width(cur));
                    }
                }
                GLUE_NODE => {
                    if auto_breaking
                        && (is_char_node_global(prev_p)
                            || is_non_discardable_node_global(prev_p)
                            || (node_type(prev_p) == KERN_NODE && node_subtype(prev_p) != EXPLICIT))
                    {
                        try_break_global(0, UNHYPHENATED);
                    }

                    let mut q = glue_ptr(cur);
                    if glue_spec_shrink_order(q) != NORMAL && glue_spec_shrink(q) != 0 {
                        q = finite_shrink_global(q);
                        set_glue_ptr(cur, q);
                    }
                    add_active_width(1, box_width(q));
                    add_active_width(
                        2 + glue_spec_stretch_order(q) as usize,
                        glue_spec_stretch(q),
                    );
                    add_active_width(6, glue_spec_shrink(q));

                    if second_pass && auto_breaking {
                        let mut prev_s = cur;
                        let mut s = mem_b32s1(prev_s);

                        'hyphen_probe: {
                            if s == TEX_NULL {
                                break 'hyphen_probe;
                            }

                            let mut c: i32;
                            let mut l: i32 = 0;

                            loop {
                                if is_char_node_global(s) {
                                    c = mem_b16s0(s) as i32;
                                    set_hf(mem_b16s1(s) as i32);
                                } else if node_type(s) == LIGATURE_NODE {
                                    if ligature_node_lig_ptr(s) == TEX_NULL {
                                        prev_s = s;
                                        s = mem_b32s1(prev_s);
                                        continue;
                                    }
                                    q = ligature_node_lig_ptr(s);
                                    c = mem_b16s0(q) as i32;
                                    set_hf(mem_b16s1(q) as i32);
                                } else if node_type(s) == KERN_NODE && node_subtype(s) == NORMAL {
                                    prev_s = s;
                                    s = mem_b32s1(prev_s);
                                    continue;
                                } else if node_type(s) == MATH_NODE && node_subtype(s) >= L_CODE {
                                    prev_s = s;
                                    s = mem_b32s1(prev_s);
                                    continue;
                                } else if node_type(s) == WHATSIT_NODE {
                                    if node_subtype(s) == NATIVE_WORD_NODE
                                        || node_subtype(s) == NATIVE_WORD_NODE_AT
                                    {
                                        l = 0;
                                        while l < native_node_len(s) {
                                            c = get_native_usv_global(s, l);
                                            if lc_code_global(c) != 0 {
                                                set_hf(native_node_font(s));
                                                prev_s = s;
                                                if lc_code_global(c) == c
                                                    || int_par_global(INT_PAR_UC_HYPH) > 0
                                                {
                                                    break;
                                                } else {
                                                    break 'hyphen_probe;
                                                }
                                            }
                                            if c >= 65_536 {
                                                l += 1;
                                            }
                                            l += 1;
                                        }
                                        if l < native_node_len(s) {
                                            break;
                                        }
                                    }

                                    if node_subtype(s) == LANGUAGE_NODE {
                                        set_cur_lang(language_node_what_lang(s));
                                        set_l_hyf(language_node_what_lhm(s));
                                        set_r_hyf(language_node_what_rhm(s));
                                        set_current_hyph_index_global();
                                    }
                                    prev_s = s;
                                    s = mem_b32s1(prev_s);
                                    continue;
                                } else {
                                    break 'hyphen_probe;
                                }

                                if hyph_index() == 0 || c > 255 {
                                    set_hc_entry(0, lc_code_global(c));
                                } else if trie_trc_entry(hyph_index() + c) != c as u16 {
                                    set_hc_entry(0, 0);
                                } else {
                                    set_hc_entry(0, trie_tro_entry(hyph_index() + c));
                                }

                                if hc_entry(0) != 0 {
                                    if hc_entry(0) == c || int_par_global(INT_PAR_UC_HYPH) > 0 {
                                        break;
                                    } else {
                                        break 'hyphen_probe;
                                    }
                                }

                                prev_s = s;
                                s = mem_b32s1(prev_s);
                            }

                            set_hyf_char(hyphen_char_entry(crate::globals::hf()));
                            if crate::globals::hyf_char() < 0
                                || crate::globals::hyf_char() > TOO_BIG_CHAR - 1
                            {
                                break 'hyphen_probe;
                            }
                            set_ha(prev_s);
                            if l_hyf() + r_hyf() > crate::scan::max_hyphenatable_length() {
                                break 'hyphen_probe;
                            }

                            if crate::globals::ha() != TEX_NULL
                                && crate::globals::ha() < hi_mem_min()
                                && node_type(crate::globals::ha()) == WHATSIT_NODE
                                && (node_subtype(crate::globals::ha()) == NATIVE_WORD_NODE
                                    || node_subtype(crate::globals::ha()) == NATIVE_WORD_NODE_AT)
                            {
                                s = mem_b32s1(crate::globals::ha());
                                loop {
                                    if !is_char_node_global(s) {
                                        match node_type(s) {
                                            LIGATURE_NODE => {}
                                            KERN_NODE => {
                                                if node_subtype(s) != NORMAL {
                                                    break;
                                                }
                                            }
                                            WHATSIT_NODE | GLUE_NODE | PENALTY_NODE | INS_NODE
                                            | ADJUST_NODE | MARK_NODE => break,
                                            _ => break 'hyphen_probe,
                                        }
                                    }
                                    s = mem_b32s1(s);
                                }

                                set_hn(0);
                                'restart_native: loop {
                                    let len = native_node_len(crate::globals::ha());
                                    l = 0;
                                    while l < len {
                                        c = get_native_usv_global(crate::globals::ha(), l);
                                        if hyph_index() == 0 || c > 255 {
                                            set_hc_entry(0, lc_code_global(c));
                                        } else if trie_trc_entry(hyph_index() + c) != c as u16 {
                                            set_hc_entry(0, 0);
                                        } else {
                                            set_hc_entry(0, trie_tro_entry(hyph_index() + c));
                                        }

                                        if hc_entry(0) == 0 {
                                            if hn() > 0 {
                                                split_native_tail_for_linebreak_global(l, false);
                                                break 'restart_native;
                                            }
                                        } else if hn() == 0 && l > 0 {
                                            split_native_tail_for_linebreak_global(l, true);
                                            continue 'restart_native;
                                        } else if hn() as i32
                                            == crate::scan::max_hyphenatable_length()
                                        {
                                            break 'restart_native;
                                        } else {
                                            set_hn(hn() + 1);
                                            if c < 65_536 {
                                                set_hu_entry(hn() as usize, c);
                                                set_hc_entry(hn() as usize, hc_entry(0));
                                            } else {
                                                set_hu_entry(
                                                    hn() as usize,
                                                    (c - 65_536) / 1024 + 0xD800,
                                                );
                                                set_hc_entry(
                                                    hn() as usize,
                                                    (hc_entry(0) - 65_536) / 1024 + 0xD800,
                                                );
                                                set_hn(hn() + 1);
                                                set_hu_entry(hn() as usize, c % 1024 + 0xDC00);
                                                set_hc_entry(
                                                    hn() as usize,
                                                    hc_entry(0) % 1024 + 0xDC00,
                                                );
                                                l += 1;
                                            }
                                            set_hyf_bchar(TOO_BIG_CHAR);
                                        }
                                        l += 1;
                                    }
                                    break 'restart_native;
                                }
                            } else {
                                set_hn(0);
                                'collect_word: loop {
                                    if is_char_node_global(s) {
                                        if mem_b16s1(s) as i32 != crate::globals::hf() {
                                            break;
                                        }
                                        set_hyf_bchar(mem_b16s0(s) as i32);
                                        c = crate::globals::hyf_bchar();
                                        if hyph_index() == 0 || c > 255 {
                                            set_hc_entry(0, lc_code_global(c));
                                        } else if trie_trc_entry(hyph_index() + c) != c as u16 {
                                            set_hc_entry(0, 0);
                                        } else {
                                            set_hc_entry(0, trie_tro_entry(hyph_index() + c));
                                        }
                                        if hc_entry(0) == 0
                                            || hc_entry(0) > crate::globals::max_hyph_char()
                                            || hn() as i32 == crate::scan::max_hyphenatable_length()
                                        {
                                            break;
                                        }
                                        set_hb(s);
                                        set_hn(hn() + 1);
                                        set_hu_entry(hn() as usize, c);
                                        set_hc_entry(hn() as usize, hc_entry(0));
                                        set_hyf_bchar(TOO_BIG_CHAR);
                                    } else if node_type(s) == LIGATURE_NODE {
                                        if ligature_node_lig_font(s) != crate::globals::hf() {
                                            break;
                                        }
                                        let mut j = hn();
                                        q = ligature_node_lig_ptr(s);
                                        if q > TEX_NULL {
                                            set_hyf_bchar(mem_b16s0(q) as i32);
                                        }
                                        while q > TEX_NULL {
                                            c = mem_b16s0(q) as i32;
                                            if hyph_index() == 0 || c > 255 {
                                                set_hc_entry(0, lc_code_global(c));
                                            } else if trie_trc_entry(hyph_index() + c) != c as u16 {
                                                set_hc_entry(0, 0);
                                            } else {
                                                set_hc_entry(0, trie_tro_entry(hyph_index() + c));
                                            }
                                            if hc_entry(0) == 0
                                                || hc_entry(0) > crate::globals::max_hyph_char()
                                                || j as i32
                                                    == crate::scan::max_hyphenatable_length()
                                            {
                                                break 'collect_word;
                                            }
                                            j += 1;
                                            set_hu_entry(j as usize, c);
                                            set_hc_entry(j as usize, hc_entry(0));
                                            q = mem_b32s1(q);
                                        }
                                        set_hb(s);
                                        set_hn(j);
                                        if odd(node_subtype(s) as u8) {
                                            set_hyf_bchar(font_bchar_entry(crate::globals::hf()));
                                        } else {
                                            set_hyf_bchar(TOO_BIG_CHAR);
                                        }
                                    } else if node_type(s) == KERN_NODE && node_subtype(s) == NORMAL
                                    {
                                        set_hb(s);
                                        set_hyf_bchar(font_bchar_entry(crate::globals::hf()));
                                    } else {
                                        break;
                                    }
                                    s = mem_b32s1(s);
                                }
                            }

                            if (hn() as i32) < l_hyf() + r_hyf() {
                                break 'hyphen_probe;
                            }

                            loop {
                                if !is_char_node_global(s) {
                                    match node_type(s) {
                                        LIGATURE_NODE => {}
                                        KERN_NODE => {
                                            if node_subtype(s) != NORMAL {
                                                break;
                                            }
                                        }
                                        WHATSIT_NODE | GLUE_NODE | PENALTY_NODE | INS_NODE
                                        | ADJUST_NODE | MARK_NODE => break,
                                        MATH_NODE => {
                                            if node_subtype(s) >= L_CODE {
                                                break;
                                            } else {
                                                break 'hyphen_probe;
                                            }
                                        }
                                        _ => break 'hyphen_probe,
                                    }
                                }
                                s = mem_b32s1(s);
                            }

                            hyphenate();
                        }
                    }
                }
                KERN_NODE => {
                    if node_subtype(cur) == EXPLICIT {
                        if !is_char_node_global(mem_b32s1(cur))
                            && auto_breaking
                            && node_type(mem_b32s1(cur)) == GLUE_NODE
                        {
                            try_break_global(0, UNHYPHENATED);
                        }
                        add_active_width(1, box_width(cur));
                    } else {
                        add_active_width(1, box_width(cur));
                    }
                }
                LIGATURE_NODE => {
                    let f = ligature_node_lig_font(cur);
                    set_xtx_ligature_present(true);
                    add_active_width(
                        1,
                        font_character_width_global(f, ligature_node_lig_char(cur)),
                    );
                }
                DISC_NODE => {
                    let mut s = disc_pre_break(cur);
                    set_disc_width(0);
                    if s == TEX_NULL {
                        try_break_global(int_par_global(INT_PAR_EX_HYPHEN_PENALTY), HYPHENATED);
                    } else {
                        set_disc_width(0);
                        while s != TEX_NULL {
                            let before = active_width(1);
                            add_node_width_to_active(s, c"disc3".as_ptr());
                            set_disc_width(crate::globals::disc_width() + active_width(1) - before);
                            set_active_width(1, before);
                            s = mem_b32s1(s);
                        }
                        add_active_width(1, crate::globals::disc_width());
                        try_break_global(int_par_global(INT_PAR_HYPHEN_PENALTY), HYPHENATED);
                        add_active_width(1, -crate::globals::disc_width());
                    }

                    let mut count = disc_replace_count(cur);
                    s = mem_b32s1(cur);
                    while count > 0 {
                        add_node_width_to_active(s, c"disc4".as_ptr());
                        count -= 1;
                        s = mem_b32s1(s);
                    }

                    prev_p = cur;
                    set_global_prev_p(cur);
                    set_cur_p(s);
                    continue;
                }
                MATH_NODE => {
                    if node_subtype(cur) < L_CODE {
                        auto_breaking = odd(node_subtype(cur) as u8);
                    }
                    if !is_char_node_global(mem_b32s1(cur))
                        && auto_breaking
                        && node_type(mem_b32s1(cur)) == GLUE_NODE
                    {
                        try_break_global(0, UNHYPHENATED);
                    }
                    add_active_width(1, box_width(cur));
                }
                PENALTY_NODE => {
                    try_break_global(penalty_node_penalty(cur), UNHYPHENATED);
                }
                MARK_NODE | INS_NODE | ADJUST_NODE => {}
                _ => crate::errors::confusion(c"paragraph".as_ptr()),
            }

            prev_p = cur;
            set_global_prev_p(cur);
            set_cur_p(mem_b32s1(cur));
        }

        if crate::globals::cur_p() == TEX_NULL {
            try_break_global(EJECT_PENALTY, HYPHENATED);
            if choose_best_break_global() {
                break 'passes;
            }
        }

        free_active_and_passive_lists_global();

        if !second_pass {
            set_threshold(int_par_global(INT_PAR_TOLERANCE));
            second_pass = true;
            set_final_pass(dimen_par_global(DIMEN_PAR_EMERGENCY_STRETCH) <= 0);
        } else {
            set_background(
                2,
                background(2) + dimen_par_global(DIMEN_PAR_EMERGENCY_STRETCH),
            );
            set_final_pass(true);
        }
    }

    finish_last_line_fit_global(last_line_fill);

    post_line_break(d);

    free_final_linebreak_lists_global();

    set_pack_begin_line(0);
}

#[inline]
fn odd(v: u8) -> bool {
    v % 2 == 1
}

#[inline]
fn odd_i32(v: i32) -> bool {
    (v & 1) != 0
}

/// Phase-C: threaded to `&mut MemState`. Touches the `mem` arena directly and
/// allocates through the threaded [`crate::mem::get_avail`]. On arena
/// exhaustion it returns `true` (the `01b` §5 sentinel) rather than calling
/// `runaway()`/`overflow()` under the borrow; `post_line_break` performs the
/// abort after the borrow drops.
#[inline]
fn update_lr_stack(mem: &mut MemState, q: i32, lr_ptr: &mut i32) -> bool {
    let q_math = math_node(q);
    let q_subtype = q_math.subtype(mem) as i32;
    if odd_i32(q_subtype) {
        if *lr_ptr != TEX_NULL
            && mem.b32s0(*lr_ptr) == L_CODE as i32 * (q_subtype / L_CODE as i32) + END_M_CODE as i32
        {
            let temp_ptr = *lr_ptr;
            *lr_ptr = mem.b32s1(temp_ptr);
            mem.set_b32s1(temp_ptr, mem.avail);
            mem.avail = temp_ptr;
        }
    } else {
        let temp_ptr = crate::mem::get_avail(mem);
        if temp_ptr == crate::mem::ALLOC_FAILED {
            return true;
        }
        mem.set_b32s0(
            temp_ptr,
            L_CODE as i32 * (q_subtype / L_CODE as i32) + END_M_CODE as i32,
        );
        mem.set_b32s1(temp_ptr, *lr_ptr);
        *lr_ptr = temp_ptr;
    }
    false
}

fn reverse_best_breaks(mem: &mut MemState, linebreak: &mut LinebreakState) {
    let mut q = active(linebreak.best_bet).break_node(mem).0;
    linebreak.cur_p = TEX_NULL;
    loop {
        let r = passive_node(q);
        q = r.next_break(mem).0;
        r.set_next_break(mem, MemPtr::from_raw(linebreak.cur_p));
        linebreak.cur_p = r.idx().0;
        if q == TEX_NULL {
            break;
        }
    }
}

struct PostLineHpack {
    q: i32,
    cur_width: i32,
    cur_indent: i32,
    semantic_pagination: bool,
    disc_break: bool,
    post_disc_break: bool,
}

enum PostLineAbort {
    Node(NodeRecycleAbort),
    HlistStackFailure,
}

fn update_lr_stack_checked(
    mem: &mut MemState,
    q: i32,
    lr_ptr: &mut i32,
) -> Result<(), PostLineAbort> {
    if update_lr_stack(mem, q, lr_ptr) {
        Err(PostLineAbort::Node(NodeRecycleAbort::OverflowRunaway))
    } else {
        Ok(())
    }
}

fn post_line_break_prefix(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    linebreak: &mut LinebreakState,
    eqtb: &EqtbState,
    fonts: &FontState,
    config: &ConfigState,
    lr_ptr: &mut i32,
    cur_line: i32,
) -> Result<PostLineHpack, PostLineAbort> {
    if eqtb.b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
        let mut q = mem.b32s1(TEMP_HEAD);

        if *lr_ptr != TEX_NULL {
            let mut temp_ptr = *lr_ptr;
            let mut r = q;
            loop {
                let s = crate::mem::new_math(mem, 0, (mem.b32s0(temp_ptr) - 1) as i16);
                if s == crate::mem::ALLOC_FAILED {
                    return Err(PostLineAbort::Node(NodeRecycleAbort::Overflow));
                }
                math_node(s).set_link(mem, MemPtr::from_raw(r));
                r = s;
                temp_ptr = mem.b32s1(temp_ptr);
                if temp_ptr == TEX_NULL {
                    break;
                }
            }
            mem.set_b32s1(TEMP_HEAD, r);
        }

        let cur_break = passive_node(linebreak.cur_p).cur_break(mem).0;
        while q != cur_break {
            if q < mem.hi_mem_min && mem.b16s1(q) as i32 == MATH_NODE {
                update_lr_stack_checked(mem, q, lr_ptr)?;
            }
            q = mem.b32s1(q);
        }
    }

    let mut q = passive_node(linebreak.cur_p).cur_break(mem).0;
    let mut disc_break = false;
    let mut post_disc_break = false;
    let mut glue_break = false;

    if q == TEX_NULL {
        q = TEMP_HEAD;
        while mem.b32s1(q) != TEX_NULL {
            q = mem.b32s1(q);
        }
    } else if mem.b16s1(q) as i32 == GLUE_NODE {
        let q_glue = glue_node(q);
        let old_spec = q_glue.glue_ptr(mem);
        crate::mem::delete_glue_ref(mem, old_spec.0);
        let right_skip = eqtb.b32s1(GLUE_BASE + GLUE_PAR_RIGHT_SKIP as i32);
        q_glue.set_glue_ptr(mem, MemPtr::from_raw(right_skip));
        q_glue.set_subtype(mem, GLUE_PAR_RIGHT_SKIP as u16 + 1);
        let right_skip_spec = glue_spec(right_skip);
        right_skip_spec.set_ref_count(mem, right_skip_spec.ref_count(mem) + 1);
        glue_break = true;
    } else if mem.b16s1(q) as i32 == DISC_NODE {
        let q_disc = disc_node(q);
        let mut t = q_disc.subtype(mem);
        let mut r;

        if t == 0 {
            r = q_disc.link(mem).0;
        } else {
            r = q;
            while t > 1 {
                r = mem.b32s1(r);
                t -= 1;
            }

            let s = mem.b32s1(r);
            r = mem.b32s1(s);
            mem.set_b32s1(s, TEX_NULL);
            crate::packaging::flush_node_list(mem, q_disc.link(mem).0)
                .map_err(|msg| PostLineAbort::Node(NodeRecycleAbort::Confusion(msg)))?;
            q_disc.set_subtype(mem, 0);
        }

        if q_disc.post_break(mem).0 != TEX_NULL {
            let mut s = q_disc.post_break(mem).0;
            while mem.b32s1(s) != TEX_NULL {
                s = mem.b32s1(s);
            }
            mem.set_b32s1(s, r);
            r = q_disc.post_break(mem).0;
            q_disc.set_post_break(mem, MemPtr::NULL);
            post_disc_break = true;
        }

        if q_disc.pre_break(mem).0 != TEX_NULL {
            let mut s = q_disc.pre_break(mem).0;
            q_disc.set_link(mem, MemPtr::from_raw(s));
            while mem.b32s1(s) != TEX_NULL {
                s = mem.b32s1(s);
            }
            q_disc.set_pre_break(mem, MemPtr::NULL);
            q = s;
        }

        mem.set_b32s1(q, r);
        disc_break = true;
    } else if mem.b16s1(q) as i32 == KERN_NODE {
        kern_node(q).set_width(mem, Scaled(0));
    } else if mem.b16s1(q) as i32 == MATH_NODE {
        math_node(q).set_width(mem, Scaled(0));

        if eqtb.b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
            update_lr_stack_checked(mem, q, lr_ptr)?;
        }
    }

    if eqtb.b32s1(INT_BASE + INT_PAR_XETEX_PROTRUDE_CHARS) > 0 {
        q = insert_right_margin_kern(
            mem,
            fonts,
            linebreak,
            q,
            disc_break,
            linebreak.last_rightmost_char,
        )
        .map_err(|e| match e {
            MarginKernAbort::PdfError => PostLineAbort::HlistStackFailure,
            MarginKernAbort::Overflow => PostLineAbort::Node(NodeRecycleAbort::Overflow),
        })?;
    }

    if !glue_break {
        let r = crate::mem::new_param_glue(mem, eqtb, GLUE_PAR_RIGHT_SKIP);
        if r == crate::mem::ALLOC_FAILED {
            return Err(PostLineAbort::Node(NodeRecycleAbort::Overflow));
        }
        glue_node(r).set_link(mem, MemPtr::from_raw(mem.b32s1(q)));
        mem.set_b32s1(q, r);
        q = r;
    }

    if eqtb.b32s1(INT_BASE + INT_PAR_TEXXET) > 0 && *lr_ptr != TEX_NULL {
        let mut s = TEMP_HEAD;
        let mut r = mem.b32s1(s);
        while r != q {
            s = r;
            r = mem.b32s1(s);
        }

        r = *lr_ptr;
        while r != TEX_NULL {
            let temp_ptr = crate::mem::new_math(mem, 0, mem.b32s0(r) as i16);
            if temp_ptr == crate::mem::ALLOC_FAILED {
                return Err(PostLineAbort::Node(NodeRecycleAbort::Overflow));
            }
            mem.set_b32s1(s, temp_ptr);
            s = temp_ptr;
            r = mem.b32s1(r);
        }
        mem.set_b32s1(s, q);
    }

    let r = mem.b32s1(q);
    mem.set_b32s1(q, TEX_NULL);
    q = mem.b32s1(TEMP_HEAD);
    mem.set_b32s1(TEMP_HEAD, r);

    if eqtb.b32s1(INT_BASE + INT_PAR_XETEX_PROTRUDE_CHARS) > 0 {
        q = insert_left_margin_kern(mem, fonts, linebreak, q, linebreak.last_leftmost_char)
            .map_err(|e| match e {
                MarginKernAbort::PdfError => PostLineAbort::HlistStackFailure,
                MarginKernAbort::Overflow => PostLineAbort::Node(NodeRecycleAbort::Overflow),
            })?;
    }

    if eqtb.b32s1(GLUE_BASE + GLUE_PAR_LEFT_SKIP as i32) != 0 {
        let r = crate::mem::new_param_glue(mem, eqtb, GLUE_PAR_LEFT_SKIP);
        if r == crate::mem::ALLOC_FAILED {
            return Err(PostLineAbort::Node(NodeRecycleAbort::Overflow));
        }
        glue_node(r).set_link(mem, MemPtr::from_raw(q));
        q = r;
    }

    let (cur_width, cur_indent) = if cur_line > linebreak.last_special_line {
        (linebreak.second_width, linebreak.second_indent)
    } else if eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE) == TEX_NULL {
        (linebreak.first_width, linebreak.first_indent)
    } else {
        let shape = eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
        (
            mem.b32s1(shape + 2 * cur_line),
            mem.b32s1(shape + 2 * cur_line - 1),
        )
    };

    scanner.adjust_tail = ADJUST_HEAD;
    scanner.pre_adjust_tail = PRE_ADJUST_HEAD;

    Ok(PostLineHpack {
        q,
        cur_width,
        cur_indent,
        semantic_pagination: config.semantic_pagination_enabled,
        disc_break,
        post_disc_break,
    })
}

fn post_line_break_suffix(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    linebreak: &mut LinebreakState,
    eqtb: &EqtbState,
    just_box: i32,
    hpack: PostLineHpack,
    d: bool,
    lr_ptr: &mut i32,
    cur_line: &mut i32,
) -> Result<bool, PostLineAbort> {
    scanner.just_box = just_box;
    let just = mem
        .classify_box(MemIdx::new(just_box))
        .expect("valid hpack box type")
        .expect("hpack returns an hlist");
    just.set_shift_amount(mem, Scaled(hpack.cur_indent));

    if PRE_ADJUST_HEAD != scanner.pre_adjust_tail {
        mem.set_b32s1(crate::globals::cur_list_tail(), mem.b32s1(PRE_ADJUST_HEAD));
        crate::globals::set_cur_list_tail(scanner.pre_adjust_tail);
    }

    scanner.pre_adjust_tail = TEX_NULL;
    crate::packaging::append_to_vlist(mem, eqtb, just).map_err(PostLineAbort::Node)?;

    if ADJUST_HEAD != scanner.adjust_tail {
        mem.set_b32s1(crate::globals::cur_list_tail(), mem.b32s1(ADJUST_HEAD));
        crate::globals::set_cur_list_tail(scanner.adjust_tail);
    }

    scanner.adjust_tail = TEX_NULL;

    if *cur_line + 1 != linebreak.best_line {
        let mut q = eqtb.b32s1(ETEX_PEN_BASE + ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES);
        let mut pen;
        if q != TEX_NULL {
            let mut r = *cur_line;
            if r > mem.b32s1(q + 1) {
                r = mem.b32s1(q + 1);
            }
            pen = mem.b32s1(q + r + 1);
        } else {
            pen = eqtb.b32s1(INT_BASE + INT_PAR_INTER_LINE_PENALTY);
        }

        q = eqtb.b32s1(ETEX_PEN_BASE + ETEX_PENALTIES_PAR_CLUB_PENALTIES);
        if q != TEX_NULL {
            let mut r = *cur_line - crate::globals::cur_list_prev_graf();
            if r > mem.b32s1(q + 1) {
                r = mem.b32s1(q + 1);
            }
            pen += mem.b32s1(q + r + 1);
        } else if *cur_line == crate::globals::cur_list_prev_graf() + 1 {
            pen += eqtb.b32s1(INT_BASE + INT_PAR_CLUB_PENALTY);
        }

        q = if d {
            eqtb.b32s1(ETEX_PEN_BASE + ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES)
        } else {
            eqtb.b32s1(ETEX_PEN_BASE + ETEX_PENALTIES_PAR_WIDOW_PENALTIES)
        };

        if q != TEX_NULL {
            let mut r = linebreak.best_line - *cur_line - 1;
            if r > mem.b32s1(q + 1) {
                r = mem.b32s1(q + 1);
            }
            pen += mem.b32s1(q + r + 1);
        } else if *cur_line + 2 == linebreak.best_line {
            if d {
                pen += eqtb.b32s1(INT_BASE + INT_PAR_DISPLAY_WIDOW_PENALTY);
            } else {
                pen += eqtb.b32s1(INT_BASE + INT_PAR_WIDOW_PENALTY);
            }
        }

        if hpack.disc_break {
            pen += eqtb.b32s1(INT_BASE + INT_PAR_BROKEN_PENALTY);
        }

        if pen != 0 {
            let r = crate::mem::new_penalty(mem, pen);
            if r == crate::mem::ALLOC_FAILED {
                return Err(PostLineAbort::Node(NodeRecycleAbort::Overflow));
            }
            mem.set_b32s1(crate::globals::cur_list_tail(), r);
            crate::globals::set_cur_list_tail(r);
        }
    }

    *cur_line += 1;
    linebreak.cur_p = passive_node(linebreak.cur_p).next_break(mem).0;

    if linebreak.cur_p != TEX_NULL && !hpack.post_disc_break {
        let cur_break = passive_node(linebreak.cur_p).cur_break(mem).0;
        let mut r = TEMP_HEAD;
        let mut q;
        loop {
            q = mem.b32s1(r);

            if q == cur_break
                || crate::mem::is_char_node(mem, q)
                || is_non_discardable_node(mem, q)
                || (mem.b16s1(q) as i32 == KERN_NODE
                    && mem.b16s0(q) != EXPLICIT
                    && mem.b16s0(q) != SPACE_ADJUSTMENT)
            {
                break;
            }

            r = q;

            if mem.b16s1(q) as i32 == MATH_NODE && eqtb.b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
                update_lr_stack_checked(mem, q, lr_ptr)?;
            }
        }

        if r != TEMP_HEAD {
            mem.set_b32s1(r, TEX_NULL);
            crate::packaging::flush_node_list(mem, mem.b32s1(TEMP_HEAD))
                .map_err(|msg| PostLineAbort::Node(NodeRecycleAbort::Confusion(msg)))?;
            mem.set_b32s1(TEMP_HEAD, q);
        }
    }

    Ok(linebreak.cur_p == TEX_NULL)
}

// translated from xetex-linebreak.c:971 (post_line_break)
/// Deferred cold-abort signal from [`post_line_break_in`] (§5): the threaded
/// core runs entirely under one `&mut EngineState` borrow, so the
/// with_state-bearing abort/print-and-die paths it would hit cannot run inline
/// — they are returned and fired by the [`post_line_break`] wrapper after the
/// borrow drops. On a passing run the outcome is always `Done`.
enum PostLineBreakOutcome {
    Done,
    Node(NodeRecycleAbort),
    PdfError {
        proc_name: &'static std::ffi::CStr,
        message: &'static std::ffi::CStr,
    },
    Confusion(&'static std::ffi::CStr),
}

pub unsafe fn post_line_break(d: bool) {
    match crate::state::with_state_mut(|st| post_line_break_in(st, d)) {
        PostLineBreakOutcome::Done => {}
        PostLineBreakOutcome::Node(e) => node_recycle_abort(e),
        PostLineBreakOutcome::PdfError { proc_name, message } => {
            crate::errors::pdf_error(proc_name.as_ptr(), message.as_ptr())
        }
        PostLineBreakOutcome::Confusion(message) => crate::errors::confusion(message.as_ptr()),
    }
}

/// Threaded core (Phase C) of [`post_line_break`]: holds one `&mut EngineState`
/// borrow across the whole per-line loop. The `hpack` packaging (and its
/// in-order over/underfull diagnostic) runs via
/// [`crate::packaging::hpack_emit_in`] under that borrow; cold aborts ride the
/// [`PostLineBreakOutcome`] sentinel.
fn post_line_break_in(st: &mut crate::state::EngineState, d: bool) -> PostLineBreakOutcome {
    let mut lr_ptr = crate::globals::cur_list_e_tex_aux();

    reverse_best_breaks(&mut st.mem, &mut st.linebreak);

    let mut cur_line = crate::globals::cur_list_prev_graf() + 1;
    loop {
        let hpack = match post_line_break_prefix(
            &mut st.mem,
            &mut st.scanner,
            &mut st.linebreak,
            &st.eqtb,
            &st.fonts,
            &st.config,
            &mut lr_ptr,
            cur_line,
        ) {
            Ok(hpack) => hpack,
            Err(PostLineAbort::Node(e)) => return PostLineBreakOutcome::Node(e),
            Err(PostLineAbort::HlistStackFailure) => {
                return PostLineBreakOutcome::PdfError {
                    proc_name: c"insert_right_margin_kern",
                    message: c"hlist stack failure",
                }
            }
        };

        let (w, m) = if hpack.semantic_pagination {
            (0, ADDITIONAL)
        } else {
            (hpack.cur_width, EXACTLY)
        };
        let just_box = match crate::packaging::hpack_emit_in(st, hpack.q, w, m) {
            Ok(b) => b,
            Err(e) => return PostLineBreakOutcome::Node(e),
        };

        let done = match post_line_break_suffix(
            &mut st.mem,
            &mut st.scanner,
            &mut st.linebreak,
            &st.eqtb,
            just_box,
            hpack,
            d,
            &mut lr_ptr,
            &mut cur_line,
        ) {
            Ok(done) => done,
            Err(PostLineAbort::Node(e)) => return PostLineBreakOutcome::Node(e),
            Err(PostLineAbort::HlistStackFailure) => {
                return PostLineBreakOutcome::PdfError {
                    proc_name: c"insert_left_margin_kern",
                    message: c"hlist stack failure",
                }
            }
        };

        if done {
            break;
        }
    }

    if cur_line != st.linebreak.best_line || st.mem.b32s1(TEMP_HEAD) != TEX_NULL {
        return PostLineBreakOutcome::Confusion(c"line breaking");
    }

    let best_line = st.linebreak.best_line;
    crate::globals::set_cur_list_prev_graf(best_line - 1);
    crate::globals::set_cur_list_e_tex_aux(lr_ptr);
    PostLineBreakOutcome::Done
}

enum TryBreakAbort {
    Node(NodeRecycleAbort),
    HlistStackFailure,
}

// translated from xetex-linebreak.c:978 (try_break)
unsafe fn try_break(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    linebreak: &mut LinebreakState,
    eqtb: &EqtbState,
    fonts: &mut FontState,
    config: &ConfigState,
    errors: &mut ErrorState,
    mut pi: i32,
    break_type: i16,
) -> Result<(), TryBreakAbort> {
    if config.semantic_pagination_enabled && linebreak.cur_p != TEX_NULL {
        return Ok(());
    }

    if pi.abs() >= INF_PENALTY {
        if pi > 0 {
            return Ok(());
        }
        pi = EJECT_PENALTY;
    }

    let mut no_break_yet = true;
    let mut prev_r = ACTIVE_LIST;
    let mut prev_prev_r = TEX_NULL;
    let mut old_l = 0;
    let mut line_width = 0;
    let mut g = 0;

    for k in 1..=6 {
        linebreak.cur_active_width[k] = scanner.active_width[k];
    }

    loop {
        let r = if prev_r == ACTIVE_LIST {
            mem.b32s1(ACTIVE_LIST)
        } else if mem.b16s1(prev_r) as i32 == DELTA_NODE {
            delta_node(prev_r).link(mem).0
        } else {
            active(prev_r).link(mem).0
        };

        if mem.b16s1(r) as i32 == DELTA_NODE {
            let r_delta = delta_node(r);
            linebreak.cur_active_width[1] += r_delta.dwidth(mem).0;
            linebreak.cur_active_width[2] += r_delta.dstretch0(mem).0;
            linebreak.cur_active_width[3] += r_delta.dstretch1(mem).0;
            linebreak.cur_active_width[4] += r_delta.dstretch2(mem).0;
            linebreak.cur_active_width[5] += r_delta.dstretch3(mem).0;
            linebreak.cur_active_width[6] += r_delta.dshrink(mem).0;
            prev_prev_r = prev_r;
            prev_r = r;
            continue;
        }

        let l = active(r).line_number(mem);
        if l > old_l {
            if linebreak.minimum_demerits < AWFUL_BAD
                && (old_l != linebreak.easy_line || r == LAST_ACTIVE)
            {
                if no_break_yet {
                    no_break_yet = false;
                    for k in 1..=6 {
                        linebreak.break_width[k] = linebreak.background[k];
                    }
                    let mut s = linebreak.cur_p;

                    if break_type > UNHYPHENATED && linebreak.cur_p != TEX_NULL {
                        let cur_disc = disc_node(linebreak.cur_p);
                        let mut t = cur_disc.subtype(mem) as i32;
                        let mut v = linebreak.cur_p;
                        s = cur_disc.post_break(mem).0;

                        while t > 0 {
                            t -= 1;
                            v = mem.b32s1(v);
                            if crate::mem::is_char_node(mem, v) {
                                let f = mem.b16s1(v) as i32;
                                linebreak.break_width[1] -=
                                    font_character_width(fonts, f, mem.b16s0(v));
                            } else {
                                match mem.b16s1(v) as i32 {
                                    LIGATURE_NODE => {
                                        let lig = ligature_node(v);
                                        let f = lig.font(mem).0;
                                        fonts.set_xtx_ligature_present(true);
                                        linebreak.break_width[1] -=
                                            font_character_width(fonts, f, lig.character(mem));
                                    }
                                    HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                                        linebreak.break_width[1] -= typed_list_width_in(mem, v);
                                    }
                                    WHATSIT_NODE => {
                                        let sub = mem.b16s0(v);
                                        if sub == NATIVE_WORD_NODE
                                            || sub == NATIVE_WORD_NODE_AT
                                            || sub == GLYPH_NODE
                                            || sub == PIC_NODE
                                            || sub == PDF_NODE
                                        {
                                            linebreak.break_width[1] -= mem.b32s1(v + 1);
                                        } else {
                                            return Err(TryBreakAbort::Node(
                                                NodeRecycleAbort::Confusion(c"disc1a"),
                                            ));
                                        }
                                    }
                                    _ => {
                                        return Err(TryBreakAbort::Node(
                                            NodeRecycleAbort::Confusion(c"disc1"),
                                        ));
                                    }
                                }
                            }
                        }

                        while s != TEX_NULL {
                            if crate::mem::is_char_node(mem, s) {
                                let f = mem.b16s1(s) as i32;
                                linebreak.break_width[1] +=
                                    font_character_width(fonts, f, mem.b16s0(s));
                            } else {
                                match mem.b16s1(s) as i32 {
                                    LIGATURE_NODE => {
                                        let lig = ligature_node(s);
                                        let f = lig.font(mem).0;
                                        fonts.set_xtx_ligature_present(true);
                                        linebreak.break_width[1] +=
                                            font_character_width(fonts, f, lig.character(mem));
                                    }
                                    HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                                        linebreak.break_width[1] += typed_list_width_in(mem, s);
                                    }
                                    WHATSIT_NODE => {
                                        let sub = mem.b16s0(s);
                                        if sub == NATIVE_WORD_NODE
                                            || sub == NATIVE_WORD_NODE_AT
                                            || sub == GLYPH_NODE
                                            || sub == PIC_NODE
                                            || sub == PDF_NODE
                                        {
                                            linebreak.break_width[1] += mem.b32s1(s + 1);
                                        } else {
                                            return Err(TryBreakAbort::Node(
                                                NodeRecycleAbort::Confusion(c"disc2a"),
                                            ));
                                        }
                                    }
                                    _ => {
                                        return Err(TryBreakAbort::Node(
                                            NodeRecycleAbort::Confusion(c"disc2"),
                                        ));
                                    }
                                }
                            }

                            s = mem.b32s1(s);
                        }

                        linebreak.break_width[1] += linebreak.disc_width;
                        if cur_disc.post_break(mem).0 == TEX_NULL {
                            s = mem.b32s1(v);
                        }
                    }

                    while s != TEX_NULL {
                        if crate::mem::is_char_node(mem, s) {
                            break;
                        }

                        match mem.b16s1(s) as i32 {
                            GLUE_NODE => {
                                let v = glue_node(s).glue_spec(mem).expect("glue node has spec");
                                linebreak.break_width[1] -= v.width(mem).0;
                                linebreak.break_width[2 + v.stretch_order(mem) as usize] -=
                                    v.stretch(mem).0;
                                linebreak.break_width[6] -= v.shrink(mem).0;
                            }
                            PENALTY_NODE => {}
                            MATH_NODE => {
                                linebreak.break_width[1] -= math_node(s).width(mem).0;
                            }
                            KERN_NODE => {
                                let kern = kern_node(s);
                                if kern.subtype(mem) != EXPLICIT {
                                    break;
                                }
                                linebreak.break_width[1] -= kern.width(mem).0;
                            }
                            _ => break,
                        }

                        s = mem.b32s1(s);
                    }
                }

                if mem.b16s1(prev_r) as i32 == DELTA_NODE {
                    let prev_delta = delta_node(prev_r);
                    prev_delta.set_dwidth(
                        mem,
                        Scaled(
                            prev_delta.dwidth(mem).0 - linebreak.cur_active_width[1]
                                + linebreak.break_width[1],
                        ),
                    );
                    prev_delta.set_dstretch0(
                        mem,
                        Scaled(
                            prev_delta.dstretch0(mem).0 - linebreak.cur_active_width[2]
                                + linebreak.break_width[2],
                        ),
                    );
                    prev_delta.set_dstretch1(
                        mem,
                        Scaled(
                            prev_delta.dstretch1(mem).0 - linebreak.cur_active_width[3]
                                + linebreak.break_width[3],
                        ),
                    );
                    prev_delta.set_dstretch2(
                        mem,
                        Scaled(
                            prev_delta.dstretch2(mem).0 - linebreak.cur_active_width[4]
                                + linebreak.break_width[4],
                        ),
                    );
                    prev_delta.set_dstretch3(
                        mem,
                        Scaled(
                            prev_delta.dstretch3(mem).0 - linebreak.cur_active_width[5]
                                + linebreak.break_width[5],
                        ),
                    );
                    prev_delta.set_dshrink(
                        mem,
                        Scaled(
                            prev_delta.dshrink(mem).0 - linebreak.cur_active_width[6]
                                + linebreak.break_width[6],
                        ),
                    );
                } else if prev_r == ACTIVE_LIST {
                    for k in 1..=6 {
                        scanner.active_width[k] = linebreak.break_width[k];
                    }
                } else {
                    let q =
                        alloc_delta_node(mem, MemPtr::from_raw(r)).map_err(TryBreakAbort::Node)?;
                    q.set_dwidth(
                        mem,
                        Scaled(linebreak.break_width[1] - linebreak.cur_active_width[1]),
                    );
                    q.set_dstretch0(
                        mem,
                        Scaled(linebreak.break_width[2] - linebreak.cur_active_width[2]),
                    );
                    q.set_dstretch1(
                        mem,
                        Scaled(linebreak.break_width[3] - linebreak.cur_active_width[3]),
                    );
                    q.set_dstretch2(
                        mem,
                        Scaled(linebreak.break_width[4] - linebreak.cur_active_width[4]),
                    );
                    q.set_dstretch3(
                        mem,
                        Scaled(linebreak.break_width[5] - linebreak.cur_active_width[5]),
                    );
                    q.set_dshrink(
                        mem,
                        Scaled(linebreak.break_width[6] - linebreak.cur_active_width[6]),
                    );
                    active(prev_r).set_link(mem, MemPtr::from_raw(q.idx().0));
                    prev_prev_r = prev_r;
                    prev_r = q.idx().0;
                }

                let adj = eqtb.b32s1(INT_BASE + INT_PAR_ADJ_DEMERITS).abs();
                if adj >= MAX_HALFWORD - linebreak.minimum_demerits {
                    linebreak.minimum_demerits = AWFUL_BAD - 1;
                } else {
                    linebreak.minimum_demerits += adj;
                }

                for fit_class in VERY_LOOSE_FIT..=TIGHT_FIT {
                    if linebreak.minimal_demerits[fit_class] <= linebreak.minimum_demerits {
                        let passive_p = alloc_passive_node(mem).map_err(TryBreakAbort::Node)?;
                        passive_p.set_link(mem, MemPtr::from_raw(linebreak.passive));
                        linebreak.passive = passive_p.idx().0;
                        passive_p.set_cur_break(mem, MemPtr::from_raw(linebreak.cur_p));
                        passive_p
                            .set_prev_break(mem, MemPtr::from_raw(linebreak.best_place[fit_class]));

                        let q_active = alloc_active_node(
                            mem,
                            linebreak.active_node_size as i32,
                            MemPtr::from_raw(r),
                        )
                        .map_err(TryBreakAbort::Node)?;
                        q_active.set_break_node(mem, MemPtr::from_raw(linebreak.passive));
                        q_active.set_line_number(mem, linebreak.best_pl_line[fit_class] + 1);
                        q_active.set_fitness(mem, fit_class as u16);
                        q_active.set_break_type(mem, break_type as u16);
                        q_active.set_total_demerits(mem, linebreak.minimal_demerits[fit_class]);

                        if linebreak.do_last_line_fit {
                            q_active.set_shortfall(mem, Scaled(linebreak.best_pl_short[fit_class]));
                            q_active.set_glue(mem, Scaled(linebreak.best_pl_glue[fit_class]));
                        }

                        if mem.b16s1(prev_r) as i32 == DELTA_NODE {
                            delta_node(prev_r).set_link(mem, MemPtr::from_raw(q_active.idx().0));
                        } else {
                            active(prev_r).set_link(mem, MemPtr::from_raw(q_active.idx().0));
                        }
                        prev_r = q_active.idx().0;
                    }

                    linebreak.minimal_demerits[fit_class] = MAX_HALFWORD;
                }

                linebreak.minimum_demerits = MAX_HALFWORD;

                if r != LAST_ACTIVE {
                    let q =
                        alloc_delta_node(mem, MemPtr::from_raw(r)).map_err(TryBreakAbort::Node)?;
                    q.set_dwidth(
                        mem,
                        Scaled(linebreak.cur_active_width[1] - linebreak.break_width[1]),
                    );
                    q.set_dstretch0(
                        mem,
                        Scaled(linebreak.cur_active_width[2] - linebreak.break_width[2]),
                    );
                    q.set_dstretch1(
                        mem,
                        Scaled(linebreak.cur_active_width[3] - linebreak.break_width[3]),
                    );
                    q.set_dstretch2(
                        mem,
                        Scaled(linebreak.cur_active_width[4] - linebreak.break_width[4]),
                    );
                    q.set_dstretch3(
                        mem,
                        Scaled(linebreak.cur_active_width[5] - linebreak.break_width[5]),
                    );
                    q.set_dshrink(
                        mem,
                        Scaled(linebreak.cur_active_width[6] - linebreak.break_width[6]),
                    );
                    if mem.b16s1(prev_r) as i32 == DELTA_NODE {
                        delta_node(prev_r).set_link(mem, MemPtr::from_raw(q.idx().0));
                    } else {
                        active(prev_r).set_link(mem, MemPtr::from_raw(q.idx().0));
                    }
                    prev_prev_r = prev_r;
                    prev_r = q.idx().0;
                }
            }

            if r == LAST_ACTIVE {
                return Ok(());
            }

            if l > linebreak.easy_line {
                line_width = linebreak.second_width;
                old_l = MAX_HALFWORD - 1;
            } else {
                old_l = l;
                if l > linebreak.last_special_line {
                    line_width = linebreak.second_width;
                } else if eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE) == TEX_NULL {
                    line_width = linebreak.first_width;
                } else {
                    line_width = mem.b32s1(eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE) + 2 * l);
                }
            }
        }

        let mut artificial_demerits;
        let mut shortfall;
        if config.semantic_pagination_enabled {
            line_width = linebreak.cur_active_width[1];
            artificial_demerits = true;
            shortfall = 0;
        } else {
            artificial_demerits = false;
            shortfall = line_width - linebreak.cur_active_width[1];
            if eqtb.b32s1(INT_BASE + INT_PAR_XETEX_PROTRUDE_CHARS) > 1 {
                shortfall += total_pw(mem, fonts, linebreak, r, linebreak.cur_p)
                    .ok_or(TryBreakAbort::HlistStackFailure)?;
            }
        }

        let mut b = 0;
        let mut fit_class = DECENT_FIT;
        let mut jumped_to_found = false;

        if shortfall > 0 {
            if linebreak.cur_active_width[3] != 0
                || linebreak.cur_active_width[4] != 0
                || linebreak.cur_active_width[5] != 0
            {
                let mut last_line_fit_found = false;
                let r_active = active(r);
                if linebreak.do_last_line_fit
                    && linebreak.cur_p == TEX_NULL
                    && r_active.shortfall(mem).0 != 0
                    && r_active.glue(mem).0 > 0
                    && linebreak.cur_active_width[3] == linebreak.fill_width[0]
                    && linebreak.cur_active_width[4] == linebreak.fill_width[1]
                    && linebreak.cur_active_width[5] == linebreak.fill_width[2]
                {
                    g = if r_active.shortfall(mem).0 > 0 {
                        linebreak.cur_active_width[2]
                    } else {
                        linebreak.cur_active_width[6]
                    };

                    if g > 0 {
                        errors.arith_error = false;
                        g = crate::scaled_math::fract(
                            g,
                            r_active.shortfall(mem).0,
                            r_active.glue(mem).0,
                            MAX_HALFWORD,
                        );
                        if eqtb.b32s1(INT_BASE + INT_PAR_LAST_LINE_FIT) < 1000 {
                            g = crate::scaled_math::fract(
                                g,
                                eqtb.b32s1(INT_BASE + INT_PAR_LAST_LINE_FIT),
                                1000,
                                MAX_HALFWORD,
                            );
                        }
                        if errors.arith_error {
                            g = if r_active.shortfall(mem).0 > 0 {
                                MAX_HALFWORD
                            } else {
                                -MAX_HALFWORD
                            };
                        }

                        if g > 0 {
                            if g > shortfall {
                                g = shortfall;
                            }
                            if g > 7_230_584 && linebreak.cur_active_width[2] < 1_663_497 {
                                b = INF_BAD;
                                fit_class = VERY_LOOSE_FIT;
                            } else {
                                b = crate::helpers::badness(g, linebreak.cur_active_width[2]);
                                fit_class = if b > 12 {
                                    if b > 99 {
                                        VERY_LOOSE_FIT
                                    } else {
                                        LOOSE_FIT
                                    }
                                } else {
                                    DECENT_FIT
                                };
                            }
                            last_line_fit_found = true;
                            jumped_to_found = true;
                        } else if g < 0 {
                            if -g > linebreak.cur_active_width[6] {
                                g = -linebreak.cur_active_width[6];
                            }
                            b = crate::helpers::badness(-g, linebreak.cur_active_width[6]);
                            fit_class = if b > 12 { TIGHT_FIT } else { DECENT_FIT };
                            last_line_fit_found = true;
                            jumped_to_found = true;
                        }
                    }
                }

                if !last_line_fit_found {
                    shortfall = 0;
                    b = 0;
                    fit_class = DECENT_FIT;
                }
            } else if shortfall > 7_230_584 && linebreak.cur_active_width[2] < 1_663_497 {
                b = INF_BAD;
                fit_class = VERY_LOOSE_FIT;
            } else {
                b = crate::helpers::badness(shortfall, linebreak.cur_active_width[2]);
                fit_class = if b > 12 {
                    if b > 99 {
                        VERY_LOOSE_FIT
                    } else {
                        LOOSE_FIT
                    }
                } else {
                    DECENT_FIT
                };
            }
        } else {
            if -shortfall > linebreak.cur_active_width[6] {
                b = INF_BAD + 1;
            } else {
                b = crate::helpers::badness(-shortfall, linebreak.cur_active_width[6]);
            }
            fit_class = if b > 12 { TIGHT_FIT } else { DECENT_FIT };
        }

        if linebreak.do_last_line_fit && !jumped_to_found {
            if linebreak.cur_p == TEX_NULL {
                shortfall = 0;
            }
            g = if shortfall > 0 {
                linebreak.cur_active_width[2]
            } else if shortfall < 0 {
                linebreak.cur_active_width[6]
            } else {
                0
            };
        }

        let node_r_stays_active;
        if b > INF_BAD || pi == EJECT_PENALTY {
            if linebreak.final_pass
                && linebreak.minimum_demerits == AWFUL_BAD
                && active(r).link(mem).0 == LAST_ACTIVE
                && prev_r == ACTIVE_LIST
            {
                artificial_demerits = true;
            } else if b > linebreak.threshold {
                deactivate_node(mem, scanner, linebreak, r, &mut prev_r, &mut prev_prev_r);
                continue;
            }
            node_r_stays_active = false;
        } else {
            prev_r = r;
            if b > linebreak.threshold {
                continue;
            }
            node_r_stays_active = true;
        }

        let mut d = if artificial_demerits {
            0
        } else {
            let mut d = eqtb.b32s1(INT_BASE + INT_PAR_LINE_PENALTY) + b;
            if d.abs() >= 10_000 {
                d = 100_000_000;
            } else {
                d *= d;
            }

            if pi != 0 {
                if pi > 0 {
                    d += pi * pi;
                } else if pi > EJECT_PENALTY {
                    d -= pi * pi;
                }
            }

            if break_type == HYPHENATED && active(r).break_type(mem) as i16 == HYPHENATED {
                if linebreak.cur_p != TEX_NULL {
                    d += eqtb.b32s1(INT_BASE + INT_PAR_DOUBLE_HYPHEN_DEMERITS);
                } else {
                    d += eqtb.b32s1(INT_BASE + INT_PAR_FINAL_HYPHEN_DEMERITS);
                }
            }

            if ((fit_class as i32) - active(r).fitness(mem) as i32).abs() > 1 {
                d += eqtb.b32s1(INT_BASE + INT_PAR_ADJ_DEMERITS);
            }

            d
        };

        d += active(r).total_demerits(mem);
        if d <= linebreak.minimal_demerits[fit_class] {
            linebreak.minimal_demerits[fit_class] = d;
            linebreak.best_place[fit_class] = active(r).break_node(mem).0;
            linebreak.best_pl_line[fit_class] = l;

            if linebreak.do_last_line_fit {
                linebreak.best_pl_short[fit_class] = shortfall;
                linebreak.best_pl_glue[fit_class] = g;
            }

            if d < linebreak.minimum_demerits {
                linebreak.minimum_demerits = d;
            }
        }

        if node_r_stays_active {
            continue;
        }

        deactivate_node(mem, scanner, linebreak, r, &mut prev_r, &mut prev_prev_r);
    }

    fn deactivate_node(
        mem: &mut MemState,
        scanner: &mut ScannerState,
        linebreak: &mut LinebreakState,
        r: i32,
        prev_r: &mut i32,
        prev_prev_r: &mut i32,
    ) {
        if *prev_r == ACTIVE_LIST {
            mem.set_b32s1(ACTIVE_LIST, active(r).link(mem).0);
        } else if mem.b16s1(*prev_r) as i32 == DELTA_NODE {
            delta_node(*prev_r).set_link(mem, active(r).link(mem));
        } else {
            active(*prev_r).set_link(mem, active(r).link(mem));
        }
        crate::mem::free_node(mem, r, linebreak.active_node_size as i32);

        if *prev_r == ACTIVE_LIST {
            let r = mem.b32s1(ACTIVE_LIST);
            if mem.b16s1(r) as i32 == DELTA_NODE {
                let r_delta = delta_node(r);
                scanner.active_width[1] += r_delta.dwidth(mem).0;
                scanner.active_width[2] += r_delta.dstretch0(mem).0;
                scanner.active_width[3] += r_delta.dstretch1(mem).0;
                scanner.active_width[4] += r_delta.dstretch2(mem).0;
                scanner.active_width[5] += r_delta.dstretch3(mem).0;
                scanner.active_width[6] += r_delta.dshrink(mem).0;
                linebreak.cur_active_width[1] = scanner.active_width[1];
                linebreak.cur_active_width[2] = scanner.active_width[2];
                linebreak.cur_active_width[3] = scanner.active_width[3];
                linebreak.cur_active_width[4] = scanner.active_width[4];
                linebreak.cur_active_width[5] = scanner.active_width[5];
                linebreak.cur_active_width[6] = scanner.active_width[6];
                mem.set_b32s1(ACTIVE_LIST, r_delta.link(mem).0);
                crate::mem::free_node(mem, r, DELTA_NODE_SIZE);
            }
        } else if mem.b16s1(*prev_r) as i32 == DELTA_NODE {
            let prev_delta = delta_node(*prev_r);
            let r = prev_delta.link(mem).0;
            if r == LAST_ACTIVE {
                linebreak.cur_active_width[1] -= prev_delta.dwidth(mem).0;
                linebreak.cur_active_width[2] -= prev_delta.dstretch0(mem).0;
                linebreak.cur_active_width[3] -= prev_delta.dstretch1(mem).0;
                linebreak.cur_active_width[4] -= prev_delta.dstretch2(mem).0;
                linebreak.cur_active_width[5] -= prev_delta.dstretch3(mem).0;
                linebreak.cur_active_width[6] -= prev_delta.dshrink(mem).0;
                if *prev_prev_r == ACTIVE_LIST {
                    mem.set_b32s1(ACTIVE_LIST, LAST_ACTIVE);
                } else {
                    active(*prev_prev_r).set_link(mem, MemPtr::from_raw(LAST_ACTIVE));
                }
                crate::mem::free_node(mem, *prev_r, DELTA_NODE_SIZE);
                *prev_r = *prev_prev_r;
            } else if mem.b16s1(r) as i32 == DELTA_NODE {
                let r_delta = delta_node(r);
                linebreak.cur_active_width[1] += r_delta.dwidth(mem).0;
                linebreak.cur_active_width[2] += r_delta.dstretch0(mem).0;
                linebreak.cur_active_width[3] += r_delta.dstretch1(mem).0;
                linebreak.cur_active_width[4] += r_delta.dstretch2(mem).0;
                linebreak.cur_active_width[5] += r_delta.dstretch3(mem).0;
                linebreak.cur_active_width[6] += r_delta.dshrink(mem).0;
                prev_delta.set_dwidth(
                    mem,
                    Scaled(prev_delta.dwidth(mem).0 + r_delta.dwidth(mem).0),
                );
                prev_delta.set_dstretch0(
                    mem,
                    Scaled(prev_delta.dstretch0(mem).0 + r_delta.dstretch0(mem).0),
                );
                prev_delta.set_dstretch1(
                    mem,
                    Scaled(prev_delta.dstretch1(mem).0 + r_delta.dstretch1(mem).0),
                );
                prev_delta.set_dstretch2(
                    mem,
                    Scaled(prev_delta.dstretch2(mem).0 + r_delta.dstretch2(mem).0),
                );
                prev_delta.set_dstretch3(
                    mem,
                    Scaled(prev_delta.dstretch3(mem).0 + r_delta.dstretch3(mem).0),
                );
                prev_delta.set_dshrink(
                    mem,
                    Scaled(prev_delta.dshrink(mem).0 + r_delta.dshrink(mem).0),
                );
                prev_delta.set_link(mem, r_delta.link(mem));
                crate::mem::free_node(mem, r, DELTA_NODE_SIZE);
            }
        }
    }
}

pub unsafe fn try_break_global(pi: i32, break_type: i16) {
    match crate::state::with_state_mut(|st| {
        let st = &mut *st;
        try_break(
            &mut st.mem,
            &mut st.scanner,
            &mut st.linebreak,
            &st.eqtb,
            &mut st.fonts,
            &st.config,
            &mut st.errors,
            pi,
            break_type,
        )
    }) {
        Ok(()) => {}
        Err(TryBreakAbort::Node(e)) => node_recycle_abort(e),
        Err(TryBreakAbort::HlistStackFailure) => {
            crate::errors::pdf_error(c"total_pw".as_ptr(), c"hlist stack failure".as_ptr())
        }
    }
}

// translated from xetex-linebreak.c:2010 (hyphenate)
/// Cold-abort sentinel (§5) for the threaded native-word hyphenation island in
/// [`hyphenate`]'s `goto_found`: the island returns this and the orchestrator
/// runs the divergent tail after the state borrow drops.
enum HyphNativeAbort {
    Recycle(NodeRecycleAbort),
    NativeChar(crate::fonts::NativeCharAbort),
    Confusion(&'static std::ffi::CStr),
}

fn hyphenate_native_word_if_needed(
    st: &mut EngineState,
    hn_v: i16,
) -> Result<bool, HyphNativeAbort> {
    let ha = st.linebreak.ha;
    let is_native = ha != TEX_NULL
        && !crate::mem::is_char_node(&st.mem, ha)
        && st.mem.b16s1(ha) as i32 == WHATSIT_NODE
        && (st.mem.b16s0(ha) == NATIVE_WORD_NODE || st.mem.b16s0(ha) == NATIVE_WORD_NODE_AT);
    if !is_native {
        return Ok(false);
    }

    let hf = st.hyph.hf;
    let hyf_char = st.hyph.hyf_char;
    let l_hyf = st.hyph.l_hyf;
    let r_hyf = st.hyph.r_hyf;

    let mut s = st.linebreak.cur_p;
    while st.mem.b32s1(s) != ha {
        s = st.mem.b32s1(s);
    }

    st.hyph.hyphen_passed = 0;
    for j in l_hyf as i16..=(hn_v - r_hyf as i16) {
        if odd(st.hyph.hyf[j as usize]) {
            let hp = st.hyph.hyphen_passed;
            let q = crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, hf, (j - hp) as i32)
                .map_err(HyphNativeAbort::Recycle)?;
            let subtype = st.mem.b16s0(ha);
            st.mem.set_b16s0(q, subtype);
            // SAFETY: both nodes are sized for the copied UTF-16 range;
            // the mem arena never reallocates.
            unsafe {
                let dst = st.mem.as_u16_ptr_mut(q + NATIVE_NODE_SIZE);
                let src = st.mem.as_u16_ptr(ha + NATIVE_NODE_SIZE);
                for i in 0..=(j - hp - 1) {
                    *dst.add(i as usize) = *src.add((i + hp) as usize);
                }
                set_native_metrics(&st.mem, &st.eqtb, &st.fonts, q);
            }
            st.mem.set_b32s1(s, q);
            s = q;

            let q = crate::mem::new_disc(&mut st.mem);
            let nc = crate::fonts::new_native_character_in(st, hf, hyf_char as u32)
                .map_err(HyphNativeAbort::NativeChar)?;
            st.mem.set_b32s0(q + 1, nc);
            st.mem.set_b32s1(s, q);
            s = q;
            st.hyph.hyphen_passed = j;
        }
    }

    let new_hn = st.mem.b16s1(ha + 4) as i16;
    st.linebreak.hn = new_hn;
    let hp = st.hyph.hyphen_passed;
    let q = crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, hf, (new_hn - hp) as i32)
        .map_err(HyphNativeAbort::Recycle)?;
    let subtype = st.mem.b16s0(ha);
    st.mem.set_b16s0(q, subtype);
    // SAFETY: see above.
    unsafe {
        let dst = st.mem.as_u16_ptr_mut(q + NATIVE_NODE_SIZE);
        let src = st.mem.as_u16_ptr(ha + NATIVE_NODE_SIZE);
        for i in 0..=(new_hn - hp - 1) {
            *dst.add(i as usize) = *src.add((i + hp) as usize);
        }
        set_native_metrics(&st.mem, &st.eqtb, &st.fonts, q);
    }
    st.mem.set_b32s1(s, q);
    s = q;
    let after = st.mem.b32s1(ha);
    st.mem.set_b32s1(s, after);
    st.mem.set_b32s1(ha, TEX_NULL);
    crate::packaging::flush_node_list(&mut st.mem, ha).map_err(HyphNativeAbort::Confusion)?;
    Ok(true)
}

pub unsafe fn hyphenate() {
    use crate::globals::{
        cur_lang, hc_entry, hn, hyf_distance_entry, hyf_entry, hyf_next_entry, hyf_num_entry,
        hyph_link_entry, hyph_list_entry, hyph_word_entry, max_hyph_char, mem_b32s0, mem_b32s1,
        op_start_entry, r_hyf, set_hc_entry, set_hn, set_hyf_entry, str_pool_entry, str_start,
        trie_trc_entry, trie_trl_entry, trie_tro_entry,
    };

    let mut hn_v = hn();
    for j in 0..=hn_v {
        set_hyf_entry(j as usize, 0);
    }

    let mut h = hc_entry(1);
    hn_v += 1;
    set_hn(hn_v);
    set_hc_entry(hn_v as usize, cur_lang() as i32);

    for j in 2..=hn_v {
        h = (h + h + hc_entry(j as usize)) % HYPH_PRIME;
    }

    loop {
        let k = hyph_word_entry(h);
        if k == 0 {
            break;
        }

        if crate::string_pool::length_global(k) == hn_v as i32 {
            let mut j = 1;
            let mut u = str_start(k - TOO_BIG_CHAR);
            loop {
                if str_pool_entry(u) as i32 != hc_entry(j as usize) {
                    break;
                }
                j += 1;
                u += 1;
                if j > hn_v {
                    let mut s = hyph_list_entry(h);
                    while s != TEX_NULL {
                        set_hyf_entry(mem_b32s0(s) as usize, 1);
                        s = mem_b32s1(s);
                    }
                    hn_v -= 1;
                    set_hn(hn_v);
                    goto_found(hn_v);
                    return;
                }
            }
        }

        let link = hyph_link_entry(h);
        if link == 0 {
            break;
        }
        h = link as i32 - 1;
    }

    hn_v -= 1;
    set_hn(hn_v);
    if trie_trc_entry(cur_lang() as i32 + 1) != cur_lang() as u16 {
        return;
    }

    set_hc_entry(0, 0);
    set_hc_entry((hn_v + 1) as usize, 0);
    set_hc_entry((hn_v + 2) as usize, max_hyph_char());

    for j in 0..=(hn_v - r_hyf() as i16 + 1) {
        let mut z = trie_trl_entry(cur_lang() as i32 + 1) + hc_entry(j as usize);
        let mut l = j;
        while hc_entry(l as usize) == trie_trc_entry(z) as i32 {
            if trie_tro_entry(z) != MIN_TRIE_OP {
                let mut v = trie_tro_entry(z);
                loop {
                    v += op_start_entry(cur_lang() as usize);
                    let i = l - hyf_distance_entry(v);
                    if hyf_num_entry(v) as u8 > hyf_entry(i as usize) {
                        set_hyf_entry(i as usize, hyf_num_entry(v) as u8);
                    }
                    v = hyf_next_entry(v) as i32;
                    if v == MIN_TRIE_OP {
                        break;
                    }
                }
            }
            l += 1;
            z = trie_trl_entry(z) + hc_entry(l as usize);
        }
    }

    goto_found(hn_v);

    unsafe fn goto_found(hn_v: i16) {
        use crate::globals::{
            avail, bchar_label_entry, cur_p, font_bchar_entry, ha, hb, hf, hu_entry, hyf_bchar,
            hyf_char, hyf_entry, hyphen_passed, l_hyf, max_hyph_char, mem_b16s0, mem_b16s1,
            mem_b32s1, r_hyf, set_avail, set_hu_entry, set_hyf_entry, set_hyphen_passed,
            set_init_lft, set_init_lig, set_init_list, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0,
            set_mem_b32s1,
        };

        for j in 0..=(l_hyf() as i16 - 1) {
            set_hyf_entry(j as usize, 0);
        }
        for j in 0..=(r_hyf() as i16 - 1) {
            set_hyf_entry((hn_v - j) as usize, 0);
        }

        let mut any_hyphen = false;
        for j in l_hyf() as i16..=(hn_v - r_hyf() as i16) {
            if odd(hyf_entry(j as usize)) {
                any_hyphen = true;
                break;
            }
        }
        if !any_hyphen {
            return;
        }

        // Native-word hyphenation: split `ha`'s native_word_node into measured
        // fragments separated by discretionaries whose pre-break is a fresh
        // native hyphen character. Threaded into a single `with_state_mut` island
        // now that `new_native_character_in` no longer re-enters the state
        // RefCell (the 135/136 measurer wall is cleared): `new_native_word_node`,
        // `set_native_metrics`, `new_disc`, `new_native_character_in` and
        // `flush_node_list` all run under one borrow, in the original allocation
        // order. Cold aborts ride `HyphNativeAbort` and fire after the borrow
        // drops (§5).
        let native_result =
            crate::state::with_state_mut(|st| hyphenate_native_word_if_needed(st, hn_v));
        match native_result {
            Ok(true) => return,
            Ok(false) => {}
            Err(HyphNativeAbort::Recycle(e)) => node_recycle_abort(e),
            Err(HyphNativeAbort::NativeChar(nc)) => nc.finish(),
            Err(HyphNativeAbort::Confusion(msg)) => crate::errors::confusion(msg.as_ptr()),
        }

        let q_final = mem_b32s1(hb());
        set_mem_b32s1(hb(), TEX_NULL);
        let r = mem_b32s1(ha());
        set_mem_b32s1(ha(), TEX_NULL);
        let bchar = hyf_bchar();

        let (mut s, mut j) = if is_char_node_global(ha()) {
            if mem_b16s1(ha()) as i32 != hf() {
                set_hu_entry(0, max_hyph_char());
                set_init_lig(false);
                set_init_list(TEX_NULL);
                (ha(), 0)
            } else {
                set_init_list(ha());
                set_init_lig(false);
                set_hu_entry(0, mem_b16s0(ha()) as i32);
                let mut s = cur_p();
                while mem_b32s1(s) != ha() {
                    s = mem_b32s1(s);
                }
                (s, 0)
            }
        } else if mem_b16s1(ha()) as i32 == LIGATURE_NODE {
            if mem_b16s1(ha() + 1) as i32 != hf() {
                set_hu_entry(0, max_hyph_char());
                set_init_lig(false);
                set_init_list(TEX_NULL);
                (ha(), 0)
            } else {
                set_init_list(mem_b32s1(ha() + 1));
                set_init_lig(true);
                set_init_lft(mem_b16s0(ha()) > 1);
                set_hu_entry(0, mem_b16s0(ha() + 1) as i32);
                if crate::globals::init_list() == TEX_NULL && crate::globals::init_lft() {
                    set_hu_entry(0, max_hyph_char());
                    set_init_lig(false);
                }
                crate::mem::free_node_global(ha(), SMALL_NODE_SIZE);
                let mut s = cur_p();
                while mem_b32s1(s) != ha() {
                    s = mem_b32s1(s);
                }
                (s, 0)
            }
        } else if !is_char_node_global(r)
            && mem_b16s1(r) as i32 == LIGATURE_NODE
            && mem_b16s0(r) > 1
        {
            set_hu_entry(0, max_hyph_char());
            set_init_lig(false);
            set_init_list(TEX_NULL);
            (ha(), 0)
        } else {
            set_init_list(TEX_NULL);
            (ha(), 1)
        };

        crate::packaging::flush_node_list_global(r);

        loop {
            let mut l = j;
            j = reconstitute_global(j, hn_v, bchar, hyf_char()) + 1;
            if hyphen_passed() == 0 {
                set_mem_b32s1(s, mem_b32s1(HOLD_HEAD));
                while mem_b32s1(s) > TEX_NULL {
                    s = mem_b32s1(s);
                }
                if odd(hyf_entry((j - 1) as usize)) {
                    l = j;
                    set_hyphen_passed(j - 1);
                    set_mem_b32s1(HOLD_HEAD, TEX_NULL);
                }
            }

            if hyphen_passed() > 0 {
                loop {
                    let r = crate::mem::get_node_global(SMALL_NODE_SIZE);
                    set_mem_b32s1(r, mem_b32s1(HOLD_HEAD));
                    set_mem_b16s1(r, DISC_NODE as u16);
                    let mut major_tail = r;
                    let mut r_count = 0;
                    while mem_b32s1(major_tail) > TEX_NULL {
                        major_tail = mem_b32s1(major_tail);
                        r_count += 1;
                    }

                    let mut i = hyphen_passed();
                    set_hyf_entry(i as usize, 0);
                    let mut minor_tail = TEX_NULL;
                    set_mem_b32s0(r + 1, TEX_NULL);
                    let hyf_node = crate::main_control::new_character(hf(), hyf_char() as u16);
                    let mut saved_c = 0;
                    if hyf_node != TEX_NULL {
                        i += 1;
                        saved_c = hu_entry(i as usize);
                        set_hu_entry(i as usize, hyf_char());
                        set_mem_b32s1(hyf_node, avail());
                        set_avail(hyf_node);
                    }

                    while l <= i {
                        l = reconstitute_global(l, i, font_bchar_entry(hf()), TOO_BIG_CHAR) + 1;
                        if mem_b32s1(HOLD_HEAD) > TEX_NULL {
                            if minor_tail == TEX_NULL {
                                set_mem_b32s0(r + 1, mem_b32s1(HOLD_HEAD));
                            } else {
                                set_mem_b32s1(minor_tail, mem_b32s1(HOLD_HEAD));
                            }
                            minor_tail = mem_b32s1(HOLD_HEAD);
                            while mem_b32s1(minor_tail) > TEX_NULL {
                                minor_tail = mem_b32s1(minor_tail);
                            }
                        }
                    }

                    if hyf_node != TEX_NULL {
                        set_hu_entry(i as usize, saved_c);
                        l = i;
                        i -= 1;
                    }

                    minor_tail = TEX_NULL;
                    set_mem_b32s1(r + 1, TEX_NULL);
                    let mut c_loc = 0;
                    let mut c = 0;
                    if bchar_label_entry(hf()) != NON_ADDRESS {
                        l -= 1;
                        c = hu_entry(l as usize);
                        c_loc = l;
                        set_hu_entry(l as usize, max_hyph_char());
                    }

                    while l < j {
                        loop {
                            l = reconstitute_global(l, hn_v, bchar, TOO_BIG_CHAR) + 1;
                            if c_loc > 0 {
                                set_hu_entry(c_loc as usize, c);
                                c_loc = 0;
                            }
                            if mem_b32s1(HOLD_HEAD) > TEX_NULL {
                                if minor_tail == TEX_NULL {
                                    set_mem_b32s1(r + 1, mem_b32s1(HOLD_HEAD));
                                } else {
                                    set_mem_b32s1(minor_tail, mem_b32s1(HOLD_HEAD));
                                }
                                minor_tail = mem_b32s1(HOLD_HEAD);
                                while mem_b32s1(minor_tail) > TEX_NULL {
                                    minor_tail = mem_b32s1(minor_tail);
                                }
                            }
                            if l >= j {
                                break;
                            }
                        }

                        while l > j {
                            j = reconstitute_global(j, hn_v, bchar, TOO_BIG_CHAR) + 1;
                            set_mem_b32s1(major_tail, mem_b32s1(HOLD_HEAD));
                            while mem_b32s1(major_tail) > TEX_NULL {
                                major_tail = mem_b32s1(major_tail);
                                r_count += 1;
                            }
                        }
                    }

                    if r_count > 127 {
                        set_mem_b32s1(s, mem_b32s1(r));
                        set_mem_b32s1(r, TEX_NULL);
                        crate::packaging::flush_node_list_global(r);
                    } else {
                        set_mem_b32s1(s, r);
                        set_mem_b16s0(r, r_count as u16);
                    }
                    s = major_tail;
                    set_hyphen_passed(j - 1);
                    set_mem_b32s1(HOLD_HEAD, TEX_NULL);

                    if !odd(hyf_entry((j - 1) as usize)) {
                        break;
                    }
                }
            }

            if j > hn_v {
                break;
            }
        }

        set_mem_b32s1(s, q_final);
        crate::mem::flush_list_global(crate::globals::init_list());
    }
}

// ---- moved verbatim from scan.rs (workstream 02) ----

// translated from xetex-xetex0.c:6968 (char_pw)
pub fn char_pw_in(
    mem: &MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    p: i32,
    side: i16,
) -> i32 {
    const WHATSIT_NODE: u16 = 8;
    const NATIVE_WORD_NODE: u16 = 40;
    const NATIVE_WORD_NODE_AT: u16 = 41;
    const GLYPH_NODE: u16 = 42;
    const QUAD_CODE: i32 = 6; // xetex-constants.h:298

    if side == 0 {
        linebreak.last_leftmost_char = TEX_NULL;
    } else {
        linebreak.last_rightmost_char = TEX_NULL;
    }
    if p == TEX_NULL {
        return 0;
    }

    // NATIVE_WORD_NODE or NATIVE_WORD_NODE_AT whatsit
    if p != TEX_NULL
        && p < mem.hi_mem_min
        && mem.b16s1(p) == WHATSIT_NODE
        && (mem.b16s0(p) == NATIVE_WORD_NODE || mem.b16s0(p) == NATIVE_WORD_NODE_AT)
    {
        if !mem.ptr(p + 5).is_null() {
            // NATIVE_NODE_glyph_info_ptr != NULL
            let f = mem.b16s2(p + 4) as i32; // NATIVE_NODE_font
            let cp = unsafe { real_get_native_word_cp(mem.node_addr(p), side as libc::c_int) };
            return crate::scaled_math::round_xn_over_d(
                fonts.font_info_b32s1(QUAD_CODE + fonts.param_base(f)),
                cp,
                1000,
            );
        } else {
            return 0;
        }
    }

    // GLYPH_NODE whatsit
    if p != TEX_NULL
        && p < mem.hi_mem_min
        && mem.b16s1(p) == WHATSIT_NODE
        && mem.b16s0(p) == GLYPH_NODE
    {
        let f = mem.b16s2(p + 4) as i32; // mem[p + 4].b16.s2
        let code = mem.b16s1(p + 4) as u32; // mem[p + 4].b16.s1 (char code)
        let cp = unsafe { get_cp_code(f, code, side as i32) };
        return crate::scaled_math::round_xn_over_d(
            fonts.font_info_b32s1(QUAD_CODE + fonts.param_base(f)),
            cp,
            1000,
        );
    }

    // Adjust p for ligature node or non-char node
    let p = if p < mem.hi_mem_min {
        if mem.b16s1(p) == crate::consts::LIGATURE_NODE {
            p + 1 // LIGATURE_NODE: the char is at p+1
        } else {
            return 0;
        }
    } else {
        p
    };

    let f = mem.b16s1(p) as i32; // CHAR_NODE_font(p) = mem[p].b16.s1
    let c = unsafe { get_cp_code(f, mem.b16s0(p) as u32, side as i32) };
    if side == 0 {
        linebreak.last_leftmost_char = p;
    } else {
        linebreak.last_rightmost_char = p;
    }
    if c == 0 {
        return 0;
    }
    crate::scaled_math::round_xn_over_d(
        fonts.font_info_b32s1(QUAD_CODE + fonts.param_base(f)),
        c,
        1000,
    )
}
/// Insert locale-aware linebreaking nodes for `len` glyphs starting at `native_text[s]`.
///
/// translated from xetex-xetex0.c:6493 (do_locale_linebreaks)
pub unsafe fn do_locale_linebreaks(s: i32, len: i32) {
    use crate::globals::{
        cur_list_tail, mem_as_u16_ptr_mut, mem_b32s1, set_cur_list_tail, set_mem_b32s1,
    };
    let native_text = crate::globals::native_text();

    let locale = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_LINEBREAK_LOCALE);
    let use_glyph = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0;

    if locale == 0 || len == 1 {
        let node = new_native_word_node_global(main_f, len);
        set_mem_b32s1(cur_list_tail(), node);
        set_cur_list_tail(mem_b32s1(cur_list_tail()));
        let text = mem_as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE_NN);
        for i in 0..len {
            *text.add(i as usize) = *native_text.add((s + i) as usize);
        }
        measure_native_node_global(
            crate::globals::mem_node_addr(cur_list_tail()),
            use_glyph as libc::c_int,
        );
    } else {
        let use_skip = eqtb_b32s1(GLUE_BASE + GLUE_PAR_XETEX_LINEBREAK_SKIP) != 0;
        let penalty_val = eqtb_b32s1(INT_BASE + INT_PAR_XETEX_LINEBREAK_PENALTY);
        let use_penalty = penalty_val != 0 || !use_skip;
        linebreak_start(main_f, locale, native_text.add(s as usize), len);
        let mut offs: i32 = 0;
        loop {
            let prev_offs = offs;
            offs = linebreak_next_global(main_f);
            if offs > 0 {
                if prev_offs != 0 {
                    if use_penalty {
                        let node = crate::mem::new_penalty_global(penalty_val);
                        set_mem_b32s1(cur_list_tail(), node);
                        set_cur_list_tail(mem_b32s1(cur_list_tail()));
                    }
                    if use_skip {
                        let node =
                            crate::mem::new_param_glue_global(GLUE_PAR_XETEX_LINEBREAK_SKIP as i16);
                        set_mem_b32s1(cur_list_tail(), node);
                        set_cur_list_tail(mem_b32s1(cur_list_tail()));
                    }
                }
                let node = new_native_word_node_global(main_f, offs - prev_offs);
                set_mem_b32s1(cur_list_tail(), node);
                set_cur_list_tail(mem_b32s1(cur_list_tail()));
                let text = mem_as_u16_ptr_mut(cur_list_tail() + NATIVE_NODE_SIZE_NN);
                for i in prev_offs..offs {
                    *text.add((i - prev_offs) as usize) = *native_text.add((s + i) as usize);
                }
                measure_native_node_global(
                    crate::globals::mem_node_addr(cur_list_tail()),
                    use_glyph as libc::c_int,
                );
            }
            if offs < 0 {
                break;
            }
        }
    }
}
