//! Input stack: token-list and file levels.

use crate::consts::{
    BACKED_UP, CS_TOKEN_FLAG, FROZEN_CONTROL_SEQUENCE, INSERTED, INT_BASE, LOCAL_BASE, OUTPUT_TEXT,
    TEX_NULL, TOKEN_LIST, WRITE_TEXT,
};
use crate::errors::{EngineAbort, ErrorOutcome};
use crate::io::u_close;
use crate::mem::{flush_list, free_node, get_avail};
use crate::state::{EngineState, InputClusterState, MemState};
use crate::units::MemPtr;

pub(crate) const PARAMETER: u16 = 0;
pub(crate) const U_TEMPLATE: u16 = 1;
pub(crate) const V_TEMPLATE: u16 = 2;
pub(crate) const MACRO: u16 = 6;
pub(crate) const MARK_TEXT: u16 = 15;
pub(crate) const EVERY_EOF_TEXT: u16 = 16;
pub(crate) const TECTONIC_CODA_TEXT: u16 = 19;

pub(crate) const MID_LINE: u16 = 1;
pub(crate) const SKIP_BLANKS: u16 = 17;

pub(crate) const LEFT_BRACE_LIMIT: i32 = 0x40_0000;
pub(crate) const RIGHT_BRACE_LIMIT: i32 = 0x60_0000;

pub(crate) const FROZEN_RELAX: i32 = FROZEN_CONTROL_SEQUENCE + 7;

pub(crate) const INT_PAR_TRACING_MACROS: i32 = 30;
const LOCAL_OUTPUT_ROUTINE: i32 = 1;
const ASSIGN_TOKS: u16 = 73;

pub(crate) enum InputAbort {
    Overflow {
        what: *const libc::c_char,
        n: i32,
    },
    Fatal(*const libc::c_char),
    /// Aborts without printing a TeX error first.
    TtAbort(*const libc::c_char),
    MainMemory,
    InputClose,
}

impl InputAbort {
    pub(crate) fn into_engine_abort(self) -> EngineAbort {
        match self {
            InputAbort::Overflow { what, n } => EngineAbort::Overflow { what, n },
            InputAbort::Fatal(msg) => EngineAbort::Fatal(msg),
            InputAbort::TtAbort(msg) => EngineAbort::TtAbort(msg),
            InputAbort::MainMemory => crate::mem::main_memory_overflow_abort(),
            InputAbort::InputClose => {
                EngineAbort::TtAbort(c"serious internal error closing an input file".as_ptr())
            }
        }
    }
}

#[inline]
fn push_input(st: &mut EngineState) -> Result<(), InputAbort> {
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
    Ok(())
}

#[inline]
fn pop_input(st: &mut EngineState) {
    st.input.input_ptr -= 1;
    st.input.cur_input = st.input.input_stack[st.input.input_ptr as usize];
    st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
}

