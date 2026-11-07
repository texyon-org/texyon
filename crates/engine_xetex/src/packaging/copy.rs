//! Copying and destroying node lists: `copy_node_list`, `flush_node_list`, `just_copy`.

use crate::consts::{
    BOX_NODE_SIZE, GLYPH_NODE_SIZE, INS_NODE_SIZE, MARGIN_KERN_NODE_SIZE, MEDIUM_NODE_SIZE,
    MIN_HALFWORD, PIC_NODE_SIZE, RULE_NODE_SIZE, R_CODE, SMALL_NODE_SIZE, STYLE_NODE_SIZE, SUB_BOX,
};
use crate::errors::Abort;
use crate::mem::{end_lr_type, TEMP_HEAD};
use crate::node::{
    AnyNode, BoxNode, DiscNode, FixedNode, GlueNode, LanguageNode, NoadVariant, NodeKind, NodePtr,
    OpenNode, WhatsitNode, WhatsitVariant, WriteNode,
};
use crate::state::{MemState, ScannerState};
use crate::units::{Dir, MemIdx, MemPtr, Scaled};

const SYNCTEX_FIELD_SIZE: i32 = 1;
const MEMORY_WORD_SIZE: i32 = 8;

/// `r` typed like `src`, whose header word `r` has just received.
#[inline]
fn copied_node<K: NodeKind>(_src: NodePtr<K>, r: MemPtr) -> NodePtr<K> {
    NodePtr::from_idx_unchecked(MemIdx::new(r))
}

impl MemState {
    pub(super) fn as_disc(&self, p: MemPtr) -> Option<NodePtr<DiscNode>> {
        if self.is_char_node(p) {
            return None;
        }
        self.classify_disc(p).ok().flatten()
    }

    /// The size of whatsit `w`, `None` for an unknown subtype.
    fn whatsit_node_size(&self, w: NodePtr<WhatsitNode>) -> Option<i32> {
        Some(match w.variant(self)? {
            WhatsitVariant::Open(_) => OpenNode::SIZE,
            WhatsitVariant::Write(_) | WhatsitVariant::Close(_) | WhatsitVariant::Special(_) => {
                WriteNode::SIZE
            }
            WhatsitVariant::Language(_) => LanguageNode::SIZE,
            WhatsitVariant::SavePos => SMALL_NODE_SIZE,
            WhatsitVariant::NativeWord(word) => i32::from(self[word].size),
            WhatsitVariant::Glyph(_) => GLYPH_NODE_SIZE,
            WhatsitVariant::Pic(pic) => {
                PIC_NODE_SIZE
                    + (i32::from(self[pic].path_len) + MEMORY_WORD_SIZE - 1) / MEMORY_WORD_SIZE
            }
        })
    }

    /// Copy the first `words` words of node `src` to node `dst`.
    #[inline]
    fn copy_words(&mut self, dst: MemPtr, src: MemPtr, words: i32) {
        for w in (0..words).rev() {
            self.set_word(dst + w, self.word(src + w));
        }
    }

    fn copy_box_shell(
        &mut self,
        src: NodePtr<BoxNode>,
        list: MemPtr,
    ) -> Result<NodePtr<BoxNode>, Abort> {
        let r = self.get_node(BOX_NODE_SIZE)?;
        self.copy_words(r, src.ptr(), BOX_NODE_SIZE);
        let r = copied_node(src, r);
        self[r].list_ptr = list;
        Ok(r)
    }

    /// A copy of glue node `p` sharing its spec. The leader pointer is left to the caller.
    fn copy_glue_shell(&mut self, p: NodePtr<GlueNode>) -> Result<NodePtr<GlueNode>, Abort> {
        let spec = self[p].glue_ptr;
        let r = self.get_node(MEDIUM_NODE_SIZE)?;
        self.add_glue_ref(spec);
        let sync = MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE;
        self.set_word(r, self.word(p.ptr()));
        self.set_word(r + sync, self.word(p.ptr() + sync));
        let r = copied_node(p, r);
        self[r].glue_ptr = spec;
        Ok(r)
    }

    /// Allocate a copy of whatsit `w`, returning it and how many of its leading
    /// words are still to be copied.
    fn copy_whatsit(&mut self, w: NodePtr<WhatsitNode>) -> Result<(MemPtr, i32), Abort> {
        let Some(size) = self.whatsit_node_size(w) else {
            return Err(Abort::Confusion("ext2"));
        };
        let r = self.get_node(size)?;
        match w.variant(self) {
            Some(WhatsitVariant::NativeWord(word)) => {
                self.copy_words(r, word.ptr(), size);
                let copy = copied_node(word, r);
                self[copy].clear_glyphs();
                copy.share_native_glyphs(self, word);
                Ok((r, 0))
            }
            Some(WhatsitVariant::Write(p) | WhatsitVariant::Special(p)) => {
                self.add_token_ref(self[p].tokens);
                Ok((r, size))
            }
            Some(WhatsitVariant::SavePos) => Ok((r, 1)),
            _ => Ok((r, size)),
        }
    }

