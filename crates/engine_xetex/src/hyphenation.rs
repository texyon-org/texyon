//! Hyphenation patterns and exceptions, and the packed-trie builder.

use crate::cmd::{CHAR_GIVEN, CHAR_NUM, LETTER, OTHER_CHAR, RIGHT_BRACE, SPACER};
pub(crate) use crate::consts::BIGGEST_LANG;
use crate::consts::{BIGGEST_CHAR, MIN_TRIE_OP};
use crate::errors::Abort;
use crate::params::IntPar;
use crate::state::{EngineState, EqtbState, HyphState};
use crate::units::{MemPtr, StrNumber};

pub(crate) const TRIE_OP_SIZE: i32 = 35_111;
pub(crate) const MAX_TRIE_OP: u16 = 65_535;
pub(crate) const HYPH_PRIME: i32 = 607;

const APPENDIX_H: &str = "(See Appendix H.)";

/// A pattern op: hyphen value `num` at `distance` positions before the end of
/// the match, followed by op `next` (`MIN_TRIE_OP` ends the chain).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct HyphOp {
    pub distance: u16,
    pub num: u8,
    pub next: u16,
}

/// The packed trie and its ops.
impl HyphState {
    #[inline]
    pub(crate) fn hyph_op(&self, v: usize) -> HyphOp {
        HyphOp {
            distance: self.hyf_distance[v],
            num: self.hyf_num[v] as u8,
            next: self.hyf_next[v],
        }
    }

    fn set_hyph_op(&mut self, v: usize, op: HyphOp) {
        self.hyf_distance[v] = op.distance;
        self.hyf_num[v] = u16::from(op.num);
        self.hyf_next[v] = op.next;
    }

    /// The base of slot `p`'s children.
    #[inline]
    pub(crate) fn trie_link(&self, p: usize) -> usize {
        self.trie_trl[p] as usize
    }

    #[inline]
    fn set_trie_link(&mut self, p: usize, link: usize) {
        self.trie_trl[p] = link as i32;
    }

    #[inline]
    pub(crate) fn trie_char(&self, p: usize) -> u16 {
        self.trie_trc[p]
    }

    #[inline]
    pub(crate) fn trie_op(&self, p: usize) -> usize {
        self.trie_tro[p] as usize
    }

    /// The previous free slot while packing, stored in `trie_op`.
    #[inline]
    fn trie_back(&self, p: usize) -> usize {
        self.trie_tro[p] as usize
    }

    #[inline]
    fn set_trie_back(&mut self, p: usize, back: usize) {
        self.trie_tro[p] = back as i32;
    }

    fn clear_trie_slot(&mut self, p: usize) {
        self.trie_trl[p] = 0;
        self.trie_tro[p] = i32::from(MIN_TRIE_OP);
        self.trie_trc[p] = 0;
    }

    /// The root of language `lang`'s patterns, if it has any.
    pub(crate) fn pattern_root(&self, lang: u8) -> Option<usize> {
        let slot = usize::from(lang) + 1;
        (self.trie_char(slot) == u16::from(lang)).then(|| self.trie_link(slot))
    }
}

/// The linked trie that INITEX builds from `\patterns`.
impl HyphState {
    #[inline]
    fn trie_l(&self, p: usize) -> usize {
        self.trie_l[p] as usize
    }

    #[inline]
    fn set_trie_l(&mut self, p: usize, q: usize) {
        self.trie_l[p] = q as u32;
    }

    #[inline]
    fn trie_r(&self, p: usize) -> usize {
        self.trie_r[p] as usize
    }

    #[inline]
    fn set_trie_r(&mut self, p: usize, q: usize) {
        self.trie_r[p] = q as u32;
    }

    #[inline]
    fn trie_hash(&self, p: usize) -> usize {
        self.trie_hash[p] as usize
    }

