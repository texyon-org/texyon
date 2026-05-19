//! Shipout helpers translated from `xetex/xetex-shipout.c`.

use crate::globals::{
    avail, char_base_entry, cur_area, cur_dir, cur_ext, cur_h, cur_list_mode, cur_name,
    cur_page_height, cur_tok, cur_v, def_ref, doing_leaders, eqtb_b32s1, file_offset,
    font_area_entry, font_check_entry, font_dsize_entry, font_info_b16, font_info_b32s1,
    font_name_entry, font_ptr, font_size_entry, font_used_entry, init_pool_ptr, job_name, last_bop,
    log_opened, lr_problems, lr_ptr, max_h, max_print_line, max_push, max_v, mem_b16s0, mem_b16s1,
    mem_b16s2, mem_b32s0, mem_b32s1, mem_gr, mem_word, name_of_file, pool_ptr, pool_size, rule_dp,
    rule_ht, rule_wd, rust_stdout, selector, semantic_pagination_enabled, set_avail, set_cur_area,
    set_cur_cs, set_cur_ext, set_cur_h, set_cur_h_offset, set_cur_list_mode, set_cur_name,
    set_cur_page_height, set_cur_page_width, set_cur_v, set_cur_v_offset, set_dead_cycles,
    set_doing_leaders, set_doing_special, set_font_ptr, set_font_used_entry, set_help_line,
    set_help_ptr, set_last_bop, set_lr_problems, set_lr_ptr, set_max_h, set_max_push, set_max_v,
    set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1, set_mem_word, set_pdf_last_x_pos,
    set_pdf_last_y_pos, set_pool_ptr, set_rule_dp, set_rule_ht, set_rule_wd, set_selector,
    set_temp_ptr, set_total_pages, set_write_file, set_write_open, shell_escape_enabled,
    str_pool_entry, str_pool_ptr, str_ptr, str_start, temp_ptr, term_offset, total_pages,
    width_base_entry, write_file, write_loc, write_open, xdv_buffer_entry,
};
use crate::mem::{free_node, get_avail, get_node, new_math};

extern "C" {
    fn make_font_def(f: i32) -> libc::c_int;
    fn xmalloc(size: libc::size_t) -> *mut libc::c_void;
    fn ttstub_output_write(
        handle: usize,
        data: *const libc::c_char,
        len: libc::size_t,
    ) -> libc::size_t;
    fn ttstub_output_close(handle: usize) -> libc::c_int;
    fn ttstub_output_open(path: *const libc::c_char, is_gz: libc::c_int) -> usize;
    fn ttstub_output_flush(handle: usize) -> libc::c_int;
    fn ttstub_shell_escape(cmd: *const u16, len: libc::size_t);
    fn _tt_abort(fmt: *const libc::c_char) -> !;
    static output_file_extension: *const libc::c_char;
    fn hlist_out();
}

// DVI output file state — file-statics promoted from xetex-shipout.c.
// Each carries #[no_mangle] so the remaining C functions in xetex-shipout.c
// can reach them via `extern` declarations.
// #[allow] because the names must match C for the ABI link.
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut dvi_file: usize = 0;
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut output_file_name: i32 = 0;
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut dvi_buf: *mut u8 = core::ptr::null_mut();
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut dvi_limit: i32 = 0;
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut dvi_ptr: i32 = 0;
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut dvi_offset: i32 = 0;
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut dvi_gone: i32 = 0;
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut down_ptr: i32 = 0;
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut right_ptr: i32 = 0;
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut cur_s: i32 = 0;
// Promoted from C file-statics in session 113 — used by movement, special_out, pic_out.
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut dvi_h: i32 = 0;
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut dvi_v: i32 = 0;
// Promoted from C file-static in session 115 so Rust ship_out and remaining C
// list-output functions share the current DVI font.
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut dvi_f: i32 = 0;

// xetex-shipout.c:11–12
const DVI_BUF_SIZE: i32 = 16384;
const HALF_BUF: i32 = 8192;
const TEX_INFINITY: i32 = 0x7FFF_FFFF;
const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
const MIN_HALFWORD: i32 = -0x0FFF_FFFF;
const TEX_NULL: i32 = -0x0FFF_FFFF;
// xetex-constants.h:240
const POP: u8 = 142;
const PRE: u8 = 247;
const BOP: u8 = 139;
const PUSH: u8 = 141;
const PUT_RULE: u8 = 137;
const FNT1: u8 = 235;
const SET_GLYPHS: u8 = 253;
const FNT_NUM_0: u8 = 171;

// xetex-constants.h:105,114. EDGE_NODE is an alias of STYLE_NODE.
const HLIST_NODE: u16 = 0;
const VLIST_NODE: u16 = 1;
const RULE_NODE: u16 = 2;
const LIGATURE_NODE: u16 = 6;
const WHATSIT_NODE: u16 = 8;
const MATH_NODE: u16 = 9;
const GLUE_NODE: u16 = 10;
const KERN_NODE: u16 = 11;
const EDGE_NODE: u16 = 14;
const SMALL_NODE_SIZE: i32 = 2;
const MEDIUM_NODE_SIZE: i32 = 3;
const EDGE_NODE_SIZE: i32 = 3;
const GLUE_SPEC_SIZE: i32 = 4;
const AAT_FONT_FLAG: i32 = 65533;
const OTGR_FONT_FLAG: i32 = 65534;
const FNT_DEF1: u8 = 243;
const DEFINE_NATIVE_FONT: u8 = 252;
const TOO_BIG_CHAR: i32 = 65536;

// DVI movement opcodes — xetex-constants.h:241–242,244–245
const DOWN1: u8 = 157;
const RIGHT1: u8 = 143;
const XXX1: u8 = 239;
const XXX4: u8 = 242;

// Movement node states — xetex-constants.h:393–401
const MOV_NONE_SEEN: i32 = 0;
const MOV_Y_HERE: i32 = 1;
const MOV_Z_HERE: i32 = 2;
const MOV_YZ_OK: i32 = 3;
const MOV_Y_OK: i32 = 4;
const MOV_Z_OK: i32 = 5;
const MOV_Y_SEEN: i32 = 6;
const MOV_D_FIXED: i32 = 6;
const MOV_Z_SEEN: i32 = 12;

// Whatsit subtype codes — xetex_format.h:457–461
const OPEN_NODE: u16 = 0;
const WRITE_NODE: u16 = 1;
const CLOSE_NODE: u16 = 2;
const SPECIAL_NODE: u16 = 3;
const LANGUAGE_NODE: u16 = 4;
const NATIVE_WORD_NODE: u16 = 40;
const NATIVE_WORD_NODE_AT: u16 = 41;
const GLYPH_NODE: u16 = 42;
const PIC_NODE: u16 = 43;
const PDF_NODE: u16 = 44;
const PDF_SAVE_POS_NODE: u16 = 21;

// Glue and leader constants: xetex-constants.h:175,182,275,281; xetex_format.h:422.
const FILLL: u16 = 3;
const NORMAL: u16 = 0;
const STRETCHING: u16 = 1;
const SHRINKING: u16 = 2;
const A_LEADERS: u16 = 100;
const C_LEADERS: u16 = 101;

// MATH_NODE LR constants: xetex-constants.h:142-143; xetex_format.h:437.
const L_CODE: u16 = 4;
const R_CODE: u16 = 8;
const END_M_CODE: u16 = 3;

// xetex-constants.h:133, tectonic_bridge_core_generated.h:12
const PIC_NODE_SIZE: i32 = 9;
const INVALID_HANDLE: usize = 0;

// INT_PAR index for \tracingonline — xetex_format.h
const INT_PAR_TRACING_ONLINE: i32 = 29;

