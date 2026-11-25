//! Hyphenation patterns and exceptions, and the packed-trie builder.

use std::array;

use crate::base::cmd::Cmd;
use crate::base::command::{Command, Usv};
pub(crate) use crate::base::consts::BIGGEST_LANG;
use crate::base::consts::{BIGGEST_CHAR, NULL_FONT};
use crate::base::params::IntPar;
use crate::base::units::{FontId, MemPtr, StrNumber};
use crate::env::EngineEnv;
use crate::fonts::LigChar;
use crate::print::Esc;
use crate::print::errors::{Abort, Capacity, OrConfusion, error};
use crate::scanning::scan::{get_x_token, scan_char_num, scan_left_brace, scan_toks};
use crate::state::EngineState;
use crate::tables::eqtb::EqtbState;

pub(crate) const TRIE_OP_SIZE: i32 = 35_111;
pub(crate) const HYPH_PRIME: i32 = 607;

const APPENDIX_H: &str = "(See Appendix H.)";

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub(crate) struct TriePtr(usize);

impl TriePtr {
    /// A free slot's link, and "no family" in `trie_ref`.
    pub(crate) const NULL: TriePtr = TriePtr(0);

    #[inline]
    pub(crate) const fn from_raw(raw: i32) -> TriePtr {
        TriePtr(raw as usize)
    }

    #[inline]
    pub(crate) const fn raw(self) -> i32 {
        self.0 as i32
    }

    #[inline]
    pub(crate) fn index(self) -> usize {
        self.0
    }

    #[inline]
    pub(crate) fn plus(self, c: usize) -> TriePtr {
        TriePtr(self.0 + c)
    }

    #[inline]
    fn minus(self, c: usize) -> TriePtr {
        TriePtr(self.0 - c)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct TrieNode(usize);

impl TrieNode {
    /// No node, and the root.
    pub(crate) const ROOT: TrieNode = TrieNode(0);

    #[inline]
    fn is_some(self) -> bool {
        self.0 > 0
    }
}

/// A pattern op number within its language, `NONE` ending a chain.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub(crate) struct TrieOp(u16);

impl TrieOp {
    pub(crate) const NONE: TrieOp = TrieOp(0);
    pub(crate) const MAX: TrieOp = TrieOp(65_535);

    #[inline]
    pub(crate) const fn raw(self) -> u16 {
        self.0
    }

    #[inline]
    pub(crate) fn index(self) -> usize {
        usize::from(self.0)
    }
}

/// A pattern op: hyphen value `num` at `distance` positions before the end of
/// the match, followed by op `next`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct HyphOp {
    pub distance: u16,
    pub num: u8,
    pub next: TrieOp,
}

impl HyphState {
    #[inline]
    pub(crate) fn hyph_op(&self, op_base: usize, v: TrieOp) -> HyphOp {
        self.op_at(op_base + v.index())
    }

    #[inline]
    fn op_at(&self, i: usize) -> HyphOp {
        HyphOp {
            distance: self.hyf_distance[i],
            num: self.hyf_num[i] as u8,
            next: TrieOp(self.hyf_next[i]),
        }
    }

    fn set_op_at(&mut self, i: usize, op: HyphOp) {
        self.hyf_distance[i] = op.distance;
        self.hyf_num[i] = u16::from(op.num);
        self.hyf_next[i] = op.next.0;
    }

    #[inline]
    pub(crate) fn trie_link(&self, p: TriePtr) -> TriePtr {
        TriePtr(self.trie_trl[p.0] as usize)
    }

    #[inline]
    fn set_trie_link(&mut self, p: TriePtr, link: TriePtr) {
        self.trie_trl[p.0] = link.0 as i32;
    }

    #[inline]
    pub(crate) fn trie_char(&self, p: TriePtr) -> u16 {
        self.trie_trc[p.0]
    }

    #[inline]
    pub(crate) fn trie_op(&self, p: TriePtr) -> TrieOp {
        TrieOp(self.trie_tro[p.0] as u16)
    }

    /// The previous free slot while packing, stored in `trie_op`.
    #[inline]
    fn trie_back(&self, p: TriePtr) -> TriePtr {
        TriePtr(self.trie_tro[p.0] as usize)
    }

