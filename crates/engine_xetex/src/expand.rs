//! Expansion: `expand`, `macro_call`, conditionals and `conv_toks`.

use crate::cmd::{
    ACTIVE_CHAR, CALL, CONVERT, CS_NAME, END_CS_NAME, END_TEMPLATE, EXPAND_AFTER, FI_OR_ELSE,
    IF_TEST, IGNORE, INPUT, LEFT_BRACE, LONG_CALL, MAX_COMMAND, NO_EXPAND, OUTER_CALL, OUT_PARAM,
    RELAX, THE, TOP_BOT_MARK, UNDEFINED_CS,
};
use crate::codes::{
    ConvertCode, FiOrElseCode, IfCode, InputCode, MarkCode, ScannerStatus, TheCode, ValueLevel,
};
use crate::consts::{
    ACTIVE_BASE, BIGGEST_USV, FROZEN_DONT_EXPAND, FROZEN_ENDV, FROZEN_PRIMITIVE, HMODE, MMODE,
    NULL_CS, NULL_FONT, PRIM_EQTB_BASE, SINGLE_BASE, TEX_NULL, TOO_BIG_USV, UNDEFINED_PRIMITIVE,
    VMODE,
};
use crate::errors::{error, Abort};
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::input::{back_error, back_input, begin_token_list, TokenType, TOKEN_LIST};
use crate::mem::{BACKUP_HEAD, GARBAGE, TEMP_HEAD, ZERO_GLUE};
use crate::node::{AnyBox, AnyNode, CondNode, NodePtr};
use crate::output::{
    begin_diagnostic, capture_to_diagnostic, diagnostic_begin_capture_warning_here, end_diagnostic,
    error_here_with_diagnostic, print, print_char, print_cmd_chr, print_err, print_esc, print_int,
    print_scaled, show_cur_cmd_chr, sprint_cs,
};
use crate::params::{GluePar, IntPar};
use crate::scan::{
    copy_delimited_tokens, copy_group_tokens, fetch_box, get_next, get_token, get_x_non_blank,
    get_x_token, scan_int, store_new_token, NO_EXPAND_FLAG,
};
use crate::state::{EngineState, MemState, Selector};
use crate::token::Token;
use crate::units::{FontId, MemPtr, Scaled, StrNumber};

/// The `\the` variant in `cur_chr`.
pub(crate) fn the_code(st: &EngineState) -> Result<TheCode, Abort> {
    TheCode::from_raw(st.scanner.cur_chr).ok_or(Abort::Confusion("the"))
}

pub(crate) fn ins_the_toks(st: &mut EngineState) -> Result<(), Abort> {
    let tail = crate::scan::the_toks(st, the_code(st)?)?;
    ins_temp_head_list(st, tail)
}

/// Inserts the list at `link(temp_head)` whose tail is `tail`.
fn ins_temp_head_list(st: &mut EngineState, tail: MemPtr) -> Result<(), Abort> {
    GARBAGE.set_link(&mut st.mem, tail);
    begin_token_list(st, TEMP_HEAD.link(&st.mem), TokenType::Inserted)?;
    Ok(())
}

fn pop_cond_node(st: &mut EngineState) {
    let p = NodePtr::<CondNode>::at(st.scanner.cond_ptr);
    (st.scanner.cur_if, st.scanner.if_line, st.scanner.if_limit) = p.saved_state(&st.mem);
    st.scanner.cond_ptr = p.link(&st.mem);
    p.free(&mut st.mem);
}

fn pop_fi(st: &mut EngineState) {
    if st.input.if_stack[st.input.in_open as usize] == st.scanner.cond_ptr {
        crate::output::if_warning(st);
    }
    pop_cond_node(st);
}

/// Handles the `\else` or `\fi` that ended a skipped conditional branch.
fn skipped_branch_end(st: &mut EngineState) {
    if st.scanner.cur_chr == FiOrElseCode::Fi.raw() {
        pop_fi(st);
    } else {
        st.scanner.if_limit = FiOrElseCode::Fi;
    }
}

fn get_x_token_or_active_char(st: &mut EngineState) -> Result<(), Abort> {
    get_x_token(st)?;
    if st.scanner.cur_cmd == RELAX && st.scanner.cur_chr == NO_EXPAND_FLAG {
        st.scanner.cur_cmd = ACTIVE_CHAR;
        st.scanner.cur_chr = st.scanner.cur_tok.cs() - ACTIVE_BASE;
    }
    Ok(())
}

fn get_next_unexpanded(st: &mut EngineState) -> Result<(), Abort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = ScannerStatus::Normal;
    get_next(st)?;
    st.scanner.scanner_status = save_scanner_status;
    Ok(())
}

fn get_token_unexpanded(st: &mut EngineState) -> Result<(), Abort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = ScannerStatus::Normal;
    get_token(st)?;
    st.scanner.scanner_status = save_scanner_status;
    Ok(())
}

/// Copies the character tokens of `list` into `buffer[first..]`, returning the end index.
fn token_chars_to_buffer(st: &mut EngineState, list: MemPtr) -> Result<i32, Abort> {
    let mut j = st.input.first;
    let mut p = list;
    while !p.is_null() {
        if j >= st.input.max_buf_stack {
            st.input.max_buf_stack = j + 1;
            if st.input.max_buf_stack == st.input.buf_size {
                return Err(Abort::Overflow {
                    what: "buffer size",
                    n: st.input.buf_size,
                });
            }
        }
        st.input.buffer[j as usize] = p.token(&st.mem).chr();
        j += 1;
        p = p.link(&st.mem);
    }
    Ok(j)
}

/// Scans the tokens of a `\csname`-like name up to the terminating control sequence.
/// Returns the head node and the last node of the scanned list.
fn scan_cs_name_tokens(st: &mut EngineState) -> Result<(MemPtr, MemPtr), Abort> {
    let head = st.mem.get_avail()?;
    let mut tail = head;
    let saved_in_csname = st.scanner.is_in_csname;
    st.scanner.is_in_csname = true;
    loop {
        get_x_token(st)?;
        if st.scanner.cur_cs != 0 {
            break;
        }
        store_new_token(st, &mut tail, st.scanner.cur_tok)?;
    }
    if st.scanner.cur_cmd != END_CS_NAME {
        error_here_with_diagnostic(st, "Missing ");
        print_esc(st, "endcsname");
        print(st, " inserted");
        capture_to_diagnostic(st, None);
        st.errors.help(&[
            "The control sequence marked <to be read again> should",
            r"not appear between \csname and \endcsname.",
        ]);
        back_error(st)?;
    }
    st.scanner.is_in_csname = saved_in_csname;
    Ok((head, tail))
}

