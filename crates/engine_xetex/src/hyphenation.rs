//! Hyphenation pattern / exception loading and the packed-trie builder,
//! translated from the hyphenation part of xetex-ini.c.

// `TRIE_OP_SIZE`, `MAX_TRIE_OP`, and `BIGGEST_LANG` are `pub(crate)` because the
// format-dump code in `ini.rs` (later `format_io.rs`) still references them.
pub(crate) const TRIE_OP_SIZE: i32 = 35_111;
const NEG_TRIE_OP_SIZE: i32 = -35_111;
pub(crate) const MAX_TRIE_OP: u16 = 65_535;
const MIN_TRIE_OP: i32 = 0;
pub(crate) const BIGGEST_LANG: i32 = 255;
const TRIE_OP_HASH_SIZE: usize = (TRIE_OP_SIZE - NEG_TRIE_OP_SIZE + 1) as usize;

#[no_mangle]
pub static mut _trie_op_hash_array: [i32; TRIE_OP_HASH_SIZE] = [0; TRIE_OP_HASH_SIZE];

use crate::consts::{BIGGEST_CHAR, INT_BASE, LC_CODE_BASE, TOO_BIG_CHAR};
use crate::globals::{
    cur_chr, cur_cmd, cur_lang, cur_val, eqtb_b32s1, hc_entry, hyf_distance_entry, hyf_entry,
    hyf_next_entry, hyf_num_entry, hyph_count, hyph_index, hyph_link_entry, hyph_next, hyph_size,
    hyph_start, hyph_word_entry, init_pool_ptr, max_hyph_char, max_op_used, op_start_entry,
    pool_ptr, pool_size, set_cur_chr, set_cur_cmd, set_cur_lang, set_hc_entry, set_help_line,
    set_help_ptr, set_hyf_distance_entry, set_hyf_entry, set_hyf_next_entry, set_hyf_num_entry,
    set_hyph_count, set_hyph_index, set_hyph_link_entry, set_hyph_list_entry, set_hyph_next,
    set_hyph_start, set_hyph_word_entry, set_max_hyph_char, set_max_op_used, set_op_start_entry,
    set_pool_ptr, set_str_pool, set_str_ptr, set_trie_c_entry, set_trie_hash_entry,
    set_trie_l_entry, set_trie_max, set_trie_min_entry, set_trie_not_ready, set_trie_o_entry,
    set_trie_op_lang_entry, set_trie_op_ptr, set_trie_op_val_entry, set_trie_ptr, set_trie_r_entry,
    set_trie_taken_entry, set_trie_trc_entry, set_trie_trl_entry, set_trie_tro_entry,
    set_trie_used_entry, str_pool_entry, str_ptr, str_start, trie_c_entry, trie_hash_entry,
    trie_l_entry, trie_max, trie_min_entry, trie_not_ready, trie_o_entry, trie_op_lang_entry,
    trie_op_ptr, trie_op_val_entry, trie_ptr, trie_r_entry, trie_size, trie_taken_entry,
    trie_trc_entry, trie_trl_entry, trie_tro_entry, trie_used_entry,
};

// ---- moved verbatim from ini.rs (workstream 02) ----

