//! Building token lists: `str_toks`, `the_toks`, `scan_toks`, `read_toks` and `\scantokens`.

use crate::bridge::output_flush;
use crate::cmd::Cmd;
use crate::codes::{ScannerStatus, TheCode, ValueLevel};
use crate::command::RelaxCode;
use crate::consts::{ACTIVE_BASE, CLOSED, JUST_OPEN, NORMAL};
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::expand::expand;
use crate::input::NEW_LINE;
use crate::node::PseudoWord;
use crate::node::{NodePtr, PseudoFileNode, PseudoLineNode};
use crate::output::{print, print_int};
use crate::params::IntPar;
use crate::state::{EngineState, MemState, Selector};
use crate::token::{Build, TokCursor, Token, TokenList};
use crate::units::CsId;
use crate::units::{BufPos, StrNumber};

use super::internal::{scan_something_internal, CurVal};
use super::next::{get_next, get_token, get_x_token, x_token};
use super::scan_left_brace;
use crate::command::Command;

const ZERO_TOKEN: Token = Token::other(b'0');

impl MemState {
    /// Appends `str_pool[b..pool_ptr]` to `out` as tokens and drops it from the pool.
    pub(crate) fn str_toks_cat(
        &mut self,
        strings: &mut crate::state::StringsState,
        b: crate::units::PoolPos,
        cat: Option<Cmd>,
        out: &Build,
    ) -> Result<(), Abort> {
        if strings.pool_ptr + 1 > strings.pool_size {
            return Err(strings.pool_overflow());
        }

        let mut k = b;

        while k < strings.pool_ptr {
            let mut c = i32::from(strings.str_pool[k.index()]);

            let t = if c == i32::from(b' ') && cat.is_none() {
                Token::SPACE
            } else {
                if (0xD800..0xDC00).contains(&c) && k + 1 < strings.pool_ptr {
                    let lo = i32::from(strings.str_pool[(k + 1).index()]);
                    if (0xDC00..0xE000).contains(&lo) {
                        k += 1;
                        c = 0x10000
                            + ((c - 0xD800) << 10)
                            + (i32::from(strings.str_pool[k.index()]) - 0xDC00);
                    }
                }

                match cat {
                    None => Token::new(Cmd::OtherChar, c),
                    Some(Cmd::ACTIVE_CHAR) => Token::from_cs(ACTIVE_BASE + c),
                    Some(cat) => Token::new(cat, c),
                }
            };

            self.tokens.push(out, t)?;
            k += 1;
        }

        strings.pool_ptr = b;
        Ok(())
    }
}

/// Appends `\endlinechar` to the line just read and starts reading it.
#[inline]
pub(crate) fn finish_line(st: &mut EngineState) {
    let end_char = st.eqtb.int_par(IntPar::EndLineChar);
    if (0..=255).contains(&end_char) {
        st.input.buffer[st.input.cur_input.limit] = end_char;
    } else {
        st.input.cur_input.limit -= 1;
    }
    st.input.first = st.input.cur_input.limit + 1;
    st.input.cur_input.loc = st.input.cur_input.start;
}

fn close_read_file(st: &mut EngineState, m: usize) -> Result<(), Abort> {
    let slot = std::mem::take(&mut st.input.read_file[m]);
    if crate::io::u_close(slot) {
        return Err(Abort::InputClose);
    }
    st.input.read_open[m] = CLOSED;
    Ok(())
}

