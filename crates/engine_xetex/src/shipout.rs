//! Shipout helpers translated from `xetex/xetex-shipout.c`.

use crate::globals::{
    eqtb_b32s1, font_area_entry, font_check_entry, font_dsize_entry, font_name_entry,
    font_ptr, font_size_entry, font_used_entry, last_bop, max_h, max_push, max_v, mem_b32s1,
    semantic_pagination_enabled, set_font_ptr, set_last_bop, set_mem_b16s0, set_mem_b16s1,
    set_mem_b32s1, set_total_pages, str_pool_entry, str_start, total_pages, xdv_buffer_entry,
};
use crate::mem::{free_node, get_node};

extern "C" {
    fn make_font_def(f: i32) -> libc::c_int;
    fn xmalloc(size: libc::size_t) -> *mut libc::c_void;
    fn ttstub_output_write(
        handle: usize,
        data: *const libc::c_char,
        len: libc::size_t,
    ) -> libc::size_t;
    fn ttstub_output_close(handle: usize) -> libc::c_int;
    fn _tt_abort(fmt: *const libc::c_char) -> !;
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

// xetex-shipout.c:11–12
const DVI_BUF_SIZE: i32 = 16384;
const HALF_BUF: i32 = 8192;
const TEX_INFINITY: i32 = 0x7FFF_FFFF;
const TEX_NULL: i32 = -0x0FFF_FFFF;
// xetex-constants.h:240
const POP: u8 = 142;

// xetex-constants.h:105,114. EDGE_NODE is an alias of STYLE_NODE.
const EDGE_NODE: u16 = 14;
const EDGE_NODE_SIZE: i32 = 3;
const AAT_FONT_FLAG: i32 = 65533;
const OTGR_FONT_FLAG: i32 = 65534;
const FNT_DEF1: u8 = 243;
const DEFINE_NATIVE_FONT: u8 = 252;
const TOO_BIG_CHAR: i32 = 65536;

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

// translated from xetex-shipout.c:2332 (dvi_pop)
#[no_mangle]
pub unsafe extern "C" fn dvi_pop(l: i32) {
    if l == dvi_offset + dvi_ptr && dvi_ptr > 0 {
        dvi_ptr -= 1;
    } else {
        dvi_out(POP);
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
// scan.rs FONT_BASE
const FONT_BASE: i32 = 0;

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
