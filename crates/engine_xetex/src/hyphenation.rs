//! Hyphenation pattern / exception loading and the packed-trie builder,
//! translated from the hyphenation part of xetex-ini.c.

// `TRIE_OP_SIZE`, `MAX_TRIE_OP`, and `BIGGEST_LANG` are `pub(crate)` because the
// format-dump code in `ini.rs` (later `format_io.rs`) still references them.
pub(crate) const TRIE_OP_SIZE: i32 = 35_111;
const NEG_TRIE_OP_SIZE: i32 = -35_111;
pub(crate) const MAX_TRIE_OP: u16 = 65_535;
const MIN_TRIE_OP: i32 = 0;
pub(crate) const BIGGEST_LANG: i32 = 255;

use crate::consts::{BIGGEST_CHAR, INT_BASE, LC_CODE_BASE, TOO_BIG_CHAR};
use crate::globals::{
    cur_chr, cur_cmd, cur_lang, cur_val, eqtb_b32s1, hc_entry, hyph_count, hyph_index,
    hyph_link_entry, hyph_next, hyph_size, hyph_start, hyph_word_entry, init_pool_ptr,
    max_hyph_char, pool_ptr, pool_size, set_cur_chr, set_cur_cmd, set_cur_lang, set_hc_entry,
    set_help_line, set_help_ptr, set_hyf_entry, set_hyph_count, set_hyph_index,
    set_hyph_link_entry, set_hyph_list_entry, set_hyph_next, set_hyph_word_entry,
    set_max_hyph_char, set_pool_ptr, set_str_pool, set_str_ptr, str_pool_entry, str_ptr, str_start,
    trie_not_ready, trie_trc_entry, trie_trl_entry, trie_tro_entry,
};
use crate::state::HyphState;

// ---- moved verbatim from ini.rs (workstream 02) ----

#[inline]
pub(crate) unsafe fn set_trie_op_hash(k: i32, v: i32) {
    crate::state::with_state_mut(|state| {
        state.hyph.trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize] = v;
    })
}

#[inline]
fn trie_op_hash_in(hyph: &HyphState, k: i32) -> i32 {
    hyph.trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize]
}

#[inline]
fn set_trie_op_hash_in(hyph: &mut HyphState, k: i32, v: i32) {
    hyph.trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize] = v;
}

enum TrieAbort {
    PatternMemoryOverflow(i32),
    PatternMemoryOpsOverflow(i32),
    PatternMemoryOpsPerLanguageOverflow(i32),
}

