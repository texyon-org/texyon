//! Engine init / run / cleanup: the `tt_run_engine` entry point, the
//! initialize_* primers, primitive registration, and teardown — translated
//! from the driver part of xetex-ini.c.

use crate::consts::{
    ACTIVE_BASE, BATCH_MODE, BIGGEST_CHAR, BOX_BASE, CAT_CODE_BASE, CS_TOKEN_FLAG, CUR_FONT_LOC,
    DEL_CODE_BASE, DIMEN_BASE, EQTB_SIZE, ERROR_STOP_MODE, ETEX_PEN_BASE, GLUE_BASE, GLUE_NODE,
    INT_BASE, LC_CODE_BASE, LOCAL_BASE, MATH_CODE_BASE, MATH_FONT_BASE, MAX_HALFWORD, NUMBER_USVS,
    PRIM_EQTB_BASE, SELECTOR_TERM_AND_LOG, SELECTOR_TERM_ONLY, SF_CODE_BASE, SINGLE_BASE,
    TOKS_BASE, TOO_BIG_CHAR, UC_CODE_BASE,
};
use crate::format_io::{load_fmt_file, store_fmt_file};
use crate::globals::{
    buf_size, cond_ptr, cur_chr, cur_if, cur_input, cur_level, cur_mark, cur_val, disc_ptr,
    eqtb_b32s1, eqtb_top, eqtb_word, first, font_area_entry, font_layout_engine_ptr,
    font_mapping_ptr, font_max, font_mem_size, history, hyph_next, hyph_size, if_line,
    in_initex_mode, in_open, input_file, interaction, job_name, last_glue, mem_b16s0, mem_b32s1,
    open_parens, par_loc, pool_size, rover, sa_root, selector, set_adjust_tail, set_after_token,
    set_align_ptr, set_avail, set_buffer_entry, set_cancel_boundary, set_cond_ptr, set_cs_count,
    set_cur_align, set_cur_boundary, set_cur_dir, set_cur_f, set_cur_group, set_cur_head,
    set_cur_if, set_cur_level, set_cur_list_aux_b32s1, set_cur_list_e_tex_aux, set_cur_list_head,
    set_cur_list_mode, set_cur_list_mode_line, set_cur_list_prev_graf, set_cur_list_tail,
    set_cur_loop, set_cur_mark, set_cur_order, set_cur_pre_head, set_cur_pre_tail, set_cur_span,
    set_cur_tail_a, set_cur_val, set_cur_val_level, set_dead_cycles, set_deletions_allowed,
    set_disc_ptr, set_doing_leaders, set_doing_special, set_dyn_used, set_edit_name_start,
    set_empty_b32s0, set_empty_b32s1, set_eqtb_b16s0, set_eqtb_b16s1, set_eqtb_b32s1,
    set_eqtb_word, set_error_count, set_error_line, set_expand_depth_count,
    set_file_line_error_style_p, set_first, set_font_info_b32s1, set_font_max, set_font_mem_size,
    set_format_ident, set_half_error_line, set_hash_high, set_hash_s1, set_hash_used, set_help_ptr,
    set_hi_mem_min, set_hyph_count, set_hyph_link_entry, set_hyph_list_entry, set_hyph_next,
    set_hyph_start, set_hyph_word_entry, set_hyphen_char_entry, set_if_limit, set_if_line,
    set_ins_disc, set_insert_penalties, set_interaction, set_is_in_csname, set_last_badness,
    set_last_bop, set_last_glue, set_last_kern, set_last_node_type, set_last_penalty, set_lft_hit,
    set_ligature_present, set_lo_mem_max, set_long_help_seen, set_lr_problems, set_lr_ptr,
    set_mag_set, set_max_h, set_max_hyph_char, set_max_nest_stack, set_max_op_used,
    set_max_print_line, set_max_push, set_max_reg_help_line, set_max_reg_num, set_max_save_stack,
    set_max_v, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1, set_mem_end,
    set_mem_word, set_native_text, set_native_text_size, set_nest_ptr, set_no_new_control_sequence,
    set_null_character, set_open_parens, set_output_active, set_pack_begin_line, set_page_contents,
    set_page_so_far, set_page_tail, set_par_loc, set_par_token, set_pool_ptr, set_pre_adjust_tail,
    set_prim_s0, set_prim_s1, set_prim_used, set_pseudo_files, set_radix, set_read_open_entry,
    set_rover, set_rt_hit, set_sa_chain, set_sa_level, set_sa_null_b32s0, set_sa_null_b32s1,
    set_sa_root, set_save_ptr, set_selector, set_set_box_allowed, set_shown_mode,
    set_skew_char_entry, set_spec_log, set_stop_at_space, set_str_ptr, set_str_start, set_temp_ptr,
    set_total_pages, set_trie_c_entry, set_trie_l_entry, set_trie_not_ready, set_trie_op_ptr,
    set_trie_ptr, set_trie_r_entry, set_trie_used_entry, set_two_to_the, set_use_err_help,
    set_var_used, set_write_loc, set_write_open, set_xeq_level, str_ptr, str_start,
    string_vacancies, temp_ptr, trie_not_ready, two_to_the,
};
use crate::hyphenation::set_trie_op_hash;
use crate::io::init_io;

