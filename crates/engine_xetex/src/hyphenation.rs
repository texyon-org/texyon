//! Hyphenation patterns and exceptions, and the packed-trie builder.

use crate::cmd::{CHAR_GIVEN, CHAR_NUM, LETTER, OTHER_CHAR, RIGHT_BRACE, SPACER};
pub(crate) use crate::consts::BIGGEST_LANG;
use crate::consts::{
    BIGGEST_CHAR, INT_BASE, INT_PAR_LANGUAGE, INT_PAR_SAVING_HYPH_CODES, LC_CODE_BASE, MIN_TRIE_OP,
    TOO_BIG_CHAR,
};
use crate::main_control::{mc_propagate_error, MainControlAbort};
use crate::state::{EngineState, EqtbState, HyphState};
use crate::units::MemPtr;

pub(crate) const TRIE_OP_SIZE: i32 = 35_111;
const NEG_TRIE_OP_SIZE: i32 = -TRIE_OP_SIZE;
pub(crate) const MAX_TRIE_OP: u16 = 65_535;
pub(crate) const HYPH_PRIME: i32 = 607;

const APPENDIX_H: &str = "(See Appendix H.)";

#[inline]
fn trie_op_hash_in(hyph: &HyphState, k: i32) -> i32 {
    hyph.trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize]
}

#[inline]
fn set_trie_op_hash_in(hyph: &mut HyphState, k: i32, v: i32) {
    hyph.trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize] = v;
}

/// `\language` clamped to a valid language number (out of range means 0).
fn norm_lang(lang: i32) -> u8 {
    if lang <= 0 || lang > BIGGEST_LANG {
        0
    } else {
        lang as u8
    }
}

/// Point `hyph_index` at the `\lccode` table saved with `cur_lang`'s patterns, or 0.
pub(crate) fn set_hyph_index(hyph: &mut HyphState) {
    let slot = (hyph.hyph_start + hyph.cur_lang as i32) as usize;
    hyph.hyph_index = if hyph.trie_trc[slot] != hyph.cur_lang as u16 {
        0
    } else {
        hyph.trie_trl[slot]
    };
}

