//! Box packing (`hpack`, `vpackage`), vertical breaking (`vert_break`,
//! `vsplit`, `prune_page_top`, `do_marks`) and node-list copying and freeing.

use crate::consts::{
    ACCENT_NOAD_SIZE, A_LEADERS, BEFORE, BOX_BASE, BOX_NODE_SIZE, DIMEN_PAR_HFUZZ,
    DIMEN_PAR_LINE_SKIP_LIMIT, DIMEN_PAR_OVERFULL_RULE, DIMEN_PAR_SPLIT_MAX_DEPTH, DIMEN_PAR_VFUZZ,
    EJECT_PENALTY, EXPLICIT, FIL, FILL, FILLL, FRACTION_NOAD_SIZE, GLUE_PAR_BASELINE_SKIP,
    GLUE_PAR_LINE_SKIP, GLUE_PAR_SPLIT_TOP_SKIP, GLYPH_NODE_SIZE, IGNORE_DEPTH, INDEX_NODE_SIZE,
    INF_BAD, INF_PENALTY, INS_NODE_SIZE, INT_PAR_HBADNESS, INT_PAR_SAVING_VDISCARDS,
    INT_PAR_TEXXET, INT_PAR_VBADNESS, INT_PAR_XETEX_UPWARDS, MARGIN_KERN_NODE_SIZE,
    MARK_CLASS_NODE_SIZE, MAX_HALFWORD, MEDIUM_NODE_SIZE, MIN_HALFWORD, NOAD_SIZE, NORMAL,
    OPEN_NODE_SIZE, PIC_NODE_SIZE, RADICAL_NOAD_SIZE, RULE_NODE_SIZE, R_CODE, SMALL_NODE_SIZE,
    SPLIT_BOT_MARK_CODE, SPLIT_FIRST_MARK_CODE, STYLE_NODE_SIZE, SUB_BOX, TEX_NULL,
    WRITE_NODE_SIZE,
};
use crate::errors::ErrorOutcome;
use crate::fonts::{effective_char, new_native_word_node};
use crate::main_control::VSPLIT_CODE;
use crate::mem::{
    add_glue_ref, add_token_ref, end_lr_type, free_avail, get_avail, is_char_node, list_tail,
    pop_lr, predecessor, push_lr, GARBAGE, TEMP_HEAD,
};
use crate::node::{
    AnyBox, AnyNode, CharNode, DiscNode, GlueNode, GlueSpecNode, HListNode, IsBox, KernNode,
    MarkNode, NodeKind, NodePtr, RuleDimen, SaNode, VListNode, WhatsitNode, WhatsitSubtype,
};
use crate::pagebuilder::{FIRE_UP_DONE, FIRE_UP_INIT};
use crate::scan::{NodeRecycleAbort, BOX_VAL, MARK_VAL};
use crate::state::{EngineState, EqtbState, FontState, MemState, ScannerState, ShipoutState};
use crate::units::{GlueSign, MemIdx, MemPtr, NodeType, Scaled};

/// The word of a box whose link field is its `list_ptr`.
const LIST_OFFSET: i32 = 5;

const SYNCTEX_FIELD_SIZE: i32 = 1;
const MEMORY_WORD_SIZE: i32 = 8;

pub(crate) const EXACTLY: i16 = 0;
pub(crate) const ADDITIONAL: i16 = 1;

const VSPLIT_INIT: i16 = 0;
const DESTROY_MARKS: i16 = 3;

fn as_disc(mem: &MemState, p: MemPtr) -> Option<NodePtr<DiscNode>> {
    if is_char_node(mem, p) {
        return None;
    }
    mem.classify_disc(MemIdx::new(p)).ok().flatten()
}

/// Store `v` in the first empty slot of a two-slot record.
#[inline]
fn push_slot<T>(slots: &mut [Option<T>; 2], v: T) {
    let i = usize::from(slots[0].is_some());
    slots[i] = Some(v);
}

/// The size of whatsit `w`, `None` for an unknown subtype.
fn whatsit_node_size(mem: &MemState, w: NodePtr<WhatsitNode>) -> Option<i32> {
    use WhatsitSubtype::*;
    Some(match w.whatsit_subtype(mem)? {
        Open => OPEN_NODE_SIZE,
        Write | Special => WRITE_NODE_SIZE,
        Close | Language | PdfSavePos => SMALL_NODE_SIZE,
        NativeWord | NativeWordAt => w.native_size(mem),
        Glyph => GLYPH_NODE_SIZE,
        Pic | Pdf => {
            PIC_NODE_SIZE + (w.pic_path_len(mem) as i32 + MEMORY_WORD_SIZE - 1) / MEMORY_WORD_SIZE
        }
    })
}

#[inline]
fn get_node(mem: &mut MemState, size: i32) -> Result<MemPtr, NodeRecycleAbort> {
    crate::mem::get_node(mem, size).ok_or(NodeRecycleAbort::Overflow)
}

/// `r` typed like `src`, whose header word `r` has just received.
#[inline]
fn copied_node<K: NodeKind>(_src: NodePtr<K>, r: MemPtr) -> NodePtr<K> {
    // SAFETY: `r`'s NODE_type word was copied from `src`'s.
    unsafe { NodePtr::from_idx_unchecked(MemIdx::new(r)) }
}

/// Copy the first `words` words of node `src` to node `dst`.
#[inline]
fn copy_words(mem: &mut MemState, dst: MemPtr, src: MemPtr, words: i32) {
    for w in (0..words).rev() {
        mem.set_word(dst + w, mem.word(src + w));
    }
}

fn copy_box_shell(
    mem: &mut MemState,
    src: AnyBox,
    list: MemPtr,
) -> Result<AnyBox, NodeRecycleAbort> {
    let r = get_node(mem, BOX_NODE_SIZE)?;
    copy_words(mem, r, src.ptr(), BOX_NODE_SIZE);
    let r = match src {
        AnyBox::HList(b) => AnyBox::HList(copied_node(b, r)),
        AnyBox::VList(b) => AnyBox::VList(copied_node(b, r)),
    };
    r.set_list_ptr(mem, list);
    Ok(r)
}

/// A copy of glue node `p` sharing its spec; the leader pointer is left to the caller.
fn copy_glue_shell(
    mem: &mut MemState,
    p: NodePtr<GlueNode>,
) -> Result<NodePtr<GlueNode>, NodeRecycleAbort> {
    let spec = p.glue_ptr(mem);
    let r = get_node(mem, MEDIUM_NODE_SIZE)?;
    add_glue_ref(mem, spec);
    let sync = MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE;
    mem.set_word(r, mem.word(p.ptr()));
    mem.set_word(r + sync, mem.word(p.ptr() + sync));
    let r = copied_node(p, r);
    r.set_glue_ptr(mem, spec);
    Ok(r)
}

/// Allocate a copy of whatsit `w`, returning it and how many of its leading
/// words are still to be copied.
fn copy_whatsit(
    mem: &mut MemState,
    w: NodePtr<WhatsitNode>,
) -> Result<(MemPtr, i32), NodeRecycleAbort> {
    let Some(size) = whatsit_node_size(mem, w) else {
        return Err(NodeRecycleAbort::Confusion(c"ext2"));
    };
    let r = get_node(mem, size)?;
    match w.whatsit_subtype(mem) {
        Some(WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt) => {
            copy_words(mem, r, w.ptr(), size);
            let copy = copied_node(w, r);
            copy.set_native_glyph_info_handle(mem, 0);
            copy.set_native_glyph_count(mem, 0);
            copy.share_native_glyphs(mem, w);
            Ok((r, 0))
        }
        Some(WhatsitSubtype::Write | WhatsitSubtype::Special) => {
            add_token_ref(mem, w.write_tokens(mem));
            Ok((r, size))
        }
        Some(WhatsitSubtype::PdfSavePos) => Ok((r, 1)),
        _ => Ok((r, size)),
    }
}

/// Copy the list `p` without the contents of its boxes, linking the copies
/// after `h` and ending them with `t`.
pub fn just_copy(mem: &mut MemState, p: i32, h: i32, t: i32) -> Result<(), NodeRecycleAbort> {
    let mut p = MemPtr(p);
    let mut h = MemPtr(h);

    while !p.is_null() {
        let copy = if is_char_node(mem, p) {
            let r = get_avail(mem).ok_or(NodeRecycleAbort::OverflowRunaway)?;
            Some((r, 1))
        } else {
            match mem.classify(MemIdx::new(p)).ok() {
                Some(AnyNode::HList(b)) => Some((
                    copy_box_shell(mem, AnyBox::HList(b), MemPtr::NULL)?.ptr(),
                    0,
                )),
                Some(AnyNode::VList(b)) => Some((
                    copy_box_shell(mem, AnyBox::VList(b), MemPtr::NULL)?.ptr(),
                    0,
                )),
                Some(AnyNode::Rule(_)) => Some((get_node(mem, RULE_NODE_SIZE)?, RULE_NODE_SIZE)),
                Some(AnyNode::Ligature(lig)) => {
                    let r = get_avail(mem).ok_or(NodeRecycleAbort::OverflowRunaway)?;
                    mem.set_word(r, mem.word(lig.ptr() + 1));
                    Some((r, 0))
                }
                Some(AnyNode::Kern(_) | AnyNode::Math(_)) => {
                    Some((get_node(mem, MEDIUM_NODE_SIZE)?, MEDIUM_NODE_SIZE))
                }
                Some(AnyNode::Glue(g)) => {
                    let r = copy_glue_shell(mem, g)?;
                    r.set_leader_ptr(mem, MemPtr::NULL);
                    Some((r.ptr(), 0))
                }
                Some(AnyNode::Whatsit(w)) => Some(copy_whatsit(mem, w)?),
                _ => None,
            }
        };

        if let Some((r, words)) = copy {
            copy_words(mem, r, p, words);
            h.set_link(mem, r);
            h = r;
        }
        p = p.link(mem);
    }

    h.set_link(mem, MemPtr(t));
    Ok(())
}

