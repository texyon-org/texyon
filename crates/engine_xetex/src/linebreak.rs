use crate::consts::{
    ADJUST_HEAD, AWFUL_BAD, DIMEN_BASE, DIMEN_PAR_HANG_INDENT, DIMEN_PAR_HSIZE, EJECT_PENALTY,
    ETEX_PEN_BASE, EXPLICIT, GLUE_BASE, INT_BASE, INT_PAR_HANG_AFTER, INT_PAR_LOOSENESS,
    INT_PAR_TEXXET, LC_CODE_BASE, LIG_TAG, LOCAL_BASE, LOCAL_PAR_SHAPE, MAX_HALFWORD, MIN_TRIE_OP,
    NON_ADDRESS, SMALL_NODE_SIZE, SPACE_ADJUSTMENT, TEX_NULL, TOO_BIG_CHAR,
};
use crate::ext::{
    linebreak_next, linebreak_start_in, measure_native_node, real_get_native_word_cp,
};
use crate::node::{
    ActiveNode, AnyBox, AnyNode, DeltaNode, DiscNode, GlueNode, GlueSpecNode, LigatureNode, MathNode, NodePtr, PassiveNode, PenaltyNode, WhatsitNode, WhatsitSubtype,
};
use crate::scan::NodeRecycleAbort;
use crate::state::{
    ConfigState, EngineState, EqtbState, ErrorState, FontState, HyphState, LinebreakState,
    MemState, ScannerState,
};
use crate::units::{MemIdx, MemPtr, Scaled};

const GLUE_PAR_XETEX_LINEBREAK_SKIP: i32 = 15;
const INT_PAR_XETEX_LINEBREAK_LOCALE: i32 = 66;
const INT_PAR_XETEX_LINEBREAK_PENALTY: i32 = 67;

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
const NORMAL: u16 = 0;
const HOLD_HEAD: i32 = 4_999_995;
const ACTIVE_LIST: i32 = 4_999_992;
const LAST_ACTIVE: i32 = ACTIVE_LIST;
const PASSIVE_NODE_SIZE: i32 = 2;
const ACTIVE_NODE_SIZE_NORMAL: i16 = 3;
const ACTIVE_NODE_SIZE_EXTENDED: i16 = 5;
const DELTA_NODE_SIZE: i32 = 7;
const HYPH_PRIME: i32 = 607;
const INT_PAR_PRETOLERANCE: i32 = 0;
const INT_PAR_TOLERANCE: i32 = 1;
const INT_PAR_LINE_PENALTY: i32 = 2;
const INT_PAR_HYPHEN_PENALTY: i32 = 3;
const INT_PAR_EX_HYPHEN_PENALTY: i32 = 4;
const INT_PAR_DOUBLE_HYPHEN_DEMERITS: i32 = 14;
const INT_PAR_FINAL_HYPHEN_DEMERITS: i32 = 15;
const INT_PAR_ADJ_DEMERITS: i32 = 16;
const INT_PAR_UC_HYPH: i32 = 38;
const INT_PAR_LAST_LINE_FIT: i32 = 62;
const INT_PAR_XETEX_USE_GLYPH_METRICS: i32 = 72;
const TEMP_HEAD: i32 = 4_999_996;
const PRE_ADJUST_HEAD: i32 = 4_999_985;
const GLUE_PAR_LEFT_SKIP: i16 = 7;
const GLUE_PAR_RIGHT_SKIP: i16 = 8;
const GLUE_PAR_PAR_FILL_SKIP: i16 = 14;
const INT_PAR_CLUB_PENALTY: i32 = 5;
const INT_PAR_WIDOW_PENALTY: i32 = 6;
const INT_PAR_DISPLAY_WIDOW_PENALTY: i32 = 7;
const INT_PAR_BROKEN_PENALTY: i32 = 8;
const INT_PAR_INTER_LINE_PENALTY: i32 = 13;
const INT_PAR_XETEX_PROTRUDE_CHARS: i32 = 68;
const DIMEN_PAR_EMERGENCY_STRETCH: i32 = 20;
const ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES: i32 = 0;
const ETEX_PENALTIES_PAR_CLUB_PENALTIES: i32 = 1;
const ETEX_PENALTIES_PAR_WIDOW_PENALTIES: i32 = 2;
const ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES: i32 = 3;
const L_CODE: u16 = 4;
const END_M_CODE: u16 = 3;
const EXACTLY: i16 = 0;
const ADDITIONAL: i16 = 1;
const UNHYPHENATED: i16 = 0;
const HYPHENATED: i16 = 1;
const VERY_LOOSE_FIT: usize = 0;
const TIGHT_FIT: usize = 3;
const DECENT_FIT: usize = 2;
const LOOSE_FIT: usize = 1;
const INF_BAD: i32 = 10_000;
const INF_PENALTY: i32 = 10_000;

extern "C" {
    fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;
}

// translated from xetex-linebreak.c:77 (get_native_usv)
pub fn get_native_usv(mem: &MemState, p: i32, i: i32) -> i32 {
    let text = whatsit_node_in(mem, p).native_text_ptr(mem);
    let c = unsafe { *text.add(i as usize) };

    if (0xD800..0xDC00).contains(&c) {
        return 0x10000 + (c as i32 - 0xD800) * 0x400 + unsafe { *text.add(i as usize + 1) as i32 }
            - 0xDC00;
    }

    c as i32
}

#[inline]
fn font_character_width(fonts: &mut FontState, f: i32, c: u16) -> i32 {
    let ec = crate::fonts::effective_char(fonts, true, f, c);
    let info = fonts.font_info_b16(fonts.char_base_entry(f) + ec);
    fonts.font_info_b32s1(fonts.width_base_entry(f) + info.s3 as i32)
}

/// Wrap an active-list pointer as a typed [`NodePtr<ActiveNode>`]. This is
/// the active list's single **context-classification** boundary:
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
fn node_subtype_in(mem: &MemState, p: i32) -> u16 {
    mem.b16s0(p)
}

#[inline]
fn box_width_in(mem: &MemState, p: i32) -> i32 {
    mem.b32s1(p + 1)
}

#[inline]
fn typed_list_width_in(mem: &MemState, node: i32) -> i32 {
    match mem
        .classify(MemIdx::new(node))
        .expect("expected valid width-bearing node")
    {
        AnyNode::HList(p) => p.width(mem).0,
        AnyNode::VList(p) => p.width(mem).0,
        AnyNode::Rule(p) => p.width(mem).raw(),
        AnyNode::Kern(p) => p.width(mem).0,
        AnyNode::Math(p) => p.width(mem).0,
        _ => unreachable!("typed width for non-width node"),
    }
}

#[inline]
fn glue_ptr_in(mem: &MemState, p: i32) -> i32 {
    glue_node(p).glue_ptr(mem).0
}

#[inline]
fn set_glue_ptr_in(mem: &mut MemState, p: i32, v: i32) {
    glue_node(p).set_glue_ptr(mem, MemPtr::from_raw(v));
}

#[inline]
fn glue_leader_ptr_in(mem: &MemState, p: i32) -> i32 {
    glue_node(p).leader_ptr(mem).0
}

#[inline]
fn glue_spec_stretch_order_in(mem: &MemState, p: i32) -> u16 {
    glue_spec(p).stretch_order(mem)
}

#[inline]
fn glue_spec_shrink_order_in(mem: &MemState, p: i32) -> u16 {
    glue_spec(p).shrink_order(mem)
}

#[inline]
fn glue_spec_stretch_in(mem: &MemState, p: i32) -> i32 {
    glue_spec(p).stretch(mem).0
}

#[inline]
fn glue_spec_shrink_in(mem: &MemState, p: i32) -> i32 {
    glue_spec(p).shrink(mem).0
}

#[inline]
fn penalty_node_penalty_in(mem: &MemState, p: i32) -> i32 {
    penalty_node(p).penalty(mem)
}

#[inline]
fn set_penalty_node_penalty_in(mem: &mut MemState, p: i32, v: i32) {
    penalty_node(p).set_penalty(mem, v);
}

#[inline]
fn whatsit_node_in(mem: &MemState, p: i32) -> NodePtr<WhatsitNode> {
    mem.classify_whatsit(MemIdx::new(p))
        .ok()
        .flatten()
        .expect("expected whatsit node")
}

#[inline]
fn language_node_what_lang_in(mem: &MemState, p: i32) -> u8 {
    whatsit_node_in(mem, p).what_lang(mem) as u8
}

#[inline]
fn language_node_what_lhm_in(mem: &MemState, p: i32) -> i32 {
    whatsit_node_in(mem, p).what_lhm(mem) as i32
}

#[inline]
fn language_node_what_rhm_in(mem: &MemState, p: i32) -> i32 {
    whatsit_node_in(mem, p).what_rhm(mem) as i32
}

#[inline]
fn ligature_node_lig_font_in(mem: &MemState, p: i32) -> i32 {
    ligature_node(p).font(mem).0
}

#[inline]
fn ligature_node_lig_char_in(mem: &MemState, p: i32) -> u16 {
    ligature_node(p).character(mem)
}

#[inline]
fn ligature_node_lig_ptr_in(mem: &MemState, p: i32) -> i32 {
    ligature_node(p).lig_ptr(mem).0
}

#[inline]
fn disc_pre_break_in(mem: &MemState, p: i32) -> i32 {
    disc_node(p).pre_break(mem).0
}

#[inline]
fn disc_replace_count_in(mem: &MemState, p: i32) -> i32 {
    disc_node(p).subtype(mem) as i32
}

#[inline]
fn native_node_len_in(mem: &MemState, p: i32) -> i32 {
    whatsit_node_in(mem, p).native_length(mem) as i32
}

#[inline]
fn set_native_node_len_in(mem: &mut MemState, p: i32, v: i32) {
    whatsit_node_in(mem, p).set_native_length(mem, v as u16);
}

#[inline]
fn native_node_font_in(mem: &MemState, p: i32) -> i32 {
    whatsit_node_in(mem, p).native_font(mem).0
}

#[inline]
pub unsafe fn copy_native_tail(mem: &mut MemState, q: i32, ha: i32, start: i32) {
    let qh = whatsit_node_in(mem, q);
    let dst = qh.native_text_ptr_mut(mem);
    let hah = whatsit_node_in(mem, ha);
    let src = hah.native_text_ptr(mem);
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
        set_native_metrics(&st.mem, &st.eqtb, &st.fonts, q)?;
    }
    st.mem.set_b32s1(q, st.mem.b32s1(ha));
    st.mem.set_b32s1(ha, q);
    set_native_node_len_in(&mut st.mem, ha, start);
    unsafe {
        set_native_metrics(&st.mem, &st.eqtb, &st.fonts, ha)?;
    }
    if advance_ha {
        st.linebreak.ha = st.mem.b32s1(ha);
    }
    Ok(())
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
    p.width(mem).0 == 0 && p.height(mem).0 == 0 && p.depth(mem).0 == 0 && p.list_ptr(mem).is_null()
}

