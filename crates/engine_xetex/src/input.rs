//! Input stack: token-list and file levels.

use crate::cmd::Cmd;
use crate::codes::code_enum;
use crate::consts::FROZEN_RELAX;
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::io::u_close;
use crate::node::{NodePtr, PseudoFileNode};
use crate::params::{IntPar, TokPar};
use crate::state::{EngineState, InputClusterState, InputName, InputState, MemState};
use crate::token::{Token, TokenList};
use crate::units::StrNumber;

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
    pub(crate) fn label(self, loc: crate::token::TokCursor) -> &'static str {
        match self {
            TokenType::Parameter => "<argument> ",
            TokenType::UTemplate | TokenType::VTemplate => "<template> ",
            TokenType::BackedUp | TokenType::BackedUpChar if loc.is_end() => "<recently read> ",
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

/// The most input levels open at once.
pub(crate) const STACK_SIZE: usize = 5_000;

pub(crate) const TOKEN_LIST: u16 = 0;
pub(crate) const MID_LINE: u16 = 1;
pub(crate) const SKIP_BLANKS: u16 = 17;
pub(crate) const NEW_LINE: u16 = 33;

#[inline]
fn push_input(st: &mut EngineState) -> Result<(), Abort> {
    let stack = &mut st.input.input_stack;
    stack
        .push(st.input.cur_input)
        .map_err(|_| stack.overflow(crate::errors::Capacity::InputStack))
}

#[inline]
fn pop_input(st: &mut EngineState) -> Result<(), Abort> {
    st.input.cur_input = st.input.input_stack.pop().or_confusion("pop_input")?;
    st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
    Ok(())
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
            st.mem.tokens.flush(ci.token_start());
        }
        t => {
            st.mem.tokens.delete_ref(ci.token_start());
            if t == TokenType::Macro {
                while st.input.param_stack.len() > ci.param_start() {
                    if let Some(param) = st.input.param_stack.pop() {
                        st.mem.tokens.flush(param.list);
                    }
                }
            } else if t == TokenType::Output && !st.shipout.output_can_end {
                return Err(Abort::Fatal("Unbalanced output routine"));
            }
        }
    }
    pop_input(st)?;
    Ok(())
}

pub(crate) fn back_input(st: &mut EngineState) -> Result<(), Abort> {
    while st.input.cur_input.state == TOKEN_LIST
        && st.input.cur_input.token_loc().is_end()
        && st.input.cur_input.token_type() != TokenType::VTemplate
        && st.input.cur_input.token_type() != TokenType::Output
    {
        end_token_list(st)?;
    }
    if st.scanner.cur_tok.is_brace() {
        if st.scanner.cur_tok.is_left_brace() {
            st.scanner.align_state -= 1;
        } else {
            st.scanner.align_state += 1;
        }
    }
    push_input(st)?;
    let at = st
        .mem
        .tokens
        .back_up(st.input.input_stack.len(), st.scanner.cur_tok)?;
    st.input.cur_input.state = TOKEN_LIST;
    st.input.cur_input.set_token_start(TokenList::NULL);
    st.input.cur_input.set_token_loc(at);
    st.input.cur_input.set_token_type(TokenType::BackedUp);
    Ok(())
}

impl MemState {
    pub(crate) fn pseudo_close(&mut self, input: &mut InputClusterState) -> Result<(), Abort> {
        let file =
            NodePtr::<PseudoFileNode>::from_ptr(input.pseudo_files).or_confusion("pseudo file")?;
        let PseudoFileNode { lines, link, .. } = self[file];
        file.free(self);
        input.pseudo_files = link;
        let mut line = lines;
        while let Some(l) = line {
            let header = self[l];
            line = header.link;
            self.free_array(l);
        }
        Ok(())
    }
}

pub(crate) fn begin_file_reading(st: &mut EngineState) -> Result<(), Abort> {
    if st.input.in_open == st.input.max_in_open {
        return Err(Abort::Overflow {
            capacity: crate::errors::Capacity::TextInputLevels,
            n: i32::try_from(st.input.max_in_open).unwrap_or(i32::MAX),
        });
    }
    if st.input.first == st.input.buf_size {
        return Err(st.input.buffer_overflow());
    }
    st.input.in_open += 1;
    push_input(st)?;
    let idx = st.input.in_open;
    st.input.cur_input.index = u16::try_from(idx).or_confusion("in_open")?;
    st.input.open_levels[idx] = crate::state::OpenLevel {
        outer_line: st.mem.line,
        grp: st.save_stack.cur_boundary,
        cond: st.scanner.conds.top(),
        source_name: StrNumber::NONE,
        full_source_name: StrNumber::NONE,
        eof_seen: false,
    };
    st.input.cur_input.start = st.input.first;
    st.input.cur_input.state = MID_LINE;
    st.input.cur_input.name = InputName::Terminal;
    st.input.cur_input.synctex_tag = 0;
    st.mem.cur_synctex_tag = 0;
    Ok(())
}