pub(crate) fn read_toks(
    st: &mut EngineState,
    n: i32,
    r: CsId,
    line: bool,
) -> Result<TokenList, Abort> {
    st.scanner.scanner_status = ScannerStatus::Defining;
    st.scanner.warning_index = r;
    let b = st.mem.tokens.begin()?;
    st.mem.tokens.push(&b, Token::END_MATCH)?;

    let m = if (0..=15).contains(&n) {
        n as usize
    } else {
        16
    };
    let s = st.scanner.align_state;
    st.scanner.align_state = 1_000_000;

    loop {
        crate::input::begin_file_reading(st)?;
        st.input.cur_input.name = match u8::try_from(m) {
            Ok(stream) if stream < 16 => crate::state::InputName::Read(stream),
            _ => crate::state::InputName::ReadTerminal,
        };

        if st.input.read_open[m] == CLOSED {
            return Err(Abort::Internal("terminal input forbidden"));
        } else if st.input.read_open[m] == JUST_OPEN {
            if crate::io::input_slot_line(st, crate::io::FileSlot::Read(m))? != 0 {
                st.input.read_open[m] = NORMAL as u8;
            } else {
                close_read_file(st, m)?;
            }
        } else {
            if crate::io::input_slot_line(st, crate::io::FileSlot::Read(m))? == 0 {
                close_read_file(st, m)?;
                if st.scanner.align_state != 1_000_000 {
                    crate::errors::runaway(st);
                    st.scanner.align_state = 1_000_000;
                    st.input.cur_input.limit = BufPos::ZERO;
                    crate::errors::error(
                        st,
                        ("File ended within ", crate::output::Esc("read")),
                        &[r"This \read has unbalanced braces."],
                    )?;
                }
            }
        }

        st.input.cur_input.limit = st.input.last;
        finish_line(st);
        st.input.cur_input.state = NEW_LINE;

        if line {
            let mut loc = st.input.cur_input.loc;
            while loc <= st.input.cur_input.limit {
                let c = st.input.buffer[loc];
                loc += 1;
                let tok = if c == i32::from(b' ') {
                    Token::SPACE
                } else {
                    Token::new(Cmd::OtherChar, c)
                };
                st.scanner.cur_tok = tok;
                st.mem.tokens.push(&b, tok)?;
            }
        } else {
            loop {
                get_token(st)?;
                if st.scanner.cur_tok == Token::ZERO {
                    break;
                }
                if st.scanner.align_state < 1_000_000 {
                    loop {
                        get_token(st)?;
                        if st.scanner.cur_tok == Token::ZERO {
                            break;
                        }
                    }
                    st.scanner.align_state = 1_000_000;
                    break;
                }
                st.mem.tokens.push(&b, st.scanner.cur_tok)?;
            }
        }
        crate::input::end_file_reading(st)?;

        if st.scanner.align_state == 1_000_000 {
            break;
        }
    }

    st.scanner.scanner_status = ScannerStatus::Normal;
    st.scanner.align_state = s;
    let list = st.mem.tokens.finish(b)?;
    st.scanner.def_ref = list;
    Ok(list)
}

fn tokens_to_string(st: &mut EngineState, list: TokenList) -> Result<StrNumber, Abort> {
    if st.shipout.selector == Selector::NewString {
        return Err(Abort::PdfError(&[
            "tokens",
            "tokens_to_string() called while selector = new_string",
        ]));
    }
    st.shipout.old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    let arg = st.mem.tokens.cursor(list);
    let l = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list(st, arg, TokCursor::END, l);
    st.shipout.selector = st.shipout.old_setting;
    st.strings.make_string()
}

