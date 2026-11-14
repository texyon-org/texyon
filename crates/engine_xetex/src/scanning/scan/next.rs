//! Getting the next token: `get_next`, `get_token`, `get_x_token` and `pass_text`.

use super::Command;
use super::toks::finish_line;
use crate::base::cmd::Cmd;
use crate::base::codes::{FiOrElseCode, ScannerStatus};
use crate::base::command::{RelaxCode, Usv};
use crate::base::consts::BIGGEST_USV;
use crate::base::mode::Mode;
use crate::base::params::{IntPar, TokPar};
use crate::base::token::{Build, TokCursor, Token, TokenKind, TokenList};
use crate::base::units::BufPos;
use crate::ffi::bridge::output_flush;
use crate::files::io::{FileSlot, input_slot_line};
use crate::memory::mem::MemState;
use crate::memory::node::{AlignRecord, NodePtr};
use crate::print::errors::{Abort, OrConfusion, error, ins_error, runaway};
use crate::print::{CsName, Selector, file_warning, print_char, show_cur_cmd_chr};
use crate::scanning::expand::{expand, macro_call};
use crate::scanning::input::{
    InputClusterState, InputName, MID_LINE, NEW_LINE, SKIP_BLANKS, TOKEN_LIST, TokenType,
    begin_token_list, end_file_reading, end_token_list, pseudo_input,
};
use crate::state::EngineState;
use crate::tables::cs::{CsId, Frozen};

impl MemState {
    /// Whether the macro whose token list starts at `ref_count` is `\protected`.
    #[inline]
    pub(crate) fn is_protected_macro(&self, body: TokenList) -> bool {
        self.tokens.peek(self.tokens.cursor(body)) == Some(Token::PROTECTED)
    }
}

pub(crate) fn get_x_or_protected(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        get_token(st)?;
        if st.scanner.cur.cmd() <= Cmd::MAX_COMMAND {
            return Ok(());
        }
        if st.scanner.cur.cmd() >= Cmd::Call
            && st.scanner.cur.cmd() < Cmd::EndTemplate
            && st
                .scanner
                .cur
                .macro_body()
                .is_some_and(|b| st.mem.is_protected_macro(b))
        {
            return Ok(());
        }
        expand(st)?;
    }
}

/// `get_x_token` until the result is neither a space nor `\relax`.
pub(crate) fn get_x_non_blank_non_relax(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        get_x_token(st)?;
        if st.scanner.cur.cmd() != Cmd::Spacer && st.scanner.cur.cmd() != Cmd::Relax {
            return Ok(());
        }
    }
}

fn check_outer_validity(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.scanner_status == ScannerStatus::Normal {
        return Ok(());
    }
    st.errors.deletions_allowed = false;

    if let Some(cs) = st.scanner.cur_cs {
        let ci = st.input.cur_input;
        if ci.state == TOKEN_LIST
            || !matches!(ci.name, InputName::Read(_) | InputName::ReadTerminal)
        {
            let p = st.mem.tokens.single(Token::from_cs(cs))?;
            begin_token_list(st, p, TokenType::BackedUp)?;
        }
        st.scanner.cur = Command::Spacer(Usv::SPACE);
    }

    let outcome = if st.scanner.scanner_status > ScannerStatus::Skipping {
        runaway(st);
        let what = if st.scanner.cur_cs.is_none() {
            "File ended"
        } else {
            st.scanner.cur_cs = None;
            "Forbidden control sequence found"
        };

        let closer = match st.scanner.scanner_status {
            ScannerStatus::Matching => st.input.par_token,
            _ => Token::right_brace(b'}'),
        };
        let inserted = st.mem.tokens.single(closer)?;
        let scanning = match st.scanner.scanner_status {
            ScannerStatus::Defining => " while scanning definition",
            ScannerStatus::Matching => {
                st.scanner.long_state = Cmd::OuterCall;
                " while scanning use"
            }
            ScannerStatus::Aligning => {
                st.mem
                    .tokens
                    .prepend(inserted, Token::from_cs(Frozen::Cr.cs()))?;
                st.scanner.align_state = -1_000_000;
                " while scanning preamble"
            }
            ScannerStatus::Absorbing => " while scanning text",
            _ => "",
        };

        begin_token_list(st, inserted, TokenType::Inserted)?;
        let macro_name = CsName(st.scanner.warning_index);
        error(
            st,
            (what, scanning, " of ", macro_name),
            &[
                "I suspect you have forgotten a `}', causing me",
                "to read past where you wanted me to stop.",
                "I'll try to recover; but if the error is serious,",
                "you'd better type `E' or `X' now and fix your file.",
            ],
        )
    } else {
        let message = (
            "Incomplete ",
            Command::IfTest(st.scanner.cur_if),
            "; all text was ignored after line ",
            st.scanner.skip_line,
        );
        let cause = if st.scanner.cur_cs.is_some() {
            st.scanner.cur_cs = None;
            "A forbidden control sequence occurred in skipped text."
        } else {
            "The file ended while I was skipping conditional text."
        };
        st.scanner.cur_tok = Token::from_cs(Frozen::Fi.cs());
        ins_error(
            st,
            message,
            &[
                cause,
                r"This kind of error happens when you say `\if...' and forget",
                r"the matching `\fi'. I've inserted a `\fi'; this might work.",
            ],
        )
    };
    st.errors.deletions_allowed = true;
    outcome
}

