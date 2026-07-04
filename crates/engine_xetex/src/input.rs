//! Input-stack management functions translated from `xetex/xetex-xetex0.c`.
use crate::consts::{
    BACKED_UP, CS_TOKEN_FLAG, INSERTED, INT_BASE, LOCAL_BASE, OUTPUT_TEXT, TOKEN_LIST, WRITE_TEXT,
};
use crate::io::u_close;


extern "C" {
}

// Input-state discriminant constants (xetex-constants.h:200–206, 261).
const U_TEMPLATE: u16 = 1;
const V_TEMPLATE: u16 = 2;
const MACRO: u16 = 6;
const MARK_TEXT: u16 = 15; // xetex-constants.h:214
// Token encoding (xetex-constants.h:348, xetex_format.h:201, xetex-constants.h:62).
const FROZEN_RELAX: i32 = 2_243_226 + 7; // FROZEN_CONTROL_SEQUENCE + 7
                                         // Scanner-state constants (xetex-constants.h:221).
const MID_LINE: u16 = 1;

// Brace-token limits for align_state tracking (xetex-constants.h:350,352).
const LEFT_BRACE_LIMIT: i32 = 0x40_0000;
const RIGHT_BRACE_LIMIT: i32 = 0x60_0000;

// ---------------------------------------------------------------------------
// InputAbort — cold-path sentinel for the input-stack layer
// ---------------------------------------------------------------------------

/// Cold abort/divergent paths. The caller propagates the sentinel as an
/// [`EngineAbort`](crate::errors::EngineAbort) via
/// [`into_engine_abort`](InputAbort::into_engine_abort). Never reached on a
/// passing run.
pub(crate) enum InputAbort {
    /// `overflow(what, n)` — capacity exhaustion (input stack / buffer).
    Overflow { what: *const libc::c_char, n: i32 },
    /// `fatal_error(msg)` — interwoven alignment preambles.
    Fatal(*const libc::c_char),
    /// `_tt_abort(msg)` — direct longjmp abort (input-buffer overflow / invalid
    /// file input mode). Unlike `Fatal` this does not print a TeX error first.
    TtAbort(*const libc::c_char),
    /// Main-memory arena exhaustion (the `get_avail`/`get_node` `ALLOC_FAILED`
    /// sentinel surfaced into the input layer).
    MainMemory,
    /// Serious internal error closing an input file at end-of-file (the C
    /// `ttstub_input_close` wrapper `longjmp`'d here) — reached from
    /// `end_file_reading_in`'s `u_close`.
    InputClose,
}