fn trace_diagnostic(st: &mut EngineState, text: &str) {
    begin_diagnostic(st);
    diagnostic_begin_capture_warning_here(st);
    print(st, text);
    capture_to_diagnostic(st, None);
    end_diagnostic(st, false);
}

fn change_if_limit(
    st: &mut EngineState,
    l: FiOrElseCode,
    p: impl crate::units::MemAddr,
) -> Result<(), Abort> {
    let p = p.addr();
    if p == st.scanner.cond_ptr.0 {
        st.scanner.if_limit = l;
        return Ok(());
    }
    let mut q = st.scanner.cond_ptr;
    while !q.is_null() {
        let next = q.link(&st.mem);
        if next.0 == p {
            NodePtr::<CondNode>::at(q).set_if_limit(&mut st.mem, l);
            return Ok(());
        }
        q = next;
    }
    Err(Abort::Confusion("if"))
}

pub(crate) fn conditional(st: &mut EngineState) -> Result<(), Abort> {
    if st.eqtb.int_par(IntPar::TracingIfs) > 0 && st.eqtb.int_par(IntPar::TracingCommands) <= 1 {
        show_cur_cmd_chr(st);
    }

    let cond_node = NodePtr::<CondNode>::alloc(&mut st.mem)?;
    cond_node.set_link(&mut st.mem, st.scanner.cond_ptr);
    cond_node.set_if_limit(&mut st.mem, st.scanner.if_limit);
    cond_node.set_cur_if(&mut st.mem, st.scanner.cur_if as u16);
    cond_node.set_if_line(&mut st.mem, st.scanner.if_line);
    st.scanner.cond_ptr = cond_node.ptr();
    st.scanner.cur_if = st.scanner.cur_chr as i16;
    st.scanner.if_limit = FiOrElseCode::If;
    st.scanner.if_line = st.mem.line;

    let save_cond_ptr = st.scanner.cond_ptr.0;
    let (this_if, is_unless) = IfCode::decode(st.scanner.cur_chr).ok_or(Abort::Confusion("if"))?;

    let mut b = match this_if {
        IfCode::Char | IfCode::Cat => {
            get_x_token_or_active_char(st)?;
            let (m, n) = if st.scanner.cur_cmd > ACTIVE_CHAR || st.scanner.cur_chr > BIGGEST_USV {
                (RELAX, TOO_BIG_USV)
            } else {
                (st.scanner.cur_cmd, st.scanner.cur_chr)
            };
            get_x_token_or_active_char(st)?;
            if st.scanner.cur_cmd > ACTIVE_CHAR || st.scanner.cur_chr > BIGGEST_USV {
                st.scanner.cur_cmd = RELAX;
                st.scanner.cur_chr = TOO_BIG_USV;
            }
            if this_if == IfCode::Char {
                n == st.scanner.cur_chr
            } else {
                m == st.scanner.cur_cmd
            }
        }
        IfCode::Num | IfCode::Dim => {
            scan_int_or_dimen(st, this_if)?;
            let n = st.scanner.cur_val;
            get_x_non_blank(st)?;
            let relation =
                if (Token::other(b'<')..=Token::other(b'>')).contains(&st.scanner.cur_tok) {
                    st.scanner.cur_tok.chr() as u8
                } else {
                    error_here_with_diagnostic(st, "Missing = inserted for ");
                    print_cmd_chr(st, IF_TEST, this_if.raw());
                    capture_to_diagnostic(st, None);
                    st.errors
                        .help(&["I was expecting to see `<', `=', or `>'. Didn't."]);
                    back_error(st)?;
                    b'='
                };
            scan_int_or_dimen(st, this_if)?;
            let cv = st.scanner.cur_val;
            match relation {
                b'<' => n < cv,
                b'=' => n == cv,
                _ => n > cv,
            }
        }
        IfCode::Odd => {
            scan_int(st)?;
            st.scanner.cur_val % 2 != 0
        }
        IfCode::VMode => st.input.cur_list.mode.abs() == VMODE,
        IfCode::HMode => st.input.cur_list.mode.abs() == HMODE,
        IfCode::MMode => st.input.cur_list.mode.abs() == MMODE,
        IfCode::Inner => st.input.cur_list.mode < 0,
        IfCode::Void | IfCode::HBox | IfCode::VBox => {
            crate::scan::scan_register_num(st)?;
            let contents = fetch_box(st, st.scanner.cur_val);
            match this_if {
                IfCode::Void => contents.is_none(),
                IfCode::HBox => matches!(contents, Some(AnyBox::HList(_))),
                _ => matches!(contents, Some(AnyBox::VList(_))),
            }
        }
        IfCode::X => {
            let save_scanner_status = st.scanner.scanner_status;
            st.scanner.scanner_status = ScannerStatus::Normal;
            get_next(st)?;
            let n = st.scanner.cur_cs;
            let p = st.scanner.cur_cmd;
            let q = st.scanner.cur_chr;
            get_next(st)?;
            let b = if st.scanner.cur_cmd != p {
                false
            } else if st.scanner.cur_cmd < CALL {
                st.scanner.cur_chr == q
            } else {
                let mut p = st.scanner.cur_chr_ptr().link(&st.mem);
                let mut q = st.eqtb.equiv_ptr(n).link(&st.mem);
                if p == q {
                    true
                } else {
                    while !p.is_null() && !q.is_null() {
                        if p.token(&st.mem) != q.token(&st.mem) {
                            p = MemPtr::NULL;
                        } else {
                            p = p.link(&st.mem);
                            q = q.link(&st.mem);
                        }
                    }
                    p.is_null() && q.is_null()
                }
            };
            st.scanner.scanner_status = save_scanner_status;
            b
        }
        IfCode::Eof => {
            crate::scan::scan_four_bit_int_or_18(st)?;
            if st.scanner.cur_val == 18 {
                !st.config.shell_escape_enabled
            } else {
                st.input.read_open[st.scanner.cur_val as usize] == crate::consts::CLOSED
            }
        }
        IfCode::True => true,
        IfCode::False => false,
        IfCode::Defined => {
            get_next_unexpanded(st)?;
            st.scanner.cur_cmd != UNDEFINED_CS
        }
        IfCode::CsName => {
            let (head, tail) = scan_cs_name_tokens(st)?;
            let first = st.input.first;
            let m = token_chars_to_buffer(st, head.link(&st.mem))?;
            st.scanner.cur_cs = if m == first {
                NULL_CS
            } else if m == first + 1 {
                SINGLE_BASE + st.input.buffer[first as usize]
            } else {
                st.eqtb
                    .id_lookup(&mut st.strings, &st.input.buffer, first, m - first)?
            };
            st.mem.flush_list_with_tail(head, tail);
            st.eqtb.eq_type(st.scanner.cur_cs) != UNDEFINED_CS
        }
        IfCode::InCsName => st.scanner.is_in_csname,
        IfCode::FontChar => {
            let f = crate::scan::scan_font_ident(st)?;
            crate::scan::scan_usv_num(st)?;
            let c = st.scanner.cur_val;
            let fa = st.fonts.font_area[f];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                st.fonts.map_char_to_glyph(f, c)? > 0
            } else if i32::from(st.fonts.font_bc[f]) <= c && i32::from(st.fonts.font_ec[f]) >= c {
                st.fonts.effective_char_info(f, c as u16).exists()
            } else {
                false
            }
        }
        IfCode::Case => {
            scan_int(st)?;
            let mut n = st.scanner.cur_val;
            if st.eqtb.int_par(IntPar::TracingCommands) > 1 {
                begin_diagnostic(st);
                diagnostic_begin_capture_warning_here(st);
                print(st, "{case ");
                print_int(st, n);
                print_char(st, b'}');
                capture_to_diagnostic(st, None);
                end_diagnostic(st, false);
            }
            while n != 0 {
                crate::scan::pass_text(st)?;
                if st.scanner.cond_ptr.0 == save_cond_ptr {
                    if st.scanner.cur_chr != FiOrElseCode::Or.raw() {
                        skipped_branch_end(st);
                        return Ok(());
                    }
                    n = n.wrapping_sub(1);
                } else if st.scanner.cur_chr == FiOrElseCode::Fi.raw() {
                    pop_fi(st);
                }
            }
            change_if_limit(st, FiOrElseCode::Or, save_cond_ptr)?;
            return Ok(());
        }
        IfCode::Primitive => {
            get_next_unexpanded(st)?;
            let m = st.eqtb.prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)?;
            st.scanner.cur_cmd != UNDEFINED_CS
                && m != UNDEFINED_PRIMITIVE
                && st.scanner.cur_cmd == st.eqtb.eq_type(PRIM_EQTB_BASE + m)
                && st.scanner.cur_chr == st.eqtb.equiv(PRIM_EQTB_BASE + m)
        }
    };

    if is_unless {
        b = !b;
    }

    if st.eqtb.int_par(IntPar::TracingCommands) > 1 {
        trace_diagnostic(st, if b { "{true}" } else { "{false}" });
    }

    if b {
        change_if_limit(st, FiOrElseCode::Else, save_cond_ptr)?;
        return Ok(());
    }

    loop {
        crate::scan::pass_text(st)?;
        if st.scanner.cond_ptr.0 == save_cond_ptr {
            if st.scanner.cur_chr != FiOrElseCode::Or.raw() {
                skipped_branch_end(st);
                return Ok(());
            }
            error_here_with_diagnostic(st, "Extra ");
            print_esc(st, "or");
            capture_to_diagnostic(st, None);
            st.errors
                .help(&[r"I'm ignoring this; it doesn't match any \if."]);
            error(st)?;
        } else if st.scanner.cur_chr == FiOrElseCode::Fi.raw() {
            pop_fi(st);
        }
    }
}

