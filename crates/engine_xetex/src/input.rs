//! Input stack: token-list and file levels.

use crate::cmd::ASSIGN_TOKS;
use crate::codes::code_enum;
use crate::consts::{FROZEN_RELAX, TEX_NULL};
use crate::errors::Abort;
use crate::io::u_close;
use crate::node::{NodePtr, PseudoLineNode};
use crate::params::{IntPar, TokPar};
use crate::state::{EngineState, InputClusterState, InputState, MemState};
use crate::token::Token;
use crate::units::{MemPtr, StrNumber};

code_enum! {
    /// What a token-list level reads (`token_type`). It lives in `index`,
    /// which holds the file number of a file level.
    #[derive(PartialOrd, Ord)]
    pub(crate) enum TokenType: u16 {
        Parameter = 0,
        UTemplate = 1,
        VTemplate = 2,
        BackedUp = 3,
        BackedUpChar = 4,
        Inserted = 5,
        Macro = 6,
        Output = 7,
        EveryPar = 8,
        EveryMath = 9,
        EveryDisplay = 10,
        EveryHbox = 11,
        EveryVbox = 12,
        EveryJob = 13,
        EveryCr = 14,
        Mark = 15,
        EveryEof = 16,
        InterChar = 17,
        Write = 18,
        TectonicCoda = 19,
    }
}

impl TokenType {
    /// How `show_context` introduces a level of this type, except a macro.
    pub(crate) fn label(self, loc: i32) -> &'static str {
        match self {
            TokenType::Parameter => "<argument> ",
            TokenType::UTemplate | TokenType::VTemplate => "<template> ",
            TokenType::BackedUp | TokenType::BackedUpChar if loc == TEX_NULL => "<recently read> ",
            TokenType::BackedUp | TokenType::BackedUpChar => "<to be read again> ",
            TokenType::Inserted => "<inserted text> ",
            TokenType::Macro => unreachable!("a macro level shows its name"),
            TokenType::Output => "<output> ",
            TokenType::EveryPar => "<everypar> ",
            TokenType::EveryMath => "<everymath> ",
            TokenType::EveryDisplay => "<everydisplay> ",
            TokenType::EveryHbox => "<everyhbox> ",
            TokenType::EveryVbox => "<everyvbox> ",
            TokenType::EveryJob => "<everyjob> ",
            TokenType::EveryCr => "<everycr> ",
            TokenType::Mark => "<mark> ",
            TokenType::EveryEof => "<everyeof> ",
            TokenType::InterChar => "<XeTeXinterchartoks> ",
            TokenType::Write => "<write> ",
            TokenType::TectonicCoda => "<TectonicCodaTokens> ",
        }
    }
}

impl InputState {
    /// The type of a token-list level.
    #[inline]
    pub(crate) fn token_type(&self) -> TokenType {
        TokenType::from_raw(i32::from(self.index)).expect("token list level")
    }

    #[inline]
    pub(crate) fn set_token_type(&mut self, t: TokenType) {
        self.index = t as u16;
    }
}

pub(crate) const TOKEN_LIST: u16 = 0;
pub(crate) const MID_LINE: u16 = 1;
pub(crate) const SKIP_BLANKS: u16 = 17;
pub(crate) const NEW_LINE: u16 = 33;

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
    match ci.token_type() {
        TokenType::Parameter | TokenType::VTemplate => {}
        TokenType::UTemplate => {
            if st.scanner.align_state > 500_000 {
                st.scanner.align_state = 0;
            } else {
                return Err(Abort::Fatal(
                    "(interwoven alignment preambles are not allowed)",
                ));
            }
        }
        TokenType::BackedUp | TokenType::BackedUpChar | TokenType::Inserted => {
            st.mem.flush_list(ci.token_start());
        }
        t => {
            st.mem.delete_token_ref(ci.token_start());
            if t == TokenType::Macro {
                while st.input.param_ptr > ci.limit {
                    st.input.param_ptr -= 1;
                    st.mem
                        .flush_list(st.input.param_stack[st.input.param_ptr as usize]);
                }
            }
        }
    }
    pop_input(st);
    Ok(())
}