const LEVEL_ONE: u16 = 1;
const GET_NODE_HEAD_SENTINEL: i32 = 0x4000_0000;

const MEM_TOP: i32 = 4_999_999;
const FORMAT_HASH_OFFSET: i32 = 514;
const HASH_PRIME: i32 = 557_111;
const HASH_SIZE: i32 = 2_242_711;
const HASH_BASE: i32 = 1_114_114;
const HASH_TOP_UNDEFINED_CONTROL_SEQUENCE: i32 = 2_254_339;
const FONT_BASE: i32 = 0;
const MAX_FONT_MAX: i32 = 9_000;
const HISTORY_FATAL_ERROR: i32 = 3;
const HISTORY_SPOTLESS: i32 = 0;
const NEW_LINE: u16 = 33;
const EMPTY_STRING: i32 = 65_536;
const NON_ADDRESS: i32 = 0;
const SELECTOR_NO_PRINT: i32 = 16;
const INT_PAR_TIME: i32 = 20;
const INT_PAR_DAY: i32 = 21;
const INT_PAR_MONTH: i32 = 22;
const INT_PAR_YEAR: i32 = 23;
const INT_PAR_XETEX_GENERATE_ACTUAL_TEXT: i32 = 79;
const INT_PAR_END_LINE_CHAR: i32 = 48;
const NORMAL: i32 = 0;
const PC_INT_BASE: i32 = 7_826_729;

// The `TRE_*` capacity statics are set by `tt_run_engine`; the two that
// `load_fmt_file` also reads are `pub(crate)`.
pub(crate) static mut TRE_POOL_SIZE: i32 = 0;
static mut TRE_STRING_VACANCIES: i32 = 0;
pub(crate) static mut TRE_MAX_STRINGS: i32 = 0;
static mut TRE_TRIE_SIZE: i32 = 0;
static mut TRE_HYPH_SIZE: i32 = 0;

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

// Raw lifecycle state used by tt_run_engine. Kept local because this top-level
// transition glue allocates many globals in one place and will disappear when
// EngineState replaces the C globals.
extern "C" {
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

#[inline]
pub(crate) unsafe fn intpar(n: i32) -> i32 {
    eqtb_b32s1(PC_INT_BASE + n)
}

// ---- moved verbatim from ini.rs (workstream 02) ----

// translated from xetex-ini.c:3452 (get_strings_started)
pub unsafe fn get_strings_started() {
    set_pool_ptr(0);
    set_str_ptr(0);
    set_str_start(0, 0);
    set_str_ptr(TOO_BIG_CHAR);

    if crate::string_pool::load_pool_strings_global(pool_size() - string_vacancies()) == 0 {
        _tt_abort(c"must increase pool_size".as_ptr());
    }
}
// translated from xetex-ini.c:554 (sort_avail)
pub unsafe fn sort_avail() {
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
pub unsafe fn primitive(ident: *const libc::c_char, c: u16, o: i32) {
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

        set_cur_val(crate::hash::id_lookup_global(first(), len));
        set_str_ptr(str_ptr() - 1);
        set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
        set_hash_s1(cur_val(), s);
        prim_val = crate::hash::prim_lookup_global(s);
    } else {
        let ch = *ident as u8 as i32;
        set_cur_val(ch + SINGLE_BASE);
        prim_val = crate::hash::prim_lookup_global(ch);
    }

    set_eqtb_b16s0(cur_val(), LEVEL_ONE);
    set_eqtb_b16s1(cur_val(), c);
    set_eqtb_b32s1(cur_val(), o);
    set_eqtb_b16s0(PRIM_EQTB_BASE + prim_val, LEVEL_ONE);
    set_eqtb_b16s1(PRIM_EQTB_BASE + prim_val, c);
    set_eqtb_b32s1(PRIM_EQTB_BASE + prim_val, o);
}
// translated from xetex-ini.c:2201 (final_cleanup)
pub unsafe fn final_cleanup() {
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
pub unsafe fn initialize_more_variables() {
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
pub unsafe fn initialize_more_initex_variables() {
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
pub unsafe fn initialize_primitives() {
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