    #[inline]
    fn set_trie_back(&mut self, p: TriePtr, back: TriePtr) {
        self.trie_tro[p.0] = back.0 as i32;
    }

    fn clear_trie_slot(&mut self, p: TriePtr) {
        self.trie_trl[p.0] = 0;
        self.trie_tro[p.0] = i32::from(TrieOp::NONE.0);
        self.trie_trc[p.0] = 0;
    }

    pub(crate) fn pattern_root(&self, lang: u8) -> Option<TriePtr> {
        let slot = TriePtr(usize::from(lang) + 1);
        (self.trie_char(slot) == u16::from(lang)).then(|| self.trie_link(slot))
    }

    #[inline]
    fn trie_min(&self, c: usize) -> TriePtr {
        TriePtr(self.trie_min[c] as usize)
    }

    #[inline]
    fn set_trie_min(&mut self, c: usize, p: TriePtr) {
        self.trie_min[c] = p.0 as u32;
    }
}

impl HyphState {
    #[inline]
    fn trie_l(&self, p: TrieNode) -> TrieNode {
        TrieNode(self.trie_l[p.0] as usize)
    }

    #[inline]
    fn set_trie_l(&mut self, p: TrieNode, q: TrieNode) {
        self.trie_l[p.0] = q.0 as u32;
    }

    #[inline]
    fn trie_r(&self, p: TrieNode) -> TrieNode {
        TrieNode(self.trie_r[p.0] as usize)
    }

    #[inline]
    fn set_trie_r(&mut self, p: TrieNode, q: TrieNode) {
        self.trie_r[p.0] = q.0 as u32;
    }

    #[inline]
    fn trie_c(&self, p: TrieNode) -> u16 {
        self.trie_c[p.0]
    }

    #[inline]
    fn trie_o(&self, p: TrieNode) -> TrieOp {
        TrieOp(self.trie_o[p.0])
    }

    #[inline]
    fn set_trie_o(&mut self, p: TrieNode, op: TrieOp) {
        self.trie_o[p.0] = op.0;
    }

    #[inline]
    fn trie_hash(&self, h: usize) -> TrieNode {
        TrieNode(self.trie_hash[h] as usize)
    }

    #[inline]
    fn set_trie_hash(&mut self, h: usize, q: TrieNode) {
        self.trie_hash[h] = q.0 as u32;
    }

    #[inline]
    fn trie_ref(&self, p: TrieNode) -> TriePtr {
        TriePtr(self.trie_hash[p.0] as usize)
    }

    #[inline]
    fn set_trie_ref(&mut self, p: TrieNode, h: TriePtr) {
        self.trie_hash[p.0] = h.0 as u32;
    }

    #[inline]
    fn trie_op_hash(&self, k: i32) -> i32 {
        self.trie_op_hash_array[(k + TRIE_OP_SIZE) as usize]
    }

    #[inline]
    fn set_trie_op_hash(&mut self, k: i32, v: i32) {
        self.trie_op_hash_array[(k + TRIE_OP_SIZE) as usize] = v;
    }
}

impl HyphState {
    pub(crate) fn set_hyph_index(&mut self) {
        let slot = self.hyph_start.plus(usize::from(self.cur_lang));
        self.hyph_index = (self.trie_char(slot) == u16::from(self.cur_lang))
            .then(|| self.trie_link(slot))
            .filter(|&p| p != TriePtr::NULL);
    }

    #[inline]
    pub(crate) fn hyph_code(&self, eqtb: &EqtbState, c: i32) -> i32 {
        let Some(index) = self.hyph_index.filter(|_| c <= 255) else {
            return eqtb.lc_code(c);
        };
        let slot = index.plus(c as usize);
        if self.trie_char(slot) == c as u16 {
            self.trie_tro[slot.0]
        } else {
            0
        }
    }

    /// The exception slot after `h` in its hash chain, stored one up so 0 ends the chain.
    #[inline]
    pub(crate) fn hyph_chain_next(&self, h: usize) -> Option<usize> {
        usize::from(self.hyph_link[h]).checked_sub(1)
    }

