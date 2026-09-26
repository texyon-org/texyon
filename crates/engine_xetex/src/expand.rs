//! Expansion: `expand`, `macro_call`, conditionals and `conv_toks`.

use crate::cmd::{
    ACTIVE_CHAR, CALL, CONVERT, CS_NAME, END_CS_NAME, END_TEMPLATE, EXPAND_AFTER, FI_OR_ELSE,
    IF_TEST, IGNORE, INPUT, LEFT_BRACE, LONG_CALL, MAX_COMMAND, NO_EXPAND, OUTER_CALL, OUT_PARAM,
    RELAX, THE, TOP_BOT_MARK, UNDEFINED_CS,
};
use crate::consts::{
    ACTIVE_BASE, BIGGEST_USV, CS_TOKEN_FLAG, FROZEN_DONT_EXPAND, FROZEN_ENDV, FROZEN_PRIMITIVE,
    GLUE_PAR_LEFT_SKIP, GLUE_PAR_RIGHT_SKIP, HMODE, INT_PAR_TRACING_COMMANDS, INT_PAR_TRACING_IFS,
    INT_PAR_TRACING_MACROS, INT_PAR_TRACING_STACK_LEVELS, MMODE, NULL_CS, PRIM_EQTB_BASE,
    SELECTOR_NEW_STRING, SINGLE_BASE, TEX_NULL, TOO_BIG_CHAR, TOO_BIG_USV, UNDEFINED_PRIMITIVE,
    UNLESS_CODE, VMODE,
};
use crate::errors::error;
use crate::errors::Abort;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::input::{
    back_error, back_input, begin_token_list, INSERTED, LEFT_BRACE_LIMIT, MACRO, MARK_TEXT,
    RIGHT_BRACE_LIMIT, TOKEN_LIST, V_TEMPLATE,
};
use crate::mem::{BACKUP_HEAD, GARBAGE, TEMP_HEAD, ZERO_GLUE};
use crate::node::{AnyBox, AnyNode, CondNode, NodePtr};
use crate::output::{
    begin_diagnostic, capture_to_diagnostic, diagnostic_begin_capture_warning_here, end_diagnostic,
    error_here_with_diagnostic, print, print_char, print_cmd_chr, print_err, print_esc, print_int,
    print_scaled, show_cur_cmd_chr, sprint_cs,
};
use crate::scan::{
    fetch_box, get_next, get_token, get_x_non_blank, get_x_token, scan_int, store_new_token,
    END_MATCH_TOKEN, MARK_VAL, MATCHING, MATCH_TOKEN, MAX_CHAR_VAL, NORMAL_SCANNER, NO_EXPAND_FLAG,
    OTHER_TOKEN, SPACE_TOKEN,
};
use crate::state::{EngineState, MemState};
use crate::units::{MemIdx, MemPtr};

extern "C" {
    fn usingGraphite(engine: *mut libc::c_void) -> bool;
}

pub(crate) const IF_CHAR_CODE: i16 = 0;
pub(crate) const IF_CAT_CODE: i16 = 1;
pub(crate) const IF_INT_CODE: i16 = 2;
pub(crate) const IF_DIM_CODE: i16 = 3;
pub(crate) const IF_ODD_CODE: i16 = 4;
pub(crate) const IF_VMODE_CODE: i16 = 5;
pub(crate) const IF_HMODE_CODE: i16 = 6;
pub(crate) const IF_MMODE_CODE: i16 = 7;
pub(crate) const IF_INNER_CODE: i16 = 8;
pub(crate) const IF_VOID_CODE: i16 = 9;
pub(crate) const IF_HBOX_CODE: i16 = 10;
pub(crate) const IF_VBOX_CODE: i16 = 11;
pub(crate) const IFX_CODE: i16 = 12;
pub(crate) const IF_EOF_CODE: i16 = 13;
pub(crate) const IF_TRUE_CODE: i16 = 14;
pub(crate) const IF_FALSE_CODE: i16 = 15;
pub(crate) const IF_CASE_CODE: i16 = 16;
pub(crate) const IF_DEF_CODE: i16 = 17;
pub(crate) const IF_CS_CODE: i16 = 18;
pub(crate) const IF_FONT_CHAR_CODE: i16 = 19;
pub(crate) const IF_IN_CSNAME_CODE: i16 = 20;
pub(crate) const IF_PRIMITIVE_CODE: i16 = 21;
pub(crate) const IF_CODE: u8 = 1;
pub(crate) const FI_CODE: u8 = 2;
pub(crate) const ELSE_CODE: u8 = 3;
pub(crate) const OR_CODE: u8 = 4;