/// Appends to `out` what `\the`, `\unexpanded` or `\detokenize` gives.
pub(crate) fn the_toks(st: &mut EngineState, code: TheCode, out: &Build) -> Result<(), Abort> {
    if code != TheCode::The {
        let text = scan_general_text(st)?;
        if code == TheCode::Unexpanded {
            return st.mem.tokens.push_list(out, text);
        }
        let old_setting = st.shipout.selector;
        st.shipout.selector = Selector::NewString;
        let b = st.strings.pool_ptr;
        crate::output::token_show(st, text);
        st.mem.tokens.flush(text);
        st.shipout.selector = old_setting;
        return st.mem.str_toks_cat(&mut st.strings, b, None, out);
    }
    get_x_token(st)?;
    let val = scan_something_internal(st, ValueLevel::Tok, false)?;
    if let CurVal::Ident(_) | CurVal::Tok(_) = val {
        match val {
            CurVal::Ident(loc) => st.mem.tokens.push(out, Token::from_cs(loc))?,
            CurVal::Tok(list) => {
                let mut c = st.mem.tokens.cursor(list);
                while let Some(t) = st.mem.tokens.read(&mut c) {
                    st.mem.tokens.push(out, t)?;
                }
            }
            _ => {}
        }
        Ok(())
    } else {
        let old_setting = st.shipout.selector;
        st.shipout.selector = Selector::NewString;
        let b = st.strings.pool_ptr;
        match val {
            CurVal::Int(n) => print_int(st, n),
            CurVal::Dimen(d) => {
                crate::output::print_scaled(st, d);
                print(st, "pt");
            }
            CurVal::Glue(spec) | CurVal::Mu(spec) => {
                let unit = if let CurVal::Mu(_) = val { "mu" } else { "pt" };
                crate::output::print_spec(st, spec, Some(unit));
                st.mem.delete_glue_ref(spec);
            }
            CurVal::Ident(_) | CurVal::Tok(_) => {}
        }
        st.shipout.selector = old_setting;
        st.mem.str_toks_cat(&mut st.strings, b, None, out)
    }
}
/// Scans a definition's parameter text and body, or a general text, into `def_ref`.
pub(crate) fn scan_toks(
    st: &mut EngineState,
    macro_def: bool,
    xpand: bool,
) -> Result<TokenList, Abort> {
    if macro_def {
        st.scanner.scanner_status = ScannerStatus::Defining;
    } else {
        st.scanner.scanner_status = ScannerStatus::Absorbing;
    }
    st.scanner.warning_index = st.scanner.cur_cs;
    let b = st.mem.tokens.begin()?;
    let mut hash_brace = Token::ZERO;
    let mut t = ZERO_TOKEN;

    'found: {
        if macro_def {
            let ended_with_brace = loop {
                get_token(st)?;
                if st.scanner.cur_tok.is_brace() {
                    break false;
                }
                if let Command::MacParam(c) = st.scanner.cur {
                    let s = Token::match_(c.raw());
                    get_token(st)?;
                    if st.scanner.cur_tok.is_left_brace() {
                        hash_brace = st.scanner.cur_tok;
                        st.mem.tokens.push(&b, st.scanner.cur_tok)?;
                        st.mem.tokens.push(&b, Token::END_MATCH)?;
                        break true;
                    }
                    if t == Token::other(b'9') {
                        crate::errors::error(
                            st,
                            "You already have nine parameters",
                            &[
                                "I'm going to ignore the # sign you just used,",
                                "as well as the token that followed it.",
                            ],
                        )?;
                        continue;
                    }
                    t = Token(t.0 + 1);
                    if st.scanner.cur_tok != t {
                        crate::errors::back_error(
                            st,
                            "Parameters must be numbered consecutively",
                            &[
                                "I've inserted the digit you should have used after the #.",
                                "Type `1' to delete what you did use.",
                            ],
                        )?;
                    }
                    st.scanner.cur_tok = s;
                }
                st.mem.tokens.push(&b, st.scanner.cur_tok)?;
            };
            if !ended_with_brace {
                st.mem.tokens.push(&b, Token::END_MATCH)?;
                if st.scanner.cur.cmd() == Cmd::RightBrace {
                    st.scanner.align_state += 1;
                    crate::errors::error(
                        st,
                        "Missing { inserted",
                        &[
                            r"Where was the left brace? You said something like `\def\a}',",
                            r"which I'm going to interpret as `\def\a{}'.",
                        ],
                    )?;
                    break 'found;
                }
            }
        } else {
            scan_left_brace(st)?;
        }

        let mut unbalance: i32 = 1;
        loop {
            if xpand {
                loop {
                    get_next(st)?;
                    let body = st.scanner.cur.macro_body();
                    if body.is_some_and(|b| st.mem.is_protected_macro(b)) {
                        st.scanner.cur = Command::Relax(RelaxCode::NoExpand);
                    }
                    if st.scanner.cur.cmd() <= Cmd::MAX_COMMAND {
                        break;
                    }
                    if let Command::The(code) = st.scanner.cur {
                        the_toks(st, code, &b)?;
                    } else {
                        expand(st)?;
                    }
                }
                x_token(st)?;
            } else {
                if super::next::copy_group_tokens(st, &b, &mut unbalance, macro_def)? {
                    break 'found;
                }
                get_token(st)?;
            }

            if st.scanner.cur_tok.is_brace() {
                if st.scanner.cur.cmd() < Cmd::RightBrace {
                    unbalance += 1;
                } else {
                    unbalance -= 1;
                    if unbalance == 0 {
                        break 'found;
                    }
                }
            } else if st.scanner.cur.cmd() == Cmd::MacParam && macro_def {
                let s = st.scanner.cur_tok;
                if xpand {
                    get_x_token(st)?;
                } else {
                    get_token(st)?;
                }
                if st.scanner.cur.cmd() != Cmd::MacParam {
                    if st.scanner.cur_tok <= ZERO_TOKEN || st.scanner.cur_tok > t {
                        crate::errors::back_error(
                            st,
                            (
                                "Illegal parameter number in definition of ",
                                crate::output::CsName(st.scanner.warning_index),
                            ),
                            &[
                                "You meant to type ## instead of #, right?",
                                "Or maybe a } was forgotten somewhere earlier, and things",
                                "are all screwed up? I'm going to assume that you meant ##.",
                            ],
                        )?;
                        st.scanner.cur_tok = s;
                    } else {
                        st.scanner.cur_tok =
                            Token::out_param(st.scanner.cur_tok.chr() - i32::from(b'0'));
                    }
                }
            }
            st.mem.tokens.push(&b, st.scanner.cur_tok)?;
        }
    }
    st.scanner.scanner_status = ScannerStatus::Normal;
    if hash_brace != Token::ZERO {
        st.mem.tokens.push(&b, hash_brace)?;
    }
    let list = st.mem.tokens.finish(b)?;
    st.scanner.def_ref = list;
    Ok(list)
}

