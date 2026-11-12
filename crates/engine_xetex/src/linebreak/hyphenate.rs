//! Post-hyphenation: `hyphenate`, `reconstitute` and native-word hyphens.

use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::fonts::{CharTag, LigChar, LigKernAction, LigOp};
use crate::hyphenation::TrieOp;
use crate::main_control::NewChar;
use crate::mem::HOLD_HEAD;
use crate::node::{
    AnyNode, CharNode, DiscNode, HyphPosNode, LigBoundary, LigItem, LigatureNode, NativeWordNode,
    NodePtr, View,
};
use crate::params::IntPar;
use crate::state::{EngineState, EqtbState, FontState, HyphState, MemState};
use crate::units::MemIdx;
use crate::units::{FontId, MemPtr, Scaled, StrNumber};

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

impl MemState {
    /// A fresh char node of font `f` holding `c`, linked after `t`.
    fn append_char(&mut self, t: MemIdx, f: FontId, c: u16) -> Result<MemIdx, Abort> {
        let q = self.new_char(f, c)?;
        t.set_link(self, q.ptr());
        Ok(q.idx())
    }
}

/// The registers of one `reconstitute` run.
struct Reconstitution {
    /// The position in `hu` being rebuilt, and the last one.
    j: usize,
    n: usize,
    bchar: LigChar,
    hchar: LigChar,
    /// The hyphen char still to be tried at `j`.
    cur_rh: LigChar,
    cur_l: LigChar,
    cur_r: LigChar,
    /// The node the pending ligature's characters follow.
    cur_q: MemIdx,
    /// The last node built.
    t: MemIdx,
    lig_stack: Option<NodePtr<LigItem>>,
    ligature_present: bool,
    lft_hit: bool,
    rt_hit: bool,
}

impl Reconstitution {
    /// Loads `cur_r` and the hyphen char still to be tried for position `j`.
    fn load_right(&mut self, hyph: &HyphState) {
        self.cur_r = if self.j < self.n {
            hyph.hu[self.j + 1]
        } else {
            self.bchar
        };
        self.cur_rh = if odd(hyph.hyf[self.j]) {
            self.hchar
        } else {
            LigChar::NONE
        };
    }

    /// Pops the top lig item, appending its char node after `t`, and loads the next `cur_r`.
    fn pop_lig_stack(&mut self, mem: &mut MemState, hyph: &HyphState) {
        let Some(item) = self.lig_stack else {
            return;
        };
        if let Some(component) = mem[item].component {
            self.t.set_link(mem, Some(component));
            self.t = component;
            self.j += 1;
        }
        self.lig_stack = NodePtr::from_ptr(item.idx().link(mem));
        item.free(mem);
        match self.lig_stack {
            None => self.load_right(hyph),
            Some(top) => self.cur_r = LigChar::of(mem[top].character),
        }
    }
}