    /// Copy the list `p` without the contents of its boxes, linking the copies
    /// after `h` and ending them with `t`.
    pub fn just_copy(&mut self, mut p: MemPtr, mut h: MemPtr, t: MemPtr) -> Result<(), Abort> {
        while !p.is_null() {
            let copy = if self.is_char_node(p) {
                let r = self.get_avail()?;
                Some((r, 1))
            } else {
                match self.classify(p).ok() {
                    Some(AnyNode::Box(b)) => Some((self.copy_box_shell(b, MemPtr::NULL)?.ptr(), 0)),
                    Some(AnyNode::Rule(_)) => {
                        Some((self.get_node(RULE_NODE_SIZE)?, RULE_NODE_SIZE))
                    }
                    Some(AnyNode::Ligature(lig)) => {
                        let r = self.get_avail()?;
                        self.set_word(r, self.word(lig.ptr() + 1));
                        Some((r, 0))
                    }
                    Some(AnyNode::Kern(_) | AnyNode::Math(_)) => {
                        Some((self.get_node(MEDIUM_NODE_SIZE)?, MEDIUM_NODE_SIZE))
                    }
                    Some(AnyNode::Glue(g)) => {
                        let r = self.copy_glue_shell(g)?;
                        self[r].leader_ptr = MemPtr::NULL;
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
    pub fn just_reverse(&mut self, scanner: &mut ScannerState, p: MemPtr) -> Result<(), Abort> {
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

        let t = self.new_edge(scanner.cur_dir, Scaled::ZERO)?;
        let mut l = t.ptr();
        scanner.cur_dir = scanner.cur_dir.reversed();

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
                    let subtype = self[math].subtype;
                    if subtype % 2 == 1 {
                        if scanner.lr_ptr.info(self) != end_lr_type(subtype) {
                            math.into_kern(self);
                            scanner.lr_problems += 1;
                        } else {
                            self.pop_lr(&mut scanner.lr_ptr);

                            if n > MIN_HALFWORD {
                                n -= 1;
                                self[math].subtype = subtype - 1;
                            } else if m > MIN_HALFWORD {
                                m -= 1;
                                math.into_kern(self);
                            } else {
                                self[t].width = self[math].width;
                                self[t].link = q;
                                self.free_node(p, MEDIUM_NODE_SIZE);
                                break 'done;
                            }
                        }
                    } else {
                        self.push_lr(&mut scanner.lr_ptr, end_lr_type(subtype))?;

                        if n > MIN_HALFWORD
                            || Dir::from_code(subtype / R_CODE) != Some(scanner.cur_dir)
                        {
                            n += 1;
                            self[math].subtype = subtype + 1;
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
            if self.is_char_node(p) {
                self.free_avail(p);
                p = q;
                continue;
            }

            let size = match self.classify(p).map_err(|_| Abort::Confusion("flushing"))? {
                AnyNode::Box(b) => {
                    self.flush_node_list(self[b].list_ptr)?;
                    BOX_NODE_SIZE
                }
                AnyNode::Unset(u) => {
                    self.flush_node_list(self[u].list_ptr)?;
                    BOX_NODE_SIZE
                }
                AnyNode::Rule(_) => RULE_NODE_SIZE,
                AnyNode::Ins(ins) => {
                    self.flush_node_list(self[ins].ins_ptr)?;
                    self.delete_glue_ref(self[ins].split_top_ptr);
                    INS_NODE_SIZE
                }
                AnyNode::Whatsit(w) => {
                    let size = self.whatsit_node_size(w).ok_or(Abort::Confusion("ext3"))?;
                    match w.variant(self) {
                        Some(WhatsitVariant::Write(p) | WhatsitVariant::Special(p)) => {
                            self.delete_token_ref(self[p].tokens);
                        }
                        Some(WhatsitVariant::NativeWord(word)) => word.release_native_glyphs(self),
                        _ => {}
                    }
                    size
                }
                AnyNode::Glue(glue) => {
                    self.delete_glue_ref(self[glue].glue_ptr);
                    self.flush_node_list(self[glue].leader_ptr)?;
                    MEDIUM_NODE_SIZE
                }
                AnyNode::Kern(_) | AnyNode::Math(_) | AnyNode::Penalty(_) => MEDIUM_NODE_SIZE,
                AnyNode::MarginKern(_) => MARGIN_KERN_NODE_SIZE,
                AnyNode::Ligature(lig) => {
                    self.flush_node_list(self[lig].lig_ptr)?;
                    SMALL_NODE_SIZE
                }
                AnyNode::Mark(mark) => {
                    self.delete_token_ref(self[mark].mark_ptr);
                    SMALL_NODE_SIZE
                }
                AnyNode::Disc(disc) => {
                    self.flush_node_list(self[disc].pre_break)?;
                    self.flush_node_list(self[disc].post_break)?;
                    SMALL_NODE_SIZE
                }
                AnyNode::Adjust(adjust) => {
                    self.flush_node_list(self[adjust].adjust_ptr)?;
                    SMALL_NODE_SIZE
                }
                AnyNode::Style(_) => STYLE_NODE_SIZE,
                AnyNode::Choice(choice) => {
                    self.flush_node_list(self[choice].display_mlist)?;
                    self.flush_node_list(self[choice].text_mlist)?;
                    self.flush_node_list(self[choice].script_mlist)?;
                    self.flush_node_list(self[choice].script_script_mlist)?;
                    STYLE_NODE_SIZE
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
                    v.size()
                }
            };
            self.free_node(p, size);
            p = q;
        }
        Ok(())
    }

    /// A deep copy of the list `p`.
    /// A copy of box `b` and its contents.
    pub(crate) fn copy_box(&mut self, b: NodePtr<BoxNode>) -> Result<NodePtr<BoxNode>, Abort> {
        let r = self.copy_list(b.ptr())?;
        Ok(copied_node(b, r))
    }

    pub fn copy_node_list(&mut self, p: MemPtr) -> Result<MemPtr, Abort> {
        self.copy_list(p)
    }

    fn copy_list(&mut self, mut p: MemPtr) -> Result<MemPtr, Abort> {
        let h = self.get_avail()?;
        let mut q = h;
        while !p.is_null() {
            let (r, words) = if self.is_char_node(p) {
                (self.get_avail()?, 1)
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
    fn copy_node(&mut self, p: MemPtr) -> Result<(MemPtr, i32), Abort> {
        let node = self.classify(p).map_err(|_| Abort::Confusion("copying"))?;
        Ok(match node {
            AnyNode::Box(b) => {
                let list = self.copy_list(self[b].list_ptr)?;
                (self.copy_box_shell(b, list)?.ptr(), 0)
            }
            AnyNode::Unset(u) => {
                let r = self.get_node(BOX_NODE_SIZE)?;
                self.copy_words(r, p, BOX_NODE_SIZE);
                let list = self.copy_list(self[u].list_ptr)?;
                self[copied_node(u, r)].list_ptr = list;
                (r, 0)
            }
            AnyNode::Rule(_) => (
                self.get_node(RULE_NODE_SIZE)?,
                RULE_NODE_SIZE - SYNCTEX_FIELD_SIZE,
            ),
            AnyNode::Ins(ins) => {
                let r = self.get_node(INS_NODE_SIZE)?;
                self.copy_words(r, p, INS_NODE_SIZE);
                self.add_glue_ref(self[ins].split_top_ptr);
                let list = self.copy_list(self[ins].ins_ptr)?;
                self[copied_node(ins, r)].ins_ptr = list;
                (r, 0)
            }
            AnyNode::Whatsit(w) => self.copy_whatsit(w)?,
            AnyNode::Glue(g) => {
                let r = self.copy_glue_shell(g)?;
                let leaders = self.copy_list(self[g].leader_ptr)?;
                self[r].leader_ptr = leaders;
                (r.ptr(), 0)
            }
            AnyNode::Kern(_) | AnyNode::Math(_) | AnyNode::Penalty(_) => {
                (self.get_node(MEDIUM_NODE_SIZE)?, MEDIUM_NODE_SIZE)
            }
            AnyNode::MarginKern(_) => {
                (self.get_node(MARGIN_KERN_NODE_SIZE)?, MARGIN_KERN_NODE_SIZE)
            }
            AnyNode::Ligature(lig) => {
                let r = self.get_node(SMALL_NODE_SIZE)?;
                self.copy_words(r, p, SMALL_NODE_SIZE);
                let components = self.copy_list(self[lig].lig_ptr)?;
                self[copied_node(lig, r)].lig_ptr = components;
                (r, 0)
            }
            AnyNode::Disc(disc) => {
                let r = self.get_node(SMALL_NODE_SIZE)?;
                self.copy_words(r, p, 1);
                let copy = copied_node(disc, r);
                let pre_break = self.copy_list(self[disc].pre_break)?;
                self[copy].pre_break = pre_break;
                let post_break = self.copy_list(self[disc].post_break)?;
                self[copy].post_break = post_break;
                (r, 0)
            }
            AnyNode::Mark(mark) => {
                let r = self.get_node(SMALL_NODE_SIZE)?;
                self.add_token_ref(self[mark].mark_ptr);
                (r, SMALL_NODE_SIZE)
            }
            AnyNode::Adjust(adjust) => {
                let r = self.get_node(SMALL_NODE_SIZE)?;
                self.copy_words(r, p, 1);
                let list = self.copy_list(self[adjust].adjust_ptr)?;
                self[copied_node(adjust, r)].adjust_ptr = list;
                (r, 0)
            }
            _ => return Err(Abort::Confusion("copying")),
        })
    }
}
