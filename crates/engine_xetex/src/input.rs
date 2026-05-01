//! Input-stack management functions translated from `xetex/xetex-xetex0.c`.
//!
//! Phase A: all state accessed through the C globals via `src/globals.rs`.

use crate::globals::{
    align_state, avail, buf_size, buffer_entry, cond_ptr, cur_boundary, cur_cs, cur_input, cur_tok,
    first, in_open, input_file, input_ptr, input_stack, last, line, line_stack, max_buf_stack,
    max_in_open, max_in_stack, mem_b16s0, mem_b16s1, mem_b16s2, mem_b16s3, mem_b32s0, mem_b32s1,
    param_ptr, param_stack, pseudo_files, set_align_state, set_avail, set_buffer_entry,
    set_cur_input, set_cur_input_index, set_cur_input_limit, set_cur_input_loc, set_cur_input_name,
    set_cur_input_start, set_cur_input_state, set_cur_input_synctex_tag, set_cur_tok, set_eof_seen,
    set_first, set_full_source_filename_stack, set_grp_stack, set_if_stack, set_in_open,
    set_input_file, set_input_ptr, set_input_stack, set_last, set_line, set_line_stack,
    set_max_buf_stack, set_max_in_stack, set_mem_b32s0, set_mem_b32s1, set_param_ptr,
    set_pseudo_files, set_source_filename_stack, stack_size,
};

use crate::output::print_cmd_chr;

extern "C" {
    fn u_close(f: *mut libc::c_void);
    fn capture_to_diagnostic(diag: *mut std::ffi::c_void);
}

// Input-state discriminant constants (xetex-constants.h:200–206, 261).
const TOKEN_LIST: u16 = 0;
const U_TEMPLATE: u16 = 1;
const V_TEMPLATE: u16 = 2;
const BACKED_UP: u16 = 3;
const INSERTED: u16 = 5;
const MACRO: u16 = 6;
const OUTPUT_TEXT: u16 = 7;    // xetex-constants.h:206
const MARK_TEXT: u16 = 15;     // xetex-constants.h:214
const WRITE_TEXT: u16 = 18;    // xetex-constants.h:217
// Token encoding (xetex-constants.h:348, xetex_format.h:201, xetex-constants.h:62).
const CS_TOKEN_FLAG: i32 = 0x1FFFFFF;
const FROZEN_RELAX: i32 = 2_243_226 + 7; // FROZEN_CONTROL_SEQUENCE + 7
// Scanner-state constants (xetex-constants.h:221).
const MID_LINE: u16 = 1;

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

// ---------------------------------------------------------------------------
// pseudo_close
// ---------------------------------------------------------------------------

/// Close the top pseudo-file, freeing its node chain.
///
/// translated from xetex-xetex0.c:2766 (pseudo_close)
#[no_mangle]
pub unsafe extern "C" fn pseudo_close() {
    let p = mem_b32s1(pseudo_files());
    let mut q = mem_b32s0(pseudo_files());
    // Free the pseudo_files head node onto the avail list.
    set_mem_b32s1(pseudo_files(), avail());
    set_avail(pseudo_files());
    set_pseudo_files(p);
    // Walk the chain and free each block node.
    while q != crate::mem::TEX_NULL {
        let next = mem_b32s1(q);
        crate::mem::free_node(q, mem_b32s0(q));
        q = next;
    }
}

// ---------------------------------------------------------------------------
// begin_file_reading
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:3097 (begin_file_reading)
#[no_mangle]
pub unsafe extern "C" fn begin_file_reading() {
    if in_open() == max_in_open() {
        crate::errors::overflow(c"text input levels".as_ptr(), max_in_open());
    }
    if first() == buf_size() {
        crate::errors::overflow(c"buffer size".as_ptr(), buf_size());
    }
    set_in_open(in_open() + 1);
    let ptr = input_ptr();
    if ptr > max_in_stack() {
        set_max_in_stack(ptr);
        if ptr == stack_size() {
            crate::errors::overflow(c"input stack size".as_ptr(), stack_size());
        }
    }
    set_input_stack(ptr, cur_input());
    set_input_ptr(ptr + 1);
    let idx = in_open();
    set_cur_input_index(idx as u16);
    set_source_filename_stack(idx, 0);
    set_full_source_filename_stack(idx, 0);
    set_eof_seen(idx, false);
    set_grp_stack(idx, cur_boundary());
    set_if_stack(idx, cond_ptr());
    set_line_stack(idx, line());
    set_cur_input_start(first());
    set_cur_input_state(MID_LINE);
    set_cur_input_name(0);
    set_cur_input_synctex_tag(0);
}