/// Makes `cs` the current control sequence, with its meaning in `cur`.
#[inline(always)]
pub(super) fn fetch_cs_meaning(st: &mut EngineState, cs: CsId) {
    st.scanner.cur_cs = Some(cs);
    st.scanner.cur = st.eqtb.meaning(cs);
}

#[inline(always)]
fn is_lc_hex(c: i32) -> bool {
    (0x30..=0x39).contains(&c) || (0x61..=0x66).contains(&c)
}

#[inline]
fn hex_digit_value(c: i32) -> i32 {
    if c <= i32::from(b'9') {
        c - i32::from(b'0')
    } else {
        c - i32::from(b'a') + 10
    }
}

impl InputClusterState {
    /// Reduces a `^^` sequence at `buffer[k - 1..]` inside a control-sequence name.
    /// Returns whether the buffer changed, in which case the name must be rescanned.
    #[cold]
    fn reduce_cs_sup_mark(&mut self, k: BufPos, chr: i32) -> bool {
        let limit = self.cur_input.limit;
        let buf = &mut self.buffer;
        let mut sup_count: i32 = 2;
        while sup_count < 6 && k + 2 * sup_count - 2 <= limit && buf[k + sup_count - 1] == chr {
            sup_count += 1;
        }
        let digits = (k + sup_count - 1).until(k + 2 * sup_count - 1);
        let (new_chr, shift) = if digits.clone().all(|i| is_lc_hex(buf[i])) {
            let new_chr = digits.fold(0, |v, i| 16 * v + hex_digit_value(buf[i]));
            if new_chr > BIGGEST_USV {
                return false;
            }
            (new_chr, 2 * sup_count - 1)
        } else {
            let c = buf[k + 1];
            if c >= 128 {
                return false;
            }
            (if c < 64 { c + 64 } else { c - 64 }, 2)
        };
        buf[k - 1] = new_chr;
        self.cur_input.limit -= shift;
        let mut i = k;
        while i <= self.cur_input.limit {
            self.buffer[i] = self.buffer[i + shift];
            i += 1;
        }
        true
    }
}

