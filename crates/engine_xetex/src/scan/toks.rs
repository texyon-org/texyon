//! Building token lists: `str_toks`, `the_toks`, `scan_toks`, `read_toks` and `\scantokens`.

use crate::bridge::output_flush;
use crate::cmd::Cmd;
use crate::codes::{ScannerStatus, TheCode, ValueLevel};
use crate::consts::{ACTIVE_BASE, CLOSED, JUST_OPEN, NORMAL, TEX_NULL};
use crate::errors::Abort;
use crate::expand::expand;
use crate::input::NEW_LINE;
use crate::node::{NodePtr, PseudoFileNode, PseudoLineNode};
use crate::output::{print, print_int};
use crate::overlay::PseudoWord;
use crate::params::IntPar;
use crate::state::{EngineState, MemState, Selector};
use crate::token::TEMP_HEAD_TOKS;
use crate::token::{Token, TokenList};
use crate::units::{BufPos, EqtbPtr, StrNumber};

use super::internal::{scan_something_internal, CurVal};
use super::next::{get_next, get_token, get_x_token, x_token};
use super::{scan_left_brace, NO_EXPAND_FLAG};
use crate::command::Command;

const ZERO_TOKEN: Token = Token::other(b'0');

impl MemState {
    /// Converts `str_pool[b..pool_ptr]` to a token list at `link(temp_head)` and returns its tail.
    pub(crate) fn str_toks_cat(
        &mut self,
        strings: &mut crate::state::StringsState,
        b: crate::units::PoolPos,
        cat: Option<Cmd>,
    ) -> Result<TokenList, Abort> {
        if strings.pool_ptr + 1 > strings.pool_size {
            return Err(strings.pool_overflow());
        }

        let mut p = TEMP_HEAD_TOKS;
        p.set_link(&mut self.tokens, TokenList::NULL);
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

            let q = self.tokens.get_avail()?;
            p.set_link(&mut self.tokens, q);
            q.set_token(&mut self.tokens, t);
            p = q;
            k += 1;
        }

        strings.pool_ptr = b;
        Ok(p)
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
    r: EqtbPtr,
    line: bool,
) -> Result<TokenList, Abort> {
    st.scanner.scanner_status = ScannerStatus::Defining;
    st.scanner.warning_index = r;
    let def_ref = new_def_ref(st)?;
    let mut p = def_ref;
    store_new_token(st, &mut p, Token::END_MATCH)?;

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
                st.scanner.cur_chr = c;
                st.scanner.cur_tok = tok;
                store_new_token(st, &mut p, tok)?;
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
                store_new_token(st, &mut p, st.scanner.cur_tok)?;
            }
        }
        crate::input::end_file_reading(st)?;

        if st.scanner.align_state == 1_000_000 {
            break;
        }
    }

    st.scanner.scanner_status = ScannerStatus::Normal;
    st.scanner.align_state = s;
    Ok(def_ref)
}

fn tokens_to_string(st: &mut EngineState, p: TokenList) -> Result<StrNumber, Abort> {
    if st.shipout.selector == Selector::NewString {
        return Err(Abort::PdfError(&[
            "tokens",
            "tokens_to_string() called while selector = new_string",
        ]));
    }
    st.shipout.old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    let arg = p.link(&st.mem.tokens);
    let l = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list(st, arg, TokenList::NULL, l);
    st.shipout.selector = st.shipout.old_setting;
    st.strings.make_string()
}

/// Appends `token` to the token list whose tail is `tail`.
#[inline(always)]
pub(crate) fn store_new_token(
    st: &mut EngineState,
    tail: &mut TokenList,
    token: Token,
) -> Result<(), Abort> {
    st.mem.tokens.store_new_token(tail, token)
}

/// Starts a new reference-counted token list in `def_ref` and returns it.
fn new_def_ref(st: &mut EngineState) -> Result<TokenList, Abort> {
    let def_ref = st.mem.tokens.get_avail()?;
    st.scanner.def_ref = def_ref;
    def_ref.set_ref_count(&mut st.mem.tokens, TEX_NULL);
    Ok(def_ref)
}

