//! Input-stack management functions translated from `xetex/xetex-xetex0.c`.
//!
//! Phase A: all state accessed through the C globals via `src/globals.rs`.

use crate::globals::{
    align_state, cur_input, cur_tok, input_ptr, input_stack, max_in_stack, param_ptr, param_stack,
    set_align_state, set_cur_input, set_cur_input_index, set_cur_input_loc, set_cur_input_start,
    set_cur_input_state, set_input_ptr, set_input_stack, set_max_in_stack, set_param_ptr,
    stack_size,
};

// Input-state discriminant constants (xetex-constants.h:200–206, 261).
const TOKEN_LIST: u16 = 0;
const U_TEMPLATE: u16 = 1;
const V_TEMPLATE: u16 = 2;
const BACKED_UP: u16 = 3;
const INSERTED: u16 = 5;
const MACRO: u16 = 6;

// Brace-token limits for align_state tracking (xetex-constants.h:350,352).
const LEFT_BRACE_LIMIT: i32 = 0x40_0000;
const RIGHT_BRACE_LIMIT: i32 = 0x60_0000;

// ---------------------------------------------------------------------------
// end_token_list
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:3108 (end_token_list)
#[no_mangle]
pub unsafe extern "C" fn end_token_list() {
    let ci = cur_input();
    if ci.index >= BACKED_UP {
        if ci.index <= INSERTED {
            crate::mem::flush_list(ci.start);
        } else {
            crate::mem::delete_token_ref(ci.start);
            if ci.index == MACRO {
                while param_ptr() > ci.limit {
                    set_param_ptr(param_ptr() - 1);
                    crate::mem::flush_list(param_stack(param_ptr()));
                }
            }
        }
    } else if ci.index == U_TEMPLATE {
        if align_state() > 500_000 {
            set_align_state(0);
        } else {
            crate::errors::fatal_error(c"(interwoven alignment preambles are not allowed)".as_ptr());
        }
    }
    let ptr = input_ptr() - 1;
    set_input_ptr(ptr);
    set_cur_input(input_stack(ptr));
}

// ---------------------------------------------------------------------------
// back_input
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:3136 (back_input)
#[no_mangle]
pub unsafe extern "C" fn back_input() {
    while cur_input().state == TOKEN_LIST
        && cur_input().loc == crate::mem::TEX_NULL
        && cur_input().index != V_TEMPLATE
    {
        end_token_list();
    }
    let p = crate::mem::get_avail();
    crate::globals::set_mem_b32s0(p, cur_tok());
    if cur_tok() < RIGHT_BRACE_LIMIT {
        if cur_tok() < LEFT_BRACE_LIMIT {
            set_align_state(align_state() - 1);
        } else {
            set_align_state(align_state() + 1);
        }
    }
    let ptr = input_ptr();
    if ptr > max_in_stack() {
        set_max_in_stack(ptr);
        if ptr == stack_size() {
            crate::errors::overflow(c"input stack size".as_ptr(), stack_size());
        }
    }
    set_input_stack(ptr, cur_input());
    set_input_ptr(ptr + 1);
    set_cur_input_state(TOKEN_LIST);
    set_cur_input_start(p);
    set_cur_input_index(BACKED_UP);
    set_cur_input_loc(p);
}

// ---------------------------------------------------------------------------
// back_error
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:3167 (back_error)
#[no_mangle]
pub unsafe extern "C" fn back_error() {
    back_input();
    crate::errors::error();
}

// ---------------------------------------------------------------------------
// ins_error
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:3174 (ins_error)
#[no_mangle]
pub unsafe extern "C" fn ins_error() {
    back_input();
    set_cur_input_index(INSERTED);
    crate::errors::error();
}
