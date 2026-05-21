//! Rust translation of initialization helpers from xetex-ini.c.

use crate::globals::{
    buf_size, cond_ptr, cur_chr, cur_cmd, cur_if, cur_input, cur_lang, cur_level, cur_mark,
    cur_val, disc_ptr, eqtb_b32s1, eqtb_top, eqtb_word, first, font_area_entry,
    font_layout_engine_ptr, font_mapping_ptr, font_max, hc_entry, history,
    hyf_distance_entry, hyf_entry, hyf_next_entry, hyf_num_entry, hyph_count, hyph_index,
    hyph_link_entry, hyph_next, hyph_size, hyph_start, hyph_word_entry, if_line, in_open,
    init_pool_ptr, in_initex_mode, input_file, interaction, job_name, last, last_glue,
    max_hyph_char,
    max_op_used, mem_b16s0, mem_b32s1, op_start_entry, open_parens, pool_ptr, pool_size,
    rover, sa_root, selector, set_adjust_tail, set_after_token, set_align_ptr, set_avail,
    set_buffer_entry, set_cancel_boundary, set_cond_ptr, set_cs_count, set_cur_align,
    set_cur_boundary, set_cur_chr, set_cur_cmd, set_cur_dir, set_cur_f, set_cur_group,
    set_cur_head, set_cur_if, set_cur_input_limit, set_cur_input_loc, set_cur_lang,
    set_cur_level, set_cur_list_aux_b32s1, set_cur_list_e_tex_aux, set_cur_list_head,
    set_cur_list_mode, set_cur_list_mode_line, set_cur_list_prev_graf, set_cur_list_tail,
    set_cur_loop, set_cur_mark, set_cur_order, set_cur_pre_head, set_cur_pre_tail,
    set_cur_span, set_cur_tail_a, set_cur_val, set_cur_val_level, set_dead_cycles,
    set_deletions_allowed, set_disc_ptr, set_doing_leaders, set_doing_special,
    set_dyn_used, set_edit_name_start, set_empty_b32s0, set_empty_b32s1, set_eqtb_b16s0,
    set_eqtb_b16s1, set_eqtb_b32s1, set_eqtb_word, set_error_count,
    set_expand_depth_count, set_first, set_format_ident, set_hash_high, set_hash_s1,
    set_hash_used, set_hc_entry, set_help_line, set_help_ptr, set_hi_mem_min,
    set_hyf_distance_entry, set_hyf_entry, set_hyf_next_entry, set_hyf_num_entry,
    set_hyph_count, set_hyph_index, set_hyph_link_entry, set_hyph_list_entry,
    set_hyph_next, set_hyph_start, set_hyph_word_entry, set_if_limit, set_if_line,
    set_input_file, set_ins_disc, set_insert_penalties, set_interaction, set_is_in_csname,
    set_last, set_last_badness, set_last_bop, set_last_glue, set_last_kern,
    set_last_node_type, set_last_penalty, set_lft_hit, set_ligature_present,
    set_lo_mem_max, set_long_help_seen, set_lr_problems, set_lr_ptr, set_mag_set,
    set_max_h, set_max_hyph_char, set_max_nest_stack, set_max_op_used, set_max_push,
    set_max_reg_help_line, set_max_reg_num, set_max_save_stack, set_max_v,
    set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1, set_mem_end,
    set_mem_word, set_native_text, set_native_text_size, set_nest_ptr,
    set_no_new_control_sequence, set_null_character, set_op_start_entry, set_open_parens,
    set_output_active, set_pack_begin_line, set_page_contents, set_page_so_far,
    set_page_tail, set_pool_ptr, set_pre_adjust_tail, set_prim_s0, set_prim_s1,
    set_prim_used, set_pseudo_files, set_radix, set_read_open_entry, set_rover,
    set_rt_hit, set_sa_chain, set_sa_level, set_sa_null_b32s0, set_sa_null_b32s1,
    set_sa_root, set_save_ptr, set_selector, set_set_box_allowed, set_shown_mode,
    set_spec_log, set_stop_at_space, set_str_pool, set_str_ptr, set_str_start,
    set_temp_ptr, set_total_pages, set_trie_c_entry, set_trie_hash_entry,
    set_trie_l_entry, set_trie_max, set_trie_min_entry, set_trie_not_ready,
    set_trie_o_entry, set_trie_op_lang_entry, set_trie_op_ptr, set_trie_op_val_entry,
    set_trie_ptr, set_trie_r_entry, set_trie_taken_entry, set_trie_trc_entry,
    set_trie_trl_entry, set_trie_tro_entry, set_trie_used_entry, set_two_to_the,
    set_use_err_help, set_var_used, set_write_open, set_xeq_level, str_ptr,
    str_pool_entry, str_start, string_vacancies, temp_ptr, trie_c_entry, trie_hash_entry,
    trie_l_entry, trie_max, trie_min_entry, trie_not_ready, trie_o_entry,
    par_loc, set_par_loc, set_par_token, set_write_loc,
    trie_op_lang_entry, trie_op_ptr, trie_op_val_entry, trie_ptr, trie_r_entry,
    trie_size, trie_taken_entry, trie_trc_entry, trie_trl_entry, trie_tro_entry,
    trie_used_entry, two_to_the,
};
use crate::io::UFile;

const INVALID_HANDLE: usize = 0;
const UTF8: i16 = 1;
const TOO_BIG_CHAR: i32 = 65_536;
const SINGLE_BASE: i32 = 1_114_113;
const PRIM_EQTB_BASE: i32 = 2_243_238;
const LEVEL_ONE: u16 = 1;
const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
const GET_NODE_HEAD_SENTINEL: i32 = 0x4000_0000;
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
    fn store_fmt_file();
    fn xmalloc(size: libc::size_t) -> *mut libc::c_void;
    static xetex_primitives: XetexPrimitiveDef;
    static xetex_primitives_count: libc::size_t;
}

/// Mirror of C `xetex_format_primitive_def_t` (xetex_format.h:715).
/// Layout: pointer (8) + u8 cmd (1+3 pad) + i32 chr (4) + i32 extra_init (4) + 4 trailing pad = 24.
#[repr(C)]
#[derive(Copy, Clone)]
struct XetexPrimitiveDef {
    name: *const libc::c_char,
    cmd: u8,
    chr: i32,
    extra_init: i32,
}

const _: () = assert!(std::mem::size_of::<XetexPrimitiveDef>() == 24);

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