/// Reverse the hlist after `p` for TeX--XeT, leaving the result after `TEMP_HEAD`.
pub fn just_reverse(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    p: i32,
) -> Result<(), NodeRecycleAbort> {
    let p = MemPtr(p);
    let mut m = MIN_HALFWORD;
    let mut n = MIN_HALFWORD;

    let mut q = if TEMP_HEAD.link(mem).is_null() {
        just_copy(mem, p.link(mem).0, TEMP_HEAD.0, TEX_NULL)?;
        TEMP_HEAD.link(mem)
    } else {
        let q = p.link(mem);
        p.set_link(mem, MemPtr::NULL);
        flush_node_list(mem, TEMP_HEAD.link(mem)).map_err(NodeRecycleAbort::Confusion)?;
        q
    };

    let t = crate::shipout::new_edge(mem, scanner.cur_dir, 0).ok_or(NodeRecycleAbort::Overflow)?;
    let mut l = t.ptr();
    scanner.cur_dir = 1 - scanner.cur_dir;

    'done: {
        while !q.is_null() {
            if is_char_node(mem, q) {
                loop {
                    let p = q;
                    q = p.link(mem);
                    p.set_link(mem, l);
                    l = p;
                    if !is_char_node(mem, q) {
                        break;
                    }
                }
                continue;
            }

            let p = q;
            q = p.link(mem);

            if let Ok(AnyNode::Math(math)) = mem.classify(MemIdx::new(p)) {
                let subtype = math.subtype(mem);
                if subtype % 2 == 1 {
                    if scanner.lr_ptr.info(mem) != end_lr_type(subtype) {
                        math.into_kern(mem);
                        scanner.lr_problems += 1;
                    } else {
                        pop_lr(mem, &mut scanner.lr_ptr);

                        if n > MIN_HALFWORD {
                            n -= 1;
                            math.set_subtype(mem, subtype - 1);
                        } else if m > MIN_HALFWORD {
                            m -= 1;
                            math.into_kern(mem);
                        } else {
                            t.set_width(mem, math.width(mem));
                            t.set_link(mem, q);
                            crate::mem::free_node(mem, p, MEDIUM_NODE_SIZE);
                            break 'done;
                        }
                    }
                } else {
                    push_lr(mem, &mut scanner.lr_ptr, end_lr_type(subtype))
                        .ok_or(NodeRecycleAbort::OverflowRunaway)?;

                    if n > MIN_HALFWORD || (subtype / R_CODE) as i16 != scanner.cur_dir {
                        n += 1;
                        math.set_subtype(mem, subtype + 1);
                    } else {
                        math.into_kern(mem);
                        m += 1;
                    }
                }
            }

            p.set_link(mem, l);
            l = p;
        }
    }

    TEMP_HEAD.set_link(mem, l);
    Ok(())
}

/// Free every node of the list `p` and everything it owns.
pub fn flush_node_list(
    mem: &mut MemState,
    p: impl crate::units::MemAddr,
) -> Result<(), &'static std::ffi::CStr> {
    let mut p = MemPtr(p.addr());
    while !p.is_null() {
        let q = p.link(mem);
        if is_char_node(mem, p) {
            free_avail(mem, p);
            p = q;
            continue;
        }

        let size = match mem.classify(MemIdx::new(p)).map_err(|_| c"flushing")? {
            AnyNode::HList(b) => {
                flush_node_list(mem, b.list_ptr(mem))?;
                BOX_NODE_SIZE
            }
            AnyNode::VList(b) => {
                flush_node_list(mem, b.list_ptr(mem))?;
                BOX_NODE_SIZE
            }
            AnyNode::Unset(u) => {
                flush_node_list(mem, u.list_ptr(mem))?;
                BOX_NODE_SIZE
            }
            AnyNode::Rule(_) => RULE_NODE_SIZE,
            AnyNode::Ins(ins) => {
                flush_node_list(mem, ins.ins_ptr(mem))?;
                crate::mem::delete_glue_ref(mem, ins.split_top_ptr(mem));
                INS_NODE_SIZE
            }
            AnyNode::Whatsit(w) => {
                let size = whatsit_node_size(mem, w).ok_or(c"ext3")?;
                match w.whatsit_subtype(mem) {
                    Some(WhatsitSubtype::Write | WhatsitSubtype::Special) => {
                        crate::mem::delete_token_ref(mem, w.write_tokens(mem));
                    }
                    Some(WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt) => {
                        w.release_native_glyphs(mem);
                    }
                    _ => {}
                }
                size
            }
            AnyNode::Glue(glue) => {
                crate::mem::delete_glue_ref(mem, glue.glue_ptr(mem));
                flush_node_list(mem, glue.leader_ptr(mem))?;
                MEDIUM_NODE_SIZE
            }
            AnyNode::Kern(_) | AnyNode::Math(_) | AnyNode::Penalty(_) => MEDIUM_NODE_SIZE,
            AnyNode::MarginKern(_) => MARGIN_KERN_NODE_SIZE,
            AnyNode::Ligature(lig) => {
                flush_node_list(mem, lig.lig_ptr(mem))?;
                SMALL_NODE_SIZE
            }
            AnyNode::Mark(mark) => {
                crate::mem::delete_token_ref(mem, mark.mark_ptr(mem));
                SMALL_NODE_SIZE
            }
            AnyNode::Disc(disc) => {
                flush_node_list(mem, disc.pre_break(mem))?;
                flush_node_list(mem, disc.post_break(mem))?;
                SMALL_NODE_SIZE
            }
            AnyNode::Adjust(adjust) => {
                flush_node_list(mem, adjust.adjust_ptr(mem))?;
                SMALL_NODE_SIZE
            }
            AnyNode::Style(_) => STYLE_NODE_SIZE,
            AnyNode::Choice(choice) => {
                flush_node_list(mem, choice.display_mlist(mem))?;
                flush_node_list(mem, choice.text_mlist(mem))?;
                flush_node_list(mem, choice.script_mlist(mem))?;
                flush_node_list(mem, choice.script_script_mlist(mem))?;
                STYLE_NODE_SIZE
            }
            AnyNode::Noad(noad) => match noad.noad_type(mem) {
                NodeType::LeftNoad | NodeType::RightNoad => NOAD_SIZE,
                NodeType::FractionNoad => {
                    flush_node_list(mem, noad.supscr_field(mem))?;
                    flush_node_list(mem, noad.subscr_field(mem))?;
                    FRACTION_NOAD_SIZE
                }
                noad_type => {
                    if noad.nucleus_type(mem) >= SUB_BOX {
                        flush_node_list(mem, noad.nucleus_field(mem))?;
                    }
                    if noad.supscr_type(mem) >= SUB_BOX {
                        flush_node_list(mem, noad.supscr_field(mem))?;
                    }
                    if noad.subscr_type(mem) >= SUB_BOX {
                        flush_node_list(mem, noad.subscr_field(mem))?;
                    }
                    match noad_type {
                        NodeType::RadicalNoad => RADICAL_NOAD_SIZE,
                        NodeType::AccentNoad => ACCENT_NOAD_SIZE,
                        _ => NOAD_SIZE,
                    }
                }
            },
        };
        crate::mem::free_node(mem, p, size);
        p = q;
    }
    Ok(())
}

/// A deep copy of the list `p`.
/// A copy of box `b` and its contents.
pub(crate) fn copy_box(mem: &mut MemState, b: AnyBox) -> Result<AnyBox, NodeRecycleAbort> {
    let r = copy_list(mem, b.ptr())?;
    Ok(match b {
        AnyBox::HList(h) => AnyBox::HList(copied_node(h, r)),
        AnyBox::VList(v) => AnyBox::VList(copied_node(v, r)),
    })
}

pub fn copy_node_list(mem: &mut MemState, p: i32) -> Result<i32, NodeRecycleAbort> {
    Ok(copy_list(mem, MemPtr(p))?.0)
}

fn copy_list(mem: &mut MemState, mut p: MemPtr) -> Result<MemPtr, NodeRecycleAbort> {
    let h = get_avail(mem).ok_or(NodeRecycleAbort::OverflowRunaway)?;
    let mut q = h;
    while !p.is_null() {
        let (r, words) = if is_char_node(mem, p) {
            (get_avail(mem).ok_or(NodeRecycleAbort::OverflowRunaway)?, 1)
        } else {
            copy_node(mem, p)?
        };
        copy_words(mem, r, p, words);
        q.set_link(mem, r);
        q = r;
        p = p.link(mem);
    }
    q.set_link(mem, MemPtr::NULL);
    let result = h.link(mem);
    free_avail(mem, h);
    Ok(result)
}