// Token constants — xetex-constants.h:204,217,348–351
const INSERTED: u16 = 5;
const WRITE_TEXT: u16 = 18;
const CS_TOKEN_FLAG: i32 = 0x1FF_FFFF;
const LEFT_BRACE_TOKEN: i32 = 0x20_0000;
const RIGHT_BRACE_TOKEN: i32 = 0x40_0000;
// END_WRITE = FROZEN_CONTROL_SEQUENCE + 8 = 2_243_226 + 8
const END_WRITE: i32 = 2_243_234;

// Output selector values — xetex-core.h:43–50
const SELECTOR_TERM_ONLY: i32 = 17;
const SELECTOR_LOG_ONLY: i32 = 18;
const SELECTOR_TERM_AND_LOG: i32 = 19;
const SELECTOR_NEW_STRING: i32 = 21;

/// Create a new edge node of subtype `s` and width `w`.
///
/// translated from xetex-shipout.c:1708 (new_edge)
#[no_mangle]
pub unsafe extern "C" fn new_edge(s: i16, w: i32) -> i32 {
    let p = get_node(EDGE_NODE_SIZE);
    set_mem_b16s1(p, EDGE_NODE);
    set_mem_b16s0(p, s as u16);
    set_mem_b32s1(p + 1, w);
    set_mem_b32s1(p + 2, 0);
    p
}

/// translated from xetex-shipout.c:2403 (dvi_four)
#[no_mangle]
pub unsafe extern "C" fn dvi_four(mut x: i32) {
    if x >= 0 {
        dvi_out((x / 0x1000000) as u8);
    } else {
        x += 0x40000000;
        x += 0x40000000;
        dvi_out(((x / 0x1000000) + 128) as u8);
    }

    x %= 0x1000000;
    dvi_out((x / 0x10000) as u8);

    x %= 0x10000;
    dvi_out((x / 0x100) as u8);
    dvi_out((x % 0x100) as u8);
}

/// translated from xetex-shipout.c:2423 (dvi_two)
#[no_mangle]
pub unsafe extern "C" fn dvi_two(s: u16) {
    dvi_out((s / 0x100) as u8);
    dvi_out((s % 0x100) as u8);
}

/// translated from xetex-shipout.c:1784 (dvi_native_font_def)
unsafe fn dvi_native_font_def(f: i32) {
    dvi_out(DEFINE_NATIVE_FONT);
    dvi_four(f - 1);
    let font_def_length = make_font_def(f);

    for i in 0..font_def_length {
        dvi_out(xdv_buffer_entry(i));
    }
}

/// translated from xetex-shipout.c:1798 (dvi_font_def)
#[no_mangle]
pub unsafe extern "C" fn dvi_font_def(f: i32) {
    if font_area_entry(f) == AAT_FONT_FLAG || font_area_entry(f) == OTGR_FONT_FLAG {
        dvi_native_font_def(f);
    } else {
        if f <= 256 {
            dvi_out(FNT_DEF1);
            dvi_out((f - 1) as u8);
        } else {
            dvi_out(FNT_DEF1 + 1);
            dvi_out(((f - 1) / 256) as u8);
            dvi_out(((f - 1) % 256) as u8);
        }

        let font_check = font_check_entry(f);
        dvi_out(font_check.s3 as u8);
        dvi_out(font_check.s2 as u8);
        dvi_out(font_check.s1 as u8);
        dvi_out(font_check.s0 as u8);
        dvi_four(font_size_entry(f));
        dvi_four(font_dsize_entry(f));
        dvi_out(crate::string_pool::length(font_area_entry(f)) as u8);

        let font_name = font_name_entry(f);
        let mut l = 0;
        let mut k = str_start(font_name - TOO_BIG_CHAR);

        while l == 0 && k < str_start(font_name + 1 - TOO_BIG_CHAR) {
            if str_pool_entry(k) == b':' as u16 {
                l = k - str_start(font_name - TOO_BIG_CHAR);
            }
            k += 1;
        }

        if l == 0 {
            l = crate::string_pool::length(font_name);
        }

        dvi_out(l as u8);

        k = str_start(font_area_entry(f) - TOO_BIG_CHAR);
        let mut for_end = str_start(font_area_entry(f) + 1 - TOO_BIG_CHAR) - 1;
        while k <= for_end {
            dvi_out(str_pool_entry(k) as u8);
            k += 1;
        }

        k = str_start(font_name - TOO_BIG_CHAR);
        for_end = str_start(font_name - TOO_BIG_CHAR) + l - 1;
        while k <= for_end {
            dvi_out(str_pool_entry(k) as u8);
            k += 1;
        }
    }
}

// translated from xetex-shipout.c:48 (initialize_shipout_variables)
#[no_mangle]
pub unsafe extern "C" fn initialize_shipout_variables() {
    output_file_name = 0;
    // xmalloc_array(eight_bits, DVI_BUF_SIZE) = xmalloc((DVI_BUF_SIZE+1) * sizeof(u8))
    dvi_buf = xmalloc((DVI_BUF_SIZE + 1) as libc::size_t) as *mut u8;
    dvi_limit = DVI_BUF_SIZE;
    dvi_ptr = 0;
    dvi_offset = 0;
    dvi_gone = 0;
    down_ptr = TEX_NULL;
    right_ptr = TEX_NULL;
    cur_s = -1;
}

// translated from xetex-shipout.c:63 (deinitialize_shipout_variables)
#[no_mangle]
pub unsafe extern "C" fn deinitialize_shipout_variables() {
    libc::free(dvi_buf as *mut libc::c_void);
    dvi_buf = core::ptr::null_mut();
}

// translated from xetex-shipout.c:2296 (write_to_dvi)
// #[no_mangle] because finalize_dvi_file (still in C) calls this.
#[no_mangle]
pub unsafe extern "C" fn write_to_dvi(a: i32, b: i32) {
    let n = (b - a + 1) as libc::size_t;
    if ttstub_output_write(dvi_file, dvi_buf.add(a as usize) as *const libc::c_char, n) != n {
        _tt_abort(c"failed to write data to XDV file".as_ptr());
    }
}

// translated from xetex-shipout.c:2306 (dvi_swap)
unsafe fn dvi_swap() {
    if dvi_ptr > TEX_INFINITY - dvi_offset {
        cur_s = -2;
        crate::errors::fatal_error(c"dvi length exceeds 0x7FFFFFFF".as_ptr());
    }
    if dvi_limit == DVI_BUF_SIZE {
        write_to_dvi(0, HALF_BUF - 1);
        dvi_limit = HALF_BUF;
        dvi_offset += DVI_BUF_SIZE;
        dvi_ptr = 0;
    } else {
        write_to_dvi(HALF_BUF, DVI_BUF_SIZE - 1);
        dvi_limit = DVI_BUF_SIZE;
    }
    dvi_gone += HALF_BUF;
}

// translated from xetex-shipout.c:71 (dvi_out)
#[no_mangle]
pub unsafe extern "C" fn dvi_out(c: u8) {
    *dvi_buf.add(dvi_ptr as usize) = c;
    dvi_ptr += 1;
    if dvi_ptr == dvi_limit {
        dvi_swap();
    }
}

