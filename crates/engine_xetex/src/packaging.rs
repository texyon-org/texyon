//! Box packing (`hpack`, `vpackage`), vertical breaking (`vert_break`,
//! `vsplit`, `prune_page_top`, `do_marks`) and node-list copying and freeing.

use crate::consts::{
    ACCENT_NOAD_SIZE, A_LEADERS, BEFORE, BOX_NODE_SIZE, DIMEN_PAR_HFUZZ, DIMEN_PAR_LINE_SKIP_LIMIT,
    DIMEN_PAR_OVERFULL_RULE, DIMEN_PAR_SPLIT_MAX_DEPTH, DIMEN_PAR_VFUZZ, EJECT_PENALTY, EXPLICIT,
    FIL, FILL, FILLL, FRACTION_NOAD_SIZE, GLUE_PAR_BASELINE_SKIP, GLUE_PAR_LINE_SKIP,
    GLUE_PAR_SPLIT_TOP_SKIP, GLYPH_NODE_SIZE, IGNORE_DEPTH, INDEX_NODE_SIZE, INF_BAD, INF_PENALTY,
    INS_NODE_SIZE, INT_PAR_HBADNESS, INT_PAR_SAVING_VDISCARDS, INT_PAR_TEXXET, INT_PAR_VBADNESS,
    INT_PAR_XETEX_UPWARDS, MARGIN_KERN_NODE_SIZE, MARK_CLASS_NODE_SIZE, MAX_HALFWORD,
    MEDIUM_NODE_SIZE, MIN_HALFWORD, NOAD_SIZE, NORMAL, OPEN_NODE_SIZE, PIC_NODE_SIZE,
    RADICAL_NOAD_SIZE, RULE_NODE_SIZE, R_CODE, SMALL_NODE_SIZE, SPLIT_BOT_MARK_CODE,
    SPLIT_FIRST_MARK_CODE, STYLE_NODE_SIZE, SUB_BOX, WRITE_NODE_SIZE,
};
use crate::errors::ErrorOutcome;
use crate::main_control::VSPLIT_CODE;
use crate::mem::{end_lr_type, GARBAGE, TEMP_HEAD};
use crate::node::{
    AnyBox, AnyNode, CharNode, DiscNode, GlueNode, GlueSpecNode, HListNode, IsBox, KernNode,
    MarkNode, NodeKind, NodePtr, RuleDimen, SaNode, VListNode, WhatsitNode, WhatsitSubtype,
};
use crate::pagebuilder::{FIRE_UP_DONE, FIRE_UP_INIT};
use crate::scan::{NodeRecycleAbort, MARK_VAL};
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

impl MemState {
    fn as_disc(&self, p: MemPtr) -> Option<NodePtr<DiscNode>> {
        if self.is_char_node(p) {
            return None;
        }
        self.classify_disc(MemIdx::new(p)).ok().flatten()
    }
}

/// Store `v` in the first empty slot of a two-slot record.
#[inline]
fn push_slot<T>(slots: &mut [Option<T>; 2], v: T) {
    let i = usize::from(slots[0].is_some());
    slots[i] = Some(v);
}

impl MemState {
    /// The size of whatsit `w`, `None` for an unknown subtype.
    fn whatsit_node_size(&self, w: NodePtr<WhatsitNode>) -> Option<i32> {
        use WhatsitSubtype::*;
        Some(match w.whatsit_subtype(self)? {
            Open => OPEN_NODE_SIZE,
            Write | Special => WRITE_NODE_SIZE,
            Close | Language | PdfSavePos => SMALL_NODE_SIZE,
            NativeWord | NativeWordAt => w.native_size(self),
            Glyph => GLYPH_NODE_SIZE,
            Pic | Pdf => {
                PIC_NODE_SIZE
                    + (w.pic_path_len(self) as i32 + MEMORY_WORD_SIZE - 1) / MEMORY_WORD_SIZE
            }
        })
    }

    #[inline]
    fn get_node_or_overflow(&mut self, size: i32) -> Result<MemPtr, NodeRecycleAbort> {
        self.get_node(size).ok_or(NodeRecycleAbort::Overflow)
    }
}

/// `r` typed like `src`, whose header word `r` has just received.
#[inline]
fn copied_node<K: NodeKind>(_src: NodePtr<K>, r: MemPtr) -> NodePtr<K> {
    // SAFETY: `r`'s NODE_type word was copied from `src`'s.
    unsafe { NodePtr::from_idx_unchecked(MemIdx::new(r)) }
}

impl MemState {
    /// Copy the first `words` words of node `src` to node `dst`.
    #[inline]
    fn copy_words(&mut self, dst: MemPtr, src: MemPtr, words: i32) {
        for w in (0..words).rev() {
            self.set_word(dst + w, self.word(src + w));
        }
    }

    fn copy_box_shell(&mut self, src: AnyBox, list: MemPtr) -> Result<AnyBox, NodeRecycleAbort> {
        let r = self.get_node_or_overflow(BOX_NODE_SIZE)?;
        self.copy_words(r, src.ptr(), BOX_NODE_SIZE);
        let r = match src {
            AnyBox::HList(b) => AnyBox::HList(copied_node(b, r)),
            AnyBox::VList(b) => AnyBox::VList(copied_node(b, r)),
        };
        r.set_list_ptr(self, list);
        Ok(r)
    }

    /// A copy of glue node `p` sharing its spec. The leader pointer is left to the caller.
    fn copy_glue_shell(
        &mut self,
        p: NodePtr<GlueNode>,
    ) -> Result<NodePtr<GlueNode>, NodeRecycleAbort> {
        let spec = p.glue_ptr(self);
        let r = self.get_node_or_overflow(MEDIUM_NODE_SIZE)?;
        self.add_glue_ref(spec);
        let sync = MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE;
        self.set_word(r, self.word(p.ptr()));
        self.set_word(r + sync, self.word(p.ptr() + sync));
        let r = copied_node(p, r);
        r.set_glue_ptr(self, spec);
        Ok(r)
    }

    /// Allocate a copy of whatsit `w`, returning it and how many of its leading
    /// words are still to be copied.
    fn copy_whatsit(&mut self, w: NodePtr<WhatsitNode>) -> Result<(MemPtr, i32), NodeRecycleAbort> {
        let Some(size) = self.whatsit_node_size(w) else {
            return Err(NodeRecycleAbort::Confusion(c"ext2"));
        };
        let r = self.get_node_or_overflow(size)?;
        match w.whatsit_subtype(self) {
            Some(WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt) => {
                self.copy_words(r, w.ptr(), size);
                let copy = copied_node(w, r);
                copy.set_native_glyph_info_handle(self, 0);
                copy.set_native_glyph_count(self, 0);
                copy.share_native_glyphs(self, w);
                Ok((r, 0))
            }
            Some(WhatsitSubtype::Write | WhatsitSubtype::Special) => {
                self.add_token_ref(w.write_tokens(self));
                Ok((r, size))
            }
            Some(WhatsitSubtype::PdfSavePos) => Ok((r, 1)),
            _ => Ok((r, size)),
        }
    }