// ---------------------------------------------------------------------------
// end_file_reading
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:3127 (end_file_reading)
#[no_mangle]
pub unsafe extern "C" fn end_file_reading() {
    set_first(cur_input().start);
    set_line(line_stack(cur_input().index as i32));
    let name = cur_input().name;
    if name == 18 || name == 19 {
        pseudo_close();
    } else if name > 17 {
        let idx = cur_input().index as i32;
        u_close(input_file(idx));
        set_input_file(idx, std::ptr::null_mut());
    }
    let ptr = input_ptr() - 1;
    set_input_ptr(ptr);
    set_cur_input(input_stack(ptr));
    set_in_open(in_open() - 1);
}

// ---------------------------------------------------------------------------
// pseudo_input
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:2722 (pseudo_input)
#[no_mangle]
pub unsafe extern "C" fn pseudo_input() -> bool {
    set_last(first());
    let p = mem_b32s0(pseudo_files());
    if p == crate::mem::TEX_NULL {
        return false;
    }
    set_mem_b32s0(pseudo_files(), mem_b32s1(p));
    let sz = mem_b32s0(p);
    if 4 * sz - 3 >= buf_size() - last() {
        set_cur_input_loc(first());
        set_cur_input_limit(last() - 1);
        crate::errors::overflow(c"buffer size".as_ptr(), buf_size());
    }
    set_last(first());
    let mut r = p + 1;
    let for_end = p + sz - 1;
    while r <= for_end {
        let w0 = mem_b16s3(r) as i32;
        let w1 = mem_b16s2(r) as i32;
        let w2 = mem_b16s1(r) as i32;
        let w3 = mem_b16s0(r) as i32;
        set_buffer_entry(last(), w0);
        set_buffer_entry(last() + 1, w1);
        set_buffer_entry(last() + 2, w2);
        set_buffer_entry(last() + 3, w3);
        set_last(last() + 4);
        r += 1;
    }
    if last() >= max_buf_stack() {
        set_max_buf_stack(last() + 1);
    }
    while last() > first() && buffer_entry(last() - 1) == ' ' as i32 {
        set_last(last() - 1);
    }
    crate::mem::free_node(p, sz);
    true
}

// ---------------------------------------------------------------------------
// insert_relax
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:3998 (insert_relax)
#[no_mangle]
pub unsafe extern "C" fn insert_relax() {
    set_cur_tok(CS_TOKEN_FLAG + cur_cs());
    back_input();
    set_cur_tok(CS_TOKEN_FLAG + FROZEN_RELAX);
    back_input();
    set_cur_input_index(INSERTED);
}

// ---------------------------------------------------------------------------
// begin_token_list
// ---------------------------------------------------------------------------

// INT_BASE + INT_PAR__tracing_macros (xetex_format.h:208, :65)
const INT_BASE_BTL: i32 = 7_826_729;
const INT_PAR_TRACING_MACROS: i32 = 30; // xetex_format.h:65
const LOCAL_BASE_BTL: i32 = 2_254_871;  // xetex_format.h:208
const LOCAL__OUTPUT_ROUTINE_BTL: i32 = 1; // xetex_format.h:173
const ASSIGN_TOKS_BTL: u16 = 73;        // xetex_format.h:313

/// translated from xetex-xetex0.c:2970 (begin_token_list)
#[no_mangle]
pub unsafe extern "C" fn begin_token_list(p: i32, t: u16) {
    if input_ptr() > max_in_stack() {
        set_max_in_stack(input_ptr());
        if input_ptr() == stack_size() {
            crate::errors::overflow(c"input stack size".as_ptr(), stack_size());
        }
    }
    set_input_stack(input_ptr(), cur_input());
    set_input_ptr(input_ptr() + 1);
    set_cur_input_state(TOKEN_LIST);
    set_cur_input_start(p);
    set_cur_input_index(t);
    if t >= MACRO {
        set_mem_b32s0(p, mem_b32s0(p) + 1); // increment ref count
        if t == MACRO {
            set_cur_input_limit(param_ptr());
        } else {
            set_cur_input_loc(mem_b32s1(p));
            if crate::globals::eqtb_b32s1(INT_BASE_BTL + INT_PAR_TRACING_MACROS) > 1 {
                crate::output::begin_diagnostic();
                crate::output::diagnostic_begin_capture_warning_here();
                crate::output::print_nl_cstr(c"".as_ptr());
                match t {
                    MARK_TEXT => crate::output::print_esc_cstr(c"mark".as_ptr()),
                    WRITE_TEXT => crate::output::print_esc_cstr(c"write".as_ptr()),
                    _ => print_cmd_chr(
                        ASSIGN_TOKS_BTL,
                        t as i32 + LOCAL_BASE_BTL + LOCAL__OUTPUT_ROUTINE_BTL - OUTPUT_TEXT as i32,
                    ),
                }
                crate::output::print_cstr(c"->".as_ptr());
                crate::output::token_show(p);
                capture_to_diagnostic(std::ptr::null_mut());
                crate::output::end_diagnostic(false);
            }
        }
    } else {
        set_cur_input_loc(p);
    }
}