    pub(crate) fn exception_hash(&self, n: usize) -> usize {
        self.hc[1..=n]
            .iter()
            .fold(0, |h, &c| (h + h + c) % HYPH_PRIME) as usize
    }

    fn new_trie_op(&mut self, lang: u8, op: HyphOp) -> Result<TrieOp, Abort> {
        let mut h = (i32::from(op.num)
            + 313 * i32::from(op.distance)
            + 361 * i32::from(op.next.0)
            + 1009 * i32::from(lang))
        .abs()
            % (2 * TRIE_OP_SIZE)
            - TRIE_OP_SIZE;

        loop {
            let l = self.trie_op_hash(h);
            if l == 0 {
                if self.trie_op_ptr == TRIE_OP_SIZE {
                    return Err(Abort::Overflow {
                        capacity: Capacity::PatternMemoryOps,
                        n: TRIE_OP_SIZE,
                    });
                }

                let used = &mut self.trie_used[usize::from(lang)];
                if *used == TrieOp::MAX.0 {
                    return Err(Abort::Overflow {
                        capacity: Capacity::PatternMemoryOpsPerLanguage,
                        n: i32::from(TrieOp::MAX.0 - TrieOp::NONE.0),
                    });
                }
                *used += 1;
                let u = TrieOp(*used);

                self.trie_op_ptr += 1;
                self.max_op_used = self.max_op_used.max(u.0);

                let p = self.trie_op_ptr as usize;
                self.set_op_at(p, op);
                self.trie_op_lang[p] = lang;
                self.set_trie_op_hash(h, self.trie_op_ptr);
                self.trie_op_val[p] = u.0;
                return Ok(u);
            }

            let l = l as usize;
            if self.op_at(l) == op && self.trie_op_lang[l] == lang {
                return Ok(TrieOp(self.trie_op_val[l]));
            }

            if h > -TRIE_OP_SIZE {
                h -= 1;
            } else {
                h = TRIE_OP_SIZE;
            }
        }
    }

    fn trie_node(&mut self, p: TrieNode) -> TrieNode {
        let mut h = i32::from(self.trie_c(p))
            .wrapping_add(1009_i32.wrapping_mul(i32::from(self.trie_o(p).0)))
            .wrapping_add(2718_i32.wrapping_mul(self.trie_l[p.0] as i32))
            .wrapping_add(3142_i32.wrapping_mul(self.trie_r[p.0] as i32))
            .wrapping_abs()
            % self.trie_size;

        loop {
            let q = self.trie_hash(h as usize);
            if !q.is_some() {
                self.set_trie_hash(h as usize, p);
                return p;
            }

            if self.trie_c(q) == self.trie_c(p)
                && self.trie_o(q) == self.trie_o(p)
                && self.trie_l(q) == self.trie_l(p)
                && self.trie_r(q) == self.trie_r(p)
            {
                return q;
            }

            if h > 0 {
                h -= 1;
            } else {
                h = self.trie_size;
            }
        }
    }

    fn compress_trie(&mut self, p: TrieNode) -> TrieNode {
        if !p.is_some() {
            return TrieNode::ROOT;
        }

        let l = self.compress_trie(self.trie_l(p));
        self.set_trie_l(p, l);
        let r = self.compress_trie(self.trie_r(p));
        self.set_trie_r(p, r);
        self.trie_node(p)
    }

