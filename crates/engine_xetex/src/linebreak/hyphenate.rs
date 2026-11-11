//! Post-hyphenation: `hyphenate`, `reconstitute` and native-word hyphens.

use crate::consts::TEX_NULL;
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::fonts::{CharTag, LigChar};
use crate::hyphenation::TrieOp;
use crate::main_control::{LigStack, NewChar};
use crate::mem::HOLD_HEAD;
use crate::node::{
    AnyNode, CharNode, DiscNode, HyphPosNode, LigBoundary, LigItem, LigatureNode, NativeWordNode,
    NodePtr, View,
};
use crate::params::IntPar;
use crate::state::{EngineState, EqtbState, FontState, MemState};
use crate::units::{FontId, FontInfoIdx, MemPtr, Scaled, StrNumber};

use super::odd;
use super::prehyph::Word;

impl MemState {
    /// The Unicode scalar at UTF-16 offset `i` of `w`'s text.
    pub(super) fn get_native_usv(&self, w: NodePtr<NativeWordNode>, i: usize) -> i32 {
        let text = self.entries(w);
        let c = text[i];

        if (0xD800..0xDC00).contains(&c) {
            let lo = text[i + 1];
            return 0x10000 + (i32::from(c) - 0xD800) * 0x400 + i32::from(lo) - 0xDC00;
        }

        i32::from(c)
    }

    /// Copy `len` UTF-16 units of `src`'s text, starting at `start`, into `dst`.
    fn copy_native_text(
        &mut self,
        dst: NodePtr<NativeWordNode>,
        src: NodePtr<NativeWordNode>,
        start: usize,
        len: usize,
    ) -> Result<(), Abort> {
        if len == 0 {
            return Ok(());
        }
        let (to, from) = self.entries_mut_and(dst, src)?;
        to[..len].copy_from_slice(&from[start..start + len]);
        Ok(())
    }

    pub(crate) fn set_native_metrics(
        &mut self,
        eqtb: &EqtbState,
        fonts: &FontState,
        w: NodePtr<NativeWordNode>,
    ) -> Result<(), Abort> {
        let use_glyph_metrics = eqtb.int_par(IntPar::XetexUseGlyphMetrics) > 0;
        fonts.measure_native_node(self, w, use_glyph_metrics)
    }

    /// A native-word node of font `f` holding `len` units of `src`'s text from `start`.
    fn native_word_fragment(
        &mut self,
        eqtb: &EqtbState,
        fonts: &FontState,
        f: FontId,
        src: NodePtr<NativeWordNode>,
        start: usize,
        len: usize,
    ) -> Result<NodePtr<NativeWordNode>, Abort> {
        let q = self.new_native_word_node(eqtb, f, len as i32)?;
        self[q].subtype = self[src].subtype;
        self.copy_native_text(q, src, start, len)?;
        self.set_native_metrics(eqtb, fonts, q)?;
        Ok(q)
    }
}

/// Split the native word `ha` at UTF-16 offset `start`, returning the tail
/// linked after it.
pub(super) fn split_native_word(
    st: &mut EngineState,
    ha: NodePtr<NativeWordNode>,
    start: usize,
) -> Result<NodePtr<NativeWordNode>, Abort> {
    let len = usize::from(st.mem[ha].length);
    let tail =
        st.mem
            .native_word_fragment(&st.eqtb, &st.fonts, st.hyph.hf, ha, start, len - start)?;
    let next = st.mem[ha].link;
    st.mem[tail].link = next;
    let node = &mut st.mem[ha];
    node.link = tail.ptr();
    node.length = start as u16;
    st.mem.set_native_metrics(&st.eqtb, &st.fonts, ha)?;
    Ok(tail)
}

/// The top of `mainloop.lig_stack` as a lig item, which is all `reconstitute` pushes.
#[inline]
fn top_item(st: &EngineState) -> Option<NodePtr<LigItem>> {
    match st.mainloop.lig_stack? {
        LigStack::Item(item) => Some(item),
        LigStack::Char(c) => Some(NodePtr::at(c.ptr())),
    }
}

impl MemState {
    /// A fresh char node of font `f` holding `c`, linked after `t`.
    fn append_char(&mut self, t: MemPtr, f: FontId, c: u16) -> Result<MemPtr, Abort> {
        let q = self.new_char(f, c)?;
        t.set_link(self, q.ptr());
        Ok(q.ptr())
    }
}

