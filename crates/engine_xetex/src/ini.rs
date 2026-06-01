//! Rust translation of initialization helpers from xetex-ini.c.

use crate::consts::{
    ACTIVE_BASE, BATCH_MODE, BIGGEST_CHAR, BOX_BASE, CAT_CODE_BASE, CS_TOKEN_FLAG, CUR_FONT_LOC,
    DEL_CODE_BASE, DIMEN_BASE, EQTB_SIZE, ERROR_STOP_MODE, ETEX_PEN_BASE, GLUE_BASE, GLUE_NODE,
    INT_BASE, LC_CODE_BASE, LOCAL_BASE, MATH_CODE_BASE, MATH_FONT_BASE, MAX_HALFWORD, MIN_HALFWORD,
    NUMBER_USVS, PRIM_EQTB_BASE, SCROLL_MODE, SELECTOR_TERM_AND_LOG, SELECTOR_TERM_ONLY,
    SF_CODE_BASE, SINGLE_BASE, TOKS_BASE, TOO_BIG_CHAR, UC_CODE_BASE,
};
use crate::globals::{
    avail, buf_size, cond_ptr, cs_count, cur_chr, cur_if, cur_input, cur_level, cur_mark, cur_val,
    disc_ptr, dyn_used, eqtb_b16s0, eqtb_b16s1, eqtb_b32s1, eqtb_top, eqtb_word, first, fmem_ptr,
    font_area_entry, font_dsize_entry, font_layout_engine_ptr, font_mapping_ptr, font_max,
    font_mem_size, font_name_entry, font_ptr, font_size_entry, format_ident, hash_high, hash_used,
    hi_mem_min, history, hyph_count, hyph_link_entry, hyph_list_entry, hyph_next, hyph_size,
    hyph_start, hyph_word_entry, if_line, in_initex_mode, in_open, init_pool_ptr, input_file,
    interaction, job_name, last, last_glue, lo_mem_max, max_hyph_char, mem_b16s0, mem_b32s0,
    mem_b32s1, mem_end, open_parens, par_loc, pool_ptr, pool_size, rover, sa_root, selector,
    set_adjust_tail, set_after_token, set_align_ptr, set_avail, set_buffer_entry,
    set_cancel_boundary, set_cond_ptr, set_cs_count, set_cur_align, set_cur_boundary, set_cur_dir,
    set_cur_f, set_cur_group, set_cur_head, set_cur_if, set_cur_input_limit, set_cur_input_loc,
    set_cur_level, set_cur_list_aux_b32s1, set_cur_list_e_tex_aux, set_cur_list_head,
    set_cur_list_mode, set_cur_list_mode_line, set_cur_list_prev_graf, set_cur_list_tail,
    set_cur_loop, set_cur_mark, set_cur_order, set_cur_pre_head, set_cur_pre_tail, set_cur_span,
    set_cur_tail_a, set_cur_val, set_cur_val_level, set_dead_cycles, set_deletions_allowed,
    set_disc_ptr, set_doing_leaders, set_doing_special, set_dyn_used, set_edit_name_start,
    set_empty_b32s0, set_empty_b32s1, set_eqtb_b16s0, set_eqtb_b16s1, set_eqtb_b32s1,
    set_eqtb_word, set_error_count, set_error_line, set_expand_depth_count,
    set_file_line_error_style_p, set_first, set_fmem_ptr, set_font_info_b32s1, set_font_max,
    set_font_mem_size, set_font_ptr, set_format_ident, set_half_error_line, set_hash_high,
    set_hash_s1, set_hash_used, set_help_line, set_help_ptr, set_hi_mem_min, set_hyph_count,
    set_hyph_link_entry, set_hyph_list_entry, set_hyph_next, set_hyph_start, set_hyph_word_entry,
    set_hyphen_char_entry, set_if_limit, set_if_line, set_input_file, set_ins_disc,
    set_insert_penalties, set_interaction, set_is_in_csname, set_last, set_last_badness,
    set_last_bop, set_last_glue, set_last_kern, set_last_node_type, set_last_penalty, set_lft_hit,
    set_ligature_present, set_lo_mem_max, set_long_help_seen, set_lr_problems, set_lr_ptr,
    set_mag_set, set_max_h, set_max_hyph_char, set_max_nest_stack, set_max_op_used,
    set_max_print_line, set_max_push, set_max_reg_help_line, set_max_reg_num, set_max_save_stack,
    set_max_v, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1, set_mem_end,
    set_mem_word, set_native_text, set_native_text_size, set_nest_ptr, set_no_new_control_sequence,
    set_null_character, set_op_start_entry, set_open_parens, set_output_active,
    set_pack_begin_line, set_page_contents, set_page_so_far, set_page_tail, set_par_loc,
    set_par_token, set_pool_ptr, set_pre_adjust_tail, set_prim_s0, set_prim_s1, set_prim_used,
    set_pseudo_files, set_radix, set_read_open_entry, set_rover, set_rt_hit, set_sa_chain,
    set_sa_level, set_sa_null_b32s0, set_sa_null_b32s1, set_sa_root, set_save_ptr, set_selector,
    set_set_box_allowed, set_shown_mode, set_skew_char_entry, set_spec_log, set_stop_at_space,
    set_str_ptr, set_str_start, set_temp_ptr, set_total_pages, set_trie_c_entry, set_trie_l_entry,
    set_trie_max, set_trie_not_ready, set_trie_op_ptr, set_trie_ptr, set_trie_r_entry,
    set_trie_used_entry, set_two_to_the, set_use_err_help, set_var_used, set_write_loc,
    set_write_open, set_xeq_level, str_ptr, str_start, string_vacancies, temp_ptr, trie_max,
    trie_not_ready, trie_op_ptr, trie_size, trie_used_entry, two_to_the, var_used, write_loc,
};
use crate::io::UFile;

