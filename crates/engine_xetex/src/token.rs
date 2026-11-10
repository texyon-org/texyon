//! TeX tokens: `cmd * max_char_val + chr` for a character, `cs_token_flag + p`
//! for a control sequence.

use crate::cmd::Cmd;
use crate::consts::{CS_TOKEN_FLAG, TEX_NULL};
use crate::errors::Abort;
use crate::journal::WordJournal;
use crate::units::EqtbPtr;

/// One more than the largest character code a token can carry.
pub(crate) const MAX_CHAR_VAL: i32 = 0x20_0000;

/// A token as stored in token lists and `cur_tok`. Ordered like its raw value,
/// so brace and parameter tests are range checks as in TeX.
#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default, bytemuck::Pod, bytemuck::Zeroable,
)]
#[repr(transparent)]
pub(crate) struct Token(pub(crate) i32);

impl Token {
    /// `cur_cmd = cur_chr = 0`: the end of a `\read` line, or no pending
    /// `\afterassignment` token.
    pub(crate) const ZERO: Token = Token(0);
    pub(crate) const SPACE: Token = Token::new(Cmd::Spacer, b' ' as i32);
    /// Marks a `\protected` macro, right after its reference count.
    pub(crate) const PROTECTED: Token = Token(Token::END_MATCH.0 + 1);
    pub(crate) const END_MATCH: Token = Token::new(Cmd::END_MATCH, 0);

    /// The character token of category `cmd`.
    #[inline(always)]
    pub(crate) const fn new(cmd: Cmd, chr: i32) -> Token {
        Token(cmd.raw() * MAX_CHAR_VAL + chr)
    }

    /// The token of control sequence `p`.
    #[inline(always)]
    pub(crate) const fn from_cs(p: EqtbPtr) -> Token {
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

    /// The `#n` parameter token in a macro body.
    #[inline(always)]
    pub(crate) const fn out_param(n: i32) -> Token {
        Token::new(Cmd::OUT_PARAM, n)
    }

    /// The `#` of a parameter in a macro's parameter text, with its catcode 6 character.
    #[inline(always)]
    pub(crate) const fn match_(c: i32) -> Token {
        Token::new(Cmd::MATCH, c)
    }

    #[inline(always)]
    pub(crate) const fn is_cs(self) -> bool {
        self.0 >= CS_TOKEN_FLAG
    }

    /// The control sequence of a [`Token::is_cs`] token.
    #[inline(always)]
    pub(crate) const fn cs(self) -> EqtbPtr {
        EqtbPtr::from_raw(self.0 - CS_TOKEN_FLAG)
    }

    /// The category of a character token.
    #[inline(always)]
    pub(crate) const fn cmd(self) -> Cmd {
        Cmd::from_catcode_bits(self.0 / MAX_CHAR_VAL)
    }

    /// The character code of a character token.
    #[inline(always)]
    pub(crate) const fn chr(self) -> i32 {
        self.0 % MAX_CHAR_VAL
    }

    /// `tok < left_brace_limit`.
    #[inline(always)]
    pub(crate) const fn is_left_brace(self) -> bool {
        self.0 < LEFT_BRACE_LIMIT
    }

    /// `tok < right_brace_limit`: a left or right brace.
    #[inline(always)]
    pub(crate) const fn is_brace(self) -> bool {
        self.0 < RIGHT_BRACE_LIMIT
    }

    /// A `match` or `end_match` token of a parameter text.
    #[inline(always)]
    pub(crate) const fn is_match(self) -> bool {
        self.0 >= Token::new(Cmd::MATCH, 0).0 && self.0 <= Token::END_MATCH.0
    }

    /// The digit of an other-character `0`..`9` token.
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

/// A pointer into a token list: its reference-count head, one of its token
/// nodes, or null. It indexes the [`TokenStore`], not `mem`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub(crate) struct TokenList(i32);

/// The null list. Not the zero bit pattern `Zeroable` gives.
impl Default for TokenList {
    #[inline]
    fn default() -> Self {
        Self::NULL
    }
}

impl TokenList {
    pub(crate) const NULL: TokenList = TokenList(TEX_NULL);

    /// The list from its raw form in an eqtb or save stack word, a node field or a format.
    #[inline(always)]
    pub(crate) const fn from_raw(raw: i32) -> TokenList {
        TokenList(raw)
    }

    /// The raw form for an eqtb or save stack word, a node field or a format.
    #[inline(always)]
    pub(crate) const fn raw(self) -> i32 {
        self.0
    }

    #[inline(always)]
    pub(crate) fn is_null(self) -> bool {
        self.0 == TEX_NULL
    }

    #[inline(always)]
    fn slot(self) -> usize {
        self.0 as usize
    }

    #[inline(always)]
    pub(crate) fn link(self, toks: &TokenStore) -> TokenList {
        toks.data[self.slot()].link
    }

    #[inline(always)]
    pub(crate) fn set_link(self, toks: &mut TokenStore, q: TokenList) {
        toks.node_mut(self).link = q;
    }

    /// The token held by this node.
    #[inline(always)]
    pub(crate) fn token(self, toks: &TokenStore) -> Token {
        Token(toks.data[self.slot()].info)
    }

    #[inline(always)]
    pub(crate) fn set_token(self, toks: &mut TokenStore, t: Token) {
        toks.node_mut(self).info = t.0;
    }

    /// The reference count in the head node, one less than the number of
    /// references.
    #[inline(always)]
    pub(crate) fn ref_count(self, toks: &TokenStore) -> i32 {
        toks.data[self.slot()].info
    }

    #[inline(always)]
    pub(crate) fn set_ref_count(self, toks: &mut TokenStore, v: i32) {
        toks.node_mut(self).info = v;
    }
}

/// One token node: a token, or a list's reference count in its head, and the next node.
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub(crate) struct TokNode {
    info: i32,
    link: TokenList,
}

impl TokNode {
    const FRESH: TokNode = TokNode {
        info: 0,
        link: TokenList::NULL,
    };
}

/// Scratch heads for building token lists.
pub(crate) const TEMP_HEAD_TOKS: TokenList = TokenList(0);
pub(crate) const HOLD_HEAD_TOKS: TokenList = TokenList(1);
/// The one-token list `\endtemplate` that ends an omitted template.
pub(crate) const OMIT_TEMPLATE: TokenList = TokenList(2);
/// Where `scan_keyword` keeps the tokens it backs up.
pub(crate) const BACKUP_HEAD: TokenList = TokenList(3);
pub(crate) const FIXED_HEADS: usize = 4;

/// The token lists, kept apart from `mem`. Nodes are one word each and freed to `avail`.
pub(crate) struct TokenStore {
    data: Vec<TokNode>,
    avail: TokenList,
    pub(crate) journal: WordJournal<TokNode>,
}

impl Default for TokenStore {
    fn default() -> Self {
        Self {
            data: vec![TokNode::FRESH; FIXED_HEADS],
            avail: TokenList::NULL,
            journal: WordJournal::default(),
        }
    }
}

/// The token store's scalars, for checkpoints and formats.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct TokenStoreMarks {
    pub(crate) len: usize,
    pub(crate) avail: TokenList,
}

impl TokenStore {
    #[inline(always)]
    fn node_mut(&mut self, p: TokenList) -> &mut TokNode {
        let i = p.slot();
        if self.journal.armed() {
            self.journal.note(i, self.data[i]);
        }
        &mut self.data[i]
    }

