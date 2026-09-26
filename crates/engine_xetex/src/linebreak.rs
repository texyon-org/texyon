//! The paragraph builder: `line_break`, `post_line_break`, `hyphenate` and
//! character protrusion.

use crate::consts::{
    ADJUST_HEAD, AWFUL_BAD, DIMEN_PAR_HANG_INDENT, DIMEN_PAR_HSIZE, EJECT_PENALTY, ETEX_PEN_BASE,
    EXPLICIT, GLUE_BASE, INT_PAR_HANG_AFTER, INT_PAR_LOOSENESS, INT_PAR_TEXXET, LC_CODE_BASE,
    LIG_TAG, LOCAL_BASE, LOCAL_PAR_SHAPE, MAX_HALFWORD, MEM_TOP, MIN_TRIE_OP, NON_ADDRESS,
    RULE_NODE, SMALL_NODE_SIZE, SPACE_ADJUSTMENT, TEX_NULL, TOO_BIG_CHAR,
};
use crate::ext::{linebreak_next, linebreak_start_in, measure_native_node};
use crate::hyphenation::{hyph_code, set_hyph_index, HYPH_PRIME};
use crate::mem::is_char_node;
use crate::node::{
    ActiveNode, AnyBox, AnyNode, DeltaNode, DiscNode, GlueNode, GlueSpecNode, HListNode, LigItem,
    MathNode, NodePtr, PassiveNode, WhatsitNode, WhatsitSubtype,
};
use crate::packaging::{as_native_word, dimen_par, int_par, list_tail, predecessor};
use crate::scan::NodeRecycleAbort;
use crate::state::{
    ConfigState, EngineState, EqtbState, ErrorState, FontState, LinebreakState, MemState,
    ScannerState,
};
use crate::units::{MemAddr, MemIdx, MemPtr, Scaled};

const GLUE_PAR_LEFT_SKIP: i16 = 7;
const GLUE_PAR_RIGHT_SKIP: i16 = 8;
const GLUE_PAR_PAR_FILL_SKIP: i16 = 14;
const GLUE_PAR_XETEX_LINEBREAK_SKIP: i16 = 15;
const INT_PAR_PRETOLERANCE: i32 = 0;
const INT_PAR_TOLERANCE: i32 = 1;
const INT_PAR_LINE_PENALTY: i32 = 2;
const INT_PAR_HYPHEN_PENALTY: i32 = 3;
const INT_PAR_EX_HYPHEN_PENALTY: i32 = 4;
const INT_PAR_CLUB_PENALTY: i32 = 5;
const INT_PAR_WIDOW_PENALTY: i32 = 6;
const INT_PAR_DISPLAY_WIDOW_PENALTY: i32 = 7;
const INT_PAR_BROKEN_PENALTY: i32 = 8;
const INT_PAR_INTER_LINE_PENALTY: i32 = 13;
const INT_PAR_DOUBLE_HYPHEN_DEMERITS: i32 = 14;
const INT_PAR_FINAL_HYPHEN_DEMERITS: i32 = 15;
const INT_PAR_ADJ_DEMERITS: i32 = 16;
const INT_PAR_UC_HYPH: i32 = 38;
const INT_PAR_LAST_LINE_FIT: i32 = 62;
const INT_PAR_XETEX_LINEBREAK_LOCALE: i32 = 66;
const INT_PAR_XETEX_LINEBREAK_PENALTY: i32 = 67;
const INT_PAR_XETEX_PROTRUDE_CHARS: i32 = 68;
const INT_PAR_XETEX_USE_GLYPH_METRICS: i32 = 72;
const DIMEN_PAR_EMERGENCY_STRETCH: i32 = 20;
const ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES: i32 = 0;
const ETEX_PENALTIES_PAR_CLUB_PENALTIES: i32 = 1;
const ETEX_PENALTIES_PAR_WIDOW_PENALTIES: i32 = 2;
const ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES: i32 = 3;

const HOLD_HEAD: MemPtr = MemPtr(MEM_TOP - 4);
const TEMP_HEAD: MemPtr = MemPtr(MEM_TOP - 3);
const ACTIVE_LIST: MemPtr = MemPtr(MEM_TOP - 7);
const LAST_ACTIVE: MemPtr = ACTIVE_LIST;
const PRE_ADJUST_HEAD: MemPtr = MemPtr(MEM_TOP - 14);
const ZERO_GLUE: MemPtr = MemPtr(0);

const DELTA_NODE: u16 = RULE_NODE;
const PASSIVE_NODE_SIZE: i32 = 2;
const ACTIVE_NODE_SIZE_NORMAL: i16 = 3;
const ACTIVE_NODE_SIZE_EXTENDED: i16 = 5;
const DELTA_NODE_SIZE: i32 = 7;
const MAX_HLIST_STACK: i16 = 512;
const BEST_BET_SET: &str = "best_bet is set by choose_best_break before use";

const NORMAL: u16 = 0;
const L_CODE: u16 = 4;
const EXACTLY: i16 = 0;
const ADDITIONAL: i16 = 1;
const UNHYPHENATED: i16 = 0;
const HYPHENATED: i16 = 1;
const VERY_LOOSE_FIT: usize = 0;
const LOOSE_FIT: usize = 1;
const DECENT_FIT: usize = 2;
const TIGHT_FIT: usize = 3;
const INF_BAD: i32 = 10_000;
const INF_PENALTY: i32 = 10_000;
const QUAD_CODE: i32 = 6;

extern "C" {
    fn get_cp_code(font_num: i32, code: u32, side: i32) -> i32;
}

#[inline]
fn odd(v: u8) -> bool {
    v % 2 == 1
}

/// The spec of glue parameter `n` (never null).
#[inline]
fn glue_par(eqtb: &EqtbState, n: i16) -> NodePtr<GlueSpecNode> {
    glue_spec(MemPtr(eqtb.b32s1(GLUE_BASE + n as i32)))
}

#[inline]
fn glue_spec(spec: MemPtr) -> NodePtr<GlueSpecNode> {
    NodePtr::<GlueSpecNode>::from_glue_ptr(spec).expect("non-null glue spec")
}

#[inline]
fn has_infinite_shrink(mem: &MemState, spec: NodePtr<GlueSpecNode>) -> bool {
    spec.shrink_order(mem) != NORMAL && spec.shrink(mem).0 != 0
}

#[inline]
fn font_character_width(fonts: &mut FontState, f: i32, c: u16) -> i32 {
    let ec = crate::fonts::effective_char(fonts, true, f, c);
    let info = fonts.font_info_b16(fonts.char_base_entry(f) + ec);
    fonts.font_info_b32s1(fonts.width_base_entry(f) + info.s3 as i32)
}

/// A node on the active list, which carries no `NODE_type` of its own.
#[inline]
fn active(p: impl MemAddr) -> NodePtr<ActiveNode> {
    let p = p.addr();
    // SAFETY: the caller is walking the active list; `p` is a real active
    // node (the context boundary that justifies `from_idx_unchecked`).
    unsafe { NodePtr::<ActiveNode>::from_idx_unchecked(MemIdx::new(p)) }
}

#[inline]
fn delta_node(p: impl MemAddr) -> NodePtr<DeltaNode> {
    let p = p.addr();
    // SAFETY: the caller is walking the active list and has already established
    // this node as `DELTA_NODE` in that list context.
    unsafe { NodePtr::<DeltaNode>::from_idx_unchecked(MemIdx::new(p)) }
}

#[inline]
fn passive_node(p: impl MemAddr) -> NodePtr<PassiveNode> {
    let p = p.addr();
    // SAFETY: the caller is walking or constructing the passive list; passive
    // nodes are context-classified records with no NODE_type header.
    unsafe { NodePtr::<PassiveNode>::from_idx_unchecked(MemIdx::new(p)) }
}

/// An active-list entry. Its type word holds the break type (0/1) or
/// `DELTA_NODE` (2), which `classify()` would misread as HList/VList/Rule.
#[derive(Copy, Clone)]
enum ActiveOrDelta {
    Active(NodePtr<ActiveNode>),
    Delta(NodePtr<DeltaNode>),
}

#[inline]
fn classify_active_or_delta(mem: &MemState, p: impl MemAddr) -> ActiveOrDelta {
    let p = p.addr();
    if mem.b16s1(p) == DELTA_NODE {
        ActiveOrDelta::Delta(delta_node(p))
    } else {
        ActiveOrDelta::Active(active(p))
    }
}

/// A delta node's six components, indexed like `active_width` (slot 0 unused).
#[inline]
fn delta_widths(d: NodePtr<DeltaNode>, mem: &MemState) -> [i32; 7] {
    [
        0,
        d.dwidth(mem).0,
        d.dstretch0(mem).0,
        d.dstretch1(mem).0,
        d.dstretch2(mem).0,
        d.dstretch3(mem).0,
        d.dshrink(mem).0,
    ]
}

#[inline]
fn set_delta_widths(d: NodePtr<DeltaNode>, mem: &mut MemState, w: &[i32; 7]) {
    d.set_dwidth(mem, Scaled(w[1]));
    d.set_dstretch0(mem, Scaled(w[2]));
    d.set_dstretch1(mem, Scaled(w[3]));
    d.set_dstretch2(mem, Scaled(w[4]));
    d.set_dstretch3(mem, Scaled(w[5]));
    d.set_dshrink(mem, Scaled(w[6]));
}

fn alloc_active_node(
    mem: &mut MemState,
    active_node_size: i32,
    link: MemPtr,
) -> Result<NodePtr<ActiveNode>, NodeRecycleAbort> {
    let p = crate::mem::get_node(mem, active_node_size).ok_or(NodeRecycleAbort::Overflow)?;
    let p = active(p);
    p.set_link(mem, link);
    Ok(p)
}

/// A new delta node before `link` holding `w[1..=6]`.
fn alloc_delta_node(
    mem: &mut MemState,
    link: MemPtr,
    w: &[i32; 7],
) -> Result<NodePtr<DeltaNode>, NodeRecycleAbort> {
    let p = crate::mem::get_node(mem, DELTA_NODE_SIZE).ok_or(NodeRecycleAbort::Overflow)?;
    let p = delta_node(p);
    p.set_link(mem, link);
    mem.set_b16s1(p.idx().0, DELTA_NODE);
    mem.set_b16s0(p.idx().0, 0);
    set_delta_widths(p, mem, w);
    Ok(p)
}

fn alloc_passive_node(mem: &mut MemState) -> Result<NodePtr<PassiveNode>, NodeRecycleAbort> {
    let p = crate::mem::get_node(mem, PASSIVE_NODE_SIZE).ok_or(NodeRecycleAbort::Overflow)?;
    Ok(passive_node(p))
}

/// The Unicode scalar at UTF-16 offset `i` of `w`'s text.
fn get_native_usv(mem: &MemState, w: NodePtr<WhatsitNode>, i: i32) -> i32 {
    let text = w.native_text_ptr(mem);
    // SAFETY: `i` lies within the node's text.
    let c = unsafe { *text.add(i as usize) };

    if (0xD800..0xDC00).contains(&c) {
        // SAFETY: a high surrogate is followed by its low surrogate.
        let lo = unsafe { *text.add(i as usize + 1) };
        return 0x10000 + (c as i32 - 0xD800) * 0x400 + lo as i32 - 0xDC00;
    }

    c as i32
}

/// Copy `len` UTF-16 units of `src`'s text, starting at `start`, into `dst`.
fn copy_native_text(
    mem: &mut MemState,
    dst: NodePtr<WhatsitNode>,
    src: NodePtr<WhatsitNode>,
    start: i32,
    len: i32,
) {
    if len <= 0 {
        return;
    }
    let dst = dst.native_text_ptr_mut(mem);
    let src = src.native_text_ptr(mem);
    // SAFETY: both nodes are sized for the copied range, are distinct, and the
    // arena is not reallocated in between.
    unsafe { std::ptr::copy_nonoverlapping(src.add(start as usize), dst, len as usize) };
}

pub(crate) fn set_native_metrics(
    mem: &mut MemState,
    eqtb: &EqtbState,
    fonts: &FontState,
    w: NodePtr<WhatsitNode>,
) -> Result<(), crate::ext::NativeFontAbort> {
    let use_glyph_metrics = int_par(eqtb, INT_PAR_XETEX_USE_GLYPH_METRICS) > 0;
    // SAFETY: the measurer only reads font tables over FFI.
    unsafe { measure_native_node(fonts, mem, w, use_glyph_metrics) }
}

/// A native-word node of font `f` holding `len` units of `src`'s text from `start`.
fn native_word_fragment(
    mem: &mut MemState,
    eqtb: &EqtbState,
    fonts: &FontState,
    f: i32,
    src: NodePtr<WhatsitNode>,
    start: i32,
    len: i32,
) -> Result<NodePtr<WhatsitNode>, NodeRecycleAbort> {
    let q = crate::fonts::new_native_word_node(mem, eqtb, f, len)?;
    q.set_subtype(mem, src.subtype(mem));
    copy_native_text(mem, q, src, start, len);
    set_native_metrics(mem, eqtb, fonts, q)?;
    Ok(q)
}

/// Split the native word `ha` at UTF-16 offset `start`, returning the tail
/// linked after it.
fn split_native_word(
    st: &mut EngineState,
    ha: NodePtr<WhatsitNode>,
    start: i32,
) -> Result<NodePtr<WhatsitNode>, NodeRecycleAbort> {
    let len = ha.native_length(&st.mem) as i32;
    let tail = native_word_fragment(
        &mut st.mem,
        &st.eqtb,
        &st.fonts,
        st.hyph.hf,
        ha,
        start,
        len - start,
    )?;
    let next = ha.link(&st.mem);
    tail.set_link(&mut st.mem, next);
    ha.set_link(&mut st.mem, tail.ptr());
    ha.set_native_length(&mut st.mem, start as u16);
    set_native_metrics(&mut st.mem, &st.eqtb, &st.fonts, ha)?;
    Ok(tail)
}

