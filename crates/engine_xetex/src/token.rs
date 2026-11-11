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
pub(crate) struct TokPtr(i32);

/// The null list. Not the zero bit pattern `Zeroable` gives.
impl Default for TokPtr {
    #[inline]
    fn default() -> Self {
        Self::NULL
    }
}

impl TokPtr {
    pub(crate) const NULL: TokPtr = TokPtr(TEX_NULL);

    /// The list from its raw form in an eqtb or save stack word, a node field or a format.
    #[inline(always)]
    pub(crate) const fn from_raw(raw: i32) -> TokPtr {
        TokPtr(raw)
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
    pub(crate) fn link(self, toks: &TokenStore) -> TokPtr {
        toks.data[self.slot()].link
    }

    #[inline(always)]
    pub(crate) fn set_link(self, toks: &mut TokenStore, q: TokPtr) {
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
    link: TokPtr,
}

impl TokNode {
    const FRESH: TokNode = TokNode {
        info: 0,
        link: TokPtr::NULL,
    };
}

/// The head and the token of the list `\endtemplate` that ends an omitted template.
const OMIT_TEMPLATE_HEAD: TokPtr = TokPtr(0);
const OMIT_TEMPLATE_TOKEN: TokPtr = TokPtr(1);
/// The nodes of the fixed lists, at the start of the store.
pub(crate) const FIXED_HEADS: usize = 2;
/// Half of `MAIN_MEMORY_WORDS` in two-word token nodes, until nodes and tokens share one budget.
const MAX_TOKEN_NODES: usize = 1 << 24;

/// The token lists, kept apart from `mem`. Nodes are one word each and freed to `avail`.
pub(crate) struct TokenStore {
    data: Vec<TokNode>,
    avail: TokPtr,
    pub(crate) journal: WordJournal<TokNode>,
    /// The lists being built, innermost last: each head and tail.
    builds: Vec<(TokPtr, TokPtr)>,
}

impl Default for TokenStore {
    fn default() -> Self {
        Self {
            data: vec![TokNode::FRESH; FIXED_HEADS],
            avail: TokPtr::NULL,
            journal: WordJournal::default(),
            builds: Vec::new(),
        }
    }
}

/// The token store's scalars, for checkpoints and formats.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct TokenStoreMarks {
    pub(crate) len: usize,
    pub(crate) avail: TokPtr,
}

impl TokenStore {
    #[inline(always)]
    fn node_mut(&mut self, p: TokPtr) -> &mut TokNode {
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
    pub(crate) fn contains(&self, p: TokPtr) -> bool {
        usize::try_from(p.0).is_ok_and(|i| i < self.data.len())
    }

    pub(crate) fn marks(&self) -> TokenStoreMarks {
        debug_assert!(
            self.builds.is_empty(),
            "checkpoint while a token list is built"
        );
        TokenStoreMarks {
            len: self.data.len(),
            avail: self.avail,
        }
    }

    /// Rewinds to `marks`, after the journal put back the words that existed then.
    pub(crate) fn restore_marks(&mut self, marks: TokenStoreMarks) {
        self.builds.clear();
        self.data.truncate(marks.len);
        self.avail = marks.avail;
        if self.journal.armed() {
            self.journal.rebase(marks.len);
        }
    }

    /// Empties the store to `len` fresh nodes with free list `avail`, for a format load to fill.
    pub(crate) fn reset_for_load(&mut self, len: usize, avail: TokPtr) {
        self.data = vec![TokNode::FRESH; len];
        self.avail = avail;
    }

    /// TeX's `get_avail` for a token node.
    #[inline(always)]
    fn get_avail(&mut self) -> Result<TokPtr, Abort> {
        let p = self.avail;
        if p.is_null() {
            return self.grow();
        }
        self.avail = p.link(self);
        p.set_link(self, TokPtr::NULL);
        Ok(p)
    }

    #[cold]
    fn grow(&mut self) -> Result<TokPtr, Abort> {
        if self.data.len() >= MAX_TOKEN_NODES {
            return Err(Abort::AvailOverflow);
        }
        let p = i32::try_from(self.data.len()).map_err(|_| Abort::AvailOverflow)?;
        self.data.push(TokNode::FRESH);
        Ok(TokPtr(p))
    }

    /// Returns the one node `p` to the free list.
    #[inline]
    fn free_avail(&mut self, p: TokPtr) {
        p.set_link(self, self.avail);
        self.avail = p;
    }

    /// Returns the list `p` to the free list.
    fn flush_list(&mut self, p: TokPtr) {
        if p.is_null() {
            return;
        }
        let tail = self.list_tail(p);
        tail.set_link(self, self.avail);
        self.avail = p;
    }

    /// The last node of the non-empty list starting at `p`.
    #[inline]
    fn list_tail(&self, mut p: TokPtr) -> TokPtr {
        while !p.link(self).is_null() {
            p = p.link(self);
        }
        p
    }

    /// TeX's `store_new_token`: appends `tok` after `tail` and makes it the tail.
    #[inline(always)]
    fn store_new_token(&mut self, tail: &mut TokPtr, tok: Token) -> Result<(), Abort> {
        let q = self.get_avail()?;
        tail.set_link(self, q);
        q.set_token(self, tok);
        *tail = q;
        Ok(())
    }

    #[inline]
    pub(crate) fn add_ref(&mut self, list: TokenList) {
        let p = list.0;
        p.set_ref_count(self, p.ref_count(self) + 1);
    }

    /// Drops a reference to `list`, freeing it with the last one.
    pub(crate) fn delete_ref(&mut self, list: TokenList) {
        let p = list.0;
        if p.ref_count(self) == TEX_NULL {
            self.flush_list(p);
        } else {
            p.set_ref_count(self, p.ref_count(self) - 1);
        }
    }
}

/// A token list: its tokens and a reference count. Null for no list.
#[derive(Copy, Clone, PartialEq, Eq, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub(crate) struct TokenList(TokPtr);

/// The null list. Not the zero bit pattern `Zeroable` gives.
impl Default for TokenList {
    #[inline]
    fn default() -> Self {
        Self::NULL
    }
}

impl TokenList {
    pub(crate) const NULL: TokenList = TokenList(TokPtr::NULL);
    /// The one-token list `\endtemplate` that ends an omitted template.
    pub(crate) const OMIT_TEMPLATE: TokenList = TokenList(OMIT_TEMPLATE_HEAD);