    /// The nodes in use, fixed heads included, for checkpoints and digests.
    pub(crate) fn nodes(&self) -> &[TokNode] {
        &self.data
    }

    /// Closes the journal's epoch, see [`WordJournal::seal`].
    pub(crate) fn seal_journal(&mut self) -> crate::journal::WordDelta<TokNode> {
        self.journal.seal(&self.data)
    }

    pub(crate) fn nodes_mut(&mut self) -> &mut [TokNode] {
        &mut self.data
    }

    /// Whether `p` is a node of this store, for printing a possibly clobbered list.
    pub(crate) fn contains(&self, p: TokenList) -> bool {
        usize::try_from(p.0).is_ok_and(|i| i < self.data.len())
    }

    pub(crate) fn marks(&self) -> TokenStoreMarks {
        TokenStoreMarks {
            len: self.data.len(),
            avail: self.avail,
        }
    }

    /// Rewinds to `marks`, after the journal put back the words that existed then.
    pub(crate) fn restore_marks(&mut self, marks: TokenStoreMarks) {
        self.data.truncate(marks.len);
        self.avail = marks.avail;
        if self.journal.armed() {
            self.journal.rebase(marks.len);
        }
    }

    /// Empties the store to `len` fresh nodes with free list `avail`, for a format load to fill.
    pub(crate) fn reset_for_load(&mut self, len: usize, avail: TokenList) {
        self.data = vec![TokNode::FRESH; len];
        self.avail = avail;
    }

    /// TeX's `get_avail` for a token node.
    #[inline(always)]
    pub(crate) fn get_avail(&mut self) -> Result<TokenList, Abort> {
        let p = self.avail;
        if p.is_null() {
            return self.grow();
        }
        self.avail = p.link(self);
        p.set_link(self, TokenList::NULL);
        Ok(p)
    }

    #[cold]
    fn grow(&mut self) -> Result<TokenList, Abort> {
        let p = i32::try_from(self.data.len()).map_err(|_| Abort::AvailOverflow)?;
        self.data.push(TokNode::FRESH);
        Ok(TokenList(p))
    }

    /// Returns the one node `p` to the free list.
    #[inline]
    pub(crate) fn free_avail(&mut self, p: TokenList) {
        p.set_link(self, self.avail);
        self.avail = p;
    }

    /// Returns the list `p` to the free list.
    pub(crate) fn flush_list(&mut self, p: TokenList) {
        if p.is_null() {
            return;
        }
        let tail = self.list_tail(p);
        self.flush_list_with_tail(p, tail);
    }

    /// `flush_list` for a list whose last node `tail` the caller already knows.
    #[inline]
    pub(crate) fn flush_list_with_tail(&mut self, p: TokenList, tail: TokenList) {
        if p.is_null() {
            return;
        }
        debug_assert_eq!(self.list_tail(p), tail);
        tail.set_link(self, self.avail);
        self.avail = p;
    }

    /// The last node of the non-empty list starting at `p`.
    #[inline]
    pub(crate) fn list_tail(&self, mut p: TokenList) -> TokenList {
        while !p.link(self).is_null() {
            p = p.link(self);
        }
        p
    }

    /// TeX's `store_new_token`: appends `tok` after `tail` and makes it the tail.
    #[inline(always)]
    pub(crate) fn store_new_token(
        &mut self,
        tail: &mut TokenList,
        tok: Token,
    ) -> Result<(), Abort> {
        let q = self.get_avail()?;
        tail.set_link(self, q);
        q.set_token(self, tok);
        *tail = q;
        Ok(())
    }

    #[inline]
    pub(crate) fn add_ref(&mut self, p: TokenList) {
        p.set_ref_count(self, p.ref_count(self) + 1);
    }

    /// Drops a reference to token list `p`, freeing it with the last one.
    pub(crate) fn delete_ref(&mut self, p: TokenList) {
        if p.ref_count(self) == TEX_NULL {
            self.flush_list(p);
        } else {
            p.set_ref_count(self, p.ref_count(self) - 1);
        }
    }
}
