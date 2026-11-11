//! Copying and destroying node lists: `copy_node_list`, `flush_node_list`, `just_copy`.

use crate::consts::{MIN_HALFWORD, SUB_BOX};
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::mem::TEMP_HEAD;
use crate::node::CharNode;
use crate::node::{
    AnyNode, BoxNode, DiscNode, GlueNode, HeadNode, MathNode, NoadVariant, Node, NodePtr, RuleNode,
    View, WhatsitNode, WhatsitVariant,
};
use crate::state::MemState;

use super::LrState;
use crate::units::{MemIdx, MemPtr, Scaled};

/// `r` typed like `src`, whose header word `r` has just received.
#[inline]
fn copied_node<K: View>(_src: NodePtr<K>, r: MemPtr) -> NodePtr<K> {
    NodePtr::from_idx_unchecked(MemIdx::new(r))
}

impl MemState {
    pub(super) fn as_disc(&self, p: MemPtr) -> Option<NodePtr<DiscNode>> {
        if self.is_char_node(p) {
            return None;
        }
        self.classify_disc(p).ok().flatten()
    }

    /// A new node of `p`'s kind holding every field of `p`.
    fn copy_of<K: Node>(&mut self, p: NodePtr<K>) -> Result<NodePtr<K>, Abort> {
        let r = NodePtr::<K>::alloc(self)?;
        self[r] = self[p];
        Ok(r)
    }

    fn copy_box_shell(
        &mut self,
        src: NodePtr<BoxNode>,
        list: MemPtr,
    ) -> Result<NodePtr<BoxNode>, Abort> {
        let r = self.copy_of(src)?;
        self[r].list_ptr = list;
        Ok(r)
    }

    /// A copy of glue node `p` sharing its spec. The leader pointer is left to the caller.
    fn copy_glue_shell(&mut self, p: NodePtr<GlueNode>) -> Result<NodePtr<GlueNode>, Abort> {
        self.add_glue_ref(self[p].glue_ptr);
        self.copy_of(p)
    }

    /// A copy of whatsit `w`, sharing what it refers to.
    fn copy_whatsit(&mut self, w: NodePtr<WhatsitNode>) -> Result<MemPtr, Abort> {
        Ok(match w.variant(self).or_confusion("ext2")? {
            WhatsitVariant::Open(p) => self.copy_of(p)?.ptr(),
            WhatsitVariant::Write(p)
            | WhatsitVariant::Special(p)
            | WhatsitVariant::LateSpecial(p) => {
                self.tokens.add_ref(self[p].tokens);
                self.copy_of(p)?.ptr()
            }
            WhatsitVariant::Close(p) => self.copy_of(p)?.ptr(),
            WhatsitVariant::Language(p) => self.copy_of(p)?.ptr(),
            WhatsitVariant::SavePos(p) => self.copy_of(p)?.ptr(),
            WhatsitVariant::NativeWord(word) => {
                let copy = self.copy_of(word)?;
                self[copy].clear_glyphs();
                copy.share_native_glyphs(self, word);
                self.share_payload(copy, word)?;
                copy.ptr()
            }
            WhatsitVariant::Glyph(p) => self.copy_of(p)?.ptr(),
            WhatsitVariant::Pic(pic) => {
                let copy = self.copy_of(pic)?;
                self.share_payload(copy, pic)?;
                copy.ptr()
            }
        })
    }

    /// Copy the list `p` without the contents of its boxes, linking the copies
    /// after `h` and ending them with `t`.
    pub fn just_copy(&mut self, mut p: MemPtr, mut h: MemPtr, t: MemPtr) -> Result<(), Abort> {
        while !p.is_null() {
            let copy = if let Some(ch) = self.as_char(p) {
                let CharNode {
                    font, character, ..
                } = self[ch];
                Some(self.new_char(font, character)?.ptr())
            } else {
                match self.classify(p).ok() {
                    Some(AnyNode::Box(b)) => Some(self.copy_box_shell(b, MemPtr::NULL)?.ptr()),
                    Some(AnyNode::Rule(r)) => Some(self.copy_of(r)?.ptr()),
                    Some(AnyNode::Ligature(lig)) => {
                        let (f, c) = (self[lig].font, self[lig].character);
                        Some(self.new_char(f, c)?.ptr())
                    }
                    Some(AnyNode::Kern(k)) => Some(self.copy_of(k)?.ptr()),
                    Some(AnyNode::Math(m)) => Some(self.copy_of(m)?.ptr()),
                    Some(AnyNode::Glue(g)) => {
                        let r = self.copy_glue_shell(g)?;
                        self[r].leader_ptr = MemPtr::NULL;
                        Some(r.ptr())
                    }
                    Some(AnyNode::Whatsit(w)) => Some(self.copy_whatsit(w)?),
                    _ => None,
                }
            };

            if let Some(r) = copy {
                h.set_link(self, r);
                h = r;
            }
            p = p.link(self);
        }

        h.set_link(self, t);
        Ok(())
    }