/// Allocate a copy of the non-char node `p`, returning it and how many of its
/// leading words are still to be copied.
fn copy_node(mem: &mut MemState, p: MemPtr) -> Result<(MemPtr, i32), NodeRecycleAbort> {
    let node = mem
        .classify(MemIdx::new(p))
        .map_err(|_| NodeRecycleAbort::Confusion(c"copying"))?;
    Ok(match node {
        AnyNode::HList(b) => {
            let list = copy_list(mem, b.list_ptr(mem))?;
            (copy_box_shell(mem, AnyBox::HList(b), list)?.ptr(), 0)
        }
        AnyNode::VList(b) => {
            let list = copy_list(mem, b.list_ptr(mem))?;
            (copy_box_shell(mem, AnyBox::VList(b), list)?.ptr(), 0)
        }
        AnyNode::Unset(u) => {
            let r = get_node(mem, BOX_NODE_SIZE)?;
            copy_words(mem, r, p, BOX_NODE_SIZE);
            let list = copy_list(mem, u.list_ptr(mem))?;
            copied_node(u, r).set_list_ptr(mem, list);
            (r, 0)
        }
        AnyNode::Rule(_) => (
            get_node(mem, RULE_NODE_SIZE)?,
            RULE_NODE_SIZE - SYNCTEX_FIELD_SIZE,
        ),
        AnyNode::Ins(ins) => {
            let r = get_node(mem, INS_NODE_SIZE)?;
            copy_words(mem, r, p, INS_NODE_SIZE);
            add_glue_ref(mem, ins.split_top_ptr(mem));
            let list = copy_list(mem, ins.ins_ptr(mem))?;
            copied_node(ins, r).set_ins_ptr(mem, list);
            (r, 0)
        }
        AnyNode::Whatsit(w) => copy_whatsit(mem, w)?,
        AnyNode::Glue(g) => {
            let r = copy_glue_shell(mem, g)?;
            let leaders = copy_list(mem, g.leader_ptr(mem))?;
            r.set_leader_ptr(mem, leaders);
            (r.ptr(), 0)
        }
        AnyNode::Kern(_) | AnyNode::Math(_) | AnyNode::Penalty(_) => {
            (get_node(mem, MEDIUM_NODE_SIZE)?, MEDIUM_NODE_SIZE)
        }
        AnyNode::MarginKern(_) => (get_node(mem, MARGIN_KERN_NODE_SIZE)?, MARGIN_KERN_NODE_SIZE),
        AnyNode::Ligature(lig) => {
            let r = get_node(mem, SMALL_NODE_SIZE)?;
            copy_words(mem, r, p, SMALL_NODE_SIZE);
            let components = copy_list(mem, lig.lig_ptr(mem))?;
            copied_node(lig, r).set_lig_ptr(mem, components);
            (r, 0)
        }
        AnyNode::Disc(disc) => {
            let r = get_node(mem, SMALL_NODE_SIZE)?;
            copy_words(mem, r, p, 1);
            let copy = copied_node(disc, r);
            let pre_break = copy_list(mem, disc.pre_break(mem))?;
            copy.set_pre_break(mem, pre_break);
            let post_break = copy_list(mem, disc.post_break(mem))?;
            copy.set_post_break(mem, post_break);
            (r, 0)
        }
        AnyNode::Mark(mark) => {
            let r = get_node(mem, SMALL_NODE_SIZE)?;
            add_token_ref(mem, mark.mark_ptr(mem));
            (r, SMALL_NODE_SIZE)
        }
        AnyNode::Adjust(adjust) => {
            let r = get_node(mem, SMALL_NODE_SIZE)?;
            copy_words(mem, r, p, 1);
            let list = copy_list(mem, adjust.adjust_ptr(mem))?;
            copied_node(adjust, r).set_adjust_ptr(mem, list);
            (r, 0)
        }
        _ => return Err(NodeRecycleAbort::Confusion(c"copying")),
    })
}

/// Append box `b` to the current vertical list, preceded by interline glue.
pub fn append_to_vlist(
    cl: &mut crate::mem_types::ListStateRecord,
    mem: &mut MemState,
    eqtb: &EqtbState,
    b: AnyBox,
) -> Result<(), NodeRecycleAbort> {
    let upwards = eqtb.int_par(INT_PAR_XETEX_UPWARDS) > 0;
    let prev_depth = cl.aux_s1();
    if prev_depth > IGNORE_DEPTH {
        let baseline_skip = eqtb.glue_par(GLUE_PAR_BASELINE_SKIP);
        let d = baseline_skip.width(mem).0
            - prev_depth
            - if upwards {
                b.depth(mem).0
            } else {
                b.height(mem).0
            };
        let p = if d < eqtb.dimen_par(DIMEN_PAR_LINE_SKIP_LIMIT) {
            crate::mem::new_param_glue(mem, eqtb, GLUE_PAR_LINE_SKIP)
                .ok_or(NodeRecycleAbort::Overflow)?
        } else {
            let p = crate::mem::new_skip_param(mem, eqtb, GLUE_PAR_BASELINE_SKIP)
                .ok_or(NodeRecycleAbort::Overflow)?;
            p.glue_ptr(mem).set_width(mem, Scaled(d));
            p
        };
        cl.tail_append(mem, p.ptr());
    }
    cl.tail_append(mem, b.ptr());
    cl.set_aux_s1(if upwards {
        b.height(mem).0
    } else {
        b.depth(mem).0
    });
    Ok(())
}

pub(crate) fn append_to_vlist_emit_in(
    st: &mut EngineState,
    b: AnyBox,
) -> Result<(), NodeRecycleAbort> {
    append_to_vlist(&mut st.input.cur_list, &mut st.mem, &st.eqtb, b)
}

struct VertBreakCursor {
    p: MemPtr,
    prev_p: MemPtr,
    /// Whether glue after the node at `p` is a legal breakpoint.
    breakable: bool,
    /// The same for `prev_p`.
    prev_breakable: bool,
    least_cost: i32,
    best_place: MemPtr,
    prev_dp: i32,
    finish_glue_after_shrinkage: Option<NodePtr<GlueNode>>,
}

enum VertBreakStep {
    Done(MemPtr),
    InfiniteShrinkage {
        glue: NodePtr<GlueNode>,
        spec: NodePtr<GlueSpecNode>,
    },
}

enum VertBreakNext {
    NotFound,
    UpdateHeights(VertBreakUpdateSource),
    CheckPi {
        pi: i32,
        update_after: Option<VertBreakUpdateSource>,
    },
}

#[derive(Copy, Clone)]
enum VertBreakUpdateSource {
    Kern(NodePtr<KernNode>),
    Glue(NodePtr<GlueNode>),
}

fn start_vert_break(scanner: &mut ScannerState, p: MemPtr) -> VertBreakCursor {
    scanner.active_width[1..].fill(0);

    VertBreakCursor {
        p,
        prev_p: p,
        breakable: false,
        prev_breakable: false,
        least_cost: MAX_HALFWORD,
        best_place: MemPtr::NULL,
        prev_dp: 0,
        finish_glue_after_shrinkage: None,
    }
}

fn advance_vert_break(
    mem: &MemState,
    scanner: &mut ScannerState,
    cursor: &mut VertBreakCursor,
    d: i32,
) {
    if cursor.prev_dp > d {
        scanner.active_width[1] += cursor.prev_dp - d;
        cursor.prev_dp = d;
    }
    cursor.prev_p = cursor.p;
    cursor.prev_breakable = cursor.breakable;
    cursor.p = cursor.prev_p.link(mem);
}

fn finish_vert_break_updated_height(
    mem: &MemState,
    scanner: &mut ScannerState,
    cursor: &mut VertBreakCursor,
    width: i32,
    d: i32,
) {
    scanner.active_width[1] += cursor.prev_dp + width;
    cursor.prev_dp = 0;
    advance_vert_break(mem, scanner, cursor, d);
}

fn classify_vert_break_next(
    mem: &MemState,
    scanner: &mut ScannerState,
    cursor: &mut VertBreakCursor,
) -> Result<VertBreakNext, NodeRecycleAbort> {
    if cursor.p.is_null() {
        return Ok(VertBreakNext::CheckPi {
            pi: EJECT_PENALTY,
            update_after: None,
        });
    }

    let node = mem
        .classify(MemIdx::new(cursor.p))
        .map_err(|_| NodeRecycleAbort::Confusion(c"vertbreak"))?;
    cursor.breakable = node.precedes_break();
    match node {
        AnyNode::HList(b) => {
            scanner.active_width[1] += cursor.prev_dp + b.height(mem).0;
            cursor.prev_dp = b.depth(mem).0;
            Ok(VertBreakNext::NotFound)
        }
        AnyNode::VList(b) => {
            scanner.active_width[1] += cursor.prev_dp + b.height(mem).0;
            cursor.prev_dp = b.depth(mem).0;
            Ok(VertBreakNext::NotFound)
        }
        AnyNode::Rule(rule) => {
            scanner.active_width[1] += cursor.prev_dp + rule.height(mem).raw();
            cursor.prev_dp = rule.depth(mem).raw();
            Ok(VertBreakNext::NotFound)
        }
        AnyNode::Whatsit(w) => {
            if matches!(
                w.whatsit_subtype(mem),
                Some(WhatsitSubtype::Pic | WhatsitSubtype::Pdf)
            ) {
                scanner.active_width[1] += cursor.prev_dp + w.height(mem).0;
                cursor.prev_dp = w.depth(mem).0;
            }
            Ok(VertBreakNext::NotFound)
        }
        AnyNode::Glue(glue) => {
            if cursor.prev_breakable {
                Ok(VertBreakNext::CheckPi {
                    pi: 0,
                    update_after: Some(VertBreakUpdateSource::Glue(glue)),
                })
            } else {
                Ok(VertBreakNext::UpdateHeights(VertBreakUpdateSource::Glue(
                    glue,
                )))
            }
        }
        AnyNode::Kern(kern) => {
            let next = kern.link(mem);
            if !next.is_null() && matches!(mem.classify(MemIdx::new(next)), Ok(AnyNode::Glue(_))) {
                Ok(VertBreakNext::CheckPi {
                    pi: 0,
                    update_after: Some(VertBreakUpdateSource::Kern(kern)),
                })
            } else {
                Ok(VertBreakNext::UpdateHeights(VertBreakUpdateSource::Kern(
                    kern,
                )))
            }
        }
        AnyNode::Penalty(penalty) => Ok(VertBreakNext::CheckPi {
            pi: penalty.penalty(mem),
            update_after: None,
        }),
        AnyNode::Ins(_) | AnyNode::Mark(_) => Ok(VertBreakNext::NotFound),
        _ => Err(NodeRecycleAbort::Confusion(c"vertbreak")),
    }
}