pub(crate) const NUMBER_CODE: i32 = 0;
pub(crate) const ROMAN_NUMERAL_CODE: i32 = 1;
pub(crate) const STRING_CODE: i32 = 2;
pub(crate) const MEANING_CODE: i32 = 3;
pub(crate) const FONT_NAME_CODE: i32 = 4;
pub(crate) const ETEX_REVISION_CODE: i32 = 5;
pub(crate) const EXPANDED_CODE: i32 = 6;
pub(crate) const LEFT_MARGIN_KERN_CODE: i32 = 16;
pub(crate) const RIGHT_MARGIN_KERN_CODE: i32 = 17;
pub(crate) const PDF_STRCMP_CODE: i32 = 18;
pub(crate) const PDF_CREATION_DATE_CODE: i32 = 22;
pub(crate) const PDF_FILE_MOD_DATE_CODE: i32 = 23;
pub(crate) const PDF_FILE_SIZE_CODE: i32 = 24;
pub(crate) const PDF_MDFIVE_SUM_CODE: i32 = 25;
pub(crate) const PDF_FILE_DUMP_CODE: i32 = 26;
pub(crate) const UNIFORM_DEVIATE_CODE: i32 = 29;
pub(crate) const NORMAL_DEVIATE_CODE: i32 = 30;
pub(crate) const XETEX_REVISION_CODE: i32 = 33;
pub(crate) const XETEX_VARIATION_NAME_CODE: i32 = 34;
pub(crate) const XETEX_FEATURE_NAME_CODE: i32 = 35;
pub(crate) const XETEX_SELECTOR_NAME_CODE: i32 = 36;
pub(crate) const XETEX_GLYPH_NAME_CODE: i32 = 37;
pub(crate) const XETEX_UCHAR_CODE: i32 = 38;
pub(crate) const XETEX_UCHARCAT_CODE: i32 = 39;
pub(crate) const JOB_NAME_CODE: i32 = 40;

pub(crate) const MARKS_CODE: i32 = 5;

pub(crate) fn ins_the_toks(st: &mut EngineState) -> Result<(), Abort> {
    let tail = crate::scan::the_toks(st)?;
    ins_temp_head_list(st, tail)
}

/// Inserts the list at `link(temp_head)` whose tail is `tail`.
fn ins_temp_head_list(st: &mut EngineState, tail: MemPtr) -> Result<(), Abort> {
    GARBAGE.set_link(&mut st.mem, tail);
    begin_token_list(st, TEMP_HEAD.link(&st.mem), INSERTED)?;
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
    if st.scanner.cur_chr == FI_CODE as i32 {
        pop_fi(st);
    } else {
        st.scanner.if_limit = FI_CODE;
    }
}

fn get_x_token_or_active_char(st: &mut EngineState) -> Result<(), Abort> {
    get_x_token(st)?;
    if st.scanner.cur_cmd == RELAX && st.scanner.cur_chr == NO_EXPAND_FLAG {
        st.scanner.cur_cmd = ACTIVE_CHAR;
        st.scanner.cur_chr = st.scanner.cur_tok - (CS_TOKEN_FLAG + ACTIVE_BASE);
    }
    Ok(())
}

fn get_next_unexpanded(st: &mut EngineState) -> Result<(), Abort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = NORMAL_SCANNER;
    get_next(st)?;
    st.scanner.scanner_status = save_scanner_status;
    Ok(())
}

fn get_token_unexpanded(st: &mut EngineState) -> Result<(), Abort> {
    let save_scanner_status = st.scanner.scanner_status;
    st.scanner.scanner_status = NORMAL_SCANNER;
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
        st.input.buffer[j as usize] = p.info(&st.mem) % MAX_CHAR_VAL;
        j += 1;
        p = p.link(&st.mem);
    }
    Ok(j)
}

/// Scans the tokens of a `\csname`-like name up to the terminating control sequence.
fn scan_cs_name_tokens(st: &mut EngineState) -> Result<MemPtr, Abort> {
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
        capture_to_diagnostic(st, std::ptr::null_mut());
        st.errors.help(&[
            "The control sequence marked <to be read again> should",
            r"not appear between \csname and \endcsname.",
        ]);
        back_error(st)?;
    }
    st.scanner.is_in_csname = saved_in_csname;
    Ok(head)
}

fn trace_diagnostic(st: &mut EngineState, text: &str) {
    begin_diagnostic(st);
    diagnostic_begin_capture_warning_here(st);
    print(st, text);
    capture_to_diagnostic(st, std::ptr::null_mut());
    end_diagnostic(st, false);
}

