//! Hyphenation patterns and exceptions, and the packed-trie builder.

use crate::cmd::{CHAR_GIVEN, CHAR_NUM, LETTER, OTHER_CHAR, RIGHT_BRACE, SPACER};
pub(crate) use crate::consts::BIGGEST_LANG;
use crate::consts::{
    BIGGEST_CHAR, INT_PAR_LANGUAGE, INT_PAR_SAVING_HYPH_CODES, MIN_TRIE_OP, TOO_BIG_CHAR,
};
use crate::main_control::{mc_propagate_error, MainControlAbort};
use crate::state::{EngineState, EqtbState, HyphState};
use crate::units::MemPtr;

pub(crate) const TRIE_OP_SIZE: i32 = 35_111;
const NEG_TRIE_OP_SIZE: i32 = -TRIE_OP_SIZE;
pub(crate) const MAX_TRIE_OP: u16 = 65_535;
pub(crate) const HYPH_PRIME: i32 = 607;

const APPENDIX_H: &str = "(See Appendix H.)";

impl HyphState {
    #[inline]
    fn trie_op_hash(&self, k: i32) -> i32 {
        self.trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize]
    }

    #[inline]
    fn set_trie_op_hash(&mut self, k: i32, v: i32) {
        self.trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize] = v;
    }

    /// `\language` clamped to a valid language number (out of range means 0).

    /// Point `hyph_index` at the `\lccode` table saved with `cur_lang`'s patterns, or 0.
    pub(crate) fn set_hyph_index(&mut self) {
        let slot = (self.hyph_start + self.cur_lang as i32) as usize;
        self.hyph_index = if self.trie_trc[slot] != self.cur_lang as u16 {
            0
        } else {
            self.trie_trl[slot]
        };
    }

    /// The hyphenation code of `c` under the current `hyph_index`.
    #[inline]
    pub(crate) fn hyph_code(&self, eqtb: &EqtbState, c: i32) -> i32 {
        if self.hyph_index == 0 || c > 255 {
            eqtb.lc_code(c)
        } else if self.trie_trc[(self.hyph_index + c) as usize] != c as u16 {
            0
        } else {
            self.trie_tro[(self.hyph_index + c) as usize]
        }
    }
}

pub(crate) enum TrieAbort {
    Patterns(i32),
    Ops(i32),
    OpsPerLanguage(i32),
}

impl TrieAbort {
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        let (what, n) = match self {
            TrieAbort::Patterns(n) => (c"pattern memory", n),
            TrieAbort::Ops(n) => (c"pattern memory ops", n),
            TrieAbort::OpsPerLanguage(n) => (c"pattern memory ops per language", n),
        };
        crate::errors::EngineAbort::Overflow {
            what: what.as_ptr(),
            n,
        }
    }
}

impl HyphState {
    fn new_trie_op(&mut self, cur_lang: u8, d: i16, n: i16, v: u16) -> Result<u16, TrieAbort> {
        let mut h = (n as i32 + 313 * d as i32 + 361 * v as i32 + 1009 * cur_lang as i32).abs()
            % (TRIE_OP_SIZE - NEG_TRIE_OP_SIZE)
            + NEG_TRIE_OP_SIZE;

        loop {
            let l = self.trie_op_hash(h);
            if l == 0 {
                if self.trie_op_ptr == TRIE_OP_SIZE {
                    return Err(TrieAbort::Ops(TRIE_OP_SIZE));
                }

                let lang = cur_lang as usize;
                let mut u = self.trie_used[lang];
                if u == MAX_TRIE_OP {
                    return Err(TrieAbort::OpsPerLanguage(MAX_TRIE_OP as i32 - MIN_TRIE_OP));
                }

                self.trie_op_ptr += 1;
                u += 1;
                self.trie_used[lang] = u;
                self.max_op_used = self.max_op_used.max(u);

                let op_ptr = self.trie_op_ptr as usize;
                self.hyf_distance[op_ptr] = d;
                self.hyf_num[op_ptr] = n;
                self.hyf_next[op_ptr] = v;
                self.trie_op_lang[op_ptr] = cur_lang;
                self.set_trie_op_hash(h, self.trie_op_ptr);
                self.trie_op_val[op_ptr] = u;
                return Ok(u);
            }

            let op = l as usize;
            if self.hyf_distance[op] == d
                && self.hyf_num[op] == n
                && self.hyf_next[op] == v
                && self.trie_op_lang[op] == cur_lang
            {
                return Ok(self.trie_op_val[op]);
            }

            if h > NEG_TRIE_OP_SIZE {
                h -= 1;
            } else {
                h = TRIE_OP_SIZE;
            }
        }
    }