/// Whether `p` is a char node or a node that glue may break after.
#[inline]
fn is_non_discardable(mem: &MemState, p: MemPtr) -> bool {
    is_char_node(mem, p)
        || mem
            .classify(MemIdx::new(p))
            .is_ok_and(|n| n.precedes_break())
}

#[inline]
fn is_empty_hlist(mem: &MemState, b: NodePtr<HListNode>) -> bool {
    b.width(mem).0 == 0 && b.height(mem).0 == 0 && b.depth(mem).0 == 0 && b.list_ptr(mem).is_null()
}

#[inline]
fn is_empty_box(mem: &MemState, p: MemPtr) -> bool {
    matches!(mem.classify_box(MemIdx::new(p)), Ok(Some(AnyBox::HList(b))) if is_empty_hlist(mem, b))
}

/// The list of `p` if `p` is an hlist with a nonempty list.
#[inline]
fn nonempty_hlist(mem: &MemState, p: MemPtr) -> Option<MemPtr> {
    match mem.classify_box(MemIdx::new(p)) {
        Ok(Some(AnyBox::HList(b))) => Some(b.list_ptr(mem)).filter(|list| !list.is_null()),
        _ => None,
    }
}

fn ignorable_protrusion_node(mem: &MemState, p: MemPtr) -> bool {
    if is_char_node(mem, p) {
        return false;
    }
    match mem.classify(MemIdx::new(p)) {
        Ok(AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_) | AnyNode::Penalty(_)) => true,
        Ok(AnyNode::Disc(d)) => {
            d.pre_break(mem).is_null() && d.post_break(mem).is_null() && d.subtype(mem) == 0
        }
        Ok(AnyNode::Math(m)) => m.width(mem).0 == 0,
        Ok(AnyNode::Kern(k)) => k.width(mem).0 == 0 || k.subtype(mem) == NORMAL,
        Ok(AnyNode::Glue(g)) => g.glue_ptr(mem) == ZERO_GLUE,
        Ok(AnyNode::HList(h)) => is_empty_hlist(mem, h),
        _ => false,
    }
}

fn push_node(linebreak: &mut LinebreakState, p: MemPtr) -> bool {
    let level = linebreak.hlist_stack_level;
    if level > MAX_HLIST_STACK {
        return false;
    }

    linebreak.hlist_stack[level as usize] = p;
    linebreak.hlist_stack_level = level + 1;
    true
}

fn pop_node(linebreak: &mut LinebreakState) -> Option<MemPtr> {
    let level = linebreak.hlist_stack_level - 1;
    linebreak.hlist_stack_level = level;
    if level < 0 {
        return None;
    }

    Some(linebreak.hlist_stack[level as usize])
}

/// Total protrusion of the line from `q`'s break to `cur_p`, which is `p_disc`
/// for a hyphenated break.
fn total_pw(
    errors: &mut ErrorState,
    mem: &MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    q: NodePtr<ActiveNode>,
    p_disc: Option<NodePtr<DiscNode>>,
) -> Option<i32> {
    let break_node = q.break_node(mem);
    let mut l = if break_node.is_null() {
        linebreak.first_p
    } else {
        passive_node(break_node).cur_break(mem)
    };

    let r = crate::mem::prev_rightmost(mem, linebreak.global_prev_p, linebreak.cur_p);
    let r = match p_disc {
        Some(d) if !d.pre_break(mem).is_null() => list_tail(mem, d.pre_break(mem)),
        _ => find_protchar_right(mem, linebreak, l, r)?,
    };

    let l_disc = if l.is_null() {
        None
    } else {
        mem.classify_disc(MemIdx::new(l)).ok().flatten()
    };
    l = match l_disc {
        Some(d) if !d.post_break(mem).is_null() => d.post_break(mem),
        Some(d) => {
            l = d.link(mem);
            for _ in 0..d.subtype(mem) {
                if !l.link(mem).is_null() {
                    l = l.link(mem);
                }
            }
            find_protchar_left(mem, linebreak, l, true)?
        }
        None => find_protchar_left(mem, linebreak, l, true)?,
    };

    Some(
        char_pw(errors, mem, fonts, linebreak, l, 0) + char_pw(errors, mem, fonts, linebreak, r, 1),
    )
}

fn find_protchar_left(
    mem: &MemState,
    linebreak: &mut LinebreakState,
    mut l: MemPtr,
    d: bool,
) -> Option<MemPtr> {
    if is_empty_box(mem, l) {
        l = l.link(mem);
    } else if d {
        while !l.link(mem).is_null() && !is_non_discardable(mem, l) {
            l = l.link(mem);
        }
    }

    linebreak.hlist_stack_level = 0;
    let mut run = true;
    loop {
        let t = l;
        if run {
            while let Some(list) = nonempty_hlist(mem, l) {
                if !push_node(linebreak, l) {
                    return None;
                }
                l = list;
            }
        }

        while run && ignorable_protrusion_node(mem, l) {
            while l.link(mem).is_null() && linebreak.hlist_stack_level > 0 {
                l = pop_node(linebreak)?;
            }

            if !l.link(mem).is_null() {
                l = l.link(mem);
            } else if linebreak.hlist_stack_level == 0 {
                run = false;
            }
        }

        if t == l {
            return Some(l);
        }
    }
}

fn find_protchar_right(
    mem: &MemState,
    linebreak: &mut LinebreakState,
    mut l: MemPtr,
    mut r: MemPtr,
) -> Option<MemPtr> {
    if r.is_null() {
        return Some(MemPtr::NULL);
    }

    linebreak.hlist_stack_level = 0;
    let mut run = true;
    loop {
        let t = r;
        if run {
            while let Some(list) = nonempty_hlist(mem, r) {
                if !push_node(linebreak, l) || !push_node(linebreak, r) {
                    return None;
                }
                l = list;
                r = list_tail(mem, l);
            }
        }

        while run && ignorable_protrusion_node(mem, r) {
            while r == l && linebreak.hlist_stack_level > 0 {
                r = pop_node(linebreak)?;
                l = pop_node(linebreak)?;
            }

            if r != l && !r.is_null() {
                r = crate::mem::prev_rightmost(mem, l, r);
            } else if r == l && linebreak.hlist_stack_level == 0 {
                run = false;
            }
        }

        if t == r {
            return Some(r);
        }
    }
}

/// The protrusion (`\lpcode`/`\rpcode` scaled to the font quad) of `p`, recording
/// the protruding char in `last_leftmost_char`/`last_rightmost_char`.
fn char_pw(
    errors: &mut ErrorState,
    mem: &MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    p: MemPtr,
    side: i16,
) -> i32 {
    fn protrusion(errors: &mut ErrorState, fonts: &FontState, f: i32, cp: i32) -> i32 {
        let quad = fonts.font_info_b32s1(QUAD_CODE + fonts.param_base(f));
        crate::scaled_math::round_xn_over_d(errors, quad, cp, 1000)
    }

    if side == 0 {
        linebreak.last_leftmost_char = MemPtr::NULL;
    } else {
        linebreak.last_rightmost_char = MemPtr::NULL;
    }
    if p.is_null() {
        return 0;
    }

    let (char_ptr, f, c) = if is_char_node(mem, p) {
        (p, mem.b16s1(p) as i32, mem.b16s0(p))
    } else {
        match mem.classify(MemIdx::new(p)) {
            Ok(AnyNode::Whatsit(w)) => match w.whatsit_subtype(mem) {
                Some(s) if s.is_native_word() => {
                    if w.native_glyph_info_handle(mem) == 0 {
                        return 0;
                    }
                    let f = w.native_font(mem).0;
                    // SAFETY: `p` is a native word with glyph info.
                    let cp =
                        unsafe { crate::ext::real_get_native_word_cp(mem, w, side as libc::c_int) };
                    return protrusion(errors, fonts, f, cp);
                }
                Some(WhatsitSubtype::Glyph) => {
                    let f = w.native_font(mem).0;
                    // SAFETY: plain FFI lookup by font and glyph code.
                    let cp = unsafe { get_cp_code(f, w.native_glyph(mem) as u32, side as i32) };
                    return protrusion(errors, fonts, f, cp);
                }
                _ => return 0,
            },
            Ok(AnyNode::Ligature(lig)) => (lig.ptr() + 1, lig.font(mem).0, lig.character(mem)),
            _ => return 0,
        }
    };

    // SAFETY: plain FFI lookup by font and character code.
    let cp = unsafe { get_cp_code(f, c as u32, side as i32) };
    if side == 0 {
        linebreak.last_leftmost_char = char_ptr;
    } else {
        linebreak.last_rightmost_char = char_ptr;
    }
    if cp == 0 {
        return 0;
    }
    protrusion(errors, fonts, f, cp)
}

pub(crate) enum MarginKernAbort {
    Overflow,
    PdfError,
}

/// Insert the right margin kern of the line ending at `q`, which is the last
/// node of a discretionary's pre-break list when `pre_break_tail` is set.
fn insert_right_margin_kern(
    errors: &mut ErrorState,
    mem: &mut MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    q: MemPtr,
    pre_break_tail: bool,
) -> Result<MemPtr, MarginKernAbort> {
    let (p, ptmp) = if pre_break_tail {
        (q, q)
    } else {
        let head = mem.link_ptr(TEMP_HEAD);
        let ptmp = crate::mem::prev_rightmost(mem, head, q);
        let p = find_protchar_right(mem, linebreak, head, ptmp).ok_or(MarginKernAbort::PdfError)?;
        (p, ptmp)
    };

    let w = char_pw(errors, mem, fonts, linebreak, p, 1);
    if w == 0 {
        return Ok(q);
    }

    let k = crate::mem::alloc_margin_kern(mem, -w, linebreak.last_rightmost_char.0, 1)
        .ok_or(MarginKernAbort::Overflow)?;
    k.set_link(mem, ptmp.link(mem));
    ptmp.set_link(mem, k.ptr());
    Ok(if ptmp == q { q.link(mem) } else { q })
}

fn insert_left_margin_kern(
    errors: &mut ErrorState,
    mem: &mut MemState,
    fonts: &FontState,
    linebreak: &mut LinebreakState,
    q: MemPtr,
) -> Result<MemPtr, MarginKernAbort> {
    let p = find_protchar_left(mem, linebreak, q, false).ok_or(MarginKernAbort::PdfError)?;

    let w = char_pw(errors, mem, fonts, linebreak, p, 0);
    if w == 0 {
        return Ok(q);
    }

    let k = crate::mem::alloc_margin_kern(mem, -w, linebreak.last_leftmost_char.0, 0)
        .ok_or(MarginKernAbort::Overflow)?;
    k.set_link(mem, q);
    Ok(k.ptr())
}

/// A copy of `spec` with finite shrink, releasing `spec`.
pub(crate) fn finite_shrink(
    mem: &mut MemState,
    spec: NodePtr<GlueSpecNode>,
) -> Result<NodePtr<GlueSpecNode>, NodeRecycleAbort> {
    let q = crate::mem::alloc_spec(mem, spec.ptr()).ok_or(NodeRecycleAbort::Overflow)?;
    q.set_shrink_order(mem, NORMAL);
    crate::mem::delete_glue_ref(mem, spec.ptr());
    Ok(q)
}

/// A lig item on `mainloop.lig_stack`, which `reconstitute` fills only with lig items.
#[inline]
fn lig_item(p: MemPtr) -> NodePtr<LigItem> {
    // SAFETY: `reconstitute` pushes only lig items, which carry no NODE_type.
    unsafe { NodePtr::<LigItem>::from_idx_unchecked(MemIdx::new(p)) }
}

/// A fresh char node of font `f` holding `c`, linked after `t`.
fn append_char(mem: &mut MemState, t: MemPtr, f: i32, c: u16) -> Result<MemPtr, NodeRecycleAbort> {
    let q = crate::mem::get_avail(mem).ok_or(NodeRecycleAbort::OverflowRunaway)?;
    t.set_link(mem, q);
    mem.set_b16s1(q, f as u16);
    mem.set_b16s0(q, c);
    Ok(q)
}

/// Load `cur_r` for position `j` and return the hyphen char still to be tried there.
fn update_right(st: &mut EngineState, j: i16, n: i16, bchar: i32, hchar: i32) -> i32 {
    st.mainloop.cur_r = if j < n {
        st.hyph.hu[(j + 1) as usize]
    } else {
        bchar
    };
    if odd(st.hyph.hyf[j as usize]) {
        hchar
    } else {
        TOO_BIG_CHAR
    }
}

/// Pop the top lig item, appending its char node after `t`, and load the next `cur_r`.
#[allow(clippy::too_many_arguments)]
fn pop_lig_stack(
    st: &mut EngineState,
    t: &mut MemPtr,
    j: &mut i16,
    n: i16,
    bchar: i32,
    hchar: i32,
    cur_rh: &mut i32,
) {
    let item = lig_item(st.mainloop.lig_stack);
    let component = item.component(&st.mem);
    if component.0 > TEX_NULL {
        t.set_link(&mut st.mem, component);
        *t = component;
        *j += 1;
    }
    st.mainloop.lig_stack = item.ptr().link(&st.mem);
    crate::mem::free_node(&mut st.mem, item.ptr(), SMALL_NODE_SIZE);
    if st.mainloop.lig_stack.is_null() {
        *cur_rh = update_right(st, *j, n, bchar, hchar);
    } else {
        st.mainloop.cur_r = lig_item(st.mainloop.lig_stack).character(&st.mem) as i32;
    }
}