// translated from xetex-shipout.c:56 (ship_out)
#[no_mangle]
pub unsafe extern "C" fn ship_out(p: i32) {
    crate::synctex::synctex_sheet(eqtb_b32s1(INT_BASE + INT_PAR_MAG));

    if job_name() == 0 {
        crate::scan::open_log_file();
    }

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) > 0 {
        crate::output::print_nl_cstr(c"".as_ptr());
        crate::output::print_ln();
        crate::output::print_cstr(c"Completed box being shipped out".as_ptr());
    }

    if term_offset() > max_print_line() - 9 {
        crate::output::print_ln();
    } else if term_offset() > 0 || file_offset() > 0 {
        crate::output::print_char(b' ' as i32);
    }

    crate::output::print_char(b'[' as i32);
    let mut j: u8 = 9;
    while j > 0 && eqtb_b32s1(COUNT_BASE + j as i32) == 0 {
        j -= 1;
    }

    for k in 0..=j {
        crate::output::print_int(eqtb_b32s1(COUNT_BASE + k as i32));
        if k < j {
            crate::output::print_char(b'.' as i32);
        }
    }

    ttstub_output_flush(rust_stdout());

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) > 0 {
        crate::output::print_char(b']' as i32);
        crate::output::begin_diagnostic();
        crate::scan::show_box(p);
        crate::output::end_diagnostic(true);
    }

    if mem_b32s1(p + 3) > MAX_HALFWORD
        || mem_b32s1(p + 2) > MAX_HALFWORD
        || mem_b32s1(p + 3) + mem_b32s1(p + 2) + eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET)
            > MAX_HALFWORD
        || mem_b32s1(p + 1) + eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET) > MAX_HALFWORD
    {
        crate::output::error_here_with_diagnostic(c"Huge page cannot be shipped out".as_ptr());
        crate::output::capture_to_diagnostic(core::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"The page just created is more than 18 feet tall or".as_ptr(),
        );
        set_help_line(
            0,
            c"more than 18 feet wide, so I suspect something went wrong.".as_ptr(),
        );
        crate::errors::error();

        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) <= 0 {
            crate::output::begin_diagnostic();
            crate::output::print_nl_cstr(c"The following box has been deleted:".as_ptr());
            crate::scan::show_box(p);
            crate::output::end_diagnostic(true);
        }
    } else {
        let page_height =
            mem_b32s1(p + 3) + mem_b32s1(p + 2) + eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET);
        if page_height > max_v() {
            set_max_v(page_height);
        }

        let page_width = mem_b32s1(p + 1) + eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET);
        if page_width > max_h() {
            set_max_h(page_width);
        }

        dvi_h = 0;
        dvi_v = 0;
        set_cur_h(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET));
        dvi_f = FONT_BASE;

        set_cur_h_offset(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET) + ONE_INCH);
        set_cur_v_offset(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET) + ONE_INCH);

        if eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH) != 0 {
            set_cur_page_width(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH));
        } else {
            set_cur_page_width(
                mem_b32s1(p + 1) + 2 * (eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_H_OFFSET) + ONE_INCH),
            );
        }

        if eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT) != 0 {
            set_cur_page_height(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT));
        } else {
            set_cur_page_height(
                mem_b32s1(p + 3)
                    + mem_b32s1(p + 2)
                    + 2 * (eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET) + ONE_INCH),
            );
        }

        if output_file_name == 0 {
            if job_name() == 0 {
                crate::scan::open_log_file();
            }
            crate::scan::pack_job_name(output_file_extension);
            dvi_file = ttstub_output_open(name_of_file(), 0);
            if dvi_file == INVALID_HANDLE {
                let mut buf = [0u8; 512];
                libc::snprintf(
                    buf.as_mut_ptr() as *mut libc::c_char,
                    buf.len(),
                    c"cannot open output file \"%s\"".as_ptr(),
                    name_of_file(),
                );
                _tt_abort(buf.as_ptr() as *const libc::c_char);
            }
            output_file_name = crate::scan::make_name_string();
        }

        if total_pages() == 0 {
            dvi_out(PRE);
            if semantic_pagination_enabled() {
                dvi_out(SPX_ID_BYTE);
            } else {
                dvi_out(XDV_ID_BYTE);
            }
            dvi_four(25_400_000);
            dvi_four(473_628_672);
            crate::save::prepare_mag();
            dvi_four(eqtb_b32s1(INT_BASE + INT_PAR_MAG));

            let output_comment = c"tectonic";
            let l = libc::strlen(output_comment.as_ptr()) as u8;
            dvi_out(l);
            for s in 0..l {
                dvi_out(*output_comment.as_ptr().add(s as usize) as u8);
            }
        }

        let page_loc = dvi_offset + dvi_ptr;
        dvi_out(BOP);
        for k in 0..10 {
            dvi_four(eqtb_b32s1(COUNT_BASE + k));
        }
        dvi_four(last_bop());
        set_last_bop(page_loc);

        let old_setting = selector();
        set_selector(SELECTOR_NEW_STRING);
        crate::output::print_cstr(c"pdf:pagesize ".as_ptr());
        if eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH) <= 0
            || eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT) <= 0
        {
            crate::output::print_cstr(c"default".as_ptr());
        } else {
            crate::output::print_cstr(c"width".as_ptr());
            crate::output::print(b' ' as i32);
            crate::output::print_scaled(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_WIDTH));
            crate::output::print_cstr(c"pt".as_ptr());
            crate::output::print(b' ' as i32);
            crate::output::print_cstr(c"height".as_ptr());
            crate::output::print(b' ' as i32);
            crate::output::print_scaled(eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_PDF_PAGE_HEIGHT));
            crate::output::print_cstr(c"pt".as_ptr());
        }
        set_selector(old_setting);

        dvi_out(XXX1);
        dvi_out(crate::hash::cur_length() as u8);
        let start = str_start(str_ptr() - TOO_BIG_CHAR);
        let end = pool_ptr();
        for s in start..end {
            dvi_out(str_pool_entry(s) as u8);
        }
        set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));

        set_cur_v(mem_b32s1(p + 3) + eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_V_OFFSET));
        set_temp_ptr(p);
        if mem_b16s1(p) == VLIST_NODE {
            vlist_out();
        } else {
            hlist_out();
        }

        dvi_out(EOP);
        set_total_pages(total_pages() + 1);
        cur_s = -1;
    }

    if lr_problems() > 0 {
        crate::output::print_ln();
        crate::output::print_nl_cstr(c"\\endL or \\endR problem (".as_ptr());
        crate::output::print_int(lr_problems() / 10000);
        crate::output::print_cstr(c" missing, ".as_ptr());
        crate::output::print_int(lr_problems() % 10000);
        crate::output::print_cstr(c" extra".as_ptr());
        set_lr_problems(0);
        crate::output::print_char(b')' as i32);
        crate::output::print_ln();
    }

    if lr_ptr() != TEX_NULL || cur_dir() != LEFT_TO_RIGHT {
        crate::errors::confusion(c"LR3".as_ptr());
    }

    if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_OUTPUT) <= 0 {
        crate::output::print_char(b']' as i32);
    }

    set_dead_cycles(0);
    ttstub_output_flush(rust_stdout());
    crate::scan::flush_node_list(p);
    crate::synctex::synctex_teehs();
}

// translated from xetex-shipout.c:2332 (dvi_pop)
#[no_mangle]
pub unsafe extern "C" fn dvi_pop(l: i32) {
    if l == dvi_offset + dvi_ptr && dvi_ptr > 0 {
        dvi_ptr -= 1;
    } else {
        dvi_out(POP);
    }
}

unsafe fn ensure_dvi_font(f: i32) {
    if f != dvi_f {
        if !font_used_entry(f) {
            dvi_font_def(f);
            set_font_used_entry(f, true);
        }

        if f <= 64 {
            dvi_out((f as u8) + FNT_NUM_0 - 1);
        } else if f <= 256 {
            dvi_out(FNT1);
            dvi_out((f - 1) as u8);
        } else {
            dvi_out(FNT1 + 1);
            dvi_out(((f - 1) / 256) as u8);
            dvi_out(((f - 1) % 256) as u8);
        }

        dvi_f = f;
    }
}