    fn trie_node(&mut self, p: i32) -> i32 {
        let pu = p as usize;
        let mut h = (self.trie_c[pu] as i32)
            .wrapping_add(1009_i32.wrapping_mul(self.trie_o[pu] as i32))
            .wrapping_add(2718_i32.wrapping_mul(self.trie_l[pu]))
            .wrapping_add(3142_i32.wrapping_mul(self.trie_r[pu]))
            .wrapping_abs()
            % self.trie_size;

        loop {
            let q = self.trie_hash[h as usize];
            if q == 0 {
                self.trie_hash[h as usize] = p;
                return p;
            }

            let qu = q as usize;
            if self.trie_c[qu] == self.trie_c[pu]
                && self.trie_o[qu] == self.trie_o[pu]
                && self.trie_l[qu] == self.trie_l[pu]
                && self.trie_r[qu] == self.trie_r[pu]
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

    fn compress_trie(&mut self, p: i32) -> i32 {
        if p == 0 {
            return 0;
        }

        self.trie_l[p as usize] = self.compress_trie(self.trie_l[p as usize]);
        self.trie_r[p as usize] = self.compress_trie(self.trie_r[p as usize]);
        self.trie_node(p)
    }

    fn first_fit(&mut self, p: i32) -> Result<(), TrieAbort> {
        let c = self.trie_c[p as usize] as i32;
        let mut z = self.trie_min[c as usize];
        let mut h;

        loop {
            h = z - c;
            if self.trie_max < h + self.max_hyph_char {
                if self.trie_size <= h + self.max_hyph_char {
                    return Err(TrieAbort::Patterns(self.trie_size));
                }

                loop {
                    self.trie_max += 1;
                    let max = self.trie_max as usize;
                    self.trie_taken[max] = false;
                    self.trie_trl[max] = self.trie_max + 1;
                    self.trie_tro[max] = self.trie_max - 1;
                    if self.trie_max == h + self.max_hyph_char {
                        break;
                    }
                }
            }

            let mut found = !self.trie_taken[h as usize];
            let mut q = self.trie_r[p as usize];
            while found && q > 0 {
                if self.trie_trl[(h + self.trie_c[q as usize] as i32) as usize] == 0 {
                    found = false;
                } else {
                    q = self.trie_r[q as usize];
                }
            }

            if found {
                break;
            }

            z = self.trie_trl[z as usize];
        }

        self.trie_taken[h as usize] = true;
        self.trie_hash[p as usize] = h;
        let mut q = p;
        loop {
            z = h + self.trie_c[q as usize] as i32;
            let mut l = self.trie_tro[z as usize];
            let r = self.trie_trl[z as usize];
            self.trie_tro[r as usize] = l;
            self.trie_trl[l as usize] = r;
            self.trie_trl[z as usize] = 0;

            if l < self.max_hyph_char {
                let ll = z.min(self.max_hyph_char);
                loop {
                    self.trie_min[l as usize] = r;
                    l += 1;
                    if l == ll {
                        break;
                    }
                }
            }

            q = self.trie_r[q as usize];
            if q == 0 {
                break;
            }
        }
        Ok(())
    }

    fn trie_pack(&mut self, mut p: i32) -> Result<(), TrieAbort> {
        loop {
            let q = self.trie_l[p as usize];
            if q > 0 && self.trie_hash[q as usize] == 0 {
                self.first_fit(q)?;
                self.trie_pack(q)?;
            }

            p = self.trie_r[p as usize];
            if p == 0 {
                return Ok(());
            }
        }
    }

    fn trie_fix(&mut self, mut p: i32) {
        let z = self.trie_hash[p as usize];
        loop {
            let q = self.trie_l[p as usize];
            let c = self.trie_c[p as usize];
            let slot = (z + c as i32) as usize;
            self.trie_trl[slot] = self.trie_hash[q as usize];
            self.trie_trc[slot] = c;
            self.trie_tro[slot] = self.trie_o[p as usize] as i32;

            if q > 0 {
                self.trie_fix(q);
            }

            p = self.trie_r[p as usize];
            if p == 0 {
                break;
            }
        }
    }

    pub(crate) fn init_trie(&mut self) -> Result<(), TrieAbort> {
        self.tables_gen = self.tables_gen.wrapping_add(1);
        self.max_hyph_char += 1;
        self.op_start[0] = -MIN_TRIE_OP;

        for j in 1..=BIGGEST_LANG as usize {
            self.op_start[j] = self.op_start[j - 1] + self.trie_used[j - 1] as i32;
        }

        let op_ptr = self.trie_op_ptr;
        for j in 1..=op_ptr {
            let lang = self.trie_op_lang[j as usize] as usize;
            let val = self.op_start[lang] + self.trie_op_val[j as usize] as i32;
            self.set_trie_op_hash(j, val);
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

        self.trie_r[0] = self.compress_trie(self.trie_r[0]);
        self.trie_l[0] = self.compress_trie(self.trie_l[0]);

        self.trie_hash[..=self.trie_ptr as usize].fill(0);

        for p in 0..=BIGGEST_CHAR {
            self.trie_min[p as usize] = p + 1;
        }

        self.trie_trl[0] = 1;
        self.trie_max = 0;

        let trie_l0 = self.trie_l[0];
        if trie_l0 != 0 {
            self.first_fit(trie_l0)?;
            self.trie_pack(trie_l0)?;
        }

        let trie_r0 = self.trie_r[0];
        if trie_r0 != 0 {
            if trie_l0 == 0 {
                for p in 0..=255 {
                    self.trie_min[p as usize] = p + 2;
                }
            }
            self.first_fit(trie_r0)?;
            self.trie_pack(trie_r0)?;
            self.hyph_start = self.trie_hash[trie_r0 as usize];
        }

        if self.trie_max == 0 {
            for r in 0..=self.max_hyph_char as usize {
                self.trie_trl[r] = 0;
                self.trie_tro[r] = MIN_TRIE_OP;
                self.trie_trc[r] = 0;
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
                let s = self.trie_trl[r as usize];
                self.trie_trl[r as usize] = 0;
                self.trie_tro[r as usize] = MIN_TRIE_OP;
                self.trie_trc[r as usize] = 0;
                r = s;
                if r > self.trie_max {
                    break;
                }
            }
        }

        self.trie_trc[0] = b'?' as u16;
        self.trie_not_ready = false;
        Ok(())
    }

    fn trie_insert_node(
        &mut self,
        q: i32,
        p_ref: &mut i32,
        c: u16,
        first_child: bool,
    ) -> Result<(), TrieAbort> {
        if self.trie_ptr == self.trie_size {
            return Err(TrieAbort::Patterns(self.trie_size));
        }

        self.trie_ptr += 1;
        let p = self.trie_ptr;
        self.trie_r[p as usize] = *p_ref;
        *p_ref = p;
        self.trie_l[p as usize] = 0;
        if first_child {
            self.trie_l[q as usize] = p;
        } else {
            self.trie_r[q as usize] = p;
        }
        self.trie_c[p as usize] = c;
        self.trie_o[p as usize] = MIN_TRIE_OP as u16;
        Ok(())
    }

    /// Insert the pattern `hc[1..=k]` into the trie, returning its node, its op,
    /// and whether the node already carried an op (a duplicate pattern).
    fn insert_pattern_word(&mut self, k: i16) -> Result<(i32, u16, bool), TrieAbort> {
        if self.hc[1] == 0 {
            self.hyf[0] = 0;
        }
        if self.hc[k as usize] == 0 {
            self.hyf[k as usize] = 0;
        }

        let mut l = k;
        let mut v: u16 = MIN_TRIE_OP as u16;
        loop {
            if self.hyf[l as usize] != 0 {
                v = self.new_trie_op(self.cur_lang, k - l, self.hyf[l as usize] as i16, v)?;
            }
            if l == 0 {
                break;
            }
            l -= 1;
        }

        let mut q: i32 = 0;
        self.hc[0] = self.cur_lang as i32;
        while l <= k {
            let c = self.hc[l as usize] as u16;
            l += 1;
            let mut p = self.trie_l[q as usize];
            let mut first_child = true;

            while p > 0 && c > self.trie_c[p as usize] {
                q = p;
                p = self.trie_r[q as usize];
                first_child = false;
            }

            if p == 0 || c < self.trie_c[p as usize] {
                self.trie_insert_node(q, &mut p, c, first_child)?;
            }

            q = p;
        }

        Ok((q, v, self.trie_o[q as usize] != MIN_TRIE_OP as u16))
    }

    fn save_hyph_codes(&mut self, lc_codes: &[i32; 256]) -> Result<(), TrieAbort> {
        let c = self.cur_lang as u16;
        let mut p: i32 = 0;

        let mut q: i32;
        loop {
            q = p;
            p = self.trie_r[q as usize];
            if p == 0 || c <= self.trie_c[p as usize] {
                break;
            }
        }

        if p == 0 || c < self.trie_c[p as usize] {
            self.trie_insert_node(q, &mut p, c, false)?;
        }

        q = p;
        p = self.trie_l[q as usize];
        let mut first_child = true;

        for (ci, &lc) in (0u16..=255).zip(lc_codes.iter()) {
            if lc > 0 || (ci == 255 && first_child) {
                if p == 0 {
                    self.trie_insert_node(q, &mut p, ci, first_child)?;
                } else {
                    self.trie_c[p as usize] = ci;
                }
                self.trie_o[p as usize] = lc as u16;
                q = p;
                p = self.trie_r[q as usize];
                first_child = false;
            }
        }

        if first_child {
            self.trie_l[q as usize] = 0;
        } else {
            self.trie_r[q as usize] = 0;
        }

        Ok(())
    }
}

/// Finish an error begun at the call site: set its help text (in display
/// order) and run the recoverable `error()`.
fn raise_error(st: &mut EngineState, help: &[&'static str]) -> Result<(), MainControlAbort> {
    crate::output::capture_to_diagnostic(st, std::ptr::null_mut());
    st.errors.help(help);
    mc_propagate_error(crate::errors::error(st))
}

pub(crate) fn new_patterns(st: &mut EngineState) -> Result<(), MainControlAbort> {
    st.hyph.tables_gen = st.hyph.tables_gen.wrapping_add(1);

    if !st.hyph.trie_not_ready {
        crate::output::error_here_with_diagnostic(st, c"Too late for ".as_ptr());
        crate::output::print_esc_cstr(st, c"patterns".as_ptr());
        raise_error(
            st,
            &["All patterns must be given before typesetting begins."],
        )?;

        let _ = crate::scan::scan_toks(st, false, false)?;
        st.mem.flush_list(st.scanner.def_ref);
        return Ok(());
    }

    st.hyph.cur_lang = crate::helpers::norm_lang(st.eqtb.int_par(INT_PAR_LANGUAGE));

    crate::scan::scan_left_brace(st)?;
    let mut k: i16 = 0;
    st.hyph.hyf[0] = 0;
    let mut digit_sensed = false;

    loop {
        crate::scan::get_x_token(st)?;
        match st.scanner.cur_cmd {
            LETTER | OTHER_CHAR => {
                let cur_chr = st.scanner.cur_chr;
                if digit_sensed || !(b'0' as i32..=b'9' as i32).contains(&cur_chr) {
                    if cur_chr == b'.' as i32 {
                        st.scanner.cur_chr = 0;
                    } else {
                        st.scanner.cur_chr = st.eqtb.lc_code(cur_chr);
                        if st.scanner.cur_chr == 0 {
                            crate::output::error_here_with_diagnostic(st, c"Nonletter".as_ptr());
                            raise_error(st, &[APPENDIX_H])?;
                        }
                    }

                    if st.scanner.cur_chr > st.hyph.max_hyph_char {
                        st.hyph.max_hyph_char = st.scanner.cur_chr;
                    }

                    if k < st.eqtb.max_hyphenatable_length() as i16 {
                        k += 1;
                        st.hyph.hc[k as usize] = st.scanner.cur_chr;
                        st.hyph.hyf[k as usize] = 0;
                        digit_sensed = false;
                    }
                } else if k < st.eqtb.max_hyphenatable_length() as i16 {
                    st.hyph.hyf[k as usize] = (cur_chr - b'0' as i32) as u8;
                    digit_sensed = true;
                }
            }
            SPACER | RIGHT_BRACE => {
                if k > 0 {
                    let (q, op, duplicate) = st.hyph.insert_pattern_word(k)?;
                    if duplicate {
                        crate::output::error_here_with_diagnostic(
                            st,
                            c"Duplicate pattern".as_ptr(),
                        );
                        raise_error(st, &[APPENDIX_H])?;
                    }
                    st.hyph.trie_o[q as usize] = op;
                }

                if st.scanner.cur_cmd == RIGHT_BRACE {
                    break;
                }

                k = 0;
                st.hyph.hyf[0] = 0;
                digit_sensed = false;
            }
            _ => {
                crate::output::error_here_with_diagnostic(st, c"Bad ".as_ptr());
                crate::output::print_esc_cstr(st, c"patterns".as_ptr());
                raise_error(st, &[APPENDIX_H])?;
            }
        }
    }

    if st.eqtb.int_par(INT_PAR_SAVING_HYPH_CODES) > 0 {
        let lc_codes: [i32; 256] = std::array::from_fn(|ci| st.eqtb.lc_code(ci as i32));
        st.hyph.save_hyph_codes(&lc_codes)?;
    }
    Ok(())
}

pub(crate) fn new_hyph_exceptions(st: &mut EngineState) -> Result<(), MainControlAbort> {
    st.hyph.tables_gen = st.hyph.tables_gen.wrapping_add(1);

    crate::scan::scan_left_brace(st)?;

    st.hyph.cur_lang = crate::helpers::norm_lang(st.eqtb.int_par(INT_PAR_LANGUAGE));
    if st.hyph.trie_not_ready {
        st.hyph.hyph_index = 0;
    } else {
        st.hyph.set_hyph_index();
    }

    let mut n: i16 = 0;
    let mut p = MemPtr::NULL;

    loop {
        crate::scan::get_x_token(st)?;

        // reswitch: \char re-dispatches as CHAR_GIVEN
        loop {
            match st.scanner.cur_cmd {
                LETTER | OTHER_CHAR | CHAR_GIVEN => {
                    if st.scanner.cur_chr == b'-' as i32 {
                        if n < st.eqtb.max_hyphenatable_length() as i16 {
                            let q = crate::scan::get_avail_scc(st)?;
                            q.set_link(&mut st.mem, p);
                            q.set_info(&mut st.mem, n as i32);
                            p = q;
                        }
                    } else {
                        st.hyph.hc[0] = st.hyph.hyph_code(&st.eqtb, st.scanner.cur_chr);

                        if st.hyph.hc[0] == 0 {
                            crate::output::error_here_with_diagnostic(st, c"Not a letter".as_ptr());
                            raise_error(
                                st,
                                &[
                                    r"Letters in \hyphenation words must have \lccode>0.",
                                    "Proceed; I'll ignore the character I just read.",
                                ],
                            )?;
                        } else if n < st.eqtb.max_hyphenatable_length() as i16 {
                            let hc0 = st.hyph.hc[0];
                            n += 1;
                            if hc0 < 65536 {
                                st.hyph.hc[n as usize] = hc0;
                            } else {
                                st.hyph.hc[n as usize] = (hc0 - 65536) / 1024 + 0xD800;
                                n += 1;
                                st.hyph.hc[n as usize] = hc0 % 1024 + 0xDC00;
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
                    crate::output::error_here_with_diagnostic(st, c"Improper ".as_ptr());
                    crate::output::print_esc_cstr(st, c"hyphenation".as_ptr());
                    crate::output::print_cstr(st, c" will be flushed".as_ptr());
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

/// Enter the word `hc[1..n]` with hyphen positions `p` into the exception table.
fn store_hyph_exception(
    st: &mut EngineState,
    mut n: i16,
    p: MemPtr,
) -> Result<(), MainControlAbort> {
    n += 1;
    st.hyph.hc[n as usize] = st.hyph.cur_lang as i32;
    if st.strings.pool_ptr + n as i32 > st.strings.pool_size {
        return Err(MainControlAbort::Input(
            crate::input::InputAbort::Overflow {
                what: c"pool size".as_ptr(),
                n: st.strings.pool_size - st.strings.init_pool_ptr,
            },
        ));
    }

    let mut h: i32 = 0;
    for j in 1..=n as usize {
        h = (h + h + st.hyph.hc[j]) % HYPH_PRIME;
        st.strings.str_pool[st.strings.pool_ptr as usize] = st.hyph.hc[j] as u16;
        st.strings.pool_ptr += 1;
    }

    let mut s = st.strings.make_string()?;

    if st.hyph.hyph_next <= HYPH_PRIME {
        while st.hyph.hyph_next > 0 && st.hyph.hyph_word[(st.hyph.hyph_next - 1) as usize] > 0 {
            st.hyph.hyph_next -= 1;
        }
    }

    if st.hyph.hyph_count == st.hyph.hyph_size || st.hyph.hyph_next == 0 {
        return Err(MainControlAbort::Input(
            crate::input::InputAbort::Overflow {
                what: c"exception dictionary".as_ptr(),
                n: st.hyph.hyph_size,
            },
        ));
    }

    st.hyph.hyph_count += 1;

    loop {
        let k = st.hyph.hyph_word[h as usize];
        if k == 0 {
            break;
        }

        if st.strings.length(k) == st.strings.length(s) {
            let strings = &st.strings;
            let u = strings.str_start[(k - TOO_BIG_CHAR) as usize] as usize;
            let end = strings.str_start[(k + 1 - TOO_BIG_CHAR) as usize] as usize;
            let v = strings.str_start[(s - TOO_BIG_CHAR) as usize] as usize;
            if strings.str_pool[u..end] == strings.str_pool[v..v + (end - u)] {
                st.strings.str_ptr -= 1;
                st.strings.pool_ptr = st.strings.cur_string_start();
                s = k;
                st.hyph.hyph_count -= 1;
                break;
            }
        }

        if st.hyph.hyph_link[h as usize] == 0 {
            st.hyph.hyph_link[h as usize] = st.hyph.hyph_next as u16;
            if st.hyph.hyph_next >= st.hyph.hyph_size {
                st.hyph.hyph_next = HYPH_PRIME;
            }
            if st.hyph.hyph_next > HYPH_PRIME {
                st.hyph.hyph_next += 1;
            }
        }
        h = st.hyph.hyph_link[h as usize] as i32 - 1;
    }

    st.hyph.hyph_word[h as usize] = s;
    st.hyph.hyph_list[h as usize] = p.0;
    Ok(())
}
