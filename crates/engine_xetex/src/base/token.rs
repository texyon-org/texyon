//! TeX tokens: `cmd * max_char_val + chr` for a character, `cs_token_flag + p`
//! for a control sequence.

use std::mem;

use bytemuck::Zeroable;

use crate::base::arg_stack::{ArgDelta, ArgRef, ArgStack};
use crate::base::cmd::Cmd;
use crate::base::consts::{MAX_HALFWORD, TEX_NULL};
use crate::preview::journal::{WordDelta, WordJournal};
use crate::print::errors::Abort;
use crate::scanning::input::STACK_SIZE;
use crate::tables::cs::CsId;

const MAX_CHAR_VAL: i32 = 0x20_0000;
/// Control sequence tokens count up from here, past every character token.
const CS_TOKEN_FLAG: i32 = 0x1FF_FFFF;

/// A token as stored in token lists and `cur_tok`. Ordered like its raw value,
/// so brace and parameter tests are range checks as in TeX.
#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, bytemuck::Pod, bytemuck::Zeroable,
)]
#[repr(transparent)]
pub(crate) struct Token(pub(crate) i32);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum TokenKind {
    Cs(CsId),
    Char(Cmd, i32),
}

impl Token {
    /// `cur_cmd = cur_chr = 0`: the end of a `\read` line, or no pending
    /// `\afterassignment` token.
    pub(crate) const ZERO: Token = Token(0);
    pub(crate) const SPACE: Token = Token::new(Cmd::Spacer, b' ' as i32);
    /// Marks a `\protected` macro, right after its reference count.
    pub(crate) const PROTECTED: Token = Token(Token::END_MATCH.0 + 1);
    pub(crate) const END_MATCH: Token = Token::new(Cmd::END_MATCH, 0);

    #[inline(always)]
    pub(crate) const fn new(cmd: Cmd, chr: i32) -> Token {
        Token(cmd.raw() * MAX_CHAR_VAL + chr)
    }

    #[inline(always)]
    pub(crate) const fn from_cs(p: CsId) -> Token {
        Token(CS_TOKEN_FLAG + p.raw())
    }

    #[inline(always)]
    pub(crate) const fn left_brace(c: u8) -> Token {
        Token::new(Cmd::LeftBrace, c as i32)
    }

    #[inline(always)]
    pub(crate) const fn right_brace(c: u8) -> Token {
        Token::new(Cmd::RightBrace, c as i32)
    }

    #[inline(always)]
    pub(crate) const fn math_shift(c: u8) -> Token {
        Token::new(Cmd::MathShift, c as i32)
    }

    #[inline(always)]
    pub(crate) const fn tab(c: u8) -> Token {
        Token::new(Cmd::TabMark, c as i32)
    }

    #[inline(always)]
    pub(crate) const fn letter(c: u8) -> Token {
        Token::new(Cmd::Letter, c as i32)
    }

    #[inline(always)]
    pub(crate) const fn other(c: u8) -> Token {
        Token::new(Cmd::OtherChar, c as i32)
    }

    #[inline(always)]
    pub(crate) const fn out_param(n: i32) -> Token {
        Token::new(Cmd::OUT_PARAM, n)
    }

    #[inline(always)]
    pub(crate) const fn match_(c: i32) -> Token {
        Token::new(Cmd::MATCH, c)
    }

    #[inline(always)]
    pub(crate) const fn is_cs(self) -> bool {
        self.0 >= CS_TOKEN_FLAG
    }

    pub(crate) fn can_name(top: CsId) -> bool {
        i64::from(CS_TOKEN_FLAG) + i64::from(top.raw()) <= i64::from(MAX_HALFWORD)
    }

    #[inline(always)]
    pub(crate) const fn cs(self) -> Option<CsId> {
        // A character token lies below the flag, so it maps to a negative raw value.
        CsId::from_raw(self.0 - CS_TOKEN_FLAG)
    }

    #[inline(always)]
    pub(crate) const fn char(self) -> Option<(Cmd, i32)> {
        if self.is_cs() {
            None
        } else {
            Some((self.cmd(), self.chr()))
        }
    }

    #[inline(always)]
    pub(crate) const fn kind(self) -> TokenKind {
        match self.cs() {
            Some(cs) => TokenKind::Cs(cs),
            None => TokenKind::Char(self.cmd(), self.chr()),
        }
    }

    #[inline(always)]
    const fn cmd(self) -> Cmd {
        Cmd::from_catcode_bits(self.0 / MAX_CHAR_VAL)
    }