/// The central tokenizer.
#[inline(always)]
pub(crate) fn get_next(st: &mut EngineState) -> Result<(), Abort> {
    if st.input.cur_input.state == TOKEN_LIST {
        let p = st.input.cur_input.token_loc();
        if let Some(t) = st.mem.tokens.peek(p) {
            let kind = t.kind();
            let plain = match kind {
                TokenKind::Cs(cs) => {
                    let cmd = st.eqtb.meaning(cs).cmd();
                    cmd < Cmd::OuterCall
                        && (!(Cmd::TabMark..=Cmd::CarRet).contains(&cmd)
                            || st.scanner.align_state != 0)
                }
                TokenKind::Char(cmd, _) => {
                    cmd != Cmd::OUT_PARAM && (cmd != Cmd::TabMark || st.scanner.align_state != 0)
                }
            };
            if plain {
                st.scanner.logical_clock = st.scanner.logical_clock.wrapping_add(1);
                st.input.cur_input.set_token_loc(st.mem.tokens.next(p));
                match kind {
                    TokenKind::Cs(cs) => fetch_cs_meaning(st, cs),
                    TokenKind::Char(cmd, chr) => {
                        st.scanner.cur_cs = None;
                        st.scanner.cur = Command::from_char(cmd, chr).or_confusion("token")?;
                        match cmd {
                            Cmd::LeftBrace => st.scanner.align_state += 1,
                            Cmd::RightBrace => st.scanner.align_state -= 1,
                            _ => {}
                        }
                    }
                }
                return Ok(());
            }
        }
    }
    get_next_slow(st)
}