    /// Copy the list `p` without the contents of its boxes, linking the copies
    /// after `h` and ending them with `t`.
    pub fn just_copy(
        &mut self,
        mut p: MemPtr,
        mut h: MemPtr,
        t: MemPtr,
    ) -> Result<(), NodeRecycleAbort> {
        while !p.is_null() {
            let copy = if self.is_char_node(p) {
                let r = self.get_avail().ok_or(NodeRecycleAbort::OverflowRunaway)?;
                Some((r, 1))
            } else {
                match self.classify(MemIdx::new(p)).ok() {
                    Some(AnyNode::HList(b)) => Some((
                        self.copy_box_shell(AnyBox::HList(b), MemPtr::NULL)?.ptr(),
                        0,
                    )),
                    Some(AnyNode::VList(b)) => Some((
                        self.copy_box_shell(AnyBox::VList(b), MemPtr::NULL)?.ptr(),
                        0,
                    )),
                    Some(AnyNode::Rule(_)) => {
                        Some((self.get_node_or_overflow(RULE_NODE_SIZE)?, RULE_NODE_SIZE))
                    }
                    Some(AnyNode::Ligature(lig)) => {
                        let r = self.get_avail().ok_or(NodeRecycleAbort::OverflowRunaway)?;
                        self.set_word(r, self.word(lig.ptr() + 1));
                        Some((r, 0))
                    }
                    Some(AnyNode::Kern(_) | AnyNode::Math(_)) => Some((
                        self.get_node_or_overflow(MEDIUM_NODE_SIZE)?,
                        MEDIUM_NODE_SIZE,
                    )),
                    Some(AnyNode::Glue(g)) => {
                        let r = self.copy_glue_shell(g)?;
                        r.set_leader_ptr(self, MemPtr::NULL);
                        Some((r.ptr(), 0))
                    }
                    Some(AnyNode::Whatsit(w)) => Some(self.copy_whatsit(w)?),
                    _ => None,
                }
            };

            if let Some((r, words)) = copy {
                self.copy_words(r, p, words);
                h.set_link(self, r);
                h = r;
            }
            p = p.link(self);
        }

        h.set_link(self, t);
        Ok(())
    }

    /// Reverse the hlist after `p` for TeX--XeT, leaving the result after `TEMP_HEAD`.
    pub fn just_reverse(
        &mut self,
        scanner: &mut ScannerState,
        p: MemPtr,
    ) -> Result<(), NodeRecycleAbort> {
        let mut m = MIN_HALFWORD;
        let mut n = MIN_HALFWORD;

        let mut q = if TEMP_HEAD.link(self).is_null() {
            self.just_copy(p.link(self), TEMP_HEAD, MemPtr::NULL)?;
            TEMP_HEAD.link(self)
        } else {
            let q = p.link(self);
            p.set_link(self, MemPtr::NULL);
            self.flush_node_list(TEMP_HEAD.link(self))
                .map_err(NodeRecycleAbort::Confusion)?;
            q
        };

        let t = self
            .new_edge(scanner.cur_dir, 0)
            .ok_or(NodeRecycleAbort::Overflow)?;
        let mut l = t.ptr();
        scanner.cur_dir = 1 - scanner.cur_dir;

        'done: {
            while !q.is_null() {
                if self.is_char_node(q) {
                    loop {
                        let p = q;
                        q = p.link(self);
                        p.set_link(self, l);
                        l = p;
                        if !self.is_char_node(q) {
                            break;
                        }
                    }
                    continue;
                }

                let p = q;
                q = p.link(self);

                if let Ok(AnyNode::Math(math)) = self.classify(MemIdx::new(p)) {
                    let subtype = math.subtype(self);
                    if subtype % 2 == 1 {
                        if scanner.lr_ptr.info(self) != end_lr_type(subtype) {
                            math.into_kern(self);
                            scanner.lr_problems += 1;
                        } else {
                            self.pop_lr(&mut scanner.lr_ptr);

                            if n > MIN_HALFWORD {
                                n -= 1;
                                math.set_subtype(self, subtype - 1);
                            } else if m > MIN_HALFWORD {
                                m -= 1;
                                math.into_kern(self);
                            } else {
                                t.set_width(self, math.width(self));
                                t.set_link(self, q);
                                self.free_node(p, MEDIUM_NODE_SIZE);
                                break 'done;
                            }
                        }
                    } else {
                        self.push_lr(&mut scanner.lr_ptr, end_lr_type(subtype))
                            .ok_or(NodeRecycleAbort::OverflowRunaway)?;

                        if n > MIN_HALFWORD || (subtype / R_CODE) as i16 != scanner.cur_dir {
                            n += 1;
                            math.set_subtype(self, subtype + 1);
                        } else {
                            math.into_kern(self);
                            m += 1;
                        }
                    }
                }

                p.set_link(self, l);
                l = p;
            }
        }