fn vert_break_resume(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    cursor: &mut VertBreakCursor,
    h: i32,
    d: i32,
) -> Result<VertBreakStep, NodeRecycleAbort> {
    loop {
        if let Some(glue) = cursor.finish_glue_after_shrinkage.take() {
            let spec = glue.glue_ptr(mem);
            finish_vert_break_updated_height(mem, scanner, cursor, spec.width(mem).0, d);
            continue;
        }

        let update_source = match classify_vert_break_next(mem, scanner, cursor)? {
            VertBreakNext::NotFound => None,
            VertBreakNext::UpdateHeights(source) => Some(source),
            VertBreakNext::CheckPi { pi, update_after } => {
                if pi < INF_PENALTY {
                    let aw = &scanner.active_width;
                    let b = if aw[1] < h {
                        if aw[3] != 0 || aw[4] != 0 || aw[5] != 0 {
                            0
                        } else {
                            crate::helpers::badness(h - aw[1], aw[2])
                        }
                    } else if aw[1] - h > aw[6] {
                        MAX_HALFWORD
                    } else {
                        crate::helpers::badness(aw[1] - h, aw[6])
                    };
                    let b = if b < MAX_HALFWORD {
                        if pi <= EJECT_PENALTY {
                            pi
                        } else if b < INF_BAD {
                            b + pi
                        } else {
                            100_000
                        }
                    } else {
                        b
                    };
                    if b <= cursor.least_cost {
                        cursor.best_place = cursor.p;
                        cursor.least_cost = b;
                        scanner.best_height_plus_depth = scanner.active_width[1] + cursor.prev_dp;
                    }
                    if b == MAX_HALFWORD || pi <= EJECT_PENALTY {
                        return Ok(VertBreakStep::Done(cursor.best_place));
                    }
                }
                update_after
            }
        };

        match update_source {
            Some(VertBreakUpdateSource::Kern(kern)) => {
                finish_vert_break_updated_height(mem, scanner, cursor, kern.width(mem).0, d);
            }
            Some(VertBreakUpdateSource::Glue(glue)) => {
                let spec = glue.glue_ptr(mem);
                scanner.active_width[2 + spec.stretch_order(mem) as usize] += spec.stretch(mem).0;
                scanner.active_width[6] += spec.shrink(mem).0;
                if spec.shrink_order(mem) != NORMAL && spec.shrink(mem).0 != 0 {
                    cursor.finish_glue_after_shrinkage = Some(glue);
                    return Ok(VertBreakStep::InfiniteShrinkage { glue, spec });
                }
                finish_vert_break_updated_height(mem, scanner, cursor, spec.width(mem).0, d);
            }
            None => advance_vert_break(mem, scanner, cursor, d),
        }
    }
}

fn print_vert_break_infinite_shrinkage_error_in(st: &mut EngineState) -> ErrorOutcome {
    crate::output::print_err_in(st, c"Infinite glue shrinkage found in box being split");
    st.errors.help(&[
        r"The box you are \vsplitting contains some infinitely",
        r"shrinkable glue, e.g., `\vss' or `\vskip 0pt minus 1fil'.",
        "Such glue doesn't belong there; but you can safely proceed,",
        "since the offensive shrinkability has been made finite.",
    ]);
    crate::errors::error_in(st)
}

fn repair_vert_break_infinite_shrinkage(
    mem: &mut MemState,
    glue: NodePtr<GlueNode>,
    spec: NodePtr<GlueSpecNode>,
) -> Result<(), NodeRecycleAbort> {
    let spec = crate::linebreak::finite_shrink(mem, spec)?;
    glue.set_glue_ptr(mem, spec);
    Ok(())
}

pub(crate) enum VertBreakAbort {
    NodeRecycle(NodeRecycleAbort),
    Error(ErrorOutcome),
}

impl From<NodeRecycleAbort> for VertBreakAbort {
    fn from(e: NodeRecycleAbort) -> Self {
        VertBreakAbort::NodeRecycle(e)
    }
}

impl VertBreakAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            VertBreakAbort::NodeRecycle(e) => e.into_engine_abort(),
            VertBreakAbort::Error(o) => o.into_engine_abort(),
        }
    }
}

/// The best place to break the vlist `p` for a height `h` with maximum depth `d`.
pub(crate) fn vert_break_in(
    st: &mut EngineState,
    p: i32,
    h: i32,
    d: i32,
) -> Result<i32, VertBreakAbort> {
    let mut cursor = start_vert_break(&mut st.scanner, MemPtr(p));
    loop {
        match vert_break_resume(&mut st.mem, &mut st.scanner, &mut cursor, h, d)? {
            VertBreakStep::Done(best_place) => return Ok(best_place.0),
            VertBreakStep::InfiniteShrinkage { glue, spec } => {
                let outcome = print_vert_break_infinite_shrinkage_error_in(st);
                if !matches!(outcome, ErrorOutcome::Continue) {
                    return Err(VertBreakAbort::Error(outcome));
                }
                repair_vert_break_infinite_shrinkage(&mut st.mem, glue, spec)?;
            }
        }
    }
}

struct VsplitState {
    h: i32,
    v: NodePtr<VListNode>,
    split_max_d: i32,
    vert_break: VertBreakCursor,
}

#[derive(Copy, Clone)]
struct VsplitOutcome {
    result: Option<AnyBox>,
    packages: [Option<PackOutcome>; 2],
}

enum VsplitStart {
    Done(VsplitOutcome),
    NeedsVboxDiagnostic,
    Ready(VsplitState),
}

enum VsplitStep {
    Done(VsplitOutcome),
    InfiniteShrinkage {
        glue: NodePtr<GlueNode>,
        spec: NodePtr<GlueSpecNode>,
    },
}

/// The contents of box register `n`, leaving `n` in `cur_val`.
fn find_box_register(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    eqtb: &EqtbState,
    n: i32,
) -> Result<MemPtr, NodeRecycleAbort> {
    scanner.cur_val = n;
    if scanner.cur_val < 256 {
        return Ok(MemPtr(eqtb.b32s1(BOX_BASE + scanner.cur_val)));
    }
    let element = crate::save::find_sa_element(mem, scanner, BOX_VAL, scanner.cur_val);
    Ok(element.map_or(MemPtr::NULL, |e| e.sa_ptr(mem)))
}

fn reset_vsplit_marks(
    mem: &mut MemState,
    scanner: &mut ScannerState,
) -> Result<(), NodeRecycleAbort> {
    flush_node_list(mem, scanner.disc_ptr[VSPLIT_CODE as usize])
        .map_err(NodeRecycleAbort::Confusion)?;
    scanner.disc_ptr[VSPLIT_CODE as usize] = MemPtr::NULL;

    if let Some(root) = scanner.sa_root[MARK_VAL as usize].as_idx() {
        if do_marks(mem, scanner, VSPLIT_INIT, 0, root) {
            scanner.sa_root[MARK_VAL as usize] = MemPtr::NULL;
        }
    }

    if !scanner.cur_mark[SPLIT_FIRST_MARK_CODE].is_null() {
        crate::mem::delete_token_ref(mem, scanner.cur_mark[SPLIT_FIRST_MARK_CODE]);
        scanner.cur_mark[SPLIT_FIRST_MARK_CODE] = MemPtr::NULL;
        crate::mem::delete_token_ref(mem, scanner.cur_mark[SPLIT_BOT_MARK_CODE]);
        scanner.cur_mark[SPLIT_BOT_MARK_CODE] = MemPtr::NULL;
    }

    Ok(())
}

fn start_vsplit(
    mem: &mut MemState,
    eqtb: &EqtbState,
    scanner: &mut ScannerState,
    n: i32,
    h: i32,
) -> Result<VsplitStart, NodeRecycleAbort> {
    let v = find_box_register(mem, scanner, eqtb, n)?;
    reset_vsplit_marks(mem, scanner)?;

    if v.is_null() {
        return Ok(VsplitStart::Done(VsplitOutcome {
            result: None,
            packages: [None, None],
        }));
    }

    let v = match mem
        .classify_box(MemIdx::new(v))
        .map_err(|_| NodeRecycleAbort::Confusion(c"vsplit"))?
    {
        Some(AnyBox::VList(v)) => v,
        Some(AnyBox::HList(_)) | None => return Ok(VsplitStart::NeedsVboxDiagnostic),
    };

    Ok(VsplitStart::Ready(VsplitState {
        h,
        v,
        split_max_d: eqtb.dimen_par(DIMEN_PAR_SPLIT_MAX_DEPTH),
        vert_break: start_vert_break(scanner, v.list_ptr(mem)),
    }))
}

fn collect_split_mark(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    mark: NodePtr<MarkNode>,
) -> Result<(), NodeRecycleAbort> {
    let mark_class = mark.mark_class(mem);
    let mark_ptr = mark.mark_ptr(mem);
    if mark_class != 0 {
        let element = crate::save::create_sa_element(mem, scanner, MARK_VAL, mark_class)
            .ok_or(NodeRecycleAbort::Overflow)?;
        let (first, bot) = (SPLIT_FIRST_MARK_CODE as i32, SPLIT_BOT_MARK_CODE as i32);
        if element.child(mem, first).is_null() {
            element.set_child(mem, first, mark_ptr);
            add_token_ref(mem, mark_ptr);
        } else {
            crate::mem::delete_token_ref(mem, element.child(mem, bot));
        }
        element.set_child(mem, bot, mark_ptr);
        add_token_ref(mem, mark_ptr);
    } else if scanner.cur_mark[SPLIT_FIRST_MARK_CODE].is_null() {
        scanner.cur_mark[SPLIT_FIRST_MARK_CODE] = mark_ptr;
        scanner.cur_mark[SPLIT_BOT_MARK_CODE] = mark_ptr;
        mark_ptr.set_info(mem, mark_ptr.info(mem) + 2);
    } else {
        crate::mem::delete_token_ref(mem, scanner.cur_mark[SPLIT_BOT_MARK_CODE]);
        scanner.cur_mark[SPLIT_BOT_MARK_CODE] = mark_ptr;
        add_token_ref(mem, mark_ptr);
    }
    Ok(())
}

/// Detach the part of `state.v`'s list before `q`, recording its marks.
fn split_vlist_at_break(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    state: &VsplitState,
    q: MemPtr,
) -> Result<MemPtr, NodeRecycleAbort> {
    let mut p = state.v.list_ptr(mem);
    if p == q {
        state.v.set_list_ptr(mem, MemPtr::NULL);
    } else {
        loop {
            if let AnyNode::Mark(mark) = mem
                .classify(MemIdx::new(p))
                .map_err(|_| NodeRecycleAbort::Confusion(c"vsplit"))?
            {
                collect_split_mark(mem, scanner, mark)?;
            }
            if p.link(mem) == q {
                p.set_link(mem, MemPtr::NULL);
                break;
            }
            p = p.link(mem);
        }
    }

    Ok(state.v.list_ptr(mem))
}