fn trie_abort(e: TrieAbort) -> ! {
    match e {
        TrieAbort::PatternMemoryOverflow(size) => unsafe {
            crate::errors::overflow(c"pattern memory".as_ptr(), size)
        },
        TrieAbort::PatternMemoryOpsOverflow(size) => unsafe {
            crate::errors::overflow(c"pattern memory ops".as_ptr(), size)
        },
        TrieAbort::PatternMemoryOpsPerLanguageOverflow(size) => unsafe {
            crate::errors::overflow(c"pattern memory ops per language".as_ptr(), size)
        },
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
                return Err(TrieAbort::PatternMemoryOpsOverflow(TRIE_OP_SIZE));
            }

            let lang = cur_lang as usize;
            let mut u = hyph.trie_used[lang];
            if u == MAX_TRIE_OP {
                return Err(TrieAbort::PatternMemoryOpsPerLanguageOverflow(
                    MAX_TRIE_OP as i32 - MIN_TRIE_OP,
                ));
            }

            hyph.trie_op_ptr += 1;
            u += 1;
            hyph.trie_used[lang] = u;
            if u > hyph.max_op_used {
                hyph.max_op_used = u;
            }

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

        if h > -TRIE_OP_SIZE {
            h -= 1;
        } else {
            h = TRIE_OP_SIZE;
        }
    }
}
// translated from xetex-ini.c:632 (trie_node)
fn trie_node(hyph: &mut HyphState, p: i32) -> i32 {
    let mut h = (hyph.trie_c[p as usize] as i32)
        .wrapping_add((1009_i32).wrapping_mul(hyph.trie_o[p as usize] as i32))
        .wrapping_add((2718_i32).wrapping_mul(hyph.trie_l[p as usize]))
        .wrapping_add((3142_i32).wrapping_mul(hyph.trie_r[p as usize]))
        .wrapping_abs()
        % hyph.trie_size;

    loop {
        let q = hyph.trie_hash[h as usize];
        if q == 0 {
            hyph.trie_hash[h as usize] = p;
            return p;
        }

        if hyph.trie_c[q as usize] == hyph.trie_c[p as usize]
            && hyph.trie_o[q as usize] == hyph.trie_o[p as usize]
            && hyph.trie_l[q as usize] == hyph.trie_l[p as usize]
            && hyph.trie_r[q as usize] == hyph.trie_r[p as usize]
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
// translated from xetex-ini.c:655 (compress_trie)
fn compress_trie(hyph: &mut HyphState, p: i32) -> i32 {
    if p == 0 {
        return 0;
    }

    let l = compress_trie(hyph, hyph.trie_l[p as usize]);
    hyph.trie_l[p as usize] = l;
    let r = compress_trie(hyph, hyph.trie_r[p as usize]);
    hyph.trie_r[p as usize] = r;
    trie_node(hyph, p)
}
// translated from xetex-ini.c:638 (first_fit)
fn first_fit(hyph: &mut HyphState, p: i32) -> Result<(), TrieAbort> {
    let c = hyph.trie_c[p as usize] as i32;
    let mut z = hyph.trie_min[c as usize];
    let mut h;

    loop {
        h = z - c;
        if hyph.trie_max < h + hyph.max_hyph_char {
            if hyph.trie_size <= h + hyph.max_hyph_char {
                return Err(TrieAbort::PatternMemoryOverflow(hyph.trie_size));
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
            let ll = if z < hyph.max_hyph_char {
                z
            } else {
                hyph.max_hyph_char
            };

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
// translated from xetex-ini.c:695 (trie_pack)
fn trie_pack(hyph: &mut HyphState, mut p: i32) -> Result<(), TrieAbort> {
    loop {
        let q = hyph.trie_l[p as usize];
        if q > 0 && hyph.trie_hash[q as usize] == 0 {
            first_fit(hyph, q)?;
            trie_pack(hyph, q)?;
        }

        p = hyph.trie_r[p as usize];
        if p == 0 {
            break;
        }
    }
    Ok(())
}
// translated from xetex-ini.c:708 (trie_fix)
fn trie_fix(hyph: &mut HyphState, mut p: i32) {
    let z = hyph.trie_hash[p as usize];
    loop {
        let q = hyph.trie_l[p as usize];
        let c = hyph.trie_c[p as usize];
        hyph.trie_trl[(z + c as i32) as usize] = hyph.trie_hash[q as usize];
        hyph.trie_trc[(z + c as i32) as usize] = c;
        hyph.trie_tro[(z + c as i32) as usize] = hyph.trie_o[p as usize] as i32;

        if q > 0 {
            trie_fix(hyph, q);
        }

        p = hyph.trie_r[p as usize];
        if p == 0 {
            break;
        }
    }
}
// translated from xetex-ini.c:858 (init_trie)
pub unsafe fn init_trie() {
    if let Err(e) = crate::state::with_state_mut(|state| init_trie_in(&mut state.hyph)) {
        trie_abort(e);
    }
}

fn init_trie_in(hyph: &mut HyphState) -> Result<(), TrieAbort> {
    hyph.max_hyph_char += 1;
    hyph.op_start[0] = -MIN_TRIE_OP;

    let mut j = 1;
    while j <= BIGGEST_LANG {
        let prev = (j - 1) as usize;
        hyph.op_start[j as usize] = hyph.op_start[prev] + hyph.trie_used[prev] as i32;
        j += 1;
    }

    let op_ptr = hyph.trie_op_ptr;
    j = 1;
    while j <= op_ptr {
        let lang = hyph.trie_op_lang[j as usize] as usize;
        let val = hyph.op_start[lang] + hyph.trie_op_val[j as usize] as i32;
        set_trie_op_hash_in(hyph, j, val);
        j += 1;
    }

    j = 1;
    while j <= op_ptr {
        while trie_op_hash_in(hyph, j) > j {
            let k = trie_op_hash_in(hyph, j);
            let ju = j as usize;
            let ku = k as usize;

            let t = hyph.hyf_distance[ku];
            hyph.hyf_distance[ku] = hyph.hyf_distance[ju];
            hyph.hyf_distance[ju] = t;

            let t = hyph.hyf_num[ku];
            hyph.hyf_num[ku] = hyph.hyf_num[ju];
            hyph.hyf_num[ju] = t;

            let t = hyph.hyf_next[ku];
            hyph.hyf_next[ku] = hyph.hyf_next[ju];
            hyph.hyf_next[ju] = t;

            let hash_k = trie_op_hash_in(hyph, k);
            set_trie_op_hash_in(hyph, j, hash_k);
            set_trie_op_hash_in(hyph, k, k);
        }
        j += 1;
    }

    let mut p = 0;
    while p <= hyph.trie_size {
        hyph.trie_hash[p as usize] = 0;
        p += 1;
    }

    let r = hyph.trie_r[0];
    hyph.trie_r[0] = compress_trie(hyph, r);
    let l = hyph.trie_l[0];
    hyph.trie_l[0] = compress_trie(hyph, l);

    p = 0;
    while p <= hyph.trie_ptr {
        hyph.trie_hash[p as usize] = 0;
        p += 1;
    }

    p = 0;
    while p <= BIGGEST_CHAR {
        hyph.trie_min[p as usize] = p + 1;
        p += 1;
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
            p = 0;
            while p <= 255 {
                hyph.trie_min[p as usize] = p + 2;
                p += 1;
            }
        }
        first_fit(hyph, trie_r0)?;
        trie_pack(hyph, trie_r0)?;
        hyph.hyph_start = hyph.trie_hash[trie_r0 as usize];
    }

    if hyph.trie_max == 0 {
        let mut r = 0;
        while r <= hyph.max_hyph_char {
            hyph.trie_trl[r as usize] = 0;
            hyph.trie_tro[r as usize] = MIN_TRIE_OP;
            hyph.trie_trc[r as usize] = 0;
            r += 1;
        }
        hyph.trie_max = hyph.max_hyph_char;
    } else {
        if trie_r0 > 0 {
            trie_fix(hyph, trie_r0);
        }
        if trie_l0 > 0 {
            trie_fix(hyph, trie_l0);
        }

        let trie_max_val = hyph.trie_max;
        let mut r = 0;
        loop {
            let s = hyph.trie_trl[r as usize];
            hyph.trie_trl[r as usize] = 0;
            hyph.trie_tro[r as usize] = MIN_TRIE_OP;
            hyph.trie_trc[r as usize] = 0;
            r = s;
            if r > trie_max_val {
                break;
            }
        }
    }

    hyph.trie_trc[0] = b'?' as u16;
    hyph.trie_not_ready = false;
    Ok(())
}

struct PatternWordInsert {
    q: i32,
    op: u16,
    duplicate: bool,
}

fn trie_insert_node_in(
    hyph: &mut HyphState,
    q: i32,
    p_ref: &mut i32,
    c: u16,
    first_child: bool,
) -> Result<(), TrieAbort> {
    if hyph.trie_ptr == hyph.trie_size {
        return Err(TrieAbort::PatternMemoryOverflow(hyph.trie_size));
    }

    hyph.trie_ptr += 1;
    hyph.trie_r[hyph.trie_ptr as usize] = *p_ref;
    *p_ref = hyph.trie_ptr;
    hyph.trie_l[*p_ref as usize] = 0;
    if first_child {
        hyph.trie_l[q as usize] = *p_ref;
    } else {
        hyph.trie_r[q as usize] = *p_ref;
    }
    hyph.trie_c[*p_ref as usize] = c;
    hyph.trie_o[*p_ref as usize] = MIN_TRIE_OP as u16;
    Ok(())
}

fn insert_pattern_word_in(hyph: &mut HyphState, k: i16) -> Result<PatternWordInsert, TrieAbort> {
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
        if l > 0 {
            l -= 1;
        } else {
            break;
        }
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

    Ok(PatternWordInsert {
        q,
        op: v,
        duplicate: hyph.trie_o[q as usize] != MIN_TRIE_OP as u16,
    })
}

fn set_pattern_word_op_in(hyph: &mut HyphState, insert: PatternWordInsert) {
    hyph.trie_o[insert.q as usize] = insert.op;
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

    for ci in 0u16..=255u16 {
        let lc = lc_codes[ci as usize];
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

// translated from xetex-ini.c:643 (new_patterns)
pub unsafe fn new_patterns() {
    const INT_PAR_LANGUAGE: i32 = 50;
    const INT_PAR_SAVING_HYPH_CODES: i32 = 64;
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const SPACER: u8 = 10;
    const RIGHT_BRACE_CMD: u8 = 2;

    if trie_not_ready() {
        let lang = eqtb_b32s1(INT_BASE + INT_PAR_LANGUAGE);
        if lang <= 0 || lang > BIGGEST_LANG {
            set_cur_lang(0);
        } else {
            set_cur_lang(lang as u8);
        }

        crate::scan::scan_left_brace();
        let mut k: i16 = 0;
        set_hyf_entry(0, 0);
        let mut digit_sensed = false;

        'done: {
            loop {
                crate::scan::get_x_token();
                match cur_cmd() {
                    LETTER | OTHER_CHAR => {
                        if digit_sensed || cur_chr() < b'0' as i32 || cur_chr() > b'9' as i32 {
                            if cur_chr() == b'.' as i32 {
                                set_cur_chr(0);
                            } else {
                                set_cur_chr(eqtb_b32s1(LC_CODE_BASE + cur_chr()));
                                if cur_chr() == 0 {
                                    crate::output::error_here_with_diagnostic(
                                        c"Nonletter".as_ptr(),
                                    );
                                    crate::output::capture_to_diagnostic(std::ptr::null_mut());
                                    set_help_ptr(1);
                                    set_help_line(0, c"(See Appendix H.)".as_ptr());
                                    crate::errors::error();
                                }
                            }

                            if cur_chr() > max_hyph_char() {
                                set_max_hyph_char(cur_chr());
                            }

                            if k < crate::scan::max_hyphenatable_length() as i16 {
                                k += 1;
                                set_hc_entry(k as usize, cur_chr());
                                set_hyf_entry(k as usize, 0);
                                digit_sensed = false;
                            }
                        } else if k < crate::scan::max_hyphenatable_length() as i16 {
                            set_hyf_entry(k as usize, (cur_chr() - b'0' as i32) as u8);
                            digit_sensed = true;
                        }
                    }
                    SPACER | RIGHT_BRACE_CMD => {
                        if k > 0 {
                            let insert = match crate::state::with_state_mut(|state| {
                                insert_pattern_word_in(&mut state.hyph, k)
                            }) {
                                Ok(insert) => insert,
                                Err(e) => trie_abort(e),
                            };

                            if insert.duplicate {
                                crate::output::error_here_with_diagnostic(
                                    c"Duplicate pattern".as_ptr(),
                                );
                                crate::output::capture_to_diagnostic(std::ptr::null_mut());
                                set_help_ptr(1);
                                set_help_line(0, c"(See Appendix H.)".as_ptr());
                                crate::errors::error();
                            }

                            crate::state::with_state_mut(|state| {
                                set_pattern_word_op_in(&mut state.hyph, insert);
                            });
                        }

                        if cur_cmd() == RIGHT_BRACE_CMD {
                            break 'done;
                        }

                        k = 0;
                        set_hyf_entry(0, 0);
                        digit_sensed = false;
                    }
                    _ => {
                        crate::output::error_here_with_diagnostic(c"Bad ".as_ptr());
                        crate::output::print_esc_cstr(c"patterns".as_ptr());
                        crate::output::capture_to_diagnostic(std::ptr::null_mut());
                        set_help_ptr(1);
                        set_help_line(0, c"(See Appendix H.)".as_ptr());
                        crate::errors::error();
                    }
                }
            }
        } // 'done

        // saving_hyph_codes: record lc_code table alongside the patterns
        if eqtb_b32s1(INT_BASE + INT_PAR_SAVING_HYPH_CODES) > 0 {
            let mut lc_codes = [0; 256];
            for ci in 0u16..=255u16 {
                lc_codes[ci as usize] = eqtb_b32s1(LC_CODE_BASE + ci as i32);
            }
            if let Err(e) =
                crate::state::with_state_mut(|state| save_hyph_codes_in(&mut state.hyph, &lc_codes))
            {
                trie_abort(e);
            }
        }
    } else {
        crate::output::error_here_with_diagnostic(c"Too late for ".as_ptr());
        crate::output::print_esc_cstr(c"patterns".as_ptr());
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(1);
        set_help_line(
            0,
            c"All patterns must be given before typesetting begins.".as_ptr(),
        );
        crate::errors::error();

        let _ = crate::scan::scan_toks(false, false);
        crate::mem::flush_list_global(crate::globals::def_ref());
    }
}
// translated from xetex-ini.c:863 (new_hyph_exceptions)
pub unsafe fn new_hyph_exceptions() {
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const CHAR_GIVEN: u8 = 68;
    const CHAR_NUM: u8 = 16;
    const SPACER: u8 = 10;
    const RIGHT_BRACE_CMD: u8 = 2;
    const INT_PAR_LANGUAGE: i32 = 50;
    const HYPH_PRIME: i32 = 607;

    crate::scan::scan_left_brace();

    let lang = eqtb_b32s1(INT_BASE + INT_PAR_LANGUAGE);
    if lang <= 0 || lang > BIGGEST_LANG {
        set_cur_lang(0);
    } else {
        set_cur_lang(lang as u8);
    }

    if trie_not_ready() {
        set_hyph_index(0);
    } else if trie_trc_entry(hyph_start() + cur_lang() as i32) != cur_lang() as u16 {
        set_hyph_index(0);
    } else {
        set_hyph_index(trie_trl_entry(hyph_start() + cur_lang() as i32));
    }

    // not_found1:
    let mut n: i16 = 0;
    let mut p: i32 = crate::mem::TEX_NULL;

    loop {
        crate::scan::get_x_token();

        // reswitch: re-evaluate after CHAR_NUM changes cur_cmd
        loop {
            match cur_cmd() {
                LETTER | OTHER_CHAR | CHAR_GIVEN => {
                    if cur_chr() == b'-' as i32 {
                        if n < crate::scan::max_hyphenatable_length() as i16 {
                            let q = crate::mem::get_avail_global();
                            crate::globals::set_mem_b32s1(q, p);
                            crate::globals::set_mem_b32s0(q, n as i32);
                            p = q;
                        }
                    } else {
                        let hc0 = if hyph_index() == 0 || cur_chr() > 255 {
                            eqtb_b32s1(LC_CODE_BASE + cur_chr())
                        } else if trie_trc_entry(hyph_index() + cur_chr()) != cur_chr() as u16 {
                            0
                        } else {
                            trie_tro_entry(hyph_index() + cur_chr())
                        };
                        set_hc_entry(0, hc0);

                        if hc_entry(0) == 0 {
                            crate::output::error_here_with_diagnostic(c"Not a letter".as_ptr());
                            crate::output::capture_to_diagnostic(std::ptr::null_mut());
                            set_help_ptr(2);
                            set_help_line(
                                1,
                                c"Letters in \\hyphenation words must have \\lccode>0.".as_ptr(),
                            );
                            set_help_line(
                                0,
                                c"Proceed; I'll ignore the character I just read.".as_ptr(),
                            );
                            crate::errors::error();
                        } else if n < crate::scan::max_hyphenatable_length() as i16 {
                            n += 1;
                            if hc_entry(0) < 65536 {
                                set_hc_entry(n as usize, hc_entry(0));
                            } else {
                                set_hc_entry(n as usize, (hc_entry(0) - 65536) / 1024 + 55296);
                                n += 1;
                                set_hc_entry(n as usize, hc_entry(0) % 1024 + 56320);
                            }
                        }
                    }
                    break;
                }
                CHAR_NUM => {
                    crate::scan::scan_char_num();
                    set_cur_chr(cur_val());
                    set_cur_cmd(CHAR_GIVEN);
                    // continue inner loop → re-run match with new cur_cmd
                }
                SPACER | RIGHT_BRACE_CMD => {
                    if n > 1 {
                        n += 1;
                        set_hc_entry(n as usize, cur_lang() as i32);
                        if pool_ptr() + n as i32 > pool_size() {
                            crate::errors::overflow(
                                c"pool size".as_ptr(),
                                pool_size() - init_pool_ptr(),
                            );
                        }

                        let mut h: i32 = 0;
                        for j in 1i16..=n {
                            h = (h + h + hc_entry(j as usize)) % HYPH_PRIME;
                            set_str_pool(pool_ptr(), hc_entry(j as usize) as u16);
                            set_pool_ptr(pool_ptr() + 1);
                        }

                        let mut s = crate::string_pool::make_string_global();

                        if hyph_next() <= HYPH_PRIME {
                            while hyph_next() > 0 && hyph_word_entry(hyph_next() - 1) > 0 {
                                set_hyph_next(hyph_next() - 1);
                            }
                        }

                        if hyph_count() == hyph_size() || hyph_next() == 0 {
                            crate::errors::overflow(c"exception dictionary".as_ptr(), hyph_size());
                        }

                        set_hyph_count(hyph_count() + 1);

                        // Hash lookup / insert into exception table
                        loop {
                            if hyph_word_entry(h) == 0 {
                                break;
                            }
                            let k = hyph_word_entry(h);

                            // Check if this slot holds the same word
                            let matched = if crate::string_pool::length_global(k)
                                != crate::string_pool::length_global(s)
                            {
                                false
                            } else {
                                let mut u = str_start(k - TOO_BIG_CHAR);
                                let mut v = str_start(s - TOO_BIG_CHAR);
                                let end = str_start(k + 1 - TOO_BIG_CHAR);
                                let mut ok = true;
                                while u != end {
                                    if str_pool_entry(u) != str_pool_entry(v) {
                                        ok = false;
                                        break;
                                    }
                                    u += 1;
                                    v += 1;
                                }
                                ok
                            };

                            if matched {
                                // found: merge into existing slot
                                set_str_ptr(str_ptr() - 1);
                                set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
                                s = hyph_word_entry(h);
                                set_hyph_count(hyph_count() - 1);
                                break;
                            }

                            // not_found: collision resolution
                            if hyph_link_entry(h) == 0 {
                                set_hyph_link_entry(h, hyph_next() as u16);
                                if hyph_next() >= hyph_size() {
                                    set_hyph_next(HYPH_PRIME);
                                }
                                if hyph_next() > HYPH_PRIME {
                                    set_hyph_next(hyph_next() + 1);
                                }
                            }
                            h = hyph_link_entry(h) as i32 - 1;
                        }

                        set_hyph_word_entry(h, s);
                        set_hyph_list_entry(h, p);
                    }

                    if cur_cmd() == RIGHT_BRACE_CMD {
                        return;
                    }

                    n = 0;
                    p = crate::mem::TEX_NULL;
                    break;
                }
                _ => {
                    crate::output::error_here_with_diagnostic(c"Improper ".as_ptr());
                    crate::output::print_esc_cstr(c"hyphenation".as_ptr());
                    crate::output::print_cstr(c" will be flushed".as_ptr());
                    crate::output::capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(2);
                    set_help_line(
                        1,
                        c"Hyphenation exceptions must contain only letters".as_ptr(),
                    );
                    set_help_line(
                        0,
                        c"and hyphens. But continue; I'll forgive and forget.".as_ptr(),
                    );
                    crate::errors::error();
                    break;
                }
            }
        }

        let _ = &p; // suppress unused-variable warning on 'outer
    }
}