/// Load `cur_r` for position `j` and return the hyphen char still to be tried there.
fn update_right(
    st: &mut EngineState,
    j: usize,
    n: usize,
    bchar: LigChar,
    hchar: LigChar,
) -> LigChar {
    st.mainloop.cur_r = if j < n { st.hyph.hu[j + 1] } else { bchar };
    if odd(st.hyph.hyf[j]) {
        hchar
    } else {
        LigChar::NONE
    }
}

/// Pop the top lig item, appending its char node after `t`, and load the next `cur_r`.
#[allow(clippy::too_many_arguments)]
fn pop_lig_stack(
    st: &mut EngineState,
    t: &mut MemPtr,
    j: &mut usize,
    n: usize,
    bchar: LigChar,
    hchar: LigChar,
    cur_rh: &mut LigChar,
) {
    let Some(item) = top_item(st) else {
        return;
    };
    let component = st.mem[item].component;
    if component.raw() > TEX_NULL {
        t.set_link(&mut st.mem, component);
        *t = component;
        *j += 1;
    }
    st.mainloop.lig_stack = st.mem.lig_stack_at(item.ptr().link(&st.mem));
    item.free(&mut st.mem);
    match st.mainloop.lig_stack {
        None => *cur_rh = update_right(st, *j, n, bchar, hchar),
        Some(top) => st.mainloop.cur_r = top.character(&st.mem),
    }
}