pub(crate) fn end_token_list_in(st: &mut EngineState) -> Result<(), InputAbort> {
    let ci = st.input.cur_input;
    if ci.index >= BACKED_UP {
        if ci.index <= INSERTED {
            flush_list(&mut st.mem, MemPtr::from_raw(ci.start));
        } else {
            crate::mem::delete_token_ref(&mut st.mem, ci.start);
            if ci.index == MACRO {
                while st.input.param_ptr > ci.limit {
                    st.input.param_ptr -= 1;
                    let p = st.input.param_stack[st.input.param_ptr as usize];
                    flush_list(&mut st.mem, MemPtr::from_raw(p));
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
    pop_input(st);
    Ok(())
}

pub(crate) fn back_input_in(st: &mut EngineState) -> Result<(), InputAbort> {
    while st.input.cur_input.state == TOKEN_LIST
        && st.input.cur_input.loc == TEX_NULL
        && st.input.cur_input.index != V_TEMPLATE
    {
        end_token_list_in(st)?;
    }
    let p = get_avail(&mut st.mem).ok_or(InputAbort::MainMemory)?;
    p.set_info(&mut st.mem, st.scanner.cur_tok);
    if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
        if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
            st.scanner.align_state -= 1;
        } else {
            st.scanner.align_state += 1;
        }
    }
    push_input(st)?;
    st.input.cur_input.state = TOKEN_LIST;
    st.input.cur_input.start = p.0;
    st.input.cur_input.index = BACKED_UP;
    st.input.cur_input.loc = p.0;
    Ok(())
}

pub(crate) fn back_error_in(st: &mut EngineState) -> Result<ErrorOutcome, InputAbort> {
    back_input_in(st)?;
    Ok(crate::errors::error_in(st))
}

pub(crate) fn ins_error_in(st: &mut EngineState) -> Result<ErrorOutcome, InputAbort> {
    back_input_in(st)?;
    st.input.cur_input.index = INSERTED;
    Ok(crate::errors::error_in(st))
}

pub(crate) fn pseudo_close(mem: &mut MemState, input: &mut InputClusterState) {
    let file = input.pseudo_files;
    let next_file = file.link(mem);
    let mut line = file.info(mem);
    crate::mem::free_avail(mem, file);
    input.pseudo_files = next_file;
    while line != TEX_NULL {
        let next = mem.b32s1(line);
        let size = mem.b32s0(line);
        free_node(mem, MemPtr::from_raw(line), size);
        line = next;
    }
}

pub(crate) fn begin_file_reading_in(st: &mut EngineState) -> Result<(), InputAbort> {
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
    push_input(st)?;
    let idx = st.input.in_open as usize;
    st.input.cur_input.index = idx as u16;
    st.input.source_filename_stack[idx] = 0;
    st.input.full_source_filename_stack[idx] = 0;
    st.input.eof_seen[idx] = false;
    st.input.grp_stack[idx] = st.save_stack.cur_boundary;
    st.input.if_stack[idx] = st.scanner.cond_ptr;
    st.input.line_stack[idx] = st.mem.line;
    st.input.cur_input.start = st.input.first;
    st.input.cur_input.state = MID_LINE;
    st.input.cur_input.name = 0;
    st.input.cur_input.synctex_tag = 0;
    st.mem.cur_synctex_tag = 0;
    Ok(())
}

pub(crate) fn end_file_reading_in(st: &mut EngineState) -> Result<(), InputAbort> {
    st.input.first = st.input.cur_input.start;
    st.mem.line = st.input.line_stack[st.input.cur_input.index as usize];
    match st.input.cur_input.name {
        18 | 19 => pseudo_close(&mut st.mem, &mut st.input),
        name if name > 17 => {
            let idx = st.input.cur_input.index as usize;
            let slot = std::mem::take(&mut st.input.input_file[idx]);
            // SAFETY: FFI close of the open input file handle at this stack slot.
            if unsafe { u_close(slot) } {
                return Err(InputAbort::InputClose);
            }
        }
        _ => {}
    }
    pop_input(st);
    st.input.in_open -= 1;
    Ok(())
}

pub(crate) fn pseudo_input_in(st: &mut EngineState) -> Result<bool, InputAbort> {
    st.input.last = st.input.first;
    let files = st.input.pseudo_files;
    let line = files.info(&st.mem);
    if line == TEX_NULL {
        return Ok(false);
    }
    let rest = st.mem.b32s1(line);
    files.set_info(&mut st.mem, rest);
    let size = st.mem.b32s0(line);
    if 4 * size - 3 >= st.input.buf_size - st.input.last {
        st.input.cur_input.loc = st.input.first;
        st.input.cur_input.limit = st.input.last - 1;
        return Err(InputAbort::Overflow {
            what: c"buffer size".as_ptr(),
            n: st.input.buf_size,
        });
    }
    st.input.last = st.input.first;
    for w in line + 1..line + size {
        let last = st.input.last as usize;
        st.input.buffer[last..last + 4].copy_from_slice(&[
            st.mem.b16s3(w) as i32,
            st.mem.b16s2(w) as i32,
            st.mem.b16s1(w) as i32,
            st.mem.b16s0(w) as i32,
        ]);
        st.input.last += 4;
    }
    if st.input.last >= st.input.max_buf_stack {
        st.input.max_buf_stack = st.input.last + 1;
    }
    while st.input.last > st.input.first
        && st.input.buffer[(st.input.last - 1) as usize] == ' ' as i32
    {
        st.input.last -= 1;
    }
    free_node(&mut st.mem, MemPtr::from_raw(line), size);
    Ok(true)
}

pub(crate) fn insert_relax_in(st: &mut EngineState) -> Result<(), InputAbort> {
    st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
    back_input_in(st)?;
    st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_RELAX;
    back_input_in(st)?;
    st.input.cur_input.index = INSERTED;
    Ok(())
}

pub(crate) fn begin_token_list_in(st: &mut EngineState, p: i32, t: u16) -> Result<(), InputAbort> {
    push_input(st)?;
    st.input.cur_input.state = TOKEN_LIST;
    st.input.cur_input.start = p;
    st.input.cur_input.index = t;
    if t < MACRO {
        st.input.cur_input.loc = p;
        return Ok(());
    }
    let list = MemPtr::from_raw(p);
    let ref_count = list.info(&st.mem);
    list.set_info(&mut st.mem, ref_count + 1);
    if t == MACRO {
        st.input.cur_input.limit = st.input.param_ptr;
        return Ok(());
    }
    st.input.cur_input.loc = list.link(&st.mem).0;
    if st.eqtb.b32s1(INT_BASE + INT_PAR_TRACING_MACROS) > 1 {
        use crate::output::{
            begin_diagnostic_in, capture_to_diagnostic_in,
            diagnostic_begin_capture_warning_here_in, end_diagnostic_in, print_cmd_chr_in,
            print_cstr_in, print_esc_cstr_in, print_nl_cstr_in, token_show_in,
        };
        begin_diagnostic_in(st);
        diagnostic_begin_capture_warning_here_in(st);
        print_nl_cstr_in(st, c"".as_ptr());
        match t {
            MARK_TEXT => print_esc_cstr_in(st, c"mark".as_ptr()),
            WRITE_TEXT => print_esc_cstr_in(st, c"write".as_ptr()),
            _ => print_cmd_chr_in(
                st,
                ASSIGN_TOKS,
                t as i32 + LOCAL_BASE + LOCAL_OUTPUT_ROUTINE - OUTPUT_TEXT as i32,
            ),
        }
        print_cstr_in(st, c"->".as_ptr());
        token_show_in(st, p);
        capture_to_diagnostic_in(st, std::ptr::null_mut());
        end_diagnostic_in(st, false);
    }
    Ok(())
}