impl InputAbort {
    /// Map to the unified [`EngineAbort`] terminal, without longjmp-ing.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            InputAbort::Overflow { what, n } => crate::errors::EngineAbort::Overflow { what, n },
            InputAbort::Fatal(msg) => crate::errors::EngineAbort::Fatal(msg),
            InputAbort::TtAbort(msg) => crate::errors::EngineAbort::TtAbort(msg),
            InputAbort::MainMemory => crate::mem::main_memory_overflow_abort(),
            InputAbort::InputClose => crate::errors::EngineAbort::TtAbort(
                c"serious internal error closing an input file".as_ptr(),
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// end_token_list
// ---------------------------------------------------------------------------

pub(crate) fn end_token_list_in(st: &mut crate::state::EngineState) -> Result<(), InputAbort> {
    let ci = st.input.cur_input;
    if ci.index >= BACKED_UP {
        if ci.index <= INSERTED {
            crate::mem::flush_list(&mut st.mem, ci.start);
        } else {
            crate::mem::delete_token_ref(&mut st.mem, ci.start);
            if ci.index == MACRO {
                while st.input.param_ptr > ci.limit {
                    st.input.param_ptr -= 1;
                    let p = st.input.param_stack[st.input.param_ptr as usize];
                    crate::mem::flush_list(&mut st.mem, p);
                }
            }
        }
    } else if ci.index == U_TEMPLATE {
        if st.scanner.align_state > 500_000 {
            st.scanner.align_state = 0;
        } else {
            return Err(InputAbort::Fatal(
                c"(interwoven alignment preambles are not allowed)".as_ptr(),
            ));
        }
    }
    let ptr = st.input.input_ptr - 1;
    st.input.input_ptr = ptr;
    st.input.cur_input = st.input.input_stack[ptr as usize];
    st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
    Ok(())
}

// ---------------------------------------------------------------------------
// back_input
// ---------------------------------------------------------------------------

pub(crate) fn back_input_in(st: &mut crate::state::EngineState) -> Result<(), InputAbort> {
    while st.input.cur_input.state == TOKEN_LIST
        && st.input.cur_input.loc == crate::mem::TEX_NULL
        && st.input.cur_input.index != V_TEMPLATE
    {
        end_token_list_in(st)?;
    }
    let p = crate::mem::get_avail(&mut st.mem);
    if p == crate::mem::ALLOC_FAILED {
        return Err(InputAbort::MainMemory);
    }
    st.mem.set_b32s0(p, st.scanner.cur_tok);
    if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
        if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
            st.scanner.align_state -= 1;
        } else {
            st.scanner.align_state += 1;
        }
    }
    let ptr = st.input.input_ptr;
    if ptr > st.input.max_in_stack {
        st.input.max_in_stack = ptr;
        if ptr == st.input.stack_size {
            return Err(InputAbort::Overflow {
                what: c"input stack size".as_ptr(),
                n: st.input.stack_size,
            });
        }
    }
    st.input.input_stack[ptr as usize] = st.input.cur_input;
    st.input.input_ptr = ptr + 1;
    st.input.cur_input.state = TOKEN_LIST;
    st.input.cur_input.start = p;
    st.input.cur_input.index = BACKED_UP;
    st.input.cur_input.loc = p;
    Ok(())
}

// ---------------------------------------------------------------------------
// back_error
// ---------------------------------------------------------------------------

/// [`back_input_in`] then [`error_in`].
pub(crate) fn back_error_in(
    st: &mut crate::state::EngineState,
) -> Result<crate::errors::ErrorOutcome, InputAbort> {
    back_input_in(st)?;
    Ok(crate::errors::error_in(st))
}

// ---------------------------------------------------------------------------
// ins_error
// ---------------------------------------------------------------------------

/// [`back_input_in`], stamp the backed-up token list as `INSERTED`, then
/// [`error_in`].
pub(crate) fn ins_error_in(
    st: &mut crate::state::EngineState,
) -> Result<crate::errors::ErrorOutcome, InputAbort> {
    back_input_in(st)?;
    st.input.cur_input.index = INSERTED;
    Ok(crate::errors::error_in(st))
}

// ---------------------------------------------------------------------------
// pseudo_close
// ---------------------------------------------------------------------------

/// Close the top pseudo-file, freeing its node chain.
///
/// translated from xetex-xetex0.c:2766 (pseudo_close)
///
/// Pure node recycling (no allocation).
pub(crate) fn pseudo_close(
    mem: &mut crate::state::MemState,
    input: &mut crate::state::InputClusterState,
) {
    let pf = input.pseudo_files.0;
    let p = mem.b32s1(pf);
    let mut q = mem.b32s0(pf);
    // Free the pseudo_files head node onto the avail list.
    let av = mem.avail;
    mem.set_b32s1(pf, av);
    mem.avail = pf;
    input.pseudo_files.0 = p;
    // Walk the chain and free each block node.
    while q != crate::mem::TEX_NULL {
        let next = mem.b32s1(q);
        let sz = mem.b32s0(q);
        crate::mem::free_node(mem, q, sz);
        q = next;
    }
}

// ---------------------------------------------------------------------------
// begin_file_reading
// ---------------------------------------------------------------------------