fn scan_int_or_dimen(st: &mut EngineState, this_if: IfCode) -> Result<(), Abort> {
    if this_if == IfCode::Num {
        scan_int(st)
    } else {
        crate::scan::scan_dimen(st, false, false, false)
    }
}

/// Scanner state saved around a `conv_toks` primitive that reads a general text.
struct SavedScan {
    status: ScannerStatus,
    warning_index: i32,
    def_ref: i32,
    pending_str: bool,
}

/// Saves the scanner state and seals a partially built string so the pool can be reused.
fn save_scan(st: &mut EngineState) -> Result<SavedScan, Abort> {
    let saved = SavedScan {
        status: st.scanner.scanner_status,
        warning_index: st.scanner.warning_index,
        def_ref: st.scanner.def_ref.0,
        pending_str: st.strings.cur_string_start() < st.strings.pool_ptr,
    };
    if saved.pending_str {
        st.strings.make_string()?;
    }
    Ok(saved)
}

fn restore_scan(st: &mut EngineState, saved: &SavedScan) {
    st.scanner.def_ref.0 = saved.def_ref;
    st.scanner.warning_index = saved.warning_index;
    st.scanner.scanner_status = saved.status;
}

fn unseal_pending_str(st: &mut EngineState, saved: &SavedScan) {
    if saved.pending_str {
        st.strings.str_ptr -= 1;
    }
}

/// Scans a general text and returns it as a new pool string.
fn scan_text_to_string(st: &mut EngineState, saved: &SavedScan) -> Result<StrNumber, Abort> {
    crate::scan::scan_pdf_ext_toks(st)?;
    if st.shipout.selector == Selector::NewString {
        return Err(Abort::PdfError(&[
            "tokens",
            "tokens_to_string() called while selector = new_string",
        ]));
    }
    let old_selector = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    let text = st.scanner.def_ref.link(&st.mem);
    let limit = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list(st, text, TEX_NULL, limit);
    st.shipout.selector = old_selector;
    let s = st.strings.make_string()?;
    st.mem.delete_token_ref(st.scanner.def_ref);
    restore_scan(st, saved);
    Ok(s)
}

/// Inserts the pool text from `b` produced for the file-query string `s`.
fn ins_file_query_result(
    st: &mut EngineState,
    saved: &SavedScan,
    s: StrNumber,
    b: i32,
) -> Result<(), Abort> {
    let tail = st.mem.str_toks_cat(&mut st.strings, b, 0)?;
    st.strings.flush_str(s);
    ins_temp_head_list(st, tail)?;
    unseal_pending_str(st, saved);
    Ok(())
}