/// `get_next` for everything but a plain token from a token list.
#[inline(never)]
fn get_next_slow(st: &mut EngineState) -> Result<(), Abort> {
    st.scanner.logical_clock = st.scanner.logical_clock.wrapping_add(1);

    'restart: loop {
        st.scanner.cur_cs = None;

        if st.input.cur_input.state != TOKEN_LIST {
            'texswitch: loop {
                if st.input.cur_input.loc <= st.input.cur_input.limit {
                    let loc = st.input.cur_input.loc;
                    let mut chr = st.input.buffer[loc];
                    st.input.cur_input.loc = loc + 1;

                    if (0xD800..0xDC00).contains(&chr)
                        && loc < st.input.cur_input.limit
                        && (0xDC00..0xE000).contains(&st.input.buffer[loc + 1])
                    {
                        let lower = st.input.buffer[loc + 1] - 0xDC00;
                        st.input.cur_input.loc = loc + 2;
                        chr = 65536 + (chr - 0xD800) * 1024 + lower;
                    }

                    'reswitch: loop {
                        let mut cat = st.eqtb.cat_code(chr);
                        let mut char_token = true;

                        // Dispatch on state + category (states 1, 17, 33, categories 0..=15).
                        match i32::from(st.input.cur_input.state) + i32::from(cat) {
                            // Ignored characters, and spaces when skipping blanks.
                            10 | 26 | 42 | 27 | 43 => continue 'texswitch,

                            // Escape: scan a control sequence name.
                            1 | 17 | 33 => {
                                let cs = if st.input.cur_input.loc > st.input.cur_input.limit {
                                    CsId::NULL
                                } else {
                                    'start_cs: loop {
                                        let start = st.input.cur_input.loc;
                                        let first_chr = st.input.buffer[start];
                                        let first_cat = st.eqtb.cat_code(first_chr);
                                        let mut k = start + 1;
                                        st.input.cur_input.state = if first_cat == Cmd::Letter
                                            || first_cat == Cmd::Spacer
                                        {
                                            SKIP_BLANKS
                                        } else {
                                            MID_LINE
                                        };

                                        if first_cat == Cmd::Letter && k <= st.input.cur_input.limit
                                        {
                                            let mut chr;
                                            let mut cat;
                                            loop {
                                                chr = st.input.buffer[k];
                                                cat = st.eqtb.cat_code(chr);
                                                k += 1;
                                                if cat != Cmd::Letter
                                                    || k > st.input.cur_input.limit
                                                {
                                                    break;
                                                }
                                            }
                                            if cat == Cmd::SupMark
                                                && st.input.buffer[k] == chr
                                                && k < st.input.cur_input.limit
                                                && st.input.reduce_cs_sup_mark(k, chr)
                                            {
                                                continue 'start_cs;
                                            }
                                            if cat != Cmd::Letter {
                                                k -= 1;
                                            }
                                            if k > st.input.cur_input.loc + 1 {
                                                let loc = st.input.cur_input.loc;
                                                let cs = st.eqtb.id_lookup(
                                                    &mut st.strings,
                                                    st.input.buffer.span(loc, k),
                                                )?;
                                                st.input.cur_input.loc = k;
                                                break 'start_cs cs;
                                            }
                                        } else if first_cat == Cmd::SupMark
                                            && st.input.buffer[k] == first_chr
                                            && k < st.input.cur_input.limit
                                            && st.input.reduce_cs_sup_mark(k, first_chr)
                                        {
                                            continue 'start_cs;
                                        }

                                        let loc = st.input.cur_input.loc;
                                        let cs = match CsId::of_char(st.input.buffer[loc]) {
                                            Some(cs) => cs,
                                            None => st.eqtb.id_lookup(
                                                &mut st.strings,
                                                st.input.buffer.span(loc, loc + 1),
                                            )?,
                                        };
                                        st.input.cur_input.loc = loc + 1;
                                        break 'start_cs cs;
                                    }
                                };
                                char_token = false;
                                fetch_cs_meaning(st, cs);
                                if st.scanner.cur.cmd() >= Cmd::OuterCall {
                                    check_outer_validity(st)?;
                                }
                            }

                            // Active character.
                            14 | 30 | 46 => {
                                char_token = false;
                                let cs = CsId::of_active(chr).or_confusion("active char")?;
                                fetch_cs_meaning(st, cs);
                                st.input.cur_input.state = MID_LINE;
                                if st.scanner.cur.cmd() >= Cmd::OuterCall {
                                    check_outer_validity(st)?;
                                }
                            }

                            // Superscript: maybe a `^^` notation.
                            8 | 24 | 40 => {
                                let loc = st.input.cur_input.loc;
                                let limit = st.input.cur_input.limit;
                                let buf = &st.input.buffer;
                                if chr == buf[loc] && loc < limit {
                                    let mut sup_count: i32 = 2;
                                    while sup_count < 6
                                        && loc + 2 * sup_count - 2 <= limit
                                        && chr == buf[loc + sup_count - 1]
                                    {
                                        sup_count += 1;
                                    }
                                    let digits =
                                        (loc + sup_count - 1).until(loc + 2 * sup_count - 1);
                                    if digits.clone().all(|i| is_lc_hex(buf[i])) {
                                        let new_chr =
                                            digits.fold(0, |v, i| 16 * v + hex_digit_value(buf[i]));
                                        if new_chr <= BIGGEST_USV {
                                            chr = new_chr;
                                            st.input.cur_input.loc = loc + 2 * sup_count - 1;
                                            continue 'reswitch;
                                        }
                                        chr = buf[loc];
                                    } else {
                                        let c = buf[loc + 1];
                                        if c < 128 {
                                            st.input.cur_input.loc = loc + 2;
                                            chr = if c < 64 { c + 64 } else { c - 64 };
                                            continue 'reswitch;
                                        }
                                    }
                                }
                                st.input.cur_input.state = MID_LINE;
                            }

                            // Invalid character.
                            16 | 32 | 48 => {
                                st.errors.deletions_allowed = false;
                                error(
                                    st,
                                    "Text line contains an invalid character",
                                    &[
                                        "A funny symbol that I can't read has just been input.",
                                        "Continue, and I'll forget that it ever happened.",
                                    ],
                                )?;
                                st.errors.deletions_allowed = true;
                                continue 'restart;
                            }

                            // Space in mid-line.
                            11 => {
                                st.input.cur_input.state = SKIP_BLANKS;
                                chr = i32::from(b' ');
                            }

                            // End of line in mid-line.
                            6 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                cat = Cmd::Spacer;
                                chr = i32::from(b' ');
                            }

                            // Comments, and end of line when skipping blanks.
                            15 | 31 | 47 | 22 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                continue 'texswitch;
                            }

                            // End of an empty line: `\par`.
                            38 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                char_token = false;
                                let par = st.input.par_loc.or_confusion("par_loc")?;
                                fetch_cs_meaning(st, par);
                                if st.scanner.cur.cmd() >= Cmd::OuterCall {
                                    check_outer_validity(st)?;
                                }
                            }

                            2 => st.scanner.align_state += 1,
                            18 | 34 => {
                                st.input.cur_input.state = MID_LINE;
                                st.scanner.align_state += 1;
                            }
                            3 => st.scanner.align_state -= 1,
                            19 | 35 => {
                                st.input.cur_input.state = MID_LINE;
                                st.scanner.align_state -= 1;
                            }

                            // Other delimiters after blanks or at line start.
                            20 | 21 | 23 | 25 | 28 | 29 | 36 | 37 | 39 | 41 | 44 | 45 => {
                                st.input.cur_input.state = MID_LINE;
                            }

                            _ => {}
                        }
                        if char_token {
                            st.scanner.cur =
                                Command::from_char(cat, chr).or_confusion("character category")?;
                        }
                        break 'reswitch;
                    }
                    break 'texswitch;
                }

                st.input.cur_input.state = NEW_LINE;

                if st.input.cur_input.name.reads_lines() {
                    st.mem.line += 1;
                    st.input.first = st.input.cur_input.start;

                    if !st.input.force_eof {
                        let got_line = if let InputName::Pseudo { .. } = st.input.cur_input.name {
                            pseudo_input(st)?
                        } else {
                            let slot = FileSlot::Input(st.input.cur_input.index as usize);
                            input_slot_line(st, slot)? != 0
                        };
                        if got_line {
                            st.input.cur_input.limit = st.input.last;
                        } else {
                            let index = st.input.cur_input.index as usize;
                            let every_eof = st.eqtb.tok_par(TokPar::EveryEof);
                            if !every_eof.is_null() && !st.input.open_levels[index].eof_seen {
                                st.input.cur_input.limit = st.input.first - 1;
                                st.input.open_levels[index].eof_seen = true;
                                begin_token_list(st, every_eof, TokenType::EveryEof)?;
                                continue 'restart;
                            }
                            st.input.force_eof = true;
                        }
                    }

                    if st.input.force_eof {
                        let in_open = st.input.in_open;
                        if st.eqtb.int_par(IntPar::TracingNesting) > 0
                            && (st.input.open_levels[in_open].grp != st.save_stack.cur_boundary
                                || st.input.open_levels[in_open].cond != st.scanner.conds.top())
                        {
                            file_warning(st);
                        }
                        if st.input.cur_input.name.shows_parens() {
                            print_char(st, b')');
                            st.input.open_parens -= 1;
                            output_flush(st.shipout.rust_stdout);
                        }
                        st.input.force_eof = false;
                        end_file_reading(st)?;
                        check_outer_validity(st)?;
                        continue 'restart;
                    }

                    finish_line(st);
                } else {
                    if st.input.cur_input.name != InputName::Terminal {
                        st.scanner.cur = Command::Relax(RelaxCode::Zero);
                        return Ok(());
                    }
                    if !st.input.input_stack.is_empty() {
                        end_file_reading(st)?;
                        continue 'restart;
                    }
                    let coda = st.eqtb.tok_par(TokPar::TectonicCoda);
                    if !st.scanner.used_tectonic_coda_tokens && !coda.is_null() {
                        st.scanner.used_tectonic_coda_tokens = true;
                        begin_token_list(st, coda, TokenType::TectonicCoda)?;
                        continue 'restart;
                    }
                    return Err(Abort::NoLegalEnd {
                        open_log: st.shipout.selector < Selector::LogOnly,
                    });
                }
            }
        } else if let Some(t) = st.mem.tokens.peek(st.input.cur_input.token_loc()) {
            let p = st.input.cur_input.token_loc();
            st.input.cur_input.set_token_loc(st.mem.tokens.next(p));

            match t.kind() {
                TokenKind::Cs(cs) => {
                    fetch_cs_meaning(st, cs);
                    if st.scanner.cur.cmd() >= Cmd::OuterCall {
                        if st.scanner.cur.cmd() == Cmd::DontExpand {
                            let next = st.mem.tokens.peek(st.input.cur_input.token_loc());
                            let cs = next.and_then(Token::cs).or_confusion("dont expand")?;
                            st.input.cur_input.set_token_loc(TokCursor::END);
                            fetch_cs_meaning(st, cs);
                            if st.scanner.cur.cmd() > Cmd::MAX_COMMAND {
                                st.scanner.cur = Command::Relax(RelaxCode::NoExpand);
                            }
                        } else {
                            if st.scanner.cur_cs == Some(Frozen::EndWrite.cs())
                                && matches!(st.input.cur_list.mode, Mode::None)
                            {
                                return Err(Abort::Fatal("Unbalanced write command"));
                            }
                            check_outer_validity(st)?;
                        }
                    }
                }
                TokenKind::Char(cmd, chr) => {
                    if cmd == Cmd::OUT_PARAM {
                        let k = st.input.cur_input.param_start() + chr as usize - 1;
                        let param = st.input.param_stack[k].list;
                        begin_token_list(st, param, TokenType::Parameter)?;
                        continue 'restart;
                    }
                    st.scanner.cur = Command::from_char(cmd, chr).or_confusion("token")?;
                    match cmd {
                        Cmd::LeftBrace => st.scanner.align_state += 1,
                        Cmd::RightBrace => st.scanner.align_state -= 1,
                        _ => {}
                    }
                }
            }
        } else {
            end_token_list(st)?;
            continue 'restart;
        }

        // An alignment entry ends at `&` or `\cr` when braces are balanced.
        if st.scanner.cur.cmd() <= Cmd::CarRet
            && st.scanner.cur.cmd() >= Cmd::TabMark
            && st.scanner.align_state == 0
        {
            if st.scanner.scanner_status == ScannerStatus::Aligning
                || st.scanner.cur_align.is_none()
            {
                return Err(Abort::Fatal(
                    "(interwoven alignment preambles are not allowed)",
                ));
            }
            let column =
                NodePtr::<AlignRecord>::from_ptr(st.scanner.cur_align).or_confusion("align")?;
            let saved_cmd = st.mem[column].extra_info;
            st.mem[column].extra_info = st.scanner.cur.encode().1;
            let template = if Cmd::from_stored(saved_cmd) == Cmd::Omit {
                TokenList::OMIT_TEMPLATE
            } else {
                st.mem[column].v_part
            };
            begin_token_list(st, template, TokenType::VTemplate)?;
            st.scanner.align_state = 1_000_000;
            continue 'restart;
        }
        break 'restart;
    }
    Ok(())
}

