//! Rust translation of initialization helpers from xetex-ini.c.

use crate::globals::{
    cur_lang, first, hyf_distance_entry, hyf_next_entry, hyf_num_entry, last, max_op_used,
    pool_size, set_buffer_entry, set_cur_input_limit, set_cur_input_loc, set_first,
    set_hyf_distance_entry, set_hyf_next_entry, set_hyf_num_entry, set_input_file, set_last,
    set_max_op_used, set_pool_ptr, set_str_ptr, set_str_start, set_trie_op_lang_entry,
    set_trie_op_ptr, set_trie_op_val_entry, set_trie_used_entry, string_vacancies,
    trie_op_lang_entry, trie_op_ptr, trie_op_val_entry, trie_used_entry,
};
use crate::io::UFile;

const INVALID_HANDLE: usize = 0;
const UTF8: i16 = 1;
const TOO_BIG_CHAR: i32 = 65_536;
const TRIE_OP_SIZE: i32 = 35_111;
const NEG_TRIE_OP_SIZE: i32 = -35_111;
const MAX_TRIE_OP: u16 = 65_535;
const MIN_TRIE_OP: i32 = 0;
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