/// Rebuild `hu[j..]` after `HOLD_HEAD` as chars, ligatures and kerns, returning
/// the index of the last character consumed.
fn reconstitute(
    st: &mut EngineState,
    mut j: i16,
    n: i16,
    mut bchar: i32,
    mut hchar: i32,
) -> Result<i16, NodeRecycleAbort> {
    st.hyph.hyphen_passed = 0;
    let mut t = HOLD_HEAD;
    let mut w = 0;
    HOLD_HEAD.set_link(&mut st.mem, MemPtr::NULL);
    st.mainloop.cur_l = st.hyph.hu[j as usize];
    st.mainloop.cur_q = t;

    if j == 0 {
        st.mainloop.ligature_present = st.hyph.init_lig;
        let mut p = st.hyph.init_list;
        if st.mainloop.ligature_present {
            st.mainloop.lft_hit = st.hyph.init_lft;
        }
        while p.0 > TEX_NULL {
            let c = st.mem.b16s0(p);
            t = append_char(&mut st.mem, t, st.hyph.hf, c)?;
            p = p.link(&st.mem);
        }
    } else if st.mainloop.cur_l < TOO_BIG_CHAR {
        t = append_char(&mut st.mem, t, st.hyph.hf, st.mainloop.cur_l as u16)?;
    }

    st.mainloop.lig_stack = MemPtr::NULL;
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
                let effective = crate::fonts::effective_char(
                    &mut st.fonts,
                    true,
                    st.hyph.hf,
                    st.mainloop.cur_l as u16,
                );
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
                        if j == n && st.mainloop.lig_stack.is_null() {
                            st.mainloop.rt_hit = true;
                        }

                        match q.s1 {
                            1 | 5 => {
                                st.mainloop.cur_l = q.s0 as i32;
                                st.mainloop.ligature_present = true;
                            }
                            2 | 6 => {
                                st.mainloop.cur_r = q.s0 as i32;
                                if st.mainloop.lig_stack.0 > TEX_NULL {
                                    lig_item(st.mainloop.lig_stack)
                                        .set_character(&mut st.mem, st.mainloop.cur_r as u16);
                                } else {
                                    let item = crate::mem::alloc_lig_item(
                                        &mut st.mem,
                                        st.mainloop.cur_r as u16,
                                    )
                                    .ok_or(NodeRecycleAbort::Overflow)?;
                                    st.mainloop.lig_stack = item.ptr();
                                    if j == n {
                                        bchar = TOO_BIG_CHAR;
                                    } else {
                                        let p = crate::mem::get_avail(&mut st.mem)
                                            .ok_or(NodeRecycleAbort::OverflowRunaway)?;
                                        item.set_component(&mut st.mem, p);
                                        st.mem.set_b16s0(p, st.hyph.hu[(j + 1) as usize] as u16);
                                        st.mem.set_b16s1(p, st.hyph.hf as u16);
                                    }
                                }
                            }
                            3 => {
                                st.mainloop.cur_r = q.s0 as i32;
                                let below = st.mainloop.lig_stack;
                                let item = crate::mem::alloc_lig_item(
                                    &mut st.mem,
                                    st.mainloop.cur_r as u16,
                                )
                                .ok_or(NodeRecycleAbort::Overflow)?;
                                st.mainloop.lig_stack = item.ptr();
                                item.ptr().set_link(&mut st.mem, below);
                            }
                            7 | 11 => {
                                if st.mainloop.ligature_present {
                                    t = wrap_lig(st, false)?;
                                }
                                st.mainloop.cur_q = t;
                                st.mainloop.cur_l = q.s0 as i32;
                                st.mainloop.ligature_present = true;
                            }
                            _ => {
                                st.mainloop.cur_l = q.s0 as i32;
                                st.mainloop.ligature_present = true;
                                if st.mainloop.lig_stack.0 > TEX_NULL {
                                    pop_lig_stack(st, &mut t, &mut j, n, bchar, hchar, &mut cur_rh);
                                } else if j == n {
                                    break 'continue_;
                                } else {
                                    t = append_char(
                                        &mut st.mem,
                                        t,
                                        st.hyph.hf,
                                        st.mainloop.cur_r as u16,
                                    )?;
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
            t = wrap_lig(st, true)?;
        }

        if w != 0 {
            let kern = crate::mem::alloc_kern(&mut st.mem, w).ok_or(NodeRecycleAbort::Overflow)?;
            t.set_link(&mut st.mem, kern.ptr());
            t = kern.ptr();
            w = 0;
            kern.set_sync_tag(&mut st.mem, 0);
        }

        if st.mainloop.lig_stack.0 > TEX_NULL {
            st.mainloop.cur_q = t;
            st.mainloop.cur_l = lig_item(st.mainloop.lig_stack).character(&st.mem) as i32;
            st.mainloop.ligature_present = true;
            pop_lig_stack(st, &mut t, &mut j, n, bchar, hchar, &mut cur_rh);
            continue;
        }

        return Ok(j);
    }
}

/// Replace the chars after `cur_q` by a ligature of `cur_l`, returning it.
/// `rt` also records a right boundary hit when the lig stack is empty.
fn wrap_lig(st: &mut EngineState, rt: bool) -> Result<MemPtr, NodeRecycleAbort> {
    let components = st.mainloop.cur_q.link(&st.mem);
    let lig = crate::mem::alloc_ligature(
        &mut st.mem,
        st.hyph.hf,
        st.mainloop.cur_l as u16,
        components,
    )
    .ok_or(NodeRecycleAbort::Overflow)?;
    if st.mainloop.lft_hit {
        lig.set_subtype(&mut st.mem, 2);
        st.mainloop.lft_hit = false;
    }
    if rt && st.mainloop.rt_hit && st.mainloop.lig_stack.is_null() {
        let s = lig.subtype(&st.mem);
        lig.set_subtype(&mut st.mem, s + 1);
        st.mainloop.rt_hit = false;
    }
    st.mainloop.cur_q.set_link(&mut st.mem, lig.ptr());
    st.mainloop.ligature_present = false;
    Ok(lig.ptr())
}

fn start_linebreak_pass(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    linebreak: &mut LinebreakState,
    errors: &mut ErrorState,
    prev_graf: i32,
) -> Result<(), NodeRecycleAbort> {
    let q = alloc_active_node(mem, linebreak.active_node_size as i32, LAST_ACTIVE)?;
    q.set_break_type(mem, UNHYPHENATED as u16);
    q.set_fitness(mem, DECENT_FIT as u16);
    q.set_break_node(mem, MemPtr::NULL);
    q.set_line_number(mem, prev_graf + 1);
    q.set_total_demerits(mem, 0);
    ACTIVE_LIST.set_link(mem, q.ptr());
    if linebreak.do_last_line_fit {
        q.set_shortfall(mem, Scaled(0));
        q.set_glue(mem, Scaled(0));
    }

    scanner.active_width[1..].copy_from_slice(&linebreak.background[1..]);
    linebreak.passive = MemPtr::NULL;
    errors.font_in_short_display = 0;
    linebreak.cur_p = mem.link_ptr(TEMP_HEAD);
    linebreak.global_prev_p = linebreak.cur_p;
    linebreak.first_p = linebreak.cur_p;
    Ok(())
}

fn free_break_lists(mem: &mut MemState, linebreak: &LinebreakState) {
    let mut q = ACTIVE_LIST.link(mem);
    while q != LAST_ACTIVE {
        let next = q.link(mem);
        let size = match classify_active_or_delta(mem, q) {
            ActiveOrDelta::Delta(_) => DELTA_NODE_SIZE,
            ActiveOrDelta::Active(_) => linebreak.active_node_size as i32,
        };
        crate::mem::free_node(mem, q, size);
        q = next;
    }

    q = linebreak.passive;
    while !q.is_null() {
        let next = passive_node(q).link(mem);
        crate::mem::free_node(mem, q, PASSIVE_NODE_SIZE);
        q = next;
    }
}

/// Pick `best_bet` and `best_line`, returning whether the result satisfies `\looseness`.
fn choose_best_break(mem: &MemState, linebreak: &mut LinebreakState, eqtb: &EqtbState) -> bool {
    if ACTIVE_LIST.link(mem) == LAST_ACTIVE {
        return false;
    }

    let mut r = ACTIVE_LIST.link(mem);
    let mut fewest_demerits = MAX_HALFWORD;
    loop {
        if let ActiveOrDelta::Active(a) = classify_active_or_delta(mem, r) {
            if a.total_demerits(mem) < fewest_demerits {
                fewest_demerits = a.total_demerits(mem);
                linebreak.best_bet = Some(a);
            }
        }
        r = r.link(mem);
        if r == LAST_ACTIVE {
            break;
        }
    }
    linebreak.best_line = linebreak.best_bet.expect(BEST_BET_SET).line_number(mem);

    let looseness = int_par(eqtb, INT_PAR_LOOSENESS);
    if looseness == 0 {
        return true;
    }

    let mut r = ACTIVE_LIST.link(mem);
    let mut actual_looseness = 0;
    loop {
        if let ActiveOrDelta::Active(a) = classify_active_or_delta(mem, r) {
            let line_diff = a.line_number(mem) - linebreak.best_line;
            if (line_diff < actual_looseness && looseness <= line_diff)
                || (line_diff > actual_looseness && looseness >= line_diff)
            {
                linebreak.best_bet = Some(a);
                actual_looseness = line_diff;
                fewest_demerits = a.total_demerits(mem);
            } else if line_diff == actual_looseness && a.total_demerits(mem) < fewest_demerits {
                linebreak.best_bet = Some(a);
                fewest_demerits = a.total_demerits(mem);
            }
        }
        r = r.link(mem);
        if r == LAST_ACTIVE {
            break;
        }
    }

    linebreak.best_line = linebreak.best_bet.expect(BEST_BET_SET).line_number(mem);
    actual_looseness == looseness || linebreak.final_pass
}

/// Stretch the `\parfillskip` of the last line by the `\lastlinefit` adjustment.
fn finish_last_line_fit(
    mem: &mut MemState,
    linebreak: &mut LinebreakState,
    last_line_fill: NodePtr<GlueNode>,
) -> Result<(), NodeRecycleAbort> {
    if !linebreak.do_last_line_fit {
        return Ok(());
    }

    let best = linebreak.best_bet.expect(BEST_BET_SET);
    if best.shortfall(mem).0 == 0 {
        linebreak.do_last_line_fit = false;
        return Ok(());
    }

    let old_spec = last_line_fill.glue_ptr(mem);
    let spec = crate::mem::alloc_spec(mem, old_spec).ok_or(NodeRecycleAbort::Overflow)?;
    crate::mem::delete_glue_ref(mem, old_spec);

    spec.set_width(
        mem,
        Scaled(spec.width(mem).0 + best.shortfall(mem).0 - best.glue(mem).0),
    );
    spec.set_stretch(mem, Scaled(0));
    last_line_fill.set_glue_ptr(mem, spec.ptr());
    Ok(())
}

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
    /// A non-`Continue` outcome of the infinite-shrinkage `error()`.
    Error(crate::errors::ErrorOutcome),
}

impl LineBreakOutcome {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
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

impl From<NodeRecycleAbort> for LineBreakOutcome {
    fn from(e: NodeRecycleAbort) -> Self {
        LineBreakOutcome::Node(e)
    }
}

impl From<TryBreakAbort> for LineBreakOutcome {
    fn from(e: TryBreakAbort) -> Self {
        LineBreakOutcome::TryBreak(e)
    }
}

impl From<HyphenateAbort> for LineBreakOutcome {
    fn from(e: HyphenateAbort) -> Self {
        LineBreakOutcome::Hyphenate(e)
    }
}

impl From<crate::hyphenation::TrieAbort> for LineBreakOutcome {
    fn from(e: crate::hyphenation::TrieAbort) -> Self {
        LineBreakOutcome::Trie(e)
    }
}

/// Make `spec`'s shrink finite, reporting it on the paragraph's first offence.
fn finite_shrink_emit(
    st: &mut EngineState,
    spec: NodePtr<GlueSpecNode>,
) -> Result<NodePtr<GlueSpecNode>, LineBreakOutcome> {
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
    Ok(finite_shrink(&mut st.mem, spec)?)
}

#[inline]
fn line_break_try_break(
    st: &mut EngineState,
    pi: i32,
    break_type: i16,
    disc: Option<NodePtr<DiscNode>>,
) -> Result<(), TryBreakAbort> {
    let s = &mut *st;
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
        disc,
    )
}

/// TeX's `kern_break`: a kern or math node followed by glue is a legal breakpoint.
#[inline]
fn kern_break(
    st: &mut EngineState,
    next: MemPtr,
    auto_breaking: bool,
) -> Result<(), TryBreakAbort> {
    if !is_char_node(&st.mem, next)
        && auto_breaking
        && matches!(st.mem.classify(MemIdx::new(next)), Ok(AnyNode::Glue(_)))
    {
        line_break_try_break(st, 0, UNHYPHENATED, None)?;
    }
    Ok(())
}

/// The width of a node in a discretionary's list. `Err(true)` for a whatsit
/// without dimensions, `Err(false)` for any other node that cannot occur there.
#[inline]
fn disc_node_width(mem: &MemState, fonts: &mut FontState, p: MemPtr) -> Result<i32, bool> {
    if is_char_node(mem, p) {
        return Ok(font_character_width(
            fonts,
            mem.b16s1(p) as i32,
            mem.b16s0(p),
        ));
    }

    match mem.classify(MemIdx::new(p)) {
        Ok(AnyNode::Ligature(lig)) => {
            let f = lig.font(mem).0;
            fonts.set_xtx_ligature_present(true);
            Ok(font_character_width(fonts, f, lig.character(mem)))
        }
        Ok(
            node @ (AnyNode::HList(_) | AnyNode::VList(_) | AnyNode::Rule(_) | AnyNode::Kern(_)),
        ) => Ok(node.hlist_width(mem)),
        Ok(AnyNode::Whatsit(w)) => w
            .whatsit_subtype(mem)
            .is_some_and(|s| s.has_dimensions())
            .then(|| w.width(mem).0)
            .ok_or(true),
        _ => Err(false),
    }
}