fn scan_nonnegative_int(
    st: &mut EngineState,
    keyword: &str,
    err: &str,
    help: &'static str,
) -> Result<i32, Abort> {
    st.scanner.cur_val = 0;
    if crate::scan::scan_keyword(st, keyword)? {
        scan_int(st)?;
        if st.scanner.cur_val < 0 {
            print_err(st, err);
            st.errors.help(&[help, "I changed this one to zero."]);
            crate::errors::int_error(st, st.scanner.cur_val)?;
            st.scanner.cur_val = 0;
        }
    }
    Ok(st.scanner.cur_val)
}

impl MemState {
    /// Whether a node may be skipped when looking for a margin kern at the edge of a line.
    fn is_margin_kern_filler(&self, node: AnyNode, par_skip: GluePar) -> bool {
        match node {
            AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_) | AnyNode::Penalty(_) => true,
            AnyNode::Disc(d) => {
                d.pre_break(self).is_null() && d.post_break(self).is_null() && d.subtype(self) == 0
            }
            AnyNode::Math(m) => m.width(self) == Scaled::ZERO,
            AnyNode::Kern(k) => k.width(self) == Scaled::ZERO || k.subtype(self) == 0,
            AnyNode::Glue(g) => {
                g.glue_ptr(self) == ZERO_GLUE || g.subtype(self) == par_skip.subtype()
            }
            AnyNode::HList(h) => {
                h.width(self) == Scaled::ZERO
                    && h.height(self) == Scaled::ZERO
                    && h.depth(self) == Scaled::ZERO
                    && h.list_ptr(self).is_null()
            }
            _ => false,
        }
    }
}

/// Prints the width of the margin kern of `side` found by walking from `p` with `next`,
/// skipping fillers, or `0pt` if there is none.
fn print_margin_kern(
    st: &mut EngineState,
    mut p: MemPtr,
    side: u16,
    par_skip: GluePar,
    next: impl Fn(&crate::state::MemState, MemPtr) -> MemPtr,
) {
    let width = loop {
        if p.is_null() || st.mem.is_char_node(p) {
            break None;
        }
        match st.mem.classify(p) {
            Ok(node) if st.mem.is_margin_kern_filler(node, par_skip) => p = next(&st.mem, p),
            Ok(AnyNode::MarginKern(mk)) if mk.subtype(&st.mem) == side => {
                break Some(mk.width(&st.mem))
            }
            _ => break None,
        }
    };
    match width {
        Some(w) => print_scaled(st, w),
        None => print_char(st, i32::from(b'0')),
    }
    print(st, "pt");
}

