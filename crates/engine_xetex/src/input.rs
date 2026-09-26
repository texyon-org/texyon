//! Input stack: token-list and file levels.

use crate::cmd::ASSIGN_TOKS;
use crate::consts::{
    CS_TOKEN_FLAG, FROZEN_RELAX, INT_PAR_TRACING_MACROS, LOCAL_BASE, LOCAL_OUTPUT_ROUTINE, TEX_NULL,
};
use crate::errors::Abort;
use crate::io::u_close;
use crate::node::{NodePtr, PseudoLineNode};
use crate::state::{EngineState, InputClusterState, MemState};
use crate::units::{MemPtr, StrNumber};

pub(crate) const PARAMETER: u16 = 0;
pub(crate) const U_TEMPLATE: u16 = 1;
pub(crate) const V_TEMPLATE: u16 = 2;
pub(crate) const BACKED_UP: u16 = 3;
pub(crate) const BACKED_UP_CHAR: u16 = 4;
pub(crate) const INSERTED: u16 = 5;
pub(crate) const MACRO: u16 = 6;
pub(crate) const OUTPUT_TEXT: u16 = 7;
pub(crate) const EVERY_PAR_TEXT: u16 = 8;
pub(crate) const EVERY_MATH_TEXT: u16 = 9;
pub(crate) const EVERY_DISPLAY_TEXT: u16 = 10;
pub(crate) const EVERY_HBOX_TEXT: u16 = 11;
pub(crate) const EVERY_VBOX_TEXT: u16 = 12;
pub(crate) const EVERY_JOB_TEXT: u16 = 13;
pub(crate) const EVERY_CR_TEXT: u16 = 14;
pub(crate) const MARK_TEXT: u16 = 15;
pub(crate) const EVERY_EOF_TEXT: u16 = 16;
pub(crate) const INTER_CHAR_TEXT: u16 = 17;
pub(crate) const WRITE_TEXT: u16 = 18;
pub(crate) const TECTONIC_CODA_TEXT: u16 = 19;

pub(crate) const TOKEN_LIST: u16 = 0;
pub(crate) const MID_LINE: u16 = 1;
pub(crate) const SKIP_BLANKS: u16 = 17;
pub(crate) const NEW_LINE: u16 = 33;

pub(crate) const LEFT_BRACE_LIMIT: i32 = 0x40_0000;
pub(crate) const RIGHT_BRACE_LIMIT: i32 = 0x60_0000;

