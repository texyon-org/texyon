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
use crate::globals::{cur_val};
use crate::state::HyphState;

// ---- moved verbatim from ini.rs (workstream 02) ----

#[inline]
fn trie_op_hash_in(hyph: &HyphState, k: i32) -> i32 {
    hyph.trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize]
}

#[inline]
fn set_trie_op_hash_in(hyph: &mut HyphState, k: i32, v: i32) {
    hyph.trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize] = v;
}

pub(crate) enum TrieAbort {
    PatternMemoryOverflow(i32),
    PatternMemoryOpsOverflow(i32),
    PatternMemoryOpsPerLanguageOverflow(i32),
}

impl TrieAbort {
    /// Map to the unified [`EngineAbort`] terminal (`01d` §5.5), mirroring
    /// [`trie_abort`] without longjmp-ing.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            TrieAbort::PatternMemoryOverflow(size) => crate::errors::EngineAbort::Overflow {
                what: c"pattern memory".as_ptr(),
                n: size,
            },
            TrieAbort::PatternMemoryOpsOverflow(size) => crate::errors::EngineAbort::Overflow {
                what: c"pattern memory ops".as_ptr(),
                n: size,
            },
            TrieAbort::PatternMemoryOpsPerLanguageOverflow(size) => {
                crate::errors::EngineAbort::Overflow {
                    what: c"pattern memory ops per language".as_ptr(),
                    n: size,
                }
            }
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