const INVALID_HANDLE: usize = 0;
const UTF8: i16 = 1;
const LEVEL_ONE: u16 = 1;
const GET_NODE_HEAD_SENTINEL: i32 = 0x4000_0000;

// The trie/hyphenation constants and `_trie_op_hash_array` now live in
// `hyphenation.rs`; the format-dump code below still references three of them.
use crate::hyphenation::{init_trie, set_trie_op_hash, BIGGEST_LANG, MAX_TRIE_OP, TRIE_OP_SIZE};

extern "C" {
    fn _tt_abort(fmt: *const libc::c_char) -> !;
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

// translated from xetex-ini.c:554 (sort_avail)
#[no_mangle]
pub unsafe extern "C" fn sort_avail() {
    let _ = crate::mem::get_node_global(GET_NODE_HEAD_SENTINEL);
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

    let mut c = cur_chr();

    if c != 1 {
        set_eqtb_b32s1(INT_BASE + INT_PAR_NEW_LINE_CHAR, -1);
    }

    if job_name() == 0 {
        crate::io::open_log_file();
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

    if cur_level() > LEVEL_ONE {
        crate::output::print_nl(b'(' as i32);
        crate::output::print_esc_cstr(c"end occurred ".as_ptr());
        crate::output::print_cstr(c"inside a group at level ".as_ptr());
        crate::output::print_int(cur_level() as i32 - 1);
        crate::output::print_char(b')' as i32);
        crate::output::show_save_groups();
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
        crate::mem::free_node_global(temp_ptr(), IF_NODE_SIZE);
    }

    if history() != HISTORY_SPOTLESS
        && (history() == HISTORY_WARNING_ISSUED || interaction() < ERROR_STOP_MODE)
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
                    crate::mem::delete_token_ref_global(cur_mark(c as usize));
                }
                if c as usize >= SPLIT_BOT_MARK_CODE {
                    break;
                }
                c += 1;
            }

            if sa_root(MARK_VAL) != crate::mem::TEX_NULL
                && crate::packaging::do_marks_global(3, 0, sa_root(MARK_VAL))
            {
                set_sa_root(MARK_VAL, crate::mem::TEX_NULL);
            }

            c = LAST_BOX_CODE as i32;
            loop {
                crate::packaging::flush_node_list_global(disc_ptr(c as usize));
                if c as usize >= VSPLIT_CODE {
                    break;
                }
                c += 1;
            }

            if last_glue() != MAX_HALFWORD {
                crate::mem::delete_glue_ref_global(last_glue());
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
    const VMODE: i16 = 1;
    const MEM_TOP: i32 = 4_999_999;
    const CONTRIB_HEAD: i32 = MEM_TOP - 1;
    const PAGE_HEAD: i32 = MEM_TOP - 2;
    const IGNORE_DEPTH: i32 = -65_536_000;
    const EMPTY: u8 = 0;
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
    const HI_MEM_STAT_USAGE: i32 = 15;
    const FROZEN_CONTROL_SEQUENCE: i32 = 2_243_226;
    const FROZEN_END_TEMPLATE: i32 = FROZEN_CONTROL_SEQUENCE + 5;
    const UNDEFINED_CS: u16 = 103;
    const UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
    const LEVEL_ZERO: u16 = 0;
    const LOCAL_PAR_SHAPE: i32 = 0;
    const LOCAL_OUTPUT_ROUTINE: i32 = 1;
    const NUM_ETEX_PENALTIES: i32 = 4;
    const NUMBER_REGS: i32 = 256;
    const FONT_BASE: i32 = 0;
    const NUMBER_MATH_FAMILIES: i32 = 256;
    const NUMBER_MATH_FONTS: i32 = 3 * NUMBER_MATH_FAMILIES;
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
        (x & 0xFF) << 24
    }
    fn set_class(x: i32) -> i32 {
        (x & 0x07) << 21
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
const HASH_PRIME: i32 = 557_111;
const HASH_SIZE: i32 = 2_242_711;
const HASH_BASE: i32 = 1_114_114;
const HASH_TOP_UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
const FONT_BASE: i32 = 0;
const MAX_FONT_MAX: i32 = 9_000;
const CONTRIB_HEAD: i32 = MEM_TOP - 1;
const PAGE_HEAD: i32 = MEM_TOP - 2;
const PRE_ADJUST_HEAD: i32 = MEM_TOP - 14;
const HI_MEM_STAT_USAGE: i32 = 15;
const SELECTOR_NO_PRINT: i32 = 16;
const HISTORY_FATAL_ERROR: i32 = 3;
const HISTORY_SPOTLESS: i32 = 0;
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
const FORMAT_INT_BASE: i32 = 7_826_729;
const SELECTOR_LOG_ONLY: i32 = 18;
const SELECTOR_NEW_STRING: i32 = 21;
const TTBC_FILE_FORMAT_FORMAT: libc::c_int = 10;
const SUP_MAX_STRINGS: i32 = 2_097_151;
const SUP_FONT_MEM_SIZE: i32 = 147_483_647;
const SUP_POOL_SIZE: i32 = 40_000_000;
const SUP_HASH_EXTRA: i32 = 2_097_151;

static mut TRE_POOL_SIZE: i32 = 0;
static mut TRE_STRING_VACANCIES: i32 = 0;
static mut TRE_MAX_STRINGS: i32 = 0;
static mut TRE_TRIE_SIZE: i32 = 0;
static mut TRE_HYPH_SIZE: i32 = 0;

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
    #[link_name = "expand_depth"]
    static mut tre_expand_depth: i32;
    // `mem` is now Rust-owned (`EngineState.mem.data`); no FFI alias. (session 58)
    // `eqtb` is now Rust-owned (`EngineState.eqtb.data`); no FFI alias. (session 51)
    // `hash` is now Rust-owned (`EngineState.eqtb.hash`); no FFI alias. (session 52)

    #[link_name = "bad"]
    static mut tre_bad: i32;

    #[link_name = "output_file_extension"]
    static mut tre_output_file_extension: *const libc::c_char;
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
        crate::io::close_files_and_terminate();
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
    crate::io::pack_job_name(c".fmt".as_ptr());

    let fmt_out = ttstub_output_open(crate::globals::name_of_file(), 0);
    if fmt_out == INVALID_HANDLE {
        _tt_abort(c"cannot open format output file".as_ptr());
    }

    crate::output::print_nl_cstr(c"Beginning to dump on file ".as_ptr());
    crate::output::print(crate::io::make_name_string());

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
        let val = crate::state::with_state(|state| state.scanner.sa_root[k]);
        dump_int(fmt_out, val);
    }

    // `mem` is now a Rust-owned `Vec`; take a stable base pointer once (the
    // thread-local owner never reallocates) so the free-list `dump_slice`
    // calls below — interleaved with `mem_*`/`var_used` accessor reads — hold
    // no borrow. (session 58, the session-51 eqtb pattern.)
    let mem_ptr = crate::state::with_state(|s| s.mem.data.as_ptr());
    let mut p = 0;
    let mut q = rover();
    let mut x = 0;
    loop {
        let len = q + 2 - p;
        dump_slice(fmt_out, mem_ptr.add(p as usize), len);
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
    dump_slice(fmt_out, mem_ptr.add(p as usize), lo_mem_max() + 1 - p);
    x += lo_mem_max() + 1 - p;

    dump_int(fmt_out, hi_mem_min());
    dump_int(fmt_out, avail());
    dump_slice(
        fmt_out,
        mem_ptr.add(hi_mem_min() as usize),
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

    // `eqtb` is now a Rust-owned `Vec`; take a stable base pointer once (the
    // thread-local owner never reallocates) so the sparse-region `dump_slice`
    // calls below — interleaved with `eqtb_*` accessor reads — hold no borrow.
    let eqtb_ptr = crate::state::with_state(|s| s.eqtb.data.as_ptr());
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
        dump_slice(fmt_out, eqtb_ptr.add(k as usize), l - k);
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
        dump_slice(fmt_out, eqtb_ptr.add(k as usize), l - k);
        k = j + 1;
        dump_int(fmt_out, k - l);
        if k > EQTB_SIZE {
            break;
        }
    }

    if hash_high() > 0 {
        dump_slice(fmt_out, eqtb_ptr.add((EQTB_SIZE + 1) as usize), hash_high());
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

    let fmp = fmem_ptr();
    dump_int(fmt_out, fmp);
    crate::state::with_state(|s| {
        dump_slice(fmt_out, s.fonts.font_info.as_ptr(), fmp);
    });
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
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.hyphen_char.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
        dump_slice(
            fmt_out,
            state.fonts.skew_char.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
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
    let font_dump_count = font_ptr() + 1;
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.char_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.width_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.height_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.depth_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.italic_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.lig_kern_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.kern_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.exten_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
        dump_slice(
            fmt_out,
            state.fonts.param_base.as_ptr().add(FONT_BASE as usize),
            font_dump_count,
        );
    });
    crate::state::with_state(|state| {
        dump_slice(
            fmt_out,
            state.fonts.font_glue.as_ptr().add(FONT_BASE as usize),
            font_ptr() + 1,
        );
    });
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
    crate::globals::set_name_of_file(tre_TEX_format_default as *mut libc::c_char);
    crate::globals::set_name_length(libc::strlen(tre_TEX_format_default) as i32);
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
        crate::state::with_state_mut(|state| {
            state.fonts.font_info = Vec::new();
            state.strings.str_pool = Vec::new();
            state.strings.str_start = Vec::new();
            state.eqtb.data = Vec::new();
            state.eqtb.hash = Vec::new();
            state.mem.data = Vec::new();
        });
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
    if !(0..=SUP_HASH_EXTRA).contains(&x) {
        bad_format_file();
    }
    set_hash_high(x);
    let hash_high_val = hash_high();
    crate::state::with_state_mut(|s| {
        if s.eqtb.hash_extra < hash_high_val {
            s.eqtb.hash_extra = hash_high_val;
        }
        s.eqtb.eqtb_top = EQTB_SIZE + s.eqtb.hash_extra;
        s.eqtb.hash_top = if s.eqtb.hash_extra == 0 {
            UNDEFINED_CONTROL_SEQUENCE
        } else {
            s.eqtb.eqtb_top
        };
        let n = (1 + s.eqtb.hash_top - 514) as usize;
        s.eqtb.hash = vec![crate::mem_types::B32x2 { s0: 0, s1: 0 }; n];
    });

    crate::globals::set_hash_s0(FORMAT_HASH_BASE, 0);
    crate::globals::set_hash_s1(FORMAT_HASH_BASE, 0);
    for h in (FORMAT_HASH_BASE + 1)..=crate::globals::hash_top() {
        crate::globals::set_hash_s0(h, crate::globals::hash_s0(FORMAT_HASH_BASE));
        crate::globals::set_hash_s1(h, crate::globals::hash_s1(FORMAT_HASH_BASE));
    }

    // `xmalloc_array(n)` allocated `n + 1` words; match that inclusive size.
    let eqtb_words_load = (crate::globals::eqtb_top() as usize) + 1;
    crate::state::with_state_mut(|s| {
        s.eqtb.data = vec![
            crate::mem_types::MemoryWord {
                b32: crate::mem_types::B32x2 { s0: 0, s1: 0 }
            };
            eqtb_words_load
        ];
    });
    set_eqtb_b16s1(UNDEFINED_CONTROL_SEQUENCE, UNDEFINED_CS as u16);
    set_eqtb_b32s1(UNDEFINED_CONTROL_SEQUENCE, crate::mem::TEX_NULL);
    set_eqtb_b16s0(UNDEFINED_CONTROL_SEQUENCE, 0);
    let undefined_entry = eqtb_word(UNDEFINED_CONTROL_SEQUENCE);
    for e in (EQTB_SIZE + 1)..=crate::globals::eqtb_top() {
        set_eqtb_word(e, undefined_entry);
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
    // `xmalloc_array(MEM_TOP)` allocated `MEM_TOP + 1` words; match it.
    let mem_words_load = (MEM_TOP as usize) + 1;
    crate::state::with_state_mut(|s| {
        s.mem.data = vec![
            crate::mem_types::MemoryWord {
                b32: crate::mem_types::B32x2 { s0: 0, s1: 0 }
            };
            mem_words_load
        ];
    });

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
    if TRE_POOL_SIZE < pool_ptr() + tre_pool_free {
        TRE_POOL_SIZE = pool_ptr() + tre_pool_free;
    }
    crate::state::with_state_mut(|state| {
        state.strings.pool_size = TRE_POOL_SIZE;
    });

    x = undump_int(fmt_in);
    if x < 0 {
        bad_format_file();
    }
    if x > SUP_MAX_STRINGS - tre_strings_free {
        _tt_abort(c"must increase sup_strings".as_ptr());
    }
    set_str_ptr(x);
    if TRE_MAX_STRINGS < str_ptr() + tre_strings_free {
        TRE_MAX_STRINGS = str_ptr() + tre_strings_free;
    }
    crate::state::with_state_mut(|state| {
        state.strings.max_strings = TRE_MAX_STRINGS;
    });

    crate::state::with_state_mut(|state| {
        state.strings.str_start = vec![0i32; (TRE_MAX_STRINGS as usize) + 1];
    });
    let n = str_ptr() - TOO_BIG_CHAR + 1;
    let limit = pool_ptr();
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(fmt_in, state.strings.str_start.as_mut_ptr(), n, 0, limit)
    }) {
        bad_format_file();
    }
    crate::state::with_state_mut(|state| {
        state.strings.str_pool = vec![0u16; (TRE_POOL_SIZE as usize) + 1];
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
    if !(1019..=MEM_TOP - HI_MEM_STAT_USAGE).contains(&x) {
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
        crate::state::with_state_mut(|state| state.scanner.sa_root[idx as usize] = x);
    }

    // `mem` is now a Rust-owned `Vec`; take a stable mutable base pointer once
    // (never reallocated) for the free-list `undump_slice` calls — interleaved
    // with `mem_*` accessor reads, so it holds no borrow. (session 58)
    let mem_ptr = crate::state::with_state_mut(|s| s.mem.data.as_mut_ptr());
    p = 0;
    q = rover();
    loop {
        undump_slice(fmt_in, mem_ptr.add(p as usize), q + 2 - p);
        p = q + mem_b32s0(q);
        if p > lo_mem_max() || (q >= mem_b32s1(q + 1) && mem_b32s1(q + 1) != rover()) {
            bad_format_file();
        }
        q = mem_b32s1(q + 1);
        if q == rover() {
            break;
        }
    }

    undump_slice(fmt_in, mem_ptr.add(p as usize), lo_mem_max() + 1 - p);

    x = undump_int(fmt_in);
    if x < lo_mem_max() + 1 || x > PRE_ADJUST_HEAD {
        bad_format_file();
    }
    set_hi_mem_min(x);

    x = undump_int(fmt_in);
    if !(MIN_HALFWORD..=MEM_TOP).contains(&x) {
        bad_format_file();
    }
    set_avail(x);

    set_mem_end(MEM_TOP);
    undump_slice(
        fmt_in,
        mem_ptr.add(hi_mem_min() as usize),
        mem_end() + 1 - hi_mem_min(),
    );
    set_var_used(undump_int(fmt_in));
    set_dyn_used(undump_int(fmt_in));

    // `eqtb` is now a Rust-owned `Vec`; take a stable mutable base pointer once
    // (never reallocated) for the sparse-region `undump_slice`/replication below.
    let eqtb_ptr = crate::state::with_state_mut(|s| s.eqtb.data.as_mut_ptr());
    k = ACTIVE_BASE;
    loop {
        x = undump_int(fmt_in);
        if x < 1 || k + x > EQTB_SIZE + 1 {
            bad_format_file();
        }

        undump_slice(fmt_in, eqtb_ptr.add(k as usize), x);
        k += x;

        x = undump_int(fmt_in);
        if x < 0 || k + x > EQTB_SIZE + 1 {
            bad_format_file();
        }

        for j1 in k..=(k + x - 1) {
            *eqtb_ptr.add(j1 as usize) = *eqtb_ptr.add((k - 1) as usize);
        }
        k += x;
        if k > EQTB_SIZE {
            break;
        }
    }

    if hash_high() > 0 {
        undump_slice(fmt_in, eqtb_ptr.add((EQTB_SIZE + 1) as usize), hash_high());
    }

    x = undump_int(fmt_in);
    if x < FORMAT_HASH_BASE || x > crate::globals::hash_top() {
        bad_format_file();
    }
    set_par_loc(x);
    set_par_token(CS_TOKEN_FLAG + par_loc());

    x = undump_int(fmt_in);
    if x < FORMAT_HASH_BASE || x > crate::globals::hash_top() {
        bad_format_file();
    }
    set_write_loc(x);

    for prim_idx in 0..=PRIM_SIZE {
        let value: crate::mem_types::B32x2 = undump_one(fmt_in);
        set_prim_s0(prim_idx, value.s0);
        set_prim_s1(prim_idx, value.s1);
    }

    x = undump_int(fmt_in);
    if !(FORMAT_HASH_BASE..=FROZEN_CONTROL_SEQUENCE).contains(&x) {
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

    let hash_ptr = crate::state::with_state_mut(|s| s.eqtb.hash.as_mut_ptr());
    undump_slice(
        fmt_in,
        hash_ptr.add((hash_used() + 1 - 514) as usize),
        (UNDEFINED_CONTROL_SEQUENCE - 1) - hash_used(),
    );
    if hash_high() > 0 {
        undump_slice(
            fmt_in,
            hash_ptr.add((EQTB_SIZE + 1 - 514) as usize),
            hash_high(),
        );
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

    // pre-compute before mutable borrow (both backed by with_state)
    let fms = font_mem_size();
    let fmp = fmem_ptr();
    let zero_word = crate::mem_types::MemoryWord {
        b32: crate::mem_types::B32x2 { s0: 0, s1: 0 },
    };
    crate::state::with_state_mut(|s| {
        s.fonts.font_info = vec![zero_word; (fms as usize) + 1];
        undump_slice(fmt_in, s.fonts.font_info.as_mut_ptr(), fmp);
    });

    x = undump_int(fmt_in);
    if x < FONT_BASE {
        bad_format_file();
    }
    if x > FONT_BASE + MAX_FONT_MAX {
        _tt_abort(c"must increase font_max".as_ptr());
    }
    set_font_ptr(x);

    let font_check_len = (font_max() as usize) + 1;
    crate::state::with_state_mut(|state| {
        state.fonts.font_layout_engine = vec![std::ptr::null_mut(); font_check_len];
        state.fonts.font_mapping = vec![std::ptr::null_mut(); font_check_len];
        state.fonts.font_flags = vec![0i8; font_check_len];
        state.fonts.font_letter_space = vec![0i32; font_check_len];
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
        state.fonts.font_glue = vec![0i32; font_check_len];
    });
    let base_arr_len = (font_max() as usize) + 1;
    crate::state::with_state_mut(|state| {
        state.fonts.hyphen_char = vec![0i32; base_arr_len];
        state.fonts.skew_char = vec![0i32; base_arr_len];
        state.fonts.char_base = vec![0i32; base_arr_len];
        state.fonts.width_base = vec![0i32; base_arr_len];
        state.fonts.height_base = vec![0i32; base_arr_len];
        state.fonts.depth_base = vec![0i32; base_arr_len];
        state.fonts.italic_base = vec![0i32; base_arr_len];
        state.fonts.lig_kern_base = vec![0i32; base_arr_len];
        state.fonts.kern_base = vec![0i32; base_arr_len];
        state.fonts.exten_base = vec![0i32; base_arr_len];
        state.fonts.param_base = vec![0i32; base_arr_len];
    });

    let font_undump_count = font_ptr() + 1;
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.font_check.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
        );
    });
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.font_size.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.font_dsize.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
        );
    });
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(
            fmt_in,
            state.fonts.font_params.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
            MIN_HALFWORD,
            MAX_HALFWORD,
        )
    }) {
        bad_format_file();
    }
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.hyphen_char.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.skew_char.as_mut_ptr().add(FONT_BASE as usize),
            font_undump_count,
        );
    });
    let font_count = font_undump_count;
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
    let base_undump_count = font_ptr() + 1;
    crate::state::with_state_mut(|state| {
        undump_slice(
            fmt_in,
            state.fonts.char_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.width_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.height_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.depth_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.italic_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state
                .fonts
                .lig_kern_base
                .as_mut_ptr()
                .add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.kern_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.exten_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
        undump_slice(
            fmt_in,
            state.fonts.param_base.as_mut_ptr().add(FONT_BASE as usize),
            base_undump_count,
        );
    });
    let font_glue_count = font_ptr() + 1;
    let font_glue_max = lo_mem_max();
    if !crate::state::with_state_mut(|state| {
        undump_checked_i32(
            fmt_in,
            state.fonts.font_glue.as_mut_ptr().add(FONT_BASE as usize),
            font_glue_count,
            MIN_HALFWORD,
            font_glue_max,
        )
    }) {
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
        if !(MIN_HALFWORD..=MAX_HALFWORD).contains(&x) {
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

    TRE_POOL_SIZE = 6_250_000;
    TRE_STRING_VACANCIES = 90_000;
    tre_pool_free = 47_500;
    TRE_MAX_STRINGS = 565_536;
    tre_strings_free = 100;
    set_font_mem_size(8_000_000);
    set_font_max(9_000);
    TRE_TRIE_SIZE = 1_000_000;
    TRE_HYPH_SIZE = 8_191;
    tre_buf_size = 200_000;
    tre_nest_size = 500;
    tre_max_in_open = 15;
    tre_param_size = 10_000;
    tre_save_size = 80_000;
    tre_stack_size = 5_000;
    set_error_line(79);
    set_half_error_line(50);
    set_max_print_line(79);
    tre_expand_depth = 10_000;

    crate::state::with_state_mut(|state| {
        state.eqtb.hash_extra = 600_000;
        state.strings.pool_size = TRE_POOL_SIZE;
        state.strings.string_vacancies = TRE_STRING_VACANCIES;
        state.strings.max_strings = TRE_MAX_STRINGS;
        state.hyph.trie_size = TRE_TRIE_SIZE;
        state.hyph.hyph_size = TRE_HYPH_SIZE;
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
            s.input.input_file = vec![std::ptr::null_mut(); n];
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
        let n = (TRE_HYPH_SIZE as usize) + 1;
        crate::state::with_state_mut(|s| {
            s.hyph.hyph_word = vec![0i32; n];
            s.hyph.hyph_list = vec![0i32; n];
            s.hyph.hyph_link = vec![0u16; n];
        });
    }

    if in_initex_mode() {
        // `xmalloc_array(MEM_TOP + 1)` allocated `MEM_TOP + 2` words; match it.
        let mem_words_init = (MEM_TOP as usize) + 2;
        crate::state::with_state_mut(|s| {
            s.mem.data = vec![
                crate::mem_types::MemoryWord {
                    b32: crate::mem_types::B32x2 { s0: 0, s1: 0 }
                };
                mem_words_init
            ];
        });
        crate::state::with_state_mut(|s| {
            s.eqtb.eqtb_top = EQTB_SIZE + s.eqtb.hash_extra;
            s.eqtb.hash_top = if s.eqtb.hash_extra == 0 {
                HASH_TOP_UNDEFINED_CONTROL_SEQUENCE
            } else {
                s.eqtb.eqtb_top
            };

            let n = (1 + s.eqtb.hash_top - 514) as usize;
            s.eqtb.hash = vec![crate::mem_types::B32x2 { s0: 0, s1: 0 }; n];
        });

        crate::globals::set_hash_s0(HASH_BASE, 0);
        crate::globals::set_hash_s1(HASH_BASE, 0);
        let hash_base = crate::globals::hash_s0(HASH_BASE);
        let hash_base_s1 = crate::globals::hash_s1(HASH_BASE);
        let mut h = HASH_BASE + 1;
        let h_top = crate::globals::hash_top();
        while h <= h_top {
            crate::globals::set_hash_s0(h, hash_base);
            crate::globals::set_hash_s1(h, hash_base_s1);
            h += 1;
        }
        crate::globals::set_hash_used(h_top);

        // pre-compute font_mem_size before the mutable borrow (backed by with_state)
        let fms_initex = font_mem_size();
        let zero_word_initex = crate::mem_types::MemoryWord {
            b32: crate::mem_types::B32x2 { s0: 0, s1: 0 },
        };
        // `xcalloc_array(n)` allocated `n + 1` zeroed words; match that.
        let eqtb_words_initex = (crate::globals::eqtb_top() as usize) + 1;
        crate::state::with_state_mut(|state| {
            state.eqtb.data = vec![zero_word_initex; eqtb_words_initex];
            state.strings.str_start = vec![0i32; (TRE_MAX_STRINGS as usize) + 1];
            state.strings.str_pool = vec![0u16; (TRE_POOL_SIZE as usize) + 1];
            state.fonts.font_info = vec![zero_word_initex; (fms_initex as usize) + 1];
        });
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
    if tre_save_size > MAX_HALFWORD || TRE_MAX_STRINGS > MAX_HALFWORD {
        tre_bad = 17;
    }
    if tre_buf_size > MAX_HALFWORD {
        tre_bad = 18;
    }
    if CS_TOKEN_FLAG + EQTB_SIZE + crate::globals::hash_extra() > MAX_HALFWORD {
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

        let font_check_len = (font_max() as usize) + 1;
        crate::state::with_state_mut(|state| {
            state.fonts.font_layout_engine = vec![std::ptr::null_mut(); font_check_len];
            state.fonts.font_mapping = vec![std::ptr::null_mut(); font_check_len];
            state.fonts.font_flags = vec![0i8; font_check_len];
            state.fonts.font_letter_space = vec![0i32; font_check_len];
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
            state.fonts.font_glue = vec![0i32; font_check_len];
        });
        let base_fmt_len = (font_max() as usize) + 1;
        crate::state::with_state_mut(|state| {
            state.fonts.hyphen_char = vec![0i32; base_fmt_len];
            state.fonts.skew_char = vec![0i32; base_fmt_len];
            state.fonts.char_base = vec![0i32; base_fmt_len];
            state.fonts.width_base = vec![0i32; base_fmt_len];
            state.fonts.height_base = vec![0i32; base_fmt_len];
            state.fonts.depth_base = vec![0i32; base_fmt_len];
            state.fonts.italic_base = vec![0i32; base_fmt_len];
            state.fonts.lig_kern_base = vec![0i32; base_fmt_len];
            state.fonts.kern_base = vec![0i32; base_fmt_len];
            state.fonts.exten_base = vec![0i32; base_fmt_len];
            state.fonts.param_base = vec![0i32; base_fmt_len];
        });

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

    let font_used_len = (font_max() as usize) + 1;
    crate::state::with_state_mut(|state| {
        state.fonts.font_used = vec![false; font_used_len];
    });

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
    crate::state::with_state_mut(|st| crate::synctex::synctex_init_command(st));
    crate::io::start_input(input_file_name);
    crate::globals::set_history(HISTORY_SPOTLESS);
    crate::main_control::main_control();
    final_cleanup();
    crate::io::close_files_and_terminate();
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
    // `eqtb` cleanup is now the Rust `Vec` drop on `EngineState.eqtb.data`. (session 51)
    // `hash` cleanup is now the Rust `Vec` drop on `EngineState.eqtb.hash`. (session 52)
    // `mem` cleanup is now the Rust `Vec` drop on `EngineState.mem.data`. (session 58)
    // font_info cleanup moved to EngineState.fonts.font_info (Vec::new() reset).
}

// translated from xetex-ini.c:2226 (tt_cleanup)
pub unsafe fn tt_cleanup() {
    tt_cleanup_pdf_files_close();
    libc::free(tt_cleanup_TEX_format_default as *mut _);
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
            crate::io::u_close(f);
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
        s.input.name_of_file = Vec::new();
        s.input.name_of_file16 = Vec::new();
    });
    crate::state::with_state_mut(|s| {
        s.hyph.hyph_word = Vec::new();
        s.hyph.hyph_list = Vec::new();
        s.hyph.hyph_link = Vec::new();
    });

    // Clear scanner native_text state
    crate::state::with_state_mut(|s| {
        s.scanner.native_text = Vec::new();
        s.scanner.native_text_size = 0;
        s.scanner.native_len = 0;
        s.scanner.save_native_len = 0;
    });

    // Free arrays allocated in load_fmt_file
    crate::state::with_state_mut(|state| {
        state.eqtb.data = Vec::new();
        state.eqtb.hash = Vec::new();
        state.mem.data = Vec::new();
    });
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
        state.fonts.font_glue = Vec::new();
        state.fonts.font_used = Vec::new();
    });

    crate::state::with_state_mut(|state| {
        state.fonts.font_info = Vec::new();
        state.fonts.font_layout_engine = Vec::new();
        state.fonts.font_mapping = Vec::new();
        state.fonts.font_flags = Vec::new();
        state.fonts.font_letter_space = Vec::new();
        state.fonts.hyphen_char = Vec::new();
        state.fonts.skew_char = Vec::new();
        state.fonts.char_base = Vec::new();
        state.fonts.width_base = Vec::new();
        state.fonts.height_base = Vec::new();
        state.fonts.depth_base = Vec::new();
        state.fonts.italic_base = Vec::new();
        state.fonts.lig_kern_base = Vec::new();
        state.fonts.kern_base = Vec::new();
        state.fonts.exten_base = Vec::new();
        state.fonts.param_base = Vec::new();
        state.fonts.xdv_buffer = Vec::new();
        state.fonts.mapped_text = Vec::new();
    });

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

const UNDEFINED_CS: i32 = 103;
const NORMAL: i32 = 0;

const INTER_CHAR_VAL: i32 = 6;

const PC_INT_BASE: i32 = 7_826_729;

const AAT_FONT_FLAG: i32 = 0xFFFF;
const OTGR_FONT_FLAG: i32 = 0xFFFE;

#[inline]
pub(crate) unsafe fn intpar(n: i32) -> i32 {
    eqtb_b32s1(PC_INT_BASE + n)
}