pub(crate) fn end_file_reading(st: &mut EngineState) -> Result<(), Abort> {
    st.input.first = st.input.cur_input.start;
    st.mem.line = st.input.open_levels[usize::from(st.input.cur_input.index)].outer_line;
    match st.input.cur_input.name {
        InputName::Pseudo { .. } => st.mem.pseudo_close(&mut st.input)?,
        InputName::File(_) => {
            let idx = st.input.cur_input.index as usize;
            let slot = std::mem::take(&mut st.input.input_file[idx]);
            if u_close(slot) {
                return Err(Abort::InputClose);
            }
        }
        _ => {}
    }
    pop_input(st)?;
    st.input.in_open -= 1;
    Ok(())
}

pub(crate) fn pseudo_input(st: &mut EngineState) -> Result<bool, Abort> {
    st.input.last = st.input.first;
    let file =
        NodePtr::<PseudoFileNode>::from_ptr(st.input.pseudo_files).or_confusion("pseudo file")?;
    let Some(line) = st.mem[file].lines else {
        return Ok(false);
    };
    let header = st.mem[line];
    st.mem[file].lines = header.link;
    let size = header.size;
    if 4 * size - 3 >= st.input.buf_size - st.input.last {
        st.input.cur_input.loc = st.input.first;
        st.input.cur_input.limit = st.input.last - 1;
        return Err(st.input.buffer_overflow());
    }
    st.input.last = st.input.first;
    for word in st.mem.entries(line) {
        let last = st.input.last.index();
        let chars = word.chars().map(i32::from);
        st.input.buffer.0[last..last + 4].copy_from_slice(&chars);
        st.input.last += 4;
    }
    if st.input.last >= st.input.max_buf_stack {
        st.input.max_buf_stack = st.input.last + 1;
    }
    while st.input.last > st.input.first && st.input.buffer[st.input.last - 1] == i32::from(b' ') {
        st.input.last -= 1;
    }
    st.mem.free_array(line);
    Ok(true)
}

/// Puts `token` in front of the unread backed-up token list on top of the input stack.
pub(crate) fn prepend_to_backed_up(st: &mut EngineState, token: Token) -> Result<(), Abort> {
    let ci = st.input.cur_input;
    let mut list = ci.token_start();
    debug_assert_eq!(
        ci.token_loc(),
        st.mem.tokens.level_start(list, ci.token_loc())
    );
    if list.is_null() {
        let rest = st
            .mem
            .tokens
            .peek(ci.token_loc())
            .or_confusion("backup slot")?;
        let b = st.mem.tokens.begin()?;
        st.mem.tokens.push(&b, token)?;
        st.mem.tokens.push(&b, rest)?;
        list = st.mem.tokens.finish(b)?;
        st.input.cur_input.set_token_start(list);
    } else {
        st.mem.tokens.prepend(list, token)?;
    }
    st.input.cur_input.set_token_loc(st.mem.tokens.cursor(list));
    Ok(())
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
    list: TokenList,
    t: TokenType,
) -> Result<(), Abort> {
    push_input(st)?;
    st.input.cur_input.state = TOKEN_LIST;
    st.input.cur_input.set_token_start(list);
    st.input.cur_input.set_token_type(t);
    st.input.cur_input.set_token_loc(st.mem.tokens.cursor(list));
    if t < TokenType::Macro {
        return Ok(());
    }
    st.mem.tokens.add_ref(list);
    if t == TokenType::Macro {
        st.input
            .cur_input
            .set_param_start(st.input.param_stack.len());
        return Ok(());
    }
    if st.eqtb.int_par(IntPar::TracingMacros) > 1 {
        use crate::output::{
            begin_diagnostic, end_diagnostic, print, print_cmd_chr, print_esc, print_nl, token_show,
        };
        begin_diagnostic(st);
        print_nl(st, "");
        match t {
            TokenType::Mark => print_esc(st, "mark"),
            TokenType::Write => print_esc(st, "write"),
            _ => print_cmd_chr(
                st,
                Cmd::AssignToks,
                TokPar::from_raw(t.raw() - TokenType::Output.raw() + TokPar::OutputRoutine.raw())
                    .map_or(-1, |p| p.loc().raw()),
            ),
        }
        print(st, "->");
        token_show(st, list);
        end_diagnostic(st, false);
    }
    Ok(())
}