/// Rebuild `hu[j..]` after `HOLD_HEAD` as chars, ligatures and kerns, returning
/// the index of the last character consumed.
fn reconstitute(
    st: &mut EngineState,
    mut j: usize,
    n: usize,
    mut bchar: LigChar,
    mut hchar: LigChar,
) -> Result<usize, Abort> {
    st.hyph.hyphen_passed = 0;
    let mut t = HOLD_HEAD;
    let mut w = Scaled::ZERO;
    HOLD_HEAD.set_link(&mut st.mem, MemPtr::NULL);
    st.mainloop.cur_l = st.hyph.hu[j];
    st.mainloop.cur_q = t;

    if j == 0 {
        st.mainloop.ligature_present = st.hyph.init_lig;
        let mut p = st.hyph.init_list;
        if st.mainloop.ligature_present {
            st.mainloop.lft_hit = st.hyph.init_lft;
        }
        while p.raw() > TEX_NULL {
            let c = st.mem[NodePtr::<CharNode>::at(p)].character;
            t = st.mem.append_char(t, st.hyph.hf, c)?;
            p = p.link(&st.mem);
        }
    } else if st.mainloop.cur_l.is_char() {
        t = st
            .mem
            .append_char(t, st.hyph.hf, st.mainloop.cur_l.code())?;
    }

    st.mainloop.lig_stack = None;
    let mut cur_rh = update_right(st, j, n, bchar, hchar);

    loop {
        'continue_: loop {
            let (mut k, mut q) = if st.mainloop.cur_l == LigChar::NONE {
                let k = st.fonts.bchar_label[st.hyph.hf];
                if k == FontInfoIdx::NON_ADDRESS {
                    break 'continue_;
                }
                (k, st.fonts.lig_kern_step(k))
            } else {
                let info = st
                    .fonts
                    .effective_char_info(st.hyph.hf, st.mainloop.cur_l.code());
                if info.tag() != CharTag::Lig {
                    break 'continue_;
                }
                st.fonts.lig_kern_start(st.hyph.hf, info)
            };

            let mut test_char = if cur_rh.is_char() {
                cur_rh
            } else {
                st.mainloop.cur_r
            };

            loop {
                if q.matches(test_char) {
                    if cur_rh.is_char() {
                        st.hyph.hyphen_passed = j;
                        hchar = LigChar::NONE;
                        cur_rh = LigChar::NONE;
                        continue 'continue_;
                    }

                    if hchar.is_char() && odd(st.hyph.hyf[j]) {
                        st.hyph.hyphen_passed = j;
                        hchar = LigChar::NONE;
                    }

                    if !q.is_kern() {
                        if st.mainloop.cur_l == LigChar::NONE {
                            st.mainloop.lft_hit = true;
                        }
                        if j == n && st.mainloop.lig_stack.is_none() {
                            st.mainloop.rt_hit = true;
                        }

                        match q.lig_op() {
                            1 | 5 => {
                                st.mainloop.cur_l = LigChar::of(q.lig_char());
                                st.mainloop.ligature_present = true;
                            }
                            2 | 6 => {
                                st.mainloop.cur_r = LigChar::of(q.lig_char());
                                if let Some(top) = top_item(st) {
                                    st.mem[top].character = st.mainloop.cur_r.code();
                                } else {
                                    let item = st.mem.new_lig_item(st.mainloop.cur_r.code())?;
                                    st.mainloop.lig_stack = Some(LigStack::Item(item));
                                    if j == n {
                                        bchar = LigChar::NONE;
                                    } else {
                                        let p = st
                                            .mem
                                            .new_char(st.hyph.hf, st.hyph.hu[j + 1].code())?;
                                        st.mem[item].component = p.ptr();
                                    }
                                }
                            }
                            3 => {
                                st.mainloop.cur_r = LigChar::of(q.lig_char());
                                let below =
                                    st.mainloop.lig_stack.map_or(MemPtr::NULL, LigStack::ptr);
                                let item = st.mem.new_lig_item(st.mainloop.cur_r.code())?;
                                st.mainloop.lig_stack = Some(LigStack::Item(item));
                                item.ptr().set_link(&mut st.mem, below);
                            }
                            7 | 11 => {
                                if st.mainloop.ligature_present {
                                    t = wrap_lig(st, false)?;
                                }
                                st.mainloop.cur_q = t;
                                st.mainloop.cur_l = LigChar::of(q.lig_char());
                                st.mainloop.ligature_present = true;
                            }
                            _ => {
                                st.mainloop.cur_l = LigChar::of(q.lig_char());
                                st.mainloop.ligature_present = true;
                                if st.mainloop.lig_stack.is_some() {
                                    pop_lig_stack(st, &mut t, &mut j, n, bchar, hchar, &mut cur_rh);
                                } else if j == n {
                                    break 'continue_;
                                } else {
                                    t = st.mem.append_char(
                                        t,
                                        st.hyph.hf,
                                        st.mainloop.cur_r.code(),
                                    )?;
                                    j += 1;
                                    cur_rh = update_right(st, j, n, bchar, hchar);
                                }
                            }
                        }

                        if q.lig_op() > 4 && q.lig_op() != 7 {
                            break 'continue_;
                        }
                        continue 'continue_;
                    }

                    w = st.fonts.char_kern(st.hyph.hf, q);
                    break 'continue_;
                }

                if q.is_last() {
                    if cur_rh == LigChar::NONE {
                        break 'continue_;
                    }
                    cur_rh = LigChar::NONE;
                    continue 'continue_;
                }

                k = q.next_index(k);
                q = st.fonts.lig_kern_step(k);
                test_char = if cur_rh.is_char() {
                    cur_rh
                } else {
                    st.mainloop.cur_r
                };
            }
        }

        if st.mainloop.ligature_present {
            t = wrap_lig(st, true)?;
        }

        if w != Scaled::ZERO {
            let kern = st.mem.new_kern(w)?;
            t.set_link(&mut st.mem, kern.ptr());
            t = kern.ptr();
            w = Scaled::ZERO;
            st.mem[kern].sync.tag = 0;
        }

        if let Some(top) = st.mainloop.lig_stack {
            st.mainloop.cur_q = t;
            st.mainloop.cur_l = top.character(&st.mem);
            st.mainloop.ligature_present = true;
            pop_lig_stack(st, &mut t, &mut j, n, bchar, hchar, &mut cur_rh);
            continue;
        }

        return Ok(j);
    }
}

/// Replace the chars after `cur_q` by a ligature of `cur_l`, returning it.
/// `rt` also records a right boundary hit when the lig stack is empty.
fn wrap_lig(st: &mut EngineState, rt: bool) -> Result<MemPtr, Abort> {
    let components = st.mainloop.cur_q.link(&st.mem);
    let lig = st
        .mem
        .new_ligature(st.hyph.hf, st.mainloop.cur_l.code(), components)?;
    let mut boundary = LigBoundary::None;
    if st.mainloop.lft_hit {
        boundary = LigBoundary::Left;
        st.mainloop.lft_hit = false;
    }
    if rt && st.mainloop.rt_hit && st.mainloop.lig_stack.is_none() {
        boundary = boundary.with_right();
        st.mainloop.rt_hit = false;
    }
    st.mem[lig].boundary = boundary.into();
    st.mainloop.cur_q.set_link(&mut st.mem, lig.ptr());
    st.mainloop.ligature_present = false;
    Ok(lig.ptr())
}

