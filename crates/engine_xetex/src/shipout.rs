//! Shipout helpers translated from `xetex/xetex-shipout.c`.

use crate::globals::{
    font_area_entry, font_check_entry, font_dsize_entry, font_name_entry, font_size_entry,
    set_mem_b16s0, set_mem_b16s1, set_mem_b32s1, str_pool_entry, str_start, xdv_buffer_entry,
};
use crate::mem::get_node;

extern "C" {
    fn dvi_out(c: u8);
    fn make_font_def(f: i32) -> libc::c_int;
}

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