// translated from xetex-ini.c:643 (new_patterns)
#[no_mangle]
pub unsafe extern "C" fn new_patterns() {
    const INT_BASE: i32 = 7_826_729;
    const INT_PAR_LANGUAGE: i32 = 50;
    const INT_PAR_SAVING_HYPH_CODES: i32 = 64;
    const LC_CODE_BASE: i32 = 3_370_281;
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
                                    v = new_trie_op(
                                        (k - l) as i16,
                                        hyf_entry(l as usize) as i16,
                                        v,
                                    );
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
        crate::mem::flush_list(crate::globals::def_ref());
    }
}

// translated from xetex-ini.c:863 (new_hyph_exceptions)
#[no_mangle]
pub unsafe extern "C" fn new_hyph_exceptions() {
    const INT_BASE: i32 = 7_826_729;
    const LC_CODE_BASE: i32 = 3_370_281;
    const LETTER: u8 = 11;
    const OTHER_CHAR: u8 = 12;
    const CHAR_GIVEN: u8 = 68;
    const CHAR_NUM: u8 = 16;
    const SPACER: u8 = 10;
    const RIGHT_BRACE_CMD: u8 = 2;
    const INT_PAR_LANGUAGE: i32 = 50;
    const HYPH_PRIME: i32 = 607;
    const TOO_BIG_CHAR: i32 = 65_536;

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
                            let q = crate::mem::get_avail();
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
                                set_hc_entry(
                                    n as usize,
                                    (hc_entry(0) - 65536) / 1024 + 55296,
                                );
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

                        let mut s = crate::string_pool::make_string();

                        if hyph_next() <= HYPH_PRIME {
                            while hyph_next() > 0 && hyph_word_entry(hyph_next() - 1) > 0 {
                                set_hyph_next(hyph_next() - 1);
                            }
                        }

                        if hyph_count() == hyph_size() || hyph_next() == 0 {
                            crate::errors::overflow(
                                c"exception dictionary".as_ptr(),
                                hyph_size(),
                            );
                        }

                        set_hyph_count(hyph_count() + 1);

                        // Hash lookup / insert into exception table
                        loop {
                            if hyph_word_entry(h) == 0 {
                                break;
                            }
                            let k = hyph_word_entry(h);

                            // Check if this slot holds the same word
                            let matched = if crate::string_pool::length(k)
                                != crate::string_pool::length(s)
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

// translated from xetex-ini.c:554 (sort_avail)
#[no_mangle]
pub unsafe extern "C" fn sort_avail() {
    let _ = crate::mem::get_node(GET_NODE_HEAD_SENTINEL);
    let mut p = mem_b32s1(rover() + 1);
    set_mem_b32s1(rover() + 1, MAX_HALFWORD);
    let old_rover = rover();

    /*136:*/
    while p != old_rover {
        if p < rover() {
            let q = p;
            p = mem_b32s1(q + 1);
            set_mem_b32s1(q + 1, rover());
            set_rover(q);
        } else {
            let mut q = rover();
            while mem_b32s1(q + 1) < p {
                q = mem_b32s1(q + 1);
            }
            let r = mem_b32s1(p + 1);
            set_mem_b32s1(p + 1, mem_b32s1(q + 1));
            set_mem_b32s1(q + 1, p);
            p = r;
        }
    }

    p = rover();
    while mem_b32s1(p + 1) != MAX_HALFWORD {
        crate::globals::set_mem_b32s0(mem_b32s1(p + 1) + 1, p);
        p = mem_b32s1(p + 1);
    }

    set_mem_b32s1(p + 1, rover());
    crate::globals::set_mem_b32s0(rover() + 1, p);
}

// translated from xetex-ini.c:597 (primitive)
#[no_mangle]
pub unsafe extern "C" fn primitive(ident: *const libc::c_char, c: u16, o: i32) {
    let len = libc::strlen(ident) as i32;
    let prim_val: i32;

    if len > 1 {
        let s = crate::texmfmp::maketexstring(ident);

        if first() + len > buf_size() + 1 {
            crate::errors::overflow(c"buffer size".as_ptr(), buf_size());
        }

        for i in 0..len {
            let b = *ident.offset(i as isize) as u8 as i32;
            set_buffer_entry(first() + i, b);
        }

        set_cur_val(crate::hash::id_lookup(first(), len));
        set_str_ptr(str_ptr() - 1);
        set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
        set_hash_s1(cur_val(), s);
        prim_val = crate::hash::prim_lookup(s);
    } else {
        let ch = *ident as u8 as i32;
        set_cur_val(ch + SINGLE_BASE);
        prim_val = crate::hash::prim_lookup(ch);
    }

    set_eqtb_b16s0(cur_val(), LEVEL_ONE);
    set_eqtb_b16s1(cur_val(), c);
    set_eqtb_b32s1(cur_val(), o);
    set_eqtb_b16s0(PRIM_EQTB_BASE + prim_val, LEVEL_ONE);
    set_eqtb_b16s1(PRIM_EQTB_BASE + prim_val, c);
    set_eqtb_b32s1(PRIM_EQTB_BASE + prim_val, o);
}

// translated from xetex-ini.c:2201 (final_cleanup)
#[no_mangle]
pub unsafe extern "C" fn final_cleanup() {
    const INT_BASE: i32 = 7_826_729;
    const INT_PAR_NEW_LINE_CHAR: i32 = 49;
    const IF_NODE_SIZE: i32 = 2;
    const IF_TEST: u16 = 107;
    const TOKEN_LIST: u16 = 0;
    const TOP_MARK_CODE: usize = 0;
    const SPLIT_BOT_MARK_CODE: usize = 4;
    const LAST_BOX_CODE: usize = 2;
    const VSPLIT_CODE: usize = 3;
    const MARK_VAL: usize = 7;
    const HISTORY_SPOTLESS: i32 = 0;
    const HISTORY_WARNING_ISSUED: i32 = 1;
    const ERROR_STOP_MODE: u8 = 3;
    const SELECTOR_TERM_ONLY: i32 = 17;
    const SELECTOR_TERM_AND_LOG: i32 = 19;

    let mut c = cur_chr();

    if c != 1 {
        set_eqtb_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR, -1);
    }

    if job_name() == 0 {
        crate::scan::open_log_file();
    }

    while crate::globals::input_ptr() > 0 {
        if cur_input().state == TOKEN_LIST {
            crate::input::end_token_list();
        } else {
            crate::input::end_file_reading();
        }
    }

    while open_parens() > 0 {
        crate::output::print_cstr(c" )".as_ptr());
        set_open_parens(open_parens() - 1);
    }

    if cur_level() as u16 > LEVEL_ONE {
        crate::output::print_nl(b'(' as i32);
        crate::output::print_esc_cstr(c"end occurred ".as_ptr());
        crate::output::print_cstr(c"inside a group at level ".as_ptr());
        crate::output::print_int(cur_level() as i32 - 1);
        crate::output::print_char(b')' as i32);
        crate::scan::show_save_groups();
    }

    while cond_ptr() != crate::mem::TEX_NULL {
        crate::output::print_nl(b'(' as i32);
        crate::output::print_esc_cstr(c"end occurred ".as_ptr());
        crate::output::print_cstr(c"when ".as_ptr());
        crate::output::print_cmd_chr(IF_TEST, cur_if() as i32);
        if if_line() != 0 {
            crate::output::print_cstr(c" on line ".as_ptr());
            crate::output::print_int(if_line());
        }
        crate::output::print_cstr(c" was incomplete)".as_ptr());
        set_if_line(mem_b32s1(cond_ptr() + 1));
        set_cur_if(mem_b16s0(cond_ptr()) as i16);
        set_temp_ptr(cond_ptr());
        set_cond_ptr(mem_b32s1(cond_ptr()));
        crate::mem::free_node(temp_ptr(), IF_NODE_SIZE);
    }

    if history() != HISTORY_SPOTLESS
        && (history() == HISTORY_WARNING_ISSUED || (interaction() as u8) < ERROR_STOP_MODE)
        && selector() == SELECTOR_TERM_AND_LOG
    {
        set_selector(SELECTOR_TERM_ONLY);
        crate::output::print_nl_cstr(
            c"(see the transcript file for additional information)".as_ptr(),
        );
        set_selector(SELECTOR_TERM_AND_LOG);
    }

    if c == 1 {
        if in_initex_mode() {
            c = TOP_MARK_CODE as i32;
            loop {
                if cur_mark(c as usize) != crate::mem::TEX_NULL {
                    crate::mem::delete_token_ref(cur_mark(c as usize));
                }
                if c as usize >= SPLIT_BOT_MARK_CODE {
                    break;
                }
                c += 1;
            }

            if sa_root(MARK_VAL) != crate::mem::TEX_NULL
                && crate::scan::do_marks(3, 0, sa_root(MARK_VAL))
            {
                set_sa_root(MARK_VAL, crate::mem::TEX_NULL);
            }

            c = LAST_BOX_CODE as i32;
            loop {
                crate::scan::flush_node_list(disc_ptr(c as usize));
                if c as usize >= VSPLIT_CODE {
                    break;
                }
                c += 1;
            }

            if last_glue() != MAX_HALFWORD {
                crate::mem::delete_glue_ref(last_glue());
            }
            store_fmt_file();
            return;
        }
        crate::output::print_nl_cstr(c"(\\dump is performed only by INITEX)".as_ptr());
    }
}

// translated from xetex-ini.c:2210 (initialize_more_variables)
#[no_mangle]
pub unsafe extern "C" fn initialize_more_variables() {
    const ERROR_STOP_MODE: u8 = 3;
    const VMODE: i16 = 1;
    const MEM_TOP: i32 = 4_999_999;
    const CONTRIB_HEAD: i32 = MEM_TOP - 1;
    const PAGE_HEAD: i32 = MEM_TOP - 2;
    const IGNORE_DEPTH: i32 = -65_536_000;
    const EMPTY: u8 = 0;
    const INT_BASE: i32 = 7_826_729;
    const EQTB_SIZE: i32 = 8_941_458;
    const PRIM_SIZE: i32 = 2100;
    const TOP_MARK_CODE: usize = 0;
    const FIRST_MARK_CODE: usize = 1;
    const BOT_MARK_CODE: usize = 2;
    const SPLIT_FIRST_MARK_CODE: usize = 3;
    const SPLIT_BOT_MARK_CODE: usize = 4;
    const INT_VAL: u8 = 0;
    const NORMAL: u8 = 0;
    const CLOSED: u8 = 2;
    const TOO_BIG_LANG: i32 = 256;
    const LEVEL_ZERO_U16: u16 = 0;
    const BOTTOM_LEVEL: u8 = 0;
    const LEFT_TO_RIGHT: i16 = 0;
    const MARK_VAL: usize = 7;
    const LAST_BOX_CODE: usize = 2;
    const VSPLIT_CODE: usize = 3;
    const HYPH_PRIME: i32 = 607;

    set_doing_special(false);
    set_native_text_size(128);
    set_native_text(
        xmalloc((128 * std::mem::size_of::<u16>()) as libc::size_t) as *mut u16,
    );

    set_interaction(ERROR_STOP_MODE);

    set_deletions_allowed(true);
    set_set_box_allowed(true);
    set_error_count(0);
    set_help_ptr(0);
    set_use_err_help(false);

    set_two_to_the(0, 1);
    for k in 1..=30usize {
        set_two_to_the(k, 2 * two_to_the(k - 1));
    }

    set_spec_log(1, 93_032_640);
    set_spec_log(2, 38_612_034);
    set_spec_log(3, 17_922_280);
    set_spec_log(4, 8_662_214);
    set_spec_log(5, 4_261_238);
    set_spec_log(6, 2_113_709);
    set_spec_log(7, 1_052_693);
    set_spec_log(8, 525_315);
    set_spec_log(9, 262_400);
    set_spec_log(10, 131_136);
    set_spec_log(11, 65_552);
    set_spec_log(12, 32_772);
    set_spec_log(13, 16_385);
    for k in 14..=27usize {
        set_spec_log(k, two_to_the(27 - k));
    }
    set_spec_log(28, 1);

    set_nest_ptr(0);
    set_max_nest_stack(0);
    set_cur_list_mode(VMODE);
    set_cur_list_head(CONTRIB_HEAD);
    set_cur_list_tail(CONTRIB_HEAD);
    set_cur_list_e_tex_aux(crate::mem::TEX_NULL);
    set_cur_list_aux_b32s1(IGNORE_DEPTH);
    set_cur_list_mode_line(0);
    set_cur_list_prev_graf(0);
    set_shown_mode(0);
    set_page_contents(EMPTY);
    set_page_tail(PAGE_HEAD);
    set_last_glue(MAX_HALFWORD);
    set_last_penalty(0);
    set_last_kern(0);
    set_last_node_type(-1);
    set_page_so_far(7, 0);

    for k in INT_BASE..=EQTB_SIZE {
        set_xeq_level(k, LEVEL_ONE);
    }

    set_no_new_control_sequence(true);
    set_prim_s0(0, 0);
    set_prim_s1(0, 0);

    for k in 1..=PRIM_SIZE {
        set_prim_s0(k, 0);
        set_prim_s1(k, 0);
    }

    set_save_ptr(0);
    set_cur_level(LEVEL_ONE);
    set_cur_group(BOTTOM_LEVEL);
    set_cur_boundary(0);
    set_max_save_stack(0);
    set_mag_set(0);
    set_expand_depth_count(0);
    set_is_in_csname(false);
    set_cur_mark(TOP_MARK_CODE, crate::mem::TEX_NULL);
    set_cur_mark(FIRST_MARK_CODE, crate::mem::TEX_NULL);
    set_cur_mark(BOT_MARK_CODE, crate::mem::TEX_NULL);
    set_cur_mark(SPLIT_FIRST_MARK_CODE, crate::mem::TEX_NULL);
    set_cur_mark(SPLIT_BOT_MARK_CODE, crate::mem::TEX_NULL);
    set_cur_val(0);
    set_cur_val_level(INT_VAL);
    set_radix(0);
    set_cur_order(NORMAL);

    for k in 0..=16usize {
        set_read_open_entry(k, CLOSED);
    }

    set_cond_ptr(crate::mem::TEX_NULL);
    set_if_limit(NORMAL);
    set_cur_if(0);
    set_if_line(0);
    set_null_character(crate::mem_types::B16x4 {
        s0: 0,
        s1: 0,
        s2: 0,
        s3: 0,
    });
    set_total_pages(0);
    set_max_v(0);
    set_max_h(0);
    set_max_push(0);
    set_last_bop(-1);
    set_doing_leaders(false);
    set_dead_cycles(0);
    set_adjust_tail(crate::mem::TEX_NULL);
    set_last_badness(0);
    set_pre_adjust_tail(crate::mem::TEX_NULL);
    set_pack_begin_line(0);
    set_empty_b32s1(EMPTY as i32);
    set_empty_b32s0(crate::mem::TEX_NULL);
    set_align_ptr(crate::mem::TEX_NULL);
    set_cur_align(crate::mem::TEX_NULL);
    set_cur_span(crate::mem::TEX_NULL);
    set_cur_loop(crate::mem::TEX_NULL);
    set_cur_head(crate::mem::TEX_NULL);
    set_cur_tail_a(crate::mem::TEX_NULL);
    set_cur_pre_head(crate::mem::TEX_NULL);
    set_cur_pre_tail(crate::mem::TEX_NULL);
    set_cur_f(0);
    set_max_hyph_char(TOO_BIG_LANG);

    for z in 0..=hyph_size() {
        set_hyph_word_entry(z, 0);
        set_hyph_list_entry(z, crate::mem::TEX_NULL);
        set_hyph_link_entry(z, 0);
    }

    set_hyph_count(0);
    set_hyph_next(HYPH_PRIME + 1);
    if hyph_next() > hyph_size() {
        set_hyph_next(HYPH_PRIME);
    }

    set_output_active(false);
    set_insert_penalties(0);
    set_ligature_present(false);
    set_cancel_boundary(false);
    set_lft_hit(false);
    set_rt_hit(false);
    set_ins_disc(false);
    set_after_token(0);
    set_long_help_seen(false);
    set_format_ident(0);

    for k in 0..=17usize {
        set_write_open(k, false);
    }

    set_lr_ptr(crate::mem::TEX_NULL);
    set_lr_problems(0);
    set_cur_dir(LEFT_TO_RIGHT);
    set_pseudo_files(crate::mem::TEX_NULL);
    set_sa_root(MARK_VAL, crate::mem::TEX_NULL);
    set_sa_null_b32s0(crate::mem::TEX_NULL);
    set_sa_null_b32s1(crate::mem::TEX_NULL);
    set_sa_chain(crate::mem::TEX_NULL);
    set_sa_level(LEVEL_ZERO_U16);
    set_disc_ptr(LAST_BOX_CODE, crate::mem::TEX_NULL);
    set_disc_ptr(VSPLIT_CODE, crate::mem::TEX_NULL);
    set_edit_name_start(0);
    set_stop_at_space(true);
}

// translated from xetex-ini.c:2212 (initialize_more_initex_variables)
#[no_mangle]
pub unsafe extern "C" fn initialize_more_initex_variables() {
    const MEM_TOP: i32 = 4_999_999;
    const PAGE_INS_HEAD: i32 = MEM_TOP;
    const PAGE_HEAD: i32 = MEM_TOP - 2;
    const ACTIVE_LIST: i32 = MEM_TOP - 7;
    const END_SPAN: i32 = MEM_TOP - 9;
    const OMIT_TEMPLATE: i32 = MEM_TOP - 10;
    const PRE_ADJUST_HEAD: i32 = MEM_TOP - 14;

    const NORMAL_U16: u16 = 0;
    const FIL: u16 = 1;
    const FILL: u16 = 2;
    const HYPHENATED: u16 = 1;
    const SPLIT_UP: u16 = 1;
    const GLUE_NODE: u16 = 10;
    const HI_MEM_STAT_USAGE: i32 = 15;
    const CS_TOKEN_FLAG: i32 = 0x1FFF_FFF;
    const FROZEN_CONTROL_SEQUENCE: i32 = 2_243_226;
    const FROZEN_END_TEMPLATE: i32 = FROZEN_CONTROL_SEQUENCE + 5;
    const UNDEFINED_CS: u16 = 103;
    const UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
    const LEVEL_ZERO: u16 = 0;
    const ACTIVE_BASE: i32 = 1;
    const GLUE_BASE: i32 = 2_254_340;
    const LOCAL_BASE: i32 = 2_254_871;
    const LOCAL_PAR_SHAPE: i32 = 0;
    const LOCAL_OUTPUT_ROUTINE: i32 = 1;
    const ETEX_PEN_BASE: i32 = 2_255_140;
    const NUM_ETEX_PENALTIES: i32 = 4;
    const TOKS_BASE: i32 = 2_254_884;
    const NUMBER_REGS: i32 = 256;
    const BOX_BASE: i32 = 2_255_144;
    const CUR_FONT_LOC: i32 = 2_255_400;
    const FONT_BASE: i32 = 0;
    const MATH_FONT_BASE: i32 = 2_255_401;
    const NUMBER_MATH_FAMILIES: i32 = 256;
    const NUMBER_MATH_FONTS: i32 = 3 * NUMBER_MATH_FAMILIES;
    const CAT_CODE_BASE: i32 = 2_256_169;
    const INT_BASE: i32 = 7_826_729;
    const DIMEN_BASE: i32 = 8_941_180;
    const DEL_CODE_BASE: i32 = 7_827_068;
    const LC_CODE_BASE: i32 = 3_370_281;
    const UC_CODE_BASE: i32 = 4_484_393;
    const SF_CODE_BASE: i32 = 5_598_505;
    const MATH_CODE_BASE: i32 = 6_712_617;
    const EQTB_SIZE: i32 = 8_941_458;
    const NUMBER_USVS: i32 = 0x10_FFFF + 1;
    const GLUE_REF: u16 = 119;
    const SHAPE_REF: u16 = 120;
    const BOX_REF: u16 = 121;
    const DATA: u16 = 122;
    const OTHER_CHAR: i32 = 12;
    const LETTER: i32 = 11;
    const CAR_RET: i32 = 5;
    const SPACER: i32 = 10;
    const ESCAPE: i32 = 0;
    const COMMENT: i32 = 14;
    const INVALID_CHAR: i32 = 15;
    const IGNORE: i32 = 9;
    const VAR_FAM_CLASS: i32 = 7;
    const CARRIAGE_RETURN: i32 = 13;
    const INT_PAR_MAG: i32 = 17;
    const INT_PAR_TOLERANCE: i32 = 1;
    const INT_PAR_HANG_AFTER: i32 = 41;
    const INT_PAR_MAX_DEAD_CYCLES: i32 = 40;
    const INT_PAR_ESCAPE_CHAR: i32 = 45;
    const INT_PAR_END_LINE_CHAR: i32 = 48;
    const INT_PAR_XETEX_HYPHENATABLE_LENGTH: i32 = 80;
    const PRIM_SIZE: i32 = 2100;
    const TRIE_OP_SIZE_I32: i32 = 35111;
    const BIGGEST_LANG: usize = 255;
    const MIN_TRIE_OP_U16: u16 = 0;
    const INT_VAL: usize = 0;
    const INTER_CHAR_VAL: usize = 6;

    // set_family(x) = (x & 0xFF) << 24; set_class(x) = (x & 0x07) << 21
    fn set_family(x: i32) -> i32 {
        ((x & 0xFF) << 24) as i32
    }
    fn set_class(x: i32) -> i32 {
        ((x & 0x07) << 21) as i32
    }

    for k in 1..=19 {
        set_mem_b32s1(k, 0);
    }

    let mut k = 0;
    while k <= 19 {
        set_mem_b32s1(k, crate::mem::TEX_NULL + 1);
        set_mem_b16s1(k, NORMAL_U16);
        set_mem_b16s0(k, NORMAL_U16);
        k += 4;
    }

    set_mem_b32s1(6, 65536);
    set_mem_b16s1(4, FIL);
    set_mem_b32s1(10, 65536);
    set_mem_b16s1(8, FILL);
    set_mem_b32s1(14, 65536);
    set_mem_b16s1(12, FIL);
    set_mem_b32s1(15, 65536);
    set_mem_b16s0(12, FIL);
    set_mem_b32s1(18, -65536);
    set_mem_b16s1(16, FIL);
    set_rover(20);
    set_mem_b32s1(rover(), MAX_HALFWORD);
    set_mem_b32s0(rover(), 1000);
    set_mem_b32s0(rover() + 1, rover());
    set_mem_b32s1(rover() + 1, rover());
    set_lo_mem_max(rover() + 1000);
    set_mem_b32s1(crate::globals::lo_mem_max(), crate::mem::TEX_NULL);
    set_mem_b32s0(crate::globals::lo_mem_max(), crate::mem::TEX_NULL);

    let template = crate::globals::mem_word(crate::globals::lo_mem_max());
    for k in PRE_ADJUST_HEAD..=MEM_TOP {
        set_mem_word(k, template);
    }

    set_mem_b32s0(OMIT_TEMPLATE, CS_TOKEN_FLAG + FROZEN_END_TEMPLATE);
    set_mem_b32s1(END_SPAN, u16::MAX as i32 + 1);
    set_mem_b32s0(END_SPAN, crate::mem::TEX_NULL);
    set_mem_b16s1(ACTIVE_LIST, HYPHENATED);
    set_mem_b32s0(ACTIVE_LIST + 1, MAX_HALFWORD);
    set_mem_b16s0(ACTIVE_LIST, 0);
    set_mem_b16s0(PAGE_INS_HEAD, 255);
    set_mem_b16s1(PAGE_INS_HEAD, SPLIT_UP);
    set_mem_b32s1(PAGE_INS_HEAD, PAGE_INS_HEAD);
    set_mem_b16s1(PAGE_HEAD, GLUE_NODE);
    set_mem_b16s0(PAGE_HEAD, NORMAL_U16);
    set_avail(crate::mem::TEX_NULL);
    set_mem_end(MEM_TOP);
    set_hi_mem_min(PRE_ADJUST_HEAD);
    set_var_used(20);
    set_dyn_used(HI_MEM_STAT_USAGE);

    set_eqtb_b16s1(UNDEFINED_CONTROL_SEQUENCE, UNDEFINED_CS);
    set_eqtb_b32s1(UNDEFINED_CONTROL_SEQUENCE, crate::mem::TEX_NULL);
    set_eqtb_b16s0(UNDEFINED_CONTROL_SEQUENCE, LEVEL_ZERO);

    let undefined_entry = eqtb_word(UNDEFINED_CONTROL_SEQUENCE);
    for k in ACTIVE_BASE..=eqtb_top() {
        set_eqtb_word(k, undefined_entry);
    }

    set_eqtb_b32s1(GLUE_BASE, 0);
    set_eqtb_b16s0(GLUE_BASE, LEVEL_ONE);
    set_eqtb_b16s1(GLUE_BASE, GLUE_REF);

    let glue_base_entry = eqtb_word(GLUE_BASE);
    for k in (GLUE_BASE + 1)..=(LOCAL_BASE - 1) {
        set_eqtb_word(k, glue_base_entry);
    }

    set_mem_b32s1(0, mem_b32s1(0) + 531);
    set_eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE, crate::mem::TEX_NULL);
    set_eqtb_b16s1(LOCAL_BASE + LOCAL_PAR_SHAPE, SHAPE_REF);
    set_eqtb_b16s0(LOCAL_BASE + LOCAL_PAR_SHAPE, LEVEL_ONE);

    let par_shape_entry = eqtb_word(LOCAL_BASE + LOCAL_PAR_SHAPE);
    for k in ETEX_PEN_BASE..=(NUM_ETEX_PENALTIES - 1) {
        set_eqtb_word(k, par_shape_entry);
    }

    for k in (LOCAL_BASE + LOCAL_OUTPUT_ROUTINE)..=(TOKS_BASE + NUMBER_REGS - 1) {
        set_eqtb_word(k, undefined_entry);
    }

    set_eqtb_b32s1(BOX_BASE, crate::mem::TEX_NULL);
    set_eqtb_b16s1(BOX_BASE, BOX_REF);
    set_eqtb_b16s0(BOX_BASE, LEVEL_ONE);

    let box_base_entry = eqtb_word(BOX_BASE);
    for k in (BOX_BASE + 1)..=(BOX_BASE + NUMBER_REGS - 1) {
        set_eqtb_word(k, box_base_entry);
    }

    set_eqtb_b32s1(CUR_FONT_LOC, FONT_BASE);
    set_eqtb_b16s1(CUR_FONT_LOC, DATA);
    set_eqtb_b16s0(CUR_FONT_LOC, LEVEL_ONE);

    let cur_font_entry = eqtb_word(CUR_FONT_LOC);
    for k in MATH_FONT_BASE..=(MATH_FONT_BASE + NUMBER_MATH_FONTS - 1) {
        set_eqtb_word(k, cur_font_entry);
    }

    set_eqtb_b32s1(CAT_CODE_BASE, 0);
    set_eqtb_b16s1(CAT_CODE_BASE, DATA);
    set_eqtb_b16s0(CAT_CODE_BASE, LEVEL_ONE);

    let cat_code_entry = eqtb_word(CAT_CODE_BASE);
    for k in (CAT_CODE_BASE + 1)..=(INT_BASE - 1) {
        set_eqtb_word(k, cat_code_entry);
    }

    for k in 0..NUMBER_USVS {
        set_eqtb_b32s1(CAT_CODE_BASE + k, OTHER_CHAR);
        set_eqtb_b32s1(MATH_CODE_BASE + k, k);
        set_eqtb_b32s1(SF_CODE_BASE + k, 1000);
    }

    set_eqtb_b32s1(CAT_CODE_BASE + 13, CAR_RET);
    set_eqtb_b32s1(CAT_CODE_BASE + 32, SPACER);
    set_eqtb_b32s1(CAT_CODE_BASE + 92, ESCAPE);
    set_eqtb_b32s1(CAT_CODE_BASE + 37, COMMENT);
    set_eqtb_b32s1(CAT_CODE_BASE + 127, INVALID_CHAR);
    set_eqtb_b32s1(CAT_CODE_BASE, IGNORE);

    for k in (b'0' as i32)..=(b'9' as i32) {
        set_eqtb_b32s1(MATH_CODE_BASE + k, k + set_class(VAR_FAM_CLASS));
    }

    for k in (b'A' as i32)..=(b'Z' as i32) {
        set_eqtb_b32s1(CAT_CODE_BASE + k, LETTER);
        set_eqtb_b32s1(CAT_CODE_BASE + k + 32, LETTER);
        set_eqtb_b32s1(MATH_CODE_BASE + k, k + set_family(1) + set_class(VAR_FAM_CLASS));
        set_eqtb_b32s1(
            MATH_CODE_BASE + k + 32,
            k + 32 + set_family(1) + set_class(VAR_FAM_CLASS),
        );
        set_eqtb_b32s1(LC_CODE_BASE + k, k + 32);
        set_eqtb_b32s1(LC_CODE_BASE + k + 32, k + 32);
        set_eqtb_b32s1(UC_CODE_BASE + k, k);
        set_eqtb_b32s1(UC_CODE_BASE + k + 32, k);
        set_eqtb_b32s1(SF_CODE_BASE + k, 999);
    }

    for k in INT_BASE..=(DEL_CODE_BASE - 1) {
        set_eqtb_b32s1(k, 0);
    }

    set_eqtb_b32s1(INT_BASE + INT_PAR_MAG, 1000);
    set_eqtb_b32s1(INT_BASE + INT_PAR_TOLERANCE, 10000);
    set_eqtb_b32s1(INT_BASE + INT_PAR_HANG_AFTER, 1);
    set_eqtb_b32s1(INT_BASE + INT_PAR_MAX_DEAD_CYCLES, 25);
    set_eqtb_b32s1(INT_BASE + INT_PAR_ESCAPE_CHAR, b'\\' as i32);
    set_eqtb_b32s1(INT_BASE + INT_PAR_END_LINE_CHAR, CARRIAGE_RETURN);

    for k in 0..NUMBER_USVS {
        set_eqtb_b32s1(DEL_CODE_BASE + k, -1);
    }

    set_eqtb_b32s1(DEL_CODE_BASE + 46, 0);

    for k in DIMEN_BASE..=EQTB_SIZE {
        set_eqtb_b32s1(k, 0);
    }

    set_prim_used(PRIM_SIZE);
    set_hash_used(FROZEN_CONTROL_SEQUENCE);
    set_hash_high(0);
    set_cs_count(0);

    for k in (-TRIE_OP_SIZE_I32)..=TRIE_OP_SIZE_I32 {
        set_trie_op_hash(k, 0);
    }

    for k in 0..=BIGGEST_LANG {
        set_trie_used_entry(k, MIN_TRIE_OP_U16);
    }

    set_max_op_used(MIN_TRIE_OP_U16);
    set_trie_op_ptr(0);
    set_trie_not_ready(true);

    set_format_ident(crate::texmfmp::maketexstring(c" (INITEX)".as_ptr()));

    set_max_reg_num(32767);
    set_max_reg_help_line(c"A register number must be between 0 and 32767.".as_ptr());

    for i in INT_VAL..=INTER_CHAR_VAL {
        set_sa_root(i, crate::mem::TEX_NULL);
    }

    set_eqtb_b32s1(INT_BASE + INT_PAR_XETEX_HYPHENATABLE_LENGTH, 63);
}

