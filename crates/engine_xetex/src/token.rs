//! TeX tokens: `cmd * max_char_val + chr` for a character, `cs_token_flag + p`
//! for a control sequence.

use crate::cmd::Cmd;
use crate::consts::CS_TOKEN_FLAG;
use crate::state::MemState;
use crate::units::{EqtbPtr, MemPtr};

/// One more than the largest character code a token can carry.
pub(crate) const MAX_CHAR_VAL: i32 = 0x20_0000;

/// A token as stored in token lists and `cur_tok`. Ordered like its raw value,
/// so brace and parameter tests are range checks as in TeX.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
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
/// nodes, or null.
#[derive(Copy, Clone, PartialEq, Eq, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub(crate) struct TokenList(MemPtr);

/// The null list. Not the zero bit pattern `Zeroable` gives.
impl Default for TokenList {
    #[inline]
    fn default() -> Self {
        Self::NULL
    }
}

impl TokenList {
    pub(crate) const NULL: TokenList = TokenList(MemPtr::NULL);

    /// The token list a memory word or an overloaded field points at.
    #[inline(always)]
    pub(crate) const fn from_ptr(p: MemPtr) -> TokenList {
        TokenList(p)
    }

    /// The pointer for a memory word or an overloaded field.
    #[inline(always)]
    pub(crate) const fn ptr(self) -> MemPtr {
        self.0
    }

    #[inline(always)]
    pub(crate) fn is_null(self) -> bool {
        self.0.is_null()
    }

    #[inline(always)]
    pub(crate) fn link(self, mem: &MemState) -> TokenList {
        TokenList(self.0.link(mem))
    }

    #[inline(always)]
    pub(crate) fn set_link(self, mem: &mut MemState, q: TokenList) {
        self.0.set_link(mem, q.0);
    }

    /// The token held by this node.
    #[inline(always)]
    pub(crate) fn token(self, mem: &MemState) -> Token {
        Token(mem.b32s0(self.0))
    }

    #[inline(always)]
    pub(crate) fn set_token(self, mem: &mut MemState, t: Token) {
        mem.set_b32s0(self.0, t.0);
    }

    /// The reference count in the head node, one less than the number of
    /// references.
    #[inline(always)]
    pub(crate) fn ref_count(self, mem: &MemState) -> i32 {
        mem.b32s0(self.0)
    }

    #[inline(always)]
    pub(crate) fn set_ref_count(self, mem: &mut MemState, v: i32) {
        mem.set_b32s0(self.0, v);
    }
}