/// The hyphenation code of `c` under the current `hyph_index`.
#[inline]
pub(crate) fn hyph_code(hyph: &HyphState, eqtb: &EqtbState, c: i32) -> i32 {
    if hyph.hyph_index == 0 || c > 255 {
        eqtb.b32s1(LC_CODE_BASE + c)
    } else if hyph.trie_trc[(hyph.hyph_index + c) as usize] != c as u16 {
        0
    } else {
        hyph.trie_tro[(hyph.hyph_index + c) as usize]
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

fn new_trie_op_in(
    hyph: &mut HyphState,
    cur_lang: u8,
    d: i16,
    n: i16,
    v: u16,
) -> Result<u16, TrieAbort> {
    let mut h = (n as i32 + 313 * d as i32 + 361 * v as i32 + 1009 * cur_lang as i32).abs()
        % (TRIE_OP_SIZE - NEG_TRIE_OP_SIZE)
        + NEG_TRIE_OP_SIZE;

    loop {
        let l = trie_op_hash_in(hyph, h);
        if l == 0 {
            if hyph.trie_op_ptr == TRIE_OP_SIZE {
                return Err(TrieAbort::Ops(TRIE_OP_SIZE));
            }

            let lang = cur_lang as usize;
            let mut u = hyph.trie_used[lang];
            if u == MAX_TRIE_OP {
                return Err(TrieAbort::OpsPerLanguage(MAX_TRIE_OP as i32 - MIN_TRIE_OP));
            }

            hyph.trie_op_ptr += 1;
            u += 1;
            hyph.trie_used[lang] = u;
            hyph.max_op_used = hyph.max_op_used.max(u);

            let op_ptr = hyph.trie_op_ptr as usize;
            hyph.hyf_distance[op_ptr] = d;
            hyph.hyf_num[op_ptr] = n;
            hyph.hyf_next[op_ptr] = v;
            hyph.trie_op_lang[op_ptr] = cur_lang;
            set_trie_op_hash_in(hyph, h, hyph.trie_op_ptr);
            hyph.trie_op_val[op_ptr] = u;
            return Ok(u);
        }

        let op = l as usize;
        if hyph.hyf_distance[op] == d
            && hyph.hyf_num[op] == n
            && hyph.hyf_next[op] == v
            && hyph.trie_op_lang[op] == cur_lang
        {
            return Ok(hyph.trie_op_val[op]);
        }

        if h > NEG_TRIE_OP_SIZE {
            h -= 1;
        } else {
            h = TRIE_OP_SIZE;
        }
    }
}

fn trie_node(hyph: &mut HyphState, p: i32) -> i32 {
    let pu = p as usize;
    let mut h = (hyph.trie_c[pu] as i32)
        .wrapping_add(1009_i32.wrapping_mul(hyph.trie_o[pu] as i32))
        .wrapping_add(2718_i32.wrapping_mul(hyph.trie_l[pu]))
        .wrapping_add(3142_i32.wrapping_mul(hyph.trie_r[pu]))
        .wrapping_abs()
        % hyph.trie_size;

    loop {
        let q = hyph.trie_hash[h as usize];
        if q == 0 {
            hyph.trie_hash[h as usize] = p;
            return p;
        }

        let qu = q as usize;
        if hyph.trie_c[qu] == hyph.trie_c[pu]
            && hyph.trie_o[qu] == hyph.trie_o[pu]
            && hyph.trie_l[qu] == hyph.trie_l[pu]
            && hyph.trie_r[qu] == hyph.trie_r[pu]
        {
            return q;
        }

        if h > 0 {
            h -= 1;
        } else {
            h = hyph.trie_size;
        }
    }
}

fn compress_trie(hyph: &mut HyphState, p: i32) -> i32 {
    if p == 0 {
        return 0;
    }

    hyph.trie_l[p as usize] = compress_trie(hyph, hyph.trie_l[p as usize]);
    hyph.trie_r[p as usize] = compress_trie(hyph, hyph.trie_r[p as usize]);
    trie_node(hyph, p)
}

fn first_fit(hyph: &mut HyphState, p: i32) -> Result<(), TrieAbort> {
    let c = hyph.trie_c[p as usize] as i32;
    let mut z = hyph.trie_min[c as usize];
    let mut h;

    loop {
        h = z - c;
        if hyph.trie_max < h + hyph.max_hyph_char {
            if hyph.trie_size <= h + hyph.max_hyph_char {
                return Err(TrieAbort::Patterns(hyph.trie_size));
            }

            loop {
                hyph.trie_max += 1;
                let max = hyph.trie_max as usize;
                hyph.trie_taken[max] = false;
                hyph.trie_trl[max] = hyph.trie_max + 1;
                hyph.trie_tro[max] = hyph.trie_max - 1;
                if hyph.trie_max == h + hyph.max_hyph_char {
                    break;
                }
            }
        }

        let mut found = !hyph.trie_taken[h as usize];
        let mut q = hyph.trie_r[p as usize];
        while found && q > 0 {
            if hyph.trie_trl[(h + hyph.trie_c[q as usize] as i32) as usize] == 0 {
                found = false;
            } else {
                q = hyph.trie_r[q as usize];
            }
        }

        if found {
            break;
        }

        z = hyph.trie_trl[z as usize];
    }

    hyph.trie_taken[h as usize] = true;
    hyph.trie_hash[p as usize] = h;
    let mut q = p;
    loop {
        z = h + hyph.trie_c[q as usize] as i32;
        let mut l = hyph.trie_tro[z as usize];
        let r = hyph.trie_trl[z as usize];
        hyph.trie_tro[r as usize] = l;
        hyph.trie_trl[l as usize] = r;
        hyph.trie_trl[z as usize] = 0;

        if l < hyph.max_hyph_char {
            let ll = z.min(hyph.max_hyph_char);
            loop {
                hyph.trie_min[l as usize] = r;
                l += 1;
                if l == ll {
                    break;
                }
            }
        }

        q = hyph.trie_r[q as usize];
        if q == 0 {
            break;
        }
    }
    Ok(())
}

fn trie_pack(hyph: &mut HyphState, mut p: i32) -> Result<(), TrieAbort> {
    loop {
        let q = hyph.trie_l[p as usize];
        if q > 0 && hyph.trie_hash[q as usize] == 0 {
            first_fit(hyph, q)?;
            trie_pack(hyph, q)?;
        }

        p = hyph.trie_r[p as usize];
        if p == 0 {
            return Ok(());
        }
    }
}

fn trie_fix(hyph: &mut HyphState, mut p: i32) {
    let z = hyph.trie_hash[p as usize];
    loop {
        let q = hyph.trie_l[p as usize];
        let c = hyph.trie_c[p as usize];
        let slot = (z + c as i32) as usize;
        hyph.trie_trl[slot] = hyph.trie_hash[q as usize];
        hyph.trie_trc[slot] = c;
        hyph.trie_tro[slot] = hyph.trie_o[p as usize] as i32;

        if q > 0 {
            trie_fix(hyph, q);
        }

        p = hyph.trie_r[p as usize];
        if p == 0 {
            break;
        }
    }
}

pub(crate) fn init_trie_in(hyph: &mut HyphState) -> Result<(), TrieAbort> {
    hyph.tables_gen = hyph.tables_gen.wrapping_add(1);
    hyph.max_hyph_char += 1;
    hyph.op_start[0] = -MIN_TRIE_OP;

    for j in 1..=BIGGEST_LANG as usize {
        hyph.op_start[j] = hyph.op_start[j - 1] + hyph.trie_used[j - 1] as i32;
    }

    let op_ptr = hyph.trie_op_ptr;
    for j in 1..=op_ptr {
        let lang = hyph.trie_op_lang[j as usize] as usize;
        let val = hyph.op_start[lang] + hyph.trie_op_val[j as usize] as i32;
        set_trie_op_hash_in(hyph, j, val);
    }

    for j in 1..=op_ptr {
        while trie_op_hash_in(hyph, j) > j {
            let k = trie_op_hash_in(hyph, j);
            let (ju, ku) = (j as usize, k as usize);
            hyph.hyf_distance.swap(ju, ku);
            hyph.hyf_num.swap(ju, ku);
            hyph.hyf_next.swap(ju, ku);
            set_trie_op_hash_in(hyph, j, trie_op_hash_in(hyph, k));
            set_trie_op_hash_in(hyph, k, k);
        }
    }

    hyph.trie_hash[..=hyph.trie_size as usize].fill(0);

    hyph.trie_r[0] = compress_trie(hyph, hyph.trie_r[0]);
    hyph.trie_l[0] = compress_trie(hyph, hyph.trie_l[0]);

    hyph.trie_hash[..=hyph.trie_ptr as usize].fill(0);

    for p in 0..=BIGGEST_CHAR {
        hyph.trie_min[p as usize] = p + 1;
    }

    hyph.trie_trl[0] = 1;
    hyph.trie_max = 0;

    let trie_l0 = hyph.trie_l[0];
    if trie_l0 != 0 {
        first_fit(hyph, trie_l0)?;
        trie_pack(hyph, trie_l0)?;
    }

    let trie_r0 = hyph.trie_r[0];
    if trie_r0 != 0 {
        if trie_l0 == 0 {
            for p in 0..=255 {
                hyph.trie_min[p as usize] = p + 2;
            }
        }
        first_fit(hyph, trie_r0)?;
        trie_pack(hyph, trie_r0)?;
        hyph.hyph_start = hyph.trie_hash[trie_r0 as usize];
    }

    if hyph.trie_max == 0 {
        for r in 0..=hyph.max_hyph_char as usize {
            hyph.trie_trl[r] = 0;
            hyph.trie_tro[r] = MIN_TRIE_OP;
            hyph.trie_trc[r] = 0;
        }
        hyph.trie_max = hyph.max_hyph_char;
    } else {
        if trie_r0 > 0 {
            trie_fix(hyph, trie_r0);
        }
        if trie_l0 > 0 {
            trie_fix(hyph, trie_l0);
        }

        let mut r = 0;
        loop {
            let s = hyph.trie_trl[r as usize];
            hyph.trie_trl[r as usize] = 0;
            hyph.trie_tro[r as usize] = MIN_TRIE_OP;
            hyph.trie_trc[r as usize] = 0;
            r = s;
            if r > hyph.trie_max {
                break;
            }
        }
    }

    hyph.trie_trc[0] = b'?' as u16;
    hyph.trie_not_ready = false;
    Ok(())
}

fn trie_insert_node_in(
    hyph: &mut HyphState,
    q: i32,
    p_ref: &mut i32,
    c: u16,
    first_child: bool,
) -> Result<(), TrieAbort> {
    if hyph.trie_ptr == hyph.trie_size {
        return Err(TrieAbort::Patterns(hyph.trie_size));
    }

    hyph.trie_ptr += 1;
    let p = hyph.trie_ptr;
    hyph.trie_r[p as usize] = *p_ref;
    *p_ref = p;
    hyph.trie_l[p as usize] = 0;
    if first_child {
        hyph.trie_l[q as usize] = p;
    } else {
        hyph.trie_r[q as usize] = p;
    }
    hyph.trie_c[p as usize] = c;
    hyph.trie_o[p as usize] = MIN_TRIE_OP as u16;
    Ok(())
}

/// Insert the pattern `hc[1..=k]` into the trie, returning its node, its op,
/// and whether the node already carried an op (a duplicate pattern).
fn insert_pattern_word_in(hyph: &mut HyphState, k: i16) -> Result<(i32, u16, bool), TrieAbort> {
    if hyph.hc[1] == 0 {
        hyph.hyf[0] = 0;
    }
    if hyph.hc[k as usize] == 0 {
        hyph.hyf[k as usize] = 0;
    }

    let mut l = k;
    let mut v: u16 = MIN_TRIE_OP as u16;
    loop {
        if hyph.hyf[l as usize] != 0 {
            v = new_trie_op_in(hyph, hyph.cur_lang, k - l, hyph.hyf[l as usize] as i16, v)?;
        }
        if l == 0 {
            break;
        }
        l -= 1;
    }

    let mut q: i32 = 0;
    hyph.hc[0] = hyph.cur_lang as i32;
    while l <= k {
        let c = hyph.hc[l as usize] as u16;
        l += 1;
        let mut p = hyph.trie_l[q as usize];
        let mut first_child = true;

        while p > 0 && c > hyph.trie_c[p as usize] {
            q = p;
            p = hyph.trie_r[q as usize];
            first_child = false;
        }

        if p == 0 || c < hyph.trie_c[p as usize] {
            trie_insert_node_in(hyph, q, &mut p, c, first_child)?;
        }

        q = p;
    }

    Ok((q, v, hyph.trie_o[q as usize] != MIN_TRIE_OP as u16))
}

fn save_hyph_codes_in(hyph: &mut HyphState, lc_codes: &[i32; 256]) -> Result<(), TrieAbort> {
    let c = hyph.cur_lang as u16;
    let mut p: i32 = 0;

    let mut q: i32;
    loop {
        q = p;
        p = hyph.trie_r[q as usize];
        if p == 0 || c <= hyph.trie_c[p as usize] {
            break;
        }
    }

    if p == 0 || c < hyph.trie_c[p as usize] {
        trie_insert_node_in(hyph, q, &mut p, c, false)?;
    }

    q = p;
    p = hyph.trie_l[q as usize];
    let mut first_child = true;

    for (ci, &lc) in (0u16..=255).zip(lc_codes.iter()) {
        if lc > 0 || (ci == 255 && first_child) {
            if p == 0 {
                trie_insert_node_in(hyph, q, &mut p, ci, first_child)?;
            } else {
                hyph.trie_c[p as usize] = ci;
            }
            hyph.trie_o[p as usize] = lc as u16;
            q = p;
            p = hyph.trie_r[q as usize];
            first_child = false;
        }
    }

    if first_child {
        hyph.trie_l[q as usize] = 0;
    } else {
        hyph.trie_r[q as usize] = 0;
    }

    Ok(())
}

/// Finish an error begun at the call site: set its help text (in display
/// order) and run the recoverable `error()`.
fn raise_error(st: &mut EngineState, help: &[&'static str]) -> Result<(), MainControlAbort> {
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help(help);
    mc_propagate_error(crate::errors::error_in(st))
}

pub(crate) fn new_patterns_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    st.hyph.tables_gen = st.hyph.tables_gen.wrapping_add(1);

    if !st.hyph.trie_not_ready {
        crate::output::error_here_with_diagnostic_in(st, c"Too late for ".as_ptr());
        crate::output::print_esc_cstr_in(st, c"patterns".as_ptr());
        raise_error(
            st,
            &["All patterns must be given before typesetting begins."],
        )?;

        let _ = crate::scan::scan_toks_in(st, false, false)?;
        crate::mem::flush_list(&mut st.mem, st.scanner.def_ref);
        return Ok(());
    }

    st.hyph.cur_lang = norm_lang(st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE));

    crate::scan::scan_left_brace_in(st)?;
    let mut k: i16 = 0;
    st.hyph.hyf[0] = 0;
    let mut digit_sensed = false;

    loop {
        crate::scan::get_x_token_in(st)?;
        match st.scanner.cur_cmd {
            LETTER | OTHER_CHAR => {
                let cur_chr = st.scanner.cur_chr;
                if digit_sensed || !(b'0' as i32..=b'9' as i32).contains(&cur_chr) {
                    if cur_chr == b'.' as i32 {
                        st.scanner.cur_chr = 0;
                    } else {
                        st.scanner.cur_chr = st.eqtb.b32s1(LC_CODE_BASE + cur_chr);
                        if st.scanner.cur_chr == 0 {
                            crate::output::error_here_with_diagnostic_in(st, c"Nonletter".as_ptr());
                            raise_error(st, &[APPENDIX_H])?;
                        }
                    }

                    if st.scanner.cur_chr > st.hyph.max_hyph_char {
                        st.hyph.max_hyph_char = st.scanner.cur_chr;
                    }

                    if k < crate::scan::max_hyphenatable_length_in(&st.eqtb) as i16 {
                        k += 1;
                        st.hyph.hc[k as usize] = st.scanner.cur_chr;
                        st.hyph.hyf[k as usize] = 0;
                        digit_sensed = false;
                    }
                } else if k < crate::scan::max_hyphenatable_length_in(&st.eqtb) as i16 {
                    st.hyph.hyf[k as usize] = (cur_chr - b'0' as i32) as u8;
                    digit_sensed = true;
                }
            }
            SPACER | RIGHT_BRACE => {
                if k > 0 {
                    let (q, op, duplicate) = insert_pattern_word_in(&mut st.hyph, k)?;
                    if duplicate {
                        crate::output::error_here_with_diagnostic_in(
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
                crate::output::error_here_with_diagnostic_in(st, c"Bad ".as_ptr());
                crate::output::print_esc_cstr_in(st, c"patterns".as_ptr());
                raise_error(st, &[APPENDIX_H])?;
            }
        }
    }

    if st.eqtb.b32s1(INT_BASE + INT_PAR_SAVING_HYPH_CODES) > 0 {
        let lc_codes: [i32; 256] =
            std::array::from_fn(|ci| st.eqtb.b32s1(LC_CODE_BASE + ci as i32));
        save_hyph_codes_in(&mut st.hyph, &lc_codes)?;
    }
    Ok(())
}

pub(crate) fn new_hyph_exceptions_in(st: &mut EngineState) -> Result<(), MainControlAbort> {
    st.hyph.tables_gen = st.hyph.tables_gen.wrapping_add(1);

    crate::scan::scan_left_brace_in(st)?;

    st.hyph.cur_lang = norm_lang(st.eqtb.b32s1(INT_BASE + INT_PAR_LANGUAGE));
    if st.hyph.trie_not_ready {
        st.hyph.hyph_index = 0;
    } else {
        set_hyph_index(&mut st.hyph);
    }

    let mut n: i16 = 0;
    let mut p = MemPtr::NULL;

    loop {
        crate::scan::get_x_token_in(st)?;

        // reswitch: \char re-dispatches as CHAR_GIVEN
        loop {
            match st.scanner.cur_cmd {
                LETTER | OTHER_CHAR | CHAR_GIVEN => {
                    if st.scanner.cur_chr == b'-' as i32 {
                        if n < crate::scan::max_hyphenatable_length_in(&st.eqtb) as i16 {
                            let q = crate::scan::get_avail_scc(st)?;
                            q.set_link(&mut st.mem, p);
                            q.set_info(&mut st.mem, n as i32);
                            p = q;
                        }
                    } else {
                        st.hyph.hc[0] = hyph_code(&st.hyph, &st.eqtb, st.scanner.cur_chr);

                        if st.hyph.hc[0] == 0 {
                            crate::output::error_here_with_diagnostic_in(
                                st,
                                c"Not a letter".as_ptr(),
                            );
                            raise_error(
                                st,
                                &[
                                    r"Letters in \hyphenation words must have \lccode>0.",
                                    "Proceed; I'll ignore the character I just read.",
                                ],
                            )?;
                        } else if n < crate::scan::max_hyphenatable_length_in(&st.eqtb) as i16 {
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
                    crate::scan::scan_char_num_in(st)?;
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
                    crate::output::error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
                    crate::output::print_esc_cstr_in(st, c"hyphenation".as_ptr());
                    crate::output::print_cstr_in(st, c" will be flushed".as_ptr());
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

    let mut s = crate::string_pool::make_string(&mut st.strings)?;

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

        if crate::string_pool::length(&st.strings, k) == crate::string_pool::length(&st.strings, s)
        {
            let strings = &st.strings;
            let u = strings.str_start[(k - TOO_BIG_CHAR) as usize] as usize;
            let end = strings.str_start[(k + 1 - TOO_BIG_CHAR) as usize] as usize;
            let v = strings.str_start[(s - TOO_BIG_CHAR) as usize] as usize;
            if strings.str_pool[u..end] == strings.str_pool[v..v + (end - u)] {
                st.strings.str_ptr -= 1;
                st.strings.pool_ptr =
                    st.strings.str_start[(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
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