pub(crate) fn conv_toks(st: &mut EngineState) -> Result<(), Abort> {
    let c = ConvertCode::from_raw(st.scanner.cur_chr).ok_or(Abort::Confusion("convert"))?;
    let mut cat: i32 = 0;
    let mut fnt = NULL_FONT;
    let mut arg1: i32 = 0;
    let mut arg2: i32 = 0;
    let mut margin_list = MemPtr::NULL;

    match c {
        ConvertCode::Number | ConvertCode::RomanNumeral | ConvertCode::UniformDeviate => {
            scan_int(st)?
        }
        ConvertCode::String | ConvertCode::Meaning => get_token_unexpanded(st)?,
        ConvertCode::FontName => fnt = crate::scan::scan_font_ident(st)?,
        ConvertCode::Expanded => {
            let saved = save_scan(st)?;
            crate::scan::scan_pdf_ext_toks(st)?;
            st.scanner.warning_index = saved.warning_index;
            st.scanner.scanner_status = saved.status;
            let def_ref = st.scanner.def_ref;
            let list = def_ref.link(&st.mem);
            begin_token_list(st, list, TokenType::Inserted)?;
            st.mem.free_avail(st.scanner.def_ref);
            st.scanner.def_ref.0 = saved.def_ref;
            unseal_pending_str(st, &saved);
            return Ok(());
        }
        ConvertCode::LeftMarginKern | ConvertCode::RightMarginKern => {
            crate::scan::scan_register_num(st)?;
            let Some(AnyBox::HList(hbox)) = fetch_box(st, st.scanner.cur_val) else {
                return Err(Abort::PdfError(&[
                    "marginkern",
                    "a non-empty hbox expected",
                ]));
            };
            margin_list = hbox.list_ptr(&st.mem);
        }
        ConvertCode::CreationDate => {
            let b = st.strings.pool_ptr;
            st.texmf.creation_date(&mut st.strings);
            let tail = st.mem.str_toks_cat(&mut st.strings, b, 0)?;
            ins_temp_head_list(st, tail)?;
        }
        ConvertCode::FileModDate => {
            let saved = save_scan(st)?;
            let s = scan_text_to_string(st, &saved)?;
            let b = st.strings.pool_ptr;
            st.strings.file_mod_date(s)?;
            return ins_file_query_result(st, &saved, s, b);
        }
        ConvertCode::FileSize => {
            let saved = save_scan(st)?;
            let s = scan_text_to_string(st, &saved)?;
            let b = st.strings.pool_ptr;
            st.strings.file_size(s)?;
            return ins_file_query_result(st, &saved, s, b);
        }
        ConvertCode::MdfiveSum => {
            let saved = save_scan(st)?;
            let is_file = crate::scan::scan_keyword(st, "file")?;
            let s = scan_text_to_string(st, &saved)?;
            let b = st.strings.pool_ptr;
            st.strings.md5_sum(s, is_file);
            return ins_file_query_result(st, &saved, s, b);
        }
        ConvertCode::FileDump => {
            let saved = save_scan(st)?;
            let offset = scan_nonnegative_int(
                st,
                "offset",
                "Bad file offset",
                "A file offset must be between 0 and 2^_31_-1,",
            )?;
            let length = scan_nonnegative_int(
                st,
                "length",
                "Bad dump length",
                "A dump length must be between 0 and 2^_31_-1,",
            )?;
            let s = scan_text_to_string(st, &saved)?;
            let b = st.strings.pool_ptr;
            st.strings.file_dump(s, offset, length)?;
            return ins_file_query_result(st, &saved, s, b);
        }
        ConvertCode::Strcmp => {
            let saved = save_scan(st)?;
            crate::scan::compare_strings(st)?;
            restore_scan(st, &saved);
            unseal_pending_str(st, &saved);
        }
        ConvertCode::Uchar => crate::scan::scan_usv_num(st)?,
        ConvertCode::Ucharcat => {
            crate::scan::scan_usv_num(st)?;
            let saved_chr = st.scanner.cur_val;
            scan_int(st)?;
            let code = st.scanner.cur_val;
            if code < i32::from(LEFT_BRACE)
                || code > i32::from(ACTIVE_CHAR)
                || code == i32::from(OUT_PARAM)
                || code == i32::from(IGNORE)
            {
                error_here_with_diagnostic(st, "Invalid code (");
                print_int(st, code);
                print(st, "), should be in the ranges 1..4, 6..8, 10..13");
                capture_to_diagnostic(st, None);
                st.errors
                    .help(&["I'm going to use 12 instead of that illegal code value."]);
                error(st)?;
                cat = 12;
            } else {
                cat = code;
            }
            st.scanner.cur_val = saved_chr;
        }
        ConvertCode::XetexVariationName => {
            fnt = crate::scan::scan_font_ident(st)?;
            if st.fonts.font_area[fnt] == AAT_FONT_FLAG {
                scan_int(st)?;
                arg1 = st.scanner.cur_val;
                arg2 = 0;
            } else {
                not_font_error(st, c, fnt, "; not an AAT font")?;
            }
        }
        ConvertCode::XetexFeatureName | ConvertCode::XetexSelectorName => {
            fnt = crate::scan::scan_font_ident(st)?;
            if is_aat_or_graphite(st, fnt) {
                scan_int(st)?;
                arg1 = st.scanner.cur_val;
                if c == ConvertCode::XetexSelectorName {
                    scan_int(st)?;
                    arg2 = st.scanner.cur_val;
                } else {
                    arg2 = 0;
                }
            } else {
                not_font_error(st, c, fnt, "; not an AAT or Graphite font")?;
            }
        }
        ConvertCode::XetexGlyphName => {
            fnt = crate::scan::scan_font_ident(st)?;
            let fa = st.fonts.font_area[fnt];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                scan_int(st)?;
                arg1 = st.scanner.cur_val;
            } else {
                not_font_error(st, c, fnt, "; not a native platform font")?;
            }
        }
        ConvertCode::JobName if st.strings.job_name == StrNumber::NONE => {
            crate::io::open_log_file(st)?;
        }
        _ => {}
    }

    let old_selector = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    let b = st.strings.pool_ptr;

    match c {
        ConvertCode::Number | ConvertCode::Strcmp => print_int(st, st.scanner.cur_val),
        ConvertCode::RomanNumeral => crate::output::print_roman_int(st, st.scanner.cur_val),
        ConvertCode::String => {
            if st.scanner.cur_cs != 0 {
                sprint_cs(st, st.scanner.cur_cs);
            } else {
                print_char(st, st.scanner.cur_chr);
            }
        }
        ConvertCode::Meaning => crate::output::print_meaning(st),
        ConvertCode::FontName => crate::output::print_font_name_and_size(st, fnt),
        ConvertCode::EtexRevision => print(st, ".6"),
        ConvertCode::LeftMarginKern | ConvertCode::RightMarginKern => {
            let list = margin_list;
            if c == ConvertCode::LeftMarginKern {
                print_margin_kern(st, list, 0, GluePar::LeftSkip, |mem, p| p.link(mem));
            } else {
                let last = st.mem.prev_rightmost(list, MemPtr::NULL);
                print_margin_kern(st, last, 1, GluePar::RightSkip, move |mem, p| {
                    mem.prev_rightmost(list, p)
                });
            }
        }
        ConvertCode::UniformDeviate => {
            let v = st.errors.unif_rand(st.scanner.cur_val);
            print_int(st, v);
        }
        ConvertCode::NormalDeviate => {
            let v = crate::scaled_math::norm_rand(st);
            print_int(st, v);
        }
        ConvertCode::Uchar | ConvertCode::Ucharcat => print_char(st, st.scanner.cur_val),
        ConvertCode::XetexRevision => print(st, ".999994"),
        // AAT fonts have no names to print on this platform.
        ConvertCode::XetexVariationName => {}
        ConvertCode::XetexFeatureName | ConvertCode::XetexSelectorName => {
            let engine = st.fonts.shared_layout_engine(fnt);
            if let Some(engine) = engine.as_deref().filter(|e| e.using_graphite()) {
                crate::ext::gr_print_font_name(st, c, engine, arg1, arg2);
            }
        }
        ConvertCode::XetexGlyphName => {
            let fa = st.fonts.font_area[fnt];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                crate::ext::print_glyph_name(st, fnt, arg1)?;
            }
        }
        ConvertCode::JobName => {
            let job_name = st.strings.job_name;
            crate::output::print_file_name(st, job_name, StrNumber::NONE, StrNumber::NONE);
        }
        _ => {}
    }

    st.shipout.selector = old_selector;
    let tail = st.mem.str_toks_cat(&mut st.strings, b, cat as i16)?;
    ins_temp_head_list(st, tail)
}

fn is_aat_or_graphite(st: &EngineState, f: FontId) -> bool {
    let fa = st.fonts.font_area[f];
    fa == AAT_FONT_FLAG
        || st
            .fonts
            .layout_engine(f)
            .is_some_and(|e| e.using_graphite())
}

fn not_font_error(st: &mut EngineState, c: ConvertCode, f: FontId, msg: &str) -> Result<(), Abort> {
    crate::errors::not_font_error_common(st, CONVERT, c.raw(), f, msg)
}

/// Puts `token` in front of the backed-up token list on top of the input stack.
fn prepend_to_backed_up(st: &mut EngineState, token: Token) -> Result<(), Abort> {
    let p = st.mem.get_avail()?;
    p.set_token(&mut st.mem, token);
    p.set_link(&mut st.mem, st.input.cur_input.token_loc());
    st.input.cur_input.set_token_list(p);
    Ok(())
}