    fn first_fit(&mut self, p: TrieNode) -> Result<(), Abort> {
        let c = usize::from(self.trie_c(p));
        let max_hyph_char = self.max_hyph_char as usize;
        let mut z = self.trie_min(c);
        let mut h;

        loop {
            h = z.minus(c);
            let top = h.plus(max_hyph_char);
            if self.trie_max < top {
                if self.trie_size as usize <= top.0 {
                    return Err(Abort::Overflow {
                        capacity: Capacity::PatternMemory,
                        n: self.trie_size,
                    });
                }

                loop {
                    self.trie_max = self.trie_max.plus(1);
                    let max = self.trie_max;
                    self.trie_taken[max.0] = false;
                    self.set_trie_link(max, max.plus(1));
                    self.set_trie_back(max, max.minus(1));
                    if max == top {
                        break;
                    }
                }
            }

            let mut found = !self.trie_taken[h.0];
            let mut q = self.trie_r(p);
            while found && q.is_some() {
                if self.trie_link(h.plus(usize::from(self.trie_c(q)))) == TriePtr::NULL {
                    found = false;
                } else {
                    q = self.trie_r(q);
                }
            }

            if found {
                break;
            }

            z = self.trie_link(z);
        }

        self.trie_taken[h.0] = true;
        self.set_trie_ref(p, h);
        let mut q = p;
        loop {
            z = h.plus(usize::from(self.trie_c(q)));
            let mut l = self.trie_back(z);
            let r = self.trie_link(z);
            self.set_trie_back(r, l);
            self.set_trie_link(l, r);
            self.set_trie_link(z, TriePtr::NULL);

            // The slots below `max_hyph_char` double as character codes here.
            if l.0 < max_hyph_char {
                let ll = z.0.min(max_hyph_char);
                loop {
                    self.set_trie_min(l.0, r);
                    l = l.plus(1);
                    if l.0 == ll {
                        break;
                    }
                }
            }

            q = self.trie_r(q);
            if !q.is_some() {
                break;
            }
        }
        Ok(())
    }

    fn trie_pack(&mut self, mut p: TrieNode) -> Result<(), Abort> {
        loop {
            let q = self.trie_l(p);
            if q.is_some() && self.trie_ref(q) == TriePtr::NULL {
                self.first_fit(q)?;
                self.trie_pack(q)?;
            }

            p = self.trie_r(p);
            if !p.is_some() {
                return Ok(());
            }
        }
    }

    fn trie_fix(&mut self, mut p: TrieNode) {
        let z = self.trie_ref(p);
        loop {
            let q = self.trie_l(p);
            let c = self.trie_c(p);
            let slot = z.plus(usize::from(c));
            self.set_trie_link(slot, self.trie_ref(q));
            self.trie_trc[slot.0] = c;
            self.trie_tro[slot.0] = i32::from(self.trie_o(p).0);

            if q.is_some() {
                self.trie_fix(q);
            }

            p = self.trie_r(p);
            if !p.is_some() {
                break;
            }
        }
    }

    pub(crate) fn init_trie(&mut self) -> Result<(), Abort> {
        self.tables_gen = self.tables_gen.wrapping_add(1);
        self.max_hyph_char += 1;
        self.op_start[0] = 0;

        for j in 1..=BIGGEST_LANG as usize {
            self.op_start[j] = self.op_start[j - 1] + usize::from(self.trie_used[j - 1]);
        }

        let op_ptr = self.trie_op_ptr;
        for j in 1..=op_ptr {
            let lang = usize::from(self.trie_op_lang[j as usize]);
            let val = self.op_start[lang] + usize::from(self.trie_op_val[j as usize]);
            self.set_trie_op_hash(j, val as i32);
        }

        for j in 1..=op_ptr {
            while self.trie_op_hash(j) > j {
                let k = self.trie_op_hash(j);
                let (ju, ku) = (j as usize, k as usize);
                self.hyf_distance.swap(ju, ku);
                self.hyf_num.swap(ju, ku);
                self.hyf_next.swap(ju, ku);
                self.set_trie_op_hash(j, self.trie_op_hash(k));
                self.set_trie_op_hash(k, k);
            }
        }

        self.trie_hash[..=self.trie_size as usize].fill(0);

        let r = self.compress_trie(self.trie_r(TrieNode::ROOT));
        self.set_trie_r(TrieNode::ROOT, r);
        let l = self.compress_trie(self.trie_l(TrieNode::ROOT));
        self.set_trie_l(TrieNode::ROOT, l);

        self.trie_hash[..=self.trie_ptr as usize].fill(0);

        for (min, p) in self.trie_min[..=BIGGEST_CHAR as usize].iter_mut().zip(1..) {
            *min = p;
        }

        self.set_trie_link(TriePtr::NULL, TriePtr(1));
        self.trie_max = TriePtr::NULL;

        let trie_l0 = self.trie_l(TrieNode::ROOT);
        if trie_l0.is_some() {
            self.first_fit(trie_l0)?;
            self.trie_pack(trie_l0)?;
        }

        let trie_r0 = self.trie_r(TrieNode::ROOT);
        if trie_r0.is_some() {
            if !trie_l0.is_some() {
                for (min, p) in self.trie_min[..=255].iter_mut().zip(2..) {
                    *min = p;
                }
            }
            self.first_fit(trie_r0)?;
            self.trie_pack(trie_r0)?;
            self.hyph_start = self.trie_ref(trie_r0);
        }

        if self.trie_max == TriePtr::NULL {
            for r in 0..=self.max_hyph_char as usize {
                self.clear_trie_slot(TriePtr(r));
            }
            self.trie_max = TriePtr(self.max_hyph_char as usize);
        } else {
            if trie_r0.is_some() {
                self.trie_fix(trie_r0);
            }
            if trie_l0.is_some() {
                self.trie_fix(trie_l0);
            }

            let mut r = TriePtr::NULL;
            loop {
                let s = self.trie_link(r);
                self.clear_trie_slot(r);
                r = s;
                if r > self.trie_max {
                    break;
                }
            }
        }

        self.trie_trc[0] = u16::from(b'?');
        self.trie_not_ready = false;
        Ok(())
    }