/// Rebuild `hu[j..]` after `HOLD_HEAD` as chars, ligatures and kerns, returning
/// the index of the last character consumed.
fn reconstitute(
    st: &mut EngineState,
    j: usize,
    n: usize,
    bchar: LigChar,
    hchar: LigChar,
) -> Result<usize, Abort> {
    st.hyph.hyphen_passed = 0;
    let mut w = Scaled::ZERO;
    HOLD_HEAD.set_link(&mut st.mem, None);
    let mut r = Reconstitution {
        j,
        n,
        bchar,
        hchar,
        cur_rh: LigChar::NONE,
        cur_l: st.hyph.hu[j],
        cur_r: LigChar::NONE,
        cur_q: HOLD_HEAD,
        t: HOLD_HEAD,
        lig_stack: None,
        ligature_present: false,
        lft_hit: false,
        rt_hit: false,
    };

    if j == 0 {
        r.ligature_present = st.hyph.init_lig;
        let mut p = st.hyph.init_list;
        if r.ligature_present {
            r.lft_hit = st.hyph.init_lft;
        }
        while let Some(q) = p {
            let c = st.mem[NodePtr::<CharNode>::at(q)].character;
            r.t = st.mem.append_char(r.t, st.hyph.hf, c)?;
            p = q.link(&st.mem);
        }
    } else if r.cur_l.is_char() {
        r.t = st.mem.append_char(r.t, st.hyph.hf, r.cur_l.code())?;
    }

    r.load_right(&st.hyph);

    loop {
        'continue_: loop {
            let hf = st.hyph.hf;
            let (mut k, mut q) = if r.cur_l == LigChar::NONE {
                let Some(k) = st.fonts.bchar_label(hf) else {
                    break 'continue_;
                };
                (k, st.fonts.lig_kern_step(hf, k)?)
            } else {
                let info = st.fonts.effective_char_info(hf, r.cur_l.code());
                let Some(CharTag::Lig(start)) = info.map(|i| i.tag) else {
                    break 'continue_;
                };
                st.fonts.lig_kern_start(hf, start)?
            };

            let mut test_char = if r.cur_rh.is_char() {
                r.cur_rh
            } else {
                r.cur_r
            };

            loop {
                if let Some(action) = q.action_for(test_char) {
                    if r.cur_rh.is_char() {
                        st.hyph.hyphen_passed = r.j;
                        r.hchar = LigChar::NONE;
                        r.cur_rh = LigChar::NONE;
                        continue 'continue_;
                    }

                    if r.hchar.is_char() && odd(st.hyph.hyf[r.j]) {
                        st.hyph.hyphen_passed = r.j;
                        r.hchar = LigChar::NONE;
                    }

                    let (op, lig_char) = match action {
                        LigKernAction::Kern(kern) => {
                            w = kern;
                            break 'continue_;
                        }
                        LigKernAction::Lig { op, char } => (op, LigChar::of(char)),
                    };
                    if r.cur_l == LigChar::NONE {
                        r.lft_hit = true;
                    }
                    if r.j == r.n && r.lig_stack.is_none() {
                        r.rt_hit = true;
                    }

                    match op {
                        LigOp::KeepRight | LigOp::KeepRightSkip => {
                            r.cur_l = lig_char;
                            r.ligature_present = true;
                        }
                        LigOp::KeepLeft | LigOp::KeepLeftSkip => {
                            r.cur_r = lig_char;
                            if let Some(top) = r.lig_stack {
                                st.mem[top].character = r.cur_r.code();
                            } else {
                                let item = st.mem.new_lig_item(r.cur_r.code())?;
                                r.lig_stack = Some(item);
                                if r.j == r.n {
                                    r.bchar = LigChar::NONE;
                                } else {
                                    let p =
                                        st.mem.new_char(st.hyph.hf, st.hyph.hu[r.j + 1].code())?;
                                    st.mem[item].component = p.ptr();
                                }
                            }
                        }
                        LigOp::KeepBoth => {
                            r.cur_r = lig_char;
                            let below = r.lig_stack.map(NodePtr::idx);
                            let item = st.mem.new_lig_item(r.cur_r.code())?;
                            r.lig_stack = Some(item);
                            item.idx().set_link(&mut st.mem, below);
                        }
                        LigOp::KeepBothSkip | LigOp::KeepBothSkip2 => {
                            if r.ligature_present {
                                wrap_lig(st, &mut r, false)?;
                            }
                            r.cur_q = r.t;
                            r.cur_l = lig_char;
                            r.ligature_present = true;
                        }
                        LigOp::Replace => {
                            r.cur_l = lig_char;
                            r.ligature_present = true;
                            if r.lig_stack.is_some() {
                                r.pop_lig_stack(&mut st.mem, &st.hyph);
                            } else if r.j == r.n {
                                break 'continue_;
                            } else {
                                r.t = st.mem.append_char(r.t, st.hyph.hf, r.cur_r.code())?;
                                r.j += 1;
                                r.load_right(&st.hyph);
                            }
                        }
                    }

                    if matches!(
                        op,
                        LigOp::KeepRightSkip | LigOp::KeepLeftSkip | LigOp::KeepBothSkip2
                    ) {
                        break 'continue_;
                    }
                    continue 'continue_;
                }

                let Some(next) = q.next(k) else {
                    if r.cur_rh == LigChar::NONE {
                        break 'continue_;
                    }
                    r.cur_rh = LigChar::NONE;
                    continue 'continue_;
                };
                k = next;
                q = st.fonts.lig_kern_step(hf, k)?;
                test_char = if r.cur_rh.is_char() {
                    r.cur_rh
                } else {
                    r.cur_r
                };
            }
        }

        if r.ligature_present {
            wrap_lig(st, &mut r, true)?;
        }

        if w != Scaled::ZERO {
            let kern = st.mem.new_kern(w)?;
            r.t.set_link(&mut st.mem, kern.ptr());
            r.t = kern.idx();
            w = Scaled::ZERO;
            st.mem[kern].sync.tag = 0;
        }

        if let Some(top) = r.lig_stack {
            r.cur_q = r.t;
            r.cur_l = LigChar::of(st.mem[top].character);
            r.ligature_present = true;
            r.pop_lig_stack(&mut st.mem, &st.hyph);
            continue;
        }

        return Ok(r.j);
    }
}