    /// Reverse the hlist after `p` for TeX--XeT, leaving the result after `TEMP_HEAD`.
    pub(crate) fn just_reverse(&mut self, lr: &mut LrState, p: MemPtr) -> Result<(), Abort> {
        let mut m = MIN_HALFWORD;
        let mut n = MIN_HALFWORD;

        let mut q = if TEMP_HEAD.link(self).is_null() {
            self.just_copy(p.link(self), TEMP_HEAD, MemPtr::NULL)?;
            TEMP_HEAD.link(self)
        } else {
            let q = p.link(self);
            p.set_link(self, MemPtr::NULL);
            self.flush_node_list(TEMP_HEAD.link(self))?;
            q
        };

        let t = self.new_edge(lr.dir, Scaled::ZERO)?;
        let mut l = t.ptr();
        lr.dir = lr.dir.reversed();

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

                if let Ok(AnyNode::Math(math)) = self.classify(p) {
                    let code = self[math].code()?;
                    if code.is_end() {
                        if self.lr_code(lr.ptr) != Some(code.lr_end()) {
                            math.into_kern(self);
                            lr.problems += 1;
                        } else {
                            self.pop_lr(&mut lr.ptr);

                            if n > MIN_HALFWORD {
                                n -= 1;
                                self[math].subtype = code.as_begin().into();
                            } else if m > MIN_HALFWORD {
                                m -= 1;
                                math.into_kern(self);
                            } else {
                                self[t].width = self[math].width;
                                self[t].link = q;
                                self.free_node(p, MathNode::SIZE);
                                break 'done;
                            }
                        }
                    } else {
                        self.push_lr(&mut lr.ptr, code.lr_end())?;

                        if n > MIN_HALFWORD || code.dir() != lr.dir {
                            n += 1;
                            self[math].subtype = code.as_end().into();
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
    pub fn flush_node_list(&mut self, mut p: MemPtr) -> Result<(), Abort> {
        while !p.is_null() {
            let q = p.link(self);
            match self.classify(p).or_confusion("flushing")? {
                AnyNode::Char(ch) => ch.free(self),
                AnyNode::Box(b) => {
                    self.flush_node_list(self[b].list_ptr)?;
                    b.free(self);
                }
                AnyNode::Unset(u) => {
                    self.flush_node_list(self[u].list_ptr)?;
                    u.free(self);
                }
                AnyNode::Rule(r) => r.free(self),
                AnyNode::Ins(ins) => {
                    self.flush_node_list(self[ins].ins_ptr)?;
                    self.delete_glue_ref(self[ins].split_top_ptr);
                    ins.free(self);
                }
                AnyNode::Whatsit(w) => match w.variant(self).or_confusion("ext3")? {
                    WhatsitVariant::Open(p) => p.free(self),
                    WhatsitVariant::Write(p)
                    | WhatsitVariant::Special(p)
                    | WhatsitVariant::LateSpecial(p) => {
                        self.tokens.delete_ref(self[p].tokens);
                        p.free(self);
                    }
                    WhatsitVariant::Close(p) => p.free(self),
                    WhatsitVariant::Language(p) => p.free(self),
                    WhatsitVariant::SavePos(p) => p.free(self),
                    WhatsitVariant::NativeWord(word) => {
                        word.release_native_glyphs(self);
                        self.release_payload(word);
                        word.free(self);
                    }
                    WhatsitVariant::Glyph(p) => p.free(self),
                    WhatsitVariant::Pic(pic) => {
                        self.release_payload(pic);
                        pic.free(self);
                    }
                },
                AnyNode::Glue(glue) => {
                    self.delete_glue_ref(self[glue].glue_ptr);
                    self.flush_node_list(self[glue].leader_ptr)?;
                    glue.free(self);
                }
                AnyNode::Kern(k) => k.free(self),
                AnyNode::Math(m) => m.free(self),
                AnyNode::Penalty(pen) => pen.free(self),
                AnyNode::MarginKern(m) => m.free(self),
                AnyNode::Ligature(lig) => {
                    self.flush_node_list(self[lig].lig_ptr)?;
                    lig.free(self);
                }
                AnyNode::Mark(mark) => {
                    self.tokens.delete_ref(self[mark].mark_ptr);
                    mark.free(self);
                }
                AnyNode::Disc(disc) => {
                    self.flush_node_list(self[disc].pre_break)?;
                    self.flush_node_list(self[disc].post_break)?;
                    disc.free(self);
                }
                AnyNode::Adjust(adjust) => {
                    self.flush_node_list(self[adjust].adjust_ptr)?;
                    adjust.free(self);
                }
                AnyNode::Style(style) => style.free(self),
                AnyNode::Choice(choice) => {
                    self.flush_node_list(self[choice].display_mlist)?;
                    self.flush_node_list(self[choice].text_mlist)?;
                    self.flush_node_list(self[choice].script_mlist)?;
                    self.flush_node_list(self[choice].script_script_mlist)?;
                    choice.free(self);
                }
                AnyNode::Noad(any) => {
                    let v = any.variant(self);
                    if let NoadVariant::Fraction(f) = v {
                        self.flush_node_list(self[f].numerator.info())?;
                        self.flush_node_list(self[f].denominator.info())?;
                    } else if let Some(n) = v.as_noad() {
                        for field in [self[n].nucleus, self[n].supscr, self[n].subscr] {
                            if field.math_type >= SUB_BOX {
                                self.flush_node_list(field.info())?;
                            }
                        }
                    }
                    self.free_node(p, v.size());
                }
            }
            p = q;
        }
        Ok(())
    }

    /// A copy of box `b` and its contents.
    pub(crate) fn copy_box(&mut self, b: NodePtr<BoxNode>) -> Result<NodePtr<BoxNode>, Abort> {
        let r = self.copy_list(b.ptr())?;
        Ok(copied_node(b, r))
    }

    /// A deep copy of the list `p`.
    pub fn copy_node_list(&mut self, p: MemPtr) -> Result<MemPtr, Abort> {
        self.copy_list(p)
    }

    fn copy_list(&mut self, mut p: MemPtr) -> Result<MemPtr, Abort> {
        let h = NodePtr::<HeadNode>::alloc(self)?;
        let mut q = h.ptr();
        while !p.is_null() {
            let r = self.copy_node(p)?;
            q.set_link(self, r);
            q = r;
            p = p.link(self);
        }
        q.set_link(self, MemPtr::NULL);
        let result = self[h].link;
        h.free(self);
        Ok(result)
    }

    /// A copy of node `p` and everything it owns.
    fn copy_node(&mut self, p: MemPtr) -> Result<MemPtr, Abort> {
        let node = self.classify(p).or_confusion("copying")?;
        Ok(match node {
            AnyNode::Char(ch) => {
                let CharNode {
                    font, character, ..
                } = self[ch];
                self.new_char(font, character)?.ptr()
            }
            AnyNode::Box(b) => {
                let list = self.copy_list(self[b].list_ptr)?;
                self.copy_box_shell(b, list)?.ptr()
            }
            AnyNode::Unset(u) => {
                let r = self.copy_of(u)?;
                let list = self.copy_list(self[u].list_ptr)?;
                self[r].list_ptr = list;
                r.ptr()
            }
            AnyNode::Rule(rule) => {
                // The copy keeps its own SyncTeX stamp.
                let r = NodePtr::<RuleNode>::alloc(self)?;
                let (sync_tag, sync_line) = (self[r].sync.tag, self[r].sync.line);
                self[r] = self[rule];
                (self[r].sync.tag, self[r].sync.line) = (sync_tag, sync_line);
                r.ptr()
            }
            AnyNode::Ins(ins) => {
                let r = self.copy_of(ins)?;
                self.add_glue_ref(self[ins].split_top_ptr);
                let list = self.copy_list(self[ins].ins_ptr)?;
                self[r].ins_ptr = list;
                r.ptr()
            }
            AnyNode::Whatsit(w) => self.copy_whatsit(w)?,
            AnyNode::Glue(g) => {
                let r = self.copy_glue_shell(g)?;
                let leaders = self.copy_list(self[g].leader_ptr)?;
                self[r].leader_ptr = leaders;
                r.ptr()
            }
            AnyNode::Kern(k) => self.copy_of(k)?.ptr(),
            AnyNode::Math(m) => self.copy_of(m)?.ptr(),
            AnyNode::Penalty(pen) => self.copy_of(pen)?.ptr(),
            AnyNode::MarginKern(m) => self.copy_of(m)?.ptr(),
            AnyNode::Ligature(lig) => {
                let r = self.copy_of(lig)?;
                let components = self.copy_list(self[lig].lig_ptr)?;
                self[r].lig_ptr = components;
                r.ptr()
            }
            AnyNode::Disc(disc) => {
                let r = self.copy_of(disc)?;
                let pre_break = self.copy_list(self[disc].pre_break)?;
                self[r].pre_break = pre_break;
                let post_break = self.copy_list(self[disc].post_break)?;
                self[r].post_break = post_break;
                r.ptr()
            }
            AnyNode::Mark(mark) => {
                self.tokens.add_ref(self[mark].mark_ptr);
                self.copy_of(mark)?.ptr()
            }
            AnyNode::Adjust(adjust) => {
                let r = self.copy_of(adjust)?;
                let list = self.copy_list(self[adjust].adjust_ptr)?;
                self[r].adjust_ptr = list;
                r.ptr()
            }
            _ => return Err(Abort::Confusion("copying")),
        })
    }
}
