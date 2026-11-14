//! Expansion: `expand`, `macro_call`, conditionals and `conv_toks`.

use std::array;
use std::cmp::Ordering;

use crate::base::cmd::Cmd;
use crate::base::codes::{
    ConvertCode, FiOrElseCode, IfCode, IfTest, InputCode, ScannerStatus, TheCode,
};
use crate::base::command::{Command, RelaxCode};
use crate::base::consts::{BIGGEST_USV, CLOSED, NULL_FONT, TOO_BIG_USV};
use crate::base::params::{GluePar, IntPar};
use crate::base::token::{Build, TokCursor, Token, TokenKind, TokenList};
use crate::base::units::{BufPos, FontId, MemPtr, PoolPos, Scaled, StrNumber};
use crate::files::io::{open_log_file, start_input};
use crate::fonts::ext::{gr_print_font_name, print_glyph_name};
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG, Side};
use crate::memory::mem::{MemState, ZERO_GLUE};
use crate::memory::node::{AnyNode, GlueKind, KernSubtype};
use crate::print::errors::{
    Abort, Capacity, OrConfusion, back_error, error, ins_error, not_font_error_common, runaway,
};
use crate::print::{
    CsName, Esc, Selector, begin_diagnostic, end_diagnostic, if_warning, print, print_char,
    print_cs, print_file_name, print_font_name_and_size, print_int, print_ln, print_meaning,
    print_nl, print_roman_int, print_scaled, show_cur_cmd_chr, show_token_list, sprint_cs,
    token_show,
};
use crate::scanning::cond_stack::CondId;
use crate::scanning::input::{
    InputName, MacroParam, TOKEN_LIST, TokenType, back_input, begin_token_list, end_token_list,
    insert_relax, prepend_to_backed_up,
};
use crate::scanning::pgfmath::{native_macro, redirect};
use crate::scanning::scan::{
    compare_strings, copy_delimited_tokens, copy_group_tokens, fetch_box, get_next, get_token,
    get_x_non_blank, get_x_token, pass_text, pseudo_start, scan_dimen, scan_font_ident,
    scan_four_bit_int_or_18, scan_int, scan_keyword, scan_pdf_ext_toks, scan_register_num,
    scan_usv_num, the_toks,
};
use crate::state::EngineState;
use crate::tables::cs::{CsId, CsKind, Frozen};
use crate::tables::eqtb::{Register, define};

fn ins_the_toks(st: &mut EngineState, code: TheCode) -> Result<(), Abort> {
    let b = st.mem.tokens.begin()?;
    the_toks(st, code, &b)?;
    let list = st.mem.tokens.finish(b)?;
    begin_token_list(st, list, TokenType::Inserted)
}

fn pop_cond_node(st: &mut EngineState) -> Result<(), Abort> {
    let p = st.scanner.conds.pop().or_confusion("if")?;
    (st.scanner.cur_if, st.scanner.if_line, st.scanner.if_limit) =
        (p.cur_if, p.if_line, p.if_limit);
    Ok(())
}

fn pop_fi(st: &mut EngineState) -> Result<(), Abort> {
    if st.input.open_levels[st.input.in_open].cond == st.scanner.conds.top() {
        if_warning(st);
    }
    pop_cond_node(st)
}

fn skipped_branch_end(st: &mut EngineState) -> Result<(), Abort> {
    if st.scanner.fi_or_else() == Some(FiOrElseCode::Fi) {
        pop_fi(st)?;
    } else {
        st.scanner.if_limit = FiOrElseCode::Fi.into();
    }
    Ok(())
}