#[inline]
fn par_shape_width(mem: &MemState, shape: i32, l: i32) -> i32 {
    mem.b32s1(shape + 2 * l)
}

#[inline]
fn par_shape_indent(mem: &MemState, shape: i32, l: i32) -> i32 {
    mem.b32s1(shape + 2 * l - 1)
}

pub(crate) fn line_break_in(st: &mut EngineState, d: bool) -> LineBreakOutcome {
    match line_break(st, d) {
        Ok(()) => LineBreakOutcome::Done,
        Err(outcome) => outcome,
    }
}

fn line_break(st: &mut EngineState, d: bool) -> Result<(), LineBreakOutcome> {
    st.scanner.pack_begin_line = st.input.cur_list.mode_line;
    let first = st.input.cur_list.head.link(&st.mem);
    TEMP_HEAD.set_link(&mut st.mem, first);

    let tail = st.input.cur_list.tail;
    let tail_glue = if is_char_node(&st.mem, tail) {
        None
    } else if let Ok(AnyNode::Glue(g)) = st.mem.classify(MemIdx::new(tail)) {
        Some(g)
    } else {
        None
    };
    match tail_glue {
        None => {
            let p = crate::mem::alloc_penalty(&mut st.mem, INF_PENALTY)
                .ok_or(LineBreakOutcome::Overflow)?;
            tail.set_link(&mut st.mem, p.ptr());
            st.input.cur_list.tail = p.ptr();
        }
        Some(tail_glue) => {
            // The trailing glue becomes the infinite penalty in place.
            let spec = tail_glue.glue_ptr(&st.mem);
            let leaders = tail_glue.leader_ptr(&st.mem);
            let tail_penalty = tail_glue.into_penalty(&mut st.mem);
            crate::mem::delete_glue_ref(&mut st.mem, spec);
            crate::packaging::flush_node_list(&mut st.mem, leaders)
                .map_err(LineBreakOutcome::Confusion)?;
            tail_penalty.set_penalty(&mut st.mem, INF_PENALTY);
        }
    }

    let last_line_fill =
        crate::mem::alloc_param_glue(&mut st.mem, &st.eqtb, GLUE_PAR_PAR_FILL_SKIP)
            .ok_or(LineBreakOutcome::Overflow)?;
    st.input
        .cur_list
        .tail
        .set_link(&mut st.mem, last_line_fill.ptr());
    st.hyph.init_l_hyf = st.input.cur_list.prev_graf / 0x0040_0000;
    st.hyph.init_r_hyf = (st.input.cur_list.prev_graf / 65_536) % 64;
    let init_cur_lang = (st.input.cur_list.prev_graf % 65_536) as u8;

    crate::mem::pop_nest_in(&mut st.mem, &mut st.input);
    st.errors.no_shrink_error_yet = true;

    for n in [GLUE_PAR_LEFT_SKIP, GLUE_PAR_RIGHT_SKIP] {
        let spec = glue_par(&st.eqtb, n);
        if has_infinite_shrink(&st.mem, spec) {
            let spec = finite_shrink_emit(st, spec)?;
            st.eqtb.set_b32s1(GLUE_BASE + n as i32, spec.ptr().0);
        }
    }

    let left_skip = glue_par(&st.eqtb, GLUE_PAR_LEFT_SKIP);
    let right_skip = glue_par(&st.eqtb, GLUE_PAR_RIGHT_SKIP);
    let mem = &st.mem;
    let background = &mut st.linebreak.background;
    background[1] = left_skip.width(mem).0 + right_skip.width(mem).0;
    background[2..=5].fill(0);
    background[2 + left_skip.stretch_order(mem) as usize] = left_skip.stretch(mem).0;
    background[2 + right_skip.stretch_order(mem) as usize] += right_skip.stretch(mem).0;
    background[6] = left_skip.shrink(mem).0 + right_skip.shrink(mem).0;

    st.linebreak.do_last_line_fit = false;
    st.linebreak.active_node_size = ACTIVE_NODE_SIZE_NORMAL;
    if int_par(&st.eqtb, INT_PAR_LAST_LINE_FIT) > 0 {
        let spec = glue_spec(last_line_fill.glue_ptr(&st.mem));
        let lb = &mut st.linebreak;
        if spec.stretch(&st.mem).0 > 0
            && spec.stretch_order(&st.mem) > NORMAL
            && lb.background[3..=5] == [0, 0, 0]
        {
            lb.do_last_line_fit = true;
            lb.active_node_size = ACTIVE_NODE_SIZE_EXTENDED;
            lb.fill_width = [0; 3];
            lb.fill_width[(spec.stretch_order(&st.mem) - 1) as usize] = spec.stretch(&st.mem).0;
        }
    }

    st.linebreak.minimum_demerits = AWFUL_BAD;
    st.linebreak.minimal_demerits = [AWFUL_BAD; 4];

    let par_shape = st.eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
    let lb = &mut st.linebreak;
    if par_shape == TEX_NULL {
        let hang_indent = dimen_par(&st.eqtb, DIMEN_PAR_HANG_INDENT);
        let hsize = dimen_par(&st.eqtb, DIMEN_PAR_HSIZE);
        if hang_indent == 0 {
            lb.last_special_line = 0;
            lb.second_width = hsize;
            lb.second_indent = 0;
        } else {
            let hang_after = int_par(&st.eqtb, INT_PAR_HANG_AFTER);
            lb.last_special_line = hang_after.abs();
            if hang_after < 0 {
                lb.first_width = hsize - hang_indent.abs();
                lb.first_indent = hang_indent.max(0);
                lb.second_width = hsize;
                lb.second_indent = 0;
            } else {
                lb.first_width = hsize;
                lb.first_indent = 0;
                lb.second_width = hsize - hang_indent.abs();
                lb.second_indent = hang_indent.max(0);
            }
        }
    } else {
        lb.last_special_line = st.mem.b32s0(par_shape) - 1;
        lb.second_width = par_shape_width(&st.mem, par_shape, lb.last_special_line + 1);
        lb.second_indent = par_shape_indent(&st.mem, par_shape, lb.last_special_line + 1);
    }

    lb.easy_line = if int_par(&st.eqtb, INT_PAR_LOOSENESS) == 0 {
        lb.last_special_line
    } else {
        MAX_HALFWORD
    };

    lb.threshold = int_par(&st.eqtb, INT_PAR_PRETOLERANCE);
    let mut second_pass;
    if lb.threshold >= 0 {
        second_pass = false;
        lb.final_pass = false;
    } else {
        lb.threshold = int_par(&st.eqtb, INT_PAR_TOLERANCE);
        second_pass = true;
        lb.final_pass = dimen_par(&st.eqtb, DIMEN_PAR_EMERGENCY_STRETCH) <= 0;
    }

    loop {
        st.linebreak.threshold = st.linebreak.threshold.min(INF_BAD);

        if second_pass {
            if st.hyph.trie_not_ready {
                crate::hyphenation::init_trie_in(&mut st.hyph)?;
            }
            st.hyph.cur_lang = init_cur_lang;
            st.hyph.l_hyf = st.hyph.init_l_hyf;
            st.hyph.r_hyf = st.hyph.init_r_hyf;
            set_hyph_index(&mut st.hyph);
        }

        start_linebreak_pass(
            &mut st.mem,
            &mut st.scanner,
            &mut st.linebreak,
            &mut st.errors,
            st.input.cur_list.prev_graf,
        )?;
        let mut auto_breaking = true;
        // Whether glue after the previous node is a legal breakpoint.
        let mut prev_breakable = false;

        while !st.linebreak.cur_p.is_null() && ACTIVE_LIST.link(&st.mem) != LAST_ACTIVE {
            let mut cur = st.linebreak.cur_p;

            if is_char_node(&st.mem, cur) {
                prev_breakable = true;
                st.linebreak.global_prev_p = cur;
                loop {
                    let f = st.mem.b16s1(cur) as i32;
                    let c = st.mem.b16s0(cur);
                    st.scanner.active_width[1] += font_character_width(&mut st.fonts, f, c);
                    cur = cur.link(&st.mem);
                    if !is_char_node(&st.mem, cur) {
                        break;
                    }
                }
                st.linebreak.cur_p = cur;
            }

            let Ok(node) = st.mem.classify(MemIdx::new(cur)) else {
                return Err(LineBreakOutcome::Confusion(c"paragraph"));
            };
            match node {
                AnyNode::HList(_) | AnyNode::VList(_) | AnyNode::Rule(_) => {
                    st.scanner.active_width[1] += node.hlist_width(&st.mem);
                }
                AnyNode::Whatsit(w) => {
                    let subtype = w.whatsit_subtype(&st.mem);
                    if subtype == Some(WhatsitSubtype::Language) {
                        set_language(st, w);
                    } else if subtype.is_some_and(|s| s.has_dimensions()) {
                        st.scanner.active_width[1] += w.width(&st.mem).0;
                    }
                }
                AnyNode::Glue(cur_glue) => {
                    if auto_breaking && prev_breakable {
                        line_break_try_break(st, 0, UNHYPHENATED, None)?;
                    }

                    let mut spec = glue_spec(cur_glue.glue_ptr(&st.mem));
                    if has_infinite_shrink(&st.mem, spec) {
                        spec = finite_shrink_emit(st, spec)?;
                        cur_glue.set_glue_ptr(&mut st.mem, spec.ptr());
                    }
                    let aw = &mut st.scanner.active_width;
                    aw[1] += spec.width(&st.mem).0;
                    aw[2 + spec.stretch_order(&st.mem) as usize] += spec.stretch(&st.mem).0;
                    aw[6] += spec.shrink(&st.mem).0;

                    if second_pass && auto_breaking {
                        try_hyphenate(st, cur)?;
                    }
                }
                AnyNode::Kern(k) => {
                    if k.subtype(&st.mem) == EXPLICIT {
                        kern_break(st, k.link(&st.mem), auto_breaking)?;
                    }
                    st.scanner.active_width[1] += k.width(&st.mem).0;
                }
                AnyNode::Ligature(lig) => {
                    let f = lig.font(&st.mem).0;
                    st.fonts.set_xtx_ligature_present(true);
                    let c = lig.character(&st.mem);
                    st.scanner.active_width[1] += font_character_width(&mut st.fonts, f, c);
                }
                AnyNode::Disc(d) => {
                    let pre_break = d.pre_break(&st.mem);
                    st.linebreak.disc_width = 0;
                    if pre_break.is_null() {
                        let pi = int_par(&st.eqtb, INT_PAR_EX_HYPHEN_PENALTY);
                        line_break_try_break(st, pi, HYPHENATED, Some(d))?;
                    } else {
                        let mut s = pre_break;
                        while !s.is_null() {
                            st.linebreak.disc_width +=
                                disc_node_width(&st.mem, &mut st.fonts, s)
                                    .map_err(|_| LineBreakOutcome::Confusion(c"disc3"))?;
                            s = s.link(&st.mem);
                        }
                        st.scanner.active_width[1] += st.linebreak.disc_width;
                        let pi = int_par(&st.eqtb, INT_PAR_HYPHEN_PENALTY);
                        line_break_try_break(st, pi, HYPHENATED, Some(d))?;
                        st.scanner.active_width[1] -= st.linebreak.disc_width;
                    }

                    let mut s = d.link(&st.mem);
                    for _ in 0..d.subtype(&st.mem) {
                        st.scanner.active_width[1] += disc_node_width(&st.mem, &mut st.fonts, s)
                            .map_err(|_| LineBreakOutcome::Confusion(c"disc4"))?;
                        s = s.link(&st.mem);
                    }

                    prev_breakable = true;
                    st.linebreak.global_prev_p = cur;
                    st.linebreak.cur_p = s;
                    continue;
                }
                AnyNode::Math(m) => {
                    if m.subtype(&st.mem) < L_CODE {
                        auto_breaking = odd(m.subtype(&st.mem) as u8);
                    }
                    kern_break(st, m.link(&st.mem), auto_breaking)?;
                    st.scanner.active_width[1] += m.width(&st.mem).0;
                }
                AnyNode::Penalty(pen) => {
                    line_break_try_break(st, pen.penalty(&st.mem), UNHYPHENATED, None)?;
                }
                AnyNode::Mark(_) | AnyNode::Ins(_) | AnyNode::Adjust(_) => {}
                _ => return Err(LineBreakOutcome::Confusion(c"paragraph")),
            }

            prev_breakable = node.precedes_break()
                || matches!(node, AnyNode::Kern(k) if k.subtype(&st.mem) != EXPLICIT);
            st.linebreak.global_prev_p = cur;
            st.linebreak.cur_p = cur.link(&st.mem);
        }

        if st.linebreak.cur_p.is_null() {
            line_break_try_break(st, EJECT_PENALTY, HYPHENATED, None)?;
            if choose_best_break(&st.mem, &mut st.linebreak, &st.eqtb) {
                break;
            }
        }

        free_break_lists(&mut st.mem, &st.linebreak);

        if !second_pass {
            st.linebreak.threshold = int_par(&st.eqtb, INT_PAR_TOLERANCE);
            second_pass = true;
            st.linebreak.final_pass = dimen_par(&st.eqtb, DIMEN_PAR_EMERGENCY_STRETCH) <= 0;
        } else {
            st.linebreak.background[2] += dimen_par(&st.eqtb, DIMEN_PAR_EMERGENCY_STRETCH);
            st.linebreak.final_pass = true;
        }
    }

    finish_last_line_fit(&mut st.mem, &mut st.linebreak, last_line_fill)?;

    match post_line_break_in(st, d) {
        PostLineBreakOutcome::Done => {}
        other => return Err(LineBreakOutcome::PostLineBreak(other)),
    }

    free_break_lists(&mut st.mem, &st.linebreak);

    st.scanner.pack_begin_line = 0;
    Ok(())
}