pub(crate) fn begin_file_reading_in(st: &mut crate::state::EngineState) -> Result<(), InputAbort> {
    if st.input.in_open == st.input.max_in_open {
        return Err(InputAbort::Overflow {
            what: c"text input levels".as_ptr(),
            n: st.input.max_in_open,
        });
    }
    if st.input.first == st.input.buf_size {
        return Err(InputAbort::Overflow {
            what: c"buffer size".as_ptr(),
            n: st.input.buf_size,
        });
    }
    st.input.in_open += 1;
    let ptr = st.input.input_ptr;
    if ptr > st.input.max_in_stack {
        st.input.max_in_stack = ptr;
        if ptr == st.input.stack_size {
            return Err(InputAbort::Overflow {
                what: c"input stack size".as_ptr(),
                n: st.input.stack_size,
            });
        }
    }
    st.input.input_stack[ptr as usize] = st.input.cur_input;
    st.input.input_ptr = ptr + 1;
    let idx = st.input.in_open as usize;
    st.input.cur_input.index = idx as u16;
    st.input.source_filename_stack[idx] = 0;
    st.input.full_source_filename_stack[idx] = 0;
    st.input.eof_seen[idx] = false;
    st.input.grp_stack[idx] = st.save_stack.cur_boundary;
    st.input.if_stack[idx] = st.scanner.cond_ptr.0;
    st.input.line_stack[idx] = st.mem.line;
    st.input.cur_input.start = st.input.first;
    st.input.cur_input.state = MID_LINE;
    st.input.cur_input.name = 0;
    st.input.cur_input.synctex_tag = 0;
    st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
    Ok(())
}

// ---------------------------------------------------------------------------
// end_file_reading
// ---------------------------------------------------------------------------

/// On `u_close`'s serious internal close error, the old C `ttstub_input_close`
/// longjmp aborted before the input-stack pop.
pub(crate) fn end_file_reading_in(st: &mut crate::state::EngineState) -> Result<(), InputAbort> {
    st.input.first = st.input.cur_input.start;
    st.mem.line = st.input.line_stack[st.input.cur_input.index as usize];
    let name = st.input.cur_input.name;
    if name == 18 || name == 19 {
        let s = &mut *st;
        pseudo_close(&mut s.mem, &mut s.input);
    } else if name > 17 {
        let idx = st.input.cur_input.index as usize;
        // SAFETY: FFI close of the open input file handle at this stack slot.
        if unsafe { u_close(st.input.input_file[idx]) } {
            return Err(InputAbort::InputClose);
        }
        st.input.input_file[idx] = std::ptr::null_mut();
    }
    let ptr = st.input.input_ptr - 1;
    st.input.input_ptr = ptr;
    st.input.cur_input = st.input.input_stack[ptr as usize];
    st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
    st.input.in_open -= 1;
    Ok(())
}

// ---------------------------------------------------------------------------
// pseudo_input
// ---------------------------------------------------------------------------

/// translated from xetex-xetex0.c:2722 (pseudo_input)
///
/// The buffer-size `overflow` restores `loc`/`limit` first, matching the
/// original.
pub(crate) fn pseudo_input_in(st: &mut crate::state::EngineState) -> Result<bool, InputAbort> {
    st.input.last = st.input.first;
    let p = st.mem.b32s0(st.input.pseudo_files);
    if p == crate::mem::TEX_NULL {
        return Ok(false);
    }
    let next = st.mem.b32s1(p);
    st.mem.set_b32s0(st.input.pseudo_files, next);
    let sz = st.mem.b32s0(p);
    if 4 * sz - 3 >= st.input.buf_size - st.input.last {
        st.input.cur_input.loc = st.input.first;
        st.input.cur_input.limit = st.input.last - 1;
        return Err(InputAbort::Overflow {
            what: c"buffer size".as_ptr(),
            n: st.input.buf_size,
        });
    }
    st.input.last = st.input.first;
    let mut r = p + 1;
    let for_end = p + sz - 1;
    while r <= for_end {
        let w0 = st.mem.b16s3(r) as i32;
        let w1 = st.mem.b16s2(r) as i32;
        let w2 = st.mem.b16s1(r) as i32;
        let w3 = st.mem.b16s0(r) as i32;
        let last = st.input.last as usize;
        st.input.buffer[last] = w0;
        st.input.buffer[last + 1] = w1;
        st.input.buffer[last + 2] = w2;
        st.input.buffer[last + 3] = w3;
        st.input.last += 4;
        r += 1;
    }
    if st.input.last >= st.input.max_buf_stack {
        st.input.max_buf_stack = st.input.last + 1;
    }
    while st.input.last > st.input.first && st.input.buffer[(st.input.last - 1) as usize] == ' ' as i32 {
        st.input.last -= 1;
    }
    crate::mem::free_node(&mut st.mem, p, sz);
    Ok(true)
}