fn store_split_remainder(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    eqtb: &mut EqtbState,
    q: MemPtr,
) -> Result<(), NodeRecycleAbort> {
    if scanner.cur_val < 256 {
        eqtb.set_b32s1(BOX_BASE + scanner.cur_val, q.0);
        return Ok(());
    }
    if let Some(element) = crate::save::find_sa_element(mem, scanner, BOX_VAL, scanner.cur_val) {
        element.set_sa_ptr(mem, q);
        element.set_sa_ref(mem, element.sa_ref(mem) + 1);
        crate::save::delete_sa_ref(mem, scanner, element.idx());
    }
    Ok(())
}

fn finish_vsplit_after_break(
    mem: &mut MemState,
    eqtb: &mut EqtbState,
    scanner: &mut ScannerState,
    shipout: &mut ShipoutState,
    state: &VsplitState,
    q: MemPtr,
) -> Result<VsplitOutcome, NodeRecycleAbort> {
    let mut packages = [None, None];
    let p = split_vlist_at_break(mem, scanner, state, q)?;
    let save_discards = eqtb.int_par(INT_PAR_SAVING_VDISCARDS) > 0;
    let mut q = prune_page_top(mem, scanner, eqtb, q.0, save_discards)?;

    crate::mem::free_node(mem, state.v.ptr(), BOX_NODE_SIZE);
    if !q.is_null() {
        let packed = vpackage_in(mem, eqtb, scanner, shipout, q, 0, ADDITIONAL, MAX_HALFWORD)?;
        q = packed.r_box.ptr();
        push_slot(&mut packages, packed);
    }
    store_split_remainder(mem, scanner, eqtb, q)?;

    let packed = vpackage_in(
        mem,
        eqtb,
        scanner,
        shipout,
        p,
        state.h,
        EXACTLY,
        state.split_max_d,
    )?;
    let result = Some(packed.r_box);
    push_slot(&mut packages, packed);

    Ok(VsplitOutcome { result, packages })
}

fn vsplit_resume(
    mem: &mut MemState,
    eqtb: &mut EqtbState,
    scanner: &mut ScannerState,
    shipout: &mut ShipoutState,
    state: &mut VsplitState,
) -> Result<VsplitStep, NodeRecycleAbort> {
    match vert_break_resume(
        mem,
        scanner,
        &mut state.vert_break,
        state.h,
        state.split_max_d,
    )? {
        VertBreakStep::Done(q) => {
            let outcome = finish_vsplit_after_break(mem, eqtb, scanner, shipout, state, q)?;
            Ok(VsplitStep::Done(outcome))
        }
        VertBreakStep::InfiniteShrinkage { glue, spec } => {
            Ok(VsplitStep::InfiniteShrinkage { glue, spec })
        }
    }
}

fn print_vsplit_needs_vbox_error_in(st: &mut EngineState) -> ErrorOutcome {
    crate::output::error_here_with_diagnostic_in(st, c"".as_ptr());
    crate::output::print_esc_cstr_in(st, c"vsplit".as_ptr());
    crate::output::print_cstr_in(st, c" needs a ".as_ptr());
    crate::output::print_esc_cstr_in(st, c"vbox".as_ptr());
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help(&[
        r"The box you are trying to split is an \hbox.",
        "I can't split such a box, so I'll leave it alone.",
    ]);
    crate::errors::error_in(st)
}

fn print_vsplit_packages_in(st: &mut EngineState, outcome: VsplitOutcome) {
    for package in outcome.packages.into_iter().flatten() {
        for diag in package.diags.into_iter().flatten() {
            print_vpack_diag_in(st, package.r_box, diag);
        }
    }
}

pub(crate) enum VsplitAbort {
    Node(NodeRecycleAbort),
    Error(ErrorOutcome),
}

impl From<NodeRecycleAbort> for VsplitAbort {
    fn from(e: NodeRecycleAbort) -> Self {
        VsplitAbort::Node(e)
    }
}

impl VsplitAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            VsplitAbort::Node(e) => e.into_engine_abort(),
            VsplitAbort::Error(o) => o.into_engine_abort(),
        }
    }
}

/// `\vsplit n to h`: the box split off the top of box register `n`.
pub(crate) fn vsplit_in(
    st: &mut EngineState,
    n: i32,
    h: i32,
) -> Result<Option<AnyBox>, VsplitAbort> {
    let mut state = match start_vsplit(&mut st.mem, &st.eqtb, &mut st.scanner, n, h)? {
        VsplitStart::Done(outcome) => {
            print_vsplit_packages_in(st, outcome);
            return Ok(outcome.result);
        }
        VsplitStart::NeedsVboxDiagnostic => {
            let outcome = print_vsplit_needs_vbox_error_in(st);
            if !matches!(outcome, ErrorOutcome::Continue) {
                return Err(VsplitAbort::Error(outcome));
            }
            return Ok(None);
        }
        VsplitStart::Ready(state) => state,
    };

    loop {
        match vsplit_resume(
            &mut st.mem,
            &mut st.eqtb,
            &mut st.scanner,
            &mut st.shipout,
            &mut state,
        )? {
            VsplitStep::Done(outcome) => {
                print_vsplit_packages_in(st, outcome);
                return Ok(outcome.result);
            }
            VsplitStep::InfiniteShrinkage { glue, spec } => {
                let outcome = print_vert_break_infinite_shrinkage_error_in(st);
                if !matches!(outcome, ErrorOutcome::Continue) {
                    return Err(VsplitAbort::Error(outcome));
                }
                repair_vert_break_infinite_shrinkage(&mut st.mem, glue, spec)?;
            }
        }
    }
}

/// Insert `\splittopskip` glue between `prev_p` and the box or rule `p` of `height`.
fn insert_split_top_skip_before(
    mem: &mut MemState,
    eqtb: &EqtbState,
    prev_p: MemPtr,
    p: MemPtr,
    height: i32,
) -> Result<(), NodeRecycleAbort> {
    let q = crate::mem::new_skip_param(mem, eqtb, GLUE_PAR_SPLIT_TOP_SKIP)
        .ok_or(NodeRecycleAbort::Overflow)?;
    prev_p.set_link(mem, q.ptr());
    q.set_link(mem, p);
    let spec = q.glue_ptr(mem);
    let avail_ht = spec.width(mem).0;
    if avail_ht > height {
        spec.set_width(mem, Scaled(avail_ht - height));
    } else {
        spec.set_width(mem, Scaled(0));
    }
    Ok(())
}

/// Unlink the discardable `node` after `prev_p`, saving it after `r` (or
/// flushing it), and return its successor.
fn prune_discard_node(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    prev_p: MemPtr,
    r: &mut MemPtr,
    node: MemPtr,
    save_discards: bool,
) -> Result<MemPtr, NodeRecycleAbort> {
    let q = node.link(mem);
    node.set_link(mem, MemPtr::NULL);
    prev_p.set_link(mem, q);
    if save_discards {
        if scanner.disc_ptr[VSPLIT_CODE as usize].is_null() {
            scanner.disc_ptr[VSPLIT_CODE as usize] = node;
        } else {
            r.set_link(mem, node);
        }
        *r = node;
    } else {
        flush_node_list(mem, node).map_err(NodeRecycleAbort::Confusion)?;
    }
    Ok(q)
}

/// Remove the discardable items from the top of the vlist `p`, putting
/// `\splittopskip` before its first box or rule.
pub fn prune_page_top(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    eqtb: &EqtbState,
    p: i32,
    s: bool,
) -> Result<MemPtr, NodeRecycleAbort> {
    let mut p = MemPtr(p);
    let mut prev_p = TEMP_HEAD;
    let mut r = MemPtr::NULL;
    TEMP_HEAD.set_link(mem, p);

    while !p.is_null() {
        let node = mem
            .classify(MemIdx::new(p))
            .map_err(|_| NodeRecycleAbort::Confusion(c"pruning"))?;
        let height = match node {
            AnyNode::HList(b) => b.height(mem).0,
            AnyNode::VList(b) => b.height(mem).0,
            AnyNode::Rule(rule) => rule.height(mem).raw(),
            AnyNode::Whatsit(_) | AnyNode::Mark(_) | AnyNode::Ins(_) => {
                prev_p = p;
                p = prev_p.link(mem);
                continue;
            }
            AnyNode::Glue(_) | AnyNode::Kern(_) | AnyNode::Penalty(_) => {
                p = prune_discard_node(mem, scanner, prev_p, &mut r, p, s)?;
                continue;
            }
            _ => return Err(NodeRecycleAbort::Confusion(c"pruning")),
        };
        insert_split_top_skip_before(mem, eqtb, prev_p, p, height)?;
        p = MemPtr::NULL;
    }

    Ok(TEMP_HEAD.link(mem))
}