pub(crate) fn expand(st: &mut EngineState) -> Result<(), Abort> {
    st.scanner.expand_depth_count += 1;
    if st.scanner.expand_depth_count >= st.scanner.expand_depth {
        return Err(Abort::Overflow {
            what: "expansion depth",
            n: st.scanner.expand_depth,
        });
    }

    let cv_backup = st.scanner.cur_val;
    let cvl_backup = st.scanner.cur_val_level;
    let radix_backup = st.scanner.radix;
    let co_backup = st.scanner.cur_order;
    let backup_backup = BACKUP_HEAD.link(&st.mem);

    'reswitch: loop {
        if st.scanner.cur_cmd >= CALL {
            if st.scanner.cur_cmd < END_TEMPLATE {
                macro_call(st)?;
            } else {
                st.scanner.cur_tok = Token::from_cs(FROZEN_ENDV);
                back_input(st)?;
            }
            break;
        }
        if st.eqtb.int_par(IntPar::TracingCommands) > 1 {
            show_cur_cmd_chr(st);
        }
        match st.scanner.cur_cmd {
            TOP_BOT_MARK => {
                let mark = MarkCode::from_raw(st.scanner.cur_chr % MarkCode::MARKS)
                    .ok_or(Abort::Confusion("top_bot_mark"))?;
                if st.scanner.cur_chr >= MarkCode::MARKS {
                    crate::scan::scan_register_num(st)?;
                } else {
                    st.scanner.cur_val = 0;
                }
                if st.scanner.cur_val == 0 {
                    st.scanner.cur_ptr = st.scanner.cur_mark[mark];
                } else {
                    let n = st.scanner.cur_val;
                    if let Some(sa) = st.mem.find_sa_element(&mut st.scanner, ValueLevel::Mark, n) {
                        st.scanner.cur_ptr = sa.mark(&st.mem, mark);
                    }
                }
                if !st.scanner.cur_ptr.is_null() {
                    begin_token_list(st, st.scanner.cur_ptr, TokenType::Mark)?;
                }
            }
            EXPAND_AFTER => {
                if st.scanner.cur_chr == 0 {
                    get_token(st)?;
                    let t = st.scanner.cur_tok;
                    get_token(st)?;
                    if st.scanner.cur_cmd > MAX_COMMAND {
                        expand(st)?;
                    } else {
                        back_input(st)?;
                    }
                    st.scanner.cur_tok = t;
                    back_input(st)?;
                } else {
                    get_token(st)?;
                    if st.scanner.cur_cmd == IF_TEST && st.scanner.cur_chr != IfCode::Case.raw() {
                        st.scanner.cur_chr += IfCode::UNLESS;
                        continue 'reswitch;
                    }
                    error_here_with_diagnostic(st, "You can't use `");
                    print_esc(st, "unless");
                    print(st, "' before `");
                    print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
                    print_char(st, i32::from(b'\''));
                    capture_to_diagnostic(st, None);
                    st.errors
                        .help(&["Continue, and I'll forget that it ever happened."]);
                    back_error(st)?;
                }
            }
            NO_EXPAND => {
                if st.scanner.cur_chr == 0 {
                    get_token_unexpanded(st)?;
                    let t = st.scanner.cur_tok;
                    back_input(st)?;
                    if t.is_cs() {
                        prepend_to_backed_up(st, Token::from_cs(FROZEN_DONT_EXPAND))?;
                    }
                } else {
                    get_token_unexpanded(st)?;
                    st.scanner.cur_cs =
                        st.eqtb.prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)?;
                    if st.scanner.cur_cs != UNDEFINED_PRIMITIVE {
                        let cmd = st.eqtb.eq_type(PRIM_EQTB_BASE + st.scanner.cur_cs);
                        if cmd > MAX_COMMAND {
                            st.scanner.cur_cmd = cmd;
                            st.scanner.cur_chr = st.eqtb.equiv(PRIM_EQTB_BASE + st.scanner.cur_cs);
                            st.scanner.cur_tok = Token::new(st.scanner.cur_cmd, st.scanner.cur_chr);
                            st.scanner.cur_cs = 0;
                            continue 'reswitch;
                        }
                        back_input(st)?;
                        prepend_to_backed_up(st, Token::from_cs(FROZEN_PRIMITIVE))?;
                    }
                }
            }
            CS_NAME => {
                let (head, tail) = scan_cs_name_tokens(st)?;
                let first = st.input.first;
                let j = token_chars_to_buffer(st, head.link(&st.mem))?;
                st.scanner.cur_cs = if j > first + 1 || st.input.buffer[first as usize] > 65535 {
                    st.eqtb.no_new_control_sequence = false;
                    let cs =
                        st.eqtb
                            .id_lookup(&mut st.strings, &st.input.buffer, first, j - first)?;
                    st.eqtb.no_new_control_sequence = true;
                    cs
                } else if j == first {
                    NULL_CS
                } else {
                    SINGLE_BASE + st.input.buffer[first as usize]
                };
                st.mem.flush_list_with_tail(head, tail);
                let cs = st.scanner.cur_cs;
                if st.eqtb.eq_type(cs) == UNDEFINED_CS {
                    let _ = st.eqtb.eq_define(
                        &mut st.save_stack,
                        &mut st.mem,
                        &mut st.scanner,
                        cs,
                        RELAX,
                        TOO_BIG_USV,
                    );
                }
                st.scanner.cur_tok = Token::from_cs(st.scanner.cur_cs);
                back_input(st)?;
            }
            CONVERT => conv_toks(st)?,
            THE => ins_the_toks(st)?,
            IF_TEST => conditional(st)?,
            FI_OR_ELSE => {
                if st.eqtb.int_par(IntPar::TracingIfs) > 0
                    && st.eqtb.int_par(IntPar::TracingCommands) <= 1
                {
                    show_cur_cmd_chr(st);
                }
                if st.scanner.cur_chr > st.scanner.if_limit.raw() {
                    if st.scanner.if_limit == FiOrElseCode::If {
                        crate::input::insert_relax(st)?;
                    } else {
                        error_here_with_diagnostic(st, "Extra ");
                        print_cmd_chr(st, FI_OR_ELSE, st.scanner.cur_chr);
                        capture_to_diagnostic(st, None);
                        st.errors
                            .help(&[r"I'm ignoring this; it doesn't match any \if."]);
                        error(st)?;
                    }
                } else {
                    while st.scanner.cur_chr != FiOrElseCode::Fi.raw() {
                        crate::scan::pass_text(st)?;
                    }
                    pop_fi(st);
                }
            }
            INPUT => match InputCode::from_raw(st.scanner.cur_chr) {
                Some(InputCode::EndInput) => st.input.force_eof = true,
                Some(InputCode::ScanTokens) => crate::scan::pseudo_start(st)?,
                _ if st.scanner.name_in_progress => crate::input::insert_relax(st)?,
                _ => crate::io::start_input(st, None)?,
            },
            _ => {
                print_err(st, "Undefined control sequence");
                st.errors.help(&[
                    "The control sequence at the end of the top line",
                    r"of your error message was never \def'ed. If you have",
                    r"misspelled it (e.g., `\hobx'), type `I' and the correct",
                    r"spelling (e.g., `I\hbox'). Otherwise just continue,",
                    "and I'll forget about whatever was undefined.",
                ]);
                error(st)?;
            }
        }
        break;
    }

    st.scanner.cur_val = cv_backup;
    st.scanner.cur_val_level = cvl_backup;
    st.scanner.radix = radix_backup;
    st.scanner.cur_order = co_backup;
    BACKUP_HEAD.set_link(&mut st.mem, backup_backup);
    st.scanner.expand_depth_count -= 1;
    Ok(())
}