/// `get_x_token` until the result is not a space.
#[inline]
pub(crate) fn get_x_non_blank(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        get_x_token(st)?;
        if st.scanner.cur.cmd() != Cmd::Spacer {
            return Ok(());
        }
    }
}

#[inline(always)]
fn pack_cur_tok(st: &mut EngineState) {
    st.scanner.cur_tok = match st.scanner.cur_cs {
        None => st.scanner.cur.char_token(),
        Some(cs) => Token::from_cs(cs),
    };
}

pub(crate) fn x_token(st: &mut EngineState) -> Result<(), Abort> {
    while st.scanner.cur.cmd() > Cmd::MAX_COMMAND {
        expand(st)?;
        get_next(st)?;
    }
    pack_cur_tok(st);
    Ok(())
}

pub(crate) fn get_x_token(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        if st.cancellation.at_expansion_token() {
            return Err(Abort::Cancelled);
        }
        get_next(st)?;
        if st.scanner.cur.cmd() <= Cmd::MAX_COMMAND {
            break;
        }
        if st.scanner.cur.cmd() >= Cmd::Call {
            if st.scanner.cur.cmd() < Cmd::EndTemplate {
                macro_call(st)?;
            } else {
                st.scanner.cur_cs = Some(Frozen::Endv.cs());
                st.scanner.cur = Command::Endv;
                break;
            }
        } else {
            expand(st)?;
        }
    }
    pack_cur_tok(st);
    Ok(())
}