// translated from xetex-shipout.c:866 (vlist_out)
#[no_mangle]
pub unsafe extern "C" fn vlist_out() {
    let mut cur_g = 0;
    let mut cur_glue = 0.0;
    let this_box = temp_ptr();
    let g_order = mem_b16s0(this_box + 5);
    let g_sign = mem_b16s1(this_box + 5);
    let mut p = mem_b32s1(this_box + 5);
    let upwards = mem_b16s0(this_box) == 1;

    cur_s += 1;
    if cur_s > 0 {
        dvi_out(PUSH);
    }

    if cur_s > max_push() {
        set_max_push(cur_s);
    }

    let save_loc = dvi_offset + dvi_ptr;
    let left_edge = cur_h();
    crate::synctex::synctex_vlist(this_box);

    if upwards {
        set_cur_v(cur_v() + mem_b32s1(this_box + 2));
    } else {
        set_cur_v(cur_v() - mem_b32s1(this_box + 3));
    }

    let top_edge = cur_v();

    while p != TEX_NULL {
        if is_char_node(p) {
            crate::errors::confusion(c"vlistout".as_ptr());
        }

        let mut go_next = true;
        match mem_b16s1(p) {
            HLIST_NODE | VLIST_NODE => {
                if mem_b32s1(p + 5) == TEX_NULL {
                    if upwards {
                        set_cur_v(cur_v() - mem_b32s1(p + 2));
                    } else {
                        set_cur_v(cur_v() + mem_b32s1(p + 3));
                    }

                    if mem_b16s1(p) == VLIST_NODE {
                        crate::synctex::synctex_void_vlist(p, this_box);
                    } else {
                        crate::synctex::synctex_void_hlist(p, this_box);
                    }

                    if upwards {
                        set_cur_v(cur_v() - mem_b32s1(p + 3));
                    } else {
                        set_cur_v(cur_v() + mem_b32s1(p + 2));
                    }
                } else {
                    if upwards {
                        set_cur_v(cur_v() - mem_b32s1(p + 2));
                    } else {
                        set_cur_v(cur_v() + mem_b32s1(p + 3));
                    }

                    if cur_v() != dvi_v {
                        movement(cur_v() - dvi_v, DOWN1);
                        dvi_v = cur_v();
                    }

                    let save_h = dvi_h;
                    let save_v = dvi_v;

                    if cur_dir() == RIGHT_TO_LEFT {
                        set_cur_h(left_edge - mem_b32s1(p + 4));
                    } else {
                        set_cur_h(left_edge + mem_b32s1(p + 4));
                    }

                    set_temp_ptr(p);
                    if mem_b16s1(p) == VLIST_NODE {
                        vlist_out();
                    } else {
                        hlist_out();
                    }

                    dvi_h = save_h;
                    dvi_v = save_v;

                    if upwards {
                        set_cur_v(save_v - mem_b32s1(p + 3));
                    } else {
                        set_cur_v(save_v + mem_b32s1(p + 2));
                    }

                    set_cur_h(left_edge);
                }
            }
            RULE_NODE => {
                set_rule_ht(mem_b32s1(p + 3));
                set_rule_dp(mem_b32s1(p + 2));
                set_rule_wd(mem_b32s1(p + 1));
                finish_vlist_rule(this_box, left_edge, upwards);
            }
            WHATSIT_NODE => match mem_b16s0(p) {
                GLYPH_NODE => {
                    set_cur_v(cur_v() + mem_b32s1(p + 3));
                    set_cur_h(left_edge);

                    if cur_h() != dvi_h {
                        movement(cur_h() - dvi_h, RIGHT1);
                        dvi_h = cur_h();
                    }

                    if cur_v() != dvi_v {
                        movement(cur_v() - dvi_v, DOWN1);
                        dvi_v = cur_v();
                    }

                    let f = mem_b16s2(p + 4) as i32;
                    ensure_dvi_font(f);

                    dvi_out(SET_GLYPHS);
                    dvi_four(0);
                    dvi_two(1);
                    dvi_four(0);
                    dvi_four(0);
                    dvi_two(mem_b16s1(p + 4));

                    set_cur_v(cur_v() + mem_b32s1(p + 2));
                    set_cur_h(left_edge);
                }
                PIC_NODE | PDF_NODE => {
                    let save_h = dvi_h;
                    let save_v = dvi_v;
                    set_cur_v(cur_v() + mem_b32s1(p + 3));
                    pic_out(p);
                    dvi_h = save_h;
                    dvi_v = save_v;
                    set_cur_v(save_v + mem_b32s1(p + 2));
                    set_cur_h(left_edge);
                }
                PDF_SAVE_POS_NODE => {
                    set_pdf_last_x_pos(cur_h() + ONE_INCH_LESS_ONE);
                    set_pdf_last_y_pos(cur_page_height() - cur_v() - ONE_INCH_LESS_ONE);
                }
                _ => out_what(p),
            },
            GLUE_NODE => {
                let g = mem_b32s0(p + 1);
                set_rule_ht(mem_b32s1(g + 1) - cur_g);

                if g_sign != NORMAL {
                    if g_sign == STRETCHING {
                        if mem_b16s1(g) == g_order {
                            cur_glue += mem_b32s1(g + 2) as f64;
                            let glue_temp = (mem_gr(this_box + 6) * cur_glue)
                                .clamp(-1_000_000_000.0, 1_000_000_000.0);
                            cur_g = crate::scaled_math::tex_round(glue_temp);
                        }
                    } else if mem_b16s0(g) == g_order {
                        cur_glue -= mem_b32s1(g + 3) as f64;
                        let glue_temp = (mem_gr(this_box + 6) * cur_glue)
                            .clamp(-1_000_000_000.0, 1_000_000_000.0);
                        cur_g = crate::scaled_math::tex_round(glue_temp);
                    }
                }

                set_rule_ht(rule_ht() + cur_g);

                if mem_b16s0(p) >= A_LEADERS {
                    let leader_box = mem_b32s1(p + 1);

                    if mem_b16s1(leader_box) == RULE_NODE {
                        set_rule_wd(mem_b32s1(leader_box + 1));
                        set_rule_dp(0);
                        finish_vlist_rule(this_box, left_edge, upwards);
                    } else {
                        let leader_ht = mem_b32s1(leader_box + 3) + mem_b32s1(leader_box + 2);

                        if leader_ht > 0 && rule_ht() > 0 {
                            set_rule_ht(rule_ht() + 10);
                            let edge = cur_v() + rule_ht();
                            let mut lx = 0;

                            if mem_b16s0(p) == A_LEADERS {
                                let save_v = cur_v();
                                set_cur_v(
                                    top_edge + leader_ht * ((cur_v() - top_edge) / leader_ht),
                                );
                                if cur_v() < save_v {
                                    set_cur_v(cur_v() + leader_ht);
                                }
                            } else {
                                let lq = rule_ht() / leader_ht;
                                let lr = rule_ht() % leader_ht;

                                if mem_b16s0(p) == C_LEADERS {
                                    set_cur_v(cur_v() + (lr / 2));
                                } else {
                                    lx = lr / (lq + 1);
                                    set_cur_v(cur_v() + ((lr - (lq - 1) * lx) / 2));
                                }
                            }

                            while cur_v() + leader_ht <= edge {
                                if cur_dir() == RIGHT_TO_LEFT {
                                    set_cur_h(left_edge - mem_b32s1(leader_box + 4));
                                } else {
                                    set_cur_h(left_edge + mem_b32s1(leader_box + 4));
                                }

                                if cur_h() != dvi_h {
                                    movement(cur_h() - dvi_h, RIGHT1);
                                    dvi_h = cur_h();
                                }

                                let save_h = dvi_h;
                                set_cur_v(cur_v() + mem_b32s1(leader_box + 3));

                                if cur_v() != dvi_v {
                                    movement(cur_v() - dvi_v, DOWN1);
                                    dvi_v = cur_v();
                                }

                                let save_v = dvi_v;
                                set_temp_ptr(leader_box);
                                let outer_doing_leaders = doing_leaders();
                                set_doing_leaders(true);

                                if mem_b16s1(leader_box) == VLIST_NODE {
                                    vlist_out();
                                } else {
                                    hlist_out();
                                }

                                set_doing_leaders(outer_doing_leaders);
                                dvi_v = save_v;
                                dvi_h = save_h;
                                set_cur_h(left_edge);
                                set_cur_v(save_v - mem_b32s1(leader_box + 3) + leader_ht + lx);
                            }

                            set_cur_v(edge - 10);
                            go_next = true;
                        } else {
                            move_past_vlist_rule(upwards);
                        }
                    }
                } else {
                    move_past_vlist_rule(upwards);
                }
            }
            KERN_NODE => {
                if upwards {
                    set_cur_v(cur_v() - mem_b32s1(p + 1));
                } else {
                    set_cur_v(cur_v() + mem_b32s1(p + 1));
                }
            }
            _ => {}
        }

        if go_next {
            p = mem_b32s1(p);
        }
    }

    crate::synctex::synctex_tsilv(this_box);
    prune_movements(save_loc);

    if cur_s > 0 {
        dvi_pop(save_loc);
    }
    cur_s -= 1;
}