pub(crate) fn pseudo_start(st: &mut EngineState) -> Result<(), Abort> {
    let text = scan_general_text(st)?;
    let old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    crate::output::token_show(st, text);
    st.shipout.selector = old_setting;
    st.mem.tokens.flush(text);

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(st.strings.pool_overflow());
    }
    let s = st.strings.make_string()?;
    st.strings.str_pool[st.strings.pool_ptr.index()] = u16::from(b' ');
    let mut l = st.strings.str_start(s);
    let nl = st.eqtb.int_par(IntPar::NewLineChar) as u16;
    let file = NodePtr::<PseudoFileNode>::alloc(&mut st.mem)?;
    st.mem[file].lines = None;
    let mut last_line: Option<NodePtr<PseudoLineNode>> = None;

    // Each line becomes a node of words holding four characters each.
    while l < st.strings.pool_ptr {
        let m = l;
        while l < st.strings.pool_ptr && st.strings.str_pool[l.index()] != nl {
            l += 1;
        }
        let size = ((l - m + 7) / 4).max(2);
        let line = NodePtr::<PseudoLineNode>::alloc(&mut st.mem)?;
        match last_line {
            None => st.mem[file].lines = line.into(),
            Some(prev) => st.mem[prev].link = line.into(),
        }
        last_line = Some(line);
        let node = &mut st.mem[line];
        node.size = size;
        node.link = None;
        st.mem
            .alloc_payload(line, usize::try_from(size - 1).or_confusion("pseudo line")?)?;
        let mut k = m;
        if let Some((last, full)) = st.mem.entries_mut(line).split_last_mut() {
            for word in full {
                let pool = &st.strings.str_pool[k.index()..];
                *word = PseudoWord::new([pool[0], pool[1], pool[2], pool[3]]);
                k += 4;
            }
            let mut chars = [u16::from(b' '); 4];
            for (i, c) in chars.iter_mut().enumerate() {
                if l > k + i as i32 {
                    *c = st.strings.str_pool[(k + i as i32).index()];
                }
            }
            *last = PseudoWord::new(chars);
        }
        if st.strings.str_pool[l.index()] == nl {
            l += 1;
        }
    }

    st.mem[file].link = st.input.pseudo_files;
    st.input.pseudo_files = file.ptr();
    st.strings.flush_str(s);

    crate::input::begin_file_reading(st)?;
    st.mem.line = 0;
    let start = st.input.cur_input.start;
    st.input.cur_input.limit = start;
    st.input.cur_input.loc = start + 1;

    if st.eqtb.int_par(IntPar::TracingScanTokens) > 0 {
        if st.shipout.term_offset > st.config.max_print_line - 3 {
            crate::output::print_ln(st);
        } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
            crate::output::print_char(st, b' ');
        }
        st.input.cur_input.name = crate::state::InputName::Pseudo { traced: true };
        print(st, "( ");
        st.input.open_parens += 1;
        output_flush(st.shipout.rust_stdout);
    } else {
        st.input.cur_input.name = crate::state::InputName::Pseudo { traced: false };
        st.input.cur_input.synctex_tag = 0;
        st.mem.cur_synctex_tag = st.input.cur_input.synctex_tag;
    }
    Ok(())
}
/// Scans a balanced text into a list of its own.
fn scan_general_text(st: &mut EngineState) -> Result<TokenList, Abort> {
    let saved_status = st.scanner.scanner_status;
    let saved_warning_index = st.scanner.warning_index;
    st.scanner.scanner_status = ScannerStatus::Absorbing;
    st.scanner.warning_index = st.scanner.cur_cs;
    let b = st.mem.tokens.begin()?;
    scan_left_brace(st)?;
    let mut unbalance: i32 = 1;
    loop {
        get_token(st)?;
        if st.scanner.cur_tok.is_brace() {
            if st.scanner.cur.cmd() < Cmd::RightBrace {
                unbalance += 1;
            } else {
                unbalance -= 1;
                if unbalance == 0 {
                    break;
                }
            }
        }
        st.mem.tokens.push(&b, st.scanner.cur_tok)?;
    }
    st.scanner.scanner_status = saved_status;
    st.scanner.warning_index = saved_warning_index;
    st.mem.tokens.finish(b)
}

pub(crate) fn scan_pdf_ext_toks(st: &mut EngineState) -> Result<(), Abort> {
    scan_toks(st, false, true)?;
    Ok(())
}

fn scan_toks_to_string(st: &mut EngineState) -> Result<StrNumber, Abort> {
    scan_toks(st, false, true)?;
    let s = tokens_to_string(st, st.scanner.def_ref)?;
    st.mem.tokens.delete_ref(st.scanner.def_ref);
    Ok(s)
}

/// `\strcmp`: compares two general texts as strings.
pub(crate) fn compare_strings(st: &mut EngineState) -> Result<i32, Abort> {
    let save_cur_cs = st.scanner.cur_cs;
    let s1 = scan_toks_to_string(st)?;
    st.scanner.cur_cs = save_cur_cs;
    let s2 = scan_toks_to_string(st)?;

    let range = |s: StrNumber| &st.strings.str_pool[st.strings.str_range(s)];
    let result = match range(s1).cmp(range(s2)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    };

    st.strings.flush_str(s2);
    st.strings.flush_str(s1);
    Ok(result)
}
