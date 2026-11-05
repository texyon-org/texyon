//! Getting the next token: `get_next`, `get_token`, `get_x_token` and `pass_text`.

use crate::bridge::output_flush;
use crate::cmd::Cmd;
use crate::codes::{FiOrElseCode, ScannerStatus};
use crate::consts::{
    BIGGEST_USV, FROZEN_CR, FROZEN_ENDV, FROZEN_FI, NULL_CS, SINGLE_BASE, TEX_NULL,
};
use crate::errors::{error, Abort};
use crate::expand::{expand, macro_call};
use crate::input::{
    begin_token_list, ins_error, TokenType, MID_LINE, NEW_LINE, SKIP_BLANKS, TOKEN_LIST,
};
use crate::mem::OMIT_TEMPLATE;
use crate::node::{AlignRecord, NodePtr};
use crate::output::{
    capture_to_diagnostic, error_here_with_diagnostic, print, print_cmd_chr, print_err, print_int,
};
use crate::params::{IntPar, TokPar};
use crate::state::{EngineState, InputClusterState, MemState, Selector};
use crate::token::Token;
use crate::units::MemPtr;

use super::toks::finish_line;
use super::NO_EXPAND_FLAG;

impl MemState {
    /// Whether the macro whose token list starts at `ref_count` is `\protected`.
    #[inline]
    pub(crate) fn is_protected_macro(&self, ref_count: MemPtr) -> bool {
        ref_count.link(self).token(self) == Token::PROTECTED
    }
}

pub(crate) fn get_x_or_protected(st: &mut EngineState) -> Result<(), Abort> {
    loop {
        get_token(st)?;
        if st.scanner.cur_cmd <= Cmd::MAX_COMMAND {
            return Ok(());
        }
        if st.scanner.cur_cmd >= Cmd::Call
            && st.scanner.cur_cmd < Cmd::EndTemplate
            && st.mem.is_protected_macro(st.scanner.cur_chr_ptr())
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
        if st.scanner.cur_cmd != Cmd::Spacer && st.scanner.cur_cmd != Cmd::Relax {
            return Ok(());
        }
    }
}

fn check_outer_validity(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.scanner_status == ScannerStatus::Normal {
        return Ok(());
    }
    st.errors.deletions_allowed = false;

    if st.scanner.cur_cs != 0 {
        let ci = st.input.cur_input;
        if ci.state == TOKEN_LIST || ci.name < 1 || ci.name > 17 {
            let p = st.mem.get_avail()?;
            p.set_token(&mut st.mem, Token::from_cs(st.scanner.cur_cs));
            begin_token_list(st, p, TokenType::BackedUp)?;
        }
        st.scanner.cur_cmd = Cmd::Spacer;
        st.scanner.cur_chr = i32::from(b' ');
    }

    let outcome = if st.scanner.scanner_status > ScannerStatus::Skipping {
        crate::errors::runaway(st);
        if st.scanner.cur_cs == 0 {
            error_here_with_diagnostic(st, "File ended");
        } else {
            st.scanner.cur_cs = 0;
            error_here_with_diagnostic(st, "Forbidden control sequence found");
        }

        let p = st.mem.get_avail()?;
        let mut inserted = p;
        match st.scanner.scanner_status {
            ScannerStatus::Defining => {
                print(st, " while scanning definition");
                p.set_token(&mut st.mem, Token::right_brace(b'}'));
            }
            ScannerStatus::Matching => {
                print(st, " while scanning use");
                p.set_token(&mut st.mem, st.input.par_token);
                st.scanner.long_state = Cmd::OuterCall;
            }
            ScannerStatus::Aligning => {
                print(st, " while scanning preamble");
                p.set_token(&mut st.mem, Token::right_brace(b'}'));
                inserted = st.mem.get_avail()?;
                inserted.set_link(&mut st.mem, p);
                inserted.set_token(&mut st.mem, Token::from_cs(FROZEN_CR));
                st.scanner.align_state = -1_000_000;
            }
            ScannerStatus::Absorbing => {
                print(st, " while scanning text");
                p.set_token(&mut st.mem, Token::right_brace(b'}'));
            }
            _ => {}
        }

        begin_token_list(st, inserted, TokenType::Inserted)?;
        print(st, " of ");
        let warning_index = st.scanner.warning_index;
        crate::output::sprint_cs(st, warning_index);
        capture_to_diagnostic(st, None);
        st.errors.help(&[
            "I suspect you have forgotten a `}', causing me",
            "to read past where you wanted me to stop.",
            "I'll try to recover; but if the error is serious,",
            "you'd better type `E' or `X' now and fix your file.",
        ]);
        error(st)
    } else {
        error_here_with_diagnostic(st, "Incomplete ");
        let cur_if = st.scanner.cur_if;
        print_cmd_chr(st, Cmd::IfTest, i32::from(cur_if));
        print(st, "; all text was ignored after line ");
        let skip_line = st.scanner.skip_line;
        print_int(st, skip_line);
        capture_to_diagnostic(st, None);
        let cause = if st.scanner.cur_cs != 0 {
            st.scanner.cur_cs = 0;
            "A forbidden control sequence occurred in skipped text."
        } else {
            "The file ended while I was skipping conditional text."
        };
        st.errors.help(&[
            cause,
            r"This kind of error happens when you say `\if...' and forget",
            r"the matching `\fi'. I've inserted a `\fi'; this might work.",
        ]);
        st.scanner.cur_tok = Token::from_cs(FROZEN_FI);
        ins_error(st)
    };
    st.errors.deletions_allowed = true;
    outcome
}