unsafe fn finish_vlist_rule(this_box: i32, left_edge: i32, upwards: bool) {
    if rule_wd() == NULL_FLAG {
        set_rule_wd(mem_b32s1(this_box + 1));
    }

    set_rule_ht(rule_ht() + rule_dp());

    if upwards {
        set_cur_v(cur_v() - rule_ht());
    } else {
        set_cur_v(cur_v() + rule_ht());
    }

    if rule_ht() > 0 && rule_wd() > 0 {
        if cur_dir() == RIGHT_TO_LEFT {
            set_cur_h(cur_h() - rule_wd());
        }

        if cur_h() != dvi_h {
            movement(cur_h() - dvi_h, RIGHT1);
            dvi_h = cur_h();
        }

        if cur_v() != dvi_v {
            movement(cur_v() - dvi_v, DOWN1);
            dvi_v = cur_v();
        }

        dvi_out(PUT_RULE);
        dvi_four(rule_ht());
        dvi_four(rule_wd());
        set_cur_h(left_edge);
    }
}

unsafe fn move_past_vlist_rule(upwards: bool) {
    if upwards {
        set_cur_v(cur_v() - rule_ht());
    } else {
        set_cur_v(cur_v() + rule_ht());
    }
}

// xetex-constants.h:238,240,248,249,226,227
const EOP: u8 = 140;
const POST: u8 = 248;
const POST_POST: u8 = 249;
const XDV_ID_BYTE: u8 = 7;
const SPX_ID_BYTE: u8 = 100;
// xetex-constants.h:117
const MOVEMENT_NODE_SIZE: i32 = 3;
// xetex_format.h:52; xetex-xetexd.h INT_BASE
const INT_BASE: i32 = 7_826_729;
const INT_PAR_MAG: i32 = 17;
const INT_PAR_TRACING_OUTPUT: i32 = 34;
const DIMEN_BASE: i32 = 8_941_180;
const DIMEN_PAR_H_OFFSET: i32 = 18;
const DIMEN_PAR_V_OFFSET: i32 = 19;
const DIMEN_PAR_PDF_PAGE_WIDTH: i32 = 21;
const DIMEN_PAR_PDF_PAGE_HEIGHT: i32 = 22;
const COUNT_BASE: i32 = 7_826_812;
// scan.rs FONT_BASE
const FONT_BASE: i32 = 0;
const LEFT_TO_RIGHT: i16 = 0;
const RIGHT_TO_LEFT: i16 = 1;
const ONE_INCH: i32 = 4_736_287;
const ONE_INCH_LESS_ONE: i32 = 4_736_286;
const NULL_FLAG: i32 = -0x4000_0000;

// translated from xetex-shipout.c:1930 (prune_movements)
#[no_mangle]
pub unsafe extern "C" fn prune_movements(l: i32) {
    while down_ptr != TEX_NULL {
        if mem_b32s1(down_ptr + 2) < l {
            break;
        }
        let p = down_ptr;
        down_ptr = mem_b32s1(p);
        free_node(p, MOVEMENT_NODE_SIZE);
    }
    while right_ptr != TEX_NULL {
        if mem_b32s1(right_ptr + 2) < l {
            return;
        }
        let p = right_ptr;
        right_ptr = mem_b32s1(p);
        free_node(p, MOVEMENT_NODE_SIZE);
    }
}

// print_c_string: local equivalent of xetex-xetexd.h static inline print_c_string.
unsafe fn print_c_string_here(s: *const libc::c_char) {
    let mut p = s;
    while *p != 0 {
        crate::output::print_char(*p as i32);
        p = p.add(1);
    }
}

// translated from xetex-shipout.c:2173 (finalize_dvi_file)
#[no_mangle]
pub unsafe extern "C" fn finalize_dvi_file() {
    while cur_s > -1 {
        if cur_s > 0 {
            dvi_out(POP);
        } else {
            dvi_out(EOP);
            set_total_pages(total_pages() + 1);
        }
        cur_s -= 1;
    }

    if total_pages() == 0 {
        crate::output::print_nl_cstr(c"No pages of output.".as_ptr());
        return;
    }

    if cur_s == -2 {
        // DVI got too big; message already printed
        return;
    }

    dvi_out(POST);
    dvi_four(last_bop());
    set_last_bop(dvi_offset + dvi_ptr - 5);
    dvi_four(25_400_000); // magic: conversion ratio for sp
    dvi_four(473_628_672); // magic: conversion ratio for sp
    crate::save::prepare_mag();
    dvi_four(eqtb_b32s1(INT_BASE + INT_PAR_MAG));
    dvi_four(max_v());
    dvi_four(max_h());
    dvi_out((max_push() / 256) as u8);
    dvi_out((max_push() % 256) as u8);
    dvi_out(((total_pages() / 256) % 256) as u8);
    dvi_out((total_pages() % 256) as u8);

    let mut fp = font_ptr();
    while fp > FONT_BASE {
        if font_used_entry(fp) {
            dvi_font_def(fp);
        }
        fp -= 1;
    }
    set_font_ptr(fp);

    dvi_out(POST_POST);
    dvi_four(last_bop());

    if semantic_pagination_enabled() {
        dvi_out(SPX_ID_BYTE);
    } else {
        dvi_out(XDV_ID_BYTE);
    }

    let mut k = (4 + (DVI_BUF_SIZE - dvi_ptr) % 4) as u8;
    while k > 0 {
        dvi_out(223);
        k -= 1;
    }

    if dvi_limit == HALF_BUF {
        write_to_dvi(HALF_BUF, DVI_BUF_SIZE - 1);
    }

    if dvi_ptr > TEX_INFINITY - dvi_offset {
        cur_s = -2;
        crate::errors::fatal_error(c"dvi length exceeds 0x7FFFFFFF".as_ptr());
    }

    if dvi_ptr > 0 {
        write_to_dvi(0, dvi_ptr - 1);
    }

    let k = ttstub_output_close(dvi_file) as u8;

    if k == 0 {
        crate::output::print_nl_cstr(c"Output written on ".as_ptr());
        crate::output::print(output_file_name);
        crate::output::print_cstr(c" (".as_ptr());
        crate::output::print_int(total_pages());
        if total_pages() != 1 {
            crate::output::print_cstr(c" pages".as_ptr());
        } else {
            crate::output::print_cstr(c" page".as_ptr());
        }
        crate::output::print_cstr(c", ".as_ptr());
        crate::output::print_int(dvi_offset + dvi_ptr);
        crate::output::print_cstr(c" bytes).".as_ptr());
    } else {
        crate::output::print_nl_cstr(c"Error ".as_ptr());
        crate::output::print_int(k as i32);
        crate::output::print_cstr(c" (".as_ptr());
        print_c_string_here(libc::strerror(k as libc::c_int));
        crate::output::print_cstr(c") generating output;".as_ptr());
        crate::output::print_nl_cstr(c"file ".as_ptr());
        crate::output::print(output_file_name);
        crate::output::print_cstr(c" may not be valid.".as_ptr());
        // XeTeX adds history = OUTPUT_FAILURE = 4 here; not implemented.
    }
}