pub(crate) fn init_trie_in(hyph: &mut HyphState) -> Result<(), TrieAbort> {
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
pub(crate) fn new_patterns_in(
    st: &mut crate::state::EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    use crate::main_control::mc_propagate_error;
    const INT_PAR_LANGUAGE: i32 = 50;
    const INT_PAR_SAVING_HYPH_CODES: i32 = 64;
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const SPACER: u8 = 10;
    const RIGHT_BRACE_CMD: u8 = 2;

    if st.hyph.trie_not_ready {
        let lang = unsafe { st.eqtb.data[(INT_BASE + INT_PAR_LANGUAGE) as usize].b32.s1 };
        if lang <= 0 || lang > BIGGEST_LANG {
            st.hyph.cur_lang = 0;
        } else {
            st.hyph.cur_lang = lang as u8;
        }

        crate::scan::scan_left_brace_in(st)?;
        let mut k: i16 = 0;
        st.hyph.hyf[0] = 0;
        let mut digit_sensed = false;

        'done: {
            loop {
                crate::scan::get_x_token_in(st)?;
                match st.scanner.cur_cmd {
                    LETTER | OTHER_CHAR => {
                        if digit_sensed || st.scanner.cur_chr < b'0' as i32 || st.scanner.cur_chr > b'9' as i32 {
                            if st.scanner.cur_chr == b'.' as i32 {
                                st.scanner.cur_chr = 0;
                            } else {
                                st.scanner.cur_chr = unsafe {
                                    st.eqtb.data[(LC_CODE_BASE + st.scanner.cur_chr) as usize].b32.s1
                                };
                                if st.scanner.cur_chr == 0 {
                                    crate::output::error_here_with_diagnostic_in(
                                        st,
                                        c"Nonletter".as_ptr(),
                                    );
                                    crate::output::capture_to_diagnostic_in(
                                        st,
                                        std::ptr::null_mut(),
                                    );
                                    st.errors.help_ptr = 1;
                                    st.errors.help_line[0] = c"(See Appendix H.)".as_ptr();
                                    mc_propagate_error(crate::errors::error_in(st))?;
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
                            st.hyph.hyf[k as usize] = (st.scanner.cur_chr - b'0' as i32) as u8;
                            digit_sensed = true;
                        }
                    }
                    SPACER | RIGHT_BRACE_CMD => {
                        if k > 0 {
                            let insert = insert_pattern_word_in(&mut st.hyph, k)?;

                            if insert.duplicate {
                                crate::output::error_here_with_diagnostic_in(
                                    st,
                                    c"Duplicate pattern".as_ptr(),
                                );
                                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                                st.errors.help_ptr = 1;
                                st.errors.help_line[0] = c"(See Appendix H.)".as_ptr();
                                mc_propagate_error(crate::errors::error_in(st))?;
                            }

                            set_pattern_word_op_in(&mut st.hyph, insert);
                        }

                        if st.scanner.cur_cmd == RIGHT_BRACE_CMD {
                            break 'done;
                        }

                        k = 0;
                        st.hyph.hyf[0] = 0;
                        digit_sensed = false;
                    }
                    _ => {
                        crate::output::error_here_with_diagnostic_in(st, c"Bad ".as_ptr());
                        crate::output::print_esc_cstr_in(st, c"patterns".as_ptr());
                        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                        st.errors.help_ptr = 1;
                        st.errors.help_line[0] = c"(See Appendix H.)".as_ptr();
                        mc_propagate_error(crate::errors::error_in(st))?;
                    }
                }
            }
        } // 'done

        // saving_hyph_codes: record lc_code table alongside the patterns
        if unsafe {
            st.eqtb.data[(INT_BASE + INT_PAR_SAVING_HYPH_CODES) as usize]
                .b32
                .s1
        } > 0
        {
            let mut lc_codes = [0; 256];
            for ci in 0u16..=255u16 {
                lc_codes[ci as usize] =
                    unsafe { st.eqtb.data[(LC_CODE_BASE + ci as i32) as usize].b32.s1 };
            }
            save_hyph_codes_in(&mut st.hyph, &lc_codes)?;
        }
    } else {
        crate::output::error_here_with_diagnostic_in(st, c"Too late for ".as_ptr());
        crate::output::print_esc_cstr_in(st, c"patterns".as_ptr());
        crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
        st.errors.help_ptr = 1;
        st.errors.help_line[0] = c"All patterns must be given before typesetting begins.".as_ptr();
        mc_propagate_error(crate::errors::error_in(st))?;

        let _ = crate::scan::scan_toks_in(st, false, false)?;
        crate::mem::flush_list(&mut st.mem, st.scanner.def_ref);
    }
    Ok(())
}

// translated from xetex-ini.c:863 (new_hyph_exceptions)
pub(crate) fn new_hyph_exceptions_in(
    st: &mut crate::state::EngineState,
) -> Result<(), crate::main_control::MainControlAbort> {
    use crate::main_control::{mc_propagate_error, MainControlAbort};
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const CHAR_GIVEN: u8 = 68;
    const CHAR_NUM: u8 = 16;
    const SPACER: u8 = 10;
    const RIGHT_BRACE_CMD: u8 = 2;
    const INT_PAR_LANGUAGE: i32 = 50;
    const HYPH_PRIME: i32 = 607;

    crate::scan::scan_left_brace_in(st)?;

    let lang = unsafe { st.eqtb.data[(INT_BASE + INT_PAR_LANGUAGE) as usize].b32.s1 };
    if lang <= 0 || lang > BIGGEST_LANG {
        st.hyph.cur_lang = 0;
    } else {
        st.hyph.cur_lang = lang as u8;
    }

    if st.hyph.trie_not_ready {
        st.hyph.hyph_index = 0;
    } else if st.hyph.trie_trc[(st.hyph.hyph_start + st.hyph.cur_lang as i32) as usize]
        != st.hyph.cur_lang as u16
    {
        st.hyph.hyph_index = 0;
    } else {
        st.hyph.hyph_index =
            st.hyph.trie_trl[(st.hyph.hyph_start + st.hyph.cur_lang as i32) as usize];
    }

    // not_found1:
    let mut n: i16 = 0;
    let mut p: i32 = crate::mem::TEX_NULL;

    loop {
        crate::scan::get_x_token_in(st)?;

        // reswitch: re-evaluate after CHAR_NUM changes cur_cmd
        loop {
            match st.scanner.cur_cmd {
                LETTER | OTHER_CHAR | CHAR_GIVEN => {
                    if st.scanner.cur_chr == b'-' as i32 {
                        if n < crate::scan::max_hyphenatable_length_in(&st.eqtb) as i16 {
                            let q = crate::scan::get_avail_scc(st)?;
                            st.mem.set_b32s1(q, p);
                            st.mem.set_b32s0(q, n as i32);
                            p = q;
                        }
                    } else {
                        let hc0 = if st.hyph.hyph_index == 0 || st.scanner.cur_chr > 255 {
                            unsafe { st.eqtb.data[(LC_CODE_BASE + st.scanner.cur_chr) as usize].b32.s1 }
                        } else if st.hyph.trie_trc[(st.hyph.hyph_index + st.scanner.cur_chr) as usize]
                            != st.scanner.cur_chr as u16
                        {
                            0
                        } else {
                            st.hyph.trie_tro[(st.hyph.hyph_index + st.scanner.cur_chr) as usize]
                        };
                        st.hyph.hc[0] = hc0;

                        if st.hyph.hc[0] == 0 {
                            crate::output::error_here_with_diagnostic_in(
                                st,
                                c"Not a letter".as_ptr(),
                            );
                            crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                            st.errors.help_ptr = 2;
                            st.errors.help_line[1] =
                                c"Letters in \\hyphenation words must have \\lccode>0.".as_ptr();
                            st.errors.help_line[0] =
                                c"Proceed; I'll ignore the character I just read.".as_ptr();
                            mc_propagate_error(crate::errors::error_in(st))?;
                        } else if n < crate::scan::max_hyphenatable_length_in(&st.eqtb) as i16 {
                            n += 1;
                            if st.hyph.hc[0] < 65536 {
                                st.hyph.hc[n as usize] = st.hyph.hc[0];
                            } else {
                                st.hyph.hc[n as usize] = (st.hyph.hc[0] - 65536) / 1024 + 55296;
                                n += 1;
                                st.hyph.hc[n as usize] = st.hyph.hc[0] % 1024 + 56320;
                            }
                        }
                    }
                    break;
                }
                CHAR_NUM => {
                    crate::scan::scan_char_num_in(st)?;
                    st.scanner.cur_chr = cur_val();
                    st.scanner.cur_cmd = CHAR_GIVEN;
                    // continue inner loop → re-run match with new cur_cmd
                }
                SPACER | RIGHT_BRACE_CMD => {
                    if n > 1 {
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
                        for j in 1i16..=n {
                            h = (h + h + st.hyph.hc[j as usize]) % HYPH_PRIME;
                            st.strings.str_pool[st.strings.pool_ptr as usize] =
                                st.hyph.hc[j as usize] as u16;
                            st.strings.pool_ptr += 1;
                        }

                        let mut s = crate::string_pool::make_string(&mut st.strings)?;

                        if st.hyph.hyph_next <= HYPH_PRIME {
                            while st.hyph.hyph_next > 0
                                && st.hyph.hyph_word[(st.hyph.hyph_next - 1) as usize] > 0
                            {
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

                        // Hash lookup / insert into exception table
                        loop {
                            if st.hyph.hyph_word[h as usize] == 0 {
                                break;
                            }
                            let k = st.hyph.hyph_word[h as usize];

                            // Check if this slot holds the same word
                            let matched = if crate::string_pool::length(&st.strings, k)
                                != crate::string_pool::length(&st.strings, s)
                            {
                                false
                            } else {
                                let mut u = st.strings.str_start[(k - TOO_BIG_CHAR) as usize];
                                let mut v = st.strings.str_start[(s - TOO_BIG_CHAR) as usize];
                                let end = st.strings.str_start[(k + 1 - TOO_BIG_CHAR) as usize];
                                let mut ok = true;
                                while u != end {
                                    if st.strings.str_pool[u as usize]
                                        != st.strings.str_pool[v as usize]
                                    {
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
                                st.strings.str_ptr -= 1;
                                st.strings.pool_ptr = st.strings.str_start
                                    [(st.strings.str_ptr - TOO_BIG_CHAR) as usize];
                                s = st.hyph.hyph_word[h as usize];
                                st.hyph.hyph_count -= 1;
                                break;
                            }

                            // not_found: collision resolution
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
                        st.hyph.hyph_list[h as usize] = p;
                    }

                    if st.scanner.cur_cmd == RIGHT_BRACE_CMD {
                        return Ok(());
                    }

                    n = 0;
                    p = crate::mem::TEX_NULL;
                    break;
                }
                _ => {
                    crate::output::error_here_with_diagnostic_in(st, c"Improper ".as_ptr());
                    crate::output::print_esc_cstr_in(st, c"hyphenation".as_ptr());
                    crate::output::print_cstr_in(st, c" will be flushed".as_ptr());
                    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                    st.errors.help_ptr = 2;
                    st.errors.help_line[1] =
                        c"Hyphenation exceptions must contain only letters".as_ptr();
                    st.errors.help_line[0] =
                        c"and hyphens. But continue; I'll forgive and forget.".as_ptr();
                    mc_propagate_error(crate::errors::error_in(st))?;
                    break;
                }
            }
        }

        let _ = &p; // suppress unused-variable warning on 'outer
    }
}