/// Hyphenate the native word `ha` by splitting it into fragments separated by
/// discretionaries.
fn hyphenate_native_word(
    st: &mut EngineState,
    word: &Word,
    ha: NodePtr<NativeWordNode>,
    hn: usize,
) -> Result<(), Abort> {
    let hf = st.hyph.hf;
    let hyf_char = st.hyph.hyf_char;
    let mut s = st.mem.predecessor(word.start, ha.ptr());

    st.hyph.hyphen_passed = 0;
    for j in st.hyph.l_hyf as usize..=hn - st.hyph.r_hyf as usize {
        if odd(st.hyph.hyf[j]) {
            let hp = st.hyph.hyphen_passed;
            let q = st
                .mem
                .native_word_fragment(&st.eqtb, &st.fonts, hf, ha, hp, j - hp)?;
            s.set_link(&mut st.mem, q.ptr());

            let disc = st.mem.new_disc()?;
            let hyphen = crate::fonts::new_native_character(st, hf, u32::from(hyf_char.code()))?;
            st.mem[disc].pre_break = hyphen.ptr();
            st.mem[q].link = disc.ptr();
            s = disc.ptr();
            st.hyph.hyphen_passed = j;
        }
    }

    let hn = st.mem[ha].length;
    let hp = st.hyph.hyphen_passed;
    let q = st
        .mem
        .native_word_fragment(&st.eqtb, &st.fonts, hf, ha, hp, usize::from(hn) - hp)?;
    s.set_link(&mut st.mem, q.ptr());
    let next = st.mem[ha].link;
    st.mem[q].link = next;
    st.mem[ha].link = MemPtr::NULL;
    st.mem.flush_node_list(ha.ptr())
}

/// Find the hyphen positions of the word in `hc[1..=hn]` (from the exception
/// dictionary or the patterns) and insert the discretionaries. `native_ha` is
/// the word's node when it is a native word.
pub(super) fn hyphenate(
    st: &mut EngineState,
    word: &Word,
    native_ha: Option<NodePtr<NativeWordNode>>,
) -> Result<(), Abort> {
    let hn = usize::from(word.hn);
    st.hyph.hyf[..=hn].fill(0);

    st.hyph.hc[hn + 1] = i32::from(st.hyph.cur_lang);
    let mut h = st.hyph.exception_hash(hn + 1);
    loop {
        let k = st.hyph.hyph_word[h];
        if k == StrNumber::NONE {
            break;
        }

        if st.strings.length(k) == (hn + 1) as i32
            && st.strings.str_pool[st.strings.str_range(k)]
                .iter()
                .zip(&st.hyph.hc[1..=hn + 1])
                .all(|(&u, &c)| i32::from(u) == c)
        {
            let mut s = NodePtr::<HyphPosNode>::from_ptr(st.hyph.hyph_list[h]);
            while let Some(pos) = s {
                let HyphPosNode { pos, link, .. } = st.mem[pos];
                st.hyph.hyf[usize::try_from(pos).or_confusion("hyphen position")?] = 1;
                s = NodePtr::from_ptr(link);
            }
            return insert_hyphens(st, word, hn, native_ha);
        }

        let Some(next) = st.hyph.hyph_chain_next(h) else {
            break;
        };
        h = next;
    }

    let Some(root) = st.hyph.pattern_root(st.hyph.cur_lang) else {
        return Ok(());
    };

    let hyph = &mut st.hyph;
    hyph.hc[0] = 0;
    hyph.hc[hn + 1] = 0;
    hyph.hc[hn + 2] = hyph.max_hyph_char;
    let op_base = hyph.op_start[usize::from(hyph.cur_lang)];

    for j in 0..=hn + 1 - hyph.r_hyf as usize {
        let mut z = root.plus(hyph.hc[j] as usize);
        let mut l = j;
        while hyph.hc[l] == i32::from(hyph.trie_char(z)) {
            let mut v = hyph.trie_op(z);
            while v != TrieOp::NONE {
                let op = hyph.hyph_op(op_base, v);
                let i = l - usize::from(op.distance);
                if op.num > hyph.hyf[i] {
                    hyph.hyf[i] = op.num;
                }
                v = op.next;
            }
            l += 1;
            z = hyph.trie_link(z).plus(hyph.hc[l] as usize);
        }
    }

    insert_hyphens(st, word, hn, native_ha)
}