pub(crate) fn pass_text(st: &mut EngineState) -> Result<(), Abort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = ScannerStatus::Skipping;
    let mut l: i32 = 0;
    st.scanner.skip_line = st.mem.line;

    loop {
        get_next(st)?;
        if st.scanner.cur.cmd() == Cmd::FiOrElse {
            if l == 0 {
                break;
            }
            if st.scanner.fi_or_else() == Some(FiOrElseCode::Fi) {
                l -= 1;
            }
        } else if st.scanner.cur.cmd() == Cmd::IfTest {
            l += 1;
        }
    }
    st.scanner.scanner_status = save_scanner_status;
    if st.eqtb.int_par(IntPar::TracingIfs) > 0 {
        show_cur_cmd_chr(st);
    }
    Ok(())
}
pub(crate) fn get_token(st: &mut EngineState) -> Result<(), Abort> {
    st.eqtb.no_new_control_sequence = false;
    get_next(st)?;
    st.eqtb.no_new_control_sequence = true;
    pack_cur_tok(st);
    Ok(())
}

/// Whether `get_next` would deliver token-list token `t` with no side effect
/// beyond brace counting, and `t` is not a parameter character if `stop_at_mac_param`.
#[inline(always)]
fn plain_list_token(st: &EngineState, t: Token, stop_at_mac_param: bool) -> bool {
    let cmd = match t.kind() {
        TokenKind::Cs(cs) => {
            let cmd = st.eqtb.meaning(cs).cmd();
            if cmd >= Cmd::OuterCall
                || t == st.input.par_token && st.scanner.long_state != Cmd::LongCall
            {
                return false;
            }
            cmd
        }
        TokenKind::Char(Cmd::OUT_PARAM, _) => return false,
        TokenKind::Char(cmd, _) => cmd,
    };
    !((cmd == Cmd::TabMark || cmd == Cmd::CarRet) && st.scanner.align_state == 0
        || stop_at_mac_param && cmd == Cmd::MacParam)
}