#[inline]
fn ignorable_protrusion_node_in(mem: &MemState, p: i32) -> bool {
    use crate::node::AnyNode;
    if crate::mem::is_char_node(mem, p) {
        return false;
    }
    // Classify once at the boundary (the node is non-char, so its type is valid).
    match mem.classify(MemIdx::new(p)).expect("non-char node has a valid type") {
        AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_) | AnyNode::Penalty(_) => true,
        AnyNode::Disc(d) => {
            d.pre_break(mem).is_null() && d.post_break(mem).is_null() && d.subtype(mem) == 0
        }
        AnyNode::Math(m) => m.width(mem).0 == 0,
        AnyNode::Kern(k) => k.width(mem).0 == 0 || k.subtype(mem) == NORMAL,
        AnyNode::Glue(g) => g.glue_ptr(mem).0 == 0,
        AnyNode::HList(h) => {
            h.width(mem).0 == 0
                && h.height(mem).0 == 0
                && h.depth(mem).0 == 0
                && h.list_ptr(mem).is_null()
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
pub fn total_pw(errors: &mut crate::state::ErrorState, 
    mem: &mut MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    q: i32,
    p: i32,
) -> Option<i32> {
    let break_node = active(q).break_node(mem);
    let mut l = if break_node.is_null() {
        linebreak.first_p
    } else {
        passive_node(break_node.0).cur_break(mem).0
    };

    let mut r = crate::mem::prev_rightmost(mem, linebreak.global_prev_p, p);
    if p != TEX_NULL && mem.b16s1(p) as i32 == DISC_NODE {
        let p_disc = disc_node(p);
        if !p_disc.pre_break(mem).is_null() {
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
        if !l_disc.post_break(mem).is_null() {
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

    Some(char_pw_in(errors, mem, fonts, linebreak, l, 0) + char_pw_in(errors, mem, fonts, linebreak, r, 1))
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
        while run && mem.b16s1(l) as i32 == HLIST_NODE && !box_node(mem, l).list_ptr(mem).is_null()
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
        while run && mem.b16s1(r) as i32 == HLIST_NODE && !box_node(mem, r).list_ptr(mem).is_null()
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

pub fn insert_right_margin_kern(errors: &mut crate::state::ErrorState, 
    mem: &mut MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    q: i32,
    disc_break: bool,
    last_rightmost: i32,
) -> Result<i32, MarginKernAbort> {
    let (p, ptmp) =
        right_margin_kern_target(mem, linebreak, q, disc_break).ok_or(MarginKernAbort::PdfError)?;

    let w = char_pw_in(errors, mem, fonts, linebreak, p, 1);
    if w == 0 {
        return Ok(q);
    }

    let Some(k) = crate::mem::alloc_margin_kern(mem, -w, last_rightmost, 1) else {
        return Err(MarginKernAbort::Overflow);
    };
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

pub fn insert_left_margin_kern(errors: &mut crate::state::ErrorState, 
    mem: &mut MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    q: i32,
    last_leftmost: i32,
) -> Result<i32, MarginKernAbort> {
    let p = left_margin_kern_target(mem, linebreak, q).ok_or(MarginKernAbort::PdfError)?;

    let w = char_pw_in(errors, mem, fonts, linebreak, p, 0);
    if w == 0 {
        return Ok(q);
    }

    let Some(k) = crate::mem::alloc_margin_kern(mem, -w, last_leftmost, 0) else {
        return Err(MarginKernAbort::Overflow);
    };
    k.set_link(mem, MemPtr::from_raw(q));
    Ok(k.idx().0)
}

/// translated from xetex-linebreak.c:2377 (finite_shrink)
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

/// translated from xetex-linebreak.c:2402 (reconstitute)
pub fn reconstitute(
    st: &mut crate::state::EngineState,
    mut j: i16,
    n: i16,
    mut bchar: i32,
    mut hchar: i32,
) -> Result<i16, NodeRecycleAbort> {
    #[inline]
    fn odd(v: u8) -> bool {
        v % 2 == 1
    }

    #[inline]
    fn update_right(
        st: &mut crate::state::EngineState,
        j: i16,
        n: i16,
        bchar: i32,
        hchar: i32,
    ) -> i32 {
        if j < n {
            st.mainloop.cur_r = st.hyph.hu[(j + 1) as usize];
        } else {
            st.mainloop.cur_r = bchar;
        }
        if odd(st.hyph.hyf[j as usize]) {
            hchar
        } else {
            TOO_BIG_CHAR
        }
    }

    st.hyph.hyphen_passed = 0;
    let mut t = HOLD_HEAD;
    let mut w = 0;
    st.mem.set_b32s1(HOLD_HEAD, TEX_NULL);
    st.mainloop.cur_l = st.hyph.hu[j as usize];
    st.mainloop.cur_q = t;

    if j == 0 {
        st.mainloop.ligature_present = st.hyph.init_lig;
        let mut p = st.hyph.init_list;
        if st.mainloop.ligature_present {
            st.mainloop.lft_hit = st.hyph.init_lft;
        }
        while p > TEX_NULL {
            let q = crate::mem::get_avail(&mut st.mem);
            if q == crate::mem::ALLOC_FAILED {
                return Err(NodeRecycleAbort::OverflowRunaway);
            }
            st.mem.set_b32s1(t, q);
            t = st.mem.b32s1(t);
            st.mem.set_b16s1(t, st.hyph.hf as u16);
            st.mem.set_b16s0(t, st.mem.b16s0(p));
            p = st.mem.b32s1(p);
        }
    } else if st.mainloop.cur_l < TOO_BIG_CHAR {
        let q = crate::mem::get_avail(&mut st.mem);
        if q == crate::mem::ALLOC_FAILED {
            return Err(NodeRecycleAbort::OverflowRunaway);
        }
        st.mem.set_b32s1(t, q);
        t = st.mem.b32s1(t);
        st.mem.set_b16s1(t, st.hyph.hf as u16);
        st.mem.set_b16s0(t, st.mainloop.cur_l as u16);
    }

    st.mainloop.lig_stack = TEX_NULL;
    let mut cur_rh = update_right(st, j, n, bchar, hchar);

    loop {
        'continue_: loop {
            let mut k;
            let mut q;

            if st.mainloop.cur_l == TOO_BIG_CHAR {
                k = st.fonts.bchar_label_entry(st.hyph.hf);
                if k == NON_ADDRESS {
                    break 'continue_;
                }
                q = st.fonts.font_info_b16(k);
            } else {
                let char_base = st.fonts.char_base_entry(st.hyph.hf);
                let effective = crate::fonts::effective_char(&mut st.fonts, true, st.hyph.hf, st.mainloop.cur_l as u16);
                q = st.fonts.font_info_b16(char_base + effective);
                if q.s1 % 4 != LIG_TAG {
                    break 'continue_;
                }
                k = st.fonts.lig_kern_base_entry(st.hyph.hf) + q.s0 as i32;
                q = st.fonts.font_info_b16(k);
                if q.s3 > 128 {
                    k = st.fonts.lig_kern_base_entry(st.hyph.hf)
                        + 256 * q.s1 as i32
                        + q.s0 as i32
                        + 32768
                        - 256 * 128;
                    q = st.fonts.font_info_b16(k);
                }
            }

            let mut test_char = if cur_rh < TOO_BIG_CHAR {
                cur_rh
            } else {
                st.mainloop.cur_r
            };

            loop {
                if q.s2 as i32 == test_char && q.s3 <= 128 {
                    if cur_rh < TOO_BIG_CHAR {
                        st.hyph.hyphen_passed = j;
                        hchar = TOO_BIG_CHAR;
                        cur_rh = TOO_BIG_CHAR;
                        continue 'continue_;
                    }

                    if hchar < TOO_BIG_CHAR && odd(st.hyph.hyf[j as usize]) {
                        st.hyph.hyphen_passed = j;
                        hchar = TOO_BIG_CHAR;
                    }

                    if q.s1 < 128 {
                        if st.mainloop.cur_l == TOO_BIG_CHAR {
                            st.mainloop.lft_hit = true;
                        }
                        if j == n && st.mainloop.lig_stack == TEX_NULL {
                            st.mainloop.rt_hit = true;
                        }

                        match q.s1 {
                            1 | 5 => {
                                st.mainloop.cur_l = q.s0 as i32;
                                st.mainloop.ligature_present = true;
                            }
                            2 | 6 => {
                                st.mainloop.cur_r = q.s0 as i32;
                                if st.mainloop.lig_stack > TEX_NULL {
                                    st.mem.set_b16s0(st.mainloop.lig_stack, st.mainloop.cur_r as u16);
                                } else {
                                    let lig_item = crate::mem::new_lig_item(&mut st.mem, st.mainloop.cur_r as u16);
                                    if lig_item == crate::mem::ALLOC_FAILED {
                                        return Err(NodeRecycleAbort::Overflow);
                                    }
                                    st.mainloop.lig_stack = lig_item;
                                    if j == n {
                                        bchar = TOO_BIG_CHAR;
                                    } else {
                                        let p = crate::mem::get_avail(&mut st.mem);
                                        if p == crate::mem::ALLOC_FAILED {
                                            return Err(NodeRecycleAbort::OverflowRunaway);
                                        }
                                        st.mem.set_b32s1(st.mainloop.lig_stack + 1, p);
                                        st.mem.set_b16s0(p, st.hyph.hu[(j + 1) as usize] as u16);
                                        st.mem.set_b16s1(p, st.hyph.hf as u16);
                                    }
                                }
                            }
                            3 => {
                                st.mainloop.cur_r = q.s0 as i32;
                                let p = st.mainloop.lig_stack;
                                let lig_item = crate::mem::new_lig_item(&mut st.mem, st.mainloop.cur_r as u16);
                                if lig_item == crate::mem::ALLOC_FAILED {
                                    return Err(NodeRecycleAbort::Overflow);
                                }
                                st.mainloop.lig_stack = lig_item;
                                st.mem.set_b32s1(st.mainloop.lig_stack, p);
                            }
                            7 | 11 => {
                                if st.mainloop.ligature_present {
                                    let mq = st.mem.b32s1(st.mainloop.cur_q);
                                    let p = crate::mem::new_ligature(
                                        &mut st.mem,
                                        st.hyph.hf,
                                        st.mainloop.cur_l as u16,
                                        mq,
                                    );
                                    if p == crate::mem::ALLOC_FAILED {
                                        return Err(NodeRecycleAbort::Overflow);
                                    }
                                    if st.mainloop.lft_hit {
                                        st.mem.set_b16s0(p, 2);
                                        st.mainloop.lft_hit = false;
                                    }
                                    st.mem.set_b32s1(st.mainloop.cur_q, p);
                                    t = p;
                                    st.mainloop.ligature_present = false;
                                }
                                st.mainloop.cur_q = t;
                                st.mainloop.cur_l = q.s0 as i32;
                                st.mainloop.ligature_present = true;
                            }
                            _ => {
                                st.mainloop.cur_l = q.s0 as i32;
                                st.mainloop.ligature_present = true;
                                if st.mainloop.lig_stack > TEX_NULL {
                                    if st.mem.b32s1(st.mainloop.lig_stack + 1) > TEX_NULL {
                                        st.mem.set_b32s1(t, st.mem.b32s1(st.mainloop.lig_stack + 1));
                                        t = st.mem.b32s1(t);
                                        j += 1;
                                    }
                                    let p = st.mainloop.lig_stack;
                                    st.mainloop.lig_stack = st.mem.b32s1(p);
                                    crate::mem::free_node(&mut st.mem, p, SMALL_NODE_SIZE);
                                    if st.mainloop.lig_stack == TEX_NULL {
                                        cur_rh = update_right(st, j, n, bchar, hchar);
                                    } else {
                                        st.mainloop.cur_r = st.mem.b16s0(st.mainloop.lig_stack) as i32;
                                    }
                                } else if j == n {
                                    break 'continue_;
                                } else {
                                    let p = crate::mem::get_avail(&mut st.mem);
                                    if p == crate::mem::ALLOC_FAILED {
                                        return Err(NodeRecycleAbort::OverflowRunaway);
                                    }
                                    st.mem.set_b32s1(t, p);
                                    t = st.mem.b32s1(t);
                                    st.mem.set_b16s1(t, st.hyph.hf as u16);
                                    st.mem.set_b16s0(t, st.mainloop.cur_r as u16);
                                    j += 1;
                                    cur_rh = update_right(st, j, n, bchar, hchar);
                                }
                            }
                        }

                        if q.s1 > 4 && q.s1 != 7 {
                            break 'continue_;
                        }
                        continue 'continue_;
                    }

                    w = st.fonts.font_info_b32s1(
                        st.fonts.kern_base_entry(st.hyph.hf) + 256 * q.s1 as i32 + q.s0 as i32,
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
                q = st.fonts.font_info_b16(k);
                test_char = if cur_rh < TOO_BIG_CHAR {
                    cur_rh
                } else {
                    st.mainloop.cur_r
                };
            }
        }

        if st.mainloop.ligature_present {
            let mq = st.mem.b32s1(st.mainloop.cur_q);
            let p = crate::mem::new_ligature(&mut st.mem, st.hyph.hf, st.mainloop.cur_l as u16, mq);
            if p == crate::mem::ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow);
            }
            if st.mainloop.lft_hit {
                st.mem.set_b16s0(p, 2);
                st.mainloop.lft_hit = false;
            }
            if st.mainloop.rt_hit && st.mainloop.lig_stack == TEX_NULL {
                st.mem.set_b16s0(p, st.mem.b16s0(p) + 1);
                st.mainloop.rt_hit = false;
            }
            st.mem.set_b32s1(st.mainloop.cur_q, p);
            t = p;
            st.mainloop.ligature_present = false;
        }

        if w != 0 {
            let p = crate::mem::new_kern(&mut st.mem, w);
            if p == crate::mem::ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow);
            }
            st.mem.set_b32s1(t, p);
            t = st.mem.b32s1(t);
            w = 0;
            st.mem.set_b32s0(t + 2, 0);
        }

        if st.mainloop.lig_stack > TEX_NULL {
            st.mainloop.cur_q = t;
            st.mainloop.cur_l = st.mem.b16s0(st.mainloop.lig_stack) as i32;
            st.mainloop.ligature_present = true;
            if st.mem.b32s1(st.mainloop.lig_stack + 1) > TEX_NULL {
                st.mem.set_b32s1(t, st.mem.b32s1(st.mainloop.lig_stack + 1));
                t = st.mem.b32s1(t);
                j += 1;
            }
            let p = st.mainloop.lig_stack;
            st.mainloop.lig_stack = st.mem.b32s1(p);
            crate::mem::free_node(&mut st.mem, p, SMALL_NODE_SIZE);
            if st.mainloop.lig_stack == TEX_NULL {
                cur_rh = update_right(st, j, n, bchar, hchar);
            } else {
                st.mainloop.cur_r = st.mem.b16s0(st.mainloop.lig_stack) as i32;
            }
            continue;
        }

        return Ok(j);
    }
}

#[inline]
pub unsafe fn set_native_metrics(
    mem: &MemState,
    eqtb: &EqtbState,
    fonts: &FontState,
    p: i32,
) -> Result<(), crate::ext::NativeFontAbort> {
    measure_native_node(
        fonts,
        mem.node_addr(p),
        (eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0) as libc::c_int,
    )
}

#[inline]
fn int_par(eqtb: &EqtbState, n: i32) -> i32 {
    eqtb.b32s1(INT_BASE + n)
}

#[inline]
fn dimen_par(eqtb: &EqtbState, n: i32) -> i32 {
    eqtb.b32s1(DIMEN_BASE + n)
}

#[inline]
fn glue_par(eqtb: &EqtbState, n: i16) -> i32 {
    eqtb.b32s1(GLUE_BASE + n as i32)
}

#[inline]
fn lc_code(eqtb: &EqtbState, c: i32) -> i32 {
    eqtb.b32s1(LC_CODE_BASE + c)
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

fn finish_last_line_fit(
    mem: &mut MemState,
    linebreak: &mut LinebreakState,
    last_line_fill: NodePtr<GlueNode>,
) -> Result<(), NodeRecycleAbort> {
    if !linebreak.do_last_line_fit {
        return Ok(());
    }

    let best = active(linebreak.best_bet);
    if best.shortfall(mem).0 == 0 {
        linebreak.do_last_line_fit = false;
        return Ok(());
    }

    let old_spec = last_line_fill.glue_ptr(mem);
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
    last_line_fill.set_glue_ptr(mem, MemPtr::from_raw(q));
    Ok(())
}

// translated from xetex-linebreak.c:90 (line_break)
/// Cold-abort outcomes deferred out of [`line_break_in`].
pub(crate) enum LineBreakOutcome {
    Done,
    Overflow,
    Node(NodeRecycleAbort),
    TryBreak(TryBreakAbort),
    Hyphenate(HyphenateAbort),
    Trie(crate::hyphenation::TrieAbort),
    PostLineBreak(PostLineBreakOutcome),
    Confusion(&'static core::ffi::CStr),
    /// Deferred tail of the recoverable `error()` raised by the
    /// `finite_shrink` diagnostic. `Continue` never reaches here (it stays
    /// inline); any cold abort (`Halt`/`Hundred`/…) is finished.
    Error(crate::errors::ErrorOutcome),
}

impl LineBreakOutcome {
    /// Map to the unified [`EngineAbort`] terminal. `Done` never reaches a
    /// terminal (callers filter it before propagating).
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        const MEM_TOP: i32 = 4_999_999;
        match self {
            LineBreakOutcome::Done => {
                unreachable!("LineBreakOutcome::Done is never propagated as a terminal abort")
            }
            LineBreakOutcome::Overflow => crate::errors::EngineAbort::Overflow {
                what: c"main memory size".as_ptr(),
                n: MEM_TOP + 1,
            },
            LineBreakOutcome::Node(e) => e.into_engine_abort(),
            LineBreakOutcome::TryBreak(a) => a.into_engine_abort(),
            LineBreakOutcome::Hyphenate(a) => a.into_engine_abort(),
            LineBreakOutcome::Trie(e) => e.into_engine_abort(),
            LineBreakOutcome::PostLineBreak(o) => o.into_engine_abort(),
            LineBreakOutcome::Confusion(message) => {
                crate::errors::EngineAbort::Confusion(message.as_ptr())
            }
            LineBreakOutcome::Error(o) => o.into_engine_abort(),
        }
    }
}

/// When this is the first infinite-shrinkage hit of the paragraph
/// (`no_shrink_error_yet`), emit the recoverable diagnostic through the
/// `error_*_in` path, then make the glue spec's shrink finite via
/// [`finite_shrink`]. The recoverable `error()` returns
/// `ErrorOutcome::Continue` on the hot path; any cold abort is deferred to the
/// [`line_break`] wrapper via [`LineBreakOutcome::Error`].
fn finite_shrink_emit(st: &mut EngineState, p: i32) -> Result<i32, LineBreakOutcome> {
    if st.errors.no_shrink_error_yet {
        st.errors.no_shrink_error_yet = false;
        crate::output::error_here_with_diagnostic_in(
            st,
            c"Infinite glue shrinkage found in a paragraph".as_ptr(),
        );
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 5;
        st.errors.help_line[4] = c"The paragraph just ended includes some glue that has".as_ptr();
        st.errors.help_line[3] =
            c"infinite shrinkability, e.g., `\\hskip 0pt minus 1fil'.".as_ptr();
        st.errors.help_line[2] = c"Such glue doesn't belong there---it allows a paragraph".as_ptr();
        st.errors.help_line[1] =
            c"of any length to fit on one line. But it's safe to proceed,".as_ptr();
        st.errors.help_line[0] =
            c"since the offensive shrinkability has been made finite.".as_ptr();
        match crate::errors::error_in(st) {
            crate::errors::ErrorOutcome::Continue => {}
            other => return Err(LineBreakOutcome::Error(other)),
        }
    }
    finite_shrink(&mut st.mem, p).map_err(LineBreakOutcome::Node)
}

/// Run `try_break` with the disjoint borrows split out of `st`, mapping its
/// `Result` so [`line_break_in`] can defer the abort.
#[inline]
fn line_break_try_break(
    st: &mut EngineState,
    pi: i32,
    break_type: i16,
) -> Result<(), TryBreakAbort> {
    let s = &mut *st;
    // SAFETY: `try_break`'s `unsafe` is the native-measurer FFI on its cold
    // paths; every cluster it touches is threaded through these disjoint
    // borrows of `EngineState`.
    unsafe {
        try_break(
            &mut s.mem,
            &mut s.scanner,
            &mut s.linebreak,
            &s.eqtb,
            &mut s.fonts,
            &s.config,
            &mut s.errors,
            pi,
            break_type,
        )
    }
}

/// Translation of the linebreak `add_active_width(1, …)`
/// helper for a single discardable list node: adds the node's width to
/// `scanner.active_width[1]`. Returns the confusion message on the unexpected
/// node-type / bad-whatsit branch so the caller can defer it.
fn add_node_width_to_active(
    mem: &MemState,
    fonts: &mut FontState,
    scanner: &mut ScannerState,
    p: i32,
    confusion_name: &'static core::ffi::CStr,
) -> Option<&'static core::ffi::CStr> {
    if crate::mem::is_char_node(mem, p) {
        let f = mem.b16s1(p) as i32;
        scanner.active_width[1] += font_character_width(fonts, f, mem.b16s0(p));
        return None;
    }

    match node_type_in(mem, p) {
        LIGATURE_NODE => {
            let f = ligature_node_lig_font_in(mem, p);
            fonts.set_xtx_ligature_present(true);
            scanner.active_width[1] +=
                font_character_width(fonts, f, ligature_node_lig_char_in(mem, p));
        }
        HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
            scanner.active_width[1] += box_width_in(mem, p);
        }
        WHATSIT_NODE => {
            if WhatsitSubtype::from_raw(node_subtype_in(mem, p)).is_some_and(|s| s.has_dimensions())
            {
                scanner.active_width[1] += box_width_in(mem, p);
            } else {
                return Some(confusion_name);
            }
        }
        _ => return Some(confusion_name),
    }
    None
}

/// Core of [`line_break`]. Cold aborts ride the [`LineBreakOutcome`] sentinel.
pub(crate) fn line_break_in(st: &mut EngineState, d: bool) -> LineBreakOutcome {
    use crate::scan::max_hyphenatable_length_in;

    macro_rules! aw {
        ($k:expr, $v:expr) => {{
            let v = $v;
            st.scanner.active_width[$k] += v;
        }};
    }

    st.scanner.pack_begin_line = st.input.cur_list.mode_line;
    let head = st.input.cur_list.head;
    st.mem.set_b32s1(TEMP_HEAD, st.mem.b32s1(head));

    let tail = st.input.cur_list.tail;
    if crate::mem::is_char_node(&st.mem, tail) || node_type_in(&st.mem, tail) != GLUE_NODE {
        let p = crate::mem::new_penalty(&mut st.mem, INF_PENALTY);
        if p == crate::mem::ALLOC_FAILED {
            return LineBreakOutcome::Overflow;
        }
        st.mem.set_b32s1(tail, p);
        st.input.cur_list.tail = p;
    } else {
        st.mem.set_b16s1(tail, PENALTY_NODE as u16);
        let gp = glue_ptr_in(&st.mem, tail);
        crate::mem::delete_glue_ref(&mut st.mem, gp);
        let gl = glue_leader_ptr_in(&st.mem, tail);
        if let Err(msg) = crate::packaging::flush_node_list(&mut st.mem, gl) {
            return LineBreakOutcome::Confusion(msg);
        }
        set_penalty_node_penalty_in(&mut st.mem, tail, INF_PENALTY);
    }

    let last_line_fill = crate::mem::new_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_PAR_FILL_SKIP);
    if last_line_fill == crate::mem::ALLOC_FAILED {
        return LineBreakOutcome::Overflow;
    }
    let last_line_fill_node = glue_node(last_line_fill);
    st.mem
        .set_b32s1(st.input.cur_list.tail, last_line_fill);
    st.hyph.init_l_hyf = st.input.cur_list.prev_graf / 0x0040_0000;
    st.hyph.init_r_hyf = (st.input.cur_list.prev_graf / 65_536) % 64;
    let init_cur_lang = (st.input.cur_list.prev_graf % 65_536) as u8;

    crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
    st.errors.no_shrink_error_yet = true;

    {
        let ls = glue_par(&st.eqtb, GLUE_PAR_LEFT_SKIP);
        if glue_spec_shrink_order_in(&st.mem, ls) != NORMAL && glue_spec_shrink_in(&st.mem, ls) != 0
        {
            let p = match finite_shrink_emit(st, ls) {
                Ok(v) => v,
                Err(o) => return o,
            };
            st.eqtb.set_b32s1(GLUE_BASE + GLUE_PAR_LEFT_SKIP as i32, p);
        }
    }
    {
        let rs = glue_par(&st.eqtb, GLUE_PAR_RIGHT_SKIP);
        if glue_spec_shrink_order_in(&st.mem, rs) != NORMAL && glue_spec_shrink_in(&st.mem, rs) != 0
        {
            let p = match finite_shrink_emit(st, rs) {
                Ok(v) => v,
                Err(o) => return o,
            };
            st.eqtb.set_b32s1(GLUE_BASE + GLUE_PAR_RIGHT_SKIP as i32, p);
        }
    }

    let q = glue_par(&st.eqtb, GLUE_PAR_LEFT_SKIP);
    let r = glue_par(&st.eqtb, GLUE_PAR_RIGHT_SKIP);
    st.linebreak.background[1] = box_width_in(&st.mem, q) + box_width_in(&st.mem, r);
    for k in 2..=5 {
        st.linebreak.background[k] = 0;
    }
    st.linebreak.background[2 + glue_spec_stretch_order_in(&st.mem, q) as usize] =
        glue_spec_stretch_in(&st.mem, q);
    {
        let idx = 2 + glue_spec_stretch_order_in(&st.mem, r) as usize;
        st.linebreak.background[idx] += glue_spec_stretch_in(&st.mem, r);
    }
    st.linebreak.background[6] = glue_spec_shrink_in(&st.mem, q) + glue_spec_shrink_in(&st.mem, r);

    st.linebreak.do_last_line_fit = false;
    st.linebreak.active_node_size = ACTIVE_NODE_SIZE_NORMAL;
    if int_par(&st.eqtb, INT_PAR_LAST_LINE_FIT) > 0 {
        let q = glue_ptr_in(&st.mem, last_line_fill);
        if glue_spec_stretch_in(&st.mem, q) > 0
            && glue_spec_stretch_order_in(&st.mem, q) > NORMAL
            && st.linebreak.background[3] == 0
            && st.linebreak.background[4] == 0
            && st.linebreak.background[5] == 0
        {
            st.linebreak.do_last_line_fit = true;
            st.linebreak.active_node_size = ACTIVE_NODE_SIZE_EXTENDED;
            st.linebreak.fill_width[0] = 0;
            st.linebreak.fill_width[1] = 0;
            st.linebreak.fill_width[2] = 0;
            st.linebreak.fill_width[(glue_spec_stretch_order_in(&st.mem, q) - 1) as usize] =
                glue_spec_stretch_in(&st.mem, q);
        }
    }

    st.linebreak.minimum_demerits = AWFUL_BAD;
    for fit in [TIGHT_FIT, DECENT_FIT, LOOSE_FIT, VERY_LOOSE_FIT] {
        st.linebreak.minimal_demerits[fit] = AWFUL_BAD;
    }

    if st.eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE) == TEX_NULL {
        if dimen_par(&st.eqtb, DIMEN_PAR_HANG_INDENT) == 0 {
            st.linebreak.last_special_line = 0;
            st.linebreak.second_width = dimen_par(&st.eqtb, DIMEN_PAR_HSIZE);
            st.linebreak.second_indent = 0;
        } else {
            st.linebreak.last_special_line = int_par(&st.eqtb, INT_PAR_HANG_AFTER).abs();
            if int_par(&st.eqtb, INT_PAR_HANG_AFTER) < 0 {
                st.linebreak.first_width = dimen_par(&st.eqtb, DIMEN_PAR_HSIZE)
                    - dimen_par(&st.eqtb, DIMEN_PAR_HANG_INDENT).abs();
                st.linebreak.first_indent = if dimen_par(&st.eqtb, DIMEN_PAR_HANG_INDENT) >= 0 {
                    dimen_par(&st.eqtb, DIMEN_PAR_HANG_INDENT)
                } else {
                    0
                };
                st.linebreak.second_width = dimen_par(&st.eqtb, DIMEN_PAR_HSIZE);
                st.linebreak.second_indent = 0;
            } else {
                st.linebreak.first_width = dimen_par(&st.eqtb, DIMEN_PAR_HSIZE);
                st.linebreak.first_indent = 0;
                st.linebreak.second_width = dimen_par(&st.eqtb, DIMEN_PAR_HSIZE)
                    - dimen_par(&st.eqtb, DIMEN_PAR_HANG_INDENT).abs();
                st.linebreak.second_indent = if dimen_par(&st.eqtb, DIMEN_PAR_HANG_INDENT) >= 0 {
                    dimen_par(&st.eqtb, DIMEN_PAR_HANG_INDENT)
                } else {
                    0
                };
            }
        }
    } else {
        let par_shape = st.eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
        st.linebreak.last_special_line = st.mem.b32s0(par_shape) - 1;
        st.linebreak.second_width = st
            .mem
            .b32s1(par_shape + 2 * (st.linebreak.last_special_line + 1));
        st.linebreak.second_indent = st
            .mem
            .b32s1(par_shape + 2 * st.linebreak.last_special_line + 1);
    }

    if int_par(&st.eqtb, INT_PAR_LOOSENESS) == 0 {
        st.linebreak.easy_line = st.linebreak.last_special_line;
    } else {
        st.linebreak.easy_line = MAX_HALFWORD;
    }

    st.linebreak.threshold = int_par(&st.eqtb, INT_PAR_PRETOLERANCE);
    let mut second_pass;
    if st.linebreak.threshold >= 0 {
        second_pass = false;
        st.linebreak.final_pass = false;
    } else {
        st.linebreak.threshold = int_par(&st.eqtb, INT_PAR_TOLERANCE);
        second_pass = true;
        st.linebreak.final_pass = dimen_par(&st.eqtb, DIMEN_PAR_EMERGENCY_STRETCH) <= 0;
    }

    'passes: loop {
        if st.linebreak.threshold > INF_BAD {
            st.linebreak.threshold = INF_BAD;
        }

        if second_pass {
            if st.hyph.trie_not_ready {
                if let Err(e) = crate::hyphenation::init_trie_in(&mut st.hyph) {
                    return LineBreakOutcome::Trie(e);
                }
            }
            st.hyph.cur_lang = init_cur_lang;
            st.hyph.l_hyf = st.hyph.init_l_hyf;
            st.hyph.r_hyf = st.hyph.init_r_hyf;
            set_current_hyph_index(&mut st.hyph);
        }

        let mut prev_p = match start_linebreak_pass(
            &mut st.mem,
            &mut st.scanner,
            &mut st.linebreak,
            &mut st.errors,
            st.input.cur_list.prev_graf,
        ) {
            Ok(v) => v,
            Err(e) => return LineBreakOutcome::Node(e),
        };
        let mut auto_breaking = true;

        while st.linebreak.cur_p != TEX_NULL && st.mem.b32s1(ACTIVE_LIST) != LAST_ACTIVE {
            let mut cur = st.linebreak.cur_p;

            if crate::mem::is_char_node(&st.mem, cur) {
                prev_p = cur;
                st.linebreak.global_prev_p = cur;
                loop {
                    let f = st.mem.b16s1(cur) as i32;
                    let c = st.mem.b16s0(cur);
                    aw!(1, font_character_width(&mut st.fonts, f, c));
                    cur = st.mem.b32s1(cur);
                    st.linebreak.cur_p = cur;
                    if !crate::mem::is_char_node(&st.mem, cur) {
                        break;
                    }
                }
            }

            cur = st.linebreak.cur_p;
            match node_type_in(&st.mem, cur) {
                HLIST_NODE | VLIST_NODE | RULE_NODE => {
                    aw!(1, box_width_in(&st.mem, cur));
                }
                WHATSIT_NODE => {
                    let subtype = WhatsitSubtype::from_raw(node_subtype_in(&st.mem, cur));
                    if subtype == Some(WhatsitSubtype::Language) {
                        st.hyph.cur_lang = language_node_what_lang_in(&st.mem, cur);
                        st.hyph.l_hyf = language_node_what_lhm_in(&st.mem, cur);
                        st.hyph.r_hyf = language_node_what_rhm_in(&st.mem, cur);
                        set_current_hyph_index(&mut st.hyph);
                    } else if subtype.is_some_and(|s| s.has_dimensions()) {
                        aw!(1, box_width_in(&st.mem, cur));
                    }
                }
                GLUE_NODE => {
                    if auto_breaking
                        && (crate::mem::is_char_node(&st.mem, prev_p)
                            || is_non_discardable_node(&st.mem, prev_p)
                            || (node_type_in(&st.mem, prev_p) == KERN_NODE
                                && node_subtype_in(&st.mem, prev_p) != EXPLICIT))
                    {
                        if let Err(e) = line_break_try_break(st, 0, UNHYPHENATED) {
                            return LineBreakOutcome::TryBreak(e);
                        }
                    }

                    let mut q = glue_ptr_in(&st.mem, cur);
                    if glue_spec_shrink_order_in(&st.mem, q) != NORMAL
                        && glue_spec_shrink_in(&st.mem, q) != 0
                    {
                        q = match finite_shrink_emit(st, q) {
                            Ok(v) => v,
                            Err(o) => return o,
                        };
                        set_glue_ptr_in(&mut st.mem, cur, q);
                    }
                    aw!(1, box_width_in(&st.mem, q));
                    aw!(
                        2 + glue_spec_stretch_order_in(&st.mem, q) as usize,
                        glue_spec_stretch_in(&st.mem, q)
                    );
                    aw!(6, glue_spec_shrink_in(&st.mem, q));

                    if second_pass && auto_breaking {
                        let mut prev_s = cur;
                        let mut s = st.mem.b32s1(prev_s);

                        'hyphen_probe: {
                            if s == TEX_NULL {
                                break 'hyphen_probe;
                            }

                            let mut c: i32;
                            let mut l: i32;

                            loop {
                                if crate::mem::is_char_node(&st.mem, s) {
                                    c = st.mem.b16s0(s) as i32;
                                    st.hyph.hf = st.mem.b16s1(s) as i32;
                                } else if node_type_in(&st.mem, s) == LIGATURE_NODE {
                                    if ligature_node_lig_ptr_in(&st.mem, s) == TEX_NULL {
                                        prev_s = s;
                                        s = st.mem.b32s1(prev_s);
                                        continue;
                                    }
                                    q = ligature_node_lig_ptr_in(&st.mem, s);
                                    c = st.mem.b16s0(q) as i32;
                                    st.hyph.hf = st.mem.b16s1(q) as i32;
                                } else if node_type_in(&st.mem, s) == KERN_NODE
                                    && node_subtype_in(&st.mem, s) == NORMAL
                                {
                                    prev_s = s;
                                    s = st.mem.b32s1(prev_s);
                                    continue;
                                } else if node_type_in(&st.mem, s) == MATH_NODE
                                    && node_subtype_in(&st.mem, s) >= L_CODE
                                {
                                    prev_s = s;
                                    s = st.mem.b32s1(prev_s);
                                    continue;
                                } else if node_type_in(&st.mem, s) == WHATSIT_NODE {
                                    if WhatsitSubtype::from_raw(node_subtype_in(&st.mem, s))
                                        .is_some_and(|sub| sub.is_native_word())
                                    {
                                        l = 0;
                                        while l < native_node_len_in(&st.mem, s) {
                                            c = get_native_usv(&st.mem, s, l);
                                            if lc_code(&st.eqtb, c) != 0 {
                                                st.hyph.hf = native_node_font_in(&st.mem, s);
                                                prev_s = s;
                                                if lc_code(&st.eqtb, c) == c
                                                    || int_par(&st.eqtb, INT_PAR_UC_HYPH) > 0
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
                                        if l < native_node_len_in(&st.mem, s) {
                                            break;
                                        }
                                    }

                                    if WhatsitSubtype::from_raw(node_subtype_in(&st.mem, s))
                                        == Some(WhatsitSubtype::Language)
                                    {
                                        st.hyph.cur_lang = language_node_what_lang_in(&st.mem, s);
                                        st.hyph.l_hyf = language_node_what_lhm_in(&st.mem, s);
                                        st.hyph.r_hyf = language_node_what_rhm_in(&st.mem, s);
                                        set_current_hyph_index(&mut st.hyph);
                                    }
                                    prev_s = s;
                                    s = st.mem.b32s1(prev_s);
                                    continue;
                                } else {
                                    break 'hyphen_probe;
                                }

                                if st.hyph.hyph_index == 0 || c > 255 {
                                    st.hyph.hc[0] = lc_code(&st.eqtb, c);
                                } else if st.hyph.trie_trc[(st.hyph.hyph_index + c) as usize]
                                    != c as u16
                                {
                                    st.hyph.hc[0] = 0;
                                } else {
                                    st.hyph.hc[0] =
                                        st.hyph.trie_tro[(st.hyph.hyph_index + c) as usize];
                                }

                                if st.hyph.hc[0] != 0 {
                                    if st.hyph.hc[0] == c || int_par(&st.eqtb, INT_PAR_UC_HYPH) > 0
                                    {
                                        break;
                                    } else {
                                        break 'hyphen_probe;
                                    }
                                }

                                prev_s = s;
                                s = st.mem.b32s1(prev_s);
                            }

                            st.hyph.hyf_char = st.fonts.hyphen_char[st.hyph.hf as usize];
                            if st.hyph.hyf_char < 0 || st.hyph.hyf_char > TOO_BIG_CHAR - 1 {
                                break 'hyphen_probe;
                            }
                            st.linebreak.ha = prev_s;
                            if st.hyph.l_hyf + st.hyph.r_hyf > max_hyphenatable_length_in(&st.eqtb)
                            {
                                break 'hyphen_probe;
                            }

                            if st.linebreak.ha != TEX_NULL
                                && st.linebreak.ha < st.mem.hi_mem_min
                                && node_type_in(&st.mem, st.linebreak.ha) == WHATSIT_NODE
                                && WhatsitSubtype::from_raw(node_subtype_in(&st.mem, st.linebreak.ha))
                                    .is_some_and(|sub| sub.is_native_word())
                            {
                                s = st.mem.b32s1(st.linebreak.ha);
                                loop {
                                    if !crate::mem::is_char_node(&st.mem, s) {
                                        match node_type_in(&st.mem, s) {
                                            LIGATURE_NODE => {}
                                            KERN_NODE => {
                                                if node_subtype_in(&st.mem, s) != NORMAL {
                                                    break;
                                                }
                                            }
                                            WHATSIT_NODE | GLUE_NODE | PENALTY_NODE | INS_NODE
                                            | ADJUST_NODE | MARK_NODE => break,
                                            _ => break 'hyphen_probe,
                                        }
                                    }
                                    s = st.mem.b32s1(s);
                                }

                                st.linebreak.hn = 0;
                                'restart_native: loop {
                                    let len = native_node_len_in(&st.mem, st.linebreak.ha);
                                    l = 0;
                                    while l < len {
                                        c = get_native_usv(&st.mem, st.linebreak.ha, l);
                                        if st.hyph.hyph_index == 0 || c > 255 {
                                            st.hyph.hc[0] = lc_code(&st.eqtb, c);
                                        } else if st.hyph.trie_trc
                                            [(st.hyph.hyph_index + c) as usize]
                                            != c as u16
                                        {
                                            st.hyph.hc[0] = 0;
                                        } else {
                                            st.hyph.hc[0] =
                                                st.hyph.trie_tro[(st.hyph.hyph_index + c) as usize];
                                        }

                                        if st.hyph.hc[0] == 0 {
                                            if st.linebreak.hn > 0 {
                                                if let Err(e) =
                                                    split_native_tail_for_linebreak(st, l, false)
                                                {
                                                    return LineBreakOutcome::Node(e);
                                                }
                                                break 'restart_native;
                                            }
                                        } else if st.linebreak.hn == 0 && l > 0 {
                                            if let Err(e) =
                                                split_native_tail_for_linebreak(st, l, true)
                                            {
                                                return LineBreakOutcome::Node(e);
                                            }
                                            continue 'restart_native;
                                        } else if st.linebreak.hn as i32
                                            == max_hyphenatable_length_in(&st.eqtb)
                                        {
                                            break 'restart_native;
                                        } else {
                                            st.linebreak.hn += 1;
                                            if c < 65_536 {
                                                st.hyph.hu[st.linebreak.hn as usize] = c;
                                                st.hyph.hc[st.linebreak.hn as usize] =
                                                    st.hyph.hc[0];
                                            } else {
                                                st.hyph.hu[st.linebreak.hn as usize] =
                                                    (c - 65_536) / 1024 + 0xD800;
                                                st.hyph.hc[st.linebreak.hn as usize] =
                                                    (st.hyph.hc[0] - 65_536) / 1024 + 0xD800;
                                                st.linebreak.hn += 1;
                                                st.hyph.hu[st.linebreak.hn as usize] =
                                                    c % 1024 + 0xDC00;
                                                st.hyph.hc[st.linebreak.hn as usize] =
                                                    st.hyph.hc[0] % 1024 + 0xDC00;
                                                l += 1;
                                            }
                                            st.hyph.hyf_bchar = TOO_BIG_CHAR;
                                        }
                                        l += 1;
                                    }
                                    break 'restart_native;
                                }
                            } else {
                                st.linebreak.hn = 0;
                                'collect_word: loop {
                                    if crate::mem::is_char_node(&st.mem, s) {
                                        if st.mem.b16s1(s) as i32 != st.hyph.hf {
                                            break;
                                        }
                                        st.hyph.hyf_bchar = st.mem.b16s0(s) as i32;
                                        c = st.hyph.hyf_bchar;
                                        if st.hyph.hyph_index == 0 || c > 255 {
                                            st.hyph.hc[0] = lc_code(&st.eqtb, c);
                                        } else if st.hyph.trie_trc
                                            [(st.hyph.hyph_index + c) as usize]
                                            != c as u16
                                        {
                                            st.hyph.hc[0] = 0;
                                        } else {
                                            st.hyph.hc[0] =
                                                st.hyph.trie_tro[(st.hyph.hyph_index + c) as usize];
                                        }
                                        if st.hyph.hc[0] == 0
                                            || st.hyph.hc[0] > st.hyph.max_hyph_char
                                            || st.linebreak.hn as i32
                                                == max_hyphenatable_length_in(&st.eqtb)
                                        {
                                            break;
                                        }
                                        st.linebreak.hb = s;
                                        st.linebreak.hn += 1;
                                        st.hyph.hu[st.linebreak.hn as usize] = c;
                                        st.hyph.hc[st.linebreak.hn as usize] = st.hyph.hc[0];
                                        st.hyph.hyf_bchar = TOO_BIG_CHAR;
                                    } else if node_type_in(&st.mem, s) == LIGATURE_NODE {
                                        if ligature_node_lig_font_in(&st.mem, s) != st.hyph.hf {
                                            break;
                                        }
                                        let mut j = st.linebreak.hn;
                                        q = ligature_node_lig_ptr_in(&st.mem, s);
                                        if q > TEX_NULL {
                                            st.hyph.hyf_bchar = st.mem.b16s0(q) as i32;
                                        }
                                        while q > TEX_NULL {
                                            c = st.mem.b16s0(q) as i32;
                                            if st.hyph.hyph_index == 0 || c > 255 {
                                                st.hyph.hc[0] = lc_code(&st.eqtb, c);
                                            } else if st.hyph.trie_trc
                                                [(st.hyph.hyph_index + c) as usize]
                                                != c as u16
                                            {
                                                st.hyph.hc[0] = 0;
                                            } else {
                                                st.hyph.hc[0] = st.hyph.trie_tro
                                                    [(st.hyph.hyph_index + c) as usize];
                                            }
                                            if st.hyph.hc[0] == 0
                                                || st.hyph.hc[0] > st.hyph.max_hyph_char
                                                || j as i32 == max_hyphenatable_length_in(&st.eqtb)
                                            {
                                                break 'collect_word;
                                            }
                                            j += 1;
                                            st.hyph.hu[j as usize] = c;
                                            st.hyph.hc[j as usize] = st.hyph.hc[0];
                                            q = st.mem.b32s1(q);
                                        }
                                        st.linebreak.hb = s;
                                        st.linebreak.hn = j;
                                        if odd(node_subtype_in(&st.mem, s) as u8) {
                                            st.hyph.hyf_bchar =
                                                st.fonts.font_bchar[st.hyph.hf as usize];
                                        } else {
                                            st.hyph.hyf_bchar = TOO_BIG_CHAR;
                                        }
                                    } else if node_type_in(&st.mem, s) == KERN_NODE
                                        && node_subtype_in(&st.mem, s) == NORMAL
                                    {
                                        st.linebreak.hb = s;
                                        st.hyph.hyf_bchar =
                                            st.fonts.font_bchar[st.hyph.hf as usize];
                                    } else {
                                        break;
                                    }
                                    s = st.mem.b32s1(s);
                                }
                            }

                            if (st.linebreak.hn as i32) < st.hyph.l_hyf + st.hyph.r_hyf {
                                break 'hyphen_probe;
                            }

                            loop {
                                if !crate::mem::is_char_node(&st.mem, s) {
                                    match node_type_in(&st.mem, s) {
                                        LIGATURE_NODE => {}
                                        KERN_NODE => {
                                            if node_subtype_in(&st.mem, s) != NORMAL {
                                                break;
                                            }
                                        }
                                        WHATSIT_NODE | GLUE_NODE | PENALTY_NODE | INS_NODE
                                        | ADJUST_NODE | MARK_NODE => break,
                                        MATH_NODE => {
                                            if node_subtype_in(&st.mem, s) >= L_CODE {
                                                break;
                                            } else {
                                                break 'hyphen_probe;
                                            }
                                        }
                                        _ => break 'hyphen_probe,
                                    }
                                }
                                s = st.mem.b32s1(s);
                            }

                            if let Err(e) = hyphenate_in(st) {
                                return LineBreakOutcome::Hyphenate(e);
                            }
                        }
                    }
                }
                KERN_NODE => {
                    if node_subtype_in(&st.mem, cur) == EXPLICIT {
                        if !crate::mem::is_char_node(&st.mem, st.mem.b32s1(cur))
                            && auto_breaking
                            && node_type_in(&st.mem, st.mem.b32s1(cur)) == GLUE_NODE
                        {
                            if let Err(e) = line_break_try_break(st, 0, UNHYPHENATED) {
                                return LineBreakOutcome::TryBreak(e);
                            }
                        }
                        aw!(1, box_width_in(&st.mem, cur));
                    } else {
                        aw!(1, box_width_in(&st.mem, cur));
                    }
                }
                LIGATURE_NODE => {
                    let f = ligature_node_lig_font_in(&st.mem, cur);
                    st.fonts.set_xtx_ligature_present(true);
                    let lc = ligature_node_lig_char_in(&st.mem, cur);
                    aw!(1, font_character_width(&mut st.fonts, f, lc));
                }
                DISC_NODE => {
                    let mut s = disc_pre_break_in(&st.mem, cur);
                    st.linebreak.disc_width = 0;
                    if s == TEX_NULL {
                        if let Err(e) = line_break_try_break(
                            st,
                            int_par(&st.eqtb, INT_PAR_EX_HYPHEN_PENALTY),
                            HYPHENATED,
                        ) {
                            return LineBreakOutcome::TryBreak(e);
                        }
                    } else {
                        st.linebreak.disc_width = 0;
                        while s != TEX_NULL {
                            let before = st.scanner.active_width[1];
                            {
                                let sx = &mut *st;
                                if let Some(msg) = add_node_width_to_active(
                                    &sx.mem,
                                    &mut sx.fonts,
                                    &mut sx.scanner,
                                    s,
                                    c"disc3",
                                ) {
                                    return LineBreakOutcome::Confusion(msg);
                                }
                            }
                            st.linebreak.disc_width += st.scanner.active_width[1] - before;
                            st.scanner.active_width[1] = before;
                            s = st.mem.b32s1(s);
                        }
                        aw!(1, st.linebreak.disc_width);
                        if let Err(e) = line_break_try_break(
                            st,
                            int_par(&st.eqtb, INT_PAR_HYPHEN_PENALTY),
                            HYPHENATED,
                        ) {
                            return LineBreakOutcome::TryBreak(e);
                        }
                        aw!(1, -st.linebreak.disc_width);
                    }

                    let mut count = disc_replace_count_in(&st.mem, cur);
                    s = st.mem.b32s1(cur);
                    while count > 0 {
                        {
                            let sx = &mut *st;
                            if let Some(msg) = add_node_width_to_active(
                                &sx.mem,
                                &mut sx.fonts,
                                &mut sx.scanner,
                                s,
                                c"disc4",
                            ) {
                                return LineBreakOutcome::Confusion(msg);
                            }
                        }
                        count -= 1;
                        s = st.mem.b32s1(s);
                    }

                    prev_p = cur;
                    st.linebreak.global_prev_p = cur;
                    st.linebreak.cur_p = s;
                    continue;
                }
                MATH_NODE => {
                    if node_subtype_in(&st.mem, cur) < L_CODE {
                        auto_breaking = odd(node_subtype_in(&st.mem, cur) as u8);
                    }
                    if !crate::mem::is_char_node(&st.mem, st.mem.b32s1(cur))
                        && auto_breaking
                        && node_type_in(&st.mem, st.mem.b32s1(cur)) == GLUE_NODE
                    {
                        if let Err(e) = line_break_try_break(st, 0, UNHYPHENATED) {
                            return LineBreakOutcome::TryBreak(e);
                        }
                    }
                    aw!(1, box_width_in(&st.mem, cur));
                }
                PENALTY_NODE => {
                    if let Err(e) = line_break_try_break(
                        st,
                        penalty_node_penalty_in(&st.mem, cur),
                        UNHYPHENATED,
                    ) {
                        return LineBreakOutcome::TryBreak(e);
                    }
                }
                MARK_NODE | INS_NODE | ADJUST_NODE => {}
                _ => return LineBreakOutcome::Confusion(c"paragraph"),
            }

            prev_p = cur;
            st.linebreak.global_prev_p = cur;
            st.linebreak.cur_p = st.mem.b32s1(cur);
        }

        if st.linebreak.cur_p == TEX_NULL {
            if let Err(e) = line_break_try_break(st, EJECT_PENALTY, HYPHENATED) {
                return LineBreakOutcome::TryBreak(e);
            }
            if choose_best_break(&st.mem, &mut st.linebreak, &st.eqtb) {
                break 'passes;
            }
        }

        free_active_and_passive_lists(&mut st.mem, &mut st.linebreak);

        if !second_pass {
            st.linebreak.threshold = int_par(&st.eqtb, INT_PAR_TOLERANCE);
            second_pass = true;
            st.linebreak.final_pass = dimen_par(&st.eqtb, DIMEN_PAR_EMERGENCY_STRETCH) <= 0;
        } else {
            st.linebreak.background[2] += dimen_par(&st.eqtb, DIMEN_PAR_EMERGENCY_STRETCH);
            st.linebreak.final_pass = true;
        }
    }

    if let Err(e) = finish_last_line_fit(&mut st.mem, &mut st.linebreak, last_line_fill_node) {
        return LineBreakOutcome::Node(e);
    }

    match post_line_break_in(st, d) {
        PostLineBreakOutcome::Done => {}
        other => return LineBreakOutcome::PostLineBreak(other),
    }

    free_final_linebreak_lists(&mut st.mem, &mut st.linebreak);

    st.scanner.pack_begin_line = 0;
    LineBreakOutcome::Done
}