    #[inline(always)]
    const fn chr(self) -> i32 {
        self.0 % MAX_CHAR_VAL
    }

    #[inline(always)]
    pub(crate) const fn is_left_brace(self) -> bool {
        self.0 < LEFT_BRACE_LIMIT
    }

    #[inline(always)]
    pub(crate) const fn is_brace(self) -> bool {
        self.0 < RIGHT_BRACE_LIMIT
    }

    #[inline(always)]
    pub(crate) const fn is_match(self) -> bool {
        self.0 >= Token::new(Cmd::MATCH, 0).0 && self.0 <= Token::END_MATCH.0
    }

    #[inline(always)]
    pub(crate) const fn other_digit(self) -> Option<i32> {
        if self.0 >= Token::other(b'0').0 && self.0 <= Token::other(b'9').0 {
            Some(self.0 - Token::other(b'0').0)
        } else {
            None
        }
    }
}

const LEFT_BRACE_LIMIT: i32 = Token::new(Cmd::RightBrace, 0).0;
const RIGHT_BRACE_LIMIT: i32 = Token::new(Cmd::MathShift, 0).0;

/// A token list: an index into the store's list table. Null for no list.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Hash, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub(crate) struct TokenList(i32);

impl TokenList {
    pub(crate) const NULL: TokenList = TokenList(TEX_NULL);
    /// The one-token list `\endtemplate` that ends an omitted template.
    pub(crate) const OMIT_TEMPLATE: TokenList = TokenList(1);

    #[inline(always)]
    pub(crate) const fn from_raw(raw: i32) -> TokenList {
        TokenList(raw)
    }

    #[inline(always)]
    pub(crate) const fn raw(self) -> i32 {
        self.0
    }

    #[inline(always)]
    pub(crate) fn is_null(self) -> bool {
        self.0 == TEX_NULL
    }

