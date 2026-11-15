//! Input stack: token-list and file levels.

use std::mem;
use std::ops::{Index, IndexMut};

use crate::base::arg_stack::ArgRef;
use crate::base::codes::code_enum;
use crate::base::command::Command;
use crate::base::params::{IntPar, TokPar};
use crate::base::stack::TexStack;
use crate::base::token::{TokCursor, Token, TokenList};
use crate::base::units::{BufPos, MemPtr, SavePtr, StrNumber};
use crate::files::io::{UFile, u_close};
use crate::memory::mem::MemState;
use crate::memory::mem_types::ListStateRecord;
use crate::memory::node::{NodePtr, PseudoFileNode};
use crate::print::errors::{Abort, Capacity, OrConfusion};
use crate::print::{
    begin_diagnostic, end_diagnostic, print, print_command, print_esc, print_nl, token_show,
};
use crate::scanning::cond_stack::CondId;
use crate::state::EngineState;
use crate::tables::cs::{CsId, Frozen};

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
    /// The token parameter a level of this type runs, as `\tracingmacros` names it.
    /// WEB counts from `output_text`, which leaves the coda without one.
    pub(crate) const fn tok_par(self) -> Option<TokPar> {
        match self {
            TokenType::Output => Some(TokPar::OutputRoutine),
            TokenType::EveryPar => Some(TokPar::EveryPar),
            TokenType::EveryMath => Some(TokPar::EveryMath),
            TokenType::EveryDisplay => Some(TokPar::EveryDisplay),
            TokenType::EveryHbox => Some(TokPar::EveryHbox),
            TokenType::EveryVbox => Some(TokPar::EveryVbox),
            TokenType::EveryJob => Some(TokPar::EveryJob),
            TokenType::EveryCr => Some(TokPar::EveryCr),
            TokenType::Mark => Some(TokPar::ErrHelp),
            TokenType::EveryEof => Some(TokPar::EveryEof),
            TokenType::InterChar => Some(TokPar::XetexInterCharToks),
            _ => None,
        }
    }

    pub(crate) fn label(self, loc: TokCursor) -> &'static str {
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
    #[inline]
    pub(crate) fn token_type(&self) -> TokenType {
        TokenType::from_raw(i32::from(self.index)).expect("token list level")
    }

    #[inline]
    pub(crate) fn set_token_type(&mut self, t: TokenType) {
        self.index = t as u16;
    }
}

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
        .map_err(|_| stack.overflow(Capacity::InputStack))
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
                let first = ci.param_start();
                if let Some(&param) = st.input.param_stack.entries().get(first) {
                    st.mem.tokens.pop_args_to(param);
                    st.input.param_stack.truncate(first);
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
            capacity: Capacity::TextInputLevels,
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
    st.input.open_levels[idx] = OpenLevel {
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
            let slot = mem::take(&mut st.input.input_file[idx]);
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
    st.input
        .cur_input
        .set_token_loc(st.mem.tokens.cursor(list).into());
    Ok(())
}

pub(crate) fn insert_relax(st: &mut EngineState) -> Result<(), Abort> {
    st.scanner.cur_tok = Token::from_cs(st.scanner.cur_cs.or_confusion("insert_relax")?);
    back_input(st)?;
    st.scanner.cur_tok = Token::from_cs(Frozen::Relax.cs());
    back_input(st)?;
    st.input.cur_input.set_token_type(TokenType::Inserted);
    Ok(())
}

/// Starts reading parameter `k` of the innermost macro, for a `#` in its body.
pub(crate) fn begin_argument(st: &mut EngineState, k: usize) -> Result<(), Abort> {
    let param = *st
        .input
        .param_stack
        .entries()
        .get(k)
        .or_confusion("macro parameter")?;
    push_input(st)?;
    let ci = &mut st.input.cur_input;
    ci.state = TOKEN_LIST;
    ci.set_token_start(TokenList::NULL);
    ci.set_token_type(TokenType::Parameter);
    ci.set_token_loc(st.mem.tokens.arg_cursor(param));
    ci.set_param_start(k);
    Ok(())
}

/// Where token-list level `ci` began reading, for showing its context.
pub(crate) fn level_start(st: &EngineState, ci: &InputState) -> TokCursor {
    if ci.token_type() == TokenType::Parameter
        && let Some(param) = st.input.param_stack.entries().get(ci.param_start())
    {
        return st.mem.tokens.arg_cursor(*param);
    }
    st.mem.tokens.level_start(ci.token_start(), ci.token_loc())
}

pub(crate) fn begin_token_list(
    st: &mut EngineState,
    list: TokenList,
    t: TokenType,
) -> Result<(), Abort> {
    debug_assert!(
        t != TokenType::Parameter,
        "arguments start with begin_argument"
    );
    push_input(st)?;
    st.input.cur_input.state = TOKEN_LIST;
    st.input.cur_input.set_token_start(list);
    st.input.cur_input.set_token_type(t);
    st.input
        .cur_input
        .set_token_loc(st.mem.tokens.cursor(list).into());
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
        begin_diagnostic(st);
        print_nl(st, "");
        match t {
            TokenType::Mark => print_esc(st, "mark"),
            TokenType::Write => print_esc(st, "write"),
            _ => match t.tok_par() {
                Some(par) => print_command(st, Command::AssignToks(par.loc())),
                None => print_esc(st, TokPar::ErrHelp.name()),
            },
        }
        print(st, "->");
        token_show(st, list);
        end_diagnostic(st, false);
    }
    Ok(())
}

