//! Rust translation of initialization helpers from xetex-ini.c.

use crate::globals::{
    after_token, avail, buf_size, cond_ptr, cs_count, cur_chr, cur_cmd, cur_cs, cur_if, cur_input,
    cur_lang, cur_level, cur_mark, cur_ptr, cur_tok, cur_val, def_ref, disc_ptr, dyn_used,
    eqtb_b16s0, eqtb_b16s1, eqtb_b32s1, eqtb_top, eqtb_word, first, fmem_ptr, font_area_entry,
    font_dsize_entry, font_layout_engine_ptr, font_mapping_ptr, font_max, font_mem_size,
    font_name_entry, font_ptr, font_size_entry, format_ident, hash_high, hash_used, hc_entry,
    hi_mem_min, history, hyf_distance_entry, hyf_entry, hyf_next_entry, hyf_num_entry, hyph_count,
    hyph_index, hyph_link_entry, hyph_list_entry, hyph_next, hyph_size, hyph_start,
    hyph_word_entry, if_line, in_initex_mode, in_open, init_pool_ptr, input_file, interaction,
    job_name, last, last_glue, lo_mem_max, max_hyph_char, max_op_used, mem_b16s0, mem_b32s0,
    mem_b32s1, mem_end, op_start_entry, open_parens, par_loc, pool_ptr, pool_size, rover, sa_root,
    selector, set_adjust_tail, set_after_token, set_align_ptr, set_avail, set_box_allowed,
    set_buffer_entry, set_cancel_boundary, set_cond_ptr, set_cs_count, set_cur_align,
    set_cur_boundary, set_cur_chr, set_cur_cmd, set_cur_cs, set_cur_dir, set_cur_f, set_cur_group,
    set_cur_head, set_cur_if, set_cur_input_limit, set_cur_input_loc, set_cur_lang, set_cur_level,
    set_cur_list_aux_b32s1, set_cur_list_e_tex_aux, set_cur_list_head, set_cur_list_mode,
    set_cur_list_mode_line, set_cur_list_prev_graf, set_cur_list_tail, set_cur_loop, set_cur_mark,
    set_cur_order, set_cur_pre_head, set_cur_pre_tail, set_cur_ptr, set_cur_span, set_cur_tail_a,
    set_cur_tok, set_cur_val, set_cur_val_level, set_dead_cycles, set_deletions_allowed,
    set_disc_ptr, set_doing_leaders, set_doing_special, set_dyn_used, set_edit_name_start,
    set_empty_b32s0, set_empty_b32s1, set_eqtb_b16s0, set_eqtb_b16s1, set_eqtb_b32s1,
    set_eqtb_word, set_error_count, set_error_line, set_expand_depth_count,
    set_file_line_error_style_p, set_first, set_fmem_ptr, set_font_info_b32s1, set_font_max,
    set_font_mem_size, set_font_ptr, set_format_ident, set_half_error_line, set_hash_high,
    set_hash_s1, set_hash_used, set_hc_entry, set_help_line, set_help_ptr, set_hi_mem_min,
    set_hyf_distance_entry, set_hyf_entry, set_hyf_next_entry, set_hyf_num_entry, set_hyph_count,
    set_hyph_index, set_hyph_link_entry, set_hyph_list_entry, set_hyph_next, set_hyph_start,
    set_hyph_word_entry, set_hyphen_char_entry, set_if_limit, set_if_line, set_input_file,
    set_ins_disc, set_insert_penalties, set_interaction, set_is_in_csname, set_last,
    set_last_badness, set_last_bop, set_last_glue, set_last_kern, set_last_node_type,
    set_last_penalty, set_lft_hit, set_ligature_present, set_lo_mem_max, set_long_help_seen,
    set_lr_problems, set_lr_ptr, set_mag_set, set_max_h, set_max_hyph_char, set_max_nest_stack,
    set_max_op_used, set_max_print_line, set_max_push, set_max_reg_help_line, set_max_reg_num,
    set_max_save_stack, set_max_v, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1,
    set_mem_end, set_mem_word, set_native_text, set_native_text_size, set_nest_ptr,
    set_no_new_control_sequence, set_null_character, set_op_start_entry, set_open_parens,
    set_output_active, set_pack_begin_line, set_page_contents, set_page_so_far, set_page_tail,
    set_par_loc, set_par_token, set_pool_ptr, set_pre_adjust_tail, set_prim_s0, set_prim_s1,
    set_prim_used, set_pseudo_files, set_radix, set_read_open_entry, set_rover, set_rt_hit,
    set_sa_chain, set_sa_level, set_sa_null_b32s0, set_sa_null_b32s1, set_sa_root, set_save_ptr,
    set_selector, set_set_box_allowed, set_shown_mode, set_skew_char_entry, set_spec_log,
    set_stop_at_space, set_str_pool, set_str_ptr, set_str_start, set_temp_ptr, set_total_pages,
    set_trie_c_entry, set_trie_hash_entry, set_trie_l_entry, set_trie_max, set_trie_min_entry,
    set_trie_not_ready, set_trie_o_entry, set_trie_op_lang_entry, set_trie_op_ptr,
    set_trie_op_val_entry, set_trie_ptr, set_trie_r_entry, set_trie_taken_entry,
    set_trie_trc_entry, set_trie_trl_entry, set_trie_tro_entry, set_trie_used_entry,
    set_two_to_the, set_use_err_help, set_var_used, set_write_loc, set_write_open, set_xeq_level,
    str_pool_entry, str_ptr, str_start, string_vacancies, temp_ptr, trie_c_entry, trie_hash_entry,
    trie_l_entry, trie_max, trie_min_entry, trie_not_ready, trie_o_entry, trie_op_lang_entry,
    trie_op_ptr, trie_op_val_entry, trie_ptr, trie_r_entry, trie_size, trie_taken_entry,
    trie_trc_entry, trie_trl_entry, trie_tro_entry, trie_used_entry, two_to_the, var_used,
    write_loc,
};
use crate::io::UFile;