#[inline]
fn is_char_node(p: i32) -> bool {
    (p as u32) >= 0x0C00_0000
}

#[inline]
fn math_node_lr_dir(p: i32) -> u16 {
    mem_b16s0(p) / R_CODE
}

#[inline]
fn math_node_end_lr_type(p: i32) -> u16 {
    L_CODE * (mem_b16s0(p) / L_CODE) + END_M_CODE
}

#[inline]
fn font_character_width(f: i32, c: i32) -> i32 {
    let info = font_info_b16(char_base_entry(f) + c);
    font_info_b32s1(width_base_entry(f) + info.s3 as i32)
}

// translated from xetex-shipout.c:1470 (reverse)
// Called by hlist_out (still in C) -> #[no_mangle].
#[no_mangle]
pub unsafe extern "C" fn reverse(
    this_box: i32,
    t: i32,
    cur_g: *mut i32,
    cur_glue: *mut f64,
) -> i32 {
    let g_order = mem_b16s0(this_box + 5);
    let g_sign = mem_b16s1(this_box + 5);
    let mut l = t;
    let mut p = temp_ptr();
    let mut m = MIN_HALFWORD;
    let mut n = MIN_HALFWORD;

    loop {
        while p != TEX_NULL {
            if is_char_node(p) {
                loop {
                    let f = mem_b16s1(p) as i32;
                    let c = mem_b16s0(p);
                    set_cur_h(
                        cur_h() + font_character_width(f, crate::scan::effective_char(true, f, c)),
                    );
                    let q = mem_b32s1(p);
                    set_mem_b32s1(p, l);
                    l = p;
                    p = q;
                    if !is_char_node(p) {
                        break;
                    }
                }
                continue;
            }

            let q = mem_b32s1(p);
            let mut add_width = true;

            'reswitch: loop {
                match mem_b16s1(p) {
                    HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                        set_rule_wd(mem_b32s1(p + 1));
                    }
                    WHATSIT_NODE => match mem_b16s0(p) {
                        NATIVE_WORD_NODE | NATIVE_WORD_NODE_AT | GLYPH_NODE | PIC_NODE
                        | PDF_NODE => {
                            set_rule_wd(mem_b32s1(p + 1));
                        }
                        _ => add_width = false,
                    },
                    GLUE_NODE => {
                        let mut g = mem_b32s1(p + 1);
                        set_rule_wd(mem_b32s1(g + 1) - *cur_g);

                        if g_sign != NORMAL {
                            if g_sign == STRETCHING {
                                if mem_b16s1(g) == g_order {
                                    *cur_glue += mem_b32s1(g + 2) as f64;
                                    let mut glue_temp = mem_gr(this_box + 6) * *cur_glue;
                                    glue_temp = glue_temp.clamp(-1_000_000_000.0, 1_000_000_000.0);
                                    *cur_g = crate::scaled_math::tex_round(glue_temp);
                                }
                            } else if mem_b16s0(g) == g_order {
                                *cur_glue -= mem_b32s1(g + 3) as f64;
                                let mut glue_temp = mem_gr(this_box + 6) * *cur_glue;
                                glue_temp = glue_temp.clamp(-1_000_000_000.0, 1_000_000_000.0);
                                *cur_g = crate::scaled_math::tex_round(glue_temp);
                            }
                        }

                        set_rule_wd(rule_wd() + *cur_g);

                        if (g_sign == STRETCHING && mem_b16s1(g) == g_order)
                            || (g_sign == SHRINKING && mem_b16s0(g) == g_order)
                        {
                            if mem_b32s1(g) == TEX_NULL {
                                free_node(g, GLUE_SPEC_SIZE);
                            } else {
                                set_mem_b32s1(g, mem_b32s1(g) - 1);
                            }

                            if mem_b16s0(p) < A_LEADERS {
                                set_mem_b16s1(p, KERN_NODE);
                                set_mem_b32s1(p + 1, rule_wd());
                            } else {
                                g = get_node(GLUE_SPEC_SIZE);
                                set_mem_b16s1(g, FILLL + 1);
                                set_mem_b16s0(g, FILLL + 1);
                                set_mem_b32s1(g + 1, rule_wd());
                                set_mem_b32s1(g + 2, 0);
                                set_mem_b32s1(g + 3, 0);
                                set_mem_b32s1(p + 1, g);
                            }
                        }
                    }
                    LIGATURE_NODE => {
                        crate::mem::flush_list(mem_b32s1(p + 1));
                        set_temp_ptr(p);
                        p = get_avail();
                        set_mem_word(p, mem_word(temp_ptr() + 1));
                        set_mem_b32s1(p, q);
                        free_node(temp_ptr(), SMALL_NODE_SIZE);
                        continue 'reswitch;
                    }
                    MATH_NODE => {
                        set_rule_wd(mem_b32s1(p + 1));

                        if mem_b16s0(p) % 2 != 0 {
                            if mem_b32s0(lr_ptr()) as u16 != math_node_end_lr_type(p) {
                                set_mem_b16s1(p, KERN_NODE);
                                set_lr_problems(lr_problems() + 1);
                            } else {
                                set_temp_ptr(lr_ptr());
                                set_lr_ptr(mem_b32s1(temp_ptr()));
                                set_mem_b32s1(temp_ptr(), avail());
                                set_avail(temp_ptr());

                                if n > MIN_HALFWORD {
                                    n -= 1;
                                    set_mem_b16s0(p, mem_b16s0(p) - 1);
                                } else {
                                    set_mem_b16s1(p, KERN_NODE);

                                    if m > MIN_HALFWORD {
                                        m -= 1;
                                    } else {
                                        free_node(p, MEDIUM_NODE_SIZE);
                                        set_mem_b32s1(t, q);
                                        set_mem_b32s1(t + 1, rule_wd());
                                        set_mem_b32s1(t + 2, -cur_h() - rule_wd());
                                        return l;
                                    }
                                }
                            }
                        } else {
                            set_temp_ptr(get_avail());
                            set_mem_b32s0(temp_ptr(), math_node_end_lr_type(p) as i32);
                            set_mem_b32s1(temp_ptr(), lr_ptr());
                            set_lr_ptr(temp_ptr());

                            if n > MIN_HALFWORD || math_node_lr_dir(p) as i16 != cur_dir() {
                                n += 1;
                                set_mem_b16s0(p, mem_b16s0(p) + 1);
                            } else {
                                set_mem_b16s1(p, KERN_NODE);
                                m += 1;
                            }
                        }
                    }
                    EDGE_NODE => {
                        crate::errors::confusion(c"LR2".as_ptr());
                    }
                    _ => add_width = false,
                }
                break;
            }

            if add_width {
                set_cur_h(cur_h() + rule_wd());
            }

            set_mem_b32s1(p, l);

            if mem_b16s1(p) == KERN_NODE && (rule_wd() == 0 || l == TEX_NULL) {
                free_node(p, MEDIUM_NODE_SIZE);
                p = l;
            }

            l = p;
            p = q;
        }

        if t == TEX_NULL && m == MIN_HALFWORD && n == MIN_HALFWORD {
            return l;
        }

        p = new_math(0, mem_b32s0(lr_ptr()) as i16);
        set_lr_problems(lr_problems() + 10000);
    }
}