/// Apply mark action `a` to the sparse-array mark tree `q` at level `l`.
/// Returns whether the record at `q` was freed.
pub fn do_marks(mem: &mut MemState, scanner: &mut ScannerState, a: i16, l: i16, q: MemIdx) -> bool {
    let node = NodePtr::<SaNode>::at(q);

    if l < 4 {
        for i in 0..=15 {
            scanner.cur_ptr = node.child(mem, i);
            if let Some(cur) = scanner.cur_ptr.as_idx() {
                if do_marks(mem, scanner, a, l + 1, cur) {
                    node.set_child(mem, i, MemPtr::NULL);
                    node.set_ref_count(mem, node.ref_count(mem) - 1);
                }
            }
        }

        if node.ref_count(mem) == 0 {
            crate::mem::free_node(mem, MemPtr::from_idx(q), INDEX_NODE_SIZE);
            return true;
        }
        return false;
    }

    // Mark-class slots by mark code: 0 top, 1 first, 2 bot, 3 split first, 4 split bot.
    match a {
        VSPLIT_INIT => {
            if !node.child(mem, 3).is_null() {
                crate::mem::delete_token_ref(mem, node.child(mem, 3));
                node.set_child(mem, 3, MemPtr::NULL);
                crate::mem::delete_token_ref(mem, node.child(mem, 4));
                node.set_child(mem, 4, MemPtr::NULL);
            }
        }
        FIRE_UP_INIT => {
            if !node.child(mem, 2).is_null() {
                if !node.child(mem, 0).is_null() {
                    crate::mem::delete_token_ref(mem, node.child(mem, 0));
                }
                crate::mem::delete_token_ref(mem, node.child(mem, 1));
                node.set_child(mem, 1, MemPtr::NULL);
                let bot = node.child(mem, 2);
                if bot.link(mem).is_null() {
                    crate::mem::delete_token_ref(mem, bot);
                    node.set_child(mem, 2, MemPtr::NULL);
                } else {
                    add_token_ref(mem, bot);
                }
                node.set_child(mem, 0, node.child(mem, 2));
            }
        }
        FIRE_UP_DONE => {
            let top = node.child(mem, 0);
            if !top.is_null() && node.child(mem, 1).is_null() {
                node.set_child(mem, 1, top);
                add_token_ref(mem, top);
            }
        }
        DESTROY_MARKS => {
            for i in 0..=4 {
                scanner.cur_ptr = node.child(mem, i);
                if !scanner.cur_ptr.is_null() {
                    crate::mem::delete_token_ref(mem, scanner.cur_ptr);
                    node.set_child(mem, i, MemPtr::NULL);
                }
            }
        }
        _ => {}
    }

    if node.child(mem, 2).is_null() && node.child(mem, 4).is_null() {
        crate::mem::free_node(mem, MemPtr::from_idx(q), MARK_CLASS_NODE_SIZE);
        return true;
    }
    false
}

#[derive(Copy, Clone)]
pub(crate) enum PackDiag {
    HUnderfull { badness: i32, loose: bool },
    HOverfull { excess: i32 },
    HTight { badness: i32 },
    LrProblem { missing: i32, extra: i32 },
    VUnderfull { badness: i32, loose: bool },
    VOverfull { excess: i32 },
    VTight { badness: i32 },
}

#[derive(Copy, Clone)]
struct PackOutcome {
    r_box: AnyBox,
    diags: [Option<PackDiag>; 2],
}

/// The highest glue order with a nonzero total.
#[inline]
pub(crate) fn glue_order(totals: &[i32; 4]) -> u16 {
    if totals[FILLL as usize] != 0 {
        FILLL
    } else if totals[FILL as usize] != 0 {
        FILL
    } else if totals[FIL as usize] != 0 {
        FIL
    } else {
        NORMAL
    }
}

/// Set `r`'s glue to distribute `excess` over the highest-order `totals`,
/// returning that order.
fn set_box_glue<K: IsBox>(
    mem: &mut MemState,
    r: NodePtr<K>,
    sign: GlueSign,
    excess: f64,
    totals: &[i32; 4],
) -> u16 {
    let o = glue_order(totals);
    r.set_glue_order(mem, o);
    r.set_glue_sign(mem, sign);
    if totals[o as usize] != 0 {
        r.set_glue_set(mem, excess / totals[o as usize] as f64);
    } else {
        r.set_glue_sign(mem, GlueSign::Normal);
        r.set_glue_set(mem, 0.0);
    }
    o
}

fn clear_box_glue<K: IsBox>(mem: &mut MemState, r: NodePtr<K>) {
    r.set_glue_sign(mem, GlueSign::Normal);
    r.set_glue_order(mem, NORMAL);
    r.set_glue_set(mem, 0.0);
}

/// Merge the run of native words in `w`'s font that follows `w` (possibly
/// across empty discretionaries) into one node. `q` trails `w`; returns the
/// new `q` and the node now in `w`'s place.
fn merge_native_words(
    mem: &mut MemState,
    eqtb: &EqtbState,
    fonts: &FontState,
    r_node: NodePtr<HListNode>,
    mut q: MemPtr,
    w: NodePtr<WhatsitNode>,
) -> Result<(MemPtr, NodePtr<WhatsitNode>), NodeRecycleAbort> {
    let p = w.ptr();
    let font = w.native_font(mem).0;
    let same_font = |mem: &MemState, p: MemPtr| {
        mem.as_native_word(p)
            .filter(|w| w.native_font(mem).0 == font)
    };

    let mut k = if q != r_node.ptr() + LIST_OFFSET {
        as_disc(mem, q).map_or(0, |disc| disc.subtype(mem) as i32)
    } else {
        0
    };
    while q.link(mem) != p {
        k -= 1;
        q = q.link(mem);
        if let Some(disc) = as_disc(mem, q) {
            k = disc.subtype(mem) as i32;
        }
    }

    let mut pp = p.link(mem);
    while k <= 0 && !pp.is_null() && !is_char_node(mem, pp) {
        if let Some(next) = same_font(mem, pp) {
            pp = next.link(mem);
        } else if let Some(next) = as_disc(mem, pp).and_then(|disc| same_font(mem, disc.link(mem)))
        {
            pp = next.link(mem);
        } else {
            break;
        }
    }
    if pp == p.link(mem) {
        return Ok((q, w));
    }

    let mut total_chars: i32 = 0;
    let mut s = p;
    while s != pp {
        if let Some(sw) = mem.as_native_word(s) {
            total_chars += sw.native_length(mem) as i32;
        }
        s = s.link(mem);
    }

    let merged = new_native_word_node(mem, eqtb, font, total_chars)?;
    merged.set_subtype(mem, w.subtype(mem));
    q.set_link(mem, merged.ptr());
    let last = predecessor(mem, p, pp);
    merged.set_link(mem, last.link(mem));
    last.set_link(mem, MemPtr::NULL);

    total_chars = 0;
    let mut s = p;
    loop {
        if let Some(sw) = mem.as_native_word(s) {
            let count = sw.native_length(mem) as usize;
            let src = sw.native_text_ptr(mem);
            let dst = merged.native_text_ptr_mut(mem);
            // SAFETY: `merged` holds the fragments' total length, the fragments are
            // distinct nodes, and the arena is not reallocated here.
            unsafe { std::ptr::copy_nonoverlapping(src, dst.add(total_chars as usize), count) };
            total_chars += count as i32;
        }
        if s.link(mem).is_null() {
            break;
        }
        s = s.link(mem);
    }
    flush_node_list(mem, p).map_err(NodeRecycleAbort::Confusion)?;
    crate::linebreak::set_native_metrics(mem, eqtb, fonts, merged)?;
    Ok((q, merged))
}