#[inline]
fn push_input(st: &mut EngineState) -> Result<(), Abort> {
    let ptr = st.input.input_ptr;
    if ptr > st.input.max_in_stack {
        st.input.max_in_stack = ptr;
        if ptr == st.input.stack_size {
            return Err(Abort::Overflow {
                what: "input stack size",
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

pub(crate) fn end_token_list(st: &mut EngineState) -> Result<(), Abort> {
    let ci = st.input.cur_input;
    if ci.index >= BACKED_UP {
        if ci.index <= INSERTED {
            st.mem.flush_list(MemPtr::from_raw(ci.start));
        } else {
            st.mem.delete_token_ref(MemPtr(ci.start));
            if ci.index == MACRO {
                while st.input.param_ptr > ci.limit {
                    st.input.param_ptr -= 1;
                    st.mem
                        .flush_list(st.input.param_stack[st.input.param_ptr as usize]);
                }
            }
        }
    } else if ci.index == U_TEMPLATE {
        if st.scanner.align_state > 500_000 {
            st.scanner.align_state = 0;
        } else {
            return Err(Abort::Fatal(
                "(interwoven alignment preambles are not allowed)",
            ));
        }
    }
    pop_input(st);
    Ok(())
}

pub(crate) fn back_input(st: &mut EngineState) -> Result<(), Abort> {
    while st.input.cur_input.state == TOKEN_LIST
        && st.input.cur_input.loc == TEX_NULL
        && st.input.cur_input.index != V_TEMPLATE
    {
        end_token_list(st)?;
    }
    let p = st.mem.get_avail()?;
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

pub(crate) fn back_error(st: &mut EngineState) -> Result<(), Abort> {
    back_input(st)?;
    crate::errors::error(st)
}

pub(crate) fn ins_error(st: &mut EngineState) -> Result<(), Abort> {
    back_input(st)?;
    st.input.cur_input.index = INSERTED;
    crate::errors::error(st)
}

impl MemState {
    pub(crate) fn pseudo_close(&mut self, input: &mut InputClusterState) {
        let file = input.pseudo_files;
        let next_file = file.link(self);
        let mut line = MemPtr(file.info(self));
        self.free_avail(file);
        input.pseudo_files = next_file;
        while let Some(l) = NodePtr::<PseudoLineNode>::from_ptr(line) {
            line = l.link(self);
            self.free_node(l.ptr(), l.size(self));
        }
    }
}

pub(crate) fn begin_file_reading(st: &mut EngineState) -> Result<(), Abort> {
    if st.input.in_open == st.input.max_in_open {
        return Err(Abort::Overflow {
            what: "text input levels",
            n: st.input.max_in_open,
        });
    }
    if st.input.first == st.input.buf_size {
        return Err(Abort::Overflow {
            what: "buffer size",
            n: st.input.buf_size,
        });
    }
    st.input.in_open += 1;
    push_input(st)?;
    let idx = st.input.in_open as usize;
    st.input.cur_input.index = idx as u16;
    st.input.source_filename_stack[idx] = StrNumber::NONE;
    st.input.full_source_filename_stack[idx] = StrNumber::NONE;
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

pub(crate) fn end_file_reading(st: &mut EngineState) -> Result<(), Abort> {
    st.input.first = st.input.cur_input.start;
    st.mem.line = st.input.line_stack[st.input.cur_input.index as usize];
    match st.input.cur_input.name {
        18 | 19 => st.mem.pseudo_close(&mut st.input),
        name if name > 17 => {
            let idx = st.input.cur_input.index as usize;
            let slot = std::mem::take(&mut st.input.input_file[idx]);
            if u_close(slot) {
                return Err(Abort::InputClose);
            }
        }
        _ => {}
    }
    pop_input(st);
    st.input.in_open -= 1;
    Ok(())
}

pub(crate) fn pseudo_input(st: &mut EngineState) -> Result<bool, Abort> {
    st.input.last = st.input.first;
    let files = st.input.pseudo_files;
    let Some(line) = NodePtr::<PseudoLineNode>::from_ptr(MemPtr(files.info(&st.mem))) else {
        return Ok(false);
    };
    let rest = line.link(&st.mem);
    files.set_info(&mut st.mem, rest.0);
    let size = line.size(&st.mem);
    if 4 * size - 3 >= st.input.buf_size - st.input.last {
        st.input.cur_input.loc = st.input.first;
        st.input.cur_input.limit = st.input.last - 1;
        return Err(Abort::Overflow {
            what: "buffer size",
            n: st.input.buf_size,
        });
    }
    st.input.last = st.input.first;
    for k in 1..size {
        let last = st.input.last as usize;
        let chars = line.chars(&st.mem, k).map(i32::from);
        st.input.buffer[last..last + 4].copy_from_slice(&chars);
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
    st.mem.free_node(line.ptr(), size);
    Ok(true)
}

pub(crate) fn insert_relax(st: &mut EngineState) -> Result<(), Abort> {
    st.scanner.cur_tok = CS_TOKEN_FLAG + st.scanner.cur_cs;
    back_input(st)?;
    st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_RELAX;
    back_input(st)?;
    st.input.cur_input.index = INSERTED;
    Ok(())
}

pub(crate) fn begin_token_list(st: &mut EngineState, list: MemPtr, t: u16) -> Result<(), Abort> {
    push_input(st)?;
    st.input.cur_input.state = TOKEN_LIST;
    st.input.cur_input.start = list.0;
    st.input.cur_input.index = t;
    if t < MACRO {
        st.input.cur_input.loc = list.0;
        return Ok(());
    }
    st.mem.add_token_ref(list);
    if t == MACRO {
        st.input.cur_input.limit = st.input.param_ptr;
        return Ok(());
    }
    st.input.cur_input.loc = list.link(&st.mem).0;
    if st.eqtb.int_par(INT_PAR_TRACING_MACROS) > 1 {
        use crate::output::{
            begin_diagnostic, capture_to_diagnostic, diagnostic_begin_capture_warning_here,
            end_diagnostic, print, print_cmd_chr, print_esc, print_nl, token_show,
        };
        begin_diagnostic(st);
        diagnostic_begin_capture_warning_here(st);
        print_nl(st, "");
        match t {
            MARK_TEXT => print_esc(st, "mark"),
            WRITE_TEXT => print_esc(st, "write"),
            _ => print_cmd_chr(
                st,
                ASSIGN_TOKS,
                t as i32 + LOCAL_BASE + LOCAL_OUTPUT_ROUTINE - OUTPUT_TEXT as i32,
            ),
        }
        print(st, "->");
        token_show(st, list);
        capture_to_diagnostic(st, None);
        end_diagnostic(st, false);
    }
    Ok(())
}
