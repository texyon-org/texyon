//! Copying and destroying node lists: `copy_node_list`, `flush_node_list`, `just_copy`.

use super::LrState;
use crate::base::consts::{MIN_HALFWORD, SUB_BOX};
use crate::base::units::{MemIdx, MemPtr, Scaled};
use crate::memory::mem::MemState;
use crate::memory::mem::TEMP_HEAD;
use crate::memory::node::CharNode;
use crate::memory::node::{
    AnyNode, BoxNode, DiscNode, GlueNode, HeadNode, MathNode, NoadVariant, Node, NodePtr, RuleNode,
    View, WhatsitNode, WhatsitVariant,
};
use crate::print::errors::Abort;
use crate::print::errors::OrConfusion;

/// `r` typed like `src`, whose header word `r` has just received.
#[inline]
fn copied_node<K: View>(_src: NodePtr<K>, r: MemIdx) -> NodePtr<K> {
    NodePtr::from_idx_unchecked(r)
}

impl MemState {
    pub(super) fn as_disc(&self, p: MemPtr) -> Option<NodePtr<DiscNode>> {
        let p = p.filter(|&p| !self.is_char_node(p))?;
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
    fn copy_whatsit(&mut self, w: NodePtr<WhatsitNode>) -> Result<MemIdx, Abort> {
        Ok(match w.variant(self).or_confusion("ext2")? {
            WhatsitVariant::Open(p) => self.copy_of(p)?.idx(),
            WhatsitVariant::Write(p)
            | WhatsitVariant::Special(p)
            | WhatsitVariant::LateSpecial(p) => {
                self.tokens.add_ref(self[p].tokens);
                self.copy_of(p)?.idx()
            }
            WhatsitVariant::Close(p) => self.copy_of(p)?.idx(),
            WhatsitVariant::Language(p) => self.copy_of(p)?.idx(),
            WhatsitVariant::SavePos(p) => self.copy_of(p)?.idx(),
            WhatsitVariant::NativeWord(word) => {
                let copy = self.copy_array(word)?;
                self[copy].clear_glyphs();
                copy.share_native_glyphs(self, word);
                copy.idx()
            }
            WhatsitVariant::Glyph(p) => self.copy_of(p)?.idx(),
            WhatsitVariant::Pic(pic) => self.copy_array(pic)?.idx(),
        })
    }

    /// Copy the list `p` without the contents of its boxes, linking the copies
    /// after `h` and ending them with `t`.
    pub fn just_copy(&mut self, mut p: MemPtr, mut h: MemIdx, t: MemPtr) -> Result<(), Abort> {
        while let Some(here) = p {
            let copy = if let Some(ch) = self.as_char(p) {
                let CharNode {
                    font, character, ..
                } = self[ch];
                Some(self.new_char(font, character)?.idx())
            } else {
                match self.classify(here).ok() {
                    Some(AnyNode::Box(b)) => Some(self.copy_box_shell(b, None)?.idx()),
                    Some(AnyNode::Rule(r)) => Some(self.copy_of(r)?.idx()),
                    Some(AnyNode::Ligature(lig)) => {
                        let (f, c) = (self[lig].font, self[lig].character);
                        Some(self.new_char(f, c)?.idx())
                    }
                    Some(AnyNode::Kern(k)) => Some(self.copy_of(k)?.idx()),
                    Some(AnyNode::Math(m)) => Some(self.copy_of(m)?.idx()),
                    Some(AnyNode::Glue(g)) => {
                        let r = self.copy_glue_shell(g)?;
                        self[r].leader_ptr = None;
                        Some(r.idx())
                    }
                    Some(AnyNode::Whatsit(w)) => Some(self.copy_whatsit(w)?),
                    _ => None,
                }
            };

            if let Some(r) = copy {
                h.set_link(self, Some(r));
                h = r;
            }
            p = here.link(self);
        }

        h.set_link(self, t);
        Ok(())
    }

    /// Reverse the hlist after `p` for TeX--XeT, leaving the result after `TEMP_HEAD`.
    pub(crate) fn just_reverse(&mut self, lr: &mut LrState, p: MemIdx) -> Result<(), Abort> {
        let mut m = MIN_HALFWORD;
        let mut n = MIN_HALFWORD;

        let mut q = if TEMP_HEAD.link(self).is_none() {
            self.just_copy(p.link(self), TEMP_HEAD, None)?;
            TEMP_HEAD.link(self)
        } else {
            let q = p.link(self);
            p.set_link(self, None);
            self.flush_node_list(TEMP_HEAD.link(self))?;
            q
        };

        let t = self.new_edge(lr.dir, Scaled::ZERO)?;
        let mut l = t.ptr();
        lr.dir = lr.dir.reversed();

        'done: {
            while let Some(p) = q {
                if self.is_char_node(p) {
                    while let Some(p) = q.filter(|&c| self.is_char_node(c)) {
                        q = p.link(self);
                        p.set_link(self, l);
                        l = Some(p);
                    }
                    continue;
                }

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
                                let width = self[math].width;
                                let node = &mut self[t];
                                node.width = width;
                                node.link = q;
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
                l = Some(p);
            }
        }

        TEMP_HEAD.set_link(self, l);
        Ok(())
    }