    #[inline]
    fn set_trie_hash(&mut self, p: usize, q: usize) {
        self.trie_hash[p] = q as u32;
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
    /// Point `hyph_index` at the `\lccode` table saved with `cur_lang`'s patterns, or 0.
    pub(crate) fn set_hyph_index(&mut self) {
        let slot = self.hyph_start as usize + usize::from(self.cur_lang);
        self.hyph_index = if self.trie_char(slot) == u16::from(self.cur_lang) {
            self.trie_link(slot)
        } else {
            0
        };
    }

    /// The hyphenation code of `c` under the current `hyph_index`.
    #[inline]
    pub(crate) fn hyph_code(&self, eqtb: &EqtbState, c: i32) -> i32 {
        if self.hyph_index == 0 || c > 255 {
            return eqtb.lc_code(c);
        }
        let slot = self.hyph_index + c as usize;
        if self.trie_char(slot) == c as u16 {
            self.trie_tro[slot]
        } else {
            0
        }
    }

    /// The exception-dictionary hash of the word `hc[1..=n]`.
    pub(crate) fn exception_hash(&self, n: usize) -> usize {
        self.hc[1..=n]
            .iter()
            .fold(0, |h, &c| (h + h + c) % HYPH_PRIME) as usize
    }

    fn new_trie_op(&mut self, lang: u8, op: HyphOp) -> Result<u16, Abort> {
        let mut h = (i32::from(op.num)
            + 313 * i32::from(op.distance)
            + 361 * i32::from(op.next)
            + 1009 * i32::from(lang))
        .abs()
            % (2 * TRIE_OP_SIZE)
            - TRIE_OP_SIZE;

        loop {
            let l = self.trie_op_hash(h);
            if l == 0 {
                if self.trie_op_ptr == TRIE_OP_SIZE {
                    return Err(Abort::Overflow {
                        what: "pattern memory ops",
                        n: TRIE_OP_SIZE,
                    });
                }

                let used = &mut self.trie_used[usize::from(lang)];
                if *used == MAX_TRIE_OP {
                    return Err(Abort::Overflow {
                        what: "pattern memory ops per language",
                        n: i32::from(MAX_TRIE_OP - MIN_TRIE_OP),
                    });
                }
                *used += 1;
                let u = *used;

                self.trie_op_ptr += 1;
                self.max_op_used = self.max_op_used.max(u);

                let p = self.trie_op_ptr as usize;
                self.set_hyph_op(p, op);
                self.trie_op_lang[p] = lang;
                self.set_trie_op_hash(h, self.trie_op_ptr);
                self.trie_op_val[p] = u;
                return Ok(u);
            }

            let l = l as usize;
            if self.hyph_op(l) == op && self.trie_op_lang[l] == lang {
                return Ok(self.trie_op_val[l]);
            }

            if h > -TRIE_OP_SIZE {
                h -= 1;
            } else {
                h = TRIE_OP_SIZE;
            }
        }
    }

    fn trie_node(&mut self, p: usize) -> usize {
        let mut h = i32::from(self.trie_c[p])
            .wrapping_add(1009_i32.wrapping_mul(i32::from(self.trie_o[p])))
            .wrapping_add(2718_i32.wrapping_mul(self.trie_l[p] as i32))
            .wrapping_add(3142_i32.wrapping_mul(self.trie_r[p] as i32))
            .wrapping_abs()
            % self.trie_size;

        loop {
            let q = self.trie_hash(h as usize);
            if q == 0 {
                self.set_trie_hash(h as usize, p);
                return p;
            }

            if self.trie_c[q] == self.trie_c[p]
                && self.trie_o[q] == self.trie_o[p]
                && self.trie_l[q] == self.trie_l[p]
                && self.trie_r[q] == self.trie_r[p]
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

    fn compress_trie(&mut self, p: usize) -> usize {
        if p == 0 {
            return 0;
        }

        let l = self.compress_trie(self.trie_l(p));
        self.set_trie_l(p, l);
        let r = self.compress_trie(self.trie_r(p));
        self.set_trie_r(p, r);
        self.trie_node(p)
    }

    /// Place the family starting at `p` where each of its characters finds a free slot.
    fn first_fit(&mut self, p: usize) -> Result<(), Abort> {
        let c = usize::from(self.trie_c[p]);
        let max_hyph_char = self.max_hyph_char as usize;
        let mut z = self.trie_min[c] as usize;
        let mut h;

        loop {
            h = z - c;
            let top = h + max_hyph_char;
            if (self.trie_max as usize) < top {
                if self.trie_size as usize <= top {
                    return Err(Abort::Overflow {
                        what: "pattern memory",
                        n: self.trie_size,
                    });
                }

                loop {
                    self.trie_max += 1;
                    let max = self.trie_max as usize;
                    self.trie_taken[max] = false;
                    self.set_trie_link(max, max + 1);
                    self.set_trie_back(max, max - 1);
                    if max == top {
                        break;
                    }
                }
            }

            let mut found = !self.trie_taken[h];
            let mut q = self.trie_r(p);
            while found && q > 0 {
                if self.trie_link(h + usize::from(self.trie_c[q])) == 0 {
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

        self.trie_taken[h] = true;
        self.set_trie_hash(p, h);
        let mut q = p;
        loop {
            z = h + usize::from(self.trie_c[q]);
            let mut l = self.trie_back(z);
            let r = self.trie_link(z);
            self.set_trie_back(r, l);
            self.set_trie_link(l, r);
            self.set_trie_link(z, 0);

            if l < max_hyph_char {
                let ll = z.min(max_hyph_char);
                loop {
                    self.trie_min[l] = r as u32;
                    l += 1;
                    if l == ll {
                        break;
                    }
                }
            }

            q = self.trie_r(q);
            if q == 0 {
                break;
            }
        }
        Ok(())
    }

    fn trie_pack(&mut self, mut p: usize) -> Result<(), Abort> {
        loop {
            let q = self.trie_l(p);
            if q > 0 && self.trie_hash(q) == 0 {
                self.first_fit(q)?;
                self.trie_pack(q)?;
            }

            p = self.trie_r(p);
            if p == 0 {
                return Ok(());
            }
        }
    }

    fn trie_fix(&mut self, mut p: usize) {
        let z = self.trie_hash(p);
        loop {
            let q = self.trie_l(p);
            let c = self.trie_c[p];
            let slot = z + usize::from(c);
            self.set_trie_link(slot, self.trie_hash(q));
            self.trie_trc[slot] = c;
            self.trie_tro[slot] = i32::from(self.trie_o[p]);

            if q > 0 {
                self.trie_fix(q);
            }

            p = self.trie_r(p);
            if p == 0 {
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

        let r = self.compress_trie(self.trie_r(0));
        self.set_trie_r(0, r);
        let l = self.compress_trie(self.trie_l(0));
        self.set_trie_l(0, l);

        self.trie_hash[..=self.trie_ptr as usize].fill(0);

        for (min, p) in self.trie_min[..=BIGGEST_CHAR as usize].iter_mut().zip(1..) {
            *min = p;
        }

        self.set_trie_link(0, 1);
        self.trie_max = 0;

        let trie_l0 = self.trie_l(0);
        if trie_l0 != 0 {
            self.first_fit(trie_l0)?;
            self.trie_pack(trie_l0)?;
        }

        let trie_r0 = self.trie_r(0);
        if trie_r0 != 0 {
            if trie_l0 == 0 {
                for (min, p) in self.trie_min[..=255].iter_mut().zip(2..) {
                    *min = p;
                }
            }
            self.first_fit(trie_r0)?;
            self.trie_pack(trie_r0)?;
            self.hyph_start = self.trie_hash(trie_r0) as i32;
        }

        if self.trie_max == 0 {
            for r in 0..=self.max_hyph_char as usize {
                self.clear_trie_slot(r);
            }
            self.trie_max = self.max_hyph_char;
        } else {
            if trie_r0 > 0 {
                self.trie_fix(trie_r0);
            }
            if trie_l0 > 0 {
                self.trie_fix(trie_l0);
            }

            let mut r = 0;
            loop {
                let s = self.trie_link(r);
                self.clear_trie_slot(r);
                r = s;
                if r > self.trie_max as usize {
                    break;
                }
            }
        }

        self.trie_trc[0] = u16::from(b'?');
        self.trie_not_ready = false;
        Ok(())
    }

    /// Insert a node for `c` in front of `*p`, as `q`'s first child or right sibling.
    fn trie_insert_node(
        &mut self,
        q: usize,
        p: &mut usize,
        c: u16,
        first_child: bool,
    ) -> Result<(), Abort> {
        if self.trie_ptr == self.trie_size {
            return Err(Abort::Overflow {
                what: "pattern memory",
                n: self.trie_size,
            });
        }

        self.trie_ptr += 1;
        let new = self.trie_ptr as usize;
        self.set_trie_r(new, *p);
        *p = new;
        self.set_trie_l(new, 0);
        if first_child {
            self.set_trie_l(q, new);
        } else {
            self.set_trie_r(q, new);
        }
        self.trie_c[new] = c;
        self.trie_o[new] = MIN_TRIE_OP;
        Ok(())
    }

    /// Insert the pattern `hc[1..=k]` into the trie, returning its node, its op,
    /// and whether the node already carried an op (a duplicate pattern).
    fn insert_pattern_word(&mut self, k: usize) -> Result<(usize, u16, bool), Abort> {
        if self.hc[1] == 0 {
            self.hyf[0] = 0;
        }
        if self.hc[k] == 0 {
            self.hyf[k] = 0;
        }

        let mut v = MIN_TRIE_OP;
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

        let mut q = 0;
        self.hc[0] = i32::from(self.cur_lang);
        for l in 0..=k {
            let c = self.hc[l] as u16;
            let mut p = self.trie_l(q);
            let mut first_child = true;

            while p > 0 && c > self.trie_c[p] {
                q = p;
                p = self.trie_r(q);
                first_child = false;
            }

            if p == 0 || c < self.trie_c[p] {
                self.trie_insert_node(q, &mut p, c, first_child)?;
            }

            q = p;
        }

        Ok((q, v, self.trie_o[q] != MIN_TRIE_OP))
    }

    fn save_hyph_codes(&mut self, lc_codes: &[i32; 256]) -> Result<(), Abort> {
        let c = u16::from(self.cur_lang);
        let mut p = 0;

        let mut q;
        loop {
            q = p;
            p = self.trie_r(q);
            if p == 0 || c <= self.trie_c[p] {
                break;
            }
        }

        if p == 0 || c < self.trie_c[p] {
            self.trie_insert_node(q, &mut p, c, false)?;
        }

        q = p;
        p = self.trie_l(q);
        let mut first_child = true;

        for (ci, &lc) in (0u16..=255).zip(lc_codes.iter()) {
            if lc > 0 || (ci == 255 && first_child) {
                if p == 0 {
                    self.trie_insert_node(q, &mut p, ci, first_child)?;
                } else {
                    self.trie_c[p] = ci;
                }
                self.trie_o[p] = lc as u16;
                q = p;
                p = self.trie_r(q);
                first_child = false;
            }
        }

        if first_child {
            self.set_trie_l(q, 0);
        } else {
            self.set_trie_r(q, 0);
        }

        Ok(())
    }
}

/// `\XeTeXhyphenatablelength` as a word length.
fn max_word_len(eqtb: &EqtbState) -> usize {
    usize::try_from(eqtb.max_hyphenatable_length()).unwrap_or(0)
}

/// Finish an error begun at the call site: set its help text (in display
/// order) and run the recoverable `error()`.
fn raise_error(st: &mut EngineState, help: &[&'static str]) -> Result<(), Abort> {
    crate::output::capture_to_diagnostic(st, None);
    st.errors.help(help);
    crate::errors::error(st)
}

pub(crate) fn new_patterns(st: &mut EngineState) -> Result<(), Abort> {
    st.hyph.tables_gen = st.hyph.tables_gen.wrapping_add(1);

    if !st.hyph.trie_not_ready {
        crate::output::error_here_with_diagnostic(st, "Too late for ");
        crate::output::print_esc(st, "patterns");
        raise_error(
            st,
            &["All patterns must be given before typesetting begins."],
        )?;

        let _ = crate::scan::scan_toks(st, false, false)?;
        st.mem.flush_list(st.scanner.def_ref);
        return Ok(());
    }

    st.hyph.cur_lang = crate::helpers::norm_lang(st.eqtb.int_par(IntPar::Language));

    crate::scan::scan_left_brace(st)?;
    let mut k = 0;
    st.hyph.hyf[0] = 0;
    let mut digit_sensed = false;

    loop {
        crate::scan::get_x_token(st)?;
        match st.scanner.cur_cmd {
            LETTER | OTHER_CHAR => {
                let cur_chr = st.scanner.cur_chr;
                let digit = u8::try_from(cur_chr).ok().filter(u8::is_ascii_digit);
                if let Some(d) = digit.filter(|_| !digit_sensed) {
                    if k < max_word_len(&st.eqtb) {
                        st.hyph.hyf[k] = d - b'0';
                        digit_sensed = true;
                    }
                } else {
                    if cur_chr == i32::from(b'.') {
                        st.scanner.cur_chr = 0;
                    } else {
                        st.scanner.cur_chr = st.eqtb.lc_code(cur_chr);
                        if st.scanner.cur_chr == 0 {
                            crate::output::error_here_with_diagnostic(st, "Nonletter");
                            raise_error(st, &[APPENDIX_H])?;
                        }
                    }

                    if st.scanner.cur_chr > st.hyph.max_hyph_char {
                        st.hyph.max_hyph_char = st.scanner.cur_chr;
                    }

                    if k < max_word_len(&st.eqtb) {
                        k += 1;
                        st.hyph.hc[k] = st.scanner.cur_chr;
                        st.hyph.hyf[k] = 0;
                        digit_sensed = false;
                    }
                }
            }
            SPACER | RIGHT_BRACE => {
                if k > 0 {
                    let (q, op, duplicate) = st.hyph.insert_pattern_word(k)?;
                    if duplicate {
                        crate::output::error_here_with_diagnostic(st, "Duplicate pattern");
                        raise_error(st, &[APPENDIX_H])?;
                    }
                    st.hyph.trie_o[q] = op;
                }

                if st.scanner.cur_cmd == RIGHT_BRACE {
                    break;
                }

                k = 0;
                st.hyph.hyf[0] = 0;
                digit_sensed = false;
            }
            _ => {
                crate::output::error_here_with_diagnostic(st, "Bad ");
                crate::output::print_esc(st, "patterns");
                raise_error(st, &[APPENDIX_H])?;
            }
        }
    }

    if st.eqtb.int_par(IntPar::SavingHyphCodes) > 0 {
        let lc_codes: [i32; 256] = std::array::from_fn(|ci| st.eqtb.lc_code(ci as i32));
        st.hyph.save_hyph_codes(&lc_codes)?;
    }
    Ok(())
}

pub(crate) fn new_hyph_exceptions(st: &mut EngineState) -> Result<(), Abort> {
    st.hyph.tables_gen = st.hyph.tables_gen.wrapping_add(1);

    crate::scan::scan_left_brace(st)?;

    st.hyph.cur_lang = crate::helpers::norm_lang(st.eqtb.int_par(IntPar::Language));
    if st.hyph.trie_not_ready {
        st.hyph.hyph_index = 0;
    } else {
        st.hyph.set_hyph_index();
    }

    let mut n = 0;
    let mut p = MemPtr::NULL;

    loop {
        crate::scan::get_x_token(st)?;

        // reswitch: \char re-dispatches as CHAR_GIVEN
        loop {
            match st.scanner.cur_cmd {
                LETTER | OTHER_CHAR | CHAR_GIVEN => {
                    if st.scanner.cur_chr == i32::from(b'-') {
                        if n < max_word_len(&st.eqtb) {
                            let q = st.mem.get_avail()?;
                            q.set_link(&mut st.mem, p);
                            q.set_info(&mut st.mem, n as i32);
                            p = q;
                        }
                    } else {
                        st.hyph.hc[0] = st.hyph.hyph_code(&st.eqtb, st.scanner.cur_chr);

                        if st.hyph.hc[0] == 0 {
                            crate::output::error_here_with_diagnostic(st, "Not a letter");
                            raise_error(
                                st,
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
                CHAR_NUM => {
                    crate::scan::scan_char_num(st)?;
                    st.scanner.cur_chr = st.scanner.cur_val;
                    st.scanner.cur_cmd = CHAR_GIVEN;
                }
                SPACER | RIGHT_BRACE => {
                    if n > 1 {
                        store_hyph_exception(st, n, p)?;
                    }

                    if st.scanner.cur_cmd == RIGHT_BRACE {
                        return Ok(());
                    }

                    n = 0;
                    p = MemPtr::NULL;
                    break;
                }
                _ => {
                    crate::output::error_here_with_diagnostic(st, "Improper ");
                    crate::output::print_esc(st, "hyphenation");
                    crate::output::print(st, " will be flushed");
                    raise_error(
                        st,
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

/// Enter the word `hc[1..=n]` with hyphen positions `p` into the exception table.
fn store_hyph_exception(st: &mut EngineState, mut n: usize, p: MemPtr) -> Result<(), Abort> {
    n += 1;
    st.hyph.hc[n] = i32::from(st.hyph.cur_lang);
    if st.strings.pool_ptr + n as i32 > st.strings.pool_size {
        return Err(Abort::PoolOverflow);
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
            what: "exception dictionary",
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
            st.strings.str_ptr -= 1;
            st.strings.pool_ptr = st.strings.cur_string_start();
            s = k;
            hyph.hyph_count -= 1;
            break;
        }

        if hyph.hyph_link[h] == 0 {
            hyph.hyph_link[h] = hyph.hyph_next as u16;
            if hyph.hyph_next >= hyph.hyph_size {
                hyph.hyph_next = HYPH_PRIME;
            }
            if hyph.hyph_next > HYPH_PRIME {
                hyph.hyph_next += 1;
            }
        }
        h = usize::from(hyph.hyph_link[h]) - 1;
    }

    hyph.hyph_word[h] = s;
    hyph.hyph_list[h] = p;
    Ok(())
}