    fn trie_insert_node(
        &mut self,
        q: TrieNode,
        p: &mut TrieNode,
        c: u16,
        first_child: bool,
    ) -> Result<(), Abort> {
        if self.trie_ptr == self.trie_size {
            return Err(Abort::Overflow {
                capacity: Capacity::PatternMemory,
                n: self.trie_size,
            });
        }

        self.trie_ptr += 1;
        let new = TrieNode(self.trie_ptr as usize);
        self.set_trie_r(new, *p);
        *p = new;
        self.set_trie_l(new, TrieNode::ROOT);
        if first_child {
            self.set_trie_l(q, new);
        } else {
            self.set_trie_r(q, new);
        }
        self.trie_c[new.0] = c;
        self.set_trie_o(new, TrieOp::NONE);
        Ok(())
    }

    /// Insert the pattern `hc[1..=k]` into the trie, returning its node, its op,
    /// and whether the node already carried an op (a duplicate pattern).
    fn insert_pattern_word(&mut self, k: usize) -> Result<(TrieNode, TrieOp, bool), Abort> {
        if self.hc[1] == 0 {
            self.hyf[0] = 0;
        }
        if self.hc[k] == 0 {
            self.hyf[k] = 0;
        }

        let mut v = TrieOp::NONE;
        for l in (0..=k).rev() {
            if self.hyf[l] != 0 {
                let op = HyphOp {
                    distance: (k - l) as u16,
                    num: self.hyf[l],
                    next: v,
                };
                v = self.new_trie_op(self.cur_lang, op)?;
            }
        }

        let mut q = TrieNode::ROOT;
        self.hc[0] = i32::from(self.cur_lang);
        for l in 0..=k {
            let c = self.hc[l] as u16;
            let mut p = self.trie_l(q);
            let mut first_child = true;

            while p.is_some() && c > self.trie_c(p) {
                q = p;
                p = self.trie_r(q);
                first_child = false;
            }

            if !p.is_some() || c < self.trie_c(p) {
                self.trie_insert_node(q, &mut p, c, first_child)?;
            }

            q = p;
        }

        Ok((q, v, self.trie_o(q) != TrieOp::NONE))
    }