// translated from xetex-shipout.c:1763 (movement)
// Called from hlist_out and vlist_out (still in C) → #[no_mangle].
#[no_mangle]
pub unsafe extern "C" fn movement(mut w: i32, o: u8) {
    let mut q = crate::mem::get_node(MOVEMENT_NODE_SIZE);
    set_mem_b32s1(q + 1, w);
    set_mem_b32s1(q + 2, dvi_offset + dvi_ptr);

    if o == DOWN1 {
        set_mem_b32s1(q, down_ptr);
        down_ptr = q;
    } else {
        set_mem_b32s1(q, right_ptr);
        right_ptr = q;
    }

    let mut p = mem_b32s1(q);
    let mut mstate: i32 = MOV_NONE_SEEN;

    // 'search either breaks with Some(found_node) or falls through with None.
    let found_p: Option<i32> = 'search: {
        while p != TEX_NULL {
            if mem_b32s1(p + 1) == w {
                match mstate + mem_b32s0(p) {
                    // NONE_SEEN+YZ_OK=3, NONE_SEEN+Y_OK=4, Z_SEEN+YZ_OK=15, Z_SEEN+Y_OK=16
                    3 | 4 | 15 | 16 => {
                        if mem_b32s1(p + 2) < dvi_gone {
                            break 'search None;
                        }
                        let mut k = mem_b32s1(p + 2) - dvi_offset;
                        if k < 0 {
                            k += DVI_BUF_SIZE;
                        }
                        *dvi_buf.add(k as usize) += 5;
                        set_mem_b32s0(p, MOV_Y_HERE);
                        break 'search Some(p);
                    }
                    // NONE_SEEN+Z_OK=5, Y_SEEN+YZ_OK=9, Y_SEEN+Z_OK=11
                    5 | 9 | 11 => {
                        if mem_b32s1(p + 2) < dvi_gone {
                            break 'search None;
                        }
                        let mut k = mem_b32s1(p + 2) - dvi_offset;
                        if k < 0 {
                            k += DVI_BUF_SIZE;
                        }
                        *dvi_buf.add(k as usize) += 10;
                        set_mem_b32s0(p, MOV_Z_HERE);
                        break 'search Some(p);
                    }
                    // NONE_SEEN+Y_HERE=1, NONE_SEEN+Z_HERE=2, Y_SEEN+Z_HERE=8, Z_SEEN+Y_HERE=13
                    1 | 2 | 8 | 13 => break 'search Some(p),
                    _ => {}
                }
            } else {
                match mstate + mem_b32s0(p) {
                    1 => mstate = MOV_Y_SEEN,     // NONE_SEEN+Y_HERE
                    2 => mstate = MOV_Z_SEEN,     // NONE_SEEN+Z_HERE
                    8 | 13 => break 'search None, // Y_SEEN+Z_HERE, Z_SEEN+Y_HERE
                    _ => {}
                }
            }
            p = mem_b32s1(p); // LLIST_link(p)
        }
        None
    };

    match found_p {
        None => {
            // not_found: emit the shortest encoding
            set_mem_b32s0(q, MOV_YZ_OK);
            if w.abs() >= 0x800000 {
                dvi_out(o.wrapping_add(3));
                dvi_four(w);
                return;
            }
            if w.abs() >= 0x8000 {
                dvi_out(o.wrapping_add(2));
                if w < 0 {
                    w += 0x1000000;
                }
                dvi_out((w / 0x10000) as u8); // lab2
                w %= 0x10000;
                dvi_out((w / 256) as u8); // lab1
                dvi_out((w % 256) as u8);
                return;
            }
            if w.abs() >= 128 {
                dvi_out(o.wrapping_add(1));
                if w < 0 {
                    w += 0x10000;
                }
                dvi_out((w / 256) as u8); // lab2
                dvi_out((w % 256) as u8); // lab1
                return;
            }
            dvi_out(o);
            if w < 0 {
                w += 256;
            }
            dvi_out((w % 256) as u8); // lab1
        }
        Some(p) => {
            // found: reuse previous y/z command
            set_mem_b32s0(q, mem_b32s0(p));
            if mem_b32s0(q) == MOV_Y_HERE {
                dvi_out(o.wrapping_add(4));
                while mem_b32s1(q) != p {
                    q = mem_b32s1(q);
                    match mem_b32s0(q) {
                        MOV_YZ_OK => set_mem_b32s0(q, MOV_Z_OK),
                        MOV_Y_OK => set_mem_b32s0(q, MOV_D_FIXED),
                        _ => {}
                    }
                }
            } else {
                dvi_out(o.wrapping_add(9));
                while mem_b32s1(q) != p {
                    q = mem_b32s1(q);
                    match mem_b32s0(q) {
                        MOV_YZ_OK => set_mem_b32s0(q, MOV_Y_OK),
                        MOV_Z_OK => set_mem_b32s0(q, MOV_D_FIXED),
                        _ => {}
                    }
                }
            }
        }
    }
}

// translated from xetex-shipout.c:1930 (special_out)
// Only called from out_what (Rust) → private.
unsafe fn special_out(p: i32) {
    if cur_h() != dvi_h {
        movement(cur_h() - dvi_h, RIGHT1);
        dvi_h = cur_h();
    }
    if cur_v() != dvi_v {
        movement(cur_v() - dvi_v, DOWN1);
        dvi_v = cur_v();
    }
    set_doing_special(true);
    let old_setting = selector();
    set_selector(SELECTOR_NEW_STRING);
    crate::output::show_token_list(
        mem_b32s1(mem_b32s1(p + 1)),
        TEX_NULL,
        pool_size() - pool_ptr(),
    );
    set_selector(old_setting);

    if pool_ptr() + 1 > pool_size() {
        crate::errors::overflow(c"pool size".as_ptr(), pool_size() - init_pool_ptr());
    }

    let cur_len = pool_ptr() - str_start(str_ptr() - TOO_BIG_CHAR);
    if cur_len < 256 {
        dvi_out(XXX1);
        dvi_out(cur_len as u8);
    } else {
        dvi_out(XXX4);
        dvi_four(cur_len);
    }

    let mut k = str_start(str_ptr() - TOO_BIG_CHAR);
    let for_end = pool_ptr() - 1;
    while k <= for_end {
        dvi_out(str_pool_entry(k) as u8);
        k += 1;
    }
    set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
    set_doing_special(false);
}