        TEMP_HEAD.set_link(self, l);
        Ok(())
    }

    /// Free every node of the list `p` and everything it owns.
    pub fn flush_node_list(
        &mut self,
        p: impl crate::units::MemAddr,
    ) -> Result<(), &'static std::ffi::CStr> {
        let mut p = MemPtr(p.addr());
        while !p.is_null() {
            let q = p.link(self);
            if self.is_char_node(p) {
                self.free_avail(p);
                p = q;
                continue;
            }

            let size = match self.classify(MemIdx::new(p)).map_err(|_| c"flushing")? {
                AnyNode::HList(b) => {
                    self.flush_node_list(b.list_ptr(self))?;
                    BOX_NODE_SIZE
                }
                AnyNode::VList(b) => {
                    self.flush_node_list(b.list_ptr(self))?;
                    BOX_NODE_SIZE
                }
                AnyNode::Unset(u) => {
                    self.flush_node_list(u.list_ptr(self))?;
                    BOX_NODE_SIZE
                }
                AnyNode::Rule(_) => RULE_NODE_SIZE,
                AnyNode::Ins(ins) => {
                    self.flush_node_list(ins.ins_ptr(self))?;
                    self.delete_glue_ref(ins.split_top_ptr(self));
                    INS_NODE_SIZE
                }
                AnyNode::Whatsit(w) => {
                    let size = self.whatsit_node_size(w).ok_or(c"ext3")?;
                    match w.whatsit_subtype(self) {
                        Some(WhatsitSubtype::Write | WhatsitSubtype::Special) => {
                            self.delete_token_ref(w.write_tokens(self));
                        }
                        Some(WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt) => {
                            w.release_native_glyphs(self);
                        }
                        _ => {}
                    }
                    size
                }
                AnyNode::Glue(glue) => {
                    self.delete_glue_ref(glue.glue_ptr(self));
                    self.flush_node_list(glue.leader_ptr(self))?;
                    MEDIUM_NODE_SIZE
                }
                AnyNode::Kern(_) | AnyNode::Math(_) | AnyNode::Penalty(_) => MEDIUM_NODE_SIZE,
                AnyNode::MarginKern(_) => MARGIN_KERN_NODE_SIZE,
                AnyNode::Ligature(lig) => {
                    self.flush_node_list(lig.lig_ptr(self))?;
                    SMALL_NODE_SIZE
                }
                AnyNode::Mark(mark) => {
                    self.delete_token_ref(mark.mark_ptr(self));
                    SMALL_NODE_SIZE
                }
                AnyNode::Disc(disc) => {
                    self.flush_node_list(disc.pre_break(self))?;
                    self.flush_node_list(disc.post_break(self))?;
                    SMALL_NODE_SIZE
                }
                AnyNode::Adjust(adjust) => {
                    self.flush_node_list(adjust.adjust_ptr(self))?;
                    SMALL_NODE_SIZE
                }
                AnyNode::Style(_) => STYLE_NODE_SIZE,
                AnyNode::Choice(choice) => {
                    self.flush_node_list(choice.display_mlist(self))?;
                    self.flush_node_list(choice.text_mlist(self))?;
                    self.flush_node_list(choice.script_mlist(self))?;
                    self.flush_node_list(choice.script_script_mlist(self))?;
                    STYLE_NODE_SIZE
                }
                AnyNode::Noad(noad) => match noad.noad_type(self) {
                    NodeType::LeftNoad | NodeType::RightNoad => NOAD_SIZE,
                    NodeType::FractionNoad => {
                        self.flush_node_list(noad.supscr().info(self))?;
                        self.flush_node_list(noad.subscr().info(self))?;
                        FRACTION_NOAD_SIZE
                    }
                    noad_type => {
                        if noad.nucleus().math_type(self) >= SUB_BOX {
                            self.flush_node_list(noad.nucleus().info(self))?;
                        }
                        if noad.supscr().math_type(self) >= SUB_BOX {
                            self.flush_node_list(noad.supscr().info(self))?;
                        }
                        if noad.subscr().math_type(self) >= SUB_BOX {
                            self.flush_node_list(noad.subscr().info(self))?;
                        }
                        match noad_type {
                            NodeType::RadicalNoad => RADICAL_NOAD_SIZE,
                            NodeType::AccentNoad => ACCENT_NOAD_SIZE,
                            _ => NOAD_SIZE,
                        }
                    }
                },
            };
            self.free_node(p, size);
            p = q;
        }
        Ok(())
    }

    /// A deep copy of the list `p`.
    /// A copy of box `b` and its contents.
    pub(crate) fn copy_box(&mut self, b: AnyBox) -> Result<AnyBox, NodeRecycleAbort> {
        let r = self.copy_list(b.ptr())?;
        Ok(match b {
            AnyBox::HList(h) => AnyBox::HList(copied_node(h, r)),
            AnyBox::VList(v) => AnyBox::VList(copied_node(v, r)),
        })
    }

    pub fn copy_node_list(&mut self, p: i32) -> Result<i32, NodeRecycleAbort> {
        Ok(self.copy_list(MemPtr(p))?.0)
    }

    fn copy_list(&mut self, mut p: MemPtr) -> Result<MemPtr, NodeRecycleAbort> {
        let h = self.get_avail().ok_or(NodeRecycleAbort::OverflowRunaway)?;
        let mut q = h;
        while !p.is_null() {
            let (r, words) = if self.is_char_node(p) {
                (
                    self.get_avail().ok_or(NodeRecycleAbort::OverflowRunaway)?,
                    1,
                )
            } else {
                self.copy_node(p)?
            };
            self.copy_words(r, p, words);
            q.set_link(self, r);
            q = r;
            p = p.link(self);
        }
        q.set_link(self, MemPtr::NULL);
        let result = h.link(self);
        self.free_avail(h);
        Ok(result)
    }

    /// Allocate a copy of the non-char node `p`, returning it and how many of its
    /// leading words are still to be copied.
    fn copy_node(&mut self, p: MemPtr) -> Result<(MemPtr, i32), NodeRecycleAbort> {
        let node = self
            .classify(MemIdx::new(p))
            .map_err(|_| NodeRecycleAbort::Confusion(c"copying"))?;
        Ok(match node {
            AnyNode::HList(b) => {
                let list = self.copy_list(b.list_ptr(self))?;
                (self.copy_box_shell(AnyBox::HList(b), list)?.ptr(), 0)
            }
            AnyNode::VList(b) => {
                let list = self.copy_list(b.list_ptr(self))?;
                (self.copy_box_shell(AnyBox::VList(b), list)?.ptr(), 0)
            }
            AnyNode::Unset(u) => {
                let r = self.get_node_or_overflow(BOX_NODE_SIZE)?;
                self.copy_words(r, p, BOX_NODE_SIZE);
                let list = self.copy_list(u.list_ptr(self))?;
                copied_node(u, r).set_list_ptr(self, list);
                (r, 0)
            }
            AnyNode::Rule(_) => (
                self.get_node_or_overflow(RULE_NODE_SIZE)?,
                RULE_NODE_SIZE - SYNCTEX_FIELD_SIZE,
            ),
            AnyNode::Ins(ins) => {
                let r = self.get_node_or_overflow(INS_NODE_SIZE)?;
                self.copy_words(r, p, INS_NODE_SIZE);
                self.add_glue_ref(ins.split_top_ptr(self));
                let list = self.copy_list(ins.ins_ptr(self))?;
                copied_node(ins, r).set_ins_ptr(self, list);
                (r, 0)
            }
            AnyNode::Whatsit(w) => self.copy_whatsit(w)?,
            AnyNode::Glue(g) => {
                let r = self.copy_glue_shell(g)?;
                let leaders = self.copy_list(g.leader_ptr(self))?;
                r.set_leader_ptr(self, leaders);
                (r.ptr(), 0)
            }
            AnyNode::Kern(_) | AnyNode::Math(_) | AnyNode::Penalty(_) => (
                self.get_node_or_overflow(MEDIUM_NODE_SIZE)?,
                MEDIUM_NODE_SIZE,
            ),
            AnyNode::MarginKern(_) => (
                self.get_node_or_overflow(MARGIN_KERN_NODE_SIZE)?,
                MARGIN_KERN_NODE_SIZE,
            ),
            AnyNode::Ligature(lig) => {
                let r = self.get_node_or_overflow(SMALL_NODE_SIZE)?;
                self.copy_words(r, p, SMALL_NODE_SIZE);
                let components = self.copy_list(lig.lig_ptr(self))?;
                copied_node(lig, r).set_lig_ptr(self, components);
                (r, 0)
            }
            AnyNode::Disc(disc) => {
                let r = self.get_node_or_overflow(SMALL_NODE_SIZE)?;
                self.copy_words(r, p, 1);
                let copy = copied_node(disc, r);
                let pre_break = self.copy_list(disc.pre_break(self))?;
                copy.set_pre_break(self, pre_break);
                let post_break = self.copy_list(disc.post_break(self))?;
                copy.set_post_break(self, post_break);
                (r, 0)
            }
            AnyNode::Mark(mark) => {
                let r = self.get_node_or_overflow(SMALL_NODE_SIZE)?;
                self.add_token_ref(mark.mark_ptr(self));
                (r, SMALL_NODE_SIZE)
            }
            AnyNode::Adjust(adjust) => {
                let r = self.get_node_or_overflow(SMALL_NODE_SIZE)?;
                self.copy_words(r, p, 1);
                let list = self.copy_list(adjust.adjust_ptr(self))?;
                copied_node(adjust, r).set_adjust_ptr(self, list);
                (r, 0)
            }
            _ => return Err(NodeRecycleAbort::Confusion(c"copying")),
        })
    }
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
            mem.new_param_glue(eqtb, GLUE_PAR_LINE_SKIP)
                .ok_or(NodeRecycleAbort::Overflow)?
        } else {
            let p = mem
                .new_skip_param(eqtb, GLUE_PAR_BASELINE_SKIP)
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

impl MemState {
    fn advance_vert_break(&self, scanner: &mut ScannerState, cursor: &mut VertBreakCursor, d: i32) {
        if cursor.prev_dp > d {
            scanner.active_width[1] += cursor.prev_dp - d;
            cursor.prev_dp = d;
        }
        cursor.prev_p = cursor.p;
        cursor.prev_breakable = cursor.breakable;
        cursor.p = cursor.prev_p.link(self);
    }

    fn finish_vert_break_updated_height(
        &self,
        scanner: &mut ScannerState,
        cursor: &mut VertBreakCursor,
        width: i32,
        d: i32,
    ) {
        scanner.active_width[1] += cursor.prev_dp + width;
        cursor.prev_dp = 0;
        self.advance_vert_break(scanner, cursor, d);
    }

    fn classify_vert_break_next(
        &self,
        scanner: &mut ScannerState,
        cursor: &mut VertBreakCursor,
    ) -> Result<VertBreakNext, NodeRecycleAbort> {
        if cursor.p.is_null() {
            return Ok(VertBreakNext::CheckPi {
                pi: EJECT_PENALTY,
                update_after: None,
            });
        }

        let node = self
            .classify(MemIdx::new(cursor.p))
            .map_err(|_| NodeRecycleAbort::Confusion(c"vertbreak"))?;
        cursor.breakable = node.precedes_break();
        match node {
            AnyNode::HList(b) => {
                scanner.active_width[1] += cursor.prev_dp + b.height(self).0;
                cursor.prev_dp = b.depth(self).0;
                Ok(VertBreakNext::NotFound)
            }
            AnyNode::VList(b) => {
                scanner.active_width[1] += cursor.prev_dp + b.height(self).0;
                cursor.prev_dp = b.depth(self).0;
                Ok(VertBreakNext::NotFound)
            }
            AnyNode::Rule(rule) => {
                scanner.active_width[1] += cursor.prev_dp + rule.height(self).raw();
                cursor.prev_dp = rule.depth(self).raw();
                Ok(VertBreakNext::NotFound)
            }
            AnyNode::Whatsit(w) => {
                if matches!(
                    w.whatsit_subtype(self),
                    Some(WhatsitSubtype::Pic | WhatsitSubtype::Pdf)
                ) {
                    scanner.active_width[1] += cursor.prev_dp + w.height(self).0;
                    cursor.prev_dp = w.depth(self).0;
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
                let next = kern.link(self);
                if !next.is_null()
                    && matches!(self.classify(MemIdx::new(next)), Ok(AnyNode::Glue(_)))
                {
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
                pi: penalty.penalty(self),
                update_after: None,
            }),
            AnyNode::Ins(_) | AnyNode::Mark(_) => Ok(VertBreakNext::NotFound),
            _ => Err(NodeRecycleAbort::Confusion(c"vertbreak")),
        }
    }

    fn vert_break_resume(
        &mut self,
        scanner: &mut ScannerState,
        cursor: &mut VertBreakCursor,
        h: i32,
        d: i32,
    ) -> Result<VertBreakStep, NodeRecycleAbort> {
        loop {
            if let Some(glue) = cursor.finish_glue_after_shrinkage.take() {
                let spec = glue.glue_ptr(self);
                self.finish_vert_break_updated_height(scanner, cursor, spec.width(self).0, d);
                continue;
            }

            let update_source = match self.classify_vert_break_next(scanner, cursor)? {
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
                            scanner.best_height_plus_depth =
                                scanner.active_width[1] + cursor.prev_dp;
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
                    self.finish_vert_break_updated_height(scanner, cursor, kern.width(self).0, d);
                }
                Some(VertBreakUpdateSource::Glue(glue)) => {
                    let spec = glue.glue_ptr(self);
                    scanner.active_width[2 + spec.stretch_order(self) as usize] +=
                        spec.stretch(self).0;
                    scanner.active_width[6] += spec.shrink(self).0;
                    if spec.shrink_order(self) != NORMAL && spec.shrink(self).0 != 0 {
                        cursor.finish_glue_after_shrinkage = Some(glue);
                        return Ok(VertBreakStep::InfiniteShrinkage { glue, spec });
                    }
                    self.finish_vert_break_updated_height(scanner, cursor, spec.width(self).0, d);
                }
                None => self.advance_vert_break(scanner, cursor, d),
            }
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

impl MemState {
    fn repair_vert_break_infinite_shrinkage(
        &mut self,
        glue: NodePtr<GlueNode>,
        spec: NodePtr<GlueSpecNode>,
    ) -> Result<(), NodeRecycleAbort> {
        let spec = self.finite_shrink(spec)?;
        glue.set_glue_ptr(self, spec);
        Ok(())
    }
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
        match st
            .mem
            .vert_break_resume(&mut st.scanner, &mut cursor, h, d)?
        {
            VertBreakStep::Done(best_place) => return Ok(best_place.0),
            VertBreakStep::InfiniteShrinkage { glue, spec } => {
                let outcome = print_vert_break_infinite_shrinkage_error_in(st);
                if !matches!(outcome, ErrorOutcome::Continue) {
                    return Err(VertBreakAbort::Error(outcome));
                }
                st.mem.repair_vert_break_infinite_shrinkage(glue, spec)?;
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

impl MemState {
    /// The contents of box register `n`, leaving `n` in `cur_val`.
    fn reset_vsplit_marks(&mut self, scanner: &mut ScannerState) -> Result<(), NodeRecycleAbort> {
        self.flush_node_list(scanner.disc_ptr[VSPLIT_CODE as usize])
            .map_err(NodeRecycleAbort::Confusion)?;
        scanner.disc_ptr[VSPLIT_CODE as usize] = MemPtr::NULL;

        if let Some(root) = scanner.sa_root[MARK_VAL as usize].as_idx() {
            if self.do_marks(scanner, VSPLIT_INIT, 0, root) {
                scanner.sa_root[MARK_VAL as usize] = MemPtr::NULL;
            }
        }

        if !scanner.cur_mark[SPLIT_FIRST_MARK_CODE].is_null() {
            self.delete_token_ref(scanner.cur_mark[SPLIT_FIRST_MARK_CODE]);
            scanner.cur_mark[SPLIT_FIRST_MARK_CODE] = MemPtr::NULL;
            self.delete_token_ref(scanner.cur_mark[SPLIT_BOT_MARK_CODE]);
            scanner.cur_mark[SPLIT_BOT_MARK_CODE] = MemPtr::NULL;
        }

        Ok(())
    }
}

fn start_vsplit(st: &mut EngineState, n: i32, h: i32) -> Result<VsplitStart, NodeRecycleAbort> {
    st.scanner.cur_val = n;
    let v = crate::scan::fetch_box(st, n);
    st.mem.reset_vsplit_marks(&mut st.scanner)?;

    let v = match v {
        None => {
            return Ok(VsplitStart::Done(VsplitOutcome {
                result: None,
                packages: [None, None],
            }))
        }
        Some(AnyBox::VList(v)) => v,
        Some(AnyBox::HList(_)) => return Ok(VsplitStart::NeedsVboxDiagnostic),
    };

    Ok(VsplitStart::Ready(VsplitState {
        h,
        v,
        split_max_d: st.eqtb.dimen_par(DIMEN_PAR_SPLIT_MAX_DEPTH),
        vert_break: start_vert_break(&mut st.scanner, v.list_ptr(&st.mem)),
    }))
}

impl MemState {
    fn collect_split_mark(
        &mut self,
        scanner: &mut ScannerState,
        mark: NodePtr<MarkNode>,
    ) -> Result<(), NodeRecycleAbort> {
        let mark_class = mark.mark_class(self);
        let mark_ptr = mark.mark_ptr(self);
        if mark_class != 0 {
            let element = self
                .create_sa_element(scanner, MARK_VAL, mark_class)
                .ok_or(NodeRecycleAbort::Overflow)?;
            let (first, bot) = (SPLIT_FIRST_MARK_CODE as i32, SPLIT_BOT_MARK_CODE as i32);
            if element.child(self, first).is_null() {
                element.set_child(self, first, mark_ptr);
                self.add_token_ref(mark_ptr);
            } else {
                self.delete_token_ref(element.child(self, bot));
            }
            element.set_child(self, bot, mark_ptr);
            self.add_token_ref(mark_ptr);
        } else if scanner.cur_mark[SPLIT_FIRST_MARK_CODE].is_null() {
            scanner.cur_mark[SPLIT_FIRST_MARK_CODE] = mark_ptr;
            scanner.cur_mark[SPLIT_BOT_MARK_CODE] = mark_ptr;
            mark_ptr.set_info(self, mark_ptr.info(self) + 2);
        } else {
            self.delete_token_ref(scanner.cur_mark[SPLIT_BOT_MARK_CODE]);
            scanner.cur_mark[SPLIT_BOT_MARK_CODE] = mark_ptr;
            self.add_token_ref(mark_ptr);
        }
        Ok(())
    }

    /// Detach the part of `state.v`'s list before `q`, recording its marks.
    fn split_vlist_at_break(
        &mut self,
        scanner: &mut ScannerState,
        state: &VsplitState,
        q: MemPtr,
    ) -> Result<MemPtr, NodeRecycleAbort> {
        let mut p = state.v.list_ptr(self);
        if p == q {
            state.v.set_list_ptr(self, MemPtr::NULL);
        } else {
            loop {
                if let AnyNode::Mark(mark) = self
                    .classify(MemIdx::new(p))
                    .map_err(|_| NodeRecycleAbort::Confusion(c"vsplit"))?
                {
                    self.collect_split_mark(scanner, mark)?;
                }
                if p.link(self) == q {
                    p.set_link(self, MemPtr::NULL);
                    break;
                }
                p = p.link(self);
            }
        }

        Ok(state.v.list_ptr(self))
    }

    fn finish_vsplit_after_break(
        &mut self,
        eqtb: &mut EqtbState,
        scanner: &mut ScannerState,
        shipout: &mut ShipoutState,
        state: &VsplitState,
        q: MemPtr,
    ) -> Result<VsplitOutcome, NodeRecycleAbort> {
        let mut packages = [None, None];
        let p = self.split_vlist_at_break(scanner, state, q)?;
        let save_discards = eqtb.int_par(INT_PAR_SAVING_VDISCARDS) > 0;
        let mut q = self.prune_page_top(scanner, eqtb, q.0, save_discards)?;

        self.free_node(state.v.ptr(), BOX_NODE_SIZE);
        if !q.is_null() {
            let packed = self.vpackage(eqtb, scanner, shipout, q, 0, ADDITIONAL, MAX_HALFWORD)?;
            q = packed.r_box.ptr();
            push_slot(&mut packages, packed);
        }
        let n = scanner.cur_val;
        self.change_box(scanner, eqtb, n, q);

        let packed = self.vpackage(
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
        &mut self,
        eqtb: &mut EqtbState,
        scanner: &mut ScannerState,
        shipout: &mut ShipoutState,
        state: &mut VsplitState,
    ) -> Result<VsplitStep, NodeRecycleAbort> {
        match self.vert_break_resume(scanner, &mut state.vert_break, state.h, state.split_max_d)? {
            VertBreakStep::Done(q) => {
                let outcome = self.finish_vsplit_after_break(eqtb, scanner, shipout, state, q)?;
                Ok(VsplitStep::Done(outcome))
            }
            VertBreakStep::InfiniteShrinkage { glue, spec } => {
                Ok(VsplitStep::InfiniteShrinkage { glue, spec })
            }
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
    let mut state = match start_vsplit(st, n, h)? {
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
        match st
            .mem
            .vsplit_resume(&mut st.eqtb, &mut st.scanner, &mut st.shipout, &mut state)?
        {
            VsplitStep::Done(outcome) => {
                print_vsplit_packages_in(st, outcome);
                return Ok(outcome.result);
            }
            VsplitStep::InfiniteShrinkage { glue, spec } => {
                let outcome = print_vert_break_infinite_shrinkage_error_in(st);
                if !matches!(outcome, ErrorOutcome::Continue) {
                    return Err(VsplitAbort::Error(outcome));
                }
                st.mem.repair_vert_break_infinite_shrinkage(glue, spec)?;
            }
        }
    }
}

impl MemState {
    /// Insert `\splittopskip` glue between `prev_p` and the box or rule `p` of `height`.
    fn insert_split_top_skip_before(
        &mut self,
        eqtb: &EqtbState,
        prev_p: MemPtr,
        p: MemPtr,
        height: i32,
    ) -> Result<(), NodeRecycleAbort> {
        let q = self
            .new_skip_param(eqtb, GLUE_PAR_SPLIT_TOP_SKIP)
            .ok_or(NodeRecycleAbort::Overflow)?;
        prev_p.set_link(self, q.ptr());
        q.set_link(self, p);
        let spec = q.glue_ptr(self);
        let avail_ht = spec.width(self).0;
        if avail_ht > height {
            spec.set_width(self, Scaled(avail_ht - height));
        } else {
            spec.set_width(self, Scaled(0));
        }
        Ok(())
    }

    /// Unlink the discardable `node` after `prev_p`, saving it after `r` (or
    /// flushing it), and return its successor.
    fn prune_discard_node(
        &mut self,
        scanner: &mut ScannerState,
        prev_p: MemPtr,
        r: &mut MemPtr,
        node: MemPtr,
        save_discards: bool,
    ) -> Result<MemPtr, NodeRecycleAbort> {
        let q = node.link(self);
        node.set_link(self, MemPtr::NULL);
        prev_p.set_link(self, q);
        if save_discards {
            if scanner.disc_ptr[VSPLIT_CODE as usize].is_null() {
                scanner.disc_ptr[VSPLIT_CODE as usize] = node;
            } else {
                r.set_link(self, node);
            }
            *r = node;
        } else {
            self.flush_node_list(node)
                .map_err(NodeRecycleAbort::Confusion)?;
        }
        Ok(q)
    }

    /// Remove the discardable items from the top of the vlist `p`, putting
    /// `\splittopskip` before its first box or rule.
    pub fn prune_page_top(
        &mut self,
        scanner: &mut ScannerState,
        eqtb: &EqtbState,
        p: i32,
        s: bool,
    ) -> Result<MemPtr, NodeRecycleAbort> {
        let mut p = MemPtr(p);
        let mut prev_p = TEMP_HEAD;
        let mut r = MemPtr::NULL;
        TEMP_HEAD.set_link(self, p);

        while !p.is_null() {
            let node = self
                .classify(MemIdx::new(p))
                .map_err(|_| NodeRecycleAbort::Confusion(c"pruning"))?;
            let height = match node {
                AnyNode::HList(b) => b.height(self).0,
                AnyNode::VList(b) => b.height(self).0,
                AnyNode::Rule(rule) => rule.height(self).raw(),
                AnyNode::Whatsit(_) | AnyNode::Mark(_) | AnyNode::Ins(_) => {
                    prev_p = p;
                    p = prev_p.link(self);
                    continue;
                }
                AnyNode::Glue(_) | AnyNode::Kern(_) | AnyNode::Penalty(_) => {
                    p = self.prune_discard_node(scanner, prev_p, &mut r, p, s)?;
                    continue;
                }
                _ => return Err(NodeRecycleAbort::Confusion(c"pruning")),
            };
            self.insert_split_top_skip_before(eqtb, prev_p, p, height)?;
            p = MemPtr::NULL;
        }

        Ok(TEMP_HEAD.link(self))
    }

    /// Apply mark action `a` to the sparse-array mark tree `q` at level `l`.
    /// Returns whether the record at `q` was freed.
    pub fn do_marks(&mut self, scanner: &mut ScannerState, a: i16, l: i16, q: MemIdx) -> bool {
        let node = NodePtr::<SaNode>::at(q);

        if l < 4 {
            for i in 0..=15 {
                scanner.cur_ptr = node.child(self, i);
                if let Some(cur) = scanner.cur_ptr.as_idx() {
                    if self.do_marks(scanner, a, l + 1, cur) {
                        node.set_child(self, i, MemPtr::NULL);
                        node.set_ref_count(self, node.ref_count(self) - 1);
                    }
                }
            }

            if node.ref_count(self) == 0 {
                self.free_node(MemPtr::from_idx(q), INDEX_NODE_SIZE);
                return true;
            }
            return false;
        }

        // Mark-class slots by mark code: 0 top, 1 first, 2 bot, 3 split first, 4 split bot.
        match a {
            VSPLIT_INIT => {
                if !node.child(self, 3).is_null() {
                    self.delete_token_ref(node.child(self, 3));
                    node.set_child(self, 3, MemPtr::NULL);
                    self.delete_token_ref(node.child(self, 4));
                    node.set_child(self, 4, MemPtr::NULL);
                }
            }
            FIRE_UP_INIT => {
                if !node.child(self, 2).is_null() {
                    if !node.child(self, 0).is_null() {
                        self.delete_token_ref(node.child(self, 0));
                    }
                    self.delete_token_ref(node.child(self, 1));
                    node.set_child(self, 1, MemPtr::NULL);
                    let bot = node.child(self, 2);
                    if bot.link(self).is_null() {
                        self.delete_token_ref(bot);
                        node.set_child(self, 2, MemPtr::NULL);
                    } else {
                        self.add_token_ref(bot);
                    }
                    node.set_child(self, 0, node.child(self, 2));
                }
            }
            FIRE_UP_DONE => {
                let top = node.child(self, 0);
                if !top.is_null() && node.child(self, 1).is_null() {
                    node.set_child(self, 1, top);
                    self.add_token_ref(top);
                }
            }
            DESTROY_MARKS => {
                for i in 0..=4 {
                    scanner.cur_ptr = node.child(self, i);
                    if !scanner.cur_ptr.is_null() {
                        self.delete_token_ref(scanner.cur_ptr);
                        node.set_child(self, i, MemPtr::NULL);
                    }
                }
            }
            _ => {}
        }

        if node.child(self, 2).is_null() && node.child(self, 4).is_null() {
            self.free_node(MemPtr::from_idx(q), MARK_CLASS_NODE_SIZE);
            return true;
        }
        false
    }
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

impl MemState {
    /// Set `r`'s glue to distribute `excess` over the highest-order `totals`,
    /// returning that order.
    fn set_box_glue<K: IsBox>(
        &mut self,
        r: NodePtr<K>,
        sign: GlueSign,
        excess: f64,
        totals: &[i32; 4],
    ) -> u16 {
        let o = glue_order(totals);
        r.set_glue_order(self, o);
        r.set_glue_sign(self, sign);
        if totals[o as usize] != 0 {
            r.set_glue_set(self, excess / totals[o as usize] as f64);
        } else {
            r.set_glue_sign(self, GlueSign::Normal);
            r.set_glue_set(self, 0.0);
        }
        o
    }

    fn clear_box_glue<K: IsBox>(&mut self, r: NodePtr<K>) {
        r.set_glue_sign(self, GlueSign::Normal);
        r.set_glue_order(self, NORMAL);
        r.set_glue_set(self, 0.0);
    }

    /// `replace_count(q)` if `type(q)=disc_node`, read like TeX does even when
    /// `q` is a char node (whose font sits in the type field).
    fn disc_replace_count(&self, q: MemPtr) -> Option<i32> {
        match self.as_char(q) {
            Some(ch) => {
                (ch.font(self).0 == NodeType::Disc as i32).then(|| ch.character(self) as i32)
            }
            None => self.as_disc(q).map(|disc| disc.subtype(self) as i32),
        }
    }

    /// Merge the run of native words in `w`'s font that follows `w` (possibly
    /// across empty discretionaries) into one node. `q` trails `w`. Returns the
    /// new `q` and the node now in `w`'s place.
    fn merge_native_words(
        &mut self,
        eqtb: &EqtbState,
        fonts: &FontState,
        r_node: NodePtr<HListNode>,
        mut q: MemPtr,
        w: NodePtr<WhatsitNode>,
    ) -> Result<(MemPtr, NodePtr<WhatsitNode>), NodeRecycleAbort> {
        let p = w.ptr();
        let font = w.native_font(self).0;
        let same_font = |mem: &MemState, p: MemPtr| {
            mem.as_native_word(p)
                .filter(|w| w.native_font(mem).0 == font)
        };

        let mut k = if q != r_node.ptr() + LIST_OFFSET {
            self.disc_replace_count(q).unwrap_or(0)
        } else {
            0
        };
        while q.link(self) != p {
            k -= 1;
            q = q.link(self);
            if let Some(count) = self.disc_replace_count(q) {
                k = count;
            }
        }

        let mut pp = p.link(self);
        while k <= 0 && !pp.is_null() && !self.is_char_node(pp) {
            if let Some(next) = same_font(self, pp) {
                pp = next.link(self);
            } else if let Some(next) = self
                .as_disc(pp)
                .and_then(|disc| same_font(self, disc.link(self)))
            {
                pp = next.link(self);
            } else {
                break;
            }
        }
        if pp == p.link(self) {
            return Ok((q, w));
        }

        let mut total_chars: i32 = 0;
        let mut s = p;
        while s != pp {
            if let Some(sw) = self.as_native_word(s) {
                total_chars += sw.native_length(self) as i32;
            }
            s = s.link(self);
        }

        let merged = self.new_native_word_node(eqtb, font, total_chars)?;
        merged.set_subtype(self, w.subtype(self));
        q.set_link(self, merged.ptr());
        let last = self.predecessor(p, pp);
        merged.set_link(self, last.link(self));
        last.set_link(self, MemPtr::NULL);

        total_chars = 0;
        let mut s = p;
        loop {
            if let Some(sw) = self.as_native_word(s) {
                let count = sw.native_length(self) as usize;
                let src = sw.native_text_ptr(self);
                let dst = merged.native_text_ptr_mut(self);
                // SAFETY: `merged` holds the fragments' total length, the fragments are
                // distinct nodes, and the arena is not reallocated here.
                unsafe { std::ptr::copy_nonoverlapping(src, dst.add(total_chars as usize), count) };
                total_chars += count as i32;
            }
            if s.link(self).is_null() {
                break;
            }
            s = s.link(self);
        }
        self.flush_node_list(p)
            .map_err(NodeRecycleAbort::Confusion)?;
        self.set_native_metrics(eqtb, fonts, merged)?;
        Ok((q, merged))
    }

    #[allow(clippy::too_many_arguments)]
    fn hpack(
        &mut self,
        eqtb: &EqtbState,
        fonts: &mut FontState,
        scanner: &mut ScannerState,
        shipout: &mut ShipoutState,
        list: MemPtr,
        w: i32,
        m: i16,
    ) -> Result<PackOutcome, NodeRecycleAbort> {
        shipout.last_badness = 0;
        let r_node = NodePtr::<HListNode>::alloc(self).ok_or(NodeRecycleAbort::Overflow)?;
        r_node.set_subtype(self, 0);
        r_node.set_shift_amount(self, Scaled(0));
        r_node.set_list_ptr(self, list);
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
            self.push_lr(&mut scanner.lr_ptr, BEFORE.into())
                .ok_or(NodeRecycleAbort::OverflowRunaway)?;
        }

        let mut p = list;
        'outer: while !p.is_null() {
            while let Some(ch) = self.as_char(p) {
                let f = ch.font(self).0;
                let c = fonts.effective_char(f, ch.character(self));
                let info = fonts.char_info(f, c);
                x += fonts.char_width(f, info);
                h = h.max(fonts.char_height(f, info));
                d = d.max(fonts.char_depth(f, info));
                p = p.link(self);
            }

            if p.is_null() {
                break;
            }

            match self.classify(MemIdx::new(p)) {
                Ok(AnyNode::HList(b)) => {
                    let s = b.shift_amount(self).0;
                    x += b.width(self).0;
                    h = h.max(b.height(self).0 - s);
                    d = d.max(b.depth(self).0 + s);
                }
                Ok(AnyNode::VList(b)) => {
                    let s = b.shift_amount(self).0;
                    x += b.width(self).0;
                    h = h.max(b.height(self).0 - s);
                    d = d.max(b.depth(self).0 + s);
                }
                Ok(AnyNode::Rule(rule)) => {
                    x += rule.width(self).raw();
                    h = h.max(rule.height(self).raw());
                    d = d.max(rule.depth(self).raw());
                }
                Ok(AnyNode::Unset(u)) => {
                    x += u.width(self).0;
                    h = h.max(u.height(self).0);
                    d = d.max(u.depth(self).0);
                }
                Ok(AnyNode::Adjust(adjust)) => {
                    if !scanner.adjust_tail.is_null() || !scanner.pre_adjust_tail.is_null() {
                        q = self.predecessor(q, p);
                        let adjust_list = adjust.adjust_ptr(self);
                        let tail = if adjust.subtype(self) != 0 {
                            &mut scanner.pre_adjust_tail
                        } else {
                            &mut scanner.adjust_tail
                        };
                        if tail.is_null() {
                            return Err(NodeRecycleAbort::Confusion(c"pre vadjust"));
                        }
                        tail.set_link(self, adjust_list);
                        *tail = self.list_tail(*tail);
                        p = adjust.link(self);
                        self.free_node(adjust.ptr(), SMALL_NODE_SIZE);
                        q.set_link(self, p);
                        p = q;
                    }
                }
                Ok(AnyNode::Ins(_) | AnyNode::Mark(_)) => {
                    if !scanner.adjust_tail.is_null() || !scanner.pre_adjust_tail.is_null() {
                        q = self.predecessor(q, p);
                        scanner.adjust_tail.set_link(self, p);
                        scanner.adjust_tail = p;
                        p = p.link(self);
                        q.set_link(self, p);
                        p = q;
                    }
                }
                Ok(AnyNode::Whatsit(w)) => match w.whatsit_subtype(self) {
                    Some(WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt) => {
                        let (new_q, native) = self.merge_native_words(eqtb, fonts, r_node, q, w)?;
                        q = new_q;
                        p = native.ptr();
                        h = h.max(native.height(self).0);
                        d = d.max(native.depth(self).0);
                        x += native.width(self).0;
                    }
                    Some(WhatsitSubtype::Glyph | WhatsitSubtype::Pic | WhatsitSubtype::Pdf) => {
                        h = h.max(w.height(self).0);
                        d = d.max(w.depth(self).0);
                        x += w.width(self).0;
                    }
                    _ => {}
                },
                Ok(AnyNode::Glue(glue)) => {
                    let spec = glue.glue_ptr(self);
                    x += spec.width(self).0;
                    scanner.total_stretch[spec.stretch_order(self) as usize] +=
                        spec.stretch(self).0;
                    scanner.total_shrink[spec.shrink_order(self) as usize] += spec.shrink(self).0;
                    if glue.subtype(self) >= A_LEADERS {
                        let leader = glue.leader_ptr(self);
                        let (leader_h, leader_d) = match self.classify(MemIdx::new(leader)) {
                            Ok(AnyNode::HList(b)) => (b.height(self).0, b.depth(self).0),
                            Ok(AnyNode::VList(b)) => (b.height(self).0, b.depth(self).0),
                            Ok(AnyNode::Rule(rule)) => {
                                (rule.height(self).raw(), rule.depth(self).raw())
                            }
                            _ => (self.b32s1(leader + 3), self.b32s1(leader + 2)),
                        };
                        h = h.max(leader_h);
                        d = d.max(leader_d);
                    }
                }
                Ok(AnyNode::Kern(kern)) => {
                    x += kern.width(self).0;
                }
                Ok(AnyNode::MarginKern(kern)) => {
                    x += kern.width(self).0;
                }
                Ok(AnyNode::Math(math)) => {
                    x += math.width(self).0;
                    if texxet {
                        let subtype = math.subtype(self);
                        if subtype % 2 == 1 {
                            if scanner.lr_ptr.info(self) == end_lr_type(subtype) {
                                self.pop_lr(&mut scanner.lr_ptr);
                            } else {
                                scanner.lr_problems += 1;
                                math.into_kern(self).set_subtype(self, EXPLICIT);
                            }
                        } else {
                            self.push_lr(&mut scanner.lr_ptr, end_lr_type(subtype))
                                .ok_or(NodeRecycleAbort::OverflowRunaway)?;
                        }
                    }
                }
                Ok(AnyNode::Ligature(lig)) => {
                    // Measure the ligature as its character through the scratch char node.
                    let scratch = NodePtr::<CharNode>::at(GARBAGE);
                    scratch.set_font(self, lig.font(self));
                    scratch.set_character(self, lig.character(self));
                    GARBAGE.set_link(self, lig.link(self));
                    p = GARBAGE;
                    fonts.xtx_ligature_present = true;
                    continue 'outer;
                }
                _ => {}
            }
            p = p.link(self);
        }

        if !scanner.adjust_tail.is_null() {
            scanner.adjust_tail.set_link(self, MemPtr::NULL);
        }
        if !scanner.pre_adjust_tail.is_null() {
            scanner.pre_adjust_tail.set_link(self, MemPtr::NULL);
        }
        r_node.set_height(self, Scaled(h));
        r_node.set_depth(self, Scaled(d));

        let w = if m == ADDITIONAL { x + w } else { w };
        r_node.set_width(self, Scaled(w));
        x = w - x;

        let has_list = !r_node.list_ptr(self).is_null();
        if x == 0 {
            self.clear_box_glue(r_node);
        } else if x > 0 {
            let o = self.set_box_glue(
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
            let o = self.set_box_glue(
                r_node,
                GlueSign::Shrinking,
                (-x) as f64,
                &scanner.total_shrink,
            );
            if scanner.total_shrink[o as usize] < (-x) && o == NORMAL && has_list {
                shipout.last_badness = 1_000_000;
                r_node.set_glue_set(self, 1.0);
                let excess = (-x) - scanner.total_shrink[NORMAL as usize];
                let hfuzz = eqtb.dimen_par(DIMEN_PAR_HFUZZ);
                if excess > hfuzz || eqtb.int_par(INT_PAR_HBADNESS) < 100 {
                    let overfull_rule = eqtb.dimen_par(DIMEN_PAR_OVERFULL_RULE);
                    if overfull_rule > 0 && excess > hfuzz {
                        q = self.list_tail(q);
                        let rule = self.new_rule().ok_or(NodeRecycleAbort::Overflow)?;
                        q.set_link(self, rule.ptr());
                        rule.set_width(self, RuleDimen::Set(Scaled(overfull_rule)));
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
            if scanner.lr_ptr.info(self) != BEFORE.into() {
                q = self.list_tail(q);
                loop {
                    let math = self
                        .new_math(0, scanner.lr_ptr.info(self) as u16)
                        .ok_or(NodeRecycleAbort::Overflow)?;
                    q.set_link(self, math.ptr());
                    q = math.ptr();
                    scanner.lr_problems += 10000;
                    self.pop_lr(&mut scanner.lr_ptr);
                    if scanner.lr_ptr.info(self) == BEFORE.into() {
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
            self.pop_lr(&mut scanner.lr_ptr);
            if !scanner.lr_ptr.is_null() {
                return Err(NodeRecycleAbort::Confusion(c"LR1"));
            }
        }

        Ok(PackOutcome {
            r_box: AnyBox::HList(r_node),
            diags,
        })
    }
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
    let outcome = st.mem.hpack(
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

impl MemState {
    #[allow(clippy::too_many_arguments)]
    fn vpackage(
        &mut self,
        eqtb: &EqtbState,
        scanner: &mut ScannerState,
        shipout: &mut ShipoutState,
        list: MemPtr,
        h: i32,
        m: i16,
        l: i32,
    ) -> Result<PackOutcome, NodeRecycleAbort> {
        shipout.last_badness = 0;
        let r_node = NodePtr::<VListNode>::alloc(self).ok_or(NodeRecycleAbort::Overflow)?;
        let upwards = eqtb.int_par(INT_PAR_XETEX_UPWARDS) > 0;
        r_node.set_subtype(self, u16::from(upwards));
        r_node.set_shift_amount(self, Scaled(0));
        r_node.set_list_ptr(self, list);

        let mut w: i32 = 0;
        let mut d: i32 = 0;
        let mut x: i32 = 0;
        scanner.total_stretch = [0; 4];
        scanner.total_shrink = [0; 4];
        let mut diags = [None, None];

        let mut p = list;
        while !p.is_null() {
            if self.is_char_node(p) {
                return Err(NodeRecycleAbort::Confusion(c"vpack"));
            }
            match self.classify(MemIdx::new(p)) {
                Ok(AnyNode::HList(b)) => {
                    x += d + b.height(self).0;
                    d = b.depth(self).0;
                    w = w.max(b.width(self).0 + b.shift_amount(self).0);
                }
                Ok(AnyNode::VList(b)) => {
                    x += d + b.height(self).0;
                    d = b.depth(self).0;
                    w = w.max(b.width(self).0 + b.shift_amount(self).0);
                }
                Ok(AnyNode::Rule(rule)) => {
                    x += d + rule.height(self).raw();
                    d = rule.depth(self).raw();
                    w = w.max(rule.width(self).raw());
                }
                Ok(AnyNode::Unset(u)) => {
                    x += d + u.height(self).0;
                    d = u.depth(self).0;
                    w = w.max(u.width(self).0);
                }
                Ok(AnyNode::Whatsit(wn)) => {
                    if matches!(
                        wn.whatsit_subtype(self),
                        Some(WhatsitSubtype::Pic | WhatsitSubtype::Pdf)
                    ) {
                        x += d + wn.height(self).0;
                        d = wn.depth(self).0;
                        w = w.max(wn.width(self).0);
                    }
                }
                Ok(AnyNode::Glue(glue)) => {
                    x += d;
                    d = 0;
                    let spec = glue.glue_ptr(self);
                    x += spec.width(self).0;
                    scanner.total_stretch[spec.stretch_order(self) as usize] +=
                        spec.stretch(self).0;
                    scanner.total_shrink[spec.shrink_order(self) as usize] += spec.shrink(self).0;
                    if glue.subtype(self) >= A_LEADERS {
                        let leader = glue.leader_ptr(self);
                        let leader_w = match self.classify(MemIdx::new(leader)) {
                            Ok(AnyNode::HList(b)) => b.width(self).0,
                            Ok(AnyNode::VList(b)) => b.width(self).0,
                            Ok(AnyNode::Rule(rule)) => rule.width(self).raw(),
                            _ => self.b32s1(leader + 1),
                        };
                        w = w.max(leader_w);
                    }
                }
                Ok(AnyNode::Kern(kern)) => {
                    x += d + kern.width(self).0;
                    d = 0;
                }
                _ => {}
            }
            p = p.link(self);
        }

        r_node.set_width(self, Scaled(w));
        if d > l {
            x += d - l;
            r_node.set_depth(self, Scaled(l));
        } else {
            r_node.set_depth(self, Scaled(d));
        }

        let h = if m == ADDITIONAL { x + h } else { h };
        r_node.set_height(self, Scaled(h));
        x = h - x;

        let has_list = !r_node.list_ptr(self).is_null();
        if x == 0 {
            self.clear_box_glue(r_node);
        } else if x > 0 {
            let o = self.set_box_glue(
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
            let o = self.set_box_glue(
                r_node,
                GlueSign::Shrinking,
                (-(x as i64)) as f64,
                &scanner.total_shrink,
            );
            if scanner.total_shrink[o as usize] < (-x) && o == NORMAL && has_list {
                shipout.last_badness = 1_000_000;
                r_node.set_glue_set(self, 1.0);
                let excess = (-x) - scanner.total_shrink[NORMAL as usize];
                if excess > eqtb.dimen_par(DIMEN_PAR_VFUZZ) || eqtb.int_par(INT_PAR_VBADNESS) < 100
                {
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
    let outcome = st
        .mem
        .vpackage(&st.eqtb, &mut st.scanner, &mut st.shipout, list, h, m, l)?;
    for diag in outcome.diags.into_iter().flatten() {
        print_vpack_diag_in(st, outcome.r_box, diag);
    }
    Ok(outcome.r_box)
}