    fn save_hyph_codes(&mut self, lc_codes: &[i32; 256]) -> Result<(), Abort> {
        let c = u16::from(self.cur_lang);
        let mut p = TrieNode::ROOT;

        let mut q;
        loop {
            q = p;
            p = self.trie_r(q);
            if !p.is_some() || c <= self.trie_c(p) {
                break;
            }
        }

        if !p.is_some() || c < self.trie_c(p) {
            self.trie_insert_node(q, &mut p, c, false)?;
        }

        q = p;
        p = self.trie_l(q);
        let mut first_child = true;

        for (ci, &lc) in (0u16..=255).zip(lc_codes.iter()) {
            if lc > 0 || (ci == 255 && first_child) {
                if !p.is_some() {
                    self.trie_insert_node(q, &mut p, ci, first_child)?;
                } else {
                    self.trie_c[p.0] = ci;
                }
                self.set_trie_o(p, TrieOp(lc as u16));
                q = p;
                p = self.trie_r(q);
                first_child = false;
            }
        }

        if first_child {
            self.set_trie_l(q, TrieNode::ROOT);
        } else {
            self.set_trie_r(q, TrieNode::ROOT);
        }

        Ok(())
    }
}

fn max_word_len(eqtb: &EqtbState) -> usize {
    usize::try_from(eqtb.max_hyphenatable_length()).unwrap_or(0)
}

pub(crate) fn new_patterns(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    st.hyph.tables_gen = st.hyph.tables_gen.wrapping_add(1);

    if !st.hyph.trie_not_ready {
        error(
            st,
            env,
            ("Too late for ", Esc("patterns")),
            &["All patterns must be given before typesetting begins."],
        )?;

        let list = scan_toks(st, env, false, false)?;
        st.mem.tokens.flush(list);
        return Ok(());
    }

    st.hyph.cur_lang = norm_lang(st.eqtb.int_par(IntPar::Language));

    scan_left_brace(st, env)?;
    let mut k = 0;
    st.hyph.hyf[0] = 0;
    let mut digit_sensed = false;

    loop {
        get_x_token(st, env)?;
        match st.scanner.cur.cmd() {
            Cmd::Letter | Cmd::OtherChar => {
                let cur_chr = st.scanner.cur.char_code().or_confusion("pattern letter")?;
                let digit = u8::try_from(cur_chr).ok().filter(u8::is_ascii_digit);
                if let Some(d) = digit.filter(|_| !digit_sensed) {
                    if k < max_word_len(&st.eqtb) {
                        st.hyph.hyf[k] = d - b'0';
                        digit_sensed = true;
                    }
                } else {
                    let code = if cur_chr == i32::from(b'.') {
                        0
                    } else {
                        let code = st.eqtb.lc_code(cur_chr);
                        if code == 0 {
                            error(st, env, "Nonletter", &[APPENDIX_H])?;
                        }
                        code
                    };

                    if code > st.hyph.max_hyph_char {
                        st.hyph.max_hyph_char = code;
                    }

                    if k < max_word_len(&st.eqtb) {
                        k += 1;
                        st.hyph.hc[k] = code;
                        st.hyph.hyf[k] = 0;
                        digit_sensed = false;
                    }
                }
            }
            Cmd::Spacer | Cmd::RightBrace => {
                if k > 0 {
                    let (q, op, duplicate) = st.hyph.insert_pattern_word(k)?;
                    if duplicate {
                        error(st, env, "Duplicate pattern", &[APPENDIX_H])?;
                    }
                    st.hyph.set_trie_o(q, op);
                }

                if st.scanner.cur.cmd() == Cmd::RightBrace {
                    break;
                }

                k = 0;
                st.hyph.hyf[0] = 0;
                digit_sensed = false;
            }
            _ => {
                error(st, env, ("Bad ", Esc("patterns")), &[APPENDIX_H])?;
            }
        }
    }

    if st.eqtb.int_par(IntPar::SavingHyphCodes) > 0 {
        let lc_codes: [i32; 256] = array::from_fn(|ci| st.eqtb.lc_code(ci as i32));
        st.hyph.save_hyph_codes(&lc_codes)?;
    }
    Ok(())
}

pub(crate) fn new_hyph_exceptions(st: &mut EngineState, env: &mut EngineEnv) -> Result<(), Abort> {
    st.hyph.tables_gen = st.hyph.tables_gen.wrapping_add(1);

    scan_left_brace(st, env)?;

    st.hyph.cur_lang = norm_lang(st.eqtb.int_par(IntPar::Language));
    if st.hyph.trie_not_ready {
        st.hyph.hyph_index = None;
    } else {
        st.hyph.set_hyph_index();
    }

    let mut n = 0;
    let mut p = None;

    loop {
        get_x_token(st, env)?;

        // reswitch: \char re-dispatches as Cmd::CharGiven
        loop {
            match st.scanner.cur.cmd() {
                Cmd::Letter | Cmd::OtherChar | Cmd::CharGiven => {
                    let cur_chr = st
                        .scanner
                        .cur
                        .char_code()
                        .or_confusion("hyphenation letter")?;
                    if cur_chr == i32::from(b'-') {
                        if n < max_word_len(&st.eqtb) {
                            p = st.mem.new_hyph_pos(n, p)?;
                        }
                    } else {
                        st.hyph.hc[0] = st.hyph.hyph_code(&st.eqtb, cur_chr);

                        if st.hyph.hc[0] == 0 {
                            error(
                                st,
                                env,
                                "Not a letter",
                                &[
                                    r"Letters in \hyphenation words must have \lccode>0.",
                                    "Proceed; I'll ignore the character I just read.",
                                ],
                            )?;
                        } else if n < max_word_len(&st.eqtb) {
                            let hc0 = st.hyph.hc[0];
                            n += 1;
                            if hc0 < 65536 {
                                st.hyph.hc[n] = hc0;
                            } else {
                                st.hyph.hc[n] = (hc0 - 65536) / 1024 + 0xD800;
                                n += 1;
                                st.hyph.hc[n] = hc0 % 1024 + 0xDC00;
                            }
                        }
                    }
                    break;
                }
                Cmd::CharNum => {
                    let c = scan_char_num(st, env)?;
                    st.scanner.cur = Command::CharGiven(Usv::new(c).or_confusion("char_num")?);
                }
                Cmd::Spacer | Cmd::RightBrace => {
                    if n > 1 {
                        store_hyph_exception(st, n, p)?;
                    }

                    if st.scanner.cur.cmd() == Cmd::RightBrace {
                        return Ok(());
                    }

                    n = 0;
                    p = None;
                    break;
                }
                _ => {
                    error(
                        st,
                        env,
                        ("Improper ", Esc("hyphenation"), " will be flushed"),
                        &[
                            "Hyphenation exceptions must contain only letters",
                            "and hyphens. But continue; I'll forgive and forget.",
                        ],
                    )?;
                    break;
                }
            }
        }
    }
}

fn store_hyph_exception(st: &mut EngineState, mut n: usize, p: MemPtr) -> Result<(), Abort> {
    n += 1;
    st.hyph.hc[n] = i32::from(st.hyph.cur_lang);
    if st.strings.pool_ptr + n as i32 > st.strings.pool_size {
        return Err(st.strings.pool_overflow());
    }

    let mut h = st.hyph.exception_hash(n);
    for &c in &st.hyph.hc[1..=n] {
        st.strings.append_char(c as u16);
    }

    let mut s = st.strings.make_string()?;

    let hyph = &mut st.hyph;
    if hyph.hyph_next <= HYPH_PRIME {
        while hyph.hyph_next > 0 && hyph.hyph_word[(hyph.hyph_next - 1) as usize] > StrNumber::NONE
        {
            hyph.hyph_next -= 1;
        }
    }

    if hyph.hyph_count == hyph.hyph_size || hyph.hyph_next == 0 {
        return Err(Abort::Overflow {
            capacity: Capacity::ExceptionDictionary,
            n: hyph.hyph_size,
        });
    }

    hyph.hyph_count += 1;

    loop {
        let k = hyph.hyph_word[h];
        if k == StrNumber::NONE {
            break;
        }

        let strings = &st.strings;
        if strings.length(k) == strings.length(s)
            && strings.str_pool[strings.str_range(k)] == strings.str_pool[strings.str_range(s)]
        {
            st.strings.flush_string();
            s = k;
            hyph.hyph_count -= 1;
            break;
        }

        h = match hyph.hyph_chain_next(h) {
            Some(next) => next,
            None => {
                // `hyph_next` is a link too, and nonzero after the overflow check.
                let link = hyph.hyph_next;
                hyph.hyph_link[h] = link as u16;
                if hyph.hyph_next >= hyph.hyph_size {
                    hyph.hyph_next = HYPH_PRIME;
                }
                if hyph.hyph_next > HYPH_PRIME {
                    hyph.hyph_next += 1;
                }
                link as usize - 1
            }
        };
    }

    hyph.hyph_word[h] = s;
    hyph.hyph_list[h] = p;
    Ok(())
}

pub(crate) struct HyphState {
    /// Generation of the rarely-mutated trie and exception tables.
    pub tables_gen: u64,
    pub hf: FontId,
    pub max_hyph_char: i32,
    pub init_list: MemPtr,
    pub init_lig: bool,
    pub init_lft: bool,
    pub hyphen_passed: usize,
    pub trie_max: TriePtr,
    pub trie_size: i32,
    pub trie_ptr: i32,
    pub hyph_count: i32,
    pub hyph_size: i32,
    pub hyph_next: i32,
    pub trie_op_ptr: i32,
    pub max_op_used: u16,
    pub trie_not_ready: bool,
    pub hyph_start: TriePtr,
    pub hyph_index: Option<TriePtr>,
    pub cur_lang: u8,
    pub hyf_char: LigChar,
    pub l_hyf: i32,
    pub r_hyf: i32,
    pub hyf_bchar: LigChar,
    pub hc: Vec<i32>,
    /// The word's characters, with boundary sentinels at the ends.
    pub hu: Vec<LigChar>,
    pub hyf: Vec<u8>,
    pub trie_min: Vec<u32>,
    pub hyf_distance: Vec<u16>,
    pub hyf_num: Vec<u16>,
    pub hyf_next: Vec<u16>,
    pub op_start: Vec<usize>,
    pub trie_used: Vec<u16>,
    pub trie_op_lang: Vec<u8>,
    pub trie_op_val: Vec<u16>,
    pub trie_trl: Vec<i32>,
    pub trie_tro: Vec<i32>,
    pub trie_trc: Vec<u16>,
    pub trie_c: Vec<u16>,
    pub trie_o: Vec<u16>,
    pub trie_l: Vec<u32>,
    pub trie_r: Vec<u32>,
    pub trie_hash: Vec<u32>,
    pub trie_taken: Vec<bool>,
    pub hyph_word: Vec<StrNumber>,
    pub hyph_list: Vec<MemPtr>,
    pub hyph_link: Vec<u16>,
    pub trie_op_hash_array: Vec<i32>,
}

impl Default for HyphState {
    fn default() -> Self {
        Self {
            tables_gen: 0,
            hf: NULL_FONT,
            max_hyph_char: 0,
            init_list: None,
            init_lig: false,
            init_lft: false,
            hyphen_passed: 0,
            trie_max: TriePtr::NULL,
            trie_size: 0,
            trie_ptr: 0,
            hyph_count: 0,
            hyph_size: 0,
            hyph_next: 0,
            trie_op_ptr: 0,
            max_op_used: 0,
            trie_not_ready: false,
            hyph_start: TriePtr::NULL,
            hyph_index: None,
            cur_lang: 0,
            hyf_char: LigChar::NONE,
            l_hyf: 0,
            r_hyf: 0,
            hyf_bchar: LigChar::NONE,
            hc: vec![0; 4099],
            hu: vec![LigChar::NONE; 4097],
            hyf: vec![0; 4097],
            trie_min: vec![0; 65536],
            hyf_distance: vec![0; 35112],
            hyf_num: vec![0; 35112],
            hyf_next: vec![0; 35112],
            op_start: vec![0; 256],
            trie_used: vec![0; 256],
            trie_op_lang: vec![0; 35112],
            trie_op_val: vec![0; 35112],
            trie_trl: Vec::new(),
            trie_tro: Vec::new(),
            trie_trc: Vec::new(),
            trie_c: Vec::new(),
            trie_o: Vec::new(),
            trie_l: Vec::new(),
            trie_r: Vec::new(),
            trie_hash: Vec::new(),
            trie_taken: Vec::new(),
            hyph_word: Vec::new(),
            hyph_list: Vec::new(),
            hyph_link: Vec::new(),
            trie_op_hash_array: vec![0; 70223],
        }
    }
}

pub(crate) fn norm_min(h: i32) -> i16 {
    h.clamp(1, 63) as i16
}

pub(crate) fn norm_lang(lang: i32) -> u8 {
    if lang <= 0 || lang > BIGGEST_LANG {
        0
    } else {
        lang as u8
    }
}