// translated from xetex-ini.c:2215 (initialize_primitives)
#[no_mangle]
pub unsafe extern "C" fn initialize_primitives() {
    const CS_TOKEN_FLAG: i32 = 0x1FFF_FFF;
    const FROZEN_CONTROL_SEQUENCE: i32 = 2_243_226;
    const FROZEN_PROTECTION: i32 = FROZEN_CONTROL_SEQUENCE;
    const FROZEN_END_TEMPLATE: i32 = FROZEN_CONTROL_SEQUENCE + 5;
    const FROZEN_ENDV: i32 = FROZEN_CONTROL_SEQUENCE + 6;
    const END_WRITE: i32 = FROZEN_CONTROL_SEQUENCE + 8;
    const FROZEN_DONT_EXPAND: i32 = FROZEN_CONTROL_SEQUENCE + 9;
    const FROZEN_PRIMITIVE: i32 = FROZEN_CONTROL_SEQUENCE + 11;

    const ENDV: u16 = 9;
    const IGNORE_SPACES: u16 = 39;
    const OUTER_CALL: u16 = 115;
    const END_TEMPLATE: u16 = 117;
    const DONT_EXPAND: u16 = 118;

    const MEM_TOP: i32 = 4_999_999;
    const NULL_LIST: i32 = MEM_TOP - 11;

    const XF_PRIM_INIT_NONE: i32 = 0;
    const XF_PRIM_INIT_PAR: i32 = 1;
    const XF_PRIM_INIT_WRITE: i32 = 2;

    set_no_new_control_sequence(false);
    set_first(0);

    let table_ptr = &raw const xetex_primitives;
    let count = xetex_primitives_count;
    for i in 0..count {
        let prim = table_ptr.add(i).read();

        if !prim.name.is_null() {
            primitive(prim.name, prim.cmd as u16, prim.chr);

            match prim.extra_init {
                XF_PRIM_INIT_NONE => {}
                XF_PRIM_INIT_PAR => {
                    set_par_loc(cur_val());
                    set_par_token(CS_TOKEN_FLAG + par_loc());
                }
                XF_PRIM_INIT_WRITE => {
                    set_write_loc(cur_val());
                }
                frozen => {
                    set_hash_s1(frozen, crate::texmfmp::maketexstring(prim.name));
                    set_eqtb_word(frozen, eqtb_word(cur_val()));
                }
            }
        }
    }

    set_hash_s1(
        FROZEN_END_TEMPLATE,
        crate::texmfmp::maketexstring(c"endtemplate".as_ptr()),
    );
    set_eqtb_b16s1(FROZEN_END_TEMPLATE, END_TEMPLATE);
    set_eqtb_b32s1(FROZEN_END_TEMPLATE, NULL_LIST);
    set_eqtb_b16s0(FROZEN_END_TEMPLATE, LEVEL_ONE);

    set_hash_s1(
        FROZEN_ENDV,
        crate::texmfmp::maketexstring(c"endtemplate".as_ptr()),
    );
    set_eqtb_b16s1(FROZEN_ENDV, ENDV);
    set_eqtb_b32s1(FROZEN_ENDV, NULL_LIST);
    set_eqtb_b16s0(FROZEN_ENDV, LEVEL_ONE);

    set_hash_s1(
        FROZEN_DONT_EXPAND,
        crate::texmfmp::maketexstring(c"notexpanded:".as_ptr()),
    );
    set_eqtb_b16s1(FROZEN_DONT_EXPAND, DONT_EXPAND);

    set_hash_s1(
        FROZEN_PRIMITIVE,
        crate::texmfmp::maketexstring(c"primitive".as_ptr()),
    );
    set_eqtb_b16s1(FROZEN_PRIMITIVE, IGNORE_SPACES);
    set_eqtb_b32s1(FROZEN_PRIMITIVE, 1);
    set_eqtb_b16s0(FROZEN_PRIMITIVE, LEVEL_ONE);

    set_hash_s1(
        FROZEN_PROTECTION,
        crate::texmfmp::maketexstring(c"inaccessible".as_ptr()),
    );

    set_hash_s1(
        END_WRITE,
        crate::texmfmp::maketexstring(c"endwrite".as_ptr()),
    );
    set_eqtb_b16s0(END_WRITE, LEVEL_ONE);
    set_eqtb_b16s1(END_WRITE, OUTER_CALL);
    set_eqtb_b32s1(END_WRITE, crate::mem::TEX_NULL);

    set_no_new_control_sequence(true);
}