/// The next expanded token as `\if` and `\ifcat` compare it: TeX's `(cur_cmd, cur_chr)`,
/// a `\noexpand`ed active character as itself and anything but a character as `\relax`.
fn if_char_key(st: &mut EngineState) -> Result<(Cmd, i32), Abort> {
    get_x_token(st)?;
    let (cmd, chr) = if st.scanner.cur == Command::Relax(RelaxCode::NoExpand) {
        let cs = st.scanner.cur_tok.cs().or_confusion("noexpand")?;
        // Any other control sequence becomes a code past the scalar values, so `\relax`.
        let chr = match cs.kind() {
            CsKind::Active(c) => c.raw(),
            _ => TOO_BIG_USV,
        };
        (Cmd::ACTIVE_CHAR, chr)
    } else {
        st.scanner.cur.encode()
    };
    Ok(if cmd > Cmd::ACTIVE_CHAR || chr > BIGGEST_USV {
        (Cmd::Relax, TOO_BIG_USV)
    } else {
        (cmd, chr)
    })
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

fn token_chars_to_buffer(st: &mut EngineState, list: TokenList) -> Result<BufPos, Abort> {
    let mut j = st.input.first;
    let mut p = st.mem.tokens.cursor(list);
    while let Some(t) = st.mem.tokens.read(&mut p) {
        if j >= st.input.max_buf_stack {
            st.input.max_buf_stack = j + 1;
            if st.input.max_buf_stack == st.input.buf_size {
                return Err(st.input.buffer_overflow());
            }
        }
        let (_, c) = t.char().or_confusion("cs name")?;
        st.input.buffer[j] = c;
        j += 1;
    }
    Ok(j)
}

fn scan_cs_name_tokens(st: &mut EngineState) -> Result<TokenList, Abort> {
    let b = st.mem.tokens.begin()?;
    let saved_in_csname = st.scanner.is_in_csname;
    st.scanner.is_in_csname = true;
    loop {
        get_x_token(st)?;
        if !st.scanner.cur_cs.is_none() {
            break;
        }
        st.mem.tokens.push(&b, st.scanner.cur_tok)?;
    }
    if st.scanner.cur.cmd() != Cmd::EndCsName {
        back_error(
            st,
            ("Missing ", Esc("endcsname"), " inserted"),
            &[
                "The control sequence marked <to be read again> should",
                r"not appear between \csname and \endcsname.",
            ],
        )?;
    }
    st.scanner.is_in_csname = saved_in_csname;
    st.mem.tokens.finish(b)
}

fn trace_diagnostic(st: &mut EngineState, text: &str) {
    begin_diagnostic(st);
    print(st, text);
    end_diagnostic(st, false);
}

fn change_if_limit(st: &mut EngineState, l: FiOrElseCode, p: CondId) -> Result<(), Abort> {
    if st.scanner.conds.top() == Some(p) {
        st.scanner.if_limit = l.into();
        return Ok(());
    }
    st.scanner.conds.above_mut(p).or_confusion("if")?.if_limit = l.into();
    Ok(())
}

fn conditional(st: &mut EngineState, test: IfTest) -> Result<(), Abort> {
    let IfTest {
        code: this_if,
        unless: is_unless,
    } = test;
    if st.eqtb.int_par(IntPar::TracingIfs) > 0 && st.eqtb.int_par(IntPar::TracingCommands) <= 1 {
        show_cur_cmd_chr(st);
    }

    let save_cond_ptr =
        st.scanner
            .conds
            .push(st.scanner.cur_if, st.scanner.if_limit, st.scanner.if_line);
    st.scanner.cur_if = test;
    st.scanner.if_limit = FiOrElseCode::If.into();
    st.scanner.if_line = st.mem.line;

    let mut b = match this_if {
        IfCode::Char | IfCode::Cat => {
            let (m, n) = if_char_key(st)?;
            let (cmd, chr) = if_char_key(st)?;
            if this_if == IfCode::Char {
                n == chr
            } else {
                m == cmd
            }
        }
        IfCode::Num | IfCode::Dim => {
            let n = scan_int_or_dimen(st, this_if)?;
            get_x_non_blank(st)?;
            let relation = match st.scanner.cur_tok.char() {
                Some((Cmd::OtherChar, c)) if (i32::from(b'<')..=i32::from(b'>')).contains(&c) => {
                    c.cmp(&i32::from(b'='))
                }
                _ => {
                    back_error(
                        st,
                        (
                            "Missing = inserted for ",
                            Command::IfTest(IfTest {
                                code: this_if,
                                unless: false,
                            }),
                        ),
                        &["I was expecting to see `<', `=', or `>'. Didn't."],
                    )?;
                    Ordering::Equal
                }
            };
            let cv = scan_int_or_dimen(st, this_if)?;
            n.cmp(&cv) == relation
        }
        IfCode::Odd => scan_int(st)? % 2 != 0,
        IfCode::VMode => st.input.cur_list.mode.is_vertical(),
        IfCode::HMode => st.input.cur_list.mode.is_horizontal(),
        IfCode::MMode => st.input.cur_list.mode.is_math(),
        IfCode::Inner => st.input.cur_list.mode.is_inner(),
        IfCode::Void | IfCode::HBox | IfCode::VBox => {
            let n = scan_register_num(st)?;
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
            let first = st.scanner.cur;
            get_next(st)?;
            let second = st.scanner.cur;
            let b = if first.cmd() != second.cmd() {
                false
            } else if let (Some(p), Some(q)) = (second.macro_body(), first.macro_body()) {
                p == q || st.mem.tokens.iter(p).eq(st.mem.tokens.iter(q))
            } else {
                first == second
            };
            st.scanner.scanner_status = save_scanner_status;
            b
        }
        IfCode::Eof => {
            let n = scan_four_bit_int_or_18(st)?;
            if n == 18 {
                !st.config.shell_escape_enabled
            } else {
                st.input.read_open[n as usize] == CLOSED
            }
        }
        IfCode::True => true,
        IfCode::False => false,
        IfCode::Defined => {
            get_next_unexpanded(st)?;
            st.scanner.cur.cmd() != Cmd::UndefinedCs
        }
        IfCode::CsName => {
            let name = scan_cs_name_tokens(st)?;
            let first = st.input.first;
            let m = token_chars_to_buffer(st, name)?;
            // Upstream keeps `single_base+c` here for c above U+FFFF and reads past `null_cs`.
            let single = CsId::of_char(st.input.buffer[first]).filter(|_| m == first + 1);
            let cs = if m == first {
                CsId::NULL
            } else if let Some(cs) = single {
                cs
            } else {
                st.eqtb
                    .id_lookup(&mut st.strings, st.input.buffer.span(first, m))?
            };
            st.scanner.cur_cs = Some(cs);
            st.mem.tokens.flush(name);
            st.eqtb.meaning(cs).cmd() != Cmd::UndefinedCs
        }
        IfCode::InCsName => st.scanner.is_in_csname,
        IfCode::FontChar => {
            let f = scan_font_ident(st)?;
            let c = scan_usv_num(st)?;
            let fa = st.fonts.font_area[f];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                st.fonts.map_char_to_glyph(f, c)? > 0
            } else if i32::from(st.fonts.font_bc[f]) <= c && i32::from(st.fonts.font_ec[f]) >= c {
                st.fonts.effective_char_info(f, c as u16).is_some()
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
                pass_text(st)?;
                if st.scanner.conds.top() == Some(save_cond_ptr) {
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
            st.scanner.cur.cmd() != Cmd::UndefinedCs
                && m.is_some_and(|m| st.scanner.cur == st.eqtb.meaning(CsId::primitive(m)))
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
        pass_text(st)?;
        if st.scanner.conds.top() == Some(save_cond_ptr) {
            if st.scanner.fi_or_else() != Some(FiOrElseCode::Or) {
                skipped_branch_end(st)?;
                return Ok(());
            }
            error(
                st,
                ("Extra ", Esc("or")),
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
        Ok(scan_dimen(st, false)?.raw())
    }
}

struct SavedScan {
    status: ScannerStatus,
    warning_index: Option<CsId>,
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

fn scan_text_to_string(st: &mut EngineState, saved: &SavedScan) -> Result<StrNumber, Abort> {
    scan_pdf_ext_toks(st)?;
    if st.shipout.selector == Selector::NewString {
        return Err(Abort::PdfError(&[
            "tokens",
            "tokens_to_string() called while selector = new_string",
        ]));
    }
    let old_selector = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    let text = st.mem.tokens.cursor(st.scanner.def_ref);
    let limit = st.strings.pool_size - st.strings.pool_ptr;
    show_token_list(st, text, TokCursor::END, limit);
    st.shipout.selector = old_selector;
    let s = st.strings.make_string()?;
    st.mem.tokens.delete_ref(st.scanner.def_ref);
    restore_scan(st, saved);
    Ok(s)
}

fn ins_file_query_result(
    st: &mut EngineState,
    saved: &SavedScan,
    s: StrNumber,
    b: PoolPos,
) -> Result<(), Abort> {
    let list = str_list(st, b, None)?;
    st.strings.flush_str(s);
    begin_token_list(st, list, TokenType::Inserted)?;
    unseal_pending_str(st, saved);
    Ok(())
}

fn scan_nonnegative_int(
    st: &mut EngineState,
    keyword: &str,
    err: &str,
    help: &'static str,
) -> Result<i32, Abort> {
    if !scan_keyword(st, keyword)? {
        return Ok(0);
    }
    let n = scan_int(st)?;
    if n < 0 {
        error(
            st,
            (err, " (", n, ")"),
            &[help, "I changed this one to zero."],
        )?;
        return Ok(0);
    }
    Ok(n)
}

impl MemState {
    fn is_margin_kern_filler(&self, node: AnyNode, par_skip: GluePar) -> bool {
        match node {
            AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_) | AnyNode::Penalty(_) => true,
            AnyNode::Disc(d) => {
                self[d].pre_break.is_none()
                    && self[d].post_break.is_none()
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
                    && self[h].list_ptr.is_none()
            }
            _ => false,
        }
    }
}

fn print_margin_kern(
    st: &mut EngineState,
    mut p: MemPtr,
    side: Side,
    par_skip: GluePar,
    next: impl Fn(&MemState, MemPtr) -> MemPtr,
) {
    let width = loop {
        let Some(here) = p.filter(|&x| !st.mem.is_char_node(x)) else {
            break None;
        };
        match st.mem.classify(here) {
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
    let mut margin_list = None;

    match c {
        ConvertCode::Number | ConvertCode::RomanNumeral | ConvertCode::UniformDeviate => {
            num = scan_int(st)?;
        }
        ConvertCode::String | ConvertCode::Meaning => get_token_unexpanded(st)?,
        ConvertCode::FontName => fnt = scan_font_ident(st)?,
        ConvertCode::Expanded => {
            let saved = save_scan(st)?;
            scan_pdf_ext_toks(st)?;
            st.scanner.warning_index = saved.warning_index;
            st.scanner.scanner_status = saved.status;
            begin_token_list(st, st.scanner.def_ref, TokenType::Inserted)?;
            st.scanner.def_ref = saved.def_ref;
            unseal_pending_str(st, &saved);
            return Ok(());
        }
        ConvertCode::LeftMarginKern | ConvertCode::RightMarginKern => {
            let n = scan_register_num(st)?;
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
            st.strings.push_bytes_or_fill(&st.shipout.start_time_str);
            let list = str_list(st, b, None)?;
            begin_token_list(st, list, TokenType::Inserted)?;
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
            let is_file = scan_keyword(st, "file")?;
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
            num = compare_strings(st)?;
            restore_scan(st, &saved);
            unseal_pending_str(st, &saved);
        }
        ConvertCode::Uchar => num = scan_usv_num(st)?,
        ConvertCode::Ucharcat => {
            let saved_chr = scan_usv_num(st)?;
            let code = scan_int(st)?;
            let valid = Cmd::from_raw(code).filter(|c| {
                (Cmd::LeftBrace..=Cmd::ACTIVE_CHAR).contains(c)
                    && *c != Cmd::OUT_PARAM
                    && *c != Cmd::IGNORE
            });
            if let Some(valid) = valid {
                cat = Some(valid);
            } else {
                error(
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
            fnt = scan_font_ident(st)?;
            if st.fonts.font_area[fnt] == AAT_FONT_FLAG {
                arg1 = scan_int(st)?;
                arg2 = 0;
            } else {
                not_font_error(st, c, fnt, "; not an AAT font")?;
            }
        }
        ConvertCode::XetexFeatureName | ConvertCode::XetexSelectorName => {
            fnt = scan_font_ident(st)?;
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
            fnt = scan_font_ident(st)?;
            let fa = st.fonts.font_area[fnt];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                arg1 = scan_int(st)?;
            } else {
                not_font_error(st, c, fnt, "; not a native platform font")?;
            }
        }
        ConvertCode::JobName if st.strings.job_name == StrNumber::NONE => {
            open_log_file(st)?;
        }
        _ => {}
    }

    let old_selector = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    let b = st.strings.pool_ptr;

    match c {
        ConvertCode::Number | ConvertCode::Strcmp => print_int(st, num),
        ConvertCode::RomanNumeral => print_roman_int(st, num),
        ConvertCode::String => match st.scanner.cur_tok.kind() {
            TokenKind::Cs(cs) => sprint_cs(st, Some(cs)),
            TokenKind::Char(_, c) => print_char(st, c),
        },
        ConvertCode::Meaning => print_meaning(st),
        ConvertCode::FontName => print_font_name_and_size(st, fnt),
        ConvertCode::EtexRevision => print(st, ".6"),
        ConvertCode::LeftMarginKern | ConvertCode::RightMarginKern => {
            let list = margin_list;
            if c == ConvertCode::LeftMarginKern {
                print_margin_kern(st, list, Side::Left, GluePar::LeftSkip, |mem, p| {
                    p.and_then(|x| x.link(mem))
                });
            } else {
                let last = st.mem.prev_rightmost(list, None);
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
                gr_print_font_name(st, c, engine, arg1, arg2);
            }
        }
        ConvertCode::XetexGlyphName => {
            let fa = st.fonts.font_area[fnt];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                print_glyph_name(st, fnt, arg1)?;
            }
        }
        ConvertCode::JobName => {
            let job_name = st.strings.job_name;
            print_file_name(st, job_name, StrNumber::NONE, StrNumber::NONE);
        }
        _ => {}
    }

    st.shipout.selector = old_selector;
    let list = str_list(st, b, cat)?;
    begin_token_list(st, list, TokenType::Inserted)
}

fn str_list(st: &mut EngineState, b: PoolPos, cat: Option<Cmd>) -> Result<TokenList, Abort> {
    let out = st.mem.tokens.begin()?;
    st.mem.str_toks_cat(&mut st.strings, b, cat, &out)?;
    st.mem.tokens.finish(out)
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
    not_font_error_common(st, Command::Convert(c), f, msg)
}

pub(crate) fn expand(st: &mut EngineState) -> Result<(), Abort> {
    st.scanner.expand_depth_count += 1;
    if st.scanner.expand_depth_count >= st.scanner.expand_depth {
        return Err(Abort::Overflow {
            capacity: Capacity::ExpansionDepth,
            n: st.scanner.expand_depth,
        });
    }

    'reswitch: loop {
        if st.scanner.cur.cmd() >= Cmd::Call {
            if st.scanner.cur.cmd() < Cmd::EndTemplate {
                macro_call(st)?;
            } else {
                st.scanner.cur_tok = Token::from_cs(Frozen::Endv.cs());
                back_input(st)?;
            }
            break;
        }
        if st.eqtb.int_par(IntPar::TracingCommands) > 1 {
            show_cur_cmd_chr(st);
        }
        match st.scanner.cur {
            Command::TopBotMark { code: mark, marks } => {
                let class = if marks {
                    scan_register_num(st)?
                } else {
                    Register::of(0)
                };
                let text = if class == Register::of(0) {
                    st.scanner.cur_mark[mark]
                } else {
                    st.scanner.mark_classes.get(class)[mark]
                };
                if !text.is_null() {
                    begin_token_list(st, text, TokenType::Mark)?;
                }
            }
            Command::ExpandAfter { unless } => {
                if !unless {
                    get_token(st)?;
                    let t = st.scanner.cur_tok;
                    get_token(st)?;
                    if st.scanner.cur.cmd() > Cmd::MAX_COMMAND {
                        expand(st)?;
                    } else {
                        back_input(st)?;
                    }
                    st.scanner.cur_tok = t;
                    back_input(st)?;
                } else {
                    get_token(st)?;
                    if let Command::IfTest(IfTest { code, .. }) = st.scanner.cur
                        && code != IfCode::Case
                    {
                        st.scanner.cur = Command::IfTest(IfTest { code, unless: true });
                        continue 'reswitch;
                    }
                    back_error(
                        st,
                        (
                            "You can't use `",
                            Esc("unless"),
                            "' before `",
                            st.scanner.cur,
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
                    if t.is_cs() && t != Token::from_cs(Frozen::EndWrite.cs()) {
                        prepend_to_backed_up(st, Token::from_cs(Frozen::DontExpand.cs()))?;
                    }
                } else {
                    get_token_unexpanded(st)?;
                    let prim = st.eqtb.prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)?;
                    if let Some(prim) = prim {
                        let m = st.eqtb.meaning(CsId::primitive(prim));
                        if m.cmd() > Cmd::MAX_COMMAND {
                            st.scanner.cur = m;
                            st.scanner.cur_tok = m.char_token();
                            st.scanner.cur_cs = None;
                            continue 'reswitch;
                        }
                        back_input(st)?;
                        prepend_to_backed_up(st, Token::from_cs(Frozen::Primitive.cs()))?;
                    }
                }
            }
            Command::CsName => {
                let name = scan_cs_name_tokens(st)?;
                let first = st.input.first;
                let j = token_chars_to_buffer(st, name)?;
                let cs = match CsId::of_char(st.input.buffer[first]) {
                    Some(_) if j == first => CsId::NULL,
                    Some(single) if j == first + 1 => single,
                    _ => {
                        st.eqtb.no_new_control_sequence = false;
                        let cs = st
                            .eqtb
                            .id_lookup(&mut st.strings, st.input.buffer.span(first, j))?;
                        st.eqtb.no_new_control_sequence = true;
                        cs
                    }
                };
                st.scanner.cur_cs = Some(cs);
                st.mem.tokens.flush(name);
                if st.eqtb.meaning(cs).cmd() == Cmd::UndefinedCs {
                    define(st, false, cs, Command::Relax(RelaxCode::Primitive))?;
                }
                st.scanner.cur_tok = Token::from_cs(cs);
                back_input(st)?;
            }
            Command::Convert(code) => conv_toks(st, code)?,
            Command::The(code) => ins_the_toks(st, code)?,
            Command::IfTest(test) => conditional(st, test)?,
            Command::FiOrElse(code) => {
                if st.eqtb.int_par(IntPar::TracingIfs) > 0
                    && st.eqtb.int_par(IntPar::TracingCommands) <= 1
                {
                    show_cur_cmd_chr(st);
                }
                if st.scanner.if_limit < code {
                    if st.scanner.if_limit == FiOrElseCode::If {
                        insert_relax(st)?;
                    } else {
                        error(
                            st,
                            ("Extra ", Command::FiOrElse(code)),
                            &[r"I'm ignoring this; it doesn't match any \if."],
                        )?;
                    }
                } else {
                    while st.scanner.fi_or_else() != Some(FiOrElseCode::Fi) {
                        pass_text(st)?;
                    }
                    pop_fi(st)?;
                }
            }
            Command::Input(code) => match code {
                InputCode::EndInput => st.input.force_eof = true,
                InputCode::ScanTokens => pseudo_start(st)?,
                _ if st.scanner.name_in_progress => insert_relax(st)?,
                _ => start_input(st, None)?,
            },
            _ => {
                error(
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

    st.scanner.expand_depth_count -= 1;
    Ok(())
}

#[cold]
fn runaway_par_in_argument(
    st: &mut EngineState,
    pstack: &[TokenList],
    arg: Build,
    unbalance: i32,
) -> Result<(), Abort> {
    if st.scanner.long_state == Cmd::Call {
        runaway(st);
        back_error(
            st,
            (
                "Paragraph ended before ",
                CsName(st.scanner.warning_index),
                " was complete",
            ),
            &[
                "I suspect you've forgotten a `}', causing me to apply this",
                "control sequence to too much text. How can we recover?",
                "My plan is to forget the whole thing and hope for the best.",
            ],
        )?;
    }
    st.mem.tokens.abandon(arg);
    st.scanner.align_state -= unbalance;
    for &list in pstack {
        st.mem.tokens.flush(list);
    }
    Ok(())
}

#[cold]
fn extra_right_brace_in_argument(st: &mut EngineState) -> Result<(), Abort> {
    back_input(st)?;
    st.scanner.align_state += 1;
    st.scanner.long_state = Cmd::Call;
    st.scanner.cur_tok = st.input.par_token;
    ins_error(
        st,
        (
            "Argument of ",
            CsName(st.scanner.warning_index),
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
            print_ln(st);
            print_char(st, b'~');
            for _ in 0..st.input.input_depth() {
                print_char(st, b'.');
            }
            print_cs(st, st.scanner.warning_index);
            token_show(st, ref_count);
        } else {
            print_char(st, b'~');
            print_char(st, b'~');
            print_cs(st, st.scanner.warning_index);
        }
    } else {
        print_ln(st);
        print_cs(st, st.scanner.warning_index);
        token_show(st, ref_count);
    }
    end_diagnostic(st, false);
}

#[cold]
fn trace_macro_param(st: &mut EngineState, match_chr: i32, n: usize, list: TokenList) {
    let stack_levels = st.eqtb.int_par(IntPar::TracingStackLevels);
    if stack_levels == 0 || st.input.input_depth() < stack_levels {
        begin_diagnostic(st);
        print_nl(st, StrNumber::single_char(match_chr));
        print_int(st, i32::try_from(n).unwrap_or(i32::MAX));
        print(st, "<-");
        let from = st.mem.tokens.cursor(list);
        show_token_list(st, from, TokCursor::END, 1000);
        end_diagnostic(st, false);
    }
}

pub(crate) fn macro_call(st: &mut EngineState) -> Result<(), Abort> {
    if let Some(cs) = redirect(st) {
        st.scanner.cur_cs = Some(cs);
        st.scanner.cur = st.eqtb.meaning(cs);
    }
    let name = st.scanner.cur_cs.or_confusion("macro_call")?;
    let save_scanner_status = st.scanner.scanner_status;
    let save_warning_index = st.scanner.warning_index;
    st.scanner.warning_index = Some(name);
    let body = st.scanner.cur.macro_body().or_confusion("macro_call")?;
    let mut r = st.mem.tokens.cursor(body);
    let mut n: usize = 0;

    if st.eqtb.int_par(IntPar::TracingMacros) > 0 {
        trace_macro_call(st, body);
    }

    if st.mem.is_protected_macro(body) {
        r = st.mem.tokens.next(r);
    }

    let mut pstack = [TokenList::NULL; 9];
    let mut match_chr: i32 = 0;
    // A parameter text always ends with `END_MATCH`.
    let tok = |st: &EngineState, c: TokCursor| st.mem.tokens.peek(c).unwrap_or(Token::END_MATCH);

    'exit: {
        if tok(st, r) != Token::END_MATCH {
            st.scanner.scanner_status = ScannerStatus::Matching;
            let mut unbalance: i32 = 0;
            st.scanner.long_state = st.eqtb.meaning(name).cmd();
            st.scanner.long_state = st.scanner.long_state.without_outer();

            loop {
                let arg = st.mem.tokens.begin()?;
                let s: Option<TokCursor>;
                let mut m: i32 = 0;
                let rt = tok(st, r);
                if let Some((Cmd::MATCH, c)) = rt.char() {
                    match_chr = c;
                    r = st.mem.tokens.next(r);
                    s = Some(r);
                } else {
                    s = None;
                }

                'continue_: loop {
                    if s == Some(r) && !tok(st, r).is_match() {
                        copy_delimited_tokens(st, &arg, &mut m, tok(st, r))?;
                    }
                    get_token(st)?;

                    if st.scanner.cur_tok == tok(st, r) {
                        r = st.mem.tokens.next(r);
                        if tok(st, r).is_match() {
                            if st.scanner.cur_tok.is_left_brace() {
                                st.scanner.align_state -= 1;
                            }
                            break 'continue_;
                        }
                        continue 'continue_;
                    }

                    if s != Some(r) {
                        let Some(s) = s else {
                            error(
                                st,
                                (
                                    "Use of ",
                                    CsName(st.scanner.warning_index),
                                    " doesn't match its definition",
                                ),
                                &[
                                    r"If you say, e.g., `\def\a1{...}', then you must always",
                                    r"put `1' after `\a', since control sequence names are",
                                    "made up of letters only. The macro here has not been",
                                    "followed by the required stuff, so I'm ignoring it.",
                                ],
                            )?;
                            st.mem.tokens.abandon(arg);
                            for &list in &pstack[..n] {
                                st.mem.tokens.flush(list);
                            }
                            break 'exit;
                        };
                        let mut t = s;
                        loop {
                            st.mem.tokens.push(&arg, tok(st, t))?;
                            m += 1;
                            let mut u = st.mem.tokens.next(t);
                            let mut v = s;
                            loop {
                                if u == r {
                                    if st.scanner.cur_tok != tok(st, v) {
                                        break;
                                    }
                                    r = st.mem.tokens.next(v);
                                    continue 'continue_;
                                }
                                if tok(st, u) != tok(st, v) {
                                    break;
                                }
                                u = st.mem.tokens.next(u);
                                v = st.mem.tokens.next(v);
                            }
                            t = st.mem.tokens.next(t);
                            if t == r {
                                break;
                            }
                        }
                        r = s;
                    }

                    if st.scanner.cur_tok == st.input.par_token
                        && st.scanner.long_state != Cmd::LongCall
                    {
                        runaway_par_in_argument(st, &pstack[..n], arg, unbalance)?;
                        break 'exit;
                    }

                    if st.scanner.cur_tok.is_brace() {
                        if !st.scanner.cur_tok.is_left_brace() {
                            extra_right_brace_in_argument(st)?;
                            continue 'continue_;
                        }
                        unbalance = 1;
                        loop {
                            st.mem.tokens.push(&arg, st.scanner.cur_tok)?;
                            if copy_group_tokens(st, &arg, &mut unbalance, false)? {
                                break;
                            }

                            get_token(st)?;

                            if st.scanner.cur_tok == st.input.par_token
                                && st.scanner.long_state != Cmd::LongCall
                            {
                                runaway_par_in_argument(st, &pstack[..n], arg, unbalance)?;
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
                        st.mem.tokens.push(&arg, st.scanner.cur_tok)?;
                    } else {
                        if st.scanner.cur_tok == Token::SPACE && tok(st, r).is_match() {
                            continue 'continue_;
                        }
                        st.mem.tokens.push(&arg, st.scanner.cur_tok)?;
                    }

                    m += 1;
                    if !tok(st, r).is_match() {
                        continue 'continue_;
                    }
                    break 'continue_;
                }

                if s.is_some() {
                    if m == 1 && st.mem.tokens.last_pushed(&arg).is_some_and(Token::is_brace) {
                        st.mem.tokens.drop_first_last(&arg);
                    }
                    pstack[n] = st.mem.tokens.finish(arg)?;
                    n += 1;
                    if st.eqtb.int_par(IntPar::TracingMacros) > 0 {
                        trace_macro_param(st, match_chr, n, pstack[n - 1]);
                    }
                } else {
                    st.mem.tokens.abandon(arg);
                }

                if tok(st, r) == Token::END_MATCH {
                    break;
                }
            }
        }

        if native_macro(st, name, &pstack[..n]) {
            for &list in &pstack[..n] {
                st.mem.tokens.flush(list);
            }
            break 'exit;
        }

        while st.input.cur_input.state == TOKEN_LIST
            && st.input.cur_input.token_loc().is_end()
            && st.input.cur_input.token_type() != TokenType::VTemplate
            && st.input.cur_input.token_type() != TokenType::Output
        {
            end_token_list(st)?;
        }

        begin_token_list(st, body, TokenType::Macro)?;
        st.input.cur_input.name = InputName::Macro(name);
        st.input.cur_input.set_token_loc(st.mem.tokens.next(r));

        if n > 0 {
            let params: [MacroParam; 9] = array::from_fn(|k| MacroParam { list: pstack[k] });
            let stack = &mut st.input.param_stack;
            stack
                .push_all(&params[..n])
                .map_err(|_| stack.overflow(Capacity::ParameterStack))?;
        }
    }

    st.scanner.scanner_status = save_scanner_status;
    st.scanner.warning_index = save_warning_index;
    Ok(())
}