/// Reports a `\par` inside a non-`\long` macro argument and discards the arguments so far.
#[cold]
fn runaway_par_in_argument(
    st: &mut EngineState,
    pstack: &mut [MemPtr; 9],
    n: i32,
    unbalance: i32,
) -> Result<(), Abort> {
    if st.scanner.long_state == CALL {
        crate::errors::runaway(st);
        error_here_with_diagnostic(st, "Paragraph ended before ");
        sprint_cs(st, st.scanner.warning_index);
        print(st, " was complete");
        capture_to_diagnostic(st, None);
        st.errors.help(&[
            "I suspect you've forgotten a `}', causing me to apply this",
            "control sequence to too much text. How can we recover?",
            "My plan is to forget the whole thing and hope for the best.",
        ]);
        back_error(st)?;
    }
    pstack[n as usize] = TEMP_HEAD.link(&st.mem);
    st.scanner.align_state -= unbalance;
    for &list in &pstack[..=n as usize] {
        st.mem.flush_list(list);
    }
    Ok(())
}

#[cold]
fn extra_right_brace_in_argument(st: &mut EngineState) -> Result<(), Abort> {
    back_input(st)?;
    error_here_with_diagnostic(st, "Argument of ");
    sprint_cs(st, st.scanner.warning_index);
    print(st, " has an extra }");
    capture_to_diagnostic(st, None);
    st.errors.help(&[
        "I've run across a `}' that doesn't seem to match anything.",
        r"For example, `\def\a#1{...}' and `\a}' would produce",
        r"this error. If you simply proceed now, the `\par' that",
        "I've just inserted will cause me to report a runaway",
        "argument that might be the root of the problem. But if",
        "your `}' was spurious, just type `2' and it will go away.",
    ]);
    st.scanner.align_state += 1;
    st.scanner.long_state = CALL;
    st.scanner.cur_tok = st.input.par_token;
    crate::input::ins_error(st)
}

#[cold]
fn trace_macro_call(st: &mut EngineState, ref_count: MemPtr) {
    begin_diagnostic(st);
    diagnostic_begin_capture_warning_here(st);
    let stack_levels = st.eqtb.int_par(IntPar::TracingStackLevels);
    if stack_levels > 0 {
        if st.input.input_ptr < stack_levels {
            crate::output::print_ln(st);
            print_char(st, b'~');
            for _ in 0..st.input.input_ptr {
                print_char(st, b'.');
            }
            crate::output::print_cs(st, st.scanner.warning_index);
            crate::output::token_show(st, ref_count);
        } else {
            print_char(st, b'~');
            print_char(st, b'~');
            crate::output::print_cs(st, st.scanner.warning_index);
        }
    } else {
        crate::output::print_ln(st);
        crate::output::print_cs(st, st.scanner.warning_index);
        crate::output::token_show(st, ref_count);
    }
    capture_to_diagnostic(st, None);
    end_diagnostic(st, false);
}

#[cold]
fn trace_macro_param(st: &mut EngineState, match_chr: i32, n: i32, list: MemPtr) {
    let stack_levels = st.eqtb.int_par(IntPar::TracingStackLevels);
    if stack_levels == 0 || st.input.input_ptr < stack_levels {
        begin_diagnostic(st);
        diagnostic_begin_capture_warning_here(st);
        crate::output::print_nl(st, match_chr);
        print_int(st, n);
        print(st, "<-");
        crate::output::show_token_list(st, list, TEX_NULL, 1000);
        capture_to_diagnostic(st, None);
        end_diagnostic(st, false);
    }
}