impl MemState {
    /// Append the list after `HOLD_HEAD` to `disc`'s pre- or post-break list,
    /// whose current tail is `minor_tail`.
    fn append_to_disc_list(
        &mut self,
        disc: NodePtr<DiscNode>,
        post: bool,
        minor_tail: &mut MemPtr,
    ) {
        let hold = HOLD_HEAD.link(self);
        if hold.raw() <= TEX_NULL {
            return;
        }
        if !minor_tail.is_null() {
            minor_tail.set_link(self, hold);
        } else if post {
            self[disc].post_break = hold;
        } else {
            self[disc].pre_break = hold;
        }
        *minor_tail = self.list_tail(hold);
    }
}

/// Replace the word `ha..hb` by a reconstituted list with a discretionary at
/// every odd `hyf` position.
fn insert_hyphens(
    st: &mut EngineState,
    word: &Word,
    hn: usize,
    native_ha: Option<NodePtr<NativeWordNode>>,
) -> Result<(), Abort> {
    let l_hyf = st.hyph.l_hyf as usize;
    let r_hyf = st.hyph.r_hyf as usize;
    st.hyph.hyf[..l_hyf].fill(0);
    st.hyph.hyf[hn + 1 - r_hyf..=hn].fill(0);

    if !st.hyph.hyf[l_hyf..=hn - r_hyf].iter().any(|&h| odd(h)) {
        return Ok(());
    }

    if let Some(ha) = native_ha {
        return hyphenate_native_word(st, word, ha, hn);
    }

    let hb = word.hb;
    let ha = word.ha;
    let q_final = hb.link(&st.mem);
    hb.set_link(&mut st.mem, MemPtr::NULL);
    let r = ha.link(&st.mem);
    ha.set_link(&mut st.mem, MemPtr::NULL);
    let bchar = st.hyph.hyf_bchar;

    let found2 = |st: &mut EngineState| {
        st.hyph.hu[0] = LigChar::from_raw(st.hyph.max_hyph_char);
        st.hyph.init_lig = false;
        st.hyph.init_list = MemPtr::NULL;
        (ha, 0)
    };
    let (mut s, mut j) = if let Some(ha_ch) = st.mem.as_char(ha) {
        if st.mem[ha_ch].font != st.hyph.hf {
            found2(st)
        } else {
            st.hyph.init_list = ha;
            st.hyph.init_lig = false;
            st.hyph.hu[0] = LigChar::of(st.mem[ha_ch].character);
            (st.mem.predecessor(word.start, ha), 0)
        }
    } else if let Ok(AnyNode::Ligature(ha_lig)) = st.mem.classify(ha) {
        if st.mem[ha_lig].font != st.hyph.hf {
            found2(st)
        } else {
            st.hyph.init_list = st.mem[ha_lig].lig_ptr;
            st.hyph.init_lig = true;
            st.hyph.init_lft = st.mem[ha_lig].boundary.has_left();
            st.hyph.hu[0] = LigChar::of(st.mem[ha_lig].character);
            if st.hyph.init_list.is_null() && st.hyph.init_lft {
                st.hyph.hu[0] = LigChar::from_raw(st.hyph.max_hyph_char);
                st.hyph.init_lig = false;
            }
            st.mem.free_node(ha, LigatureNode::SIZE);
            (st.mem.predecessor(word.start, ha), 0)
        }
    } else if !st.mem.is_char_node(r)
        && matches!(
            st.mem.classify(r),
            Ok(AnyNode::Ligature(lig)) if st.mem[lig].boundary.has_left()
        )
    {
        found2(st)
    } else {
        st.hyph.init_list = MemPtr::NULL;
        (ha, 1)
    };

    st.mem.flush_node_list(r)?;

    loop {
        let mut l = j;
        j = reconstitute(st, j, hn, bchar, st.hyph.hyf_char)? + 1;
        if st.hyph.hyphen_passed == 0 {
            let hold = HOLD_HEAD.link(&st.mem);
            s.set_link(&mut st.mem, hold);
            s = st.mem.list_tail(s);
            if odd(st.hyph.hyf[j - 1]) {
                l = j;
                st.hyph.hyphen_passed = j - 1;
                HOLD_HEAD.set_link(&mut st.mem, MemPtr::NULL);
            }
        }

        if st.hyph.hyphen_passed > 0 {
            loop {
                let disc = NodePtr::<DiscNode>::alloc(&mut st.mem)?;
                let hold = HOLD_HEAD.link(&st.mem);
                st.mem[disc].link = hold;
                let mut major_tail = disc.ptr();
                let mut r_count = 0;
                while major_tail.link(&st.mem).raw() > TEX_NULL {
                    major_tail = major_tail.link(&st.mem);
                    r_count += 1;
                }

                let mut i = st.hyph.hyphen_passed;
                st.hyph.hyf[i] = 0;
                let mut minor_tail = MemPtr::NULL;
                st.mem[disc].pre_break = MemPtr::NULL;
                let hyf_node =
                    crate::main_control::new_character(st, st.hyph.hf, st.hyph.hyf_char.code())?;
                let mut saved_c = LigChar::NONE;
                if let Some(node) = hyf_node {
                    i += 1;
                    saved_c = st.hyph.hu[i];
                    st.hyph.hu[i] = st.hyph.hyf_char;
                    match node {
                        NewChar::Char(c) => c.free(&mut st.mem),
                        NewChar::Native(_) => return Err(Abort::Confusion("hyphen char")),
                    }
                }

                while l <= i {
                    let font_bchar = st.fonts.font_bchar[st.hyph.hf];
                    l = reconstitute(st, l, i, font_bchar, LigChar::NONE)? + 1;
                    st.mem.append_to_disc_list(disc, false, &mut minor_tail);
                }

                if hyf_node.is_some() {
                    st.hyph.hu[i] = saved_c;
                    l = i;
                }

                minor_tail = MemPtr::NULL;
                st.mem[disc].post_break = MemPtr::NULL;
                let mut c_loc = 0;
                let mut c = LigChar::NONE;
                if st.fonts.bchar_label[st.hyph.hf] != FontInfoIdx::NON_ADDRESS {
                    l -= 1;
                    c = st.hyph.hu[l];
                    c_loc = l;
                    st.hyph.hu[l] = LigChar::from_raw(st.hyph.max_hyph_char);
                }

                while l < j {
                    loop {
                        l = reconstitute(st, l, hn, bchar, LigChar::NONE)? + 1;
                        if c_loc > 0 {
                            st.hyph.hu[c_loc] = c;
                            c_loc = 0;
                        }
                        st.mem.append_to_disc_list(disc, true, &mut minor_tail);
                        if l >= j {
                            break;
                        }
                    }

                    while l > j {
                        j = reconstitute(st, j, hn, bchar, LigChar::NONE)? + 1;
                        let hold = HOLD_HEAD.link(&st.mem);
                        major_tail.set_link(&mut st.mem, hold);
                        while major_tail.link(&st.mem).raw() > TEX_NULL {
                            major_tail = major_tail.link(&st.mem);
                            r_count += 1;
                        }
                    }
                }

                if r_count > 127 {
                    let replacement = st.mem[disc].link;
                    s.set_link(&mut st.mem, replacement);
                    st.mem[disc].link = MemPtr::NULL;
                    st.mem.flush_node_list(disc.ptr())?;
                } else {
                    s.set_link(&mut st.mem, disc.ptr());
                    st.mem[disc].replace_count =
                        u16::try_from(r_count).or_confusion("hyphenate")?;
                }
                s = major_tail;
                st.hyph.hyphen_passed = j - 1;
                HOLD_HEAD.set_link(&mut st.mem, MemPtr::NULL);

                if !odd(st.hyph.hyf[j - 1]) {
                    break;
                }
            }
        }

        if j > hn {
            break;
        }
    }

    s.set_link(&mut st.mem, q_final);
    st.mem.flush_records::<CharNode>(st.hyph.init_list);
    Ok(())
}