pub(crate) fn back_input(st: &mut EngineState) -> Result<(), Abort> {
    while st.input.cur_input.state == TOKEN_LIST
        && st.input.cur_input.loc == TEX_NULL
        && st.input.cur_input.token_type() != TokenType::VTemplate
    {
        end_token_list(st)?;
    }
    let p = st.mem.get_avail()?;
    p.set_token(&mut st.mem, st.scanner.cur_tok);
    if st.scanner.cur_tok.is_brace() {
        if st.scanner.cur_tok.is_left_brace() {
            st.scanner.align_state -= 1;
        } else {
            st.scanner.align_state += 1;
        }
    }
    push_input(st)?;
    st.input.cur_input.state = TOKEN_LIST;
    st.input.cur_input.set_token_list(p);
    st.input.cur_input.set_token_type(TokenType::BackedUp);
    Ok(())
}

pub(crate) fn back_error(st: &mut EngineState) -> Result<(), Abort> {
    back_input(st)?;
    crate::errors::error(st)
}

pub(crate) fn ins_error(st: &mut EngineState) -> Result<(), Abort> {
    back_input(st)?;
    st.input.cur_input.set_token_type(TokenType::Inserted);
    crate::errors::error(st)
}

impl MemState {
    pub(crate) fn pseudo_close(&mut self, input: &mut InputClusterState) {
        let file = input.pseudo_files;
        let next_file = file.link(self);
        let mut line = file.info_ptr(self);
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
    let Some(line) = NodePtr::<PseudoLineNode>::from_ptr(files.info_ptr(&st.mem)) else {
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
        && st.input.buffer[(st.input.last - 1) as usize] == i32::from(b' ')
    {
        st.input.last -= 1;
    }
    st.mem.free_node(line.ptr(), size);
    Ok(true)
}

pub(crate) fn insert_relax(st: &mut EngineState) -> Result<(), Abort> {
    st.scanner.cur_tok = Token::from_cs(st.scanner.cur_cs);
    back_input(st)?;
    st.scanner.cur_tok = Token::from_cs(FROZEN_RELAX);
    back_input(st)?;
    st.input.cur_input.set_token_type(TokenType::Inserted);
    Ok(())
}

pub(crate) fn begin_token_list(
    st: &mut EngineState,
    list: MemPtr,
    t: TokenType,
) -> Result<(), Abort> {
    push_input(st)?;
    st.input.cur_input.state = TOKEN_LIST;
    st.input.cur_input.start = list.0;
    st.input.cur_input.set_token_type(t);
    if t < TokenType::Macro {
        st.input.cur_input.set_token_loc(list);
        return Ok(());
    }
    st.mem.add_token_ref(list);
    if t == TokenType::Macro {
        st.input.cur_input.limit = st.input.param_ptr;
        return Ok(());
    }
    st.input.cur_input.set_token_loc(list.link(&st.mem));
    if st.eqtb.int_par(IntPar::TracingMacros) > 1 {
        use crate::output::{
            begin_diagnostic, capture_to_diagnostic, diagnostic_begin_capture_warning_here,
            end_diagnostic, print, print_cmd_chr, print_esc, print_nl, token_show,
        };
        begin_diagnostic(st);
        diagnostic_begin_capture_warning_here(st);
        print_nl(st, "");
        match t {
            TokenType::Mark => print_esc(st, "mark"),
            TokenType::Write => print_esc(st, "write"),
            _ => print_cmd_chr(
                st,
                ASSIGN_TOKS,
                t.raw() - TokenType::Output.raw() + TokPar::OutputRoutine.loc(),
            ),
        }
        print(st, "->");
        token_show(st, list);
        capture_to_diagnostic(st, None);
        end_diagnostic(st, false);
    }
    Ok(())
}