pub(crate) fn conditional(st: &mut EngineState) -> Result<(), Abort> {
    if st.eqtb.int_par(INT_PAR_TRACING_IFS) > 0 && st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) <= 1 {
        show_cur_cmd_chr(st);
    }

    let cond_node = NodePtr::<CondNode>::alloc(&mut st.mem)?;
    cond_node.set_link(&mut st.mem, st.scanner.cond_ptr);
    cond_node.set_if_limit(&mut st.mem, st.scanner.if_limit as u16);
    cond_node.set_cur_if(&mut st.mem, st.scanner.cur_if as u16);
    cond_node.set_if_line(&mut st.mem, st.scanner.if_line);
    st.scanner.cond_ptr = cond_node.ptr();
    st.scanner.cur_if = st.scanner.cur_chr as i16;
    st.scanner.if_limit = IF_CODE;
    st.scanner.if_line = st.mem.line;

    let save_cond_ptr = st.scanner.cond_ptr.0;
    let is_unless = st.scanner.cur_chr >= UNLESS_CODE;
    let this_if = (st.scanner.cur_chr % UNLESS_CODE) as i16;

    let mut b = match this_if {
        IF_CHAR_CODE | IF_CAT_CODE => {
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
            if this_if == IF_CHAR_CODE {
                n == st.scanner.cur_chr
            } else {
                m == st.scanner.cur_cmd
            }
        }
        IF_INT_CODE | IF_DIM_CODE => {
            scan_int_or_dimen(st, this_if)?;
            let n = st.scanner.cur_val;
            get_x_non_blank(st)?;
            let relation = if (OTHER_TOKEN + b'<' as i32..=OTHER_TOKEN + b'>' as i32)
                .contains(&st.scanner.cur_tok)
            {
                (st.scanner.cur_tok - OTHER_TOKEN) as u8
            } else {
                error_here_with_diagnostic(st, "Missing = inserted for ");
                print_cmd_chr(st, IF_TEST, this_if as i32);
                capture_to_diagnostic(st, std::ptr::null_mut());
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
        IF_ODD_CODE => {
            scan_int(st)?;
            st.scanner.cur_val % 2 != 0
        }
        IF_VMODE_CODE => st.input.cur_list.mode.abs() == VMODE,
        IF_HMODE_CODE => st.input.cur_list.mode.abs() == HMODE,
        IF_MMODE_CODE => st.input.cur_list.mode.abs() == MMODE,
        IF_INNER_CODE => st.input.cur_list.mode < 0,
        IF_VOID_CODE | IF_HBOX_CODE | IF_VBOX_CODE => {
            crate::scan::scan_register_num(st)?;
            let contents = fetch_box(st, st.scanner.cur_val);
            match this_if {
                IF_VOID_CODE => contents.is_none(),
                IF_HBOX_CODE => matches!(contents, Some(AnyBox::HList(_))),
                _ => matches!(contents, Some(AnyBox::VList(_))),
            }
        }
        IFX_CODE => {
            let save_scanner_status = st.scanner.scanner_status;
            st.scanner.scanner_status = NORMAL_SCANNER;
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
                let mut p = MemPtr(st.scanner.cur_chr).link(&st.mem);
                let mut q = MemPtr(st.eqtb.equiv(n)).link(&st.mem);
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
        IF_EOF_CODE => {
            crate::scan::scan_four_bit_int_or_18(st)?;
            if st.scanner.cur_val == 18 {
                !st.config.shell_escape_enabled
            } else {
                st.input.read_open[st.scanner.cur_val as usize] == crate::consts::CLOSED
            }
        }
        IF_TRUE_CODE => true,
        IF_FALSE_CODE => false,
        IF_DEF_CODE => {
            get_next_unexpanded(st)?;
            st.scanner.cur_cmd != UNDEFINED_CS
        }
        IF_CS_CODE => {
            let head = scan_cs_name_tokens(st)?;
            let first = st.input.first;
            let m = token_chars_to_buffer(st, head.link(&st.mem))?;
            st.scanner.cur_cs = if m == first {
                NULL_CS
            } else if m == first + 1 {
                SINGLE_BASE + st.input.buffer[first as usize]
            } else {
                st.eqtb
                    .id_lookup(&mut st.strings, &st.input.buffer, first, m - first)
                    .map_err(|err| err)?
            };
            st.mem.flush_list(head);
            st.eqtb.eq_type(st.scanner.cur_cs) != UNDEFINED_CS
        }
        IF_IN_CSNAME_CODE => st.scanner.is_in_csname,
        IF_FONT_CHAR_CODE => {
            crate::scan::scan_font_ident(st)?;
            let f = st.scanner.cur_val;
            crate::scan::scan_usv_num(st)?;
            let c = st.scanner.cur_val;
            let fa = st.fonts.font_area[f as usize];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                unsafe { st.fonts.map_char_to_glyph(f, c) }? > 0
            } else if st.fonts.font_bc[f as usize] as i32 <= c
                && st.fonts.font_ec[f as usize] as i32 >= c
            {
                let ec = st.fonts.effective_char(f, c as u16);
                let info = unsafe {
                    st.fonts.font_info[(st.fonts.char_base[f as usize] + ec) as usize].b16
                };
                info.s3 > 0
            } else {
                false
            }
        }
        IF_CASE_CODE => {
            scan_int(st)?;
            let mut n = st.scanner.cur_val;
            if st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) > 1 {
                begin_diagnostic(st);
                diagnostic_begin_capture_warning_here(st);
                print(st, "{case ");
                print_int(st, n);
                print_char(st, b'}' as i32);
                capture_to_diagnostic(st, std::ptr::null_mut());
                end_diagnostic(st, false);
            }
            while n != 0 {
                crate::scan::pass_text(st)?;
                if st.scanner.cond_ptr.0 == save_cond_ptr {
                    if st.scanner.cur_chr != OR_CODE as i32 {
                        skipped_branch_end(st);
                        return Ok(());
                    }
                    n -= 1;
                } else if st.scanner.cur_chr == FI_CODE as i32 {
                    pop_fi(st);
                }
            }
            crate::output::change_if_limit(st, OR_CODE as i16, save_cond_ptr)?;
            return Ok(());
        }
        IF_PRIMITIVE_CODE => {
            get_next_unexpanded(st)?;
            let m = st
                .eqtb
                .prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)
                .map_err(|err| err)?;
            st.scanner.cur_cmd != UNDEFINED_CS
                && m != UNDEFINED_PRIMITIVE
                && st.scanner.cur_cmd == st.eqtb.eq_type(PRIM_EQTB_BASE + m)
                && st.scanner.cur_chr == st.eqtb.equiv(PRIM_EQTB_BASE + m)
        }
        _ => false,
    };

    if is_unless {
        b = !b;
    }

    if st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) > 1 {
        trace_diagnostic(st, if b { "{true}" } else { "{false}" });
    }

    if b {
        crate::output::change_if_limit(st, ELSE_CODE as i16, save_cond_ptr)?;
        return Ok(());
    }

    loop {
        crate::scan::pass_text(st)?;
        if st.scanner.cond_ptr.0 == save_cond_ptr {
            if st.scanner.cur_chr != OR_CODE as i32 {
                skipped_branch_end(st);
                return Ok(());
            }
            error_here_with_diagnostic(st, "Extra ");
            print_esc(st, "or");
            capture_to_diagnostic(st, std::ptr::null_mut());
            st.errors
                .help(&[r"I'm ignoring this; it doesn't match any \if."]);
            error(st)?;
        } else if st.scanner.cur_chr == FI_CODE as i32 {
            pop_fi(st);
        }
    }
}