/// Replaces the chars after `cur_q` by a ligature of `cur_l`, which becomes `t`.
/// `rt` also records a right boundary hit when the lig stack is empty.
fn wrap_lig(st: &mut EngineState, r: &mut Reconstitution, rt: bool) -> Result<(), Abort> {
    let components = r.cur_q.link(&st.mem);
    let lig = st
        .mem
        .new_ligature(st.hyph.hf, r.cur_l.code(), components)?;
    let mut boundary = LigBoundary::None;
    if r.lft_hit {
        boundary = LigBoundary::Left;
        r.lft_hit = false;
    }
    if rt && r.rt_hit && r.lig_stack.is_none() {
        boundary = boundary.with_right();
        r.rt_hit = false;
    }
    st.mem[lig].boundary = boundary.into();
    r.cur_q.set_link(&mut st.mem, lig.ptr());
    r.ligature_present = false;
    r.t = lig.idx();
    Ok(())
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
    let start = word.start.or_confusion("hyphenate")?;
    let mut s = st.mem.predecessor(start, ha.ptr())?;

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
            s = disc.idx();
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
    st.mem[ha].link = None;
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
        let Some(hold) = HOLD_HEAD.link(self) else {
            return;
        };
        match *minor_tail {
            Some(t) => t.set_link(self, Some(hold)),
            None if post => self[disc].post_break = Some(hold),
            None => self[disc].pre_break = Some(hold),
        }
        *minor_tail = Some(self.list_tail(hold));
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

    let hb = word.hb.or_confusion("hyphenate")?;
    let ha = word.ha.or_confusion("hyphenate")?;
    let start = word.start.or_confusion("hyphenate")?;
    let q_final = hb.link(&st.mem);
    hb.set_link(&mut st.mem, None);
    let r = ha.link(&st.mem);
    ha.set_link(&mut st.mem, None);
    let bchar = st.hyph.hyf_bchar;

    let found2 = |st: &mut EngineState| {
        st.hyph.hu[0] = LigChar::from_raw(st.hyph.max_hyph_char);
        st.hyph.init_lig = false;
        st.hyph.init_list = None;
        (ha, 0)
    };
    let (mut s, mut j) = if let Some(ha_ch) = st.mem.as_char(Some(ha)) {
        if st.mem[ha_ch].font != st.hyph.hf {
            found2(st)
        } else {
            st.hyph.init_list = Some(ha);
            st.hyph.init_lig = false;
            st.hyph.hu[0] = LigChar::of(st.mem[ha_ch].character);
            (st.mem.predecessor(start, Some(ha))?, 0)
        }
    } else if let Ok(AnyNode::Ligature(ha_lig)) = st.mem.classify(ha) {
        if st.mem[ha_lig].font != st.hyph.hf {
            found2(st)
        } else {
            st.hyph.init_list = st.mem[ha_lig].lig_ptr;
            st.hyph.init_lig = true;
            st.hyph.init_lft = st.mem[ha_lig].boundary.has_left();
            st.hyph.hu[0] = LigChar::of(st.mem[ha_lig].character);
            if st.hyph.init_list.is_none() && st.hyph.init_lft {
                st.hyph.hu[0] = LigChar::from_raw(st.hyph.max_hyph_char);
                st.hyph.init_lig = false;
            }
            st.mem.free_node(ha, LigatureNode::SIZE);
            (st.mem.predecessor(start, Some(ha))?, 0)
        }
    } else if st
        .mem
        .classify_non_char(r)
        .is_some_and(|n| matches!(n, AnyNode::Ligature(lig) if st.mem[lig].boundary.has_left()))
    {
        found2(st)
    } else {
        st.hyph.init_list = None;
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
                HOLD_HEAD.set_link(&mut st.mem, None);
            }
        }

        if st.hyph.hyphen_passed > 0 {
            loop {
                let disc = NodePtr::<DiscNode>::alloc(&mut st.mem)?;
                let hold = HOLD_HEAD.link(&st.mem);
                st.mem[disc].link = hold;
                let mut major_tail = disc.idx();
                let mut r_count = 0;
                while let Some(q) = major_tail.link(&st.mem) {
                    major_tail = q;
                    r_count += 1;
                }

                let mut i = st.hyph.hyphen_passed;
                st.hyph.hyf[i] = 0;
                let mut minor_tail = None;
                st.mem[disc].pre_break = None;
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

                minor_tail = None;
                st.mem[disc].post_break = None;
                let mut c_loc = 0;
                let mut c = LigChar::NONE;
                if st.fonts.bchar_label(st.hyph.hf).is_some() {
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
                        while let Some(q) = major_tail.link(&st.mem) {
                            major_tail = q;
                            r_count += 1;
                        }
                    }
                }

                if r_count > 127 {
                    let replacement = st.mem[disc].link;
                    s.set_link(&mut st.mem, replacement);
                    st.mem[disc].link = None;
                    st.mem.flush_node_list(disc.ptr())?;
                } else {
                    s.set_link(&mut st.mem, disc.ptr());
                    st.mem[disc].replace_count =
                        u16::try_from(r_count).or_confusion("hyphenate")?;
                }
                s = major_tail;
                st.hyph.hyphen_passed = j - 1;
                HOLD_HEAD.set_link(&mut st.mem, None);

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
