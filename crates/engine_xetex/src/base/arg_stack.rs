//! Macro arguments, kept on a stack apart from the token lists: an argument
//! lives exactly as long as its macro's level, so they die in reverse order.

use std::mem;

use crate::base::token::Token;
use crate::print::errors::Abort;

/// A macro argument: where its tokens start on the argument stack, and how many.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ArgRef {
    start: u32,
    len: u32,
}

impl ArgRef {
    pub(super) fn new(start: u32, len: u32) -> Self {
        Self { start, len }
    }

    pub(crate) fn start(self) -> u32 {
        self.start
    }

    pub(crate) fn end(self) -> u32 {
        self.start + self.len
    }
}

/// The arguments of every open macro level, oldest first.
#[derive(Default)]
pub(crate) struct ArgStack {
    tokens: Vec<Token>,
    journal: ArgJournal,
}

impl ArgStack {
    pub(crate) fn len(&self) -> usize {
        self.tokens.len()
    }

    #[inline(always)]
    pub(crate) fn get(&self, i: usize) -> Option<Token> {
        self.tokens.get(i).copied()
    }

    pub(crate) fn live(&self) -> &[Token] {
        &self.tokens
    }

    /// Pushes `tokens`, which hold `args` at offsets from their start, and moves `args` along.
    /// Tokens before the first argument stay behind, so popping to it frees everything.
    pub(crate) fn push_all(&mut self, tokens: &[Token], args: &mut [ArgRef]) -> Result<(), Abort> {
        let Some(skip) = args.first().map(|arg| arg.start) else {
            return Ok(());
        };
        let len = u32::try_from(self.tokens.len()).map_err(|_| Abort::AvailOverflow)?;
        self.tokens.extend_from_slice(&tokens[skip as usize..]);
        for arg in args {
            arg.start = arg.start - skip + len;
        }
        Ok(())
    }

    #[cfg(test)]
    fn push(&mut self, tokens: &[Token]) -> Result<ArgRef, Abort> {
        let mut arg = [ArgRef::new(
            0,
            u32::try_from(tokens.len()).map_err(|_| Abort::AvailOverflow)?,
        )];
        self.push_all(tokens, &mut arg)?;
        Ok(arg[0])
    }

    /// Drops `arg` and every argument pushed after it.
    pub(crate) fn pop_to(&mut self, arg: ArgRef) {
        let to = arg.start as usize;
        if self.journal.armed {
            self.journal.note_truncate(&self.tokens, to);
        }
        self.tokens.truncate(to);
    }

    pub(crate) fn arm_journal(&mut self) {
        self.journal.arm(self.tokens.len());
    }

    pub(crate) fn disarm_journal(&mut self) {
        self.journal = ArgJournal::default();
    }

    pub(crate) fn seal_journal(&mut self) -> ArgDelta {
        let delta = ArgDelta {
            len: self.journal.len,
            low: self.journal.low,
            saved: mem::take(&mut self.journal.saved),
        };
        self.journal.arm(self.tokens.len());
        delta
    }

    pub(crate) fn snapshot_journal(&self) -> ArgDelta {
        ArgDelta {
            len: self.journal.len,
            low: self.journal.low,
            saved: self.journal.saved.clone(),
        }
    }

    pub(crate) fn apply(&mut self, delta: &ArgDelta) {
        self.tokens.resize(delta.len, Token::ZERO);
        let saved = &mut self.tokens[delta.low..delta.len];
        for (slot, &t) in saved.iter_mut().rev().zip(&delta.saved) {
            *slot = t;
        }
    }

    /// Sets the length a checkpoint recorded, after its deltas restored the tokens.
    pub(crate) fn restore_len(&mut self, len: usize) {
        self.tokens.resize(len, Token::ZERO);
        if self.journal.armed {
            self.journal.arm(len);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.tokens.clear();
    }
}

/// What the stack held at its arm-time length, from the lowest length since.
/// Pushes overwrite only below that low mark, so truncation is the one place to save.
#[derive(Default)]
struct ArgJournal {
    armed: bool,
    len: usize,
    low: usize,
    /// The tokens from `low` to `len`, last first, so a lower truncation appends.
    saved: Vec<Token>,
}

impl ArgJournal {
    fn arm(&mut self, len: usize) {
        self.armed = true;
        self.len = len;
        self.low = len;
        self.saved.clear();
    }

    fn note_truncate(&mut self, tokens: &[Token], to: usize) {
        if to < self.low {
            self.saved.extend(tokens[to..self.low].iter().rev());
            self.low = to;
        }
    }
}

/// Undoes one epoch of the argument stack: the tokens from `low` to `len`, last first.
#[derive(Clone, Default)]
pub(crate) struct ArgDelta {
    len: usize,
    low: usize,
    saved: Vec<Token>,
}

impl ArgDelta {
    /// `older` restores further back, so it wins where both saved a token.
    /// The newer delta only adds what lies below `older`'s low mark, which
    /// `older`'s epoch never touched.
    pub(crate) fn compose_with_older(&mut self, mut older: ArgDelta) {
        if self.low < older.low {
            let below = older.low - self.low;
            older
                .saved
                .extend_from_slice(&self.saved[self.saved.len() - below..]);
            older.low = self.low;
        }
        *self = older;
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.saved.capacity() * size_of::<Token>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(v: &[i32]) -> Vec<Token> {
        v.iter().map(|&x| Token(x)).collect()
    }

    fn values(s: &ArgStack) -> Vec<i32> {
        s.live().iter().map(|t| t.0).collect()
    }

    #[test]
    fn an_epoch_that_pops_below_its_start_and_refills_rewinds() {
        let mut s = ArgStack::default();
        s.push(&toks(&[1, 2])).unwrap();
        let b = s.push(&toks(&[3, 4, 5])).unwrap();
        s.arm_journal();
        s.pop_to(b);
        s.push(&toks(&[9])).unwrap();
        s.pop_to(ArgRef { start: 1, len: 0 });
        s.push(&toks(&[7, 7, 7, 7, 7])).unwrap();
        let delta = s.seal_journal();
        s.apply(&delta);
        assert_eq!(values(&s), [1, 2, 3, 4, 5]);
    }

    #[test]
    fn composed_deltas_rewind_two_epochs() {
        let mut s = ArgStack::default();
        s.push(&toks(&[1, 2, 3])).unwrap();
        s.arm_journal();
        s.pop_to(ArgRef { start: 2, len: 1 });
        s.push(&toks(&[8, 8])).unwrap();
        let older = s.seal_journal();
        s.pop_to(ArgRef { start: 0, len: 1 });
        s.push(&toks(&[6])).unwrap();
        let mut newer = s.seal_journal();
        newer.compose_with_older(older);
        s.apply(&newer);
        assert_eq!(values(&s), [1, 2, 3]);
    }

    #[test]
    fn pushes_above_the_arm_length_need_no_saving() {
        let mut s = ArgStack::default();
        s.push(&toks(&[1])).unwrap();
        s.arm_journal();
        let a = s.push(&toks(&[2, 3])).unwrap();
        s.pop_to(a);
        let delta = s.seal_journal();
        assert_eq!(delta.allocated_bytes(), 0);
        s.push(&toks(&[4])).unwrap();
        s.apply(&delta);
        assert_eq!(values(&s), [1]);
    }
}