pub(crate) fn macro_call(st: &mut EngineState) -> Result<(), Abort> {
    if st.config.pgfmath == crate::pgfmath::Mode::Check
        || st.config.pgfmath == crate::pgfmath::Mode::Native
            && st.eqtb.pgfmath.is_try(st.scanner.cur_cs)
    {
        if let Some(cs) = crate::pgfmath::intercept(st) {
            st.scanner.cur_cs = cs;
            st.scanner.cur_cmd = st.eqtb.eq_type(cs);
            st.scanner.cur_chr = st.eqtb.equiv(cs);
        }
    }
    let save_scanner_status = st.scanner.scanner_status;
    let save_warning_index = st.scanner.warning_index;
    st.scanner.warning_index = st.scanner.cur_cs;
    let ref_count = st.scanner.cur_chr_ptr();
    let mut r = ref_count.link(&st.mem);
    let mut n: i32 = 0;

    if st.eqtb.int_par(IntPar::TracingMacros) > 0 {
        trace_macro_call(st, ref_count);
    }

    if st.mem.is_protected_macro(ref_count) {
        r = r.link(&st.mem);
    }

    let mut pstack = [MemPtr::NULL; 9];
    let mut ptail = [MemPtr::NULL; 9];
    let mut match_chr: i32 = 0;
    let mut rbrace_ptr = MemPtr::NULL;

    'exit: {
        if r.token(&st.mem) != Token::END_MATCH {
            st.scanner.scanner_status = ScannerStatus::Matching;
            let mut unbalance: i32 = 0;
            st.scanner.long_state = st.eqtb.eq_type(st.scanner.cur_cs);
            if st.scanner.long_state >= OUTER_CALL {
                st.scanner.long_state -= 2;
            }

            loop {
                TEMP_HEAD.set_link(&mut st.mem, MemPtr::NULL);
                let s: MemPtr;
                let mut p = MemPtr::NULL;
                let mut m: i32 = 0;
                let rt = r.token(&st.mem);
                if rt >= Token::END_MATCH || rt < Token::match_(0) {
                    s = MemPtr::NULL;
                } else {
                    match_chr = rt.chr();
                    s = r.link(&st.mem);
                    r = s;
                    p = TEMP_HEAD;
                }

                'continue_: loop {
                    if s == r && !s.is_null() && !r.token(&st.mem).is_match() {
                        copy_delimited_tokens(st, &mut p, &mut m, r.token(&st.mem))?;
                    }
                    get_token(st)?;

                    if st.scanner.cur_tok == r.token(&st.mem) {
                        r = r.link(&st.mem);
                        if r.token(&st.mem).is_match() {
                            if st.scanner.cur_tok.is_left_brace() {
                                st.scanner.align_state -= 1;
                            }
                            break 'continue_;
                        }
                        continue 'continue_;
                    }

                    if s != r {
                        if s.is_null() {
                            error_here_with_diagnostic(st, "Use of ");
                            sprint_cs(st, st.scanner.warning_index);
                            print(st, " doesn't match its definition");
                            capture_to_diagnostic(st, None);
                            st.errors.help(&[
                                r"If you say, e.g., `\def\a1{...}', then you must always",
                                r"put `1' after `\a', since control sequence names are",
                                "made up of letters only. The macro here has not been",
                                "followed by the required stuff, so I'm ignoring it.",
                            ]);
                            error(st)?;
                            break 'exit;
                        }
                        // Contribute the partly matched delimiter to the argument.
                        let mut t = s;
                        loop {
                            let tok = t.token(&st.mem);
                            store_new_token(st, &mut p, tok)?;
                            m += 1;
                            let mut u = t.link(&st.mem);
                            let mut v = s;
                            loop {
                                if u == r {
                                    if st.scanner.cur_tok != v.token(&st.mem) {
                                        break;
                                    }
                                    r = v.link(&st.mem);
                                    continue 'continue_;
                                }
                                if u.token(&st.mem) != v.token(&st.mem) {
                                    break;
                                }
                                u = u.link(&st.mem);
                                v = v.link(&st.mem);
                            }
                            t = t.link(&st.mem);
                            if t == r {
                                break;
                            }
                        }
                        r = s;
                    }

                    if st.scanner.cur_tok == st.input.par_token
                        && st.scanner.long_state != LONG_CALL
                    {
                        runaway_par_in_argument(st, &mut pstack, n, unbalance)?;
                        break 'exit;
                    }

                    if st.scanner.cur_tok.is_brace() {
                        if !st.scanner.cur_tok.is_left_brace() {
                            extra_right_brace_in_argument(st)?;
                            continue 'continue_;
                        }
                        unbalance = 1;
                        loop {
                            store_new_token(st, &mut p, st.scanner.cur_tok)?;
                            if copy_group_tokens(st, &mut p, &mut unbalance, false)? {
                                break;
                            }

                            get_token(st)?;

                            if st.scanner.cur_tok == st.input.par_token
                                && st.scanner.long_state != LONG_CALL
                            {
                                runaway_par_in_argument(st, &mut pstack, n, unbalance)?;
                                break 'exit;
                            }

                            if st.scanner.cur_tok.is_brace() {
                                if st.scanner.cur_tok.is_left_brace() {
                                    unbalance += 1;
                                } else {
                                    unbalance -= 1;
                                    if unbalance == 0 {
                                        break;
                                    }
                                }
                            }
                        }
                        rbrace_ptr = p;
                        store_new_token(st, &mut p, st.scanner.cur_tok)?;
                    } else {
                        if st.scanner.cur_tok == Token::SPACE && r.token(&st.mem).is_match() {
                            continue 'continue_;
                        }
                        store_new_token(st, &mut p, st.scanner.cur_tok)?;
                    }

                    m += 1;
                    if !r.token(&st.mem).is_match() {
                        continue 'continue_;
                    }
                    break 'continue_;
                }

                if !s.is_null() {
                    if m == 1 && p.token(&st.mem).is_brace() {
                        // Strip the braces around a single braced argument.
                        rbrace_ptr.set_link(&mut st.mem, MemPtr::NULL);
                        st.mem.free_avail(p);
                        p = TEMP_HEAD.link(&st.mem);
                        pstack[n as usize] = p.link(&st.mem);
                        ptail[n as usize] = rbrace_ptr;
                        st.mem.free_avail(p);
                    } else {
                        pstack[n as usize] = TEMP_HEAD.link(&st.mem);
                        ptail[n as usize] = p;
                    }
                    n += 1;
                    if st.eqtb.int_par(IntPar::TracingMacros) > 0 {
                        trace_macro_param(st, match_chr, n, pstack[(n - 1) as usize]);
                    }
                }

                if r.token(&st.mem) == Token::END_MATCH {
                    break;
                }
            }
        }

        if n > 0 && st.config.pgfmath == crate::pgfmath::Mode::Native {
            if let Some(kind) = st.eqtb.pgfmath.native(st.scanner.warning_index) {
                if crate::pgfmath::native_macro(st, kind, &pstack[..n as usize]) {
                    for k in 0..n as usize {
                        st.mem.flush_list_with_tail(pstack[k], ptail[k]);
                    }
                    break 'exit;
                }
            }
        }

        while st.input.cur_input.state == TOKEN_LIST
            && st.input.cur_input.loc == TEX_NULL
            && st.input.cur_input.token_type() != TokenType::VTemplate
        {
            crate::input::end_token_list(st)?;
        }

        begin_token_list(st, ref_count, TokenType::Macro)?;
        st.input.cur_input.name = st.scanner.warning_index;
        st.input.cur_input.set_token_loc(r.link(&st.mem));

        if n > 0 {
            if st.input.param_ptr + n > st.input.max_param_stack {
                st.input.max_param_stack = st.input.param_ptr + n;
                if st.input.max_param_stack > st.input.param_size {
                    return Err(Abort::Overflow {
                        what: "parameter stack size",
                        n: st.input.param_size,
                    });
                }
            }
            let base = st.input.param_ptr as usize;
            st.input.param_stack[base..base + n as usize].copy_from_slice(&pstack[..n as usize]);
            st.input.param_tail[base..base + n as usize].copy_from_slice(&ptail[..n as usize]);
            st.input.param_ptr += n;
        }
    }

    st.scanner.scanner_status = save_scanner_status;
    st.scanner.warning_index = save_warning_index;
    Ok(())
}