/// Advances past the token at `q` as `get_next` would and returns the next position.
#[inline(always)]
fn consume_list_token(st: &mut EngineState, q: TokCursor) -> TokCursor {
    st.scanner.logical_clock = st.scanner.logical_clock.wrapping_add(1);
    let next = st.mem.tokens.next(q);
    st.input.cur_input.set_token_loc(next);
    next
}

/// `get_token` and `store_new_token` for the rest of a group, straight from the
/// current token list. Stops before a token that needs `get_next` (or a
/// parameter character if `stop_at_mac_param`), or returns true once the group
/// closes, with the closing brace consumed as `cur_tok`.
#[inline(always)]
pub(crate) fn copy_group_tokens(
    st: &mut EngineState,
    p: &Build,
    unbalance: &mut i32,
    stop_at_mac_param: bool,
) -> Result<bool, Abort> {
    if st.input.cur_input.state != TOKEN_LIST {
        return Ok(false);
    }
    let mut q = st.input.cur_input.token_loc();
    while let Some(t) = st.mem.tokens.peek(q) {
        if t.is_brace() {
            if t.is_left_brace() {
                st.scanner.align_state += 1;
                *unbalance += 1;
            } else {
                st.scanner.align_state -= 1;
                *unbalance -= 1;
                if *unbalance == 0 {
                    consume_list_token(st, q);
                    st.scanner.cur_cs = None;
                    st.scanner.cur = t
                        .char()
                        .and_then(|(cmd, chr)| Command::from_char(cmd, chr))
                        .or_confusion("token")?;
                    st.scanner.cur_tok = t;
                    return Ok(true);
                }
            }
        } else if !plain_list_token(st, t, stop_at_mac_param) {
            break;
        }
        st.mem.tokens.push(p, t)?;
        q = consume_list_token(st, q);
    }
    Ok(false)
}

/// `get_token` and `store_new_token` for delimited-argument tokens, straight
/// from the current token list, up to a brace, `delim`, or a token that needs
/// `get_next`. Counts the copied tokens in `m`.
#[inline]
pub(crate) fn copy_delimited_tokens(
    st: &mut EngineState,
    p: &Build,
    m: &mut i32,
    delim: Token,
) -> Result<(), Abort> {
    if st.input.cur_input.state != TOKEN_LIST {
        return Ok(());
    }
    let mut q = st.input.cur_input.token_loc();
    while let Some(t) = st.mem.tokens.peek(q) {
        if t == delim || t.is_brace() || !plain_list_token(st, t, false) {
            break;
        }
        st.mem.tokens.push(p, t)?;
        *m += 1;
        q = consume_list_token(st, q);
    }
    Ok(())
}