pub(crate) fn the_toks(st: &mut EngineState, code: TheCode) -> Result<TokenList, Abort> {
    if code != TheCode::The {
        let tail = scan_general_text(st)?;
        if code == TheCode::Unexpanded {
            return Ok(tail);
        }
        let old_setting = st.shipout.selector;
        st.shipout.selector = Selector::NewString;
        let b = st.strings.pool_ptr;
        let p = st.mem.tokens.get_avail()?;
        let list = TEMP_HEAD_TOKS.link(&st.mem.tokens);
        p.set_link(&mut st.mem.tokens, list);
        crate::output::token_show(st, p);
        st.mem.tokens.flush_list(p);
        st.shipout.selector = old_setting;
        return st.mem.str_toks_cat(&mut st.strings, b, None);
    }
    get_x_token(st)?;
    let val = scan_something_internal(st, ValueLevel::Tok, false)?;
    if let CurVal::Ident(_) | CurVal::Tok(_) = val {
        let mut p = TEMP_HEAD_TOKS;
        p.set_link(&mut st.mem.tokens, TokenList::NULL);
        match val {
            CurVal::Ident(loc) => store_new_token(st, &mut p, Token::from_cs(loc))?,
            CurVal::Tok(list) if !list.is_null() => {
                let mut r = list.link(&st.mem.tokens);
                while !r.is_null() {
                    store_new_token(st, &mut p, r.token(&st.mem.tokens))?;
                    r = r.link(&st.mem.tokens);
                }
            }
            _ => {}
        }
        Ok(p)
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
                crate::output::print_spec(st, spec.ptr(), Some(unit));
                st.mem.delete_glue_ref(spec);
            }
            CurVal::Ident(_) | CurVal::Tok(_) => {}
        }
        st.shipout.selector = old_setting;
        st.mem.str_toks_cat(&mut st.strings, b, None)
    }
}
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
    let def_ref = new_def_ref(st)?;
    let mut p = def_ref;
    let mut hash_brace = Token::ZERO;
    let mut t = ZERO_TOKEN;

    'found: {
        if macro_def {
            let ended_with_brace = loop {
                get_token(st)?;
                if st.scanner.cur_tok.is_brace() {
                    break false;
                }
                if st.scanner.cur_cmd == Cmd::MacParam {
                    let s = Token::match_(st.scanner.cur_chr);
                    get_token(st)?;
                    if st.scanner.cur_tok.is_left_brace() {
                        hash_brace = st.scanner.cur_tok;
                        store_new_token(st, &mut p, st.scanner.cur_tok)?;
                        store_new_token(st, &mut p, Token::END_MATCH)?;
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
                store_new_token(st, &mut p, st.scanner.cur_tok)?;
            };
            if !ended_with_brace {
                store_new_token(st, &mut p, Token::END_MATCH)?;
                if st.scanner.cur_cmd == Cmd::RightBrace {
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
                    if matches!(
                        st.scanner.try_command(),
                        Some(Command::Call { body, .. }) if st.mem.is_protected_macro(body)
                    ) {
                        st.scanner.cur_cmd = Cmd::Relax;
                        st.scanner.cur_chr = NO_EXPAND_FLAG;
                    }
                    if st.scanner.cur_cmd <= Cmd::MAX_COMMAND {
                        break;
                    }
                    if let Some(Command::The(code)) = st.scanner.try_command() {
                        let tail = the_toks(st, code)?;
                        let list = TEMP_HEAD_TOKS.link(&st.mem.tokens);
                        if !list.is_null() {
                            p.set_link(&mut st.mem.tokens, list);
                            p = tail;
                        }
                    } else {
                        expand(st)?;
                    }
                }
                x_token(st)?;
            } else {
                if super::next::copy_group_tokens(st, &mut p, &mut unbalance, macro_def)? {
                    break 'found;
                }
                get_token(st)?;
            }

            if st.scanner.cur_tok.is_brace() {
                if st.scanner.cur_cmd < Cmd::RightBrace {
                    unbalance += 1;
                } else {
                    unbalance -= 1;
                    if unbalance == 0 {
                        break 'found;
                    }
                }
            } else if st.scanner.cur_cmd == Cmd::MacParam && macro_def {
                let s = st.scanner.cur_tok;
                if xpand {
                    get_x_token(st)?;
                } else {
                    get_token(st)?;
                }
                if st.scanner.cur_cmd != Cmd::MacParam {
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
                        st.scanner.cur_tok = Token::out_param(st.scanner.cur_chr - i32::from(b'0'));
                    }
                }
            }
            store_new_token(st, &mut p, st.scanner.cur_tok)?;
        }
    }
    st.scanner.scanner_status = ScannerStatus::Normal;
    if hash_brace != Token::ZERO {
        store_new_token(st, &mut p, hash_brace)?;
    }
    Ok(p)
}