// All tt_cleanup_ items need #[link_name] since the Rust names carry a prefix
// that doesn't exist in C, and some also clash with existing accessor functions.
extern "C" {
    #[link_name = "pdf_files_close"]
    fn tt_cleanup_pdf_files_close();
    #[link_name = "destroy_font_manager"]
    fn tt_cleanup_destroy_font_manager();
    #[link_name = "TECkit_DisposeConverter"]
    fn tt_cleanup_TECkit_DisposeConverter(cnv: *mut libc::c_void);
    #[link_name = "TEX_format_default"]
    static mut tt_cleanup_TEX_format_default: *mut libc::c_char;
    #[link_name = "font_used"]
    static mut tt_cleanup_font_used: *mut libc::c_void;
    #[link_name = "buffer"]
    static mut tt_cleanup_buffer: *mut libc::c_void;
    #[link_name = "nest"]
    static mut tt_cleanup_nest: *mut libc::c_void;
    #[link_name = "save_stack"]
    static mut tt_cleanup_save_stack: *mut libc::c_void;
    #[link_name = "input_stack"]
    static mut tt_cleanup_input_stack: *mut libc::c_void;
    #[link_name = "input_file"]
    static mut tt_cleanup_input_file: *mut libc::c_void;
    #[link_name = "line_stack"]
    static mut tt_cleanup_line_stack: *mut libc::c_void;
    #[link_name = "eof_seen"]
    static mut tt_cleanup_eof_seen: *mut libc::c_void;
    #[link_name = "grp_stack"]
    static mut tt_cleanup_grp_stack: *mut libc::c_void;
    #[link_name = "if_stack"]
    static mut tt_cleanup_if_stack: *mut libc::c_void;
    #[link_name = "source_filename_stack"]
    static mut tt_cleanup_source_filename_stack: *mut libc::c_void;
    #[link_name = "full_source_filename_stack"]
    static mut tt_cleanup_full_source_filename_stack: *mut libc::c_void;
    #[link_name = "param_stack"]
    static mut tt_cleanup_param_stack: *mut libc::c_void;
    #[link_name = "hyph_word"]
    static mut tt_cleanup_hyph_word: *mut libc::c_void;
    #[link_name = "hyph_list"]
    static mut tt_cleanup_hyph_list: *mut libc::c_void;
    #[link_name = "hyph_link"]
    static mut tt_cleanup_hyph_link: *mut libc::c_void;
    #[link_name = "native_text"]
    static mut tt_cleanup_native_text: *mut libc::c_void;
    #[link_name = "tt_cleanup_free_yhash"]
    fn tt_cleanup_yhash();
    #[link_name = "eqtb"]
    static mut tt_cleanup_eqtb: *mut libc::c_void;
    #[link_name = "mem"]
    static mut tt_cleanup_mem: *mut libc::c_void;
    #[link_name = "str_start"]
    static mut tt_cleanup_str_start: *mut libc::c_void;
    #[link_name = "str_pool"]
    static mut tt_cleanup_str_pool: *mut libc::c_void;
    #[link_name = "font_info"]
    static mut tt_cleanup_font_info: *mut libc::c_void;
    #[link_name = "font_mapping"]
    static mut tt_cleanup_font_mapping: *mut libc::c_void;
    #[link_name = "font_layout_engine"]
    static mut tt_cleanup_font_layout_engine: *mut libc::c_void;
    #[link_name = "font_flags"]
    static mut tt_cleanup_font_flags: *mut libc::c_void;
    #[link_name = "font_letter_space"]
    static mut tt_cleanup_font_letter_space: *mut libc::c_void;
    #[link_name = "font_check"]
    static mut tt_cleanup_font_check: *mut libc::c_void;
    #[link_name = "font_size"]
    static mut tt_cleanup_font_size: *mut libc::c_void;
    #[link_name = "font_dsize"]
    static mut tt_cleanup_font_dsize: *mut libc::c_void;
    #[link_name = "font_params"]
    static mut tt_cleanup_font_params: *mut libc::c_void;
    #[link_name = "font_name"]
    static mut tt_cleanup_font_name: *mut libc::c_void;
    #[link_name = "font_area"]
    static mut tt_cleanup_font_area: *mut libc::c_void;
    #[link_name = "font_bc"]
    static mut tt_cleanup_font_bc: *mut libc::c_void;
    #[link_name = "font_ec"]
    static mut tt_cleanup_font_ec: *mut libc::c_void;
    #[link_name = "font_glue"]
    static mut tt_cleanup_font_glue: *mut libc::c_void;
    #[link_name = "hyphen_char"]
    static mut tt_cleanup_hyphen_char: *mut libc::c_void;
    #[link_name = "skew_char"]
    static mut tt_cleanup_skew_char: *mut libc::c_void;
    #[link_name = "bchar_label"]
    static mut tt_cleanup_bchar_label: *mut libc::c_void;
    #[link_name = "font_bchar"]
    static mut tt_cleanup_font_bchar: *mut libc::c_void;
    #[link_name = "font_false_bchar"]
    static mut tt_cleanup_font_false_bchar: *mut libc::c_void;
    #[link_name = "char_base"]
    static mut tt_cleanup_char_base: *mut libc::c_void;
    #[link_name = "width_base"]
    static mut tt_cleanup_width_base: *mut libc::c_void;
    #[link_name = "height_base"]
    static mut tt_cleanup_height_base: *mut libc::c_void;
    #[link_name = "depth_base"]
    static mut tt_cleanup_depth_base: *mut libc::c_void;
    #[link_name = "italic_base"]
    static mut tt_cleanup_italic_base: *mut libc::c_void;
    #[link_name = "lig_kern_base"]
    static mut tt_cleanup_lig_kern_base: *mut libc::c_void;
    #[link_name = "kern_base"]
    static mut tt_cleanup_kern_base: *mut libc::c_void;
    #[link_name = "exten_base"]
    static mut tt_cleanup_exten_base: *mut libc::c_void;
    #[link_name = "param_base"]
    static mut tt_cleanup_param_base: *mut libc::c_void;
    #[link_name = "trie_trl"]
    static mut tt_cleanup_trie_trl: *mut libc::c_void;
    #[link_name = "trie_tro"]
    static mut tt_cleanup_trie_tro: *mut libc::c_void;
    #[link_name = "trie_trc"]
    static mut tt_cleanup_trie_trc: *mut libc::c_void;
}