const INVALID_HANDLE: usize = 0;
const UTF8: i16 = 1;
const TOO_BIG_CHAR: i32 = 65_536;
const SINGLE_BASE: i32 = 1_114_113;
const PRIM_EQTB_BASE: i32 = 2_243_238;
const LEVEL_ONE: u16 = 1;
const MIN_HALFWORD: i32 = -0x0FFF_FFFF;
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
    fn xmalloc(size: libc::size_t) -> *mut libc::c_void;
    fn xcalloc(nelem: libc::size_t, elsize: libc::size_t) -> *mut libc::c_void;
    fn set_cp_code(font_num: i32, code: u32, side: i32, value: i32);
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

                        let mut s = crate::string_pool::make_string();

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
                            let matched =
                                if crate::string_pool::length(k) != crate::string_pool::length(s) {
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
    set_native_text(xmalloc((128 * std::mem::size_of::<u16>()) as libc::size_t) as *mut u16);

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
        set_eqtb_b32s1(
            MATH_CODE_BASE + k,
            k + set_family(1) + set_class(VAR_FAM_CLASS),
        );
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

const FORMAT_HASH_OFFSET: i32 = 514;
const MEM_TOP: i32 = 4_999_999;
const EQTB_SIZE: i32 = 8_941_458;
const HASH_PRIME: i32 = 557_111;
const HASH_SIZE: i32 = 2_242_711;
const HASH_BASE: i32 = 1_114_114;
const HASH_TOP_UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
const CS_TOKEN_FLAG: i32 = 0x1FFF_FFF;
const FONT_BASE: i32 = 0;
const MAX_FONT_MAX: i32 = 9_000;
const CONTRIB_HEAD: i32 = MEM_TOP - 1;
const PAGE_HEAD: i32 = MEM_TOP - 2;
const PRE_ADJUST_HEAD: i32 = MEM_TOP - 14;
const HI_MEM_STAT_USAGE: i32 = 15;
const SELECTOR_NO_PRINT: i32 = 16;
const SELECTOR_TERM_ONLY: i32 = 17;
const HISTORY_FATAL_ERROR: i32 = 3;
const HISTORY_SPOTLESS: i32 = 0;
const BATCH_MODE: u8 = 0;
const NEW_LINE: u16 = 33;
const EMPTY_STRING: i32 = 65_536;
const NON_ADDRESS: i32 = 0;
const INT_PAR_TIME: i32 = 20;
const INT_PAR_DAY: i32 = 21;
const INT_PAR_MONTH: i32 = 22;
const INT_PAR_YEAR: i32 = 23;
const INT_PAR_XETEX_GENERATE_ACTUAL_TEXT: i32 = 79;
const INT_PAR_END_LINE_CHAR: i32 = 48;
const INT_PAR_TRACING_STATS: i32 = 31;
const FORMAT_HEADER_MAGIC: i32 = 0x5454_4E43;
const FORMAT_FOOTER_MAGIC: i32 = 0x0000_029A;
const FORMAT_SERIAL: i32 = 33;
const FORMAT_HASH_PRIME: i32 = 8_501;
const FORMAT_HYPH_PRIME: i32 = 607;
const FORMAT_HASH_BASE: i32 = 2_228_226;
const INT_VAL: i32 = 0;
const FROZEN_CONTROL_SEQUENCE: i32 = 2_243_226;
const UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
const FONT_ID_BASE: i32 = 2_245_338;
const PRIM_SIZE: i32 = 2_100;
const ACTIVE_BASE: i32 = 1;
const FORMAT_INT_BASE: i32 = 7_826_729;
const SELECTOR_LOG_ONLY: i32 = 18;
const SELECTOR_TERM_AND_LOG: i32 = 19;
const SELECTOR_NEW_STRING: i32 = 21;
const ERROR_STOP_MODE: u8 = 3;
const SCROLL_MODE: u8 = 2;
const TTBC_FILE_FORMAT_FORMAT: libc::c_int = 10;
const SUP_MAX_STRINGS: i32 = 2_097_151;
const SUP_FONT_MEM_SIZE: i32 = 147_483_647;
const SUP_POOL_SIZE: i32 = 40_000_000;
const SUP_HASH_EXTRA: i32 = 2_097_151;

static mut tre_pool_size: i32 = 0;
static mut tre_string_vacancies: i32 = 0;
static mut tre_max_strings: i32 = 0;
static mut tre_trie_size: i32 = 0;
static mut tre_hyph_size: i32 = 0;

#[inline]
unsafe fn xmalloc_array<T>(n: i32) -> *mut T {
    xmalloc(((n as usize) + 1) * std::mem::size_of::<T>()) as *mut T
}

#[inline]
unsafe fn xcalloc_array<T>(n: i32) -> *mut T {
    xcalloc((n as usize) + 1, std::mem::size_of::<T>()) as *mut T
}

// Raw lifecycle state used by tt_run_engine. Kept local because this top-level
// transition glue allocates many globals in one place and will disappear when
// EngineState replaces the C globals.
extern "C" {
    fn ttstub_output_open(path: *const libc::c_char, is_gz: libc::c_int) -> usize;
    fn ttstub_output_write(
        handle: usize,
        data: *const libc::c_char,
        len: libc::size_t,
    ) -> libc::size_t;
    fn ttstub_output_close(handle: usize) -> libc::c_int;
    fn ttstub_output_flush(handle: usize) -> libc::c_int;
    fn ttstub_input_open(
        path: *const libc::c_char,
        format: libc::c_int,
        is_gz: libc::c_int,
    ) -> usize;
    fn ttstub_input_read(handle: usize, data: *mut u8, len: libc::size_t) -> libc::ssize_t;
    fn ttstub_input_close(handle: usize) -> libc::c_int;
    fn ttstub_output_open_stdout() -> usize;
    fn tt_run_engine_allocate_yhash();
    fn tt_cleanup_free_yhash();
    fn pdf_files_init();

    #[link_name = "TEX_format_default"]
    static mut tre_TEX_format_default: *mut libc::c_char;

    #[link_name = "format_default_length"]
    static mut tre_format_default_length: i32;
    #[link_name = "pool_free"]
    static mut tre_pool_free: i32;
    #[link_name = "strings_free"]
    static mut tre_strings_free: i32;
    #[link_name = "buf_size"]
    static mut tre_buf_size: i32;
    #[link_name = "nest_size"]
    static mut tre_nest_size: i32;
    #[link_name = "max_in_open"]
    static mut tre_max_in_open: i32;
    #[link_name = "param_size"]
    static mut tre_param_size: i32;
    #[link_name = "save_size"]
    static mut tre_save_size: i32;
    #[link_name = "stack_size"]
    static mut tre_stack_size: i32;
    #[link_name = "hash_extra"]
    static mut tre_hash_extra: i32;
    #[link_name = "expand_depth"]
    static mut tre_expand_depth: i32;
    #[link_name = "mem"]
    static mut tre_mem: *mut crate::mem_types::MemoryWord;
    #[link_name = "hash"]
    static mut tre_hash: *mut crate::mem_types::B32x2;
    #[link_name = "eqtb_top"]
    static mut tre_eqtb_top: i32;
    #[link_name = "hash_top"]
    static mut tre_hash_top: i32;
    #[link_name = "eqtb"]
    static mut tre_eqtb: *mut crate::mem_types::MemoryWord;

    #[link_name = "font_info"]
    static mut tre_font_info: *mut crate::mem_types::MemoryWord;
    #[link_name = "bad"]
    static mut tre_bad: i32;

    #[link_name = "output_file_extension"]
    static mut tre_output_file_extension: *const libc::c_char;
    #[link_name = "font_mapping"]
    static mut tre_font_mapping: *mut *mut libc::c_void;
    #[link_name = "font_layout_engine"]
    static mut tre_font_layout_engine: *mut *mut libc::c_void;
    #[link_name = "font_flags"]
    static mut tre_font_flags: *mut libc::c_char;
    #[link_name = "font_letter_space"]
    static mut tre_font_letter_space: *mut i32;
    #[link_name = "font_glue"]
    static mut tre_font_glue: *mut i32;
    #[link_name = "hyphen_char"]
    static mut tre_hyphen_char: *mut i32;
    #[link_name = "skew_char"]
    static mut tre_skew_char: *mut i32;
    #[link_name = "char_base"]
    static mut tre_char_base: *mut i32;
    #[link_name = "width_base"]
    static mut tre_width_base: *mut i32;
    #[link_name = "height_base"]
    static mut tre_height_base: *mut i32;
    #[link_name = "depth_base"]
    static mut tre_depth_base: *mut i32;
    #[link_name = "italic_base"]
    static mut tre_italic_base: *mut i32;
    #[link_name = "lig_kern_base"]
    static mut tre_lig_kern_base: *mut i32;
    #[link_name = "kern_base"]
    static mut tre_kern_base: *mut i32;
    #[link_name = "exten_base"]
    static mut tre_exten_base: *mut i32;
    #[link_name = "param_base"]
    static mut tre_param_base: *mut i32;
    #[link_name = "font_used"]
    static mut tre_font_used: *mut bool;
    #[link_name = "sa_root"]
    static mut fmt_sa_root: [i32; 8];
}

#[inline]
unsafe fn dump_bytes(fmt_out: usize, ptr: *const u8, item_size: usize, nitems: usize) {
    let byte_len = item_size * nitems;
    let written = if cfg!(target_endian = "little") && item_size > 1 {
        let mut bytes = std::slice::from_raw_parts(ptr, byte_len).to_vec();
        for item in bytes.chunks_exact_mut(item_size) {
            item.reverse();
        }
        ttstub_output_write(fmt_out, bytes.as_ptr() as *const libc::c_char, byte_len)
    } else {
        ttstub_output_write(fmt_out, ptr as *const libc::c_char, byte_len)
    };

    if written != byte_len {
        _tt_abort(c"could not write format file data".as_ptr());
    }
}

#[inline]
unsafe fn dump_slice<T>(fmt_out: usize, ptr: *const T, nitems: i32) {
    if nitems > 0 {
        dump_bytes(
            fmt_out,
            ptr as *const u8,
            std::mem::size_of::<T>(),
            nitems as usize,
        );
    }
}

#[inline]
unsafe fn dump_one<T: Copy>(fmt_out: usize, value: T) {
    dump_slice(fmt_out, &value as *const T, 1);
}

#[inline]
unsafe fn dump_int(fmt_out: usize, value: i32) {
    dump_one(fmt_out, value);
}

#[inline]
unsafe fn dump_b32(fmt_out: usize, s0: i32, s1: i32) {
    dump_one(fmt_out, crate::mem_types::B32x2 { s0, s1 });
}

#[inline]
unsafe fn undump_bytes(fmt_in: usize, ptr: *mut u8, item_size: usize, nitems: usize) {
    let byte_len = item_size * nitems;
    let read = ttstub_input_read(fmt_in, ptr, byte_len);
    if read < 0 || read as usize != byte_len {
        _tt_abort(c"could not read format file data".as_ptr());
    }

    if cfg!(target_endian = "little") && item_size > 1 {
        let bytes = std::slice::from_raw_parts_mut(ptr, byte_len);
        for item in bytes.chunks_exact_mut(item_size) {
            item.reverse();
        }
    }
}

#[inline]
unsafe fn undump_slice<T>(fmt_in: usize, ptr: *mut T, nitems: i32) {
    if nitems > 0 {
        undump_bytes(
            fmt_in,
            ptr as *mut u8,
            std::mem::size_of::<T>(),
            nitems as usize,
        );
    }
}

#[inline]
unsafe fn undump_one<T: Copy>(fmt_in: usize) -> T {
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    undump_slice(fmt_in, value.as_mut_ptr(), 1);
    value.assume_init()
}

#[inline]
unsafe fn undump_int(fmt_in: usize) -> i32 {
    undump_one(fmt_in)
}

unsafe fn undump_checked_i32(
    fmt_in: usize,
    ptr: *mut i32,
    nitems: i32,
    low: i32,
    high: i32,
) -> bool {
    undump_slice(fmt_in, ptr, nitems);
    for i in 0..nitems {
        let value = *ptr.add(i as usize);
        if value < low || value > high {
            return false;
        }
    }
    true
}

unsafe fn undump_upper_i32(fmt_in: usize, ptr: *mut i32, nitems: i32, high: i32) -> bool {
    undump_slice(fmt_in, ptr, nitems);
    for i in 0..nitems {
        if *ptr.add(i as usize) > high {
            return false;
        }
    }
    true
}

unsafe fn undump_upper_u16(fmt_in: usize, ptr: *mut u16, nitems: i32, high: u16) -> bool {
    undump_slice(fmt_in, ptr, nitems);
    for i in 0..nitems {
        if *ptr.add(i as usize) > high {
            return false;
        }
    }
    true
}

// translated from xetex-ini.c:587 (store_fmt_file)
#[no_mangle]
pub unsafe extern "C" fn store_fmt_file() {
    if crate::globals::save_ptr() != 0 {
        crate::output::error_here_with_diagnostic(c"You can't dump inside a group".as_ptr());
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(1);
        set_help_line(0, c"`{...\\dump}' is a no-no.".as_ptr());

        if interaction() == ERROR_STOP_MODE {
            set_interaction(SCROLL_MODE);
        }
        if crate::globals::log_opened() {
            crate::errors::error();
        }

        crate::globals::set_history(HISTORY_FATAL_ERROR);
        crate::scan::close_files_and_terminate();
        ttstub_output_flush(crate::globals::rust_stdout());
        _tt_abort(c"\\dump inside a group".as_ptr());
    }

    set_selector(SELECTOR_NEW_STRING);
    crate::output::print_cstr(c" (preloaded format=".as_ptr());
    crate::output::print(job_name());
    crate::output::print_char(b' ' as i32);
    crate::output::print_int(intpar(INT_PAR_YEAR));
    crate::output::print_char(b'.' as i32);
    crate::output::print_int(intpar(INT_PAR_MONTH));
    crate::output::print_char(b'.' as i32);
    crate::output::print_int(intpar(INT_PAR_DAY));
    crate::output::print_char(b')' as i32);

    if interaction() == BATCH_MODE {
        set_selector(SELECTOR_LOG_ONLY);
    } else {
        set_selector(SELECTOR_TERM_AND_LOG);
    }

    if pool_ptr() + 1 > pool_size() {
        crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr());
    }

    set_format_ident(crate::string_pool::make_string());
    crate::scan::pack_job_name(c".fmt".as_ptr());

    let fmt_out = ttstub_output_open(crate::globals::name_of_file(), 0);
    if fmt_out == INVALID_HANDLE {
        _tt_abort(c"cannot open format output file".as_ptr());
    }

    crate::output::print_nl_cstr(c"Beginning to dump on file ".as_ptr());
    crate::output::print(crate::scan::make_name_string());

    set_str_ptr(str_ptr() - 1);
    set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));

    crate::output::print_nl_cstr(c"".as_ptr());
    crate::output::print(format_ident());

    dump_int(fmt_out, FORMAT_HEADER_MAGIC);
    dump_int(fmt_out, FORMAT_SERIAL);
    dump_int(fmt_out, hash_high());

    while crate::globals::pseudo_files() != crate::mem::TEX_NULL {
        crate::input::pseudo_close();
    }

    dump_int(fmt_out, MEM_TOP);
    dump_int(fmt_out, EQTB_SIZE);
    dump_int(fmt_out, FORMAT_HASH_PRIME);
    dump_int(fmt_out, FORMAT_HYPH_PRIME);

    dump_int(fmt_out, pool_ptr());
    dump_int(fmt_out, str_ptr());
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.strings.str_start.as_ptr(),
            str_ptr() - TOO_BIG_CHAR + 1,
        );
        dump_slice(fmt_out, state.strings.str_pool.as_ptr(), pool_ptr());
    });

    crate::output::print_ln();
    crate::output::print_int(str_ptr());
    crate::output::print_cstr(c" strings of total length ".as_ptr());
    crate::output::print_int(pool_ptr());

    sort_avail();
    set_var_used(0);
    dump_int(fmt_out, lo_mem_max());
    dump_int(fmt_out, rover());

    for k in INT_VAL as usize..=INTER_CHAR_VAL as usize {
        dump_int(fmt_out, fmt_sa_root[k]);
    }

    let mut p = 0;
    let mut q = rover();
    let mut x = 0;
    loop {
        let len = q + 2 - p;
        dump_slice(fmt_out, tre_mem.add(p as usize), len);
        x += len;
        set_var_used(var_used() + q - p);
        p = q + mem_b32s0(q);
        q = mem_b32s1(q + 1);
        if q == rover() {
            break;
        }
    }

    set_var_used(var_used() + lo_mem_max() - p);
    set_dyn_used(mem_end() + 1 - hi_mem_min());
    dump_slice(fmt_out, tre_mem.add(p as usize), lo_mem_max() + 1 - p);
    x += lo_mem_max() + 1 - p;

    dump_int(fmt_out, hi_mem_min());
    dump_int(fmt_out, avail());
    dump_slice(
        fmt_out,
        tre_mem.add(hi_mem_min() as usize),
        mem_end() + 1 - hi_mem_min(),
    );
    x += mem_end() + 1 - hi_mem_min();

    p = avail();
    while p != crate::mem::TEX_NULL {
        set_dyn_used(dyn_used() - 1);
        p = mem_b32s1(p);
    }

    dump_int(fmt_out, var_used());
    dump_int(fmt_out, dyn_used());

    crate::output::print_ln();
    crate::output::print_int(x);
    crate::output::print_cstr(c" memory locations dumped; current usage is ".as_ptr());
    crate::output::print_int(var_used());
    crate::output::print_char(b'&' as i32);
    crate::output::print_int(dyn_used());

    let mut k = ACTIVE_BASE;
    loop {
        let mut j = k;
        while j < FORMAT_INT_BASE - 1 {
            if eqtb_b32s1(j) == eqtb_b32s1(j + 1)
                && eqtb_b16s1(j) == eqtb_b16s1(j + 1)
                && eqtb_b16s0(j) == eqtb_b16s0(j + 1)
            {
                break;
            }
            j += 1;
        }
        let l = if j < FORMAT_INT_BASE - 1 {
            j += 1;
            let l = j;
            while j < FORMAT_INT_BASE - 1
                && eqtb_b32s1(j) == eqtb_b32s1(j + 1)
                && eqtb_b16s1(j) == eqtb_b16s1(j + 1)
                && eqtb_b16s0(j) == eqtb_b16s0(j + 1)
            {
                j += 1;
            }
            l
        } else {
            FORMAT_INT_BASE
        };

        dump_int(fmt_out, l - k);
        dump_slice(fmt_out, tre_eqtb.add(k as usize), l - k);
        k = j + 1;
        dump_int(fmt_out, k - l);
        if k == FORMAT_INT_BASE {
            break;
        }
    }

    loop {
        let mut j = k;
        while j < EQTB_SIZE {
            if eqtb_b32s1(j) == eqtb_b32s1(j + 1) {
                break;
            }
            j += 1;
        }
        let l = if j < EQTB_SIZE {
            j += 1;
            let l = j;
            while j < EQTB_SIZE && eqtb_b32s1(j) == eqtb_b32s1(j + 1) {
                j += 1;
            }
            l
        } else {
            EQTB_SIZE + 1
        };

        dump_int(fmt_out, l - k);
        dump_slice(fmt_out, tre_eqtb.add(k as usize), l - k);
        k = j + 1;
        dump_int(fmt_out, k - l);
        if k > EQTB_SIZE {
            break;
        }
    }

    if hash_high() > 0 {
        dump_slice(fmt_out, tre_eqtb.add((EQTB_SIZE + 1) as usize), hash_high());
    }

    dump_int(fmt_out, par_loc());
    dump_int(fmt_out, write_loc());

    for p in 0..=PRIM_SIZE {
        dump_b32(
            fmt_out,
            crate::globals::prim_s0(p),
            crate::globals::prim_s1(p),
        );
    }

    dump_int(fmt_out, hash_used());
    set_cs_count((FROZEN_CONTROL_SEQUENCE - 1) - hash_used() + hash_high());

    for p in FORMAT_HASH_BASE..=hash_used() {
        if crate::globals::hash_s1(p) != 0 {
            dump_int(fmt_out, p);
            dump_b32(
                fmt_out,
                crate::globals::hash_s0(p),
                crate::globals::hash_s1(p),
            );
            set_cs_count(cs_count() + 1);
        }
    }

    for p in (hash_used() + 1)..=(UNDEFINED_CONTROL_SEQUENCE - 1) {
        dump_b32(
            fmt_out,
            crate::globals::hash_s0(p),
            crate::globals::hash_s1(p),
        );
    }
    if hash_high() > 0 {
        for p in (EQTB_SIZE + 1)..=(EQTB_SIZE + hash_high()) {
            dump_b32(
                fmt_out,
                crate::globals::hash_s0(p),
                crate::globals::hash_s1(p),
            );
        }
    }

    dump_int(fmt_out, cs_count());

    crate::output::print_ln();
    crate::output::print_int(cs_count());
    crate::output::print_cstr(c" multiletter control sequences".as_ptr());

    dump_int(fmt_out, fmem_ptr());
    dump_slice(fmt_out, tre_font_info, fmem_ptr());
    dump_int(fmt_out, font_ptr());
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_check.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_size.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
        dump_slice(
            fmt_out,
            state.fonts.font_dsize.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_params.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    dump_slice(
        fmt_out,
        tre_hyphen_char.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    dump_slice(
        fmt_out,
        tre_skew_char.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_name.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_area.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_bc.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_ec.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    dump_slice(
        fmt_out,
        tre_char_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    dump_slice(
        fmt_out,
        tre_width_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    dump_slice(
        fmt_out,
        tre_height_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    dump_slice(
        fmt_out,
        tre_depth_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    dump_slice(
        fmt_out,
        tre_italic_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    dump_slice(
        fmt_out,
        tre_lig_kern_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    dump_slice(
        fmt_out,
        tre_kern_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    dump_slice(
        fmt_out,
        tre_exten_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    dump_slice(
        fmt_out,
        tre_param_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    dump_slice(
        fmt_out,
        tre_font_glue.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.bchar_label.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_bchar.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state
                .fonts
                .font_false_bchar
                .as_ptr()
                .add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });

    for k in FONT_BASE..=font_ptr() {
        crate::output::print_nl_cstr(c"\\font".as_ptr());
        crate::output::print_esc(crate::globals::hash_s1(FONT_ID_BASE + k));
        crate::output::print_char(b'=' as i32);

        if font_area_entry(k) == AAT_FONT_FLAG
            || font_area_entry(k) == OTGR_FONT_FLAG
            || !font_mapping_ptr(k).is_null()
        {
            crate::output::print_file_name(font_name_entry(k), EMPTY_STRING, EMPTY_STRING);
            crate::output::error_here_with_diagnostic(
                c"Can't \\dump a format with native fonts or font-mappings".as_ptr(),
            );
            crate::output::capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(3);
            set_help_line(2, c"You really, really don't want to do this.".as_ptr());
            set_help_line(1, c"It won't work, and only confuses me.".as_ptr());
            set_help_line(
                0,
                c"(Load them at runtime, not as part of the format file.)".as_ptr(),
            );
            crate::errors::error();
        } else {
            crate::output::print_file_name(font_name_entry(k), font_area_entry(k), EMPTY_STRING);
        }

        if font_size_entry(k) != font_dsize_entry(k) {
            crate::output::print_cstr(c" at ".as_ptr());
            crate::output::print_scaled(font_size_entry(k));
            crate::output::print_cstr(c"pt".as_ptr());
        }
    }

    crate::output::print_ln();
    crate::output::print_int(fmem_ptr() - 7);
    crate::output::print_cstr(c" words of font info for ".as_ptr());
    crate::output::print_int(font_ptr());
    if font_ptr() != FONT_BASE + 1 {
        crate::output::print_cstr(c" preloaded fonts".as_ptr());
    } else {
        crate::output::print_cstr(c" preloaded font".as_ptr());
    }

    dump_int(fmt_out, hyph_count());
    if hyph_next() <= FORMAT_HYPH_PRIME {
        set_hyph_next(hyph_size());
    }
    dump_int(fmt_out, hyph_next());

    for k in 0..=hyph_size() {
        if hyph_word_entry(k) != 0 {
            dump_int(fmt_out, k + 65_536 * hyph_link_entry(k) as i32);
            dump_int(fmt_out, hyph_word_entry(k));
            dump_int(fmt_out, hyph_list_entry(k));
        }
    }

    crate::output::print_ln();
    crate::output::print_int(hyph_count());
    if hyph_count() != 1 {
        crate::output::print_cstr(c" hyphenation exceptions".as_ptr());
    } else {
        crate::output::print_cstr(c" hyphenation exception".as_ptr());
    }

    if trie_not_ready() {
        init_trie();
    }

    dump_int(fmt_out, trie_max());
    dump_int(fmt_out, hyph_start());
    let fmt_trie_trl = crate::state::with_state(|state| state.hyph.trie_trl.as_ptr());
    let fmt_trie_tro = crate::state::with_state(|state| state.hyph.trie_tro.as_ptr());
    let fmt_trie_trc = crate::state::with_state(|state| state.hyph.trie_trc.as_ptr());
    dump_slice(fmt_out, fmt_trie_trl, trie_max() + 1);
    dump_slice(fmt_out, fmt_trie_tro, trie_max() + 1);
    dump_slice(fmt_out, fmt_trie_trc, trie_max() + 1);
    dump_int(fmt_out, max_hyph_char());
    dump_int(fmt_out, trie_op_ptr());
    let fmt_hyf_distance = crate::state::with_state(|state| state.hyph.hyf_distance.as_ptr());
    let fmt_hyf_num = crate::state::with_state(|state| state.hyph.hyf_num.as_ptr());
    let fmt_hyf_next = crate::state::with_state(|state| state.hyph.hyf_next.as_ptr());
    dump_slice(fmt_out, fmt_hyf_distance.add(1), trie_op_ptr());
    dump_slice(fmt_out, fmt_hyf_num.add(1), trie_op_ptr());
    dump_slice(fmt_out, fmt_hyf_next.add(1), trie_op_ptr());

    crate::output::print_nl_cstr(c"Hyphenation trie of length ".as_ptr());
    crate::output::print_int(trie_max());
    crate::output::print_cstr(c" has ".as_ptr());
    crate::output::print_int(trie_op_ptr());
    if trie_op_ptr() != 1 {
        crate::output::print_cstr(c" ops".as_ptr());
    } else {
        crate::output::print_cstr(c" op".as_ptr());
    }
    crate::output::print_cstr(c" out of ".as_ptr());
    crate::output::print_int(TRIE_OP_SIZE);

    for k in (0..=BIGGEST_LANG as usize).rev() {
        let used = trie_used_entry(k);
        if used > 0 {
            crate::output::print_nl_cstr(c"  ".as_ptr());
            crate::output::print_int(used as i32);
            crate::output::print_cstr(c" for language ".as_ptr());
            crate::output::print_int(k as i32);
            dump_int(fmt_out, k as i32);
            dump_int(fmt_out, used as i32);
        }
    }

    dump_int(fmt_out, FORMAT_FOOTER_MAGIC);
    set_eqtb_b32s1(PC_INT_BASE + INT_PAR_TRACING_STATS, 0);
    ttstub_output_close(fmt_out);
}

// translated from xetex-ini.c:590 (pack_buffered_name)
unsafe fn pack_buffered_name(_n: i16, _a: i32, _b: i32) {
    libc::free(crate::globals::name_of_file() as *mut libc::c_void);
    let name = xmalloc((tre_format_default_length as usize) + 1) as *mut libc::c_char;
    libc::strcpy(name, tre_TEX_format_default);
    crate::globals::set_name_of_file(name);
    crate::globals::set_name_length(libc::strlen(name) as i32);
}

#[inline]
unsafe fn bad_format_file() -> ! {
    _tt_abort(c"fatal format file error".as_ptr());
}

// translated from xetex-ini.c:602 (load_fmt_file)
#[no_mangle]
pub unsafe extern "C" fn load_fmt_file() -> bool {
    let mut j: i32;
    let mut k: i32;
    let mut p: i32;
    let mut q: i32;
    let mut x: i32;

    j = cur_input().loc;
    pack_buffered_name((tre_format_default_length - 4) as i16, 1, 0);

    let fmt_in = ttstub_input_open(crate::globals::name_of_file(), TTBC_FILE_FORMAT_FORMAT, 0);
    if fmt_in == INVALID_HANDLE {
        _tt_abort(c"cannot open the format file".as_ptr());
    }

    set_cur_input_loc(j);

    if in_initex_mode() {
        libc::free(tre_font_info as *mut libc::c_void);
        crate::state::with_state_mut(|state| {
            state.strings.str_pool = Vec::new();
            state.strings.str_start = Vec::new();
        });
        tt_cleanup_free_yhash();
        libc::free(tre_eqtb as *mut libc::c_void);
        libc::free(tre_mem as *mut libc::c_void);
        tre_mem = std::ptr::null_mut();
    }

    x = undump_int(fmt_in);
    if x != FORMAT_HEADER_MAGIC {
        bad_format_file();
    }

    x = undump_int(fmt_in);
    if x != FORMAT_SERIAL {
        _tt_abort(c"format file is of the wrong version".as_ptr());
    }

    x = undump_int(fmt_in);
    if x < 0 || x > SUP_HASH_EXTRA {
        bad_format_file();
    }
    set_hash_high(x);
    if tre_hash_extra < hash_high() {
        tre_hash_extra = hash_high();
    }

    tre_eqtb_top = EQTB_SIZE + tre_hash_extra;
    if tre_hash_extra == 0 {
        tre_hash_top = UNDEFINED_CONTROL_SEQUENCE;
    } else {
        tre_hash_top = tre_eqtb_top;
    }

    tt_run_engine_allocate_yhash();
    crate::globals::set_hash_s0(FORMAT_HASH_BASE, 0);
    crate::globals::set_hash_s1(FORMAT_HASH_BASE, 0);
    for h in (FORMAT_HASH_BASE + 1)..=tre_hash_top {
        crate::globals::set_hash_s0(h, crate::globals::hash_s0(FORMAT_HASH_BASE));
        crate::globals::set_hash_s1(h, crate::globals::hash_s1(FORMAT_HASH_BASE));
    }

    tre_eqtb = xmalloc_array::<crate::mem_types::MemoryWord>(tre_eqtb_top);
    set_eqtb_b16s1(UNDEFINED_CONTROL_SEQUENCE, UNDEFINED_CS as u16);
    set_eqtb_b32s1(UNDEFINED_CONTROL_SEQUENCE, crate::mem::TEX_NULL);
    set_eqtb_b16s0(UNDEFINED_CONTROL_SEQUENCE, 0);
    for e in (EQTB_SIZE + 1)..=tre_eqtb_top {
        *tre_eqtb.add(e as usize) = *tre_eqtb.add(UNDEFINED_CONTROL_SEQUENCE as usize);
    }

    set_max_reg_num(32_767);
    set_max_reg_help_line(c"A register number must be between 0 and 32767.".as_ptr());

    x = undump_int(fmt_in);
    if x != MEM_TOP {
        bad_format_file();
    }

    set_cur_list_head(CONTRIB_HEAD);
    set_cur_list_tail(CONTRIB_HEAD);
    set_page_tail(PAGE_HEAD);
    tre_mem = xmalloc_array::<crate::mem_types::MemoryWord>(MEM_TOP);

    x = undump_int(fmt_in);
    if x != EQTB_SIZE {
        bad_format_file();
    }
    x = undump_int(fmt_in);
    if x != FORMAT_HASH_PRIME {
        bad_format_file();
    }
    x = undump_int(fmt_in);
    if x != FORMAT_HYPH_PRIME {
        bad_format_file();
    }

    x = undump_int(fmt_in);
    if x < 0 {
        bad_format_file();
    }
    if x > SUP_POOL_SIZE - tre_pool_free {
        _tt_abort(c"must increase string_pool_size".as_ptr());
    }
    set_pool_ptr(x);
    if tre_pool_size < pool_ptr() + tre_pool_free {
        tre_pool_size = pool_ptr() + tre_pool_free;
    }
    crate::state::with_state_mut(|state| {
        state.strings.pool_size = tre_pool_size;
    });

    x = undump_int(fmt_in);
    if x < 0 {
        bad_format_file();
    }
    if x > SUP_MAX_STRINGS - tre_strings_free {
        _tt_abort(c"must increase sup_strings".as_ptr());
    }
    set_str_ptr(x);
    if tre_max_strings < str_ptr() + tre_strings_free {
        tre_max_strings = str_ptr() + tre_strings_free;
    }
    crate::state::with_state_mut(|state| {
        state.strings.max_strings = tre_max_strings;
    });

    crate::state::with_state_mut(|state| {
        state.strings.str_start = vec![0i32; (tre_max_strings as usize) + 1];
    });
    let n = str_ptr() - TOO_BIG_CHAR + 1;
    let limit = pool_ptr();
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(fmt_in, state.strings.str_start.as_mut_ptr(), n, 0, limit)
    }) {
        bad_format_file();
    }
    crate::state::with_state_mut(|state| {
        state.strings.str_pool = vec![0u16; (tre_pool_size as usize) + 1];
    });
    let limit = pool_ptr();
    crate::state::with_state_mut(|state| {
        undump_slice(fmt_in, state.strings.str_pool.as_mut_ptr(), limit);
    });

    crate::state::with_state_mut(|state| {
        state.strings.init_str_ptr = state.strings.str_ptr;
        state.strings.init_pool_ptr = state.strings.pool_ptr;
    });

    x = undump_int(fmt_in);
    if x < 1019 || x > MEM_TOP - HI_MEM_STAT_USAGE {
        bad_format_file();
    }
    set_lo_mem_max(x);

    x = undump_int(fmt_in);
    if x < 20 || x > lo_mem_max() {
        bad_format_file();
    }
    set_rover(x);

    for idx in INT_VAL..=INTER_CHAR_VAL {
        x = undump_int(fmt_in);
        if x < MIN_HALFWORD || x > lo_mem_max() {
            bad_format_file();
        }
        fmt_sa_root[idx as usize] = x;
    }

    p = 0;
    q = rover();
    loop {
        undump_slice(fmt_in, tre_mem.add(p as usize), q + 2 - p);
        p = q + mem_b32s0(q);
        if p > lo_mem_max() || (q >= mem_b32s1(q + 1) && mem_b32s1(q + 1) != rover()) {
            bad_format_file();
        }
        q = mem_b32s1(q + 1);
        if q == rover() {
            break;
        }
    }

    undump_slice(fmt_in, tre_mem.add(p as usize), lo_mem_max() + 1 - p);

    x = undump_int(fmt_in);
    if x < lo_mem_max() + 1 || x > PRE_ADJUST_HEAD {
        bad_format_file();
    }
    set_hi_mem_min(x);

    x = undump_int(fmt_in);
    if x < MIN_HALFWORD || x > MEM_TOP {
        bad_format_file();
    }
    set_avail(x);

    set_mem_end(MEM_TOP);
    undump_slice(
        fmt_in,
        tre_mem.add(hi_mem_min() as usize),
        mem_end() + 1 - hi_mem_min(),
    );
    set_var_used(undump_int(fmt_in));
    set_dyn_used(undump_int(fmt_in));

    k = ACTIVE_BASE;
    loop {
        x = undump_int(fmt_in);
        if x < 1 || k + x > EQTB_SIZE + 1 {
            bad_format_file();
        }

        undump_slice(fmt_in, tre_eqtb.add(k as usize), x);
        k += x;

        x = undump_int(fmt_in);
        if x < 0 || k + x > EQTB_SIZE + 1 {
            bad_format_file();
        }

        for j1 in k..=(k + x - 1) {
            *tre_eqtb.add(j1 as usize) = *tre_eqtb.add((k - 1) as usize);
        }
        k += x;
        if k > EQTB_SIZE {
            break;
        }
    }

    if hash_high() > 0 {
        undump_slice(fmt_in, tre_eqtb.add((EQTB_SIZE + 1) as usize), hash_high());
    }

    x = undump_int(fmt_in);
    if x < FORMAT_HASH_BASE || x > tre_hash_top {
        bad_format_file();
    }
    set_par_loc(x);
    set_par_token(CS_TOKEN_FLAG + par_loc());

    x = undump_int(fmt_in);
    if x < FORMAT_HASH_BASE || x > tre_hash_top {
        bad_format_file();
    }
    set_write_loc(x);

    for prim_idx in 0..=PRIM_SIZE {
        let value: crate::mem_types::B32x2 = undump_one(fmt_in);
        set_prim_s0(prim_idx, value.s0);
        set_prim_s1(prim_idx, value.s1);
    }

    x = undump_int(fmt_in);
    if x < FORMAT_HASH_BASE || x > FROZEN_CONTROL_SEQUENCE {
        bad_format_file();
    }
    set_hash_used(x);

    p = FORMAT_HASH_BASE - 1;
    loop {
        x = undump_int(fmt_in);
        if x < p + 1 || x > hash_used() {
            bad_format_file();
        }
        p = x;
        let value: crate::mem_types::B32x2 = undump_one(fmt_in);
        crate::globals::set_hash_s0(p, value.s0);
        crate::globals::set_hash_s1(p, value.s1);
        if p == hash_used() {
            break;
        }
    }

    undump_slice(
        fmt_in,
        tre_hash.add((hash_used() + 1) as usize),
        (UNDEFINED_CONTROL_SEQUENCE - 1) - hash_used(),
    );
    if hash_high() > 0 {
        undump_slice(fmt_in, tre_hash.add((EQTB_SIZE + 1) as usize), hash_high());
    }

    set_cs_count(undump_int(fmt_in));

    x = undump_int(fmt_in);
    if x < 7 {
        bad_format_file();
    }
    if x > SUP_FONT_MEM_SIZE {
        _tt_abort(c"must increase font_mem_size".as_ptr());
    }
    set_fmem_ptr(x);
    if fmem_ptr() > font_mem_size() {
        set_font_mem_size(fmem_ptr());
    }

    tre_font_info = xmalloc_array::<crate::mem_types::MemoryWord>(font_mem_size());
    undump_slice(fmt_in, tre_font_info, fmem_ptr());

    x = undump_int(fmt_in);
    if x < FONT_BASE {
        bad_format_file();
    }
    if x > FONT_BASE + MAX_FONT_MAX {
        _tt_abort(c"must increase font_max".as_ptr());
    }
    set_font_ptr(x);

    tre_font_mapping = xcalloc_array::<*mut libc::c_void>(font_max());
    tre_font_layout_engine = xcalloc_array::<*mut libc::c_void>(font_max());
    tre_font_flags = xmalloc_array::<libc::c_char>(font_max());
    tre_font_letter_space = xmalloc_array::<i32>(font_max());
    let font_check_len = (font_max() as usize) + 1;
    crate::state::with_state_mut(|state| {
        state.fonts.font_check = vec![
            crate::mem_types::B16x4 {
                s0: 0,
                s1: 0,
                s2: 0,
                s3: 0,
            };
            font_check_len
        ];
        state.fonts.font_size = vec![0i32; font_check_len];
        state.fonts.font_dsize = vec![0i32; font_check_len];
        state.fonts.font_params = vec![0i32; font_check_len];
        state.fonts.font_name = vec![0i32; font_check_len];
        state.fonts.font_area = vec![0i32; font_check_len];
        state.fonts.font_bc = vec![0u16; font_check_len];
        state.fonts.font_ec = vec![0u16; font_check_len];
        state.fonts.bchar_label = vec![0i32; font_check_len];
        state.fonts.font_bchar = vec![0i32; font_check_len];
        state.fonts.font_false_bchar = vec![0i32; font_check_len];
    });
    tre_font_glue = xmalloc_array::<i32>(font_max());
    tre_hyphen_char = xmalloc_array::<i32>(font_max());
    tre_skew_char = xmalloc_array::<i32>(font_max());
    tre_char_base = xmalloc_array::<i32>(font_max());
    tre_width_base = xmalloc_array::<i32>(font_max());
    tre_height_base = xmalloc_array::<i32>(font_max());
    tre_depth_base = xmalloc_array::<i32>(font_max());
    tre_italic_base = xmalloc_array::<i32>(font_max());
    tre_lig_kern_base = xmalloc_array::<i32>(font_max());
    tre_kern_base = xmalloc_array::<i32>(font_max());
    tre_exten_base = xmalloc_array::<i32>(font_max());
    tre_param_base = xmalloc_array::<i32>(font_max());

    for font_idx in FONT_BASE..=font_ptr() {
        *tre_font_mapping.add(font_idx as usize) = std::ptr::null_mut();
    }

    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.font_check.as_mut_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.font_size.as_mut_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
        undump_slice(
            fmt_in,
            state.fonts.font_dsize.as_mut_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(
            fmt_in,
            state.fonts.font_params.as_mut_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
            MIN_HALFWORD,
            MAX_HALFWORD,
        )
    }) {
        bad_format_file();
    }
    undump_slice(
        fmt_in,
        tre_hyphen_char.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    undump_slice(
        fmt_in,
        tre_skew_char.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    let font_count = font_ptr() + 1;
    let max_str = str_ptr();
    if !crate::state::with_state_mut(|state| {
        undump_upper_i32(
            fmt_in,
            state.fonts.font_name.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
            max_str,
        )
    }) {
        bad_format_file();
    }
    if !crate::state::with_state_mut(|state| {
        undump_upper_i32(
            fmt_in,
            state.fonts.font_area.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
            max_str,
        )
    }) {
        bad_format_file();
    }
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.font_bc.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
        );
    });
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.font_ec.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
        );
    });
    undump_slice(
        fmt_in,
        tre_char_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    undump_slice(
        fmt_in,
        tre_width_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    undump_slice(
        fmt_in,
        tre_height_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    undump_slice(
        fmt_in,
        tre_depth_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    undump_slice(
        fmt_in,
        tre_italic_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    undump_slice(
        fmt_in,
        tre_lig_kern_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    undump_slice(
        fmt_in,
        tre_kern_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    undump_slice(
        fmt_in,
        tre_exten_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    undump_slice(
        fmt_in,
        tre_param_base.add(FONT_BASE as usize),
        font_ptr() + 1,
    );
    if !undump_checked_i32(
        fmt_in,
        tre_font_glue.add(FONT_BASE as usize),
        font_ptr() + 1,
        MIN_HALFWORD,
        lo_mem_max(),
    ) {
        bad_format_file();
    }
    let max_font_index = fmem_ptr() - 1;
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(
            fmt_in,
            state.fonts.bchar_label.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
            0,
            max_font_index,
        )
    }) {
        bad_format_file();
    }
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(
            fmt_in,
            state.fonts.font_bchar.as_mut_ptr().add(FONT_BASE as usize),
            font_count,
            0,
            TOO_BIG_CHAR,
        )
    }) {
        bad_format_file();
    }
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(
            fmt_in,
            state
                .fonts
                .font_false_bchar
                .as_mut_ptr()
                .add(FONT_BASE as usize),
            font_count,
            0,
            TOO_BIG_CHAR,
        )
    }) {
        bad_format_file();
    }

    x = undump_int(fmt_in);
    if x < 0 {
        bad_format_file();
    }
    if x > hyph_size() {
        _tt_abort(c"must increase hyph_size".as_ptr());
    }
    set_hyph_count(x);

    x = undump_int(fmt_in);
    if x < FORMAT_HYPH_PRIME {
        bad_format_file();
    }
    if x > hyph_size() {
        _tt_abort(c"must increase hyph_size".as_ptr());
    }
    set_hyph_next(x);

    j = 0;
    for _ in 1..=hyph_count() {
        j = undump_int(fmt_in);
        if j < 0 {
            bad_format_file();
        }
        if j > 65_535 {
            set_hyph_next(j / 65_536);
            j -= hyph_next() * 65_536;
        } else {
            set_hyph_next(0);
        }

        if j >= hyph_size() || hyph_next() > hyph_size() {
            bad_format_file();
        }
        set_hyph_link_entry(j, hyph_next() as u16);

        x = undump_int(fmt_in);
        if x < 0 || x > str_ptr() {
            bad_format_file();
        }
        set_hyph_word_entry(j, x);

        x = undump_int(fmt_in);
        if x < MIN_HALFWORD || x > MAX_HALFWORD {
            bad_format_file();
        }
        set_hyph_list_entry(j, x);
    }

    j += 1;
    if j < FORMAT_HYPH_PRIME {
        j = FORMAT_HYPH_PRIME;
    }
    set_hyph_next(j);
    if hyph_next() >= hyph_size() {
        set_hyph_next(FORMAT_HYPH_PRIME);
    } else if hyph_next() >= FORMAT_HYPH_PRIME {
        set_hyph_next(hyph_next() + 1);
    }

    x = undump_int(fmt_in);
    if x < 0 {
        bad_format_file();
    }
    if x > trie_size() {
        _tt_abort(c"must increase trie_size".as_ptr());
    }
    j = x;
    set_trie_max(j);

    x = undump_int(fmt_in);
    if x < 0 || x > j {
        bad_format_file();
    }
    set_hyph_start(x);

    crate::state::with_state_mut(|state| {
        if state.hyph.trie_trl.is_empty() {
            state.hyph.trie_trl = vec![0i32; (j as usize) + 1];
        }
        undump_slice(fmt_in, state.hyph.trie_trl.as_mut_ptr(), j + 1);
        if state.hyph.trie_tro.is_empty() {
            state.hyph.trie_tro = vec![0i32; (j as usize) + 1];
        }
        undump_slice(fmt_in, state.hyph.trie_tro.as_mut_ptr(), j + 1);
        if state.hyph.trie_trc.is_empty() {
            state.hyph.trie_trc = vec![0u16; (j as usize) + 1];
        }
        undump_slice(fmt_in, state.hyph.trie_trc.as_mut_ptr(), j + 1);
    });

    set_max_hyph_char(undump_int(fmt_in));

    x = undump_int(fmt_in);
    if x < 0 {
        bad_format_file();
    }
    if x > TRIE_OP_SIZE {
        _tt_abort(c"must increase TRIE_OP_SIZE".as_ptr());
    }
    j = x;
    set_trie_op_ptr(j);

    let fmt_hyf_distance =
        crate::state::with_state_mut(|state| state.hyph.hyf_distance.as_mut_ptr());
    let fmt_hyf_num = crate::state::with_state_mut(|state| state.hyph.hyf_num.as_mut_ptr());
    let fmt_hyf_next = crate::state::with_state_mut(|state| state.hyph.hyf_next.as_mut_ptr());
    undump_slice(fmt_in, fmt_hyf_distance.add(1), j);
    undump_slice(fmt_in, fmt_hyf_num.add(1), j);
    if !undump_upper_u16(fmt_in, fmt_hyf_next.add(1), j, MAX_TRIE_OP) {
        bad_format_file();
    }

    for lang in 0..=BIGGEST_LANG as usize {
        set_trie_used_entry(lang, 0);
    }

    k = BIGGEST_LANG + 1;
    while j > 0 {
        x = undump_int(fmt_in);
        if x < 0 || x > k - 1 {
            bad_format_file();
        }
        k = x;

        x = undump_int(fmt_in);
        if x < 1 || x > j {
            bad_format_file();
        }
        set_trie_used_entry(k as usize, x as u16);
        j -= x;
        set_op_start_entry(k as usize, j);
    }

    set_trie_not_ready(false);

    x = undump_int(fmt_in);
    if x != FORMAT_FOOTER_MAGIC {
        bad_format_file();
    }

    ttstub_input_close(fmt_in);
    true
}

// translated from xetex-ini.c:1474 (tt_run_engine)
#[no_mangle]
pub unsafe extern "C" fn tt_run_engine(
    dump_name: *const libc::c_char,
    input_file_name: *const libc::c_char,
    build_date: libc::time_t,
) -> i32 {
    tre_TEX_format_default = xmalloc(libc::strlen(dump_name) + 1) as *mut libc::c_char;
    libc::strcpy(tre_TEX_format_default, dump_name);
    tre_format_default_length = libc::strlen(dump_name) as i32;
    crate::globals::set_rust_stdout(ttstub_output_open_stdout());

    if crate::globals::file_line_error_style_p() < 0 {
        set_file_line_error_style_p(0);
    }

    tre_pool_size = 6_250_000;
    tre_string_vacancies = 90_000;
    tre_pool_free = 47_500;
    tre_max_strings = 565_536;
    tre_strings_free = 100;
    set_font_mem_size(8_000_000);
    set_font_max(9_000);
    tre_trie_size = 1_000_000;
    tre_hyph_size = 8_191;
    tre_buf_size = 200_000;
    tre_nest_size = 500;
    tre_max_in_open = 15;
    tre_param_size = 10_000;
    tre_save_size = 80_000;
    tre_stack_size = 5_000;
    set_error_line(79);
    set_half_error_line(50);
    set_max_print_line(79);
    tre_hash_extra = 600_000;
    tre_expand_depth = 10_000;

    crate::state::with_state_mut(|state| {
        state.strings.pool_size = tre_pool_size;
        state.strings.string_vacancies = tre_string_vacancies;
        state.strings.max_strings = tre_max_strings;
        state.hyph.trie_size = tre_trie_size;
        state.hyph.hyph_size = tre_hyph_size;
    });

    {
        // Rust-owned input buffer (`buf_size + 1` words, matching the former
        // `xmalloc_array` shape). Allocated fresh each run; released by
        // `tt_cleanup`. Never reallocated during a run (overflow errors
        // instead of growing). Zero-init replaces the former explicit
        // `write_bytes(buffer, 0, buf_size)`.
        let n = (tre_buf_size as usize) + 1;
        crate::state::with_state_mut(|s| {
            s.input.buffer = vec![0i32; n];
        });
    }
    {
        // Rust-owned semantic-nest stack (`nest_size + 1` records, matching the
        // former `xmalloc_array` shape). Allocated fresh each run; released by
        // `tt_cleanup`. Never reallocated during a run.
        let n = (tre_nest_size as usize) + 1;
        crate::state::with_state_mut(|s| {
            s.input.nest = vec![
                crate::mem_types::ListStateRecord {
                    mode: 0,
                    head: 0,
                    tail: 0,
                    e_tex_aux: 0,
                    prev_graf: 0,
                    mode_line: 0,
                    aux: crate::mem_types::MemoryWord {
                        b32: crate::mem_types::B32x2 { s0: 0, s1: 0 },
                    },
                };
                n
            ];
        });
    }
    {
        // Rust-owned save-stack arena (`save_size + 1` words, matching the
        // former `xmalloc_array` shape). Allocated fresh each run; released by
        // `tt_cleanup`. Never reallocated during a run.
        let n = (tre_save_size as usize) + 1;
        crate::state::with_state_mut(|s| {
            s.save_stack.data = vec![
                crate::mem_types::MemoryWord {
                    b32: crate::mem_types::B32x2 { s0: 0, s1: 0 },
                };
                n
            ];
        });
    }
    {
        // Rust-owned main input-file stack (`max_in_open + 1` handles, matching
        // the former `xmalloc_array` shape). Allocated fresh each run; released
        // by `tt_cleanup`. Never reallocated during a run.
        let n = (tre_max_in_open as usize) + 1;
        crate::state::with_state_mut(|s| {
            s.input.input_file = vec![std::ptr::null_mut::<libc::c_void>(); n];
        });
    }
    {
        // Rust-owned input stack (`stack_size + 1` records, matching the former
        // `xmalloc_array` shape). Allocated fresh each run; released by
        // `tt_cleanup`. Never reallocated during a run.
        let n = (tre_stack_size as usize) + 1;
        crate::state::with_state_mut(|s| {
            s.input.input_stack = vec![
                crate::globals::InputState {
                    state: 0,
                    index: 0,
                    start: 0,
                    loc: 0,
                    limit: 0,
                    name: 0,
                    synctex_tag: 0,
                };
                n
            ];
        });
    }
    {
        // Rust-owned input line/group stacks, each `max_in_open + 1` words
        // (matching the former `xmalloc_array` shape). Allocated fresh each
        // run; released by `tt_cleanup`. Never reallocated during a run.
        let n = (tre_max_in_open as usize) + 1;
        crate::state::with_state_mut(|s| {
            s.input.line_stack = vec![0i32; n];
            s.input.grp_stack = vec![0i32; n];
            s.input.if_stack = vec![0i32; n];
            s.input.source_filename_stack = vec![0i32; n];
            s.input.full_source_filename_stack = vec![0i32; n];
            s.input.eof_seen = vec![false; n];
        });
    }
    {
        // Rust-owned macro-parameter stack (`param_size + 1` words, matching
        // the former `xmalloc_array` shape). Allocated fresh each run; released
        // by `tt_cleanup`. Never reallocated during a run.
        let n = (tre_param_size as usize) + 1;
        crate::state::with_state_mut(|s| {
            s.input.param_stack = vec![0i32; n];
        });
    }
    {
        // Rust-owned hyphenation-exception table (`hyph_size + 1` words,
        // matching the former `xmalloc_array` shape). Allocated fresh each
        // run; released by `tt_cleanup`. Serialized in format dump/undump
        // through the normal accessors.
        let n = (tre_hyph_size as usize) + 1;
        crate::state::with_state_mut(|s| {
            s.hyph.hyph_word = vec![0i32; n];
            s.hyph.hyph_list = vec![0i32; n];
            s.hyph.hyph_link = vec![0u16; n];
        });
    }

    if in_initex_mode() {
        tre_mem = xmalloc_array::<crate::mem_types::MemoryWord>(MEM_TOP + 1);
        tre_eqtb_top = EQTB_SIZE + tre_hash_extra;
        tre_hash_top = if tre_hash_extra == 0 {
            HASH_TOP_UNDEFINED_CONTROL_SEQUENCE
        } else {
            tre_eqtb_top
        };

        tt_run_engine_allocate_yhash();
        crate::globals::set_hash_s0(HASH_BASE, 0);
        crate::globals::set_hash_s1(HASH_BASE, 0);
        let hash_base = crate::globals::hash_s0(HASH_BASE);
        let hash_base_s1 = crate::globals::hash_s1(HASH_BASE);
        let mut h = HASH_BASE + 1;
        while h <= tre_hash_top {
            crate::globals::set_hash_s0(h, hash_base);
            crate::globals::set_hash_s1(h, hash_base_s1);
            h += 1;
        }
        crate::globals::set_hash_used(tre_hash_top);

        tre_eqtb = xcalloc_array::<crate::mem_types::MemoryWord>(tre_eqtb_top);
        crate::state::with_state_mut(|state| {
            state.strings.str_start = vec![0i32; (tre_max_strings as usize) + 1];
            state.strings.str_pool = vec![0u16; (tre_pool_size as usize) + 1];
        });
        tre_font_info = xmalloc_array::<crate::mem_types::MemoryWord>(font_mem_size());
    }

    crate::globals::set_history(HISTORY_FATAL_ERROR);
    tre_bad = 0;
    if crate::globals::half_error_line() < 30
        || crate::globals::half_error_line() > crate::globals::error_line() - 15
    {
        tre_bad = 1;
    }
    if crate::globals::max_print_line() < 60 {
        tre_bad = 2;
    }
    if 1100 > MEM_TOP {
        tre_bad = 4;
    }
    if HASH_PRIME > HASH_SIZE {
        tre_bad = 5;
    }
    if tre_max_in_open >= 128 {
        tre_bad = 6;
    }
    if MEM_TOP < 267 {
        tre_bad = 7;
    }
    if crate::mem::TEX_NULL > 0 {
        tre_bad = 12;
    }
    if MAX_FONT_MAX < crate::mem::TEX_NULL || MAX_FONT_MAX > MAX_HALFWORD {
        tre_bad = 15;
    }
    if font_max() > FONT_BASE + 9000 {
        tre_bad = 16;
    }
    if tre_save_size > MAX_HALFWORD || tre_max_strings > MAX_HALFWORD {
        tre_bad = 17;
    }
    if tre_buf_size > MAX_HALFWORD {
        tre_bad = 18;
    }
    if CS_TOKEN_FLAG + EQTB_SIZE + tre_hash_extra > MAX_HALFWORD {
        tre_bad = 21;
    }
    if FORMAT_HASH_OFFSET < 0 || FORMAT_HASH_OFFSET > HASH_BASE {
        tre_bad = 42;
    }
    if tre_format_default_length > i32::MAX {
        tre_bad = 31;
    }
    if 2 * MAX_HALFWORD < MEM_TOP {
        tre_bad = 41;
    }
    if tre_bad > 0 {
        let mut msg = [0 as libc::c_char; 80];
        libc::snprintf(
            msg.as_mut_ptr(),
            msg.len(),
            c"failed internal consistency check #%d".as_ptr(),
            tre_bad,
        );
        _tt_abort(msg.as_ptr());
    }

    initialize_more_variables();
    if in_initex_mode() {
        get_strings_started();
        initialize_more_initex_variables();
        initialize_primitives();
        crate::state::with_state_mut(|state| {
            state.strings.init_str_ptr = state.strings.str_ptr;
            state.strings.init_pool_ptr = state.strings.pool_ptr;
        });
    }

    crate::math::initialize_math_variables();
    crate::pagebuilder::initialize_pagebuilder_variables();
    crate::shipout::initialize_shipout_variables();
    let mut epoch = 0;
    let mut micros = 0;
    crate::texmfmp::get_seconds_and_micros(&mut epoch, &mut micros);
    crate::globals::set_epochseconds(epoch);
    crate::globals::set_microseconds(micros);
    crate::texmfmp::init_start_time(build_date);

    set_selector(SELECTOR_TERM_ONLY);
    crate::globals::set_tally(0);
    crate::globals::set_term_offset(0);
    crate::globals::set_file_offset(0);
    crate::globals::set_job_name(0);
    crate::globals::set_name_in_progress(false);
    crate::globals::set_log_opened(false);
    tre_output_file_extension = if crate::globals::semantic_pagination_enabled() {
        c".spx".as_ptr()
    } else {
        c".xdv".as_ptr()
    };

    crate::globals::set_input_ptr(0);
    crate::globals::set_max_in_stack(0);
    crate::globals::set_source_filename_stack(0, 0);
    crate::globals::set_full_source_filename_stack(0, 0);
    crate::globals::set_in_open(0);
    set_open_parens(0);
    crate::globals::set_max_buf_stack(0);
    crate::globals::set_grp_stack(0, 0);
    crate::globals::set_if_stack(0, crate::mem::TEX_NULL);
    crate::globals::set_param_ptr(0);
    crate::globals::set_max_param_stack(0);
    crate::globals::set_used_tectonic_coda_tokens(false);
    crate::globals::set_gave_char_warning_help(false);
    set_first(0);

    crate::globals::set_scanner_status(NORMAL as u8);
    crate::globals::set_warning_index(crate::mem::TEX_NULL);
    set_first(1);
    crate::globals::set_cur_input_state(NEW_LINE);
    crate::globals::set_cur_input_start(1);
    crate::globals::set_cur_input_index(0);
    crate::globals::set_line(0);
    crate::globals::set_cur_input_name(0);
    crate::globals::set_force_eof(false);
    crate::globals::set_align_state(1_000_000);

    init_io();

    if in_initex_mode() {
        crate::globals::set_no_new_control_sequence(false);
        set_max_reg_num(32767);
        set_max_reg_help_line(c"A register number must be between 0 and 32767.".as_ptr());
    }
    crate::globals::set_no_new_control_sequence(true);

    if !in_initex_mode() && !load_fmt_file() {
        return history();
    }

    if intpar(INT_PAR_END_LINE_CHAR) < 0 || intpar(INT_PAR_END_LINE_CHAR) > BIGGEST_CHAR {
        crate::globals::set_cur_input_limit(crate::globals::cur_input().limit - 1);
    } else {
        set_buffer_entry(
            crate::globals::cur_input().limit,
            intpar(INT_PAR_END_LINE_CHAR),
        );
    }

    if in_initex_mode() {
        set_eqtb_b32s1(PC_INT_BASE + INT_PAR_TIME, 0);
        set_eqtb_b32s1(PC_INT_BASE + INT_PAR_DAY, 0);
        set_eqtb_b32s1(PC_INT_BASE + INT_PAR_MONTH, 0);
        set_eqtb_b32s1(PC_INT_BASE + INT_PAR_YEAR, 0);
    } else {
        let mut time = 0;
        let mut day = 0;
        let mut month = 0;
        let mut year = 0;
        crate::texmfmp::get_date_and_time(build_date, &mut time, &mut day, &mut month, &mut year);
        set_eqtb_b32s1(PC_INT_BASE + INT_PAR_TIME, time);
        set_eqtb_b32s1(PC_INT_BASE + INT_PAR_DAY, day);
        set_eqtb_b32s1(PC_INT_BASE + INT_PAR_MONTH, month);
        set_eqtb_b32s1(PC_INT_BASE + INT_PAR_YEAR, year);
    }

    if trie_not_ready() {
        crate::state::with_state_mut(|state| {
            let trie_len = (state.hyph.trie_size as usize) + 1;
            state.hyph.trie_trl = vec![0i32; trie_len];
            state.hyph.trie_tro = vec![0i32; trie_len];
            state.hyph.trie_trc = vec![0u16; trie_len];
            state.hyph.trie_c = vec![0u16; trie_len];
            state.hyph.trie_o = vec![0u16; trie_len];
            state.hyph.trie_l = vec![0i32; trie_len];
            state.hyph.trie_r = vec![0i32; trie_len];
            state.hyph.trie_hash = vec![0i32; trie_len];
            state.hyph.trie_taken = vec![false; trie_len];
        });
        set_trie_l_entry(0, 0);
        set_trie_c_entry(0, 0);
        set_trie_ptr(0);
        set_trie_r_entry(0, 0);
        set_hyph_start(0);

        tre_font_mapping = xcalloc_array::<*mut libc::c_void>(font_max());
        tre_font_layout_engine = xcalloc_array::<*mut libc::c_void>(font_max());
        tre_font_flags = xcalloc_array::<libc::c_char>(font_max());
        tre_font_letter_space = xcalloc_array::<i32>(font_max());
        let font_check_len = (font_max() as usize) + 1;
        crate::state::with_state_mut(|state| {
            state.fonts.font_check = vec![
                crate::mem_types::B16x4 {
                    s0: 0,
                    s1: 0,
                    s2: 0,
                    s3: 0,
                };
                font_check_len
            ];
            state.fonts.font_size = vec![0i32; font_check_len];
            state.fonts.font_dsize = vec![0i32; font_check_len];
            state.fonts.font_params = vec![0i32; font_check_len];
            state.fonts.font_name = vec![0i32; font_check_len];
            state.fonts.font_area = vec![0i32; font_check_len];
            state.fonts.font_bc = vec![0u16; font_check_len];
            state.fonts.font_ec = vec![0u16; font_check_len];
            state.fonts.bchar_label = vec![0i32; font_check_len];
            state.fonts.font_bchar = vec![0i32; font_check_len];
            state.fonts.font_false_bchar = vec![0i32; font_check_len];
        });
        tre_font_glue = xcalloc_array::<i32>(font_max());
        tre_hyphen_char = xcalloc_array::<i32>(font_max());
        tre_skew_char = xcalloc_array::<i32>(font_max());
        tre_char_base = xcalloc_array::<i32>(font_max());
        tre_width_base = xcalloc_array::<i32>(font_max());
        tre_height_base = xcalloc_array::<i32>(font_max());
        tre_depth_base = xcalloc_array::<i32>(font_max());
        tre_italic_base = xcalloc_array::<i32>(font_max());
        tre_lig_kern_base = xcalloc_array::<i32>(font_max());
        tre_kern_base = xcalloc_array::<i32>(font_max());
        tre_exten_base = xcalloc_array::<i32>(font_max());
        tre_param_base = xcalloc_array::<i32>(font_max());

        crate::globals::set_font_ptr(FONT_BASE);
        crate::globals::set_fmem_ptr(7);
        crate::globals::set_font_name_entry(
            FONT_BASE,
            crate::texmfmp::maketexstring(c"nullfont".as_ptr()),
        );
        crate::globals::set_font_area_entry(FONT_BASE, EMPTY_STRING);
        set_hyphen_char_entry(FONT_BASE, b'-' as i32);
        set_skew_char_entry(FONT_BASE, -1);
        crate::globals::set_bchar_label_entry(FONT_BASE, NON_ADDRESS);
        crate::globals::set_font_bchar_entry(FONT_BASE, TOO_BIG_CHAR);
        crate::globals::set_font_false_bchar_entry(FONT_BASE, TOO_BIG_CHAR);
        crate::globals::set_font_bc_entry(FONT_BASE, 1);
        crate::globals::set_font_ec_entry(FONT_BASE, 0);
        crate::globals::set_font_size_entry(FONT_BASE, 0);
        crate::globals::set_font_dsize_entry(FONT_BASE, 0);
        crate::globals::set_char_base_entry(FONT_BASE, 0);
        crate::globals::set_width_base_entry(FONT_BASE, 0);
        crate::globals::set_height_base_entry(FONT_BASE, 0);
        crate::globals::set_depth_base_entry(FONT_BASE, 0);
        crate::globals::set_italic_base_entry(FONT_BASE, 0);
        crate::globals::set_lig_kern_base_entry(FONT_BASE, 0);
        crate::globals::set_kern_base_entry(FONT_BASE, 0);
        crate::globals::set_exten_base_entry(FONT_BASE, 0);
        crate::globals::set_font_glue(FONT_BASE, crate::mem::TEX_NULL);
        crate::globals::set_font_params(FONT_BASE, 7);
        crate::globals::set_font_mapping_ptr(FONT_BASE, std::ptr::null_mut());
        crate::globals::set_param_base(FONT_BASE, -1);

        for font_k in 0..=6 {
            set_font_info_b32s1(font_k, 0);
        }
    }

    tre_font_used = xmalloc_array::<bool>(font_max());
    for font_k in 0..=font_max() {
        *tre_font_used.add(font_k as usize) = false;
    }

    crate::globals::set_random_seed(
        crate::globals::microseconds() * 1000 + (crate::globals::epochseconds() % 1_000_000),
    );
    crate::scaled_math::init_randoms(crate::globals::random_seed());

    if interaction() == BATCH_MODE {
        set_selector(SELECTOR_NO_PRINT);
    } else {
        set_selector(SELECTOR_TERM_ONLY);
    }

    if crate::globals::semantic_pagination_enabled() {
        set_eqtb_b32s1(PC_INT_BASE + INT_PAR_XETEX_GENERATE_ACTUAL_TEXT, 1);
    }

    pdf_files_init();
    crate::synctex::synctex_init_command();
    crate::scan::start_input(input_file_name);
    crate::globals::set_history(HISTORY_SPOTLESS);
    crate::scan::main_control();
    final_cleanup();
    crate::scan::close_files_and_terminate();
    tt_cleanup();

    history()
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
    #[link_name = "native_text"]
    static mut tt_cleanup_native_text: *mut libc::c_void;
    #[link_name = "tt_cleanup_free_yhash"]
    fn tt_cleanup_yhash();
    #[link_name = "eqtb"]
    static mut tt_cleanup_eqtb: *mut libc::c_void;
    #[link_name = "mem"]
    static mut tt_cleanup_mem: *mut libc::c_void;

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
    #[link_name = "font_glue"]
    static mut tt_cleanup_font_glue: *mut libc::c_void;
    #[link_name = "hyphen_char"]
    static mut tt_cleanup_hyphen_char: *mut libc::c_void;
    #[link_name = "skew_char"]
    static mut tt_cleanup_skew_char: *mut libc::c_void;
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

    // Release the Rust-owned save-stack arena (was a C `malloc`'d array).
    crate::state::with_state_mut(|s| {
        s.save_stack.data = Vec::new();
    });

    // Release the Rust-owned nest, input-file, input, macro-parameter, input
    // line/group, and buffer arrays (were C `malloc`'d arrays).
    crate::state::with_state_mut(|s| {
        s.input.nest = Vec::new();
        s.input.input_file = Vec::new();
        s.input.input_stack = Vec::new();
        s.input.param_stack = Vec::new();
        s.input.line_stack = Vec::new();
        s.input.grp_stack = Vec::new();
        s.input.if_stack = Vec::new();
        s.input.source_filename_stack = Vec::new();
        s.input.full_source_filename_stack = Vec::new();
        s.input.eof_seen = Vec::new();
        s.input.buffer = Vec::new();
    });
    crate::state::with_state_mut(|s| {
        s.hyph.hyph_word = Vec::new();
        s.hyph.hyph_list = Vec::new();
        s.hyph.hyph_link = Vec::new();
    });

    // initialize_more_variables @ 3277
    libc::free(tt_cleanup_native_text);

    // Free arrays allocated in load_fmt_file
    tt_cleanup_yhash(); // frees file-static yhash via C helper
    libc::free(tt_cleanup_eqtb);
    libc::free(tt_cleanup_mem);
    crate::state::with_state_mut(|state| {
        state.strings.str_start = Vec::new();
        state.strings.str_pool = Vec::new();
        state.fonts.font_check = Vec::new();
        state.fonts.font_size = Vec::new();
        state.fonts.font_dsize = Vec::new();
        state.fonts.font_params = Vec::new();
        state.fonts.font_name = Vec::new();
        state.fonts.font_area = Vec::new();
        state.fonts.font_bc = Vec::new();
        state.fonts.font_ec = Vec::new();
        state.fonts.bchar_label = Vec::new();
        state.fonts.font_bchar = Vec::new();
        state.fonts.font_false_bchar = Vec::new();
    });
    libc::free(tt_cleanup_font_info);

    libc::free(tt_cleanup_font_mapping);
    libc::free(tt_cleanup_font_layout_engine);
    libc::free(tt_cleanup_font_flags);
    libc::free(tt_cleanup_font_letter_space);
    libc::free(tt_cleanup_font_glue);
    libc::free(tt_cleanup_hyphen_char);
    libc::free(tt_cleanup_skew_char);
    libc::free(tt_cleanup_char_base);
    libc::free(tt_cleanup_width_base);
    libc::free(tt_cleanup_height_base);
    libc::free(tt_cleanup_depth_base);
    libc::free(tt_cleanup_italic_base);
    libc::free(tt_cleanup_lig_kern_base);
    libc::free(tt_cleanup_kern_base);
    libc::free(tt_cleanup_exten_base);
    libc::free(tt_cleanup_param_base);

    // Release Rust-owned hyphenation trie arrays (were C `malloc`'d arrays).
    crate::state::with_state_mut(|state| {
        state.hyph.trie_trl = Vec::new();
        state.hyph.trie_tro = Vec::new();
        state.hyph.trie_trc = Vec::new();
        state.hyph.trie_c = Vec::new();
        state.hyph.trie_o = Vec::new();
        state.hyph.trie_l = Vec::new();
        state.hyph.trie_r = Vec::new();
        state.hyph.trie_hash = Vec::new();
        state.hyph.trie_taken = Vec::new();
    });
}

// Command/eqtb constants used by `prefixed_command` (xetex-constants.h).
const PREFIX: i32 = 95;
const MAX_NON_PREFIXED_COMMAND: i32 = 71;
const SPACER: i32 = 10;
const RELAX_CMD: i32 = 0;
const RIGHT_BRACE: i32 = 2;
const LEFT_BRACE: i32 = 1;
const TOKS_REGISTER: i32 = 72;
const ASSIGN_TOKS: i32 = 73;
const ASSIGN_INT: i32 = 74;
const ASSIGN_DIMEN: i32 = 75;
const ASSIGN_GLUE: i32 = 76;
const ASSIGN_MU_GLUE: i32 = 77;
const ASSIGN_FONT_DIMEN: i32 = 78;
const ASSIGN_FONT_INT: i32 = 79;
const SET_AUX: i32 = 80;
const SET_PREV_GRAF: i32 = 81;
const SET_PAGE_DIMEN: i32 = 82;
const SET_PAGE_INT: i32 = 83;
const SET_BOX_DIMEN: i32 = 84;
const SET_SHAPE: i32 = 85;
const DEF_CODE: i32 = 86;
const XETEX_DEF_CODE: i32 = 87;
const DEF_FAMILY: i32 = 88;
const SET_FONT: i32 = 89;
const DEF_FONT: i32 = 90;
const REGISTER: i32 = 91;
const ADVANCE: i32 = 92;
const MULTIPLY: i32 = 93;
const DIVIDE: i32 = 94;
const LET: i32 = 96;
const SHORTHAND_DEF: i32 = 97;
const READ_TO_CS: i32 = 98;
const DEF: i32 = 99;
const SET_BOX: i32 = 100;
const HYPH_DATA: i32 = 101;
const SET_INTERACTION: i32 = 102;
const UNDEFINED_CS: i32 = 103;
const CALL: i32 = 113;
const GLUE_REF: i32 = 119;
const SHAPE_REF: i32 = 120;
const DATA: i32 = 122;
const CHAR_GIVEN: i32 = 68;
const MATH_GIVEN: i32 = 69;
const XETEX_MATH_GIVEN: i32 = 70;
const NORMAL: i32 = 0;

const CHAR_DEF_CODE: i32 = 0;
const MATH_CHAR_DEF_CODE: i32 = 1;
const COUNT_DEF_CODE: i32 = 2;
const DIMEN_DEF_CODE: i32 = 3;
const SKIP_DEF_CODE: i32 = 4;
const MU_SKIP_DEF_CODE: i32 = 5;
const TOKS_DEF_CODE: i32 = 6;
const CHAR_SUB_DEF_CODE: i32 = 7;
const XETEX_MATH_CHAR_NUM_DEF_CODE: i32 = 8;
const XETEX_MATH_CHAR_DEF_CODE: i32 = 9;

const GLUE_VAL: i32 = 2;
const MU_VAL: i32 = 3;
const TOK_VAL: i32 = 5;
const INTER_CHAR_VAL: i32 = 6;

const PROTECTED_TOKEN: i32 = 0x1C0_0000 + 1; // END_MATCH_TOKEN + 1
const LEFT_BRACE_TOKEN: i32 = 0x20_0000;
const RIGHT_BRACE_TOKEN: i32 = 0x40_0000;
const OTHER_TOKEN: i32 = 0x180_0000;
const TEX_NULL: i32 = -0x0FFF_FFFF; // MIN_HALFWORD

const CUR_FONT_LOC: i32 = 2_255_400;
const COUNT_BASE: i32 = 7_826_812;
const SCALED_BASE: i32 = 8_941_203;
const SKIP_BASE: i32 = 2_254_359;
const MU_SKIP_BASE: i32 = 2_254_615;
const PC_LOCAL_BASE: i32 = 2_254_871;
const TOKS_BASE: i32 = 2_254_884;
const CAT_CODE_BASE: i32 = 2_256_169;
const SF_CODE_BASE: i32 = 5_598_505;
const MATH_CODE_BASE: i32 = 6_712_617;
const PC_INT_BASE: i32 = 7_826_729;
const DEL_CODE_BASE: i32 = 7_827_068;
const LOCAL__PAR_SHAPE: i32 = 0;
const LOCAL__OUTPUT_ROUTINE: i32 = 1;
const LOCAL__XETEX_INTER_CHAR_TOKS: i32 = 11;
const INT_PAR__TRACING_COMMANDS: i32 = 36;
const INT_PAR__GLOBAL_DEFS: i32 = 43;
const CHAR_CLASS_LIMIT: i32 = 4096;

const TOO_BIG_USV: i32 = 1_114_112;
const ACTIVE_MATH_CHAR: i32 = 0x1F_FFFF;
const MAX_CHAR_CODE: i32 = 15;
const BIGGEST_USV: i32 = 0x10_FFFF;
const BOX_FLAG: i32 = 0x4000_0000;
const GLOBAL_BOX_FLAG: i32 = 0x4000_8000;
const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 0xFFFE;
const LP_CODE_BASE: i32 = 2;
const RP_CODE_BASE: i32 = 3;
const PC_LEFT_SIDE: i32 = 0;
const PC_RIGHT_SIDE: i32 = 1;

#[inline]
unsafe fn intpar(n: i32) -> i32 {
    eqtb_b32s1(PC_INT_BASE + n)
}

// translated from xetex-ini.c:583 (prefixed_command)
#[no_mangle]
pub unsafe extern "C" fn prefixed_command() {
    fn set_class(x: i32) -> i32 {
        (((x as u32) & 0x07) << 21) as i32
    }
    fn set_family(x: i32) -> i32 {
        (((x as u32) & 0xFF) << 24) as i32
    }

    let mut a: i32 = 0;
    let mut f: i32;
    let mut j: i32;
    let mut k: i32;
    let mut p: i32 = 0;
    let mut q: i32;
    let mut n: i32;
    let mut e: bool;

    while cur_cmd() as i32 == PREFIX {
        if (a / cur_chr()) & 1 == 0 {
            a += cur_chr();
        }

        loop {
            crate::scan::get_x_token();
            if !(cur_cmd() as i32 == SPACER || cur_cmd() as i32 == RELAX_CMD) {
                break;
            }
        }

        if (cur_cmd() as i32) <= MAX_NON_PREFIXED_COMMAND {
            crate::output::error_here_with_diagnostic(c"You can't use a prefix with `".as_ptr());
            crate::output::print_cmd_chr(cur_cmd() as u16, cur_chr());
            crate::output::print_char('\'' as i32);
            crate::output::capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(1);
            set_help_line(
                0,
                c"I'll pretend you didn't say \\long or \\outer or \\global or \\protected."
                    .as_ptr(),
            );
            crate::input::back_error();
            return;
        }

        if intpar(INT_PAR__TRACING_COMMANDS) > 2 {
            crate::output::show_cur_cmd_chr();
        }
    }

    if a >= 8 {
        j = PROTECTED_TOKEN;
        a -= 8;
    } else {
        j = 0;
    }

    if cur_cmd() as i32 != DEF && (a % 4 != 0 || j != 0) {
        crate::output::error_here_with_diagnostic(c"You can't use `".as_ptr());
        crate::output::print_esc_cstr(c"long".as_ptr());
        crate::output::print_cstr(c"' or `".as_ptr());
        crate::output::print_esc_cstr(c"outer".as_ptr());
        crate::output::print_cstr(c"' or `".as_ptr());
        crate::output::print_esc_cstr(c"protected".as_ptr());
        crate::output::print_cstr(c"' with `".as_ptr());
        crate::output::print_cmd_chr(cur_cmd() as u16, cur_chr());
        crate::output::print_char('\'' as i32);
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(1);
        set_help_line(
            0,
            c"I'll pretend you didn't say \\long or \\outer or \\protected here.".as_ptr(),
        );
        crate::errors::error();
    }

    if intpar(INT_PAR__GLOBAL_DEFS) != 0 {
        if intpar(INT_PAR__GLOBAL_DEFS) < 0 {
            if a >= 4 {
                a -= 4;
            }
        } else if a < 4 {
            a += 4;
        }
    }

    'sw: {
        match cur_cmd() as i32 {
            SET_FONT => {
                if a >= 4 {
                    crate::save::geq_define(CUR_FONT_LOC, DATA as u16, cur_chr());
                } else {
                    crate::save::eq_define(CUR_FONT_LOC, DATA as u16, cur_chr());
                }
            }

            DEF => {
                if cur_chr() & 1 != 0 && a < 4 && intpar(INT_PAR__GLOBAL_DEFS) >= 0 {
                    a += 4;
                }

                e = cur_chr() >= 2;
                crate::scan::get_r_token();
                p = cur_cs();
                q = crate::scan::scan_toks(true, e);

                if j != 0 {
                    q = crate::mem::get_avail();
                    set_mem_b32s0(q, j);
                    set_mem_b32s1(q, mem_b32s1(def_ref()));
                    set_mem_b32s1(def_ref(), q);
                }

                if a >= 4 {
                    crate::save::geq_define(p, (CALL + (a % 4)) as u16, def_ref());
                } else {
                    crate::save::eq_define(p, (CALL + (a % 4)) as u16, def_ref());
                }
            }

            LET => {
                n = cur_chr();
                crate::scan::get_r_token();
                p = cur_cs();

                if n == NORMAL {
                    loop {
                        crate::scan::get_token();
                        if cur_cmd() as i32 != SPACER {
                            break;
                        }
                    }

                    if cur_tok() == OTHER_TOKEN + '=' as i32 {
                        crate::scan::get_token();
                        if cur_cmd() as i32 == SPACER {
                            crate::scan::get_token();
                        }
                    }
                } else {
                    crate::scan::get_token();
                    q = cur_tok();
                    crate::scan::get_token();
                    crate::input::back_input();
                    set_cur_tok(q);
                    crate::input::back_input();
                }

                if cur_cmd() as i32 >= CALL {
                    set_mem_b32s0(cur_chr(), mem_b32s0(cur_chr()) + 1);
                } else if cur_cmd() as i32 == REGISTER || cur_cmd() as i32 == TOKS_REGISTER {
                    if cur_chr() < 0 || cur_chr() > 19 {
                        set_mem_b32s0(cur_chr() + 1, mem_b32s0(cur_chr() + 1) + 1);
                    }
                }

                if a >= 4 {
                    crate::save::geq_define(p, cur_cmd() as u16, cur_chr());
                } else {
                    crate::save::eq_define(p, cur_cmd() as u16, cur_chr());
                }
            }

            SHORTHAND_DEF => {
                if cur_chr() == CHAR_SUB_DEF_CODE {
                    crate::output::error_here_with_diagnostic(
                        c"vestigial MLTeX shorthand encountered??".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(1);
                    set_help_line(
                        0,
                        c"This should never happen in Tectonic, where MLTeX has been excised."
                            .as_ptr(),
                    );
                    crate::errors::error();
                } else {
                    n = cur_chr();
                    crate::scan::get_r_token();
                    p = cur_cs();

                    if a >= 4 {
                        crate::save::geq_define(p, RELAX_CMD as u16, TOO_BIG_USV);
                    } else {
                        crate::save::eq_define(p, RELAX_CMD as u16, TOO_BIG_USV);
                    }

                    crate::scan::scan_optional_equals();

                    match n {
                        CHAR_DEF_CODE => {
                            crate::scan::scan_usv_num();
                            if a >= 4 {
                                crate::save::geq_define(p, CHAR_GIVEN as u16, cur_val());
                            } else {
                                crate::save::eq_define(p, CHAR_GIVEN as u16, cur_val());
                            }
                        }
                        MATH_CHAR_DEF_CODE => {
                            crate::scan::scan_fifteen_bit_int();
                            if a >= 4 {
                                crate::save::geq_define(p, MATH_GIVEN as u16, cur_val());
                            } else {
                                crate::save::eq_define(p, MATH_GIVEN as u16, cur_val());
                            }
                        }
                        XETEX_MATH_CHAR_NUM_DEF_CODE => {
                            crate::scan::scan_xetex_math_char_int();
                            if a >= 4 {
                                crate::save::geq_define(p, XETEX_MATH_GIVEN as u16, cur_val());
                            } else {
                                crate::save::eq_define(p, XETEX_MATH_GIVEN as u16, cur_val());
                            }
                        }
                        XETEX_MATH_CHAR_DEF_CODE => {
                            crate::scan::scan_math_class_int();
                            n = set_class(cur_val());
                            crate::scan::scan_math_fam_int();
                            n = n.wrapping_add(set_family(cur_val()));
                            crate::scan::scan_usv_num();
                            n = n.wrapping_add(cur_val());
                            if a >= 4 {
                                crate::save::geq_define(p, XETEX_MATH_GIVEN as u16, n);
                            } else {
                                crate::save::eq_define(p, XETEX_MATH_GIVEN as u16, n);
                            }
                        }
                        _ => {
                            crate::scan::scan_register_num();

                            if cur_val() > 255 {
                                j = n - 2;

                                if j > MU_VAL {
                                    j = TOK_VAL;
                                }

                                crate::save::find_sa_element(j as u16, cur_val(), true);
                                set_mem_b32s0(cur_ptr() + 1, mem_b32s0(cur_ptr() + 1) + 1);

                                if j == TOK_VAL {
                                    j = TOKS_REGISTER;
                                } else {
                                    j = REGISTER;
                                }

                                if a >= 4 {
                                    crate::save::geq_define(p, j as u16, cur_ptr());
                                } else {
                                    crate::save::eq_define(p, j as u16, cur_ptr());
                                }
                            } else {
                                match n {
                                    COUNT_DEF_CODE => {
                                        if a >= 4 {
                                            crate::save::geq_define(
                                                p,
                                                ASSIGN_INT as u16,
                                                COUNT_BASE + cur_val(),
                                            );
                                        } else {
                                            crate::save::eq_define(
                                                p,
                                                ASSIGN_INT as u16,
                                                COUNT_BASE + cur_val(),
                                            );
                                        }
                                    }
                                    DIMEN_DEF_CODE => {
                                        if a >= 4 {
                                            crate::save::geq_define(
                                                p,
                                                ASSIGN_DIMEN as u16,
                                                SCALED_BASE + cur_val(),
                                            );
                                        } else {
                                            crate::save::eq_define(
                                                p,
                                                ASSIGN_DIMEN as u16,
                                                SCALED_BASE + cur_val(),
                                            );
                                        }
                                    }
                                    SKIP_DEF_CODE => {
                                        if a >= 4 {
                                            crate::save::geq_define(
                                                p,
                                                ASSIGN_GLUE as u16,
                                                SKIP_BASE + cur_val(),
                                            );
                                        } else {
                                            crate::save::eq_define(
                                                p,
                                                ASSIGN_GLUE as u16,
                                                SKIP_BASE + cur_val(),
                                            );
                                        }
                                    }
                                    MU_SKIP_DEF_CODE => {
                                        if a >= 4 {
                                            crate::save::geq_define(
                                                p,
                                                ASSIGN_MU_GLUE as u16,
                                                MU_SKIP_BASE + cur_val(),
                                            );
                                        } else {
                                            crate::save::eq_define(
                                                p,
                                                ASSIGN_MU_GLUE as u16,
                                                MU_SKIP_BASE + cur_val(),
                                            );
                                        }
                                    }
                                    TOKS_DEF_CODE => {
                                        if a >= 4 {
                                            crate::save::geq_define(
                                                p,
                                                ASSIGN_TOKS as u16,
                                                TOKS_BASE + cur_val(),
                                            );
                                        } else {
                                            crate::save::eq_define(
                                                p,
                                                ASSIGN_TOKS as u16,
                                                TOKS_BASE + cur_val(),
                                            );
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            READ_TO_CS => {
                j = cur_chr();
                crate::scan::scan_int();
                n = cur_val();
                if !crate::scan::scan_keyword(c"to".as_ptr()) {
                    crate::output::error_here_with_diagnostic(c"Missing `to' inserted".as_ptr());
                    crate::output::capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(2);
                    set_help_line(
                        1,
                        c"You should have said `\\read<number> to \\cs'.".as_ptr(),
                    );
                    set_help_line(0, c"I'm going to look for the \\cs now.".as_ptr());
                    crate::errors::error();
                }

                crate::scan::get_r_token();
                p = cur_cs();
                crate::scan::read_toks(n, p, j);

                if a >= 4 {
                    crate::save::geq_define(p, CALL as u16, cur_val());
                } else {
                    crate::save::eq_define(p, CALL as u16, cur_val());
                }
            }

            TOKS_REGISTER | ASSIGN_TOKS => {
                q = cur_cs();
                e = false;

                if cur_cmd() as i32 == TOKS_REGISTER {
                    if cur_chr() == 0 {
                        crate::scan::scan_register_num();
                        if cur_val() > 255 {
                            crate::save::find_sa_element(TOK_VAL as u16, cur_val(), true);
                            set_cur_chr(cur_ptr());
                            e = true;
                        } else {
                            set_cur_chr(TOKS_BASE + cur_val());
                        }
                    } else {
                        e = true;
                    }
                } else if cur_chr() == PC_LOCAL_BASE + LOCAL__XETEX_INTER_CHAR_TOKS {
                    crate::scan::scan_char_class_not_ignored();
                    set_cur_ptr(cur_val());
                    crate::scan::scan_char_class_not_ignored();
                    crate::save::find_sa_element(
                        INTER_CHAR_VAL as u16,
                        cur_ptr() * CHAR_CLASS_LIMIT + cur_val(),
                        true,
                    );
                    set_cur_chr(cur_ptr());
                    e = true;
                }

                p = cur_chr();
                crate::scan::scan_optional_equals();

                loop {
                    crate::scan::get_x_token();
                    if !(cur_cmd() as i32 == SPACER || cur_cmd() as i32 == RELAX_CMD) {
                        break;
                    }
                }

                if cur_cmd() as i32 != LEFT_BRACE
                    && (cur_cmd() as i32 == TOKS_REGISTER || cur_cmd() as i32 == ASSIGN_TOKS)
                {
                    if cur_cmd() as i32 == TOKS_REGISTER {
                        if cur_chr() == 0 {
                            crate::scan::scan_register_num();
                            if cur_val() < 256 {
                                q = eqtb_b32s1(TOKS_BASE + cur_val());
                            } else {
                                crate::save::find_sa_element(TOK_VAL as u16, cur_val(), false);
                                if cur_ptr() == TEX_NULL {
                                    q = TEX_NULL;
                                } else {
                                    q = mem_b32s1(cur_ptr() + 1);
                                }
                            }
                        } else {
                            q = mem_b32s1(cur_chr() + 1);
                        }
                    } else if cur_chr() == PC_LOCAL_BASE + LOCAL__XETEX_INTER_CHAR_TOKS {
                        crate::scan::scan_char_class_not_ignored();
                        set_cur_ptr(cur_val());
                        crate::scan::scan_char_class_not_ignored();
                        crate::save::find_sa_element(
                            INTER_CHAR_VAL as u16,
                            cur_ptr() * CHAR_CLASS_LIMIT + cur_val(),
                            false,
                        );
                        if cur_ptr() == TEX_NULL {
                            q = TEX_NULL;
                        } else {
                            q = mem_b32s1(cur_ptr() + 1);
                        }
                    } else {
                        q = eqtb_b32s1(cur_chr());
                    }

                    if q == TEX_NULL {
                        if e {
                            if a >= 4 {
                                crate::save::gsa_def(p, TEX_NULL);
                            } else {
                                crate::save::sa_def(p, TEX_NULL);
                            }
                        } else if a >= 4 {
                            crate::save::geq_define(p, UNDEFINED_CS as u16, TEX_NULL);
                        } else {
                            crate::save::eq_define(p, UNDEFINED_CS as u16, TEX_NULL);
                        }
                    } else {
                        set_mem_b32s0(q, mem_b32s0(q) + 1);
                        if e {
                            if a >= 4 {
                                crate::save::gsa_def(p, q);
                            } else {
                                crate::save::sa_def(p, q);
                            }
                        } else if a >= 4 {
                            crate::save::geq_define(p, CALL as u16, q);
                        } else {
                            crate::save::eq_define(p, CALL as u16, q);
                        }
                    }

                    break 'sw;
                }

                crate::input::back_input();
                set_cur_cs(q);
                q = crate::scan::scan_toks(false, false);

                if mem_b32s1(def_ref()) == TEX_NULL {
                    if e {
                        if a >= 4 {
                            crate::save::gsa_def(p, TEX_NULL);
                        } else {
                            crate::save::sa_def(p, TEX_NULL);
                        }
                    } else if a >= 4 {
                        crate::save::geq_define(p, UNDEFINED_CS as u16, TEX_NULL);
                    } else {
                        crate::save::eq_define(p, UNDEFINED_CS as u16, TEX_NULL);
                    }

                    set_mem_b32s1(def_ref(), avail());
                    set_avail(def_ref());
                } else {
                    if p == PC_LOCAL_BASE + LOCAL__OUTPUT_ROUTINE && !e {
                        set_mem_b32s1(q, crate::mem::get_avail());
                        q = mem_b32s1(q);
                        set_mem_b32s0(q, RIGHT_BRACE_TOKEN + 125);
                        q = crate::mem::get_avail();
                        set_mem_b32s0(q, LEFT_BRACE_TOKEN + 123);
                        set_mem_b32s1(q, mem_b32s1(def_ref()));
                        set_mem_b32s1(def_ref(), q);
                    }

                    if e {
                        if a >= 4 {
                            crate::save::gsa_def(p, def_ref());
                        } else {
                            crate::save::sa_def(p, def_ref());
                        }
                    } else if a >= 4 {
                        crate::save::geq_define(p, CALL as u16, def_ref());
                    } else {
                        crate::save::eq_define(p, CALL as u16, def_ref());
                    }
                }
            }

            ASSIGN_INT => {
                p = cur_chr();
                crate::scan::scan_optional_equals();
                crate::scan::scan_int();
                if a >= 4 {
                    crate::save::geq_word_define(p, cur_val());
                } else {
                    crate::save::eq_word_define(p, cur_val());
                }
            }

            ASSIGN_DIMEN => {
                p = cur_chr();
                crate::scan::scan_optional_equals();
                crate::scan::scan_dimen(false, false, false);
                if a >= 4 {
                    crate::save::geq_word_define(p, cur_val());
                } else {
                    crate::save::eq_word_define(p, cur_val());
                }
            }

            ASSIGN_GLUE | ASSIGN_MU_GLUE => {
                p = cur_chr();
                n = cur_cmd() as i32;
                crate::scan::scan_optional_equals();
                if n == ASSIGN_MU_GLUE {
                    crate::scan::scan_glue(MU_VAL as i16);
                } else {
                    crate::scan::scan_glue(GLUE_VAL as i16);
                }
                crate::scan::trap_zero_glue();
                if a >= 4 {
                    crate::save::geq_define(p, GLUE_REF as u16, cur_val());
                } else {
                    crate::save::eq_define(p, GLUE_REF as u16, cur_val());
                }
            }

            XETEX_DEF_CODE => {
                if cur_chr() == SF_CODE_BASE {
                    p = cur_chr();
                    crate::scan::scan_usv_num();
                    p += cur_val();
                    n = eqtb_b32s1(SF_CODE_BASE + cur_val()) % 65536;
                    crate::scan::scan_optional_equals();
                    crate::scan::scan_char_class();
                    let v = cur_val().wrapping_mul(65536).wrapping_add(n);
                    if a >= 4 {
                        crate::save::geq_define(p, DATA as u16, v);
                    } else {
                        crate::save::eq_define(p, DATA as u16, v);
                    }
                } else if cur_chr() == MATH_CODE_BASE {
                    p = cur_chr();
                    crate::scan::scan_usv_num();
                    p += cur_val();
                    crate::scan::scan_optional_equals();
                    crate::scan::scan_xetex_math_char_int();
                    if a >= 4 {
                        crate::save::geq_define(p, DATA as u16, cur_val());
                    } else {
                        crate::save::eq_define(p, DATA as u16, cur_val());
                    }
                } else if cur_chr() == MATH_CODE_BASE + 1 {
                    p = cur_chr() - 1;
                    crate::scan::scan_usv_num();
                    p += cur_val();
                    crate::scan::scan_optional_equals();
                    crate::scan::scan_math_class_int();
                    n = set_class(cur_val());
                    crate::scan::scan_math_fam_int();
                    n = n.wrapping_add(set_family(cur_val()));
                    crate::scan::scan_usv_num();
                    n = n.wrapping_add(cur_val());
                    if a >= 4 {
                        crate::save::geq_define(p, DATA as u16, n);
                    } else {
                        crate::save::eq_define(p, DATA as u16, n);
                    }
                } else if cur_chr() == DEL_CODE_BASE {
                    p = cur_chr();
                    crate::scan::scan_usv_num();
                    p += cur_val();
                    crate::scan::scan_optional_equals();
                    crate::scan::scan_int();
                    if a >= 4 {
                        crate::save::geq_word_define(p, cur_val());
                    } else {
                        crate::save::eq_word_define(p, cur_val());
                    }
                } else {
                    p = cur_chr() - 1;
                    crate::scan::scan_usv_num();
                    p += cur_val();
                    crate::scan::scan_optional_equals();
                    n = 0x4000_0000; // "extended delimiter code flag"
                    crate::scan::scan_math_fam_int();
                    n = n.wrapping_add(cur_val().wrapping_mul(0x20_0000)); // extended delimiter family
                    crate::scan::scan_usv_num();
                    n = n.wrapping_add(cur_val());
                    if a >= 4 {
                        crate::save::geq_word_define(p, n);
                    } else {
                        crate::save::eq_word_define(p, n);
                    }
                }
            }

            DEF_CODE => {
                if cur_chr() == CAT_CODE_BASE {
                    n = MAX_CHAR_CODE;
                } else if cur_chr() == MATH_CODE_BASE {
                    n = 0x8000;
                } else if cur_chr() == SF_CODE_BASE {
                    n = 0x7FFF;
                } else if cur_chr() == DEL_CODE_BASE {
                    n = 0xFFFFFF;
                } else {
                    n = BIGGEST_USV;
                }

                p = cur_chr();
                crate::scan::scan_usv_num();
                p += cur_val();
                crate::scan::scan_optional_equals();
                crate::scan::scan_int();

                if (cur_val() < 0 && p < DEL_CODE_BASE) || cur_val() > n {
                    crate::output::error_here_with_diagnostic(c"Invalid code (".as_ptr());
                    crate::output::print_int(cur_val());
                    if p < DEL_CODE_BASE {
                        crate::output::print_cstr(c"), should be in the range 0..".as_ptr());
                    } else {
                        crate::output::print_cstr(c"), should be at most ".as_ptr());
                    }
                    crate::output::print_int(n);
                    crate::output::capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(1);
                    set_help_line(
                        0,
                        c"I'm going to use 0 instead of that illegal code value.".as_ptr(),
                    );
                    crate::errors::error();
                    set_cur_val(0);
                }

                if p < MATH_CODE_BASE {
                    if p >= SF_CODE_BASE {
                        n = eqtb_b32s1(p) / 65536;
                        let v = n.wrapping_mul(65536).wrapping_add(cur_val());
                        if a >= 4 {
                            crate::save::geq_define(p, DATA as u16, v);
                        } else {
                            crate::save::eq_define(p, DATA as u16, v);
                        }
                    } else if a >= 4 {
                        crate::save::geq_define(p, DATA as u16, cur_val());
                    } else {
                        crate::save::eq_define(p, DATA as u16, cur_val());
                    }
                } else if p < DEL_CODE_BASE {
                    if cur_val() == 32768 {
                        set_cur_val(ACTIVE_MATH_CHAR);
                    } else {
                        let v = set_class(cur_val() / 4096)
                            .wrapping_add(set_family((cur_val() % 4096) / 256))
                            .wrapping_add(cur_val() % 256);
                        set_cur_val(v);
                    }

                    if a >= 4 {
                        crate::save::geq_define(p, DATA as u16, cur_val());
                    } else {
                        crate::save::eq_define(p, DATA as u16, cur_val());
                    }
                } else if a >= 4 {
                    crate::save::geq_word_define(p, cur_val());
                } else {
                    crate::save::eq_word_define(p, cur_val());
                }
            }

            DEF_FAMILY => {
                p = cur_chr();
                crate::scan::scan_math_fam_int();
                p += cur_val();
                crate::scan::scan_optional_equals();
                crate::scan::scan_font_ident();
                if a >= 4 {
                    crate::save::geq_define(p, DATA as u16, cur_val());
                } else {
                    crate::save::eq_define(p, DATA as u16, cur_val());
                }
            }

            REGISTER | ADVANCE | MULTIPLY | DIVIDE => {
                crate::scan::do_register_command(a as i16);
            }

            SET_BOX => {
                crate::scan::scan_register_num();
                if a >= 4 {
                    n = GLOBAL_BOX_FLAG + cur_val();
                } else {
                    n = BOX_FLAG + cur_val();
                }

                crate::scan::scan_optional_equals();

                if set_box_allowed() {
                    crate::scan::scan_box(n);
                } else {
                    crate::output::error_here_with_diagnostic(c"Improper ".as_ptr());
                    crate::output::print_esc_cstr(c"setbox".as_ptr());
                    crate::output::capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(2);
                    set_help_line(
                        1,
                        c"Sorry, \\setbox is not allowed after \\halign in a display,".as_ptr(),
                    );
                    set_help_line(
                        0,
                        c"or between \\accent and an accented character.".as_ptr(),
                    );
                    crate::errors::error();
                }
            }

            SET_AUX => {
                crate::scan::alter_aux();
            }

            SET_PREV_GRAF => {
                crate::scan::alter_prev_graf();
            }

            SET_PAGE_DIMEN => {
                crate::scan::alter_page_so_far();
            }

            SET_PAGE_INT => {
                crate::scan::alter_integer();
            }

            SET_BOX_DIMEN => {
                crate::scan::alter_box_dimen();
            }

            SET_SHAPE => {
                q = cur_chr();
                crate::scan::scan_optional_equals();
                crate::scan::scan_int();
                n = cur_val();

                if n <= 0 {
                    p = TEX_NULL;
                } else if q > PC_LOCAL_BASE + LOCAL__PAR_SHAPE {
                    n = (cur_val() / 2) + 1;
                    p = crate::mem::get_node(2 * n + 1);
                    set_mem_b32s0(p, n);
                    n = cur_val();
                    set_mem_b32s1(p + 1, n);

                    j = p + 2;
                    while j <= p + n + 1 {
                        crate::scan::scan_int();
                        set_mem_b32s1(j, cur_val());
                        j += 1;
                    }

                    if n & 1 == 0 {
                        set_mem_b32s1(p + n + 2, 0);
                    }
                } else {
                    p = crate::mem::get_node(2 * n + 1);
                    set_mem_b32s0(p, n);

                    j = 1;
                    while j <= n {
                        crate::scan::scan_dimen(false, false, false);
                        set_mem_b32s1(p + 2 * j - 1, cur_val());
                        crate::scan::scan_dimen(false, false, false);
                        set_mem_b32s1(p + 2 * j, cur_val());
                        j += 1;
                    }
                }

                if a >= 4 {
                    crate::save::geq_define(q, SHAPE_REF as u16, p);
                } else {
                    crate::save::eq_define(q, SHAPE_REF as u16, p);
                }
            }

            HYPH_DATA => {
                if cur_chr() == 1 {
                    if in_initex_mode() {
                        new_patterns();
                        break 'sw;
                    }

                    crate::output::error_here_with_diagnostic(
                        c"Patterns can be loaded only by INITEX".as_ptr(),
                    );
                    crate::output::capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(0);
                    crate::errors::error();

                    loop {
                        crate::scan::get_token();
                        if cur_cmd() as i32 == RIGHT_BRACE {
                            break;
                        }
                    }

                    return;
                } else {
                    new_hyph_exceptions();
                    break 'sw;
                }
            }

            ASSIGN_FONT_DIMEN => {
                crate::scan::find_font_dimen(true);
                k = cur_val();
                crate::scan::scan_optional_equals();
                crate::scan::scan_dimen(false, false, false);
                set_font_info_b32s1(k, cur_val());
            }

            ASSIGN_FONT_INT => {
                n = cur_chr();
                crate::scan::scan_font_ident();
                f = cur_val();

                if n < 2 {
                    crate::scan::scan_optional_equals();
                    crate::scan::scan_int();
                    if n == 0 {
                        set_hyphen_char_entry(f, cur_val());
                    } else {
                        set_skew_char_entry(f, cur_val());
                    }
                } else {
                    if font_area_entry(f) == AAT_FONT_FLAG || font_area_entry(f) == OTGR_FONT_FLAG {
                        crate::scan::scan_glyph_number(f);
                    } else {
                        crate::scan::scan_char_num();
                    }

                    p = cur_val();
                    crate::scan::scan_optional_equals();
                    crate::scan::scan_int();
                    match n {
                        LP_CODE_BASE => set_cp_code(f, p as u32, PC_LEFT_SIDE, cur_val()),
                        RP_CODE_BASE => set_cp_code(f, p as u32, PC_RIGHT_SIDE, cur_val()),
                        _ => {}
                    }
                }
            }

            DEF_FONT => {
                crate::scan::new_font(a as i16);
            }

            SET_INTERACTION => {
                crate::scan::new_interaction();
            }

            _ => {
                crate::errors::confusion(c"prefix".as_ptr());
            }
        }
    }

    // done:
    if after_token() != 0 {
        set_cur_tok(after_token());
        crate::input::back_input();
        set_after_token(0);
    }
}