/// Sets `cur_cmd` and `cur_chr` to the meaning of `cur_cs`.
#[inline(always)]
pub(super) fn fetch_cs_meaning(st: &mut EngineState) {
    st.scanner.cur_cmd = st.eqtb.eq_type(st.scanner.cur_cs);
    st.scanner.cur_chr = st.eqtb.equiv(st.scanner.cur_cs);
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
    fn reduce_cs_sup_mark(&mut self, k: i32, chr: i32) -> bool {
        let limit = self.cur_input.limit;
        let buf = &mut self.buffer;
        let mut sup_count: i32 = 2;
        while sup_count < 6
            && k + 2 * sup_count - 2 <= limit
            && buf[(k + sup_count - 1) as usize] == chr
        {
            sup_count += 1;
        }
        let digits = (k + sup_count - 1)..(k + 2 * sup_count - 1);
        let (new_chr, shift) = if digits.clone().all(|i| is_lc_hex(buf[i as usize])) {
            let new_chr = digits.fold(0, |v, i| 16 * v + hex_digit_value(buf[i as usize]));
            if new_chr > BIGGEST_USV {
                return false;
            }
            (new_chr, 2 * sup_count - 1)
        } else {
            let c = buf[(k + 1) as usize];
            if c >= 128 {
                return false;
            }
            (if c < 64 { c + 64 } else { c - 64 }, 2)
        };
        buf[(k - 1) as usize] = new_chr;
        self.cur_input.limit -= shift;
        let mut i = k;
        while i <= self.cur_input.limit {
            self.buffer[i as usize] = self.buffer[(i + shift) as usize];
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
        if !p.is_null() {
            let t = p.token(&st.mem);
            let plain = if t.is_cs() {
                let cmd = st.eqtb.eq_type(t.cs());
                cmd < Cmd::OuterCall
                    && (!(Cmd::TabMark..=Cmd::CarRet).contains(&cmd) || st.scanner.align_state != 0)
            } else {
                let cmd = t.cmd();
                cmd != Cmd::OUT_PARAM && (cmd != Cmd::TabMark || st.scanner.align_state != 0)
            };
            if plain {
                st.scanner.logical_clock = st.scanner.logical_clock.wrapping_add(1);
                st.input.cur_input.set_token_loc(p.link(&st.mem));
                if t.is_cs() {
                    st.scanner.cur_cs = t.cs();
                    fetch_cs_meaning(st);
                } else {
                    st.scanner.cur_cs = 0;
                    st.scanner.cur_cmd = t.cmd();
                    st.scanner.cur_chr = t.chr();
                    match st.scanner.cur_cmd {
                        Cmd::LeftBrace => st.scanner.align_state += 1,
                        Cmd::RightBrace => st.scanner.align_state -= 1,
                        _ => {}
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
        st.scanner.cur_cs = 0;

        if st.input.cur_input.state != TOKEN_LIST {
            'texswitch: loop {
                if st.input.cur_input.loc <= st.input.cur_input.limit {
                    let loc = st.input.cur_input.loc;
                    st.scanner.cur_chr = st.input.buffer[loc as usize];
                    st.input.cur_input.loc = loc + 1;

                    if (0xD800..0xDC00).contains(&st.scanner.cur_chr)
                        && loc < st.input.cur_input.limit
                        && (0xDC00..0xE000).contains(&st.input.buffer[(loc + 1) as usize])
                    {
                        let lower = st.input.buffer[(loc + 1) as usize] - 0xDC00;
                        st.input.cur_input.loc = loc + 2;
                        st.scanner.cur_chr = 65536 + (st.scanner.cur_chr - 0xD800) * 1024 + lower;
                    }

                    'reswitch: loop {
                        let cat = st.eqtb.cat_code(st.scanner.cur_chr);
                        st.scanner.cur_cmd = cat;

                        // Dispatch on state + category (states 1, 17, 33, categories 0..=15).
                        match i32::from(st.input.cur_input.state) + i32::from(cat) {
                            // Ignored characters, and spaces when skipping blanks.
                            10 | 26 | 42 | 27 | 43 => continue 'texswitch,

                            // Escape: scan a control sequence name.
                            1 | 17 | 33 => {
                                if st.input.cur_input.loc > st.input.cur_input.limit {
                                    st.scanner.cur_cs = NULL_CS;
                                } else {
                                    'start_cs: loop {
                                        let start = st.input.cur_input.loc;
                                        let first_chr = st.input.buffer[start as usize];
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
                                                chr = st.input.buffer[k as usize];
                                                cat = st.eqtb.cat_code(chr);
                                                k += 1;
                                                if cat != Cmd::Letter
                                                    || k > st.input.cur_input.limit
                                                {
                                                    break;
                                                }
                                            }
                                            if cat == Cmd::SupMark
                                                && st.input.buffer[k as usize] == chr
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
                                                st.scanner.cur_cs = st.eqtb.id_lookup(
                                                    &mut st.strings,
                                                    &st.input.buffer,
                                                    loc,
                                                    k - loc,
                                                )?;
                                                st.input.cur_input.loc = k;
                                                break 'start_cs;
                                            }
                                        } else if first_cat == Cmd::SupMark
                                            && st.input.buffer[k as usize] == first_chr
                                            && k < st.input.cur_input.limit
                                            && st.input.reduce_cs_sup_mark(k, first_chr)
                                        {
                                            continue 'start_cs;
                                        }

                                        let loc = st.input.cur_input.loc;
                                        if st.input.buffer[loc as usize] > 65535 {
                                            st.scanner.cur_cs = st.eqtb.id_lookup(
                                                &mut st.strings,
                                                &st.input.buffer,
                                                loc,
                                                1,
                                            )?;
                                        } else {
                                            st.scanner.cur_cs =
                                                SINGLE_BASE + st.input.buffer[loc as usize];
                                        }
                                        st.input.cur_input.loc = loc + 1;
                                        break 'start_cs;
                                    }
                                }
                                fetch_cs_meaning(st);
                                if st.scanner.cur_cmd >= Cmd::OuterCall {
                                    check_outer_validity(st)?;
                                }
                            }

                            // Active character.
                            14 | 30 | 46 => {
                                st.scanner.cur_cs = st.scanner.cur_chr + 1;
                                fetch_cs_meaning(st);
                                st.input.cur_input.state = MID_LINE;
                                if st.scanner.cur_cmd >= Cmd::OuterCall {
                                    check_outer_validity(st)?;
                                }
                            }

                            // Superscript: maybe a `^^` notation.
                            8 | 24 | 40 => {
                                let loc = st.input.cur_input.loc;
                                let limit = st.input.cur_input.limit;
                                let buf = &st.input.buffer;
                                if st.scanner.cur_chr == buf[loc as usize] && loc < limit {
                                    let mut sup_count: i32 = 2;
                                    while sup_count < 6
                                        && loc + 2 * sup_count - 2 <= limit
                                        && st.scanner.cur_chr == buf[(loc + sup_count - 1) as usize]
                                    {
                                        sup_count += 1;
                                    }
                                    let digits = (loc + sup_count - 1)..(loc + 2 * sup_count - 1);
                                    if digits.clone().all(|i| is_lc_hex(buf[i as usize])) {
                                        let new_chr = digits.fold(0, |v, i| {
                                            16 * v + hex_digit_value(buf[i as usize])
                                        });
                                        if new_chr <= BIGGEST_USV {
                                            st.scanner.cur_chr = new_chr;
                                            st.input.cur_input.loc = loc + 2 * sup_count - 1;
                                            continue 'reswitch;
                                        }
                                        st.scanner.cur_chr = buf[loc as usize];
                                    } else {
                                        let c = buf[(loc + 1) as usize];
                                        if c < 128 {
                                            st.input.cur_input.loc = loc + 2;
                                            st.scanner.cur_chr =
                                                if c < 64 { c + 64 } else { c - 64 };
                                            continue 'reswitch;
                                        }
                                    }
                                }
                                st.input.cur_input.state = MID_LINE;
                            }

                            // Invalid character.
                            16 | 32 | 48 => {
                                print_err(st, "Text line contains an invalid character");
                                st.errors.help(&[
                                    "A funny symbol that I can't read has just been input.",
                                    "Continue, and I'll forget that it ever happened.",
                                ]);
                                st.errors.deletions_allowed = false;
                                error(st)?;
                                st.errors.deletions_allowed = true;
                                continue 'restart;
                            }

                            // Space in mid-line.
                            11 => {
                                st.input.cur_input.state = SKIP_BLANKS;
                                st.scanner.cur_chr = i32::from(b' ');
                            }

                            // End of line in mid-line.
                            6 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                st.scanner.cur_cmd = Cmd::Spacer;
                                st.scanner.cur_chr = i32::from(b' ');
                            }

                            // Comments, and end of line when skipping blanks.
                            15 | 31 | 47 | 22 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                continue 'texswitch;
                            }

                            // End of an empty line: `\par`.
                            38 => {
                                st.input.cur_input.loc = st.input.cur_input.limit + 1;
                                st.scanner.cur_cs = st.input.par_loc;
                                fetch_cs_meaning(st);
                                if st.scanner.cur_cmd >= Cmd::OuterCall {
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
                        break 'reswitch;
                    }
                    break 'texswitch;
                }

                st.input.cur_input.state = NEW_LINE;

                if st.input.cur_input.name > 17 {
                    st.mem.line += 1;
                    st.input.first = st.input.cur_input.start;

                    if !st.input.force_eof {
                        let got_line = if st.input.cur_input.name <= 19 {
                            crate::input::pseudo_input(st)?
                        } else {
                            let slot =
                                crate::io::FileSlot::Input(st.input.cur_input.index as usize);
                            crate::io::input_slot_line(st, slot)? != 0
                        };
                        if got_line {
                            st.input.cur_input.limit = st.input.last;
                        } else {
                            let index = st.input.cur_input.index as usize;
                            let every_eof = st.eqtb.tok_par(TokPar::EveryEof);
                            if !every_eof.is_null() && !st.input.eof_seen[index] {
                                st.input.cur_input.limit = st.input.first - 1;
                                st.input.eof_seen[index] = true;
                                begin_token_list(st, every_eof, TokenType::EveryEof)?;
                                continue 'restart;
                            }
                            st.input.force_eof = true;
                        }
                    }

                    if st.input.force_eof {
                        let in_open = st.input.in_open as usize;
                        if st.eqtb.int_par(IntPar::TracingNesting) > 0
                            && (st.input.grp_stack[in_open] != st.save_stack.cur_boundary
                                || st.input.if_stack[in_open] != st.scanner.cond_ptr)
                        {
                            crate::output::file_warning(st);
                        }
                        if st.input.cur_input.name >= 19 {
                            crate::output::print_char(st, b')');
                            st.input.open_parens -= 1;
                            output_flush(st.shipout.rust_stdout);
                        }
                        st.input.force_eof = false;
                        crate::input::end_file_reading(st)?;
                        check_outer_validity(st)?;
                        continue 'restart;
                    }

                    finish_line(st);
                } else {
                    if st.input.cur_input.name != 0 {
                        st.scanner.cur_cmd = Cmd::Relax;
                        st.scanner.cur_chr = 0;
                        return Ok(());
                    }
                    if st.input.input_ptr > 0 {
                        crate::input::end_file_reading(st)?;
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
        } else if !st.input.cur_input.token_loc().is_null() {
            let p = st.input.cur_input.token_loc();
            let t = p.token(&st.mem);
            st.input.cur_input.set_token_loc(p.link(&st.mem));

            if t.is_cs() {
                st.scanner.cur_cs = t.cs();
                fetch_cs_meaning(st);
                if st.scanner.cur_cmd >= Cmd::OuterCall {
                    if st.scanner.cur_cmd == Cmd::DontExpand {
                        st.scanner.cur_cs = st.input.cur_input.token_loc().token(&st.mem).cs();
                        st.input.cur_input.set_token_loc(MemPtr::NULL);
                        fetch_cs_meaning(st);
                        if st.scanner.cur_cmd > Cmd::MAX_COMMAND {
                            st.scanner.cur_cmd = Cmd::Relax;
                            st.scanner.cur_chr = NO_EXPAND_FLAG;
                        }
                    } else {
                        check_outer_validity(st)?;
                    }
                }
            } else {
                st.scanner.cur_cmd = t.cmd();
                st.scanner.cur_chr = t.chr();
                match st.scanner.cur_cmd {
                    Cmd::LeftBrace => st.scanner.align_state += 1,
                    Cmd::RightBrace => st.scanner.align_state -= 1,
                    Cmd::OUT_PARAM => {
                        let param = st.input.param_stack
                            [(st.input.cur_input.limit + st.scanner.cur_chr - 1) as usize];
                        begin_token_list(st, param, TokenType::Parameter)?;
                        continue 'restart;
                    }
                    _ => {}
                }
            }
        } else {
            crate::input::end_token_list(st)?;
            continue 'restart;
        }

        // An alignment entry ends at `&` or `\cr` when braces are balanced.
        if st.scanner.cur_cmd <= Cmd::CarRet
            && st.scanner.cur_cmd >= Cmd::TabMark
            && st.scanner.align_state == 0
        {
            if st.scanner.scanner_status == ScannerStatus::Aligning
                || st.scanner.cur_align.0 == TEX_NULL
            {
                return Err(Abort::Fatal(
                    "(interwoven alignment preambles are not allowed)",
                ));
            }
            let column = NodePtr::<AlignRecord>::at(st.scanner.cur_align);
            let saved_cmd = column.extra_info(&st.mem);
            column.set_extra_info(&mut st.mem, st.scanner.cur_chr);
            let template = if Cmd::from_stored(saved_cmd) == Cmd::Omit {
                OMIT_TEMPLATE
            } else {
                column.v_part(&st.mem)
            };
            begin_token_list(st, template, TokenType::VTemplate)?;
            st.scanner.cur_cmd = Cmd::from_stored(saved_cmd);
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
        if st.scanner.cur_cmd != Cmd::Spacer {
            return Ok(());
        }
    }
}

#[inline(always)]
fn pack_cur_tok(st: &mut EngineState) {
    st.scanner.cur_tok = if st.scanner.cur_cs == 0 {
        Token::new(st.scanner.cur_cmd, st.scanner.cur_chr)
    } else {
        Token::from_cs(st.scanner.cur_cs)
    };
}

pub(crate) fn x_token(st: &mut EngineState) -> Result<(), Abort> {
    while st.scanner.cur_cmd > Cmd::MAX_COMMAND {
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
        if st.scanner.cur_cmd <= Cmd::MAX_COMMAND {
            break;
        }
        if st.scanner.cur_cmd >= Cmd::Call {
            if st.scanner.cur_cmd < Cmd::EndTemplate {
                macro_call(st)?;
            } else {
                st.scanner.cur_cs = FROZEN_ENDV;
                st.scanner.cur_cmd = Cmd::Endv;
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
        if st.scanner.cur_cmd == Cmd::FiOrElse {
            if l == 0 {
                break;
            }
            if st.scanner.cur_chr == FiOrElseCode::Fi.raw() {
                l -= 1;
            }
        } else if st.scanner.cur_cmd == Cmd::IfTest {
            l += 1;
        }
    }
    st.scanner.scanner_status = save_scanner_status;
    if st.eqtb.int_par(IntPar::TracingIfs) > 0 {
        crate::output::show_cur_cmd_chr(st);
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
    let cmd = if t.is_cs() {
        let cmd = st.eqtb.eq_type(t.cs());
        if cmd >= Cmd::OuterCall
            || t == st.input.par_token && st.scanner.long_state != Cmd::LongCall
        {
            return false;
        }
        cmd
    } else {
        if t.cmd() == Cmd::OUT_PARAM {
            return false;
        }
        t.cmd()
    };
    !((cmd == Cmd::TabMark || cmd == Cmd::CarRet) && st.scanner.align_state == 0
        || stop_at_mac_param && cmd == Cmd::MacParam)
}

/// Advances past the token at `q` as `get_next` would and returns the next node.
#[inline(always)]
fn consume_list_token(st: &mut EngineState, q: MemPtr) -> MemPtr {
    st.scanner.logical_clock = st.scanner.logical_clock.wrapping_add(1);
    let next = q.link(&st.mem);
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
    p: &mut MemPtr,
    unbalance: &mut i32,
    stop_at_mac_param: bool,
) -> Result<bool, Abort> {
    if st.input.cur_input.state != TOKEN_LIST {
        return Ok(false);
    }
    let mut q = st.input.cur_input.token_loc();
    while !q.is_null() {
        let t = q.token(&st.mem);
        if t.is_brace() {
            if t.is_left_brace() {
                st.scanner.align_state += 1;
                *unbalance += 1;
            } else {
                st.scanner.align_state -= 1;
                *unbalance -= 1;
                if *unbalance == 0 {
                    consume_list_token(st, q);
                    st.scanner.cur_cs = 0;
                    st.scanner.cur_cmd = t.cmd();
                    st.scanner.cur_chr = t.chr();
                    st.scanner.cur_tok = t;
                    return Ok(true);
                }
            }
        } else if !plain_list_token(st, t, stop_at_mac_param) {
            break;
        }
        st.mem.store_new_token(p, t)?;
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
    p: &mut MemPtr,
    m: &mut i32,
    delim: Token,
) -> Result<(), Abort> {
    if st.input.cur_input.state != TOKEN_LIST {
        return Ok(());
    }
    let mut q = st.input.cur_input.token_loc();
    while !q.is_null() {
        let t = q.token(&st.mem);
        if t == delim || t.is_brace() || !plain_list_token(st, t, false) {
            break;
        }
        st.mem.store_new_token(p, t)?;
        *m += 1;
        q = consume_list_token(st, q);
    }
    Ok(())
}