/// Switch hyphenation to the language of a `\language` whatsit.
fn set_language(st: &mut EngineState, w: NodePtr<WhatsitNode>) {
    st.hyph.cur_lang = w.what_lang(&st.mem) as u8;
    st.hyph.l_hyf = w.what_lhm(&st.mem) as i32;
    st.hyph.r_hyf = w.what_rhm(&st.mem) as i32;
    set_hyph_index(&mut st.hyph);
}

#[inline]
fn lc_code(eqtb: &EqtbState, c: i32) -> i32 {
    eqtb.b32s1(LC_CODE_BASE + c)
}

/// Scan from `s` to the end of the word, returning the node that ends it, or
/// `None` if a node there forbids hyphenation. `math_ok` lets a math node end it.
fn find_word_end(mem: &MemState, mut s: MemPtr, math_ok: bool) -> Option<MemPtr> {
    loop {
        if !is_char_node(mem, s) {
            match mem.classify(MemIdx::new(s)) {
                Ok(AnyNode::Ligature(_)) => {}
                Ok(AnyNode::Kern(k)) => {
                    if k.subtype(mem) != NORMAL {
                        return Some(s);
                    }
                }
                Ok(
                    AnyNode::Whatsit(_)
                    | AnyNode::Glue(_)
                    | AnyNode::Penalty(_)
                    | AnyNode::Ins(_)
                    | AnyNode::Adjust(_)
                    | AnyNode::Mark(_),
                ) => return Some(s),
                Ok(AnyNode::Math(m)) if math_ok => {
                    return (m.subtype(mem) >= L_CODE).then_some(s);
                }
                _ => return None,
            }
        }
        s = s.link(mem);
    }
}