#[inline]
fn odd(v: u8) -> bool {
    v % 2 == 1
}

#[inline]
fn odd_i32(v: i32) -> bool {
    (v & 1) != 0
}

/// Allocates through [`crate::mem::get_avail`]. On arena exhaustion it returns
/// `true` (the sentinel) rather than calling `runaway()`/`overflow()`;
/// `post_line_break` performs the abort.
#[inline]
fn update_lr_stack(mem: &mut MemState, q: NodePtr<MathNode>, lr_ptr: &mut i32) -> bool {
    let q_subtype = q.subtype(mem) as i32;
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
    q: NodePtr<MathNode>,
    lr_ptr: &mut i32,
) -> Result<(), PostLineAbort> {
    if update_lr_stack(mem, q, lr_ptr) {
        Err(PostLineAbort::Node(NodeRecycleAbort::OverflowRunaway))
    } else {
        Ok(())
    }
}

fn post_line_break_prefix(errors: &mut crate::state::ErrorState, 
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
                update_lr_stack_checked(mem, math_node(q), lr_ptr)?;
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
    } else if let AnyNode::Glue(q_glue) = mem.classify(MemIdx::new(q)).expect("valid break node") {
        let old_spec = q_glue.glue_ptr(mem);
        crate::mem::delete_glue_ref(mem, old_spec.0);
        let right_skip = eqtb.b32s1(GLUE_BASE + GLUE_PAR_RIGHT_SKIP as i32);
        q_glue.set_glue_ptr(mem, MemPtr::from_raw(right_skip));
        q_glue.set_subtype(mem, GLUE_PAR_RIGHT_SKIP as u16 + 1);
        let right_skip_spec = glue_spec(right_skip);
        right_skip_spec.set_ref_count(mem, right_skip_spec.ref_count(mem) + 1);
        glue_break = true;
    } else if let AnyNode::Disc(q_disc) = mem.classify(MemIdx::new(q)).expect("valid break node") {
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

        if !q_disc.post_break(mem).is_null() {
            let mut s = q_disc.post_break(mem).0;
            while mem.b32s1(s) != TEX_NULL {
                s = mem.b32s1(s);
            }
            mem.set_b32s1(s, r);
            r = q_disc.post_break(mem).0;
            q_disc.set_post_break(mem, MemPtr::NULL);
            post_disc_break = true;
        }

        if !q_disc.pre_break(mem).is_null() {
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
    } else if let AnyNode::Kern(q_kern) = mem.classify(MemIdx::new(q)).expect("valid break node") {
        q_kern.set_width(mem, Scaled(0));
    } else if let AnyNode::Math(q_math) = mem.classify(MemIdx::new(q)).expect("valid break node") {
        q_math.set_width(mem, Scaled(0));

        if eqtb.b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
            update_lr_stack_checked(mem, q_math, lr_ptr)?;
        }
    }

    if eqtb.b32s1(INT_BASE + INT_PAR_XETEX_PROTRUDE_CHARS) > 0 {
        q = insert_right_margin_kern(errors, 
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
        q = insert_left_margin_kern(errors, mem, fonts, linebreak, q, linebreak.last_leftmost_char)
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

fn post_line_break_suffix(cl: &mut crate::mem_types::ListStateRecord, 
    mem: &mut MemState,
    scanner: &mut ScannerState,
    linebreak: &mut LinebreakState,
    eqtb: &EqtbState,
    just_box: AnyBox,
    hpack: PostLineHpack,
    d: bool,
    lr_ptr: &mut i32,
    cur_line: &mut i32,
) -> Result<bool, PostLineAbort> {
    scanner.just_box = just_box.idx().0;
    just_box.set_shift_amount(mem, Scaled(hpack.cur_indent));

    if PRE_ADJUST_HEAD != scanner.pre_adjust_tail {
        mem.set_b32s1(cl.tail, mem.b32s1(PRE_ADJUST_HEAD));
        cl.tail = scanner.pre_adjust_tail;
    }

    scanner.pre_adjust_tail = TEX_NULL;
    crate::packaging::append_to_vlist(cl, mem, eqtb, just_box).map_err(PostLineAbort::Node)?;

    if ADJUST_HEAD != scanner.adjust_tail {
        mem.set_b32s1(cl.tail, mem.b32s1(ADJUST_HEAD));
        cl.tail = scanner.adjust_tail;
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
            let mut r = *cur_line - cl.prev_graf;
            if r > mem.b32s1(q + 1) {
                r = mem.b32s1(q + 1);
            }
            pen += mem.b32s1(q + r + 1);
        } else if *cur_line == cl.prev_graf + 1 {
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
            mem.set_b32s1(cl.tail, r);
            cl.tail = r;
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
                update_lr_stack_checked(mem, math_node(q), lr_ptr)?;
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
/// Deferred cold-abort signal from [`post_line_break_in`], returned to and
/// fired by the [`post_line_break`] wrapper. On a passing run the outcome is
/// always `Done`.
pub(crate) enum PostLineBreakOutcome {
    Done,
    Node(NodeRecycleAbort),
    PdfError {
        proc_name: &'static std::ffi::CStr,
        message: &'static std::ffi::CStr,
    },
    Confusion(&'static std::ffi::CStr),
}

impl PostLineBreakOutcome {
    /// Map to the unified [`EngineAbort`] terminal. `Done` never reaches a
    /// terminal.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            PostLineBreakOutcome::Done => {
                unreachable!("PostLineBreakOutcome::Done is never propagated as a terminal abort")
            }
            PostLineBreakOutcome::Node(e) => e.into_engine_abort(),
            PostLineBreakOutcome::PdfError { proc_name, message } => {
                crate::errors::EngineAbort::PdfError {
                    t: proc_name.as_ptr(),
                    p: message.as_ptr(),
                }
            }
            PostLineBreakOutcome::Confusion(message) => {
                crate::errors::EngineAbort::Confusion(message.as_ptr())
            }
        }
    }
}

/// Core of [`post_line_break`]. The `hpack` packaging (and its in-order
/// over/underfull diagnostic) runs via [`crate::packaging::hpack_emit_in`];
/// cold aborts ride the [`PostLineBreakOutcome`] sentinel.
fn post_line_break_in(st: &mut crate::state::EngineState, d: bool) -> PostLineBreakOutcome {
    let mut lr_ptr = st.input.cur_list.e_tex_aux;

    reverse_best_breaks(&mut st.mem, &mut st.linebreak);

    let mut cur_line = st.input.cur_list.prev_graf + 1;
    loop {
        let hpack = match post_line_break_prefix(
            &mut st.errors,
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

        let done = match post_line_break_suffix(&mut st.input.cur_list, 
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
    st.input.cur_list.prev_graf = best_line - 1;
    st.input.cur_list.e_tex_aux = lr_ptr;
    PostLineBreakOutcome::Done
}

pub(crate) enum TryBreakAbort {
    Node(NodeRecycleAbort),
    HlistStackFailure,
}

impl TryBreakAbort {
    /// Map to the unified [`EngineAbort`] terminal.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            TryBreakAbort::Node(e) => e.into_engine_abort(),
            TryBreakAbort::HlistStackFailure => crate::errors::EngineAbort::PdfError {
                t: c"total_pw".as_ptr(),
                p: c"hlist stack failure".as_ptr(),
            },
        }
    }
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
                                match mem.classify(MemIdx::new(v)).expect("disc node valid") {
                                    AnyNode::Ligature(lig) => {
                                        let f = lig.font(mem).0;
                                        fonts.set_xtx_ligature_present(true);
                                        linebreak.break_width[1] -=
                                            font_character_width(fonts, f, lig.character(mem));
                                    }
                                    AnyNode::HList(_)
                                    | AnyNode::VList(_)
                                    | AnyNode::Rule(_)
                                    | AnyNode::Kern(_) => {
                                        linebreak.break_width[1] -= typed_list_width_in(mem, v);
                                    }
                                    AnyNode::Whatsit(w) => {
                                        if w.whatsit_subtype(mem).is_some_and(|s| s.has_dimensions())
                                        {
                                            linebreak.break_width[1] -= w.width(mem).0;
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
                                match mem.classify(MemIdx::new(s)).expect("disc node valid") {
                                    AnyNode::Ligature(lig) => {
                                        let f = lig.font(mem).0;
                                        fonts.set_xtx_ligature_present(true);
                                        linebreak.break_width[1] +=
                                            font_character_width(fonts, f, lig.character(mem));
                                    }
                                    AnyNode::HList(_)
                                    | AnyNode::VList(_)
                                    | AnyNode::Rule(_)
                                    | AnyNode::Kern(_) => {
                                        linebreak.break_width[1] += typed_list_width_in(mem, s);
                                    }
                                    AnyNode::Whatsit(w) => {
                                        if w.whatsit_subtype(mem).is_some_and(|s| s.has_dimensions())
                                        {
                                            linebreak.break_width[1] += w.width(mem).0;
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
                        if cur_disc.post_break(mem).is_null() {
                            s = mem.b32s1(v);
                        }
                    }

                    while s != TEX_NULL {
                        if crate::mem::is_char_node(mem, s) {
                            break;
                        }

                        match mem.classify(MemIdx::new(s)).expect("disc node valid") {
                            AnyNode::Glue(g) => {
                                let v = g.glue_spec(mem).expect("glue node has spec");
                                linebreak.break_width[1] -= v.width(mem).0;
                                linebreak.break_width[2 + v.stretch_order(mem) as usize] -=
                                    v.stretch(mem).0;
                                linebreak.break_width[6] -= v.shrink(mem).0;
                            }
                            AnyNode::Penalty(_) => {}
                            AnyNode::Math(m) => {
                                linebreak.break_width[1] -= m.width(mem).0;
                            }
                            AnyNode::Kern(kern) => {
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
                    active(prev_r).set_link(mem, MemPtr::from_idx(q.idx()));
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
                            delta_node(prev_r).set_link(mem, MemPtr::from_idx(q_active.idx()));
                        } else {
                            active(prev_r).set_link(mem, MemPtr::from_idx(q_active.idx()));
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
                        delta_node(prev_r).set_link(mem, MemPtr::from_idx(q.idx()));
                    } else {
                        active(prev_r).set_link(mem, MemPtr::from_idx(q.idx()));
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
                shortfall += total_pw(errors, mem, fonts, linebreak, r, linebreak.cur_p)
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
                        g = crate::scaled_math::fract_in(
                            errors,
                            g,
                            r_active.shortfall(mem).0,
                            r_active.glue(mem).0,
                            MAX_HALFWORD,
                        );
                        if eqtb.b32s1(INT_BASE + INT_PAR_LAST_LINE_FIT) < 1000 {
                            g = crate::scaled_math::fract_in(
                                errors,
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

// translated from xetex-linebreak.c:2010 (hyphenate)
/// Cold-abort sentinel for the native-word hyphenation island in
/// [`hyphenate`]'s `goto_found`: the island returns this and the orchestrator
/// runs the divergent tail.
pub(crate) enum HyphNativeAbort {
    Recycle(NodeRecycleAbort),
    NativeChar(crate::fonts::NativeCharAbort),
    Confusion(&'static std::ffi::CStr),
    /// A native-font measurement cold abort.
    NativeFont(crate::ext::NativeFontAbort),
}

impl From<crate::ext::NativeFontAbort> for HyphNativeAbort {
    fn from(e: crate::ext::NativeFontAbort) -> Self {
        HyphNativeAbort::NativeFont(e)
    }
}

impl HyphNativeAbort {
    /// Map to the unified [`EngineAbort`] terminal.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            HyphNativeAbort::Recycle(e) => e.into_engine_abort(),
            HyphNativeAbort::NativeChar(nc) => nc.into_engine_abort(),
            HyphNativeAbort::Confusion(msg) => crate::errors::EngineAbort::Confusion(msg.as_ptr()),
            HyphNativeAbort::NativeFont(e) => e.into_engine_abort(),
        }
    }
}

pub(crate) enum HyphenateAbort {
    MainMemoryOverflow,
    Native(HyphNativeAbort),
    NewCharacter(crate::main_control::NewCharacterAbort),
    Recycle(NodeRecycleAbort),
    Confusion(&'static std::ffi::CStr),
}

impl HyphenateAbort {
    /// Map to the unified [`EngineAbort`] terminal. `MainMemoryOverflow`
    /// is a plain overflow (no `runaway`).
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        const MEM_TOP: i32 = 4_999_999;
        match self {
            HyphenateAbort::MainMemoryOverflow => crate::errors::EngineAbort::Overflow {
                what: c"main memory size".as_ptr(),
                n: MEM_TOP + 1,
            },
            HyphenateAbort::Native(a) => a.into_engine_abort(),
            HyphenateAbort::NewCharacter(nc) => nc.into_engine_abort(),
            HyphenateAbort::Recycle(e) => e.into_engine_abort(),
            HyphenateAbort::Confusion(msg) => crate::errors::EngineAbort::Confusion(msg.as_ptr()),
        }
    }
}

fn hyphenate_native_word_if_needed(
    st: &mut EngineState,
    hn_v: i16,
) -> Result<bool, HyphNativeAbort> {
    let ha = st.linebreak.ha;
    let is_native = ha != TEX_NULL
        && !crate::mem::is_char_node(&st.mem, ha)
        && st.mem.b16s1(ha) as i32 == WHATSIT_NODE
        && WhatsitSubtype::from_raw(st.mem.b16s0(ha)).is_some_and(|s| s.is_native_word());
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
                let qh = whatsit_node_in(&st.mem, q);
                let dst = qh.native_text_ptr_mut(&mut st.mem);
                let hah = whatsit_node_in(&st.mem, ha);
                let src = hah.native_text_ptr(&st.mem);
                for i in 0..=(j - hp - 1) {
                    *dst.add(i as usize) = *src.add((i + hp) as usize);
                }
                set_native_metrics(&st.mem, &st.eqtb, &st.fonts, q)?;
            }
            st.mem.set_b32s1(s, q);
            s = q;

            let q = crate::mem::new_disc(&mut st.mem);
            let nc = crate::fonts::new_native_character_in(st, hf, hyf_char as u32)
                .map_err(HyphNativeAbort::NativeChar)?;
            disc_node(q).set_pre_break(&mut st.mem, MemPtr::from_raw(nc));
            st.mem.set_b32s1(s, q);
            s = q;
            st.hyph.hyphen_passed = j;
        }
    }

    let new_hn = whatsit_node_in(&st.mem, ha).native_length(&st.mem) as i16;
    st.linebreak.hn = new_hn;
    let hp = st.hyph.hyphen_passed;
    let q = crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, hf, (new_hn - hp) as i32)
        .map_err(HyphNativeAbort::Recycle)?;
    let subtype = st.mem.b16s0(ha);
    st.mem.set_b16s0(q, subtype);
    // SAFETY: see above.
    unsafe {
        let qh = whatsit_node_in(&st.mem, q);
        let dst = qh.native_text_ptr_mut(&mut st.mem);
        let hah = whatsit_node_in(&st.mem, ha);
        let src = hah.native_text_ptr(&st.mem);
        for i in 0..=(new_hn - hp - 1) {
            *dst.add(i as usize) = *src.add((i + hp) as usize);
        }
        set_native_metrics(&st.mem, &st.eqtb, &st.fonts, q)?;
    }
    st.mem.set_b32s1(s, q);
    s = q;
    let after = st.mem.b32s1(ha);
    st.mem.set_b32s1(s, after);
    st.mem.set_b32s1(ha, TEX_NULL);
    crate::packaging::flush_node_list(&mut st.mem, ha).map_err(HyphNativeAbort::Confusion)?;
    Ok(true)
}

fn hyphenate_in(st: &mut EngineState) -> Result<(), HyphenateAbort> {
    let mut hn_v = st.linebreak.hn;
    for j in 0..=hn_v {
        st.hyph.hyf[j as usize] = 0;
    }

    let mut h = st.hyph.hc[1];
    hn_v += 1;
    st.linebreak.hn = hn_v;
    st.hyph.hc[hn_v as usize] = st.hyph.cur_lang as i32;

    for j in 2..=hn_v {
        h = (h + h + st.hyph.hc[j as usize]) % HYPH_PRIME;
    }

    loop {
        let k = st.hyph.hyph_word[h as usize];
        if k == 0 {
            break;
        }

        if crate::string_pool::length(&st.strings, k) == hn_v as i32 {
            let mut j = 1;
            let mut u = st.strings.str_start[(k - TOO_BIG_CHAR) as usize];
            loop {
                if st.strings.str_pool[u as usize] as i32 != st.hyph.hc[j as usize] {
                    break;
                }
                j += 1;
                u += 1;
                if j > hn_v {
                    let mut s = st.hyph.hyph_list[h as usize];
                    while s != TEX_NULL {
                        st.hyph.hyf[st.mem.b32s0(s) as usize] = 1;
                        s = st.mem.b32s1(s);
                    }
                    hn_v -= 1;
                    st.linebreak.hn = hn_v;
                    goto_found(st, hn_v)?;
                    return Ok(());
                }
            }
        }

        let link = st.hyph.hyph_link[h as usize];
        if link == 0 {
            break;
        }
        h = link as i32 - 1;
    }

    hn_v -= 1;
    st.linebreak.hn = hn_v;
    if st.hyph.trie_trc[st.hyph.cur_lang as usize + 1] != st.hyph.cur_lang as u16 {
        return Ok(());
    }

    st.hyph.hc[0] = 0;
    st.hyph.hc[(hn_v + 1) as usize] = 0;
    st.hyph.hc[(hn_v + 2) as usize] = st.hyph.max_hyph_char;

    for j in 0..=(hn_v - st.hyph.r_hyf as i16 + 1) {
        let mut z = st.hyph.trie_trl[st.hyph.cur_lang as usize + 1] + st.hyph.hc[j as usize];
        let mut l = j;
        while st.hyph.hc[l as usize] == st.hyph.trie_trc[z as usize] as i32 {
            if st.hyph.trie_tro[z as usize] != MIN_TRIE_OP {
                let mut v = st.hyph.trie_tro[z as usize];
                loop {
                    v += st.hyph.op_start[st.hyph.cur_lang as usize];
                    let i = l - st.hyph.hyf_distance[v as usize];
                    if st.hyph.hyf_num[v as usize] as u8 > st.hyph.hyf[i as usize] {
                        st.hyph.hyf[i as usize] = st.hyph.hyf_num[v as usize] as u8;
                    }
                    v = st.hyph.hyf_next[v as usize] as i32;
                    if v == MIN_TRIE_OP {
                        break;
                    }
                }
            }
            l += 1;
            z = st.hyph.trie_trl[z as usize] + st.hyph.hc[l as usize];
        }
    }

    goto_found(st, hn_v)
}

fn goto_found(st: &mut EngineState, hn_v: i16) -> Result<(), HyphenateAbort> {
    for j in 0..=(st.hyph.l_hyf as i16 - 1) {
        st.hyph.hyf[j as usize] = 0;
    }
    for j in 0..=(st.hyph.r_hyf as i16 - 1) {
        st.hyph.hyf[(hn_v - j) as usize] = 0;
    }

    let mut any_hyphen = false;
    for j in st.hyph.l_hyf as i16..=(hn_v - st.hyph.r_hyf as i16) {
        if odd(st.hyph.hyf[j as usize]) {
            any_hyphen = true;
            break;
        }
    }
    if !any_hyphen {
        return Ok(());
    }

    // Native-word hyphenation: split `ha`'s native_word_node into measured
    // fragments separated by discretionaries whose pre-break is a fresh
    // native hyphen character. `new_native_word_node`, `set_native_metrics`,
    // `new_disc`, `new_native_character_in` and `flush_node_list` run in the
    // original allocation order. Cold aborts ride `HyphNativeAbort`.
    match hyphenate_native_word_if_needed(st, hn_v).map_err(HyphenateAbort::Native)? {
        true => return Ok(()),
        false => {}
    }

    let q_final = st.mem.b32s1(st.linebreak.hb);
    st.mem.set_b32s1(st.linebreak.hb, TEX_NULL);
    let r = st.mem.b32s1(st.linebreak.ha);
    st.mem.set_b32s1(st.linebreak.ha, TEX_NULL);
    let bchar = st.hyph.hyf_bchar;

    let (mut s, mut j) = if crate::mem::is_char_node(&st.mem, st.linebreak.ha) {
        if st.mem.b16s1(st.linebreak.ha) as i32 != st.hyph.hf {
            st.hyph.hu[0] = st.hyph.max_hyph_char;
            st.hyph.init_lig = false;
            st.hyph.init_list = TEX_NULL;
            (st.linebreak.ha, 0)
        } else {
            st.hyph.init_list = st.linebreak.ha;
            st.hyph.init_lig = false;
            st.hyph.hu[0] = st.mem.b16s0(st.linebreak.ha) as i32;
            let mut s = st.linebreak.cur_p;
            while st.mem.b32s1(s) != st.linebreak.ha {
                s = st.mem.b32s1(s);
            }
            (s, 0)
        }
    } else if st.mem.b16s1(st.linebreak.ha) as i32 == LIGATURE_NODE {
        let ha_lig = st
            .mem
            .classify_ligature(MemIdx::new(st.linebreak.ha))
            .expect("valid ligature node type")
            .expect("ligature branch");
        if ha_lig.font(&st.mem).0 != st.hyph.hf {
            st.hyph.hu[0] = st.hyph.max_hyph_char;
            st.hyph.init_lig = false;
            st.hyph.init_list = TEX_NULL;
            (st.linebreak.ha, 0)
        } else {
            st.hyph.init_list = ha_lig.lig_ptr(&st.mem).0;
            st.hyph.init_lig = true;
            st.hyph.init_lft = ha_lig.subtype(&st.mem) > 1;
            st.hyph.hu[0] = ha_lig.character(&st.mem) as i32;
            if st.hyph.init_list == TEX_NULL && st.hyph.init_lft {
                st.hyph.hu[0] = st.hyph.max_hyph_char;
                st.hyph.init_lig = false;
            }
            crate::mem::free_node(&mut st.mem, st.linebreak.ha, SMALL_NODE_SIZE);
            let mut s = st.linebreak.cur_p;
            while st.mem.b32s1(s) != st.linebreak.ha {
                s = st.mem.b32s1(s);
            }
            (s, 0)
        }
    } else if !crate::mem::is_char_node(&st.mem, r)
        && st.mem.b16s1(r) as i32 == LIGATURE_NODE
        && st
            .mem
            .classify_ligature(MemIdx::new(r))
            .expect("valid ligature node type")
            .expect("ligature branch")
            .subtype(&st.mem)
            > 1
    {
        st.hyph.hu[0] = st.hyph.max_hyph_char;
        st.hyph.init_lig = false;
        st.hyph.init_list = TEX_NULL;
        (st.linebreak.ha, 0)
    } else {
        st.hyph.init_list = TEX_NULL;
        (st.linebreak.ha, 1)
    };

    crate::packaging::flush_node_list(&mut st.mem, r).map_err(HyphenateAbort::Confusion)?;

    loop {
        let mut l = j;
        let hyf_char = st.hyph.hyf_char;
        j = reconstitute(
            st,
            j,
            hn_v,
            bchar,
            hyf_char,
        )
        .map_err(HyphenateAbort::Recycle)?
            + 1;
        if st.hyph.hyphen_passed == 0 {
            st.mem.set_b32s1(s, st.mem.b32s1(HOLD_HEAD));
            while st.mem.b32s1(s) > TEX_NULL {
                s = st.mem.b32s1(s);
            }
            if odd(st.hyph.hyf[(j - 1) as usize]) {
                l = j;
                st.hyph.hyphen_passed = j - 1;
                st.mem.set_b32s1(HOLD_HEAD, TEX_NULL);
            }
        }

        if st.hyph.hyphen_passed > 0 {
            loop {
                let r_node = NodePtr::<DiscNode>::alloc(&mut st.mem)
                    .ok_or(HyphenateAbort::MainMemoryOverflow)?;
                let r = r_node.idx().0;
                let hold = st.mem.b32s1(HOLD_HEAD);
                r_node.set_link(&mut st.mem, MemPtr::from_raw(hold));
                let mut major_tail = r;
                let mut r_count = 0;
                while st.mem.b32s1(major_tail) > TEX_NULL {
                    major_tail = st.mem.b32s1(major_tail);
                    r_count += 1;
                }

                let mut i = st.hyph.hyphen_passed;
                st.hyph.hyf[i as usize] = 0;
                let mut minor_tail = TEX_NULL;
                r_node.set_pre_break(&mut st.mem, MemPtr::from_raw(TEX_NULL));
                let hyf_node =
                    crate::main_control::new_character_in(st, st.hyph.hf, st.hyph.hyf_char as u16)
                        .map_err(HyphenateAbort::NewCharacter)?;
                let mut saved_c = 0;
                if hyf_node != TEX_NULL {
                    i += 1;
                    saved_c = st.hyph.hu[i as usize];
                    st.hyph.hu[i as usize] = st.hyph.hyf_char;
                    st.mem.set_b32s1(hyf_node, st.mem.avail);
                    st.mem.avail = hyf_node;
                }

                while l <= i {
                    let font_bchar = st.fonts.font_bchar_entry(st.hyph.hf);
                    l = reconstitute(
                        st,
                        l,
                        i,
                        font_bchar,
                        TOO_BIG_CHAR,
                    )
                    .map_err(HyphenateAbort::Recycle)?
                        + 1;
                    if st.mem.b32s1(HOLD_HEAD) > TEX_NULL {
                        if minor_tail == TEX_NULL {
                            let hold = st.mem.b32s1(HOLD_HEAD);
                            r_node.set_pre_break(&mut st.mem, MemPtr::from_raw(hold));
                        } else {
                            st.mem.set_b32s1(minor_tail, st.mem.b32s1(HOLD_HEAD));
                        }
                        minor_tail = st.mem.b32s1(HOLD_HEAD);
                        while st.mem.b32s1(minor_tail) > TEX_NULL {
                            minor_tail = st.mem.b32s1(minor_tail);
                        }
                    }
                }

                if hyf_node != TEX_NULL {
                    st.hyph.hu[i as usize] = saved_c;
                    l = i;
                    i -= 1;
                }

                minor_tail = TEX_NULL;
                r_node.set_post_break(&mut st.mem, MemPtr::from_raw(TEX_NULL));
                let mut c_loc = 0;
                let mut c = 0;
                if st.fonts.bchar_label_entry(st.hyph.hf) != NON_ADDRESS {
                    l -= 1;
                    c = st.hyph.hu[l as usize];
                    c_loc = l;
                    st.hyph.hu[l as usize] = st.hyph.max_hyph_char;
                }

                while l < j {
                    loop {
                        l = reconstitute(
                            st,
                            l,
                            hn_v,
                            bchar,
                            TOO_BIG_CHAR,
                        )
                        .map_err(HyphenateAbort::Recycle)?
                            + 1;
                        if c_loc > 0 {
                            st.hyph.hu[c_loc as usize] = c;
                            c_loc = 0;
                        }
                        if st.mem.b32s1(HOLD_HEAD) > TEX_NULL {
                            if minor_tail == TEX_NULL {
                                let hold = st.mem.b32s1(HOLD_HEAD);
                                r_node.set_post_break(&mut st.mem, MemPtr::from_raw(hold));
                            } else {
                                st.mem.set_b32s1(minor_tail, st.mem.b32s1(HOLD_HEAD));
                            }
                            minor_tail = st.mem.b32s1(HOLD_HEAD);
                            while st.mem.b32s1(minor_tail) > TEX_NULL {
                                minor_tail = st.mem.b32s1(minor_tail);
                            }
                        }
                        if l >= j {
                            break;
                        }
                    }

                    while l > j {
                        j = reconstitute(
                            st,
                            j,
                            hn_v,
                            bchar,
                            TOO_BIG_CHAR,
                        )
                        .map_err(HyphenateAbort::Recycle)?
                            + 1;
                        st.mem.set_b32s1(major_tail, st.mem.b32s1(HOLD_HEAD));
                        while st.mem.b32s1(major_tail) > TEX_NULL {
                            major_tail = st.mem.b32s1(major_tail);
                            r_count += 1;
                        }
                    }
                }

                if r_count > 127 {
                    let replacement = r_node.link(&st.mem);
                    st.mem.set_b32s1(s, replacement.0);
                    r_node.set_link(&mut st.mem, MemPtr::from_raw(TEX_NULL));
                    crate::packaging::flush_node_list(&mut st.mem, r)
                        .map_err(HyphenateAbort::Confusion)?;
                } else {
                    st.mem.set_b32s1(s, r);
                    r_node.set_subtype(&mut st.mem, r_count as u16);
                }
                s = major_tail;
                st.hyph.hyphen_passed = j - 1;
                st.mem.set_b32s1(HOLD_HEAD, TEX_NULL);

                if !odd(st.hyph.hyf[(j - 1) as usize]) {
                    break;
                }
            }
        }

        if j > hn_v {
            break;
        }
    }

    st.mem.set_b32s1(s, q_final);
    crate::mem::flush_list(&mut st.mem, st.hyph.init_list);
    Ok(())
}

// translated from xetex-xetex0.c:6968 (char_pw)
pub fn char_pw_in(errors: &mut crate::state::ErrorState, 
    mem: &MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    p: i32,
    side: i16,
) -> i32 {
    const WHATSIT_NODE: u16 = 8;
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
        && WhatsitSubtype::from_raw(mem.b16s0(p)).is_some_and(|s| s.is_native_word())
    {
        if !whatsit_node_in(mem, p).native_glyph_info_ptr(mem).is_null() {
            // NATIVE_NODE_glyph_info_ptr != NULL
            let f = whatsit_node_in(mem, p).native_font(mem).0; // NATIVE_NODE_font
            let cp = unsafe { real_get_native_word_cp(mem.node_addr(p), side as libc::c_int) };
            return crate::scaled_math::round_xn_over_d(errors, 
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
        && WhatsitSubtype::from_raw(mem.b16s0(p)) == Some(WhatsitSubtype::Glyph)
    {
        let w = whatsit_node_in(mem, p);
        let f = w.native_font(mem).0; // mem[p + 4].b16.s2
        let code = w.native_glyph(mem) as u32; // mem[p + 4].b16.s1 (char code)
        let cp = unsafe { get_cp_code(f, code, side as i32) };
        return crate::scaled_math::round_xn_over_d(errors, 
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
    crate::scaled_math::round_xn_over_d(errors, 
        fonts.font_info_b32s1(QUAD_CODE + fonts.param_base(f)),
        c,
        1000,
    )
}
/// Insert locale-aware linebreaking nodes for `len` glyphs starting at `native_text[s]`.
///
/// translated from xetex-xetex0.c:6493 (do_locale_linebreaks)
pub(crate) unsafe fn do_locale_linebreaks_in(
    st: &mut crate::state::EngineState,
    s: i32,
    len: i32,
) -> Result<(), NodeRecycleAbort> {
    // The main-loop current font.
    // Bound to a local (it is read-only here) so the reads below need no `st`
    // reborrow alongside the `linebreak_start_in(st, …)` call.
    let main_f = st.mainloop.main_f;
    // native_text is a scanner Vec that is not resized during this routine, so a
    // raw pointer captured once stays valid across the mem allocations below.
    let native_text = st.scanner.native_text.as_mut_ptr();

    let locale = st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_LINEBREAK_LOCALE);
    let use_glyph = st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0;

    if locale == 0 || len == 1 {
        let node = crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, main_f, len)?;
        st.mem.set_b32s1(st.input.cur_list.tail, node);
        st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);
        let t = st.input.cur_list.tail;
        let text = whatsit_node_in(&st.mem, t).native_text_ptr_mut(&mut st.mem);
        for i in 0..len {
            *text.add(i as usize) = *native_text.add((s + i) as usize);
        }
        let addr = st.mem.node_addr(st.input.cur_list.tail);
        measure_native_node(&st.fonts, addr, use_glyph as libc::c_int)?;
    } else {
        let use_skip = st.eqtb.b32s1(GLUE_BASE + GLUE_PAR_XETEX_LINEBREAK_SKIP) != 0;
        let penalty_val = st.eqtb.b32s1(INT_BASE + INT_PAR_XETEX_LINEBREAK_PENALTY);
        let use_penalty = penalty_val != 0 || !use_skip;
        linebreak_start_in(st, main_f, locale, native_text.add(s as usize), len)?;
        let mut offs: i32 = 0;
        loop {
            let prev_offs = offs;
            offs = linebreak_next(&st.fonts, main_f);
            if offs > 0 {
                if prev_offs != 0 {
                    if use_penalty {
                        let node = crate::mem::new_penalty(&mut st.mem, penalty_val);
                        if node == crate::mem::ALLOC_FAILED {
                            return Err(NodeRecycleAbort::Overflow);
                        }
                        st.mem.set_b32s1(st.input.cur_list.tail, node);
                        st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);
                    }
                    if use_skip {
                        let node = crate::mem::new_param_glue(
                            &mut st.mem,
                            &st.eqtb,
                            GLUE_PAR_XETEX_LINEBREAK_SKIP as i16,
                        );
                        if node == crate::mem::ALLOC_FAILED {
                            return Err(NodeRecycleAbort::Overflow);
                        }
                        st.mem.set_b32s1(st.input.cur_list.tail, node);
                        st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);
                    }
                }
                let node = crate::fonts::new_native_word_node(
                    &mut st.mem,
                    &st.eqtb,
                    main_f,
                    offs - prev_offs,
                )?;
                st.mem.set_b32s1(st.input.cur_list.tail, node);
                st.input.cur_list.tail = st.mem.b32s1(st.input.cur_list.tail);
                let t = st.input.cur_list.tail;
                let text = whatsit_node_in(&st.mem, t).native_text_ptr_mut(&mut st.mem);
                for i in prev_offs..offs {
                    *text.add((i - prev_offs) as usize) = *native_text.add((s + i) as usize);
                }
                let addr = st.mem.node_addr(st.input.cur_list.tail);
                measure_native_node(&st.fonts, addr, use_glyph as libc::c_int)?;
            }
            if offs < 0 {
                break;
            }
        }
    }
    Ok(())
}