fn scan_int_or_dimen(st: &mut EngineState, this_if: i16) -> Result<(), Abort> {
    if this_if == IF_INT_CODE {
        scan_int(st)
    } else {
        crate::scan::scan_dimen(st, false, false, false)
    }
}

/// Scanner state saved around a `conv_toks` primitive that reads a general text.
struct SavedScan {
    status: u8,
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
fn scan_text_to_string(st: &mut EngineState, saved: &SavedScan) -> Result<i32, Abort> {
    crate::scan::scan_pdf_ext_toks(st)?;
    if st.shipout.selector == SELECTOR_NEW_STRING {
        return Err(Abort::PdfError(&[
            "tokens",
            "tokens_to_string() called while selector = new_string",
        ]));
    }
    let old_selector = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
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
    s: i32,
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
    fn is_margin_kern_filler(&self, node: AnyNode, par_skip: i16) -> bool {
        match node {
            AnyNode::Ins(_) | AnyNode::Mark(_) | AnyNode::Adjust(_) | AnyNode::Penalty(_) => true,
            AnyNode::Disc(d) => {
                d.pre_break(self).is_null() && d.post_break(self).is_null() && d.subtype(self) == 0
            }
            AnyNode::Math(m) => m.width(self).0 == 0,
            AnyNode::Kern(k) => k.width(self).0 == 0 || k.subtype(self) == 0,
            AnyNode::Glue(g) => {
                g.glue_ptr(self) == ZERO_GLUE || g.subtype(self) == (par_skip + 1) as u16
            }
            AnyNode::HList(h) => {
                h.width(self).0 == 0
                    && h.height(self).0 == 0
                    && h.depth(self).0 == 0
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
    par_skip: i16,
    next: impl Fn(&crate::state::MemState, MemPtr) -> MemPtr,
) {
    let width = loop {
        if p.is_null() || st.mem.is_char_node(p) {
            break None;
        }
        match st.mem.classify(MemIdx::new(p)) {
            Ok(node) if st.mem.is_margin_kern_filler(node, par_skip) => p = next(&st.mem, p),
            Ok(AnyNode::MarginKern(mk)) if mk.subtype(&st.mem) == side => {
                break Some(mk.width(&st.mem).0)
            }
            _ => break None,
        }
    };
    match width {
        Some(w) => print_scaled(st, w),
        None => print_char(st, '0' as i32),
    }
    print(st, "pt");
}

pub(crate) fn conv_toks(st: &mut EngineState) -> Result<(), Abort> {
    let c = st.scanner.cur_chr;
    let mut cat: i32 = 0;
    let mut fnt: i32 = 0;
    let mut arg1: i32 = 0;
    let mut arg2: i32 = 0;
    let mut margin_list = TEX_NULL;

    match c {
        NUMBER_CODE | ROMAN_NUMERAL_CODE | UNIFORM_DEVIATE_CODE => scan_int(st)?,
        STRING_CODE | MEANING_CODE => get_token_unexpanded(st)?,
        FONT_NAME_CODE => crate::scan::scan_font_ident(st)?,
        EXPANDED_CODE => {
            let saved = save_scan(st)?;
            crate::scan::scan_pdf_ext_toks(st)?;
            st.scanner.warning_index = saved.warning_index;
            st.scanner.scanner_status = saved.status;
            let def_ref = st.scanner.def_ref;
            let list = def_ref.link(&st.mem);
            begin_token_list(st, list, INSERTED)?;
            st.mem.free_avail(st.scanner.def_ref);
            st.scanner.def_ref.0 = saved.def_ref;
            unseal_pending_str(st, &saved);
            return Ok(());
        }
        LEFT_MARGIN_KERN_CODE | RIGHT_MARGIN_KERN_CODE => {
            crate::scan::scan_register_num(st)?;
            let Some(AnyBox::HList(hbox)) = fetch_box(st, st.scanner.cur_val) else {
                return Err(Abort::PdfError(&[
                    "marginkern",
                    "a non-empty hbox expected",
                ]));
            };
            margin_list = hbox.list_ptr(&st.mem).0;
        }
        PDF_CREATION_DATE_CODE => {
            let b = st.strings.pool_ptr;
            st.texmf.getcreationdate(&mut st.strings);
            let tail = st.mem.str_toks_cat(&mut st.strings, b, 0)?;
            ins_temp_head_list(st, tail)?;
        }
        PDF_FILE_MOD_DATE_CODE => {
            let saved = save_scan(st)?;
            let s = scan_text_to_string(st, &saved)?;
            let b = st.strings.pool_ptr;
            unsafe { st.strings.getfilemoddate(s) }?;
            return ins_file_query_result(st, &saved, s, b);
        }
        PDF_FILE_SIZE_CODE => {
            let saved = save_scan(st)?;
            let s = scan_text_to_string(st, &saved)?;
            let b = st.strings.pool_ptr;
            unsafe { st.strings.getfilesize(s) }?;
            return ins_file_query_result(st, &saved, s, b);
        }
        PDF_MDFIVE_SUM_CODE => {
            let saved = save_scan(st)?;
            let is_file = crate::scan::scan_keyword(st, "file")?;
            let s = scan_text_to_string(st, &saved)?;
            let b = st.strings.pool_ptr;
            unsafe { st.strings.getmd5sum(s, is_file) };
            return ins_file_query_result(st, &saved, s, b);
        }
        PDF_FILE_DUMP_CODE => {
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
            unsafe { st.strings.getfiledump(s, offset, length) }?;
            return ins_file_query_result(st, &saved, s, b);
        }
        PDF_STRCMP_CODE => {
            let saved = save_scan(st)?;
            crate::scan::compare_strings(st)?;
            restore_scan(st, &saved);
            unseal_pending_str(st, &saved);
        }
        XETEX_UCHAR_CODE => crate::scan::scan_usv_num(st)?,
        XETEX_UCHARCAT_CODE => {
            crate::scan::scan_usv_num(st)?;
            let saved_chr = st.scanner.cur_val;
            scan_int(st)?;
            let code = st.scanner.cur_val;
            if code < LEFT_BRACE as i32
                || code > ACTIVE_CHAR as i32
                || code == OUT_PARAM as i32
                || code == IGNORE as i32
            {
                error_here_with_diagnostic(st, "Invalid code (");
                print_int(st, code);
                print(st, "), should be in the ranges 1..4, 6..8, 10..13");
                capture_to_diagnostic(st, std::ptr::null_mut());
                st.errors
                    .help(&["I'm going to use 12 instead of that illegal code value."]);
                error(st)?;
                cat = 12;
            } else {
                cat = code;
            }
            st.scanner.cur_val = saved_chr;
        }
        XETEX_VARIATION_NAME_CODE => {
            crate::scan::scan_font_ident(st)?;
            fnt = st.scanner.cur_val;
            if st.fonts.font_area[fnt as usize] == AAT_FONT_FLAG {
                scan_int(st)?;
                arg1 = st.scanner.cur_val;
                arg2 = 0;
            } else {
                not_font_error(st, c, fnt, "; not an AAT font")?;
            }
        }
        XETEX_FEATURE_NAME_CODE | XETEX_SELECTOR_NAME_CODE => {
            crate::scan::scan_font_ident(st)?;
            fnt = st.scanner.cur_val;
            if is_aat_or_graphite(st, fnt) {
                scan_int(st)?;
                arg1 = st.scanner.cur_val;
                if c == XETEX_SELECTOR_NAME_CODE {
                    scan_int(st)?;
                    arg2 = st.scanner.cur_val;
                } else {
                    arg2 = 0;
                }
            } else {
                not_font_error(st, c, fnt, "; not an AAT or Graphite font")?;
            }
        }
        XETEX_GLYPH_NAME_CODE => {
            crate::scan::scan_font_ident(st)?;
            fnt = st.scanner.cur_val;
            let fa = st.fonts.font_area[fnt as usize];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                scan_int(st)?;
                arg1 = st.scanner.cur_val;
            } else {
                not_font_error(st, c, fnt, "; not a native platform font")?;
            }
        }
        JOB_NAME_CODE => {
            if st.strings.job_name == 0 {
                unsafe { crate::io::open_log_file(st) }?;
            }
        }
        _ => {}
    }

    let old_selector = st.shipout.selector;
    st.shipout.selector = SELECTOR_NEW_STRING;
    let b = st.strings.pool_ptr;

    match c {
        NUMBER_CODE | PDF_STRCMP_CODE => print_int(st, st.scanner.cur_val),
        ROMAN_NUMERAL_CODE => crate::output::print_roman_int(st, st.scanner.cur_val),
        STRING_CODE => {
            if st.scanner.cur_cs != 0 {
                sprint_cs(st, st.scanner.cur_cs);
            } else {
                print_char(st, st.scanner.cur_chr);
            }
        }
        MEANING_CODE => crate::output::print_meaning(st),
        FONT_NAME_CODE => {
            let f = st.scanner.cur_val as usize;
            let name = st.fonts.font_name[f];
            let fa = st.fonts.font_area[f];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                let start = st.strings.str_start[(name - TOO_BIG_CHAR) as usize] as usize;
                let len = st.strings.length(name).max(0) as usize;
                let quote_char = if st.strings.str_pool[start..start + len].contains(&(b'"' as u16))
                {
                    '\'' as i32
                } else {
                    '"' as i32
                };
                print_char(st, quote_char);
                crate::output::print(st, name);
                print_char(st, quote_char);
            } else {
                crate::output::print(st, name);
            }
            if st.fonts.font_size[f] != st.fonts.font_dsize[f] {
                print(st, " at ");
                print_scaled(st, st.fonts.font_size[f]);
                print(st, "pt");
            }
        }
        ETEX_REVISION_CODE => print(st, ".6"),
        LEFT_MARGIN_KERN_CODE | RIGHT_MARGIN_KERN_CODE => {
            let list = margin_list;
            if c == LEFT_MARGIN_KERN_CODE {
                print_margin_kern(st, MemPtr(list), 0, GLUE_PAR_LEFT_SKIP, |mem, p| {
                    p.link(mem)
                });
            } else {
                let last = st.mem.prev_rightmost(MemPtr(list), MemPtr::NULL);
                print_margin_kern(st, last, 1, GLUE_PAR_RIGHT_SKIP, move |mem, p| {
                    mem.prev_rightmost(MemPtr(list), p)
                });
            }
        }
        UNIFORM_DEVIATE_CODE => {
            let v = st.errors.unif_rand(st.scanner.cur_val);
            print_int(st, v);
        }
        NORMAL_DEVIATE_CODE => {
            let v = crate::scaled_math::norm_rand(st);
            print_int(st, v);
        }
        XETEX_UCHAR_CODE | XETEX_UCHARCAT_CODE => print_char(st, st.scanner.cur_val),
        XETEX_REVISION_CODE => print(st, ".999994"),
        XETEX_VARIATION_NAME_CODE => {
            if st.fonts.font_area[fnt as usize] == AAT_FONT_FLAG {
                let engine = st.fonts.font_layout_engine_ptr(fnt);
                crate::ext::aat_print_font_name(c, engine, arg1, arg2);
            }
        }
        XETEX_FEATURE_NAME_CODE | XETEX_SELECTOR_NAME_CODE => {
            let fa = st.fonts.font_area[fnt as usize];
            let engine = st.fonts.font_layout_engine_ptr(fnt);
            if fa == AAT_FONT_FLAG {
                crate::ext::aat_print_font_name(c, engine, arg1, arg2);
            } else if fa == OTGR_FONT_FLAG && unsafe { usingGraphite(engine) } {
                unsafe { crate::ext::gr_print_font_name(st, c, engine, arg1, arg2) };
            }
        }
        XETEX_GLYPH_NAME_CODE => {
            let fa = st.fonts.font_area[fnt as usize];
            if fa == AAT_FONT_FLAG || fa == OTGR_FONT_FLAG {
                unsafe { crate::ext::print_glyph_name(st, fnt, arg1) }?;
            }
        }
        JOB_NAME_CODE => {
            let job_name = st.strings.job_name;
            crate::output::print_file_name(st, job_name, 0, 0);
        }
        _ => {}
    }

    st.shipout.selector = old_selector;
    let tail = st.mem.str_toks_cat(&mut st.strings, b, cat as i16)?;
    ins_temp_head_list(st, tail)
}

fn is_aat_or_graphite(st: &EngineState, f: i32) -> bool {
    let fa = st.fonts.font_area[f as usize];
    fa == AAT_FONT_FLAG
        || (fa == OTGR_FONT_FLAG && unsafe { usingGraphite(st.fonts.font_layout_engine_ptr(f)) })
}

fn not_font_error(st: &mut EngineState, c: i32, f: i32, msg: &str) -> Result<(), Abort> {
    crate::errors::not_font_error_common(st, CONVERT, c, f, msg)
}

/// Puts `token` in front of the backed-up token list on top of the input stack.
fn prepend_to_backed_up(st: &mut EngineState, token: i32) -> Result<(), Abort> {
    let p = st.mem.get_avail()?;
    p.set_info(&mut st.mem, token);
    p.set_link(&mut st.mem, MemPtr::from_raw(st.input.cur_input.loc));
    st.input.cur_input.start = p.0;
    st.input.cur_input.loc = p.0;
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
                st.scanner.cur_tok = CS_TOKEN_FLAG + FROZEN_ENDV;
                back_input(st)?;
            }
            break;
        }
        if st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) > 1 {
            show_cur_cmd_chr(st);
        }
        match st.scanner.cur_cmd {
            TOP_BOT_MARK => {
                let t = st.scanner.cur_chr % MARKS_CODE;
                if st.scanner.cur_chr >= MARKS_CODE {
                    crate::scan::scan_register_num(st)?;
                } else {
                    st.scanner.cur_val = 0;
                }
                if st.scanner.cur_val == 0 {
                    st.scanner.cur_ptr = st.scanner.cur_mark[t as usize];
                } else {
                    let n = st.scanner.cur_val;
                    if let Some(sa) = st.mem.find_sa_element(&mut st.scanner, MARK_VAL, n) {
                        st.scanner.cur_ptr = sa.child(&st.mem, t);
                    }
                }
                if !st.scanner.cur_ptr.is_null() {
                    begin_token_list(st, st.scanner.cur_ptr, MARK_TEXT)?;
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
                    if st.scanner.cur_cmd == IF_TEST && st.scanner.cur_chr != IF_CASE_CODE as i32 {
                        st.scanner.cur_chr += UNLESS_CODE;
                        continue 'reswitch;
                    }
                    error_here_with_diagnostic(st, "You can't use `");
                    print_esc(st, "unless");
                    print(st, "' before `");
                    print_cmd_chr(st, st.scanner.cur_cmd, st.scanner.cur_chr);
                    print_char(st, '\'' as i32);
                    capture_to_diagnostic(st, std::ptr::null_mut());
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
                    if t >= CS_TOKEN_FLAG {
                        prepend_to_backed_up(st, CS_TOKEN_FLAG + FROZEN_DONT_EXPAND)?;
                    }
                } else {
                    get_token_unexpanded(st)?;
                    st.scanner.cur_cs = st
                        .eqtb
                        .prim_lookup_cs(&mut st.strings, st.scanner.cur_cs)
                        .map_err(|e| e)?;
                    if st.scanner.cur_cs != UNDEFINED_PRIMITIVE {
                        let cmd = st.eqtb.eq_type(PRIM_EQTB_BASE + st.scanner.cur_cs);
                        if cmd > MAX_COMMAND {
                            st.scanner.cur_cmd = cmd;
                            st.scanner.cur_chr = st.eqtb.equiv(PRIM_EQTB_BASE + st.scanner.cur_cs);
                            st.scanner.cur_tok =
                                st.scanner.cur_cmd as i32 * MAX_CHAR_VAL + st.scanner.cur_chr;
                            st.scanner.cur_cs = 0;
                            continue 'reswitch;
                        }
                        back_input(st)?;
                        prepend_to_backed_up(st, CS_TOKEN_FLAG + FROZEN_PRIMITIVE)?;
                    }
                }
            }
            CS_NAME => {
                let head = scan_cs_name_tokens(st)?;
                let first = st.input.first;
                let j = token_chars_to_buffer(st, head.link(&st.mem))?;
                st.scanner.cur_cs = if j > first + 1 || st.input.buffer[first as usize] > 65535 {
                    st.eqtb.no_new_control_sequence = false;
                    let cs = st
                        .eqtb
                        .id_lookup(&mut st.strings, &st.input.buffer, first, j - first)
                        .map_err(|e| e)?;
                    st.eqtb.no_new_control_sequence = true;
                    cs
                } else if j == first {
                    NULL_CS
                } else {
                    SINGLE_BASE + st.input.buffer[first as usize]
                };
                st.mem.flush_list(head);
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
                st.scanner.cur_tok = st.scanner.cur_cs + CS_TOKEN_FLAG;
                back_input(st)?;
            }
            CONVERT => conv_toks(st)?,
            THE => ins_the_toks(st)?,
            IF_TEST => conditional(st)?,
            FI_OR_ELSE => {
                if st.eqtb.int_par(INT_PAR_TRACING_IFS) > 0
                    && st.eqtb.int_par(INT_PAR_TRACING_COMMANDS) <= 1
                {
                    show_cur_cmd_chr(st);
                }
                if st.scanner.cur_chr > st.scanner.if_limit as i32 {
                    if st.scanner.if_limit == IF_CODE {
                        crate::input::insert_relax(st)?;
                    } else {
                        error_here_with_diagnostic(st, "Extra ");
                        print_cmd_chr(st, FI_OR_ELSE, st.scanner.cur_chr);
                        capture_to_diagnostic(st, std::ptr::null_mut());
                        st.errors
                            .help(&[r"I'm ignoring this; it doesn't match any \if."]);
                        error(st)?;
                    }
                } else {
                    while st.scanner.cur_chr != FI_CODE as i32 {
                        crate::scan::pass_text(st)?;
                    }
                    pop_fi(st);
                }
            }
            INPUT => {
                if st.scanner.cur_chr == 1 {
                    st.input.force_eof = true;
                } else if st.scanner.cur_chr == 2 {
                    crate::scan::pseudo_start(st)?;
                } else if st.scanner.name_in_progress {
                    crate::input::insert_relax(st)?;
                } else {
                    unsafe { crate::io::start_input(st, None) }?;
                }
            }
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
        capture_to_diagnostic(st, std::ptr::null_mut());
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
    capture_to_diagnostic(st, std::ptr::null_mut());
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
    let stack_levels = st.eqtb.int_par(INT_PAR_TRACING_STACK_LEVELS);
    if stack_levels > 0 {
        if st.input.input_ptr < stack_levels {
            crate::output::print_ln(st);
            print_char(st, b'~' as i32);
            for _ in 0..st.input.input_ptr {
                print_char(st, b'.' as i32);
            }
            crate::output::print_cs(st, st.scanner.warning_index);
            crate::output::token_show(st, ref_count);
        } else {
            print_char(st, b'~' as i32);
            print_char(st, b'~' as i32);
            crate::output::print_cs(st, st.scanner.warning_index);
        }
    } else {
        crate::output::print_ln(st);
        crate::output::print_cs(st, st.scanner.warning_index);
        crate::output::token_show(st, ref_count);
    }
    capture_to_diagnostic(st, std::ptr::null_mut());
    end_diagnostic(st, false);
}