pub(crate) fn pseudo_start(st: &mut EngineState) -> Result<(), Abort> {
    scan_general_text(st)?;
    let old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    crate::output::token_show(st, TEMP_HEAD_TOKS);
    st.shipout.selector = old_setting;
    let head = TEMP_HEAD_TOKS.link(&st.mem.tokens);
    st.mem.tokens.flush_list(head);

    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(st.strings.pool_overflow());
    }
    let s = st.strings.make_string()?;
    st.strings.str_pool[st.strings.pool_ptr.index()] = u16::from(b' ');
    let mut l = st.strings.str_start(s);
    let nl = st.eqtb.int_par(IntPar::NewLineChar) as u16;
    let file = NodePtr::<PseudoFileNode>::alloc(&mut st.mem)?;
    st.mem[file].lines = None.into();
    let mut last_line: Option<NodePtr<PseudoLineNode>> = None;

    // Each line becomes a node of words holding four characters each.
    while l < st.strings.pool_ptr {
        let m = l;
        while l < st.strings.pool_ptr && st.strings.str_pool[l.index()] != nl {
            l += 1;
        }
        let size = ((l - m + 7) / 4).max(2);
        let line = NodePtr::<PseudoLineNode>::at(st.mem.get_node(size)?);
        match last_line {
            None => st.mem[file].lines = line.into(),
            Some(prev) => st.mem[prev].link = line.into(),
        }
        last_line = Some(line);
        st.mem[line].size = size;
        st.mem[line].link = None.into();
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
/// Scans a balanced text into a list at `link(temp_head)` and returns its tail.
fn scan_general_text(st: &mut EngineState) -> Result<TokenList, Abort> {
    let saved_status = st.scanner.scanner_status;
    let saved_warning_index = st.scanner.warning_index;
    let saved_def_ref = st.scanner.def_ref;
    st.scanner.scanner_status = ScannerStatus::Absorbing;
    st.scanner.warning_index = st.scanner.cur_cs;
    let mut p = new_def_ref(st)?;
    scan_left_brace(st)?;
    let mut unbalance: i32 = 1;
    loop {
        get_token(st)?;
        if st.scanner.cur_tok.is_brace() {
            if st.scanner.cur_cmd < Cmd::RightBrace {
                unbalance += 1;
            } else {
                unbalance -= 1;
                if unbalance == 0 {
                    break;
                }
            }
        }
        store_new_token(st, &mut p, st.scanner.cur_tok)?;
    }
    let def_ref = st.scanner.def_ref;
    let list = def_ref.link(&st.mem.tokens);
    st.mem.tokens.free_avail(def_ref);
    let tail = if list.is_null() { TEMP_HEAD_TOKS } else { p };
    TEMP_HEAD_TOKS.set_link(&mut st.mem.tokens, list);
    st.scanner.scanner_status = saved_status;
    st.scanner.warning_index = saved_warning_index;
    st.scanner.def_ref = saved_def_ref;
    Ok(tail)
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