#[allow(clippy::too_many_arguments)]
fn hpack_in(
    mem: &mut MemState,
    eqtb: &EqtbState,
    fonts: &mut FontState,
    scanner: &mut ScannerState,
    shipout: &mut ShipoutState,
    list: MemPtr,
    w: i32,
    m: i16,
) -> Result<PackOutcome, NodeRecycleAbort> {
    shipout.last_badness = 0;
    let r_node = NodePtr::<HListNode>::alloc(mem).ok_or(NodeRecycleAbort::Overflow)?;
    r_node.set_subtype(mem, 0);
    r_node.set_shift_amount(mem, Scaled(0));
    r_node.set_list_ptr(mem, list);
    // `q` trails `p`, starting at the word whose link is the list pointer.
    let mut q = r_node.ptr() + LIST_OFFSET;
    let mut h: i32 = 0;
    let mut d: i32 = 0;
    let mut x: i32 = 0;
    scanner.total_stretch = [0; 4];
    scanner.total_shrink = [0; 4];
    let mut diags = [None, None];
    let texxet = eqtb.int_par(INT_PAR_TEXXET) > 0;

    if texxet {
        push_lr(mem, &mut scanner.lr_ptr, BEFORE.into())
            .ok_or(NodeRecycleAbort::OverflowRunaway)?;
    }

    let mut p = list;
    'outer: while !p.is_null() {
        while let Some(ch) = mem.as_char(p) {
            let f = ch.font(mem).0;
            let c = effective_char(fonts, f, ch.character(mem));
            let info = fonts.font_info_b16(fonts.char_base_entry(f) + c);
            x += fonts.font_info_b32s1(fonts.width_base_entry(f) + info.s3 as i32);
            h = h.max(fonts.font_info_b32s1(fonts.height_base_entry(f) + info.s2 as i32 / 16));
            d = d.max(fonts.font_info_b32s1(fonts.depth_base_entry(f) + info.s2 as i32 % 16));
            p = p.link(mem);
        }

        if p.is_null() {
            break;
        }

        match mem.classify(MemIdx::new(p)) {
            Ok(AnyNode::HList(b)) => {
                let s = b.shift_amount(mem).0;
                x += b.width(mem).0;
                h = h.max(b.height(mem).0 - s);
                d = d.max(b.depth(mem).0 + s);
            }
            Ok(AnyNode::VList(b)) => {
                let s = b.shift_amount(mem).0;
                x += b.width(mem).0;
                h = h.max(b.height(mem).0 - s);
                d = d.max(b.depth(mem).0 + s);
            }
            Ok(AnyNode::Rule(rule)) => {
                x += rule.width(mem).raw();
                h = h.max(rule.height(mem).raw());
                d = d.max(rule.depth(mem).raw());
            }
            Ok(AnyNode::Unset(u)) => {
                x += u.width(mem).0;
                h = h.max(u.height(mem).0);
                d = d.max(u.depth(mem).0);
            }
            Ok(AnyNode::Adjust(adjust)) => {
                if !scanner.adjust_tail.is_null() || !scanner.pre_adjust_tail.is_null() {
                    q = predecessor(mem, q, p);
                    let adjust_list = adjust.adjust_ptr(mem);
                    let tail = if adjust.subtype(mem) != 0 {
                        &mut scanner.pre_adjust_tail
                    } else {
                        &mut scanner.adjust_tail
                    };
                    if tail.is_null() {
                        return Err(NodeRecycleAbort::Confusion(c"pre vadjust"));
                    }
                    tail.set_link(mem, adjust_list);
                    *tail = list_tail(mem, *tail);
                    p = adjust.link(mem);
                    crate::mem::free_node(mem, adjust.ptr(), SMALL_NODE_SIZE);
                    q.set_link(mem, p);
                    p = q;
                }
            }
            Ok(AnyNode::Ins(_) | AnyNode::Mark(_)) => {
                if !scanner.adjust_tail.is_null() || !scanner.pre_adjust_tail.is_null() {
                    q = predecessor(mem, q, p);
                    scanner.adjust_tail.set_link(mem, p);
                    scanner.adjust_tail = p;
                    p = p.link(mem);
                    q.set_link(mem, p);
                    p = q;
                }
            }
            Ok(AnyNode::Whatsit(w)) => match w.whatsit_subtype(mem) {
                Some(WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt) => {
                    let (new_q, native) = merge_native_words(mem, eqtb, fonts, r_node, q, w)?;
                    q = new_q;
                    p = native.ptr();
                    h = h.max(native.height(mem).0);
                    d = d.max(native.depth(mem).0);
                    x += native.width(mem).0;
                }
                Some(WhatsitSubtype::Glyph | WhatsitSubtype::Pic | WhatsitSubtype::Pdf) => {
                    h = h.max(w.height(mem).0);
                    d = d.max(w.depth(mem).0);
                    x += w.width(mem).0;
                }
                _ => {}
            },
            Ok(AnyNode::Glue(glue)) => {
                let spec = glue.glue_ptr(mem);
                x += spec.width(mem).0;
                scanner.total_stretch[spec.stretch_order(mem) as usize] += spec.stretch(mem).0;
                scanner.total_shrink[spec.shrink_order(mem) as usize] += spec.shrink(mem).0;
                if glue.subtype(mem) >= A_LEADERS {
                    let leader = glue.leader_ptr(mem);
                    let (leader_h, leader_d) = match mem.classify(MemIdx::new(leader)) {
                        Ok(AnyNode::HList(b)) => (b.height(mem).0, b.depth(mem).0),
                        Ok(AnyNode::VList(b)) => (b.height(mem).0, b.depth(mem).0),
                        Ok(AnyNode::Rule(rule)) => (rule.height(mem).raw(), rule.depth(mem).raw()),
                        _ => (mem.b32s1(leader + 3), mem.b32s1(leader + 2)),
                    };
                    h = h.max(leader_h);
                    d = d.max(leader_d);
                }
            }
            Ok(AnyNode::Kern(kern)) => {
                x += kern.width(mem).0;
            }
            Ok(AnyNode::Math(math)) => {
                x += math.width(mem).0;
                if texxet {
                    let subtype = math.subtype(mem);
                    if subtype % 2 == 1 {
                        if scanner.lr_ptr.info(mem) == end_lr_type(subtype) {
                            pop_lr(mem, &mut scanner.lr_ptr);
                        } else {
                            scanner.lr_problems += 1;
                            math.into_kern(mem).set_subtype(mem, EXPLICIT);
                        }
                    } else {
                        push_lr(mem, &mut scanner.lr_ptr, end_lr_type(subtype))
                            .ok_or(NodeRecycleAbort::OverflowRunaway)?;
                    }
                }
            }
            Ok(AnyNode::Ligature(lig)) => {
                // Measure the ligature as its character through the scratch char node.
                let scratch = NodePtr::<CharNode>::at(GARBAGE);
                scratch.set_font(mem, lig.font(mem));
                scratch.set_character(mem, lig.character(mem));
                GARBAGE.set_link(mem, lig.link(mem));
                p = GARBAGE;
                fonts.xtx_ligature_present = true;
                continue 'outer;
            }
            _ => {}
        }
        p = p.link(mem);
    }

    if !scanner.adjust_tail.is_null() {
        scanner.adjust_tail.set_link(mem, MemPtr::NULL);
    }
    if !scanner.pre_adjust_tail.is_null() {
        scanner.pre_adjust_tail.set_link(mem, MemPtr::NULL);
    }
    r_node.set_height(mem, Scaled(h));
    r_node.set_depth(mem, Scaled(d));

    let w = if m == ADDITIONAL { x + w } else { w };
    r_node.set_width(mem, Scaled(w));
    x = w - x;

    let has_list = !r_node.list_ptr(mem).is_null();
    if x == 0 {
        clear_box_glue(mem, r_node);
    } else if x > 0 {
        let o = set_box_glue(
            mem,
            r_node,
            GlueSign::Stretching,
            x as f64,
            &scanner.total_stretch,
        );
        if o == NORMAL && has_list {
            shipout.last_badness =
                crate::helpers::badness(x, scanner.total_stretch[NORMAL as usize]);
            if shipout.last_badness > eqtb.int_par(INT_PAR_HBADNESS) {
                push_slot(
                    &mut diags,
                    PackDiag::HUnderfull {
                        badness: shipout.last_badness,
                        loose: shipout.last_badness <= 100,
                    },
                );
            }
        }
    } else {
        let o = set_box_glue(
            mem,
            r_node,
            GlueSign::Shrinking,
            (-x) as f64,
            &scanner.total_shrink,
        );
        if scanner.total_shrink[o as usize] < (-x) && o == NORMAL && has_list {
            shipout.last_badness = 1_000_000;
            r_node.set_glue_set(mem, 1.0);
            let excess = (-x) - scanner.total_shrink[NORMAL as usize];
            let hfuzz = eqtb.dimen_par(DIMEN_PAR_HFUZZ);
            if excess > hfuzz || eqtb.int_par(INT_PAR_HBADNESS) < 100 {
                let overfull_rule = eqtb.dimen_par(DIMEN_PAR_OVERFULL_RULE);
                if overfull_rule > 0 && excess > hfuzz {
                    q = list_tail(mem, q);
                    let rule = crate::mem::new_rule(mem).ok_or(NodeRecycleAbort::Overflow)?;
                    q.set_link(mem, rule.ptr());
                    rule.set_width(mem, RuleDimen::Set(Scaled(overfull_rule)));
                }
                push_slot(&mut diags, PackDiag::HOverfull { excess });
            }
        } else if o == NORMAL && has_list {
            shipout.last_badness =
                crate::helpers::badness(-x, scanner.total_shrink[NORMAL as usize]);
            if shipout.last_badness > eqtb.int_par(INT_PAR_HBADNESS) {
                push_slot(
                    &mut diags,
                    PackDiag::HTight {
                        badness: shipout.last_badness,
                    },
                );
            }
        }
    }

    if texxet {
        if scanner.lr_ptr.info(mem) != BEFORE.into() {
            q = list_tail(mem, q);
            loop {
                let math = crate::mem::new_math(mem, 0, scanner.lr_ptr.info(mem) as u16)
                    .ok_or(NodeRecycleAbort::Overflow)?;
                q.set_link(mem, math.ptr());
                q = math.ptr();
                scanner.lr_problems += 10000;
                pop_lr(mem, &mut scanner.lr_ptr);
                if scanner.lr_ptr.info(mem) == BEFORE.into() {
                    break;
                }
            }
        }
        if scanner.lr_problems > 0 {
            push_slot(
                &mut diags,
                PackDiag::LrProblem {
                    missing: scanner.lr_problems / 10000,
                    extra: scanner.lr_problems % 10000,
                },
            );
            scanner.lr_problems = 0;
        }
        pop_lr(mem, &mut scanner.lr_ptr);
        if !scanner.lr_ptr.is_null() {
            return Err(NodeRecycleAbort::Confusion(c"LR1"));
        }
    }

    Ok(PackOutcome {
        r_box: AnyBox::HList(r_node),
        diags,
    })
}

pub(crate) fn print_hpack_diag_in(st: &mut EngineState, box_ptr: AnyBox, diag: PackDiag) {
    use crate::output::{
        diagnostic_begin_capture_warning_here_in, print_cstr_in, print_int_in, print_ln_in,
        print_nl_cstr_in, print_scaled_in,
    };
    print_ln_in(st);
    diagnostic_begin_capture_warning_here_in(st);
    match diag {
        PackDiag::HUnderfull { badness, loose } => {
            if loose {
                print_nl_cstr_in(st, c"Loose".as_ptr());
            } else {
                print_nl_cstr_in(st, c"Underfull".as_ptr());
            }
            print_cstr_in(st, c" \\hbox (badness ".as_ptr());
            print_int_in(st, badness);
        }
        PackDiag::HOverfull { excess } => {
            print_nl_cstr_in(st, c"Overfull \\hbox (".as_ptr());
            print_scaled_in(st, excess);
            print_cstr_in(st, c"pt too wide".as_ptr());
        }
        PackDiag::HTight { badness } => {
            print_nl_cstr_in(st, c"Tight \\hbox (badness ".as_ptr());
            print_int_in(st, badness);
        }
        PackDiag::LrProblem { missing, extra } => {
            print_nl_cstr_in(st, c"\\endL or \\endR problem (".as_ptr());
            print_int_in(st, missing);
            print_cstr_in(st, c" missing, ".as_ptr());
            print_int_in(st, extra);
            print_cstr_in(st, c" extra".as_ptr());
        }
        PackDiag::VUnderfull { .. } | PackDiag::VOverfull { .. } | PackDiag::VTight { .. } => {
            unreachable!("vpack diagnostic passed to hpack printer")
        }
    }
    print_hpack_common_ending_in(st, box_ptr);
}

fn print_hpack_common_ending_in(st: &mut EngineState, box_ptr: AnyBox) {
    use crate::output::{
        begin_diagnostic_in, capture_to_diagnostic_in, end_diagnostic_in, print_cstr_in,
        print_int_in, print_ln_in, short_display_in, show_box_in,
    };
    if st.shipout.output_active {
        print_cstr_in(st, c") has occurred while \\output is active".as_ptr());
    } else {
        let pack_begin_line = st.scanner.pack_begin_line;
        if pack_begin_line != 0 {
            if pack_begin_line > 0 {
                print_cstr_in(st, c") in paragraph at lines ".as_ptr());
            } else {
                print_cstr_in(st, c") in alignment at lines ".as_ptr());
            }
            print_int_in(st, pack_begin_line.abs());
            print_cstr_in(st, c"--".as_ptr());
        } else {
            print_cstr_in(st, c") detected at line ".as_ptr());
        }
        print_int_in(st, st.mem.line);
    }
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    print_ln_in(st);
    st.errors.font_in_short_display = 0;
    let list = box_ptr.list_ptr(&st.mem).0;
    short_display_in(st, list);
    print_ln_in(st);
    begin_diagnostic_in(st);
    show_box_in(st, box_ptr.idx().0);
    end_diagnostic_in(st, true);
}