#[inline]
unsafe fn trie_op_hash(k: i32) -> i32 {
    _trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize]
}
#[inline]
pub(crate) unsafe fn set_trie_op_hash(k: i32, v: i32) {
    _trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize] = v;
}
// translated from xetex-ini.c:630 (new_trie_op)
pub unsafe fn new_trie_op(d: i16, n: i16, v: u16) -> u16 {
    let mut h = (n as i32 + 313 * d as i32 + 361 * v as i32 + 1009 * cur_lang() as i32).abs()
        % (TRIE_OP_SIZE - NEG_TRIE_OP_SIZE)
        + NEG_TRIE_OP_SIZE;

    loop {
        let l = trie_op_hash(h);
        if l == 0 {
            if trie_op_ptr() == TRIE_OP_SIZE {
                crate::errors::overflow(c"pattern memory ops".as_ptr(), TRIE_OP_SIZE);
            }

            let lang = cur_lang() as usize;
            let mut u = trie_used_entry(lang);
            if u == MAX_TRIE_OP {
                crate::errors::overflow(
                    c"pattern memory ops per language".as_ptr(),
                    MAX_TRIE_OP as i32 - MIN_TRIE_OP,
                );
            }

            set_trie_op_ptr(trie_op_ptr() + 1);
            u += 1;
            set_trie_used_entry(lang, u);
            if u > max_op_used() {
                set_max_op_used(u);
            }

            set_hyf_distance_entry(trie_op_ptr(), d);
            set_hyf_num_entry(trie_op_ptr(), n);
            set_hyf_next_entry(trie_op_ptr(), v);
            set_trie_op_lang_entry(trie_op_ptr(), cur_lang());
            set_trie_op_hash(h, trie_op_ptr());
            set_trie_op_val_entry(trie_op_ptr(), u);
            return u;
        }

        if hyf_distance_entry(l) == d
            && hyf_num_entry(l) == n
            && hyf_next_entry(l) == v
            && trie_op_lang_entry(l) == cur_lang()
        {
            return trie_op_val_entry(l);
        }

        if h > -TRIE_OP_SIZE {
            h -= 1;
        } else {
            h = TRIE_OP_SIZE;
        }
    }
}
// translated from xetex-ini.c:632 (trie_node)
pub fn trie_node(p: i32) -> i32 {
    let mut h = (trie_c_entry(p) as i32)
        .wrapping_add((1009_i32).wrapping_mul(trie_o_entry(p) as i32))
        .wrapping_add((2718_i32).wrapping_mul(trie_l_entry(p)))
        .wrapping_add((3142_i32).wrapping_mul(trie_r_entry(p)))
        .wrapping_abs()
        % trie_size();

    loop {
        let q = trie_hash_entry(h);
        if q == 0 {
            set_trie_hash_entry(h, p);
            return p;
        }

        if trie_c_entry(q) == trie_c_entry(p)
            && trie_o_entry(q) == trie_o_entry(p)
            && trie_l_entry(q) == trie_l_entry(p)
            && trie_r_entry(q) == trie_r_entry(p)
        {
            return q;
        }

        if h > 0 {
            h -= 1;
        } else {
            h = trie_size();
        }
    }
}
// translated from xetex-ini.c:655 (compress_trie)
pub fn compress_trie(p: i32) -> i32 {
    if p == 0 {
        return 0;
    }

    set_trie_l_entry(p, compress_trie(trie_l_entry(p)));
    set_trie_r_entry(p, compress_trie(trie_r_entry(p)));
    trie_node(p)
}
// translated from xetex-ini.c:638 (first_fit)
pub unsafe fn first_fit(p: i32) {
    let c = trie_c_entry(p) as i32;
    let mut z = trie_min_entry(c);
    let mut h;

    loop {
        h = z - c;
        if trie_max() < h + max_hyph_char() {
            if trie_size() <= h + max_hyph_char() {
                crate::errors::overflow(c"pattern memory".as_ptr(), trie_size());
            }

            loop {
                set_trie_max(trie_max() + 1);
                set_trie_taken_entry(trie_max(), false);
                set_trie_trl_entry(trie_max(), trie_max() + 1);
                set_trie_tro_entry(trie_max(), trie_max() - 1);
                if trie_max() == h + max_hyph_char() {
                    break;
                }
            }
        }

        let mut found = !trie_taken_entry(h);
        let mut q = trie_r_entry(p);
        while found && q > 0 {
            if trie_trl_entry(h + trie_c_entry(q) as i32) == 0 {
                found = false;
            } else {
                q = trie_r_entry(q);
            }
        }

        if found {
            break;
        }

        z = trie_trl_entry(z);
    }

    set_trie_taken_entry(h, true);
    set_trie_hash_entry(p, h);
    let mut q = p;
    loop {
        z = h + trie_c_entry(q) as i32;
        let mut l = trie_tro_entry(z);
        let r = trie_trl_entry(z);
        set_trie_tro_entry(r, l);
        set_trie_trl_entry(l, r);
        set_trie_trl_entry(z, 0);

        if l < max_hyph_char() {
            let ll = if z < max_hyph_char() {
                z
            } else {
                max_hyph_char()
            };

            loop {
                set_trie_min_entry(l, r);
                l += 1;
                if l == ll {
                    break;
                }
            }
        }

        q = trie_r_entry(q);
        if q == 0 {
            break;
        }
    }
}
// translated from xetex-ini.c:695 (trie_pack)
pub unsafe fn trie_pack(mut p: i32) {
    loop {
        let q = trie_l_entry(p);
        if q > 0 && trie_hash_entry(q) == 0 {
            first_fit(q);
            trie_pack(q);
        }

        p = trie_r_entry(p);
        if p == 0 {
            break;
        }
    }
}
// translated from xetex-ini.c:708 (trie_fix)
pub fn trie_fix(mut p: i32) {
    let z = trie_hash_entry(p);
    loop {
        let q = trie_l_entry(p);
        let c = trie_c_entry(p);
        set_trie_trl_entry(z + c as i32, trie_hash_entry(q));
        set_trie_trc_entry(z + c as i32, c);
        set_trie_tro_entry(z + c as i32, trie_o_entry(p) as i32);

        if q > 0 {
            trie_fix(q);
        }

        p = trie_r_entry(p);
        if p == 0 {
            break;
        }
    }
}
// translated from xetex-ini.c:858 (init_trie)
pub unsafe fn init_trie() {
    set_max_hyph_char(max_hyph_char() + 1);
    set_op_start_entry(0, -MIN_TRIE_OP);

    let mut j = 1;
    while j <= BIGGEST_LANG {
        let prev = (j - 1) as usize;
        let val = op_start_entry(prev) + trie_used_entry(prev) as i32;
        set_op_start_entry(j as usize, val);
        j += 1;
    }

    let op_ptr = trie_op_ptr();
    j = 1;
    while j <= op_ptr {
        let lang = trie_op_lang_entry(j) as usize;
        let val = op_start_entry(lang) + trie_op_val_entry(j) as i32;
        set_trie_op_hash(j, val);
        j += 1;
    }

    j = 1;
    while j <= op_ptr {
        while trie_op_hash(j) > j {
            let k = trie_op_hash(j);
            let t = hyf_distance_entry(k);
            set_hyf_distance_entry(k, hyf_distance_entry(j));
            set_hyf_distance_entry(j, t);

            let t = hyf_num_entry(k);
            set_hyf_num_entry(k, hyf_num_entry(j));
            set_hyf_num_entry(j, t);

            let t = hyf_next_entry(k);
            set_hyf_next_entry(k, hyf_next_entry(j));
            set_hyf_next_entry(j, t);

            let hash_k = trie_op_hash(k);
            set_trie_op_hash(j, hash_k);
            set_trie_op_hash(k, k);
        }
        j += 1;
    }

    let mut p = 0;
    let trie_size_val = trie_size();
    while p <= trie_size_val {
        set_trie_hash_entry(p, 0);
        p += 1;
    }

    set_trie_r_entry(0, compress_trie(trie_r_entry(0)));
    set_trie_l_entry(0, compress_trie(trie_l_entry(0)));

    p = 0;
    let trie_ptr_val = trie_ptr();
    while p <= trie_ptr_val {
        set_trie_hash_entry(p, 0);
        p += 1;
    }

    p = 0;
    while p <= BIGGEST_CHAR {
        set_trie_min_entry(p, p + 1);
        p += 1;
    }

    set_trie_trl_entry(0, 1);
    set_trie_max(0);

    let trie_l0 = trie_l_entry(0);
    if trie_l0 != 0 {
        first_fit(trie_l0);
        trie_pack(trie_l0);
    }

    let trie_r0 = trie_r_entry(0);
    if trie_r0 != 0 {
        if trie_l0 == 0 {
            p = 0;
            while p <= 255 {
                set_trie_min_entry(p, p + 2);
                p += 1;
            }
        }
        first_fit(trie_r0);
        trie_pack(trie_r0);
        set_hyph_start(trie_hash_entry(trie_r0));
    }

    if trie_max() == 0 {
        let mut r = 0;
        let max_hyph = max_hyph_char();
        while r <= max_hyph {
            set_trie_trl_entry(r, 0);
            set_trie_tro_entry(r, MIN_TRIE_OP);
            set_trie_trc_entry(r, 0);
            r += 1;
        }
        set_trie_max(max_hyph);
    } else {
        if trie_r0 > 0 {
            trie_fix(trie_r0);
        }
        if trie_l0 > 0 {
            trie_fix(trie_l0);
        }

        let trie_max_val = trie_max();
        let mut r = 0;
        loop {
            let s = trie_trl_entry(r);
            set_trie_trl_entry(r, 0);
            set_trie_tro_entry(r, MIN_TRIE_OP);
            set_trie_trc_entry(r, 0);
            r = s;
            if r > trie_max_val {
                break;
            }
        }
    }

    set_trie_trc_entry(0, b'?' as u16);
    set_trie_not_ready(false);
}
// translated from xetex-ini.c:643 (new_patterns)
pub unsafe fn new_patterns() {
    const INT_PAR_LANGUAGE: i32 = 50;
    const INT_PAR_SAVING_HYPH_CODES: i32 = 64;
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const SPACER: u8 = 10;
    const RIGHT_BRACE_CMD: u8 = 2;

    // Inline helper: insert a new trie node at position trie_ptr+1 and link it.
    // Returns new p (= trie_ptr after increment).
    macro_rules! trie_insert_node {
        ($q:expr, $p_ref:expr, $c:expr, $first_child:expr) => {{
            if trie_ptr() == trie_size() {
                crate::errors::overflow(c"pattern memory".as_ptr(), trie_size());
            }
            set_trie_ptr(trie_ptr() + 1);
            set_trie_r_entry(trie_ptr(), $p_ref);
            $p_ref = trie_ptr();
            set_trie_l_entry($p_ref, 0);
            if $first_child {
                set_trie_l_entry($q, $p_ref);
            } else {
                set_trie_r_entry($q, $p_ref);
            }
            set_trie_c_entry($p_ref, $c);
            set_trie_o_entry($p_ref, MIN_TRIE_OP as u16);
        }};
    }

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
                            if hc_entry(1) == 0 {
                                set_hyf_entry(0, 0);
                            }
                            if hc_entry(k as usize) == 0 {
                                set_hyf_entry(k as usize, 0);
                            }

                            let mut l = k;
                            let mut v: u16 = MIN_TRIE_OP as u16;
                            loop {
                                if hyf_entry(l as usize) != 0 {
                                    v = new_trie_op(k - l, hyf_entry(l as usize) as i16, v);
                                }
                                if l > 0 {
                                    l -= 1;
                                } else {
                                    break;
                                }
                            }

                            // l is now 0; walk hc[0..=k] into trie
                            let mut q: i32 = 0;
                            set_hc_entry(0, cur_lang() as i32);
                            while l <= k {
                                let c = hc_entry(l as usize) as u16;
                                l += 1;
                                let mut p = trie_l_entry(q);
                                let mut first_child = true;

                                while p > 0 && c > trie_c_entry(p) {
                                    q = p;
                                    p = trie_r_entry(q);
                                    first_child = false;
                                }

                                if p == 0 || c < trie_c_entry(p) {
                                    trie_insert_node!(q, p, c, first_child);
                                }

                                q = p;
                            }

                            if trie_o_entry(q) != MIN_TRIE_OP as u16 {
                                crate::output::error_here_with_diagnostic(
                                    c"Duplicate pattern".as_ptr(),
                                );
                                crate::output::capture_to_diagnostic(std::ptr::null_mut());
                                set_help_ptr(1);
                                set_help_line(0, c"(See Appendix H.)".as_ptr());
                                crate::errors::error();
                            }

                            set_trie_o_entry(q, v);
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
            let c = cur_lang() as u16;
            let mut first_child = false;
            let mut p: i32 = 0;

            let mut q: i32;
            loop {
                q = p;
                p = trie_r_entry(q);
                if p == 0 || c <= trie_c_entry(p) {
                    break;
                }
            }

            if p == 0 || c < trie_c_entry(p) {
                trie_insert_node!(q, p, c, first_child);
            }

            q = p;
            p = trie_l_entry(q);
            first_child = true;

            for ci in 0u16..=255u16 {
                let lc = eqtb_b32s1(LC_CODE_BASE + ci as i32);
                if lc > 0 || (ci == 255 && first_child) {
                    if p == 0 {
                        trie_insert_node!(q, p, ci, first_child);
                    } else {
                        set_trie_c_entry(p, ci);
                    }
                    set_trie_o_entry(p, lc as u16);
                    q = p;
                    p = trie_r_entry(q);
                    first_child = false;
                }
            }

            if first_child {
                set_trie_l_entry(q, 0);
            } else {
                set_trie_r_entry(q, 0);
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