// translated from xetex-ini.c:2226 (tt_cleanup)
#[no_mangle]
pub unsafe extern "C" fn tt_cleanup() {
    tt_cleanup_pdf_files_close();
    libc::free(tt_cleanup_TEX_format_default as *mut _);
    libc::free(tt_cleanup_font_used);
    crate::shipout::deinitialize_shipout_variables();

    tt_cleanup_destroy_font_manager();

    for font_k in 0..font_max() {
        let engine = font_layout_engine_ptr(font_k);
        if !engine.is_null() {
            crate::ext::release_font_engine(engine, font_area_entry(font_k));
            crate::globals::set_font_layout_engine_ptr(font_k, std::ptr::null_mut());
        }
        let mapping = font_mapping_ptr(font_k);
        if !mapping.is_null() {
            tt_cleanup_TECkit_DisposeConverter(mapping);
            crate::globals::set_font_mapping_ptr(font_k, std::ptr::null_mut());
        }
    }

    for i in 1..=in_open() {
        let f = input_file(i);
        if !f.is_null() {
            crate::io::u_close(f as *mut crate::io::UFile);
        }
    }

    // Free the big allocated arrays
    libc::free(tt_cleanup_buffer);
    libc::free(tt_cleanup_nest);
    libc::free(tt_cleanup_save_stack);
    libc::free(tt_cleanup_input_stack);
    libc::free(tt_cleanup_input_file);
    libc::free(tt_cleanup_line_stack);
    libc::free(tt_cleanup_eof_seen);
    libc::free(tt_cleanup_grp_stack);
    libc::free(tt_cleanup_if_stack);
    libc::free(tt_cleanup_source_filename_stack);
    libc::free(tt_cleanup_full_source_filename_stack);
    libc::free(tt_cleanup_param_stack);
    libc::free(tt_cleanup_hyph_word);
    libc::free(tt_cleanup_hyph_list);
    libc::free(tt_cleanup_hyph_link);

    // initialize_more_variables @ 3277
    libc::free(tt_cleanup_native_text);

    // Free arrays allocated in load_fmt_file
    tt_cleanup_yhash(); // frees file-static yhash via C helper
    libc::free(tt_cleanup_eqtb);
    libc::free(tt_cleanup_mem);
    libc::free(tt_cleanup_str_start);
    libc::free(tt_cleanup_str_pool);
    libc::free(tt_cleanup_font_info);

    libc::free(tt_cleanup_font_mapping);
    libc::free(tt_cleanup_font_layout_engine);
    libc::free(tt_cleanup_font_flags);
    libc::free(tt_cleanup_font_letter_space);
    libc::free(tt_cleanup_font_check);
    libc::free(tt_cleanup_font_size);
    libc::free(tt_cleanup_font_dsize);
    libc::free(tt_cleanup_font_params);
    libc::free(tt_cleanup_font_name);
    libc::free(tt_cleanup_font_area);
    libc::free(tt_cleanup_font_bc);
    libc::free(tt_cleanup_font_ec);
    libc::free(tt_cleanup_font_glue);
    libc::free(tt_cleanup_hyphen_char);
    libc::free(tt_cleanup_skew_char);
    libc::free(tt_cleanup_bchar_label);
    libc::free(tt_cleanup_font_bchar);
    libc::free(tt_cleanup_font_false_bchar);
    libc::free(tt_cleanup_char_base);
    libc::free(tt_cleanup_width_base);
    libc::free(tt_cleanup_height_base);
    libc::free(tt_cleanup_depth_base);
    libc::free(tt_cleanup_italic_base);
    libc::free(tt_cleanup_lig_kern_base);
    libc::free(tt_cleanup_kern_base);
    libc::free(tt_cleanup_exten_base);
    libc::free(tt_cleanup_param_base);

    // mfree(trie_trl) ≡ free(trie_trl); trie_trl = NULL
    libc::free(tt_cleanup_trie_trl);
    tt_cleanup_trie_trl = std::ptr::null_mut();
    libc::free(tt_cleanup_trie_tro);
    tt_cleanup_trie_tro = std::ptr::null_mut();
    libc::free(tt_cleanup_trie_trc);
    tt_cleanup_trie_trc = std::ptr::null_mut();
}