/// `hpack`, printing its over/underfull diagnostics in order.
pub(crate) fn hpack_emit_in(
    st: &mut EngineState,
    list: MemPtr,
    w: i32,
    m: i16,
) -> Result<AnyBox, NodeRecycleAbort> {
    let outcome = hpack_in(
        &mut st.mem,
        &st.eqtb,
        &mut st.fonts,
        &mut st.scanner,
        &mut st.shipout,
        list,
        w,
        m,
    )?;
    for diag in outcome.diags.into_iter().flatten() {
        print_hpack_diag_in(st, outcome.r_box, diag);
    }
    Ok(outcome.r_box)
}

#[allow(clippy::too_many_arguments)]
fn vpackage_in(
    mem: &mut MemState,
    eqtb: &EqtbState,
    scanner: &mut ScannerState,
    shipout: &mut ShipoutState,
    list: MemPtr,
    h: i32,
    m: i16,
    l: i32,
) -> Result<PackOutcome, NodeRecycleAbort> {
    shipout.last_badness = 0;
    let r_node = NodePtr::<VListNode>::alloc(mem).ok_or(NodeRecycleAbort::Overflow)?;
    let upwards = eqtb.int_par(INT_PAR_XETEX_UPWARDS) > 0;
    r_node.set_subtype(mem, u16::from(upwards));
    r_node.set_shift_amount(mem, Scaled(0));
    r_node.set_list_ptr(mem, list);

    let mut w: i32 = 0;
    let mut d: i32 = 0;
    let mut x: i32 = 0;
    scanner.total_stretch = [0; 4];
    scanner.total_shrink = [0; 4];
    let mut diags = [None, None];

    let mut p = list;
    while !p.is_null() {
        if is_char_node(mem, p) {
            return Err(NodeRecycleAbort::Confusion(c"vpack"));
        }
        match mem.classify(MemIdx::new(p)) {
            Ok(AnyNode::HList(b)) => {
                x += d + b.height(mem).0;
                d = b.depth(mem).0;
                w = w.max(b.width(mem).0 + b.shift_amount(mem).0);
            }
            Ok(AnyNode::VList(b)) => {
                x += d + b.height(mem).0;
                d = b.depth(mem).0;
                w = w.max(b.width(mem).0 + b.shift_amount(mem).0);
            }
            Ok(AnyNode::Rule(rule)) => {
                x += d + rule.height(mem).raw();
                d = rule.depth(mem).raw();
                w = w.max(rule.width(mem).raw());
            }
            Ok(AnyNode::Unset(u)) => {
                x += d + u.height(mem).0;
                d = u.depth(mem).0;
                w = w.max(u.width(mem).0);
            }
            Ok(AnyNode::Whatsit(wn)) => {
                if matches!(
                    wn.whatsit_subtype(mem),
                    Some(WhatsitSubtype::Pic | WhatsitSubtype::Pdf)
                ) {
                    x += d + wn.height(mem).0;
                    d = wn.depth(mem).0;
                    w = w.max(wn.width(mem).0);
                }
            }
            Ok(AnyNode::Glue(glue)) => {
                x += d;
                d = 0;
                let spec = glue.glue_ptr(mem);
                x += spec.width(mem).0;
                scanner.total_stretch[spec.stretch_order(mem) as usize] += spec.stretch(mem).0;
                scanner.total_shrink[spec.shrink_order(mem) as usize] += spec.shrink(mem).0;
                if glue.subtype(mem) >= A_LEADERS {
                    let leader = glue.leader_ptr(mem);
                    let leader_w = match mem.classify(MemIdx::new(leader)) {
                        Ok(AnyNode::HList(b)) => b.width(mem).0,
                        Ok(AnyNode::VList(b)) => b.width(mem).0,
                        Ok(AnyNode::Rule(rule)) => rule.width(mem).raw(),
                        _ => mem.b32s1(leader + 1),
                    };
                    w = w.max(leader_w);
                }
            }
            Ok(AnyNode::Kern(kern)) => {
                x += d + kern.width(mem).0;
                d = 0;
            }
            _ => {}
        }
        p = p.link(mem);
    }

    r_node.set_width(mem, Scaled(w));
    if d > l {
        x += d - l;
        r_node.set_depth(mem, Scaled(l));
    } else {
        r_node.set_depth(mem, Scaled(d));
    }

    let h = if m == ADDITIONAL { x + h } else { h };
    r_node.set_height(mem, Scaled(h));
    x = h - x;

    let has_list = !r_node.list_ptr(mem).is_null();
    if x == 0 {
        clear_box_glue(mem, r_node);
    } else if x > 0 {
        let o = set_box_glue(
            mem,
            r_node,
            GlueSign::Stretching,
            x as f64,
            &scanner.total_stretch,
        );
        if o == NORMAL && has_list {
            shipout.last_badness =
                crate::helpers::badness(x, scanner.total_stretch[NORMAL as usize]);
            if shipout.last_badness > eqtb.int_par(INT_PAR_VBADNESS) {
                push_slot(
                    &mut diags,
                    PackDiag::VUnderfull {
                        badness: shipout.last_badness,
                        loose: shipout.last_badness <= 100,
                    },
                );
            }
        }
    } else {
        let o = set_box_glue(
            mem,
            r_node,
            GlueSign::Shrinking,
            (-(x as i64)) as f64,
            &scanner.total_shrink,
        );
        if scanner.total_shrink[o as usize] < (-x) && o == NORMAL && has_list {
            shipout.last_badness = 1_000_000;
            r_node.set_glue_set(mem, 1.0);
            let excess = (-x) - scanner.total_shrink[NORMAL as usize];
            if excess > eqtb.dimen_par(DIMEN_PAR_VFUZZ) || eqtb.int_par(INT_PAR_VBADNESS) < 100 {
                push_slot(&mut diags, PackDiag::VOverfull { excess });
            }
        } else if o == NORMAL && has_list {
            shipout.last_badness =
                crate::helpers::badness(-x, scanner.total_shrink[NORMAL as usize]);
            if shipout.last_badness > eqtb.int_par(INT_PAR_VBADNESS) {
                push_slot(
                    &mut diags,
                    PackDiag::VTight {
                        badness: shipout.last_badness,
                    },
                );
            }
        }
    }

    Ok(PackOutcome {
        r_box: AnyBox::VList(r_node),
        diags,
    })
}

pub(crate) fn print_vpack_diag_in(st: &mut EngineState, box_ptr: AnyBox, diag: PackDiag) {
    use crate::output::{
        diagnostic_begin_capture_warning_here_in, print_cstr_in, print_int_in, print_ln_in,
        print_nl_cstr_in, print_scaled_in,
    };
    print_ln_in(st);
    diagnostic_begin_capture_warning_here_in(st);
    match diag {
        PackDiag::VUnderfull { badness, loose } => {
            if loose {
                print_nl_cstr_in(st, c"Loose".as_ptr());
            } else {
                print_nl_cstr_in(st, c"Underfull".as_ptr());
            }
            print_cstr_in(st, c" \\vbox (badness ".as_ptr());
            print_int_in(st, badness);
        }
        PackDiag::VOverfull { excess } => {
            print_nl_cstr_in(st, c"Overfull \\vbox (".as_ptr());
            print_scaled_in(st, excess);
            print_cstr_in(st, c"pt too high".as_ptr());
        }
        PackDiag::VTight { badness } => {
            print_nl_cstr_in(st, c"Tight \\vbox (badness ".as_ptr());
            print_int_in(st, badness);
        }
        PackDiag::HUnderfull { .. }
        | PackDiag::HOverfull { .. }
        | PackDiag::HTight { .. }
        | PackDiag::LrProblem { .. } => {
            unreachable!("hpack diagnostic passed to vpack printer")
        }
    }
    print_vpack_common_ending_in(st, box_ptr);
}

fn print_vpack_common_ending_in(st: &mut EngineState, box_ptr: AnyBox) {
    use crate::output::{
        begin_diagnostic_in, capture_to_diagnostic_in, end_diagnostic_in, print_cstr_in,
        print_int_in, print_ln_in, show_box_in,
    };
    if st.shipout.output_active {
        print_cstr_in(st, c") has occurred while \\output is active".as_ptr());
    } else {
        let pack_begin_line = st.scanner.pack_begin_line;
        if pack_begin_line != 0 {
            print_cstr_in(st, c") in alignment at lines ".as_ptr());
            print_int_in(st, pack_begin_line.abs());
            print_cstr_in(st, c"--".as_ptr());
        } else {
            print_cstr_in(st, c") detected at line ".as_ptr());
        }
        print_int_in(st, st.mem.line);
        print_ln_in(st);
    }
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    begin_diagnostic_in(st);
    show_box_in(st, box_ptr.idx().0);
    end_diagnostic_in(st, true);
}

/// `vpackage`, printing its over/underfull diagnostics in order.
pub(crate) fn vpackage_emit_in(
    st: &mut EngineState,
    list: MemPtr,
    h: i32,
    m: i16,
    l: i32,
) -> Result<AnyBox, NodeRecycleAbort> {
    let outcome = vpackage_in(
        &mut st.mem,
        &st.eqtb,
        &mut st.scanner,
        &mut st.shipout,
        list,
        h,
        m,
        l,
    )?;
    for diag in outcome.diags.into_iter().flatten() {
        print_vpack_diag_in(st, outcome.r_box, diag);
    }
    Ok(outcome.r_box)
}