// ---------------------------------------------------------------------------
// insert_relax
// ---------------------------------------------------------------------------

/// Back up the current control sequence followed by a frozen `\relax`, marking
/// the stack `INSERTED`.
pub(crate) fn insert_relax_in(st: &mut crate::state::EngineState) -> Result<(), InputAbort> {
    st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
    back_input_in(st)?;
    st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_RELAX;
    back_input_in(st)?;
    st.input.cur_input.index = INSERTED;
    Ok(())
}

// ---------------------------------------------------------------------------
// begin_token_list
// ---------------------------------------------------------------------------

// INT_BASE + INT_PAR__tracing_macros (xetex_format.h:208, :65)
const INT_PAR_TRACING_MACROS: i32 = 30; // xetex_format.h:65
const LOCAL__OUTPUT_ROUTINE_BTL: i32 = 1; // xetex_format.h:173
const ASSIGN_TOKS_BTL: u16 = 73; // xetex_format.h:313

pub(crate) fn begin_token_list_in(
    st: &mut crate::state::EngineState,
    p: i32,
    t: u16,
) -> Result<(), InputAbort> {
    if st.input.input_ptr > st.input.max_in_stack {
        st.input.max_in_stack = st.input.input_ptr;
        if st.input.input_ptr == st.input.stack_size {
            return Err(InputAbort::Overflow {
                what: c"input stack size".as_ptr(),
                n: st.input.stack_size,
            });
        }
    }
    st.input.input_stack[st.input.input_ptr as usize] = st.input.cur_input;
    st.input.input_ptr += 1;
    st.input.cur_input.state = TOKEN_LIST;
    st.input.cur_input.start = p;
    st.input.cur_input.index = t;
    if t >= MACRO {
        st.mem.set_b32s0(p, st.mem.b32s0(p) + 1); // increment ref count
        if t == MACRO {
            st.input.cur_input.limit = st.input.param_ptr;
        } else {
            st.input.cur_input.loc = st.mem.b32s1(p);
            if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_MACROS) > 1 {
                crate::output::begin_diagnostic_in(st);
                crate::output::diagnostic_begin_capture_warning_here_in(st);
                crate::output::print_nl_cstr_in(st, c"".as_ptr());
                match t {
                    MARK_TEXT => crate::output::print_esc_cstr_in(st, c"mark".as_ptr()),
                    WRITE_TEXT => crate::output::print_esc_cstr_in(st, c"write".as_ptr()),
                    _ => crate::output::print_cmd_chr_in(
                        st,
                        ASSIGN_TOKS_BTL,
                        t as i32 + LOCAL_BASE + LOCAL__OUTPUT_ROUTINE_BTL - OUTPUT_TEXT as i32,
                    ),
                }
                crate::output::print_cstr_in(st, c"->".as_ptr());
                crate::output::token_show_in(st, p);
                crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
                crate::output::end_diagnostic_in(st, false);
            }
        }
    } else {
        st.input.cur_input.loc = p;
    }
    Ok(())
}