    /// The list from its raw form in an eqtb or save stack word, a node field or a format.
    #[inline(always)]
    pub(crate) const fn from_raw(raw: i32) -> TokenList {
        TokenList(TokPtr::from_raw(raw))
    }

    /// The raw form for an eqtb or save stack word, a node field or a format.
    #[inline(always)]
    pub(crate) const fn raw(self) -> i32 {
        self.0.raw()
    }

    #[inline(always)]
    pub(crate) fn is_null(self) -> bool {
        self.0.is_null()
    }
}

/// Where a reader of a token list is: the next token, or the end.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct TokCursor(TokPtr);

impl TokCursor {
    pub(crate) const END: TokCursor = TokCursor(TokPtr::NULL);

    #[inline(always)]
    pub(crate) fn is_end(self) -> bool {
        self.0.is_null()
    }

    /// The position as a number, for checkpoint digests.
    pub(crate) fn raw(self) -> i32 {
        self.0.raw()
    }
}

/// A token list being built. Builds nest and end innermost first.
#[must_use]
#[derive(Debug)]
pub(crate) struct Build(usize);

impl TokenStore {
    /// A reader at the first token of `list`, at the end for the null list.
    #[inline(always)]
    pub(crate) fn cursor(&self, list: TokenList) -> TokCursor {
        if list.is_null() {
            TokCursor::END
        } else {
            TokCursor(list.0.link(self))
        }
    }

    /// The token at `c`, `None` at the end.
    #[inline(always)]
    pub(crate) fn peek(&self, c: TokCursor) -> Option<Token> {
        (!c.is_end()).then(|| c.0.token(self))
    }

    /// The position after the token at `c`, which must not be the end.
    #[inline(always)]
    pub(crate) fn next(&self, c: TokCursor) -> TokCursor {
        TokCursor(c.0.link(self))
    }

    /// The token at `c`, moving `c` past it. `None` at the end.
    #[inline(always)]
    pub(crate) fn read(&self, c: &mut TokCursor) -> Option<Token> {
        let t = self.peek(*c)?;
        *c = self.next(*c);
        Some(t)
    }

    /// The tokens of `list`.
    pub(crate) fn iter(&self, list: TokenList) -> TokIter<'_> {
        TokIter {
            store: self,
            at: self.cursor(list),
        }
    }

    /// Whether `c` lies in the store, for printing a possibly clobbered list.
    pub(crate) fn holds(&self, c: TokCursor) -> bool {
        self.contains(c.0)
    }

    /// Whether `list` lies in the store, for printing a possibly clobbered list.
    pub(crate) fn holds_list(&self, list: TokenList) -> bool {
        self.contains(list.0)
    }

    pub(crate) fn is_empty(&self, list: TokenList) -> bool {
        self.cursor(list).is_end()
    }

    /// Starts building a list.
    pub(crate) fn begin(&mut self) -> Result<Build, Abort> {
        let head = self.get_avail()?;
        head.set_ref_count(self, TEX_NULL);
        self.builds.push((head, head));
        Ok(Build(self.builds.len() - 1))
    }

