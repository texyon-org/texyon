//! TeX tokens: `cmd * max_char_val + chr` for a character, `cs_token_flag + p`
//! for a control sequence.

use crate::cmd::{
    END_MATCH, LEFT_BRACE, LETTER, MATCH, MATH_SHIFT, OTHER_CHAR, OUT_PARAM, RIGHT_BRACE, SPACER,
    TAB_MARK,
};
use crate::consts::CS_TOKEN_FLAG;

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
    pub(crate) const SPACE: Token = Token::new(SPACER, b' ' as i32);
    /// Marks a `\protected` macro, right after its reference count.
    pub(crate) const PROTECTED: Token = Token(Token::END_MATCH.0 + 1);
    pub(crate) const END_MATCH: Token = Token::new(END_MATCH, 0);

    /// The character token of category `cmd`.
    #[inline(always)]
    pub(crate) const fn new(cmd: u8, chr: i32) -> Token {
        Token(cmd as i32 * MAX_CHAR_VAL + chr)
    }

    /// The token of control sequence `p`.
    #[inline(always)]
    pub(crate) const fn from_cs(p: i32) -> Token {
        Token(CS_TOKEN_FLAG + p)
    }

    #[inline(always)]
    pub(crate) const fn left_brace(c: u8) -> Token {
        Token::new(LEFT_BRACE, c as i32)
    }

    #[inline(always)]
    pub(crate) const fn right_brace(c: u8) -> Token {
        Token::new(RIGHT_BRACE, c as i32)
    }

    #[inline(always)]
    pub(crate) const fn math_shift(c: u8) -> Token {
        Token::new(MATH_SHIFT, c as i32)
    }

    #[inline(always)]
    pub(crate) const fn tab(c: u8) -> Token {
        Token::new(TAB_MARK, c as i32)
    }

    #[inline(always)]
    pub(crate) const fn letter(c: u8) -> Token {
        Token::new(LETTER, c as i32)
    }

    #[inline(always)]
    pub(crate) const fn other(c: u8) -> Token {
        Token::new(OTHER_CHAR, c as i32)
    }

    /// The `#n` parameter token in a macro body.
    #[inline(always)]
    pub(crate) const fn out_param(n: i32) -> Token {
        Token::new(OUT_PARAM, n)
    }

    /// The `#` of a parameter in a macro's parameter text, with its catcode 6 character.
    #[inline(always)]
    pub(crate) const fn match_(c: i32) -> Token {
        Token::new(MATCH, c)
    }

    #[inline(always)]
    pub(crate) const fn is_cs(self) -> bool {
        self.0 >= CS_TOKEN_FLAG
    }

    /// The control sequence of a [`Token::is_cs`] token.
    #[inline(always)]
    pub(crate) const fn cs(self) -> i32 {
        self.0 - CS_TOKEN_FLAG
    }

    /// The category of a character token.
    #[inline(always)]
    pub(crate) const fn cmd(self) -> u8 {
        (self.0 / MAX_CHAR_VAL) as u8
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
        self.0 >= Token::new(MATCH, 0).0 && self.0 <= Token::END_MATCH.0
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

const LEFT_BRACE_LIMIT: i32 = Token::new(RIGHT_BRACE, 0).0;
const RIGHT_BRACE_LIMIT: i32 = Token::new(MATH_SHIFT, 0).0;