/// Try to hyphenate the word following the glue node `cur`.
fn try_hyphenate(st: &mut EngineState, cur: MemPtr) -> Result<(), LineBreakOutcome> {
    let max_len = crate::scan::max_hyphenatable_length_in(&st.eqtb);
    let uc_hyph = int_par(&st.eqtb, INT_PAR_UC_HYPH) > 0;
    let mut prev_s = cur;
    let mut s = prev_s.link(&st.mem);
    if s.is_null() {
        return Ok(());
    }

    // Skip to the first letter, setting `hf`.
    loop {
        let c;
        if is_char_node(&st.mem, s) {
            c = st.mem.b16s0(s) as i32;
            st.hyph.hf = st.mem.b16s1(s) as i32;
        } else {
            match st.mem.classify(MemIdx::new(s)) {
                Ok(AnyNode::Ligature(lig)) if !lig.lig_ptr(&st.mem).is_null() => {
                    let q = lig.lig_ptr(&st.mem);
                    c = st.mem.b16s0(q) as i32;
                    st.hyph.hf = st.mem.b16s1(q) as i32;
                }
                Ok(AnyNode::Ligature(_)) => {
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                Ok(AnyNode::Kern(k)) if k.subtype(&st.mem) == NORMAL => {
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                Ok(AnyNode::Math(m)) if m.subtype(&st.mem) >= L_CODE => {
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                Ok(AnyNode::Whatsit(w)) => {
                    let subtype = w.whatsit_subtype(&st.mem);
                    if subtype.is_some_and(|sub| sub.is_native_word()) {
                        let len = w.native_length(&st.mem) as i32;
                        let mut l = 0;
                        while l < len {
                            let c = get_native_usv(&st.mem, w, l);
                            if lc_code(&st.eqtb, c) != 0 {
                                st.hyph.hf = w.native_font(&st.mem).0;
                                prev_s = s;
                                if lc_code(&st.eqtb, c) == c || uc_hyph {
                                    break;
                                }
                                return Ok(());
                            }
                            if c >= 65_536 {
                                l += 1;
                            }
                            l += 1;
                        }
                        if l < len {
                            break;
                        }
                    }

                    if subtype == Some(WhatsitSubtype::Language) {
                        set_language(st, w);
                    }
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                _ => return Ok(()),
            }
        }

        st.hyph.hc[0] = hyph_code(&st.hyph, &st.eqtb, c);
        if st.hyph.hc[0] != 0 {
            if st.hyph.hc[0] == c || uc_hyph {
                break;
            }
            return Ok(());
        }

        prev_s = s;
        s = prev_s.link(&st.mem);
    }

    st.hyph.hyf_char = st.fonts.hyphen_char[st.hyph.hf as usize];
    if st.hyph.hyf_char < 0 || st.hyph.hyf_char > TOO_BIG_CHAR - 1 {
        return Ok(());
    }
    st.linebreak.ha = prev_s;
    if st.hyph.l_hyf + st.hyph.r_hyf > max_len {
        return Ok(());
    }

    let native_ha = match as_native_word(&st.mem, st.linebreak.ha) {
        Some(ha) => {
            let Some(end) = find_word_end(&st.mem, ha.link(&st.mem), false) else {
                return Ok(());
            };
            s = end;
            Some(collect_native_word(st, ha, max_len)?)
        }
        None => {
            s = collect_word(st, s, max_len);
            None
        }
    };

    if (st.linebreak.hn as i32) < st.hyph.l_hyf + st.hyph.r_hyf
        || find_word_end(&st.mem, s, true).is_none()
    {
        return Ok(());
    }

    Ok(hyphenate(st, native_ha)?)
}

/// Collect the letters of the native word `ha` into `hu`/`hc`, splitting it
/// where the hyphenatable part begins or ends. Returns the word's final node.
fn collect_native_word(
    st: &mut EngineState,
    mut ha: NodePtr<WhatsitNode>,
    max_len: i32,
) -> Result<NodePtr<WhatsitNode>, NodeRecycleAbort> {
    st.linebreak.hn = 0;
    'restart: loop {
        let len = ha.native_length(&st.mem) as i32;
        let mut l = 0;
        while l < len {
            let c = get_native_usv(&st.mem, ha, l);
            let hc0 = hyph_code(&st.hyph, &st.eqtb, c);
            st.hyph.hc[0] = hc0;

            if hc0 == 0 {
                if st.linebreak.hn > 0 {
                    split_native_word(st, ha, l)?;
                    return Ok(ha);
                }
            } else if st.linebreak.hn == 0 && l > 0 {
                ha = split_native_word(st, ha, l)?;
                st.linebreak.ha = ha.ptr();
                continue 'restart;
            } else if st.linebreak.hn as i32 == max_len {
                return Ok(ha);
            } else {
                st.linebreak.hn += 1;
                let hn = st.linebreak.hn as usize;
                if c < 65_536 {
                    st.hyph.hu[hn] = c;
                    st.hyph.hc[hn] = hc0;
                } else {
                    st.hyph.hu[hn] = (c - 65_536) / 1024 + 0xD800;
                    st.hyph.hc[hn] = (hc0 - 65_536) / 1024 + 0xD800;
                    st.linebreak.hn += 1;
                    st.hyph.hu[hn + 1] = c % 1024 + 0xDC00;
                    st.hyph.hc[hn + 1] = hc0 % 1024 + 0xDC00;
                    l += 1;
                }
                st.hyph.hyf_bchar = TOO_BIG_CHAR;
            }
            l += 1;
        }
        return Ok(ha);
    }
}

/// Collect the letters of the word starting at `s` into `hu`/`hc`, setting
/// `hb` and `hyf_bchar`. Returns the node after the collected letters.
fn collect_word(st: &mut EngineState, mut s: MemPtr, max_len: i32) -> MemPtr {
    st.linebreak.hn = 0;
    loop {
        if is_char_node(&st.mem, s) {
            if st.mem.b16s1(s) as i32 != st.hyph.hf {
                return s;
            }
            let c = st.mem.b16s0(s) as i32;
            st.hyph.hyf_bchar = c;
            let hc0 = hyph_code(&st.hyph, &st.eqtb, c);
            st.hyph.hc[0] = hc0;
            if hc0 == 0 || hc0 > st.hyph.max_hyph_char || st.linebreak.hn as i32 == max_len {
                return s;
            }
            st.linebreak.hb = s;
            st.linebreak.hn += 1;
            st.hyph.hu[st.linebreak.hn as usize] = c;
            st.hyph.hc[st.linebreak.hn as usize] = hc0;
            st.hyph.hyf_bchar = TOO_BIG_CHAR;
        } else {
            match st.mem.classify(MemIdx::new(s)) {
                Ok(AnyNode::Ligature(lig)) => {
                    if lig.font(&st.mem).0 != st.hyph.hf {
                        return s;
                    }
                    let mut j = st.linebreak.hn;
                    let mut q = lig.lig_ptr(&st.mem);
                    if q.0 > TEX_NULL {
                        st.hyph.hyf_bchar = st.mem.b16s0(q) as i32;
                    }
                    while q.0 > TEX_NULL {
                        let c = st.mem.b16s0(q) as i32;
                        let hc0 = hyph_code(&st.hyph, &st.eqtb, c);
                        st.hyph.hc[0] = hc0;
                        if hc0 == 0 || hc0 > st.hyph.max_hyph_char || j as i32 == max_len {
                            return s;
                        }
                        j += 1;
                        st.hyph.hu[j as usize] = c;
                        st.hyph.hc[j as usize] = hc0;
                        q = q.link(&st.mem);
                    }
                    st.linebreak.hb = s;
                    st.linebreak.hn = j;
                    st.hyph.hyf_bchar = if odd(lig.subtype(&st.mem) as u8) {
                        st.fonts.font_bchar[st.hyph.hf as usize]
                    } else {
                        TOO_BIG_CHAR
                    };
                }
                Ok(AnyNode::Kern(k)) if k.subtype(&st.mem) == NORMAL => {
                    st.linebreak.hb = s;
                    st.hyph.hyf_bchar = st.fonts.font_bchar[st.hyph.hf as usize];
                }
                _ => return s,
            }
        }
        s = s.link(&st.mem);
    }
}

fn reverse_best_breaks(mem: &mut MemState, linebreak: &mut LinebreakState) {
    let mut q = linebreak.best_bet.expect(BEST_BET_SET).break_node(mem);
    linebreak.cur_p = MemPtr::NULL;
    loop {
        let r = passive_node(q);
        q = r.next_break(mem);
        r.set_next_break(mem, linebreak.cur_p);
        linebreak.cur_p = r.ptr();
        if q.is_null() {
            break;
        }
    }
}

/// Track the LR nesting of a math node on the `lr_ptr` stack.
fn update_lr_stack(
    mem: &mut MemState,
    q: NodePtr<MathNode>,
    lr_ptr: &mut MemPtr,
) -> Result<(), NodeRecycleAbort> {
    let subtype = q.subtype(mem);
    let end_lr = crate::packaging::end_lr_type(subtype);
    if subtype % 2 == 1 {
        if !lr_ptr.is_null() && lr_ptr.info(mem) == end_lr {
            crate::packaging::pop_lr(mem, lr_ptr);
        }
        Ok(())
    } else {
        crate::packaging::push_lr(mem, lr_ptr, end_lr)
    }
}

struct PostLineHpack {
    q: MemPtr,
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

impl From<NodeRecycleAbort> for PostLineAbort {
    fn from(e: NodeRecycleAbort) -> Self {
        PostLineAbort::Node(e)
    }
}

impl From<MarginKernAbort> for PostLineAbort {
    fn from(e: MarginKernAbort) -> Self {
        match e {
            MarginKernAbort::PdfError => PostLineAbort::HlistStackFailure,
            MarginKernAbort::Overflow => PostLineAbort::Node(NodeRecycleAbort::Overflow),
        }
    }
}

/// Break off the next line of the paragraph after `TEMP_HEAD`, ready for `hpack`.
#[allow(clippy::too_many_arguments)]
fn post_line_break_prefix(
    errors: &mut ErrorState,
    mem: &mut MemState,
    scanner: &mut ScannerState,
    linebreak: &mut LinebreakState,
    eqtb: &EqtbState,
    fonts: &FontState,
    config: &ConfigState,
    lr_ptr: &mut MemPtr,
    cur_line: i32,
) -> Result<PostLineHpack, PostLineAbort> {
    let texxet = int_par(eqtb, INT_PAR_TEXXET) > 0;
    let protrude = int_par(eqtb, INT_PAR_XETEX_PROTRUDE_CHARS) > 0;
    let cur_break = passive_node(linebreak.cur_p).cur_break(mem);

    if texxet {
        let mut q = TEMP_HEAD.link(mem);
        if !lr_ptr.is_null() {
            let mut temp_ptr = *lr_ptr;
            let mut r = q;
            loop {
                let s = crate::mem::alloc_math(mem, 0, (temp_ptr.info(mem) - 1) as i16)
                    .ok_or(NodeRecycleAbort::Overflow)?;
                s.set_link(mem, r);
                r = s.ptr();
                temp_ptr = temp_ptr.link(mem);
                if temp_ptr.is_null() {
                    break;
                }
            }
            TEMP_HEAD.set_link(mem, r);
        }

        while q != cur_break {
            if !is_char_node(mem, q) {
                if let Ok(AnyNode::Math(m)) = mem.classify(MemIdx::new(q)) {
                    update_lr_stack(mem, m, lr_ptr)?;
                }
            }
            q = q.link(mem);
        }
    }

    let mut q = cur_break;
    let mut disc_break = false;
    let mut post_disc_break = false;
    let mut glue_break = false;
    let mut pre_break_tail = false;

    if q.is_null() {
        q = list_tail(mem, TEMP_HEAD);
    } else {
        match mem.classify(MemIdx::new(q)) {
            Ok(AnyNode::Glue(g)) => {
                let old_spec = g.glue_ptr(mem);
                crate::mem::delete_glue_ref(mem, old_spec);
                let right_skip = glue_par(eqtb, GLUE_PAR_RIGHT_SKIP);
                g.set_glue_ptr(mem, right_skip.ptr());
                g.set_subtype(mem, GLUE_PAR_RIGHT_SKIP as u16 + 1);
                right_skip.set_ref_count(mem, right_skip.ref_count(mem) + 1);
                glue_break = true;
            }
            Ok(AnyNode::Disc(d)) => {
                let mut r;
                let t = d.subtype(mem);
                if t == 0 {
                    r = d.link(mem);
                } else {
                    r = q;
                    for _ in 1..t {
                        r = r.link(mem);
                    }
                    let s = r.link(mem);
                    r = s.link(mem);
                    s.set_link(mem, MemPtr::NULL);
                    crate::packaging::flush_node_list(mem, d.link(mem))
                        .map_err(NodeRecycleAbort::Confusion)?;
                    d.set_subtype(mem, 0);
                }

                let post_break = d.post_break(mem);
                if !post_break.is_null() {
                    list_tail(mem, post_break).set_link(mem, r);
                    r = post_break;
                    d.set_post_break(mem, MemPtr::NULL);
                    post_disc_break = true;
                }

                let pre_break = d.pre_break(mem);
                if !pre_break.is_null() {
                    d.set_link(mem, pre_break);
                    q = list_tail(mem, pre_break);
                    d.set_pre_break(mem, MemPtr::NULL);
                    pre_break_tail = true;
                }

                q.set_link(mem, r);
                disc_break = true;
            }
            Ok(AnyNode::Kern(k)) => k.set_width(mem, Scaled(0)),
            Ok(AnyNode::Math(m)) => {
                m.set_width(mem, Scaled(0));
                if texxet {
                    update_lr_stack(mem, m, lr_ptr)?;
                }
            }
            _ => {}
        }
    }

    if protrude {
        q = insert_right_margin_kern(errors, mem, fonts, linebreak, q, pre_break_tail)?;
    }

    if !glue_break {
        let r = crate::mem::alloc_param_glue(mem, eqtb, GLUE_PAR_RIGHT_SKIP)
            .ok_or(NodeRecycleAbort::Overflow)?;
        r.set_link(mem, q.link(mem));
        q.set_link(mem, r.ptr());
        q = r.ptr();
    }

    if texxet && !lr_ptr.is_null() {
        let mut s = TEMP_HEAD;
        let mut r = s.link(mem);
        while r != q {
            s = r;
            r = s.link(mem);
        }

        r = *lr_ptr;
        while !r.is_null() {
            let m = crate::mem::alloc_math(mem, 0, r.info(mem) as i16)
                .ok_or(NodeRecycleAbort::Overflow)?;
            s.set_link(mem, m.ptr());
            s = m.ptr();
            r = r.link(mem);
        }
        s.set_link(mem, q);
    }

    let r = q.link(mem);
    q.set_link(mem, MemPtr::NULL);
    q = TEMP_HEAD.link(mem);
    TEMP_HEAD.set_link(mem, r);

    if protrude {
        q = insert_left_margin_kern(errors, mem, fonts, linebreak, q)?;
    }

    if glue_par(eqtb, GLUE_PAR_LEFT_SKIP).ptr() != ZERO_GLUE {
        let r = crate::mem::alloc_param_glue(mem, eqtb, GLUE_PAR_LEFT_SKIP)
            .ok_or(NodeRecycleAbort::Overflow)?;
        r.set_link(mem, q);
        q = r.ptr();
    }

    let par_shape = eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
    let (cur_width, cur_indent) = if cur_line > linebreak.last_special_line {
        (linebreak.second_width, linebreak.second_indent)
    } else if par_shape == TEX_NULL {
        (linebreak.first_width, linebreak.first_indent)
    } else {
        (
            par_shape_width(mem, par_shape, cur_line),
            par_shape_indent(mem, par_shape, cur_line),
        )
    };

    scanner.adjust_tail = MemPtr(ADJUST_HEAD);
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

/// Entry `n` (clamped to the last) of the eTeX penalty array `k`, if it is set.
fn etex_penalty(mem: &MemState, eqtb: &EqtbState, k: i32, n: i32) -> Option<i32> {
    let q = eqtb.b32s1(ETEX_PEN_BASE + k);
    if q == TEX_NULL {
        return None;
    }
    Some(mem.b32s1(q + n.min(mem.b32s1(q + 1)) + 1))
}

/// Append the packed line and the interline penalty, then prune discardable
/// nodes from the start of the next line. Returns whether the paragraph is done.
#[allow(clippy::too_many_arguments)]
fn post_line_break_suffix(
    cl: &mut crate::mem_types::ListStateRecord,
    mem: &mut MemState,
    scanner: &mut ScannerState,
    linebreak: &mut LinebreakState,
    eqtb: &EqtbState,
    just_box: AnyBox,
    hpack: PostLineHpack,
    d: bool,
    lr_ptr: &mut MemPtr,
    cur_line: &mut i32,
) -> Result<bool, NodeRecycleAbort> {
    scanner.just_box = just_box.ptr();
    just_box.set_shift_amount(mem, Scaled(hpack.cur_indent));

    if scanner.pre_adjust_tail != PRE_ADJUST_HEAD {
        cl.tail.set_link(mem, PRE_ADJUST_HEAD.link(mem));
        cl.tail = scanner.pre_adjust_tail;
    }
    scanner.pre_adjust_tail = MemPtr::NULL;

    crate::packaging::append_to_vlist(cl, mem, eqtb, just_box)?;

    if scanner.adjust_tail != MemPtr(ADJUST_HEAD) {
        cl.tail.set_link(mem, MemPtr(ADJUST_HEAD).link(mem));
        cl.tail = scanner.adjust_tail;
    }
    scanner.adjust_tail = MemPtr::NULL;

    if *cur_line + 1 != linebreak.best_line {
        let mut pen = etex_penalty(
            mem,
            eqtb,
            ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES,
            *cur_line,
        )
        .unwrap_or_else(|| int_par(eqtb, INT_PAR_INTER_LINE_PENALTY));

        match etex_penalty(
            mem,
            eqtb,
            ETEX_PENALTIES_PAR_CLUB_PENALTIES,
            *cur_line - cl.prev_graf,
        ) {
            Some(p) => pen += p,
            None if *cur_line == cl.prev_graf + 1 => pen += int_par(eqtb, INT_PAR_CLUB_PENALTY),
            None => {}
        }

        let (widow_array, widow_penalty) = if d {
            (
                ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES,
                INT_PAR_DISPLAY_WIDOW_PENALTY,
            )
        } else {
            (ETEX_PENALTIES_PAR_WIDOW_PENALTIES, INT_PAR_WIDOW_PENALTY)
        };
        match etex_penalty(mem, eqtb, widow_array, linebreak.best_line - *cur_line - 1) {
            Some(p) => pen += p,
            None if *cur_line + 2 == linebreak.best_line => pen += int_par(eqtb, widow_penalty),
            None => {}
        }

        if hpack.disc_break {
            pen += int_par(eqtb, INT_PAR_BROKEN_PENALTY);
        }

        if pen != 0 {
            let r = crate::mem::alloc_penalty(mem, pen).ok_or(NodeRecycleAbort::Overflow)?;
            cl.tail.set_link(mem, r.ptr());
            cl.tail = r.ptr();
        }
    }

    *cur_line += 1;
    linebreak.cur_p = passive_node(linebreak.cur_p).next_break(mem);

    if !linebreak.cur_p.is_null() && !hpack.post_disc_break {
        let texxet = int_par(eqtb, INT_PAR_TEXXET) > 0;
        let cur_break = passive_node(linebreak.cur_p).cur_break(mem);
        let mut r = TEMP_HEAD;
        let mut q;
        loop {
            q = r.link(mem);
            if q == cur_break || is_char_node(mem, q) {
                break;
            }
            let node = mem.classify(MemIdx::new(q));
            match node {
                Ok(node) if node.precedes_break() => break,
                Ok(AnyNode::Kern(k))
                    if k.subtype(mem) != EXPLICIT && k.subtype(mem) != SPACE_ADJUSTMENT =>
                {
                    break
                }
                _ => {}
            }

            r = q;
            if texxet {
                if let Ok(AnyNode::Math(m)) = node {
                    update_lr_stack(mem, m, lr_ptr)?;
                }
            }
        }

        if r != TEMP_HEAD {
            r.set_link(mem, MemPtr::NULL);
            crate::packaging::flush_node_list(mem, TEMP_HEAD.link(mem))
                .map_err(NodeRecycleAbort::Confusion)?;
            TEMP_HEAD.set_link(mem, q);
        }
    }

    Ok(linebreak.cur_p.is_null())
}

/// Cold-abort outcomes deferred out of [`post_line_break_in`].
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

/// Break the paragraph into lines along the chosen breakpoints and append them.
fn post_line_break_in(st: &mut EngineState, d: bool) -> PostLineBreakOutcome {
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

        match post_line_break_suffix(
            &mut st.input.cur_list,
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
            Ok(true) => break,
            Ok(false) => {}
            Err(e) => return PostLineBreakOutcome::Node(e),
        }
    }

    if cur_line != st.linebreak.best_line || !TEMP_HEAD.link(&st.mem).is_null() {
        return PostLineBreakOutcome::Confusion(c"line breaking");
    }

    st.input.cur_list.prev_graf = st.linebreak.best_line - 1;
    st.input.cur_list.e_tex_aux = lr_ptr;
    PostLineBreakOutcome::Done
}

pub(crate) enum TryBreakAbort {
    Node(NodeRecycleAbort),
    HlistStackFailure,
}

impl TryBreakAbort {
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

impl From<NodeRecycleAbort> for TryBreakAbort {
    fn from(e: NodeRecycleAbort) -> Self {
        TryBreakAbort::Node(e)
    }
}

#[inline]
fn stretch_fitness(b: i32) -> usize {
    if b > 99 {
        VERY_LOOSE_FIT
    } else if b > 12 {
        LOOSE_FIT
    } else {
        DECENT_FIT
    }
}

#[inline]
fn shrink_fitness(b: i32) -> usize {
    if b > 12 {
        TIGHT_FIT
    } else {
        DECENT_FIT
    }
}

/// Badness and fitness class of a line that is `shortfall` short with `stretch` available.
#[inline]
fn stretch_badness(shortfall: i32, stretch: i32) -> (i32, usize) {
    if shortfall > 7_230_584 && stretch < 1_663_497 {
        (INF_BAD, VERY_LOOSE_FIT)
    } else {
        let b = crate::helpers::badness(shortfall, stretch);
        (b, stretch_fitness(b))
    }
}

/// Set `break_width` to the width of the line material after a break at
/// `cur_p`, which is `disc` for a hyphenated break.
fn compute_break_width(
    mem: &MemState,
    fonts: &mut FontState,
    linebreak: &mut LinebreakState,
    disc: Option<NodePtr<DiscNode>>,
) -> Result<(), TryBreakAbort> {
    linebreak.break_width[1..].copy_from_slice(&linebreak.background[1..]);
    let mut s = linebreak.cur_p;

    if let Some(cur_disc) = disc {
        let mut v = cur_disc.ptr();
        s = cur_disc.post_break(mem);

        for _ in 0..cur_disc.subtype(mem) {
            v = v.link(mem);
            linebreak.break_width[1] -= disc_node_width(mem, fonts, v).map_err(|whatsit| {
                NodeRecycleAbort::Confusion(if whatsit { c"disc1a" } else { c"disc1" })
            })?;
        }

        while !s.is_null() {
            linebreak.break_width[1] += disc_node_width(mem, fonts, s).map_err(|whatsit| {
                NodeRecycleAbort::Confusion(if whatsit { c"disc2a" } else { c"disc2" })
            })?;
            s = s.link(mem);
        }

        linebreak.break_width[1] += linebreak.disc_width;
        if cur_disc.post_break(mem).is_null() {
            s = v.link(mem);
        }
    }

    while !s.is_null() && !is_char_node(mem, s) {
        match mem.classify(MemIdx::new(s)) {
            Ok(AnyNode::Glue(g)) => {
                let spec = glue_spec(g.glue_ptr(mem));
                linebreak.break_width[1] -= spec.width(mem).0;
                linebreak.break_width[2 + spec.stretch_order(mem) as usize] -= spec.stretch(mem).0;
                linebreak.break_width[6] -= spec.shrink(mem).0;
            }
            Ok(AnyNode::Penalty(_)) => {}
            Ok(AnyNode::Math(m)) => linebreak.break_width[1] -= m.width(mem).0,
            Ok(AnyNode::Kern(k)) if k.subtype(mem) == EXPLICIT => {
                linebreak.break_width[1] -= k.width(mem).0;
            }
            _ => break,
        }
        s = s.link(mem);
    }
    Ok(())
}

/// Record the feasible breaks found so far as new active nodes before `r`, with
/// delta nodes around them (TeX's "insert new active nodes").
#[allow(clippy::too_many_arguments)]
fn insert_new_active_nodes(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    linebreak: &mut LinebreakState,
    eqtb: &EqtbState,
    r: MemPtr,
    prev_r: &mut MemPtr,
    prev_prev_r: &mut MemPtr,
    break_type: i16,
) -> Result<(), TryBreakAbort> {
    let bw = linebreak.break_width;
    let caw = linebreak.cur_active_width;
    if let ActiveOrDelta::Delta(prev_delta) = classify_active_or_delta(mem, *prev_r) {
        let old = delta_widths(prev_delta, mem);
        let w = std::array::from_fn(|k| old[k] - caw[k] + bw[k]);
        set_delta_widths(prev_delta, mem, &w);
    } else if *prev_r == ACTIVE_LIST {
        scanner.active_width[1..].copy_from_slice(&bw[1..]);
    } else {
        let w = std::array::from_fn(|k| bw[k] - caw[k]);
        let q = alloc_delta_node(mem, r, &w)?;
        prev_r.set_link(mem, q.ptr());
        *prev_prev_r = *prev_r;
        *prev_r = q.ptr();
    }

    let adj = int_par(eqtb, INT_PAR_ADJ_DEMERITS).abs();
    if adj >= MAX_HALFWORD - linebreak.minimum_demerits {
        linebreak.minimum_demerits = AWFUL_BAD - 1;
    } else {
        linebreak.minimum_demerits += adj;
    }

    for fit_class in VERY_LOOSE_FIT..=TIGHT_FIT {
        if linebreak.minimal_demerits[fit_class] <= linebreak.minimum_demerits {
            let passive = alloc_passive_node(mem)?;
            passive.set_link(mem, linebreak.passive);
            linebreak.passive = passive.ptr();
            passive.set_cur_break(mem, linebreak.cur_p);
            passive.set_prev_break(mem, linebreak.best_place[fit_class]);

            let q = alloc_active_node(mem, linebreak.active_node_size as i32, r)?;
            q.set_break_node(mem, linebreak.passive);
            q.set_line_number(mem, linebreak.best_pl_line[fit_class] + 1);
            q.set_fitness(mem, fit_class as u16);
            q.set_break_type(mem, break_type as u16);
            q.set_total_demerits(mem, linebreak.minimal_demerits[fit_class]);
            if linebreak.do_last_line_fit {
                q.set_shortfall(mem, Scaled(linebreak.best_pl_short[fit_class]));
                q.set_glue(mem, Scaled(linebreak.best_pl_glue[fit_class]));
            }

            prev_r.set_link(mem, q.ptr());
            *prev_r = q.ptr();
        }
        linebreak.minimal_demerits[fit_class] = MAX_HALFWORD;
    }
    linebreak.minimum_demerits = MAX_HALFWORD;

    if r != LAST_ACTIVE {
        let w = std::array::from_fn(|k| caw[k] - bw[k]);
        let q = alloc_delta_node(mem, r, &w)?;
        prev_r.set_link(mem, q.ptr());
        *prev_prev_r = *prev_r;
        *prev_r = q.ptr();
    }
    Ok(())
}

/// Consider a break at `cur_p` with penalty `pi`, updating the active list.
#[allow(clippy::too_many_arguments)]
fn try_break(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    linebreak: &mut LinebreakState,
    eqtb: &EqtbState,
    fonts: &mut FontState,
    config: &ConfigState,
    errors: &mut ErrorState,
    mut pi: i32,
    break_type: i16,
    disc: Option<NodePtr<DiscNode>>,
) -> Result<(), TryBreakAbort> {
    if config.semantic_pagination_enabled && !linebreak.cur_p.is_null() {
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
    let mut prev_prev_r = MemPtr::NULL;
    let mut old_l = 0;
    let mut line_width = 0;
    let mut g = 0;

    linebreak.cur_active_width[1..].copy_from_slice(&scanner.active_width[1..]);

    loop {
        let r = prev_r.link(mem);

        if let ActiveOrDelta::Delta(r_delta) = classify_active_or_delta(mem, r) {
            let w = delta_widths(r_delta, mem);
            for (width, delta) in linebreak.cur_active_width.iter_mut().zip(w) {
                *width += delta;
            }
            prev_prev_r = prev_r;
            prev_r = r;
            continue;
        }

        let r_active = active(r);
        let l = r_active.line_number(mem);
        if l > old_l {
            if linebreak.minimum_demerits < AWFUL_BAD
                && (old_l != linebreak.easy_line || r == LAST_ACTIVE)
            {
                if no_break_yet {
                    no_break_yet = false;
                    compute_break_width(mem, fonts, linebreak, disc)?;
                }
                insert_new_active_nodes(
                    mem,
                    scanner,
                    linebreak,
                    eqtb,
                    r,
                    &mut prev_r,
                    &mut prev_prev_r,
                    break_type,
                )?;
            }

            if r == LAST_ACTIVE {
                return Ok(());
            }

            if l > linebreak.easy_line {
                line_width = linebreak.second_width;
                old_l = MAX_HALFWORD - 1;
            } else {
                old_l = l;
                let par_shape = eqtb.b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
                line_width = if l > linebreak.last_special_line {
                    linebreak.second_width
                } else if par_shape == TEX_NULL {
                    linebreak.first_width
                } else {
                    par_shape_width(mem, par_shape, l)
                };
            }
        }

        let caw = &linebreak.cur_active_width;
        let mut artificial_demerits;
        let mut shortfall;
        if config.semantic_pagination_enabled {
            line_width = caw[1];
            artificial_demerits = true;
            shortfall = 0;
        } else {
            artificial_demerits = false;
            shortfall = line_width - caw[1];
            if int_par(eqtb, INT_PAR_XETEX_PROTRUDE_CHARS) > 1 {
                shortfall += total_pw(errors, mem, fonts, linebreak, r_active, disc)
                    .ok_or(TryBreakAbort::HlistStackFailure)?;
            }
        }

        let caw = linebreak.cur_active_width;
        let mut b = 0;
        let mut fit_class = DECENT_FIT;
        let mut last_line_fit_found = false;

        if shortfall > 0 {
            if caw[3] != 0 || caw[4] != 0 || caw[5] != 0 {
                if linebreak.do_last_line_fit
                    && linebreak.cur_p.is_null()
                    && r_active.shortfall(mem).0 != 0
                    && r_active.glue(mem).0 > 0
                    && caw[3..6] == linebreak.fill_width
                {
                    g = if r_active.shortfall(mem).0 > 0 {
                        caw[2]
                    } else {
                        caw[6]
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
                        let last_line_fit = int_par(eqtb, INT_PAR_LAST_LINE_FIT);
                        if last_line_fit < 1000 {
                            g = crate::scaled_math::fract_in(
                                errors,
                                g,
                                last_line_fit,
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
                            g = g.min(shortfall);
                            (b, fit_class) = stretch_badness(g, caw[2]);
                            last_line_fit_found = true;
                        } else if g < 0 {
                            g = g.max(-caw[6]);
                            b = crate::helpers::badness(-g, caw[6]);
                            fit_class = shrink_fitness(b);
                            last_line_fit_found = true;
                        }
                    }
                }

                if !last_line_fit_found {
                    shortfall = 0;
                    b = 0;
                    fit_class = DECENT_FIT;
                }
            } else {
                (b, fit_class) = stretch_badness(shortfall, caw[2]);
            }
        } else {
            b = if -shortfall > caw[6] {
                INF_BAD + 1
            } else {
                crate::helpers::badness(-shortfall, caw[6])
            };
            fit_class = shrink_fitness(b);
        }

        if linebreak.do_last_line_fit && !last_line_fit_found {
            if linebreak.cur_p.is_null() {
                shortfall = 0;
            }
            g = match shortfall.signum() {
                1 => caw[2],
                -1 => caw[6],
                _ => 0,
            };
        }

        let node_r_stays_active;
        if b > INF_BAD || pi == EJECT_PENALTY {
            if linebreak.final_pass
                && linebreak.minimum_demerits == AWFUL_BAD
                && r_active.link(mem) == LAST_ACTIVE
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
            let mut d = int_par(eqtb, INT_PAR_LINE_PENALTY) + b;
            if d.abs() >= 10_000 {
                d = 100_000_000;
            } else {
                d *= d;
            }

            if pi > 0 {
                d += pi * pi;
            } else if pi < 0 && pi > EJECT_PENALTY {
                d -= pi * pi;
            }

            if break_type == HYPHENATED && r_active.break_type(mem) as i16 == HYPHENATED {
                d += if linebreak.cur_p.is_null() {
                    int_par(eqtb, INT_PAR_FINAL_HYPHEN_DEMERITS)
                } else {
                    int_par(eqtb, INT_PAR_DOUBLE_HYPHEN_DEMERITS)
                };
            }

            if (fit_class as i32 - r_active.fitness(mem) as i32).abs() > 1 {
                d += int_par(eqtb, INT_PAR_ADJ_DEMERITS);
            }

            d
        };

        d += r_active.total_demerits(mem);
        if d <= linebreak.minimal_demerits[fit_class] {
            linebreak.minimal_demerits[fit_class] = d;
            linebreak.best_place[fit_class] = r_active.break_node(mem);
            linebreak.best_pl_line[fit_class] = l;
            if linebreak.do_last_line_fit {
                linebreak.best_pl_short[fit_class] = shortfall;
                linebreak.best_pl_glue[fit_class] = g;
            }
            linebreak.minimum_demerits = linebreak.minimum_demerits.min(d);
        }

        if !node_r_stays_active {
            deactivate_node(mem, scanner, linebreak, r, &mut prev_r, &mut prev_prev_r);
        }
    }
}

/// Unlink and free the active node `r`, merging the delta nodes around it.
fn deactivate_node(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    linebreak: &mut LinebreakState,
    r: MemPtr,
    prev_r: &mut MemPtr,
    prev_prev_r: &mut MemPtr,
) {
    let next = r.link(mem);
    prev_r.set_link(mem, next);
    crate::mem::free_node(mem, r, linebreak.active_node_size as i32);

    if *prev_r == ACTIVE_LIST {
        let r = ACTIVE_LIST.link(mem);
        if let ActiveOrDelta::Delta(r_delta) = classify_active_or_delta(mem, r) {
            let w = delta_widths(r_delta, mem);
            for (width, delta) in scanner.active_width.iter_mut().zip(w) {
                *width += delta;
            }
            linebreak.cur_active_width[1..].copy_from_slice(&scanner.active_width[1..]);
            ACTIVE_LIST.set_link(mem, r_delta.link(mem));
            crate::mem::free_node(mem, r, DELTA_NODE_SIZE);
        }
    } else if let ActiveOrDelta::Delta(prev_delta) = classify_active_or_delta(mem, *prev_r) {
        let r = prev_r.link(mem);
        if r == LAST_ACTIVE {
            let w = delta_widths(prev_delta, mem);
            for (width, delta) in linebreak.cur_active_width.iter_mut().zip(w) {
                *width -= delta;
            }
            prev_prev_r.set_link(mem, LAST_ACTIVE);
            crate::mem::free_node(mem, *prev_r, DELTA_NODE_SIZE);
            *prev_r = *prev_prev_r;
        } else if let ActiveOrDelta::Delta(r_delta) = classify_active_or_delta(mem, r) {
            let rw = delta_widths(r_delta, mem);
            let pw = delta_widths(prev_delta, mem);
            for (width, delta) in linebreak.cur_active_width.iter_mut().zip(rw) {
                *width += delta;
            }
            set_delta_widths(prev_delta, mem, &std::array::from_fn(|k| pw[k] + rw[k]));
            prev_delta.set_link(mem, r_delta.link(mem));
            crate::mem::free_node(mem, r, DELTA_NODE_SIZE);
        }
    }
}

/// Cold aborts of the native-word hyphenation path.
pub(crate) enum HyphNativeAbort {
    Recycle(NodeRecycleAbort),
    NativeChar(crate::fonts::NativeCharAbort),
    Confusion(&'static std::ffi::CStr),
}

impl HyphNativeAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            HyphNativeAbort::Recycle(e) => e.into_engine_abort(),
            HyphNativeAbort::NativeChar(nc) => nc.into_engine_abort(),
            HyphNativeAbort::Confusion(msg) => crate::errors::EngineAbort::Confusion(msg.as_ptr()),
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
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
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

impl From<NodeRecycleAbort> for HyphenateAbort {
    fn from(e: NodeRecycleAbort) -> Self {
        HyphenateAbort::Recycle(e)
    }
}

/// Hyphenate the native word `ha` by splitting it into fragments separated by
/// discretionaries.
fn hyphenate_native_word(
    st: &mut EngineState,
    ha: NodePtr<WhatsitNode>,
    hn: i16,
) -> Result<(), HyphNativeAbort> {
    let hf = st.hyph.hf;
    let hyf_char = st.hyph.hyf_char;
    let mut s = predecessor(&st.mem, st.linebreak.cur_p, ha.ptr());

    st.hyph.hyphen_passed = 0;
    for j in st.hyph.l_hyf as i16..=(hn - st.hyph.r_hyf as i16) {
        if odd(st.hyph.hyf[j as usize]) {
            let hp = st.hyph.hyphen_passed;
            let q = native_word_fragment(
                &mut st.mem,
                &st.eqtb,
                &st.fonts,
                hf,
                ha,
                hp as i32,
                (j - hp) as i32,
            )
            .map_err(HyphNativeAbort::Recycle)?;
            s.set_link(&mut st.mem, q.ptr());

            let disc = crate::mem::alloc_disc(&mut st.mem)
                .expect("main memory arena has room for a disc node");
            let hyphen = crate::fonts::new_native_character_in(st, hf, hyf_char as u32)
                .map_err(HyphNativeAbort::NativeChar)?;
            disc.set_pre_break(&mut st.mem, hyphen.ptr());
            q.set_link(&mut st.mem, disc.ptr());
            s = disc.ptr();
            st.hyph.hyphen_passed = j;
        }
    }

    let hn = ha.native_length(&st.mem) as i16;
    st.linebreak.hn = hn;
    let hp = st.hyph.hyphen_passed;
    let q = native_word_fragment(
        &mut st.mem,
        &st.eqtb,
        &st.fonts,
        hf,
        ha,
        hp as i32,
        (hn - hp) as i32,
    )
    .map_err(HyphNativeAbort::Recycle)?;
    s.set_link(&mut st.mem, q.ptr());
    let next = ha.link(&st.mem);
    q.set_link(&mut st.mem, next);
    ha.set_link(&mut st.mem, MemPtr::NULL);
    crate::packaging::flush_node_list(&mut st.mem, ha.ptr()).map_err(HyphNativeAbort::Confusion)
}

/// Find the hyphen positions of the word in `hc[1..=hn]` (from the exception
/// dictionary or the patterns) and insert the discretionaries. `native_ha` is
/// the word's node when it is a native word.
fn hyphenate(
    st: &mut EngineState,
    native_ha: Option<NodePtr<WhatsitNode>>,
) -> Result<(), HyphenateAbort> {
    let mut hn = st.linebreak.hn;
    st.hyph.hyf[..=hn as usize].fill(0);

    let mut h = st.hyph.hc[1];
    hn += 1;
    st.hyph.hc[hn as usize] = st.hyph.cur_lang as i32;
    for j in 2..=hn as usize {
        h = (h + h + st.hyph.hc[j]) % HYPH_PRIME;
    }

    loop {
        let k = st.hyph.hyph_word[h as usize];
        if k == 0 {
            break;
        }

        if crate::string_pool::length(&st.strings, k) == hn as i32 {
            let start = st.strings.str_start[(k - TOO_BIG_CHAR) as usize] as usize;
            let word = &st.strings.str_pool[start..start + hn as usize];
            if word
                .iter()
                .zip(&st.hyph.hc[1..=hn as usize])
                .all(|(&u, &c)| u as i32 == c)
            {
                let mut s = MemPtr(st.hyph.hyph_list[h as usize]);
                while !s.is_null() {
                    st.hyph.hyf[s.info(&st.mem) as usize] = 1;
                    s = s.link(&st.mem);
                }
                hn -= 1;
                st.linebreak.hn = hn;
                return insert_hyphens(st, hn, native_ha);
            }
        }

        let link = st.hyph.hyph_link[h as usize];
        if link == 0 {
            break;
        }
        h = link as i32 - 1;
    }

    hn -= 1;
    st.linebreak.hn = hn;
    let lang = st.hyph.cur_lang as usize;
    if st.hyph.trie_trc[lang + 1] != st.hyph.cur_lang as u16 {
        return Ok(());
    }

    let hyph = &mut st.hyph;
    hyph.hc[0] = 0;
    hyph.hc[(hn + 1) as usize] = 0;
    hyph.hc[(hn + 2) as usize] = hyph.max_hyph_char;

    for j in 0..=(hn - hyph.r_hyf as i16 + 1) {
        let mut z = hyph.trie_trl[lang + 1] + hyph.hc[j as usize];
        let mut l = j;
        while hyph.hc[l as usize] == hyph.trie_trc[z as usize] as i32 {
            if hyph.trie_tro[z as usize] != MIN_TRIE_OP {
                let mut v = hyph.trie_tro[z as usize];
                loop {
                    v += hyph.op_start[lang];
                    let i = l - hyph.hyf_distance[v as usize];
                    if hyph.hyf_num[v as usize] as u8 > hyph.hyf[i as usize] {
                        hyph.hyf[i as usize] = hyph.hyf_num[v as usize] as u8;
                    }
                    v = hyph.hyf_next[v as usize] as i32;
                    if v == MIN_TRIE_OP {
                        break;
                    }
                }
            }
            l += 1;
            z = hyph.trie_trl[z as usize] + hyph.hc[l as usize];
        }
    }

    insert_hyphens(st, hn, native_ha)
}

/// Append the list after `HOLD_HEAD` to `disc`'s pre- or post-break list,
/// whose current tail is `minor_tail`.
fn append_to_disc_list(
    mem: &mut MemState,
    disc: NodePtr<DiscNode>,
    post: bool,
    minor_tail: &mut MemPtr,
) {
    let hold = HOLD_HEAD.link(mem);
    if hold.0 <= TEX_NULL {
        return;
    }
    if !minor_tail.is_null() {
        minor_tail.set_link(mem, hold);
    } else if post {
        disc.set_post_break(mem, hold);
    } else {
        disc.set_pre_break(mem, hold);
    }
    *minor_tail = list_tail(mem, hold);
}

/// Replace the word `ha..hb` by a reconstituted list with a discretionary at
/// every odd `hyf` position.
fn insert_hyphens(
    st: &mut EngineState,
    hn: i16,
    native_ha: Option<NodePtr<WhatsitNode>>,
) -> Result<(), HyphenateAbort> {
    let l_hyf = st.hyph.l_hyf as i16;
    let r_hyf = st.hyph.r_hyf as i16;
    for j in 0..l_hyf {
        st.hyph.hyf[j as usize] = 0;
    }
    for j in 0..r_hyf {
        st.hyph.hyf[(hn - j) as usize] = 0;
    }

    if !(l_hyf..=hn - r_hyf).any(|j| odd(st.hyph.hyf[j as usize])) {
        return Ok(());
    }

    if let Some(ha) = native_ha {
        return hyphenate_native_word(st, ha, hn).map_err(HyphenateAbort::Native);
    }

    let hb = st.linebreak.hb;
    let ha = st.linebreak.ha;
    let q_final = hb.link(&st.mem);
    hb.set_link(&mut st.mem, MemPtr::NULL);
    let r = ha.link(&st.mem);
    ha.set_link(&mut st.mem, MemPtr::NULL);
    let bchar = st.hyph.hyf_bchar;

    let found2 = |st: &mut EngineState| {
        st.hyph.hu[0] = st.hyph.max_hyph_char;
        st.hyph.init_lig = false;
        st.hyph.init_list = MemPtr::NULL;
        (ha, 0)
    };
    let (mut s, mut j) = if is_char_node(&st.mem, ha) {
        if st.mem.b16s1(ha) as i32 != st.hyph.hf {
            found2(st)
        } else {
            st.hyph.init_list = ha;
            st.hyph.init_lig = false;
            st.hyph.hu[0] = st.mem.b16s0(ha) as i32;
            (predecessor(&st.mem, st.linebreak.cur_p, ha), 0)
        }
    } else if let Ok(AnyNode::Ligature(ha_lig)) = st.mem.classify(MemIdx::new(ha)) {
        if ha_lig.font(&st.mem).0 != st.hyph.hf {
            found2(st)
        } else {
            st.hyph.init_list = ha_lig.lig_ptr(&st.mem);
            st.hyph.init_lig = true;
            st.hyph.init_lft = ha_lig.subtype(&st.mem) > 1;
            st.hyph.hu[0] = ha_lig.character(&st.mem) as i32;
            if st.hyph.init_list.is_null() && st.hyph.init_lft {
                st.hyph.hu[0] = st.hyph.max_hyph_char;
                st.hyph.init_lig = false;
            }
            crate::mem::free_node(&mut st.mem, ha, SMALL_NODE_SIZE);
            (predecessor(&st.mem, st.linebreak.cur_p, ha), 0)
        }
    } else if !is_char_node(&st.mem, r)
        && matches!(
            st.mem.classify(MemIdx::new(r)),
            Ok(AnyNode::Ligature(lig)) if lig.subtype(&st.mem) > 1
        )
    {
        found2(st)
    } else {
        st.hyph.init_list = MemPtr::NULL;
        (ha, 1)
    };

    crate::packaging::flush_node_list(&mut st.mem, r).map_err(HyphenateAbort::Confusion)?;

    loop {
        let mut l = j;
        j = reconstitute(st, j, hn, bchar, st.hyph.hyf_char)? + 1;
        if st.hyph.hyphen_passed == 0 {
            let hold = HOLD_HEAD.link(&st.mem);
            s.set_link(&mut st.mem, hold);
            s = list_tail(&st.mem, s);
            if odd(st.hyph.hyf[(j - 1) as usize]) {
                l = j;
                st.hyph.hyphen_passed = j - 1;
                HOLD_HEAD.set_link(&mut st.mem, MemPtr::NULL);
            }
        }

        if st.hyph.hyphen_passed > 0 {
            loop {
                let disc = NodePtr::<DiscNode>::alloc(&mut st.mem)
                    .ok_or(HyphenateAbort::MainMemoryOverflow)?;
                let hold = HOLD_HEAD.link(&st.mem);
                disc.set_link(&mut st.mem, hold);
                let mut major_tail = disc.ptr();
                let mut r_count = 0;
                while major_tail.link(&st.mem).0 > TEX_NULL {
                    major_tail = major_tail.link(&st.mem);
                    r_count += 1;
                }

                let mut i = st.hyph.hyphen_passed;
                st.hyph.hyf[i as usize] = 0;
                let mut minor_tail = MemPtr::NULL;
                disc.set_pre_break(&mut st.mem, MemPtr::NULL);
                let hyf_node =
                    crate::main_control::new_character_in(st, st.hyph.hf, st.hyph.hyf_char as u16)
                        .map_err(HyphenateAbort::NewCharacter)?;
                let mut saved_c = 0;
                if hyf_node != TEX_NULL {
                    i += 1;
                    saved_c = st.hyph.hu[i as usize];
                    st.hyph.hu[i as usize] = st.hyph.hyf_char;
                    crate::packaging::free_avail(&mut st.mem, MemPtr(hyf_node));
                }

                while l <= i {
                    let font_bchar = st.fonts.font_bchar_entry(st.hyph.hf);
                    l = reconstitute(st, l, i, font_bchar, TOO_BIG_CHAR)? + 1;
                    append_to_disc_list(&mut st.mem, disc, false, &mut minor_tail);
                }

                if hyf_node != TEX_NULL {
                    st.hyph.hu[i as usize] = saved_c;
                    l = i;
                }

                minor_tail = MemPtr::NULL;
                disc.set_post_break(&mut st.mem, MemPtr::NULL);
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
                        l = reconstitute(st, l, hn, bchar, TOO_BIG_CHAR)? + 1;
                        if c_loc > 0 {
                            st.hyph.hu[c_loc as usize] = c;
                            c_loc = 0;
                        }
                        append_to_disc_list(&mut st.mem, disc, true, &mut minor_tail);
                        if l >= j {
                            break;
                        }
                    }

                    while l > j {
                        j = reconstitute(st, j, hn, bchar, TOO_BIG_CHAR)? + 1;
                        let hold = HOLD_HEAD.link(&st.mem);
                        major_tail.set_link(&mut st.mem, hold);
                        while major_tail.link(&st.mem).0 > TEX_NULL {
                            major_tail = major_tail.link(&st.mem);
                            r_count += 1;
                        }
                    }
                }

                if r_count > 127 {
                    let replacement = disc.link(&st.mem);
                    s.set_link(&mut st.mem, replacement);
                    disc.set_link(&mut st.mem, MemPtr::NULL);
                    crate::packaging::flush_node_list(&mut st.mem, disc.ptr())
                        .map_err(HyphenateAbort::Confusion)?;
                } else {
                    s.set_link(&mut st.mem, disc.ptr());
                    disc.set_subtype(&mut st.mem, r_count as u16);
                }
                s = major_tail;
                st.hyph.hyphen_passed = j - 1;
                HOLD_HEAD.set_link(&mut st.mem, MemPtr::NULL);

                if !odd(st.hyph.hyf[(j - 1) as usize]) {
                    break;
                }
            }
        }

        if j > hn {
            break;
        }
    }

    s.set_link(&mut st.mem, q_final);
    crate::mem::flush_list(&mut st.mem, st.hyph.init_list);
    Ok(())
}

/// Append a native word of font `f` holding `text[..len]` to the current list.
///
/// # Safety
/// `text` must be valid for `len` reads.
unsafe fn append_native_word(
    st: &mut EngineState,
    f: i32,
    text: *const u16,
    len: i32,
) -> Result<(), NodeRecycleAbort> {
    let w = crate::fonts::new_native_word_node(&mut st.mem, &st.eqtb, f, len)?;
    st.input.cur_list.tail.set_link(&mut st.mem, w.ptr());
    st.input.cur_list.tail = w.ptr();
    if len > 0 {
        let dst = w.native_text_ptr_mut(&mut st.mem);
        std::ptr::copy_nonoverlapping(text, dst, len as usize);
    }
    set_native_metrics(&mut st.mem, &st.eqtb, &st.fonts, w)?;
    Ok(())
}

/// Append `native_text[s..s + len]` as native words, split at the locale's
/// line-break opportunities when `\XeTeXlinebreaklocale` is set.
///
/// # Safety
/// `s..s + len` must lie within `scanner.native_text`.
pub(crate) unsafe fn do_locale_linebreaks_in(
    st: &mut EngineState,
    s: i32,
    len: i32,
) -> Result<(), NodeRecycleAbort> {
    let main_f = st.mainloop.main_f;
    // `native_text` is not resized here, so the pointer stays valid across the allocations.
    let native_text = st.scanner.native_text.as_mut_ptr();

    let locale = int_par(&st.eqtb, INT_PAR_XETEX_LINEBREAK_LOCALE);
    if locale == 0 || len == 1 {
        return append_native_word(st, main_f, native_text.add(s as usize), len);
    }

    let use_skip = glue_par(&st.eqtb, GLUE_PAR_XETEX_LINEBREAK_SKIP).ptr() != ZERO_GLUE;
    let penalty = int_par(&st.eqtb, INT_PAR_XETEX_LINEBREAK_PENALTY);
    let use_penalty = penalty != 0 || !use_skip;
    linebreak_start_in(st, main_f, locale, native_text.add(s as usize), len)?;
    let mut offs = 0;
    loop {
        let prev_offs = offs;
        offs = linebreak_next(&st.fonts, main_f);
        if offs > 0 {
            if prev_offs != 0 {
                if use_penalty {
                    let node = crate::mem::alloc_penalty(&mut st.mem, penalty)
                        .ok_or(NodeRecycleAbort::Overflow)?;
                    st.input.cur_list.tail.set_link(&mut st.mem, node.ptr());
                    st.input.cur_list.tail = node.ptr();
                }
                if use_skip {
                    let node = crate::mem::alloc_param_glue(
                        &mut st.mem,
                        &st.eqtb,
                        GLUE_PAR_XETEX_LINEBREAK_SKIP,
                    )
                    .ok_or(NodeRecycleAbort::Overflow)?;
                    st.input.cur_list.tail.set_link(&mut st.mem, node.ptr());
                    st.input.cur_list.tail = node.ptr();
                }
            }
            append_native_word(
                st,
                main_f,
                native_text.add((s + prev_offs) as usize),
                offs - prev_offs,
            )?;
        }
        if offs < 0 {
            return Ok(());
        }
    }
}
