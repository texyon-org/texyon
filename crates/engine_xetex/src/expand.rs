//! Expansion: `expand`, `macro_call`, conditionals and `conv_toks`.

use crate::cmd::Cmd;
use crate::codes::{
    ConvertCode, FiOrElseCode, IfCode, InputCode, ScannerStatus, TheCode, ValueLevel,
};
use crate::command::Command;
use crate::consts::{
    ACTIVE_BASE, BIGGEST_CHAR, BIGGEST_USV, END_WRITE, FROZEN_DONT_EXPAND, FROZEN_ENDV,
    FROZEN_PRIMITIVE, HMODE, MMODE, NULL_CS, NULL_FONT, SINGLE_BASE, TOO_BIG_USV, VMODE,
};
use crate::errors::Abort;
use crate::errors::OrConfusion;
use crate::fonts::{Side, AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::input::{back_input, begin_token_list, TokenType, TOKEN_LIST};
use crate::mem::{BACKUP_HEAD, GARBAGE, TEMP_HEAD_TOKS, ZERO_GLUE};
use crate::node::{link_of, AnyNode, CondNode, GlueKind, KernSubtype, NodePtr, SaLeaf};
use crate::output::{
    begin_diagnostic, end_diagnostic, print, print_char, print_int, print_scaled, show_cur_cmd_chr,
    sprint_cs,
};
use crate::params::{GluePar, IntPar};
use crate::scan::{
    copy_delimited_tokens, copy_group_tokens, fetch_box, get_next, get_token, get_x_non_blank,
    get_x_token, scan_int, store_new_token,
};
use crate::state::{EngineState, InputName, MacroParam, MemState, Selector};
use crate::token::{Token, TokenList};
use crate::units::{BufPos, EqtbPtr, FontId, MemPtr, Scaled, StrNumber};

fn ins_the_toks(st: &mut EngineState, code: TheCode) -> Result<(), Abort> {
    let tail = crate::scan::the_toks(st, code)?;
    ins_temp_head_list(st, tail)
}

/// Inserts the list at `link(temp_head)` whose tail is `tail`.
fn ins_temp_head_list(st: &mut EngineState, tail: TokenList) -> Result<(), Abort> {
    GARBAGE.set_link(&mut st.mem, tail.ptr());
    begin_token_list(st, TEMP_HEAD_TOKS.link(&st.mem), TokenType::Inserted)?;
    Ok(())
}

fn pop_cond_node(st: &mut EngineState) -> Result<(), Abort> {
    let p = st.scanner.cond_ptr.or_confusion("if")?;
    (st.scanner.cur_if, st.scanner.if_line, st.scanner.if_limit) = st.mem[p].saved_state();
    st.scanner.cond_ptr = p.next(&st.mem);
    p.free(&mut st.mem);
    Ok(())
}

fn pop_fi(st: &mut EngineState) -> Result<(), Abort> {
    if st.input.open_levels[st.input.in_open].cond == st.scanner.cond_ptr {
        crate::output::if_warning(st);
    }
    pop_cond_node(st)
}

/// Handles the `\else` or `\fi` that ended a skipped conditional branch.
fn skipped_branch_end(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.fi_or_else() == Some(FiOrElseCode::Fi) {
        pop_fi(st)?;
    } else {
        st.scanner.if_limit = FiOrElseCode::Fi.into();
    }
    Ok(())
}

fn get_x_token_or_active_char(st: &mut EngineState) -> Result<(), Abort> {
    get_x_token(st)?;
    if st.scanner.try_command() == Some(Command::Relax { no_expand: true }) {
        st.scanner.cur_cmd = Cmd::ACTIVE_CHAR;
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
fn token_chars_to_buffer(st: &mut EngineState, list: TokenList) -> Result<BufPos, Abort> {
    let mut j = st.input.first;
    let mut p = list;
    while !p.is_null() {
        if j >= st.input.max_buf_stack {
            st.input.max_buf_stack = j + 1;
            if st.input.max_buf_stack == st.input.buf_size {
                return Err(st.input.buffer_overflow());
            }
        }
        st.input.buffer[j] = p.token(&st.mem).chr();
        j += 1;
        p = p.link(&st.mem);
    }
    Ok(j)
}

/// Scans the tokens of a `\csname`-like name up to the terminating control sequence.
/// Returns the head node and the last node of the scanned list.
fn scan_cs_name_tokens(st: &mut EngineState) -> Result<(TokenList, TokenList), Abort> {
    let head = st.mem.get_avail_token()?;
    let mut tail = head;
    let saved_in_csname = st.scanner.is_in_csname;
    st.scanner.is_in_csname = true;
    loop {
        get_x_token(st)?;
        if !st.scanner.cur_cs.is_none() {
            break;
        }
        store_new_token(st, &mut tail, st.scanner.cur_tok)?;
    }
    if st.scanner.cur_cmd != Cmd::EndCsName {
        crate::errors::back_error(
            st,
            ("Missing ", crate::output::Esc("endcsname"), " inserted"),
            &[
                "The control sequence marked <to be read again> should",
                r"not appear between \csname and \endcsname.",
            ],
        )?;
    }
    st.scanner.is_in_csname = saved_in_csname;
    Ok((head, tail))
}

fn trace_diagnostic(st: &mut EngineState, text: &str) {
    begin_diagnostic(st);
    print(st, text);
    end_diagnostic(st, false);
}

fn change_if_limit(
    st: &mut EngineState,
    l: FiOrElseCode,
    p: Option<NodePtr<CondNode>>,
) -> Result<(), Abort> {
    if p == st.scanner.cond_ptr {
        st.scanner.if_limit = l.into();
        return Ok(());
    }
    let mut q = st.scanner.cond_ptr;
    while let Some(cond) = q {
        let next = cond.next(&st.mem);
        if next == p {
            st.mem[cond].if_limit = l.into();
            return Ok(());
        }
        q = next;
    }
    Err(Abort::Confusion("if"))
}

fn conditional(st: &mut EngineState, this_if: IfCode, is_unless: bool) -> Result<(), Abort> {
    if st.eqtb.int_par(IntPar::TracingIfs) > 0 && st.eqtb.int_par(IntPar::TracingCommands) <= 1 {
        show_cur_cmd_chr(st);
    }

    let cond_node = NodePtr::<CondNode>::alloc(&mut st.mem)?;
    let node = &mut st.mem[cond_node];
    node.link = link_of(st.scanner.cond_ptr);
    node.if_limit = st.scanner.if_limit;
    node.cur_if = st.scanner.cur_if as u16;
    node.if_line = st.scanner.if_line;
    st.scanner.cond_ptr = Some(cond_node);
    st.scanner.cur_if = (this_if.raw() + if is_unless { IfCode::UNLESS } else { 0 }) as i16;
    st.scanner.if_limit = FiOrElseCode::If.into();
    st.scanner.if_line = st.mem.line;

    let save_cond_ptr = st.scanner.cond_ptr;

    let mut b = match this_if {
        IfCode::Char | IfCode::Cat => {
            get_x_token_or_active_char(st)?;
            let (m, n) =
                if st.scanner.cur_cmd > Cmd::ACTIVE_CHAR || st.scanner.cur_chr > BIGGEST_USV {
                    (Cmd::Relax, TOO_BIG_USV)
                } else {
                    (st.scanner.cur_cmd, st.scanner.cur_chr)
                };
            get_x_token_or_active_char(st)?;
            if st.scanner.cur_cmd > Cmd::ACTIVE_CHAR || st.scanner.cur_chr > BIGGEST_USV {
                st.scanner.cur_cmd = Cmd::Relax;
                st.scanner.cur_chr = TOO_BIG_USV;
            }
            if this_if == IfCode::Char {
                n == st.scanner.cur_chr
            } else {
                m == st.scanner.cur_cmd
            }
        }
        IfCode::Num | IfCode::Dim => {
            let n = scan_int_or_dimen(st, this_if)?;
            get_x_non_blank(st)?;
            let relation =
                if (Token::other(b'<')..=Token::other(b'>')).contains(&st.scanner.cur_tok) {
                    st.scanner.cur_tok.chr() as u8
                } else {
                    crate::errors::back_error(
                        st,
                        (
                            "Missing = inserted for ",
                            crate::output::CmdChr(Cmd::IfTest, this_if.raw()),
                        ),
                        &["I was expecting to see `<', `=', or `>'. Didn't."],
                    )?;
                    b'='
                };
            let cv = scan_int_or_dimen(st, this_if)?;
            match relation {
                b'<' => n < cv,
                b'=' => n == cv,
                _ => n > cv,
            }
        }
        IfCode::Odd => scan_int(st)? % 2 != 0,
        IfCode::VMode => st.input.cur_list.mode.abs() == VMODE,
        IfCode::HMode => st.input.cur_list.mode.abs() == HMODE,
        IfCode::MMode => st.input.cur_list.mode.abs() == MMODE,
        IfCode::Inner => st.input.cur_list.mode < 0,
        IfCode::Void | IfCode::HBox | IfCode::VBox => {
            let n = crate::scan::scan_register_num(st)?;
            let contents = fetch_box(st, n);
            match this_if {
                IfCode::Void => contents.is_none(),
                IfCode::HBox => contents.is_some_and(|b| st.mem[b].is_hlist()),
                _ => contents.is_some_and(|b| st.mem[b].is_vlist()),
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
            } else if st.scanner.cur_cmd < Cmd::Call {
                st.scanner.cur_chr == q
            } else {
                let mut p = st.scanner.cur_chr_tokens().link(&st.mem);
                let mut q = st.eqtb.equiv_tokens(n).link(&st.mem);
                if p == q {
                    true
                } else {
                    while !p.is_null() && !q.is_null() {
                        if p.token(&st.mem) != q.token(&st.mem) {
                            p = TokenList::NULL;
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
            let n = crate::scan::scan_four_bit_int_or_18(st)?;
            if n == 18 {
                !st.config.shell_escape_enabled
            } else {
                st.input.read_open[n as usize] == crate::consts::CLOSED
            }
        }
        IfCode::True => true,
        IfCode::False => false,
        IfCode::Defined => {
            get_next_unexpanded(st)?;
            st.scanner.cur_cmd != Cmd::UndefinedCs
        }
        IfCode::CsName => {
            let (head, tail) = scan_cs_name_tokens(st)?;
            let first = st.input.first;
            let m = token_chars_to_buffer(st, head.link(&st.mem))?;
            // Upstream keeps `single_base+c` here for c above U+FFFF and reads past `null_cs`.
            st.scanner.cur_cs = if m == first {
                NULL_CS
            } else if m == first + 1 && st.input.buffer[first] <= BIGGEST_CHAR {
                SINGLE_BASE + st.input.buffer[first]
            } else {
                st.eqtb
                    .id_lookup(&mut st.strings, st.input.buffer.span(first, m))?
            };
            st.mem.flush_token_list_with_tail(head, tail);
            st.eqtb.eq_type(st.scanner.cur_cs) != Cmd::UndefinedCs
        }
        IfCode::InCsName => st.scanner.is_in_csname,
        IfCode::FontChar => {
            let f = crate::scan::scan_font_ident(st)?;
            let c = crate::scan::scan_usv_num(st)?;
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
            let mut n = scan_int(st)?;
            if st.eqtb.int_par(IntPar::TracingCommands) > 1 {
                begin_diagnostic(st);
                print(st, "{case ");
                print_int(st, n);
                print_char(st, b'}');
                end_diagnostic(st, false);
            }
            while n != 0 {
                crate::scan::pass_text(st)?;
                if st.scanner.cond_ptr == save_cond_ptr {
                    if st.scanner.fi_or_else() != Some(FiOrElseCode::Or) {
                        skipped_branch_end(st)?;
                        return Ok(());
                    }
                    n = n.wrapping_sub(1);
                } else if st.scanner.fi_or_else() == Some(FiOrElseCode::Fi) {
                    pop_fi(st)?;
                }
            }
            change_if_limit(st, FiOrElseCode::Or, save_cond_ptr)?;
            return Ok(());
        }
        IfCode::Primitive => {
            get_next_unexpanded(st)?;
            let m = st.eqtb.prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)?;
            st.scanner.cur_cmd != Cmd::UndefinedCs
                && m.is_some_and(|m| {
                    st.scanner.cur_cmd == st.eqtb.eq_type(m.eqtb_loc())
                        && st.scanner.cur_chr == st.eqtb.equiv(m.eqtb_loc())
                })
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
        if st.scanner.cond_ptr == save_cond_ptr {
            if st.scanner.fi_or_else() != Some(FiOrElseCode::Or) {
                skipped_branch_end(st)?;
                return Ok(());
            }
            crate::errors::error(
                st,
                ("Extra ", crate::output::Esc("or")),
                &[r"I'm ignoring this; it doesn't match any \if."],
            )?;
        } else if st.scanner.fi_or_else() == Some(FiOrElseCode::Fi) {
            pop_fi(st)?;
        }
    }
}

/// An `\ifnum` or `\ifdim` operand, a dimension compared as its raw value.
fn scan_int_or_dimen(st: &mut EngineState, this_if: IfCode) -> Result<i32, Abort> {
    if this_if == IfCode::Num {
        scan_int(st)
    } else {
        Ok(crate::scan::scan_dimen(st, false, false)?.raw())
    }
}

/// Scanner state saved around a `conv_toks` primitive that reads a general text.
struct SavedScan {
    status: ScannerStatus,
    warning_index: EqtbPtr,
    def_ref: TokenList,
    pending_str: bool,
}

/// Saves the scanner state and seals a partially built string so the pool can be reused.
fn save_scan(st: &mut EngineState) -> Result<SavedScan, Abort> {
    let saved = SavedScan {
        status: st.scanner.scanner_status,
        warning_index: st.scanner.warning_index,
        def_ref: st.scanner.def_ref,
        pending_str: st.strings.cur_string_start() < st.strings.pool_ptr,
    };
    if saved.pending_str {
        st.strings.make_string()?;
    }
    Ok(saved)
}

fn restore_scan(st: &mut EngineState, saved: &SavedScan) {
    st.scanner.def_ref = saved.def_ref;
    st.scanner.warning_index = saved.warning_index;
    st.scanner.scanner_status = saved.status;
}

fn unseal_pending_str(st: &mut EngineState, saved: &SavedScan) {
    if saved.pending_str {
        st.strings.str_ptr = st.strings.str_ptr.prev();
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
    crate::output::show_token_list(st, text, TokenList::NULL, limit);
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
    b: crate::units::PoolPos,
) -> Result<(), Abort> {
    let tail = st.mem.str_toks_cat(&mut st.strings, b, None)?;
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
    if !crate::scan::scan_keyword(st, keyword)? {
        return Ok(0);
    }
    let n = scan_int(st)?;
    if n < 0 {
        crate::errors::error(
            st,
            (err, " (", n, ")"),
            &[help, "I changed this one to zero."],
        )?;
        return Ok(0);
    }
    Ok(n)
}

impl MemState {
    /// Whether a node may be skipped when looking for a margin kern at the edge of a line.
    fn is_margin_kern_filler(&self, node: AnyNode, par_skip: GluePar) -> bool {
        match node {
            AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_) | AnyNode::Penalty(_) => true,
            AnyNode::Disc(d) => {
                self[d].pre_break.is_null()
                    && self[d].post_break.is_null()
                    && self[d].replace_count == 0
            }
            AnyNode::Math(m) => self[m].width == Scaled::ZERO,
            AnyNode::Kern(k) => {
                self[k].width == Scaled::ZERO || self[k].subtype == KernSubtype::Normal
            }
            AnyNode::Glue(g) => {
                self[g].glue_ptr == ZERO_GLUE || self[g].subtype == GlueKind::Param(par_skip)
            }
            AnyNode::Box(h) if self[h].is_hlist() => {
                self[h].width == Scaled::ZERO
                    && self[h].height == Scaled::ZERO
                    && self[h].depth == Scaled::ZERO
                    && self[h].list_ptr.is_null()
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
    side: Side,
    par_skip: GluePar,
    next: impl Fn(&crate::state::MemState, MemPtr) -> MemPtr,
) {
    let width = loop {
        if p.is_null() || st.mem.is_char_node(p) {
            break None;
        }
        match st.mem.classify(p) {
            Ok(node) if st.mem.is_margin_kern_filler(node, par_skip) => p = next(&st.mem, p),
            Ok(AnyNode::MarginKern(mk)) if st.mem[mk].side == side => break Some(st.mem[mk].width),
            _ => break None,
        }
    };
    match width {
        Some(w) => print_scaled(st, w),
        None => print_char(st, i32::from(b'0')),
    }
    print(st, "pt");
}

fn conv_toks(st: &mut EngineState, c: ConvertCode) -> Result<(), Abort> {
    let mut cat: Option<Cmd> = None;
    let mut fnt = NULL_FONT;
    let mut arg1: i32 = 0;
    let mut arg2: i32 = 0;
    let mut num: i32 = 0;
    let mut margin_list = MemPtr::NULL;

    match c {
        ConvertCode::Number | ConvertCode::RomanNumeral | ConvertCode::UniformDeviate => {
            num = scan_int(st)?;
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
            st.mem.free_avail(st.scanner.def_ref.ptr());
            st.scanner.def_ref = saved.def_ref;
            unseal_pending_str(st, &saved);
            return Ok(());
        }
        ConvertCode::LeftMarginKern | ConvertCode::RightMarginKern => {
            let n = crate::scan::scan_register_num(st)?;
            let Some(hbox) = fetch_box(st, n).filter(|&b| st.mem[b].is_hlist()) else {
                return Err(Abort::PdfError(&[
                    "marginkern",
                    "a non-empty hbox expected",
                ]));
            };
            margin_list = st.mem[hbox].list_ptr;
        }
        ConvertCode::CreationDate => {
            let b = st.strings.pool_ptr;
            st.texmf.creation_date(&mut st.strings);
            let tail = st.mem.str_toks_cat(&mut st.strings, b, None)?;
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
            num = crate::scan::compare_strings(st)?;
            restore_scan(st, &saved);
            unseal_pending_str(st, &saved);
        }
        ConvertCode::Uchar => num = crate::scan::scan_usv_num(st)?,
        ConvertCode::Ucharcat => {
            let saved_chr = crate::scan::scan_usv_num(st)?;
            let code = scan_int(st)?;
            let valid = Cmd::from_raw(code).filter(|c| {
                (Cmd::LeftBrace..=Cmd::ACTIVE_CHAR).contains(c)
                    && *c != Cmd::OUT_PARAM
                    && *c != Cmd::IGNORE
            });
            if let Some(valid) = valid {
                cat = Some(valid);
            } else {
                crate::errors::error(
                    st,
                    (
                        "Invalid code (",
                        code,
                        "), should be in the ranges 1..4, 6..8, 10..13",
                    ),
                    &["I'm going to use 12 instead of that illegal code value."],
                )?;
                cat = Some(Cmd::OtherChar);
            }
            num = saved_chr;
        }
        ConvertCode::XetexVariationName => {
            fnt = crate::scan::scan_font_ident(st)?;
            if st.fonts.font_area[fnt] == AAT_FONT_FLAG {
                arg1 = scan_int(st)?;
                arg2 = 0;
            } else {
                not_font_error(st, c, fnt, "; not an AAT font")?;
            }
        }
        ConvertCode::XetexFeatureName | ConvertCode::XetexSelectorName => {
            fnt = crate::scan::scan_font_ident(st)?;
            if is_aat_or_graphite(st, fnt) {
                arg1 = scan_int(st)?;
                if c == ConvertCode::XetexSelectorName {
                    arg2 = scan_int(st)?;
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
                arg1 = scan_int(st)?;
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
        ConvertCode::Number | ConvertCode::Strcmp => print_int(st, num),
        ConvertCode::RomanNumeral => crate::output::print_roman_int(st, num),
        ConvertCode::String => {
            if !st.scanner.cur_cs.is_none() {
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
                print_margin_kern(st, list, Side::Left, GluePar::LeftSkip, |mem, p| {
                    p.link(mem)
                });
            } else {
                let last = st.mem.prev_rightmost(list, MemPtr::NULL);
                print_margin_kern(st, last, Side::Right, GluePar::RightSkip, move |mem, p| {
                    mem.prev_rightmost(list, p)
                });
            }
        }
        ConvertCode::UniformDeviate => {
            let v = st.random.unif_rand(num);
            print_int(st, v);
        }
        ConvertCode::NormalDeviate => {
            let v = st.random.norm_rand();
            print_int(st, v);
        }
        ConvertCode::Uchar | ConvertCode::Ucharcat => print_char(st, num),
        ConvertCode::XetexRevision => print(st, ".999997"),
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
    let tail = st.mem.str_toks_cat(&mut st.strings, b, cat)?;
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
    crate::errors::not_font_error_common(st, Cmd::Convert, c.raw(), f, msg)
}

/// Puts `token` in front of the backed-up token list on top of the input stack.
fn prepend_to_backed_up(st: &mut EngineState, token: Token) -> Result<(), Abort> {
    let p = st.mem.get_avail_token()?;
    p.set_token(&mut st.mem, token);
    p.set_link(&mut st.mem, st.input.cur_input.token_loc());
    st.input.cur_input.set_token_list(p);
    Ok(())
}

pub(crate) fn expand(st: &mut EngineState) -> Result<(), Abort> {
    st.scanner.expand_depth_count += 1;
    if st.scanner.expand_depth_count >= st.scanner.expand_depth {
        return Err(Abort::Overflow {
            capacity: crate::errors::Capacity::ExpansionDepth,
            n: st.scanner.expand_depth,
        });
    }

    let radix_backup = st.scanner.radix;
    let co_backup = st.scanner.cur_order;
    let backup_backup = BACKUP_HEAD.link(&st.mem);

    'reswitch: loop {
        if st.scanner.cur_cmd >= Cmd::Call {
            if st.scanner.cur_cmd < Cmd::EndTemplate {
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
        match st.scanner.command()? {
            Command::TopBotMark { code: mark, marks } => {
                let n = if marks {
                    crate::scan::scan_register_num(st)?
                } else {
                    0
                };
                if n == 0 {
                    st.scanner.cur_ptr = st.scanner.cur_mark[mark].ptr();
                } else if let Some(SaLeaf::MarkClass(m)) =
                    st.mem.find_sa_element(&mut st.scanner, ValueLevel::Mark, n)
                {
                    st.scanner.cur_ptr = st.mem[m].mark(mark).ptr();
                }
                if !st.scanner.cur_ptr.is_null() {
                    begin_token_list(st, TokenList::from_ptr(st.scanner.cur_ptr), TokenType::Mark)?;
                }
            }
            Command::ExpandAfter { unless } => {
                if !unless {
                    get_token(st)?;
                    let t = st.scanner.cur_tok;
                    get_token(st)?;
                    if st.scanner.cur_cmd > Cmd::MAX_COMMAND {
                        expand(st)?;
                    } else {
                        back_input(st)?;
                    }
                    st.scanner.cur_tok = t;
                    back_input(st)?;
                } else {
                    get_token(st)?;
                    if matches!(
                        st.scanner.try_command(),
                        Some(Command::IfTest { code, .. }) if code != IfCode::Case
                    ) {
                        st.scanner.cur_chr += IfCode::UNLESS;
                        continue 'reswitch;
                    }
                    crate::errors::back_error(
                        st,
                        (
                            "You can't use `",
                            crate::output::Esc("unless"),
                            "' before `",
                            crate::output::CmdChr(st.scanner.cur_cmd, st.scanner.cur_chr),
                            '\'',
                        ),
                        &["Continue, and I'll forget that it ever happened."],
                    )?;
                }
            }
            Command::NoExpand { primitive } => {
                if !primitive {
                    get_token_unexpanded(st)?;
                    let t = st.scanner.cur_tok;
                    back_input(st)?;
                    if t.is_cs() && t != Token::from_cs(END_WRITE) {
                        prepend_to_backed_up(st, Token::from_cs(FROZEN_DONT_EXPAND))?;
                    }
                } else {
                    get_token_unexpanded(st)?;
                    let prim = st.eqtb.prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)?;
                    if let Some(prim) = prim {
                        let cmd = st.eqtb.eq_type(prim.eqtb_loc());
                        if cmd > Cmd::MAX_COMMAND {
                            st.scanner.cur_cmd = cmd;
                            st.scanner.cur_chr = st.eqtb.equiv(prim.eqtb_loc());
                            st.scanner.cur_tok = Token::new(st.scanner.cur_cmd, st.scanner.cur_chr);
                            st.scanner.cur_cs = crate::units::EqtbPtr::NONE;
                            continue 'reswitch;
                        }
                        back_input(st)?;
                        prepend_to_backed_up(st, Token::from_cs(FROZEN_PRIMITIVE))?;
                    }
                }
            }
            Command::CsName => {
                let (head, tail) = scan_cs_name_tokens(st)?;
                let first = st.input.first;
                let j = token_chars_to_buffer(st, head.link(&st.mem))?;
                st.scanner.cur_cs = if j > first + 1 || st.input.buffer[first] > 65535 {
                    st.eqtb.no_new_control_sequence = false;
                    let cs = st
                        .eqtb
                        .id_lookup(&mut st.strings, st.input.buffer.span(first, j))?;
                    st.eqtb.no_new_control_sequence = true;
                    cs
                } else if j == first {
                    NULL_CS
                } else {
                    SINGLE_BASE + st.input.buffer[first]
                };
                st.mem.flush_token_list_with_tail(head, tail);
                let cs = st.scanner.cur_cs;
                if st.eqtb.eq_type(cs) == Cmd::UndefinedCs {
                    let _ = st.eqtb.eq_define(
                        &mut st.save_stack,
                        &mut st.mem,
                        &mut st.scanner,
                        cs,
                        Cmd::Relax,
                        TOO_BIG_USV,
                    );
                }
                st.scanner.cur_tok = Token::from_cs(st.scanner.cur_cs);
                back_input(st)?;
            }
            Command::Convert(code) => conv_toks(st, code)?,
            Command::The(code) => ins_the_toks(st, code)?,
            Command::IfTest { code, unless } => conditional(st, code, unless)?,
            Command::FiOrElse(code) => {
                if st.eqtb.int_par(IntPar::TracingIfs) > 0
                    && st.eqtb.int_par(IntPar::TracingCommands) <= 1
                {
                    show_cur_cmd_chr(st);
                }
                if st.scanner.if_limit < code {
                    if st.scanner.if_limit == FiOrElseCode::If {
                        crate::input::insert_relax(st)?;
                    } else {
                        crate::errors::error(
                            st,
                            ("Extra ", crate::output::CmdChr(Cmd::FiOrElse, code.raw())),
                            &[r"I'm ignoring this; it doesn't match any \if."],
                        )?;
                    }
                } else {
                    while st.scanner.fi_or_else() != Some(FiOrElseCode::Fi) {
                        crate::scan::pass_text(st)?;
                    }
                    pop_fi(st)?;
                }
            }
            Command::Input(code) => match code {
                InputCode::EndInput => st.input.force_eof = true,
                InputCode::ScanTokens => crate::scan::pseudo_start(st)?,
                _ if st.scanner.name_in_progress => crate::input::insert_relax(st)?,
                _ => crate::io::start_input(st, None)?,
            },
            _ => {
                crate::errors::error(
                    st,
                    "Undefined control sequence",
                    &[
                        "The control sequence at the end of the top line",
                        r"of your error message was never \def'ed. If you have",
                        r"misspelled it (e.g., `\hobx'), type `I' and the correct",
                        r"spelling (e.g., `I\hbox'). Otherwise just continue,",
                        "and I'll forget about whatever was undefined.",
                    ],
                )?;
            }
        }
        break;
    }

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
    pstack: &mut [TokenList; 9],
    n: i32,
    unbalance: i32,
) -> Result<(), Abort> {
    if st.scanner.long_state == Cmd::Call {
        crate::errors::runaway(st);
        crate::errors::back_error(
            st,
            (
                "Paragraph ended before ",
                crate::output::CsName(st.scanner.warning_index),
                " was complete",
            ),
            &[
                "I suspect you've forgotten a `}', causing me to apply this",
                "control sequence to too much text. How can we recover?",
                "My plan is to forget the whole thing and hope for the best.",
            ],
        )?;
    }
    pstack[n as usize] = TEMP_HEAD_TOKS.link(&st.mem);
    st.scanner.align_state -= unbalance;
    for &list in &pstack[..=n as usize] {
        st.mem.flush_token_list(list);
    }
    Ok(())
}

#[cold]
fn extra_right_brace_in_argument(st: &mut EngineState) -> Result<(), Abort> {
    back_input(st)?;
    st.scanner.align_state += 1;
    st.scanner.long_state = Cmd::Call;
    st.scanner.cur_tok = st.input.par_token;
    crate::errors::ins_error(
        st,
        (
            "Argument of ",
            crate::output::CsName(st.scanner.warning_index),
            " has an extra }",
        ),
        &[
            "I've run across a `}' that doesn't seem to match anything.",
            r"For example, `\def\a#1{...}' and `\a}' would produce",
            r"this error. If you simply proceed now, the `\par' that",
            "I've just inserted will cause me to report a runaway",
            "argument that might be the root of the problem. But if",
            "your `}' was spurious, just type `2' and it will go away.",
        ],
    )
}

#[cold]
fn trace_macro_call(st: &mut EngineState, ref_count: TokenList) {
    begin_diagnostic(st);
    let stack_levels = st.eqtb.int_par(IntPar::TracingStackLevels);
    if stack_levels > 0 {
        if st.input.input_depth() < stack_levels {
            crate::output::print_ln(st);
            print_char(st, b'~');
            for _ in 0..st.input.input_depth() {
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
    end_diagnostic(st, false);
}

#[cold]
fn trace_macro_param(st: &mut EngineState, match_chr: i32, n: i32, list: TokenList) {
    let stack_levels = st.eqtb.int_par(IntPar::TracingStackLevels);
    if stack_levels == 0 || st.input.input_depth() < stack_levels {
        begin_diagnostic(st);
        crate::output::print_nl(st, StrNumber::single_char(match_chr));
        print_int(st, n);
        print(st, "<-");
        crate::output::show_token_list(st, list, TokenList::NULL, 1000);
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
    let ref_count = st.scanner.cur_chr_tokens();
    let mut r = ref_count.link(&st.mem);
    let mut n: i32 = 0;

    if st.eqtb.int_par(IntPar::TracingMacros) > 0 {
        trace_macro_call(st, ref_count);
    }

    if st.mem.is_protected_macro(ref_count) {
        r = r.link(&st.mem);
    }

    let mut pstack = [TokenList::NULL; 9];
    let mut ptail = [TokenList::NULL; 9];
    let mut match_chr: i32 = 0;
    let mut rbrace_ptr = TokenList::NULL;

    'exit: {
        if r.token(&st.mem) != Token::END_MATCH {
            st.scanner.scanner_status = ScannerStatus::Matching;
            let mut unbalance: i32 = 0;
            st.scanner.long_state = st.eqtb.eq_type(st.scanner.cur_cs);
            st.scanner.long_state = st.scanner.long_state.without_outer();

            loop {
                TEMP_HEAD_TOKS.set_link(&mut st.mem, TokenList::NULL);
                let s: TokenList;
                let mut p = TokenList::NULL;
                let mut m: i32 = 0;
                let rt = r.token(&st.mem);
                if rt >= Token::END_MATCH || rt < Token::match_(0) {
                    s = TokenList::NULL;
                } else {
                    match_chr = rt.chr();
                    s = r.link(&st.mem);
                    r = s;
                    p = TEMP_HEAD_TOKS;
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
                            crate::errors::error(
                                st,
                                (
                                    "Use of ",
                                    crate::output::CsName(st.scanner.warning_index),
                                    " doesn't match its definition",
                                ),
                                &[
                                    r"If you say, e.g., `\def\a1{...}', then you must always",
                                    r"put `1' after `\a', since control sequence names are",
                                    "made up of letters only. The macro here has not been",
                                    "followed by the required stuff, so I'm ignoring it.",
                                ],
                            )?;
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
                        && st.scanner.long_state != Cmd::LongCall
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
                                && st.scanner.long_state != Cmd::LongCall
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
                        rbrace_ptr.set_link(&mut st.mem, TokenList::NULL);
                        st.mem.free_avail(p.ptr());
                        p = TEMP_HEAD_TOKS.link(&st.mem);
                        pstack[n as usize] = p.link(&st.mem);
                        ptail[n as usize] = rbrace_ptr;
                        st.mem.free_avail(p.ptr());
                    } else {
                        pstack[n as usize] = TEMP_HEAD_TOKS.link(&st.mem);
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
                        st.mem.flush_token_list_with_tail(pstack[k], ptail[k]);
                    }
                    break 'exit;
                }
            }
        }

        while st.input.cur_input.state == TOKEN_LIST
            && st.input.cur_input.token_loc().is_null()
            && st.input.cur_input.token_type() != TokenType::VTemplate
            && st.input.cur_input.token_type() != TokenType::Output
        {
            crate::input::end_token_list(st)?;
        }

        begin_token_list(st, ref_count, TokenType::Macro)?;
        st.input.cur_input.name = InputName::Macro(st.scanner.warning_index);
        st.input.cur_input.set_token_loc(r.link(&st.mem));

        if n > 0 {
            let params: [MacroParam; 9] = std::array::from_fn(|k| MacroParam {
                list: pstack[k],
                tail: ptail[k],
            });
            let stack = &mut st.input.param_stack;
            stack
                .push_all(&params[..n as usize])
                .map_err(|_| stack.overflow(crate::errors::Capacity::ParameterStack))?;
        }
    }

    st.scanner.scanner_status = save_scanner_status;
    st.scanner.warning_index = save_warning_index;
    Ok(())
}