    #[inline(always)]
    fn slot(self) -> Option<usize> {
        if self.0 > 0 {
            usize::try_from(self.0).ok()
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct TokCursor {
    pos: u32,
    end: u32,
}

/// Set in a cursor's positions when it reads the argument stack instead of the lists.
const ARG_TAG: u32 = 1 << 31;
const _: () = assert!(MAX_TOKENS < ARG_TAG as usize);

/// A cursor into a stored token list, never into an argument, so reading needs no tag test.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct ListCursor {
    pos: u32,
    end: u32,
}

impl ListCursor {
    pub(crate) const END: ListCursor = ListCursor { pos: 0, end: 0 };
}

impl From<ListCursor> for TokCursor {
    #[inline(always)]
    fn from(c: ListCursor) -> TokCursor {
        TokCursor {
            pos: c.pos,
            end: c.end,
        }
    }
}

/// A position in a token list that the store can read.
pub(crate) trait Cursor: Copy {
    fn is_end(self) -> bool;
    fn advanced(self) -> Self;
    fn token(self, store: &TokenStore) -> Option<Token>;
}

impl Cursor for ListCursor {
    #[inline(always)]
    fn is_end(self) -> bool {
        self.pos == self.end
    }

    #[inline(always)]
    fn advanced(self) -> Self {
        ListCursor {
            pos: self.pos + 1,
            end: self.end,
        }
    }

    #[inline(always)]
    fn token(self, store: &TokenStore) -> Option<Token> {
        store.tokens.get(self.pos as usize).copied()
    }
}

impl Cursor for TokCursor {
    #[inline(always)]
    fn is_end(self) -> bool {
        self.pos == self.end
    }

    #[inline(always)]
    fn advanced(self) -> Self {
        TokCursor {
            pos: self.pos + 1,
            end: self.end,
        }
    }

    #[inline(always)]
    fn token(self, store: &TokenStore) -> Option<Token> {
        if self.pos & ARG_TAG == 0 {
            store.tokens.get(self.pos as usize).copied()
        } else {
            store.args.get((self.pos & !ARG_TAG) as usize)
        }
    }
}

impl TokCursor {
    pub(crate) const END: TokCursor = TokCursor { pos: 0, end: 0 };

    #[inline(always)]
    pub(crate) fn is_end(self) -> bool {
        self.pos == self.end
    }

    pub(crate) fn parts(self) -> (u32, u32) {
        (self.pos, self.end)
    }
}

#[must_use]
#[derive(Debug)]
pub(crate) struct Build(usize);

/// Arguments built back to back in the build buffer, for one macro call, at
/// offsets from `base`. They go onto the argument stack only once the macro's level is pushed.
#[must_use]
#[derive(Debug)]
pub(crate) struct BuiltArgs {
    base: usize,
    args: [ArgRef; 9],
    n: usize,
}

impl BuiltArgs {
    pub(crate) fn len(&self) -> usize {
        self.n
    }
}

/// A list's run of tokens and its references. A free entry has no references
/// and keeps the next free entry in `start`.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ListSlot {
    start: u32,
    len: u32,
    refs: u32,
}

const NONE: u32 = u32::MAX;
const EXACT_CLASSES: u32 = 16;
const N_CLASSES: usize = 56;
const CLASS_CAPS: [u32; N_CLASSES] = class_caps();

const fn class_caps() -> [u32; N_CLASSES] {
    let mut caps = [0u32; N_CLASSES];
    let mut k = 1;
    while k < N_CLASSES {
        caps[k] = if k as u32 <= EXACT_CLASSES {
            k as u32
        } else {
            let prev = caps[k - 1];
            prev.saturating_add(prev.div_ceil(2))
        };
        k += 1;
    }
    caps
}

#[inline]
fn class_for(len: u32) -> usize {
    if len <= EXACT_CLASSES {
        return len as usize;
    }
    CLASS_CAPS.partition_point(|&cap| cap < len)
}

/// Half of `MAIN_MEMORY_WORDS`, until nodes and tokens share one budget.
const MAX_TOKENS: usize = 1 << 25;

/// Tokens 1..=`STACK_SIZE` hold the token a backed-up level at that input stack depth reads.
const BACKUP_SLOTS: usize = STACK_SIZE;
/// The tokens before the first run: [`TokenList::OMIT_TEMPLATE`] and the backup slots.
const RESERVED: usize = 1 + BACKUP_SLOTS;

/// The token lists, kept apart from `mem`: runs of tokens back to back, and a
/// table of lists that says where each run is.
pub(crate) struct TokenStore {
    tokens: Vec<Token>,
    lists: Vec<ListSlot>,
    free_runs: [u32; N_CLASSES],
    free_lists: u32,
    tokens_journal: WordJournal<Token>,
    lists_journal: WordJournal<ListSlot>,
    build: Vec<Token>,
    /// Each open build's start in `build`, and the tokens it dropped from its front.
    builds: Vec<(usize, usize)>,
    args: ArgStack,
}

impl Default for TokenStore {
    fn default() -> Self {
        Self {
            tokens: vec![Token(0); RESERVED],
            // Slot 0 is unused, so no list is ever 0.
            lists: vec![
                ListSlot::zeroed(),
                ListSlot {
                    start: 0,
                    len: 1,
                    refs: 1,
                },
            ],
            free_runs: [NONE; N_CLASSES],
            free_lists: NONE,
            tokens_journal: WordJournal::default(),
            lists_journal: WordJournal::default(),
            build: Vec::new(),
            builds: Vec::new(),
            args: ArgStack::default(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct TokenStoreMarks {
    pub(crate) tokens: usize,
    pub(crate) lists: usize,
    pub(crate) free_runs: [u32; N_CLASSES],
    pub(crate) free_lists: u32,
    pub(crate) args: usize,
}

#[derive(Clone, Default)]
pub(crate) struct TokenDelta {
    tokens: WordDelta<Token>,
    lists: WordDelta<ListSlot>,
    args: ArgDelta,
}

impl TokenDelta {
    pub(crate) fn compose_with_older(&mut self, older: TokenDelta) {
        self.tokens.compose_with_older(older.tokens);
        self.lists.compose_with_older(older.lists);
        self.args.compose_with_older(older.args);
    }

    pub(crate) fn compact(&mut self) {
        self.tokens.compact();
        self.lists.compact();
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.tokens.allocated_bytes() + self.lists.allocated_bytes() + self.args.allocated_bytes()
    }
}

impl TokenStore {
    #[inline(always)]
    fn set_token(&mut self, i: usize, t: Token) {
        if self.tokens_journal.armed() {
            self.tokens_journal.note(i, self.tokens[i]);
        }
        self.tokens[i] = t;
    }

    #[inline(always)]
    fn set_slot(&mut self, i: usize, slot: ListSlot) {
        if self.lists_journal.armed() {
            self.lists_journal.note(i, self.lists[i]);
        }
        self.lists[i] = slot;
    }

    #[inline(always)]
    fn slot_of(&self, list: TokenList) -> Option<ListSlot> {
        list.slot().and_then(|i| self.lists.get(i)).copied()
    }

    pub(crate) fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    pub(crate) fn lists(&self) -> &[ListSlot] {
        &self.lists
    }

    pub(crate) fn arm_journal(&mut self) {
        self.tokens_journal.arm(self.tokens.len());
        self.lists_journal.arm(self.lists.len());
        self.args.arm_journal();
    }

    pub(crate) fn disarm_journal(&mut self) {
        self.tokens_journal.disarm();
        self.lists_journal.disarm();
        self.args.disarm_journal();
    }

    pub(crate) fn seal_journal(&mut self) -> TokenDelta {
        TokenDelta {
            tokens: self.tokens_journal.seal(&self.tokens),
            lists: self.lists_journal.seal(&self.lists),
            args: self.args.seal_journal(),
        }
    }

    pub(crate) fn snapshot_journal(&self) -> TokenDelta {
        TokenDelta {
            tokens: self.tokens_journal.snapshot(),
            lists: self.lists_journal.snapshot(),
            args: self.args.snapshot_journal(),
        }
    }

    pub(crate) fn apply(&mut self, delta: &TokenDelta) {
        delta.tokens.apply_to(&mut self.tokens);
        delta.lists.apply_to(&mut self.lists);
        self.args.apply(&delta.args);
    }

    /// The tokens on the argument stack, for checkpoint digests.
    pub(crate) fn live_args(&self) -> &[Token] {
        self.args.live()
    }

    pub(crate) fn marks(&self) -> TokenStoreMarks {
        debug_assert!(
            self.builds.is_empty(),
            "checkpoint while a token list is built"
        );
        TokenStoreMarks {
            tokens: self.tokens.len(),
            lists: self.lists.len(),
            free_runs: self.free_runs,
            free_lists: self.free_lists,
            args: self.args.len(),
        }
    }

    pub(crate) fn restore_marks(&mut self, marks: TokenStoreMarks) {
        self.builds.clear();
        self.build.clear();
        self.tokens.truncate(marks.tokens);
        self.lists.truncate(marks.lists);
        self.free_runs = marks.free_runs;
        self.free_lists = marks.free_lists;
        self.args.restore_len(marks.args);
        if self.tokens_journal.armed() {
            self.tokens_journal.rebase(marks.tokens);
            self.lists_journal.rebase(marks.lists);
        }
    }

    pub(crate) fn reset_for_load(&mut self, marks: TokenStoreMarks) {
        self.tokens = vec![Token(0); marks.tokens];
        self.lists = vec![ListSlot::zeroed(); marks.lists];
        self.free_runs = marks.free_runs;
        self.free_lists = marks.free_lists;
        self.args.clear();
    }

    pub(crate) fn is_consistent(&self) -> bool {
        let tokens = self.tokens.len();
        let in_tokens =
            |start: u32, len: u32| (start as usize).saturating_add(len as usize) <= tokens;
        let lists_ok = self
            .lists
            .iter()
            .all(|slot| slot.refs == 0 || in_tokens(slot.start, slot.len));
        let runs_ok = self
            .free_runs
            .iter()
            .zip(CLASS_CAPS)
            .all(|(&head, cap)| head == NONE || in_tokens(head, cap));
        let lists_free_ok =
            self.free_lists == NONE || (self.free_lists as usize) < self.lists.len();
        tokens >= RESERVED && lists_ok && runs_ok && lists_free_ok
    }

    pub(crate) fn parts_mut(&mut self) -> (&mut [Token], &mut [ListSlot]) {
        (&mut self.tokens, &mut self.lists)
    }

    fn alloc_run(&mut self, len: u32) -> Result<u32, Abort> {
        let class = class_for(len);
        let head = self.free_runs[class];
        if head != NONE {
            self.free_runs[class] = self.tokens[head as usize].0.cast_unsigned();
            return Ok(head);
        }
        let start = self.tokens.len();
        let cap = CLASS_CAPS[class] as usize;
        if start + cap > MAX_TOKENS {
            return Err(Abort::AvailOverflow);
        }
        self.tokens.resize(start + cap, Token(0));
        u32::try_from(start).map_err(|_| Abort::AvailOverflow)
    }

    fn free_run(&mut self, start: u32, len: u32) {
        if len == 0 {
            return;
        }
        let class = class_for(len);
        let next = Token(self.free_runs[class].cast_signed());
        self.set_token(start as usize, next);
        self.free_runs[class] = start;
    }

    fn alloc_list(&mut self, start: u32, len: u32) -> Result<TokenList, Abort> {
        let slot = ListSlot {
            start,
            len,
            refs: 1,
        };
        let i = if self.free_lists != NONE {
            let i = self.free_lists as usize;
            self.free_lists = self.lists[i].start;
            self.set_slot(i, slot);
            i
        } else {
            self.lists.push(slot);
            self.lists.len() - 1
        };
        i32::try_from(i)
            .map(TokenList)
            .map_err(|_| Abort::AvailOverflow)
    }

    fn store_list(&mut self, tokens: &[Token]) -> Result<TokenList, Abort> {
        let len = u32::try_from(tokens.len()).map_err(|_| Abort::AvailOverflow)?;
        let start = if len == 0 { 0 } else { self.alloc_run(len)? };
        let at = start as usize;
        if self.tokens_journal.armed() {
            self.tokens_journal
                .note_run(at, &self.tokens[at..at + tokens.len()]);
        }
        self.tokens[at..at + tokens.len()].copy_from_slice(tokens);
        self.alloc_list(start, len)
    }

    pub(crate) fn init_omit_template(&mut self, t: Token) {
        self.tokens[0] = t;
        self.lists[1] = ListSlot {
            start: 0,
            len: 1,
            refs: 1,
        };
    }

    #[inline(always)]
    pub(crate) fn cursor(&self, list: TokenList) -> ListCursor {
        match self.slot_of(list) {
            Some(slot) => ListCursor {
                pos: slot.start,
                end: slot.start + slot.len,
            },
            None => ListCursor::END,
        }
    }

    #[inline]
    pub(crate) fn back_up(&mut self, depth: usize, t: Token) -> Result<TokCursor, Abort> {
        let pos = u32::try_from(depth)
            .ok()
            .filter(|_| (1..=BACKUP_SLOTS).contains(&depth))
            .ok_or(Abort::Confusion("backup slot"))?;
        self.set_token(depth, t);
        Ok(TokCursor { pos, end: pos + 1 })
    }

    /// Where a token-list level reading `list`, now at `loc`, began. A level
    /// without a list reads a backup slot, the token before `loc.end`.
    pub(crate) fn level_start(&self, list: TokenList, loc: TokCursor) -> TokCursor {
        let slot = loc.end.wrapping_sub(1) as usize;
        if list.is_null() && (1..=BACKUP_SLOTS).contains(&slot) {
            TokCursor {
                pos: loc.end - 1,
                end: loc.end,
            }
        } else {
            self.cursor(list).into()
        }
    }

    #[inline(always)]
    pub(crate) fn peek<C: Cursor>(&self, c: C) -> Option<Token> {
        if c.is_end() { None } else { c.token(self) }
    }

    /// A cursor reading the argument `arg`.
    pub(crate) fn arg_cursor(&self, arg: ArgRef) -> TokCursor {
        TokCursor {
            pos: arg.start() | ARG_TAG,
            end: arg.end() | ARG_TAG,
        }
    }

    #[inline(always)]
    pub(crate) fn next<C: Cursor>(&self, c: C) -> C {
        debug_assert!(!c.is_end());
        c.advanced()
    }

    #[inline(always)]
    pub(crate) fn read<C: Cursor>(&self, c: &mut C) -> Option<Token> {
        let t = self.peek(*c)?;
        *c = self.next(*c);
        Some(t)
    }

    pub(crate) fn iter(&self, list: TokenList) -> TokIter<'_> {
        TokIter {
            store: self,
            at: self.cursor(list),
        }
    }

    /// Whether `c` lies in the store, for printing a possibly clobbered list.
    pub(crate) fn holds(&self, c: impl Into<TokCursor>) -> bool {
        let c = c.into();
        if c.end & ARG_TAG == 0 {
            c.end as usize <= self.tokens.len()
        } else {
            (c.end & !ARG_TAG) as usize <= self.args.len()
        }
    }

    /// Whether `list` is a live list of the store, for printing a possibly clobbered list.
    pub(crate) fn holds_list(&self, list: TokenList) -> bool {
        self.slot_of(list).is_some_and(|slot| slot.refs > 0)
    }

    pub(crate) fn is_empty(&self, list: TokenList) -> bool {
        self.slot_of(list).is_none_or(|slot| slot.len == 0)
    }

    pub(crate) fn begin(&mut self) -> Result<Build, Abort> {
        self.builds.push((self.build.len(), 0));
        Ok(Build(self.builds.len() - 1))
    }

    #[inline(always)]
    fn top(&self, b: &Build) -> usize {
        debug_assert_eq!(b.0 + 1, self.builds.len(), "token list builds out of order");
        b.0
    }

    #[inline(always)]
    pub(crate) fn push(&mut self, b: &Build, t: Token) -> Result<(), Abort> {
        self.top(b);
        if self.build.len() >= MAX_TOKENS {
            return Err(Abort::AvailOverflow);
        }
        self.build.push(t);
        Ok(())
    }

    #[inline]
    pub(crate) fn last_pushed(&self, b: &Build) -> Option<Token> {
        let (start, skip) = self.builds[self.top(b)];
        if self.build.len() > start + skip {
            self.build.last().copied()
        } else {
            None
        }
    }

    pub(crate) fn push_list(&mut self, b: &Build, list: TokenList) -> Result<(), Abort> {
        self.top(b);
        if let Some(slot) = self.slot_of(list) {
            let (start, end) = (slot.start as usize, (slot.start + slot.len) as usize);
            self.build.extend_from_slice(&self.tokens[start..end]);
        }
        self.flush(list);
        Ok(())
    }

    pub(crate) fn innermost_built(&self) -> Vec<Token> {
        self.builds.last().map_or_else(Vec::new, |&(start, skip)| {
            self.build[start + skip..].to_vec()
        })
    }

    pub(crate) fn finish(&mut self, b: Build) -> Result<TokenList, Abort> {
        let (start, skip) = self.builds[self.top(&b)];
        self.builds.pop();
        let built = mem::take(&mut self.build);
        let list = self.store_list(&built[start + skip..]);
        self.build = built;
        self.build.truncate(start);
        list
    }

    /// Starts collecting a macro call's arguments.
    pub(crate) fn begin_args(&self) -> BuiltArgs {
        BuiltArgs {
            base: self.build.len(),
            args: [ArgRef::default(); 9],
            n: 0,
        }
    }

    /// Ends build `b` as the next argument in `args`, leaving its tokens in the build buffer.
    #[inline]
    pub(crate) fn finish_arg(&mut self, args: &mut BuiltArgs, b: Build) -> Result<(), Abort> {
        let (start, skip) = self.builds[self.top(&b)];
        self.builds.pop();
        let slot = args
            .args
            .get_mut(args.n)
            .ok_or(Abort::Confusion("macro arguments"))?;
        let from = start + skip;
        let offset = u32::try_from(from - args.base).map_err(|_| Abort::AvailOverflow)?;
        let len = u32::try_from(self.build.len() - from).map_err(|_| Abort::AvailOverflow)?;
        *slot = ArgRef::new(offset, len);
        args.n += 1;
        Ok(())
    }

    /// The tokens of built argument `k`.
    pub(crate) fn built_arg(&self, args: &BuiltArgs, k: usize) -> &[Token] {
        let arg = args.args[k];
        &self.build[args.base + arg.start() as usize..args.base + arg.end() as usize]
    }

    /// Moves the built arguments onto the argument stack, with the tokens
    /// between them, and returns where they landed and how many there are.
    #[inline]
    pub(crate) fn push_args(&mut self, mut args: BuiltArgs) -> Result<([ArgRef; 9], usize), Abort> {
        let base = args.base;
        if self.tokens.len() + self.args.len() + (self.build.len() - base) > MAX_TOKENS {
            return Err(Abort::AvailOverflow);
        }
        self.args
            .push_all(&self.build[base..], &mut args.args[..args.n])?;
        self.build.truncate(base);
        Ok((args.args, args.n))
    }

    /// Drops built arguments that never reach the argument stack.
    pub(crate) fn drop_args(&mut self, args: BuiltArgs) {
        self.build.truncate(args.base);
    }

    /// Drops `arg` and every argument pushed after it.
    pub(crate) fn pop_args_to(&mut self, arg: ArgRef) {
        self.args.pop_to(arg);
    }

    pub(crate) fn abandon(&mut self, b: Build) {
        let (start, _) = self.builds[self.top(&b)];
        self.builds.pop();
        self.build.truncate(start);
    }

    pub(crate) fn drop_first_last(&mut self, b: &Build) {
        let i = self.top(b);
        debug_assert!(self.build.len() >= self.builds[i].0 + 2 && self.builds[i].1 == 0);
        self.build.pop();
        self.builds[i].1 = 1;
    }

    pub(crate) fn single(&mut self, t: Token) -> Result<TokenList, Abort> {
        self.store_list(&[t])
    }

    fn replace_run(&mut self, list: TokenList, tokens: &[Token]) -> Result<(), Abort> {
        let (Some(i), Some(slot)) = (list.slot(), self.slot_of(list)) else {
            return Err(Abort::Confusion("token list"));
        };
        let len = u32::try_from(tokens.len()).map_err(|_| Abort::AvailOverflow)?;
        self.free_run(slot.start, slot.len);
        let start = self.alloc_run(len)?;
        let at = start as usize;
        if self.tokens_journal.armed() {
            self.tokens_journal
                .note_run(at, &self.tokens[at..at + tokens.len()]);
        }
        self.tokens[at..at + tokens.len()].copy_from_slice(tokens);
        self.set_slot(i, ListSlot { start, len, ..slot });
        Ok(())
    }

    pub(crate) fn prepend(&mut self, list: TokenList, t: Token) -> Result<(), Abort> {
        let mut tokens = vec![t];
        tokens.extend(self.iter(list));
        self.replace_run(list, &tokens)
    }

    pub(crate) fn append(&mut self, list: TokenList, t: Token) -> Result<(), Abort> {
        let mut tokens: Vec<Token> = self.iter(list).collect();
        tokens.push(t);
        self.replace_run(list, &tokens)
    }

    pub(crate) fn map_tokens(&mut self, list: TokenList, mut f: impl FnMut(Token) -> Token) {
        if let Some(slot) = self.slot_of(list) {
            for i in slot.start as usize..(slot.start + slot.len) as usize {
                let t = f(self.tokens[i]);
                self.set_token(i, t);
            }
        }
    }

    pub(crate) fn flush(&mut self, list: TokenList) {
        let (Some(i), Some(slot)) = (list.slot(), self.slot_of(list)) else {
            return;
        };
        self.free_run(slot.start, slot.len);
        self.set_slot(
            i,
            ListSlot {
                start: self.free_lists,
                len: 0,
                refs: 0,
            },
        );
        self.free_lists = u32::try_from(i).unwrap_or(NONE);
    }

    #[inline]
    pub(crate) fn add_ref(&mut self, list: TokenList) {
        if let (Some(i), Some(slot)) = (list.slot(), self.slot_of(list)) {
            self.set_slot(
                i,
                ListSlot {
                    refs: slot.refs + 1,
                    ..slot
                },
            );
        }
    }

    pub(crate) fn delete_ref(&mut self, list: TokenList) {
        let (Some(i), Some(slot)) = (list.slot(), self.slot_of(list)) else {
            return;
        };
        if slot.refs <= 1 {
            self.flush(list);
        } else {
            self.set_slot(
                i,
                ListSlot {
                    refs: slot.refs - 1,
                    ..slot
                },
            );
        }
    }
}

pub(crate) struct TokIter<'a> {
    store: &'a TokenStore,
    at: ListCursor,
}

impl Iterator for TokIter<'_> {
    type Item = Token;

    #[inline]
    fn next(&mut self) -> Option<Token> {
        self.store.read(&mut self.at)
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use super::*;

    fn tokens(store: &TokenStore, list: TokenList) -> Vec<i32> {
        store.iter(list).map(|t| t.0).collect()
    }

    fn list_of(store: &mut TokenStore, values: &[i32]) -> TokenList {
        let b = store.begin().unwrap();
        for &v in values {
            store.push(&b, Token(v)).unwrap();
        }
        store.finish(b).unwrap()
    }

    #[test]
    fn popping_braced_arguments_leaves_the_argument_stack_empty() {
        let mut store = TokenStore::default();
        let mut built = store.begin_args();
        for arg in [[1, 2, 3], [4, 5, 6]] {
            let b = store.begin().unwrap();
            for v in arg {
                store.push(&b, Token(v)).unwrap();
            }
            store.drop_first_last(&b);
            store.finish_arg(&mut built, b).unwrap();
        }
        let (args, n) = store.push_args(built).unwrap();
        let read = |a: ArgRef| {
            let mut c = store.arg_cursor(a);
            iter::from_fn(|| store.read(&mut c).map(|t| t.0)).collect::<Vec<_>>()
        };
        assert_eq!((read(args[0]), read(args[1]), n), (vec![2], vec![5], 2));
        store.pop_args_to(args[0]);
        assert_eq!(store.live_args(), []);
    }

    #[test]
    fn every_length_fits_its_class_and_not_the_one_below() {
        for len in 1..=100_000u32 {
            let class = class_for(len);
            assert!(CLASS_CAPS[class] >= len, "{len}");
            assert!(CLASS_CAPS[class - 1] < len, "{len}");
        }
        assert!(CLASS_CAPS[N_CLASSES - 1] as usize >= MAX_TOKENS);
    }

    #[test]
    fn a_freed_run_is_reused_by_the_next_list_of_its_class() {
        let mut store = TokenStore::default();
        let a = list_of(&mut store, &[1, 2, 3]);
        let b = list_of(&mut store, &[4, 5, 6]);
        let (start_a, start_b) = (store.cursor(a).pos, store.cursor(b).pos);
        let len = store.tokens().len();
        store.flush(a);
        store.flush(b);
        let c = list_of(&mut store, &[7, 8, 9]);
        let d = list_of(&mut store, &[10, 11, 12]);
        assert_eq!(
            (store.cursor(c).pos, store.cursor(d).pos),
            (start_b, start_a)
        );
        assert_eq!(store.tokens().len(), len);
        assert_eq!(
            (tokens(&store, c), tokens(&store, d)),
            (vec![7, 8, 9], vec![10, 11, 12])
        );
        assert_eq!((c, d), (b, a));
    }

    #[test]
    fn the_last_reference_frees_a_list() {
        let mut store = TokenStore::default();
        let a = list_of(&mut store, &[1]);
        store.add_ref(a);
        store.delete_ref(a);
        assert!(store.holds_list(a));
        store.delete_ref(a);
        assert!(!store.holds_list(a));
    }

    #[test]
    fn nested_builds_end_innermost_first() {
        let mut store = TokenStore::default();
        let outer = store.begin().unwrap();
        store.push(&outer, Token(1)).unwrap();
        let inner = store.begin().unwrap();
        store.push(&inner, Token(2)).unwrap();
        assert_eq!(store.innermost_built(), [Token(2)]);
        let inner = store.finish(inner).unwrap();
        store.push(&outer, Token(3)).unwrap();
        store.push_list(&outer, inner).unwrap();
        let outer = store.finish(outer).unwrap();
        assert_eq!(tokens(&store, outer), [1, 3, 2]);
        assert_eq!(outer, inner, "the entry `push_list` freed is reused");
    }

    #[test]
    fn dropping_an_arguments_braces_keeps_its_inside() {
        let mut store = TokenStore::default();
        for (built, inside) in [(vec![10, 1, 2, 20], vec![1, 2]), (vec![10, 20], vec![])] {
            let b = store.begin().unwrap();
            for &v in &built {
                store.push(&b, Token(v)).unwrap();
            }
            store.drop_first_last(&b);
            assert_eq!(store.last_pushed(&b), inside.last().copied().map(Token));
            let list = store.finish(b).unwrap();
            assert_eq!(tokens(&store, list), inside);
            assert_eq!(store.is_empty(list), inside.is_empty());
        }
    }

    #[test]
    fn a_backup_slot_is_read_and_shown_as_a_one_token_level() {
        let mut store = TokenStore::default();
        let runs_from = store.tokens().len();
        let mut at = store.back_up(7, Token(42)).unwrap();
        let start = store.level_start(TokenList::NULL, at);
        assert_eq!(store.read(&mut at), Some(Token(42)));
        assert!(at.is_end());
        assert_eq!(store.level_start(TokenList::NULL, at), start);
        assert_eq!(store.read(&mut start.clone()), Some(Token(42)));
        assert_eq!(
            store.level_start(TokenList::NULL, TokCursor::END),
            TokCursor::END
        );
        assert!(store.back_up(0, Token(1)).is_err());
        assert!(store.back_up(BACKUP_SLOTS + 1, Token(1)).is_err());
        let a = list_of(&mut store, &[1]);
        assert_eq!(store.cursor(a).pos as usize, runs_from);
    }

    #[test]
    fn prepend_and_append_move_a_fresh_list_to_a_larger_run() {
        let mut store = TokenStore::default();
        let a = list_of(&mut store, &[2]);
        store.prepend(a, Token(1)).unwrap();
        store.append(a, Token(3)).unwrap();
        store.map_tokens(a, |t| Token(t.0 * 10));
        assert_eq!(tokens(&store, a), [10, 20, 30]);
        let one = list_of(&mut store, &[5]);
        assert_eq!(tokens(&store, one), [5]);
        assert_eq!(tokens(&store, a), [10, 20, 30]);
    }
}