    #[inline(always)]
    fn top(&self, b: &Build) -> usize {
        debug_assert_eq!(b.0 + 1, self.builds.len(), "token list builds out of order");
        b.0
    }

    /// Appends `t` to the list `b` builds.
    #[inline(always)]
    pub(crate) fn push(&mut self, b: &Build, t: Token) -> Result<(), Abort> {
        let i = self.top(b);
        let mut tail = self.builds[i].1;
        self.store_new_token(&mut tail, t)?;
        self.builds[i].1 = tail;
        Ok(())
    }

    /// The last token pushed, `None` while the build is empty.
    #[inline]
    pub(crate) fn last_pushed(&self, b: &Build) -> Option<Token> {
        let (head, tail) = self.builds[b.0];
        (tail != head).then(|| tail.token(self))
    }

    /// Ends the build, giving the list with one reference. It may be empty.
    pub(crate) fn finish(&mut self, b: Build) -> TokenList {
        let i = self.top(&b);
        let (head, _) = self.builds.swap_remove(i);
        TokenList(head)
    }

    /// Moves the tokens of `list` to the end of the build, freeing `list`.
    pub(crate) fn push_list(&mut self, b: &Build, list: TokenList) -> Result<(), Abort> {
        let mut c = self.cursor(list);
        while let Some(t) = self.read(&mut c) {
            self.push(b, t)?;
        }
        self.flush(list);
        Ok(())
    }

    /// What the innermost open build holds so far, for a runaway message.
    pub(crate) fn innermost_built(&self) -> TokCursor {
        self.builds
            .last()
            .map_or(TokCursor::END, |&(head, _)| TokCursor(head.link(self)))
    }

    /// Ends the build and frees what it built.
    pub(crate) fn abandon(&mut self, b: Build) {
        let i = self.top(&b);
        let (head, _) = self.builds.swap_remove(i);
        self.flush_list(head);
    }

    /// Drops the first and last token built, an argument's outer braces.
    pub(crate) fn drop_first_last(&mut self, b: &Build) {
        let i = self.top(b);
        let head = self.builds[i].0;
        let first = head.link(self);
        let mut before_last = first;
        while before_last.link(self) != self.builds[i].1 {
            before_last = before_last.link(self);
        }
        let last = self.builds[i].1;
        self.free_avail(last);
        if before_last == first {
            head.set_link(self, TokPtr::NULL);
            self.builds[i].1 = head;
        } else {
            before_last.set_link(self, TokPtr::NULL);
            head.set_link(self, first.link(self));
            self.builds[i].1 = before_last;
        }
        self.free_avail(first);
    }

    /// Sets up [`TokenList::OMIT_TEMPLATE`] to hold `t`, before the journal is armed.
    pub(crate) fn init_omit_template(&mut self, t: Token) {
        OMIT_TEMPLATE_HEAD.set_ref_count(self, TEX_NULL);
        OMIT_TEMPLATE_HEAD.set_link(self, OMIT_TEMPLATE_TOKEN);
        OMIT_TEMPLATE_TOKEN.set_token(self, t);
        OMIT_TEMPLATE_TOKEN.set_link(self, TokPtr::NULL);
    }

    /// A list of the one token `t`.
    pub(crate) fn single(&mut self, t: Token) -> Result<TokenList, Abort> {
        let b = self.begin()?;
        self.push(&b, t)?;
        Ok(self.finish(b))
    }

    /// Puts `t` in front of the fresh list `list`.
    pub(crate) fn prepend(&mut self, list: TokenList, t: Token) -> Result<(), Abort> {
        let q = self.get_avail()?;
        q.set_token(self, t);
        q.set_link(self, list.0.link(self));
        list.0.set_link(self, q);
        Ok(())
    }

    /// Puts `t` at the end of the fresh list `list`.
    pub(crate) fn append(&mut self, list: TokenList, t: Token) -> Result<(), Abort> {
        let mut tail = self.list_tail(list.0);
        self.store_new_token(&mut tail, t)
    }

    /// Rewrites each token of the fresh list `list` with `f`.
    pub(crate) fn map_tokens(&mut self, list: TokenList, mut f: impl FnMut(Token) -> Token) {
        let mut p = list.0.link(self);
        while !p.is_null() {
            let t = f(p.token(self));
            p.set_token(self, t);
            p = p.link(self);
        }
    }

    /// Frees `list` whatever its reference count.
    pub(crate) fn flush(&mut self, list: TokenList) {
        self.flush_list(list.0);
    }
}

/// The tokens of a list, from [`TokenStore::iter`].
pub(crate) struct TokIter<'a> {
    store: &'a TokenStore,
    at: TokCursor,
}

impl Iterator for TokIter<'_> {
    type Item = Token;

    #[inline]
    fn next(&mut self) -> Option<Token> {
        self.store.read(&mut self.at)
    }
}