#[derive(Clone, Default, Debug)]
pub(crate) struct InputBuffer(pub Vec<i32>);

impl Index<BufPos> for InputBuffer {
    type Output = i32;
    #[inline(always)]
    fn index(&self, p: BufPos) -> &i32 {
        &self.0[p.index()]
    }
}

impl IndexMut<BufPos> for InputBuffer {
    #[inline(always)]
    fn index_mut(&mut self, p: BufPos) -> &mut i32 {
        &mut self.0[p.index()]
    }
}

impl InputBuffer {
    #[inline]
    pub(crate) fn span(&self, start: BufPos, end: BufPos) -> &[i32] {
        &self.0[start.index()..end.index()]
    }
}

/// What TeX keeps per open input file in `line_stack`, `grp_stack`,
/// `if_stack`, `eof_seen` and the source name stacks.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub(crate) struct OpenLevel {
    pub outer_line: i32,
    pub grp: Option<SavePtr>,
    pub cond: Option<CondId>,
    pub source_name: StrNumber,
    pub full_source_name: StrNumber,
    pub eof_seen: bool,
}

#[derive(Copy, Clone, Default)]
pub(crate) struct InputState {
    pub state: u16,
    pub index: u16,
    pub start: BufPos,
    pub loc: BufPos,
    pub limit: BufPos,
    pub name: InputName,
    pub synctex_tag: i32,
    /// A token-list level's list, which it frees or drops a reference to when it ends.
    tok_start: TokenList,
    tok_loc: TokCursor,
    param_start: u32,
}

/// What an input level reads from, WEB's overloaded `name` field.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub(crate) enum InputName {
    /// The terminal, or an `\insert`ed line above the bottom level.
    #[default]
    Terminal,
    Read(u8),
    ReadTerminal,
    /// A `\scantokens` pseudo file, `traced` when `\tracingscantokens` showed it like a file.
    Pseudo {
        traced: bool,
    },
    File(StrNumber),
    Macro(CsId),
}

impl InputName {
    /// Whether lines come from a file or pseudo file, WEB's `name>17`.
    pub(crate) fn reads_lines(self) -> bool {
        matches!(self, Self::Pseudo { .. } | Self::File(_))
    }

    /// Whether the level prints parentheses around its input, WEB's `name>=19`.
    pub(crate) fn shows_parens(self) -> bool {
        matches!(self, Self::Pseudo { traced: true } | Self::File(_))
    }
}

impl InputClusterState {
    pub(crate) fn buffer_overflow(&self) -> Abort {
        Abort::Overflow {
            capacity: Capacity::BufferSize,
            n: self.buf_size.raw(),
        }
    }

    #[inline]
    pub(crate) fn input_depth(&self) -> i32 {
        i32::try_from(self.input_stack.len()).unwrap_or(i32::MAX)
    }
}

impl InputState {
    /// `limit` of a macro level: where its parameters start on the parameter stack.
    /// An argument level keeps which parameter it reads here instead.
    #[inline]
    pub(crate) fn param_start(&self) -> usize {
        usize::try_from(self.param_start).unwrap_or(usize::MAX)
    }

    #[inline]
    pub(crate) fn set_param_start(&mut self, n: usize) {
        self.param_start = u32::try_from(n).unwrap_or(u32::MAX);
    }
}

impl InputState {
    #[inline]
    pub(crate) fn token_start(&self) -> TokenList {
        self.tok_start
    }
    #[inline]
    pub(crate) fn token_loc(&self) -> TokCursor {
        self.tok_loc
    }
    #[inline]
    pub(crate) fn set_token_loc(&mut self, c: TokCursor) {
        self.tok_loc = c;
    }
    #[inline]
    pub(crate) fn set_token_start(&mut self, list: TokenList) {
        self.tok_start = list;
    }
}

#[derive(Default)]
pub(crate) enum InputSlot {
    #[default]
    Empty,
    Terminal,
    Open(Box<UFile>),
}

#[derive(Default)]
pub(crate) struct InputClusterState {
    pub param_stack: TexStack<ArgRef>,
    /// Per open input file, indexed by `in_open`. Level 0 is the terminal.
    pub open_levels: Vec<OpenLevel>,
    pub buffer: InputBuffer,
    pub input_stack: TexStack<InputState>,
    pub cur_input: InputState,
    pub input_file: Vec<InputSlot>,
    pub nest: TexStack<ListStateRecord>,
    pub cur_list: ListStateRecord,
    pub open_parens: i32,
    pub in_open: usize,
    pub pseudo_files: MemPtr,
    pub buf_size: BufPos,
    pub max_in_open: usize,
    pub base_ptr: usize,
    pub par_loc: Option<CsId>,
    pub par_token: Token,
    pub force_eof: bool,
    pub first: BufPos,
    pub last: BufPos,
    pub max_buf_stack: BufPos,
    pub read_open: [u8; 17],
    pub read_file: [InputSlot; 16],
}

impl InputClusterState {
    #[inline]
    pub(crate) fn buffer_in_use(&self) -> usize {
        (self.max_buf_stack.max(self.last).max(self.first) + 1).index()
    }
}

/// Cap on one run of native-font text, which upstream grows until memory runs out.
pub(crate) const NATIVE_TEXT_MAX: i32 = 1 << 20;
