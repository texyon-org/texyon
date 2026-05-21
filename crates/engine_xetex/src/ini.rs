//! Rust translation of initialization helpers from xetex-ini.c.

use crate::globals::{
    cur_lang, first, hyf_distance_entry, hyf_next_entry, hyf_num_entry, last, max_hyph_char,
    max_op_used, op_start_entry, pool_size, set_buffer_entry, set_cur_input_limit,
    set_cur_input_loc, set_first, set_hyf_distance_entry, set_hyf_next_entry, set_hyf_num_entry,
    set_hyph_start, set_input_file, set_last, set_max_hyph_char, set_max_op_used,
    set_op_start_entry, set_pool_ptr, set_str_ptr, set_str_start, set_trie_hash_entry,
    set_trie_l_entry, set_trie_max, set_trie_min_entry, set_trie_not_ready,
    set_trie_op_lang_entry, set_trie_op_ptr, set_trie_op_val_entry, set_trie_r_entry,
    set_trie_taken_entry, set_trie_trc_entry, set_trie_trl_entry, set_trie_tro_entry,
    set_trie_used_entry, string_vacancies, trie_c_entry, trie_hash_entry, trie_l_entry, trie_max,
    trie_min_entry, trie_o_entry, trie_op_lang_entry, trie_op_ptr, trie_op_val_entry, trie_ptr,
    trie_r_entry, trie_size, trie_taken_entry, trie_trl_entry, trie_tro_entry, trie_used_entry,
};
use crate::io::UFile;

const INVALID_HANDLE: usize = 0;
const UTF8: i16 = 1;
const TOO_BIG_CHAR: i32 = 65_536;
const TRIE_OP_SIZE: i32 = 35_111;
const NEG_TRIE_OP_SIZE: i32 = -35_111;
const MAX_TRIE_OP: u16 = 65_535;
const MIN_TRIE_OP: i32 = 0;
const BIGGEST_LANG: i32 = 255;
const BIGGEST_CHAR: i32 = 0xFFFF;
const TRIE_OP_HASH_SIZE: usize = (TRIE_OP_SIZE - NEG_TRIE_OP_SIZE + 1) as usize;

#[no_mangle]
pub static mut _trie_op_hash_array: [i32; TRIE_OP_HASH_SIZE] = [0; TRIE_OP_HASH_SIZE];

extern "C" {
    fn _tt_abort(fmt: *const libc::c_char) -> !;
}

#[inline]
unsafe fn trie_op_hash(k: i32) -> i32 {
    _trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize]
}

#[inline]
unsafe fn set_trie_op_hash(k: i32, v: i32) {
    _trie_op_hash_array[(k - NEG_TRIE_OP_SIZE) as usize] = v;
}

static mut STDIN_UFILE: UFile = UFile {
    handle: INVALID_HANDLE,
    saved_char: -1,
    skip_next_lf: 0,
    encoding_mode: UTF8,
    conversion_data: std::ptr::null_mut(),
};

// translated from xetex-ini.c:3035 (init_io)
#[no_mangle]
pub unsafe extern "C" fn init_io() {
    // This is largely vestigial at this point.
    STDIN_UFILE.handle = INVALID_HANDLE;
    STDIN_UFILE.saved_char = -1;
    STDIN_UFILE.skip_next_lf = 0;
    STDIN_UFILE.encoding_mode = UTF8;
    STDIN_UFILE.conversion_data = std::ptr::null_mut();
    set_input_file(0, (&raw mut STDIN_UFILE).cast());

    set_buffer_entry(first(), 0);
    set_last(first());
    set_cur_input_loc(first());
    set_cur_input_limit(last());
    set_first(last() + 1);
}

// translated from xetex-ini.c:3452 (get_strings_started)
#[no_mangle]
pub unsafe extern "C" fn get_strings_started() {
    set_pool_ptr(0);
    set_str_ptr(0);
    set_str_start(0, 0);
    set_str_ptr(TOO_BIG_CHAR);

    if crate::string_pool::load_pool_strings(pool_size() - string_vacancies()) == 0 {
        _tt_abort(c"must increase pool_size".as_ptr());
    }
}

// translated from xetex-ini.c:630 (new_trie_op)
#[no_mangle]
pub unsafe extern "C" fn new_trie_op(d: i16, n: i16, v: u16) -> u16 {
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
#[no_mangle]
pub unsafe extern "C" fn trie_node(p: i32) -> i32 {
    let mut h = (trie_c_entry(p) as i32
        + 1009 * trie_o_entry(p) as i32
        + 2718 * trie_l_entry(p)
        + 3142 * trie_r_entry(p))
    .abs()
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
#[no_mangle]
pub unsafe extern "C" fn compress_trie(p: i32) -> i32 {
    if p == 0 {
        return 0;
    }

    set_trie_l_entry(p, compress_trie(trie_l_entry(p)));
    set_trie_r_entry(p, compress_trie(trie_r_entry(p)));
    trie_node(p)
}

// translated from xetex-ini.c:638 (first_fit)
#[no_mangle]
pub unsafe extern "C" fn first_fit(p: i32) {
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
#[no_mangle]
pub unsafe extern "C" fn trie_pack(mut p: i32) {
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
#[no_mangle]
pub unsafe extern "C" fn trie_fix(mut p: i32) {
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
#[no_mangle]
pub unsafe extern "C" fn init_trie() {
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