#[cold]
fn trace_macro_param(st: &mut EngineState, match_chr: i32, n: i32, list: MemPtr) {
    let stack_levels = st.eqtb.int_par(INT_PAR_TRACING_STACK_LEVELS);
    if stack_levels == 0 || st.input.input_ptr < stack_levels {
        begin_diagnostic(st);
        diagnostic_begin_capture_warning_here(st);
        crate::output::print_nl(st, match_chr);
        print_int(st, n);
        print(st, "<-");
        crate::output::show_token_list(st, list, TEX_NULL, 1000);
        capture_to_diagnostic(st, std::ptr::null_mut());
        end_diagnostic(st, false);
    }
}

pub(crate) fn macro_call(st: &mut EngineState) -> Result<(), Abort> {
    let save_scanner_status = st.scanner.scanner_status;
    let save_warning_index = st.scanner.warning_index;
    st.scanner.warning_index = st.scanner.cur_cs;
    let ref_count = MemPtr(st.scanner.cur_chr);
    let mut r = ref_count.link(&st.mem);
    let mut n: i32 = 0;

    if st.eqtb.int_par(INT_PAR_TRACING_MACROS) > 0 {
        trace_macro_call(st, ref_count);
    }

    if st.mem.is_protected_macro(ref_count) {
        r = r.link(&st.mem);
    }

    let mut pstack = [MemPtr::NULL; 9];
    let mut match_chr: i32 = 0;
    let mut rbrace_ptr = MemPtr::NULL;
    let is_match = |t: i32| (MATCH_TOKEN..=END_MATCH_TOKEN).contains(&t);

    'exit: {
        if r.token(&st.mem) != END_MATCH_TOKEN {
            st.scanner.scanner_status = MATCHING;
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
                if rt >= END_MATCH_TOKEN || rt < MATCH_TOKEN {
                    s = MemPtr::NULL;
                } else {
                    match_chr = rt - MATCH_TOKEN;
                    s = r.link(&st.mem);
                    r = s;
                    p = TEMP_HEAD;
                }

                'continue_: loop {
                    get_token(st)?;

                    if st.scanner.cur_tok == r.token(&st.mem) {
                        r = r.link(&st.mem);
                        if is_match(r.token(&st.mem)) {
                            if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
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
                            capture_to_diagnostic(st, std::ptr::null_mut());
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

                    if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
                        if st.scanner.cur_tok >= LEFT_BRACE_LIMIT {
                            extra_right_brace_in_argument(st)?;
                            continue 'continue_;
                        }
                        unbalance = 1;
                        loop {
                            store_new_token(st, &mut p, st.scanner.cur_tok)?;

                            get_token(st)?;

                            if st.scanner.cur_tok == st.input.par_token
                                && st.scanner.long_state != LONG_CALL
                            {
                                runaway_par_in_argument(st, &mut pstack, n, unbalance)?;
                                break 'exit;
                            }

                            if st.scanner.cur_tok < RIGHT_BRACE_LIMIT {
                                if st.scanner.cur_tok < LEFT_BRACE_LIMIT {
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
                        if st.scanner.cur_tok == SPACE_TOKEN && is_match(r.token(&st.mem)) {
                            continue 'continue_;
                        }
                        store_new_token(st, &mut p, st.scanner.cur_tok)?;
                    }

                    m += 1;
                    if !is_match(r.token(&st.mem)) {
                        continue 'continue_;
                    }
                    break 'continue_;
                }

                if !s.is_null() {
                    if m == 1 && p.token(&st.mem) < RIGHT_BRACE_LIMIT {
                        // Strip the braces around a single braced argument.
                        rbrace_ptr.set_link(&mut st.mem, MemPtr::NULL);
                        st.mem.free_avail(p);
                        p = TEMP_HEAD.link(&st.mem);
                        pstack[n as usize] = p.link(&st.mem);
                        st.mem.free_avail(p);
                    } else {
                        pstack[n as usize] = TEMP_HEAD.link(&st.mem);
                    }
                    n += 1;
                    if st.eqtb.int_par(INT_PAR_TRACING_MACROS) > 0 {
                        trace_macro_param(st, match_chr, n, pstack[(n - 1) as usize]);
                    }
                }

                if r.token(&st.mem) == END_MATCH_TOKEN {
                    break;
                }
            }
        }

        while st.input.cur_input.state == TOKEN_LIST
            && st.input.cur_input.loc == TEX_NULL
            && st.input.cur_input.index != V_TEMPLATE
        {
            crate::input::end_token_list(st)?;
        }

        begin_token_list(st, ref_count, MACRO)?;
        st.input.cur_input.name = st.scanner.warning_index;
        st.input.cur_input.loc = r.link(&st.mem).0;

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
            st.input.param_ptr += n;
        }
    }

    st.scanner.scanner_status = save_scanner_status;
    st.scanner.warning_index = save_warning_index;
    Ok(())
}