// translated from xetex-shipout.c:1976 (write_out)
// Only called from out_what (Rust) → private.
unsafe fn write_out(p: i32) {
    let q = crate::mem::get_avail();
    set_mem_b32s0(q, RIGHT_BRACE_TOKEN + b'}' as i32);
    let r = crate::mem::get_avail();
    set_mem_b32s1(q, r);
    set_mem_b32s0(r, CS_TOKEN_FLAG + END_WRITE);
    crate::input::begin_token_list(q, INSERTED);
    crate::input::begin_token_list(mem_b32s1(p + 1), WRITE_TEXT);
    let q2 = crate::mem::get_avail();
    set_mem_b32s0(q2, LEFT_BRACE_TOKEN + b'{' as i32);
    crate::input::begin_token_list(q2, INSERTED);

    let old_mode = cur_list_mode();
    set_cur_list_mode(0);
    set_cur_cs(write_loc());
    crate::scan::scan_toks(false, true);
    crate::scan::get_token();

    if cur_tok() != CS_TOKEN_FLAG + END_WRITE {
        crate::output::error_here_with_diagnostic(c"Unbalanced write command".as_ptr());
        crate::output::capture_to_diagnostic(core::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"On this page there's a \\write with fewer real {'s than }'s.".as_ptr(),
        );
        set_help_line(0, c"I can't handle that very well; good luck.".as_ptr());
        crate::errors::error();
        loop {
            crate::scan::get_token();
            if cur_tok() == CS_TOKEN_FLAG + END_WRITE {
                break;
            }
        }
    }

    set_cur_list_mode(old_mode);
    crate::input::end_token_list();
    let old_setting = selector();
    let j = mem_b32s0(p + 1);

    if j == 18 {
        set_selector(SELECTOR_NEW_STRING);
    } else if write_open(j as usize) {
        set_selector(j);
    } else {
        if j == 17 && selector() == SELECTOR_TERM_AND_LOG {
            set_selector(SELECTOR_LOG_ONLY);
        }
        crate::output::print_nl_cstr(c"".as_ptr());
    }

    crate::output::token_show(def_ref());
    crate::output::print_ln();
    crate::mem::flush_list(def_ref());

    if j == 18 {
        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) <= 0 {
            set_selector(SELECTOR_LOG_ONLY);
        } else {
            set_selector(SELECTOR_TERM_AND_LOG);
        }
        if !log_opened() {
            set_selector(SELECTOR_TERM_ONLY);
        }

        // Tectonic: don't emit warnings for shell-escape when enabled;
        // the driver provides a better UX.
        if !shell_escape_enabled() {
            crate::output::diagnostic_begin_capture_warning_here();
            crate::output::print_nl_cstr(c"runsystem(".as_ptr());
            let base = str_start(str_ptr() - TOO_BIG_CHAR);
            let cur_len = pool_ptr() - base;
            for d in 0..cur_len {
                crate::output::print(str_pool_entry(base + d) as i32);
            }
            crate::output::print_cstr(c")...".as_ptr());
            crate::output::print_cstr(c"disabled".as_ptr());
            crate::output::print_char(b'.' as i32);
            crate::output::capture_to_diagnostic(core::ptr::null_mut());
            crate::output::print_nl_cstr(c"".as_ptr());
            crate::output::print_ln();
        } else {
            let base = str_start(str_ptr() - TOO_BIG_CHAR);
            let cur_len = pool_ptr() - base;
            ttstub_shell_escape(str_pool_ptr(base), cur_len as libc::size_t);
        }

        set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
    }

    set_selector(old_setting);
}

// translated from xetex-shipout.c:2071 (pic_out)
// Called from hlist_out (still in C) → #[no_mangle].
#[no_mangle]
pub unsafe extern "C" fn pic_out(p: i32) {
    if cur_h() != dvi_h {
        movement(cur_h() - dvi_h, RIGHT1);
        dvi_h = cur_h();
    }
    if cur_v() != dvi_v {
        movement(cur_v() - dvi_v, DOWN1);
        dvi_v = cur_v();
    }

    let old_setting = selector();
    set_selector(SELECTOR_NEW_STRING);
    crate::output::print_cstr(c"pdf:image ".as_ptr());
    crate::output::print_cstr(c"matrix ".as_ptr());
    crate::output::print_scaled(mem_b32s0(p + 5));
    crate::output::print(b' ' as i32);
    crate::output::print_scaled(mem_b32s1(p + 5));
    crate::output::print(b' ' as i32);
    crate::output::print_scaled(mem_b32s0(p + 6));
    crate::output::print(b' ' as i32);
    crate::output::print_scaled(mem_b32s1(p + 6));
    crate::output::print(b' ' as i32);
    crate::output::print_scaled(mem_b32s0(p + 7));
    crate::output::print(b' ' as i32);
    crate::output::print_scaled(mem_b32s1(p + 7));
    crate::output::print(b' ' as i32);
    crate::output::print_cstr(c"page ".as_ptr());
    crate::output::print_int(mem_b16s0(p + 4) as i32);
    crate::output::print(b' ' as i32);

    match mem_b16s1(p + 8) {
        1 => crate::output::print_cstr(c"pagebox cropbox ".as_ptr()),
        2 => crate::output::print_cstr(c"pagebox mediabox ".as_ptr()),
        3 => crate::output::print_cstr(c"pagebox bleedbox ".as_ptr()),
        4 => crate::output::print_cstr(c"pagebox trimbox ".as_ptr()),
        5 => crate::output::print_cstr(c"pagebox artbox ".as_ptr()),
        _ => {}
    }

    crate::output::print(b'(' as i32);
    let path_len = mem_b16s1(p + 4) as i32;
    let path_ptr = crate::globals::mem_node_addr(p + PIC_NODE_SIZE) as *const u8;
    for i in 0..path_len {
        crate::output::print_raw_char(*path_ptr.add(i as usize) as u16, true);
    }
    crate::output::print(b')' as i32);

    set_selector(old_setting);
    let cur_len = pool_ptr() - str_start(str_ptr() - TOO_BIG_CHAR);
    if cur_len < 256 {
        dvi_out(XXX1);
        dvi_out(cur_len as u8);
    } else {
        dvi_out(XXX4);
        dvi_four(cur_len);
    }

    let mut k = str_start(str_ptr() - TOO_BIG_CHAR);
    while k < pool_ptr() {
        dvi_out(str_pool_entry(k) as u8);
        k += 1;
    }
    set_pool_ptr(str_start(str_ptr() - TOO_BIG_CHAR));
}

// translated from xetex-shipout.c:1682 (out_what)
// Declared in xetex-xetexd.h (external linkage) → #[no_mangle].
#[no_mangle]
pub unsafe extern "C" fn out_what(p: i32) {
    match mem_b16s0(p) {
        OPEN_NODE | WRITE_NODE | CLOSE_NODE => {
            if !doing_leaders() {
                let j = mem_b32s0(p + 1);
                if mem_b16s0(p) == WRITE_NODE {
                    write_out(p);
                    return;
                }
                if write_open(j as usize) {
                    ttstub_output_close(write_file(j as usize));
                }
                if mem_b16s0(p) == CLOSE_NODE {
                    set_write_open(j as usize, false);
                    return;
                }
                // OPEN_NODE
                if j < 16 {
                    set_cur_name(mem_b32s1(p + 1));
                    set_cur_area(mem_b32s0(p + 2));
                    set_cur_ext(mem_b32s1(p + 2));
                    if crate::string_pool::length(cur_ext()) == 0 {
                        set_cur_ext(crate::texmfmp::maketexstring(c".tex".as_ptr()));
                    }
                    crate::scan::pack_file_name(cur_name(), cur_area(), cur_ext());
                    let handle = ttstub_output_open(name_of_file(), 0);
                    if handle == INVALID_HANDLE {
                        let mut buf = [0u8; 512];
                        libc::snprintf(
                            buf.as_mut_ptr() as *mut libc::c_char,
                            buf.len(),
                            c"cannot open output file \"%s\"".as_ptr(),
                            name_of_file(),
                        );
                        _tt_abort(buf.as_ptr() as *const libc::c_char);
                    }
                    set_write_file(j as usize, handle);
                    set_write_open(j as usize, true);
                    if log_opened() {
                        let old_setting = selector();
                        if eqtb_b32s1(INT_BASE + INT_PAR_TRACING_ONLINE) <= 0 {
                            set_selector(SELECTOR_LOG_ONLY);
                        } else {
                            set_selector(SELECTOR_TERM_AND_LOG);
                        }
                        crate::output::print_nl_cstr(c"\\openout".as_ptr());
                        crate::output::print_int(j);
                        crate::output::print_cstr(c" = `".as_ptr());
                        crate::output::print_file_name(cur_name(), cur_area(), cur_ext());
                        crate::output::print_cstr(c"'.".as_ptr());
                        crate::output::print_nl_cstr(c"".as_ptr());
                        crate::output::print_ln();
                        set_selector(old_setting);
                    }
                }
            }
        }
        SPECIAL_NODE => special_out(p),
        LANGUAGE_NODE => {}
        _ => crate::errors::confusion(c"ext4".as_ptr()),
    }
}