    /// Free every node of the list `p` and everything it owns.
    pub fn flush_node_list(&mut self, mut p: MemPtr) -> Result<(), Abort> {
        while let Some(here) = p {
            let q = here.link(self);
            match self.classify(here).or_confusion("flushing")? {
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
                        word.free_native(self);
                    }
                    WhatsitVariant::Glyph(p) => p.free(self),
                    WhatsitVariant::Pic(pic) => self.free_array(pic),
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
                    self.free_node(here, v.size());
                }
            }
            p = q;
        }
        Ok(())
    }

    /// A copy of box `b` and its contents.
    pub(crate) fn copy_box(&mut self, b: NodePtr<BoxNode>) -> Result<NodePtr<BoxNode>, Abort> {
        let r = self.copy_list(b.ptr())?.or_confusion("copy_box")?;
        Ok(copied_node(b, r))
    }

    /// A deep copy of the list `p`.
    pub fn copy_node_list(&mut self, p: MemPtr) -> Result<MemPtr, Abort> {
        self.copy_list(p)
    }

    fn copy_list(&mut self, mut p: MemPtr) -> Result<MemPtr, Abort> {
        let h = NodePtr::<HeadNode>::alloc(self)?;
        let mut q = h.idx();
        while let Some(here) = p {
            let r = self.copy_node(here)?;
            q.set_link(self, Some(r));
            q = r;
            p = here.link(self);
        }
        q.set_link(self, None);
        let result = self[h].link;
        h.free(self);
        Ok(result)
    }

    /// A copy of node `p` and everything it owns.
    fn copy_node(&mut self, p: MemIdx) -> Result<MemIdx, Abort> {
        let node = self.classify(p).or_confusion("copying")?;
        Ok(match node {
            AnyNode::Char(ch) => {
                let CharNode {
                    font, character, ..
                } = self[ch];
                self.new_char(font, character)?.idx()
            }
            AnyNode::Box(b) => {
                let list = self.copy_list(self[b].list_ptr)?;
                self.copy_box_shell(b, list)?.idx()
            }
            AnyNode::Unset(u) => {
                let r = self.copy_of(u)?;
                let list = self.copy_list(self[u].list_ptr)?;
                self[r].list_ptr = list;
                r.idx()
            }
            AnyNode::Rule(rule) => {
                // The copy keeps its own SyncTeX stamp.
                let r = NodePtr::<RuleNode>::alloc(self)?;
                let (sync_tag, sync_line) = (self[r].sync.tag, self[r].sync.line);
                self[r] = self[rule];
                (self[r].sync.tag, self[r].sync.line) = (sync_tag, sync_line);
                r.idx()
            }
            AnyNode::Ins(ins) => {
                let r = self.copy_of(ins)?;
                self.add_glue_ref(self[ins].split_top_ptr);
                let list = self.copy_list(self[ins].ins_ptr)?;
                self[r].ins_ptr = list;
                r.idx()
            }
            AnyNode::Whatsit(w) => self.copy_whatsit(w)?,
            AnyNode::Glue(g) => {
                let r = self.copy_glue_shell(g)?;
                let leaders = self.copy_list(self[g].leader_ptr)?;
                self[r].leader_ptr = leaders;
                r.idx()
            }
            AnyNode::Kern(k) => self.copy_of(k)?.idx(),
            AnyNode::Math(m) => self.copy_of(m)?.idx(),
            AnyNode::Penalty(pen) => self.copy_of(pen)?.idx(),
            AnyNode::MarginKern(m) => self.copy_of(m)?.idx(),
            AnyNode::Ligature(lig) => {
                let r = self.copy_of(lig)?;
                let components = self.copy_list(self[lig].lig_ptr)?;
                self[r].lig_ptr = components;
                r.idx()
            }
            AnyNode::Disc(disc) => {
                let r = self.copy_of(disc)?;
                let pre_break = self.copy_list(self[disc].pre_break)?;
                self[r].pre_break = pre_break;
                let post_break = self.copy_list(self[disc].post_break)?;
                self[r].post_break = post_break;
                r.idx()
            }
            AnyNode::Mark(mark) => {
                self.tokens.add_ref(self[mark].mark_ptr);
                self.copy_of(mark)?.idx()
            }
            AnyNode::Adjust(adjust) => {
                let r = self.copy_of(adjust)?;
                let list = self.copy_list(self[adjust].adjust_ptr)?;
                self[r].adjust_ptr = list;
                r.idx()
            }
            _ => return Err(Abort::Confusion("copying")),
        })
    }
}
